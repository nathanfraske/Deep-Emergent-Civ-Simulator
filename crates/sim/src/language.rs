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
//! A word is a renderable surface form, not an opaque id: it is built by sampling a
//! data-defined character pool ([`CharacterPool`], the per-culture phonetic substrate of
//! design Part 33, including any custom characters), so different cultures' words look
//! unlike one another. Determinism by construction: the sample is drawn by counter-based
//! RNG keyed on the coiner, the concept, and the tick, never by hashing content (which the
//! open R-LANG-DET item flags as collision-prone), and every walk is id-ordered. The
//! deeper Part 33 pieces, concepts as regions over the semantic substrate, full
//! phonological generation, and sound-change drift, wait on the R-LANG-DET resolution that
//! pins their order-sensitive procedures; this layer deliberately stays within what is
//! deterministic without it.

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
/// region over the semantic substrate (design 33.1) is a later refinement.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ConceptId(pub u32);

/// An emergent word form: a surface string built from a culture's character pool. Two
/// cultures almost never build the same form for a concept, which is what makes their
/// lexicons diverge, and the form is renderable rather than an opaque id.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Word(pub String);

/// A data-defined pool of characters (or syllables) a culture builds its words from, with
/// the word-length range. The membership is data the owner provides (the phonetic
/// substrate of design Part 33), and an entry can be any string, so custom characters and
/// invented glyphs are denotable. Each culture or race can carry its own pool, which is
/// what makes their languages look unlike one another rather than only differ by id; the
/// dev fixtures supply a placeholder pool, never an authored inventory.
#[derive(Clone, Debug, Default)]
pub struct CharacterPool {
    chars: Vec<String>,
    min_len: u32,
    max_len: u32,
}

impl CharacterPool {
    /// Build a pool from its characters and the inclusive word-length range (in pool
    /// elements). An empty or degenerate range is clamped so coining always terminates.
    pub fn new(chars: impl IntoIterator<Item = String>, min_len: u32, max_len: u32) -> Self {
        let min_len = min_len.max(1);
        CharacterPool {
            chars: chars.into_iter().collect(),
            min_len,
            max_len: max_len.max(min_len),
        }
    }

    /// Whether the pool has no characters (coining then yields an empty word).
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    /// Coin a fresh word by sampling a length and then that many characters from the pool,
    /// each draw on a distinct counter of the supplied keyed RNG. Deterministic: the same
    /// key yields the same word on every machine, and a different pool or key yields a
    /// different surface form.
    pub fn coin(&self, rng: Rng) -> Word {
        if self.chars.is_empty() {
            return Word(String::new());
        }
        let span = self.max_len - self.min_len + 1;
        let len = self.min_len + rng.range_u32(0, span);
        let mut s = String::new();
        for i in 0..len {
            let idx = rng.range_u32(i as u64 + 1, self.chars.len() as u32) as usize;
            s.push_str(&self.chars[idx]);
        }
        Word(s)
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
    fn a_lexicon_adopts_and_realigns() {
        let c = ConceptId(7);
        let mut lex = Lexicon::new();
        assert_eq!(lex.word_for(c), None);
        lex.adopt(c, Word("kuna".to_string()));
        assert_eq!(lex.word_for(c), Some(&Word("kuna".to_string())));
        // realigning replaces the word.
        lex.adopt(c, Word("terva".to_string()));
        assert_eq!(lex.word_for(c), Some(&Word("terva".to_string())));
        assert_eq!(lex.len(), 1);
    }

    #[test]
    fn a_pool_coins_a_renderable_word_deterministically() {
        let pool = CharacterPool::new(["ka", "lo", "mi", "tu", "ne", "sa"].map(String::from), 2, 3);
        let rng = Rng::for_coords(0x5EED, &[1, 2, 3]);
        let w1 = pool.coin(rng);
        let w2 = pool.coin(rng);
        assert_eq!(w1, w2, "the same key coins the same word");
        assert!(
            !w1.0.is_empty(),
            "the word is a renderable string: {}",
            w1.0
        );
    }
}
