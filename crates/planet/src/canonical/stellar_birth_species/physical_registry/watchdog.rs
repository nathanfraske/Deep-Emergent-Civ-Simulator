//! Independent top-down watchdog for the physical species registry.
//!
//! The producer closes derivations from their roots. This checker first traces
//! every declared member back through its prerequisites, detects cycles with
//! bounded explicit stacks, and then performs a separate forward worklist
//! closure. Exact expressions are evaluated from an explicit postorder with
//! independently decoded rationals. The shared surface is limited to the model
//! and SHA-256 primitive.

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
struct Rational {
    negative: bool,
    numerator: BigUint,
    denominator: BigUint,
}

#[derive(Debug, Clone)]
struct Scalar {
    value: Rational,
    dimension: DimensionVector,
}

#[derive(Debug, Clone)]
struct TracedRule {
    member: VerifiedPhysicalMember,
    prerequisites: Vec<SpeciesContentIdentity>,
}

#[derive(Debug, Clone, Copy)]
struct Budget {
    evaluation_used: u64,
    closure_used: u64,
    evaluation_limit: u64,
    closure_limit: u64,
}

impl Budget {
    const fn new(caps: ValidationCaps) -> Self {
        Self {
            evaluation_used: 0,
            closure_used: 0,
            evaluation_limit: caps.evaluation_steps,
            closure_limit: caps.closure_steps,
        }
    }

    fn evaluation(&mut self, amount: u64) -> Result<(), PhysicalRegistryRefusalCode> {
        self.evaluation_used = self
            .evaluation_used
            .checked_add(amount)
            .ok_or(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded)?;
        if self.evaluation_used > self.evaluation_limit {
            return Err(PhysicalRegistryRefusalCode::EvaluationStepLimitExceeded);
        }
        Ok(())
    }

    fn closure(&mut self, amount: u64) -> Result<(), PhysicalRegistryRefusalCode> {
        self.closure_used = self
            .closure_used
            .checked_add(amount)
            .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)?;
        if self.closure_used > self.closure_limit {
            return Err(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded);
        }
        Ok(())
    }
}

struct WireRecord {
    bytes: Vec<u8>,
    ceiling: usize,
}

impl WireRecord {
    fn start(domain: &[u8], caps: ValidationCaps) -> Result<Self, PhysicalRegistryRefusalCode> {
        let ceiling = usize::try_from(caps.canonical_bytes)
            .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        if domain.len() > ceiling {
            return Err(PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded);
        }
        Ok(Self {
            bytes: domain.to_vec(),
            ceiling,
        })
    }

