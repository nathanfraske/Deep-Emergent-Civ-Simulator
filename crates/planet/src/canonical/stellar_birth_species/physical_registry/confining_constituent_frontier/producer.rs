//! Forward inventory for the confining constituent input frontier.

use super::{
    encode_evaluation, encode_input,
    model::{
        ConfiningConstituentCheckerOutput, ConfiningConstituentDecision,
        ConfiningConstituentEvaluation, ConfiningConstituentInput, ConfiningConstituentObligation,
        ConfiningConstituentRefusalCode, ConfiningConstituentResourceContract, INPUT_SCHEMA_ID,
        MAX_CANONICAL_BYTES, MAX_INTERNAL_SEED_COUNT, MAX_WORK_UNITS, PRODUCER_ID,
    },
    resource_contract_sha256, tagged_digest,
};
use civsim_units::digest::sha256;
use std::collections::BTreeSet;

const REQUIRED_AUTHORITIES: [ConfiningConstituentObligation; 8] = [
    ConfiningConstituentObligation::RepresentationFamilyMembershipClosure,
    ConfiningConstituentObligation::ConfiningDynamicsScaleOrEquivalent,
    ConfiningConstituentObligation::ExcitationProfileCoverage,
    ConfiningConstituentObligation::ExactRestMassOrMasslessProof,
    ConfiningConstituentObligation::ChargeStateStatisticsDispositionCoverage,
    ConfiningConstituentObligation::ConstituentApplicabilityValidity,
    ConfiningConstituentObligation::ConstituentConservationBinding,
    ConfiningConstituentObligation::TransitionSeparationChannelCoverage,
];

pub(super) fn inspect(
    input: &ConfiningConstituentInput,
) -> Result<ConfiningConstituentCheckerOutput, ConfiningConstituentRefusalCode> {
    if input.resources != ConfiningConstituentResourceContract::PRODUCTION {
        return Err(ConfiningConstituentRefusalCode::ResourceContractMismatch);
    }
    if input.schema_id != INPUT_SCHEMA_ID {
        return Err(ConfiningConstituentRefusalCode::InputSchemaMismatch);
    }
    if input.source_profile_claim_id != super::super::strong_profile::CLAIM_ID {
        return Err(ConfiningConstituentRefusalCode::SourceProfileMismatch);
    }
    let binding_identities = [
        input.profile_root_identity,
        input.sector_identity,
        input.carrier_identity,
        input.constraint_law_identity,
    ];
    if input.source_profile_receipt_sha256 == [0; 32]
        || binding_identities
            .iter()
            .any(|identity| identity.0 == [0; 32])
    {
        return Err(ConfiningConstituentRefusalCode::InvalidSourceBinding);
    }
    if binding_identities
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != binding_identities.len()
    {
        return Err(ConfiningConstituentRefusalCode::IdentityAlias);
    }

    let seed_count = u32::try_from(input.internal_seeds.len())
        .map_err(|_| ConfiningConstituentRefusalCode::InternalSeedCapacityExceeded)?;
    if seed_count == 0 {
        return Err(ConfiningConstituentRefusalCode::EmptyInternalSeedSet);
    }
    if seed_count > MAX_INTERNAL_SEED_COUNT {
        return Err(ConfiningConstituentRefusalCode::InternalSeedCapacityExceeded);
    }
    let work_units = u64::from(seed_count)
        .checked_add(REQUIRED_AUTHORITIES.len() as u64)
        .ok_or(ConfiningConstituentRefusalCode::WorkLimitExceeded)?;
    if work_units > MAX_WORK_UNITS {
        return Err(ConfiningConstituentRefusalCode::WorkLimitExceeded);
    }

    let mut internal_seeds = input.internal_seeds.clone();
    internal_seeds.sort_unstable();
    if internal_seeds.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ConfiningConstituentRefusalCode::DuplicateInternalSeed);
    }
    if internal_seeds.iter().any(|seed| {
        seed.identity.0 == [0; 32]
            || seed.source_profile_receipt_sha256 != input.source_profile_receipt_sha256
    }) {
        return Err(ConfiningConstituentRefusalCode::InternalSeedBindingMismatch);
    }
    if !internal_seeds
        .iter()
        .any(|seed| seed.identity == input.carrier_identity)
    {
        return Err(ConfiningConstituentRefusalCode::CarrierSeedMissing);
    }
    let reserved_non_seed_identities = [
        input.profile_root_identity,
        input.sector_identity,
        input.constraint_law_identity,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    if internal_seeds
        .iter()
        .any(|seed| reserved_non_seed_identities.contains(&seed.identity))
    {
        return Err(ConfiningConstituentRefusalCode::IdentityAlias);
    }

    let decision = ConfiningConstituentDecision::BlockedOpenProofs {
        missing: REQUIRED_AUTHORITIES.into_iter().collect(),
    };
    let evaluation = ConfiningConstituentEvaluation {
        input_sha256: sha256(&encode_input(input)),
        source_profile_receipt_sha256: input.source_profile_receipt_sha256,
        profile_root_identity: input.profile_root_identity,
        sector_identity: input.sector_identity,
        carrier_identity: input.carrier_identity,
        constraint_law_identity: input.constraint_law_identity,
        internal_seed_identities: internal_seeds.iter().map(|seed| seed.identity).collect(),
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
        b"civsim.confining-constituent-frontier.producer-trace.v1",
        &[
            &evaluation.input_sha256,
            &u64::from(seed_count).to_be_bytes(),
            &u64::try_from(REQUIRED_AUTHORITIES.len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
            &u64::try_from(evaluation.decision.missing().len())
                .unwrap_or(u64::MAX)
                .to_be_bytes(),
            PRODUCER_ID.as_bytes(),
        ],
    );
    Ok(ConfiningConstituentCheckerOutput {
        evaluation,
        canonical_bytes,
        resource_contract_sha256: resource_contract_sha256(input.resources),
        trace_sha256,
    })
}
