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

//! Axiomatic beliefs: representation and the single-mind update kernel (design Part 28, the
//! resolved R-AXIOM work, record 62.4).
//!
//! A race begins at the dawn of sentience with intrinsic beliefs: a value profile (Part 21),
//! a small set of axioms about the world, the self, others, and the sacred, and an epistemic
//! stance for how it decides what is true. An axiom is not one number: position (where the
//! stance sits) and hold (how hard it is to move) are separate, so two races can both hold
//! that outsiders are kin, one as an unquestioned axiom and one as a revisable hunch. The
//! mechanism here is one fixed, audited kernel; the axiom axes, the source modes, the
//! domains, and every coefficient are per-race data (Principle 11), so the model runs across
//! races rather than over cultures alone.
//!
//! This brick builds the representation and the single-mind appraisal-and-update kernel: an
//! incoming event is appraised through the axiom before it can touch the axiom (evidential
//! pressure is the event's salience times the source weight for its provenance times one
//! minus the dogmatism damping), and an AGM entrenchment gate decides the outcome. Below the
//! gate the pressure is assimilated (the caller routes it to a value or a fast belief facet,
//! Part 9), so belief perseverance and confirmation bias fall out of the gate rather than
//! being coded as exceptions; above it the axiom accommodates, moving toward the evidence by
//! a step that grows with the pressure, so a single high-salience source-weighted event
//! clears even a deeply entrenched axiom in one move, the revelation jump, emergent from the
//! one formula rather than a special branch.
//!
//! Design decisions taken for this build (each non-final, revisitable at stress-test time):
//!
//! - `AxiomDomain` is a data registry ([`DomainRegistry`]) seeded with the five standard
//!   domains, not a closed enum. The kernel never dispatches on the domain (it is a grouping
//!   and lexicon label), so a closed list there would author one level of worldview ontology
//!   on the content path; the registry keeps the five as an overridable default seed and lets
//!   a strange race recarve them (Principle 11; the research paper's own caveat that no
//!   human-derived taxonomy is universal).
//! - The Friedkin-Johnsen stubbornness anchor is a per-axiom base modulated by the epistemic
//!   stance ([`EpistemicStance::effective_stubbornness`]): the positive base keeps the anchor
//!   above zero, which is what keeps a population in lasting disagreement rather than
//!   collapsing to consensus.
//! - The bounded evidence ring evicts FIFO by recency (a ring buffer of recent corroborating
//!   and disconfirming tags, as the paper specifies); a recency-weighted-pressure hybrid is
//!   reserved for the stress-test batch.
//!
//! Deferred to later bricks (their dependencies are not built): enculturation (the
//! Friedkin-Johnsen anchored average over a group), bounded-confidence schism (whose prestige
//! arm waits on a status system), inheritance (the heritable-plus-encultured seed blend, on a
//! new keyed `Phase`), calcification of unchallenged axioms, and the two level-of-detail
//! group regimes. The numeric calibrations are reserved owner values (the `axiom.*` manifest
//! entries), supplied as data, never invented here.

use std::collections::{BTreeMap, VecDeque};

use civsim_core::{EventId, Fixed};

use crate::decision::Curve;
use crate::value::ValueProfile;

/// A data-defined axiom-axis identifier (a bipolar worldview axis a race carries, Part 40).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AxiomAxisId(pub u32);

/// A data-defined source-mode identifier (tradition, evidence, revelation, authority,
/// intuition, or a race's own, Part 40). The kernel never matches on a specific source-mode
/// id; a mode is privileged only by the weight a race's epistemic stance gives it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SourceModeId(pub u32);

/// A data-defined axiom-domain identifier (the grouping a worldview axis belongs to, Part
/// 40). A registry id rather than a closed enum, so the domain set is overridable per race.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AxiomDomainId(pub u32);

/// A domain definition: an id and its name (and, later, a descriptor lexicon for prose). Pure
/// data; the kernel does not read it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DomainDef {
    /// The domain's stable id.
    pub id: AxiomDomainId,
    /// The domain's name.
    pub name: String,
}

