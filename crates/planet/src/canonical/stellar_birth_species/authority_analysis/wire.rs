//! Canonical text encoding for the bounded live species derivation diagnostic.

use super::SpeciesDerivationAnalysisArtifact;
use crate::canonical::transcript::canonical_text;
use std::fmt;

pub(in crate::canonical) fn write_species_derivation_analysis(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    artifact: &SpeciesDerivationAnalysisArtifact,
) -> fmt::Result {
    match artifact {
        SpeciesDerivationAnalysisArtifact::Invalid(invalid) => {
            writeln!(f, "{prefix}.error.code={}", invalid.error_code)?;
            writeln!(
                f,
                "{prefix}.error.detail={}",
                canonical_text(&invalid.detail)
            )
        }
        SpeciesDerivationAnalysisArtifact::Computed(analysis) => {
            write_schema_bindings(f, prefix, analysis)?;
            write_floor_anchor(f, prefix, analysis)?;
            write_law_premise_frontier(f, prefix, analysis)?;
            write_premise_admission_frontier(f, prefix, analysis)?;
            write_physical_registry_frontier(f, prefix, analysis)?;
            writeln!(
                f,
                "{prefix}.frontier.source={}",
                canonical_text(analysis.frontier_source_id)
            )?;
            writeln!(
                f,
                "{prefix}.frontier.scope={}",
                canonical_text(analysis.frontier_scope_id)
            )?;
            writeln!(
                f,
                "{prefix}.frontier.completeness_claim={}",
                analysis.frontier_completeness_claim
            )?;
            writeln!(
                f,
                "{prefix}.candidate_member_count={}",
                analysis.candidate_member_count
            )?;
            writeln!(
                f,
                "{prefix}.verified_support_member_count={}",
                analysis.verified_support_member_count
            )?;
            writeln!(
                f,
                "{prefix}.species_support_value_payload_present={}",
                analysis.species_support_value_payload_present
            )?;
            writeln!(
                f,
                "{prefix}.residual_slot_claim={}",
                analysis.residual_slot_claim
            )?;
            write_protocol_statuses(f, prefix, analysis)?;
            write_string_list(f, prefix, "open_proof", &analysis.open_proof_ids)?;
            writeln!(f, "{prefix}.attempt_count={}", analysis.attempts.len())?;
            for (index, attempt) in analysis.attempts.iter().enumerate() {
                let attempt_prefix = format!("{prefix}.attempt.{index:04}");
                writeln!(f, "{attempt_prefix}.id={}", canonical_text(attempt.id))?;
                writeln!(f, "{attempt_prefix}.status={}", attempt.status.id())?;
                write_string_list(f, &attempt_prefix, "input", &attempt.input_ids)?;
                write_string_list(f, &attempt_prefix, "open_proof", &attempt.open_proof_ids)?;
            }
            Ok(())
        }
    }
}

fn write_premise_admission_frontier(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    let frontier = &analysis.premise_admission_frontier;
    let route_prefix = format!("{prefix}.premise_admission_frontier");
    for (field, value) in [
        ("schema", frontier.schema_id),
        ("producer_id", frontier.producer_id),
        ("watchdog_id", frontier.watchdog_id),
        ("derived_decision_id", frontier.derived_decision_id),
        ("next_target_decision_id", frontier.next_target_decision_id),
        ("authority_effect", frontier.authority_effect),
    ] {
        writeln!(f, "{route_prefix}.{field}={}", canonical_text(value))?;
    }
    for (field, digest) in [
        ("producer_result.sha256", frontier.producer_result_sha256),
        ("watchdog_result.sha256", frontier.watchdog_result_sha256),
        (
            "derived_claim_identity.sha256",
            frontier.derived_claim_identity_sha256,
        ),
        (
            "derived_role_identity.sha256",
            frontier.derived_role_identity_sha256,
        ),
        (
            "derived_content_identity.sha256",
            frontier.derived_content_identity_sha256,
        ),
    ] {
        write_digest(f, &route_prefix, field, digest)?;
    }
    writeln!(
        f,
        "{route_prefix}.derived_route_count={}",
        frontier.derived_route_count
    )?;
    writeln!(
        f,
        "{route_prefix}.current_descriptor_role_count={}",
        frontier.current_descriptor_role_count
    )?;
    writeln!(
        f,
        "{route_prefix}.current_relation_target_count={}",
        frontier.current_relation_target_count
    )?;
    writeln!(
        f,
        "{route_prefix}.current_constraint_law_count={}",
        frontier.current_constraint_law_count
    )?;
    writeln!(
        f,
        "{route_prefix}.derivation_frontier_complete={}",
        frontier.derivation_frontier_complete
    )?;
    writeln!(
        f,
        "{route_prefix}.irreducible_protocol_started={}",
        frontier.irreducible_protocol_started
    )?;
    writeln!(
        f,
        "{route_prefix}.premise_admission_authority={}",
        frontier.premise_admission_authority
    )?;
    writeln!(
        f,
        "{route_prefix}.species_membership_authority={}",
        frontier.species_membership_authority
    )
}

