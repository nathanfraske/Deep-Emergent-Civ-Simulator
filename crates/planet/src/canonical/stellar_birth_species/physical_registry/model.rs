//! Private model for physical species derivation and closure.
//!
//! Artifact families, relation roles, and dimension axes are admitted data,
//! not a closed list selected by the caller or by familiar-universe names.
//! Ledger tier and provenance are accounting fields only. Evidence custody
//! without the complete admission route is represented so both validators can
//! refuse it.

use super::super::SpeciesContentIdentity;
use super::charged_profile::ChargedProfileAdmissionCapability;
use super::primitive_profile::PrimitiveProfileAdmissionCapability;
use super::repository_roots::RepositoryRootAdmissionCapability;
pub(super) use civsim_ledger::{Provenance as ProvenanceMark, Tier as LedgerTier};

pub(super) const REGISTRY_SCHEMA_ID: &str =
    "civsim.planet.stellar-birth-physical-species-registry.v5";
pub(super) const PROOF_GRAPH_SCHEMA_ID: &str = "civsim.planet.stellar-birth-species-proof-graph.v4";
pub(super) const PRODUCER_ID: &str = "civsim.planet.stellar-birth-physical-species-producer.v7";
pub(super) const WATCHDOG_ID: &str = "civsim.planet.stellar-birth-physical-species-watchdog.v6";

pub(super) const MAX_ARTIFACT_COUNT: u32 = 4_096;
pub(super) const MAX_REGISTRY_MEMBER_COUNT: u32 = 4_096;
pub(super) const MAX_REFERENCES_PER_ARTIFACT: u32 = 4_096;
pub(super) const MAX_TOTAL_REFERENCE_COUNT: u32 = 65_536;
pub(super) const MAX_EXPRESSION_NODE_COUNT: u32 = 65_536;
pub(super) const MAX_EXPRESSION_EDGE_COUNT: u32 = 131_072;
pub(super) const MAX_EXPRESSION_DEPTH: u32 = 1_024;
pub(super) const MAX_RATIONAL_COMPONENT_BITS: u32 = 4_096;
pub(super) const MAX_INTERMEDIATE_COMPONENT_BITS: u32 = 65_536;
pub(super) const MAX_DIMENSION_ABS_EXPONENT: i32 = 4_096;
pub(super) const MAX_DIMENSION_TERM_COUNT: u32 = 64;
pub(super) const MAX_EVALUATION_STEPS: u64 = 1_000_000;
pub(super) const MAX_CLOSURE_STEPS: u64 = 1_000_000;
pub(super) const MAX_CANONICAL_BYTES: u32 = 16_777_216;
pub(super) const MAX_CANONICAL_TOKEN_BYTES: u32 = 192;
pub(super) const MAX_CONTENT_BYTES: u32 = 1_048_576;

