// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! THE MIE-GRUENEISEN-DEBYE SOLVER: rung 3 of the thermoelastic ladder.
//!
//! # What it computes
//!
//! A phase's molar volume, isothermal bulk modulus and volumetric expansivity AT a requested pressure and
//! temperature, from six per-phase anchors, rather than by reading an ambient row somewhere it does not
//! apply. The ladder's whole reason for existing is that reading a 300 K row at 1600 K produced a number
//! that matched measurement by cancellation; this is the rung that answers instead of refusing.
//!
//! The equation of state is the standard reduced form: a 300 K cold isotherm from third-order
//! Birch-Murnaghan, plus a thermal pressure from a Debye model whose characteristic temperature and
//! Grueneisen parameter both vary with volume.
//!
//! ```text
//!   P(V, T)   = P_BM(V) + P_th(V, T) - P_th(V, T_ref)
//!   P_BM(V)   = (3/2) K_0 [r^(7/3) - r^(5/3)] {1 + (3/4)(K_0' - 4)[r^(2/3) - 1]},  r = V_0/V
//!   P_th(V,T) = gamma(V) E_th(V, T) / V
//!   gamma(V)  = gamma_0 (V/V_0)^q
//!   theta(V)  = theta_0 exp[(gamma_0/q)(1 - (V/V_0)^q)]
//!   E_th(V,T) = 9 n R T (T/theta)^3 Integral_0^(theta/T) x^3/(e^x - 1) dx
//! ```
//!
//! # Which Debye temperature, and why the type says so
//!
//! `theta_0` here is the EFFECTIVE Debye temperature, fit by its source to the vibrational entropy near
//! 1000 K. It is not the elastic Debye temperature that
//! [`crate::thermoelastic::derived_elastic_debye_temperature`] computes from the moduli, and the two are
//! not interchangeable: across the seven banked phases their ratio runs 0.83 to 1.22, and the entry point
//! below accepts only [`EffectiveDebyeTemperature`], which has no constructor reachable from the elastic
//! side. That is deliberate. The substitution was made in this repository on the strength of a single
//! forsterite spot-check that happened to land on the crossover, and a comment saying "do not do this"
//! would not have stopped it.
//!
//! # Determinism
//!
//! Everything here is fixed-point and no loop exits on a tolerance. The volume inversion is bisection run
//! to a constant step count; the Debye integral's interval count is a pure function of its upper limit,
//! chosen to bound the STEP rather than the count. Both are deterministic, and the distinction matters:
//! what breaks reproducibility is a trip count that depends on how quickly a particular input converged,
//! not one that depends on the input itself. A constant count here was in fact WORSE, because it let the
//! step grow with the range and cost 0.2 percent accuracy at the top of it.
//!
//! # Cost
//!
//! One response costs roughly `BISECTION_STEPS * (SIMPSON_INTERVALS + 1)` evaluations of `Fixed::exp`,
//! about two thousand at ordinary interior states, rising where `theta/T` is large enough to widen the
//! quadrature. That is a worldgen-time budget, not a per-tick one, and callers on a tick path should
//! cache per (phase, state bucket) rather than call this in a loop.

use civsim_core::Fixed;
use civsim_physics::thermoelastic_anchors::EffectiveDebyeTemperature;

/// The FLOOR on Simpson intervals for the Debye integral. The actual count rises with the upper limit so
/// the step stays bounded; this is the minimum, used wherever `theta/T` is small.
const SIMPSON_INTERVALS: u32 = 32;

/// The upper limit beyond which the Debye integral is its infinite value to within this representation.
/// Past `x = 30` the integrand is under `2.5e-10`, below `Fixed`'s own resolution.
const MAX_INTEGRATION_LIMIT: i32 = 30;

/// Bisection steps for the volume inversion. `2^40` divisions of the bracket land far below `Fixed`'s own
/// resolution, so the loop is limited by the representation rather than by the count.
const BISECTION_STEPS: u32 = 40;

/// The six anchors, assembled and validated. Construction is the ONLY route into the solver, so a caller
/// cannot assemble a partial or mixed-provenance set and evaluate it anyway.
#[derive(Clone, Copy, Debug)]
pub struct MgdAnchors {
    /// Reference molar volume at 300 K and 1 bar (cm^3/mol).
    pub v0_cm3: Fixed,
    /// Reference isothermal bulk modulus (GPa).
    pub k0_gpa: Fixed,
    /// Pressure derivative of the bulk modulus, dimensionless.
    pub k0_prime: Fixed,
    /// The EFFECTIVE Debye temperature. See the module documentation.
    pub theta_0: EffectiveDebyeTemperature,
    /// Reference Grueneisen parameter, dimensionless.
    pub gamma_0: Fixed,
    /// Volume exponent in `gamma = gamma_0 (V/V_0)^q`, dimensionless.
    pub q: Fixed,
    /// Atoms per formula unit, matching the molar volume's basis.
    pub atoms_per_formula_unit: u32,
}

