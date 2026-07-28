use super::*;
use std::cmp::Ordering;

#[test]
fn independent_pair_agrees_on_one_exact_neutral_bound_member() {
    let producer_packet = producer::sealed_packet().unwrap();
    let watchdog_packet = watchdog::sealed_packet().unwrap();
    assert_eq!(producer_packet, watchdog_packet);

    let produced = producer::inspect(&producer_packet).unwrap();
    let watched = watchdog::inspect(&watchdog_packet).unwrap();
    assert_eq!(produced, watched);
    assert_eq!(produced.candidates.len(), ARTIFACT_COUNT);
    assert_eq!(
        produced
            .dimensionless_ground_energy
            .cmp_rat(&BigRat::from_i64(-1).div(&BigRat::from_i64(2))),
        Ordering::Equal
    );
    assert_eq!(
        produced
            .binding_mass_factor
            .cmp_rat(&BigRat::from_i64(1).div(&BigRat::from_i64(4))),
        Ordering::Equal
    );
    assert_eq!(
        produced.normalization_value.cmp_rat(&BigRat::from_i64(1)),
        Ordering::Equal
    );
    assert_eq!(
        produced.radial_residual_norm.cmp_rat(&BigRat::from_i64(0)),
        Ordering::Equal
    );
    assert_eq!(
        produced
            .candidate_mass_interval
            .upper
            .cmp_rat(&produced.constituent_threshold_interval.lower),
        Ordering::Less
    );
    assert_eq!(
        produced
            .candidate_mass_interval
            .lower
            .cmp_rat(&produced.decay_threshold_interval.upper),
        Ordering::Greater
    );
}

#[test]
fn projection_keeps_constituent_binding_separate_from_open_decay() {
    let projection = construct_profile_projection().unwrap();
    assert_eq!(projection.candidate_artifacts.len(), ARTIFACT_COUNT);
    assert_eq!(projection.receipt.artifact_count, ARTIFACT_COUNT as u32);
    assert_eq!(
        projection.receipt.binding_disposition_id,
        "strictly_below_free_constituent_threshold"
    );
    assert_eq!(
        projection.receipt.decay_disposition_id,
        "energetically_open_neutral_massless_carrier_family"
    );
    assert!(!projection.receipt.global_stability_claim);
    assert!(!projection.receipt.conditioned_support_authority);
    assert!(!projection.receipt.membership_authority);
    assert_eq!(projection.receipt.authority_effect, "none");
}

#[test]
fn both_live_canary_suites_cover_the_same_nonzero_case_count() {
    let packet = producer::sealed_packet().unwrap();
    let produced = producer::canary_evidence(&packet).unwrap();
    let watched = watchdog::canary_evidence(&packet).unwrap();
    assert_eq!(produced.case_count, watched.case_count);
    assert!(produced.case_count >= 18);
    assert_ne!(produced.transcript_id, watched.transcript_id);
    assert_ne!(produced.transcript_sha256, watched.transcript_sha256);
}

#[test]
fn exact_threshold_capability_does_not_mint_authority() {
    let projection = construct_profile_projection().unwrap();
    let produced = producer::inspect(&producer::sealed_packet().unwrap()).unwrap();
    assert_eq!(projection.receipt.member, produced.member);
    assert_eq!(projection.receipt.authority_effect, "none");
    assert_ne!(projection.receipt.pair_receipt_sha256, [0; 32]);
}
