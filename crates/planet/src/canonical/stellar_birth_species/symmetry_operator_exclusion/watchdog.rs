// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Direct exact construction of the affine variation.

use super::{
    AlgebraIdentity, CheckerOutput, ExactVariationTerm, GeneratorKind, OperatorVariation,
    SymmetryOperatorEvaluation, SymmetryOperatorExclusionInput, SymmetryOperatorExclusionRefusal,
    VariationGenerator, MAX_ACTION_TERMS, MAX_FIELD_COMPONENTS, MAX_OPERATORS, MAX_OPERATOR_TERMS,
    MAX_TOTAL_OPERATOR_TERMS, MAX_TOTAL_VARIATION_TERMS, MAX_VARIATION_TERMS, RESULT_SCHEMA_ID,
};
use std::collections::BTreeMap;

type ShiftMap = BTreeMap<AlgebraIdentity, Vec<(AlgebraIdentity, i128)>>;
type CanonicalOperator = (
    AlgebraIdentity,
    Vec<(AlgebraIdentity, AlgebraIdentity, i128)>,
);

pub(super) fn evaluate(
    input: &SymmetryOperatorExclusionInput,
) -> Result<CheckerOutput, SymmetryOperatorExclusionRefusal> {
    if input.claim_identity.0.iter().all(|byte| *byte == 0) {
        return Err(SymmetryOperatorExclusionRefusal::InvalidClaimIdentity);
    }
    if input.subject_identity.0.iter().all(|byte| *byte == 0) {
        return Err(SymmetryOperatorExclusionRefusal::InvalidSubjectIdentity);
    }
    if input
        .action
        .symmetry_identity
        .0
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(SymmetryOperatorExclusionRefusal::InvalidSymmetryIdentity);
    }
    if input
        .applicability_domain_identity
        .0
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(SymmetryOperatorExclusionRefusal::InvalidApplicabilityDomainIdentity);
    }

    let fields = inspect_fields(&input.field_components)?;
    let shifts = inspect_action(input, &fields)?;
    let operators = inspect_operators(input, &fields)?;
    inspect_variation_cost(&shifts, &operators)?;
    let mut results = Vec::with_capacity(operators.len());
    for (operator_identity, terms) in &operators {
        let mut raw = Vec::<ExactVariationTerm>::new();
        for (left, right, coefficient) in terms.iter().rev() {
            let left_shifts = shifts.get(left).map(Vec::as_slice).unwrap_or(&[]);
            let right_shifts = shifts.get(right).map(Vec::as_slice).unwrap_or(&[]);
            for (shift, action_coefficient) in left_shifts.iter().rev() {
                let coefficient = coefficient
                    .checked_mul(*action_coefficient)
                    .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                raw.push(canonical_term(
                    VariationGenerator {
                        kind: GeneratorKind::Shift,
                        identity: *shift,
                    },
                    VariationGenerator {
                        kind: GeneratorKind::Field,
                        identity: *right,
                    },
                    coefficient,
                ));
            }
            for (shift, action_coefficient) in right_shifts.iter().rev() {
                let coefficient = coefficient
                    .checked_mul(*action_coefficient)
                    .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                raw.push(canonical_term(
                    VariationGenerator {
                        kind: GeneratorKind::Field,
                        identity: *left,
                    },
                    VariationGenerator {
                        kind: GeneratorKind::Shift,
                        identity: *shift,
                    },
                    coefficient,
                ));
            }
            for (left_shift, left_coefficient) in left_shifts.iter().rev() {
                for (right_shift, right_coefficient) in right_shifts.iter().rev() {
                    let coefficient = coefficient
                        .checked_mul(*left_coefficient)
                        .and_then(|value| value.checked_mul(*right_coefficient))
                        .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                    raw.push(canonical_term(
                        VariationGenerator {
                            kind: GeneratorKind::Shift,
                            identity: *left_shift,
                        },
                        VariationGenerator {
                            kind: GeneratorKind::Shift,
                            identity: *right_shift,
                        },
                        coefficient,
                    ));
                }
            }
            if raw.len() > MAX_VARIATION_TERMS {
                return Err(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded);
            }
        }
        raw.sort_unstable_by_key(|term| (term.first, term.second));
        let mut reduced = Vec::<ExactVariationTerm>::new();
        for term in raw {
            if let Some(last) = reduced.last_mut() {
                if last.first == term.first && last.second == term.second {
                    last.coefficient = last
                        .coefficient
                        .checked_add(term.coefficient)
                        .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                    if last.coefficient == 0 {
                        reduced.pop();
                    }
                    continue;
                }
            }
            if term.coefficient != 0 {
                reduced.push(term);
                if reduced.len() > MAX_VARIATION_TERMS {
                    return Err(SymmetryOperatorExclusionRefusal::VariationCapacityExceeded);
                }
            }
        }
        results.push(OperatorVariation {
            operator_identity: *operator_identity,
            terms: reduced,
        });
    }

    let evaluation = SymmetryOperatorEvaluation {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        symmetry_identity: input.action.symmetry_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        operators: results,
    };
    let canonical_bytes = encode_result(input, &fields, &shifts, &operators, &evaluation)?;
    Ok(CheckerOutput {
        evaluation,
        canonical_bytes,
    })
}

