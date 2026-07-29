//! Forward authority for one raw theory-profile protocol request.

use super::{
    profile_digest, profile_protocol, tagged_digest, BuckinghamPiDisposition, ChaosDisposition,
    CoverageBindingOutput, DerivationCoverageCapability, DerivationCoverageSeal,
    IrreducibleProtocolCapability, IrreducibleProtocolSeal, LawClaimIdentity, LiveCanaryTranscript,
    PairedProtocolReceipts, PhaseOutput, PhysicalContentIdentity, PremiseAdmissionDecision,
    PremiseAdmissionRouteInput, PremiseKey, PreparedAssessment, PreparedOutput,
    ProtocolBindingOutput, SemanticRoleIdentity, TheoryProfileAdmissionRequest,
    PROFILE_CHAOS_PAIR_DOMAIN, PROFILE_EXHAUSTION_DOMAIN, PROFILE_GAP_DOMAIN,
    PROFILE_OWNER_PAIR_DOMAIN, PROFILE_PI_PAIR_DOMAIN, PROFILE_RESIDUAL_PAIR_DOMAIN,
    PROFILE_SLOT_IDENTITY_DOMAIN, PROFILE_SLOT_PAIR_DOMAIN,
};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

const MAX_OCCUPIED_PROFILE_SLOTS: usize = 4_096;
const MAX_CANARY_PREIMAGE_BYTES: usize = 1_048_576;
const EXPECTED_LIVE_CANARY_CASE_COUNT: u32 = 3;
pub(super) const LIVE_CANARY_TRANSCRIPT_ID: &str =
    "civsim.planet.theory-profile-protocol.forward-live-canary-transcript.v1";
const LIVE_CANARY_TRANSCRIPT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-live-canary-transcript.v1";
const LIVE_CANARY_CASE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-live-canary-case.v1";
const REQUEST_MUTANT_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-request-mutant-preimage.v1";
const COVERAGE_MUTANT_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-coverage-mutant-preimage.v1";
const OPEN_RESULT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.forward-open-result.v1";
const PROFILE_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-profile-result.v1";
const COVERAGE_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-coverage-result.v1";
const FINAL_RESULT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.forward-final-result.v1";
const PREPARE_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-prepare-trace.v1";
const COVERAGE_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-coverage-trace.v1";
const FINAL_TRACE_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.forward-final-trace.v1";
const COVERAGE_BINDING_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-coverage-binding-result.v1";
const COVERAGE_BINDING_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-coverage-binding-trace.v1";
const PROTOCOL_BINDING_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-protocol-binding-result.v1";
const PROTOCOL_BINDING_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.forward-protocol-binding-trace.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ForwardCanaryCase {
    case_identity: &'static str,
    mutant_preimage_sha256: [u8; 32],
    observed_refusal_id: &'static str,
}

pub(super) fn execute_live_canaries(
    request: &TheoryProfileAdmissionRequest,
    coverage: DerivationCoverageCapability,
) -> Result<LiveCanaryTranscript, &'static str> {
    let mut cases = Vec::with_capacity(EXPECTED_LIVE_CANARY_CASE_COUNT as usize);

    let mut zero_claim_request = request.clone();
    zero_claim_request.claim_identity = [0; 32];
    record_forward_refusal(
        &mut cases,
        "raw_request_zero_claim_identity",
        encode_request_mutant_forward(&zero_claim_request)?,
        prepare(&zero_claim_request),
        "invalid_theory_profile_admission_request",
    )?;

    let mut padded_slot_request = request.clone();
    padded_slot_request.residual_slot_id.insert(0, ' ');
    record_forward_refusal(
        &mut cases,
        "raw_request_noncanonical_residual_slot",
        encode_request_mutant_forward(&padded_slot_request)?,
        prepare(&padded_slot_request),
        "invalid_theory_profile_admission_request",
    )?;

    let mut substituted_coverage = coverage;
    substituted_coverage.capability_sha256[0] ^= 1;
    record_forward_refusal(
        &mut cases,
        "internal_coverage_capability_digest_substitution",
        encode_coverage_mutant_forward(&substituted_coverage)?,
        inspect_coverage(request, substituted_coverage),
        "theory_profile_protocol_coverage_route_refused",
    )?;

    let case_count =
        u32::try_from(cases.len()).map_err(|_| "theory_profile_protocol_producer_canary_failed")?;
    if case_count != EXPECTED_LIVE_CANARY_CASE_COUNT {
        return Err("theory_profile_protocol_producer_canary_failed");
    }
    let mut transcript = Vec::new();
    append_canary_field(&mut transcript, 1, LIVE_CANARY_TRANSCRIPT_DOMAIN)?;
    append_canary_field(&mut transcript, 2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes())?;
    append_canary_field(&mut transcript, 3, &case_count.to_be_bytes())?;
    let mut case_receipts = BTreeSet::new();
    for case in cases {
        let case_bytes = encode_forward_canary_case(case)?;
        let case_receipt = sha256(&case_bytes);
        if case_receipt == [0; 32] || !case_receipts.insert(case_receipt) {
            return Err("theory_profile_protocol_producer_canary_failed");
        }
        append_canary_field(&mut transcript, 4, &case_bytes)?;
    }
    let transcript_sha256 = sha256(&transcript);
    if transcript_sha256 == [0; 32] {
        return Err("theory_profile_protocol_producer_canary_failed");
    }
    Ok(LiveCanaryTranscript {
        transcript_id: LIVE_CANARY_TRANSCRIPT_ID,
        case_count,
        transcript_sha256,
    })
}

