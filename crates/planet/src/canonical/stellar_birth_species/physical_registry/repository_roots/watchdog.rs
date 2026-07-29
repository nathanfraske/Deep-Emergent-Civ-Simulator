//! Top-down repository root watchdog.
//!
//! Decimal parsing, source traversal, framing, and integer encoding are
//! intentionally separate from the producer implementation.

mod canary;

use super::super::model::{
    AdmissionRoute, ArtifactIdentity, ArtifactPayload, CanonicalArtifact, DimensionTerm,
    DimensionVector, ExactExpression, ExactExpressionNode, ExactRationalWire,
    MassProjectionArtifact, MassProjectionScope, ReceiptBinding, ScalarCoordinateArtifact,
    SI_AMOUNT_AXIS, SI_CURRENT_AXIS, SI_LENGTH_AXIS, SI_LUMINOUS_INTENSITY_AXIS, SI_MASS_AXIS,
    SI_TEMPERATURE_AXIS, SI_TIME_AXIS,
};
use super::model::*;
use crate::canonical::floor_magnitudes::AuditedFloorView;
use civsim_units::{
    bignum::BigUint,
    digest::sha256,
    physics_floor::{
        sealed_physical_floor_authority_binding, sealed_physical_floor_dimension_columns,
        sealed_physical_floor_receipt_fingerprints,
    },
};
use std::collections::{BTreeMap, BTreeSet};

const WATCHDOG_SOURCE_LIMIT: usize = 64;
const WATCHDOG_TOKEN_BYTE_LIMIT: usize = 192;
const WATCHDOG_DECIMAL_BYTE_LIMIT: usize = 256;
const WATCHDOG_DECIMAL_EXPONENT_LIMIT: i32 = 1_024;
const WATCHDOG_CANONICAL_BYTE_LIMIT: usize = 1_048_576;
const WATCHDOG_DIMENSION_EXPONENT_LIMIT: i32 = 4_096;
const WATCHDOG_RATIONAL_COMPONENT_BIT_LIMIT: u32 = 4_096;
const WATCHDOG_PURE_MASS_DIMENSION: DimensionVector = DimensionVector::one(SI_MASS_AXIS);

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
struct DecodedDecimal {
    wire: ExactRationalWire,
}

struct Wire {
    bytes: Vec<u8>,
    limit: usize,
}

fn watchdog_dimension_from_si_exponents(
    exponents: [i16; 7],
) -> Result<DimensionVector, RepositoryRootRefusalCode> {
    let axes = [
        SI_LENGTH_AXIS,
        SI_MASS_AXIS,
        SI_TIME_AXIS,
        SI_CURRENT_AXIS,
        SI_TEMPERATURE_AXIS,
        SI_AMOUNT_AXIS,
        SI_LUMINOUS_INTENSITY_AXIS,
    ];
    let mut terms = Vec::with_capacity(axes.len());
    for index in (0..axes.len()).rev() {
        let exponent = exponents[index];
        if exponent != 0 {
            terms.push(DimensionTerm {
                axis: axes[index],
                exponent,
            });
        }
    }
    DimensionVector::from_terms(terms)
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)
}

fn mass_projection_eligible(dimension: DimensionVector, exact_value: &ExactRationalWire) -> bool {
    let positive = !exact_value.negative && exact_value.numerator_be.iter().any(|byte| *byte != 0);
    positive && dimension == WATCHDOG_PURE_MASS_DIMENSION
}

