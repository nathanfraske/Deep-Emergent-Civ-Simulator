use super::*;
use std::collections::BTreeSet;

fn id(tag: u8) -> [u8; 32] {
    let mut identity = [tag; 32];
    identity[0] = tag.wrapping_add(1);
    identity
}

fn request() -> TheoryProfileAdmissionRequest {
    TheoryProfileAdmissionRequest {
        claim_identity: id(151),
        role_identity: id(152),
        content_identity: id(153),
        profile_input_sha256: id(154),
        source_custody_sha256: id(155),
        applicability_receipt_sha256: id(156),
        validity_receipt_sha256: id(157),
        residual_slot_id: "planet.test.independent-theory-profile.v1".to_owned(),
        occupied_profile_slots: Vec::new(),
        owner_admission_record: "owner-reviewed-independent-profile-v1".to_owned(),
    }
}

#[test]
fn independent_pair_reconstructs_and_seals_the_same_profile_assessment() {
    let request = request();
    let produced = producer::prepare(&request).unwrap();
    let watched = watchdog::prepare(&request).unwrap();
    assert_eq!(produced.assessment, watched.assessment);
    assert_eq!(produced.canonical_bytes, watched.canonical_bytes);
    assert_ne!(produced.open_result_sha256, watched.open_result_sha256);
    assert_ne!(
        produced.protocol_result_sha256,
        watched.protocol_result_sha256
    );
    assert_ne!(produced.trace_sha256, watched.trace_sha256);

    let coverage_produced = producer::bind_coverage(&request, &produced, &watched).unwrap();
    let coverage_watched = watchdog::bind_coverage(&request, &produced, &watched).unwrap();
    assert_eq!(coverage_produced.capability, coverage_watched.capability);
    assert_eq!(
        coverage_produced.canonical_bytes,
        coverage_watched.canonical_bytes
    );
    assert_ne!(
        coverage_produced.result_sha256,
        coverage_watched.result_sha256
    );
    let protocol_produced =
        producer::bind_protocol(&request, &produced, &watched, coverage_produced.capability)
            .unwrap();
    let protocol_watched =
        watchdog::bind_protocol(&request, &produced, &watched, coverage_watched.capability)
            .unwrap();
    assert_eq!(protocol_produced.capability, protocol_watched.capability);
    assert_eq!(protocol_produced.receipts, protocol_watched.receipts);
    assert_eq!(
        protocol_produced.canonical_bytes,
        protocol_watched.canonical_bytes
    );
    assert_ne!(
        protocol_produced.result_sha256,
        protocol_watched.result_sha256
    );

    let evidence = inspect(&request).unwrap();
    assert_eq!(evidence.profile_protocol_schema_id, SCHEMA_ID);
    assert_eq!(evidence.profile_protocol_producer_id, PRODUCER_ID);
    assert_eq!(evidence.profile_protocol_watchdog_id, WATCHDOG_ID);
    assert_eq!(
        evidence.decision_id,
        "irreducible_protocol_structurally_bound"
    );
    assert_ne!(
        evidence.protocol_producer_result_sha256,
        evidence.protocol_watchdog_result_sha256
    );
    assert_ne!(
        evidence.open_producer_result_sha256,
        evidence.open_watchdog_result_sha256
    );
    assert_ne!(
        evidence.final_producer_result_sha256,
        evidence.final_watchdog_result_sha256
    );
    assert_ne!(
        evidence.profile_protocol_producer_trace_sha256,
        evidence.profile_protocol_watchdog_trace_sha256
    );
    assert_eq!(
        evidence.profile_protocol_producer_canary_transcript_id,
        producer::LIVE_CANARY_TRANSCRIPT_ID
    );
    assert_eq!(
        evidence.profile_protocol_watchdog_canary_transcript_id,
        watchdog::LIVE_CANARY_TRANSCRIPT_ID
    );
    assert_eq!(evidence.profile_protocol_producer_canary_case_count, 3);
    assert_eq!(
        evidence.profile_protocol_producer_canary_case_count,
        evidence.profile_protocol_watchdog_canary_case_count
    );
    assert_ne!(evidence.profile_protocol_producer_canary_sha256, [0; 32]);
    assert_ne!(evidence.profile_protocol_watchdog_canary_sha256, [0; 32]);
    assert_ne!(
        evidence.profile_protocol_producer_canary_sha256,
        evidence.profile_protocol_watchdog_canary_sha256
    );
    assert_ne!(evidence.profile_protocol_pair_receipt_sha256, [0; 32]);
    assert!(!evidence.premise_admission_authority);
    assert!(!evidence.species_membership_authority);
    assert!(!evidence.global_derivation_coverage);
    assert_eq!(evidence.authority_effect, "none");
}

