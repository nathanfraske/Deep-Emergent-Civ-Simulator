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

//! The typological substrate (R-LANG-TYPOLOGY, design 33.4): grammar typology as a
//! data-defined registry rather than a closed set of Rust enums, the sibling of the value
//! substrate (Part 21), the semantic substrate (33.1), the institution-function substrate
//! (Part 36), and the modality substrate (record 62.13). The mechanism here is fixed Rust;
//! the parameters, their values, the per-parameter priors, and the harmonic correlations
//! are data and grow with the world, so a culture that grammaticalises a dimension the
//! authors never enumerated is representable by adding data, never by editing an enum.
//!
//! The pieces, mirroring the scoped proposal (`docs/working/LANG_TYPOLOGY_SCOPED_PROPOSAL.md`):
//!
//! - [`TypologyRegistry`]: the open parameter registry. A parameter is a registered
//!   dimension of typological variation (dominant object-verb order, adposition order,
//!   alignment per locus, the three morphological axes); its values are a registered set.
//!   Each parameter carries a `sample_priority`, the anchor-first canonical sampling order
//!   (data, grounded in the branching-direction anchor of the typological record), with
//!   ties broken by parameter id, so the conditional draw order is a function of the data
//!   rather than of struct-field order or a hash-map walk.
//! - [`TypologyPrior`]: the per-parameter draw distribution as integer language counts
//!   with a per-parameter provenance citation. The shipped seed is the cross-linguistic
//!   (WALS) record; a race may carry its own prior as a different shape over the same
//!   descriptive space, the per-race-differentiable knob. A race enters the draw through
//!   this data only, never through the draw key, so two races with identical prior data
//!   sample identically (the label-blindness discipline of `Substance::content_id`).
//! - [`HarmonyModel`]: the Greenbergian and Dryer word-order correlations as a data table
//!   of directional biases between parameter-values, read by the sampler, never a Rust
//!   branch. A pair the typological record shows NOT to correlate carries no row, which is
//!   as load-bearing as the positive rows: absence keeps the model from over-reaching into
//!   an order the data does not support.
//! - [`sample_profile`]: the seeded sampler, mechanism fixed. It walks the parameters in
//!   canonical order and, for each, tilts the prior by the biases whose condition was
//!   already drawn this pass, gates the tilt by a reserved disharmony probability so a
//!   rare disharmonic language stays reachable, and draws under the 33.4 key discipline
//!   (`DrawKey`, `Phase::LANG_TYPOLOGY`, the parameter's canonical position as the slot).
//!   The pass is a pure function of the seed, the culture coordinate, the tick, and the
//!   data, so it replays bit for bit and is independent of thread count (33.10).
//! - [`TypologyProfile`]: a culture's grammar as a canonical sorted vector over the
//!   registry, the replacement for the design's closed `GrammarParams` struct, carried by
//!   [`crate::language::Language`] and inherited through `fork`.
//! - [`typology_distance`]: the 33.5 grammatical component as a distance over the generic
//!   profile vector under reserved per-parameter weights, with a per-parameter
//!   [`crate::value::GroundMetric`] hook where a parameter's values have their own
//!   similarity structure, exactly as value axes are compiled.
//!
//! Steering holds (Principle 9, structural): no parameter and no value carries a
//! sophistication or advancement field, and none can be added without it being data the
//! Steering Audit reads; the sampler is gloss-blind (a value's label never enters a draw,
//! proven by test); relabelling value ids permutes the tilted weights exactly (the
//! typology-permutation invariant, proven by test); and harmony is a tendency, never a
//! correctness rule (with the disharmony gate at one the tilt is ignored entirely, proven
//! bit-identical to sampling under an empty harmony model).
//!
//! The typological drift operator (the proposal's section G) is deferred behind this
//! static generator on the owner's sequencing call, alongside the deeper 33.4 generation
//! machinery it would move; the substrate it will act over is this registry.

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::value::GroundMetric;
use civsim_core::{DrawKey, Fixed, Phase};
use std::collections::BTreeMap;
use std::fmt;

/// A typological parameter: a registered dimension of variation. A registry id, never a
/// closed enum (R-LANG-TYPOLOGY).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypologyParamId(pub u32);

/// A value on a typological parameter. A registry id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TypologyValueId(pub u32);

/// One value of a typological parameter, with its etic gloss (a short descriptive label
/// for rendering, the same sanctioned hardcoding as the prime lemmas, design 33.2; it
/// never enters a draw, which the gloss-blindness test proves).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypologyValueDef {
    /// The value's registry id.
    pub id: TypologyValueId,
    /// The etic label, for example "OV" or "ergative-absolutive".
    pub gloss: String,
}

/// A typological parameter definition: its registered values and its place in the
/// canonical anchor-first sampling order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypologyParamDef {
    /// The parameter's registry id.
    pub id: TypologyParamId,
    /// The etic label, for example "order of object and verb".
    pub gloss: String,
    /// The registered values, kept sorted by id for a canonical walk.
    pub values: Vec<TypologyValueDef>,
    /// The anchor-first sampling order (data): lower draws earlier, ties broken by
    /// parameter id. A harmony bias may only condition on a parameter that draws earlier.
    pub sample_priority: u32,
    /// Provenance: the typological record this parameter is grounded in.
    pub source: String,
}

/// The open typological parameter registry, kept sorted by parameter id.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TypologyRegistry {
    params: Vec<TypologyParamDef>,
}

/// The most values one parameter may register, a Q32.32-style representability bound (it
/// keeps the tilted-weight total under the exact-draw ceiling), engine mechanics rather
/// than a reserved realism value. The widest WALS inventory shipped is seven.
pub const MAX_VALUES_PER_PARAM: usize = 64;

/// The largest prior count one value may carry, the same representability bound (a count
/// widened to Q32.32 bits must stay under the per-value tilt cap). WALS samples top out
/// near 1,500.
pub const MAX_PRIOR_COUNT: u32 = 1 << 24;