fn inspect_fields(
    input: &[AlgebraIdentity],
) -> Result<Vec<AlgebraIdentity>, SymmetryOperatorExclusionRefusal> {
    let count = input.len();
    if count == 0 {
        return Err(SymmetryOperatorExclusionRefusal::EmptyFieldBasis);
    }
    if count > MAX_FIELD_COMPONENTS {
        return Err(SymmetryOperatorExclusionRefusal::FieldCapacityExceeded);
    }
    let mut by_identity = BTreeMap::new();
    for identity in input.iter().rev() {
        if identity.0.iter().all(|byte| *byte == 0) {
            return Err(SymmetryOperatorExclusionRefusal::InvalidFieldComponentIdentity);
        }
        if by_identity.insert(*identity, ()).is_some() {
            return Err(SymmetryOperatorExclusionRefusal::DuplicateFieldComponent);
        }
    }
    Ok(by_identity.into_keys().collect())
}

fn inspect_action(
    input: &SymmetryOperatorExclusionInput,
    fields: &[AlgebraIdentity],
) -> Result<ShiftMap, SymmetryOperatorExclusionRefusal> {
    if input.action.terms.len() > MAX_ACTION_TERMS {
        return Err(SymmetryOperatorExclusionRefusal::ActionTermCapacityExceeded);
    }
    let field_index = fields
        .iter()
        .enumerate()
        .map(|(index, identity)| (*identity, index))
        .collect::<BTreeMap<_, _>>();
    let mut raw = BTreeMap::<AlgebraIdentity, Vec<(AlgebraIdentity, i128)>>::new();
    for term in input.action.terms.iter().rev() {
        if term.coefficient == 0 || term.shift_generator.0.iter().all(|byte| *byte == 0) {
            return Err(SymmetryOperatorExclusionRefusal::InvalidActionTerm);
        }
        if !field_index.contains_key(&term.field_component) {
            return Err(SymmetryOperatorExclusionRefusal::ActionReferencesUnknownField);
        }
        raw.entry(term.field_component)
            .or_default()
            .push((term.shift_generator, i128::from(term.coefficient)));
    }
    for terms in raw.values_mut() {
        terms.sort_unstable_by_key(|(identity, _)| *identity);
        let mut position = 0;
        while position + 1 < terms.len() {
            if terms[position].0 != terms[position + 1].0 {
                position += 1;
                continue;
            }
            let next = terms[position]
                .1
                .checked_add(terms[position + 1].1)
                .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
            terms[position].1 = next;
            terms.remove(position + 1);
            if next == 0 {
                terms.remove(position);
            }
        }
    }
    raw.retain(|_, terms| !terms.is_empty());
    Ok(raw)
}

