//! Independent parser and semantic checker for the absolute-floor admission claim.
//!
//! The producer supplies only canonical bytes. This module deliberately does
//! not call `Ledger::build`, `AbsolutePhysicsFloor::admit`, the producer's
//! receipt helpers, or the repository admission tables.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub(crate) const CHECKER_IMPLEMENTATION_ID: &str =
    "civsim.units.floor-catalog-admission-watchdog.v1";

const INPUT_SCHEMA_ID: &str = "civsim.units.floor-catalog-admission-input.v1";
const MAX_COLLECTION_ITEMS: usize = 4_096;
const MAX_TEXT_BYTES: usize = 1 << 20;

// This is the independent, owner-reviewed pin for the complete ordered catalog
// and exhaustion-receipt input bytes. It is intentionally separate from the
// producer declarations and the physical authority digest.
const EXPECTED_INPUT_SHA256: [u8; 32] = [
    0x95, 0xc4, 0x8e, 0xf3, 0x65, 0x75, 0xa5, 0x8e, 0x60, 0xb3, 0x25, 0x88, 0xda, 0xab, 0xf3, 0x47,
    0xd8, 0xcd, 0xd2, 0x77, 0x72, 0x81, 0x2d, 0x98, 0x3a, 0x99, 0xc1, 0x30, 0x4a, 0x66, 0x3c, 0xf4,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FloorAdmissionWatchdogReceipt {
    input_digest: [u8; 32],
    result_digest: [u8; 32],
    entry_count: usize,
    receipt_count: usize,
}

impl FloorAdmissionWatchdogReceipt {
    pub(crate) const fn input_digest(&self) -> [u8; 32] {
        self.input_digest
    }

    pub(crate) const fn result_digest(&self) -> [u8; 32] {
        self.result_digest
    }

    pub(crate) const fn entry_count(&self) -> usize {
        self.entry_count
    }

    pub(crate) const fn receipt_count(&self) -> usize {
        self.receipt_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FloorAdmissionWatchdogError {
    Malformed(String),
    Refused(String),
    CanonicalInputMismatch { expected: [u8; 32], found: [u8; 32] },
}

impl fmt::Display for FloorAdmissionWatchdogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "malformed floor-admission bytes: {detail}"),
            Self::Refused(detail) => write!(f, "floor-admission watchdog refused: {detail}"),
            Self::CanonicalInputMismatch { expected, found } => write!(
                f,
                "floor-admission input digest expected {} but found {}",
                hex(expected),
                hex(found)
            ),
        }
    }
}

impl std::error::Error for FloorAdmissionWatchdogError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckedTier {
    Universal,
    Reference,
    Residue,
    Contingency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckedProvenance {
    Derived,
    Measured,
    Estimator,
    Closure,
    Authored,
    WrittenState,
    Contingency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckedEntry {
    id: String,
    tier: CheckedTier,
    provenance: CheckedProvenance,
    inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckedReceipt {
    entry_id: String,
    phenomenon: String,
    residual_slot: String,
    buckingham_pi_groups: usize,
}

struct ByteCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn u64(&mut self, field: &str) -> Result<u64, FloorAdmissionWatchdogError> {
        let end = self
            .offset
            .checked_add(8)
            .ok_or_else(|| FloorAdmissionWatchdogError::Malformed(field.to_owned()))?;
        let raw = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| FloorAdmissionWatchdogError::Malformed(format!("truncated {field}")))?;
        self.offset = end;
        let mut fixed = [0_u8; 8];
        fixed.copy_from_slice(raw);
        Ok(u64::from_le_bytes(fixed))
    }

    fn count(&mut self, field: &str) -> Result<usize, FloorAdmissionWatchdogError> {
        let value = usize::try_from(self.u64(field)?).map_err(|_| {
            FloorAdmissionWatchdogError::Malformed(format!("{field} does not fit usize"))
        })?;
        if value > MAX_COLLECTION_ITEMS {
            return Err(FloorAdmissionWatchdogError::Malformed(format!(
                "{field} exceeds the bounded parser limit"
            )));
        }
        Ok(value)
    }

    fn text(&mut self, field: &str) -> Result<String, FloorAdmissionWatchdogError> {
        let len = usize::try_from(self.u64(&format!("{field}.length"))?).map_err(|_| {
            FloorAdmissionWatchdogError::Malformed(format!("{field} length does not fit usize"))
        })?;
        if len > MAX_TEXT_BYTES {
            return Err(FloorAdmissionWatchdogError::Malformed(format!(
                "{field} exceeds the bounded parser limit"
            )));
        }
        let end = self.offset.checked_add(len).ok_or_else(|| {
            FloorAdmissionWatchdogError::Malformed(format!("{field} length overflow"))
        })?;
        let raw = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| FloorAdmissionWatchdogError::Malformed(format!("truncated {field}")))?;
        self.offset = end;
        std::str::from_utf8(raw)
            .map(str::to_owned)
            .map_err(|_| FloorAdmissionWatchdogError::Malformed(format!("{field} is not UTF-8")))
    }