/// The per-value tilted-weight ceiling: a tilt product saturates here rather than
/// wrapping, and with at most [`MAX_VALUES_PER_PARAM`] values the total stays below
/// 2^62, so the pick draw is exact in u128. Engine mechanics, not a realism value.
const WEIGHT_CAP: u128 = 1 << 56;

impl TypologyRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        TypologyRegistry::default()
    }

    /// Register a parameter. Values are sorted by id and the parameter is inserted in id
    /// order, so registration order never shows in a walk (proven by test). Duplicates
    /// are surfaced by [`validate`], loud rather than silently overwritten.
    pub fn add_param(&mut self, mut p: TypologyParamDef) {
        p.values.sort_by_key(|v| v.id);
        self.params.push(p);
        self.params.sort_by_key(|p| p.id);
    }

    /// The registered parameters in id order.
    pub fn params(&self) -> &[TypologyParamDef] {
        &self.params
    }

    /// One parameter by id.
    pub fn param(&self, id: TypologyParamId) -> Option<&TypologyParamDef> {
        self.params.iter().find(|p| p.id == id)
    }

    /// The canonical sampling order: indices into `params` sorted by
    /// (`sample_priority`, id). The anchor draws first; ties break on id, so the order is
    /// total and a function of the data alone.
    pub fn sampling_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.params.len()).collect();
        order.sort_by_key(|&i| (self.params[i].sample_priority, self.params[i].id));
        order
    }
}

/// One parameter's prior rows (value-id-sorted counts) with their provenance citation.
type PriorEntry = (Vec<(TypologyValueId, u32)>, String);

/// The per-parameter prior: integer language counts per value (the raw typological
/// record, so no normalisation division enters the draw), with a provenance citation.
/// Every value of the parameter must carry a row, zero allowed but explicit, so a typo
/// cannot silently zero a value out (a missing row is a load error).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TypologyPrior {
    rows: BTreeMap<TypologyParamId, PriorEntry>,
}

impl TypologyPrior {
    /// An empty prior.
    pub fn new() -> Self {
        TypologyPrior::default()
    }

    /// Set one parameter's counts (sorted by value id on insert) and their provenance.
    pub fn set(
        &mut self,
        param: TypologyParamId,
        mut counts: Vec<(TypologyValueId, u32)>,
        source: impl Into<String>,
    ) {
        counts.sort_by_key(|&(v, _)| v);
        self.rows.insert(param, (counts, source.into()));
    }

    /// One parameter's counts, sorted by value id.
    pub fn counts(&self, param: TypologyParamId) -> Option<&[(TypologyValueId, u32)]> {
        self.rows.get(&param).map(|(c, _)| c.as_slice())
    }

    /// One parameter's provenance.
    pub fn source(&self, param: TypologyParamId) -> Option<&str> {
        self.rows.get(&param).map(|(_, s)| s.as_str())
    }

    /// The parameters this prior covers, in id order.
    pub fn params(&self) -> impl Iterator<Item = TypologyParamId> + '_ {
        self.rows.keys().copied()
    }
}

/// One directional harmonic bias: given that `given_param` drew `given_value`, the prior
/// mass of `then_value` on `then_param` is multiplied by `weight`. The weight is a
/// reserved tier read from the calibration manifest (the typological record reports
/// proportions of genera, not coefficients, so the honest encoding is an ordinal tier
/// whose numeric value is the owner's), and the row carries the proportion as provenance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HarmonyBias {
    /// The conditioning parameter (must draw earlier in the canonical order).
    pub given_param: TypologyParamId,
    /// The conditioning value.
    pub given_value: TypologyValueId,
    /// The conditioned parameter.
    pub then_param: TypologyParamId,
    /// The value whose prior mass the bias multiplies.
    pub then_value: TypologyValueId,
    /// The multiplicative tilt (above one favours; validated positive).
    pub weight: Fixed,
    /// Provenance: the correlation record this row encodes.
    pub source: String,
}

/// The harmony model: the directional biases, kept in a canonical order. A
/// non-correlating pair carries no row; the sampler then draws that parameter from its
/// own marginal, the negative result the typological record shows.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct HarmonyModel {
    biases: Vec<HarmonyBias>,
}

impl HarmonyModel {
    /// An empty model (every parameter draws from its marginal).
    pub fn new() -> Self {
        HarmonyModel::default()
    }

    /// Add one bias row, kept sorted by (then, given) so any walk is canonical.
    pub fn add(&mut self, b: HarmonyBias) {
        self.biases.push(b);
        self.biases
            .sort_by_key(|b| (b.then_param, b.then_value, b.given_param, b.given_value));
    }

    /// The rows, in canonical order.
    pub fn biases(&self) -> &[HarmonyBias] {
        &self.biases
    }
}

/// A culture's grammar: one drawn value per registered parameter, sorted by parameter id.
/// The replacement for the design's closed `GrammarParams` (33.4).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TypologyProfile {
    values: Vec<(TypologyParamId, TypologyValueId)>,
}

impl TypologyProfile {
    /// A profile from pairs, sorted by parameter id.
    pub fn new(mut values: Vec<(TypologyParamId, TypologyValueId)>) -> Self {
        values.sort_by_key(|&(p, _)| p);
        TypologyProfile { values }
    }

    /// The drawn value on one parameter.
    pub fn get(&self, param: TypologyParamId) -> Option<TypologyValueId> {
        self.values
            .binary_search_by_key(&param, |&(p, _)| p)
            .ok()
            .map(|i| self.values[i].1)
    }

    /// The pairs in parameter-id order.
    pub fn entries(&self) -> &[(TypologyParamId, TypologyValueId)] {
        &self.values
    }

