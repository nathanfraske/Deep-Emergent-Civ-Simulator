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
//! The PRESSURE extension adds one more number, `dV_fus` (the molar volume change on fusion), from which the
//! Clapeyron slope `dT_m/dP = dV_fus / (dH_fus/T_m)` DERIVES as a thermodynamic identity (not a fit), so the
//! solidus moves with depth and the surface machinery runs unchanged on the shifted melting points: the
//! diopside slope lands near 63 K/GPa (measured 60 to 75), the Di-An eutectic climbs to about 1668 K at 1 GPa,
//! and the decompression-melting productivity the eruption column reads is `dF/dP` off this. A NEGATIVE
//! `dV_fus` (water ice, the denser-melt anomaly) lowers the melting point with pressure instead, the derived
//! root of the cryovolcanism buoyancy problem, on the same law.
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

/// An endmember's melting signature. The surface rung reads the two-number `(T_m, dH_fus)`; the pressure
/// extension adds the third, `dV_fus`, the molar volume change on fusion (a measured `[M]` material property,
/// the same character as `dH_fus`), from which the Clapeyron slope `dT_m/dP = dV_fus / (dH_fus/T_m)` DERIVES
/// (a thermodynamic identity, not a fit), so the solidus moves with depth. A positive `dV_fus` (the melt is
/// less dense, the silicate case) raises the melting point with pressure; a NEGATIVE one (the melt is denser,
/// the water-ice anomaly) lowers it, the derived root of the cryovolcanism buoyancy problem.
#[derive(Clone, Copy, Debug)]
pub struct Endmember {
    /// The pure-endmember melting point `T_m` (kelvin), a banked column (Lindemann or measured).
    pub melting_point_k: Fixed,
    /// The molar enthalpy of fusion `dH_fus` (joules per mole), a banked column (Richard's rule or measured).
    pub fusion_enthalpy_j_per_mol: Fixed,
    /// The molar volume change on fusion `dV_fus` (cubic centimetres per mole, the petrology convention), a
    /// measured `[M]` column. Positive for a less-dense melt (raises `T_m` with pressure), negative for a
    /// denser melt (lowers it). Zero leaves the endmember pressure-insensitive (the surface-only rung).
    pub fusion_volume_cm3_per_mol: Fixed,
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

/// The unclamped partition ratio `K_i = x_i^solid / x_i^liquid = exp[(dH_fus,i/R)(1/T - 1/T_m,i)]` of an
/// endmember between an ideal solid solution and an ideal liquid, the Turnbull form of the equal-chemical-
/// potential condition with `dG_fus = dH_fus(1 - T/T_m)` (the same `dCp = 0` grade as the Schroeder-van Laar
/// liquidus, which is this ratio's reciprocal at a pure solid). Below the melting point `K > 1` (the solid is
/// enriched in `i`); above it `K < 1`. Unlike [`liquidus_mole_fraction`] it does NOT clamp, since the solid-
/// solution loop needs the raw ratio on both sides of `T_m`. `None` on a non-physical input.
fn partition_ratio(endmember: Endmember, temperature_k: Fixed) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO
        || endmember.melting_point_k <= Fixed::ZERO
        || endmember.fusion_enthalpy_j_per_mol < Fixed::ZERO
    {
        return None;
    }
    let r = molar_gas_constant();
    let inv_t = Fixed::ONE.checked_div(temperature_k)?;
    let inv_tm = Fixed::ONE.checked_div(endmember.melting_point_k)?;
    let diff = inv_t.checked_sub(inv_tm)?;
    let dh_over_r = endmember.fusion_enthalpy_j_per_mol.checked_div(r)?;
    Some(dh_over_r.checked_mul(diff)?.exp())
}

/// Clamp a fraction to the unit interval `[0, 1]`, the tail-truncation of a lever rule outside its two-phase
/// window.
fn clamp_unit(v: Fixed) -> Fixed {
    if v < Fixed::ZERO {
        Fixed::ZERO
    } else if v > Fixed::ONE {
        Fixed::ONE
    } else {
        v
    }
}

/// The binary SOLID-SOLUTION LOOP of two ideal endmembers that mix in BOTH the solid and the liquid (a
/// complete solid solution, the olivine forsterite-fayalite case, the other binary topology beside the
/// eutectic of pure immiscible solids). At a temperature strictly between the two pure melting points the
/// liquid and a coexisting solid solution have DIFFERENT compositions, the liquidus and solidus of a lens:
/// solving the two equal-chemical-potential conditions (one per endmember, `x_i^s = K_i x_i^l`) with each
/// phase summing to one gives `x_B^liq = (K_A - 1)/(K_A - K_B)` and `x_B^sol = K_B * x_B^liq` in closed form,
/// no iteration. Returns `(x_B^liquidus, x_B^solidus)`, the liquid and solid `B`-fractions. `None` outside the
/// melting-point interval (no two-phase equilibrium there) or if the two endmembers share a signature (a
/// degenerate loop). Whether a pair forms a loop or a eutectic is a property of the pair (their solid
/// miscibility), read from the pair rather than authored; this is the loop branch, [`binary_eutectic`] the
/// eutectic branch.
pub fn binary_solid_solution_loop(
    a: Endmember,
    b: Endmember,
    temperature_k: Fixed,
) -> Option<(Fixed, Fixed)> {
    let (tm_lo, tm_hi) = if a.melting_point_k < b.melting_point_k {
        (a.melting_point_k, b.melting_point_k)
    } else {
        (b.melting_point_k, a.melting_point_k)
    };
    if temperature_k < tm_lo || temperature_k > tm_hi {
        return None;
    }
    let k_a = partition_ratio(a, temperature_k)?;
    let k_b = partition_ratio(b, temperature_k)?;
    let denom = k_a.checked_sub(k_b)?;
    if denom == Fixed::ZERO {
        return None; // identical signatures: no lens
    }
    let x_b_liq = k_a.checked_sub(Fixed::ONE)?.checked_div(denom)?;
    let x_b_sol = k_b.checked_mul(x_b_liq)?;
    Some((clamp_unit(x_b_liq), clamp_unit(x_b_sol)))
}

