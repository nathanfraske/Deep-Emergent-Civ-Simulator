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

//! Emergent lexicon over a data-defined articulation substrate (design Part 33, the
//! resolved R-LANG-DET and R-LANG-MODALITY work).
//!
//! A concept is a meaning the engine tracks; a word is the surface form a culture attaches
//! to it. A word form is not an opaque string: it is a sequence of form primitives, and
//! each primitive is a [`FormSegment`], a canonical bundle of simultaneous feature values
//! over a production modality's feature dimensions (record 62.13). A spoken word is a long
//! sequence of thin bundles, a sign a short sequence of thick ones, a chromatic flash a
//! sequence of colour bundles: the mechanism is one, the modality is data. The bundle is
//! stored sorted by feature-dimension id with one value per dimension, so two machines build
//! a bit-identical primitive (the R-LANG-DET canonicalisation), and a form is a `Vec` of
//! these, walked left to right.
//!
//! The substrate ([`ArticulationSubstrate`]) is the data registry of production modalities,
//! feature dimensions, and their contrastive values, sibling to the value substrate (Part 21)
//! and the access-channel registry (Part 40); membership is data and the mechanism is fixed
//! Rust (Principle 11). A culture builds its words from a [`FormSystem`], a selection of
//! producible primitives in one modality with a length range. Determinism by construction:
//! every form is integer feature indices, every coinage is counter-based RNG keyed on the
//! coiner, the concept, and the tick, and every walk is id-ordered. The deeper Part 33 pieces
//! that wait on later increments: regular form change (drift over generations as feature
//! rewrites in innovation-index order), the cross-culture distance over the shared semantic
//! substrate, and per-being produce and perceive channels.

use std::collections::BTreeMap;

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::typology::TypologyProfile;
use civsim_core::{Fixed, Rng};

/// The reserved calibration the naming game needs: how often a speaker coins a fresh
/// variant instead of reusing its word, the seed of drift. Read from the manifest,
/// failing loud while reserved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LanguageParams {
    /// The per-interaction probability of coining a fresh word form (0..1).
    pub innovation_rate: Fixed,
}

impl LanguageParams {
    /// Read the language calibration from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(LanguageParams {
            innovation_rate: m.require_fixed("language.innovation_rate")?,
        })
    }
}

/// A concept: a meaning the engine tracks. Here an identifier; its representation as a
/// region over the semantic substrate (design 33.1) is a later increment.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ConceptId(pub u32);

/// A production modality: a channel a body emits in (acoustic, manual, chromatic, and so
/// on). A data registry id, never a closed enum (R-LANG-MODALITY).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct ProductionModalityId(pub u32);

/// A feature dimension a modality's primitives contrast over (place, manner, voicing for
/// the acoustic modality; handshape, location, movement for the manual one). A registry id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FeatureDimId(pub u32);

/// A contrastive value on a feature dimension. A registry id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FeatureValueId(pub u32);

/// One contrastive value on a dimension, with its etic gloss lemma: a short surface token
/// for rendering. The lemma is the one sanctioned hardcoding (design 33.2), finite and
/// mechanism rather than world content.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FeatureValueDef {
    /// The value's registry id.
    pub id: FeatureValueId,
    /// The surface token this value renders as.
    pub gloss: String,
}

/// A feature dimension: its contrastive values, and whether every well-formed primitive must
/// fill it. Values are kept sorted by id for a canonical walk.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FeatureDimDef {
    /// The dimension's registry id.
    pub id: FeatureDimId,
    /// The contrastive values on this dimension, sorted by id.
    pub values: Vec<FeatureValueDef>,
    /// Whether every primitive in this modality must carry a value on this dimension.
    pub obligatory: bool,
}

/// Whether a modality lays its forms out in a linear sequence (so word order, and the
/// dependency-integration parse cost of holding one constituent before its head arrives, apply)
/// or presents its structure simultaneously (a chromatic flash carrying every feature at once, a
/// posture held whole), where a linear word-order harmony tilt has nothing to act on. The gate on
/// the R-LANG-TYPOLOGY harmony tilt: a simultaneous modality suppresses it and the typology draws
/// from its untilted marginal. Data on the modality, not a race branch (Principle 9). The default
/// is `Sequential`, so the existing acoustic and manual modality data keep their linear word order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub enum Linearization {
    /// Forms are a left-to-right sequence: word order exists and the parse-cost tilt applies.
    #[default]
    Sequential,
    /// Forms are presented all at once: there is no linear order for a harmony tilt to bias.
    Simultaneous,
}

