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

//! The standing false-belief and deception battery for recursive theory of mind
//! (design Part 37, record 62.11), run through the crate's public API as a CI gate.
//!
//! Each case is a scripted deterministic micro-world replayed through the nested
//! inference engine, asserting committed integer argmaxes (no floating point, so the
//! result is bit-reproducible). The pass criterion the resolution sets is that the
//! nested frame commits the literature-correct value and that value differs from
//! projection, so a tight threshold cannot turn the battery into vacuous unknown
//! passes. The weights and thresholds here are clearly-labelled fixtures, not the
//! owner's reserved manifest values; this gate proves the mechanism, while the tuned
//! numbers stay reserved and fail loud in production.

use civsim_core::{Fixed, StableId};
use civsim_sim::evidence::{AttrKindId, InferenceFrame, InferenceParams};
use civsim_sim::tom::{
    detects_deception, AccessChannelId, AccessChannelRegistry, AccessWeights, EvidenceOrder,
    NestedFrame, ProjectionRejected,
};

const WITNESSED: AccessChannelId = AccessChannelId(1);
const TOLD: AccessChannelId = AccessChannelId(2);
const SAID: AccessChannelId = AccessChannelId(3);
const ONE: Fixed = Fixed::ONE;

// One attribute (where a thing is) and its candidate values for the transfer tasks.
const LOCATION: AttrKindId = AttrKindId(0);
const BASKET: u32 = 10;
const BOX: u32 = 20;
// A second attribute (what a container holds) for the unexpected-contents task.
const CONTENTS: AttrKindId = AttrKindId(1);
const PENCILS: u32 = 30;
const CANDY: u32 = 40;

const MODELLER: StableId = StableId(1);

fn registry() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        r#"
[[channels]]
id = 1
name = "witnessed"
margin_steps = 1
[[channels]]
id = 2
name = "told"
margin_steps = 0
[[channels]]
id = 3
name = "said"
margin_steps = -1
"#,
    )
    .expect("registry parses")
}

fn weights() -> AccessWeights {
    // Fixture weights. The hard constraint holds: witnessed (4) strictly exceeds told
    // (3) and said (2), so a witnessed access out-ranks a contrary assertion.
    AccessWeights::from_pairs([
        (WITNESSED, Fixed::from_int(4)),
        (TOLD, Fixed::from_int(3)),
        (SAID, Fixed::from_int(2)),
    ])
}

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(10),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

#[test]
fn battery_classic_transfer_diverges_from_projection() {
    // Sally-Anne: Sally saw the marble in the basket and left; it moved to the box.
    let r = registry();
    let w = weights();
    let p = params();
    let sally = StableId(2);

    let mut sally_model = NestedFrame::new(sally, 1, LOCATION, [BASKET, BOX]);
    sally_model
        .observe_access(&w, WITNESSED, BASKET, ONE, MODELLER)
        .unwrap();
    // The modeller saw the move; that is world evidence and is refused by the model.
    assert_eq!(
        sally_model.admit(EvidenceOrder::World, BOX, Fixed::from_int(5), ONE, MODELLER),
        Err(ProjectionRejected)
    );

    let mut own = InferenceFrame::new(StableId(99), LOCATION, [BASKET, BOX]);
    own.add_evidence(BASKET, Fixed::from_int(4), ONE, MODELLER);
    own.add_evidence(BOX, Fixed::from_int(5), ONE, MODELLER);

    assert_eq!(own.commit(&p), Some(BOX));
    assert_eq!(sally_model.commit(&p), Some(BASKET));
    assert_ne!(sally_model.commit(&p), own.commit(&p));
    assert!(r.by_name("witnessed").is_some());
}

#[test]
fn battery_true_belief_control() {
    // When the target has access to the truth, the model matches it (no over-correction).
    let w = weights();
    let p = params();
    let mut sally_model = NestedFrame::new(StableId(2), 1, LOCATION, [BASKET, BOX]);
    sally_model
        .observe_access(&w, WITNESSED, BOX, ONE, MODELLER)
        .unwrap();
    assert_eq!(sally_model.commit(&p), Some(BOX));
}

#[test]
fn battery_unexpected_contents() {
    let w = weights();
    let p = params();
    let mut model = NestedFrame::new(StableId(3), 1, CONTENTS, [PENCILS, CANDY]);
    model
        .observe_access(&w, WITNESSED, PENCILS, ONE, MODELLER)
        .unwrap();
    assert_eq!(
        model.admit(
            EvidenceOrder::World,
            CANDY,
            Fixed::from_int(9),
            ONE,
            MODELLER
        ),
        Err(ProjectionRejected)
    );
    assert_eq!(model.commit(&p), Some(PENCILS));
}

#[test]
fn battery_lie_believed_then_seen_through() {
    let w = weights();
    let p = params();

    // Believed: told the box with no counter-access.
    let mut victim = NestedFrame::new(StableId(4), 1, LOCATION, [BASKET, BOX]);
    victim
        .observe_access(&w, TOLD, BOX, ONE, StableId(5))
        .unwrap();
    assert_eq!(victim.commit(&p), Some(BOX), "the lie is believed");

    // Seen through: the modeller's access-built model of the speaker's own belief
    // (witnessed basket) out-ranks the speaker's contrary assertion (box).
    let mut speaker_model = NestedFrame::new(StableId(6), 1, LOCATION, [BASKET, BOX]);
    speaker_model
        .observe_access(&w, WITNESSED, BASKET, ONE, MODELLER)
        .unwrap();
    speaker_model
        .observe_access(&w, SAID, BOX, ONE, MODELLER)
        .unwrap();
    assert_eq!(speaker_model.commit(&p), Some(BASKET));
    assert!(detects_deception(&speaker_model, BOX, &p));
    assert!(!detects_deception(&speaker_model, BASKET, &p));
}

#[test]
fn battery_second_order_false_belief() {
    // Depth 2: the modeller models A's belief about B's belief, reachable only through
    // two access levels and distinct from the modeller's own knowledge.
    let w = weights();
    let p = params();
    let park = 50u32;
    let church = 60u32;
    let mut a_about_b = NestedFrame::new(StableId(11), 2, AttrKindId(2), [park, church]);
    a_about_b
        .observe_access(&w, WITNESSED, park, ONE, MODELLER)
        .unwrap();
    assert_eq!(a_about_b.depth(), 2);
    assert_eq!(a_about_b.commit(&p), Some(park));
}
