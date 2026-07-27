use super::*;

fn identity(byte: u8) -> AlgebraIdentity {
    AlgebraIdentity([byte; 32])
}

fn numbered_identity(number: u64) -> AlgebraIdentity {
    let mut bytes = [0_u8; 32];
    bytes[..8].copy_from_slice(&number.to_be_bytes());
    bytes[31] = 1;
    AlgebraIdentity(bytes)
}

fn action_term(field: u8, shift: u8, coefficient: i64) -> AffineActionTerm {
    AffineActionTerm {
        field_component: identity(field),
        shift_generator: identity(shift),
        coefficient,
    }
}

fn operator_term(left: u8, right: u8, coefficient: i64) -> QuadraticOperatorTerm {
    QuadraticOperatorTerm {
        left_field: identity(left),
        right_field: identity(right),
        coefficient,
    }
}

fn request(
    fields: Vec<u8>,
    action_terms: Vec<AffineActionTerm>,
    operators: Vec<QuadraticOperatorCandidate>,
) -> SymmetryOperatorExclusionInput {
    SymmetryOperatorExclusionInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        field_components: fields.into_iter().map(identity).collect(),
        action: AffineSymmetryAction {
            symmetry_identity: identity(4),
            terms: action_terms,
        },
        operators,
    }
}

fn mass_like_operator() -> QuadraticOperatorCandidate {
    QuadraticOperatorCandidate {
        operator_identity: identity(90),
        terms: vec![
            operator_term(10, 10, 1),
            operator_term(11, 11, -1),
            operator_term(12, 12, -1),
            operator_term(13, 13, -1),
        ],
    }
}

fn gauge_like_request() -> SymmetryOperatorExclusionInput {
    request(
        vec![10, 11, 12, 13],
        vec![
            action_term(10, 20, 1),
            action_term(11, 21, 1),
            action_term(12, 22, 1),
            action_term(13, 23, 1),
        ],
        vec![mass_like_operator()],
    )
}

fn assert_both_refuse(
    input: &SymmetryOperatorExclusionInput,
    expected: SymmetryOperatorExclusionRefusal,
) {
    assert_eq!(producer::evaluate(input), Err(expected));
    assert_eq!(watchdog::evaluate(input), Err(expected));
    assert_eq!(inspect_symmetry_operator_exclusion(input), Err(expected));
}