const DIMENSION_TERM_STORAGE: usize = MAX_DIMENSION_TERM_COUNT as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionAxisIdentity(pub(super) [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionTerm {
    pub(super) axis: DimensionAxisIdentity,
    pub(super) exponent: i16,
}

impl DimensionTerm {
    const EMPTY: Self = Self {
        axis: DimensionAxisIdentity([0; 32]),
        exponent: 0,
    };
}

/// A bounded, variable-cardinality dimension over data-defined axis identities.
///
/// The current floor uses seven SI axes, but neither the type nor its wire
/// format reserves those axes as the complete physical basis. Terms are stored
/// in ascending axis-identity order with no zero exponent or duplicate axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DimensionVector {
    term_count: u8,
    terms: [DimensionTerm; DIMENSION_TERM_STORAGE],
}

impl DimensionVector {
    pub(super) const fn dimensionless() -> Self {
        Self {
            term_count: 0,
            terms: [DimensionTerm::EMPTY; DIMENSION_TERM_STORAGE],
        }
    }

    pub(super) const fn one(axis: DimensionAxisIdentity) -> Self {
        let mut terms = [DimensionTerm::EMPTY; DIMENSION_TERM_STORAGE];
        terms[0] = DimensionTerm { axis, exponent: 1 };
        Self {
            term_count: 1,
            terms,
        }
    }

    pub(super) fn from_terms(
        mut terms: Vec<DimensionTerm>,
    ) -> Result<Self, PhysicalRegistryRefusalCode> {
        if terms.len() > DIMENSION_TERM_STORAGE {
            return Err(PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded);
        }
        terms.sort_unstable_by_key(|term| term.axis);
        if terms
            .iter()
            .any(|term| term.exponent == 0 || term.axis.0 == [0; 32])
        {
            return Err(PhysicalRegistryRefusalCode::DimensionEncodingInvalid);
        }
        if terms.windows(2).any(|pair| pair[0].axis == pair[1].axis) {
            return Err(PhysicalRegistryRefusalCode::DimensionEncodingInvalid);
        }
        let mut stored = [DimensionTerm::EMPTY; DIMENSION_TERM_STORAGE];
        stored[..terms.len()].copy_from_slice(&terms);
        Ok(Self {
            term_count: u8::try_from(terms.len())
                .map_err(|_| PhysicalRegistryRefusalCode::DimensionTermCapacityExceeded)?,
            terms: stored,
        })
    }

    pub(super) fn terms(&self) -> &[DimensionTerm] {
        &self.terms[..usize::from(self.term_count)]
    }
}

/// SHA-256 inputs for the current floor's SI dimension-axis identities.
///
/// Each axis identity is the SHA-256 digest of the exact UTF-8 bytes below.
/// The `civsim.dimension-axis` prefix domain-separates these identities from
/// artifact, member, receipt, and arbitrary content hashes. The version suffix
/// makes a future identity-contract change explicit.
pub(super) const SI_DIMENSION_AXIS_IDENTITY_INPUTS: [&[u8]; 7] = [
    b"civsim.dimension-axis.si.length.v1",
    b"civsim.dimension-axis.si.mass.v1",
    b"civsim.dimension-axis.si.time.v1",
    b"civsim.dimension-axis.si.current.v1",
    b"civsim.dimension-axis.si.temperature.v1",
    b"civsim.dimension-axis.si.amount.v1",
    b"civsim.dimension-axis.si.luminous-intensity.v1",
];

pub(super) const SI_LENGTH_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0x55, 0xef, 0xd3, 0x80, 0xbe, 0xb3, 0xe4, 0xbc, 0x51, 0xde, 0x1c, 0x3f, 0x8c, 0x4f, 0x58, 0x75,
    0x72, 0x31, 0x7e, 0x75, 0xc4, 0xb4, 0xa5, 0x28, 0x53, 0xd1, 0xf1, 0xb2, 0x5c, 0xe8, 0x40, 0x0b,
]);
pub(super) const SI_MASS_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0xee, 0x06, 0xd1, 0x51, 0x65, 0x5c, 0xa3, 0x06, 0x36, 0xe5, 0xe1, 0x1f, 0xfd, 0x7a, 0x5e, 0x1e,
    0x83, 0x8b, 0xc0, 0x7f, 0x46, 0x1e, 0x3c, 0xf4, 0xce, 0x4e, 0x8e, 0x3d, 0x39, 0x53, 0x11, 0xe0,
]);
pub(super) const SI_TIME_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0x3b, 0x09, 0xbc, 0xc6, 0xaa, 0x3c, 0xd3, 0x2d, 0x6b, 0xf0, 0xd4, 0xef, 0x5c, 0x87, 0x58, 0x93,
    0x1c, 0x1f, 0x52, 0x1b, 0x62, 0xa8, 0x38, 0x1d, 0x71, 0xc7, 0xcc, 0x2e, 0xab, 0x48, 0xd9, 0x98,
]);
pub(super) const SI_CURRENT_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0x33, 0xf3, 0x44, 0x68, 0xb9, 0xa5, 0xbb, 0x32, 0x8f, 0x56, 0x70, 0xfa, 0xef, 0x25, 0x6c, 0x80,
    0x96, 0xc1, 0x0d, 0x31, 0x24, 0xe1, 0xef, 0x06, 0x76, 0xe9, 0xc6, 0x7f, 0x02, 0x9d, 0x8f, 0x37,
]);
pub(super) const SI_TEMPERATURE_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0xdb, 0x4d, 0xc3, 0x81, 0x7d, 0xec, 0xf6, 0xc6, 0xc2, 0x52, 0x4d, 0x85, 0xd2, 0x0e, 0x1b, 0x41,
    0x87, 0x45, 0xb3, 0xd9, 0xc1, 0x28, 0x3f, 0x62, 0x19, 0xf9, 0x7b, 0x62, 0x5c, 0x7a, 0x95, 0xb8,
]);
pub(super) const SI_AMOUNT_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0xe1, 0xee, 0xe0, 0x36, 0x67, 0xb5, 0x71, 0x5a, 0x9c, 0x5f, 0x10, 0x9f, 0xbb, 0xd7, 0xeb, 0xb8,
    0xb8, 0x08, 0xa6, 0xcd, 0xb7, 0x46, 0x72, 0x78, 0xfa, 0x0a, 0x62, 0x42, 0x06, 0x49, 0xf4, 0x61,
]);
pub(super) const SI_LUMINOUS_INTENSITY_AXIS: DimensionAxisIdentity = DimensionAxisIdentity([
    0x1d, 0xd8, 0xc7, 0xe9, 0x93, 0x46, 0x9c, 0xa2, 0x02, 0xbb, 0x9c, 0x79, 0xcc, 0x96, 0xba, 0x87,
    0xbc, 0x92, 0x84, 0x0b, 0x3c, 0x66, 0x3c, 0x4a, 0xef, 0x90, 0xa3, 0xc9, 0x7b, 0xd5, 0xab, 0xac,
]);

pub(super) const SI_DIMENSION_AXES: [DimensionAxisIdentity; 7] = [
    SI_LENGTH_AXIS,
    SI_MASS_AXIS,
    SI_TIME_AXIS,
    SI_CURRENT_AXIS,
    SI_TEMPERATURE_AXIS,
    SI_AMOUNT_AXIS,
    SI_LUMINOUS_INTENSITY_AXIS,
];

