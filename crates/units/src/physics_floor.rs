//! Independent admission authority for the absolute physical floor.
//!
//! Source custody and the seven provenance marks account for a candidate, but
//! neither authorizes it. This module is the lower-layer seal shared by the
//! canonical runner and every causal physics consumer. It checks the ordered
//! constant declarations against an independent registry, pins the complete
//! derive-first receipts by digest, and admits only that exact floor.

#[path = "physics_floor_source_evidence.rs"]
mod source_evidence;

use crate::authority_watchdog::{verify_floor_pi_budgets, AuthorityWatchdogError};
use crate::dimensional_analysis::SiDimensionColumn;
use crate::floor_admission_watchdog::{
    verify_floor_catalog_admission_bytes, FloorAdmissionWatchdogReceipt,
    FloorAdmissionWatchdogRejectionClass, CHECKER_IMPLEMENTATION_ID,
};
use crate::fundamentals::{
    Fundamental, FundamentalRole, SiDimension, COMPOSITES, PHYSICAL_INVARIANTS,
    REPRESENTATION_DEFINITIONS, SI_BASE_DIMENSION_IDS, SI_REPRESENTATION_SCHEMA_ID,
};
use civsim_ledger::{
    AbsolutePhysicsFloor, ChaosProtocolReceipt, ChaosRegimeReceipt, DerivationExhaustionReceipt,
    Entry, FloorAdmissionError, GapLawReceipt, Ledger, LedgerError, Provenance, ResidualLawReceipt,
    Tier,
};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;

fn fundamental_id(symbol: &str) -> String {
    format!("fundamental.{symbol}")
}

