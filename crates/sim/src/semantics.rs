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

//! The semantic substrate: concepts as points over a meaning ground metric (design Part 33.1).
//!
//! A concept is a meaning region, anchored by a prototype: a point over the axes of a meaning
//! [`GroundMetric`], sibling to the value substrate a race carries over its value axes (Part 21).
//! The substrate is a [`ConceptId`]-to-[`Concept`] map, membership data the world grows rather than
//! an authored human concept list (Principle 11): two races carry different prototypes and diverge
//! in what they can tell apart, and nothing here reads a race identifier (Principle 9).
//!
//! From the geometry two thresholds and one quantization step DERIVE, so they stop being authored
//! numbers:
//!
//! - the discrimination threshold, the finest meaning gap two concepts are held apart by: the
//!   minimum positive inter-meaning distance the substrate exhibits, floored by the perceptual
//!   just-noticeable difference (the sensorium resolution [`crate::langmod::perceptual_geometry`]
//!   reads), because a gap below what the channel resolves cannot be discriminated;
//! - the lexicalisation threshold, the coarsest meaning separation a dedicated form is spent on:
//!   the occupied meaning span spread over the channel's contrast budget (how many distinguishable
//!   forms the channel affords), floored by the discrimination threshold, because a distinction
//!   finer than discrimination cannot be lexicalised distinctly;
//! - the substrate quantization step, a small integer fraction of the discrimination threshold: the
//!   coarsest grid that still keeps two concepts one discrimination-threshold apart in distinct
//!   cells is half that threshold (the separability bound), one knob reused for the distance walk,
//!   the walk key, and the correspondence tie window, consistent with the ground-metric granularity
//!   ([`GroundMetric::resolution`]).
//!
//! Everything is integer fixed-point and draws no randomness (Principle 3): the inter-meaning
//! distances, the two thresholds, and the quantization step are pure functions of the prototypes
//! and the two perceptual inputs, walked in [`ConceptId`] order. The point-to-point distance is the
//! pinned Euclidean form the metric reduces to for independent meaning axes (exactly as
//! [`civsim_foundation::value::euclidean_distance`] over value axes); a structured meaning metric weights it,
//! the same flagged seam [`civsim_foundation::value::value_distance`] leaves open, so the separation and span
//! read `None` there rather than run on an invented weighted metric.

use std::collections::BTreeMap;

use civsim_core::Fixed;

use crate::language::ConceptId;
use civsim_foundation::value::{GroundMetric, StructureKind};

/// A concept: a meaning region anchored by a prototype point over the meaning ground metric's axes
/// (design Part 33.1). The prototype is the region's centre; the neighbourhood within the
/// discrimination threshold is the region it owns. Held as data, never an authored gloss.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Concept {
    /// The prototype: a magnitude on each meaning axis, a point over the meaning ground metric.
    pub prototype: Vec<Fixed>,
}

impl Concept {
    /// A concept at the given prototype point.
    pub fn new(prototype: impl IntoIterator<Item = Fixed>) -> Self {
        Concept {
            prototype: prototype.into_iter().collect(),
        }
    }
}

/// The two derived concept thresholds over a semantic substrate: the discrimination threshold (the
/// finest meaning gap two concepts are held apart by) and the lexicalisation threshold (the coarsest
/// separation a dedicated form is spent on). Both DERIVE from the substrate geometry and the two
/// perceptual inputs (design Part 33.1); neither is an authored number.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ConceptThresholds {
    /// The discrimination threshold: the minimum positive inter-meaning distance, floored by the
    /// perceptual just-noticeable difference.
    pub discrimination: Fixed,
    /// The lexicalisation threshold: the meaning span spread over the contrast budget, floored by
    /// the discrimination threshold.
    pub lexicalisation: Fixed,
}