/// A production modality definition: the feature dimensions its primitives are built from,
/// in id order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProductionModalityDef {
    /// The modality's registry id.
    pub id: ProductionModalityId,
    /// The feature dimensions a primitive in this modality contrasts over, sorted by id.
    pub dims: Vec<FeatureDimId>,
    /// Whether this modality is laid out sequentially or presented simultaneously: the gate on
    /// whether the linear word-order harmony tilt applies (default [`Linearization::Sequential`]).
    pub linearization: Linearization,
}

/// One form primitive: a canonical bundle of simultaneous feature values over a modality's
/// dimensions. Stored sorted by [`FeatureDimId`] with one value per dimension, so the bundle
/// carries no order of its own and two builders produce a bit-identical primitive (the
/// R-LANG-DET canonicalisation). The sequential axis lives in [`Word`], never here.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct FormSegment {
    features: Vec<(FeatureDimId, FeatureValueId)>,
}

impl FormSegment {
    /// Build a canonical primitive from its feature values: sorted by dimension id, with one
    /// value per dimension (the first given for a repeated dimension), so the result is
    /// independent of the order the values were supplied.
    pub fn new(features: impl IntoIterator<Item = (FeatureDimId, FeatureValueId)>) -> Self {
        let mut features: Vec<(FeatureDimId, FeatureValueId)> = features.into_iter().collect();
        features.sort_by_key(|(d, _)| d.0);
        features.dedup_by_key(|(d, _)| d.0);
        FormSegment { features }
    }

    /// The feature values of this primitive, in dimension-id order.
    pub fn features(&self) -> &[(FeatureDimId, FeatureValueId)] {
        &self.features
    }
}

/// An emergent word form: a sequence of form primitives in one production modality. Two
/// cultures almost never build the same form for a concept, which is what makes their
/// lexicons diverge, and the form is renderable through the substrate rather than an opaque
/// id. Comparable and ordered so the naming game converges and the lexicon walks canonically.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Word {
    modality: ProductionModalityId,
    segments: Vec<FormSegment>,
}

impl Word {
    /// A word from a modality and its sequence of primitives.
    pub fn new(
        modality: ProductionModalityId,
        segments: impl IntoIterator<Item = FormSegment>,
    ) -> Self {
        Word {
            modality,
            segments: segments.into_iter().collect(),
        }
    }

    /// The production modality this word is in.
    pub fn modality(&self) -> ProductionModalityId {
        self.modality
    }

    /// The form's primitives, left to right.
    pub fn segments(&self) -> &[FormSegment] {
        &self.segments
    }

    /// Whether the form has no primitives.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// How many primitives the form has.
    pub fn len(&self) -> usize {
        self.segments.len()
    }
}

/// A culture's articulation system: a selection of producible primitives in one modality,
/// with the word-length range. The inventory is data the owner provides (the per-race
/// producible form set of design 33.3); the dev fixtures supply a placeholder, never an
/// authored inventory. Coining is deterministic.
#[derive(Clone, Debug, Default)]
pub struct FormSystem {
    modality: ProductionModalityId,
    inventory: Vec<FormSegment>,
    min_len: u32,
    max_len: u32,
}

impl FormSystem {
    /// Build a form system from its modality, producible primitives, and the inclusive
    /// word-length range. The inventory is sorted for a canonical index, and a degenerate
    /// range is clamped so coining always terminates.
    pub fn new(
        modality: ProductionModalityId,
        inventory: impl IntoIterator<Item = FormSegment>,
        min_len: u32,
        max_len: u32,
    ) -> Self {
        let mut inventory: Vec<FormSegment> = inventory.into_iter().collect();
        inventory.sort();
        let min_len = min_len.max(1);
        FormSystem {
            modality,
            inventory,
            min_len,
            max_len: max_len.max(min_len),
        }
    }

    /// Whether the inventory is empty (coining then yields an empty form).
    pub fn is_empty(&self) -> bool {
        self.inventory.is_empty()
    }

