//! Open, value-free coordinate membership contract for stellar state.
//!
//! This schema describes how future coordinates earn membership. It contains
//! no coordinate entries, dimensions, tensor ranks, or physical magnitudes.

pub(super) const STATE_COORDINATE_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-state-coordinate-registry.v1";
pub(super) const DIMENSION_BASIS_REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-dimension-basis-registry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateMembershipRule {
    PhysicalClosureDerivedOrNamedRefusal,
}

impl CoordinateMembershipRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::PhysicalClosureDerivedOrNamedRefusal => {
                "physical_closure_derived_or_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateIdentityRule {
    CanonicalCompleteDescriptorDigest,
}

impl CoordinateIdentityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalCompleteDescriptorDigest => {
                "canonical_semantics_basis_dimension_domain_tensor_normalization_reference_sector_dependency_digest"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateDimensionRule {
    ExactExponentVectorBoundToActiveFloorBasis,
}

impl CoordinateDimensionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ExactExponentVectorBoundToActiveFloorBasis => {
                "exact_exponent_vector_bound_to_active_floor_basis"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateIndexRule {
    CanonicalRegisteredDomainProduct,
}

impl CoordinateIndexRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CanonicalRegisteredDomainProduct => "canonical_registered_domain_product",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateTensorRule {
    ExplicitRankSymmetryVarianceAndPolarAxialCharacter,
}

impl CoordinateTensorRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ExplicitRankSymmetryVarianceAndPolarAxialCharacter => {
                "explicit_rank_symmetry_variance_and_polar_axial_character"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateNormalizationRule {
    ExplicitPhysicalNormalizationOrNotApplicable,
}

impl CoordinateNormalizationRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ExplicitPhysicalNormalizationOrNotApplicable => {
                "explicit_physical_normalization_or_not_applicable"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateReferenceRule {
    ExplicitGaugeOrPhysicalReferenceSemantics,
}

impl CoordinateReferenceRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::ExplicitGaugeOrPhysicalReferenceSemantics => {
                "explicit_gauge_or_physical_reference_semantics"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateCompletenessRule {
    EveryActiveRegimeDependencyPresentOrNamedRefusal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateCollisionRule {
    CompleteDescriptorCollisionIsNamedRefusal,
}

impl CoordinateCollisionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::CompleteDescriptorCollisionIsNamedRefusal => {
                "complete_descriptor_collision_is_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateExtensionRule {
    VersionedPresealRegistryExtensionOnly,
}

impl CoordinateExtensionRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::VersionedPresealRegistryExtensionOnly => {
                "versioned_preseal_registry_extension_only"
            }
        }
    }
}

impl CoordinateCompletenessRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EveryActiveRegimeDependencyPresentOrNamedRefusal => {
                "every_active_regime_dependency_present_or_named_refusal"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateCapacityRule {
    EngineLimitIsNamedRefusal,
}

impl CoordinateCapacityRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CoordinateOrdinalRule {
    SerializationOnly,
}

impl CoordinateOrdinalRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::SerializationOnly => "serialization_only_never_identity_or_coordinate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DimensionBasisRule {
    DerivedOrImmutablePresealAdmittedMembership,
    CanonicalSemanticBasisDigestIdentity,
    VariableCardinality,
    ExactSignedIntegerVectorMatchingBasisCardinality,
    NewSealedVersionAndRunBeforeExtension,
    EngineLimitIsNamedRefusal,
    SerializationOnly,
}

impl DimensionBasisRule {
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::DerivedOrImmutablePresealAdmittedMembership => {
                "derived_or_immutable_preseal_admitted_membership"
            }
            Self::CanonicalSemanticBasisDigestIdentity => {
                "canonical_semantic_basis_digest_identity"
            }
            Self::VariableCardinality => "variable_cardinality",
            Self::ExactSignedIntegerVectorMatchingBasisCardinality => {
                "exact_signed_integer_vector_matching_basis_cardinality"
            }
            Self::NewSealedVersionAndRunBeforeExtension => {
                "new_sealed_version_and_run_before_extension"
            }
            Self::EngineLimitIsNamedRefusal => "engine_limit_is_named_refusal",
            Self::SerializationOnly => "serialization_only_never_identity_or_dimension",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct DimensionBasisRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: DimensionBasisRule,
    pub(crate) identity_rule: DimensionBasisRule,
    pub(crate) cardinality_rule: DimensionBasisRule,
    pub(crate) exponent_encoding_rule: DimensionBasisRule,
    pub(crate) extension_rule: DimensionBasisRule,
    pub(crate) capacity_rule: DimensionBasisRule,
    pub(crate) ordinal_rule: DimensionBasisRule,
}

impl DimensionBasisRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_dimension_basis_registry_schema()
    }
}

pub(super) const fn canonical_dimension_basis_registry_schema() -> DimensionBasisRegistrySchema {
    DimensionBasisRegistrySchema {
        schema_id: DIMENSION_BASIS_REGISTRY_SCHEMA_ID,
        membership_rule: DimensionBasisRule::DerivedOrImmutablePresealAdmittedMembership,
        identity_rule: DimensionBasisRule::CanonicalSemanticBasisDigestIdentity,
        cardinality_rule: DimensionBasisRule::VariableCardinality,
        exponent_encoding_rule:
            DimensionBasisRule::ExactSignedIntegerVectorMatchingBasisCardinality,
        extension_rule: DimensionBasisRule::NewSealedVersionAndRunBeforeExtension,
        capacity_rule: DimensionBasisRule::EngineLimitIsNamedRefusal,
        ordinal_rule: DimensionBasisRule::SerializationOnly,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StateCoordinateRegistrySchema {
    pub(crate) schema_id: &'static str,
    pub(crate) membership_rule: CoordinateMembershipRule,
    pub(crate) identity_rule: CoordinateIdentityRule,
    pub(crate) dimension_rule: CoordinateDimensionRule,
    pub(crate) index_rule: CoordinateIndexRule,
    pub(crate) tensor_rule: CoordinateTensorRule,
    pub(crate) normalization_rule: CoordinateNormalizationRule,
    pub(crate) reference_rule: CoordinateReferenceRule,
    pub(crate) completeness_rule: CoordinateCompletenessRule,
    pub(crate) collision_rule: CoordinateCollisionRule,
    pub(crate) extension_rule: CoordinateExtensionRule,
    pub(crate) capacity_rule: CoordinateCapacityRule,
    pub(crate) ordinal_rule: CoordinateOrdinalRule,
    pub(crate) dimension_basis_registry: DimensionBasisRegistrySchema,
}

impl StateCoordinateRegistrySchema {
    pub(super) fn is_canonical(&self) -> bool {
        self == &canonical_state_coordinate_registry_schema()
            && self.dimension_basis_registry.is_canonical()
    }
}

pub(super) const fn canonical_state_coordinate_registry_schema() -> StateCoordinateRegistrySchema {
    StateCoordinateRegistrySchema {
        schema_id: STATE_COORDINATE_REGISTRY_SCHEMA_ID,
        membership_rule: CoordinateMembershipRule::PhysicalClosureDerivedOrNamedRefusal,
        identity_rule: CoordinateIdentityRule::CanonicalCompleteDescriptorDigest,
        dimension_rule: CoordinateDimensionRule::ExactExponentVectorBoundToActiveFloorBasis,
        index_rule: CoordinateIndexRule::CanonicalRegisteredDomainProduct,
        tensor_rule: CoordinateTensorRule::ExplicitRankSymmetryVarianceAndPolarAxialCharacter,
        normalization_rule:
            CoordinateNormalizationRule::ExplicitPhysicalNormalizationOrNotApplicable,
        reference_rule: CoordinateReferenceRule::ExplicitGaugeOrPhysicalReferenceSemantics,
        completeness_rule:
            CoordinateCompletenessRule::EveryActiveRegimeDependencyPresentOrNamedRefusal,
        collision_rule: CoordinateCollisionRule::CompleteDescriptorCollisionIsNamedRefusal,
        extension_rule: CoordinateExtensionRule::VersionedPresealRegistryExtensionOnly,
        capacity_rule: CoordinateCapacityRule::EngineLimitIsNamedRefusal,
        ordinal_rule: CoordinateOrdinalRule::SerializationOnly,
        dimension_basis_registry: canonical_dimension_basis_registry_schema(),
    }
}

/// Read-only view of the open stellar-state coordinate registry contract.
#[derive(Debug, Clone, Copy)]
pub struct StateCoordinateRegistrySchemaView<'a> {
    schema: &'a StateCoordinateRegistrySchema,
}

/// Read-only view of the open physical dimension-basis contract.
#[derive(Debug, Clone, Copy)]
pub struct DimensionBasisRegistrySchemaView<'a> {
    schema: &'a DimensionBasisRegistrySchema,
}

impl<'a> StateCoordinateRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a StateCoordinateRegistrySchema) -> Self {
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

