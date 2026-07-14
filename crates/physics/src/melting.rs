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

//! MELTING as the kernel's third phase type (rung 0, ideal solution): the liquidus, the eutectic, and the
//! melt fraction of a binary, DERIVED-IN-FORM from each endmember's two-number melting signature `(T_m,
//! dH_fus)` rather than read from a fitted melting curve. This is the melting sibling of the boiling turn's
//! two-number theorem (`(T_boil, dH_vap)` drives the Rankine-Kirchhoff saturation curve): the same
//! Clausius-Clapeyron form at the other phase boundary, so the gas-liquid-solid triple closes on one
//! machinery.
//!
//! The physics, from the equality of chemical potentials with no fitted constant. A pure solid endmember `i`
//! is in equilibrium with an ideal liquid of mole fraction `x_i` along its SATURATION curve (Schroeder-van
//! Laar): `ln x_i = -(dH_fus,i / R) * (1/T - 1/T_m,i)`, so `x_i(T) = exp[-(dH_fus,i/R)(1/T - 1/T_m,i)]`. In a
//! binary, the two endmember curves fall from their pure melting points and CROSS at the EUTECTIC, the lowest
//! temperature at which any liquid exists, where the liquid is saturated in BOTH solids: `x_A(T) + x_B(T) =
//! 1`. Nothing about the eutectic is authored; it emerges where the two saturation curves meet. Between the
//! solidus (the eutectic temperature for a bulk in the eutectic field) and the liquidus, the MELT FRACTION is
//! the LEVER RULE, so `F(T, X)` is an OUTPUT of the phase geometry, never an input, and the eruption arc's
//! productivity reads it.
//!
//! The GRADE is ideal-solution, declared and self-checked, never hidden. Run on petrology's founding system,
//! diopside-anorthite, the ideal eutectic lands near 1608 K at `x_An` about 0.30 against Bowen's measured
//! 1547 K and about 0.36: roughly 60 K and 20 percent off, exactly the ideal-solution band, plainly
//! labelled (the calibrated non-ideality, the Margules excess terms, is the next rung). The `(T_m, dH_fus)`
//! per endmember are the whole information content of this rung, banked columns (`T_m` by the Lindemann
//! criterion or measured, `dH_fus` by Richard's rule or measured), so the law here consumes them and derives
//! the phase diagram above them.
//!
//! Determinism (Principle 3): the molar gas constant derives once from the CODATA fundamentals
//! (`R = N_A * k_B`, never an authored decimal), and the arithmetic is fixed-point throughout (the
//! exponential through the pinned [`Fixed::exp`], a bounded bisection for the eutectic), so every result is a
//! pure function of the endmember signatures.

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use civsim_units::fundamentals;
use std::sync::OnceLock;

/// The molar gas constant `R = N_A * k_B` (J/mol/K), DERIVED once from the two CODATA fundamentals and
/// projected to `Fixed`, never an authored decimal. Memoized; a pure load constant.
fn molar_gas_constant() -> Fixed {
    static R: OnceLock<Fixed> = OnceLock::new();
    *R.get_or_init(|| {
        let n_a = BigRat::from_decimal_str(
            fundamentals::fundamental("N_A")
                .expect("Avogadro is a registered fundamental")
                .value,
        )
        .expect("Avogadro parses");
        let k_b = BigRat::from_decimal_str(
            fundamentals::fundamental("k_B")
                .expect("Boltzmann is a registered fundamental")
                .value,
        )
        .expect("Boltzmann parses");
        Fixed::from_bits_i128(
            n_a.mul(&k_b)
                .round_to_scale(Fixed::FRAC_BITS)
                .expect("R = N_A k_B ~ 8.314 fits Q32.32"),
        )
        .expect("R projects to Fixed")
    })
}

/// An endmember's two-number melting signature, the whole information content of the ideal rung.
#[derive(Clone, Copy, Debug)]
pub struct Endmember {
    /// The pure-endmember melting point `T_m` (kelvin), a banked column (Lindemann or measured).
    pub melting_point_k: Fixed,
    /// The molar enthalpy of fusion `dH_fus` (joules per mole), a banked column (Richard's rule or measured).
    pub fusion_enthalpy_j_per_mol: Fixed,
}

