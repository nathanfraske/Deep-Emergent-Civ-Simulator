use super::*;
use crate::canonical::stellar_birth_species::law_premise::{CandidateProof, PremiseCapabilitySeal};
use std::collections::BTreeSet;

fn id(tag: u8) -> [u8; 32] {
    let mut identity = [tag; 32];
    identity[0] = tag.wrapping_add(1);
    identity
}

fn key(role: u8, content: u8) -> PremiseKey {
    PremiseKey {
        role: SemanticRoleIdentity(id(role)),
        content: PhysicalContentIdentity(id(content)),
    }
}

fn seal_seed(mut seed: ClaimScopedPremiseCapability) -> ClaimScopedPremiseCapability {
    let produced = super::super::producer::capability_digest(&seed);
    let watched = super::super::watchdog::capability_digest(&seed);
    assert_eq!(produced, watched);
    seed.capability_sha256 = produced;
    seed
}

fn seed(claim: u8, role: u8, content: u8, receipt_base: u8) -> ClaimScopedPremiseCapability {
    seal_seed(ClaimScopedPremiseCapability {
        claim_identity: LawClaimIdentity(id(claim)),
        key: key(role, content),
        upstream_capability_sha256: id(receipt_base),
        semantic_producer_receipt_sha256: id(receipt_base.wrapping_add(1)),
        semantic_watchdog_receipt_sha256: id(receipt_base.wrapping_add(2)),
        applicability_receipt_sha256: id(receipt_base.wrapping_add(3)),
        validity_receipt_sha256: id(receipt_base.wrapping_add(4)),
        proof: CandidateProof::AdmittedContent,
        capability_sha256: [0; 32],
        _seal: PremiseCapabilitySeal,
    })
}

fn seal_rule(mut rule: VerifiedPremiseDerivationRule) -> VerifiedPremiseDerivationRule {
    let produced = producer::rule_capability_digest(&rule);
    let watched = watchdog::rule_capability_digest(&rule);
    assert_eq!(produced, watched);
    rule.capability_sha256 = produced;
    rule
}

fn rule(
    claim: u8,
    rule_id: u8,
    premises: Vec<PremiseKey>,
    conclusion: PremiseKey,
    receipt_base: u8,
) -> VerifiedPremiseDerivationRule {
    seal_rule(VerifiedPremiseDerivationRule {
        claim_identity: LawClaimIdentity(id(claim)),
        rule_identity: DerivationRuleIdentity(id(rule_id)),
        premises,
        conclusion,
        semantic_producer_receipt_sha256: id(receipt_base),
        semantic_watchdog_receipt_sha256: id(receipt_base.wrapping_add(1)),
        applicability_receipt_sha256: id(receipt_base.wrapping_add(2)),
        validity_receipt_sha256: id(receipt_base.wrapping_add(3)),
        capability_sha256: [0; 32],
        _seal: DerivationRuleSeal,
    })
}

fn input(
    target: PremiseKey,
    seeds: Vec<ClaimScopedPremiseCapability>,
    rules: Vec<VerifiedPremiseDerivationRule>,
) -> PremiseAdmissionRouteInput {
    PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(id(7)),
        target,
        admitted_seeds: seeds,
        admitted_rules: rules,
        derivation_coverage: None,
        irreducible_protocol: None,
    }
}

fn seal_coverage(mut coverage: DerivationCoverageCapability) -> DerivationCoverageCapability {
    let produced = producer::coverage_capability_digest(&coverage);
    let watched = watchdog::coverage_capability_digest(&coverage);
    assert_eq!(produced, watched);
    coverage.capability_sha256 = produced;
    coverage
}

fn coverage(target: PremiseKey, catalog_sha256: [u8; 32]) -> DerivationCoverageCapability {
    seal_coverage(DerivationCoverageCapability {
        claim_identity: LawClaimIdentity(id(7)),
        target,
        derivation_catalog_sha256: catalog_sha256,
        producer_receipt_sha256: id(80),
        watchdog_receipt_sha256: id(81),
        capability_sha256: [0; 32],
        _seal: DerivationCoverageSeal,
    })
}

