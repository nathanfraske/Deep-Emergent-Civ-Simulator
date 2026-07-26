//! Typed input and output for repository-owned physical coordinate roots.

use super::super::model::{
    AdmittedArtifact, ArtifactIdentity, ArtifactPayload, DimensionVector, LedgerTier,
    ProvenanceMark, ReceiptBinding,
};

pub(in super::super) const REPOSITORY_ROOT_PACKET_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-packet.v5";
pub(super) const REPOSITORY_ROOT_PROJECTION_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-projection.v5";
pub(super) const REPOSITORY_ROOT_RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-receipt.v5";
pub(super) const REPOSITORY_ROOT_REFUSAL_RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-refusal-receipt.v5";
pub(super) const REPOSITORY_ROOT_ANCESTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-ancestry.v2";
pub(super) const REPOSITORY_ROOT_PRODUCER_RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-producer-receipt.v5";
pub(super) const REPOSITORY_ROOT_WATCHDOG_RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-watchdog-receipt.v5";
pub(super) const REPOSITORY_ROOT_PRODUCER_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-producer.v6";
pub(super) const REPOSITORY_ROOT_WATCHDOG_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-watchdog.v6";
pub(super) const REPOSITORY_ROOT_CLAIM_ID: &str =
    "planet.stellar-species-floor-coordinate-projection";
pub(super) const REPOSITORY_ROOT_CANARY_SUITE_ID: &str =
    "civsim.planet.stellar-birth-repository-physical-root-canaries.v6";
pub(super) const COORDINATE_CONTENT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-physical-invariant-coordinate.v3";
pub(super) const COORDINATE_ROLE: &str = "physical-invariant-coordinate-only";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum RepositoryRootSourceRole {
    PhysicalInvariant,
}

impl RepositoryRootSourceRole {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::PhysicalInvariant => "physical_invariant",
        }
    }
}

/// One sealed physical-invariant record supplied by the repository adapter.
///
/// The packet constructor is private to the physical registry. Callers outside
/// that module cannot supply values or relabel representation definitions as
/// physical invariant coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RepositoryRootSource {
    pub(in super::super) role: RepositoryRootSourceRole,
    pub(in super::super) entry_id: String,
    pub(in super::super) symbol: String,
    pub(in super::super) central_decimal: String,
    pub(in super::super) uncertainty_kind: String,
    pub(in super::super) uncertainty_decimal: String,
    pub(in super::super) dimension: DimensionVector,
    pub(in super::super) source_tier: LedgerTier,
    pub(in super::super) source_provenance: ProvenanceMark,
    pub(in super::super) exhaustion_binding: ReceiptBinding,
}

impl RepositoryRootSource {
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn new(
        entry_id: impl Into<String>,
        symbol: impl Into<String>,
        central_decimal: impl Into<String>,
        uncertainty_kind: impl Into<String>,
        uncertainty_decimal: impl Into<String>,
        dimension: DimensionVector,
        source_tier: LedgerTier,
        source_provenance: ProvenanceMark,
        exhaustion_binding: ReceiptBinding,
    ) -> Self {
        Self {
            role: RepositoryRootSourceRole::PhysicalInvariant,
            entry_id: entry_id.into(),
            symbol: symbol.into(),
            central_decimal: central_decimal.into(),
            uncertainty_kind: uncertainty_kind.into(),
            uncertainty_decimal: uncertainty_decimal.into(),
            dimension,
            source_tier,
            source_provenance,
            exhaustion_binding,
        }
    }
}

/// Complete, typed input to both independent root projectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RepositoryRootPacket {
    pub(in super::super) schema_id: String,
    pub(in super::super) floor_authority: ReceiptBinding,
    pub(in super::super) sources: Vec<RepositoryRootSource>,
    pub(in super::super) membership_authority: bool,
}

