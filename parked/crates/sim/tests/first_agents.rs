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

//! The first multi-agent scene, run through the public API. Three minds perceive, pass
//! a rumour, hold a false belief, and run a deception that is seen through, all from the
//! resolved belief (Part 9) and theory-of-mind (Part 37) mechanisms. The scene proves
//! the cognition works end to end between agents, and that it is deterministic: every
//! mind's state hashes identically on replay and regardless of the order its evidence
//! arrived. The weights and thresholds are clearly-labelled fixtures, not the owner's
//! reserved numbers.

use civsim_bio::evidence::{AttrKindId, InferenceParams};
use civsim_bio::tom::{AccessChannelId, AccessWeights};
use civsim_core::{Fixed, StableId};
use civsim_sim::{AccessObs, Mind};

const LOCATION: AttrKindId = AttrKindId(0);
const BASKET: u32 = 10;
const BOX: u32 = 20;
const HYPS: [u32; 2] = [BASKET, BOX];

const WITNESSED: AccessChannelId = AccessChannelId(1);
const TOLD: AccessChannelId = AccessChannelId(2);

const ANNA: StableId = StableId(1); // the witness
const BORIS: StableId = StableId(2); // the hearer
const CLARA: StableId = StableId(3); // the skeptic who watched Anna
const MARBLE: StableId = StableId(99);

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

fn weights() -> AccessWeights {
    // Fixture weights, witnessed strictly above told (the hard constraint).
    AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (TOLD, Fixed::from_int(3))])
}

#[test]
fn three_minds_perceive_gossip_hold_a_false_belief_and_see_a_lie() {
    let p = params();
    let w = weights();
    let witnessed = Fixed::from_int(4);
    let told = Fixed::from_int(3);
    let strong = Fixed::from_int(9);

    let mut anna = Mind::new(ANNA, Fixed::ONE);
    let mut boris = Mind::new(BORIS, Fixed::ONE);
    let mut clara = Mind::new(CLARA, Fixed::ONE);

    // 1. Anna witnesses the marble in the basket.
    anna.consider(MARBLE, LOCATION, HYPS, BASKET, witnessed, ANNA);
    assert_eq!(anna.belief(MARBLE, LOCATION, &p), Some(BASKET));

    // 2. Anna tells Boris; he comes to believe it too.
    boris.consider(MARBLE, LOCATION, HYPS, BASKET, told, ANNA);
    assert_eq!(boris.belief(MARBLE, LOCATION, &p), Some(BASKET));

    // 3. Anna, having told Boris, models that Boris now believes the basket.
    let told_boris = AccessObs {
        channel: TOLD,
        toward: BASKET,
        from: ANNA,
    };
    anna.model(&w, BORIS, LOCATION, HYPS, told_boris).unwrap();
    assert_eq!(anna.modeled_belief(BORIS, LOCATION, &p), Some(BASKET));

    // 4. The marble is moved to the box and Anna sees it move. Anna now believes the
    //    box, but Boris (who was not there) still believes the basket, and Anna's model
    //    of Boris correctly holds that false belief, distinct from Anna's own.
    anna.consider(MARBLE, LOCATION, HYPS, BOX, strong, ANNA);
    assert_eq!(anna.belief(MARBLE, LOCATION, &p), Some(BOX));
    assert_eq!(boris.belief(MARBLE, LOCATION, &p), Some(BASKET));
    assert_eq!(anna.modeled_belief(BORIS, LOCATION, &p), Some(BASKET));
    assert_ne!(
        anna.modeled_belief(BORIS, LOCATION, &p),
        anna.belief(MARBLE, LOCATION, &p),
        "Anna's model of Boris diverges from her own belief (not projection)"
    );

    // 5. Clara watched Anna witness the box, so Clara models that Anna believes the box.
    let saw_anna = AccessObs {
        channel: WITNESSED,
        toward: BOX,
        from: CLARA,
    };
    clara.model(&w, ANNA, LOCATION, HYPS, saw_anna).unwrap();
    assert_eq!(clara.modeled_belief(ANNA, LOCATION, &p), Some(BOX));

    // 6. Anna lies to Clara that the marble is in the basket. Clara sees through it,
    //    because her access-built model of Anna says Anna believes the box. The gate
    //    means Clara does not adopt the lie.
    let asserted = BASKET;
    assert!(clara.detects_lie(ANNA, LOCATION, asserted, &p));
    if !clara.detects_lie(ANNA, LOCATION, asserted, &p) {
        clara.consider(MARBLE, LOCATION, HYPS, asserted, told, ANNA);
    }
    assert_eq!(
        clara.belief(MARBLE, LOCATION, &p),
        None,
        "Clara refused the lie, so she formed no belief from it"
    );
}

#[test]
fn the_scene_is_reproducible_and_order_independent() {
    let p = params();
    let w = weights();
    let witnessed = Fixed::from_int(4);
    let strong = Fixed::from_int(9);

    // Build Anna's epistemic state in two different evidence orders.
    let build = |reversed: bool| -> u128 {
        let mut anna = Mind::new(ANNA, Fixed::ONE);
        let told_boris = AccessObs {
            channel: TOLD,
            toward: BASKET,
            from: ANNA,
        };
        let apply = |which: u8, a: &mut Mind| match which {
            0 => a.consider(MARBLE, LOCATION, HYPS, BASKET, witnessed, ANNA),
            1 => a.consider(MARBLE, LOCATION, HYPS, BOX, strong, ANNA),
            _ => a.model(&w, BORIS, LOCATION, HYPS, told_boris).unwrap(),
        };
        let order: [u8; 3] = if reversed { [2, 1, 0] } else { [0, 1, 2] };
        for which in order {
            apply(which, &mut anna);
        }
        anna.state_hash(&p, &p)
    };

    let forward = build(false);
    assert_eq!(forward, build(false), "replay reproduces the same mind");
    assert_eq!(
        forward,
        build(true),
        "the mind's state is independent of the order evidence arrived"
    );
}