fn record_forward_refusal<T>(
    cases: &mut Vec<ForwardCanaryCase>,
    case_identity: &'static str,
    mutant_preimage: Vec<u8>,
    observed: Result<T, &'static str>,
    expected_refusal_id: &'static str,
) -> Result<(), &'static str> {
    let observed_refusal_id = match observed {
        Err(observed_refusal_id) if observed_refusal_id == expected_refusal_id => {
            observed_refusal_id
        }
        _ => return Err("theory_profile_protocol_producer_canary_failed"),
    };
    let mutant_preimage_sha256 = sha256(&mutant_preimage);
    if mutant_preimage_sha256 == [0; 32] {
        return Err("theory_profile_protocol_producer_canary_failed");
    }
    cases.push(ForwardCanaryCase {
        case_identity,
        mutant_preimage_sha256,
        observed_refusal_id,
    });
    Ok(())
}

fn encode_forward_canary_case(case: ForwardCanaryCase) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    append_canary_field(&mut bytes, 1, LIVE_CANARY_CASE_DOMAIN)?;
    append_canary_field(&mut bytes, 2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes())?;
    append_canary_field(&mut bytes, 3, case.case_identity.as_bytes())?;
    append_canary_field(&mut bytes, 4, &case.mutant_preimage_sha256)?;
    append_canary_field(&mut bytes, 5, b"refused")?;
    append_canary_field(&mut bytes, 6, case.observed_refusal_id.as_bytes())?;
    Ok(bytes)
}

fn encode_request_mutant_forward(
    request: &TheoryProfileAdmissionRequest,
) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    for (tag, value) in [
        (1, REQUEST_MUTANT_PREIMAGE_DOMAIN),
        (2, request.claim_identity.as_slice()),
        (3, request.role_identity.as_slice()),
        (4, request.content_identity.as_slice()),
        (5, request.profile_input_sha256.as_slice()),
        (6, request.source_custody_sha256.as_slice()),
        (7, request.applicability_receipt_sha256.as_slice()),
        (8, request.validity_receipt_sha256.as_slice()),
        (9, request.residual_slot_id.as_bytes()),
    ] {
        append_canary_field(&mut bytes, tag, value)?;
    }
    let mut occupied = request
        .occupied_profile_slots
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    occupied.sort_unstable();
    if occupied.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("theory_profile_protocol_producer_canary_failed");
    }
    append_canary_field(
        &mut bytes,
        10,
        &u32::try_from(occupied.len())
            .map_err(|_| "theory_profile_protocol_producer_canary_resource_limit")?
            .to_be_bytes(),
    )?;
    for slot in occupied {
        append_canary_field(&mut bytes, 11, slot.as_bytes())?;
    }
    append_canary_field(&mut bytes, 12, request.owner_admission_record.as_bytes())?;
    Ok(bytes)
}

fn encode_coverage_mutant_forward(
    coverage: &DerivationCoverageCapability,
) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    for (tag, value) in [
        (1, COVERAGE_MUTANT_PREIMAGE_DOMAIN),
        (2, coverage.claim_identity.0.as_slice()),
        (3, coverage.target.role.0.as_slice()),
        (4, coverage.target.content.0.as_slice()),
        (5, coverage.derivation_catalog_sha256.as_slice()),
        (6, coverage.producer_receipt_sha256.as_slice()),
        (7, coverage.watchdog_receipt_sha256.as_slice()),
        (8, coverage.capability_sha256.as_slice()),
    ] {
        append_canary_field(&mut bytes, tag, value)?;
    }
    Ok(bytes)
}

