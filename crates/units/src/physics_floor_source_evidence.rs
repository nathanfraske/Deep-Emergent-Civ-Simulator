//! Runtime binding from the measured floor declarations to audited source-row facts.
//!
//! The repository does not redistribute the NIST table. Two Python
//! implementations independently parse explicit live and archived audits.
//! Their checked agreement receipt and the factual row spans are embedded here
//! so the Rust floor seal cannot drift to a second copied table.

use super::PhysicalInvariantAdmission;
use sha2::{Digest, Sha256};
use std::borrow::Cow;

pub(super) const FACTS_SCHEMA_ID: &str = "civsim.units.codata-2018-floor-facts.v1";
pub(super) const EVIDENCE_SCHEMA_ID: &str = "civsim.units.codata-2018-floor-evidence-binding.v2";

const SOURCE_ID: &str = "nist_codata_2018_ascii";
const SOURCE_SHA256: &str = "8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1";
const SOURCE_BYTES: &str = "40689";
const FACTS_SHA256: [u8; 32] = [
    0xc0, 0x83, 0x9f, 0x9c, 0x34, 0x97, 0x72, 0x4f, 0x04, 0x52, 0x2b, 0x70, 0x69, 0x9c, 0x96, 0x9d,
    0x22, 0x5d, 0xb6, 0x2b, 0xaa, 0xd5, 0x50, 0x76, 0x38, 0x16, 0x2f, 0x27, 0x42, 0x59, 0xda, 0x11,
];
const RECEIPT_SHA256: [u8; 32] = [
    0x08, 0xc6, 0x7f, 0x6d, 0x6e, 0x45, 0x43, 0xe7, 0xcb, 0x35, 0xbe, 0x38, 0x3f, 0x08, 0xaa, 0x1b,
    0xbe, 0x73, 0x61, 0x87, 0x8f, 0xf7, 0x8b, 0xbc, 0x85, 0x67, 0xd0, 0x81, 0x04, 0xf9, 0x79, 0x83,
];

pub(super) const FACTS_BYTES: &[u8] = include_bytes!("../data/codata_2018_floor_facts.tsv");
pub(super) const RECEIPT_BYTES: &[u8] =
    include_bytes!("../data/codata_2018_floor_evidence_receipt.json");

const COLUMNS: [&str; 10] = [
    "symbol",
    "source_line",
    "byte_offset",
    "byte_length",
    "row_sha256",
    "source_anchor",
    "normalized_value",
    "absolute_uncertainty",
    "source_unit",
    "si_dimension",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SourceEvidenceBinding {
    digest: [u8; 32],
}

impl SourceEvidenceBinding {
    pub(super) const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FactualRow<'a> {
    symbol: &'a str,
    source_line: &'a str,
    byte_offset: &'a str,
    byte_length: &'a str,
    row_sha256: &'a str,
    source_anchor: &'a str,
    normalized_value: &'a str,
    absolute_uncertainty: &'a str,
    source_unit: &'a str,
    si_dimension: &'a str,
}

fn canonical_repository_text<'a>(raw: &'a [u8], label: &str) -> Result<Cow<'a, [u8]>, String> {
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
            return Err(format!("{label} contains a bare carriage return"));
        }
        canonical.push(b'\n');
        cursor += 2;
    }
    Ok(Cow::Owned(canonical))
}

fn sha256(raw: &[u8]) -> [u8; 32] {
    Sha256::digest(raw).into()
}

fn parse_header(line: &str, key: &str, value: &str) -> Result<(), String> {
    let mut fields = line.split('\t');
    if fields.next() != Some(key) || fields.next() != Some(value) || fields.next().is_some() {
        return Err(format!(
            "CODATA floor facts header '{key}' differs from its reviewed value"
        ));
    }
    Ok(())
}

fn parse_row(line: &str) -> Result<FactualRow<'_>, String> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() != COLUMNS.len() + 1 || fields[0] != "row" {
        return Err("CODATA floor facts row has the wrong arity".to_owned());
    }
    Ok(FactualRow {
        symbol: fields[1],
        source_line: fields[2],
        byte_offset: fields[3],
        byte_length: fields[4],
        row_sha256: fields[5],
        source_anchor: fields[6],
        normalized_value: fields[7],
        absolute_uncertainty: fields[8],
        source_unit: fields[9],
        si_dimension: fields[10],
    })
}

