//! Public read-only views over the bounded live species derivation refusal.

use super::{
    SpeciesDerivationAnalysis, SpeciesDerivationAnalysisArtifact, SpeciesDerivationAttempt,
};
use crate::canonical::stellar_birth_species::physical_registry::RepositoryRootAdmissionCensusRow;

/// Read-only view of the authority analysis attached to the open joint measure.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesDerivationAnalysisView<'a> {
    artifact: &'a SpeciesDerivationAnalysisArtifact,
}

/// Read-only view of one blocked derive-first attempt.
#[derive(Debug, Clone, Copy)]
pub struct SpeciesDerivationAttemptView<'a> {
    attempt: &'a SpeciesDerivationAttempt,
}

/// Read-only view of one identity-keyed physical-root admission.
#[derive(Debug, Clone, Copy)]
pub struct PhysicalRootAdmissionView<'a> {
    admission: &'a RepositoryRootAdmissionCensusRow,
}

impl<'a> SpeciesDerivationAnalysisView<'a> {
    pub(in crate::canonical) const fn new(artifact: &'a SpeciesDerivationAnalysisArtifact) -> Self {
        Self { artifact }
    }

    fn computed(self) -> Option<&'a SpeciesDerivationAnalysis> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(analysis) => Some(analysis),
            SpeciesDerivationAnalysisArtifact::Invalid(_) => None,
        }
    }

    pub fn is_computed(self) -> bool {
        self.computed().is_some()
    }

    pub fn floor_binding_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_binding_schema_id)
    }

    pub fn floor_binding_sha256(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_binding_sha256.as_str())
    }

    pub fn structure_schema_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.structure_schema_id)
    }

    pub fn species_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.species_registry_schema_id)
    }

    pub fn stellar_state_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.stellar_state_schema_id)
    }

    pub fn state_coordinate_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.state_coordinate_registry_schema_id)
    }

    pub fn interaction_sector_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.interaction_sector_registry_schema_id)
    }

    pub fn physical_regime_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_regime_registry_schema_id)
    }

    pub fn reducer_law_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.reducer_law_id)
    }

    pub fn floor_anchor_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.id)
    }

    pub fn floor_anchor_symbol(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.symbol)
    }

    pub fn floor_anchor_bits(self) -> Option<i128> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.bits)
    }

    pub fn floor_anchor_scale_bits(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.scale_bits)
    }

    pub fn floor_anchor_membership_authority(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.membership_authority)
    }

    pub fn floor_anchor_role(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.floor_mass_anchor.role)
    }

    pub fn physical_registry_root_receipt_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_receipt_schema_id)
    }

    pub fn physical_registry_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.registry_schema_id)
    }

    pub fn physical_registry_proof_graph_schema_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.proof_graph_schema_id)
    }

    pub fn physical_registry_root_claim_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_claim_id)
    }

    pub fn physical_registry_root_input_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_input_sha256)
    }

    pub fn physical_registry_root_result_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_result_sha256)
    }

    pub fn physical_registry_root_producer_result_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .root_producer_result_sha256
        })
    }

    pub fn physical_registry_root_watchdog_result_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .root_watchdog_result_sha256
        })
    }

    pub fn physical_registry_root_producer_resource_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .root_producer_resource_sha256
        })
    }

    pub fn physical_registry_root_watchdog_resource_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .root_watchdog_resource_sha256
        })
    }

    pub fn physical_registry_root_canary_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_canary_sha256)
    }

    pub fn physical_registry_root_canary_suite_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_canary_suite_id)
    }

    pub fn physical_registry_root_decision_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_decision_id)
    }

    pub fn physical_registry_root_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_receipt_sha256)
    }

    pub fn physical_registry_root_producer_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_producer_id)
    }

    pub fn physical_registry_root_watchdog_id(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.root_watchdog_id)
    }

    pub fn physical_vocabulary_receipt_schema_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_receipt_schema_id
                .as_str()
        })
    }

    pub fn physical_vocabulary_claim_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_claim_id
                .as_str()
        })
    }

    pub fn physical_vocabulary_producer_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_producer_id
                .as_str()
        })
    }

    pub fn physical_vocabulary_watchdog_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_watchdog_id
                .as_str()
        })
    }

    pub fn physical_vocabulary_descriptor_role_identities(self) -> Option<&'a [[u8; 32]]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_descriptor_role_identities
                .as_slice()
        })
    }

    pub fn physical_vocabulary_relation_target_identities(self) -> Option<&'a [[u8; 32]]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_relation_target_identities
                .as_slice()
        })
    }

    pub fn physical_vocabulary_constraint_law_identities(self) -> Option<&'a [[u8; 32]]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_constraint_law_identities
                .as_slice()
        })
    }

    pub fn physical_vocabulary_counts(self) -> Option<(u32, u32, u32, u32)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.vocabulary_root_count,
                frontier.vocabulary_descriptor_role_count,
                frontier.vocabulary_relation_target_count,
                frontier.vocabulary_constraint_law_count,
            )
        })
    }

    pub fn physical_vocabulary_scope(self) -> Option<(bool, bool, bool)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.vocabulary_current_input_partition_complete,
                frontier.vocabulary_global_coverage,
                frontier.vocabulary_membership_authority,
            )
        })
    }

    pub fn physical_vocabulary_checker_result_sha256(self) -> Option<([u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.vocabulary_producer_result_sha256,
                frontier.vocabulary_watchdog_result_sha256,
            )
        })
    }

    pub fn physical_vocabulary_checker_resource_sha256(self) -> Option<([u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.vocabulary_producer_resource_sha256,
                frontier.vocabulary_watchdog_resource_sha256,
            )
        })
    }

    pub fn physical_vocabulary_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .vocabulary_receipt_sha256
        })
    }

    pub fn physical_registry_scalar_coordinate_count(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.scalar_coordinate_count)
    }

    pub fn physical_registry_membership_neutral_mass_projection_count(self) -> Option<u32> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .membership_neutral_mass_projection_count
        })
    }

    pub fn physical_registry_admitted_root_count(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.admitted_root_count)
    }

    pub fn physical_registry_membership_authority(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.membership_authority)
    }

    pub fn physical_registry_root_admissions(
        self,
    ) -> impl Iterator<Item = PhysicalRootAdmissionView<'a>> + 'a {
        self.computed().into_iter().flat_map(|analysis| {
            analysis
                .physical_registry_frontier
                .root_admission_census
                .iter()
                .map(|admission| PhysicalRootAdmissionView { admission })
        })
    }

    pub fn physical_registry_refusal_code(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.registry_refusal_code)
    }

    pub fn physical_registry_member_count(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.registry_member_count)
    }

    pub fn physical_registry_coverage_claim(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.registry_coverage_claim)
    }

    pub fn physical_registry_authority_effect(self) -> Option<&'a str> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .registry_authority_effect
        })
    }

    pub fn physical_registry_open_obligations(self) -> &'a [&'static str] {
        self.computed().map_or(&[], |analysis| {
            analysis
                .physical_registry_frontier
                .open_obligations
                .as_slice()
        })
    }

    pub fn frontier_source_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.frontier_source_id)
    }

    pub fn frontier_scope_id(self) -> Option<&'a str> {
        self.computed().map(|analysis| analysis.frontier_scope_id)
    }

    pub fn frontier_completeness_claim(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.frontier_completeness_claim)
    }

    pub fn candidate_member_count(self) -> Option<usize> {
        self.computed()
            .map(|analysis| analysis.candidate_member_count)
    }

    pub fn verified_support_member_count(self) -> Option<usize> {
        self.computed()
            .map(|analysis| analysis.verified_support_member_count)
    }

    pub fn value_payload_present(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.value_payload_present)
    }

    pub fn residual_slot_claim(self) -> Option<bool> {
        self.computed().map(|analysis| analysis.residual_slot_claim)
    }

    pub fn derive_first_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.derive_first_status.id())
    }

    pub fn buckingham_pi_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.buckingham_pi_status.id())
    }

    pub fn gap_law_status_id(self) -> Option<&'static str> {
        self.computed().map(|analysis| analysis.gap_law_status.id())
    }

    pub fn chaos_protocol_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.chaos_protocol_status.id())
    }

    pub fn residual_law_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.residual_law_status.id())
    }

    pub fn unique_residual_slot_status_id(self) -> Option<&'static str> {
        self.computed()
            .map(|analysis| analysis.unique_residual_slot_status.id())
    }

    pub fn open_proof_ids(self) -> &'a [String] {
        self.computed()
            .map_or(&[], |analysis| analysis.open_proof_ids.as_slice())
    }

    pub fn attempts(self) -> impl ExactSizeIterator<Item = SpeciesDerivationAttemptView<'a>> + 'a {
        let attempts: &'a [SpeciesDerivationAttempt] = self
            .computed()
            .map_or(&[], |analysis| analysis.attempts.as_slice());
        attempts
            .iter()
            .map(|attempt| SpeciesDerivationAttemptView { attempt })
    }

    pub fn error_code(self) -> Option<&'a str> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(_) => None,
            SpeciesDerivationAnalysisArtifact::Invalid(invalid) => Some(invalid.error_code),
        }
    }

    pub fn error_detail(self) -> Option<&'a str> {
        match self.artifact {
            SpeciesDerivationAnalysisArtifact::Computed(_) => None,
            SpeciesDerivationAnalysisArtifact::Invalid(invalid) => Some(&invalid.detail),
        }
    }
}

impl PhysicalRootAdmissionView<'_> {
    pub const fn identity_sha256(self) -> [u8; 32] {
        self.admission.identity_sha256
    }

    pub const fn tier_id(self) -> &'static str {
        self.admission.tier_id
    }

    pub const fn provenance_tag(self) -> &'static str {
        self.admission.provenance_tag
    }

    pub const fn route_id(self) -> &'static str {
        self.admission.route_id
    }
}

impl<'a> SpeciesDerivationAttemptView<'a> {
    pub fn id(self) -> &'a str {
        self.attempt.id
    }

    pub fn status_id(self) -> &'static str {
        self.attempt.status.id()
    }

    pub fn input_ids(self) -> &'a [String] {
        &self.attempt.input_ids
    }

    pub fn open_proof_ids(self) -> &'a [String] {
        &self.attempt.open_proof_ids
    }
}
