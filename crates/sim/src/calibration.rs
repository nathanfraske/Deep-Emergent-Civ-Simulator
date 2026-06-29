// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The calibration manifest (runbook section 4, design Principle 11).
//!
//! A mechanism is fixed Rust; the numbers it needs are the owner's, and until he
//! sets them they are reserved, not guessed. Every reserved value is one entry in
//! `calibration/reserved.toml`. The loader reads the manifest at startup; a
//! `reserved` entry with an empty value loads as a sentinel, and any system that
//! reads an unset required value fails loudly rather than running on a silent
//! default. Nothing is hardcoded inline; every tuneable is a named manifest entry.
//!
//! Two build profiles follow: [`Profile::Development`], in which a system whose
//! required values are still reserved is gated off; and [`Profile::Calibrated`], in
//! which the build refuses to start if any enabled system has a required value
//! still reserved.

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// One reserved value, mirroring the `calibration/reserved.toml` schema.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ReservedValue {
    /// Namespaced identifier, for example `compose.max_depth`.
    pub id: String,
    /// The ground on which the owner would decide the value. Mandatory and concrete.
    pub basis: String,
    /// `"reserved"` or `"set"`.
    pub status: String,
    /// The owner's number; empty while reserved.
    #[serde(default)]
    pub value: String,
    /// The unit the value is expressed in.
    #[serde(default)]
    pub unit: String,
    /// Who set it, once set.
    #[serde(default)]
    pub set_by: String,
    /// When it was set, once set.
    #[serde(default)]
    pub set_date: String,
    /// A pointer back to the mechanism (design part, record, audit section).
    pub source: String,
}

impl ReservedValue {
    /// Whether this entry has graduated from reserved to set with a non-empty value.
    pub fn is_set(&self) -> bool {
        self.status == "set" && !self.value.trim().is_empty()
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct ManifestFile {
    #[serde(default)]
    reserved: Vec<ReservedValue>,
}

/// The build profile (runbook section 4d).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Gated systems whose required values are reserved run only structural and
    /// determinism tests; reading an unset value is still an error.
    Development,
    /// The build refuses to start if any enabled system has a required value still
    /// reserved.
    Calibrated,
}

/// What can go wrong when reading a calibration value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalibrationError {
    /// The manifest could not be parsed.
    Parse(String),
    /// The manifest file could not be read.
    Io(String),
    /// No entry exists for the requested id.
    Unknown(String),
    /// The entry exists but is still reserved (the fail-loud sentinel).
    Reserved(String),
    /// The entry is set, but its value could not be read as the requested type.
    BadValue { id: String, detail: String },
    /// One or more enabled required values are still reserved (calibrated profile).
    UnsatisfiedRequirements(Vec<String>),
    /// A duplicate id appears in the manifest.
    Duplicate(String),
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalibrationError::Parse(m) => write!(f, "manifest parse error: {m}"),
            CalibrationError::Io(m) => write!(f, "manifest read error: {m}"),
            CalibrationError::Unknown(id) => write!(f, "no calibration entry for id '{id}'"),
            CalibrationError::Reserved(id) => write!(
                f,
                "calibration value '{id}' is reserved and unset; the owner must set it before it is read (never fabricate a value)"
            ),
            CalibrationError::BadValue { id, detail } => {
                write!(f, "calibration value '{id}' could not be read: {detail}")
            }
            CalibrationError::UnsatisfiedRequirements(ids) => {
                write!(f, "calibrated profile refused to start; reserved required values: {}", ids.join(", "))
            }
            CalibrationError::Duplicate(id) => write!(f, "duplicate calibration id '{id}'"),
        }
    }
}

impl std::error::Error for CalibrationError {}

/// The loaded calibration manifest: the entries, in file order, keyed by id.
#[derive(Debug)]
pub struct CalibrationManifest {
    order: Vec<String>,
    values: HashMap<String, ReservedValue>,
}