fn write_law_premise_frontier(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    let frontier = &analysis.law_premise_frontier;
    let premise_prefix = format!("{prefix}.law_premise_frontier");
    for (field, value) in [
        ("premise_id", frontier.premise_id),
        ("artifact_schema", frontier.artifact_schema_id),
        ("receipt_schema", frontier.receipt_schema_id),
        ("canary_suite_id", frontier.canary_suite_id),
        (
            "producer_implementation_id",
            frontier.producer_implementation_id,
        ),
        (
            "watchdog_implementation_id",
            frontier.watchdog_implementation_id,
        ),
        (
            "producer_canary.transcript_id",
            frontier.producer_canary_transcript_id,
        ),
        (
            "watchdog_canary.transcript_id",
            frontier.watchdog_canary_transcript_id,
        ),
        ("decision_id", frontier.decision_id),
        ("output.symbol", frontier.output_symbol),
        ("tier", frontier.tier_id),
        ("provenance", frontier.provenance_tag),
        ("content_proof_kind", frontier.content_proof_kind),
        ("authority_effect", frontier.authority_effect),
    ] {
        writeln!(f, "{premise_prefix}.{field}={}", canonical_text(value))?;
    }
    writeln!(f, "{premise_prefix}.output.bits={}", frontier.output_bits)?;
    writeln!(
        f,
        "{premise_prefix}.output.scale_bits={}",
        frontier.output_scale_bits
    )?;
    for (field, digest) in [
        (
            "output.projection_receipt.sha256",
            frontier.output_projection_receipt_sha256,
        ),
        (
            "canonical_relation.sha256",
            frontier.canonical_relation_sha256,
        ),
        ("producer_result.sha256", frontier.producer_result_sha256),
        ("watchdog_result.sha256", frontier.watchdog_result_sha256),
        ("claim_identity.sha256", frontier.claim_identity_sha256),
        ("role_identity.sha256", frontier.role_identity_sha256),
        ("content_identity.sha256", frontier.content_identity_sha256),
        (
            "upstream_capability.sha256",
            frontier.upstream_capability_sha256,
        ),
        (
            "applicability_receipt.sha256",
            frontier.applicability_receipt_sha256,
        ),
        ("validity_receipt.sha256", frontier.validity_receipt_sha256),
        ("ancestry_receipt.sha256", frontier.ancestry_receipt_sha256),
        ("pair_receipt.sha256", frontier.pair_receipt_sha256),
        ("producer_canary.sha256", frontier.producer_canary_sha256),
        ("watchdog_canary.sha256", frontier.watchdog_canary_sha256),
        ("capability.sha256", frontier.capability_sha256),
    ] {
        write_digest(f, &premise_prefix, field, digest)?;
    }
    writeln!(
        f,
        "{premise_prefix}.producer_canary.case_count={}",
        frontier.producer_canary_case_count
    )?;
    writeln!(
        f,
        "{premise_prefix}.watchdog_canary.case_count={}",
        frontier.watchdog_canary_case_count
    )?;
    writeln!(
        f,
        "{premise_prefix}.requested_premise_coverage={}",
        frontier.requested_premise_coverage
    )?;
    writeln!(
        f,
        "{premise_prefix}.species_membership_authority={}",
        frontier.species_membership_authority
    )?;
    writeln!(
        f,
        "{premise_prefix}.global_physical_premise_coverage={}",
        frontier.global_physical_premise_coverage
    )
}

