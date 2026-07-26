//! Conditional physical species proof graph and independent closure pair.
//!
//! The current repository supplies independently projected physical
//! coordinates but no species-forming law. The executable production result is
//! therefore a structured refusal after root validation. Synthetic tests can
//! exercise full closure, but agreement cannot construct the dormant registry
//! authority or enter conditioned support.

mod model;
mod producer;
mod repository_roots;
mod vocabulary;
mod watchdog;

#[cfg(test)]
mod tests;

use crate::canonical::stellar_birth_structure::stellar_birth_structure_schema;
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use model::{
    AuthorityEffect, CheckerPairBinding, PhysicalRegistryInput, PhysicalRegistryRefusal,
    PhysicalRegistryRefusalCode, PhysicalRegistryResourceContract, ReceiptBinding,
    StructureAuthorityBinding, VerifiedPhysicalSpeciesRegistry, PRODUCER_ID, PROOF_GRAPH_SCHEMA_ID,
    REGISTRY_SCHEMA_ID, WATCHDOG_ID,
};
use repository_roots::{
    decide_repository_roots, verify_current_refusal, verify_projection, verify_projection_receipt,
    RepositoryRootDecision, RepositoryRootProjectionReceipt, RepositoryRootRefusal,
    RepositoryRootRefusalCode,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RepositoryRootAdmissionCensusRow {
    pub(super) identity_sha256: [u8; 32],
    pub(super) tier_id: &'static str,
    pub(super) provenance_tag: &'static str,
    pub(super) route_id: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RepositoryPhysicalRegistryFrontier {
    pub(super) registry_schema_id: &'static str,
    pub(super) proof_graph_schema_id: &'static str,
    pub(super) root_receipt_schema_id: &'static str,
    pub(super) root_claim_id: &'static str,
    pub(super) root_input_sha256: [u8; 32],
    pub(super) root_result_sha256: [u8; 32],
    pub(super) root_producer_result_sha256: [u8; 32],
    pub(super) root_watchdog_result_sha256: [u8; 32],
    pub(super) root_producer_resource_sha256: [u8; 32],
    pub(super) root_watchdog_resource_sha256: [u8; 32],
    pub(super) root_canary_suite_id: &'static str,
    pub(super) root_canary_sha256: [u8; 32],
    pub(super) root_decision_id: &'static str,
    pub(super) root_receipt_sha256: [u8; 32],
    pub(super) root_producer_id: &'static str,
    pub(super) root_watchdog_id: &'static str,
    pub(super) scalar_coordinate_count: u32,
    pub(super) membership_neutral_mass_projection_count: u32,
    pub(super) admitted_root_count: u32,
    pub(super) root_admission_census: Vec<RepositoryRootAdmissionCensusRow>,
    pub(super) membership_authority: bool,
    pub(super) vocabulary_receipt_schema_id: String,
    pub(super) vocabulary_claim_id: String,
    pub(super) vocabulary_producer_id: String,
    pub(super) vocabulary_watchdog_id: String,
    pub(super) vocabulary_descriptor_role_identities: Vec<[u8; 32]>,
    pub(super) vocabulary_relation_target_identities: Vec<[u8; 32]>,
    pub(super) vocabulary_constraint_law_identities: Vec<[u8; 32]>,
    pub(super) vocabulary_root_count: u32,
    pub(super) vocabulary_descriptor_role_count: u32,
    pub(super) vocabulary_relation_target_count: u32,
    pub(super) vocabulary_constraint_law_count: u32,
    pub(super) vocabulary_current_input_partition_complete: bool,
    pub(super) vocabulary_global_coverage: bool,
    pub(super) vocabulary_membership_authority: bool,
    pub(super) vocabulary_producer_result_sha256: [u8; 32],
    pub(super) vocabulary_watchdog_result_sha256: [u8; 32],
    pub(super) vocabulary_producer_resource_sha256: [u8; 32],
    pub(super) vocabulary_watchdog_resource_sha256: [u8; 32],
    pub(super) vocabulary_receipt_sha256: [u8; 32],
    pub(super) registry_refusal_code: &'static str,
    pub(super) registry_member_count: u32,
    pub(super) registry_coverage_claim: bool,
    pub(super) registry_authority_effect: &'static str,
    pub(super) open_obligations: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RepositoryPhysicalRegistryFrontierError {
    pub(super) code: &'static str,
    root_refusal: Option<Box<RepositoryRootRefusal>>,
}

impl RepositoryPhysicalRegistryFrontierError {
    fn from_code(code: &'static str) -> Self {
        Self {
            code,
            root_refusal: None,
        }
    }

    fn from_input_error(error: RepositoryInputError) -> Self {
        let code = error.physical_code().id();
        let root_refusal = match error {
            RepositoryInputError::Physical(_) => None,
            RepositoryInputError::Root(refusal) => Some(refusal),
        };
        Self { code, root_refusal }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryInputError {
    Physical(PhysicalRegistryRefusalCode),
    Root(Box<RepositoryRootRefusal>),
}

impl RepositoryInputError {
    fn physical_code(&self) -> PhysicalRegistryRefusalCode {
        match self {
            Self::Physical(code) => *code,
            Self::Root(refusal)
                if refusal.code == RepositoryRootRefusalCode::CheckerDisagreement =>
            {
                PhysicalRegistryRefusalCode::FloorCoordinateProjectionCheckerDisagreement
            }
            Self::Root(_) => PhysicalRegistryRefusalCode::FloorCoordinateProjectionInvalid,
        }
    }
}

fn inspect_physical_registry(
    input: &PhysicalRegistryInput,
) -> Result<VerifiedPhysicalSpeciesRegistry, PhysicalRegistryRefusal> {
    let producer_resource_contract_sha256 = producer::resource_contract_sha256();
    let watchdog_resource_contract_sha256 = watchdog::resource_contract_sha256();
    if producer_resource_contract_sha256 != watchdog_resource_contract_sha256 {
        return Err(PhysicalRegistryRefusal::from_code(
            PhysicalRegistryRefusalCode::CheckerDisagreement,
        ));
    }
    let produced = producer::validate_and_encode(input);
    let watched = watchdog::validate_and_encode(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.members == watched.members
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            if produced.resource_contract_sha256 != producer_resource_contract_sha256
                || watched.resource_contract_sha256 != watchdog_resource_contract_sha256
            {
                return Err(PhysicalRegistryRefusal::from_code(
                    PhysicalRegistryRefusalCode::CheckerDisagreement,
                ));
            }
            Ok(VerifiedPhysicalSpeciesRegistry {
                members: produced.members,
                canonical_bytes: produced.canonical_bytes,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                authority_effect: AuthorityEffect::None,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => {
            Err(PhysicalRegistryRefusal::from_code(produced))
        }
        _ => Err(PhysicalRegistryRefusal::from_code(
            PhysicalRegistryRefusalCode::CheckerDisagreement,
        )),
    }
}

fn repository_input() -> Result<PhysicalRegistryInput, PhysicalRegistryRefusalCode> {
    repository_input_and_projection()
        .map(|(input, _)| input)
        .map_err(|error| error.physical_code())
}

fn repository_input_and_projection(
) -> Result<(PhysicalRegistryInput, RepositoryRootProjectionReceipt), RepositoryInputError> {
    let floor = sealed_physical_floor_authority_binding().map_err(|_| {
        RepositoryInputError::Physical(PhysicalRegistryRefusalCode::FloorBindingMismatch)
    })?;
    let root_projection = match decide_repository_roots() {
        RepositoryRootDecision::Projected(projection) => projection,
        RepositoryRootDecision::Refused(refusal) => {
            if !verify_current_refusal(&refusal) {
                return Err(RepositoryInputError::Physical(
                    PhysicalRegistryRefusalCode::FloorCoordinateProjectionInvalid,
                ));
            }
            return Err(RepositoryInputError::Root(Box::new(refusal)));
        }
    };
    if root_projection.membership_authority
        || root_projection.receipt.membership_authority
        || !verify_projection(&root_projection)
    {
        return Err(RepositoryInputError::Physical(
            PhysicalRegistryRefusalCode::FloorCoordinateProjectionInvalid,
        ));
    }
    let vocabulary_binding = vocabulary::derive_binding(&root_projection.admitted_artifacts)
        .map_err(|()| {
            RepositoryInputError::Physical(
                PhysicalRegistryRefusalCode::PhysicalVocabularyBindingMismatch,
            )
        })?;
    if !vocabulary_binding.current_input_partition_complete
        || vocabulary_binding.global_physical_vocabulary_coverage
        || vocabulary_binding.membership_authority
    {
        return Err(RepositoryInputError::Physical(
            PhysicalRegistryRefusalCode::PhysicalVocabularyBindingMismatch,
        ));
    }
    let projection_receipt = root_projection.receipt.clone();
    let structure = stellar_birth_structure_schema().map_err(|_| {
        RepositoryInputError::Physical(PhysicalRegistryRefusalCode::StructureBindingMismatch)
    })?;
    Ok((
        PhysicalRegistryInput {
            schema_id: REGISTRY_SCHEMA_ID.to_owned(),
            proof_graph_schema_id: PROOF_GRAPH_SCHEMA_ID.to_owned(),
            floor_binding: ReceiptBinding {
                schema_id: floor.schema_id().as_str().to_owned(),
                digest_sha256: floor.digest(),
            },
            structure_binding: StructureAuthorityBinding {
                structure_schema_id: structure.schema_id.to_owned(),
                species_registry_schema_id: structure.species_registry.schema_id.to_owned(),
                stellar_state_schema_id: structure.stellar_state.schema_id.to_owned(),
                state_coordinate_registry_schema_id: structure
                    .stellar_state
                    .state_coordinate_registry
                    .schema_id
                    .to_owned(),
                interaction_sector_registry_schema_id: structure
                    .stellar_state
                    .interaction_sector_registry
                    .schema_id
                    .to_owned(),
                physical_regime_registry_schema_id: structure
                    .stellar_state
                    .physical_regime_registry
                    .schema_id
                    .to_owned(),
            },
            checker_pair: CheckerPairBinding {
                producer_id: PRODUCER_ID.to_owned(),
                watchdog_id: WATCHDOG_ID.to_owned(),
            },
            resources: PhysicalRegistryResourceContract::PRODUCTION,
            vocabulary_binding,
            admitted_artifacts: root_projection.admitted_artifacts,
            declared_members: Vec::new(),
        },
        projection_receipt,
    ))
}

fn resolve_repository_physical_species_registry(
) -> Result<VerifiedPhysicalSpeciesRegistry, PhysicalRegistryRefusal> {
    repository_input()
        .map_err(PhysicalRegistryRefusal::from_code)
        .and_then(|input| inspect_physical_registry(&input))
}

const fn is_repository_scientific_refusal(code: PhysicalRegistryRefusalCode) -> bool {
    matches!(
        code,
        PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRules
            | PhysicalRegistryRefusalCode::PhysicalVocabularyCoverageIncomplete
    )
}

fn root_admission_census(
    artifacts: &[model::AdmittedArtifact],
) -> Result<Vec<RepositoryRootAdmissionCensusRow>, RepositoryPhysicalRegistryFrontierError> {
    let mut census = artifacts
        .iter()
        .map(|artifact| {
            let provenance_tag = artifact.admission.provenance.bracket_tag().ok_or_else(|| {
                RepositoryPhysicalRegistryFrontierError::from_code("root_provenance_not_canonical")
            })?;
            let route_id = match &artifact.admission.route {
                model::AdmissionRoute::Derived(_) => "derived",
                model::AdmissionRoute::Irreducible(_) => "irreducible",
                model::AdmissionRoute::EvidenceCustodyOnly { .. } => "evidence_custody_only",
            };
            Ok(RepositoryRootAdmissionCensusRow {
                identity_sha256: artifact.claimed_identity.0,
                tier_id: artifact.admission.tier.id(),
                provenance_tag,
                route_id,
            })
        })
        .collect::<Result<Vec<_>, RepositoryPhysicalRegistryFrontierError>>()?;
    census.sort_unstable();
    if census
        .windows(2)
        .any(|pair| pair[0].identity_sha256 == pair[1].identity_sha256)
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "root_admission_census_invalid",
        ));
    }
    Ok(census)
}

pub(super) fn repository_physical_registry_frontier(
) -> Result<RepositoryPhysicalRegistryFrontier, RepositoryPhysicalRegistryFrontierError> {
    let (input, receipt) = repository_input_and_projection()
        .map_err(RepositoryPhysicalRegistryFrontierError::from_input_error)?;
    let admitted_root_count = u32::try_from(input.admitted_artifacts.len())
        .map_err(|_| RepositoryPhysicalRegistryFrontierError::from_code("root_count_overflow"))?;
    let vocabulary_descriptor_role_count =
        u32::try_from(input.vocabulary_binding.descriptor_role_identities.len()).map_err(|_| {
            RepositoryPhysicalRegistryFrontierError::from_code("vocabulary_count_overflow")
        })?;
    let vocabulary_relation_target_count =
        u32::try_from(input.vocabulary_binding.relation_target_identities.len()).map_err(|_| {
            RepositoryPhysicalRegistryFrontierError::from_code("vocabulary_count_overflow")
        })?;
    let vocabulary_constraint_law_count =
        u32::try_from(input.vocabulary_binding.constraint_law_identities.len()).map_err(|_| {
            RepositoryPhysicalRegistryFrontierError::from_code("vocabulary_count_overflow")
        })?;
    if !verify_projection_receipt(&receipt)
        || admitted_root_count
            != receipt
                .scalar_coordinate_count
                .checked_add(receipt.mass_projection_count)
                .ok_or_else(|| {
                    RepositoryPhysicalRegistryFrontierError::from_code("root_count_overflow")
                })?
        || input.vocabulary_binding.root_count != admitted_root_count
        || vocabulary_relation_target_count != admitted_root_count
        || !input.vocabulary_binding.current_input_partition_complete
        || input.vocabulary_binding.global_physical_vocabulary_coverage
        || input.vocabulary_binding.membership_authority
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "root_projection_invariant_violation",
        ));
    }
    let refusal = match inspect_physical_registry(&input) {
        Err(refusal) if is_repository_scientific_refusal(refusal.code) => refusal,
        Err(refusal) => {
            return Err(RepositoryPhysicalRegistryFrontierError::from_code(
                refusal.code.id(),
            ));
        }
        Ok(_) => {
            return Err(RepositoryPhysicalRegistryFrontierError::from_code(
                "repository_registry_unexpectedly_admitted",
            ));
        }
    };
    let root_admission_census = root_admission_census(&input.admitted_artifacts)?;
    if root_admission_census.len() != usize::try_from(admitted_root_count).unwrap_or(usize::MAX)
        || root_admission_census
            .windows(2)
            .any(|pair| pair[0].identity_sha256 == pair[1].identity_sha256)
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "root_admission_census_invalid",
        ));
    }
    Ok(RepositoryPhysicalRegistryFrontier {
        registry_schema_id: REGISTRY_SCHEMA_ID,
        proof_graph_schema_id: PROOF_GRAPH_SCHEMA_ID,
        root_receipt_schema_id: receipt.schema_id,
        root_claim_id: receipt.claim_id,
        root_input_sha256: receipt.input_sha256,
        root_result_sha256: receipt.result_sha256,
        root_producer_result_sha256: receipt.producer_result_sha256,
        root_watchdog_result_sha256: receipt.watchdog_result_sha256,
        root_producer_resource_sha256: receipt.producer_resource_contract_sha256,
        root_watchdog_resource_sha256: receipt.watchdog_resource_contract_sha256,
        root_canary_suite_id: receipt.canary_suite_id,
        root_canary_sha256: receipt.canary_sha256,
        root_decision_id: receipt.decision_id,
        root_receipt_sha256: receipt.receipt_sha256,
        root_producer_id: receipt.producer_id,
        root_watchdog_id: receipt.watchdog_id,
        scalar_coordinate_count: receipt.scalar_coordinate_count,
        membership_neutral_mass_projection_count: receipt.mass_projection_count,
        admitted_root_count,
        root_admission_census,
        membership_authority: receipt.membership_authority,
        vocabulary_receipt_schema_id: input.vocabulary_binding.schema_id,
        vocabulary_claim_id: input.vocabulary_binding.claim_id,
        vocabulary_producer_id: input.vocabulary_binding.producer_id,
        vocabulary_watchdog_id: input.vocabulary_binding.watchdog_id,
        vocabulary_descriptor_role_identities: input
            .vocabulary_binding
            .descriptor_role_identities
            .into_iter()
            .map(|identity| identity.0)
            .collect(),
        vocabulary_relation_target_identities: input
            .vocabulary_binding
            .relation_target_identities
            .into_iter()
            .map(|identity| identity.0)
            .collect(),
        vocabulary_constraint_law_identities: input
            .vocabulary_binding
            .constraint_law_identities
            .into_iter()
            .map(|identity| identity.0)
            .collect(),
        vocabulary_root_count: input.vocabulary_binding.root_count,
        vocabulary_descriptor_role_count,
        vocabulary_relation_target_count,
        vocabulary_constraint_law_count,
        vocabulary_current_input_partition_complete: input
            .vocabulary_binding
            .current_input_partition_complete,
        vocabulary_global_coverage: input.vocabulary_binding.global_physical_vocabulary_coverage,
        vocabulary_membership_authority: input.vocabulary_binding.membership_authority,
        vocabulary_producer_result_sha256: input.vocabulary_binding.producer_result_sha256,
        vocabulary_watchdog_result_sha256: input.vocabulary_binding.watchdog_result_sha256,
        vocabulary_producer_resource_sha256: input
            .vocabulary_binding
            .producer_resource_contract_sha256,
        vocabulary_watchdog_resource_sha256: input
            .vocabulary_binding
            .watchdog_resource_contract_sha256,
        vocabulary_receipt_sha256: input.vocabulary_binding.receipt_sha256,
        registry_refusal_code: refusal.code.id(),
        registry_member_count: refusal.member_count,
        registry_coverage_claim: refusal.coverage_claim,
        registry_authority_effect: refusal.authority_effect.id(),
        open_obligations: refusal.open_obligations,
    })
}
