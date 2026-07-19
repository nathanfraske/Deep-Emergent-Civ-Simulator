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

//! Stage 6, property emission: the measurable properties a realized assemblage emits, each DERIVED from the
//! floor and the realized state, never authored, so an alien material emits its own properties from its own
//! data (`docs/working/STAGE6_PROPERTY_EMISSION_DESIGN.md`, gate-ruled on #189).
//!
//! This is the first core slice, the two pieces the thermal properties rest on and reserve no value:
//!
//! - [`density_g_per_cm3`]: `rho = M / V_m`, the molar mass (periodic table) over the anchored molar volume, a
//!   pure ratio of floor data.
//! - [`debye_temperature`]: the Debye temperature `Theta_D = (h_bar/k_B) * c_s * (6*pi^2*n)^(1/3)`, reusing the
//!   freezer's built sound speed `c_s = sqrt(B_0/rho)` and the number density `n = 1/V_atom`, with the Planck-
//!   Boltzmann fold `h/k_B` an exact SI ratio and the `(6*pi^2)^(1/3)/(2*pi)` a `PI`-and-`cbrt` pure-math factor.
//!   This is the `theta_D` SIBLING the freezer deferred (`crates/materials/src/freezer.rs`: "built only when its
//!   S_vib / Debye-Cp consumer arrives"): Stage 6's Slack conductivity, Grueneisen expansion, and Debye heat
//!   capacity are that consumer, so it is built now, reserving no value beyond the exact unit fold.
//!
//! The Debye temperature has TWO paths. The BULK path ([`debye_temperature`] / [`PropertyRoute::debye_temperature`])
//! uses only `c_s = sqrt(B_0/rho)`, so with no shear modulus it OVERESTIMATES by roughly 30 percent (iron ~609 K
//! from the bulk speed against a measured ~470 K), the bulk-elastic approximation the Lindemann `T_m` also
//! carries. The SHEAR-AWARE path ([`debye_velocity_km_per_s`] / [`PropertyRoute::debye_temperature_shear_aware`])
//! uses the Debye-averaged velocity `v_D` over the longitudinal and transverse modes, which needs the shear
//! modulus `G = k*B_0` (through the moduli slice's reserved Pugh ratio `k`); fed the true `v_D`, iron's `Theta_D`
//! lands ~470 K. So the bulk-elastic limit is RETIRED for any metal with a Pugh ratio, the bulk path remaining the
//! fallback where `k` is unknown. The Slack conductivity downstream (which scales as `Theta_D^3`) should read the
//! shear-aware path. Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.
//!
//! The DEBYE HEAT CAPACITY ([`debye_heat_capacity_j_per_mol_k`] over [`debye_function`]) is the first consumer of
//! the Debye temperature: `C_v = 3R * D(Theta_D/T)` per mole of atoms, the bounded Debye integral evaluated by a
//! capped composite-Simpson rule (the integrand rewritten to keep `e^x` in-window). It reserves no value: `R` is a
//! floor constant, `D` is pure math, and the atom count is data. It is `C_v` (constant volume); the small
//! `C_p - C_v` correction rides the thermal-expansion slice once the Grueneisen `alpha` is built.
//!
//! STRENGTH ([`theoretical_shear_strength_gpa`], [`operative_shear_strength_gpa`]) reads the shear modulus: the
//! Frenkel ideal `tau_th = G/(2*pi)` (no reserved value, the upper bound a real material approaches) scaled by a
//! per-class knock-down `in (0, 1]` (the ONE reserved coefficient, spanning `~1e-2` for a soft annealed metal to
//! `~1` for a covalent solid or a defect-free whisker) to the operative strength. The richer Hall-Petch
//! grain-size model is a flagged design-first follow-on.
//!
//! THERMAL EXPANSION ([`volumetric_thermal_expansion_per_k`], [`linear_thermal_expansion_per_k`]) is the Grueneisen
//! relation `alpha_V = gamma_G * C_v / (K * V_m)` over the built `C_v`, the anchored `K`/`V_m`, and the ONE reserved
//! Grueneisen `gamma_G` (shared with the Slack conductivity, one hunt for both). Iron lands `alpha_L ~ 1.0e-5 /K`.
//!
//! SURFACE ENERGY ([`surface_energy_j_per_m2`]) is the broken-bond `gamma_sv = f_surf * E_coh_per_atom / A_atom`
//! over the built cohesive energy and atomic volume, reserving ONE per-class surface-bond fraction `f_surf`
//! (`~0.18`). This is the derivation the freezer REJECTED for the solid-liquid `gamma_sl`, landing where it belongs:
//! the solid-vapour surface energy. Iron lands `~2.4 J/m^2`. The GRAIN-BOUNDARY energy
//! ([`grain_boundary_energy_j_per_m2`]) is its sibling, `gamma_gb = r_gb * gamma_sv` reserving one per-class
//! grain-boundary-to-surface ratio `r_gb` (`~0.3`, high-angle), feeding grain growth and Hall-Petch.
//!
//! LATTICE THERMAL CONDUCTIVITY ([`lattice_thermal_conductivity_w_per_m_k`]) is the Slack model over the shear-aware
//! `Theta_D^3`, reusing the expansion's `gamma_G` and the CITED universal Slack constants, so it reserves NO new
//! coefficient. Its prefactor is FACTORED per the dimensionless-constant law (a fundamental-constant fold assembled
//! from the `k_B`/`hbar`/`amu` mantissas, times the pure-math `3*cbrt(4)/(20*pi^3)`, times the one cited phase-space
//! number `0.849`, times the `gamma`-correction), reassembly-tested both directions. It is ORDER-OF-MAGNITUDE
//! (within `~3x` for simple crystals, an upper bound for anharmonic ones like rutile) and is the LATTICE part only:
//! a metal's total conductivity is electronic-dominated (the deferred sub-arc).
//!
//! THERMAL DIFFUSIVITY ([`thermal_diffusivity_m2_per_s`]) is the pure composition `alpha = kappa / (rho * c_p) =
//! kappa * V_m / C_v` (the mass cancels), reserving NO value over the built conductivity, molar volume, and `C_v`.
//! It inherits the conductivity's order-of-magnitude and lattice-only reach.

use civsim_core::Fixed;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::rose_eos;

use crate::freezer;

const ZERO: Fixed = Fixed::ZERO;

/// The mass density `rho = M / V_m` (g/cm^3), the molar mass (g/mol) over the molar volume (cm^3/mol): a pure
/// ratio of floor data, no reserved value. This is the density the freezer's sound speed reads. Non-positive
/// inputs yield zero (no density without a mass and a volume).
// @derives: a phase's density <- molar mass + molar volume
pub fn density_g_per_cm3(molar_mass_g_per_mol: Fixed, molar_volume_cm3_per_mol: Fixed) -> Fixed {
    if molar_mass_g_per_mol <= ZERO || molar_volume_cm3_per_mol <= ZERO {
        return ZERO;
    }
    molar_mass_g_per_mol
        .checked_div(molar_volume_cm3_per_mol)
        .unwrap_or(Fixed::MAX)
}

/// The Debye-fold working constant `(h/k_B) * 10^13 * (6*pi^2)^(1/3) / (2*pi)`, mapping `c_s[km/s]` and
/// `V_atom[A^3]` to `Theta_D[K]`: the Planck-Boltzmann ratio `h/k_B` as the exact SI rational
/// `662607015 / 1380649` (from `h = 6.62607015e-34 J*s` and `k_B = 1.380649e-23 J/K`, times the `10^13` folding
/// the `km/s` and `A^-1` unit powers to per-kelvin), and `(6*pi^2)^(1/3)/(2*pi)` derived from `Fixed::PI` and the
/// built `cbrt`. About `297.7`. No authored decimal.
fn debye_fold() -> Fixed {
    // (h/k_B) * 10^13 = 662607015 / 1380649 ~ 479.924 (exact SI ratio at the working scale).
    let planck_boltzmann = Fixed::from_ratio(662_607_015, 1_380_649);
    // (6*pi^2)^(1/3) / (2*pi), the Debye-wavevector-over-h_bar pure-math factor.
    let six_pi_sq = Fixed::from_int(6)
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_mul(Fixed::PI))
        .unwrap_or(ZERO);
    let two_pi = Fixed::from_int(2).checked_mul(Fixed::PI).unwrap_or(ZERO);
    let math_factor = six_pi_sq.cbrt().checked_div(two_pi).unwrap_or(ZERO);
    planck_boltzmann
        .checked_mul(math_factor)
        .unwrap_or(Fixed::MAX)
}

/// The Debye temperature `Theta_D` (K) from the bulk sound speed and the atomic volume:
/// `Theta_D = (h_bar/k_B) * c_s * (6*pi^2*n)^(1/3)` with `n = 1/V_atom`, folded to
/// `Theta_D = debye_fold() * c_s / cbrt(V_atom)`. Reuses the freezer's built sound speed (`c_s = sqrt(B_0/rho)`,
/// km/s) and the atomic volume (A^3); `V_atom^(-1/3) = 1/cbrt(V_atom)` is the built exact op. Reserves no value
/// beyond the exact fold. See the module HONEST LIMIT: with the bulk sound speed (no shear modulus) this
/// OVERESTIMATES the shear-aware Debye temperature by roughly 30 percent, the bulk-elastic approximation the
/// Lindemann `T_m` also carries, refined when a shear modulus is anchored. Non-positive inputs yield zero.
// @derives: the Debye temperature <- sound speed + atomic volume
pub fn debye_temperature(sound_speed_km_per_s: Fixed, atomic_volume_angstrom3: Fixed) -> Fixed {
    if sound_speed_km_per_s <= ZERO || atomic_volume_angstrom3 <= ZERO {
        return ZERO;
    }
    let cube_root_v = atomic_volume_angstrom3.cbrt();
    if cube_root_v <= ZERO {
        return ZERO;
    }
    debye_fold()
        .checked_mul(sound_speed_km_per_s)
        .and_then(|x| x.checked_div(cube_root_v))
        .unwrap_or(Fixed::MAX)
}

/// The shear modulus `G` (GPa) `= k * K`, the Pugh modulus ratio times the anchored bulk modulus. The Pugh ratio
/// `k = G/K` is the ONE RESERVED-with-basis per-class coefficient of the elastic-and-hardness family (Pugh 1954;
/// the Chen-Tse 2011 hardness `k`), caller-supplied and never planted: `~0.5` for ductile metals (iron `~0.48`),
/// higher for brittle/covalent solids, per bonding class, primary-verified before entry. The isotropic bulk
/// modulus `K = B_0` alone cannot fix the shear stiffness (the derivation-hunt bottoms out here: `G/K` needs the
/// bonding directionality the volume-only Rose EOS does not carry), so `k` is the irreducible residual, and from
/// it `G`, `E`, and Poisson all derive. Non-positive inputs yield zero.
pub fn shear_modulus_gpa(bulk_modulus_gpa: Fixed, pugh_ratio: Fixed) -> Fixed {
    if bulk_modulus_gpa <= ZERO || pugh_ratio <= ZERO {
        return ZERO;
    }
    bulk_modulus_gpa
        .checked_mul(pugh_ratio)
        .unwrap_or(Fixed::MAX)
}

/// Young's modulus `E` (GPa) `= 9*K*G / (3*K + G)`, the isotropic elastic relation from the bulk and shear
/// moduli. No reserved value (a pure function of the two moduli). Non-positive inputs, or a degenerate zero
/// denominator, yield zero.
pub fn youngs_modulus_gpa(bulk_modulus_gpa: Fixed, shear_modulus_gpa: Fixed) -> Fixed {
    if bulk_modulus_gpa <= ZERO || shear_modulus_gpa <= ZERO {
        return ZERO;
    }
    let denom = Fixed::from_int(3)
        .checked_mul(bulk_modulus_gpa)
        .map(|x| x.saturating_add(shear_modulus_gpa));
    let num = Fixed::from_int(9)
        .checked_mul(bulk_modulus_gpa)
        .and_then(|x| x.checked_mul(shear_modulus_gpa));
    match (num, denom) {
        (Some(n), Some(d)) if d > ZERO => n.checked_div(d).unwrap_or(Fixed::MAX),
        _ => ZERO,
    }
}

/// Poisson's ratio `nu = (3*K - 2*G) / (2*(3*K + G))`, the isotropic elastic relation from the bulk and shear
/// moduli, the trivial companion the gate named. No reserved value. It can be negative (an auxetic solid) where
/// `G > 1.5*K`, which the signed difference carries; a degenerate zero denominator yields zero.
pub fn poisson_ratio(bulk_modulus_gpa: Fixed, shear_modulus_gpa: Fixed) -> Fixed {
    if bulk_modulus_gpa <= ZERO || shear_modulus_gpa <= ZERO {
        return ZERO;
    }
    let three_k = Fixed::from_int(3)
        .checked_mul(bulk_modulus_gpa)
        .unwrap_or(Fixed::MAX);
    let num = three_k.checked_sub(
        Fixed::from_int(2)
            .checked_mul(shear_modulus_gpa)
            .unwrap_or(Fixed::MAX),
    );
    let denom = Fixed::from_int(2).checked_mul(three_k.saturating_add(shear_modulus_gpa));
    match (num, denom) {
        (Some(n), Some(d)) if d > ZERO => n.checked_div(d).unwrap_or(Fixed::MAX),
        _ => ZERO,
    }
}

