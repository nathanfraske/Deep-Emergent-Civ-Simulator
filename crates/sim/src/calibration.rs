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

use crate::decision::Curve;
use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[allow(clippy::disallowed_types)] // R-CANON-WALK opt-out, justified below
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
    /// The three-way-test category (AGENTIC_ADDENDUM section 9): `fundamental`, `per_world`, `derivable`,
    /// or `defect`. Empty during migration, read as `Category::Unclassified`. ADDITIVE: an absent field
    /// does not break an in-flight entry, but a NON-EMPTY value that is not one of the four fails loud (a
    /// mislabel fails the build), so once the per-entry sweep lands every entry is born categorized.
    #[serde(default)]
    pub category: String,
    /// The provenance tag (the genesis-forward provenance-DAG accounting): `measured`, `closure`,
    /// `contingency`, or `derived`. Empty during migration, read as `Provenance::Unclassified`. This axis
    /// is ORTHOGONAL to `category`: category is the three-way authorship test (is it an authored world
    /// value), provenance is the refutability test (could an independent laboratory refute this number
    /// WITHOUT running this simulator). ADDITIVE (an absent field is Unclassified); an unknown value fails
    /// loud.
    #[serde(default)]
    pub provenance: String,
    /// For a `derived` value, the ids it derives FROM: the provenance-DAG edges. A derived value's
    /// EFFECTIVE provenance is the worst-case join over these transitive inputs (it is only as pinned as
    /// its least-pinned input, and closure-tainted the moment its DAG touches a closure), so authorship
    /// hides not in the lines tagged `closure` but in the `derived` lines whose ancestry passes through
    /// one. A non-derived value declares none.
    #[serde(default)]
    pub inputs: Vec<String>,
}

/// The three-way-test category of a reserved value (AGENTIC_ADDENDUM section 9, the fundamental-constants
/// floor), machine-checked so every entry is born categorized rather than sitting in an ambiguous manifest.
/// The fourth DEFECT state records a value that fits none of the three legitimate categories (a global
/// authored magnitude that is a bug in the derivation), so it is FLAGGED rather than laundered into a
/// legitimate category. UNCLASSIFIED is the additive-migration default for an entry that has not yet
/// declared its category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// (1) A fundamental universal constant on the small closed fundamentals list (c, k_B, h, e, eps_0, N_A).
    Fundamental,
    /// (2) A per-world / per-substance / per-race datum read from the world's own data.
    PerWorld,
    /// (3) Derivable from (1) and (2); never stored as its own number once its substrate lands.
    Derivable,
    /// A value that fits none of the three: a defect (a bug in the derivation), flagged not laundered.
    Defect,
    /// Not yet declared: the additive-migration default for an absent or empty category field.
    Unclassified,
}

/// The provenance tag of a value (the genesis-forward provenance-DAG accounting, orthogonal to
/// [`Category`]). The operational test that decides the tag: could an independent laboratory refute this
/// number WITHOUT running this simulator? Yes for a MEASURED floor value (refutable by observation,
/// carrying error bars) and a CONTINGENCY per-world initial condition; no for a CLOSURE (an unpinned or
/// weakly-pinned free knob where turning it changes outcomes without contradicting a measurement). A
/// DERIVED value is computed from others by a named law and is only as pinned as its least-pinned input,
/// so its EFFECTIVE provenance is the worst-case join up the DAG: it passes the refutability test exactly
/// when its ancestry bottoms out entirely in measured and contingency leaves, and it is closure-tainted
/// the moment the DAG touches a single closure. The closure-reachability query over this axis is the true
/// free-knob surface of the calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// A pinned floor value, refutable by observation without the sim (carries error bars, not free).
    Measured,
    /// A per-world sampled initial condition (the layer-4 contingency vector), authored because nature
    /// did not derive it either, but with derived evolution, attractors, and priors.
    Contingency,
    /// Computed from other values by a named law; only as pinned as its least-pinned transitive input.
    Derived,
    /// An unpinned or weakly-pinned free knob: turning it changes outcomes without contradicting a
    /// measurement. This is the authorship surface the reachability query hunts.
    Closure,
    /// Not yet declared: the additive-migration default for an absent or empty provenance field.
    Unclassified,
}