/// Number of independently admitted physical coordinates in the v1 floor.
pub const PHYSICAL_FLOOR_LEN: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalInvariantAdmission {
    symbol: &'static str,
    tier: Tier,
    provenance: Provenance,
    value: &'static str,
    unit: &'static str,
    dimension: SiDimension,
    source_id: &'static str,
    source_sha256: &'static str,
    source_anchor: &'static str,
    uncertainty_kind: &'static str,
    uncertainty_decimal: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RepresentationDefinitionFingerprint {
    symbol: &'static str,
    value: &'static str,
    unit: &'static str,
    dimension: SiDimension,
    source_id: &'static str,
    source_sha256: &'static str,
    source_anchor: &'static str,
}

const REPRESENTATION_DEFINITION_FINGERPRINTS: [RepresentationDefinitionFingerprint; 7] = [
    RepresentationDefinitionFingerprint {
        symbol: "Delta_nu_Cs",
        value: "9192631770",
        unit: "Hz",
        dimension: SiDimension::new(0, 0, -1, 0, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "hyperfine transition frequency of Cs-133",
    },
    RepresentationDefinitionFingerprint {
        symbol: "c",
        value: "299792458",
        unit: "m/s",
        dimension: SiDimension::new(1, 0, -1, 0, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "speed of light in vacuum",
    },
    RepresentationDefinitionFingerprint {
        symbol: "h",
        value: "6.62607015e-34",
        unit: "J*s",
        dimension: SiDimension::new(2, 1, -1, 0, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "Planck constant",
    },
    RepresentationDefinitionFingerprint {
        symbol: "e",
        value: "1.602176634e-19",
        unit: "C",
        dimension: SiDimension::new(0, 0, 1, 1, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "elementary charge",
    },
    RepresentationDefinitionFingerprint {
        symbol: "k_B",
        value: "1.380649e-23",
        unit: "J/K",
        dimension: SiDimension::new(2, 1, -2, 0, -1, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "Boltzmann constant",
    },
    RepresentationDefinitionFingerprint {
        symbol: "N_A",
        value: "6.02214076e23",
        unit: "1/mol",
        dimension: SiDimension::new(0, 0, 0, 0, 0, -1, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "Avogadro constant",
    },
    RepresentationDefinitionFingerprint {
        symbol: "K_cd",
        value: "683",
        unit: "lm/W",
        dimension: SiDimension::new(-2, -1, 3, 0, 0, 0, 1),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "luminous efficacy",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExecutionRelationFingerprint {
    symbol: &'static str,
    formula: &'static str,
    inputs: &'static [&'static str],
    unit: &'static str,
    dimension: SiDimension,
}

const EXECUTION_RELATION_FINGERPRINTS: [ExecutionRelationFingerprint; 4] = [
    ExecutionRelationFingerprint {
        symbol: "eps_0",
        formula: "e^2 / (2 * alpha * h * c)",
        inputs: &["e", "alpha", "h", "c"],
        unit: "F/m",
        dimension: SiDimension::new(-3, -1, 4, 2, 0, 0, 0),
    },
    ExecutionRelationFingerprint {
        symbol: "sigma",
        formula: "2 * pi^5 * k_B^4 / (15 * h^3 * c^2)",
        inputs: &["k_B", "h", "c"],
        unit: "W/(m^2*K^4)",
        dimension: SiDimension::new(0, 1, -3, 0, -4, 0, 0),
    },
    ExecutionRelationFingerprint {
        symbol: "R",
        formula: "N_A * k_B",
        inputs: &["N_A", "k_B"],
        unit: "J/(mol*K)",
        dimension: SiDimension::new(2, 1, -2, 0, -1, -1, 0),
    },
    ExecutionRelationFingerprint {
        symbol: "A3_per_cm3_mol",
        formula: "10^24 / N_A",
        inputs: &["N_A"],
        unit: "angstrom^3/(cm^3/mol)",
        dimension: SiDimension::new(0, 0, 0, 0, 0, 1, 0),
    },
];

const PHYSICAL_INVARIANT_ADMISSIONS: [PhysicalInvariantAdmission; 3] = [
    PhysicalInvariantAdmission {
        symbol: "alpha",
        tier: Tier::Universal,
        provenance: Provenance::Measured,
        value: "7.2973525693e-3",
        unit: "1",
        dimension: SiDimension::DIMENSIONLESS,
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "fine-structure constant",
        uncertainty_kind: "standard",
        uncertainty_decimal: "1.1e-12",
    },
    PhysicalInvariantAdmission {
        symbol: "G",
        tier: Tier::Universal,
        provenance: Provenance::Measured,
        value: "6.67430e-11",
        unit: "m^3/(kg*s^2)",
        dimension: SiDimension::new(3, -1, -2, 0, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "Newtonian constant of gravitation",
        uncertainty_kind: "standard",
        uncertainty_decimal: "1.5e-15",
    },
    PhysicalInvariantAdmission {
        symbol: "m_e",
        tier: Tier::Universal,
        provenance: Provenance::Measured,
        value: "9.1093837015e-31",
        unit: "kg",
        dimension: SiDimension::new(0, 1, 0, 0, 0, 0, 0),
        source_id: "nist_codata_2018_ascii",
        source_sha256: "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1",
        source_anchor: "electron mass",
        uncertainty_kind: "standard",
        uncertainty_decimal: "2.8e-40",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReceiptFingerprint {
    entry_id: &'static str,
    sha256: [u8; 32],
}

const DERIVATION_EXHAUSTION_SCHEMA_ID: &str = "civsim.floor.irreducible.derivation-exhaustion.v1";
const BUCKINGHAM_PI_SCHEMA_ID: &str = "civsim.floor.irreducible.buckingham-pi.v1";
const GAP_LAW_SCHEMA_ID: &str = "civsim.floor.irreducible.gap-law.v1";
const CHAOS_PROTOCOL_SCHEMA_ID: &str = "civsim.floor.irreducible.chaos-protocol.v1";
const RESIDUAL_LAW_SCHEMA_ID: &str = "civsim.floor.irreducible.residual-law.v1";
const RESIDUAL_SLOT_SCHEMA_ID: &str = "civsim.floor.irreducible.residual-slot.v1";
const OWNER_ADMISSION_SCHEMA_ID: &str = "civsim.floor.irreducible.owner-admission.v1";
const OWNER_ADMISSION_DECISION_ID: &str = "owner.admitted";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IrreducibleComponentDigests {
    derivation_exhaustion: [u8; 32],
    buckingham_pi: [u8; 32],
    gap_law: [u8; 32],
    chaos_protocol: [u8; 32],
    residual_law: [u8; 32],
    residual_slot: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IrreducibleAdmissionPin {
    entry_id: &'static str,
    component_digests: IrreducibleComponentDigests,
    owner_admission: [u8; 32],
}

const fn pinned_sha256(value: &str) -> [u8; 32] {
    const fn nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            _ => panic!("physical-floor SHA-256 pins must be lowercase hexadecimal"),
        }
    }

    let bytes = value.as_bytes();
    assert!(
        bytes.len() == 64,
        "physical-floor SHA-256 pins must contain 64 hexadecimal characters"
    );
    let mut digest = [0_u8; 32];
    let mut index = 0;
    while index < digest.len() {
        digest[index] = (nibble(bytes[index * 2]) << 4) | nibble(bytes[index * 2 + 1]);
        index += 1;
    }
    digest
}

// These digests are an authority separate from the receipt constructor. They
// cover every attempt, phenomenon, Pi budget, residual slot, Gap/Residual
// field, and typed Chaos Protocol branch using the length-prefixed v2 encoding
// below.
const RECEIPT_FINGERPRINTS: [ReceiptFingerprint; 3] = [
    ReceiptFingerprint {
        entry_id: "fundamental.alpha",
        sha256: [
            0xad, 0x4a, 0x55, 0xa1, 0x57, 0xe4, 0x41, 0x6d, 0x58, 0xee, 0x06, 0x62, 0xd1, 0x94,
            0xe2, 0x3c, 0x20, 0x20, 0x18, 0xee, 0x25, 0x69, 0xc5, 0x69, 0x0f, 0x85, 0x91, 0xe0,
            0xfc, 0xb2, 0x83, 0x97,
        ],
    },
    ReceiptFingerprint {
        entry_id: "fundamental.G",
        sha256: [
            0x84, 0x42, 0xf9, 0xb5, 0x8e, 0x7c, 0xbf, 0xf1, 0xab, 0xa3, 0x6a, 0x8c, 0x5f, 0xc7,
            0x31, 0xd2, 0xa1, 0xbd, 0xc5, 0x98, 0xdf, 0x0f, 0xfb, 0x77, 0x2f, 0xe0, 0xc3, 0x73,
            0xe2, 0x5c, 0x0c, 0xf5,
        ],
    },
    ReceiptFingerprint {
        entry_id: "fundamental.m_e",
        sha256: [
            0x2c, 0x40, 0xc6, 0xd1, 0x9d, 0xae, 0xf4, 0x20, 0xe1, 0xb2, 0xc9, 0xac, 0xa9, 0xb5,
            0x3a, 0x2c, 0xe7, 0xc7, 0xdd, 0x4d, 0x83, 0x7c, 0xfd, 0x3e, 0xb9, 0xdf, 0x6b, 0x8f,
            0xec, 0xe0, 0x77, 0xea,
        ],
    },
];

// These pins are the explicit, owner-reviewed admission capability for the
// complete typed irreducible route of each physical-floor leaf. The producer
// and independent watchdog both reconstruct the six component receipts before
// this decision can participate in the sealed floor.
const IRREDUCIBLE_ADMISSION_PINS: [IrreducibleAdmissionPin; 3] = [
    IrreducibleAdmissionPin {
        entry_id: "fundamental.G",
        component_digests: IrreducibleComponentDigests {
            derivation_exhaustion: pinned_sha256(
                "11e0bd7322675af01356adef991fe733607784bf4419ce27f1c8624d8dc395e4",
            ),
            buckingham_pi: pinned_sha256(
                "8d6d45803cc9a5c009e343153f1b8d6b0bf9fd12e10a63b08547218602289ac2",
            ),
            gap_law: pinned_sha256(
                "cee0090ae632694e5f6d1e0b2b9eb73486156b76ec6c96f926f8fb09364f99c0",
            ),
            chaos_protocol: pinned_sha256(
                "ea4d8b9a8a20e112c5b8ab2e0bf4676fa135aaec0f71784075fff31f6da20882",
            ),
            residual_law: pinned_sha256(
                "3afd2460306186cb97a27c9baf24710d9b1b317d0fe378d67648b25c7631bd85",
            ),
            residual_slot: pinned_sha256(
                "438b6cabfc18284c5a4e0b8e7da98bc0e01389e1cacad56b3a914179b8753feb",
            ),
        },
        owner_admission: pinned_sha256(
            "33203440a2a2531a7bf75b38d84ac9038d37cdcfd03b6cdf8428d3c5bfeefab4",
        ),
    },
    IrreducibleAdmissionPin {
        entry_id: "fundamental.alpha",
        component_digests: IrreducibleComponentDigests {
            derivation_exhaustion: pinned_sha256(
                "1eb5b2a316dff3c41c9977d6ae4bb80e3ac93b8c4ec50ff6b577fa155e6eb6ae",
            ),
            buckingham_pi: pinned_sha256(
                "b2cc3478181ff2ec99cb1b81e688e5f1152dcda6ac9405cdf9a43f6c193e1519",
            ),
            gap_law: pinned_sha256(
                "4bf62828abb84d9188838d141c2ac259d1a31ab7c8431c8ac92242328f7065d7",
            ),
            chaos_protocol: pinned_sha256(
                "1393549c161c151bbddd9cb80789c84a9017ba6f1a22e6c244d1920a5c91752f",
            ),
            residual_law: pinned_sha256(
                "3eea2cc69581146a37efcf3239768f00c0b37feeb886f26ab4632301a43fdcb6",
            ),
            residual_slot: pinned_sha256(
                "f9f8d5f9c86dae191d89f7b1c8773d2d1cff99e908ae762d45a33c8f7185ebf7",
            ),
        },
        owner_admission: pinned_sha256(
            "87dd3c7d0e16299650718a08660ab4c053cba6f16207dc9ed1a3db14b911ba31",
        ),
    },
    IrreducibleAdmissionPin {
        entry_id: "fundamental.m_e",
        component_digests: IrreducibleComponentDigests {
            derivation_exhaustion: pinned_sha256(
                "dbde9739032849acd9d742686f6edb4d0d456f0e2395d58184e91da19b6f6729",
            ),
            buckingham_pi: pinned_sha256(
                "56a3d44ec9c59b7615454a91e0a29807e5c6b195af174fd31d6acd09c8b0a9cf",
            ),
            gap_law: pinned_sha256(
                "f6417844044d867b5e396701968c301f37c1dccb3d40ace27ddf7a912223db7a",
            ),
            chaos_protocol: pinned_sha256(
                "4f355aead350d84362176a7e84d754b77726c3ab670f2e0e36602c5ab70782d7",
            ),
            residual_law: pinned_sha256(
                "fefe123c102eb146d0f7b460d19011fb7b8062b3f02fa8762e5e9d8e220e7bdd",
            ),
            residual_slot: pinned_sha256(
                "7ed9465fecc33d4fbb74b3d3e6a2d18ff28922778bc371c7d1fd5418de7f84ff",
            ),
        },
        owner_admission: pinned_sha256(
            "5adeff2f8457b47543636189722b9c7f898aad8e54f8be7b9721ae6753da9394",
        ),
    },
];

const FLOOR_ADMISSION_INPUT_SCHEMA_ID: &str = "civsim.units.floor-catalog-admission-input.v3";
const FLOOR_ADMISSION_PAIR_SCHEMA_ID: &str = "civsim.units.floor-catalog-admission-pair.v3";
const FLOOR_ADMISSION_CLAIM_ID: &str = "floor.catalog-admission";
const FLOOR_ADMISSION_PRODUCER_IMPLEMENTATION_ID: &str =
    "civsim.units.canonical-floor-irreducible-admission.v3";

// This receipt covers the exact Q32.32 constant table and its CPU/GPU source
// occurrences. It does not certify whole-domain kernel behavior.
const FIXED_MATH_TABLE_AUTHORITY_RECEIPT: &[u8] =
    include_bytes!("../data/fixed_math_authority_receipt.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FloorCatalogAdmissionPairReceipt {
    digest: [u8; 32],
}

impl FloorCatalogAdmissionPairReceipt {
    const fn digest(&self) -> [u8; 32] {
        self.digest
    }
}

/// Typed identity of the byte encoding used to bind the complete physical-floor
/// authority. A different field set, field order, or encoding requires a new
/// identity rather than silently changing the v4 digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFloorAuthoritySchemaId(&'static str);

impl PhysicalFloorAuthoritySchemaId {
    pub const V5: Self = Self("civsim.units.physical-floor-authority-binding.v5");

    /// Stable schema spelling for transcripts and proof artifacts.
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for PhysicalFloorAuthoritySchemaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

/// Read-only binding of every independent declaration and receipt authority
/// that defines the repository's physical floor.
///
/// Its fields are private and it has no caller-supplied constructor. Consumers
/// can record the typed schema and digest, but cannot manufacture a replacement
/// authority through this API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFloorAuthorityBinding {
    schema_id: PhysicalFloorAuthoritySchemaId,
    digest: [u8; 32],
}

impl PhysicalFloorAuthorityBinding {
    /// Verify the declarations and independent receipt pins before constructing
    /// the sole repository-owned v5 authority binding.
    pub fn sealed() -> Result<Self, AuditedCatalogError> {
        verify_units_declarations()?;
        let source_evidence = source_evidence::verify(&PHYSICAL_INVARIANT_ADMISSIONS)
            .map_err(AuditedCatalogError::DefinitionMismatch)?;
        let receipts = physical_invariant_receipts();
        verify_receipt_fingerprints(&receipts)?;
        let catalog = audited_substrate_ledger()?;
        let entries = catalog.entries().cloned().collect::<Vec<_>>();
        let admission_pair = verify_floor_catalog_admission_pair(&entries, &receipts)?;
        let pi_watchdog =
            verify_floor_pi_budgets(&receipts).map_err(AuditedCatalogError::Watchdog)?;
        canonical_repository_text(
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            "fixed-math authority receipt",
        )?;

        let schema_id = PhysicalFloorAuthoritySchemaId::V5;
        let digest = physical_floor_authority_digest(
            schema_id,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair.digest(),
            pi_watchdog.digest(),
            source_evidence.digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        if digest != EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "physical-floor authority binding under schema '{}' expected {} but found {}",
                schema_id,
                hex(&EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST),
                hex(&digest)
            )));
        }

        Ok(Self { schema_id, digest })
    }

    /// Typed identity of the digest encoding.
    pub const fn schema_id(&self) -> PhysicalFloorAuthoritySchemaId {
        self.schema_id
    }

    /// Raw SHA-256 digest for exact machine comparison.
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Lowercase hexadecimal SHA-256 digest for canonical text records.
    pub fn digest_hex(&self) -> String {
        hex(&self.digest)
    }
}

/// Construct the verified, read-only physical-floor authority binding.
pub fn sealed_physical_floor_authority_binding(
) -> Result<PhysicalFloorAuthorityBinding, AuditedCatalogError> {
    PhysicalFloorAuthorityBinding::sealed()
}

/// Read-only binding for one admitted floor leaf's complete derive-first
/// exhaustion receipt. The digest is one of the independently reviewed pins
/// included in the sealed physical-floor authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFloorReceiptFingerprint {
    entry_id: &'static str,
    schema_id: &'static str,
    digest: [u8; 32],
}

impl PhysicalFloorReceiptFingerprint {
    /// Stable floor-entry identity covered by this fingerprint.
    pub const fn entry_id(self) -> &'static str {
        self.entry_id
    }

    /// Versioned encoding used to fingerprint the complete receipt.
    pub const fn schema_id(self) -> &'static str {
        self.schema_id
    }

    /// Raw SHA-256 digest for exact downstream ancestry binding.
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

/// Return the independently pinned exhaustion-receipt fingerprints only after
/// the complete repository floor authority has replayed successfully.
pub fn sealed_physical_floor_receipt_fingerprints(
) -> Result<[PhysicalFloorReceiptFingerprint; PHYSICAL_FLOOR_LEN], AuditedCatalogError> {
    let _authority = sealed_physical_floor_authority_binding()?;
    Ok(
        RECEIPT_FINGERPRINTS.map(|fingerprint| PhysicalFloorReceiptFingerprint {
            entry_id: fingerprint.entry_id,
            schema_id: RECEIPT_FINGERPRINT_SCHEMA_ID,
            digest: fingerprint.sha256,
        }),
    )
}

/// One identity-keyed, fully typed irreducible-admission capability from the
/// sealed physical floor.
///
/// The six route receipts are separately domain-bound. The owner receipt pins
/// their exact tuple, while the independent-watchdog receipt binds the
/// floor-catalog pair and Buckingham-Pi watchdog that replayed it. Construction
/// is private and succeeds only after the complete v5 floor authority verifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFloorIrreducibleAdmissionBinding {
    entry_id: &'static str,
    derivation_exhaustion_receipt: [u8; 32],
    buckingham_pi_receipt: [u8; 32],
    gap_law_receipt: [u8; 32],
    chaos_protocol_receipt: [u8; 32],
    residual_law_receipt: [u8; 32],
    residual_slot_receipt: [u8; 32],
    owner_admission_receipt: [u8; 32],
    independent_watchdog_receipt: [u8; 32],
}

impl PhysicalFloorIrreducibleAdmissionBinding {
    pub const SCHEMA_ID: &'static str =
        "civsim.units.physical-floor-irreducible-admission-binding.v1";

    /// Stable schema identity for the complete per-leaf route binding.
    pub const fn schema_id(self) -> &'static str {
        Self::SCHEMA_ID
    }

    /// Stable floor-leaf identity.
    pub const fn entry_id(self) -> &'static str {
        self.entry_id
    }

    /// Derive-first exhaustion receipt digest.
    pub const fn derivation_exhaustion_receipt(self) -> [u8; 32] {
        self.derivation_exhaustion_receipt
    }

    /// Buckingham-Pi declaration receipt digest.
    pub const fn buckingham_pi_receipt(self) -> [u8; 32] {
        self.buckingham_pi_receipt
    }

    /// Gap-Law receipt digest.
    pub const fn gap_law_receipt(self) -> [u8; 32] {
        self.gap_law_receipt
    }

    /// Typed Chaos-Protocol branch receipt digest.
    pub const fn chaos_protocol_receipt(self) -> [u8; 32] {
        self.chaos_protocol_receipt
    }

    /// Residual-Law receipt digest.
    pub const fn residual_law_receipt(self) -> [u8; 32] {
        self.residual_law_receipt
    }

    /// Unique residual-slot receipt digest.
    pub const fn residual_slot_receipt(self) -> [u8; 32] {
        self.residual_slot_receipt
    }

    /// Exact owner-admission decision over the six typed route receipts.
    pub const fn owner_admission_receipt(self) -> [u8; 32] {
        self.owner_admission_receipt
    }

    /// Independent floor-catalog and Buckingham-Pi replay receipt.
    pub const fn independent_watchdog_receipt(self) -> [u8; 32] {
        self.independent_watchdog_receipt
    }
}

fn independent_floor_leaf_watchdog_digest(
    entry_id: &str,
    floor_admission_pair_digest: [u8; 32],
    pi_watchdog_digest: [u8; 32],
) -> [u8; 32] {
    let mut digest = LengthPrefixedSha256::new();
    digest.text("civsim.units.physical-floor-leaf-independent-watchdog.v1");
    digest.text(entry_id);
    digest.bytes(&floor_admission_pair_digest);
    digest.bytes(&pi_watchdog_digest);
    digest.finish()
}

/// Return the exact identity-ordered irreducible-admission capabilities after
/// replaying the complete sealed physical-floor authority.
pub fn sealed_physical_floor_irreducible_admissions(
) -> Result<[PhysicalFloorIrreducibleAdmissionBinding; PHYSICAL_FLOOR_LEN], AuditedCatalogError> {
    let _authority = sealed_physical_floor_authority_binding()?;
    let receipts = physical_invariant_receipts();
    verify_irreducible_admission_pins(&receipts, &IRREDUCIBLE_ADMISSION_PINS)?;
    let entries = audited_substrate_ledger()?
        .entries()
        .cloned()
        .collect::<Vec<_>>();
    let floor_pair = verify_floor_catalog_admission_pair(&entries, &receipts)?;
    let pi_watchdog = verify_floor_pi_budgets(&receipts).map_err(AuditedCatalogError::Watchdog)?;

    Ok(
        IRREDUCIBLE_ADMISSION_PINS.map(|pin| PhysicalFloorIrreducibleAdmissionBinding {
            entry_id: pin.entry_id,
            derivation_exhaustion_receipt: pin.component_digests.derivation_exhaustion,
            buckingham_pi_receipt: pin.component_digests.buckingham_pi,
            gap_law_receipt: pin.component_digests.gap_law,
            chaos_protocol_receipt: pin.component_digests.chaos_protocol,
            residual_law_receipt: pin.component_digests.residual_law,
            residual_slot_receipt: pin.component_digests.residual_slot,
            owner_admission_receipt: pin.owner_admission,
            independent_watchdog_receipt: independent_floor_leaf_watchdog_digest(
                pin.entry_id,
                floor_pair.digest(),
                pi_watchdog.digest(),
            ),
        }),
    )
}

/// Return the exact ordered measured definitions covered by the sealed
/// physical-floor authority.
///
/// This is a read-only authority projection. It does not admit caller values
/// and it excludes every SI representation definition by construction.
pub fn sealed_physical_floor_definitions(
) -> Result<[Fundamental; PHYSICAL_FLOOR_LEN], AuditedCatalogError> {
    let _authority = sealed_physical_floor_authority_binding()?;
    Ok(PHYSICAL_INVARIANTS)
}

/// Exact ordered identities and SI dimensions of the admitted physical floor.
///
/// This projection exposes no magnitudes, sources, or caller lookup surface.
/// Downstream dimensional proofs can therefore bind to the same verified
/// authority without duplicating the private admission table.
pub fn sealed_physical_floor_dimension_columns(
) -> Result<Vec<SiDimensionColumn>, AuditedCatalogError> {
    let _authority = sealed_physical_floor_authority_binding()?;
    Ok(PHYSICAL_INVARIANT_ADMISSIONS
        .iter()
        .map(|admission| {
            SiDimensionColumn::new(&fundamental_id(admission.symbol), admission.dimension)
        })
        .collect())
}

// This pin is deliberately independent of the declaration tables and digest
// constructor above. It is updated only after reviewing a schema-versioned
// authority change.
const EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST: [u8; 32] = [
    0x23, 0xf4, 0x13, 0x20, 0x3f, 0xa4, 0x63, 0x92, 0x5e, 0xf5, 0x63, 0xdd, 0x8b, 0x83, 0xbc, 0x46,
    0x0e, 0x60, 0x98, 0x22, 0x1a, 0xdc, 0xcb, 0x3c, 0x7c, 0x2a, 0x66, 0x11, 0x25, 0x94, 0xa3, 0xef,
];

struct LengthPrefixedSha256(Sha256);

impl LengthPrefixedSha256 {
    fn new() -> Self {
        Self(Sha256::new())
    }

    fn bytes(&mut self, value: &[u8]) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value);
    }

    fn text(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn count(&mut self, value: usize) {
        self.bytes(&(value as u64).to_le_bytes());
    }

    fn dimension(&mut self, value: SiDimension) {
        let exponents = value.exponents();
        self.count(exponents.len());
        for exponent in exponents {
            self.bytes(&exponent.to_le_bytes());
        }
    }

    fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

fn irreducible_component_digests(
    receipt: &DerivationExhaustionReceipt,
) -> IrreducibleComponentDigests {
    let mut derivation_exhaustion = LengthPrefixedSha256::new();
    derivation_exhaustion.text(DERIVATION_EXHAUSTION_SCHEMA_ID);
    derivation_exhaustion.text(&receipt.entry_id);
    derivation_exhaustion.text(&receipt.phenomenon);
    derivation_exhaustion.count(receipt.derivation_attempts.len());
    for attempt in &receipt.derivation_attempts {
        derivation_exhaustion.text(attempt);
    }

    let mut buckingham_pi = LengthPrefixedSha256::new();
    buckingham_pi.text(BUCKINGHAM_PI_SCHEMA_ID);
    buckingham_pi.text(&receipt.entry_id);
    buckingham_pi.text(&receipt.phenomenon);
    buckingham_pi.count(receipt.buckingham_pi_groups);

    let mut gap_law = LengthPrefixedSha256::new();
    gap_law.text(GAP_LAW_SCHEMA_ID);
    gap_law.text(&receipt.entry_id);
    gap_law.text(&receipt.gap_law.reference_validity);
    gap_law.text(&receipt.gap_law.gap_dispatch);
    gap_law.text(&receipt.gap_law.smooth_systematics);
    gap_law.text(&receipt.gap_law.scale_free_limit);

    let mut chaos_protocol = LengthPrefixedSha256::new();
    chaos_protocol.text(CHAOS_PROTOCOL_SCHEMA_ID);
    chaos_protocol.text(&receipt.entry_id);
    match &receipt.gap_law.chaos_protocol {
        ChaosProtocolReceipt::NotApplicable { basis } => {
            chaos_protocol.text("not_applicable");
            chaos_protocol.text(basis);
        }
        ChaosProtocolReceipt::Dynamical {
            classification,
            regime_partition,
            transition_law,
            regimes,
        } => {
            chaos_protocol.text("dynamical");
            chaos_protocol.text(classification);
            chaos_protocol.text(regime_partition);
            chaos_protocol.text(transition_law);
            chaos_protocol.count(regimes.len());
            for regime in regimes {
                match regime {
                    ChaosRegimeReceipt::ResolvedTrajectory {
                        validity_domain,
                        resolution_bound,
                        evolution_postcondition,
                        exact_replay,
                    } => {
                        chaos_protocol.text("resolved_trajectory");
                        chaos_protocol.text(validity_domain);
                        chaos_protocol.text(resolution_bound);
                        chaos_protocol.text(evolution_postcondition);
                        chaos_protocol.text(exact_replay);
                    }
                    ChaosRegimeReceipt::SubresolutionMeasure {
                        validity_domain,
                        stationary_measure,
                        conservation_projection,
                        stability_postcondition,
                        coordinate_discipline,
                        exact_replay,
                    } => {
                        chaos_protocol.text("subresolution_measure");
                        chaos_protocol.text(validity_domain);
                        chaos_protocol.text(stationary_measure);
                        chaos_protocol.text(conservation_projection);
                        chaos_protocol.text(stability_postcondition);
                        chaos_protocol.text(coordinate_discipline);
                        chaos_protocol.text(exact_replay);
                    }
                }
            }
        }
    }

    let mut residual_law = LengthPrefixedSha256::new();
    residual_law.text(RESIDUAL_LAW_SCHEMA_ID);
    residual_law.text(&receipt.entry_id);
    residual_law.text(&receipt.residual_law.conservation);
    residual_law.text(&receipt.residual_law.disequilibrium);
    residual_law.text(&receipt.residual_law.fluctuation_dissipation);
    residual_law.text(&receipt.residual_law.dimensional_analysis);

    let mut residual_slot = LengthPrefixedSha256::new();
    residual_slot.text(RESIDUAL_SLOT_SCHEMA_ID);
    residual_slot.text(&receipt.entry_id);
    residual_slot.text(&receipt.phenomenon);
    residual_slot.text(&receipt.residual_slot);

    IrreducibleComponentDigests {
        derivation_exhaustion: derivation_exhaustion.finish(),
        buckingham_pi: buckingham_pi.finish(),
        gap_law: gap_law.finish(),
        chaos_protocol: chaos_protocol.finish(),
        residual_law: residual_law.finish(),
        residual_slot: residual_slot.finish(),
    }
}

fn owner_admission_digest(entry_id: &str, components: IrreducibleComponentDigests) -> [u8; 32] {
    let mut digest = LengthPrefixedSha256::new();
    digest.text(OWNER_ADMISSION_SCHEMA_ID);
    digest.text(entry_id);
    digest.text(OWNER_ADMISSION_DECISION_ID);
    digest.bytes(&components.derivation_exhaustion);
    digest.bytes(&components.buckingham_pi);
    digest.bytes(&components.gap_law);
    digest.bytes(&components.chaos_protocol);
    digest.bytes(&components.residual_law);
    digest.bytes(&components.residual_slot);
    digest.finish()
}

fn verify_irreducible_admission_pins(
    receipts: &[DerivationExhaustionReceipt],
    pins: &[IrreducibleAdmissionPin],
) -> Result<(), AuditedCatalogError> {
    let mut canonical_receipts = receipts.iter().collect::<Vec<_>>();
    canonical_receipts.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    if canonical_receipts.len() != pins.len() {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "{} irreducible floor receipts have {} owner-admission pins",
            canonical_receipts.len(),
            pins.len()
        )));
    }
    let mut mismatches = Vec::new();
    for (receipt, pin) in canonical_receipts.into_iter().zip(pins) {
        let components = irreducible_component_digests(receipt);
        let owner_admission = owner_admission_digest(&receipt.entry_id, components);
        if receipt.entry_id != pin.entry_id
            || components != pin.component_digests
            || owner_admission != pin.owner_admission
        {
            mismatches.push(format!(
                "{} components={} owner={}",
                receipt.entry_id,
                irreducible_component_digest_hex(components),
                hex(&owner_admission)
            ));
        }
    }
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(AuditedCatalogError::DefinitionMismatch(format!(
            "irreducible owner-admission mismatch: {}",
            mismatches.join("; ")
        )))
    }
}

