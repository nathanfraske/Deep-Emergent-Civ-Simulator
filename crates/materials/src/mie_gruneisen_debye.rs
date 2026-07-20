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
//! One response costs roughly `BISECTION_STEPS * (SIMPSON_INTERVALS + 1)` evaluations of `Fixed::exp` for
//! the volume inversion alone, about two thousand at ordinary interior states, rising where `theta/T` is
//! large enough to widen the quadrature. The stability-edge solve adds two pressure evaluations per bracket
//! step and two per bisection step, and each banded derivative costs four evaluations where an unbanded one
//! costs two. That is a worldgen-time budget, not a per-tick one, and callers on a tick path should cache
//! per (phase, state bucket) rather than call this in a loop.

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl MgdAnchors {
    /// Assemble the six anchors for a phase from the banked columns, or refuse.
    ///
    /// # The gamma this reads, and the one it must not
    ///
    /// It reads `gamma_eos_debye` DIRECTLY and never `GruneisenRow::gamma()`. That accessor returns the
    /// ladder's preferred value, the measured thermodynamic gamma where one exists, and the thermodynamic
    /// and EoS-Debye gammas are two different weightings of the vibrational spectrum. `gruneisen.toml`'s
    /// own header says so: "Do NOT read gamma_eos_debye into the gamma_thermodynamic slot", and the
    /// converse binds here. The EoS gamma is the one jointly fit with this row's `theta_0` and `q`, so it
    /// is the only one that belongs in this parameter set.
    ///
    /// This is the same error the module documentation records for the Debye temperature, one quantity
    /// over. Both would have been a plausible-looking value from a nearby column.
    ///
    /// # One fit, or nothing
    ///
    /// `V_0`, `K_0`, `K_0'`, `theta_0` and `q` all come from THIS row, which transcribes one source table,
    /// and `gamma_0` is checked against that same fit through `pairs_with_banked_gamma`. A row whose cells
    /// were estimated from systematics, or whose fit does not reproduce the banked `gamma_0`, is refused
    /// rather than assembled: the parameters of a joint inversion are meaningful together and not
    /// individually.
    pub fn from_banked(
        phase: &str,
        gruneisen: &civsim_physics::gruneisen::GruneisenTable,
        anchors: &civsim_physics::thermoelastic_anchors::ThermoelasticAnchors,
    ) -> Option<Self> {
        Self::families(phase, gruneisen, anchors)
            .0
            .into_iter()
            .next()
            .map(|f| f.anchors)
    }

    /// EVERY source inversion the row transcribes, assembled separately and tagged, with the families that
    /// could not be assembled and why.
    ///
    /// # The Gap Law applied where a point was being returned
    ///
    /// The column banks two global inversions per mantle row and this assembly used to pick one, so a
    /// quantity the data holds as a disagreement reached the engine as a point. Enstatite is the live case:
    /// `q` is 7.8 in 2005 and 3.4 in 2011, with `gamma_0` moving 0.67 to 0.78, and `q` enters two
    /// exponentials. At moderate mantle compression those two published inversions give thermal pressures
    /// differing by more than a factor of two, and one of them was arriving with no delta attached.
    ///
    /// # What survived before, and why it was not enough
    ///
    /// `ChannelAgreement` recorded THAT the channels disagree. It did not record by how much or toward
    /// what, and a consumer cannot do anything with a boolean disagreement except ignore it.
    ///
    /// # One joint fit, per family, still
    ///
    /// The primary family reads `gamma_0` from the BANK and is refused unless
    /// `pairs_with_banked_gamma` holds, exactly as before. A successor family reads its OWN `gamma_0` and
    /// is refused if it has none, because mixing one inversion's `q` with another's `gamma_0` is the pair
    /// that was never jointly constrained. Neither family borrows a cell from the other.
    ///
    /// # Identical channels are ONE determination
    ///
    /// Corundum, forsterite and fayalite are reproduced cell for cell by the successor. Two byte-identical
    /// anchor sets are one answer heard twice, not two answers, so they collapse to a single branch that
    /// names both families in `concurring`. The ensemble is then absent where the data holds no
    /// disagreement, rather than being ceremony on every row.
    // @derives: a phase's per-inversion MGD anchor sets <- each source family's own jointly-fit cells
    pub fn families(
        phase: &str,
        gruneisen: &civsim_physics::gruneisen::GruneisenTable,
        anchors: &civsim_physics::thermoelastic_anchors::ThermoelasticAnchors,
    ) -> (Vec<FamilyAnchors>, Vec<FamilyExclusion>) {
        let mut assembled: Vec<FamilyAnchors> = Vec::new();
        let mut excluded: Vec<FamilyExclusion> = Vec::new();
        let Some(row) = anchors.row(phase) else {
            return (assembled, excluded);
        };
        let banked_gamma = gruneisen.row(phase).and_then(|r| r.gamma_eos_debye);

        for (index, channel) in row.channels.iter().enumerate() {
            let mut reasons: Vec<String> = Vec::new();
            // The row's own refusal flag and cell grades bind every channel: a row the file marks unusable
            // is unusable in both inversions.
            if !row.all_cells_fit() {
                reasons.push(
                    "the row's cells are not all fit: a systematics estimate or the file's own \
                     usable_as_anchor refusal"
                        .to_string(),
                );
            }
            if matches!(channel.q_grade, Some(g) if !g.usable_as_anchor()) {
                reasons.push("this channel's q is from systematics rather than fit".to_string());
            }
            // gamma_0: the bank for the primary, the channel's own for a successor. Never the other's.
            let gamma_0 = if index == 0 {
                if !row.pairs_with_banked_gamma() {
                    reasons.push(
                        "this row's own fit does not reproduce the banked gamma_0, so its cells and the \
                         bank are two inversions"
                            .to_string(),
                    );
                }
                banked_gamma
            } else {
                if channel.gamma_0.is_none() {
                    reasons.push(
                        "this channel transcribes no gamma_0 of its own, and reading the banked one \
                         against its q would pair two inversions that were never jointly constrained"
                            .to_string(),
                    );
                }
                channel.gamma_0
            };
            let cells = (
                channel.v0_cm3,
                channel.k0_gpa,
                channel.k0_prime,
                channel.theta_0,
                gamma_0,
                channel.q,
                channel.atoms_per_formula_unit,
            );
            let complete = match cells {
                (Some(v0), Some(k0), Some(kp), Some(theta_0), Some(g0), Some(q), Some(atoms)) => {
                    Some(MgdAnchors {
                        v0_cm3: v0,
                        k0_gpa: k0,
                        k0_prime: kp,
                        theta_0,
                        gamma_0: g0,
                        q,
                        atoms_per_formula_unit: atoms,
                    })
                }
                _ => {
                    reasons.push("this channel does not transcribe all six anchors".to_string());
                    None
                }
            };
            match (complete, reasons.is_empty()) {
                (Some(a), true) => {
                    // A channel that reproduces an already-assembled one cell for cell is that one answer
                    // heard twice. Recorded as concurrence rather than admitted as a second branch.
                    if let Some(prior) = assembled.iter_mut().find(|f| f.anchors == a) {
                        prior.concurring.push(channel.family.clone());
                    } else {
                        assembled.push(FamilyAnchors {
                            family: channel.family.clone(),
                            concurring: Vec::new(),
                            anchors: a,
                            q_band: channel.q_band,
                            theta_0_band_k: channel.theta_0_band_k,
                            gamma_0_band: channel.gamma_0_band,
                        });
                    }
                }
                _ => excluded.push(FamilyExclusion {
                    family: channel.family.clone(),
                    reasons,
                }),
            }
        }
        (assembled, excluded)
    }
}

