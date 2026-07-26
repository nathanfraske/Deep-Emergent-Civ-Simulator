//! Independent projection of sealed physical invariants into registry roots.
//!
//! This module can create only scalar coordinates and membership-neutral
//! identity mass projections. It has no constructor for a member-forming rule.

mod model;
mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) use model::RepositoryRootSource;
pub(super) use model::{
    RepositoryRootDecision, RepositoryRootProjection, RepositoryRootProjectionReceipt,
    RepositoryRootRefusal, RepositoryRootRefusalCode,
};

use super::model::{
    AdmissionRoute, AdmittedArtifact, DerivedAdmission, ProvenanceMark, ReceiptBinding,
    RootAdmission,
};
use civsim_units::digest::sha256;
use model::{
    RepositoryRootCanaryBinding, RepositoryRootCheckerOutcome, RepositoryRootPacket,
    RepositoryRootRefusalReceipt, RepositoryRootRefusalStage,
    RepositoryRootResourceContractOutcome, RootCandidate, RootCheckerOutput,
    REPOSITORY_ROOT_ANCESTRY_SCHEMA_ID, REPOSITORY_ROOT_CANARY_SUITE_ID, REPOSITORY_ROOT_CLAIM_ID,
    REPOSITORY_ROOT_PRODUCER_ID, REPOSITORY_ROOT_PRODUCER_RECEIPT_SCHEMA_ID,
    REPOSITORY_ROOT_PROJECTION_SCHEMA_ID, REPOSITORY_ROOT_RECEIPT_SCHEMA_ID,
    REPOSITORY_ROOT_REFUSAL_RECEIPT_SCHEMA_ID, REPOSITORY_ROOT_WATCHDOG_ID,
    REPOSITORY_ROOT_WATCHDOG_RECEIPT_SCHEMA_ID,
};

const CHECKER_CLAIM_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.checker-claim.v1";
const PAIR_RECEIPT_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.pair-receipt.v5";
const REFUSAL_RECEIPT_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.refusal-receipt.v3";
const PACKET_OBSERVATION_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.packet-observation.v3";

/// Opaque proof that one artifact was minted by the agreed repository-root
/// pair. Receipt-shaped values outside this module cannot construct it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RepositoryRootAdmissionCapability {
    claimed_identity: super::model::ArtifactIdentity,
    admission: RootAdmission,
    pair_receipt_sha256: [u8; 32],
}

impl RepositoryRootAdmissionCapability {
    pub(super) const fn claimed_identity(&self) -> super::model::ArtifactIdentity {
        self.claimed_identity
    }

    pub(super) const fn admission(&self) -> &RootAdmission {
        &self.admission
    }

    pub(super) const fn pair_receipt_sha256(&self) -> [u8; 32] {
        self.pair_receipt_sha256
    }
}

/// Independently extract the sealed floor on both paths, project it, exercise
/// the mutation suite, and create `[D]` admissions only after full agreement.
pub(super) fn project_repository_roots(
) -> Result<RepositoryRootProjection, RepositoryRootRefusalCode> {
    match decide_repository_roots() {
        RepositoryRootDecision::Projected(projection) => Ok(projection),
        RepositoryRootDecision::Refused(refusal) => {
            debug_assert!(verify_refusal(&refusal));
            Err(refusal.code)
        }
    }
}

/// Preserve the pair's claim-scoped refusal evidence instead of reducing every
/// failure to a bare code. The refusal variant has no root capability fields.
pub(super) fn decide_repository_roots() -> RepositoryRootDecision {
    decide_packet_pair(
        producer::sealed_floor_packet(),
        watchdog::sealed_floor_packet(),
        resource_outcome(producer::resource_contract_sha256()),
        resource_outcome(watchdog::resource_contract_sha256()),
    )
}

#[cfg(test)]
fn inspect_unsealed_packet_for_test(
    packet: &RepositoryRootPacket,
) -> Result<RootCheckerOutput, RepositoryRootRefusalCode> {
    match (
        producer::project_unsealed(packet),
        watchdog::project_unsealed(packet),
    ) {
        (Ok(produced), Ok(watched)) if produced == watched => Ok(produced),
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(RepositoryRootRefusalCode::CheckerDisagreement),
    }
}

#[cfg(test)]
fn refusal_for_invalid_packet_for_test(packet: &RepositoryRootPacket) -> RepositoryRootRefusal {
    match decide_packet_pair(
        Ok(packet.clone()),
        Ok(packet.clone()),
        resource_outcome(producer::resource_contract_sha256()),
        resource_outcome(watchdog::resource_contract_sha256()),
    ) {
        RepositoryRootDecision::Refused(refusal) => refusal,
        RepositoryRootDecision::Projected(_) => {
            panic!("an invalid test packet must not mint a root projection")
        }
    }
}

