//! Bottom-up repository root projector.

mod canary;

use super::super::model::{
    AdmissionRoute, ArtifactIdentity, ArtifactPayload, CanonicalArtifact, DimensionTerm,
    DimensionVector, ExactExpression, ExactExpressionNode, ExactRationalWire, LedgerTier,
    MassProjectionArtifact, MassProjectionScope, ProvenanceMark, ReceiptBinding,
    ScalarCoordinateArtifact, SI_AMOUNT_AXIS, SI_CURRENT_AXIS, SI_LENGTH_AXIS,
    SI_LUMINOUS_INTENSITY_AXIS, SI_MASS_AXIS, SI_TEMPERATURE_AXIS, SI_TIME_AXIS,
};
use super::model::*;
use civsim_units::{
    bignum::BigUint,
    digest::sha256,
    fundamentals::FundamentalRole,
    physics_floor::{
        sealed_physical_floor_authority_binding, sealed_physical_floor_definitions,
        sealed_physical_floor_receipt_fingerprints,
    },
};
use std::collections::BTreeSet;

const PRODUCER_SOURCE_LIMIT: usize = 64;
const PRODUCER_TOKEN_BYTE_LIMIT: usize = 192;
const PRODUCER_DECIMAL_BYTE_LIMIT: usize = 256;
const PRODUCER_DECIMAL_EXPONENT_LIMIT: i32 = 1_024;
const PRODUCER_CANONICAL_BYTE_LIMIT: usize = 1_048_576;
const PRODUCER_DIMENSION_EXPONENT_LIMIT: i32 = 4_096;
const PRODUCER_RATIONAL_COMPONENT_BIT_LIMIT: u32 = 4_096;
const PRODUCER_PURE_MASS_DIMENSION: DimensionVector = DimensionVector::one(SI_MASS_AXIS);

const INPUT_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.input.v3";
const SOURCE_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.source.v2";
const COORDINATE_CONTENT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.coordinate-content.v3";
const SCALAR_ANCESTRY_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.scalar-ancestry.v2";
const MASS_ANCESTRY_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.mass-ancestry.v2";
const PROJECTION_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.projection.v3";
const PROJECTED_ARTIFACT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.projected-artifact.v3";
const ANCESTRY_MANIFEST_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.ancestry-manifest.v1";
const ANCESTRY_MANIFEST_ENTRY_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.ancestry-manifest-entry.v1";
const RECEIPT_DOMAIN: &[u8] = b"civsim.physical-species.receipt.v1";
const ARTIFACT_DOMAIN: &[u8] = b"civsim.physical-species.artifact.v3";
const CONTENT_DOMAIN: &[u8] = b"civsim.physical-species.content.v1";
const RATIONAL_DOMAIN: &[u8] = b"civsim.physical-species.rational.v1";
const EXPRESSION_DOMAIN: &[u8] = b"civsim.physical-species.expression.v1";
const EXPRESSION_NODE_DOMAIN: &[u8] = b"civsim.physical-species.expression-node.v1";
const RESOURCE_CONTRACT_DOMAIN: &[u8] =
    b"civsim.planet.repository-physical-root.resource-contract.v4";
const PAIR_RECEIPT_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.pair-receipt.v5";
const CHECKER_CLAIM_DOMAIN: &[u8] = b"civsim.planet.repository-physical-root.checker-claim.v1";

#[derive(Debug, Clone)]
struct ParsedDecimal {
    wire: ExactRationalWire,
}

struct Record {
    bytes: Vec<u8>,
}

fn producer_dimension_from_si_exponents(
    exponents: [i16; 7],
) -> Result<DimensionVector, RepositoryRootRefusalCode> {
    let [length, mass, time, current, temperature, amount, luminous_intensity] = exponents;
    let mut terms = Vec::with_capacity(7);
    if length != 0 {
        terms.push(DimensionTerm {
            axis: SI_LENGTH_AXIS,
            exponent: length,
        });
    }
    if mass != 0 {
        terms.push(DimensionTerm {
            axis: SI_MASS_AXIS,
            exponent: mass,
        });
    }
    if time != 0 {
        terms.push(DimensionTerm {
            axis: SI_TIME_AXIS,
            exponent: time,
        });
    }
    if current != 0 {
        terms.push(DimensionTerm {
            axis: SI_CURRENT_AXIS,
            exponent: current,
        });
    }
    if temperature != 0 {
        terms.push(DimensionTerm {
            axis: SI_TEMPERATURE_AXIS,
            exponent: temperature,
        });
    }
    if amount != 0 {
        terms.push(DimensionTerm {
            axis: SI_AMOUNT_AXIS,
            exponent: amount,
        });
    }
    if luminous_intensity != 0 {
        terms.push(DimensionTerm {
            axis: SI_LUMINOUS_INTENSITY_AXIS,
            exponent: luminous_intensity,
        });
    }
    DimensionVector::from_terms(terms)
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)
}

fn mass_projection_eligible(dimension: DimensionVector, exact_value: &ExactRationalWire) -> bool {
    dimension == PRODUCER_PURE_MASS_DIMENSION
        && !exact_value.negative
        && exact_value.numerator_be.iter().any(|byte| *byte != 0)
}