/// The reference temperature the anchors are stated at (K). The cold isotherm is the 300 K one, so the
/// thermal pressure is taken as a DIFFERENCE from this temperature rather than as an absolute.
const REFERENCE_TEMPERATURE_K: i32 = 300;

/// `x^3 / (e^x - 1)`, the Debye integrand, with the removable singularity at the origin handled by series
/// rather than by a division that loses its significant digits.
///
/// Below the crossover the direct form subtracts two nearly equal quantities: at `x = 1e-4`, `e^x - 1` is
/// about `1e-4` against a `Fixed` resolution of `2.33e-10`, so the quotient keeps only a few digits. The
/// series `x^2 (1 - x/2 + x^2/12)` is exact to the same order and loses nothing.
fn debye_integrand(x: Fixed) -> Option<Fixed> {
    let crossover = Fixed::ONE.checked_div(Fixed::from_int(64))?;
    if x <= crossover {
        // x^2 (1 - x/2 + x^2/12)
        let x2 = x.checked_mul(x)?;
        let half_x = x.checked_div(Fixed::from_int(2))?;
        let x2_over_12 = x2.checked_div(Fixed::from_int(12))?;
        let bracket = Fixed::ONE.checked_sub(half_x)?.checked_add(x2_over_12)?;
        return x2.checked_mul(bracket);
    }
    let denom = x.exp().checked_sub(Fixed::ONE)?;
    if denom <= Fixed::ZERO {
        return None;
    }
    x.checked_mul(x)?.checked_mul(x)?.checked_div(denom)
}

/// `Integral_0^y x^3/(e^x - 1) dx` by Simpson's rule, over enough intervals to bound the step.
///
/// Returns `None` on a negative upper limit or an unrepresentable intermediate. At `y = 0` the integral is
/// zero, which is the correct value rather than a special case.
pub fn debye_integral(y: Fixed) -> Option<Fixed> {
    if y < Fixed::ZERO {
        return None;
    }
    if y == Fixed::ZERO {
        return Some(Fixed::ZERO);
    }
    // THE UPPER LIMIT IS CLAMPED AT 30, which is exact to this representation rather than an
    // approximation. Beyond x = 30 the integrand is below `30^3 e^-30`, about `2.5e-10`, and the whole
    // remaining tail integrates to less than `Fixed`'s own resolution. A cold phase with `theta/T > 30`
    // therefore gets the infinite-limit integral, which is the correct `T^4` behaviour.
    let y_eff = if y > Fixed::from_int(MAX_INTEGRATION_LIMIT) {
        Fixed::from_int(MAX_INTEGRATION_LIMIT)
    } else {
        y
    };
    // THE INTERVAL COUNT SCALES WITH THE RANGE so the STEP stays bounded. A constant count looks more
    // deterministic and is not: it makes `h` grow with `y`, and at `y = 30` a 32-interval rule gave
    // `h = 0.94` across an integrand that peaks near `x = 2.8`, for a 0.2 percent error. Determinism
    // does not require a constant trip count, only that the count be a pure function of the inputs
    // rather than of how quickly something converged. This one is `ceil(8 y)` rounded up to even.
    // Integer arithmetic on the raw bits rather than a float round-trip: `ceil(8 y)` is
    // `(8 y + ONE - 1) >> FRAC_BITS`, and no float enters the deterministic path at all.
    let eight_y = y_eff.checked_mul(Fixed::from_int(8))?.to_bits();
    let steps = ((eight_y + (1i64 << Fixed::FRAC_BITS) - 1) >> Fixed::FRAC_BITS) as u32;
    let n = core::cmp::max(SIMPSON_INTERVALS, steps + (steps % 2));
    let y = y_eff;
    let h = y.checked_div(Fixed::from_int(n as i32))?;
    // Simpson: h/3 [f_0 + 4(odd) + 2(even interior) + f_n]
    let mut acc = debye_integrand(Fixed::ZERO)?.checked_add(debye_integrand(y)?)?;
    for i in 1..n {
        let x = h.checked_mul(Fixed::from_int(i as i32))?;
        let f = debye_integrand(x)?;
        let w = if i % 2 == 1 { 4 } else { 2 };
        acc = acc.checked_add(f.checked_mul(Fixed::from_int(w))?)?;
    }
    acc.checked_mul(h)?.checked_div(Fixed::from_int(3))
}

