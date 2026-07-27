//! Conditional physical species proof graph and independent closure pair.
//!
//! The current repository supplies independently projected physical
//! coordinates plus one claim-local, irreducibly admitted unbroken abelian
//! profile. The executable production result closes that one local member while
//! keeping global vocabulary coverage, complete species membership, and
//! conditioned support false.

mod model;
mod primitive_profile;
mod producer;
mod repository_roots;
mod vocabulary;
mod watchdog;

#[cfg(test)]
mod tests;

use crate::canonical::stellar_birth_structure::stellar_birth_structure_schema;
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use model::{
    AdmissionCapabilityKind, AuthorityEffect, CheckerPairBinding, PhysicalRegistryInput,
    PhysicalRegistryRefusal, PhysicalRegistryRefusalCode, PhysicalRegistryResourceContract,
    ReceiptBinding, StructureAuthorityBinding, VerifiedPhysicalSpeciesRegistry, PRODUCER_ID,
    PROOF_GRAPH_SCHEMA_ID, REGISTRY_SCHEMA_ID, WATCHDOG_ID,
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
pub(super) struct RepositoryPrimitiveProfileFrontier {
    pub(super) receipt_schema_id: &'static str,
    pub(super) claim_id: &'static str,
    pub(super) profile_id: &'static str,
    pub(super) theory_class_id: &'static str,
    pub(super) residual_slot_id: &'static str,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) member_id: &'static str,
    pub(super) symmetry_id: &'static str,
    pub(super) field_id: &'static str,
    pub(super) operator_id: &'static str,
    pub(super) state_id: &'static str,
    pub(super) sector_id: &'static str,
    pub(super) validity_id: &'static str,
    pub(super) helicity_id: &'static str,
    pub(super) statistics_id: &'static str,
    pub(super) charge_id: &'static str,
    pub(super) current_id: &'static str,
    pub(super) stability_id: &'static str,
    pub(super) transition_id: &'static str,
    pub(super) excluded_term_id: &'static str,
    pub(super) artifact_count: u32,
    pub(super) admission_census: Vec<RepositoryRootAdmissionCensusRow>,
    pub(super) member_sha256: [u8; 32],
    pub(super) pair_receipt_sha256: [u8; 32],
    pub(super) symmetry_basis_element_count: u32,
    pub(super) symmetry_excluded_operator_count: u32,
    pub(super) symmetry_action_binding_sha256: [u8; 32],
    pub(super) derivation_catalog_sha256: [u8; 32],
    pub(super) repository_catalog_sha256: [u8; 32],
    pub(super) protocol_producer_result_sha256: [u8; 32],
    pub(super) protocol_watchdog_result_sha256: [u8; 32],
    pub(super) derivation_coverage_capability_sha256: [u8; 32],
    pub(super) irreducible_protocol_capability_sha256: [u8; 32],
    pub(super) derive_first_status_id: &'static str,
    pub(super) buckingham_pi_status_id: &'static str,
    pub(super) gap_law_status_id: &'static str,
    pub(super) chaos_protocol_status_id: &'static str,
    pub(super) residual_law_status_id: &'static str,
    pub(super) residual_slot_status_id: &'static str,
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
    pub(super) admitted_artifact_count: u32,
    pub(super) root_admission_census: Vec<RepositoryRootAdmissionCensusRow>,
    pub(super) membership_authority: bool,
    pub(super) primitive_profile: RepositoryPrimitiveProfileFrontier,
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
        .map(|(input, _, _)| input)
        .map_err(|error| error.physical_code())
}

fn repository_input_and_projection() -> Result<
    (
        PhysicalRegistryInput,
        RepositoryRootProjectionReceipt,
        primitive_profile::PrimitiveProfileReceipt,
    ),
    RepositoryInputError,
> {
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
    let primitive_profile = primitive_profile::project_admitted_profile().map_err(|_| {
        RepositoryInputError::Physical(
            PhysicalRegistryRefusalCode::PrimitiveProfileProjectionInvalid,
        )
    })?;
    let primitive_profile_receipt = primitive_profile.receipt.clone();
    let declared_members = vec![primitive_profile.member];
    let mut admitted_artifacts = root_projection.admitted_artifacts;
    admitted_artifacts.extend(primitive_profile.admitted_artifacts);
    let vocabulary_binding = vocabulary::derive_binding(&admitted_artifacts).map_err(|()| {
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
            admitted_artifacts,
            declared_members,
        },
        projection_receipt,
        primitive_profile_receipt,
    ))
}

