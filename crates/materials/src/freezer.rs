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

//! Stage 5, the freezer: the realized assemblage as kinetics race the world's cooling rate, and the rate-law
//! kernel's first consumer. Its atom is the thermally-activated self-diffusivity `D = nu * exp(-E*/(R*T))`,
//! which is [`civsim_physics::laws::arrhenius_rate`] with `nu = c_s/a` the attempt-frequency prefactor and
//! `E*/(R*T)` the reduced barrier.
//!
//! The freezing/self-diffusion barrier `E*` is the DERIVE-FIRST Form B ruled on #187: `E* = f * E_coh`, the
//! per-class vacancy fraction of the already-built Rose cohesive energy, reused directly rather than routed
//! through the composite `g * R * T_m`. Because the barrier `Q = H_vf + H_vm` and the melting point both scale
//! with `E_coh`, the spec's `g` is the composite `g = k * f` (`k = E_coh/(R*T_m)` the cohesive-to-melting
//! ratio, `f` the vacancy fraction); Form B pulls `k` out and reuses the derived `E_coh`, one derivation hop
//! shorter and one correlation fewer. The vacancy fraction `f` is RESERVED-with-basis (never entered here): the
//! fraction of the cohesive energy spent forming plus moving the diffusion carrier (`H_vf + H_vm`), cited to
//! Brown & Ashby 1980 / Sherby & Simnad 1962, keyed off the material's bonding class, verified at the primary
//! source before entry. The derive-first win is REAL but PARTIAL: the barrier reads one per-class empirical
//! constant either way (the vacancy energetics are not floored; deriving `H_vf`/`H_vm` from the bonding would
//! drive `f` toward zero, the named follow-on). `R` (the molar gas constant) is DERIVED (`N_A * k_B`), not
//! authored, and the caller supplies it; the kernel sees only the dimensionless `E*/(R*T)`, blind to whether
//! the caller worked in molar (`R*T`) or per-particle (`k_B*T`) units.
//!
//! This module builds the barrier and the rate composition (the kernel consumed, `E_coh` reused) and the
//! DERIVED Lindemann melting point `T_m` ([`debye_melting_point`]). The `T_m` derivation is NOT gated on the
//! fractional-power primitive (task #45): a prove-it check found the Lindemann `^(2/3)` is `cbrt^2` over the
//! built exact `cbrt`, and the Lindemann-Gilvarry chain collapses algebraically to `T_m ~ B_0 * V_atom`, so it
//! is buildable now and is alien-general (any substance with EOS anchors derives its own `T_m`, no cited
//! melting point). It also builds the attempt frequency `nu = c_s/a` ([`attempt_frequency_per_ps`] over
//! [`sound_speed_km_per_s`]) closing the `D0 ~ a^2 * nu` normalization, and the Frost-Ashby creep axis `T/T_m`
//! ([`homologous_temperature`]); the consistency twin (`g*R*T_m` and `f*E_coh` agree within class scatter)
//! lives in `tests/freezer_consistency.rs`. The remaining pieces are the end-to-end route diffusivity (wiring
//! `nu` from the anchors, which needs the atomic mass), the Dodson closure temperature (a freezer-side root-find
//! around the kernel), and the `theta_D` sibling (built only when its S_vib / Debye-Cp consumer arrives, not by
//! unwinding the `T_m` collapse). The sub-kT polymorph terminal resolves by the derived `kT` boundary, never a
//! reserved threshold (the Gap-Law discipline: the resolution boundary is a physical quantity).

use civsim_core::Fixed;
use civsim_physics::laws;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::rose_eos;

use crate::metallic::MetallicRoute;

const ZERO: Fixed = Fixed::ZERO;

/// The self-diffusion / freezing barrier `E* = f * E_coh` (Form B, gate-ruled #187): the per-class vacancy
/// fraction `f` of the cohesive energy `E_coh`, in the cohesive energy's own units (kJ/mol for the metallic
/// route). `f` is RESERVED-with-basis (`H_vf + H_vm` as a fraction of `E_coh`, cited Brown & Ashby 1980 /
/// Sherby & Simnad 1962, per bonding class); the caller supplies it from the reserved data, so no value is
/// planted here. Guards non-positive inputs (no cohesion or no fraction: no barrier); an overflowing product
/// saturates to [`Fixed::MAX`] (an insurmountable barrier, which the rate reads as the frozen regime).
pub fn diffusion_barrier(cohesive_energy: Fixed, vacancy_fraction: Fixed) -> Fixed {
    if cohesive_energy <= ZERO || vacancy_fraction <= ZERO {
        return ZERO;
    }
    cohesive_energy
        .checked_mul(vacancy_fraction)
        .unwrap_or(Fixed::MAX)
}