fn irreducible_component_digest_hex(components: IrreducibleComponentDigests) -> String {
    [
        ("derivation_exhaustion", components.derivation_exhaustion),
        ("buckingham_pi", components.buckingham_pi),
        ("gap_law", components.gap_law),
        ("chaos_protocol", components.chaos_protocol),
        ("residual_law", components.residual_law),
        ("residual_slot", components.residual_slot),
    ]
    .into_iter()
    .map(|(label, digest)| format!("{label}:{}", hex(&digest)))
    .collect::<Vec<_>>()
    .join(",")
}

fn canonical_repository_text<'a>(
    raw: &'a [u8],
    label: &str,
) -> Result<Cow<'a, [u8]>, AuditedCatalogError> {
    if !raw.contains(&b'\r') {
        return Ok(Cow::Borrowed(raw));
    }
    let mut canonical = Vec::with_capacity(raw.len());
    let mut cursor = 0;
    while cursor < raw.len() {
        if raw[cursor] != b'\r' {
            canonical.push(raw[cursor]);
            cursor += 1;
            continue;
        }
        if raw.get(cursor + 1) != Some(&b'\n') {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "{label} contains a carriage return not paired with line feed"
            )));
        }
        canonical.push(b'\n');
        cursor += 2;
    }
    Ok(Cow::Owned(canonical))
}

