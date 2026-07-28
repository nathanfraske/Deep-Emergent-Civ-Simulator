//! Public read-only views over the bounded live species derivation frontier.

use super::{
    SpeciesDerivationAnalysis, SpeciesDerivationAnalysisArtifact, SpeciesDerivationAttempt,
};
use crate::canonical::stellar_birth_species::physical_registry::RepositoryRootAdmissionCensusRow;

/// The action, derivation-catalog, coverage, and irreducible-protocol digests
/// bound by the locally admitted primitive profile.
pub type PrimitiveProfileEvidenceDigests = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

/// Charge-conjugation producer, charge-conjugation watchdog, mass-transport
/// producer, and mass-transport watchdog digests.
pub type ChargedProfileCheckerEvidenceDigests = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

/// Constituent threshold, decay family, candidate mass interval, and
/// separation-threshold interval digests for the neutral bound profile.
pub type NeutralBoundProfileChannelDigests = ([u8; 32], [u8; 32], [u8; 32], [u8; 32]);

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

/// Read-only semantic labels for the one locally admitted primitive profile.
#[derive(Debug, Clone, Copy)]
pub struct PrimitiveProfileSemanticsView<'a> {
    member_id: &'a str,
    symmetry_id: &'a str,
    field_id: &'a str,
    operator_id: &'a str,
    state_id: &'a str,
    sector_id: &'a str,
    validity_id: &'a str,
    excluded_term_id: &'a str,
    helicity_id: &'a str,
    statistics_id: &'a str,
    charge_id: &'a str,
    current_id: &'a str,
    stability_id: &'a str,
    transition_id: &'a str,
}

impl<'a> PrimitiveProfileSemanticsView<'a> {
    pub const fn dynamical_identity(
        self,
    ) -> (
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    ) {
        (
            self.member_id,
            self.symmetry_id,
            self.field_id,
            self.operator_id,
            self.state_id,
            self.sector_id,
            self.validity_id,
            self.excluded_term_id,
        )
    }

    pub const fn member_properties(self) -> (&'a str, &'a str, &'a str, &'a str, &'a str, &'a str) {
        (
            self.helicity_id,
            self.statistics_id,
            self.charge_id,
            self.current_id,
            self.stability_id,
            self.transition_id,
        )
    }
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