/// Build the producer's floor packet by traversing the sealed ledger,
/// definitions, and receipt pins in their declared order.
pub(super) fn sealed_floor_packet() -> Result<RepositoryRootPacket, RepositoryRootRefusalCode> {
    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let authority = sealed_physical_floor_authority_binding()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let definitions = sealed_physical_floor_definitions()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let fingerprints = sealed_physical_floor_receipt_fingerprints()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    if floor.len() != definitions.len() || floor.len() != fingerprints.len() {
        return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
    }

    let mut sources = Vec::with_capacity(floor.len());
    for ((entry, definition), fingerprint) in floor.entries().zip(definitions).zip(fingerprints) {
        let expected_entry_id = format!("fundamental.{}", definition.symbol);
        if entry.id != expected_entry_id
            || fingerprint.entry_id() != entry.id
            || definition.role != FundamentalRole::PhysicalInvariant
            || !entry.inputs.is_empty()
        {
            return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
        }
        sources.push(RepositoryRootSource::new(
            entry.id.clone(),
            definition.symbol,
            definition.value,
            definition.uncertainty.kind_id(),
            definition.uncertainty.decimal(),
            producer_dimension_from_si_exponents(definition.dimension.exponents().map(i16::from))
                .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?,
            entry.tier,
            entry.provenance,
            ReceiptBinding {
                schema_id: fingerprint.schema_id().to_owned(),
                digest_sha256: fingerprint.digest(),
            },
        ));
    }
    Ok(RepositoryRootPacket::new(
        ReceiptBinding {
            schema_id: authority.schema_id().as_str().to_owned(),
            digest_sha256: authority.digest(),
        },
        sources,
    ))
}

