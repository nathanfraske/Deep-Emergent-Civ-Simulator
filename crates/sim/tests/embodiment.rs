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

//! The body arc closing the R-BEHAVIOR-EVOLVE loop: a wound to the per-part body (R-WOUND) lowers the
//! derived integrity the evolved controller reads (Part 35, Part 8.4), and a natural weapon lets a
//! controller decide to strike, the predator-prey closure. These span the body, homeostasis, and
//! controller modules, so they live as an integration test.

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::Fixed;
use civsim_sim::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
use civsim_sim::body::{
    apply_insult, Body, BodyParams, DamageModeRegistry, FluidRegistry, Insult, TissueRegistry,
    BLOOD, CUT,
};
use civsim_sim::controller::{Controller, ControllerLayout};
use civsim_sim::homeostasis::{
    AffordanceRegistry, Homeostasis, HomeostaticRegistry, INTEGRITY, MOVE, STRIKE,
};

fn plan(mass: (i64, i64), legs: usize, weapons: usize) -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(mass.0, mass.1),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: (0..weapons)
            .map(|i| Part {
                kind: i as u16,
                development: Fixed::from_ratio(3, 4),
            })
            .collect(),
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        }],
        locomotion: (0..legs).map(|_| 1u16).collect(),
        organs: vec![],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(1, 2),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

#[test]
fn a_wound_lowers_the_integrity_the_controller_reads() {
    // The R-WOUND to R-BEHAVIOR-EVOLVE link: the derived body integrity is refreshed into the
    // homeostatic integrity axis, and the controller's percept carries it, so a hurt being reads its
    // own condition.
    let reg = HomeostaticRegistry::dev_embodied(); // energy, water, integrity
    let afford = AffordanceRegistry::dev_default();
    let layout = ControllerLayout::new(&reg, &afford, 0);
    let params = BodyParams::dev_default();
    let fluids = FluidRegistry::dev_default();

    let mut body = Body::from_body_plan(
        &plan((3, 4), 4, 0),
        BLOOD,
        &params,
        &BodyPlanRegistry::dev_default(),
    );
    let mut homeo = Homeostasis::from_mass(&reg, Fixed::from_ratio(3, 4));

    // Refresh integrity from the body (the derived mirror, design Part 35).
    homeo.set_level(INTEGRITY, body.integrity(&fluids));
    assert_eq!(
        homeo.level(INTEGRITY),
        Fixed::ONE,
        "a whole body reads full integrity"
    );

    // The integrity input the controller sees (axis index 2 -> input base 5*2 = 10, the level slot).
    let input_before =
        layout.build_input(&homeo, &BTreeSet::new(), &BTreeMap::new(), &BTreeMap::new());
    assert_eq!(
        input_before[10],
        Fixed::ONE,
        "the percept carries full integrity"
    );

    // Wound the torso.
    let modes = DamageModeRegistry::dev_default();
    let tissues = TissueRegistry::dev_default();
    let torso = body.parts.iter().position(|p| p.name == "torso").unwrap();
    let insult = Insult {
        mode: CUT,
        force: Fixed::from_int(200),
        contact_area: Fixed::from_ratio(1, 100_000),
        delivered_energy: Fixed::from_int(1),
        delta_t: Fixed::ZERO,
    };
    apply_insult(&mut body, torso, &insult, &modes, &tissues, &params);

    homeo.set_level(INTEGRITY, body.integrity(&fluids));
    assert!(
        homeo.level(INTEGRITY) < Fixed::ONE,
        "the wound lowered the integrity reserve"
    );
    let input_after =
        layout.build_input(&homeo, &BTreeSet::new(), &BTreeMap::new(), &BTreeMap::new());
    assert!(
        input_after[10] < input_before[10],
        "and the controller's percept now carries the wound"
    );
}

#[test]
fn a_body_with_a_weapon_affords_and_can_decide_to_strike() {
    // The predator-prey closure: a body bearing a natural weapon affords STRIKE, and a controller
    // whose strike output wins chooses it. A weaponless body never can.
    let reg = HomeostaticRegistry::dev_default();
    let afford = AffordanceRegistry::dev_predator(); // move, ingest, strike
    let layout = ControllerLayout::new(&reg, &afford, 0);

    let armed = plan((1, 1), 4, 1); // bears a weapon
    let unarmed = plan((1, 1), 4, 0);
    assert!(
        afford.afforded(&armed).contains(&STRIKE),
        "a body with a weapon affords a strike"
    );
    assert!(
        !afford.afforded(&unarmed).contains(&STRIKE),
        "a weaponless body affords no strike"
    );

    // A controller that drives the strike output high: with strike afforded, it decides to strike.
    // Output layout (canonical id order): move [act,dx,dy], ingest [act], strike [act,dx,dy].
    let n_in = layout.n_in();
    let mut w = vec![Fixed::ZERO; layout.weight_count()];
    let bias = n_in - 1;
    // strike activation is the 5th output (index 4): move=0..2, ingest=3, strike=4..6.
    w[4 * n_in + bias] = Fixed::ONE; // strike wants to fire (from the bias)
    let controller = Controller::from_weights(n_in, layout.n_out(), 0, w);

    let homeo = Homeostasis::from_mass(&reg, Fixed::from_ratio(1, 2));
    let input = layout.build_input(&homeo, &BTreeSet::new(), &BTreeMap::new(), &BTreeMap::new());
    let (out, _) = controller.evaluate(&input, &[]);
    let decision = layout.decide(&out, &afford.afforded(&armed)).unwrap();
    assert_eq!(
        decision.affordance, STRIKE,
        "the controller decides to strike"
    );
    // Against a body that cannot strike, the same controller falls back to another afforded act.
    let decision2 = layout.decide(&out, &afford.afforded(&unarmed)).unwrap();
    assert_ne!(
        decision2.affordance, STRIKE,
        "a body that cannot strike does not decide to"
    );
    // MOVE is the lowest-id fallback with zero activation.
    assert_eq!(decision2.affordance, MOVE);
}
