// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Sparse polynomial substitution followed by exact subtraction.

use super::{
    AffineSymmetryAction, AlgebraIdentity, CheckerOutput, ExactVariationTerm, GeneratorKind,
    OperatorVariation, QuadraticOperatorCandidate, SymmetryOperatorEvaluation,
    SymmetryOperatorExclusionInput, SymmetryOperatorExclusionRefusal, VariationGenerator,
    MAX_ACTION_TERMS, MAX_FIELD_COMPONENTS, MAX_OPERATORS, MAX_OPERATOR_TERMS,
    MAX_TOTAL_OPERATOR_TERMS, MAX_VARIATION_TERMS, RESULT_SCHEMA_ID,
};
use std::collections::{BTreeMap, BTreeSet};

type Polynomial = BTreeMap<(VariationGenerator, VariationGenerator), i128>;
type ActionMap = BTreeMap<AlgebraIdentity, Vec<(AlgebraIdentity, i128)>>;
type CanonicalOperatorTerms = BTreeMap<(AlgebraIdentity, AlgebraIdentity), i128>;
type CanonicalOperator = (AlgebraIdentity, CanonicalOperatorTerms);

pub(super) fn evaluate(
    input: &SymmetryOperatorExclusionInput,
) -> Result<CheckerOutput, SymmetryOperatorExclusionRefusal> {
    validate_identity(
        input.claim_identity,
        SymmetryOperatorExclusionRefusal::InvalidClaimIdentity,
    )?;
    validate_identity(
        input.subject_identity,
        SymmetryOperatorExclusionRefusal::InvalidSubjectIdentity,
    )?;
    validate_identity(
        input.action.symmetry_identity,
        SymmetryOperatorExclusionRefusal::InvalidSymmetryIdentity,
    )?;
    validate_identity(
        input.applicability_domain_identity,
        SymmetryOperatorExclusionRefusal::InvalidApplicabilityDomainIdentity,
    )?;

    let fields = canonical_fields(&input.field_components)?;
    let action = canonical_action(&input.action, &fields)?;
    let operators = canonical_operators(&input.operators, &fields)?;
    validate_variation_cost(&action, &operators)?;
    let mut variations = Vec::with_capacity(operators.len());
    for (operator_identity, terms) in &operators {
        let mut transformed = Polynomial::new();
        for ((left, right), coefficient) in terms {
            let left_substitution = substitution(*left, &action);
            let right_substitution = substitution(*right, &action);
            for (left_generator, left_coefficient) in &left_substitution {
                for (right_generator, right_coefficient) in &right_substitution {
                    let product = coefficient
                        .checked_mul(*left_coefficient)
                        .and_then(|value| value.checked_mul(*right_coefficient))
                        .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                    accumulate(&mut transformed, *left_generator, *right_generator, product)?;
                }
            }
            accumulate(
                &mut transformed,
                VariationGenerator {
                    kind: GeneratorKind::Field,
                    identity: *left,
                },
                VariationGenerator {
                    kind: GeneratorKind::Field,
                    identity: *right,
                },
                coefficient
                    .checked_neg()
                    .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?,
            )?;
        }
        let terms = transformed
            .into_iter()
            .map(|((first, second), coefficient)| ExactVariationTerm {
                first,
                second,
                coefficient,
            })
            .collect::<Vec<_>>();
        if terms.len() > MAX_VARIATION_TERMS {
            return Err(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded);
        }
        variations.push(OperatorVariation {
            operator_identity: *operator_identity,
            terms,
        });
    }

    let evaluation = SymmetryOperatorEvaluation {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        symmetry_identity: input.action.symmetry_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        operators: variations,
    };
    let canonical_bytes = encode(input, &fields, &action, &operators, &evaluation)?;
    Ok(CheckerOutput {
        evaluation,
        canonical_bytes,
    })
}

fn validate_identity(
    identity: AlgebraIdentity,
    refusal: SymmetryOperatorExclusionRefusal,
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    if identity.0 == [0; 32] {
        Err(refusal)
    } else {
        Ok(())
    }
}

fn canonical_fields(
    input: &[AlgebraIdentity],
) -> Result<Vec<AlgebraIdentity>, SymmetryOperatorExclusionRefusal> {
    if input.is_empty() {
        return Err(SymmetryOperatorExclusionRefusal::EmptyFieldBasis);
    }
    if input.len() > MAX_FIELD_COMPONENTS {
        return Err(SymmetryOperatorExclusionRefusal::FieldCapacityExceeded);
    }
    let mut fields = input.to_vec();
    if fields.iter().any(|identity| identity.0 == [0; 32]) {
        return Err(SymmetryOperatorExclusionRefusal::InvalidFieldComponentIdentity);
    }
    fields.sort_unstable();
    if fields.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(SymmetryOperatorExclusionRefusal::DuplicateFieldComponent);
    }
    Ok(fields)
}

