//! Bottom-up physical species derivation producer.

mod resource;

use super::super::SpeciesContentIdentity;
use super::model::*;
use crate::canonical::stellar_birth_structure::stellar_birth_structure_schema;
use civsim_units::{
    bignum::BigUint, digest::sha256, physics_floor::sealed_physical_floor_authority_binding,
};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

const ARTIFACT_DOMAIN_V3: &[u8] = b"civsim.physical-species.artifact.v3";
const EXACT_MASSLESS_ARTIFACT_DOMAIN_V4: &[u8] = b"civsim.physical-species.artifact.v4";
const MEMBER_DOMAIN: &[u8] = b"civsim.physical-species.member.v2";
const EXPRESSION_DOMAIN: &[u8] = b"civsim.physical-species.expression.v1";
const EXPRESSION_NODE_DOMAIN: &[u8] = b"civsim.physical-species.expression-node.v1";
const REGISTRY_DOMAIN: &[u8] = b"civsim.physical-species.registry.v5";

#[derive(Debug, Clone)]
struct Fraction {
    negative: bool,
    numerator: BigUint,
    denominator: BigUint,
}

#[derive(Debug, Clone)]
struct EvaluatedScalar {
    value: Fraction,
    dimension: DimensionVector,
}

#[derive(Debug, Clone)]
struct ExpressionDigestBundle {
    digests: Vec<[u8; 32]>,
    output_digest: [u8; 32],
    edge_count: u32,
    depths: Vec<Option<u32>>,
    order: Vec<usize>,
}

#[derive(Debug, Clone)]
struct DerivedRule {
    output: VerifiedPhysicalMember,
    constituents: Vec<SpeciesContentIdentity>,
}

#[derive(Debug, Clone, Copy)]
struct WorkMeter {
    evaluation_left: u64,
    closure_left: u64,
}

impl WorkMeter {
    const fn new(caps: ValidationCaps) -> Self {
        Self {
            evaluation_left: caps.evaluation_steps,
            closure_left: caps.closure_steps,
        }
    }

    fn evaluation(&mut self, units: u64) -> Result<(), PhysicalRegistryRefusalCode> {
        self.evaluation_left = self
            .evaluation_left
            .checked_sub(units)
            .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
        Ok(())
    }

    fn closure(&mut self, units: u64) -> Result<(), PhysicalRegistryRefusalCode> {
        self.closure_left = self
            .closure_left
            .checked_sub(units)
            .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
        Ok(())
    }
}

struct RecordBuilder {
    bytes: Vec<u8>,
    cap: usize,
}

impl RecordBuilder {
    fn new(domain: &[u8], caps: ValidationCaps) -> Result<Self, PhysicalRegistryRefusalCode> {
        let mut builder = Self {
            bytes: Vec::new(),
            cap: usize::try_from(caps.canonical_bytes)
                .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?,
        };
        builder.append(domain)?;
        Ok(builder)
    }

