//! Variable-cardinality component identity and topology contract.
//!
//! This is a schema for a future realized registry, not a registry instance.
//! It fixes how identities, ordering, topology, and engine-capacity failures
//! must behave without choosing a component count or constructing state.

pub(super) const COMPONENT_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-component-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentCardinalityRule {
    RealizationCoordinateDefinedFromJointMeasureSupport,
}

impl ComponentCardinalityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::RealizationCoordinateDefinedFromJointMeasureSupport => {
                "realization_coordinate_defined_from_joint_measure_support"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentIdentityRule {
    ContentDerivedFromJointMeasureAndCoordinate,
}

impl ComponentIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ContentDerivedFromJointMeasureAndCoordinate => {
                "content_derived_from_joint_measure_and_coordinate"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentOrderingRule {
    LexicographicContentIdentity,
}

impl ComponentOrderingRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::LexicographicContentIdentity => "lexicographic_content_identity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentTopologyRule {
    VariableCardinalityLabeledHypergraph,
}

impl ComponentTopologyRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::VariableCardinalityLabeledHypergraph => "variable_cardinality_labeled_hypergraph",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl ComponentCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentOrdinalRule {
    SerializationOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentEncodingRule {
    CanonicalPhysicalContentBytes,
}

impl ComponentEncodingRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalPhysicalContentBytes => "canonical_physical_content_bytes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentCollisionRule {
    SymmetryClassOrNamedHashCollisionRefusal,
}

impl ComponentCollisionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SymmetryClassOrNamedHashCollisionRefusal => {
                "symmetry_class_or_named_hash_collision_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ComponentSymmetryRule {
    PermutationEquivariantMultiset,
}

impl ComponentSymmetryRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::PermutationEquivariantMultiset => "permutation_equivariant_multiset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TopologyLabelAuthorityRule {
    DerivedPhysicalRelationOnly,
}

impl TopologyLabelAuthorityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivedPhysicalRelationOnly => "derived_physical_relation_only",
        }
    }
}

impl ComponentOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_coordinate",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ComponentRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) cardinality_rule: ComponentCardinalityRule,
    pub(crate) identity_rule: ComponentIdentityRule,
    pub(crate) ordering_rule: ComponentOrderingRule,
    pub(crate) topology_rule: ComponentTopologyRule,
    pub(crate) capacity_rule: ComponentCapacityRule,
    pub(crate) ordinal_rule: ComponentOrdinalRule,
    pub(crate) encoding_rule: ComponentEncodingRule,
    pub(crate) collision_rule: ComponentCollisionRule,
    pub(crate) symmetry_rule: ComponentSymmetryRule,
    pub(crate) topology_label_authority_rule: TopologyLabelAuthorityRule,
}

impl ComponentRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self.schema_id == COMPONENT_REGISTRY_SCHEMA_ID
            && matches!(
                self.cardinality_rule,
                ComponentCardinalityRule::RealizationCoordinateDefinedFromJointMeasureSupport
            )
            && matches!(
                self.identity_rule,
                ComponentIdentityRule::ContentDerivedFromJointMeasureAndCoordinate
            )
            && matches!(
                self.ordering_rule,
                ComponentOrderingRule::LexicographicContentIdentity
            )
            && matches!(
                self.topology_rule,
                ComponentTopologyRule::VariableCardinalityLabeledHypergraph
            )
            && matches!(
                self.capacity_rule,
                ComponentCapacityRule::EngineLimitIsNamedRefusal
            )
            && matches!(self.ordinal_rule, ComponentOrdinalRule::SerializationOnly)
            && matches!(
                self.encoding_rule,
                ComponentEncodingRule::CanonicalPhysicalContentBytes
            )
            && matches!(
                self.collision_rule,
                ComponentCollisionRule::SymmetryClassOrNamedHashCollisionRefusal
            )
            && matches!(
                self.symmetry_rule,
                ComponentSymmetryRule::PermutationEquivariantMultiset
            )
            && matches!(
                self.topology_label_authority_rule,
                TopologyLabelAuthorityRule::DerivedPhysicalRelationOnly
            )
    }
}

pub(super) const fn canonical_component_registry_schema() -> ComponentRegistrySchema {
    ComponentRegistrySchema {
        schema_id: COMPONENT_REGISTRY_SCHEMA_ID,
        cardinality_rule:
            ComponentCardinalityRule::RealizationCoordinateDefinedFromJointMeasureSupport,
        identity_rule: ComponentIdentityRule::ContentDerivedFromJointMeasureAndCoordinate,
        ordering_rule: ComponentOrderingRule::LexicographicContentIdentity,
        topology_rule: ComponentTopologyRule::VariableCardinalityLabeledHypergraph,
        capacity_rule: ComponentCapacityRule::EngineLimitIsNamedRefusal,
        ordinal_rule: ComponentOrdinalRule::SerializationOnly,
        encoding_rule: ComponentEncodingRule::CanonicalPhysicalContentBytes,
        collision_rule: ComponentCollisionRule::SymmetryClassOrNamedHashCollisionRefusal,
        symmetry_rule: ComponentSymmetryRule::PermutationEquivariantMultiset,
        topology_label_authority_rule: TopologyLabelAuthorityRule::DerivedPhysicalRelationOnly,
    }
}
