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

//! The two Steering Audit invariants the R-CONVERSE resolution adds for the dialogue
//! layer (design Part 41, the analogues of the basis-independence and modality-swap
//! tests):
//!
//! - Content-blindness of force: paired worlds differing only by a permutation of the
//!   force decomposition, the move-kind labels, and the move and effect ids must show
//!   invariant outcomes, so a move's effect is a pure function of the affordance it
//!   realises and never of its label. Here the two worlds run the same scene over
//!   substrates that label and number everything differently while realising the same
//!   affordances, and their canonical state hashes must match.
//! - Channel swap at equal capacity: an equal-capacity change of the channel a
//!   conversation runs over leaves outcomes invariant. The said channel's weight is its
//!   capacity; swapping the channel id while keeping the weight must leave the state hash
//!   unchanged, and a genuine change of capacity must change it (so the test is not
//!   vacuous).

use civsim_bio::evidence::InferenceParams;
use civsim_bio::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_core::{Fixed, StableId};
use civsim_sim::dialogue::{
    EffectSign, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind, MoveKindDef, MoveKindId,
    MoveRegistry,
};
use civsim_sim::world::{GossipParams, Stimulus, TickInput, World};
use civsim_sim::AttrKindId;

const WITNESSED: AccessChannelId = AccessChannelId(1);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

/// Run the same two-promoted-minds scene over a given dialogue substrate and said-channel
/// configuration, returning the canonical state hash. The scene: a speaker observes a
/// value, then four ticks of dialogue spread and ground it. Everything but the substrate
/// and channel under test is held fixed.
fn run(
    floor: ForceFloor,
    registry: MoveRegistry,
    said_id: AccessChannelId,
    said_weight: Fixed,
) -> u128 {
    let mut w = World::new(
        params(),
        params(),
        AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (said_id, said_weight)]),
    )
    .with_seed(0x0005_7EE2);
    w.set_channels(AccessChannelRegistry {
        channels: vec![
            AccessChannelDef {
                id: WITNESSED,
                name: "witnessed".to_string(),
                margin_steps: Some(1),
            },
            AccessChannelDef {
                id: said_id,
                name: "said".to_string(),
                margin_steps: Some(-1),
            },
        ],
    });
    w.set_gossip(GossipParams {
        told_weight: Fixed::from_int(3),
        trust_baseline: Fixed::ONE,
        trust_penalty: Fixed::from_ratio(1, 2),
    });
    w.set_dialogue(registry, floor).unwrap();
    let s = w.spawn(Fixed::ONE);
    let l = w.spawn(Fixed::ONE);
    w.set_place(s, 1);
    w.set_place(l, 1);
    w.promote(s);
    w.promote(l);
    w.tick(&[TickInput {
        mind: s,
        ordinal: 0,
        stim: Stimulus::Observe {
            subject: StableId(99),
            attr: AttrKindId(0),
            hyps: vec![10, 20],
            toward: 10,
            weight: Fixed::from_int(5),
            from: s,
        },
    }]);
    for _ in 0..4 {
        w.tick(&[]);
    }
    w.state_hash()
}

/// The canonical labelling: effect ids 1..3, move ids 1..3, plain names.
fn canonical() -> (ForceFloor, MoveRegistry) {
    let floor = ForceFloor {
        effects: vec![
            ForceEffectDef {
                id: ForceEffectId(1),
                kind: ForceKind::TellEvidence,
                sign: EffectSign::Neutral,
                name: "assert".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(2),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Positive,
                name: "accept".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(3),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Negative,
                name: "refuse".to_string(),
            },
        ],
    };
    let registry = MoveRegistry {
        moves: vec![
            MoveKindDef {
                id: MoveKindId(1),
                name: "assertion".to_string(),
                force: vec![ForceEffectId(1)],
                expects: vec![MoveKindId(2), MoveKindId(3)],
                sincerity_judged: true,
                felicity: vec![],
                gloss: "tells that".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(2),
                name: "acceptance".to_string(),
                force: vec![ForceEffectId(2)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "agrees".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(3),
                name: "refusal".to_string(),
                force: vec![ForceEffectId(3)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "declines".to_string(),
            },
        ],
    };
    (floor, registry)
}

/// The same affordances under a deliberately different labelling: effect ids 7..9 in a
/// different order, move ids 5..7 in a different order, different names and glosses. Only
/// the realised affordances (told-evidence, positive uptake, negative uptake) are shared.
fn permuted() -> (ForceFloor, MoveRegistry) {
    let floor = ForceFloor {
        effects: vec![
            ForceEffectDef {
                id: ForceEffectId(7),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Negative,
                name: "nope".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(8),
                kind: ForceKind::TellEvidence,
                sign: EffectSign::Neutral,
                name: "claim".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(9),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Positive,
                name: "yes".to_string(),
            },
        ],
    };
    let registry = MoveRegistry {
        moves: vec![
            MoveKindDef {
                id: MoveKindId(5),
                name: "rebuff".to_string(),
                force: vec![ForceEffectId(7)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "spurns".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(6),
                name: "avowal".to_string(),
                force: vec![ForceEffectId(8)],
                expects: vec![MoveKindId(7), MoveKindId(5)],
                sincerity_judged: true,
                felicity: vec![],
                gloss: "avers that".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(7),
                name: "assent".to_string(),
                force: vec![ForceEffectId(9)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "assents".to_string(),
            },
        ],
    };
    (floor, registry)
}

#[test]
fn content_blindness_of_force() {
    let (af, ar) = canonical();
    let (bf, br) = permuted();
    let a = run(af, ar, AccessChannelId(3), Fixed::from_int(2));
    let b = run(bf, br, AccessChannelId(3), Fixed::from_int(2));
    assert_eq!(
        a, b,
        "outcomes are invariant under a permutation of the force decomposition, ids, and labels"
    );
}

#[test]
fn channel_swap_at_equal_capacity_is_invariant() {
    let (f1, r1) = canonical();
    let (f2, r2) = canonical();
    let (f3, r3) = canonical();
    // Same capacity (weight 2), different channel id: outcomes must not move.
    let a = run(f1, r1, AccessChannelId(3), Fixed::from_int(2));
    let b = run(f2, r2, AccessChannelId(5), Fixed::from_int(2));
    assert_eq!(
        a, b,
        "an equal-capacity channel swap leaves outcomes invariant"
    );
    // A genuine change of capacity must move the second-order outcome, so the test above
    // is not vacuously passing on an outcome that ignores the channel entirely.
    let c = run(f3, r3, AccessChannelId(5), Fixed::from_int(4));
    assert_ne!(
        a, c,
        "a different-capacity channel changes the grounding outcome"
    );
}