    /// The modality this system builds forms in.
    pub fn modality(&self) -> ProductionModalityId {
        self.modality
    }

    /// Coin a fresh word by sampling a length and then that many primitives from the
    /// inventory, each draw on a distinct counter of the supplied keyed RNG. Deterministic:
    /// the same key yields the same form on every machine, and a different inventory or key
    /// yields a different form.
    pub fn coin(&self, rng: Rng) -> Word {
        if self.inventory.is_empty() {
            return Word::new(self.modality, []);
        }
        let span = self.max_len - self.min_len + 1;
        let len = self.min_len + rng.range_u32(0, span);
        let mut segments = Vec::with_capacity(len as usize);
        for i in 0..len {
            let idx = rng.range_u32(i as u64 + 1, self.inventory.len() as u32) as usize;
            segments.push(self.inventory[idx].clone());
        }
        Word::new(self.modality, segments)
    }
}

/// The articulation substrate: the data registries of modalities, dimensions, and values
/// that forms are built from and rendered through. Sibling to the value substrate (Part 21);
/// membership is data, the mechanism is fixed (Principle 11).
#[derive(Clone, Debug, Default)]
pub struct ArticulationSubstrate {
    dims: BTreeMap<FeatureDimId, FeatureDimDef>,
    modalities: BTreeMap<ProductionModalityId, ProductionModalityDef>,
}

impl ArticulationSubstrate {
    /// An empty substrate.
    pub fn new() -> Self {
        ArticulationSubstrate::default()
    }

    /// Register a feature dimension (its values kept sorted by id).
    pub fn add_dim(&mut self, mut dim: FeatureDimDef) {
        dim.values.sort_by_key(|v| v.id.0);
        self.dims.insert(dim.id, dim);
    }

    /// Register a production modality (its dimensions kept sorted by id).
    pub fn add_modality(&mut self, mut m: ProductionModalityDef) {
        m.dims.sort_by_key(|d| d.0);
        self.modalities.insert(m.id, m);
    }

    /// Render a word to its surface string: each primitive rendered in the modality's
    /// dimension order by concatenating its values' gloss lemmas, primitives concatenated.
    /// The deterministic engine-side gloss the legibility guarantee rests on (design 33.2).
    pub fn render(&self, word: &Word) -> String {
        let modality = match self.modalities.get(&word.modality) {
            Some(m) => m,
            None => return String::new(),
        };
        let mut s = String::new();
        for seg in &word.segments {
            for dim in &modality.dims {
                if let Some((_, val)) = seg.features.iter().find(|(d, _)| d == dim) {
                    if let Some(dd) = self.dims.get(dim) {
                        if let Some(vd) = dd.values.iter().find(|v| v.id == *val) {
                            s.push_str(&vd.gloss);
                        }
                    }
                }
            }
        }
        s
    }

    /// A convenience for the common single-dimension case (and the acoustic-syllable case):
    /// one production modality with one feature dimension carrying a value per token, each
    /// rendering as that token, and a form system whose inventory is one primitive per
    /// token. The real per-race substrate is owner data; this builds a valid substrate from
    /// surface tokens so development and tests have a concrete, renderable language.
    pub fn syllabic(
        tokens: impl IntoIterator<Item = String>,
        min_len: u32,
        max_len: u32,
    ) -> (Self, FormSystem) {
        let modality = ProductionModalityId(0);
        let dim = FeatureDimId(0);
        let values: Vec<FeatureValueDef> = tokens
            .into_iter()
            .enumerate()
            .map(|(i, gloss)| FeatureValueDef {
                id: FeatureValueId(i as u32),
                gloss,
            })
            .collect();
        let inventory: Vec<FormSegment> = values
            .iter()
            .map(|v| FormSegment::new([(dim, v.id)]))
            .collect();
        let mut substrate = ArticulationSubstrate::new();
        substrate.add_dim(FeatureDimDef {
            id: dim,
            values,
            obligatory: true,
        });
        substrate.add_modality(ProductionModalityDef {
            id: modality,
            dims: vec![dim],
            // The syllabic convenience is a sequential (spoken/signed) modality: word order applies.
            linearization: Linearization::default(),
        });
        let forms = FormSystem::new(modality, inventory, min_len, max_len);
        (substrate, forms)
    }
}