    fn required_text(&mut self, field: &str) -> Result<String, FloorAdmissionWatchdogError> {
        let value = self.text(field)?;
        if value.trim().is_empty() {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "{field} is empty"
            )));
        }
        Ok(value)
    }

    fn finish(self) -> Result<(), FloorAdmissionWatchdogError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(FloorAdmissionWatchdogError::Malformed(format!(
                "{} trailing bytes remain",
                self.bytes.len() - self.offset
            )))
        }
    }
}

fn parse_tier(value: &str) -> Result<CheckedTier, FloorAdmissionWatchdogError> {
    match value {
        "universal" => Ok(CheckedTier::Universal),
        "reference" => Ok(CheckedTier::Reference),
        "residue" => Ok(CheckedTier::Residue),
        "contingency" => Ok(CheckedTier::Contingency),
        _ => Err(FloorAdmissionWatchdogError::Refused(format!(
            "unknown tier '{value}'"
        ))),
    }
}

fn parse_provenance(value: &str) -> Result<CheckedProvenance, FloorAdmissionWatchdogError> {
    match value {
        "derived" => Ok(CheckedProvenance::Derived),
        "measured" => Ok(CheckedProvenance::Measured),
        "estimator" => Ok(CheckedProvenance::Estimator),
        "closure" => Ok(CheckedProvenance::Closure),
        "authored" => Ok(CheckedProvenance::Authored),
        "written_state" => Ok(CheckedProvenance::WrittenState),
        "contingency" => Ok(CheckedProvenance::Contingency),
        _ => Err(FloorAdmissionWatchdogError::Refused(format!(
            "unknown provenance '{value}'"
        ))),
    }
}

fn parse_entries(
    cursor: &mut ByteCursor<'_>,
) -> Result<Vec<CheckedEntry>, FloorAdmissionWatchdogError> {
    let count = cursor.count("entry_count")?;
    if count == 0 {
        return Err(FloorAdmissionWatchdogError::Refused(
            "the absolute floor is empty".into(),
        ));
    }
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let id = cursor.required_text(&format!("entry[{index}].id"))?;
        let tier = parse_tier(&cursor.required_text(&format!("entry[{index}].tier"))?)?;
        let provenance =
            parse_provenance(&cursor.required_text(&format!("entry[{index}].provenance"))?)?;
        let input_count = cursor.count(&format!("entry[{index}].input_count"))?;
        let mut inputs = Vec::with_capacity(input_count);
        for input_index in 0..input_count {
            inputs.push(cursor.required_text(&format!("entry[{index}].input[{input_index}]"))?);
        }
        entries.push(CheckedEntry {
            id,
            tier,
            provenance,
            inputs,
        });
    }
    Ok(entries)
}

fn read_required_fields(
    cursor: &mut ByteCursor<'_>,
    prefix: &str,
    fields: &[&str],
) -> Result<(), FloorAdmissionWatchdogError> {
    for field in fields {
        cursor.required_text(&format!("{prefix}.{field}"))?;
    }
    Ok(())
}

fn parse_chaos_protocol(
    cursor: &mut ByteCursor<'_>,
    index: usize,
) -> Result<(), FloorAdmissionWatchdogError> {
    let prefix = format!("receipt[{index}].chaos");
    let branch = cursor.required_text(&format!("{prefix}.branch"))?;
    match branch.as_str() {
        "not_applicable" => {
            cursor.required_text(&format!("{prefix}.basis"))?;
        }
        "dynamical" => {
            read_required_fields(
                cursor,
                &prefix,
                &["classification", "regime_partition", "transition_law"],
            )?;
            let regime_count = cursor.count(&format!("{prefix}.regime_count"))?;
            if regime_count == 0 {
                return Err(FloorAdmissionWatchdogError::Refused(format!(
                    "{prefix} has no regimes"
                )));
            }
            for regime_index in 0..regime_count {
                let regime_prefix = format!("{prefix}.regime[{regime_index}]");
                let kind = cursor.required_text(&format!("{regime_prefix}.kind"))?;
                match kind.as_str() {
                    "resolved_trajectory" => read_required_fields(
                        cursor,
                        &regime_prefix,
                        &[
                            "validity_domain",
                            "resolution_bound",
                            "evolution_postcondition",
                            "exact_replay",
                        ],
                    )?,
                    "subresolution_measure" => read_required_fields(
                        cursor,
                        &regime_prefix,
                        &[
                            "validity_domain",
                            "stationary_measure",
                            "conservation_projection",
                            "stability_postcondition",
                            "coordinate_discipline",
                            "exact_replay",
                        ],
                    )?,
                    _ => {
                        return Err(FloorAdmissionWatchdogError::Refused(format!(
                            "{regime_prefix} has unknown kind '{kind}'"
                        )));
                    }
                }
            }
        }
        _ => {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "{prefix} has unknown branch '{branch}'"
            )));
        }
    }
    Ok(())
}

