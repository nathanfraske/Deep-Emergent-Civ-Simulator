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

//! The correlation classifier (D2b): the Stage-3 class dispatch that SITES the correlated regime BEFORE the
//! energy route, so a Mott insulator (NiO) is never routed to the metallic branch band theory would call it.
//!
//! It compares the on-site Coulomb `U` against the itinerant bandwidth `W` (Mott-Hubbard), both DERIVED from the
//! cited `[M]` floor columns D2a built:
//! - `U = U_atomic(q)`, the differential ionization energy `IE_{q+1} - IE_q` from the ionization ladder (the
//!   d-d Coulomb the Mott picture keys on), for the cation at its charge `q`.
//! - `W ~ r_d^3 / d^5` (Harrison), with `r_d` the 3d-state radius (derived from cited `Zeff`) and `d` the
//!   interionic distance (the Shannon-sum r0).
//!
//! THE SINGLE CALIBRATED `U/W` NORMALIZATION (gate-ruled, the only identifiable form). `U/W = (screening /
//! C_Harrison) * U_atomic * d^5 / r_d^3`; the in-crystal screening and Harrison's prefactor are DEGENERATE in the
//! MIT fit (only their ratio is determined), so a single `[D]`-from-`[M]` constant is the identifiable quantity,
//! not a decomposed pair (Harrison's prefactor is in his book, unfetchable, and we do not fabricate it). The
//! classifier keys on the raw ratio `rho = U_atomic * d^5 / r_d^3` and dispatches against the MIT reference set's
//! own clusters, so the calibrated constant is implicit in the thresholds, no fabricated number.
//!
//! THE VALIDATION IS THE ORDERING, NOT THE THRESHOLD (gate requirement). The single constant only places the
//! line; it cannot reorder the set. So the real test is whether the DERIVED `rho` ORDERS the MIT reference set,
//! the observed insulators above the observed metals. If it does, the classifier is validated and the separation
//! margin (`insulator_min / metal_max`) is the honesty number (the analog of the D1 band's 4.08 percent). If it
//! does NOT separate, no constant can fix it and [`CorrelationClassifier::calibrate`] REFUSES (returns
//! [`CalibrationError::NotSeparable`]), the honest signal that the columns are insufficient rather than a fitted
//! line papering over the failure.
//!
//! THE WINDOW IS A REAL REFUSAL. A material whose `rho` falls in the gap between the metal and insulator clusters
//! is [`CorrelationClass::Window`], estimators-FORBIDDEN (escalate to measured or compute-once), the substrate
//! refusing correctly where `U/W ~ 1`. So is a material the classifier cannot score (a charge beyond the seeded
//! ladder, an ion absent from the columns) or one out of the calibrated structure class.
//!
//! HONEST LIMIT (gate scope note): the single constant absorbs the rock-salt M-M-versus-M-O structure factor, so
//! it is calibrated for the ROCK-SALT-oxide class. A correlated material of a different structure (rutile VO2,
//! corundum V2O3) has a different structure factor and should escalate (or carry its own class calibration),
//! never be routed by this rock-salt-fit threshold. D2b's scope is the rock-salt divalent monoxides; other
//! structures and higher charges escalate as out of scope.
//!
//! HONEST LIMIT (the scope is NOT fully code-enforced, audit findings, named follow-ons). Two Terran-scope
//! encodings the classifier does not yet read from data: (a) the interionic distance is read at a hardcoded
//! octahedral coordination (`OCTAHEDRAL = 6`, the rock-salt cation coordination that also lives as data in
//! `prototypes.toml`), while `identify_correlated_pair` admits ANY binary d-block-cation oxide, so a non-rock-salt
//! 1:1 correlated oxide is scored at coordination 6 (a wrong `d`, hence a wrong `rho`) rather than escalating as
//! the note above promises; enforcing that escalation needs the substance's own structure datum (read the
//! coordination from the structure floor + escalate a non-rock-salt structure), a follow-on. (b) The bandwidth
//! axis reads the 3d-STATE radius (`d_state_radius`, `N_STAR_3D = 3`), so the classifier is correct only for the
//! 3d transition series (its calibration set); a 4d/5d/4f correlated centre needs both a general principal quantum
//! number derived from the element's configuration AND its own per-series calibration, a named follow-on. Within
//! the seeded 3d rock-salt monoxides both hold by construction, so the current results are sound; the gaps are
//! that the code, not only the data, encodes the Terran scope.