impl Provenance {
    /// The pinned-ness rank, lower is worse (less pinned), for the worst-case join up the DAG. Closure is
    /// the worst, an Unclassified (unknown) value is treated as more suspect than a declared Derived, and
    /// Measured and Contingency are the pinned leaves. The join of a set is the member of minimum rank.
    fn rank(self) -> u8 {
        match self {
            Provenance::Closure => 0,
            Provenance::Unclassified => 1,
            Provenance::Derived => 2,
            Provenance::Contingency => 3,
            Provenance::Measured => 4,
        }
    }
}

impl ReservedValue {
    /// Whether this entry has graduated from reserved to set with a non-empty value.
    pub fn is_set(&self) -> bool {
        self.status == "set" && !self.value.trim().is_empty()
    }

    /// This entry's three-way-test category. An empty field reads UNCLASSIFIED (the migration default); a
    /// non-empty field that is not one of the four known values fails loud (a mislabel fails the build).
    pub fn category(&self) -> Result<Category, CalibrationError> {
        match self.category.trim() {
            "" => Ok(Category::Unclassified),
            "fundamental" => Ok(Category::Fundamental),
            "per_world" => Ok(Category::PerWorld),
            "derivable" => Ok(Category::Derivable),
            "defect" => Ok(Category::Defect),
            other => Err(CalibrationError::BadValue {
                id: self.id.clone(),
                detail: format!(
                    "unknown category '{other}', expected fundamental, per_world, derivable, or defect"
                ),
            }),
        }
    }

    /// This entry's DECLARED provenance tag (not the effective, DAG-joined one, which the manifest
    /// resolves). An empty field reads UNCLASSIFIED (the migration default); a non-empty field that is not
    /// one of the four known values fails loud (a mislabel fails the build).
    pub fn provenance(&self) -> Result<Provenance, CalibrationError> {
        match self.provenance.trim() {
            "" => Ok(Provenance::Unclassified),
            "measured" => Ok(Provenance::Measured),
            "contingency" => Ok(Provenance::Contingency),
            "derived" => Ok(Provenance::Derived),
            "closure" => Ok(Provenance::Closure),
            other => Err(CalibrationError::BadValue {
                id: self.id.clone(),
                detail: format!(
                    "unknown provenance '{other}', expected measured, contingency, derived, or closure"
                ),
            }),
        }
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
    // The reserved-value manifest is a name-keyed config lookup (get by key), read at
    // startup and never iterated into a state hash (R-CANON-WALK).
    #[allow(clippy::disallowed_types)]
    values: HashMap<String, ReservedValue>,
}

impl CalibrationManifest {
    /// Parse a manifest from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, CalibrationError> {
        let file: ManifestFile =
            toml::from_str(s).map_err(|e| CalibrationError::Parse(e.to_string()))?;
        let mut order = Vec::with_capacity(file.reserved.len());
        #[allow(clippy::disallowed_types)] // R-CANON-WALK opt-out, justified below
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

    /// The born-categorized gate (AGENTIC_ADDENDUM section 9): parse every entry's three-way-test category,
    /// failing loud on the FIRST non-empty value that is not one of the four known categories, so a mislabel
    /// fails the build. Returns the ids still UNCLASSIFIED (an empty category field), the migration remainder
    /// the per-entry sweep closes; an empty return means every entry is born categorized. This is ADDITIVE:
    /// an absent field is tolerated as UNCLASSIFIED and never errors, only an invalid one does.
    pub fn validate_categories(&self) -> Result<Vec<&str>, CalibrationError> {
        let mut unclassified = Vec::new();
        for id in &self.order {
            if self.values[id].category()? == Category::Unclassified {
                unclassified.push(id.as_str());
            }
        }
        Ok(unclassified)
    }

    /// The EFFECTIVE provenance of a value: the worst-case join of its declared provenance with the
    /// effective provenance of every input, transitively up the provenance DAG. A value is only as pinned
    /// as its least-pinned transitive input (the join takes the minimum-rank member), so a `derived` value
    /// whose ancestry touches a `closure` resolves to `Closure` here even though its own declared tag is
    /// `derived`. Only a `derived` value joins over inputs; a leaf's effective provenance is its own tag.
    /// Cycle-safe: an id already on the current resolution path is not re-entered (a cycle is a defect
    /// [`Self::validate_provenance`] reports; here it simply does not loop), and an input naming an unknown
    /// id contributes `Unclassified` (a suspect leaf) rather than panicking.
    pub fn effective_provenance(&self, id: &str) -> Result<Provenance, CalibrationError> {
        let mut on_path = std::collections::BTreeSet::new();
        self.effective_provenance_inner(id, &mut on_path)
    }

