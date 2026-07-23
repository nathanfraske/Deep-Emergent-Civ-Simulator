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

//! The canonical provenance ledger.
//!
//! Every accounted value belongs to one of four tiers and carries one of seven
//! provenance types. The tiers are the broad layers settled in
//! `docs/PROVENANCE_LEDGER.md`: universal constants and laws, reference
//! evidence, residue, and generated realization contingency. The seven types are
//! derived `[D]`, measured `[M]`, estimator `[E]`, closure `[C]`, authored `[A]`,
//! written state `[W]`, and contingency `[X]`.
//!
//! This crate contains no world values and no runtime defaults. It only makes the
//! classification, dependency graph, and worst-case provenance join mechanical.
//! A valid ledger is an accounting result, not permission to enter the planetary
//! run path. [`AbsolutePhysicsFloor`] applies that narrower admission contract.

mod admission;

pub use admission::{
    AbsolutePhysicsFloor, ChaosProtocolReceipt, ChaosRegimeReceipt, DerivationExhaustionReceipt,
    FloorAdmissionError, GapLawReceipt, ResidualLawReceipt,
};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// A value's provenance under the owner's canonical seven-type taxonomy.
///
/// `UnverifiedMeasurementCandidate` and `Unclassified` are fail-closed audit
/// states, not additional provenance types. A canonical [`Ledger`] rejects both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Provenance {
    /// `[D]` Computed from named inputs by a named law.
    Derived,
    /// `[M]` Refutable by observation without running the simulator.
    Measured,
    /// A measurement claim that does not yet carry machine-linked evidence.
    UnverifiedMeasurementCandidate,
    /// `[E]` A banded approximation over measured or derived inputs.
    Estimator,
    /// `[C]` A model closure, barred from the initial absolute floor.
    Closure,
    /// `[A]` A hand-authored magnitude, barred from the initial absolute floor.
    Authored,
    /// `[W]` Computed history generated within the causal run.
    WrittenState,
    /// `[X]` A realization contingency generated from an admitted measure.
    Contingency,
    /// Missing classification. This is never legal in a canonical ledger.
    Unclassified,
}

impl Provenance {
    /// The seven canonical types, in stable display order.
    pub const CANONICAL: [Self; 7] = [
        Self::Derived,
        Self::Measured,
        Self::Estimator,
        Self::Closure,
        Self::Authored,
        Self::WrittenState,
        Self::Contingency,
    ];

    /// Whether this is one of the seven canonical provenance types.
    pub const fn is_canonical(self) -> bool {
        matches!(
            self,
            Self::Derived
                | Self::Measured
                | Self::Estimator
                | Self::Closure
                | Self::Authored
                | Self::WrittenState
                | Self::Contingency
        )
    }

    /// The stable ledger spelling.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Derived => "derived",
            Self::Measured => "measured",
            Self::UnverifiedMeasurementCandidate => "unverified_measurement_candidate",
            Self::Estimator => "estimator",
            Self::Closure => "closure",
            Self::Authored => "authored",
            Self::WrittenState => "written_state",
            Self::Contingency => "contingency",
            Self::Unclassified => "",
        }
    }

    /// The canonical bracket tag, when this is one of the seven accounting
    /// provenance types.
    pub const fn bracket_tag(self) -> Option<&'static str> {
        match self {
            Self::Derived => Some("[D]"),
            Self::Measured => Some("[M]"),
            Self::Estimator => Some("[E]"),
            Self::Closure => Some("[C]"),
            Self::Authored => Some("[A]"),
            Self::WrittenState => Some("[W]"),
            Self::Contingency => Some("[X]"),
            Self::UnverifiedMeasurementCandidate | Self::Unclassified => None,
        }
    }

    /// Parse the stable ledger spelling.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "" => Self::Unclassified,
            "derived" => Self::Derived,
            "measured" => Self::Measured,
            "unverified_measurement_candidate" => Self::UnverifiedMeasurementCandidate,
            "estimator" => Self::Estimator,
            "closure" => Self::Closure,
            "authored" => Self::Authored,
            "written_state" => Self::WrittenState,
            "contingency" => Self::Contingency,
            _ => return None,
        })
    }

    /// Pinnedness rank used by the worst-case DAG join. Lower is less pinned.
    ///
    /// This preserves the rank ordering already enforced by the repository's
    /// unified provenance audit.
    pub const fn rank(self) -> u8 {
        match self {
            Self::Authored => 0,
            Self::Closure => 1,
            Self::Unclassified => 2,
            Self::UnverifiedMeasurementCandidate => 3,
            Self::Estimator => 4,
            Self::Derived => 5,
            Self::WrittenState => 6,
            Self::Contingency => 7,
            Self::Measured => 8,
        }
    }
}