fn append_canary_field(bytes: &mut Vec<u8>, tag: u16, value: &[u8]) -> Result<(), &'static str> {
    let next_len = bytes
        .len()
        .checked_add(10)
        .and_then(|len| len.checked_add(value.len()))
        .ok_or("theory_profile_protocol_producer_canary_resource_limit")?;
    if next_len > MAX_CANARY_PREIMAGE_BYTES {
        return Err("theory_profile_protocol_producer_canary_resource_limit");
    }
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(value.len())
            .map_err(|_| "theory_profile_protocol_producer_canary_resource_limit")?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(value);
    Ok(())
}

pub(super) fn prepare(
    request: &TheoryProfileAdmissionRequest,
) -> Result<PreparedOutput, &'static str> {
    validate_request(request)?;
    let request_sha256 = super::request_sha256(request);
    let claim_identity = LawClaimIdentity(request.claim_identity);
    let target = PremiseKey {
        role: SemanticRoleIdentity(request.role_identity),
        content: PhysicalContentIdentity(request.content_identity),
    };
    let eps0 = super::super::derived_relation::repository_derived_relation_capability()
        .ok_or("sealed_derived_seed_unavailable")?;
    let route_input = PremiseAdmissionRouteInput {
        claim_identity,
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: None,
        irreducible_protocol: None,
    };
    let open = super::super::producer::route(&route_input)
        .map_err(|_| "theory_profile_protocol_open_route_refused")?;
    let PremiseAdmissionDecision::OpenDerivationFrontier(frontier) = &open.evaluation.decision
    else {
        return Err("theory_profile_was_not_an_open_derivation_frontier");
    };
    if open.evaluation.claim_identity != claim_identity
        || open.evaluation.target != target
        || open.evaluation.seed_count != 0
        || open.evaluation.rule_count != 0
        || !frontier.reachable.is_empty()
        || !frontier.unresolved_dependencies.is_empty()
    {
        return Err("unexpected_theory_profile_derivation_reachability");
    }

    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|_| "sealed_floor_unavailable_for_slot_check")?;
    let mut occupied_residual_slots = floor
        .entries()
        .filter_map(|entry| floor.receipt(&entry.id))
        .map(|receipt| receipt.residual_slot.trim().to_owned())
        .collect::<Vec<_>>();
    occupied_residual_slots.extend(
        request
            .occupied_profile_slots
            .iter()
            .map(|slot| slot.trim().to_owned()),
    );
    let protocol_input = profile_protocol::ProfileProtocolInput {
        claim_identity: request.claim_identity,
        role_identity: request.role_identity,
        content_identity: request.content_identity,
        profile_input_sha256: request.profile_input_sha256,
        source_custody_sha256: request.source_custody_sha256,
        applicability_receipt_sha256: request.applicability_receipt_sha256,
        validity_receipt_sha256: request.validity_receipt_sha256,
        derivation_catalog_sha256: frontier.derivation_catalog_sha256,
        repository_seeds: vec![profile_protocol::CatalogSeed {
            claim_identity: eps0.claim_identity.0,
            role_identity: eps0.key.role.0,
            content_identity: eps0.key.content.0,
            capability_sha256: eps0.capability_sha256,
        }],
        repository_rules: Vec::new(),
        residual_slot_id: request.residual_slot_id.clone(),
        occupied_residual_slots,
        owner_admission_record: request.owner_admission_record.clone(),
    };
    let protocol = profile_protocol::inspect_forward(&protocol_input)?;
    if protocol.assessment.repository_seed_count != 1
        || protocol.assessment.repository_rule_count != 0
        || protocol.assessment.target_scoped_seed_count != open.evaluation.seed_count
        || protocol.assessment.target_scoped_rule_count != open.evaluation.rule_count
        || protocol.assessment.numeric_basis_count != 0
        || protocol.assessment.trajectory_coordinate_count != 0
    {
        return Err("profile_protocol_catalog_mismatch");
    }
    if protocol.assessment.residual_slot_collision {
        return Err("profile_protocol_residual_slot_collision");
    }
    if !receipts_valid(protocol.receipts) {
        return Err("profile_protocol_receipt_invalid");
    }

    let residual_slot_identity = tagged_digest(
        PROFILE_SLOT_IDENTITY_DOMAIN,
        &[
            request.residual_slot_id.trim().as_bytes(),
            &request.content_identity,
        ],
    );
    let assessment = PreparedAssessment {
        request_sha256,
        target_claim_identity: request.claim_identity,
        target_role_identity: request.role_identity,
        target_content_identity: request.content_identity,
        seed_count: open.evaluation.seed_count,
        rule_count: open.evaluation.rule_count,
        derivation_catalog_sha256: frontier.derivation_catalog_sha256,
        protocol: protocol.assessment,
        residual_slot_identity,
    };
    let canonical_bytes = super::encode_assessment(&assessment);
    let open_result_sha256 =
        super::side_result_sha256(OPEN_RESULT_DOMAIN, &open.canonical_bytes, None);
    let protocol_result_sha256 = super::side_result_sha256(
        PROFILE_RESULT_DOMAIN,
        &canonical_bytes,
        Some(protocol.receipts),
    );
    let trace_sha256 = tagged_digest(
        PREPARE_TRACE_DOMAIN,
        &[
            &request_sha256,
            &eps0.capability_sha256,
            &open_result_sha256,
            &protocol_result_sha256,
            &residual_slot_identity,
        ],
    );
    if [
        request_sha256,
        open_result_sha256,
        protocol_result_sha256,
        trace_sha256,
        residual_slot_identity,
    ]
    .contains(&[0; 32])
    {
        return Err("theory_profile_protocol_prepare_receipt_invalid");
    }
    Ok(PreparedOutput {
        authority_id: super::PRODUCER_ID,
        assessment,
        canonical_bytes,
        protocol_receipts: protocol.receipts,
        open_result_sha256,
        protocol_result_sha256,
        trace_sha256,
    })
}

