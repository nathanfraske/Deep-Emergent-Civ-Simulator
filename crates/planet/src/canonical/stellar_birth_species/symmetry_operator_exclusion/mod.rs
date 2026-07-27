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

mod applicability_proof;
mod producer;
mod quadratic_basis;
mod watchdog;

#[cfg(test)]
mod tests;

use civsim_units::digest::sha256;

const RESULT_SCHEMA_ID: &str = "civsim.planet.symmetry-operator-exclusion-evaluation.v1";
const PRODUCER_ID: &str = "civsim.planet.symmetry-operator-exclusion.substitution-producer.v1";
const WATCHDOG_ID: &str = "civsim.planet.symmetry-operator-exclusion.direct-variation-watchdog.v1";
const SCOPE_ACTION_BINDING_DOMAIN: &[u8] =
    b"civsim.planet.symmetry-operator-exclusion.scope-action-binding.v1";

const MAX_FIELD_COMPONENTS: usize = 64;
const MAX_ACTION_TERMS: usize = 4_096;
const MAX_OPERATORS: usize = 1_024;
const MAX_OPERATOR_TERMS: usize = 4_096;
const MAX_TOTAL_OPERATOR_TERMS: usize = 16_384;
const MAX_VARIATION_TERMS: usize = 65_536;
const MAX_TOTAL_VARIATION_TERMS: usize = MAX_VARIATION_TERMS * 4;

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

/// One conditional action without a caller-authored operator catalog.
///
/// The complete quadratic runner derives its operator basis from the field
/// components. This shape still carries no field-ontology, action-admission,
/// applicability, or validity capability.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScopedSymmetryActionInput {
    claim_identity: AlgebraIdentity,
    subject_identity: AlgebraIdentity,
    applicability_domain_identity: AlgebraIdentity,
    field_components: Vec<AlgebraIdentity>,
    action: AffineSymmetryAction,
}

/// Conditional scope evidence for one exact complete quadratic evaluation.
///
/// Facts and rules remain caller-supplied diagnostics. The combined runner
/// derives the action-binding fact from the independently agreed algebra
/// result, so a proof for another action cannot be substituted.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConditionalQuadraticSymmetryInput {
    symmetry: ScopedSymmetryActionInput,
    premise_facts: Vec<AlgebraIdentity>,
    inference_rules: Vec<applicability_proof::ScopeInferenceRule>,
    required_applicability_fact: AlgebraIdentity,
    required_validity_fact: AlgebraIdentity,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompleteQuadraticSymmetryReport {
    basis: quadratic_basis::QuadraticOperatorBasisReport,
    exclusion: SymmetryOperatorExclusionReport,
}

impl CompleteQuadraticSymmetryReport {
    fn basis_element_count(&self) -> usize {
        self.basis.element_count()
    }

    fn excluded_operator_count(&self) -> usize {
        self.exclusion.excluded_operator_count()
    }

    fn invariant_operator_count(&self) -> usize {
        self.exclusion.invariant_operator_count()
    }

    const fn scoped_homogeneous_quadratic_basis_coverage(&self) -> bool {
        true
    }

    const fn global_operator_basis_coverage(&self) -> bool {
        false
    }

    const fn field_ontology_authority(&self) -> bool {
        false
    }