    fn effective_provenance_inner(
        &self,
        id: &str,
        on_path: &mut std::collections::BTreeSet<String>,
    ) -> Result<Provenance, CalibrationError> {
        let Some(v) = self.values.get(id) else {
            return Ok(Provenance::Unclassified);
        };
        let own = v.provenance()?;
        if own != Provenance::Derived {
            return Ok(own);
        }
        if !on_path.insert(id.to_string()) {
            return Ok(Provenance::Unclassified);
        }
        let mut worst = own;
        for input in &v.inputs {
            let eff = self.effective_provenance_inner(input, on_path)?;
            if eff.rank() < worst.rank() {
                worst = eff;
            }
        }
        on_path.remove(id);
        Ok(worst)
    }

    /// The closure-reachability query: the ids whose EFFECTIVE provenance is `Closure` through an
    /// INHERITED taint (their own declared provenance is not `closure`, but a closure sits somewhere in
    /// their transitive inputs). This is the subtle free-knob surface, the `derived` lines whose ancestry
    /// passes through a closure, distinct from the declared closures themselves (the roots). Returned in
    /// file order, deterministic.
    pub fn closure_reachable(&self) -> Result<Vec<&str>, CalibrationError> {
        let mut out = Vec::new();
        for id in &self.order {
            let own = self.values[id].provenance()?;
            if own != Provenance::Closure && self.effective_provenance(id)? == Provenance::Closure {
                out.push(id.as_str());
            }
        }
        Ok(out)
    }

    /// The provenance gate, sibling to [`Self::validate_categories`]: parse every entry's provenance and
    /// check the DAG is well-formed. A `derived` value must declare at least one input (else it is not
    /// derived); a non-derived value must declare none (a leaf has no DAG edges); every input must name an
    /// id the manifest carries; and the DAG must be acyclic (a cycle has no well-defined worst-case join).
    /// Fails loud on the first structural defect. Returns the ids still UNCLASSIFIED (the migration
    /// remainder), like `validate_categories`; an empty return means every entry declares a provenance.
    /// ADDITIVE: an absent field is Unclassified and never errors.
    pub fn validate_provenance(&self) -> Result<Vec<&str>, CalibrationError> {
        let mut unclassified = Vec::new();
        for id in &self.order {
            let v = &self.values[id];
            let p = v.provenance()?;
            if p == Provenance::Derived {
                if v.inputs.is_empty() {
                    return Err(CalibrationError::BadValue {
                        id: id.clone(),
                        detail: "a derived value must declare at least one input (the provenance-DAG edges it derives from)".to_string(),
                    });
                }
            } else if !v.inputs.is_empty() {
                return Err(CalibrationError::BadValue {
                    id: id.clone(),
                    detail: format!("a {p:?} value declares inputs, but only a derived value has provenance-DAG edges"),
                });
            }
            for input in &v.inputs {
                if !self.values.contains_key(input) {
                    return Err(CalibrationError::BadValue {
                        id: id.clone(),
                        detail: format!("input '{input}' names an id the manifest does not carry"),
                    });
                }
            }
            if p == Provenance::Unclassified {
                unclassified.push(id.as_str());
            }
        }
        self.check_acyclic()?;
        Ok(unclassified)
    }

