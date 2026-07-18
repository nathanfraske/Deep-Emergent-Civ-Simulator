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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! A narrated run of the Dawn Band, so the emergence is visible rather than only
//! asserted. Run with: `cargo run -p civsim-sim --example dawn_band`.
//!
//! It is the same scene as the `dawn_band` integration test, built from the development
//! fixtures profile, printed tick by tick: who believes the news, what word each mind
//! uses for the concept, and what each chose to do. Every number is a labelled fixture,
//! never an owner value, and the run is keyed on a seed so it replays identically.

use civsim_bio::calibration::{CalibrationManifest, Profile};
use civsim_bio::decision::{
    ActionDef, ActionId, Behaviour, Consideration, Curve, DriveDef, DriveId, InputId,
};
use civsim_bio::evidence::AttrKindId;
use civsim_bio::tom::AccessChannelRegistry;
use civsim_core::{Fixed, StableId};
use civsim_sim::language::{ArticulationSubstrate, ConceptId, DriftParams, LangId, LanguageParams};
use civsim_sim::world::{Trace, World};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/profiles/dev-fixtures.toml"
);

const LOCATION: AttrKindId = AttrKindId(0);
const RIVER: u32 = 10;
const QUARRY: StableId = StableId(99);
const CONCEPT: ConceptId = ConceptId(1);
const HERE: u32 = 1;

const NAMES: [&str; 5] = ["Ada", "Boro", "Cael", "Dara", "Esk"];

fn channels() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        "[[channels]]\nid = 1\nname = \"witnessed\"\nmargin_steps = 1\n\
         [[channels]]\nid = 2\nname = \"told\"\nmargin_steps = 0\n\
         [[channels]]\nid = 3\nname = \"said\"\nmargin_steps = -1\n",
    )
    .unwrap()
}

fn behaviour() -> Behaviour {
    let hunger = DriveId(0);
    Behaviour {
        drives: vec![DriveDef {
            id: hunger,
            rise_per_tick: Fixed::from_ratio(1, 5),
            satisfy_amount: Fixed::from_ratio(1, 2),
        }],
        curves: vec![Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::ONE, Fixed::ONE),
        ])],
        actions: vec![
            ActionDef {
                id: ActionId(0),
                weight: Fixed::ONE,
                considerations: vec![Consideration {
                    input: InputId(hunger.0),
                    curve: 0,
                }],
                satisfies: vec![hunger],
            },
            ActionDef {
                id: ActionId(1),
                weight: Fixed::from_ratio(1, 4),
                considerations: vec![],
                satisfies: vec![],
            },
        ],
    }
}

fn build(seed: u64) -> (World, Vec<StableId>, ArticulationSubstrate) {
    let manifest = CalibrationManifest::load(FIXTURES).expect("fixtures load");
    let mut w = World::from_manifest(&manifest, &channels(), Profile::Development)
        .expect("world builds")
        .with_seed(seed);
    w.set_behaviour(behaviour());
    w.set_language(LanguageParams::from_manifest(&manifest).unwrap());
    // A placeholder articulation system (a dev fixture, not an authored phonetic inventory):
    // words for a concept are built by sampling these syllable primitives, and rendered to a
    // surface string through the substrate.
    let (substrate, forms) = ArticulationSubstrate::syllabic(
        ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du"].map(String::from),
        2,
        3,
    );
    w.set_form_system(forms);
    // Drift on, so the shared word changes over generations (regular form change, 33.4).
    w.set_drift(DriftParams::from_manifest(&manifest).unwrap());
    w.set_concepts([CONCEPT]);
    let band: Vec<StableId> = (0..5).map(|_| w.spawn(Fixed::ONE)).collect();
    for &m in &band {
        w.set_place(m, HERE);
    }
    w.emit_trace(Trace {
        id: StableId(900),
        place: HERE,
        channel: civsim_sim::SenseChannelId::DEFAULT,
        subject: QUARRY,
        attr: LOCATION,
        hyps: vec![RIVER, 20],
        value: RIVER,
        salience: Fixed::ONE,
        weight: Fixed::from_int(5),
        from: StableId(900),
    });
    (w, band, substrate)
}

