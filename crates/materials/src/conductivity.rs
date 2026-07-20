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

//! THE THERMAL-CONDUCTIVITY LADDER: one quantity, a measured rung and an estimated rung, one entry point.
//!
//! WHY A LADDER AND NOT A CHOICE. Two models of `k` exist and they are NOT competitors, they are RUNGS of the
//! lookup order this engine already runs for every other quantity: measured before estimator, dispatched per
//! material on ANCHOR AVAILABILITY. Nobody at a call site ever picks a physical model.
//!
//! - TOP RUNG, [`hofmeister_lattice_conductivity`]: a MEASURED `kappa_298` anchor carrying derived TEMPERATURE
//!   dependence off the banked Grueneisen parameter and the caller's expansivity integral. NO PRESSURE TERM
//!   AND NO BULK MODULUS ENTER IT, stated here because this line once claimed both: the form is a temperature
//!   form, valid in the AMBIENT frame its anchors were measured in, and [`assemblage_conductivity_at`] refuses
//!   outside that frame rather than extrapolating. Highest accuracy, available only where a mineral HAS a
//!   measured anchor.
//! - ESTIMATOR RUNG, [`crate::properties::lattice_thermal_conductivity_w_per_m_k`] (Slack): no anchor needed,
//!   evaluable for anything with banked columns, carrying the band its own docstring declares (roughly 3x
//!   symmetric on simple cells, ONE-SIDED on complex cells, where it is an intrinsic UPPER BOUND that can sit
//!   several-fold above truth; rutile is its own convicting exhibit at ~43 against a measured ~9). That band
//!   is a TYPED quantity here ([`EstimatorBand`]), because "no band supplied" and "no uncertainty exists" are
//!   different claims and an `Option` spelled them the same way.
//!
//! WHERE NO MEASUREMENT EXISTS, Slack's magnitude serves as the `[E]`-grade anchor with its one-sided
//! upper-bound band declared, and Hofmeister's class-keyed exponent governs the temperature shape ON BOTH RUNGS,
//! because the exponent split IS the same physics as the validity split: Slack's `a = 1` matches ice at `612/T`
//! and nearly matches MgO at `0.9`, while complex silicates take `0.33`.
//!
//! THE DOCTRINE THIS INSTANTIATES (standing, and it will recur): SAME-RUNG DUPLICATES are the
//! redundant-parameter defect at MODEL level and stay forbidden. DIFFERENT-RUNG models with a DECLARED ORDER are
//! the ladder. And the ladder carries a free integrity mechanism: WHEREVER BOTH RUNGS CAN EVALUATE, THE
//! DISAGREEMENT IS COMPUTED AND LOGGED AS A DIAGNOSTIC, NEVER SILENTLY RESOLVED
//! ([`rung_disagreement_ratio`]). MgO-class minerals are permanent OVERLAP SENTINELS, two models compared BY
//! CONSTRUCTION on every run, which turns "never compared" from a risk into an impossibility. That is the whole
//! point: the defect this module was built to avoid was two models answering one question in different call
//! sites, never compared, disagreeing several-fold, which is the k/kappa finding in a bigger coat.
//!
//! WHY THIS PAYS FOR THE ALIEN. The geotherm's minerals have measured anchors, so the top rung serves the front
//! lane. But exotic condensates (the carbide slice) will have NO `kappa_298` rows at all, and SLACK'S RUNG IS
//! THE ONLY LEGAL CONDUCTIVITY PATH AN ALIEN PHASE WILL EVER HAVE: banked columns in, banded estimate out,
//! upper-bound honesty attached. Hofmeister bolted BESIDE Slack would have served Earth minerals and stranded
//! every alien one.
//!
//! HOW THIS MODULE CAME TO EXIST, recorded because the rule it completed binds every future ruling: a ruling
//! ordered Hofmeister built as new machinery. A check for an existing conductivity found Slack already banked,
//! from the same estimator roster the ruling channel had itself written down. So the premise line COMPLETES
//! SYMMETRICALLY: existence claims and ABSENCE claims are one class. A ruling that says "wire X" verifies
//! PRESENCE; a ruling that says "BUILD X" verifies ABSENCE. One line either way. Checking for the thing before
//! building a second one is the named standing practice, and the first time it ran in the build direction it
//! prevented a ~5x silent disagreement from shipping.

use std::fmt;

use crate::properties::lattice_thermal_conductivity_w_per_m_k;
use civsim_core::Fixed;
use civsim_physics::crystal_field::{iron_valence_state, IronValence};
use civsim_physics::gruneisen::GruneisenTable;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_physics::phase_conductivity::PhaseConductivityTable;

const ZERO: Fixed = Fixed::ZERO;

/// The reference temperature Hofmeister's lattice form is anchored at: 298 K, the standard state the measured
/// `kappa_298` rows are reported against. It is the SOURCE'S OWN reference, not a chosen scale.
pub fn hofmeister_reference_temperature_k() -> Fixed {
    Fixed::from_int(298)
}

/// The PRESSURE the lattice form is anchored at: 1 bar, the frame every cited `kappa_298` row in
/// `crates/physics/data/phase_conductivity.toml` was measured in, and the frame the banked Grueneisen rows
/// declare for themselves (`pressure_bar = "1"`). It is the SOURCES' own reference rather than a chosen datum.
///
/// This exists because the lattice form has no pressure term at all. A quantity with no pressure dependence is
/// valid at exactly one pressure, and naming which one is what lets [`assemblage_conductivity_at`] refuse
/// outside it instead of a comment declaring the limit while the code answers anyway.
pub fn hofmeister_reference_pressure_bar() -> Fixed {
    Fixed::ONE
}

/// How far from [`hofmeister_reference_pressure_bar`] the ambient ladder may be read before it refuses.
///
/// ZERO, and zero is the absence of a claim rather than a chosen tolerance. The form carries no pressure
/// dependence, so any nonzero slack would be an assertion about how far the neglected term stays tolerable, and
/// this module has no basis for such an assertion: the term Hofmeister's fuller form carries is
/// `(1 + (K_0'/K_0) P)`, and neither `K_0'` nor `K_0` is an input to the lattice function.
///
/// RESERVED, with its basis, as `conductivity.ambient_frame_pressure_slack_bar` in `calibration/reserved.toml`:
/// the owner sets it at the pressure where the omitted pressure term moves the aggregate by more than the
/// aggregate's own reported band ([`AssemblageConductivity::band_up`] and `band_down`), which is a DERIVABLE
/// criterion once `K_0'` is banked beside `K_0` for the census phases and stops being an owner decision at all.
pub fn ambient_frame_pressure_slack_bar() -> Fixed {
    ZERO
}

/// WHERE THE CITED CALIBRATION PLACES A CELL COUNT, and with what number, so the two DIFFERENT reasons this
/// classifier can decline to hand back an exponent stay distinguishable at the call site. A gap in the cited
/// set and a value the owner has not set yet are both refusals and they need different fetches to close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExponentClass {
    /// The COMPLEX-CELL class, `n >= 6`, carrying Hofmeister's cited silicate exponent `a = 0.33`.
    Complex(Fixed),
    /// The SIMPLE-LATTICE class, `n <= 2`, carrying NO number. The cited set brackets this class with two
    /// determinations that disagree (ice at `612/T`, so `a ~ 1`, and MgO at `a = 0.9`) and pins no single
    /// value between them, so the class exponent is RESERVED for the owner
    /// (`conductivity.simple_class_temperature_exponent` in `calibration/reserved.toml`) rather than
    /// interpolated here. A phase in this class evaluates on its OWN cited exponent
    /// ([`PhaseConductivity::measured_exponent_a`]) or it refuses.
    SimpleReserved,
    /// `2 < n < 6`: the cited set places nothing here at all. A DATA gap, closed by a determination in that
    /// range, never by a number chosen inside it.
    Unplaced,
    /// `n < 1`: a cell with no atoms is not a lattice.
    NotALattice,
}

/// The class-keyed temperature exponent `a` in Hofmeister's lattice form `kappa ~ (298/T)^a`, keyed on
/// ATOMS PER PRIMITIVE CELL, the class variable already banked and already in Slack's own signature.
///
/// THE PHYSICS OF THE SPLIT: a simple lattice's phonons scatter through few channels and the conductivity falls
/// as roughly `1/T` (the Umklapp limit). A complex cell has many optical branches that carry little heat but
/// scatter plenty, so the decline is far shallower. That is the SAME physics as Slack's validity split: Slack's
/// single-scattering form is built for the simple case, which is why it lands within its band on simple cells
/// and OVERSTATES complex ones.
///
/// THE CALIBRATION SET, and its honest limit. Three determinations are cited: ice at `612/T` (`a ~ 1`), MgO at
/// `a = 0.9`, and complex silicates at `a = 0.33` (Hofmeister). The CELL-COUNT boundary is calibrated on the
/// cited set Slack's own docstring convicts itself with: diamond, NaCl, and MgO (all `n = 2`) land inside its
/// band, while rutile (`n = 6`) is overstated ~5x.
///
/// SO THE COMPLEX CLASS CARRIES A CITED NUMBER AND THE SIMPLE CLASS DOES NOT, and this classifier now says so.
/// One determination pins `a = 0.33` for `n >= 6`. TWO determinations bracket `n <= 2` and disagree, at `1` and
/// `0.9`, and the class value between them is nobody's measurement. This branch returned `0.95`, the arithmetic
/// midpoint of the two, which was an unattributed interpolation sitting inside a classifier: the silent-scalar
/// class the adjacent `2 < n < 6` refusal exists to forbid, authored at the same site that refuses its
/// neighbour. The width it hid is real rather than cosmetic: across the bracketing pair the conductivity at
/// 1600 K moves by roughly 18 percent, reported as a point value with no band.
///
/// SO `n <= 2` REFUSES TOO, until the owner sets `conductivity.simple_class_temperature_exponent`. A phase in
/// that class can still evaluate by carrying its own cited exponent
/// ([`PhaseConductivity::measured_exponent_a`]), which is the per-phase route the refusal text has always told
/// callers to take and which nothing implemented until now. MgO is the live case: its `a = 0.9` is a cited
/// per-phase determination, so a fetched per-phase exponent column restores the ladder's overlap sentinel
/// without anyone choosing a class number.
pub fn lattice_exponent_class(atoms_per_primitive_cell: i32) -> ExponentClass {
    if atoms_per_primitive_cell < 1 {
        return ExponentClass::NotALattice;
    }
    if atoms_per_primitive_cell <= 2 {
        // The simple-lattice limit, where the Umklapp `1/T` sits. Ice (612/T) and MgO (0.9) both land here
        // and they do not agree, so the class carries no number of its own.
        return ExponentClass::SimpleReserved;
    }
    if atoms_per_primitive_cell >= 6 {
        // The complex-cell class: Hofmeister's cited silicate exponent, the one number the set does pin.
        return ExponentClass::Complex(Fixed::from_ratio(33, 100));
    }
    // 2 < n < 6: the cited set places nothing here. Refuse rather than author a boundary.
    ExponentClass::Unplaced
}

/// The class exponent where the cited set pins one, and `None` everywhere it does not. A thin read of
/// [`lattice_exponent_class`], kept because a caller that only needs the number should not have to match the
/// class; a caller that needs to know WHY it got nothing matches the class instead.
pub fn lattice_exponent_for_cell(atoms_per_primitive_cell: i32) -> Option<Fixed> {
    match lattice_exponent_class(atoms_per_primitive_cell) {
        ExponentClass::Complex(a) => Some(a),
        ExponentClass::SimpleReserved | ExponentClass::Unplaced | ExponentClass::NotALattice => {
            None
        }
    }
}

/// HOFMEISTER'S LATTICE CONDUCTIVITY at a temperature (W/(m*K)), the TOP RUNG:
///
/// `kappa_lat(T) = kappa_298 * (298/T)^a * exp[-(4*gamma + 1/3) * integral(alpha d theta)]`
///
/// The measured `kappa_298` sets the MAGNITUDE; everything that moves it with temperature is DERIVED. The
/// power-law factor is the phonon-scattering decline with its class-keyed exponent
/// ([`lattice_exponent_for_cell`]); the exponential is the thermal-expansion correction, where `gamma` is the
/// banked Grueneisen parameter and the integral is expansivity accumulated from the reference temperature to
/// `T`. The `4` and the `1/3` are the form's own coefficients, not knobs.
///
/// THE EXPANSIVITY INTEGRAL is the caller's, because only the caller knows whether its expansivity is constant
/// over the range: `integral(alpha d theta)` from 298 to `T`. For a constant `alpha` that is `alpha * (T - 298)`,
/// which is the common case for a lid-temperature span; a caller with a temperature-dependent `alpha` integrates
/// its own and passes the result. Passing a bare `alpha` here would author the constancy assumption invisibly.
///
/// THE FRAME IS AMBIENT AND THE SIGNATURE IS WHERE THAT IS VISIBLE: no pressure enters this function and no
/// bulk modulus does either. The form is `kappa(T)` at the pressure its `kappa_298` anchor was measured at,
/// which for every row in the cited column is 1 bar. Two comments in this module used to describe it as
/// `k(T,P)` deriving off a banked bulk modulus, and the word `bulk` appeared in the module exactly twice, both
/// times in a comment and neither time in a code path. The pressure frame is now carried on the aggregate
/// ([`AssemblageConductivity::frame_pressure_bar`]) and checked by [`assemblage_conductivity_at`], so a caller
/// at depth is refused rather than described as in-frame by a docstring.
///
/// `None` on a non-positive temperature or anchor, or a fixed-point overflow. Deterministic fixed-point.
// @derives: lattice thermal conductivity k(T) at the anchor's own ambient pressure frame <- a measured kappa_298 anchor + the banked Grueneisen parameter + the caller's expansivity integral (measured rung)
pub fn hofmeister_lattice_conductivity(
    kappa_298: Fixed,
    exponent_a: Fixed,
    gruneisen: Fixed,
    expansivity_integral: Fixed,
    temperature: Fixed,
) -> Option<Fixed> {
    if temperature <= ZERO || kappa_298 <= ZERO {
        return None;
    }
    // The power-law decline (298/T)^a.
    let ratio = hofmeister_reference_temperature_k().checked_div(temperature)?;
    if ratio <= ZERO {
        return None;
    }
    let decline = ratio.powf(exponent_a);
    // The expansion correction exp[-(4 gamma + 1/3) * integral(alpha d theta)].
    let four_gamma = gruneisen.checked_mul(Fixed::from_int(4))?;
    let coefficient = four_gamma.checked_add(Fixed::ONE.checked_div(Fixed::from_int(3))?)?;
    let exponent = ZERO.checked_sub(coefficient.checked_mul(expansivity_integral)?)?;
    let correction = exponent.exp();
    kappa_298.checked_mul(decline)?.checked_mul(correction)
}

