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

//! The semantic-prime anchor set (design Part 33.1).
//!
//! The naming game coordinates a starter lexicon over a small set of the most salient
//! anchor concepts (33.9). Those anchors are the semantic primes of the Natural Semantic
//! Metalanguage (Wierzbicka and Goddard), the roughly sixty-five indefinable meanings
//! every documented language lexicalises, which is why they are the right universal floor
//! for a culture's first words.
//!
//! The prime concept inventory is the NSM anchor set the dawn dynamic coordinates first
//! (33.1, 33.9): a finite, universal, authored set of salient meanings, the innate
//! semantic content the substrate is grounded in. Each prime also carries an English gloss
//! lemma; the per-axis authored lemma is the "one sanctioned hardcoding" 33.2 names, and
//! the same justification applies here: the lemma is mechanism rather than world content,
//! it never feeds an agent's reasoning, and it surfaces only in the deterministic English
//! gist the renderer shows (every `nsm_gloss` caller is an example or test, never a
//! canonical path). The concept ids and lemmas here are a starting inventory authored once;
//! a data-file loader is the next increment, and the concept-as-substrate-region grounding
//! (so primes drift, split, and merge) is a later one. The words a culture coins for these
//! meanings remain fully emergent.

use crate::language::ConceptId;

/// A semantic prime: a universal anchor meaning and its English gloss lemma.
#[derive(Clone, Copy, Debug)]
pub struct Prime {
    pub concept: ConceptId,
    pub gloss: &'static str,
}

/// The NSM prime exponents, grouped by their semantic category. The list is the
/// roughly sixty-five primes of the current inventory; the gloss is the lemma, not a
/// claim about any culture's word.
const NSM_PRIMES: &[&str] = &[
    // Substantives.
    "I",
    "you",
    "someone",
    "something",
    "people",
    "body",
    // Relational substantives.
    "kind",
    "part",
    // Determiners.
    "this",
    "the same",
    "other",
    // Quantifiers.
    "one",
    "two",
    "some",
    "all",
    "many",
    "few",
    // Evaluators.
    "good",
    "bad",
    // Descriptors.
    "big",
    "small",
    // Mental predicates.
    "think",
    "know",
    "want",
    "don't want",
    "feel",
    "see",
    "hear",
    // Speech.
    "say",
    "words",
    "true",
    // Actions, events, movement.
    "do",
    "happen",
    "move",
    // Existence, possession.
    "be somewhere",
    "there is",
    "be someone",
    "mine",
    // Life and death.
    "live",
    "die",
    // Time.
    "when",
    "now",
    "before",
    "after",
    "a long time",
    "a short time",
    "for some time",
    "moment",
    // Space.
    "where",
    "here",
    "above",
    "below",
    "far",
    "near",
    "side",
    "inside",
    "touch",
    // Logical concepts.
    "not",
    "maybe",
    "can",
    "because",
    "if",
    // Intensifier, augmentor.
    "very",
    "more",
    // Similarity.
    "like",
];

/// The full prime inventory as concept-and-gloss pairs. Concept ids are assigned
/// `1..=len` in declaration order, a stable starting assignment.
pub fn nsm_primes() -> Vec<Prime> {
    NSM_PRIMES
        .iter()
        .enumerate()
        .map(|(i, &gloss)| Prime {
            concept: ConceptId(i as u32 + 1),
            gloss,
        })
        .collect()
}

/// Just the concept ids of the prime set, for `World::set_concepts`.
pub fn nsm_concept_ids() -> Vec<ConceptId> {
    (1..=NSM_PRIMES.len() as u32).map(ConceptId).collect()
}

/// The English gloss lemma of a prime concept, or `None` if the id is not a prime.
pub fn nsm_gloss(c: ConceptId) -> Option<&'static str> {
    let idx = c.0.checked_sub(1)? as usize;
    NSM_PRIMES.get(idx).copied()
}

/// The number of primes in the inventory.
pub fn nsm_prime_count() -> usize {
    NSM_PRIMES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inventory_is_the_prime_set() {
        // The NSM inventory is around sixty-five primes; this list is exactly sixty-five.
        assert_eq!(nsm_prime_count(), 65);
        assert_eq!(nsm_primes().len(), 65);
        assert_eq!(nsm_concept_ids().len(), 65);
    }

    #[test]
    fn concept_ids_are_distinct_and_one_based() {
        let ids = nsm_concept_ids();
        assert_eq!(ids.first().copied(), Some(ConceptId(1)));
        assert_eq!(ids.last().copied(), Some(ConceptId(65)));
        let mut sorted = ids.clone();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "no duplicate concept ids");
    }

    #[test]
    fn gloss_round_trips_and_rejects_non_primes() {
        assert_eq!(nsm_gloss(ConceptId(1)), Some("I"));
        assert_eq!(nsm_gloss(ConceptId(65)), Some("like"));
        assert_eq!(nsm_gloss(ConceptId(0)), None, "zero is not a prime id");
        assert_eq!(
            nsm_gloss(ConceptId(66)),
            None,
            "past the end is not a prime"
        );
    }

    #[test]
    fn every_prime_has_a_nonempty_gloss() {
        for prime in nsm_primes() {
            assert!(
                !prime.gloss.is_empty(),
                "prime {:?} has an empty gloss",
                prime.concept
            );
        }
    }
}