fn canonical_action(
    action: &AffineSymmetryAction,
    fields: &[AlgebraIdentity],
) -> Result<ActionMap, SymmetryOperatorExclusionRefusal> {
    if action.terms.len() > MAX_ACTION_TERMS {
        return Err(SymmetryOperatorExclusionRefusal::ActionTermCapacityExceeded);
    }
    let field_set = fields.iter().copied().collect::<BTreeSet<_>>();
    let mut coefficients = BTreeMap::<(AlgebraIdentity, AlgebraIdentity), i128>::new();
    for term in &action.terms {
        if term.coefficient == 0 || term.shift_generator.0 == [0; 32] {
            return Err(SymmetryOperatorExclusionRefusal::InvalidActionTerm);
        }
        if !field_set.contains(&term.field_component) {
            return Err(SymmetryOperatorExclusionRefusal::ActionReferencesUnknownField);
        }
        let key = (term.field_component, term.shift_generator);
        let next = coefficients
            .get(&key)
            .copied()
            .unwrap_or(0)
            .checked_add(i128::from(term.coefficient))
            .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
        if next == 0 {
            coefficients.remove(&key);
        } else {
            coefficients.insert(key, next);
        }
    }
    let mut action_map = BTreeMap::<AlgebraIdentity, Vec<(AlgebraIdentity, i128)>>::new();
    for ((field, shift), coefficient) in coefficients {
        action_map
            .entry(field)
            .or_default()
            .push((shift, coefficient));
    }
    Ok(action_map)
}

fn canonical_operators(
    input: &[QuadraticOperatorCandidate],
    fields: &[AlgebraIdentity],
) -> Result<Vec<CanonicalOperator>, SymmetryOperatorExclusionRefusal> {
    if input.is_empty() {
        return Err(SymmetryOperatorExclusionRefusal::EmptyOperatorSet);
    }
    if input.len() > MAX_OPERATORS {
        return Err(SymmetryOperatorExclusionRefusal::OperatorCapacityExceeded);
    }
    let total_terms = input
        .iter()
        .try_fold(0_usize, |total, operator| {
            total.checked_add(operator.terms.len())
        })
        .ok_or(SymmetryOperatorExclusionRefusal::TotalOperatorTermCapacityExceeded)?;
    if total_terms > MAX_TOTAL_OPERATOR_TERMS {
        return Err(SymmetryOperatorExclusionRefusal::TotalOperatorTermCapacityExceeded);
    }

    let field_set = fields.iter().copied().collect::<BTreeSet<_>>();
    let mut identities = BTreeSet::new();
    let mut operators = Vec::with_capacity(input.len());
    for operator in input {
        if operator.operator_identity.0 == [0; 32] {
            return Err(SymmetryOperatorExclusionRefusal::InvalidOperatorIdentity);
        }
        if !identities.insert(operator.operator_identity) {
            return Err(SymmetryOperatorExclusionRefusal::DuplicateOperatorIdentity);
        }
        if operator.terms.len() > MAX_OPERATOR_TERMS {
            return Err(SymmetryOperatorExclusionRefusal::OperatorTermCapacityExceeded);
        }
        let mut terms = BTreeMap::new();
        for term in &operator.terms {
            if term.coefficient == 0 {
                return Err(SymmetryOperatorExclusionRefusal::InvalidOperatorTerm);
            }
            if !field_set.contains(&term.left_field) || !field_set.contains(&term.right_field) {
                return Err(SymmetryOperatorExclusionRefusal::OperatorReferencesUnknownField);
            }
            let key = if term.left_field <= term.right_field {
                (term.left_field, term.right_field)
            } else {
                (term.right_field, term.left_field)
            };
            let next = terms
                .get(&key)
                .copied()
                .unwrap_or(0_i128)
                .checked_add(i128::from(term.coefficient))
                .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
            if next == 0 {
                terms.remove(&key);
            } else {
                terms.insert(key, next);
            }
        }
        if terms.is_empty() {
            return Err(SymmetryOperatorExclusionRefusal::ZeroOperator);
        }
        operators.push((operator.operator_identity, terms));
    }
    operators.sort_unstable_by_key(|(identity, _)| *identity);
    Ok(operators)
}

