//! Private model for physical species derivation and closure.
//!
//! Every admitted root carries one of the canonical two routes. Ledger tier
//! and provenance are accounting fields only. Evidence custody without the
//! complete admission route is represented so both validators can refuse it.

use super::super::SpeciesContentIdentity;
pub(super) use civsim_ledger::{Provenance as ProvenanceMark, Tier as LedgerTier};

pub(super) const REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-physical-species-registry.v1";
pub(super) const PROOF_GRAPH_SCHEMA_ID: &str = "civsim.planet.stellar-birth-species-proof-graph.v1";
pub(super) const PRODUCER_ID: &str = "civsim.planet.stellar-birth-physical-species-producer.v1";
pub(super) const WATCHDOG_ID: &str = "civsim.planet.stellar-birth-physical-species-watchdog.v1";

pub(super) const MAX_ARTIFACT_COUNT: u32 = 4_096;
pub(super) const MAX_REGISTRY_MEMBER_COUNT: u32 = 4_096;
pub(super) const MAX_REFERENCES_PER_ARTIFACT: u32 = 4_096;
pub(super) const MAX_TOTAL_REFERENCE_COUNT: u32 = 65_536;
pub(super) const MAX_EXPRESSION_NODE_COUNT: u32 = 65_536;
pub(super) const MAX_EXPRESSION_EDGE_COUNT: u32 = 131_072;
pub(super) const MAX_EXPRESSION_DEPTH: u32 = 1_024;
pub(super) const MAX_RATIONAL_COMPONENT_BITS: u32 = 4_096;
pub(super) const MAX_INTERMEDIATE_COMPONENT_BITS: u32 = 65_536;
pub(super) const MAX_DIMENSION_ABS_EXPONENT: i32 = 4_096;
pub(super) const MAX_EVALUATION_STEPS: u64 = 1_000_000;
pub(super) const MAX_CLOSURE_STEPS: u64 = 1_000_000;
pub(super) const MAX_CANONICAL_BYTES: u32 = 16_777_216;
pub(super) const MAX_CANONICAL_TOKEN_BYTES: u32 = 192;
pub(super) const MAX_CONTENT_BYTES: u32 = 1_048_576;