/// The mole fraction `x_i` of endmember `i` in an ideal liquid saturated with the pure solid `i` at a
/// temperature, the Schroeder-van Laar liquidus `x_i = exp[-(dH_fus/R)(1/T - 1/T_m)]`. At or above the
/// endmember's melting point the solid cannot saturate the liquid, so the result clamps to one (a pure liquid
/// of `i` is stable). `None` on a non-positive temperature, melting point, or a negative fusion enthalpy.
pub fn liquidus_mole_fraction(endmember: Endmember, temperature_k: Fixed) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO
        || endmember.melting_point_k <= Fixed::ZERO
        || endmember.fusion_enthalpy_j_per_mol < Fixed::ZERO
    {
        return None;
    }
    if temperature_k >= endmember.melting_point_k {
        return Some(Fixed::ONE);
    }
    let r = molar_gas_constant();
    // (1/T - 1/T_m), a small positive difference below the melting point.
    let inv_t = Fixed::ONE.checked_div(temperature_k)?;
    let inv_tm = Fixed::ONE.checked_div(endmember.melting_point_k)?;
    let diff = inv_t.checked_sub(inv_tm)?;
    // arg = -(dH_fus/R) * (1/T - 1/T_m).
    let dh_over_r = endmember.fusion_enthalpy_j_per_mol.checked_div(r)?;
    let arg = Fixed::ZERO.checked_sub(dh_over_r.checked_mul(diff)?)?;
    Some(arg.exp())
}

/// The number of bisection steps the eutectic solve takes. The saturation-sum is monotone in temperature over
/// the bracket, so the bisection halves the interval each step to the fixed-point floor; an engine-convergence
/// bound, not world content.
const EUTECTIC_BISECTION_STEPS: u32 = 52;

