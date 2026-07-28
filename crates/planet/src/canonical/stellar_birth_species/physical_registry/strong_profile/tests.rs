use super::*;
use civsim_ledger::{Provenance, Tier};

#[test]
fn independent_routes_agree_without_minting_a_member() {
    let producer_packet = producer::sealed_packet().expect("producer packet");
    let watchdog_packet = watchdog::sealed_packet().expect("watchdog packet");
    assert_eq!(producer_packet, watchdog_packet);

    let produced = producer::inspect(&producer_packet).expect("forward entailment");
    let watched = watchdog::inspect(&watchdog_packet).expect("reverse obligation");
    assert_eq!(produced, watched);
    assert_eq!(produced.candidates.len(), ARTIFACT_COUNT);
    assert_eq!(MEMBER_COUNT, 0);

    let projection = construct_profile_projection().expect("paired projection");
    assert_eq!(projection.receipt.artifact_count, ARTIFACT_COUNT as u32);
    assert_eq!(projection.receipt.member_count, 0);
    assert!(projection.receipt.confining_asymptotic_boundary_admitted);
    assert!(!projection.receipt.confinement_theorem_claim);
    assert!(!projection.receipt.membership_authority);
    assert!(!projection.receipt.carrier_species_membership);
    assert_eq!(projection.receipt.authority_effect, "none");
}

#[test]
fn root_is_one_irreducible_admission_and_all_consequences_are_derived() {
    let projection = construct_profile_projection().expect("paired projection");
    let authored = projection
        .candidate_artifacts
        .iter()
        .filter(|candidate| {
            candidate.admission.tier == Tier::Residue
                && candidate.admission.provenance == Provenance::Authored
                && matches!(candidate.admission.route, AdmissionRoute::Irreducible(_))
        })
        .count();
    let derived = projection
        .candidate_artifacts
        .iter()
        .filter(|candidate| {
            candidate.admission.tier == Tier::Residue
                && candidate.admission.provenance == Provenance::Derived
                && matches!(candidate.admission.route, AdmissionRoute::Derived(_))
        })
        .count();
    assert_eq!(authored, 1);
    assert_eq!(derived, ARTIFACT_COUNT - 1);
}

#[test]
fn slot_order_is_neutral_and_scope_escalations_are_refused() {
    let sealed = producer::sealed_packet().expect("sealed packet");
    let baseline = producer::inspect_against_sealed(&sealed, &sealed).expect("baseline");
    let mut reordered = sealed.clone();
    reordered.occupied_profile_slots.reverse();
    assert_eq!(
        producer::inspect_against_sealed(&reordered, &sealed).expect("forward reordered"),
        baseline
    );
    assert_eq!(
        watchdog::inspect_against_sealed(&reordered, &sealed).expect("reverse reordered"),
        baseline
    );

    for mutate in [
        |packet: &mut StrongProfilePacket| packet.membership_authority = true,
        |packet: &mut StrongProfilePacket| packet.carrier_species_membership = true,
        |packet: &mut StrongProfilePacket| packet.confinement_theorem_claim = true,
    ] {
        let mut mutant = sealed.clone();
        mutate(&mut mutant);
        assert_eq!(
            producer::inspect_against_sealed(&mutant, &sealed),
            Err(StrongProfileRefusal::UnsupportedScope)
        );
        assert_eq!(
            watchdog::inspect_against_sealed(&mutant, &sealed),
            Err(StrongProfileRefusal::UnsupportedScope)
        );
    }
}

#[test]
fn mutation_canaries_are_live_and_distinct() {
    let packet = producer::sealed_packet().expect("sealed packet");
    let forward = producer::canary_evidence(&packet).expect("forward canaries");
    let reverse = watchdog::canary_evidence(&packet).expect("reverse canaries");
    assert_eq!(forward.case_count, reverse.case_count);
    assert!(forward.case_count >= 20);
    assert_ne!(forward.transcript_id, reverse.transcript_id);
    assert_ne!(forward.transcript_sha256, [0; 32]);
    assert_ne!(reverse.transcript_sha256, [0; 32]);
    assert_ne!(forward.transcript_sha256, reverse.transcript_sha256);
}
