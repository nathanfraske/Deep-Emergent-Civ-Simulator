//! Independent admission authority for the absolute physical floor.
//!
//! Source custody and the seven provenance marks account for a candidate, but
//! neither authorizes it. This module is the lower-layer seal shared by the
//! canonical runner and every causal physics consumer. It checks the ordered
//! constant declarations against an independent registry, pins the complete
//! derive-first receipts by digest, and admits only that exact floor.

use crate::dimensional_analysis::SiDimensionColumn;
use crate::fundamentals::{
    FundamentalRole, SiDimension, COMPOSITES, PHYSICAL_INVARIANTS, REPRESENTATION_DEFINITIONS,
    SI_BASE_DIMENSION_IDS, SI_REPRESENTATION_SCHEMA_ID,
};
#[cfg(test)]
use civsim_ledger::ChaosRegimeReceipt;
use civsim_ledger::{
    AbsolutePhysicsFloor, ChaosProtocolReceipt, DerivationExhaustionReceipt, Entry,
    FloorAdmissionError, GapLawReceipt, Ledger, LedgerError, Provenance, ResidualLawReceipt, Tier,
};
use sha2::{Digest, Sha256};
use std::fmt;

fn fundamental_id(symbol: &str) -> String {
    format!("fundamental.{symbol}")
}

/// Number of independently admitted physical coordinates in the v1 floor.
pub const PHYSICAL_FLOOR_LEN: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalInvariantAdmission {
    symbol: &'static str,
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

/// Typed identity of the byte encoding used to bind the complete physical-floor
/// authority. A different field set, field order, or encoding requires a new
/// identity rather than silently changing the v1 digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFloorAuthoritySchemaId(&'static str);

impl PhysicalFloorAuthoritySchemaId {
    pub const V1: Self = Self("civsim.units.physical-floor-authority-binding.v1");

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
    /// the sole repository-owned v1 authority binding.
    pub fn sealed() -> Result<Self, AuditedCatalogError> {
        verify_units_declarations()?;
        verify_receipt_fingerprints(&physical_invariant_receipts())?;

        let schema_id = PhysicalFloorAuthoritySchemaId::V1;
        let digest = physical_floor_authority_digest(
            schema_id,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
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
    0x0a, 0x64, 0xc0, 0x51, 0x36, 0x83, 0xf0, 0x44, 0x61, 0xb1, 0x1d, 0x3b, 0x6d, 0xf9, 0xf1, 0x8a,
    0x4d, 0x28, 0x38, 0x25, 0x30, 0x0b, 0xae, 0x71, 0x3b, 0x9a, 0x26, 0xd2, 0x80, 0xc1, 0x33, 0x67,
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

#[allow(clippy::too_many_arguments)]
fn physical_floor_authority_digest(
    schema_id: PhysicalFloorAuthoritySchemaId,
    representation_schema_id: &str,
    base_dimension_ids: &[&str],
    representation_fingerprints: &[RepresentationDefinitionFingerprint],
    physical_admissions: &[PhysicalInvariantAdmission],
    receipt_fingerprint_schema_id: &str,
    receipt_fingerprints: &[ReceiptFingerprint],
    execution_relation_fingerprints: &[ExecutionRelationFingerprint],
) -> [u8; 32] {
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

fn physical_invariant_receipts() -> Vec<DerivationExhaustionReceipt> {
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
            tier: Tier::Universal,
            provenance: Provenance::Measured,
            inputs: Vec::new(),
        });
    Ledger::build(invariants).map_err(AuditedCatalogError::Ledger)
}

/// Verify both ordered identities and independently pinned receipt contents.
pub fn verify_absolute_physics_floor(
    floor: &AbsolutePhysicsFloor,
) -> Result<(), AuditedCatalogError> {
    let expected = audited_substrate_ledger()?;
    let mut admitted_entries = floor.entries();
    let mut expected_entries = expected.entries();
    let mut index = 0_usize;
    loop {
        match (admitted_entries.next(), expected_entries.next()) {
            (Some(found), Some(required)) if found == required => {}
            (Some(found), Some(required)) => {
                return Err(AuditedCatalogError::FloorMismatch(format!(
                    "entry {index} '{}' does not match sealed entry '{}'",
                    found.id, required.id
                )));
            }
            (Some(found), None) => {
                return Err(AuditedCatalogError::FloorMismatch(format!(
                    "unaudited entry '{}' appears at position {index}",
                    found.id
                )));
            }
            (None, Some(required)) => {
                return Err(AuditedCatalogError::FloorMismatch(format!(
                    "sealed entry '{}' is absent at position {index}",
                    required.id
                )));
            }
            (None, None) => break,
        }
        index += 1;
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
    Ok(())
}

/// Construct the sole repository-owned absolute physical floor.
pub fn sealed_absolute_physics_floor() -> Result<AbsolutePhysicsFloor, SealedFloorError> {
    let catalog = audited_substrate_ledger().map_err(SealedFloorError::Catalog)?;
    let receipts = physical_invariant_receipts();
    verify_receipt_fingerprints(&receipts).map_err(SealedFloorError::Catalog)?;
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
}

impl fmt::Display for AuditedCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DefinitionMismatch(detail) => write!(f, "sealed catalog mismatch: {detail}"),
            Self::FloorMismatch(detail) => write!(f, "absolute-floor seal mismatch: {detail}"),
            Self::Ledger(error) => write!(f, "invalid catalog graph: {error}"),
        }
    }
}

impl std::error::Error for AuditedCatalogError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealedFloorError {
    Catalog(AuditedCatalogError),
    Admission(FloorAdmissionError),
}

impl fmt::Display for SealedFloorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Catalog(error) => write!(f, "cannot build sealed catalog: {error}"),
            Self::Admission(error) => write!(f, "cannot admit sealed physical floor: {error}"),
        }
    }
}

impl std::error::Error for SealedFloorError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_floor_authority_matches_independent_binding_pin() {
        let binding = sealed_physical_floor_authority_binding().unwrap();
        assert_eq!(binding.schema_id(), PhysicalFloorAuthoritySchemaId::V1);
        assert_eq!(binding.digest(), EXPECTED_PHYSICAL_FLOOR_AUTHORITY_DIGEST);
        assert_eq!(binding.digest_hex(), hex(&binding.digest()));
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
            PhysicalFloorAuthoritySchemaId::V1,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &PHYSICAL_INVARIANT_ADMISSIONS,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            &EXECUTION_RELATION_FINGERPRINTS,
        );
        let mut changed_admissions = PHYSICAL_INVARIANT_ADMISSIONS;
        changed_admissions[0].source_anchor = "copied changed source anchor";
        let changed = physical_floor_authority_digest(
            PhysicalFloorAuthoritySchemaId::V1,
            SI_REPRESENTATION_SCHEMA_ID,
            &SI_BASE_DIMENSION_IDS,
            &REPRESENTATION_DEFINITION_FINGERPRINTS,
            &changed_admissions,
            RECEIPT_FINGERPRINT_SCHEMA_ID,
            &RECEIPT_FINGERPRINTS,
            &EXECUTION_RELATION_FINGERPRINTS,
        );

        assert_ne!(changed, original);
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
