use crate::{Entry, Ledger, Provenance, Tier};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// The Chaos Protocol branch carried by the Gap Law.
///
/// A deterministic trajectory is valid only while admitted input bands remain
/// resolved. Dynamical systems carry an explicit regime partition so mixed or
/// changing behavior cannot be compressed into one global label. A regime
/// whose divergence becomes sub-resolution must use a derived stationary
/// measure plus a deterministic realization coordinate, never one fixed path
/// presented as a uniquely derived physical history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosProtocolReceipt {
    /// The admitted item is not a dynamical evolution or branch selection.
    NotApplicable { basis: String },
    /// One or more physical regimes connected by a declared transition law.
    Dynamical {
        classification: String,
        regime_partition: String,
        transition_law: String,
        regimes: Vec<ChaosRegimeReceipt>,
    },
}

/// One domain in a dynamical Gap Law partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosRegimeReceipt {
    /// Input uncertainty remains resolved over the claimed evolution.
    ResolvedTrajectory {
        validity_domain: String,
        resolution_bound: String,
        evolution_postcondition: String,
        exact_replay: String,
    },
    /// Sub-resolution divergence requires evolution by physical measure.
    SubresolutionMeasure {
        validity_domain: String,
        stationary_measure: String,
        conservation_projection: String,
        stability_postcondition: String,
        coordinate_discipline: String,
        exact_replay: String,
    },
}

impl ChaosRegimeReceipt {
    pub const fn kind_id(&self) -> &'static str {
        match self {
            Self::ResolvedTrajectory { .. } => "resolved_trajectory",
            Self::SubresolutionMeasure { .. } => "subresolution_measure",
        }
    }

    pub fn evidence(&self) -> Vec<(&'static str, &str)> {
        match self {
            Self::ResolvedTrajectory {
                validity_domain,
                resolution_bound,
                evolution_postcondition,
                exact_replay,
            } => vec![
                ("chaos.regime.validity_domain", validity_domain),
                ("chaos.regime.resolution_bound", resolution_bound),
                (
                    "chaos.regime.evolution_postcondition",
                    evolution_postcondition,
                ),
                ("chaos.regime.exact_replay", exact_replay),
            ],
            Self::SubresolutionMeasure {
                validity_domain,
                stationary_measure,
                conservation_projection,
                stability_postcondition,
                coordinate_discipline,
                exact_replay,
            } => vec![
                ("chaos.regime.validity_domain", validity_domain),
                ("chaos.regime.stationary_measure", stationary_measure),
                (
                    "chaos.regime.conservation_projection",
                    conservation_projection,
                ),
                (
                    "chaos.regime.stability_postcondition",
                    stability_postcondition,
                ),
                ("chaos.regime.coordinate_discipline", coordinate_discipline),
                ("chaos.regime.exact_replay", exact_replay),
            ],
        }
    }
}

impl ChaosProtocolReceipt {
    /// Stable serialization tag for the protocol branch.
    pub const fn kind_id(&self) -> &'static str {
        match self {
            Self::NotApplicable { .. } => "not_applicable",
            Self::Dynamical { .. } => "dynamical",
        }
    }

    /// Ordered evidence fields for admission fingerprints and replay receipts.
    pub fn evidence(&self) -> Vec<(&'static str, &str)> {
        match self {
            Self::NotApplicable { basis } => vec![("chaos.basis", basis)],
            Self::Dynamical {
                classification,
                regime_partition,
                transition_law,
                regimes,
            } => {
                let mut evidence = vec![
                    ("chaos.classification", classification.as_str()),
                    ("chaos.regime_partition", regime_partition.as_str()),
                    ("chaos.transition_law", transition_law.as_str()),
                ];
                if regimes.is_empty() {
                    evidence.push(("chaos.regime", ""));
                } else {
                    for regime in regimes {
                        evidence.push(("chaos.regime", regime.kind_id()));
                        evidence.extend(regime.evidence());
                    }
                }
                evidence
            }
        }
    }
}

/// Evidence that the Gap Law obligations were considered for one irreducible
/// initial-floor leaf at any tier.
///
/// These are evidence-bearing statements, not boolean claims that the ledger
/// can prove. An admission refuses when any obligation is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapLawReceipt {
    pub reference_validity: String,
    pub gap_dispatch: String,
    pub smooth_systematics: String,
    pub scale_free_limit: String,
    pub chaos_protocol: ChaosProtocolReceipt,
}