/// The axiom-domain registry (design Part 28, the R-AXIOM data-registry decision). The
/// mechanism that reads a domain is fixed; the membership is data and grows with the world.
/// [`DomainRegistry::standard_seed`] is an overridable default fixture of the five domains the
/// human-derived instruments suggest, not a closed list.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DomainRegistry {
    domains: BTreeMap<AxiomDomainId, DomainDef>,
}

impl DomainRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        DomainRegistry {
            domains: BTreeMap::new(),
        }
    }

    /// Register a domain.
    pub fn insert(&mut self, def: DomainDef) {
        self.domains.insert(def.id, def);
    }

    /// A domain by id.
    pub fn get(&self, id: AxiomDomainId) -> Option<&DomainDef> {
        self.domains.get(&id)
    }

    /// How many domains are registered.
    pub fn len(&self) -> usize {
        self.domains.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.domains.is_empty()
    }

    /// The default seed: the five standard domains (World, Selfhood, Others, Sacred,
    /// Epistemic) the Social-Axioms and Primals instruments suggest, as a labelled default
    /// fixture a world may override or extend. Membership is data; this is a convenience seed,
    /// not a closed taxonomy.
    pub fn standard_seed() -> Self {
        let mut r = DomainRegistry::new();
        for (i, name) in ["World", "Selfhood", "Others", "Sacred", "Epistemic"]
            .into_iter()
            .enumerate()
        {
            r.insert(DomainDef {
                id: AxiomDomainId(i as u32),
                name: name.to_string(),
            });
        }
        r
    }
}

/// A source-mode definition: an id and its name. Pure data; source modes are a registry, not
/// a closed enum.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceModeDef {
    /// The source mode's stable id.
    pub id: SourceModeId,
    /// The source mode's name (tradition, evidence, revelation, ...).
    pub name: String,
}

/// One appraised piece of evidence bearing on an axiom: which event it came from, through
/// which source mode, and the signed pressure it exerts (positive toward the axis's positive
/// pole, negative toward the negative pole). Magnitude is the appraised weight; sign is the
/// direction the evidence pulls the stance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EvidenceTag {
    /// The event this evidence came from.
    pub event: EventId,
    /// The source mode it arrived through.
    pub source: SourceModeId,
    /// The signed appraised pressure.
    pub pressure: Fixed,
}

/// The bounded evidence ring of an axiom: a fixed-capacity FIFO buffer of the most recent
/// corroborating and disconfirming tags, the first bounded slice of a justification model
/// (the full provenance graph is a later goal). When full, the oldest tag is evicted
/// (recency eviction, the design's choice; a recency-weighted-pressure hybrid is reserved).
/// Bounded memory is what keeps the per-entity counter-RNG and the state hash bit-identical.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EvidenceRing {
    cap: usize,
    tags: VecDeque<EvidenceTag>,
}

impl EvidenceRing {
    /// A ring of the given capacity (the reserved `axiom.evidence_ring_capacity`).
    pub fn new(cap: usize) -> Self {
        EvidenceRing {
            cap,
            tags: VecDeque::new(),
        }
    }

    /// Push a tag, evicting the oldest if the ring is at capacity (FIFO). A zero-capacity ring
    /// stores nothing, so an axiom configured with no ring never accumulates pressure.
    pub fn push(&mut self, tag: EvidenceTag) {
        if self.cap == 0 {
            return;
        }
        if self.tags.len() == self.cap {
            self.tags.pop_front();
        }
        self.tags.push_back(tag);
    }

    /// The signed accumulated pressure over the ring, in FIFO order. Summed with saturating
    /// adds so a long-running ring never panics; the ring is small and bounded, so the sum is
    /// exact in practice and order is the ring's fixed insertion order.
    pub fn accumulated_pressure(&self) -> Fixed {
        self.tags
            .iter()
            .fold(Fixed::ZERO, |acc, t| acc.saturating_add(t.pressure))
    }