fn decide_packet_pair(
    producer_packet: Result<RepositoryRootPacket, RepositoryRootRefusalCode>,
    watchdog_packet: Result<RepositoryRootPacket, RepositoryRootRefusalCode>,
    producer_resource_contract: RepositoryRootResourceContractOutcome,
    watchdog_resource_contract: RepositoryRootResourceContractOutcome,
) -> RepositoryRootDecision {
    let producer_packet_sha256 = producer_packet
        .as_ref()
        .map_or([0; 32], packet_observation_sha256);
    let watchdog_packet_sha256 = watchdog_packet
        .as_ref()
        .map_or([0; 32], packet_observation_sha256);

    let (producer_resource_sha256, watchdog_resource_sha256) =
        match matching_resource_digests(&producer_resource_contract, &watchdog_resource_contract) {
            Some(digests) => digests,
            None => {
                let producer_outcome = extraction_outcome(
                    producer_packet,
                    RepositoryRootRefusalCode::ResourceContractDisagreement,
                );
                let watchdog_outcome = extraction_outcome(
                    watchdog_packet,
                    RepositoryRootRefusalCode::ResourceContractDisagreement,
                );
                return refused_decision(
                    RepositoryRootRefusalCode::ResourceContractDisagreement,
                    RepositoryRootRefusalStage::ResourceContractPreflight,
                    producer_packet_sha256,
                    watchdog_packet_sha256,
                    producer_outcome,
                    watchdog_outcome,
                    producer_resource_contract,
                    watchdog_resource_contract,
                    None,
                );
            }
        };

    let (producer_packet, watchdog_packet) = match (producer_packet, watchdog_packet) {
        (Ok(producer_packet), Ok(watchdog_packet)) => (producer_packet, watchdog_packet),
        (producer_packet, watchdog_packet) => {
            let refusal_code = match (&producer_packet, &watchdog_packet) {
                (Err(producer_code), Err(watchdog_code)) if producer_code == watchdog_code => {
                    *producer_code
                }
                _ => RepositoryRootRefusalCode::CheckerDisagreement,
            };
            let producer_outcome = extraction_outcome(
                producer_packet,
                RepositoryRootRefusalCode::CheckerDisagreement,
            );
            let watchdog_outcome = extraction_outcome(
                watchdog_packet,
                RepositoryRootRefusalCode::CheckerDisagreement,
            );
            return refused_decision(
                refusal_code,
                RepositoryRootRefusalStage::PacketExtraction,
                producer_packet_sha256,
                watchdog_packet_sha256,
                producer_outcome,
                watchdog_outcome,
                producer_resource_contract,
                watchdog_resource_contract,
                None,
            );
        }
    };

    if producer_packet != watchdog_packet {
        return refused_decision(
            RepositoryRootRefusalCode::SealedFloorPacketMismatch,
            RepositoryRootRefusalStage::PacketAgreement,
            producer_packet_sha256,
            watchdog_packet_sha256,
            RepositoryRootCheckerOutcome::NotRun {
                reason: RepositoryRootRefusalCode::SealedFloorPacketMismatch,
            },
            RepositoryRootCheckerOutcome::NotRun {
                reason: RepositoryRootRefusalCode::SealedFloorPacketMismatch,
            },
            producer_resource_contract,
            watchdog_resource_contract,
            None,
        );
    }

    let produced = producer::project(&producer_packet);
    let watched = watchdog::project(&watchdog_packet);
    let producer_outcome = checker_outcome(&produced);
    let watchdog_outcome = checker_outcome(&watched);

    let (produced, watched) = match (produced, watched) {
        (Ok(produced), Ok(watched)) if produced == watched => (produced, watched),
        (Err(producer_code), Err(watchdog_code)) if producer_code == watchdog_code => {
            return refused_decision(
                producer_code,
                RepositoryRootRefusalStage::CheckerAgreement,
                producer_packet_sha256,
                watchdog_packet_sha256,
                producer_outcome,
                watchdog_outcome,
                producer_resource_contract,
                watchdog_resource_contract,
                None,
            );
        }
        _ => {
            return refused_decision(
                RepositoryRootRefusalCode::CheckerDisagreement,
                RepositoryRootRefusalStage::CheckerAgreement,
                producer_packet_sha256,
                watchdog_packet_sha256,
                producer_outcome,
                watchdog_outcome,
                producer_resource_contract,
                watchdog_resource_contract,
                None,
            );
        }
    };

    let canary_sha256 = match (
        producer::canary_digest(&producer_packet, &produced),
        watchdog::canary_digest(&watchdog_packet, &watched),
    ) {
        (Ok(producer_digest), Ok(watchdog_digest)) if producer_digest == watchdog_digest => {
            producer_digest
        }
        _ => {
            return refused_decision(
                RepositoryRootRefusalCode::CanaryFailure,
                RepositoryRootRefusalStage::Canary,
                producer_packet_sha256,
                watchdog_packet_sha256,
                producer_outcome,
                watchdog_outcome,
                producer_resource_contract,
                watchdog_resource_contract,
                None,
            );
        }
    };

    let projection = match finish_agreed_projection(
        produced,
        watched,
        canary_sha256,
        producer_resource_sha256,
        watchdog_resource_sha256,
    ) {
        Ok(projection) => projection,
        Err(code) => {
            let (code, refusal_stage) = projection_finalization_refusal(code);
            return refused_decision(
                code,
                refusal_stage,
                producer_packet_sha256,
                watchdog_packet_sha256,
                producer_outcome,
                watchdog_outcome,
                producer_resource_contract,
                watchdog_resource_contract,
                Some(canary_sha256),
            );
        }
    };
    RepositoryRootDecision::Projected(projection)
}