impl GapLawReceipt {
    fn evidence(&self) -> Vec<(&'static str, &str)> {
        let mut evidence: Vec<(&'static str, &str)> = vec![
            ("gap.reference_validity", self.reference_validity.as_str()),
            ("gap.gap_dispatch", self.gap_dispatch.as_str()),
            ("gap.smooth_systematics", self.smooth_systematics.as_str()),
            ("gap.scale_free_limit", self.scale_free_limit.as_str()),
        ];
        evidence.push(("gap.chaos_protocol", self.chaos_protocol.kind_id()));
        evidence.extend(self.chaos_protocol.evidence());
        evidence
    }
}

/// Evidence that the Residual Law obligations were considered for one
/// irreducible initial-floor leaf at any tier.
///
/// The dimensional-analysis field supplements the mechanical Buckingham-Pi
/// budget on [`DerivationExhaustionReceipt`]. It does not replace that budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualLawReceipt {
    pub conservation: String,
    pub disequilibrium: String,
    pub fluctuation_dissipation: String,
    pub dimensional_analysis: String,
}

impl ResidualLawReceipt {
    fn evidence(&self) -> [(&'static str, &str); 4] {
        [
            ("residual.conservation", &self.conservation),
            ("residual.disequilibrium", &self.disequilibrium),
            (
                "residual.fluctuation_dissipation",
                &self.fluctuation_dissipation,
            ),
            ("residual.dimensional_analysis", &self.dimensional_analysis),
        ]
    }
}

/// Auditable proof obligation for admitting one irreducible leaf after
/// derivation has been exhausted.
///
/// `buckingham_pi_groups` is the maximum number of residual slots admitted for
/// the named phenomenon. Every receipt for that phenomenon must declare the
/// same budget, and each slot may be occupied at most once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationExhaustionReceipt {
    pub entry_id: String,
    pub phenomenon: String,
    pub derivation_attempts: Vec<String>,
    pub residual_slot: String,
    pub buckingham_pi_groups: usize,
    pub gap_law: GapLawReceipt,
    pub residual_law: ResidualLawReceipt,
}

/// A provenance graph admitted as the immutable initial physics floor.
///
/// Construction consumes a [`Ledger`] so it cannot be changed after admission.
/// The planetary runner accepts this type rather than accepting generic ledger
/// accounting as authorization. The wrapper carries identities, ancestry, and
/// receipts only. It has no API that binds a caller-supplied magnitude to an
/// admitted identity.
#[derive(Debug, Clone)]
pub struct AbsolutePhysicsFloor {
    ledger: Ledger,
    receipts: BTreeMap<String, DerivationExhaustionReceipt>,
    pi_budgets: BTreeMap<String, usize>,
}

