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

//! The transmission beat inside `World::tick` (design Parts 20, 23, 25, 41, R-DEEPTECH, world-wiring
//! increment 7): a co-located band spreads the techniques it holds, a copy drifts a fidelity-scaled
//! amount, a band too small to sustain a technique loses it (a dark age) while a larger band keeps it
//! and a later re-contact rediscovers it, and a high-fidelity culture deepens where a low-fidelity one
//! stays shallow, all from per-being cognition with no race branch. Every number here is a labelled
//! fixture, never an owner value.

use civsim_core::{Fixed, StableId};
use civsim_sim::{AccessWeights, DesignId, InferenceParams, TransmissionParams, World};

const A: DesignId = 0xA;
const B: DesignId = 0xB;
const C: DesignId = 0xC;

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// Transmission calibrations as a labelled fixture: a small base drift, a practitioner floor of three
/// (a design a band holds with fewer than three erodes there), and a brisk loss rate so a dark age
/// resolves within a test's tick budget. Never owner values.
fn transmission(floor: u32) -> TransmissionParams {
    TransmissionParams {
        drift_rate: Fixed::from_ratio(3, 100),
        loss_practitioner_floor: floor,
        loss_rate: Fixed::from_ratio(1, 4),
    }
}

fn world(seed: u64) -> World {
    World::new(params(), params(), AccessWeights::default()).with_seed(seed)
}

/// Spawn a band of `n` beings at `place`, each with the given reasoning acuity (which, with a neutral
/// memory, is the copy fidelity the transmission kernel reads). Returns their ids in spawn order.
fn band(w: &mut World, place: u32, n: usize, acuity: Fixed) -> Vec<StableId> {
    (0..n)
        .map(|_| {
            let id = w.spawn(acuity);
            w.set_place(id, place);
            id
        })
        .collect()
}

/// The count of held copies at proficiency at least `depth` across a set of beings: the culture's
/// knowledge mass at that depth, the read the fidelity dual turns on.
fn deep_copies(w: &World, ids: &[StableId], depth: Fixed) -> usize {
    ids.iter()
        .map(|&id| {
            w.knowledge_of(id)
                .map(|k| {
                    k.known
                        .iter()
                        .filter(|&&d| k.proficiency_of(d) >= depth)
                        .count()
                })
                .unwrap_or(0)
        })
        .sum()
}

#[test]
fn a_design_spreads_through_a_band_and_the_run_replays() {
    // One high-fidelity band: a single being originates three designs, and the transmission beat
    // spreads them to every co-located member over ticks. The whole trajectory replays bit for bit,
    // and a different seed drives a different (still deterministic) drift.
    let build = |seed: u64| {
        let mut w = world(seed);
        let members = band(&mut w, 1, 6, Fixed::from_ratio(98, 100));
        w.set_transmission(transmission(3));
        for &d in &[A, B, C] {
            w.originate_design(members[0], d, Fixed::ONE);
        }
        (w, members)
    };

    let (mut w, members) = build(0x7A1);
    assert_eq!(
        w.holders_of(A),
        1,
        "only the originator holds A at the dawn"
    );
    for _ in 0..12 {
        w.tick(&[]);
    }
    assert_eq!(
        w.holders_of(A),
        members.len() as u32,
        "the design spread to the whole co-located band"
    );
    assert!(
        w.holders_of(B) > 1 && w.holders_of(C) > 1,
        "the band's other originated designs spread too"
    );
    // High fidelity ratchets the copies in at depth (a copy lands near the holder's proficiency).
    assert!(
        deep_copies(&w, &members, Fixed::from_ratio(1, 2)) >= members.len(),
        "the high-fidelity copies implanted at depth"
    );

    let (mut w2, _) = build(0x7A1);
    for _ in 0..12 {
        w2.tick(&[]);
    }
    assert_eq!(
        w.state_hash(),
        w2.state_hash(),
        "the tick-driven transmission beat replays bit for bit"
    );
    let (mut w3, _) = build(0x7A2);
    for _ in 0..12 {
        w3.tick(&[]);
    }
    assert_ne!(
        w.state_hash(),
        w3.state_hash(),
        "a different seed drifts the copies differently"
    );
}

