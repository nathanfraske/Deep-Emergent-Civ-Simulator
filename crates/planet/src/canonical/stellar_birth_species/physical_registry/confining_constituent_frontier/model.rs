use super::super::model::ArtifactIdentity;

pub(super) const INPUT_SCHEMA_ID: &str = "civsim.planet.confining-constituent-frontier-input.v1";
pub(super) const DIAGNOSTIC_SCHEMA_ID: &str =
    "civsim.planet.confining-constituent-frontier-agreement.v1";
pub(super) const CLAIM_ID: &str = "planet.confining-sector.constituent-input-frontier";
pub(super) const PRODUCER_ID: &str =
    "civsim.planet.confining-constituent-frontier.forward-inventory-producer.v1";
pub(super) const WATCHDOG_ID: &str =
    "civsim.planet.confining-constituent-frontier.reverse-obligation-watchdog.v1";

pub(super) const MAX_INTERNAL_SEED_COUNT: u32 = 4_096;
pub(super) const MAX_WORK_UNITS: u64 = 65_536;
pub(super) const MAX_CANONICAL_BYTES: u32 = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ConfiningConstituentObligation {
    RepresentationFamilyMembershipClosure,
    ConfiningDynamicsScaleOrEquivalent,
    ExcitationProfileCoverage,
    ExactRestMassOrMasslessProof,
    ChargeStateStatisticsDispositionCoverage,
    ConstituentApplicabilityValidity,
    ConstituentConservationBinding,
    TransitionSeparationChannelCoverage,
}

