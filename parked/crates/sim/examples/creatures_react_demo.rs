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

//! Creatures-react (mechanism B3) watchable proof-of-mechanism: a MIND-LESS creature moves toward or away
//! from a nearby emitting being BY ITS OWN PERCEPTION, driven only by `learn::creature_being_direction` (the
//! magnitude-graded toward-pull) fed through its controller's being block against a heritable freely-signed
//! weight. This is the movement the shipped `full --creatures` run cannot show at founder-zero (a real pull
//! times a zero weight is zero movement); here a NON-ZERO being-weight is INJECTED, TEST-ONLY (never the
//! run_world default, so no authored reaction ships), to demonstrate that the wired mechanism produces
//! movement once selection has lifted a weight either way. The SIGN is what a lineage's selection sets: a
//! POSITIVE weight (this "hunter") steps toward the emitter, a NEGATIVE weight (this "fleer") steps away, and
//! nothing in the mechanism authors which; the demo injects both signs to show both outcomes are reachable
//! from the one wire. Deterministic: no RNG, so the walk replays bit-for-bit.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::Fixed;
use civsim_sim::controller::{Controller, ControllerLayout};
use civsim_sim::homeostasis::{Homeostasis, HomeostaticRegistry, INGEST, MOVE};
use civsim_sim::learn::creature_being_direction;
use civsim_sim::percept::PerceptRegistry;
use civsim_sim::AffordanceRegistry;
use civsim_world::Coord3;

/// A creature controller with an INJECTED being-attraction reaction norm: it wants to move (MOVE activation
/// from the bias) and it steers its MOVE heading along the being block's ATTRACTION direction (the pair the
/// creatures-react wire fills), scaled by `gain`. A positive gain steers TOWARD the perceived being, a
/// negative gain AWAY; the sign is the only thing that differs, and in the shipped world it is a heritable
/// freely-signed weight selection sets, never authored (here it is injected test-only to demonstrate both).
fn creature_with_being_weight(layout: &ControllerLayout, gain: Fixed) -> Controller {
    let n_in = layout.n_in();
    let bias = n_in - 1;
    let being_base = layout.being_input_base();
    // The being block is [avoid_dx, avoid_dy, attract_dx, attract_dy]; the wire fills the attraction pair.
    let attract_dx = being_base + 2;
    let attract_dy = being_base + 3;
    // MOVE is output 0 (activation), its heading dx/dy at outputs 1 and 2; the reaction-norm weight feeding
    // output `o` from input `i` is at flat index `o * n_in + i`.
    let mut w = vec![Fixed::ZERO; layout.weight_count()];
    w[bias] = Fixed::ONE; // MOVE activation from the bias: the creature wants to move
    w[n_in + attract_dx] = gain; // MOVE heading dx follows the being-attraction dx, scaled by the sign
    w[2 * n_in + attract_dy] = gain; // MOVE heading dy follows the being-attraction dy
    Controller::from_weights(layout.n_in(), layout.n_out(), layout.hidden(), w)
}

/// The horizontal distance (in cells) between two ground coordinates, for the readout.
fn dist(a: Coord3, b: Coord3) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    (dx * dx + dy * dy).sqrt()
}

fn main() {
    // A being-inclusive controller layout (the block the creature's percept fills), no percepts, no hidden.
    let homeo_reg = HomeostaticRegistry::dev_grazer();
    let layout = ControllerLayout::with_percepts_and_being(
        &homeo_reg,
        &AffordanceRegistry::dev_default(),
        &PerceptRegistry::empty(),
        true,
        0,
    );
    let homeo = Homeostasis::from_mass(&homeo_reg, Fixed::ONE);
    // A nearby emitting being, due east, and the creature's own perceived magnitude for it (a mid-strength
    // transduced activation; the reach that produced it already accounted for distance).
    let emitter = Coord3::ground(12, 0);
    let magnitude = Fixed::from_ratio(1, 2);
    let ticks = 14;

    println!(
        "Creatures-react (B3) demo: a mind-less creature moves by its OWN perception of a nearby being.\n\
         The emitter is at (12, 0). The creature perceives it (magnitude {:.2}) and steers by its being\n\
         block against an INJECTED heritable weight (test-only; the sign is what selection sets in the world).\n",
        magnitude.to_f64_lossy()
    );

    for (label, gain) in [
        ("HUNTER (positive weight -> toward)", Fixed::from_int(4)),
        ("FLEER  (negative weight -> away)  ", Fixed::from_int(-4)),
    ] {
        let controller = creature_with_being_weight(&layout, gain);
        let mut here = Coord3::ground(0, 0);
        let start = dist(here, emitter);
        println!("=== {label} ===");
        for t in 0..ticks {
            // The creature's OWN magnitude-graded toward-pull over the one perceived emitter (no belief, no
            // category), exactly what the run-path wire computes and writes into the being block.
            let (px, py) = creature_being_direction(here, &[(emitter, magnitude)]);
            let being = [Fixed::ZERO, Fixed::ZERO, px, py];
            let input = layout.build_input_full_with_conviction(
                &homeo,
                &BTreeSet::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &[],
                &[],
                &[],
                &[],
                &[],
                &being,
            );
            let (out, _) = controller.evaluate(&input, &[]);
            // Step by the SIGN of the decided MOVE heading (one cell per tick), so the walk is watchable.
            if let Some(d) = layout.decide(&out, &[MOVE, INGEST]) {
                if d.affordance == MOVE {
                    if let Some((hx, hy)) = d.heading {
                        let step = |h: Fixed| -> i32 {
                            if h > Fixed::ZERO {
                                1
                            } else if h < Fixed::ZERO {
                                -1
                            } else {
                                0
                            }
                        };
                        here = Coord3::ground(here.x + step(hx), here.y + step(hy));
                    }
                }
            }
            let d = dist(here, emitter);
            let bar = "#".repeat((d.min(20.0)) as usize);
            println!(
                "  tick {t:>2}: creature at ({:>3}, {:>3})  distance {d:>5.1} |{bar}",
                here.x, here.y
            );
        }
        let end = dist(here, emitter);
        // Self-verifying: a positive weight closes the distance (toward), a negative one opens it (away).
        if gain > Fixed::ZERO {
            assert!(
                end < start,
                "the hunter (positive weight) moves TOWARD the emitter"
            );
        } else {
            assert!(
                end > start,
                "the fleer (negative weight) moves AWAY from the emitter"
            );
        }
        println!("  start distance {start:.1} -> end distance {end:.1}\n");
    }
    println!(
        "Both signs reachable from ONE wire: the creature perceives (creature_being_direction), the being\n\
         block carries the pull, and only the sign of its heritable weight decides toward vs away. In the\n\
         shipped world that weight is founder-zero and the sign EMERGES from creature selection (the queued\n\
         reproduction/behaviour-selection slice); nothing here authors it. Deterministic: this replays exactly."
    );
}