#[test]
fn affine_local_shift_excludes_the_supplied_quadratic_mass_operator() {
    let report = inspect_symmetry_operator_exclusion(&gauge_like_request()).unwrap();
    assert_eq!(report.producer_id, PRODUCER_ID);
    assert_eq!(report.watchdog_id, WATCHDOG_ID);
    assert_ne!(report.producer_id, report.watchdog_id);
    assert_eq!(report.producer_result_sha256, report.watchdog_result_sha256);
    assert_ne!(report.producer_result_sha256, [0; 32]);
    assert_eq!(report.excluded_operator_count(), 1);
    assert_eq!(report.invariant_operator_count(), 0);
    assert_eq!(report.evaluation.operators[0].terms.len(), 8);
    assert!(report.requested_operator_coverage());
    assert!(!report.global_operator_basis_coverage());
    assert!(!report.applicability_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn unchanged_spectator_operator_remains_invariant() {
    let input = request(
        vec![10, 11],
        vec![action_term(10, 20, 1)],
        vec![QuadraticOperatorCandidate {
            operator_identity: identity(91),
            terms: vec![operator_term(11, 11, 3)],
        }],
    );
    let report = inspect_symmetry_operator_exclusion(&input).unwrap();
    assert_eq!(report.excluded_operator_count(), 0);
    assert_eq!(report.invariant_operator_count(), 1);
}

#[test]
fn arrival_order_and_commutative_term_orientation_do_not_change_the_result() {
    let baseline = gauge_like_request();
    let baseline_report = inspect_symmetry_operator_exclusion(&baseline).unwrap();
    let mut permuted = baseline;
    permuted.field_components.reverse();
    permuted.action.terms.reverse();
    permuted.operators.reverse();
    permuted.operators[0].terms.reverse();
    for term in &mut permuted.operators[0].terms {
        std::mem::swap(&mut term.left_field, &mut term.right_field);
    }
    let permuted_report = inspect_symmetry_operator_exclusion(&permuted).unwrap();
    assert_eq!(baseline_report, permuted_report);
}

#[test]
fn unrelated_alien_operator_extension_does_not_relabel_existing_variation() {
    let baseline = gauge_like_request();
    let baseline_report = inspect_symmetry_operator_exclusion(&baseline).unwrap();
    let mut extended = baseline;
    extended.field_components.push(identity(200));
    extended.operators.push(QuadraticOperatorCandidate {
        operator_identity: identity(201),
        terms: vec![operator_term(200, 200, 7)],
    });
    let extended_report = inspect_symmetry_operator_exclusion(&extended).unwrap();
    assert_eq!(
        baseline_report.evaluation.operators[0],
        extended_report.evaluation.operators[0]
    );
    assert_eq!(extended_report.excluded_operator_count(), 1);
    assert_eq!(extended_report.invariant_operator_count(), 1);
}

#[test]
fn malformed_identities_and_unknown_fields_refuse_identically() {
    let mut invalid = gauge_like_request();
    invalid.claim_identity = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &invalid,
        SymmetryOperatorExclusionRefusal::InvalidClaimIdentity,
    );

    let mut duplicate = gauge_like_request();
    duplicate.field_components.push(identity(10));
    assert_both_refuse(
        &duplicate,
        SymmetryOperatorExclusionRefusal::DuplicateFieldComponent,
    );

    let mut zero_field = gauge_like_request();
    zero_field.field_components[0] = AlgebraIdentity([0; 32]);
    assert_both_refuse(
        &zero_field,
        SymmetryOperatorExclusionRefusal::InvalidFieldComponentIdentity,
    );

    let mut unknown_action = gauge_like_request();
    unknown_action.action.terms[0].field_component = identity(199);
    assert_both_refuse(
        &unknown_action,
        SymmetryOperatorExclusionRefusal::ActionReferencesUnknownField,
    );

    let mut unknown_operator = gauge_like_request();
    unknown_operator.operators[0].terms[0].left_field = identity(199);
    assert_both_refuse(
        &unknown_operator,
        SymmetryOperatorExclusionRefusal::OperatorReferencesUnknownField,
    );
}

#[test]
fn empty_duplicate_zero_and_oversized_operator_sets_refuse() {
    let mut empty = gauge_like_request();
    empty.operators.clear();
    assert_both_refuse(&empty, SymmetryOperatorExclusionRefusal::EmptyOperatorSet);

    let mut duplicate = gauge_like_request();
    duplicate.operators.push(mass_like_operator());
    assert_both_refuse(
        &duplicate,
        SymmetryOperatorExclusionRefusal::DuplicateOperatorIdentity,
    );

    let mut zero = gauge_like_request();
    zero.operators[0].terms.push(operator_term(10, 10, -1));
    zero.operators[0]
        .terms
        .retain(|term| term.left_field == identity(10) && term.right_field == identity(10));
    assert_both_refuse(&zero, SymmetryOperatorExclusionRefusal::ZeroOperator);

    let mut oversized = gauge_like_request();
    oversized.operators[0].terms = vec![operator_term(10, 10, 1); MAX_OPERATOR_TERMS + 1];
    assert_both_refuse(
        &oversized,
        SymmetryOperatorExclusionRefusal::OperatorTermCapacityExceeded,
    );
}

#[test]
fn checked_exact_arithmetic_refuses_overflow() {
    let input = request(
        vec![10],
        vec![action_term(10, 20, i64::MAX)],
        vec![QuadraticOperatorCandidate {
            operator_identity: identity(90),
            terms: vec![operator_term(10, 10, i64::MAX)],
        }],
    );
    assert_both_refuse(&input, SymmetryOperatorExclusionRefusal::ArithmeticOverflow);
}

#[test]
fn canonical_expansion_budget_refuses_before_either_algorithm_expands() {
    let field = identity(10);
    let action_terms = (1_u64..=257)
        .map(|number| AffineActionTerm {
            field_component: field,
            shift_generator: numbered_identity(number),
            coefficient: 1,
        })
        .collect();
    let input = SymmetryOperatorExclusionInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        field_components: vec![field],
        action: AffineSymmetryAction {
            symmetry_identity: identity(4),
            terms: action_terms,
        },
        operators: vec![QuadraticOperatorCandidate {
            operator_identity: identity(90),
            terms: vec![QuadraticOperatorTerm {
                left_field: field,
                right_field: field,
                coefficient: 1,
            }],
        }],
    };
    assert_both_refuse(
        &input,
        SymmetryOperatorExclusionRefusal::VariationCapacityExceeded,
    );
}

#[test]
fn aggregate_expansion_budget_refuses_a_large_complete_request_preflight() {
    let field = identity(10);
    let action_terms = (1_u64..=128)
        .map(|number| AffineActionTerm {
            field_component: field,
            shift_generator: numbered_identity(number),
            coefficient: 1,
        })
        .collect();
    let operators = (1_u64..=16)
        .map(|number| QuadraticOperatorCandidate {
            operator_identity: numbered_identity(10_000 + number),
            terms: vec![QuadraticOperatorTerm {
                left_field: field,
                right_field: field,
                coefficient: 1,
            }],
        })
        .collect();
    let input = SymmetryOperatorExclusionInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        field_components: vec![field],
        action: AffineSymmetryAction {
            symmetry_identity: identity(4),
            terms: action_terms,
        },
        operators,
    };
    assert_both_refuse(
        &input,
        SymmetryOperatorExclusionRefusal::TotalVariationCapacityExceeded,
    );
}

