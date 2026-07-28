use super::*;
use civsim_ledger::Provenance;
use std::sync::OnceLock;

fn profile_projection() -> ChargedProfileProjection {
    static BASELINE: OnceLock<ChargedProfileProjection> = OnceLock::new();
    BASELINE
        .get_or_init(|| {
            construct_profile_projection().expect("charged profile projection constructs")
        })
        .clone()
}

#[test]
fn charged_profile_closes_one_orientation_invariant_pair() {
    let projection = profile_projection();
    assert_eq!(projection.candidate_artifacts.len(), ARTIFACT_COUNT);
    assert_eq!(projection.members.len(), MEMBER_COUNT);
    assert!(projection.members.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(projection.receipt.members, projection.members);
    assert_ne!(projection.receipt.pair_receipt_sha256, [0; 32]);
    assert_ne!(
        projection.receipt.charge_conjugation_producer_sha256,
        projection.receipt.charge_conjugation_watchdog_sha256
    );
    assert_ne!(
        projection.receipt.mass_transport_producer_sha256,
        projection.receipt.mass_transport_watchdog_sha256
    );
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
    assert!(!projection.receipt.global_physical_vocabulary_coverage);
    assert!(!projection.receipt.membership_authority);
    assert_eq!(projection.receipt.authority_effect, "none");
}

#[test]
fn charge_input_order_does_not_select_the_pair() {
    let mut packet = producer::sealed_packet().expect("sealed packet");
    let canonical = producer::inspect(&packet).expect("canonical orientation");
    packet.charge_weights.reverse();
    let reordered = producer::inspect(&packet).expect("reordered orientation");
    assert_eq!(canonical, reordered);
}

#[test]
fn independent_canary_transcripts_are_nonempty_and_distinct() {
    let projection = profile_projection();
    assert!(projection.receipt.producer_canary.case_count >= 30);
    assert_eq!(
        projection.receipt.producer_canary.case_count,
        projection.receipt.watchdog_canary.case_count
    );
    assert_ne!(
        projection.receipt.producer_canary.transcript_sha256,
        projection.receipt.watchdog_canary.transcript_sha256
    );
}