/// Build the watchdog's floor packet through an independent keyed join over
/// the sealed ledger, typed execution view, dimensions, and receipt pins.
pub(super) fn sealed_floor_packet() -> Result<RepositoryRootPacket, RepositoryRootRefusalCode> {
    let floor = crate::canonical::sealed_absolute_physics_floor()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let floor_view = AuditedFloorView::from_floor(&floor)
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let authority = sealed_physical_floor_authority_binding()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let fingerprints = sealed_physical_floor_receipt_fingerprints()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
    let dimensions = sealed_physical_floor_dimension_columns()
        .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;

    let mut fingerprints_by_id = fingerprints
        .into_iter()
        .map(|fingerprint| (fingerprint.entry_id(), fingerprint))
        .collect::<BTreeMap<_, _>>();
    let mut dimensions_by_id = dimensions
        .into_iter()
        .map(|column| (column.id().to_owned(), column.dimension()))
        .collect::<BTreeMap<_, _>>();
    let mut sources = Vec::with_capacity(floor.len());
    for entry in floor.entries() {
        let symbol = entry
            .id
            .strip_prefix("fundamental.")
            .ok_or(RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
        let definition = floor_view
            .execution
            .physical_invariant_definition(symbol)
            .ok_or(RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
        let dimension = dimensions_by_id
            .remove(&entry.id)
            .ok_or(RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
        let fingerprint = fingerprints_by_id
            .remove(entry.id.as_str())
            .ok_or(RepositoryRootRefusalCode::SealedFloorPacketMismatch)?;
        if definition.symbol.as_bytes() != symbol.as_bytes()
            || definition.role.id().as_bytes() != b"physical_invariant"
            || definition.dimension != dimension
            || entry.tier.number() != 1
            || entry.tier.id().as_bytes() != b"universal"
            || entry.provenance.bracket_tag() != Some("[M]")
            || entry.provenance.tag().as_bytes() != b"measured"
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
            watchdog_dimension_from_si_exponents(dimension.exponents().map(i16::from))
                .map_err(|_| RepositoryRootRefusalCode::SealedFloorPacketMismatch)?,
            entry.tier,
            entry.provenance,
            ReceiptBinding {
                schema_id: fingerprint.schema_id().to_owned(),
                digest_sha256: fingerprint.digest(),
            },
        ));
    }
    if sources.len() != floor_view.len()
        || !fingerprints_by_id.is_empty()
        || !dimensions_by_id.is_empty()
    {
        return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
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
    let mut wire = Wire::start(RESOURCE_CONTRACT_DOMAIN)?;
    wire.push(
        1,
        &u64::try_from(WATCHDOG_SOURCE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    wire.push(
        2,
        &u64::try_from(WATCHDOG_TOKEN_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    wire.push(
        3,
        &u64::try_from(WATCHDOG_DECIMAL_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    wire.push(4, &WATCHDOG_DECIMAL_EXPONENT_LIMIT.to_be_bytes())?;
    wire.push(
        5,
        &u64::try_from(WATCHDOG_CANONICAL_BYTE_LIMIT)
            .map_err(|_| RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?
            .to_be_bytes(),
    )?;
    wire.push(6, &WATCHDOG_DIMENSION_EXPONENT_LIMIT.to_be_bytes())?;
    wire.push(7, &WATCHDOG_RATIONAL_COMPONENT_BIT_LIMIT.to_be_bytes())?;
    wire.push(8, &write_dimension(WATCHDOG_PURE_MASS_DIMENSION))?;
    wire.push(9, b"physical_invariant")?;
    wire.push(10, b"universal")?;
    wire.push(11, b"[M]")?;
    wire.push(12, b"semantic-source-order-canonicalized")?;
    wire.push(13, b"sealed-identity-receipt-binding-required")?;
    wire.push(14, b"paired-post-projection-verifier-required")?;
    Ok(sha256(&wire.finish()))
}

impl Wire {
    fn start(domain: &[u8]) -> Result<Self, RepositoryRootRefusalCode> {
        if domain.len() > WATCHDOG_CANONICAL_BYTE_LIMIT {
            return Err(RepositoryRootRefusalCode::CanonicalByteLimitExceeded);
        }
        Ok(Self {
            bytes: domain.to_vec(),
            limit: WATCHDOG_CANONICAL_BYTE_LIMIT,
        })
    }

    fn push(&mut self, tag: u16, payload: &[u8]) -> Result<(), RepositoryRootRefusalCode> {
        let payload_length = u64::try_from(payload.len())
            .map_err(|_| RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?;
        let remaining = self
            .limit
            .checked_sub(self.bytes.len())
            .ok_or(RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?;
        let required = 10_usize
            .checked_add(payload.len())
            .ok_or(RepositoryRootRefusalCode::CanonicalByteLimitExceeded)?;
        if required > remaining {
            return Err(RepositoryRootRefusalCode::CanonicalByteLimitExceeded);
        }
        self.bytes.extend_from_slice(&tag.to_be_bytes());
        self.bytes.extend_from_slice(&payload_length.to_be_bytes());
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
    check_sealed_packet(packet)?;
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
    let input_bytes = write_input(packet, &sources)?;
    let input_sha256 = sha256(&input_bytes);
    let mut candidates = Vec::new();

    for source in sources {
        let exact_value = parse_decimal_from_right(&source.central_decimal)?;
        let uncertainty = parse_decimal_from_right(&source.uncertainty_decimal)?;
        if uncertainty.wire.negative {
            return Err(RepositoryRootRefusalCode::NegativeUncertainty);
        }
        let source_wire = write_source(source)?;
        let coordinate = CanonicalArtifact {
            schema_id: COORDINATE_CONTENT_SCHEMA_ID.to_owned(),
            canonical_bytes: write_coordinate_content(
                source,
                &exact_value.wire,
                &uncertainty.wire,
            )?,
        };
        let coordinate_payload =
            ArtifactPayload::ScalarCoordinate(Box::new(ScalarCoordinateArtifact {
                coordinate,
                exact_value: exact_value.wire.clone(),
                dimension: source.dimension,
            }));
        let coordinate_identity = identify_payload(&coordinate_payload)?;
        let coordinate_ancestry =
            write_scalar_ancestry(&packet.floor_authority, &source_wire, coordinate_identity)?;
        candidates.push(RootCandidate {
            source_entry_id: source.entry_id.clone(),
            source_tier: source.source_tier,
            kind: ProjectedRootKind::ScalarCoordinate,
            identity: coordinate_identity,
            payload: coordinate_payload,
            ancestry_digest_sha256: sha256(&coordinate_ancestry),
        });

        if mass_projection_eligible(source.dimension, &exact_value.wire) {
            let projection_payload = ArtifactPayload::MassProjection(MassProjectionArtifact {
                expression: ExactExpression {
                    nodes: vec![ExactExpressionNode::Coordinate(coordinate_identity)],
                    output_node: 0,
                },
                scope: MassProjectionScope::MembershipNeutral,
                uncertainty_transport: None,
            });
            let projection_identity = identify_payload(&projection_payload)?;
            let projection_ancestry =
                write_mass_ancestry(&packet.floor_authority, &source_wire, coordinate_identity)?;
            candidates.push(RootCandidate {
                source_entry_id: source.entry_id.clone(),
                source_tier: source.source_tier,
                kind: ProjectedRootKind::MembershipNeutralMassProjection,
                identity: projection_identity,
                payload: projection_payload,
                ancestry_digest_sha256: sha256(&projection_ancestry),
            });
        }
    }

    candidates.sort_by(|left, right| left.identity.cmp(&right.identity));
    reject_identity_collisions(&candidates)?;
    let scalar_coordinate_count = u32::try_from(
        candidates
            .iter()
            .filter(|candidate| candidate.kind == ProjectedRootKind::ScalarCoordinate)
            .count(),
    )
    .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    let mass_projection_count = u32::try_from(
        candidates
            .iter()
            .filter(|candidate| {
                candidate.kind == ProjectedRootKind::MembershipNeutralMassProjection
            })
            .count(),
    )
    .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    let canonical_bytes = write_projection(
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

fn check_sealed_packet(packet: &RepositoryRootPacket) -> Result<(), RepositoryRootRefusalCode> {
    let expected = sealed_floor_packet()?;
    if packet.floor_authority.schema_id.as_bytes() != expected.floor_authority.schema_id.as_bytes()
        || packet.floor_authority.digest_sha256 != expected.floor_authority.digest_sha256
    {
        return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
    }
    if packet.sources.len() != expected.sources.len() {
        return Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch);
    }

    let mut expected_by_entry = expected
        .sources
        .iter()
        .map(|source| (source.entry_id.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    for source in &packet.sources {
        let Some(sealed) = expected_by_entry.remove(source.entry_id.as_str()) else {
            return Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch);
        };
        if source != sealed {
            return Err(RepositoryRootRefusalCode::SealedFloorPacketMismatch);
        }
    }
    if !expected_by_entry.is_empty() {
        return Err(RepositoryRootRefusalCode::SealedSourceBindingMismatch);
    }
    Ok(())
}

pub(super) fn verify_projection(projection: &RepositoryRootProjection) -> bool {
    inspect_projection(projection).is_ok()
}

pub(super) fn canary_digest(
    packet: &RepositoryRootPacket,
    baseline: &RootCheckerOutput,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    canary::packet_digest(packet, baseline)
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
    inspect_projection_core(projection)
}

fn inspect_projection(
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    let sealed_packet = sealed_floor_packet()?;
    let sealed_projection = project(&sealed_packet)?;
    let sealed_result_sha256 = sha256(&sealed_projection.canonical_bytes);
    let sealed_ancestry_manifest_sha256 = ancestry_manifest_sha256(&sealed_projection.candidates)?;
    let receipt = &projection.receipt;
    if receipt.input_sha256 != sealed_projection.input_sha256
        || receipt.result_sha256 != sealed_result_sha256
        || receipt.producer_result_sha256 != sealed_result_sha256
        || receipt.watchdog_result_sha256 != sealed_result_sha256
        || receipt.producer_ancestry_manifest_sha256 != sealed_ancestry_manifest_sha256
        || receipt.watchdog_ancestry_manifest_sha256 != sealed_ancestry_manifest_sha256
        || receipt.scalar_coordinate_count != sealed_projection.scalar_coordinate_count
        || receipt.mass_projection_count != sealed_projection.mass_projection_count
        || projection.checker_candidates != sealed_projection.candidates
        || projection.canonical_bytes != sealed_projection.canonical_bytes
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    let local_resource_contract_sha256 = resource_contract_sha256()?;
    if receipt.producer_resource_contract_sha256 != local_resource_contract_sha256
        || receipt.watchdog_resource_contract_sha256 != local_resource_contract_sha256
    {
        return Err(RepositoryRootRefusalCode::ResourceContractDisagreement);
    }

    if receipt.canary_sha256 != canary_digest(&sealed_packet, &sealed_projection)? {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }

    inspect_projection_core(projection)?;
    if projection.post_projection_canary_sha256 != post_projection_canary_digest(projection)? {
        return Err(RepositoryRootRefusalCode::CanaryFailure);
    }
    Ok(())
}

fn inspect_projection_core(
    projection: &RepositoryRootProjection,
) -> Result<(), RepositoryRootRefusalCode> {
    let receipt = &projection.receipt;
    let ancestry_manifest_sha256 = ancestry_manifest_sha256(&projection.checker_candidates)?;
    if projection.membership_authority
        || !verify_projection_receipt(receipt)
        || sha256(&projection.canonical_bytes) != receipt.result_sha256
        || receipt.producer_ancestry_manifest_sha256 != ancestry_manifest_sha256
        || receipt.watchdog_ancestry_manifest_sha256 != ancestry_manifest_sha256
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    let candidate_count = u32::try_from(projection.checker_candidates.len())
        .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    let admitted_count = u32::try_from(projection.admitted_artifacts.len())
        .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    if candidate_count != admitted_count {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }
    if projection
        .checker_candidates
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(RepositoryRootRefusalCode::ArtifactIdentityCollision);
    }

    let scalar_coordinate_count = u32::try_from(
        projection
            .checker_candidates
            .iter()
            .filter(|candidate| candidate.kind == ProjectedRootKind::ScalarCoordinate)
            .count(),
    )
    .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    let mass_projection_count = candidate_count
        .checked_sub(scalar_coordinate_count)
        .ok_or(RepositoryRootRefusalCode::UnexpectedArtifactKind)?;
    if scalar_coordinate_count != receipt.scalar_coordinate_count
        || mass_projection_count != receipt.mass_projection_count
        || write_projection(
            receipt.input_sha256,
            &projection.checker_candidates,
            scalar_coordinate_count,
            mass_projection_count,
        )? != projection.canonical_bytes
    {
        return Err(RepositoryRootRefusalCode::ReceiptInvalid);
    }

    let scalar_identities = projection
        .checker_candidates
        .iter()
        .rev()
        .filter_map(|candidate| {
            (candidate.kind == ProjectedRootKind::ScalarCoordinate).then_some(candidate.identity)
        })
        .collect::<BTreeSet<_>>();
    for index in (0..projection.checker_candidates.len()).rev() {
        let candidate = &projection.checker_candidates[index];
        let artifact = &projection.admitted_artifacts[index];
        if candidate.source_tier.number() != 1
            || candidate.source_tier.id().as_bytes() != b"universal"
            || artifact.claimed_identity != candidate.identity
            || artifact.payload != candidate.payload
            || identify_payload(&artifact.payload)? != artifact.claimed_identity
        {
            return Err(RepositoryRootRefusalCode::ReceiptInvalid);
        }
        match (&candidate.payload, candidate.kind) {
            (ArtifactPayload::ScalarCoordinate(_), ProjectedRootKind::ScalarCoordinate) => {}
            (
                ArtifactPayload::MassProjection(mass),
                ProjectedRootKind::MembershipNeutralMassProjection,
            ) => {
                if mass.scope.id().as_bytes() != b"membership-neutral"
                    || mass.expression.output_node != 0
                    || mass.expression.nodes.len() != 1
                {
                    return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
                }
                let Some(ExactExpressionNode::Coordinate(identity)) = mass.expression.nodes.first()
                else {
                    return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
                };
                if !scalar_identities.contains(identity) {
                    return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
                }
            }
            _ => return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind),
        }

        let AdmissionRoute::Derived(admission) = &artifact.admission.route else {
            return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
        };
        if artifact.admission.tier.number() != 1
            || artifact.admission.tier.id().as_bytes() != b"universal"
            || artifact.admission.provenance.bracket_tag() != Some("[D]")
            || artifact.admission.provenance.tag().as_bytes() != b"derived"
            || artifact.capability_claimed_identity().0 != artifact.claimed_identity.0
            || artifact.capability_admission().tier.number() != artifact.admission.tier.number()
            || artifact.capability_admission().provenance.tag()
                != artifact.admission.provenance.tag()
            || artifact.capability_admission().route != artifact.admission.route
            || artifact.repository_root_pair_receipt_sha256() != Some(receipt.receipt_sha256)
            || admission.ancestry_receipt.schema_id.as_bytes()
                != REPOSITORY_ROOT_ANCESTRY_SCHEMA_ID.as_bytes()
            || admission.ancestry_receipt.digest_sha256 != candidate.ancestry_digest_sha256
            || admission.semantic_checker_receipt.schema_id.as_bytes()
                != REPOSITORY_ROOT_PRODUCER_RECEIPT_SCHEMA_ID.as_bytes()
            || admission.semantic_checker_receipt.digest_sha256
                != claim_digest(
                    REPOSITORY_ROOT_PRODUCER_ID,
                    receipt.input_sha256,
                    receipt.producer_result_sha256,
                    receipt.receipt_sha256,
                    candidate,
                )?
            || admission.independent_watchdog_receipt.schema_id.as_bytes()
                != REPOSITORY_ROOT_WATCHDOG_RECEIPT_SCHEMA_ID.as_bytes()
            || admission.independent_watchdog_receipt.digest_sha256
                != claim_digest(
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
    receipt.schema_id.as_bytes() == REPOSITORY_ROOT_RECEIPT_SCHEMA_ID.as_bytes()
        && receipt.claim_id.as_bytes() == REPOSITORY_ROOT_CLAIM_ID.as_bytes()
        && receipt.producer_id.as_bytes() == REPOSITORY_ROOT_PRODUCER_ID.as_bytes()
        && receipt.watchdog_id.as_bytes() == REPOSITORY_ROOT_WATCHDOG_ID.as_bytes()
        && receipt.input_sha256.iter().any(|byte| *byte != 0)
        && receipt.result_sha256 == receipt.watchdog_result_sha256
        && receipt.watchdog_result_sha256 == receipt.producer_result_sha256
        && receipt.watchdog_ancestry_manifest_sha256 == receipt.producer_ancestry_manifest_sha256
        && receipt
            .watchdog_ancestry_manifest_sha256
            .iter()
            .any(|byte| *byte != 0)
        && receipt.watchdog_resource_contract_sha256 == receipt.producer_resource_contract_sha256
        && receipt
            .watchdog_resource_contract_sha256
            .iter()
            .any(|byte| *byte != 0)
        && receipt.canary_suite_id.as_bytes() == REPOSITORY_ROOT_CANARY_SUITE_ID.as_bytes()
        && receipt.canary_sha256.iter().any(|byte| *byte != 0)
        && receipt.scalar_coordinate_count != 0
        && receipt.mass_projection_count <= receipt.scalar_coordinate_count
        && !receipt.membership_authority
        && receipt.decision_id.as_bytes() == b"agreed_projected"
        && receipt.receipt_sha256.iter().any(|byte| *byte != 0)
        && receipt_digest(receipt) == receipt.receipt_sha256
}

fn receipt_digest(receipt: &RepositoryRootProjectionReceipt) -> [u8; 32] {
    let Ok(mut wire) = Wire::start(PAIR_RECEIPT_DOMAIN) else {
        return [0; 32];
    };
    let payloads: [(u16, Vec<u8>); 18] = [
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
    ];
    for (tag, payload) in payloads {
        if wire.push(tag, &payload).is_err() {
            return [0; 32];
        }
    }
    sha256(&wire.finish())
}

fn claim_digest(
    checker_id: &str,
    input_sha256: [u8; 32],
    result_sha256: [u8; 32],
    pair_receipt_sha256: [u8; 32],
    candidate: &RootCandidate,
) -> Result<[u8; 32], RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CHECKER_CLAIM_DOMAIN)?;
    wire.push(1, REPOSITORY_ROOT_PROJECTION_SCHEMA_ID.as_bytes())?;
    wire.push(2, checker_id.as_bytes())?;
    wire.push(3, &input_sha256)?;
    wire.push(4, &result_sha256)?;
    wire.push(5, &pair_receipt_sha256)?;
    wire.push(6, candidate.kind.id().as_bytes())?;
    wire.push(7, &candidate.identity.0)?;
    wire.push(8, candidate.source_entry_id.as_bytes())?;
    wire.push(9, &[candidate.source_tier.number()])?;
    wire.push(10, candidate.source_tier.id().as_bytes())?;
    wire.push(11, &candidate.ancestry_digest_sha256)?;
    Ok(sha256(&wire.finish()))
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
    let mut manifest = Wire::start(ANCESTRY_MANIFEST_DOMAIN)?;
    let count = u64::try_from(candidates.len())
        .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    manifest.push(1, &count.to_be_bytes())?;
    for candidate in candidates {
        if !candidate
            .ancestry_digest_sha256
            .iter()
            .any(|byte| *byte != 0)
        {
            return Err(RepositoryRootRefusalCode::MissingBindingDigest);
        }
        let mut entry = Wire::start(ANCESTRY_MANIFEST_ENTRY_DOMAIN)?;
        entry.push(1, candidate.kind.id().as_bytes())?;
        entry.push(2, candidate.source_entry_id.as_bytes())?;
        entry.push(3, &[candidate.source_tier.number()])?;
        entry.push(4, candidate.source_tier.id().as_bytes())?;
        entry.push(5, &candidate.identity.0)?;
        entry.push(6, &candidate.ancestry_digest_sha256)?;
        manifest.push(2, &entry.finish())?;
    }
    Ok(sha256(&manifest.finish()))
}

fn validate_sources(
    packet: &RepositoryRootPacket,
) -> Result<Vec<&RepositoryRootSource>, RepositoryRootRefusalCode> {
    if packet.schema_id.as_bytes() != REPOSITORY_ROOT_PACKET_SCHEMA_ID.as_bytes() {
        return Err(RepositoryRootRefusalCode::SchemaMismatch);
    }
    if packet.membership_authority {
        return Err(RepositoryRootRefusalCode::MembershipAuthorityPresent);
    }
    check_receipt(&packet.floor_authority)?;
    let count = u32::try_from(packet.sources.len())
        .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?;
    if count == 0 {
        return Err(RepositoryRootRefusalCode::EmptySourceSet);
    }
    if usize::try_from(count).map_or(true, |count| count > WATCHDOG_SOURCE_LIMIT) {
        return Err(RepositoryRootRefusalCode::SourceCapacityExceeded);
    }

    let mut entry_ids = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    let mut leaf_fingerprints = BTreeSet::new();
    for source in &packet.sources {
        check_source(source)?;
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
    let mut sources = packet.sources.iter().collect::<Vec<_>>();
    sources.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    Ok(sources)
}

fn check_source(source: &RepositoryRootSource) -> Result<(), RepositoryRootRefusalCode> {
    if !is_token(&source.entry_id)
        || !is_token(&source.symbol)
        || !is_token(&source.uncertainty_kind)
    {
        return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
    }
    check_receipt(&source.exhaustion_binding)?;
    let expected_entry_id = ["fundamental.", source.symbol.as_str()].concat();
    if source.role.id().as_bytes() != b"physical_invariant"
        || source.entry_id.as_bytes() != expected_entry_id.as_bytes()
    {
        return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
    }
    if source.source_tier.number() != 1 || source.source_tier.id().as_bytes() != b"universal" {
        return Err(RepositoryRootRefusalCode::SourceTierNotUniversal);
    }
    if source.source_provenance.bracket_tag() != Some("[M]")
        || source.source_provenance.tag().as_bytes() != b"measured"
    {
        return Err(RepositoryRootRefusalCode::SourceProvenanceNotMeasured);
    }
    if source
        .dimension
        .terms()
        .iter()
        .any(|term| i32::from(term.exponent).abs() > WATCHDOG_DIMENSION_EXPONENT_LIMIT)
    {
        return Err(RepositoryRootRefusalCode::DimensionExponentLimitExceeded);
    }
    parse_decimal_from_right(&source.central_decimal)?;
    if parse_decimal_from_right(&source.uncertainty_decimal)?
        .wire
        .negative
    {
        return Err(RepositoryRootRefusalCode::NegativeUncertainty);
    }
    Ok(())
}

fn check_receipt(receipt: &ReceiptBinding) -> Result<(), RepositoryRootRefusalCode> {
    if receipt.digest_sha256.iter().all(|byte| *byte == 0) {
        return Err(RepositoryRootRefusalCode::MissingBindingDigest);
    }
    if !is_token(&receipt.schema_id) {
        return Err(RepositoryRootRefusalCode::CanonicalTextInvalid);
    }
    Ok(())
}

fn is_token(value: &str) -> bool {
    let Ok(length) = u32::try_from(value.len()) else {
        return false;
    };
    length > 0
        && usize::try_from(length).is_ok_and(|length| length <= WATCHDOG_TOKEN_BYTE_LIMIT)
        && value.as_bytes().iter().copied().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/' | b'[' | b']')
        })
}

fn parse_decimal_from_right(value: &str) -> Result<DecodedDecimal, RepositoryRootRefusalCode> {
    let length = value.len();
    if length == 0 || length > WATCHDOG_DECIMAL_BYTE_LIMIT || !value.is_ascii() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let bytes = value.as_bytes();
    let (negative, unsigned) = match bytes.first().copied() {
        Some(b'-') => (true, &bytes[1..]),
        Some(b'+') | None => return Err(RepositoryRootRefusalCode::DecimalInvalid),
        _ => (false, bytes),
    };
    if unsigned.is_empty() || unsigned.contains(&b'E') {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let exponent_markers = unsigned
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'e').then_some(index))
        .collect::<Vec<_>>();
    if exponent_markers.len() > 1 {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (mantissa, exponent) = match exponent_markers.first().copied() {
        Some(index) => (
            &unsigned[..index],
            parse_exponent_from_right(&unsigned[index + 1..])?,
        ),
        None => (unsigned, 0),
    };
    if mantissa.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }

    let dots = mantissa
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'.').then_some(index))
        .collect::<Vec<_>>();
    if dots.len() > 1 {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let fractional_digits = match dots.first().copied() {
        Some(index) => {
            if index == 0 || index + 1 == mantissa.len() {
                return Err(RepositoryRootRefusalCode::DecimalInvalid);
            }
            i32::try_from(mantissa.len() - index - 1)
                .map_err(|_| RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?
        }
        None => 0,
    };

    let ten = BigUint::from_u64(10);
    let mut place = BigUint::from_u64(1);
    let mut coefficient = BigUint::zero();
    let mut digit_count = 0_usize;
    for byte in mantissa.iter().rev().copied() {
        if byte == b'.' {
            continue;
        }
        if !byte.is_ascii_digit() {
            return Err(RepositoryRootRefusalCode::DecimalInvalid);
        }
        let contribution = place.mul(&BigUint::from_u64(u64::from(byte - b'0')));
        coefficient = coefficient.add(&contribution);
        place = place.mul(&ten);
        digit_count = digit_count
            .checked_add(1)
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
    }
    if digit_count == 0 || (negative && coefficient.is_zero()) {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }

    let scale = fractional_digits
        .checked_sub(exponent)
        .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
    let mut numerator;
    let mut denominator;
    if scale < 0 {
        numerator = coefficient.mul(&BigUint::ten_pow(scale.unsigned_abs()));
        denominator = BigUint::from_u64(1);
    } else {
        numerator = coefficient;
        denominator = BigUint::ten_pow(
            u32::try_from(scale)
                .map_err(|_| RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?,
        );
    }
    normalize_fraction(&mut numerator, &mut denominator)?;
    if numerator.bit_len() > WATCHDOG_RATIONAL_COMPONENT_BIT_LIMIT
        || denominator.bit_len() > WATCHDOG_RATIONAL_COMPONENT_BIT_LIMIT
    {
        return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
    }
    Ok(DecodedDecimal {
        wire: ExactRationalWire {
            negative,
            numerator_be: encode_biguint_by_words(&numerator)?,
            denominator_be: encode_biguint_by_words(&denominator)?,
        },
    })
}

fn parse_exponent_from_right(bytes: &[u8]) -> Result<i32, RepositoryRootRefusalCode> {
    if bytes.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (negative, digits) = match bytes.first().copied() {
        Some(b'-') => (true, &bytes[1..]),
        Some(b'+') => (false, &bytes[1..]),
        Some(_) => (false, bytes),
        None => return Err(RepositoryRootRefusalCode::DecimalInvalid),
    };
    if digits.is_empty() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    if digits.len() > 4 {
        return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
    }
    let mut magnitude = 0_i32;
    let mut place = 1_i32;
    for digit in digits.iter().rev().copied() {
        if !digit.is_ascii_digit() {
            return Err(RepositoryRootRefusalCode::DecimalInvalid);
        }
        magnitude = magnitude
            .checked_add(
                i32::from(digit - b'0')
                    .checked_mul(place)
                    .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?,
            )
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
        if magnitude > WATCHDOG_DECIMAL_EXPONENT_LIMIT {
            return Err(RepositoryRootRefusalCode::DecimalMagnitudeExceeded);
        }
        place = place
            .checked_mul(10)
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
    }
    Ok(if negative { -magnitude } else { magnitude })
}

fn normalize_fraction(
    numerator: &mut BigUint,
    denominator: &mut BigUint,
) -> Result<(), RepositoryRootRefusalCode> {
    if denominator.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let common = numerator.gcd(denominator);
    if common.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    let (top, top_remainder) = numerator.divmod(&common);
    let (bottom, bottom_remainder) = denominator.divmod(&common);
    if !top_remainder.is_zero() || !bottom_remainder.is_zero() {
        return Err(RepositoryRootRefusalCode::DecimalInvalid);
    }
    *numerator = top;
    *denominator = bottom;
    Ok(())
}

fn encode_biguint_by_words(value: &BigUint) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    if value.is_zero() {
        return Ok(vec![0]);
    }
    let radix = BigUint::from_u64(65_536);
    let mut cursor = value.clone();
    let mut words = Vec::<u16>::new();
    while !cursor.is_zero() {
        let (quotient, remainder) = cursor.divmod(&radix);
        let word = remainder
            .to_u128()
            .and_then(|value| u16::try_from(value).ok())
            .ok_or(RepositoryRootRefusalCode::DecimalMagnitudeExceeded)?;
        words.push(word);
        cursor = quotient;
    }
    words.reverse();
    let mut bytes = Vec::with_capacity(words.len().saturating_mul(2));
    for (index, word) in words.into_iter().enumerate() {
        let encoded = word.to_be_bytes();
        if index == 0 && encoded[0] == 0 {
            bytes.push(encoded[1]);
        } else {
            bytes.extend_from_slice(&encoded);
        }
    }
    Ok(bytes)
}

fn write_input(
    packet: &RepositoryRootPacket,
    sources: &[&RepositoryRootSource],
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(INPUT_DOMAIN)?;
    wire.push(1, packet.schema_id.as_bytes())?;
    wire.push(2, &[u8::from(packet.membership_authority)])?;
    wire.push(
        3,
        &u64::try_from(sources.len())
            .map_err(|_| RepositoryRootRefusalCode::SourceCapacityExceeded)?
            .to_be_bytes(),
    )?;
    for source in sources {
        wire.push(4, &write_semantic_source(source)?)?;
    }
    Ok(wire.finish())
}

fn write_source(source: &RepositoryRootSource) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(SOURCE_DOMAIN)?;
    wire.push(1, source.role.id().as_bytes())?;
    wire.push(2, source.entry_id.as_bytes())?;
    wire.push(3, source.symbol.as_bytes())?;
    wire.push(4, source.central_decimal.as_bytes())?;
    wire.push(5, source.uncertainty_kind.as_bytes())?;
    wire.push(6, source.uncertainty_decimal.as_bytes())?;
    wire.push(7, &write_dimension(source.dimension))?;
    wire.push(8, source.source_tier.id().as_bytes())?;
    wire.push(
        9,
        source
            .source_provenance
            .bracket_tag()
            .ok_or(RepositoryRootRefusalCode::SourceProvenanceNotMeasured)?
            .as_bytes(),
    )?;
    wire.push(10, &write_receipt(&source.exhaustion_binding)?)?;
    Ok(wire.finish())
}

fn write_coordinate_content(
    source: &RepositoryRootSource,
    exact_value: &ExactRationalWire,
    uncertainty: &ExactRationalWire,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(COORDINATE_CONTENT_DOMAIN)?;
    wire.push(1, COORDINATE_ROLE.as_bytes())?;
    wire.push(2, source.role.id().as_bytes())?;
    wire.push(3, source.entry_id.as_bytes())?;
    wire.push(4, source.uncertainty_kind.as_bytes())?;
    wire.push(5, &write_rational(exact_value)?)?;
    wire.push(6, &write_rational(uncertainty)?)?;
    wire.push(7, &write_dimension(source.dimension))?;
    wire.push(8, &[0])?;
    Ok(wire.finish())
}

fn write_semantic_source(
    source: &RepositoryRootSource,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let exact_value = parse_decimal_from_right(&source.central_decimal)?;
    let uncertainty = parse_decimal_from_right(&source.uncertainty_decimal)?;
    let mut wire = Wire::start(COORDINATE_CONTENT_DOMAIN)?;
    wire.push(1, source.role.id().as_bytes())?;
    wire.push(2, source.entry_id.as_bytes())?;
    wire.push(3, source.uncertainty_kind.as_bytes())?;
    wire.push(4, &write_rational(&exact_value.wire)?)?;
    wire.push(5, &write_rational(&uncertainty.wire)?)?;
    wire.push(6, &write_dimension(source.dimension))?;
    Ok(wire.finish())
}

fn write_scalar_ancestry(
    floor_authority: &ReceiptBinding,
    source_wire: &[u8],
    coordinate_identity: ArtifactIdentity,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(SCALAR_ANCESTRY_DOMAIN)?;
    wire.push(1, &write_receipt(floor_authority)?)?;
    wire.push(2, source_wire)?;
    wire.push(3, &coordinate_identity.0)?;
    Ok(wire.finish())
}

fn write_mass_ancestry(
    floor_authority: &ReceiptBinding,
    source_wire: &[u8],
    coordinate_identity: ArtifactIdentity,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(MASS_ANCESTRY_DOMAIN)?;
    wire.push(1, &write_receipt(floor_authority)?)?;
    wire.push(2, source_wire)?;
    wire.push(3, &coordinate_identity.0)?;
    wire.push(4, b"identity-projection")?;
    wire.push(5, b"membership-neutral")?;
    Ok(wire.finish())
}

fn write_receipt(receipt: &ReceiptBinding) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(RECEIPT_DOMAIN)?;
    wire.push(1, receipt.schema_id.as_bytes())?;
    wire.push(2, &receipt.digest_sha256)?;
    Ok(wire.finish())
}

fn write_rational(value: &ExactRationalWire) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(RATIONAL_DOMAIN)?;
    wire.push(1, &[u8::from(value.negative)])?;
    wire.push(2, &value.numerator_be)?;
    wire.push(3, &value.denominator_be)?;
    Ok(wire.finish())
}

fn write_dimension(dimension: DimensionVector) -> Vec<u8> {
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

fn identify_payload(
    payload: &ArtifactPayload,
) -> Result<ArtifactIdentity, RepositoryRootRefusalCode> {
    Ok(ArtifactIdentity(sha256(&write_payload(payload)?)))
}

fn write_payload(payload: &ArtifactPayload) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(ARTIFACT_DOMAIN)?;
    match payload {
        ArtifactPayload::ScalarCoordinate(coordinate) => {
            wire.push(1, b"scalar-coordinate")?;
            wire.push(2, &write_content(&coordinate.coordinate)?)?;
            wire.push(3, &write_rational(&coordinate.exact_value)?)?;
            wire.push(4, &write_dimension(coordinate.dimension))?;
        }
        ArtifactPayload::MassProjection(projection) => {
            wire.push(1, b"mass-projection")?;
            wire.push(2, &write_identity_expression(&projection.expression)?)?;
            wire.push(3, projection.scope.id().as_bytes())?;
        }
        _ => return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind),
    }
    Ok(wire.finish())
}

fn write_content(content: &CanonicalArtifact) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(CONTENT_DOMAIN)?;
    wire.push(1, content.schema_id.as_bytes())?;
    wire.push(2, &content.canonical_bytes)?;
    Ok(wire.finish())
}

fn write_identity_expression(
    expression: &ExactExpression,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    if expression.output_node != 0 || expression.nodes.len() != 1 {
        return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind);
    }
    let identity = match expression.nodes.first() {
        Some(ExactExpressionNode::Coordinate(identity)) => *identity,
        _ => return Err(RepositoryRootRefusalCode::UnexpectedArtifactKind),
    };
    let mut node = Wire::start(EXPRESSION_NODE_DOMAIN)?;
    node.push(1, b"coordinate")?;
    node.push(2, &identity.0)?;
    let node_digest = sha256(&node.finish());

    let mut wire = Wire::start(EXPRESSION_DOMAIN)?;
    wire.push(1, &node_digest)?;
    wire.push(2, &1_u32.to_be_bytes())?;
    wire.push(3, &0_u32.to_be_bytes())?;
    wire.push(4, &node_digest)?;
    Ok(wire.finish())
}

fn reject_identity_collisions(
    candidates: &[RootCandidate],
) -> Result<(), RepositoryRootRefusalCode> {
    for pair in candidates.windows(2) {
        if pair[0].identity == pair[1].identity {
            return Err(RepositoryRootRefusalCode::ArtifactIdentityCollision);
        }
    }
    Ok(())
}

fn write_projection(
    input_sha256: [u8; 32],
    candidates: &[RootCandidate],
    scalar_coordinate_count: u32,
    mass_projection_count: u32,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(PROJECTION_DOMAIN)?;
    wire.push(1, REPOSITORY_ROOT_PROJECTION_SCHEMA_ID.as_bytes())?;
    wire.push(2, &input_sha256)?;
    wire.push(3, &[0])?;
    wire.push(4, &scalar_coordinate_count.to_be_bytes())?;
    wire.push(5, &mass_projection_count.to_be_bytes())?;
    for candidate in candidates {
        wire.push(6, &write_projected_artifact(candidate)?)?;
    }
    Ok(wire.finish())
}

fn write_projected_artifact(
    candidate: &RootCandidate,
) -> Result<Vec<u8>, RepositoryRootRefusalCode> {
    let mut wire = Wire::start(PROJECTED_ARTIFACT_DOMAIN)?;
    wire.push(1, candidate.kind.id().as_bytes())?;
    wire.push(2, candidate.source_entry_id.as_bytes())?;
    wire.push(3, &candidate.identity.0)?;
    Ok(wire.finish())
}