fn projection_finalization_refusal(
    code: RepositoryRootRefusalCode,
) -> (RepositoryRootRefusalCode, RepositoryRootRefusalStage) {
    (code, RepositoryRootRefusalStage::ProjectionReceipt)
}

fn extraction_outcome(
    packet: Result<RepositoryRootPacket, RepositoryRootRefusalCode>,
    reason: RepositoryRootRefusalCode,
) -> RepositoryRootCheckerOutcome {
    match packet {
        Ok(_) => RepositoryRootCheckerOutcome::NotRun { reason },
        Err(code) => RepositoryRootCheckerOutcome::Refused { code },
    }
}

fn checker_outcome(
    result: &Result<RootCheckerOutput, RepositoryRootRefusalCode>,
) -> RepositoryRootCheckerOutcome {
    match result {
        Ok(output) => RepositoryRootCheckerOutcome::Projected {
            input_sha256: output.input_sha256,
            result_sha256: sha256(&output.canonical_bytes),
        },
        Err(code) => RepositoryRootCheckerOutcome::Refused { code: *code },
    }
}

fn resource_outcome(
    result: Result<[u8; 32], RepositoryRootRefusalCode>,
) -> RepositoryRootResourceContractOutcome {
    match result {
        Ok(digest_sha256) => RepositoryRootResourceContractOutcome::Bound { digest_sha256 },
        Err(code) => RepositoryRootResourceContractOutcome::Refused { code },
    }
}