fn resolve_repository_physical_species_registry(
) -> Result<VerifiedPhysicalSpeciesRegistry, PhysicalRegistryRefusal> {
    repository_input()
        .map_err(PhysicalRegistryRefusal::from_code)
        .and_then(|input| inspect_physical_registry(&input))
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
    let (input, receipt, primitive_profile_receipt) = repository_input_and_projection()
        .map_err(RepositoryPhysicalRegistryFrontierError::from_input_error)?;
    let admitted_artifact_count = u32::try_from(input.admitted_artifacts.len())
        .map_err(|_| RepositoryPhysicalRegistryFrontierError::from_code("root_count_overflow"))?;
    let admitted_root_count = u32::try_from(
        input
            .admitted_artifacts
            .iter()
            .filter(|artifact| {
                artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
            })
            .count(),
    )
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
        || input.vocabulary_binding.root_count != admitted_artifact_count
        || vocabulary_relation_target_count != admitted_artifact_count
        || !input.vocabulary_binding.current_input_partition_complete
        || input.vocabulary_binding.global_physical_vocabulary_coverage
        || input.vocabulary_binding.membership_authority
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "root_projection_invariant_violation",
        ));
    }
    let registry = inspect_physical_registry(&input)
        .map_err(|refusal| RepositoryPhysicalRegistryFrontierError::from_code(refusal.code.id()))?;
    let registry_member_count = u32::try_from(registry.members.len())
        .map_err(|_| RepositoryPhysicalRegistryFrontierError::from_code("member_count_overflow"))?;
    if registry_member_count != 1
        || registry.members[0].identity != primitive_profile_receipt.member
        || registry.authority_effect != AuthorityEffect::None
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "repository_registry_projection_mismatch",
        ));
    }
    let repository_root_artifacts = input
        .admitted_artifacts
        .iter()
        .filter(|artifact| {
            artifact.admission_capability_kind() == AdmissionCapabilityKind::RepositoryRoot
        })
        .cloned()
        .collect::<Vec<_>>();
    let repository_root_admission_census = root_admission_census(&repository_root_artifacts)?;
    let primitive_profile_artifacts = input
        .admitted_artifacts
        .iter()
        .filter(|artifact| {
            artifact.admission_capability_kind() == AdmissionCapabilityKind::PrimitiveProfile
        })
        .cloned()
        .collect::<Vec<_>>();
    let primitive_profile_admission_census = root_admission_census(&primitive_profile_artifacts)?;
    if repository_root_admission_census.len()
        != usize::try_from(admitted_root_count).unwrap_or(usize::MAX)
        || repository_root_admission_census
            .windows(2)
            .any(|pair| pair[0].identity_sha256 == pair[1].identity_sha256)
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "root_admission_census_invalid",
        ));
    }
    if primitive_profile_admission_census.len()
        != usize::try_from(primitive_profile_receipt.artifact_count).unwrap_or(usize::MAX)
        || primitive_profile_admission_census
            .iter()
            .filter(|row| {
                row.tier_id == "residue"
                    && row.provenance_tag == "[A]"
                    && row.route_id == "irreducible"
            })
            .count()
            != 1
        || primitive_profile_admission_census
            .iter()
            .filter(|row| {
                row.tier_id == "residue" && row.provenance_tag == "[D]" && row.route_id == "derived"
            })
            .count()
            != primitive_profile_admission_census.len().saturating_sub(1)
    {
        return Err(RepositoryPhysicalRegistryFrontierError::from_code(
            "primitive_profile_admission_census_invalid",
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
        admitted_artifact_count,
        root_admission_census: repository_root_admission_census,
        membership_authority: receipt.membership_authority,
        primitive_profile: RepositoryPrimitiveProfileFrontier {
            receipt_schema_id: primitive_profile_receipt.schema_id,
            claim_id: primitive_profile_receipt.claim_id,
            profile_id: primitive_profile_receipt.profile_id,
            theory_class_id: primitive_profile_receipt.theory_class_id,
            residual_slot_id: primitive_profile_receipt.residual_slot_id,
            producer_id: primitive_profile_receipt.producer_id,
            watchdog_id: primitive_profile_receipt.watchdog_id,
            member_id: primitive_profile::MEMBER_ID,
            symmetry_id: primitive_profile::SYMMETRY_ID,
            field_id: primitive_profile::FIELD_ID,
            operator_id: primitive_profile::OPERATOR_ID,
            state_id: primitive_profile::STATE_ID,
            sector_id: primitive_profile::SECTOR_ID,
            validity_id: primitive_profile::VALIDITY_ID,
            helicity_id: primitive_profile::HELICITY_ID,
            statistics_id: primitive_profile::STATISTICS_ID,
            charge_id: primitive_profile::CHARGE_ID,
            current_id: primitive_profile::CURRENT_ID,
            stability_id: primitive_profile::STABILITY_ID,
            transition_id: primitive_profile::TRANSITION_ID,
            excluded_term_id: primitive_profile::EXCLUDED_TERM_ID,
            artifact_count: primitive_profile_receipt.artifact_count,
            admission_census: primitive_profile_admission_census,
            member_sha256: primitive_profile_receipt.member.0,
            pair_receipt_sha256: primitive_profile_receipt.pair_receipt_sha256,
            symmetry_basis_element_count: primitive_profile_receipt.symmetry_basis_element_count,
            symmetry_excluded_operator_count: primitive_profile_receipt
                .symmetry_excluded_operator_count,
            symmetry_action_binding_sha256: primitive_profile_receipt
                .symmetry_action_binding_sha256,
            derivation_catalog_sha256: primitive_profile_receipt.derivation_catalog_sha256,
            repository_catalog_sha256: primitive_profile_receipt.repository_catalog_sha256,
            protocol_producer_result_sha256: primitive_profile_receipt
                .protocol_producer_result_sha256,
            protocol_watchdog_result_sha256: primitive_profile_receipt
                .protocol_watchdog_result_sha256,
            derivation_coverage_capability_sha256: primitive_profile_receipt
                .derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256: primitive_profile_receipt
                .irreducible_protocol_capability_sha256,
            derive_first_status_id: primitive_profile_receipt.protocol.derive_first_status_id,
            buckingham_pi_status_id: primitive_profile_receipt.protocol.buckingham_pi_status_id,
            gap_law_status_id: primitive_profile_receipt.protocol.gap_law_status_id,
            chaos_protocol_status_id: primitive_profile_receipt.protocol.chaos_protocol_status_id,
            residual_law_status_id: primitive_profile_receipt.protocol.residual_law_status_id,
            residual_slot_status_id: primitive_profile_receipt.protocol.residual_slot_status_id,
        },
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
        registry_refusal_code: "none",
        registry_member_count,
        registry_coverage_claim: false,
        registry_authority_effect: registry.authority_effect.id(),
        open_obligations: vec![
            "complete_global_physical_vocabulary_coverage",
            "complete_registry_closure_domain",
            "conditioned_species_support",
            "species_mass_uncertainty_transport",
            "charged_matter_profile",
        ],
    })
}
