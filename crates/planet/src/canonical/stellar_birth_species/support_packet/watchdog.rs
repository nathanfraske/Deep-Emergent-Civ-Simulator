//! Independent structural watchdog for the species support packet.
//!
//! This implementation uses ordered maps and sets, right-to-left integer
//! decoding, balanced rational reduction, and independently assembled frames.

use super::model::{
    ArtifactBinding, ConditionedSupportPacket, ExactUnsignedRationalWire, PacketRefusal,
    SpeciesDescriptor, SpeciesDescriptorPacket, SpeciesSupportPacket, SupportDisposition,
    ValidationCaps, CONDITIONED_SUPPORT_SCHEMA_ID, DESCRIPTOR_PACKET_SCHEMA_ID,
    MAX_CANONICAL_BYTES, MAX_CANONICAL_TOKEN_BYTES, MAX_INTERMEDIATE_COMPONENT_BITS,
    MAX_MEMBER_COUNT, MAX_OPERATION_UNITS, MAX_RATIONAL_COMPONENT_BITS, PACKET_SCHEMA_ID,
    PRODUCER_CHECKER_ID, WATCHDOG_CHECKER_ID,
};
use civsim_units::{bignum::BigUint, digest::sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

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
struct ExactFraction {
    top: BigUint,
    bottom: BigUint,
}

struct WorkBudget {
    consumed: u64,
    maximum: u64,
}

impl WorkBudget {
    fn from_caps(caps: ValidationCaps) -> Self {
        Self {
            consumed: 0,
            maximum: caps.operation_units,
        }
    }

    fn spend(&mut self, amount: u64) -> Result<(), PacketRefusal> {
        match self.consumed.checked_add(amount) {
            Some(total) if total <= self.maximum => {
                self.consumed = total;
                Ok(())
            }
            _ => Err(PacketRefusal::OperationLimitExceeded),
        }
    }
}

pub(super) fn validate_and_encode(packet: &SpeciesSupportPacket) -> Result<Vec<u8>, PacketRefusal> {
    validate_and_encode_with_caps(packet, ValidationCaps::PRODUCTION)
}

pub(super) fn validate_and_encode_with_caps(
    packet: &SpeciesSupportPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    check_declared_contract(packet)?;
    check_cardinalities(packet, caps)?;
    let mut budget = WorkBudget::from_caps(caps);

    for binding in [
        &packet.descriptors.floor_binding,
        &packet.descriptors.structure_binding,
        &packet.descriptors.species_registry_binding,
        &packet.descriptors.replay_binding,
    ] {
        check_binding(binding, caps)?;
    }
    check_descriptor_sequence(&packet.descriptors.members)?;
    let mut descriptor_inventory = BTreeMap::new();
    for descriptor in &packet.descriptors.members {
        check_descriptor(descriptor, caps, &mut budget)?;
        if let Some(previous) = descriptor_inventory.insert(descriptor.claimed_identity, descriptor)
        {
            return Err(if previous == descriptor {
                PacketRefusal::DuplicateContentIdentity
            } else {
                PacketRefusal::ContentIdentityCollision
            });
        }
    }

    check_binding(&packet.conditioned_support.joint_measure_binding, caps)?;
    check_binding(&packet.conditioned_support.conditioning_binding, caps)?;
    check_binding(&packet.conditioned_support.replay_binding, caps)?;
    if packet.conditioned_support.replay_binding != packet.descriptors.replay_binding {
        return Err(PacketRefusal::ReplayBindingMismatch);
    }

    let descriptor_bytes = write_descriptor_packet(&packet.descriptors, caps)?;
    if packet.conditioned_support.descriptor_packet_sha256 != sha256(&descriptor_bytes) {
        return Err(PacketRefusal::DescriptorPacketDigestMismatch);
    }

    check_disposition_sequence(&packet.conditioned_support.dispositions)?;
    let mut disposition_inventory = BTreeMap::new();
    let mut weights = Vec::new();
    for disposition in &packet.conditioned_support.dispositions {
        if disposition_inventory
            .insert(disposition.identity(), disposition)
            .is_some()
        {
            return Err(PacketRefusal::DuplicateDisposition);
        }
        match disposition {
            SupportDisposition::Positive {
                number_fraction, ..
            } => {
                let fraction = read_rational(number_fraction, caps, &mut budget)?;
                if fraction.top.is_zero() {
                    return Err(PacketRefusal::NonPositiveSupportWeight);
                }
                weights.push(fraction);
            }
            SupportDisposition::ExactZero {
                zero_derivation, ..
            } => {
                if check_binding(zero_derivation, caps).is_err() {
                    return Err(PacketRefusal::UnprovedExactZero);
                }
            }
        }
    }

    check_complete_coverage(&descriptor_inventory, &disposition_inventory)?;
    check_simplex_balanced(weights, caps, &mut budget)?;

    let support_bytes = write_conditioned_support(&packet.conditioned_support, caps)?;
    write_complete_packet(packet, &descriptor_bytes, &support_bytes, caps)
}

fn check_declared_contract(packet: &SpeciesSupportPacket) -> Result<(), PacketRefusal> {
    let schemas = (
        packet.schema_id.as_str(),
        packet.descriptors.schema_id.as_str(),
        packet.conditioned_support.schema_id.as_str(),
    );
    if schemas
        != (
            PACKET_SCHEMA_ID,
            DESCRIPTOR_PACKET_SCHEMA_ID,
            CONDITIONED_SUPPORT_SCHEMA_ID,
        )
    {
        return Err(PacketRefusal::SchemaMismatch);
    }
    if (
        packet.checker_pair.producer_id.as_str(),
        packet.checker_pair.watchdog_id.as_str(),
    ) != (PRODUCER_CHECKER_ID, WATCHDOG_CHECKER_ID)
    {
        return Err(PacketRefusal::CheckerPairMismatch);
    }
    let found = (
        packet.resources.max_member_count,
        packet.resources.max_rational_component_bits,
        packet.resources.max_intermediate_component_bits,
        packet.resources.max_operation_units,
        packet.resources.max_canonical_bytes,
        packet.resources.max_canonical_token_bytes,
    );
    let expected = (
        MAX_MEMBER_COUNT,
        MAX_RATIONAL_COMPONENT_BITS,
        MAX_INTERMEDIATE_COMPONENT_BITS,
        MAX_OPERATION_UNITS,
        MAX_CANONICAL_BYTES,
        MAX_CANONICAL_TOKEN_BYTES,
    );
    if found != expected {
        return Err(PacketRefusal::ResourceContractMismatch);
    }
    Ok(())
}

fn check_cardinalities(
    packet: &SpeciesSupportPacket,
    caps: ValidationCaps,
) -> Result<(), PacketRefusal> {
    if packet.descriptors.members.is_empty() {
        return Err(PacketRefusal::EmptyRegistry);
    }
    for count in [
        packet.descriptors.members.len(),
        packet.conditioned_support.dispositions.len(),
    ] {
        if usize::try_from(caps.member_count).map_or(true, |limit| count > limit) {
            return Err(PacketRefusal::MemberCapacityExceeded);
        }
    }
    Ok(())
}

fn check_descriptor_sequence(members: &[SpeciesDescriptor]) -> Result<(), PacketRefusal> {
    for index in 1..members.len() {
        let prior = &members[index - 1];
        let current = &members[index];
        if prior.claimed_identity > current.claimed_identity {
            return Err(PacketRefusal::NonCanonicalDescriptorOrder);
        }
        if prior.claimed_identity == current.claimed_identity {
            return Err(if prior == current {
                PacketRefusal::DuplicateContentIdentity
            } else {
                PacketRefusal::ContentIdentityCollision
            });
        }
    }
    Ok(())
}

fn check_disposition_sequence(dispositions: &[SupportDisposition]) -> Result<(), PacketRefusal> {
    for index in 1..dispositions.len() {
        match dispositions[index - 1]
            .identity()
            .cmp(&dispositions[index].identity())
        {
            Ordering::Less => {}
            Ordering::Equal => return Err(PacketRefusal::DuplicateDisposition),
            Ordering::Greater => return Err(PacketRefusal::NonCanonicalDispositionOrder),
        }
    }
    Ok(())
}

fn check_descriptor(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
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
        check_binding(binding, caps)?;
    }
    read_rational(&descriptor.rest_mass_si, caps, budget)?;
    if descriptor_identity_with_budget(descriptor, caps, budget)? != descriptor.claimed_identity {
        return Err(PacketRefusal::ContentIdentityMismatch);
    }
    Ok(())
}