/// One mind's lexicon: the word it currently uses for each concept it has a word for.
#[derive(Clone, Debug, Default)]
pub struct Lexicon {
    by_concept: BTreeMap<ConceptId, Word>,
}

impl Lexicon {
    /// An empty lexicon.
    pub fn new() -> Self {
        Lexicon::default()
    }

    /// The word this mind uses for a concept, if it has one.
    pub fn word_for(&self, concept: ConceptId) -> Option<&Word> {
        self.by_concept.get(&concept)
    }

    /// Learn or realign: use this word for this concept from now on.
    pub fn adopt(&mut self, concept: ConceptId, word: Word) {
        self.by_concept.insert(concept, word);
    }

    /// The concept-to-word pairs, in concept-id order, for a canonical walk.
    pub fn entries(&self) -> impl Iterator<Item = (&ConceptId, &Word)> + '_ {
        self.by_concept.iter()
    }

    /// Render an ordered sequence of concepts as this mind's coined words, joined by
    /// spaces, so a thought can be shown in the band's own emergent language rather than
    /// an English gist (design 33.2). A concept the mind has no word for yet renders as the
    /// `unknown` placeholder. This is the legibility layer over the naming game: it reads
    /// the words a culture coined and never invents one, so it never enters canon.
    pub fn utterance(
        &self,
        concepts: &[ConceptId],
        substrate: &ArticulationSubstrate,
        unknown: &str,
    ) -> String {
        concepts
            .iter()
            .map(|c| {
                self.word_for(*c)
                    .map(|w| substrate.render(w))
                    .unwrap_or_else(|| unknown.to_string())
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// How many concepts this mind has a word for.
    pub fn len(&self) -> usize {
        self.by_concept.len()
    }

    /// Whether the lexicon is empty.
    pub fn is_empty(&self) -> bool {
        self.by_concept.is_empty()
    }
}

/// A language lineage id: a descent line that drifts as a unit (design 33.4).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct LangId(pub u32);

/// A regular form change (the generalised sound change of design 33.4): a feature value on
/// one dimension becomes another value on that dimension, applied at once to every form in a
/// lineage's lexicon. Composes with other rules in innovation-index order, so feeding and
/// bleeding fall out of relative chronology (the R-LANG-DET core). Unconditioned here; an
/// environment condition is a later refinement.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FormChangeRule {
    /// The feature dimension the change acts on.
    pub dim: FeatureDimId,
    /// The value that is rewritten.
    pub from: FeatureValueId,
    /// The value it becomes.
    pub to: FeatureValueId,
    /// The chronological index: rules apply in ascending order, the output of each feeding
    /// the next. Append-only and never reordered, so the index is a total order over history.
    pub innovation_index: u64,
}

impl FormChangeRule {
    /// Rewrite a word by this rule: every primitive carrying `(dim, from)` takes `(dim, to)`
    /// instead, re-canonicalised. Applied at once across the form (each primitive rewritten
    /// from its pre-rule value), so the result is independent of primitive order.
    pub fn apply(&self, word: &Word) -> Word {
        let segments = word.segments().iter().map(|seg| {
            let features = seg.features().iter().map(|&(d, v)| {
                if d == self.dim && v == self.from {
                    (d, self.to)
                } else {
                    (d, v)
                }
            });
            FormSegment::new(features)
        });
        Word::new(word.modality(), segments)
    }
}

/// The reserved calibrations drift needs: how often a lineage innovates a form change, and
/// how many ticks make a generation. Read from the manifest, failing loud while reserved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DriftParams {
    /// The per-generation probability that a lineage innovates one regular form change.
    pub sound_change_rate: Fixed,
    /// How many ticks make one generation (the drift cadence).
    ///
    /// DEFERRED derivation (the R-CELESTIAL / R-AGING seam): a generation is a race's maturity in
    /// world-time, so `generation_ticks` should derive as `race.maturity_years` (in orbits) times
    /// the orbital year in ticks (`clock::ticks_from_seconds(orbital.orbital_period_seconds,
    /// base_tick)`). Both halves now exist: the orbital half is the celestial substrate
    /// (`civsim_world::OrbitalElements` plus `clock::ticks_from_seconds`), and the maturity half is
    /// the per-race `Race::maturity_years` datum. The wiring is not forced here because `DriftParams`
    /// carries no `Race` or `OrbitalElements` to derive from; threading them through would change
    /// this reader's signature and every call site, so it is left as a manifest value
    /// (`language.generation_ticks`, still the flagged Earth interim) until the drift path carries a
    /// race and an orbit.
    pub generation_ticks: u64,
}