pub(super) fn bind_coverage(
    request: &TheoryProfileAdmissionRequest,
    produced: &PreparedOutput,
    watched: &PreparedOutput,
) -> Result<CoverageBindingOutput, &'static str> {
    validate_request(request)?;
    validate_prepared_pair(request, produced, watched)?;
    let assessment = produced.assessment;
    let mut capability = DerivationCoverageCapability {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target: request_target(request),
        derivation_catalog_sha256: assessment.derivation_catalog_sha256,
        producer_receipt_sha256: produced.protocol_receipts.coverage_sha256,
        watchdog_receipt_sha256: watched.protocol_receipts.coverage_sha256,
        capability_sha256: [0; 32],
        _seal: DerivationCoverageSeal,
    };
    capability.capability_sha256 = super::super::producer::coverage_capability_digest(&capability);
    if capability.capability_sha256 == [0; 32] {
        return Err("theory_profile_protocol_coverage_capability_invalid");
    }
    let derivation_exhaustion_receipt_sha256 = profile_digest(
        PROFILE_EXHAUSTION_DOMAIN,
        &[
            &produced.open_result_sha256,
            &watched.open_result_sha256,
            &capability.capability_sha256,
            &assessment.derivation_catalog_sha256,
        ],
    );
    let canonical_bytes =
        encode_coverage_binding(&capability, derivation_exhaustion_receipt_sha256);
    let result_sha256 =
        super::side_result_sha256(COVERAGE_BINDING_RESULT_DOMAIN, &canonical_bytes, None);
    let trace_sha256 = tagged_digest(
        COVERAGE_BINDING_TRACE_DOMAIN,
        &[
            &super::request_sha256(request),
            &result_sha256,
            &capability.capability_sha256,
            &derivation_exhaustion_receipt_sha256,
        ],
    );
    if [
        result_sha256,
        trace_sha256,
        derivation_exhaustion_receipt_sha256,
    ]
    .contains(&[0; 32])
        || result_sha256 == trace_sha256
    {
        return Err("theory_profile_protocol_coverage_binding_receipt_invalid");
    }
    Ok(CoverageBindingOutput {
        capability,
        derivation_exhaustion_receipt_sha256,
        canonical_bytes,
        result_sha256,
        trace_sha256,
    })
}