/// The self-diffusivity `D = nu * exp(-E*/(R*T))` over the rate-law kernel ([`laws::arrhenius_rate`]), closing
/// the canonical `D0 ~ a^2 * nu`. The barrier is Form B ([`diffusion_barrier`]); the reduced barrier `E*/(R*T)`
/// is formed by [`laws::reduced_barrier`] at the MOLAR scale (`E_coh` in kJ/mol, `R*T` in kJ/mol, the kernel
/// blind to the scale so no `R = N_A*k_B` composite drift enters), and `nu = c_s/a` is the caller's attempt
/// frequency, supplied at a working scale whose value is representable (the SI attempt frequency `~1e13 Hz`
/// overflows Q32.32, the same fold the Eyring prefactor documents). `R` is the DERIVED molar gas constant. The
/// rate freezes out below about `0.77 * T_m` (the kernel's honest exp-window limit at `E*/(R*T) > 22`), the
/// physical freeze-out. A non-positive thermal scale collapses the rate to zero. Deterministic fixed-point.
pub fn self_diffusivity(
    attempt_frequency: Fixed,
    cohesive_energy: Fixed,
    vacancy_fraction: Fixed,
    gas_constant: Fixed,
    temperature: Fixed,
) -> Fixed {
    let e_star = diffusion_barrier(cohesive_energy, vacancy_fraction);
    let thermal = match gas_constant.checked_mul(temperature) {
        Some(rt) if rt > ZERO => rt,
        _ => return ZERO, // no thermal scale (or an overflowing one): no crossing
    };
    let reduced = laws::reduced_barrier(e_star, thermal);
    laws::arrhenius_rate(attempt_frequency, reduced)
}

/// The derived Lindemann melting temperature `T_m` (K), the algebraic collapse of the Lindemann-Gilvarry
/// criterion when the Debye temperature is taken from the bulk sound speed `c_s = sqrt(B_0/rho)`. Walking the
/// chain `T_m = C_L*M*theta_D^2*V^(2/3)` with `theta_D = C_theta*c_s*n^(1/3)` and `c_s^2 = B_0*V_atom/M`, the
/// atomic mass `M` CANCELS and `V^(1/3)*V^(2/3) = V`, collapsing to
/// `T_m = delta^2 * [(6*pi^2)^(2/3)/9] * (B_0 * V_atom / k_B)`. So the melting point is the elastic energy per
/// atom `B_0 * V_atom` (a pressure times a volume, an energy) over the thermal scale, times the squared
/// Lindemann ratio and a pure-math factor. Every fractional power in the original chain is a built exact
/// `sqrt`/`cbrt`/integer power (no fractional-power primitive), so this is buildable now and admits the alien:
/// any substance with EOS anchors derives its own `T_m` from its own bond-strength physics, no cited melting
/// point needed.
///
/// The Lindemann ratio `delta` (the critical vibrational amplitude as a fraction of the interatomic distance,
/// famously near 0.1) is RESERVED-with-basis, per bonding class `[E]`, keyed off the material's class, verified
/// at the primary source before entry; the caller supplies it, so no value is planted. `[(6*pi^2)^(2/3)/9]` is
/// derived here from `Fixed::PI` and the exact `cbrt`. The `k_B` fold (`10^-21 J / k_B`, mapping `GPa*A^3` to
/// kelvin) is the exact rational `10^8 / 1380649` from the SI-exact Boltzmann constant, folded once at the
/// atomic scale (the raw `10^-21`/`k_B` underflows Q32.32, the same fold `nernst_emf` and the Eyring prefactor
/// use). HONEST LIMIT: with only the bulk modulus among the anchors (no shear modulus), the Debye velocity is
/// approximated by the bulk sound speed, so the transverse modes are folded into `delta`; a shear-aware Debye
/// average is the follow-on when the elastic anchors carry `G`. Non-positive inputs (no elastic scale, no
/// volume, or no ratio) yield zero (no melting point).
pub fn debye_melting_point(
    bulk_modulus_gpa: Fixed,
    atomic_volume_angstrom3: Fixed,
    lindemann_ratio: Fixed,
) -> Fixed {
    if bulk_modulus_gpa <= ZERO || atomic_volume_angstrom3 <= ZERO || lindemann_ratio <= ZERO {
        return ZERO;
    }
    // The elastic energy per atom in the atomic-scale unit (GPa*A^3).
    let elastic = match bulk_modulus_gpa.checked_mul(atomic_volume_angstrom3) {
        Some(e) => e,
        None => return Fixed::MAX,
    };
    let delta_sq = lindemann_ratio.powi(2);
    // delta^2 * K_num * K_fold * (B_0 * V_atom): the big factors (K_num*K_fold*elastic ~ 1e5 K) then the
    // delta^2 ~ 1e-2 scaling, so no intermediate overflows for physical inputs.
    elastic
        .checked_mul(lindemann_numeric_factor())
        .and_then(|x| x.checked_mul(kb_fold_gpa_angstrom3_to_kelvin()))
        .and_then(|x| x.checked_mul(delta_sq))
        .unwrap_or(Fixed::MAX)
}

