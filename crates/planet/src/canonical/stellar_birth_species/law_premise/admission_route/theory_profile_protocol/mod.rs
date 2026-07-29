//! Independently paired protocol authority for one repository theory profile.
//!
//! Each side starts from the raw claim-scoped request, reconstructs the live
//! catalog and occupied-slot inventory, and executes its own derive-first
//! route. Each side also constructs and attests the typed coverage and
//! irreducible-protocol bindings. The shared layer only compares canonical
//! outputs and seals a non-minting agreement record.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use super::{
    profile_digest, profile_protocol, BuckinghamPiDisposition, ChaosDisposition,
    DerivationCoverageCapability, DerivationCoverageSeal, IrreducibleProtocolCapability,
    IrreducibleProtocolSeal, LawClaimIdentity, PhysicalContentIdentity,
    PremiseAdmissionCheckerOutput, PremiseAdmissionDecision, PremiseAdmissionEvaluation,
    PremiseAdmissionRouteInput, PremiseKey, SemanticRoleIdentity, TheoryProfileAdmissionEvidence,
    TheoryProfileAdmissionRequest, TheoryProfileProtocolSeal, PROFILE_CHAOS_PAIR_DOMAIN,
    PROFILE_EXHAUSTION_DOMAIN, PROFILE_GAP_DOMAIN, PROFILE_OWNER_PAIR_DOMAIN,
    PROFILE_PI_PAIR_DOMAIN, PROFILE_RESIDUAL_PAIR_DOMAIN, PROFILE_SLOT_IDENTITY_DOMAIN,
    PROFILE_SLOT_PAIR_DOMAIN, PROFILE_WATCHDOG_DOMAIN,
};
use civsim_units::digest::sha256;

pub(super) const SCHEMA_ID: &str = "civsim.planet.theory-profile-protocol-capability.v1";
pub(super) const PRODUCER_ID: &str = "civsim.planet.theory-profile-protocol.forward-authority.v1";
pub(super) const WATCHDOG_ID: &str = "civsim.planet.theory-profile-protocol.reverse-authority.v1";

