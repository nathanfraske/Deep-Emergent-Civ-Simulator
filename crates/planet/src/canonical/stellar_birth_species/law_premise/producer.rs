//! Sorted-vector physical-law premise selector.

use super::{
    CandidateProof, ClaimScopedPremiseCapability, LawPremiseRefusal, LawPremiseRequest,
    LawPremiseSelection, PremiseKey, RequirementProof, SelectedPremise, SelectedProof,
    MAX_CANDIDATE_COUNT, MAX_REQUIREMENT_COUNT,
};
use civsim_units::digest::sha256;

const CAPABILITY_DOMAIN: &[u8] = b"civsim.law-premise-capability.v1";

pub(super) fn select(
    request: &LawPremiseRequest,
) -> Result<LawPremiseSelection, LawPremiseRefusal> {
    if request.claim_identity.0 == [0; 32] {
        return Err(LawPremiseRefusal::InvalidClaimIdentity);
    }
    if request.requirements.is_empty() {
        return Err(LawPremiseRefusal::EmptyRequirementSet);
    }
    if request.requirements.len() > MAX_REQUIREMENT_COUNT {
        return Err(LawPremiseRefusal::RequirementCapacityExceeded);
    }
    if request.candidates.len() > MAX_CANDIDATE_COUNT {
        return Err(LawPremiseRefusal::CandidateCapacityExceeded);
    }

    if request.requirements.iter().any(|requirement| {
        invalid_key(requirement.key)
            || match requirement.proof {
                RequirementProof::AdmittedContent | RequirementProof::VerifiedDerivedContent => {
                    false
                }
                RequirementProof::ExactZero {
                    subject_identity,
                    excluded_term_identity,
                    symmetry_identity,
                } => {
                    subject_identity == [0; 32]
                        || excluded_term_identity == [0; 32]
                        || symmetry_identity == [0; 32]
                        || subject_identity == excluded_term_identity
                }
            }
    }) {
        return Err(LawPremiseRefusal::InvalidRequirement);
    }

    let mut requirements = request.requirements.clone();
    requirements.sort_unstable_by_key(|requirement| requirement.key);
    if requirements
        .windows(2)
        .any(|pair| pair[0].key == pair[1].key)
    {
        return Err(LawPremiseRefusal::DuplicateRequirement);
    }

    if request.candidates.iter().any(invalid_candidate) {
        return Err(LawPremiseRefusal::InvalidCandidate);
    }
    if request
        .candidates
        .iter()
        .any(|candidate| candidate.claim_identity != request.claim_identity)
    {
        return Err(LawPremiseRefusal::CandidateOutsideClaim);
    }
    if request
        .candidates
        .iter()
        .any(|candidate| !evidence_is_valid(candidate))
    {
        return Err(LawPremiseRefusal::InvalidCapabilityEvidence);
    }
    if request
        .candidates
        .iter()
        .any(|candidate| capability_digest(candidate) != candidate.capability_sha256)
    {
        return Err(LawPremiseRefusal::CapabilityDigestMismatch);
    }

    let mut candidates = request.candidates.iter().collect::<Vec<_>>();
    candidates.sort_unstable_by_key(|candidate| (candidate.key, candidate.capability_sha256));

    let mut selected = Vec::with_capacity(requirements.len());
    let mut candidate_start = 0;
    for requirement in requirements {
        while candidate_start < candidates.len()
            && candidates[candidate_start].key < requirement.key
        {
            candidate_start += 1;
        }
        let mut candidate_end = candidate_start;
        while candidate_end < candidates.len() && candidates[candidate_end].key == requirement.key {
            candidate_end += 1;
        }
        let matching = &candidates[candidate_start..candidate_end];
        let candidate = match matching {
            [] => return Err(LawPremiseRefusal::MissingRequiredPremise),
            [candidate] => *candidate,
            _ => return Err(LawPremiseRefusal::AmbiguousRequiredPremise),
        };
        let proof = match (requirement.proof, candidate.proof) {
            (RequirementProof::AdmittedContent, CandidateProof::AdmittedContent) => {
                SelectedProof::AdmittedContent
            }
            (RequirementProof::VerifiedDerivedContent, CandidateProof::VerifiedDerivedContent) => {
                SelectedProof::VerifiedDerivedContent
            }
            (
                RequirementProof::ExactZero {
                    subject_identity,
                    excluded_term_identity,
                    symmetry_identity,
                },
                CandidateProof::ExactZero(proof),
            ) if proof.subject_identity == subject_identity
                && proof.excluded_term_identity == excluded_term_identity
                && proof.symmetry_identity == symmetry_identity =>
            {
                SelectedProof::ExactZero(proof)
            }
            _ => return Err(LawPremiseRefusal::ProofKindMismatch),
        };
        selected.push(SelectedPremise {
            key: requirement.key,
            capability_sha256: candidate.capability_sha256,
            proof,
        });
        candidate_start = candidate_end;
    }

    Ok(LawPremiseSelection {
        claim_identity: request.claim_identity,
        selected,
    })
}

