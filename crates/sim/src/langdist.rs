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

//! The language-distance layer: how far two languages have diverged (design Part 33.5).
//!
//! Distance over a structured space, reusing the value-distance machinery (Part 21) rather than a
//! new metric. It has three fixed-point components:
//!
//! - [`lexical_distance`] over two lexicons: the share of non-cognate (replaced or one-sided) forms.
//!   A concept both cultures hold whose forms sit within the correspondence tie window (the
//!   [`crate::semantics::substrate_quantization`] the caller passes) is cognate; a replaced form or a
//!   concept one side lacks is non-cognate. Two cases carry no separate authored constant: a concept
//!   one side lacks (one-sided coverage) counts as fully non-cognate, and a form-disjoint or
//!   cross-modality pair (whose forms can never correspond) reaches the ceiling of one, both the
//!   normalised language image of [`crate::value::incommensurability_ceiling`] (the untranslatable
//!   magnitude) rather than a distinct number.
//! - [`phonological_distance`] over two form systems: the feature-based Jaccard distance between the
//!   two producible inventories (a form-disjoint or cross-modality pair reaching one), so sounds one
//!   side cannot make contribute to a phonetic incommensurability.
//! - the grammatical component: the existing [`crate::typology::typology_distance`] over the
//!   typological parameter vector, supplied by the caller.
//!
//! [`language_distance`] combines the three under [`ComponentWeights`]. The weights are not the
//! authored 0.6/0.25/0.15: they DERIVE from each layer's own order-2 diversity across a corpus
//! ([`distance_component_weights`]), the same Hill-number measure [`crate::typology::information_weights`]
//! uses, normalised to sum to exactly one by deterministic residual absorption into the lexical
//! component (R-LANG-DET). Lexical distance dominating mutual intelligibility EMERGES from the
//! entropy of real lexicons (a lexicon carries far more effectively-distinct forms than a form
//! inventory carries phonemes or a grammar carries settings) rather than being asserted.
//!
//! Everything is integer fixed-point and draws no randomness (Principle 3), keys off forms,
//! inventories, and typology data rather than a race identifier (Principle 9), and is invariant to
//! permuting which language is which (each measure reads a symmetric pooled multiset).

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::Fixed;

use crate::language::{FormSegment, FormSystem, Lexicon, Word};
use crate::typology::{TypologyParamId, TypologyProfile, TypologyValueId};

/// The normalised form distance between two word-forms, in `[0, ONE]`. Two forms in different
/// production modalities can never correspond, so they read the ceiling of one (the form-disjoint /
/// cross-modality case); within one modality it is the share of primitive positions that differ over
/// the longer form, so exact forms read zero and wholly different forms read one. A pure function.
fn form_distance(a: &Word, b: &Word) -> Fixed {
    if a.modality() != b.modality() {
        return Fixed::ONE;
    }
    let max = a.segments().len().max(b.segments().len());
    if max == 0 {
        return Fixed::ZERO;
    }
    let mut differing = 0i32;
    for i in 0..max {
        if a.segments().get(i) != b.segments().get(i) {
            differing += 1;
        }
    }
    Fixed::from_int(differing)
        .checked_div(Fixed::from_int(max as i32))
        .unwrap_or(Fixed::ONE)
}

/// The lexical distance between two lexicons: the share of non-cognate (replaced or one-sided) forms
/// over the union of concepts either holds (design Part 33.5). A concept both hold is cognate when
/// its two forms sit within `cognate_window` (the correspondence tie window, the
/// [`crate::semantics::substrate_quantization`]); a replaced form beyond the window, and a concept
/// one side lacks, count as non-cognate. The one-sided contribution and the all-non-cognate ceiling
/// are the normalised language image of [`crate::value::incommensurability_ceiling`], not a separate
/// authored constant: a form-disjoint pair reaches one because every cell is non-cognate. Symmetric
/// under swapping the two lexicons; the walk is over concept ids in canonical order; no race
/// identifier enters. Two empty lexicons read zero.
pub fn lexical_distance(a: &Lexicon, b: &Lexicon, cognate_window: Fixed) -> Fixed {
    let mut concepts: BTreeSet<_> = BTreeSet::new();
    for (c, _) in a.entries() {
        concepts.insert(*c);
    }
    for (c, _) in b.entries() {
        concepts.insert(*c);
    }
    if concepts.is_empty() {
        return Fixed::ZERO;
    }
    let mut non_cognate = 0i32;
    for c in &concepts {
        match (a.word_for(*c), b.word_for(*c)) {
            (Some(wa), Some(wb)) => {
                if form_distance(wa, wb) > cognate_window {
                    non_cognate += 1;
                }
            }
            // One-sided coverage: a concept one side lacks is fully non-cognate, the per-cell
            // incommensurability contribution (the untranslatable magnitude), never dropped.
            _ => non_cognate += 1,
        }
    }
    Fixed::from_int(non_cognate)
        .checked_div(Fixed::from_int(concepts.len() as i32))
        .unwrap_or(Fixed::ZERO)
}

