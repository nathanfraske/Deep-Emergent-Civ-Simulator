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

//! The Dawn Band: the working-prototype capstone. A band of minds, built from the
//! development fixtures manifest, runs every phase together over many ticks: they
//! perceive a placed event, want and choose actions, pass the news as gossip while
//! modelling each other, and coordinate a shared word for a concept. The scene asserts
//! that each emergent property appears and that the whole run replays bit for bit.
//!
//! Everything stochastic is keyed on the seed, and every calibration is loaded from the
//! clearly-labelled fixtures profile, not invented; a production run would load the
//! authoritative manifest and fail loud until the owner sets the real numbers.

use civsim_core::{Fixed, StableId};
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::decision::{
    ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef, DriveId,
};
use civsim_sim::evidence::AttrKindId;
use civsim_sim::language::{ArticulationSubstrate, ConceptId, LanguageParams};
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::world::{Trace, World};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/profiles/dev-fixtures.toml"
);
const RESERVED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/reserved.toml"
);

const LOCATION: AttrKindId = AttrKindId(0);
const RIVER: u32 = 10;
const CAMP: u32 = 20;
const HERE: u32 = 1;

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

// A small fixture behaviour: a hunger drive that rises, and forage/rest actions.
fn behaviour() -> Behaviour {
    let hunger = DriveId(0);
    let ramp = Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]);
    Behaviour {
        drives: vec![DriveDef {
            id: hunger,
            rise_per_tick: Fixed::from_ratio(1, 5),
            satisfy_amount: Fixed::from_ratio(1, 2),
        }],
        curves: vec![ramp],
        actions: vec![
            ActionDef {
                id: ActionId(0), // forage
                weight: Fixed::ONE,
                considerations: vec![Consideration {
                    drive: hunger,
                    curve: 0,
                }],
                satisfies: vec![hunger],
            },
            ActionDef {
                id: ActionId(1), // rest
                weight: Fixed::from_ratio(1, 4),
                considerations: vec![],
                satisfies: vec![],
            },
        ],
    }
}

fn dawn_band(seed: u64, path: &str, profile: Profile) -> (World, Vec<StableId>) {
    let manifest = CalibrationManifest::load(path).expect("manifest loads");
    let mut w = World::from_manifest(&manifest, &channels(), profile)
        .expect("world builds from manifest")
        .with_seed(seed);
    w.set_behaviour(behaviour());
    w.set_language(LanguageParams::from_manifest(&manifest).expect("language fixture"));
    let (_substrate, forms) = ArticulationSubstrate::syllabic(
        ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo"].map(String::from),
        2,
        3,
    );
    w.set_form_system(forms);
    w.set_concepts([ConceptId(1)]);

    let band: Vec<StableId> = (0..5).map(|_| w.spawn(Fixed::ONE)).collect();
    for &m in &band {
        w.set_place(m, HERE);
    }
    // A witnessed event: the quarry is at the river. A highly salient trace co-located
    // with the band, so a witness forms the belief and then the band passes it on.
    w.emit_trace(Trace {
        id: StableId(900),
        place: HERE,
        channel: civsim_sim::SenseChannelId::DEFAULT,
        subject: StableId(99),
        attr: LOCATION,
        hyps: vec![RIVER, CAMP],
        value: RIVER,
        salience: Fixed::ONE,
        weight: Fixed::from_int(5),
        from: StableId(900),
    });
    (w, band)
}

#[test]
fn the_dawn_band_lives_and_replays() {
    let (mut w, band) = dawn_band(0xDA7, FIXTURES, Profile::Development);
    // With a live innovation rate the band's consensus is punctuated: it reaches one
    // shared word, then an occasional fresh coinage splits it and it re-converges. So we
    // check that consensus is reached during the run rather than that it holds at the
    // final tick, which would be a coin flip on whether an innovation had just fired.
    let c = ConceptId(1);
    let mut converged = false;
    for _ in 0..40 {
        w.tick(&[]);
        let w0 = w.word_for(band[0], c);
        if w0.is_some() && band.iter().all(|&m| w.word_for(m, c) == w0) {
            converged = true;
        }
    }

    let bp = *w.belief_params();

    // Perception and gossip: the whole band came to believe the quarry is at the river.
    let believers = band
        .iter()
        .filter(|&&m| w.mind(m).unwrap().belief(StableId(99), LOCATION, &bp) == Some(RIVER))
        .count();
    assert_eq!(believers, band.len(), "the news reached the whole band");

    // Decision: every agent has chosen an action.
    for &m in &band {
        assert!(w.last_action(m).is_some(), "every agent acted");
    }

    // Language: the band coined a word and reached a shared convention during the run.
    assert!(w.word_for(band[0], c).is_some(), "the band coined a word");
    assert!(
        converged,
        "the band reached one shared word during the run (the naming game converges)"
    );

    // Determinism: a fresh run of the same scene reproduces the same world exactly.
    let (mut w2, _) = dawn_band(0xDA7, FIXTURES, Profile::Development);
    for _ in 0..40 {
        w2.tick(&[]);
    }
    assert_eq!(
        w.state_hash(),
        w2.state_hash(),
        "the whole Dawn Band replays bit for bit"
    );

    // A different seed yields a different history.
    let (mut w3, _) = dawn_band(0x999, FIXTURES, Profile::Development);
    for _ in 0..40 {
        w3.tick(&[]);
    }
    assert_ne!(
        w.state_hash(),
        w3.state_hash(),
        "a different seed gives a different world"
    );
}

#[test]
fn the_dawn_band_lives_under_the_calibrated_manifest() {
    // The confirm: the band runs on the owner's set values from the authoritative
    // manifest under Calibrated, not the dev fixtures, so the cognition, gossip, and
    // language calibrations are exercised on the real numbers. The scene content (the
    // behaviour, the phonology pool, the trace) is test scaffolding, not reserved
    // values; only the owner's calibrations come from the manifest.
    let (mut w, band) = dawn_band(0xDA7, RESERVED, Profile::Calibrated);
    let c = ConceptId(1);
    let mut converged = false;
    for _ in 0..40 {
        w.tick(&[]);
        let w0 = w.word_for(band[0], c);
        if w0.is_some() && band.iter().all(|&m| w.word_for(m, c) == w0) {
            converged = true;
        }
    }

    let bp = *w.belief_params();
    let believers = band
        .iter()
        .filter(|&&m| w.mind(m).unwrap().belief(StableId(99), LOCATION, &bp) == Some(RIVER))
        .count();
    assert_eq!(
        believers,
        band.len(),
        "the calibrated band all come to believe the river"
    );

    assert!(
        w.word_for(band[0], c).is_some(),
        "the calibrated band coined a word"
    );
    assert!(
        converged,
        "the calibrated band reached one shared word during the run"
    );

    let (mut w2, _) = dawn_band(0xDA7, RESERVED, Profile::Calibrated);
    for _ in 0..40 {
        w2.tick(&[]);
    }
    assert_eq!(
        w.state_hash(),
        w2.state_hash(),
        "the calibrated run replays bit for bit"
    );
}