    /// The tags, oldest first.
    pub fn tags(&self) -> impl Iterator<Item = &EvidenceTag> + '_ {
        self.tags.iter()
    }

    /// How many tags the ring holds.
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Whether the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

/// How a mind decides what is true and defends it (design Part 28). The source weights are
/// normalized to a unit sum at construction (the load-time canonicalization), so the kernel
/// reads them without dividing in the hot path. The four scalars parametrize the one update
/// kernel for every belief: dogmatism damps all accommodation, seizing is the urgency to lock
/// onto an early answer, freezing the permanence with which it then defends it, and certainty
/// the default entrenchment a new belief gets.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EpistemicStance {
    source_weights: BTreeMap<SourceModeId, Fixed>,
    /// Global damping on all accommodation (0..1).
    pub dogmatism: Fixed,
    /// Urgency: how fast the mind locks onto an early answer (need for closure).
    pub seizing: Fixed,
    /// Permanence: how strongly it then defends it.
    pub freezing: Fixed,
    /// Knowledge as fixed versus evolving: the default entrenchment a new belief gets.
    pub certainty: Fixed,
}

impl EpistemicStance {
    /// Build a stance, normalizing the raw source weights to a unit sum (Principle 3: the
    /// canonicalization happens once here, never as a division in the per-event path). If the
    /// raw weights sum to zero the weights are left empty and every source weight reads zero.
    pub fn new(
        raw_weights: impl IntoIterator<Item = (SourceModeId, Fixed)>,
        dogmatism: Fixed,
        seizing: Fixed,
        freezing: Fixed,
        certainty: Fixed,
    ) -> Self {
        let raw: BTreeMap<SourceModeId, Fixed> = raw_weights.into_iter().collect();
        let sum = raw
            .values()
            .fold(Fixed::ZERO, |acc, &w| acc.saturating_add(w));
        let source_weights = if sum == Fixed::ZERO {
            BTreeMap::new()
        } else {
            raw.into_iter().map(|(m, w)| (m, w.div(sum))).collect()
        };
        EpistemicStance {
            source_weights,
            dogmatism,
            seizing,
            freezing,
            certainty,
        }
    }

    /// The (normalized) weight this stance gives a source mode, or zero if it weights that
    /// mode not at all. The kernel keys on the weight, never on the mode's id.
    pub fn source_weight(&self, mode: SourceModeId) -> Fixed {
        self.source_weights
            .get(&mode)
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// The sum of the normalized source weights (for verification; one within fixed-point
    /// rounding when any weight was supplied).
    pub fn source_weight_sum(&self) -> Fixed {
        self.source_weights
            .values()
            .fold(Fixed::ZERO, |acc, &w| acc.saturating_add(w))
    }

    /// The effective Friedkin-Johnsen stubbornness anchor for an axiom, the hybrid of a
    /// per-axiom base and the mind's epistemic temperament (the owner's decision): the base is
    /// lifted toward one by the mind's dogmatism and freezing, `theta = base + (1 - base) *
    /// pull` with `pull = (dogmatism + freezing) / 2` clamped to `[0, 1]`. Because the base is
    /// the floor, theta stays at or above it, so a positive base keeps theta above zero, which
    /// is what holds a population in lasting disagreement rather than collapsing to consensus
    /// (the DeGroot degenerate case). The relative weighting of dogmatism versus freezing is a
    /// tunable; the mean is the chosen form. Used by the deferred enculturation brick.
    pub fn effective_stubbornness(&self, base: Fixed) -> Fixed {
        let base = base.clamp(Fixed::ZERO, Fixed::ONE);
        let pull = (self.dogmatism + self.freezing)
            .mul(Fixed::from_ratio(1, 2))
            .clamp(Fixed::ZERO, Fixed::ONE);
        (base + (Fixed::ONE - base).mul(pull)).clamp(Fixed::ZERO, Fixed::ONE)
    }
}

/// A foundational stance: few per race, deep, slow (design Part 28). Position (`stance`) and
/// hold (`strength`, `confidence`, `entrenchment`) are separate fields, after Rokeach's
/// central-peripheral architecture and AGM epistemic entrenchment. Every field is fixed-point
/// or an integer rank, so the kernel is deterministic.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Axiom {
    /// Which per-race axiom axis this is.
    pub axis: AxiomAxisId,
    /// Signed position on the bipolar axis, in `[-1, 1]`.
    pub stance: Fixed,
    /// Conviction magnitude, `0..1`.
    pub strength: Fixed,
    /// Evidential weight behind the stance, `0..1`.
    pub confidence: Fixed,
    /// AGM ordering rank: the gate deciding what yields first. Higher is harder to move.
    pub entrenchment: i32,
    /// How often this axiom is invoked in appraisal.
    pub salience: Fixed,
    /// The per-axiom Friedkin-Johnsen stubbornness base (the hybrid floor; the effective
    /// anchor is [`EpistemicStance::effective_stubbornness`] of this base). `0..1`.
    pub stubbornness: Fixed,
    /// The heritable anchor stance, set at birth, immutable (the FJ prejudice).
    pub innate_seed: Fixed,
    /// The bounded evidence ring feeding the entrenchment gate.
    pub evidence: EvidenceRing,
}