fn seal_protocol(mut protocol: IrreducibleProtocolCapability) -> IrreducibleProtocolCapability {
    let produced = producer::protocol_capability_digest(&protocol);
    let watched = watchdog::protocol_capability_digest(&protocol);
    assert_eq!(produced, watched);
    protocol.capability_sha256 = produced;
    protocol
}

fn nondynamical_protocol(
    target: PremiseKey,
    coverage: DerivationCoverageCapability,
) -> IrreducibleProtocolCapability {
    seal_protocol(IrreducibleProtocolCapability {
        claim_identity: LawClaimIdentity(id(7)),
        target,
        derivation_coverage_capability_sha256: coverage.capability_sha256,
        buckingham_pi: BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256: id(90),
            producer_receipt_sha256: id(91),
            watchdog_receipt_sha256: id(92),
            residual_group_count: 1,
        },
        gap_law_receipt_sha256: id(93),
        chaos: ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256: id(94),
            watchdog_inapplicability_receipt_sha256: id(95),
        },
        residual_law_producer_receipt_sha256: id(96),
        residual_law_watchdog_receipt_sha256: id(97),
        residual_slot_identity: id(98),
        unique_slot_producer_receipt_sha256: id(99),
        unique_slot_watchdog_receipt_sha256: id(100),
        owner_admission_receipt_sha256: id(101),
        capability_sha256: [0; 32],
        _seal: IrreducibleProtocolSeal,
    })
}

fn derivable_fixture() -> PremiseAdmissionRouteInput {
    let first = key(10, 20);
    let second = key(11, 21);
    let intermediate = key(12, 22);
    let target = key(13, 23);
    input(
        target,
        vec![seed(7, 10, 20, 30), seed(7, 11, 21, 40)],
        vec![
            rule(7, 50, vec![first], intermediate, 60),
            rule(7, 51, vec![intermediate, second], target, 70),
        ],
    )
}

#[test]
fn repository_frontier_reports_the_derived_seed_and_completed_local_profile_protocol() {
    let frontier = repository_premise_admission_frontier().unwrap();
    assert_eq!(
        frontier.schema_id,
        "civsim.planet.law-premise-derive-first-route.v1"
    );
    assert_eq!(frontier.derived_route_count, 1);
    assert_eq!(frontier.derived_decision_id, "derived");
    assert_ne!(frontier.producer_result_sha256, [0; 32]);
    assert_eq!(
        frontier.producer_result_sha256,
        frontier.watchdog_result_sha256
    );
    assert_eq!(frontier.current_descriptor_role_count, 88);
    assert_eq!(frontier.current_relation_target_count, 105);
    assert_eq!(frontier.current_constraint_law_count, 4);
    assert_eq!(frontier.next_target_decision_id, "next_target_not_bound");
    assert!(!frontier.derivation_frontier_complete);
    assert!(frontier.irreducible_protocol_started);
    assert!(!frontier.premise_admission_authority);
    assert!(!frontier.species_membership_authority);
    assert_eq!(frontier.authority_effect, "none");
}

#[test]
fn next_target_blocker_tracks_each_missing_capability_class() {
    assert_eq!(
        next_target_decision(0, 0, 0),
        "no_admitted_semantic_target_role"
    );
    assert_eq!(
        next_target_decision(1, 0, 0),
        "no_admitted_semantic_target_content"
    );
    assert_eq!(
        next_target_decision(1, 1, 0),
        "no_admitted_premise_derivation_rule"
    );
    assert_eq!(next_target_decision(1, 1, 1), "next_target_not_bound");
}