impl ConfiningConstituentObligation {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::RepresentationFamilyMembershipClosure => {
                "confining.representation_family_membership_closure"
            }
            Self::ConfiningDynamicsScaleOrEquivalent => "confining.dynamics_scale_or_equivalent",
            Self::ExcitationProfileCoverage => "confining.excitation_profile_coverage",
            Self::ExactRestMassOrMasslessProof => "confining.exact_rest_mass_or_massless_proof",
            Self::ChargeStateStatisticsDispositionCoverage => {
                "confining.charge_state_statistics_disposition_coverage"
            }
            Self::ConstituentApplicabilityValidity => {
                "confining.constituent_applicability_validity"
            }
            Self::ConstituentConservationBinding => "confining.constituent_conservation_binding",
            Self::TransitionSeparationChannelCoverage => {
                "confining.transition_separation_channel_coverage"
            }
        }
    }

    pub(super) const fn ordinal(self) -> u8 {
        match self {
            Self::RepresentationFamilyMembershipClosure => 0,
            Self::ConfiningDynamicsScaleOrEquivalent => 1,
            Self::ExcitationProfileCoverage => 2,
            Self::ExactRestMassOrMasslessProof => 3,
            Self::ChargeStateStatisticsDispositionCoverage => 4,
            Self::ConstituentApplicabilityValidity => 5,
            Self::ConstituentConservationBinding => 6,
            Self::TransitionSeparationChannelCoverage => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ConfiningConstituentResourceContract {
    pub(super) max_internal_seed_count: u32,
    pub(super) max_work_units: u64,
    pub(super) max_canonical_bytes: u32,
}

impl ConfiningConstituentResourceContract {
    pub(super) const PRODUCTION: Self = Self {
        max_internal_seed_count: MAX_INTERNAL_SEED_COUNT,
        max_work_units: MAX_WORK_UNITS,
        max_canonical_bytes: MAX_CANONICAL_BYTES,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct InternalConstituentSeed {
    pub(super) identity: ArtifactIdentity,
    pub(super) source_profile_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfiningConstituentInput {
    pub(super) schema_id: &'static str,
    pub(super) source_profile_claim_id: &'static str,
    pub(super) source_profile_receipt_sha256: [u8; 32],
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) sector_identity: ArtifactIdentity,
    pub(super) carrier_identity: ArtifactIdentity,
    pub(super) constraint_law_identity: ArtifactIdentity,
    pub(super) internal_seeds: Vec<InternalConstituentSeed>,
    pub(super) resources: ConfiningConstituentResourceContract,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ConfiningConstituentDecision {
    BlockedOpenProofs {
        missing: Vec<ConfiningConstituentObligation>,
    },
}

impl ConfiningConstituentDecision {
    pub(super) const fn id(&self) -> &'static str {
        match self {
            Self::BlockedOpenProofs { .. } => "blocked_open_proofs",
        }
    }

    pub(super) fn missing(&self) -> &[ConfiningConstituentObligation] {
        match self {
            Self::BlockedOpenProofs { missing } => missing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfiningConstituentEvaluation {
    pub(super) input_sha256: [u8; 32],
    pub(super) source_profile_receipt_sha256: [u8; 32],
    pub(super) profile_root_identity: ArtifactIdentity,
    pub(super) sector_identity: ArtifactIdentity,
    pub(super) carrier_identity: ArtifactIdentity,
    pub(super) constraint_law_identity: ArtifactIdentity,
    pub(super) internal_seed_identities: Vec<ArtifactIdentity>,
    pub(super) decision: ConfiningConstituentDecision,
    pub(super) constituent_candidate_count: u32,
    pub(super) membership_authority: bool,
    pub(super) spectrum_authority: bool,
    pub(super) authority_effect: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfiningConstituentCheckerOutput {
    pub(super) evaluation: ConfiningConstituentEvaluation,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) resource_contract_sha256: [u8; 32],
    pub(super) trace_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species::physical_registry) enum ConfiningConstituentRefusalCode
{
    ResourceContractMismatch,
    InputSchemaMismatch,
    SourceProfileMismatch,
    InvalidSourceBinding,
    IdentityAlias,
    InternalSeedCapacityExceeded,
    EmptyInternalSeedSet,
    DuplicateInternalSeed,
    CarrierSeedMissing,
    InternalSeedBindingMismatch,
    WorkLimitExceeded,
    CanonicalBytesExceeded,
    CheckerDisagreement,
    AgreementReceiptMismatch,
}

impl ConfiningConstituentRefusalCode {
    pub(in crate::canonical::stellar_birth_species::physical_registry) const fn id(
        self,
    ) -> &'static str {
        match self {
            Self::ResourceContractMismatch => "resource_contract_mismatch",
            Self::InputSchemaMismatch => "input_schema_mismatch",
            Self::SourceProfileMismatch => "source_profile_mismatch",
            Self::InvalidSourceBinding => "invalid_source_binding",
            Self::IdentityAlias => "identity_alias",
            Self::InternalSeedCapacityExceeded => "internal_seed_capacity_exceeded",
            Self::EmptyInternalSeedSet => "empty_internal_seed_set",
            Self::DuplicateInternalSeed => "duplicate_internal_seed",
            Self::CarrierSeedMissing => "carrier_seed_missing",
            Self::InternalSeedBindingMismatch => "internal_seed_binding_mismatch",
            Self::WorkLimitExceeded => "work_limit_exceeded",
            Self::CanonicalBytesExceeded => "canonical_bytes_exceeded",
            Self::CheckerDisagreement => "checker_disagreement",
            Self::AgreementReceiptMismatch => "agreement_receipt_mismatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::canonical::stellar_birth_species::physical_registry) struct ConfiningConstituentFrontierReceipt
{
    pub(in crate::canonical::stellar_birth_species::physical_registry) schema_id: &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) claim_id: &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) producer_id: &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) watchdog_id: &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) decision_id: &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) source_profile_receipt_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) internal_seed_identities:
        Vec<ArtifactIdentity>,
    pub(in crate::canonical::stellar_birth_species::physical_registry) missing_authority_ids:
        Vec<&'static str>,
    pub(in crate::canonical::stellar_birth_species::physical_registry) constituent_candidate_count:
        u32,
    pub(in crate::canonical::stellar_birth_species::physical_registry) membership_authority: bool,
    pub(in crate::canonical::stellar_birth_species::physical_registry) spectrum_authority: bool,
    pub(in crate::canonical::stellar_birth_species::physical_registry) authority_effect:
        &'static str,
    pub(in crate::canonical::stellar_birth_species::physical_registry) producer_result_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) watchdog_result_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) producer_trace_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) watchdog_trace_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) producer_resource_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) watchdog_resource_sha256:
        [u8; 32],
    pub(in crate::canonical::stellar_birth_species::physical_registry) receipt_sha256: [u8; 32],
}