impl AbsolutePhysicsFloor {
    /// Validate and admit an initial absolute floor.
    ///
    /// Every non-derived leaf, including a Universal `[M]` candidate, must carry
    /// a complete exhaustion receipt. Tier and citation never bypass the
    /// derive-first obligation. Values derived from named ancestry are admitted
    /// only when the ledger proves nonempty leaf ancestry. Authored values,
    /// closures, written state, and caller-supplied contingencies are never
    /// initial-floor inputs.
    pub fn admit(
        ledger: Ledger,
        receipts: impl IntoIterator<Item = DerivationExhaustionReceipt>,
    ) -> Result<Self, FloorAdmissionError> {
        if ledger.is_empty() {
            return Err(FloorAdmissionError::EmptyFloor);
        }

        for entry in ledger.entries() {
            validate_entry(&ledger, entry)?;
        }

        let mut receipts_by_entry = BTreeMap::new();
        let mut pi_budgets = BTreeMap::new();
        let mut residual_slots = BTreeSet::new();

        for receipt in receipts {
            let Some(entry) = ledger.get(&receipt.entry_id) else {
                return Err(FloorAdmissionError::ReceiptForMissingEntry(
                    receipt.entry_id,
                ));
            };
            if !requires_exhaustion_receipt(entry) {
                return Err(FloorAdmissionError::ReceiptForIneligibleEntry(
                    receipt.entry_id,
                ));
            }
            if receipts_by_entry.contains_key(&receipt.entry_id) {
                return Err(FloorAdmissionError::DuplicateReceipt(receipt.entry_id));
            }
            if receipt.derivation_attempts.is_empty() {
                return Err(FloorAdmissionError::MissingDerivationAttempts(
                    receipt.entry_id,
                ));
            }
            for attempt in &receipt.derivation_attempts {
                require_evidence(&receipt.entry_id, "derivation_attempt", attempt)?;
            }
            require_evidence(&receipt.entry_id, "phenomenon", &receipt.phenomenon)?;
            require_evidence(&receipt.entry_id, "residual_slot", &receipt.residual_slot)?;
            for (field, evidence) in receipt.gap_law.evidence() {
                require_evidence(&receipt.entry_id, field, evidence)?;
            }
            for (field, evidence) in receipt.residual_law.evidence() {
                require_evidence(&receipt.entry_id, field, evidence)?;
            }

            let phenomenon = receipt.phenomenon.trim().to_owned();
            let residual_slot = receipt.residual_slot.trim().to_owned();
            match pi_budgets.get(&phenomenon) {
                Some(expected) if *expected != receipt.buckingham_pi_groups => {
                    return Err(FloorAdmissionError::InconsistentBuckinghamPiBudget {
                        phenomenon,
                        expected: *expected,
                        found: receipt.buckingham_pi_groups,
                    });
                }
                Some(_) => {}
                None => {
                    pi_budgets.insert(phenomenon.clone(), receipt.buckingham_pi_groups);
                }
            }
            if !residual_slots.insert((phenomenon.clone(), residual_slot.clone())) {
                return Err(FloorAdmissionError::DuplicateResidualSlot {
                    phenomenon,
                    residual_slot,
                });
            }
            receipts_by_entry.insert(receipt.entry_id.clone(), receipt);
        }

        for entry in ledger
            .entries()
            .filter(|entry| requires_exhaustion_receipt(entry))
        {
            if !receipts_by_entry.contains_key(&entry.id) {
                return Err(FloorAdmissionError::MissingDerivationExhaustionReceipt(
                    entry.id.clone(),
                ));
            }
        }

        let mut admitted_by_phenomenon = BTreeMap::new();
        for receipt in receipts_by_entry.values() {
            *admitted_by_phenomenon
                .entry(receipt.phenomenon.trim().to_owned())
                .or_insert(0_usize) += 1;
        }
        for (phenomenon, admitted) in admitted_by_phenomenon {
            let budget = pi_budgets[&phenomenon];
            if admitted > budget {
                return Err(FloorAdmissionError::BuckinghamPiBudgetExceeded {
                    phenomenon,
                    admitted,
                    budget,
                });
            }
        }

        Ok(Self {
            ledger,
            receipts: receipts_by_entry,
            pi_budgets,
        })
    }

    /// Number of admitted accounting entries.
    pub fn len(&self) -> usize {
        self.ledger.len()
    }

    /// Whether the admitted accounting floor contains no entries.
    pub fn is_empty(&self) -> bool {
        self.ledger.is_empty()
    }

    /// Admitted entries in their declared order.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.ledger.entries()
    }

    /// Look up an admitted entry by stable identity.
    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.ledger.get(id)
    }

    /// Look up the exhaustion receipt for an admitted irreducible leaf.
    pub fn receipt(&self, id: &str) -> Option<&DerivationExhaustionReceipt> {
        self.receipts.get(id)
    }

    /// Look up the declared Buckingham-Pi budget for a phenomenon.
    pub fn buckingham_pi_budget(&self, phenomenon: &str) -> Option<usize> {
        self.pi_budgets.get(phenomenon.trim()).copied()
    }
}

fn validate_entry(ledger: &Ledger, entry: &Entry) -> Result<(), FloorAdmissionError> {
    if entry.tier == Tier::Contingency || entry.provenance == Provenance::Contingency {
        return Err(FloorAdmissionError::CallerSuppliedContingency {
            entry_id: entry.id.clone(),
        });
    }
    if matches!(
        entry.provenance,
        Provenance::Authored | Provenance::Closure | Provenance::WrittenState
    ) {
        return Err(FloorAdmissionError::ForbiddenInitialProvenance {
            entry_id: entry.id.clone(),
            provenance: entry.provenance,
        });
    }
    if entry.provenance == Provenance::Derived {
        if !has_leaf_ancestry(ledger, &entry.id) {
            return Err(FloorAdmissionError::DerivedWithoutLeafAncestry(
                entry.id.clone(),
            ));
        }
        return Ok(());
    }
    if entry.tier == Tier::Universal && entry.provenance != Provenance::Measured {
        return Err(FloorAdmissionError::IneligibleUniversalLeaf {
            entry_id: entry.id.clone(),
            provenance: entry.provenance,
        });
    }
    Ok(())
}

