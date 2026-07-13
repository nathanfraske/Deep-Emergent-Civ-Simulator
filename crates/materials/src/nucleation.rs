// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Stage 5, §5: grain size from classical nucleation theory, the freezer output side's last piece and the
//! rate-law kernel's third consumer. The steady-state nucleation rate is `I = I_0 * exp(-dG*/(k_B*T))`, which is
//! [`civsim_physics::laws::arrhenius_rate`] with the nucleation reduced barrier `dG*/(k_B*T)`; the grain size
//! follows from the balance of the nucleation rate against the growth rate.
//!
//! THE RESERVED RESIDUAL (gate-ruled framing 3, #188). The derivation-hunt reduced the whole mechanism to ONE
//! reserved knob, the Turnbull interfacial coefficient `C` (`~0.45` close-packed metals, `~0.32` for the
//! Bi/Sb/Ge/water class, Turnbull 1950), caller-supplied and never planted. The SECOND physical degree of
//! freedom (the build's prove-it catch: the nucleation barrier `gamma~^3/dGv~^2 = C^3*(dS_f/R)/(1-T/Tm)^2`
//! leaves a net `dS_f/R` that `beta_gamma = C*(dS_f/R)` cannot recover, and the critical radius needs `C` alone)
//! is NOT a second authored knob: it is the melt entropy `dS_f`, DERIVED from the MEASURED heat of fusion
//! `dH_f = T_m * dS_f` (a cited `[M]` datum joining the `metal_eos` anchors, `dS_f = dH_f/T_m` over the built
//! Lindemann `T_m`). So the interfacial energy and the bulk driving force, physically independent inputs, key
//! off one reserved interface coefficient plus one measured melt enthalpy (PD6: the floor could not derive the
//! melt entropy, so the substrate is developed with a refutable datum rather than authoring a knob).
//!
//! THE DERIVE-CLEAN CHAIN. The solid-liquid interfacial energy is the Turnbull relation
//! `gamma_sl = C * dH_f / (N_A^(1/3) * V_m^(2/3))` ([`interfacial_energy`]), NOT the broken-bond `E_coh` route
//! (that yields the solid-VAPOR energy, rejected in the hunt). Everything else derives: the Richards ratio
//! `dS_f/R = dH_f/(R*T_m)` ([`richards_ratio`]), the dimensionless interfacial energy `beta_gamma = C*(dS_f/R)`
//! ([`reduced_interfacial_energy`]) and driving force `(dS_f/R)*(1 - T/T_m)` ([`reduced_driving_force`]), the
//! reduced barrier ([`reduced_nucleation_barrier`], dimensionless and `O(1)` so no overflow), the critical size
//! ([`critical_radius_over_spacing`], [`critical_atom_count`]), the Zeldovich factor ([`zeldovich_factor`], the
//! built `Fixed::sqrt` over the barrier curvature), and the prefactor `I_0 = N_s*Z*beta*` ([`nucleation_prefactor`],
//! composing the density-derived site count, the Zeldovich factor, and the built self-diffusivity attachment
//! rate, reserving nothing). The grain size is the Avrami balance ([`avrami_grain_size`]).
//!
//! HONEST LIMITS, each at its site: the HOMOGENEOUS baseline (heterogeneous nucleation multiplies `dG*` by the
//! contact-angle potency `f(theta)` and changes `N_s` to substrate sites, so the wetting angle `theta` is the
//! reserved residual of a later follow-on, not this slice); the CRYSTALLINE regime (a glass has no CNT barrier
//! of this form); and the lumped single-rate, reduced-order treatment (not a full cluster population balance or
//! cooling-path integral). Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.

use civsim_core::Fixed;
use civsim_physics::laws;

const ZERO: Fixed = Fixed::ZERO;

/// The Turnbull unit fold mapping `C * dH_f[kJ/mol] / V_atom[A^3]^(2/3)` to `gamma_sl[J/m^2]`: the exact SI ratio
/// `10^23 / N_A` (the `10^3` kJ-to-J and the `10^20` from `(A^3 -> m^3)^(2/3) = 10^-20 m^2` folded with the
/// per-mole-to-per-atom `1/N_A`), `= 10^8 / 602214076 ~ 0.16605`, from the SI-exact Avogadro constant. Folded
/// once at the atomic scale (the raw `10^3/N_A` underflows Q32.32), the same care the `k_B` and Avogadro folds
/// elsewhere take.
fn turnbull_energy_fold() -> Fixed {
    Fixed::from_ratio(100_000_000, 602_214_076)
}