    fn field(&mut self, tag: u16, payload: &[u8]) -> Result<(), PhysicalRegistryRefusalCode> {
        self.append(&tag.to_be_bytes())?;
        let length = u64::try_from(payload.len())
            .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        self.append(&length.to_be_bytes())?;
        self.append(payload)
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), PhysicalRegistryRefusalCode> {
        let next = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or(PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        if next > self.cap {
            return Err(PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(super) fn resource_contract_sha256() -> [u8; 32] {
    resource::contract_sha256()
}

#[cfg(test)]
pub(super) fn resource_contract_sha256_with_profile_for_test(work_profile_id: &[u8]) -> [u8; 32] {
    resource::contract_sha256_with_profile_for_test(work_profile_id)
}

pub(super) fn validate_and_encode(
    input: &PhysicalRegistryInput,
) -> Result<ValidatedRegistry, PhysicalRegistryRefusalCode> {
    validate_and_encode_with_caps(input, resource::PRODUCTION_CAPS)
}

pub(super) fn validate_and_encode_with_caps(
    input: &PhysicalRegistryInput,
    caps: ValidationCaps,
) -> Result<ValidatedRegistry, PhysicalRegistryRefusalCode> {
    validate_contract(input)?;
    validate_cardinalities(input, caps)?;

    let mut meter = WorkMeter::new(caps);
    let artifacts = validate_artifacts(input, caps, &mut meter)?;
    if artifacts.is_empty() {
        return Err(PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots);
    }
    let production_vocabulary_binding = input.admitted_artifacts.iter().any(|artifact| {
        artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
    });
    if production_vocabulary_binding
        && !super::vocabulary::producer_accepts_binding(
            &input.admitted_artifacts,
            &input.vocabulary_binding,
        )
    {
        return Err(PhysicalRegistryRefusalCode::PhysicalVocabularyBindingMismatch);
    }
    let rules = build_rules(&artifacts, caps, &mut meter)?;
    if rules.is_empty() {
        return Err(PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRules);
    }
    let dependency_count = rules.iter().try_fold(0_usize, |total, rule| {
        total
            .checked_add(rule.constituents.len())
            .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)
    })?;
    meter.closure(resource::closure_work_units(
        rules.len(),
        dependency_count,
        input.declared_members.len(),
    )?)?;
    let members = close_registry(&rules)?;
    if members.is_empty() {
        return Err(PhysicalRegistryRefusalCode::EmptyRegistryIsNotClosure);
    }
    compare_declared_closure(&members, &input.declared_members, caps)?;
    let canonical_bytes = encode_registry(input, &artifacts, &members, caps, &mut meter)?;
    Ok(ValidatedRegistry {
        members,
        canonical_bytes,
        resource_contract_sha256: resource_contract_sha256(),
    })
}

#[cfg(test)]
pub(super) fn derive_artifact_identity_for_test(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, PhysicalRegistryRefusalCode> {
    derive_artifact_identity_for_authority(payload)
}

pub(super) fn derive_artifact_identity_for_authority(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, PhysicalRegistryRefusalCode> {
    let mut meter = WorkMeter::new(ValidationCaps::PRODUCTION);
    derive_artifact_identity(payload, ValidationCaps::PRODUCTION, &mut meter)
}

#[cfg(test)]
pub(super) fn derive_member_identity_for_test(
    member: &MemberBlueprint,
) -> Result<SpeciesContentIdentity, PhysicalRegistryRefusalCode> {
    derive_member_identity_for_authority(member)
}

pub(super) fn derive_member_identity_for_authority(
    member: &MemberBlueprint,
) -> Result<SpeciesContentIdentity, PhysicalRegistryRefusalCode> {
    let mut meter = WorkMeter::new(ValidationCaps::PRODUCTION);
    derive_member_identity(member, ValidationCaps::PRODUCTION, &mut meter)
}

fn validate_contract(input: &PhysicalRegistryInput) -> Result<(), PhysicalRegistryRefusalCode> {
    if input.schema_id != REGISTRY_SCHEMA_ID || input.proof_graph_schema_id != PROOF_GRAPH_SCHEMA_ID
    {
        return Err(PhysicalRegistryRefusalCode::SchemaMismatch);
    }
    if input.checker_pair.producer_id != PRODUCER_ID
        || input.checker_pair.watchdog_id != WATCHDOG_ID
    {
        return Err(PhysicalRegistryRefusalCode::CheckerPairMismatch);
    }
    if input.resources != resource::PRODUCTION_RESOURCE_CONTRACT {
        return Err(PhysicalRegistryRefusalCode::ResourceContractMismatch);
    }
    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PhysicalRegistryRefusalCode::FloorBindingMismatch)?;
    if input.floor_binding.schema_id != floor.schema_id().as_str()
        || input.floor_binding.digest_sha256 == [0; 32]
        || input.floor_binding.digest_sha256 != floor.digest()
    {
        return Err(PhysicalRegistryRefusalCode::FloorBindingMismatch);
    }

    let structure = stellar_birth_structure_schema()
        .map_err(|_| PhysicalRegistryRefusalCode::StructureBindingMismatch)?;
    let expected = (
        structure.schema_id,
        structure.species_registry.schema_id,
        structure.stellar_state.schema_id,
        structure.stellar_state.state_coordinate_registry.schema_id,
        structure
            .stellar_state
            .interaction_sector_registry
            .schema_id,
        structure.stellar_state.physical_regime_registry.schema_id,
    );
    let found = (
        input.structure_binding.structure_schema_id.as_str(),
        input.structure_binding.species_registry_schema_id.as_str(),
        input.structure_binding.stellar_state_schema_id.as_str(),
        input
            .structure_binding
            .state_coordinate_registry_schema_id
            .as_str(),
        input
            .structure_binding
            .interaction_sector_registry_schema_id
            .as_str(),
        input
            .structure_binding
            .physical_regime_registry_schema_id
            .as_str(),
    );
    if found != expected {
        return Err(PhysicalRegistryRefusalCode::StructureBindingMismatch);
    }
    Ok(())
}

fn validate_cardinalities(
    input: &PhysicalRegistryInput,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if input.admitted_artifacts.len()
        > usize::try_from(caps.artifact_count)
            .map_err(|_| PhysicalRegistryRefusalCode::ArtifactCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::ArtifactCapacityExceeded);
    }
    if input.declared_members.len()
        > usize::try_from(caps.registry_member_count)
            .map_err(|_| PhysicalRegistryRefusalCode::RegistryCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::RegistryCapacityExceeded);
    }
    Ok(())
}

fn validate_artifacts<'a>(
    input: &'a PhysicalRegistryInput,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<BTreeMap<ArtifactIdentity, &'a AdmittedArtifact>, PhysicalRegistryRefusalCode> {
    let mut grouped = BTreeMap::<ArtifactIdentity, &AdmittedArtifact>::new();
    for artifact in &input.admitted_artifacts {
        if let Some(existing) = grouped.get(&artifact.claimed_identity) {
            return Err(if existing.payload == artifact.payload {
                PhysicalRegistryRefusalCode::DuplicateArtifactIdentity
            } else {
                PhysicalRegistryRefusalCode::ArtifactIdentityCollision
            });
        }
        grouped.insert(artifact.claimed_identity, artifact);
    }

    let mut residual_slots = BTreeSet::new();
    let mut total_references = 0_u32;
    for artifact in grouped.values() {
        validate_admission(&artifact.admission, caps, &mut residual_slots)?;
        let reference_count = artifact_reference_count(&artifact.payload)?;
        if reference_count > caps.references_per_artifact {
            return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
        }
        total_references = total_references
            .checked_add(reference_count)
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?;
        if total_references > caps.total_reference_count {
            return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
        }

        let expected = derive_artifact_identity(&artifact.payload, caps, meter)?;
        if expected != artifact.claimed_identity {
            return Err(PhysicalRegistryRefusalCode::ArtifactIdentityMismatch);
        }
        validate_admission_capability(artifact)?;
    }
    for artifact in grouped.values() {
        if let ArtifactPayload::MassProjection(projection) = &artifact.payload {
            validate_mass_uncertainty_transport(projection, &grouped, caps)?;
        }
    }
    Ok(grouped)
}

fn validate_mass_uncertainty_transport(
    projection: &MassProjectionArtifact,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let proof = match (projection.scope, projection.uncertainty_transport.as_ref()) {
        (MassProjectionScope::MembershipNeutral, None) => return Ok(()),
        (MassProjectionScope::SpeciesRestMass, Some(proof)) => proof,
        _ => return Err(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid),
    };
    for receipt in [
        &proof.source_pair_receipt,
        &proof.producer_receipt,
        &proof.watchdog_receipt,
    ] {
        validate_receipt(receipt, caps)
            .map_err(|_| PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid)?;
    }
    validate_distinct_receipts([
        &proof.source_pair_receipt,
        &proof.producer_receipt,
        &proof.watchdog_receipt,
    ])
    .map_err(|_| PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid)?;
    if !projection.expression.nodes.iter().any(
        |node| matches!(node, ExactExpressionNode::Coordinate(identity) if *identity == proof.source_coordinate),
    ) {
        return Err(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid);
    }
    let source = artifacts
        .get(&proof.source_coordinate)
        .ok_or(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid)?;
    if !matches!(source.payload, ArtifactPayload::ScalarCoordinate(_)) {
        return Err(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid);
    }
    match source.admission_capability_kind() {
        AdmissionCapabilityKind::RepositoryRoot => {
            if source.repository_root_pair_receipt_sha256()
                != Some(proof.source_pair_receipt.digest_sha256)
            {
                return Err(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid);
            }
        }
        #[cfg(test)]
        AdmissionCapabilityKind::ExactTest => {}
        AdmissionCapabilityKind::PrimitiveProfile | AdmissionCapabilityKind::ChargedProfile => {
            return Err(PhysicalRegistryRefusalCode::MassUncertaintyTransportInvalid);
        }
    }
    Ok(())
}

fn validate_admission_capability(
    artifact: &AdmittedArtifact,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if artifact.capability_claimed_identity() != artifact.claimed_identity
        || artifact.capability_admission() != &artifact.admission
    {
        return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
    }
    match artifact.admission_capability_kind() {
        AdmissionCapabilityKind::RepositoryRoot => {
            if artifact
                .repository_root_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
            {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::PrimitiveProfile => {
            if artifact
                .primitive_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
                || artifact
                    .primitive_profile_root_identity()
                    .is_none_or(|identity| identity.0 == [0; 32])
                || artifact.repository_root_pair_receipt_sha256().is_some()
            {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        AdmissionCapabilityKind::ChargedProfile => {
            if artifact
                .charged_profile_pair_receipt_sha256()
                .is_none_or(|digest| digest == [0; 32])
                || artifact
                    .charged_profile_root_identity()
                    .is_none_or(|identity| identity.0 == [0; 32])
                || artifact.repository_root_pair_receipt_sha256().is_some()
                || artifact.primitive_profile_pair_receipt_sha256().is_some()
            {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        #[cfg(test)]
        AdmissionCapabilityKind::ExactTest => {
            if artifact.repository_root_pair_receipt_sha256().is_some()
                || artifact.primitive_profile_pair_receipt_sha256().is_some()
                || artifact.charged_profile_pair_receipt_sha256().is_some()
            {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
    }
    Ok(())
}

fn validate_admission(
    admission: &RootAdmission,
    caps: ValidationCaps,
    residual_slots: &mut BTreeSet<String>,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if !admission.provenance.is_canonical() {
        return Err(PhysicalRegistryRefusalCode::NoncanonicalProvenance);
    }
    if matches!(
        admission.provenance,
        ProvenanceMark::WrittenState | ProvenanceMark::Contingency
    ) {
        return Err(PhysicalRegistryRefusalCode::GeneratedProvenanceCannotBeRoot);
    }
    match &admission.route {
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            validate_receipt(source_receipt, caps)?;
            Err(PhysicalRegistryRefusalCode::EvidenceCustodyIsNotAdmission)
        }
        AdmissionRoute::Derived(route) => {
            if admission.provenance != ProvenanceMark::Derived {
                return Err(PhysicalRegistryRefusalCode::DerivedAdmissionProvenanceMismatch);
            }
            for receipt in [
                &route.ancestry_receipt,
                &route.semantic_checker_receipt,
                &route.independent_watchdog_receipt,
            ] {
                validate_receipt(receipt, caps)?;
            }
            validate_distinct_receipts([
                &route.ancestry_receipt,
                &route.semantic_checker_receipt,
                &route.independent_watchdog_receipt,
            ])?;
            Ok(())
        }
        AdmissionRoute::Irreducible(route) => {
            if admission.provenance == ProvenanceMark::Derived {
                return Err(PhysicalRegistryRefusalCode::IrreducibleAdmissionProvenanceInvalid);
            }
            if !canonical_token(&route.residual_slot_id, caps) {
                return Err(PhysicalRegistryRefusalCode::CanonicalTextInvalid);
            }
            if !residual_slots.insert(route.residual_slot_id.clone()) {
                return Err(PhysicalRegistryRefusalCode::DuplicateResidualSlot);
            }
            for receipt in [
                &route.derivation_exhaustion_receipt,
                &route.buckingham_pi_receipt,
                &route.gap_law_receipt,
                &route.chaos_protocol_receipt,
                &route.residual_law_receipt,
                &route.residual_slot_receipt,
                &route.owner_admission_receipt,
                &route.independent_watchdog_receipt,
            ] {
                validate_receipt(receipt, caps)?;
            }
            validate_distinct_receipts([
                &route.derivation_exhaustion_receipt,
                &route.buckingham_pi_receipt,
                &route.gap_law_receipt,
                &route.chaos_protocol_receipt,
                &route.residual_law_receipt,
                &route.residual_slot_receipt,
                &route.owner_admission_receipt,
                &route.independent_watchdog_receipt,
            ])?;
            Ok(())
        }
    }
}

fn validate_distinct_receipts<const N: usize>(
    receipts: [&ReceiptBinding; N],
) -> Result<(), PhysicalRegistryRefusalCode> {
    let distinct = receipts.into_iter().collect::<BTreeSet<_>>();
    if distinct.len() != N {
        return Err(PhysicalRegistryRefusalCode::DuplicateAdmissionReceipt);
    }
    Ok(())
}

fn validate_receipt(
    receipt: &ReceiptBinding,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if !canonical_token(&receipt.schema_id, caps) {
        return Err(PhysicalRegistryRefusalCode::CanonicalTextInvalid);
    }
    if receipt.digest_sha256 == [0; 32] {
        return Err(PhysicalRegistryRefusalCode::MissingBindingDigest);
    }
    Ok(())
}

fn canonical_token(value: &str, caps: ValidationCaps) -> bool {
    !value.is_empty()
        && value.len() <= usize::try_from(caps.canonical_token_bytes).unwrap_or(0)
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/' | b'[' | b']')
        })
}

fn validate_content(
    content: &CanonicalArtifact,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if !canonical_token(&content.schema_id, caps) {
        return Err(PhysicalRegistryRefusalCode::CanonicalTextInvalid);
    }
    if content.canonical_bytes.is_empty() {
        return Err(PhysicalRegistryRefusalCode::ContentByteLimitExceeded);
    }
    if content.canonical_bytes.len()
        > usize::try_from(caps.content_bytes)
            .map_err(|_| PhysicalRegistryRefusalCode::ContentByteLimitExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::ContentByteLimitExceeded);
    }
    Ok(())
}

fn artifact_reference_count(payload: &ArtifactPayload) -> Result<u32, PhysicalRegistryRefusalCode> {
    let count = match payload {
        ArtifactPayload::ScalarCoordinate(_) | ArtifactPayload::PhysicalDescriptor(_) => 0_usize,
        ArtifactPayload::ConstraintLaw(law) => requirement_reference_count(&law.requirements)?,
        ArtifactPayload::MassProjection(projection) => projection
            .expression
            .nodes
            .iter()
            .try_fold(0_usize, |count, node| {
                count
                    .checked_add(match node {
                        ExactExpressionNode::Coordinate(_) => 1,
                        ExactExpressionNode::IntegerPower { .. } => 1,
                        _ => 2,
                    })
                    .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
            })?
            .checked_add(usize::from(projection.uncertainty_transport.is_some()))
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?,
        ArtifactPayload::ExactMasslessLaw(law) => {
            checked_reference_lengths(&[requirement_reference_count(&law.requirements)?, 2])?
        }
        ArtifactPayload::SpeciesDerivation(rule) => checked_reference_lengths(&[
            1,
            relation_reference_count(&rule.artifact_inputs)?,
            rule.constituents.len(),
            member_reference_count(&rule.output)?,
        ])?,
    };
    u32::try_from(count).map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
}

fn checked_reference_lengths(lengths: &[usize]) -> Result<usize, PhysicalRegistryRefusalCode> {
    lengths.iter().try_fold(0_usize, |count, length| {
        count
            .checked_add(*length)
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
    })
}

fn requirement_reference_count(
    requirements: &RequirementSet,
) -> Result<usize, PhysicalRegistryRefusalCode> {
    checked_reference_lengths(&[
        relation_reference_count(&requirements.artifact_relations)?,
        requirements.species_dependencies.len(),
    ])
}

fn member_reference_count(member: &MemberBlueprint) -> Result<usize, PhysicalRegistryRefusalCode> {
    checked_reference_lengths(&[
        requirement_reference_count(&member.requirements)?,
        1,
        relation_reference_count(&member.constraint_laws)?,
    ])
}

fn relation_reference_count(
    relations: &[ArtifactRelation],
) -> Result<usize, PhysicalRegistryRefusalCode> {
    relations
        .len()
        .checked_mul(2)
        .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
}

fn derive_artifact_identity(
    payload: &ArtifactPayload,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<ArtifactIdentity, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::artifact_identity_work_units())?;
    Ok(ArtifactIdentity(sha256(&encode_artifact_payload(
        payload, caps, meter,
    )?)))
}

fn build_rules(
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Vec<DerivedRule>, PhysicalRegistryRefusalCode> {
    let mut rules = Vec::new();
    let mut identities = BTreeMap::<SpeciesContentIdentity, VerifiedPhysicalMember>::new();
    for artifact in artifacts.values() {
        let ArtifactPayload::SpeciesDerivation(rule) = &artifact.payload else {
            continue;
        };
        validate_descriptor_reference(rule.derivation_kind, artifacts)?;
        let artifact_inputs =
            validate_artifact_relations(&rule.artifact_inputs, artifacts, caps, |_| true)?;
        if artifact_inputs.is_empty() {
            return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
        }
        let constituents = sorted_unique_species(&rule.constituents)?;

        let member = validate_member(&rule.output, rule.derivation_kind, artifacts, caps, meter)?;
        if member.requirements.species_dependencies != constituents {
            return Err(PhysicalRegistryRefusalCode::DependencyMismatch);
        }
        if identities.insert(member.identity, member.clone()).is_some() {
            return Err(PhysicalRegistryRefusalCode::DuplicateMemberDerivation);
        }
        rules.push(DerivedRule {
            output: member,
            constituents,
        });
    }
    let outputs = rules
        .iter()
        .map(|rule| rule.output.identity)
        .collect::<BTreeSet<_>>();
    if rules.iter().any(|rule| {
        rule.constituents
            .iter()
            .any(|identity| !outputs.contains(identity))
    }) {
        return Err(PhysicalRegistryRefusalCode::UnknownSpeciesDependency);
    }
    rules.sort_by_key(|rule| rule.output.identity);
    Ok(rules)
}

fn validate_descriptor_reference(
    identity: ArtifactIdentity,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let descriptor = artifacts
        .get(&identity)
        .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
    if !matches!(descriptor.payload, ArtifactPayload::PhysicalDescriptor(_)) {
        return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
    }
    Ok(())
}

fn validate_artifact_relations<F>(
    relations: &[ArtifactRelation],
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    target_matches: F,
) -> Result<Vec<ArtifactRelation>, PhysicalRegistryRefusalCode>
where
    F: Fn(&ArtifactPayload) -> bool,
{
    let relations = sorted_unique_relations(relations)?;
    if relations.len()
        > usize::try_from(caps.references_per_artifact)
            .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
    }
    for relation in &relations {
        validate_descriptor_reference(relation.role, artifacts)?;
        let target = artifacts
            .get(&relation.target)
            .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
        if !target_matches(&target.payload) {
            return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
        }
    }
    Ok(relations)
}

fn sorted_unique_relations(
    values: &[ArtifactRelation],
) -> Result<Vec<ArtifactRelation>, PhysicalRegistryRefusalCode> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PhysicalRegistryRefusalCode::DuplicateRequirement);
    }
    Ok(sorted)
}

fn validate_member(
    blueprint: &MemberBlueprint,
    derivation_kind: ArtifactIdentity,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<VerifiedPhysicalMember, PhysicalRegistryRefusalCode> {
    validate_content(&blueprint.physical_content, caps)?;
    validate_descriptor_reference(derivation_kind, artifacts)?;
    let requirements = normalize_requirements(&blueprint.requirements, caps)?;
    validate_artifact_relations(&requirements.artifact_relations, artifacts, caps, |_| true)?;

    let constraints =
        validate_artifact_relations(&blueprint.constraint_laws, artifacts, caps, |payload| {
            matches!(payload, ArtifactPayload::ConstraintLaw(_))
        })?;
    if constraints.is_empty() {
        return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
    }
    for relation in constraints {
        let constraint = artifacts
            .get(&relation.target)
            .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
        let ArtifactPayload::ConstraintLaw(constraint) = &constraint.payload else {
            return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
        };
        if normalize_requirements(&constraint.requirements, caps)? != requirements {
            return Err(PhysicalRegistryRefusalCode::DependencyMismatch);
        }
    }

    let (rest_mass_si, mass_dimension) = match blueprint.mass_proof {
        MassProofReference::Projection(identity) => {
            let projection = artifacts
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
            let ArtifactPayload::MassProjection(projection) = &projection.payload else {
                return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
            };
            if projection.scope != MassProjectionScope::SpeciesRestMass {
                return Err(PhysicalRegistryRefusalCode::MassProjectionNotAuthorizedForMember);
            }
            let scalar = evaluate_expression(&projection.expression, artifacts, caps, meter)?;
            if scalar.dimension != MASS_DIMENSION {
                return Err(PhysicalRegistryRefusalCode::MassDimensionMismatch);
            }
            if scalar.value.negative || scalar.value.numerator.is_zero() {
                return Err(PhysicalRegistryRefusalCode::NonPositiveMass);
            }
            (fraction_to_wire(&scalar.value)?, scalar.dimension)
        }
        MassProofReference::ExactMassless(identity) => {
            let massless = artifacts
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
            let ArtifactPayload::ExactMasslessLaw(massless) = &massless.payload else {
                return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
            };
            if normalize_requirements(&massless.requirements, caps)? != requirements {
                return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
            }
            validate_exact_zero_mass_proof(&massless.proof, &requirements, artifacts, caps)?;
            (
                ExactRationalWire {
                    negative: false,
                    numerator_be: vec![0],
                    denominator_be: vec![1],
                },
                MASS_DIMENSION,
            )
        }
    };

    let identity = derive_member_identity(blueprint, caps, meter)?;
    Ok(VerifiedPhysicalMember {
        identity,
        physical_content: blueprint.physical_content.clone(),
        rest_mass_si,
        mass_dimension,
        derivation_kind,
        requirements,
    })
}

fn validate_exact_zero_mass_proof(
    proof: &ExactZeroMassProof,
    requirements: &RequirementSet,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if proof.subject == proof.symmetry {
        return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
    }
    validate_descriptor_reference(proof.subject, artifacts)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    validate_descriptor_reference(proof.symmetry, artifacts)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    validate_content(&proof.excluded_term, caps)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    for receipt in [
        &proof.applicability_receipt,
        &proof.exclusion_producer_receipt,
        &proof.exclusion_watchdog_receipt,
    ] {
        validate_receipt(receipt, caps)
            .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    }
    validate_distinct_receipts([
        &proof.applicability_receipt,
        &proof.exclusion_producer_receipt,
        &proof.exclusion_watchdog_receipt,
    ])
    .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    let targets = requirements
        .artifact_relations
        .iter()
        .map(|relation| relation.target)
        .collect::<BTreeSet<_>>();
    if !targets.contains(&proof.subject) || !targets.contains(&proof.symmetry) {
        return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
    }
    Ok(())
}

fn normalize_requirements(
    requirements: &RequirementSet,
    caps: ValidationCaps,
) -> Result<RequirementSet, PhysicalRegistryRefusalCode> {
    let artifact_relations = sorted_unique_relations(&requirements.artifact_relations)?;
    let species_dependencies = sorted_unique_species(&requirements.species_dependencies)?;
    if artifact_relations.is_empty() {
        return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
    }
    for length in [artifact_relations.len(), species_dependencies.len()] {
        if length
            > usize::try_from(caps.references_per_artifact)
                .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?
        {
            return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
        }
    }
    Ok(RequirementSet {
        artifact_relations,
        species_dependencies,
    })
}

fn sorted_unique_species(
    values: &[SpeciesContentIdentity],
) -> Result<Vec<SpeciesContentIdentity>, PhysicalRegistryRefusalCode> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PhysicalRegistryRefusalCode::DuplicateRequirement);
    }
    Ok(sorted)
}

fn close_registry(
    rules: &[DerivedRule],
) -> Result<Vec<VerifiedPhysicalMember>, PhysicalRegistryRefusalCode> {
    let output_to_rule = rules
        .iter()
        .enumerate()
        .map(|(index, rule)| (rule.output.identity, index))
        .collect::<BTreeMap<_, _>>();
    let mut remaining = rules
        .iter()
        .map(|rule| rule.constituents.len())
        .collect::<Vec<_>>();
    let mut dependents = vec![Vec::<usize>::new(); rules.len()];
    for (index, rule) in rules.iter().enumerate() {
        for constituent in &rule.constituents {
            let dependency = *output_to_rule
                .get(constituent)
                .ok_or(PhysicalRegistryRefusalCode::UnknownSpeciesDependency)?;
            dependents[dependency].push(index);
        }
    }
    for successors in &mut dependents {
        successors.sort_unstable();
    }
    let mut ready = remaining
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut closed = BTreeMap::<SpeciesContentIdentity, VerifiedPhysicalMember>::new();
    while let Some(index) = ready.pop_first() {
        let rule = &rules[index];
        if closed
            .insert(rule.output.identity, rule.output.clone())
            .is_some()
        {
            return Err(PhysicalRegistryRefusalCode::DuplicateMemberDerivation);
        }
        for dependent in &dependents[index] {
            let count = remaining
                .get_mut(*dependent)
                .ok_or(PhysicalRegistryRefusalCode::DerivationCycle)?;
            *count = count
                .checked_sub(1)
                .ok_or(PhysicalRegistryRefusalCode::DerivationCycle)?;
            if *count == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if closed.len() != rules.len() {
        return Err(PhysicalRegistryRefusalCode::DerivationCycle);
    }
    Ok(closed.into_values().collect())
}

fn compare_declared_closure(
    members: &[VerifiedPhysicalMember],
    declared: &[SpeciesContentIdentity],
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if declared.len()
        > usize::try_from(caps.registry_member_count)
            .map_err(|_| PhysicalRegistryRefusalCode::RegistryCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::RegistryCapacityExceeded);
    }
    let mut declared_sorted = declared.to_vec();
    declared_sorted.sort_unstable();
    if declared_sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PhysicalRegistryRefusalCode::DuplicateRegistryMember);
    }
    let derived = members
        .iter()
        .map(|member| member.identity)
        .collect::<Vec<_>>();
    if derived
        .iter()
        .any(|identity| declared_sorted.binary_search(identity).is_err())
    {
        return Err(PhysicalRegistryRefusalCode::MissingClosureMember);
    }
    if declared_sorted
        .iter()
        .any(|identity| derived.binary_search(identity).is_err())
    {
        return Err(PhysicalRegistryRefusalCode::ExtraClosureMember);
    }
    Ok(())
}

