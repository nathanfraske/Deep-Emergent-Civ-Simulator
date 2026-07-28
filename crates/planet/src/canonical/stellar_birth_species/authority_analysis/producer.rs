//! Producer for the partial species derivation frontier.

use super::{AnalysisProgress, SpeciesDerivationAttempt, LIVE_PHYSICAL_REGISTRY_ATTEMPT_ID};
use crate::canonical::stellar_birth_species::physical_registry::RepositoryPhysicalRegistryFrontier;
use std::collections::BTreeSet;

pub(super) struct ProducedFrontier {
    pub(super) attempts: Vec<SpeciesDerivationAttempt>,
    pub(super) open_proof_ids: Vec<String>,
    pub(super) candidate_member_count: usize,
    pub(super) verified_support_member_count: usize,
    pub(super) species_support_value_payload_present: bool,
    pub(super) residual_slot_claim: bool,
    pub(super) derive_first_status: AnalysisProgress,
    pub(super) buckingham_pi_status: AnalysisProgress,
    pub(super) gap_law_status: AnalysisProgress,
    pub(super) chaos_protocol_status: AnalysisProgress,
    pub(super) residual_law_status: AnalysisProgress,
    pub(super) unique_residual_slot_status: AnalysisProgress,
}

pub(super) fn produce_frontier(physical: &RepositoryPhysicalRegistryFrontier) -> ProducedFrontier {
    // Report the local closure and its first open global obligations.
    // Downstream support and reduction paths remain closed, so listing later
    // authored proof guesses here would imply coverage the live run lacks.
    let attempts = vec![SpeciesDerivationAttempt {
        id: LIVE_PHYSICAL_REGISTRY_ATTEMPT_ID,
        status: AnalysisProgress::BlockedOpenProofs,
        input_ids: vec![
            physical.root_claim_id.to_owned(),
            physical.primitive_profile.claim_id.to_owned(),
            physical.charged_profile.claim_id.to_owned(),
            physical.vocabulary_claim_id.clone(),
        ],
        open_proof_ids: physical
            .open_obligations
            .iter()
            .map(|obligation| (*obligation).to_owned())
            .collect(),
    }];
    let open_proof_ids = attempts
        .iter()
        .flat_map(|attempt| attempt.open_proof_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ProducedFrontier {
        attempts,
        open_proof_ids,
        candidate_member_count: usize::try_from(physical.registry_member_count)
            .unwrap_or(usize::MAX),
        verified_support_member_count: 0,
        species_support_value_payload_present: false,
        residual_slot_claim: true,
        derive_first_status: AnalysisProgress::ExecutedOpenFrontier,
        buckingham_pi_status: AnalysisProgress::SemanticInapplicabilityPaired,
        gap_law_status: AnalysisProgress::ExecutedAndBound,
        chaos_protocol_status: AnalysisProgress::NondynamicalInapplicabilityPaired,
        residual_law_status: AnalysisProgress::ExecutedAndBound,
        unique_residual_slot_status: AnalysisProgress::CollisionCheckedUnique,
    }
}