/// Debye thermal energy (J/mol) for `n` atoms per formula unit at temperature `t_k` and characteristic
/// temperature `theta_k`.
///
/// `E_th = 9 n R T (T/theta)^3 Integral_0^(theta/T) x^3/(e^x - 1) dx`, which tends to the Dulong-Petit
/// `3 n R T` as `theta/T -> 0`. That limit is the test this function is checked against, because it is a
/// value the physics fixes independently of any fit.
// @derives: a phase's Debye thermal energy <- its atom count, the requested temperature and its characteristic Debye temperature
pub fn debye_thermal_energy_j_per_mol(t_k: Fixed, theta_k: Fixed, atoms: u32) -> Option<Fixed> {
    if t_k <= Fixed::ZERO || theta_k <= Fixed::ZERO || atoms == 0 {
        return None;
    }
    let r = civsim_physics::gas_thermochemistry::molar_gas_constant()?;
    let y = theta_k.checked_div(t_k)?;
    let integral = debye_integral(y)?;
    let t_over_theta = t_k.checked_div(theta_k)?;
    let cube = t_over_theta
        .checked_mul(t_over_theta)?
        .checked_mul(t_over_theta)?;
    // ORDER MATTERS HERE, and getting it wrong overflows on a physically ordinary input. `(T/theta)^3`
    // and the integral are RECIPROCALLY large and small: at `T = 20000` and `theta = 800` the cube is
    // 15625 while the integral is `2.1e-5`. Their product is `1/3`, but forming `9 n R T (T/theta)^3`
    // first reaches `1.6e11` and blows past the `2.1e9` ceiling on the way to an answer near `3.5e6`.
    // Multiplying the reciprocal pair together FIRST keeps every intermediate inside the window. This is
    // the representation discipline the log-space work already records: the operation order is part of
    // the correctness, not a style choice.
    let shape = cube.checked_mul(integral)?;
    Fixed::from_int(9)
        .checked_mul(Fixed::from_int(atoms as i32))?
        .checked_mul(r)?
        .checked_mul(t_k)?
        .checked_mul(shape)
}

/// The volume-dependent Grueneisen parameter, `gamma_0 (V/V_0)^q`.
// @derives: a phase's Grueneisen parameter at volume <- its reference gamma, volume ratio and volume exponent
fn gamma_at(anchors: &MgdAnchors, v_cm3: Fixed) -> Option<Fixed> {
    let ratio = v_cm3.checked_div(anchors.v0_cm3)?;
    if ratio <= Fixed::ZERO {
        return None;
    }
    anchors.gamma_0.checked_mul(ratio.powf(anchors.q))
}

/// The volume-dependent Debye temperature, `theta_0 exp[(gamma_0/q)(1 - (V/V_0)^q)]`.
///
/// This is the integrated form of `gamma = -dln(theta)/dln(V)` under a constant `q`, so the `theta` the
/// thermal energy uses and the `gamma` the thermal pressure uses describe the same solid rather than two
/// unrelated parameterisations.
// @derives: a phase's Debye temperature at volume <- its reference Debye temperature, gamma and volume exponent
fn theta_at(anchors: &MgdAnchors, v_cm3: Fixed) -> Option<Fixed> {
    if anchors.q == Fixed::ZERO {
        return None;
    }
    let ratio = v_cm3.checked_div(anchors.v0_cm3)?;
    if ratio <= Fixed::ZERO {
        return None;
    }
    let exponent = anchors
        .gamma_0
        .checked_div(anchors.q)?
        .checked_mul(Fixed::ONE.checked_sub(ratio.powf(anchors.q))?)?;
    anchors.theta_0.kelvin().checked_mul(exponent.exp())
}