pub(super) fn resource_contract_sha256() -> Result<[u8; 32], RepositoryRootRefusalCode> {
    let mut record = Record::new(RESOURCE_CONTRACT_DOMAIN)?;
    record.field(
        1,
        &u64::try_from(PRODUCER_SOURCE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    record.field(
        2,
        &u64::try_from(PRODUCER_TOKEN_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    record.field(
        3,
        &u64::try_from(PRODUCER_DECIMAL_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    record.field(4, &PRODUCER_DECIMAL_EXPONENT_LIMIT.to_be_bytes())?;
    record.field(
        5,
        &u64::try_from(PRODUCER_CANONICAL_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?
            .to_be_bytes(),
    )?;
    record.field(6, &PRODUCER_DIMENSION_EXPONENT_LIMIT.to_be_bytes())?;
    record.field(7, &PRODUCER_RATIONAL_COMPONENT_BIT_LIMIT.to_be_bytes())?;
    record.field(8, &encode_dimension(PRODUCER_PURE_MASS_DIMENSION))?;
    record.field(9, b"physical_invariant")?;
    record.field(10, b"universal")?;
    record.field(11, b"[M]")?;
    record.field(12, b"semantic-source-order-canonicalized")?;
    record.field(13, b"sealed-identity-receipt-binding-required")?;
    record.field(14, b"paired-post-projection-verifier-required")?;
    Ok(sha256(&record.finish()))
}

impl Record {
    fn new(domain: &[u8]) -> Result<Self, RepositoryRootRefusalCode> {
        if domain.len() > PRODUCER_CANONICAL_BYTE_LIMIT {
            return Err(RepositoryRootRefusalCode::CanonicalByteLimitExceeded);
        }
        Ok(Self {
            bytes: domain.to_vec(),
        })
    }

    fn field(&mut self, tag: u16, payload: &[u8]) -> Result<(), RepositoryRootRefusalCode> {
        let next = self
            .bytes
            .len()
            .checked_add(2)
            .and_then(|length| length.checked_add(8))
            .and_then(|length| length.checked_add(payload.len()))
            .ok_or(RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?;
        if next > PRODUCER_CANONICAL_BYTE_LIMIT {
            return Err(RepositoryRootRefusalCode::CanonicalByteLimitExceeded);
        }
        self.bytes.extend_from_slice(&tag.to_be_bytes());
        self.bytes.extend_from_slice(
            &u64::try_from(payload.len())
                .map_err(|_| RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?
                .to_be_bytes(),
        );
        self.bytes.extend_from_slice(payload);
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(super) fn project(
    packet: &RepositoryRootPacket,
) -> Result<RootCheckerOutput, RepositoryRootRefusalCode> {
    let sources = validate_sources(packet)?;
    validate_sealed_packet(packet)?;
    project_validated(packet, sources)
}

#[cfg(test)]
pub(super) fn project_unsealed(
    packet: &RepositoryRootPacket,
) -> Result<RootCheckerOutput, RepositoryRootRefusalCode> {
    let sources = validate_sources(packet)?;
    project_validated(packet, sources)
}

fn project_validated(
    packet: &RepositoryRootPacket,
    sources: Vec<&RepositoryRootSource>,
) -> Result<RootCheckerOutput, RepositoryRootRefusalCode> {
    let input_bytes = encode_input(packet, &sources)?;
    let input_sha256 = sha256(&input_bytes);
    let mut candidates = Vec::with_capacity(sources.len().saturating_mul(2));

    for source in sources {
        let exact_value = parse_decimal_left_to_right(&source.central_decimal)?;
        let uncertainty = parse_decimal_left_to_right(&source.uncertainty_decimal)?;
        if uncertainty.wire.negative {
            return Err(RepositoryRootRefusalCode::NegativeUncertainty);
        }
        let source_bytes = encode_source(source)?;
        let content = CanonicalArtifact {
            schema_id: COORDINATE_CONTENT_SCHEMA_ID.to_owned(),
            canonical_bytes: encode_coordinate_content(
                source,
                &exact_value.wire,
                &uncertainty.wire,
            )?,
        };
        let scalar_payload =
            ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
                coordinate: content,
                exact_value: exact_value.wire.clone(),
                dimension: source.dimension,
            }));
        let scalar_identity = derive_artifact_identity(&scalar_payload)?;
        let scalar_ancestry =
            encode_scalar_ancestry(&packet.floor_authority, &source_bytes, scalar_identity)?;
        candidates.push(RootCandidate {
            source_entry_id: source.entry_id.clone(),
            source_tier: source.source_tier,
            kind: ProjectedRootKind::ScalarCoordinate,
            identity: scalar_identity,
            payload: scalar_payload,
            ancestry_digest_sha256: sha256(&scalar_ancestry),
        });

        if mass_projection_eligible(source.dimension, &exact_value.wire) {
            let mass_payload = ArtifactPayload::MassProjection(MassProjectionArtifact {
                expression: ExactExpression {
                    nodes: vec![ExactExpressionNode::Coordinate(scalar_identity)],
                    output_node: 0,
                },
                scope: MassProjectionScope::MembershipNeutral,
            });
            let mass_identity = derive_artifact_identity(&mass_payload)?;
            let mass_ancestry =
                encode_mass_ancestry(&packet.floor_authority, &source_bytes, scalar_identity)?;
            candidates.push(RootCandidate {
                source_entry_id: source.entry_id.clone(),
                source_tier: source.source_tier,
                kind: ProjectedRootKind::MembershipNeutralMassProjection,
                identity: mass_identity,
                payload: mass_payload,
                ancestry_digest_sha256: sha256(&mass_ancestry),
            });
        }
    }

    candidates.sort_by_key(|candidate| candidate.identity);
    ensure_unique_identities(&candidates)?;
    let scalar_coordinate_count = count_kind(&candidates, ProjectedRootKind::ScalarCoordinate)?;
    let mass_projection_count = count_kind(
        &candidates,
        ProjectedRootKind::MembershipNeutralMassProjection,
    )?;
    let canonical_bytes = encode_projection(
        input_sha256,
        &candidates,
        scalar_coordinate_count,
        mass_projection_count,
    )?;
    Ok(RootCheckerOutput {
        input_sha256,
        candidates,
        canonical_bytes,
        scalar_coordinate_count,
        mass_projection_count,
    })
}

fn validate_sealed_packet(packet: &RepositoryRootPacket) -> Result<(), RepositoryRootRefusalCode> {
    let expected = sealed_floor_packet()?;
    if packet.floor_authority != expected.floor_authority {
        return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
    }
    if packet.sources.len() != expected.sources.len() {
        return Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch);
    }

    let expected_by_id = expected
        .sources
        .iter()
        .map(|source| (source.entry_id.as_str(), source))
        .collect::<std::collections::BTreeMap<_, _>>();
    for source in &packet.sources {
        let Some(sealed) = expected_by_id.get(source.entry_id.as_str()) else {
            return Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch);
        };
        if *source != **sealed {
            return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
        }
    }
    Ok(())
}

pub(super) fn verify_projection(projection: &RepositoryRootProjection) -> bool {
    verify_projection_against_sealed_floor(projection).is_ok()
}

fn verify_projection_against_sealed_floor(
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    let packet = sealed_floor_packet()?;
    let baseline = project(&packet)?;
    let resource_sha256 = resource_contract_sha256()?;
    let expected_canary_sha256 = canary_digest(&packet, &baseline)?;
    let expected_result_sha256 = sha256(&baseline.canonical_bytes);
    let expected_ancestry_manifest_sha256 = ancestry_manifest_sha256(&baseline.candidates)?;
    let receipt = &projection.receipt;

    if receipt.input_sha256 != baseline.input_sha256
        || receipt.result_sha256 != expected_result_sha256
        || receipt.producer_result_sha256 != expected_result_sha256
        || receipt.watchdog_result_sha256 != expected_result_sha256
        || receipt.producer_ancestry_manifest_sha256 != expected_ancestry_manifest_sha256
        || receipt.watchdog_ancestry_manifest_sha256 != expected_ancestry_manifest_sha256
        || receipt.producer_resource_contract_sha256 != resource_sha256
        || receipt.watchdog_resource_contract_sha256 != resource_sha256
        || receipt.canary_sha256 != expected_canary_sha256
        || receipt.scalar_coordinate_count != baseline.scalar_coordinate_count
        || receipt.mass_projection_count != baseline.mass_projection_count
        || projection.checker_candidates != baseline.candidates
        || projection.canonical_bytes != baseline.canonical_bytes
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    verify_projection_core(projection)?;
    if projection.post_projection_canary_sha256 != post_projection_canary_digest(projection)? {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }
    Ok(())
}

pub(super) fn canary_digest(
    packet: &RepositoryRootPacket,
    baseline: &RootCheckerOutput,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::digest(packet, baseline)
}

pub(super) fn post_projection_canary_digest(
    projection: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::projection_digest(projection)
}

#[cfg(test)]
pub(super) fn canary_packet_preimage_sha256_for_test(
    packet: &RepositoryRootPacket,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::packet_preimage_sha256_for_test(packet)
}

#[cfg(test)]
pub(super) fn canary_packet_observation_sha256_for_test(
    id: &str,
    packet: &RepositoryRootPacket,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::packet_observation_sha256_for_test(id, packet)
}

#[cfg(test)]
pub(super) fn canary_projection_preimage_sha256_for_test(
    projection: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::projection_preimage_sha256_for_test(projection)
}

#[cfg(test)]
pub(super) fn canary_projection_observation_sha256_for_test(
    id: &str,
    projection: &RepositoryRootProjection,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::projection_observation_sha256_for_test(id, projection)
}

#[cfg(test)]
pub(super) fn inspect_projection_core_for_test(
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    verify_projection_core(projection)
}

fn verify_projection_core(
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    let receipt = &projection.receipt;
    let ancestry_manifest_sha256 = ancestry_manifest_sha256(&projection.checker_candidates)?;
    if projection.membership_authority
        || !verify_projection_receipt(receipt)
        || sha256(&projection.canonical_bytes) != receipt.result_sha256
        || receipt.producer_ancestry_manifest_sha256 != ancestry_manifest_sha256
        || receipt.watchdog_ancestry_manifest_sha256 != ancestry_manifest_sha256
        || projection.checker_candidates.len() != projection.admitted_artifacts.len()
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    let scalar_coordinate_count = count_kind(
        &projection.checker_candidates,
        ProjectedRootKind::ScalarCoordinate,
    )?;
    let mass_projection_count = count_kind(
        &projection.checker_candidates,
        ProjectedRootKind::MembershipNeutralMassProjection,
    )?;
    if scalar_coordinate_count != receipt.scalar_coordinate_count
        || mass_projection_count != receipt.mass_projection_count
        || encode_projection(
            receipt.input_sha256,
            &projection.checker_candidates,
            scalar_coordinate_count,
            mass_projection_count,
        )? != projection.canonical_bytes
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    ensure_unique_identities(&projection.checker_candidates)?;
    if projection
        .checker_candidates
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(RepositoryRootRefusalCode::ArtifactIdentityCollision);
    }
    let scalar_identities = projection
        .checker_candidates
        .iter()
        .filter_map(|candidate| {
            (candidate.kind == ProjectedRootKind::ScalarCoordinate).then_some(candidate.identity)
        })
        .collect::<BTreeSet<_>>();
    for (candidate, artifact) in projection
        .checker_candidates
        .iter()
        .zip(&projection.admitted_artifacts)
    {
        if candidate.source_tier != LedgerTier::Universal
            || candidate.identity != artifact.claimed_identity
            || candidate.payload != artifact.payload
            || derive_artifact_identity(&artifact.payload)? != artifact.claimed_identity
        {
            return Err(RepositoryRootRefusalCode::ReceiptInvalid);
        }
        match (candidate.kind, &candidate.payload) {
            (ProjectedRootKind::ScalarCoordinate, ArtifactPayload::ScalarCoordinate(_)) => {}
            (
                ProjectedRootKind::MembershipNeutralMassProjection,
                ArtifactPayload::MassProjection(projection),
            ) if projection.scope == MassProjectionScope::MembershipNeutral
                && projection.expression.output_node == 0 =>
            {
                let Some(ExactExpressionNode::Coordinate(identity)) =
                    projection.expression.nodes.first()
                else {
                    return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
                };
                if projection.expression.nodes.len() != 1 || !scalar_identities.contains(identity) {
                    return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
                }
            }
            _ => return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind),
        }

        let AdmissionRoute::Derived(admission) = &artifact.admission.route else {
            return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
        };
        if artifact.admission.tier != LedgerTier::Universal
            || artifact.admission.provenance != ProvenanceMark::Derived
            || artifact.capability_claimed_identity() != artifact.claimed_identity
            || artifact.capability_admission() != &artifact.admission
            || artifact.repository_root_pair_receipt_sha256() != Some(receipt.receipt_sha256)
            || admission.ancestry_receipt.schema_id != REPOSITORY_ROOT_ANCESTRY_SCHEMA_ID
            || admission.ancestry_receipt.digest_sha256 != candidate.ancestry_digest_sha256
            || admission.semantic_checker_receipt.schema_id
                != REPOSITORY_ROOT_PRODUCER_RECEIPT_SCHEMA_ID
            || admission.semantic_checker_receipt.digest_sha256
                != checker_claim_digest(
                    REPOSITORY_ROOT_PRODUCER_ID,
                    receipt.input_sha256,
                    receipt.producer_result_sha256,
                    receipt.receipt_sha256,
                    candidate,
                )?
            || admission.independent_watchdog_receipt.schema_id
                != REPOSITORY_ROOT_WATCHDOG_RECEIPT_SCHEMA_ID
            || admission.independent_watchdog_receipt.digest_sha256
                != checker_claim_digest(
                    REPOSITORY_ROOT_WATCHDOG_ID,
                    receipt.input_sha256,
                    receipt.watchdog_result_sha256,
                    receipt.receipt_sha256,
                    candidate,
                )?
        {
            return Err(RepositoryRootRefusalCode::ReceiptInvalid);
        }
    }
    Ok(())
}

pub(super) fn verify_projection_receipt(receipt: &RepositoryRootProjectionReceipt) -> bool {
    receipt.schema_id == REPOSITORY_ROOT_RECEIPT_SCHEMA_ID
        && receipt.claim_id == REPOSITORY_ROOT_CLAIM_ID
        && receipt.producer_id == REPOSITORY_ROOT_PRODUCER_ID
        && receipt.watchdog_id == REPOSITORY_ROOT_WATCHDOG_ID
        && receipt.input_sha256 != [0; 32]
        && receipt.result_sha256 == receipt.producer_result_sha256
        && receipt.producer_result_sha256 == receipt.watchdog_result_sha256
        && receipt.producer_ancestry_manifest_sha256 == receipt.watchdog_ancestry_manifest_sha256
        && receipt.producer_ancestry_manifest_sha256 != [0; 32]
        && receipt.producer_resource_contract_sha256 == receipt.watchdog_resource_contract_sha256
        && receipt.producer_resource_contract_sha256 != [0; 32]
        && receipt.canary_suite_id == REPOSITORY_ROOT_CANARY_SUITE_ID
        && receipt.canary_sha256 != [0; 32]
        && receipt.scalar_coordinate_count > 0
        && receipt.mass_projection_count <= receipt.scalar_coordinate_count
        && !receipt.membership_authority
        && receipt.decision_id == "agreed_projected"
        && receipt.receipt_sha256 != [0; 32]
        && pair_receipt_digest(receipt) == receipt.receipt_sha256
}

fn pair_receipt_digest(receipt: &RepositoryRootProjectionReceipt) -> [u8; 32] {
    let Ok(mut record) = Record::new(PAIR_RECEIPT_DOMAIN) else {
        return [0; 32];
    };
    for (tag, payload) in [
        (1, receipt.schema_id.as_bytes().to_vec()),
        (2, receipt.claim_id.as_bytes().to_vec()),
        (3, receipt.producer_id.as_bytes().to_vec()),
        (4, receipt.watchdog_id.as_bytes().to_vec()),
        (5, receipt.input_sha256.to_vec()),
        (6, receipt.result_sha256.to_vec()),
        (7, receipt.producer_result_sha256.to_vec()),
        (8, receipt.watchdog_result_sha256.to_vec()),
        (9, receipt.producer_resource_contract_sha256.to_vec()),
        (10, receipt.watchdog_resource_contract_sha256.to_vec()),
        (11, receipt.canary_suite_id.as_bytes().to_vec()),
        (12, receipt.canary_sha256.to_vec()),
        (13, receipt.scalar_coordinate_count.to_be_bytes().to_vec()),
        (14, receipt.mass_projection_count.to_be_bytes().to_vec()),
        (15, vec![u8::from(receipt.membership_authority)]),
        (16, receipt.decision_id.as_bytes().to_vec()),
        (17, receipt.producer_ancestry_manifest_sha256.to_vec()),
        (18, receipt.watchdog_ancestry_manifest_sha256.to_vec()),
    ] {
        if record.field(tag, &payload).is_err() {
            return [0; 32];
        }
    }
    sha256(&record.finish())
}

fn checker_claim_digest(
    checker_id: &str,
    input_sha256: [u8; 32],
    result_sha256: [u8; 32],
    pair_receipt_sha256: [u8; 32],
    candidate: &RootCandidate,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    let mut record = Record::new(CHECKER_CLAIM_DOMAIN)?;
    record.field(1, REPOSITORY_ROOT_PROJECTION_SCHEMA_ID.as_bytes())?;
    record.field(2, checker_id.as_bytes())?;
    record.field(3, &input_sha256)?;
    record.field(4, &result_sha256)?;
    record.field(5, &pair_receipt_sha256)?;
    record.field(6, candidate.kind.id().as_bytes())?;
    record.field(7, &candidate.identity.0)?;
    record.field(8, candidate.source_entry_id.as_bytes())?;
    record.field(9, &[candidate.source_tier.number()])?;
    record.field(10, candidate.source_tier.id().as_bytes())?;
    record.field(11, &candidate.ancestry_digest_sha256)?;
    Ok(sha256(&record.finish()))
}

pub(super) fn ancestry_manifest_sha256(
    candidates: &[RootCandidate],
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    if candidates
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(RepositoryRootRefusalCode::ArtifactIdentityCollision);
    }
    let mut manifest = Record::new(ANCESTRY_MANIFEST_DOMAIN)?;
    manifest.field(
        1,
        &u64::try_from(candidates.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for candidate in candidates {
        if candidate.ancestry_digest_sha256 == [0; 32] {
            return Err(RepositoryRootRefusalCode::MissingBindingDigest);
        }
        let mut entry = Record::new(ANCESTRY_MANIFEST_ENTRY_DOMAIN)?;
        entry.field(1, candidate.kind.id().as_bytes())?;
        entry.field(2, candidate.source_entry_id.as_bytes())?;
        entry.field(3, &[candidate.source_tier.number()])?;
        entry.field(4, candidate.source_tier.id().as_bytes())?;
        entry.field(5, &candidate.identity.0)?;
        entry.field(6, &candidate.ancestry_digest_sha256)?;
        manifest.field(2, &entry.finish())?;
    }
    Ok(sha256(&manifest.finish()))
}

fn validate_sources(
    packet: &RepositoryRootPacket,
) -> Result<Vec<&RepositoryRootSource>, RepositoryRootRefusalCode> {
    if packet.schema_id != REPOSITORY_ROOT_PACKET_SCHEMA_ID {
        return Err(RepositoryRootRefusalCode::SchemaMismatch);
    }
    if packet.membership_authority {
        return Err(RepositoryRootRefusalCode::MembershipAuthorityPresent);
    }
    validate_receipt(&packet.floor_authority)?;
    if packet.sources.is_empty() {
        return Err(RepositoryRootRefusalCode::EmptySourceSet);
    }
    if packet.sources.len() > PRODUCER_SOURCE_LIMIT {
        return Err(RepositoryRootRefusalCode::SourceCapacityExceeded);
    }

    let mut sources = packet.sources.iter().collect::<Vec<_>>();
    let mut entry_ids = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    let mut leaf_fingerprints = BTreeSet::new();
    for source in &sources {
        validate_source(source)?;
        if !entry_ids.insert(source.entry_id.as_str()) {
            return Err(RepositoryRootRefusalCode::DuplicateEntryId);
        }
        if !symbols.insert(source.symbol.as_str()) {
            return Err(RepositoryRootRefusalCode::DuplicateSymbol);
        }
        if !leaf_fingerprints.insert(source.exhaustion_binding.digest_sha256) {
            return Err(RepositoryRootRefusalCode::DuplicateLeafFingerprint);
        }
        if source.exhaustion_binding.digest_sha256 == packet.floor_authority.digest_sha256 {
            return Err(RepositoryRootRefusalCode::DuplicateLeafFingerprint);
        }
    }
    sources.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    Ok(sources)
}

fn validate_source(source: &RepositoryRootSource) -> Result<(), RepositoryRootRefusalCode> {
    for token in [
        source.entry_id.as_str(),
        source.symbol.as_str(),
        source.uncertainty_kind.as_str(),
    ] {
        if !canonical_token(token) {
            return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
        }
    }
    validate_receipt(&source.exhaustion_binding)?;
    if source.role != RepositoryRootSourceRole::PhysicalInvariant
        || source.entry_id != format!("fundamental.{}", source.symbol)
    {
        return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
    }
    if source.source_tier != LedgerTier::Universal {
        return Err(RepositoryRootRefusalCode::SourceTierNotUniversal);
    }
    if source.source_provenance != ProvenanceMark::Measured {
        return Err(RepositoryRootRefusalCode::SourceProvenanceNotMeasured);
    }
    for term in source.dimension.terms() {
        if i32::from(term.exponent).abs() > PRODUCER_DIMENSION_EXPONENT_LIMIT {
            return Err(RepositoryRootRefusalCode::DimensionExponentLimitExceeded);
        }
    }
    parse_decimal_left_to_right(&source.central_decimal)?;
    let uncertainty = parse_decimal_left_to_right(&source.uncertainty_decimal)?;
    if uncertainty.wire.negative {
        return Err(RepositoryRootRefusalCode::NegativeUncertainty);
    }
    Ok(())
}

fn validate_receipt(receipt: &ReceiptBinding) -> Result<(), RepositoryRootRefusalCode> {
    if !canonical_token(&receipt.schema_id) {
        return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
    }
    if receipt.digest_sha256 == [0; 32] {
        return Err(RepositoryRootRefusalCode::MissingBindingDigest);
    }
    Ok(())
}

fn canonical_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= PRODUCER_TOKEN_BYTE_LIMIT
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/' | b'[' | b']')
        })
}

fn parse_decimal_left_to_right(value: &str) -> Result<ParsedDecimal, RepositoryRootRefusalCode> {
    if value.is_empty() || value.len() > PRODUCER_DECIMAL_BYTE_LIMIT || !value.is_ascii() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let bytes = value.as_bytes();
    let (negative, unsigned) = match bytes.first() {
        Some(b'-') => (true, &bytes[1..]),
        Some(b'+') | None => return Err(RepositoryRootRefusalCode::DecimalInvalid),
        _ => (false, bytes),
    };
    if unsigned.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }

    let exponent_position = unsigned.iter().position(|byte| *byte == b'e');
    if unsigned.contains(&b'E')
        || exponent_position.is_some_and(|position| unsigned[position + 1..].contains(&b'e'))
    {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (mantissa, exponent) = match exponent_position {
        Some(position) => (
            &unsigned[..position],
            parse_exponent_left_to_right(&unsigned[position + 1..])?,
        ),
        None => (unsigned, 0),
    };
    if mantissa.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }

    let mut coefficient = BigUint::zero();
    let ten = BigUint::from_u64(10);
    let mut saw_digit = false;
    let mut saw_dot = false;
    let mut digit_before_dot = false;
    let mut digit_after_dot = false;
    let mut fractional_digits = 0_i32;
    for byte in mantissa {
        match *byte {
            b'0'..=b'9' => {
                saw_digit = true;
                if saw_dot {
                    digit_after_dot = true;
                    fractional_digits = fractional_digits
                        .checked_add(1)
                        .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
                } else {
                    digit_before_dot = true;
                }
                coefficient = coefficient
                    .mul(&ten)
                    .add(&BigUint::from_u64(u64::from(*byte - b'0')));
            }
            b'.' if !saw_dot => saw_dot = true,
            _ => return Err(RepositoryRootRefusalCode::DecimalInvalid),
        }
    }
    if !saw_digit || !digit_before_dot || (saw_dot && !digit_after_dot) {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    if negative && coefficient.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }

    let decimal_scale = fractional_digits
        .checked_sub(exponent)
        .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
    let (mut numerator, mut denominator) = if decimal_scale >= 0 {
        (
            coefficient,
            BigUint::ten_pow(
                u32::try_from(decimal_scale)
                    .map_err(|_| RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?,
            ),
        )
    } else {
        (
            coefficient.mul(&BigUint::ten_pow(decimal_scale.unsigned_abs())),
            BigUint::from_u64(1),
        )
    };
    reduce(&mut numerator, &mut denominator)?;
    check_rational_magnitude(&numerator, &denominator)?;
    Ok(ParsedDecimal {
        wire: ExactRationalWire {
            negative,
            numerator_be: encode_biguint_by_division(&numerator)?,
            denominator_be: encode_biguint_by_division(&denominator)?,
        },
    })
}

fn parse_exponent_left_to_right(bytes: &[u8]) -> Result<i32, RepositoryRootRefusalCode> {
    if bytes.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (negative, digits) = match bytes[0] {
        b'-' => (true, &bytes[1..]),
        b'+' => (false, &bytes[1..]),
        _ => (false, bytes),
    };
    if digits.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    if digits.len() > 4 {
        return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
    }
    let mut magnitude = 0_i32;
    for digit in digits {
        if !digit.is_ascii_digit() {
            return Err(RepositoryRootRefusalCode::DecimalInvalid);
        }
        magnitude = magnitude
            .checked_mul(10)
            .and_then(|current| current.checked_add(i32::from(*digit - b'0')))
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
        if magnitude > PRODUCER_DECIMAL_EXPONENT_LIMIT {
            return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
        }
    }
    Ok(if negative { -magnitude } else { magnitude })
}

fn reduce(
    numerator: &mut BigUint,
    denominator: &mut BigUint,
) -> Result<(), RepositoryRootRefusalCode> {
    if denominator.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let divisor = numerator.gcd(denominator);
    if divisor.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (reduced_numerator, numerator_remainder) = numerator.divmod(&divisor);
    let (reduced_denominator, denominator_remainder) = denominator.divmod(&divisor);
    if !numerator_remainder.is_zero() || !denominator_remainder.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    *numerator = reduced_numerator;
    *denominator = reduced_denominator;
    Ok(())
}

fn check_rational_magnitude(
    numerator: &BigUint,
    denominator: &BigUint,
) -> Result<(), RepositoryRootRefusalCode> {
    if numerator.bit_len() > PRODUCER_RATIONAL_COMPONENT_BIT_LIMIT
        || denominator.bit_len() > PRODUCER_RATIONAL_COMPONENT_BIT_LIMIT
    {
        return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
    }
    Ok(())
}

fn encode_biguint_by_division(value: &BigUint) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    if value.is_zero() {
        return Ok(vec![0]);
    }
    let base = BigUint::from_u64(256);
    let mut cursor = value.clone();
    let mut reversed = Vec::new();
    while !cursor.is_zero() {
        let (quotient, remainder) = cursor.divmod(&base);
        let byte = remainder
            .to_u128()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
        reversed.push(byte);
        cursor = quotient;
    }
    reversed.reverse();
    Ok(reversed)
}

fn encode_input(
    packet: &RepositoryRootPacket,
    sources: &[&RepositoryRootSource],
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(INPUT_DOMAIN)?;
    record.field(1, packet.schema_id.as_bytes())?;
    record.field(2, &[u8::from(packet.membership_authority)])?;
    record.field(
        3,
        &u64::try_from(sources.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for source in sources {
        record.field(4, &encode_semantic_source(source)?)?;
    }
    Ok(record.finish())
}

fn encode_source(source: &RepositoryRootSource) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(SOURCE_DOMAIN)?;
    record.field(1, source.role.id().as_bytes())?;
    record.field(2, source.entry_id.as_bytes())?;
    record.field(3, source.symbol.as_bytes())?;
    record.field(4, source.central_decimal.as_bytes())?;
    record.field(5, source.uncertainty_kind.as_bytes())?;
    record.field(6, source.uncertainty_decimal.as_bytes())?;
    record.field(7, &encode_dimension(source.dimension))?;
    record.field(8, source.source_tier.id().as_bytes())?;
    record.field(
        9,
        source
            .source_provenance
            .bracket_tag()
            .ok_or(RepositoryRootRefusalCode::SourceProvenanceNotMeasured)?
            .as_bytes(),
    )?;
    record.field(10, &encode_receipt(&source.exhaustion_binding)?)?;
    Ok(record.finish())
}

fn encode_coordinate_content(
    source: &RepositoryRootSource,
    exact_value: &ExactRationalWire,
    uncertainty: &ExactRationalWire,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(COORDINATE_CONTENT_DOMAIN)?;
    record.field(1, COORDINATE_ROLE.as_bytes())?;
    record.field(2, source.role.id().as_bytes())?;
    record.field(3, source.entry_id.as_bytes())?;
    record.field(4, source.uncertainty_kind.as_bytes())?;
    record.field(5, &encode_rational(exact_value)?)?;
    record.field(6, &encode_rational(uncertainty)?)?;
    record.field(7, &encode_dimension(source.dimension))?;
    record.field(8, &[0])?;
    Ok(record.finish())
}

fn encode_semantic_source(
    source: &RepositoryRootSource,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let exact_value = parse_decimal_left_to_right(&source.central_decimal)?;
    let uncertainty = parse_decimal_left_to_right(&source.uncertainty_decimal)?;
    let mut record = Record::new(COORDINATE_CONTENT_DOMAIN)?;
    record.field(1, source.role.id().as_bytes())?;
    record.field(2, source.entry_id.as_bytes())?;
    record.field(3, source.uncertainty_kind.as_bytes())?;
    record.field(4, &encode_rational(&exact_value.wire)?)?;
    record.field(5, &encode_rational(&uncertainty.wire)?)?;
    record.field(6, &encode_dimension(source.dimension))?;
    Ok(record.finish())
}

fn encode_scalar_ancestry(
    floor_authority: &ReceiptBinding,
    source_bytes: &[u8],
    scalar_identity: ArtifactIdentity,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(SCALAR_ANCESTRY_DOMAIN)?;
    record.field(1, &encode_receipt(floor_authority)?)?;
    record.field(2, source_bytes)?;
    record.field(3, &scalar_identity.0)?;
    Ok(record.finish())
}

fn encode_mass_ancestry(
    floor_authority: &ReceiptBinding,
    source_bytes: &[u8],
    scalar_identity: ArtifactIdentity,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(MASS_ANCESTRY_DOMAIN)?;
    record.field(1, &encode_receipt(floor_authority)?)?;
    record.field(2, source_bytes)?;
    record.field(3, &scalar_identity.0)?;
    record.field(4, b"identity-projection")?;
    record.field(5, b"membership-neutral")?;
    Ok(record.finish())
}

fn encode_receipt(receipt: &ReceiptBinding) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(RECEIPT_DOMAIN)?;
    record.field(1, receipt.schema_id.as_bytes())?;
    record.field(2, &receipt.digest_sha256)?;
    Ok(record.finish())
}

fn encode_rational(value: &ExactRationalWire) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(RATIONAL_DOMAIN)?;
    record.field(1, &[u8::from(value.negative)])?;
    record.field(2, &value.numerator_be)?;
    record.field(3, &value.denominator_be)?;
    Ok(record.finish())
}

