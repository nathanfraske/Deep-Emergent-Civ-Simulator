//! Private wire model for the byte-neutral species support packet.
//!
//! These types carry structural bindings only. A digest records which bytes a
//! future physical authority must prove; its presence does not prove the
//! referenced claim and none of these types can mint a species authority.

use super::super::SpeciesContentIdentity;

pub(super) const PACKET_SCHEMA_ID: &str = "civsim.planet.stellar-birth-species-support-packet.v1";
pub(super) const DESCRIPTOR_PACKET_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-species-descriptor-packet.v1";
pub(super) const CONDITIONED_SUPPORT_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-conditioned-species-support.v1";
pub(super) const PRODUCER_CHECKER_ID: &str =
    "civsim.planet.stellar-birth-species-packet-producer.v1";
pub(super) const WATCHDOG_CHECKER_ID: &str =
    "civsim.planet.stellar-birth-species-packet-watchdog.v1";

pub(super) const MAX_MEMBER_COUNT: u32 = 4_096;
pub(super) const MAX_RATIONAL_COMPONENT_BITS: u32 = 4_096;
pub(super) const MAX_INTERMEDIATE_COMPONENT_BITS: u32 = 65_536;
pub(super) const MAX_OPERATION_UNITS: u64 = 1_000_000;
pub(super) const MAX_CANONICAL_BYTES: u32 = 16_777_216;
pub(super) const MAX_CANONICAL_TOKEN_BYTES: u32 = 192;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ArtifactBinding {
    pub(super) schema_id: String,
    pub(super) digest_sha256: [u8; 32],
}

/// A canonical unsigned rational. Components are minimal big-endian byte
/// strings and the denominator is positive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactUnsignedRationalWire {
    pub(super) numerator_be: Vec<u8>,
    pub(super) denominator_be: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SpeciesDescriptor {
    pub(super) claimed_identity: SpeciesContentIdentity,
    pub(super) physical_content: ArtifactBinding,
    pub(super) rest_mass_si: ExactUnsignedRationalWire,
    pub(super) mass_ancestry: ArtifactBinding,
    pub(super) dimension_ancestry: ArtifactBinding,
    pub(super) charge_and_state: ArtifactBinding,
    pub(super) active_sector_set: ArtifactBinding,
    pub(super) validity_domain: ArtifactBinding,
    pub(super) dependency_closure: ArtifactBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeciesDescriptorPacket {
    pub(super) schema_id: String,
    pub(super) floor_binding: ArtifactBinding,
    pub(super) structure_binding: ArtifactBinding,
    pub(super) species_registry_binding: ArtifactBinding,
    pub(super) replay_binding: ArtifactBinding,
    pub(super) members: Vec<SpeciesDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SupportDisposition {
    Positive {
        identity: SpeciesContentIdentity,
        number_fraction: ExactUnsignedRationalWire,
    },
    ExactZero {
        identity: SpeciesContentIdentity,
        zero_derivation: ArtifactBinding,
    },
}

impl SupportDisposition {
    pub(super) const fn identity(&self) -> SpeciesContentIdentity {
        match self {
            Self::Positive { identity, .. } | Self::ExactZero { identity, .. } => *identity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConditionedSupportPacket {
    pub(super) schema_id: String,
    pub(super) descriptor_packet_sha256: [u8; 32],
    pub(super) joint_measure_binding: ArtifactBinding,
    pub(super) conditioning_binding: ArtifactBinding,
    pub(super) replay_binding: ArtifactBinding,
    pub(super) dispositions: Vec<SupportDisposition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckerPairBinding {
    pub(super) producer_id: String,
    pub(super) watchdog_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PacketResourceContract {
    pub(super) max_member_count: u32,
    pub(super) max_rational_component_bits: u32,
    pub(super) max_intermediate_component_bits: u32,
    pub(super) max_operation_units: u64,
    pub(super) max_canonical_bytes: u32,
    pub(super) max_canonical_token_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SpeciesSupportPacket {
    pub(super) schema_id: String,
    pub(super) checker_pair: CheckerPairBinding,
    pub(super) resources: PacketResourceContract,
    pub(super) descriptors: SpeciesDescriptorPacket,
    pub(super) conditioned_support: ConditionedSupportPacket,
}

/// Execution caps are fixed for production validation. Tests may tighten one
/// cap to exercise a refusal without constructing a maximum-sized packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidationCaps {
    pub(super) member_count: u32,
    pub(super) rational_component_bits: u32,
    pub(super) intermediate_component_bits: u32,
    pub(super) operation_units: u64,
    pub(super) canonical_bytes: u32,
    pub(super) canonical_token_bytes: u32,
}

impl ValidationCaps {
    pub(super) const PRODUCTION: Self = Self {
        member_count: MAX_MEMBER_COUNT,
        rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        operation_units: MAX_OPERATION_UNITS,
        canonical_bytes: MAX_CANONICAL_BYTES,
        canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PacketRefusal {
    SchemaMismatch,
    CheckerPairMismatch,
    ResourceContractMismatch,
    EmptyRegistry,
    MemberCapacityExceeded,
    CanonicalTextInvalid,
    MissingBindingDigest,
    RationalEncodingInvalid,
    RationalComponentLimitExceeded,
    RationalNotReduced,
    IntermediateRationalLimitExceeded,
    OperationLimitExceeded,
    CanonicalByteLimitExceeded,
    NonCanonicalDescriptorOrder,
    DuplicateContentIdentity,
    ContentIdentityCollision,
    ContentIdentityMismatch,
    DescriptorPacketDigestMismatch,
    ReplayBindingMismatch,
    NonCanonicalDispositionOrder,
    DuplicateDisposition,
    UnknownDisposition,
    MissingDisposition,
    NonPositiveSupportWeight,
    UnprovedExactZero,
    NonUnitCompositionSimplex,
    CheckerDisagreement,
}
