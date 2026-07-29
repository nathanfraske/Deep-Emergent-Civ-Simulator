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

//! Candidate provenance audit for the broad physics data population.
//!
//! This is not the canonical absolute floor and cannot admit a value. The canonical floor contains only the
//! entries in the generated four-tier by seven-mark ledger inventory. This sidecar keeps migration candidates
//! auditable: it maps each broad physics-data row to one of the seven accounting marks, or to the fail-closed
//! `unverified_measurement_candidate` sentinel when `[M]` has not been established. It also carries the
//! `derive_first_defect` marker: a bulk material property stored on a substance rather than derived from its
//! mineral or element components is an admit-the-alien defect orthogonal to the candidate evidence claim.
//! The marker never promotes a row to `[M]`. A `[C]` or `[A]` record here is an inadmissible candidate, never
//! a canonical input.

use serde::Deserialize;

/// One broad physics-data candidate's provenance audit record.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorCandidateRecord {
    /// The broad-data entry id (an axis, substance, element, or phase id), matching the manifests.
    pub id: String,
    /// One of the seven canonical provenance spellings, or the fail-closed measurement-candidate sentinel.
    #[serde(rename = "grade")]
    pub status: String,
    /// For a `derived` status, the named candidate quantities or laws it computes from.
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
    /// quantity back to the candidate quantities it computes from, but a LEAF has nowhere left to point:
    /// nothing connects a measured candidate value
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

/// Candidate audit records loaded from `crates/physics/data/floor_provenance.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct FloorCandidateRegistry {
    /// The per-entry candidate records. The TOML array retains its historical `[[grade]]` spelling.
    #[serde(default, rename = "grade")]
    pub records: Vec<FloorCandidateRecord>,
}

impl FloorCandidateRegistry {
    /// Parse a register from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, String> {
        toml::from_str(s).map_err(|e| e.to_string())
    }

    /// The embedded register, built from the crate's data so a caller needs no filesystem path.
    pub fn embedded() -> Result<Self, String> {
        Self::from_toml_str(include_str!("../data/floor_provenance.toml"))
    }

    /// A candidate audit record by entry id.
    pub fn record(&self, id: &str) -> Option<&FloorCandidateRecord> {
        self.records.iter().find(|record| record.id == id)
    }

    /// Candidate rows marked closure or authored. These are excluded from the initial absolute floor and remain
    /// visible here so an old reserved coupling cannot be mistaken for an admitted input.
    pub fn inadmissible_candidates(&self) -> Vec<&str> {
        self.records
            .iter()
            .filter(|record| record.status == "closure" || record.status == "authored")
            .map(|record| record.id.as_str())
            .collect()
    }

    /// The DERIVE-FIRST / GENERALITY defects: bulk material properties stored on a substance rather than
    /// derived from components, separately countable from the inadmissible-candidate set (a value can be measured
    /// and a generality defect at once). The materials buildout resolves these. Returned in register order.
    pub fn derive_first_defects(&self) -> Vec<&str> {
        self.records
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
    fn the_embedded_candidate_register_keeps_reserved_couplings_inadmissible() {
        let reg =
            FloorCandidateRegistry::embedded().expect("the embedded candidate register parses");
        assert_eq!(
            reg.records.len(),
            203,
            "every broad physics-data candidate has an audit record"
        );
        // These rows remain visible precisely because closure or authored candidates cannot enter the initial
        // absolute floor. The canonical inventory is the separate authority.
        let surface = reg.inadmissible_candidates();
        assert_eq!(
            surface,
            vec!["chem.corrosion_susceptibility", "chem.solute_affinity",],
            "the two active reserved couplings remain inadmissible candidates"
        );
        // The derive-first / generality defects (bulk properties stored not derived) are separately
        // countable and non-empty; each is a measured value AND a generality defect.
        assert!(
            !reg.derive_first_defects().is_empty(),
            "the composite-material bulk rows carry the derive-first defect marker"
        );
        // The accounting taxonomy stays exactly seven. The measurement-candidate spelling is an audit sentinel,
        // not an eighth mark and never `[M]`.
        const SEVEN: [&str; 7] = [
            "measured",
            "derived",
            "estimator",
            "closure",
            "authored",
            "contingency",
            "written_state",
        ];
        for record in &reg.records {
            assert!(
                SEVEN.contains(&record.status.as_str())
                    || record.status == "unverified_measurement_candidate",
                "{} has an unknown candidate status '{}'",
                record.id,
                record.status
            );
            if record.status == "derived" {
                assert!(
                    !record.derived_from.is_empty(),
                    "{} is derived but names no inputs",
                    record.id
                );
            }
        }
    }
}
