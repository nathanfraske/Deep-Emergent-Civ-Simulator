// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Canonical fixed-point saturation for conditional scope facts.

use super::{
    AlgebraIdentity, ScopeCheckerOutput, ScopeInferenceRule, ScopeProofEvaluation, ScopeProofInput,
    ScopeProofRefusal, ScopeRulePremise, MAX_CANONICAL_BYTES, MAX_INFERENCE_RULES,
    MAX_PREMISE_FACTS, MAX_RULE_PREMISES, MAX_TOTAL_RULE_PREMISES, MAX_WORK_UNITS,
    RESULT_SCHEMA_ID,
};
use std::collections::BTreeSet;

pub(super) fn prove(input: &ScopeProofInput) -> Result<ScopeCheckerOutput, ScopeProofRefusal> {
    validate_identity(
        input.claim_identity,
        ScopeProofRefusal::InvalidClaimIdentity,
    )?;
    validate_identity(
        input.subject_identity,
        ScopeProofRefusal::InvalidSubjectIdentity,
    )?;
    validate_identity(
        input.applicability_domain_identity,
        ScopeProofRefusal::InvalidApplicabilityDomainIdentity,
    )?;
    validate_identity(
        input.action_binding_fact,
        ScopeProofRefusal::InvalidActionBindingFact,
    )?;
    validate_identity(
        input.required_applicability_fact,
        ScopeProofRefusal::InvalidRequiredApplicabilityFact,
    )?;
    validate_identity(
        input.required_validity_fact,
        ScopeProofRefusal::InvalidRequiredValidityFact,
    )?;
    if input.required_applicability_fact == input.required_validity_fact {
        return Err(ScopeProofRefusal::RequiredFactAlias);
    }

    let premise_facts = canonical_premise_facts(input)?;
    let (inference_rules, total_rule_premises) = canonical_rules(input)?;
    validate_work_bound(
        premise_facts.len(),
        inference_rules.len(),
        total_rule_premises,
    )?;

    let reachable = saturate(
        &premise_facts,
        Some(input.action_binding_fact),
        &inference_rules,
    );
    if !reachable.contains(&input.required_applicability_fact) {
        return Err(ScopeProofRefusal::ApplicabilityUnproved);
    }
    if !reachable.contains(&input.required_validity_fact) {
        return Err(ScopeProofRefusal::ValidityUnproved);
    }

    let without_action = saturate(&premise_facts, None, &inference_rules);
    if without_action.contains(&input.required_applicability_fact) {
        return Err(ScopeProofRefusal::ApplicabilityNotActionBound);
    }
    if without_action.contains(&input.required_validity_fact) {
        return Err(ScopeProofRefusal::ValidityNotActionBound);
    }

    let evaluation = ScopeProofEvaluation {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        action_binding_fact: input.action_binding_fact,
        premise_facts,
        inference_rules,
        required_applicability_fact: input.required_applicability_fact,
        required_validity_fact: input.required_validity_fact,
        reachable_facts: reachable.into_iter().collect(),
    };
    let canonical_bytes = encode(&evaluation)?;
    Ok(ScopeCheckerOutput {
        evaluation,
        canonical_bytes,
    })
}

fn validate_identity(
    identity: AlgebraIdentity,
    refusal: ScopeProofRefusal,
) -> Result<(), ScopeProofRefusal> {
    if identity.0 == [0; 32] {
        Err(refusal)
    } else {
        Ok(())
    }
}

fn canonical_premise_facts(
    input: &ScopeProofInput,
) -> Result<Vec<AlgebraIdentity>, ScopeProofRefusal> {
    if input.premise_facts.len() > MAX_PREMISE_FACTS {
        return Err(ScopeProofRefusal::PremiseFactCapacityExceeded);
    }
    let mut facts = input.premise_facts.clone();
    if facts.iter().any(|fact| fact.0 == [0; 32]) {
        return Err(ScopeProofRefusal::InvalidPremiseFact);
    }
    facts.sort_unstable();
    if facts.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ScopeProofRefusal::DuplicatePremiseFact);
    }
    Ok(facts)
}

