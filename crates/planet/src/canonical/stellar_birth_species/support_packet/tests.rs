use super::{
    inspect_packet,
    model::{
        ArtifactBinding, CheckerPairBinding, ConditionedSupportPacket, ExactUnsignedRationalWire,
        PacketRefusal, PacketResourceContract, SpeciesDescriptor, SpeciesDescriptorPacket,
        SpeciesSupportPacket, SupportDisposition, ValidationCaps, CONDITIONED_SUPPORT_SCHEMA_ID,
        DESCRIPTOR_PACKET_SCHEMA_ID, MAX_CANONICAL_BYTES, MAX_CANONICAL_TOKEN_BYTES,
        MAX_INTERMEDIATE_COMPONENT_BITS, MAX_MEMBER_COUNT, MAX_OPERATION_UNITS,
        MAX_RATIONAL_COMPONENT_BITS, PACKET_SCHEMA_ID, PRODUCER_CHECKER_ID, WATCHDOG_CHECKER_ID,
    },
    producer, watchdog,
};
use crate::canonical::stellar_birth_species::SpeciesContentIdentity;

fn magnitude(value: u128) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }
    let bytes = value.to_be_bytes();
    bytes[bytes.iter().position(|byte| *byte != 0).unwrap()..].to_vec()
}

fn ratio(numerator: u128, denominator: u128) -> ExactUnsignedRationalWire {
    ExactUnsignedRationalWire {
        numerator_be: magnitude(numerator),
        denominator_be: magnitude(denominator),
    }
}

fn binding(tag: u8) -> ArtifactBinding {
    ArtifactBinding {
        schema_id: format!("synthetic.binding.{tag}"),
        digest_sha256: [tag; 32],
    }
}

fn resources() -> PacketResourceContract {
    PacketResourceContract {
        max_member_count: MAX_MEMBER_COUNT,
        max_rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        max_intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        max_operation_units: MAX_OPERATION_UNITS,
        max_canonical_bytes: MAX_CANONICAL_BYTES,
        max_canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
    }
}

fn descriptor(tag: u8, rest_mass: ExactUnsignedRationalWire) -> SpeciesDescriptor {
    let mut descriptor = SpeciesDescriptor {
        claimed_identity: SpeciesContentIdentity([0; 32]),
        physical_content: binding(tag),
        rest_mass_si: rest_mass,
        mass_ancestry: binding(tag.wrapping_add(1)),
        dimension_ancestry: binding(tag.wrapping_add(2)),
        charge_and_state: binding(tag.wrapping_add(3)),
        active_sector_set: binding(tag.wrapping_add(4)),
        validity_domain: binding(tag.wrapping_add(5)),
        dependency_closure: binding(tag.wrapping_add(6)),
    };
    let produced =
        producer::derive_descriptor_identity(&descriptor, ValidationCaps::PRODUCTION).unwrap();
    let watched =
        watchdog::derive_descriptor_identity(&descriptor, ValidationCaps::PRODUCTION).unwrap();
    assert_eq!(produced, watched);
    descriptor.claimed_identity = produced;
    descriptor
}

fn packet(
    mut members: Vec<SpeciesDescriptor>,
    dispositions: impl FnOnce(&[SpeciesDescriptor]) -> Vec<SupportDisposition>,
) -> SpeciesSupportPacket {
    members.sort_by_key(|member| member.claimed_identity);
    let replay = binding(231);
    let descriptors = SpeciesDescriptorPacket {
        schema_id: DESCRIPTOR_PACKET_SCHEMA_ID.to_owned(),
        floor_binding: binding(232),
        structure_binding: binding(233),
        species_registry_binding: binding(234),
        replay_binding: replay.clone(),
        members,
    };
    let produced_digest =
        producer::descriptor_packet_digest(&descriptors, ValidationCaps::PRODUCTION).unwrap();
    let watched_digest =
        watchdog::descriptor_packet_digest(&descriptors, ValidationCaps::PRODUCTION).unwrap();
    assert_eq!(produced_digest, watched_digest);
    let mut support_rows = dispositions(&descriptors.members);
    support_rows.sort_by_key(SupportDisposition::identity);
    SpeciesSupportPacket {
        schema_id: PACKET_SCHEMA_ID.to_owned(),
        checker_pair: CheckerPairBinding {
            producer_id: PRODUCER_CHECKER_ID.to_owned(),
            watchdog_id: WATCHDOG_CHECKER_ID.to_owned(),
        },
        resources: resources(),
        descriptors,
        conditioned_support: ConditionedSupportPacket {
            schema_id: CONDITIONED_SUPPORT_SCHEMA_ID.to_owned(),
            descriptor_packet_sha256: produced_digest,
            joint_measure_binding: binding(235),
            conditioning_binding: binding(236),
            replay_binding: replay,
            dispositions: support_rows,
        },
    }
}