use civsim_core::Fixed;
use civsim_physics::d_state_radius::DStateRadii;
use civsim_physics::ionic_radii::IonicRadii;
use civsim_physics::ionization_ladder::IonizationLadder;
use civsim_physics::mit_reference::{MitReference, ObservedClass};
use civsim_physics::periodic::PeriodicTable;

/// The coordination the interionic distance is read at (octahedral, the rock-salt monoxide site).
const OCTAHEDRAL: u8 = 6;

/// The correlated-regime class the dispatch sites, keyed BEFORE the energy route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrelationClass {
    /// Itinerant (`U/W` small): route to the metallic machinery.
    Itinerant,
    /// Localized (`U/W` large): route to the localized machinery (Hund, crystal field, superexchange).
    Localized,
    /// The `U/W ~ 1` window: estimators FORBIDDEN, escalate to measured or compute-once (the correct refusal).
    Window,
    /// Out of the classifier's scope (not a scorable rock-salt binary correlated oxide, an ion absent from the
    /// columns, or a charge beyond the seeded ladder): escalate rather than route.
    OutOfScope,
}

/// The energy route the correlation guard directs a composition to, keyed on its correlation class BEFORE any
/// energy estimate: the whole point of the D2 arc, so a Mott insulator (Localized) is routed AWAY from the
/// metallic estimator and never handed a confident metallic number. Each route is a SLOT; an unimplemented slot
/// is an honest escalation (never a fabricated number), so until D3-c fills the metallic slot and a later slice
/// fills the localized machinery, invoking those routes escalates. The guard's routing DECISION is complete now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyRoute {
    /// The metallic energy route (the Itinerant class): the Rose/Vinet EOS plus Miedema. Filled by D3-c; until
    /// then this slot is unimplemented and invoking it escalates.
    Metallic,
    /// The localized machinery (the Localized class): Hund, crystal field, superexchange. A later slice; until
    /// then this slot is unimplemented and invoking it escalates.
    Localized,
    /// Estimators FORBIDDEN: escalate to measured or compute-once. The Window (near `U/W = 1`) and the
    /// out-of-scope classes route here directly, and any unimplemented slot collapses here too.
    Escalate,
}

/// The pure mapping from a correlation class to its energy route, the correlation guard's decision independent of
/// scoring: Itinerant to the metallic route, Localized to the localized machinery, Window and OutOfScope to
/// escalation (estimators forbidden). A Localized material is routed AWAY from the metallic route, the
/// correlation-hardening spec's whole point.
pub fn route_of_class(class: CorrelationClass) -> EnergyRoute {
    match class {
        CorrelationClass::Itinerant => EnergyRoute::Metallic,
        CorrelationClass::Localized => EnergyRoute::Localized,
        CorrelationClass::Window | CorrelationClass::OutOfScope => EnergyRoute::Escalate,
    }
}

/// What can go wrong calibrating the classifier against the MIT reference set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationError {
    /// A reference material could not be scored (a floor column is missing for it), so the calibration is
    /// incomplete and the threshold cannot be trusted.
    UnscorableReference(String),
    /// The reference set has no metals, or no insulators, so no threshold can be placed.
    DegenerateReference,
    /// The DERIVED `rho` does NOT separate the observed insulators above the observed metals (`insulator_min <=
    /// metal_max`): no threshold works, so the columns are insufficient for the set. The honest refusal.
    NotSeparable {
        /// The largest `rho` among the observed metals.
        metal_max: Fixed,
        /// The smallest `rho` among the observed insulators.
        insulator_min: Fixed,
    },
}

/// The correlation classifier over the D2a floor columns, calibrated against the MIT reference set.
pub struct CorrelationClassifier<'a> {
    table: &'a PeriodicTable,
    ladder: &'a IonizationLadder,
    d_state: &'a DStateRadii,
    radii: &'a IonicRadii,
    /// The largest `rho` among the observed metals (the itinerant-side threshold).
    metal_max: Fixed,
    /// The smallest `rho` among the observed insulators (the localized-side threshold).
    insulator_min: Fixed,
}