/// The outcome of appraising one event against an axiom.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appraisal {
    /// The pressure was below the entrenchment gate: the axiom did not move, and the caller
    /// routes the signed pressure to a value or a fast belief facet (Part 9). This is where
    /// belief perseverance and confirmation bias come from.
    Assimilated {
        /// The signed pressure the event exerted.
        pressure: Fixed,
    },
    /// The pressure cleared the gate: the axiom moved from `from` to `to`.
    Accommodated {
        /// The stance before the move.
        from: Fixed,
        /// The stance after the move.
        to: Fixed,
        /// The signed pressure the event exerted.
        pressure: Fixed,
    },
}

impl Axiom {
    /// Appraise an incoming event and update (design Part 28). The event supports `toward`, a
    /// position on the axis in `[-1, 1]` (where the evidence points), with the given
    /// `salience` and the `source_weight` the believer's epistemic stance gives the event's
    /// provenance. Evidential pressure magnitude is `salience * source_weight * (1 -
    /// dogmatism)`; its sign is the direction from the current stance toward the evidence. The
    /// signed pressure is pushed into the FIFO ring, and the accumulated ring pressure is the
    /// net signed total. If its magnitude exceeds `threshold` (the entrenchment-gated value for this
    /// axiom's rank, from the reserved curve), the axiom accommodates: it moves toward the
    /// evidence by a step that grows with the accumulated pressure and the per-axis
    /// `plasticity`, capped at a full move, so an extreme single event jumps in one step
    /// (the revelation jump, emergent). Otherwise the event is assimilated and the stance does
    /// not move. A pure deterministic update given its inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn appraise(
        &mut self,
        event: EventId,
        source: SourceModeId,
        toward: Fixed,
        salience: Fixed,
        source_weight: Fixed,
        dogmatism: Fixed,
        threshold: Fixed,
        plasticity: Fixed,
    ) -> Appraisal {
        let magnitude = salience
            .mul(source_weight)
            .mul(Fixed::ONE - dogmatism)
            .clamp(Fixed::ZERO, Fixed::MAX);
        let direction = sign_toward(self.stance, toward);
        let signed = direction.mul(magnitude);
        self.evidence.push(EvidenceTag {
            event,
            source,
            pressure: signed,
        });
        let accumulated = self.evidence.accumulated_pressure();
        let net = accumulated.abs();
        if net > threshold {
            let pole = if accumulated >= Fixed::ZERO {
                Fixed::ONE
            } else {
                Fixed::ZERO - Fixed::ONE
            };
            let step = plasticity.mul(net).clamp(Fixed::ZERO, Fixed::ONE);
            let from = self.stance;
            let to = (self.stance + step.mul(pole - self.stance))
                .clamp(Fixed::ZERO - Fixed::ONE, Fixed::ONE);
            self.stance = to;
            Appraisal::Accommodated {
                from,
                to,
                pressure: signed,
            }
        } else {
            Appraisal::Assimilated { pressure: signed }
        }
    }
}