fn substitution(field: AlgebraIdentity, action: &ActionMap) -> Vec<(VariationGenerator, i128)> {
    let mut result = vec![(
        VariationGenerator {
            kind: GeneratorKind::Field,
            identity: field,
        },
        1,
    )];
    if let Some(shifts) = action.get(&field) {
        result.extend(shifts.iter().map(|(identity, coefficient)| {
            (
                VariationGenerator {
                    kind: GeneratorKind::Shift,
                    identity: *identity,
                },
                *coefficient,
            )
        }));
    }
    result
}

fn validate_variation_cost(
    action: &ActionMap,
    operators: &[CanonicalOperator],
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    for (_, terms) in operators {
        let mut expansion_terms = 0_usize;
        for (left, right) in terms.keys() {
            let left_width = action
                .get(left)
                .map_or(1_usize, |shifts| shifts.len().saturating_add(1));
            let right_width = action
                .get(right)
                .map_or(1_usize, |shifts| shifts.len().saturating_add(1));
            let term_cost = left_width
                .checked_mul(right_width)
                .and_then(|value| value.checked_sub(1))
                .ok_or(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded)?;
            expansion_terms = expansion_terms
                .checked_add(term_cost)
                .ok_or(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded)?;
            if expansion_terms > MAX_VARIATION_TERMS {
                return Err(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded);
            }
        }
    }
    Ok(())
}

fn accumulate(
    polynomial: &mut Polynomial,
    left: VariationGenerator,
    right: VariationGenerator,
    coefficient: i128,
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    let key = if left <= right {
        (left, right)
    } else {
        (right, left)
    };
    let next = polynomial
        .get(&key)
        .copied()
        .unwrap_or(0)
        .checked_add(coefficient)
        .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
    if next == 0 {
        polynomial.remove(&key);
    } else {
        polynomial.insert(key, next);
    }
    Ok(())
}

fn encode(
    input: &SymmetryOperatorExclusionInput,
    fields: &[AlgebraIdentity],
    action: &ActionMap,
    operators: &[CanonicalOperator],
    evaluation: &SymmetryOperatorEvaluation,
) -> Result<Vec<u8>, SymmetryOperatorExclusionRefusal> {
    let mut output = Vec::new();
    field(&mut output, 1, RESULT_SCHEMA_ID.as_bytes())?;
    field(&mut output, 2, &input.claim_identity.0)?;
    field(&mut output, 3, &input.subject_identity.0)?;
    field(&mut output, 4, &input.action.symmetry_identity.0)?;
    field(&mut output, 5, &input.applicability_domain_identity.0)?;
    for identity in fields {
        field(&mut output, 10, &identity.0)?;
    }
    for (component, shifts) in action {
        for (shift, coefficient) in shifts {
            let mut record = Vec::new();
            field(&mut record, 1, &component.0)?;
            field(&mut record, 2, &shift.0)?;
            field(&mut record, 3, &coefficient.to_be_bytes())?;
            field(&mut output, 11, &record)?;
        }
    }
    for (identity, terms) in operators {
        let mut record = Vec::new();
        field(&mut record, 1, &identity.0)?;
        for ((left, right), coefficient) in terms {
            let mut term = Vec::new();
            field(&mut term, 1, &left.0)?;
            field(&mut term, 2, &right.0)?;
            field(&mut term, 3, &coefficient.to_be_bytes())?;
            field(&mut record, 2, &term)?;
        }
        field(&mut output, 12, &record)?;
    }
    for operator in &evaluation.operators {
        let mut record = Vec::new();
        field(&mut record, 1, &operator.operator_identity.0)?;
        for term in &operator.terms {
            let mut encoded = Vec::new();
            field(&mut encoded, 1, &[generator_tag(term.first.kind)])?;
            field(&mut encoded, 2, &term.first.identity.0)?;
            field(&mut encoded, 3, &[generator_tag(term.second.kind)])?;
            field(&mut encoded, 4, &term.second.identity.0)?;
            field(&mut encoded, 5, &term.coefficient.to_be_bytes())?;
            field(&mut record, 2, &encoded)?;
        }
        field(&mut output, 13, &record)?;
    }
    Ok(output)
}

const fn generator_tag(kind: GeneratorKind) -> u8 {
    match kind {
        GeneratorKind::Field => 0,
        GeneratorKind::Shift => 1,
    }
}

fn field(
    output: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    let length = u64::try_from(payload.len())
        .map_err(|_| SymmetryOperatorExclusionRefusal::VariationCapacityExceeded)?;
    output.extend_from_slice(&tag.to_be_bytes());
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(payload);
    Ok(())
}
