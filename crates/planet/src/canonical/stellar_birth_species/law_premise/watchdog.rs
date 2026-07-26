//! Map-reconstructed physical-law premise selector.

use super::{
    CandidateProof, ClaimScopedPremiseCapability, LawPremiseRefusal, LawPremiseRequest,
    LawPremiseSelection, PremiseKey, RequirementProof, SelectedPremise, SelectedProof,
    MAX_CANDIDATE_COUNT, MAX_REQUIREMENT_COUNT,
};
use civsim_units::digest::sha256;
use std::collections::{BTreeMap, BTreeSet};

const CAPABILITY_DOMAIN: &[u8] = b"civsim.law-premise-capability.v1";

pub(super) fn select(
    request: &LawPremiseRequest,
) -> Result<LawPremiseSelection, LawPremiseRefusal> {
    if request.claim_identity.0.iter().all(|byte| *byte == 0) {
        return Err(LawPremiseRefusal::InvalidClaimIdentity);
    }
    let requirement_count = request.requirements.len();
    if requirement_count == 0 {
        return Err(LawPremiseRefusal::EmptyRequirementSet);
    }
    if requirement_count > MAX_REQUIREMENT_COUNT {
        return Err(LawPremiseRefusal::RequirementCapacityExceeded);
    }
    if request.candidates.len() > MAX_CANDIDATE_COUNT {
        return Err(LawPremiseRefusal::CandidateCapacityExceeded);
    }

    let invalid_requirement = request.requirements.iter().any(|requirement| {
        requirement.key.role.0.iter().all(|byte| *byte == 0)
            || requirement.key.content.0.iter().all(|byte| *byte == 0)
            || match requirement.proof {
                RequirementProof::AdmittedContent => false,
                RequirementProof::ExactZero {
                    subject_identity,
                    excluded_term_identity,
                    symmetry_identity,
                } => {
                    subject_identity.iter().all(|byte| *byte == 0)
                        || excluded_term_identity.iter().all(|byte| *byte == 0)
                        || symmetry_identity.iter().all(|byte| *byte == 0)
                        || subject_identity == excluded_term_identity
                }
            }
    });
    if invalid_requirement {
        return Err(LawPremiseRefusal::InvalidRequirement);
    }

    let mut requirements = BTreeMap::new();
    for requirement in &request.requirements {
        if requirements.insert(requirement.key, requirement).is_some() {
            return Err(LawPremiseRefusal::DuplicateRequirement);
        }
    }

    if request.candidates.iter().any(candidate_is_invalid) {
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
        .any(|candidate| !evidence_is_independent(candidate))
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

    let mut candidates_by_key: BTreeMap<PremiseKey, Vec<&ClaimScopedPremiseCapability>> =
        BTreeMap::new();
    for candidate in request.candidates.iter().rev() {
        candidates_by_key
            .entry(candidate.key)
            .or_default()
            .push(candidate);
    }

    let mut selected = Vec::with_capacity(requirements.len());
    for (key, requirement) in requirements {
        let candidates = candidates_by_key
            .get(&key)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let candidate = match candidates {
            [] => return Err(LawPremiseRefusal::MissingRequiredPremise),
            [candidate] => *candidate,
            _ => return Err(LawPremiseRefusal::AmbiguousRequiredPremise),
        };
        let proof = match (&requirement.proof, &candidate.proof) {
            (RequirementProof::AdmittedContent, CandidateProof::AdmittedContent) => {
                SelectedProof::AdmittedContent
            }
            (
                RequirementProof::ExactZero {
                    subject_identity,
                    excluded_term_identity,
                    symmetry_identity,
                },
                CandidateProof::ExactZero(proof),
            ) if proof.subject_identity == *subject_identity
                && proof.excluded_term_identity == *excluded_term_identity
                && proof.symmetry_identity == *symmetry_identity =>
            {
                SelectedProof::ExactZero(*proof)
            }
            _ => return Err(LawPremiseRefusal::ProofKindMismatch),
        };
        selected.push(SelectedPremise {
            key,
            capability_sha256: candidate.capability_sha256,
            proof,
        });
    }

    Ok(LawPremiseSelection {
        claim_identity: request.claim_identity,
        selected,
    })
}

fn candidate_is_invalid(candidate: &ClaimScopedPremiseCapability) -> bool {
    let key_is_zero = candidate.key.role.0.iter().all(|byte| *byte == 0)
        || candidate.key.content.0.iter().all(|byte| *byte == 0);
    if key_is_zero {
        return true;
    }
    if let CandidateProof::ExactZero(proof) = candidate.proof {
        return proof.subject_identity.iter().all(|byte| *byte == 0)
            || proof.excluded_term_identity.iter().all(|byte| *byte == 0)
            || proof.symmetry_identity.iter().all(|byte| *byte == 0)
            || proof.subject_identity == proof.excluded_term_identity
            || proof.exclusion_receipt_sha256.iter().all(|byte| *byte == 0);
    }
    false
}

fn evidence_is_independent(candidate: &ClaimScopedPremiseCapability) -> bool {
    let mut receipts = BTreeSet::new();
    let base = [
        candidate.upstream_capability_sha256,
        candidate.semantic_producer_receipt_sha256,
        candidate.semantic_watchdog_receipt_sha256,
        candidate.applicability_receipt_sha256,
        candidate.validity_receipt_sha256,
    ];
    for receipt in base {
        if receipt.iter().all(|byte| *byte == 0) || !receipts.insert(receipt) {
            return false;
        }
    }
    match candidate.proof {
        CandidateProof::AdmittedContent => true,
        CandidateProof::ExactZero(proof) => {
            !proof.exclusion_receipt_sha256.iter().all(|byte| *byte == 0)
                && receipts.insert(proof.exclusion_receipt_sha256)
        }
    }
}

pub(super) fn capability_digest(candidate: &ClaimScopedPremiseCapability) -> [u8; 32] {
    let proof_kind = match candidate.proof {
        CandidateProof::AdmittedContent => vec![0],
        CandidateProof::ExactZero(_) => vec![1],
    };
    let (subject, excluded_term, symmetry, exclusion_receipt) = match candidate.proof {
        CandidateProof::AdmittedContent => (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
        CandidateProof::ExactZero(proof) => (
            proof.subject_identity.to_vec(),
            proof.excluded_term_identity.to_vec(),
            proof.symmetry_identity.to_vec(),
            proof.exclusion_receipt_sha256.to_vec(),
        ),
    };
    let fields = BTreeMap::from([
        (1_u16, candidate.claim_identity.0.to_vec()),
        (2, candidate.key.role.0.to_vec()),
        (3, candidate.key.content.0.to_vec()),
        (4, candidate.upstream_capability_sha256.to_vec()),
        (5, candidate.semantic_producer_receipt_sha256.to_vec()),
        (6, candidate.semantic_watchdog_receipt_sha256.to_vec()),
        (7, candidate.applicability_receipt_sha256.to_vec()),
        (8, candidate.validity_receipt_sha256.to_vec()),
        (9, proof_kind),
        (10, subject),
        (11, excluded_term),
        (12, symmetry),
        (13, exclusion_receipt),
    ]);
    let field_bytes = fields
        .into_iter()
        .map(|(tag, bytes)| {
            let mut framed = Vec::with_capacity(10 + bytes.len());
            framed.extend_from_slice(&tag.to_be_bytes());
            framed.extend_from_slice(
                &u64::try_from(bytes.len())
                    .expect("law-premise capability fields have fixed bounded lengths")
                    .to_be_bytes(),
            );
            framed.extend_from_slice(&bytes);
            framed
        })
        .collect::<Vec<_>>();
    let mut preimage = CAPABILITY_DOMAIN.to_vec();
    for field in field_bytes {
        preimage.extend_from_slice(&field);
    }
    sha256(&preimage)
}