fn parse_receipts(
    cursor: &mut ByteCursor<'_>,
) -> Result<Vec<CheckedReceipt>, FloorAdmissionWatchdogError> {
    let count = cursor.count("receipt_count")?;
    let mut receipts = Vec::with_capacity(count);
    for index in 0..count {
        let prefix = format!("receipt[{index}]");
        let entry_id = cursor.required_text(&format!("{prefix}.entry_id"))?;
        let phenomenon = cursor.required_text(&format!("{prefix}.phenomenon"))?;
        let attempt_count = cursor.count(&format!("{prefix}.attempt_count"))?;
        if attempt_count == 0 {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "{prefix} has no derivation attempts"
            )));
        }
        for attempt_index in 0..attempt_count {
            cursor.required_text(&format!("{prefix}.attempt[{attempt_index}]"))?;
        }
        let residual_slot = cursor.required_text(&format!("{prefix}.residual_slot"))?;
        let buckingham_pi_groups = usize::try_from(
            cursor.u64(&format!("{prefix}.buckingham_pi_groups"))?,
        )
        .map_err(|_| {
            FloorAdmissionWatchdogError::Malformed(format!(
                "{prefix}.buckingham_pi_groups does not fit usize"
            ))
        })?;
        read_required_fields(
            cursor,
            &prefix,
            &[
                "gap.reference_validity",
                "gap.gap_dispatch",
                "gap.smooth_systematics",
                "gap.scale_free_limit",
            ],
        )?;
        parse_chaos_protocol(cursor, index)?;
        read_required_fields(
            cursor,
            &prefix,
            &[
                "residual.conservation",
                "residual.disequilibrium",
                "residual.fluctuation_dissipation",
                "residual.dimensional_analysis",
            ],
        )?;
        receipts.push(CheckedReceipt {
            entry_id,
            phenomenon,
            residual_slot,
            buckingham_pi_groups,
        });
    }
    Ok(receipts)
}

fn validate_entries(entries: &[CheckedEntry]) -> Result<(), FloorAdmissionWatchdogError> {
    let mut ids = BTreeSet::new();
    for entry in entries {
        if !ids.insert(entry.id.as_str()) {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "duplicate catalog member '{}'",
                entry.id
            )));
        }
        if entry.tier == CheckedTier::Contingency
            || entry.provenance == CheckedProvenance::Contingency
        {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "catalog member '{}' is caller-supplied contingency",
                entry.id
            )));
        }
        if matches!(
            entry.provenance,
            CheckedProvenance::Closure
                | CheckedProvenance::Authored
                | CheckedProvenance::WrittenState
        ) {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "catalog member '{}' has forbidden initial provenance",
                entry.id
            )));
        }
        match entry.provenance {
            CheckedProvenance::Derived if entry.inputs.is_empty() => {
                return Err(FloorAdmissionWatchdogError::Refused(format!(
                    "derived catalog member '{}' has no inputs",
                    entry.id
                )));
            }
            CheckedProvenance::Derived => {}
            _ if !entry.inputs.is_empty() => {
                return Err(FloorAdmissionWatchdogError::Refused(format!(
                    "non-derived catalog member '{}' names inputs",
                    entry.id
                )));
            }
            _ => {}
        }
        if entry.tier == CheckedTier::Universal && entry.provenance != CheckedProvenance::Measured {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "universal catalog member '{}' is not measured",
                entry.id
            )));
        }
    }
    for entry in entries {
        for input in &entry.inputs {
            if !ids.contains(input.as_str()) {
                return Err(FloorAdmissionWatchdogError::Refused(format!(
                    "catalog member '{}' names absent input '{input}'",
                    entry.id
                )));
            }
        }
    }
    validate_acyclic(entries)
}

