use super::{
    finish_pair, inspect,
    model::{
        ConfiningConstituentDecision, ConfiningConstituentObligation,
        ConfiningConstituentRefusalCode, InternalConstituentSeed, MAX_INTERNAL_SEED_COUNT,
    },
    producer, repository_frontier, repository_input, watchdog,
};
use crate::canonical::stellar_birth_species::physical_registry::{
    model::ArtifactIdentity, strong_profile,
};
use civsim_units::digest::sha256;

const ALL_OBLIGATIONS: [ConfiningConstituentObligation; 8] = [
    ConfiningConstituentObligation::RepresentationFamilyMembershipClosure,
    ConfiningConstituentObligation::ConfiningDynamicsScaleOrEquivalent,
    ConfiningConstituentObligation::ExcitationProfileCoverage,
    ConfiningConstituentObligation::ExactRestMassOrMasslessProof,
    ConfiningConstituentObligation::ChargeStateStatisticsDispositionCoverage,
    ConfiningConstituentObligation::ConstituentApplicabilityValidity,
    ConfiningConstituentObligation::ConstituentConservationBinding,
    ConfiningConstituentObligation::TransitionSeparationChannelCoverage,
];

fn identity(label: &str) -> ArtifactIdentity {
    ArtifactIdentity(sha256(label.as_bytes()))
}

fn production_input() -> super::model::ConfiningConstituentInput {
    let strong = strong_profile::construct_profile_projection()
        .expect("strong profile")
        .receipt;
    repository_input(&strong).expect("frontier input")
}

#[test]
fn repository_frontier_exposes_one_internal_seed_and_no_constituent_candidate() {
    let strong = strong_profile::construct_profile_projection()
        .expect("strong profile")
        .receipt;
    let report = repository_frontier(&strong).expect("typed open frontier");
    assert_eq!(report.decision_id, "blocked_open_proofs");
    assert_eq!(
        report.internal_seed_identities,
        vec![strong.carrier_identity]
    );
    assert_eq!(report.missing_authority_ids.len(), ALL_OBLIGATIONS.len());
    assert_eq!(report.constituent_candidate_count, 0);
    assert!(!report.membership_authority);
    assert!(!report.spectrum_authority);
    assert_eq!(report.authority_effect, "none");
    assert_ne!(report.producer_result_sha256, report.watchdog_result_sha256);
    assert_ne!(report.producer_trace_sha256, report.watchdog_trace_sha256);
    assert_ne!(report.receipt_sha256, [0; 32]);
}

#[test]
fn every_gap_stays_open_until_a_real_upstream_authority_pair_exists() {
    let report = inspect(&production_input()).expect("typed open decision");
    assert_eq!(
        report.missing_authority_ids,
        ALL_OBLIGATIONS
            .into_iter()
            .map(ConfiningConstituentObligation::id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn input_arrival_order_cannot_change_the_decision_or_receipt() {
    let mut input = production_input();
    for label in ["alien.internal-role.a", "alien.internal-role.b"] {
        input.internal_seeds.push(InternalConstituentSeed {
            identity: identity(label),
            source_profile_receipt_sha256: input.source_profile_receipt_sha256,
        });
    }
    let baseline = inspect(&input).expect("baseline");
    input.internal_seeds.reverse();
    assert_eq!(inspect(&input).expect("permuted"), baseline);
}

#[test]
fn unfamiliar_and_thaumic_seed_identities_are_extension_monotone() {
    let input = production_input();
    let baseline = inspect(&input).expect("baseline");
    let mut extended = input.clone();
    for label in [
        "unfamiliar.internal-role.with-no-familiar-name",
        "thaumic.internal-role.obeying-the-same-law-boundary",
    ] {
        extended.internal_seeds.push(InternalConstituentSeed {
            identity: identity(label),
            source_profile_receipt_sha256: extended.source_profile_receipt_sha256,
        });
    }
    let report = inspect(&extended).expect("alien-safe extension");
    assert_eq!(report.decision_id, baseline.decision_id);
    assert_eq!(report.missing_authority_ids, baseline.missing_authority_ids);
    assert_eq!(report.constituent_candidate_count, 0);
    assert_eq!(report.internal_seed_identities.len(), 3);
}

#[test]
fn reserved_source_role_aliases_fail_closed_in_both_paths() {
    let input = production_input();
    for reserved in [
        input.profile_root_identity,
        input.sector_identity,
        input.constraint_law_identity,
    ] {
        let mut aliased = input.clone();
        aliased.internal_seeds.push(InternalConstituentSeed {
            identity: reserved,
            source_profile_receipt_sha256: aliased.source_profile_receipt_sha256,
        });
        assert_eq!(
            producer::inspect(&aliased),
            Err(ConfiningConstituentRefusalCode::IdentityAlias)
        );
        assert_eq!(
            watchdog::inspect(&aliased),
            Err(ConfiningConstituentRefusalCode::IdentityAlias)
        );
    }
}

#[test]
fn substituted_seed_receipt_fails_closed_in_both_paths() {
    let mut substituted = production_input();
    substituted.internal_seeds[0].source_profile_receipt_sha256 = [9; 32];
    assert_eq!(
        producer::inspect(&substituted),
        Err(ConfiningConstituentRefusalCode::InternalSeedBindingMismatch)
    );
    assert_eq!(
        watchdog::inspect(&substituted),
        Err(ConfiningConstituentRefusalCode::InternalSeedBindingMismatch)
    );
}

#[test]
fn missing_carrier_seed_and_resource_growth_fail_closed() {
    let mut missing_carrier = production_input();
    missing_carrier.internal_seeds[0].identity = identity("not-the-carrier");
    assert_eq!(
        inspect(&missing_carrier),
        Err(ConfiningConstituentRefusalCode::CarrierSeedMissing)
    );

    let mut oversized = production_input();
    oversized.internal_seeds = (0..=MAX_INTERNAL_SEED_COUNT)
        .map(|index| InternalConstituentSeed {
            identity: identity(&format!("seed-{index}")),
            source_profile_receipt_sha256: oversized.source_profile_receipt_sha256,
        })
        .collect();
    assert_eq!(
        inspect(&oversized),
        Err(ConfiningConstituentRefusalCode::InternalSeedCapacityExceeded)
    );
}

#[test]
fn outer_pair_refuses_checker_disagreement() {
    let input = production_input();
    let produced = producer::inspect(&input).expect("producer");
    let mut watched = watchdog::inspect(&input).expect("watchdog");
    watched.evaluation.decision = ConfiningConstituentDecision::BlockedOpenProofs {
        missing: ALL_OBLIGATIONS[1..].to_vec(),
    };
    assert_eq!(
        finish_pair(Ok(produced), Ok(watched)),
        Err(ConfiningConstituentRefusalCode::CheckerDisagreement)
    );
}
