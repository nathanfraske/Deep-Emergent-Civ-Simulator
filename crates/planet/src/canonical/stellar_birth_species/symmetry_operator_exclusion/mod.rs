// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Exact conditional evaluation of affine symmetry actions on quadratic operators.
//!
//! The producer expands the transformed polynomial and subtracts the original.
//! The watchdog independently constructs only the direct variation terms. Both
//! use exact bounded integer arithmetic and must agree on the complete
//! canonical result.
//!
//! This evaluator proves a conditional algebraic statement about the supplied
//! action and requested operator set. It cannot admit that action, establish
//! the physical applicability domain, prove that the requested set is a
//! complete operator basis, or mint a species. Production therefore has no
//! input constructor and returns a typed refusal.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use civsim_units::digest::sha256;

const RESULT_SCHEMA_ID: &str = "civsim.planet.symmetry-operator-exclusion-evaluation.v1";
const PRODUCER_ID: &str = "civsim.planet.symmetry-operator-exclusion.substitution-producer.v1";
const WATCHDOG_ID: &str = "civsim.planet.symmetry-operator-exclusion.direct-variation-watchdog.v1";

const MAX_FIELD_COMPONENTS: usize = 64;
const MAX_ACTION_TERMS: usize = 4_096;
const MAX_OPERATORS: usize = 1_024;
const MAX_OPERATOR_TERMS: usize = 4_096;
const MAX_TOTAL_OPERATOR_TERMS: usize = 16_384;
const MAX_VARIATION_TERMS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct AlgebraIdentity([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GeneratorKind {
    Field,
    Shift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VariationGenerator {
    kind: GeneratorKind,
    identity: AlgebraIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AffineActionTerm {
    field_component: AlgebraIdentity,
    shift_generator: AlgebraIdentity,
    coefficient: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AffineSymmetryAction {
    symmetry_identity: AlgebraIdentity,
    terms: Vec<AffineActionTerm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuadraticOperatorTerm {
    left_field: AlgebraIdentity,
    right_field: AlgebraIdentity,
    coefficient: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QuadraticOperatorCandidate {
    operator_identity: AlgebraIdentity,
    terms: Vec<QuadraticOperatorTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymmetryOperatorExclusionInput {
    claim_identity: AlgebraIdentity,
    subject_identity: AlgebraIdentity,
    applicability_domain_identity: AlgebraIdentity,
    field_components: Vec<AlgebraIdentity>,
    action: AffineSymmetryAction,
    operators: Vec<QuadraticOperatorCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ExactVariationTerm {
    first: VariationGenerator,
    second: VariationGenerator,
    coefficient: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatorVariation {
    operator_identity: AlgebraIdentity,
    terms: Vec<ExactVariationTerm>,
}

impl OperatorVariation {
    fn is_invariant(&self) -> bool {
        self.terms.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymmetryOperatorEvaluation {
    claim_identity: AlgebraIdentity,
    subject_identity: AlgebraIdentity,
    symmetry_identity: AlgebraIdentity,
    applicability_domain_identity: AlgebraIdentity,
    operators: Vec<OperatorVariation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckerOutput {
    evaluation: SymmetryOperatorEvaluation,
    canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SymmetryOperatorExclusionReport {
    evaluation: SymmetryOperatorEvaluation,
    producer_id: &'static str,
    watchdog_id: &'static str,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
}

impl SymmetryOperatorExclusionReport {
    fn excluded_operator_count(&self) -> usize {
        self.evaluation
            .operators
            .iter()
            .filter(|operator| !operator.is_invariant())
            .count()
    }

    fn invariant_operator_count(&self) -> usize {
        self.evaluation
            .operators
            .iter()
            .filter(|operator| operator.is_invariant())
            .count()
    }

    const fn requested_operator_coverage(&self) -> bool {
        true
    }

    const fn global_operator_basis_coverage(&self) -> bool {
        false
    }

    const fn applicability_authority(&self) -> bool {
        false
    }

    const fn species_membership_authority(&self) -> bool {
        false
    }

    const fn authority_effect(&self) -> &'static str {
        "none"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymmetryOperatorExclusionRefusal {
    NoAdmittedSymmetryActionInput,
    InvalidClaimIdentity,
    InvalidSubjectIdentity,
    InvalidSymmetryIdentity,
    InvalidApplicabilityDomainIdentity,
    EmptyFieldBasis,
    FieldCapacityExceeded,
    InvalidFieldComponentIdentity,
    DuplicateFieldComponent,
    ActionTermCapacityExceeded,
    InvalidActionTerm,
    ActionReferencesUnknownField,
    EmptyOperatorSet,
    OperatorCapacityExceeded,
    InvalidOperatorIdentity,
    DuplicateOperatorIdentity,
    OperatorTermCapacityExceeded,
    TotalOperatorTermCapacityExceeded,
    InvalidOperatorTerm,
    OperatorReferencesUnknownField,
    ZeroOperator,
    ArithmeticOverflow,
    VariationCapacityExceeded,
    CheckerDisagreement,
}

impl SymmetryOperatorExclusionRefusal {
    const fn id(self) -> &'static str {
        match self {
            Self::NoAdmittedSymmetryActionInput => "no_admitted_symmetry_action_input",
            Self::InvalidClaimIdentity => "invalid_claim_identity",
            Self::InvalidSubjectIdentity => "invalid_subject_identity",
            Self::InvalidSymmetryIdentity => "invalid_symmetry_identity",
            Self::InvalidApplicabilityDomainIdentity => "invalid_applicability_domain_identity",
            Self::EmptyFieldBasis => "empty_field_basis",
            Self::FieldCapacityExceeded => "field_capacity_exceeded",
            Self::InvalidFieldComponentIdentity => "invalid_field_component_identity",
            Self::DuplicateFieldComponent => "duplicate_field_component",
            Self::ActionTermCapacityExceeded => "action_term_capacity_exceeded",
            Self::InvalidActionTerm => "invalid_action_term",
            Self::ActionReferencesUnknownField => "action_references_unknown_field",
            Self::EmptyOperatorSet => "empty_operator_set",
            Self::OperatorCapacityExceeded => "operator_capacity_exceeded",
            Self::InvalidOperatorIdentity => "invalid_operator_identity",
            Self::DuplicateOperatorIdentity => "duplicate_operator_identity",
            Self::OperatorTermCapacityExceeded => "operator_term_capacity_exceeded",
            Self::TotalOperatorTermCapacityExceeded => "total_operator_term_capacity_exceeded",
            Self::InvalidOperatorTerm => "invalid_operator_term",
            Self::OperatorReferencesUnknownField => "operator_references_unknown_field",
            Self::ZeroOperator => "zero_operator",
            Self::ArithmeticOverflow => "arithmetic_overflow",
            Self::VariationCapacityExceeded => "variation_capacity_exceeded",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

fn inspect_symmetry_operator_exclusion(
    input: &SymmetryOperatorExclusionInput,
) -> Result<SymmetryOperatorExclusionReport, SymmetryOperatorExclusionRefusal> {
    let produced = producer::evaluate(input);
    let watched = watchdog::evaluate(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.evaluation == watched.evaluation
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            let producer_result_sha256 = sha256(&produced.canonical_bytes);
            let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
            Ok(SymmetryOperatorExclusionReport {
                evaluation: produced.evaluation,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                producer_result_sha256,
                watchdog_result_sha256,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(SymmetryOperatorExclusionRefusal::CheckerDisagreement),
    }
}

fn repository_symmetry_operator_exclusion(
) -> Result<SymmetryOperatorExclusionReport, SymmetryOperatorExclusionRefusal> {
    Err(SymmetryOperatorExclusionRefusal::NoAdmittedSymmetryActionInput)
}
