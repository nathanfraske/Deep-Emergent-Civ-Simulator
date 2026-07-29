//! Reverse authority for one raw theory-profile protocol request.

use super::{
    profile_digest, profile_protocol, tagged_digest, BuckinghamPiDisposition, ChaosDisposition,
    CoverageBindingOutput, DerivationCoverageCapability, DerivationCoverageSeal,
    IrreducibleProtocolCapability, IrreducibleProtocolSeal, LawClaimIdentity, LiveCanaryTranscript,
    PairedProtocolReceipts, PhaseOutput, PhysicalContentIdentity, PremiseAdmissionDecision,
    PremiseAdmissionRouteInput, PremiseKey, PreparedAssessment, PreparedOutput,
    ProtocolBindingOutput, SemanticRoleIdentity, TheoryProfileAdmissionRequest,
    PROFILE_CHAOS_PAIR_DOMAIN, PROFILE_EXHAUSTION_DOMAIN, PROFILE_GAP_DOMAIN,
    PROFILE_OWNER_PAIR_DOMAIN, PROFILE_PI_PAIR_DOMAIN, PROFILE_RESIDUAL_PAIR_DOMAIN,
    PROFILE_SLOT_IDENTITY_DOMAIN, PROFILE_SLOT_PAIR_DOMAIN, PROFILE_WATCHDOG_DOMAIN,
};
use civsim_units::digest::sha256;
use std::collections::{BTreeMap, BTreeSet};

const MAX_OCCUPIED_PROFILE_SLOTS: usize = 4_096;
const MAX_CANARY_PREIMAGE_BYTES: usize = 1_048_576;
const EXPECTED_LIVE_CANARY_CASE_COUNT: u32 = 3;
pub(super) const LIVE_CANARY_TRANSCRIPT_ID: &str =
    "civsim.planet.theory-profile-protocol.reverse-live-canary-transcript.v1";
const LIVE_CANARY_TRANSCRIPT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-live-canary-transcript.v1";
const LIVE_CANARY_CASE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-live-canary-case.v1";
const REQUEST_MUTANT_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-request-mutant-preimage.v1";
const COVERAGE_MUTANT_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-coverage-mutant-preimage.v1";
const OPEN_RESULT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.reverse-open-result.v1";
const PROFILE_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-profile-result.v1";
const COVERAGE_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-coverage-result.v1";
const FINAL_RESULT_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.reverse-final-result.v1";
const PREPARE_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-prepare-trace.v1";
const COVERAGE_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-coverage-trace.v1";
const FINAL_TRACE_DOMAIN: &[u8] = b"civsim.planet.theory-profile-protocol.reverse-final-trace.v1";
const COVERAGE_BINDING_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-coverage-binding-result.v1";
const COVERAGE_BINDING_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-coverage-binding-trace.v1";
const PROTOCOL_BINDING_RESULT_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-protocol-binding-result.v1";
const PROTOCOL_BINDING_TRACE_DOMAIN: &[u8] =
    b"civsim.planet.theory-profile-protocol.reverse-protocol-binding-trace.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReverseCanaryObservation {
    mutant_preimage_sha256: [u8; 32],
    observed_refusal_id: &'static str,
}