fn evaluate_expression(
    expression: &ExactExpression,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<EvaluatedScalar, PhysicalRegistryRefusalCode> {
    let ExpressionDigestBundle { depths, order, .. } = expression_digests(expression, caps, meter)?;
    let mut values = vec![None::<EvaluatedScalar>; expression.nodes.len()];
    for index in order {
        meter.evaluation(resource::expression_node_execution_work_units())?;
        let value = match &expression.nodes[index] {
            ExactExpressionNode::Coordinate(identity) => {
                let artifact = artifacts
                    .get(identity)
                    .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
                let ArtifactPayload::ScalarCoordinate(coordinate) = &artifact.payload else {
                    return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
                };
                EvaluatedScalar {
                    value: parse_rational(&coordinate.exact_value, caps, meter)?,
                    dimension: checked_dimension(coordinate.dimension, caps)?,
                }
            }
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Subtract { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                let left_value = indexed_value(&values, *left)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                let right_value = indexed_value(&values, *right)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                match &expression.nodes[index] {
                    ExactExpressionNode::Add { .. } => {
                        if left_value.dimension != right_value.dimension {
                            return Err(PhysicalRegistryRefusalCode::DimensionMismatch);
                        }
                        EvaluatedScalar {
                            value: add_fraction(
                                &left_value.value,
                                &right_value.value,
                                false,
                                caps,
                                meter,
                            )?,
                            dimension: left_value.dimension,
                        }
                    }
                    ExactExpressionNode::Subtract { .. } => {
                        if left_value.dimension != right_value.dimension {
                            return Err(PhysicalRegistryRefusalCode::DimensionMismatch);
                        }
                        EvaluatedScalar {
                            value: add_fraction(
                                &left_value.value,
                                &right_value.value,
                                true,
                                caps,
                                meter,
                            )?,
                            dimension: left_value.dimension,
                        }
                    }
                    ExactExpressionNode::Multiply { .. } => EvaluatedScalar {
                        value: multiply_fraction(
                            &left_value.value,
                            &right_value.value,
                            caps,
                            meter,
                        )?,
                        dimension: combine_dimensions(
                            left_value.dimension,
                            right_value.dimension,
                            false,
                            caps,
                        )?,
                    },
                    _ => return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid),
                }
            }
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                let numerator_value = indexed_value(&values, *numerator)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                let denominator_value = indexed_value(&values, *denominator)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                EvaluatedScalar {
                    value: divide_fraction(
                        &numerator_value.value,
                        &denominator_value.value,
                        caps,
                        meter,
                    )?,
                    dimension: combine_dimensions(
                        numerator_value.dimension,
                        denominator_value.dimension,
                        true,
                        caps,
                    )?,
                }
            }
            ExactExpressionNode::IntegerPower { base, exponent } => {
                let base_value = indexed_value(&values, *base)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                EvaluatedScalar {
                    value: power_fraction(&base_value.value, *exponent, caps, meter)?,
                    dimension: power_dimension(base_value.dimension, *exponent, caps)?,
                }
            }
        };
        values[index] = Some(value);
    }
    let output = usize::try_from(expression.output_node)
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    if depths.get(output).copied().flatten().is_none() {
        return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid);
    }
    values
        .get(output)
        .and_then(Clone::clone)
        .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)
}