    const fn action_admission_authority(&self) -> bool {
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

    fn action_binding_identity(&self) -> AlgebraIdentity {
        let mut bytes = Vec::with_capacity(SCOPE_ACTION_BINDING_DOMAIN.len() + 512);
        bytes.extend_from_slice(SCOPE_ACTION_BINDING_DOMAIN);
        append_scope_binding_field(&mut bytes, 1, self.basis.schema_id().as_bytes());
        append_scope_binding_field(&mut bytes, 2, self.basis.producer_id().as_bytes());
        append_scope_binding_field(&mut bytes, 3, self.basis.watchdog_id().as_bytes());
        append_scope_binding_field(&mut bytes, 4, &self.basis.producer_result_sha256());
        append_scope_binding_field(&mut bytes, 5, &self.basis.watchdog_result_sha256());
        append_scope_binding_field(&mut bytes, 6, RESULT_SCHEMA_ID.as_bytes());
        append_scope_binding_field(&mut bytes, 7, self.exclusion.producer_id.as_bytes());
        append_scope_binding_field(&mut bytes, 8, self.exclusion.watchdog_id.as_bytes());
        append_scope_binding_field(&mut bytes, 9, &self.exclusion.producer_result_sha256);
        append_scope_binding_field(&mut bytes, 10, &self.exclusion.watchdog_result_sha256);
        AlgebraIdentity(sha256(&bytes))
    }
}

fn append_scope_binding_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConditionalQuadraticSymmetryReport {
    scope: applicability_proof::ScopeProofReport,
    symmetry: CompleteQuadraticSymmetryReport,
}

impl ConditionalQuadraticSymmetryReport {
    fn basis_element_count(&self) -> usize {
        self.symmetry.basis_element_count()
    }

    fn excluded_operator_count(&self) -> usize {
        self.symmetry.excluded_operator_count()
    }

    fn invariant_operator_count(&self) -> usize {
        self.symmetry.invariant_operator_count()
    }

    const fn conditional_applicability_proved(&self) -> bool {
        true
    }

    const fn conditional_validity_proved(&self) -> bool {
        true
    }

    const fn exact_action_binding_proved(&self) -> bool {
        true
    }

    const fn premise_admission_authority(&self) -> bool {
        false
    }

    const fn field_ontology_authority(&self) -> bool {
        false
    }

    const fn action_admission_authority(&self) -> bool {
        false
    }

    const fn physical_operator_family_authority(&self) -> bool {
        false
    }

    const fn species_membership_authority(&self) -> bool {
        false
    }

    const fn authority_effect(&self) -> &'static str {
        "none"
    }
}

/// Data-defined request used by an enrolled theory-profile authority.
///
/// Every identity is opaque at this boundary. The caller supplies the complete
/// field-component basis, affine action, and one exact quadratic term to test.
/// No familiar particle, gauge group, component count, or coefficient pattern
/// is selected inside the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileSymmetryRequest {
    pub(in crate::canonical::stellar_birth_species) claim_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) subject_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) applicability_domain_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) symmetry_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) field_components: Vec<[u8; 32]>,
    pub(in crate::canonical::stellar_birth_species) shifts: Vec<TheoryProfileShift>,
    pub(in crate::canonical::stellar_birth_species) excluded_operator_identity: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) excluded_operator_terms:
        Vec<TheoryProfileQuadraticTerm>,
    pub(in crate::canonical::stellar_birth_species) admitted_scope_fact: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) required_applicability_fact: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) required_validity_fact: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileShift {
    pub(in crate::canonical::stellar_birth_species) field_component: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) shift_generator: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) coefficient: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileQuadraticTerm {
    pub(in crate::canonical::stellar_birth_species) left_field: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) right_field: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) coefficient: i64,
}

/// Executed evidence for one claim-local exact-zero admission attempt.
///
/// The report proves the supplied algebra, the complete finite homogeneous
/// commutative quadratic basis, exclusion of the exact requested operator, and
/// action-dependent scope closure. It does not by itself admit the supplied
/// ontology or create species membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct TheoryProfileSymmetryEvidence {
    pub(in crate::canonical::stellar_birth_species) basis_element_count: u32,
    pub(in crate::canonical::stellar_birth_species) basis_excluded_count: u32,
    pub(in crate::canonical::stellar_birth_species) basis_invariant_count: u32,
    pub(in crate::canonical::stellar_birth_species) excluded_operator_count: u32,
    pub(in crate::canonical::stellar_birth_species) invariant_operator_count: u32,
    pub(in crate::canonical::stellar_birth_species) scope_reachable_fact_count: u32,
    pub(in crate::canonical::stellar_birth_species) basis_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) basis_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) exclusion_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) exclusion_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) scope_producer_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) scope_watchdog_result_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) action_binding_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) applicability_receipt_sha256: [u8; 32],
    pub(in crate::canonical::stellar_birth_species) validity_receipt_sha256: [u8; 32],
}