fn one_member_packet(rest_mass: ExactUnsignedRationalWire) -> SpeciesSupportPacket {
    packet(vec![descriptor(17, rest_mass)], |members| {
        vec![SupportDisposition::Positive {
            identity: members[0].claimed_identity,
            number_fraction: ratio(1, 1),
        }]
    })
}

fn two_member_packet() -> SpeciesSupportPacket {
    packet(
        vec![descriptor(31, ratio(7, 3)), descriptor(61, ratio(0, 1))],
        |members| {
            vec![
                SupportDisposition::Positive {
                    identity: members[0].claimed_identity,
                    number_fraction: ratio(1, 3),
                },
                SupportDisposition::Positive {
                    identity: members[1].claimed_identity,
                    number_fraction: ratio(2, 3),
                },
            ]
        },
    )
}

fn assert_both_refuse(packet: &SpeciesSupportPacket, refusal: PacketRefusal) {
    assert_eq!(
        producer::validate_and_encode(packet).unwrap_err(),
        refusal,
        "producer refusal"
    );
    assert_eq!(
        watchdog::validate_and_encode(packet).unwrap_err(),
        refusal,
        "watchdog refusal"
    );
}

fn assert_both_refuse_with_caps(
    packet: &SpeciesSupportPacket,
    caps: ValidationCaps,
    refusal: PacketRefusal,
) {
    assert_eq!(
        producer::validate_and_encode_with_caps(packet, caps).unwrap_err(),
        refusal,
        "producer bounded refusal"
    );
    assert_eq!(
        watchdog::validate_and_encode_with_caps(packet, caps).unwrap_err(),
        refusal,
        "watchdog bounded refusal"
    );
}

#[test]
fn unfamiliar_and_massless_packets_agree_without_class_dispatch() {
    let unfamiliar = one_member_packet(ratio(7, 3));
    let massless = one_member_packet(ratio(0, 1));
    for packet in [&unfamiliar, &massless] {
        let produced = producer::validate_and_encode(packet).unwrap();
        let watched = watchdog::validate_and_encode(packet).unwrap();
        assert_eq!(produced, watched);
        assert_eq!(inspect_packet(packet).unwrap(), produced);
        assert_eq!(
            inspect_packet(packet).unwrap(),
            inspect_packet(packet).unwrap()
        );
    }
}

#[test]
fn explicit_exact_zero_disposition_is_complete_and_normalized() {
    let packet = packet(
        vec![descriptor(71, ratio(5, 2)), descriptor(91, ratio(0, 1))],
        |members| {
            vec![
                SupportDisposition::Positive {
                    identity: members[0].claimed_identity,
                    number_fraction: ratio(1, 1),
                },
                SupportDisposition::ExactZero {
                    identity: members[1].claimed_identity,
                    zero_derivation: binding(201),
                },
            ]
        },
    );
    assert!(inspect_packet(&packet).is_ok());
}

#[test]
fn schemas_checkers_and_every_declared_capacity_are_bound() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut changed = baseline.clone();
    changed.schema_id.push_str(".changed");
    assert_both_refuse(&changed, PacketRefusal::SchemaMismatch);

    let mut changed = baseline.clone();
    changed.checker_pair.watchdog_id.push_str(".changed");
    assert_both_refuse(&changed, PacketRefusal::CheckerPairMismatch);

    for mutation in 0..6 {
        let mut changed = baseline.clone();
        match mutation {
            0 => changed.resources.max_member_count += 1,
            1 => changed.resources.max_rational_component_bits += 1,
            2 => changed.resources.max_intermediate_component_bits += 1,
            3 => changed.resources.max_operation_units += 1,
            4 => changed.resources.max_canonical_bytes += 1,
            5 => changed.resources.max_canonical_token_bytes += 1,
            _ => unreachable!(),
        }
        assert_both_refuse(&changed, PacketRefusal::ResourceContractMismatch);
    }
}

#[test]
fn each_runtime_resource_guard_has_a_direct_canary() {
    let packet = one_member_packet(ratio(7, 3));

    let mut caps = ValidationCaps::PRODUCTION;
    caps.member_count = 0;
    assert_both_refuse_with_caps(&packet, caps, PacketRefusal::MemberCapacityExceeded);

    let mut caps = ValidationCaps::PRODUCTION;
    caps.rational_component_bits = 1;
    assert_both_refuse_with_caps(&packet, caps, PacketRefusal::RationalComponentLimitExceeded);

    let mut caps = ValidationCaps::PRODUCTION;
    caps.intermediate_component_bits = 0;
    assert_both_refuse_with_caps(
        &two_member_packet(),
        caps,
        PacketRefusal::IntermediateRationalLimitExceeded,
    );

    let mut caps = ValidationCaps::PRODUCTION;
    caps.operation_units = 0;
    assert_both_refuse_with_caps(&packet, caps, PacketRefusal::OperationLimitExceeded);

    let mut caps = ValidationCaps::PRODUCTION;
    caps.canonical_bytes = 16;
    assert_both_refuse_with_caps(&packet, caps, PacketRefusal::CanonicalByteLimitExceeded);

    let mut caps = ValidationCaps::PRODUCTION;
    caps.canonical_token_bytes = 4;
    assert_both_refuse_with_caps(&packet, caps, PacketRefusal::CanonicalTextInvalid);
}

