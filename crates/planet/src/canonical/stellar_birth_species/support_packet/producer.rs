//! Canonical packet producer and structural validator.
//!
//! This implementation uses sorted adjacent scans, a merge join, left-to-right
//! integer decoding, and a left-fold exact sum. The watchdog deliberately uses
//! different algorithms.

use super::model::{
    ArtifactBinding, ConditionedSupportPacket, ExactUnsignedRationalWire, PacketRefusal,
    SpeciesDescriptor, SpeciesDescriptorPacket, SpeciesSupportPacket, SupportDisposition,
    ValidationCaps, CONDITIONED_SUPPORT_SCHEMA_ID, DESCRIPTOR_PACKET_SCHEMA_ID,
    MAX_CANONICAL_BYTES, MAX_CANONICAL_TOKEN_BYTES, MAX_INTERMEDIATE_COMPONENT_BITS,
    MAX_MEMBER_COUNT, MAX_OPERATION_UNITS, MAX_RATIONAL_COMPONENT_BITS, PACKET_SCHEMA_ID,
    PRODUCER_CHECKER_ID, WATCHDOG_CHECKER_ID,
};
use civsim_units::{bignum::BigUint, digest::sha256};
use std::cmp::Ordering;

const DESCRIPTOR_CONTENT_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-species-descriptor-content.v1\0";
const DESCRIPTOR_RECORD_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-species-descriptor-record.v1\0";
const DESCRIPTOR_PACKET_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-species-descriptor-packet.v1\0";
const SUPPORT_DISPOSITION_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-species-support-disposition.v1\0";
const CONDITIONED_SUPPORT_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-conditioned-species-support.v1\0";
const CHECKER_PAIR_DOMAIN: &[u8] = b"civsim.planet.stellar-birth-species-checker-pair.v1\0";
const RESOURCE_CONTRACT_DOMAIN: &[u8] =
    b"civsim.planet.stellar-birth-species-resource-contract.v1\0";
const COMPLETE_PACKET_DOMAIN: &[u8] = b"civsim.planet.stellar-birth-species-support-packet.v1\0";

#[derive(Debug, Clone)]
struct Fraction {
    numerator: BigUint,
    denominator: BigUint,
}

struct OperationMeter {
    used: u64,
    limit: u64,
}

impl OperationMeter {
    fn new(limit: u64) -> Self {
        Self { used: 0, limit }
    }

    fn charge(&mut self, units: u64) -> Result<(), PacketRefusal> {
        self.used = self
            .used
            .checked_add(units)
            .ok_or(PacketRefusal::OperationLimitExceeded)?;
        if self.used > self.limit {
            return Err(PacketRefusal::OperationLimitExceeded);
        }
        Ok(())
    }
}

struct TlvBuilder {
    bytes: Vec<u8>,
    limit: usize,
}

impl TlvBuilder {
    fn with_domain(domain: &[u8], caps: ValidationCaps) -> Result<Self, PacketRefusal> {
        let limit = caps.canonical_bytes as usize;
        if domain.len() > limit {
            return Err(PacketRefusal::CanonicalByteLimitExceeded);
        }
        Ok(Self {
            bytes: domain.to_vec(),
            limit,
        })
    }

