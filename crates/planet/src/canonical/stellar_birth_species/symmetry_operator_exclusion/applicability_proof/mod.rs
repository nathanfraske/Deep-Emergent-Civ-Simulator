// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Exact conditional applicability and validity closure.
//!
//! The producer saturates opaque positive inference rules. The watchdog
//! independently tracks unresolved premises through a dependency index. Both
//! prove the required facts, then repeat the calculation without the exact
//! action-binding fact. A successful report therefore proves that both
//! conclusions depend on the evaluated action rather than on a reusable
//! assertion.
//!
//! This pair does not admit a premise, rule, field ontology, action, or
//! physical domain. Production supplies none and returns a typed refusal.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::AlgebraIdentity;
use civsim_units::digest::sha256;

const RESULT_SCHEMA_ID: &str = "civsim.planet.symmetry-scope-applicability-proof-evaluation.v1";
const PRODUCER_ID: &str = "civsim.planet.symmetry-scope-applicability.fixed-point-producer.v1";
const WATCHDOG_ID: &str = "civsim.planet.symmetry-scope-applicability.dependency-watchdog.v1";

const MAX_PREMISE_FACTS: usize = 4_096;
const MAX_INFERENCE_RULES: usize = 256;
const MAX_RULE_PREMISES: usize = 256;
const MAX_TOTAL_RULE_PREMISES: usize = 4_096;
const MAX_WORK_UNITS: usize = 1_100_000;
const MAX_CANONICAL_BYTES: usize = 2_097_152;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ScopeRulePremise {
    Fact(AlgebraIdentity),
    EvaluatedAction,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ScopeInferenceRule {
    pub(super) premises: Vec<ScopeRulePremise>,
    pub(super) conclusion: AlgebraIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeProofInput {
    pub(super) claim_identity: AlgebraIdentity,
    pub(super) subject_identity: AlgebraIdentity,
    pub(super) applicability_domain_identity: AlgebraIdentity,
    pub(super) action_binding_fact: AlgebraIdentity,
    pub(super) premise_facts: Vec<AlgebraIdentity>,
    pub(super) inference_rules: Vec<ScopeInferenceRule>,
    pub(super) required_applicability_fact: AlgebraIdentity,
    pub(super) required_validity_fact: AlgebraIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeProofEvaluation {
    pub(super) claim_identity: AlgebraIdentity,
    pub(super) subject_identity: AlgebraIdentity,
    pub(super) applicability_domain_identity: AlgebraIdentity,
    pub(super) action_binding_fact: AlgebraIdentity,
    pub(super) premise_facts: Vec<AlgebraIdentity>,
    pub(super) inference_rules: Vec<ScopeInferenceRule>,
    pub(super) required_applicability_fact: AlgebraIdentity,
    pub(super) required_validity_fact: AlgebraIdentity,
    pub(super) reachable_facts: Vec<AlgebraIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeCheckerOutput {
    pub(super) evaluation: ScopeProofEvaluation,
    pub(super) canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScopeProofReport {
    evaluation: ScopeProofEvaluation,
    producer_id: &'static str,
    watchdog_id: &'static str,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
}

impl ScopeProofReport {
    pub(super) fn reachable_fact_count(&self) -> usize {
        self.evaluation.reachable_facts.len()
    }

    pub(super) const fn action_binding_fact(&self) -> AlgebraIdentity {
        self.evaluation.action_binding_fact
    }

    pub(super) const fn conditional_applicability_proved(&self) -> bool {
        true
    }

    pub(super) const fn conditional_validity_proved(&self) -> bool {
        true
    }

    pub(super) const fn action_dependency_proved(&self) -> bool {
        true
    }

    pub(super) const fn premise_admission_authority(&self) -> bool {
        false
    }

    pub(super) const fn applicability_authority(&self) -> bool {
        false
    }

    pub(super) const fn validity_authority(&self) -> bool {
        false
    }

    pub(super) const fn species_membership_authority(&self) -> bool {
        false
    }

    pub(super) const fn authority_effect(&self) -> &'static str {
        "none"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScopeProofRefusal {
    NoAdmittedScopePremiseInput,
    InvalidClaimIdentity,
    InvalidSubjectIdentity,
    InvalidApplicabilityDomainIdentity,
    InvalidActionBindingFact,
    PremiseFactCapacityExceeded,
    InvalidPremiseFact,
    DuplicatePremiseFact,
    InferenceRuleCapacityExceeded,
    EmptyRulePremises,
    RulePremiseCapacityExceeded,
    TotalRulePremiseCapacityExceeded,
    InvalidRuleFactIdentity,
    DuplicateRulePremise,
    DuplicateInferenceRule,
    InvalidRequiredApplicabilityFact,
    InvalidRequiredValidityFact,
    RequiredFactAlias,
    ArithmeticOverflow,
    WorkLimitExceeded,
    CanonicalByteCapacityExceeded,
    ApplicabilityUnproved,
    ValidityUnproved,
    ApplicabilityNotActionBound,
    ValidityNotActionBound,
    CheckerDisagreement,
}

impl ScopeProofRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::NoAdmittedScopePremiseInput => "no_admitted_scope_premise_input",
            Self::InvalidClaimIdentity => "invalid_claim_identity",
            Self::InvalidSubjectIdentity => "invalid_subject_identity",
            Self::InvalidApplicabilityDomainIdentity => "invalid_applicability_domain_identity",
            Self::InvalidActionBindingFact => "invalid_action_binding_fact",
            Self::PremiseFactCapacityExceeded => "premise_fact_capacity_exceeded",
            Self::InvalidPremiseFact => "invalid_premise_fact",
            Self::DuplicatePremiseFact => "duplicate_premise_fact",
            Self::InferenceRuleCapacityExceeded => "inference_rule_capacity_exceeded",
            Self::EmptyRulePremises => "empty_rule_premises",
            Self::RulePremiseCapacityExceeded => "rule_premise_capacity_exceeded",
            Self::TotalRulePremiseCapacityExceeded => "total_rule_premise_capacity_exceeded",
            Self::InvalidRuleFactIdentity => "invalid_rule_fact_identity",
            Self::DuplicateRulePremise => "duplicate_rule_premise",
            Self::DuplicateInferenceRule => "duplicate_inference_rule",
            Self::InvalidRequiredApplicabilityFact => "invalid_required_applicability_fact",
            Self::InvalidRequiredValidityFact => "invalid_required_validity_fact",
            Self::RequiredFactAlias => "required_fact_alias",
            Self::ArithmeticOverflow => "arithmetic_overflow",
            Self::WorkLimitExceeded => "work_limit_exceeded",
            Self::CanonicalByteCapacityExceeded => "canonical_byte_capacity_exceeded",
            Self::ApplicabilityUnproved => "applicability_unproved",
            Self::ValidityUnproved => "validity_unproved",
            Self::ApplicabilityNotActionBound => "applicability_not_action_bound",
            Self::ValidityNotActionBound => "validity_not_action_bound",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

pub(super) fn inspect_scope_proof(
    input: &ScopeProofInput,
) -> Result<ScopeProofReport, ScopeProofRefusal> {
    let produced = producer::prove(input);
    let watched = watchdog::prove(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.evaluation == watched.evaluation
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            let producer_result_sha256 = sha256(&produced.canonical_bytes);
            let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
            Ok(ScopeProofReport {
                evaluation: produced.evaluation,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                producer_result_sha256,
                watchdog_result_sha256,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(ScopeProofRefusal::CheckerDisagreement),
    }
}

fn repository_scope_proof() -> Result<ScopeProofReport, ScopeProofRefusal> {
    Err(ScopeProofRefusal::NoAdmittedScopePremiseInput)
}