/// The phonological distance between two form systems: the feature-based Jaccard distance between
/// their producible primitive inventories (design Part 33.5), in `[0, ONE]`. Two systems in
/// different modalities are form-disjoint and read the ceiling of one; within one modality it is the
/// symmetric-difference share over the union, so identical inventories read zero and disjoint ones
/// read one. A primitive one side cannot make lands in the symmetric difference, the phonetic
/// incommensurability. Symmetric; no race identifier; two empty inventories read zero.
pub fn phonological_distance(a: &FormSystem, b: &FormSystem) -> Fixed {
    if a.modality() != b.modality() {
        return Fixed::ONE;
    }
    let sa: BTreeSet<&FormSegment> = a.inventory().iter().collect();
    let sb: BTreeSet<&FormSegment> = b.inventory().iter().collect();
    let union = sa.union(&sb).count();
    if union == 0 {
        return Fixed::ZERO;
    }
    let intersection = sa.intersection(&sb).count();
    let sym_diff = union - intersection;
    Fixed::from_int(sym_diff as i32)
        .checked_div(Fixed::from_int(union as i32))
        .unwrap_or(Fixed::ZERO)
}

/// The three language-distance component weights, summing to exactly [`Fixed::ONE`] (design Part
/// 33.5). Derived from each layer's own entropy by [`distance_component_weights`], never authored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ComponentWeights {
    /// The lexical component's weight (the residual absorber, so the three sum to exactly one).
    pub lexical: Fixed,
    /// The phonological component's weight.
    pub phonological: Fixed,
    /// The grammatical component's weight.
    pub grammatical: Fixed,
}

/// Combine the three component distances into one language distance under derived weights (design
/// Part 33.5). Mutual intelligibility is then a decreasing function of this combined distance,
/// consumed as a friction on cross-language transmission (Parts 9, 23). A pure function of the three
/// component distances and the weights.
pub fn language_distance(
    lexical: Fixed,
    phonological: Fixed,
    grammatical: Fixed,
    weights: &ComponentWeights,
) -> Fixed {
    Fixed::saturating_sum([
        weights.lexical.mul(lexical),
        weights.phonological.mul(phonological),
        weights.grammatical.mul(grammatical),
    ])
}

/// The order-2 diversity (Hill number `N^2 / sum(c_i^2)`, the inverse-Simpson effective count) of a
/// set of integer counts, read as `Fixed` bits, the same measure [`crate::typology::information_weights`]
/// uses. Integer-exact and deterministic: `N` and `sum(c_i^2)` accumulate in `u128`, so the result
/// is bit-identical across machines; an all-zero (or empty) count set has no diversity and reads
/// zero, never a fabricated one. Saturates at [`Fixed::MAX`] for an astronomically diverse corpus.
fn order2_diversity(counts: impl IntoIterator<Item = u64>) -> Fixed {
    let mut n: u128 = 0;
    let mut sum_sq: u128 = 0;
    for c in counts {
        let c = c as u128;
        n = n.saturating_add(c);
        sum_sq = sum_sq.saturating_add(c.saturating_mul(c));
    }
    if sum_sq == 0 {
        return Fixed::ZERO;
    }
    let n_sq = n.saturating_mul(n);
    let shifted = n_sq.checked_shl(32).unwrap_or(u128::MAX);
    let bits = shifted / sum_sq;
    Fixed::from_bits_i128(bits.min(i64::MAX as u128) as i128).unwrap_or(Fixed::MAX)
}

/// Count each distinct key's occurrences across an iterator, in a canonical (sorted) map.
fn tally<K: Ord>(items: impl IntoIterator<Item = K>) -> BTreeMap<K, u64> {
    let mut counts: BTreeMap<K, u64> = BTreeMap::new();
    for k in items {
        *counts.entry(k).or_insert(0) += 1;
    }
    counts
}