pub(super) const MASS_DIMENSION: DimensionVector = DimensionVector([0, 1, 0, 0, 0, 0, 0]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ArtifactIdentity(pub(super) [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReceiptBinding {
    pub(super) schema_id: String,
    pub(super) digest_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CanonicalArtifact {
    pub(super) schema_id: String,
    pub(super) canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DerivedAdmission {
    pub(super) ancestry_receipt: ReceiptBinding,
    pub(super) semantic_checker_receipt: ReceiptBinding,
    pub(super) independent_watchdog_receipt: ReceiptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct IrreducibleAdmission {
    pub(super) derivation_exhaustion_receipt: ReceiptBinding,
    pub(super) buckingham_pi_receipt: ReceiptBinding,
    pub(super) gap_law_receipt: ReceiptBinding,
    pub(super) chaos_protocol_receipt: ReceiptBinding,
    pub(super) residual_law_receipt: ReceiptBinding,
    pub(super) residual_slot_id: String,
    pub(super) residual_slot_receipt: ReceiptBinding,
    pub(super) owner_admission_receipt: ReceiptBinding,
    pub(super) independent_watchdog_receipt: ReceiptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AdmissionRoute {
    Derived(DerivedAdmission),
    Irreducible(Box<IrreducibleAdmission>),
    EvidenceCustodyOnly { source_receipt: ReceiptBinding },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RootAdmission {
    pub(super) tier: LedgerTier,
    pub(super) provenance: ProvenanceMark,
    pub(super) route: AdmissionRoute,
}

/// Canonical signed rational. Components are minimal big-endian byte strings,
/// the denominator is positive, and zero is nonnegative.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactRationalWire {
    pub(super) negative: bool,
    pub(super) numerator_be: Vec<u8>,
    pub(super) denominator_be: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionVector(pub(super) [i16; 7]);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ScalarCoordinateArtifact {
    pub(super) coordinate: CanonicalArtifact,
    pub(super) exact_value: ExactRationalWire,
    pub(super) dimension: DimensionVector,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExactExpressionNode {
    Coordinate(ArtifactIdentity),
    Add { left: u32, right: u32 },
    Subtract { left: u32, right: u32 },
    Multiply { left: u32, right: u32 },
    Divide { numerator: u32, denominator: u32 },
    IntegerPower { base: u32, exponent: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactExpression {
    pub(super) nodes: Vec<ExactExpressionNode>,
    pub(super) output_node: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RequirementSet {
    pub(super) state_coordinates: Vec<ArtifactIdentity>,
    pub(super) active_sectors: Vec<ArtifactIdentity>,
    pub(super) validity_regimes: Vec<ArtifactIdentity>,
    pub(super) species_dependencies: Vec<SpeciesContentIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MassProofReference {
    Projection(ArtifactIdentity),
    ExactMassless(ArtifactIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MemberBlueprint {
    pub(super) physical_content: CanonicalArtifact,
    pub(super) requirements: RequirementSet,
    pub(super) mass_proof: MassProofReference,
    pub(super) stability_law: ArtifactIdentity,
    pub(super) transition_law: ArtifactIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ConstraintLawArtifact {
    pub(super) requirements: RequirementSet,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MassProjectionArtifact {
    pub(super) expression: ExactExpression,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MasslessLawArtifact {
    pub(super) state_coordinates: Vec<ArtifactIdentity>,
    pub(super) active_sectors: Vec<ArtifactIdentity>,
    pub(super) validity_regimes: Vec<ArtifactIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DirectFloorSpeciesArtifact {
    pub(super) output: MemberBlueprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ElementaryExcitationArtifact {
    pub(super) fields: Vec<ArtifactIdentity>,
    pub(super) operators: Vec<ArtifactIdentity>,
    pub(super) output: MemberBlueprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CompositeBoundStateArtifact {
    pub(super) constituents: Vec<SpeciesContentIdentity>,
    pub(super) operators: Vec<ArtifactIdentity>,
    pub(super) output: MemberBlueprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ArtifactPayload {
    ScalarCoordinate(ScalarCoordinateArtifact),
    FieldContent(CanonicalArtifact),
    Operator(CanonicalArtifact),
    StateCoordinate(CanonicalArtifact),
    InteractionSector(CanonicalArtifact),
    ValidityRegime(CanonicalArtifact),
    StabilityLaw(ConstraintLawArtifact),
    TransitionLaw(ConstraintLawArtifact),
    MassProjection(MassProjectionArtifact),
    ExactMasslessLaw(MasslessLawArtifact),
    DirectFloorSpecies(DirectFloorSpeciesArtifact),
    ElementaryExcitation(ElementaryExcitationArtifact),
    CompositeBoundState(CompositeBoundStateArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct AdmittedArtifact {
    pub(super) claimed_identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructureAuthorityBinding {
    pub(super) structure_schema_id: String,
    pub(super) species_registry_schema_id: String,
    pub(super) stellar_state_schema_id: String,
    pub(super) state_coordinate_registry_schema_id: String,
    pub(super) interaction_sector_registry_schema_id: String,
    pub(super) physical_regime_registry_schema_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckerPairBinding {
    pub(super) producer_id: String,
    pub(super) watchdog_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhysicalRegistryResourceContract {
    pub(super) max_artifact_count: u32,
    pub(super) max_registry_member_count: u32,
    pub(super) max_references_per_artifact: u32,
    pub(super) max_total_reference_count: u32,
    pub(super) max_expression_node_count: u32,
    pub(super) max_expression_edge_count: u32,
    pub(super) max_expression_depth: u32,
    pub(super) max_rational_component_bits: u32,
    pub(super) max_intermediate_component_bits: u32,
    pub(super) max_dimension_abs_exponent: i32,
    pub(super) max_evaluation_steps: u64,
    pub(super) max_closure_steps: u64,
    pub(super) max_canonical_bytes: u32,
    pub(super) max_canonical_token_bytes: u32,
    pub(super) max_content_bytes: u32,
}

impl PhysicalRegistryResourceContract {
    pub(super) const PRODUCTION: Self = Self {
        max_artifact_count: MAX_ARTIFACT_COUNT,
        max_registry_member_count: MAX_REGISTRY_MEMBER_COUNT,
        max_references_per_artifact: MAX_REFERENCES_PER_ARTIFACT,
        max_total_reference_count: MAX_TOTAL_REFERENCE_COUNT,
        max_expression_node_count: MAX_EXPRESSION_NODE_COUNT,
        max_expression_edge_count: MAX_EXPRESSION_EDGE_COUNT,
        max_expression_depth: MAX_EXPRESSION_DEPTH,
        max_rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        max_intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        max_dimension_abs_exponent: MAX_DIMENSION_ABS_EXPONENT,
        max_evaluation_steps: MAX_EVALUATION_STEPS,
        max_closure_steps: MAX_CLOSURE_STEPS,
        max_canonical_bytes: MAX_CANONICAL_BYTES,
        max_canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
        max_content_bytes: MAX_CONTENT_BYTES,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalRegistryInput {
    pub(super) schema_id: String,
    pub(super) proof_graph_schema_id: String,
    pub(super) floor_binding: ReceiptBinding,
    pub(super) structure_binding: StructureAuthorityBinding,
    pub(super) checker_pair: CheckerPairBinding,
    pub(super) resources: PhysicalRegistryResourceContract,
    pub(super) admitted_artifacts: Vec<AdmittedArtifact>,
    pub(super) declared_members: Vec<SpeciesContentIdentity>,
}

/// Tests may tighten one cap to exercise a refusal without building a maximum
/// sized graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidationCaps {
    pub(super) artifact_count: u32,
    pub(super) registry_member_count: u32,
    pub(super) references_per_artifact: u32,
    pub(super) total_reference_count: u32,
    pub(super) expression_node_count: u32,
    pub(super) expression_edge_count: u32,
    pub(super) expression_depth: u32,
    pub(super) rational_component_bits: u32,
    pub(super) intermediate_component_bits: u32,
    pub(super) dimension_abs_exponent: i32,
    pub(super) evaluation_steps: u64,
    pub(super) closure_steps: u64,
    pub(super) canonical_bytes: u32,
    pub(super) canonical_token_bytes: u32,
    pub(super) content_bytes: u32,
}

impl ValidationCaps {
    pub(super) const PRODUCTION: Self = Self {
        artifact_count: MAX_ARTIFACT_COUNT,
        registry_member_count: MAX_REGISTRY_MEMBER_COUNT,
        references_per_artifact: MAX_REFERENCES_PER_ARTIFACT,
        total_reference_count: MAX_TOTAL_REFERENCE_COUNT,
        expression_node_count: MAX_EXPRESSION_NODE_COUNT,
        expression_edge_count: MAX_EXPRESSION_EDGE_COUNT,
        expression_depth: MAX_EXPRESSION_DEPTH,
        rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        dimension_abs_exponent: MAX_DIMENSION_ABS_EXPONENT,
        evaluation_steps: MAX_EVALUATION_STEPS,
        closure_steps: MAX_CLOSURE_STEPS,
        canonical_bytes: MAX_CANONICAL_BYTES,
        canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
        content_bytes: MAX_CONTENT_BYTES,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum DerivationRoute {
    DirectFloorProperty,
    ElementaryExcitation,
    CompositeBoundState,
}

impl DerivationRoute {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::DirectFloorProperty => "direct_floor_property",
            Self::ElementaryExcitation => "elementary_excitation",
            Self::CompositeBoundState => "composite_bound_state",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VerifiedPhysicalMember {
    pub(super) identity: SpeciesContentIdentity,
    pub(super) physical_content: CanonicalArtifact,
    pub(super) rest_mass_si: ExactRationalWire,
    pub(super) mass_dimension: DimensionVector,
    pub(super) route: DerivationRoute,
    pub(super) requirements: RequirementSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedRegistry {
    pub(super) members: Vec<VerifiedPhysicalMember>,
    pub(super) canonical_bytes: Vec<u8>,
}

/// Agreement is conditional on the admitted lower artifacts supplied to the
/// pair. It is not the dormant `SpeciesRegistryAuthority` and has no route to
/// the conditioned-support reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerifiedPhysicalSpeciesRegistry {
    pub(super) members: Vec<VerifiedPhysicalMember>,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthorityEffect {
    None,
}

impl AuthorityEffect {
    pub(super) const fn id(self) -> &'static str {
        "none"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PhysicalRegistryRefusalCode {
    SchemaMismatch,
    CheckerPairMismatch,
    ResourceContractMismatch,
    FloorBindingMismatch,
    StructureBindingMismatch,
    ArtifactCapacityExceeded,
    RegistryCapacityExceeded,
    ReferenceCapacityExceeded,
    ExpressionNodeCapacityExceeded,
    ExpressionEdgeCapacityExceeded,
    ExpressionDepthExceeded,
    RationalComponentLimitExceeded,
    IntermediateComponentLimitExceeded,
    DimensionExponentLimitExceeded,
    EvaluationStepLimitExceeded,
    ClosureStepLimitExceeded,
    CanonicalByteLimitExceeded,
    ContentByteLimitExceeded,
    CanonicalTextInvalid,
    MissingBindingDigest,
    EvidenceCustodyIsNotAdmission,
    NoncanonicalProvenance,
    DerivedAdmissionProvenanceMismatch,
    IrreducibleAdmissionProvenanceInvalid,
    GeneratedProvenanceCannotBeRoot,
    DuplicateAdmissionReceipt,
    DuplicateResidualSlot,
    RationalEncodingInvalid,
    RationalNotReduced,
    ArtifactIdentityMismatch,
    DuplicateArtifactIdentity,
    ArtifactIdentityCollision,
    DuplicateRegistryMember,
    UnknownArtifactReference,
    ArtifactKindMismatch,
    RequirementSetEmpty,
    DuplicateRequirement,
    DependencyMismatch,
    UnknownSpeciesDependency,
    ExpressionEmpty,
    ExpressionOutputInvalid,
    ExpressionCycle,
    ExpressionContainsUnusedNode,
    DivisionByZero,
    DimensionMismatch,
    MassDimensionMismatch,
    NonPositiveMass,
    UnprovedExactZero,
    NoAdmittedSpeciesDerivationRoots,
    EmptyRegistryIsNotClosure,
    DerivationCycle,
    DuplicateMemberDerivation,
    MissingClosureMember,
    ExtraClosureMember,
    CheckerDisagreement,
}

impl PhysicalRegistryRefusalCode {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SchemaMismatch => "schema_mismatch",
            Self::CheckerPairMismatch => "checker_pair_mismatch",
            Self::ResourceContractMismatch => "resource_contract_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::StructureBindingMismatch => "structure_binding_mismatch",
            Self::ArtifactCapacityExceeded => "artifact_capacity_exceeded",
            Self::RegistryCapacityExceeded => "registry_capacity_exceeded",
            Self::ReferenceCapacityExceeded => "reference_capacity_exceeded",
            Self::ExpressionNodeCapacityExceeded => "expression_node_capacity_exceeded",
            Self::ExpressionEdgeCapacityExceeded => "expression_edge_capacity_exceeded",
            Self::ExpressionDepthExceeded => "expression_depth_exceeded",
            Self::RationalComponentLimitExceeded => "rational_component_limit_exceeded",
            Self::IntermediateComponentLimitExceeded => "intermediate_component_limit_exceeded",
            Self::DimensionExponentLimitExceeded => "dimension_exponent_limit_exceeded",
            Self::EvaluationStepLimitExceeded => "evaluation_step_limit_exceeded",
            Self::ClosureStepLimitExceeded => "closure_step_limit_exceeded",
            Self::CanonicalByteLimitExceeded => "canonical_byte_limit_exceeded",
            Self::ContentByteLimitExceeded => "content_byte_limit_exceeded",
            Self::CanonicalTextInvalid => "canonical_text_invalid",
            Self::MissingBindingDigest => "missing_binding_digest",
            Self::EvidenceCustodyIsNotAdmission => "evidence_custody_is_not_admission",
            Self::NoncanonicalProvenance => "noncanonical_provenance",
            Self::DerivedAdmissionProvenanceMismatch => "derived_admission_provenance_mismatch",
            Self::IrreducibleAdmissionProvenanceInvalid => {
                "irreducible_admission_provenance_invalid"
            }
            Self::GeneratedProvenanceCannotBeRoot => "generated_provenance_cannot_be_root",
            Self::DuplicateAdmissionReceipt => "duplicate_admission_receipt",
            Self::DuplicateResidualSlot => "duplicate_residual_slot",
            Self::RationalEncodingInvalid => "rational_encoding_invalid",
            Self::RationalNotReduced => "rational_not_reduced",
            Self::ArtifactIdentityMismatch => "artifact_identity_mismatch",
            Self::DuplicateArtifactIdentity => "duplicate_artifact_identity",
            Self::ArtifactIdentityCollision => "artifact_identity_collision",
            Self::DuplicateRegistryMember => "duplicate_registry_member",
            Self::UnknownArtifactReference => "unknown_artifact_reference",
            Self::ArtifactKindMismatch => "artifact_kind_mismatch",
            Self::RequirementSetEmpty => "requirement_set_empty",
            Self::DuplicateRequirement => "duplicate_requirement",
            Self::DependencyMismatch => "dependency_mismatch",
            Self::UnknownSpeciesDependency => "unknown_species_dependency",
            Self::ExpressionEmpty => "expression_empty",
            Self::ExpressionOutputInvalid => "expression_output_invalid",
            Self::ExpressionCycle => "expression_cycle",
            Self::ExpressionContainsUnusedNode => "expression_contains_unused_node",
            Self::DivisionByZero => "division_by_zero",
            Self::DimensionMismatch => "dimension_mismatch",
            Self::MassDimensionMismatch => "mass_dimension_mismatch",
            Self::NonPositiveMass => "non_positive_mass",
            Self::UnprovedExactZero => "unproved_exact_zero",
            Self::NoAdmittedSpeciesDerivationRoots => "no_admitted_species_derivation_roots",
            Self::EmptyRegistryIsNotClosure => "empty_registry_is_not_closure",
            Self::DerivationCycle => "derivation_cycle",
            Self::DuplicateMemberDerivation => "duplicate_member_derivation",
            Self::MissingClosureMember => "missing_closure_member",
            Self::ExtraClosureMember => "extra_closure_member",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalRegistryRefusal {
    pub(super) code: PhysicalRegistryRefusalCode,
    pub(super) member_count: u32,
    pub(super) coverage_claim: bool,
    pub(super) authority_effect: AuthorityEffect,
    pub(super) open_obligations: Vec<&'static str>,
}

impl PhysicalRegistryRefusal {
    pub(super) fn from_code(code: PhysicalRegistryRefusalCode) -> Self {
        let open_obligations =
            if code == PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots {
                vec![
                    "floor_species_property_attribution",
                    "admitted_interaction_sector_membership",
                    "admitted_state_coordinate_membership",
                    "stable_excitation_or_bound_state_derivation",
                    "complete_registry_closure_domain",
                    "certified_mass_projection",
                ]
            } else {
                Vec::new()
            };
        Self {
            code,
            member_count: 0,
            coverage_claim: false,
            authority_effect: AuthorityEffect::None,
            open_obligations,
        }
    }
}