fn encode_dimension(dimension: DimensionVector) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(4 + dimension.terms().len() * 34);
    bytes.extend_from_slice(
        &u32::try_from(dimension.terms().len())
            .expect("dimension term storage fits u32")
            .to_be_bytes(),
    );
    for term in dimension.terms() {
        bytes.extend_from_slice(&term.axis.0);
        bytes.extend_from_slice(&term.exponent.to_be_bytes());
    }
    bytes
}

fn derive_artifact_identity(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, RepositoryRootRefusalCode> {
    Ok(ArtifactIdentity(sha256(&encode_artifact_payload(payload)?)))
}

fn encode_artifact_payload(
    payload: &ArtifactPayload,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(ARTIFACT_DOMAIN)?;
    match payload {
        ArtifactPayload::ScalarCoordinate(coordinate) => {
            record.field(1, b"scalar-coordinate")?;
            record.field(2, &encode_content(&coordinate.coordinate)?)?;
            record.field(3, &encode_rational(&coordinate.exact_value)?)?;
            record.field(4, &encode_dimension(coordinate.dimension))?;
        }
        ArtifactPayload::MassProjection(projection) => {
            record.field(1, b"mass-projection")?;
            record.field(2, &encode_identity_expression(&projection.expression)?)?;
            record.field(3, projection.scope.id().as_bytes())?;
        }
        _ => return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind),
    }
    Ok(record.finish())
}

