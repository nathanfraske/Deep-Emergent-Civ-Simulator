use std::collections::{BTreeMap, BTreeSet};

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
    let fields = [
        resources.max_root_count.to_be_bytes().to_vec(),
        resources
            .max_partition_identity_count
            .to_be_bytes()
            .to_vec(),
        resources.max_work_units.to_be_bytes().to_vec(),
        resources.max_canonical_bytes.to_be_bytes().to_vec(),
    ];
    let total_len = RESOURCE_DOMAIN.len() + fields.iter().map(Vec::len).sum::<usize>();
    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(RESOURCE_DOMAIN);
    for field in fields {
        bytes.extend(field);
    }
    sha256(&bytes)
}

pub(super) fn classify(
    roots: &[AdmittedArtifact],
    resources: VocabularyResourceContract,
) -> Result<VocabularyCheckerOutput, VocabularyRefusalCode> {
    let root_count =
        u32::try_from(roots.len()).map_err(|_| VocabularyRefusalCode::RootCountExceeded)?;
    if root_count > resources.max_root_count {
        return Err(VocabularyRefusalCode::RootCountExceeded);
    }

    let mut root_kinds = BTreeMap::new();
    for artifact in roots.iter().rev() {
        inspect_admission(artifact)?;
        let kind = match &artifact.payload {
            ArtifactPayload::PhysicalDescriptor(_) => RootKind::Descriptor,
            ArtifactPayload::ConstraintLaw(_) => RootKind::ConstraintLaw,
            ArtifactPayload::ScalarCoordinate(_)
            | ArtifactPayload::MassProjection(_)
            | ArtifactPayload::ExactMasslessLaw(_)
            | ArtifactPayload::SpeciesDerivation(_) => RootKind::Other,
        };
        if root_kinds.insert(artifact.claimed_identity, kind).is_some() {
            return Err(VocabularyRefusalCode::DuplicateRootIdentity);
        }
    }

    let targets = root_kinds.keys().copied().collect::<BTreeSet<_>>();
    let roles = root_kinds
        .iter()
        .filter_map(|(identity, kind)| (*kind == RootKind::Descriptor).then_some(*identity))
        .collect::<BTreeSet<_>>();
    let laws = root_kinds
        .iter()
        .filter_map(|(identity, kind)| (*kind == RootKind::ConstraintLaw).then_some(*identity))
        .collect::<BTreeSet<_>>();
    let partition_count = roles
        .len()
        .checked_add(targets.len())
        .and_then(|count| count.checked_add(laws.len()))
        .and_then(|count| u32::try_from(count).ok())
        .ok_or(VocabularyRefusalCode::PartitionIdentityCountExceeded)?;
    if partition_count > resources.max_partition_identity_count {
        return Err(VocabularyRefusalCode::PartitionIdentityCountExceeded);
    }

    let work_units = u64::from(root_count)
        .checked_mul(3)
        .and_then(|work| work.checked_add(u64::from(partition_count)))
        .ok_or(VocabularyRefusalCode::WorkLimitExceeded)?;
    if work_units > resources.max_work_units {
        return Err(VocabularyRefusalCode::WorkLimitExceeded);
    }

    let partitions = VocabularyPartitions {
        descriptor_role_identities: roles.into_iter().collect(),
        relation_target_identities: targets.into_iter().collect(),
        constraint_law_identities: laws.into_iter().collect(),
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
    let counts = (
        u32::try_from(roots.len()),
        u32::try_from(binding.descriptor_role_identities.len()),
        u32::try_from(binding.relation_target_identities.len()),
        u32::try_from(binding.constraint_law_identities.len()),
    );
    let (Ok(root_count), Ok(role_count), Ok(target_count), Ok(law_count)) = counts else {
        return false;
    };
    let local_resource = resource_contract_sha256_for(resources);
    let local_result = sha256(&output.canonical_bytes);
    let identities_match = [
        (
            binding.descriptor_role_identities.as_slice(),
            output.partitions.descriptor_role_identities.as_slice(),
        ),
        (
            binding.relation_target_identities.as_slice(),
            output.partitions.relation_target_identities.as_slice(),
        ),
        (
            binding.constraint_law_identities.as_slice(),
            output.partitions.constraint_law_identities.as_slice(),
        ),
    ]
    .into_iter()
    .all(|(bound, observed)| bound == observed);
    binding.schema_id.as_str() == VOCABULARY_RECEIPT_SCHEMA_ID
        && binding.claim_id.as_str() == VOCABULARY_CLAIM_ID
        && (binding.watchdog_id.as_str(), binding.producer_id.as_str())
            == (WATCHDOG_ID, PRODUCER_ID)
        && binding.root_count == root_count
        && target_count == root_count
        && binding.current_input_partition_complete
        && !(binding.global_physical_vocabulary_coverage || binding.membership_authority)
        && identities_match
        && (
            binding.watchdog_result_sha256,
            binding.producer_result_sha256,
        ) == (local_result, local_result)
        && (
            binding.watchdog_resource_contract_sha256,
            binding.producer_resource_contract_sha256,
        ) == (local_resource, local_resource)
        && binding.receipt_sha256.iter().any(|byte| *byte != 0)
        && binding.receipt_sha256
            == binding_receipt_digest(binding, role_count, target_count, law_count)
}

fn binding_receipt_digest(
    binding: &PhysicalVocabularyBinding,
    role_count: u32,
    target_count: u32,
    law_count: u32,
) -> [u8; 32] {
    let payloads = [
        binding.schema_id.as_bytes().to_vec(),
        binding.claim_id.as_bytes().to_vec(),
        binding.producer_id.as_bytes().to_vec(),
        binding.watchdog_id.as_bytes().to_vec(),
        binding.root_count.to_be_bytes().to_vec(),
        role_count.to_be_bytes().to_vec(),
        target_count.to_be_bytes().to_vec(),
        law_count.to_be_bytes().to_vec(),
        vec![u8::from(binding.current_input_partition_complete)],
        vec![u8::from(binding.global_physical_vocabulary_coverage)],
        vec![u8::from(binding.membership_authority)],
        binding.producer_result_sha256.to_vec(),
        binding.watchdog_result_sha256.to_vec(),
        binding.producer_resource_contract_sha256.to_vec(),
        binding.watchdog_resource_contract_sha256.to_vec(),
    ];
    let mut bytes = Vec::new();
    for (index, payload) in payloads.iter().enumerate() {
        let tag = u16::try_from(index + 1).expect("vocabulary receipt tag fits u16");
        let length = u32::try_from(payload.len()).expect("vocabulary receipt field fits u32");
        bytes.extend(tag.to_be_bytes());
        bytes.extend(length.to_be_bytes());
        bytes.extend(payload);
    }
    sha256(&bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootKind {
    Descriptor,
    ConstraintLaw,
    Other,
}

fn inspect_admission(artifact: &AdmittedArtifact) -> Result<(), VocabularyRefusalCode> {
    if artifact.claimed_identity != artifact.capability_claimed_identity()
        || &artifact.admission != artifact.capability_admission()
    {
        return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
    }
    match artifact.admission_capability_kind() {
        AdmissionCapabilityKind::RepositoryRoot => {
            match artifact.repository_root_pair_receipt_sha256() {
                Some(digest) if digest.iter().any(|byte| *byte != 0) => {}
                _ => return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch),
            }
        }
        AdmissionCapabilityKind::PrimitiveProfile => {
            if artifact
                .primitive_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest.iter().all(|byte| *byte == 0))
                || artifact
                    .primitive_profile_root_identity()
                    .is_none_or(|identity| identity.0.iter().all(|byte| *byte == 0))
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::ChargedProfile => {
            if artifact
                .charged_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest.iter().all(|byte| *byte == 0))
                || artifact
                    .charged_profile_root_identity()
                    .is_none_or(|identity| identity.0.iter().all(|byte| *byte == 0))
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::NeutralBoundProfile => {
            if artifact
                .neutral_bound_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest.iter().all(|byte| *byte == 0))
                || artifact
                    .neutral_bound_profile_root_identity()
                    .is_none_or(|identity| identity.0.iter().all(|byte| *byte == 0))
            {
                return Err(VocabularyRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        #[cfg(test)]
        AdmissionCapabilityKind::ExactTest => {}
    }
    match &artifact.admission.route {
        AdmissionRoute::Derived(_) | AdmissionRoute::Irreducible(_) => Ok(()),
        AdmissionRoute::EvidenceCustodyOnly { .. } => {
            Err(VocabularyRefusalCode::EvidenceCustodyOnly)
        }
    }
}

pub(super) fn encode_partitions(
    partitions: &VocabularyPartitions,
    resources: VocabularyResourceContract,
) -> Result<Vec<u8>, VocabularyRefusalCode> {
    if has_order_fault(&partitions.descriptor_role_identities)
        || has_order_fault(&partitions.relation_target_identities)
        || has_order_fault(&partitions.constraint_law_identities)
    {
        return Err(VocabularyRefusalCode::NoncanonicalPartition);
    }

    let mut frame = Frame::new(VOCABULARY_SCHEMA_ID.as_bytes(), resources)?;
    frame.add(
        1,
        &u32::try_from(partitions.relation_target_identities.len())
            .map_err(|_| VocabularyRefusalCode::RootCountExceeded)?
            .to_be_bytes(),
    )?;
    for (tag, identities) in [
        (2, partitions.descriptor_role_identities.as_slice()),
        (3, partitions.relation_target_identities.as_slice()),
        (4, partitions.constraint_law_identities.as_slice()),
    ] {
        for identity in identities {
            frame.add(tag, identity.0.as_slice())?;
        }
    }
    for (tag, flag) in [(5, true), (6, false), (7, false)] {
        frame.add(tag, &[u8::from(flag)])?;
    }
    Ok(frame.finish())
}

fn has_order_fault(identities: &[ArtifactIdentity]) -> bool {
    identities
        .iter()
        .zip(identities.iter().skip(1))
        .any(|(left, right)| left >= right)
}

struct Frame {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl Frame {
    fn new(
        domain: &[u8],
        resources: VocabularyResourceContract,
    ) -> Result<Self, VocabularyRefusalCode> {
        let max_bytes = usize::try_from(resources.max_canonical_bytes)
            .map_err(|_| VocabularyRefusalCode::CanonicalBytesExceeded)?;
        if domain.len() > max_bytes {
            return Err(VocabularyRefusalCode::CanonicalBytesExceeded);
        }
        Ok(Self {
            bytes: domain.to_vec(),
            max_bytes,
        })
    }

    fn add(&mut self, tag: u16, payload: &[u8]) -> Result<(), VocabularyRefusalCode> {
        let payload_len = u32::try_from(payload.len())
            .map_err(|_| VocabularyRefusalCode::CanonicalBytesExceeded)?;
        let required = 6usize
            .checked_add(payload.len())
            .and_then(|field_len| self.bytes.len().checked_add(field_len))
            .ok_or(VocabularyRefusalCode::CanonicalBytesExceeded)?;
        if required > self.max_bytes {
            return Err(VocabularyRefusalCode::CanonicalBytesExceeded);
        }
        self.bytes.extend(tag.to_be_bytes());
        self.bytes.extend(payload_len.to_be_bytes());
        self.bytes.extend(payload);
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