pub(super) fn bind_protocol(
    request: &TheoryProfileAdmissionRequest,
    produced: &PreparedOutput,
    watched: &PreparedOutput,
    coverage: DerivationCoverageCapability,
) -> Result<ProtocolBindingOutput, &'static str> {
    validate_request(request)?;
    validate_prepared_pair(request, produced, watched)?;
    let assessment = produced.assessment;
    if assessment.protocol.numeric_basis_count != 0 {
        return Err("theory_profile_protocol_buckingham_pi_disposition_unproved");
    }
    if assessment.protocol.trajectory_coordinate_count != 0 {
        return Err("theory_profile_protocol_chaos_disposition_unproved");
    }
    if coverage.claim_identity != LawClaimIdentity(request.claim_identity)
        || coverage.target != request_target(request)
        || coverage.derivation_catalog_sha256 != assessment.derivation_catalog_sha256
        || coverage.producer_receipt_sha256 != produced.protocol_receipts.coverage_sha256
        || coverage.watchdog_receipt_sha256 != watched.protocol_receipts.coverage_sha256
        || coverage.capability_sha256 == [0; 32]
        || super::super::producer::coverage_capability_digest(&coverage)
            != coverage.capability_sha256
    {
        return Err("theory_profile_protocol_coverage_binding_invalid");
    }

    let receipts = paired_protocol_receipts(produced, watched, assessment);
    if !paired_receipts_valid(receipts) {
        return Err("theory_profile_protocol_paired_receipt_invalid");
    }
    let mut capability = IrreducibleProtocolCapability {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target: request_target(request),
        derivation_coverage_capability_sha256: coverage.capability_sha256,
        buckingham_pi: BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256: produced.protocol_receipts.buckingham_pi_sha256,
            watchdog_receipt_sha256: watched.protocol_receipts.buckingham_pi_sha256,
        },
        gap_law_receipt_sha256: receipts.gap_law_receipt_sha256,
        chaos: ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256: produced
                .protocol_receipts
                .chaos_protocol_sha256,
            watchdog_inapplicability_receipt_sha256: watched
                .protocol_receipts
                .chaos_protocol_sha256,
        },
        residual_law_producer_receipt_sha256: produced.protocol_receipts.residual_law_sha256,
        residual_law_watchdog_receipt_sha256: watched.protocol_receipts.residual_law_sha256,
        residual_slot_identity: assessment.residual_slot_identity,
        unique_slot_producer_receipt_sha256: produced.protocol_receipts.residual_slot_sha256,
        unique_slot_watchdog_receipt_sha256: watched.protocol_receipts.residual_slot_sha256,
        owner_admission_receipt_sha256: receipts.owner_admission_receipt_sha256,
        capability_sha256: [0; 32],
        _seal: IrreducibleProtocolSeal,
    };
    capability.capability_sha256 = super::super::producer::protocol_capability_digest(&capability);
    if capability.capability_sha256 == [0; 32] {
        return Err("theory_profile_protocol_irreducible_capability_invalid");
    }
    let canonical_bytes = encode_protocol_binding(&capability, receipts);
    let result_sha256 =
        super::side_result_sha256(PROTOCOL_BINDING_RESULT_DOMAIN, &canonical_bytes, None);
    let trace_sha256 = tagged_digest(
        PROTOCOL_BINDING_TRACE_DOMAIN,
        &[
            &super::request_sha256(request),
            &result_sha256,
            &coverage.capability_sha256,
            &capability.capability_sha256,
        ],
    );
    if result_sha256 == [0; 32] || trace_sha256 == [0; 32] || result_sha256 == trace_sha256 {
        return Err("theory_profile_protocol_binding_receipt_invalid");
    }
    Ok(ProtocolBindingOutput {
        capability,
        receipts,
        canonical_bytes,
        result_sha256,
        trace_sha256,
    })
}

pub(super) fn inspect_coverage(
    request: &TheoryProfileAdmissionRequest,
    coverage: DerivationCoverageCapability,
) -> Result<PhaseOutput, &'static str> {
    validate_request(request)?;
    let target = request_target(request);
    let input = PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: Some(coverage),
        irreducible_protocol: None,
    };
    let output = super::super::producer::route(&input)
        .map_err(|_| "theory_profile_protocol_coverage_route_refused")?;
    match &output.evaluation.decision {
        PremiseAdmissionDecision::IrreducibleProtocolRequired {
            target: observed_target,
            derivation_coverage_capability_sha256,
        } if *observed_target == target
            && *derivation_coverage_capability_sha256 == coverage.capability_sha256 => {}
        _ => return Err("irreducible_protocol_was_not_required"),
    }
    validate_phase_evaluation(
        request,
        &output.evaluation,
        coverage.derivation_catalog_sha256,
    )?;
    phase_output(
        request,
        output,
        COVERAGE_RESULT_DOMAIN,
        COVERAGE_TRACE_DOMAIN,
        &coverage.capability_sha256,
    )
}