fn has_leaf_ancestry(ledger: &Ledger, id: &str) -> bool {
    let Some(entry) = ledger.get(id) else {
        return false;
    };
    if entry.provenance != Provenance::Derived {
        return true;
    }
    !entry.inputs.is_empty()
        && entry
            .inputs
            .iter()
            .all(|input| has_leaf_ancestry(ledger, input))
}

fn requires_exhaustion_receipt(entry: &Entry) -> bool {
    entry.provenance != Provenance::Derived
}

fn require_evidence(
    entry_id: &str,
    field: &'static str,
    evidence: &str,
) -> Result<(), FloorAdmissionError> {
    if evidence.trim().is_empty() {
        Err(FloorAdmissionError::MissingReceiptEvidence {
            entry_id: entry_id.to_owned(),
            field,
        })
    } else {
        Ok(())
    }
}

/// Why an accounting ledger cannot be admitted as the initial absolute floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorAdmissionError {
    EmptyFloor,
    CallerSuppliedContingency {
        entry_id: String,
    },
    ForbiddenInitialProvenance {
        entry_id: String,
        provenance: Provenance,
    },
    IneligibleUniversalLeaf {
        entry_id: String,
        provenance: Provenance,
    },
    DerivedWithoutLeafAncestry(String),
    ReceiptForMissingEntry(String),
    ReceiptForIneligibleEntry(String),
    DuplicateReceipt(String),
    MissingDerivationAttempts(String),
    MissingReceiptEvidence {
        entry_id: String,
        field: &'static str,
    },
    MissingDerivationExhaustionReceipt(String),
    InconsistentBuckinghamPiBudget {
        phenomenon: String,
        expected: usize,
        found: usize,
    },
    DuplicateResidualSlot {
        phenomenon: String,
        residual_slot: String,
    },
    BuckinghamPiBudgetExceeded {
        phenomenon: String,
        admitted: usize,
        budget: usize,
    },
}

impl fmt::Display for FloorAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFloor => write!(f, "the absolute physics floor contains no entries"),
            Self::CallerSuppliedContingency { entry_id } => write!(
                f,
                "initial floor entry {entry_id} is a caller-supplied contingency"
            ),
            Self::ForbiddenInitialProvenance {
                entry_id,
                provenance,
            } => write!(
                f,
                "initial floor entry {entry_id} has forbidden provenance {}",
                provenance.tag()
            ),
            Self::IneligibleUniversalLeaf {
                entry_id,
                provenance,
            } => write!(
                f,
                "universal floor leaf {entry_id} has ineligible provenance {}",
                provenance.tag()
            ),
            Self::DerivedWithoutLeafAncestry(id) => {
                write!(f, "derived floor entry {id} has no nonempty leaf ancestry")
            }
            Self::ReceiptForMissingEntry(id) => {
                write!(f, "derivation-exhaustion receipt names absent entry {id}")
            }
            Self::ReceiptForIneligibleEntry(id) => write!(
                f,
                "derivation-exhaustion receipt names non-irreducible entry {id}"
            ),
            Self::DuplicateReceipt(id) => {
                write!(f, "entry {id} has more than one exhaustion receipt")
            }
            Self::MissingDerivationAttempts(id) => {
                write!(f, "entry {id} names no attempted derivations")
            }
            Self::MissingReceiptEvidence { entry_id, field } => {
                write!(f, "entry {entry_id} has no evidence for {field}")
            }
            Self::MissingDerivationExhaustionReceipt(id) => write!(
                f,
                "irreducible floor leaf {id} has no derivation-exhaustion receipt"
            ),
            Self::InconsistentBuckinghamPiBudget {
                phenomenon,
                expected,
                found,
            } => write!(
                f,
                "phenomenon {phenomenon} declares inconsistent Buckingham-Pi budgets {expected} and {found}"
            ),
            Self::DuplicateResidualSlot {
                phenomenon,
                residual_slot,
            } => write!(
                f,
                "phenomenon {phenomenon} assigns residual slot {residual_slot} more than once"
            ),
            Self::BuckinghamPiBudgetExceeded {
                phenomenon,
                admitted,
                budget,
            } => write!(
                f,
                "phenomenon {phenomenon} admits {admitted} residual inputs against a Buckingham-Pi budget of {budget}"
            ),
        }
    }
}