#[allow(clippy::too_many_arguments)]
fn physical_floor_authority_digest(
    schema_id: PhysicalFloorAuthoritySchemaId,
    representation_schema_id: &str,
    base_dimension_ids: &[&str],
    representation_fingerprints: &[RepresentationDefinitionFingerprint],
    physical_admissions: &[PhysicalInvariantAdmission],
    receipt_fingerprint_schema_id: &str,
    receipt_fingerprints: &[ReceiptFingerprint],
    floor_admission_pair_digest: [u8; 32],
    pi_watchdog_digest: [u8; 32],
    source_evidence_digest: [u8; 32],
    fixed_math_table_authority_receipt: &[u8],
    execution_relation_fingerprints: &[ExecutionRelationFingerprint],
) -> [u8; 32] {
    let canonical_fixed_math_receipt = canonical_repository_text(
        fixed_math_table_authority_receipt,
        "fixed-math authority receipt",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let mut encoder = LengthPrefixedSha256::new();
    encoder.text(schema_id.as_str());

    encoder.text("representation_schema");
    encoder.text(representation_schema_id);
    encoder.count(base_dimension_ids.len());
    for dimension_id in base_dimension_ids {
        encoder.text(dimension_id);
    }

    encoder.text("ordered_representation_fingerprints");
    encoder.count(representation_fingerprints.len());
    for fingerprint in representation_fingerprints {
        encoder.text(fingerprint.symbol);
        encoder.text(fingerprint.value);
        encoder.text(fingerprint.unit);
        encoder.dimension(fingerprint.dimension);
        encoder.text(fingerprint.source_id);
        encoder.text(fingerprint.source_sha256);
        encoder.text(fingerprint.source_anchor);
    }

    encoder.text("ordered_physical_invariant_admissions");
    encoder.count(physical_admissions.len());
    for admission in physical_admissions {
        encoder.text(admission.symbol);
        encoder.text(admission.tier.id());
        encoder.text(admission.provenance.tag());
        encoder.text(admission.value);
        encoder.text(admission.unit);
        encoder.dimension(admission.dimension);
        encoder.text(admission.source_id);
        encoder.text(admission.source_sha256);
        encoder.text(admission.source_anchor);
        encoder.text(admission.uncertainty_kind);
        encoder.text(admission.uncertainty_decimal);
    }

    encoder.text("receipt_fingerprint_authority");
    encoder.text(receipt_fingerprint_schema_id);
    encoder.count(receipt_fingerprints.len());
    for fingerprint in receipt_fingerprints {
        encoder.text(fingerprint.entry_id);
        encoder.bytes(&fingerprint.sha256);
    }

    encoder.text("independent_floor_catalog_admission_pair");
    encoder.bytes(&floor_admission_pair_digest);

    encoder.text("independent_pi_watchdog");
    encoder.bytes(&pi_watchdog_digest);

    encoder.text("independent_codata_source_evidence");
    encoder.text(source_evidence::EVIDENCE_SCHEMA_ID);
    encoder.bytes(&source_evidence_digest);

    encoder.text("exact_fixed_math_table_authority_receipt");
    encoder.bytes(canonical_fixed_math_receipt.as_ref());

    encoder.text("ordered_execution_relation_fingerprints");
    encoder.count(execution_relation_fingerprints.len());
    for fingerprint in execution_relation_fingerprints {
        encoder.text(fingerprint.symbol);
        encoder.text(fingerprint.formula);
        encoder.count(fingerprint.inputs.len());
        for input in fingerprint.inputs {
            encoder.text(input);
        }
        encoder.text(fingerprint.unit);
        encoder.dimension(fingerprint.dimension);
    }

    encoder.finish()
}

struct CanonicalByteWriter {
    bytes: Vec<u8>,
}

impl CanonicalByteWriter {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn count(&mut self, value: usize) {
        self.bytes.extend_from_slice(&(value as u64).to_le_bytes());
    }

    fn text(&mut self, value: &str) {
        self.count(value.len());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn digest(&mut self, value: &[u8; 32]) {
        self.bytes.extend_from_slice(value);
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

fn floor_catalog_admission_bytes(
    entries: &[Entry],
    receipts: &[DerivationExhaustionReceipt],
    owner_admissions: &[IrreducibleAdmissionPin],
) -> Vec<u8> {
    let mut canonical_entries = entries.iter().collect::<Vec<_>>();
    canonical_entries.sort_by(|left, right| left.id.cmp(&right.id));
    let mut canonical_receipts = receipts.iter().collect::<Vec<_>>();
    canonical_receipts.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    let mut canonical_owner_admissions = owner_admissions.iter().collect::<Vec<_>>();
    canonical_owner_admissions.sort_by(|left, right| left.entry_id.cmp(right.entry_id));

    let mut writer = CanonicalByteWriter::new();
    writer.text(FLOOR_ADMISSION_INPUT_SCHEMA_ID);
    writer.count(canonical_entries.len());
    for entry in canonical_entries {
        writer.text(&entry.id);
        writer.text(entry.tier.id());
        writer.text(entry.provenance.tag());
        writer.count(entry.inputs.len());
        let mut canonical_inputs = entry.inputs.iter().collect::<Vec<_>>();
        canonical_inputs.sort();
        for input in canonical_inputs {
            writer.text(input);
        }
    }

    writer.count(canonical_receipts.len());
    for receipt in canonical_receipts {
        writer.text(&receipt.entry_id);
        writer.text(&receipt.phenomenon);
        writer.count(receipt.derivation_attempts.len());
        for attempt in &receipt.derivation_attempts {
            writer.text(attempt);
        }
        writer.text(&receipt.residual_slot);
        writer.count(receipt.buckingham_pi_groups);
        writer.text(&receipt.gap_law.reference_validity);
        writer.text(&receipt.gap_law.gap_dispatch);
        writer.text(&receipt.gap_law.smooth_systematics);
        writer.text(&receipt.gap_law.scale_free_limit);
        match &receipt.gap_law.chaos_protocol {
            ChaosProtocolReceipt::NotApplicable { basis } => {
                writer.text("not_applicable");
                writer.text(basis);
            }
            ChaosProtocolReceipt::Dynamical {
                classification,
                regime_partition,
                transition_law,
                regimes,
            } => {
                writer.text("dynamical");
                writer.text(classification);
                writer.text(regime_partition);
                writer.text(transition_law);
                writer.count(regimes.len());
                for regime in regimes {
                    match regime {
                        ChaosRegimeReceipt::ResolvedTrajectory {
                            validity_domain,
                            resolution_bound,
                            evolution_postcondition,
                            exact_replay,
                        } => {
                            writer.text("resolved_trajectory");
                            writer.text(validity_domain);
                            writer.text(resolution_bound);
                            writer.text(evolution_postcondition);
                            writer.text(exact_replay);
                        }
                        ChaosRegimeReceipt::SubresolutionMeasure {
                            validity_domain,
                            stationary_measure,
                            conservation_projection,
                            stability_postcondition,
                            coordinate_discipline,
                            exact_replay,
                        } => {
                            writer.text("subresolution_measure");
                            writer.text(validity_domain);
                            writer.text(stationary_measure);
                            writer.text(conservation_projection);
                            writer.text(stability_postcondition);
                            writer.text(coordinate_discipline);
                            writer.text(exact_replay);
                        }
                    }
                }
            }
        }
        writer.text(&receipt.residual_law.conservation);
        writer.text(&receipt.residual_law.disequilibrium);
        writer.text(&receipt.residual_law.fluctuation_dissipation);
        writer.text(&receipt.residual_law.dimensional_analysis);
    }

    writer.count(canonical_owner_admissions.len());
    for admission in canonical_owner_admissions {
        writer.text(admission.entry_id);
        writer.text(OWNER_ADMISSION_SCHEMA_ID);
        writer.text(OWNER_ADMISSION_DECISION_ID);
        writer.digest(&admission.component_digests.derivation_exhaustion);
        writer.digest(&admission.component_digests.buckingham_pi);
        writer.digest(&admission.component_digests.gap_law);
        writer.digest(&admission.component_digests.chaos_protocol);
        writer.digest(&admission.component_digests.residual_law);
        writer.digest(&admission.component_digests.residual_slot);
        writer.digest(&admission.owner_admission);
    }
    writer.finish()
}

fn producer_accepts_floor_claim(
    entries: &[Entry],
    receipts: &[DerivationExhaustionReceipt],
    owner_admissions: &[IrreducibleAdmissionPin],
) -> bool {
    let Ok(ledger) = Ledger::build(entries.iter().cloned()) else {
        return false;
    };
    AbsolutePhysicsFloor::admit(ledger, receipts.iter().cloned()).is_ok()
        && verify_irreducible_admission_pins(receipts, owner_admissions).is_ok()
}

fn producer_result_digest(
    input_digest: [u8; 32],
    entry_count: usize,
    receipt_count: usize,
) -> [u8; 32] {
    let mut result = Sha256::new();
    result.update(FLOOR_ADMISSION_PRODUCER_IMPLEMENTATION_ID.as_bytes());
    result.update(input_digest);
    result.update((entry_count as u64).to_le_bytes());
    result.update((receipt_count as u64).to_le_bytes());
    result.update(b"admitted");
    result.finalize().into()
}

fn record_floor_admission_canary(
    digest: &mut LengthPrefixedSha256,
    id: &str,
    entries: &[Entry],
    receipts: &[DerivationExhaustionReceipt],
    owner_admissions: &[IrreducibleAdmissionPin],
    expected_producer_acceptance: bool,
    expected_checker_rejection: FloorAdmissionWatchdogRejectionClass,
) -> Result<(), AuditedCatalogError> {
    let bytes = floor_catalog_admission_bytes(entries, receipts, owner_admissions);
    let producer_accepted = producer_accepts_floor_claim(entries, receipts, owner_admissions);
    let checker_rejection = match verify_floor_catalog_admission_bytes(&bytes) {
        Ok(_) => {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "floor-admission mutation canary '{id}' expected checker rejection '{}' but the checker accepted",
                expected_checker_rejection.stable_id()
            )));
        }
        Err(error) => error.rejection_class(),
    };
    if producer_accepted != expected_producer_acceptance
        || checker_rejection != expected_checker_rejection
    {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "floor-admission mutation canary '{id}' expected producer={expected_producer_acceptance}, checker={} but observed producer={producer_accepted}, checker={}",
            expected_checker_rejection.stable_id(),
            checker_rejection.stable_id()
        )));
    }
    digest.text(id);
    digest.text(if producer_accepted {
        "producer.accepted"
    } else {
        "producer.refused"
    });
    digest.text(checker_rejection.stable_id());
    Ok(())
}