/// The binary EUTECTIC of two ideal endmembers: the temperature at which both saturation curves meet
/// (`x_A + x_B = 1`, the liquid saturated in both solids) and the liquid's mole fraction of `B` there. The
/// eutectic is the lowest temperature at which any liquid exists, and it EMERGES from the crossing, never
/// authored. Returns `(T_eutectic, x_B_eutectic)`. `None` if either endmember is non-physical or the bracket
/// is degenerate.
pub fn binary_eutectic(a: Endmember, b: Endmember) -> Option<(Fixed, Fixed)> {
    // Bracket: from a low temperature up to the lower of the two melting points (above it, that solid can no
    // longer saturate, so the eutectic lies below both). The saturation sum x_A + x_B rises with temperature.
    let hi0 = if a.melting_point_k < b.melting_point_k {
        a.melting_point_k
    } else {
        b.melting_point_k
    };
    if hi0 <= Fixed::ZERO {
        return None;
    }
    let mut lo = hi0.checked_div(Fixed::from_int(2))?; // half the lower melting point, safely below the eutectic
    let mut hi = hi0;
    let two = Fixed::from_int(2);
    let sat_minus_one = |t: Fixed| -> Option<Fixed> {
        let xa = liquidus_mole_fraction(a, t)?;
        let xb = liquidus_mole_fraction(b, t)?;
        xa.checked_add(xb)?.checked_sub(Fixed::ONE)
    };
    for _ in 0..EUTECTIC_BISECTION_STEPS {
        let mid = lo.checked_add(hi)?.checked_div(two)?;
        // sat rises with T, so sat-1 < 0 below the eutectic and > 0 above it.
        if sat_minus_one(mid)? > Fixed::ZERO {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let t_e = lo.checked_add(hi)?.checked_div(two)?;
    let x_b = liquidus_mole_fraction(b, t_e)?;
    Some((t_e, x_b))
}

/// The equilibrium MELT FRACTION `F` of a binary of bulk composition `bulk_fraction_b` (the mole fraction of
/// endmember `B`) at a temperature, by the LEVER RULE. Below the eutectic the assemblage is all solid
/// (`F = 0`); at and above this bulk's liquidus it is all liquid (`F = 1`); between, one solid coexists with a
/// liquid on the SATURATION branch of that solid, and `F` is the lever between them. Which solid survives, and
/// so which branch the liquid follows, is read from the emergent eutectic composition rather than authored: a
/// bulk poorer in `B` than the eutectic is `A`-primary (the residual solid is pure `A`, the liquid on the
/// `A`-saturated branch whose `B`-fraction is `1 - x_A^liq(T)`, so `F = X_B / (1 - x_A^liq)`); a bulk richer in
/// `B` is `B`-primary (the mirror: residual pure `B`, liquid on the `B`-saturated branch, so
/// `F = (1 - X_B) / (1 - x_B^liq)`). `None` on a non-physical endmember or a bulk fraction outside `[0, 1]`.
pub fn batch_melt_fraction(
    a: Endmember,
    b: Endmember,
    bulk_fraction_b: Fixed,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if bulk_fraction_b < Fixed::ZERO || bulk_fraction_b > Fixed::ONE {
        return None;
    }
    let (t_e, x_b_e) = binary_eutectic(a, b)?;
    if temperature_k < t_e {
        return Some(Fixed::ZERO);
    }
    // The eutectic composition is the boundary between the two primary fields, itself an emergent output, so
    // which solid is the residual and which saturation branch the liquid follows is read from it, not authored.
    if bulk_fraction_b <= x_b_e {
        // A-primary: the residual solid is pure A, and the liquid rides the A-saturated branch, whose
        // B-fraction is 1 - x_A^liq(T). As T rises above the eutectic, x_A^liq rises, so this falls toward the
        // bulk value; the liquidus for this bulk is where it meets it (all molten).
        let x_a_liq = liquidus_mole_fraction(a, temperature_k)?;
        let x_b_liq = Fixed::ONE.checked_sub(x_a_liq)?;
        if x_b_liq <= bulk_fraction_b {
            return Some(Fixed::ONE);
        }
        // Lever rule with pure-A solid (its B-fraction is zero): F = (X_B - 0)/(x_B^liq - 0).
        bulk_fraction_b.checked_div(x_b_liq)
    } else {
        // B-primary: the mirror. The residual solid is pure B, and the liquid rides the B-saturated branch,
        // whose B-fraction is x_B^liq(T) directly, rising toward the bulk value as T climbs to the liquidus.
        let x_b_liq = liquidus_mole_fraction(b, temperature_k)?;
        if x_b_liq >= bulk_fraction_b {
            return Some(Fixed::ONE);
        }
        // Lever rule with pure-B solid (its B-fraction is one): F = (1 - X_B)/(1 - x_B^liq).
        let bulk_a = Fixed::ONE.checked_sub(bulk_fraction_b)?;
        let liq_a = Fixed::ONE.checked_sub(x_b_liq)?;
        bulk_a.checked_div(liq_a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }
    // Diopside and anorthite, the memory-flagged two-number signatures the ideal rung is validated on.
    fn diopside() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(1665),
            fusion_enthalpy_j_per_mol: Fixed::from_int(138_000),
        }
    }
    fn anorthite() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(1830),
            fusion_enthalpy_j_per_mol: Fixed::from_int(133_000),
        }
    }

    #[test]
    fn the_gas_constant_derives_from_the_fundamentals() {
        assert!(
            close(molar_gas_constant(), 8.314, 1e-3),
            "R = N_A k_B ~ 8.314, got {}",
            molar_gas_constant().to_f64_lossy()
        );
    }

    #[test]
    fn a_pure_endmember_saturates_fully_at_its_melting_point() {
        // At T_m the liquidus mole fraction is one (pure melt); below it, less than one and falling.
        let di = diopside();
        assert_eq!(
            liquidus_mole_fraction(di, di.melting_point_k),
            Some(Fixed::ONE)
        );
        let x_below = liquidus_mole_fraction(di, Fixed::from_int(1600))
            .unwrap()
            .to_f64_lossy();
        assert!(
            x_below < 1.0 && x_below > 0.0,
            "below the melting point the solid saturates a liquid of x < 1, got {x_below}"
        );
    }

    #[test]
    fn the_diopside_anorthite_eutectic_lands_in_the_ideal_solution_band() {
        // The pre-registered gate: the ideal eutectic near 1608 K at x_An ~ 0.30, against Bowen's measured
        // 1547 K and ~0.36 (60 K and 20% off, the ideal-solution band, plainly labelled).
        let (t_e, x_an) = binary_eutectic(diopside(), anorthite()).expect("the curves cross");
        assert!(
            close(t_e, 1608.0, 4.0),
            "ideal eutectic temperature ~ 1608 K, got {}",
            t_e.to_f64_lossy()
        );
        assert!(
            close(x_an, 0.30, 0.01),
            "ideal eutectic composition x_An ~ 0.30, got {}",
            x_an.to_f64_lossy()
        );
        // And it sits below both pure melting points, as a eutectic must.
        assert!(t_e.to_f64_lossy() < 1665.0 && t_e.to_f64_lossy() < 1830.0);
    }

    #[test]
    fn the_melt_fraction_rises_from_zero_at_the_solidus_to_one_at_the_liquidus() {
        // The lever rule: an An-poor bulk (x_An = 0.15) is all solid below the eutectic, partially molten just
        // above it, and fully molten at its liquidus, F an output of the phase geometry.
        let (di, an) = (diopside(), anorthite());
        let f_below =
            batch_melt_fraction(di, an, Fixed::from_ratio(15, 100), Fixed::from_int(1600)).unwrap();
        let f_just =
            batch_melt_fraction(di, an, Fixed::from_ratio(15, 100), Fixed::from_int(1610)).unwrap();
        let f_high =
            batch_melt_fraction(di, an, Fixed::from_ratio(15, 100), Fixed::from_int(1700)).unwrap();
        assert_eq!(f_below, Fixed::ZERO, "all solid below the eutectic");
        assert!(
            f_just.to_f64_lossy() > 0.0 && f_just.to_f64_lossy() < 1.0,
            "partial melt just above the eutectic, got {}",
            f_just.to_f64_lossy()
        );
        assert_eq!(f_high, Fixed::ONE, "all liquid at the liquidus");
        assert!(
            f_high >= f_just && f_just >= f_below,
            "F rises monotonically with temperature"
        );
    }

    #[test]
    fn the_melt_fraction_mirrors_on_the_b_primary_side() {
        // An An-RICH bulk (x_An = 0.50, richer in anorthite than the eutectic's ~0.30) sits in the
        // anorthite-primary field: pure anorthite is the residual solid, the liquid rides the anorthite-
        // saturated branch, and F climbs from the eutectic lever value to one at this bulk's liquidus.
        let (di, an) = (diopside(), anorthite());
        let bulk = Fixed::from_ratio(50, 100);
        let f_below = batch_melt_fraction(di, an, bulk, Fixed::from_int(1600)).unwrap();
        let f_just = batch_melt_fraction(di, an, bulk, Fixed::from_int(1610)).unwrap();
        let f_high = batch_melt_fraction(di, an, bulk, Fixed::from_int(1750)).unwrap();
        assert_eq!(f_below, Fixed::ZERO, "all solid below the eutectic");
        assert!(
            f_just.to_f64_lossy() > 0.5 && f_just.to_f64_lossy() < 1.0,
            "partial melt just above the eutectic, above the lever floor (1-X)/(1-x_e) ~ 0.71, got {}",
            f_just.to_f64_lossy()
        );
        assert_eq!(f_high, Fixed::ONE, "all liquid past this bulk's liquidus");
        assert!(
            f_high >= f_just && f_just >= f_below,
            "F rises monotonically with temperature"
        );
    }

    #[test]
    fn it_is_deterministic_and_guards_its_inputs() {
        let di = diopside();
        assert_eq!(
            liquidus_mole_fraction(di, Fixed::from_int(1600)),
            liquidus_mole_fraction(di, Fixed::from_int(1600)),
            "the liquidus replays byte for byte"
        );
        assert_eq!(liquidus_mole_fraction(di, Fixed::ZERO), None);
        assert_eq!(
            batch_melt_fraction(di, anorthite(), Fixed::from_int(2), Fixed::from_int(1700)),
            None,
            "a bulk fraction outside [0,1] is rejected"
        );
    }
}