fn write_physical_registry_frontier(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    let frontier = &analysis.physical_registry_frontier;
    let frontier_prefix = format!("{prefix}.physical_registry_frontier");
    for (field, value) in [
        ("registry_schema", frontier.registry_schema_id),
        ("proof_graph_schema", frontier.proof_graph_schema_id),
        ("root_receipt_schema", frontier.root_receipt_schema_id),
        ("root_claim_id", frontier.root_claim_id),
        ("root_producer_id", frontier.root_producer_id),
        ("root_watchdog_id", frontier.root_watchdog_id),
        ("root_canary_suite_id", frontier.root_canary_suite_id),
        ("root_decision_id", frontier.root_decision_id),
        (
            "vocabulary_receipt_schema",
            frontier.vocabulary_receipt_schema_id.as_str(),
        ),
        ("vocabulary_claim_id", frontier.vocabulary_claim_id.as_str()),
        (
            "vocabulary_producer_id",
            frontier.vocabulary_producer_id.as_str(),
        ),
        (
            "vocabulary_watchdog_id",
            frontier.vocabulary_watchdog_id.as_str(),
        ),
        (
            "primitive_profile_receipt_schema",
            frontier.primitive_profile.receipt_schema_id,
        ),
        (
            "primitive_profile_claim_id",
            frontier.primitive_profile.claim_id,
        ),
        (
            "primitive_profile_id",
            frontier.primitive_profile.profile_id,
        ),
        (
            "primitive_profile_theory_class_id",
            frontier.primitive_profile.theory_class_id,
        ),
        (
            "primitive_profile_residual_slot_id",
            frontier.primitive_profile.residual_slot_id,
        ),
        (
            "primitive_profile_producer_id",
            frontier.primitive_profile.producer_id,
        ),
        (
            "primitive_profile_watchdog_id",
            frontier.primitive_profile.watchdog_id,
        ),
        (
            "primitive_profile_member_id",
            frontier.primitive_profile.member_id,
        ),
        (
            "primitive_profile_symmetry_id",
            frontier.primitive_profile.symmetry_id,
        ),
        (
            "primitive_profile_field_id",
            frontier.primitive_profile.field_id,
        ),
        (
            "primitive_profile_operator_id",
            frontier.primitive_profile.operator_id,
        ),
        (
            "primitive_profile_state_id",
            frontier.primitive_profile.state_id,
        ),
        (
            "primitive_profile_sector_id",
            frontier.primitive_profile.sector_id,
        ),
        (
            "primitive_profile_validity_id",
            frontier.primitive_profile.validity_id,
        ),
        (
            "primitive_profile_helicity_id",
            frontier.primitive_profile.helicity_id,
        ),
        (
            "primitive_profile_statistics_id",
            frontier.primitive_profile.statistics_id,
        ),
        (
            "primitive_profile_charge_id",
            frontier.primitive_profile.charge_id,
        ),
        (
            "primitive_profile_current_id",
            frontier.primitive_profile.current_id,
        ),
        (
            "primitive_profile_stability_id",
            frontier.primitive_profile.stability_id,
        ),
        (
            "primitive_profile_transition_id",
            frontier.primitive_profile.transition_id,
        ),
        (
            "primitive_profile_excluded_term_id",
            frontier.primitive_profile.excluded_term_id,
        ),
        (
            "primitive_profile_derive_first_status",
            frontier.primitive_profile.derive_first_status_id,
        ),
        (
            "primitive_profile_buckingham_pi_status",
            frontier.primitive_profile.buckingham_pi_status_id,
        ),
        (
            "primitive_profile_gap_law_status",
            frontier.primitive_profile.gap_law_status_id,
        ),
        (
            "primitive_profile_chaos_protocol_status",
            frontier.primitive_profile.chaos_protocol_status_id,
        ),
        (
            "primitive_profile_residual_law_status",
            frontier.primitive_profile.residual_law_status_id,
        ),
        (
            "primitive_profile_residual_slot_status",
            frontier.primitive_profile.residual_slot_status_id,
        ),
        ("registry_refusal_code", frontier.registry_refusal_code),
        (
            "registry_authority_effect",
            frontier.registry_authority_effect,
        ),
    ] {
        writeln!(f, "{frontier_prefix}.{field}={}", canonical_text(value))?;
    }
    for (field, digest) in [
        ("root_input.sha256", frontier.root_input_sha256),
        ("root_result.sha256", frontier.root_result_sha256),
        (
            "root_producer_result.sha256",
            frontier.root_producer_result_sha256,
        ),
        (
            "root_watchdog_result.sha256",
            frontier.root_watchdog_result_sha256,
        ),
        (
            "root_producer_resource.sha256",
            frontier.root_producer_resource_sha256,
        ),
        (
            "root_watchdog_resource.sha256",
            frontier.root_watchdog_resource_sha256,
        ),
        ("root_canary.sha256", frontier.root_canary_sha256),
        ("root_receipt.sha256", frontier.root_receipt_sha256),
        (
            "vocabulary_producer_result.sha256",
            frontier.vocabulary_producer_result_sha256,
        ),
        (
            "vocabulary_watchdog_result.sha256",
            frontier.vocabulary_watchdog_result_sha256,
        ),
        (
            "vocabulary_producer_resource.sha256",
            frontier.vocabulary_producer_resource_sha256,
        ),
        (
            "vocabulary_watchdog_resource.sha256",
            frontier.vocabulary_watchdog_resource_sha256,
        ),
        (
            "vocabulary_receipt.sha256",
            frontier.vocabulary_receipt_sha256,
        ),
        (
            "primitive_profile_member.sha256",
            frontier.primitive_profile.member_sha256,
        ),
        (
            "primitive_profile_pair_receipt.sha256",
            frontier.primitive_profile.pair_receipt_sha256,
        ),
        (
            "primitive_profile_symmetry_action_binding.sha256",
            frontier.primitive_profile.symmetry_action_binding_sha256,
        ),
        (
            "primitive_profile_derivation_catalog.sha256",
            frontier.primitive_profile.derivation_catalog_sha256,
        ),
        (
            "primitive_profile_repository_catalog.sha256",
            frontier.primitive_profile.repository_catalog_sha256,
        ),
        (
            "primitive_profile_protocol_producer_result.sha256",
            frontier.primitive_profile.protocol_producer_result_sha256,
        ),
        (
            "primitive_profile_protocol_watchdog_result.sha256",
            frontier.primitive_profile.protocol_watchdog_result_sha256,
        ),
        (
            "primitive_profile_derivation_coverage_capability.sha256",
            frontier
                .primitive_profile
                .derivation_coverage_capability_sha256,
        ),
        (
            "primitive_profile_irreducible_protocol_capability.sha256",
            frontier
                .primitive_profile
                .irreducible_protocol_capability_sha256,
        ),
    ] {
        write_digest(f, &frontier_prefix, field, digest)?;
    }
    writeln!(
        f,
        "{frontier_prefix}.scalar_coordinate_count={}",
        frontier.scalar_coordinate_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.membership_neutral_mass_projection_count={}",
        frontier.membership_neutral_mass_projection_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.admitted_root_count={}",
        frontier.admitted_root_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.admitted_artifact_count={}",
        frontier.admitted_artifact_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.primitive_profile_artifact_count={}",
        frontier.primitive_profile.artifact_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.primitive_profile_symmetry_basis_element_count={}",
        frontier.primitive_profile.symmetry_basis_element_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.primitive_profile_symmetry_excluded_operator_count={}",
        frontier.primitive_profile.symmetry_excluded_operator_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.primitive_profile_admission_count={}",
        frontier.primitive_profile.admission_census.len()
    )?;
    for (index, admission) in frontier
        .primitive_profile
        .admission_census
        .iter()
        .enumerate()
    {
        let admission_prefix = format!("{frontier_prefix}.primitive_profile_admission.{index:04}");
        write_digest(
            f,
            &admission_prefix,
            "identity.sha256",
            admission.identity_sha256,
        )?;
        writeln!(
            f,
            "{admission_prefix}.tier={}",
            canonical_text(admission.tier_id)
        )?;
        writeln!(
            f,
            "{admission_prefix}.provenance={}",
            canonical_text(admission.provenance_tag)
        )?;
        writeln!(
            f,
            "{admission_prefix}.route={}",
            canonical_text(admission.route_id)
        )?;
    }
    writeln!(
        f,
        "{frontier_prefix}.root_admission_count={}",
        frontier.root_admission_census.len()
    )?;
    for (index, admission) in frontier.root_admission_census.iter().enumerate() {
        let admission_prefix = format!("{frontier_prefix}.root_admission.{index:04}");
        write_digest(
            f,
            &admission_prefix,
            "identity.sha256",
            admission.identity_sha256,
        )?;
        writeln!(
            f,
            "{admission_prefix}.tier={}",
            canonical_text(admission.tier_id)
        )?;
        writeln!(
            f,
            "{admission_prefix}.provenance={}",
            canonical_text(admission.provenance_tag)
        )?;
        writeln!(
            f,
            "{admission_prefix}.route={}",
            canonical_text(admission.route_id)
        )?;
    }
    writeln!(
        f,
        "{frontier_prefix}.membership_authority={}",
        frontier.membership_authority
    )?;
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.root_count={}",
        frontier.vocabulary_root_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.descriptor_role_count={}",
        frontier.vocabulary_descriptor_role_count
    )?;
    for (index, identity) in frontier
        .vocabulary_descriptor_role_identities
        .iter()
        .enumerate()
    {
        write_digest(
            f,
            &frontier_prefix,
            &format!("vocabulary.descriptor_role.{index:04}.sha256"),
            *identity,
        )?;
    }
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.relation_target_count={}",
        frontier.vocabulary_relation_target_count
    )?;
    for (index, identity) in frontier
        .vocabulary_relation_target_identities
        .iter()
        .enumerate()
    {
        write_digest(
            f,
            &frontier_prefix,
            &format!("vocabulary.relation_target.{index:04}.sha256"),
            *identity,
        )?;
    }
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.constraint_law_count={}",
        frontier.vocabulary_constraint_law_count
    )?;
    for (index, identity) in frontier
        .vocabulary_constraint_law_identities
        .iter()
        .enumerate()
    {
        write_digest(
            f,
            &frontier_prefix,
            &format!("vocabulary.constraint_law.{index:04}.sha256"),
            *identity,
        )?;
    }
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.current_input_partition_complete={}",
        frontier.vocabulary_current_input_partition_complete
    )?;
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.global_coverage={}",
        frontier.vocabulary_global_coverage
    )?;
    writeln!(
        f,
        "{frontier_prefix}.vocabulary.membership_authority={}",
        frontier.vocabulary_membership_authority
    )?;
    writeln!(
        f,
        "{frontier_prefix}.registry_member_count={}",
        frontier.registry_member_count
    )?;
    writeln!(
        f,
        "{frontier_prefix}.registry_coverage_claim={}",
        frontier.registry_coverage_claim
    )?;
    writeln!(
        f,
        "{frontier_prefix}.open_obligation_count={}",
        frontier.open_obligations.len()
    )?;
    for (index, obligation) in frontier.open_obligations.iter().enumerate() {
        writeln!(
            f,
            "{frontier_prefix}.open_obligation.{index:04}={}",
            canonical_text(obligation)
        )?;
    }
    Ok(())
}

