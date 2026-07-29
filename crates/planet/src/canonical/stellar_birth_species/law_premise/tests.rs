use super::*;

fn id(tag: u8) -> [u8; 32] {
    let mut identity = [tag; 32];
    identity[31] = tag.wrapping_add(1);
    identity
}

fn key(role: u8, content: u8) -> PremiseKey {
    PremiseKey {
        role: SemanticRoleIdentity(id(role)),
        content: PhysicalContentIdentity(id(content)),
    }
}

fn admitted_requirement(role: u8, content: u8) -> LawPremiseRequirement {
    LawPremiseRequirement {
        key: key(role, content),
        proof: RequirementProof::AdmittedContent,
    }
}

fn exact_zero_requirement(
    role: u8,
    content: u8,
    subject: u8,
    excluded_term: u8,
    symmetry: u8,
) -> LawPremiseRequirement {
    LawPremiseRequirement {
        key: key(role, content),
        proof: RequirementProof::ExactZero {
            subject_identity: id(subject),
            excluded_term_identity: id(excluded_term),
            symmetry_identity: id(symmetry),
        },
    }
}

fn seal_candidate(mut candidate: ClaimScopedPremiseCapability) -> ClaimScopedPremiseCapability {
    let produced = producer::capability_digest(&candidate);
    let watched = watchdog::capability_digest(&candidate);
    assert_eq!(produced, watched);
    candidate.capability_sha256 = produced;
    candidate
}

fn candidate(claim: u8, role: u8, content: u8, receipt_base: u8) -> ClaimScopedPremiseCapability {
    seal_candidate(ClaimScopedPremiseCapability {
        claim_identity: LawClaimIdentity(id(claim)),
        key: key(role, content),
        upstream_capability_sha256: id(receipt_base),
        semantic_producer_receipt_sha256: id(receipt_base.wrapping_add(1)),
        semantic_watchdog_receipt_sha256: id(receipt_base.wrapping_add(2)),
        applicability_receipt_sha256: id(receipt_base.wrapping_add(3)),
        validity_receipt_sha256: id(receipt_base.wrapping_add(4)),
        proof: CandidateProof::AdmittedContent,
        capability_sha256: [0; 32],
        _seal: PremiseCapabilitySeal,
    })
}

fn exact_zero_candidate(
    claim: u8,
    role: u8,
    content: u8,
    receipt_base: u8,
    subject: u8,
    excluded_term: u8,
    symmetry: u8,
) -> ClaimScopedPremiseCapability {
    seal_candidate(ClaimScopedPremiseCapability {
        claim_identity: LawClaimIdentity(id(claim)),
        key: key(role, content),
        upstream_capability_sha256: id(receipt_base),
        semantic_producer_receipt_sha256: id(receipt_base.wrapping_add(1)),
        semantic_watchdog_receipt_sha256: id(receipt_base.wrapping_add(2)),
        applicability_receipt_sha256: id(receipt_base.wrapping_add(3)),
        validity_receipt_sha256: id(receipt_base.wrapping_add(4)),
        proof: CandidateProof::ExactZero(ExactZeroProof {
            subject_identity: id(subject),
            excluded_term_identity: id(excluded_term),
            symmetry_identity: id(symmetry),
            exclusion_receipt_sha256: id(receipt_base.wrapping_add(6)),
        }),
        capability_sha256: [0; 32],
        _seal: PremiseCapabilitySeal,
    })
}

fn request(
    claim: u8,
    requirements: Vec<LawPremiseRequirement>,
    candidates: Vec<ClaimScopedPremiseCapability>,
) -> LawPremiseRequest {
    LawPremiseRequest {
        claim_identity: LawClaimIdentity(id(claim)),
        requirements,
        candidates,
    }
}