fn indexed_value(
    values: &[Option<EvaluatedScalar>],
    index: u32,
) -> Result<Option<EvaluatedScalar>, PhysicalRegistryRefusalCode> {
    let index =
        usize::try_from(index).map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    values
        .get(index)
        .cloned()
        .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)
}

fn expression_digests(
    expression: &ExactExpression,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<ExpressionDigestBundle, PhysicalRegistryRefusalCode> {
    if expression.nodes.is_empty() {
        return Err(PhysicalRegistryRefusalCode::ExpressionEmpty);
    }
    if expression.nodes.len()
        > usize::try_from(caps.expression_node_count)
            .map_err(|_| PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded);
    }
    let output = usize::try_from(expression.output_node)
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    if output >= expression.nodes.len() {
        return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid);
    }

    let mut edge_count = 0_u32;
    for node in &expression.nodes {
        edge_count = edge_count
            .checked_add(match node {
                ExactExpressionNode::Coordinate(_) => 0,
                ExactExpressionNode::IntegerPower { .. } => 1,
                _ => 2,
            })
            .ok_or(PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded)?;
    }
    if edge_count > caps.expression_edge_count {
        return Err(PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded);
    }
    let node_count = u32::try_from(expression.nodes.len())
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded)?;
    meter.evaluation(resource::expression_work_units(node_count, edge_count)?)?;

    let reachable = reachable_expression_nodes(expression)?;
    if reachable.len() != expression.nodes.len() {
        return Err(PhysicalRegistryRefusalCode::ExpressionContainsUnusedNode);
    }

    let mut indegrees = vec![0_u32; expression.nodes.len()];
    let mut dependents = vec![Vec::<usize>::new(); expression.nodes.len()];
    for (index, node) in expression.nodes.iter().enumerate() {
        let mut add_dependency = |dependency: u32| {
            let dependency = expression_index(dependency, expression.nodes.len())?;
            indegrees[index] = indegrees[index]
                .checked_add(1)
                .ok_or(PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded)?;
            dependents[dependency].push(index);
            Ok::<(), PhysicalRegistryRefusalCode>(())
        };
        match node {
            ExactExpressionNode::Coordinate(_) => {}
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Subtract { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                add_dependency(*left)?;
                add_dependency(*right)?;
            }
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                add_dependency(*numerator)?;
                add_dependency(*denominator)?;
            }
            ExactExpressionNode::IntegerPower { base, .. } => add_dependency(*base)?,
        }
    }
    for successors in &mut dependents {
        successors.sort_unstable();
    }

    let mut ready = indegrees
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect::<BTreeSet<_>>();
    let mut digests = vec![None::<[u8; 32]>; expression.nodes.len()];
    let mut depths = vec![None::<u32>; expression.nodes.len()];
    let mut order = Vec::with_capacity(expression.nodes.len());
    while let Some(index) = ready.pop_first() {
        let (digest, depth) = match &expression.nodes[index] {
            ExactExpressionNode::Coordinate(identity) => {
                let mut record = RecordBuilder::new(EXPRESSION_NODE_DOMAIN, caps)?;
                record.field(1, b"coordinate")?;
                record.field(2, &identity.0)?;
                (sha256(&record.finish()), 1)
            }
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                let (mut left_digest, left_depth) = indexed_digest(&digests, &depths, *left)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let (mut right_digest, right_depth) = indexed_digest(&digests, &depths, *right)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                if right_digest < left_digest {
                    std::mem::swap(&mut left_digest, &mut right_digest);
                }
                let mut record = RecordBuilder::new(EXPRESSION_NODE_DOMAIN, caps)?;
                record.field(
                    1,
                    if matches!(expression.nodes[index], ExactExpressionNode::Add { .. }) {
                        b"add"
                    } else {
                        b"multiply"
                    },
                )?;
                record.field(2, &left_digest)?;
                record.field(3, &right_digest)?;
                (
                    sha256(&record.finish()),
                    left_depth.max(right_depth).saturating_add(1),
                )
            }
            ExactExpressionNode::Subtract { left, right } => {
                let (left_digest, left_depth) = indexed_digest(&digests, &depths, *left)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let (right_digest, right_depth) = indexed_digest(&digests, &depths, *right)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let mut record = RecordBuilder::new(EXPRESSION_NODE_DOMAIN, caps)?;
                record.field(1, b"subtract")?;
                record.field(2, &left_digest)?;
                record.field(3, &right_digest)?;
                (
                    sha256(&record.finish()),
                    left_depth.max(right_depth).saturating_add(1),
                )
            }
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                let (numerator_digest, numerator_depth) =
                    indexed_digest(&digests, &depths, *numerator)?
                        .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let (denominator_digest, denominator_depth) =
                    indexed_digest(&digests, &depths, *denominator)?
                        .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let mut record = RecordBuilder::new(EXPRESSION_NODE_DOMAIN, caps)?;
                record.field(1, b"divide")?;
                record.field(2, &numerator_digest)?;
                record.field(3, &denominator_digest)?;
                (
                    sha256(&record.finish()),
                    numerator_depth.max(denominator_depth).saturating_add(1),
                )
            }
            ExactExpressionNode::IntegerPower { base, exponent } => {
                let (base_digest, base_depth) = indexed_digest(&digests, &depths, *base)?
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
                let mut record = RecordBuilder::new(EXPRESSION_NODE_DOMAIN, caps)?;
                record.field(1, b"integer-power")?;
                record.field(2, &base_digest)?;
                record.field(3, &exponent.to_be_bytes())?;
                (sha256(&record.finish()), base_depth.saturating_add(1))
            }
        };
        if depth > caps.expression_depth {
            return Err(PhysicalRegistryRefusalCode::ExpressionDepthExceeded);
        }
        digests[index] = Some(digest);
        depths[index] = Some(depth);
        order.push(index);
        for dependent in &dependents[index] {
            indegrees[*dependent] = indegrees[*dependent]
                .checked_sub(1)
                .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
            if indegrees[*dependent] == 0 {
                ready.insert(*dependent);
            }
        }
    }
    if order.len() != expression.nodes.len() {
        return Err(PhysicalRegistryRefusalCode::ExpressionCycle);
    }
    let concrete = digests
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(PhysicalRegistryRefusalCode::ExpressionCycle)?;
    Ok(ExpressionDigestBundle {
        output_digest: concrete[output],
        digests: concrete,
        edge_count,
        depths,
        order,
    })
}