/// One source inversion's assembled anchor set, tagged with the family it came from and the marginal bands
/// that family's source states.
///
/// The bands are the source's own MARGINAL uncertainties, carried as stated and deliberately not
/// propagated into a single band on the response. The parameters of a global inversion are correlated and
/// the papers publish the marginals without the covariance matrix, so combining them as if independent
/// would move the result by an unknown amount in an unknown direction. Carrying them per anchor says what
/// is known; a propagated scalar would say more than the sources do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyAnchors {
    /// The source inversion these cells came from.
    pub family: String,
    /// Other families whose cells are byte-identical to these, so they are this same determination.
    pub concurring: Vec<String>,
    /// The six anchors, jointly fit.
    pub anchors: MgdAnchors,
    /// `q`'s marginal band as this family states it.
    pub q_band: Option<Fixed>,
    /// `theta_0`'s marginal band as this family states it (K).
    pub theta_0_band_k: Option<Fixed>,
    /// `gamma_0`'s marginal band as this family states it.
    pub gamma_0_band: Option<Fixed>,
}

/// A source family whose anchors could not be assembled, with EVERY reason rather than the first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyExclusion {
    /// The family that was excluded.
    pub family: String,
    /// Every reason it was excluded. A family can fail several tests at once, and reporting one would
    /// understate what it would take to bring it in.
    pub reasons: Vec<String>,
}

/// The reference temperature the anchors are stated at (K). The cold isotherm is the 300 K one, so the
/// thermal pressure is taken as a DIFFERENCE from this temperature rather than as an absolute.
///
/// PUBLIC SO THE ASSUMPTION CAN BE CHECKED AGAINST THE DATA'S OWN DECLARATION. Every anchor row transcribes
/// its reference state, and this solver holds a constant: if the two ever disagree the anchors would be
/// re-anchored silently at a temperature they were not fit against. The ladder tests one against the other
/// rather than trusting that they match.
pub const REFERENCE_TEMPERATURE_K: i32 = 300;

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
    // THE DECAYING FORM, `x^3 e^-x / (1 - e^-x)`, which is algebraically the same as `x^3 / (e^x - 1)` and
    // numerically not the same at all.
    //
    // The direct form was WRONG over this function's own integration range. `Fixed::exp` saturates to
    // `Fixed::MAX` above an argument of about 22 while the Debye integral is evaluated to 30, so across that
    // whole upper span the denominator was a constant `Fixed::MAX - 1` and the code integrated roughly
    // `x^3 / Fixed::MAX` instead of an exponentially decaying tail. At `x = 25` that is `7.3e-6` against a
    // true `2.2e-7`, a factor of 33; by `x = 30` it is `1.3e-5` against `2.5e-9`, a factor of 5000. The tail
    // was not merely imprecise, it was rising where the physics falls. Found by review.
    //
    // `e^-x` FAILS IN THE SAFE DIRECTION: it underflows smoothly toward zero exactly where `e^x` saturates,
    // so the integrand goes to zero as the physics does, and the residual error at the far end is bounded by
    // an ulp rather than by the representation's ceiling.
    let neg_exp = (Fixed::ZERO.checked_sub(x)?).exp();
    let denom = Fixed::ONE.checked_sub(neg_exp)?;
    if denom <= Fixed::ZERO {
        return None;
    }
    let x3 = x.checked_mul(x)?.checked_mul(x)?;
    x3.checked_mul(neg_exp)?.checked_div(denom)
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

/// Whether the `q -> 0` logarithmic limit is the more accurate of the two forms at this state.
///
/// THE CROSSOVER IS DERIVED BY BALANCING THE TWO ERRORS, not chosen. Write `L = ln(V/V_0)` and `u` for one
/// `Fixed` ulp, which is `2^-FRAC_BITS`:
///
/// ```text
///   direct form   1 - r^q carries an ulp, and gamma_0/|q| amplifies it:   exponent error  gamma_0 u / |q|
///   limit form    E(q) = -gamma_0 L - gamma_0 q L^2/2 - ..., dropping
///                 the series after the first term costs:                  exponent error  gamma_0 |q| L^2 / 2
///   they are equal when   u/|q| = |q| L^2 / 2,   that is   |q L| = sqrt(2 u)
/// ```
///
/// So the test is on the PRODUCT `|q L|` at the state being asked about, against `sqrt(2 u)`. Nothing here
/// is authored: `u` is the representation's own resolution and the 2 is the second-order Taylor
/// coefficient. `Fixed::from_bits(2)` IS `2 u`, so the crossover is written in the representation whose
/// resolution set it, and it moves on its own if `FRAC_BITS` ever does.
///
/// Comparing the product rather than solving for a threshold `q` avoids dividing by `L`, which is zero at
/// the reference volume; there both forms return `theta_0` and this one takes the limit branch.
// @derives: the branch point between the direct and limiting Debye-temperature forms <- the representation's ulp and the second-order Taylor coefficient
fn logarithmic_limit_is_the_better_branch(q: Fixed, ln_ratio: Fixed) -> bool {
    let crossover = Fixed::from_bits(2).sqrt();
    match q.abs().checked_mul(ln_ratio.abs()) {
        Some(product) => product < crossover,
        // An overflow means `|q L|` is enormous, which is as far from the limit as a state can be.
        None => false,
    }
}