fn encode_content(content: &CanonicalArtifact) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(CONTENT_DOMAIN)?;
    record.field(1, content.schema_id.as_bytes())?;
    record.field(2, &content.canonical_bytes)?;
    Ok(record.finish())
}

fn encode_identity_expression(
    expression: &ExactExpression,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let [ExactExpressionNode::Coordinate(identity)] = expression.nodes.as_slice() else {
        return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
    };
    if expression.output_node != 0 {
        return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
    }
    let mut node = Record::new(EXPRESSION_NODE_DOMAIN)?;
    node.field(1, b"coordinate")?;
    node.field(2, &identity.0)?;
    let node_digest = sha256(&node.finish());

    let mut expression_record = Record::new(EXPRESSION_DOMAIN)?;
    expression_record.field(1, &node_digest)?;
    expression_record.field(2, &1_u32.to_be_bytes())?;
    expression_record.field(3, &0_u32.to_be_bytes())?;
    expression_record.field(4, &node_digest)?;
    Ok(expression_record.finish())
}

fn ensure_unique_identities(candidates: &[RootCandidate]) -> Result<(), RepositoryRootRefusalCode> {
    let mut identities = BTreeSet::new();
    for candidate in candidates {
        if !identities.insert(candidate.identity) {
            return Err(RepositoryRootRefusalCode::ArtifactIdentityCollision);
        }
    }
    Ok(())
}

