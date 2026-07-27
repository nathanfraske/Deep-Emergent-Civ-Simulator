// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Sorted-field nested-pair quadratic basis derivation.

use super::{
    AlgebraIdentity, BasisCheckerOutput, QuadraticBasisElement, QuadraticOperatorBasis,
    QuadraticOperatorBasisInput, QuadraticOperatorBasisRefusal, BASIS_SCHEMA_ID,
    MAX_FIELD_COMPONENTS, MAX_OPERATORS, MONOMIAL_ID_DOMAIN,
};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

pub(super) fn derive(
    input: &QuadraticOperatorBasisInput,
) -> Result<BasisCheckerOutput, QuadraticOperatorBasisRefusal> {
    validate_identity(
        input.claim_identity,
        QuadraticOperatorBasisRefusal::InvalidClaimIdentity,
    )?;
    validate_identity(
        input.subject_identity,
        QuadraticOperatorBasisRefusal::InvalidSubjectIdentity,
    )?;
    validate_identity(
        input.applicability_domain_identity,
        QuadraticOperatorBasisRefusal::InvalidApplicabilityDomainIdentity,
    )?;
    let fields = canonical_fields(&input.field_components)?;
    let count = fields
        .len()
        .checked_mul(fields.len().saturating_add(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(QuadraticOperatorBasisRefusal::BasisCapacityExceeded)?;
    if count > MAX_OPERATORS {
        return Err(QuadraticOperatorBasisRefusal::BasisCapacityExceeded);
    }

    let mut identities = BTreeSet::new();
    let mut elements = Vec::with_capacity(count);
    for left_index in 0..fields.len() {
        for right_index in left_index..fields.len() {
            let left_field = fields[left_index];
            let right_field = fields[right_index];
            let operator_identity = derive_operator_identity(left_field, right_field)?;
            if !identities.insert(operator_identity) {
                return Err(QuadraticOperatorBasisRefusal::OperatorIdentityCollision);
            }
            elements.push(QuadraticBasisElement {
                operator_identity,
                left_field,
                right_field,
            });
        }
    }
    let basis = QuadraticOperatorBasis {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        field_components: fields,
        elements,
    };
    let canonical_bytes = encode(&basis)?;
    Ok(BasisCheckerOutput {
        basis,
        canonical_bytes,
    })
}

fn validate_identity(
    identity: AlgebraIdentity,
    refusal: QuadraticOperatorBasisRefusal,
) -> Result<(), QuadraticOperatorBasisRefusal> {
    if identity.0 == [0; 32] {
        Err(refusal)
    } else {
        Ok(())
    }
}

fn canonical_fields(
    input: &[AlgebraIdentity],
) -> Result<Vec<AlgebraIdentity>, QuadraticOperatorBasisRefusal> {
    if input.is_empty() {
        return Err(QuadraticOperatorBasisRefusal::EmptyFieldBasis);
    }
    if input.len() > MAX_FIELD_COMPONENTS {
        return Err(QuadraticOperatorBasisRefusal::FieldCapacityExceeded);
    }
    let mut fields = input.to_vec();
    if fields.iter().any(|identity| identity.0 == [0; 32]) {
        return Err(QuadraticOperatorBasisRefusal::InvalidFieldComponentIdentity);
    }
    fields.sort_unstable();
    if fields.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(QuadraticOperatorBasisRefusal::DuplicateFieldComponent);
    }
    Ok(fields)
}

fn derive_operator_identity(
    left: AlgebraIdentity,
    right: AlgebraIdentity,
) -> Result<AlgebraIdentity, QuadraticOperatorBasisRefusal> {
    let mut preimage = Vec::with_capacity(MONOMIAL_ID_DOMAIN.len() + 64);
    preimage.extend_from_slice(MONOMIAL_ID_DOMAIN);
    preimage.extend_from_slice(&left.0);
    preimage.extend_from_slice(&right.0);
    let identity = AlgebraIdentity(sha256(&preimage));
    if identity.0 == [0; 32] {
        Err(QuadraticOperatorBasisRefusal::DerivedOperatorIdentityInvalid)
    } else {
        Ok(identity)
    }
}

fn encode(basis: &QuadraticOperatorBasis) -> Result<Vec<u8>, QuadraticOperatorBasisRefusal> {
    let mut output = Vec::new();
    field(&mut output, 1, BASIS_SCHEMA_ID.as_bytes())?;
    field(&mut output, 2, &basis.claim_identity.0)?;
    field(&mut output, 3, &basis.subject_identity.0)?;
    field(&mut output, 4, &basis.applicability_domain_identity.0)?;
    for identity in &basis.field_components {
        field(&mut output, 10, &identity.0)?;
    }
    for element in &basis.elements {
        let mut record = Vec::new();
        field(&mut record, 1, &element.operator_identity.0)?;
        field(&mut record, 2, &element.left_field.0)?;
        field(&mut record, 3, &element.right_field.0)?;
        field(&mut output, 11, &record)?;
    }
    Ok(output)
}

fn field(
    output: &mut Vec<u8>,
    tag: u16,
    payload: &[u8],
) -> Result<(), QuadraticOperatorBasisRefusal> {
    let length = u64::try_from(payload.len())
        .map_err(|_| QuadraticOperatorBasisRefusal::BasisCapacityExceeded)?;
    output.extend_from_slice(&tag.to_be_bytes());
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(payload);
    Ok(())
}