#[test]
fn independent_algorithms_agree_over_small_integer_action_grid() {
    for action_coefficient in -3_i64..=3 {
        if action_coefficient == 0 {
            continue;
        }
        for operator_coefficient in -3_i64..=3 {
            if operator_coefficient == 0 {
                continue;
            }
            let input = request(
                vec![10, 11],
                vec![
                    action_term(10, 20, action_coefficient),
                    action_term(11, 20, -action_coefficient),
                ],
                vec![QuadraticOperatorCandidate {
                    operator_identity: identity(90),
                    terms: vec![
                        operator_term(10, 10, operator_coefficient),
                        operator_term(10, 11, -operator_coefficient),
                        operator_term(11, 11, operator_coefficient),
                    ],
                }],
            );
            assert_eq!(producer::evaluate(&input), watchdog::evaluate(&input));
        }
    }
}

#[test]
fn production_has_no_admitted_action_or_authority_effect() {
    let refusal = repository_symmetry_operator_exclusion().unwrap_err();
    assert_eq!(
        refusal,
        SymmetryOperatorExclusionRefusal::NoAdmittedSymmetryActionInput
    );
    assert_eq!(refusal.id(), "no_admitted_symmetry_action_input");
    assert_eq!(
        repository_complete_quadratic_symmetry().unwrap_err(),
        CompleteQuadraticSymmetryRefusal::NoAdmittedSymmetryActionInput
    );
    assert_eq!(
        repository_conditional_quadratic_symmetry().unwrap_err(),
        ConditionalQuadraticSymmetryRefusal::NoAdmittedScopePremiseInput
    );
}

#[test]
fn complete_quadratic_runner_evaluates_every_derived_basis_element() {
    let input = ScopedSymmetryActionInput {
        claim_identity: identity(1),
        subject_identity: identity(2),
        applicability_domain_identity: identity(3),
        field_components: vec![identity(10), identity(11)],
        action: AffineSymmetryAction {
            symmetry_identity: identity(4),
            terms: vec![action_term(10, 20, 1)],
        },
    };
    let report = inspect_complete_quadratic_symmetry(&input).unwrap();
    assert_eq!(report.basis_element_count(), 3);
    assert_eq!(report.excluded_operator_count(), 2);
    assert_eq!(report.invariant_operator_count(), 1);
    assert!(report.scoped_homogeneous_quadratic_basis_coverage());
    assert!(!report.global_operator_basis_coverage());
    assert!(!report.field_ontology_authority());
    assert!(!report.action_admission_authority());
    assert!(!report.applicability_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn conditional_runner_binds_scope_proofs_to_the_exact_evaluated_action() {
    let input = ConditionalQuadraticSymmetryInput {
        symmetry: ScopedSymmetryActionInput {
            claim_identity: identity(1),
            subject_identity: identity(2),
            applicability_domain_identity: identity(3),
            field_components: vec![identity(10), identity(11)],
            action: AffineSymmetryAction {
                symmetry_identity: identity(4),
                terms: vec![action_term(10, 20, 1)],
            },
        },
        premise_facts: vec![identity(70)],
        inference_rules: vec![
            applicability_proof::ScopeInferenceRule {
                premises: vec![
                    applicability_proof::ScopeRulePremise::EvaluatedAction,
                    applicability_proof::ScopeRulePremise::Fact(identity(70)),
                ],
                conclusion: identity(71),
            },
            applicability_proof::ScopeInferenceRule {
                premises: vec![applicability_proof::ScopeRulePremise::Fact(identity(71))],
                conclusion: identity(72),
            },
            applicability_proof::ScopeInferenceRule {
                premises: vec![applicability_proof::ScopeRulePremise::Fact(identity(71))],
                conclusion: identity(73),
            },
        ],
        required_applicability_fact: identity(72),
        required_validity_fact: identity(73),
    };
    let report = inspect_conditional_quadratic_symmetry(&input).unwrap();
    let mut altered_action = input.clone();
    altered_action.symmetry.action.terms[0].coefficient = 2;
    let altered_report = inspect_conditional_quadratic_symmetry(&altered_action).unwrap();
    assert_eq!(report.basis_element_count(), 3);
    assert_eq!(report.excluded_operator_count(), 2);
    assert_eq!(report.invariant_operator_count(), 1);
    assert_eq!(report.scope.reachable_fact_count(), 4);
    assert_eq!(
        report.scope.action_binding_fact(),
        report.symmetry.action_binding_identity()
    );
    assert_ne!(
        report.scope.action_binding_fact(),
        altered_report.scope.action_binding_fact()
    );
    assert!(report.conditional_applicability_proved());
    assert!(report.conditional_validity_proved());
    assert!(report.exact_action_binding_proved());
    assert!(!report.premise_admission_authority());
    assert!(!report.field_ontology_authority());
    assert!(!report.action_admission_authority());
    assert!(!report.physical_operator_family_authority());
    assert!(!report.species_membership_authority());
    assert_eq!(report.authority_effect(), "none");
}