fn write_digest(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    field: &str,
    digest: [u8; 32],
) -> fmt::Result {
    write!(f, "{prefix}.{field}=")?;
    for byte in digest {
        write!(f, "{byte:02x}")?;
    }
    writeln!(f)
}

fn write_schema_bindings(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    for (field, value) in [
        ("floor_binding.schema", analysis.floor_binding_schema_id),
        ("structure_schema", analysis.structure_schema_id),
        (
            "species_registry_schema",
            analysis.species_registry_schema_id,
        ),
        ("stellar_state_schema", analysis.stellar_state_schema_id),
        (
            "state_coordinate_registry_schema",
            analysis.state_coordinate_registry_schema_id,
        ),
        (
            "interaction_sector_registry_schema",
            analysis.interaction_sector_registry_schema_id,
        ),
        (
            "physical_regime_registry_schema",
            analysis.physical_regime_registry_schema_id,
        ),
        ("reducer_law_id", analysis.reducer_law_id),
    ] {
        writeln!(f, "{prefix}.{field}={}", canonical_text(value))?;
    }
    writeln!(
        f,
        "{prefix}.floor_binding.sha256={}",
        analysis.floor_binding_sha256
    )
}

fn write_floor_anchor(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    writeln!(
        f,
        "{prefix}.floor_anchor.id={}",
        canonical_text(analysis.floor_mass_anchor.id)
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.symbol={}",
        canonical_text(analysis.floor_mass_anchor.symbol)
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.bits={}",
        analysis.floor_mass_anchor.bits
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.scale_bits={}",
        analysis.floor_mass_anchor.scale_bits
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.role={}",
        analysis.floor_mass_anchor.role
    )?;
    writeln!(
        f,
        "{prefix}.floor_anchor.membership_authority={}",
        analysis.floor_mass_anchor.membership_authority
    )
}

fn write_protocol_statuses(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    analysis: &super::SpeciesDerivationAnalysis,
) -> fmt::Result {
    for (field, status) in [
        ("derive_first", analysis.derive_first_status),
        ("buckingham_pi", analysis.buckingham_pi_status),
        ("gap_law", analysis.gap_law_status),
        ("chaos", analysis.chaos_protocol_status),
        ("residual_law", analysis.residual_law_status),
        ("unique_residual_slot", analysis.unique_residual_slot_status),
    ] {
        writeln!(f, "{prefix}.protocol.{field}={}", status.id())?;
    }
    Ok(())
}

fn write_string_list(
    f: &mut fmt::Formatter<'_>,
    prefix: &str,
    field: &str,
    values: &[String],
) -> fmt::Result {
    writeln!(f, "{prefix}.{field}_count={}", values.len())?;
    for (index, value) in values.iter().enumerate() {
        writeln!(f, "{prefix}.{field}.{index:04}={}", canonical_text(value))?;
    }
    Ok(())
}