    fn field(&mut self, tag: u8, payload: &[u8]) -> Result<(), PacketRefusal> {
        let payload_len =
            u32::try_from(payload.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
        let next_len = self
            .bytes
            .len()
            .checked_add(5)
            .and_then(|value| value.checked_add(payload.len()))
            .ok_or(PacketRefusal::CanonicalByteLimitExceeded)?;
        if next_len > self.limit {
            return Err(PacketRefusal::CanonicalByteLimitExceeded);
        }
        self.bytes.push(tag);
        self.bytes.extend_from_slice(&payload_len.to_be_bytes());
        self.bytes.extend_from_slice(payload);
        Ok(())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

pub(super) fn validate_and_encode(packet: &SpeciesSupportPacket) -> Result<Vec<u8>, PacketRefusal> {
    validate_and_encode_with_caps(packet, ValidationCaps::PRODUCTION)
}

pub(super) fn validate_and_encode_with_caps(
    packet: &SpeciesSupportPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    validate_contract(packet)?;
    preflight_counts(packet, caps)?;
    let mut meter = OperationMeter::new(caps.operation_units);

    validate_binding(&packet.descriptors.floor_binding, caps)?;
    validate_binding(&packet.descriptors.structure_binding, caps)?;
    validate_binding(&packet.descriptors.species_registry_binding, caps)?;
    validate_binding(&packet.descriptors.replay_binding, caps)?;
    validate_descriptor_order(&packet.descriptors.members)?;
    for descriptor in &packet.descriptors.members {
        validate_descriptor(descriptor, caps, &mut meter)?;
    }

    validate_binding(&packet.conditioned_support.joint_measure_binding, caps)?;
    validate_binding(&packet.conditioned_support.conditioning_binding, caps)?;
    validate_binding(&packet.conditioned_support.replay_binding, caps)?;
    if packet.descriptors.replay_binding != packet.conditioned_support.replay_binding {
        return Err(PacketRefusal::ReplayBindingMismatch);
    }

    let descriptor_bytes = encode_descriptor_packet(&packet.descriptors, caps)?;
    if sha256(&descriptor_bytes) != packet.conditioned_support.descriptor_packet_sha256 {
        return Err(PacketRefusal::DescriptorPacketDigestMismatch);
    }

    validate_disposition_order(&packet.conditioned_support.dispositions)?;
    let mut positive_weights = Vec::new();
    for disposition in &packet.conditioned_support.dispositions {
        match disposition {
            SupportDisposition::Positive {
                number_fraction, ..
            } => {
                let fraction = parse_rational(number_fraction, caps, &mut meter)?;
                if fraction.numerator.is_zero() {
                    return Err(PacketRefusal::NonPositiveSupportWeight);
                }
                positive_weights.push(fraction);
            }
            SupportDisposition::ExactZero {
                zero_derivation, ..
            } => {
                if validate_binding(zero_derivation, caps).is_err() {
                    return Err(PacketRefusal::UnprovedExactZero);
                }
            }
        }
    }
    validate_coverage(
        &packet.descriptors.members,
        &packet.conditioned_support.dispositions,
    )?;
    validate_unit_simplex(positive_weights, caps, &mut meter)?;

    let support_bytes = encode_conditioned_support(&packet.conditioned_support, caps)?;
    encode_complete_packet(packet, &descriptor_bytes, &support_bytes, caps)
}

fn validate_contract(packet: &SpeciesSupportPacket) -> Result<(), PacketRefusal> {
    if packet.schema_id != PACKET_SCHEMA_ID
        || packet.descriptors.schema_id != DESCRIPTOR_PACKET_SCHEMA_ID
        || packet.conditioned_support.schema_id != CONDITIONED_SUPPORT_SCHEMA_ID
    {
        return Err(PacketRefusal::SchemaMismatch);
    }
    if packet.checker_pair.producer_id != PRODUCER_CHECKER_ID
        || packet.checker_pair.watchdog_id != WATCHDOG_CHECKER_ID
    {
        return Err(PacketRefusal::CheckerPairMismatch);
    }
    let resources = packet.resources;
    if resources.max_member_count != MAX_MEMBER_COUNT
        || resources.max_rational_component_bits != MAX_RATIONAL_COMPONENT_BITS
        || resources.max_intermediate_component_bits != MAX_INTERMEDIATE_COMPONENT_BITS
        || resources.max_operation_units != MAX_OPERATION_UNITS
        || resources.max_canonical_bytes != MAX_CANONICAL_BYTES
        || resources.max_canonical_token_bytes != MAX_CANONICAL_TOKEN_BYTES
    {
        return Err(PacketRefusal::ResourceContractMismatch);
    }
    Ok(())
}

fn preflight_counts(
    packet: &SpeciesSupportPacket,
    caps: ValidationCaps,
) -> Result<(), PacketRefusal> {
    if packet.descriptors.members.is_empty() {
        return Err(PacketRefusal::EmptyRegistry);
    }
    let member_count = u32::try_from(packet.descriptors.members.len())
        .map_err(|_| PacketRefusal::MemberCapacityExceeded)?;
    let disposition_count = u32::try_from(packet.conditioned_support.dispositions.len())
        .map_err(|_| PacketRefusal::MemberCapacityExceeded)?;
    if member_count > caps.member_count || disposition_count > caps.member_count {
        return Err(PacketRefusal::MemberCapacityExceeded);
    }
    Ok(())
}

fn validate_descriptor_order(members: &[SpeciesDescriptor]) -> Result<(), PacketRefusal> {
    for pair in members.windows(2) {
        match pair[0].claimed_identity.cmp(&pair[1].claimed_identity) {
            Ordering::Less => {}
            Ordering::Greater => return Err(PacketRefusal::NonCanonicalDescriptorOrder),
            Ordering::Equal if pair[0] == pair[1] => {
                return Err(PacketRefusal::DuplicateContentIdentity)
            }
            Ordering::Equal => return Err(PacketRefusal::ContentIdentityCollision),
        }
    }
    Ok(())
}

fn validate_disposition_order(dispositions: &[SupportDisposition]) -> Result<(), PacketRefusal> {
    for pair in dispositions.windows(2) {
        match pair[0].identity().cmp(&pair[1].identity()) {
            Ordering::Less => {}
            Ordering::Equal => return Err(PacketRefusal::DuplicateDisposition),
            Ordering::Greater => return Err(PacketRefusal::NonCanonicalDispositionOrder),
        }
    }
    Ok(())
}

fn validate_descriptor(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<(), PacketRefusal> {
    for binding in [
        &descriptor.physical_content,
        &descriptor.mass_ancestry,
        &descriptor.dimension_ancestry,
        &descriptor.charge_and_state,
        &descriptor.active_sector_set,
        &descriptor.validity_domain,
        &descriptor.dependency_closure,
    ] {
        validate_binding(binding, caps)?;
    }
    parse_rational(&descriptor.rest_mass_si, caps, meter)?;
    if derive_descriptor_identity_with_meter(descriptor, caps, meter)?
        != descriptor.claimed_identity
    {
        return Err(PacketRefusal::ContentIdentityMismatch);
    }
    Ok(())
}

pub(super) fn derive_descriptor_identity(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<super::super::SpeciesContentIdentity, PacketRefusal> {
    let mut meter = OperationMeter::new(caps.operation_units);
    for binding in [
        &descriptor.physical_content,
        &descriptor.mass_ancestry,
        &descriptor.dimension_ancestry,
        &descriptor.charge_and_state,
        &descriptor.active_sector_set,
        &descriptor.validity_domain,
        &descriptor.dependency_closure,
    ] {
        validate_binding(binding, caps)?;
    }
    parse_rational(&descriptor.rest_mass_si, caps, &mut meter)?;
    derive_descriptor_identity_with_meter(descriptor, caps, &mut meter)
}

fn derive_descriptor_identity_with_meter(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<super::super::SpeciesContentIdentity, PacketRefusal> {
    meter.charge(1)?;
    let content = encode_descriptor_content(descriptor, caps)?;
    Ok(super::super::SpeciesContentIdentity(sha256(&content)))
}

pub(super) fn descriptor_packet_digest(
    packet: &SpeciesDescriptorPacket,
    caps: ValidationCaps,
) -> Result<[u8; 32], PacketRefusal> {
    Ok(sha256(&encode_descriptor_packet(packet, caps)?))
}

fn validate_binding(binding: &ArtifactBinding, caps: ValidationCaps) -> Result<(), PacketRefusal> {
    if !canonical_token(&binding.schema_id, caps) {
        return Err(PacketRefusal::CanonicalTextInvalid);
    }
    if binding.digest_sha256 == [0; 32] {
        return Err(PacketRefusal::MissingBindingDigest);
    }
    Ok(())
}

fn canonical_token(value: &str, caps: ValidationCaps) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || u32::try_from(bytes.len()).map_or(true, |length| length > caps.canonical_token_bytes)
    {
        return false;
    }
    let first = bytes[0];
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
    })
}

fn parse_rational(
    wire: &ExactUnsignedRationalWire,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<Fraction, PacketRefusal> {
    validate_component_encoding(&wire.numerator_be, false, caps)?;
    validate_component_encoding(&wire.denominator_be, true, caps)?;
    let numerator = decode_component(&wire.numerator_be, meter)?;
    let denominator = decode_component(&wire.denominator_be, meter)?;
    ensure_intermediate(&numerator, caps)?;
    ensure_intermediate(&denominator, caps)?;
    meter.charge(1)?;
    if numerator.gcd(&denominator) != BigUint::from_u64(1) {
        return Err(PacketRefusal::RationalNotReduced);
    }
    Ok(Fraction {
        numerator,
        denominator,
    })
}

fn validate_component_encoding(
    bytes: &[u8],
    denominator: bool,
    caps: ValidationCaps,
) -> Result<(), PacketRefusal> {
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == 0) || (denominator && bytes == [0]) {
        return Err(PacketRefusal::RationalEncodingInvalid);
    }
    let bit_length = component_bit_length(bytes)?;
    if bit_length > caps.rational_component_bits {
        return Err(PacketRefusal::RationalComponentLimitExceeded);
    }
    Ok(())
}

fn component_bit_length(bytes: &[u8]) -> Result<u32, PacketRefusal> {
    if bytes == [0] {
        return Ok(0);
    }
    let prefix_bits = u32::try_from(bytes.len().saturating_sub(1))
        .ok()
        .and_then(|length| length.checked_mul(8))
        .ok_or(PacketRefusal::RationalComponentLimitExceeded)?;
    prefix_bits
        .checked_add(8 - bytes[0].leading_zeros())
        .ok_or(PacketRefusal::RationalComponentLimitExceeded)
}

fn decode_component(bytes: &[u8], meter: &mut OperationMeter) -> Result<BigUint, PacketRefusal> {
    let mut value = BigUint::zero();
    for byte in bytes {
        meter.charge(2)?;
        value = value.shl_bits(8);
        if *byte != 0 {
            value = value.add(&BigUint::from_u64(u64::from(*byte)));
        }
    }
    Ok(value)
}

fn validate_coverage(
    members: &[SpeciesDescriptor],
    dispositions: &[SupportDisposition],
) -> Result<(), PacketRefusal> {
    let mut member_index = 0;
    let mut disposition_index = 0;
    while member_index < members.len() && disposition_index < dispositions.len() {
        match members[member_index]
            .claimed_identity
            .cmp(&dispositions[disposition_index].identity())
        {
            Ordering::Less => return Err(PacketRefusal::MissingDisposition),
            Ordering::Greater => return Err(PacketRefusal::UnknownDisposition),
            Ordering::Equal => {
                member_index += 1;
                disposition_index += 1;
            }
        }
    }
    if member_index < members.len() {
        return Err(PacketRefusal::MissingDisposition);
    }
    if disposition_index < dispositions.len() {
        return Err(PacketRefusal::UnknownDisposition);
    }
    Ok(())
}

fn validate_unit_simplex(
    weights: Vec<Fraction>,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<(), PacketRefusal> {
    let mut sum = Fraction {
        numerator: BigUint::zero(),
        denominator: BigUint::from_u64(1),
    };
    for weight in weights {
        sum = add_fraction(&sum, &weight, caps, meter)?;
    }
    if sum.numerator != sum.denominator {
        return Err(PacketRefusal::NonUnitCompositionSimplex);
    }
    Ok(())
}

fn add_fraction(
    left: &Fraction,
    right: &Fraction,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<Fraction, PacketRefusal> {
    let left_numerator = bounded_mul(&left.numerator, &right.denominator, caps, meter)?;
    let right_numerator = bounded_mul(&right.numerator, &left.denominator, caps, meter)?;
    meter.charge(1)?;
    let numerator = left_numerator.add(&right_numerator);
    ensure_intermediate(&numerator, caps)?;
    let denominator = bounded_mul(&left.denominator, &right.denominator, caps, meter)?;
    reduce_fraction(numerator, denominator, caps, meter)
}

fn bounded_mul(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<BigUint, PacketRefusal> {
    meter.charge(1)?;
    if !left.is_zero() && !right.is_zero() {
        let guaranteed_bits = left
            .bit_len()
            .saturating_add(right.bit_len())
            .saturating_sub(1);
        if guaranteed_bits > caps.intermediate_component_bits {
            return Err(PacketRefusal::IntermediateRationalLimitExceeded);
        }
    }
    let product = left.mul(right);
    ensure_intermediate(&product, caps)?;
    Ok(product)
}

fn reduce_fraction(
    numerator: BigUint,
    denominator: BigUint,
    caps: ValidationCaps,
    meter: &mut OperationMeter,
) -> Result<Fraction, PacketRefusal> {
    meter.charge(3)?;
    let divisor = numerator.gcd(&denominator);
    let (numerator, _) = numerator.divmod(&divisor);
    let (denominator, _) = denominator.divmod(&divisor);
    ensure_intermediate(&numerator, caps)?;
    ensure_intermediate(&denominator, caps)?;
    Ok(Fraction {
        numerator,
        denominator,
    })
}

fn ensure_intermediate(value: &BigUint, caps: ValidationCaps) -> Result<(), PacketRefusal> {
    if value.bit_len() > caps.intermediate_component_bits {
        return Err(PacketRefusal::IntermediateRationalLimitExceeded);
    }
    Ok(())
}

fn encode_binding(
    binding: &ArtifactBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut output = TlvBuilder::with_domain(b"artifact-binding.v1\0", caps)?;
    output.field(1, binding.schema_id.as_bytes())?;
    output.field(2, &binding.digest_sha256)?;
    Ok(output.finish())
}

fn encode_rational(
    rational: &ExactUnsignedRationalWire,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut output = TlvBuilder::with_domain(b"unsigned-rational.v1\0", caps)?;
    output.field(1, &rational.numerator_be)?;
    output.field(2, &rational.denominator_be)?;
    Ok(output.finish())
}

fn encode_descriptor_content(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut output = TlvBuilder::with_domain(DESCRIPTOR_CONTENT_DOMAIN, caps)?;
    output.field(1, &encode_binding(&descriptor.physical_content, caps)?)?;
    output.field(2, &encode_rational(&descriptor.rest_mass_si, caps)?)?;
    output.field(3, &encode_binding(&descriptor.mass_ancestry, caps)?)?;
    output.field(4, &encode_binding(&descriptor.dimension_ancestry, caps)?)?;
    output.field(5, &encode_binding(&descriptor.charge_and_state, caps)?)?;
    output.field(6, &encode_binding(&descriptor.active_sector_set, caps)?)?;
    output.field(7, &encode_binding(&descriptor.validity_domain, caps)?)?;
    output.field(8, &encode_binding(&descriptor.dependency_closure, caps)?)?;
    Ok(output.finish())
}

fn encode_descriptor_record(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut output = TlvBuilder::with_domain(DESCRIPTOR_RECORD_DOMAIN, caps)?;
    output.field(1, &descriptor.claimed_identity.0)?;
    output.field(2, &encode_descriptor_content(descriptor, caps)?)?;
    Ok(output.finish())
}

fn encode_descriptor_packet(
    packet: &SpeciesDescriptorPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let records = packet
        .members
        .iter()
        .map(|member| encode_descriptor_record(member, caps))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = TlvBuilder::with_domain(DESCRIPTOR_PACKET_DOMAIN, caps)?;
    output.field(1, packet.schema_id.as_bytes())?;
    output.field(2, &encode_binding(&packet.floor_binding, caps)?)?;
    output.field(3, &encode_binding(&packet.structure_binding, caps)?)?;
    output.field(4, &encode_binding(&packet.species_registry_binding, caps)?)?;
    output.field(5, &encode_binding(&packet.replay_binding, caps)?)?;
    output.field(6, &encode_list(&records, caps)?)?;
    Ok(output.finish())
}

fn encode_disposition(
    disposition: &SupportDisposition,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut output = TlvBuilder::with_domain(SUPPORT_DISPOSITION_DOMAIN, caps)?;
    match disposition {
        SupportDisposition::Positive {
            identity,
            number_fraction,
        } => {
            output.field(1, &[1])?;
            output.field(2, &identity.0)?;
            output.field(3, &encode_rational(number_fraction, caps)?)?;
        }
        SupportDisposition::ExactZero {
            identity,
            zero_derivation,
        } => {
            output.field(1, &[2])?;
            output.field(2, &identity.0)?;
            output.field(3, &encode_binding(zero_derivation, caps)?)?;
        }
    }
    Ok(output.finish())
}

fn encode_conditioned_support(
    packet: &ConditionedSupportPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let records = packet
        .dispositions
        .iter()
        .map(|disposition| encode_disposition(disposition, caps))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = TlvBuilder::with_domain(CONDITIONED_SUPPORT_DOMAIN, caps)?;
    output.field(1, packet.schema_id.as_bytes())?;
    output.field(2, &packet.descriptor_packet_sha256)?;
    output.field(3, &encode_binding(&packet.joint_measure_binding, caps)?)?;
    output.field(4, &encode_binding(&packet.conditioning_binding, caps)?)?;
    output.field(5, &encode_binding(&packet.replay_binding, caps)?)?;
    output.field(6, &encode_list(&records, caps)?)?;
    Ok(output.finish())
}

fn encode_complete_packet(
    packet: &SpeciesSupportPacket,
    descriptor_bytes: &[u8],
    support_bytes: &[u8],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut checker = TlvBuilder::with_domain(CHECKER_PAIR_DOMAIN, caps)?;
    checker.field(1, packet.checker_pair.producer_id.as_bytes())?;
    checker.field(2, packet.checker_pair.watchdog_id.as_bytes())?;

    let mut resources = TlvBuilder::with_domain(RESOURCE_CONTRACT_DOMAIN, caps)?;
    resources.field(1, &packet.resources.max_member_count.to_be_bytes())?;
    resources.field(
        2,
        &packet.resources.max_rational_component_bits.to_be_bytes(),
    )?;
    resources.field(
        3,
        &packet
            .resources
            .max_intermediate_component_bits
            .to_be_bytes(),
    )?;
    resources.field(4, &packet.resources.max_operation_units.to_be_bytes())?;
    resources.field(5, &packet.resources.max_canonical_bytes.to_be_bytes())?;
    resources.field(6, &packet.resources.max_canonical_token_bytes.to_be_bytes())?;

    let mut output = TlvBuilder::with_domain(COMPLETE_PACKET_DOMAIN, caps)?;
    output.field(1, packet.schema_id.as_bytes())?;
    output.field(2, &checker.finish())?;
    output.field(3, &resources.finish())?;
    output.field(4, descriptor_bytes)?;
    output.field(5, support_bytes)?;
    Ok(output.finish())
}

fn encode_list(records: &[Vec<u8>], caps: ValidationCaps) -> Result<Vec<u8>, PacketRefusal> {
    let count =
        u32::try_from(records.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
    let mut output = Vec::with_capacity(4);
    output.extend_from_slice(&count.to_be_bytes());
    for record in records {
        let length =
            u32::try_from(record.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
        let next_len = output
            .len()
            .checked_add(4)
            .and_then(|value| value.checked_add(record.len()))
            .ok_or(PacketRefusal::CanonicalByteLimitExceeded)?;
        if next_len > caps.canonical_bytes as usize {
            return Err(PacketRefusal::CanonicalByteLimitExceeded);
        }
        output.extend_from_slice(&length.to_be_bytes());
        output.extend_from_slice(record);
    }
    Ok(output)
}