impl<'a> CorrelationClassifier<'a> {
    /// Calibrate the classifier against the MIT reference set: score every reference material, then place the
    /// itinerant/localized thresholds at the metal and insulator clusters. Refuses (returns an error) if a
    /// reference is unscorable, the set is degenerate, or the derived `rho` does not separate the classes (the
    /// gate's requirement that a non-separating set be flagged, never papered over with a fitted line).
    pub fn calibrate(
        table: &'a PeriodicTable,
        ladder: &'a IonizationLadder,
        d_state: &'a DStateRadii,
        radii: &'a IonicRadii,
        reference: &MitReference,
    ) -> Result<Self, CalibrationError> {
        // A partly-built classifier with sentinel thresholds, used only to score the reference materials.
        let scorer = CorrelationClassifier {
            table,
            ladder,
            d_state,
            radii,
            metal_max: Fixed::ZERO,
            insulator_min: Fixed::ZERO,
        };
        let mut metal_max: Option<Fixed> = None;
        let mut insulator_min: Option<Fixed> = None;
        for material in reference.materials() {
            let rho = scorer
                .rho(&material.composition)
                .ok_or_else(|| CalibrationError::UnscorableReference(material.name.clone()))?;
            match material.observed_class {
                ObservedClass::Metal => {
                    metal_max = Some(match metal_max {
                        Some(m) if m >= rho => m,
                        _ => rho,
                    });
                }
                ObservedClass::Insulator => {
                    insulator_min = Some(match insulator_min {
                        Some(m) if m <= rho => m,
                        _ => rho,
                    });
                }
            }
        }
        let metal_max = metal_max.ok_or(CalibrationError::DegenerateReference)?;
        let insulator_min = insulator_min.ok_or(CalibrationError::DegenerateReference)?;
        // The ordering IS the validation: the insulators must sit above the metals. If not, no threshold works.
        if insulator_min <= metal_max {
            return Err(CalibrationError::NotSeparable {
                metal_max,
                insulator_min,
            });
        }
        Ok(CorrelationClassifier {
            table,
            ladder,
            d_state,
            radii,
            metal_max,
            insulator_min,
        })
    }

    /// The separation margin `insulator_min / metal_max`, the measured honesty number (the analog of the D1
    /// band's 4.08 percent): how cleanly the derived `rho` separates the observed insulators from the metals.
    /// Greater than one iff the set separates (the classifier only calibrates when it does).
    pub fn separation_ratio(&self) -> Fixed {
        self.insulator_min
            .checked_div(self.metal_max)
            .unwrap_or(Fixed::ZERO)
    }

    /// The raw Mott-Hubbard ratio `rho = U_atomic * d^5 / r_d^3` for a composition, or `None` when it is not a
    /// scorable rock-salt binary correlated oxide (not a clean binary cation-anion pair, the cation has no
    /// d-state radius or ionization ladder reach at its charge, or an ion is absent from the radii). The absolute
    /// scale is not physical (the calibration absorbs it); only the ratio across compositions is load-bearing.
    pub fn rho(&self, composition: &[(String, u32)]) -> Option<Fixed> {
        let pair = self.identify_correlated_pair(composition)?;
        let u_atomic = self.ladder.atomic_u(&pair.cation, pair.cation_charge)?;
        let r_d = self.d_state.radius(&pair.cation)?;
        let cation_radius = self
            .radii
            .radius(&pair.cation, pair.cation_charge as i8, OCTAHEDRAL)?
            .crystal_radius;
        let anion_radius = self
            .radii
            .radius(&pair.anion, pair.anion_charge, OCTAHEDRAL)?
            .crystal_radius;
        let d = cation_radius + anion_radius;
        // rho = u_atomic * d^5 / r_d^3, all checked (small magnitudes, but never fabricate an overflow).
        let d2 = d.checked_mul(d)?;
        let d4 = d2.checked_mul(d2)?;
        let d5 = d4.checked_mul(d)?;
        let rd2 = r_d.checked_mul(r_d)?;
        let rd3 = rd2.checked_mul(r_d)?;
        u_atomic.checked_mul(d5)?.checked_div(rd3)
    }

