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

//! The runtime spine driven from the manifest path (RUNBOOK section 3). A world of
//! minds is built from the development fixtures profile, run through a scripted
//! multi-tick scene, and checked for replay determinism. A second test runs the same
//! scene under the now-calibrated authoritative manifest, and a third proves the
//! fail-loud gate still refuses a system whose required value is reserved, so production
//! never runs on an unset number.

use civsim_core::{Fixed, StableId};
use civsim_sim::agent::AccessObs;
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::evidence::AttrKindId;
use civsim_sim::tom::{AccessChannelId, AccessChannelRegistry};
use civsim_sim::world::{Stimulus, TickInput, World};
use civsim_world::OrbitalElements;

const LOCATION: AttrKindId = AttrKindId(0);
const BASKET: u32 = 10;
const BOX: u32 = 20;
const WITNESSED: AccessChannelId = AccessChannelId(1);
const TOLD: AccessChannelId = AccessChannelId(2);
const MARBLE: StableId = StableId(99);

// The committed files, found by a compile-time path so the test does not depend on the
// working directory.
const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/profiles/dev-fixtures.toml"
);
const RESERVED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/reserved.toml"
);

fn channels() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        r#"
[[channels]]
id = 1
name = "witnessed"
[[channels]]
id = 2
name = "told"
[[channels]]
id = 3
name = "said"
"#,
    )
    .unwrap()
}

fn observe(mind: StableId, ordinal: u32, toward: u32, weight: i32, from: StableId) -> TickInput {
    TickInput {
        mind,
        ordinal,
        stim: Stimulus::Observe {
            subject: MARBLE,
            attr: LOCATION,
            hyps: vec![BASKET, BOX],
            toward,
            weight: Fixed::from_int(weight),
            from,
        },
    }
}

fn model(
    mind: StableId,
    ordinal: u32,
    target: StableId,
    channel: AccessChannelId,
    toward: u32,
    from: StableId,
) -> TickInput {
    TickInput {
        mind,
        ordinal,
        stim: Stimulus::Model {
            target,
            attr: LOCATION,
            hyps: vec![BASKET, BOX],
            obs: AccessObs {
                channel,
                toward,
                from,
            },
        },
    }
}

// Run the scripted scene and return the world's per-tick state hashes.
fn run_scene(path: &str, profile: Profile) -> Vec<u128> {
    let manifest = CalibrationManifest::load(path).expect("manifest loads");
    // The reserved manifest leaves the world's orbit unset (owner-set per world), so a calibrated
    // build supplies a labelled fixture orbit rather than a fabricated one; the dev fixtures declare
    // their own orbit, so the production from_manifest derives the life cadence from it.
    let built = match profile {
        Profile::Calibrated => World::from_manifest_with_orbital(
            &manifest,
            &channels(),
            profile,
            OrbitalElements::dev_earth(),
        ),
        Profile::Development => World::from_manifest(&manifest, &channels(), profile),
    };
    let mut w = built.expect("world builds from manifest");

    let anna = w.spawn(Fixed::ONE);
    let boris = w.spawn(Fixed::ONE);
    let clara = w.spawn(Fixed::ONE);

    let mut hashes = Vec::new();

    // Tick 1: Anna witnesses the basket; Boris is told the basket.
    w.tick(&[
        observe(anna, 0, BASKET, 4, anna),
        observe(boris, 0, BASKET, 3, anna),
    ]);
    hashes.push(w.state_hash());

    // Tick 2: Anna models that Boris (told) believes the basket; Clara watched Anna
    // witness the box.
    w.tick(&[
        model(anna, 0, boris, TOLD, BASKET, anna),
        model(clara, 0, anna, WITNESSED, BOX, clara),
    ]);
    hashes.push(w.state_hash());

    // Tick 3: the marble moves and Anna sees it move to the box.
    w.tick(&[observe(anna, 0, BOX, 9, anna)]);
    hashes.push(w.state_hash());

    // Assert the scene played out: Anna believes the box, Boris still the basket,
    // Anna's model of Boris is the basket (a false belief, not projection), and Clara
    // sees through Anna asserting the basket.
    let bp = *w.belief_params();
    let mp = *w.meta_params();
    assert_eq!(
        w.mind(anna).unwrap().belief(MARBLE, LOCATION, &bp),
        Some(BOX)
    );
    assert_eq!(
        w.mind(boris).unwrap().belief(MARBLE, LOCATION, &bp),
        Some(BASKET)
    );
    assert_eq!(
        w.mind(anna).unwrap().modeled_belief(boris, LOCATION, &mp),
        Some(BASKET)
    );
    assert!(w
        .mind(clara)
        .unwrap()
        .detects_lie(anna, LOCATION, BASKET, &mp));

    hashes
}

#[test]
fn the_manifest_driven_scene_replays_deterministically() {
    let a = run_scene(FIXTURES, Profile::Development);
    let b = run_scene(FIXTURES, Profile::Development);
    assert_eq!(a, b, "the same scene reproduces the same per-tick hashes");
    assert_eq!(a.len(), 3);
    // The world changes from tick to tick (it is not stuck).
    assert_ne!(a[0], a[1]);
    assert_ne!(a[1], a[2]);
}

#[test]
fn the_authoritative_manifest_runs_the_calibrated_prototype() {
    // The prototype slice of reserved values is now set, so the authoritative manifest
    // builds and the cognition scene plays out under the owner's calibrated numbers,
    // replaying deterministically.
    let a = run_scene(RESERVED, Profile::Calibrated);
    let b = run_scene(RESERVED, Profile::Calibrated);
    assert_eq!(a, b, "the calibrated scene replays deterministically");
}

#[test]
fn the_fail_loud_gate_still_refuses_a_reserved_system() {
    // A system whose required value is still reserved must refuse under Calibrated, so
    // the fail-loud guarantee holds for everything not yet calibrated.
    let manifest = CalibrationManifest::load(RESERVED).expect("reserved manifest loads");
    let gate = manifest.gate(Profile::Calibrated, &["langdet.salience_decay_rate"]);
    assert!(
        gate.is_err(),
        "an uncalibrated system must still fail loud under Calibrated"
    );
}