    /// Whether the profile is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// The number of parameters with a drawn value.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

/// What can go wrong loading or sampling the substrate. Loud, never a silent skip.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TypologyError {
    /// A duplicate parameter or value id.
    Duplicate(String),
    /// A reference names a parameter that does not exist.
    UnknownParam(String),
    /// A reference names a value its parameter does not carry.
    UnknownValue(String),
    /// A parameter has no values, or more than the representability bound.
    BadValueSet(String),
    /// A prior is missing, does not cover every value exactly once, has a zero total, or
    /// carries a count past the representability bound.
    BadPrior(String),
    /// A bias conditions on a parameter that draws at or after its target (the
    /// conditional order would be circular or undefined), targets itself, is a duplicate
    /// row, or carries a non-positive weight.
    BadBias(String),
}

impl fmt::Display for TypologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypologyError::Duplicate(m) => write!(f, "duplicate typology id: {m}"),
            TypologyError::UnknownParam(m) => write!(f, "unknown typology parameter: {m}"),
            TypologyError::UnknownValue(m) => write!(f, "unknown typology value: {m}"),
            TypologyError::BadValueSet(m) => write!(f, "bad typology value set: {m}"),
            TypologyError::BadPrior(m) => write!(f, "bad typology prior: {m}"),
            TypologyError::BadBias(m) => write!(f, "bad harmony bias: {m}"),
        }
    }
}

impl std::error::Error for TypologyError {}

/// Validate the registry, prior, and harmony model together, the load-time gate: every
/// cross-reference resolves, every prior covers its parameter exactly, and every bias
/// conditions strictly earlier in the canonical sampling order (a forward or circular
/// condition is an error, never a silent no-op).
pub fn validate(
    registry: &TypologyRegistry,
    prior: &TypologyPrior,
    harmony: &HarmonyModel,
) -> Result<(), TypologyError> {
    // Parameters: unique ids, value sets unique and within the representability bound.
    for w in registry.params.windows(2) {
        if w[0].id == w[1].id {
            return Err(TypologyError::Duplicate(format!("parameter {:?}", w[0].id)));
        }
    }
    for p in &registry.params {
        if p.values.is_empty() || p.values.len() > MAX_VALUES_PER_PARAM {
            return Err(TypologyError::BadValueSet(format!(
                "parameter {:?} has {} values (must be 1..={MAX_VALUES_PER_PARAM})",
                p.id,
                p.values.len()
            )));
        }
        for w in p.values.windows(2) {
            if w[0].id == w[1].id {
                return Err(TypologyError::Duplicate(format!(
                    "value {:?} on parameter {:?}",
                    w[0].id, p.id
                )));
            }
        }
    }
    // Priors: present for every parameter, covering every value exactly once, a positive
    // total, counts within the representability bound.
    for p in &registry.params {
        let counts = prior
            .counts(p.id)
            .ok_or_else(|| TypologyError::BadPrior(format!("parameter {:?} has no prior", p.id)))?;
        if counts.len() != p.values.len()
            || counts
                .iter()
                .zip(p.values.iter())
                .any(|(&(cv, _), pv)| cv != pv.id)
        {
            return Err(TypologyError::BadPrior(format!(
                "prior for parameter {:?} must cover each of its values exactly once",
                p.id
            )));
        }
        if counts.iter().any(|&(_, c)| c > MAX_PRIOR_COUNT) {
            return Err(TypologyError::BadPrior(format!(
                "a count on parameter {:?} exceeds the representability bound",
                p.id
            )));
        }
        if counts.iter().all(|&(_, c)| c == 0) {
            return Err(TypologyError::BadPrior(format!(
                "prior for parameter {:?} has a zero total",
                p.id
            )));
        }
    }
    for pid in prior.params() {
        if registry.param(pid).is_none() {
            return Err(TypologyError::UnknownParam(format!(
                "prior references {pid:?}"
            )));
        }
    }
    // Biases: references resolve, the condition draws strictly earlier, weights positive,
    // no self-condition, no duplicate row.
    let order = registry.sampling_order();
    let position: BTreeMap<TypologyParamId, usize> = order
        .iter()
        .enumerate()
        .map(|(pos, &i)| (registry.params[i].id, pos))
        .collect();
    for w in harmony.biases.windows(2) {
        let key = |b: &HarmonyBias| (b.given_param, b.given_value, b.then_param, b.then_value);
        if key(&w[0]) == key(&w[1]) {
            return Err(TypologyError::BadBias(format!(
                "duplicate bias {:?}/{:?} -> {:?}/{:?}",
                w[0].given_param, w[0].given_value, w[0].then_param, w[0].then_value
            )));
        }
    }
    for b in &harmony.biases {
        let given = registry.param(b.given_param).ok_or_else(|| {
            TypologyError::UnknownParam(format!("bias given {:?}", b.given_param))
        })?;
        let then = registry
            .param(b.then_param)
            .ok_or_else(|| TypologyError::UnknownParam(format!("bias then {:?}", b.then_param)))?;
        if !given.values.iter().any(|v| v.id == b.given_value) {
            return Err(TypologyError::UnknownValue(format!(
                "bias given value {:?} on {:?}",
                b.given_value, b.given_param
            )));
        }
        if !then.values.iter().any(|v| v.id == b.then_value) {
            return Err(TypologyError::UnknownValue(format!(
                "bias then value {:?} on {:?}",
                b.then_value, b.then_param
            )));
        }
        if b.given_param == b.then_param {
            return Err(TypologyError::BadBias(format!(
                "bias on {:?} conditions on itself",
                b.then_param
            )));
        }
        if position[&b.given_param] >= position[&b.then_param] {
            return Err(TypologyError::BadBias(format!(
                "bias {:?} -> {:?} conditions on a parameter that draws at or after its target",
                b.given_param, b.then_param
            )));
        }
        if b.weight <= Fixed::ZERO {
            return Err(TypologyError::BadBias(format!(
                "bias {:?} -> {:?} has a non-positive weight",
                b.given_param, b.then_param
            )));
        }
    }
    Ok(())
}