fn floor_admission_canary_digest(
    entries: &[Entry],
    receipts: &[DerivationExhaustionReceipt],
    owner_admissions: &[IrreducibleAdmissionPin],
) -> Result<[u8; 32], AuditedCatalogError> {
    if entries.is_empty() || receipts.is_empty() {
        return Err(AuditedCatalogError::DefinitionMismatch(
            "floor-admission canaries require a nonempty canonical claim".into(),
        ));
    }
    let mut digest = LengthPrefixedSha256::new();
    digest.text("civsim.units.floor-catalog-admission-canaries.v3");

    let canonical_bytes = floor_catalog_admission_bytes(entries, receipts, owner_admissions);
    let mut permuted_entries = entries.to_vec();
    permuted_entries.reverse();
    let mut permuted_receipts = receipts.to_vec();
    permuted_receipts.reverse();
    let permuted_bytes =
        floor_catalog_admission_bytes(&permuted_entries, &permuted_receipts, owner_admissions);
    if !producer_accepts_floor_claim(&permuted_entries, &permuted_receipts, owner_admissions)
        || canonical_bytes != permuted_bytes
        || verify_floor_catalog_admission_bytes(&permuted_bytes).is_err()
    {
        return Err(AuditedCatalogError::DefinitionMismatch(
            "floor-admission arrival-order permutation changed canonical authority".into(),
        ));
    }
    digest.text("arrival_order_permutation");
    digest.text("producer.accepted");
    digest.text("checker.accepted");
    digest.bytes(&Sha256::digest(&canonical_bytes));

    let mut changed_tier = entries.to_vec();
    changed_tier[0].tier = Tier::Reference;
    record_floor_admission_canary(
        &mut digest,
        "changed_tier",
        &changed_tier,
        receipts,
        owner_admissions,
        true,
        FloorAdmissionWatchdogRejectionClass::CanonicalInputCustody,
    )?;

    let mut changed_provenance = entries.to_vec();
    changed_provenance[0].provenance = Provenance::Estimator;
    record_floor_admission_canary(
        &mut digest,
        "changed_provenance",
        &changed_provenance,
        receipts,
        owner_admissions,
        false,
        FloorAdmissionWatchdogRejectionClass::SemanticValidation,
    )?;

    let mut changed_member = entries.to_vec();
    let old_id = changed_member[0].id.clone();
    changed_member[0].id.push_str(".mutated");
    let mut changed_member_receipts = receipts.to_vec();
    for receipt in &mut changed_member_receipts {
        if receipt.entry_id == old_id {
            receipt.entry_id = changed_member[0].id.clone();
        }
    }
    record_floor_admission_canary(
        &mut digest,
        "changed_catalog_member",
        &changed_member,
        &changed_member_receipts,
        owner_admissions,
        false,
        FloorAdmissionWatchdogRejectionClass::SemanticValidation,
    )?;

    let mut missing_receipt_semantics = receipts.to_vec();
    missing_receipt_semantics[0].gap_law.reference_validity = "   ".into();
    record_floor_admission_canary(
        &mut digest,
        "missing_receipt_semantics",
        entries,
        &missing_receipt_semantics,
        owner_admissions,
        false,
        FloorAdmissionWatchdogRejectionClass::SemanticValidation,
    )?;

    let mut changed_receipt_authority = receipts.to_vec();
    changed_receipt_authority[0].derivation_attempts[0]
        .push_str(" copied authority-changing suffix");
    record_floor_admission_canary(
        &mut digest,
        "changed_receipt_authority",
        entries,
        &changed_receipt_authority,
        owner_admissions,
        false,
        FloorAdmissionWatchdogRejectionClass::SemanticValidation,
    )?;

    let mut changed_owner_admission = owner_admissions.to_vec();
    changed_owner_admission[0].owner_admission[0] ^= 1;
    record_floor_admission_canary(
        &mut digest,
        "changed_owner_admission",
        entries,
        receipts,
        &changed_owner_admission,
        false,
        FloorAdmissionWatchdogRejectionClass::SemanticValidation,
    )?;

    Ok(digest.finish())
}

fn verify_floor_catalog_admission_pair(
    entries: &[Entry],
    receipts: &[DerivationExhaustionReceipt],
) -> Result<FloorCatalogAdmissionPairReceipt, AuditedCatalogError> {
    verify_irreducible_admission_pins(receipts, &IRREDUCIBLE_ADMISSION_PINS)?;
    let ledger = Ledger::build(entries.iter().cloned()).map_err(AuditedCatalogError::Ledger)?;
    let floor = AbsolutePhysicsFloor::admit(ledger, receipts.iter().cloned()).map_err(|error| {
        AuditedCatalogError::DefinitionMismatch(format!(
            "floor-admission producer refused canonical input: {error}"
        ))
    })?;
    let bytes = floor_catalog_admission_bytes(entries, receipts, &IRREDUCIBLE_ADMISSION_PINS);
    let input_digest: [u8; 32] = Sha256::digest(&bytes).into();
    let producer_result = producer_result_digest(input_digest, floor.len(), receipts.len());
    let checker: FloorAdmissionWatchdogReceipt = verify_floor_catalog_admission_bytes(&bytes)
        .map_err(|error| {
            AuditedCatalogError::DefinitionMismatch(format!(
                "independent floor-admission checker refused canonical input: {error}"
            ))
        })?;
    if checker.input_digest() != input_digest
        || checker.entry_count() != floor.len()
        || checker.receipt_count() != receipts.len()
    {
        return Err(AuditedCatalogError::DefinitionMismatch(
            "floor-admission producer and checker did not agree on the exact input or row counts"
                .into(),
        ));
    }
    let canary_digest =
        floor_admission_canary_digest(entries, receipts, &IRREDUCIBLE_ADMISSION_PINS)?;

    let mut pair = LengthPrefixedSha256::new();
    pair.text(FLOOR_ADMISSION_PAIR_SCHEMA_ID);
    pair.text(FLOOR_ADMISSION_CLAIM_ID);
    pair.text(FLOOR_ADMISSION_PRODUCER_IMPLEMENTATION_ID);
    pair.text(CHECKER_IMPLEMENTATION_ID);
    pair.bytes(&input_digest);
    pair.bytes(&producer_result);
    pair.bytes(&checker.result_digest());
    pair.bytes(&canary_digest);
    pair.text("agreement.admitted");
    Ok(FloorCatalogAdmissionPairReceipt {
        digest: pair.finish(),
    })
}