/// The equilibrium MELT FRACTION `F` of a binary complete SOLID SOLUTION of bulk `B`-fraction `bulk_fraction_b`
/// at a temperature, by the LEVER RULE across the lens of [`binary_solid_solution_loop`]. Below the lower pure
/// melting point the whole system is one solid solution (`F = 0`); above the higher it is all liquid
/// (`F = 1`); between, a solid of composition `x_B^sol` coexists with a liquid of `x_B^liq`, and
/// `F = (X_B - x_B^sol)/(x_B^liq - x_B^sol)`, clamped to `[0, 1]` for a bulk outside this temperature's two-
/// phase window. The lever is orientation-independent (it reads the same whichever endmember is the higher-
/// melting one), so the caller need not order the pair. `None` on a non-physical endmember, a degenerate loop,
/// or a bulk fraction outside `[0, 1]`.
pub fn solution_melt_fraction(
    a: Endmember,
    b: Endmember,
    bulk_fraction_b: Fixed,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if bulk_fraction_b < Fixed::ZERO || bulk_fraction_b > Fixed::ONE {
        return None;
    }
    let (tm_lo, tm_hi) = if a.melting_point_k < b.melting_point_k {
        (a.melting_point_k, b.melting_point_k)
    } else {
        (b.melting_point_k, a.melting_point_k)
    };
    if tm_lo <= Fixed::ZERO {
        return None;
    }
    if temperature_k <= tm_lo {
        return Some(Fixed::ZERO);
    }
    if temperature_k >= tm_hi {
        return Some(Fixed::ONE);
    }
    let (x_b_liq, x_b_sol) = binary_solid_solution_loop(a, b, temperature_k)?;
    let numer = bulk_fraction_b.checked_sub(x_b_sol)?;
    let denom = x_b_liq.checked_sub(x_b_sol)?;
    if denom == Fixed::ZERO {
        return None;
    }
    Some(clamp_unit(numer.checked_div(denom)?))
}

/// The pressure-shifted melting point `T_m(P) = T_m + (dV_fus / dS_fus) * P` of an endmember, the linear
/// CLAPEYRON law with `dS_fus = dH_fus/T_m` (the rung-0 grade: `dV_fus` and `dS_fus` taken constant). The
/// pressure is in bar (the petrology convention), the volume in cubic centimetres per mole, and the unit
/// bridge `1 cm^3.bar = 0.1 J` matches the petrology Gibbs pressure-work term. A positive `dV_fus` raises the
/// melting point with pressure; a negative one lowers it (the ice anomaly). `None` on a non-physical
/// endmember (a non-positive melting point or fusion enthalpy, so the entropy of fusion is undefined).
pub fn melting_point_at_pressure(endmember: Endmember, pressure_bar: Fixed) -> Option<Fixed> {
    if endmember.melting_point_k <= Fixed::ZERO
        || endmember.fusion_enthalpy_j_per_mol <= Fixed::ZERO
    {
        return None;
    }
    // dS_fus = dH_fus / T_m (joules per mole per kelvin).
    let ds = endmember
        .fusion_enthalpy_j_per_mol
        .checked_div(endmember.melting_point_k)?;
    if ds == Fixed::ZERO {
        return None;
    }
    // dT_m = (dV[cm^3] * P[bar] * 0.1 J/cm^3.bar) / dS.
    let work = endmember
        .fusion_volume_cm3_per_mol
        .checked_mul(pressure_bar)?
        .checked_mul(Fixed::from_ratio(1, 10))?;
    let shift = work.checked_div(ds)?;
    endmember.melting_point_k.checked_add(shift)
}

/// An endmember with its melting point shifted to the Clapeyron value at a pressure, so the surface machinery
/// (liquidus, eutectic, melt fraction) runs unchanged at depth: the pressure enters ONLY through the moved
/// melting point, which reuses every surface derivation rather than duplicating it. `None` if the shift is
/// non-physical.
fn at_pressure(endmember: Endmember, pressure_bar: Fixed) -> Option<Endmember> {
    Some(Endmember {
        melting_point_k: melting_point_at_pressure(endmember, pressure_bar)?,
        ..endmember
    })
}

/// The binary EUTECTIC at a pressure: [`binary_eutectic`] on the two Clapeyron-shifted endmembers, so the
/// eutectic temperature rises with depth as the pure melting points do. `None` on a non-physical input.
pub fn binary_eutectic_at_pressure(
    a: Endmember,
    b: Endmember,
    pressure_bar: Fixed,
) -> Option<(Fixed, Fixed)> {
    binary_eutectic(at_pressure(a, pressure_bar)?, at_pressure(b, pressure_bar)?)
}

/// The batch MELT FRACTION at a pressure: [`batch_melt_fraction`] on the two Clapeyron-shifted endmembers. At
/// a fixed temperature a positive-`dV` system melts LESS at depth (the solidus has risen past it), the
/// productivity term the decompression-melting column reads. `None` on a non-physical input.
pub fn batch_melt_fraction_at_pressure(
    a: Endmember,
    b: Endmember,
    bulk_fraction_b: Fixed,
    temperature_k: Fixed,
    pressure_bar: Fixed,
) -> Option<Fixed> {
    batch_melt_fraction(
        at_pressure(a, pressure_bar)?,
        at_pressure(b, pressure_bar)?,
        bulk_fraction_b,
        temperature_k,
    )
}

/// The number of bisection steps the multi-component solidus solve takes, the same fixed-point-floor
/// convergence bound as the eutectic; engine convergence, not world content.
const SOLIDUS_BISECTION_STEPS: u32 = 52;