    pub fn dimension_rule_id(self) -> &'static str {
        self.schema.dimension_rule.id()
    }

    pub fn index_rule_id(self) -> &'static str {
        self.schema.index_rule.id()
    }

    pub fn tensor_rule_id(self) -> &'static str {
        self.schema.tensor_rule.id()
    }

    pub fn normalization_rule_id(self) -> &'static str {
        self.schema.normalization_rule.id()
    }

    pub fn reference_rule_id(self) -> &'static str {
        self.schema.reference_rule.id()
    }

    pub fn completeness_rule_id(self) -> &'static str {
        self.schema.completeness_rule.id()
    }

    pub fn collision_rule_id(self) -> &'static str {
        self.schema.collision_rule.id()
    }

    pub fn extension_rule_id(self) -> &'static str {
        self.schema.extension_rule.id()
    }

    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }

    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }

    pub fn dimension_basis_registry(self) -> DimensionBasisRegistrySchemaView<'a> {
        DimensionBasisRegistrySchemaView::new(&self.schema.dimension_basis_registry)
    }
}

impl<'a> DimensionBasisRegistrySchemaView<'a> {
    pub(in crate::canonical) const fn new(schema: &'a DimensionBasisRegistrySchema) -> Self {
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

    pub fn cardinality_rule_id(self) -> &'static str {
        self.schema.cardinality_rule.id()
    }

    pub fn exponent_encoding_rule_id(self) -> &'static str {
        self.schema.exponent_encoding_rule.id()
    }

    pub fn extension_rule_id(self) -> &'static str {
        self.schema.extension_rule.id()
    }

    pub fn capacity_rule_id(self) -> &'static str {
        self.schema.capacity_rule.id()
    }

    pub fn ordinal_rule_id(self) -> &'static str {
        self.schema.ordinal_rule.id()
    }
}