impl DriftParams {
    /// Read the drift calibration from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(DriftParams {
            sound_change_rate: m.require_fixed("language.sound_change_rate")?,
            generation_ticks: m.require_i64("language.generation_ticks")?.max(1) as u64,
        })
    }
}

impl FormSystem {
    /// The contrastive values present per dimension in the inventory, in id order: the
    /// substrate a form change can act on, read from this lineage's own producible set.
    pub fn dim_values(&self) -> BTreeMap<FeatureDimId, Vec<FeatureValueId>> {
        let mut out: BTreeMap<FeatureDimId, Vec<FeatureValueId>> = BTreeMap::new();
        for seg in &self.inventory {
            for &(d, v) in seg.features() {
                let vals = out.entry(d).or_default();
                if !vals.contains(&v) {
                    vals.push(v);
                }
            }
        }
        for vals in out.values_mut() {
            vals.sort_by_key(|v| v.0);
        }
        out
    }
}

/// A language lineage: a descent line with its own articulation system, the log of regular
/// form changes it has undergone, and a pointer to its parent, so a family tree is
/// reconstructable by walking parents and replaying logs (design 33.4). The lineage is the
/// unit that drifts; its speakers' lexicons are rewritten by each change it innovates.
#[derive(Clone, Debug)]
pub struct Language {
    id: LangId,
    parent: Option<LangId>,
    form_system: FormSystem,
    change_log: Vec<FormChangeRule>,
    typology: TypologyProfile,
}

impl Language {
    /// A root lineage with no parent. The typology profile starts empty; a culture-genesis
    /// caller samples one over the typological registry (R-LANG-TYPOLOGY) and attaches it
    /// with [`Language::set_typology`].
    pub fn new(id: LangId, form_system: FormSystem) -> Self {
        Language {
            id,
            parent: None,
            form_system,
            change_log: Vec::new(),
            typology: TypologyProfile::default(),
        }
    }

    /// This lineage's typology profile: its grammar as a canonical vector over the
    /// typological parameter registry, the data-defined replacement for the design's
    /// closed `GrammarParams` (33.4, R-LANG-TYPOLOGY).
    pub fn typology(&self) -> &TypologyProfile {
        &self.typology
    }

    /// Attach a sampled typology profile.
    pub fn set_typology(&mut self, typology: TypologyProfile) {
        self.typology = typology;
    }

    /// This lineage's id.
    pub fn id(&self) -> LangId {
        self.id
    }

    /// The parent lineage this descended from, if any.
    pub fn parent(&self) -> Option<LangId> {
        self.parent
    }

    /// The articulation system words in this lineage are coined from.
    pub fn form_system(&self) -> &FormSystem {
        &self.form_system
    }

    /// The regular form changes this lineage has undergone, in innovation order.
    pub fn change_log(&self) -> &[FormChangeRule] {
        &self.change_log
    }

    /// Fork a daughter lineage: it inherits the form system and the full change log (so it
    /// shares this lineage's history) and points back to this lineage as its parent. The
    /// daughter then drifts independently, becoming a sister of any other daughter. This is
    /// the split of design 33.4; the trigger that fires it on a population separating couples
    /// to movement and is added there.
    pub fn fork(&self, daughter: LangId) -> Self {
        Language {
            id: daughter,
            parent: Some(self.id),
            form_system: self.form_system.clone(),
            change_log: self.change_log.clone(),
            typology: self.typology.clone(),
        }
    }