/// The MULTI-SATURATION SOLIDUS of an open assemblage of ideal endmembers at a pressure: the temperature at
/// which a single liquid is simultaneously saturated in EVERY solid, `sum_i x_i(T, P) = 1`, the generalization
/// of the binary eutectic (`x_A + x_B = 1`) to N components. This is the temperature at which the first liquid
/// appears in a fertile assemblage, so a peridotite (olivine, pyroxene, an aluminous phase) has its solidus
/// derived from its mineral signatures rather than authored. The assemblage is a SLICE, so its membership is
/// data that grows with the world (Principle 11): a new phase is a new row, not a code change, and an alien
/// assemblage is the same call. Each added component deepens the eutectic depression, so the solidus falls
/// below every pure melting point. The pressure enters through the Clapeyron shift, so the solidus rises with
/// depth. `None` on an empty assemblage or a non-physical endmember. The grade is ideal-solution: on the four-
/// mineral lherzolite assemblage the solidus lands near 1520 K against the measured ~1373 K (about 150 K high,
/// the ideal-solution band, the rung-1 calibration target), plainly labelled.
pub fn multicomponent_solidus(endmembers: &[Endmember], pressure_bar: Fixed) -> Option<Fixed> {
    if endmembers.is_empty() {
        return None;
    }
    // Shift every endmember to the pressure so the surface saturation curves run on the moved melting points.
    let mut shifted: Vec<Endmember> = Vec::with_capacity(endmembers.len());
    for em in endmembers {
        shifted.push(at_pressure(*em, pressure_bar)?);
    }
    // The saturation sum over the shifted endmembers; it rises monotonically with temperature.
    let sat = |t: Fixed| -> Option<Fixed> {
        let mut s = Fixed::ZERO;
        for em in &shifted {
            s = s.checked_add(liquidus_mole_fraction(*em, t)?)?;
        }
        Some(s)
    };
    // The upper bracket is the lowest shifted melting point: there the lowest phase saturates fully, so the
    // sum is at least one. The solidus lies at or below it.
    let mut hi = shifted[0].melting_point_k;
    for em in &shifted {
        if em.melting_point_k < hi {
            hi = em.melting_point_k;
        }
    }
    if hi <= Fixed::ZERO {
        return None;
    }
    let two = Fixed::from_int(2);
    // Find a lower bracket where the sum drops below one, halving down from the upper bound (more components
    // push the solidus lower, so the bracket may need to widen).
    let mut lo = hi;
    let mut bracketed = false;
    for _ in 0..16 {
        lo = lo.checked_div(two)?;
        if sat(lo)? < Fixed::ONE {
            bracketed = true;
            break;
        }
    }
    if !bracketed {
        return None;
    }
    let mut hi_b = hi;
    for _ in 0..SOLIDUS_BISECTION_STEPS {
        let mid = lo.checked_add(hi_b)?.checked_div(two)?;
        if sat(mid)? > Fixed::ONE {
            hi_b = mid;
        } else {
            lo = mid;
        }
    }
    lo.checked_add(hi_b)?.checked_div(two)
}

/// The FIRST-MELT (eutectic) LIQUID COMPOSITION of an assemblage: the mole fraction of each endmember in the
/// liquid at the multi-saturation solidus, in the input order, normalized to sum to one. At the solidus the
/// saturation curves already sum to one (that is what [`multicomponent_solidus`] solves), so each endmember's
/// `x_i = liquidus_mole_fraction(i, T_sol)` IS its share of the first liquid: the low-melting endmembers
/// dominate it and the high-melting ones are left in the residue. This is how the crust DERIVES from the
/// mantle rather than being authored: a peridotite's first melt comes out enriched in the fusible minerals
/// (clinopyroxene, plagioclase: a basalt) and depleted in olivine, so the melt is the crust and the residue is
/// the refractory mantle, both emergent from the same signatures. `None` on an empty or non-physical
/// assemblage. Returns the composition paired with the solidus temperature it was taken at.
pub fn eutectic_liquid_composition(
    endmembers: &[Endmember],
    pressure_bar: Fixed,
) -> Option<(Fixed, Vec<Fixed>)> {
    let t_sol = multicomponent_solidus(endmembers, pressure_bar)?;
    let mut xs: Vec<Fixed> = Vec::with_capacity(endmembers.len());
    let mut total = Fixed::ZERO;
    for em in endmembers {
        let shifted = at_pressure(*em, pressure_bar)?;
        let x = liquidus_mole_fraction(shifted, t_sol)?;
        total = total.checked_add(x)?;
        xs.push(x);
    }
    if total <= Fixed::ZERO {
        return None;
    }
    for x in &mut xs {
        *x = x.checked_div(total)?; // normalize the tiny fixed-point residual to an exact unit sum
    }
    Some((t_sol, xs))
}

/// The result of an adiabatic decompression melting column: the crustal thickness it produces, the melt
/// fraction at the top of the column, and the pressure at which melting began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeltColumn {
    /// The crustal thickness (kilometres) the pooled melt makes.
    pub crust_thickness_km: Fixed,
    /// The melt fraction at the shallowest, most-molten top of the column.
    pub max_melt_fraction: Fixed,
    /// The pressure (gigapascals) at which the rising mantle first crossed the solidus and began to melt.
    pub onset_pressure_gpa: Fixed,
}