    /// Depth-first cycle detection over the provenance-DAG input edges: a cycle has no well-defined
    /// worst-case join, so it fails loud. Iterative (an explicit stack of `(id, next-input-index)`), so a
    /// deep chain does not overflow the call stack.
    fn check_acyclic(&self) -> Result<(), CalibrationError> {
        const UNVISITED: u8 = 0;
        const IN_PROGRESS: u8 = 1;
        const DONE: u8 = 2;
        // An ordered BTreeMap (not the crate's unordered lookup map): deterministic, R-CANON-WALK-clean.
        let mut mark: std::collections::BTreeMap<&str, u8> = std::collections::BTreeMap::new();
        for start in &self.order {
            if mark.get(start.as_str()).copied().unwrap_or(UNVISITED) != UNVISITED {
                continue;
            }
            let mut stack: Vec<(&str, usize)> = vec![(start.as_str(), 0)];
            mark.insert(start.as_str(), IN_PROGRESS);
            while let Some(&(id, idx)) = stack.last() {
                let inputs = self
                    .values
                    .get(id)
                    .map(|v| v.inputs.as_slice())
                    .unwrap_or(&[]);
                if idx < inputs.len() {
                    let child = inputs[idx].as_str();
                    stack.last_mut().unwrap().1 += 1;
                    match mark.get(child).copied().unwrap_or(UNVISITED) {
                        IN_PROGRESS => {
                            return Err(CalibrationError::BadValue {
                                id: child.to_string(),
                                detail: "a provenance-DAG cycle: this id is reachable from its own inputs"
                                    .to_string(),
                            });
                        }
                        DONE => {}
                        _ => {
                            mark.insert(child, IN_PROGRESS);
                            stack.push((child, 0));
                        }
                    }
                } else {
                    mark.insert(id, DONE);
                    stack.pop();
                }
            }
        }
        Ok(())
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

    /// A required non-negative count, for a reserved value whose shape is a `usize` budget (a cognition
    /// depth cap, a ring size). Reads the integer through [`Self::require_i64`] and refuses a negative
    /// value fail-loud rather than wrapping it, so a mis-signed count is a build error, not a silent
    /// enormous cap.
    pub fn require_usize(&self, id: &str) -> Result<usize, CalibrationError> {
        let v = self.require_i64(id)?;
        usize::try_from(v).map_err(|_| CalibrationError::BadValue {
            id: id.to_string(),
            detail: format!("count must be non-negative, got {v}"),
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

    /// A required map value, for a reserved value whose shape is a variable-membership
    /// set (the per-operator drift rates, a per-axis drain vector, a named-component
    /// bundle). Parsed from a `"key1=v1,key2=v2"` string into a deterministically-ordered
    /// map of fixed-point values, each value taking the same exact decimal-to-fixed path
    /// as [`Self::require_fixed`] so the map is bit-identical across machines, and the membership
    /// grows with the data rather than being fixed in code (Principle 11). Fails loud if
    /// reserved, malformed, empty, or carrying a duplicate key.
    pub fn require_map(&self, id: &str) -> Result<BTreeMap<String, Fixed>, CalibrationError> {
        let raw = self.require_str(id)?;
        let mut map = BTreeMap::new();
        for pair in raw.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            let (key, val) = pair
                .split_once('=')
                .ok_or_else(|| CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("map entry '{pair}' is not key=value"),
                })?;
            let key = key.trim().to_string();
            let value =
                parse_decimal_fixed(val.trim()).map_err(|detail| CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("value for '{key}': {detail}"),
                })?;
            if map.insert(key.clone(), value).is_some() {
                return Err(CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("duplicate map key '{key}'"),
                });
            }
        }
        if map.is_empty() {
            return Err(CalibrationError::BadValue {
                id: id.to_string(),
                detail: "empty map".to_string(),
            });
        }
        Ok(map)
    }

    /// A required response-curve value, for a reserved value whose shape is a set of `(x, y)`
    /// points (the memory-to-ring-slots map, an entrenchment-threshold curve). Parsed from an
    /// `"x1=y1,x2=y2"` string into a [`Curve`], each coordinate taking the same exact
    /// decimal-to-fixed path as [`Self::require_fixed`] so the curve is bit-identical across machines,
    /// and the membership (the number and placement of points) grows with the data rather than
    /// being fixed in code (Principle 11). The points need not be pre-sorted; [`Curve::new`]
    /// orders them. Fails loud if reserved, malformed, or empty.
    pub fn require_curve(&self, id: &str) -> Result<Curve, CalibrationError> {
        let raw = self.require_str(id)?;
        let mut points: Vec<(Fixed, Fixed)> = Vec::new();
        for pair in raw.split(',') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            let (xs, ys) = pair
                .split_once('=')
                .ok_or_else(|| CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("curve point '{pair}' is not x=y"),
                })?;
            let x =
                parse_decimal_fixed(xs.trim()).map_err(|detail| CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("point x '{}': {detail}", xs.trim()),
                })?;
            let y =
                parse_decimal_fixed(ys.trim()).map_err(|detail| CalibrationError::BadValue {
                    id: id.to_string(),
                    detail: format!("point y '{}': {detail}", ys.trim()),
                })?;
            points.push((x, y));
        }
        if points.is_empty() {
            return Err(CalibrationError::BadValue {
                id: id.to_string(),
                detail: "empty curve".to_string(),
            });
        }
        Ok(Curve::new(points))
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

