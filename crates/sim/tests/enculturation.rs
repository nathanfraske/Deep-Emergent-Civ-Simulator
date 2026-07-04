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

//! Enculturation over a band: the Friedkin-Johnsen anchored average (design Part 28). Members
//! move toward the band's confidence-weighted mean stance but stay anchored to their own
//! innate seeds, so a population reaches persistent disagreement rather than consensus.

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

/// Intrinsic beliefs with one axiom on `AXIS`: the given stance and innate seed, a stubbornness
/// base of 0.25, and a placid epistemic stance (zero dogmatism and freezing, so the effective
/// stubbornness equals the base). Confidence is 0.5 so members weigh equally.
fn beliefs(stance: Fixed, seed: Fixed) -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::new(),
        axioms: vec![Axiom {
            axis: AXIS,
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 1,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: seed,
            evidence: EvidenceRing::new(2),
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

fn stance_of(w: &World, id: StableId) -> Fixed {
    w.intrinsic_of(id).unwrap().axioms[0].stance
}

#[test]
fn members_move_toward_the_mean_but_stay_anchored() {
    let mut w = World::new(params(), params(), AccessWeights::default());
    w.set_stubbornness_split(Fixed::from_ratio(1, 2)); // labelled fixture split weight
    let a = w.spawn(Fixed::ONE);
    let b = w.spawn(Fixed::ONE);
    // A at the negative pole, B at the positive, each anchored to its own seed there.
    w.set_intrinsic(a, beliefs(Fixed::ZERO, Fixed::ZERO));
    w.set_intrinsic(b, beliefs(Fixed::ONE, Fixed::ONE));

    w.enculturate_band(&[a, b], AXIS);

    // The mean is 0.5; with theta = 0.25, A rises to 0.375 and B falls to 0.625.
    let sa = stance_of(&w, a);
    let sb = stance_of(&w, b);
    assert_eq!(sa, Fixed::from_ratio(3, 8), "A moved up toward the mean");
    assert_eq!(sb, Fixed::from_ratio(5, 8), "B moved down toward the mean");
    assert!(
        sa < sb,
        "the two stay distinct: lasting disagreement, not consensus"
    );
}

#[test]
fn a_population_with_distinct_seeds_never_collapses_to_one_point() {
    let mut w = World::new(params(), params(), AccessWeights::default());
    w.set_stubbornness_split(Fixed::from_ratio(1, 2)); // labelled fixture split weight
    let a = w.spawn(Fixed::ONE);
    let b = w.spawn(Fixed::ONE);
    w.set_intrinsic(a, beliefs(Fixed::ZERO, Fixed::ZERO));
    w.set_intrinsic(b, beliefs(Fixed::ONE, Fixed::ONE));
    // Many rounds: the anchored average reaches a persistent spread, not a single stance.
    for _ in 0..20 {
        w.enculturate_band(&[a, b], AXIS);
    }
    let sa = stance_of(&w, a);
    let sb = stance_of(&w, b);
    assert!(
        sb - sa >= Fixed::from_ratio(1, 8),
        "a stable gap remains between the two"
    );
}

#[test]
fn an_aligned_band_does_not_drift() {
    // All members already share the stance and seed: the mean equals their stance, so the
    // anchored average leaves them where they are.
    let mut w = World::new(params(), params(), AccessWeights::default());
    w.set_stubbornness_split(Fixed::from_ratio(1, 2)); // labelled fixture split weight
    let a = w.spawn(Fixed::ONE);
    let b = w.spawn(Fixed::ONE);
    let s = Fixed::from_ratio(1, 2);
    w.set_intrinsic(a, beliefs(s, s));
    w.set_intrinsic(b, beliefs(s, s));
    w.enculturate_band(&[a, b], AXIS);
    assert_eq!(stance_of(&w, a), s);
    assert_eq!(stance_of(&w, b), s);
}

#[test]
fn enculturation_replays_deterministically() {
    let round = || {
        let mut w = World::new(params(), params(), AccessWeights::default());
        w.set_stubbornness_split(Fixed::from_ratio(1, 2)); // labelled fixture split weight
        let a = w.spawn(Fixed::ONE);
        let b = w.spawn(Fixed::ONE);
        let c = w.spawn(Fixed::ONE);
        w.set_intrinsic(a, beliefs(Fixed::ZERO, Fixed::ZERO));
        w.set_intrinsic(b, beliefs(Fixed::ONE, Fixed::ONE));
        w.set_intrinsic(c, beliefs(Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)));
        for _ in 0..5 {
            w.enculturate_band(&[a, b, c], AXIS);
        }
        (stance_of(&w, a), stance_of(&w, b), stance_of(&w, c))
    };
    assert_eq!(round(), round(), "the same band enculturates bit for bit");
}
