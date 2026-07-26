//! Claim-scoped physical-law premise selection.
//!
//! A law request names opaque semantic-role and physical-content identities.
//! A candidate can satisfy that request only when one upstream capability is
//! bound to the same claim, role, content, applicability, validity, and
//! independently produced semantic evidence. No value, name, ordinal, catalog
//! position, or sole-candidate fallback exists in this model.
//!
//! Exact-zero claims carry a symmetry and exclusion proof object. A marker
//! saying that a term is absent cannot satisfy the request.
//!
//! This module checks selection mechanics only. It cannot prove that the
//! requested roles are physically sufficient, that an upstream capability is
//! scientifically sound, or that a law is true. Production has no constructor
//! for [`ClaimScopedPremiseCapability`], and every successful report has
//! `authority_effect=none`.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

const MAX_REQUIREMENT_COUNT: usize = 4_096;
const MAX_CANDIDATE_COUNT: usize = 8_192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LawClaimIdentity([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticRoleIdentity([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PhysicalContentIdentity([u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PremiseKey {
    role: SemanticRoleIdentity,
    content: PhysicalContentIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Evidence forms at this boundary, not a closed list of physical law kinds.
enum RequirementProof {
    AdmittedContent,
    ExactZero {
        subject_identity: [u8; 32],
        excluded_term_identity: [u8; 32],
        symmetry_identity: [u8; 32],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LawPremiseRequirement {
    key: PremiseKey,
    proof: RequirementProof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExactZeroProof {
    subject_identity: [u8; 32],
    excluded_term_identity: [u8; 32],
    symmetry_identity: [u8; 32],
    exclusion_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Candidate evidence forms, independent of the opaque physical role.
enum CandidateProof {
    AdmittedContent,
    ExactZero(ExactZeroProof),
}

/// Opaque upstream evidence for one exact claim-role-content tuple.
///
/// There is intentionally no production constructor. A future authority pair
/// must mint this object from a complete derived or irreducible admission
/// route and enroll that narrow claim in the authority watchdog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ClaimScopedPremiseCapability {
    claim_identity: LawClaimIdentity,
    key: PremiseKey,
    upstream_capability_sha256: [u8; 32],
    semantic_producer_receipt_sha256: [u8; 32],
    semantic_watchdog_receipt_sha256: [u8; 32],
    applicability_receipt_sha256: [u8; 32],
    validity_receipt_sha256: [u8; 32],
    proof: CandidateProof,
    capability_sha256: [u8; 32],
    _seal: PremiseCapabilitySeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PremiseCapabilitySeal;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LawPremiseRequest {
    claim_identity: LawClaimIdentity,
    requirements: Vec<LawPremiseRequirement>,
    candidates: Vec<ClaimScopedPremiseCapability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedProof {
    AdmittedContent,
    ExactZero(ExactZeroProof),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedPremise {
    key: PremiseKey,
    capability_sha256: [u8; 32],
    proof: SelectedProof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LawPremiseSelection {
    claim_identity: LawClaimIdentity,
    selected: Vec<SelectedPremise>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LawPremiseRefusal {
    InvalidClaimIdentity,
    EmptyRequirementSet,
    RequirementCapacityExceeded,
    CandidateCapacityExceeded,
    InvalidRequirement,
    DuplicateRequirement,
    InvalidCandidate,
    CandidateOutsideClaim,
    InvalidCapabilityEvidence,
    CapabilityDigestMismatch,
    MissingRequiredPremise,
    AmbiguousRequiredPremise,
    ProofKindMismatch,
    CheckerDisagreement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LawPremiseSelectionReport {
    selection: LawPremiseSelection,
}

impl LawPremiseSelectionReport {
    const fn requested_requirement_coverage(&self) -> bool {
        true
    }

    const fn global_physical_premise_coverage(&self) -> bool {
        false
    }

    const fn authority_effect(&self) -> &'static str {
        "none"
    }
}

fn inspect_law_premises(
    request: &LawPremiseRequest,
) -> Result<LawPremiseSelectionReport, LawPremiseRefusal> {
    let produced = producer::select(request);
    let watched = watchdog::select(request);
    match (produced, watched) {
        (Ok(produced), Ok(watched)) if produced == watched => Ok(LawPremiseSelectionReport {
            selection: produced,
        }),
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(LawPremiseRefusal::CheckerDisagreement),
    }
}