/// The pure-math factor `16*pi/3` in the classical nucleation barrier `dG* = 16*pi*gamma^3/(3*dGv^2)`, derived
/// from `Fixed::PI` (no authored decimal): `~16.755`.
fn sixteen_pi_over_three() -> Fixed {
    Fixed::from_int(16)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_div(Fixed::from_int(3)))
        .unwrap_or(ZERO)
}

/// The pure-math factor `4*pi/3` (the sphere volume factor for the critical-nucleus atom count), from `Fixed::PI`.
fn four_pi_over_three() -> Fixed {
    Fixed::from_int(4)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_div(Fixed::from_int(3)))
        .unwrap_or(ZERO)
}

/// The pure-math factor `3*pi` in the Zeldovich factor `Z = sqrt(dG*/(3*pi*k_B*T*n*^2))`, from `Fixed::PI`.
fn three_pi() -> Fixed {
    Fixed::from_int(3).checked_mul(Fixed::PI).unwrap_or(ZERO)
}

/// The solid-liquid interfacial energy `gamma_sl` (J/m^2), the Turnbull (1950) relation
/// `gamma_sl = C * dH_f / (N_A^(1/3) * V_m^(2/3)) = C * (dH_f/N_A) / V_atom^(2/3)`: the Turnbull coefficient `C`
/// times the per-atom heat of fusion over the atomic interfacial area. `C` is the ONE RESERVED-with-basis knob
/// (`~0.45` close-packed metals, `~0.32` Bi/Sb/Ge/water, Turnbull 1950, per bonding class, primary-verified
/// before entry), caller-supplied and never planted; `dH_f` is the MEASURED `[M]` molar heat of fusion (kJ/mol)
/// the caller reads from the anchors; `V_atom^(2/3) = cbrt(V_atom)^2` is the built exact op. The melting point
/// cancels in the Turnbull-Richards collapse, so it does not enter here. Non-positive inputs yield zero (no
/// interfacial energy).
pub fn interfacial_energy(
    atomic_volume_angstrom3: Fixed,
    molar_heat_of_fusion: Fixed,
    turnbull_coefficient: Fixed,
) -> Fixed {
    if atomic_volume_angstrom3 <= ZERO
        || molar_heat_of_fusion <= ZERO
        || turnbull_coefficient <= ZERO
    {
        return ZERO;
    }
    // V_atom^(2/3) = cbrt(V_atom)^2, the exact built op (the Lindemann factor's sibling).
    let area = atomic_volume_angstrom3.cbrt().powi(2);
    if area <= ZERO {
        return ZERO;
    }
    turnbull_coefficient
        .checked_mul(molar_heat_of_fusion)
        .and_then(|x| x.checked_mul(turnbull_energy_fold()))
        .and_then(|x| x.checked_div(area))
        .unwrap_or(Fixed::MAX)
}

/// The Richards ratio `dS_f/R = dH_f/(R*T_m)`: the MEASURED entropy of fusion in units of the gas constant, the
/// mechanism's second degree of freedom, now DERIVED from the measured heat of fusion and the built Lindemann
/// melting point rather than reserved. Near `1` for close-packed metals (Richards' rule) and near `3` for the
/// covalent semimetals (Si, Ge, Sb, Bi), so it carries real per-class physics the interface coefficient does
/// not. Non-positive inputs yield zero.
pub fn richards_ratio(
    molar_heat_of_fusion: Fixed,
    melting_point: Fixed,
    gas_constant: Fixed,
) -> Fixed {
    if molar_heat_of_fusion <= ZERO || melting_point <= ZERO || gas_constant <= ZERO {
        return ZERO;
    }
    match gas_constant.checked_mul(melting_point) {
        Some(rt) if rt > ZERO => molar_heat_of_fusion.checked_div(rt).unwrap_or(Fixed::MAX),
        _ => ZERO,
    }
}

