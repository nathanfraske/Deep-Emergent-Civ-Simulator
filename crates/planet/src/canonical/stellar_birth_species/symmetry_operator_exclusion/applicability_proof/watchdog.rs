// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Dependency-index reconstruction of conditional scope facts.

use super::{
    AlgebraIdentity, ScopeCheckerOutput, ScopeInferenceRule, ScopeProofEvaluation, ScopeProofInput,
    ScopeProofRefusal, ScopeRulePremise, MAX_CANONICAL_BYTES, MAX_INFERENCE_RULES,
    MAX_PREMISE_FACTS, MAX_RULE_PREMISES, MAX_TOTAL_RULE_PREMISES, MAX_WORK_UNITS,
    RESULT_SCHEMA_ID,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn prove(input: &ScopeProofInput) -> Result<ScopeCheckerOutput, ScopeProofRefusal> {
    if input.claim_identity.0.iter().all(|byte| *byte == 0) {
        return Err(ScopeProofRefusal::InvalidClaimIdentity);
    }
    if input.subject_identity.0.iter().all(|byte| *byte == 0) {
        return Err(ScopeProofRefusal::InvalidSubjectIdentity);
    }
    if input
        .applicability_domain_identity
        .0
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(ScopeProofRefusal::InvalidApplicabilityDomainIdentity);
    }
    if input.action_binding_fact.0.iter().all(|byte| *byte == 0) {
        return Err(ScopeProofRefusal::InvalidActionBindingFact);
    }
    if input
        .required_applicability_fact
        .0
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(ScopeProofRefusal::InvalidRequiredApplicabilityFact);
    }
    if input.required_validity_fact.0.iter().all(|byte| *byte == 0) {
        return Err(ScopeProofRefusal::InvalidRequiredValidityFact);
    }
    if input.required_applicability_fact == input.required_validity_fact {
        return Err(ScopeProofRefusal::RequiredFactAlias);
    }

    if input.premise_facts.len() > MAX_PREMISE_FACTS {
        return Err(ScopeProofRefusal::PremiseFactCapacityExceeded);
    }
    if input
        .premise_facts
        .iter()
        .any(|fact| fact.0.iter().all(|byte| *byte == 0))
    {
        return Err(ScopeProofRefusal::InvalidPremiseFact);
    }
    let premise_set = input.premise_facts.iter().copied().collect::<BTreeSet<_>>();
    if premise_set.len() != input.premise_facts.len() {
        return Err(ScopeProofRefusal::DuplicatePremiseFact);
    }
    let premise_facts = premise_set.into_iter().collect::<Vec<_>>();

    if input.inference_rules.len() > MAX_INFERENCE_RULES {
        return Err(ScopeProofRefusal::InferenceRuleCapacityExceeded);
    }
    let mut rule_set = BTreeSet::new();
    let mut duplicate_rule = false;
    let mut total_rule_premises = 0_usize;
    for rule in &input.inference_rules {
        if rule.premises.is_empty() {
            return Err(ScopeProofRefusal::EmptyRulePremises);
        }
        if rule.premises.len() > MAX_RULE_PREMISES {
            return Err(ScopeProofRefusal::RulePremiseCapacityExceeded);
        }
        if rule.conclusion.0.iter().all(|byte| *byte == 0)
            || rule.premises.iter().any(|premise| {
                matches!(
                    premise,
                    ScopeRulePremise::Fact(identity)
                        if identity.0.iter().all(|byte| *byte == 0)
                )
            })
        {
            return Err(ScopeProofRefusal::InvalidRuleFactIdentity);
        }
        let premise_set = rule.premises.iter().copied().collect::<BTreeSet<_>>();
        if premise_set.len() != rule.premises.len() {
            return Err(ScopeProofRefusal::DuplicateRulePremise);
        }
        total_rule_premises = total_rule_premises
            .checked_add(premise_set.len())
            .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
        if total_rule_premises > MAX_TOTAL_RULE_PREMISES {
            return Err(ScopeProofRefusal::TotalRulePremiseCapacityExceeded);
        }
        if !rule_set.insert(ScopeInferenceRule {
            premises: premise_set.into_iter().collect(),
            conclusion: rule.conclusion,
        }) {
            duplicate_rule = true;
        }
    }
    if duplicate_rule {
        return Err(ScopeProofRefusal::DuplicateInferenceRule);
    }
    let inference_rules = rule_set.into_iter().collect::<Vec<_>>();
    validate_work_bound(
        premise_facts.len(),
        inference_rules.len(),
        total_rule_premises,
    )?;

    let reachable = close_dependencies(
        &premise_facts,
        Some(input.action_binding_fact),
        &inference_rules,
    )?;
    if !reachable.contains(&input.required_applicability_fact) {
        return Err(ScopeProofRefusal::ApplicabilityUnproved);
    }
    if !reachable.contains(&input.required_validity_fact) {
        return Err(ScopeProofRefusal::ValidityUnproved);
    }

    let without_action = close_dependencies(&premise_facts, None, &inference_rules)?;
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
    let canonical_bytes = serialize(&evaluation)?;
    Ok(ScopeCheckerOutput {
        evaluation,
        canonical_bytes,
    })
}