/// The direction from a current stance toward an evidence position: `+1`, `-1`, or `0` when
/// they coincide. Integer-clean (no division), so the sign is exact.
fn sign_toward(stance: Fixed, toward: Fixed) -> Fixed {
    if toward > stance {
        Fixed::ONE
    } else if toward < stance {
        Fixed::ZERO - Fixed::ONE
    } else {
        Fixed::ZERO
    }
}

/// The per-race registry entry for an axiom axis (design Part 28, Part 40): its poles, the
/// domain it belongs to (by registry id), and its per-axis dynamics defaults. The numbers are
/// reserved for calibration. The full per-axis plasticity curve (by age or context) is
/// deferred; for now `plasticity` is the scalar step scale the kernel reads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AxiomAxisDef {
    /// The axis's stable id.
    pub id: AxiomAxisId,
    /// The negative pole's name (for example "domination", "hostile", "dangerous").
    pub negative_pole: String,
    /// The positive pole's name (for example "reverence", "trusting", "safe").
    pub positive_pole: String,
    /// Which domain this axis belongs to (a registry id, not a closed enum).
    pub domain: AxiomDomainId,
    /// The heritable fraction of the innate seed (reserved).
    pub heritability: Fixed,
    /// Entrenchment gained per quiet phase, to a cap (reserved; the calcification brick reads
    /// it).
    pub calcify: Fixed,
    /// The accommodation step scale (reserved; the full per-axis plasticity curve is
    /// deferred).
    pub plasticity: Fixed,
}

/// The per-race axiom-axis registry (data).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AxiomAxisRegistry {
    axes: BTreeMap<AxiomAxisId, AxiomAxisDef>,
}

impl AxiomAxisRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        AxiomAxisRegistry {
            axes: BTreeMap::new(),
        }
    }

    /// Register an axis.
    pub fn insert(&mut self, def: AxiomAxisDef) {
        self.axes.insert(def.id, def);
    }

    /// An axis by id.
    pub fn get(&self, id: AxiomAxisId) -> Option<&AxiomAxisDef> {
        self.axes.get(&id)
    }

    /// How many axes are registered.
    pub fn len(&self) -> usize {
        self.axes.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }
}

/// A race or being's intrinsic beliefs, seeded at the dawn (design Part 28): an innate value
/// profile over the race's value axes (Part 21), a small set of axioms, and the epistemic
/// stance that parametrizes the kernel.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntrinsicBeliefs {
    /// The innate value profile.
    pub values: ValueProfile,
    /// The innate foundational stances.
    pub axioms: Vec<Axiom>,
    /// How this mind decides what is true.
    pub epistemic: EpistemicStance,
}