/// The intrinsic Vickers hardness `H_V` (GPa), the Chen-Tse (2011) correlation `H_V = 2*(k^2 * G)^0.585 - 3`,
/// where `k = G/K` is the Pugh ratio (the SAME reserved coefficient as the moduli, so hardness adds NO new
/// per-class value) and `G` the derived shear modulus. The `^0.585` is the built `Fixed::powf`. The form
/// constants `{2, 0.585, 3}` are the CITED Chen-Tse fitted parameters (universal, not per-class, not per-world),
/// verified at the primary source, an empirical moduli-to-hardness correlation. Clamped non-negative (the `-3`
/// drives a very soft solid to zero). HONEST LIMITS: this is the INTRINSIC (dislocation-free) hardness; the
/// operative hardness of a ductile metal is lower (dislocation plasticity), the strength knock-down the follow-on.
/// The correlation is validated on hard covalent solids (diamond lands ~95 GPa against ~96 measured, its home
/// turf) and is LESS accurate off that turf, in two named directions: (1) for soft low-`k` metals it OVERSTATES
/// (iron's intrinsic ~8 GPa against a much softer annealed ~1 GPa, the same low-`k` metallic-bonding case
/// Chen-Tse note), the intrinsic-versus-operative gap the strength slice's knock-down carries; (2) for IONIC
/// solids it OVERSTATES more strongly (rock-salt MgO ~23 GPa here against a measured ~9, NaCl ~2 GPa against
/// ~0.25), because ionic bonding admits easy dislocation glide on the ionic slip systems, decoupling the operative
/// hardness from the elastic moduli the correlation reads. So OFF the covalent domain (ductile-metal or ionic) the
/// emitted value is an intrinsic UPPER BOUND on the operative hardness, named as such rather than trusted as the
/// operative number: a moduli-to-hardness correlation cannot see the slip-system physics that softens those
/// classes. Non-positive inputs yield zero.
pub fn chen_tse_hardness_gpa(shear_modulus_gpa: Fixed, pugh_ratio: Fixed) -> Fixed {
    if shear_modulus_gpa <= ZERO || pugh_ratio <= ZERO {
        return ZERO;
    }
    // base = k^2 * G, formed with checked multiplies (not the wrapping powi), the checked-innermost discipline.
    let base = match pugh_ratio
        .checked_mul(pugh_ratio)
        .and_then(|k_sq| k_sq.checked_mul(shear_modulus_gpa))
    {
        Some(x) if x > ZERO => x,
        _ => return ZERO,
    };
    // ^0.585 (the cited Chen-Tse exponent) over the built powf; then 2*(..) - 3, clamped non-negative.
    let powered = base.powf(Fixed::from_ratio(585, 1000));
    Fixed::from_int(2)
        .checked_mul(powered)
        .and_then(|x| x.checked_sub(Fixed::from_int(3)))
        .map(|v| v.max(ZERO))
        .unwrap_or(ZERO)
}

/// The Debye-averaged sound velocity `v_D` (km/s), the shear-aware speed the Debye temperature properly uses:
/// `3/v_D^3 = 1/v_L^3 + 2/v_T^3` over the longitudinal `v_L = sqrt((K + 4G/3)/rho)` and the transverse
/// `v_T = sqrt(G/rho)` (the two transverse modes weighted twice). It reuses the freezer's `sqrt(modulus/rho) =
/// km/s` fold for each and the built `cbrt` for the average, and it needs the shear modulus `G` (so it retires
/// the bulk-elastic limit `debye_temperature` carries: fed the true `v_D`, the Debye temperature lands the
/// measured value rather than the ~30 percent bulk overestimate). The cubes are formed with checked multiplies,
/// not the wrapping `powi`. No reserved value beyond the moduli's Pugh ratio `k` (which `G` already carries).
/// Non-positive inputs yield zero.
pub fn debye_velocity_km_per_s(
    bulk_modulus_gpa: Fixed,
    shear_modulus_gpa: Fixed,
    density_g_per_cm3: Fixed,
) -> Fixed {
    if bulk_modulus_gpa <= ZERO || shear_modulus_gpa <= ZERO || density_g_per_cm3 <= ZERO {
        return ZERO;
    }
    // The P-wave (longitudinal) modulus M = K + 4G/3; v_L = sqrt(M/rho), v_T = sqrt(G/rho), both km/s.
    let four_g_thirds = match shear_modulus_gpa
        .checked_mul(Fixed::from_int(4))
        .and_then(|x| x.checked_div(Fixed::from_int(3)))
    {
        Some(x) => x,
        None => return ZERO,
    };
    let p_wave_modulus = bulk_modulus_gpa.saturating_add(four_g_thirds);
    let v_l = freezer::sound_speed_km_per_s(p_wave_modulus, density_g_per_cm3);
    let v_t = freezer::sound_speed_km_per_s(shear_modulus_gpa, density_g_per_cm3);
    if v_l <= ZERO || v_t <= ZERO {
        return ZERO;
    }
    // 3 / v_D^3 = 1/v_L^3 + 2/v_T^3, so v_D = cbrt(3 / (1/v_L^3 + 2/v_T^3)). Cubes via checked multiplies.
    let v_l3 = v_l.checked_mul(v_l).and_then(|x| x.checked_mul(v_l));
    let v_t3 = v_t.checked_mul(v_t).and_then(|x| x.checked_mul(v_t));
    let (vl3, vt3) = match (v_l3, v_t3) {
        (Some(a), Some(b)) if a > ZERO && b > ZERO => (a, b),
        _ => return ZERO,
    };
    let inv_sum = match (
        Fixed::ONE.checked_div(vl3),
        Fixed::from_int(2).checked_div(vt3),
    ) {
        (Some(a), Some(b)) => a.saturating_add(b),
        _ => return ZERO,
    };
    if inv_sum <= ZERO {
        return ZERO;
    }
    match Fixed::from_int(3).checked_div(inv_sum) {
        Some(x) if x > ZERO => x.cbrt(),
        _ => ZERO,
    }
}

/// The molar gas constant `R` (J/(mol*K)) as the exact CODATA value `N_A * k_B = 8.314462618 J/(mol*K)`, a
/// physics FLOOR constant (the same status as the Planck-Boltzmann fold `h/k_B` in [`debye_fold`]), NOT a reserved
/// value: `from_ratio(8314462618, 1000000000)`.
fn gas_constant_j_per_mol_k() -> Fixed {
    Fixed::from_ratio(8_314_462_618, 1_000_000_000)
}

/// The number of composite-Simpson intervals for the Debye integral over `[0, min(y, X_CAP)]`. A NUMERICAL
/// resolution (the algorithm matches the exact Debye function to five decimals by this count), not a physics
/// value; must stay EVEN for Simpson's rule.
const DEBYE_SIMPSON_INTERVALS: i32 = 64;

/// The Debye integrand `f(x) = x^4 * e^x / (e^x - 1)^2`, written in the algebraically-identical form
/// `x^4 / (e^x + e^-x - 2) = x^4 / (2*(cosh x - 1))`. The rewrite is load-bearing: it keeps `e^x` inside the
/// deterministic exp window and never forms the `(e^x - 1)^2` that overflows Q32.32 at large `x` (at `x = 20`,
/// `e^x ~ 4.85e8` is representable but its square is not). Zero at `x <= 0` (the smooth limit, `f ~ x^2` as
/// `x -> 0`), and zero where the denominator collapses below fixed-point resolution (an `x` so small its
/// contribution to the integral is negligible).
fn debye_integrand(x: Fixed) -> Fixed {
    if x <= ZERO {
        return ZERO;
    }
    let ex = x.exp();
    let emx = (-x).exp();
    // denom = e^x + e^-x - 2 = 2*(cosh x - 1), strictly positive for x > 0.
    let denom = match ex.saturating_add(emx).checked_sub(Fixed::from_int(2)) {
        Some(d) if d > ZERO => d,
        _ => return ZERO,
    };
    let x2 = match x.checked_mul(x) {
        Some(v) => v,
        None => return ZERO,
    };
    let x4 = match x2.checked_mul(x2) {
        Some(v) => v,
        None => return ZERO,
    };
    x4.checked_div(denom).unwrap_or(ZERO)
}

/// The Debye function `D(y) = (3/y^3) * integral_0^y  x^4 * e^x / (e^x - 1)^2 dx`, with `y = Theta_D / T`: the
/// temperature scaling of the Debye heat capacity (`C_v = 3nR * D(Theta_D/T)`). It runs from `D -> 1` as
/// `y -> 0` (the Dulong-Petit high-temperature limit) to `D -> (4*pi^4/5) / y^3` as `y -> infinity` (the
/// `T^3` low-temperature tail). The integral is a composite Simpson rule over `[0, min(y, X_CAP)]` with
/// `X_CAP = 20`, a NUMERICAL bound: beyond `x ~ 20` the integrand is below `2e-5` of the total AND `e^x` stays
/// inside the exp window, so the cap loses nothing and keeps the arithmetic in-range. Over the rewritten
/// [`debye_integrand`], this is a pure-math function: no reserved value, no physics constant. Non-positive `y`
/// yields `1` (the high-temperature limit).
pub fn debye_function(theta_over_t: Fixed) -> Fixed {
    if theta_over_t <= ZERO {
        return Fixed::ONE;
    }
    // X_CAP = 20: the numerical integration bound (see the doc). min(y, X_CAP) is the upper limit.
    let upper = theta_over_t.min(Fixed::from_int(20));
    let n = DEBYE_SIMPSON_INTERVALS;
    let h = match upper.checked_div(Fixed::from_int(n)) {
        Some(v) if v > ZERO => v,
        _ => return Fixed::ONE,
    };
    // Composite Simpson: integral ~ (h/3) * [f(0) + f(upper) + 4*sum(odd) + 2*sum(even)].
    let mut acc = debye_integrand(ZERO).saturating_add(debye_integrand(upper));
    let mut i = 1i32;
    while i < n {
        let x = h.checked_mul(Fixed::from_int(i)).unwrap_or(upper);
        let weight = if i % 2 == 1 { 4 } else { 2 };
        let term = debye_integrand(x)
            .checked_mul(Fixed::from_int(weight))
            .unwrap_or(ZERO);
        acc = acc.saturating_add(term);
        i += 1;
    }
    let integral = match acc
        .checked_mul(h)
        .and_then(|s| s.checked_div(Fixed::from_int(3)))
    {
        Some(v) => v,
        None => return ZERO,
    };
    // D = 3 * integral / y^3, the cube via checked multiplies.
    let y3 = match theta_over_t
        .checked_mul(theta_over_t)
        .and_then(|y2| y2.checked_mul(theta_over_t))
    {
        Some(v) if v > ZERO => v,
        // y so small its cube underflows: the answer is the Dulong-Petit high-T limit D ~ 1.
        _ => return Fixed::ONE,
    };
    Fixed::from_int(3)
        .checked_mul(integral)
        .and_then(|x| x.checked_div(y3))
        .unwrap_or(ZERO)
}

/// The Debye molar heat capacity at constant volume `C_v` (J/(mol*K)) per mole of ATOMS: `C_v = 3R * D(Theta_D/T)`,
/// the Debye model over the built Debye temperature and the temperature, with `R` the exact gas constant (floor)
/// and `D` the [`debye_function`]. No reserved value. Per mole of a compound with `n` atoms per formula unit,
/// multiply by `n` (that count is DATA, so the per-substance basis is a data row, not a rewrite). `C_v` rises
/// from `0` at `T = 0` (the `T^3` law) to the Dulong-Petit ceiling `3R ~ 24.94 J/(mol*K)` as `T >> Theta_D`.
/// HONEST LIMITS: this is `C_v` (constant VOLUME). The MEASURED `C_p` (constant pressure) is slightly higher,
/// `C_p = C_v + 9 * alpha^2 * B_T * V_m * T` (a few percent near room temperature for a solid); that correction
/// is the thermal-expansion slice's, folded in once the Grueneisen `alpha` is built (`alpha` derives from
/// `gamma_G`), so `C_v` is the piece that rests only on the Debye temperature. The Debye model itself also omits
/// the electronic and anharmonic heat capacity (why iron's Debye `C_v ~ 22 J/(mol*K)` at 300 K sits below the
/// measured `~25`), the reduced-order reach stated at its site. Non-positive `Theta_D` or temperature yields zero.
pub fn debye_heat_capacity_j_per_mol_k(debye_temperature: Fixed, temperature: Fixed) -> Fixed {
    if debye_temperature <= ZERO || temperature <= ZERO {
        return ZERO;
    }
    let y = match debye_temperature.checked_div(temperature) {
        Some(v) => v,
        None => return ZERO,
    };
    let d = debye_function(y);
    Fixed::from_int(3)
        .checked_mul(gas_constant_j_per_mol_k())
        .and_then(|three_r| three_r.checked_mul(d))
        .unwrap_or(Fixed::MAX)
}