pub(super) const MASS_DIMENSION: DimensionVector = DimensionVector::one(SI_MASS_AXIS);

pub(super) fn dimension_from_si_exponents(
    exponents: [i16; 7],
) -> Result<DimensionVector, PhysicalRegistryRefusalCode> {
    let terms = SI_DIMENSION_AXES
        .into_iter()
        .zip(exponents)
        .filter_map(|(axis, exponent)| (exponent != 0).then_some(DimensionTerm { axis, exponent }))
        .collect();
    DimensionVector::from_terms(terms)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ArtifactIdentity(pub(super) [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ReceiptBinding {
    pub(super) schema_id: String,
    pub(super) digest_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CanonicalArtifact {
    pub(super) schema_id: String,
    pub(super) canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct DerivedAdmission {
    pub(super) ancestry_receipt: ReceiptBinding,
    pub(super) semantic_checker_receipt: ReceiptBinding,
    pub(super) independent_watchdog_receipt: ReceiptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct IrreducibleAdmission {
    pub(super) derivation_exhaustion_receipt: ReceiptBinding,
    pub(super) buckingham_pi_receipt: ReceiptBinding,
    pub(super) gap_law_receipt: ReceiptBinding,
    pub(super) chaos_protocol_receipt: ReceiptBinding,
    pub(super) residual_law_receipt: ReceiptBinding,
    pub(super) residual_slot_id: String,
    pub(super) residual_slot_receipt: ReceiptBinding,
    pub(super) owner_admission_receipt: ReceiptBinding,
    pub(super) independent_watchdog_receipt: ReceiptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AdmissionRoute {
    Derived(DerivedAdmission),
    Irreducible(Box<IrreducibleAdmission>),
    EvidenceCustodyOnly { source_receipt: ReceiptBinding },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RootAdmission {
    pub(super) tier: LedgerTier,
    pub(super) provenance: ProvenanceMark,
    pub(super) route: AdmissionRoute,
}

/// Canonical signed rational. Components are minimal big-endian byte strings,
/// the denominator is positive, and zero is nonnegative.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactRationalWire {
    pub(super) negative: bool,
    pub(super) numerator_be: Vec<u8>,
    pub(super) denominator_be: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ScalarCoordinateArtifact {
    pub(super) coordinate: CanonicalArtifact,
    pub(super) exact_value: ExactRationalWire,
    pub(super) dimension: DimensionVector,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ExactExpressionNode {
    Coordinate(ArtifactIdentity),
    Add { left: u32, right: u32 },
    Subtract { left: u32, right: u32 },
    Multiply { left: u32, right: u32 },
    Divide { numerator: u32, denominator: u32 },
    IntegerPower { base: u32, exponent: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactExpression {
    pub(super) nodes: Vec<ExactExpressionNode>,
    pub(super) output_node: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ArtifactRelation {
    /// An admitted descriptor that defines the relation's physical role.
    pub(super) role: ArtifactIdentity,
    pub(super) target: ArtifactIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RequirementSet {
    pub(super) artifact_relations: Vec<ArtifactRelation>,
    pub(super) species_dependencies: Vec<SpeciesContentIdentity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MassProofReference {
    Projection(ArtifactIdentity),
    ExactMassless(ArtifactIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MemberBlueprint {
    pub(super) physical_content: CanonicalArtifact,
    pub(super) requirements: RequirementSet,
    pub(super) mass_proof: MassProofReference,
    /// Role-to-law edges. Stability, transition, thaumic, and unfamiliar
    /// constraints are registry data rather than closed Rust variants.
    pub(super) constraint_laws: Vec<ArtifactRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ConstraintLawArtifact {
    pub(super) requirements: RequirementSet,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MassProjectionArtifact {
    pub(super) expression: ExactExpression,
    pub(super) scope: MassProjectionScope,
    pub(super) uncertainty_transport: Option<MassUncertaintyTransportProof>,
}

/// Claim-local proof that a measured coordinate's uncertainty identity travels
/// with a species rest-mass projection.
///
/// The source coordinate's canonical identity already binds its central value,
/// uncertainty kind, uncertainty value, dimension, floor ancestry, and root
/// pair capability. These receipts bind the profile pair that inspected the
/// projection without turning a coordinate into species authority on its own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MassUncertaintyTransportProof {
    pub(super) source_coordinate: ArtifactIdentity,
    pub(super) source_pair_receipt: ReceiptBinding,
    pub(super) producer_receipt: ReceiptBinding,
    pub(super) watchdog_receipt: ReceiptBinding,
}

/// Whether an exact mass-dimension expression may satisfy a species rest-mass
/// proof. Floor-coordinate projections remain membership-neutral until a
/// separate uncertainty-transport authority is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MassProjectionScope {
    MembershipNeutral,
    SpeciesRestMass,
}

impl MassProjectionScope {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::MembershipNeutral => "membership-neutral",
            Self::SpeciesRestMass => "species-rest-mass",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MasslessLawArtifact {
    pub(super) requirements: RequirementSet,
    pub(super) proof: ExactZeroMassProof,
}

/// Claim-scoped proof object for an exact zero rest-mass term.
///
/// A missing mass term is not a proof. The subject, excluded term, preserving
/// symmetry, applicability domain, and independent exclusion receipts are all
/// identity-bound into the massless-law artifact.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ExactZeroMassProof {
    pub(super) subject: ArtifactIdentity,
    pub(super) excluded_term: CanonicalArtifact,
    pub(super) symmetry: ArtifactIdentity,
    pub(super) applicability_receipt: ReceiptBinding,
    pub(super) exclusion_producer_receipt: ReceiptBinding,
    pub(super) exclusion_watchdog_receipt: ReceiptBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SpeciesDerivationArtifact {
    /// An admitted descriptor defining the derivation family. This keeps the
    /// validator open to unfamiliar and thaumic law-abiding mechanisms.
    pub(super) derivation_kind: ArtifactIdentity,
    pub(super) artifact_inputs: Vec<ArtifactRelation>,
    pub(super) constituents: Vec<SpeciesContentIdentity>,
    pub(super) output: MemberBlueprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ArtifactPayload {
    ScalarCoordinate(Box<ScalarCoordinateArtifact>),
    /// An admitted opaque descriptor. Its semantic role comes from verified
    /// relation edges, never from this payload variant.
    PhysicalDescriptor(CanonicalArtifact),
    ConstraintLaw(ConstraintLawArtifact),
    MassProjection(MassProjectionArtifact),
    ExactMasslessLaw(MasslessLawArtifact),
    SpeciesDerivation(SpeciesDerivationArtifact),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct AdmittedArtifact {
    pub(super) claimed_identity: ArtifactIdentity,
    pub(super) admission: RootAdmission,
    pub(super) payload: ArtifactPayload,
    admission_capability: VerifiedAdmissionCapability,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum VerifiedAdmissionCapability {
    RepositoryRoot(RepositoryRootAdmissionCapability),
    PrimitiveProfile(PrimitiveProfileAdmissionCapability),
    ChargedProfile(ChargedProfileAdmissionCapability),
    #[cfg(test)]
    ExactTest {
        claimed_identity: ArtifactIdentity,
        admission: RootAdmission,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AdmissionCapabilityKind {
    RepositoryRoot,
    PrimitiveProfile,
    ChargedProfile,
    #[cfg(test)]
    ExactTest,
}

impl AdmittedArtifact {
    pub(super) fn from_repository_root(
        claimed_identity: ArtifactIdentity,
        admission: RootAdmission,
        payload: ArtifactPayload,
        capability: RepositoryRootAdmissionCapability,
    ) -> Self {
        debug_assert_eq!(capability.claimed_identity(), claimed_identity);
        debug_assert_eq!(capability.admission(), &admission);
        debug_assert_ne!(capability.pair_receipt_sha256(), [0; 32]);
        Self {
            claimed_identity,
            admission,
            payload,
            admission_capability: VerifiedAdmissionCapability::RepositoryRoot(capability),
        }
    }

    pub(super) fn from_primitive_profile(
        claimed_identity: ArtifactIdentity,
        admission: RootAdmission,
        payload: ArtifactPayload,
        capability: PrimitiveProfileAdmissionCapability,
    ) -> Self {
        debug_assert_eq!(capability.claimed_identity(), claimed_identity);
        debug_assert_eq!(capability.admission(), &admission);
        debug_assert_ne!(capability.profile_root_identity().0, [0; 32]);
        debug_assert_ne!(capability.pair_receipt_sha256(), [0; 32]);
        Self {
            claimed_identity,
            admission,
            payload,
            admission_capability: VerifiedAdmissionCapability::PrimitiveProfile(capability),
        }
    }

    pub(super) fn from_charged_profile(
        claimed_identity: ArtifactIdentity,
        admission: RootAdmission,
        payload: ArtifactPayload,
        capability: ChargedProfileAdmissionCapability,
    ) -> Self {
        debug_assert_eq!(capability.claimed_identity(), claimed_identity);
        debug_assert_eq!(capability.admission(), &admission);
        debug_assert_ne!(capability.profile_root_identity().0, [0; 32]);
        debug_assert_ne!(capability.pair_receipt_sha256(), [0; 32]);
        Self {
            claimed_identity,
            admission,
            payload,
            admission_capability: VerifiedAdmissionCapability::ChargedProfile(capability),
        }
    }

    #[cfg(test)]
    pub(super) fn from_exact_test_recomputation(
        claimed_identity: ArtifactIdentity,
        admission: RootAdmission,
        payload: ArtifactPayload,
    ) -> Self {
        Self {
            claimed_identity,
            admission: admission.clone(),
            payload,
            admission_capability: VerifiedAdmissionCapability::ExactTest {
                claimed_identity,
                admission,
            },
        }
    }

    #[cfg(test)]
    pub(super) fn refresh_exact_test_capability(&mut self) {
        match &mut self.admission_capability {
            VerifiedAdmissionCapability::ExactTest {
                claimed_identity,
                admission,
            } => {
                *claimed_identity = self.claimed_identity;
                *admission = self.admission.clone();
            }
            VerifiedAdmissionCapability::RepositoryRoot(_) => {
                panic!("repository-root capabilities cannot be refreshed by a test fixture")
            }
            VerifiedAdmissionCapability::PrimitiveProfile(_) => {
                panic!("primitive-profile capabilities cannot be refreshed by a test fixture")
            }
            VerifiedAdmissionCapability::ChargedProfile(_) => {
                panic!("charged-profile capabilities cannot be refreshed by a test fixture")
            }
        }
    }

    pub(super) const fn capability_claimed_identity(&self) -> ArtifactIdentity {
        match &self.admission_capability {
            VerifiedAdmissionCapability::RepositoryRoot(capability) => {
                capability.claimed_identity()
            }
            VerifiedAdmissionCapability::PrimitiveProfile(capability) => {
                capability.claimed_identity()
            }
            VerifiedAdmissionCapability::ChargedProfile(capability) => {
                capability.claimed_identity()
            }
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest {
                claimed_identity, ..
            } => *claimed_identity,
        }
    }

    pub(super) const fn admission_capability_kind(&self) -> AdmissionCapabilityKind {
        match &self.admission_capability {
            VerifiedAdmissionCapability::RepositoryRoot(_) => {
                AdmissionCapabilityKind::RepositoryRoot
            }
            VerifiedAdmissionCapability::PrimitiveProfile(_) => {
                AdmissionCapabilityKind::PrimitiveProfile
            }
            VerifiedAdmissionCapability::ChargedProfile(_) => {
                AdmissionCapabilityKind::ChargedProfile
            }
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => AdmissionCapabilityKind::ExactTest,
        }
    }

    pub(super) const fn capability_admission(&self) -> &RootAdmission {
        match &self.admission_capability {
            VerifiedAdmissionCapability::RepositoryRoot(capability) => capability.admission(),
            VerifiedAdmissionCapability::PrimitiveProfile(capability) => capability.admission(),
            VerifiedAdmissionCapability::ChargedProfile(capability) => capability.admission(),
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { admission, .. } => admission,
        }
    }

    pub(super) const fn repository_root_pair_receipt_sha256(&self) -> Option<[u8; 32]> {
        match &self.admission_capability {
            VerifiedAdmissionCapability::RepositoryRoot(capability) => {
                Some(capability.pair_receipt_sha256())
            }
            VerifiedAdmissionCapability::PrimitiveProfile(_) => None,
            VerifiedAdmissionCapability::ChargedProfile(_) => None,
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => None,
        }
    }

    pub(super) const fn primitive_profile_pair_receipt_sha256(&self) -> Option<[u8; 32]> {
        match &self.admission_capability {
            VerifiedAdmissionCapability::PrimitiveProfile(capability) => {
                Some(capability.pair_receipt_sha256())
            }
            VerifiedAdmissionCapability::RepositoryRoot(_) => None,
            VerifiedAdmissionCapability::ChargedProfile(_) => None,
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => None,
        }
    }

    pub(super) const fn primitive_profile_root_identity(&self) -> Option<ArtifactIdentity> {
        match &self.admission_capability {
            VerifiedAdmissionCapability::PrimitiveProfile(capability) => {
                Some(capability.profile_root_identity())
            }
            VerifiedAdmissionCapability::RepositoryRoot(_) => None,
            VerifiedAdmissionCapability::ChargedProfile(_) => None,
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => None,
        }
    }

    pub(super) const fn charged_profile_pair_receipt_sha256(&self) -> Option<[u8; 32]> {
        match &self.admission_capability {
            VerifiedAdmissionCapability::ChargedProfile(capability) => {
                Some(capability.pair_receipt_sha256())
            }
            VerifiedAdmissionCapability::RepositoryRoot(_)
            | VerifiedAdmissionCapability::PrimitiveProfile(_) => None,
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => None,
        }
    }

    pub(super) const fn charged_profile_root_identity(&self) -> Option<ArtifactIdentity> {
        match &self.admission_capability {
            VerifiedAdmissionCapability::ChargedProfile(capability) => {
                Some(capability.profile_root_identity())
            }
            VerifiedAdmissionCapability::RepositoryRoot(_)
            | VerifiedAdmissionCapability::PrimitiveProfile(_) => None,
            #[cfg(test)]
            VerifiedAdmissionCapability::ExactTest { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructureAuthorityBinding {
    pub(super) structure_schema_id: String,
    pub(super) species_registry_schema_id: String,
    pub(super) stellar_state_schema_id: String,
    pub(super) state_coordinate_registry_schema_id: String,
    pub(super) interaction_sector_registry_schema_id: String,
    pub(super) physical_regime_registry_schema_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckerPairBinding {
    pub(super) producer_id: String,
    pub(super) watchdog_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PhysicalRegistryResourceContract {
    pub(super) max_artifact_count: u32,
    pub(super) max_registry_member_count: u32,
    pub(super) max_references_per_artifact: u32,
    pub(super) max_total_reference_count: u32,
    pub(super) max_expression_node_count: u32,
    pub(super) max_expression_edge_count: u32,
    pub(super) max_expression_depth: u32,
    pub(super) max_rational_component_bits: u32,
    pub(super) max_intermediate_component_bits: u32,
    pub(super) max_dimension_abs_exponent: i32,
    pub(super) max_dimension_term_count: u32,
    pub(super) max_evaluation_steps: u64,
    pub(super) max_closure_steps: u64,
    pub(super) max_canonical_bytes: u32,
    pub(super) max_canonical_token_bytes: u32,
    pub(super) max_content_bytes: u32,
}

impl PhysicalRegistryResourceContract {
    pub(super) const PRODUCTION: Self = Self {
        max_artifact_count: MAX_ARTIFACT_COUNT,
        max_registry_member_count: MAX_REGISTRY_MEMBER_COUNT,
        max_references_per_artifact: MAX_REFERENCES_PER_ARTIFACT,
        max_total_reference_count: MAX_TOTAL_REFERENCE_COUNT,
        max_expression_node_count: MAX_EXPRESSION_NODE_COUNT,
        max_expression_edge_count: MAX_EXPRESSION_EDGE_COUNT,
        max_expression_depth: MAX_EXPRESSION_DEPTH,
        max_rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        max_intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        max_dimension_abs_exponent: MAX_DIMENSION_ABS_EXPONENT,
        max_dimension_term_count: MAX_DIMENSION_TERM_COUNT,
        max_evaluation_steps: MAX_EVALUATION_STEPS,
        max_closure_steps: MAX_CLOSURE_STEPS,
        max_canonical_bytes: MAX_CANONICAL_BYTES,
        max_canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
        max_content_bytes: MAX_CONTENT_BYTES,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalVocabularyBinding {
    pub(super) schema_id: String,
    pub(super) claim_id: String,
    pub(super) producer_id: String,
    pub(super) watchdog_id: String,
    pub(super) descriptor_role_identities: Vec<ArtifactIdentity>,
    pub(super) relation_target_identities: Vec<ArtifactIdentity>,
    pub(super) constraint_law_identities: Vec<ArtifactIdentity>,
    pub(super) root_count: u32,
    pub(super) current_input_partition_complete: bool,
    pub(super) global_physical_vocabulary_coverage: bool,
    pub(super) membership_authority: bool,
    pub(super) producer_result_sha256: [u8; 32],
    pub(super) watchdog_result_sha256: [u8; 32],
    pub(super) producer_resource_contract_sha256: [u8; 32],
    pub(super) watchdog_resource_contract_sha256: [u8; 32],
    pub(super) receipt_sha256: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalRegistryInput {
    pub(super) schema_id: String,
    pub(super) proof_graph_schema_id: String,
    pub(super) floor_binding: ReceiptBinding,
    pub(super) structure_binding: StructureAuthorityBinding,
    pub(super) checker_pair: CheckerPairBinding,
    pub(super) resources: PhysicalRegistryResourceContract,
    pub(super) vocabulary_binding: PhysicalVocabularyBinding,
    pub(super) admitted_artifacts: Vec<AdmittedArtifact>,
    pub(super) declared_members: Vec<SpeciesContentIdentity>,
}

/// Tests may tighten one cap to exercise a refusal without building a maximum
/// sized graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValidationCaps {
    pub(super) artifact_count: u32,
    pub(super) registry_member_count: u32,
    pub(super) references_per_artifact: u32,
    pub(super) total_reference_count: u32,
    pub(super) expression_node_count: u32,
    pub(super) expression_edge_count: u32,
    pub(super) expression_depth: u32,
    pub(super) rational_component_bits: u32,
    pub(super) intermediate_component_bits: u32,
    pub(super) dimension_abs_exponent: i32,
    pub(super) dimension_term_count: u32,
    pub(super) evaluation_steps: u64,
    pub(super) closure_steps: u64,
    pub(super) canonical_bytes: u32,
    pub(super) canonical_token_bytes: u32,
    pub(super) content_bytes: u32,
}

impl ValidationCaps {
    pub(super) const PRODUCTION: Self = Self {
        artifact_count: MAX_ARTIFACT_COUNT,
        registry_member_count: MAX_REGISTRY_MEMBER_COUNT,
        references_per_artifact: MAX_REFERENCES_PER_ARTIFACT,
        total_reference_count: MAX_TOTAL_REFERENCE_COUNT,
        expression_node_count: MAX_EXPRESSION_NODE_COUNT,
        expression_edge_count: MAX_EXPRESSION_EDGE_COUNT,
        expression_depth: MAX_EXPRESSION_DEPTH,
        rational_component_bits: MAX_RATIONAL_COMPONENT_BITS,
        intermediate_component_bits: MAX_INTERMEDIATE_COMPONENT_BITS,
        dimension_abs_exponent: MAX_DIMENSION_ABS_EXPONENT,
        dimension_term_count: MAX_DIMENSION_TERM_COUNT,
        evaluation_steps: MAX_EVALUATION_STEPS,
        closure_steps: MAX_CLOSURE_STEPS,
        canonical_bytes: MAX_CANONICAL_BYTES,
        canonical_token_bytes: MAX_CANONICAL_TOKEN_BYTES,
        content_bytes: MAX_CONTENT_BYTES,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct VerifiedPhysicalMember {
    pub(super) identity: SpeciesContentIdentity,
    pub(super) physical_content: CanonicalArtifact,
    pub(super) rest_mass_si: ExactRationalWire,
    pub(super) mass_dimension: DimensionVector,
    pub(super) derivation_kind: ArtifactIdentity,
    pub(super) requirements: RequirementSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ValidatedRegistry {
    pub(super) members: Vec<VerifiedPhysicalMember>,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) resource_contract_sha256: [u8; 32],
}

/// Agreement is conditional on the admitted lower artifacts supplied to the
/// pair. It is not the dormant `SpeciesRegistryAuthority` and has no route to
/// the conditioned-support reducer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VerifiedPhysicalSpeciesRegistry {
    pub(super) members: Vec<VerifiedPhysicalMember>,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) producer_id: &'static str,
    pub(super) watchdog_id: &'static str,
    pub(super) authority_effect: AuthorityEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthorityEffect {
    None,
}

impl AuthorityEffect {
    pub(super) const fn id(self) -> &'static str {
        "none"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PhysicalRegistryRefusalCode {
    SchemaMismatch,
    CheckerPairMismatch,
    ResourceContractMismatch,
    FloorBindingMismatch,
    FloorCoordinateProjectionInvalid,
    FloorCoordinateProjectionCheckerDisagreement,
    PrimitiveProfileProjectionInvalid,
    ChargedProfileProjectionInvalid,
    PhysicalVocabularyBindingMismatch,
    StructureBindingMismatch,
    ArtifactCapacityExceeded,
    RegistryCapacityExceeded,
    ReferenceCapacityExceeded,
    ExpressionNodeCapacityExceeded,
    ExpressionEdgeCapacityExceeded,
    ExpressionDepthExceeded,
    RationalComponentLimitExceeded,
    IntermediateComponentLimitExceeded,
    DimensionExponentLimitExceeded,
    DimensionTermCapacityExceeded,
    DimensionEncodingInvalid,
    EvaluationStepLimitExceeded,
    ClosureStepLimitExceeded,
    CanonicalByteLimitExceeded,
    ContentByteLimitExceeded,
    CanonicalTextInvalid,
    MissingBindingDigest,
    EvidenceCustodyIsNotAdmission,
    NoncanonicalProvenance,
    DerivedAdmissionProvenanceMismatch,
    IrreducibleAdmissionProvenanceInvalid,
    GeneratedProvenanceCannotBeRoot,
    DuplicateAdmissionReceipt,
    DuplicateResidualSlot,
    AdmissionCapabilityMismatch,
    RationalEncodingInvalid,
    RationalNotReduced,
    ArtifactIdentityMismatch,
    DuplicateArtifactIdentity,
    ArtifactIdentityCollision,
    DuplicateRegistryMember,
    UnknownArtifactReference,
    ArtifactKindMismatch,
    RequirementSetEmpty,
    DuplicateRequirement,
    DependencyMismatch,
    UnknownSpeciesDependency,
    ExpressionEmpty,
    ExpressionOutputInvalid,
    ExpressionCycle,
    ExpressionContainsUnusedNode,
    DivisionByZero,
    DimensionMismatch,
    MassDimensionMismatch,
    MassProjectionNotAuthorizedForMember,
    MassUncertaintyTransportInvalid,
    NonPositiveMass,
    UnprovedExactZero,
    NoAdmittedSpeciesDerivationRoots,
    NoAdmittedSpeciesDerivationRules,
    EmptyRegistryIsNotClosure,
    DerivationCycle,
    DuplicateMemberDerivation,
    MissingClosureMember,
    ExtraClosureMember,
    CheckerDisagreement,
}

impl PhysicalRegistryRefusalCode {
    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::SchemaMismatch => "schema_mismatch",
            Self::CheckerPairMismatch => "checker_pair_mismatch",
            Self::ResourceContractMismatch => "resource_contract_mismatch",
            Self::FloorBindingMismatch => "floor_binding_mismatch",
            Self::FloorCoordinateProjectionInvalid => "floor_coordinate_projection_invalid",
            Self::FloorCoordinateProjectionCheckerDisagreement => {
                "floor_coordinate_projection_checker_disagreement"
            }
            Self::PrimitiveProfileProjectionInvalid => "primitive_profile_projection_invalid",
            Self::ChargedProfileProjectionInvalid => "charged_profile_projection_invalid",
            Self::PhysicalVocabularyBindingMismatch => "physical_vocabulary_binding_mismatch",
            Self::StructureBindingMismatch => "structure_binding_mismatch",
            Self::ArtifactCapacityExceeded => "artifact_capacity_exceeded",
            Self::RegistryCapacityExceeded => "registry_capacity_exceeded",
            Self::ReferenceCapacityExceeded => "reference_capacity_exceeded",
            Self::ExpressionNodeCapacityExceeded => "expression_node_capacity_exceeded",
            Self::ExpressionEdgeCapacityExceeded => "expression_edge_capacity_exceeded",
            Self::ExpressionDepthExceeded => "expression_depth_exceeded",
            Self::RationalComponentLimitExceeded => "rational_component_limit_exceeded",
            Self::IntermediateComponentLimitExceeded => "intermediate_component_limit_exceeded",
            Self::DimensionExponentLimitExceeded => "dimension_exponent_limit_exceeded",
            Self::DimensionTermCapacityExceeded => "dimension_term_capacity_exceeded",
            Self::DimensionEncodingInvalid => "dimension_encoding_invalid",
            Self::EvaluationStepLimitExceeded => "evaluation_step_limit_exceeded",
            Self::ClosureStepLimitExceeded => "closure_step_limit_exceeded",
            Self::CanonicalByteLimitExceeded => "canonical_byte_limit_exceeded",
            Self::ContentByteLimitExceeded => "content_byte_limit_exceeded",
            Self::CanonicalTextInvalid => "canonical_text_invalid",
            Self::MissingBindingDigest => "missing_binding_digest",
            Self::EvidenceCustodyIsNotAdmission => "evidence_custody_is_not_admission",
            Self::NoncanonicalProvenance => "noncanonical_provenance",
            Self::DerivedAdmissionProvenanceMismatch => "derived_admission_provenance_mismatch",
            Self::IrreducibleAdmissionProvenanceInvalid => {
                "irreducible_admission_provenance_invalid"
            }
            Self::GeneratedProvenanceCannotBeRoot => "generated_provenance_cannot_be_root",
            Self::DuplicateAdmissionReceipt => "duplicate_admission_receipt",
            Self::DuplicateResidualSlot => "duplicate_residual_slot",
            Self::AdmissionCapabilityMismatch => "admission_capability_mismatch",
            Self::RationalEncodingInvalid => "rational_encoding_invalid",
            Self::RationalNotReduced => "rational_not_reduced",
            Self::ArtifactIdentityMismatch => "artifact_identity_mismatch",
            Self::DuplicateArtifactIdentity => "duplicate_artifact_identity",
            Self::ArtifactIdentityCollision => "artifact_identity_collision",
            Self::DuplicateRegistryMember => "duplicate_registry_member",
            Self::UnknownArtifactReference => "unknown_artifact_reference",
            Self::ArtifactKindMismatch => "artifact_kind_mismatch",
            Self::RequirementSetEmpty => "requirement_set_empty",
            Self::DuplicateRequirement => "duplicate_requirement",
            Self::DependencyMismatch => "dependency_mismatch",
            Self::UnknownSpeciesDependency => "unknown_species_dependency",
            Self::ExpressionEmpty => "expression_empty",
            Self::ExpressionOutputInvalid => "expression_output_invalid",
            Self::ExpressionCycle => "expression_cycle",
            Self::ExpressionContainsUnusedNode => "expression_contains_unused_node",
            Self::DivisionByZero => "division_by_zero",
            Self::DimensionMismatch => "dimension_mismatch",
            Self::MassDimensionMismatch => "mass_dimension_mismatch",
            Self::MassProjectionNotAuthorizedForMember => {
                "mass_projection_not_authorized_for_member"
            }
            Self::MassUncertaintyTransportInvalid => "mass_uncertainty_transport_invalid",
            Self::NonPositiveMass => "non_positive_mass",
            Self::UnprovedExactZero => "unproved_exact_zero",
            Self::NoAdmittedSpeciesDerivationRoots => "no_admitted_species_derivation_roots",
            Self::NoAdmittedSpeciesDerivationRules => "no_admitted_species_derivation_rules",
            Self::EmptyRegistryIsNotClosure => "empty_registry_is_not_closure",
            Self::DerivationCycle => "derivation_cycle",
            Self::DuplicateMemberDerivation => "duplicate_member_derivation",
            Self::MissingClosureMember => "missing_closure_member",
            Self::ExtraClosureMember => "extra_closure_member",
            Self::CheckerDisagreement => "checker_disagreement",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalRegistryRefusal {
    pub(super) code: PhysicalRegistryRefusalCode,
    pub(super) member_count: u32,
    pub(super) coverage_claim: bool,
    pub(super) authority_effect: AuthorityEffect,
    pub(super) open_obligations: Vec<&'static str>,
}

impl PhysicalRegistryRefusal {
    pub(super) fn from_code(code: PhysicalRegistryRefusalCode) -> Self {
        let open_obligations = match code {
            PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRoots => vec![
                "admitted_physical_coordinate_roots",
                "floor_species_property_attribution",
                "admitted_physical_descriptor_roles",
                "admitted_physical_relation_targets",
                "admitted_constraint_laws",
                "admitted_species_derivation_rules",
                "complete_registry_closure_domain",
                "certified_mass_projection",
            ],
            PhysicalRegistryRefusalCode::NoAdmittedSpeciesDerivationRules => vec![
                "admitted_physical_descriptor_roles",
                "admitted_constraint_laws",
                "admitted_species_derivation_rules",
                "complete_global_physical_vocabulary_coverage",
                "complete_registry_closure_domain",
                "species_mass_uncertainty_transport",
            ],
            _ => Vec::new(),
        };
        Self {
            code,
            member_count: 0,
            coverage_claim: false,
            authority_effect: AuthorityEffect::None,
            open_obligations,
        }
    }
}