fn verify_units_declarations() -> Result<(), AuditedCatalogError> {
    if SI_REPRESENTATION_SCHEMA_ID != "civsim.units.si-representation.v1"
        || REPRESENTATION_DEFINITIONS.len() != REPRESENTATION_DEFINITION_FINGERPRINTS.len()
    {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "SI representation schema '{}' does not match its sealed seven-definition v1 contract",
            SI_REPRESENTATION_SCHEMA_ID
        )));
    }
    for (candidate, fingerprint) in REPRESENTATION_DEFINITIONS
        .iter()
        .zip(REPRESENTATION_DEFINITION_FINGERPRINTS)
    {
        if candidate.symbol != fingerprint.symbol
            || candidate.value != fingerprint.value
            || candidate.unit != fingerprint.unit
            || candidate.dimension != fingerprint.dimension
            || candidate.role != FundamentalRole::RepresentationDefinition
            || candidate.source_id != fingerprint.source_id
            || candidate.source_sha256 != fingerprint.source_sha256
            || candidate.source_anchor != fingerprint.source_anchor
            || candidate.uncertainty.kind_id() != "exact"
            || candidate.uncertainty.decimal() != "0"
        {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "units declaration for sealed representation definition '{}' does not match its independent ordered fingerprint",
                fingerprint.symbol
            )));
        }
    }

    if COMPOSITES.len() != EXECUTION_RELATION_FINGERPRINTS.len() {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "units declares {} execution relations but the sealed representation contract contains {}",
            COMPOSITES.len(),
            EXECUTION_RELATION_FINGERPRINTS.len()
        )));
    }
    for (candidate, fingerprint) in COMPOSITES.iter().zip(EXECUTION_RELATION_FINGERPRINTS) {
        if candidate.symbol != fingerprint.symbol
            || candidate.formula != fingerprint.formula
            || candidate.fundamentals != fingerprint.inputs
            || candidate.unit != fingerprint.unit
            || candidate.dimension != fingerprint.dimension
        {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "units declaration for sealed execution relation '{}' does not match its independent ordered fingerprint",
                fingerprint.symbol
            )));
        }
    }

    if PHYSICAL_FLOOR_LEN != PHYSICAL_INVARIANT_ADMISSIONS.len()
        || PHYSICAL_INVARIANTS.len() != PHYSICAL_INVARIANT_ADMISSIONS.len()
    {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "units declares {} physical invariant candidates, the public floor length is {}, and the sealed admission registry contains {}",
            PHYSICAL_INVARIANTS.len(),
            PHYSICAL_FLOOR_LEN,
            PHYSICAL_INVARIANT_ADMISSIONS.len()
        )));
    }
    for (candidate, admitted) in PHYSICAL_INVARIANTS
        .iter()
        .zip(PHYSICAL_INVARIANT_ADMISSIONS)
    {
        let uncertainty = candidate.uncertainty;
        if candidate.symbol != admitted.symbol
            || candidate.value != admitted.value
            || candidate.unit != admitted.unit
            || candidate.dimension != admitted.dimension
            || candidate.role != FundamentalRole::PhysicalInvariant
            || candidate.source_id != admitted.source_id
            || candidate.source_sha256 != admitted.source_sha256
            || candidate.source_anchor != admitted.source_anchor
            || uncertainty.kind_id() != admitted.uncertainty_kind
            || uncertainty.decimal() != admitted.uncertainty_decimal
        {
            return Err(AuditedCatalogError::DefinitionMismatch(format!(
                "units declaration for sealed physical invariant '{}' does not match its independent ordered admission fingerprint",
                admitted.symbol
            )));
        }
    }
    source_evidence::verify(&PHYSICAL_INVARIANT_ADMISSIONS)
        .map_err(AuditedCatalogError::DefinitionMismatch)?;
    Ok(())
}

fn common_gap_law(reference_validity: &str, scale_free_limit: &str) -> GapLawReceipt {
    GapLawReceipt {
        reference_validity: reference_validity.to_owned(),
        gap_dispatch: "No authored gap branch is admitted; absence of the invariant causes a refusal rather than a substitute value".into(),
        smooth_systematics: "The invariant is not an interpolated table or smooth fit, so no trend residual can hide in this slot".into(),
        scale_free_limit: scale_free_limit.to_owned(),
        chaos_protocol: ChaosProtocolReceipt::NotApplicable {
            basis: "An invariant coordinate is time-independent and selects no trajectory, attractor, stochastic closure, or sub-resolution branch"
                .into(),
        },
    }
}

fn common_residual_law(dimensional_analysis: &str) -> ResidualLawReceipt {
    ResidualLawReceipt {
        conservation:
            "The invariant parameterizes a law and is not a source or sink of a conserved stock"
                .into(),
        disequilibrium: "The invariant is not a disequilibrium state or a frozen flux".into(),
        fluctuation_dissipation:
            "No stochastic or dissipative term is introduced by this invariant coordinate".into(),
        dimensional_analysis: dimensional_analysis.to_owned(),
    }
}

pub(crate) fn physical_invariant_receipts() -> Vec<DerivationExhaustionReceipt> {
    vec![
        DerivationExhaustionReceipt {
            entry_id: fundamental_id("alpha"),
            phenomenon: "electromagnetic.coupling".into(),
            derivation_attempts: vec![
                "The exact SI definitions c, h, and e fix representation scales but leave the dimensionless electromagnetic coupling undetermined".into(),
                "Vacuum permittivity is the equivalent SI coordinate e^2/(2*alpha*h*c), so admitting eps_0 instead would only rename this residual slot".into(),
                "The gravity-matter scale coordinates G and m_e provide no law that fixes electromagnetic coupling strength".into(),
            ],
            residual_slot: "coupling.fine_structure".into(),
            buckingham_pi_groups: 1,
            gap_law: common_gap_law(
                "The pinned CODATA 2018 fine-structure row supplies value, uncertainty, unit, checksum, and stable source anchor",
                "The alpha-to-zero limit removes electromagnetic interaction and is a distinct physical theory, not a derivation of the observed coupling",
            ),
            residual_law: common_residual_law(
                "Buckingham-Pi leaves exactly one dimensionless electromagnetic coupling group after the SI representation definitions are removed",
            ),
        },
        DerivationExhaustionReceipt {
            entry_id: fundamental_id("G"),
            phenomenon: "gravity_matter.scale_basis".into(),
            derivation_attempts: vec![
                "The exact SI definitions, including Delta_nu_Cs, fix the coordinate system but do not determine G*m_e^2/(h*c)".into(),
                "Electromagnetic alpha supplies an independent dimensionless group and does not close the gravitational coupling".into(),
                "Replacing G with a Planck mass or electron gravitational coupling changes coordinates without reducing the two-coordinate gravity-matter rank".into(),
            ],
            residual_slot: "coupling.newtonian_gravity".into(),
            buckingham_pi_groups: 2,
            gap_law: common_gap_law(
                "The pinned CODATA 2018 Newtonian-gravitation row supplies value, uncertainty, unit, checksum, and stable source anchor",
                "The G-to-zero limit removes gravitation and is a distinct theory, not a derivation of the measured coupling",
            ),
            residual_law: common_residual_law(
                "For G, m_e, h, c, and Delta_nu_Cs, Buckingham-Pi leaves two independent groups: G*m_e^2/(h*c) and m_e*c^2/(h*Delta_nu_Cs); this slot carries the former",
            ),
        },
        DerivationExhaustionReceipt {
            entry_id: fundamental_id("m_e"),
            phenomenon: "gravity_matter.scale_basis".into(),
            derivation_attempts: vec![
                "The exact SI definitions establish the coordinate system but do not determine m_e*c^2/(h*Delta_nu_Cs)".into(),
                "Neither electromagnetic alpha nor Newtonian G fixes the independent matter scale without a further dimensionless mass ratio".into(),
                "Replacing m_e with a Compton wavelength or Planck-mass ratio changes coordinates without reducing the two-coordinate gravity-matter rank".into(),
            ],
            residual_slot: "scale.electron_mass".into(),
            buckingham_pi_groups: 2,
            gap_law: common_gap_law(
                "The pinned CODATA 2018 electron-mass row supplies value, uncertainty, unit, checksum, and stable source anchor",
                "The zero-mass limit changes the particle content and is not a derivation of the observed matter scale",
            ),
            residual_law: common_residual_law(
                "For G, m_e, h, c, and Delta_nu_Cs, Buckingham-Pi leaves two independent groups: G*m_e^2/(h*c) and m_e*c^2/(h*Delta_nu_Cs); this slot carries the latter",
            ),
        },
    ]
}

const RECEIPT_FINGERPRINT_SCHEMA_ID: &str = "civsim.floor.exhaustion-receipt-fingerprint.v2";

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn receipt_fingerprint(receipt: &DerivationExhaustionReceipt) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_field(&mut hasher, RECEIPT_FINGERPRINT_SCHEMA_ID);
    hash_field(&mut hasher, &receipt.entry_id);
    hash_field(&mut hasher, &receipt.phenomenon);
    hasher.update((receipt.derivation_attempts.len() as u64).to_le_bytes());
    for attempt in &receipt.derivation_attempts {
        hash_field(&mut hasher, attempt);
    }
    hash_field(&mut hasher, &receipt.residual_slot);
    hasher.update((receipt.buckingham_pi_groups as u64).to_le_bytes());
    hash_field(&mut hasher, &receipt.gap_law.reference_validity);
    hash_field(&mut hasher, &receipt.gap_law.gap_dispatch);
    hash_field(&mut hasher, &receipt.gap_law.smooth_systematics);
    hash_field(&mut hasher, &receipt.gap_law.scale_free_limit);
    hash_field(&mut hasher, receipt.gap_law.chaos_protocol.kind_id());
    for (field, evidence) in receipt.gap_law.chaos_protocol.evidence() {
        hash_field(&mut hasher, field);
        hash_field(&mut hasher, evidence);
    }
    hash_field(&mut hasher, &receipt.residual_law.conservation);
    hash_field(&mut hasher, &receipt.residual_law.disequilibrium);
    hash_field(&mut hasher, &receipt.residual_law.fluctuation_dissipation);
    hash_field(&mut hasher, &receipt.residual_law.dimensional_analysis);
    hasher.finalize().into()
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn verify_receipt_fingerprints(
    receipts: &[DerivationExhaustionReceipt],
) -> Result<(), AuditedCatalogError> {
    if receipts.len() != RECEIPT_FINGERPRINTS.len() {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "receipt constructor produced {} rows but the independent seal contains {}",
            receipts.len(),
            RECEIPT_FINGERPRINTS.len()
        )));
    }
    let mismatches: Vec<_> = receipts
        .iter()
        .zip(RECEIPT_FINGERPRINTS)
        .filter_map(|(receipt, expected)| {
            let found = receipt_fingerprint(receipt);
            (receipt.entry_id != expected.entry_id || found != expected.sha256).then(|| {
                format!(
                    "{} expected {} found {}",
                    expected.entry_id,
                    hex(&expected.sha256),
                    hex(&found)
                )
            })
        })
        .collect();
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(AuditedCatalogError::DefinitionMismatch(format!(
            "derive-first receipt fingerprint mismatch: {}",
            mismatches.join("; ")
        )))
    }
}