/// The dimensionless interfacial energy `gamma~ = beta_gamma = C * (dS_f/R)`, the Turnbull-Richards collapse: the
/// interfacial energy in units of `k_B*T_m` per atomic area. Derived from the reserved Turnbull `C` and the
/// derived Richards ratio. Non-positive inputs yield zero.
pub fn reduced_interfacial_energy(richards_ratio: Fixed, turnbull_coefficient: Fixed) -> Fixed {
    if richards_ratio <= ZERO || turnbull_coefficient <= ZERO {
        return ZERO;
    }
    turnbull_coefficient
        .checked_mul(richards_ratio)
        .unwrap_or(Fixed::MAX)
}

/// The dimensionless volumetric driving force `dGv~ = (dS_f/R) * (1 - T/T_m)`: the undercooling free-energy gain
/// in units of `k_B*T_m` per atomic volume, reading the derived Richards ratio, the built `T_m`, and the
/// environment `T`. Zero at or above the melting point (no undercooling, no solidification driving force) and
/// zero for a non-positive Richards ratio or melting point.
pub fn reduced_driving_force(
    richards_ratio: Fixed,
    melting_point: Fixed,
    temperature: Fixed,
) -> Fixed {
    if richards_ratio <= ZERO || melting_point <= ZERO {
        return ZERO;
    }
    // The undercooling fraction 1 - T/T_m, clamped non-negative (at or above T_m there is no driving force).
    let ratio = match temperature.max(ZERO).checked_div(melting_point) {
        Some(x) => x,
        None => return ZERO,
    };
    let undercooling = match Fixed::ONE.checked_sub(ratio) {
        Some(u) if u > ZERO => u,
        _ => return ZERO, // T >= T_m: no undercooling
    };
    richards_ratio
        .checked_mul(undercooling)
        .unwrap_or(Fixed::MAX)
}

/// The reduced nucleation barrier `dG*/(k_B*T) = (16*pi/3) * gamma~^3 / dGv~^2 * (T_m/T)`, the classical
/// nucleation barrier `dG* = 16*pi*gamma^3/(3*dGv^2)` written in the dimensionless `beta_gamma`/`dGv~` variables
/// (so every operand is `O(1)` and Q32.32 never overflows, where the absolute `gamma^3` and `dGv^2` would). This
/// is the value the rate-law kernel consumes as the nucleation reduced barrier. A non-positive driving force (no
/// undercooling) or temperature yields [`Fixed::MAX`] (an infinite barrier, which the kernel reads as no
/// nucleation).
pub fn reduced_nucleation_barrier(
    reduced_interfacial_energy: Fixed,
    reduced_driving_force: Fixed,
    melting_point: Fixed,
    temperature: Fixed,
) -> Fixed {
    if reduced_interfacial_energy <= ZERO
        || reduced_driving_force <= ZERO
        || melting_point <= ZERO
        || temperature <= ZERO
    {
        return Fixed::MAX;
    }
    let gamma_cubed = reduced_interfacial_energy.powi(3);
    let dgv_squared = reduced_driving_force.powi(2);
    if dgv_squared <= ZERO {
        return Fixed::MAX;
    }
    // (16*pi/3) * gamma~^3 / dGv~^2 * (T_m/T).
    let tm_over_t = match melting_point.checked_div(temperature) {
        Some(x) if x > ZERO => x,
        _ => return Fixed::MAX,
    };
    sixteen_pi_over_three()
        .checked_mul(gamma_cubed)
        .and_then(|x| x.checked_div(dgv_squared))
        .and_then(|x| x.checked_mul(tm_over_t))
        .unwrap_or(Fixed::MAX)
}

/// The steady-state nucleation rate `I = I_0 * exp(-dG*/(k_B*T))` over the rate-law kernel
/// ([`laws::arrhenius_rate`]), the kernel's third consumer: the composed prefactor `I_0`
/// ([`nucleation_prefactor`]) and the nucleation reduced barrier ([`reduced_nucleation_barrier`]). The kernel's
/// exp-window freeze-out (`reduced_barrier > 22 -> zero`) is the physical nucleation suppression at shallow
/// undercooling (a barrier too high to cross), with nucleation turning on as undercooling deepens the driving
/// force and lowers the barrier.
pub fn nucleation_rate(prefactor: Fixed, reduced_barrier: Fixed) -> Fixed {
    laws::arrhenius_rate(prefactor, reduced_barrier)
}