fn canonical_rules(
    input: &ScopeProofInput,
) -> Result<(Vec<ScopeInferenceRule>, usize), ScopeProofRefusal> {
    if input.inference_rules.len() > MAX_INFERENCE_RULES {
        return Err(ScopeProofRefusal::InferenceRuleCapacityExceeded);
    }
    let mut total_premises = 0_usize;
    let mut rules = Vec::with_capacity(input.inference_rules.len());
    for rule in &input.inference_rules {
        if rule.premises.is_empty() {
            return Err(ScopeProofRefusal::EmptyRulePremises);
        }
        if rule.premises.len() > MAX_RULE_PREMISES {
            return Err(ScopeProofRefusal::RulePremiseCapacityExceeded);
        }
        if rule.conclusion.0 == [0; 32]
            || rule.premises.iter().any(|premise| {
                matches!(premise, ScopeRulePremise::Fact(identity) if identity.0 == [0; 32])
            })
        {
            return Err(ScopeProofRefusal::InvalidRuleFactIdentity);
        }
        let mut premises = rule.premises.clone();
        premises.sort_unstable();
        if premises.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScopeProofRefusal::DuplicateRulePremise);
        }
        total_premises = total_premises
            .checked_add(premises.len())
            .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
        if total_premises > MAX_TOTAL_RULE_PREMISES {
            return Err(ScopeProofRefusal::TotalRulePremiseCapacityExceeded);
        }
        rules.push(ScopeInferenceRule {
            premises,
            conclusion: rule.conclusion,
        });
    }
    rules.sort_unstable();
    if rules.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ScopeProofRefusal::DuplicateInferenceRule);
    }
    Ok((rules, total_premises))
}

fn validate_work_bound(
    fact_count: usize,
    rule_count: usize,
    premise_count: usize,
) -> Result<(), ScopeProofRefusal> {
    let passes = rule_count
        .checked_add(1)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let scan_work = premise_count
        .checked_mul(passes)
        .and_then(|value| value.checked_mul(2))
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let work = scan_work
        .checked_add(fact_count)
        .and_then(|value| value.checked_add(rule_count))
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    if work > MAX_WORK_UNITS {
        Err(ScopeProofRefusal::WorkLimitExceeded)
    } else {
        Ok(())
    }
}

fn saturate(
    premise_facts: &[AlgebraIdentity],
    include_evaluated_action: Option<AlgebraIdentity>,
    rules: &[ScopeInferenceRule],
) -> BTreeSet<AlgebraIdentity> {
    let mut reachable = premise_facts.iter().copied().collect::<BTreeSet<_>>();
    loop {
        let mut changed = false;
        for rule in rules {
            if rule.premises.iter().all(|premise| match premise {
                ScopeRulePremise::Fact(identity) => reachable.contains(identity),
                ScopeRulePremise::EvaluatedAction => include_evaluated_action.is_some(),
            }) {
                changed |= reachable.insert(rule.conclusion);
            }
        }
        if !changed {
            return reachable;
        }
    }
}

fn encode(evaluation: &ScopeProofEvaluation) -> Result<Vec<u8>, ScopeProofRefusal> {
    let mut output = Vec::new();
    field(&mut output, 1, RESULT_SCHEMA_ID.as_bytes())?;
    field(&mut output, 2, &evaluation.claim_identity.0)?;
    field(&mut output, 3, &evaluation.subject_identity.0)?;
    field(&mut output, 4, &evaluation.applicability_domain_identity.0)?;
    field(&mut output, 5, &evaluation.action_binding_fact.0)?;
    for fact in &evaluation.premise_facts {
        field(&mut output, 10, &fact.0)?;
    }
    for rule in &evaluation.inference_rules {
        let mut record = Vec::new();
        for premise in &rule.premises {
            let mut premise_record = Vec::new();
            match premise {
                ScopeRulePremise::Fact(identity) => {
                    field(&mut premise_record, 1, b"fact")?;
                    field(&mut premise_record, 2, &identity.0)?;
                }
                ScopeRulePremise::EvaluatedAction => {
                    field(&mut premise_record, 1, b"evaluated-action")?;
                }
            }
            field(&mut record, 1, &premise_record)?;
        }
        field(&mut record, 2, &rule.conclusion.0)?;
        field(&mut output, 11, &record)?;
    }
    field(&mut output, 12, &evaluation.required_applicability_fact.0)?;
    field(&mut output, 13, &evaluation.required_validity_fact.0)?;
    for fact in &evaluation.reachable_facts {
        field(&mut output, 14, &fact.0)?;
    }
    field(&mut output, 15, &[1])?;
    field(&mut output, 16, &[1])?;
    field(&mut output, 17, &[1])?;
    Ok(output)
}

fn field(output: &mut Vec<u8>, tag: u16, payload: &[u8]) -> Result<(), ScopeProofRefusal> {
    let next_len = output
        .len()
        .checked_add(2)
        .and_then(|value| value.checked_add(8))
        .and_then(|value| value.checked_add(payload.len()))
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    if next_len > MAX_CANONICAL_BYTES {
        return Err(ScopeProofRefusal::CanonicalByteCapacityExceeded);
    }
    output.extend_from_slice(&tag.to_be_bytes());
    output.extend_from_slice(
        &u64::try_from(payload.len())
            .map_err(|_| ScopeProofRefusal::CanonicalByteCapacityExceeded)?
            .to_be_bytes(),
    );
    output.extend_from_slice(payload);
    Ok(())
}