#[test]
fn paired_receipts_are_distinct_and_bind_raw_request_substitution() {
    let base_request = request();
    let baseline = inspect(&base_request).unwrap();
    let receipts = [
        baseline.derivation_exhaustion_receipt_sha256,
        baseline.buckingham_pi_receipt_sha256,
        baseline.gap_law_receipt_sha256,
        baseline.chaos_protocol_receipt_sha256,
        baseline.residual_law_receipt_sha256,
        baseline.residual_slot_receipt_sha256,
        baseline.owner_admission_receipt_sha256,
        baseline.independent_watchdog_receipt_sha256,
        baseline.profile_protocol_pair_receipt_sha256,
    ];
    assert!(receipts.iter().all(|receipt| *receipt != [0; 32]));
    assert_eq!(
        receipts.into_iter().collect::<BTreeSet<_>>().len(),
        receipts.len()
    );

    let mut substituted = base_request;
    substituted.validity_receipt_sha256 = id(171);
    let observed = inspect(&substituted).unwrap();
    assert_ne!(
        baseline.profile_protocol_pair_receipt_sha256,
        observed.profile_protocol_pair_receipt_sha256
    );
    assert_ne!(
        baseline.irreducible_protocol_capability_sha256,
        observed.irreducible_protocol_capability_sha256
    );
    assert_ne!(
        baseline.profile_protocol_producer_canary_sha256,
        observed.profile_protocol_producer_canary_sha256
    );
    assert_ne!(
        baseline.profile_protocol_watchdog_canary_sha256,
        observed.profile_protocol_watchdog_canary_sha256
    );

    let mut target_substituted = request();
    target_substituted.role_identity = id(172);
    target_substituted.content_identity = id(173);
    let target_observed = inspect(&target_substituted).unwrap();
    for (before, after) in [
        (
            baseline.buckingham_pi_receipt_sha256,
            target_observed.buckingham_pi_receipt_sha256,
        ),
        (
            baseline.gap_law_receipt_sha256,
            target_observed.gap_law_receipt_sha256,
        ),
        (
            baseline.chaos_protocol_receipt_sha256,
            target_observed.chaos_protocol_receipt_sha256,
        ),
        (
            baseline.residual_law_receipt_sha256,
            target_observed.residual_law_receipt_sha256,
        ),
        (
            baseline.residual_slot_receipt_sha256,
            target_observed.residual_slot_receipt_sha256,
        ),
        (
            baseline.owner_admission_receipt_sha256,
            target_observed.owner_admission_receipt_sha256,
        ),
    ] {
        assert_ne!(before, after);
    }
}

#[test]
fn live_canaries_execute_each_real_validator_and_all_six_mutants_refuse() {
    let request = request();
    let produced = producer::prepare(&request).unwrap();
    let watched = watchdog::prepare(&request).unwrap();
    let coverage = producer::bind_coverage(&request, &produced, &watched)
        .unwrap()
        .capability;
    let protocol = producer::bind_protocol(&request, &produced, &watched, coverage)
        .unwrap()
        .capability;

    let producer_canary = producer::execute_live_canaries(&request, coverage).unwrap();
    let watchdog_canary =
        watchdog::execute_live_canaries(&request, &produced, &watched, coverage).unwrap();
    assert_eq!(producer_canary.case_count, 3);
    assert_eq!(producer_canary.case_count, watchdog_canary.case_count);
    assert_ne!(producer_canary.transcript_id, watchdog_canary.transcript_id);
    assert_ne!(
        producer_canary.transcript_sha256,
        watchdog_canary.transcript_sha256
    );

    let mut zero_claim_request = request.clone();
    zero_claim_request.claim_identity = [0; 32];
    assert_eq!(
        producer::prepare(&zero_claim_request),
        Err("invalid_theory_profile_admission_request")
    );
    let mut padded_slot_request = request.clone();
    padded_slot_request.residual_slot_id.insert(0, ' ');
    assert_eq!(
        producer::prepare(&padded_slot_request),
        Err("invalid_theory_profile_admission_request")
    );
    let mut invalid_producer_coverage = coverage;
    invalid_producer_coverage.capability_sha256[0] ^= 1;
    assert_eq!(
        producer::inspect_coverage(&request, invalid_producer_coverage),
        Err("theory_profile_protocol_coverage_route_refused")
    );

    let mut padded_owner_request = request.clone();
    padded_owner_request.owner_admission_record.push(' ');
    assert_eq!(
        watchdog::prepare(&padded_owner_request),
        Err("invalid_theory_profile_admission_request")
    );
    let mut aliased_receipt_request = request.clone();
    aliased_receipt_request.validity_receipt_sha256 =
        aliased_receipt_request.applicability_receipt_sha256;
    assert_eq!(
        watchdog::prepare(&aliased_receipt_request),
        Err("invalid_theory_profile_admission_request")
    );
    let mut invalid_watchdog_coverage = coverage;
    invalid_watchdog_coverage.watchdog_receipt_sha256[31] ^= 0x80;
    assert_eq!(
        watchdog::bind_protocol(&request, &produced, &watched, invalid_watchdog_coverage),
        Err("theory_profile_protocol_coverage_binding_invalid")
    );

    let final_produced = producer::inspect_final(&request, coverage, protocol).unwrap();
    assert_eq!(
        final_produced.evaluation.decision.id(),
        "irreducible_protocol_structurally_bound"
    );
}

