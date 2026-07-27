// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0; see LICENSE.

//! Complete monomial basis for a supplied opaque field-component set.
//!
//! This pair derives every unordered degree-two monomial over the supplied
//! components. It proves completeness only for that finite homogeneous
//! commutative quadratic polynomial space. It cannot prove that the supplied
//! fields are a complete physical ontology, that every physically relevant
//! operator is quadratic, or that an action applies to the claimed subject.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::{
    AlgebraIdentity, QuadraticOperatorCandidate, QuadraticOperatorTerm, MAX_FIELD_COMPONENTS,
    MAX_OPERATORS,
};
use civsim_units::digest::sha256;

const BASIS_SCHEMA_ID: &str = "civsim.planet.complete-quadratic-operator-basis.v1";
const MONOMIAL_ID_DOMAIN: &[u8] = b"civsim.planet.quadratic-monomial-identity.v1";
const PRODUCER_ID: &str = "civsim.planet.quadratic-operator-basis.nested-pair-producer.v1";
const WATCHDOG_ID: &str = "civsim.planet.quadratic-operator-basis.triangular-index-watchdog.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuadraticOperatorBasisInput {
    pub(super) claim_identity: AlgebraIdentity,
    pub(super) subject_identity: AlgebraIdentity,
    pub(super) applicability_domain_identity: AlgebraIdentity,
    pub(super) field_components: Vec<AlgebraIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct QuadraticBasisElement {
    pub(super) operator_identity: AlgebraIdentity,
    pub(super) left_field: AlgebraIdentity,
    pub(super) right_field: AlgebraIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuadraticOperatorBasis {
    pub(super) claim_identity: AlgebraIdentity,
    pub(super) subject_identity: AlgebraIdentity,
    pub(super) applicability_domain_identity: AlgebraIdentity,
    pub(super) field_components: Vec<AlgebraIdentity>,
    pub(super) elements: Vec<QuadraticBasisElement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BasisCheckerOutput {
    pub(super) basis: QuadraticOperatorBasis,
    pub(super) canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct QuadraticOperatorBasisReport {
    basis: QuadraticOperatorBasis,
    producer_id: &'static str,
    watchdog_id: &'static str,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
}

impl QuadraticOperatorBasisReport {
    pub(super) const fn schema_id(&self) -> &'static str {
        BASIS_SCHEMA_ID
    }

    pub(super) const fn producer_id(&self) -> &'static str {
        self.producer_id
    }

    pub(super) const fn watchdog_id(&self) -> &'static str {
        self.watchdog_id
    }

    pub(super) fn element_count(&self) -> usize {
        self.basis.elements.len()
    }

    pub(super) fn operator_candidates(&self) -> Vec<QuadraticOperatorCandidate> {
        self.basis
            .elements
            .iter()
            .map(|element| QuadraticOperatorCandidate {
                operator_identity: element.operator_identity,
                terms: vec![QuadraticOperatorTerm {
                    left_field: element.left_field,
                    right_field: element.right_field,
                    coefficient: 1,
                }],
            })
            .collect()
    }

    pub(super) const fn producer_result_sha256(&self) -> [u8; 32] {
        self.producer_result_sha256
    }

    pub(super) const fn watchdog_result_sha256(&self) -> [u8; 32] {
        self.watchdog_result_sha256
    }

    pub(super) const fn scoped_homogeneous_quadratic_basis_coverage(&self) -> bool {
        true
    }

    pub(super) const fn global_operator_basis_coverage(&self) -> bool {
        false
    }

    pub(super) const fn field_ontology_authority(&self) -> bool {
        false
    }

    pub(super) const fn applicability_authority(&self) -> bool {
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
pub(super) enum QuadraticOperatorBasisRefusal {
    NoAdmittedFieldOntologyInput,
    InvalidClaimIdentity,
    InvalidSubjectIdentity,
    InvalidApplicabilityDomainIdentity,
    EmptyFieldBasis,
    FieldCapacityExceeded,
    InvalidFieldComponentIdentity,
    DuplicateFieldComponent,
    BasisCapacityExceeded,
    DerivedOperatorIdentityInvalid,
    OperatorIdentityCollision,
    CheckerDisagreement,
}

impl QuadraticOperatorBasisRefusal {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::NoAdmittedFieldOntologyInput => "no_admitted_field_ontology_input",
            Self::InvalidClaimIdentity => "invalid_claim_identity",
            Self::InvalidSubjectIdentity => "invalid_subject_identity",
            Self::InvalidApplicabilityDomainIdentity => "invalid_applicability_domain_identity",
            Self::EmptyFieldBasis => "empty_field_basis",
            Self::FieldCapacityExceeded => "field_capacity_exceeded",
            Self::InvalidFieldComponentIdentity => "invalid_field_component_identity",
            Self::DuplicateFieldComponent => "duplicate_field_component",
            Self::BasisCapacityExceeded => "basis_capacity_exceeded",
            Self::DerivedOperatorIdentityInvalid => "derived_operator_identity_invalid",
            Self::OperatorIdentityCollision => "operator_identity_collision",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

pub(super) fn inspect_quadratic_operator_basis(
    input: &QuadraticOperatorBasisInput,
) -> Result<QuadraticOperatorBasisReport, QuadraticOperatorBasisRefusal> {
    let produced = producer::derive(input);
    let watched = watchdog::derive(input);
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.basis == watched.basis
                && produced.canonical_bytes == watched.canonical_bytes =>
        {
            let producer_result_sha256 = sha256(&produced.canonical_bytes);
            let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
            Ok(QuadraticOperatorBasisReport {
                basis: produced.basis,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                producer_result_sha256,
                watchdog_result_sha256,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(QuadraticOperatorBasisRefusal::CheckerDisagreement),
    }
}

fn repository_quadratic_operator_basis(
) -> Result<QuadraticOperatorBasisReport, QuadraticOperatorBasisRefusal> {
    Err(QuadraticOperatorBasisRefusal::NoAdmittedFieldOntologyInput)
}

const fn maximum_field_components() -> usize {
    MAX_FIELD_COMPONENTS
}

const fn maximum_basis_elements() -> usize {
    MAX_OPERATORS
}
