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

//! Booting the Mirror world under [`Profile::Calibrated`] and iterating its fail-loud chain (the
//! mirror-tempest-calibration arc). The scenario-resolution side (`scenarios/mirror.toml` resolved
//! against `calibration/reserved.toml`) is already proven by
//! `every_canonical_scenario_resolves_against_the_real_manifest`; the fail-louds this file surfaces
//! come from the world-BUILD, the `X::from_manifest` reads inside
//! [`civsim_sim::build_dawn_runner`] that refuse to run on a still-reserved value (Principle 11).
//!
//! While Mirror's boot path carries reserved values the test FAILS LOUD and names the first blocking
//! key, so each run maps the next calibration the owner must set. The determinism and
//! worker-invariance checks are guarded inside the success branch, so they are inert until the boot
//! succeeds and light up automatically once the owner has set the reserved values the read-set needs
//! (they follow the `determinism_harness.rs` / `world_build.rs` pattern: build twice from one seed
//! and compare `state_hash` after N ticks, and sweep the worker count for a bit-identical trace).
//!
//! The peoples here are a synthetic labelled fixture, not owner data, but they are now EMBODIED: each
//! founding race carries a mobile body plan and the dawn carries an [`EmbodimentGenesis`], so the
//! embodiment-gated physiology reads arm and a boot proves a full embodied Mirror stands up under the
//! calibrated profile (rather than a disembodied cognition-only dawn). The world-build `from_manifest`
//! read-set therefore now covers the embodiment path Mirror's real dawn would drive; while any of it
//! is still reserved the test FAILS LOUD and names the first blocking key. This file sets no
//! calibration value; setting them is the owner's, through the reserved-values panel.

#![allow(dead_code)]

use std::collections::BTreeMap;

use civsim_core::{Fixed, GaussApprox};
use civsim_sim::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
use civsim_sim::calibration::{CalibrationError, CalibrationManifest, Profile};
use civsim_sim::homeostasis::{AffordanceRegistry, HomeostaticRegistry};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::runner::Runner;
use civsim_sim::scenario::{Scenario, ScenarioResolution};
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    build_dawn_runner, Axiom, AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId,
    BreedingSystemRegistry, Channel, CognitionChannel, Curve, DawnPeoples, DominanceMode,
    EmbodimentGenesis, EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool,
    GeneSet, GeneticScheme, IntrinsicBeliefs, PersonalityProfile, PersonalityRegistry, Race,
    RaceId, ReproductionMode, SchemeId, SourceModeId, TraitAxisId, TraitDef, ValueAxisId,
    ValueProfile,
};
use civsim_world::{FlatBounded, TileMap};

/// The real reserved manifest, the owner's calibration of record.
const RESERVED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/reserved.toml"
);

/// The Mirror scenario file, the grounded baseline world.
const MIRROR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scenarios/mirror.toml");

/// A fixed boot seed, so a failed boot and a future successful boot are both reproducible.
const SEED: u64 = 0xB0_0D_1E;

/// The reserved manifest loaded (never mutated: this file sets no value).
fn reserved_manifest() -> CalibrationManifest {
    CalibrationManifest::load(RESERVED).expect("the reserved manifest loads")
}

/// The Mirror scenario resolved against the reserved manifest. Resolution is dial-only and already
/// proven to work against the real manifest, so this expects success; the fail-louds under test come
/// from the world-build below, not from here.
fn mirror_resolution(manifest: &CalibrationManifest) -> ScenarioResolution {
    Scenario::load(MIRROR)
        .expect("the Mirror scenario loads")
        .resolve(manifest)
        .expect("the Mirror scenario resolves against the reserved manifest (dials only)")
}