/// The tilted per-value weights for one parameter, given the values already drawn this
/// pass: each value's prior count widened to Q32.32 bits, multiplied by the weight of
/// every applicable bias, saturating at the representability cap. Exposed as a pure
/// function so the permutation invariant is testable bit-exactly. The caller has
/// validated; an unresolved reference here reads as an untouched weight.
pub fn tilted_weights(
    prior_counts: &[(TypologyValueId, u32)],
    param: TypologyParamId,
    drawn: &[(TypologyParamId, TypologyValueId)],
    harmony: &HarmonyModel,
) -> Vec<(TypologyValueId, u128)> {
    prior_counts
        .iter()
        .map(|&(v, c)| {
            // Q32.32: the integer count as fixed-point bits.
            let mut w = (c as u128) << 32;
            for b in harmony.biases() {
                if b.then_param == param
                    && b.then_value == v
                    && drawn
                        .iter()
                        .any(|&(dp, dv)| dp == b.given_param && dv == b.given_value)
                {
                    // Fixed multiply in u128: (w * bits) >> 32, saturating at the cap.
                    w = w
                        .checked_mul(b.weight.to_bits() as u128)
                        .map(|x| x >> 32)
                        .unwrap_or(WEIGHT_CAP)
                        .min(WEIGHT_CAP);
                }
            }
            (v, w.min(WEIGHT_CAP))
        })
        .collect()
}

/// Sample one culture's typology profile: the seeded, deterministic pass of the scoped
/// proposal's section D. Walks the parameters in the canonical anchor-first order; for
/// each, draws the disharmony gate then the value, both under
/// `DrawKey::entity(culture, tick, Phase::LANG_TYPOLOGY)` with the parameter's canonical
/// position as the slot, so every draw coordinate is canonical and camera-free (33.10,
/// R-RNG-COORD). With the gate open (disharmonic) the parameter draws from its untilted
/// marginal, so a disharmonic language is rarer, never unreachable.
pub fn sample_profile(
    registry: &TypologyRegistry,
    prior: &TypologyPrior,
    harmony: &HarmonyModel,
    disharmony: Fixed,
    master_seed: u64,
    culture: u64,
    tick: u64,
) -> Result<TypologyProfile, TypologyError> {
    validate(registry, prior, harmony)?;
    let order = registry.sampling_order();
    let mut drawn: Vec<(TypologyParamId, TypologyValueId)> = Vec::with_capacity(order.len());
    for (pos, &i) in order.iter().enumerate() {
        let p = &registry.params[i];
        let counts = prior.counts(p.id).expect("validated: prior present");
        let rng = DrawKey::entity(culture, tick, Phase::LANG_TYPOLOGY)
            .slot(pos as u32)
            .rng(master_seed);
        // Counter 0: the disharmony gate. Open means this parameter ignores the tilt and
        // draws from its own marginal, the reserved leak that keeps disharmony reachable.
        let disharmonic = rng.unit_fixed(0) < disharmony;
        let weights = if disharmonic {
            tilted_weights(counts, p.id, &[], &HarmonyModel::new())
        } else {
            tilted_weights(counts, p.id, &drawn, harmony)
        };
        // Counter 1: the value pick, a cumulative walk in value-id order over the exact
        // integer weights. The total stays below 2^62 by the caps, so the scaled draw is
        // exact in u128.
        let total: u128 = weights.iter().map(|&(_, w)| w).sum();
        debug_assert!(total > 0, "validated: positive prior total");
        let r = ((rng.at(1) as u128) * total) >> 64;
        let mut acc = 0u128;
        let mut chosen = weights.last().expect("validated: non-empty values").0;
        for &(v, w) in &weights {
            acc += w;
            if r < acc {
                chosen = v;
                break;
            }
        }
        drawn.push((p.id, chosen));
    }
    Ok(TypologyProfile::new(drawn))
}

/// A per-parameter structured distance: a compiled [`GroundMetric`] over the parameter's
/// values plus the value-id-to-index map it is compiled against, for parameters whose
/// values are not all equidistant (word orders differing in one constituent sit closer
/// than a full reversal). Compiled offline exactly as value axes are (Part 21).
#[derive(Clone, Debug)]
pub struct ValueMetric {
    /// The compiled metric.
    pub metric: GroundMetric,
    /// The index each value id occupies in the compiled metric.
    pub index_of: BTreeMap<TypologyValueId, usize>,
}

/// The 33.5 grammatical component over two profiles: for each parameter both profiles
/// carry and the reserved weights name, a categorical unit distance (zero when equal,
/// one when not), or the parameter's [`ValueMetric`] where one is supplied, scaled by the
/// parameter's reserved weight and accumulated by saturating sum, one narrowing, no
/// float. Parameters absent from either profile or from the weights contribute nothing;
/// the cross-language incommensurability of a wholly non-shared registry is the 33.5
/// language-distance layer's concern, not this component's.
pub fn typology_distance(
    a: &TypologyProfile,
    b: &TypologyProfile,
    weights: &BTreeMap<TypologyParamId, Fixed>,
    metrics: &BTreeMap<TypologyParamId, ValueMetric>,
) -> Fixed {
    let mut terms: Vec<Fixed> = Vec::new();
    for &(param, av) in a.entries() {
        let Some(bv) = b.get(param) else { continue };
        let Some(&w) = weights.get(&param) else {
            continue;
        };
        let unit = match metrics.get(&param) {
            Some(m) => match (m.index_of.get(&av), m.index_of.get(&bv)) {
                (Some(&i), Some(&j)) => m.metric.between(i, j),
                _ => {
                    if av == bv {
                        Fixed::ZERO
                    } else {
                        Fixed::ONE
                    }
                }
            },
            None => {
                if av == bv {
                    Fixed::ZERO
                } else {
                    Fixed::ONE
                }
            }
        };
        terms.push(w.checked_mul(unit).unwrap_or(Fixed::MAX));
    }
    Fixed::saturating_sum(terms)
}

/// The reserved typology calibrations, read fail-loud from the manifest (Principle 11):
/// the two harmony tier weights (the typological record reports proportions, so the tiers
/// are ordinal and their numeric tilts are the owner's) and the disharmony probability.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypologyParams {
    /// The strong-tier multiplicative tilt (the adposition-class correlations).
    pub harmony_strong: Fixed,
    /// The weak-tier multiplicative tilt (the genitive-class correlation).
    pub harmony_weak: Fixed,
    /// The probability a parameter ignores the tilt and draws from its marginal.
    pub disharmony: Fixed,
}