/// Third-order Birch-Murnaghan cold pressure (GPa) at a molar volume.
// @derives: a phase's cold-isotherm pressure <- its reference volume, bulk modulus and pressure derivative
fn birch_murnaghan_gpa(anchors: &MgdAnchors, v_cm3: Fixed) -> Option<Fixed> {
    if v_cm3 <= Fixed::ZERO {
        return None;
    }
    let r = anchors.v0_cm3.checked_div(v_cm3)?;
    let third = Fixed::ONE.checked_div(Fixed::from_int(3))?;
    let r13 = r.powf(third);
    let r23 = r13.checked_mul(r13)?;
    let r53 = r23.checked_mul(r13)?.checked_mul(r13)?.checked_mul(r13)?;
    let r73 = r53.checked_mul(r23)?;
    let lead = Fixed::from_int(3)
        .checked_div(Fixed::from_int(2))?
        .checked_mul(anchors.k0_gpa)?
        .checked_mul(r73.checked_sub(r53)?)?;
    let correction = Fixed::from_int(3)
        .checked_div(Fixed::from_int(4))?
        .checked_mul(anchors.k0_prime.checked_sub(Fixed::from_int(4))?)?
        .checked_mul(r23.checked_sub(Fixed::ONE)?)?;
    lead.checked_mul(Fixed::ONE.checked_add(correction)?)
}

/// Thermal pressure (GPa) at a volume and temperature: `gamma(V) E_th(V,T) / V`.
///
/// The unit bridge is exact rather than a fudge: `E_th` is J/mol and `V` is cm^3/mol, so `E_th/V` is
/// J/cm^3, which is MPa, which is GPa after dividing by 1000.
// @derives: a phase's thermal pressure at a state <- its Grueneisen parameter, Debye thermal energy and molar volume
fn thermal_pressure_gpa(anchors: &MgdAnchors, v_cm3: Fixed, t_k: Fixed) -> Option<Fixed> {
    let gamma = gamma_at(anchors, v_cm3)?;
    let theta = theta_at(anchors, v_cm3)?;
    let e_th = debye_thermal_energy_j_per_mol(t_k, theta, anchors.atoms_per_formula_unit)?;
    gamma
        .checked_mul(e_th)?
        .checked_div(v_cm3)?
        .checked_div(Fixed::from_int(1000))
}

/// Total pressure (GPa) at a volume and temperature, cold isotherm plus the thermal pressure REFERENCED to
/// 300 K so the anchors' own reference state reproduces itself.
// @derives: a phase's pressure at a state <- its cold isotherm and the Debye thermal pressure above the reference temperature
pub fn pressure_gpa(anchors: &MgdAnchors, v_cm3: Fixed, t_k: Fixed) -> Option<Fixed> {
    let cold = birch_murnaghan_gpa(anchors, v_cm3)?;
    let hot = thermal_pressure_gpa(anchors, v_cm3, t_k)?;
    let reference = thermal_pressure_gpa(anchors, v_cm3, Fixed::from_int(REFERENCE_TEMPERATURE_K))?;
    cold.checked_add(hot)?.checked_sub(reference)
}

/// Why the solver could not answer. Each variant is a REFUSAL rather than a fallback value.
///
/// `PartialEq` without `Eq`: the bracket-report fields are `f64` diagnostics for a human reading a
/// refusal message, deliberately outside the fixed-point path because nothing computes with them.
#[derive(Clone, Debug, PartialEq)]
pub enum MgdFailure {
    /// An intermediate left the representable window, or an input was non-physical.
    Unrepresentable,
    /// The requested state lies outside the bracket the inversion searches, so no volume in the searched
    /// range produces the requested pressure. Reported rather than clamped: clamping to the bracket edge
    /// would return a confident wrong answer.
    OutsideBracket {
        /// The pressure asked for (GPa).
        requested_gpa: f64,
        /// The pressure at the compressed edge of the bracket (GPa).
        bracket_high_gpa: f64,
        /// The pressure at the expanded edge of the bracket (GPa).
        bracket_low_gpa: f64,
    },
}

/// One phase's state-resolved response from the MGD rung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MgdResponse {
    /// Molar volume at the requested state (cm^3/mol).
    pub molar_volume_cm3: Fixed,
    /// Isothermal bulk modulus at the requested state (GPa).
    pub bulk_modulus_gpa: Fixed,
    /// Volumetric thermal expansivity at the requested state (per K).
    pub alpha_per_k: Fixed,
}