/// The critical-nucleus radius in units of the atomic spacing, `r*/a = 2 * beta_gamma / dGv~ = 2C/(1 - T/T_m)`:
/// the Gibbs-Thomson critical size. It needs the Turnbull `C` (through `beta_gamma/dGv~ = C/(1 - T/T_m)`)
/// independently of `beta_gamma`, one witness that the mechanism carries two degrees of freedom. A non-positive
/// driving force yields zero (no finite critical size without a driving force).
pub fn critical_radius_over_spacing(
    reduced_interfacial_energy: Fixed,
    reduced_driving_force: Fixed,
) -> Fixed {
    if reduced_interfacial_energy <= ZERO || reduced_driving_force <= ZERO {
        return ZERO;
    }
    Fixed::from_int(2)
        .checked_mul(reduced_interfacial_energy)
        .and_then(|x| x.checked_div(reduced_driving_force))
        .unwrap_or(Fixed::MAX)
}

/// The number of atoms in the critical nucleus, `n* = (4*pi/3) * (r*/a)^3` (a sphere of radius `r*` measured in
/// atomic volumes). Derived from the critical radius; a non-positive radius yields zero.
pub fn critical_atom_count(critical_radius_over_spacing: Fixed) -> Fixed {
    if critical_radius_over_spacing <= ZERO {
        return ZERO;
    }
    four_pi_over_three()
        .checked_mul(critical_radius_over_spacing.powi(3))
        .unwrap_or(Fixed::MAX)
}

/// The Zeldovich factor `Z = sqrt(dG*/(3*pi*k_B*T*n*^2)) = sqrt(reduced_barrier/(3*pi)) / n*`: the barrier-
/// curvature flatness factor (the fraction of critical nuclei that grow rather than redissolve), dimensionless
/// and typically `~0.01-0.1`, derived from the reduced barrier and the critical atom count over the built
/// `Fixed::sqrt`. Reserves no value. A non-positive atom count or barrier yields zero.
pub fn zeldovich_factor(reduced_barrier: Fixed, critical_atom_count: Fixed) -> Fixed {
    if reduced_barrier <= ZERO || critical_atom_count <= ZERO {
        return ZERO;
    }
    let inner = match reduced_barrier.checked_div(three_pi()) {
        Some(x) if x > ZERO => x,
        _ => return ZERO,
    };
    inner
        .sqrt()
        .checked_div(critical_atom_count)
        .unwrap_or(ZERO)
}

/// The steady-state CNT prefactor `I_0 = N_s * Z * beta*`: the number density of nucleation sites `N_s`, the
/// Zeldovich factor `Z`, and the atomic attachment rate `beta*`. Reserves NOTHING: `N_s` is density-derived (the
/// caller supplies the material density over the atomic mass), `Z` is the built-`sqrt` barrier curvature
/// ([`zeldovich_factor`]), and `beta*` is the built self-diffusivity ([`crate::freezer::self_diffusivity`], the
/// attempt frequency through the kernel) times the geometric surface-atom count of the critical nucleus. A pure
/// composition of built quantities. A non-positive factor yields zero.
pub fn nucleation_prefactor(
    number_density: Fixed,
    zeldovich_factor: Fixed,
    attachment_rate: Fixed,
) -> Fixed {
    if number_density <= ZERO || zeldovich_factor <= ZERO || attachment_rate <= ZERO {
        return ZERO;
    }
    number_density
        .checked_mul(zeldovich_factor)
        .and_then(|x| x.checked_mul(attachment_rate))
        .unwrap_or(Fixed::MAX)
}

