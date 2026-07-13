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

//! The localized energy route (D3 follow-on #3): the machinery the correlation guard's
//! [`crate::correlation::EnergyRoute::Localized`] slot points to, the first cut being a MEASURED Born-Haber fill.
//!
//! A Mott insulator (NiO/CoO/FeO/MnO, the correlation guard's Localized class) is ionically bonded, so its lattice
//! energy would in principle route to the D1 Born-Lande ionic estimator. But D1 cannot score a TM oxide: the Born
//! exponent keys on the ion's isoelectronic noble-gas core, and a 3d cation (Ni2+ = [Ar]3d8, 26 electrons) is not
//! isoelectronic with any noble gas, so it falls through, and the d-electron Born exponent is not cleanly
//! derivable (compressibility-fitting is circular; the Ar-core value is wrong for a 3d-mediated repulsion). So the
//! honest fill is the MEASURED Born-Haber lattice energy, the TOP rung of the provenance ladder (measured row,
//! then estimator, then compute-once): escalating to the cited measurement when the substrate cannot DERIVE the
//! value is the ladder working as designed, not a lookup dodging derive-first (the gate's ruling).
//!
//! WHY NO DERIVED BAND. The value is MEASURED (`tm_oxide_lattice_energy.toml`), so its uncertainty is the
//! measurement's, not a model band; there is no derived self-uncertainty to report (unlike the D1 ionic band or
//! the D3-c metallic Rose-ratio band, whose ranking quantities are DERIVED). The route reports the cited `[M]`
//! lattice energy directly for the seeded insulators and escalates (`None`) for any unseeded oxide.
//!
//! THE SHARED DEEPER FOLLOW-ON. A cited Born-Mayer `rho` for the TM oxides would upgrade this measured fill to a
//! DERIVATION for the UNSEEDED oxides, the generality the measured column cannot give, and it is the same
//! TM-oxide gap that blocks the itinerant-oxide metallic route, so one coherent follow-on serves both (flagged,
//! not built, until a cited non-circular `rho` is grounded). No consumer is wired to this route in any pinned run
//! path yet (byte-neutral).

use civsim_core::Fixed;
use civsim_physics::tm_oxide_lattice_energy::TmOxideLatticeEnergies;

/// The localized energy route over the cited TM-oxide Born-Haber lattice energies.
pub struct LocalizedRoute<'a> {
    energies: &'a TmOxideLatticeEnergies,
}

impl<'a> LocalizedRoute<'a> {
    /// Bind the localized route to the cited TM-oxide Born-Haber lattice-energy set.
    pub fn new(energies: &'a TmOxideLatticeEnergies) -> Self {
        LocalizedRoute { energies }
    }

    /// The signed lattice energy (kJ/mol, released/negative) of a seeded Mott insulator, the cited MEASURED `[M]`
    /// value the disposer ranks a localized candidate on, or `None` (escalate) for any unseeded oxide. This is the
    /// consumer the correlation guard's Localized slot routes to; the metals the guard routes elsewhere never
    /// reach it.
    pub fn localized_energy(&self, composition: &[(String, u32)]) -> Option<Fixed> {
        self.energies.lattice_energy(composition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation::{CorrelationClassifier, EnergyRoute};
    use civsim_physics::d_state_radius::DStateRadii;
    use civsim_physics::ionic_radii::IonicRadii;
    use civsim_physics::ionization_ladder::IonizationLadder;
    use civsim_physics::mit_reference::MitReference;
    use civsim_physics::periodic::PeriodicTable;

    fn energies() -> TmOxideLatticeEnergies {
        TmOxideLatticeEnergies::standard().expect("the TM-oxide set loads")
    }

    fn comp(pairs: &[(&str, u32)]) -> Vec<(String, u32)> {
        pairs.iter().map(|(s, c)| ((*s).to_string(), *c)).collect()
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn a_seeded_mott_insulator_scores_the_cited_measured_value() {
        let e = energies();
        let route = LocalizedRoute::new(&e);
        let nio = route
            .localized_energy(&comp(&[("Ni", 1), ("O", 1)]))
            .expect("NiO scores");
        assert!(
            close(nio, -3908.0, 0.5),
            "NiO localized energy = -3908 kJ/mol (cited), got {}",
            nio.to_f64_lossy()
        );
        // All four seeded insulators score.
        for oxide in [("Ni", 1), ("Co", 1), ("Fe", 1), ("Mn", 1)] {
            assert!(
                route.localized_energy(&comp(&[oxide, ("O", 1)])).is_some(),
                "{}O scores a localized energy",
                oxide.0
            );
        }
    }

    #[test]
    fn an_unseeded_oxide_escalates() {
        let e = energies();
        let route = LocalizedRoute::new(&e);
        // TiO/VO are itinerant (metals), not seeded insulators, so the localized route escalates on them.
        assert!(
            route
                .localized_energy(&comp(&[("Ti", 1), ("O", 1)]))
                .is_none(),
            "TiO escalates in the localized route (not a seeded insulator)"
        );
        // An oxide outside the seeded set escalates rather than guessing.
        assert!(
            route
                .localized_energy(&comp(&[("Mg", 1), ("O", 1)]))
                .is_none(),
            "MgO escalates (not a seeded correlated insulator)"
        );
    }

    #[test]
    fn the_localized_route_is_the_guards_localized_consumer() {
        // THE INTEGRATION (the gate's ruling): the correlation guard routes a Mott insulator (NiO) to the
        // Localized slot, and this route fills that slot with a real cited number. The itinerant metals the guard
        // routes to Metallic never reach the localized route, and the localized route escalates on them anyway.
        let table = PeriodicTable::standard().expect("periodic table");
        let ladder = IonizationLadder::standard().expect("ionization ladder");
        let d_state = DStateRadii::standard().expect("d-state radii");
        let radii = IonicRadii::standard().expect("ionic radii");
        let mit = MitReference::standard().expect("MIT reference set");
        let guard = CorrelationClassifier::calibrate(&table, &ladder, &d_state, &radii, &mit)
            .expect("the classifier calibrates");
        let e = energies();
        let route = LocalizedRoute::new(&e);

        // NiO: the guard routes it Localized, and the localized route gives a real cited number.
        let nio = comp(&[("Ni", 1), ("O", 1)]);
        assert_eq!(guard.route(&nio), EnergyRoute::Localized);
        assert!(
            route.localized_energy(&nio).is_some(),
            "the Localized slot now has a real consumer for NiO"
        );

        // TiO: the guard routes it Metallic (not Localized), and the localized route would escalate on it anyway.
        let tio = comp(&[("Ti", 1), ("O", 1)]);
        assert_eq!(guard.route(&tio), EnergyRoute::Metallic);
        assert!(route.localized_energy(&tio).is_none());
    }
}