/// The pure-math Lindemann-Gilvarry collapse factor `(6*pi^2)^(2/3)/9`, derived from `Fixed::PI` and the exact
/// `cbrt` (no authored decimal): `6*pi^2 ~ 59.22`, its cube root squared `~15.20`, over nine `~1.689`.
fn lindemann_numeric_factor() -> Fixed {
    let six_pi_sq = Fixed::from_int(6)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_mul(Fixed::PI))
        .unwrap_or(ZERO);
    // (6*pi^2)^(2/3) = cbrt(6*pi^2)^2, both exact built ops.
    let two_thirds_power = six_pi_sq.cbrt().powi(2);
    two_thirds_power
        .checked_div(Fixed::from_int(9))
        .unwrap_or(ZERO)
}

/// The `k_B` fold mapping the atomic-scale elastic energy `B_0[GPa]*V_atom[A^3]` (`= 10^-21 J`) to kelvin:
/// `10^-21 / k_B`. With the SI-exact `k_B = 1.380649e-23 J/K`, this is the exact rational `10^8 / 1380649
/// ~ 72.43 K/(GPa*A^3)`, folded once at the cited atomic scale (the raw `10^-21`/`k_B` underflows Q32.32).
fn kb_fold_gpa_angstrom3_to_kelvin() -> Fixed {
    Fixed::from_ratio(100_000_000, 1_380_649)
}

/// The bulk sound speed `c_s = sqrt(B_0/rho)` in km/s, from the bulk modulus (GPa) and the density (g/cm^3).
/// The unit fold is exact and needs no constant: `sqrt(GPa / (g/cm^3)) = sqrt(10^9 Pa / 10^3 kg.m^-3) =
/// sqrt(10^6 m^2/s^2) = 10^3 m/s = 1 km/s`, so the square root of `B_0[GPa]/rho[g/cm^3]` is already in km/s
/// (the atomic-scale working unit, representable where the SI `~5000 m/s` also fits but the elastic modulus in
/// pascals would not). Non-positive inputs (no elastic scale or no mass) yield zero.
pub fn sound_speed_km_per_s(bulk_modulus_gpa: Fixed, density_g_per_cm3: Fixed) -> Fixed {
    if bulk_modulus_gpa <= ZERO || density_g_per_cm3 <= ZERO {
        return ZERO;
    }
    match bulk_modulus_gpa.checked_div(density_g_per_cm3) {
        Some(ratio) => ratio.sqrt(),
        None => ZERO,
    }
}