/// The entrenchment-gated pressure threshold for an axiom of the given rank, read from the
/// reserved entrenchment-threshold curve (`axiom.entrenchment_curve`). The rank is mapped
/// through the curve as its `x`; a higher rank yields a higher threshold (harder to move).
/// The curve is data; this is the one place rank becomes a threshold.
pub fn entrenchment_threshold(curve: &Curve, rank: i32) -> Fixed {
    curve.eval(Fixed::from_int(rank))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRADITION: SourceModeId = SourceModeId(0);
    const EVIDENCE: SourceModeId = SourceModeId(1);
    const AXIS: AxiomAxisId = AxiomAxisId(0);
    const EV: EventId = EventId(1);

    fn ring(cap: usize) -> EvidenceRing {
        EvidenceRing::new(cap)
    }

    fn axiom(stance: Fixed, entrenchment: i32, cap: usize) -> Axiom {
        Axiom {
            axis: AXIS,
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: stance,
            evidence: ring(cap),
        }
    }

    #[test]
    fn domain_registry_seed_has_the_five_standard_domains_and_is_overridable() {
        let mut r = DomainRegistry::standard_seed();
        assert_eq!(r.len(), 5);
        assert_eq!(r.get(AxiomDomainId(3)).unwrap().name, "Sacred");
        // Overridable: a strange race can add or recarve a domain.
        r.insert(DomainDef {
            id: AxiomDomainId(5),
            name: "Ancestors".to_string(),
        });
        assert_eq!(r.len(), 6);
    }

    #[test]
    fn source_weights_normalize_to_unit_sum_at_construction() {
        // Raw weights 1 and 3 normalize to 0.25 and 0.75.
        let s = EpistemicStance::new(
            [
                (TRADITION, Fixed::from_int(1)),
                (EVIDENCE, Fixed::from_int(3)),
            ],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(s.source_weight(TRADITION), Fixed::from_ratio(1, 4));
        assert_eq!(s.source_weight(EVIDENCE), Fixed::from_ratio(3, 4));
        assert_eq!(s.source_weight_sum(), Fixed::ONE);
        // An unweighted mode reads zero; the kernel never keys on the mode id.
        assert_eq!(s.source_weight(SourceModeId(99)), Fixed::ZERO);
    }

    #[test]
    fn effective_stubbornness_floors_at_the_base_and_rises_with_temperament() {
        let base = Fixed::from_ratio(1, 4);
        // A placid mind: theta equals the base (no epistemic lift).
        let placid = EpistemicStance::new(
            [(EVIDENCE, Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        );
        assert_eq!(placid.effective_stubbornness(base), base);
        // A dogmatic, freezing mind: theta is lifted above the base but stays in range.
        let rigid = EpistemicStance::new(
            [(TRADITION, Fixed::ONE)],
            Fixed::ONE,
            Fixed::ZERO,
            Fixed::ONE,
            Fixed::ZERO,
        );
        let theta = rigid.effective_stubbornness(base);
        assert!(theta > base, "temperament lifts the anchor");
        assert!(theta <= Fixed::ONE);
        // A positive base keeps theta strictly above zero (no DeGroot consensus collapse).
        assert!(placid.effective_stubbornness(Fixed::from_ratio(1, 10)) > Fixed::ZERO);
    }

    #[test]
    fn evidence_ring_is_fifo_and_bounded() {
        let mut r = ring(2);
        for i in 0..3u64 {
            r.push(EvidenceTag {
                event: EventId(i),
                source: EVIDENCE,
                pressure: Fixed::from_int(i as i32 + 1),
            });
        }
        assert_eq!(r.len(), 2, "capacity holds");
        let events: Vec<u64> = r.tags().map(|t| t.event.0).collect();
        assert_eq!(events, vec![1, 2], "the oldest tag was evicted");
        // Accumulated pressure is the sum of the surviving tags (2 + 3).
        assert_eq!(r.accumulated_pressure(), Fixed::from_int(5));
    }

    #[test]
    fn sub_threshold_pressure_is_assimilated_and_the_axiom_does_not_move() {
        // A low-salience event against a high threshold: assimilated, stance unchanged. This
        // is belief perseverance, emergent from the gate.
        let mut a = axiom(Fixed::ZERO, 10, 4);
        let high_threshold = Fixed::from_int(100);
        let outcome = a.appraise(
            EV,
            EVIDENCE,
            Fixed::ONE,               // evidence points to the positive pole
            Fixed::from_ratio(1, 10), // low salience
            Fixed::ONE,
            Fixed::ZERO,
            high_threshold,
            Fixed::ONE,
        );
        assert!(matches!(outcome, Appraisal::Assimilated { .. }));
        assert_eq!(a.stance, Fixed::ZERO, "the axiom did not move");
    }

    #[test]
    fn repeated_sub_threshold_evidence_never_moves_the_axiom() {
        // Belief perseverance under a drip of weak disconfirmation: still assimilated each
        // time, because each event alone is below the gate and the ring is capacity 1 so
        // pressure does not accumulate past one tag.
        let mut a = axiom(Fixed::ZERO, 10, 1);
        let threshold = Fixed::from_int(10);
        for i in 0..5u64 {
            let outcome = a.appraise(
                EventId(i),
                EVIDENCE,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ONE,
                Fixed::ZERO,
                threshold,
                Fixed::ONE,
            );
            assert!(matches!(outcome, Appraisal::Assimilated { .. }));
        }
        assert_eq!(a.stance, Fixed::ZERO);
    }

    #[test]
    fn supra_threshold_pressure_accommodates_toward_the_evidence() {
        let mut a = axiom(Fixed::ZERO, 1, 4);
        let low_threshold = Fixed::from_ratio(1, 100);
        let outcome = a.appraise(
            EV,
            EVIDENCE,
            Fixed::ONE, // toward the positive pole
            Fixed::ONE,
            Fixed::ONE,
            Fixed::ZERO,
            low_threshold,
            Fixed::from_ratio(1, 4), // a modest step
        );
        match outcome {
            Appraisal::Accommodated { from, to, .. } => {
                assert_eq!(from, Fixed::ZERO);
                assert!(to > Fixed::ZERO, "moved toward the positive pole");
                assert!(to <= Fixed::ONE);
            }
            _ => panic!("expected accommodation"),
        }
    }

    #[test]
    fn a_revelation_jump_clears_a_high_entrenchment_in_one_step() {
        // The emergent revelation jump: a single overwhelming event (high salience, full
        // source weight) on a deeply entrenched axiom moves it most of the way to the pole in
        // one step, with no special branch, because the step saturates with the pressure.
        let mut a = axiom(Fixed::ZERO - Fixed::ONE, 100, 4); // stance at the negative pole, deeply held
        let high_threshold = Fixed::from_int(5);
        let outcome = a.appraise(
            EV,
            EVIDENCE,
            Fixed::ONE,          // revelation points to the opposite pole
            Fixed::from_int(50), // overwhelming salience
            Fixed::ONE,
            Fixed::ZERO,
            high_threshold,
            Fixed::ONE, // full plasticity
        );
        match outcome {
            Appraisal::Accommodated { from, to, .. } => {
                assert_eq!(from, Fixed::ZERO - Fixed::ONE);
                assert_eq!(
                    to,
                    Fixed::ONE,
                    "the jump cleared the gate to the far pole in one step"
                );
            }
            _ => panic!("expected a revelation jump (accommodation)"),
        }
    }

    #[test]
    fn appraisal_is_deterministic_across_identical_runs() {
        let run = || {
            let mut a = axiom(Fixed::ZERO, 2, 3);
            let t = Fixed::from_ratio(1, 100);
            for i in 0..4u64 {
                a.appraise(
                    EventId(i),
                    EVIDENCE,
                    Fixed::ONE,
                    Fixed::from_ratio(1, 2),
                    Fixed::ONE,
                    Fixed::ZERO,
                    t,
                    Fixed::from_ratio(1, 8),
                );
            }
            a.stance
        };
        assert_eq!(
            run(),
            run(),
            "the same event sequence yields the same stance"
        );
    }

    #[test]
    fn entrenchment_threshold_rises_with_rank() {
        // A monotone reserved curve: rank 0 -> low threshold, rank 10 -> high threshold.
        let curve = Curve::new([
            (Fixed::from_int(0), Fixed::from_int(1)),
            (Fixed::from_int(10), Fixed::from_int(100)),
        ]);
        let lo = entrenchment_threshold(&curve, 0);
        let hi = entrenchment_threshold(&curve, 10);
        assert!(hi > lo, "a higher rank is harder to move");
        assert_eq!(lo, Fixed::from_int(1));
        assert_eq!(hi, Fixed::from_int(100));
    }
}
