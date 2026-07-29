//! Noncausal stellar-classification projection contract.
//!
//! Taxonomies are versioned presentation views over a sealed physical history.
//! They do not own physical predicates, state, or mechanism dispatch.

pub(super) const CLASSIFICATION_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-classification-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationMembershipRule {
    ProjectionFromSealedPhysicalHistoryOnly,
}

impl ClassificationMembershipRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ProjectionFromSealedPhysicalHistoryOnly => {
                "projection_from_sealed_physical_history_only"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationIdentityRule {
    TaxonomyEntryAndVersionTuple,
}

impl ClassificationIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::TaxonomyEntryAndVersionTuple => "taxonomy_entry_and_version_tuple",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationDisplayNameRule {
    NonidentifyingMetadataWithNoCrossRegistryEquivalence,
}

impl ClassificationDisplayNameRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::NonidentifyingMetadataWithNoCrossRegistryEquivalence => {
                "nonidentifying_metadata_with_no_cross_registry_equivalence"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationCardinalityRule {
    ZeroOrMoreOverlappingEntries,
}

impl ClassificationCardinalityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ZeroOrMoreOverlappingEntries => "zero_or_more_overlapping_entries",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationUnclassifiedRule {
    AbsencePreservesEveryPhysicalRecord,
}

impl ClassificationUnclassifiedRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AbsencePreservesEveryPhysicalRecord => "absence_preserves_every_physical_record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationBoundaryRule {
    ReadOnlyTotalProjectionFromOneImmutableCausalTranscript,
}

impl ClassificationBoundaryRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ReadOnlyTotalProjectionFromOneImmutableCausalTranscript => {
                "read_only_total_projection_from_one_immutable_causal_transcript"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationCausalAuthorityRule {
    None,
}

impl ClassificationCausalAuthorityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationSelectorRule {
    CannotSelectMeasureCoordinateInitialBoundaryMechanismOrTransition,
}

impl ClassificationSelectorRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CannotSelectMeasureCoordinateInitialBoundaryMechanismOrTransition => {
                "cannot_select_measure_coordinate_initial_boundary_mechanism_or_transition"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationMutationRule {
    AddRemoveRenameReorderPreservesPhysicalTranscript,
}

impl ClassificationMutationRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::AddRemoveRenameReorderPreservesPhysicalTranscript => {
                "add_remove_rename_reorder_preserves_physical_transcript"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationViewerRule {
    PreferencesQueriesLabelsAndOrderFilterOrRenderOnly,
}

impl ClassificationViewerRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::PreferencesQueriesLabelsAndOrderFilterOrRenderOnly => {
                "preferences_queries_labels_and_order_filter_or_render_only"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationVersionRule {
    CausalRuleChangeRequiresDistinctSealedCausalVersion,
}

impl ClassificationVersionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CausalRuleChangeRequiresDistinctSealedCausalVersion => {
                "causal_rule_change_requires_distinct_sealed_causal_version"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationCapacityRule {
    EngineLimitIsNamedPresentationRefusal,
}

impl ClassificationCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedPresentationRefusal => {
                "engine_limit_is_named_presentation_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ClassificationOrdinalRule {
    SerializationOnly,
}

impl ClassificationOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_dispatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ClassificationRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: ClassificationMembershipRule,
    pub(crate) identity_rule: ClassificationIdentityRule,
    pub(crate) display_name_rule: ClassificationDisplayNameRule,
    pub(crate) cardinality_rule: ClassificationCardinalityRule,
    pub(crate) unclassified_rule: ClassificationUnclassifiedRule,
    pub(crate) boundary_rule: ClassificationBoundaryRule,
    pub(crate) causal_authority_rule: ClassificationCausalAuthorityRule,
    pub(crate) selector_rule: ClassificationSelectorRule,
    pub(crate) mutation_rule: ClassificationMutationRule,
    pub(crate) viewer_rule: ClassificationViewerRule,
    pub(crate) version_rule: ClassificationVersionRule,
    pub(crate) capacity_rule: ClassificationCapacityRule,
    pub(crate) ordinal_rule: ClassificationOrdinalRule,
}

impl ClassificationRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_classification_registry_schema()
    }
}

pub(super) const fn canonical_classification_registry_schema() -> ClassificationRegistrySchema {
    ClassificationRegistrySchema {
        schema_id: CLASSIFICATION_REGISTRY_SCHEMA_ID,
        membership_rule: ClassificationMembershipRule::ProjectionFromSealedPhysicalHistoryOnly,
        identity_rule: ClassificationIdentityRule::TaxonomyEntryAndVersionTuple,
        display_name_rule:
            ClassificationDisplayNameRule::NonidentifyingMetadataWithNoCrossRegistryEquivalence,
        cardinality_rule: ClassificationCardinalityRule::ZeroOrMoreOverlappingEntries,
        unclassified_rule: ClassificationUnclassifiedRule::AbsencePreservesEveryPhysicalRecord,
        boundary_rule:
            ClassificationBoundaryRule::ReadOnlyTotalProjectionFromOneImmutableCausalTranscript,
        causal_authority_rule: ClassificationCausalAuthorityRule::None,
        selector_rule:
            ClassificationSelectorRule::CannotSelectMeasureCoordinateInitialBoundaryMechanismOrTransition,
        mutation_rule:
            ClassificationMutationRule::AddRemoveRenameReorderPreservesPhysicalTranscript,
        viewer_rule: ClassificationViewerRule::PreferencesQueriesLabelsAndOrderFilterOrRenderOnly,
        version_rule:
            ClassificationVersionRule::CausalRuleChangeRequiresDistinctSealedCausalVersion,
        capacity_rule: ClassificationCapacityRule::EngineLimitIsNamedPresentationRefusal,
        ordinal_rule: ClassificationOrdinalRule::SerializationOnly,
    }
}

/// Read-only view of the noncausal classification projection contract.
#[derive(Debug, Clone, Copy)]
pub struct ClassificationRegistrySchemaView<'a> {
    schema: &'a ClassificationRegistrySchema,
}

impl<'a> ClassificationRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a ClassificationRegistrySchema) -> Self {
        Self { schema }
    }

    pub fn schema_id(self) -> &'static str {
        self.schema.schema_id
    }
    pub fn membership_rule_id(self) -> &'static str {
        self.schema.membership_rule.id()
    }
    pub fn identity_rule_id(self) -> &'static str {
        self.schema.identity_rule.id()
    }
    pub fn display_name_rule_id(self) -> &'static str {
        self.schema.display_name_rule.id()
    }
    pub fn cardinality_rule_id(self) -> &'static str {
        self.schema.cardinality_rule.id()
    }
    pub fn unclassified_rule_id(self) -> &'static str {
        self.schema.unclassified_rule.id()
    }
    pub fn boundary_rule_id(self) -> &'static str {
        self.schema.boundary_rule.id()
    }
    pub fn causal_authority_rule_id(self) -> &'static str {
        self.schema.causal_authority_rule.id()
    }
    pub fn selector_rule_id(self) -> &'static str {
        self.schema.selector_rule.id()
    }
    pub fn mutation_rule_id(self) -> &'static str {
        self.schema.mutation_rule.id()
    }
    pub fn viewer_rule_id(self) -> &'static str {
        self.schema.viewer_rule.id()
    }
    pub fn version_rule_id(self) -> &'static str {
        self.schema.version_rule.id()
    }
    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }
    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }
}
