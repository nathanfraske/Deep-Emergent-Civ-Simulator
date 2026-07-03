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
use crate::language::Linearization;
use crate::value::GroundMetric;
use civsim_core::{DrawKey, Fixed, Phase};
use civsim_physics::laws;
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

/// The normalized parse-cost ceiling passed to [`laws::parse_cost`]: parse cost is reported as a
/// FRACTION in `[0, 1]`, so the softmax temperature is the single scale of the derived tilt and no
/// second free magnitude hides in the cost cap. Engine mechanics, not a realism value.
const COST_MAX: Fixed = Fixed::ONE;

/// The representability ceiling on the derived tilt passed to [`laws::harmony_tilt`]: the saturating
/// exponential is bounded here so the tilt stays a small `Fixed`, well inside the per-value
/// [`WEIGHT_CAP`] that keeps the cumulative pick exact. Engine mechanics, not a realism value.
const TILT_MAX: Fixed = Fixed::from_int(1 << 20);

/// The parse-cost tilt parameters threaded to the sampler: the per-race working-memory capacity that
/// softens parse cost, the single reserved softmax temperature that scales the derived tilt, and the
/// two engine caps ([`COST_MAX`], [`TILT_MAX`]). Memory is per-race DATA (a race that invests in
/// working memory feels weaker harmony pressure, proven by test); the temperature is the one reserved
/// manifest scale (validated against the human WALS/Dryer row, not authored as tiers); the caps are
/// representability bounds. Bundled so the sampler signature stays readable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TiltParams {
    /// The parser's working-memory capacity (per-race data): softens the parse cost of a domain.
    pub memory_capacity: Fixed,
    /// The softmax temperature (the single reserved tilt scale): a smaller temperature bites harder.
    pub temperature: Fixed,
    /// The normalized parse-cost ceiling (engine mechanics; [`COST_MAX`]).
    pub cost_max: Fixed,
    /// The tilt representability ceiling (engine mechanics; [`TILT_MAX`]).
    pub tilt_max: Fixed,
}

impl TiltParams {
    /// The tilt parameters for a race with the given working-memory capacity and the reserved
    /// softmax temperature, filling the two engine caps so they stay out of every call site.
    pub fn new(memory_capacity: Fixed, temperature: Fixed) -> Self {
        TiltParams {
            memory_capacity,
            temperature,
            cost_max: COST_MAX,
            tilt_max: TILT_MAX,
        }
    }
}

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

/// One directional harmonic bias: given that `given_param` drew `given_value`, choosing
/// `then_value` on `then_param` keeps the branching direction consistent. The row no longer
/// carries an authored tilt; it carries a data-defined `structural_weight`, how many
/// linearization decisions the pairing constrains (its dependency-integration domain extent).
/// The multiplicative tilt DERIVES from that structural weight through the parse-cost floor
/// (`laws::parse_cost` then `laws::harmony_tilt`), so the strong-versus-weak distinction is DATA
/// (a larger structural weight for a pairing that constrains more decisions) rather than two
/// authored numbers. The tilt scale is the single reserved softmax temperature, validated against
/// the human WALS/Dryer proportions the row carries as provenance rather than authored from them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HarmonyBias {
    /// The conditioning parameter (must draw earlier in the canonical order).
    pub given_param: TypologyParamId,
    /// The conditioning value.
    pub given_value: TypologyValueId,
    /// The conditioned parameter.
    pub then_param: TypologyParamId,
    /// The harmonic (branching-consistent) value on the conditioned parameter.
    pub then_value: TypologyValueId,
    /// The structural weight: how many head-dependent linearization decisions this pairing
    /// constrains, as a dependency-integration domain extent (data, validated positive). A
    /// two-sided fully-phrasal pairing constrains more than a one-sided partly-non-phrasal one
    /// (Dryer's Branching Direction Theory), so strong-vs-weak is this magnitude, never a tilt.
    pub structural_weight: Fixed,
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
        if b.structural_weight <= Fixed::ZERO {
            return Err(TypologyError::BadBias(format!(
                "bias {:?} -> {:?} has a non-positive structural weight",
                b.given_param, b.then_param
            )));
        }
    }
    Ok(())
}

/// The dependency-integration domain extent one harmony bias contributes given the values already
/// drawn: its data-defined `structural_weight` (how many linearization decisions the pairing
/// constrains) when its conditioning value has been drawn, and zero otherwise. The scalar magnitude
/// the parse-cost floor reads. Direction-blind and label-blind: it reads the structural weight and
/// whether the condition is present, never which value id is head-initial, so the kernels it feeds
/// cannot privilege a direction (Principle 9).
fn linearization_domain(bias: &HarmonyBias, drawn: &[(TypologyParamId, TypologyValueId)]) -> Fixed {
    if drawn
        .iter()
        .any(|&(dp, dv)| dp == bias.given_param && dv == bias.given_value)
    {
        bias.structural_weight
    } else {
        Fixed::ZERO
    }
}

/// The derived multiplicative tilt for choosing `value` on `param`, composing the
/// [`linearization_domain`] fold through [`laws::parse_cost`] and [`laws::harmony_tilt`]. The
/// harmonic value's avoided dependency-integration cost is the ORDER-INDEPENDENT saturating sum of
/// the structural weights of every firing bias for `(param, value)`; that folded extent runs through
/// the softening parse-cost law and the softmax tilt law to a weight `>= ONE`. A value no bias
/// favours folds to a zero extent, a zero cost, and a tilt of exactly one (its bare marginal). This
/// is the mechanism that REPLACES the former authored per-bias tilt: the tilt derives from the
/// parse-cost floor, and strong-versus-weak lives in the structural weights (data).
fn derived_tilt(
    param: TypologyParamId,
    value: TypologyValueId,
    drawn: &[(TypologyParamId, TypologyValueId)],
    harmony: &HarmonyModel,
    tilt: &TiltParams,
) -> Fixed {
    let reduction = integrated_parse_cost(
        harmony
            .biases()
            .iter()
            .filter(|b| b.then_param == param && b.then_value == value)
            .map(|b| linearization_domain(b, drawn)),
        tilt.memory_capacity,
        tilt.cost_max,
    );
    laws::harmony_tilt(reduction, tilt.temperature, tilt.tilt_max)
}