fn belief_str(w: &World, m: StableId) -> &'static str {
    let bp = *w.belief_params();
    match w.mind(m).unwrap().belief(QUARRY, LOCATION, &bp) {
        Some(RIVER) => "river",
        Some(_) => "camp ",
        None => "  ?  ",
    }
}

fn word_str(w: &World, sub: &ArticulationSubstrate, m: StableId) -> String {
    match w.word_for(m, CONCEPT) {
        Some(word) => sub.render(&word),
        None => "-".to_string(),
    }
}

fn action_str(w: &World, m: StableId) -> &'static str {
    match w.last_action(m) {
        Some(ActionId(0)) => "forage",
        Some(ActionId(1)) => "rest  ",
        Some(_) => "?     ",
        None => "-     ",
    }
}

fn snapshot(w: &World, sub: &ArticulationSubstrate, band: &[StableId], label: &str) {
    println!("\n-- {label} (tick {}) --", w.clock());
    println!("    name    belief   word   doing");
    for (i, &m) in band.iter().enumerate() {
        println!(
            "    {:<6}  {}   {:<6}   {}",
            NAMES[i],
            belief_str(w, m),
            word_str(w, sub, m),
            action_str(w, m)
        );
    }
}

fn believers(w: &World, band: &[StableId]) -> usize {
    let bp = *w.belief_params();
    band.iter()
        .filter(|&&m| w.mind(m).unwrap().belief(QUARRY, LOCATION, &bp) == Some(RIVER))
        .count()
}

fn distinct_words(w: &World, sub: &ArticulationSubstrate, band: &[StableId]) -> usize {
    let mut words: Vec<String> = band
        .iter()
        .filter_map(|&m| w.word_for(m, CONCEPT))
        .map(|word| sub.render(&word))
        .collect();
    words.sort();
    words.dedup();
    words.len()
}

fn main() {
    let seed = 0xDA7;
    let (mut w, band, substrate) = build(seed);

    println!("The Dawn Band: five minds at one place, seed {seed:#x}.");
    println!(
        "A trace says the quarry is at the river. Watch the news, the word, and the work spread."
    );
    snapshot(&w, &substrate, &band, "before any tick");

    for tick in 1..=40u32 {
        w.tick(&[]);
        // A compact per-tick line; detail snapshots at a few moments.
        println!(
            "tick {:>2}: believe-river {}/5, distinct words {}, foraging {}/5",
            tick,
            believers(&w, &band),
            distinct_words(&w, &substrate, &band),
            band.iter()
                .filter(|&&m| w.last_action(m) == Some(ActionId(0)))
                .count(),
        );
        if tick == 1 || tick == 5 || tick == 15 {
            snapshot(&w, &substrate, &band, "snapshot");
        }
    }
    snapshot(&w, &substrate, &band, "final");

    println!("\nOutcome:");
    println!(
        "  the whole band believes the quarry is at the river: {}",
        believers(&w, &band) == band.len()
    );
    println!(
        "  the band settled on a single shared word for the concept: {} (word {})",
        distinct_words(&w, &substrate, &band) == 1,
        word_str(&w, &substrate, band[0])
    );
    if let Some(lang) = w.lineage(LangId(0)) {
        println!(
            "  the language underwent {} regular form changes as it drifted (33.4)",
            lang.change_log().len()
        );
    }

    // Determinism, shown rather than asserted: the same seed reproduces the same world.
    let (mut again, _, _) = build(seed);
    for _ in 0..40 {
        again.tick(&[]);
    }
    let (mut other, _, _) = build(0x999);
    for _ in 0..40 {
        other.tick(&[]);
    }
    println!("\nDeterminism:");
    println!("  state hash this run : {:032x}", w.state_hash());
    println!("  same seed replayed  : {:032x}", again.state_hash());
    println!("  different seed (999) : {:032x}", other.state_hash());
    println!(
        "  same seed matches: {}, different seed differs: {}",
        w.state_hash() == again.state_hash(),
        w.state_hash() != other.state_hash()
    );
}