/// The volume-dependent Debye temperature, `theta_0 exp[(gamma_0/q)(1 - (V/V_0)^q)]`, and its `q -> 0` limit.
///
/// This is the integrated form of `gamma = -dln(theta)/dln(V)` under a constant `q`, so the `theta` the
/// thermal energy uses and the `gamma` the thermal pressure uses describe the same solid rather than two
/// unrelated parameterisations.
///
/// # The limit is a model member, and the neighbourhood is worse than the limit
///
/// An earlier version returned `None` at `q == 0`, refusing a solid the model describes perfectly well. The
/// limit is finite and analytic: as `q -> 0`, `(gamma_0/q)(1 - (V/V_0)^q) -> -gamma_0 ln(V/V_0)`, so
/// `theta -> theta_0 (V/V_0)^(-gamma_0)`. [`gamma_at`] had accepted `q = 0` all along, returning the
/// constant `gamma_0` that the same limit gives, so the two halves of one parameterisation disagreed about
/// whether `q = 0` names a solid.
///
/// The sharper defect was the NEIGHBOURHOOD of zero, which no guard covered at all. The direct form's
/// numerator `1 - (V/V_0)^q` has true magnitude about `|q ln(V/V_0)|`, and `gamma_0/q` then amplifies
/// whatever survives. At `q = 1e-6` and `V/V_0 = 0.85` that numerator is about `1.6e-7` against a `Fixed`
/// resolution of `2.33e-10`: roughly three significant digits, multiplied by a million, and then
/// exponentiated. Refusing at exactly zero while answering confidently either side of it is the wrong shape
/// of defence, and [`logarithmic_limit_is_the_better_branch`] replaces it with the derived crossover.
// @derives: a phase's Debye temperature at volume <- its reference Debye temperature, gamma and volume exponent
fn theta_at(anchors: &MgdAnchors, v_cm3: Fixed) -> Option<Fixed> {
    let ratio = v_cm3.checked_div(anchors.v0_cm3)?;
    if ratio <= Fixed::ZERO {
        return None;
    }
    let ln_ratio = ratio.ln();
    let exponent = if logarithmic_limit_is_the_better_branch(anchors.q, ln_ratio) {
        // `E = -gamma_0 ln(V/V_0)`, so `theta = theta_0 (V/V_0)^(-gamma_0)`.
        Fixed::ZERO.checked_sub(anchors.gamma_0.checked_mul(ln_ratio)?)?
    } else {
        anchors
            .gamma_0
            .checked_div(anchors.q)?
            .checked_mul(Fixed::ONE.checked_sub(ratio.powf(anchors.q))?)?
    };
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
    /// An intermediate left the representable window, or an input was non-physical. A NUMERICAL refusal:
    /// the state may be perfectly ordinary and this representation could not carry the arithmetic to it.
    Unrepresentable,
    /// The solved state lies past the phase's mechanical stability limit, where `K_T <= 0` and the solid is
    /// not a solid. A PHYSICAL refusal, and the split from [`MgdFailure::Unrepresentable`] is the point:
    /// the two rode in one variant, and `thermoelastic.rs` said so in prose ("past the spinodal, or an
    /// unrepresentable intermediate"), which left a caller unable to tell a phase that has no stable state
    /// at these conditions from one this engine cannot describe.
    PastSpinodal {
        /// The located fold volume (cm^3/mol), bisected on `K_T = 0` rather than walked to.
        v_fold_cm3: f64,
        /// The temperature the fold was located at, since the spinodal moves with it.
        t_k: f64,
    },
    /// The requested state lies outside the bracket the inversion searches, so no volume in the searched
    /// range produces the requested pressure. Reported rather than clamped: clamping to the bracket edge
    /// would return a confident wrong answer.
    ///
    /// The two edges are both reported so a caller can tell which was crossed: a request ABOVE
    /// `bracket_high_gpa` is past the compressed end of the searched range, and one BELOW `bracket_low_gpa`
    /// has no mechanically stable volume at all, the expanded end being the spinodal itself.
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
///
/// # The capacity is carried, and the reason it has to be
///
/// `C_V` is computed here on the way to `alpha` and was DISCARDED at the function boundary, so the one
/// consumer that needed a heat capacity reached for Dulong-Petit `3R/M` instead. That substitution is the
/// defect this whole ladder was written to end, one quantity over: measured against this Debye `C_V` at
/// forsterite's `theta_0` of 809 K, the Dulong-Petit value runs +39.4 percent at 300 K, +2.7 percent at
/// 800 K and -2.8 percent at 1600 K. At mantle temperature the errors offset to about 3 percent, which is
/// agreement by CANCELLATION rather than by physics, and cancellation at one temperature is exactly what
/// the module documentation on `crate::thermoelastic` records as the original finding.
///
/// # The bands are numerical, not physical
///
/// `bulk_modulus_band_gpa` and `c_v_band_j_per_mol_k` bound the error of the DIFFERENCING, and nothing
/// else. They say how well the central differences resolved their derivatives at this state; they say
/// nothing about how well the six anchors describe the phase, which is the input band and lives on
/// `crate::thermoelastic::ThermoelasticBranch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MgdResponse {
    /// Molar volume at the requested state (cm^3/mol).
    pub molar_volume_cm3: Fixed,
    /// Isothermal bulk modulus at the requested state (GPa).
    pub bulk_modulus_gpa: Fixed,
    /// Volumetric thermal expansivity at the requested state (per K).
    pub alpha_per_k: Fixed,
    /// Isochoric heat capacity at the requested state (J/mol/K), on the SAME molar basis as
    /// `MgdAnchors::atoms_per_formula_unit`, from the same Debye model the thermal pressure uses.
    pub c_v_j_per_mol_k: Fixed,
    /// The numerical band on `bulk_modulus_gpa` (GPa). See [`central_difference_with_band`].
    pub bulk_modulus_band_gpa: Fixed,
    /// The numerical band on `c_v_j_per_mol_k` (J/mol/K).
    pub c_v_band_j_per_mol_k: Fixed,
    /// The numerical band on `alpha_per_k` (per K), propagated from the two differences it divides and
    /// multiplies, so an expansivity does not arrive looking better resolved than its own inputs.
    pub alpha_band_per_k: Fixed,
}

/// The divisor setting the volume step of the `K_T` central difference: `h = V / 10_000`.
///
/// Named rather than written inline because the spinodal search and the response now take the derivative
/// through the SAME helper, and two copies of a differencing step is how a located fold and a reported
/// modulus quietly stop describing the same curve.
const DERIVATIVE_STEP_DIVISOR: i32 = 10_000;

/// A central difference WITH the numerical band that says how well it resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BandedDerivative {
    /// The derivative at the coarse step `h`.
    value: Fixed,
    /// The band on `value`: Richardson truncation plus the representation's own floor.
    band: Fixed,
}