impl CalibrationManifest {
    /// Parse a manifest from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, CalibrationError> {
        let file: ManifestFile =
            toml::from_str(s).map_err(|e| CalibrationError::Parse(e.to_string()))?;
        let mut order = Vec::with_capacity(file.reserved.len());
        let mut values = HashMap::with_capacity(file.reserved.len());
        for entry in file.reserved {
            if values.contains_key(&entry.id) {
                return Err(CalibrationError::Duplicate(entry.id));
            }
            order.push(entry.id.clone());
            values.insert(entry.id.clone(), entry);
        }
        Ok(CalibrationManifest { order, values })
    }

    /// Load a manifest from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CalibrationError> {
        let text =
            std::fs::read_to_string(path).map_err(|e| CalibrationError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The entry for an id, set or reserved.
    pub fn get(&self, id: &str) -> Option<&ReservedValue> {
        self.values.get(id)
    }

    /// Whether an id exists and is set.
    pub fn is_set(&self, id: &str) -> bool {
        self.values.get(id).map(|v| v.is_set()).unwrap_or(false)
    }

    /// Whether an id exists and is still reserved.
    pub fn is_reserved(&self, id: &str) -> bool {
        self.values.get(id).map(|v| !v.is_set()).unwrap_or(false)
    }

    /// Every entry, in file order.
    pub fn entries(&self) -> impl Iterator<Item = &ReservedValue> + '_ {
        self.order.iter().map(move |id| &self.values[id])
    }

    /// The ids still reserved, in file order: the standing review queue that CI and
    /// the reserved-values panel render (runbook section 4d).
    pub fn reserved_ids(&self) -> Vec<&str> {
        self.order
            .iter()
            .filter(|id| !self.values[*id].is_set())
            .map(|s| s.as_str())
            .collect()
    }

    /// The raw set value string for an id, or a fail-loud error if reserved.
    pub fn require_str(&self, id: &str) -> Result<&str, CalibrationError> {
        let entry = self
            .values
            .get(id)
            .ok_or_else(|| CalibrationError::Unknown(id.to_string()))?;
        if !entry.is_set() {
            return Err(CalibrationError::Reserved(id.to_string()));
        }
        Ok(entry.value.trim())
    }

    /// A required integer value. Fails loud if reserved or unparseable.
    pub fn require_i64(&self, id: &str) -> Result<i64, CalibrationError> {
        let raw = self.require_str(id)?;
        raw.parse::<i64>().map_err(|e| CalibrationError::BadValue {
            id: id.to_string(),
            detail: format!("not an integer: {e}"),
        })
    }

    /// A required fixed-point value, parsed from a decimal string without ever
    /// going through floating point, so the result is exact and deterministic.
    pub fn require_fixed(&self, id: &str) -> Result<Fixed, CalibrationError> {
        let raw = self.require_str(id)?;
        parse_decimal_fixed(raw).map_err(|detail| CalibrationError::BadValue {
            id: id.to_string(),
            detail,
        })
    }

    /// Enforce the calibrated profile: every id in `enabled` must exist and be set.
    /// Returns the list of unsatisfied (unknown or reserved) ids as an error.
    pub fn ensure_all_set(&self, enabled: &[&str]) -> Result<(), CalibrationError> {
        let missing: Vec<String> = enabled
            .iter()
            .filter(|id| !self.is_set(id))
            .map(|s| s.to_string())
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(CalibrationError::UnsatisfiedRequirements(missing))
        }
    }

    /// The profile gate. Under [`Profile::Calibrated`], refuse to start if any
    /// enabled required value is still reserved. Under [`Profile::Development`],
    /// always proceed (gated systems are simply not enabled).
    pub fn gate(&self, profile: Profile, enabled: &[&str]) -> Result<(), CalibrationError> {
        match profile {
            Profile::Development => Ok(()),
            Profile::Calibrated => self.ensure_all_set(enabled),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether the manifest is empty.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

/// Parse a decimal string into [`Fixed`] using only integer arithmetic, so the
/// conversion is exact to the fixed-point grid and identical across machines.
fn parse_decimal_fixed(s: &str) -> Result<Fixed, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty value".to_string());
    }
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int_str, frac_str) = match body.split_once('.') {
        Some((a, b)) => (a, b),
        None => (body, ""),
    };
    if frac_str.len() > 30 {
        return Err("too many fractional digits".to_string());
    }
    let int_val: i128 = if int_str.is_empty() {
        0
    } else {
        int_str
            .parse::<i128>()
            .map_err(|e| format!("bad integer part: {e}"))?
    };
    let mut bits: i128 = int_val << 32;
    if !frac_str.is_empty() {
        let digits: i128 = frac_str
            .parse::<i128>()
            .map_err(|e| format!("bad fractional part: {e}"))?;
        let mut den: i128 = 1;
        for _ in 0..frac_str.len() {
            den *= 10;
        }
        bits += (digits << 32) / den;
    }
    if neg {
        bits = -bits;
    }
    if bits < i64::MIN as i128 || bits > i64::MAX as i128 {
        return Err("value out of Q32.32 range".to_string());
    }
    Ok(Fixed::from_bits(bits as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[reserved]]
id = "compose.max_depth"
basis = "the per-tick budget and the depth at which marginal proxy gain falls below noise"
status = "reserved"
value = ""
unit = "levels"
source = "Part 41 composition mechanism; record 62.10; audit section 1l"

[[reserved]]
id = "evidence.decay_rate"
basis = "set equal to the transmission subsystem drift and loss rates for consistency"
status = "set"
value = "0.25"
unit = "per_day"
set_by = "Nathan M. Fraske"
set_date = "2026-06-29"
source = "Part 9 evidence engine; record 62.6"

[[reserved]]
id = "tier.promote_threshold"
basis = "the in-world significance at which an individual becomes load-bearing"
status = "set"
value = "8"
unit = "count"
set_by = "Nathan M. Fraske"
set_date = "2026-06-29"
source = "Part 54 tier consistency; record 62.9"
"#;

    #[test]
    fn parses_and_indexes() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        assert_eq!(m.len(), 3);
        assert!(m.is_reserved("compose.max_depth"));
        assert!(m.is_set("evidence.decay_rate"));
    }

    #[test]
    fn reading_a_reserved_value_fails_loud() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        let err = m.require_fixed("compose.max_depth").unwrap_err();
        assert_eq!(
            err,
            CalibrationError::Reserved("compose.max_depth".to_string())
        );
    }

    #[test]
    fn reading_an_unknown_value_is_distinct_from_reserved() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        assert_eq!(
            m.require_i64("does.not.exist").unwrap_err(),
            CalibrationError::Unknown("does.not.exist".to_string())
        );
    }

    #[test]
    fn set_values_read_back_exactly() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        assert_eq!(
            m.require_fixed("evidence.decay_rate").unwrap(),
            Fixed::from_ratio(1, 4)
        );
        assert_eq!(m.require_i64("tier.promote_threshold").unwrap(), 8);
    }

    #[test]
    fn reserved_ids_are_the_review_queue() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        assert_eq!(m.reserved_ids(), vec!["compose.max_depth"]);
    }

    #[test]
    fn calibrated_profile_refuses_with_reserved_requirement() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        // A system requiring the still-reserved depth cannot start under calibrated.
        let err = m
            .gate(Profile::Calibrated, &["compose.max_depth"])
            .unwrap_err();
        assert_eq!(
            err,
            CalibrationError::UnsatisfiedRequirements(vec!["compose.max_depth".to_string()])
        );
        // Development always proceeds.
        assert!(m.gate(Profile::Development, &["compose.max_depth"]).is_ok());
        // A system requiring only set values starts under either profile.
        assert!(m
            .gate(
                Profile::Calibrated,
                &["evidence.decay_rate", "tier.promote_threshold"]
            )
            .is_ok());
    }

    #[test]
    fn decimal_parse_is_exact_and_signed() {
        assert_eq!(parse_decimal_fixed("1").unwrap(), Fixed::from_int(1));
        assert_eq!(parse_decimal_fixed("0.5").unwrap(), Fixed::from_ratio(1, 2));
        assert_eq!(
            parse_decimal_fixed("-0.25").unwrap(),
            Fixed::from_ratio(-1, 4)
        );
        assert_eq!(parse_decimal_fixed("2.0").unwrap(), Fixed::from_int(2));
        assert!(parse_decimal_fixed("abc").is_err());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let dup = r#"
[[reserved]]
id = "x.y"
basis = "b"
status = "reserved"
source = "s"
[[reserved]]
id = "x.y"
basis = "b"
status = "reserved"
source = "s"
"#;
        assert_eq!(
            CalibrationManifest::from_toml_str(dup).unwrap_err(),
            CalibrationError::Duplicate("x.y".to_string())
        );
    }
}