/// A reduced-order grain size from the nucleation-growth balance, the Avrami/JMAK scaling `d ~ (G/I)^(1/4)`: the
/// grain linear size set by the competition between the growth velocity `G` and the nucleation rate `I` (a fast
/// nucleation rate gives many small grains, a fast growth gives few large grains). The `^(1/4)` is two built
/// `Fixed::sqrt` (no fractional-power primitive). REDUCED-ORDER: a lumped Avrami estimate over a single
/// nucleation-growth regime, not a full cluster population balance or a cooling-path integral (the honest
/// limit). The growth rate `G` composes from the built self-diffusivity (interface-limited), caller-supplied.
/// A non-positive nucleation rate yields zero (no grains form without nucleation).
pub fn avrami_grain_size(growth_rate: Fixed, nucleation_rate: Fixed) -> Fixed {
    if growth_rate <= ZERO || nucleation_rate <= ZERO {
        return ZERO;
    }
    match growth_rate.checked_div(nucleation_rate) {
        Some(ratio) if ratio > ZERO => ratio.sqrt().sqrt(),
        _ => ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test fixtures, clearly test-only and NOT canonical anchor entries (the canonical dH_f column is the
    // primary-verified [M] anchor slice). The Turnbull coefficient C ~ 0.45 is the CITED close-packed-metal
    // value (Turnbull 1950), fed INTO the magnitude check with the measured gamma_sl as the target band, never
    // the reverse (non-circular). Iron dH_f = 13.81 kJ/mol is the CRC heat of fusion.
    fn turnbull_c() -> Fixed {
        Fixed::from_ratio(45, 100) // C ~ 0.45, cited close-packed metals (test-only)
    }
    fn iron_heat_of_fusion() -> Fixed {
        Fixed::from_ratio(1381, 100) // dH_f = 13.81 kJ/mol (CRC, test-only)
    }
    fn iron_atomic_volume() -> Fixed {
        Fixed::from_ratio(1177, 100) // V_atom ~ 11.77 A^3 (Fe, from V_m = 7.09 cm^3/mol)
    }
    fn r_kj_per_mol_k() -> Fixed {
        Fixed::from_ratio(8314, 1_000_000) // R = 8.314e-3 kJ/(mol K), derived (N_A * k_B)
    }
    fn iron_melting_point() -> Fixed {
        Fixed::from_int(1811) // built Lindemann T_m ~ 1811 K (here a fixture standing in for it)
    }
    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn interfacial_energy_lands_iron_non_circularly() {
        // NON-CIRCULAR magnitude check (the confirmation-bias discipline): feed the CITED Turnbull C = 0.45 and
        // the MEASURED iron dH_f = 13.81 kJ/mol, and require the derived gamma_sl to land iron's measured
        // solid-liquid interfacial energy ~0.204 J/m^2 within Turnbull's own ~10-20% accuracy. C is cited-
        // independent (not back-solved from gamma_sl), so the match is a real consequence, not a fit.
        let gamma = interfacial_energy(iron_atomic_volume(), iron_heat_of_fusion(), turnbull_c());
        assert!(
            close(gamma, 0.204, 0.04),
            "cited C and measured dH_f land iron's ~0.204 J/m^2 within Turnbull's scatter: {gamma:?}"
        );
        // Monotone in the heat of fusion: a higher melt enthalpy raises the interfacial energy (the Turnbull
        // proportionality), and a larger atomic area lowers it.
        let higher_hf = interfacial_energy(iron_atomic_volume(), Fixed::from_int(20), turnbull_c());
        assert!(higher_hf > gamma, "a higher heat of fusion raises gamma_sl");
        let larger_atom =
            interfacial_energy(Fixed::from_int(30), iron_heat_of_fusion(), turnbull_c());
        assert!(larger_atom < gamma, "a larger atomic area lowers gamma_sl");
        // Guards: no volume, no enthalpy, or no coefficient, no interfacial energy.
        assert_eq!(
            interfacial_energy(ZERO, iron_heat_of_fusion(), turnbull_c()),
            ZERO
        );
        assert_eq!(
            interfacial_energy(iron_atomic_volume(), ZERO, turnbull_c()),
            ZERO
        );
        assert_eq!(
            interfacial_energy(iron_atomic_volume(), iron_heat_of_fusion(), ZERO),
            ZERO
        );
    }

    #[test]
    fn the_richards_ratio_derives_the_melt_entropy_from_the_measured_enthalpy() {
        // dS_f/R = dH_f/(R*T_m): iron 13.81/(0.008314*1811) = 13.81/15.06 ~ 0.917, the near-R Richards value for
        // a close-packed metal (the measured melt entropy, derived not reserved).
        let ratio = richards_ratio(
            iron_heat_of_fusion(),
            iron_melting_point(),
            r_kj_per_mol_k(),
        );
        assert!(
            close(ratio, 0.917, 0.02),
            "iron's derived dS_f/R ~0.92 (near Richards' R): {ratio:?}"
        );
        // A higher heat of fusion (or a lower melting point) raises the melt entropy ratio.
        assert!(
            richards_ratio(Fixed::from_int(30), iron_melting_point(), r_kj_per_mol_k()) > ratio
        );
        assert_eq!(
            richards_ratio(ZERO, iron_melting_point(), r_kj_per_mol_k()),
            ZERO,
            "no enthalpy: no melt entropy"
        );
    }

    #[test]
    fn the_barrier_falls_with_undercooling_and_the_rate_turns_on() {
        // The nucleation barrier over the kernel: at SHALLOW undercooling the reduced barrier is huge (nucleation
        // suppressed, the kernel's exp-window freeze-out), and it FALLS as undercooling deepens (the driving
        // force grows), turning the rate on. This is the physical nucleation behavior.
        let dsf = richards_ratio(
            iron_heat_of_fusion(),
            iron_melting_point(),
            r_kj_per_mol_k(),
        );
        let beta_gamma = reduced_interfacial_energy(dsf, turnbull_c());
        let prefactor = Fixed::ONE; // a normalized I_0 at the working scale

        // Shallow undercooling T = 0.8*T_m (~1449 K): the barrier is high, the rate freezes out to zero.
        let t_shallow = Fixed::from_int(1449);
        let dgv_shallow = reduced_driving_force(dsf, iron_melting_point(), t_shallow);
        let barrier_shallow =
            reduced_nucleation_barrier(beta_gamma, dgv_shallow, iron_melting_point(), t_shallow);
        let rate_shallow = nucleation_rate(prefactor, barrier_shallow);
        assert_eq!(
            rate_shallow, ZERO,
            "shallow undercooling: the barrier is too high, nucleation is suppressed"
        );

        // Deep undercooling T = 0.5*T_m (~905 K): the barrier falls inside the window, the rate turns on.
        let t_deep = Fixed::from_int(905);
        let dgv_deep = reduced_driving_force(dsf, iron_melting_point(), t_deep);
        let barrier_deep =
            reduced_nucleation_barrier(beta_gamma, dgv_deep, iron_melting_point(), t_deep);
        assert!(
            barrier_deep < barrier_shallow,
            "deeper undercooling lowers the nucleation barrier"
        );
        let rate_deep = nucleation_rate(prefactor, barrier_deep);
        assert!(
            rate_deep > ZERO && rate_deep <= prefactor,
            "deep undercooling: nucleation turns on (0 < rate <= I_0)"
        );

        // No driving force (at the melting point): infinite barrier, no rate.
        let dgv_none = reduced_driving_force(dsf, iron_melting_point(), iron_melting_point());
        assert_eq!(dgv_none, ZERO, "no undercooling at T_m: no driving force");
        assert_eq!(
            reduced_nucleation_barrier(
                beta_gamma,
                dgv_none,
                iron_melting_point(),
                iron_melting_point()
            ),
            Fixed::MAX,
            "no driving force: an infinite barrier"
        );
    }

    #[test]
    fn the_mechanism_carries_two_degrees_of_freedom() {
        // The build's prove-it catch, encoded: two (C, dS_f/R) splits with the SAME beta_gamma give DIFFERENT
        // barriers and critical radii, so the mechanism needs both the interface coefficient and the
        // melt entropy, not one folded value. beta_gamma = C*(dS_f/R) = 0.45*0.92 = 0.30*1.38 ~ 0.414.
        let tm = iron_melting_point();
        let t = Fixed::from_int(905); // deep undercooling so both barriers are finite
        let split_a_c = Fixed::from_ratio(45, 100);
        let split_a_dsf = Fixed::from_ratio(92, 100);
        let split_b_c = Fixed::from_ratio(30, 100);
        let split_b_dsf = Fixed::from_ratio(138, 100);
        let beta_a = reduced_interfacial_energy(split_a_dsf, split_a_c);
        let beta_b = reduced_interfacial_energy(split_b_dsf, split_b_c);
        // The two beta_gamma agree (the collapse the gamma_sl sub-result sees).
        assert!(
            close(beta_a, beta_b.to_f64_lossy(), 0.01),
            "same beta_gamma from both splits"
        );
        // But the barriers differ (the driving force re-exposes dS_f/R independently).
        let barrier_a =
            reduced_nucleation_barrier(beta_a, reduced_driving_force(split_a_dsf, tm, t), tm, t);
        let barrier_b =
            reduced_nucleation_barrier(beta_b, reduced_driving_force(split_b_dsf, tm, t), tm, t);
        assert!(
            !close(barrier_a, barrier_b.to_f64_lossy(), 0.5),
            "same beta_gamma, DIFFERENT barrier: the mechanism needs the second dof"
        );
        // The critical radius needs C alone (r*/a = 2C/(1 - T/T_m)), so the two splits differ there too.
        let r_a = critical_radius_over_spacing(beta_a, reduced_driving_force(split_a_dsf, tm, t));
        let r_b = critical_radius_over_spacing(beta_b, reduced_driving_force(split_b_dsf, tm, t));
        assert!(
            !close(r_a, r_b.to_f64_lossy(), 0.1),
            "the critical radius needs C independently of beta_gamma"
        );
    }

    #[test]
    fn the_zeldovich_factor_and_prefactor_compose() {
        // Z = sqrt(reduced_barrier/(3*pi))/n*, a small positive factor for a physical barrier and a many-atom
        // critical nucleus. n* from r*/a; a representative deep-undercooling case.
        let dsf = richards_ratio(
            iron_heat_of_fusion(),
            iron_melting_point(),
            r_kj_per_mol_k(),
        );
        let beta_gamma = reduced_interfacial_energy(dsf, turnbull_c());
        let t = Fixed::from_int(905);
        let dgv = reduced_driving_force(dsf, iron_melting_point(), t);
        let barrier = reduced_nucleation_barrier(beta_gamma, dgv, iron_melting_point(), t);
        let r_star = critical_radius_over_spacing(beta_gamma, dgv);
        let n_star = critical_atom_count(r_star);
        assert!(
            n_star > Fixed::ONE,
            "the critical nucleus holds several atoms"
        );
        let z = zeldovich_factor(barrier, n_star);
        assert!(
            z > ZERO && z < Fixed::ONE,
            "the Zeldovich factor is a small positive fraction"
        );
        // I_0 = N_s * Z * beta* composes (all caller-supplied from built sources); a bigger density or faster
        // attachment raises it, and it reserves nothing.
        let i0 = nucleation_prefactor(Fixed::from_int(100), z, Fixed::from_int(30));
        assert!(i0 > ZERO, "the prefactor composes from built quantities");
        assert!(
            nucleation_prefactor(Fixed::from_int(200), z, Fixed::from_int(30)) > i0,
            "a higher site density raises I_0"
        );
        assert_eq!(
            nucleation_prefactor(ZERO, z, Fixed::from_int(30)),
            ZERO,
            "no sites: no prefactor"
        );
    }

    #[test]
    fn the_grain_size_balances_nucleation_and_growth() {
        // d ~ (G/I)^(1/4): faster nucleation gives smaller grains, faster growth gives larger grains.
        let base = avrami_grain_size(Fixed::from_int(16), Fixed::ONE);
        assert!(close(base, 2.0, 0.001), "(16/1)^(1/4) = 2");
        let faster_nucleation = avrami_grain_size(Fixed::from_int(16), Fixed::from_int(16));
        assert!(
            faster_nucleation < base,
            "faster nucleation gives smaller grains"
        );
        let faster_growth = avrami_grain_size(Fixed::from_int(256), Fixed::ONE);
        assert!(faster_growth > base, "faster growth gives larger grains");
        assert_eq!(
            avrami_grain_size(Fixed::from_int(16), ZERO),
            ZERO,
            "no nucleation: no grains"
        );
    }

    #[test]
    fn the_nucleation_core_is_deterministic() {
        // Principle 3: the same inputs return the same bits across the whole chain.
        let dsf = richards_ratio(
            iron_heat_of_fusion(),
            iron_melting_point(),
            r_kj_per_mol_k(),
        );
        let beta_gamma = reduced_interfacial_energy(dsf, turnbull_c());
        let t = Fixed::from_int(905);
        let dgv = reduced_driving_force(dsf, iron_melting_point(), t);
        let barrier_1 = reduced_nucleation_barrier(beta_gamma, dgv, iron_melting_point(), t);
        let barrier_2 = reduced_nucleation_barrier(beta_gamma, dgv, iron_melting_point(), t);
        assert_eq!(barrier_1, barrier_2);
        assert_eq!(
            interfacial_energy(iron_atomic_volume(), iron_heat_of_fusion(), turnbull_c()),
            interfacial_energy(iron_atomic_volume(), iron_heat_of_fusion(), turnbull_c())
        );
    }
}