fn reachable_expression_nodes(
    expression: &ExactExpression,
) -> Result<BTreeSet<usize>, PhysicalRegistryRefusalCode> {
    let output = usize::try_from(expression.output_node)
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    if output >= expression.nodes.len() {
        return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid);
    }
    let mut reachable = BTreeSet::new();
    let mut stack = vec![output];
    while let Some(index) = stack.pop() {
        if !reachable.insert(index) {
            continue;
        }
        let node = expression
            .nodes
            .get(index)
            .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
        match node {
            ExactExpressionNode::Coordinate(_) => {}
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Subtract { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                stack.push(expression_index(*left, expression.nodes.len())?);
                stack.push(expression_index(*right, expression.nodes.len())?);
            }
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                stack.push(expression_index(*numerator, expression.nodes.len())?);
                stack.push(expression_index(*denominator, expression.nodes.len())?);
            }
            ExactExpressionNode::IntegerPower { base, .. } => {
                stack.push(expression_index(*base, expression.nodes.len())?);
            }
        }
    }
    Ok(reachable)
}

fn expression_index(index: u32, length: usize) -> Result<usize, PhysicalRegistryRefusalCode> {
    let index =
        usize::try_from(index).map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    if index >= length {
        return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid);
    }
    Ok(index)
}

fn indexed_digest(
    digests: &[Option<[u8; 32]>],
    depths: &[Option<u32>],
    index: u32,
) -> Result<Option<([u8; 32], u32)>, PhysicalRegistryRefusalCode> {
    let index = expression_index(index, digests.len())?;
    Ok(digests[index].zip(depths[index]))
}