/// The denominator of the substrate quantization step: the coarsest grid that keeps two concepts
/// one discrimination-threshold apart in distinct cells is half that threshold (the separability
/// bound, a sampling fact rather than an owner tuneable). A small integer fraction, so the quantized
/// walk never merges two discriminable concepts into one cell.
pub const QUANTIZATION_DIVISOR: i32 = 2;

/// The semantic substrate: the meaning ground metric plus the [`ConceptId`]-to-[`Concept`] map
/// (design Part 33.1). The metric is the meaning space's structure; the concepts are points in it.
#[derive(Clone, Debug)]
pub struct SemanticSubstrate {
    metric: GroundMetric,
    concepts: BTreeMap<ConceptId, Concept>,
}

impl SemanticSubstrate {
    /// An empty substrate over a meaning ground metric.
    pub fn new(metric: GroundMetric) -> Self {
        SemanticSubstrate {
            metric,
            concepts: BTreeMap::new(),
        }
    }

    /// Register (or replace) a concept at an id.
    pub fn insert(&mut self, id: ConceptId, concept: Concept) {
        self.concepts.insert(id, concept);
    }

    /// The meaning ground metric this substrate's prototypes live over.
    pub fn metric(&self) -> &GroundMetric {
        &self.metric
    }

    /// The concepts, in [`ConceptId`] order, for a canonical walk.
    pub fn concepts(&self) -> impl Iterator<Item = (&ConceptId, &Concept)> + '_ {
        self.concepts.iter()
    }

    /// How many concepts the substrate holds.
    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    /// Whether the substrate holds no concepts.
    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    /// The Euclidean inter-meaning distance between two prototypes over their shared components, the
    /// pinned form the metric reduces to for independent meaning axes (mirroring
    /// [`civsim_foundation::value::euclidean_distance`]). Computed with the deterministic [`Fixed::sqrt`].
    fn prototype_distance(a: &[Fixed], b: &[Fixed]) -> Fixed {
        let n = a.len().min(b.len());
        let mut acc = Fixed::ZERO;
        for i in 0..n {
            let d = a[i] - b[i];
            acc += d.mul(d);
        }
        acc.sqrt()
    }

    /// The minimum positive inter-meaning distance the substrate exhibits, or `None` when there are
    /// fewer than two concepts (no pair) or the meaning metric is structured (the flagged weighted
    /// seam, left open rather than run on an invented metric). The walk is over [`ConceptId`]-ordered
    /// pairs, so it is deterministic; two coincident prototypes contribute no positive gap.
    pub fn min_positive_separation(&self) -> Option<Fixed> {
        if self.metric.kind() != StructureKind::Independent {
            return None;
        }
        let points: Vec<&Vec<Fixed>> = self.concepts.values().map(|c| &c.prototype).collect();
        let mut best: Option<Fixed> = None;
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let d = Self::prototype_distance(points[i], points[j]);
                if d > Fixed::ZERO {
                    best = Some(match best {
                        Some(b) => b.min(d),
                        None => d,
                    });
                }
            }
        }
        best
    }

    /// The occupied meaning span: the maximum inter-meaning distance any two concepts sit apart, or
    /// `None` under the same fewer-than-two or structured-metric conditions as
    /// [`Self::min_positive_separation`]. The extent the contrast budget is spread over.
    pub fn meaning_span(&self) -> Option<Fixed> {
        if self.metric.kind() != StructureKind::Independent {
            return None;
        }
        let points: Vec<&Vec<Fixed>> = self.concepts.values().map(|c| &c.prototype).collect();
        if points.len() < 2 {
            return None;
        }
        let mut span = Fixed::ZERO;
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let d = Self::prototype_distance(points[i], points[j]);
                if d > span {
                    span = d;
                }
            }
        }
        Some(span)
    }
}