/// The four broad ledger tiers settled for the planetary and stellar substrate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// Tier 1: universal constants, identities, and mechanisms.
    Universal,
    /// Tier 2: reference evidence and compute-once results after admission.
    Reference,
    /// Tier 3: estimators, closures, and residue accounting.
    Residue,
    /// Tier 4: generated realization contingency, never a caller input.
    Contingency,
}

impl Tier {
    /// All four tiers in numeric order.
    pub const ALL: [Self; 4] = [
        Self::Universal,
        Self::Reference,
        Self::Residue,
        Self::Contingency,
    ];

    /// The ledger's one-based tier number.
    pub const fn number(self) -> u8 {
        match self {
            Self::Universal => 1,
            Self::Reference => 2,
            Self::Residue => 3,
            Self::Contingency => 4,
        }
    }

    /// The stable ledger spelling.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::Reference => "reference",
            Self::Residue => "residue",
            Self::Contingency => "contingency",
        }
    }
}

/// One classified value or law in the canonical ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Stable, namespaced identity.
    pub id: String,
    /// Which of the four layers owns the entry.
    pub tier: Tier,
    /// Declared provenance before the transitive worst-case join.
    pub provenance: Provenance,
    /// Named ledger inputs. A derived entry must name at least one.
    pub inputs: Vec<String>,
}

/// A validated, deterministic provenance graph.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    order: Vec<String>,
    entries: BTreeMap<String, Entry>,
}

/// Regenerated counts and membership for every tier and provenance type.
///
/// The matrix is total: all four tiers and all seven canonical types appear even
/// when their count is zero. This prevents an absent class from disappearing
/// from reports and being mistaken for an unmeasured class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    counts: BTreeMap<(Tier, Provenance), usize>,
    members: BTreeMap<(Tier, Provenance), Vec<String>>,
}

impl Inventory {
    /// Count in one matrix cell.
    pub fn count(&self, tier: Tier, provenance: Provenance) -> usize {
        self.counts.get(&(tier, provenance)).copied().unwrap_or(0)
    }

    /// Stable member ids in one matrix cell.
    pub fn members(&self, tier: Tier, provenance: Provenance) -> &[String] {
        self.members
            .get(&(tier, provenance))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Total entries in one tier.
    pub fn tier_total(&self, tier: Tier) -> usize {
        Provenance::CANONICAL
            .into_iter()
            .map(|provenance| self.count(tier, provenance))
            .sum()
    }

    /// Total entries in the inventory.
    pub fn total(&self) -> usize {
        Tier::ALL
            .into_iter()
            .map(|tier| self.tier_total(tier))
            .sum()
    }
}

impl fmt::Display for Inventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ledger_inventory=civsim.ledger.inventory.v2")?;
        writeln!(f, "total={}", self.total())?;
        for tier in Tier::ALL {
            writeln!(f, "tier.{}.name={}", tier.number(), tier.id())?;
            writeln!(f, "tier.{}.total={}", tier.number(), self.tier_total(tier))?;
            for provenance in Provenance::CANONICAL {
                writeln!(
                    f,
                    "tier.{}.{}.tag={}",
                    tier.number(),
                    provenance.tag(),
                    provenance
                        .bracket_tag()
                        .expect("every canonical provenance has a bracket tag")
                )?;
                writeln!(
                    f,
                    "tier.{}.{}={}",
                    tier.number(),
                    provenance.tag(),
                    self.count(tier, provenance)
                )?;
                for id in self.members(tier, provenance) {
                    writeln!(f, "tier.{}.{}.entry={id}", tier.number(), provenance.tag())?;
                }
            }
        }
        Ok(())
    }
}

/// Why a provenance ledger graph is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    EmptyId,
    Duplicate(String),
    NonCanonicalProvenance(String),
    DerivedWithoutInputs(String),
    InputOnLeaf(String),
    MissingInput { id: String, input: String },
    Cycle(String),
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId => write!(f, "a ledger entry has an empty id"),
            Self::Duplicate(id) => write!(f, "duplicate ledger entry: {id}"),
            Self::NonCanonicalProvenance(id) => {
                write!(
                    f,
                    "ledger entry {id} does not carry one of the seven provenance types"
                )
            }
            Self::DerivedWithoutInputs(id) => {
                write!(f, "derived ledger entry {id} names no inputs")
            }
            Self::InputOnLeaf(id) => write!(f, "non-derived ledger entry {id} names inputs"),
            Self::MissingInput { id, input } => {
                write!(f, "ledger entry {id} names missing input {input}")
            }
            Self::Cycle(id) => write!(f, "provenance cycle reaches {id}"),
        }
    }
}