fn parse_rational(
    wire: &ExactRationalWire,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    validate_component(&wire.numerator_be, caps)?;
    validate_component(&wire.denominator_be, caps)?;
    let numerator = decode_component(&wire.numerator_be, meter)?;
    let denominator = decode_component(&wire.denominator_be, meter)?;
    if denominator.is_zero() || (wire.negative && numerator.is_zero()) {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    let divisor = numerator.gcd(&denominator);
    if divisor.cmp_big(&BigUint::from_u64(1)) != Ordering::Equal {
        return Err(PhysicalRegistryRefusalCode::RationalNotReduced);
    }
    let fraction = Fraction {
        negative: wire.negative,
        numerator,
        denominator,
    };
    check_fraction_size(&fraction, caps)?;
    Ok(fraction)
}

fn validate_component(
    bytes: &[u8],
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == 0) {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    let bit_length = component_bit_length(bytes)?;
    if bit_length > caps.rational_component_bits {
        return Err(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded);
    }
    Ok(())
}

fn component_bit_length(bytes: &[u8]) -> Result<u32, PhysicalRegistryRefusalCode> {
    let first = *bytes
        .first()
        .ok_or(PhysicalRegistryRefusalCode::RationalEncodingInvalid)?;
    if first == 0 {
        return Ok(0);
    }
    let tail = u32::try_from(bytes.len() - 1)
        .map_err(|_| PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)?
        .checked_mul(8)
        .ok_or(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)?;
    tail.checked_add(8 - first.leading_zeros())
        .ok_or(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)
}

fn decode_component(
    bytes: &[u8],
    meter: &mut WorkMeter,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::component_decode_work_units(bytes.len())?)?;
    let mut value = BigUint::zero();
    for byte in bytes {
        value = value.shl_bits(8).add(&BigUint::from_u64(u64::from(*byte)));
    }
    Ok(value)
}

fn add_fraction(
    left: &Fraction,
    right: &Fraction,
    subtract_right: bool,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::additive_rational_core_work_units())?;
    let common_denominator = left.denominator.gcd(&right.denominator);
    let left_factor = exact_quotient(&right.denominator, &common_denominator)?;
    let right_factor = exact_quotient(&left.denominator, &common_denominator)?;
    let left_scaled = bounded_multiply(&left.numerator, &left_factor, caps, meter)?;
    let right_scaled = bounded_multiply(&right.numerator, &right_factor, caps, meter)?;
    let denominator = bounded_multiply(&left.denominator, &left_factor, caps, meter)?;
    let right_negative = right.negative ^ subtract_right;
    let (negative, numerator) = match (left.negative, right_negative) {
        (false, false) => (false, left_scaled.add(&right_scaled)),
        (true, true) => (true, left_scaled.add(&right_scaled)),
        _ => match left_scaled.cmp_big(&right_scaled) {
            Ordering::Greater => (left.negative, left_scaled.sub(&right_scaled)),
            Ordering::Less => (right_negative, right_scaled.sub(&left_scaled)),
            Ordering::Equal => (false, BigUint::zero()),
        },
    };
    let value = Fraction {
        negative,
        numerator,
        denominator,
    };
    meter.evaluation(resource::rational_reduction_work_units(
        value.numerator.is_zero(),
    ))?;
    normalize_fraction(value, caps)
}

fn multiply_fraction(
    left: &Fraction,
    right: &Fraction,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::multiplicative_rational_core_work_units())?;
    let left_cross = left.numerator.gcd(&right.denominator);
    let right_cross = right.numerator.gcd(&left.denominator);
    let left_numerator = exact_quotient(&left.numerator, &left_cross)?;
    let right_denominator = exact_quotient(&right.denominator, &left_cross)?;
    let right_numerator = exact_quotient(&right.numerator, &right_cross)?;
    let left_denominator = exact_quotient(&left.denominator, &right_cross)?;
    let value = Fraction {
        negative: left.negative ^ right.negative,
        numerator: bounded_multiply(&left_numerator, &right_numerator, caps, meter)?,
        denominator: bounded_multiply(&left_denominator, &right_denominator, caps, meter)?,
    };
    meter.evaluation(resource::rational_reduction_work_units(
        value.numerator.is_zero(),
    ))?;
    normalize_fraction(value, caps)
}

fn divide_fraction(
    numerator: &Fraction,
    denominator: &Fraction,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    if denominator.numerator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    let reciprocal = Fraction {
        negative: denominator.negative,
        numerator: denominator.denominator.clone(),
        denominator: denominator.numerator.clone(),
    };
    multiply_fraction(numerator, &reciprocal, caps, meter)
}

fn exact_quotient(
    numerator: &BigUint,
    denominator: &BigUint,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    if denominator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    let (quotient, remainder) = numerator.divmod(denominator);
    if !remainder.is_zero() {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    Ok(quotient)
}

fn power_fraction(
    value: &Fraction,
    exponent: i16,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    if exponent == 0 {
        return Ok(Fraction {
            negative: false,
            numerator: BigUint::from_u64(1),
            denominator: BigUint::from_u64(1),
        });
    }
    let power = u32::from(exponent.unsigned_abs());
    if exponent < 0 && value.numerator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    meter.evaluation(producer_power_work_units(power)?)?;
    let mut powered = Fraction {
        negative: value.negative && power % 2 == 1,
        numerator: bounded_power_left_to_right(&value.numerator, power, caps)?,
        denominator: bounded_power_left_to_right(&value.denominator, power, caps)?,
    };
    if exponent < 0 {
        std::mem::swap(&mut powered.numerator, &mut powered.denominator);
    }
    normalize_fraction(powered, caps)
}

fn bounded_power_left_to_right(
    base: &BigUint,
    exponent: u32,
    caps: ValidationCaps,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    let highest_bit = u32::BITS
        .checked_sub(exponent.leading_zeros())
        .and_then(|width| width.checked_sub(1))
        .ok_or(PhysicalRegistryRefusalCode::RationalEncodingInvalid)?;
    let mut result = BigUint::from_u64(1);
    for bit in (0..=highest_bit).rev() {
        result = bounded_multiply_value(&result, &result, caps)?;
        if exponent & (1_u32 << bit) != 0 {
            result = bounded_multiply_value(&result, base, caps)?;
        }
    }
    Ok(result)
}

fn producer_power_work_units(exponent: u32) -> Result<u64, PhysicalRegistryRefusalCode> {
    resource::power_work_units(exponent)
}

#[cfg(test)]
pub(super) fn power_work_units_for_test(exponent: u32) -> Result<u64, PhysicalRegistryRefusalCode> {
    producer_power_work_units(exponent)
}

fn bounded_multiply(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::bounded_product_work_units())?;
    bounded_multiply_value(left, right, caps)
}