#[test]
fn rational_wire_defects_refuse_before_identity_or_normalization() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut changed = baseline.clone();
    changed.descriptors.members[0]
        .rest_mass_si
        .numerator_be
        .clear();
    assert_both_refuse(&changed, PacketRefusal::RationalEncodingInvalid);

    let mut changed = baseline.clone();
    changed.descriptors.members[0].rest_mass_si.numerator_be = vec![0, 1];
    assert_both_refuse(&changed, PacketRefusal::RationalEncodingInvalid);

    let mut changed = baseline.clone();
    changed.descriptors.members[0].rest_mass_si.denominator_be = vec![0];
    assert_both_refuse(&changed, PacketRefusal::RationalEncodingInvalid);

    let mut changed = baseline.clone();
    changed.descriptors.members[0].rest_mass_si = ratio(2, 4);
    assert_both_refuse(&changed, PacketRefusal::RationalNotReduced);
}

#[test]
fn descriptor_identity_ancestry_and_order_defects_refuse() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut changed = baseline.clone();
    changed.descriptors.members[0].mass_ancestry.digest_sha256 = [0; 32];
    assert_both_refuse(&changed, PacketRefusal::MissingBindingDigest);

    let mut changed = baseline.clone();
    changed.descriptors.members[0].physical_content.schema_id = "Invalid".to_owned();
    assert_both_refuse(&changed, PacketRefusal::CanonicalTextInvalid);

    let mut changed = baseline.clone();
    changed.descriptors.members[0].claimed_identity.0[0] ^= 1;
    assert_both_refuse(&changed, PacketRefusal::ContentIdentityMismatch);

    let mut duplicate = baseline.clone();
    duplicate
        .descriptors
        .members
        .push(duplicate.descriptors.members[0].clone());
    assert_both_refuse(&duplicate, PacketRefusal::DuplicateContentIdentity);

    let mut collision = baseline.clone();
    let mut second = collision.descriptors.members[0].clone();
    second.physical_content = binding(211);
    collision.descriptors.members.push(second);
    assert_both_refuse(&collision, PacketRefusal::ContentIdentityCollision);

    let mut reordered = two_member_packet();
    reordered.descriptors.members.reverse();
    assert_both_refuse(&reordered, PacketRefusal::NonCanonicalDescriptorOrder);
}

#[test]
fn every_descriptor_content_field_changes_both_identity_and_packet_digest() {
    let baseline = one_member_packet(ratio(7, 3));
    let baseline_descriptor = &baseline.descriptors.members[0];
    let baseline_identity = baseline_descriptor.claimed_identity;
    let baseline_packet_digest = baseline.conditioned_support.descriptor_packet_sha256;

    for field in 0..8 {
        let mut changed_descriptor = baseline_descriptor.clone();
        match field {
            0 => changed_descriptor.physical_content = binding(141),
            1 => changed_descriptor.rest_mass_si = ratio(8, 3),
            2 => changed_descriptor.mass_ancestry = binding(142),
            3 => changed_descriptor.dimension_ancestry = binding(143),
            4 => changed_descriptor.charge_and_state = binding(144),
            5 => changed_descriptor.active_sector_set = binding(145),
            6 => changed_descriptor.validity_domain = binding(146),
            7 => changed_descriptor.dependency_closure = binding(147),
            _ => unreachable!(),
        }
        let produced_identity =
            producer::derive_descriptor_identity(&changed_descriptor, ValidationCaps::PRODUCTION)
                .unwrap();
        let watched_identity =
            watchdog::derive_descriptor_identity(&changed_descriptor, ValidationCaps::PRODUCTION)
                .unwrap();
        assert_eq!(produced_identity, watched_identity, "field {field}");
        assert_ne!(produced_identity, baseline_identity, "field {field}");

        changed_descriptor.claimed_identity = produced_identity;
        let mut descriptor_packet = baseline.descriptors.clone();
        descriptor_packet.members[0] = changed_descriptor;
        let produced_digest =
            producer::descriptor_packet_digest(&descriptor_packet, ValidationCaps::PRODUCTION)
                .unwrap();
        let watched_digest =
            watchdog::descriptor_packet_digest(&descriptor_packet, ValidationCaps::PRODUCTION)
                .unwrap();
        assert_eq!(produced_digest, watched_digest, "field {field}");
        assert_ne!(produced_digest, baseline_packet_digest, "field {field}");
    }
}