/// Solve for the molar volume at a pressure and temperature, then read the response off the solved state.
///
/// # The inversion
///
/// Bisection on `V` over a bracket from strong compression to modest expansion, run to a FIXED step
/// count. `P(V)` decreases monotonically in `V` over the physical range, which is what makes bisection
/// sound here; the bracket is checked against the requested pressure first and the call REFUSES rather
/// than clamping when the request lies outside it.
///
/// # The response
///
/// `K_T = -V (dP/dV)_T` by central difference on the same `P(V,T)` the inversion used, so the modulus is
/// the derivative of the function that was actually solved rather than an independent analytic expression
/// that could drift from it. `alpha = gamma C_V / (K_T V)`, with `C_V` from the same Debye model, again by
/// central difference in temperature rather than by a closed form.
// @derives: a phase's molar volume, bulk modulus and expansivity at a state <- its six Mie-Grueneisen-Debye anchors
pub fn response_at(
    anchors: &MgdAnchors,
    pressure_gpa_target: Fixed,
    t_k: Fixed,
) -> Result<MgdResponse, MgdFailure> {
    let e = || MgdFailure::Unrepresentable;
    // THE LOWER EDGE is strong compression, where the Birch-Murnaghan term rises steeply and monotonically.
    let lo_edge = anchors
        .v0_cm3
        .checked_div(Fixed::from_int(3))
        .ok_or_else(e)?;

    // THE UPPER EDGE IS THE SPINODAL, FOUND, NOT AUTHORED.
    //
    // A fixed upper bracket of 1.5 V_0 is wrong, and an adversarial audit caught it. Expanding a phase far
    // enough makes the Birch-Murnaghan term flatten toward zero while the thermal pressure keeps growing
    // as gamma(V) = gamma_0 (V/V_0)^q, so P(V) TURNS AROUND and starts rising again. Past that turn K_T is
    // negative: the phase is not a mechanically stable solid there, and any root the bisection found on
    // that branch would be physically meaningless.
    //
    // Enstatite is the live case and it is not marginal. Its q is 7.8, the largest in the banked column, so
    // gamma at 1.5 V_0 is 22 times its reference value; the turn arrives at 1.156 V_0 and the old bracket
    // edge sat at +20.65 GPa. An AMBIENT-PRESSURE request therefore fell below the edge pressure and was
    // refused as out-of-bracket, on a phase that has a perfectly good ambient solution. Fayalite turns at
    // 1.418 V_0; forsterite and periclase do not turn inside the old bracket at all, which is exactly why a
    // check on one or two phases would have missed this.
    //
    // So the upper edge is SEARCHED: walk outward from the reference volume in fixed steps and stop at the
    // last point where P is still falling. That boundary is the spinodal, a property of the phase and the
    // temperature rather than a number chosen to make a bracket work, and it moves correctly when a phase's
    // q or gamma_0 changes. It is a fixed step count, so it costs the same for every input.
    const EXPANSION_STEPS: u32 = 64;
    let span = anchors
        .v0_cm3
        .checked_div(Fixed::from_int(2))
        .ok_or_else(e)?;
    let mut hi_edge = anchors.v0_cm3;
    let mut prev_p = pressure_gpa(anchors, anchors.v0_cm3, t_k).ok_or_else(e)?;
    for i in 1..=EXPANSION_STEPS {
        let v = anchors
            .v0_cm3
            .checked_add(
                span.checked_mul(Fixed::from_int(i as i32))
                    .and_then(|x| x.checked_div(Fixed::from_int(EXPANSION_STEPS as i32)))
                    .ok_or_else(e)?,
            )
            .ok_or_else(e)?;
        let p = match pressure_gpa(anchors, v, t_k) {
            Some(p) => p,
            None => break,
        };
        if p >= prev_p {
            break;
        }
        prev_p = p;
        hi_edge = v;
    }

    let mut lo = lo_edge;
    let mut hi = hi_edge;

    // P is DECREASING in V over the searched range, so the compressed edge carries the high pressure.
    let p_at_lo = pressure_gpa(anchors, lo, t_k).ok_or_else(e)?;
    let p_at_hi = pressure_gpa(anchors, hi, t_k).ok_or_else(e)?;
    if pressure_gpa_target > p_at_lo || pressure_gpa_target < p_at_hi {
        return Err(MgdFailure::OutsideBracket {
            requested_gpa: pressure_gpa_target.to_f64_lossy(),
            bracket_high_gpa: p_at_lo.to_f64_lossy(),
            bracket_low_gpa: p_at_hi.to_f64_lossy(),
        });
    }

    // FIXED trip count, no early exit. A tolerance-based break makes the iteration count depend on the
    // input, which is how a fixed-point solver quietly stops being reproducible.
    for _ in 0..BISECTION_STEPS {
        let mid = lo
            .checked_add(hi)
            .and_then(|s| s.checked_div(Fixed::from_int(2)))
            .ok_or_else(e)?;
        let p_mid = pressure_gpa(anchors, mid, t_k).ok_or_else(e)?;
        if p_mid > pressure_gpa_target {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let v = lo
        .checked_add(hi)
        .and_then(|s| s.checked_div(Fixed::from_int(2)))
        .ok_or_else(e)?;

    // K_T = -V (dP/dV)_T, central difference with a step small against V and large against resolution.
    let dv = v.checked_div(Fixed::from_int(10_000)).ok_or_else(e)?;
    let p_plus = pressure_gpa(anchors, v.checked_add(dv).ok_or_else(e)?, t_k).ok_or_else(e)?;
    let p_minus = pressure_gpa(anchors, v.checked_sub(dv).ok_or_else(e)?, t_k).ok_or_else(e)?;
    let dp_dv = p_plus
        .checked_sub(p_minus)
        .and_then(|d| d.checked_div(Fixed::from_int(2).checked_mul(dv)?))
        .ok_or_else(e)?;
    let k_t = Fixed::ZERO
        .checked_sub(v.checked_mul(dp_dv).ok_or_else(e)?)
        .ok_or_else(e)?;
    if k_t <= Fixed::ZERO {
        return Err(MgdFailure::Unrepresentable);
    }

    // C_V by central difference on the Debye thermal energy at the solved volume.
    let theta = theta_at(anchors, v).ok_or_else(e)?;
    let dt = Fixed::ONE;
    let e_plus = debye_thermal_energy_j_per_mol(
        t_k.checked_add(dt).ok_or_else(e)?,
        theta,
        anchors.atoms_per_formula_unit,
    )
    .ok_or_else(e)?;
    let e_minus = debye_thermal_energy_j_per_mol(
        t_k.checked_sub(dt).ok_or_else(e)?,
        theta,
        anchors.atoms_per_formula_unit,
    )
    .ok_or_else(e)?;
    let c_v = e_plus
        .checked_sub(e_minus)
        .and_then(|d| d.checked_div(Fixed::from_int(2).checked_mul(dt)?))
        .ok_or_else(e)?;

    // alpha = gamma C_V / (K_T V). The 1000 is the same J/cm^3-to-GPa bridge as the thermal pressure.
    let gamma = gamma_at(anchors, v).ok_or_else(e)?;
    let alpha = gamma
        .checked_mul(c_v)
        .and_then(|x| x.checked_div(k_t))
        .and_then(|x| x.checked_div(v))
        .and_then(|x| x.checked_div(Fixed::from_int(1000)))
        .ok_or_else(e)?;

    Ok(MgdResponse {
        molar_volume_cm3: v,
        bulk_modulus_gpa: k_t,
        alpha_per_k: alpha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::thermoelastic_anchors::ThermoelasticAnchors;

    fn f(x: f64) -> Fixed {
        Fixed::from_decimal_str(&format!("{x}")).expect("representable")
    }

    /// Forsterite's anchors, assembled from the banked columns exactly as a caller would.
    fn forsterite() -> MgdAnchors {
        let anc = ThermoelasticAnchors::standard().expect("anchors");
        let row = anc.row("forsterite").expect("forsterite row");
        MgdAnchors {
            v0_cm3: f(43.60),
            k0_gpa: f(128.0),
            k0_prime: f(4.2),
            theta_0: row.theta_0.expect("theta_0"),
            gamma_0: f(0.99),
            q: row.q.expect("q"),
            atoms_per_formula_unit: 7,
        }
    }

    /// THE APPROACH TO DULONG-PETIT, asserted against the analytic SERIES rather than against the limit.
    ///
    /// As `theta/T -> 0` every mode is classically excited and `E_th -> 3 n R T`. Testing only that
    /// endpoint is weak, and the weak version of this test is what I first wrote: it asserted the ratio
    /// was within half a percent of 1 at `T/theta = 25`, and failed at 0.9851. The code was right and the
    /// expectation was wrong. Expanding the integrand,
    ///
    /// ```text
    ///   x^3/(e^x - 1) = x^2 - x^3/2 + x^4/12 - ...
    ///   Integral_0^y  = y^3/3 - y^4/8 + y^5/60 - ...
    ///   E_th/(3 n R T) = 1 - 3y/8 + y^2/20 - ...,   y = theta/T
    /// ```
    ///
    /// so at `y = 0.04` the correct ratio is `0.98508`. The first quantum correction is LINEAR in `y` and
    /// does not become negligible nearly as fast as "25 times the Debye temperature" suggests.
    ///
    /// Asserting the series tests the integral's SHAPE at three separate points, which the endpoint alone
    /// cannot: a quadrature that got the leading term right and the correction wrong would pass a limit
    /// check and fail this one.
    #[test]
    fn the_debye_energy_follows_its_analytic_high_temperature_series() {
        let atoms = 7;
        let r = civsim_physics::gas_thermochemistry::molar_gas_constant().expect("R");
        let theta = Fixed::from_int(800);

        for t_int in [20_000, 8_000, 4_000] {
            let t = Fixed::from_int(t_int);
            let e = debye_thermal_energy_j_per_mol(t, theta, atoms).expect("representable");
            let dulong = Fixed::from_int(3)
                .checked_mul(Fixed::from_int(atoms as i32))
                .and_then(|x| x.checked_mul(r))
                .and_then(|x| x.checked_mul(t))
                .expect("representable");
            let ratio = e.to_f64_lossy() / dulong.to_f64_lossy();
            let y = 800.0 / f64::from(t_int);
            let series = 1.0 - 3.0 * y / 8.0 + y * y / 20.0;
            assert!(
                (ratio - series).abs() < 2e-4,
                "at theta/T = {y:.3} the series gives {series:.5} and the solver read {ratio:.5}"
            );
        }
    }

    /// The integral's OWN closed-form limit: as `y -> infinity` it tends to `pi^4/15 = 6.4939`.
    #[test]
    fn the_debye_integral_tends_to_its_analytic_infinite_limit() {
        let big = debye_integral(Fixed::from_int(30))
            .expect("representable")
            .to_f64_lossy();
        assert!(
            (6.40..=6.50).contains(&big),
            "the Debye integral tends to pi^4/15 = 6.4939; read {big:.4}"
        );
        // And the small-y limit is y^3/3, which is where the series branch is exercised.
        let small = debye_integral(f(0.03))
            .expect("representable")
            .to_f64_lossy();
        let expect = 0.03_f64.powi(3) / 3.0;
        assert!(
            (small - expect).abs() / expect < 0.02,
            "for small y the integral tends to y^3/3 = {expect:.3e}; read {small:.3e}"
        );
    }

    /// THE REFERENCE STATE REPRODUCES ITSELF. At 300 K and ambient pressure the solver must return the
    /// reference volume it was given, because that is what the anchors mean.
    ///
    /// This is a self-consistency check and is labelled as one: it proves the inversion and the pressure
    /// function agree, and it proves nothing about the world. The magnitude checks below do that.
    #[test]
    fn the_reference_state_returns_the_reference_volume() {
        let a = forsterite();
        let r = response_at(&a, f(0.0001), Fixed::from_int(300)).expect("ambient answers");
        let v = r.molar_volume_cm3.to_f64_lossy();
        assert!(
            (43.4..=43.8).contains(&v),
            "at the reference state the solver must return V_0 = 43.60; read {v:.3}"
        );
        let k = r.bulk_modulus_gpa.to_f64_lossy();
        assert!(
            (120.0..=136.0).contains(&k),
            "and the bulk modulus must return K_0 = 128 GPa; read {k:.1}"
        );
    }

    /// THE MAGNITUDE CHECK AGAINST MEASUREMENT, which is the one that tests the mechanism rather than its
    /// self-consistency.
    ///
    /// Forsterite's volumetric expansivity at ambient pressure and about 1000 K is roughly 4e-5 per K,
    /// and its isothermal bulk modulus falls with temperature from the 128 GPa reference. Neither number
    /// was used to build the solver, so agreement is evidence.
    #[test]
    fn forsterite_expansivity_and_modulus_at_temperature_match_measurement() {
        let a = forsterite();
        let r = response_at(&a, f(0.0001), Fixed::from_int(1000)).expect("answers at 1000 K");

        let alpha_ppm = r.alpha_per_k.to_f64_lossy() * 1e6;
        assert!(
            (30.0..=55.0).contains(&alpha_ppm),
            "forsterite's volumetric expansivity near 1000 K is about 40 ppm/K; read {alpha_ppm:.1}"
        );
        let k = r.bulk_modulus_gpa.to_f64_lossy();
        assert!(
            (100.0..=128.0).contains(&k),
            "and K_T must FALL below its 300 K value of 128 GPa, not rise; read {k:.1}"
        );
        // The volume must have expanded relative to the reference.
        let v = r.molar_volume_cm3.to_f64_lossy();
        assert!(
            v > 43.6,
            "heating at constant pressure expands the cell; read {v:.3} against V_0 = 43.60"
        );
    }

    /// COMPRESSION RAISES THE MODULUS AND SHRINKS THE CELL, the sign check that catches a transposed
    /// bracket or an inverted derivative.
    #[test]
    fn compression_stiffens_the_phase_and_reduces_its_volume() {
        let a = forsterite();
        let ambient = response_at(&a, f(0.0001), Fixed::from_int(1000)).expect("ambient");
        let deep = response_at(&a, Fixed::from_int(10), Fixed::from_int(1000)).expect("10 GPa");
        assert!(
            deep.molar_volume_cm3 < ambient.molar_volume_cm3,
            "10 GPa must compress the cell"
        );
        assert!(
            deep.bulk_modulus_gpa > ambient.bulk_modulus_gpa,
            "and stiffen it: K_T rises with pressure, roughly as K_0'"
        );
        assert!(
            deep.alpha_per_k < ambient.alpha_per_k,
            "and expansivity falls under compression, which is what a positive q encodes"
        );
    }

    /// A REQUEST OUTSIDE THE BRACKET REFUSES rather than clamping to the edge.
    #[test]
    fn a_pressure_outside_the_searched_bracket_refuses_rather_than_clamping() {
        let a = forsterite();
        let err = response_at(&a, Fixed::from_int(100_000), Fixed::from_int(1000))
            .expect_err("an absurd pressure has no volume in the bracket");
        assert!(
            matches!(err, MgdFailure::OutsideBracket { .. }),
            "and it says so rather than returning the bracket edge as an answer: {err:?}"
        );
    }

    /// THE SPINODAL CATCH, live-fired on the phase that exposed it.
    ///
    /// This is the test the previous bracket failed. Enstatite's `q` is 7.8, the largest in the banked
    /// column, so `gamma` at `1.5 V_0` is 22 times its reference value and the thermal pressure overwhelms
    /// the flattening Birch-Murnaghan term. `P(V)` turns around at `1.156 V_0`, and the old fixed bracket
    /// edge sat at `+20.65 GPa`, ABOVE ambient. An ambient-pressure query was therefore refused as
    /// out-of-bracket on a phase with a perfectly good ambient solution.
    ///
    /// Neither forsterite nor periclase turns inside the old bracket, which is why the original tests
    /// passed. A defect visible on two of seven phases and invisible on the two I happened to check is the
    /// same shape as the Debye error this module already carries: the sample chose the answer.
    #[test]
    fn every_banked_phase_solves_at_ambient_pressure_including_the_stiff_exponent_one() {
        let anc = ThermoelasticAnchors::standard().expect("anchors");
        // (phase, V_0, K_0, K_0', gamma_0, atoms per formula unit) from the banked columns.
        for (phase, v0, k0, kp, g0, atoms) in [
            ("forsterite", 43.60, 128.0, 4.2, 0.99, 7u32),
            ("periclase", 11.24, 161.0, 3.9, 1.50, 2),
            ("fayalite", 46.29, 135.0, 4.2, 1.06, 7),
            ("enstatite", 125.35, 107.0, 7.1, 0.67, 20),
        ] {
            let row = anc.row(phase).expect("row");
            let a = MgdAnchors {
                v0_cm3: f(v0),
                k0_gpa: f(k0),
                k0_prime: f(kp),
                theta_0: row.theta_0.expect("theta_0"),
                gamma_0: f(g0),
                q: row.q.expect("q"),
                atoms_per_formula_unit: atoms,
            };
            let r = response_at(&a, f(0.0001), Fixed::from_int(1000))
                .unwrap_or_else(|e| panic!("{phase} must solve at ambient and 1000 K: {e:?}"));

            // The solution must sit on the MECHANICALLY STABLE branch: expanded from the 300 K reference
            // by heating, but well inside the spinodal, with a positive bulk modulus.
            let v = r.molar_volume_cm3.to_f64_lossy();
            assert!(
                v > v0 && v < v0 * 1.15,
                "{phase}: heating to 1000 K expands the cell modestly; read {v:.3} against V_0 = {v0:.2}"
            );
            assert!(
                r.bulk_modulus_gpa > Fixed::ZERO,
                "{phase}: a solution past the spinodal would carry a NEGATIVE bulk modulus, which is the \
                 unstable branch the bracket search exists to exclude"
            );
            assert!(
                r.alpha_per_k > Fixed::ZERO,
                "{phase}: and a positive expansivity"
            );
        }
    }

    /// DETERMINISM: the same query returns bit-identical results, and the fixed trip count means that is
    /// true regardless of how hard the particular inversion was.
    #[test]
    fn the_solver_is_bit_reproducible() {
        let a = forsterite();
        let one = response_at(&a, Fixed::from_int(5), Fixed::from_int(1200)).expect("answers");
        let two = response_at(&a, Fixed::from_int(5), Fixed::from_int(1200)).expect("answers");
        assert_eq!(one, two, "same inputs, same bits");
    }
}
