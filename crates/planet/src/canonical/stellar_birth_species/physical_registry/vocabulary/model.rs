use super::super::model::ArtifactIdentity;

pub(super) const VOCABULARY_SCHEMA_ID: &str = "civsim.planet.stellar-birth-physical-vocabulary.v1";
pub(super) const VOCABULARY_CLAIM_ID: &str =
    "civsim.planet.stellar-birth-physical-vocabulary.partition.v1";
pub(super) const VOCABULARY_RECEIPT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-physical-vocabulary-agreement.v1";
pub(super) const PRODUCER_ID: &str = "civsim.planet.stellar-birth-physical-vocabulary-producer.v1";
pub(super) const WATCHDOG_ID: &str = "civsim.planet.stellar-birth-physical-vocabulary-watchdog.v1";

pub(super) const MAX_ROOT_COUNT: u32 = 4_096;
pub(super) const MAX_PARTITION_IDENTITY_COUNT: u32 = 12_288;
pub(super) const MAX_WORK_UNITS: u64 = 20_480;
pub(super) const MAX_CANONICAL_BYTES: u32 = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VocabularyResourceContract {
    pub(super) max_root_count: u32,
    pub(super) max_partition_identity_count: u32,
    pub(super) max_work_units: u64,
    pub(super) max_canonical_bytes: u32,
}

impl VocabularyResourceContract {
    pub(super) const PRODUCTION: Self = Self {
        max_root_count: MAX_ROOT_COUNT,
        max_partition_identity_count: MAX_PARTITION_IDENTITY_COUNT,
        max_work_units: MAX_WORK_UNITS,
        max_canonical_bytes: MAX_CANONICAL_BYTES,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VocabularyRefusalCode {
    ResourceContractMismatch,
    RootCountExceeded,
    PartitionIdentityCountExceeded,
    WorkLimitExceeded,
    CanonicalBytesExceeded,
    DuplicateRootIdentity,
    AdmissionCapabilityMismatch,
    EvidenceCustodyOnly,
    NoncanonicalPartition,
    CheckerDisagreement,
    AgreementReceiptMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VocabularyPartitions {
    pub(super) descriptor_role_identities: Vec<ArtifactIdentity>,
    pub(super) relation_target_identities: Vec<ArtifactIdentity>,
    pub(super) constraint_law_identities: Vec<ArtifactIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VocabularyCheckerOutput {
    pub(super) partitions: VocabularyPartitions,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) resource_contract_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VocabularyAgreementReceipt {
    pub(super) schema_id: &'static str,
    pub(super) claim_id: &'static str,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) root_count: u32,
    pub(super) descriptor_role_count: u32,
    pub(super) relation_target_count: u32,
    pub(super) constraint_law_count: u32,
    pub(super) current_input_partition_complete: bool,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) producer_result_sha256: [u8; 32],
    pub(super) watchdog_result_sha256: [u8; 32],
    pub(super) producer_resource_contract_sha256: [u8; 32],
    pub(super) watchdog_resource_contract_sha256: [u8; 32],
    pub(super) receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalVocabularyAgreement {
    pub(super) partitions: VocabularyPartitions,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) receipt: VocabularyAgreementReceipt,
}