/// THE DECLARED SCOPE of the radiative polynomial, carried on the aggregate so a consumer reads what the fit
/// was fitted TO rather than inferring a universal law from a boolean gate.
///
/// WHY THIS TYPE EXISTS (Principle 7, admit the alien). The gate on the radiative term is
/// [`PhaseConductivity::bears_ferrous_iron`], which is DERIVED cleanly by charge balance over the phase's own
/// composition, so the gate itself keys on per-phase data and admits any chemistry. The POLYNOMIAL behind the
/// gate does not: its coefficients are a fit to TERRAN SILICATES, and a clean gate in front of a parochial fit
/// applies that fit to every Fe2+-bearing phase in every world, including phases with no silicate analogue at
/// all. The exposure is REPORTED rather than removed, because removing it would leave the hot end with no
/// radiative channel and reporting it lets a consumer weigh it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadiativeFitScope {
    /// No phase in the census took the radiative term, so no scoped fit entered the answer.
    NotApplied,
    /// At least one phase took it. The coefficients are Hofmeister's fit to Fe2+-bearing TERRAN SILICATES and
    /// they carry the type-II dispute declared on [`radiative_conductivity_w_per_m_k`]. Read
    /// [`AssemblageConductivity::radiative_weight_fraction`] for how much of the census inherited it.
    TerranFerrousSilicateFit,
}

/// THE RADIATIVE conductivity (W/(m*K)) of an Fe2+-bearing phase, Hofmeister's polynomial:
///
/// `kappa_rad(T) = 0.0175 - 1.037e-4 T + 2.245e-7 T^2 - 3.407e-11 T^3`
///
/// Photons carry heat through a semi-transparent solid, and the Fe2+ absorption bands set how far they travel.
/// It matters only at the HOT end: the term is small and rises steeply with temperature, so it is a deep-mantle
/// quantity, and a caller adds it to the lattice term only for a phase that carries Fe2+.
///
/// THE FIT IS SCOPED, NOT UNIVERSAL, and the scope travels with the answer as [`RadiativeFitScope`]. These
/// coefficients were fitted to Fe2+-bearing TERRAN SILICATES. The `bears_ferrous_iron` gate that admits a phase
/// to this term is derived by charge balance and so is chemistry-blind by construction, which means the gate
/// will hand this silicate fit an alien Fe2+ carbide, sulfide or oxide the fit never saw. That is recorded as a
/// declared scope rather than corrected here, because correcting it needs an absorption-coefficient route keyed
/// on the phase's own optical data (the `optical_constants` column is the substrate that would carry it), which
/// is a fetch rather than a rewrite.
///
/// THE DECLARED DISPUTE (type-II, and it ships with the row): modern high-pressure experiments find radiative
/// transport contributing around 40 percent of olivine's conductivity at depth, against this small,
/// pressure-independent polynomial, and the field itself calls the radiative role controversial. THE BLAST
/// RADIUS IS SCOPED PLAINLY rather than waved away: radiative transport GROWS WITH TEMPERATURE and the elastic
/// lid is the COLD END of the column, so `T_e` is only weakly exposed to this band, and the disagreement lands
/// mostly on deep-mantle and slab-thermal consumers. A consumer at depth inherits the band; a lid consumer does
/// not, and neither should pretend the other's exposure.
///
/// Returns zero below the temperature where the polynomial goes non-positive (its fit does not extend to the
/// cold end, where the physical answer is that radiative transport is negligible anyway), so a cold caller reads
/// the honest zero rather than a negative conductivity.
// @derives: the radiative conductivity an Fe2+-bearing phase gains at high T, on a fit SCOPED to Terran silicates <- temperature
pub fn radiative_conductivity_w_per_m_k(temperature: Fixed) -> Fixed {
    if temperature <= ZERO {
        return ZERO;
    }
    let t = temperature;
    let c0 = Fixed::from_ratio(175, 10_000);
    let c1 = Fixed::from_ratio(1_037, 10_000_000);
    let c2 = Fixed::from_ratio(2_245, 10_000_000_000);
    // The cubic coefficient 3.407e-11 is below the Q32.32 resolution (~2.3e-10), so it is applied to the
    // SCALED temperature (T/1000)^3 with the paired 1e9 folded in: 3.407e-11 * T^3 = 3.407e-2 * (T/1000)^3.
    let c3_scaled = Fixed::from_ratio(3_407, 100_000);
    let t2 = match t.checked_mul(t) {
        Some(v) => v,
        None => return ZERO,
    };
    let t_k = match t.checked_div(Fixed::from_int(1000)) {
        Some(v) => v,
        None => return ZERO,
    };
    let t_k3 = match t_k.checked_mul(t_k).and_then(|v| v.checked_mul(t_k)) {
        Some(v) => v,
        None => return ZERO,
    };
    let linear = match c1.checked_mul(t) {
        Some(v) => v,
        None => return ZERO,
    };
    let quad = match c2.checked_mul(t2) {
        Some(v) => v,
        None => return ZERO,
    };
    let cubic = match c3_scaled.checked_mul(t_k3) {
        Some(v) => v,
        None => return ZERO,
    };
    let total = c0 - linear + quad - cubic;
    if total <= ZERO {
        ZERO
    } else {
        total
    }
}

/// THE OVERLAP SENTINEL: the ratio between the two rungs where BOTH can evaluate, `estimator / measured`.
///
/// This is the ladder's integrity mechanism, and it is the reason the ladder is safer than either model alone.
/// Wherever a mineral has BOTH a measured anchor and banked columns, the two rungs are computed and their
/// DISAGREEMENT IS REPORTED, never silently resolved. A ratio near one says the estimator is honest for that
/// class; a ratio far from one is a finding, and on a complex cell it should be ABOVE one (Slack's overstatement
/// is one-sided, the intrinsic upper bound its docstring declares).
///
/// MgO-class minerals are PERMANENT SENTINELS: they sit in both rungs' domains by construction, so every run
/// compares the two models against each other. That converts "two models, never compared" from a standing risk
/// into an impossibility, which is exactly the failure this module was built to foreclose.
///
/// `None` when either rung fails to evaluate (no comparison exists, so none is reported). Diagnostic only: no
/// caller resolves a conductivity from this, it exists to be watched.
pub fn rung_disagreement_ratio(measured_rung: Fixed, estimator_rung: Fixed) -> Option<Fixed> {
    if measured_rung <= ZERO || estimator_rung <= ZERO {
        return None;
    }
    estimator_rung.checked_div(measured_rung)
}

/// The ESTIMATOR RUNG's anchor: Slack's derived magnitude at the reference temperature, for a phase with NO
/// measured `kappa_298`. This is where option "Slack anchors Hofmeister" survives, and ONLY here: when no
/// measurement exists, Slack supplies the magnitude at 298 K and Hofmeister's class-keyed exponent carries it
/// through temperature.
///
/// THE BAND IS ONE-SIDED AND IT RIDES ALONG: on a complex cell this anchor is an INTRINSIC UPPER BOUND that can
/// sit several-fold above truth (rutile, ~43 against a measured ~9). A consumer of this rung inherits that
/// one-sidedness and must not report it as a symmetric uncertainty. On a simple cell the band is Slack's
/// declared ~3x, roughly symmetric. [`EstimatorBand`] is the type that carries which of those two shapes a
/// caller is declaring, and the aggregate refuses a rung that declares neither.
///
/// `None` when Slack cannot evaluate. Deterministic fixed-point.
pub fn estimator_anchor_298(
    gruneisen: Fixed,
    mean_atomic_mass_amu: Fixed,
    debye_temperature_k: Fixed,
    atomic_volume_angstrom3: Fixed,
    atoms_per_primitive_cell: i32,
) -> Option<Fixed> {
    let k = lattice_thermal_conductivity_w_per_m_k(
        gruneisen,
        mean_atomic_mass_amu,
        debye_temperature_k,
        atomic_volume_angstrom3,
        atoms_per_primitive_cell,
        hofmeister_reference_temperature_k(),
    );
    if k <= ZERO {
        None
    } else {
        Some(k)
    }
}

// ===== THE ROCK-LEVEL AGGREGATE: a census of phases in, one effective conductivity out =====
//
// WHY A MIXING RULE IS NOT A WEIGHTED MEAN, and why the choice was fetched rather than picked. The per-phase
// ladder above answers "what is THIS mineral's conductivity at this temperature". A rock is a mixture, and the
// effective conductivity of a mixture is NOT the volume-weighted mean of its parts: heat routes preferentially
// through the conductive phase, so the arithmetic (parallel, Voigt) mean is a rigorous UPPER bound and the
// harmonic (series, Reuss) mean a rigorous LOWER bound, and the truth sits between them at a position set by the
// GEOMETRY of the mixture. On the censuses this engine will hand it, that bracket is not a rounding detail: a
// six-phase chondritic mixture spans arithmetic 6.45 against harmonic 4.07 W/(m*K), a 49 percent spread, so the
// rule choice moves the answer by tens of percent and then moves lid thickness with it.
//
// WHAT THE LITERATURE SAYS ON THE EVIDENCE, and it is not unanimous, so both sides are recorded here rather than one.
// The GEOTHERMICS field's practical estimate is the GEOMETRIC mean: Fuchs, Schuetz, Foerster and Foerster (2013,
// Geothermics 47, 40-52) score five models against 1147 laboratory measurements and rank the geometric mean first
// (R^2 = 0.62) ahead of arithmetic (0.37), effective-medium (< 0.24), Hashin-Shtrikman (0.23) and harmonic
// (< 0.01), while calling its correspondence "not satisfying"; and that paper's own introduction records earlier
// studies in which the harmonic and arithmetic means beat the geometric one. THAT RESULT DOES NOT TRANSFER HERE,
// and the reason is scope: its two components are rock matrix and PORE FLUID, not two minerals, so it is scored
// on a problem whose geometry is pore space rather than a random grain packing.
//
// THE SOURCE THAT ANSWERS OUR QUESTION solves the heat conduction equation NUMERICALLY on a random multi-mineral
// SOLID mixture and scores the analytic closures against that ground truth: Henke, Gail and Trieloff (2016,
// A&A 589, A41), Section 3.3 and Tables 3, 4 and 6. Its verdict on the geometric mean, on a binary olivine and
// nickel-iron mixture, is quoted rather than summarized: "the geometric mean is only an approximation of low
// accuracy that should be avoided". Its equation (17), the BRUGGEMAN self-consistent effective-medium rule,
// reproduces the numerical solution for the four six-phase chondritic censuses of its Table 3 to within
// -0.40, -0.16, +0.08 and -0.35 percent at zero porosity, across phase-conductivity contrasts of 6.9:1 to 15.2:1.
// The geometric mean on those same four censuses lands at -3.3, -1.7, -1.2 and -4.8 percent, and the arithmetic
// and harmonic bounds at +32/-17 and +32/-21 percent. So on the multiphase mineral census this function exists to
// aggregate, Bruggeman is roughly an order of magnitude closer to a direct solution of the heat equation than the
// rule the geothermics literature would have handed us.
//
// SO THIS IMPLEMENTS BRUGGEMAN, Henke equation (17), which is the standard symmetric self-consistent form:
//
//     sum_i f_i (K_i - K_eff) / (K_i + 2 K_eff) = 0
//
// The equivalence of the paper's reciprocal statement to that standard form was verified symbolically rather than
// taken on sight, and the solver below was checked against the paper's own Bruggeman column on all four Table 3
// censuses (agreement better than 0.02 W/(m*K)), so what is implemented here is the authors' rule and not a
// lookalike. It carries NO authored parameter: the equation contains only the census fractions and the per-phase
// conductivities, and the `2` and `3` are the form's own coefficients from the spherical-inclusion geometry.
//
// THE HONEST ERROR THIS CARRIES, stated as a band and not as a boast:
//   - AGAINST A DIRECT NUMERICAL SOLUTION, roughly 0.5 percent for a realistic multiphase mineral census at
//     contrast up to ~15:1 (Henke Table 6 at zero porosity), degrading to ~7 percent for a near-equal binary at
//     ~6:1 contrast (Henke Table 4, where it reads high while the geometric mean reads low by a similar amount).
//   - AGAINST A REAL ROCK, unquantified here. The validation above is closure-against-computation, never
//     closure-against-laboratory-rock, and this module claims only what the artifact licenses.
//   - THE INPUT CONDUCTIVITIES DOMINATE ANYWAY where the estimator rung carries weight: Slack's rung is a
//     several-fold one-sided bound on a complex cell, so a 0.5 percent mixing rule sitting on a 3x input is
//     precise about the wrong number. [`AssemblageConductivity::measured_weight_fraction`] is how a caller sees
//     that, and it is the field to read before trusting the aggregate.
//
// TERMS DROPPED, BY NAME (each a chord this aggregate does not carry, surfaced rather than silently absorbed):
//   - POROSITY AND CRACKS. Henke's agreement is quoted at ZERO porosity; at porosity 0.30 his Bruggeman column
//     departs from the numerical solution by up to 7.5 percent. This function aggregates a SOLID census, so a
//     porous or cracked rock needs a pore phase in the census and inherits that wider error.
//   - TEXTURE AND PREFERRED ORIENTATION. Bruggeman assumes a statistically isotropic random aggregate with
//     spherical-inclusion geometry. A foliated or lineated rock conducts anisotropically and is not one number.
//   - GRAIN-BOUNDARY RESISTANCE. The numerical benchmark mixes voxels in perfect thermal contact; a real
//     aggregate has interfacial resistance that lowers the effective value.
//   - PRESSURE. The per-phase ladder above is a temperature form with no pressure dependence, so this aggregate
//     is an ambient-pressure quantity, and a caller at depth is reading outside the frame. THAT SENTENCE USED
//     TO BE THE WHOLE DEFENCE, and it defended nothing: `assemblage_conductivity` took no pressure, so it could
//     not tell a lid caller from a mantle one and answered both. The frame is now a checked argument
//     ([`assemblage_conductivity_at`]) and a field on the result, so the limit is enforced rather than
//     described. What is still DROPPED is the physics: closing it needs Hofmeister's `(1 + (K_0'/K_0) P)`
//     factor, which needs `K_0` and `K_0'` per phase, neither of which the lattice form takes today.
//   - RADIATIVE TRANSPORT'S GEOMETRY. Where a phase declares ferrous iron, its radiative term is added to that
//     phase's conductivity BEFORE mixing, which is what the component-conductivity framework requires. But
//     photons are not diffusing phonons and their mean free path can exceed the grain size, so a hot-end
//     aggregate is extrapolating a conduction mixing rule onto a channel its benchmark excluded (Henke neglects
//     radiation entirely). This compounds the type-II dispute already declared on
//     [`radiative_conductivity_w_per_m_k`].