/// The theoretical (ideal) shear strength `tau_th = G / (2*pi)` (GPa), the Frenkel limit: the stress at which a
/// perfect dislocation-free crystal shears by sliding whole atomic planes, from the shear modulus alone. No
/// reserved value (the `2*pi` is from [`Fixed::PI`]); it is the exact upper bound a material's strength can
/// approach. HONEST LIMIT: the classic sinusoidal `G/(2*pi)` overestimates the refined (DFT) ideal shear by
/// roughly a factor of two (iron `~13 GPa` here against a refined `~7 GPa`); the per-class knock-down in
/// [`operative_shear_strength_gpa`] absorbs both that model imprecision and the dislocation physics. Non-positive
/// input yields zero.
pub fn theoretical_shear_strength_gpa(shear_modulus_gpa: Fixed) -> Fixed {
    if shear_modulus_gpa <= ZERO {
        return ZERO;
    }
    let two_pi = match Fixed::from_int(2).checked_mul(Fixed::PI) {
        Some(v) if v > ZERO => v,
        _ => return ZERO,
    };
    shear_modulus_gpa.checked_div(two_pi).unwrap_or(Fixed::MAX)
}

/// The operative shear strength (GPa) `= knockdown * G / (2*pi)`, the ideal Frenkel strength scaled by the ONE
/// RESERVED-with-basis per-class knock-down fraction. In a real crystal, mobile dislocations let it shear far
/// below the ideal, so the operative (measured yield/flow) strength is `knockdown in (0, 1]` of the theoretical:
/// the basis is the ratio of the measured operative shear strength to `G/(2*pi)` per bonding/microstructural
/// class, spanning `~1e-2` for a soft annealed metal (iron `~0.012`, copper `~0.009`), through work-hardened and
/// fine-grained metals, up to `~0.7` for a covalent solid with few mobile dislocations (diamond) and `-> 1` for a
/// defect-free whisker. The knock-down is caller-supplied, never planted. This is the reduced-order per-class
/// residual the design opener named; the richer follow-on is a Hall-Petch grain-size model
/// (`sigma_y = sigma_0 + k_HP / sqrt(d)`, reading the freezer's built grain size), a two-coefficient design-first
/// piece flagged for its own slice. Non-positive inputs, or a knock-down outside `(0, 1]`, yield zero.
pub fn operative_shear_strength_gpa(shear_modulus_gpa: Fixed, knockdown: Fixed) -> Fixed {
    if shear_modulus_gpa <= ZERO || knockdown <= ZERO || knockdown > Fixed::ONE {
        return ZERO;
    }
    theoretical_shear_strength_gpa(shear_modulus_gpa)
        .checked_mul(knockdown)
        .unwrap_or(Fixed::MAX)
}

/// The VOLUMETRIC thermal expansion coefficient `alpha_V` (per kelvin), the Grueneisen relation
/// `alpha_V = gamma_G * C_v / (K * V_m)` rearranged from the thermodynamic identity
/// `gamma_G = alpha_V * K_T * V_m / C_v`. It reads the built heat capacity `C_v`, the anchored bulk modulus `K`,
/// and the molar volume `V_m`; the ONE RESERVED-with-basis per-class coefficient is the Grueneisen parameter
/// `gamma_G` (the SAME coefficient the Slack conductivity reserves, so one hunt serves both; HONEST LIMIT: the
/// thermodynamic weighted Grueneisen that governs expansion and the high-temperature acoustic Grueneisen that
/// Slack uses are not rigorously the same mode average, they differ within the per-class scatter the coefficient
/// already carries, so sharing one `gamma_G` is a sound modelling choice, not a claim the two averages are
/// physically identical). The basis:
/// `gamma_G = alpha_V * K * V_m / C_v`, the measured Grueneisen parameter per bonding class, `~1.5-2.2` for close-
/// packed metals (iron `~1.7`, aluminium `~2.2`), lower for open/covalent solids (diamond `~0.9`), caller-supplied
/// and never planted. UNIT FOLD: `K[GPa] * V_m[cm^3/mol] = kJ/mol` while `C_v` is in `J/(mol*K)`, so the `1000` in
/// the denominator is the exact `kJ -> J` conversion, not an authored value. BASIS CONSISTENCY: `C_v` and `V_m`
/// must share a basis (both per mole of atoms, or both per mole of formula units); for an element metal (one atom
/// per formula) they coincide, and a compound caller passes both per-atom (`C_v` per atom, `V_m / n`). Non-positive
/// inputs yield zero.
pub fn volumetric_thermal_expansion_per_k(
    gruneisen: Fixed,
    heat_capacity_j_per_mol_k: Fixed,
    bulk_modulus_gpa: Fixed,
    molar_volume_cm3_per_mol: Fixed,
) -> Fixed {
    if gruneisen <= ZERO
        || heat_capacity_j_per_mol_k <= ZERO
        || bulk_modulus_gpa <= ZERO
        || molar_volume_cm3_per_mol <= ZERO
    {
        return ZERO;
    }
    // denom = 1000 * K[GPa] * V_m[cm^3/mol] (the 1000 folds kJ/mol -> J/mol to match C_v's J/(mol*K)).
    let denom = match bulk_modulus_gpa
        .checked_mul(molar_volume_cm3_per_mol)
        .and_then(|kv| kv.checked_mul(Fixed::from_int(1000)))
    {
        Some(d) if d > ZERO => d,
        _ => return ZERO,
    };
    match gruneisen
        .checked_mul(heat_capacity_j_per_mol_k)
        .and_then(|num| num.checked_div(denom))
    {
        Some(a) => a,
        None => Fixed::MAX,
    }
}

/// The LINEAR thermal expansion coefficient `alpha_L = alpha_V / 3` (per kelvin), the isotropic third of the
/// volumetric [`volumetric_thermal_expansion_per_k`] (an isotropic solid expands equally on three axes). Same one
/// reserved Grueneisen `gamma_G`, same basis-consistency requirement. Iron lands `~1.04e-5 /K` against a measured
/// `~1.18e-5`. HONEST LIMIT: the linear coefficient is the isotropic average; an anisotropic (non-cubic) crystal
/// has axis-dependent expansion this single scalar does not resolve. Non-positive inputs yield zero.
pub fn linear_thermal_expansion_per_k(
    gruneisen: Fixed,
    heat_capacity_j_per_mol_k: Fixed,
    bulk_modulus_gpa: Fixed,
    molar_volume_cm3_per_mol: Fixed,
) -> Fixed {
    let volumetric = volumetric_thermal_expansion_per_k(
        gruneisen,
        heat_capacity_j_per_mol_k,
        bulk_modulus_gpa,
        molar_volume_cm3_per_mol,
    );
    volumetric.checked_div(Fixed::from_int(3)).unwrap_or(ZERO)
}

/// The surface-energy unit fold `10^23 / N_A` (`~0.16605`), mapping the broken-bond surface energy from
/// `(kJ/mol) / Angstrom^2` to `J/m^2`: the cohesive energy per atom is `E_coh[kJ/mol] * 1000 / N_A` joules and the
/// area per surface atom is `V_atom[A^3]^(2/3) * 10^-20` square metres, so their ratio carries the exact constant
/// `1000 * 10^20 / N_A = 10^23 / N_A`. The exact SI rational from `N_A = 6.02214076e23`. No authored decimal.
fn surface_energy_fold() -> Fixed {
    Fixed::from_ratio(100_000_000, 602_214_076)
}

/// The solid-vapour surface energy `gamma_sv` (J/m^2), the BROKEN-BOND model:
/// `gamma_sv = f_surf * E_coh_per_atom / A_atom`, the fraction of the cohesive energy carried by the bonds cut
/// when a surface is created, per unit surface area. This is EXACTLY the derivation the freezer REJECTED for the
/// solid-LIQUID interfacial energy `gamma_sl` (because the broken-bond count gives the solid-VAPOUR energy, the
/// wrong quantity for melt nucleation), landing where it belongs: `gamma_sv`, feeding wetting, fracture surface
/// energy, and the heterogeneous-nucleation follow-on the freezer flagged. It reads the built cohesive energy
/// `E_coh` (kJ/mol, the atomization enthalpy) and the atomic volume, folded to
/// `gamma_sv = f_surf * surface_energy_fold() * E_coh / cbrt(V_atom)^2` (the area per atom being `V_atom^(2/3)`).
/// The ONE RESERVED-with-basis per-class coefficient is the surface-bond fraction `f_surf`: the fraction of the
/// cohesive energy residing in the broken surface bonds, `~0.13-0.27` across metals (centred `~0.18`, the broken-
/// nearest-neighbour count at a close-packed surface), per bonding class and surface orientation, caller-supplied
/// and never planted. Iron lands `~2.4 J/m^2` at `f_surf ~ 0.18`. HONEST LIMITS: the fraction is an orientation-
/// AVERAGE (a real crystal has orientation-dependent `gamma_sv`); the model is validated on metals (ionic and
/// covalent surfaces cut different bonds, so `f_surf` is per-class); and the grain-boundary energy `gamma_gb` is
/// the sibling, `~1/3` of `gamma_sv` (a grain boundary breaks fewer bonds than a free surface), reserving its own
/// grain-boundary-to-surface ratio, a thin follow-on over this. Non-positive inputs yield zero.
pub fn surface_energy_j_per_m2(
    surface_bond_fraction: Fixed,
    cohesive_energy_kj_per_mol: Fixed,
    atomic_volume_angstrom3: Fixed,
) -> Fixed {
    if surface_bond_fraction <= ZERO
        || cohesive_energy_kj_per_mol <= ZERO
        || atomic_volume_angstrom3 <= ZERO
    {
        return ZERO;
    }
    // A_atom = V_atom^(2/3) = cbrt(V_atom)^2, both built exact ops.
    let area = atomic_volume_angstrom3.cbrt().powi(2);
    if area <= ZERO {
        return ZERO;
    }
    surface_bond_fraction
        .checked_mul(surface_energy_fold())
        .and_then(|x| x.checked_mul(cohesive_energy_kj_per_mol))
        .and_then(|x| x.checked_div(area))
        .unwrap_or(Fixed::MAX)
}

/// The grain-boundary energy `gamma_gb` (J/m^2) `= r_gb * gamma_sv`, the broken-bond SIBLING of the surface energy.
/// A grain boundary is two misoriented crystals meeting, so its atoms keep (misaligned) neighbours across the
/// boundary and fewer bonds are cut than at a free surface; `gamma_gb` is therefore a per-class FRACTION of the
/// solid-vapour `gamma_sv`. The ONE RESERVED-with-basis per-class coefficient is the grain-boundary-to-surface
/// ratio `r_gb`: the measured `gamma_gb / gamma_sv`, `~0.30..0.34` for HIGH-ANGLE boundaries across metals (iron
/// `~0.33`, copper `~0.34`, aluminium `~0.28`, nickel `~0.30`), caller-supplied and never planted. It reads the
/// built `gamma_sv`. Iron lands `~0.79 J/m^2` (measured `~0.78`). HONEST LIMITS: `r_gb` is the HIGH-ANGLE value;
/// a LOW-ANGLE (small-misorientation) boundary follows Read-Shockley (`gamma ~ theta*(A - ln theta)`, rising with
/// the misorientation angle), a different regime keyed on a boundary-angle datum, the follow-on. `gamma_gb` feeds
/// grain growth and the Hall-Petch strength the knock-down wants. Non-positive inputs, or a ratio outside `(0, 1]`
/// (a grain boundary cannot cost more than a free surface), yield zero.
pub fn grain_boundary_energy_j_per_m2(
    surface_energy_j_per_m2: Fixed,
    gb_to_surface_ratio: Fixed,
) -> Fixed {
    if surface_energy_j_per_m2 <= ZERO
        || gb_to_surface_ratio <= ZERO
        || gb_to_surface_ratio > Fixed::ONE
    {
        return ZERO;
    }
    surface_energy_j_per_m2
        .checked_mul(gb_to_surface_ratio)
        .unwrap_or(Fixed::MAX)
}