    pub fn law_premise_identity(self) -> Option<(&'a str, &'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.premise_id,
                frontier.artifact_schema_id,
                frontier.receipt_schema_id,
                frontier.canary_suite_id,
            )
        })
    }

    pub fn law_premise_output_symbol(self) -> Option<&'a str> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.output_symbol)
    }

    pub fn law_premise_output_bits(self) -> Option<i128> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.output_bits)
    }

    pub fn law_premise_output_scale_bits(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.output_scale_bits)
    }

    pub fn law_premise_output_projection_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .law_premise_frontier
                .output_projection_receipt_sha256
        })
    }

    pub fn law_premise_relation_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.canonical_relation_sha256)
    }

    pub fn law_premise_checker_identities(self) -> Option<(&'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.producer_implementation_id,
                frontier.watchdog_implementation_id,
                frontier.decision_id,
            )
        })
    }

    pub fn law_premise_checker_result_sha256(self) -> Option<([u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.producer_result_sha256,
                frontier.watchdog_result_sha256,
            )
        })
    }

    pub fn law_premise_canary_transcript_ids(self) -> Option<(&'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.producer_canary_transcript_id,
                frontier.watchdog_canary_transcript_id,
            )
        })
    }

    pub fn law_premise_canary_evidence(self) -> Option<[(u32, [u8; 32]); 2]> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            [
                (
                    frontier.producer_canary_case_count,
                    frontier.producer_canary_sha256,
                ),
                (
                    frontier.watchdog_canary_case_count,
                    frontier.watchdog_canary_sha256,
                ),
            ]
        })
    }

    pub fn law_premise_claim_identity_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.claim_identity_sha256)
    }

    pub fn law_premise_role_identity_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.role_identity_sha256)
    }

    pub fn law_premise_content_identity_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.content_identity_sha256)
    }

    pub fn law_premise_upstream_capability_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.upstream_capability_sha256)
    }

    pub fn law_premise_applicability_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.applicability_receipt_sha256)
    }

    pub fn law_premise_validity_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.validity_receipt_sha256)
    }

    pub fn law_premise_ancestry_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.ancestry_receipt_sha256)
    }

    pub fn law_premise_pair_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.pair_receipt_sha256)
    }

    pub fn law_premise_capability_sha256(self) -> Option<[u8; 32]> {
        self.computed()
            .map(|analysis| analysis.law_premise_frontier.capability_sha256)
    }

    pub fn law_premise_ledger_classification(self) -> Option<(&'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.tier_id,
                frontier.provenance_tag,
                frontier.content_proof_kind,
            )
        })
    }

    pub fn law_premise_scope(self) -> Option<(bool, bool, bool, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.law_premise_frontier;
            (
                frontier.requested_premise_coverage,
                frontier.species_membership_authority,
                frontier.global_physical_premise_coverage,
                frontier.authority_effect,
            )
        })
    }

    pub fn premise_admission_route_identity(self) -> Option<(&'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.schema_id,
                frontier.producer_id,
                frontier.watchdog_id,
            )
        })
    }

    pub fn premise_admission_route_result_sha256(self) -> Option<([u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.producer_result_sha256,
                frontier.watchdog_result_sha256,
            )
        })
    }

    pub fn premise_admission_derived_identity(self) -> Option<([u8; 32], [u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.derived_claim_identity_sha256,
                frontier.derived_role_identity_sha256,
                frontier.derived_content_identity_sha256,
            )
        })
    }

    pub fn premise_admission_route_counts(self) -> Option<(u32, u32, u32, u32)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.derived_route_count,
                frontier.current_descriptor_role_count,
                frontier.current_relation_target_count,
                frontier.current_constraint_law_count,
            )
        })
    }

    pub fn premise_admission_route_decisions(self) -> Option<(&'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.derived_decision_id,
                frontier.next_target_decision_id,
            )
        })
    }

    pub fn premise_admission_route_scope(self) -> Option<(bool, bool, bool, bool, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.premise_admission_frontier;
            (
                frontier.derivation_frontier_complete,
                frontier.irreducible_protocol_started,
                frontier.premise_admission_authority,
                frontier.species_membership_authority,
                frontier.authority_effect,
            )
        })
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

    pub fn physical_registry_admitted_artifact_count(self) -> Option<u32> {
        self.computed()
            .map(|analysis| analysis.physical_registry_frontier.admitted_artifact_count)
    }

    pub fn primitive_profile_identity(
        self,
    ) -> Option<(
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    )> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.primitive_profile.receipt_schema_id,
                frontier.primitive_profile.claim_id,
                frontier.primitive_profile.profile_id,
                frontier.primitive_profile.theory_class_id,
                frontier.primitive_profile.residual_slot_id,
                frontier.primitive_profile.producer_id,
                frontier.primitive_profile.watchdog_id,
            )
        })
    }

    pub fn primitive_profile_counts(self) -> Option<(u32, u32, u32)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.primitive_profile.artifact_count,
                frontier.primitive_profile.symmetry_basis_element_count,
                frontier.primitive_profile.symmetry_excluded_operator_count,
            )
        })
    }

    pub fn primitive_profile_member_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .primitive_profile
                .member_sha256
        })
    }

    pub fn primitive_profile_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .primitive_profile
                .pair_receipt_sha256
        })
    }

    pub fn primitive_profile_evidence_sha256(self) -> Option<PrimitiveProfileEvidenceDigests> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.primitive_profile.symmetry_action_binding_sha256,
                frontier.primitive_profile.derivation_catalog_sha256,
                frontier
                    .primitive_profile
                    .derivation_coverage_capability_sha256,
                frontier
                    .primitive_profile
                    .irreducible_protocol_capability_sha256,
            )
        })
    }

    pub fn primitive_profile_repository_catalog_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .primitive_profile
                .repository_catalog_sha256
        })
    }

    pub fn primitive_profile_protocol_checker_result_sha256(self) -> Option<([u8; 32], [u8; 32])> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.primitive_profile;
            (
                profile.protocol_producer_result_sha256,
                profile.protocol_watchdog_result_sha256,
            )
        })
    }

    pub fn primitive_profile_protocol_statuses(
        self,
    ) -> Option<(&'a str, &'a str, &'a str, &'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let frontier = &analysis.physical_registry_frontier;
            (
                frontier.primitive_profile.derive_first_status_id,
                frontier.primitive_profile.buckingham_pi_status_id,
                frontier.primitive_profile.gap_law_status_id,
                frontier.primitive_profile.chaos_protocol_status_id,
                frontier.primitive_profile.residual_law_status_id,
                frontier.primitive_profile.residual_slot_status_id,
            )
        })
    }

    pub fn primitive_profile_semantics(self) -> Option<PrimitiveProfileSemanticsView<'a>> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.primitive_profile;
            PrimitiveProfileSemanticsView {
                member_id: profile.member_id,
                symmetry_id: profile.symmetry_id,
                field_id: profile.field_id,
                operator_id: profile.operator_id,
                state_id: profile.state_id,
                sector_id: profile.sector_id,
                validity_id: profile.validity_id,
                excluded_term_id: profile.excluded_term_id,
                helicity_id: profile.helicity_id,
                statistics_id: profile.statistics_id,
                charge_id: profile.charge_id,
                current_id: profile.current_id,
                stability_id: profile.stability_id,
                transition_id: profile.transition_id,
            }
        })
    }

    pub fn charged_profile_identity(
        self,
    ) -> Option<(
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    )> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.charged_profile;
            (
                profile.receipt_schema_id,
                profile.claim_id,
                profile.profile_id,
                profile.theory_class_id,
                profile.residual_slot_id,
                profile.producer_id,
                profile.watchdog_id,
            )
        })
    }

    pub fn charged_profile_counts(self) -> Option<(u32, usize)> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.charged_profile;
            (profile.artifact_count, profile.member_sha256.len())
        })
    }

    pub fn charged_profile_member_sha256(self) -> Option<&'a [[u8; 32]]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .charged_profile
                .member_sha256
                .as_slice()
        })
    }

    pub fn charged_profile_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .charged_profile
                .pair_receipt_sha256
        })
    }

    pub fn charged_profile_checker_evidence_sha256(
        self,
    ) -> Option<ChargedProfileCheckerEvidenceDigests> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.charged_profile;
            (
                profile.charge_conjugation_producer_sha256,
                profile.charge_conjugation_watchdog_sha256,
                profile.mass_transport_producer_sha256,
                profile.mass_transport_watchdog_sha256,
            )
        })
    }

    pub fn charged_profile_protocol_statuses(
        self,
    ) -> Option<(&'a str, &'a str, &'a str, &'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.charged_profile;
            (
                profile.derive_first_status_id,
                profile.buckingham_pi_status_id,
                profile.gap_law_status_id,
                profile.chaos_protocol_status_id,
                profile.residual_law_status_id,
                profile.residual_slot_status_id,
            )
        })
    }

    pub fn neutral_bound_profile_identity(
        self,
    ) -> Option<(
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
        &'a str,
    )> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            (
                profile.receipt_schema_id,
                profile.claim_id,
                profile.profile_id,
                profile.theory_class_id,
                profile.residual_slot_id,
                profile.producer_id,
                profile.watchdog_id,
            )
        })
    }

    pub fn neutral_bound_profile_counts(self) -> Option<(u32, usize)> {
        self.computed().map(|analysis| {
            (
                analysis
                    .physical_registry_frontier
                    .neutral_bound_profile
                    .artifact_count,
                1,
            )
        })
    }

    pub fn neutral_bound_profile_member_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .neutral_bound_profile
                .member_sha256
        })
    }

    pub fn neutral_bound_profile_receipt_sha256(self) -> Option<[u8; 32]> {
        self.computed().map(|analysis| {
            analysis
                .physical_registry_frontier
                .neutral_bound_profile
                .pair_receipt_sha256
        })
    }

    pub fn neutral_bound_profile_checker_evidence_sha256(self) -> Option<[[u8; 32]; 10]> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            [
                profile.solver_producer_sha256,
                profile.solver_watchdog_sha256,
                profile.normalization_producer_sha256,
                profile.normalization_watchdog_sha256,
                profile.threshold_coverage_producer_sha256,
                profile.threshold_coverage_watchdog_sha256,
                profile.uncertainty_transport_producer_sha256,
                profile.uncertainty_transport_watchdog_sha256,
                profile.conservation_producer_sha256,
                profile.conservation_watchdog_sha256,
            ]
        })
    }

    pub fn neutral_bound_profile_channel_digests(
        self,
    ) -> Option<NeutralBoundProfileChannelDigests> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            (
                profile.constituent_threshold_channel_sha256,
                profile.decay_channel_family_sha256,
                profile.mass_interval_sha256,
                profile.threshold_interval_sha256,
            )
        })
    }

    pub fn neutral_bound_profile_dispositions(self) -> Option<(&'a str, &'a str)> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            (profile.binding_disposition_id, profile.decay_disposition_id)
        })
    }

    pub fn neutral_bound_profile_scope(self) -> Option<(bool, bool)> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            (
                profile.conditioned_support_authority,
                profile.global_stability_claim,
            )
        })
    }

    pub fn neutral_bound_profile_protocol_statuses(
        self,
    ) -> Option<(&'a str, &'a str, &'a str, &'a str, &'a str, &'a str)> {
        self.computed().map(|analysis| {
            let profile = &analysis.physical_registry_frontier.neutral_bound_profile;
            (
                profile.derive_first_status_id,
                profile.buckingham_pi_status_id,
                profile.gap_law_status_id,
                profile.chaos_protocol_status_id,
                profile.residual_law_status_id,
                profile.residual_slot_status_id,
            )
        })
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

    pub fn primitive_profile_admissions(
        self,
    ) -> impl Iterator<Item = PhysicalRootAdmissionView<'a>> + 'a {
        self.computed().into_iter().flat_map(|analysis| {
            analysis
                .physical_registry_frontier
                .primitive_profile
                .admission_census
                .iter()
                .map(|admission| PhysicalRootAdmissionView { admission })
        })
    }

    pub fn charged_profile_admissions(
        self,
    ) -> impl Iterator<Item = PhysicalRootAdmissionView<'a>> + 'a {
        self.computed().into_iter().flat_map(|analysis| {
            analysis
                .physical_registry_frontier
                .charged_profile
                .admission_census
                .iter()
                .map(|admission| PhysicalRootAdmissionView { admission })
        })
    }

    pub fn neutral_bound_profile_admissions(
        self,
    ) -> impl Iterator<Item = PhysicalRootAdmissionView<'a>> + 'a {
        self.computed().into_iter().flat_map(|analysis| {
            analysis
                .physical_registry_frontier
                .neutral_bound_profile
                .admission_census
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

    pub fn species_support_value_payload_present(self) -> Option<bool> {
        self.computed()
            .map(|analysis| analysis.species_support_value_payload_present)
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