pub(super) fn inspect_final(
    request: &TheoryProfileAdmissionRequest,
    coverage: DerivationCoverageCapability,
    protocol: IrreducibleProtocolCapability,
) -> Result<PhaseOutput, &'static str> {
    validate_request(request)?;
    let target = request_target(request);
    let input = PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: Some(coverage),
        irreducible_protocol: Some(protocol),
    };
    let output = super::super::producer::route(&input)
        .map_err(|_| "theory_profile_protocol_final_route_refused")?;
    match &output.evaluation.decision {
        PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
            target: observed_target,
            derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256,
        } if *observed_target == target
            && *derivation_coverage_capability_sha256 == coverage.capability_sha256
            && *irreducible_protocol_capability_sha256 == protocol.capability_sha256 => {}
        _ => return Err("irreducible_protocol_not_structurally_bound"),
    }
    validate_phase_evaluation(
        request,
        &output.evaluation,
        coverage.derivation_catalog_sha256,
    )?;
    phase_output(
        request,
        output,
        FINAL_RESULT_DOMAIN,
        FINAL_TRACE_DOMAIN,
        &protocol.capability_sha256,
    )
}

fn phase_output(
    request: &TheoryProfileAdmissionRequest,
    output: super::PremiseAdmissionCheckerOutput,
    result_domain: &[u8],
    trace_domain: &[u8],
    capability_sha256: &[u8; 32],
) -> Result<PhaseOutput, &'static str> {
    let result_sha256 = super::side_result_sha256(result_domain, &output.canonical_bytes, None);
    let trace_sha256 = tagged_digest(
        trace_domain,
        &[
            &super::request_sha256(request),
            capability_sha256,
            &result_sha256,
        ],
    );
    if result_sha256 == [0; 32] || trace_sha256 == [0; 32] || result_sha256 == trace_sha256 {
        return Err("theory_profile_protocol_phase_receipt_invalid");
    }
    Ok(PhaseOutput {
        evaluation: output.evaluation,
        canonical_bytes: output.canonical_bytes,
        result_sha256,
        trace_sha256,
        checker_owned_receipt_sha256: [0; 32],
    })
}

fn validate_phase_evaluation(
    request: &TheoryProfileAdmissionRequest,
    evaluation: &super::PremiseAdmissionEvaluation,
    derivation_catalog_sha256: [u8; 32],
) -> Result<(), &'static str> {
    if evaluation.claim_identity != LawClaimIdentity(request.claim_identity)
        || evaluation.target != request_target(request)
        || evaluation.derivation_catalog_sha256 != derivation_catalog_sha256
        || evaluation.seed_count != 0
        || evaluation.rule_count != 0
    {
        Err("theory_profile_protocol_phase_scope_invalid")
    } else {
        Ok(())
    }
}

fn validate_prepared_pair(
    request: &TheoryProfileAdmissionRequest,
    produced: &PreparedOutput,
    watched: &PreparedOutput,
) -> Result<(), &'static str> {
    if produced.authority_id != super::PRODUCER_ID
        || watched.authority_id != super::WATCHDOG_ID
        || produced.assessment != watched.assessment
        || produced.canonical_bytes != watched.canonical_bytes
        || produced.assessment.request_sha256 != super::request_sha256(request)
        || produced.assessment.target_claim_identity != request.claim_identity
        || produced.assessment.target_role_identity != request.role_identity
        || produced.assessment.target_content_identity != request.content_identity
        || !receipts_valid(produced.protocol_receipts)
        || !receipts_valid(watched.protocol_receipts)
        || produced.protocol_receipts == watched.protocol_receipts
        || produced.open_result_sha256 == [0; 32]
        || watched.open_result_sha256 == [0; 32]
        || produced.open_result_sha256 == watched.open_result_sha256
        || produced.protocol_result_sha256 == [0; 32]
        || watched.protocol_result_sha256 == [0; 32]
        || produced.protocol_result_sha256 == watched.protocol_result_sha256
        || produced.trace_sha256 == [0; 32]
        || watched.trace_sha256 == [0; 32]
        || produced.trace_sha256 == watched.trace_sha256
    {
        Err("theory_profile_protocol_prepared_pair_invalid")
    } else {
        Ok(())
    }
}