/// Which rung of the per-phase ladder a phase's conductivity came from, carried so an aggregate can report its
/// measured-versus-estimated mix rather than blending the two grades silently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConductivityRung {
    /// A MEASURED `kappa_298` anchor carried through temperature by Hofmeister's class-keyed form.
    MeasuredAnchor,
    /// Slack's DERIVED magnitude at 298 K standing in where no measurement exists, carried through temperature by
    /// the same exponent. Inherits Slack's one-sided upper-bound band on a complex cell.
    SlackEstimator,
}

/// THE SHAPE OF SLACK'S BAND, typed, because the two shapes it comes in are different claims and an absent band
/// is a third thing again.
///
/// WHY THIS IS NOT AN `Option<Fixed>`. It was one, and `None` meant two incompatible things at once: "this rung
/// carries no uncertainty" and "nobody supplied the uncertainty this rung carries". The anchor walk read the
/// second as the first, so a phase resolving through Slack contributed ZERO width to the aggregate band on a
/// magnitude the module's own text calls a several-fold one-sided bound. That is uncertainty laundering at the
/// one site the ladder exists to keep honest, and it fired in production: the only loader in the tree wrote
/// `None` for every phase it built.
///
/// THE WIDTHS ARE FACTORS, not absolute half-widths, because that is how the estimator's error is declared:
/// Slack's own docstring says roughly 3x on a simple cell, and the one complex-cell exhibit is rutile at ~43
/// against a measured ~9. A factor is also the shape that survives a magnitude change, which a several-fold
/// error needs and an additive half-width does not. Each factor must be at least one; a factor below one would
/// invert the interval and is refused rather than silently reordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EstimatorBand {
    /// A SYMMETRIC multiplicative factor `f >= 1`: the truth lies in `[anchor / f, anchor * f]`. Slack's
    /// declared shape on a simple cell, where the form is built for the physics it is being asked about.
    SymmetricFactor(Fixed),
    /// A ONE-SIDED upper bound with factor `f >= 1`: the anchor IS the ceiling and the truth lies in
    /// `[anchor / f, anchor]`. Slack's shape on a complex cell, where its single-scattering form OVERSTATES
    /// by construction. Carried separately so the aggregate can report an asymmetric band instead of reporting
    /// an excursion above a ceiling the physics says cannot be exceeded.
    UpperBoundFactor(Fixed),
    /// NO band was supplied. Distinct from a zero width, and the distinction is the point: a phase that
    /// resolves through Slack's rung carrying this is refused
    /// ([`ConductivityRefusal::NoEstimatorBand`]) rather than aggregated at zero width.
    ///
    /// This is the DEFAULT the banked loader writes, and the refusal is therefore the default path, because
    /// the width is a RESERVED value the owner has not set:
    /// `conductivity.slack_estimator_band_factor` in `calibration/reserved.toml`. Fabricating one here to keep
    /// the aggregate answering would understate an estimator's error at exactly the site that exists to
    /// declare it.
    NotSupplied,
}

/// One phase's conductivity inputs: everything the per-phase ladder needs, plus the phase's name so a refusal can
/// name it. The two rungs each carry their own band, mirroring the Gruneisen floor's row shape, so the band that
/// travels with a value is the band of the rung that supplied it.
#[derive(Clone, Debug)]
pub struct PhaseConductivity {
    /// The phase name, as the registry spells it. Carried so a refusal is a fetch list rather than a silent drop.
    pub name: String,
    /// The MEASURED `kappa_298` anchor (W/(m*K)) when the phase has one. `None` sends the phase to Slack's rung.
    pub kappa_298: Option<Fixed>,
    /// The measured anchor's symmetric half-width band, in W/(m*K), ADDITIVE (unlike the estimator rung's
    /// multiplicative factor, because a measured row reports an absolute uncertainty).
    pub kappa_298_band: Option<Fixed>,
    /// The band on Slack's estimator rung, supplied by the caller because its magnitude is class-dependent and
    /// ONE-SIDED on a complex cell (see [`estimator_anchor_298`]). Never defaulted to a width here: a fabricated
    /// band would understate an estimator's error exactly where the aggregate most needs to declare it, so the
    /// absent case is [`EstimatorBand::NotSupplied`] and it REFUSES.
    pub estimator_band: EstimatorBand,
    /// This phase's OWN cited temperature exponent `a`, when a source determines one for it, bypassing the
    /// class-keyed classifier entirely.
    ///
    /// THE ROUTE THE REFUSAL TEXT ALREADY PROMISED. Both refusal paths out of the classifier tell a caller to
    /// "supply a measured exponent for this phase", and until this field existed there was no way to. It is the
    /// closure for both: a phase in the uncalibrated `2 < n < 6` gap, and a phase in the simple class whose
    /// class exponent is reserved and unset. MgO is the live case, with a cited `a = 0.9`.
    ///
    /// `None` when no cited column supplies one, which is every phase today: no per-phase exponent column is
    /// banked in this repo. Held as `Option` rather than defaulted for the same reason Slack's three inputs are.
    pub measured_exponent_a: Option<Fixed>,
    /// The banked Gruneisen parameter, feeding both Slack's magnitude and Hofmeister's expansion correction.
    pub gruneisen: Fixed,
    /// Slack's mean atomic mass (amu). `None` when no cited column supplies it for this phase.
    pub mean_atomic_mass_amu: Option<Fixed>,
    /// Slack's Debye temperature (K). `None` when no cited column supplies it for this phase.
    pub debye_temperature_k: Option<Fixed>,
    /// Slack's atomic volume (cubic angstrom). `None` when no cited column supplies it for this phase.
    pub atomic_volume_angstrom3: Option<Fixed>,
    /// Atoms per primitive cell: the class variable that keys BOTH Slack's magnitude and Hofmeister's temperature
    /// exponent. A count the cited calibration cannot place (`2 < n < 6`), or one in the simple class whose
    /// exponent is reserved and unset (`n <= 2`), refuses the phase unless [`Self::measured_exponent_a`]
    /// supplies the exponent directly.
    pub atoms_per_primitive_cell: i32,
    /// The expansivity integral from 298 K to the evaluation temperature, the caller's own, because only the
    /// caller knows whether its expansivity is constant over the range.
    pub expansivity_integral: Fixed,
    /// Whether this phase carries Fe2+, and so gains the radiative recovery at the hot end.
    pub bears_ferrous_iron: bool,
}

impl PhaseConductivity {
    /// This phase's conductivity at a temperature, BY THE LADDER: the measured anchor when the phase has one, and
    /// Slack's derived magnitude otherwise, both carried through temperature by Hofmeister's class-keyed exponent,
    /// with the radiative term added where the phase declares ferrous iron. Reports WHICH rung it used.
    ///
    /// `anchor_shift` displaces the anchor by the rung's own band, so the caller can walk the outer edges of the
    /// uncertainty interval: zero for the central value, `+1` for the stiff edge, `-1` for the soft edge. The
    /// two rungs walk it DIFFERENTLY, because their bands are different quantities: the measured rung's is an
    /// additive half-width in W/(m*K), the estimator rung's a multiplicative factor, and the estimator's may be
    /// one-sided, in which case the stiff edge does not move because the anchor IS the ceiling.
    ///
    /// Reports which rung it used AND whether the scoped radiative fit entered, so the aggregate can carry both.
    // @derives: a phase's conductivity at a temperature <- its ladder rung's anchor, band and exponent
    fn conductivity_at(
        &self,
        temperature: Fixed,
        anchor_shift: i32,
    ) -> Result<(Fixed, ConductivityRung, bool), ConductivityRefusal> {
        // The phase's OWN cited exponent wins over the class, because a determination beats a classification.
        // Only when it has none does the cell count decide, and then the two ways the cited set can fail to
        // place a cell are reported as the two different fetches they are.
        let exponent = match self.measured_exponent_a {
            Some(a) => a,
            None => match lattice_exponent_class(self.atoms_per_primitive_cell) {
                ExponentClass::Complex(a) => a,
                ExponentClass::SimpleReserved => {
                    return Err(ConductivityRefusal::ReservedExponentUnset {
                        phase: self.name.clone(),
                        atoms_per_primitive_cell: self.atoms_per_primitive_cell,
                    })
                }
                ExponentClass::Unplaced | ExponentClass::NotALattice => {
                    return Err(ConductivityRefusal::NoExponentClass {
                        phase: self.name.clone(),
                        atoms_per_primitive_cell: self.atoms_per_primitive_cell,
                    })
                }
            },
        };
        let (anchor, rung) = match self.kappa_298 {
            Some(k) if k > ZERO => {
                // The measured rung: an ADDITIVE half-width, walked by addition. A measured row with no stated
                // band contributes no width, which is a weaker claim than the estimator case below and is left
                // as it stands: an absent measured band means the source did not band that row, and today
                // EVERY row carrying a kappa_298 anchor in the cited column carries a band with it (the loader
                // takes the wider of the explicit band and the stated relative uncertainty), so this branch is
                // reachable only from a hand-built row. The estimator case is different in kind, which is why
                // it refuses: there the absence is of a band nobody has, on a magnitude known to be off by
                // several fold.
                let a = match (anchor_shift, self.kappa_298_band) {
                    (0, _) | (_, None) => k,
                    (s, Some(b)) if s > 0 => k.checked_add(b).unwrap_or(k),
                    (_, Some(b)) => k.checked_sub(b).unwrap_or(k),
                };
                (a, ConductivityRung::MeasuredAnchor)
            }
            _ => {
                // Slack's three inputs are absent for every phase no cited column supplies, so their absence
                // routes to the SAME refusal an unevaluable estimator already raises. Holding them as `Option`
                // is what keeps a bridge from inventing a Debye temperature to satisfy the type: a fabricated
                // one would sit dormant behind a measured anchor and go live the first time a census named an
                // unmeasured phase, which is the failure this ladder exists to make impossible.
                let (mean_atomic_mass, debye_temperature, atomic_volume) = match (
                    self.mean_atomic_mass_amu,
                    self.debye_temperature_k,
                    self.atomic_volume_angstrom3,
                ) {
                    (Some(m), Some(t), Some(v)) => (m, t, v),
                    _ => {
                        return Err(ConductivityRefusal::NoRung {
                            phase: self.name.clone(),
                        })
                    }
                };
                let k = estimator_anchor_298(
                    self.gruneisen,
                    mean_atomic_mass,
                    debye_temperature,
                    atomic_volume,
                    self.atoms_per_primitive_cell,
                )
                .ok_or_else(|| ConductivityRefusal::NoRung {
                    phase: self.name.clone(),
                })?;
                // THE ESTIMATOR RUNG'S BAND IS A FACTOR, and an absent one REFUSES. This is the site the
                // laundering happened at: an `Option` read `None` as zero width, so a several-fold one-sided
                // bound entered the aggregate as an exact number.
                let a = match self.estimator_band {
                    EstimatorBand::NotSupplied => {
                        return Err(ConductivityRefusal::NoEstimatorBand {
                            phase: self.name.clone(),
                            declared_factor: None,
                        })
                    }
                    EstimatorBand::SymmetricFactor(f) | EstimatorBand::UpperBoundFactor(f)
                        if f < Fixed::ONE =>
                    {
                        // A factor below one would put the soft edge above the stiff one. Refused rather
                        // than reordered, because a silently inverted interval reports a band that is real
                        // and backwards, which is worse than no band at all.
                        return Err(ConductivityRefusal::NoEstimatorBand {
                            phase: self.name.clone(),
                            declared_factor: Some(f),
                        });
                    }
                    EstimatorBand::SymmetricFactor(f) => match anchor_shift {
                        0 => k,
                        s if s > 0 => k.checked_mul(f).ok_or_else(|| {
                            ConductivityRefusal::NonRepresentable {
                                phase: self.name.clone(),
                            }
                        })?,
                        _ => k.checked_div(f).ok_or_else(|| {
                            ConductivityRefusal::NonRepresentable {
                                phase: self.name.clone(),
                            }
                        })?,
                    },
                    // ONE-SIDED: the estimate is the ceiling, so the stiff edge stays put and only the soft
                    // edge moves. Walking it up as well would report an excursion the physics forbids.
                    EstimatorBand::UpperBoundFactor(f) => match anchor_shift {
                        s if s >= 0 => k,
                        _ => k.checked_div(f).ok_or_else(|| {
                            ConductivityRefusal::NonRepresentable {
                                phase: self.name.clone(),
                            }
                        })?,
                    },
                };
                (a, ConductivityRung::SlackEstimator)
            }
        };
        if anchor <= ZERO {
            return Err(ConductivityRefusal::NonRepresentable {
                phase: self.name.clone(),
            });
        }
        let lattice = hofmeister_lattice_conductivity(
            anchor,
            exponent,
            self.gruneisen,
            self.expansivity_integral,
            temperature,
        )
        .ok_or_else(|| ConductivityRefusal::NonRepresentable {
            phase: self.name.clone(),
        })?;
        let total = if self.bears_ferrous_iron {
            lattice
                .checked_add(radiative_conductivity_w_per_m_k(temperature))
                .ok_or_else(|| ConductivityRefusal::NonRepresentable {
                    phase: self.name.clone(),
                })?
        } else {
            lattice
        };
        if total <= ZERO {
            return Err(ConductivityRefusal::NonRepresentable {
                phase: self.name.clone(),
            });
        }
        Ok((total, rung, self.bears_ferrous_iron))
    }
}