/// The one integration semantics both the sampling tilt ([`derived_tilt`]) and the whole-grammar
/// cost ([`grammar_parse_cost`]) route through: the structural weights of the relevant biases are
/// summed FIRST (working memory holds them at once, a shared resource), then the softening
/// [`laws::parse_cost`] law is applied ONCE to that aggregate load. The alternative (apply the
/// nonlinear law per bias then sum) double-softens and makes the two consumers disagree when two
/// biases fire on one value, which is the defect this reconciles. Order-independent (the sum is a
/// saturating fold), so it is invariant to bias order.
fn integrated_parse_cost(
    weights: impl IntoIterator<Item = Fixed>,
    memory_capacity: Fixed,
    cost_max: Fixed,
) -> Fixed {
    let total = Fixed::saturating_sum(weights);
    laws::parse_cost(total, memory_capacity, cost_max)
}

/// The tilted per-value weights for one parameter, given the values already drawn this pass: each
/// value's prior count widened to Q32.32 bits, multiplied by its DERIVED parse-cost tilt (never a
/// stored coefficient), saturating at the representability cap. When `suppress` is set (a
/// simultaneous modality, or the disharmony gate open) the tilt is one for every value, so the
/// weights fall back to the untilted marginal. Exposed as a pure function so the permutation
/// invariant and the direction-neutrality invariant are testable bit-exactly. The caller has
/// validated; an unresolved reference here reads as an untouched weight.
pub fn tilted_weights(
    prior_counts: &[(TypologyValueId, u32)],
    param: TypologyParamId,
    drawn: &[(TypologyParamId, TypologyValueId)],
    harmony: &HarmonyModel,
    tilt: &TiltParams,
    suppress: bool,
) -> Vec<(TypologyValueId, u128)> {
    prior_counts
        .iter()
        .map(|&(v, c)| {
            // Q32.32: the integer count as fixed-point bits.
            let w = (c as u128) << 32;
            let t = if suppress {
                Fixed::ONE
            } else {
                derived_tilt(param, v, drawn, harmony, tilt)
            };
            // Fixed multiply in u128: (w * tilt_bits) >> 32, saturating at the cap. The derived tilt
            // is >= ONE, so an untouched value keeps its marginal (ONE.to_bits() == 1 << 32).
            let w = w
                .checked_mul(t.to_bits() as u128)
                .map(|x| x >> 32)
                .unwrap_or(WEIGHT_CAP)
                .min(WEIGHT_CAP);
            (v, w)
        })
        .collect()
}

/// The total dependency-integration parse cost of a candidate grammar: the ORDER-INDEPENDENT
/// saturating sum, over every harmony bias whose conditioning value the grammar drew, of the parse
/// cost the pairing incurs when the grammar VIOLATES it (the conditioned parameter took some value
/// other than the harmonic one), and zero when the grammar satisfies it. A fully harmonic grammar
/// violates no bias and costs zero; a mixed-branching grammar holds the structural weight of each
/// violated pairing long and pays for it. Direction-blind: the cost reads structural weights and the
/// equal-or-not of value ids, never which id is head-initial, so it cannot prefer one linear order.
pub fn grammar_parse_cost(
    profile: &TypologyProfile,
    harmony: &HarmonyModel,
    memory_capacity: Fixed,
    cost_max: Fixed,
) -> Fixed {
    // Route through the one integration semantics (sum the violated structural weights, then one
    // parse_cost), the same fold derived_tilt uses, so the sampling tilt and the whole-grammar cost
    // never disagree when two biases fire on one value.
    integrated_parse_cost(
        harmony.biases().iter().filter_map(|b| {
            match (profile.get(b.given_param), profile.get(b.then_param)) {
                (Some(gv), Some(tv)) if gv == b.given_value && tv != b.then_value => {
                    Some(b.structural_weight)
                }
                _ => None,
            }
        }),
        memory_capacity,
        cost_max,
    )
}