fn inspect_operators(
    input: &SymmetryOperatorExclusionInput,
    fields: &[AlgebraIdentity],
) -> Result<Vec<CanonicalOperator>, SymmetryOperatorExclusionRefusal> {
    if input.operators.is_empty() {
        return Err(SymmetryOperatorExclusionRefusal::EmptyOperatorSet);
    }
    if input.operators.len() > MAX_OPERATORS {
        return Err(SymmetryOperatorExclusionRefusal::OperatorCapacityExceeded);
    }
    let mut total_terms = 0_usize;
    let field_index = fields
        .iter()
        .enumerate()
        .map(|(index, identity)| (*identity, index))
        .collect::<BTreeMap<_, _>>();
    let mut by_operator = BTreeMap::<AlgebraIdentity, Vec<_>>::new();
    for operator in input.operators.iter().rev() {
        if operator.operator_identity.0.iter().all(|byte| *byte == 0) {
            return Err(SymmetryOperatorExclusionRefusal::InvalidOperatorIdentity);
        }
        if operator.terms.len() > MAX_OPERATOR_TERMS {
            return Err(SymmetryOperatorExclusionRefusal::OperatorTermCapacityExceeded);
        }
        total_terms = total_terms
            .checked_add(operator.terms.len())
            .ok_or(SymmetryOperatorExclusionRefusal::TotalOperatorTermCapacityExceeded)?;
        if total_terms > MAX_TOTAL_OPERATOR_TERMS {
            return Err(SymmetryOperatorExclusionRefusal::TotalOperatorTermCapacityExceeded);
        }
        if by_operator.contains_key(&operator.operator_identity) {
            return Err(SymmetryOperatorExclusionRefusal::DuplicateOperatorIdentity);
        }
        let mut terms = Vec::with_capacity(operator.terms.len());
        for term in operator.terms.iter().rev() {
            if term.coefficient == 0 {
                return Err(SymmetryOperatorExclusionRefusal::InvalidOperatorTerm);
            }
            if !field_index.contains_key(&term.left_field)
                || !field_index.contains_key(&term.right_field)
            {
                return Err(SymmetryOperatorExclusionRefusal::OperatorReferencesUnknownField);
            }
            let (left, right) = if term.right_field < term.left_field {
                (term.right_field, term.left_field)
            } else {
                (term.left_field, term.right_field)
            };
            terms.push((left, right, i128::from(term.coefficient)));
        }
        terms.sort_unstable_by_key(|(left, right, _)| (*left, *right));
        let mut reduced = Vec::<(AlgebraIdentity, AlgebraIdentity, i128)>::new();
        for (left, right, coefficient) in terms {
            if let Some(last) = reduced.last_mut() {
                if last.0 == left && last.1 == right {
                    last.2 = last
                        .2
                        .checked_add(coefficient)
                        .ok_or(SymmetryOperatorExclusionRefusal::ArithmeticOverflow)?;
                    if last.2 == 0 {
                        reduced.pop();
                    }
                    continue;
                }
            }
            reduced.push((left, right, coefficient));
        }
        if reduced.is_empty() {
            return Err(SymmetryOperatorExclusionRefusal::ZeroOperator);
        }
        by_operator.insert(operator.operator_identity, reduced);
    }
    Ok(by_operator.into_iter().collect())
}

fn canonical_term(
    left: VariationGenerator,
    right: VariationGenerator,
    coefficient: i128,
) -> ExactVariationTerm {
    if right < left {
        ExactVariationTerm {
            first: right,
            second: left,
            coefficient,
        }
    } else {
        ExactVariationTerm {
            first: left,
            second: right,
            coefficient,
        }
    }
}