/// A rock's DERIVED effective thermal conductivity, with the evidence a caller needs to know what it is holding.
#[derive(Clone, Debug)]
pub struct AssemblageConductivity {
    /// The Bruggeman effective conductivity (W/(m*K)).
    pub conductivity: Fixed,
    /// How far ABOVE [`Self::conductivity`] the interval reaches, the rule re-solved at the phases' stiff anchor
    /// edges. Separate from [`Self::band_down`] because the estimator rung's band can be ONE-SIDED, and a
    /// one-sided upper bound has an upward excursion of exactly zero. Collapsing the two into one symmetric
    /// number reported an excursion the physics forbids.
    pub band_up: Fixed,
    /// How far BELOW [`Self::conductivity`] the interval reaches, the rule re-solved at the phases' soft anchor
    /// edges. A phase declaring no band widens it by nothing, so this is a FLOOR on the true uncertainty
    /// wherever [`Self::measured_weight_fraction`] is below one.
    pub band_down: Fixed,
    /// How much of the census weight resolved through a MEASURED anchor rather than Slack's estimator, as a
    /// fraction. A caller needing a measured-grade value reads this rather than assuming.
    pub measured_weight_fraction: Fixed,
    /// How much of the census weight took the SCOPED radiative term, as a fraction: the exposure to the fit
    /// [`RadiativeFitScope`] names, measurable rather than a boolean.
    pub radiative_weight_fraction: Fixed,
    /// Which radiative fit, if any, entered this answer. See [`RadiativeFitScope`].
    pub radiative_scope: RadiativeFitScope,
    /// The temperature the aggregate was evaluated at, carried so a caller cannot silently reuse one temperature's
    /// aggregate at another.
    pub frame_temperature_k: Fixed,
    /// The PRESSURE the aggregate is valid at, the sibling of [`Self::frame_temperature_k`] and carried for the
    /// same reason. The ladder has no pressure term, so this is always the anchors' own measured frame
    /// ([`hofmeister_reference_pressure_bar`]); [`assemblage_conductivity_at`] is what refuses a caller whose
    /// state is outside it. Before this field existed the module declared the ambient frame in a comment and
    /// let a caller at depth read the value anyway.
    pub frame_pressure_bar: Fixed,
}

/// Why an aggregation refused, NAMING the phase so the error is the fetch list rather than a silent zero. A
/// default anywhere here would author a rock's heat transport, and a lid thickness with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConductivityRefusal {
    /// The phase resolved to NEITHER rung: no measured `kappa_298` and Slack could not evaluate on its banked
    /// columns. Add the phase's measured anchor, or the columns Slack needs.
    NoRung {
        /// The phase the census named.
        phase: String,
    },
    /// The phase's atoms-per-primitive-cell count falls in `2 < n < 6`, where the cited calibration set places no
    /// temperature exponent, so [`lattice_exponent_class`] refuses. Supply a measured exponent for this phase
    /// through [`PhaseConductivity::measured_exponent_a`]. A DATA gap: closing it needs a determination in that
    /// range.
    NoExponentClass {
        /// The phase the census named.
        phase: String,
        /// The cell count that could not be placed.
        atoms_per_primitive_cell: i32,
    },
    /// The phase sits in the SIMPLE class (`n <= 2`), where the cited set brackets the exponent with two
    /// disagreeing determinations (ice at `a ~ 1`, MgO at `a = 0.9`) and pins no value between them. The class
    /// exponent is a RESERVED value the owner has not set, so it is refused rather than interpolated. Distinct
    /// from [`Self::NoExponentClass`] because it closes differently: an owner decision on
    /// `conductivity.simple_class_temperature_exponent`, or a per-phase cited exponent in
    /// [`PhaseConductivity::measured_exponent_a`].
    ReservedExponentUnset {
        /// The phase the census named.
        phase: String,
        /// The cell count that put it in the simple class.
        atoms_per_primitive_cell: i32,
    },
    /// The phase resolved through SLACK'S ESTIMATOR rung and no usable band came with it, so the aggregate
    /// would have reported a several-fold one-sided uncertainty as a zero-width number. Refused rather than
    /// widened by a fabricated factor: the width is the reserved value
    /// `conductivity.slack_estimator_band_factor`.
    NoEstimatorBand {
        /// The phase the census named.
        phase: String,
        /// The factor supplied, when one was and it was unusable (below one, which would invert the interval),
        /// and `None` when no band was supplied at all.
        declared_factor: Option<Fixed>,
    },
    /// The aggregate was asked at a pressure outside the frame its anchors were measured in. The ladder carries
    /// no pressure term, so its only valid pressure is the anchors' own, widened by whatever slack the owner
    /// sets (`conductivity.ambient_frame_pressure_slack_bar`, zero until then).
    OutsidePressureFrame {
        /// The pressure asked for, in bar.
        requested_bar: Fixed,
        /// The pressure the anchors were measured at, in bar.
        frame_bar: Fixed,
        /// The tolerated offset from that frame, in bar.
        slack_bar: Fixed,
    },
    /// A fixed-point intermediate left the representable window, or an edge anchor went non-positive.
    NonRepresentable {
        /// The phase the census named.
        phase: String,
    },
    /// The self-consistent equation did not bracket a root, which cannot happen for positive per-phase
    /// conductivities and is therefore an arithmetic defect rather than a data gap.
    NoSelfConsistentRoot,
    /// A banked column the ladder reads carries no row for this phase, so its inputs cannot be assembled at
    /// all. Names WHICH column, because the difference between "this phase has no measured conductivity" and
    /// "this phase has no Grueneisen row" is the difference between two different fetches.
    NoBankedColumn {
        /// The phase the census named.
        phase: String,
        /// The column with no row for it, for example `atoms_per_primitive_cell` or `gruneisen`.
        column: String,
    },
}

impl fmt::Display for ConductivityRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConductivityRefusal::NoRung { phase } => write!(
                f,
                "census phase {phase} resolves to no conductivity rung: it has no measured kappa_298 and Slack \
                 cannot evaluate on its banked columns. A rock conductivity is an aggregate over resolved phases, \
                 so an unresolvable phase is refused rather than defaulted (Principle 11)."
            ),
            ConductivityRefusal::NoExponentClass {
                phase,
                atoms_per_primitive_cell,
            } => write!(
                f,
                "census phase {phase} has {atoms_per_primitive_cell} atoms per primitive cell, inside the \
                 2 < n < 6 gap the cited calibration set does not place, so no temperature exponent exists for \
                 it. Supply a measured exponent; it is refused rather than defaulted (Principle 11)."
            ),
            ConductivityRefusal::ReservedExponentUnset {
                phase,
                atoms_per_primitive_cell,
            } => write!(
                f,
                "census phase {phase} has {atoms_per_primitive_cell} atoms per primitive cell, the SIMPLE class, \
                 whose temperature exponent the cited set brackets (ice at a ~ 1, MgO at a = 0.9) without \
                 pinning a value between them. The class exponent is reserved for the owner \
                 (conductivity.simple_class_temperature_exponent); until it is set, supply this phase's own \
                 cited exponent. It previously read 0.95, the midpoint of the bracketing pair, which was an \
                 unattributed interpolation authored inside a classifier."
            ),
            ConductivityRefusal::NoEstimatorBand {
                phase,
                declared_factor,
            } => match declared_factor {
                None => write!(
                    f,
                    "census phase {phase} resolved through Slack's ESTIMATOR rung and declared no band. That \
                     rung carries a several-fold error, one-sided on a complex cell (rutile, ~43 against a \
                     measured ~9), so aggregating it at zero width would report an estimate as a measurement. \
                     The width is reserved (conductivity.slack_estimator_band_factor); refused rather than \
                     fabricated (Principle 11)."
                ),
                Some(factor) => write!(
                    f,
                    "census phase {phase} declared an estimator band factor of {}, which is below one and would \
                     put the soft edge above the stiff one. A band factor is a multiplier of at least one; \
                     refused rather than silently reordered.",
                    factor.to_f64_lossy()
                ),
            },
            ConductivityRefusal::OutsidePressureFrame {
                requested_bar,
                frame_bar,
                slack_bar,
            } => write!(
                f,
                "the conductivity ladder was asked at {} bar and its anchors are measured at {} bar, outside \
                 the {} bar of tolerated offset. The lattice form carries NO pressure term, so its frame is a \
                 point until the tolerated offset is set (conductivity.ambient_frame_pressure_slack_bar). \
                 Reading an ambient conductivity at interior pressure is the same defect the thermoelastic \
                 ladder refuses one layer over.",
                requested_bar.to_f64_lossy(),
                frame_bar.to_f64_lossy(),
                slack_bar.to_f64_lossy()
            ),
            ConductivityRefusal::NonRepresentable { phase } => write!(
                f,
                "census phase {phase} left the representable fixed-point window, or its soft band edge went \
                 non-positive"
            ),
            ConductivityRefusal::NoSelfConsistentRoot => write!(
                f,
                "the Bruggeman self-consistent equation did not bracket a root over the census conductivities, \
                 which is an arithmetic defect rather than a data gap"
            ),
            ConductivityRefusal::NoBankedColumn { phase, column } => write!(
                f,
                "census phase {phase} has no row in the banked {column} column, so its ladder inputs cannot be \
                 assembled; refused rather than defaulted, because a fabricated row here would author the \
                 rock's heat transport"
            ),
        }
    }
}

impl std::error::Error for ConductivityRefusal {}

/// The number of bisection halvings the self-consistent solve runs. This is a REPRESENTATION bound, not a world
/// value: each halving cuts the bracket in two, and 64 halvings drive any bracket representable in Q32.32 below a
/// single ULP, so the solve is converged to the arithmetic's own resolution. The loop also exits early once the
/// midpoint stops moving, which is the exact fixed-point convergence test.
const BRUGGEMAN_HALVINGS: u32 = 64;

/// Bruggeman's residual `sum_i f_i (K_i - z) / (K_i + 2 z)`, whose root is the effective conductivity. It is
/// strictly decreasing in `z` (each term's derivative is `-3 K_i / (K_i + 2z)^2 < 0`), non-negative at the
/// smallest per-phase conductivity and non-positive at the largest, so a root exists in that bracket and bisection
/// finds it. `None` on a fixed-point intermediate leaving the window.
fn bruggeman_residual(components: &[(Fixed, Fixed)], z: Fixed) -> Option<Fixed> {
    let two = Fixed::from_int(2);
    let mut acc = ZERO;
    for (fraction, k) in components {
        let denominator = k.checked_add(two.checked_mul(z)?)?;
        if denominator <= ZERO {
            return None;
        }
        let term = k.checked_sub(z)?.checked_div(denominator)?;
        acc = acc.checked_add(fraction.checked_mul(term)?)?;
    }
    Some(acc)
}

