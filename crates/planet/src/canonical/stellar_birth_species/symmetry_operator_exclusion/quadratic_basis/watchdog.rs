// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Ordered-set triangular-index quadratic basis reconstruction.

use super::{
    AlgebraIdentity, BasisCheckerOutput, QuadraticBasisElement, QuadraticOperatorBasis,
    QuadraticOperatorBasisInput, QuadraticOperatorBasisRefusal, BASIS_SCHEMA_ID,
    MAX_FIELD_COMPONENTS, MAX_OPERATORS, MONOMIAL_ID_DOMAIN,
};
use civsim_units::digest::sha256;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn derive(
    input: &QuadraticOperatorBasisInput,
) -> Result<BasisCheckerOutput, QuadraticOperatorBasisRefusal> {
    if input.claim_identity.0.iter().all(|byte| *byte == 0) {
        return Err(QuadraticOperatorBasisRefusal::InvalidClaimIdentity);
    }
    if input.subject_identity.0.iter().all(|byte| *byte == 0) {
        return Err(QuadraticOperatorBasisRefusal::InvalidSubjectIdentity);
    }
    if input
        .applicability_domain_identity
        .0
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(QuadraticOperatorBasisRefusal::InvalidApplicabilityDomainIdentity);
    }
    if input.field_components.is_empty() {
        return Err(QuadraticOperatorBasisRefusal::EmptyFieldBasis);
    }
    if input.field_components.len() > MAX_FIELD_COMPONENTS {
        return Err(QuadraticOperatorBasisRefusal::FieldCapacityExceeded);
    }
    if input
        .field_components
        .iter()
        .any(|identity| identity.0.iter().all(|byte| *byte == 0))
    {
        return Err(QuadraticOperatorBasisRefusal::InvalidFieldComponentIdentity);
    }
    let field_set = input
        .field_components
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if field_set.len() != input.field_components.len() {
        return Err(QuadraticOperatorBasisRefusal::DuplicateFieldComponent);
    }
    let fields = field_set.into_iter().collect::<Vec<_>>();
    let count = (1..=fields.len()).try_fold(0_usize, |total, width| total.checked_add(width));
    let count = count.ok_or(QuadraticOperatorBasisRefusal::BasisCapacityExceeded)?;
    if count > MAX_OPERATORS {
        return Err(QuadraticOperatorBasisRefusal::BasisCapacityExceeded);
    }

    let mut by_identity = BTreeMap::new();
    for (right_index, right_field) in fields.iter().copied().enumerate() {
        for left_field in fields[..=right_index].iter().copied() {
            let operator_identity = reconstruct_operator_identity(left_field, right_field)?;
            let element = QuadraticBasisElement {
                operator_identity,
                left_field,
                right_field,
            };
            if by_identity.insert(operator_identity, element).is_some() {
                return Err(QuadraticOperatorBasisRefusal::OperatorIdentityCollision);
            }
        }
    }
    let mut elements = by_identity.into_values().collect::<Vec<_>>();
    elements.sort_unstable_by_key(|element| (element.left_field, element.right_field));
    let basis = QuadraticOperatorBasis {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        field_components: fields,
        elements,
    };
    let canonical_bytes = serialize(&basis)?;
    Ok(BasisCheckerOutput {
        basis,
        canonical_bytes,
    })
}

fn reconstruct_operator_identity(
    left: AlgebraIdentity,
    right: AlgebraIdentity,
) -> Result<AlgebraIdentity, QuadraticOperatorBasisRefusal> {
    let parts = [MONOMIAL_ID_DOMAIN, left.0.as_slice(), right.0.as_slice()];
    let total = parts.iter().map(|part| part.len()).sum();
    let mut bytes = Vec::with_capacity(total);
    for part in parts {
        bytes.extend_from_slice(part);
    }
    let identity = AlgebraIdentity(sha256(&bytes));
    if identity.0.iter().all(|byte| *byte == 0) {
        Err(QuadraticOperatorBasisRefusal::DerivedOperatorIdentityInvalid)
    } else {
        Ok(identity)
    }
}

fn serialize(basis: &QuadraticOperatorBasis) -> Result<Vec<u8>, QuadraticOperatorBasisRefusal> {
    let mut fields = BTreeMap::<u16, Vec<Vec<u8>>>::new();
    fields
        .entry(1)
        .or_default()
        .push(BASIS_SCHEMA_ID.as_bytes().to_vec());
    fields
        .entry(2)
        .or_default()
        .push(basis.claim_identity.0.to_vec());
    fields
        .entry(3)
        .or_default()
        .push(basis.subject_identity.0.to_vec());
    fields
        .entry(4)
        .or_default()
        .push(basis.applicability_domain_identity.0.to_vec());
    for identity in &basis.field_components {
        fields.entry(10).or_default().push(identity.0.to_vec());
    }
    for element in &basis.elements {
        let mut record = Vec::new();
        push(&mut record, 1, &element.operator_identity.0)?;
        push(&mut record, 2, &element.left_field.0)?;
        push(&mut record, 3, &element.right_field.0)?;
        fields.entry(11).or_default().push(record);
    }
    let mut output = Vec::new();
    for (tag, payloads) in fields {
        for payload in payloads {
            push(&mut output, tag, &payload)?;
        }
    }
    Ok(output)
}

fn push(
    output: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
) -> Result<(), QuadraticOperatorBasisRefusal> {
    let size = u64::try_from(payload.len())
        .map_err(|_| QuadraticOperatorBasisRefusal::BasisCapacityExceeded)?;
    output.extend_from_slice(&tag.to_be_bytes());
    output.extend_from_slice(&size.to_be_bytes());
    output.extend_from_slice(payload);
    Ok(())
}