/// A central difference evaluated at `h` AND at `h/2`, returning the coarse value with a MEASURED band.
///
/// # Why a band at all, when the steps are well chosen
///
/// They are well chosen: at forsterite's ordinary states `h = V/10_000` gives a pressure difference around
/// `0.026 GPa`, about `1.1e8` ulps of headroom, for a relative truncation near `1e-8`. The defect is the
/// ABSENCE OF A MEASURED BAND rather than a bad step, and it is not decorative. `K_T = -V (dP/dV)` has
/// `dP/dV -> 0` at the spinodal, so the numerator collapses toward the ulp floor while the floor itself
/// stays put. The absolute band barely moves; the RELATIVE band blows up, in exactly the region the fold
/// search is trying to locate. The step-count defect and the missing band are one defect seen twice.
///
/// # What the band is
///
/// For a central difference the truncation goes as `h^2`, so `D(h) - D(h/2) = f''' h^2 / 8` while `D(h)`'s
/// own truncation is `f''' h^2 / 6`. The reported value is `D(h)`, so its truncation estimate is
/// `(4/3)|D(h) - D(h/2)|`. (The familiar `|D(h) - D(h/2)|/3` estimates the error of `D(h/2)`, which is four
/// times smaller and is not the number returned here; using it for `D(h)` would understate the band
/// fourfold.) To that is added the representation floor: the two evaluations each carry at least an ulp,
/// their difference carries two, and dividing by `2h` leaves `u/h`.
///
/// The floor bounds the DIFFERENCING only. It does not bound the error already accumulated inside `f`
/// itself, and no number here claims to.
// @derives: a central difference's numerical band <- its own step-halved twin and the representation's ulp
fn central_difference_with_band(
    at: Fixed,
    h: Fixed,
    f: impl Fn(Fixed) -> Option<Fixed>,
) -> Option<BandedDerivative> {
    let difference = |step: Fixed| -> Option<Fixed> {
        let plus = f(at.checked_add(step)?)?;
        let minus = f(at.checked_sub(step)?)?;
        plus.checked_sub(minus)?
            .checked_div(Fixed::from_int(2).checked_mul(step)?)
    };
    let coarse = difference(h)?;
    let fine = difference(h.checked_div(Fixed::from_int(2))?)?;
    let gap = if coarse > fine {
        coarse.checked_sub(fine)?
    } else {
        fine.checked_sub(coarse)?
    };
    let truncation = Fixed::from_int(4)
        .checked_mul(gap)?
        .checked_div(Fixed::from_int(3))?;
    let representation_floor = Fixed::from_bits(1).checked_div(h)?;
    Some(BandedDerivative {
        value: coarse,
        band: truncation.checked_add(representation_floor)?,
    })
}

/// The isothermal bulk modulus at a volume, `K_T = -V (dP/dV)_T`, with its numerical band.
///
/// Taken by central difference on the same `P(V,T)` the inversion solves, so the modulus is the derivative
/// of the function that was solved rather than an independent analytic expression that could drift from it.
// @derives: a phase's isothermal bulk modulus at a volume <- the volume derivative of its own equation of state
fn bulk_modulus_with_band(
    anchors: &MgdAnchors,
    v_cm3: Fixed,
    t_k: Fixed,
) -> Option<BandedDerivative> {
    let h = v_cm3.checked_div(Fixed::from_int(DERIVATIVE_STEP_DIVISOR))?;
    let dp_dv = central_difference_with_band(v_cm3, h, |v| pressure_gpa(anchors, v, t_k))?;
    Some(BandedDerivative {
        value: Fixed::ZERO.checked_sub(v_cm3.checked_mul(dp_dv.value)?)?,
        band: v_cm3.checked_mul(dp_dv.band)?,
    })
}

/// How far the phase can be expanded before it stops being a mechanically stable solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpansionEdge {
    /// The fold was bracketed on the SIGN of `K_T` and then bisected to `K_T = 0`. This is the spinodal.
    Spinodal(Fixed),
    /// No fold inside the searched span. This is the furthest volume the search reached and confirmed
    /// stable, and it is a search limit rather than a property of the phase.
    StableThroughout(Fixed),
}

impl ExpansionEdge {
    /// The volume either way, which is what the bracket needs.
    fn volume(self) -> Fixed {
        match self {
            ExpansionEdge::Spinodal(v) | ExpansionEdge::StableThroughout(v) => v,
        }
    }
}