/// Solve Bruggeman's self-consistent equation for the effective conductivity by bisection.
///
/// THE BRACKET IS THE PER-PHASE RANGE `[K_min, K_max]`, deliberately, and NOT the harmonic-to-arithmetic bracket
/// the invariant test checks. The sign argument on `[K_min, K_max]` is true term by term and needs no inequality:
/// at `K_min` every numerator `K_i - z` is non-negative, at `K_max` every one is non-positive. Bisecting the
/// TIGHTER `[harmonic, arithmetic]` bracket would be valid too (Jensen's inequality on the residual puts the root
/// there), but then the test asserting the result lies between those means could not fail, because the bracket
/// would have imposed it. Solving on the wider range keeps that test an independent check of the mathematics.
fn solve_bruggeman(components: &[(Fixed, Fixed)]) -> Option<Fixed> {
    let mut lo = components.first()?.1;
    let mut hi = lo;
    for (_, k) in components {
        if *k < lo {
            lo = *k;
        }
        if *k > hi {
            hi = *k;
        }
    }
    if lo <= ZERO {
        return None;
    }
    let two = Fixed::from_int(2);
    for _ in 0..BRUGGEMAN_HALVINGS {
        let mid = lo.checked_add(hi)?.checked_div(two)?;
        if mid == lo || mid == hi {
            break;
        }
        // The residual decreases in z, so a positive residual means the root lies above the midpoint.
        if bruggeman_residual(components, mid)? > ZERO {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.checked_add(hi)?.checked_div(two)
}

/// DERIVE a rock's effective thermal conductivity at a temperature from a world's own mineral census, by
/// BRUGGEMAN self-consistent effective-medium theory (Henke, Gail and Trieloff 2016, A&A 589, A41, equation 17):
///
/// `sum_i f_i (K_i - K_eff) / (K_i + 2 K_eff) = 0`
///
/// Each phase's `K_i` is resolved at `temperature` through the per-phase ladder above (measured anchor first,
/// Slack's estimator otherwise, Hofmeister's class-keyed exponent carrying either through temperature, radiative
/// term added where the phase declares ferrous iron), so the aggregate consumes the ladder rather than
/// duplicating any part of it. See the module comment above this function for what the rule was chosen over and
/// the error it carries.
///
/// `census` is a list of (phase, fraction) pairs. The fractions are VOLUME fractions, which is the transport-
/// correct weighting and the one Henke's benchmark uses; they need not sum to one, because the result is
/// normalized by the supplied total, so a partial census aggregates what it names.
///
/// Returns `Ok(None)` when the census carries no positive weight. REFUSES rather than defaults: any phase that
/// cannot resolve a conductivity returns a [`ConductivityRefusal`] naming that phase, because a rock's
/// conductivity sets its heat transport and inventing one for a missing phase would author a world's interior.
///
/// Deterministic fixed-point throughout: the solve is a fixed bisection with an exact convergence exit, and no
/// float and no logarithm enters (Bruggeman needs only the four arithmetic operations, unlike a geometric mean).
/// Assemble one phase's ladder inputs from the BANKED columns, so a caller names a phase and gets the row the
/// ladder reads rather than hand-building one.
///
/// This is the bridge the fixture-cluster replacement needs. The cited crystallographic column supplies the
/// cell count and, where the phase has one, the measured `kappa_298` anchor and its band; the Grueneisen table
/// supplies the gamma BOTH rungs need for Hofmeister's expansion correction; and the phase registry plus the
/// periodic table supply the ferrous-iron question by charge balance over the phase's own composition
/// ([`civsim_physics::crystal_field::iron_valence_state`]), so the radiative term is keyed on a DERIVED
/// oxidation state rather than a per-mineral tag. Mixed-valence phases count as ferrous-bearing, because
/// magnetite's `Fe2+` is present and radiating whatever its average charge reads.
///
/// SLACK'S THREE INPUTS ARE DERIVED HERE, and the paragraph that used to sit at this spot is retired rather
/// than edited, because it said the opposite and stayed after the code changed under it. It read that the three
/// inputs "are left absent, deliberately", that "no data file in this repo carries a per-phase Debye
/// temperature or atomic volume", and that "for the mantle census that matters every phase carries a measured
/// anchor, so the estimator rung never fires and the absence costs nothing". All three clauses stopped being
/// true when the fixture-cluster retirement derived the mean atomic mass, the atomic volume and the elastic
/// Debye temperature from tables this function already holds (below). The estimator rung DOES fire from this
/// loader now, for any phase with no measured anchor, which is the spinel and hematite case the census reaches.
///
/// SO THE ESTIMATOR BAND IS THE REMAINING ABSENCE, and it is [`EstimatorBand::NotSupplied`] here for the reason
/// the three inputs used to be `None`: no cited width exists to read. Slack's declared error is a factor, and
/// the only figures this module can cite for it are the ~3x its own docstring declares for simple cells and the
/// single complex-cell exhibit (rutile at ~43 against a measured ~9). Choosing a factor between those would be
/// authoring the very number the band exists to declare, so the width is reserved
/// (`conductivity.slack_estimator_band_factor`) and a phase that resolves through Slack REFUSES by name until
/// it is set. That refusal is a behaviour change from a silent zero-width band, and it is the intended one.
///
/// `expansivity_integral` stays the caller's own, for the reason the field's own docstring gives: only the
/// caller knows whether its expansivity is constant over the range it is integrating.
// @derives: a phase's ladder inputs <- the banked crystallographic, Grueneisen and phase-registry columns + charge balance
pub fn phase_conductivity_from_banked(
    phase_name: &str,
    conductivity: &PhaseConductivityTable,
    gruneisen: &GruneisenTable,
    moduli: Option<&civsim_physics::mineral_moduli::MineralModuli>,
    registry: &PhaseRegistry,
    periodic: &PeriodicTable,
    expansivity_integral: Fixed,
) -> Result<PhaseConductivity, ConductivityRefusal> {
    let missing = |column: &str| ConductivityRefusal::NoBankedColumn {
        phase: phase_name.to_string(),
        column: column.to_string(),
    };
    let atoms_per_primitive_cell = conductivity
        .atoms_per_primitive_cell(phase_name)
        .ok_or_else(|| missing("atoms_per_primitive_cell"))?;
    // Gamma is read by BOTH rungs, so its absence is a hard stop rather than an estimator-only gap. Quartz is
    // the live case: it holds a K'-only anomaly row with no gamma, so a census naming it refuses here.
    let (gamma, _rung) = gruneisen
        .gamma(phase_name)
        .ok_or_else(|| missing("gruneisen"))?;
    let (kappa_298, kappa_298_band) = match conductivity.kappa_298(phase_name) {
        Some((k, band)) => (Some(k), band),
        None => (None, None),
    };
    let phase = registry
        .phase(phase_name)
        .ok_or_else(|| missing("phase_registry"))?;

    // THE ESTIMATOR RUNG'S INPUTS, DERIVED. These three were hardcoded `None`, which meant the Slack rung
    // could never fire from this loader at all: a phase with no measured `kappa_298` resolved to NO rung and
    // the whole assemblage refused. Every one of them derives from tables this function already holds. The
    // docstring above records what that change made stale, because it went unrecorded for a commit.
    //
    // That gap was invisible until the fixture cluster was retired, because nothing before then ran a real
    // derived mantle composition through here. The composition that exposed it minimizes to a SPINEL-bearing
    // assemblage, and spinel has no measured `kappa_298`, so the province field refused outright.
    //
    // THE DEBYE TEMPERATURE HERE IS THE ELASTIC ONE, and this is the one place in the arc where that is the
    // correct choice rather than the error. Slack's model reads `Theta_a`, the ACOUSTIC Debye temperature,
    // and this module's own law documentation says so: "the built shear-aware `Theta_D`, which IS the
    // acoustic average". The entropy-fit effective temperature in `thermoelastic_anchors` would be the wrong
    // one, and the two are separate types precisely so that choice has to be made deliberately.
    let mean_atomic_mass_amu = civsim_physics::petrology::phase_molar_mass(phase, periodic)
        .and_then(|m| {
            let atoms: u32 = phase.composition.iter().map(|(_, c)| *c).sum();
            if atoms == 0 {
                None
            } else {
                m.checked_div(Fixed::from_int(atoms as i32))
            }
        });
    let atomic_volume_angstrom3 =
        crate::thermoelastic::atomic_volume_angstrom3(phase_name, registry);
    let debye_temperature_k = moduli.and_then(|m| {
        crate::thermoelastic::derived_elastic_debye_temperature(phase_name, registry, m, periodic)
            .map(|t| t.kelvin())
    });
    let bears_ferrous_iron = matches!(
        iron_valence_state(&phase.composition, periodic),
        IronValence::Ferrous | IronValence::Mixed
    );
    Ok(PhaseConductivity {
        name: phase_name.to_string(),
        kappa_298,
        kappa_298_band,
        // NOT a width, and NOT a zero width: the declaration that nobody supplied one. See the docstring.
        estimator_band: EstimatorBand::NotSupplied,
        // No per-phase exponent column is banked, so the classifier decides for every phase today. MgO's
        // cited a = 0.9 is the first row a fetch would put here.
        measured_exponent_a: None,
        gruneisen: gamma,
        mean_atomic_mass_amu,
        debye_temperature_k,
        atomic_volume_angstrom3,
        atoms_per_primitive_cell,
        expansivity_integral,
        bears_ferrous_iron,
    })
}

/// The aggregate IN ITS OWN AMBIENT FRAME, for a caller whose state is the anchors' state. Identical to
/// [`assemblage_conductivity_at`] called at [`hofmeister_reference_pressure_bar`], and kept as the short name
/// because the frame it assumes is now stated in the signature's absence of a pressure rather than in a comment.
///
/// A CALLER AT DEPTH WANTS THE OTHER ONE. This function cannot refuse an out-of-frame read, because it is never
/// told the pressure to compare against; it declares the frame it answers in on the result
/// ([`AssemblageConductivity::frame_pressure_bar`]) and leaves the comparison to a caller that knows its own
/// state. A caller holding a pressure should pass it and be refused when it is outside.
// @derives: a rock's effective thermal conductivity at the anchors' ambient pressure frame <- the per-phase conductivity ladder + the world's own mineral census (Bruggeman self-consistent EMT)
pub fn assemblage_conductivity(
    census: &[(&PhaseConductivity, Fixed)],
    temperature: Fixed,
) -> Result<Option<AssemblageConductivity>, ConductivityRefusal> {
    assemblage_conductivity_at(census, temperature, hofmeister_reference_pressure_bar())
}

/// DERIVE a rock's effective thermal conductivity at a temperature AND A PRESSURE from a world's own mineral
/// census, by BRUGGEMAN self-consistent effective-medium theory (Henke, Gail and Trieloff 2016, A&A 589, A41,
/// equation 17):
///
/// `sum_i f_i (K_i - K_eff) / (K_i + 2 K_eff) = 0`
///
/// Each phase's `K_i` is resolved at `temperature` through the per-phase ladder above (measured anchor first,
/// Slack's estimator otherwise, Hofmeister's class-keyed exponent carrying either through temperature, radiative
/// term added where the phase declares ferrous iron), so the aggregate consumes the ladder rather than
/// duplicating any part of it. See the module comment above this function for what the rule was chosen over and
/// the error it carries.
///
/// THE PRESSURE IS TAKEN AND CHECKED, NEVER USED. That reads like a contradiction and it is the honest shape:
/// the ladder carries no pressure term anywhere, so `pressure_bar` cannot enter the arithmetic, and the only
/// truthful thing to do with it is compare it to the frame the anchors were measured in and REFUSE outside.
/// The module used to state the ambient limit in a dropped-terms comment ("a caller at depth is reading outside
/// the frame") while `assemblage_conductivity` took no pressure and answered anyway, so the limit was declared
/// to a reader and enforced against nobody. The tolerated offset is
/// [`ambient_frame_pressure_slack_bar`], zero until the owner sets it, so today the frame is a point.
///
/// `census` is a list of (phase, fraction) pairs. The fractions are VOLUME fractions, which is the transport-
/// correct weighting and the one Henke's benchmark uses; they need not sum to one, because the result is
/// normalized by the supplied total, so a partial census aggregates what it names.
///
/// Returns `Ok(None)` when the census carries no positive weight. REFUSES rather than defaults: any phase that
/// cannot resolve a conductivity returns a [`ConductivityRefusal`] naming that phase, because a rock's
/// conductivity sets its heat transport and inventing one for a missing phase would author a world's interior.
///
/// Deterministic fixed-point throughout: the solve is a fixed bisection with an exact convergence exit, and no
/// float and no logarithm enters (Bruggeman needs only the four arithmetic operations, unlike a geometric mean).
// @derives: a rock's effective thermal conductivity in a checked pressure frame <- the per-phase conductivity ladder + the world's own mineral census + the anchors' measured frame (Bruggeman self-consistent EMT)
pub fn assemblage_conductivity_at(
    census: &[(&PhaseConductivity, Fixed)],
    temperature: Fixed,
    pressure_bar: Fixed,
) -> Result<Option<AssemblageConductivity>, ConductivityRefusal> {
    // THE FRAME GATE FIRST, before any phase resolves, so an out-of-frame caller is told about the frame rather
    // than about whichever phase happens to be missing a column.
    let frame_bar = hofmeister_reference_pressure_bar();
    let slack_bar = ambient_frame_pressure_slack_bar();
    let offset = if pressure_bar > frame_bar {
        pressure_bar.checked_sub(frame_bar)
    } else {
        frame_bar.checked_sub(pressure_bar)
    }
    .ok_or(ConductivityRefusal::OutsidePressureFrame {
        requested_bar: pressure_bar,
        frame_bar,
        slack_bar,
    })?;
    if offset > slack_bar {
        return Err(ConductivityRefusal::OutsidePressureFrame {
            requested_bar: pressure_bar,
            frame_bar,
            slack_bar,
        });
    }
    // Resolve every phase FIRST, so a phase that cannot supply a conductivity refuses HERE with its own name,
    // rather than as a silent drop deeper in the arithmetic that would bias the aggregate toward the remainder.
    let mut centre: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut soft: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut stiff: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut total = ZERO;
    let mut measured = ZERO;
    let mut radiative = ZERO;
    for (phase, fraction) in census {
        if *fraction <= ZERO {
            continue;
        }
        let (k, rung, took_radiative) = phase.conductivity_at(temperature, 0)?;
        let (k_lo, _, _) = phase.conductivity_at(temperature, -1)?;
        let (k_hi, _, _) = phase.conductivity_at(temperature, 1)?;
        centre.push((*fraction, k));
        soft.push((*fraction, k_lo));
        stiff.push((*fraction, k_hi));
        if rung == ConductivityRung::MeasuredAnchor {
            measured = measured.checked_add(*fraction).ok_or_else(|| {
                ConductivityRefusal::NonRepresentable {
                    phase: phase.name.clone(),
                }
            })?;
        }
        if took_radiative {
            radiative = radiative.checked_add(*fraction).ok_or_else(|| {
                ConductivityRefusal::NonRepresentable {
                    phase: phase.name.clone(),
                }
            })?;
        }
        total =
            total
                .checked_add(*fraction)
                .ok_or_else(|| ConductivityRefusal::NonRepresentable {
                    phase: phase.name.clone(),
                })?;
    }
    if total <= ZERO {
        return Ok(None);
    }
    // Normalize the fractions by the supplied total, so a partial census aggregates what it names.
    for set in [&mut centre, &mut soft, &mut stiff] {
        for (fraction, _) in set.iter_mut() {
            *fraction = fraction
                .checked_div(total)
                .ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
        }
    }
    let conductivity = solve_bruggeman(&centre).ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
    // The band is the rule RE-SOLVED at the edges, and it is reported as TWO numbers. Bruggeman is monotone
    // increasing in every K_i (the residual rises with each K_i and falls with z), so re-solving at the soft
    // and stiff edges brackets the central value. The two excursions are kept apart because they can differ:
    // a one-sided estimator band moves the soft edge and leaves the stiff one where it is, and collapsing that
    // to one symmetric number would report an upward excursion the rung's own physics rules out.
    let lo = solve_bruggeman(&soft).ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
    let hi = solve_bruggeman(&stiff).ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
    let up = hi.checked_sub(conductivity).unwrap_or(ZERO);
    let down = conductivity.checked_sub(lo).unwrap_or(ZERO);
    let measured_weight_fraction = measured.checked_div(total).unwrap_or(ZERO);
    let radiative_weight_fraction = radiative.checked_div(total).unwrap_or(ZERO);
    Ok(Some(AssemblageConductivity {
        conductivity,
        band_up: if up > ZERO { up } else { ZERO },
        band_down: if down > ZERO { down } else { ZERO },
        measured_weight_fraction,
        radiative_weight_fraction,
        radiative_scope: if radiative > ZERO {
            RadiativeFitScope::TerranFerrousSilicateFit
        } else {
            RadiativeFitScope::NotApplied
        },
        frame_temperature_k: temperature,
        frame_pressure_bar: frame_bar,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exponent_refuses_the_boundary_the_cited_set_does_not_place() {
        // The cited set pins ONE exponent, a = 0.33 for n >= 6 (rutile, overstated ~5x, is the cell-count
        // anchor). It says NOTHING about 2 < n < 6, so the classifier REFUSES there. Picking a boundary in
        // that gap would author a scalar invisibly, inside a classifier, which is the silent-parameter class.
        assert_eq!(
            lattice_exponent_class(6),
            ExponentClass::Complex(Fixed::from_ratio(33, 100)),
            "n = 6 is the calibrated complex class and its exponent is the cited 0.33"
        );
        for n in 3..=5 {
            assert_eq!(
                lattice_exponent_class(n),
                ExponentClass::Unplaced,
                "n = {n} sits in the gap the cited set does not place; the classifier must refuse, not guess"
            );
            assert!(lattice_exponent_for_cell(n).is_none());
        }
        assert_eq!(
            lattice_exponent_class(0),
            ExponentClass::NotALattice,
            "a cell with no atoms is not a lattice"
        );
        assert!(lattice_exponent_for_cell(0).is_none());
    }

    /// THE UNATTRIBUTED INTERPOLATION, convicted. `n <= 2` returned `Fixed::from_ratio(95, 100)`. The
    /// docstring naming its own calibration set lists ice at `612/T` (`a ~ 1`) and MgO at `a = 0.9` for this
    /// class, and 0.95 is neither: it is their arithmetic midpoint, an authored scalar sitting inside a
    /// classifier that refuses the adjacent band for exactly that reason.
    ///
    /// This test fails against the old code twice over, and both assertions are the finding rather than a
    /// preference. The first: the simple class must not hand back a number the cited set does not contain.
    /// The second: the number it did hand back was the midpoint, computed HERE from the two cited
    /// determinations rather than quoted, so the test states what the code was doing rather than what someone
    /// remembers it doing.
    #[test]
    fn the_simple_class_refuses_rather_than_interpolating_between_its_two_cited_determinations() {
        assert_eq!(
            lattice_exponent_class(2),
            ExponentClass::SimpleReserved,
            "n = 2 is the simple class and its exponent is reserved, not interpolated"
        );
        assert_eq!(
            lattice_exponent_class(1),
            ExponentClass::SimpleReserved,
            "a monatomic cell is the same class"
        );
        assert!(
            lattice_exponent_for_cell(2).is_none(),
            "the number read is refused while the class exponent is unset"
        );

        // The retired value, reconstructed from the two cited determinations it sat between, so the test
        // names what was authored instead of asserting a remembered literal.
        let ice = Fixed::ONE; // ice at 612/T, so a ~ 1
        let mgo = Fixed::from_ratio(9, 10); // MgO at a = 0.9
        let midpoint = ice
            .checked_add(mgo)
            .and_then(|s| s.checked_div(Fixed::from_int(2)))
            .expect("the midpoint of two small exponents is representable");
        assert_eq!(
            midpoint,
            Fixed::from_ratio(95, 100),
            "the retired 0.95 was exactly the midpoint of the bracketing pair, which is what made it authored"
        );

        // AND THE WIDTH IT HID IS REAL. Across the bracketing pair the conductivity at 1600 K moves by well
        // over a tenth, so collapsing the class to a point value reported a banded quantity as exact.
        let hot = Fixed::from_int(1600);
        let anchor = Fixed::from_int(5);
        let at_ice = hofmeister_lattice_conductivity(anchor, ice, ZERO, ZERO, hot).unwrap();
        let at_mgo = hofmeister_lattice_conductivity(anchor, mgo, ZERO, ZERO, hot).unwrap();
        let spread = at_mgo
            .checked_sub(at_ice)
            .and_then(|d| d.checked_div(at_ice))
            .unwrap();
        assert!(
            spread > Fixed::from_ratio(1, 10),
            "the bracketing pair disagrees by more than a tenth at 1600 K, so the class carries a real width: {}",
            spread.to_f64_lossy()
        );
    }

    #[test]
    fn a_steeper_exponent_declines_faster_than_the_shallow_silicate_one() {
        // The whole point of the class-keyed exponent: at the same temperature rise, a simple lattice (~1/T)
        // loses far more of its conductivity than a complex silicate (0.33). A single exponent for both, which
        // is what "1/T for everything" would have shipped, gets one of these two badly wrong. The steep side is
        // read from a phase's OWN cited exponent now, because the class value is reserved.
        let simple = Fixed::from_ratio(9, 10); // MgO's cited per-phase determination
        let complex = lattice_exponent_for_cell(6).unwrap();
        assert!(
            simple > complex,
            "the simple lattice's decline is the steeper one"
        );
        let hot = Fixed::from_int(1200);
        let k_simple =
            hofmeister_lattice_conductivity(Fixed::from_int(10), simple, ZERO, ZERO, hot).unwrap();
        let k_complex =
            hofmeister_lattice_conductivity(Fixed::from_int(10), complex, ZERO, ZERO, hot).unwrap();
        assert!(
            k_simple < k_complex,
            "at 1200 K the same anchor retains more conductivity under the shallow silicate exponent: simple={k_simple:?} complex={k_complex:?}"
        );
    }

    #[test]
    fn the_anchor_is_read_exactly_at_the_reference_temperature() {
        // At T = 298 the power law is unity and the expansion integral is zero, so the form must return the
        // MEASURED anchor untouched. If it does not, the top rung is not measured-magnitude at all.
        let anchor = Fixed::from_int(5);
        let k = hofmeister_lattice_conductivity(
            anchor,
            Fixed::from_ratio(33, 100),
            Fixed::from_ratio(15, 10),
            ZERO,
            hofmeister_reference_temperature_k(),
        )
        .unwrap();
        let err = (k - anchor).abs();
        assert!(
            err < Fixed::from_ratio(1, 100),
            "the anchor reads through at 298 K, got {k:?}"
        );
    }

    #[test]
    fn the_expansion_correction_only_ever_reduces_conductivity() {
        // exp[-(4 gamma + 1/3) * integral] with a positive Grueneisen and a positive expansivity integral is
        // strictly below one: thermal expansion softens the lattice and impedes phonon transport. A positive
        // correction would be the sign error this test exists to catch.
        let hot = Fixed::from_int(1000);
        let a = Fixed::from_ratio(33, 100);
        let no_expansion = hofmeister_lattice_conductivity(
            Fixed::from_int(5),
            a,
            Fixed::from_ratio(15, 10),
            ZERO,
            hot,
        )
        .unwrap();
        let expanded = hofmeister_lattice_conductivity(
            Fixed::from_int(5),
            a,
            Fixed::from_ratio(15, 10),
            Fixed::from_ratio(3, 100),
            hot,
        )
        .unwrap();
        assert!(
            expanded < no_expansion,
            "expansion impedes transport: {expanded:?} !< {no_expansion:?}"
        );
    }

    #[test]
    fn the_radiative_term_is_a_hot_end_quantity_and_stays_non_negative() {
        // It is small and rises steeply with temperature, which is exactly why T_e (the COLD end of the column)
        // is only weakly exposed to its declared dispute band while deep-mantle consumers are not.
        let cold = radiative_conductivity_w_per_m_k(Fixed::from_int(300));
        let hot = radiative_conductivity_w_per_m_k(Fixed::from_int(1800));
        assert!(
            hot > cold,
            "radiative transport grows with temperature: {hot:?} !> {cold:?}"
        );
        assert!(
            cold >= ZERO && hot >= ZERO,
            "a conductivity is never negative"
        );
        assert_eq!(
            radiative_conductivity_w_per_m_k(ZERO),
            ZERO,
            "no photons at zero temperature"
        );
    }

    #[test]
    fn the_overlap_sentinel_reports_the_disagreement_rather_than_resolving_it() {
        // The ladder's integrity mechanism. Where both rungs evaluate, the ratio is REPORTED. It resolves
        // nothing: no caller reads a conductivity from this, it exists to be watched, so that "two models
        // answering one question, never compared" is impossible rather than merely discouraged.
        let measured = Fixed::from_int(9);
        let estimated = Fixed::from_int(43); // rutile: Slack's own convicting exhibit
        let ratio = rung_disagreement_ratio(measured, estimated).unwrap();
        assert!(
            ratio > Fixed::from_int(4),
            "the sentinel surfaces Slack's complex-cell overstatement as the several-fold ratio it is, got {ratio:?}"
        );
        // One-sided by construction on a complex cell: the estimator sits ABOVE, never below.
        assert!(
            ratio > Fixed::ONE,
            "Slack's complex-cell error is an upper bound, so the ratio exceeds one"
        );
        assert!(
            rung_disagreement_ratio(ZERO, estimated).is_none(),
            "no comparison exists without both rungs"
        );
    }

    /// A phase row with the ladder inputs a test needs: a measured anchor, a complex-silicate cell (so the
    /// exponent resolves), and no expansion, so at 298 K the anchor reads through and the aggregate is pure
    /// Bruggeman on the supplied conductivities.
    fn measured_phase(name: &str, kappa: Fixed, band: Option<Fixed>) -> PhaseConductivity {
        PhaseConductivity {
            name: name.to_string(),
            kappa_298: Some(kappa),
            kappa_298_band: band,
            estimator_band: EstimatorBand::NotSupplied,
            measured_exponent_a: None,
            gruneisen: Fixed::from_ratio(15, 10),
            mean_atomic_mass_amu: Some(Fixed::from_int(20)),
            debye_temperature_k: Some(Fixed::from_int(700)),
            atomic_volume_angstrom3: Some(Fixed::from_int(20)),
            atoms_per_primitive_cell: 6,
            expansivity_integral: ZERO,
            bears_ferrous_iron: false,
        }
    }

    fn dec(s: &str) -> Fixed {
        Fixed::from_decimal_str(s).expect("a decimal literal parses")
    }

    /// Henke, Gail and Trieloff 2016 Table 3, the H-chondrite census: six phases, volume fractions and
    /// room-temperature conductivities, spanning a 15.2:1 contrast. This is the census the implemented rule was
    /// selected on, held here so the choice is checkable rather than merely cited.
    fn h_chondrite() -> Vec<(PhaseConductivity, Fixed)> {
        vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.399")),
            (
                measured_phase("orthopyroxene", dec("4.150"), None),
                dec("0.291"),
            ),
            (
                measured_phase("clinopyroxene", dec("4.660"), None),
                dec("0.061"),
            ),
            (
                measured_phase("plagioclase", dec("1.935"), None),
                dec("0.114"),
            ),
            (
                measured_phase("nickel-iron", dec("29.383"), None),
                dec("0.096"),
            ),
            (measured_phase("troilite", dec("4.600"), None), dec("0.039")),
        ]
    }

    fn borrow(census: &[(PhaseConductivity, Fixed)]) -> Vec<(&PhaseConductivity, Fixed)> {
        census.iter().map(|(p, f)| (p, *f)).collect()
    }

    /// The weighted arithmetic (parallel, upper bound) and harmonic (series, lower bound) means of the per-phase
    /// conductivities, computed independently of the solver so the invariant test is a real check.
    fn arithmetic_and_harmonic(census: &[(PhaseConductivity, Fixed)]) -> (Fixed, Fixed) {
        let total: Fixed = census
            .iter()
            .fold(ZERO, |a, (_, f)| a.checked_add(*f).unwrap());
        let mut arith = ZERO;
        let mut recip = ZERO;
        for (p, f) in census {
            let k = p.kappa_298.unwrap();
            let w = f.checked_div(total).unwrap();
            arith = arith.checked_add(w.checked_mul(k).unwrap()).unwrap();
            recip = recip.checked_add(w.checked_div(k).unwrap()).unwrap();
        }
        (arith, Fixed::ONE.checked_div(recip).unwrap())
    }

    /// THE INVARIANT THAT CONVICTS A WRONG IMPLEMENTATION WHATEVER THE RULE. Every physically admissible
    /// effective-medium closure lies between the harmonic (series) and arithmetic (parallel) means of its
    /// components, because those are the rigorous lower and upper bounds on any two-terminal composite. This test
    /// does not depend on Bruggeman being the right choice: it fails for any aggregate that leaves the bounds.
    ///
    /// It has teeth here because the bracket is WIDE: on the H-chondrite census the two means sit at roughly 4.07
    /// and 6.45 W/(m*K), a 49 percent spread. The containment is asserted STRICTLY, which additionally convicts
    /// the two implementations most likely to be written by mistake, namely returning the arithmetic mean (a
    /// volume-weighted average) or the harmonic mean, since either would land exactly on an endpoint.
    ///
    /// The bracket is independent rather than circular: [`solve_bruggeman`] bisects `[K_min, K_max]`, never the
    /// harmonic-to-arithmetic interval, so nothing in the solver imposes this result.
    #[test]
    fn the_aggregate_lies_strictly_between_the_harmonic_and_arithmetic_means() {
        let census = h_chondrite();
        let (arith, harm) = arithmetic_and_harmonic(&census);
        let agg = assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
            .expect("every phase resolves")
            .expect("a positive census");
        assert!(
            harm < arith,
            "the bounds are ordered, harm={harm:?} arith={arith:?}"
        );
        assert!(
            agg.conductivity > harm,
            "the aggregate must exceed the harmonic lower bound: {:?} !> {harm:?}",
            agg.conductivity
        );
        assert!(
            agg.conductivity < arith,
            "the aggregate must fall below the arithmetic upper bound: {:?} !< {arith:?}",
            agg.conductivity
        );
        // The bracket is wide enough that the test is not vacuous.
        let spread = arith.checked_sub(harm).unwrap();
        assert!(
            spread > Fixed::from_int(2),
            "the H-chondrite bracket spans about 2.4 W/(m*K), so containment is a real constraint, got {spread:?}"
        );
    }

    /// THE VENDORED ARTIFACT MADE EXECUTABLE. Henke, Gail and Trieloff 2016 publish their own Bruggeman column
    /// beside a direct numerical solution of the heat conduction equation for the Table 3 censuses. Reproducing
    /// that column through the PUBLIC path is what proves the implemented equation is the authors' rule rather
    /// than a lookalike, and it is the check that a transcription error in an implicit equation cannot survive.
    ///
    /// H chondrite: the paper's Bruggeman value is 4.870 W/(m*K) and its numerical ground truth is 4.890.
    #[test]
    fn the_solver_reproduces_the_published_bruggeman_column() {
        let census = h_chondrite();
        let agg = assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
            .expect("every phase resolves")
            .expect("a positive census");
        let published = dec("4.870");
        let err = agg.conductivity.checked_sub(published).unwrap().abs();
        assert!(
            err < dec("0.001"),
            "the aggregate must reproduce Henke's published Bruggeman value 4.870 for the H-chondrite census, \
             got {} (error {})",
            agg.conductivity.to_f64_lossy(),
            err.to_f64_lossy()
        );
        // And it therefore sits within half a percent of the paper's numerical solution of the heat equation.
        let numerical = dec("4.890");
        let gap = agg.conductivity.checked_sub(numerical).unwrap().abs();
        assert!(
            gap < dec("0.03"),
            "the rule tracks the numerical ground truth 4.890 to better than one percent, got {}",
            agg.conductivity.to_f64_lossy()
        );
        // Every phase carried a measured anchor, so the rung mix is fully measured.
        assert_eq!(agg.measured_weight_fraction, Fixed::ONE);
        assert_eq!(
            agg.frame_temperature_k,
            hofmeister_reference_temperature_k()
        );
    }

    /// A ONE-PHASE CENSUS IS THAT PHASE. The self-consistent equation degenerates to `K_eff = K_1`, and any
    /// aggregate that fails this is broken at the simplest possible input. Also checks that fractions which do
    /// not sum to one still normalize by their own total.
    #[test]
    fn a_single_phase_census_reproduces_that_phase_exactly() {
        let solo = vec![(measured_phase("forsterite", dec("5.188"), None), Fixed::ONE)];
        let agg = assemblage_conductivity(&borrow(&solo), hofmeister_reference_temperature_k())
            .expect("resolves")
            .expect("positive");
        let err = agg.conductivity.checked_sub(dec("5.188")).unwrap().abs();
        assert!(
            err < dec("0.001"),
            "a one-phase census IS that phase, got {}",
            agg.conductivity.to_f64_lossy()
        );
        assert_eq!(agg.band_up, ZERO, "one unbanded phase carries no width up");
        assert_eq!(
            agg.band_down, ZERO,
            "one unbanded phase carries no width down"
        );

        // A fraction that does not sum to one normalizes by the supplied total.
        let doubled = vec![(
            measured_phase("forsterite", dec("5.188"), None),
            Fixed::from_int(7),
        )];
        let agg2 = assemblage_conductivity(&borrow(&doubled), hofmeister_reference_temperature_k())
            .expect("resolves")
            .expect("positive");
        assert_eq!(
            agg2.conductivity, agg.conductivity,
            "the aggregate normalizes by the supplied total"
        );
    }

    /// THE REFUSAL IS THE LOAD-BEARING BEHAVIOUR. A phase that resolves to no rung must stop the aggregate and
    /// name itself, never fall back to a default, because a rock's conductivity sets its heat transport and a
    /// silent default would author a world's interior.
    /// THE END-TO-END PROOF the fixture-cluster replacement rests on: the real mantle census, assembled from
    /// the BANKED columns rather than from hand-built rows, produces a rock conductivity in the physical band.
    ///
    /// This is the test that would catch the chain being broken anywhere along it, because every link is a
    /// banked read: the cited cell counts and measured anchors, the Grueneisen gammas, the phase registry's
    /// compositions, and the periodic table's valences. It asserts a BAND rather than a value, because the
    /// point is the magnitude being physical, and an exact assertion here would be a fixture pretending to be
    /// a measurement.
    #[test]
    fn the_real_mantle_census_assembles_from_banked_columns_into_a_physical_conductivity() {
        let conductivity =
            PhaseConductivityTable::standard().expect("the cited conductivity column loads");
        let gruneisen = GruneisenTable::standard().expect("the Grueneisen table loads");
        let registry = PhaseRegistry::standard().expect("the phase registry loads");
        let periodic = PeriodicTable::standard().expect("the periodic table loads");

        // An olivine-plus-pyroxene mantle, the assemblage the deep-time columns actually carry.
        let rows: Vec<(PhaseConductivity, Fixed)> = [
            ("forsterite", dec("0.55")),
            ("enstatite", dec("0.35")),
            ("fayalite", dec("0.10")),
        ]
        .into_iter()
        .map(|(name, fraction)| {
            let row = phase_conductivity_from_banked(
                name,
                &conductivity,
                &gruneisen,
                None,
                &registry,
                &periodic,
                ZERO,
            )
            .unwrap_or_else(|e| panic!("{name} must assemble from the banked columns: {e}"));
            (row, fraction)
        })
        .collect();

        // Fayalite is the ferrous phase, and the flag is DERIVED by charge balance rather than tagged.
        let fayalite = &rows
            .iter()
            .find(|(r, _)| r.name == "fayalite")
            .expect("fayalite is in the census")
            .0;
        assert!(
            fayalite.bears_ferrous_iron,
            "Fe2SiO4 balances to Fe2+, so the radiative term applies"
        );
        assert!(
            !rows
                .iter()
                .find(|(r, _)| r.name == "forsterite")
                .expect("forsterite is in the census")
                .0
                .bears_ferrous_iron,
            "Mg2SiO4 carries no iron at all"
        );

        // Every mantle phase carries a MEASURED anchor, so the estimator rung never fires and Slack's absent
        // columns cost nothing. That is the claim the whole replacement leans on, so it is asserted here.
        for (row, _) in &rows {
            assert!(
                row.kappa_298.is_some(),
                "{} must resolve on the measured rung",
                row.name
            );
        }

        let aggregate = assemblage_conductivity(&borrow(&rows), dec("1600"))
            .expect("the banked mantle census must not refuse")
            .expect("a census with positive weight returns a value");
        let k = aggregate.conductivity.to_f64_lossy();
        assert!(
            (1.0..=8.0).contains(&k),
            "a silicate mantle at 1600 K should land in roughly 1 to 8 W/(m*K), read {k}"
        );
        assert_eq!(
            aggregate.measured_weight_fraction,
            Fixed::ONE,
            "the whole census is measured-rung, so the reported mix must say so"
        );
    }

    /// Quartz holds a K'-only anomaly row with NO Grueneisen gamma, and gamma is read by both rungs, so a
    /// census naming it must refuse by name rather than proceed on a substituted value. The bridge is where
    /// that refusal has to happen, because past it the ladder has no way to tell an absent gamma from a real
    /// one.
    #[test]
    fn a_phase_with_no_banked_gamma_refuses_at_the_bridge_and_names_the_column() {
        let conductivity =
            PhaseConductivityTable::standard().expect("the cited conductivity column loads");
        let gruneisen = GruneisenTable::standard().expect("the Grueneisen table loads");
        let registry = PhaseRegistry::standard().expect("the phase registry loads");
        let periodic = PeriodicTable::standard().expect("the periodic table loads");

        let refusal = phase_conductivity_from_banked(
            "quartz",
            &conductivity,
            &gruneisen,
            None,
            &registry,
            &periodic,
            ZERO,
        )
        .expect_err("quartz holds no gamma, so the bridge must refuse");
        assert_eq!(
            refusal,
            ConductivityRefusal::NoBankedColumn {
                phase: "quartz".to_string(),
                column: "gruneisen".to_string(),
            }
        );
    }

    #[test]
    fn a_phase_with_no_resolvable_rung_is_refused_and_never_defaulted() {
        let mut ghost = measured_phase("unobtainium", dec("4.0"), None);
        ghost.kappa_298 = None; // no measured anchor
        ghost.debye_temperature_k = None; // and no cited column supplies Slack's input either
        let census = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.5")),
            (ghost, dec("0.5")),
        ];
        let refusal =
            assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
                .expect_err("an unresolvable phase must be refused");
        assert_eq!(
            refusal,
            ConductivityRefusal::NoRung {
                phase: "unobtainium".to_string()
            }
        );
        assert!(
            refusal
                .to_string()
                .contains("refused rather than defaulted"),
            "the refusal explains why it is not a default: {refusal}"
        );
    }

    /// The sibling of the test above, and the reason both are kept: a phase can fail Slack's rung in two
    /// different ways, and only one of them is an ABSENT column. Here the column is PRESENT and unevaluable
    /// (a zero Debye temperature), which is the path through `estimator_anchor_298` rather than the earlier
    /// `Option` check. Both must land on the same named refusal, because a caller distinguishing "we never
    /// fetched this" from "the fetched value cannot evaluate" would be reading a provenance question off a
    /// physics failure.
    #[test]
    fn a_present_but_unevaluable_slack_input_refuses_by_the_same_name() {
        let mut ghost = measured_phase("unobtainium", dec("4.0"), None);
        ghost.kappa_298 = None; // no measured anchor, so the estimator rung is the only route
        ghost.debye_temperature_k = Some(ZERO); // the column exists and cannot evaluate
        let census = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.5")),
            (ghost, dec("0.5")),
        ];
        let refusal =
            assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
                .expect_err("an unevaluable estimator input must be refused");
        assert_eq!(
            refusal,
            ConductivityRefusal::NoRung {
                phase: "unobtainium".to_string()
            }
        );
    }

    /// The second refusal path, and it is not hypothetical: the cited calibration set places no temperature
    /// exponent for `2 < n < 6`, so [`lattice_exponent_for_cell`] refuses there. The aggregate must propagate
    /// that refusal with the phase named rather than quietly choosing an exponent.
    #[test]
    fn a_phase_in_the_uncalibrated_cell_gap_is_refused_with_its_cell_count() {
        let mut odd = measured_phase("mystery-spinel", dec("4.0"), None);
        odd.atoms_per_primitive_cell = 4;
        let census = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.5")),
            (odd, dec("0.5")),
        ];
        let refusal =
            assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
                .expect_err("an unplaceable cell count must be refused");
        assert_eq!(
            refusal,
            ConductivityRefusal::NoExponentClass {
                phase: "mystery-spinel".to_string(),
                atoms_per_primitive_cell: 4
            }
        );
    }

    /// The rung mix is REPORTED rather than blended away: a census mixing a measured anchor with a Slack-estimated
    /// phase must say how much weight was measured, because a 0.5 percent mixing rule sitting on a several-fold
    /// estimator input is precise about the wrong number.
    #[test]
    fn the_rung_mix_reports_how_much_weight_was_measured() {
        let mut estimated = measured_phase("alien-carbide", dec("4.0"), None);
        estimated.kappa_298 = None; // falls to Slack, whose banked columns are populated above
                                    // A declared band, because an estimator rung without one is refused now. The factor here is a TEST
                                    // input rather than a claim about Slack: the reserved width is the owner's.
        estimated.estimator_band = EstimatorBand::UpperBoundFactor(Fixed::from_int(3));
        let census = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.75")),
            (estimated, dec("0.25")),
        ];
        let agg = assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
            .expect("Slack evaluates on the populated columns")
            .expect("positive");
        let err = agg
            .measured_weight_fraction
            .checked_sub(dec("0.75"))
            .unwrap()
            .abs();
        assert!(
            err < dec("0.001"),
            "three quarters of the weight came from a measured anchor, got {}",
            agg.measured_weight_fraction.to_f64_lossy()
        );
    }

    /// The band is the rule RE-SOLVED at the phases' anchor edges, so a banded input widens the output and an
    /// unbanded one does not. This is the outer-interval discipline the moduli aggregate already uses, and it is
    /// what keeps an uncertainty travelling with its value instead of being dropped at the aggregation step.
    #[test]
    fn the_band_is_the_rule_re_solved_at_the_anchor_edges() {
        let unbanded = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.5")),
            (
                measured_phase("plagioclase", dec("1.935"), None),
                dec("0.5"),
            ),
        ];
        let banded = vec![
            (
                measured_phase("olivine", dec("4.349"), Some(dec("0.4"))),
                dec("0.5"),
            ),
            (
                measured_phase("plagioclase", dec("1.935"), Some(dec("0.2"))),
                dec("0.5"),
            ),
        ];
        let t = hofmeister_reference_temperature_k();
        let a = assemblage_conductivity(&borrow(&unbanded), t)
            .expect("resolves")
            .expect("positive");
        let b = assemblage_conductivity(&borrow(&banded), t)
            .expect("resolves")
            .expect("positive");
        assert_eq!(a.band_up, ZERO, "unbanded inputs carry no width up");
        assert_eq!(a.band_down, ZERO, "unbanded inputs carry no width down");
        assert!(
            b.band_up > ZERO && b.band_down > ZERO,
            "banded MEASURED inputs widen the aggregate on both sides, got up={:?} down={:?}",
            b.band_up,
            b.band_down
        );
        assert_eq!(
            a.conductivity, b.conductivity,
            "the central value does not move when only the bands are supplied"
        );
        // The band stays inside the inputs' own spread: it cannot exceed the wider per-phase band.
        assert!(
            b.band_up < dec("0.4") && b.band_down < dec("0.4"),
            "the aggregate band is bounded by the inputs it was re-solved from, got up={:?} down={:?}",
            b.band_up,
            b.band_down
        );
    }

    /// The aggregate falls with temperature, because every rung it consumes does. A rule that lost the temperature
    /// dependence of its inputs would pass every algebraic test above and still be wrong at depth.
    #[test]
    fn the_aggregate_declines_with_temperature_as_its_rungs_do() {
        let census = h_chondrite();
        let cold = assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
            .expect("resolves")
            .expect("positive");
        let hot = assemblage_conductivity(&borrow(&census), Fixed::from_int(1200))
            .expect("resolves")
            .expect("positive");
        assert!(
            hot.conductivity < cold.conductivity,
            "phonon scattering cuts conductivity with temperature: hot={:?} cold={:?}",
            hot.conductivity,
            cold.conductivity
        );
        assert_eq!(hot.frame_temperature_k, Fixed::from_int(1200));
    }

    /// THE ZERO-WIDTH CLAIM, convicted. A phase resolving through Slack's rung with no declared band used to
    /// contribute NO width to the aggregate, so a several-fold one-sided estimate was reported with the same
    /// band a measurement would carry: `None` meant "no uncertainty exists" and "no uncertainty was supplied"
    /// at the same site. It must refuse instead, and it must name the phase, because the fix for it is a fetch
    /// rather than a retry.
    ///
    /// Against the old code this passed silently with `band == 0`, which is precisely the defect.
    #[test]
    fn an_estimator_rung_with_no_declared_band_refuses_instead_of_claiming_zero_width() {
        let mut estimated = measured_phase("alien-carbide", dec("4.0"), None);
        estimated.kappa_298 = None; // no measured anchor, so Slack's rung is the only route
        estimated.estimator_band = EstimatorBand::NotSupplied;
        let census = vec![
            (measured_phase("olivine", dec("4.349"), None), dec("0.75")),
            (estimated, dec("0.25")),
        ];
        let refusal =
            assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
                .expect_err("an undeclared estimator band must be refused");
        assert_eq!(
            refusal,
            ConductivityRefusal::NoEstimatorBand {
                phase: "alien-carbide".to_string(),
                declared_factor: None,
            }
        );
        assert!(
            refusal.to_string().contains("reserved"),
            "the refusal names the width as reserved rather than missing: {refusal}"
        );

        // A factor below one would invert the interval, so it is refused rather than reordered.
        let mut inverted = measured_phase("alien-carbide", dec("4.0"), None);
        inverted.kappa_298 = None;
        inverted.estimator_band = EstimatorBand::SymmetricFactor(Fixed::from_ratio(1, 2));
        let bad = vec![(inverted, Fixed::ONE)];
        assert_eq!(
            assemblage_conductivity(&borrow(&bad), hofmeister_reference_temperature_k())
                .expect_err("a sub-unit factor must be refused"),
            ConductivityRefusal::NoEstimatorBand {
                phase: "alien-carbide".to_string(),
                declared_factor: Some(Fixed::from_ratio(1, 2)),
            }
        );
    }

    /// A ONE-SIDED BAND IS REPORTED ONE-SIDED. Slack's complex-cell error is an intrinsic UPPER bound, so the
    /// truth sits at or below the estimate and never above it. The aggregate must therefore report zero upward
    /// excursion and a real downward one. Against the old code this could not even be expressed: there was a
    /// single symmetric `band`, so a one-sided bound was reported as an interval reaching above a ceiling the
    /// physics forbids.
    ///
    /// Every bound in this test is COMPUTED from the inputs rather than asserted from intuition: a one-phase
    /// census IS that phase, so the soft edge is the anchor divided by the declared factor and the aggregate's
    /// downward excursion is the difference, exactly.
    #[test]
    fn a_one_sided_estimator_band_reports_no_upward_excursion() {
        let factor = Fixed::from_int(3);
        let mut one_sided = measured_phase("alien-carbide", dec("4.0"), None);
        one_sided.kappa_298 = None;
        one_sided.estimator_band = EstimatorBand::UpperBoundFactor(factor);
        let census = vec![(one_sided.clone(), Fixed::ONE)];
        let agg = assemblage_conductivity(&borrow(&census), hofmeister_reference_temperature_k())
            .expect("Slack evaluates on the populated columns")
            .expect("positive");
        assert_eq!(
            agg.band_up, ZERO,
            "the estimate IS the ceiling, so the interval reaches nothing above it"
        );
        // A one-phase census is that phase, so the soft edge is the centre divided by the factor and the
        // downward excursion is exactly what that division removes.
        let expected_down = agg
            .conductivity
            .checked_sub(agg.conductivity.checked_div(factor).unwrap())
            .unwrap();
        let err = agg.band_down.checked_sub(expected_down).unwrap().abs();
        assert!(
            err < dec("0.001"),
            "the downward excursion is the anchor walked down by the declared factor: got {} expected {}",
            agg.band_down.to_f64_lossy(),
            expected_down.to_f64_lossy()
        );

        // The SYMMETRIC shape is a different claim and must read differently on the same anchor and factor.
        let mut symmetric = one_sided;
        symmetric.estimator_band = EstimatorBand::SymmetricFactor(factor);
        let sym_census = vec![(symmetric, Fixed::ONE)];
        let sym =
            assemblage_conductivity(&borrow(&sym_census), hofmeister_reference_temperature_k())
                .expect("resolves")
                .expect("positive");
        assert!(
            sym.band_up > ZERO,
            "a symmetric factor DOES reach above the centre, got {:?}",
            sym.band_up
        );
        assert_eq!(
            sym.band_down, agg.band_down,
            "the two shapes agree on the downward excursion and differ only above"
        );
    }

    /// THE PRESSURE FRAME IS CHECKED, NOT DESCRIBED. The module declared "a caller at depth is reading outside
    /// the frame" in a comment while the aggregate took no pressure and answered anyway. It must now refuse,
    /// and it must carry the frame it answered in on the result.
    ///
    /// The mantle pressure used here is not an arbitrary large number: it is the pressure geodynamics evaluates
    /// its columns at, on the order of 100 kbar, which is where an ambient conductivity would have been read.
    #[test]
    fn a_caller_outside_the_ambient_pressure_frame_is_refused_rather_than_answered() {
        let census = h_chondrite();
        let t = hofmeister_reference_temperature_k();

        // In frame: the anchors' own pressure answers.
        let inside =
            assemblage_conductivity_at(&borrow(&census), t, hofmeister_reference_pressure_bar())
                .expect("the anchors' own frame resolves")
                .expect("positive");
        assert_eq!(
            inside.frame_pressure_bar,
            hofmeister_reference_pressure_bar(),
            "the result declares the frame it is valid in"
        );

        // Out of frame: a mid-mantle lithostatic pressure is refused by name.
        let deep = Fixed::from_int(100_000); // 100 kbar, 10 GPa
        let refusal = assemblage_conductivity_at(&borrow(&census), t, deep)
            .expect_err("a caller at depth must be refused");
        assert_eq!(
            refusal,
            ConductivityRefusal::OutsidePressureFrame {
                requested_bar: deep,
                frame_bar: hofmeister_reference_pressure_bar(),
                slack_bar: ambient_frame_pressure_slack_bar(),
            }
        );

        // The short entry point is the ambient one BY CONSTRUCTION, so the two agree where the frame allows.
        let ambient = assemblage_conductivity(&borrow(&census), t)
            .expect("resolves")
            .expect("positive");
        assert_eq!(ambient.conductivity, inside.conductivity);
        assert_eq!(
            ambient.frame_pressure_bar,
            hofmeister_reference_pressure_bar()
        );
    }

    /// THE RADIATIVE FIT'S SCOPE TRAVELS WITH THE ANSWER. The gate on the term is derived by charge balance and
    /// so admits any Fe2+ chemistry, while the polynomial behind it is a fit to Terran silicates. The aggregate
    /// must therefore say when that fit entered and how much of the census inherited it, rather than leaving a
    /// consumer to infer a universal law from a clean gate.
    #[test]
    fn the_radiative_fit_declares_its_scope_and_its_share_of_the_census() {
        let clean = h_chondrite();
        let hot = Fixed::from_int(1600);
        let no_iron = assemblage_conductivity(&borrow(&clean), hot)
            .expect("resolves")
            .expect("positive");
        assert_eq!(
            no_iron.radiative_scope,
            RadiativeFitScope::NotApplied,
            "no phase declares Fe2+, so no scoped fit entered"
        );
        assert_eq!(no_iron.radiative_weight_fraction, ZERO);

        let mut ferrous = measured_phase("fayalite", dec("3.161"), None);
        ferrous.bears_ferrous_iron = true;
        let mixed = vec![
            (measured_phase("forsterite", dec("5.158"), None), dec("0.7")),
            (ferrous, dec("0.3")),
        ];
        let with_iron = assemblage_conductivity(&borrow(&mixed), hot)
            .expect("resolves")
            .expect("positive");
        assert_eq!(
            with_iron.radiative_scope,
            RadiativeFitScope::TerranFerrousSilicateFit,
            "a ferrous phase pulls in the scoped fit, and the result says so"
        );
        let err = with_iron
            .radiative_weight_fraction
            .checked_sub(dec("0.3"))
            .unwrap()
            .abs();
        assert!(
            err < dec("0.001"),
            "three tenths of the census took the scoped fit, got {}",
            with_iron.radiative_weight_fraction.to_f64_lossy()
        );
    }

    /// A PHASE'S OWN CITED EXPONENT BEATS THE CLASSIFIER, which is the route both refusal messages have always
    /// told callers to take and which nothing implemented. It is what keeps the ladder's overlap sentinel
    /// reachable while the simple class's exponent is reserved: MgO carries a cited `a = 0.9`.
    #[test]
    fn a_phase_carrying_its_own_cited_exponent_evaluates_where_the_class_refuses() {
        // A simple-class phase with no per-phase exponent refuses, naming the reserved value.
        let mut periclase = measured_phase("periclase", dec("48.4"), None);
        periclase.atoms_per_primitive_cell = 2;
        let refusal = assemblage_conductivity(
            &borrow(&[(periclase.clone(), Fixed::ONE)]),
            Fixed::from_int(1000),
        )
        .expect_err("the simple class carries no exponent of its own");
        assert_eq!(
            refusal,
            ConductivityRefusal::ReservedExponentUnset {
                phase: "periclase".to_string(),
                atoms_per_primitive_cell: 2,
            }
        );

        // The same phase with its cited determination evaluates, and it evaluates AT that exponent: at 1000 K
        // the answer must equal the anchor carried by 0.9 exactly, since no expansion is supplied here.
        let mut cited = periclase;
        cited.measured_exponent_a = Some(Fixed::from_ratio(9, 10));
        let hot = Fixed::from_int(1000);
        let agg = assemblage_conductivity(&borrow(&[(cited, Fixed::ONE)]), hot)
            .expect("a cited per-phase exponent resolves")
            .expect("positive");
        let expected = hofmeister_lattice_conductivity(
            dec("48.4"),
            Fixed::from_ratio(9, 10),
            Fixed::from_ratio(15, 10),
            ZERO,
            hot,
        )
        .unwrap();
        let err = agg.conductivity.checked_sub(expected).unwrap().abs();
        assert!(
            err < dec("0.001"),
            "a one-phase census IS that phase carried by ITS cited exponent: got {} expected {}",
            agg.conductivity.to_f64_lossy(),
            expected.to_f64_lossy()
        );

        // And the cell count that put it in the uncalibrated gap is bypassed the same way.
        let mut gapped = measured_phase("mystery", dec("4.0"), None);
        gapped.atoms_per_primitive_cell = 4;
        gapped.measured_exponent_a = Some(Fixed::from_ratio(5, 10));
        assert!(
            assemblage_conductivity(&borrow(&[(gapped, Fixed::ONE)]), hot).is_ok(),
            "a cited exponent closes the 2 < n < 6 gap for the phase that carries one"
        );
    }

    /// An empty or zero-weight census is not an error and not a zero: it is the honest `None`, because there is
    /// nothing to aggregate.
    #[test]
    fn a_census_with_no_weight_returns_none_rather_than_zero() {
        let none = assemblage_conductivity(&[], hofmeister_reference_temperature_k())
            .expect("no phase to refuse");
        assert!(none.is_none());
        let zero = vec![(measured_phase("olivine", dec("4.349"), None), ZERO)];
        let agg = assemblage_conductivity(&borrow(&zero), hofmeister_reference_temperature_k())
            .expect("a zero-weight phase is skipped, not refused");
        assert!(agg.is_none(), "no positive weight means no aggregate");
    }
}
