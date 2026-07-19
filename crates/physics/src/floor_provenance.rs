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

//! The physics-floor provenance grade register (provenance register Phase 2, the floor unification;
//! `docs/PROVENANCE_LEDGER.md`).
//!
//! A sidecar to the floor manifests (which carry the two-tag `real`/`fantasy` provenance inline): it refines
//! each floor value entry's provenance into the owner's seven-tag register (the same tags the calibration
//! side carries), source-verified from each entry's own recorded ground. It also carries the
//! `derive_first_defect` marker, the gate's seam-2 correction: a bulk material property STORED on a
//! substance rather than DERIVED from its mineral or element components is a generality defect (Principle
//! 11, admit-the-alien), ORTHOGONAL to the refutability provenance, so the value stays `[M]` measured and the
//! defect is separately countable for the materials buildout that resolves it. `authoring_surface` is the
//! floor's contribution to the honesty number (the closure-plus-authored count); the full DAG join over the
//! unified calibration-plus-floor register is the calibration-side query.

use serde::Deserialize;

/// One floor value entry's seven-tag grade, keyed to the entry's id in the floor manifests.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorGrade {
    /// The floor entry id (an axis, substance, or element id), matching the manifests.
    pub id: String,
    /// The seven-tag grade: `measured`, `derived`, `estimator`, `closure`, `authored`, `contingency`, or
    /// `written_state`.
    pub grade: String,
    /// For a `derived` grade, the named floor quantities or laws it computes from.
    #[serde(default)]
    pub derived_from: Vec<String>,
    /// A bulk material property stored on the substance rather than derived from its components: a
    /// generality defect orthogonal to the refutability provenance, separately countable, resolved by the
    /// materials buildout relocating the measured floor down to the components.
    #[serde(default)]
    pub derive_first_defect: bool,
    /// The ground was ambiguous between two tags or mis-bucketed; a disclosed limit, not a directional lean.
    #[serde(default)]
    pub unsettled: bool,
    /// THE LAST HOP of the provenance DAG: the vendored source ids this entry's value traces to, resolving
    /// into the consolidated source registry.
    ///
    /// Without it the DAG stops one step short of the evidence. [`Self::derived_from`] walks a derived
    /// quantity back to the floor quantities it computes from, and `unified_provenance` joins those with
    /// the calibration side, but a LEAF has nowhere left to point: nothing connects a measured floor value
    /// to the paper, table, or dataset it was read out of, and no gate can ask whether it traces to a held
    /// primary. This closes that hop, so a walk from a derived world quantity can reach a checksummed,
    /// archived source rather than ending at a bare id.
    ///
    /// EMPTY IS THE HONEST DEFAULT, not a claim of no provenance. The existing register predates this
    /// field and carries citations in prose elsewhere; an empty list here means "not yet linked", never
    /// "unsourced". Enforcement ratchets separately once the source registry lands, the same shape the
    /// constructor and derives baselines use: grandfather the population, require it of new entries.
    ///
    /// A source whose licence forbids redistribution is still nameable here. The registry entry it points
    /// at holds a citation and a public archive witness rather than bytes (owner ruling), so this field
    /// records the trace either way and the completeness question belongs to the registry, not here.
    #[serde(default)]
    pub sources: Vec<String>,
}

/// The floor grade register, loaded from `crates/physics/data/floor_provenance.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorProvenance {
    /// The per-entry grades. Renamed from the `[[grade]]` TOML array.
    #[serde(default, rename = "grade")]
    pub grades: Vec<FloorGrade>,
}

impl FloorProvenance {
    /// Parse a register from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// The embedded register, built from the crate's data so a caller needs no filesystem path.
    pub fn embedded() -> Result<Self, String> {
        Self::from_toml_str(include_str!("../data/floor_provenance.toml"))
    }

    /// A grade by entry id.
    pub fn grade(&self, id: &str) -> Option<&FloorGrade> {
        self.grades.iter().find(|g| g.id == id)
    }

    /// The AUTHORING SURFACE: the ids graded `closure` or `authored`, the floor's contribution to the honesty
    /// number (the world-content values whose outcomes rest on set-points no laboratory could refute without
    /// running the sim). The floor `derived` entries compute from measured or law inputs with no closure
    /// ancestry, so the declared closure-plus-authored count is the effective count on the floor; the full
    /// DAG-join over the unified calibration-plus-floor register is the calibration-side query. Returned in
    /// register order, deterministic.
    pub fn authoring_surface(&self) -> Vec<&str> {
        self.grades
            .iter()
            .filter(|g| g.grade == "closure" || g.grade == "authored")
            .map(|g| g.id.as_str())
            .collect()
    }

    /// The DERIVE-FIRST / GENERALITY defects: bulk material properties stored on a substance rather than
    /// derived from components, separately countable from the authoring surface (a value can be measured and
    /// a generality defect at once). The materials buildout resolves these. Returned in register order.
    pub fn derive_first_defects(&self) -> Vec<&str> {
        self.grades
            .iter()
            .filter(|g| g.derive_first_defect)
            .map(|g| g.id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_floor_register_parses_and_the_authoring_surface_is_the_reserved_couplings() {
        let reg = FloorProvenance::embedded().expect("the embedded floor grade register parses");
        assert_eq!(
            reg.grades.len(),
            243,
            "every floor value entry is graded (235 axes/substances/elements plus 8 candidate phases, enstatite added)"
        );
        // The floor authoring surface is the handful of genuine owner-reserved biology and chemistry
        // couplings, source-verified; the physics quantity axes and the metrological atomic-weight
        // conventions are measured, not on the authoring surface (the seam PD1 caught and the gate confirmed
        // at source). The number is a QUERY over the register, never a hand-written literal.
        let surface = reg.authoring_surface();
        assert_eq!(
            surface,
            vec![
                "bio.consumer.hill_exponent",
                "bio.decomposition_rate",
                "bio.net_harm",
                "chem.corrosion_susceptibility",
                "chem.solute_affinity",
                "opt.spectral_band",
            ],
            "the floor authoring surface is the six genuine reserved couplings"
        );
        // The derive-first / generality defects (bulk properties stored not derived) are separately
        // countable and non-empty; each is a measured value AND a generality defect.
        assert!(
            !reg.derive_first_defects().is_empty(),
            "the composite-material bulk rows carry the derive-first defect marker"
        );
        // Every grade is one of the seven; a derived grade names its inputs.
        // EIGHT since 2026-07-19: `unverified_measurement_candidate` is the honest tier for a value LABELLED
        // measured with no machine-checkable evidence behind it. See the variant's doc in
        // crates/foundation/src/calibration.rs for why 244 such labels were downgraded rather than baselined.
        const SEVEN: [&str; 8] = [
            "measured",
            "unverified_measurement_candidate",
            "derived",
            "estimator",
            "closure",
            "authored",
            "contingency",
            "written_state",
        ];
        for g in &reg.grades {
            assert!(
                SEVEN.contains(&g.grade.as_str()),
                "{} has a bad grade '{}'",
                g.id,
                g.grade
            );
            if g.grade == "derived" {
                assert!(
                    !g.derived_from.is_empty(),
                    "{} is derived but names no inputs",
                    g.id
                );
            }
        }
    }
}