fn inspect_variation_cost(
    shifts: &ShiftMap,
    operators: &[CanonicalOperator],
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    let mut total_expansion_terms = 0_usize;
    for (_, terms) in operators {
        let mut expansion_terms = 0_usize;
        for (left, right, _) in terms {
            let left_width = shifts
                .get(left)
                .map_or(1_usize, |terms| terms.len().saturating_add(1));
            let right_width = shifts
                .get(right)
                .map_or(1_usize, |terms| terms.len().saturating_add(1));
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
        total_expansion_terms = total_expansion_terms
            .checked_add(expansion_terms)
            .ok_or(SymmetryOperatorExclusionRefusal::TotalVariationCapacityExceeded)?;
        if total_expansion_terms > MAX_TOTAL_VARIATION_TERMS {
            return Err(SymmetryOperatorExclusionRefusal::TotalVariationCapacityExceeded);
        }
    }
    Ok(())
}

fn encode_result(
    input: &SymmetryOperatorExclusionInput,
    fields: &[AlgebraIdentity],
    shifts: &ShiftMap,
    operators: &[CanonicalOperator],
    evaluation: &SymmetryOperatorEvaluation,
) -> Result<Vec<u8>, SymmetryOperatorExclusionRefusal> {
    let mut bytes = Vec::new();
    push(&mut bytes, 1, RESULT_SCHEMA_ID.as_bytes())?;
    push(&mut bytes, 2, &input.claim_identity.0)?;
    push(&mut bytes, 3, &input.subject_identity.0)?;
    push(&mut bytes, 4, &input.action.symmetry_identity.0)?;
    push(&mut bytes, 5, &input.applicability_domain_identity.0)?;
    for identity in fields {
        push(&mut bytes, 10, &identity.0)?;
    }
    for (field_identity, action_terms) in shifts {
        for (shift_identity, coefficient) in action_terms {
            let mut entry = Vec::new();
            push(&mut entry, 1, &field_identity.0)?;
            push(&mut entry, 2, &shift_identity.0)?;
            push(&mut entry, 3, &coefficient.to_be_bytes())?;
            push(&mut bytes, 11, &entry)?;
        }
    }
    for (operator_identity, terms) in operators {
        let mut entry = Vec::new();
        push(&mut entry, 1, &operator_identity.0)?;
        for (left, right, coefficient) in terms {
            let mut term = Vec::new();
            push(&mut term, 1, &left.0)?;
            push(&mut term, 2, &right.0)?;
            push(&mut term, 3, &coefficient.to_be_bytes())?;
            push(&mut entry, 2, &term)?;
        }
        push(&mut bytes, 12, &entry)?;
    }
    for operator in &evaluation.operators {
        let mut entry = Vec::new();
        push(&mut entry, 1, &operator.operator_identity.0)?;
        for term in &operator.terms {
            let mut encoded = Vec::new();
            let first_tag = match term.first.kind {
                GeneratorKind::Field => 0_u8,
                GeneratorKind::Shift => 1_u8,
            };
            let second_tag = match term.second.kind {
                GeneratorKind::Field => 0_u8,
                GeneratorKind::Shift => 1_u8,
            };
            push(&mut encoded, 1, &[first_tag])?;
            push(&mut encoded, 2, &term.first.identity.0)?;
            push(&mut encoded, 3, &[second_tag])?;
            push(&mut encoded, 4, &term.second.identity.0)?;
            push(&mut encoded, 5, &term.coefficient.to_be_bytes())?;
            push(&mut entry, 2, &encoded)?;
        }
        push(&mut bytes, 13, &entry)?;
    }
    Ok(bytes)
}

fn push(
    bytes: &mut Vec<u8>,
    tag: u16,
    value: &[u8],
) -> Result<(), SymmetryOperatorExclusionRefusal> {
    let size = u64::try_from(value.len())
        .map_err(|_| SymmetryOperatorExclusionRefusal::VariationCapacityExceeded)?;
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}