#[test]
fn packet_level_binding_defects_refuse_without_minting_authority() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut changed = baseline.clone();
    changed.descriptors.floor_binding.digest_sha256 = [0; 32];
    assert_both_refuse(&changed, PacketRefusal::MissingBindingDigest);

    let mut changed = baseline.clone();
    changed
        .conditioned_support
        .joint_measure_binding
        .digest_sha256 = [0; 32];
    assert_both_refuse(&changed, PacketRefusal::MissingBindingDigest);

    let mut changed = baseline.clone();
    changed
        .conditioned_support
        .conditioning_binding
        .digest_sha256 = [0; 32];
    assert_both_refuse(&changed, PacketRefusal::MissingBindingDigest);

    let mut changed = baseline.clone();
    changed.conditioned_support.replay_binding = binding(212);
    assert_both_refuse(&changed, PacketRefusal::ReplayBindingMismatch);

    let mut changed = baseline.clone();
    changed.conditioned_support.descriptor_packet_sha256[0] ^= 1;
    assert_both_refuse(&changed, PacketRefusal::DescriptorPacketDigestMismatch);
}

#[test]
fn support_must_cover_every_member_exactly_once() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut empty = baseline.clone();
    empty.descriptors.members.clear();
    assert_both_refuse(&empty, PacketRefusal::EmptyRegistry);

    let mut missing = baseline.clone();
    missing.conditioned_support.dispositions.clear();
    assert_both_refuse(&missing, PacketRefusal::MissingDisposition);

    let mut unknown = baseline.clone();
    unknown
        .conditioned_support
        .dispositions
        .push(SupportDisposition::ExactZero {
            identity: SpeciesContentIdentity([0; 32]),
            zero_derivation: binding(214),
        });
    unknown
        .conditioned_support
        .dispositions
        .sort_by_key(SupportDisposition::identity);
    assert_both_refuse(&unknown, PacketRefusal::UnknownDisposition);

    let mut duplicate = baseline.clone();
    duplicate
        .conditioned_support
        .dispositions
        .push(duplicate.conditioned_support.dispositions[0].clone());
    assert_both_refuse(&duplicate, PacketRefusal::DuplicateDisposition);

    let mut both_positive_and_zero = baseline.clone();
    both_positive_and_zero
        .conditioned_support
        .dispositions
        .push(SupportDisposition::ExactZero {
            identity: baseline.descriptors.members[0].claimed_identity,
            zero_derivation: binding(213),
        });
    assert_both_refuse(&both_positive_and_zero, PacketRefusal::DuplicateDisposition);

    let mut reordered = two_member_packet();
    reordered.conditioned_support.dispositions.reverse();
    assert_both_refuse(&reordered, PacketRefusal::NonCanonicalDispositionOrder);
}

#[test]
fn zero_and_normalization_semantics_refuse_implicit_repairs() {
    let baseline = one_member_packet(ratio(7, 3));

    let mut nonpositive = baseline.clone();
    nonpositive.conditioned_support.dispositions[0] = SupportDisposition::Positive {
        identity: baseline.descriptors.members[0].claimed_identity,
        number_fraction: ratio(0, 1),
    };
    assert_both_refuse(&nonpositive, PacketRefusal::NonPositiveSupportWeight);

    let mut unproved_zero = baseline.clone();
    unproved_zero.conditioned_support.dispositions[0] = SupportDisposition::ExactZero {
        identity: baseline.descriptors.members[0].claimed_identity,
        zero_derivation: ArtifactBinding {
            schema_id: "synthetic.binding.zero".to_owned(),
            digest_sha256: [0; 32],
        },
    };
    assert_both_refuse(&unproved_zero, PacketRefusal::UnprovedExactZero);

    let mut nonunit = baseline;
    nonunit.conditioned_support.dispositions[0] = SupportDisposition::Positive {
        identity: nonunit.descriptors.members[0].claimed_identity,
        number_fraction: ratio(1, 2),
    };
    assert_both_refuse(&nonunit, PacketRefusal::NonUnitCompositionSimplex);
}

#[test]
fn conditioned_support_accepts_multiple_exact_fractions_without_renormalization() {
    let packet = two_member_packet();
    let produced = producer::validate_and_encode(&packet).unwrap();
    let watched = watchdog::validate_and_encode(&packet).unwrap();
    assert_eq!(produced, watched);
}
