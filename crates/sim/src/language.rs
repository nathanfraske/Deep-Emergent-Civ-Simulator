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

//! Emergent lexicon: the naming game (design Part 33, the dawn coordination of 33.9).
//!
//! A concept is a meaning the engine tracks; a word is the surface form a culture
//! attaches to it. This module carries the minimal, demonstrable half of language
//! emergence: a band of agents coordinates a shared word per concept through repeated
//! interaction (Steels's naming game, Baronchelli's convergence), and two isolated bands
//! coordinate different words for the same concept, so the lexicon is emergent rather
//! than authored. A small innovation rate coins fresh variants, which is the seed of
//! drift and dialect split.
//!
//! Determinism by construction: a word form is minted by counter-based RNG keyed on the
//! coiner, the concept, and the tick, never by hashing content (which the open R-LANG-DET
//! item flags as collision-prone), and every walk is id-ordered. The deeper Part 33
//! pieces, concepts as regions over the semantic substrate, phonological generation, and
//! sound-change drift, wait on the R-LANG-DET resolution that pins their order-sensitive
//! procedures; this layer deliberately stays within what is deterministic without it.

use std::collections::BTreeMap;

use crate::calibration::{CalibrationError, CalibrationManifest};
use civsim_core::Fixed;

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
/// region over the semantic substrate (design 33.1) is a later refinement.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ConceptId(pub u32);

/// An emergent word form, minted deterministically. Two cultures almost never mint the
/// same form for a concept, which is what makes their lexicons diverge.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WordId(pub u64);

/// One mind's lexicon: the word it currently uses for each concept it has a word for.
#[derive(Clone, Debug, Default)]
pub struct Lexicon {
    by_concept: BTreeMap<ConceptId, WordId>,
}

impl Lexicon {
    /// An empty lexicon.
    pub fn new() -> Self {
        Lexicon::default()
    }

    /// The word this mind uses for a concept, if it has one.
    pub fn word_for(&self, concept: ConceptId) -> Option<WordId> {
        self.by_concept.get(&concept).copied()
    }

    /// Learn or realign: use this word for this concept from now on.
    pub fn adopt(&mut self, concept: ConceptId, word: WordId) {
        self.by_concept.insert(concept, word);
    }

    /// The concept-to-word pairs, in concept-id order, for a canonical walk.
    pub fn entries(&self) -> impl Iterator<Item = (&ConceptId, &WordId)> + '_ {
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
    fn a_lexicon_adopts_and_realigns() {
        let c = ConceptId(7);
        let mut lex = Lexicon::new();
        assert_eq!(lex.word_for(c), None);
        lex.adopt(c, WordId(100));
        assert_eq!(lex.word_for(c), Some(WordId(100)));
        // realigning replaces the word.
        lex.adopt(c, WordId(200));
        assert_eq!(lex.word_for(c), Some(WordId(200)));
        assert_eq!(lex.len(), 1);
    }
}