fn matching_resource_digests(
    producer: &RepositoryRootResourceContractOutcome,
    watchdog: &RepositoryRootResourceContractOutcome,
) -> Option<([u8; 32], [u8; 32])> {
    match (producer, watchdog) {
        (
            RepositoryRootResourceContractOutcome::Bound {
                digest_sha256: producer_digest,
            },
            RepositoryRootResourceContractOutcome::Bound {
                digest_sha256: watchdog_digest,
            },
        ) if producer_digest == watchdog_digest => Some((*producer_digest, *watchdog_digest)),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn refused_decision(
    refusal_code: RepositoryRootRefusalCode,
    refusal_stage: RepositoryRootRefusalStage,
    producer_packet_sha256: [u8; 32],
    watchdog_packet_sha256: [u8; 32],
    producer_outcome: RepositoryRootCheckerOutcome,
    watchdog_outcome: RepositoryRootCheckerOutcome,
    producer_resource_contract: RepositoryRootResourceContractOutcome,
    watchdog_resource_contract: RepositoryRootResourceContractOutcome,
    canary_sha256: Option<[u8; 32]>,
) -> RepositoryRootDecision {
    let mut receipt = RepositoryRootRefusalReceipt {
        schema_id: REPOSITORY_ROOT_REFUSAL_RECEIPT_SCHEMA_ID,
        claim_id: REPOSITORY_ROOT_CLAIM_ID,
        producer_id: REPOSITORY_ROOT_PRODUCER_ID,
        watchdog_id: REPOSITORY_ROOT_WATCHDOG_ID,
        producer_packet_sha256,
        watchdog_packet_sha256,
        producer_outcome,
        watchdog_outcome,
        producer_resource_contract,
        watchdog_resource_contract,
        canary: RepositoryRootCanaryBinding {
            suite_id: REPOSITORY_ROOT_CANARY_SUITE_ID,
            digest_sha256: canary_sha256,
        },
        refusal_stage,
        refusal_code,
        decision_id: "refused",
        membership_authority: false,
        receipt_sha256: [0; 32],
    };
    receipt.receipt_sha256 = refusal_receipt_digest(&receipt);
    debug_assert!(verify_refusal_receipt(&receipt));
    RepositoryRootDecision::Refused(RepositoryRootRefusal {
        code: refusal_code,
        receipt,
    })
}

fn packet_observation_sha256(packet: &RepositoryRootPacket) -> [u8; 32] {
    let mut bytes = PACKET_OBSERVATION_DOMAIN.to_vec();
    append_field(&mut bytes, 1, packet.schema_id.as_bytes());
    append_field(&mut bytes, 2, packet.floor_authority.schema_id.as_bytes());
    append_field(&mut bytes, 3, &packet.floor_authority.digest_sha256);
    append_field(&mut bytes, 4, &[u8::from(packet.membership_authority)]);
    append_field(
        &mut bytes,
        5,
        &u64::try_from(packet.sources.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    for source in &packet.sources {
        let mut source_bytes = Vec::new();
        append_field(&mut source_bytes, 1, source.role.id().as_bytes());
        append_field(&mut source_bytes, 2, source.entry_id.as_bytes());
        append_field(&mut source_bytes, 3, source.symbol.as_bytes());
        append_field(&mut source_bytes, 4, source.central_decimal.as_bytes());
        append_field(&mut source_bytes, 5, source.uncertainty_kind.as_bytes());
        append_field(&mut source_bytes, 6, source.uncertainty_decimal.as_bytes());
        let mut dimension = Vec::with_capacity(4 + source.dimension.terms().len() * 34);
        dimension.extend_from_slice(
            &u32::try_from(source.dimension.terms().len())
                .expect("bounded dimension term storage fits u32")
                .to_be_bytes(),
        );
        for term in source.dimension.terms() {
            dimension.extend_from_slice(&term.axis.0);
            dimension.extend_from_slice(&term.exponent.to_be_bytes());
        }
        append_field(&mut source_bytes, 7, &dimension);
        append_field(&mut source_bytes, 8, source.source_tier.id().as_bytes());
        append_field(
            &mut source_bytes,
            9,
            source.source_provenance.tag().as_bytes(),
        );
        append_field(
            &mut source_bytes,
            10,
            source.exhaustion_binding.schema_id.as_bytes(),
        );
        append_field(
            &mut source_bytes,
            11,
            &source.exhaustion_binding.digest_sha256,
        );
        append_field(&mut bytes, 6, &source_bytes);
    }
    sha256(&bytes)
}

fn finish_agreed_projection(
    produced: RootCheckerOutput,
    watched: RootCheckerOutput,
    canary_sha256: [u8; 32],
    producer_resource_contract_sha256: [u8; 32],
    watchdog_resource_contract_sha256: [u8; 32],
) -> Result<RepositoryRootProjection, RepositoryRootRefusalCode> {
    let producer_result_sha256 = sha256(&produced.canonical_bytes);
    let watchdog_result_sha256 = sha256(&watched.canonical_bytes);
    let producer_ancestry_manifest_sha256 =
        producer::ancestry_manifest_sha256(&produced.candidates)?;
    let watchdog_ancestry_manifest_sha256 =
        watchdog::ancestry_manifest_sha256(&watched.candidates)?;
    if producer_result_sha256 != watchdog_result_sha256
        || producer_ancestry_manifest_sha256 != watchdog_ancestry_manifest_sha256
        || producer_resource_contract_sha256 != watchdog_resource_contract_sha256
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }
    let mut receipt = RepositoryRootProjectionReceipt {
        schema_id: REPOSITORY_ROOT_RECEIPT_SCHEMA_ID,
        claim_id: REPOSITORY_ROOT_CLAIM_ID,
        input_sha256: produced.input_sha256,
        result_sha256: producer_result_sha256,
        producer_result_sha256,
        watchdog_result_sha256,
        producer_ancestry_manifest_sha256,
        watchdog_ancestry_manifest_sha256,
        producer_resource_contract_sha256,
        watchdog_resource_contract_sha256,
        canary_suite_id: REPOSITORY_ROOT_CANARY_SUITE_ID,
        canary_sha256,
        producer_id: REPOSITORY_ROOT_PRODUCER_ID,
        watchdog_id: REPOSITORY_ROOT_WATCHDOG_ID,
        scalar_coordinate_count: produced.scalar_coordinate_count,
        mass_projection_count: produced.mass_projection_count,
        membership_authority: false,
        decision_id: "agreed_projected",
        receipt_sha256: [0; 32],
    };
    receipt.receipt_sha256 = pair_receipt_digest(&receipt);
    let admitted_artifacts = produced
        .candidates
        .iter()
        .map(|candidate| admit_candidate(candidate, &receipt))
        .collect();
    let mut projection = RepositoryRootProjection {
        admitted_artifacts,
        canonical_bytes: produced.canonical_bytes,
        receipt,
        membership_authority: false,
        checker_candidates: produced.candidates,
        post_projection_canary_sha256: [0; 32],
    };
    let producer_post_projection_canary_sha256 =
        producer::post_projection_canary_digest(&projection)?;
    let watchdog_post_projection_canary_sha256 =
        watchdog::post_projection_canary_digest(&projection)?;
    if producer_post_projection_canary_sha256 != watchdog_post_projection_canary_sha256 {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }
    projection.post_projection_canary_sha256 = producer_post_projection_canary_sha256;
    if !verify_projection(&projection) {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }
    Ok(projection)
}

pub(in super::super) fn verify_projection_receipt(
    receipt: &RepositoryRootProjectionReceipt,
) -> bool {
    producer::verify_projection_receipt(receipt) && watchdog::verify_projection_receipt(receipt)
}

pub(in super::super) fn verify_projection(projection: &RepositoryRootProjection) -> bool {
    producer::verify_projection(projection) && watchdog::verify_projection(projection)
}

fn pair_receipt_digest(receipt: &RepositoryRootProjectionReceipt) -> [u8; 32] {
    let mut bytes = PAIR_RECEIPT_DOMAIN.to_vec();
    append_field(&mut bytes, 1, receipt.schema_id.as_bytes());
    append_field(&mut bytes, 2, receipt.claim_id.as_bytes());
    append_field(&mut bytes, 3, receipt.producer_id.as_bytes());
    append_field(&mut bytes, 4, receipt.watchdog_id.as_bytes());
    append_field(&mut bytes, 5, &receipt.input_sha256);
    append_field(&mut bytes, 6, &receipt.result_sha256);
    append_field(&mut bytes, 7, &receipt.producer_result_sha256);
    append_field(&mut bytes, 8, &receipt.watchdog_result_sha256);
    append_field(&mut bytes, 9, &receipt.producer_resource_contract_sha256);
    append_field(&mut bytes, 10, &receipt.watchdog_resource_contract_sha256);
    append_field(&mut bytes, 11, receipt.canary_suite_id.as_bytes());
    append_field(&mut bytes, 12, &receipt.canary_sha256);
    append_field(
        &mut bytes,
        13,
        &receipt.scalar_coordinate_count.to_be_bytes(),
    );
    append_field(&mut bytes, 14, &receipt.mass_projection_count.to_be_bytes());
    append_field(&mut bytes, 15, &[u8::from(receipt.membership_authority)]);
    append_field(&mut bytes, 16, receipt.decision_id.as_bytes());
    append_field(&mut bytes, 17, &receipt.producer_ancestry_manifest_sha256);
    append_field(&mut bytes, 18, &receipt.watchdog_ancestry_manifest_sha256);
    sha256(&bytes)
}

fn verify_refusal_receipt(receipt: &RepositoryRootRefusalReceipt) -> bool {
    receipt.schema_id == REPOSITORY_ROOT_REFUSAL_RECEIPT_SCHEMA_ID
        && receipt.claim_id == REPOSITORY_ROOT_CLAIM_ID
        && receipt.producer_id == REPOSITORY_ROOT_PRODUCER_ID
        && receipt.watchdog_id == REPOSITORY_ROOT_WATCHDOG_ID
        && receipt.canary.suite_id == REPOSITORY_ROOT_CANARY_SUITE_ID
        && receipt.decision_id == "refused"
        && !receipt.membership_authority
        && valid_checker_outcome(&receipt.producer_outcome, receipt.producer_packet_sha256)
        && valid_checker_outcome(&receipt.watchdog_outcome, receipt.watchdog_packet_sha256)
        && valid_resource_outcome(&receipt.producer_resource_contract)
        && valid_resource_outcome(&receipt.watchdog_resource_contract)
        && match receipt.canary.digest_sha256 {
            Some(digest) => digest != [0; 32],
            None => true,
        }
        && refusal_decision_is_consistent(receipt)
        && receipt.receipt_sha256 != [0; 32]
        && refusal_receipt_digest(receipt) == receipt.receipt_sha256
}

fn verify_refusal(refusal: &RepositoryRootRefusal) -> bool {
    refusal.code == refusal.receipt.refusal_code && verify_refusal_receipt(&refusal.receipt)
}

/// Accept refusal evidence at the production boundary only when a fresh replay
/// of the current sealed pair yields the exact same refusal. Structural receipt
/// consistency remains an internal construction check for synthetic packets.
pub(in super::super) fn verify_current_refusal(refusal: &RepositoryRootRefusal) -> bool {
    if !verify_refusal(refusal) {
        return false;
    }
    matches!(
        decide_repository_roots(),
        RepositoryRootDecision::Refused(current)
            if &current == refusal && verify_refusal(&current)
    )
}

fn valid_checker_outcome(outcome: &RepositoryRootCheckerOutcome, packet_sha256: [u8; 32]) -> bool {
    match outcome {
        RepositoryRootCheckerOutcome::Projected {
            input_sha256,
            result_sha256,
        } => packet_sha256 != [0; 32] && *input_sha256 != [0; 32] && *result_sha256 != [0; 32],
        RepositoryRootCheckerOutcome::Refused { .. } => true,
        RepositoryRootCheckerOutcome::NotRun { .. } => packet_sha256 != [0; 32],
    }
}

fn valid_resource_outcome(outcome: &RepositoryRootResourceContractOutcome) -> bool {
    match outcome {
        RepositoryRootResourceContractOutcome::Bound { digest_sha256 } => *digest_sha256 != [0; 32],
        RepositoryRootResourceContractOutcome::Refused { .. } => true,
    }
}

fn refusal_decision_is_consistent(receipt: &RepositoryRootRefusalReceipt) -> bool {
    let resource_contracts_agree = matching_resource_digests(
        &receipt.producer_resource_contract,
        &receipt.watchdog_resource_contract,
    )
    .is_some();
    let canary_digest_available = receipt.canary.digest_sha256.is_some();

    match receipt.refusal_stage {
        RepositoryRootRefusalStage::ResourceContractPreflight => {
            receipt.refusal_code == RepositoryRootRefusalCode::ResourceContractDisagreement
                && !resource_contracts_agree
                && !canary_digest_available
                && resource_preflight_outcomes_are_consistent(receipt)
        }
        RepositoryRootRefusalStage::PacketExtraction => {
            resource_contracts_agree
                && !canary_digest_available
                && packet_extraction_refusal_is_consistent(receipt)
        }
        RepositoryRootRefusalStage::PacketAgreement => {
            resource_contracts_agree
                && !canary_digest_available
                && receipt.refusal_code == RepositoryRootRefusalCode::SealedFloorPacketMismatch
                && receipt.producer_packet_sha256 != [0; 32]
                && receipt.watchdog_packet_sha256 != [0; 32]
                && receipt.producer_packet_sha256 != receipt.watchdog_packet_sha256
                && matches!(
                    receipt.producer_outcome,
                    RepositoryRootCheckerOutcome::NotRun {
                        reason: RepositoryRootRefusalCode::SealedFloorPacketMismatch
                    }
                )
                && matches!(
                    receipt.watchdog_outcome,
                    RepositoryRootCheckerOutcome::NotRun {
                        reason: RepositoryRootRefusalCode::SealedFloorPacketMismatch
                    }
                )
        }
        RepositoryRootRefusalStage::CheckerAgreement => {
            resource_contracts_agree
                && !canary_digest_available
                && checker_refusal_is_consistent(receipt)
        }
        RepositoryRootRefusalStage::Canary => {
            resource_contracts_agree
                && !canary_digest_available
                && receipt.refusal_code == RepositoryRootRefusalCode::CanaryFailure
                && projected_pair_agrees(receipt)
        }
        RepositoryRootRefusalStage::ProjectionReceipt => {
            resource_contracts_agree
                && canary_digest_available
                && matches!(
                    receipt.refusal_code,
                    RepositoryRootRefusalCode::ReceiptInvalid
                        | RepositoryRootRefusalCode::CanaryFailure
                )
                && projected_pair_agrees(receipt)
        }
    }
}

fn resource_preflight_outcomes_are_consistent(receipt: &RepositoryRootRefusalReceipt) -> bool {
    preflight_outcome_is_consistent(&receipt.producer_outcome, receipt.producer_packet_sha256)
        && preflight_outcome_is_consistent(
            &receipt.watchdog_outcome,
            receipt.watchdog_packet_sha256,
        )
}

fn preflight_outcome_is_consistent(
    outcome: &RepositoryRootCheckerOutcome,
    packet_sha256: [u8; 32],
) -> bool {
    match outcome {
        RepositoryRootCheckerOutcome::NotRun {
            reason: RepositoryRootRefusalCode::ResourceContractDisagreement,
        } => packet_sha256 != [0; 32],
        RepositoryRootCheckerOutcome::Refused { .. } => packet_sha256 == [0; 32],
        _ => false,
    }
}

fn packet_extraction_refusal_is_consistent(receipt: &RepositoryRootRefusalReceipt) -> bool {
    if receipt.producer_packet_sha256 != [0; 32] && receipt.watchdog_packet_sha256 != [0; 32] {
        return false;
    }
    if !extraction_stage_outcome_is_consistent(
        &receipt.producer_outcome,
        receipt.producer_packet_sha256,
    ) || !extraction_stage_outcome_is_consistent(
        &receipt.watchdog_outcome,
        receipt.watchdog_packet_sha256,
    ) {
        return false;
    }

    match (&receipt.producer_outcome, &receipt.watchdog_outcome) {
        (
            RepositoryRootCheckerOutcome::Refused {
                code: producer_code,
            },
            RepositoryRootCheckerOutcome::Refused {
                code: watchdog_code,
            },
        ) if producer_code == watchdog_code => receipt.refusal_code == *producer_code,
        _ => receipt.refusal_code == RepositoryRootRefusalCode::CheckerDisagreement,
    }
}

fn extraction_stage_outcome_is_consistent(
    outcome: &RepositoryRootCheckerOutcome,
    packet_sha256: [u8; 32],
) -> bool {
    match outcome {
        RepositoryRootCheckerOutcome::Refused { .. } => packet_sha256 == [0; 32],
        RepositoryRootCheckerOutcome::NotRun {
            reason: RepositoryRootRefusalCode::CheckerDisagreement,
        } => packet_sha256 != [0; 32],
        _ => false,
    }
}

fn checker_refusal_is_consistent(receipt: &RepositoryRootRefusalReceipt) -> bool {
    if receipt.producer_packet_sha256 == [0; 32]
        || receipt.producer_packet_sha256 != receipt.watchdog_packet_sha256
        || matches!(
            receipt.producer_outcome,
            RepositoryRootCheckerOutcome::NotRun { .. }
        )
        || matches!(
            receipt.watchdog_outcome,
            RepositoryRootCheckerOutcome::NotRun { .. }
        )
    {
        return false;
    }

    match (&receipt.producer_outcome, &receipt.watchdog_outcome) {
        (
            RepositoryRootCheckerOutcome::Refused {
                code: producer_code,
            },
            RepositoryRootCheckerOutcome::Refused {
                code: watchdog_code,
            },
        ) if producer_code == watchdog_code => receipt.refusal_code == *producer_code,
        (producer_outcome, watchdog_outcome) if producer_outcome != watchdog_outcome => {
            receipt.refusal_code == RepositoryRootRefusalCode::CheckerDisagreement
        }
        _ => false,
    }
}

fn projected_pair_agrees(receipt: &RepositoryRootRefusalReceipt) -> bool {
    receipt.producer_packet_sha256 != [0; 32]
        && receipt.producer_packet_sha256 == receipt.watchdog_packet_sha256
        && matches!(
            (&receipt.producer_outcome, &receipt.watchdog_outcome),
            (
                RepositoryRootCheckerOutcome::Projected {
                    input_sha256: producer_input,
                    result_sha256: producer_result,
                },
                RepositoryRootCheckerOutcome::Projected {
                    input_sha256: watchdog_input,
                    result_sha256: watchdog_result,
                },
            ) if producer_input == watchdog_input && producer_result == watchdog_result
        )
}

fn refusal_receipt_digest(receipt: &RepositoryRootRefusalReceipt) -> [u8; 32] {
    let mut bytes = REFUSAL_RECEIPT_DOMAIN.to_vec();
    append_field(&mut bytes, 1, receipt.schema_id.as_bytes());
    append_field(&mut bytes, 2, receipt.claim_id.as_bytes());
    append_field(&mut bytes, 3, receipt.producer_id.as_bytes());
    append_field(&mut bytes, 4, receipt.watchdog_id.as_bytes());
    append_field(&mut bytes, 5, &receipt.producer_packet_sha256);
    append_field(&mut bytes, 6, &receipt.watchdog_packet_sha256);
    append_field(
        &mut bytes,
        7,
        &encode_checker_outcome(&receipt.producer_outcome),
    );
    append_field(
        &mut bytes,
        8,
        &encode_checker_outcome(&receipt.watchdog_outcome),
    );
    append_field(
        &mut bytes,
        9,
        &encode_resource_outcome(&receipt.producer_resource_contract),
    );
    append_field(
        &mut bytes,
        10,
        &encode_resource_outcome(&receipt.watchdog_resource_contract),
    );
    append_field(&mut bytes, 11, receipt.canary.suite_id.as_bytes());
    match receipt.canary.digest_sha256 {
        Some(digest) => {
            append_field(&mut bytes, 12, &[1]);
            append_field(&mut bytes, 13, &digest);
        }
        None => {
            append_field(&mut bytes, 12, &[0]);
            append_field(&mut bytes, 13, &[]);
        }
    }
    append_field(&mut bytes, 14, receipt.refusal_stage.id().as_bytes());
    append_field(&mut bytes, 15, receipt.refusal_code.id().as_bytes());
    append_field(&mut bytes, 16, receipt.decision_id.as_bytes());
    append_field(&mut bytes, 17, &[u8::from(receipt.membership_authority)]);
    sha256(&bytes)
}

fn encode_checker_outcome(outcome: &RepositoryRootCheckerOutcome) -> Vec<u8> {
    let mut bytes = Vec::new();
    match outcome {
        RepositoryRootCheckerOutcome::Projected {
            input_sha256,
            result_sha256,
        } => {
            append_field(&mut bytes, 1, b"projected");
            append_field(&mut bytes, 2, input_sha256);
            append_field(&mut bytes, 3, result_sha256);
        }
        RepositoryRootCheckerOutcome::Refused { code } => {
            append_field(&mut bytes, 1, b"refused");
            append_field(&mut bytes, 2, code.id().as_bytes());
        }
        RepositoryRootCheckerOutcome::NotRun { reason } => {
            append_field(&mut bytes, 1, b"not_run");
            append_field(&mut bytes, 2, reason.id().as_bytes());
        }
    }
    bytes
}

fn encode_resource_outcome(outcome: &RepositoryRootResourceContractOutcome) -> Vec<u8> {
    let mut bytes = Vec::new();
    match outcome {
        RepositoryRootResourceContractOutcome::Bound { digest_sha256 } => {
            append_field(&mut bytes, 1, b"bound");
            append_field(&mut bytes, 2, digest_sha256);
        }
        RepositoryRootResourceContractOutcome::Refused { code } => {
            append_field(&mut bytes, 1, b"refused");
            append_field(&mut bytes, 2, code.id().as_bytes());
        }
    }
    bytes
}

fn admit_candidate(
    candidate: &RootCandidate,
    receipt: &RepositoryRootProjectionReceipt,
) -> AdmittedArtifact {
    let ancestry_receipt = ReceiptBinding {
        schema_id: REPOSITORY_ROOT_ANCESTRY_SCHEMA_ID.to_owned(),
        digest_sha256: candidate.ancestry_digest_sha256,
    };
    let semantic_checker_receipt = ReceiptBinding {
        schema_id: REPOSITORY_ROOT_PRODUCER_RECEIPT_SCHEMA_ID.to_owned(),
        digest_sha256: checker_claim_digest(
            REPOSITORY_ROOT_PRODUCER_ID,
            receipt.input_sha256,
            receipt.producer_result_sha256,
            receipt.receipt_sha256,
            candidate,
        ),
    };
    let independent_watchdog_receipt = ReceiptBinding {
        schema_id: REPOSITORY_ROOT_WATCHDOG_RECEIPT_SCHEMA_ID.to_owned(),
        digest_sha256: checker_claim_digest(
            REPOSITORY_ROOT_WATCHDOG_ID,
            receipt.input_sha256,
            receipt.watchdog_result_sha256,
            receipt.receipt_sha256,
            candidate,
        ),
    };
    let admission = RootAdmission {
        tier: candidate.source_tier,
        provenance: ProvenanceMark::Derived,
        route: AdmissionRoute::Derived(DerivedAdmission {
            ancestry_receipt,
            semantic_checker_receipt,
            independent_watchdog_receipt,
        }),
    };
    let capability = RepositoryRootAdmissionCapability {
        claimed_identity: candidate.identity,
        admission: admission.clone(),
        pair_receipt_sha256: receipt.receipt_sha256,
    };
    AdmittedArtifact::from_repository_root(
        candidate.identity,
        admission,
        candidate.payload.clone(),
        capability,
    )
}

fn checker_claim_digest(
    checker_id: &str,
    input_sha256: [u8; 32],
    result_sha256: [u8; 32],
    pair_receipt_sha256: [u8; 32],
    candidate: &RootCandidate,
) -> [u8; 32] {
    let mut bytes = CHECKER_CLAIM_DOMAIN.to_vec();
    append_field(
        &mut bytes,
        1,
        REPOSITORY_ROOT_PROJECTION_SCHEMA_ID.as_bytes(),
    );
    append_field(&mut bytes, 2, checker_id.as_bytes());
    append_field(&mut bytes, 3, &input_sha256);
    append_field(&mut bytes, 4, &result_sha256);
    append_field(&mut bytes, 5, &pair_receipt_sha256);
    append_field(&mut bytes, 6, candidate.kind.id().as_bytes());
    append_field(&mut bytes, 7, &candidate.identity.0);
    append_field(&mut bytes, 8, candidate.source_entry_id.as_bytes());
    append_field(&mut bytes, 9, &[candidate.source_tier.number()]);
    append_field(&mut bytes, 10, candidate.source_tier.id().as_bytes());
    append_field(&mut bytes, 11, &candidate.ancestry_digest_sha256);
    sha256(&bytes)
}

fn append_field(bytes: &mut Vec<u8>, tag: u16, payload: &[u8]) {
    bytes.extend_from_slice(&tag.to_be_bytes());
    bytes.extend_from_slice(
        &u64::try_from(payload.len())
            .expect("bounded root claim payload length fits u64")
            .to_be_bytes(),
    );
    bytes.extend_from_slice(payload);
}