#[test]
fn repository_theory_profile_executes_the_full_irreducible_route() {
    let request = TheoryProfileAdmissionRequest {
        claim_identity: id(151),
        role_identity: id(152),
        content_identity: id(153),
        profile_input_sha256: id(154),
        source_custody_sha256: id(155),
        applicability_receipt_sha256: id(156),
        validity_receipt_sha256: id(157),
        residual_slot_id: "planet.test.unfamiliar-theory-profile.v1".to_owned(),
        occupied_profile_slots: Vec::new(),
        owner_admission_record: "owner-reviewed-test-profile-v1".to_owned(),
    };
    let evidence = inspect_theory_profile_admission(&request).unwrap();
    assert_eq!(
        evidence.decision_id,
        "irreducible_protocol_structurally_bound"
    );
    assert_eq!(evidence.target_claim_identity, request.claim_identity);
    assert_eq!(evidence.target_role_identity, request.role_identity);
    assert_eq!(evidence.target_content_identity, request.content_identity);
    assert_eq!(evidence.seed_count, 0);
    assert_eq!(evidence.rule_count, 0);
    assert_ne!(evidence.derivation_catalog_sha256, [0; 32]);
    assert_ne!(evidence.repository_catalog_sha256, [0; 32]);
    assert_ne!(evidence.protocol_producer_result_sha256, [0; 32]);
    assert_eq!(
        evidence.protocol_producer_result_sha256,
        evidence.protocol_watchdog_result_sha256
    );
    assert_ne!(evidence.open_producer_result_sha256, [0; 32]);
    assert_eq!(
        evidence.open_producer_result_sha256,
        evidence.open_watchdog_result_sha256
    );
    assert_ne!(evidence.final_producer_result_sha256, [0; 32]);
    assert_eq!(
        evidence.final_producer_result_sha256,
        evidence.final_watchdog_result_sha256
    );
    assert_ne!(evidence.derivation_coverage_capability_sha256, [0; 32]);
    assert_ne!(evidence.irreducible_protocol_capability_sha256, [0; 32]);
    let receipts = [
        evidence.derivation_exhaustion_receipt_sha256,
        evidence.buckingham_pi_receipt_sha256,
        evidence.gap_law_receipt_sha256,
        evidence.chaos_protocol_receipt_sha256,
        evidence.residual_law_receipt_sha256,
        evidence.residual_slot_receipt_sha256,
        evidence.owner_admission_receipt_sha256,
        evidence.independent_watchdog_receipt_sha256,
    ];
    assert!(receipts.iter().all(|receipt| *receipt != [0; 32]));
    assert_eq!(
        receipts.iter().copied().collect::<BTreeSet<_>>().len(),
        receipts.len()
    );

    let mut collision = request.clone();
    collision
        .occupied_profile_slots
        .push(collision.residual_slot_id.clone());
    assert_eq!(
        inspect_theory_profile_admission(&collision),
        Err("profile_protocol_residual_slot_collision")
    );

    let mut alien = request;
    alien.claim_identity = id(161);
    alien.role_identity = id(162);
    alien.content_identity = id(163);
    alien.residual_slot_id = "planet.test.alien-theory-profile.v1".to_owned();
    let alien_evidence = inspect_theory_profile_admission(&alien).unwrap();
    assert_eq!(
        alien_evidence.decision_id,
        "irreducible_protocol_structurally_bound"
    );
    assert_ne!(
        alien_evidence.irreducible_protocol_capability_sha256,
        evidence.irreducible_protocol_capability_sha256
    );
}