/// The ADIABATIC DECOMPRESSION MELTING column beneath a divergent boundary (McKenzie-Bickle 1988), the closure
/// that turns the melt machinery into crust. Mantle at potential temperature `T_p` rises on the adiabat
/// `T(P) = T_p + m_ad * P` and begins to melt where it crosses the solidus `T_sol(P) = T_sol0 + m_sol * P`;
/// above that depth the melt fraction climbs at the productivity `dF/dP`, and the pooled melt forms a crust of
/// thickness `crust = (dF/dP) * P0^2 / (2 * rho * g)`. Every physical input is a PARAMETER the caller supplies
/// (the potential temperature from the interior thermostat, the solidus from [`multicomponent_solidus`], the
/// productivity and adiabat gradient and densities from the mantle floor), so nothing is authored in the
/// kernel: the law is fixed Rust, the numbers are the world's. Pressures are gigapascals, slopes kelvin per
/// gigapascal, the productivity per gigapascal, the density kilograms per cubic metre, gravity metres per
/// second squared. `None` if the solidus does not rise faster than the adiabat (no melting column forms) or an
/// input is non-physical; a mantle colder than the surface solidus melts nothing (a zero column). The grade is
/// the linear-productivity first pass, valid while the peak melt fraction stays well below one.
///
/// Validated against McKenzie-Bickle: a normal potential temperature (about 1588 K) on the measured peridotite
/// solidus (1373 K, 130 K/GPa) makes about 6.5 km of crust at a peak melt fraction near 0.23, and a hotter
/// mantle thickens it steeply (the Archean komatiite regime, derived rather than tagged). Fed the rung-0
/// ideal solidus instead (about 1520 K, 150 K high), the same temperature makes far less crust, the honest
/// signature of the ideal-solution offset the rung-1 calibration closes.
pub fn adiabatic_melt_column(
    potential_temperature_k: Fixed,
    solidus_surface_k: Fixed,
    solidus_slope_k_per_gpa: Fixed,
    adiabat_slope_k_per_gpa: Fixed,
    productivity_per_gpa: Fixed,
    source_density_kg_per_m3: Fixed,
    gravity_m_per_s2: Fixed,
) -> Option<MeltColumn> {
    if productivity_per_gpa < Fixed::ZERO
        || source_density_kg_per_m3 <= Fixed::ZERO
        || gravity_m_per_s2 <= Fixed::ZERO
    {
        return None;
    }
    // The solidus must rise faster than the adiabat, else the rising mantle never crosses it.
    let slope_diff = solidus_slope_k_per_gpa.checked_sub(adiabat_slope_k_per_gpa)?;
    if slope_diff <= Fixed::ZERO {
        return None;
    }
    // The surface superheat above the solidus. A mantle colder than the surface solidus melts nothing.
    let superheat = potential_temperature_k.checked_sub(solidus_surface_k)?;
    if superheat <= Fixed::ZERO {
        return Some(MeltColumn {
            crust_thickness_km: Fixed::ZERO,
            max_melt_fraction: Fixed::ZERO,
            onset_pressure_gpa: Fixed::ZERO,
        });
    }
    // The onset pressure: T_p + m_ad*P0 = T_sol0 + m_sol*P0, so P0 = superheat / (m_sol - m_ad).
    let p0 = superheat.checked_div(slope_diff)?;
    // The unclamped surface melt fraction F(0) = dF/dP * P0. It sets both the (clamped) reported melt fraction
    // and which crust-integral branch applies.
    let f_surface = productivity_per_gpa.checked_mul(p0)?;
    let f_max = clamp_unit(f_surface);
    // The crust is the integrated melt column, crust (km) = I * 1e6 / (rho g), where I = integral over [0, P0] of
    // min(F(P), 1) dP with F(P) = dF/dP * (P0 - P) rising from 0 at the onset to F(0) at the surface, and the 1e6
    // folds the GPa-to-Pa and metre-to-km conversions.
    let crust = if f_surface <= Fixed::ONE {
        // Unsaturated: the full parabola I = dF/dP * P0^2 / 2, so crust = dF/dP * P0^2 * 1e6 / (2 rho g). The small
        // P0 is squared first to stay below the fixed-point ceiling. (This branch is unchanged.)
        let p0_sq = p0.checked_mul(p0)?;
        let numer = productivity_per_gpa
            .checked_mul(p0_sq)?
            .checked_mul(Fixed::from_int(1_000_000))?;
        let denom = Fixed::from_int(2)
            .checked_mul(source_density_kg_per_m3)?
            .checked_mul(gravity_m_per_s2)?;
        numer.checked_div(denom)?
    } else {
        // Saturated (the hot komatiite regime): the melt fraction reaches 1 at the saturation pressure and stays
        // capped above it, so the parabola is truncated: I = P0 - 1/(2 * dF/dP). Meets the unsaturated branch
        // continuously at F(0) = 1 (both give P0/2). Using the full parabola here would grow the crust as if the
        // melt fraction rose past 100%, which is the conservation defect this branch closes. dF/dP > 0 here
        // (F(0) = dF/dP * P0 > 1), so the reciprocal is safe.
        let sat_correction =
            Fixed::ONE.checked_div(Fixed::from_int(2).checked_mul(productivity_per_gpa)?)?;
        let integral_gpa = p0.checked_sub(sat_correction)?;
        let numer = integral_gpa.checked_mul(Fixed::from_int(1_000_000))?;
        let denom = source_density_kg_per_m3.checked_mul(gravity_m_per_s2)?;
        numer.checked_div(denom)?
    };
    Some(MeltColumn {
        crust_thickness_km: crust,
        max_melt_fraction: f_max,
        onset_pressure_gpa: p0,
    })
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
            fusion_volume_cm3_per_mol: Fixed::from_ratio(52, 10), // ~5.2, less-dense melt
        }
    }
    fn anorthite() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(1830),
            fusion_enthalpy_j_per_mol: Fixed::from_int(133_000),
            fusion_volume_cm3_per_mol: Fixed::from_int(6),
        }
    }
    // Forsterite and fayalite, the olivine endmembers the second gate's complete solid solution runs on.
    fn forsterite() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(2163),
            fusion_enthalpy_j_per_mol: Fixed::from_int(114_000),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(39, 10), // ~3.9
        }
    }
    fn fayalite() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(1490),
            fusion_enthalpy_j_per_mol: Fixed::from_int(89_000),
            fusion_volume_cm3_per_mol: Fixed::from_int(4), // estimate, not pressure-gated here
        }
    }
    // Enstatite (MgSiO3), the pyroxene of the peridotite assemblage; incongruent, treated as a pseudo-endmember
    // for the ideal multi-saturation solidus estimate.
    fn enstatite() -> Endmember {
        Endmember {
            melting_point_k: Fixed::from_int(1830),
            fusion_enthalpy_j_per_mol: Fixed::from_int(73_000),
            fusion_volume_cm3_per_mol: Fixed::from_int(5), // estimate, not pressure-gated here
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
    fn the_forsterite_fayalite_loop_recovers_the_pure_melting_points() {
        // The other binary topology: a complete solid solution (olivine), a lens with no eutectic. At each
        // pure melting point the liquidus and solidus meet at the pure composition, and between them the solid
        // is always enriched in the higher-melting forsterite relative to the coexisting liquid.
        let (fo, fa) = (forsterite(), fayalite());
        let (xl_lo, xs_lo) = binary_solid_solution_loop(fo, fa, fa.melting_point_k).unwrap();
        assert!(
            close(xl_lo, 1.0, 0.01) && close(xs_lo, 1.0, 0.01),
            "pure fayalite at its melting point"
        );
        let (xl_hi, xs_hi) = binary_solid_solution_loop(fo, fa, fo.melting_point_k).unwrap();
        assert!(
            close(xl_hi, 0.0, 0.01) && close(xs_hi, 0.0, 0.01),
            "pure forsterite at its melting point"
        );
        // Interior: the liquid is fayalite-enriched, the solid forsterite-enriched (x_Fa^liq > x_Fa^sol).
        let (xl, xs) = binary_solid_solution_loop(fo, fa, Fixed::from_int(1800)).unwrap();
        assert!(
            xl.to_f64_lossy() > xs.to_f64_lossy(),
            "the liquid is enriched in the lower-melting fayalite, got liq {} sol {}",
            xl.to_f64_lossy(),
            xs.to_f64_lossy()
        );
        // Outside the melting-point interval there is no two-phase lens.
        assert_eq!(
            binary_solid_solution_loop(fo, fa, Fixed::from_int(1400)),
            None
        );
        assert_eq!(
            binary_solid_solution_loop(fo, fa, Fixed::from_int(2200)),
            None
        );
    }

    #[test]
    fn the_forsterite_fayalite_loop_lands_on_the_ideal_lens() {
        // The ideal lens at two checkpoints, against the closed-form reference (x_Fa^liq, x_Fa^sol):
        // (0.887, 0.365) at 1700 K and (0.641, 0.136) at 1900 K.
        let (fo, fa) = (forsterite(), fayalite());
        let (xl17, xs17) = binary_solid_solution_loop(fo, fa, Fixed::from_int(1700)).unwrap();
        assert!(
            close(xl17, 0.887, 0.015),
            "liquidus x_Fa at 1700 K ~ 0.887, got {}",
            xl17.to_f64_lossy()
        );
        assert!(
            close(xs17, 0.365, 0.015),
            "solidus x_Fa at 1700 K ~ 0.365, got {}",
            xs17.to_f64_lossy()
        );
        let (xl19, xs19) = binary_solid_solution_loop(fo, fa, Fixed::from_int(1900)).unwrap();
        assert!(
            close(xl19, 0.641, 0.015),
            "liquidus x_Fa at 1900 K ~ 0.641, got {}",
            xl19.to_f64_lossy()
        );
        assert!(
            close(xs19, 0.136, 0.015),
            "solidus x_Fa at 1900 K ~ 0.136, got {}",
            xs19.to_f64_lossy()
        );
    }

    #[test]
    fn the_solution_melt_fraction_rises_across_the_lens() {
        // An Fo50Fa50 bulk melts across the lens: all solid below its solidus, partial through it, all liquid
        // above its liquidus, F an output of the loop geometry. The reference lever values are ~0.26 at 1700 K
        // and ~0.72 at 1900 K.
        let (fo, fa) = (forsterite(), fayalite());
        let x = Fixed::from_ratio(50, 100);
        let f14 = solution_melt_fraction(fo, fa, x, Fixed::from_int(1400)).unwrap();
        let f16 = solution_melt_fraction(fo, fa, x, Fixed::from_int(1600)).unwrap();
        let f17 = solution_melt_fraction(fo, fa, x, Fixed::from_int(1700)).unwrap();
        let f19 = solution_melt_fraction(fo, fa, x, Fixed::from_int(1900)).unwrap();
        let f22 = solution_melt_fraction(fo, fa, x, Fixed::from_int(2200)).unwrap();
        assert_eq!(
            f14,
            Fixed::ZERO,
            "below the lower melting point the system is one solid solution"
        );
        assert_eq!(
            f16,
            Fixed::ZERO,
            "an Fo-rich bulk is still below its solidus at 1600 K"
        );
        assert!(
            close(f17, 0.26, 0.03),
            "partial melt ~0.26 at 1700 K, got {}",
            f17.to_f64_lossy()
        );
        assert!(
            close(f19, 0.72, 0.03),
            "partial melt ~0.72 at 1900 K, got {}",
            f19.to_f64_lossy()
        );
        assert_eq!(
            f22,
            Fixed::ONE,
            "above the higher melting point it is all liquid"
        );
        assert!(
            f22 >= f19 && f19 >= f17 && f17 >= f16,
            "F rises monotonically with temperature"
        );
    }

    #[test]
    fn the_same_eutectic_machinery_admits_the_cryogenic_alien() {
        // Prime-directive test: the SAME binary_eutectic, unchanged, on cryogenic volatile-ice rows instead of
        // silicate rows. Water ice (273.15 K, 6010 J/mol) plus ammonia (195.4 K, 5660 J/mol) is the ammonia-
        // water system whose eutectic is what makes cryovolcanism possible. The ideal eutectic lands near
        // 180 K, within ~4 K of the experimental ~176 K, so the alien is a data row, not a rewrite. The
        // composition is far off (x_NH3 ~ 0.74 against the measured ~0.33): ammonia-water is strongly non-
        // ideal (hydrogen-bonded, hydrate-forming), the honest rung-1 Margules target, not a rung-0 claim.
        let water = Endmember {
            melting_point_k: Fixed::from_ratio(27315, 100),
            fusion_enthalpy_j_per_mol: Fixed::from_int(6010),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(-16, 10), // ice is DENSER-melt anomaly: dV < 0
        };
        let ammonia = Endmember {
            melting_point_k: Fixed::from_ratio(1954, 10),
            fusion_enthalpy_j_per_mol: Fixed::from_int(5660),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(25, 10), // ~2.5, estimate, not pressure-gated here
        };
        let (t_e, x_nh3) = binary_eutectic(water, ammonia).expect("the cryogenic curves cross");
        assert!(
            close(t_e, 180.2, 3.0),
            "the ideal ammonia-water eutectic lands near 180 K (experimental ~176 K), got {}",
            t_e.to_f64_lossy()
        );
        // It is a cryogenic eutectic, far below the water melting point, and a valid liquid composition.
        assert!(t_e.to_f64_lossy() < 195.0 && t_e.to_f64_lossy() > 150.0);
        assert!(x_nh3.to_f64_lossy() > 0.0 && x_nh3.to_f64_lossy() < 1.0);
    }

    #[test]
    fn the_clapeyron_slope_derives_from_the_volume_change() {
        // The pressure term: dT_m/dP = dV_fus/dS_fus is a thermodynamic identity, so the derived diopside
        // slope is a self-check against the measured melting curve, not a fit. 10000 bar is 1 GPa; the
        // diopside slope lands ~63 K/GPa, inside the measured ~60-75 K/GPa band.
        let di = diopside();
        let t0 = melting_point_at_pressure(di, Fixed::ZERO).unwrap();
        let t1 = melting_point_at_pressure(di, Fixed::from_int(10_000)).unwrap();
        assert_eq!(
            t0, di.melting_point_k,
            "at surface pressure the melting point is unshifted"
        );
        let slope = t1.to_f64_lossy() - t0.to_f64_lossy();
        assert!(
            close(Fixed::from_int(0), slope - 62.7, 3.0),
            "diopside dT/dP ~ 63 K/GPa, got {slope}"
        );
    }

    #[test]
    fn the_eutectic_rises_with_pressure() {
        // The eutectic reuses the surface solver on the Clapeyron-shifted endmembers, so it rises with depth:
        // the Di-An eutectic climbs from ~1608 K at the surface to ~1668 K at 1 GPa. At zero pressure it is
        // byte-identical to the surface eutectic (nothing is duplicated).
        let (di, an) = (diopside(), anorthite());
        assert_eq!(
            binary_eutectic_at_pressure(di, an, Fixed::ZERO),
            binary_eutectic(di, an),
            "at surface pressure the pressure-aware eutectic is the surface eutectic"
        );
        let (t_e1, _) = binary_eutectic_at_pressure(di, an, Fixed::from_int(10_000)).unwrap();
        assert!(
            close(t_e1, 1667.6, 6.0),
            "the Di-An eutectic rises to ~1668 K at 1 GPa, got {}",
            t_e1.to_f64_lossy()
        );
        let (t_e0, _) = binary_eutectic(di, an).unwrap();
        assert!(t_e1 > t_e0, "the eutectic rises with pressure");
    }

    #[test]
    fn pressure_suppresses_melting_at_a_fixed_temperature() {
        // The decompression-melting productivity term: at a fixed temperature above the surface solidus, a
        // positive-dV system melts LESS as pressure rises (the solidus has climbed past it). An Fo-poor bulk
        // partly molten at 1625 K and the surface is frozen once the eutectic has risen above it at 0.5 GPa.
        let (di, an) = (diopside(), anorthite());
        let x = Fixed::from_ratio(15, 100);
        let f_surface =
            batch_melt_fraction_at_pressure(di, an, x, Fixed::from_int(1625), Fixed::ZERO).unwrap();
        let f_deep = batch_melt_fraction_at_pressure(
            di,
            an,
            x,
            Fixed::from_int(1625),
            Fixed::from_int(5000),
        )
        .unwrap();
        assert!(
            f_surface.to_f64_lossy() > 0.0,
            "partly molten at 1625 K at the surface"
        );
        assert!(
            f_surface > f_deep,
            "pressure suppresses melting: F {} at surface, {} at 0.5 GPa",
            f_surface.to_f64_lossy(),
            f_deep.to_f64_lossy()
        );
    }

    #[test]
    fn the_ice_anomaly_lowers_the_melting_point_with_pressure() {
        // Admit-the-alien, derived: water ice is DENSER as a melt (dV_fus < 0), so its Clapeyron slope is
        // NEGATIVE. Pressure LOWERS the water melting point (the anomaly that makes cryomagma negatively
        // buoyant), where the silicate diopside's rises. At 1000 bar (0.1 GPa) the water melting point drops
        // ~7 K, matching the measured ice-Ih slope of about -7.4 K/kbar; the sign is the derived point.
        let water = Endmember {
            melting_point_k: Fixed::from_ratio(27315, 100),
            fusion_enthalpy_j_per_mol: Fixed::from_int(6010),
            fusion_volume_cm3_per_mol: Fixed::from_ratio(-16, 10),
        };
        let t0 = melting_point_at_pressure(water, Fixed::ZERO).unwrap();
        let t1 = melting_point_at_pressure(water, Fixed::from_int(1000)).unwrap();
        assert!(
            t1 < t0,
            "water's melting point FALLS with pressure (the ice anomaly)"
        );
        let drop = t0.to_f64_lossy() - t1.to_f64_lossy();
        assert!(
            close(Fixed::from_int(0), drop - 7.3, 1.0),
            "the ice-Ih drop is ~7 K at 0.1 GPa, got {drop}"
        );
        // And the sign is opposite the silicate: diopside's melting point RISES at the same pressure.
        let di_up = melting_point_at_pressure(diopside(), Fixed::from_int(1000)).unwrap();
        assert!(
            di_up > diopside().melting_point_k,
            "the silicate rises where the ice falls"
        );
    }

    #[test]
    fn the_peridotite_solidus_derives_from_the_multi_saturation_point() {
        // The N-component generalization: the fertile lherzolite solidus is where a single liquid saturates in
        // all four minerals at once (sum x_i = 1), derived from their signatures, not authored. It lands near
        // 1520 K against the measured ~1373 K dry peridotite solidus (about 150 K high, the ideal-solution
        // band, the rung-1 target), and it sits below every pure melting point.
        let assemblage = [forsterite(), enstatite(), diopside(), anorthite()];
        let t_sol = multicomponent_solidus(&assemblage, Fixed::ZERO).unwrap();
        assert!(
            close(t_sol, 1520.0, 6.0),
            "the ideal peridotite solidus lands near 1520 K, got {}",
            t_sol.to_f64_lossy()
        );
        assert!(
            t_sol.to_f64_lossy() < 1665.0,
            "the solidus is below the lowest mineral melting point"
        );
        // Each added component deepens the eutectic depression: the four-phase solidus is below the Di-An
        // binary eutectic.
        let (t_binary, _) = binary_eutectic(diopside(), anorthite()).unwrap();
        assert!(
            t_sol < t_binary,
            "more components lower the solidus, got {} vs binary {}",
            t_sol.to_f64_lossy(),
            t_binary.to_f64_lossy()
        );
        // The solidus rises with depth through the Clapeyron shift.
        let t_deep = multicomponent_solidus(&assemblage, Fixed::from_int(10_000)).unwrap();
        assert!(t_deep > t_sol, "the solidus rises with pressure");
        // Guards: an empty assemblage has no solidus; a single phase melts at its own point.
        assert_eq!(multicomponent_solidus(&[], Fixed::ZERO), None);
        let single = multicomponent_solidus(&[diopside()], Fixed::ZERO).unwrap();
        assert!(
            close(single, 1665.0, 1.0),
            "a single phase's solidus is its melting point, got {}",
            single.to_f64_lossy()
        );
    }

    #[test]
    fn the_melting_column_reproduces_mckenzie_bickle() {
        // The crust closure with the MEASURED peridotite solidus (1373 K, 130 K/GPa) and a normal potential
        // temperature: McKenzie-Bickle's ~6.5 km of oceanic crust at a peak melt fraction near 0.23, melting
        // beginning near 1.9 GPa. The inputs are the caller's (measured, the self-check); the integrator is
        // the mechanism.
        let col = adiabatic_melt_column(
            Fixed::from_int(1588),      // potential temperature (about 1315 C)
            Fixed::from_int(1373),      // measured peridotite solidus at the surface
            Fixed::from_int(130),       // measured solidus slope, K/GPa
            Fixed::from_ratio(155, 10), // adiabat slope 15.5 K/GPa
            Fixed::from_ratio(12, 100), // productivity 0.12 /GPa
            Fixed::from_int(3300),      // mantle source density
            Fixed::from_ratio(98, 10),  // gravity
        )
        .unwrap();
        assert!(
            close(col.crust_thickness_km, 6.5, 0.5),
            "the column makes ~6.5 km of crust, got {}",
            col.crust_thickness_km.to_f64_lossy()
        );
        assert!(
            close(col.max_melt_fraction, 0.225, 0.03),
            "peak melt fraction ~0.23, got {}",
            col.max_melt_fraction.to_f64_lossy()
        );
        assert!(
            close(col.onset_pressure_gpa, 1.88, 0.1),
            "melting begins near 1.9 GPa, got {}",
            col.onset_pressure_gpa.to_f64_lossy()
        );
    }

    #[test]
    fn a_saturated_melt_column_caps_the_crust_at_full_melt() {
        // The conservation fix for the hot komatiite regime: once the surface melt fraction would exceed 1, the
        // melt fraction is physically capped at 1 above the saturation pressure, so the crust integral TRUNCATES
        // the parabola (I = P0 - 1/(2 dF/dP)) rather than growing as if the melt fraction rose past 100%. A hot
        // mantle (Tp = 2100 K) with a higher hot-regime productivity (0.3/GPa) saturates.
        let col = adiabatic_melt_column(
            Fixed::from_int(2100),
            Fixed::from_int(1373),
            Fixed::from_int(130),
            Fixed::from_ratio(155, 10),
            Fixed::from_ratio(3, 10), // productivity 0.3 /GPa (hot regime)
            Fixed::from_int(3300),
            Fixed::from_ratio(98, 10),
        )
        .unwrap();
        // The melt fraction saturates to 1.
        assert!(
            close(col.max_melt_fraction, 1.0, 1e-6),
            "the melt fraction saturates to 1, got {}",
            col.max_melt_fraction.to_f64_lossy()
        );
        // The crust is the TRUNCATED integral P0 - 1/(2 dF/dP), and strictly below the un-capped parabola the
        // conservation defect would have produced.
        let p0 = col.onset_pressure_gpa.to_f64_lossy();
        let fold = 1e6 / (3300.0 * 9.8);
        let saturated_km = (p0 - 1.0 / (2.0 * 0.3)) * fold;
        let parabola_km = (0.3 * p0 * p0 / 2.0) * fold;
        assert!(
            close(col.crust_thickness_km, saturated_km, 2.0),
            "the saturated crust is the truncated integral ~{saturated_km:.0} km, got {}",
            col.crust_thickness_km.to_f64_lossy()
        );
        assert!(
            col.crust_thickness_km.to_f64_lossy() < parabola_km * 0.95,
            "the cap reduces the crust below the un-truncated parabola ({} vs ~{parabola_km:.0} km)",
            col.crust_thickness_km.to_f64_lossy()
        );
    }

    #[test]
    fn the_melt_column_crust_is_continuous_across_saturation() {
        // The unsaturated and saturated branches meet continuously at the saturation threshold F(0) = 1 (both
        // give P0/2), so the crust has NO jump as the mantle heats across it (a discontinuity would be a
        // fixed-point/physical artifact).
        let col = |tp: i32| {
            adiabatic_melt_column(
                Fixed::from_int(tp),
                Fixed::from_int(1373),
                Fixed::from_int(130),
                Fixed::from_ratio(155, 10),
                Fixed::from_ratio(3, 10),
                Fixed::from_int(3300),
                Fixed::from_ratio(98, 10),
            )
            .unwrap()
        };
        // Saturation is near Tp ~ 1755 K (F(0) = 0.3 * P0 = 1); straddle it by 1 K each side.
        let below = col(1754);
        let above = col(1756);
        let jump = (above.crust_thickness_km.to_f64_lossy()
            - below.crust_thickness_km.to_f64_lossy())
        .abs();
        assert!(
            jump < 1.0,
            "the crust is continuous across saturation (jump {jump} km over 2 K)"
        );
        assert!(
            below.max_melt_fraction.to_f64_lossy() < 1.0,
            "the below-threshold column is unsaturated"
        );
        assert!(
            close(above.max_melt_fraction, 1.0, 1e-6),
            "the above-threshold column is saturated"
        );
    }

    #[test]
    fn a_hotter_mantle_makes_thicker_crust() {
        // The Archean komatiite regime, derived rather than tagged: a hotter mantle melts more and pools a
        // thicker crust. A mantle colder than the surface solidus melts nothing.
        let col = |tp: i32| {
            adiabatic_melt_column(
                Fixed::from_int(tp),
                Fixed::from_int(1373),
                Fixed::from_int(130),
                Fixed::from_ratio(155, 10),
                Fixed::from_ratio(12, 100),
                Fixed::from_int(3300),
                Fixed::from_ratio(98, 10),
            )
            .unwrap()
        };
        let normal = col(1588);
        let hot = col(1650);
        assert!(
            hot.crust_thickness_km > normal.crust_thickness_km,
            "a hotter mantle makes thicker crust, got {} vs {}",
            hot.crust_thickness_km.to_f64_lossy(),
            normal.crust_thickness_km.to_f64_lossy()
        );
        let cold = col(1300);
        assert_eq!(
            cold.crust_thickness_km,
            Fixed::ZERO,
            "a sub-solidus mantle melts nothing"
        );
    }

    #[test]
    fn the_derived_solidus_feeds_the_column_end_to_end() {
        // The full derived chain: mineral signatures -> multi-saturation solidus -> crust, no authored
        // solidus. The derived solidus (about 1520 K) is 150 K high but its ideal slope (~60 K/GPa) is
        // shallower than the measured 130, and the two errors partly cancel, so the fully-derived chain still
        // makes a sane ~4 km of oceanic crust at a normal potential temperature (within about 1.5x of McKenzie-
        // Bickle's 6.5 km), rising with temperature. Closing the residual to the measured value is the rung-1
        // calibration, plainly labelled.
        let assemblage = [forsterite(), enstatite(), diopside(), anorthite()];
        let t_sol0 = multicomponent_solidus(&assemblage, Fixed::ZERO).unwrap();
        let t_sol1 = multicomponent_solidus(&assemblage, Fixed::from_int(10_000)).unwrap();
        let slope = t_sol1.checked_sub(t_sol0).unwrap(); // per GPa, since 10000 bar = 1 GPa
        let col = |tp: i32| {
            adiabatic_melt_column(
                Fixed::from_int(tp),
                t_sol0,
                slope,
                Fixed::from_ratio(155, 10),
                Fixed::from_ratio(12, 100),
                Fixed::from_int(3300),
                Fixed::from_ratio(98, 10),
            )
            .unwrap()
        };
        let normal = col(1588);
        assert!(
            normal.crust_thickness_km.to_f64_lossy() > 2.0
                && normal.crust_thickness_km.to_f64_lossy() < 8.0,
            "the fully-derived chain makes sane oceanic crust (~4 km), got {}",
            normal.crust_thickness_km.to_f64_lossy()
        );
        let hot = col(1700);
        assert!(
            hot.crust_thickness_km > normal.crust_thickness_km,
            "the derived chain thickens with temperature"
        );
    }

    #[test]
    fn the_peridotite_first_melt_is_a_derived_basalt() {
        // The crust derives from the mantle: the first melt of the four-mineral lherzolite is enriched in the
        // fusible clinopyroxene and plagioclase (a basalt) and depleted in the refractory olivine, so the melt
        // is the oceanic crust and the residue the harzburgite mantle, both from the same signatures.
        let assemblage = [forsterite(), enstatite(), diopside(), anorthite()];
        let (t_sol, comp) = eutectic_liquid_composition(&assemblage, Fixed::ZERO).unwrap();
        assert!(
            close(t_sol, 1520.0, 6.0),
            "taken at the solidus, got {}",
            t_sol.to_f64_lossy()
        );
        let (fo, en, di, an) = (
            comp[0].to_f64_lossy(),
            comp[1].to_f64_lossy(),
            comp[2].to_f64_lossy(),
            comp[3].to_f64_lossy(),
        );
        let sum = fo + en + di + an;
        assert!(
            (sum - 1.0).abs() < 1e-3,
            "the liquid composition sums to one, got {sum}"
        );
        assert!(
            fo < 0.15,
            "olivine is depleted in the melt (stays in the residue), got {fo}"
        );
        assert!(di > 0.30, "clinopyroxene is enriched in the melt, got {di}");
        assert!(
            di + an > fo,
            "the melt is basaltic (cpx + plag rich, olivine poor)"
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