pub(super) fn execute_live_canaries(
    request: &TheoryProfileAdmissionRequest,
    produced: &PreparedOutput,
    watched: &PreparedOutput,
    coverage: DerivationCoverageCapability,
) -> Result<LiveCanaryTranscript, &'static str> {
    let mut cases = BTreeMap::new();

    let mut padded_owner_request = request.clone();
    padded_owner_request.owner_admission_record.push(' ');
    insert_reverse_refusal(
        &mut cases,
        "raw_request_noncanonical_owner_record",
        encode_request_mutant_reverse(&padded_owner_request)?,
        prepare(&padded_owner_request),
        "invalid_theory_profile_admission_request",
    )?;

    let mut aliased_receipt_request = request.clone();
    aliased_receipt_request.validity_receipt_sha256 =
        aliased_receipt_request.applicability_receipt_sha256;
    insert_reverse_refusal(
        &mut cases,
        "raw_request_aliased_validity_receipt",
        encode_request_mutant_reverse(&aliased_receipt_request)?,
        prepare(&aliased_receipt_request),
        "invalid_theory_profile_admission_request",
    )?;

    let mut substituted_coverage = coverage;
    substituted_coverage.watchdog_receipt_sha256[31] ^= 0x80;
    insert_reverse_refusal(
        &mut cases,
        "internal_coverage_watchdog_binding_substitution",
        encode_coverage_mutant_reverse(&substituted_coverage)?,
        bind_protocol(request, produced, watched, substituted_coverage),
        "theory_profile_protocol_coverage_binding_invalid",
    )?;

    let case_count =
        u32::try_from(cases.len()).map_err(|_| "theory_profile_protocol_watchdog_canary_failed")?;
    if case_count != EXPECTED_LIVE_CANARY_CASE_COUNT {
        return Err("theory_profile_protocol_watchdog_canary_failed");
    }
    let mut transcript_fields = vec![
        (1, LIVE_CANARY_TRANSCRIPT_DOMAIN.to_vec()),
        (2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes().to_vec()),
        (3, case_count.to_le_bytes().to_vec()),
    ];
    let mut case_receipts = BTreeSet::new();
    for (offset, (case_identity, observation)) in cases.into_iter().enumerate() {
        let case_bytes = encode_reverse_canary_case(case_identity, observation)?;
        let case_receipt = sha256(&case_bytes);
        if case_receipt == [0; 32] || !case_receipts.insert(case_receipt) {
            return Err("theory_profile_protocol_watchdog_canary_failed");
        }
        transcript_fields.push((
            u16::try_from(offset + 100)
                .map_err(|_| "theory_profile_protocol_watchdog_canary_resource_limit")?,
            case_bytes,
        ));
    }
    let transcript = encode_reverse_canary_fields(transcript_fields)?;
    let transcript_sha256 = sha256(&transcript);
    if transcript_sha256 == [0; 32] {
        return Err("theory_profile_protocol_watchdog_canary_failed");
    }
    Ok(LiveCanaryTranscript {
        transcript_id: LIVE_CANARY_TRANSCRIPT_ID,
        case_count,
        transcript_sha256,
    })
}

fn insert_reverse_refusal<T>(
    cases: &mut BTreeMap<&'static str, ReverseCanaryObservation>,
    case_identity: &'static str,
    mutant_preimage: Vec<u8>,
    observed: Result<T, &'static str>,
    expected_refusal_id: &'static str,
) -> Result<(), &'static str> {
    let observed_refusal_id = match observed {
        Err(observed_refusal_id) if observed_refusal_id == expected_refusal_id => {
            observed_refusal_id
        }
        _ => return Err("theory_profile_protocol_watchdog_canary_failed"),
    };
    let mutant_preimage_sha256 = sha256(&mutant_preimage);
    if mutant_preimage_sha256 == [0; 32]
        || cases
            .insert(
                case_identity,
                ReverseCanaryObservation {
                    mutant_preimage_sha256,
                    observed_refusal_id,
                },
            )
            .is_some()
    {
        return Err("theory_profile_protocol_watchdog_canary_failed");
    }
    Ok(())
}

fn encode_reverse_canary_case(
    case_identity: &'static str,
    observation: ReverseCanaryObservation,
) -> Result<Vec<u8>, &'static str> {
    encode_reverse_canary_fields(vec![
        (6, observation.observed_refusal_id.as_bytes().to_vec()),
        (5, b"refused".to_vec()),
        (4, observation.mutant_preimage_sha256.to_vec()),
        (3, case_identity.as_bytes().to_vec()),
        (2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes().to_vec()),
        (1, LIVE_CANARY_CASE_DOMAIN.to_vec()),
    ])
}