/// Derive the three language-distance component weights from a corpus (design Part 33.5), so the
/// authored 0.6/0.25/0.15 is retired. Each layer's weight is its own order-2 diversity across the
/// corpus divided by the total: the lexical layer over the pooled word-forms of every lexicon, the
/// phonological layer over the pooled producible primitives of every form system, and the
/// grammatical layer over the pooled `(parameter, value)` settings of every typology profile. The
/// weights are normalised to sum to exactly [`Fixed::ONE`] by deterministic residual absorption into
/// the lexical component (the R-LANG-DET rule), so the fixed-point rounding lands in one place and
/// the three always sum to one. Lexical dominance EMERGES from the data (a real lexicon carries far
/// more effectively-distinct forms than a form inventory carries phonemes), never asserted. A pure
/// function of the corpus; invariant to permuting which language is which (each diversity reads a
/// pooled multiset); no race identifier enters. A corpus with no distinguishing entropy in any layer
/// gives the lexical component the whole weight (the residual), never a fabricated split.
pub fn distance_component_weights(
    lexicons: &[Lexicon],
    form_systems: &[FormSystem],
    typologies: &[TypologyProfile],
) -> ComponentWeights {
    let lex_counts = tally(
        lexicons
            .iter()
            .flat_map(|l| l.entries().map(|(_, w)| w.clone())),
    );
    let div_lex = order2_diversity(lex_counts.into_values());

    let phon_counts = tally(
        form_systems
            .iter()
            .flat_map(|f| f.inventory().iter().cloned()),
    );
    let div_phon = order2_diversity(phon_counts.into_values());

    let gram_counts: BTreeMap<(TypologyParamId, TypologyValueId), u64> =
        tally(typologies.iter().flat_map(|t| t.entries().iter().copied()));
    let div_gram = order2_diversity(gram_counts.into_values());

    let total = Fixed::saturating_sum([div_lex, div_phon, div_gram]);
    if total <= Fixed::ZERO {
        // No distinguishing entropy anywhere: the lexical component (the dominant layer by design)
        // takes the whole weight through the residual, never a fabricated even split.
        return ComponentWeights {
            lexical: Fixed::ONE,
            phonological: Fixed::ZERO,
            grammatical: Fixed::ZERO,
        };
    }
    let phonological = div_phon.checked_div(total).unwrap_or(Fixed::ZERO);
    let grammatical = div_gram.checked_div(total).unwrap_or(Fixed::ZERO);
    // Residual absorption into the lexical component: the three sum to exactly one.
    let lexical = Fixed::ONE - phonological - grammatical;
    ComponentWeights {
        lexical,
        phonological,
        grammatical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::{ConceptId, FeatureDimId, FeatureValueId, ProductionModalityId};

    fn seg(v: u32) -> FormSegment {
        FormSegment::new([(FeatureDimId(0), FeatureValueId(v))])
    }

    fn word(modality: u32, primitives: &[u32]) -> Word {
        Word::new(
            ProductionModalityId(modality),
            primitives.iter().map(|&v| seg(v)),
        )
    }

    fn lexicon(modality: u32, entries: &[(u32, &[u32])]) -> Lexicon {
        let mut lex = Lexicon::new();
        for &(c, prims) in entries {
            lex.adopt(ConceptId(c), word(modality, prims));
        }
        lex
    }

    fn form_system(modality: u32, primitives: &[u32]) -> FormSystem {
        FormSystem::new(
            ProductionModalityId(modality),
            primitives.iter().map(|&v| seg(v)),
            1,
            2,
        )
    }

    fn typology(entries: &[(u32, u32)]) -> TypologyProfile {
        TypologyProfile::new(
            entries
                .iter()
                .map(|&(p, v)| (TypologyParamId(p), TypologyValueId(v)))
                .collect(),
        )
    }

    #[test]
    fn lexical_distance_reads_the_share_of_non_cognate_forms() {
        let window = Fixed::ZERO; // exact-form cognates only
                                  // Identical lexicons: nothing replaced, distance zero.
        let a = lexicon(0, &[(0, &[1, 2]), (1, &[3])]);
        let same = lexicon(0, &[(0, &[1, 2]), (1, &[3])]);
        assert_eq!(lexical_distance(&a, &same, window), Fixed::ZERO);
        // One of two forms replaced: half non-cognate.
        let b = lexicon(0, &[(0, &[1, 2]), (1, &[9])]);
        assert_eq!(lexical_distance(&a, &b, window), Fixed::from_ratio(1, 2));
        // A concept one side lacks (one-sided coverage) counts as non-cognate: over the union of
        // three concepts, one is one-sided, so the distance is one third even with the rest cognate.
        let c = lexicon(0, &[(0, &[1, 2]), (1, &[3]), (2, &[7])]);
        assert_eq!(lexical_distance(&a, &c, window), Fixed::from_ratio(1, 3));
    }

    #[test]
    fn form_disjoint_lexicons_reach_the_no_shared_form_ceiling() {
        // Different modalities: no form can correspond, so every shared concept is non-cognate and
        // the distance is the ceiling of one, emergent rather than a separate authored constant.
        let a = lexicon(0, &[(0, &[1]), (1, &[2])]);
        let b = lexicon(1, &[(0, &[1]), (1, &[2])]);
        assert_eq!(lexical_distance(&a, &b, Fixed::ZERO), Fixed::ONE);
    }

    #[test]
    fn phonological_distance_is_the_inventory_jaccard() {
        // Identical inventories: distance zero.
        let a = form_system(0, &[1, 2, 3]);
        let same = form_system(0, &[1, 2, 3]);
        assert_eq!(phonological_distance(&a, &same), Fixed::ZERO);
        // Share {1,2}, differ on {3} vs {4}: union {1,2,3,4}, sym diff {3,4}, distance 2/4 = 1/2.
        let b = form_system(0, &[1, 2, 4]);
        assert_eq!(phonological_distance(&a, &b), Fixed::from_ratio(1, 2));
        // Different modalities are form-disjoint: the ceiling of one.
        let cross = form_system(1, &[1, 2, 3]);
        assert_eq!(phonological_distance(&a, &cross), Fixed::ONE);
    }

    #[test]
    fn component_weights_derive_from_entropy_sum_to_one_and_differ_by_corpus() {
        // Two pairs share the same form systems and typologies (so div_phon and div_gram match), and
        // differ only in lexical entropy, so the weights must differ from the lexicons' own entropy.
        let fs = [form_system(0, &[1, 2]), form_system(0, &[1, 2])];
        let typ = [typology(&[(0, 0)]), typology(&[(0, 0)])];

        // Pair 1: every form distinct across the two lexicons (high lexical diversity).
        let hi = [
            lexicon(0, &[(0, &[1]), (1, &[2]), (2, &[3])]),
            lexicon(0, &[(0, &[4]), (1, &[5]), (2, &[6])]),
        ];
        // Pair 2: every form the same (low lexical diversity).
        let lo = [
            lexicon(0, &[(0, &[1]), (1, &[1]), (2, &[1])]),
            lexicon(0, &[(0, &[1]), (1, &[1]), (2, &[1])]),
        ];

        let w_hi = distance_component_weights(&hi, &fs, &typ);
        let w_lo = distance_component_weights(&lo, &fs, &typ);

        // Each pair's weights sum to exactly one (residual absorption).
        for w in [w_hi, w_lo] {
            assert_eq!(
                Fixed::saturating_sum([w.lexical, w.phonological, w.grammatical]),
                Fixed::ONE,
                "the weights sum to exactly one"
            );
        }
        // The high-entropy lexicon corpus weights the lexical component more heavily.
        assert!(
            w_hi.lexical > w_lo.lexical,
            "lexical dominance emerges from lexical entropy, not an authored constant"
        );
        assert_ne!(
            w_hi, w_lo,
            "two corpora with different entropy get different component weights"
        );
    }

    #[test]
    fn component_weights_are_invariant_to_permuting_language_labels() {
        let fs = [form_system(0, &[1, 2]), form_system(0, &[3, 4])];
        let typ = [typology(&[(0, 0)]), typology(&[(0, 1)])];
        let lex = [
            lexicon(0, &[(0, &[1]), (1, &[2])]),
            lexicon(0, &[(0, &[3]), (1, &[4])]),
        ];
        let forward = distance_component_weights(&lex, &fs, &typ);
        // Swap which language is first in every corpus slice: the pooled multisets are unchanged.
        let lex_rev = [lex[1].clone(), lex[0].clone()];
        let fs_rev = [fs[1].clone(), fs[0].clone()];
        let typ_rev = [typ[1].clone(), typ[0].clone()];
        let reversed = distance_component_weights(&lex_rev, &fs_rev, &typ_rev);
        assert_eq!(
            forward, reversed,
            "permuting the language labels leaves the derived weights invariant"
        );
    }

    #[test]
    fn an_entropyless_corpus_gives_lexical_the_whole_weight() {
        // No lexicons, no form systems, no typologies: no distinguishing entropy, so the lexical
        // component takes the whole weight through the residual, never a fabricated even split.
        let w = distance_component_weights(&[], &[], &[]);
        assert_eq!(w.lexical, Fixed::ONE);
        assert_eq!(w.phonological, Fixed::ZERO);
        assert_eq!(w.grammatical, Fixed::ZERO);
    }

    #[test]
    fn language_distance_combines_the_three_components_under_the_weights() {
        // Half weight to lexical, quarter each to the others: 0.5*1 + 0.25*0 + 0.25*0.5 = 0.625.
        let weights = ComponentWeights {
            lexical: Fixed::from_ratio(1, 2),
            phonological: Fixed::from_ratio(1, 4),
            grammatical: Fixed::from_ratio(1, 4),
        };
        let d = language_distance(Fixed::ONE, Fixed::ZERO, Fixed::from_ratio(1, 2), &weights);
        assert_eq!(d, Fixed::from_ratio(5, 8));
    }
}