/// Build the exact, ordered provenance catalog admitted at the physical floor.
pub fn audited_substrate_ledger() -> Result<Ledger, AuditedCatalogError> {
    verify_units_declarations()?;
    let invariants = PHYSICAL_INVARIANT_ADMISSIONS
        .into_iter()
        .map(|constant| Entry {
            id: fundamental_id(constant.symbol),
            tier: constant.tier,
            provenance: constant.provenance,
            inputs: Vec::new(),
        });
    Ledger::build(invariants).map_err(AuditedCatalogError::Ledger)
}

/// Verify the sealed authority digest, identity-keyed members, and independently
/// pinned receipt contents before the floor can authorize magnitudes.
pub fn verify_absolute_physics_floor(
    floor: &AbsolutePhysicsFloor,
) -> Result<(), AuditedCatalogError> {
    let _authority = sealed_physical_floor_authority_binding()?;
    let expected = audited_substrate_ledger()?;
    let admitted_entries = floor
        .entries()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let expected_entries = expected
        .entries()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    if let Some(id) = admitted_entries
        .keys()
        .find(|id| !expected_entries.contains_key(*id))
    {
        return Err(AuditedCatalogError::FloorMismatch(format!(
            "unaudited entry '{id}' is present"
        )));
    }
    for (id, required) in expected_entries {
        match admitted_entries.get(id) {
            Some(found) if *found == required => {}
            Some(_) => {
                return Err(AuditedCatalogError::FloorMismatch(format!(
                    "entry '{id}' differs from its sealed definition"
                )));
            }
            None => {
                return Err(AuditedCatalogError::FloorMismatch(format!(
                    "sealed entry '{id}' is absent"
                )));
            }
        }
    }

    for expected in RECEIPT_FINGERPRINTS {
        let receipt = floor.receipt(expected.entry_id).ok_or_else(|| {
            AuditedCatalogError::FloorMismatch(format!(
                "sealed derive-first receipt '{}' is absent",
                expected.entry_id
            ))
        })?;
        let found = receipt_fingerprint(receipt);
        if found != expected.sha256 {
            return Err(AuditedCatalogError::FloorMismatch(format!(
                "derive-first receipt '{}' has fingerprint {}, expected {}",
                expected.entry_id,
                hex(&found),
                hex(&expected.sha256)
            )));
        }
    }
    let receipts = floor
        .entries()
        .filter_map(|entry| floor.receipt(&entry.id).cloned())
        .collect::<Vec<_>>();
    let entries = floor.entries().cloned().collect::<Vec<_>>();
    verify_floor_catalog_admission_pair(&entries, &receipts)?;
    verify_floor_pi_budgets(&receipts).map_err(AuditedCatalogError::Watchdog)?;
    Ok(())
}

/// Construct the sole repository-owned absolute physical floor.
pub fn sealed_absolute_physics_floor() -> Result<AbsolutePhysicsFloor, SealedFloorError> {
    let _authority =
        sealed_physical_floor_authority_binding().map_err(SealedFloorError::Catalog)?;
    let catalog = audited_substrate_ledger().map_err(SealedFloorError::Catalog)?;
    let receipts = physical_invariant_receipts();
    verify_receipt_fingerprints(&receipts).map_err(SealedFloorError::Catalog)?;
    verify_floor_pi_budgets(&receipts).map_err(SealedFloorError::Watchdog)?;
    let floor =
        AbsolutePhysicsFloor::admit(catalog, receipts).map_err(SealedFloorError::Admission)?;
    verify_absolute_physics_floor(&floor).map_err(SealedFloorError::Catalog)?;
    Ok(floor)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditedCatalogError {
    DefinitionMismatch(String),
    FloorMismatch(String),
    Ledger(LedgerError),
    Watchdog(AuthorityWatchdogError),
}

impl fmt::Display for AuditedCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefinitionMismatch(detail) => write!(f, "sealed catalog mismatch: {detail}"),
            Self::FloorMismatch(detail) => write!(f, "absolute-floor seal mismatch: {detail}"),
            Self::Ledger(error) => write!(f, "invalid catalog graph: {error}"),
            Self::Watchdog(error) => write!(f, "independent authority watchdog refused: {error}"),
        }
    }
}

impl std::error::Error for AuditedCatalogError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealedFloorError {
    Catalog(AuditedCatalogError),
    Admission(FloorAdmissionError),
    Watchdog(AuthorityWatchdogError),
}

impl fmt::Display for SealedFloorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "cannot build sealed catalog: {error}"),
            Self::Admission(error) => write!(f, "cannot admit sealed physical floor: {error}"),
            Self::Watchdog(error) => write!(f, "independent authority watchdog refused: {error}"),
        }
    }
}