fn paired_protocol_receipts(
    produced: &PreparedOutput,
    watched: &PreparedOutput,
    assessment: PreparedAssessment,
) -> PairedProtocolReceipts {
    PairedProtocolReceipts {
        buckingham_pi_receipt_sha256: profile_digest(
            PROFILE_PI_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.buckingham_pi_sha256,
                &watched.protocol_receipts.buckingham_pi_sha256,
            ],
        ),
        gap_law_receipt_sha256: profile_digest(
            PROFILE_GAP_DOMAIN,
            &[
                &produced.protocol_receipts.gap_law_sha256,
                &watched.protocol_receipts.gap_law_sha256,
                &produced.protocol_result_sha256,
                &watched.protocol_result_sha256,
            ],
        ),
        chaos_protocol_receipt_sha256: profile_digest(
            PROFILE_CHAOS_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.chaos_protocol_sha256,
                &watched.protocol_receipts.chaos_protocol_sha256,
            ],
        ),
        residual_law_receipt_sha256: profile_digest(
            PROFILE_RESIDUAL_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.residual_law_sha256,
                &watched.protocol_receipts.residual_law_sha256,
            ],
        ),
        residual_slot_receipt_sha256: profile_digest(
            PROFILE_SLOT_PAIR_DOMAIN,
            &[
                &assessment.residual_slot_identity,
                &produced.protocol_receipts.residual_slot_sha256,
                &watched.protocol_receipts.residual_slot_sha256,
            ],
        ),
        owner_admission_receipt_sha256: profile_digest(
            PROFILE_OWNER_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.owner_admission_sha256,
                &watched.protocol_receipts.owner_admission_sha256,
            ],
        ),
    }
}

fn paired_receipts_valid(receipts: PairedProtocolReceipts) -> bool {
    let values = [
        receipts.buckingham_pi_receipt_sha256,
        receipts.gap_law_receipt_sha256,
        receipts.chaos_protocol_receipt_sha256,
        receipts.residual_law_receipt_sha256,
        receipts.residual_slot_receipt_sha256,
        receipts.owner_admission_receipt_sha256,
    ];
    !values.contains(&[0; 32])
        && !values
            .iter()
            .enumerate()
            .any(|(index, digest)| values[index + 1..].contains(digest))
}

fn encode_coverage_binding(
    capability: &DerivationCoverageCapability,
    derivation_exhaustion_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (tag, value) in [
        (1, super::SCHEMA_ID.as_bytes()),
        (2, super::PRODUCER_ID.as_bytes()),
        (3, super::WATCHDOG_ID.as_bytes()),
        (4, capability.claim_identity.0.as_slice()),
        (5, capability.target.role.0.as_slice()),
        (6, capability.target.content.0.as_slice()),
        (7, capability.derivation_catalog_sha256.as_slice()),
        (8, capability.producer_receipt_sha256.as_slice()),
        (9, capability.watchdog_receipt_sha256.as_slice()),
        (10, capability.capability_sha256.as_slice()),
        (11, derivation_exhaustion_receipt_sha256.as_slice()),
    ] {
        append_binding_field(&mut bytes, tag, value);
    }
    bytes
}