/// The attempt frequency `nu = c_s/a` in inverse picoseconds, from the sound speed (km/s) and the interatomic
/// spacing (angstrom). The working unit is `/ps` because the SI attempt frequency (`c_s/a ~ 10^13 Hz`)
/// overflows Q32.32; the fold is the exact unit constant `1 km/s = 10 A/ps` (`10^3 m/s = 10^13 A/s = 10 A/ps`),
/// so `nu[/ps] = 10 * c_s[km/s] / a[A]`. This is the prefactor [`self_diffusivity`] takes, closing the
/// canonical `D0 ~ a^2 * nu`. The spacing `a` is the material's own characteristic atomic length (the
/// Wigner-Seitz radius from the molar volume); the order-unity choice of length folds into the order-of-
/// magnitude `D0`, which the spec states approximately (`~10^-5 m^2/s`). Non-positive inputs yield zero.
pub fn attempt_frequency_per_ps(sound_speed_km_per_s: Fixed, spacing_angstrom: Fixed) -> Fixed {
    if sound_speed_km_per_s <= ZERO || spacing_angstrom <= ZERO {
        return ZERO;
    }
    // nu = 10 * c_s / a; the 10 is the exact km/s -> A/ps unit conversion, not an authored value.
    match sound_speed_km_per_s
        .checked_mul(Fixed::from_int(10))
        .and_then(|x| x.checked_div(spacing_angstrom))
    {
        Some(nu) => nu,
        None => Fixed::MAX,
    }
}

/// The homologous temperature `T/T_m` (dimensionless), the Frost-Ashby deformation-map axis over which creep
/// mechanisms are universal across substances. A material at a given fraction of its own melting point sits at
/// the same point on the map regardless of the absolute temperatures, which is why the axis serves every
/// substance real or invented. Clamped non-negative; a non-positive melting point (no derived `T_m`) yields
/// zero (the map is undefined without a melting scale).
pub fn homologous_temperature(temperature: Fixed, melting_point: Fixed) -> Fixed {
    if melting_point <= ZERO {
        return ZERO;
    }
    match temperature.max(ZERO).checked_div(melting_point) {
        Some(ratio) => ratio,
        None => Fixed::MAX,
    }
}

/// The freezer route bound to the metallic route and the EOS anchors, so the Form-B barrier reads the derived
/// `E_coh` and the Lindemann `T_m` reads the anchors' `B_0` and `V_m`, all for an anchored metal. The reserved
/// vacancy fraction `f` and Lindemann ratio `delta` are supplied by the caller, never planted, so the route
/// reuses the substrate's derived quantities without entering a value.
pub struct FreezerRoute<'a> {
    metallic: &'a MetallicRoute<'a>,
    anchors: &'a MetalEosAnchors,
}

impl<'a> FreezerRoute<'a> {
    /// Bind the freezer to the metallic route (the source of the derived `E_coh`) and the EOS anchors (`B_0`,
    /// `V_m`).
    pub fn new(metallic: &'a MetallicRoute<'a>, anchors: &'a MetalEosAnchors) -> Self {
        FreezerRoute { metallic, anchors }
    }

    /// The Form-B barrier `E* = f * E_coh` for an anchored metal, or `None` (escalate) when the metal carries
    /// no banked cohesive energy. `f` (the reserved vacancy fraction) is the caller's, so this reuses the
    /// derived `E_coh` without planting a value.
    pub fn barrier(&self, symbol: &str, vacancy_fraction: Fixed) -> Option<Fixed> {
        let e_coh = self.metallic.cohesive_energy(symbol)?;
        Some(diffusion_barrier(e_coh, vacancy_fraction))
    }

    /// The derived Lindemann melting point `T_m` (K) for an anchored metal, from its EOS anchors (`B_0`, `V_m`)
    /// and the reserved per-class Lindemann ratio, or `None` (escalate) when the metal carries no anchors.
    /// Reuses the built `cm^3/mol -> A^3/atom` converter for `V_atom`. `delta` is the caller's reserved value,
    /// never planted.
    pub fn melting_point(&self, symbol: &str, lindemann_ratio: Fixed) -> Option<Fixed> {
        let b0 = self.anchors.bulk_modulus_gpa(symbol)?;
        let v_m = self.anchors.molar_volume(symbol)?;
        let v_atom = v_m.checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())?;
        Some(debye_melting_point(b0, v_atom, lindemann_ratio))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metallic::MetallicRoute;
    use civsim_physics::metal_eos::MetalEosAnchors;
    use civsim_physics::periodic::PeriodicTable;

