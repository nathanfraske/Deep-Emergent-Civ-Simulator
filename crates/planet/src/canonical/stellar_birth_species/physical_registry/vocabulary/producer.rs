use super::super::model::{
    AdmissionCapabilityKind, AdmissionRoute, AdmittedArtifact, ArtifactIdentity, ArtifactPayload,
    PhysicalVocabularyBinding,
};
use super::model::{
    VocabularyCheckerOutput, VocabularyPartitions, VocabularyRefusalCode,
    VocabularyResourceContract, PRODUCER_ID, VOCABULARY_CLAIM_ID, VOCABULARY_RECEIPT_SCHEMA_ID,
    VOCABULARY_SCHEMA_ID, WATCHDOG_ID,
};
use civsim_units::digest::sha256;

const RESOURCE_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-physical-vocabulary-producer-resource.v1";

pub(super) fn resource_contract_sha256() -> [u8; 32] {
    resource_contract_sha256_for(VocabularyResourceContract::PRODUCTION)
}

fn resource_contract_sha256_for(resources: VocabularyResourceContract) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(RESOURCE_DOMAIN.len() + 32);
    bytes.extend_from_slice(RESOURCE_DOMAIN);
    bytes.extend_from_slice(&resources.max_root_count.to_be_bytes());
    bytes.extend_from_slice(&resources.max_partition_identity_count.to_be_bytes());
    bytes.extend_from_slice(&resources.max_work_units.to_be_bytes());
    bytes.extend_from_slice(&resources.max_canonical_bytes.to_be_bytes());
    sha256(&bytes)
}

pub(super) fn classify(
    roots: &[AdmittedArtifact],
    resources: VocabularyResourceContract,
) -> Result<VocabularyCheckerOutput, VocabularyRefusalCode> {
    if roots.len()
        > usize::try_from(resources.max_root_count)
            .map_err(|_| VocabularyRefusalCode::RootCountExceeded)?
    {
        return Err(VocabularyRefusalCode::RootCountExceeded);
    }

    let mut roles = Vec::new();
    let mut targets = Vec::with_capacity(roots.len());
    let mut laws = Vec::new();
    for artifact in roots {
        validate_admission(artifact)?;
        targets.push(artifact.claimed_identity);
        match &artifact.payload {
            ArtifactPayload::PhysicalDescriptor(_) => roles.push(artifact.claimed_identity),
            ArtifactPayload::ConstraintLaw(_) => laws.push(artifact.claimed_identity),
            ArtifactPayload::ScalarCoordinate(_)
            | ArtifactPayload::MassProjection(_)
            | ArtifactPayload::ExactMasslessLaw(_)
            | ArtifactPayload::SpeciesDerivation(_) => {}
        }
    }

    roles.sort_unstable();
    targets.sort_unstable();
    laws.sort_unstable();
    if targets.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(VocabularyRefusalCode::DuplicateRootIdentity);
    }

    let partition_count = roles
        .len()
        .checked_add(targets.len())
        .and_then(|count| count.checked_add(laws.len()))
        .ok_or(VocabularyRefusalCode::PartitionIdentityCountExceeded)?;
    if partition_count
        > usize::try_from(resources.max_partition_identity_count)
            .map_err(|_| VocabularyRefusalCode::PartitionIdentityCountExceeded)?
    {
        return Err(VocabularyRefusalCode::PartitionIdentityCountExceeded);
    }
    let work_units = roots
        .len()
        .checked_mul(2)
        .and_then(|count| count.checked_add(partition_count))
        .and_then(|count| u64::try_from(count).ok())
        .ok_or(VocabularyRefusalCode::WorkLimitExceeded)?;
    if work_units > resources.max_work_units {
        return Err(VocabularyRefusalCode::WorkLimitExceeded);
    }

    let partitions = VocabularyPartitions {
        descriptor_role_identities: roles,
        relation_target_identities: targets,
        constraint_law_identities: laws,
    };
    let canonical_bytes = encode_partitions(&partitions, resources)?;
    Ok(VocabularyCheckerOutput {
        partitions,
        canonical_bytes,
        resource_contract_sha256: resource_contract_sha256_for(resources),
    })
}

pub(super) fn matches_binding(
    roots: &[AdmittedArtifact],
    binding: &PhysicalVocabularyBinding,
    resources: VocabularyResourceContract,
) -> bool {
    let Ok(output) = classify(roots, resources) else {
        return false;
    };
    let Ok(root_count) = u32::try_from(roots.len()) else {
        return false;
    };
    let Ok(descriptor_role_count) = u32::try_from(binding.descriptor_role_identities.len()) else {
        return false;
    };
    let Ok(relation_target_count) = u32::try_from(binding.relation_target_identities.len()) else {
        return false;
    };
    let Ok(constraint_law_count) = u32::try_from(binding.constraint_law_identities.len()) else {
        return false;
    };
    let result_sha256 = sha256(&output.canonical_bytes);
    binding.schema_id == VOCABULARY_RECEIPT_SCHEMA_ID
        && binding.claim_id == VOCABULARY_CLAIM_ID
        && binding.producer_id == PRODUCER_ID
        && binding.watchdog_id == WATCHDOG_ID
        && binding.root_count == root_count
        && relation_target_count == root_count
        && binding.current_input_partition_complete
        && !binding.global_physical_vocabulary_coverage
        && !binding.membership_authority
        && binding.descriptor_role_identities == output.partitions.descriptor_role_identities
        && binding.relation_target_identities == output.partitions.relation_target_identities
        && binding.constraint_law_identities == output.partitions.constraint_law_identities
        && binding.producer_result_sha256 == result_sha256
        && binding.watchdog_result_sha256 == result_sha256
        && binding.producer_resource_contract_sha256 == resource_contract_sha256_for(resources)
        && binding.watchdog_resource_contract_sha256 == resource_contract_sha256_for(resources)
        && binding.receipt_sha256 != [0; 32]
        && binding.receipt_sha256
            == binding_receipt_digest(
                binding,
                descriptor_role_count,
                relation_target_count,
                constraint_law_count,
            )
}

