use super::*;
use civsim_ledger::Provenance;

#[test]
fn rejected_profile_fixture_preserves_the_proposed_dependency_cone_shape() {
    let projection =
        construct_unverified_candidate_fixture().expect("candidate fixture constructs");
    assert_eq!(projection.candidate_artifacts.len(), ARTIFACT_COUNT);
    assert_ne!(projection.member.0, [0; 32]);
    assert_eq!(projection.receipt.member, projection.member);
    assert_eq!(
        usize::try_from(projection.receipt.artifact_count).unwrap(),
        ARTIFACT_COUNT
    );
    assert!([
        projection.receipt.protocol.derive_first_status_id,
        projection.receipt.protocol.buckingham_pi_status_id,
        projection.receipt.protocol.gap_law_status_id,
        projection.receipt.protocol.chaos_protocol_status_id,
        projection.receipt.protocol.residual_law_status_id,
        projection.receipt.protocol.residual_slot_status_id,
    ]
    .into_iter()
    .all(|status| status == "assertion_only_not_admitted"));
    assert!(!projection.receipt.global_physical_vocabulary_coverage);
    assert!(!projection.receipt.membership_authority);
    assert_eq!(projection.receipt.authority_effect, "none");
    assert_ne!(projection.receipt.pair_receipt_sha256, [0; 32]);
    assert_ne!(projection.receipt.evidence_custody_receipt_sha256, [0; 32]);
    assert_eq!(projection.receipt.profile_id, PROFILE_ID);
    assert_eq!(projection.receipt.theory_class_id, THEORY_CLASS_ID);
    assert_eq!(projection.receipt.residual_slot_id, RESIDUAL_SLOT_ID);
    assert_eq!(
        projection.receipt.owner_admission_record,
        OWNER_ADMISSION_RECORD
    );
    assert_eq!(
        [
            projection.receipt.protocol.derive_first_status_id,
            projection.receipt.protocol.buckingham_pi_status_id,
            projection.receipt.protocol.gap_law_status_id,
            projection.receipt.protocol.chaos_protocol_status_id,
            projection.receipt.protocol.residual_law_status_id,
            projection.receipt.protocol.residual_slot_status_id,
        ],
        [
            DERIVE_FIRST_STATUS_ID,
            BUCKINGHAM_PI_STATUS_ID,
            GAP_LAW_STATUS_ID,
            CHAOS_PROTOCOL_STATUS_ID,
            RESIDUAL_LAW_STATUS_ID,
            RESIDUAL_SLOT_STATUS_ID,
        ]
    );
    assert!([
        projection.receipt.protocol.derivation_exhaustion_sha256,
        projection.receipt.protocol.buckingham_pi_sha256,
        projection.receipt.protocol.gap_law_sha256,
        projection.receipt.protocol.chaos_protocol_sha256,
        projection.receipt.protocol.residual_law_sha256,
        projection.receipt.protocol.residual_slot_sha256,
        projection.receipt.protocol.owner_admission_sha256,
        projection.receipt.protocol.independent_watchdog_sha256,
    ]
    .into_iter()
    .all(|digest| digest != [0; 32]));
    assert_eq!(
        projection
            .candidate_artifacts
            .iter()
            .filter(|artifact| artifact.admission.provenance == Provenance::Authored)
            .count(),
        1
    );
    assert_eq!(
        projection
            .candidate_artifacts
            .iter()
            .filter(|artifact| artifact.admission.provenance == Provenance::Derived)
            .count(),
        ARTIFACT_COUNT - 1
    );
}

#[test]
fn fixture_canary_transcripts_are_distinct_and_nonempty() {
    let projection =
        construct_unverified_candidate_fixture().expect("candidate fixture constructs");
    assert_ne!(
        projection.receipt.producer_canary.transcript_id,
        projection.receipt.watchdog_canary.transcript_id
    );
    assert_ne!(
        projection.receipt.producer_canary.transcript_sha256,
        projection.receipt.watchdog_canary.transcript_sha256
    );
    assert!(projection.receipt.producer_canary.case_count >= 23);
    assert!(projection.receipt.watchdog_canary.case_count >= 23);
}
