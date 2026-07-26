//! Producer for the non-admitting species derivation frontier.

use super::{AnalysisProgress, SpeciesDerivationAttempt, LIVE_PHYSICAL_REGISTRY_ATTEMPT_ID};
use crate::canonical::stellar_birth_species::physical_registry::RepositoryPhysicalRegistryFrontier;
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

pub(super) fn produce_frontier(physical: &RepositoryPhysicalRegistryFrontier) -> ProducedFrontier {
    // Report only the first executable refusal reached by the repository.
    // Downstream support and reduction paths are not attempted while the
    // physical registry refuses, so listing their authored proof guesses here
    // would imply coverage that the live run has not established.
    let attempts = vec![SpeciesDerivationAttempt {
        id: LIVE_PHYSICAL_REGISTRY_ATTEMPT_ID,
        status: AnalysisProgress::BlockedOpenProofs,
        input_ids: vec![
            physical.root_claim_id.to_owned(),
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