fn encode_request_mutant_reverse(
    request: &TheoryProfileAdmissionRequest,
) -> Result<Vec<u8>, &'static str> {
    let occupied_set = request
        .occupied_profile_slots
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if occupied_set.len() != request.occupied_profile_slots.len() {
        return Err("theory_profile_protocol_watchdog_canary_failed");
    }
    let mut occupied = Vec::new();
    append_reverse_canary_atom(
        &mut occupied,
        1,
        &u32::try_from(occupied_set.len())
            .map_err(|_| "theory_profile_protocol_watchdog_canary_resource_limit")?
            .to_le_bytes(),
    )?;
    for slot in occupied_set.iter().rev() {
        append_reverse_canary_atom(&mut occupied, 2, slot.as_bytes())?;
    }
    encode_reverse_canary_fields(vec![
        (12, request.owner_admission_record.as_bytes().to_vec()),
        (11, occupied),
        (10, request.residual_slot_id.as_bytes().to_vec()),
        (9, request.validity_receipt_sha256.to_vec()),
        (8, request.applicability_receipt_sha256.to_vec()),
        (7, request.source_custody_sha256.to_vec()),
        (6, request.profile_input_sha256.to_vec()),
        (5, request.content_identity.to_vec()),
        (4, request.role_identity.to_vec()),
        (3, request.claim_identity.to_vec()),
        (2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes().to_vec()),
        (1, REQUEST_MUTANT_PREIMAGE_DOMAIN.to_vec()),
    ])
}

fn encode_coverage_mutant_reverse(
    coverage: &DerivationCoverageCapability,
) -> Result<Vec<u8>, &'static str> {
    encode_reverse_canary_fields(vec![
        (9, coverage.capability_sha256.to_vec()),
        (8, coverage.watchdog_receipt_sha256.to_vec()),
        (7, coverage.producer_receipt_sha256.to_vec()),
        (6, coverage.derivation_catalog_sha256.to_vec()),
        (5, coverage.target.content.0.to_vec()),
        (4, coverage.target.role.0.to_vec()),
        (3, coverage.claim_identity.0.to_vec()),
        (2, LIVE_CANARY_TRANSCRIPT_ID.as_bytes().to_vec()),
        (1, COVERAGE_MUTANT_PREIMAGE_DOMAIN.to_vec()),
    ])
}

fn encode_reverse_canary_fields(mut fields: Vec<(u16, Vec<u8>)>) -> Result<Vec<u8>, &'static str> {
    fields.sort_unstable_by(|left, right| right.0.cmp(&left.0));
    let mut bytes = Vec::new();
    for (tag, value) in fields {
        append_reverse_canary_atom(&mut bytes, tag, &value)?;
    }
    Ok(bytes)
}

fn append_reverse_canary_atom(
    bytes: &mut Vec<u8>,
    tag: u16,
    value: &[u8],
) -> Result<(), &'static str> {
    let next_len = bytes
        .len()
        .checked_add(10)
        .and_then(|len| len.checked_add(value.len()))
        .ok_or("theory_profile_protocol_watchdog_canary_resource_limit")?;
    if next_len > MAX_CANARY_PREIMAGE_BYTES {
        return Err("theory_profile_protocol_watchdog_canary_resource_limit");
    }
    bytes.extend_from_slice(
        &u64::try_from(value.len())
            .map_err(|_| "theory_profile_protocol_watchdog_canary_resource_limit")?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(value);
    bytes.extend_from_slice(&tag.to_le_bytes());
    Ok(())
}