fn validate_work_bound(
    fact_count: usize,
    rule_count: usize,
    premise_count: usize,
) -> Result<(), ScopeProofRefusal> {
    let doubled_premises = premise_count
        .checked_mul(2)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let sweeps = rule_count
        .checked_add(1)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let scan_work = doubled_premises
        .checked_mul(sweeps)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let inventory_work = fact_count
        .checked_add(rule_count)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    let work = scan_work
        .checked_add(inventory_work)
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    if work > MAX_WORK_UNITS {
        Err(ScopeProofRefusal::WorkLimitExceeded)
    } else {
        Ok(())
    }
}

fn close_dependencies(
    premise_facts: &[AlgebraIdentity],
    action_binding_fact: Option<AlgebraIdentity>,
    rules: &[ScopeInferenceRule],
) -> Result<BTreeSet<AlgebraIdentity>, ScopeProofRefusal> {
    let mut reachable = premise_facts.iter().copied().collect::<BTreeSet<_>>();
    let mut unresolved = rules
        .iter()
        .map(|rule| rule.premises.len())
        .collect::<Vec<_>>();
    let mut dependents = BTreeMap::<ScopeRulePremise, Vec<usize>>::new();
    for (rule_index, rule) in rules.iter().enumerate().rev() {
        for premise in rule.premises.iter().rev() {
            dependents.entry(*premise).or_default().push(rule_index);
        }
    }
    let mut agenda = reachable
        .iter()
        .rev()
        .copied()
        .map(ScopeRulePremise::Fact)
        .collect::<VecDeque<_>>();
    if action_binding_fact.is_some() {
        agenda.push_front(ScopeRulePremise::EvaluatedAction);
    }
    while let Some(fact) = agenda.pop_front() {
        let Some(rule_indices) = dependents.get(&fact) else {
            continue;
        };
        for rule_index in rule_indices.iter().rev().copied() {
            unresolved[rule_index] = unresolved[rule_index]
                .checked_sub(1)
                .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
            if unresolved[rule_index] == 0 {
                let conclusion = rules[rule_index].conclusion;
                if reachable.insert(conclusion) {
                    agenda.push_back(ScopeRulePremise::Fact(conclusion));
                }
            }
        }
    }
    Ok(reachable)
}

fn serialize(evaluation: &ScopeProofEvaluation) -> Result<Vec<u8>, ScopeProofRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    fields
        .entry(1)
        .or_default()
        .push(RESULT_SCHEMA_ID.as_bytes().to_vec());
    fields
        .entry(2)
        .or_default()
        .push(evaluation.claim_identity.0.to_vec());
    fields
        .entry(3)
        .or_default()
        .push(evaluation.subject_identity.0.to_vec());
    fields
        .entry(4)
        .or_default()
        .push(evaluation.applicability_domain_identity.0.to_vec());
    fields
        .entry(5)
        .or_default()
        .push(evaluation.action_binding_fact.0.to_vec());
    for fact in &evaluation.premise_facts {
        fields.entry(10).or_default().push(fact.0.to_vec());
    }
    for rule in &evaluation.inference_rules {
        let mut record = Vec::new();
        for premise in &rule.premises {
            let mut premise_record = Vec::new();
            match premise {
                ScopeRulePremise::Fact(identity) => {
                    push(&mut premise_record, 1, b"fact")?;
                    push(&mut premise_record, 2, &identity.0)?;
                }
                ScopeRulePremise::EvaluatedAction => {
                    push(&mut premise_record, 1, b"evaluated-action")?;
                }
            }
            push(&mut record, 1, &premise_record)?;
        }
        push(&mut record, 2, &rule.conclusion.0)?;
        fields.entry(11).or_default().push(record);
    }
    fields
        .entry(12)
        .or_default()
        .push(evaluation.required_applicability_fact.0.to_vec());
    fields
        .entry(13)
        .or_default()
        .push(evaluation.required_validity_fact.0.to_vec());
    for fact in &evaluation.reachable_facts {
        fields.entry(14).or_default().push(fact.0.to_vec());
    }
    fields.entry(15).or_default().push(vec![1]);
    fields.entry(16).or_default().push(vec![1]);
    fields.entry(17).or_default().push(vec![1]);

    let mut output = Vec::new();
    for (tag, payloads) in fields {
        for payload in payloads {
            push(&mut output, tag, &payload)?;
        }
    }
    Ok(output)
}

fn push(output: &mut Vec<u8>, tag: u16, payload: &[u8]) -> Result<(), ScopeProofRefusal> {
    let header_len = 10_usize;
    let next_len = output
        .len()
        .checked_add(header_len)
        .and_then(|value| value.checked_add(payload.len()))
        .ok_or(ScopeProofRefusal::ArithmeticOverflow)?;
    if next_len > MAX_CANONICAL_BYTES {
        return Err(ScopeProofRefusal::CanonicalByteCapacityExceeded);
    }
    output.extend_from_slice(&tag.to_be_bytes());
    let size = u64::try_from(payload.len())
        .map_err(|_| ScopeProofRefusal::CanonicalByteCapacityExceeded)?;
    output.extend_from_slice(&size.to_be_bytes());
    output.extend_from_slice(payload);
    Ok(())
}