fn invalid_key(key: PremiseKey) -> bool {
    key.role.0 == [0; 32] || key.content.0 == [0; 32]
}

fn invalid_candidate(candidate: &ClaimScopedPremiseCapability) -> bool {
    if invalid_key(candidate.key) {
        return true;
    }
    match candidate.proof {
        CandidateProof::AdmittedContent | CandidateProof::VerifiedDerivedContent => false,
        CandidateProof::ExactZero(proof) => {
            proof.subject_identity == [0; 32]
                || proof.excluded_term_identity == [0; 32]
                || proof.symmetry_identity == [0; 32]
                || proof.subject_identity == proof.excluded_term_identity
                || proof.exclusion_receipt_sha256 == [0; 32]
        }
    }
}

fn evidence_is_valid(candidate: &ClaimScopedPremiseCapability) -> bool {
    let mut receipts = vec![
        candidate.upstream_capability_sha256,
        candidate.semantic_producer_receipt_sha256,
        candidate.semantic_watchdog_receipt_sha256,
        candidate.applicability_receipt_sha256,
        candidate.validity_receipt_sha256,
    ];
    if let CandidateProof::ExactZero(proof) = candidate.proof {
        receipts.push(proof.exclusion_receipt_sha256);
    }
    !receipts.contains(&[0; 32])
        && !receipts
            .iter()
            .enumerate()
            .any(|(index, receipt)| receipts[index + 1..].contains(receipt))
}

fn append_field(output: &mut Vec<u8>, tag: u16, bytes: &[u8]) {
    output.extend_from_slice(&tag.to_be_bytes());
    output.extend_from_slice(
        &u64::try_from(bytes.len())
            .expect("law-premise capability fields have fixed bounded lengths")
            .to_be_bytes(),
    );
    output.extend_from_slice(bytes);
}

pub(super) fn capability_digest(candidate: &ClaimScopedPremiseCapability) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(512);
    preimage.extend_from_slice(CAPABILITY_DOMAIN);
    append_field(&mut preimage, 1, &candidate.claim_identity.0);
    append_field(&mut preimage, 2, &candidate.key.role.0);
    append_field(&mut preimage, 3, &candidate.key.content.0);
    append_field(&mut preimage, 4, &candidate.upstream_capability_sha256);
    append_field(
        &mut preimage,
        5,
        &candidate.semantic_producer_receipt_sha256,
    );
    append_field(
        &mut preimage,
        6,
        &candidate.semantic_watchdog_receipt_sha256,
    );
    append_field(&mut preimage, 7, &candidate.applicability_receipt_sha256);
    append_field(&mut preimage, 8, &candidate.validity_receipt_sha256);
    match candidate.proof {
        CandidateProof::AdmittedContent => {
            append_field(&mut preimage, 9, &[0]);
            for tag in 10..=13 {
                append_field(&mut preimage, tag, &[]);
            }
        }
        CandidateProof::VerifiedDerivedContent => {
            append_field(&mut preimage, 9, &[2]);
            for tag in 10..=13 {
                append_field(&mut preimage, tag, &[]);
            }
        }
        CandidateProof::ExactZero(proof) => {
            append_field(&mut preimage, 9, &[1]);
            append_field(&mut preimage, 10, &proof.subject_identity);
            append_field(&mut preimage, 11, &proof.excluded_term_identity);
            append_field(&mut preimage, 12, &proof.symmetry_identity);
            append_field(&mut preimage, 13, &proof.exclusion_receipt_sha256);
        }
    }
    sha256(&preimage)
}