/// Sample one culture's typology profile: the seeded, deterministic pass of the scoped
/// proposal's section D. Walks the parameters in the canonical anchor-first order; for
/// each, draws the disharmony gate then the value, both under
/// `DrawKey::entity(culture, tick, Phase::LANG_TYPOLOGY)` with the parameter's canonical
/// position as the slot, so every draw coordinate is canonical and camera-free (33.10,
/// R-RNG-COORD). The word-order harmony tilt DERIVES from the parse-cost floor through
/// [`tilted_weights`] rather than from an authored coefficient. Two gates suppress it back to the
/// untilted marginal, on the same branch: the reserved disharmony gate (open with a small
/// probability so a disharmonic language stays reachable) and a `Linearization::Simultaneous`
/// modality (a modality with no linear word order for the tilt to act on). No new draw enters: the
/// modality gate is a deterministic conditional on the data flag.
// The nine arguments are the substrate (registry, prior, harmony), the reserved calibration (tilt,
// disharmony), the modality flag, and the canonical draw coordinate (seed, culture, tick); each is a
// distinct axis of the pass, so bundling would only hide the coordinate. The laws.rs kernels take the
// same `#[allow]` for the same reason.
#[allow(clippy::too_many_arguments)]
pub fn sample_profile(
    registry: &TypologyRegistry,
    prior: &TypologyPrior,
    harmony: &HarmonyModel,
    tilt: &TiltParams,
    linearization: Linearization,
    disharmony: Fixed,
    master_seed: u64,
    culture: u64,
    tick: u64,
) -> Result<TypologyProfile, TypologyError> {
    validate(registry, prior, harmony)?;
    // A simultaneous modality has no linear word order, so the harmony tilt has nothing to act on:
    // it is suppressed for every parameter with no extra draw (a deterministic conditional on data).
    let simultaneous = matches!(linearization, Linearization::Simultaneous);
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
        // Suppress the tilt when the gate is open OR the modality is simultaneous: both fall back to
        // the untilted marginal, the one branch.
        let suppress = disharmonic || simultaneous;
        let weights = tilted_weights(counts, p.id, &drawn, harmony, tilt, suppress);
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

/// The information weights over the typology parameters, a drop-in for the `weights` argument of
/// [`typology_distance`] (design Part 33.5). Each parameter's weight is the order-2 diversity of
/// its prior integer language counts, the Hill number of order two `N^2 / sum(c_i^2)` (the inverse
/// Simpson concentration, the effective number of distinct values the parameter takes across the
/// grounding sample). A parameter whose languages spread evenly over many values carries more
/// information about which language is which and so weighs more in the grammatical distance; a
/// parameter dominated by one value carries little and weighs near one. The weights DERIVE from the
/// prior counts (data, Principle 11), so `lang.typology_distance_weights` is no longer an authored
/// per-parameter table: two worlds with different priors weigh their parameters differently from
/// the same rule, and no human weighting is baked in.
///
/// Deterministic and integer-exact: `N` and `sum(c_i^2)` accumulate in `u128` (the counts are
/// bounded by [`MAX_PRIOR_COUNT`] and there are at most [`MAX_VALUES_PER_PARAM`] of them, so neither
/// overflows), and the weight is `(N^2 << 32) / sum(c_i^2)` read back as `Fixed` bits, so the map is
/// bit-identical across machines. A parameter whose prior is missing, or whose counts total zero,
/// contributes no weight (it is omitted from the map, exactly as [`typology_distance`] treats an
/// absent weight), so the mechanism never divides by zero or fabricates a weight for a parameter it
/// has no evidence on. The registry gives the canonical parameter walk; the output is a `BTreeMap`,
/// so its order is parameter-id order regardless.
pub fn information_weights(
    registry: &TypologyRegistry,
    prior: &TypologyPrior,
) -> BTreeMap<TypologyParamId, Fixed> {
    let mut weights = BTreeMap::new();
    for param in registry.params() {
        let Some(counts) = prior.counts(param.id) else {
            continue;
        };
        let mut n: u128 = 0;
        let mut sum_sq: u128 = 0;
        for &(_value, c) in counts {
            let c = c as u128;
            n += c;
            sum_sq += c * c;
        }
        if sum_sq == 0 {
            // No evidence on this parameter (every count zero): no weight, never a fabricated one.
            continue;
        }
        // Order-2 diversity N^2 / sum(c_i^2), read as Q32.32 bits: (N^2 << 32) / sum_sq.
        let bits = ((n * n) << 32) / sum_sq;
        weights.insert(param.id, Fixed::from_bits(bits as i64));
    }
    weights
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

/// The reserved typology calibrations, read fail-loud from the manifest (Principle 11). The two
/// former harmony tier tilts are retired: the harmony tilt now DERIVES from the parse-cost floor over
/// the per-pair structural weights (data), so the single reserved tilt scale is the softmax
/// temperature. The disharmony probability is kept (the gate that keeps a disharmonic language
/// reachable). To build a [`TiltParams`], the caller pairs this manifest temperature with a race's
/// own working-memory capacity (per-race data), never a manifest scalar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypologyParams {
    /// The softmax temperature: the single reserved scale of the derived harmony tilt.
    pub temperature: Fixed,
    /// The probability a parameter ignores the tilt and draws from its marginal.
    pub disharmony: Fixed,
}

impl TypologyParams {
    /// Read the typology calibration from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(TypologyParams {
            temperature: m.require_fixed("lang.typology_temperature")?,
            disharmony: m.require_fixed("lang.typology_disharmony")?,
        })
    }

    /// The tilt parameters for a race with the given working-memory capacity, pairing the reserved
    /// softmax temperature with that per-race datum.
    pub fn tilt_params(&self, memory_capacity: Fixed) -> TiltParams {
        TiltParams::new(memory_capacity, self.temperature)
    }
}