/// Locate the mechanical stability limit: BRACKET on the sign of `K_T`, then BISECT to `K_T = 0`.
///
/// # Why this is a solve and no longer a walk
///
/// The previous version walked `EXPANSION_STEPS` samples of `V_0/128` and kept the last one where `P` was
/// still falling. Its break test compares SAMPLES, so it cannot see a turn that happens inside one step,
/// and the located edge is therefore a function of the step count. Measured on enstatite at 1000 K, where
/// the fold is at `143.813 cm^3/mol`: the 64-step walk stops at `143.957` and the 256-step walk at
/// `143.712`, disagreeing by `0.245`, one 256-step step. The 64-step answer is PAST the fold, so the
/// bracket it produced enclosed volumes on the unstable branch, which is the branch the search exists to
/// exclude.
///
/// A second hazard sits underneath: the break is a non-strict `>=` on a quantized quantity, so once the
/// step shrinks to where successive pressures differ by less than one ulp, the walk terminates on the
/// REPRESENTATION rather than on the physical turn. It is latent at the shipped step counts and becomes the
/// binding error as the walk is refined, which is the wrong way round for a refinement.
///
/// Bracketing on the SIGN of `K_T` and bisecting removes both. A sign is a discrete predicate no
/// quantization can blur, and the located fold is then step-count independent to the bisection's own
/// tolerance rather than to the walk's step: the same two step counts now return `143.813188916` from
/// both, to the bit.
///
/// `bracket_steps` is a parameter so the step-count independence is testable rather than asserted.
// @derives: a phase's spinodal volume at a temperature <- the zero of its own isothermal bulk modulus
fn expansion_edge(anchors: &MgdAnchors, t_k: Fixed, bracket_steps: u32) -> Option<ExpansionEdge> {
    let span = anchors.v0_cm3.checked_div(Fixed::from_int(2))?;
    let mut last_stable = anchors.v0_cm3;
    let mut bracket: Option<(Fixed, Fixed)> = None;
    for i in 1..=bracket_steps {
        let v = anchors.v0_cm3.checked_add(
            span.checked_mul(Fixed::from_int(i as i32))?
                .checked_div(Fixed::from_int(bracket_steps as i32))?,
        )?;
        match bulk_modulus_with_band(anchors, v, t_k) {
            Some(k) if k.value > Fixed::ZERO => last_stable = v,
            // K_T has gone non-positive: the fold lies between the last stable sample and this one.
            Some(_) => {
                bracket = Some((last_stable, v));
                break;
            }
            // An unrepresentable intermediate ends the searched range at the last stable sample. This is a
            // limit of the arithmetic and is reported as one, never as a located spinodal.
            None => break,
        }
    }
    let Some((mut lo, mut hi)) = bracket else {
        return Some(ExpansionEdge::StableThroughout(last_stable));
    };
    // The same FIXED trip count the volume inversion uses, and for the same reason: a tolerance-based break
    // makes the iteration count depend on the input, which is how a fixed-point solver stops being
    // reproducible.
    for _ in 0..BISECTION_STEPS {
        let mid = lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?;
        match bulk_modulus_with_band(anchors, mid, t_k) {
            Some(k) if k.value > Fixed::ZERO => lo = mid,
            _ => hi = mid,
        }
    }
    Some(ExpansionEdge::Spinodal(
        lo.checked_add(hi)?.checked_div(Fixed::from_int(2))?,
    ))
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
/// the derivative of the function that was solved rather than an independent analytic expression that could
/// drift from it. `alpha = gamma C_V / (K_T V)`, with `C_V` from the same Debye model, again by central
/// difference in temperature rather than by a closed form.
///
/// Every difference is taken through [`central_difference_with_band`], so each derivative arrives with a
/// MEASURED band rather than an unstated one, and `C_V` is carried out on the response instead of dying at
/// this boundary. Both were real gaps: see [`MgdResponse`] for what the discarded capacity cost the one
/// consumer that needed a heat capacity.
// @derives: a phase's molar volume, bulk modulus, expansivity and heat capacity at a state <- its six Mie-Grueneisen-Debye anchors
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
    // gamma at 1.5 V_0 is 22 times its reference value; the turn arrives near 1.15 V_0 and the old fixed
    // bracket edge sat at +20.65 GPa. An AMBIENT-PRESSURE request therefore fell below the edge pressure
    // and was refused as out-of-bracket, on a phase that has a perfectly good ambient solution. Fayalite
    // turns at 1.42 V_0; forsterite and periclase do not turn inside the searched span at all, which is
    // exactly why a check on one or two phases would have missed this.
    //
    // The edge is SOLVED rather than walked to: `expansion_edge` brackets on the SIGN of K_T and bisects to
    // K_T = 0, so the located fold is a property of the phase and the temperature rather than of the loop
    // constant that found it. See that function for what the walk it replaced got wrong.
    const EXPANSION_BRACKET_STEPS: u32 = 64;
    let edge = expansion_edge(anchors, t_k, EXPANSION_BRACKET_STEPS).ok_or_else(e)?;

    let mut lo = lo_edge;
    let mut hi = edge.volume();

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

    // K_T = -V (dP/dV)_T, central difference with a step small against V and large against resolution,
    // now carrying the band its step-halved twin measures.
    let k = bulk_modulus_with_band(anchors, v, t_k).ok_or_else(e)?;
    let k_t = k.value;
    if k_t <= Fixed::ZERO {
        // THE PHYSICAL REFUSAL, SEPARATED FROM THE NUMERICAL ONE. A non-positive modulus at the solved
        // volume means the state is past the fold, and the fold has been located, so the refusal can name
        // where. Where no fold was found inside the searched span there is nothing to name, and a
        // non-positive modulus there is the arithmetic failing rather than the phase.
        return Err(match edge {
            ExpansionEdge::Spinodal(v_fold) => MgdFailure::PastSpinodal {
                v_fold_cm3: v_fold.to_f64_lossy(),
                t_k: t_k.to_f64_lossy(),
            },
            ExpansionEdge::StableThroughout(_) => MgdFailure::Unrepresentable,
        });
    }

    // C_V by central difference on the Debye thermal energy at the solved volume, with its own band.
    let theta = theta_at(anchors, v).ok_or_else(e)?;
    let atoms = anchors.atoms_per_formula_unit;
    let c = central_difference_with_band(t_k, Fixed::ONE, |t| {
        debye_thermal_energy_j_per_mol(t, theta, atoms)
    })
    .ok_or_else(e)?;
    let c_v = c.value;

    // alpha = gamma C_V / (K_T V). The 1000 is the same J/cm^3-to-GPa bridge as the thermal pressure.
    let gamma = gamma_at(anchors, v).ok_or_else(e)?;
    let alpha = gamma
        .checked_mul(c_v)
        .and_then(|x| x.checked_div(k_t))
        .and_then(|x| x.checked_div(v))
        .and_then(|x| x.checked_div(Fixed::from_int(1000)))
        .ok_or_else(e)?;

    // THE EXPANSIVITY'S BAND IS PROPAGATED, not re-measured. `alpha` multiplies C_V and divides K_T, so to
    // first order its RELATIVE band is the sum of theirs, which is the standard combination for a product
    // of independently-resolved factors. Both are numerical bands from the same two difference stencils, so
    // this says how well alpha was RESOLVED and nothing about how well the anchors describe the phase.
    let relative = c
        .band
        .checked_div(c_v)
        .and_then(|x| x.checked_add(k.band.checked_div(k_t)?))
        .ok_or_else(e)?;
    let alpha_band = alpha.checked_mul(relative).ok_or_else(e)?;

    Ok(MgdResponse {
        molar_volume_cm3: v,
        bulk_modulus_gpa: k_t,
        alpha_per_k: alpha,
        c_v_j_per_mol_k: c_v,
        bulk_modulus_band_gpa: k.band,
        c_v_band_j_per_mol_k: c.band,
        alpha_band_per_k: alpha_band,
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

    /// THE `q -> 0` LIMIT IS A MODEL MEMBER, and this is the test the refusal at exactly zero failed.
    ///
    /// The assertion is against the closed form `theta_0 (V/V_0)^(-gamma_0)` and against the SERIES around
    /// it rather than against a tolerance chosen by eye. Expanding the exponent,
    ///
    /// ```text
    ///   E(q) = (gamma_0/q)(1 - r^q) = -gamma_0 L - gamma_0 q L^2/2 - ...,   L = ln(V/V_0)
    /// ```
    ///
    /// so the departure from the limit is LINEAR in `q` with a coefficient this test computes, and it
    /// changes sign through zero. That is what makes the five samples monotone. A mechanism that branched
    /// in the wrong place, or kept the direct form where its numerator has lost its digits, breaks the
    /// ORDERING rather than merely losing accuracy, which is why monotonicity is asserted beside magnitude.
    #[test]
    fn the_debye_temperature_is_continuous_through_q_equals_zero() {
        let base = forsterite();
        let v = f(0.85).checked_mul(base.v0_cm3).expect("representable");
        let theta_0 = base.theta_0.kelvin().to_f64_lossy();
        let gamma_0 = base.gamma_0.to_f64_lossy();
        let l = (v.to_f64_lossy() / base.v0_cm3.to_f64_lossy()).ln();
        let limit = theta_0 * (-gamma_0 * l).exp();

        // Ascending in q, straddling zero. The departure goes as `-gamma_0 q L^2/2`, so at compression
        // (`L < 0`) theta FALLS as q rises.
        let qs = ["-0.001", "-0.00001", "0", "0.00001", "0.001"];
        let mut thetas: Vec<f64> = Vec::new();
        for q in qs {
            let a = MgdAnchors {
                q: Fixed::from_decimal_str(q).expect("representable"),
                ..base
            };
            thetas.push(
                theta_at(&a, v)
                    .unwrap_or_else(|| panic!("q = {q} is a model member and must evaluate"))
                    .to_f64_lossy(),
            );
        }
        for w in thetas.windows(2) {
            assert!(
                w[1] <= w[0],
                "theta must fall monotonically as q rises through zero at compression; read {thetas:?}"
            );
        }

        for (i, q) in qs.iter().enumerate() {
            let q: f64 = q.parse().expect("a decimal");
            let predicted = limit * (1.0 - gamma_0 * q * l * l / 2.0);
            // THE FLOOR IS THE REPRESENTATION'S, computed rather than assumed. Every branch pays the
            // `ln`/`exp` chain's own relative error, measured at 3e-8 on this state; the DIRECT branch
            // additionally amplifies one ulp of `1 - r^q` by `gamma_0/|q|`, and `powf` composes two series
            // so a few ulps enter rather than one. Ten of them is the allowance, and at `q = 1e-3` that is
            // still under a fifth of the departure being measured.
            let chain = 3e-8 * limit;
            let amplified = if q == 0.0 {
                0.0
            } else {
                10.0 * limit * gamma_0 * 2f64.powi(-32) / q.abs()
            };
            assert!(
                (thetas[i] - predicted).abs() <= chain + amplified,
                "at q = {q:e} the series gives {predicted:.6} K and the solver read {:.6} K, outside the \
                 {:.2e} K the representation allows",
                thetas[i],
                chain + amplified
            );
        }

        // AND THE SIGN IS RESOLVED AT THE ENDS, which the middle three cannot show: below the derived
        // crossover the q dependence is smaller than the representation carries, so those states resolve to
        // the limit itself and are equal to each other.
        assert!(
            thetas[0] > limit && thetas[4] < limit,
            "the two ends must straddle the limit {limit:.6} K; read {thetas:?}"
        );
        assert_eq!(
            (thetas[1], thetas[2]), (thetas[2], thetas[3]),
            "inside the crossover the limit branch answers, so these three are one value: {thetas:?}"
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

    /// Enstatite's anchors: the phase whose `q` of 7.8 puts its spinodal inside the searched span, which is
    /// what makes it the live case for both the fold solve and the band near the fold.
    fn enstatite() -> MgdAnchors {
        let anc = ThermoelasticAnchors::standard().expect("anchors");
        let row = anc.row("enstatite").expect("enstatite row");
        MgdAnchors {
            v0_cm3: f(125.35),
            k0_gpa: f(107.0),
            k0_prime: f(7.1),
            theta_0: row.theta_0.expect("theta_0"),
            gamma_0: f(0.67),
            q: row.q.expect("q"),
            atoms_per_formula_unit: 20,
        }
    }

    /// The located fold, at 64 and 256 bracketing steps.
    fn fold_at(a: &MgdAnchors, t_k: Fixed, steps: u32) -> Fixed {
        match expansion_edge(a, t_k, steps).expect("the stability search evaluates") {
            ExpansionEdge::Spinodal(v) => v,
            ExpansionEdge::StableThroughout(v) => panic!(
                "this phase turns inside the searched span; the search found no fold and reported stable \
                 through {}",
                v.to_f64_lossy()
            ),
        }
    }

    /// THE FOLD IS A SOLVE, NOT A WALK, and the step count is the test that says which.
    ///
    /// The walk this replaced compared SAMPLES, so it could not see a turn that happened inside one step
    /// and the located edge moved with the loop constant. Measured against the same walk restored for the
    /// comparison: 64 steps gave `143.957` and 256 steps gave `143.712`, a spread of `0.245`, against a
    /// true fold at `143.813`. The 64-step answer sits PAST the fold, so that bracket enclosed unstable
    /// volumes. Bracketing on the SIGN of `K_T` and bisecting returns `143.813188916` at both step counts,
    /// bit for bit.
    ///
    /// The tolerance is COMPUTED from the solver: the widest bracket the search can hand it, halved
    /// `BISECTION_STEPS` times, plus a few ulps of landing spread. The walk misses it by a factor of
    /// `6.5e7`.
    #[test]
    fn the_located_fold_is_step_count_independent() {
        let a = enstatite();
        let t = Fixed::from_int(1000);
        let coarse = fold_at(&a, t, 64).to_f64_lossy();
        let fine = fold_at(&a, t, 256).to_f64_lossy();

        let widest_bracket = a.v0_cm3.to_f64_lossy() / 2.0;
        let ulp = 2f64.powi(-32);
        // Two bisections started from different brackets converge on the same root and need not land on
        // the same ulp, so the allowance is the solver's own tolerance plus a few ulps of landing spread.
        let allowed = widest_bracket / 2f64.powi(BISECTION_STEPS as i32) + 16.0 * ulp;
        let walk_step = a.v0_cm3.to_f64_lossy() / 128.0;
        assert!(
            (coarse - fine).abs() <= allowed,
            "the located fold must not depend on the step count that bracketed it: 64 steps gave \
             {coarse:.9} and 256 gave {fine:.9}, a spread of {:.3e} against a solver tolerance of \
             {allowed:.3e}. The walk this replaced disagreed by about one walk step, {walk_step:.3}.",
            (coarse - fine).abs()
        );
    }

    /// THE BAND GROWS TOWARD THE FOLD, and it is the RELATIVE band that does.
    ///
    /// This is the second face of the step-count defect. `K_T = -V (dP/dV)` has `dP/dV -> 0` at the
    /// spinodal, so the quantity being measured collapses while the representation floor under the
    /// differencing does not move: that floor is `V * u / h` with `h = V/10_000`, which is `10_000 u`
    /// whatever `V` is. The relative band is the one guaranteed to blow up, and the assertion is on
    /// `band / K_T` for that reason. Measured on enstatite at 1000 K the relative band runs `1.11e-6` at
    /// `V_0` against `1.83e-3` at `0.999` of the fold, a factor of 1655; the absolute band over the same
    /// span moves only from `1.05e-4` to `1.11e-3` GPa, so a test written on the absolute band would rest
    /// on a factor of ten where this one rests on a factor of a thousand.
    ///
    /// That is the region the fold search operates in, which is why the two findings are one: a search that
    /// walks toward a vanishing derivative has no way to know when it has reached the noise, and now it
    /// carries the number that says.
    #[test]
    fn the_numerical_band_on_k_t_grows_toward_the_spinodal() {
        let a = enstatite();
        let t = Fixed::from_int(1000);
        let v_fold = fold_at(&a, t, 64);

        let at = |v: Fixed| {
            let k = bulk_modulus_with_band(&a, v, t)
                .expect("the modulus evaluates on the stable branch");
            (k.value.to_f64_lossy(), k.band.to_f64_lossy())
        };
        let (k_ref, band_ref) = at(a.v0_cm3);
        let near_fold = f(0.999).checked_mul(v_fold).expect("representable");
        let (k_near, band_near) = at(near_fold);

        let rel_ref = band_ref / k_ref;
        let rel_near = band_near / k_near;
        assert!(
            rel_near > 10.0 * rel_ref,
            "the relative numerical band must grow by at least an order of magnitude approaching the \
             fold: at V_0 it is {rel_ref:.3e} (K_T {k_ref:.4} GPa, band {band_ref:.3e}) and at 0.999 of \
             the fold it is {rel_near:.3e} (K_T {k_near:.4} GPa, band {band_near:.3e})"
        );

        // AND THE MODULUS STAYS RESOLVED ON THE STABLE BRANCH. `K_T` minus its own band must remain
        // positive everywhere the phase is a solid, or the rung would be reporting a stiffness it cannot
        // tell from zero.
        for fraction in ["0.9", "0.95", "0.99", "0.999"] {
            let v = Fixed::from_decimal_str(fraction)
                .expect("a decimal")
                .checked_mul(v_fold)
                .expect("representable");
            let (k, band) = at(v);
            assert!(
                k - band > 0.0,
                "at V = {:.4} ({fraction} of the fold) K_T is {k:.6} GPa against a band of {band:.3e}, so \
                 the stable branch is no longer resolved from zero",
                v.to_f64_lossy()
            );
        }
    }

    /// THE HEAT CAPACITY SURVIVES THE FUNCTION BOUNDARY, checked against an independent evaluation of the
    /// Debye capacity rather than against itself.
    ///
    /// `C_V` was computed on the way to `alpha` and then dropped, so the one consumer that needed a heat
    /// capacity used Dulong-Petit `3nR` instead. The twin here is an f64 quadrature of
    /// `C_V = 9 n R (T/theta)^3 Integral_0^(theta/T) x^4 e^x/(e^x - 1)^2 dx`, which shares no code and no
    /// representation with the central difference under test, so agreement is evidence rather than
    /// tautology. At the reference state `theta` is exactly `theta_0`, which is what makes the twin
    /// evaluable without the solver handing over its internal state.
    ///
    /// The second half is the finding: Dulong-Petit overstates by 39 percent at 300 K and by very little at
    /// 1600 K. A consumer calibrated at mantle temperature therefore sees agreement, and that agreement is
    /// CANCELLATION rather than physics, which is the defect this module's own header records.
    #[test]
    fn the_debye_heat_capacity_is_carried_out_and_departs_from_dulong_petit() {
        /// `9 n R (T/theta)^3 Integral_0^(theta/T) x^4 e^x/(e^x-1)^2 dx`, in f64 at fine resolution.
        fn twin(t: f64, theta: f64, atoms: f64) -> f64 {
            let r = 8.314_462_618_153_24_f64;
            let y = theta / t;
            let n = 20_000;
            let h = y / n as f64;
            let f = |x: f64| {
                if x <= 1e-9 {
                    return x * x; // x^4 e^x/(e^x-1)^2 -> x^2 as x -> 0
                }
                let e = (-x).exp();
                x * x * x * x * e / ((1.0 - e) * (1.0 - e))
            };
            let mut acc = f(0.0) + f(y);
            for i in 1..n {
                acc += if i % 2 == 1 {
                    4.0 * f(h * i as f64)
                } else {
                    2.0 * f(h * i as f64)
                };
            }
            9.0 * atoms * r * (t / theta).powi(3) * acc * h / 3.0
        }

        let a = forsterite();
        let atoms = f64::from(a.atoms_per_formula_unit);
        let r = civsim_physics::gas_thermochemistry::molar_gas_constant()
            .expect("R")
            .to_f64_lossy();
        let dulong_petit = 3.0 * atoms * r;

        // AT THE REFERENCE STATE theta is theta_0, so the twin needs nothing the solver keeps to itself.
        let theta_0 = a.theta_0.kelvin().to_f64_lossy();
        let ambient = response_at(&a, f(0.0001), Fixed::from_int(300)).expect("ambient answers");
        let c_v = ambient.c_v_j_per_mol_k.to_f64_lossy();
        let want = twin(300.0, theta_0, atoms);
        assert!(
            (c_v - want).abs() / want < 5e-3,
            "the carried C_V must reproduce an independent Debye evaluation: read {c_v:.4} against \
             {want:.4} J/mol/K at theta_0 = {theta_0:.0} K"
        );
        // And it must arrive with a band that resolves it.
        assert!(
            ambient.c_v_band_j_per_mol_k.to_f64_lossy() < c_v / 100.0,
            "a capacity whose numerical band is a large fraction of itself is not a measurement: {} \
             against {c_v:.4}",
            ambient.c_v_band_j_per_mol_k.to_f64_lossy()
        );

        // THE DEBYE CEILING, which no quantum capacity may cross.
        assert!(
            c_v < dulong_petit,
            "a Debye C_V approaches 3nR = {dulong_petit:.2} from BELOW and never exceeds it; read {c_v:.2}"
        );

        // THE CANCELLATION, measured at both ends of the span the interior column spans.
        let hot = response_at(&a, f(0.0001), Fixed::from_int(1600)).expect("1600 K answers");
        let c_hot = hot.c_v_j_per_mol_k.to_f64_lossy();
        let cold_overstatement = dulong_petit / c_v - 1.0;
        let hot_overstatement = dulong_petit / c_hot - 1.0;
        assert!(
            cold_overstatement > 0.25,
            "at 300 K Dulong-Petit overstates the Debye capacity by about 39 percent; measured \
             {:.1} percent",
            cold_overstatement * 100.0
        );
        assert!(
            hot_overstatement < 0.05,
            "and by 1600 K it has closed to a few percent, which is why a consumer calibrated at mantle \
             temperature saw agreement; measured {:.1} percent",
            hot_overstatement * 100.0
        );
        assert!(
            c_hot > c_v,
            "and the capacity rises with temperature toward the classical limit: {c_v:.2} at 300 K \
             against {c_hot:.2} at 1600 K"
        );
    }

    /// TWO PUBLISHED INVERSIONS, TWO BRANCHES, and the gap between them is the finding this assembly used
    /// to discard.
    ///
    /// Enstatite's `q` is 7.8 in the 2005 fit and 3.4 in the 2011 one, with `gamma_0` moving 0.67 to 0.78,
    /// and the row's own citation calls it the largest disagreement in the column. `q` enters two
    /// exponentials (`gamma = gamma_0 (V/V_0)^q` and `theta = theta_0 exp[(gamma_0/q)(1 - (V/V_0)^q)]`), so
    /// the disagreement does not stay small: at moderate mantle compression the two give thermal pressures
    /// differing by more than a factor of two. The engine returned one of them, as a point, with no delta.
    ///
    /// The band each branch is compared against is computed from the family's OWN stated `q` uncertainty
    /// through `gamma = gamma_0 (V/V_0)^q`, whose logarithmic sensitivity to `q` is exactly `ln(V/V_0)`.
    /// That is the `gamma` term only and therefore UNDERSTATES the full input band, which makes the
    /// conclusion stronger rather than weaker: the branches separate by far more than even a band that
    /// leaves out the `theta` and `gamma_0` contributions.
    #[test]
    fn enstatite_returns_two_branches_whose_thermal_pressures_differ_by_more_than_either_band() {
        let anc = ThermoelasticAnchors::standard().expect("anchors");
        let gr = civsim_physics::gruneisen::GruneisenTable::standard().expect("gruneisen");
        let (families, excluded) = MgdAnchors::families("enstatite", &gr, &anc);
        assert_eq!(
            families.len(),
            2,
            "the column banks two inversions for enstatite and they disagree, so both must survive \
             assembly as separate coherent sets; excluded {excluded:?}"
        );
        assert!(
            families[0].family == "slb2005" && families[1].family == "slb2011",
            "primary first, in the order the row declares: {:?}",
            families.iter().map(|f| &f.family).collect::<Vec<_>>()
        );
        // Neither branch borrows the other's gamma_0: the primary reads the bank, the successor its own.
        assert_ne!(
            families[0].anchors.gamma_0, families[1].anchors.gamma_0,
            "the two inversions moved gamma_0 as well as q, and each branch must carry its own"
        );

        let t = Fixed::from_int(1000);
        let compression = f(0.85);
        let mut measured: Vec<(String, f64, f64)> = Vec::new();
        for fam in &families {
            let v = compression
                .checked_mul(fam.anchors.v0_cm3)
                .expect("representable");
            let p = thermal_pressure_gpa(&fam.anchors, v, t)
                .unwrap_or_else(|| panic!("{} must evaluate a thermal pressure", fam.family))
                .to_f64_lossy();
            // d(ln gamma)/dq = ln(V/V_0), so the q term of the relative band is |ln(V/V_0)| * q_band.
            let ln_ratio = (0.85_f64).ln();
            let band = fam
                .q_band
                .map(|b| ln_ratio.abs() * b.to_f64_lossy())
                .expect("each family states its own q band");
            measured.push((fam.family.clone(), p, band));
        }

        let (lo, hi) = if measured[0].1 < measured[1].1 {
            (&measured[0], &measured[1])
        } else {
            (&measured[1], &measured[0])
        };
        let ratio = hi.1 / lo.1;
        assert!(
            ratio > 1.5,
            "at V/V_0 = 0.85 the two inversions must separate by more than half again: {} gives \
             {:.4} GPa and {} gives {:.4} GPa, ratio {ratio:.3}",
            lo.0,
            lo.1,
            hi.0,
            hi.1
        );
        // The separation must exceed EITHER branch's own band, or the disagreement would be inside the
        // uncertainty and there would be nothing to report.
        let separation = hi.1 - lo.1;
        for (family, p, band) in &measured {
            assert!(
                separation > p * band,
                "the branches separate by {separation:.4} GPa, which must exceed {family}'s own band of \
                 {:.4} GPa ({:.1} percent of {p:.4}); otherwise the two inversions agree within \
                 uncertainty and the ensemble would be ceremony",
                p * band,
                band * 100.0
            );
        }
    }

    /// AND THE ENSEMBLE IS ABSENT WHERE THE DATA HOLDS NO DISAGREEMENT.
    ///
    /// Corundum is reproduced cell for cell by the successor inversion: same `V_0`, `K_0`, `K_0'`,
    /// `theta_0`, `gamma_0` and `q`. Two byte-identical anchor sets are one answer heard twice, so they
    /// collapse to one branch that names both families rather than presenting a spread of zero as though it
    /// were a measurement of agreement. Without this the mechanism would be ceremony on every row, and the
    /// enstatite test alone could not tell the difference.
    #[test]
    fn corundum_returns_one_branch_because_the_inversions_agree() {
        let anc = ThermoelasticAnchors::standard().expect("anchors");
        let gr = civsim_physics::gruneisen::GruneisenTable::standard().expect("gruneisen");

        let (families, excluded) = MgdAnchors::families("corundum", &gr, &anc);
        assert_eq!(
            families.len(),
            1,
            "corundum's two channels are identical, so they are one determination: {families:?} / \
             excluded {excluded:?}"
        );
        assert_eq!(
            families[0].concurring,
            vec!["slb2011".to_string()],
            "and the concurring family is NAMED, so 'one branch' is distinguishable from 'only one \
             channel was read', which is the mistake this whole item is about"
        );
        // The row does carry a second channel; it collapsed on content rather than being skipped.
        let row = anc.row("corundum").expect("corundum row");
        assert_eq!(
            row.channels.len(),
            2,
            "the loader must READ both channels before deciding they agree"
        );

        // The three rows the successor reproduces exactly collapse; the two it revises do not.
        for (phase, want) in [
            ("corundum", 1usize),
            ("forsterite", 1),
            ("fayalite", 1),
            ("spinel", 2),
            ("periclase", 2),
            ("enstatite", 2),
        ] {
            let (fams, _) = MgdAnchors::families(phase, &gr, &anc);
            assert_eq!(
                fams.len(),
                want,
                "{phase}: the branch count must follow the DATA, not a policy of always one or always two"
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

    #[test]
    fn the_debye_integral_twins_a_high_precision_evaluation_across_the_whole_range() {
        // THE NUMERICAL TWIN the review asked for, and the reason it is worth having: the integrand used the
        // direct `x^3/(e^x - 1)` form, which was not merely imprecise past `Fixed::exp`'s rail near 22 but
        // RISING where the physics falls, because the saturated denominator turned the tail into `x^3/MAX`.
        // An algebraic check would not have caught that; only comparing against an independent evaluation
        // does. The twin is an f64 Simpson integration at far finer resolution, which shares no code and no
        // representation with the kernel under test.
        fn twin(y: f64) -> f64 {
            // x^3 e^-x / (1 - e^-x), the same identity, evaluated in f64 at 20000 intervals.
            let n = 20_000;
            let h = y / n as f64;
            let f = |x: f64| {
                if x <= 1e-12 {
                    return x * x; // the removable singularity's leading term
                }
                let e = (-x).exp();
                x * x * x * e / (1.0 - e)
            };
            let mut acc = f(0.0) + f(y);
            for i in 1..n {
                let x = h * i as f64;
                acc += if i % 2 == 1 { 4.0 * f(x) } else { 2.0 * f(x) };
            }
            acc * h / 3.0
        }

        // Across the whole `theta/T` span the solver meets, INCLUDING the region past the old rail.
        for y_int in [1i32, 2, 5, 8, 12, 16, 20, 22, 25, 28, 30] {
            let got = debye_integral(Fixed::from_int(y_int))
                .unwrap_or_else(|| panic!("the integral must evaluate at y = {y_int}"))
                .to_f64_lossy();
            let want = twin(f64::from(y_int));
            let rel = (got - want).abs() / want;
            assert!(
                rel < 2e-3,
                "y = {y_int}: fixed-point {got} against high-precision {want}, relative {rel:.3e}"
            );
        }

        // AND IT CONVERGES ON THE ANALYTIC INFINITE LIMIT, `pi^4 / 15`, which is the one value that needs no
        // twin at all because it is exact.
        let infinite = debye_integral(Fixed::from_int(30))
            .expect("y = 30 evaluates")
            .to_f64_lossy();
        let exact = std::f64::consts::PI.powi(4) / 15.0;
        assert!(
            (infinite - exact).abs() / exact < 2e-3,
            "the clamped upper limit reproduces pi^4/15: {infinite} against {exact}"
        );
    }
}