fn binding_receipt_digest(
    binding: &PhysicalVocabularyBinding,
    descriptor_role_count: u32,
    relation_target_count: u32,
    constraint_law_count: u32,
) -> [u8; 32] {
    let fields: [(u16, &[u8]); 15] = [
        (1, binding.schema_id.as_bytes()),
        (2, binding.claim_id.as_bytes()),
        (3, binding.producer_id.as_bytes()),
        (4, binding.watchdog_id.as_bytes()),
        (5, &binding.root_count.to_be_bytes()),
        (6, &descriptor_role_count.to_be_bytes()),
        (7, &relation_target_count.to_be_bytes()),
        (8, &constraint_law_count.to_be_bytes()),
        (9, &[u8::from(binding.current_input_partition_complete)]),
        (10, &[u8::from(binding.global_physical_vocabulary_coverage)]),
        (11, &[u8::from(binding.membership_authority)]),
        (12, &binding.producer_result_sha256),
        (13, &binding.watchdog_result_sha256),
        (14, &binding.producer_resource_contract_sha256),
        (15, &binding.watchdog_resource_contract_sha256),
    ];
    let mut bytes = Vec::new();
    for (tag, payload) in fields {
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(
            &u32::try_from(payload.len())
                .expect("fixed vocabulary receipt field fits u32")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(payload);
    }
    sha256(&bytes)
}

fn validate_admission(artifact: &AdmittedArtifact) -> Result<(), VocabularyRefusalCode> {
    if artifact.capability_claimed_identity() != artifact.claimed_identity
        || artifact.capability_admission() != &artifact.admission
    {
        return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
    }
    match artifact.admission_capability_kind() {
        AdmissionCapabilityKind::RepositoryRoot => {
            if artifact
                .repository_root_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::PrimitiveProfile => {
            if artifact
                .primitive_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
                || artifact
                    .primitive_profile_root_identity()
                    .is_none_or(|identity| identity.0 == [0; 32])
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::ChargedProfile => {
            if artifact
                .charged_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
                || artifact
                    .charged_profile_root_identity()
                    .is_none_or(|identity| identity.0 == [0; 32])
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::NeutralBoundProfile => {
            if artifact
                .neutral_bound_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
                || artifact
                    .neutral_bound_profile_root_identity()
                    .is_none_or(|identity| identity.0 == [0; 32])
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        #[cfg(test)]
        AdmissionCapabilityKind::ExactTest => {}
    }
    if matches!(
        artifact.admission.route,
        AdmissionRoute::EvidenceCustodyOnly { .. }
    ) {
        return Err(VocabularyRefusalCode::EvidenceCustodyOnly);
    }
    Ok(())
}

pub(super) fn encode_partitions(
    partitions: &VocabularyPartitions,
    resources: VocabularyResourceContract,
) -> Result<Vec<u8>, VocabularyRefusalCode> {
    if !is_strictly_ordered(&partitions.descriptor_role_identities)
        || !is_strictly_ordered(&partitions.relation_target_identities)
        || !is_strictly_ordered(&partitions.constraint_law_identities)
    {
        return Err(VocabularyRefusalCode::NoncanonicalPartition);
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(VOCABULARY_SCHEMA_ID.as_bytes());
    push_field(
        &mut bytes,
        1,
        &u32::try_from(partitions.relation_target_identities.len())
            .map_err(|_| VocabularyRefusalCode::RootCountExceeded)?
            .to_be_bytes(),
        resources,
    )?;
    for identity in &partitions.descriptor_role_identities {
        push_field(&mut bytes, 2, &identity.0, resources)?;
    }
    for identity in &partitions.relation_target_identities {
        push_field(&mut bytes, 3, &identity.0, resources)?;
    }
    for identity in &partitions.constraint_law_identities {
        push_field(&mut bytes, 4, &identity.0, resources)?;
    }
    push_field(&mut bytes, 5, &[1], resources)?;
    push_field(&mut bytes, 6, &[0], resources)?;
    push_field(&mut bytes, 7, &[0], resources)?;
    Ok(bytes)
}

fn push_field(
    bytes: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
    resources: VocabularyResourceContract,
) -> Result<(), VocabularyRefusalCode> {
    let payload_len =
        u32::try_from(payload.len()).map_err(|_| VocabularyRefusalCode::CanonicalBytesExceeded)?;
    let next_len = bytes
        .len()
        .checked_add(2 + 4)
        .and_then(|len| len.checked_add(payload.len()))
        .ok_or(VocabularyRefusalCode::CanonicalBytesExceeded)?;
    if next_len
        > usize::try_from(resources.max_canonical_bytes)
            .map_err(|_| VocabularyRefusalCode::CanonicalBytesExceeded)?
    {
        return Err(VocabularyRefusalCode::CanonicalBytesExceeded);
    }
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(&payload_len.to_be_bytes());
    bytes.extend_from_slice(payload);
    Ok(())
}

fn is_strictly_ordered(identities: &[ArtifactIdentity]) -> bool {
    identities.windows(2).all(|pair| pair[0] < pair[1])
}
