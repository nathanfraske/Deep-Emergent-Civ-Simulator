//! Non-admitting physical vocabulary derived from admitted root artifacts.
//!
//! The pair classifies only the roots it receives. Agreement may prove exact
//! coverage of that bounded input while keeping global vocabulary coverage and
//! physical membership authority false.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use civsim_units::digest::sha256;
use model::{
    PhysicalVocabularyAgreement, VocabularyAgreementReceipt, VocabularyRefusalCode,
    VocabularyResourceContract, PRODUCER_ID, VOCABULARY_CLAIM_ID, VOCABULARY_RECEIPT_SCHEMA_ID,
    WATCHDOG_ID,
};

use super::model::{AdmittedArtifact, PhysicalVocabularyBinding};

pub(super) fn derive_binding(roots: &[AdmittedArtifact]) -> Result<PhysicalVocabularyBinding, ()> {
    let agreement = derive_and_agree(roots).map_err(|_| ())?;
    Ok(PhysicalVocabularyBinding {
        schema_id: agreement.receipt.schema_id.to_owned(),
        claim_id: agreement.receipt.claim_id.to_owned(),
        producer_id: agreement.receipt.producer_id.to_owned(),
        watchdog_id: agreement.receipt.watchdog_id.to_owned(),
        descriptor_role_identities: agreement.partitions.descriptor_role_identities,
        relation_target_identities: agreement.partitions.relation_target_identities,
        constraint_law_identities: agreement.partitions.constraint_law_identities,
        root_count: agreement.receipt.root_count,
        current_input_partition_complete: agreement.receipt.current_input_partition_complete,
        global_physical_vocabulary_coverage: agreement.receipt.global_physical_vocabulary_coverage,
        membership_authority: agreement.receipt.membership_authority,
        producer_result_sha256: agreement.receipt.producer_result_sha256,
        watchdog_result_sha256: agreement.receipt.watchdog_result_sha256,
        producer_resource_contract_sha256: agreement.receipt.producer_resource_contract_sha256,
        watchdog_resource_contract_sha256: agreement.receipt.watchdog_resource_contract_sha256,
        receipt_sha256: agreement.receipt.receipt_sha256,
    })
}

pub(super) fn producer_accepts_binding(
    roots: &[AdmittedArtifact],
    binding: &PhysicalVocabularyBinding,
) -> bool {
    producer::matches_binding(roots, binding, VocabularyResourceContract::PRODUCTION)
}

pub(super) fn watchdog_accepts_binding(
    roots: &[AdmittedArtifact],
    binding: &PhysicalVocabularyBinding,
) -> bool {
    watchdog::matches_binding(roots, binding, VocabularyResourceContract::PRODUCTION)
}

fn derive_and_agree(
    roots: &[AdmittedArtifact],
) -> Result<PhysicalVocabularyAgreement, VocabularyRefusalCode> {
    derive_and_agree_with_resources(roots, VocabularyResourceContract::PRODUCTION)
}

fn derive_and_agree_with_resources(
    roots: &[AdmittedArtifact],
    resources: VocabularyResourceContract,
) -> Result<PhysicalVocabularyAgreement, VocabularyRefusalCode> {
    if resources != VocabularyResourceContract::PRODUCTION {
        return Err(VocabularyRefusalCode::ResourceContractMismatch);
    }
    let producer_resource = producer::resource_contract_sha256();
    let watchdog_resource = watchdog::resource_contract_sha256();
    if producer_resource != watchdog_resource {
        return Err(VocabularyRefusalCode::CheckerDisagreement);
    }

    let produced = producer::classify(roots, resources)?;
    let watched = watchdog::classify(roots, resources)?;
    if produced.partitions != watched.partitions
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.resource_contract_sha256 != producer_resource
        || watched.resource_contract_sha256 != watchdog_resource
    {
        return Err(VocabularyRefusalCode::CheckerDisagreement);
    }

    let root_count =
        u32::try_from(roots.len()).map_err(|_| VocabularyRefusalCode::RootCountExceeded)?;
    let descriptor_role_count = u32::try_from(produced.partitions.descriptor_role_identities.len())
        .map_err(|_| VocabularyRefusalCode::PartitionIdentityCountExceeded)?;
    let relation_target_count = u32::try_from(produced.partitions.relation_target_identities.len())
        .map_err(|_| VocabularyRefusalCode::PartitionIdentityCountExceeded)?;
    let constraint_law_count =
        u32::try_from(produced.partitions.constraint_law_identities.len())
            .map_err(|_| VocabularyRefusalCode::PartitionIdentityCountExceeded)?;
    let result_sha256 = sha256(&produced.canonical_bytes);
    let mut receipt = VocabularyAgreementReceipt {
        schema_id: VOCABULARY_RECEIPT_SCHEMA_ID,
        claim_id: VOCABULARY_CLAIM_ID,
        producer_id: PRODUCER_ID,
        watchdog_id: WATCHDOG_ID,
        root_count,
        descriptor_role_count,
        relation_target_count,
        constraint_law_count,
        current_input_partition_complete: relation_target_count == root_count,
        global_physical_vocabulary_coverage: false,
        membership_authority: false,
        producer_result_sha256: result_sha256,
        watchdog_result_sha256: result_sha256,
        producer_resource_contract_sha256: producer_resource,
        watchdog_resource_contract_sha256: watchdog_resource,
        receipt_sha256: [0; 32],
    };
    receipt.receipt_sha256 = receipt_digest(&receipt);
    let agreement = PhysicalVocabularyAgreement {
        partitions: produced.partitions,
        canonical_bytes: produced.canonical_bytes,
        receipt,
    };
    if !verify_agreement(&agreement) {
        return Err(VocabularyRefusalCode::AgreementReceiptMismatch);
    }
    Ok(agreement)
}

