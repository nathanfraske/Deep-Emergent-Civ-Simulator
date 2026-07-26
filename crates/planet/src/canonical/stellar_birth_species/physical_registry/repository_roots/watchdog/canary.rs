//! Watchdog-owned mutation canaries for the sealed root projection.

use super::super::super::model::{
    AdmissionCapabilityKind, AdmissionRoute, AdmittedArtifact, ArtifactPayload, DimensionVector,
    LedgerTier, MassProjectionScope, ProvenanceMark, RootAdmission,
};
use super::super::admit_candidate;
use super::super::model::{
    RepositoryRootPacket, RepositoryRootProjection, RepositoryRootProjectionReceipt,
    RepositoryRootRefusalCode, RootCandidate, RootCheckerOutput,
};
use super::{
    inspect_projection_core, mass_projection_eligible, parse_decimal_from_right, project,
    watchdog_dimension_from_si_exponents, write_payload, write_receipt, write_source, Wire,
    WATCHDOG_PURE_MASS_DIMENSION, WATCHDOG_SOURCE_LIMIT,
};
use civsim_units::digest::sha256;

const CANARY_RECEIPT_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.canary-receipt.v6";
const CANARY_ENTRY_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.canary-entry.v1";
const CANARY_PACKET_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-packet-preimage.v1";
const CANARY_PROJECTION_TRANSCRIPT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-projection-transcript.v1";
const CANARY_PROJECTION_PREIMAGE_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-projection-preimage.v1";
const CANARY_OBSERVED_OUTCOME_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-observed-outcome.v1";
const CANARY_CHECKER_OUTPUT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-checker-output.v1";
const CANARY_CANDIDATE_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-candidate.v1";
const CANARY_ADMISSION_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-admission.v1";
const CANARY_CAPABILITY_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-capability.v1";
const CANARY_ARTIFACT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-admitted-artifact.v1";
const CANARY_PROJECTION_RECEIPT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.canary-projection-receipt.v1";
pub(super) fn packet_digest(
    packet: &RepositoryRootPacket,
    baseline: &RootCheckerOutput,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    if packet.sources.len() < 2
        || baseline.mass_projection_count == 0
        || project(packet)? != *baseline
    {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }

    let mut transcript = Wire::start(CANARY_RECEIPT_DOMAIN)?;

    let mut decimal_digit = packet.clone();
    replace_first_digit(&mut decimal_digit.sources[0].central_decimal)?;
    record_refusal(
        &mut transcript,
        "decimal_digit_mutation",
        &decimal_digit,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut decimal_sign = packet.clone();
    decimal_sign.sources[0].central_decimal.insert(0, '-');
    record_refusal(
        &mut transcript,
        "decimal_sign_mutation",
        &decimal_sign,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut decimal_exponent = packet.clone();
    replace_exponent_digit(&mut decimal_exponent.sources[0].central_decimal)?;
    record_refusal(
        &mut transcript,
        "decimal_exponent_mutation",
        &decimal_exponent,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut dimension = packet.clone();
    dimension.sources[0].dimension = watchdog_dimension_from_si_exponents([1, 0, 0, 0, 0, 0, 0])
        .map_err(|_| RepositoryRootRefusalCode::CanaryFailure)?;
    record_refusal(
        &mut transcript,
        "sealed_dimension_binding_mutation",
        &dimension,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut source_order = packet.clone();
    let final_index = source_order
        .sources
        .len()
        .checked_sub(1)
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    source_order.sources.swap(0, final_index);
    record_canonicalization(
        &mut transcript,
        "source_order_permutation",
        &source_order,
        baseline,
    )?;

    let mut source_membership = packet.clone();
    source_membership
        .sources
        .pop()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    record_refusal(
        &mut transcript,
        "source_membership_mutation",
        &source_membership,
        RepositoryRootRefusalCode::SealedSourceBindingMismatch,
    )?;

    let mut tier = packet.clone();
    tier.sources[0].source_tier = LedgerTier::Reference;
    record_refusal(
        &mut transcript,
        "tier_mutation",
        &tier,
        RepositoryRootRefusalCode::SourceTierNotUniversal,
    )?;

    let mut provenance = packet.clone();
    provenance.sources[0].source_provenance = ProvenanceMark::Estimator;
    record_refusal(
        &mut transcript,
        "provenance_mutation",
        &provenance,
        RepositoryRootRefusalCode::SourceProvenanceNotMeasured,
    )?;

    let mut uncertainty = packet.clone();
    uncertainty.sources[0].uncertainty_decimal.push('0');
    record_refusal(
        &mut transcript,
        "uncertainty_mutation",
        &uncertainty,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut leaf_receipt = packet.clone();
    leaf_receipt.sources[0].exhaustion_binding.digest_sha256[0] ^= 0x80;
    record_refusal(
        &mut transcript,
        "leaf_receipt_fingerprint_mutation",
        &leaf_receipt,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut floor_authority = packet.clone();
    floor_authority.floor_authority.digest_sha256[0] ^= 0x40;
    record_refusal(
        &mut transcript,
        "floor_authority_mutation",
        &floor_authority,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;

    let mut mass_predicate = packet.clone();
    let mass_source = mass_predicate
        .sources
        .iter_mut()
        .find(|source| source.dimension == WATCHDOG_PURE_MASS_DIMENSION)
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    mass_source.dimension = DimensionVector::dimensionless();
    record_refusal(
        &mut transcript,
        "sealed_positive_pure_mass_dimension_binding_mutation",
        &mass_predicate,
        RepositoryRootRefusalCode::SealedFloorPacketMismatch,
    )?;
    record_mass_classification_semantics(&mut transcript, packet)?;

    let mut membership_authority = packet.clone();
    membership_authority.membership_authority = true;
    record_refusal(
        &mut transcript,
        "membership_authority_mutation",
        &membership_authority,
        RepositoryRootRefusalCode::MembershipAuthorityPresent,
    )?;

    let mut duplicated_source = packet.clone();
    duplicated_source.sources.push(
        duplicated_source
            .sources
            .first()
            .cloned()
            .ok_or(RepositoryRootRefusalCode::CanaryFailure)?,
    );
    record_refusal(
        &mut transcript,
        "duplicated_source_mutation",
        &duplicated_source,
        RepositoryRootRefusalCode::DuplicateEntryId,
    )?;

    let mut schema = packet.clone();
    schema.schema_id.push_str(".substituted");
    record_refusal(
        &mut transcript,
        "schema_substitution",
        &schema,
        RepositoryRootRefusalCode::SchemaMismatch,
    )?;

    let mut exponent_bound = packet.clone();
    exponent_bound.sources[0].central_decimal = "1e1025".to_owned();
    record_refusal(
        &mut transcript,
        "resource_bound_substitution",
        &exponent_bound,
        RepositoryRootRefusalCode::DecimalMagnitudeExceeded,
    )?;

    let mut source_bound = packet.clone();
    let first_source = packet
        .sources
        .first()
        .cloned()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    while source_bound.sources.len() <= WATCHDOG_SOURCE_LIMIT {
        source_bound.sources.push(first_source.clone());
    }
    record_refusal(
        &mut transcript,
        "source_capacity_substitution",
        &source_bound,
        RepositoryRootRefusalCode::SourceCapacityExceeded,
    )?;

    let mut representation = packet.clone();
    representation.sources[0].entry_id = "fundamental.h".to_owned();
    representation.sources[0].symbol = "h".to_owned();
    representation.sources[0].central_decimal = "6.62607015e-34".to_owned();
    representation.sources[0].uncertainty_kind = "exact".to_owned();
    representation.sources[0].uncertainty_decimal = "0".to_owned();
    representation.sources[0].dimension =
        watchdog_dimension_from_si_exponents([2, 1, -1, 0, 0, 0, 0])
            .map_err(|_| RepositoryRootRefusalCode::CanaryFailure)?;
    representation.sources[0].exhaustion_binding.digest_sha256[0] ^= 0x20;
    record_refusal(
        &mut transcript,
        "sealed_source_role_substitution",
        &representation,
        RepositoryRootRefusalCode::SealedSourceBindingMismatch,
    )?;

    let mut coordinated_rename = packet.clone();
    coordinated_rename.sources[0].entry_id = "fundamental.unfamiliar_alpha".to_owned();
    coordinated_rename.sources[0].symbol = "unfamiliar_alpha".to_owned();
    record_refusal(
        &mut transcript,
        "coordinated_identity_rename_with_old_receipt",
        &coordinated_rename,
        RepositoryRootRefusalCode::SealedSourceBindingMismatch,
    )?;

    Ok(sha256(&transcript.finish()))
}

pub(super) fn projection_digest(
    baseline: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    inspect_projection_core(baseline)?;

    let mut omitted_artifact = baseline.clone();
    omitted_artifact
        .admitted_artifacts
        .pop()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;

    let mut duplicated_artifact = baseline.clone();
    duplicated_artifact.admitted_artifacts.push(
        baseline
            .admitted_artifacts
            .first()
            .cloned()
            .ok_or(RepositoryRootRefusalCode::CanaryFailure)?,
    );

    let mut scope_substitution = baseline.clone();
    let mass_projection = scope_substitution
        .admitted_artifacts
        .iter_mut()
        .find_map(|artifact| match &mut artifact.payload {
            ArtifactPayload::MassProjection(projection) => Some(projection),
            _ => None,
        })
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    mass_projection.scope = MassProjectionScope::SpeciesRestMass;

    let mut canonical_byte = baseline.clone();
    let byte = canonical_byte
        .canonical_bytes
        .first_mut()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    *byte ^= 1;

    let mut semantic_checker_receipt = baseline.clone();
    let AdmissionRoute::Derived(admission) = &mut semantic_checker_receipt
        .admitted_artifacts
        .first_mut()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?
        .admission
        .route
    else {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    };
    admission.semantic_checker_receipt.digest_sha256[0] ^= 1;

    let mut ancestry_and_admission = baseline.clone();
    ancestry_and_admission.checker_candidates[0].ancestry_digest_sha256[0] ^= 1;
    ancestry_and_admission.admitted_artifacts = ancestry_and_admission
        .checker_candidates
        .iter()
        .map(|candidate| admit_candidate(candidate, &ancestry_and_admission.receipt))
        .collect();

    let mut transcript = Wire::start(CANARY_PROJECTION_TRANSCRIPT_DOMAIN)?;
    for (id, mutation) in [
        ("omitted_artifact", omitted_artifact),
        ("duplicated_artifact", duplicated_artifact),
        (
            "membership_neutral_to_species_rest_mass_substitution",
            scope_substitution,
        ),
        ("canonical_projection_byte_mutation", canonical_byte),
        ("admission_receipt_substitution", semantic_checker_receipt),
        (
            "coordinated_ancestry_and_admission_substitution",
            ancestry_and_admission,
        ),
    ] {
        record_projection_refusal(&mut transcript, id, &mutation)?;
    }
    Ok(sha256(&transcript.finish()))
}

fn record_refusal(
    transcript: &mut Wire,
    id: &str,
    packet: &RepositoryRootPacket,
    expected: RepositoryRootRefusalCode,
) -> Result<(), RepositoryRootRefusalCode> {
    let observed = project(packet);
    match &observed {
        Err(code) if *code == expected => {
            record_packet_observation(transcript, id, packet, &observed)
        }
        _other => {
            #[cfg(test)]
            eprintln!("watchdog root canary {id:?} expected {expected:?}, observed {_other:?}");
            Err(RepositoryRootRefusalCode::CanaryFailure)
        }
    }
}

fn record_canonicalization(
    transcript: &mut Wire,
    id: &str,
    packet: &RepositoryRootPacket,
    baseline: &RootCheckerOutput,
) -> Result<(), RepositoryRootRefusalCode> {
    let observed = project(packet);
    if observed.as_ref() != Ok(baseline) {
        #[cfg(test)]
        eprintln!("watchdog root canary {id:?} changed the canonical projection");
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }
    record_packet_observation(transcript, id, packet, &observed)
}

fn record_mass_classification_semantics(
    transcript: &mut Wire,
    packet: &RepositoryRootPacket,
) -> Result<(), RepositoryRootRefusalCode> {
    let pure_mass = packet
        .sources
        .iter()
        .rev()
        .find(|source| source.dimension == WATCHDOG_PURE_MASS_DIMENSION)
        .cloned()
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    let pure_value = parse_decimal_from_right(&pure_mass.central_decimal)
        .map_err(|_| RepositoryRootRefusalCode::CanaryFailure)?;
    let pure_mass_projection = mass_projection_eligible(pure_mass.dimension, &pure_value.wire);

    let mut dimensionless = pure_mass.clone();
    dimensionless.dimension = DimensionVector::dimensionless();
    let dimensionless_value = parse_decimal_from_right(&dimensionless.central_decimal)
        .map_err(|_| RepositoryRootRefusalCode::CanaryFailure)?;
    let dimensionless_mass_projection =
        mass_projection_eligible(dimensionless.dimension, &dimensionless_value.wire);
    if dimensionless_mass_projection || !pure_mass_projection {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }

    let mut observed = Wire::start(CANARY_OBSERVED_OUTCOME_DOMAIN)?;
    observed.push(1, b"dimensionless_mass_projection_count")?;
    observed.push(
        2,
        &u32::from(u8::from(dimensionless_mass_projection)).to_be_bytes(),
    )?;
    observed.push(3, b"pure_mass_projection_count")?;
    observed.push(4, &u32::from(u8::from(pure_mass_projection)).to_be_bytes())?;

    let mut entry = Wire::start(CANARY_ENTRY_DOMAIN)?;
    entry.push(1, b"positive_dimension_classification_semantics")?;
    entry.push(2, &write_source(&dimensionless)?)?;
    entry.push(3, &write_source(&pure_mass)?)?;
    entry.push(4, &observed.finish())?;
    transcript.push(1, &entry.finish())
}

fn record_packet_observation(
    transcript: &mut Wire,
    id: &str,
    packet: &RepositoryRootPacket,
    observed: &Result<RootCheckerOutput, RepositoryRootRefusalCode>,
) -> Result<(), RepositoryRootRefusalCode> {
    let mut entry = Wire::start(CANARY_ENTRY_DOMAIN)?;
    entry.push(1, id.as_bytes())?;
    entry.push(2, &write_packet_preimage(packet)?)?;
    entry.push(3, &write_packet_outcome(observed)?)?;
    transcript.push(1, &entry.finish())
}

fn write_packet_preimage(
    packet: &RepositoryRootPacket,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_PACKET_PREIMAGE_DOMAIN)?;
    wire.push(1, packet.schema_id.as_bytes())?;
    wire.push(2, &write_receipt(&packet.floor_authority)?)?;
    wire.push(3, &[u8::from(packet.membership_authority)])?;
    wire.push(
        4,
        &u64::try_from(packet.sources.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for source in &packet.sources {
        wire.push(5, &write_source(source)?)?;
    }
    Ok(wire.finish())
}

fn write_packet_outcome(
    observed: &Result<RootCheckerOutput, RepositoryRootRefusalCode>,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_OBSERVED_OUTCOME_DOMAIN)?;
    match observed {
        Ok(output) => {
            wire.push(1, b"projected")?;
            wire.push(2, &write_checker_output(output)?)?;
        }
        Err(code) => {
            wire.push(1, b"refused")?;
            wire.push(2, code.id().as_bytes())?;
        }
    }
    Ok(wire.finish())
}

fn write_checker_output(output: &RootCheckerOutput) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_CHECKER_OUTPUT_DOMAIN)?;
    wire.push(1, &output.input_sha256)?;
    wire.push(2, &output.canonical_bytes)?;
    wire.push(3, &output.scalar_coordinate_count.to_be_bytes())?;
    wire.push(4, &output.mass_projection_count.to_be_bytes())?;
    wire.push(
        5,
        &u64::try_from(output.candidates.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for candidate in &output.candidates {
        wire.push(6, &write_candidate(candidate)?)?;
    }
    Ok(wire.finish())
}

fn write_candidate(candidate: &RootCandidate) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_CANDIDATE_DOMAIN)?;
    wire.push(1, candidate.source_entry_id.as_bytes())?;
    wire.push(2, &[candidate.source_tier.number()])?;
    wire.push(3, candidate.source_tier.id().as_bytes())?;
    wire.push(4, candidate.kind.id().as_bytes())?;
    wire.push(5, &candidate.identity.0)?;
    wire.push(6, &write_payload(&candidate.payload)?)?;
    wire.push(7, &candidate.ancestry_digest_sha256)?;
    Ok(wire.finish())
}

fn record_projection_refusal(
    transcript: &mut Wire,
    id: &str,
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    let observed = inspect_projection_core(projection);
    if observed.is_ok() {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }
    let mut entry = Wire::start(CANARY_ENTRY_DOMAIN)?;
    entry.push(1, id.as_bytes())?;
    entry.push(2, &write_projection_preimage(projection)?)?;
    entry.push(3, &write_projection_outcome(&observed)?)?;
    transcript.push(1, &entry.finish())
}

/// Encode every mutation-relevant projection field. The one omitted field is
/// `post_projection_canary_sha256`, which is the digest of this transcript and
/// would otherwise make the preimage self-referential.
fn write_projection_preimage(
    projection: &RepositoryRootProjection,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_PROJECTION_PREIMAGE_DOMAIN)?;
    wire.push(
        1,
        &u64::try_from(projection.admitted_artifacts.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for artifact in &projection.admitted_artifacts {
        wire.push(2, &write_admitted_artifact(artifact)?)?;
    }
    wire.push(3, &projection.canonical_bytes)?;
    wire.push(4, &write_projection_receipt(&projection.receipt)?)?;
    wire.push(5, &[u8::from(projection.membership_authority)])?;
    wire.push(
        6,
        &u64::try_from(projection.checker_candidates.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for candidate in &projection.checker_candidates {
        wire.push(7, &write_candidate(candidate)?)?;
    }
    Ok(wire.finish())
}

fn write_admitted_artifact(
    artifact: &AdmittedArtifact,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    if artifact.admission_capability_kind() != AdmissionCapabilityKind::RepositoryRoot {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }
    let pair_receipt_sha256 = artifact
        .repository_root_pair_receipt_sha256()
        .ok_or(RepositoryRootRefusalCode::ReceiptInvalid)?;
    let mut capability = Wire::start(CANARY_CAPABILITY_DOMAIN)?;
    capability.push(1, b"repository-root")?;
    capability.push(2, &artifact.capability_claimed_identity().0)?;
    capability.push(3, &write_admission(artifact.capability_admission())?)?;
    capability.push(4, &pair_receipt_sha256)?;

    let mut wire = Wire::start(CANARY_ARTIFACT_DOMAIN)?;
    wire.push(1, &artifact.claimed_identity.0)?;
    wire.push(2, &write_admission(&artifact.admission)?)?;
    wire.push(3, &write_payload(&artifact.payload)?)?;
    wire.push(4, &capability.finish())?;
    Ok(wire.finish())
}

fn write_admission(admission: &RootAdmission) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_ADMISSION_DOMAIN)?;
    wire.push(1, &[admission.tier.number()])?;
    wire.push(2, admission.tier.id().as_bytes())?;
    wire.push(3, admission.provenance.tag().as_bytes())?;
    wire.push(
        4,
        admission
            .provenance
            .bracket_tag()
            .ok_or(RepositoryRootRefusalCode::ReceiptInvalid)?
            .as_bytes(),
    )?;
    let AdmissionRoute::Derived(derived) = &admission.route else {
        return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
    };
    wire.push(5, b"derived")?;
    wire.push(6, &write_receipt(&derived.ancestry_receipt)?)?;
    wire.push(7, &write_receipt(&derived.semantic_checker_receipt)?)?;
    wire.push(8, &write_receipt(&derived.independent_watchdog_receipt)?)?;
    Ok(wire.finish())
}

fn write_projection_receipt(
    receipt: &RepositoryRootProjectionReceipt,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_PROJECTION_RECEIPT_DOMAIN)?;
    wire.push(1, receipt.schema_id.as_bytes())?;
    wire.push(2, receipt.claim_id.as_bytes())?;
    wire.push(3, &receipt.input_sha256)?;
    wire.push(4, &receipt.result_sha256)?;
    wire.push(5, &receipt.producer_result_sha256)?;
    wire.push(6, &receipt.watchdog_result_sha256)?;
    wire.push(7, &receipt.producer_ancestry_manifest_sha256)?;
    wire.push(8, &receipt.watchdog_ancestry_manifest_sha256)?;
    wire.push(9, &receipt.producer_resource_contract_sha256)?;
    wire.push(10, &receipt.watchdog_resource_contract_sha256)?;
    wire.push(11, receipt.canary_suite_id.as_bytes())?;
    wire.push(12, &receipt.canary_sha256)?;
    wire.push(13, receipt.producer_id.as_bytes())?;
    wire.push(14, receipt.watchdog_id.as_bytes())?;
    wire.push(15, &receipt.scalar_coordinate_count.to_be_bytes())?;
    wire.push(16, &receipt.mass_projection_count.to_be_bytes())?;
    wire.push(17, &[u8::from(receipt.membership_authority)])?;
    wire.push(18, receipt.decision_id.as_bytes())?;
    wire.push(19, &receipt.receipt_sha256)?;
    Ok(wire.finish())
}

fn write_projection_outcome(
    observed: &Result<(), RepositoryRootRefusalCode>,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CANARY_OBSERVED_OUTCOME_DOMAIN)?;
    match observed {
        Ok(()) => wire.push(1, b"accepted")?,
        Err(code) => {
            wire.push(1, b"refused")?;
            wire.push(2, code.id().as_bytes())?;
        }
    }
    Ok(wire.finish())
}

#[cfg(test)]
pub(super) fn packet_preimage_sha256_for_test(
    packet: &RepositoryRootPacket,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    Ok(sha256(&write_packet_preimage(packet)?))
}

#[cfg(test)]
pub(super) fn packet_observation_sha256_for_test(
    id: &str,
    packet: &RepositoryRootPacket,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    let observed = project(packet);
    let mut transcript = Wire::start(CANARY_RECEIPT_DOMAIN)?;
    record_packet_observation(&mut transcript, id, packet, &observed)?;
    Ok(sha256(&transcript.finish()))
}

#[cfg(test)]
pub(super) fn projection_preimage_sha256_for_test(
    projection: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    Ok(sha256(&write_projection_preimage(projection)?))
}

#[cfg(test)]
pub(super) fn projection_observation_sha256_for_test(
    id: &str,
    projection: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    let mut transcript = Wire::start(CANARY_PROJECTION_TRANSCRIPT_DOMAIN)?;
    record_projection_refusal(&mut transcript, id, projection)?;
    Ok(sha256(&transcript.finish()))
}

fn replace_first_digit(value: &mut String) -> Result<(), RepositoryRootRefusalCode> {
    let (index, digit) = value
        .char_indices()
        .find(|(_, character)| character.is_ascii_digit())
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    let replacement = if digit == '9' {
        '8'
    } else {
        char::from_u32(u32::from(digit) + 1).ok_or(RepositoryRootRefusalCode::CanaryFailure)?
    };
    value.replace_range(index..index + digit.len_utf8(), &replacement.to_string());
    Ok(())
}

fn replace_exponent_digit(value: &mut String) -> Result<(), RepositoryRootRefusalCode> {
    let exponent_index = value
        .char_indices()
        .find_map(|(index, character)| (character == 'e').then_some(index))
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    let (offset, digit) = value[exponent_index + 1..]
        .char_indices()
        .find(|(_, character)| character.is_ascii_digit())
        .ok_or(RepositoryRootRefusalCode::CanaryFailure)?;
    let index = exponent_index + 1 + offset;
    let replacement = if digit == '9' {
        '8'
    } else {
        char::from_u32(u32::from(digit) + 1).ok_or(RepositoryRootRefusalCode::CanaryFailure)?
    };
    value.replace_range(index..index + digit.len_utf8(), &replacement.to_string());
    Ok(())
}