impl RepositoryRootPacket {
    pub(in super::super) fn new(
        floor_authority: ReceiptBinding,
        sources: Vec<RepositoryRootSource>,
    ) -> Self {
        Self {
            schema_id: REPOSITORY_ROOT_PACKET_SCHEMA_ID.to_owned(),
            floor_authority,
            sources,
            membership_authority: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ProjectedRootKind {
    ScalarCoordinate,
    MembershipNeutralMassProjection,
}

impl ProjectedRootKind {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::ScalarCoordinate => "scalar-coordinate",
            Self::MembershipNeutralMassProjection => "membership-neutral-mass-projection",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RootCandidate {
    pub(super) source_entry_id: String,
    pub(super) source_tier: LedgerTier,
    pub(super) kind: ProjectedRootKind,
    pub(super) identity: ArtifactIdentity,
    pub(super) payload: ArtifactPayload,
    pub(super) ancestry_digest_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RootCheckerOutput {
    pub(super) input_sha256: [u8; 32],
    pub(super) candidates: Vec<RootCandidate>,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) scalar_coordinate_count: u32,
    pub(super) mass_projection_count: u32,
}

/// Receipt for agreement over membership-neutral coordinate roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RepositoryRootProjectionReceipt {
    pub(in super::super) schema_id: &'static str,
    pub(in super::super) claim_id: &'static str,
    pub(in super::super) input_sha256: [u8; 32],
    pub(in super::super) result_sha256: [u8; 32],
    pub(in super::super) producer_result_sha256: [u8; 32],
    pub(in super::super) watchdog_result_sha256: [u8; 32],
    pub(in super::super) producer_ancestry_manifest_sha256: [u8; 32],
    pub(in super::super) watchdog_ancestry_manifest_sha256: [u8; 32],
    pub(in super::super) producer_resource_contract_sha256: [u8; 32],
    pub(in super::super) watchdog_resource_contract_sha256: [u8; 32],
    pub(in super::super) canary_suite_id: &'static str,
    pub(in super::super) canary_sha256: [u8; 32],
    pub(in super::super) producer_id: &'static str,
    pub(in super::super) watchdog_id: &'static str,
    pub(in super::super) scalar_coordinate_count: u32,
    pub(in super::super) mass_projection_count: u32,
    pub(in super::super) membership_authority: bool,
    pub(in super::super) decision_id: &'static str,
    pub(in super::super) receipt_sha256: [u8; 32],
}

/// Paired output ready to become the physical registry's admitted root list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RepositoryRootProjection {
    pub(in super::super) admitted_artifacts: Vec<AdmittedArtifact>,
    pub(in super::super) canonical_bytes: Vec<u8>,
    pub(in super::super) receipt: RepositoryRootProjectionReceipt,
    pub(in super::super) membership_authority: bool,
    pub(super) checker_candidates: Vec<RootCandidate>,
    /// Paired exact-mutation transcript over the complete projection. The
    /// transcript normalizes only this self-referential digest field.
    pub(super) post_projection_canary_sha256: [u8; 32],
}

/// One checker's exact outcome for a refused pair decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RepositoryRootCheckerOutcome {
    Projected {
        input_sha256: [u8; 32],
        result_sha256: [u8; 32],
    },
    Refused {
        code: RepositoryRootRefusalCode,
    },
    NotRun {
        reason: RepositoryRootRefusalCode,
    },
}

/// One checker's independently encoded resource contract, or its typed failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RepositoryRootResourceContractOutcome {
    Bound { digest_sha256: [u8; 32] },
    Refused { code: RepositoryRootRefusalCode },
}

/// Receipt-side binding of the pre-receipt packet mutation suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepositoryRootCanaryBinding {
    pub(super) suite_id: &'static str,
    pub(super) digest_sha256: Option<[u8; 32]>,
}

/// Exact pipeline stage at which a refusal became authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepositoryRootRefusalStage {
    ResourceContractPreflight,
    PacketExtraction,
    PacketAgreement,
    CheckerAgreement,
    Canary,
    ProjectionReceipt,
}

impl RepositoryRootRefusalStage {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::ResourceContractPreflight => "resource_contract_preflight",
            Self::PacketExtraction => "packet_extraction",
            Self::PacketAgreement => "packet_agreement",
            Self::CheckerAgreement => "checker_agreement",
            Self::Canary => "canary",
            Self::ProjectionReceipt => "projection_receipt",
        }
    }
}