#[test]
fn pair_receipt_binds_both_live_canary_transcripts_and_case_counts() {
    let base_fields = [b"fixed-pair-material".as_slice()];
    let producer_canary = LiveCanaryTranscript {
        transcript_id: producer::LIVE_CANARY_TRANSCRIPT_ID,
        case_count: 3,
        transcript_sha256: id(211),
    };
    let watchdog_canary = LiveCanaryTranscript {
        transcript_id: watchdog::LIVE_CANARY_TRANSCRIPT_ID,
        case_count: 3,
        transcript_sha256: id(212),
    };
    let baseline = seal_pair_receipt(&base_fields, producer_canary, watchdog_canary);

    let mut changed_digest = producer_canary;
    changed_digest.transcript_sha256[7] ^= 1;
    assert_ne!(
        baseline,
        seal_pair_receipt(&base_fields, changed_digest, watchdog_canary)
    );

    let mut changed_count = watchdog_canary;
    changed_count.case_count += 1;
    assert_ne!(
        baseline,
        seal_pair_receipt(&base_fields, producer_canary, changed_count)
    );
}

#[test]
fn occupied_slot_arrival_order_does_not_change_live_canaries_or_pair_receipt() {
    let mut forward_order = request();
    forward_order.occupied_profile_slots = vec![
        "planet.test.occupied-slot.alpha.v1".to_owned(),
        "planet.test.occupied-slot.beta.v1".to_owned(),
    ];
    let forward_evidence = inspect(&forward_order).unwrap();

    let mut reverse_order = forward_order;
    reverse_order.occupied_profile_slots.reverse();
    let reverse_evidence = inspect(&reverse_order).unwrap();

    assert_eq!(
        forward_evidence.profile_protocol_producer_canary_sha256,
        reverse_evidence.profile_protocol_producer_canary_sha256
    );
    assert_eq!(
        forward_evidence.profile_protocol_watchdog_canary_sha256,
        reverse_evidence.profile_protocol_watchdog_canary_sha256
    );
    assert_eq!(
        forward_evidence.profile_protocol_pair_receipt_sha256,
        reverse_evidence.profile_protocol_pair_receipt_sha256
    );
}

#[test]
fn malformed_and_colliding_requests_refuse_before_any_authority_effect() {
    let mut duplicate = request();
    duplicate.validity_receipt_sha256 = duplicate.applicability_receipt_sha256;
    assert_eq!(
        inspect(&duplicate),
        Err("invalid_theory_profile_admission_request")
    );

    let mut duplicate_slot = request();
    duplicate_slot.occupied_profile_slots =
        vec![" alien.slot ".to_owned(), "alien.slot".to_owned()];
    assert_eq!(
        inspect(&duplicate_slot),
        Err("invalid_theory_profile_admission_request")
    );

    let mut collision = request();
    collision
        .occupied_profile_slots
        .push(collision.residual_slot_id.clone());
    assert_eq!(
        inspect(&collision),
        Err("profile_protocol_residual_slot_collision")
    );

    let mut padded = request();
    padded.owner_admission_record = format!("{}x", " ".repeat(4_096));
    assert_eq!(
        inspect(&padded),
        Err("invalid_theory_profile_admission_request")
    );

    let mut padded_slot = request();
    padded_slot.occupied_profile_slots = vec![format!("{}slot", " ".repeat(4_096))];
    assert_eq!(
        inspect(&padded_slot),
        Err("invalid_theory_profile_admission_request")
    );
}

#[test]
fn unfamiliar_profile_identity_uses_the_same_protocol_without_familiar_selectors() {
    let baseline = inspect(&request()).unwrap();
    let mut alien = request();
    alien.claim_identity = id(181);
    alien.role_identity = id(182);
    alien.content_identity = id(183);
    alien.profile_input_sha256 = id(184);
    alien.residual_slot_id = "planet.test.unfamiliar-profile.v1".to_owned();
    let observed = inspect(&alien).unwrap();
    assert_eq!(observed.profile_protocol_schema_id, SCHEMA_ID);
    assert_eq!(observed.decision_id, baseline.decision_id);
    assert_ne!(
        observed.profile_protocol_pair_receipt_sha256,
        baseline.profile_protocol_pair_receipt_sha256
    );
    assert!(!observed.species_membership_authority);
    assert_eq!(observed.authority_effect, "none");
}