    // Test fixtures, clearly test-only and NOT canonical entries: the reserved vacancy fraction `f` is
    // exercised at its basis value (`g ~ 17-18` implies `f ~ 0.55`, `H_vf ~ 0.3 E_coh`), and `R` at the
    // derived molar gas constant in kJ/(mol K). The owner enters the canonical `f` per class after primary
    // verification; these fixtures only exercise the mechanism.
    fn f_fixture() -> Fixed {
        Fixed::from_ratio(55, 100) // f ~ 0.55, the sanity-check basis value (test-only)
    }
    fn r_kj_per_mol_k() -> Fixed {
        Fixed::from_ratio(8314, 1_000_000) // R = 8.314e-3 kJ/(mol K), derived (N_A*k_B)
    }
    fn delta_fixture() -> Fixed {
        Fixed::from_ratio(9, 100) // the Lindemann ratio delta ~ 0.09 (test-only, not a canonical entry)
    }
    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn diffusion_barrier_is_the_vacancy_fraction_of_the_cohesive_energy() {
        // E* = f * E_coh: 0.55 * 400 ~ 220 (f is not a dyadic rational, so check within tolerance).
        let e_coh = Fixed::from_int(400);
        let e_star = diffusion_barrier(e_coh, f_fixture());
        assert!(
            close(e_star, 220.0, 0.01),
            "the barrier is the vacancy fraction of the cohesive energy: {e_star:?}"
        );
        // Monotone: a deeper cohesive well is a higher barrier (same f).
        let deeper = diffusion_barrier(Fixed::from_int(500), f_fixture());
        assert!(deeper > e_star, "a deeper cohesive well raises the barrier");
        // The absence convention: no cohesion or no fraction, no barrier (self-gating).
        assert_eq!(
            diffusion_barrier(ZERO, f_fixture()),
            ZERO,
            "no cohesive energy: no barrier"
        );
        assert_eq!(
            diffusion_barrier(e_coh, ZERO),
            ZERO,
            "no vacancy fraction: no barrier"
        );
        // Deterministic (Principle 3).
        assert_eq!(e_star, diffusion_barrier(e_coh, f_fixture()));
    }

    #[test]
    fn self_diffusivity_freezes_out_cold_and_rises_with_temperature() {
        // Iron-scale cohesion (E_coh ~ 400 kJ/mol), a normalized attempt frequency (nu ~ 1 at the working
        // scale). E* = 0.55 * 400 = 220 kJ/mol; reduced = 220/(R*T). At T = 1000 K, reduced = 220/8.314 = 26.5
        // > 22, so the rate underflows to zero: the frozen regime (T well below the ~1811 K melting point). At
        // T = 2200 K, reduced = 220/18.29 = 12.0, inside the window, so the rate is positive.
        let e_coh = Fixed::from_int(400);
        let nu = Fixed::ONE;
        let cold = self_diffusivity(
            nu,
            e_coh,
            f_fixture(),
            r_kj_per_mol_k(),
            Fixed::from_int(1000),
        );
        assert_eq!(
            cold, ZERO,
            "cold: the barrier is unresolvable, the rate freezes out"
        );
        let hot = self_diffusivity(
            nu,
            e_coh,
            f_fixture(),
            r_kj_per_mol_k(),
            Fixed::from_int(2200),
        );
        assert!(hot > ZERO && hot < nu, "hot: 0 < rate < attempt frequency");
        // Monotone in temperature: hotter diffuses faster (a lower reduced barrier).
        let hotter = self_diffusivity(
            nu,
            e_coh,
            f_fixture(),
            r_kj_per_mol_k(),
            Fixed::from_int(2500),
        );
        assert!(hotter > hot, "the diffusivity rises with temperature");
        // No thermal scale, no crossing.
        assert_eq!(
            self_diffusivity(nu, e_coh, f_fixture(), r_kj_per_mol_k(), ZERO),
            ZERO,
            "no thermal scale: no diffusion"
        );
        // No attempts, no rate.
        assert_eq!(
            self_diffusivity(
                ZERO,
                e_coh,
                f_fixture(),
                r_kj_per_mol_k(),
                Fixed::from_int(2200)
            ),
            ZERO,
            "no attempt frequency: no diffusion"
        );
        // Deterministic (Principle 3).
        assert_eq!(
            hot,
            self_diffusivity(
                nu,
                e_coh,
                f_fixture(),
                r_kj_per_mol_k(),
                Fixed::from_int(2200)
            )
        );
    }