fn bounded_multiply_value(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    if !producer_product_fits_component_cap(left, right, caps.intermediate_component_bits) {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    let product = left.mul(right);
    if product.bit_len() > caps.intermediate_component_bits {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    Ok(product)
}

fn producer_product_fits_component_cap(left: &BigUint, right: &BigUint, cap: u32) -> bool {
    resource::product_fits_component_cap(left, right, cap)
}

#[cfg(test)]
pub(super) fn product_fits_component_cap_for_test(
    left: &BigUint,
    right: &BigUint,
    cap: u32,
) -> bool {
    producer_product_fits_component_cap(left, right, cap)
}

fn normalize_fraction(
    mut value: Fraction,
    caps: ValidationCaps,
) -> Result<Fraction, PhysicalRegistryRefusalCode> {
    if value.denominator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    if value.numerator.is_zero() {
        value.negative = false;
        value.denominator = BigUint::from_u64(1);
        return Ok(value);
    }
    let divisor = value.numerator.gcd(&value.denominator);
    let (numerator, numerator_remainder) = value.numerator.divmod(&divisor);
    let (denominator, denominator_remainder) = value.denominator.divmod(&divisor);
    if !numerator_remainder.is_zero() || !denominator_remainder.is_zero() {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    value.numerator = numerator;
    value.denominator = denominator;
    check_fraction_size(&value, caps)?;
    Ok(value)
}

fn check_fraction_size(
    value: &Fraction,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if value.numerator.bit_len() > caps.intermediate_component_bits
        || value.denominator.bit_len() > caps.intermediate_component_bits
    {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    Ok(())
}

fn fraction_to_wire(value: &Fraction) -> Result<ExactRationalWire, PhysicalRegistryRefusalCode> {
    Ok(ExactRationalWire {
        negative: value.negative,
        numerator_be: encode_biguint(&value.numerator)?,
        denominator_be: encode_biguint(&value.denominator)?,
    })
}

fn encode_biguint(value: &BigUint) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    if value.is_zero() {
        return Ok(vec![0]);
    }
    let base = BigUint::from_u64(256);
    let mut cursor = value.clone();
    let mut reversed = Vec::new();
    while !cursor.is_zero() {
        let (quotient, remainder) = cursor.divmod(&base);
        let byte = remainder
            .to_u128()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or(PhysicalRegistryRefusalCode::RationalEncodingInvalid)?;
        reversed.push(byte);
        cursor = quotient;
    }
    reversed.reverse();
    Ok(reversed)
}

fn checked_dimension(
    dimension: DimensionVector,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    if dimension.terms().len()
        > usize::try_from(caps.dimension_term_count)
            .map_err(|_| PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded)?
    {
        return Err(PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded);
    }
    for term in dimension.terms() {
        if i32::from(term.exponent).abs() > caps.dimension_abs_exponent {
            return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
        }
    }
    Ok(dimension)
}

fn combine_dimensions(
    left: DimensionVector,
    right: DimensionVector,
    subtract_right: bool,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    let mut output = BTreeMap::<DimensionAxisIdentity, i32>::new();
    for term in left.terms() {
        output.insert(term.axis, i32::from(term.exponent));
    }
    for term in right.terms() {
        let right_exponent = if subtract_right {
            -i32::from(term.exponent)
        } else {
            i32::from(term.exponent)
        };
        let exponent = output
            .get(&term.axis)
            .copied()
            .unwrap_or(0)
            .checked_add(right_exponent)
            .ok_or(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?;
        if exponent.abs() > caps.dimension_abs_exponent {
            return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
        }
        if exponent == 0 {
            output.remove(&term.axis);
        } else {
            output.insert(term.axis, exponent);
        }
    }
    DimensionVector::from_terms(
        output
            .into_iter()
            .map(|(axis, exponent)| {
                Ok(DimensionTerm {
                    axis,
                    exponent: i16::try_from(exponent)
                        .map_err(|_| PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?,
                })
            })
            .collect::<Result<Vec<_>, PhysicalRegistryRefusalCode>>()?,
    )
}

fn power_dimension(
    dimension: DimensionVector,
    exponent: i16,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    if exponent == 0 {
        return Ok(DimensionVector::dimensionless());
    }
    let mut output = Vec::with_capacity(dimension.terms().len());
    for term in dimension.terms() {
        let value = i32::from(term.exponent)
            .checked_mul(i32::from(exponent))
            .ok_or(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?;
        if value.abs() > caps.dimension_abs_exponent {
            return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
        }
        output.push(DimensionTerm {
            axis: term.axis,
            exponent: i16::try_from(value)
                .map_err(|_| PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?,
        });
    }
    DimensionVector::from_terms(output)
}

fn derive_member_identity(
    member: &MemberBlueprint,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<SpeciesContentIdentity, PhysicalRegistryRefusalCode> {
    meter.evaluation(resource::member_identity_work_units())?;
    Ok(SpeciesContentIdentity(sha256(&encode_member_blueprint(
        member, caps,
    )?)))
}

fn encode_artifact_payload(
    payload: &ArtifactPayload,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let identity_domain = match payload {
        ArtifactPayload::ExactMasslessLaw(_) => EXACT_MASSLESS_ARTIFACT_DOMAIN_V4,
        _ => ARTIFACT_DOMAIN_V3,
    };
    let mut output = RecordBuilder::new(identity_domain, caps)?;
    match payload {
        ArtifactPayload::ScalarCoordinate(coordinate) => {
            validate_content(&coordinate.coordinate, caps)?;
            parse_rational(&coordinate.exact_value, caps, meter)?;
            checked_dimension(coordinate.dimension, caps)?;
            output.field(1, b"scalar-coordinate")?;
            output.field(2, &encode_content(&coordinate.coordinate, caps)?)?;
            output.field(3, &encode_rational(&coordinate.exact_value, caps)?)?;
            output.field(4, &encode_dimension(coordinate.dimension))?;
        }
        ArtifactPayload::PhysicalDescriptor(content) => {
            validate_content(content, caps)?;
            output.field(1, b"physical-descriptor")?;
            output.field(2, &encode_content(content, caps)?)?;
        }
        ArtifactPayload::ConstraintLaw(law) => {
            output.field(1, b"constraint-law")?;
            output.field(2, &encode_requirements(&law.requirements, caps)?)?;
        }
        ArtifactPayload::MassProjection(projection) => {
            output.field(1, b"mass-projection")?;
            output.field(2, &encode_expression(&projection.expression, caps, meter)?)?;
            output.field(3, projection.scope.id().as_bytes())?;
            if let Some(proof) = &projection.uncertainty_transport {
                output.field(4, &proof.source_coordinate.0)?;
                output.field(5, &encode_receipt(&proof.source_pair_receipt, caps)?)?;
                output.field(6, &encode_receipt(&proof.producer_receipt, caps)?)?;
                output.field(7, &encode_receipt(&proof.watchdog_receipt, caps)?)?;
            }
        }
        ArtifactPayload::ExactMasslessLaw(law) => {
            output.field(1, b"exact-massless-law")?;
            output.field(2, &encode_requirements(&law.requirements, caps)?)?;
            output.field(3, &law.proof.subject.0)?;
            output.field(4, &encode_content(&law.proof.excluded_term, caps)?)?;
            output.field(5, &law.proof.symmetry.0)?;
            output.field(6, &encode_receipt(&law.proof.applicability_receipt, caps)?)?;
            output.field(
                7,
                &encode_receipt(&law.proof.exclusion_producer_receipt, caps)?,
            )?;
            output.field(
                8,
                &encode_receipt(&law.proof.exclusion_watchdog_receipt, caps)?,
            )?;
        }
        ArtifactPayload::SpeciesDerivation(rule) => {
            output.field(1, b"species-derivation")?;
            output.field(2, &rule.derivation_kind.0)?;
            output.field(3, &encode_artifact_relations(&rule.artifact_inputs, caps)?)?;
            output.field(4, &encode_species_id_list(&rule.constituents, caps)?)?;
            output.field(5, &encode_member_blueprint(&rule.output, caps)?)?;
        }
    }
    Ok(output.finish())
}

fn encode_expression(
    expression: &ExactExpression,
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let ExpressionDigestBundle {
        mut digests,
        output_digest,
        edge_count,
        ..
    } = expression_digests(expression, caps, meter)?;
    digests.sort_unstable();
    let mut output = RecordBuilder::new(EXPRESSION_DOMAIN, caps)?;
    output.field(1, &output_digest)?;
    output.field(
        2,
        &u32::try_from(digests.len())
            .map_err(|_| PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded)?
            .to_be_bytes(),
    )?;
    output.field(3, &edge_count.to_be_bytes())?;
    for digest in digests {
        output.field(4, &digest)?;
    }
    Ok(output.finish())
}

fn encode_content(
    content: &CanonicalArtifact,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    validate_content(content, caps)?;
    let mut output = RecordBuilder::new(b"civsim.physical-species.content.v1", caps)?;
    output.field(1, content.schema_id.as_bytes())?;
    output.field(2, &content.canonical_bytes)?;
    Ok(output.finish())
}

fn encode_receipt(
    receipt: &ReceiptBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    validate_receipt(receipt, caps)?;
    let mut output = RecordBuilder::new(b"civsim.physical-species.receipt.v1", caps)?;
    output.field(1, receipt.schema_id.as_bytes())?;
    output.field(2, &receipt.digest_sha256)?;
    Ok(output.finish())
}

fn encode_admission(
    admission: &RootAdmission,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(b"civsim.physical-species.admission.v1", caps)?;
    output.field(1, admission.tier.id().as_bytes())?;
    output.field(
        2,
        admission
            .provenance
            .bracket_tag()
            .ok_or(PhysicalRegistryRefusalCode::NoncanonicalProvenance)?
            .as_bytes(),
    )?;
    match &admission.route {
        AdmissionRoute::Derived(route) => {
            output.field(3, b"derived")?;
            output.field(4, &encode_receipt(&route.ancestry_receipt, caps)?)?;
            output.field(5, &encode_receipt(&route.semantic_checker_receipt, caps)?)?;
            output.field(
                6,
                &encode_receipt(&route.independent_watchdog_receipt, caps)?,
            )?;
        }
        AdmissionRoute::Irreducible(route) => {
            output.field(3, b"irreducible")?;
            output.field(
                4,
                &encode_receipt(&route.derivation_exhaustion_receipt, caps)?,
            )?;
            output.field(5, &encode_receipt(&route.buckingham_pi_receipt, caps)?)?;
            output.field(6, &encode_receipt(&route.gap_law_receipt, caps)?)?;
            output.field(7, &encode_receipt(&route.chaos_protocol_receipt, caps)?)?;
            output.field(8, &encode_receipt(&route.residual_law_receipt, caps)?)?;
            output.field(9, route.residual_slot_id.as_bytes())?;
            output.field(10, &encode_receipt(&route.residual_slot_receipt, caps)?)?;
            output.field(11, &encode_receipt(&route.owner_admission_receipt, caps)?)?;
            output.field(
                12,
                &encode_receipt(&route.independent_watchdog_receipt, caps)?,
            )?;
        }
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            output.field(3, b"evidence-custody-only")?;
            output.field(4, &encode_receipt(source_receipt, caps)?)?;
        }
    }
    Ok(output.finish())
}

fn encode_member_blueprint(
    member: &MemberBlueprint,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(MEMBER_DOMAIN, caps)?;
    output.field(1, &encode_content(&member.physical_content, caps)?)?;
    output.field(2, &encode_requirements(&member.requirements, caps)?)?;
    match member.mass_proof {
        MassProofReference::Projection(identity) => {
            output.field(3, b"projection")?;
            output.field(4, &identity.0)?;
        }
        MassProofReference::ExactMassless(identity) => {
            output.field(3, b"exact-massless")?;
            output.field(4, &identity.0)?;
        }
    }
    output.field(
        5,
        &encode_artifact_relations(&member.constraint_laws, caps)?,
    )?;
    Ok(output.finish())
}

fn encode_requirements(
    requirements: &RequirementSet,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(b"civsim.physical-species.requirements.v1", caps)?;
    output.field(
        1,
        &encode_artifact_relations(&requirements.artifact_relations, caps)?,
    )?;
    output.field(
        2,
        &encode_species_id_list(&requirements.species_dependencies, caps)?,
    )?;
    Ok(output.finish())
}

fn encode_artifact_relations(
    relations: &[ArtifactRelation],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let relations = sorted_unique_relations(relations)?;
    let mut output = RecordBuilder::new(b"civsim.physical-species.relations.v1", caps)?;
    for relation in relations {
        let mut edge = RecordBuilder::new(b"civsim.physical-species.relation.v1", caps)?;
        edge.field(1, &relation.role.0)?;
        edge.field(2, &relation.target.0)?;
        output.field(1, &edge.finish())?;
    }
    Ok(output.finish())
}

fn encode_species_id_list(
    values: &[SpeciesContentIdentity],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut values = values.to_vec();
    values.sort_unstable();
    let mut output = RecordBuilder::new(b"civsim.physical-species.member-list.v1", caps)?;
    for value in values {
        output.field(1, &value.0)?;
    }
    Ok(output.finish())
}

fn encode_rational(
    value: &ExactRationalWire,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(b"civsim.physical-species.rational.v1", caps)?;
    output.field(1, &[u8::from(value.negative)])?;
    output.field(2, &value.numerator_be)?;
    output.field(3, &value.denominator_be)?;
    Ok(output.finish())
}

fn encode_dimension(dimension: DimensionVector) -> Vec<u8> {
    let mut output = Vec::with_capacity(4 + dimension.terms().len() * 34);
    output.extend_from_slice(
        &u32::try_from(dimension.terms().len())
            .expect("dimension term storage fits u32")
            .to_be_bytes(),
    );
    for term in dimension.terms() {
        output.extend_from_slice(&term.axis.0);
        output.extend_from_slice(&term.exponent.to_be_bytes());
    }
    output
}

fn encode_resources(
    resources: PhysicalRegistryResourceContract,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(b"civsim.physical-species.resources.v1", caps)?;
    for (tag, bytes) in [
        (1, resources.max_artifact_count.to_be_bytes().to_vec()),
        (
            2,
            resources.max_registry_member_count.to_be_bytes().to_vec(),
        ),
        (
            3,
            resources.max_references_per_artifact.to_be_bytes().to_vec(),
        ),
        (
            4,
            resources.max_total_reference_count.to_be_bytes().to_vec(),
        ),
        (
            5,
            resources.max_expression_node_count.to_be_bytes().to_vec(),
        ),
        (
            6,
            resources.max_expression_edge_count.to_be_bytes().to_vec(),
        ),
        (7, resources.max_expression_depth.to_be_bytes().to_vec()),
        (
            8,
            resources.max_rational_component_bits.to_be_bytes().to_vec(),
        ),
        (
            9,
            resources
                .max_intermediate_component_bits
                .to_be_bytes()
                .to_vec(),
        ),
        (
            10,
            resources.max_dimension_abs_exponent.to_be_bytes().to_vec(),
        ),
        (
            11,
            resources.max_dimension_term_count.to_be_bytes().to_vec(),
        ),
        (12, resources.max_evaluation_steps.to_be_bytes().to_vec()),
        (13, resources.max_closure_steps.to_be_bytes().to_vec()),
        (14, resources.max_canonical_bytes.to_be_bytes().to_vec()),
        (
            15,
            resources.max_canonical_token_bytes.to_be_bytes().to_vec(),
        ),
        (16, resources.max_content_bytes.to_be_bytes().to_vec()),
    ] {
        output.field(tag, &bytes)?;
    }
    Ok(output.finish())
}

fn encode_vocabulary_binding(
    binding: &PhysicalVocabularyBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(b"civsim.physical-species.vocabulary-binding.v1", caps)?;
    for (tag, value) in [
        (1, binding.schema_id.as_bytes()),
        (2, binding.claim_id.as_bytes()),
        (3, binding.producer_id.as_bytes()),
        (4, binding.watchdog_id.as_bytes()),
    ] {
        output.field(tag, value)?;
    }
    output.field(5, &binding.root_count.to_be_bytes())?;
    for identity in &binding.descriptor_role_identities {
        output.field(6, &identity.0)?;
    }
    for identity in &binding.relation_target_identities {
        output.field(7, &identity.0)?;
    }
    for identity in &binding.constraint_law_identities {
        output.field(8, &identity.0)?;
    }
    output.field(9, &[u8::from(binding.current_input_partition_complete)])?;
    output.field(10, &[u8::from(binding.global_physical_vocabulary_coverage)])?;
    output.field(11, &[u8::from(binding.membership_authority)])?;
    output.field(12, &binding.producer_result_sha256)?;
    output.field(13, &binding.watchdog_result_sha256)?;
    output.field(14, &binding.producer_resource_contract_sha256)?;
    output.field(15, &binding.watchdog_resource_contract_sha256)?;
    output.field(16, &binding.receipt_sha256)?;
    Ok(output.finish())
}

fn encode_registry(
    input: &PhysicalRegistryInput,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    members: &[VerifiedPhysicalMember],
    caps: ValidationCaps,
    meter: &mut WorkMeter,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut output = RecordBuilder::new(REGISTRY_DOMAIN, caps)?;
    output.field(1, input.schema_id.as_bytes())?;
    output.field(2, input.proof_graph_schema_id.as_bytes())?;
    output.field(3, &encode_receipt(&input.floor_binding, caps)?)?;
    for (tag, schema) in [
        (4, input.structure_binding.structure_schema_id.as_bytes()),
        (
            5,
            input
                .structure_binding
                .species_registry_schema_id
                .as_bytes(),
        ),
        (
            6,
            input.structure_binding.stellar_state_schema_id.as_bytes(),
        ),
        (
            7,
            input
                .structure_binding
                .state_coordinate_registry_schema_id
                .as_bytes(),
        ),
        (
            8,
            input
                .structure_binding
                .interaction_sector_registry_schema_id
                .as_bytes(),
        ),
        (
            9,
            input
                .structure_binding
                .physical_regime_registry_schema_id
                .as_bytes(),
        ),
    ] {
        output.field(tag, schema)?;
    }
    output.field(10, input.checker_pair.producer_id.as_bytes())?;
    output.field(11, input.checker_pair.watchdog_id.as_bytes())?;
    output.field(12, &encode_resources(input.resources, caps)?)?;
    output.field(
        13,
        &encode_vocabulary_binding(&input.vocabulary_binding, caps)?,
    )?;
    for artifact in artifacts.values() {
        meter.evaluation(resource::registry_artifact_work_units())?;
        let mut record = RecordBuilder::new(b"civsim.physical-species.admitted-artifact.v1", caps)?;
        record.field(1, &artifact.claimed_identity.0)?;
        record.field(2, &encode_admission(&artifact.admission, caps)?)?;
        record.field(3, &encode_artifact_payload(&artifact.payload, caps, meter)?)?;
        output.field(14, &record.finish())?;
    }
    for member in members {
        let mut record = RecordBuilder::new(b"civsim.physical-species.verified-member.v1", caps)?;
        record.field(1, &member.identity.0)?;
        record.field(2, &encode_content(&member.physical_content, caps)?)?;
        record.field(3, &encode_rational(&member.rest_mass_si, caps)?)?;
        record.field(4, &encode_dimension(member.mass_dimension))?;
        record.field(5, &member.derivation_kind.0)?;
        record.field(6, &encode_requirements(&member.requirements, caps)?)?;
        output.field(15, &record.finish())?;
    }
    Ok(output.finish())
}
