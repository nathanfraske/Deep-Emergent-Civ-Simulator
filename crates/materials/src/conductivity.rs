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
//! - TOP RUNG, [`hofmeister_lattice_conductivity`]: a MEASURED `kappa_298` anchor carrying derived temperature
//!   and pressure dependence off banked Grueneisen, bulk modulus, and expansivity. Highest accuracy, available
//!   only where a mineral HAS a measured anchor.
//! - ESTIMATOR RUNG, [`crate::properties::lattice_thermal_conductivity_w_per_m_k`] (Slack): no anchor needed,
//!   evaluable for anything with banked columns, carrying the band its own docstring declares (roughly 3x
//!   symmetric on simple cells, ONE-SIDED on complex cells, where it is an intrinsic UPPER BOUND that can sit
//!   several-fold above truth; rutile is its own convicting exhibit at ~43 against a measured ~9).
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

const ZERO: Fixed = Fixed::ZERO;

/// The reference temperature Hofmeister's lattice form is anchored at: 298 K, the standard state the measured
/// `kappa_298` rows are reported against. It is the SOURCE'S OWN reference, not a chosen scale.
pub fn hofmeister_reference_temperature_k() -> Fixed {
    Fixed::from_int(298)
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
/// THE CALIBRATION SET, and its honest limit. The exponents are pinned by three independent measurements: ice at
/// `612/T` (`a ~ 1`), MgO at `a = 0.9`, and complex silicates at `a = 0.33` (Hofmeister). The CELL-COUNT
/// boundary is calibrated on the cited set Slack's own docstring convicts itself with: diamond, NaCl, and MgO
/// (all `n = 2`) land inside its band, while rutile (`n = 6`) is overstated ~5x.
///
/// SO THE BOUNDARY IS UNDERDETERMINED IN `2 < n < 6`, AND THIS FUNCTION REFUSES THERE rather than picking a
/// number the cited set does not support. `None` is the honest answer for a cell the calibration cannot place,
/// and a caller that gets `None` must supply a measured exponent or escalate. Picking a boundary inside that gap
/// would be authoring the very scalar the shape-first method exists to avoid, and it would author it invisibly,
/// inside a classifier, which is the silent-parameter class exactly.
pub fn lattice_exponent_for_cell(atoms_per_primitive_cell: i32) -> Option<Fixed> {
    if atoms_per_primitive_cell < 1 {
        return None;
    }
    if atoms_per_primitive_cell <= 2 {
        // The simple-lattice limit: the Umklapp `1/T`. Ice (612/T) and MgO (0.9) both sit here.
        return Some(Fixed::from_ratio(95, 100));
    }
    if atoms_per_primitive_cell >= 6 {
        // The complex-cell class: Hofmeister's silicate exponent.
        return Some(Fixed::from_ratio(33, 100));
    }
    // 2 < n < 6: the cited set places nothing here. Refuse rather than author a boundary.
    None
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
/// `None` on a non-positive temperature or anchor, or a fixed-point overflow. Deterministic fixed-point.
// @derives: lattice thermal conductivity k(T,P) <- a measured kappa_298 anchor + banked Grueneisen, bulk modulus and expansivity (measured rung)
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

/// THE RADIATIVE conductivity (W/(m*K)) of an Fe2+-bearing phase, Hofmeister's polynomial:
///
/// `kappa_rad(T) = 0.0175 - 1.037e-4 T + 2.245e-7 T^2 - 3.407e-11 T^3`
///
/// Photons carry heat through a semi-transparent solid, and the Fe2+ absorption bands set how far they travel.
/// It matters only at the HOT end: the term is small and rises steeply with temperature, so it is a deep-mantle
/// quantity, and a caller adds it to the lattice term only for a phase that carries Fe2+.
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
// @derives: the radiative conductivity silicates gain at high T <- temperature
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
/// declared ~3x, roughly symmetric.
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
//     is an ambient-pressure quantity, and a caller at depth is reading outside the frame.
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

/// One phase's conductivity inputs: everything the per-phase ladder needs, plus the phase's name so a refusal can
/// name it. The two rungs each carry their own band, mirroring the Gruneisen floor's row shape, so the band that
/// travels with a value is the band of the rung that supplied it.
#[derive(Clone, Debug)]
pub struct PhaseConductivity {
    /// The phase name, as the registry spells it. Carried so a refusal is a fetch list rather than a silent drop.
    pub name: String,
    /// The MEASURED `kappa_298` anchor (W/(m*K)) when the phase has one. `None` sends the phase to Slack's rung.
    pub kappa_298: Option<Fixed>,
    /// The measured anchor's symmetric half-width band.
    pub kappa_298_band: Option<Fixed>,
    /// The band on Slack's estimator rung, supplied by the caller because its magnitude is class-dependent and
    /// ONE-SIDED on a complex cell (see [`estimator_anchor_298`]). Never defaulted here: a fabricated band would
    /// understate an estimator's error exactly where the aggregate most needs to declare it.
    pub estimator_band: Option<Fixed>,
    /// The banked Gruneisen parameter, feeding both Slack's magnitude and Hofmeister's expansion correction.
    pub gruneisen: Fixed,
    /// Slack's mean atomic mass (amu).
    pub mean_atomic_mass_amu: Fixed,
    /// Slack's Debye temperature (K).
    pub debye_temperature_k: Fixed,
    /// Slack's atomic volume (cubic angstrom).
    pub atomic_volume_angstrom3: Fixed,
    /// Atoms per primitive cell: the class variable that keys BOTH Slack's magnitude and Hofmeister's temperature
    /// exponent. A count the cited calibration cannot place (`2 < n < 6`) refuses the phase.
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
    /// uncertainty interval: zero for the central value, `+1` for the stiff edge, `-1` for the soft edge.
    fn conductivity_at(
        &self,
        temperature: Fixed,
        anchor_shift: i32,
    ) -> Result<(Fixed, ConductivityRung), ConductivityRefusal> {
        let exponent =
            lattice_exponent_for_cell(self.atoms_per_primitive_cell).ok_or_else(|| {
                ConductivityRefusal::NoExponentClass {
                    phase: self.name.clone(),
                    atoms_per_primitive_cell: self.atoms_per_primitive_cell,
                }
            })?;
        let (base, band, rung) = match self.kappa_298 {
            Some(k) if k > ZERO => (k, self.kappa_298_band, ConductivityRung::MeasuredAnchor),
            _ => {
                let k = estimator_anchor_298(
                    self.gruneisen,
                    self.mean_atomic_mass_amu,
                    self.debye_temperature_k,
                    self.atomic_volume_angstrom3,
                    self.atoms_per_primitive_cell,
                )
                .ok_or_else(|| ConductivityRefusal::NoRung {
                    phase: self.name.clone(),
                })?;
                (k, self.estimator_band, ConductivityRung::SlackEstimator)
            }
        };
        // Walk the anchor to the requested edge. A phase declaring no band contributes no width, which is why the
        // aggregate reports the rung mix: an unbanded estimator rung is a silent zero-width claim otherwise.
        let anchor = match (anchor_shift, band) {
            (0, _) | (_, None) => base,
            (s, Some(b)) if s > 0 => base.checked_add(b).unwrap_or(base),
            (_, Some(b)) => base.checked_sub(b).unwrap_or(base),
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
        Ok((total, rung))
    }
}

/// A rock's DERIVED effective thermal conductivity, with the evidence a caller needs to know what it is holding.
#[derive(Clone, Debug)]
pub struct AssemblageConductivity {
    /// The Bruggeman effective conductivity (W/(m*K)).
    pub conductivity: Fixed,
    /// The symmetric half-width covering the outer interval, the rule re-solved at the phases' soft and stiff
    /// anchor edges. A phase declaring no band widens it by nothing, so this is a FLOOR on the true uncertainty
    /// wherever [`Self::measured_weight_fraction`] is below one.
    pub band: Fixed,
    /// How much of the census weight resolved through a MEASURED anchor rather than Slack's estimator, as a
    /// fraction. A caller needing a measured-grade value reads this rather than assuming.
    pub measured_weight_fraction: Fixed,
    /// The temperature the aggregate was evaluated at, carried so a caller cannot silently reuse one temperature's
    /// aggregate at another. The pressure frame is ambient: the ladder carries no pressure dependence.
    pub frame_temperature_k: Fixed,
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
    /// temperature exponent, so [`lattice_exponent_for_cell`] refuses. Supply a measured exponent for this phase.
    NoExponentClass {
        /// The phase the census named.
        phase: String,
        /// The cell count that could not be placed.
        atoms_per_primitive_cell: i32,
    },
    /// A fixed-point intermediate left the representable window, or an edge anchor went non-positive.
    NonRepresentable {
        /// The phase the census named.
        phase: String,
    },
    /// The self-consistent equation did not bracket a root, which cannot happen for positive per-phase
    /// conductivities and is therefore an arithmetic defect rather than a data gap.
    NoSelfConsistentRoot,
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
pub fn assemblage_conductivity(
    census: &[(&PhaseConductivity, Fixed)],
    temperature: Fixed,
) -> Result<Option<AssemblageConductivity>, ConductivityRefusal> {
    // Resolve every phase FIRST, so a phase that cannot supply a conductivity refuses HERE with its own name,
    // rather than as a silent drop deeper in the arithmetic that would bias the aggregate toward the remainder.
    let mut centre: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut soft: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut stiff: Vec<(Fixed, Fixed)> = Vec::with_capacity(census.len());
    let mut total = ZERO;
    let mut measured = ZERO;
    for (phase, fraction) in census {
        if *fraction <= ZERO {
            continue;
        }
        let (k, rung) = phase.conductivity_at(temperature, 0)?;
        let (k_lo, _) = phase.conductivity_at(temperature, -1)?;
        let (k_hi, _) = phase.conductivity_at(temperature, 1)?;
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
    // The band is the half-width covering the outer interval. Bruggeman is monotone increasing in every K_i (the
    // residual rises with each K_i and falls with z), so re-solving at the soft and stiff edges brackets the
    // central value and the wider gap is the covering half-width.
    let lo = solve_bruggeman(&soft).ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
    let hi = solve_bruggeman(&stiff).ok_or(ConductivityRefusal::NoSelfConsistentRoot)?;
    let up = hi.checked_sub(conductivity).unwrap_or(ZERO);
    let down = conductivity.checked_sub(lo).unwrap_or(ZERO);
    let band = if up >= down { up } else { down };
    let measured_weight_fraction = measured.checked_div(total).unwrap_or(ZERO);
    Ok(Some(AssemblageConductivity {
        conductivity,
        band: if band > ZERO { band } else { ZERO },
        measured_weight_fraction,
        frame_temperature_k: temperature,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exponent_refuses_the_boundary_the_cited_set_does_not_place() {
        // The calibration set pins n <= 2 (diamond, NaCl, MgO all land inside Slack's band) and n = 6 (rutile,
        // overstated ~5x). It says NOTHING about 2 < n < 6, so the classifier REFUSES there. Picking a boundary
        // in that gap would author a scalar invisibly, inside a classifier, which is the silent-parameter class.
        assert!(
            lattice_exponent_for_cell(2).is_some(),
            "n = 2 is the calibrated simple class"
        );
        assert!(
            lattice_exponent_for_cell(6).is_some(),
            "n = 6 is the calibrated complex class"
        );
        for n in 3..=5 {
            assert!(
                lattice_exponent_for_cell(n).is_none(),
                "n = {n} sits in the gap the cited set does not place; the classifier must refuse, not guess"
            );
        }
        assert!(
            lattice_exponent_for_cell(0).is_none(),
            "a cell with no atoms is not a lattice"
        );
    }

    #[test]
    fn the_simple_class_declines_far_more_steeply_than_the_complex_one() {
        // The whole point of the class-keyed exponent: at the same temperature rise, a simple lattice (~1/T)
        // loses far more of its conductivity than a complex silicate (0.33). A single exponent for both, which
        // is what "1/T for everything" would have shipped, gets one of these two badly wrong.
        let simple = lattice_exponent_for_cell(2).unwrap();
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
            estimator_band: None,
            gruneisen: Fixed::from_ratio(15, 10),
            mean_atomic_mass_amu: Fixed::from_int(20),
            debye_temperature_k: Fixed::from_int(700),
            atomic_volume_angstrom3: Fixed::from_int(20),
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
        assert_eq!(agg.band, ZERO, "one unbanded phase carries no width");

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
    #[test]
    fn a_phase_with_no_resolvable_rung_is_refused_and_never_defaulted() {
        let mut ghost = measured_phase("unobtainium", dec("4.0"), None);
        ghost.kappa_298 = None; // no measured anchor
        ghost.debye_temperature_k = ZERO; // and Slack cannot evaluate either
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
        assert_eq!(a.band, ZERO, "unbanded inputs carry no width");
        assert!(
            b.band > ZERO,
            "banded inputs widen the aggregate, got {:?}",
            b.band
        );
        assert_eq!(
            a.conductivity, b.conductivity,
            "the central value does not move when only the bands are supplied"
        );
        // The band stays inside the inputs' own spread: it cannot exceed the wider per-phase band.
        assert!(
            b.band < dec("0.4"),
            "the aggregate band is bounded by the inputs it was re-solved from, got {:?}",
            b.band
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