    /// Classify a composition by siting its `rho` against the calibrated clusters. An unscorable composition is
    /// [`CorrelationClass::OutOfScope`] (escalate), never forced into a class.
    pub fn classify(&self, composition: &[(String, u32)]) -> CorrelationClass {
        match self.rho(composition) {
            Some(rho) => self.classify_rho(rho),
            None => CorrelationClass::OutOfScope,
        }
    }

    /// The pure dispatch on a `rho` value against the calibrated thresholds: localized above the insulator
    /// cluster, itinerant below the metal cluster, and the WINDOW (escalate) in the gap between them.
    pub fn classify_rho(&self, rho: Fixed) -> CorrelationClass {
        if rho >= self.insulator_min {
            CorrelationClass::Localized
        } else if rho <= self.metal_max {
            CorrelationClass::Itinerant
        } else {
            CorrelationClass::Window
        }
    }

    /// The CORRELATION GUARD (D3-b): route a composition to its energy route by its correlation class, BEFORE any
    /// energy estimate. Classify, then map the class to its route ([`route_of_class`]). This is the class-keyed
    /// dispatch the whole D2 arc built toward: a Mott insulator (Localized) is routed AWAY from the metallic
    /// route and never handed a confident metallic number; an itinerant metal routes to the metallic route; a
    /// window or out-of-scope material escalates. The metallic and localized route slots are filled by later
    /// slices, and until then invoking them escalates (an unimplemented slot is an honest escalation), but the
    /// routing DECISION here is complete.
    pub fn route(&self, composition: &[(String, u32)]) -> EnergyRoute {
        route_of_class(self.classify(composition))
    }

    /// The correlated cation and its charge-balance-derived charge for a composition (the d-block centre the
    /// Localized route's Hund local moment keys on), or `None` for a non-correlated composition. Exposes the
    /// classifier's own pair identification so a consumer (the magnetism dispatch) reads the cation and its charge
    /// rather than re-deriving them.
    pub fn correlated_cation(&self, composition: &[(String, u32)]) -> Option<(String, u32)> {
        let pair = self.identify_correlated_pair(composition)?;
        Some((pair.cation, pair.cation_charge))
    }

    /// The correlated cation-anion roles of a binary compound: the anion is the element with a negative valence,
    /// the cation is the other AND must carry a d-state radius (a correlated d-block centre); the cation charge
    /// derives from charge balance. `None` for a non-binary, a non-d-block cation, or a non-integer balance.
    fn identify_correlated_pair(&self, composition: &[(String, u32)]) -> Option<CorrelatedPair> {
        if composition.len() != 2 {
            return None;
        }
        let mut cation: Option<(&str, u32)> = None;
        let mut anion: Option<(&str, u32, i8)> = None;
        for (symbol, count) in composition {
            let el = self.table.element(symbol)?;
            match el.valence.iter().copied().find(|v| *v < 0) {
                Some(charge) => {
                    if anion.is_some() {
                        return None;
                    }
                    anion = Some((symbol.as_str(), *count, charge));
                }
                None => {
                    if cation.is_some() {
                        return None;
                    }
                    cation = Some((symbol.as_str(), *count));
                }
            }
        }
        let (cation_symbol, cation_count) = cation?;
        let (anion_symbol, anion_count, anion_charge) = anion?;
        // The cation must be a correlated d-block centre (it carries a d-state radius), else out of scope.
        self.d_state.radius(cation_symbol)?;
        let anion_total = anion_count as i32 * anion_charge as i32; // negative
        if cation_count == 0 || anion_total % (cation_count as i32) != 0 {
            return None;
        }
        let cation_charge = -anion_total / (cation_count as i32);
        if cation_charge <= 0 {
            return None;
        }
        Some(CorrelatedPair {
            cation: cation_symbol.to_string(),
            cation_charge: cation_charge as u32,
            anion: anion_symbol.to_string(),
            anion_charge,
        })
    }
}