#[test]
fn high_and_low_fidelity_bands_diverge_in_depth_from_cognition_alone() {
    // The non-steering fidelity dual through the tick beat. Two bands identical in every way but their
    // per-being reasoning acuity spread the same three designs. The kernel never sees a race or a band;
    // the high-acuity band deepens its copies while the low-acuity band, copying at a fraction of the
    // holder's proficiency, stays shallow. The divergence falls out of per-being cognition (Principle
    // 9), not an authored per-race fidelity table.
    let mut w = world(0xF1DE);
    let hi = band(&mut w, 1, 6, Fixed::from_ratio(98, 100));
    let lo = band(&mut w, 2, 6, Fixed::from_ratio(30, 100));
    w.set_transmission(transmission(3));
    for &d in &[A, B, C] {
        w.originate_design(hi[0], d, Fixed::ONE);
        w.originate_design(lo[0], d, Fixed::ONE);
    }
    for _ in 0..20 {
        w.tick(&[]);
    }
    // Measure the learners' transmitted copies (past the originator, which holds its originals at
    // full proficiency in either band): the depth of what was copied is the fidelity signal.
    let half = Fixed::from_ratio(1, 2);
    let hi_deep = deep_copies(&w, &hi[1..], half);
    let lo_deep = deep_copies(&w, &lo[1..], half);
    assert!(
        hi_deep > lo_deep,
        "the high-fidelity band's copies land deeper: hi={hi_deep} lo={lo_deep}"
    );
    assert_eq!(
        lo_deep, 0,
        "the low-fidelity band copied nothing deep: it took a shallow fraction of the holder"
    );
}

#[test]
fn a_below_floor_band_culls_then_a_holding_band_rediscovers_it() {
    // A dark age and a rediscovery through the tick beat. A small band (two holders, below the floor of
    // three) loses a design over ticks, while a larger band (four holders, at the floor) keeps it; then
    // a holder from the keeping band moves into the small band and the beat re-transmits the design for
    // free, because its content address is stable.
    let mut w = world(0xDA27);
    let small = band(&mut w, 1, 2, Fixed::from_ratio(98, 100));
    let large = band(&mut w, 2, 4, Fixed::from_ratio(98, 100));
    w.set_transmission(transmission(3));
    for &id in small.iter().chain(large.iter()) {
        w.originate_design(id, A, Fixed::ONE);
    }
    // Run until the small band, held below its local floor, erodes and culls the design.
    for _ in 0..40 {
        w.tick(&[]);
    }
    assert!(
        small
            .iter()
            .all(|&id| !w.knowledge_of(id).unwrap().holds(A)),
        "the below-floor band lost the design (a dark age)"
    );
    assert!(
        large.iter().all(|&id| w.knowledge_of(id).unwrap().holds(A)),
        "the at-floor band kept the design (it never eroded there)"
    );

    // Rediscovery: a holder from the keeping band re-contacts the small band (co-locates) and the beat
    // re-transmits the design into it for free.
    w.set_place(large[0], 1);
    for _ in 0..3 {
        w.tick(&[]);
    }
    assert!(
        small.iter().any(|&id| w.knowledge_of(id).unwrap().holds(A)),
        "the design re-diffused into the band from a culture that still held it (rediscovery)"
    );
}

#[test]
fn an_unarmed_world_and_an_isolated_holder_do_not_transmit() {
    // Two guards. Without the transmission calibration installed the beat is a no-op, so an originated
    // design never spreads; and a holder alone in its place has no one to teach, so its design stays put
    // even with the calibration armed.
    let mut unarmed = world(0x0FF);
    let m = band(&mut unarmed, 1, 4, Fixed::ONE);
    for &d in &[A, B] {
        unarmed.originate_design(m[0], d, Fixed::ONE);
    }
    for _ in 0..10 {
        unarmed.tick(&[]);
    }
    assert_eq!(
        unarmed.holders_of(A),
        1,
        "with no transmission calibration the beat does not spread a design"
    );

    let mut armed = world(0x0FF);
    let lone = armed.spawn(Fixed::ONE);
    armed.set_place(lone, 1);
    let elsewhere = band(&mut armed, 2, 3, Fixed::ONE);
    let _ = elsewhere;
    // Floor of one so the lone holder is at its own floor and does not erode: this isolates the
    // no-co-located-learner property (no spread) from the below-floor loss the other tests exercise.
    armed.set_transmission(transmission(1));
    armed.originate_design(lone, A, Fixed::ONE);
    for _ in 0..10 {
        armed.tick(&[]);
    }
    assert_eq!(
        armed.holders_of(A),
        1,
        "a holder with no co-located learner keeps its design to itself"
    );
}