const PRODUCER_TRACE_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.producer-trace.v1";
const WATCHDOG_TRACE_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.watchdog-trace.v1";
const PAIR_RECEIPT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.pair-receipt.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LiveCanaryTranscript {
    pub(super) transcript_id: &'static str,
    pub(super) case_count: u32,
    pub(super) transcript_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PreparedAssessment {
    pub(super) request_sha256: [u8; 32],
    pub(super) target_claim_identity: [u8; 32],
    pub(super) target_role_identity: [u8; 32],
    pub(super) target_content_identity: [u8; 32],
    pub(super) seed_count: u32,
    pub(super) rule_count: u32,
    pub(super) derivation_catalog_sha256: [u8; 32],
    pub(super) protocol: profile_protocol::ProfileProtocolAssessment,
    pub(super) residual_slot_identity: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedOutput {
    pub(super) authority_id: &'static str,
    pub(super) assessment: PreparedAssessment,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) protocol_receipts: profile_protocol::ProfileProtocolCheckerReceipts,
    pub(super) open_result_sha256: [u8; 32],
    pub(super) protocol_result_sha256: [u8; 32],
    pub(super) trace_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CoverageBindingOutput {
    pub(super) capability: DerivationCoverageCapability,
    pub(super) derivation_exhaustion_receipt_sha256: [u8; 32],
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) result_sha256: [u8; 32],
    pub(super) trace_sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PairedProtocolReceipts {
    pub(super) buckingham_pi_receipt_sha256: [u8; 32],
    pub(super) gap_law_receipt_sha256: [u8; 32],
    pub(super) chaos_protocol_receipt_sha256: [u8; 32],
    pub(super) residual_law_receipt_sha256: [u8; 32],
    pub(super) residual_slot_receipt_sha256: [u8; 32],
    pub(super) owner_admission_receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProtocolBindingOutput {
    pub(super) capability: IrreducibleProtocolCapability,
    pub(super) receipts: PairedProtocolReceipts,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) result_sha256: [u8; 32],
    pub(super) trace_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhaseOutput {
    pub(super) evaluation: PremiseAdmissionEvaluation,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) result_sha256: [u8; 32],
    pub(super) trace_sha256: [u8; 32],
    pub(super) checker_owned_receipt_sha256: [u8; 32],
}

pub(super) fn inspect(
    request: &TheoryProfileAdmissionRequest,
) -> Result<TheoryProfileAdmissionEvidence, &'static str> {
    let (produced, watched) = match (producer::prepare(request), watchdog::prepare(request)) {
        (Ok(produced), Ok(watched)) => (produced, watched),
        (Err(produced), Err(watched)) if produced == watched => return Err(produced),
        _ => return Err("theory_profile_protocol_prepare_disagreement"),
    };
    if produced.assessment != watched.assessment
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.trace_sha256 == [0; 32]
        || watched.trace_sha256 == [0; 32]
        || produced.trace_sha256 == watched.trace_sha256
        || produced.open_result_sha256 == watched.open_result_sha256
        || produced.protocol_result_sha256 == watched.protocol_result_sha256
    {
        return Err("theory_profile_protocol_prepare_disagreement");
    }

    let assessment = produced.assessment;
    let (coverage_bound_produced, coverage_bound_watched) = match (
        producer::bind_coverage(request, &produced, &watched),
        watchdog::bind_coverage(request, &produced, &watched),
    ) {
        (Ok(produced), Ok(watched)) => (produced, watched),
        (Err(produced), Err(watched)) if produced == watched => return Err(produced),
        _ => return Err("theory_profile_protocol_coverage_binding_disagreement"),
    };
    ensure_coverage_binding_agreement(&coverage_bound_produced, &coverage_bound_watched)?;
    let coverage = coverage_bound_produced.capability;

    let (coverage_produced, coverage_watched) = match (
        producer::inspect_coverage(request, coverage),
        watchdog::inspect_coverage(request, coverage),
    ) {
        (Ok(produced), Ok(watched)) => (produced, watched),
        (Err(produced), Err(watched)) if produced == watched => return Err(produced),
        _ => return Err("theory_profile_protocol_coverage_disagreement"),
    };
    ensure_phase_agreement(
        &coverage_produced,
        &coverage_watched,
        "theory_profile_protocol_coverage_disagreement",
    )?;
    if coverage_produced.checker_owned_receipt_sha256 != [0; 32]
        || coverage_watched.checker_owned_receipt_sha256 != [0; 32]
    {
        return Err("theory_profile_protocol_coverage_receipt_ownership_invalid");
    }

    let (protocol_bound_produced, protocol_bound_watched) = match (
        producer::bind_protocol(request, &produced, &watched, coverage),
        watchdog::bind_protocol(request, &produced, &watched, coverage),
    ) {
        (Ok(produced), Ok(watched)) => (produced, watched),
        (Err(produced), Err(watched)) if produced == watched => return Err(produced),
        _ => return Err("theory_profile_protocol_binding_disagreement"),
    };
    ensure_protocol_binding_agreement(&protocol_bound_produced, &protocol_bound_watched)?;
    let protocol = protocol_bound_produced.capability;
    let protocol_receipts = protocol_bound_produced.receipts;

    let (final_produced, final_watched) = match (
        producer::inspect_final(request, coverage, protocol),
        watchdog::inspect_final(request, coverage, protocol),
    ) {
        (Ok(produced), Ok(watched)) => (produced, watched),
        (Err(produced), Err(watched)) if produced == watched => return Err(produced),
        _ => return Err("theory_profile_protocol_final_disagreement"),
    };
    ensure_phase_agreement(
        &final_produced,
        &final_watched,
        "theory_profile_protocol_final_disagreement",
    )?;
    if final_produced.checker_owned_receipt_sha256 != [0; 32]
        || final_watched.checker_owned_receipt_sha256 == [0; 32]
    {
        return Err("theory_profile_protocol_watchdog_receipt_ownership_invalid");
    }
    let independent_watchdog_receipt_sha256 = final_watched.checker_owned_receipt_sha256;

    let derivation_exhaustion_receipt_sha256 =
        coverage_bound_produced.derivation_exhaustion_receipt_sha256;
    let buckingham_pi_receipt_sha256 = protocol_receipts.buckingham_pi_receipt_sha256;
    let gap_law_receipt_sha256 = protocol_receipts.gap_law_receipt_sha256;
    let chaos_protocol_receipt_sha256 = protocol_receipts.chaos_protocol_receipt_sha256;
    let residual_law_receipt_sha256 = protocol_receipts.residual_law_receipt_sha256;
    let residual_slot_receipt_sha256 = protocol_receipts.residual_slot_receipt_sha256;
    let owner_admission_receipt_sha256 = protocol_receipts.owner_admission_receipt_sha256;
    let profile_protocol_producer_trace_sha256 = tagged_digest(
        PRODUCER_TRACE_DOMAIN,
        &[
            &produced.trace_sha256,
            &coverage_bound_produced.trace_sha256,
            &coverage_produced.trace_sha256,
            &protocol_bound_produced.trace_sha256,
            &final_produced.trace_sha256,
        ],
    );
    let profile_protocol_watchdog_trace_sha256 = tagged_digest(
        WATCHDOG_TRACE_DOMAIN,
        &[
            &watched.trace_sha256,
            &coverage_bound_watched.trace_sha256,
            &coverage_watched.trace_sha256,
            &protocol_bound_watched.trace_sha256,
            &final_watched.trace_sha256,
        ],
    );
    if profile_protocol_producer_trace_sha256 == [0; 32]
        || profile_protocol_watchdog_trace_sha256 == [0; 32]
        || profile_protocol_producer_trace_sha256 == profile_protocol_watchdog_trace_sha256
    {
        return Err("theory_profile_protocol_aggregate_trace_invalid");
    }
    let producer_canary = producer::execute_live_canaries(request, coverage)?;
    let watchdog_canary = watchdog::execute_live_canaries(request, &produced, &watched, coverage)?;
    if producer_canary.transcript_id != producer::LIVE_CANARY_TRANSCRIPT_ID
        || watchdog_canary.transcript_id != watchdog::LIVE_CANARY_TRANSCRIPT_ID
        || producer_canary.transcript_id == watchdog_canary.transcript_id
        || producer_canary.case_count == 0
        || producer_canary.case_count != watchdog_canary.case_count
        || producer_canary.transcript_sha256 == [0; 32]
        || watchdog_canary.transcript_sha256 == [0; 32]
        || producer_canary.transcript_sha256 == watchdog_canary.transcript_sha256
    {
        return Err("theory_profile_protocol_live_canary_invalid");
    }
    let pair_receipt_sha256 = seal_pair_receipt(
        &[
            SCHEMA_ID.as_bytes(),
            PRODUCER_ID.as_bytes(),
            WATCHDOG_ID.as_bytes(),
            &assessment.request_sha256,
            &produced.open_result_sha256,
            &watched.open_result_sha256,
            &coverage_produced.result_sha256,
            &coverage_watched.result_sha256,
            &coverage_bound_produced.result_sha256,
            &coverage_bound_watched.result_sha256,
            &protocol_bound_produced.result_sha256,
            &protocol_bound_watched.result_sha256,
            &final_produced.result_sha256,
            &final_watched.result_sha256,
            &coverage.capability_sha256,
            &protocol.capability_sha256,
            &produced.protocol_result_sha256,
            &watched.protocol_result_sha256,
            &produced.trace_sha256,
            &watched.trace_sha256,
            &coverage_produced.trace_sha256,
            &coverage_watched.trace_sha256,
            &final_produced.trace_sha256,
            &final_watched.trace_sha256,
            &independent_watchdog_receipt_sha256,
            &profile_protocol_producer_trace_sha256,
            &profile_protocol_watchdog_trace_sha256,
            &derivation_exhaustion_receipt_sha256,
            &buckingham_pi_receipt_sha256,
            &gap_law_receipt_sha256,
            &chaos_protocol_receipt_sha256,
            &residual_law_receipt_sha256,
            &residual_slot_receipt_sha256,
            &owner_admission_receipt_sha256,
        ],
        producer_canary,
        watchdog_canary,
    );
    let combined = [
        derivation_exhaustion_receipt_sha256,
        buckingham_pi_receipt_sha256,
        gap_law_receipt_sha256,
        chaos_protocol_receipt_sha256,
        residual_law_receipt_sha256,
        residual_slot_receipt_sha256,
        owner_admission_receipt_sha256,
        independent_watchdog_receipt_sha256,
        pair_receipt_sha256,
    ];
    if combined.contains(&[0; 32])
        || combined
            .iter()
            .enumerate()
            .any(|(index, digest)| combined[index + 1..].contains(digest))
    {
        return Err("collapsed_theory_profile_protocol_receipts");
    }

    Ok(TheoryProfileAdmissionEvidence {
        profile_protocol_schema_id: SCHEMA_ID,
        profile_protocol_producer_id: PRODUCER_ID,
        profile_protocol_watchdog_id: WATCHDOG_ID,
        target_claim_identity: assessment.target_claim_identity,
        target_role_identity: assessment.target_role_identity,
        target_content_identity: assessment.target_content_identity,
        seed_count: assessment.seed_count,
        rule_count: assessment.rule_count,
        derivation_catalog_sha256: assessment.derivation_catalog_sha256,
        repository_catalog_sha256: assessment.protocol.repository_catalog_sha256,
        protocol_producer_result_sha256: produced.protocol_result_sha256,
        protocol_watchdog_result_sha256: watched.protocol_result_sha256,
        open_producer_result_sha256: produced.open_result_sha256,
        open_watchdog_result_sha256: watched.open_result_sha256,
        final_producer_result_sha256: final_produced.result_sha256,
        final_watchdog_result_sha256: final_watched.result_sha256,
        derivation_coverage_capability_sha256: coverage.capability_sha256,
        irreducible_protocol_capability_sha256: protocol.capability_sha256,
        derivation_exhaustion_receipt_sha256,
        buckingham_pi_receipt_sha256,
        gap_law_receipt_sha256,
        chaos_protocol_receipt_sha256,
        residual_law_receipt_sha256,
        residual_slot_identity: assessment.residual_slot_identity,
        residual_slot_receipt_sha256,
        owner_admission_receipt_sha256,
        independent_watchdog_receipt_sha256,
        profile_protocol_producer_trace_sha256,
        profile_protocol_watchdog_trace_sha256,
        profile_protocol_producer_canary_transcript_id: producer_canary.transcript_id,
        profile_protocol_producer_canary_case_count: producer_canary.case_count,
        profile_protocol_producer_canary_sha256: producer_canary.transcript_sha256,
        profile_protocol_watchdog_canary_transcript_id: watchdog_canary.transcript_id,
        profile_protocol_watchdog_canary_case_count: watchdog_canary.case_count,
        profile_protocol_watchdog_canary_sha256: watchdog_canary.transcript_sha256,
        profile_protocol_pair_receipt_sha256: pair_receipt_sha256,
        premise_admission_authority: false,
        species_membership_authority: false,
        global_derivation_coverage: false,
        authority_effect: "none",
        decision_id: final_produced.evaluation.decision.id(),
        _seal: TheoryProfileProtocolSeal,
    })
}

fn seal_pair_receipt(
    base_fields: &[&[u8]],
    producer_canary: LiveCanaryTranscript,
    watchdog_canary: LiveCanaryTranscript,
) -> [u8; 32] {
    let producer_case_count = producer_canary.case_count.to_be_bytes();
    let watchdog_case_count = watchdog_canary.case_count.to_be_bytes();
    let mut fields = Vec::with_capacity(base_fields.len() + 6);
    fields.extend_from_slice(base_fields);
    fields.extend_from_slice(&[
        producer_canary.transcript_id.as_bytes(),
        producer_case_count.as_slice(),
        producer_canary.transcript_sha256.as_slice(),
        watchdog_canary.transcript_id.as_bytes(),
        watchdog_case_count.as_slice(),
        watchdog_canary.transcript_sha256.as_slice(),
    ]);
    tagged_digest(PAIR_RECEIPT_DOMAIN, &fields)
}

fn ensure_phase_agreement(
    produced: &PhaseOutput,
    watched: &PhaseOutput,
    refusal: &'static str,
) -> Result<(), &'static str> {
    if produced.evaluation != watched.evaluation
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.result_sha256 == watched.result_sha256
        || produced.trace_sha256 == [0; 32]
        || watched.trace_sha256 == [0; 32]
        || produced.trace_sha256 == watched.trace_sha256
    {
        Err(refusal)
    } else {
        Ok(())
    }
}