fn encode_protocol_binding(
    capability: &IrreducibleProtocolCapability,
    receipts: PairedProtocolReceipts,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (tag, value) in [
        (1, super::SCHEMA_ID.as_bytes()),
        (2, super::PRODUCER_ID.as_bytes()),
        (3, super::WATCHDOG_ID.as_bytes()),
        (4, capability.claim_identity.0.as_slice()),
        (5, capability.target.role.0.as_slice()),
        (6, capability.target.content.0.as_slice()),
        (
            7,
            capability.derivation_coverage_capability_sha256.as_slice(),
        ),
    ] {
        append_binding_field(&mut bytes, tag, value);
    }
    match capability.buckingham_pi {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            residual_group_count,
        } => {
            append_binding_field(&mut bytes, 8, b"applicable");
            append_binding_field(&mut bytes, 9, &variable_basis_receipt_sha256);
            append_binding_field(&mut bytes, 10, &producer_receipt_sha256);
            append_binding_field(&mut bytes, 11, &watchdog_receipt_sha256);
            append_binding_field(&mut bytes, 12, &residual_group_count.to_be_bytes());
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            append_binding_field(&mut bytes, 8, b"semantically_inapplicable");
            append_binding_field(&mut bytes, 10, &producer_receipt_sha256);
            append_binding_field(&mut bytes, 11, &watchdog_receipt_sha256);
        }
    }
    append_binding_field(&mut bytes, 13, &capability.gap_law_receipt_sha256);
    match capability.chaos {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            append_binding_field(&mut bytes, 14, b"nondynamical");
            append_binding_field(&mut bytes, 15, &producer_inapplicability_receipt_sha256);
            append_binding_field(&mut bytes, 16, &watchdog_inapplicability_receipt_sha256);
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            append_binding_field(&mut bytes, 14, b"dynamical");
            append_binding_field(&mut bytes, 15, &regime_partition_producer_receipt_sha256);
            append_binding_field(&mut bytes, 16, &regime_partition_watchdog_receipt_sha256);
            append_binding_field(&mut bytes, 17, &transition_law_receipt_sha256);
        }
    }
    for (tag, value) in [
        (
            18,
            capability.residual_law_producer_receipt_sha256.as_slice(),
        ),
        (
            19,
            capability.residual_law_watchdog_receipt_sha256.as_slice(),
        ),
        (20, capability.residual_slot_identity.as_slice()),
        (
            21,
            capability.unique_slot_producer_receipt_sha256.as_slice(),
        ),
        (
            22,
            capability.unique_slot_watchdog_receipt_sha256.as_slice(),
        ),
        (23, capability.owner_admission_receipt_sha256.as_slice()),
        (24, capability.capability_sha256.as_slice()),
        (25, receipts.buckingham_pi_receipt_sha256.as_slice()),
        (26, receipts.gap_law_receipt_sha256.as_slice()),
        (27, receipts.chaos_protocol_receipt_sha256.as_slice()),
        (28, receipts.residual_law_receipt_sha256.as_slice()),
        (29, receipts.residual_slot_receipt_sha256.as_slice()),
        (30, receipts.owner_admission_receipt_sha256.as_slice()),
    ] {
        append_binding_field(&mut bytes, tag, value);
    }
    bytes
}

fn append_binding_field(bytes: &mut Vec<u8>, tag: u16, value: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
    bytes.extend_from_slice(value);
}

fn validate_request(request: &TheoryProfileAdmissionRequest) -> Result<(), &'static str> {
    let fixed = [
        request.claim_identity,
        request.role_identity,
        request.content_identity,
        request.profile_input_sha256,
        request.source_custody_sha256,
        request.applicability_receipt_sha256,
        request.validity_receipt_sha256,
    ];
    if fixed.contains(&[0; 32])
        || fixed
            .iter()
            .enumerate()
            .any(|(index, digest)| fixed[index + 1..].contains(digest))
        || !valid_canonical_text(&request.residual_slot_id)
        || !valid_canonical_text(&request.owner_admission_record)
        || request.occupied_profile_slots.len() > MAX_OCCUPIED_PROFILE_SLOTS
    {
        return Err("invalid_theory_profile_admission_request");
    }
    let mut occupied = BTreeSet::new();
    for slot in &request.occupied_profile_slots {
        if !valid_canonical_text(slot) || !occupied.insert(slot.as_str()) {
            return Err("invalid_theory_profile_admission_request");
        }
    }
    Ok(())
}

fn receipts_valid(receipts: profile_protocol::ProfileProtocolCheckerReceipts) -> bool {
    let values = [
        receipts.coverage_sha256,
        receipts.buckingham_pi_sha256,
        receipts.gap_law_sha256,
        receipts.chaos_protocol_sha256,
        receipts.residual_law_sha256,
        receipts.residual_slot_sha256,
        receipts.owner_admission_sha256,
    ];
    !values.contains(&[0; 32])
        && !values
            .iter()
            .enumerate()
            .any(|(index, digest)| values[index + 1..].contains(digest))
}

fn request_target(request: &TheoryProfileAdmissionRequest) -> PremiseKey {
    PremiseKey {
        role: SemanticRoleIdentity(request.role_identity),
        content: PhysicalContentIdentity(request.content_identity),
    }
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 192 && value.is_ascii()
}

fn valid_canonical_text(value: &str) -> bool {
    valid_text(value) && value == value.trim()
}