fn count_kind(
    candidates: &[RootCandidate],
    expected: ProjectedRootKind,
) -> Result<u32, RepositoryRootRefusalCode> {
    u32::try_from(
        candidates
            .iter()
            .filter(|candidate| candidate.kind == expected)
            .count(),
    )
    .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)
}

fn encode_projection(
    input_sha256: [u8; 32],
    candidates: &[RootCandidate],
    scalar_coordinate_count: u32,
    mass_projection_count: u32,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(PROJECTION_DOMAIN)?;
    record.field(1, REPOSITORY_ROOT_PROJECTION_SCHEMA_ID.as_bytes())?;
    record.field(2, &input_sha256)?;
    record.field(3, &[0])?;
    record.field(4, &scalar_coordinate_count.to_be_bytes())?;
    record.field(5, &mass_projection_count.to_be_bytes())?;
    for candidate in candidates {
        record.field(6, &encode_projected_artifact(candidate)?)?;
    }
    Ok(record.finish())
}

fn encode_projected_artifact(
    candidate: &RootCandidate,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut record = Record::new(PROJECTED_ARTIFACT_DOMAIN)?;
    record.field(1, candidate.kind.id().as_bytes())?;
    record.field(2, candidate.source_entry_id.as_bytes())?;
    record.field(3, &candidate.identity.0)?;
    Ok(record.finish())
}