/// The Morelli-Slack Grueneisen correction factor `1 / (1 - 0.514/gamma + 0.228/gamma^2)` for the lattice
/// thermal conductivity, the weak dependence of the Slack prefactor on the Grueneisen parameter. The constants
/// `{0.514, 0.228}` are the CITED Morelli-Slack (2006) UNIVERSAL fit values (not per-class, not per-world, the
/// same status as the Chen-Tse hardness constants), so folding the prefactor through this form means the
/// conductivity reserves NO new coefficient beyond the `gamma_G` the expansion already reserves. The denominator
/// is positive for every physical `gamma > 0` (its minimum near `gamma ~ 0.7` is `~0.73`); a non-positive or
/// degenerate `gamma` returns one (no correction).
fn slack_gamma_correction(gruneisen: Fixed) -> Fixed {
    if gruneisen <= ZERO {
        return Fixed::ONE;
    }
    let over_gamma = match Fixed::ONE.checked_div(gruneisen) {
        Some(v) => v,
        None => return Fixed::ONE,
    };
    let over_gamma_sq = over_gamma.checked_mul(over_gamma).unwrap_or(ZERO);
    // denom = 1 - 0.514/gamma + 0.228/gamma^2.
    let term1 = Fixed::from_ratio(514, 1000).checked_mul(over_gamma);
    let term2 = Fixed::from_ratio(228, 1000).checked_mul(over_gamma_sq);
    let denom = match (term1, term2) {
        (Some(a), Some(b)) => Fixed::ONE.checked_sub(a).map(|x| x.saturating_add(b)),
        _ => None,
    };
    match denom {
        Some(d) if d > ZERO => Fixed::ONE.checked_div(d).unwrap_or(Fixed::ONE),
        _ => Fixed::ONE,
    }
}

/// The cited Slack-Leibfried-Schlomann phase-space number `0.849`, the ONE pure number in the conductivity
/// prefactor (the whole prefactor factors as this dimensionless number times a fundamental-constant fold times a
/// pure-math geometric factor, so only this is cited). PROVENANCE TAG:
/// `[secondary-sourced + reassembly-passing, gamma-dependent, primary pending]`, one rung BELOW top-`[M]`: the
/// secondary sources contradict each other on Julian's correction to Leibfried-Schlomann (a factor-two dispute,
/// restated constants differing `~10..30 percent`), so `0.849` carries a Slack-versus-Julian BAND and the tag
/// closes to top rung when the primary (Julian 1965, Phys. Rev. 137 A128) is fetched. This is the dimensionless-
/// constant law in force: a dimensional empirical constant factored until only a pure number is cited.
fn slack_phase_space_number() -> Fixed {
    Fixed::from_ratio(849, 1000)
}

/// The dimensionless geometric factor `3 * cbrt(4) / (20 * pi^3)` of the Leibfried-Schlomann prefactor (about
/// `7.68e-3`), DERIVED from the built `cbrt` and `Fixed::PI`, no authored decimal.
fn slack_geometric_factor() -> Fixed {
    let numerator = match Fixed::from_int(3).checked_mul(Fixed::from_int(4).cbrt()) {
        Some(v) => v,
        None => return ZERO,
    };
    let denom = Fixed::PI
        .checked_mul(Fixed::PI)
        .and_then(|x| x.checked_mul(Fixed::PI))
        .and_then(|p3| p3.checked_mul(Fixed::from_int(20)));
    match denom {
        Some(d) if d > ZERO => numerator.checked_div(d).unwrap_or(ZERO),
        _ => ZERO,
    }
}

/// The dimensional fold `(k_B/hbar)^3 * amu * Angstrom`, times `1e6` to pair with the `(Theta/100)^3` rescale in
/// [`lattice_thermal_conductivity_w_per_m_k`]. ASSEMBLED from the EXACT SI mantissas and a single collapsed power
/// of ten (the dimensionless-constant law: no folded dimensional decimal). The powers collapse cleanly:
/// `(k_B/hbar)^3` carries `10^33`, `amu` carries `10^-27`, the angstrom `10^-10`, the rescale `10^6`, netting
/// `10^2`, so the whole fold is `(1.380649/1.054571817)^3 * 1.66053906660 * 10^2 ~ 372.6`. The constituents
/// `k_B`, `hbar`, and `amu` each overflow or underflow Q32.32 alone; only this collapsed mantissa product is
/// representable, which is why the fold is assembled rather than multiplied out.
fn slack_dimensional_fold_rescaled() -> Fixed {
    // k_B/hbar as the exact mantissa ratio (the 10^11 power is carried in the collapsed 10^2 below).
    let kb_over_hbar = match Fixed::from_ratio(1_380_649, 1_000_000)
        .checked_div(Fixed::from_ratio(1_054_571_817, 1_000_000_000))
    {
        Some(v) => v,
        None => return Fixed::MAX,
    };
    let cubed = match kb_over_hbar
        .checked_mul(kb_over_hbar)
        .and_then(|sq| sq.checked_mul(kb_over_hbar))
    {
        Some(v) => v,
        None => return Fixed::MAX,
    };
    // amu mantissa 1.66053906660, then the collapsed power of ten 10^2.
    let amu_mantissa = Fixed::from_ratio(166_053_906_660, 100_000_000_000);
    cubed
        .checked_mul(amu_mantissa)
        .and_then(|m| m.checked_mul(Fixed::from_int(100)))
        .unwrap_or(Fixed::MAX)
}

/// The LATTICE (phonon) thermal conductivity `kappa_L` (W/(m*K)), the Slack model
/// `kappa_L = A(gamma) * M_bar * Theta_a^3 * delta / (gamma^2 * n^(2/3) * T)`, where `M_bar` is the mean atomic
/// mass (amu), `Theta_a` the acoustic Debye temperature (the built shear-aware `Theta_D`, which IS the acoustic
/// average), `delta = cbrt(V_atom)` the interatomic spacing, `n` the atoms per primitive cell (DATA), and
/// the Slack prefactor `A(gamma) = C_pure(gamma) * (k_B/hbar)^3 * amu * Angstrom`. It RESERVES NO NEW COEFFICIENT
/// beyond the expansion's `gamma_G`, and per the DIMENSIONLESS-CONSTANT LAW the prefactor is FACTORED, not folded:
/// the dimensional part is [`slack_dimensional_fold_rescaled`] (assembled from the `k_B`, `hbar`, `amu` exact
/// mantissas), the pure-math part is [`slack_geometric_factor`] (`3*cbrt(4)/(20*pi^3)`), the `gamma`-dependence is
/// [`slack_gamma_correction`], and the ONLY cited number is [`slack_phase_space_number`] `0.849` (with its
/// Leibfried-Schlomann-Julian band). The old folded `3.1e-6` was the collapse of exactly this structure; the
/// reassembly rebuilds it within the citation's two significant figures (the derived coefficient is `~3.04`), and
/// unlike the folded scalar the factored form carries the true `gamma`-dependence (diamond at `gamma ~ 0.9` runs
/// `~13 percent` higher). The `Theta^3` is computed as `(Theta/100)^3` with the paired `10^6` collapsed into the
/// dimensional fold, so no intermediate overflows Q32.32 (a bare `Theta^3 ~ 1e10` for a stiff solid would).
///
/// HONEST LIMITS, named at the site (this is a REDUCED-ORDER model). (1) It is ORDER-OF-MAGNITUDE: within a factor
/// of `~3` for simple crystals (with the derived prefactor: diamond `~2108` against `~2200`, NaCl `~7.1` against
/// `~6.5`, MgO `~110` against `~60`), but it OVERSTATES strongly-anharmonic or complex-cell crystals the single-
/// scattering form misses (rutile `TiO2 ~43` against a measured `~9`), so such classes are an intrinsic upper
/// bound, not a trusted value. (2) It is
/// the LATTICE conductivity only. For an INSULATOR/semiconductor that is the whole story; for a METAL the total is
/// dominated by the ELECTRONIC conductivity (Wiedemann-Franz), which needs the electronic-structure substrate (the
/// deferred Stage-6 sub-arc), and the Slack phonon form additionally over-predicts even the metal's lattice part.
/// So for a metal this is a lattice COMPONENT, not the total. Non-positive inputs yield zero.
// @derives: lattice thermal conductivity k(T) <- Grueneisen, mean atomic mass, Debye temperature, atomic volume, cell count (Slack estimator rung)
pub fn lattice_thermal_conductivity_w_per_m_k(
    gruneisen: Fixed,
    mean_atomic_mass_amu: Fixed,
    debye_temperature_k: Fixed,
    atomic_volume_angstrom3: Fixed,
    atoms_per_primitive_cell: i32,
    temperature: Fixed,
) -> Fixed {
    if gruneisen <= ZERO
        || mean_atomic_mass_amu <= ZERO
        || debye_temperature_k <= ZERO
        || atomic_volume_angstrom3 <= ZERO
        || atoms_per_primitive_cell < 1
        || temperature <= ZERO
    {
        return ZERO;
    }
    let delta = atomic_volume_angstrom3.cbrt();
    // (Theta/100)^3 via checked multiplies; the 1e6 this drops is folded into the 3.1 prefactor below.
    let theta_scaled = match debye_temperature_k.checked_div(Fixed::from_int(100)) {
        Some(v) if v > ZERO => v,
        _ => return ZERO,
    };
    let theta3 = match theta_scaled
        .checked_mul(theta_scaled)
        .and_then(|sq| sq.checked_mul(theta_scaled))
    {
        Some(v) => v,
        None => return Fixed::MAX,
    };
    // A(gamma) rescaled = (dimensional fold) * 0.849 * (geometric) * gamma_correction, the dimensionless-constant
    // law: the fundamental-constant fold and the pure-math geometric factor derived, only 0.849 cited. This
    // rebuilds the old folded 3.1 within the citation's two significant figures and carries the true gamma-shape.
    let a_rescaled = match slack_dimensional_fold_rescaled()
        .checked_mul(slack_phase_space_number())
        .and_then(|x| x.checked_mul(slack_geometric_factor()))
        .and_then(|x| x.checked_mul(slack_gamma_correction(gruneisen)))
    {
        Some(v) => v,
        None => return Fixed::MAX,
    };
    let numerator = match a_rescaled
        .checked_mul(mean_atomic_mass_amu)
        .and_then(|x| x.checked_mul(theta3))
        .and_then(|x| x.checked_mul(delta))
    {
        Some(v) => v,
        None => return Fixed::MAX,
    };
    // denominator = gamma^2 * n^(2/3) * T; n^(2/3) = cbrt(n)^2.
    let gamma_sq = match gruneisen.checked_mul(gruneisen) {
        Some(v) => v,
        None => return ZERO,
    };
    let n23 = Fixed::from_int(atoms_per_primitive_cell).cbrt().powi(2);
    let denominator = match gamma_sq
        .checked_mul(n23)
        .and_then(|x| x.checked_mul(temperature))
    {
        Some(v) if v > ZERO => v,
        _ => return ZERO,
    };
    numerator.checked_div(denominator).unwrap_or(Fixed::MAX)
}

/// The thermal diffusivity `alpha = kappa / (rho * c_p)` (m^2/s), how fast a temperature disturbance spreads. It
/// COMPOSES built quantities and reserves NO value: the volumetric heat capacity `rho * c_p` equals `C_v / V_m`
/// (the mass cancels, since `rho = M / V_m` and `c_p = C_v / M`), so `alpha = kappa * V_m / C_v`. It reads the
/// lattice conductivity `kappa` (W/(m*K)), the molar volume `V_m` (cm^3/mol), and the molar heat capacity `C_v`
/// (J/(mol*K)); the `1e-6` folds `V_m` from `cm^3/mol` to `m^3/mol` (the exact unit constant). `kappa`, `V_m`, and
/// `C_v` must share a basis (per mole of atoms, or per formula unit); the ratio `V_m / C_v` is basis-invariant.
/// HONEST LIMITS: it inherits the conductivity's reach (order-of-magnitude, LATTICE-only, so for a metal this is
/// the phonon-based diffusivity with the electronic total deferred), and it uses `C_v` for `c_p` (a deliberate
/// choice: the solid `C_p - C_v = alpha_V^2 * K * V_m * T` is `~3..5 percent` at room temperature, negligible
/// inside the factor-3 Slack `kappa` uncertainty `alpha` already carries, and the correction rides the expansion
/// slice once `alpha_V` is built). Non-positive inputs yield zero.
pub fn thermal_diffusivity_m2_per_s(
    conductivity_w_per_m_k: Fixed,
    molar_volume_cm3_per_mol: Fixed,
    heat_capacity_j_per_mol_k: Fixed,
) -> Fixed {
    if conductivity_w_per_m_k <= ZERO
        || molar_volume_cm3_per_mol <= ZERO
        || heat_capacity_j_per_mol_k <= ZERO
    {
        return ZERO;
    }
    // alpha = kappa * V_m / C_v, then * 1e-6 (cm^3/mol -> m^3/mol). The V_m/C_v then 1e-6 ordering keeps the
    // intermediate order-unity rather than forming a tiny product first.
    conductivity_w_per_m_k
        .checked_mul(molar_volume_cm3_per_mol)
        .and_then(|x| x.checked_div(heat_capacity_j_per_mol_k))
        .and_then(|x| x.checked_div(Fixed::from_int(1_000_000)))
        .unwrap_or(ZERO)
}