/// The cross-linguistic starting menu: the WALS parameter, value, and count record as
/// data, each count verified against wals.info (Dryer and Haspelmath eds., 2013) on
/// 2026-07-02. This is the human-grounded floor the registry opens from, a starting menu
/// and not a ceiling (the modality-registry stance), and the shared default prior a race
/// may override with its own shape over the same descriptive space. The harmony rows carry a
/// data-defined `structural_weight` (how many linearization decisions the pairing constrains) rather
/// than an authored tilt: strong adposition and relative-clause pairs constrain more (two-sided,
/// fully phrasal) than the weak one-sided genitive pair (Dryer's Branching Direction Theory), so
/// strong-vs-weak is this magnitude in data. The tilt itself DERIVES from the parse-cost floor with
/// the single reserved softmax temperature, validated against the WALS/Dryer proportions the rows
/// cite as provenance rather than authored from them (`wals_seed` takes no tilt arguments).
///
/// Encoded negative results, as load-bearing as the positive rows: the adjective (87A),
/// demonstrative (88A), and numeral (89A) orders carry NO harmony row, per Dryer 1992's
/// finding that they do not correlate with object-verb order; the morphological and
/// alignment parameters likewise draw from their own marginals. The six-way 81A dominant
/// word order is deliberately NOT seeded as an independent parameter: sampling it beside
/// the 83A anchor could draw contradictions (an SVO language classed OV), so the anchor
/// is 83A and the subject-order refinement waits for a coherence rule, recorded in the
/// proposal's decision batch.
pub fn wals_seed() -> (TypologyRegistry, TypologyPrior, HarmonyModel) {
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

    // The harmony rows. The structural weight is DATA: how many head-dependent linearization
    // decisions the pairing constrains, grounded in Dryer's Branching Direction Theory. The
    // adposition pairs (both ways) and the VO relative-clause pair are two-sided, fully-phrasal
    // head-complement correlations, so they constrain more (two decisions each); the genitive pair
    // is one-sided and often non-phrasal, so it constrains fewer (one). Strong-vs-weak is this
    // magnitude, and the tilt derives from it through the parse-cost floor at the reserved
    // temperature (validated against the cited proportions, not authored from them).
    const STRONG_WEIGHT: Fixed = Fixed::from_int(2);
    const WEAK_WEIGHT: Fixed = Fixed::from_int(1);
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(0), // OV
        then_param: TypologyParamId(1),
        then_value: TypologyValueId(0), // postpositions
        structural_weight: STRONG_WEIGHT,
        source: "WALS 95A: 472 OV-postpositional against 14 OV-prepositional; two-sided \
                 fully-phrasal head-complement pairing (structural weight 2)"
            .to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(1), // VO
        then_param: TypologyParamId(1),
        then_value: TypologyValueId(1), // prepositions
        structural_weight: STRONG_WEIGHT,
        source: "WALS 95A: 456 VO-prepositional against 42 VO-postpositional; two-sided \
                 fully-phrasal head-complement pairing (structural weight 2)"
            .to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(1), // VO
        then_param: TypologyParamId(3),
        then_value: TypologyValueId(0), // noun-relative clause
        structural_weight: STRONG_WEIGHT,
        source: "Dryer 1992: VO languages are overwhelmingly NRel; prenominal relatives \
                 are essentially confined to OV languages; a maximally-phrasal clause \
                 pairing (structural weight 2)"
            .to_string(),
    });
    harmony.add(HarmonyBias {
        given_param: TypologyParamId(0),
        given_value: TypologyValueId(0), // OV
        then_param: TypologyParamId(2),
        then_value: TypologyValueId(0), // genitive-noun
        structural_weight: WEAK_WEIGHT,
        source: "Dryer 1992: 0.89 of OV genera are GenN against 0.45 of VO genera, the \
                 weak correlation, one side only; a one-sided partly-non-phrasal pairing \
                 (structural weight 1)"
            .to_string(),
    });

    (reg, prior, harmony)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Canonical WALS-seed ids, for readable tests.
    const P_OV: TypologyParamId = TypologyParamId(0); // order of object and verb (the anchor)
    const P_ADP: TypologyParamId = TypologyParamId(1); // adposition order
    const P_GEN: TypologyParamId = TypologyParamId(2); // genitive order
    const P_REL: TypologyParamId = TypologyParamId(3); // relative-clause order
    const OV: TypologyValueId = TypologyValueId(0);
    const VO: TypologyValueId = TypologyValueId(1);
    const POST: TypologyValueId = TypologyValueId(0); // postpositions (OV-harmonic)
    const PREP: TypologyValueId = TypologyValueId(1); // prepositions
    const GENN: TypologyValueId = TypologyValueId(0); // genitive-noun (OV-harmonic)
    const NGEN: TypologyValueId = TypologyValueId(1); // noun-genitive
    const NREL: TypologyValueId = TypologyValueId(0); // noun-relative clause (VO-harmonic)

    // Dev fixtures, never canon: the human race's working-memory capacity paired with the reserved
    // softmax temperature, the pair the human-row calibration validates against. The owner's set
    // temperature reaches the sampler through the calibration manifest; memory is per-race data.
    fn tilt() -> TiltParams {
        TiltParams::new(Fixed::from_ratio(19, 4), Fixed::from_ratio(92, 1000))
    }
    fn disharmony() -> Fixed {
        Fixed::from_ratio(1, 20)
    }
    fn seq() -> Linearization {
        Linearization::Sequential
    }

    fn seed() -> (TypologyRegistry, TypologyPrior, HarmonyModel) {
        wals_seed()
    }

    /// The tilted proportion of `value` on `param` given `drawn`: the exact, deterministic
    /// probability the cumulative pick realizes (weight over total), read straight off
    /// `tilted_weights`. The measure the human-row calibration and the direction-neutrality
    /// invariant both assert against.
    fn tilted_proportion(
        prior: &TypologyPrior,
        harmony: &HarmonyModel,
        tilt: &TiltParams,
        param: TypologyParamId,
        drawn: &[(TypologyParamId, TypologyValueId)],
        value: TypologyValueId,
    ) -> f64 {
        let counts = prior.counts(param).unwrap();
        let w = tilted_weights(counts, param, drawn, harmony, tilt, false);
        let total: u128 = w.iter().map(|&(_, x)| x).sum();
        let target: u128 = w.iter().find(|&&(v, _)| v == value).unwrap().1;
        target as f64 / total as f64
    }

    #[test]
    fn wals_seed_validates_and_samples_every_parameter() {
        let (reg, prior, harmony) = seed();
        validate(&reg, &prior, &harmony).expect("the shipped seed validates");
        let p = sample_profile(
            &reg,
            &prior,
            &harmony,
            &tilt(),
            seq(),
            disharmony(),
            0xC17,
            7,
            0,
        )
        .expect("samples");
        assert_eq!(p.len(), reg.params().len(), "every parameter drew a value");
        for def in reg.params() {
            let v = p.get(def.id).expect("value present");
            assert!(def.values.iter().any(|d| d.id == v), "value is registered");
        }
    }

    #[test]
    fn sampling_replays_bit_for_bit_and_keys_on_the_culture() {
        let (reg, prior, harmony) = seed();
        let t = tilt();
        let a = sample_profile(&reg, &prior, &harmony, &t, seq(), disharmony(), 42, 7, 3).unwrap();
        let b = sample_profile(&reg, &prior, &harmony, &t, seq(), disharmony(), 42, 7, 3).unwrap();
        assert_eq!(a, b, "same coordinates, bit-identical profile");
        let mut any_differs = false;
        for culture in 0..50u64 {
            let c = sample_profile(
                &reg,
                &prior,
                &harmony,
                &t,
                seq(),
                disharmony(),
                42,
                culture,
                3,
            )
            .unwrap();
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
        let t = tilt();
        let mut reversed = TypologyRegistry::new();
        for p in reg.params().iter().rev() {
            reversed.add_param(p.clone());
        }
        assert_eq!(reg, reversed, "the registry walk is canonical");
        let a = sample_profile(&reg, &prior, &harmony, &t, seq(), disharmony(), 9, 1, 0).unwrap();
        let b = sample_profile(
            &reversed,
            &prior,
            &harmony,
            &t,
            seq(),
            disharmony(),
            9,
            1,
            0,
        )
        .unwrap();
        assert_eq!(a, b, "insertion order never reaches a draw");
    }

    #[test]
    fn harmony_is_a_tendency_not_a_rule() {
        // With the disharmony gate at one every parameter ignores the tilt: bit-identical
        // to sampling under an empty harmony model. The tilt can bias, never dictate.
        let (reg, prior, harmony) = seed();
        let t = tilt();
        for culture in 0..20u64 {
            let gated =
                sample_profile(&reg, &prior, &harmony, &t, seq(), Fixed::ONE, 5, culture, 0)
                    .unwrap();
            let empty = sample_profile(
                &reg,
                &prior,
                &HarmonyModel::new(),
                &t,
                seq(),
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
        // A two-parameter toy: anchor {0,1} even prior, dependent {0,1} even prior, one pairing
        // with a large structural weight and a tiny temperature so the derived tilt saturates at
        // TILT_MAX and binds the dependent. The tilted weights are exact, and over a fixed seed the
        // conditional holds on every drawn culture.
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
            structural_weight: Fixed::from_int(1000),
            source: "test".into(),
        });
        // A large structural weight (cost near the normalized ceiling) over a tiny temperature
        // saturates the derived tilt at TILT_MAX, so d0 carries count*TILT_MAX in Q32.32 bits
        // (1 * 2^20 * 2^32 = 2^52) and d1 the bare count.
        let strong = TiltParams::new(Fixed::from_int(1), Fixed::from_ratio(1, 100));
        let w = tilted_weights(
            prior.counts(TypologyParamId(1)).unwrap(),
            TypologyParamId(1),
            &[(TypologyParamId(0), TypologyValueId(0))],
            &harmony,
            &strong,
            false,
        );
        assert_eq!(
            w[0],
            (TypologyValueId(0), 1u128 << 52),
            "d0 tilt saturates at TILT_MAX"
        );
        assert_eq!(
            w[1],
            (TypologyValueId(1), 1u128 << 32),
            "d1 keeps its bare marginal"
        );
        // And on every culture this seed draws with the anchor at 0, the dependent is 0.
        let mut conditioned = 0;
        for culture in 0..64u64 {
            let p = sample_profile(
                &reg,
                &prior,
                &harmony,
                &strong,
                seq(),
                Fixed::ZERO,
                77,
                culture,
                0,
            )
            .unwrap();
            if p.get(TypologyParamId(0)) == Some(TypologyValueId(0)) {
                conditioned += 1;
                assert_eq!(
                    p.get(TypologyParamId(1)),
                    Some(TypologyValueId(0)),
                    "a saturated tilt binds the dependent on this fixed seed"
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
        // privileged by its index and the derived tilt never reads an id.
        let (_reg, prior, harmony) = seed();
        let t = tilt();
        let counts = prior.counts(P_ADP).unwrap().to_vec();
        let drawn = [(P_OV, OV)];
        let base = tilted_weights(&counts, P_ADP, &drawn, &harmony, &t, false);
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
            if nb.then_param == P_ADP {
                nb.then_value = perm(nb.then_value);
            }
            permuted_harmony.add(nb);
        }
        let mapped = tilted_weights(
            &permuted_counts,
            P_ADP,
            &drawn,
            &permuted_harmony,
            &t,
            false,
        );
        for &(v, w) in &base {
            let target = perm(v);
            let found = mapped.iter().find(|&&(mv, _)| mv == target).unwrap().1;
            assert_eq!(found, w, "weight of {v:?} maps bit-exactly to {target:?}");
        }
    }

    #[test]
    fn direction_neutrality_no_steering() {
        // THE non-steering, direction-neutrality invariant. Two races identical but for a
        // permutation swapping which adposition value id is head-initial (postpositions 0 <-> 1)
        // produce IDENTICAL parse cost, IDENTICAL derived tilts, and IDENTICAL sampled harmonic
        // proportions. The law rewards CONSISTENCY, never a direction: the kernels see only
        // structural-weight scalars, never a word-order value.
        let (reg, prior, harmony) = seed();
        let t = tilt();
        let swap01 = |v: TypologyValueId| match v.0 {
            0 => TypologyValueId(1),
            1 => TypologyValueId(0),
            _ => v,
        };
        // Build the head-initial-swapped race: only the adposition parameter's value ids move.
        let mut reg2 = TypologyRegistry::new();
        for p in reg.params() {
            let mut q = p.clone();
            if q.id == P_ADP {
                for v in &mut q.values {
                    v.id = swap01(v.id);
                }
            }
            reg2.add_param(q);
        }
        let mut prior2 = TypologyPrior::new();
        for pid in prior.params() {
            let counts: Vec<(TypologyValueId, u32)> = prior
                .counts(pid)
                .unwrap()
                .iter()
                .map(|&(v, c)| if pid == P_ADP { (swap01(v), c) } else { (v, c) })
                .collect();
            prior2.set(pid, counts, prior.source(pid).unwrap());
        }
        let mut harmony2 = HarmonyModel::new();
        for b in harmony.biases() {
            let mut nb = b.clone();
            if nb.then_param == P_ADP {
                nb.then_value = swap01(nb.then_value);
            }
            harmony2.add(nb);
        }

        // (a) IDENTICAL derived tilt: the harmonic value's tilt is the same regardless of the id it
        // wears (postpositions is id 0 in race A, id 1 in race B).
        let tilt_a = derived_tilt(P_ADP, POST, &[(P_OV, OV)], &harmony, &t);
        let tilt_b = derived_tilt(P_ADP, swap01(POST), &[(P_OV, OV)], &harmony2, &t);
        assert_eq!(tilt_a, tilt_b, "the derived tilt never reads a direction");
        assert!(
            tilt_a > Fixed::ONE,
            "the harmonic value is favoured (a real tilt)"
        );

        // (b) IDENTICAL parse cost: a mixed OV grammar and its head-initial swap cost the same.
        let mixed_a = TypologyProfile::new(vec![(P_OV, OV), (P_ADP, PREP), (P_GEN, GENN)]);
        let mixed_b = TypologyProfile::new(vec![(P_OV, OV), (P_ADP, swap01(PREP)), (P_GEN, GENN)]);
        let m = t.memory_capacity;
        assert_eq!(
            grammar_parse_cost(&mixed_a, &harmony, m, Fixed::ONE),
            grammar_parse_cost(&mixed_b, &harmony2, m, Fixed::ONE),
            "the grammar parse cost is direction-blind"
        );

        // (c) IDENTICAL sampled harmonic proportions: the exact tilted probability of postpositions
        // given OV is the same in both races.
        let p_a = tilted_proportion(&prior, &harmony, &t, P_ADP, &[(P_OV, OV)], POST);
        let p_b = tilted_proportion(&prior2, &harmony2, &t, P_ADP, &[(P_OV, OV)], swap01(POST));
        assert_eq!(
            p_a, p_b,
            "the harmonic proportion is identical under the swap"
        );

        // The realized sweep confirms it: the non-adposition parameters draw bit-identically, and
        // the empirical postpositions-given-OV proportions match within sampling tolerance.
        let n = 4000u64;
        let (mut ov_a, mut post_a, mut post_b) = (0u64, 0u64, 0u64);
        for culture in 0..n {
            let ga = sample_profile(
                &reg,
                &prior,
                &harmony,
                &t,
                seq(),
                disharmony(),
                5,
                culture,
                0,
            )
            .unwrap();
            let gb = sample_profile(
                &reg2,
                &prior2,
                &harmony2,
                &t,
                seq(),
                disharmony(),
                5,
                culture,
                0,
            )
            .unwrap();
            assert_eq!(
                ga.get(P_GEN),
                gb.get(P_GEN),
                "a non-permuted parameter draws bit-identically"
            );
            if ga.get(P_OV) == Some(OV) {
                ov_a += 1;
                if ga.get(P_ADP) == Some(POST) {
                    post_a += 1;
                }
                if gb.get(P_ADP) == Some(swap01(POST)) {
                    post_b += 1;
                }
            }
        }
        assert!(ov_a > 0, "OV drew somewhere");
        let (fa, fb) = (post_a as f64 / ov_a as f64, post_b as f64 / ov_a as f64);
        assert!(
            (fa - fb).abs() < 0.03,
            "the sampled harmonic proportion is direction-neutral: {fa} vs {fb}"
        );
    }

    #[test]
    fn a_simultaneous_modality_suppresses_the_tilt() {
        // The divergent-modality invariant. Two cultures identical but for the modality's
        // simultaneous flag diverge: the sequential one shows a harmony tilt in its sampled
        // profile, the simultaneous one shows NONE (the untilted marginal). A modality with no
        // linear word order has nothing for the tilt to act on.
        let (reg, prior, harmony) = seed();
        let t = tilt();
        // The suppressed weights equal the bare marginal; the tilted ones boost the harmonic value.
        let counts = prior.counts(P_ADP).unwrap();
        let suppressed = tilted_weights(counts, P_ADP, &[(P_OV, OV)], &harmony, &t, true);
        for &(v, w) in &suppressed {
            let c = counts.iter().find(|&&(cv, _)| cv == v).unwrap().1;
            assert_eq!(
                w,
                (c as u128) << 32,
                "a suppressed weight is the bare marginal"
            );
        }
        let tilted = tilted_weights(counts, P_ADP, &[(P_OV, OV)], &harmony, &t, false);
        let w_post_tilted = tilted.iter().find(|&&(v, _)| v == POST).unwrap().1;
        let w_post_marginal = (577u128) << 32;
        assert!(
            w_post_tilted > w_post_marginal,
            "the sequential tilt boosts the harmonic value"
        );

        // Sampled: the simultaneous culture's postpositions-given-OV proportion sits at the untilted
        // marginal, the sequential one's well above it, and the two profiles diverge.
        let marginal = 577.0 / 1184.0;
        let n = 4000u64;
        let (mut ov, mut seq_post, mut sim_post, mut diverged) = (0u64, 0u64, 0u64, false);
        for culture in 0..n {
            let s = sample_profile(
                &reg,
                &prior,
                &harmony,
                &t,
                seq(),
                disharmony(),
                8,
                culture,
                0,
            )
            .unwrap();
            let m = sample_profile(
                &reg,
                &prior,
                &harmony,
                &t,
                Linearization::Simultaneous,
                disharmony(),
                8,
                culture,
                0,
            )
            .unwrap();
            if s != m {
                diverged = true;
            }
            if s.get(P_OV) == Some(OV) {
                ov += 1;
                if s.get(P_ADP) == Some(POST) {
                    seq_post += 1;
                }
            }
            if m.get(P_OV) == Some(OV) && m.get(P_ADP) == Some(POST) {
                sim_post += 1;
            }
        }
        assert!(diverged, "the two modalities produce different grammars");
        assert!(ov > 0, "OV drew somewhere");
        let f_seq = seq_post as f64 / ov as f64;
        // The simultaneous OV count is over the same anchor distribution (same seed), so use ov.
        let f_sim = sim_post as f64 / ov as f64;
        assert!(
            f_seq > marginal + 0.2,
            "the sequential modality shows a strong harmony tilt: {f_seq}"
        );
        assert!(
            (f_sim - marginal).abs() < 0.05,
            "the simultaneous modality shows the untilted marginal ~{marginal}: {f_sim}"
        );
    }

    #[test]
    fn larger_working_memory_weakens_harmony() {
        // Per-race differentiation. Two races differing ONLY in the working-memory parameter get
        // different tilt magnitudes: a larger memory softens the parse cost, so the harmony
        // pressure is weaker (a smaller tilt and a lower harmonic proportion).
        let (_reg, prior, harmony) = seed();
        let temp = Fixed::from_ratio(92, 1000);
        let small_mem = TiltParams::new(Fixed::from_int(2), temp);
        let large_mem = TiltParams::new(Fixed::from_int(20), temp);
        let tilt_small = derived_tilt(P_ADP, POST, &[(P_OV, OV)], &harmony, &small_mem);
        let tilt_large = derived_tilt(P_ADP, POST, &[(P_OV, OV)], &harmony, &large_mem);
        assert!(
            tilt_large < tilt_small && tilt_large > Fixed::ONE,
            "a larger memory weakens (but keeps) the harmony tilt: {tilt_large:?} < {tilt_small:?}"
        );
        let p_small = tilted_proportion(&prior, &harmony, &small_mem, P_ADP, &[(P_OV, OV)], POST);
        let p_large = tilted_proportion(&prior, &harmony, &large_mem, P_ADP, &[(P_OV, OV)], POST);
        assert!(
            p_large < p_small,
            "a larger memory lowers the harmonic proportion: {p_large} < {p_small}"
        );
    }

    #[test]
    fn harmonic_grammar_costs_less_than_a_mixed_one() {
        // Monotonicity at grammar scope: a fully harmonic candidate grammar costs strictly less
        // parse cost than a mixed-branching one, and each additional violation adds cost.
        let (_reg, _prior, harmony) = seed();
        let m = tilt().memory_capacity;
        let harmonic = TypologyProfile::new(vec![
            (P_OV, OV),
            (P_ADP, POST),
            (P_GEN, GENN),
            (P_REL, NREL),
        ]);
        let one_violation = TypologyProfile::new(vec![
            (P_OV, OV),
            (P_ADP, PREP),
            (P_GEN, GENN),
            (P_REL, NREL),
        ]);
        let mixed = TypologyProfile::new(vec![
            (P_OV, OV),
            (P_ADP, PREP),
            (P_GEN, NGEN),
            (P_REL, NREL),
        ]);
        let c_harmonic = grammar_parse_cost(&harmonic, &harmony, m, Fixed::ONE);
        let c_one = grammar_parse_cost(&one_violation, &harmony, m, Fixed::ONE);
        let c_mixed = grammar_parse_cost(&mixed, &harmony, m, Fixed::ONE);
        assert_eq!(
            c_harmonic,
            Fixed::ZERO,
            "a fully harmonic grammar holds no long domains"
        );
        assert!(
            c_harmonic < c_one && c_one < c_mixed,
            "parse cost rises strictly with mixed branching: {c_harmonic:?} < {c_one:?} < {c_mixed:?}"
        );
    }

    #[test]
    fn sampling_is_deterministic_across_repeats_and_call_order() {
        // Determinism and observer independence (Principle 10). The profile is a pure function of
        // the registry, prior, harmony, tilt params, modality flag, and coordinates: bit-identical
        // on repeat and independent of any surrounding computation, so it is the same at every LOD
        // tier (no tier enters the draw). Thread-count invariance of the internal folds is proven
        // separately in crates/sim/tests/reduce_order.rs; here the fold order-independence is
        // structural (Fixed::saturating_sum in derived_tilt and grammar_parse_cost).
        let (reg, prior, harmony) = seed();
        let t = tilt();
        let first =
            sample_profile(&reg, &prior, &harmony, &t, seq(), disharmony(), 0xABC, 7, 2).unwrap();
        // Interleave a swathe of other draws, then repeat: no hidden state leaks in.
        for culture in 0..30u64 {
            let _ = sample_profile(
                &reg,
                &prior,
                &harmony,
                &t,
                seq(),
                disharmony(),
                0xABC,
                culture,
                9,
            )
            .unwrap();
        }
        let again =
            sample_profile(&reg, &prior, &harmony, &t, seq(), disharmony(), 0xABC, 7, 2).unwrap();
        assert_eq!(
            first, again,
            "the pass replays bit for bit regardless of call order"
        );
        // The derived tilt itself is a pure function of its inputs (the observer-independence unit).
        let a = derived_tilt(P_ADP, POST, &[(P_OV, OV)], &harmony, &t);
        let b = derived_tilt(P_ADP, POST, &[(P_OV, OV)], &harmony, &t);
        assert_eq!(a, b, "the derived tilt is a pure function");
    }

    #[test]
    fn the_single_temperature_reconstructs_the_human_row() {
        // The human-data-row calibration gate. With the single softmax temperature at the human
        // calibration (and the human race's working memory), the sampled harmonic proportion on the
        // adposition axis reconstructs WALS 95A (94 to 97 percent) and the genitive axis
        // reconstructs Dryer (~0.89 on the OV side), proving the ONE free scale is VALIDATED against
        // the human row, not the tiers set directly. The untilted VO/genitive side sits at the
        // WALS-language-count marginal (about 0.55), the model's honest one-sided prediction (Dryer's
        // 0.45 is a genera-based figure the one-sided bias does not tilt toward).
        let (_reg, prior, harmony) = seed();
        let t = tilt();
        let p_post = tilted_proportion(&prior, &harmony, &t, P_ADP, &[(P_OV, OV)], POST);
        assert!(
            (0.94..=0.975).contains(&p_post),
            "WALS 95A adposition harmony reconstructs ~95%: {p_post}"
        );
        let p_gen = tilted_proportion(&prior, &harmony, &t, P_GEN, &[(P_OV, OV)], GENN);
        assert!(
            (0.85..=0.93).contains(&p_gen),
            "Dryer genitive OV-side reconstructs ~0.89: {p_gen}"
        );
        // The untilted side (no OV anchor firing) sits at the bare marginal, the one-sided result.
        let p_gen_vo = tilted_proportion(&prior, &harmony, &t, P_GEN, &[(P_OV, VO)], GENN);
        let marginal = 685.0 / 1249.0;
        assert!(
            (p_gen_vo - marginal).abs() < 1e-6,
            "the VO genitive side is untilted at the marginal ~{marginal}: {p_gen_vo}"
        );
    }

    #[test]
    fn a_forward_conditioning_bias_is_a_load_error_not_a_silent_no_op() {
        let (reg, prior, mut harmony) = seed();
        // Adposition (priority 1) conditioning the anchor (priority 0): backwards.
        harmony.add(HarmonyBias {
            given_param: P_ADP,
            given_value: POST,
            then_param: P_OV,
            then_value: OV,
            structural_weight: Fixed::from_int(2),
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
            if pid == P_OV {
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
            given_param: P_OV,
            given_value: TypologyValueId(9),
            then_param: P_ADP,
            then_value: POST,
            structural_weight: Fixed::from_int(2),
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &bad_value),
            Err(TypologyError::UnknownValue(_))
        ));
        // A self-conditioning bias.
        let mut self_bias = HarmonyModel::new();
        self_bias.add(HarmonyBias {
            given_param: P_ADP,
            given_value: POST,
            then_param: P_ADP,
            then_value: PREP,
            structural_weight: Fixed::from_int(2),
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &self_bias),
            Err(TypologyError::BadBias(_))
        ));
        // A non-positive structural weight.
        let mut bad_weight = HarmonyModel::new();
        bad_weight.add(HarmonyBias {
            given_param: P_OV,
            given_value: OV,
            then_param: P_ADP,
            then_value: POST,
            structural_weight: Fixed::ZERO,
            source: "test".into(),
        });
        assert!(matches!(
            validate(&reg, &prior, &bad_weight),
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
        let a = sample_profile(
            &reg,
            &prior,
            &harmony,
            &tilt(),
            seq(),
            disharmony(),
            11,
            0,
            0,
        )
        .unwrap();
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
id = "lang.typology_temperature"
basis = "test fixture"
status = "reserved"
value = ""
unit = "temperature"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        assert!(
            TypologyParams::from_manifest(&m).is_err(),
            "a reserved temperature fails loud, never a fabricated default"
        );
    }

    #[test]
    fn information_weights_are_the_order_two_diversity_of_the_prior_counts() {
        // A three-parameter toy registry with hand-set prior counts, so the derived weight is a
        // checkable order-2 diversity N^2 / sum(c_i^2), never an authored table.
        let two_value = |id: u32, prio: u32| TypologyParamDef {
            id: TypologyParamId(id),
            gloss: "p".into(),
            values: vec![
                TypologyValueDef {
                    id: TypologyValueId(0),
                    gloss: "v0".into(),
                },
                TypologyValueDef {
                    id: TypologyValueId(1),
                    gloss: "v1".into(),
                },
            ],
            sample_priority: prio,
            source: "test fixture".into(),
        };
        let mut reg = TypologyRegistry::new();
        reg.add_param(two_value(0, 0));
        reg.add_param(two_value(1, 1));
        // A single-value parameter for the diversity-one case.
        reg.add_param(TypologyParamDef {
            id: TypologyParamId(2),
            gloss: "p2".into(),
            values: vec![TypologyValueDef {
                id: TypologyValueId(0),
                gloss: "only".into(),
            }],
            sample_priority: 2,
            source: "test fixture".into(),
        });

        let mut prior = TypologyPrior::new();
        // Param 0: an even split 3/3. N=6, sum_sq=18, diversity = 36/18 = 2 exactly.
        prior.set(
            TypologyParamId(0),
            vec![(TypologyValueId(0), 3), (TypologyValueId(1), 3)],
            "fixture",
        );
        // Param 1: a lopsided split 9/1. N=10, sum_sq=82, diversity = 100/82.
        prior.set(
            TypologyParamId(1),
            vec![(TypologyValueId(0), 9), (TypologyValueId(1), 1)],
            "fixture",
        );
        // Param 2: a single value with count 5. N=5, sum_sq=25, diversity = 1 exactly.
        prior.set(TypologyParamId(2), vec![(TypologyValueId(0), 5)], "fixture");

        let weights = information_weights(&reg, &prior);
        assert_eq!(weights.len(), 3);
        assert_eq!(
            weights[&TypologyParamId(0)],
            Fixed::from_int(2),
            "an even two-value split has diversity two"
        );
        assert_eq!(
            weights[&TypologyParamId(2)],
            Fixed::ONE,
            "a one-value parameter has diversity one"
        );
        // The lopsided parameter is more concentrated, so it carries less information and weighs
        // less than the even one but still above one, bit-exact 100/82 in Q32.32.
        let lop = weights[&TypologyParamId(1)];
        assert!(
            lop > Fixed::ONE && lop < Fixed::from_int(2),
            "a lopsided split weighs between one and the even-split diversity ({lop:?})"
        );
        assert_eq!(lop, Fixed::from_bits(((100u128 << 32) / 82) as i64));

        // Determinism: the derivation replays bit for bit.
        assert_eq!(information_weights(&reg, &prior), weights);

        // A parameter with no prior row is omitted, exactly as an absent weight, so it drops out
        // of the distance rather than fabricating a weight.
        let mut partial = TypologyPrior::new();
        partial.set(
            TypologyParamId(0),
            vec![(TypologyValueId(0), 3), (TypologyValueId(1), 3)],
            "fixture",
        );
        let partial_w = information_weights(&reg, &partial);
        assert_eq!(partial_w.len(), 1);
        assert!(!partial_w.contains_key(&TypologyParamId(1)));

        // Drop-in: the derived weights plug straight into typology_distance and score a
        // one-value difference at exactly that parameter's diversity weight.
        let a = TypologyProfile::new(vec![
            (TypologyParamId(0), TypologyValueId(0)),
            (TypologyParamId(1), TypologyValueId(0)),
        ]);
        let b = TypologyProfile::new(vec![
            (TypologyParamId(0), TypologyValueId(1)),
            (TypologyParamId(1), TypologyValueId(0)),
        ]);
        let metrics = BTreeMap::new();
        assert_eq!(
            typology_distance(&a, &b, &weights, &metrics),
            Fixed::from_int(2),
            "the derived weights are a drop-in for the distance's weights arg"
        );
    }
}