    #[test]
    fn the_freezer_route_reads_the_derived_cohesive_energy() {
        // The route reuses the built Rose E_coh: the barrier for an anchored metal is f times its cohesive
        // energy, so it tracks the metal's own derived well depth. Fe (deeper cohesion) has a higher barrier
        // than Na (shallow), the derive-first substrate reuse the fork was about.
        let table = PeriodicTable::standard().expect("periodic table");
        let anchors = MetalEosAnchors::standard().expect("metal EOS anchors");
        let metallic = MetallicRoute::new(&table, &anchors);
        let freezer = FreezerRoute::new(&metallic, &anchors);

        let fe = freezer.barrier("Fe", f_fixture()).expect("Fe barrier");
        let na = freezer.barrier("Na", f_fixture()).expect("Na barrier");
        assert!(
            fe > na && na > ZERO,
            "the barrier tracks the metal's own derived cohesive energy (Fe deeper than Na)"
        );
        // The barrier equals f times the route's own cohesive energy (the reuse is exact, no re-derivation).
        let fe_coh = metallic.cohesive_energy("Fe").expect("Fe E_coh");
        assert_eq!(
            fe,
            diffusion_barrier(fe_coh, f_fixture()),
            "the route barrier is f * E_coh over the built cohesive energy"
        );
        // A metal with no banked cohesive energy escalates (the honest refusal the metallic route already gives).
        assert!(
            freezer.barrier("Xx", f_fixture()).is_none(),
            "an unanchored symbol escalates rather than fabricating a barrier"
        );
    }