/// Mirror's terrain map, built from the scenario's resolved world STRUCTURE (the `earth` triad),
/// the same path the run harness uses (`run_world.rs`): a flat bounded grid generated from the
/// structure's biome set and worldgen params. The map content does not change the `from_manifest`
/// read-set (the population and field are armed after the reserved reads), so a modest grid suffices.
fn mirror_map(resolution: &ScenarioResolution, seed: u64) -> TileMap {
    let topo = FlatBounded::new(16, 12, 1);
    let structure = resolution.world_structure();
    TileMap::generate(seed, topo, &structure.biomes, &structure.worldgen)
}

/// The witnessed / told / said access-channel registry (a labelled fixture, matching `world_build.rs`).
/// Every channel carries `margin_steps`, so the theory-of-mind assertion ladder DERIVES and no
/// `tom.access_weight.<name>` manifest key is read on this path.
fn channels() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        "[[channels]]\nid = 1\nname = \"witnessed\"\nmargin_steps = 1\n\
         [[channels]]\nid = 2\nname = \"told\"\nmargin_steps = 0\n\
         [[channels]]\nid = 3\nname = \"said\"\nmargin_steps = -1\n",
    )
    .unwrap()
}

/// A labelled test race (copied from `world_build.rs`): two cognition genes, a two-locus biallelic
/// pool, an innate disposition, a binary breeding system, and a lifespan past maturity. Not owner data.
fn a_race(id: u32) -> Race {
    let genes = GeneSet {
        genes: vec![
            GeneDef {
                id: GeneId(0),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::ReasoningAcuity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            },
            GeneDef {
                id: GeneId(1),
                effects: vec![GeneEffect {
                    channel: Channel::Cognition(CognitionChannel::MemoryCapacity),
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode::additive(),
            },
        ],
    };
    let pool = GenePool::new(
        SchemeId(0),
        20,
        vec![Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)],
    );
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 3)]),
        axioms: vec![Axiom {
            axis: AxiomAxisId(0),
            stance: Fixed::from_ratio(1, 2),
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 5,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: Fixed::from_ratio(1, 2),
            evidence: EvidenceRing::new(4),
        }],
        epistemic: EpistemicStance::new(
            [(SourceModeId(1), Fixed::ONE)],
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        ),
    };
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: Fixed::ZERO,
        additive_mutation_step: Fixed::ZERO,
        gauss: GaussApprox::default(),
    };
    Race::new(
        RaceId(id),
        genes,
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
        Fixed::ZERO,
        80,
        18,
    )
    .with_breeding(BreedingSystemId(0))
}

/// A one-axis personality profile (copied from `world_build.rs`). Labelled fixture.
fn a_personality() -> PersonalityProfile {
    PersonalityProfile::new([TraitDef {
        axis: TraitAxisId(0),
        plasticity_curve: Curve::new([(Fixed::ZERO, Fixed::ONE), (Fixed::ONE, Fixed::ZERO)]),
        maturity_target: Fixed::from_ratio(1, 2),
    }])
}