/// Derive the concept discrimination and lexicalisation thresholds over a substrate (design Part
/// 33.1). The discrimination threshold is the minimum positive inter-meaning distance the substrate
/// exhibits, floored by the perceptual just-noticeable difference `jnd` (the sensorium resolution
/// [`crate::langmod::perceptual_geometry`] reads): a gap below what the channel resolves cannot be
/// discriminated, so the threshold cannot fall below the `jnd`; a substrate with no positive pair
/// falls to the `jnd` floor. The lexicalisation threshold is the occupied meaning span spread over
/// the channel's `contrast_budget` (the count of distinguishable forms
/// [`crate::langmod::PerceptualGeometry::contrast_budget`] yields), floored by the discrimination
/// threshold: a channel affording few contrasts spends a dedicated form only on a well-separated
/// meaning, and a distinction finer than discrimination cannot be lexicalised distinctly. A pure
/// function of the prototypes and the two perceptual scalars; no race identifier enters.
pub fn concept_thresholds(
    substrate: &SemanticSubstrate,
    jnd: Fixed,
    contrast_budget: Fixed,
) -> ConceptThresholds {
    let floor = jnd.max(Fixed::ZERO);
    let discrimination = match substrate.min_positive_separation() {
        Some(sep) => sep.max(floor),
        None => floor,
    };
    let lexicalisation = match substrate.meaning_span() {
        Some(span) => match span.checked_div(contrast_budget) {
            Some(per_contrast) => per_contrast.max(discrimination),
            None => discrimination,
        },
        None => discrimination,
    };
    ConceptThresholds {
        discrimination,
        lexicalisation,
    }
}

