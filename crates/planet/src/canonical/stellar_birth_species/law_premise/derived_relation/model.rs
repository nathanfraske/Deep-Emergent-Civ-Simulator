use super::super::{LawClaimIdentity, PhysicalContentIdentity, SemanticRoleIdentity};

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.derived-law-premise-eps0-packet.v1";
pub(super) const ARTIFACT_SCHEMA_ID: &str =
    "civsim.planet.derived-law-premise-execution-relation.v2";
pub(super) const RECEIPT_SCHEMA_ID: &str = "civsim.planet.derived-law-premise-eps0-pair-receipt.v2";
pub(super) const CANARY_SUITE_ID: &str = "civsim.planet.derived-law-premise-eps0-canaries.v2";
pub(super) const PRODUCER_CANARY_TRANSCRIPT_ID: &str =
    "civsim.planet.derived-law-premise-eps0.producer-canary-transcript.v1";
pub(super) const WATCHDOG_CANARY_TRANSCRIPT_ID: &str =
    "civsim.planet.derived-law-premise-eps0.watchdog-canary-transcript.v1";
pub(super) const PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.planet.derived-law-premise-eps0.sorted-vector-producer.v2";
pub(super) const WATCHDOG_IMPLEMENTATION_ID: &str =
    "civsim.planet.derived-law-premise-eps0.map-reconstruction-watchdog.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputRole {
    PhysicalInvariant,
    RepresentationDefinition,
}

impl InputRole {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::PhysicalInvariant => "physical_invariant",
            Self::RepresentationDefinition => "representation_definition",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConstantBinding {
    pub(super) symbol: String,
    pub(super) role: InputRole,
    pub(super) bits: i128,
    pub(super) scale_bits: u32,
    pub(super) projection_receipt_sha256: [u8; 32],
    pub(super) unit: String,
    pub(super) dimension: [i8; 7],
    pub(super) source_id: String,
    pub(super) source_sha256: String,
    pub(super) source_anchor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OutputBinding {
    pub(super) symbol: String,
    pub(super) bits: i128,
    pub(super) scale_bits: u32,
    pub(super) projection_receipt_sha256: [u8; 32],
    pub(super) unit: String,
    pub(super) dimension: [i8; 7],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AlphaAdmissionBinding {
    pub(super) schema_id: String,
    pub(super) entry_id: String,
    pub(super) derivation_exhaustion_receipt: [u8; 32],
    pub(super) buckingham_pi_receipt: [u8; 32],
    pub(super) gap_law_receipt: [u8; 32],
    pub(super) chaos_protocol_receipt: [u8; 32],
    pub(super) residual_law_receipt: [u8; 32],
    pub(super) residual_slot_receipt: [u8; 32],
    pub(super) owner_admission_receipt: [u8; 32],
    pub(super) independent_watchdog_receipt: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RelationBinding {
    pub(super) symbol: String,
    pub(super) name: String,
    pub(super) formula: String,
    pub(super) inputs: Vec<String>,
    pub(super) unit: String,
    pub(super) dimension: [i8; 7],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DerivedRelationPacket {
    pub(super) schema_id: String,
    pub(super) floor_authority_schema_id: String,
    pub(super) floor_authority_sha256: [u8; 32],
    pub(super) alpha_admission: AlphaAdmissionBinding,
    pub(super) relation: RelationBinding,
    pub(super) inputs: Vec<ConstantBinding>,
    pub(super) output: OutputBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DerivedRelationCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) claim_identity: LawClaimIdentity,
    pub(super) role_identity: SemanticRoleIdentity,
    pub(super) content_identity: PhysicalContentIdentity,
    pub(super) relation_bytes: Vec<u8>,
    pub(super) upstream_capability_sha256: [u8; 32],
    pub(super) applicability_receipt_sha256: [u8; 32],
    pub(super) validity_receipt_sha256: [u8; 32],
    pub(super) ancestry_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CanaryEvidence {
    pub(super) transcript_id: &'static str,
    pub(super) case_count: u32,
    pub(super) transcript_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DerivedRelationRefusal {
    SealedSourceUnavailable,
    PacketSchemaMismatch,
    FloorBindingInvalid,
    AlphaAdmissionInvalid,
    RelationIdentityMismatch,
    RelationFormulaMismatch,
    RelationInputMismatch,
    RelationDimensionMismatch,
    InputBindingInvalid,
    OutputBindingInvalid,
    RepresentationScaleMismatch,
    ArithmeticMismatch,
    ResourceLimitExceeded,
    CanaryFailure,
    CheckerDisagreement,
    CapabilityDigestMismatch,
    SelectionFailure,
}

impl DerivedRelationRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SealedSourceUnavailable => "sealed_source_unavailable",
            Self::PacketSchemaMismatch => "packet_schema_mismatch",
            Self::FloorBindingInvalid => "floor_binding_invalid",
            Self::AlphaAdmissionInvalid => "alpha_admission_invalid",
            Self::RelationIdentityMismatch => "relation_identity_mismatch",
            Self::RelationFormulaMismatch => "relation_formula_mismatch",
            Self::RelationInputMismatch => "relation_input_mismatch",
            Self::RelationDimensionMismatch => "relation_dimension_mismatch",
            Self::InputBindingInvalid => "input_binding_invalid",
            Self::OutputBindingInvalid => "output_binding_invalid",
            Self::RepresentationScaleMismatch => "representation_scale_mismatch",
            Self::ArithmeticMismatch => "arithmetic_mismatch",
            Self::ResourceLimitExceeded => "resource_limit_exceeded",
            Self::CanaryFailure => "canary_failure",
            Self::CheckerDisagreement => "checker_disagreement",
            Self::CapabilityDigestMismatch => "capability_digest_mismatch",
            Self::SelectionFailure => "selection_failure",
        }
    }
}