fn ensure_coverage_binding_agreement(
    produced: &CoverageBindingOutput,
    watched: &CoverageBindingOutput,
) -> Result<(), &'static str> {
    if produced.capability != watched.capability
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.derivation_exhaustion_receipt_sha256
            != watched.derivation_exhaustion_receipt_sha256
        || produced.result_sha256 == [0; 32]
        || watched.result_sha256 == [0; 32]
        || produced.result_sha256 == watched.result_sha256
        || produced.trace_sha256 == [0; 32]
        || watched.trace_sha256 == [0; 32]
        || produced.trace_sha256 == watched.trace_sha256
    {
        Err("theory_profile_protocol_coverage_binding_disagreement")
    } else {
        Ok(())
    }
}

fn ensure_protocol_binding_agreement(
    produced: &ProtocolBindingOutput,
    watched: &ProtocolBindingOutput,
) -> Result<(), &'static str> {
    if produced.capability != watched.capability
        || produced.receipts != watched.receipts
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.result_sha256 == [0; 32]
        || watched.result_sha256 == [0; 32]
        || produced.result_sha256 == watched.result_sha256
        || produced.trace_sha256 == [0; 32]
        || watched.trace_sha256 == [0; 32]
        || produced.trace_sha256 == watched.trace_sha256
    {
        Err("theory_profile_protocol_binding_disagreement")
    } else {
        Ok(())
    }
}