/// Claim-scoped evidence that both root paths refused without minting roots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepositoryRootRefusalReceipt {
    pub(super) schema_id: &'static str,
    pub(super) claim_id: &'static str,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) producer_packet_sha256: [u8; 32],
    pub(super) watchdog_packet_sha256: [u8; 32],
    pub(super) producer_outcome: RepositoryRootCheckerOutcome,
    pub(super) watchdog_outcome: RepositoryRootCheckerOutcome,
    pub(super) producer_resource_contract: RepositoryRootResourceContractOutcome,
    pub(super) watchdog_resource_contract: RepositoryRootResourceContractOutcome,
    pub(super) canary: RepositoryRootCanaryBinding,
    pub(super) refusal_stage: RepositoryRootRefusalStage,
    pub(super) refusal_code: RepositoryRootRefusalCode,
    pub(super) decision_id: &'static str,
    pub(super) membership_authority: bool,
    pub(super) receipt_sha256: [u8; 32],
}

/// Refusal payload. It deliberately contains no admitted artifact or projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct RepositoryRootRefusal {
    pub(in super::super) code: RepositoryRootRefusalCode,
    pub(super) receipt: RepositoryRootRefusalReceipt,
}

/// Complete pair decision with capability-bearing output confined to success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) enum RepositoryRootDecision {
    Projected(RepositoryRootProjection),
    Refused(RepositoryRootRefusal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum RepositoryRootRefusalCode {
    SchemaMismatch,
    MembershipAuthorityPresent,
    EmptySourceSet,
    SourceCapacityExceeded,
    CanonicalByteLimitExceeded,
    CanonicalTextInvalid,
    MissingBindingDigest,
    DuplicateEntryId,
    DuplicateSymbol,
    DuplicateLeafFingerprint,
    SourceTierNotUniversal,
    SourceProvenanceNotMeasured,
    DecimalInvalid,
    DecimalMagnitudeExceeded,
    NegativeUncertainty,
    DimensionExponentLimitExceeded,
    ArtifactIdentityCollision,
    UnexpectedArtifactKind,
    SealedFloorPacketMismatch,
    SealedSourceBindingMismatch,
    ResourceContractDisagreement,
    CanaryFailure,
    ReceiptInvalid,
    CheckerDisagreement,
}

impl RepositoryRootRefusalCode {
    pub(in super::super) const fn id(self) -> &'static str {
        match self {
            Self::SchemaMismatch => "schema_mismatch",
            Self::MembershipAuthorityPresent => "membership_authority_present",
            Self::EmptySourceSet => "empty_source_set",
            Self::SourceCapacityExceeded => "source_capacity_exceeded",
            Self::CanonicalByteLimitExceeded => "canonical_byte_limit_exceeded",
            Self::CanonicalTextInvalid => "canonical_text_invalid",
            Self::MissingBindingDigest => "missing_binding_digest",
            Self::DuplicateEntryId => "duplicate_entry_id",
            Self::DuplicateSymbol => "duplicate_symbol",
            Self::DuplicateLeafFingerprint => "duplicate_leaf_fingerprint",
            Self::SourceTierNotUniversal => "source_tier_not_universal",
            Self::SourceProvenanceNotMeasured => "source_provenance_not_measured",
            Self::DecimalInvalid => "decimal_invalid",
            Self::DecimalMagnitudeExceeded => "decimal_magnitude_exceeded",
            Self::NegativeUncertainty => "negative_uncertainty",
            Self::DimensionExponentLimitExceeded => "dimension_exponent_limit_exceeded",
            Self::ArtifactIdentityCollision => "artifact_identity_collision",
            Self::UnexpectedArtifactKind => "unexpected_artifact_kind",
            Self::SealedFloorPacketMismatch => "sealed_floor_packet_mismatch",
            Self::SealedSourceBindingMismatch => "sealed_source_binding_mismatch",
            Self::ResourceContractDisagreement => "resource_contract_disagreement",
            Self::CanaryFailure => "canary_failure",
            Self::ReceiptInvalid => "receipt_invalid",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}
