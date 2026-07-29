use super::*;
use civsim_ledger::Provenance;
use std::sync::OnceLock;

fn profile_projection() -> PrimitiveProfileProjection {
    static BASELINE: OnceLock<PrimitiveProfileProjection> = OnceLock::new();
    BASELINE
        .get_or_init(|| construct_profile_projection().expect("profile projection constructs"))
        .clone()
}

#[test]
fn profile_projection_binds_executed_admission_and_symmetry_evidence() {
    let projection = profile_projection();
    assert_eq!(projection.candidate_artifacts.len(), ARTIFACT_COUNT);
    assert_ne!(projection.member.0, [0; 32]);
    assert_eq!(projection.receipt.member, projection.member);
    assert_eq!(
        usize::try_from(projection.receipt.artifact_count).unwrap(),
        ARTIFACT_COUNT
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
    assert_ne!(projection.receipt.profile_role_identity.0, [0; 32]);
    assert_eq!(projection.receipt.symmetry_basis_element_count, 10);
    assert_eq!(projection.receipt.symmetry_excluded_operator_count, 1);
    assert_ne!(projection.receipt.symmetry_action_binding_sha256, [0; 32]);
    assert_ne!(
        projection.receipt.symmetry_applicability_receipt_sha256,
        [0; 32]
    );
    assert_ne!(projection.receipt.symmetry_validity_receipt_sha256, [0; 32]);
    assert_ne!(projection.receipt.derivation_catalog_sha256, [0; 32]);
    assert_ne!(projection.receipt.repository_catalog_sha256, [0; 32]);
    assert_ne!(projection.receipt.protocol_producer_result_sha256, [0; 32]);
    assert_ne!(
        projection.receipt.protocol_producer_result_sha256,
        projection.receipt.protocol_watchdog_result_sha256
    );
    assert_ne!(
        projection.receipt.derivation_coverage_capability_sha256,
        [0; 32]
    );
    assert_ne!(
        projection.receipt.irreducible_protocol_capability_sha256,
        [0; 32]
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
    let projection = profile_projection();
    assert_ne!(
        projection.receipt.producer_canary.transcript_id,
        projection.receipt.watchdog_canary.transcript_id
    );
    assert_ne!(
        projection.receipt.producer_canary.transcript_sha256,
        projection.receipt.watchdog_canary.transcript_sha256
    );
    assert!(projection.receipt.producer_canary.case_count >= 29);
    assert!(projection.receipt.watchdog_canary.case_count >= 29);
}