pub(super) fn request_sha256(request: &TheoryProfileAdmissionRequest) -> [u8; 32] {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, SCHEMA_ID.as_bytes());
    append_field(&mut bytes, 2, &request.claim_identity);
    append_field(&mut bytes, 3, &request.role_identity);
    append_field(&mut bytes, 4, &request.content_identity);
    append_field(&mut bytes, 5, &request.profile_input_sha256);
    append_field(&mut bytes, 6, &request.source_custody_sha256);
    append_field(&mut bytes, 7, &request.applicability_receipt_sha256);
    append_field(&mut bytes, 8, &request.validity_receipt_sha256);
    append_field(&mut bytes, 9, request.residual_slot_id.as_bytes());
    let mut occupied = request
        .occupied_profile_slots
        .iter()
        .map(|slot| slot.trim().to_owned())
        .collect::<Vec<_>>();
    occupied.sort_unstable();
    for slot in occupied {
        append_field(&mut bytes, 10, slot.as_bytes());
    }
    append_field(&mut bytes, 11, request.owner_admission_record.as_bytes());
    sha256(&bytes)
}

pub(super) fn encode_assessment(assessment: &PreparedAssessment) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, SCHEMA_ID.as_bytes());
    append_field(&mut bytes, 2, &assessment.request_sha256);
    append_field(&mut bytes, 3, &assessment.target_claim_identity);
    append_field(&mut bytes, 4, &assessment.target_role_identity);
    append_field(&mut bytes, 5, &assessment.target_content_identity);
    append_field(&mut bytes, 6, &assessment.seed_count.to_be_bytes());
    append_field(&mut bytes, 7, &assessment.rule_count.to_be_bytes());
    append_field(&mut bytes, 8, &assessment.derivation_catalog_sha256);
    append_field(
        &mut bytes,
        9,
        &assessment.protocol.repository_catalog_sha256,
    );
    append_field(
        &mut bytes,
        10,
        &assessment.protocol.repository_seed_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        11,
        &assessment.protocol.repository_rule_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        12,
        &assessment.protocol.target_scoped_seed_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        13,
        &assessment.protocol.target_scoped_rule_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        14,
        &assessment.protocol.numeric_basis_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        15,
        &assessment
            .protocol
            .trajectory_coordinate_count
            .to_be_bytes(),
    );
    append_field(
        &mut bytes,
        16,
        &assessment.protocol.occupied_slot_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        17,
        &assessment.protocol.occupied_slot_inventory_sha256,
    );
    append_field(
        &mut bytes,
        18,
        &[u8::from(assessment.protocol.residual_slot_collision)],
    );
    append_field(&mut bytes, 19, &assessment.residual_slot_identity);
    bytes
}

pub(super) fn side_result_sha256(
    domain: &[u8],
    canonical_bytes: &[u8],
    receipts: Option<profile_protocol::ProfileProtocolCheckerReceipts>,
) -> [u8; 32] {
    let mut fields = vec![canonical_bytes];
    let encoded_receipts;
    if let Some(receipts) = receipts {
        encoded_receipts = encode_receipts(receipts);
        fields.push(&encoded_receipts);
    }
    tagged_digest(domain, &fields)
}

fn encode_receipts(receipts: profile_protocol::ProfileProtocolCheckerReceipts) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (tag, digest) in [
        receipts.coverage_sha256,
        receipts.buckingham_pi_sha256,
        receipts.gap_law_sha256,
        receipts.chaos_protocol_sha256,
        receipts.residual_law_sha256,
        receipts.residual_slot_sha256,
        receipts.owner_admission_sha256,
    ]
    .into_iter()
    .enumerate()
    {
        append_field(
            &mut bytes,
            u16::try_from(tag + 1).unwrap_or(u16::MAX),
            &digest,
        );
    }
    bytes
}

pub(super) fn tagged_digest(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, domain);
    for field in fields {
        append_field(&mut bytes, 2, field);
    }
    sha256(&bytes)
}

fn append_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}