pub(super) fn derive_descriptor_identity(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<super::super::SpeciesContentIdentity, PacketRefusal> {
    let mut budget = WorkBudget::from_caps(caps);
    for binding in [
        &descriptor.physical_content,
        &descriptor.mass_ancestry,
        &descriptor.dimension_ancestry,
        &descriptor.charge_and_state,
        &descriptor.active_sector_set,
        &descriptor.validity_domain,
        &descriptor.dependency_closure,
    ] {
        check_binding(binding, caps)?;
    }
    read_rational(&descriptor.rest_mass_si, caps, &mut budget)?;
    descriptor_identity_with_budget(descriptor, caps, &mut budget)
}

fn descriptor_identity_with_budget(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<super::super::SpeciesContentIdentity, PacketRefusal> {
    budget.spend(1)?;
    let preimage = write_descriptor_content(descriptor, caps)?;
    Ok(super::super::SpeciesContentIdentity(sha256(&preimage)))
}

pub(super) fn descriptor_packet_digest(
    packet: &SpeciesDescriptorPacket,
    caps: ValidationCaps,
) -> Result<[u8; 32], PacketRefusal> {
    Ok(sha256(&write_descriptor_packet(packet, caps)?))
}

fn check_binding(binding: &ArtifactBinding, caps: ValidationCaps) -> Result<(), PacketRefusal> {
    let text = binding.schema_id.as_bytes();
    let allowed = |byte: u8| {
        byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || byte == b'.'
            || byte == b'_'
            || byte == b'-'
    };
    if text.is_empty()
        || u32::try_from(text.len()).map_or(true, |length| length > caps.canonical_token_bytes)
        || (!text[0].is_ascii_lowercase() && !text[0].is_ascii_digit())
        || text.iter().copied().any(|byte| !allowed(byte))
    {
        return Err(PacketRefusal::CanonicalTextInvalid);
    }
    if binding.digest_sha256.iter().all(|byte| *byte == 0) {
        return Err(PacketRefusal::MissingBindingDigest);
    }
    Ok(())
}

fn read_rational(
    wire: &ExactUnsignedRationalWire,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<ExactFraction, PacketRefusal> {
    check_component(&wire.numerator_be, false, caps)?;
    check_component(&wire.denominator_be, true, caps)?;
    let top = read_big_endian_from_tail(&wire.numerator_be, budget)?;
    let bottom = read_big_endian_from_tail(&wire.denominator_be, budget)?;
    check_intermediate(&top, caps)?;
    check_intermediate(&bottom, caps)?;
    budget.spend(1)?;
    let common = top.gcd(&bottom);
    if common.cmp_big(&BigUint::from_u64(1)) != Ordering::Equal {
        return Err(PacketRefusal::RationalNotReduced);
    }
    Ok(ExactFraction { top, bottom })
}

fn check_component(
    bytes: &[u8],
    is_denominator: bool,
    caps: ValidationCaps,
) -> Result<(), PacketRefusal> {
    let Some(first) = bytes.first().copied() else {
        return Err(PacketRefusal::RationalEncodingInvalid);
    };
    if (bytes.len() != 1 && first == 0) || (is_denominator && bytes == [0]) {
        return Err(PacketRefusal::RationalEncodingInvalid);
    }
    let significant = if bytes == [0] {
        0
    } else {
        let full_bytes = u32::try_from(bytes.len() - 1)
            .map_err(|_| PacketRefusal::RationalComponentLimitExceeded)?;
        full_bytes
            .checked_mul(8)
            .and_then(|bits| bits.checked_add(first.ilog2() + 1))
            .ok_or(PacketRefusal::RationalComponentLimitExceeded)?
    };
    if significant > caps.rational_component_bits {
        return Err(PacketRefusal::RationalComponentLimitExceeded);
    }
    Ok(())
}

fn read_big_endian_from_tail(
    bytes: &[u8],
    budget: &mut WorkBudget,
) -> Result<BigUint, PacketRefusal> {
    let mut total = BigUint::zero();
    for (position, byte) in bytes.iter().rev().enumerate() {
        budget.spend(2)?;
        if *byte == 0 {
            continue;
        }
        let shift = u32::try_from(position)
            .ok()
            .and_then(|value| value.checked_mul(8))
            .ok_or(PacketRefusal::RationalComponentLimitExceeded)?;
        let term = BigUint::from_u64(u64::from(*byte)).shl_bits(shift);
        total = total.add(&term);
    }
    Ok(total)
}

fn check_complete_coverage(
    descriptors: &BTreeMap<super::super::SpeciesContentIdentity, &SpeciesDescriptor>,
    dispositions: &BTreeMap<super::super::SpeciesContentIdentity, &SupportDisposition>,
) -> Result<(), PacketRefusal> {
    let descriptor_ids = descriptors.keys().copied().collect::<BTreeSet<_>>();
    let disposition_ids = dispositions.keys().copied().collect::<BTreeSet<_>>();
    let missing = descriptor_ids.difference(&disposition_ids).next().copied();
    let unknown = disposition_ids.difference(&descriptor_ids).next().copied();
    match (missing, unknown) {
        (Some(missing_id), Some(unknown_id)) if missing_id < unknown_id => {
            Err(PacketRefusal::MissingDisposition)
        }
        (Some(_), Some(_)) => Err(PacketRefusal::UnknownDisposition),
        (Some(_), None) => Err(PacketRefusal::MissingDisposition),
        (None, Some(_)) => Err(PacketRefusal::UnknownDisposition),
        (None, None) => Ok(()),
    }
}

fn check_simplex_balanced(
    mut fractions: Vec<ExactFraction>,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<(), PacketRefusal> {
    if fractions.is_empty() {
        return Err(PacketRefusal::NonUnitCompositionSimplex);
    }
    while fractions.len() > 1 {
        let mut next = Vec::with_capacity(fractions.len().div_ceil(2));
        let mut iterator = fractions.into_iter();
        while let Some(left) = iterator.next() {
            if let Some(right) = iterator.next() {
                next.push(add_by_least_common_denominator(left, right, caps, budget)?);
            } else {
                next.push(left);
            }
        }
        fractions = next;
    }
    let total = fractions
        .pop()
        .ok_or(PacketRefusal::NonUnitCompositionSimplex)?;
    if total.top.cmp_big(&total.bottom) != Ordering::Equal {
        return Err(PacketRefusal::NonUnitCompositionSimplex);
    }
    Ok(())
}

fn add_by_least_common_denominator(
    left: ExactFraction,
    right: ExactFraction,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<ExactFraction, PacketRefusal> {
    budget.spend(3)?;
    let common = left.bottom.gcd(&right.bottom);
    let (left_scale, _) = right.bottom.divmod(&common);
    let (right_scale, _) = left.bottom.divmod(&common);

    let left_top = multiply_checked(&left.top, &left_scale, caps, budget)?;
    let right_top = multiply_checked(&right.top, &right_scale, caps, budget)?;
    budget.spend(1)?;
    let top = left_top.add(&right_top);
    check_intermediate(&top, caps)?;
    let bottom = multiply_checked(&left.bottom, &left_scale, caps, budget)?;
    normalize(top, bottom, caps, budget)
}

fn normalize(
    top: BigUint,
    bottom: BigUint,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<ExactFraction, PacketRefusal> {
    budget.spend(3)?;
    let common = top.gcd(&bottom);
    let (top, _) = top.divmod(&common);
    let (bottom, _) = bottom.divmod(&common);
    check_intermediate(&top, caps)?;
    check_intermediate(&bottom, caps)?;
    Ok(ExactFraction { top, bottom })
}

fn multiply_checked(
    left: &BigUint,
    right: &BigUint,
    caps: ValidationCaps,
    budget: &mut WorkBudget,
) -> Result<BigUint, PacketRefusal> {
    budget.spend(1)?;
    if !left.is_zero() && !right.is_zero() {
        let lower_bound = left
            .bit_len()
            .saturating_add(right.bit_len())
            .saturating_sub(1);
        if lower_bound > caps.intermediate_component_bits {
            return Err(PacketRefusal::IntermediateRationalLimitExceeded);
        }
    }
    let result = left.mul(right);
    check_intermediate(&result, caps)?;
    Ok(result)
}

fn check_intermediate(value: &BigUint, caps: ValidationCaps) -> Result<(), PacketRefusal> {
    if value.bit_len() > caps.intermediate_component_bits {
        return Err(PacketRefusal::IntermediateRationalLimitExceeded);
    }
    Ok(())
}

fn new_record(domain: &[u8], caps: ValidationCaps) -> Result<Vec<u8>, PacketRefusal> {
    if domain.len() > caps.canonical_bytes as usize {
        return Err(PacketRefusal::CanonicalByteLimitExceeded);
    }
    Ok(domain.to_vec())
}

fn write_frame(
    target: &mut Vec<u8>,
    tag: u8,
    payload: &[u8],
    caps: ValidationCaps,
) -> Result<(), PacketRefusal> {
    let size =
        u32::try_from(payload.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
    let projected = target
        .len()
        .checked_add(1 + u32::BITS as usize / 8)
        .and_then(|length| length.checked_add(payload.len()))
        .ok_or(PacketRefusal::CanonicalByteLimitExceeded)?;
    if projected > caps.canonical_bytes as usize {
        return Err(PacketRefusal::CanonicalByteLimitExceeded);
    }
    target.push(tag);
    target.extend(size.to_be_bytes());
    target.extend(payload);
    Ok(())
}

fn write_binding(
    binding: &ArtifactBinding,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut bytes = new_record(b"artifact-binding.v1\0", caps)?;
    write_frame(&mut bytes, 1, binding.schema_id.as_bytes(), caps)?;
    write_frame(&mut bytes, 2, binding.digest_sha256.as_slice(), caps)?;
    Ok(bytes)
}

fn write_rational(
    rational: &ExactUnsignedRationalWire,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut bytes = new_record(b"unsigned-rational.v1\0", caps)?;
    write_frame(&mut bytes, 1, rational.numerator_be.as_slice(), caps)?;
    write_frame(&mut bytes, 2, rational.denominator_be.as_slice(), caps)?;
    Ok(bytes)
}

fn write_descriptor_content(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let fields = [
        write_binding(&descriptor.physical_content, caps)?,
        write_rational(&descriptor.rest_mass_si, caps)?,
        write_binding(&descriptor.mass_ancestry, caps)?,
        write_binding(&descriptor.dimension_ancestry, caps)?,
        write_binding(&descriptor.charge_and_state, caps)?,
        write_binding(&descriptor.active_sector_set, caps)?,
        write_binding(&descriptor.validity_domain, caps)?,
        write_binding(&descriptor.dependency_closure, caps)?,
    ];
    let mut bytes = new_record(DESCRIPTOR_CONTENT_DOMAIN, caps)?;
    for (index, field) in fields.iter().enumerate() {
        write_frame(
            &mut bytes,
            u8::try_from(index + 1).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?,
            field,
            caps,
        )?;
    }
    Ok(bytes)
}

fn write_descriptor_record(
    descriptor: &SpeciesDescriptor,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let content = write_descriptor_content(descriptor, caps)?;
    let mut bytes = new_record(DESCRIPTOR_RECORD_DOMAIN, caps)?;
    write_frame(
        &mut bytes,
        1,
        descriptor.claimed_identity.0.as_slice(),
        caps,
    )?;
    write_frame(&mut bytes, 2, &content, caps)?;
    Ok(bytes)
}

fn write_descriptor_packet(
    packet: &SpeciesDescriptorPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut member_bytes = Vec::with_capacity(packet.members.len());
    for member in &packet.members {
        member_bytes.push(write_descriptor_record(member, caps)?);
    }
    let members = write_record_list(&member_bytes, caps)?;
    let fields = [
        packet.schema_id.as_bytes().to_vec(),
        write_binding(&packet.floor_binding, caps)?,
        write_binding(&packet.structure_binding, caps)?,
        write_binding(&packet.species_registry_binding, caps)?,
        write_binding(&packet.replay_binding, caps)?,
        members,
    ];
    let mut bytes = new_record(DESCRIPTOR_PACKET_DOMAIN, caps)?;
    for (index, field) in fields.iter().enumerate() {
        write_frame(
            &mut bytes,
            u8::try_from(index + 1).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?,
            field,
            caps,
        )?;
    }
    Ok(bytes)
}

fn write_disposition(
    disposition: &SupportDisposition,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let (variant, identity, payload) = match disposition {
        SupportDisposition::Positive {
            identity,
            number_fraction,
        } => (1, identity, write_rational(number_fraction, caps)?),
        SupportDisposition::ExactZero {
            identity,
            zero_derivation,
        } => (2, identity, write_binding(zero_derivation, caps)?),
    };
    let mut bytes = new_record(SUPPORT_DISPOSITION_DOMAIN, caps)?;
    write_frame(&mut bytes, 1, &[variant], caps)?;
    write_frame(&mut bytes, 2, identity.0.as_slice(), caps)?;
    write_frame(&mut bytes, 3, &payload, caps)?;
    Ok(bytes)
}

fn write_conditioned_support(
    packet: &ConditionedSupportPacket,
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut disposition_bytes = Vec::with_capacity(packet.dispositions.len());
    for disposition in &packet.dispositions {
        disposition_bytes.push(write_disposition(disposition, caps)?);
    }
    let dispositions = write_record_list(&disposition_bytes, caps)?;
    let fields = [
        packet.schema_id.as_bytes().to_vec(),
        packet.descriptor_packet_sha256.to_vec(),
        write_binding(&packet.joint_measure_binding, caps)?,
        write_binding(&packet.conditioning_binding, caps)?,
        write_binding(&packet.replay_binding, caps)?,
        dispositions,
    ];
    let mut bytes = new_record(CONDITIONED_SUPPORT_DOMAIN, caps)?;
    for (index, field) in fields.iter().enumerate() {
        write_frame(
            &mut bytes,
            u8::try_from(index + 1).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?,
            field,
            caps,
        )?;
    }
    Ok(bytes)
}

fn write_complete_packet(
    packet: &SpeciesSupportPacket,
    descriptor_bytes: &[u8],
    support_bytes: &[u8],
    caps: ValidationCaps,
) -> Result<Vec<u8>, PacketRefusal> {
    let mut checker = new_record(CHECKER_PAIR_DOMAIN, caps)?;
    write_frame(
        &mut checker,
        1,
        packet.checker_pair.producer_id.as_bytes(),
        caps,
    )?;
    write_frame(
        &mut checker,
        2,
        packet.checker_pair.watchdog_id.as_bytes(),
        caps,
    )?;

    let resource_payloads = [
        packet.resources.max_member_count.to_be_bytes().to_vec(),
        packet
            .resources
            .max_rational_component_bits
            .to_be_bytes()
            .to_vec(),
        packet
            .resources
            .max_intermediate_component_bits
            .to_be_bytes()
            .to_vec(),
        packet.resources.max_operation_units.to_be_bytes().to_vec(),
        packet.resources.max_canonical_bytes.to_be_bytes().to_vec(),
        packet
            .resources
            .max_canonical_token_bytes
            .to_be_bytes()
            .to_vec(),
    ];
    let mut resources = new_record(RESOURCE_CONTRACT_DOMAIN, caps)?;
    for (index, payload) in resource_payloads.iter().enumerate() {
        write_frame(
            &mut resources,
            u8::try_from(index + 1).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?,
            payload,
            caps,
        )?;
    }

    let fields = [
        packet.schema_id.as_bytes(),
        checker.as_slice(),
        resources.as_slice(),
        descriptor_bytes,
        support_bytes,
    ];
    let mut bytes = new_record(COMPLETE_PACKET_DOMAIN, caps)?;
    for (index, field) in fields.iter().enumerate() {
        write_frame(
            &mut bytes,
            u8::try_from(index + 1).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?,
            field,
            caps,
        )?;
    }
    Ok(bytes)
}

fn write_record_list(records: &[Vec<u8>], caps: ValidationCaps) -> Result<Vec<u8>, PacketRefusal> {
    let count =
        u32::try_from(records.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
    let mut bytes = count.to_be_bytes().to_vec();
    for record in records {
        let length =
            u32::try_from(record.len()).map_err(|_| PacketRefusal::CanonicalByteLimitExceeded)?;
        let projected = bytes
            .len()
            .checked_add(4)
            .and_then(|size| size.checked_add(record.len()))
            .ok_or(PacketRefusal::CanonicalByteLimitExceeded)?;
        if projected > caps.canonical_bytes as usize {
            return Err(PacketRefusal::CanonicalByteLimitExceeded);
        }
        bytes.extend(length.to_be_bytes());
        bytes.extend(record);
    }
    Ok(bytes)
}
