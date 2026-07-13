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
//! HONEST LIMIT (the bulk-elastic approximation, carried from the freezer's `T_m`): the true Debye temperature
//! uses the DEBYE-AVERAGED sound velocity `v_D` (over the longitudinal and the two transverse modes), which
//! needs the shear modulus. With only the bulk modulus `B_0` among the anchors, `c_s = sqrt(B_0/rho)` is the
//! BULK sound speed, which is faster than `v_D`, so `Theta_D` is OVERESTIMATED (iron lands about 609 K from the
//! bulk speed against a measured `Theta_D` near 470 K, roughly 30 percent high). The shear-aware `v_D` (and the
//! iron-accurate `Theta_D`) is the follow-on when a shear modulus is anchored, the same elastic limit the
//! Lindemann `T_m` names; the Slack conductivity downstream inherits it (it scales as `Theta_D^3`), stated at
//! its site when built. Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.

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
}