fn validate_acyclic(entries: &[CheckedEntry]) -> Result<(), FloorAdmissionWatchdogError> {
    fn visit(
        index: usize,
        entries: &[CheckedEntry],
        by_id: &BTreeMap<&str, usize>,
        visiting: &mut BTreeSet<usize>,
        complete: &mut BTreeSet<usize>,
    ) -> Result<(), FloorAdmissionWatchdogError> {
        if complete.contains(&index) {
            return Ok(());
        }
        if !visiting.insert(index) {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "catalog cycle reaches '{}'",
                entries[index].id
            )));
        }
        for input in &entries[index].inputs {
            visit(by_id[input.as_str()], entries, by_id, visiting, complete)?;
        }
        visiting.remove(&index);
        complete.insert(index);
        Ok(())
    }

    let by_id = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut visiting = BTreeSet::new();
    let mut complete = BTreeSet::new();
    for index in 0..entries.len() {
        visit(index, entries, &by_id, &mut visiting, &mut complete)?;
    }
    Ok(())
}

fn validate_receipts(
    entries: &[CheckedEntry],
    receipts: &[CheckedReceipt],
) -> Result<(), FloorAdmissionWatchdogError> {
    let by_entry = entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut receipt_ids = BTreeSet::new();
    let mut budgets = BTreeMap::<&str, usize>::new();
    let mut admitted = BTreeMap::<&str, usize>::new();
    let mut slots = BTreeSet::<(&str, &str)>::new();

    for receipt in receipts {
        let entry = by_entry.get(receipt.entry_id.as_str()).ok_or_else(|| {
            FloorAdmissionWatchdogError::Refused(format!(
                "receipt names absent catalog member '{}'",
                receipt.entry_id
            ))
        })?;
        if entry.provenance == CheckedProvenance::Derived {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "derived catalog member '{}' carries an exhaustion receipt",
                receipt.entry_id
            )));
        }
        if !receipt_ids.insert(receipt.entry_id.as_str()) {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "catalog member '{}' carries duplicate receipts",
                receipt.entry_id
            )));
        }
        match budgets.get(receipt.phenomenon.as_str()) {
            Some(expected) if *expected != receipt.buckingham_pi_groups => {
                return Err(FloorAdmissionWatchdogError::Refused(format!(
                    "phenomenon '{}' declares inconsistent Buckingham Pi budgets",
                    receipt.phenomenon
                )));
            }
            Some(_) => {}
            None => {
                budgets.insert(receipt.phenomenon.as_str(), receipt.buckingham_pi_groups);
            }
        }
        if !slots.insert((receipt.phenomenon.as_str(), receipt.residual_slot.as_str())) {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "phenomenon '{}' repeats residual slot '{}'",
                receipt.phenomenon, receipt.residual_slot
            )));
        }
        *admitted.entry(receipt.phenomenon.as_str()).or_insert(0) += 1;
    }

    for entry in entries {
        if entry.provenance != CheckedProvenance::Derived
            && !receipt_ids.contains(entry.id.as_str())
        {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "irreducible catalog member '{}' has no exhaustion receipt",
                entry.id
            )));
        }
    }
    for (phenomenon, count) in admitted {
        if count > budgets[phenomenon] {
            return Err(FloorAdmissionWatchdogError::Refused(format!(
                "phenomenon '{phenomenon}' admits {count} slots against budget {}",
                budgets[phenomenon]
            )));
        }
    }
    Ok(())
}

pub(crate) fn verify_floor_catalog_admission_bytes(
    bytes: &[u8],
) -> Result<FloorAdmissionWatchdogReceipt, FloorAdmissionWatchdogError> {
    let mut cursor = ByteCursor::new(bytes);
    let schema = cursor.required_text("schema_id")?;
    if schema != INPUT_SCHEMA_ID {
        return Err(FloorAdmissionWatchdogError::Refused(format!(
            "unsupported input schema '{schema}'"
        )));
    }
    let entries = parse_entries(&mut cursor)?;
    let receipts = parse_receipts(&mut cursor)?;
    cursor.finish()?;

    validate_entries(&entries)?;
    validate_receipts(&entries, &receipts)?;

    let input_digest: [u8; 32] = Sha256::digest(bytes).into();
    if input_digest != EXPECTED_INPUT_SHA256 {
        return Err(FloorAdmissionWatchdogError::CanonicalInputMismatch {
            expected: EXPECTED_INPUT_SHA256,
            found: input_digest,
        });
    }

    let mut result = Sha256::new();
    result.update(CHECKER_IMPLEMENTATION_ID.as_bytes());
    result.update(input_digest);
    result.update((entries.len() as u64).to_le_bytes());
    result.update((receipts.len() as u64).to_le_bytes());
    result.update(b"admitted");
    Ok(FloorAdmissionWatchdogReceipt {
        input_digest,
        result_digest: result.finalize().into(),
        entry_count: entries.len(),
        receipt_count: receipts.len(),
    })
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
