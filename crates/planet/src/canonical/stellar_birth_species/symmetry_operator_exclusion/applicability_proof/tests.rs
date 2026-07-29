use super::*;

fn identity(number: u64) -> AlgebraIdentity {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&number.to_be_bytes());
    bytes[31] = 1;
    AlgebraIdentity(bytes)
}

fn fact(number: u64) -> ScopeRulePremise {
    ScopeRulePremise::Fact(identity(number))
}

fn rule(premises: Vec<ScopeRulePremise>, conclusion: u64) -> ScopeInferenceRule {
    ScopeInferenceRule {
        premises,
        conclusion: identity(conclusion),
    }
}

fn input() -> ScopeProofInput {
    ScopeProofInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        action_binding_fact: identity(4),
        premise_facts: vec![identity(10)],
        inference_rules: vec![
            rule(vec![ScopeRulePremise::EvaluatedAction, fact(10)], 20),
            rule(vec![fact(20)], 30),
            rule(vec![fact(20)], 40),
        ],
        required_applicability_fact: identity(30),
        required_validity_fact: identity(40),
    }
}

fn assert_both_refuse(input: &ScopeProofInput, expected: ScopeProofRefusal) {
    assert_eq!(producer::prove(input), Err(expected));
    assert_eq!(watchdog::prove(input), Err(expected));
    assert_eq!(inspect_scope_proof(input), Err(expected));
}