fn verify_agreement(agreement: &PhysicalVocabularyAgreement) -> bool {
    let receipt = &agreement.receipt;
    if receipt.schema_id != VOCABULARY_RECEIPT_SCHEMA_ID
        || receipt.claim_id != VOCABULARY_CLAIM_ID
        || receipt.producer_id != PRODUCER_ID
        || receipt.watchdog_id != WATCHDOG_ID
        || !receipt.current_input_partition_complete
        || receipt.global_physical_vocabulary_coverage
        || receipt.membership_authority
        || receipt.root_count != receipt.relation_target_count
        || usize::try_from(receipt.descriptor_role_count).ok()
            != Some(agreement.partitions.descriptor_role_identities.len())
        || usize::try_from(receipt.relation_target_count).ok()
            != Some(agreement.partitions.relation_target_identities.len())
        || usize::try_from(receipt.constraint_law_count).ok()
            != Some(agreement.partitions.constraint_law_identities.len())
        || !is_subset(
            &agreement.partitions.descriptor_role_identities,
            &agreement.partitions.relation_target_identities,
        )
        || !is_subset(
            &agreement.partitions.constraint_law_identities,
            &agreement.partitions.relation_target_identities,
        )
    {
        return false;
    }

    let resources = VocabularyResourceContract::PRODUCTION;
    let Ok(producer_bytes) = producer::encode_partitions(&agreement.partitions, resources) else {
        return false;
    };
    let Ok(watchdog_bytes) = watchdog::encode_partitions(&agreement.partitions, resources) else {
        return false;
    };
    let result_sha256 = sha256(&agreement.canonical_bytes);
    producer_bytes == agreement.canonical_bytes
        && watchdog_bytes == agreement.canonical_bytes
        && receipt.producer_result_sha256 == result_sha256
        && receipt.watchdog_result_sha256 == result_sha256
        && receipt.producer_resource_contract_sha256 == producer::resource_contract_sha256()
        && receipt.watchdog_resource_contract_sha256 == watchdog::resource_contract_sha256()
        && receipt.receipt_sha256 != [0; 32]
        && receipt.receipt_sha256 == receipt_digest(receipt)
}

fn is_subset(
    subset: &[super::model::ArtifactIdentity],
    superset: &[super::model::ArtifactIdentity],
) -> bool {
    subset
        .iter()
        .all(|identity| superset.binary_search(identity).is_ok())
}

fn receipt_digest(receipt: &VocabularyAgreementReceipt) -> [u8; 32] {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, receipt.schema_id.as_bytes());
    append_field(&mut bytes, 2, receipt.claim_id.as_bytes());
    append_field(&mut bytes, 3, receipt.producer_id.as_bytes());
    append_field(&mut bytes, 4, receipt.watchdog_id.as_bytes());
    append_field(&mut bytes, 5, &receipt.root_count.to_be_bytes());
    append_field(&mut bytes, 6, &receipt.descriptor_role_count.to_be_bytes());
    append_field(&mut bytes, 7, &receipt.relation_target_count.to_be_bytes());
    append_field(&mut bytes, 8, &receipt.constraint_law_count.to_be_bytes());
    append_field(
        &mut bytes,
        9,
        &[u8::from(receipt.current_input_partition_complete)],
    );
    append_field(
        &mut bytes,
        10,
        &[u8::from(receipt.global_physical_vocabulary_coverage)],
    );
    append_field(&mut bytes, 11, &[u8::from(receipt.membership_authority)]);
    append_field(&mut bytes, 12, &receipt.producer_result_sha256);
    append_field(&mut bytes, 13, &receipt.watchdog_result_sha256);
    append_field(&mut bytes, 14, &receipt.producer_resource_contract_sha256);
    append_field(&mut bytes, 15, &receipt.watchdog_resource_contract_sha256);
    sha256(&bytes)
}

fn append_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u32::try_from(payload.len())
            .expect("fixed receipt fields fit u32")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}