impl std::error::Error for LedgerError {}

impl Ledger {
    /// Build and validate a ledger. Entry order is retained for receipts; lookups
    /// and graph walks use ordered containers.
    pub fn build(entries: impl IntoIterator<Item = Entry>) -> Result<Self, LedgerError> {
        let mut order = Vec::new();
        let mut by_id = BTreeMap::new();
        for entry in entries {
            if entry.id.trim().is_empty() {
                return Err(LedgerError::EmptyId);
            }
            if !entry.provenance.is_canonical() {
                return Err(LedgerError::NonCanonicalProvenance(entry.id));
            }
            if entry.provenance == Provenance::Derived && entry.inputs.is_empty() {
                return Err(LedgerError::DerivedWithoutInputs(entry.id));
            }
            if entry.provenance != Provenance::Derived && !entry.inputs.is_empty() {
                return Err(LedgerError::InputOnLeaf(entry.id));
            }
            let id = entry.id.clone();
            if by_id.insert(id.clone(), entry).is_some() {
                return Err(LedgerError::Duplicate(id));
            }
            order.push(id);
        }

        for id in &order {
            for input in &by_id[id].inputs {
                if !by_id.contains_key(input) {
                    return Err(LedgerError::MissingInput {
                        id: id.clone(),
                        input: input.clone(),
                    });
                }
            }
        }

        let ledger = Self {
            order,
            entries: by_id,
        };
        ledger.validate_acyclic()?;
        Ok(ledger)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether the ledger contains no entries.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Entries in their declared order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.order.iter().map(|id| &self.entries[id])
    }

