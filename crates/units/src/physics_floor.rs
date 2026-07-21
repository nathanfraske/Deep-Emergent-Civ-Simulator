//! Independent admission authority for the absolute physical floor.
//!
//! Source custody and the seven provenance marks account for a candidate, but
//! neither authorizes it. This module is the lower-layer seal shared by the
//! canonical runner and every causal physics consumer. It checks the ordered
//! constant declarations against an independent registry, pins the complete
//! derive-first receipts by digest, and admits only that exact floor.

use crate::fundamentals::{
    FundamentalRole, SiDimension, COMPOSITES, PHYSICAL_INVARIANTS, REPRESENTATION_DEFINITIONS,
    SI_REPRESENTATION_SCHEMA_ID,
};
use civsim_ledger::{
    AbsolutePhysicsFloor, DerivationExhaustionReceipt, Entry, FloorAdmissionError, GapLawReceipt,
    Ledger, LedgerError, Provenance, ResidualLawReceipt, Tier,
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
// cover every attempt, phenomenon, Pi budget, residual slot, and Gap/Residual
// field using the length-prefixed v1 encoding below.
const RECEIPT_FINGERPRINTS: [ReceiptFingerprint; 3] = [
    ReceiptFingerprint {
        entry_id: "fundamental.alpha",
        sha256: [
            0xff, 0xa5, 0x31, 0x0c, 0x7c, 0xff, 0x5d, 0x98, 0x3b, 0xde, 0x5a, 0x33, 0x24, 0xa5,
            0x19, 0xc5, 0x60, 0xdd, 0x25, 0xdc, 0xf3, 0x02, 0xaf, 0x81, 0x16, 0x60, 0x38, 0xd3,
            0x2b, 0x27, 0x8a, 0xe0,
        ],
    },
    ReceiptFingerprint {
        entry_id: "fundamental.G",
        sha256: [
            0xd5, 0x6c, 0xee, 0xad, 0x01, 0x6b, 0x84, 0x43, 0xcd, 0x5b, 0xf7, 0xab, 0xa0, 0x4f,
            0x91, 0xb9, 0x7c, 0x02, 0x72, 0x34, 0xb0, 0x58, 0xf7, 0x88, 0xbc, 0x9d, 0x17, 0x38,
            0x2b, 0x9b, 0x4e, 0xa1,
        ],
    },
    ReceiptFingerprint {
        entry_id: "fundamental.m_e",
        sha256: [
            0x7e, 0xc7, 0x48, 0x2a, 0xb4, 0xc3, 0xba, 0x61, 0x71, 0x46, 0xfc, 0x6c, 0x74, 0x90,
            0x7c, 0x64, 0x02, 0x90, 0x46, 0x37, 0x3b, 0x67, 0x15, 0x67, 0x18, 0xde, 0x1d, 0xdc,
            0x2c, 0x62, 0x2d, 0x21,
        ],
    },
];

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

    if PHYSICAL_INVARIANTS.len() != PHYSICAL_INVARIANT_ADMISSIONS.len() {
        return Err(AuditedCatalogError::DefinitionMismatch(format!(
            "units declares {} physical invariant candidates but the sealed admission registry contains {}",
            PHYSICAL_INVARIANTS.len(),
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

const RECEIPT_FINGERPRINT_SCHEMA_ID: &str = "civsim.floor.exhaustion-receipt-fingerprint.v1";

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
}