pub(super) fn prepare(
    request: &TheoryProfileAdmissionRequest,
) -> Result<PreparedOutput, &'static str> {
    validate_request_reverse(request)?;
    let request_sha256 = super::request_sha256(request);
    let claim_identity = LawClaimIdentity(request.claim_identity);
    let target = request_target(request);
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
    let open = super::super::watchdog::route(&route_input)
        .map_err(|_| "theory_profile_protocol_open_route_refused")?;
    let PremiseAdmissionDecision::OpenDerivationFrontier(frontier) = &open.evaluation.decision
    else {
        return Err("theory_profile_was_not_an_open_derivation_frontier");
    };
    if !frontier.unresolved_dependencies.is_empty()
        || !frontier.reachable.is_empty()
        || open.evaluation.rule_count != 0
        || open.evaluation.seed_count != 0
        || open.evaluation.target != target
        || open.evaluation.claim_identity != claim_identity
    {
        return Err("unexpected_theory_profile_derivation_reachability");
    }

    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|_| "sealed_floor_unavailable_for_slot_check")?;
    let mut floor_entries = floor.entries().collect::<Vec<_>>();
    floor_entries.reverse();
    let mut occupied_residual_slots =
        Vec::with_capacity(floor_entries.len() + request.occupied_profile_slots.len());
    for entry in floor_entries {
        if let Some(receipt) = floor.receipt(&entry.id) {
            occupied_residual_slots.push(receipt.residual_slot.trim().to_owned());
        }
    }
    for slot in request.occupied_profile_slots.iter().rev() {
        occupied_residual_slots.push(slot.trim().to_owned());
    }
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
    let protocol = profile_protocol::inspect_reverse(&protocol_input)?;
    if protocol.assessment.trajectory_coordinate_count != 0
        || protocol.assessment.numeric_basis_count != 0
        || protocol.assessment.target_scoped_rule_count != open.evaluation.rule_count
        || protocol.assessment.target_scoped_seed_count != open.evaluation.seed_count
        || protocol.assessment.repository_rule_count != 0
        || protocol.assessment.repository_seed_count != 1
    {
        return Err("profile_protocol_catalog_mismatch");
    }
    if protocol.assessment.residual_slot_collision {
        return Err("profile_protocol_residual_slot_collision");
    }
    if !receipts_valid_reverse(protocol.receipts) {
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
            &residual_slot_identity,
            &protocol_result_sha256,
            &open_result_sha256,
            &eps0.capability_sha256,
            &request_sha256,
        ],
    );
    if [
        residual_slot_identity,
        trace_sha256,
        protocol_result_sha256,
        open_result_sha256,
        request_sha256,
    ]
    .iter()
    .any(|digest| digest.iter().all(|byte| *byte == 0))
    {
        return Err("theory_profile_protocol_prepare_receipt_invalid");
    }
    Ok(PreparedOutput {
        authority_id: super::WATCHDOG_ID,
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
    validate_request_reverse(request)?;
    validate_prepared_pair_reverse(request, produced, watched)?;
    let assessment = watched.assessment;
    let mut capability = DerivationCoverageCapability {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target: request_target(request),
        derivation_catalog_sha256: assessment.derivation_catalog_sha256,
        producer_receipt_sha256: produced.protocol_receipts.coverage_sha256,
        watchdog_receipt_sha256: watched.protocol_receipts.coverage_sha256,
        capability_sha256: [0; 32],
        _seal: DerivationCoverageSeal,
    };
    capability.capability_sha256 = super::super::watchdog::coverage_capability_digest(&capability);
    if capability.capability_sha256.iter().all(|byte| *byte == 0) {
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
        encode_coverage_binding_reverse(&capability, derivation_exhaustion_receipt_sha256);
    let result_sha256 =
        super::side_result_sha256(COVERAGE_BINDING_RESULT_DOMAIN, &canonical_bytes, None);
    let trace_sha256 = tagged_digest(
        COVERAGE_BINDING_TRACE_DOMAIN,
        &[
            &derivation_exhaustion_receipt_sha256,
            &capability.capability_sha256,
            &result_sha256,
            &super::request_sha256(request),
        ],
    );
    let receipts = [
        derivation_exhaustion_receipt_sha256,
        trace_sha256,
        result_sha256,
    ];
    if receipts
        .iter()
        .any(|digest| digest.iter().all(|byte| *byte == 0))
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
    validate_request_reverse(request)?;
    validate_prepared_pair_reverse(request, produced, watched)?;
    let assessment = watched.assessment;
    match (
        assessment.protocol.trajectory_coordinate_count,
        assessment.protocol.numeric_basis_count,
    ) {
        (0, 0) => {}
        (_, 0) => return Err("theory_profile_protocol_chaos_disposition_unproved"),
        _ => return Err("theory_profile_protocol_buckingham_pi_disposition_unproved"),
    }
    if super::super::watchdog::coverage_capability_digest(&coverage) != coverage.capability_sha256
        || coverage.capability_sha256 == [0; 32]
        || coverage.watchdog_receipt_sha256 != watched.protocol_receipts.coverage_sha256
        || coverage.producer_receipt_sha256 != produced.protocol_receipts.coverage_sha256
        || coverage.derivation_catalog_sha256 != assessment.derivation_catalog_sha256
        || coverage.target != request_target(request)
        || coverage.claim_identity != LawClaimIdentity(request.claim_identity)
    {
        return Err("theory_profile_protocol_coverage_binding_invalid");
    }

    let receipts = paired_protocol_receipts_reverse(watched, produced, assessment);
    if !paired_receipts_valid_reverse(receipts) {
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
    capability.capability_sha256 = super::super::watchdog::protocol_capability_digest(&capability);
    if capability.capability_sha256.iter().all(|byte| *byte == 0) {
        return Err("theory_profile_protocol_irreducible_capability_invalid");
    }
    let canonical_bytes = encode_protocol_binding_reverse(&capability, receipts);
    let result_sha256 =
        super::side_result_sha256(PROTOCOL_BINDING_RESULT_DOMAIN, &canonical_bytes, None);
    let trace_sha256 = tagged_digest(
        PROTOCOL_BINDING_TRACE_DOMAIN,
        &[
            &capability.capability_sha256,
            &coverage.capability_sha256,
            &result_sha256,
            &super::request_sha256(request),
        ],
    );
    if result_sha256.iter().all(|byte| *byte == 0)
        || trace_sha256.iter().all(|byte| *byte == 0)
        || result_sha256 == trace_sha256
    {
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
    validate_request_reverse(request)?;
    let target = request_target(request);
    let input = PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: Some(coverage),
        irreducible_protocol: None,
    };
    let output = super::super::watchdog::route(&input)
        .map_err(|_| "theory_profile_protocol_coverage_route_refused")?;
    match &output.evaluation.decision {
        PremiseAdmissionDecision::IrreducibleProtocolRequired {
            target: observed_target,
            derivation_coverage_capability_sha256,
        } if *derivation_coverage_capability_sha256 == coverage.capability_sha256
            && *observed_target == target => {}
        _ => return Err("irreducible_protocol_was_not_required"),
    }
    validate_phase_evaluation_reverse(
        request,
        &output.evaluation,
        coverage.derivation_catalog_sha256,
    )?;
    phase_output_reverse(
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
    validate_request_reverse(request)?;
    let target = request_target(request);
    let input = PremiseAdmissionRouteInput {
        claim_identity: LawClaimIdentity(request.claim_identity),
        target,
        admitted_seeds: Vec::new(),
        admitted_rules: Vec::new(),
        derivation_coverage: Some(coverage),
        irreducible_protocol: Some(protocol),
    };
    let output = super::super::watchdog::route(&input)
        .map_err(|_| "theory_profile_protocol_final_route_refused")?;
    match &output.evaluation.decision {
        PremiseAdmissionDecision::IrreducibleProtocolStructurallyBound {
            target: observed_target,
            derivation_coverage_capability_sha256,
            irreducible_protocol_capability_sha256,
        } if *irreducible_protocol_capability_sha256 == protocol.capability_sha256
            && *derivation_coverage_capability_sha256 == coverage.capability_sha256
            && *observed_target == target => {}
        _ => return Err("irreducible_protocol_not_structurally_bound"),
    }
    validate_phase_evaluation_reverse(
        request,
        &output.evaluation,
        coverage.derivation_catalog_sha256,
    )?;
    let mut phase = phase_output_reverse(
        request,
        output,
        FINAL_RESULT_DOMAIN,
        FINAL_TRACE_DOMAIN,
        &protocol.capability_sha256,
    )?;
    let checker_owned_receipt_sha256 = profile_digest(
        PROFILE_WATCHDOG_DOMAIN,
        &[
            &super::request_sha256(request),
            &phase.result_sha256,
            &phase.trace_sha256,
            &coverage.capability_sha256,
            &protocol.capability_sha256,
            &coverage.derivation_catalog_sha256,
        ],
    );
    if checker_owned_receipt_sha256 == [0; 32]
        || checker_owned_receipt_sha256 == phase.result_sha256
        || checker_owned_receipt_sha256 == phase.trace_sha256
    {
        return Err("theory_profile_protocol_watchdog_receipt_invalid");
    }
    phase.checker_owned_receipt_sha256 = checker_owned_receipt_sha256;
    Ok(phase)
}

fn phase_output_reverse(
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
            &result_sha256,
            capability_sha256,
            &super::request_sha256(request),
        ],
    );
    if result_sha256.iter().all(|byte| *byte == 0)
        || trace_sha256.iter().all(|byte| *byte == 0)
        || result_sha256 == trace_sha256
    {
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

fn validate_phase_evaluation_reverse(
    request: &TheoryProfileAdmissionRequest,
    evaluation: &super::PremiseAdmissionEvaluation,
    derivation_catalog_sha256: [u8; 32],
) -> Result<(), &'static str> {
    if evaluation.rule_count != 0
        || evaluation.seed_count != 0
        || evaluation.derivation_catalog_sha256 != derivation_catalog_sha256
        || evaluation.target != request_target(request)
        || evaluation.claim_identity != LawClaimIdentity(request.claim_identity)
    {
        Err("theory_profile_protocol_phase_scope_invalid")
    } else {
        Ok(())
    }
}

fn validate_prepared_pair_reverse(
    request: &TheoryProfileAdmissionRequest,
    produced: &PreparedOutput,
    watched: &PreparedOutput,
) -> Result<(), &'static str> {
    let digests = [
        watched.trace_sha256,
        produced.trace_sha256,
        watched.protocol_result_sha256,
        produced.protocol_result_sha256,
        watched.open_result_sha256,
        produced.open_result_sha256,
    ];
    let distinct = digests.into_iter().collect::<BTreeSet<_>>();
    if watched.authority_id != super::WATCHDOG_ID
        || produced.authority_id != super::PRODUCER_ID
        || watched.canonical_bytes != produced.canonical_bytes
        || watched.assessment != produced.assessment
        || watched.assessment.request_sha256 != super::request_sha256(request)
        || (
            watched.assessment.target_content_identity,
            watched.assessment.target_role_identity,
            watched.assessment.target_claim_identity,
        ) != (
            request.content_identity,
            request.role_identity,
            request.claim_identity,
        )
        || !receipts_valid_reverse(watched.protocol_receipts)
        || !receipts_valid_reverse(produced.protocol_receipts)
        || watched.protocol_receipts == produced.protocol_receipts
        || distinct.len() != digests.len()
        || distinct
            .iter()
            .any(|digest| digest.iter().all(|byte| *byte == 0))
    {
        Err("theory_profile_protocol_prepared_pair_invalid")
    } else {
        Ok(())
    }
}

fn paired_protocol_receipts_reverse(
    watched: &PreparedOutput,
    produced: &PreparedOutput,
    assessment: PreparedAssessment,
) -> PairedProtocolReceipts {
    PairedProtocolReceipts {
        owner_admission_receipt_sha256: profile_digest(
            PROFILE_OWNER_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.owner_admission_sha256,
                &watched.protocol_receipts.owner_admission_sha256,
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
        residual_law_receipt_sha256: profile_digest(
            PROFILE_RESIDUAL_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.residual_law_sha256,
                &watched.protocol_receipts.residual_law_sha256,
            ],
        ),
        chaos_protocol_receipt_sha256: profile_digest(
            PROFILE_CHAOS_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.chaos_protocol_sha256,
                &watched.protocol_receipts.chaos_protocol_sha256,
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
        buckingham_pi_receipt_sha256: profile_digest(
            PROFILE_PI_PAIR_DOMAIN,
            &[
                &produced.protocol_receipts.buckingham_pi_sha256,
                &watched.protocol_receipts.buckingham_pi_sha256,
            ],
        ),
    }
}

fn paired_receipts_valid_reverse(receipts: PairedProtocolReceipts) -> bool {
    let values = [
        receipts.owner_admission_receipt_sha256,
        receipts.residual_slot_receipt_sha256,
        receipts.residual_law_receipt_sha256,
        receipts.chaos_protocol_receipt_sha256,
        receipts.gap_law_receipt_sha256,
        receipts.buckingham_pi_receipt_sha256,
    ];
    let distinct = values.into_iter().collect::<BTreeSet<_>>();
    distinct.len() == values.len()
        && distinct
            .iter()
            .all(|digest| digest.iter().any(|byte| *byte != 0))
}

fn encode_coverage_binding_reverse(
    capability: &DerivationCoverageCapability,
    derivation_exhaustion_receipt_sha256: [u8; 32],
) -> Vec<u8> {
    encode_tagged_fields_reverse(vec![
        (11, derivation_exhaustion_receipt_sha256.to_vec()),
        (10, capability.capability_sha256.to_vec()),
        (9, capability.watchdog_receipt_sha256.to_vec()),
        (8, capability.producer_receipt_sha256.to_vec()),
        (7, capability.derivation_catalog_sha256.to_vec()),
        (6, capability.target.content.0.to_vec()),
        (5, capability.target.role.0.to_vec()),
        (4, capability.claim_identity.0.to_vec()),
        (3, super::WATCHDOG_ID.as_bytes().to_vec()),
        (2, super::PRODUCER_ID.as_bytes().to_vec()),
        (1, super::SCHEMA_ID.as_bytes().to_vec()),
    ])
}

fn encode_protocol_binding_reverse(
    capability: &IrreducibleProtocolCapability,
    receipts: PairedProtocolReceipts,
) -> Vec<u8> {
    let mut fields = vec![
        (30, receipts.owner_admission_receipt_sha256.to_vec()),
        (29, receipts.residual_slot_receipt_sha256.to_vec()),
        (28, receipts.residual_law_receipt_sha256.to_vec()),
        (27, receipts.chaos_protocol_receipt_sha256.to_vec()),
        (26, receipts.gap_law_receipt_sha256.to_vec()),
        (25, receipts.buckingham_pi_receipt_sha256.to_vec()),
        (24, capability.capability_sha256.to_vec()),
        (23, capability.owner_admission_receipt_sha256.to_vec()),
        (22, capability.unique_slot_watchdog_receipt_sha256.to_vec()),
        (21, capability.unique_slot_producer_receipt_sha256.to_vec()),
        (20, capability.residual_slot_identity.to_vec()),
        (19, capability.residual_law_watchdog_receipt_sha256.to_vec()),
        (18, capability.residual_law_producer_receipt_sha256.to_vec()),
        (13, capability.gap_law_receipt_sha256.to_vec()),
        (7, capability.derivation_coverage_capability_sha256.to_vec()),
        (6, capability.target.content.0.to_vec()),
        (5, capability.target.role.0.to_vec()),
        (4, capability.claim_identity.0.to_vec()),
        (3, super::WATCHDOG_ID.as_bytes().to_vec()),
        (2, super::PRODUCER_ID.as_bytes().to_vec()),
        (1, super::SCHEMA_ID.as_bytes().to_vec()),
    ];
    match capability.chaos {
        ChaosDisposition::Nondynamical {
            producer_inapplicability_receipt_sha256,
            watchdog_inapplicability_receipt_sha256,
        } => {
            fields.push((16, watchdog_inapplicability_receipt_sha256.to_vec()));
            fields.push((15, producer_inapplicability_receipt_sha256.to_vec()));
            fields.push((14, b"nondynamical".to_vec()));
        }
        ChaosDisposition::Dynamical {
            regime_partition_producer_receipt_sha256,
            regime_partition_watchdog_receipt_sha256,
            transition_law_receipt_sha256,
        } => {
            fields.push((17, transition_law_receipt_sha256.to_vec()));
            fields.push((16, regime_partition_watchdog_receipt_sha256.to_vec()));
            fields.push((15, regime_partition_producer_receipt_sha256.to_vec()));
            fields.push((14, b"dynamical".to_vec()));
        }
    }
    match capability.buckingham_pi {
        BuckinghamPiDisposition::Applicable {
            variable_basis_receipt_sha256,
            producer_receipt_sha256,
            watchdog_receipt_sha256,
            residual_group_count,
        } => {
            fields.push((12, residual_group_count.to_be_bytes().to_vec()));
            fields.push((11, watchdog_receipt_sha256.to_vec()));
            fields.push((10, producer_receipt_sha256.to_vec()));
            fields.push((9, variable_basis_receipt_sha256.to_vec()));
            fields.push((8, b"applicable".to_vec()));
        }
        BuckinghamPiDisposition::SemanticallyInapplicable {
            producer_receipt_sha256,
            watchdog_receipt_sha256,
        } => {
            fields.push((11, watchdog_receipt_sha256.to_vec()));
            fields.push((10, producer_receipt_sha256.to_vec()));
            fields.push((8, b"semantically_inapplicable".to_vec()));
        }
    }
    encode_tagged_fields_reverse(fields)
}

fn encode_tagged_fields_reverse(mut fields: Vec<(u16, Vec<u8>)>) -> Vec<u8> {
    fields.sort_unstable_by_key(|(tag, _)| *tag);
    let mut bytes = Vec::new();
    for (tag, value) in fields {
        bytes.extend_from_slice(&tag.to_be_bytes());
        bytes.extend_from_slice(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_be_bytes());
        bytes.extend_from_slice(&value);
    }
    bytes
}

fn validate_request_reverse(request: &TheoryProfileAdmissionRequest) -> Result<(), &'static str> {
    let fixed = [
        request.validity_receipt_sha256,
        request.applicability_receipt_sha256,
        request.source_custody_sha256,
        request.profile_input_sha256,
        request.content_identity,
        request.role_identity,
        request.claim_identity,
    ];
    let fixed_set = fixed.into_iter().collect::<BTreeSet<_>>();
    if fixed_set.len() != fixed.len()
        || fixed_set
            .iter()
            .any(|digest| digest.iter().all(|byte| *byte == 0))
        || !valid_canonical_text(&request.owner_admission_record)
        || !valid_canonical_text(&request.residual_slot_id)
        || request.occupied_profile_slots.len() > MAX_OCCUPIED_PROFILE_SLOTS
    {
        return Err("invalid_theory_profile_admission_request");
    }
    let mut occupied = request
        .occupied_profile_slots
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    occupied.sort_unstable();
    if occupied
        .iter()
        .rev()
        .any(|slot| !valid_canonical_text(slot))
        || occupied.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err("invalid_theory_profile_admission_request");
    }
    Ok(())
}

fn receipts_valid_reverse(receipts: profile_protocol::ProfileProtocolCheckerReceipts) -> bool {
    let values = [
        receipts.owner_admission_sha256,
        receipts.residual_slot_sha256,
        receipts.residual_law_sha256,
        receipts.chaos_protocol_sha256,
        receipts.gap_law_sha256,
        receipts.buckingham_pi_sha256,
        receipts.coverage_sha256,
    ];
    let set = values.into_iter().collect::<BTreeSet<_>>();
    set.len() == values.len()
        && set
            .iter()
            .all(|digest| digest.iter().any(|byte| *byte != 0))
}

fn request_target(request: &TheoryProfileAdmissionRequest) -> PremiseKey {
    PremiseKey {
        role: SemanticRoleIdentity(request.role_identity),
        content: PhysicalContentIdentity(request.content_identity),
    }
}

fn valid_text(value: &str) -> bool {
    (1..=192).contains(&value.len()) && value.is_ascii()
}

fn valid_canonical_text(value: &str) -> bool {
    valid_text(value) && value == value.trim()
}