impl TypologyParams {
    /// Read the typology calibration from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(TypologyParams {
            harmony_strong: m.require_fixed("lang.typology_harmony_strong")?,
            harmony_weak: m.require_fixed("lang.typology_harmony_weak")?,
            disharmony: m.require_fixed("lang.typology_disharmony")?,
        })
    }
}

/// The cross-linguistic starting menu: the WALS parameter, value, and count record as
/// data, each count verified against wals.info (Dryer and Haspelmath eds., 2013) on
/// 2026-07-02. This is the human-grounded floor the registry opens from, a starting menu
/// and not a ceiling (the modality-registry stance), and the shared default prior a race
/// may override with its own shape over the same descriptive space. The two tier weights
/// are the caller's, read from the manifest (reserved); passing them as arguments keeps
/// the tier vocabulary out of the mechanism, so a future tier is data plus one more
/// manifest entry, never an enum case.
///
/// Encoded negative results, as load-bearing as the positive rows: the adjective (87A),
/// demonstrative (88A), and numeral (89A) orders carry NO harmony row, per Dryer 1992's
/// finding that they do not correlate with object-verb order; the morphological and
/// alignment parameters likewise draw from their own marginals. The six-way 81A dominant
/// word order is deliberately NOT seeded as an independent parameter: sampling it beside
/// the 83A anchor could draw contradictions (an SVO language classed OV), so the anchor
/// is 83A and the subject-order refinement waits for a coherence rule, recorded in the
/// proposal's decision batch.
pub fn wals_seed(strong: Fixed, weak: Fixed) -> (TypologyRegistry, TypologyPrior, HarmonyModel) {
    let mut reg = TypologyRegistry::new();
    let mut prior = TypologyPrior::new();
    let mut harmony = HarmonyModel::new();

    let add = |id: u32,
               gloss: &str,
               priority: u32,
               source: &str,
               values: &[(u32, &str, u32)],
               reg: &mut TypologyRegistry,
               prior: &mut TypologyPrior| {
        reg.add_param(TypologyParamDef {
            id: TypologyParamId(id),
            gloss: gloss.to_string(),
            values: values
                .iter()
                .map(|&(vid, g, _)| TypologyValueDef {
                    id: TypologyValueId(vid),
                    gloss: g.to_string(),
                })
                .collect(),
            sample_priority: priority,
            source: source.to_string(),
        });
        prior.set(
            TypologyParamId(id),
            values
                .iter()
                .map(|&(vid, _, c)| (TypologyValueId(vid), c))
                .collect(),
            source,
        );
    };

    // The anchor: order of object and verb (the dimension every Dryer correlation is
    // stated against; the reference dimension, not a claimed cause).
    add(
        0,
        "order of object and verb",
        0,
        "WALS 83A (Dryer 2013), 1518 languages: OV 712, VO 705, no dominant 101",
        &[
            (0, "OV", 712),
            (1, "VO", 705),
            (2, "no dominant order", 101),
        ],
        &mut reg,
        &mut prior,
    );
    // The strongly correlated dependents.
    add(
        1,
        "order of adposition and noun phrase",
        1,
        "WALS 85A (Dryer 2013), 1184 languages: postpositions 577, prepositions 511, \
         inpositions 8, no dominant 58, none 30",
        &[
            (0, "postpositions", 577),
            (1, "prepositions", 511),
            (2, "inpositions", 8),
            (3, "no dominant order", 58),
            (4, "no adpositions", 30),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        2,
        "order of genitive and noun",
        1,
        "WALS 86A (Dryer 2013), 1249 languages: GenN 685, NGen 468, no dominant 96",
        &[
            (0, "genitive-noun", 685),
            (1, "noun-genitive", 468),
            (2, "no dominant order", 96),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        3,
        "order of relative clause and noun",
        1,
        "WALS 90A (Dryer 2013), 824 languages: NRel 579, RelN 141, internally headed 24, \
         correlative 7, adjoined 8, doubly headed 1, mixed 64",
        &[
            (0, "noun-relative clause", 579),
            (1, "relative clause-noun", 141),
            (2, "internally headed", 24),
            (3, "correlative", 7),
            (4, "adjoined", 8),
            (5, "doubly headed", 1),
            (6, "mixed", 64),
        ],
        &mut reg,
        &mut prior,
    );
    // The non-correlating orders (Dryer 1992's negative results): no harmony rows.
    add(
        4,
        "order of adjective and noun",
        2,
        "WALS 87A (Dryer 2013), 1367 languages: AdjN 373, NAdj 879, no dominant 110, \
         internally-headed-relative-only 5; no object-verb correlation (Dryer 1992)",
        &[
            (0, "adjective-noun", 373),
            (1, "noun-adjective", 879),
            (2, "no dominant order", 110),
            (3, "only internally-headed relative clauses", 5),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        5,
        "order of demonstrative and noun",
        2,
        "WALS 88A (Dryer 2013), 1225 languages: DemN 542, NDem 562, prefix 9, suffix 28, \
         both 17, mixed 67; no object-verb correlation (Dryer 1992)",
        &[
            (0, "demonstrative-noun", 542),
            (1, "noun-demonstrative", 562),
            (2, "demonstrative prefix", 9),
            (3, "demonstrative suffix", 28),
            (4, "before and after", 17),
            (5, "mixed", 67),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        6,
        "order of numeral and noun",
        2,
        "WALS 89A (Dryer 2013), 1154 languages: NumN 479, NNum 608, no dominant 65, \
         verb-modifying only 2; no object-verb correlation (Dryer 1992)",
        &[
            (0, "numeral-noun", 479),
            (1, "noun-numeral", 608),
            (2, "no dominant order", 65),
            (3, "numeral only modifies verb", 2),
        ],
        &mut reg,
        &mut prior,
    );
    // The three morphological axes (the traditional isolating/agglutinating/fusional/
    // polysynthetic labels recovered as regions of this space, not an enum).
    add(
        7,
        "fusion of inflectional formatives",
        3,
        "WALS 20A (Bickel and Nichols 2013), 165 languages: exclusively concatenative 125, \
         exclusively isolating 16, exclusively tonal 3, tonal/isolating 1, \
         tonal/concatenative 2, ablaut/concatenative 5, isolating/concatenative 13",
        &[
            (0, "exclusively concatenative", 125),
            (1, "exclusively isolating", 16),
            (2, "exclusively tonal", 3),
            (3, "tonal/isolating", 1),
            (4, "tonal/concatenative", 2),
            (5, "ablaut/concatenative", 5),
            (6, "isolating/concatenative", 13),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        8,
        "exponence of inflectional formatives",
        3,
        "WALS 21A (Bickel and Nichols 2013), 162 languages: monoexponential case 71, \
         case+number 8, case+referentiality 6, case+TAM 2, no case 75",
        &[
            (0, "monoexponential case", 71),
            (1, "case and number", 8),
            (2, "case and referentiality", 6),
            (3, "case and TAM", 2),
            (4, "no case", 75),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        9,
        "inflectional synthesis of the verb",
        3,
        "WALS 22A (Bickel and Nichols 2013), 145 languages, categories per word: 0-1: 5, \
         2-3: 24, 4-5: 52, 6-7: 31, 8-9: 24, 10-11: 7, 12-13: 2 (a descriptive local \
         count, never a ranking)",
        &[
            (0, "0-1 categories", 5),
            (1, "2-3 categories", 24),
            (2, "4-5 categories", 52),
            (3, "6-7 categories", 31),
            (4, "8-9 categories", 24),
            (5, "10-11 categories", 7),
            (6, "12-13 categories", 2),
        ],
        &mut reg,
        &mut prior,
    );
    // Alignment per locus (a split-ergative language aligns each locus its own way, so
    // one global alignment enum would misstate the record).
    add(
        10,
        "alignment of case marking of full noun phrases",
        3,
        "WALS 98A (Comrie 2013), 190 languages: neutral 98, nominative-accusative standard 46, \
         marked nominative 6, ergative-absolutive 32, tripartite 4, active-inactive 4",
        &[
            (0, "neutral", 98),
            (1, "nominative-accusative (standard)", 46),
            (2, "nominative-accusative (marked nominative)", 6),
            (3, "ergative-absolutive", 32),
            (4, "tripartite", 4),
            (5, "active-inactive", 4),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        11,
        "alignment of case marking of pronouns",
        3,
        "WALS 99A (Comrie 2013), 172 languages: neutral 79, nominative-accusative standard 61, \
         marked nominative 3, ergative-absolutive 20, tripartite 3, active-inactive 3, none 3",
        &[
            (0, "neutral", 79),
            (1, "nominative-accusative (standard)", 61),
            (2, "nominative-accusative (marked nominative)", 3),
            (3, "ergative-absolutive", 20),
            (4, "tripartite", 3),
            (5, "active-inactive", 3),
            (6, "none", 3),
        ],
        &mut reg,
        &mut prior,
    );
    add(
        12,
        "alignment of verbal person marking",
        3,
        "WALS 100A (Siewierska 2013), 380 languages: neutral 84, accusative 212, ergative 19, \
         active 26, hierarchical 11, split 28",
        &[
            (0, "neutral", 84),
            (1, "accusative", 212),
            (2, "ergative", 19),
            (3, "active", 26),
            (4, "hierarchical", 11),
            (5, "split", 28),
        ],
        &mut reg,
        &mut prior,
    );

    // The harmony rows. Strong: the adposition pairs both ways and the VO relative-clause
    // pair. Weak: the genitive pair, OV side only (the VO side shows no preference).
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(0), // OV
        then_param: TypologyParamId(1),
        then_value: TypologyValueId(0), // postpositions
        weight: strong,
        source: "WALS 95A: 472 OV-postpositional against 14 OV-prepositional".to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(1), // VO
        then_param: TypologyParamId(1),
        then_value: TypologyValueId(1), // prepositions
        weight: strong,
        source: "WALS 95A: 456 VO-prepositional against 42 VO-postpositional".to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(1), // VO
        then_param: TypologyParamId(3),
        then_value: TypologyValueId(0), // noun-relative clause
        weight: strong,
        source: "Dryer 1992: VO languages are overwhelmingly NRel; prenominal relatives \
                 are essentially confined to OV languages"
            .to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(0), // OV
        then_param: TypologyParamId(2),
        then_value: TypologyValueId(0), // genitive-noun
        weight: weak,
        source: "Dryer 1992: 0.89 of OV genera are GenN against 0.45 of VO genera, the \
                 weak correlation, one side only"
            .to_string(),
    });

    (reg, prior, harmony)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dev fixtures: labelled tier weights and a disharmony rate for the harness, never
    // canon. The owner's set values reach the sampler through the calibration manifest.
    fn strong() -> Fixed {
        Fixed::from_int(64)
    }
    fn weak() -> Fixed {
        Fixed::from_int(4)
    }
    fn disharmony() -> Fixed {
        Fixed::from_ratio(1, 20)
    }

    fn seed() -> (TypologyRegistry, TypologyPrior, HarmonyModel) {
        wals_seed(strong(), weak())
    }

    #[test]
    fn wals_seed_validates_and_samples_every_parameter() {
        let (reg, prior, harmony) = seed();
        validate(&reg, &prior, &harmony).expect("the shipped seed validates");
        let p = sample_profile(&reg, &prior, &harmony, disharmony(), 0xC17, 7, 0).expect("samples");
        assert_eq!(p.len(), reg.params().len(), "every parameter drew a value");
        for def in reg.params() {
            let v = p.get(def.id).expect("value present");
            assert!(def.values.iter().any(|d| d.id == v), "value is registered");
        }
    }

    #[test]
    fn sampling_replays_bit_for_bit_and_keys_on_the_culture() {
        let (reg, prior, harmony) = seed();
        let a = sample_profile(&reg, &prior, &harmony, disharmony(), 42, 7, 3).unwrap();
        let b = sample_profile(&reg, &prior, &harmony, disharmony(), 42, 7, 3).unwrap();
        assert_eq!(a, b, "same coordinates, bit-identical profile");
        let mut any_differs = false;
        for culture in 0..50u64 {
            let c = sample_profile(&reg, &prior, &harmony, disharmony(), 42, culture, 3).unwrap();
            if c != a {
                any_differs = true;
                break;
            }
        }
        assert!(
            any_differs,
            "distinct cultures draw distinct grammars somewhere"
        );
    }

    #[test]
    fn registration_order_never_shows() {
        let (reg, prior, harmony) = seed();
        let mut reversed = TypologyRegistry::new();
        for p in reg.params().iter().rev() {
            reversed.add_param(p.clone());
        }
        assert_eq!(reg, reversed, "the registry walk is canonical");
        let a = sample_profile(&reg, &prior, &harmony, disharmony(), 9, 1, 0).unwrap();
        let b = sample_profile(&reversed, &prior, &harmony, disharmony(), 9, 1, 0).unwrap();
        assert_eq!(a, b, "insertion order never reaches a draw");
    }

    #[test]
    fn harmony_is_a_tendency_not_a_rule() {
        // With the disharmony gate at one every parameter ignores the tilt: bit-identical
        // to sampling under an empty harmony model. The tilt can bias, never dictate.
        let (reg, prior, harmony) = seed();
        for culture in 0..20u64 {
            let gated = sample_profile(&reg, &prior, &harmony, Fixed::ONE, 5, culture, 0).unwrap();
            let empty = sample_profile(
                &reg,
                &prior,
                &HarmonyModel::new(),
                Fixed::ONE,
                5,
                culture,
                0,
            )
            .unwrap();
            assert_eq!(gated, empty, "an open gate draws from the marginal");
        }
    }

    #[test]
    fn the_anchor_conditions_its_dependents() {
        // A two-parameter toy: anchor {0,1} even prior, dependent {0,1} even prior, one
        // enormous bias anchor=0 -> dependent=0, disharmony zero. The tilted weights are
        // exact, and over a fixed seed the conditional holds on every drawn culture.
        let mut reg = TypologyRegistry::new();
        reg.add_param(TypologyParamDef {
            id: TypologyParamId(0),
            gloss: "anchor".into(),
            values: vec![
                TypologyValueDef {
                    id: TypologyValueId(0),
                    gloss: "a0".into(),
                },
                TypologyValueDef {
                    id: TypologyValueId(1),
                    gloss: "a1".into(),
                },
            ],
            sample_priority: 0,
            source: "test fixture".into(),
        });
        reg.add_param(TypologyParamDef {
            id: TypologyParamId(1),
            gloss: "dependent".into(),
            values: vec![
                TypologyValueDef {
                    id: TypologyValueId(0),
                    gloss: "d0".into(),
                },
                TypologyValueDef {
                    id: TypologyValueId(1),
                    gloss: "d1".into(),
                },
            ],
            sample_priority: 1,
            source: "test fixture".into(),
        });
        let mut prior = TypologyPrior::new();
        prior.set(
            TypologyParamId(0),
            vec![(TypologyValueId(0), 1), (TypologyValueId(1), 1)],
            "test",
        );
        prior.set(
            TypologyParamId(1),
            vec![(TypologyValueId(0), 1), (TypologyValueId(1), 1)],
            "test",
        );
        let mut harmony = HarmonyModel::new();
        harmony.add(HarmonyBias {
            given_param: TypologyParamId(0),
            given_value: TypologyValueId(0),
            then_param: TypologyParamId(1),
            then_value: TypologyValueId(0),
            weight: Fixed::from_int(1 << 24),
            source: "test".into(),
        });
        // The tilted weights are exact: d0 carries count*(1<<24) in Q32.32 bits, d1 the
        // bare count.
        let w = tilted_weights(
            prior.counts(TypologyParamId(1)).unwrap(),
            TypologyParamId(1),
            &[(TypologyParamId(0), TypologyValueId(0))],
            &harmony,
        );
        assert_eq!(w[0], (TypologyValueId(0), 1u128 << 56));
        assert_eq!(w[1], (TypologyValueId(1), 1u128 << 32));
        // And on every culture this seed draws with the anchor at 0, the dependent is 0.
        let mut conditioned = 0;
        for culture in 0..64u64 {
            let p = sample_profile(&reg, &prior, &harmony, Fixed::ZERO, 77, culture, 0).unwrap();
            if p.get(TypologyParamId(0)) == Some(TypologyValueId(0)) {
                conditioned += 1;
                assert_eq!(
                    p.get(TypologyParamId(1)),
                    Some(TypologyValueId(0)),
                    "a 2^24 tilt binds the dependent on this fixed seed"
                );
            }
        }
        assert!(conditioned > 0, "the anchor drew 0 somewhere in the sweep");
    }

    #[test]
    fn value_relabelling_permutes_the_tilted_weights_exactly() {
        // The typology-permutation invariant (the modality-swap analogue): relabel the
        // dependent's value ids through a permutation, map the drawn condition and the
        // bias rows the same way, and the tilted weights map bit-exactly. No value is
        // privileged by its index.
        let (_reg, prior, harmony) = seed();
        let adposition = TypologyParamId(1);
        let counts = prior.counts(adposition).unwrap().to_vec();
        let drawn = [(TypologyParamId(0), TypologyValueId(0))];
        let base = tilted_weights(&counts, adposition, &drawn, &harmony);
        // The permutation: value id v -> (v + 1) mod 5 on the adposition parameter.
        let perm = |v: TypologyValueId| TypologyValueId((v.0 + 1) % 5);
        let permuted_counts: Vec<(TypologyValueId, u32)> = {
            let mut c: Vec<_> = counts.iter().map(|&(v, n)| (perm(v), n)).collect();
            c.sort_by_key(|&(v, _)| v);
            c
        };
        let mut permuted_harmony = HarmonyModel::new();
        for b in harmony.biases() {
            let mut nb = b.clone();
            if nb.then_param == adposition {
                nb.then_value = perm(nb.then_value);
            }
            permuted_harmony.add(nb);
        }
        let mapped = tilted_weights(&permuted_counts, adposition, &drawn, &permuted_harmony);
        for &(v, w) in &base {
            let target = perm(v);
            let found = mapped.iter().find(|&&(mv, _)| mv == target).unwrap().1;
            assert_eq!(found, w, "weight of {v:?} maps bit-exactly to {target:?}");
        }
    }

    #[test]
    fn glosses_and_sources_never_reach_a_draw() {
        // Two seeds differing only in every gloss and provenance string sample
        // bit-identically: the draw reads structure and counts, never labels.
        let (reg, prior, harmony) = seed();
        let mut relabelled = TypologyRegistry::new();
        for p in reg.params() {
            let mut q = p.clone();
            q.gloss = format!("param-{}", q.id.0);
            for v in &mut q.values {
                v.gloss = format!("value-{}", v.id.0);
            }
            q.source = "relabelled".into();
            relabelled.add_param(q);
        }
        let mut prior2 = TypologyPrior::new();
        for pid in prior.params() {
            prior2.set(pid, prior.counts(pid).unwrap().to_vec(), "relabelled");
        }
        let mut harmony2 = HarmonyModel::new();
        for b in harmony.biases() {
            let mut nb = b.clone();
            nb.source = "relabelled".into();
            harmony2.add(nb);
        }
        for culture in 0..10u64 {
            let a = sample_profile(&reg, &prior, &harmony, disharmony(), 3, culture, 1).unwrap();
            let b = sample_profile(&relabelled, &prior2, &harmony2, disharmony(), 3, culture, 1)
                .unwrap();
            assert_eq!(a, b, "labels are etic surface, never draw input");
        }
    }

    #[test]
    fn a_forward_conditioning_bias_is_a_load_error_not_a_silent_no_op() {
        let (reg, prior, mut harmony) = seed();
        // Adposition (priority 1) conditioning the anchor (priority 0): backwards.
        harmony.add(HarmonyBias {
            given_param: TypologyParamId(1),
            given_value: TypologyValueId(0),
            then_param: TypologyParamId(0),
            then_value: TypologyValueId(0),
            weight: Fixed::from_int(2),
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &harmony),
            Err(TypologyError::BadBias(_))
        ));
    }

    #[test]
    fn load_errors_are_loud() {
        let (reg, prior, harmony) = seed();
        // A prior that misses a value.
        let mut short = TypologyPrior::new();
        for pid in prior.params() {
            let mut counts = prior.counts(pid).unwrap().to_vec();
            if pid == TypologyParamId(0) {
                counts.pop();
            }
            short.set(pid, counts, "test");
        }
        assert!(matches!(
            validate(&reg, &short, &harmony),
            Err(TypologyError::BadPrior(_))
        ));
        // A bias naming a value its parameter does not carry.
        let mut bad_value = HarmonyModel::new();
        bad_value.add(HarmonyBias {
            given_param: TypologyParamId(0),
            given_value: TypologyValueId(9),
            then_param: TypologyParamId(1),
            then_value: TypologyValueId(0),
            weight: Fixed::from_int(2),
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &bad_value),
            Err(TypologyError::UnknownValue(_))
        ));
        // A self-conditioning bias.
        let mut self_bias = HarmonyModel::new();
        self_bias.add(HarmonyBias {
            given_param: TypologyParamId(1),
            given_value: TypologyValueId(0),
            then_param: TypologyParamId(1),
            then_value: TypologyValueId(1),
            weight: Fixed::from_int(2),
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &self_bias),
            Err(TypologyError::BadBias(_))
        ));
        // A duplicate parameter id.
        let mut dup = TypologyRegistry::new();
        let p = reg.params()[0].clone();
        dup.add_param(p.clone());
        dup.add_param(p);
        assert!(matches!(
            validate(&dup, &prior, &HarmonyModel::new()),
            Err(TypologyError::Duplicate(_))
        ));
    }

    #[test]
    fn distance_is_symmetric_categorical_and_weighted() {
        let (reg, prior, harmony) = seed();
        let a = sample_profile(&reg, &prior, &harmony, disharmony(), 11, 0, 0).unwrap();
        let mut weights = BTreeMap::new();
        for p in reg.params() {
            weights.insert(p.id, Fixed::from_ratio(1, 13));
        }
        let metrics = BTreeMap::new();
        assert_eq!(
            typology_distance(&a, &a, &weights, &metrics),
            Fixed::ZERO,
            "identical profiles are at distance zero"
        );
        let mut b_pairs = a.entries().to_vec();
        b_pairs[0].1 = if b_pairs[0].1 == TypologyValueId(0) {
            TypologyValueId(1)
        } else {
            TypologyValueId(0)
        };
        let b = TypologyProfile::new(b_pairs);
        let d_ab = typology_distance(&a, &b, &weights, &metrics);
        let d_ba = typology_distance(&b, &a, &weights, &metrics);
        assert_eq!(d_ab, d_ba, "symmetric under argument swap");
        assert_eq!(
            d_ab,
            Fixed::from_ratio(1, 13),
            "one categorical difference contributes exactly its weight"
        );
    }

    #[test]
    fn typology_params_read_fail_loud_from_the_manifest() {
        let toml = r#"
[[reserved]]
id = "lang.typology_harmony_strong"
basis = "test fixture"
status = "reserved"
value = ""
unit = "tilt"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        assert!(
            TypologyParams::from_manifest(&m).is_err(),
            "a reserved tier weight fails loud, never a fabricated default"
        );
    }
}