/// A mobile development body plan (the thermal-coupling fixture copied from `world_build.rs`), so a
/// founder's walker has an anatomy to derive its physiology and thermoregulate from. Labelled
/// fixture, not owner data.
fn mobile_body() -> BodyPlan {
    BodyPlan {
        body_mass: Fixed::from_ratio(1, 2),
        encephalization: Fixed::from_ratio(1, 2),
        diet_breadth: Fixed::from_ratio(1, 2),
        weapons: vec![],
        covering: Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        },
        senses: vec![],
        locomotion: vec![1],
        organs: vec![],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// A synthetic EMBODIED dawn: two races on two bands, each carrying a mobile body plan, a personality
/// profile each, a binary breeding system, no mortality hazard, no language genesis, no biosphere, and
/// an embodiment genesis. Because `peoples.embodiment` is `Some`, the embodiment-gated physiology reads
/// arm and are exercised, so a boot proves a full embodied Mirror stands up under the calibrated
/// profile (or fail-louds on the next reserved physiology key). Labelled fixture, not owner data.
fn synthetic_peoples() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race(0).with_body_plan(mobile_body()));
    races.insert(RaceId(1), a_race(1).with_body_plan(mobile_body()));
    let bands = vec![
        BandSpec {
            race: RaceId(0),
            place: 10,
            members: 3,
        },
        BandSpec {
            race: RaceId(1),
            place: 20,
            members: 4,
        },
    ];
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    let mut personality = PersonalityRegistry::new();
    personality.set(RaceId(0), a_personality());
    personality.set(RaceId(1), a_personality());
    DawnPeoples {
        races,
        bands,
        breeding,
        personality,
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_thermal(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

/// Boot Mirror through the production world-build path under the given profile, returning the runner
/// or the first fail-loud calibration error. Deterministic in `seed` (the map generation and the dawn
/// draws are seeded), so two boots with one seed are bit-identical.
fn boot(profile: Profile, seed: u64) -> Result<Runner, CalibrationError> {
    let manifest = reserved_manifest();
    let resolution = mirror_resolution(&manifest);
    let map = mirror_map(&resolution, seed);
    build_dawn_runner(
        &manifest,
        &channels(),
        profile,
        &resolution,
        &map,
        &synthetic_peoples(),
        seed,
    )
}

/// A crisp one-line description of a fail-loud calibration error, naming the blocking key(s) so a run
/// of this test maps the next reserved value the owner must set for Mirror to boot.
fn describe(e: &CalibrationError) -> String {
    match e {
        CalibrationError::Reserved(id) => {
            format!("RESERVED calibration key still blocking boot: {id}")
        }
        CalibrationError::UnsatisfiedRequirements(ids) => {
            format!(
                "profile gate refused: reserved required keys: {}",
                ids.join(", ")
            )
        }
        CalibrationError::BadValue { id, detail } => format!("bad value for {id}: {detail}"),
        CalibrationError::Unknown(id) => format!("unknown calibration key: {id}"),
        other => format!("{other}"),
    }
}

#[test]
fn mirror_boots_under_calibrated_or_names_the_next_reserved_key() {
    match boot(Profile::Calibrated, SEED) {
        Ok(mut runner) => {
            // The boot succeeded: the owner has set every reserved value on Mirror's boot path. The
            // guarded checks below now light up, proving the calibrated Mirror boots AND replays
            // deterministically. (Inert until this branch is reached, so no reserved value is
            // required for the file to compile and run.)
            eprintln!(
                "Mirror booted under Profile::Calibrated: population {}",
                runner.world().map(|w| w.population()).unwrap_or(0)
            );

            const TICKS: usize = 30;

            // Determinism: build twice from one seed and compare the state hash after N ticks.
            let hash_after = |seed: u64, workers: usize| -> u128 {
                let mut r = boot(Profile::Calibrated, seed).expect("a calibrated Mirror re-boots");
                if let Some(w) = r.world_mut() {
                    w.set_workers(workers);
                }
                for _ in 0..TICKS {
                    r.step();
                }
                r.state_hash()
            };
            assert_eq!(
                hash_after(SEED, 1),
                hash_after(SEED, 1),
                "the calibrated Mirror boot replays bit for bit from one seed"
            );

            // Worker-invariance: the whole trace must be bit-identical across worker widths, or a beat
            // leaked the thread schedule (R-CMD-ORDER; the determinism_harness.rs contract).
            let baseline = hash_after(SEED, 1);
            for workers in [2usize, 3, 8] {
                assert_eq!(
                    baseline,
                    hash_after(SEED, workers),
                    "the calibrated Mirror trace diverged at {workers} workers"
                );
            }

            // Also drive the runner we already booted so the success path is exercised end to end.
            for _ in 0..TICKS {
                runner.step();
            }
            assert_eq!(runner.clock(), TICKS as u64, "the booted runner advanced");
        }
        Err(e) => {
            // Still reserved (the expected state today): fail loud and name the blocking key, so
            // running this test surfaces the next calibration Mirror needs. This does NOT set the
            // value; setting it is the owner's.
            panic!(
                "Mirror cannot yet boot under Profile::Calibrated.\n  {}\n  full error: {e}",
                describe(&e)
            );
        }
    }
}