#[test]
fn exact_role_and_content_selection_is_order_invariant_and_non_authorizing() {
    let requirements = vec![admitted_requirement(31, 81), admitted_requirement(17, 63)];
    let candidates = vec![candidate(7, 31, 81, 120), candidate(7, 17, 63, 100)];
    let expected =
        inspect_law_premises(&request(7, requirements.clone(), candidates.clone())).unwrap();

    let report = inspect_law_premises(&request(
        7,
        requirements.into_iter().rev().collect(),
        candidates.into_iter().rev().collect(),
    ))
    .unwrap();
    assert_eq!(report, expected);
    assert!(report.requested_requirement_coverage());
    assert!(!report.global_physical_premise_coverage());
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn sole_candidate_with_wrong_role_or_content_never_falls_back() {
    let requirement = admitted_requirement(31, 81);
    for wrong in [candidate(7, 32, 81, 100), candidate(7, 31, 82, 120)] {
        assert_eq!(
            inspect_law_premises(&request(7, vec![requirement], vec![wrong])),
            Err(LawPremiseRefusal::MissingRequiredPremise)
        );
    }
}

#[test]
fn unrelated_unfamiliar_candidates_are_extension_monotone() {
    let requirement = admitted_requirement(31, 81);
    let selected = candidate(7, 31, 81, 100);
    let baseline = inspect_law_premises(&request(7, vec![requirement], vec![selected])).unwrap();
    let extended = inspect_law_premises(&request(
        7,
        vec![requirement],
        vec![
            candidate(7, 210, 211, 180),
            selected,
            candidate(7, 212, 213, 200),
        ],
    ))
    .unwrap();
    assert_eq!(baseline, extended);
}

#[test]
fn exact_zero_requires_a_matching_exclusion_proof_object() {
    let requirement = exact_zero_requirement(41, 91, 131, 132, 134);
    assert_eq!(
        inspect_law_premises(&request(
            7,
            vec![requirement],
            vec![candidate(7, 41, 91, 100)]
        )),
        Err(LawPremiseRefusal::ProofKindMismatch)
    );
    assert_eq!(
        inspect_law_premises(&request(
            7,
            vec![requirement],
            vec![exact_zero_candidate(7, 41, 91, 100, 131, 133, 134)]
        )),
        Err(LawPremiseRefusal::ProofKindMismatch)
    );
    assert_eq!(
        inspect_law_premises(&request(
            7,
            vec![requirement],
            vec![exact_zero_candidate(7, 41, 91, 100, 131, 132, 135)]
        )),
        Err(LawPremiseRefusal::ProofKindMismatch)
    );

    let report = inspect_law_premises(&request(
        7,
        vec![requirement],
        vec![exact_zero_candidate(7, 41, 91, 100, 131, 132, 134)],
    ))
    .unwrap();
    assert!(matches!(
        report.selection.selected[0].proof,
        SelectedProof::ExactZero(_)
    ));
}

#[test]
fn unfamiliar_material_precursor_uses_the_same_identity_path() {
    let requirements = vec![
        admitted_requirement(20, 120),
        admitted_requirement(21, 121),
        admitted_requirement(22, 122),
        admitted_requirement(23, 123),
        admitted_requirement(24, 124),
        admitted_requirement(25, 125),
        exact_zero_requirement(26, 126, 127, 128, 129),
    ];
    let candidates = vec![
        candidate(9, 25, 125, 190),
        exact_zero_candidate(9, 26, 126, 210, 127, 128, 129),
        candidate(9, 23, 123, 170),
        candidate(9, 20, 120, 110),
        candidate(9, 24, 124, 180),
        candidate(9, 22, 122, 150),
        candidate(9, 21, 121, 130),
    ];
    let report = inspect_law_premises(&request(9, requirements, candidates)).unwrap();

    assert_eq!(report.selection.selected.len(), 7);
    assert_eq!(report.selection.claim_identity, LawClaimIdentity(id(9)));
    assert_eq!(report.authority_effect(), "none");
}

#[test]
fn missing_duplicate_cross_claim_and_capability_mutations_refuse() {
    let requirement = admitted_requirement(31, 81);
    let selected = candidate(7, 31, 81, 100);
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement], vec![])),
        Err(LawPremiseRefusal::MissingRequiredPremise)
    );
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement, requirement], vec![selected])),
        Err(LawPremiseRefusal::DuplicateRequirement)
    );
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement], vec![selected, selected])),
        Err(LawPremiseRefusal::AmbiguousRequiredPremise)
    );

    let foreign = candidate(8, 31, 81, 120);
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement], vec![foreign])),
        Err(LawPremiseRefusal::CandidateOutsideClaim)
    );

    let mut collapsed = selected;
    collapsed.semantic_watchdog_receipt_sha256 = collapsed.semantic_producer_receipt_sha256;
    collapsed = seal_candidate(collapsed);
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement], vec![collapsed])),
        Err(LawPremiseRefusal::InvalidCapabilityEvidence)
    );

    let mut stale = selected;
    stale.validity_receipt_sha256 = id(211);
    assert_eq!(
        inspect_law_premises(&request(7, vec![requirement], vec![stale])),
        Err(LawPremiseRefusal::CapabilityDigestMismatch)
    );
}

#[test]
fn malformed_and_oversized_requests_refuse_under_both_paths() {
    assert_eq!(
        inspect_law_premises(&LawPremiseRequest {
            claim_identity: LawClaimIdentity([0; 32]),
            requirements: vec![admitted_requirement(1, 2)],
            candidates: vec![],
        }),
        Err(LawPremiseRefusal::InvalidClaimIdentity)
    );
    assert_eq!(
        inspect_law_premises(&request(7, vec![], vec![])),
        Err(LawPremiseRefusal::EmptyRequirementSet)
    );

    let oversized_requirements = vec![admitted_requirement(1, 2); MAX_REQUIREMENT_COUNT + 1];
    assert_eq!(
        inspect_law_premises(&request(7, oversized_requirements, vec![])),
        Err(LawPremiseRefusal::RequirementCapacityExceeded)
    );

    let oversized_candidates = vec![candidate(7, 1, 2, 100); MAX_CANDIDATE_COUNT + 1];
    assert_eq!(
        inspect_law_premises(&request(
            7,
            vec![admitted_requirement(1, 2)],
            oversized_candidates
        )),
        Err(LawPremiseRefusal::CandidateCapacityExceeded)
    );
}

#[test]
fn independent_algorithms_agree_over_small_identity_grid() {
    for requested_role in 1..=3 {
        for requested_content in 11..=13 {
            let requirement = admitted_requirement(requested_role, requested_content);
            for candidate_role in 1..=3 {
                for candidate_content in 11..=13 {
                    let request = request(
                        7,
                        vec![requirement],
                        vec![candidate(7, candidate_role, candidate_content, 100)],
                    );
                    assert_eq!(producer::select(&request), watchdog::select(&request));
                }
            }
        }
    }
}
