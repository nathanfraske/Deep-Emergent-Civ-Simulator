//! Non-authorizing frontier for confining-sector constituent inputs.
//!
//! The admitted confining profile supplies one internal carrier-content seed.
//! It does not supply a closed representation family, a dynamics scale,
//! constituent excitation properties, or complete transition channels. Two
//! independent bounded implementations turn that exact state into a typed
//! open-frontier decision.
//!
//! This diagnostic never adds an admitted artifact, constructs a constituent
//! candidate, mints registry membership, or authorizes a bound-state spectrum.
//! Familiar, unfamiliar, pure-gauge, and thaumic sectors use the same opaque
//! identity and variable-cardinality input shape.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

pub(super) use model::ConfiningConstituentFrontierReceipt;

use self::model::{
    ConfiningConstituentCheckerOutput, ConfiningConstituentInput, ConfiningConstituentRefusalCode,
    ConfiningConstituentResourceContract, InternalConstituentSeed, CLAIM_ID, DIAGNOSTIC_SCHEMA_ID,
    INPUT_SCHEMA_ID, PRODUCER_ID, WATCHDOG_ID,
};
use super::strong_profile::StrongProfileReceipt;
use civsim_units::digest::sha256;

fn repository_input(
    strong_profile: &StrongProfileReceipt,
) -> Result<ConfiningConstituentInput, ConfiningConstituentRefusalCode> {
    if strong_profile.schema_id != super::strong_profile::RECEIPT_SCHEMA_ID
        || strong_profile.claim_id != super::strong_profile::CLAIM_ID
        || strong_profile.profile_id != super::strong_profile::PROFILE_ID
        || strong_profile.pair_receipt_sha256 == [0; 32]
        || strong_profile.profile_root_identity.0 == [0; 32]
        || strong_profile.sector_identity.0 == [0; 32]
        || strong_profile.carrier_identity.0 == [0; 32]
        || strong_profile.constraint_law_identity.0 == [0; 32]
        || strong_profile.member_count != 0
        || strong_profile.membership_authority
        || strong_profile.carrier_species_membership
        || strong_profile.conditioned_support_authority
        || strong_profile.confinement_theorem_claim
        || !strong_profile.confining_asymptotic_boundary_admitted
        || strong_profile.authority_effect != "none"
    {
        return Err(ConfiningConstituentRefusalCode::SourceProfileMismatch);
    }
    Ok(ConfiningConstituentInput {
        schema_id: INPUT_SCHEMA_ID,
        source_profile_claim_id: strong_profile.claim_id,
        source_profile_receipt_sha256: strong_profile.pair_receipt_sha256,
        profile_root_identity: strong_profile.profile_root_identity,
        sector_identity: strong_profile.sector_identity,
        carrier_identity: strong_profile.carrier_identity,
        constraint_law_identity: strong_profile.constraint_law_identity,
        internal_seeds: vec![InternalConstituentSeed {
            identity: strong_profile.carrier_identity,
            source_profile_receipt_sha256: strong_profile.pair_receipt_sha256,
        }],
        resources: ConfiningConstituentResourceContract::PRODUCTION,
    })
}

pub(super) fn repository_frontier(
    strong_profile: &StrongProfileReceipt,
) -> Result<ConfiningConstituentFrontierReceipt, ConfiningConstituentRefusalCode> {
    inspect(&repository_input(strong_profile)?)
}

fn inspect(
    input: &ConfiningConstituentInput,
) -> Result<ConfiningConstituentFrontierReceipt, ConfiningConstituentRefusalCode> {
    finish_pair(producer::inspect(input), watchdog::inspect(input))
}

fn finish_pair(
    produced: Result<ConfiningConstituentCheckerOutput, ConfiningConstituentRefusalCode>,
    watched: Result<ConfiningConstituentCheckerOutput, ConfiningConstituentRefusalCode>,
) -> Result<ConfiningConstituentFrontierReceipt, ConfiningConstituentRefusalCode> {
    match (produced, watched) {
        (Ok(produced), Ok(watched))
            if produced.evaluation == watched.evaluation
                && produced.canonical_bytes == watched.canonical_bytes
                && produced.resource_contract_sha256 == watched.resource_contract_sha256
                && produced.trace_sha256 != [0; 32]
                && watched.trace_sha256 != [0; 32]
                && produced.trace_sha256 != watched.trace_sha256 =>
        {
            let evaluation = produced.evaluation;
            let producer_result_sha256 = tagged_digest(
                b"civsim.confining-constituent-frontier.producer-result.v1",
                &[PRODUCER_ID.as_bytes(), &produced.canonical_bytes],
            );
            let watchdog_result_sha256 = tagged_digest(
                b"civsim.confining-constituent-frontier.watchdog-result.v1",
                &[WATCHDOG_ID.as_bytes(), &watched.canonical_bytes],
            );
            if producer_result_sha256 == watchdog_result_sha256 {
                return Err(ConfiningConstituentRefusalCode::CheckerDisagreement);
            }
            let missing_authority_ids = evaluation
                .decision
                .missing()
                .iter()
                .map(|obligation| obligation.id())
                .collect::<Vec<_>>();
            let receipt_sha256 = diagnostic_receipt_sha256(
                &evaluation,
                producer_result_sha256,
                watchdog_result_sha256,
                produced.trace_sha256,
                watched.trace_sha256,
                produced.resource_contract_sha256,
                watched.resource_contract_sha256,
            );
            if receipt_sha256 == [0; 32] {
                return Err(ConfiningConstituentRefusalCode::AgreementReceiptMismatch);
            }
            Ok(ConfiningConstituentFrontierReceipt {
                schema_id: DIAGNOSTIC_SCHEMA_ID,
                claim_id: CLAIM_ID,
                producer_id: PRODUCER_ID,
                watchdog_id: WATCHDOG_ID,
                decision_id: evaluation.decision.id(),
                source_profile_receipt_sha256: evaluation.source_profile_receipt_sha256,
                internal_seed_identities: evaluation.internal_seed_identities,
                missing_authority_ids,
                constituent_candidate_count: evaluation.constituent_candidate_count,
                membership_authority: evaluation.membership_authority,
                spectrum_authority: evaluation.spectrum_authority,
                authority_effect: evaluation.authority_effect,
                producer_result_sha256,
                watchdog_result_sha256,
                producer_trace_sha256: produced.trace_sha256,
                watchdog_trace_sha256: watched.trace_sha256,
                producer_resource_sha256: produced.resource_contract_sha256,
                watchdog_resource_sha256: watched.resource_contract_sha256,
                receipt_sha256,
            })
        }
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(ConfiningConstituentRefusalCode::CheckerDisagreement),
    }
}

