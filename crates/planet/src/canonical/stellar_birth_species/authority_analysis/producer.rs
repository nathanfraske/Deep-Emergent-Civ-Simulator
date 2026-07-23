//! Producer for the non-admitting species derivation frontier.

use super::{AnalysisProgress, SpeciesDerivationAttempt};
use std::collections::BTreeSet;

pub(super) struct ProducedFrontier {
    pub(super) attempts: Vec<SpeciesDerivationAttempt>,
    pub(super) open_proof_ids: Vec<String>,
    pub(super) candidate_member_count: usize,
    pub(super) verified_support_member_count: usize,
    pub(super) value_payload_present: bool,
    pub(super) residual_slot_claim: bool,
    pub(super) derive_first_status: AnalysisProgress,
    pub(super) buckingham_pi_status: AnalysisProgress,
    pub(super) gap_law_status: AnalysisProgress,
    pub(super) chaos_protocol_status: AnalysisProgress,
    pub(super) residual_law_status: AnalysisProgress,
    pub(super) unique_residual_slot_status: AnalysisProgress,
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(super) fn produce_frontier() -> ProducedFrontier {
    let attempts = vec![
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.complete_registry",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&["fundamental.alpha", "fundamental.G", "fundamental.m_e"]),
            open_proof_ids: strings(&[
                "canonical_species_state_descriptor_checker_unavailable",
                "charge_state_sector_validity_proof_unavailable",
                "physical_species_membership_derivation_unavailable",
                "rest_mass_dimension_and_ancestry_proof_unavailable",
            ]),
        },
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.complete_conditioned_support",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&[
                "stellar_birth.composition.species_number_fraction_field",
                "stellar_birth.species_state_registry",
            ]),
            open_proof_ids: strings(&[
                "conditioned_zero_or_sparse_support_semantics_unavailable",
                "finite_exact_resource_domain_unavailable",
                "joint_measure_support_binding_unavailable",
            ]),
        },
        SpeciesDerivationAttempt {
            id: "stellar_birth.species_derivation.exact_mean_mass_projection",
            status: AnalysisProgress::BlockedOpenProofs,
            input_ids: strings(&["stellar_birth.conditioned_species_state_support"]),
            open_proof_ids: strings(&[
                "finite_exact_resource_domain_unavailable",
                "integer_projection_schema_unavailable",
                "joint_measure_support_binding_unavailable",
            ]),
        },
    ];
    let open_proof_ids = attempts
        .iter()
        .flat_map(|attempt| attempt.open_proof_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ProducedFrontier {
        attempts,
        open_proof_ids,
        candidate_member_count: 0,
        verified_support_member_count: 0,
        value_payload_present: false,
        residual_slot_claim: false,
        derive_first_status: AnalysisProgress::OpenDependencies,
        buckingham_pi_status: AnalysisProgress::DimensionOnlyRelationNotPhysicalClosure,
        gap_law_status: AnalysisProgress::NotReached,
        chaos_protocol_status: AnalysisProgress::NotReached,
        residual_law_status: AnalysisProgress::NotReached,
        unique_residual_slot_status: AnalysisProgress::NotClaimed,
    }
}