/// Derive the substrate quantization step: a small integer fraction ([`QUANTIZATION_DIVISOR`]) of
/// the discrimination threshold (design Part 33.1). The coarsest grid that still keeps two concepts
/// one discrimination-threshold apart in distinct cells is half that threshold (the separability
/// bound), the one knob reused for the meaning-distance walk, the walk key, and the correspondence
/// tie window, and consistent with the ground-metric granularity ([`GroundMetric::resolution`]). A
/// pure function of the derived threshold; no owner number and no race identifier.
pub fn substrate_quantization(thresholds: &ConceptThresholds) -> Fixed {
    thresholds
        .discrimination
        .checked_div(Fixed::from_int(QUANTIZATION_DIVISOR))
        .unwrap_or(Fixed::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_foundation::value::ValueStructure;

    fn independent_substrate(concepts: &[(u32, &[i32])]) -> SemanticSubstrate {
        let k = concepts.iter().map(|(_, p)| p.len()).max().unwrap_or(0);
        let metric = GroundMetric::compile(&ValueStructure::Independent { k });
        let mut s = SemanticSubstrate::new(metric);
        for &(id, proto) in concepts {
            s.insert(
                ConceptId(id),
                Concept::new(proto.iter().map(|&c| Fixed::from_int(c))),
            );
        }
        s
    }

    #[test]
    fn min_positive_separation_is_the_finest_inter_meaning_gap() {
        // Three concepts on a line at 0, 3, 5: the gaps are 3, 2, 5; the finest is 2.
        let s = independent_substrate(&[(0, &[0, 0]), (1, &[3, 0]), (2, &[5, 0])]);
        assert_eq!(s.min_positive_separation(), Some(Fixed::from_int(2)));
        // The occupied span is the widest gap, 5.
        assert_eq!(s.meaning_span(), Some(Fixed::from_int(5)));
    }

    #[test]
    fn two_races_with_different_prototypes_get_different_concept_thresholds() {
        // Same perceptual inputs for both, so any threshold difference comes from the prototypes.
        let jnd = Fixed::from_ratio(1, 100);
        let contrast_budget = Fixed::from_int(4);
        // Race A: a (3, 4) right triangle from the origin, nearest gap 5.
        let a = independent_substrate(&[(0, &[0, 0]), (1, &[3, 4])]);
        // Race B: two concepts 8 apart, a coarser meaning geometry.
        let b = independent_substrate(&[(0, &[0, 0]), (1, &[8, 0])]);
        let ta = concept_thresholds(&a, jnd, contrast_budget);
        let tb = concept_thresholds(&b, jnd, contrast_budget);
        assert_eq!(
            ta.discrimination,
            Fixed::from_int(5),
            "A's nearest gap is 5"
        );
        assert_eq!(
            tb.discrimination,
            Fixed::from_int(8),
            "B's nearest gap is 8"
        );
        assert_ne!(
            ta.discrimination, tb.discrimination,
            "two races with different meaning prototypes discriminate at different thresholds"
        );
        assert_ne!(
            ta.lexicalisation, tb.lexicalisation,
            "and lexicalise at different thresholds"
        );
    }

    #[test]
    fn discrimination_is_floored_by_the_perceptual_jnd() {
        // Two concepts only 1 apart, but a jnd of 3: the channel cannot resolve below 3, so the
        // discrimination threshold is lifted to the jnd, never the sub-jnd geometric gap.
        let s = independent_substrate(&[(0, &[0, 0]), (1, &[1, 0])]);
        let t = concept_thresholds(&s, Fixed::from_int(3), Fixed::from_int(2));
        assert_eq!(t.discrimination, Fixed::from_int(3), "floored by the jnd");
    }

    #[test]
    fn lexicalisation_spreads_the_span_over_the_contrast_budget() {
        // Span 8, contrast budget 2: each contrast must cover 4 of meaning, above the discrimination
        // floor (the nearest gap here is 8), so the lexicalisation threshold is the discrimination
        // floor, not the finer span/budget.
        let s = independent_substrate(&[(0, &[0, 0]), (1, &[8, 0])]);
        let t = concept_thresholds(&s, Fixed::from_ratio(1, 100), Fixed::from_int(2));
        // span/budget = 8/2 = 4, discrimination = 8, so lexicalisation floors at 8.
        assert_eq!(t.lexicalisation, Fixed::from_int(8));
        // With a wider inventory the finer span/budget can drop the threshold below discrimination's
        // floor only when the span is larger than the nearest gap; here span == nearest gap, so a
        // bigger budget still floors at discrimination.
        let t_wide = concept_thresholds(&s, Fixed::from_ratio(1, 100), Fixed::from_int(16));
        assert_eq!(t_wide.lexicalisation, Fixed::from_int(8));
    }

    #[test]
    fn quantization_is_a_fixed_fraction_of_the_discrimination_threshold() {
        let jnd = Fixed::from_ratio(1, 100);
        let s = independent_substrate(&[(0, &[0, 0]), (1, &[6, 0])]);
        let t = concept_thresholds(&s, jnd, Fixed::from_int(4));
        let q = substrate_quantization(&t);
        assert_eq!(
            q,
            t.discrimination.div(Fixed::from_int(QUANTIZATION_DIVISOR)),
            "the quantization is a fixed fraction of the threshold"
        );
        assert_eq!(q, Fixed::from_int(3), "6 / 2 = 3");
        // Two concepts one discrimination-threshold apart never share a quantized cell: the step is
        // strictly less than the threshold.
        assert!(q < t.discrimination);
    }

    #[test]
    fn a_structured_meaning_metric_is_the_flagged_seam() {
        // A graph meaning metric is the weighted seam value_distance also leaves open: the
        // separation and span read None rather than run on an invented weighted metric.
        let metric = GroundMetric::compile(&ValueStructure::Graph {
            k: 2,
            edges: vec![],
        });
        let mut s = SemanticSubstrate::new(metric);
        s.insert(ConceptId(0), Concept::new([Fixed::ZERO, Fixed::ZERO]));
        s.insert(
            ConceptId(1),
            Concept::new([Fixed::from_int(3), Fixed::ZERO]),
        );
        assert_eq!(s.min_positive_separation(), None);
        assert_eq!(s.meaning_span(), None);
        // The thresholds fall back to the perceptual floor, never a fabricated weighted distance.
        let t = concept_thresholds(&s, Fixed::from_int(2), Fixed::from_int(4));
        assert_eq!(t.discrimination, Fixed::from_int(2));
    }
}