fn diagnostic_receipt_sha256(
    evaluation: &model::ConfiningConstituentEvaluation,
    producer_result_sha256: [u8; 32],
    watchdog_result_sha256: [u8; 32],
    producer_trace_sha256: [u8; 32],
    watchdog_trace_sha256: [u8; 32],
    producer_resource_sha256: [u8; 32],
    watchdog_resource_sha256: [u8; 32],
) -> [u8; 32] {
    let evaluation_sha256 = sha256(&encode_evaluation(evaluation));
    tagged_digest(
        b"civsim.confining-constituent-frontier.diagnostic-receipt.v1",
        &[
            DIAGNOSTIC_SCHEMA_ID.as_bytes(),
            CLAIM_ID.as_bytes(),
            evaluation.decision.id().as_bytes(),
            &evaluation.input_sha256,
            &evaluation.source_profile_receipt_sha256,
            &evaluation_sha256,
            &producer_result_sha256,
            &watchdog_result_sha256,
            &producer_trace_sha256,
            &watchdog_trace_sha256,
            &producer_resource_sha256,
            &watchdog_resource_sha256,
        ],
    )
}

fn encode_input(input: &ConfiningConstituentInput) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, INPUT_SCHEMA_ID.as_bytes());
    append_field(&mut bytes, 2, input.source_profile_claim_id.as_bytes());
    append_field(&mut bytes, 3, &input.source_profile_receipt_sha256);
    append_field(&mut bytes, 4, &input.profile_root_identity.0);
    append_field(&mut bytes, 5, &input.sector_identity.0);
    append_field(&mut bytes, 6, &input.carrier_identity.0);
    append_field(&mut bytes, 7, &input.constraint_law_identity.0);
    let mut seeds = input.internal_seeds.clone();
    seeds.sort_unstable();
    for seed in seeds {
        append_field(&mut bytes, 8, &seed.identity.0);
        append_field(&mut bytes, 9, &seed.source_profile_receipt_sha256);
    }
    append_field(
        &mut bytes,
        10,
        &input.resources.max_internal_seed_count.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        11,
        &input.resources.max_work_units.to_be_bytes(),
    );
    append_field(
        &mut bytes,
        12,
        &input.resources.max_canonical_bytes.to_be_bytes(),
    );
    bytes
}

fn encode_evaluation(evaluation: &model::ConfiningConstituentEvaluation) -> Vec<u8> {
    let mut bytes = Vec::new();
    append_field(
        &mut bytes,
        1,
        b"civsim.planet.confining-constituent-frontier-evaluation.v1",
    );
    append_field(&mut bytes, 2, &evaluation.input_sha256);
    append_field(&mut bytes, 3, &evaluation.source_profile_receipt_sha256);
    append_field(&mut bytes, 4, &evaluation.profile_root_identity.0);
    append_field(&mut bytes, 5, &evaluation.sector_identity.0);
    append_field(&mut bytes, 6, &evaluation.carrier_identity.0);
    append_field(&mut bytes, 7, &evaluation.constraint_law_identity.0);
    for identity in &evaluation.internal_seed_identities {
        append_field(&mut bytes, 8, &identity.0);
    }
    append_field(&mut bytes, 9, evaluation.decision.id().as_bytes());
    for missing in evaluation.decision.missing() {
        append_field(&mut bytes, 10, missing.id().as_bytes());
    }
    append_field(
        &mut bytes,
        11,
        &evaluation.constituent_candidate_count.to_be_bytes(),
    );
    append_field(&mut bytes, 12, &[u8::from(evaluation.membership_authority)]);
    append_field(&mut bytes, 13, &[u8::from(evaluation.spectrum_authority)]);
    append_field(&mut bytes, 14, evaluation.authority_effect.as_bytes());
    bytes
}

fn resource_contract_sha256(resources: model::ConfiningConstituentResourceContract) -> [u8; 32] {
    tagged_digest(
        b"civsim.confining-constituent-frontier.resources.v1",
        &[
            &resources.max_internal_seed_count.to_be_bytes(),
            &resources.max_work_units.to_be_bytes(),
            &resources.max_canonical_bytes.to_be_bytes(),
        ],
    )
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

fn tagged_digest(domain: &[u8], fields: &[&[u8]]) -> [u8; 32] {
    let mut bytes = Vec::new();
    append_field(&mut bytes, 1, domain);
    for field in fields {
        append_field(&mut bytes, 2, field);
    }
    sha256(&bytes)
}