/// The identified correlated cation-anion roles of a binary compound.
struct CorrelatedPair {
    cation: String,
    cation_charge: u32,
    anion: String,
    anion_charge: i8,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floors() -> (
        PeriodicTable,
        IonizationLadder,
        DStateRadii,
        IonicRadii,
        MitReference,
    ) {
        (
            PeriodicTable::standard().expect("periodic table"),
            IonizationLadder::standard().expect("ionization ladder"),
            DStateRadii::standard(
                &civsim_units::constants::canonical_si_execution_magnitudes()
                    .expect("the sealed physical floor projects"),
            )
            .expect("d-state radii"),
            IonicRadii::standard().expect("ionic radii"),
            MitReference::standard().expect("MIT reference set"),
        )
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    #[test]
    fn the_derived_rho_separates_the_mit_set_insulators_above_metals() {
        // THE VALIDATION (the gate's requirement): the derived rho, before any threshold, ORDERS the reference
        // set, the observed insulators (NiO/CoO/FeO/MnO) all above the observed metals (TiO/VO). If it did not,
        // calibrate would refuse. That it calibrates at all IS the ordering proof.
        let (t, l, ds, r, mit) = floors();
        let classifier = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit)
            .expect("the derived rho separates the MIT set (insulators above metals)");
        // The separation margin is the honesty number, comfortably above 1 (a clean gap).
        let ratio = classifier.separation_ratio().to_f64_lossy();
        assert!(
            ratio > 1.3,
            "the insulator/metal rho separation should be a clean margin, got {ratio}"
        );
    }

    #[test]
    fn the_insulators_are_localized_and_the_metals_itinerant() {
        // NiO (which band theory calls a metal) sites as LOCALIZED and the early-3d monoxides TiO/VO site as
        // ITINERANT, the class keyed before any energy route. HONEST SCOPE OF THIS TEST: these six are the
        // calibration set, so each classifying to its own side is a by-construction property once calibrate()
        // succeeds (an insulator has rho >= insulator_min, a metal rho <= metal_max); what this test actually
        // guards is the boundary INCLUSIVITY of classify_rho (a strict bound would drop a boundary member into
        // Window). The GENUINE validation, that the derived rho ORDERS the set at all (and refuses a set it
        // cannot), lives in the_derived_rho_separates_the_mit_set... and a_non_separating_reference_set_is_refused,
        // not here. No held-out monoxide is classified (the seeded d-block cations are the calibration set only),
        // so out-of-sample generalization is not tested; it is a flagged follow-on (a wider cited MIT set).
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        for insulator in [
            ("Ni", 1, "O", 1),
            ("Co", 1, "O", 1),
            ("Fe", 1, "O", 1),
            ("Mn", 1, "O", 1),
        ] {
            let composition = comp(&[(insulator.0, insulator.1), (insulator.2, insulator.3)]);
            assert_eq!(
                c.classify(&composition),
                CorrelationClass::Localized,
                "{}O should site as localized (a Mott insulator)",
                insulator.0
            );
        }
        for metal in [("Ti", 1, "O", 1), ("V", 1, "O", 1)] {
            let composition = comp(&[(metal.0, metal.1), (metal.2, metal.3)]);
            assert_eq!(
                c.classify(&composition),
                CorrelationClass::Itinerant,
                "{}O should site as itinerant (a correlated metal)",
                metal.0
            );
        }
    }

    #[test]
    fn the_guard_routes_a_mott_insulator_away_from_the_metallic_route() {
        // THE D2b PAYOFF (D3-b): the correlation guard routes NiO (Localized) to the localized route, AWAY from
        // the metallic estimator, so a Mott insulator is never handed a confident metallic number. The itinerant
        // metals route TO the metallic route; a window or out-of-scope material escalates.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        // NiO (a Mott insulator) is routed away from the metallic slot.
        let nio = comp(&[("Ni", 1), ("O", 1)]);
        assert_eq!(
            c.route(&nio),
            EnergyRoute::Localized,
            "NiO routes to the localized machinery, never the metallic estimator"
        );
        assert_ne!(
            c.route(&nio),
            EnergyRoute::Metallic,
            "a Mott insulator is never handed the metallic route"
        );
        // The itinerant metals route TO the metallic slot (D3-c will fill it).
        for metal in [("Ti", 1), ("V", 1)] {
            assert_eq!(
                c.route(&comp(&[(metal.0, metal.1), ("O", 1)])),
                EnergyRoute::Metallic,
                "{}O (itinerant) routes to the metallic route",
                metal.0
            );
        }
        // Out-of-scope (VO2, MgO) escalates.
        assert_eq!(
            c.route(&comp(&[("V", 1), ("O", 2)])),
            EnergyRoute::Escalate,
            "VO2 (out of scope) escalates rather than routing"
        );
    }