impl std::error::Error for SealedFloorError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_admission_pair_digest() -> [u8; 32] {
        let ledger = audited_substrate_ledger().unwrap();
        let entries = ledger.entries().cloned().collect::<Vec<_>>();
        verify_floor_catalog_admission_pair(&entries, &physical_invariant_receipts())
            .unwrap()
            .digest()
    }

    fn current_source_evidence_digest() -> [u8; 32] {
        source_evidence::verify(&PHYSICAL_INVARIANT_ADMISSIONS)
            .unwrap()
            .digest()
    }

    #[test]
    fn current_floor_authority_matches_independent_binding_pin() {
        let binding = sealed_physical_floor_authority_binding().unwrap();
        assert_eq!(binding.schema_id(), PhysicalFloorAuthoritySchemaId::V5);
        assert_eq!(binding.digest(), EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_eq!(binding.digest_hex(), hex(&binding.digest()));
    }

    #[test]
    fn public_receipt_fingerprints_are_the_reviewed_floor_pins() {
        let bindings = sealed_physical_floor_receipt_fingerprints().unwrap();
        assert_eq!(
            bindings.map(PhysicalFloorReceiptFingerprint::entry_id),
            ["fundamental.alpha", "fundamental.G", "fundamental.m_e"]
        );
        for (binding, expected) in bindings.into_iter().zip(RECEIPT_FINGERPRINTS) {
            assert_eq!(binding.schema_id(), RECEIPT_FINGERPRINT_SCHEMA_ID);
            assert_eq!(binding.entry_id(), expected.entry_id);
            assert_eq!(binding.digest(), expected.sha256);
            assert_ne!(binding.digest(), [0; 32]);
        }
    }

    #[test]
    fn public_irreducible_admissions_expose_every_typed_route_receipt() {
        let bindings = sealed_physical_floor_irreducible_admissions().unwrap();
        assert_eq!(
            bindings.map(PhysicalFloorIrreducibleAdmissionBinding::entry_id),
            ["fundamental.G", "fundamental.alpha", "fundamental.m_e"]
        );
        for binding in bindings {
            assert_eq!(
                binding.schema_id(),
                PhysicalFloorIrreducibleAdmissionBinding::SCHEMA_ID
            );
            let receipts = [
                binding.derivation_exhaustion_receipt(),
                binding.buckingham_pi_receipt(),
                binding.gap_law_receipt(),
                binding.chaos_protocol_receipt(),
                binding.residual_law_receipt(),
                binding.residual_slot_receipt(),
                binding.owner_admission_receipt(),
                binding.independent_watchdog_receipt(),
            ];
            assert!(receipts.iter().all(|receipt| *receipt != [0; 32]));
            assert_eq!(
                receipts
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                receipts.len()
            );
        }
    }

    #[test]
    fn public_floor_definitions_are_exactly_the_three_measured_invariants() {
        let definitions = sealed_physical_floor_definitions().unwrap();
        assert_eq!(
            definitions.map(|definition| definition.symbol),
            ["alpha", "G", "m_e"]
        );
        assert!(definitions
            .iter()
            .all(|definition| definition.role == FundamentalRole::PhysicalInvariant));
        assert!(definitions.iter().all(|definition| {
            !REPRESENTATION_DEFINITIONS
                .iter()
                .any(|representation| representation.symbol == definition.symbol)
        }));
    }

    #[test]
    fn fixed_math_receipt_checkout_endings_bind_one_floor_digest() {
        let canonical = canonical_repository_text(
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            "fixed-math authority receipt",
        )
        .unwrap()
        .into_owned();
        let mut crlf = Vec::with_capacity(canonical.len() * 2);
        for byte in &canonical {
            if *byte == b'\n' {
                crlf.extend_from_slice(b"\r\n");
            } else {
                crlf.push(*byte);
            }
        }
        let digest_for = |receipt: &[u8]| {
            physical_floor_authority_digest(
                PhysicalFloorAuthoritySchemaId::V5,
                SI_REPRESENTATION_SCHEMA_ID,
                &SI_BASE_DIMENSION_IDS,
                &REPRESENTATION_DEFINITION_FINGERPRINTS,
                &PHYSICAL_INVARIANT_ADMISSIONS,
                RECEIPT_FINGERPRINT_SCHEMA_ID,
                &RECEIPT_FINGERPRINTS,
                [0; 32],
                [0; 32],
                current_source_evidence_digest(),
                receipt,
                &EXECUTION_RELATION_FINGERPRINTS,
            )
        };
        assert_eq!(digest_for(&canonical), digest_for(&crlf));

        let mut bare_carriage_return = canonical;
        bare_carriage_return.push(b'\r');
        assert!(matches!(
            canonical_repository_text(&bare_carriage_return, "fixed-math authority receipt"),
            Err(AuditedCatalogError::DefinitionMismatch(_))
        ));
    }

    #[test]
    fn dimensional_projection_exposes_only_the_three_ordered_floor_coordinates() {
        let columns = sealed_physical_floor_dimension_columns().unwrap();
        assert_eq!(
            columns
                .iter()
                .map(SiDimensionColumn::id)
                .collect::<Vec<_>>(),
            vec!["fundamental.alpha", "fundamental.G", "fundamental.m_e"]
        );
        assert_eq!(columns[0].dimension(), SiDimension::DIMENSIONLESS);
        assert_eq!(
            columns[1].dimension(),
            SiDimension::new(3, -1, -2, 0, 0, 0, 0)
        );
        assert_eq!(
            columns[2].dimension(),
            SiDimension::new(0, 1, 0, 0, 0, 0, 0)
        );
    }

    #[test]
    fn copied_changed_authority_component_changes_digest() {
        let original = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let mut changed_admissions = PHYSICAL_INVARIANT_ADMISSIONS;
        changed_admissions[0].source_anchor = "copied changed source anchor";
        let changed = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &changed_admissions,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );

        assert_ne!(changed, original);
    }

    #[test]
    fn tier_and_provenance_are_bound_into_physical_authority_digest() {
        let baseline = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let mut changed_tier = PHYSICAL_INVARIANT_ADMISSIONS;
        changed_tier[0].tier = Tier::Reference;
        let tier_digest = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &changed_tier,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let mut changed_provenance = PHYSICAL_INVARIANT_ADMISSIONS;
        changed_provenance[0].provenance = Provenance::Estimator;
        let provenance_digest = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &changed_provenance,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );

        assert_ne!(tier_digest, baseline);
        assert_ne!(provenance_digest, baseline);
    }

    #[test]
    fn floor_catalog_admission_pair_is_bound_into_floor_authority() {
        let watchdog = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        let admission_pair = current_admission_pair_digest();
        let bound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let unbound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            [0; 32],
            watchdog.digest(),
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );

        assert_eq!(bound, EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_ne!(bound, unbound);
    }

    #[test]
    fn floor_catalog_admission_pair_exercises_mutation_canaries() {
        let ledger = audited_substrate_ledger().unwrap();
        let entries = ledger.entries().cloned().collect::<Vec<_>>();
        let receipts = physical_invariant_receipts();
        let digest =
            floor_admission_canary_digest(&entries, &receipts, &IRREDUCIBLE_ADMISSION_PINS)
                .unwrap();

        assert_ne!(digest, [0; 32]);
    }

    #[test]
    fn watchdog_rejection_classes_have_distinct_stable_ids() {
        assert_eq!(
            FloorAdmissionWatchdogRejectionClass::MalformedInput.stable_id(),
            "checker.rejected.malformed_input"
        );
        assert_eq!(
            FloorAdmissionWatchdogRejectionClass::SemanticValidation.stable_id(),
            "checker.rejected.semantic_validation"
        );
        assert_eq!(
            FloorAdmissionWatchdogRejectionClass::CanonicalInputCustody.stable_id(),
            "checker.rejected.canonical_input_custody"
        );
    }

    #[test]
    fn semantic_invalid_floor_mutations_are_not_canonical_pin_mismatches() {
        let ledger = audited_substrate_ledger().unwrap();
        let entries = ledger.entries().cloned().collect::<Vec<_>>();
        let receipts = physical_invariant_receipts();

        let mut changed_provenance = entries.clone();
        changed_provenance[0].provenance = Provenance::Estimator;
        assert!(!producer_accepts_floor_claim(
            &changed_provenance,
            &receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        ));
        let provenance_error =
            verify_floor_catalog_admission_bytes(&floor_catalog_admission_bytes(
                &changed_provenance,
                &receipts,
                &IRREDUCIBLE_ADMISSION_PINS,
            ))
            .unwrap_err();
        assert_eq!(
            provenance_error.rejection_class(),
            FloorAdmissionWatchdogRejectionClass::SemanticValidation
        );

        let mut missing_receipt_semantics = receipts.clone();
        missing_receipt_semantics[0].gap_law.reference_validity = "   ".into();
        assert!(!producer_accepts_floor_claim(
            &entries,
            &missing_receipt_semantics,
            &IRREDUCIBLE_ADMISSION_PINS,
        ));
        let receipt_error = verify_floor_catalog_admission_bytes(&floor_catalog_admission_bytes(
            &entries,
            &missing_receipt_semantics,
            &IRREDUCIBLE_ADMISSION_PINS,
        ))
        .unwrap_err();
        assert_eq!(
            receipt_error.rejection_class(),
            FloorAdmissionWatchdogRejectionClass::SemanticValidation
        );
    }

    #[test]
    fn alternate_catalog_claims_need_custody_or_matching_owner_admission() {
        let ledger = audited_substrate_ledger().unwrap();
        let entries = ledger.entries().cloned().collect::<Vec<_>>();
        let receipts = physical_invariant_receipts();

        let mut changed_tier = entries.clone();
        changed_tier[0].tier = Tier::Reference;
        assert!(producer_accepts_floor_claim(
            &changed_tier,
            &receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        ));
        let tier_error = verify_floor_catalog_admission_bytes(&floor_catalog_admission_bytes(
            &changed_tier,
            &receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        ))
        .unwrap_err();
        assert_eq!(
            tier_error.rejection_class(),
            FloorAdmissionWatchdogRejectionClass::CanonicalInputCustody
        );

        let mut changed_member = entries.clone();
        let old_id = changed_member[0].id.clone();
        changed_member[0].id.push_str(".mutated");
        let mut changed_member_receipts = receipts.clone();
        for receipt in &mut changed_member_receipts {
            if receipt.entry_id == old_id {
                receipt.entry_id = changed_member[0].id.clone();
            }
        }
        assert!(!producer_accepts_floor_claim(
            &changed_member,
            &changed_member_receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        ));
        let member_error = verify_floor_catalog_admission_bytes(&floor_catalog_admission_bytes(
            &changed_member,
            &changed_member_receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        ))
        .unwrap_err();
        assert_eq!(
            member_error.rejection_class(),
            FloorAdmissionWatchdogRejectionClass::SemanticValidation
        );

        let mut changed_receipt_authority = receipts.clone();
        changed_receipt_authority[0].derivation_attempts[0]
            .push_str(" copied authority-changing suffix");
        assert!(!producer_accepts_floor_claim(
            &entries,
            &changed_receipt_authority,
            &IRREDUCIBLE_ADMISSION_PINS,
        ));
        let receipt_error = verify_floor_catalog_admission_bytes(&floor_catalog_admission_bytes(
            &entries,
            &changed_receipt_authority,
            &IRREDUCIBLE_ADMISSION_PINS,
        ))
        .unwrap_err();
        assert_eq!(
            receipt_error.rejection_class(),
            FloorAdmissionWatchdogRejectionClass::SemanticValidation
        );
    }

    #[test]
    fn independent_pi_watchdog_is_bound_into_floor_authority() {
        let watchdog = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        let admission_pair = current_admission_pair_digest();
        let bound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let unbound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            [0; 32],
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        assert_eq!(bound, EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_ne!(bound, unbound);
    }

    #[test]
    fn codata_source_evidence_pair_is_bound_into_floor_authority() {
        let watchdog = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        let admission_pair = current_admission_pair_digest();
        let bound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let unbound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            [0; 32],
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        assert_eq!(bound, EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_ne!(bound, unbound);
    }

    #[test]
    fn exact_canonical_fixed_math_receipt_bytes_are_bound_into_floor_authority() {
        let watchdog = verify_floor_pi_budgets(&physical_invariant_receipts()).unwrap();
        let admission_pair = current_admission_pair_digest();
        let bound = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            current_source_evidence_digest(),
            FIXED_MATH_TABLE_AUTHORITY_RECEIPT,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let mut changed_receipt = FIXED_MATH_TABLE_AUTHORITY_RECEIPT.to_vec();
        changed_receipt[0] ^= 1;
        let changed = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V5,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            admission_pair,
            watchdog.digest(),
            current_source_evidence_digest(),
            &changed_receipt,
            &EXECUTION_RELATION_FINGERPRINTS,
        );

        assert_eq!(bound, EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_ne!(bound, changed);
    }

    #[test]
    fn receipt_constructor_matches_independent_pins() {
        verify_receipt_fingerprints(&physical_invariant_receipts()).unwrap();
    }

    #[test]
    fn all_three_declaration_tables_are_order_sealed() {
        verify_units_declarations().unwrap();
        assert_eq!(REPRESENTATION_DEFINITIONS.len(), 7);
        assert_eq!(PHYSICAL_INVARIANTS.len(), 3);
        assert_eq!(COMPOSITES.len(), 4);
    }

    #[test]
    fn sealed_floor_replays_through_independent_verification() {
        let floor = sealed_absolute_physics_floor().unwrap();
        verify_absolute_physics_floor(&floor).unwrap();
        assert_eq!(floor.len(), 3);
    }

    #[test]
    fn floor_arrival_order_is_not_physical_authority() {
        let canonical_entries = audited_substrate_ledger()
            .unwrap()
            .entries()
            .cloned()
            .collect::<Vec<_>>();
        let canonical_receipts = physical_invariant_receipts();
        let canonical_bytes = floor_catalog_admission_bytes(
            &canonical_entries,
            &canonical_receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        );

        let mut permuted_entries = canonical_entries.clone();
        permuted_entries.reverse();
        let mut permuted_receipts = canonical_receipts.clone();
        permuted_receipts.reverse();
        let permuted_bytes = floor_catalog_admission_bytes(
            &permuted_entries,
            &permuted_receipts,
            &IRREDUCIBLE_ADMISSION_PINS,
        );
        assert_eq!(permuted_bytes, canonical_bytes);

        let floor = AbsolutePhysicsFloor::admit(
            Ledger::build(permuted_entries).unwrap(),
            permuted_receipts,
        )
        .unwrap();
        verify_absolute_physics_floor(&floor).unwrap();
    }

    #[test]
    fn changed_receipt_prose_cannot_self_seal() {
        let sealed = sealed_absolute_physics_floor().unwrap();
        let mut receipts: Vec<_> = sealed
            .entries()
            .map(|entry| sealed.receipt(&entry.id).unwrap().clone())
            .collect();
        receipts[0].derivation_attempts = vec!["caller-authored exhaustion claim".into()];
        let candidate = AbsolutePhysicsFloor::admit(audited_substrate_ledger().unwrap(), receipts)
            .expect("generic admission checks structure, not repository authority");
        let error = verify_absolute_physics_floor(&candidate)
            .expect_err("the independent receipt pin must reject changed prose");
        assert!(error.to_string().contains("fundamental.alpha"));
        assert!(error.to_string().contains("fingerprint"));
    }

    #[test]
    fn changed_chaos_protocol_branch_cannot_self_seal() {
        let sealed = sealed_absolute_physics_floor().unwrap();
        let mut receipts: Vec<_> = sealed
            .entries()
            .map(|entry| sealed.receipt(&entry.id).unwrap().clone())
            .collect();
        receipts[0].gap_law.chaos_protocol = ChaosProtocolReceipt::Dynamical {
            classification: "caller reclassified an invariant as dynamical".into(),
            regime_partition: "unsupported partition claim".into(),
            transition_law: "unsupported transition claim".into(),
            regimes: vec![ChaosRegimeReceipt::ResolvedTrajectory {
                validity_domain: "unsupported domain claim".into(),
                resolution_bound: "unsupported resolution claim".into(),
                evolution_postcondition: "unsupported postcondition".into(),
                exact_replay: "unsupported replay claim".into(),
            }],
        };
        let candidate = AbsolutePhysicsFloor::admit(audited_substrate_ledger().unwrap(), receipts)
            .expect("generic admission checks structure, not repository authority");
        let error = verify_absolute_physics_floor(&candidate)
            .expect_err("the independent receipt pin must reject a changed chaos branch");
        assert!(error.to_string().contains("fundamental.alpha"));
        assert!(error.to_string().contains("fingerprint"));
    }
}