const THEORY_PROFILE_ACTION_BINDING_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-symmetry-action-binding.v1";
const THEORY_PROFILE_APPLICABILITY_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-symmetry-applicability.v1";
const THEORY_PROFILE_VALIDITY_DOMAIN: &[u8] = b"civsim.planet.theory-profile-symmetry-validity.v1";

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
    TotalVariationCapacityExceeded,
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
            Self::TotalVariationCapacityExceeded => "total_variation_capacity_exceeded",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompleteQuadraticSymmetryRefusal {
    NoAdmittedSymmetryActionInput,
    Basis(quadratic_basis::QuadraticOperatorBasisRefusal),
    Exclusion(SymmetryOperatorExclusionRefusal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConditionalQuadraticSymmetryRefusal {
    NoAdmittedScopePremiseInput,
    Symmetry(CompleteQuadraticSymmetryRefusal),
    Scope(applicability_proof::ScopeProofRefusal),
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

fn inspect_complete_quadratic_symmetry(
    input: &ScopedSymmetryActionInput,
) -> Result<CompleteQuadraticSymmetryReport, CompleteQuadraticSymmetryRefusal> {
    let basis = quadratic_basis::inspect_quadratic_operator_basis(
        &quadratic_basis::QuadraticOperatorBasisInput {
            claim_identity: input.claim_identity,
            subject_identity: input.subject_identity,
            applicability_domain_identity: input.applicability_domain_identity,
            field_components: input.field_components.clone(),
        },
    )
    .map_err(CompleteQuadraticSymmetryRefusal::Basis)?;
    let exclusion = inspect_symmetry_operator_exclusion(&SymmetryOperatorExclusionInput {
        claim_identity: input.claim_identity,
        subject_identity: input.subject_identity,
        applicability_domain_identity: input.applicability_domain_identity,
        field_components: input.field_components.clone(),
        action: input.action.clone(),
        operators: basis.operator_candidates(),
    })
    .map_err(CompleteQuadraticSymmetryRefusal::Exclusion)?;
    Ok(CompleteQuadraticSymmetryReport { basis, exclusion })
}

fn inspect_conditional_quadratic_symmetry(
    input: &ConditionalQuadraticSymmetryInput,
) -> Result<ConditionalQuadraticSymmetryReport, ConditionalQuadraticSymmetryRefusal> {
    let symmetry = inspect_complete_quadratic_symmetry(&input.symmetry)
        .map_err(ConditionalQuadraticSymmetryRefusal::Symmetry)?;
    let scope = applicability_proof::inspect_scope_proof(&applicability_proof::ScopeProofInput {
        claim_identity: input.symmetry.claim_identity,
        subject_identity: input.symmetry.subject_identity,
        applicability_domain_identity: input.symmetry.applicability_domain_identity,
        action_binding_fact: symmetry.action_binding_identity(),
        premise_facts: input.premise_facts.clone(),
        inference_rules: input.inference_rules.clone(),
        required_applicability_fact: input.required_applicability_fact,
        required_validity_fact: input.required_validity_fact,
    })
    .map_err(ConditionalQuadraticSymmetryRefusal::Scope)?;
    Ok(ConditionalQuadraticSymmetryReport { scope, symmetry })
}

pub(in crate::canonical::stellar_birth_species) fn inspect_theory_profile_symmetry(
    request: &TheoryProfileSymmetryRequest,
) -> Result<TheoryProfileSymmetryEvidence, &'static str> {
    let claim_identity = AlgebraIdentity(request.claim_identity);
    let subject_identity = AlgebraIdentity(request.subject_identity);
    let applicability_domain_identity = AlgebraIdentity(request.applicability_domain_identity);
    let field_components = request
        .field_components
        .iter()
        .copied()
        .map(AlgebraIdentity)
        .collect::<Vec<_>>();
    let action = AffineSymmetryAction {
        symmetry_identity: AlgebraIdentity(request.symmetry_identity),
        terms: request
            .shifts
            .iter()
            .map(|term| AffineActionTerm {
                field_component: AlgebraIdentity(term.field_component),
                shift_generator: AlgebraIdentity(term.shift_generator),
                coefficient: term.coefficient,
            })
            .collect(),
    };
    let scoped = ScopedSymmetryActionInput {
        claim_identity,
        subject_identity,
        applicability_domain_identity,
        field_components: field_components.clone(),
        action: action.clone(),
    };
    let complete = inspect_complete_quadratic_symmetry(&scoped)
        .map_err(|_| "complete_quadratic_symmetry_refused")?;
    let excluded = inspect_symmetry_operator_exclusion(&SymmetryOperatorExclusionInput {
        claim_identity,
        subject_identity,
        applicability_domain_identity,
        field_components,
        action,
        operators: vec![QuadraticOperatorCandidate {
            operator_identity: AlgebraIdentity(request.excluded_operator_identity),
            terms: request
                .excluded_operator_terms
                .iter()
                .map(|term| QuadraticOperatorTerm {
                    left_field: AlgebraIdentity(term.left_field),
                    right_field: AlgebraIdentity(term.right_field),
                    coefficient: term.coefficient,
                })
                .collect(),
        }],
    })
    .map_err(|_| "excluded_operator_evaluation_refused")?;

    let basis_element_count = u32::try_from(complete.basis_element_count())
        .map_err(|_| "basis_element_count_overflow")?;
    let basis_excluded_count = u32::try_from(complete.excluded_operator_count())
        .map_err(|_| "basis_excluded_count_overflow")?;
    let basis_invariant_count = u32::try_from(complete.invariant_operator_count())
        .map_err(|_| "basis_invariant_count_overflow")?;
    let excluded_operator_count = u32::try_from(excluded.excluded_operator_count())
        .map_err(|_| "excluded_operator_count_overflow")?;
    let invariant_operator_count = u32::try_from(excluded.invariant_operator_count())
        .map_err(|_| "invariant_operator_count_overflow")?;
    if basis_element_count == 0
        || basis_excluded_count != basis_element_count
        || basis_invariant_count != 0
        || excluded_operator_count != 1
        || invariant_operator_count != 0
    {
        return Err("requested_mass_operator_not_excluded");
    }

    let mut action_binding_bytes = THEORY_PROFILE_ACTION_BINDING_DOMAIN.to_vec();
    for (tag, payload) in [
        (1, complete.basis.producer_result_sha256().as_slice()),
        (2, complete.basis.watchdog_result_sha256().as_slice()),
        (3, complete.exclusion.producer_result_sha256.as_slice()),
        (4, complete.exclusion.watchdog_result_sha256.as_slice()),
        (5, excluded.producer_result_sha256.as_slice()),
        (6, excluded.watchdog_result_sha256.as_slice()),
        (7, request.excluded_operator_identity.as_slice()),
    ] {
        append_scope_binding_field(&mut action_binding_bytes, tag, payload);
    }
    let action_binding = AlgebraIdentity(sha256(&action_binding_bytes));
    let admitted_scope_fact = AlgebraIdentity(request.admitted_scope_fact);
    let required_applicability_fact = AlgebraIdentity(request.required_applicability_fact);
    let required_validity_fact = AlgebraIdentity(request.required_validity_fact);
    let scope = applicability_proof::inspect_scope_proof(&applicability_proof::ScopeProofInput {
        claim_identity,
        subject_identity,
        applicability_domain_identity,
        action_binding_fact: action_binding,
        premise_facts: vec![admitted_scope_fact],
        inference_rules: vec![
            applicability_proof::ScopeInferenceRule {
                premises: vec![
                    applicability_proof::ScopeRulePremise::EvaluatedAction,
                    applicability_proof::ScopeRulePremise::Fact(admitted_scope_fact),
                ],
                conclusion: required_applicability_fact,
            },
            applicability_proof::ScopeInferenceRule {
                premises: vec![applicability_proof::ScopeRulePremise::Fact(
                    required_applicability_fact,
                )],
                conclusion: required_validity_fact,
            },
        ],
        required_applicability_fact,
        required_validity_fact,
    })
    .map_err(|_| "action_bound_scope_refused")?;
    if !scope.conditional_applicability_proved()
        || !scope.conditional_validity_proved()
        || !scope.action_dependency_proved()
    {
        return Err("action_bound_scope_unproved");
    }
    let scope_reachable_fact_count = u32::try_from(scope.reachable_fact_count())
        .map_err(|_| "scope_reachable_fact_count_overflow")?;
    let mut applicability_bytes = THEORY_PROFILE_APPLICABILITY_DOMAIN.to_vec();
    append_scope_binding_field(&mut applicability_bytes, 1, &action_binding.0);
    append_scope_binding_field(&mut applicability_bytes, 2, &scope.producer_result_sha256());
    append_scope_binding_field(&mut applicability_bytes, 3, &scope.watchdog_result_sha256());
    append_scope_binding_field(
        &mut applicability_bytes,
        4,
        &request.required_applicability_fact,
    );
    let applicability_receipt_sha256 = sha256(&applicability_bytes);
    let mut validity_bytes = THEORY_PROFILE_VALIDITY_DOMAIN.to_vec();
    append_scope_binding_field(&mut validity_bytes, 1, &applicability_receipt_sha256);
    append_scope_binding_field(&mut validity_bytes, 2, &scope.producer_result_sha256());
    append_scope_binding_field(&mut validity_bytes, 3, &scope.watchdog_result_sha256());
    append_scope_binding_field(&mut validity_bytes, 4, &request.required_validity_fact);
    let validity_receipt_sha256 = sha256(&validity_bytes);

    Ok(TheoryProfileSymmetryEvidence {
        basis_element_count,
        basis_excluded_count,
        basis_invariant_count,
        excluded_operator_count,
        invariant_operator_count,
        scope_reachable_fact_count,
        basis_producer_result_sha256: complete.basis.producer_result_sha256(),
        basis_watchdog_result_sha256: complete.basis.watchdog_result_sha256(),
        exclusion_producer_result_sha256: excluded.producer_result_sha256,
        exclusion_watchdog_result_sha256: excluded.watchdog_result_sha256,
        scope_producer_result_sha256: scope.producer_result_sha256(),
        scope_watchdog_result_sha256: scope.watchdog_result_sha256(),
        action_binding_sha256: action_binding.0,
        applicability_receipt_sha256,
        validity_receipt_sha256,
    })
}

fn repository_symmetry_operator_exclusion(
) -> Result<SymmetryOperatorExclusionReport, SymmetryOperatorExclusionRefusal> {
    Err(SymmetryOperatorExclusionRefusal::NoAdmittedSymmetryActionInput)
}

fn repository_complete_quadratic_symmetry(
) -> Result<CompleteQuadraticSymmetryReport, CompleteQuadraticSymmetryRefusal> {
    Err(CompleteQuadraticSymmetryRefusal::NoAdmittedSymmetryActionInput)
}

fn repository_conditional_quadratic_symmetry(
) -> Result<ConditionalQuadraticSymmetryReport, ConditionalQuadraticSymmetryRefusal> {
    Err(ConditionalQuadraticSymmetryRefusal::NoAdmittedScopePremiseInput)
}
