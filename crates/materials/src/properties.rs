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

use civsim_core::Fixed;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::rose_eos;

use crate::freezer;

const ZERO: Fixed = Fixed::ZERO;

/// The mass density `rho = M / V_m` (g/cm^3), the molar mass (g/mol) over the molar volume (cm^3/mol): a pure
/// ratio of floor data, no reserved value. This is the density the freezer's sound speed reads. Non-positive
/// inputs yield zero (no density without a mass and a volume).
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
/// turf) and is LESS accurate for soft low-`k` metals (iron's intrinsic ~8 GPa against a much softer annealed
/// ~1 GPa, the same low-`k` metallic-bonding case Chen-Tse note). Non-positive inputs yield zero.
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
}