#[test]
fn bounded_rule_closure_derives_an_unfamiliar_target_with_exact_ancestry() {
    let fixture = derivable_fixture();
    let report = inspect_premise_admission_route(&fixture).unwrap();
    assert_eq!(report.decision_id(), "derived");
    let PremiseAdmissionDecision::Derived(witness) = &report.evaluation.decision else {
        panic!("target should derive")
    };
    assert_eq!(witness.target, fixture.target);
    assert_eq!(witness.seed_capability_sha256.len(), 2);
    assert_eq!(witness.rule_capability_sha256.len(), 2);
    assert!(!report.premise_admission_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn arrival_order_does_not_change_the_canonical_route() {
    let fixture = derivable_fixture();
    let baseline = inspect_premise_admission_route(&fixture).unwrap();
    let mut permuted = fixture;
    permuted.admitted_seeds.reverse();
    permuted.admitted_rules.reverse();
    for rule in &mut permuted.admitted_rules {
        rule.premises.reverse();
    }
    let observed = inspect_premise_admission_route(&permuted).unwrap();
    assert_eq!(baseline, observed);
}

#[test]
fn unrelated_alien_extension_preserves_the_existing_witness() {
    let fixture = derivable_fixture();
    let baseline = inspect_premise_admission_route(&fixture).unwrap();
    let mut extended = fixture;
    extended.admitted_seeds.push(seed(7, 200, 201, 120));
    extended
        .admitted_rules
        .push(rule(7, 202, vec![key(200, 201)], key(203, 204), 130));
    let observed = inspect_premise_admission_route(&extended).unwrap();
    assert_eq!(baseline, observed);
}

#[test]
fn unrelated_unreachable_alien_island_preserves_an_open_target_frontier() {
    let target = key(13, 23);
    let fixture = input(
        target,
        vec![seed(7, 10, 20, 30)],
        vec![rule(7, 50, vec![key(10, 20), key(11, 21)], target, 60)],
    );
    let baseline = inspect_premise_admission_route(&fixture).unwrap();
    let mut extended = fixture;
    extended.admitted_seeds.push(seed(7, 200, 201, 120));
    extended
        .admitted_rules
        .push(rule(7, 202, vec![key(203, 204)], key(205, 206), 130));
    let observed = inspect_premise_admission_route(&extended).unwrap();
    assert_eq!(baseline, observed);
}

#[test]
fn semantic_preflight_covers_the_full_synchronous_closure_bound() {
    let exact = producer::preflight_work(999, 1_001).unwrap();
    assert_eq!(exact, MAX_WORK_UNITS);
    assert_eq!(watchdog::preflight_work(999, 1_001).unwrap(), exact);

    let over = producer::preflight_work(999, 1_002).unwrap();
    assert_eq!(over, MAX_WORK_UNITS + 1_000);
    assert_eq!(watchdog::preflight_work(999, 1_002).unwrap(), over);
}

#[test]
fn failed_search_remains_an_open_frontier_without_completeness_authority() {
    let target = key(13, 23);
    let fixture = input(
        target,
        vec![seed(7, 10, 20, 30)],
        vec![rule(7, 50, vec![key(10, 20), key(11, 21)], target, 60)],
    );
    let report = inspect_premise_admission_route(&fixture).unwrap();
    let PremiseAdmissionDecision::OpenDerivationFrontier(frontier) = &report.evaluation.decision
    else {
        panic!("an incomplete search must remain open")
    };
    assert_eq!(frontier.target, target);
    assert_eq!(frontier.reachable, vec![key(10, 20)]);
    assert_eq!(frontier.unresolved_dependencies, vec![key(11, 21)]);
    assert!(!report.global_derivation_coverage());
}

#[test]
fn exact_role_and_content_prevent_shape_or_sole_candidate_fallback() {
    let target = key(13, 23);
    for wrong_seed in [seed(7, 14, 23, 30), seed(7, 13, 24, 40)] {
        let report =
            inspect_premise_admission_route(&input(target, vec![wrong_seed], vec![])).unwrap();
        assert_eq!(report.decision_id(), "open_derivation_frontier");
    }
}

#[test]
fn complete_derivation_coverage_is_required_before_irreducibility() {
    let target = key(13, 23);
    let mut fixture = input(target, vec![seed(7, 10, 20, 30)], vec![]);
    let open = inspect_premise_admission_route(&fixture).unwrap();
    let closed = coverage(target, open.evaluation.derivation_catalog_sha256);
    fixture.derivation_coverage = Some(closed);
    let report = inspect_premise_admission_route(&fixture).unwrap();
    assert_eq!(report.decision_id(), "irreducible_protocol_required");
    assert!(!report.premise_admission_authority());
}

#[test]
fn complete_typed_protocol_reaches_structural_binding_without_minting_authority() {
    let target = key(13, 23);
    let mut fixture = input(target, vec![seed(7, 10, 20, 30)], vec![]);
    let open = inspect_premise_admission_route(&fixture).unwrap();
    let closed = coverage(target, open.evaluation.derivation_catalog_sha256);
    fixture.derivation_coverage = Some(closed);
    fixture.irreducible_protocol = Some(nondynamical_protocol(target, closed));
    let report = inspect_premise_admission_route(&fixture).unwrap();
    assert_eq!(
        report.decision_id(),
        "irreducible_protocol_structurally_bound"
    );
    assert!(!report.premise_admission_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn semantic_pi_inapplicability_and_dynamical_chaos_are_explicit_branches() {
    let target = key(13, 23);
    let fixture = input(target, vec![seed(7, 10, 20, 30)], vec![]);
    let open = inspect_premise_admission_route(&fixture).unwrap();
    let closed = coverage(target, open.evaluation.derivation_catalog_sha256);
    let protocol = seal_protocol(IrreducibleProtocolCapability {
        claim_identity: LawClaimIdentity(id(7)),
        target,
        derivation_coverage_capability_sha256: closed.capability_sha256,
        buckingham_pi: BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256: id(90),
            watchdog_receipt_sha256: id(91),
        },
        gap_law_receipt_sha256: id(92),
        chaos: ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256: id(93),
            regime_partition_watchdog_receipt_sha256: id(94),
            transition_law_receipt_sha256: id(95),
        },
        residual_law_producer_receipt_sha256: id(96),
        residual_law_watchdog_receipt_sha256: id(97),
        residual_slot_identity: id(98),
        unique_slot_producer_receipt_sha256: id(99),
        unique_slot_watchdog_receipt_sha256: id(100),
        owner_admission_receipt_sha256: id(101),
        capability_sha256: [0; 32],
        _seal: IrreducibleProtocolSeal,
    });
    let mut ready = fixture;
    ready.derivation_coverage = Some(closed);
    ready.irreducible_protocol = Some(protocol);
    assert_eq!(
        inspect_premise_admission_route(&ready)
            .unwrap()
            .decision_id(),
        "irreducible_protocol_structurally_bound"
    );
}

#[test]
fn stale_catalog_and_duplicate_protocol_evidence_refuse_on_both_paths() {
    let target = key(13, 23);
    let mut fixture = input(target, vec![seed(7, 10, 20, 30)], vec![]);
    let open = inspect_premise_admission_route(&fixture).unwrap();
    let mut stale = coverage(target, open.evaluation.derivation_catalog_sha256);
    stale.derivation_catalog_sha256[0] ^= 1;
    fixture.derivation_coverage = Some(stale);
    assert_eq!(
        producer::route(&fixture),
        Err(PremiseAdmissionRefusal::DerivationCoverageCapabilityInvalid)
    );
    assert_eq!(producer::route(&fixture), watchdog::route(&fixture));

    let closed = coverage(target, open.evaluation.derivation_catalog_sha256);
    let mut protocol = nondynamical_protocol(target, closed);
    protocol.unique_slot_watchdog_receipt_sha256 = protocol.unique_slot_producer_receipt_sha256;
    protocol = seal_protocol(protocol);
    fixture.derivation_coverage = Some(closed);
    fixture.irreducible_protocol = Some(protocol);
    assert_eq!(
        producer::route(&fixture),
        Err(PremiseAdmissionRefusal::DuplicateProtocolReceipt)
    );
    assert_eq!(producer::route(&fixture), watchdog::route(&fixture));

    let mut reused_coverage_receipt = nondynamical_protocol(target, closed);
    reused_coverage_receipt.gap_law_receipt_sha256 = closed.producer_receipt_sha256;
    reused_coverage_receipt = seal_protocol(reused_coverage_receipt);
    fixture.irreducible_protocol = Some(reused_coverage_receipt);
    assert_eq!(
        producer::route(&fixture),
        Err(PremiseAdmissionRefusal::DuplicateProtocolReceipt)
    );
    assert_eq!(producer::route(&fixture), watchdog::route(&fixture));
}

#[test]
fn protocol_without_coverage_and_protocol_for_a_derived_target_refuse() {
    let target = key(13, 23);
    let fixture = input(target, vec![seed(7, 10, 20, 30)], vec![]);
    let open = inspect_premise_admission_route(&fixture).unwrap();
    let closed = coverage(target, open.evaluation.derivation_catalog_sha256);
    let protocol = nondynamical_protocol(target, closed);

    let mut missing_coverage = fixture.clone();
    missing_coverage.irreducible_protocol = Some(protocol);
    assert_eq!(
        inspect_premise_admission_route(&missing_coverage),
        Err(PremiseAdmissionRefusal::IrreducibleProtocolWithoutCoverage)
    );

    let mut derived = input(target, vec![seed(7, 13, 23, 30)], vec![]);
    let derived_catalog = inspect_premise_admission_route(&derived)
        .unwrap()
        .evaluation
        .derivation_catalog_sha256;
    let derived_coverage = coverage(target, derived_catalog);
    derived.derivation_coverage = Some(derived_coverage);
    derived.irreducible_protocol = Some(nondynamical_protocol(target, derived_coverage));
    assert_eq!(
        inspect_premise_admission_route(&derived),
        Err(PremiseAdmissionRefusal::IrreducibleProtocolCapabilityInvalid)
    );
}

#[test]
fn cycles_do_not_create_premises_and_independent_algorithms_agree() {
    let left = key(10, 20);
    let right = key(11, 21);
    let fixture = input(
        left,
        vec![],
        vec![
            rule(7, 50, vec![right], left, 60),
            rule(7, 51, vec![left], right, 70),
        ],
    );
    let produced = producer::route(&fixture);
    let watched = watchdog::route(&fixture);
    assert_eq!(produced, watched);
    assert_eq!(
        inspect_premise_admission_route(&fixture)
            .unwrap()
            .decision_id(),
        "open_derivation_frontier"
    );
}

#[test]
fn cyclic_rule_cannot_mask_a_grounded_canonical_witness() {
    let source = key(10, 20);
    let target = key(11, 21);
    let cyclic = rule(7, 40, vec![target], target, 50);
    let grounded = rule(7, 41, vec![source], target, 60);
    let fixture = input(
        target,
        vec![seed(7, 10, 20, 30)],
        vec![cyclic, grounded.clone()],
    );
    let report = inspect_premise_admission_route(&fixture).unwrap();
    let PremiseAdmissionDecision::Derived(witness) = report.evaluation.decision else {
        panic!("the grounded rule must derive the target")
    };
    assert_eq!(witness.seed_capability_sha256.len(), 1);
    assert_eq!(
        witness.rule_capability_sha256,
        vec![grounded.capability_sha256]
    );
}

#[test]
fn malformed_duplicate_and_capacity_inputs_fail_closed() {
    let target = key(13, 23);
    let mut invalid_claim = input(target, vec![], vec![]);
    invalid_claim.claim_identity = LawClaimIdentity([0; 32]);
    assert_eq!(
        inspect_premise_admission_route(&invalid_claim),
        Err(PremiseAdmissionRefusal::InvalidClaimIdentity)
    );

    let duplicate_seed = seed(7, 10, 20, 30);
    assert_eq!(
        inspect_premise_admission_route(&input(
            target,
            vec![duplicate_seed, duplicate_seed],
            vec![]
        )),
        Err(PremiseAdmissionRefusal::DuplicateSeed)
    );

    let template = rule(7, 50, vec![key(10, 20)], target, 60);
    let oversized = PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(id(7)),
        target,
        admitted_seeds: vec![],
        admitted_rules: vec![template; MAX_RULE_COUNT + 1],
        derivation_coverage: None,
        irreducible_protocol: None,
    };
    assert_eq!(
        inspect_premise_admission_route(&oversized),
        Err(PremiseAdmissionRefusal::RuleCapacityExceeded)
    );
}

#[test]
fn producer_and_watchdog_agree_over_small_unfamiliar_graphs() {
    for seed_role in 1..=3 {
        for target_role in 1..=3 {
            let source = key(seed_role, 10 + seed_role);
            let target = key(target_role, 20 + target_role);
            let rules = if source == target {
                vec![]
            } else {
                vec![rule(7, 50, vec![source], target, 60)]
            };
            let fixture = input(target, vec![seed(7, seed_role, 10 + seed_role, 30)], rules);
            assert_eq!(producer::route(&fixture), watchdog::route(&fixture));
        }
    }
}