    #[test]
    fn debye_melting_point_recovers_a_real_melting_point_at_the_lindemann_ratio() {
        // The collapse and the k_B fold are validated by feeding the LITERATURE Lindemann ratio and recovering
        // a cited melting point (used here only as a test reference, never entered into the mechanism). Iron:
        // B_0 = 170 GPa, V_m = 7.09 cm^3/mol -> V_atom ~ 11.77 A^3, and delta ~ 0.086 gives T_m ~ 1811 K (the
        // measured value). If the pure-math factor or the k_B fold were wrong, the literature delta would not
        // recover T_m, so this pins both derived constants numerically.
        let b0_fe = Fixed::from_int(170);
        let v_atom_fe = Fixed::from_ratio(709, 100)
            .checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())
            .expect("Fe atomic volume");
        let t_m_fe = debye_melting_point(b0_fe, v_atom_fe, Fixed::from_ratio(86, 1000));
        assert!(
            close(t_m_fe, 1811.0, 60.0),
            "the Lindemann ratio ~0.086 recovers iron's measured melting point ~1811 K: {t_m_fe:?}"
        );
        // T_m rises with the elastic energy per atom B_0 * V_atom (the collapse's content): a softer material of
        // the same volume melts lower at the same delta.
        let softer =
            debye_melting_point(Fixed::from_int(80), v_atom_fe, Fixed::from_ratio(86, 1000));
        assert!(
            softer < t_m_fe && softer > ZERO,
            "a softer material melts lower"
        );
        // Guards: no elastic scale, no volume, or no ratio, no melting point.
        assert_eq!(debye_melting_point(ZERO, v_atom_fe, delta_fixture()), ZERO);
        assert_eq!(debye_melting_point(b0_fe, ZERO, delta_fixture()), ZERO);
        assert_eq!(debye_melting_point(b0_fe, v_atom_fe, ZERO), ZERO);
        // Deterministic (Principle 3).
        assert_eq!(
            t_m_fe,
            debye_melting_point(b0_fe, v_atom_fe, Fixed::from_ratio(86, 1000))
        );
    }

    #[test]
    fn the_freezer_route_derives_the_melting_point_from_the_anchors() {
        // The route reads B_0 and V_m from the EOS anchors and reuses the built cm^3/mol -> A^3 converter, so a
        // stiffer, larger-volume metal melts higher at the same reserved delta. Fe (high B_0 * V_atom) melts
        // well above Na (low), tracking the elastic energy per atom, with delta the caller's reserved value.
        let table = PeriodicTable::standard().expect("periodic table");
        let anchors = MetalEosAnchors::standard().expect("metal EOS anchors");
        let metallic = MetallicRoute::new(&table, &anchors);
        let freezer = FreezerRoute::new(&metallic, &anchors);

        let fe = freezer
            .melting_point("Fe", delta_fixture())
            .expect("Fe melting point");
        let na = freezer
            .melting_point("Na", delta_fixture())
            .expect("Na melting point");
        assert!(
            fe > na && na > ZERO,
            "T_m tracks the elastic energy per atom (Fe melts above Na)"
        );
        // The route melting point equals the pure function over the converted anchors (exact reuse).
        let v_atom_fe = anchors
            .molar_volume("Fe")
            .expect("Fe V_m")
            .checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())
            .expect("Fe V_atom");
        assert_eq!(
            fe,
            debye_melting_point(
                anchors.bulk_modulus_gpa("Fe").expect("Fe B_0"),
                v_atom_fe,
                delta_fixture()
            ),
            "the route T_m is the Lindemann collapse over the anchors' B_0 and V_atom"
        );
        // An unanchored symbol escalates rather than fabricating a melting point.
        assert!(
            freezer.melting_point("Xx", delta_fixture()).is_none(),
            "an unanchored symbol escalates"
        );
    }

    #[test]
    fn the_attempt_frequency_and_creep_axis_derive_from_the_anchors() {
        // Bulk sound speed c_s = sqrt(B_0/rho) in km/s (the unit fold is exact): iron B_0 = 170 GPa, rho ~ 7.87
        // g/cm^3 -> c_s ~ 4.65 km/s. This is the BULK sound speed; the longitudinal wave, carrying the shear
        // stiffness too, runs faster (~5.9 km/s), so the B_0-only value is the bulk one, the documented limit.
        let c_s = sound_speed_km_per_s(Fixed::from_int(170), Fixed::from_ratio(787, 100));
        assert!(
            close(c_s, 4.648, 0.02),
            "iron bulk sound speed ~4.65 km/s: {c_s:?}"
        );
        // A stiffer material at the same density is faster; a denser one at the same modulus is slower.
        assert!(sound_speed_km_per_s(Fixed::from_int(340), Fixed::from_ratio(787, 100)) > c_s);
        assert!(sound_speed_km_per_s(Fixed::from_int(170), Fixed::from_int(16)) < c_s);
        assert_eq!(
            sound_speed_km_per_s(ZERO, Fixed::ONE),
            ZERO,
            "no elastic scale: no sound speed"
        );

        // nu = 10 * c_s / a in /ps: iron a ~ 1.41 A (Wigner-Seitz) -> nu ~ 33 /ps (the SI ~3.3e13 Hz overflows
        // Q32.32, hence the /ps working unit; the 10 is the exact km/s -> A/ps unit conversion).
        let nu = attempt_frequency_per_ps(c_s, Fixed::from_ratio(141, 100));
        assert!(
            close(nu, 32.96, 0.3),
            "iron attempt frequency ~33 /ps: {nu:?}"
        );
        // A shorter spacing (a denser lattice) attempts faster.
        assert!(attempt_frequency_per_ps(c_s, Fixed::ONE) > nu);
        assert_eq!(
            attempt_frequency_per_ps(ZERO, Fixed::ONE),
            ZERO,
            "no sound speed: no attempts"
        );

        // The homologous temperature T/T_m: a material at half its melting point sits at 0.5 on the Frost-Ashby
        // map regardless of the absolute scale (universal across substances).
        assert!(close(
            homologous_temperature(Fixed::from_int(900), Fixed::from_int(1800)),
            0.5,
            0.001
        ));
        assert_eq!(
            homologous_temperature(Fixed::from_int(300), ZERO),
            ZERO,
            "no melting scale: the map is undefined"
        );
        // Deterministic (Principle 3).
        assert_eq!(
            nu,
            attempt_frequency_per_ps(c_s, Fixed::from_ratio(141, 100))
        );
    }
}
