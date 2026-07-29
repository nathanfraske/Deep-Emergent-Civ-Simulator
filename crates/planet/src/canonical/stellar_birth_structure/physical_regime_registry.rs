//! Open physical-regime predicate and dispatch contract.
//!
//! Regime names and ordinals have no causal authority. Dispatch is possible
//! only through independently admitted, acyclic physical predicate proofs.

pub(super) const PHYSICAL_REGIME_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-physical-regime-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeMembershipRule {
    StateHistoryAndAdmittedLawPredicateOnly,
}

impl RegimeMembershipRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::StateHistoryAndAdmittedLawPredicateOnly => {
                "state_history_and_admitted_law_predicate_only"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimePredicateAdmissionRule {
    EveryEntryFormulaThresholdAndUseDeriveFirstOrRefusal,
}

impl RegimePredicateAdmissionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EveryEntryFormulaThresholdAndUseDeriveFirstOrRefusal => {
                "every_entry_formula_threshold_and_use_derive_first_or_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeIdentityRule {
    CanonicalPredicateAndDependencyDigest,
}

impl RegimeIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalPredicateAndDependencyDigest => {
                "canonical_predicate_and_dependency_digest"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeDependencyRule {
    OneSealedAcyclicPredispatchPhysicalDag,
}

impl RegimeDependencyRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::OneSealedAcyclicPredispatchPhysicalDag => {
                "one_sealed_acyclic_predispatch_physical_dag"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeForbiddenInputRule {
    NoTaxonomyCandidateExecutionResultOrProofBackflow,
}

impl RegimeForbiddenInputRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NoTaxonomyCandidateExecutionResultOrProofBackflow => {
                "no_taxonomy_candidate_execution_result_or_proof_backflow"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeAliasRule {
    RenameInvariantAliasMayReferenceProofNeverReplaceIt,
}

impl RegimeAliasRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::RenameInvariantAliasMayReferenceProofNeverReplaceIt => {
                "rename_invariant_alias_may_reference_proof_never_replace_it"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeApplicabilityRule {
    EachMechanismProvedIndependently,
}

impl RegimeApplicabilityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EachMechanismProvedIndependently => "each_mechanism_proved_independently",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeCompatibilityRule {
    UniqueJointlySatisfiableNonDoubleCountingTransition,
}

impl RegimeCompatibilityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::UniqueJointlySatisfiableNonDoubleCountingTransition => {
                "unique_jointly_satisfiable_non_double_counting_transition"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeSymmetryRule {
    InvariantUnderPhysicallyEquivalentMechanismPermutation,
}

impl RegimeSymmetryRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::InvariantUnderPhysicallyEquivalentMechanismPermutation => {
                "invariant_under_physically_equivalent_mechanism_permutation"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeCompositionRule {
    AdmittedLawAndGlobalConservationEntailedOnly,
}

impl RegimeCompositionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AdmittedLawAndGlobalConservationEntailedOnly => {
                "admitted_law_and_global_conservation_entailed_only"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeAmbiguityRule {
    NonuniqueIncompatibleOrUnderdeterminedIsNamedRefusal,
}

impl RegimeAmbiguityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NonuniqueIncompatibleOrUnderdeterminedIsNamedRefusal => {
                "nonunique_incompatible_or_underdetermined_is_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeDispatchRule {
    ProofIdentityNeverNameLabelOrdinalOrPriority,
}

impl RegimeDispatchRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ProofIdentityNeverNameLabelOrdinalOrPriority => {
                "proof_identity_never_name_label_ordinal_or_priority"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl RegimeCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RegimeOrdinalRule {
    SerializationOnly,
}

impl RegimeOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_dispatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PhysicalRegimeRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: RegimeMembershipRule,
    pub(crate) predicate_admission_rule: RegimePredicateAdmissionRule,
    pub(crate) identity_rule: RegimeIdentityRule,
    pub(crate) dependency_rule: RegimeDependencyRule,
    pub(crate) forbidden_input_rule: RegimeForbiddenInputRule,
    pub(crate) alias_rule: RegimeAliasRule,
    pub(crate) applicability_rule: RegimeApplicabilityRule,
    pub(crate) compatibility_rule: RegimeCompatibilityRule,
    pub(crate) symmetry_rule: RegimeSymmetryRule,
    pub(crate) composition_rule: RegimeCompositionRule,
    pub(crate) ambiguity_rule: RegimeAmbiguityRule,
    pub(crate) dispatch_rule: RegimeDispatchRule,
    pub(crate) capacity_rule: RegimeCapacityRule,
    pub(crate) ordinal_rule: RegimeOrdinalRule,
}

impl PhysicalRegimeRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_physical_regime_registry_schema()
    }
}

pub(super) const fn canonical_physical_regime_registry_schema() -> PhysicalRegimeRegistrySchema {
    PhysicalRegimeRegistrySchema {
        schema_id: PHYSICAL_REGIME_REGISTRY_SCHEMA_ID,
        membership_rule: RegimeMembershipRule::StateHistoryAndAdmittedLawPredicateOnly,
        predicate_admission_rule:
            RegimePredicateAdmissionRule::EveryEntryFormulaThresholdAndUseDeriveFirstOrRefusal,
        identity_rule: RegimeIdentityRule::CanonicalPredicateAndDependencyDigest,
        dependency_rule: RegimeDependencyRule::OneSealedAcyclicPredispatchPhysicalDag,
        forbidden_input_rule:
            RegimeForbiddenInputRule::NoTaxonomyCandidateExecutionResultOrProofBackflow,
        alias_rule: RegimeAliasRule::RenameInvariantAliasMayReferenceProofNeverReplaceIt,
        applicability_rule: RegimeApplicabilityRule::EachMechanismProvedIndependently,
        compatibility_rule:
            RegimeCompatibilityRule::UniqueJointlySatisfiableNonDoubleCountingTransition,
        symmetry_rule: RegimeSymmetryRule::InvariantUnderPhysicallyEquivalentMechanismPermutation,
        composition_rule: RegimeCompositionRule::AdmittedLawAndGlobalConservationEntailedOnly,
        ambiguity_rule: RegimeAmbiguityRule::NonuniqueIncompatibleOrUnderdeterminedIsNamedRefusal,
        dispatch_rule: RegimeDispatchRule::ProofIdentityNeverNameLabelOrdinalOrPriority,
        capacity_rule: RegimeCapacityRule::EngineLimitIsNamedRefusal,
        ordinal_rule: RegimeOrdinalRule::SerializationOnly,
    }
}

/// Read-only view of the physical-regime proof and dispatch contract.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalRegimeRegistrySchemaView<'a> {
    schema: &'a PhysicalRegimeRegistrySchema,
}

impl<'a> PhysicalRegimeRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a PhysicalRegimeRegistrySchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }
    pub fn membership_rule_id(self) -> &'static str {
        self.schema.membership_rule.id()
    }
    pub fn predicate_admission_rule_id(self) -> &'static str {
        self.schema.predicate_admission_rule.id()
    }
    pub fn identity_rule_id(self) -> &'static str {
        self.schema.identity_rule.id()
    }
    pub fn dependency_rule_id(self) -> &'static str {
        self.schema.dependency_rule.id()
    }
    pub fn forbidden_input_rule_id(self) -> &'static str {
        self.schema.forbidden_input_rule.id()
    }
    pub fn alias_rule_id(self) -> &'static str {
        self.schema.alias_rule.id()
    }
    pub fn applicability_rule_id(self) -> &'static str {
        self.schema.applicability_rule.id()
    }
    pub fn compatibility_rule_id(self) -> &'static str {
        self.schema.compatibility_rule.id()
    }
    pub fn symmetry_rule_id(self) -> &'static str {
        self.schema.symmetry_rule.id()
    }
    pub fn composition_rule_id(self) -> &'static str {
        self.schema.composition_rule.id()
    }
    pub fn ambiguity_rule_id(self) -> &'static str {
        self.schema.ambiguity_rule.id()
    }
    pub fn dispatch_rule_id(self) -> &'static str {
        self.schema.dispatch_rule.id()
    }
    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }
    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }
}
