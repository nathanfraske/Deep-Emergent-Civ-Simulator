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

//! Axiom inheritance (design Part 28): a child's innate seed is the heritable-plus-encultured
//! blend of its parent's seed and the local cultural mean, plus a bounded mutation, so a child
//! resembles both its parent and its culture and varies.

use civsim_core::{Fixed, StableId};
use civsim_sim::{
    AccessWeights, Axiom, AxiomAxisId, EpistemicStance, EvidenceRing, InferenceParams,
    IntrinsicBeliefs, SourceModeId, ValueProfile, World,
};

const AXIS: AxiomAxisId = AxiomAxisId(0);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// Intrinsic beliefs with one axiom on `AXIS`: the given stance and innate seed. Confidence
/// 0.5; a placid epistemic stance.
fn beliefs(stance: Fixed, seed: Fixed) -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::new(),
        axioms: vec![Axiom {
            axis: AXIS,
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 3,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: seed,
            evidence: EvidenceRing::new(3),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    }
}

fn child_seed(w: &World, id: StableId) -> Fixed {
    let ax = &w.intrinsic_of(id).unwrap().axioms[0];
    assert_eq!(
        ax.stance, ax.innate_seed,
        "a child starts at its inherited seed"
    );
    ax.innate_seed
}

/// A parent at seed 0 and a band whose local mean stance is 1.0.
fn parent_and_band(w: &mut World) -> (StableId, Vec<StableId>) {
    let parent = w.spawn(Fixed::ONE);
    w.set_intrinsic(parent, beliefs(Fixed::ZERO, Fixed::ZERO));
    let band: Vec<StableId> = (0..3)
        .map(|_| {
            let id = w.spawn(Fixed::ONE);
            w.set_intrinsic(id, beliefs(Fixed::ONE, Fixed::ONE));
            id
        })
        .collect();
    (parent, band)
}

#[test]
fn a_child_blends_parent_and_local_culture() {
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(7);
    let (parent, band) = parent_and_band(&mut w);
    // Heritability 0.5, no mutation: child seed = 0.5*0 (parent) + 0.5*1 (band mean) = 0.5.
    let child = w
        .inherit_child(parent, &band, Fixed::from_ratio(1, 2), Fixed::ZERO, 0)
        .unwrap();
    assert_eq!(child_seed(&w, child), Fixed::from_ratio(1, 2));
}

#[test]
fn heritability_extremes_pick_parent_or_culture() {
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(7);
    let (parent, band) = parent_and_band(&mut w);
    // h = 1, no mutation: pure parent seed (0).
    let all_parent = w
        .inherit_child(parent, &band, Fixed::ONE, Fixed::ZERO, 0)
        .unwrap();
    assert_eq!(child_seed(&w, all_parent), Fixed::ZERO);
    // h = 0, no mutation: pure local culture (1.0).
    let all_culture = w
        .inherit_child(parent, &band, Fixed::ZERO, Fixed::ZERO, 0)
        .unwrap();
    assert_eq!(child_seed(&w, all_culture), Fixed::ONE);
}

#[test]
fn mutation_stays_within_its_bound() {
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(99);
    let (parent, band) = parent_and_band(&mut w);
    let spread = Fixed::from_ratio(1, 10);
    // Blend at h=0.5 is 0.5; the mutated seed stays within [0.4, 0.6].
    for gen in 0..8u64 {
        let child = w
            .inherit_child(parent, &band, Fixed::from_ratio(1, 2), spread, gen)
            .unwrap();
        let s = child_seed(&w, child);
        assert!(
            s >= Fixed::from_ratio(4, 10) && s <= Fixed::from_ratio(6, 10),
            "the mutation is bounded: {s:?}"
        );
    }
}

#[test]
fn inheritance_replays_deterministically() {
    let draw = || {
        let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(0xC0DE);
        let (parent, band) = parent_and_band(&mut w);
        let child = w
            .inherit_child(
                parent,
                &band,
                Fixed::from_ratio(1, 2),
                Fixed::from_ratio(1, 10),
                0,
            )
            .unwrap();
        child_seed(&w, child)
    };
    assert_eq!(
        draw(),
        draw(),
        "the same world and inputs inherit the same seed"
    );
}

#[test]
fn a_childless_parent_with_no_beliefs_yields_none() {
    let mut w = World::new(params(), params(), AccessWeights::default()).with_seed(1);
    let ghost = w.spawn(Fixed::ONE); // no intrinsic beliefs set
    assert!(w
        .inherit_child(ghost, &[], Fixed::from_ratio(1, 2), Fixed::ZERO, 0)
        .is_none());
}