fn dimension_text(admission: &PhysicalInvariantAdmission) -> String {
    admission
        .dimension
        .exponents()
        .iter()
        .map(i8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn verify_facts<'a>(
    raw: &'a [u8],
    admissions: &[PhysicalInvariantAdmission],
) -> Result<Vec<FactualRow<'a>>, String> {
    let text =
        std::str::from_utf8(raw).map_err(|_| "CODATA floor facts are not UTF-8".to_owned())?;
    let lines = text.lines().collect::<Vec<_>>();
    if lines.len() != 6 + admissions.len() {
        return Err(format!(
            "CODATA floor facts contain {} lines, expected {}",
            lines.len(),
            6 + admissions.len()
        ));
    }
    parse_header(lines[0], "schema", FACTS_SCHEMA_ID)?;
    parse_header(lines[1], "source_id", SOURCE_ID)?;
    parse_header(lines[2], "source_sha256", SOURCE_SHA256)?;
    parse_header(lines[3], "source_bytes", SOURCE_BYTES)?;
    parse_header(lines[4], "source_line_endings", "LF")?;
    let mut columns = lines[5].split('\t');
    if columns.next() != Some("columns") || !columns.eq(COLUMNS) {
        return Err("CODATA floor facts columns differ from the closed schema".to_owned());
    }

    let rows = lines[6..]
        .iter()
        .map(|line| parse_row(line))
        .collect::<Result<Vec<_>, _>>()?;
    for (row, admission) in rows.iter().zip(admissions) {
        if row.symbol != admission.symbol
            || row.source_anchor != admission.source_anchor
            || row.normalized_value != admission.value
            || row.absolute_uncertainty != admission.uncertainty_decimal
            || admission.source_id != SOURCE_ID
            || admission.source_sha256 != SOURCE_SHA256
            || row.si_dimension != dimension_text(admission)
        {
            return Err(format!(
                "measured declaration '{}' differs from its audited CODATA row fact",
                admission.symbol
            ));
        }
        if row
            .source_line
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .is_none()
            || row.byte_offset.parse::<usize>().is_err()
            || row
                .byte_length
                .parse::<usize>()
                .ok()
                .filter(|value| *value > 0)
                .is_none()
            || row.row_sha256.len() != 64
            || !row.row_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || row.source_unit.trim().is_empty()
        {
            return Err(format!(
                "audited CODATA row metadata for '{}' is malformed",
                admission.symbol
            ));
        }
    }
    Ok(rows)
}

fn require_receipt_anchor(receipt: &str, anchor: &str) -> Result<(), String> {
    if receipt.matches(anchor).count() != 1 {
        return Err(format!(
            "CODATA floor evidence receipt must contain one {anchor:?}"
        ));
    }
    Ok(())
}

pub(super) fn verify(
    admissions: &[PhysicalInvariantAdmission],
) -> Result<SourceEvidenceBinding, String> {
    let facts = canonical_repository_text(FACTS_BYTES, "CODATA floor facts")?;
    let receipt = canonical_repository_text(RECEIPT_BYTES, "CODATA floor evidence receipt")?;
    if sha256(facts.as_ref()) != FACTS_SHA256 {
        return Err("CODATA floor facts differ from the reviewed byte pin".to_owned());
    }
    if sha256(receipt.as_ref()) != RECEIPT_SHA256 {
        return Err("CODATA floor evidence receipt differs from the reviewed byte pin".to_owned());
    }
    verify_facts(facts.as_ref(), admissions)?;
    let receipt_text = std::str::from_utf8(receipt.as_ref())
        .map_err(|_| "CODATA floor evidence receipt is not UTF-8".to_owned())?;
    for anchor in [
        "\"schema\": \"civsim.units.codata-2018-floor-evidence-pair.v2\"",
        "\"claim_id\": \"floor.codata-factual-row-binding\"",
        "\"implementation\": \"civsim.units.codata-fixed-column-producer.v2\"",
        "\"implementation\": \"civsim.units.codata-record-scan-watchdog.v2\"",
        "\"sha256\": \"c0839f9c3497724f04522b70699c969d225db62baad5507638162f274259da11\"",
        "\"live_archive_equal\": true",
    ] {
        require_receipt_anchor(receipt_text, anchor)?;
    }

    let mut digest = Sha256::new();
    digest.update((EVIDENCE_SCHEMA_ID.len() as u64).to_le_bytes());
    digest.update(EVIDENCE_SCHEMA_ID.as_bytes());
    digest.update((facts.len() as u64).to_le_bytes());
    digest.update(facts.as_ref());
    digest.update((receipt.len() as u64).to_le_bytes());
    digest.update(receipt.as_ref());
    Ok(SourceEvidenceBinding {
        digest: digest.finalize().into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics_floor::PHYSICAL_INVARIANT_ADMISSIONS;

    #[test]
    fn checked_source_evidence_binds_every_measured_declaration() {
        let binding = verify(&PHYSICAL_INVARIANT_ADMISSIONS).unwrap();
        assert_ne!(binding.digest(), [0; 32]);
    }

    #[test]
    fn copied_changed_floor_value_cannot_match_the_row_facts() {
        let mut changed = PHYSICAL_INVARIANT_ADMISSIONS;
        changed[0].value = "7.2973525694e-3";
        let facts = canonical_repository_text(FACTS_BYTES, "facts").unwrap();
        let error = verify_facts(facts.as_ref(), &changed).unwrap_err();
        assert!(error.contains("alpha"));
    }

    #[test]
    fn bare_carriage_return_is_not_a_checkout_equivalent() {
        let mut changed = FACTS_BYTES.to_vec();
        changed.push(b'\r');
        assert!(canonical_repository_text(&changed, "facts").is_err());
    }
}
