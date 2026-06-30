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

//! Bounded-confidence schism (design Part 28): members are influenced only by others within a
//! confidence band, so a spread-out band fractures into sects rather than converging.

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

/// Intrinsic beliefs with one axiom on `AXIS`, the stance also its innate seed, a low
/// stubbornness so movement is visible, and a placid epistemic stance. Confidence is 0.5.
fn beliefs(stance: Fixed) -> IntrinsicBeliefs {
    IntrinsicBeliefs {
        values: ValueProfile::new(),
        axioms: vec![Axiom {
            axis: AXIS,
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 1,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 8),
            innate_seed: stance,
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

/// Seed a band of members at the given stances, returning their ids in order.
fn seed_band(w: &mut World, stances: &[Fixed]) -> Vec<StableId> {
    stances
        .iter()
        .map(|&s| {
            let id = w.spawn(Fixed::ONE);
            w.set_intrinsic(id, beliefs(s));
            id
        })
        .collect()
}

#[test]
fn two_far_clusters_do_not_influence_each_other() {
    // Two tight pairs far apart: at band 0.1 the low pair and the high pair never mix.
    let mut w = World::new(params(), params(), AccessWeights::default());
    let ids = seed_band(
        &mut w,
        &[
            Fixed::ZERO,
            Fixed::from_ratio(1, 20),  // 0.05, within 0.1 of 0.0
            Fixed::from_ratio(9, 10),  // 0.9
            Fixed::from_ratio(19, 20), // 0.95, within 0.1 of 0.9
        ],
    );
    let epsilon = Fixed::from_ratio(1, 10);
    for _ in 0..10 {
        w.enculturate_band_bounded(&ids, AXIS, epsilon);
    }
    // The low pair stays low, the high pair stays high: the band split into two sects.
    assert!(stance_of(&w, ids[0]) < Fixed::from_ratio(1, 5));
    assert!(stance_of(&w, ids[1]) < Fixed::from_ratio(1, 5));
    assert!(stance_of(&w, ids[2]) > Fixed::from_ratio(4, 5));
    assert!(stance_of(&w, ids[3]) > Fixed::from_ratio(4, 5));
    let clusters = w.stance_clusters(&ids, AXIS, epsilon);
    assert_eq!(clusters.len(), 2, "two sects");
}

#[test]
fn a_band_within_one_confidence_band_stays_one_cluster() {
    let mut w = World::new(params(), params(), AccessWeights::default());
    let ids = seed_band(
        &mut w,
        &[
            Fixed::from_ratio(40, 100),
            Fixed::from_ratio(45, 100),
            Fixed::from_ratio(50, 100),
        ],
    );
    let epsilon = Fixed::from_ratio(1, 10);
    let clusters = w.stance_clusters(&ids, AXIS, epsilon);
    assert_eq!(
        clusters.len(),
        1,
        "consecutive gaps within the band: one sect"
    );
}

#[test]
fn variance_signals_fission_for_a_split_band_and_not_a_tight_one() {
    let mut w = World::new(params(), params(), AccessWeights::default());
    let split = seed_band(&mut w, &[Fixed::ZERO, Fixed::ONE]);
    let tight = seed_band(
        &mut w,
        &[Fixed::from_ratio(50, 100), Fixed::from_ratio(52, 100)],
    );
    let threshold = Fixed::from_ratio(1, 10);
    assert!(
        w.is_fissioning(&split, AXIS, threshold),
        "a band spread across the axis is fissioning"
    );
    assert!(
        !w.is_fissioning(&tight, AXIS, threshold),
        "a tight band is not fissioning"
    );
    // A uniform band has zero variance.
    let uniform = seed_band(
        &mut w,
        &[Fixed::from_ratio(3, 10), Fixed::from_ratio(3, 10)],
    );
    assert_eq!(w.axiom_variance(&uniform, AXIS), Some(Fixed::ZERO));
}

#[test]
fn bounded_enculturation_replays_deterministically() {
    let round = || {
        let mut w = World::new(params(), params(), AccessWeights::default());
        let ids = seed_band(
            &mut w,
            &[
                Fixed::ZERO,
                Fixed::from_ratio(1, 10),
                Fixed::from_ratio(1, 2),
                Fixed::ONE,
            ],
        );
        for _ in 0..6 {
            w.enculturate_band_bounded(&ids, AXIS, Fixed::from_ratio(1, 5));
        }
        ids.iter().map(|&id| stance_of(&w, id)).collect::<Vec<_>>()
    };
    assert_eq!(
        round(),
        round(),
        "bounded-confidence rounds replay bit for bit"
    );
}