    #[test]
    fn the_guard_maps_every_class_to_a_route() {
        // The pure routing decision, independent of scoring: itinerant to metallic, localized to localized,
        // window and out-of-scope to escalation. All four classes covered (the window case is unreachable by a
        // real seeded composition, so its mapping is proven here directly).
        assert_eq!(
            route_of_class(CorrelationClass::Itinerant),
            EnergyRoute::Metallic
        );
        assert_eq!(
            route_of_class(CorrelationClass::Localized),
            EnergyRoute::Localized
        );
        assert_eq!(
            route_of_class(CorrelationClass::Window),
            EnergyRoute::Escalate
        );
        assert_eq!(
            route_of_class(CorrelationClass::OutOfScope),
            EnergyRoute::Escalate
        );
    }

    #[test]
    fn a_rho_in_the_gap_is_the_window_refusal() {
        // THE DEMONSTRATE-FAILURE: a rho between the metal and insulator clusters is the WINDOW, estimators
        // forbidden (escalate), not a confident routing. The gap midpoint is the clearest window case.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        // A rho exactly between the two clusters must escalate (the model cannot classify it within U/W ~ 1).
        let midpoint = (c.metal_max + c.insulator_min)
            .checked_div(Fixed::from_int(2))
            .expect("midpoint");
        assert_eq!(
            c.classify_rho(midpoint),
            CorrelationClass::Window,
            "a rho in the metal-insulator gap escalates (the U/W ~ 1 window refusal)"
        );
    }

    #[test]
    fn an_out_of_class_material_escalates_rather_than_routing() {
        // VO2 is the window poster child: rutile V4+, out of the rock-salt divalent class AND its charge (4+)
        // is beyond the seeded ladder depth (IE5 absent), so the classifier cannot score it and ESCALATES
        // (OutOfScope) rather than confidently routing it. The honest refusal on a material out of scope.
        let (t, l, ds, r, mit) = floors();
        let c = CorrelationClassifier::calibrate(&t, &l, &ds, &r, &mit).expect("calibrates");
        // VO2: V with two O. Charge balance gives V4+, whose U needs IE5 (not seeded), so rho is None.
        assert_eq!(
            c.classify(&comp(&[("V", 1), ("O", 2)])),
            CorrelationClass::OutOfScope,
            "VO2 (V4+, beyond the seeded ladder) escalates as out of scope"
        );
        // A non-transition-metal binary (no d-state radius) is also out of scope (not a correlated centre).
        assert_eq!(
            c.classify(&comp(&[("Mg", 1), ("O", 1)])),
            CorrelationClass::OutOfScope,
            "MgO (no d-state centre) is out of the correlation classifier's scope"
        );
    }

    #[test]
    fn a_non_separating_reference_set_is_refused_not_fitted() {
        // The gate's honesty requirement: if the derived rho does NOT separate the observed classes, calibrate
        // REFUSES rather than placing a meaningless line. A deliberately mislabelled set (NiO called a metal, TiO
        // an insulator, inverting the true order) cannot separate, so calibration returns NotSeparable.
        let (t, l, ds, r, _mit) = floors();
        let inverted = MitReference::from_toml_str(
            r#"
[[reference]]
name = "nickel oxide"
composition = { Ni = 1, O = 1 }
observed_class = "metal"
source = "test (deliberately inverted)"

[[reference]]
name = "titanium oxide"
composition = { Ti = 1, O = 1 }
observed_class = "insulator"
source = "test (deliberately inverted)"
"#,
        )
        .expect("the inverted set loads");
        assert!(
            matches!(
                CorrelationClassifier::calibrate(&t, &l, &ds, &r, &inverted),
                Err(CalibrationError::NotSeparable { .. })
            ),
            "an inverted (non-separating) reference set is refused, not fitted"
        );
    }
}