#[test]
fn independent_algorithms_prove_both_facts_through_the_exact_action() {
    let report = inspect_scope_proof(&input()).unwrap();
    assert_eq!(report.producer_id, PRODUCER_ID);
    assert_eq!(report.watchdog_id, WATCHDOG_ID);
    assert_ne!(report.producer_id, report.watchdog_id);
    assert_eq!(report.producer_result_sha256, report.watchdog_result_sha256);
    assert_ne!(report.producer_result_sha256, [0; 32]);
    assert_eq!(report.reachable_fact_count(), 4);
    assert_eq!(report.action_binding_fact(), identity(4));
    assert!(report.conditional_applicability_proved());
    assert!(report.conditional_validity_proved());
    assert!(report.action_dependency_proved());
    assert!(!report.premise_admission_authority());
    assert!(!report.applicability_authority());
    assert!(!report.validity_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn arrival_order_and_rule_premise_order_are_noncausal() {
    let mut baseline = input();
    baseline.premise_facts.push(identity(11));
    let expected = inspect_scope_proof(&baseline).unwrap();

    let mut permuted = baseline;
    permuted.premise_facts.reverse();
    permuted.inference_rules.reverse();
    permuted.inference_rules[2].premises.reverse();
    let found = inspect_scope_proof(&permuted).unwrap();
    assert_eq!(found, expected);
}

#[test]
fn unrelated_alien_extension_preserves_the_required_proofs() {
    let baseline = inspect_scope_proof(&input()).unwrap();
    let mut extended = input();
    extended.premise_facts.push(identity(200));
    extended.inference_rules.push(rule(vec![fact(200)], 201));
    let extended = inspect_scope_proof(&extended).unwrap();
    assert!(extended.conditional_applicability_proved());
    assert!(extended.conditional_validity_proved());
    assert!(extended.action_dependency_proved());
    assert_eq!(
        extended.reachable_fact_count(),
        baseline.reachable_fact_count() + 2
    );
}

#[test]
fn supported_cycles_close_only_when_the_action_reaches_them() {
    let mut cyclic = input();
    cyclic.inference_rules = vec![
        rule(vec![ScopeRulePremise::EvaluatedAction, fact(10)], 20),
        rule(vec![fact(20)], 21),
        rule(vec![fact(21)], 20),
        rule(vec![fact(21)], 30),
        rule(vec![fact(21)], 40),
    ];
    let report = inspect_scope_proof(&cyclic).unwrap();
    assert_eq!(report.reachable_fact_count(), 5);
}

#[test]
fn action_binding_digest_changes_the_bound_result() {
    let baseline = inspect_scope_proof(&input()).unwrap();
    let mut changed = input();
    changed.action_binding_fact = identity(5);
    let changed = inspect_scope_proof(&changed).unwrap();
    assert_ne!(baseline, changed);
    assert_ne!(
        baseline.producer_result_sha256,
        changed.producer_result_sha256
    );
    assert_eq!(
        baseline.evaluation.reachable_facts,
        changed.evaluation.reachable_facts
    );
}

#[test]
fn unproved_or_action_independent_conclusions_refuse() {
    let mut missing_applicability = input();
    missing_applicability.inference_rules.remove(1);
    assert_both_refuse(
        &missing_applicability,
        ScopeProofRefusal::ApplicabilityUnproved,
    );

    let mut missing_validity = input();
    missing_validity.inference_rules.remove(2);
    assert_both_refuse(&missing_validity, ScopeProofRefusal::ValidityUnproved);

    let mut free_applicability = input();
    free_applicability
        .inference_rules
        .push(rule(vec![fact(10)], 30));
    assert_both_refuse(
        &free_applicability,
        ScopeProofRefusal::ApplicabilityNotActionBound,
    );

    let mut free_validity = input();
    free_validity.inference_rules.push(rule(vec![fact(10)], 40));
    assert_both_refuse(&free_validity, ScopeProofRefusal::ValidityNotActionBound);
}

#[test]
fn malformed_inputs_refuse_identically() {
    let mut invalid_claim = input();
    invalid_claim.claim_identity = AlgebraIdentity([0; 32]);
    assert_both_refuse(&invalid_claim, ScopeProofRefusal::InvalidClaimIdentity);

    let mut invalid_action = input();
    invalid_action.action_binding_fact = AlgebraIdentity([0; 32]);
    assert_both_refuse(&invalid_action, ScopeProofRefusal::InvalidActionBindingFact);

    let mut duplicate_fact = input();
    duplicate_fact.premise_facts.push(identity(10));
    assert_both_refuse(&duplicate_fact, ScopeProofRefusal::DuplicatePremiseFact);

    let mut empty_rule = input();
    empty_rule.inference_rules.push(rule(Vec::new(), 90));
    assert_both_refuse(&empty_rule, ScopeProofRefusal::EmptyRulePremises);

    let mut duplicate_premise = input();
    duplicate_premise.inference_rules.push(rule(
        vec![
            ScopeRulePremise::EvaluatedAction,
            ScopeRulePremise::EvaluatedAction,
        ],
        90,
    ));
    assert_both_refuse(&duplicate_premise, ScopeProofRefusal::DuplicateRulePremise);

    let mut duplicate_rule = input();
    duplicate_rule
        .inference_rules
        .push(duplicate_rule.inference_rules[0].clone());
    assert_both_refuse(&duplicate_rule, ScopeProofRefusal::DuplicateInferenceRule);

    let mut aliased_required = input();
    aliased_required.required_validity_fact = aliased_required.required_applicability_fact;
    assert_both_refuse(&aliased_required, ScopeProofRefusal::RequiredFactAlias);
}

#[test]
fn capacities_and_preflight_work_refuse_before_closure() {
    let mut too_many_facts = input();
    too_many_facts.premise_facts = (1..=MAX_PREMISE_FACTS as u64 + 1)
        .map(|number| identity(1_000 + number))
        .collect();
    assert_both_refuse(
        &too_many_facts,
        ScopeProofRefusal::PremiseFactCapacityExceeded,
    );

    let mut too_many_rules = input();
    too_many_rules.inference_rules = (1..=MAX_INFERENCE_RULES as u64 + 1)
        .map(|number| rule(vec![ScopeRulePremise::EvaluatedAction], 10_000 + number))
        .collect();
    assert_both_refuse(
        &too_many_rules,
        ScopeProofRefusal::InferenceRuleCapacityExceeded,
    );

    let mut too_much_work = input();
    too_much_work.premise_facts = (1..=16).map(|number| identity(20_000 + number)).collect();
    too_much_work.inference_rules = (0..MAX_INFERENCE_RULES)
        .map(|rule_index| {
            let premises = (0..16)
                .map(|premise_index| fact(20_000 + u64::try_from(premise_index + 1).unwrap()))
                .collect();
            rule(premises, 30_000 + u64::try_from(rule_index).unwrap())
        })
        .collect();
    assert_both_refuse(&too_much_work, ScopeProofRefusal::WorkLimitExceeded);
}

#[test]
fn production_has_no_admitted_scope_premises() {
    let refusal = repository_scope_proof().unwrap_err();
    assert_eq!(refusal, ScopeProofRefusal::NoAdmittedScopePremiseInput);
    assert_eq!(refusal.id(), "no_admitted_scope_premise_input");
}