    /// Innovate this generation's regular form changes, append them to the log, and return
    /// them in innovation order so a caller can rewrite the lineage's lexicons by them. With
    /// the reserved per-generation rate the lineage coins one change: a value present in its
    /// inventory on some dimension becomes another value on that dimension. Deterministic:
    /// keyed on the supplied counter RNG. A multi-rule generation would append in canonical
    /// content order (the R-LANG-DET same-tick tiebreak); one rule per generation is trivially
    /// ordered.
    pub fn innovate(&mut self, rng: Rng, params: &DriftParams) -> Vec<FormChangeRule> {
        if rng.unit_fixed(0) >= params.sound_change_rate {
            return Vec::new();
        }
        let candidates: Vec<(FeatureDimId, Vec<FeatureValueId>)> = self
            .form_system
            .dim_values()
            .into_iter()
            .filter(|(_, vs)| vs.len() >= 2)
            .collect();
        if candidates.is_empty() {
            return Vec::new();
        }
        let (dim, vals) = &candidates[rng.range_u32(1, candidates.len() as u32) as usize];
        let from = vals[rng.range_u32(2, vals.len() as u32) as usize];
        let others: Vec<FeatureValueId> = vals.iter().copied().filter(|v| *v != from).collect();
        let to = others[rng.range_u32(3, others.len() as u32) as usize];
        let rule = FormChangeRule {
            dim: *dim,
            from,
            to,
            innovation_index: self.change_log.len() as u64,
        };
        self.change_log.push(rule);
        vec![rule]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_segment_is_canonical_regardless_of_build_order() {
        let a = FormSegment::new([
            (FeatureDimId(2), FeatureValueId(5)),
            (FeatureDimId(0), FeatureValueId(1)),
        ]);
        let b = FormSegment::new([
            (FeatureDimId(0), FeatureValueId(1)),
            (FeatureDimId(2), FeatureValueId(5)),
        ]);
        assert_eq!(a, b, "the bundle carries no order of its own");
        // One value per dimension: a repeated dimension keeps the first value.
        let c = FormSegment::new([
            (FeatureDimId(0), FeatureValueId(1)),
            (FeatureDimId(0), FeatureValueId(9)),
        ]);
        assert_eq!(c.features(), &[(FeatureDimId(0), FeatureValueId(1))]);
    }

    #[test]
    fn a_lexicon_adopts_and_realigns() {
        let (_s, forms) =
            ArticulationSubstrate::syllabic(["ka", "lo", "mi"].map(String::from), 2, 2);
        let c = ConceptId(7);
        let mut lex = Lexicon::new();
        assert_eq!(lex.word_for(c), None);
        let w1 = forms.coin(Rng::for_coords(1, &[1]));
        lex.adopt(c, w1.clone());
        assert_eq!(lex.word_for(c), Some(&w1));
        let w2 = forms.coin(Rng::for_coords(2, &[2]));
        lex.adopt(c, w2.clone());
        assert_eq!(lex.word_for(c), Some(&w2));
        assert_eq!(lex.len(), 1);
    }

    #[test]
    fn coining_is_deterministic_and_renderable() {
        let (substrate, forms) = ArticulationSubstrate::syllabic(
            ["ka", "lo", "mi", "tu", "ne", "sa"].map(String::from),
            2,
            3,
        );
        let rng = Rng::for_coords(0x5EED, &[1, 2, 3]);
        let w1 = forms.coin(rng);
        let w2 = forms.coin(rng);
        assert_eq!(w1, w2, "the same key coins the same form");
        let rendered = substrate.render(&w1);
        assert!(
            !rendered.is_empty(),
            "the form renders to a surface string: {rendered}"
        );
        assert!(w1.len() >= 2 && w1.len() <= 3, "the length is in range");
    }

    #[test]
    fn an_utterance_renders_a_thought_in_coined_words() {
        let (substrate, forms) = ArticulationSubstrate::syllabic(
            ["ka", "lo", "mi", "tu", "ne", "sa"].map(String::from),
            2,
            3,
        );
        let mut lex = Lexicon::new();
        let a = forms.coin(Rng::for_coords(1, &[1]));
        let b = forms.coin(Rng::for_coords(1, &[2]));
        lex.adopt(ConceptId(10), a.clone());
        lex.adopt(ConceptId(20), b.clone());
        // A two-concept thought renders as the two coined words joined.
        let said = lex.utterance(&[ConceptId(10), ConceptId(20)], &substrate, "?");
        let expected = format!("{} {}", substrate.render(&a), substrate.render(&b));
        assert_eq!(said, expected);
        // A concept with no coined word yet shows the placeholder, never an invented word.
        let gap = lex.utterance(&[ConceptId(10), ConceptId(99)], &substrate, "?");
        assert_eq!(gap, format!("{} ?", substrate.render(&a)));
    }

    #[test]
    fn different_inventories_build_different_forms() {
        let (sa, fa) = ArticulationSubstrate::syllabic(["ka", "lo", "mi"].map(String::from), 3, 3);
        let (sb, fb) = ArticulationSubstrate::syllabic(["wo", "ha", "du"].map(String::from), 3, 3);
        let key = Rng::for_coords(42, &[7]);
        let wa = fa.coin(key);
        let wb = fb.coin(key);
        // Same key, disjoint inventories: the rendered surfaces differ, so two cultures'
        // lexicons diverge.
        assert_ne!(sa.render(&wa), sb.render(&wb));
    }

    #[test]
    fn form_change_rewrites_a_value() {
        let dim = FeatureDimId(0);
        let word = Word::new(
            ProductionModalityId(0),
            [
                FormSegment::new([(dim, FeatureValueId(1))]),
                FormSegment::new([(dim, FeatureValueId(2))]),
            ],
        );
        let rule = FormChangeRule {
            dim,
            from: FeatureValueId(1),
            to: FeatureValueId(5),
            innovation_index: 0,
        };
        let changed = rule.apply(&word);
        assert_eq!(
            changed.segments()[0].features(),
            &[(dim, FeatureValueId(5))]
        );
        assert_eq!(
            changed.segments()[1].features(),
            &[(dim, FeatureValueId(2))]
        );
    }

    #[test]
    fn rule_order_decides_the_result_feeding_and_bleeding() {
        // Chained changes: A->B then B->C turns A into C; the reverse order leaves A as B,
        // because B->C runs before any B exists. The result is a function of relative
        // chronology, the R-LANG-DET ordering pin, with no environment needed.
        let dim = FeatureDimId(0);
        let (a, b, c) = (FeatureValueId(1), FeatureValueId(2), FeatureValueId(3));
        let word = Word::new(ProductionModalityId(0), [FormSegment::new([(dim, a)])]);
        let r_ab = FormChangeRule {
            dim,
            from: a,
            to: b,
            innovation_index: 0,
        };
        let r_bc = FormChangeRule {
            dim,
            from: b,
            to: c,
            innovation_index: 1,
        };
        let fed = r_bc.apply(&r_ab.apply(&word));
        let bled = r_ab.apply(&r_bc.apply(&word));
        assert_eq!(fed.segments()[0].features(), &[(dim, c)], "A->B->C feeds");
        assert_eq!(bled.segments()[0].features(), &[(dim, b)], "B->C bleeds");
        assert_ne!(fed, bled, "order changes the outcome");
    }

    #[test]
    fn a_lineage_innovates_deterministically_and_logs() {
        let (_s, forms) =
            ArticulationSubstrate::syllabic(["ka", "lo", "mi", "tu"].map(String::from), 2, 2);
        let params = DriftParams {
            sound_change_rate: Fixed::ONE,
            generation_ticks: 1,
        };
        let mut a = Language::new(LangId(0), forms.clone());
        let mut b = Language::new(LangId(0), forms);
        let key = Rng::for_coords(7, &[0, 1]);
        let ra = a.innovate(key, &params);
        let rb = b.innovate(key, &params);
        assert_eq!(ra, rb, "the same key innovates the same change");
        assert_eq!(ra.len(), 1, "rate one coins one change");
        assert_eq!(a.change_log().len(), 1, "the change is logged");
        assert_eq!(a.change_log()[0].innovation_index, 0);
    }

    #[test]
    fn a_fork_inherits_the_log_and_points_at_its_parent() {
        let (_s, forms) =
            ArticulationSubstrate::syllabic(["ka", "lo", "mi"].map(String::from), 2, 2);
        let params = DriftParams {
            sound_change_rate: Fixed::ONE,
            generation_ticks: 1,
        };
        let mut parent = Language::new(LangId(0), forms);
        parent.innovate(Rng::for_coords(1, &[1]), &params);
        let daughter = parent.fork(LangId(1));
        assert_eq!(daughter.parent(), Some(LangId(0)));
        assert_eq!(
            daughter.change_log(),
            parent.change_log(),
            "the daughter inherits the history"
        );
    }
}
