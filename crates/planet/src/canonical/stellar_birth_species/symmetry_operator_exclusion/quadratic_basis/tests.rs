use super::*;
use std::collections::BTreeSet;

fn identity(byte: u8) -> AlgebraIdentity {
    AlgebraIdentity([byte; 32])
}

fn numbered_identity(number: u64) -> AlgebraIdentity {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&number.to_be_bytes());
    bytes[31] = 1;
    AlgebraIdentity(bytes)
}

fn input(fields: Vec<AlgebraIdentity>) -> QuadraticOperatorBasisInput {
    QuadraticOperatorBasisInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        field_components: fields,
    }
}

fn assert_both_refuse(
    input: &QuadraticOperatorBasisInput,
    expected: QuadraticOperatorBasisRefusal,
) {
    assert_eq!(producer::derive(input), Err(expected));
    assert_eq!(watchdog::derive(input), Err(expected));
    assert_eq!(inspect_quadratic_operator_basis(input), Err(expected));
}

#[test]
fn three_fields_derive_all_six_unordered_quadratic_monomials() {
    let report =
        inspect_quadratic_operator_basis(&input(vec![identity(10), identity(11), identity(12)]))
            .unwrap();
    assert_eq!(report.producer_id, PRODUCER_ID);
    assert_eq!(report.watchdog_id, WATCHDOG_ID);
    assert_ne!(report.producer_id, report.watchdog_id);
    assert_eq!(report.producer_result_sha256, report.watchdog_result_sha256);
    assert_ne!(report.producer_result_sha256, [0; 32]);
    assert_eq!(report.element_count(), 6);
    let pairs = report
        .basis
        .elements
        .iter()
        .map(|element| (element.left_field, element.right_field))
        .collect::<Vec<_>>();
    assert_eq!(
        pairs,
        vec![
            (identity(10), identity(10)),
            (identity(10), identity(11)),
            (identity(10), identity(12)),
            (identity(11), identity(11)),
            (identity(11), identity(12)),
            (identity(12), identity(12)),
        ]
    );
    assert!(report.scoped_homogeneous_quadratic_basis_coverage());
    assert!(!report.global_operator_basis_coverage());
    assert!(!report.field_ontology_authority());
    assert!(!report.applicability_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn field_arrival_order_does_not_change_basis_bytes_or_identities() {
    let baseline =
        inspect_quadratic_operator_basis(&input(vec![identity(10), identity(11), identity(12)]))
            .unwrap();
    let permuted =
        inspect_quadratic_operator_basis(&input(vec![identity(12), identity(10), identity(11)]))
            .unwrap();
    assert_eq!(baseline, permuted);
}

#[test]
fn alien_field_extension_preserves_every_existing_monomial_identity() {
    let baseline =
        inspect_quadratic_operator_basis(&input(vec![identity(10), identity(11)])).unwrap();
    let extended =
        inspect_quadratic_operator_basis(&input(vec![identity(200), identity(11), identity(10)]))
            .unwrap();
    assert_eq!(baseline.element_count(), 3);
    assert_eq!(extended.element_count(), 6);
    for element in &baseline.basis.elements {
        assert!(extended.basis.elements.contains(element));
    }
}

#[test]
fn generated_operator_candidates_have_one_exact_content_derived_term() {
    let report =
        inspect_quadratic_operator_basis(&input(vec![identity(10), identity(11)])).unwrap();
    let candidates = report.operator_candidates();
    assert_eq!(candidates.len(), 3);
    for (candidate, element) in candidates.iter().zip(&report.basis.elements) {
        assert_eq!(candidate.operator_identity, element.operator_identity);
        assert_eq!(
            candidate.terms,
            vec![QuadraticOperatorTerm {
                left_field: element.left_field,
                right_field: element.right_field,
                coefficient: 1,
            }]
        );
    }
}

#[test]
fn malformed_and_over_budget_field_sets_refuse_identically() {
    let mut invalid_claim = input(vec![identity(10)]);
    invalid_claim.claim_identity = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &invalid_claim,
        QuadraticOperatorBasisRefusal::InvalidClaimIdentity,
    );

    let mut invalid_subject = input(vec![identity(10)]);
    invalid_subject.subject_identity = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &invalid_subject,
        QuadraticOperatorBasisRefusal::InvalidSubjectIdentity,
    );

    let mut invalid_domain = input(vec![identity(10)]);
    invalid_domain.applicability_domain_identity = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &invalid_domain,
        QuadraticOperatorBasisRefusal::InvalidApplicabilityDomainIdentity,
    );

    assert_both_refuse(
        &input(Vec::new()),
        QuadraticOperatorBasisRefusal::EmptyFieldBasis,
    );

    let mut duplicate = input(vec![identity(10), identity(10)]);
    assert_both_refuse(
        &duplicate,
        QuadraticOperatorBasisRefusal::DuplicateFieldComponent,
    );
    duplicate.field_components[1] = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &duplicate,
        QuadraticOperatorBasisRefusal::InvalidFieldComponentIdentity,
    );

    let fields = (1..=45).map(numbered_identity).collect();
    assert_both_refuse(
        &input(fields),
        QuadraticOperatorBasisRefusal::BasisCapacityExceeded,
    );

    let fields = (1..=maximum_field_components() as u64 + 1)
        .map(numbered_identity)
        .collect();
    assert_both_refuse(
        &input(fields),
        QuadraticOperatorBasisRefusal::FieldCapacityExceeded,
    );
}

#[test]
fn small_alien_identity_grid_has_no_monomial_collision() {
    let fields = (1..=16).map(numbered_identity).collect();
    let report = inspect_quadratic_operator_basis(&input(fields)).unwrap();
    assert_eq!(report.element_count(), 136);
    let identities = report
        .basis
        .elements
        .iter()
        .map(|element| element.operator_identity)
        .collect::<BTreeSet<_>>();
    assert_eq!(identities.len(), report.element_count());
}

#[test]
fn every_supported_cardinality_has_the_exact_triangular_basis_size() {
    for count in 1_u64..=44 {
        let fields = (1..=count).map(numbered_identity).collect();
        let report = inspect_quadratic_operator_basis(&input(fields)).unwrap();
        let expected = usize::try_from(count * (count + 1) / 2).unwrap();
        assert_eq!(report.element_count(), expected);
    }
}

#[test]
fn production_has_no_admitted_field_ontology() {
    let refusal = repository_quadratic_operator_basis().unwrap_err();
    assert_eq!(
        refusal,
        QuadraticOperatorBasisRefusal::NoAdmittedFieldOntologyInput
    );
    assert_eq!(refusal.id(), "no_admitted_field_ontology_input");
    assert_eq!(maximum_basis_elements(), MAX_OPERATORS);
}