[[reserved]]
id = "sample.map"
basis = "a structured-value set, exercising require_map"
status = "set"
value = "alpha=0.5,gamma=0.25,beta=0.125"
unit = "set"
set_by = "Nathan M. Fraske"
set_date = "2026-07-03"
source = "test fixture"
"#;

    #[test]
    fn parses_and_indexes() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        assert_eq!(m.len(), 4);
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
    fn the_real_manifest_compound_entries_parse_as_maps() {
        let m = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();
        // The four non-sound-change drift operators are a graduated data map.
        let drift = m.require_map("lang.drift_operator_rates").unwrap();
        assert_eq!(drift.len(), 4);
        assert!(drift.contains_key("lexical_replacement") && drift.contains_key("borrowing"));
        // Its stress siblings scale the whole set and still parse as maps.
        assert_eq!(
            m.require_map("lang.drift_operator_rates.high")
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            m.require_map("lang.drift_operator_rates.low")
                .unwrap()
                .len(),
            4
        );
        // The two conformity strengths are a map; the fission and deviation thresholds
        // are separate entries, still reserved, so a scalar read of them fails loud.
        let conf = m
            .require_map("axiom.conformity_prestige_strengths")
            .unwrap();
        assert_eq!(conf.len(), 2);
        assert_eq!(conf["conformity"], conf["prestige"]);
        assert!(m.require_fixed("axiom.calcification_brittleness").is_err());
        assert!(m.require_fixed("axiom.fission_threshold").is_err());
    }

    #[test]
    fn the_real_manifest_is_fully_born_categorized() {
        // The born-categorized CI gate (AGENTIC_ADDENDUM section 9): every entry in the real manifest carries
        // a VALID three-way-test category (a mislabel, a non-empty value that is not one of the four, fails
        // validate_categories, so it fails the build) AND a NON-EMPTY one (the per-entry census sweep has
        // landed, so no entry is UNCLASSIFIED). The mechanism itself stays additive: an absent field loads as
        // UNCLASSIFIED without erroring, so an in-flight #120/#123 entry does not panic at load. This CI gate
        // is what makes "every entry born categorized" real: it catches a dropped `category` field or a
        // reverted sweep that the additive loader would otherwise tolerate silently.
        let m = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();
        let unclassified = m.validate_categories().expect(
            "every category string in the real manifest must be valid (a mislabel fails the build)",
        );
        assert!(
            unclassified.is_empty(),
            "every entry must be born categorized; these are still UNCLASSIFIED: {unclassified:?}"
        );
    }

    #[test]
    fn an_unknown_category_fails_loud_so_a_mislabel_fails_the_build() {
        let bad = r#"
[[reserved]]
id = "sample.x"
basis = "b"
status = "reserved"
source = "s"
category = "sometimes"
"#;
        let m = CalibrationManifest::from_toml_str(bad).unwrap();
        assert!(matches!(
            m.validate_categories(),
            Err(CalibrationError::BadValue { .. })
        ));
    }

    #[test]
    fn the_four_categories_parse_and_an_absent_one_is_unclassified() {
        let toml = r#"
[[reserved]]
id = "a.fundamental"
basis = "b"
status = "reserved"
source = "s"
category = "fundamental"

[[reserved]]
id = "a.per_world"
basis = "b"
status = "reserved"
source = "s"
category = "per_world"

[[reserved]]
id = "a.derivable"
basis = "b"
status = "reserved"
source = "s"
category = "derivable"

[[reserved]]
id = "a.defect"
basis = "b"
status = "reserved"
source = "s"
category = "defect"

[[reserved]]
id = "a.absent"
basis = "b"
status = "reserved"
source = "s"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        assert_eq!(
            m.get("a.fundamental").unwrap().category().unwrap(),
            Category::Fundamental
        );
        assert_eq!(
            m.get("a.per_world").unwrap().category().unwrap(),
            Category::PerWorld
        );
        assert_eq!(
            m.get("a.derivable").unwrap().category().unwrap(),
            Category::Derivable
        );
        assert_eq!(
            m.get("a.defect").unwrap().category().unwrap(),
            Category::Defect
        );
        assert_eq!(
            m.get("a.absent").unwrap().category().unwrap(),
            Category::Unclassified
        );
        // The absent one is the only UNCLASSIFIED; the four declared are born categorized.
        assert_eq!(m.validate_categories().unwrap(), vec!["a.absent"]);
    }

    #[test]
    fn a_map_value_parses_exactly_in_sorted_order_and_fails_loud() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        let map = m.require_map("sample.map").unwrap();
        assert_eq!(map.len(), 3);
        assert_eq!(map["alpha"], Fixed::from_ratio(1, 2));
        assert_eq!(map["beta"], Fixed::from_ratio(1, 8));
        assert_eq!(map["gamma"], Fixed::from_ratio(1, 4));
        // BTreeMap sorts keys regardless of source order, so the walk is deterministic.
        let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, ["alpha", "beta", "gamma"]);
        // A reserved map fails loud like any reserved read.
        assert!(matches!(
            m.require_map("compose.max_depth").unwrap_err(),
            CalibrationError::Reserved(_)
        ));
        // A malformed entry (no key=value) is a BadValue, never a silent guess.
        let bad = CalibrationManifest::from_toml_str(
            "[[reserved]]\nid = \"x\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"novalue\"\nunit = \"set\"\nset_by = \"o\"\nset_date = \"d\"\nsource = \"s\"\n",
        )
        .unwrap();
        assert!(matches!(
            bad.require_map("x").unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
    }

    #[test]
    fn require_curve_fails_loud_while_reserved() {
        let m = CalibrationManifest::from_toml_str(SAMPLE).unwrap();
        // A reserved id read as a curve fails loud, never a fabricated shape.
        assert!(matches!(
            m.require_curve("compose.max_depth").unwrap_err(),
            CalibrationError::Reserved(_)
        ));
        // An unknown id is distinct from reserved.
        assert!(matches!(
            m.require_curve("no.such.curve").unwrap_err(),
            CalibrationError::Unknown(_)
        ));
    }

    #[test]
    fn require_curve_parses_points_like_require_map() {
        let toml = "[[reserved]]\nid = \"axiom.evidence_ring_curve\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"0=0,1=8,2=14\"\nunit = \"curve\"\nset_by = \"o\"\nset_date = \"d\"\nsource = \"s\"\n";
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let curve = m.require_curve("axiom.evidence_ring_curve").unwrap();
        // The parsed curve takes the same exact decimal-to-fixed path as require_map, so it
        // reads its reference points back exactly.
        assert_eq!(curve.eval(Fixed::ZERO), Fixed::ZERO);
        assert_eq!(curve.eval(Fixed::ONE), Fixed::from_int(8));
        assert_eq!(curve.eval(Fixed::from_int(2)), Fixed::from_int(14));
        // A malformed point (no x=y) is a BadValue, never a silent guess.
        let bad = "[[reserved]]\nid = \"x\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"nopoint\"\nunit = \"curve\"\nset_by = \"o\"\nset_date = \"d\"\nsource = \"s\"\n";
        let mbad = CalibrationManifest::from_toml_str(bad).unwrap();
        assert!(matches!(
            mbad.require_curve("x").unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
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

    // A `[[reserved]]` TOML block for a provenance-DAG entry: id, its provenance tag, and (for a derived
    // value) its input edges. The other fields are filler so the entry parses.
    fn prov_entry(id: &str, provenance: &str, inputs: &[&str]) -> String {
        let inputs_toml = if inputs.is_empty() {
            String::new()
        } else {
            let list = inputs
                .iter()
                .map(|i| format!("\"{i}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("inputs = [{list}]\n")
        };
        format!(
            "[[reserved]]\nid = \"{id}\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"1\"\nunit = \"u\"\nset_by = \"o\"\nset_date = \"d\"\nsource = \"s\"\nprovenance = \"{provenance}\"\n{inputs_toml}"
        )
    }

    #[test]
    fn provenance_worst_case_join_and_closure_reachability() {
        // A small provenance DAG. sigma and year derive from measured/contingency leaves (pinned);
        // t_inf derives from a closure (eta_escape), so it is closure-tainted through the join, and
        // escape_flux inherits that taint transitively through t_inf.
        let toml = [
            prov_entry("k_B", "measured", &[]),
            prov_entry("h", "measured", &[]),
            prov_entry("mass", "contingency", &[]),
            prov_entry("eta_escape", "closure", &[]),
            prov_entry("sigma", "derived", &["k_B", "h"]),
            prov_entry("year", "derived", &["mass"]),
            prov_entry("t_inf", "derived", &["eta_escape", "mass"]),
            prov_entry("escape_flux", "derived", &["t_inf"]),
        ]
        .concat();
        let m = CalibrationManifest::from_toml_str(&toml).unwrap();
        assert_eq!(
            m.validate_provenance().unwrap(),
            Vec::<&str>::new(),
            "every entry declares a provenance"
        );
        // A derived value bottoming out entirely in measured/contingency leaves is pinned, not closure.
        assert_eq!(
            m.effective_provenance("sigma").unwrap(),
            Provenance::Derived
        );
        assert_eq!(m.effective_provenance("year").unwrap(), Provenance::Derived);
        // A derived value whose DAG touches a closure resolves to Closure, transitively.
        assert_eq!(
            m.effective_provenance("t_inf").unwrap(),
            Provenance::Closure
        );
        assert_eq!(
            m.effective_provenance("escape_flux").unwrap(),
            Provenance::Closure
        );
        // A declared leaf keeps its own tag.
        assert_eq!(m.effective_provenance("k_B").unwrap(), Provenance::Measured);
        // The closure-reachability query returns the INHERITED-taint surface, not the declared closure.
        assert_eq!(m.closure_reachable().unwrap(), vec!["t_inf", "escape_flux"]);
    }

    #[test]
    fn provenance_validation_catches_malformed_dags() {
        // A derived value with no inputs is not derived.
        let m = CalibrationManifest::from_toml_str(&prov_entry("x", "derived", &[])).unwrap();
        assert!(matches!(
            m.validate_provenance().unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
        // A non-derived value declaring inputs (a leaf has no DAG edges).
        let m = CalibrationManifest::from_toml_str(
            &[
                prov_entry("a", "measured", &[]),
                prov_entry("b", "measured", &["a"]),
            ]
            .concat(),
        )
        .unwrap();
        assert!(matches!(
            m.validate_provenance().unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
        // An input naming an unknown id.
        let m =
            CalibrationManifest::from_toml_str(&prov_entry("d", "derived", &["ghost"])).unwrap();
        assert!(matches!(
            m.validate_provenance().unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
        // A cycle in the DAG has no well-defined worst-case join.
        let m = CalibrationManifest::from_toml_str(
            &[
                prov_entry("p", "derived", &["q"]),
                prov_entry("q", "derived", &["p"]),
            ]
            .concat(),
        )
        .unwrap();
        assert!(matches!(
            m.validate_provenance().unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
    }

    #[test]
    fn provenance_parsing_is_additive_and_orthogonal_to_category() {
        // An unknown provenance fails loud (a mislabel fails the gate).
        let m = CalibrationManifest::from_toml_str(&prov_entry("x", "guessed", &[])).unwrap();
        assert!(matches!(
            m.validate_provenance().unwrap_err(),
            CalibrationError::BadValue { .. }
        ));
        // An absent provenance field is UNCLASSIFIED (additive migration), returned not errored, and it is
        // orthogonal to category: the same entry with no category is unclassified on that axis too.
        let toml = "[[reserved]]\nid = \"y\"\nbasis = \"b\"\nstatus = \"set\"\nvalue = \"1\"\nunit = \"u\"\nset_by = \"o\"\nset_date = \"d\"\nsource = \"s\"\n";
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        assert_eq!(m.validate_provenance().unwrap(), vec!["y"]);
        assert_eq!(m.validate_categories().unwrap(), vec!["y"]);
    }
}