impl std::error::Error for FloorAdmissionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence_receipt(
        entry_id: &str,
        phenomenon: &str,
        residual_slot: &str,
        buckingham_pi_groups: usize,
    ) -> DerivationExhaustionReceipt {
        DerivationExhaustionReceipt {
            entry_id: entry_id.into(),
            phenomenon: phenomenon.into(),
            derivation_attempts: vec!["named derivation search exhausted".into()],
            residual_slot: residual_slot.into(),
            buckingham_pi_groups,
            gap_law: GapLawReceipt {
                reference_validity: "reference-domain preflight recorded".into(),
                gap_dispatch: "gap branches recorded".into(),
                smooth_systematics: "smooth-systematics check recorded".into(),
                scale_free_limit: "scale-free limit check recorded".into(),
                chaos_protocol: ChaosProtocolReceipt::NotApplicable {
                    basis: "fixture has no dynamical branch".into(),
                },
            },
            residual_law: ResidualLawReceipt {
                conservation: "conservation residual disposition recorded".into(),
                disequilibrium: "freezer or live-flux disposition recorded".into(),
                fluctuation_dissipation: "partner disposition recorded".into(),
                dimensional_analysis: "dimensionless-group derivation recorded".into(),
            },
        }
    }

    fn universal_leaf(id: &str) -> Entry {
        Entry {
            id: id.into(),
            tier: Tier::Universal,
            provenance: Provenance::Measured,
            inputs: vec![],
        }
    }

    #[test]
    fn universal_measured_requires_exhaustion_and_can_anchor_derived_ancestry() {
        let ledger = Ledger::build([
            universal_leaf("fundamental.fixture"),
            Entry {
                id: "composite.fixture".into(),
                tier: Tier::Universal,
                provenance: Provenance::Derived,
                inputs: vec!["fundamental.fixture".into()],
            },
        ])
        .unwrap();

        let receipt = evidence_receipt(
            "fundamental.fixture",
            "fixture.universal",
            "fixture.invariant",
            1,
        );
        let floor = AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap();
        assert_eq!(floor.len(), 2);
        assert!(!floor.is_empty());
        assert!(floor.get("composite.fixture").is_some());
    }

    #[test]
    fn cited_authored_value_is_rejected_even_with_an_exhaustion_receipt() {
        let ledger = Ledger::build([
            universal_leaf("fundamental.fixture"),
            Entry {
                id: "reference.cited_authored".into(),
                tier: Tier::Reference,
                provenance: Provenance::Authored,
                inputs: vec![],
            },
        ])
        .unwrap();
        let receipt = evidence_receipt(
            "reference.cited_authored",
            "fixture.phenomenon",
            "fixture.slot",
            1,
        );

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap_err(),
            FloorAdmissionError::ForbiddenInitialProvenance {
                entry_id: "reference.cited_authored".into(),
                provenance: Provenance::Authored,
            }
        );
    }

    #[test]
    fn written_state_and_closure_are_not_initial_floor_inputs() {
        for provenance in [Provenance::WrittenState, Provenance::Closure] {
            let ledger = Ledger::build([Entry {
                id: "residue.forbidden".into(),
                tier: Tier::Residue,
                provenance,
                inputs: vec![],
            }])
            .unwrap();

            assert!(matches!(
                AbsolutePhysicsFloor::admit(ledger, []),
                Err(FloorAdmissionError::ForbiddenInitialProvenance {
                    provenance: found,
                    ..
                }) if found == provenance
            ));
        }
    }

    #[test]
    fn a_caller_cannot_supply_contingency_to_the_initial_floor() {
        let ledger = Ledger::build([Entry {
            id: "world.fixture".into(),
            tier: Tier::Contingency,
            provenance: Provenance::Contingency,
            inputs: vec![],
        }])
        .unwrap();

        assert!(matches!(
            AbsolutePhysicsFloor::admit(ledger, []),
            Err(FloorAdmissionError::CallerSuppliedContingency { .. })
        ));
    }

    #[test]
    fn irreducible_reference_requires_derivation_and_law_receipts() {
        let ledger = Ledger::build([Entry {
            id: "reference.fixture".into(),
            tier: Tier::Reference,
            provenance: Provenance::Measured,
            inputs: vec![],
        }])
        .unwrap();

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, []).unwrap_err(),
            FloorAdmissionError::MissingDerivationExhaustionReceipt("reference.fixture".into())
        );
    }

    #[test]
    fn complete_exhaustion_receipt_admits_an_irreducible_reference() {
        let ledger = Ledger::build([Entry {
            id: "reference.fixture".into(),
            tier: Tier::Reference,
            provenance: Provenance::Measured,
            inputs: vec![],
        }])
        .unwrap();
        let receipt = evidence_receipt("reference.fixture", "fixture.phenomenon", "slot.one", 1);

        let floor = AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap();
        assert!(floor.receipt("reference.fixture").is_some());
        assert_eq!(floor.buckingham_pi_budget("fixture.phenomenon"), Some(1));
    }

    #[test]
    fn empty_law_evidence_is_a_refusal_not_a_boolean_pass() {
        let ledger = Ledger::build([Entry {
            id: "residue.fixture".into(),
            tier: Tier::Residue,
            provenance: Provenance::Estimator,
            inputs: vec![],
        }])
        .unwrap();
        let mut receipt = evidence_receipt("residue.fixture", "fixture.phenomenon", "slot.one", 1);
        receipt.residual_law.conservation.clear();

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap_err(),
            FloorAdmissionError::MissingReceiptEvidence {
                entry_id: "residue.fixture".into(),
                field: "residual.conservation",
            }
        );
    }

    #[test]
    fn empty_chaos_protocol_evidence_is_a_refusal() {
        let ledger = Ledger::build([Entry {
            id: "residue.fixture".into(),
            tier: Tier::Residue,
            provenance: Provenance::Estimator,
            inputs: vec![],
        }])
        .unwrap();
        let mut receipt = evidence_receipt("residue.fixture", "fixture.phenomenon", "slot.one", 1);
        receipt.gap_law.chaos_protocol = ChaosProtocolReceipt::Dynamical {
            classification: "the system carries a sub-resolution regime".into(),
            regime_partition: "one declared validity domain".into(),
            transition_law: "no transition exists in the one-regime partition".into(),
            regimes: vec![ChaosRegimeReceipt::SubresolutionMeasure {
                validity_domain: "the measure carries its domain".into(),
                stationary_measure: String::new(),
                conservation_projection: "conserved quantities are projected".into(),
                stability_postcondition: "the sampled state satisfies stability checks".into(),
                coordinate_discipline: "the coordinate is content-derived and versioned".into(),
                exact_replay: "equal coordinates replay bit-for-bit".into(),
            }],
        };

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap_err(),
            FloorAdmissionError::MissingReceiptEvidence {
                entry_id: "residue.fixture".into(),
                field: "chaos.regime.stationary_measure",
            }
        );
    }

    #[test]
    fn an_empty_dynamical_regime_partition_is_a_refusal() {
        let ledger = Ledger::build([Entry {
            id: "residue.fixture".into(),
            tier: Tier::Residue,
            provenance: Provenance::Estimator,
            inputs: vec![],
        }])
        .unwrap();
        let mut receipt = evidence_receipt("residue.fixture", "fixture.phenomenon", "slot.one", 1);
        receipt.gap_law.chaos_protocol = ChaosProtocolReceipt::Dynamical {
            classification: "dynamical".into(),
            regime_partition: "claimed partition".into(),
            transition_law: "claimed transition law".into(),
            regimes: Vec::new(),
        };

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, [receipt]).unwrap_err(),
            FloorAdmissionError::MissingReceiptEvidence {
                entry_id: "residue.fixture".into(),
                field: "chaos.regime",
            }
        );
    }

    #[test]
    fn chaos_protocol_branches_have_stable_typed_evidence_order() {
        let not_applicable = ChaosProtocolReceipt::NotApplicable {
            basis: "no evolving trajectory".into(),
        };
        assert_eq!(not_applicable.kind_id(), "not_applicable");
        assert_eq!(
            not_applicable.evidence(),
            vec![("chaos.basis", "no evolving trajectory")]
        );

        let dynamical = ChaosProtocolReceipt::Dynamical {
            classification: "two physical regimes".into(),
            regime_partition: "domains are disjoint and cover support".into(),
            transition_law: "boundary crossing is derived".into(),
            regimes: vec![
                ChaosRegimeReceipt::ResolvedTrajectory {
                    validity_domain: "resolved domain".into(),
                    resolution_bound: "input bands remain resolved".into(),
                    evolution_postcondition: "evolution stays in domain".into(),
                    exact_replay: "bit-for-bit replay".into(),
                },
                ChaosRegimeReceipt::SubresolutionMeasure {
                    validity_domain: "sensitive domain".into(),
                    stationary_measure: "derived measure".into(),
                    conservation_projection: "projected".into(),
                    stability_postcondition: "checked".into(),
                    coordinate_discipline: "content-derived".into(),
                    exact_replay: "bit-for-bit replay".into(),
                },
            ],
        };
        assert_eq!(dynamical.kind_id(), "dynamical");
        assert_eq!(
            dynamical
                .evidence()
                .into_iter()
                .map(|(field, _)| field)
                .collect::<Vec<_>>(),
            vec![
                "chaos.classification",
                "chaos.regime_partition",
                "chaos.transition_law",
                "chaos.regime",
                "chaos.regime.validity_domain",
                "chaos.regime.resolution_bound",
                "chaos.regime.evolution_postcondition",
                "chaos.regime.exact_replay",
                "chaos.regime",
                "chaos.regime.validity_domain",
                "chaos.regime.stationary_measure",
                "chaos.regime.conservation_projection",
                "chaos.regime.stability_postcondition",
                "chaos.regime.coordinate_discipline",
                "chaos.regime.exact_replay",
            ]
        );
    }

    #[test]
    fn buckingham_pi_budget_is_a_per_phenomenon_ceiling() {
        let ledger = Ledger::build([
            Entry {
                id: "reference.one".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
            Entry {
                id: "residue.two".into(),
                tier: Tier::Residue,
                provenance: Provenance::Estimator,
                inputs: vec![],
            },
        ])
        .unwrap();
        let receipts = [
            evidence_receipt("reference.one", "fixture.phenomenon", "slot.one", 1),
            evidence_receipt("residue.two", "fixture.phenomenon", "slot.two", 1),
        ];

        assert_eq!(
            AbsolutePhysicsFloor::admit(ledger, receipts).unwrap_err(),
            FloorAdmissionError::BuckinghamPiBudgetExceeded {
                phenomenon: "fixture.phenomenon".into(),
                admitted: 2,
                budget: 1,
            }
        );
    }

    #[test]
    fn residual_slots_are_unique_within_a_phenomenon() {
        let ledger = Ledger::build([
            Entry {
                id: "reference.one".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
            Entry {
                id: "reference.two".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
        ])
        .unwrap();
        let receipts = [
            evidence_receipt("reference.one", "fixture.phenomenon", "slot.one", 2),
            evidence_receipt("reference.two", "fixture.phenomenon", "slot.one", 2),
        ];

        assert!(matches!(
            AbsolutePhysicsFloor::admit(ledger, receipts),
            Err(FloorAdmissionError::DuplicateResidualSlot { .. })
        ));
    }

    #[test]
    fn a_phenomenon_cannot_change_its_declared_pi_budget() {
        let ledger = Ledger::build([
            Entry {
                id: "reference.one".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
            Entry {
                id: "reference.two".into(),
                tier: Tier::Reference,
                provenance: Provenance::Measured,
                inputs: vec![],
            },
        ])
        .unwrap();
        let receipts = [
            evidence_receipt("reference.one", "fixture.phenomenon", "slot.one", 1),
            evidence_receipt("reference.two", "fixture.phenomenon", "slot.two", 2),
        ];

        assert!(matches!(
            AbsolutePhysicsFloor::admit(ledger, receipts),
            Err(FloorAdmissionError::InconsistentBuckinghamPiBudget { .. })
        ));
    }
}
