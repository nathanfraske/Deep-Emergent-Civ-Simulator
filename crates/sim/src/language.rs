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

/// A production modality definition: the feature dimensions its primitives are built from,
/// in id order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProductionModalityDef {
    /// The modality's registry id.
    pub id: ProductionModalityId,
    /// The feature dimensions a primitive in this modality contrasts over, sorted by id.
    pub dims: Vec<FeatureDimId>,
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

    /// How many concepts this mind has a word for.
    pub fn len(&self) -> usize {
        self.by_concept.len()
    }

    /// Whether the lexicon is empty.
    pub fn is_empty(&self) -> bool {
        self.by_concept.is_empty()
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
}