/// The property route bound to the periodic table and the EOS anchors, so density reads the molar mass and molar
/// volume, and the Debye temperature reuses the freezer's sound speed over the anchors, all for an anchored
/// metal. No reserved value enters (this first slice reserves none); a metal missing an anchor escalates
/// (`None`) rather than fabricating a property.
pub struct PropertyRoute<'a> {
    table: &'a PeriodicTable,
    anchors: &'a MetalEosAnchors,
}

impl<'a> PropertyRoute<'a> {
    /// Bind the property route to the periodic table (the molar mass) and the EOS anchors (`B_0`, `V_m`).
    pub fn new(table: &'a PeriodicTable, anchors: &'a MetalEosAnchors) -> Self {
        PropertyRoute { table, anchors }
    }

    /// The mass density `rho` (g/cm^3) for an anchored metal, from its molar mass and molar volume, or `None`
    /// (escalate) when the metal has no anchored molar volume or no standard atomic weight.
    pub fn density(&self, symbol: &str) -> Option<Fixed> {
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let molar_mass = self.table.element(symbol)?.standard_atomic_weight;
        if molar_mass <= ZERO {
            return None;
        }
        Some(density_g_per_cm3(molar_mass, molar_volume))
    }

    /// The Debye temperature `Theta_D` (K) for an anchored metal, reusing the freezer's bulk sound speed
    /// (`sqrt(B_0/rho)`) over the derived density and the atomic volume from the molar volume. `None` (escalate)
    /// when the metal lacks a bulk modulus, a molar volume, or a standard atomic weight. Carries the
    /// module's bulk-elastic overestimate limit.
    pub fn debye_temperature(&self, symbol: &str) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let rho = self.density(symbol)?;
        let sound_speed = freezer::sound_speed_km_per_s(bulk_modulus, rho);
        let atomic_volume =
            molar_volume.checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())?;
        Some(debye_temperature(sound_speed, atomic_volume))
    }

    /// The shear modulus `G` (GPa) for an anchored metal, `k * B_0` over the anchored bulk modulus and the
    /// caller's reserved Pugh ratio `k`. `None` (escalate) when the metal has no anchored bulk modulus. `k` is
    /// the caller's reserved coefficient, never planted.
    pub fn shear_modulus(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        Some(shear_modulus_gpa(bulk_modulus, pugh_ratio))
    }

    /// Young's modulus `E` (GPa) for an anchored metal, from the anchored bulk modulus and the derived shear
    /// modulus (`k * B_0`). `None` (escalate) when the metal has no anchored bulk modulus.
    pub fn youngs_modulus(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        Some(youngs_modulus_gpa(bulk_modulus, shear))
    }

    /// Poisson's ratio for an anchored metal, from the anchored bulk modulus and the derived shear modulus.
    /// `None` (escalate) when the metal has no anchored bulk modulus.
    pub fn poisson_ratio(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        Some(poisson_ratio(bulk_modulus, shear))
    }

    /// The intrinsic Vickers hardness `H_V` (GPa) for an anchored metal, the Chen-Tse correlation over the
    /// derived shear modulus (`k * B_0`) and the SAME reserved Pugh ratio `k` (no new coefficient). `None`
    /// (escalate) when the metal has no anchored bulk modulus. Carries the intrinsic-versus-operative and
    /// soft-low-`k`-metal limits.
    pub fn hardness(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        Some(chen_tse_hardness_gpa(shear, pugh_ratio))
    }

    /// The SHEAR-AWARE Debye temperature `Theta_D` (K) for an anchored metal, using the Debye-averaged velocity
    /// `v_D` (from the anchored `B_0`, the derived shear modulus `G = k*B_0`, and the density) rather than the
    /// bulk sound speed. This RETIRES the bulk-elastic overestimate of [`PropertyRoute::debye_temperature`]:
    /// with `G` available (through the reserved Pugh ratio `k`), iron lands `~470 K` rather than the bulk `~609`.
    /// `None` (escalate) when the metal lacks a bulk modulus, a molar volume, or a standard atomic weight.
    pub fn debye_temperature_shear_aware(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let rho = self.density(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        let v_d = debye_velocity_km_per_s(bulk_modulus, shear, rho);
        let atomic_volume =
            molar_volume.checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())?;
        Some(debye_temperature(v_d, atomic_volume))
    }

    /// The Debye molar heat capacity `C_v` (J/(mol*K), per mole of atoms) for an anchored metal at a temperature,
    /// over the SHEAR-AWARE Debye temperature (`k*B_0`-derived, the accurate one). `None` (escalate) when the
    /// metal lacks a bulk modulus, a molar volume, or a standard atomic weight. Carries the `C_v`-versus-`C_p`
    /// and electronic/anharmonic limits of [`debye_heat_capacity_j_per_mol_k`]. `k` is the caller's reserved
    /// Pugh ratio, never planted.
    pub fn heat_capacity(
        &self,
        symbol: &str,
        temperature: Fixed,
        pugh_ratio: Fixed,
    ) -> Option<Fixed> {
        let theta_d = self.debye_temperature_shear_aware(symbol, pugh_ratio)?;
        Some(debye_heat_capacity_j_per_mol_k(theta_d, temperature))
    }

    /// The theoretical (ideal) shear strength `tau_th = G/(2*pi)` (GPa) for an anchored metal, over the derived
    /// shear modulus (`k*B_0`). `None` (escalate) when the metal has no anchored bulk modulus. Reserves only the
    /// caller's Pugh ratio `k` (which `G` carries); the Frenkel limit itself has no reserved value.
    pub fn theoretical_shear_strength(&self, symbol: &str, pugh_ratio: Fixed) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        Some(theoretical_shear_strength_gpa(shear))
    }

    /// The operative shear strength (GPa) for an anchored metal, the ideal Frenkel strength scaled by the caller's
    /// reserved per-class `knockdown in (0, 1]` (see [`operative_shear_strength_gpa`] for its basis), over the
    /// derived shear modulus. `None` (escalate) when the metal has no anchored bulk modulus. Both `k` and the
    /// knock-down are caller-supplied, never planted.
    pub fn operative_shear_strength(
        &self,
        symbol: &str,
        pugh_ratio: Fixed,
        knockdown: Fixed,
    ) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let shear = shear_modulus_gpa(bulk_modulus, pugh_ratio);
        Some(operative_shear_strength_gpa(shear, knockdown))
    }

    /// The LINEAR thermal expansion coefficient `alpha_L` (per kelvin) for an anchored metal at a temperature, the
    /// Grueneisen relation over the anchored `K`/`V_m`, the built heat capacity `C_v` (via the shear-aware
    /// `Theta_D`), and the caller's reserved Grueneisen `gamma_G`. For an element metal `C_v` (per atom) and `V_m`
    /// (per atom) share a basis, so no atom-count enters. `None` (escalate) when the metal lacks a bulk modulus, a
    /// molar volume, or a standard atomic weight. `gamma_G` and the Pugh ratio `k` are caller-supplied, never
    /// planted.
    pub fn linear_thermal_expansion(
        &self,
        symbol: &str,
        temperature: Fixed,
        pugh_ratio: Fixed,
        gruneisen: Fixed,
    ) -> Option<Fixed> {
        let bulk_modulus = self.anchors.bulk_modulus_gpa(symbol)?;
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let theta_d = self.debye_temperature_shear_aware(symbol, pugh_ratio)?;
        let cv = debye_heat_capacity_j_per_mol_k(theta_d, temperature);
        Some(linear_thermal_expansion_per_k(
            gruneisen,
            cv,
            bulk_modulus,
            molar_volume,
        ))
    }

    /// The solid-vapour surface energy `gamma_sv` (J/m^2) for an anchored metal, the broken-bond model over the
    /// cohesive energy (the periodic table's atomization enthalpy) and the atomic volume (from the molar volume),
    /// with the caller's reserved surface-bond fraction `f_surf`. `None` (escalate) when the metal lacks an
    /// atomization enthalpy or a molar volume. `f_surf` is caller-supplied, never planted.
    pub fn surface_energy(&self, symbol: &str, surface_bond_fraction: Fixed) -> Option<Fixed> {
        let cohesive_energy = self.table.element(symbol)?.atomization_enthalpy?;
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let atomic_volume =
            molar_volume.checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())?;
        Some(surface_energy_j_per_m2(
            surface_bond_fraction,
            cohesive_energy,
            atomic_volume,
        ))
    }

    /// The grain-boundary energy `gamma_gb` (J/m^2) for an anchored metal, `r_gb * gamma_sv` over the broken-bond
    /// surface energy and the caller's reserved grain-boundary-to-surface ratio `r_gb` (`~0.3`, high-angle).
    /// `None` (escalate) when the metal lacks an atomization enthalpy or a molar volume. Both `f_surf` and `r_gb`
    /// are caller-supplied, never planted.
    pub fn grain_boundary_energy(
        &self,
        symbol: &str,
        surface_bond_fraction: Fixed,
        gb_to_surface_ratio: Fixed,
    ) -> Option<Fixed> {
        let surface_energy = self.surface_energy(symbol, surface_bond_fraction)?;
        Some(grain_boundary_energy_j_per_m2(
            surface_energy,
            gb_to_surface_ratio,
        ))
    }

    /// The LATTICE (phonon) thermal conductivity `kappa_L` (W/(m*K)) for an anchored metal at a temperature, the
    /// Slack model over the mean atomic mass, the shear-aware `Theta_D`, the atomic volume, and the caller's
    /// reserved Grueneisen `gamma_G` (an element metal has one atom per primitive cell). `None` (escalate) when the
    /// metal lacks a bulk modulus, a molar volume, or a standard atomic weight. IMPORTANT: for a metal this is the
    /// LATTICE COMPONENT only, not the total conductivity: a metal's heat is carried mostly by ELECTRONS
    /// (Wiedemann-Franz), which the electronic-structure sub-arc supplies, and the Slack phonon form over-predicts
    /// even the lattice part for a metal. So read this as the phonon component with the electronic total deferred,
    /// never as the metal's measured conductivity. `gamma_G` and the Pugh ratio `k` are caller-supplied.
    pub fn lattice_thermal_conductivity(
        &self,
        symbol: &str,
        temperature: Fixed,
        pugh_ratio: Fixed,
        gruneisen: Fixed,
    ) -> Option<Fixed> {
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let molar_mass = self.table.element(symbol)?.standard_atomic_weight;
        let theta_d = self.debye_temperature_shear_aware(symbol, pugh_ratio)?;
        let atomic_volume =
            molar_volume.checked_mul(rose_eos::cm3_per_mol_to_angstrom3_per_atom())?;
        // An element metal is one atom per primitive cell (BCC/FCC primitive cells hold one atom).
        Some(lattice_thermal_conductivity_w_per_m_k(
            gruneisen,
            molar_mass,
            theta_d,
            atomic_volume,
            1,
            temperature,
        ))
    }

    /// The (lattice-based) thermal diffusivity `alpha` (m^2/s) for an anchored metal at a temperature, composing
    /// the lattice conductivity, the molar volume, and the built heat capacity `C_v` (via the shear-aware
    /// `Theta_D`). `None` (escalate) when the metal lacks a bulk modulus, a molar volume, or a standard atomic
    /// weight. Inherits the conductivity's LATTICE-only limit: for a metal this is the phonon-based diffusivity,
    /// the electronic contribution deferred to the electronic-structure sub-arc. `gamma_G` and the Pugh ratio `k`
    /// are caller-supplied.
    pub fn thermal_diffusivity(
        &self,
        symbol: &str,
        temperature: Fixed,
        pugh_ratio: Fixed,
        gruneisen: Fixed,
    ) -> Option<Fixed> {
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let conductivity =
            self.lattice_thermal_conductivity(symbol, temperature, pugh_ratio, gruneisen)?;
        let theta_d = self.debye_temperature_shear_aware(symbol, pugh_ratio)?;
        let heat_capacity = debye_heat_capacity_j_per_mol_k(theta_d, temperature);
        Some(thermal_diffusivity_m2_per_s(
            conductivity,
            molar_volume,
            heat_capacity,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("periodic table")
    }
    fn anchors() -> MetalEosAnchors {
        MetalEosAnchors::standard().expect("metal EOS anchors")
    }
    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn density_is_the_molar_mass_over_the_molar_volume() {
        // Iron: M = 55.845 g/mol, V_m = 7.09 cm^3/mol -> rho ~ 7.88 g/cm^3 (measured ~7.87).
        let rho = density_g_per_cm3(Fixed::from_ratio(55845, 1000), Fixed::from_ratio(709, 100));
        assert!(
            close(rho, 7.877, 0.01),
            "iron density ~7.88 g/cm^3: {rho:?}"
        );
        // A denser packing (smaller molar volume) at the same mass raises the density; guards yield zero.
        assert!(density_g_per_cm3(Fixed::from_int(56), Fixed::from_int(5)) > rho);
        assert_eq!(density_g_per_cm3(ZERO, Fixed::from_int(7)), ZERO);
        assert_eq!(density_g_per_cm3(Fixed::from_int(56), ZERO), ZERO);
    }

    #[test]
    fn the_debye_temperature_derives_from_the_sound_speed() {
        // Iron: bulk sound speed c_s ~ 4.648 km/s (sqrt(170/7.87)), V_atom ~ 11.77 A^3.
        // Theta_D = debye_fold * c_s / cbrt(V_atom) ~ 297.7 * 4.648 / 2.272 ~ 609 K. This is the BULK-sound-speed
        // value; the measured iron Theta_D is ~470 K, and the ~30% gap is the bulk-elastic approximation (the
        // shear-aware Debye velocity is lower), the documented limit, NOT a mechanism error.
        let c_s = Fixed::from_ratio(4648, 1000);
        let v_atom = Fixed::from_ratio(1177, 100);
        let theta_d = debye_temperature(c_s, v_atom);
        assert!(
            close(theta_d, 609.0, 12.0),
            "iron Theta_D from the bulk sound speed ~609 K (bulk-elastic overestimate of the true ~470): {theta_d:?}"
        );
        // Monotone: a faster sound speed (stiffer or lighter) raises Theta_D; a larger atomic volume lowers it.
        assert!(debye_temperature(Fixed::from_int(6), v_atom) > theta_d);
        assert!(debye_temperature(c_s, Fixed::from_int(30)) < theta_d);
        // Guards: no sound speed or no volume, no Debye temperature.
        assert_eq!(debye_temperature(ZERO, v_atom), ZERO);
        assert_eq!(debye_temperature(c_s, ZERO), ZERO);
        // Deterministic (Principle 3).
        assert_eq!(theta_d, debye_temperature(c_s, v_atom));
    }

    #[test]
    fn the_property_route_reads_the_anchors() {
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);

        // Iron density through the substrate (molar mass from the table, V_m from the anchors) ~7.87 g/cm^3.
        let rho = route.density("Fe").expect("Fe density");
        assert!(close(rho, 7.877, 0.05), "route iron density ~7.87: {rho:?}");
        // A lighter, more open metal (Na) is far less dense than iron.
        let na_rho = route.density("Na").expect("Na density");
        assert!(na_rho < rho && na_rho > ZERO, "Na is less dense than Fe");

        // Iron Debye temperature through the substrate ~609 K (the bulk-sound-speed value).
        let theta_d = route.debye_temperature("Fe").expect("Fe Theta_D");
        assert!(
            close(theta_d, 609.0, 30.0),
            "route iron Theta_D ~609 K (bulk-elastic): {theta_d:?}"
        );
        // A stiffer, denser transition metal has a different Debye temperature than a soft alkali; both positive.
        let na_theta = route.debye_temperature("Na").expect("Na Theta_D");
        assert!(na_theta > ZERO && theta_d > ZERO);

        // An unanchored metal escalates rather than fabricating a property.
        assert!(
            route.density("Xx").is_none(),
            "an unanchored symbol has no density"
        );
        assert!(
            route.debye_temperature("Xx").is_none(),
            "an unanchored symbol has no Debye temperature"
        );
    }

    #[test]
    fn the_elastic_moduli_derive_from_the_bulk_modulus_and_the_pugh_ratio() {
        // NON-CIRCULAR check: feed iron's CITED Pugh ratio k = 0.48 (its measured G/K, independent) and the
        // anchored K = B_0 = 170 GPa, and require G, E, Poisson to land iron's measured values (G ~82 GPa,
        // E ~211 GPa, nu ~0.29). k is cited-independent, so the moduli match is a consequence, not a fit.
        let k_fe = Fixed::from_ratio(48, 100); // Pugh ratio ~0.48 (iron, test-only)
        let bulk = Fixed::from_int(170);
        let g = shear_modulus_gpa(bulk, k_fe);
        assert!(close(g, 81.6, 2.0), "G = k*K ~82 GPa: {g:?}");
        let e = youngs_modulus_gpa(bulk, g);
        assert!(close(e, 211.0, 6.0), "E = 9KG/(3K+G) ~211 GPa: {e:?}");
        let nu = poisson_ratio(bulk, g);
        assert!(
            close(nu, 0.29, 0.02),
            "nu = (3K-2G)/(2(3K+G)) ~0.29: {nu:?}"
        );
        // Monotone: a higher Pugh ratio (stiffer shear) raises G and E and lowers Poisson.
        let g_stiff = shear_modulus_gpa(bulk, Fixed::from_ratio(60, 100));
        assert!(g_stiff > g, "a higher Pugh ratio raises the shear modulus");
        assert!(
            youngs_modulus_gpa(bulk, g_stiff) > e,
            "a higher Pugh ratio raises Young's modulus"
        );
        assert!(
            poisson_ratio(bulk, g_stiff) < nu,
            "a higher Pugh ratio lowers Poisson's ratio"
        );
        // Guards.
        assert_eq!(shear_modulus_gpa(ZERO, k_fe), ZERO);
        assert_eq!(youngs_modulus_gpa(bulk, ZERO), ZERO);
        assert_eq!(poisson_ratio(bulk, ZERO), ZERO);
        // Through the route (reads B_0 from the anchors, reserves only the caller's k).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let route_e = route.youngs_modulus("Fe", k_fe).expect("Fe E");
        assert!(
            close(route_e, 211.0, 8.0),
            "route iron E ~211 GPa: {route_e:?}"
        );
        assert!(
            route.shear_modulus("Xx", k_fe).is_none(),
            "an unanchored metal escalates in the moduli route"
        );
    }

    #[test]
    fn chen_tse_hardness_lands_diamond_and_gives_iron_intrinsic() {
        // The Chen-Tse VALIDATION on its home turf (hard covalent solids): diamond k = G/K = 535/443 ~1.208,
        // G = 535 GPa -> H_V = 2*(1.208^2 * 535)^0.585 - 3 ~95 GPa against the measured ~96 GPa. Cited diamond
        // moduli in, measured hardness the target, non-circular.
        let diamond_k = Fixed::from_ratio(1208, 1000);
        let diamond_g = Fixed::from_int(535);
        let h_diamond = chen_tse_hardness_gpa(diamond_g, diamond_k);
        assert!(
            close(h_diamond, 95.0, 4.0),
            "Chen-Tse lands diamond ~95 GPa: {h_diamond:?}"
        );
        // Iron: k ~0.48, G ~82 GPa -> the INTRINSIC hardness ~8 GPa, higher than soft annealed iron's ~1 GPa
        // (the gap is the dislocation-plasticity knock-down, the strength slice).
        let h_iron = chen_tse_hardness_gpa(Fixed::from_int(82), Fixed::from_ratio(48, 100));
        assert!(
            close(h_iron, 8.2, 1.5),
            "iron intrinsic hardness ~8 GPa: {h_iron:?}"
        );
        assert!(h_diamond > h_iron, "diamond is far harder than iron");
        // Guards and determinism.
        assert_eq!(chen_tse_hardness_gpa(ZERO, diamond_k), ZERO);
        assert_eq!(chen_tse_hardness_gpa(diamond_g, ZERO), ZERO);
        assert_eq!(h_diamond, chen_tse_hardness_gpa(diamond_g, diamond_k));
        // Through the route (reuses k, no new coefficient).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let h_fe = route
            .hardness("Fe", Fixed::from_ratio(48, 100))
            .expect("Fe hardness");
        assert!(h_fe > ZERO, "route iron hardness is positive: {h_fe:?}");
        assert!(
            route.hardness("Xx", diamond_k).is_none(),
            "an unanchored metal escalates in the hardness route"
        );
    }

    #[test]
    fn the_shear_aware_debye_velocity_retires_the_bulk_elastic_limit() {
        // Iron: K = 170, G = k*K = 0.48*170 ~81.6, rho ~7.87. v_L = sqrt((K+4G/3)/rho) ~5.96 km/s,
        // v_T = sqrt(G/rho) ~3.22 km/s, v_D = cbrt(3/(1/v_L^3 + 2/v_T^3)) ~3.60 km/s (the shear modes drag the
        // Debye velocity well below the bulk ~4.65).
        let k_fe = Fixed::from_ratio(48, 100);
        let bulk = Fixed::from_int(170);
        let g = shear_modulus_gpa(bulk, k_fe);
        let rho = Fixed::from_ratio(787, 100);
        let v_d = debye_velocity_km_per_s(bulk, g, rho);
        assert!(
            close(v_d, 3.60, 0.15),
            "iron Debye velocity ~3.6 km/s: {v_d:?}"
        );
        // It is lower than the bulk sound speed (the shear modes soften the average).
        let v_bulk = freezer::sound_speed_km_per_s(bulk, rho);
        assert!(
            v_d < v_bulk,
            "the Debye velocity is below the bulk sound speed"
        );
        // The shear-aware Debye temperature lands iron's measured ~470 K, retiring the bulk-elastic ~609 K.
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let theta_shear = route
            .debye_temperature_shear_aware("Fe", k_fe)
            .expect("Fe shear-aware Theta_D");
        assert!(
            close(theta_shear, 470.0, 40.0),
            "shear-aware iron Theta_D ~470 K (retires the bulk ~609): {theta_shear:?}"
        );
        let theta_bulk = route.debye_temperature("Fe").expect("Fe bulk Theta_D");
        assert!(
            theta_shear < theta_bulk,
            "the shear-aware Theta_D is below the bulk overestimate"
        );
        // Guards and escalation.
        assert_eq!(debye_velocity_km_per_s(ZERO, g, rho), ZERO);
        assert!(
            route.debye_temperature_shear_aware("Xx", k_fe).is_none(),
            "an unanchored metal escalates"
        );
    }

    #[test]
    fn the_debye_heat_capacity_spans_the_dulong_petit_and_t_cubed_limits() {
        // Reference values from the exact Debye function C_v = 3R * D(Theta_D/T) (validated against a
        // high-accuracy quadrature; the capped-Simpson algorithm matches it to five decimals).
        let three_r = 24.943387854; // 3R, the Dulong-Petit ceiling J/(mol*K).

        // HIGH-T (Theta_D/T = 0.1, T = 10*Theta_D): C_v -> 3R. Theta_D = 470, T = 4700.
        let cv_hot = debye_heat_capacity_j_per_mol_k(Fixed::from_int(470), Fixed::from_int(4700));
        assert!(
            close(cv_hot, 24.93, 0.1),
            "high-T C_v approaches the Dulong-Petit 3R ~24.94: {cv_hot:?}"
        );
        assert!(
            cv_hot.to_f64_lossy() < three_r,
            "C_v never exceeds the 3R ceiling"
        );

        // IRON at 300 K, shear-aware Theta_D = 470 (y = 1.567): the Debye C_v ~22.13 J/(mol*K). This sits below
        // the measured ~25 by the documented electronic/anharmonic omission, NOT a mechanism error.
        let cv_iron = debye_heat_capacity_j_per_mol_k(Fixed::from_int(470), Fixed::from_int(300));
        assert!(
            close(cv_iron, 22.13, 0.3),
            "iron Debye C_v at 300 K ~22.1 J/(mol*K): {cv_iron:?}"
        );

        // DEEP-T (Theta_D/T = 25, hits the x_cap=20 branch): C_v ~0.1244 J/(mol*K), and it must match the
        // analytic T^3 asymptote (12*pi^4/5) * R / y^3 that the cap is designed to reproduce. Theta_D = 500, T = 20.
        let cv_cold = debye_heat_capacity_j_per_mol_k(Fixed::from_int(500), Fixed::from_int(20));
        let t3_asymptote =
            12.0 * std::f64::consts::PI.powi(4) / 5.0 * 8.314462618 / 25.0_f64.powi(3);
        assert!(
            close(cv_cold, t3_asymptote, 0.005),
            "deep-T C_v ~{t3_asymptote:.4} matches the T^3 asymptote (cap branch): {cv_cold:?}"
        );

        // Monotone: warmer is a higher heat capacity (smaller y, larger D), bounded by 3R.
        let cv_warmer = debye_heat_capacity_j_per_mol_k(Fixed::from_int(470), Fixed::from_int(600));
        assert!(
            cv_warmer > cv_iron && cv_warmer.to_f64_lossy() < three_r,
            "C_v rises with temperature toward the 3R ceiling"
        );

        // The Debye function itself: D -> 1 at high T, D in (0,1) at finite y, and deterministic (Principle 3).
        let d_hot = debye_function(Fixed::from_ratio(1, 10));
        assert!(
            close(d_hot, 0.9995, 0.001),
            "D(0.1) ~1 (Dulong-Petit): {d_hot:?}"
        );
        let d_iron = debye_function(Fixed::from_ratio(1567, 1000));
        assert!(
            d_iron > ZERO && d_iron < Fixed::ONE,
            "D(1.567) is a fraction in (0,1): {d_iron:?}"
        );
        assert_eq!(d_iron, debye_function(Fixed::from_ratio(1567, 1000)));

        // Guards.
        assert_eq!(
            debye_heat_capacity_j_per_mol_k(ZERO, Fixed::from_int(300)),
            ZERO
        );
        assert_eq!(
            debye_heat_capacity_j_per_mol_k(Fixed::from_int(470), ZERO),
            ZERO
        );

        // Through the route (over the shear-aware Theta_D, reusing the caller's Pugh ratio k = 0.48).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let cv_route = route
            .heat_capacity("Fe", Fixed::from_int(300), Fixed::from_ratio(48, 100))
            .expect("Fe heat capacity");
        assert!(
            cv_route.to_f64_lossy() > 18.0 && cv_route.to_f64_lossy() < three_r,
            "route iron C_v at 300 K is a sensible sub-Dulong-Petit value: {cv_route:?}"
        );
        assert!(
            route
                .heat_capacity("Xx", Fixed::from_int(300), Fixed::from_ratio(48, 100))
                .is_none(),
            "an unanchored metal escalates in the heat-capacity route"
        );
    }

    #[test]
    fn the_shear_strength_is_the_frenkel_ideal_scaled_by_a_per_class_knockdown() {
        // Iron: G ~82 GPa -> the Frenkel ideal tau_th = G/(2*pi) ~13.05 GPa (the dislocation-free upper bound).
        let g_iron = Fixed::from_int(82);
        let tau_th = theoretical_shear_strength_gpa(g_iron);
        assert!(
            close(tau_th, 13.05, 0.1),
            "iron theoretical shear strength G/(2pi) ~13.05 GPa: {tau_th:?}"
        );
        // Annealed iron's operative shear strength is ~0.15 GPa, so the per-class knock-down chi ~0.0115
        // (cited test-only: measured yield over the Frenkel ideal). The operative strength must land ~0.15 GPa.
        let chi_iron = Fixed::from_ratio(115, 10000); // ~0.0115, annealed iron (test-only)
        let tau_op = operative_shear_strength_gpa(g_iron, chi_iron);
        assert!(
            close(tau_op, 0.150, 0.02),
            "iron operative shear strength ~0.15 GPa: {tau_op:?}"
        );
        // A near-ideal covalent solid keeps most of the ideal strength: diamond G ~535, chi ~0.70 -> ~60 GPa.
        let tau_diamond =
            operative_shear_strength_gpa(Fixed::from_int(535), Fixed::from_ratio(70, 100));
        assert!(
            close(tau_diamond, 60.0, 3.0),
            "diamond operative shear strength ~60 GPa (near-ideal): {tau_diamond:?}"
        );
        // The operative strength never exceeds the ideal, and it rises with the knock-down.
        assert!(
            tau_op < tau_th,
            "operative strength is below the Frenkel ideal"
        );
        assert!(
            operative_shear_strength_gpa(g_iron, Fixed::from_ratio(5, 100)) > tau_op,
            "a higher knock-down raises the operative strength"
        );
        // A stiffer material has a higher ideal strength.
        assert!(
            theoretical_shear_strength_gpa(Fixed::from_int(200)) > tau_th,
            "a higher shear modulus raises the ideal strength"
        );
        // Guards: no modulus, or a knock-down outside (0, 1], yields zero; determinism.
        assert_eq!(theoretical_shear_strength_gpa(ZERO), ZERO);
        assert_eq!(operative_shear_strength_gpa(g_iron, ZERO), ZERO);
        assert_eq!(
            operative_shear_strength_gpa(g_iron, Fixed::from_ratio(15, 10)),
            ZERO,
            "a knock-down above 1 is rejected (the operative cannot exceed the ideal)"
        );
        assert_eq!(tau_th, theoretical_shear_strength_gpa(g_iron));

        // Through the route (reads B_0, derives G = k*B_0, reuses the Pugh ratio; knock-down caller-supplied).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let k_fe = Fixed::from_ratio(48, 100);
        let route_tau_th = route
            .theoretical_shear_strength("Fe", k_fe)
            .expect("Fe theoretical shear strength");
        assert!(
            close(route_tau_th, 13.05, 1.0),
            "route iron theoretical shear strength ~13 GPa: {route_tau_th:?}"
        );
        let route_tau_op = route
            .operative_shear_strength("Fe", k_fe, chi_iron)
            .expect("Fe operative shear strength");
        assert!(
            route_tau_op > ZERO && route_tau_op < route_tau_th,
            "route iron operative strength is a positive sub-ideal value: {route_tau_op:?}"
        );
        assert!(
            route.theoretical_shear_strength("Xx", k_fe).is_none(),
            "an unanchored metal escalates in the strength route"
        );
    }

    #[test]
    fn the_thermal_expansion_is_the_gruneisen_relation_over_the_built_heat_capacity() {
        // Iron: gamma_G ~1.7 (cited test-only), C_v ~22.1 J/(mol*K) at 300 K, K = 170 GPa, V_m = 7.09 cm^3/mol.
        // alpha_V = gamma*C_v/(1000*K*V_m) = 1.7*22.1/(1000*170*7.09) ~3.12e-5 /K; alpha_L = alpha_V/3 ~1.04e-5.
        let gamma_fe = Fixed::from_ratio(17, 10); // Grueneisen ~1.7 (iron, test-only)
        let cv = Fixed::from_ratio(221, 10); // ~22.1 J/(mol*K), the built Debye C_v
        let k = Fixed::from_int(170);
        let v_m = Fixed::from_ratio(709, 100);
        let alpha_v = volumetric_thermal_expansion_per_k(gamma_fe, cv, k, v_m);
        assert!(
            close(alpha_v, 3.12e-5, 3.0e-6),
            "iron volumetric expansion ~3.1e-5 /K: {alpha_v:?}"
        );
        let alpha_l = linear_thermal_expansion_per_k(gamma_fe, cv, k, v_m);
        assert!(
            close(alpha_l, 1.04e-5, 1.0e-6),
            "iron linear expansion ~1.04e-5 /K (measured ~1.18e-5): {alpha_l:?}"
        );
        // Linear is one third of volumetric (isotropic).
        assert!(
            close(alpha_l, alpha_v.to_f64_lossy() / 3.0, 1.0e-7),
            "linear is a third of volumetric"
        );
        // Monotone: a higher Grueneisen (more anharmonic) raises the expansion; stiffer K lowers it.
        assert!(
            volumetric_thermal_expansion_per_k(Fixed::from_int(2), cv, k, v_m) > alpha_v,
            "a higher Grueneisen raises expansion"
        );
        assert!(
            volumetric_thermal_expansion_per_k(gamma_fe, cv, Fixed::from_int(400), v_m) < alpha_v,
            "a stiffer bulk modulus lowers expansion"
        );
        // Guards: any non-positive input yields zero.
        assert_eq!(volumetric_thermal_expansion_per_k(ZERO, cv, k, v_m), ZERO);
        assert_eq!(
            volumetric_thermal_expansion_per_k(gamma_fe, ZERO, k, v_m),
            ZERO
        );
        assert_eq!(
            volumetric_thermal_expansion_per_k(gamma_fe, cv, ZERO, v_m),
            ZERO
        );
        assert_eq!(
            volumetric_thermal_expansion_per_k(gamma_fe, cv, k, ZERO),
            ZERO
        );
        // Determinism.
        assert_eq!(
            alpha_v,
            volumetric_thermal_expansion_per_k(gamma_fe, cv, k, v_m)
        );

        // Through the route (reads K/V_m from the anchors, computes C_v via the shear-aware Theta_D internally).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let route_alpha = route
            .linear_thermal_expansion(
                "Fe",
                Fixed::from_int(300),
                Fixed::from_ratio(48, 100),
                gamma_fe,
            )
            .expect("Fe linear expansion");
        assert!(
            close(route_alpha, 1.04e-5, 2.0e-6),
            "route iron linear expansion ~1.0e-5 /K: {route_alpha:?}"
        );
        assert!(
            route
                .linear_thermal_expansion(
                    "Xx",
                    Fixed::from_int(300),
                    Fixed::from_ratio(48, 100),
                    gamma_fe
                )
                .is_none(),
            "an unanchored metal escalates in the expansion route"
        );
    }

    #[test]
    fn the_surface_energy_is_the_broken_bond_model_over_the_cohesive_energy() {
        // Iron: E_coh ~416 kJ/mol (atomization enthalpy), V_atom ~11.77 A^3, f_surf ~0.18 (cited test-only).
        // gamma_sv = f * (1e23/N_A) * E_coh / V_atom^(2/3) = 0.18 * 0.16605 * 416 / 5.16 ~2.4 J/m^2 (measured ~2.4).
        let f_fe = Fixed::from_ratio(18, 100); // surface-bond fraction ~0.18 (iron, test-only)
        let e_coh = Fixed::from_int(416);
        let v_atom = Fixed::from_ratio(1177, 100);
        let gamma = surface_energy_j_per_m2(f_fe, e_coh, v_atom);
        assert!(
            close(gamma, 2.41, 0.15),
            "iron surface energy ~2.4 J/m^2: {gamma:?}"
        );
        // Monotone: more broken-bond fraction, or more cohesion, raises the surface energy; a larger (more open)
        // atomic volume lowers it (fewer bonds per unit area).
        assert!(
            surface_energy_j_per_m2(Fixed::from_ratio(25, 100), e_coh, v_atom) > gamma,
            "a higher surface-bond fraction raises the surface energy"
        );
        assert!(
            surface_energy_j_per_m2(f_fe, Fixed::from_int(500), v_atom) > gamma,
            "more cohesion raises the surface energy"
        );
        assert!(
            surface_energy_j_per_m2(f_fe, e_coh, Fixed::from_int(40)) < gamma,
            "a larger atomic volume lowers the surface energy"
        );
        // A soft, weakly-bound metal (Na: E_coh ~107, V_atom ~39.5) has a far lower surface energy than iron.
        let gamma_na =
            surface_energy_j_per_m2(f_fe, Fixed::from_int(107), Fixed::from_ratio(3949, 100));
        assert!(
            gamma_na > ZERO && gamma_na < gamma,
            "sodium's surface energy is well below iron's: {gamma_na:?}"
        );
        // Guards and determinism.
        assert_eq!(surface_energy_j_per_m2(ZERO, e_coh, v_atom), ZERO);
        assert_eq!(surface_energy_j_per_m2(f_fe, ZERO, v_atom), ZERO);
        assert_eq!(surface_energy_j_per_m2(f_fe, e_coh, ZERO), ZERO);
        assert_eq!(gamma, surface_energy_j_per_m2(f_fe, e_coh, v_atom));

        // Through the route (reads E_coh from the periodic table, V_m from the anchors; f_surf caller-supplied).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let route_gamma = route.surface_energy("Fe", f_fe).expect("Fe surface energy");
        assert!(
            close(route_gamma, 2.41, 0.4),
            "route iron surface energy ~2.4 J/m^2: {route_gamma:?}"
        );
        assert!(
            route.surface_energy("Xx", f_fe).is_none(),
            "an element without an atomization enthalpy or anchor escalates"
        );
    }

    #[test]
    fn the_lattice_conductivity_is_the_slack_model_within_an_order_of_magnitude() {
        // NaCl (simple ionic insulator): gamma=1.6, M_bar=29.22 amu, Theta=321, V_atom=22.36 A^3, n=2, T=300.
        // Slack kappa_L ~7.06 W/(m*K); measured total ~6.5 (an insulator, so lattice IS the total). The DERIVED
        // (factored) prefactor lands closer than the old folded 3.1 (~9.0): the dimensionless-constant law paying off.
        let nacl = lattice_thermal_conductivity_w_per_m_k(
            Fixed::from_ratio(16, 10),
            Fixed::from_ratio(2922, 100),
            Fixed::from_int(321),
            Fixed::from_ratio(2236, 100),
            2,
            Fixed::from_int(300),
        );
        assert!(
            close(nacl, 7.06, 0.6),
            "NaCl lattice conductivity ~7.1 W/(m*K), derived prefactor: {nacl:?}"
        );
        assert!(
            nacl.to_f64_lossy() > 6.5 / 3.0 && nacl.to_f64_lossy() < 6.5 * 3.0,
            "NaCl lands within an order of magnitude (factor 3) of the measured ~6.5"
        );
        // Diamond (the high-conductivity extreme): gamma=0.9, M_bar=12.011, Theta=2230, V_atom=5.674, n=2.
        // Slack kappa_L ~2108 W/(m*K); measured ~2200 (a 4% hit, better than the old folded ~2690). The
        // (Theta/100)^3 fold keeps Theta^3 ~1.1e10 in range.
        let diamond = lattice_thermal_conductivity_w_per_m_k(
            Fixed::from_ratio(9, 10),
            Fixed::from_ratio(12011, 1000),
            Fixed::from_int(2230),
            Fixed::from_ratio(5674, 1000),
            2,
            Fixed::from_int(300),
        );
        assert!(
            close(diamond, 2108.0, 150.0),
            "diamond lattice conductivity ~2108 W/(m*K), derived prefactor: {diamond:?}"
        );
        assert!(
            diamond.to_f64_lossy() > 2200.0 / 3.0 && diamond.to_f64_lossy() < 2200.0 * 3.0,
            "diamond lands within a factor 3 of the measured ~2200"
        );
        // Monotone: higher Theta raises kappa steeply (Theta^3); higher Grueneisen (more anharmonic scattering)
        // lowers it; a higher temperature lowers it (1/T).
        assert!(
            lattice_thermal_conductivity_w_per_m_k(
                Fixed::from_ratio(16, 10),
                Fixed::from_ratio(2922, 100),
                Fixed::from_int(400),
                Fixed::from_ratio(2236, 100),
                2,
                Fixed::from_int(300)
            ) > nacl,
            "a higher Debye temperature raises the lattice conductivity"
        );
        assert!(
            lattice_thermal_conductivity_w_per_m_k(
                Fixed::from_int(3),
                Fixed::from_ratio(2922, 100),
                Fixed::from_int(321),
                Fixed::from_ratio(2236, 100),
                2,
                Fixed::from_int(300)
            ) < nacl,
            "a higher Grueneisen (more anharmonic) lowers the lattice conductivity"
        );
        assert!(
            lattice_thermal_conductivity_w_per_m_k(
                Fixed::from_ratio(16, 10),
                Fixed::from_ratio(2922, 100),
                Fixed::from_int(321),
                Fixed::from_ratio(2236, 100),
                2,
                Fixed::from_int(600)
            ) < nacl,
            "a higher temperature lowers the lattice conductivity (1/T)"
        );
        // Guards: any non-positive input, or fewer than one atom per cell, yields zero; determinism.
        assert_eq!(
            lattice_thermal_conductivity_w_per_m_k(
                ZERO,
                Fixed::from_int(29),
                Fixed::from_int(321),
                Fixed::from_int(22),
                2,
                Fixed::from_int(300)
            ),
            ZERO
        );
        assert_eq!(
            lattice_thermal_conductivity_w_per_m_k(
                Fixed::from_ratio(16, 10),
                Fixed::from_int(29),
                Fixed::from_int(321),
                Fixed::from_int(22),
                0,
                Fixed::from_int(300)
            ),
            ZERO
        );
        assert_eq!(nacl, {
            lattice_thermal_conductivity_w_per_m_k(
                Fixed::from_ratio(16, 10),
                Fixed::from_ratio(2922, 100),
                Fixed::from_int(321),
                Fixed::from_ratio(2236, 100),
                2,
                Fixed::from_int(300),
            )
        });

        // Through the route (a metal: LATTICE component only, the total is electronic and deferred). Fe lattice
        // kappa is a positive figure; the point is it emits the phonon part, flagged, not a fabricated total.
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let fe_lattice = route
            .lattice_thermal_conductivity(
                "Fe",
                Fixed::from_int(300),
                Fixed::from_ratio(48, 100),
                Fixed::from_ratio(17, 10),
            )
            .expect("Fe lattice conductivity");
        assert!(
            fe_lattice > ZERO,
            "iron lattice conductivity is a positive phonon component: {fe_lattice:?}"
        );
        assert!(
            route
                .lattice_thermal_conductivity(
                    "Xx",
                    Fixed::from_int(300),
                    Fixed::from_ratio(48, 100),
                    Fixed::from_ratio(17, 10)
                )
                .is_none(),
            "an unanchored metal escalates in the conductivity route"
        );
    }

    #[test]
    fn the_thermal_diffusivity_composes_conductivity_heat_capacity_and_volume() {
        // Diamond: kappa ~2690 W/(m*K), V_m = 3.417 cm^3/mol, C_v ~4.06 J/(mol*K) at 300 K.
        // alpha = kappa * V_m * 1e-6 / C_v = 2690 * 3.417e-6 / 4.06 ~2.26e-3 m^2/s (measured ~1.2e-3, factor ~1.9,
        // inheriting the conductivity's grade).
        let alpha = thermal_diffusivity_m2_per_s(
            Fixed::from_int(2690),
            Fixed::from_ratio(3417, 1000),
            Fixed::from_ratio(406, 100),
        );
        assert!(
            close(alpha, 2.26e-3, 2.0e-4),
            "diamond thermal diffusivity ~2.26e-3 m^2/s: {alpha:?}"
        );
        // NaCl: kappa ~9, V_m = 26.94, C_v ~47.4 (per formula) -> alpha ~5.1e-6 m^2/s (measured ~3.3e-6).
        let alpha_nacl = thermal_diffusivity_m2_per_s(
            Fixed::from_int(9),
            Fixed::from_ratio(2694, 100),
            Fixed::from_ratio(474, 10),
        );
        assert!(
            close(alpha_nacl, 5.1e-6, 1.0e-6),
            "NaCl thermal diffusivity ~5.1e-6 m^2/s: {alpha_nacl:?}"
        );
        assert!(
            alpha > alpha_nacl,
            "diamond diffuses heat far faster than NaCl"
        );
        // Monotone: more conductivity raises alpha; more heat capacity (more to heat) lowers it.
        assert!(
            thermal_diffusivity_m2_per_s(
                Fixed::from_int(3000),
                Fixed::from_ratio(3417, 1000),
                Fixed::from_ratio(406, 100)
            ) > alpha,
            "a higher conductivity raises the diffusivity"
        );
        assert!(
            thermal_diffusivity_m2_per_s(
                Fixed::from_int(2690),
                Fixed::from_ratio(3417, 1000),
                Fixed::from_int(8)
            ) < alpha,
            "a higher heat capacity lowers the diffusivity"
        );
        // Guards and determinism.
        assert_eq!(
            thermal_diffusivity_m2_per_s(ZERO, Fixed::from_int(3), Fixed::from_int(4)),
            ZERO
        );
        assert_eq!(
            thermal_diffusivity_m2_per_s(Fixed::from_int(2690), Fixed::from_int(3), ZERO),
            ZERO
        );
        assert_eq!(
            alpha,
            thermal_diffusivity_m2_per_s(
                Fixed::from_int(2690),
                Fixed::from_ratio(3417, 1000),
                Fixed::from_ratio(406, 100)
            )
        );

        // Through the route (a metal: lattice-based diffusivity, the electronic total deferred).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let fe_alpha = route
            .thermal_diffusivity(
                "Fe",
                Fixed::from_int(300),
                Fixed::from_ratio(48, 100),
                Fixed::from_ratio(17, 10),
            )
            .expect("Fe thermal diffusivity");
        assert!(
            fe_alpha > ZERO,
            "iron lattice-based thermal diffusivity is positive: {fe_alpha:?}"
        );
        assert!(
            route
                .thermal_diffusivity(
                    "Xx",
                    Fixed::from_int(300),
                    Fixed::from_ratio(48, 100),
                    Fixed::from_ratio(17, 10)
                )
                .is_none(),
            "an unanchored metal escalates in the diffusivity route"
        );
    }

    #[test]
    fn the_grain_boundary_energy_is_a_per_class_fraction_of_the_surface_energy() {
        // Iron: gamma_sv ~2.4 J/m^2, r_gb ~0.33 (cited test-only, high-angle) -> gamma_gb ~0.79 J/m^2 (measured ~0.78).
        let gamma_sv = Fixed::from_ratio(240, 100);
        let r_gb = Fixed::from_ratio(33, 100);
        let gamma_gb = grain_boundary_energy_j_per_m2(gamma_sv, r_gb);
        assert!(
            close(gamma_gb, 0.792, 0.03),
            "iron grain-boundary energy ~0.79 J/m^2: {gamma_gb:?}"
        );
        // A grain boundary always costs less than a free surface (fewer bonds cut).
        assert!(gamma_gb < gamma_sv, "gamma_gb is below gamma_sv");
        // Monotone in the ratio.
        assert!(
            grain_boundary_energy_j_per_m2(gamma_sv, Fixed::from_ratio(40, 100)) > gamma_gb,
            "a higher grain-boundary-to-surface ratio raises gamma_gb"
        );
        // Guards: non-positive inputs, or a ratio above 1 (a boundary cannot cost more than a free surface), yield
        // zero; determinism.
        assert_eq!(grain_boundary_energy_j_per_m2(ZERO, r_gb), ZERO);
        assert_eq!(grain_boundary_energy_j_per_m2(gamma_sv, ZERO), ZERO);
        assert_eq!(
            grain_boundary_energy_j_per_m2(gamma_sv, Fixed::from_ratio(15, 10)),
            ZERO,
            "a ratio above 1 is rejected"
        );
        assert_eq!(gamma_gb, grain_boundary_energy_j_per_m2(gamma_sv, r_gb));

        // Through the route (composes the built surface energy; f_surf and r_gb caller-supplied).
        let t = table();
        let a = anchors();
        let route = PropertyRoute::new(&t, &a);
        let route_gb = route
            .grain_boundary_energy("Fe", Fixed::from_ratio(18, 100), r_gb)
            .expect("Fe grain-boundary energy");
        assert!(
            route_gb > ZERO
                && route_gb
                    < route
                        .surface_energy("Fe", Fixed::from_ratio(18, 100))
                        .unwrap(),
            "route iron gamma_gb is positive and below gamma_sv: {route_gb:?}"
        );
        assert!(
            route
                .grain_boundary_energy("Xx", Fixed::from_ratio(18, 100), r_gb)
                .is_none(),
            "an element without an atomization enthalpy or anchor escalates"
        );
    }

    #[test]
    fn the_slack_prefactor_reassembles_from_fundamental_constants() {
        // The dimensionless-constant law: the old folded 3.1e-6 = C_pure(gamma) * (k_B/hbar)^3 * amu * Angstrom,
        // with only the phase-space number 0.849 cited and the rest derived. Reassembly asserts BOTH directions.

        // C_pure(gamma) = 0.849 * [3*cbrt(4)/(20*pi^3)] * gamma_correction(gamma), fully dimensionless.
        // DIRECTION (structure): at gamma = 2, C_pure ~8.15e-3, matching the demanded 8.32e-3 (from 3.1e-6) at ~2%.
        let gamma = Fixed::from_int(2);
        let c_pure = slack_phase_space_number()
            .checked_mul(slack_geometric_factor())
            .and_then(|x| x.checked_mul(slack_gamma_correction(gamma)))
            .expect("C_pure");
        assert!(
            close(c_pure, 8.15e-3, 3.0e-4),
            "C_pure(2) ~8.15e-3 (demanded 8.32e-3, within ~2%): {c_pure:?}"
        );

        // DIRECTION (folded -> cited): the full rescaled coefficient (dimensional fold * C_pure) rebuilds the old
        // folded ~3.1 within the citation's two significant figures (the derived value is ~3.04).
        let a_rescaled = slack_dimensional_fold_rescaled()
            .checked_mul(c_pure)
            .expect("a_rescaled");
        assert!(
            close(a_rescaled, 3.04, 0.12),
            "the derived prefactor rebuilds the cited ~3.1 (folded), derived ~3.04: {a_rescaled:?}"
        );

        // Rider 1: the coefficient is gamma-DEPENDENT, not frozen at gamma = 2. Diamond (gamma ~0.9) runs higher.
        let c_diamond = slack_phase_space_number()
            .checked_mul(slack_geometric_factor())
            .and_then(|x| x.checked_mul(slack_gamma_correction(Fixed::from_ratio(9, 10))))
            .expect("C_pure diamond");
        assert!(
            c_diamond > c_pure,
            "C_pure carries the gamma-dependence (diamond above gamma=2), not a frozen scalar"
        );

        // The dimensional fold assembles to the expected ~372.6 (mantissa product * 10^2), representable.
        assert!(
            close(slack_dimensional_fold_rescaled(), 372.6, 1.0),
            "the (k_B/hbar)^3 * amu * Angstrom * 1e6 fold assembles to ~372.6: {:?}",
            slack_dimensional_fold_rescaled()
        );
    }
}