    fn push(&mut self, tag: u16, payload: &[u8]) -> Result<(), PhysicalRegistryRefusalCode> {
        let payload_length = u64::try_from(payload.len())
            .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        let next = self
            .bytes
            .len()
            .checked_add(2)
            .and_then(|length| length.checked_add(8))
            .and_then(|length| length.checked_add(payload.len()))
            .ok_or(PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        if next > self.ceiling {
            return Err(PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded);
        }
        self.bytes.extend_from_slice(&tag.to_be_bytes());
        self.bytes.extend_from_slice(&payload_length.to_be_bytes());
        self.bytes.extend_from_slice(payload);
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

pub(super) fn derive_artifact_identity_for_authority(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, PhysicalRegistryRefusalCode> {
    let mut budget = Budget::new(ValidationCaps::PRODUCTION);
    recompute_artifact_identity(payload, ValidationCaps::PRODUCTION, &mut budget)
}

pub(super) fn derive_member_identity_for_authority(
    member: &MemberBlueprint,
) -> Result<SpeciesContentIdentity, PhysicalRegistryRefusalCode> {
    let mut budget = Budget::new(ValidationCaps::PRODUCTION);
    budget.evaluation(resource::member_identity_work_units())?;
    Ok(SpeciesContentIdentity(sha256(&write_member_blueprint(
        member,
        ValidationCaps::PRODUCTION,
    )?)))
}

pub(super) fn validate_and_encode_with_caps(
    input: &PhysicalRegistryInput,
    caps: ValidationCaps,
) -> Result<ValidatedRegistry, PhysicalRegistryRefusalCode> {
    validate_profile(input)?;
    validate_outer_capacities(input, caps)?;

    let mut budget = Budget::new(caps);
    let artifacts = validate_artifact_inventory(input, caps, &mut budget)?;
    if artifacts.is_empty() {
        return Err(PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots);
    }
    let production_vocabulary_binding = input.admitted_artifacts.iter().any(|artifact| {
        artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
    });
    if production_vocabulary_binding
        && !super::vocabulary::watchdog_accepts_binding(
            &input.admitted_artifacts,
            &input.vocabulary_binding,
        )
    {
        return Err(PhysicalRegistryRefusalCode::PhysicalVocabularyBindingMismatch);
    }
    let rules = derive_rule_inventory(&artifacts, caps, &mut budget)?;
    if rules.is_empty() {
        return Err(PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRules);
    }
    if production_vocabulary_binding
        && !input.vocabulary_binding.global_physical_vocabulary_coverage
    {
        return Err(PhysicalRegistryRefusalCode::PhysicalVocabularyCoverageIncomplete);
    }
    let declared = normalize_declared_members(&input.declared_members, caps)?;
    let dependency_count = rules.values().try_fold(0_usize, |total, rule| {
        total
            .checked_add(rule.prerequisites.len())
            .ok_or(PhysicalRegistryRefusalCode::ClosureStepLimitExceeded)
    })?;
    budget.closure(resource::closure_work_units(
        rules.len(),
        dependency_count,
        declared.len(),
    )?)?;
    detect_derivation_cycles(&rules)?;
    reverse_trace_declared_members(&declared, &rules)?;
    let members = forward_worklist_closure(&rules)?;
    if members.is_empty() {
        return Err(PhysicalRegistryRefusalCode::EmptyRegistryIsNotClosure);
    }
    if members.len() != rules.len() {
        return Err(PhysicalRegistryRefusalCode::MissingClosureMember);
    }
    compare_closure(&members, &declared)?;

    let canonical_bytes = write_registry(input, &artifacts, &members, caps, &mut budget)?;
    Ok(ValidatedRegistry {
        members,
        canonical_bytes,
        resource_contract_sha256: resource_contract_sha256(),
    })
}

fn validate_profile(input: &PhysicalRegistryInput) -> Result<(), PhysicalRegistryRefusalCode> {
    if input.resources != resource::PRODUCTION_RESOURCE_CONTRACT {
        return Err(PhysicalRegistryRefusalCode::ResourceContractMismatch);
    }

    let structure = stellar_birth_structure_schema()
        .map_err(|_| PhysicalRegistryRefusalCode::StructureBindingMismatch)?;
    let observed_structure = (
        input
            .structure_binding
            .physical_regime_registry_schema_id
            .as_str(),
        input
            .structure_binding
            .interaction_sector_registry_schema_id
            .as_str(),
        input
            .structure_binding
            .state_coordinate_registry_schema_id
            .as_str(),
        input.structure_binding.stellar_state_schema_id.as_str(),
        input.structure_binding.species_registry_schema_id.as_str(),
        input.structure_binding.structure_schema_id.as_str(),
    );
    let required_structure = (
        structure.stellar_state.physical_regime_registry.schema_id,
        structure
            .stellar_state
            .interaction_sector_registry
            .schema_id,
        structure.stellar_state.state_coordinate_registry.schema_id,
        structure.stellar_state.schema_id,
        structure.species_registry.schema_id,
        structure.schema_id,
    );
    if observed_structure != required_structure {
        return Err(PhysicalRegistryRefusalCode::StructureBindingMismatch);
    }

    let floor = sealed_physical_floor_authority_binding()
        .map_err(|_| PhysicalRegistryRefusalCode::FloorBindingMismatch)?;
    if input.floor_binding.digest_sha256 == [0; 32]
        || input.floor_binding.digest_sha256 != floor.digest()
        || input.floor_binding.schema_id != floor.schema_id().as_str()
    {
        return Err(PhysicalRegistryRefusalCode::FloorBindingMismatch);
    }

    if (
        input.proof_graph_schema_id.as_str(),
        input.schema_id.as_str(),
    ) != (PROOF_GRAPH_SCHEMA_ID, REGISTRY_SCHEMA_ID)
    {
        return Err(PhysicalRegistryRefusalCode::SchemaMismatch);
    }
    if (
        input.checker_pair.watchdog_id.as_str(),
        input.checker_pair.producer_id.as_str(),
    ) != (WATCHDOG_ID, PRODUCER_ID)
    {
        return Err(PhysicalRegistryRefusalCode::CheckerPairMismatch);
    }
    Ok(())
}

fn validate_outer_capacities(
    input: &PhysicalRegistryInput,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if u32::try_from(input.declared_members.len())
        .map_or(true, |count| count > caps.registry_member_count)
    {
        return Err(PhysicalRegistryRefusalCode::RegistryCapacityExceeded);
    }
    if u32::try_from(input.admitted_artifacts.len())
        .map_or(true, |count| count > caps.artifact_count)
    {
        return Err(PhysicalRegistryRefusalCode::ArtifactCapacityExceeded);
    }
    Ok(())
}

fn validate_artifact_inventory<'a>(
    input: &'a PhysicalRegistryInput,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<BTreeMap<ArtifactIdentity, &'a AdmittedArtifact>, PhysicalRegistryRefusalCode> {
    let mut sorted = input.admitted_artifacts.iter().collect::<Vec<_>>();
    sorted.sort_by_key(|artifact| artifact.claimed_identity);
    for pair in sorted.windows(2) {
        if pair[0].claimed_identity == pair[1].claimed_identity {
            return Err(if pair[0].payload == pair[1].payload {
                PhysicalRegistryRefusalCode::DuplicateArtifactIdentity
            } else {
                PhysicalRegistryRefusalCode::ArtifactIdentityCollision
            });
        }
    }

    let mut residual_slots = BTreeSet::new();
    let mut total_references = 0_u32;
    let mut inventory = BTreeMap::new();
    for artifact in sorted {
        validate_root_admission(&artifact.admission, caps, &mut residual_slots)?;
        let references = count_artifact_references(&artifact.payload)?;
        if references > caps.references_per_artifact {
            return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
        }
        total_references = total_references
            .checked_add(references)
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?;
        if total_references > caps.total_reference_count {
            return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
        }

        let identity = recompute_artifact_identity(&artifact.payload, caps, budget)?;
        if identity != artifact.claimed_identity {
            return Err(PhysicalRegistryRefusalCode::ArtifactIdentityMismatch);
        }
        verify_admission_capability(artifact)?;
        inventory.insert(artifact.claimed_identity, artifact);
    }
    Ok(inventory)
}

fn verify_admission_capability(
    artifact: &AdmittedArtifact,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let bound = artifact.capability_admission();
    if artifact
        .capability_claimed_identity()
        .0
        .iter()
        .zip(artifact.claimed_identity.0)
        .any(|(left, right)| *left != right)
        || bound.tier != artifact.admission.tier
        || bound.provenance != artifact.admission.provenance
        || !same_admission_route(&bound.route, &artifact.admission.route)
    {
        return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
    }
    match artifact.admission_capability_kind() {
        AdmissionCapabilityKind::RepositoryRoot => {
            let Some(receipt_sha256) = artifact.repository_root_pair_receipt_sha256() else {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            };
            if receipt_sha256.iter().all(|byte| *byte == 0) {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
        #[cfg(test)]
        AdmissionCapabilityKind::ExactTest => {
            if artifact.repository_root_pair_receipt_sha256().is_some() {
                return Err(PhysicalRegistryRefusalCode::AdmissionCapabilityMismatch);
            }
        }
    }
    Ok(())
}

fn same_admission_route(left: &AdmissionRoute, right: &AdmissionRoute) -> bool {
    match (left, right) {
        (AdmissionRoute::Derived(left), AdmissionRoute::Derived(right)) => {
            same_receipt(
                &left.independent_watchdog_receipt,
                &right.independent_watchdog_receipt,
            ) && same_receipt(
                &left.semantic_checker_receipt,
                &right.semantic_checker_receipt,
            ) && same_receipt(&left.ancestry_receipt, &right.ancestry_receipt)
        }
        (AdmissionRoute::Irreducible(left), AdmissionRoute::Irreducible(right)) => {
            left.residual_slot_id.as_bytes() == right.residual_slot_id.as_bytes()
                && same_receipt(
                    &left.independent_watchdog_receipt,
                    &right.independent_watchdog_receipt,
                )
                && same_receipt(
                    &left.owner_admission_receipt,
                    &right.owner_admission_receipt,
                )
                && same_receipt(&left.residual_slot_receipt, &right.residual_slot_receipt)
                && same_receipt(&left.residual_law_receipt, &right.residual_law_receipt)
                && same_receipt(&left.chaos_protocol_receipt, &right.chaos_protocol_receipt)
                && same_receipt(&left.gap_law_receipt, &right.gap_law_receipt)
                && same_receipt(&left.buckingham_pi_receipt, &right.buckingham_pi_receipt)
                && same_receipt(
                    &left.derivation_exhaustion_receipt,
                    &right.derivation_exhaustion_receipt,
                )
        }
        (
            AdmissionRoute::EvidenceCustodyOnly {
                source_receipt: left,
            },
            AdmissionRoute::EvidenceCustodyOnly {
                source_receipt: right,
            },
        ) => same_receipt(left, right),
        _ => false,
    }
}

fn same_receipt(left: &ReceiptBinding, right: &ReceiptBinding) -> bool {
    left.schema_id.as_bytes() == right.schema_id.as_bytes()
        && left
            .digest_sha256
            .iter()
            .zip(right.digest_sha256)
            .all(|(left, right)| left == &right)
}

fn validate_root_admission(
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
        AdmissionRoute::Derived(derived) => {
            if admission.provenance != ProvenanceMark::Derived {
                return Err(PhysicalRegistryRefusalCode::DerivedAdmissionProvenanceMismatch);
            }
            validate_receipt_roles(
                &[
                    &derived.independent_watchdog_receipt,
                    &derived.semantic_checker_receipt,
                    &derived.ancestry_receipt,
                ],
                caps,
            )
        }
        AdmissionRoute::Irreducible(irreducible) => {
            if admission.provenance == ProvenanceMark::Derived {
                return Err(PhysicalRegistryRefusalCode::IrreducibleAdmissionProvenanceInvalid);
            }
            if !canonical_token(&irreducible.residual_slot_id, caps) {
                return Err(PhysicalRegistryRefusalCode::CanonicalTextInvalid);
            }
            if !residual_slots.insert(irreducible.residual_slot_id.clone()) {
                return Err(PhysicalRegistryRefusalCode::DuplicateResidualSlot);
            }
            validate_receipt_roles(
                &[
                    &irreducible.independent_watchdog_receipt,
                    &irreducible.owner_admission_receipt,
                    &irreducible.residual_slot_receipt,
                    &irreducible.residual_law_receipt,
                    &irreducible.chaos_protocol_receipt,
                    &irreducible.gap_law_receipt,
                    &irreducible.buckingham_pi_receipt,
                    &irreducible.derivation_exhaustion_receipt,
                ],
                caps,
            )
        }
    }
}

fn validate_receipt_roles(
    receipts: &[&ReceiptBinding],
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    for receipt in receipts {
        validate_receipt(receipt, caps)?;
    }
    for right in 1..receipts.len() {
        for left in 0..right {
            if receipts[left] == receipts[right] {
                return Err(PhysicalRegistryRefusalCode::DuplicateAdmissionReceipt);
            }
        }
    }
    Ok(())
}

fn validate_receipt(
    receipt: &ReceiptBinding,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if receipt.digest_sha256.iter().all(|byte| *byte == 0) {
        return Err(PhysicalRegistryRefusalCode::MissingBindingDigest);
    }
    if !canonical_token(&receipt.schema_id, caps) {
        return Err(PhysicalRegistryRefusalCode::CanonicalTextInvalid);
    }
    Ok(())
}

fn canonical_token(value: &str, caps: ValidationCaps) -> bool {
    let Ok(length) = u32::try_from(value.len()) else {
        return false;
    };
    length > 0
        && length <= caps.canonical_token_bytes
        && value.as_bytes().iter().copied().all(|byte| {
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
    let length = u32::try_from(content.canonical_bytes.len())
        .map_err(|_| PhysicalRegistryRefusalCode::ContentByteLimitExceeded)?;
    if length == 0 || length > caps.content_bytes {
        return Err(PhysicalRegistryRefusalCode::ContentByteLimitExceeded);
    }
    Ok(())
}

fn count_artifact_references(
    payload: &ArtifactPayload,
) -> Result<u32, PhysicalRegistryRefusalCode> {
    let count = match payload {
        ArtifactPayload::ScalarCoordinate(_) | ArtifactPayload::PhysicalDescriptor(_) => 0,
        ArtifactPayload::ConstraintLaw(law) => count_requirements(&law.requirements)?,
        ArtifactPayload::MassProjection(projection) => {
            let mut total = 0_u32;
            for node in &projection.expression.nodes {
                total = total
                    .checked_add(match node {
                        ExactExpressionNode::Coordinate(_) => 1,
                        ExactExpressionNode::IntegerPower { .. } => 1,
                        _ => 2,
                    })
                    .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?;
            }
            total
        }
        ArtifactPayload::ExactMasslessLaw(law) => count_requirements(&law.requirements)?
            .checked_add(2)
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?,
        ArtifactPayload::SpeciesDerivation(rule) => checked_lengths(&[
            1,
            usize::try_from(count_relation_references(&rule.artifact_inputs)?)
                .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?,
            rule.constituents.len(),
            usize::try_from(count_member_references(&rule.output)?)
                .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?,
        ])?,
    };
    Ok(count)
}

fn count_requirements(requirements: &RequirementSet) -> Result<u32, PhysicalRegistryRefusalCode> {
    checked_lengths(&[
        usize::try_from(count_relation_references(&requirements.artifact_relations)?)
            .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?,
        requirements.species_dependencies.len(),
    ])
}

fn count_member_references(member: &MemberBlueprint) -> Result<u32, PhysicalRegistryRefusalCode> {
    let constraint_references = count_relation_references(&member.constraint_laws)?;
    count_requirements(&member.requirements)?
        .checked_add(1)
        .and_then(|count| count.checked_add(constraint_references))
        .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
}

fn count_relation_references(
    relations: &[ArtifactRelation],
) -> Result<u32, PhysicalRegistryRefusalCode> {
    u32::try_from(relations.len())
        .ok()
        .and_then(|count| count.checked_mul(2))
        .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
}

fn checked_lengths(lengths: &[usize]) -> Result<u32, PhysicalRegistryRefusalCode> {
    lengths.iter().try_fold(0_u32, |total, length| {
        let length = u32::try_from(*length)
            .map_err(|_| PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)?;
        total
            .checked_add(length)
            .ok_or(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded)
    })
}

fn recompute_artifact_identity(
    payload: &ArtifactPayload,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<ArtifactIdentity, PhysicalRegistryRefusalCode> {
    budget.evaluation(resource::artifact_identity_work_units())?;
    Ok(ArtifactIdentity(sha256(&write_artifact_payload(
        payload, caps, budget,
    )?)))
}

fn derive_rule_inventory(
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<BTreeMap<SpeciesContentIdentity, TracedRule>, PhysicalRegistryRefusalCode> {
    let mut rules = BTreeMap::new();
    for artifact in artifacts.values().rev() {
        let ArtifactPayload::SpeciesDerivation(rule) = &artifact.payload else {
            continue;
        };
        validate_descriptor(rule.derivation_kind, artifacts)?;
        let inputs = validate_relations(&rule.artifact_inputs, artifacts, caps, |_| true)?;
        if inputs.is_empty() {
            return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
        }
        let prerequisites = normalize_species_ids(&rule.constituents, caps)?;
        let member =
            validate_member_blueprint(&rule.output, rule.derivation_kind, artifacts, caps, budget)?;
        if member.requirements.species_dependencies != prerequisites {
            return Err(PhysicalRegistryRefusalCode::DependencyMismatch);
        }
        if rules
            .insert(
                member.identity,
                TracedRule {
                    member,
                    prerequisites,
                },
            )
            .is_some()
        {
            return Err(PhysicalRegistryRefusalCode::DuplicateMemberDerivation);
        }
    }
    for rule in rules.values() {
        if rule
            .prerequisites
            .iter()
            .any(|identity| !rules.contains_key(identity))
        {
            return Err(PhysicalRegistryRefusalCode::UnknownSpeciesDependency);
        }
    }
    Ok(rules)
}

fn validate_member_blueprint(
    blueprint: &MemberBlueprint,
    derivation_kind: ArtifactIdentity,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<VerifiedPhysicalMember, PhysicalRegistryRefusalCode> {
    validate_content(&blueprint.physical_content, caps)?;
    validate_descriptor(derivation_kind, artifacts)?;
    let requirements = normalize_requirements(&blueprint.requirements, caps)?;
    validate_relations(&requirements.artifact_relations, artifacts, caps, |_| true)?;

    let constraints = validate_relations(&blueprint.constraint_laws, artifacts, caps, |payload| {
        matches!(payload, ArtifactPayload::ConstraintLaw(_))
    })?;
    if constraints.is_empty() {
        return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
    }
    for relation in constraints.iter().rev() {
        let law = artifacts
            .get(&relation.target)
            .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
        let ArtifactPayload::ConstraintLaw(law) = &law.payload else {
            return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
        };
        if normalize_requirements(&law.requirements, caps)? != requirements {
            return Err(PhysicalRegistryRefusalCode::DependencyMismatch);
        }
    }

    let (rest_mass_si, mass_dimension) = match blueprint.mass_proof {
        MassProofReference::Projection(identity) => {
            let artifact = artifacts
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
            let ArtifactPayload::MassProjection(projection) = &artifact.payload else {
                return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
            };
            if projection.scope != MassProjectionScope::SpeciesRestMass {
                return Err(PhysicalRegistryRefusalCode::MassProjectionNotAuthorizedForMember);
            }
            let mass =
                evaluate_expression_postorder(&projection.expression, artifacts, caps, budget)?;
            if mass.dimension != MASS_DIMENSION {
                return Err(PhysicalRegistryRefusalCode::MassDimensionMismatch);
            }
            if mass.value.negative || mass.value.numerator.is_zero() {
                return Err(PhysicalRegistryRefusalCode::NonPositiveMass);
            }
            (rational_to_wire(&mass.value)?, mass.dimension)
        }
        MassProofReference::ExactMassless(identity) => {
            let artifact = artifacts
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
            let ArtifactPayload::ExactMasslessLaw(law) = &artifact.payload else {
                return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
            };
            if normalize_requirements(&law.requirements, caps)? != requirements {
                return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
            }
            inspect_exact_zero_mass_proof(&law.proof, &requirements, artifacts, caps)?;
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

    budget.evaluation(resource::member_identity_work_units())?;
    let identity = SpeciesContentIdentity(sha256(&write_member_blueprint(blueprint, caps)?));
    Ok(VerifiedPhysicalMember {
        identity,
        physical_content: blueprint.physical_content.clone(),
        rest_mass_si,
        mass_dimension,
        derivation_kind,
        requirements,
    })
}

fn inspect_exact_zero_mass_proof(
    proof: &ExactZeroMassProof,
    requirements: &RequirementSet,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if proof.subject == proof.symmetry {
        return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
    }
    validate_descriptor(proof.subject, artifacts)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    validate_descriptor(proof.symmetry, artifacts)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    validate_content(&proof.excluded_term, caps)
        .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    let receipts = [
        &proof.applicability_receipt,
        &proof.exclusion_producer_receipt,
        &proof.exclusion_watchdog_receipt,
    ];
    for receipt in &receipts {
        validate_receipt(receipt, caps)
            .map_err(|_| PhysicalRegistryRefusalCode::UnprovedExactZero)?;
    }
    let receipt_set = receipts.into_iter().collect::<BTreeSet<_>>();
    if receipt_set.len() != receipts.len() {
        return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
    }
    let targets = requirements
        .artifact_relations
        .iter()
        .rev()
        .map(|relation| relation.target)
        .collect::<BTreeSet<_>>();
    if !(targets.contains(&proof.subject) && targets.contains(&proof.symmetry)) {
        return Err(PhysicalRegistryRefusalCode::UnprovedExactZero);
    }
    Ok(())
}

fn normalize_requirements(
    requirements: &RequirementSet,
    caps: ValidationCaps,
) -> Result<RequirementSet, PhysicalRegistryRefusalCode> {
    let normalized = RequirementSet {
        artifact_relations: normalize_relations(&requirements.artifact_relations, caps)?,
        species_dependencies: normalize_species_ids(&requirements.species_dependencies, caps)?,
    };
    if normalized.artifact_relations.is_empty() {
        return Err(PhysicalRegistryRefusalCode::RequirementSetEmpty);
    }
    Ok(normalized)
}

fn normalize_relations(
    relations: &[ArtifactRelation],
    caps: ValidationCaps,
) -> Result<Vec<ArtifactRelation>, PhysicalRegistryRefusalCode> {
    if u32::try_from(relations.len()).map_or(true, |count| count > caps.references_per_artifact) {
        return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
    }
    let mut unique = BTreeSet::new();
    for relation in relations.iter().rev() {
        if !unique.insert(relation.clone()) {
            return Err(PhysicalRegistryRefusalCode::DuplicateRequirement);
        }
    }
    Ok(unique.into_iter().collect())
}

fn normalize_species_ids(
    identities: &[SpeciesContentIdentity],
    caps: ValidationCaps,
) -> Result<Vec<SpeciesContentIdentity>, PhysicalRegistryRefusalCode> {
    if u32::try_from(identities.len()).map_or(true, |count| count > caps.references_per_artifact) {
        return Err(PhysicalRegistryRefusalCode::ReferenceCapacityExceeded);
    }
    let mut normalized = identities.to_vec();
    normalized.sort_unstable();
    if normalized.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PhysicalRegistryRefusalCode::DuplicateRequirement);
    }
    Ok(normalized)
}

fn validate_descriptor(
    identity: ArtifactIdentity,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let artifact = artifacts
        .get(&identity)
        .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
    if !matches!(artifact.payload, ArtifactPayload::PhysicalDescriptor(_)) {
        return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
    }
    Ok(())
}

fn validate_relations<F>(
    relations: &[ArtifactRelation],
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    target_matches: F,
) -> Result<Vec<ArtifactRelation>, PhysicalRegistryRefusalCode>
where
    F: Fn(&ArtifactPayload) -> bool,
{
    let relations = normalize_relations(relations, caps)?;
    for relation in relations.iter().rev() {
        validate_descriptor(relation.role, artifacts)?;
        let target = artifacts
            .get(&relation.target)
            .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
        if !target_matches(&target.payload) {
            return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
        }
    }
    Ok(relations)
}

fn normalize_declared_members(
    declared: &[SpeciesContentIdentity],
    caps: ValidationCaps,
) -> Result<Vec<SpeciesContentIdentity>, PhysicalRegistryRefusalCode> {
    if u32::try_from(declared.len()).map_or(true, |count| count > caps.registry_member_count) {
        return Err(PhysicalRegistryRefusalCode::RegistryCapacityExceeded);
    }
    let mut normalized = declared.to_vec();
    normalized.sort_unstable();
    if normalized.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(PhysicalRegistryRefusalCode::DuplicateRegistryMember);
    }
    Ok(normalized)
}

fn detect_derivation_cycles(
    rules: &BTreeMap<SpeciesContentIdentity, TracedRule>,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let mut colors = BTreeMap::<SpeciesContentIdentity, u8>::new();
    for start in rules.keys().copied() {
        if colors.get(&start).copied() == Some(2) {
            continue;
        }
        colors.insert(start, 1);
        let mut stack = vec![(start, 0_usize)];
        while let Some((identity, next_prerequisite)) = stack.last().copied() {
            let rule = rules
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::UnknownSpeciesDependency)?;
            if next_prerequisite == rule.prerequisites.len() {
                colors.insert(identity, 2);
                stack.pop();
                continue;
            }

            stack
                .last_mut()
                .ok_or(PhysicalRegistryRefusalCode::DerivationCycle)?
                .1 += 1;
            let prerequisite = rule.prerequisites[next_prerequisite];
            match colors.get(&prerequisite).copied().unwrap_or(0) {
                2 => continue,
                1 => return Err(PhysicalRegistryRefusalCode::DerivationCycle),
                _ => {}
            }
            if !rules.contains_key(&prerequisite) {
                return Err(PhysicalRegistryRefusalCode::UnknownSpeciesDependency);
            }
            if stack.len() >= rules.len() {
                return Err(PhysicalRegistryRefusalCode::DerivationCycle);
            }
            colors.insert(prerequisite, 1);
            stack.push((prerequisite, 0));
        }
    }
    Ok(())
}

fn reverse_trace_declared_members(
    declared: &[SpeciesContentIdentity],
    rules: &BTreeMap<SpeciesContentIdentity, TracedRule>,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let mut complete = BTreeSet::new();
    for start in declared.iter().rev().copied() {
        if complete.contains(&start) {
            continue;
        }
        if !rules.contains_key(&start) {
            return Err(PhysicalRegistryRefusalCode::ExtraClosureMember);
        }
        let mut stack = vec![(start, 0_usize)];
        while let Some((identity, next_prerequisite)) = stack.last().copied() {
            let rule = rules
                .get(&identity)
                .ok_or(PhysicalRegistryRefusalCode::ExtraClosureMember)?;
            if next_prerequisite == rule.prerequisites.len() {
                complete.insert(identity);
                stack.pop();
                continue;
            }

            stack
                .last_mut()
                .ok_or(PhysicalRegistryRefusalCode::ExtraClosureMember)?
                .1 += 1;
            let prerequisite = rule.prerequisites[rule.prerequisites.len() - next_prerequisite - 1];
            if complete.contains(&prerequisite) {
                continue;
            }
            if !rules.contains_key(&prerequisite) {
                return Err(PhysicalRegistryRefusalCode::UnknownSpeciesDependency);
            }
            if stack.iter().any(|(active, _)| *active == prerequisite) {
                return Err(PhysicalRegistryRefusalCode::DerivationCycle);
            }
            if stack.len() >= rules.len() {
                return Err(PhysicalRegistryRefusalCode::DerivationCycle);
            }
            stack.push((prerequisite, 0));
        }
    }
    Ok(())
}

fn forward_worklist_closure(
    rules: &BTreeMap<SpeciesContentIdentity, TracedRule>,
) -> Result<Vec<VerifiedPhysicalMember>, PhysicalRegistryRefusalCode> {
    let mut remaining = BTreeMap::<SpeciesContentIdentity, usize>::new();
    let mut consumers = BTreeMap::<SpeciesContentIdentity, Vec<SpeciesContentIdentity>>::new();
    let mut ready = BTreeSet::new();
    for (identity, rule) in rules {
        remaining.insert(*identity, rule.prerequisites.len());
        if rule.prerequisites.is_empty() {
            ready.insert(*identity);
        } else {
            for prerequisite in &rule.prerequisites {
                consumers.entry(*prerequisite).or_default().push(*identity);
            }
        }
    }
    for dependents in consumers.values_mut() {
        dependents.sort_unstable();
        dependents.dedup();
    }

    let mut closed = BTreeMap::<SpeciesContentIdentity, VerifiedPhysicalMember>::new();
    while let Some(identity) = ready.pop_first() {
        let rule = rules
            .get(&identity)
            .ok_or(PhysicalRegistryRefusalCode::ExtraClosureMember)?;
        if closed.insert(identity, rule.member.clone()).is_some() {
            continue;
        }
        if let Some(dependents) = consumers.get(&identity) {
            for dependent in dependents {
                let counter = remaining
                    .get_mut(dependent)
                    .ok_or(PhysicalRegistryRefusalCode::DependencyMismatch)?;
                *counter = counter
                    .checked_sub(1)
                    .ok_or(PhysicalRegistryRefusalCode::DependencyMismatch)?;
                if *counter == 0 {
                    ready.insert(*dependent);
                }
            }
        }
    }
    Ok(closed.into_values().collect())
}

fn compare_closure(
    members: &[VerifiedPhysicalMember],
    declared: &[SpeciesContentIdentity],
) -> Result<(), PhysicalRegistryRefusalCode> {
    let derived = members
        .iter()
        .map(|member| member.identity)
        .collect::<Vec<_>>();
    let mut left = 0;
    let mut right = 0;
    while left < derived.len() && right < declared.len() {
        match derived[left].cmp(&declared[right]) {
            Ordering::Less => return Err(PhysicalRegistryRefusalCode::MissingClosureMember),
            Ordering::Greater => return Err(PhysicalRegistryRefusalCode::ExtraClosureMember),
            Ordering::Equal => {
                left += 1;
                right += 1;
            }
        }
    }
    if left < derived.len() {
        return Err(PhysicalRegistryRefusalCode::MissingClosureMember);
    }
    if right < declared.len() {
        return Err(PhysicalRegistryRefusalCode::ExtraClosureMember);
    }
    Ok(())
}

fn evaluate_expression_postorder(
    expression: &ExactExpression,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Scalar, PhysicalRegistryRefusalCode> {
    let analysis = analyze_expression_shape(expression, caps, budget)?;
    let mut values = vec![None::<Scalar>; expression.nodes.len()];
    for index in analysis.postorder.iter().copied() {
        budget.evaluation(resource::expression_node_execution_work_units())?;
        let value = match &expression.nodes[index] {
            ExactExpressionNode::Coordinate(identity) => {
                let artifact = artifacts
                    .get(identity)
                    .ok_or(PhysicalRegistryRefusalCode::UnknownArtifactReference)?;
                let ArtifactPayload::ScalarCoordinate(coordinate) = &artifact.payload else {
                    return Err(PhysicalRegistryRefusalCode::ArtifactKindMismatch);
                };
                Scalar {
                    value: decode_rational(&coordinate.exact_value, caps, budget)?,
                    dimension: checked_dimension(coordinate.dimension, caps)?,
                }
            }
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Subtract { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                let left = checked_node_index(*left, expression.nodes.len())?;
                let right = checked_node_index(*right, expression.nodes.len())?;
                let left_value = values[left]
                    .as_ref()
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                let right_value = values[right]
                    .as_ref()
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                match &expression.nodes[index] {
                    ExactExpressionNode::Add { .. } => {
                        if left_value.dimension != right_value.dimension {
                            return Err(PhysicalRegistryRefusalCode::DimensionMismatch);
                        }
                        Scalar {
                            value: add_rationals(
                                &left_value.value,
                                &right_value.value,
                                false,
                                caps,
                                budget,
                            )?,
                            dimension: left_value.dimension,
                        }
                    }
                    ExactExpressionNode::Subtract { .. } => {
                        if left_value.dimension != right_value.dimension {
                            return Err(PhysicalRegistryRefusalCode::DimensionMismatch);
                        }
                        Scalar {
                            value: add_rationals(
                                &left_value.value,
                                &right_value.value,
                                true,
                                caps,
                                budget,
                            )?,
                            dimension: left_value.dimension,
                        }
                    }
                    ExactExpressionNode::Multiply { .. } => Scalar {
                        value: multiply_rationals(
                            &left_value.value,
                            &right_value.value,
                            caps,
                            budget,
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
                let numerator = checked_node_index(*numerator, expression.nodes.len())?;
                let denominator = checked_node_index(*denominator, expression.nodes.len())?;
                let top = values[numerator]
                    .as_ref()
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                let bottom = values[denominator]
                    .as_ref()
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                Scalar {
                    value: divide_rationals(&top.value, &bottom.value, caps, budget)?,
                    dimension: combine_dimensions(top.dimension, bottom.dimension, true, caps)?,
                }
            }
            ExactExpressionNode::IntegerPower { base, exponent } => {
                let base = checked_node_index(*base, expression.nodes.len())?;
                let base_value = values[base]
                    .as_ref()
                    .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
                Scalar {
                    value: power_rational(&base_value.value, *exponent, caps, budget)?,
                    dimension: power_dimension(base_value.dimension, *exponent, caps)?,
                }
            }
        };
        values[index] = Some(value);
    }
    values[analysis.output]
        .clone()
        .ok_or(PhysicalRegistryRefusalCode::ExpressionOutputInvalid)
}

struct ExpressionShape {
    digests: Vec<[u8; 32]>,
    output: usize,
    edge_count: u32,
    postorder: Vec<usize>,
}

fn analyze_expression_shape(
    expression: &ExactExpression,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<ExpressionShape, PhysicalRegistryRefusalCode> {
    if expression.nodes.is_empty() {
        return Err(PhysicalRegistryRefusalCode::ExpressionEmpty);
    }
    if u32::try_from(expression.nodes.len())
        .map_or(true, |count| count > caps.expression_node_count)
    {
        return Err(PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded);
    }
    let output = checked_node_index(expression.output_node, expression.nodes.len())?;

    let edge_count = expression.nodes.iter().try_fold(0_u32, |total, node| {
        total
            .checked_add(match node {
                ExactExpressionNode::Coordinate(_) => 0,
                ExactExpressionNode::IntegerPower { .. } => 1,
                _ => 2,
            })
            .ok_or(PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded)
    })?;
    if edge_count > caps.expression_edge_count {
        return Err(PhysicalRegistryRefusalCode::ExpressionEdgeCapacityExceeded);
    }
    let node_count = u32::try_from(expression.nodes.len())
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded)?;
    budget.evaluation(resource::expression_work_units(node_count, edge_count)?)?;

    let mut colors = vec![0_u8; expression.nodes.len()];
    let mut depths = vec![0_u32; expression.nodes.len()];
    let mut digests = vec![[0_u8; 32]; expression.nodes.len()];
    let mut postorder = Vec::with_capacity(expression.nodes.len());
    let mut stack = vec![(output, false, 1_u32)];
    while let Some((index, exiting, path_depth)) = stack.pop() {
        if exiting {
            let (digest, depth) =
                digest_completed_expression_node(index, expression, caps, &digests, &depths)?;
            if depth > caps.expression_depth {
                return Err(PhysicalRegistryRefusalCode::ExpressionDepthExceeded);
            }
            digests[index] = digest;
            depths[index] = depth;
            colors[index] = 2;
            postorder.push(index);
            continue;
        }

        match colors[index] {
            2 => continue,
            1 => return Err(PhysicalRegistryRefusalCode::ExpressionCycle),
            _ => {}
        }
        if path_depth > caps.expression_depth {
            return Err(PhysicalRegistryRefusalCode::ExpressionDepthExceeded);
        }
        colors[index] = 1;
        stack.push((index, true, path_depth));
        let child_depth = path_depth
            .checked_add(1)
            .ok_or(PhysicalRegistryRefusalCode::ExpressionDepthExceeded)?;
        match &expression.nodes[index] {
            ExactExpressionNode::Coordinate(_) => {}
            ExactExpressionNode::Add { left, right }
            | ExactExpressionNode::Subtract { left, right }
            | ExactExpressionNode::Multiply { left, right } => {
                let left = checked_node_index(*left, expression.nodes.len())?;
                let right = checked_node_index(*right, expression.nodes.len())?;
                stack.push((left, false, child_depth));
                stack.push((right, false, child_depth));
            }
            ExactExpressionNode::Divide {
                numerator,
                denominator,
            } => {
                let numerator = checked_node_index(*numerator, expression.nodes.len())?;
                let denominator = checked_node_index(*denominator, expression.nodes.len())?;
                stack.push((numerator, false, child_depth));
                stack.push((denominator, false, child_depth));
            }
            ExactExpressionNode::IntegerPower { base, .. } => {
                let base = checked_node_index(*base, expression.nodes.len())?;
                stack.push((base, false, child_depth));
            }
        }
    }
    if colors.contains(&0) {
        return Err(PhysicalRegistryRefusalCode::ExpressionContainsUnusedNode);
    }
    Ok(ExpressionShape {
        digests,
        output,
        edge_count,
        postorder,
    })
}

fn digest_completed_expression_node(
    index: usize,
    expression: &ExactExpression,
    caps: ValidationCaps,
    digests: &[[u8; 32]],
    depths: &[u32],
) -> Result<([u8; 32], u32), PhysicalRegistryRefusalCode> {
    let (digest, depth) = match &expression.nodes[index] {
        ExactExpressionNode::Coordinate(identity) => {
            let mut record = WireRecord::start(EXPRESSION_NODE_DOMAIN, caps)?;
            record.push(1, b"coordinate")?;
            record.push(2, &identity.0)?;
            (sha256(&record.finish()), 1)
        }
        ExactExpressionNode::Add { left, right }
        | ExactExpressionNode::Multiply { left, right } => {
            let left = checked_node_index(*left, expression.nodes.len())?;
            let right = checked_node_index(*right, expression.nodes.len())?;
            let mut children = [digests[left], digests[right]];
            children.sort_unstable();
            let mut record = WireRecord::start(EXPRESSION_NODE_DOMAIN, caps)?;
            record.push(
                1,
                if matches!(expression.nodes[index], ExactExpressionNode::Add { .. }) {
                    b"add"
                } else {
                    b"multiply"
                },
            )?;
            record.push(2, &children[0])?;
            record.push(3, &children[1])?;
            (
                sha256(&record.finish()),
                depths[left].max(depths[right]).saturating_add(1),
            )
        }
        ExactExpressionNode::Subtract { left, right } => {
            let left = checked_node_index(*left, expression.nodes.len())?;
            let right = checked_node_index(*right, expression.nodes.len())?;
            let mut record = WireRecord::start(EXPRESSION_NODE_DOMAIN, caps)?;
            record.push(1, b"subtract")?;
            record.push(2, &digests[left])?;
            record.push(3, &digests[right])?;
            (
                sha256(&record.finish()),
                depths[left].max(depths[right]).saturating_add(1),
            )
        }
        ExactExpressionNode::Divide {
            numerator,
            denominator,
        } => {
            let numerator = checked_node_index(*numerator, expression.nodes.len())?;
            let denominator = checked_node_index(*denominator, expression.nodes.len())?;
            let mut record = WireRecord::start(EXPRESSION_NODE_DOMAIN, caps)?;
            record.push(1, b"divide")?;
            record.push(2, &digests[numerator])?;
            record.push(3, &digests[denominator])?;
            (
                sha256(&record.finish()),
                depths[numerator].max(depths[denominator]).saturating_add(1),
            )
        }
        ExactExpressionNode::IntegerPower { base, exponent } => {
            let base = checked_node_index(*base, expression.nodes.len())?;
            let mut record = WireRecord::start(EXPRESSION_NODE_DOMAIN, caps)?;
            record.push(1, b"integer-power")?;
            record.push(2, &digests[base])?;
            record.push(3, &exponent.to_be_bytes())?;
            (sha256(&record.finish()), depths[base].saturating_add(1))
        }
    };
    Ok((digest, depth))
}

fn checked_node_index(index: u32, length: usize) -> Result<usize, PhysicalRegistryRefusalCode> {
    let index =
        usize::try_from(index).map_err(|_| PhysicalRegistryRefusalCode::ExpressionOutputInvalid)?;
    if index >= length {
        return Err(PhysicalRegistryRefusalCode::ExpressionOutputInvalid);
    }
    Ok(index)
}

fn decode_rational(
    wire: &ExactRationalWire,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    validate_rational_component(&wire.numerator_be, caps)?;
    validate_rational_component(&wire.denominator_be, caps)?;
    let numerator = decode_big_endian_from_right(&wire.numerator_be, budget)?;
    let denominator = decode_big_endian_from_right(&wire.denominator_be, budget)?;
    if denominator.is_zero() || (wire.negative && numerator.is_zero()) {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    let common = numerator.gcd(&denominator);
    if common.cmp_big(&BigUint::from_u64(1)) != Ordering::Equal {
        return Err(PhysicalRegistryRefusalCode::RationalNotReduced);
    }
    let value = Rational {
        negative: wire.negative,
        numerator,
        denominator,
    };
    check_rational_size(&value, caps)?;
    Ok(value)
}

fn validate_rational_component(
    bytes: &[u8],
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    let Some(first) = bytes.first().copied() else {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    };
    if bytes.len() > 1 && first == 0 {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    let bits = if first == 0 {
        0
    } else {
        u32::try_from(bytes.len() - 1)
            .map_err(|_| PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)?
            .checked_mul(8)
            .and_then(|tail| tail.checked_add(first.ilog2() + 1))
            .ok_or(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)?
    };
    if bits > caps.rational_component_bits {
        return Err(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded);
    }
    Ok(())
}

fn decode_big_endian_from_right(
    bytes: &[u8],
    budget: &mut Budget,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    budget.evaluation(resource::component_decode_work_units(bytes.len())?)?;
    let mut value = BigUint::zero();
    for (position, byte) in bytes.iter().rev().enumerate() {
        if *byte == 0 {
            continue;
        }
        let shift = u32::try_from(position)
            .ok()
            .and_then(|position| position.checked_mul(8))
            .ok_or(PhysicalRegistryRefusalCode::RationalComponentLimitExceeded)?;
        value = value.add(&BigUint::from_u64(u64::from(*byte)).shl_bits(shift));
    }
    Ok(value)
}

fn add_rationals(
    left: &Rational,
    right: &Rational,
    subtract_right: bool,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    budget.evaluation(resource::additive_rational_core_work_units())?;
    let common_denominator = left.denominator.gcd(&right.denominator);
    let (left_factor, _) = right.denominator.divmod(&common_denominator);
    let (right_factor, _) = left.denominator.divmod(&common_denominator);
    let left_numerator = bounded_product(&left.numerator, &left_factor, caps, budget)?;
    let right_numerator = bounded_product(&right.numerator, &right_factor, caps, budget)?;
    let denominator = bounded_product(&left.denominator, &left_factor, caps, budget)?;
    let right_negative = right.negative ^ subtract_right;
    let (negative, numerator) = match (left.negative, right_negative) {
        (false, false) => (false, left_numerator.add(&right_numerator)),
        (true, true) => (true, left_numerator.add(&right_numerator)),
        _ => match left_numerator.cmp_big(&right_numerator) {
            Ordering::Greater => (left.negative, left_numerator.sub(&right_numerator)),
            Ordering::Less => (right_negative, right_numerator.sub(&left_numerator)),
            Ordering::Equal => (false, BigUint::zero()),
        },
    };
    reduce_rational(
        Rational {
            negative,
            numerator,
            denominator,
        },
        caps,
        budget,
    )
}

fn multiply_rationals(
    left: &Rational,
    right: &Rational,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    budget.evaluation(resource::multiplicative_rational_core_work_units())?;
    let cross_left = left.numerator.gcd(&right.denominator);
    let cross_right = right.numerator.gcd(&left.denominator);
    let (left_numerator, _) = left.numerator.divmod(&cross_left);
    let (right_denominator, _) = right.denominator.divmod(&cross_left);
    let (right_numerator, _) = right.numerator.divmod(&cross_right);
    let (left_denominator, _) = left.denominator.divmod(&cross_right);
    reduce_rational(
        Rational {
            negative: left.negative ^ right.negative,
            numerator: bounded_product(&left_numerator, &right_numerator, caps, budget)?,
            denominator: bounded_product(&left_denominator, &right_denominator, caps, budget)?,
        },
        caps,
        budget,
    )
}

fn divide_rationals(
    numerator: &Rational,
    denominator: &Rational,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    if denominator.numerator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    let reciprocal = Rational {
        negative: denominator.negative,
        numerator: denominator.denominator.clone(),
        denominator: denominator.numerator.clone(),
    };
    multiply_rationals(numerator, &reciprocal, caps, budget)
}

fn power_rational(
    value: &Rational,
    exponent: i16,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    if exponent < 0 && value.numerator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    let mut power = u32::from(exponent.unsigned_abs());
    budget.evaluation(watchdog_power_work_units(power)?)?;
    let mut factor = if exponent < 0 {
        Rational {
            negative: value.negative,
            numerator: value.denominator.clone(),
            denominator: value.numerator.clone(),
        }
    } else {
        value.clone()
    };
    let mut result = Rational {
        negative: false,
        numerator: BigUint::from_u64(1),
        denominator: BigUint::from_u64(1),
    };
    while power != 0 {
        if power & 1 == 1 {
            result = multiply_rationals_for_power(&result, &factor, caps)?;
        }
        power >>= 1;
        if power != 0 {
            factor = multiply_rationals_for_power(&factor, &factor, caps)?;
        }
    }
    Ok(result)
}

fn watchdog_power_work_units(exponent: u32) -> Result<u64, PhysicalRegistryRefusalCode> {
    resource::power_work_units(exponent)
}

#[cfg(test)]
pub(super) fn power_work_units_for_test(exponent: u32) -> Result<u64, PhysicalRegistryRefusalCode> {
    watchdog_power_work_units(exponent)
}

fn multiply_rationals_for_power(
    left: &Rational,
    right: &Rational,
    caps: ValidationCaps,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    let left_cross = left.numerator.gcd(&right.denominator);
    let right_cross = right.numerator.gcd(&left.denominator);
    let (left_numerator, left_remainder) = left.numerator.divmod(&left_cross);
    let (right_denominator, right_denominator_remainder) = right.denominator.divmod(&left_cross);
    let (right_numerator, right_remainder) = right.numerator.divmod(&right_cross);
    let (left_denominator, left_denominator_remainder) = left.denominator.divmod(&right_cross);
    if !left_remainder.is_zero()
        || !right_denominator_remainder.is_zero()
        || !right_remainder.is_zero()
        || !left_denominator_remainder.is_zero()
    {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    reduce_rational_for_power(
        Rational {
            negative: left.negative ^ right.negative,
            numerator: bounded_product_value(&left_numerator, &right_numerator, caps)?,
            denominator: bounded_product_value(&left_denominator, &right_denominator, caps)?,
        },
        caps,
    )
}

fn bounded_product(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    budget.evaluation(resource::bounded_product_work_units())?;
    bounded_product_value(left, right, caps)
}

fn bounded_product_value(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
) -> Result<BigUint, PhysicalRegistryRefusalCode> {
    if !watchdog_product_fits_component_cap(left, right, caps.intermediate_component_bits) {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    let product = left.mul(right);
    if product.bit_len() > caps.intermediate_component_bits {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    Ok(product)
}

fn watchdog_product_fits_component_cap(left: &BigUint, right: &BigUint, cap: u32) -> bool {
    resource::product_fits_component_cap(left, right, cap)
}

#[cfg(test)]
pub(super) fn product_fits_component_cap_for_test(
    left: &BigUint,
    right: &BigUint,
    cap: u32,
) -> bool {
    watchdog_product_fits_component_cap(left, right, cap)
}

fn reduce_rational(
    mut value: Rational,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    if value.denominator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    budget.evaluation(resource::rational_reduction_work_units(
        value.numerator.is_zero(),
    ))?;
    if value.numerator.is_zero() {
        value.negative = false;
        value.denominator = BigUint::from_u64(1);
        return Ok(value);
    }
    let common = value.numerator.gcd(&value.denominator);
    let (numerator, numerator_remainder) = value.numerator.divmod(&common);
    let (denominator, denominator_remainder) = value.denominator.divmod(&common);
    if !numerator_remainder.is_zero() || !denominator_remainder.is_zero() {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    value.numerator = numerator;
    value.denominator = denominator;
    check_rational_size(&value, caps)?;
    Ok(value)
}

fn reduce_rational_for_power(
    mut value: Rational,
    caps: ValidationCaps,
) -> Result<Rational, PhysicalRegistryRefusalCode> {
    if value.denominator.is_zero() {
        return Err(PhysicalRegistryRefusalCode::DivisionByZero);
    }
    if value.numerator.is_zero() {
        value.negative = false;
        value.denominator = BigUint::from_u64(1);
        return Ok(value);
    }
    let common = value.numerator.gcd(&value.denominator);
    let (numerator, numerator_remainder) = value.numerator.divmod(&common);
    let (denominator, denominator_remainder) = value.denominator.divmod(&common);
    if !numerator_remainder.is_zero() || !denominator_remainder.is_zero() {
        return Err(PhysicalRegistryRefusalCode::RationalEncodingInvalid);
    }
    value.numerator = numerator;
    value.denominator = denominator;
    check_rational_size(&value, caps)?;
    Ok(value)
}

fn check_rational_size(
    value: &Rational,
    caps: ValidationCaps,
) -> Result<(), PhysicalRegistryRefusalCode> {
    if value.numerator.bit_len() > caps.intermediate_component_bits
        || value.denominator.bit_len() > caps.intermediate_component_bits
    {
        return Err(PhysicalRegistryRefusalCode::IntermediateComponentLimitExceeded);
    }
    Ok(())
}

fn rational_to_wire(value: &Rational) -> Result<ExactRationalWire, PhysicalRegistryRefusalCode> {
    Ok(ExactRationalWire {
        negative: value.negative,
        numerator_be: encode_biguint_by_words(&value.numerator)?,
        denominator_be: encode_biguint_by_words(&value.denominator)?,
    })
}

fn encode_biguint_by_words(value: &BigUint) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    if value.is_zero() {
        return Ok(vec![0]);
    }
    let radix = BigUint::from_u64(65_536);
    let mut cursor = value.clone();
    let mut words = Vec::<u16>::new();
    while !cursor.is_zero() {
        let (quotient, remainder) = cursor.divmod(&radix);
        let word = remainder
            .to_u128()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or(PhysicalRegistryRefusalCode::RationalEncodingInvalid)?;
        words.push(word);
        cursor = quotient;
    }
    words.reverse();
    let mut bytes = Vec::with_capacity(words.len() * 2);
    for (index, word) in words.into_iter().enumerate() {
        let encoded = word.to_be_bytes();
        if index == 0 && encoded[0] == 0 {
            bytes.push(encoded[1]);
        } else {
            bytes.extend_from_slice(&encoded);
        }
    }
    Ok(bytes)
}

fn checked_dimension(
    dimension: DimensionVector,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    if u32::try_from(dimension.terms().len())
        .map_or(true, |count| count > caps.dimension_term_count)
    {
        return Err(PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded);
    }
    if dimension
        .terms()
        .iter()
        .any(|term| i32::from(term.exponent).abs() > caps.dimension_abs_exponent)
    {
        return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
    }
    Ok(dimension)
}

fn combine_dimensions(
    left: DimensionVector,
    right: DimensionVector,
    subtract_right: bool,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    let mut result = Vec::new();
    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.terms().len() || right_index < right.terms().len() {
        let (axis, left_exponent, right_exponent) =
            match (left.terms().get(left_index), right.terms().get(right_index)) {
                (Some(left_term), Some(right_term)) => match left_term.axis.cmp(&right_term.axis) {
                    Ordering::Less => {
                        left_index += 1;
                        (left_term.axis, i32::from(left_term.exponent), 0)
                    }
                    Ordering::Greater => {
                        right_index += 1;
                        (right_term.axis, 0, i32::from(right_term.exponent))
                    }
                    Ordering::Equal => {
                        left_index += 1;
                        right_index += 1;
                        (
                            left_term.axis,
                            i32::from(left_term.exponent),
                            i32::from(right_term.exponent),
                        )
                    }
                },
                (Some(left_term), None) => {
                    left_index += 1;
                    (left_term.axis, i32::from(left_term.exponent), 0)
                }
                (None, Some(right_term)) => {
                    right_index += 1;
                    (right_term.axis, 0, i32::from(right_term.exponent))
                }
                (None, None) => break,
            };
        let signed_right = if subtract_right {
            -right_exponent
        } else {
            right_exponent
        };
        let value = left_exponent
            .checked_add(signed_right)
            .ok_or(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?;
        if value.abs() > caps.dimension_abs_exponent {
            return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
        }
        if value != 0 {
            result.push(DimensionTerm {
                axis,
                exponent: i16::try_from(value)
                    .map_err(|_| PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?,
            });
        }
    }
    DimensionVector::from_terms(result)
}

fn power_dimension(
    dimension: DimensionVector,
    exponent: i16,
    caps: ValidationCaps,
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    if exponent == 0 {
        return Ok(DimensionVector::dimensionless());
    }
    let mut result = Vec::with_capacity(dimension.terms().len());
    for term in dimension.terms().iter().rev() {
        let value = i32::from(term.exponent)
            .checked_mul(i32::from(exponent))
            .ok_or(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?;
        if value.abs() > caps.dimension_abs_exponent {
            return Err(PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded);
        }
        result.push(DimensionTerm {
            axis: term.axis,
            exponent: i16::try_from(value)
                .map_err(|_| PhysicalRegistryRefusalCode::DimensionExponentLimitExceeded)?,
        });
    }
    DimensionVector::from_terms(result)
}

fn write_artifact_payload(
    payload: &ArtifactPayload,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let identity_domain = match payload {
        ArtifactPayload::ExactMasslessLaw(_) => EXACT_MASSLESS_ARTIFACT_DOMAIN_V4,
        _ => ARTIFACT_DOMAIN_V3,
    };
    let mut record = WireRecord::start(identity_domain, caps)?;
    match payload {
        ArtifactPayload::ScalarCoordinate(coordinate) => {
            validate_content(&coordinate.coordinate, caps)?;
            decode_rational(&coordinate.exact_value, caps, budget)?;
            checked_dimension(coordinate.dimension, caps)?;
            record.push(1, b"scalar-coordinate")?;
            record.push(2, &write_content(&coordinate.coordinate, caps)?)?;
            record.push(3, &write_rational(&coordinate.exact_value, caps)?)?;
            record.push(4, &write_dimension(coordinate.dimension))?;
        }
        ArtifactPayload::PhysicalDescriptor(content) => {
            record.push(1, b"physical-descriptor")?;
            record.push(2, &write_content(content, caps)?)?;
        }
        ArtifactPayload::ConstraintLaw(law) => {
            record.push(1, b"constraint-law")?;
            record.push(2, &write_requirements(&law.requirements, caps)?)?;
        }
        ArtifactPayload::MassProjection(projection) => {
            record.push(1, b"mass-projection")?;
            record.push(2, &write_expression(&projection.expression, caps, budget)?)?;
            record.push(3, projection.scope.id().as_bytes())?;
        }
        ArtifactPayload::ExactMasslessLaw(law) => {
            record.push(1, b"exact-massless-law")?;
            record.push(2, &write_requirements(&law.requirements, caps)?)?;
            record.push(3, &law.proof.subject.0)?;
            record.push(4, &write_content(&law.proof.excluded_term, caps)?)?;
            record.push(5, &law.proof.symmetry.0)?;
            record.push(6, &write_receipt(&law.proof.applicability_receipt, caps)?)?;
            record.push(
                7,
                &write_receipt(&law.proof.exclusion_producer_receipt, caps)?,
            )?;
            record.push(
                8,
                &write_receipt(&law.proof.exclusion_watchdog_receipt, caps)?,
            )?;
        }
        ArtifactPayload::SpeciesDerivation(rule) => {
            record.push(1, b"species-derivation")?;
            record.push(2, &rule.derivation_kind.0)?;
            record.push(3, &write_relations(&rule.artifact_inputs, caps)?)?;
            record.push(4, &write_member_list(&rule.constituents, caps)?)?;
            record.push(5, &write_member_blueprint(&rule.output, caps)?)?;
        }
    }
    Ok(record.finish())
}

fn write_expression(
    expression: &ExactExpression,
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut shape = analyze_expression_shape(expression, caps, budget)?;
    let output_digest = shape.digests[shape.output];
    shape.digests.sort_unstable();
    let mut record = WireRecord::start(EXPRESSION_DOMAIN, caps)?;
    record.push(1, &output_digest)?;
    let node_count = u32::try_from(shape.digests.len())
        .map_err(|_| PhysicalRegistryRefusalCode::ExpressionNodeCapacityExceeded)?;
    record.push(2, &node_count.to_be_bytes())?;
    record.push(3, &shape.edge_count.to_be_bytes())?;
    for digest in shape.digests {
        record.push(4, &digest)?;
    }
    Ok(record.finish())
}

fn write_content(
    content: &CanonicalArtifact,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    validate_content(content, caps)?;
    let mut record = WireRecord::start(b"civsim.physical-species.content.v1", caps)?;
    record.push(1, content.schema_id.as_bytes())?;
    record.push(2, &content.canonical_bytes)?;
    Ok(record.finish())
}

fn write_receipt(
    receipt: &ReceiptBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    validate_receipt(receipt, caps)?;
    let mut record = WireRecord::start(b"civsim.physical-species.receipt.v1", caps)?;
    record.push(1, receipt.schema_id.as_bytes())?;
    record.push(2, &receipt.digest_sha256)?;
    Ok(record.finish())
}

fn write_admission(
    admission: &RootAdmission,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut record = WireRecord::start(b"civsim.physical-species.admission.v1", caps)?;
    record.push(1, admission.tier.id().as_bytes())?;
    record.push(
        2,
        admission
            .provenance
            .bracket_tag()
            .ok_or(PhysicalRegistryRefusalCode::NoncanonicalProvenance)?
            .as_bytes(),
    )?;
    match &admission.route {
        AdmissionRoute::Derived(route) => {
            record.push(3, b"derived")?;
            record.push(4, &write_receipt(&route.ancestry_receipt, caps)?)?;
            record.push(5, &write_receipt(&route.semantic_checker_receipt, caps)?)?;
            record.push(
                6,
                &write_receipt(&route.independent_watchdog_receipt, caps)?,
            )?;
        }
        AdmissionRoute::Irreducible(route) => {
            record.push(3, b"irreducible")?;
            record.push(
                4,
                &write_receipt(&route.derivation_exhaustion_receipt, caps)?,
            )?;
            record.push(5, &write_receipt(&route.buckingham_pi_receipt, caps)?)?;
            record.push(6, &write_receipt(&route.gap_law_receipt, caps)?)?;
            record.push(7, &write_receipt(&route.chaos_protocol_receipt, caps)?)?;
            record.push(8, &write_receipt(&route.residual_law_receipt, caps)?)?;
            record.push(9, route.residual_slot_id.as_bytes())?;
            record.push(10, &write_receipt(&route.residual_slot_receipt, caps)?)?;
            record.push(11, &write_receipt(&route.owner_admission_receipt, caps)?)?;
            record.push(
                12,
                &write_receipt(&route.independent_watchdog_receipt, caps)?,
            )?;
        }
        AdmissionRoute::EvidenceCustodyOnly { source_receipt } => {
            record.push(3, b"evidence-custody-only")?;
            record.push(4, &write_receipt(source_receipt, caps)?)?;
        }
    }
    Ok(record.finish())
}

fn write_member_blueprint(
    member: &MemberBlueprint,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut record = WireRecord::start(MEMBER_DOMAIN, caps)?;
    record.push(1, &write_content(&member.physical_content, caps)?)?;
    record.push(2, &write_requirements(&member.requirements, caps)?)?;
    match member.mass_proof {
        MassProofReference::Projection(identity) => {
            record.push(3, b"projection")?;
            record.push(4, &identity.0)?;
        }
        MassProofReference::ExactMassless(identity) => {
            record.push(3, b"exact-massless")?;
            record.push(4, &identity.0)?;
        }
    }
    record.push(5, &write_relations(&member.constraint_laws, caps)?)?;
    Ok(record.finish())
}

fn write_requirements(
    requirements: &RequirementSet,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut record = WireRecord::start(b"civsim.physical-species.requirements.v1", caps)?;
    record.push(1, &write_relations(&requirements.artifact_relations, caps)?)?;
    record.push(
        2,
        &write_member_list(&requirements.species_dependencies, caps)?,
    )?;
    Ok(record.finish())
}

fn write_relations(
    relations: &[ArtifactRelation],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let ordered = normalize_relations(relations, caps)?;
    let mut record = WireRecord::start(b"civsim.physical-species.relations.v1", caps)?;
    for relation in ordered {
        let mut edge = WireRecord::start(b"civsim.physical-species.relation.v1", caps)?;
        edge.push(1, &relation.role.0)?;
        edge.push(2, &relation.target.0)?;
        record.push(1, &edge.finish())?;
    }
    Ok(record.finish())
}

fn write_member_list(
    values: &[SpeciesContentIdentity],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let mut record = WireRecord::start(b"civsim.physical-species.member-list.v1", caps)?;
    for identity in ordered {
        record.push(1, &identity.0)?;
    }
    Ok(record.finish())
}

fn write_rational(
    value: &ExactRationalWire,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut record = WireRecord::start(b"civsim.physical-species.rational.v1", caps)?;
    record.push(1, &[u8::from(value.negative)])?;
    record.push(2, &value.numerator_be)?;
    record.push(3, &value.denominator_be)?;
    Ok(record.finish())
}

fn write_dimension(dimension: DimensionVector) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + dimension.terms().len() * 34);
    bytes.extend_from_slice(
        &u32::try_from(dimension.terms().len())
            .expect("dimension term storage fits u32")
            .to_be_bytes(),
    );
    for term in dimension.terms() {
        bytes.extend_from_slice(&term.axis.0);
        bytes.extend_from_slice(&term.exponent.to_be_bytes());
    }
    bytes
}

fn write_resources(
    resources: PhysicalRegistryResourceContract,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let payloads = [
        resources.max_artifact_count.to_be_bytes().to_vec(),
        resources.max_registry_member_count.to_be_bytes().to_vec(),
        resources.max_references_per_artifact.to_be_bytes().to_vec(),
        resources.max_total_reference_count.to_be_bytes().to_vec(),
        resources.max_expression_node_count.to_be_bytes().to_vec(),
        resources.max_expression_edge_count.to_be_bytes().to_vec(),
        resources.max_expression_depth.to_be_bytes().to_vec(),
        resources.max_rational_component_bits.to_be_bytes().to_vec(),
        resources
            .max_intermediate_component_bits
            .to_be_bytes()
            .to_vec(),
        resources.max_dimension_abs_exponent.to_be_bytes().to_vec(),
        resources.max_dimension_term_count.to_be_bytes().to_vec(),
        resources.max_evaluation_steps.to_be_bytes().to_vec(),
        resources.max_closure_steps.to_be_bytes().to_vec(),
        resources.max_canonical_bytes.to_be_bytes().to_vec(),
        resources.max_canonical_token_bytes.to_be_bytes().to_vec(),
        resources.max_content_bytes.to_be_bytes().to_vec(),
    ];
    let mut record = WireRecord::start(b"civsim.physical-species.resources.v1", caps)?;
    for (index, payload) in payloads.iter().enumerate() {
        let tag = u16::try_from(index + 1)
            .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        record.push(tag, payload)?;
    }
    Ok(record.finish())
}

fn write_vocabulary_binding(
    binding: &PhysicalVocabularyBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut record = WireRecord::start(b"civsim.physical-species.vocabulary-binding.v1", caps)?;
    let text_fields = [
        binding.schema_id.as_bytes(),
        binding.claim_id.as_bytes(),
        binding.producer_id.as_bytes(),
        binding.watchdog_id.as_bytes(),
    ];
    for (index, value) in text_fields.iter().enumerate() {
        let tag = u16::try_from(index + 1)
            .map_err(|_| PhysicalRegistryRefusalCode::CanonicalByteLimitExceeded)?;
        record.push(tag, value)?;
    }
    record.push(5, &binding.root_count.to_be_bytes())?;
    for (tag, identities) in [
        (6, binding.descriptor_role_identities.as_slice()),
        (7, binding.relation_target_identities.as_slice()),
        (8, binding.constraint_law_identities.as_slice()),
    ] {
        for identity in identities {
            record.push(tag, &identity.0)?;
        }
    }
    for (tag, flag) in [
        (9, binding.current_input_partition_complete),
        (10, binding.global_physical_vocabulary_coverage),
        (11, binding.membership_authority),
    ] {
        record.push(tag, &[u8::from(flag)])?;
    }
    for (tag, digest) in [
        (12, binding.producer_result_sha256),
        (13, binding.watchdog_result_sha256),
        (14, binding.producer_resource_contract_sha256),
        (15, binding.watchdog_resource_contract_sha256),
        (16, binding.receipt_sha256),
    ] {
        record.push(tag, &digest)?;
    }
    Ok(record.finish())
}

fn write_registry(
    input: &PhysicalRegistryInput,
    artifacts: &BTreeMap<ArtifactIdentity, &AdmittedArtifact>,
    members: &[VerifiedPhysicalMember],
    caps: ValidationCaps,
    budget: &mut Budget,
) -> Result<Vec<u8>, PhysicalRegistryRefusalCode> {
    let mut registry = WireRecord::start(REGISTRY_DOMAIN, caps)?;
    registry.push(1, input.schema_id.as_bytes())?;
    registry.push(2, input.proof_graph_schema_id.as_bytes())?;
    registry.push(3, &write_receipt(&input.floor_binding, caps)?)?;
    registry.push(4, input.structure_binding.structure_schema_id.as_bytes())?;
    registry.push(
        5,
        input
            .structure_binding
            .species_registry_schema_id
            .as_bytes(),
    )?;
    registry.push(
        6,
        input.structure_binding.stellar_state_schema_id.as_bytes(),
    )?;
    registry.push(
        7,
        input
            .structure_binding
            .state_coordinate_registry_schema_id
            .as_bytes(),
    )?;
    registry.push(
        8,
        input
            .structure_binding
            .interaction_sector_registry_schema_id
            .as_bytes(),
    )?;
    registry.push(
        9,
        input
            .structure_binding
            .physical_regime_registry_schema_id
            .as_bytes(),
    )?;
    registry.push(10, input.checker_pair.producer_id.as_bytes())?;
    registry.push(11, input.checker_pair.watchdog_id.as_bytes())?;
    registry.push(12, &write_resources(input.resources, caps)?)?;
    registry.push(
        13,
        &write_vocabulary_binding(&input.vocabulary_binding, caps)?,
    )?;

    for artifact in artifacts.values() {
        budget.evaluation(resource::registry_artifact_work_units())?;
        let mut record = WireRecord::start(b"civsim.physical-species.admitted-artifact.v1", caps)?;
        record.push(1, &artifact.claimed_identity.0)?;
        record.push(2, &write_admission(&artifact.admission, caps)?)?;
        record.push(3, &write_artifact_payload(&artifact.payload, caps, budget)?)?;
        registry.push(14, &record.finish())?;
    }
    for member in members {
        let mut record = WireRecord::start(b"civsim.physical-species.verified-member.v1", caps)?;
        record.push(1, &member.identity.0)?;
        record.push(2, &write_content(&member.physical_content, caps)?)?;
        record.push(3, &write_rational(&member.rest_mass_si, caps)?)?;
        record.push(4, &write_dimension(member.mass_dimension))?;
        record.push(5, &member.derivation_kind.0)?;
        record.push(6, &write_requirements(&member.requirements, caps)?)?;
        registry.push(15, &record.finish())?;
    }
    Ok(registry.finish())
}
