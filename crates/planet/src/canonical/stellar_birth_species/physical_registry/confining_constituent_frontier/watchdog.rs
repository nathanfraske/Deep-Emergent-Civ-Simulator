//! Reverse obligation reconstruction for the confining constituent frontier.

use super::{
    encode_evaluation, encode_input,
    model::{
        ConfiningConstituentCheckerOutput, ConfiningConstituentDecision,
        ConfiningConstituentEvaluation, ConfiningConstituentInput, ConfiningConstituentObligation,
        ConfiningConstituentRefusalCode, INPUT_SCHEMA_ID, MAX_CANONICAL_BYTES,
        MAX_INTERNAL_SEED_COUNT, MAX_WORK_UNITS, WATCHDOG_ID,
    },
    resource_contract_sha256, tagged_digest,
};
use civsim_units::digest::sha256;
use std::collections::BTreeMap;

const REQUIRED_BY_REVERSE_RECONSTRUCTION: [ConfiningConstituentObligation; 8] = [
    ConfiningConstituentObligation::TransitionSeparationChannelCoverage,
    ConfiningConstituentObligation::ConstituentConservationBinding,
    ConfiningConstituentObligation::ConstituentApplicabilityValidity,
    ConfiningConstituentObligation::ChargeStateStatisticsDispositionCoverage,
    ConfiningConstituentObligation::ExactRestMassOrMasslessProof,
    ConfiningConstituentObligation::ExcitationProfileCoverage,
    ConfiningConstituentObligation::ConfiningDynamicsScaleOrEquivalent,
    ConfiningConstituentObligation::RepresentationFamilyMembershipClosure,
];

pub(super) fn inspect(
    input: &ConfiningConstituentInput,
) -> Result<ConfiningConstituentCheckerOutput, ConfiningConstituentRefusalCode> {
    if input.resources.max_internal_seed_count != MAX_INTERNAL_SEED_COUNT
        || input.resources.max_work_units != MAX_WORK_UNITS
        || input.resources.max_canonical_bytes != MAX_CANONICAL_BYTES
    {
        return Err(ConfiningConstituentRefusalCode::ResourceContractMismatch);
    }
    if input.schema_id.as_bytes() != INPUT_SCHEMA_ID.as_bytes() {
        return Err(ConfiningConstituentRefusalCode::InputSchemaMismatch);
    }
    if input.source_profile_claim_id.as_bytes() != super::super::strong_profile::CLAIM_ID.as_bytes()
    {
        return Err(ConfiningConstituentRefusalCode::SourceProfileMismatch);
    }
    if input
        .source_profile_receipt_sha256
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(ConfiningConstituentRefusalCode::InvalidSourceBinding);
    }
    let mut source_bindings = BTreeMap::new();
    for (key, identity) in [
        (0_u8, input.profile_root_identity),
        (1, input.sector_identity),
        (2, input.carrier_identity),
        (3, input.constraint_law_identity),
    ]
    .into_iter()
    .rev()
    {
        if identity.0.iter().all(|byte| *byte == 0) {
            return Err(ConfiningConstituentRefusalCode::InvalidSourceBinding);
        }
        source_bindings.insert(key, identity);
    }
    let mut identity_owners = BTreeMap::new();
    for (key, identity) in &source_bindings {
        if identity_owners.insert(*identity, *key).is_some() {
            return Err(ConfiningConstituentRefusalCode::IdentityAlias);
        }
    }

    let seed_count = input.internal_seeds.len();
    if seed_count == 0 {
        return Err(ConfiningConstituentRefusalCode::EmptyInternalSeedSet);
    }
    if seed_count > MAX_INTERNAL_SEED_COUNT as usize {
        return Err(ConfiningConstituentRefusalCode::InternalSeedCapacityExceeded);
    }
    let work_units = seed_count
        .checked_add(REQUIRED_BY_REVERSE_RECONSTRUCTION.len())
        .and_then(|work| u64::try_from(work).ok())
        .ok_or(ConfiningConstituentRefusalCode::WorkLimitExceeded)?;
    if work_units > MAX_WORK_UNITS {
        return Err(ConfiningConstituentRefusalCode::WorkLimitExceeded);
    }

    let mut seeds_by_identity = BTreeMap::new();
    for seed in input.internal_seeds.iter().rev() {
        if seed.identity.0.iter().all(|byte| *byte == 0)
            || seed.source_profile_receipt_sha256 != input.source_profile_receipt_sha256
        {
            return Err(ConfiningConstituentRefusalCode::InternalSeedBindingMismatch);
        }
        if seed.identity != input.carrier_identity && identity_owners.contains_key(&seed.identity) {
            return Err(ConfiningConstituentRefusalCode::IdentityAlias);
        }
        if seeds_by_identity.insert(seed.identity, *seed).is_some() {
            return Err(ConfiningConstituentRefusalCode::DuplicateInternalSeed);
        }
    }
    if !seeds_by_identity.contains_key(&input.carrier_identity) {
        return Err(ConfiningConstituentRefusalCode::CarrierSeedMissing);
    }

    let mut missing = REQUIRED_BY_REVERSE_RECONSTRUCTION.to_vec();
    missing.sort_unstable_by_key(|obligation| obligation.ordinal());
    let decision = ConfiningConstituentDecision::BlockedOpenProofs { missing };
    let evaluation = ConfiningConstituentEvaluation {
        input_sha256: sha256(&encode_input(input)),
        source_profile_receipt_sha256: input.source_profile_receipt_sha256,
        profile_root_identity: source_bindings[&0],
        sector_identity: source_bindings[&1],
        carrier_identity: source_bindings[&2],
        constraint_law_identity: source_bindings[&3],
        internal_seed_identities: seeds_by_identity.keys().copied().collect(),
        decision,
        constituent_candidate_count: 0,
        membership_authority: false,
        spectrum_authority: false,
        authority_effect: "none",
    };
    let canonical_bytes = encode_evaluation(&evaluation);
    if canonical_bytes.len() > MAX_CANONICAL_BYTES as usize {
        return Err(ConfiningConstituentRefusalCode::CanonicalBytesExceeded);
    }
    let trace_sha256 = tagged_digest(
        b"civsim.confining-constituent-frontier.watchdog-trace.v1",
        &[
            WATCHDOG_ID.as_bytes(),
            &u64::try_from(evaluation.decision.missing().len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
            &u64::try_from(REQUIRED_BY_REVERSE_RECONSTRUCTION.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
            &u64::try_from(seeds_by_identity.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
            &evaluation.input_sha256,
        ],
    );
    Ok(ConfiningConstituentCheckerOutput {
        evaluation,
        canonical_bytes,
        resource_contract_sha256: resource_contract_sha256(input.resources),
        trace_sha256,
    })
}
