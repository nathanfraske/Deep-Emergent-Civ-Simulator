//! Consistency checker for the live species derivation refusal.
//!
//! This path re-queries the physical registry and checks faithful propagation.
//! It does not certify completeness of the registry's diagnostic vocabulary.

use super::{AnalysisBuildError, AnalysisProgress, SpeciesDerivationAnalysis};
use crate::canonical::stellar_birth_species::{
    law_premise::repository_derived_relation_premise_frontier,
    physical_registry::repository_physical_registry_frontier,
};
use crate::canonical::{
    floor_magnitudes::AuditedFloorView, stellar_birth_structure::stellar_birth_structure_schema,
};
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use std::collections::BTreeSet;

const CHECKED_FLOOR_ANCHOR_ID: &str = "fundamental.m_e";
const CHECKED_FLOOR_ANCHOR_SYMBOL: &str = "m_e";
const CHECKED_FLOOR_ANCHOR_ROLE: &str = "mass_coordinate_anchor_only";
const CHECKED_REDUCER_LAW_ID: &str = "candidate.composition_weighted_particle_mass";
const CHECKED_FRONTIER_SOURCE_ID: &str = "repository_physical_registry_live_refusal";
const CHECKED_FRONTIER_SCOPE_ID: &str = "first_executable_refusal_only";
const CHECKED_LIVE_ATTEMPT_ID: &str =
    "stellar_birth.species_derivation.live_physical_registry_refusal";

pub(super) fn validate_analysis(
    analysis: &SpeciesDerivationAnalysis,
) -> Result<(), AnalysisBuildError> {
    let binding = sealed_physical_floor_authority_binding()
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let structure = stellar_birth_structure_schema()?;
    let expected_physical_frontier = repository_physical_registry_frontier()
        .map_err(|error| AnalysisBuildError::PhysicalRegistryFrontier(error.code.to_owned()))?;
    let expected_law_premise_frontier = repository_derived_relation_premise_frontier()
        .map_err(|error| AnalysisBuildError::LawPremiseFrontier(error.code.to_owned()))?;
    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let floor_view = AuditedFloorView::from_floor(&floor)
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let expected_mass_anchor = floor_view.magnitudes.electron_mass;
    let expected_schema_ids = (
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
    let found_schema_ids = (
        analysis.structure_schema_id,
        analysis.species_registry_schema_id,
        analysis.stellar_state_schema_id,
        analysis.state_coordinate_registry_schema_id,
        analysis.interaction_sector_registry_schema_id,
        analysis.physical_regime_registry_schema_id,
    );
    if found_schema_ids != expected_schema_ids {
        return invariant("schema binding differs from the canonical structure");
    }
    if analysis.floor_binding_schema_id != binding.schema_id().as_str()
        || analysis.floor_binding_sha256 != binding.digest_hex()
    {
        return invariant("physical-floor binding differs from the sealed authority");
    }
    if analysis.reducer_law_id != CHECKED_REDUCER_LAW_ID
        || analysis.floor_mass_anchor.id != CHECKED_FLOOR_ANCHOR_ID
        || analysis.floor_mass_anchor.symbol != CHECKED_FLOOR_ANCHOR_SYMBOL
        || analysis.floor_mass_anchor.bits != expected_mass_anchor.bits()
        || analysis.floor_mass_anchor.scale_bits != expected_mass_anchor.scale_bits()
        || analysis.floor_mass_anchor.role != CHECKED_FLOOR_ANCHOR_ROLE
        || analysis.floor_mass_anchor.membership_authority
    {
        return invariant("floor mass anchor gained species authority or changed identity");
    }
    if analysis.physical_registry_frontier != expected_physical_frontier {
        return invariant("physical-registry frontier differs from its live authority result");
    }
    if analysis.law_premise_frontier != expected_law_premise_frontier {
        return invariant("law-premise frontier differs from its live authority result");
    }
    if !analysis.law_premise_frontier.requested_premise_coverage
        || analysis.law_premise_frontier.species_membership_authority
        || analysis
            .law_premise_frontier
            .global_physical_premise_coverage
        || analysis.law_premise_frontier.authority_effect != "none"
    {
        return invariant("law premise gained unsupported scope or authority");
    }
    if analysis.frontier_source_id != CHECKED_FRONTIER_SOURCE_ID
        || analysis.frontier_scope_id != CHECKED_FRONTIER_SCOPE_ID
        || analysis.frontier_completeness_claim
    {
        return invariant("diagnostic frontier gained unsupported scope or completeness");
    }
    if analysis.candidate_member_count
        != usize::try_from(expected_physical_frontier.registry_member_count).map_err(|_| {
            AnalysisBuildError::InternalInvariant("member count overflow".to_owned())
        })?
        || analysis.candidate_member_count != 0
        || analysis.verified_support_member_count != 0
        || analysis.species_support_value_payload_present
        || analysis.residual_slot_claim
        || analysis.derive_first_status != AnalysisProgress::OpenDependencies
        || analysis.buckingham_pi_status
            != AnalysisProgress::DimensionOnlyRelationNotPhysicalClosure
        || analysis.gap_law_status != AnalysisProgress::NotReached
        || analysis.chaos_protocol_status != AnalysisProgress::NotReached
        || analysis.residual_law_status != AnalysisProgress::NotReached
        || analysis.unique_residual_slot_status != AnalysisProgress::NotClaimed
    {
        return invariant("non-admitting derivation state changed");
    }
    if expected_physical_frontier.open_obligations.is_empty() {
        return invariant("live physical-registry refusal has no diagnostic obligations");
    }
    let live_proofs = expected_physical_frontier
        .open_obligations
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if live_proofs.len() != expected_physical_frontier.open_obligations.len() {
        return invariant("live physical-registry obligations are duplicated");
    }
    let [attempt] = analysis.attempts.as_slice() else {
        return invariant("diagnostic frontier is not the single live executable refusal");
    };
    if attempt.id != CHECKED_LIVE_ATTEMPT_ID
        || attempt.status != AnalysisProgress::BlockedOpenProofs
        || attempt.input_ids.as_slice()
            != [
                expected_physical_frontier.root_claim_id,
                expected_physical_frontier.vocabulary_claim_id.as_str(),
            ]
        || !matches_strings(
            &attempt.open_proof_ids,
            &expected_physical_frontier.open_obligations,
        )
    {
        return invariant("diagnostic attempt differs from the live physical-registry refusal");
    }
    let declared_proofs = analysis
        .open_proof_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if declared_proofs != live_proofs
        || declared_proofs.len() != analysis.open_proof_ids.len()
        || analysis
            .open_proof_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return invariant("declared proof frontier differs from the live refusal");
    }
    Ok(())
}

fn matches_strings(found: &[String], expected: &[&str]) -> bool {
    found.len() == expected.len()
        && found
            .iter()
            .zip(expected)
            .all(|(found, expected)| found == expected)
}

fn invariant<T>(detail: &str) -> Result<T, AnalysisBuildError> {
    Err(AnalysisBuildError::InternalInvariant(detail.to_owned()))
}