    /// Look up an entry by stable id.
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.get(id)
    }

    /// Regenerate the complete four-tier by seven-type inventory from this
    /// ledger. No separately maintained count is accepted.
    pub fn inventory(&self) -> Inventory {
        let mut counts = BTreeMap::new();
        let mut members = BTreeMap::new();
        for tier in Tier::ALL {
            for provenance in Provenance::CANONICAL {
                counts.insert((tier, provenance), 0);
                members.insert((tier, provenance), Vec::new());
            }
        }
        for entry in self.entries() {
            *counts
                .get_mut(&(entry.tier, entry.provenance))
                .expect("the complete matrix contains every canonical cell") += 1;
            members
                .get_mut(&(entry.tier, entry.provenance))
                .expect("the complete matrix contains every canonical cell")
                .push(entry.id.clone());
        }
        Inventory { counts, members }
    }

    /// Effective provenance after the transitive worst-case join.
    pub fn effective_provenance(&self, id: &str) -> Option<Provenance> {
        let mut path = BTreeSet::new();
        self.effective_inner(id, &mut path)
    }

    /// Entries whose effective provenance rests on authored or closure inputs.
    pub fn authoring_surface(&self) -> Vec<&str> {
        self.order
            .iter()
            .filter_map(|id| {
                let provenance = self.effective_provenance(id)?;
                matches!(provenance, Provenance::Authored | Provenance::Closure)
                    .then_some(id.as_str())
            })
            .collect()
    }

    fn effective_inner(&self, id: &str, path: &mut BTreeSet<String>) -> Option<Provenance> {
        let entry = self.entries.get(id)?;
        if entry.provenance != Provenance::Derived {
            return Some(entry.provenance);
        }
        if !path.insert(id.to_owned()) {
            return Some(Provenance::Unclassified);
        }
        let mut worst = entry.provenance;
        for input in &entry.inputs {
            let candidate = self.effective_inner(input, path)?;
            if candidate.rank() < worst.rank() {
                worst = candidate;
            }
        }
        path.remove(id);
        Some(worst)
    }

    fn validate_acyclic(&self) -> Result<(), LedgerError> {
        fn visit(
            ledger: &Ledger,
            id: &str,
            visiting: &mut BTreeSet<String>,
            complete: &mut BTreeSet<String>,
        ) -> Result<(), LedgerError> {
            if complete.contains(id) {
                return Ok(());
            }
            if !visiting.insert(id.to_owned()) {
                return Err(LedgerError::Cycle(id.to_owned()));
            }
            for input in &ledger.entries[id].inputs {
                visit(ledger, input, visiting, complete)?;
            }
            visiting.remove(id);
            complete.insert(id.to_owned());
            Ok(())
        }

        let mut visiting = BTreeSet::new();
        let mut complete = BTreeSet::new();
        for id in &self.order {
            visit(self, id, &mut visiting, &mut complete)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taxonomy_is_exactly_seven_types_and_four_tiers() {
        assert_eq!(Provenance::CANONICAL.len(), 7);
        assert_eq!(Tier::ALL.len(), 4);
        for (index, tier) in Tier::ALL.into_iter().enumerate() {
            assert_eq!(tier.number(), index as u8 + 1);
        }
    }

    #[test]
    fn canonical_long_and_bracket_tags_are_ordered_bijections() {
        let expected = [
            (Provenance::Derived, "derived", "[D]"),
            (Provenance::Measured, "measured", "[M]"),
            (Provenance::Estimator, "estimator", "[E]"),
            (Provenance::Closure, "closure", "[C]"),
            (Provenance::Authored, "authored", "[A]"),
            (Provenance::WrittenState, "written_state", "[W]"),
            (Provenance::Contingency, "contingency", "[X]"),
        ];
        let mut long_tags = BTreeSet::new();
        let mut bracket_tags = BTreeSet::new();

        for (provenance, (expected_provenance, long_tag, bracket_tag)) in
            Provenance::CANONICAL.into_iter().zip(expected)
        {
            assert_eq!(provenance, expected_provenance);
            assert_eq!(provenance.tag(), long_tag);
            assert_eq!(provenance.bracket_tag(), Some(bracket_tag));
            assert_eq!(Provenance::from_tag(long_tag), Some(provenance));
            assert!(long_tags.insert(long_tag));
            assert!(bracket_tags.insert(bracket_tag));
        }

        assert_eq!(long_tags.len(), Provenance::CANONICAL.len());
        assert_eq!(bracket_tags.len(), Provenance::CANONICAL.len());
        assert_eq!(Provenance::Unclassified.bracket_tag(), None);
        assert_eq!(
            Provenance::UnverifiedMeasurementCandidate.bracket_tag(),
            None
        );
    }

    #[test]
    fn derived_provenance_inherits_the_worst_input() {
        let ledger = Ledger::build([
            Entry {
                id: "fixture.measured".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
            Entry {
                id: "fixture.closure".into(),
                tier: Tier::Residue,
                provenance: Provenance::Closure,
                inputs: vec![],
            },
            Entry {
                id: "fixture.derived".into(),
                tier: Tier::Universal,
                provenance: Provenance::Derived,
                inputs: vec!["fixture.measured".into(), "fixture.closure".into()],
            },
        ])
        .unwrap();

        assert_eq!(
            ledger.effective_provenance("fixture.derived"),
            Some(Provenance::Closure)
        );
        assert_eq!(
            ledger.authoring_surface(),
            vec!["fixture.closure", "fixture.derived"]
        );
    }

    #[test]
    fn audit_sentinels_and_broken_graphs_refuse() {
        let unclassified = Ledger::build([Entry {
            id: "fixture.unclassified".into(),
            tier: Tier::Universal,
            provenance: Provenance::Unclassified,
            inputs: vec![],
        }]);
        assert!(matches!(
            unclassified,
            Err(LedgerError::NonCanonicalProvenance(_))
        ));

        let missing = Ledger::build([Entry {
            id: "fixture.derived".into(),
            tier: Tier::Universal,
            provenance: Provenance::Derived,
            inputs: vec!["fixture.absent".into()],
        }]);
        assert!(matches!(missing, Err(LedgerError::MissingInput { .. })));
    }

    #[test]
    fn inventory_regenerates_the_complete_tier_by_type_matrix() {
        let ledger = Ledger::build([
            Entry {
                id: "fixture.constant".into(),
                tier: Tier::Universal,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
            Entry {
                id: "fixture.world".into(),
                tier: Tier::Contingency,
                provenance: Provenance::Contingency,
                inputs: vec![],
            },
        ])
        .unwrap();
        let inventory = ledger.inventory();

        assert_eq!(inventory.total(), 2);
        assert_eq!(inventory.tier_total(Tier::Universal), 1);
        assert_eq!(inventory.count(Tier::Universal, Provenance::Measured), 1);
        assert_eq!(inventory.count(Tier::Universal, Provenance::Derived), 0);
        assert_eq!(
            inventory.members(Tier::Contingency, Provenance::Contingency),
            &["fixture.world".to_string()]
        );
        let rendered = inventory.to_string();
        for tier in Tier::ALL {
            for provenance in Provenance::CANONICAL {
                let tag = provenance
                    .bracket_tag()
                    .expect("canonical provenance has a bracket tag");
                assert!(rendered.contains(&format!(
                    "tier.{}.{}.tag={tag}",
                    tier.number(),
                    provenance.tag()
                )));
            }
        }
        assert!(rendered.contains("tier.1.derived=0"));
        assert!(rendered.contains("tier.4.contingency=1"));
        assert!(rendered.contains("tier.4.contingency.entry=fixture.world"));
    }
}
