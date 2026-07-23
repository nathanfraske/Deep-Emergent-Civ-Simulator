//! Independent semantic checker for the species derivation frontier.

use super::{AnalysisBuildError, AnalysisProgress, SpeciesDerivationAnalysis};
use crate::canonical::{
    floor_magnitudes::AuditedFloorView, stellar_birth_structure::stellar_birth_structure_schema,
};
use civsim_units::physics_floor::sealed_physical_floor_authority_binding;
use std::collections::BTreeSet;

const CHECKED_FLOOR_ANCHOR_ID: &str = "fundamental.m_e";
const CHECKED_FLOOR_ANCHOR_SYMBOL: &str = "m_e";
const CHECKED_FLOOR_ANCHOR_ROLE: &str = "mass_coordinate_anchor_only";
const CHECKED_REDUCER_LAW_ID: &str = "candidate.composition_weighted_particle_mass";

struct CheckedAttempt {
    id: &'static str,
    input_ids: &'static [&'static str],
    open_proof_ids: &'static [&'static str],
}

// This semantic specification is intentionally independent of producer.rs.
// It states the exact non-admitting frontier in reviewable form instead of
// importing a producer list or merely checking the producer's own checksum.
const CHECKED_ATTEMPTS: &[CheckedAttempt] = &[
    CheckedAttempt {
        id: "stellar_birth.species_derivation.complete_registry",
        input_ids: &["fundamental.alpha", "fundamental.G", "fundamental.m_e"],
        open_proof_ids: &[
            "canonical_species_state_descriptor_checker_unavailable",
            "charge_state_sector_validity_proof_unavailable",
            "physical_species_membership_derivation_unavailable",
            "rest_mass_dimension_and_ancestry_proof_unavailable",
        ],
    },
    CheckedAttempt {
        id: "stellar_birth.species_derivation.complete_conditioned_support",
        input_ids: &[
            "stellar_birth.composition.species_number_fraction_field",
            "stellar_birth.species_state_registry",
        ],
        open_proof_ids: &[
            "conditioned_zero_or_sparse_support_semantics_unavailable",
            "finite_exact_resource_domain_unavailable",
            "joint_measure_support_binding_unavailable",
        ],
    },
    CheckedAttempt {
        id: "stellar_birth.species_derivation.exact_mean_mass_projection",
        input_ids: &["stellar_birth.conditioned_species_state_support"],
        open_proof_ids: &[
            "finite_exact_resource_domain_unavailable",
            "integer_projection_schema_unavailable",
            "joint_measure_support_binding_unavailable",
        ],
    },
];

pub(super) fn validate_analysis(
    analysis: &SpeciesDerivationAnalysis,
) -> Result<(), AnalysisBuildError> {
    let binding = sealed_physical_floor_authority_binding()
        .map_err(|error| AnalysisBuildError::FloorAuthority(error.to_string()))?;
    let structure = stellar_birth_structure_schema()?;
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
    if analysis.candidate_member_count != 0
        || analysis.verified_support_member_count != 0
        || analysis.value_payload_present
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
    if analysis.attempts.is_empty() || analysis.open_proof_ids.is_empty() {
        return invariant("open derivation frontier became empty");
    }
    let attempt_ids = analysis
        .attempts
        .iter()
        .map(|attempt| attempt.id)
        .collect::<BTreeSet<_>>();
    if attempt_ids.len() != analysis.attempts.len()
        || analysis.attempts.iter().any(|attempt| {
            attempt.status != AnalysisProgress::BlockedOpenProofs
                || attempt.input_ids.is_empty()
                || attempt.open_proof_ids.is_empty()
        })
    {
        return invariant("derivation attempts are duplicated or no longer fail closed");
    }
    let proof_union = analysis
        .attempts
        .iter()
        .flat_map(|attempt| attempt.open_proof_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let declared_proofs = analysis
        .open_proof_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if proof_union != declared_proofs || declared_proofs.len() != analysis.open_proof_ids.len() {
        return invariant("open-proof union differs from the declared frontier");
    }
    if analysis.attempts.len() != CHECKED_ATTEMPTS.len()
        || analysis
            .attempts
            .iter()
            .zip(CHECKED_ATTEMPTS)
            .any(|(found, expected)| {
                found.id != expected.id
                    || found.status != AnalysisProgress::BlockedOpenProofs
                    || !matches_strings(&found.input_ids, expected.input_ids)
                    || !matches_strings(&found.open_proof_ids, expected.open_proof_ids)
            })
    {
        return invariant(
            "derivation frontier differs from the independent semantic specification",
        );
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
