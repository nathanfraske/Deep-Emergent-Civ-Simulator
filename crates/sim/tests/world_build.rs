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

//! The production world-build path (design Part 28; the world-wiring handoff, section 4): a resolved
//! scenario, a manifest, and a declared dawn population assemble into a running Runner that seeds
//! genome-real founders, ticks deterministically, advances its inner world, and derives its field
//! from the selected medium. Every number here is a labelled fixture (the dev-fixtures profile and
//! the inline test races), never an owner value.

use std::collections::BTreeMap;

use civsim_bio::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
use civsim_bio::calibration::{CalibrationManifest, Profile};
use civsim_bio::tom::AccessChannelRegistry;
use civsim_core::{Fixed, GaussApprox, StableId};
use civsim_foundation::scenario::{Scenario, ScenarioResolution};
use civsim_foundation::sensorium::SenseChannelId;
use civsim_sim::homeostasis::{
    AffordanceRegistry, HomeostaticAxisDef, HomeostaticRegistry, INTEGRITY,
};
use civsim_sim::langmod::PerceptualParams;
use civsim_sim::language::{ConceptId, FeatureDimId, ProductionModalityId};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::{
    append_controller_block, append_morphogen_block, append_scalar_channel, build_dawn_runner,
    express_program, grow, taxis_move_weights, Articulation, Axiom, AxiomAxisId, AxisSpec,
    BandSpec, BreedingSystem, BreedingSystemId, BreedingSystemRegistry, CapabilityCaps,
    CapabilityRefs, Channel, CognitionChannel, ControllerLayout, Curve, DawnPeoples, DominanceKind,
    DominanceMode, EmbodimentGenesis, EpistemicStance, EvidenceRing, FunctionLawRegistry, GeneDef,
    GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs, LanguageGenesis,
    MorphogenParamId, MorphogenProgram, PersonalityProfile, PersonalityRegistry, Race, RaceId,
    ReproductionMode, SchemeId, SourceModeId, TraitAxisId, TraitDef, ValueAxisId, ValueProfile,
};
use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/profiles/dev-fixtures.toml"
);

const RESERVED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/reserved.toml"
);

fn manifest() -> CalibrationManifest {
    CalibrationManifest::load(FIXTURES).expect("the dev-fixtures profile loads")
}

fn channels() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        "[[channels]]\nid = 1\nname = \"witnessed\"\nmargin_steps = 1\n\
         [[channels]]\nid = 2\nname = \"told\"\nmargin_steps = 0\n\
         [[channels]]\nid = 3\nname = \"said\"\nmargin_steps = -1\n",
    )
    .unwrap()
}

/// A labelled test race: two cognition genes (acuity, memory), a two-locus biallelic pool, an innate
/// disposition, a binary breeding system, and a lifespan comfortably past maturity. Not owner data.
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

/// A one-axis personality profile maturing toward a positive target, so the personality beat has
/// something to drift once the cadence fires. Labelled fixture.
fn a_personality() -> PersonalityProfile {
    PersonalityProfile::new([TraitDef {
        axis: TraitAxisId(0),
        plasticity_curve: Curve::new([(Fixed::ZERO, Fixed::ONE), (Fixed::ONE, Fixed::ZERO)]),
        maturity_target: Fixed::from_ratio(1, 2),
    }])
}

/// Two races on two bands, a personality profile for each, a binary breeding system, no mortality
/// hazard. The peoples a world-build test assembles a Runner from.
fn peoples() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_race(0));
    races.insert(RaceId(1), a_race(1));
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
        embodiment: None,
        biosphere: None,
    }
}

fn a_map(seed: u64) -> TileMap {
    let topo = FlatBounded::new(16, 12, 1);
    let biomes = BiomeSet::dev_default();
    TileMap::generate(seed, topo, &biomes, &WorldgenParams::dev_default())
}

fn a_scenario(medium: Option<&str>) -> Scenario {
    let toml = match medium {
        Some(m) => format!("[scenario]\nid = \"w\"\nname = \"W\"\nmedium = \"{m}\"\n"),
        None => "[scenario]\nid = \"w\"\nname = \"W\"\n".to_string(),
    };
    Scenario::from_toml_str(&toml).unwrap()
}

/// A labelled voice channel the founding races hear speech on.
const VOICE: SenseChannelId = SenseChannelId(1);

/// An articulating race: a race that declares a vocal-tract scale (bending the shared base geometry)
/// and a hearing resolution, so the founder step can derive its phonetic form system. Labelled
/// fixtures, not owner data.
fn articulated_race(id: u32, vocal_tract_scale: Fixed) -> Race {
    a_race(id).with_articulation(Articulation {
        vocal_tract_scale,
        hearing_resolution: Fixed::from_int(20),
    })
}

/// A labelled language genesis: five candidate sounds (shared base geometry), air acoustics, a low
/// producibility threshold so both races produce a full inventory, and short words. Not owner data.
fn a_genesis() -> LanguageGenesis {
    LanguageGenesis {
        base_lengths: (12..=16).map(|cm| Fixed::from_ratio(cm, 100)).collect(),
        modality: ProductionModalityId(0),
        dim: FeatureDimId(0),
        sound_speed: Fixed::from_int(340),
        absorption_reference: Fixed::from_ratio(1, 100000000),
        path: Fixed::from_int(10),
        perceptual: PerceptualParams {
            modes: 3,
            freq_max: Fixed::from_int(100000),
            alpha_max: Fixed::from_int(10),
            tau_max: Fixed::from_int(100),
            confusability_cap: Fixed::from_int(1000),
        },
        capability: Fixed::ONE,
        producibility_threshold: Fixed::from_ratio(1, 2),
        word_min_len: 1,
        word_max_len: 2,
        hearing_channel: VOICE,
    }
}

/// Two articulating races (different vocal tracts) on three bands: race 0 at two separate places (so
/// its two bands diverge), race 1 at a third. A language genesis so the founder step arms derived
/// languages.
fn peoples_with_language() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), articulated_race(0, Fixed::ONE));
    races.insert(RaceId(1), articulated_race(1, Fixed::from_ratio(3, 4)));
    let bands = vec![
        BandSpec {
            race: RaceId(0),
            place: 10,
            members: 4,
        },
        BandSpec {
            race: RaceId(0),
            place: 20,
            members: 4,
        },
        BandSpec {
            race: RaceId(1),
            place: 30,
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
        language: Some(a_genesis()),
        embodiment: None,
        biosphere: None,
    }
}

#[test]
fn the_founder_step_arms_derived_languages_that_run_live_and_bands_converge_and_diverge() {
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    let mut runner = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples_with_language(),
        0x2E,
    )
    .expect("a language-armed dawn assembles");

    // Run the naming game long enough for the co-located bands to coordinate their words.
    for _ in 0..800 {
        runner.step();
    }
    let w = runner.world().unwrap();
    let ids = w.being_ids();
    let band_at = |place: u32| -> Vec<_> {
        ids.iter()
            .copied()
            .filter(|&id| w.place_of(id) == Some(place))
            .collect()
    };
    let band0 = band_at(10); // race 0
    let band1 = band_at(20); // race 0, a separate place
    let band2 = band_at(30); // race 1
    assert_eq!(band0.len(), 4);
    assert_eq!(band2.len(), 4);

    // The naming game ran live on the derived form system: the band coined words for the primes.
    let concepts: Vec<ConceptId> = (1..=65).map(ConceptId).collect();
    let has_words = concepts
        .iter()
        .filter(|&&c| w.word_for(band0[0], c).is_some())
        .count();
    assert!(
        has_words > 0,
        "the founder step armed a live language: the band coined words"
    );

    // Co-located members share far more words than members at separate places: the band coordinates
    // (co-location), while separated bands and the two races diverge.
    let shared = |a, b| {
        concepts
            .iter()
            .filter(|&&c| {
                let wa = w.word_for(a, c);
                wa.is_some() && wa == w.word_for(b, c)
            })
            .count()
    };
    let within_band0 = shared(band0[0], band0[1]);
    let across_places = shared(band0[0], band1[0]); // same race, separate places
    let across_races = shared(band0[0], band2[0]);
    assert!(
        within_band0 > 0,
        "co-located members converge on shared words"
    );
    assert!(
        within_band0 > across_places,
        "separated bands of one race diverge: within {within_band0} > across {across_places}"
    );
    assert!(
        within_band0 > across_races,
        "the two races speak different languages: within {within_band0} > across {across_races}"
    );
}

#[test]
fn without_a_language_genesis_the_naming_game_stays_inert() {
    // The genesis is what makes the naming game live: the same assembly with no LanguageGenesis coins
    // no word, because no lineage carries a form system.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    let mut runner = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples(), // no language genesis
        0x2E,
    )
    .unwrap();
    for _ in 0..200 {
        runner.step();
    }
    let w = runner.world().unwrap();
    assert!(
        w.being_ids()
            .iter()
            .all(|&id| w.word_for(id, ConceptId(1)).is_none()),
        "without a genesis no lineage carries a form system, so no word is coined"
    );
}

#[test]
fn the_language_armed_build_replays_bit_for_bit() {
    let run = |seed: u64| {
        let manifest = manifest();
        let resolution = a_scenario(None).resolve(&manifest).unwrap();
        let map = a_map(0xB0);
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &peoples_with_language(),
            seed,
        )
        .unwrap();
        for _ in 0..120 {
            runner.step();
        }
        runner.state_hash()
    };
    assert_eq!(
        run(0x2E),
        run(0x2E),
        "the language-armed dawn replays bit for bit"
    );
    assert_ne!(
        run(0x2E),
        run(0x2F),
        "a different seed coins a different language"
    );
}

#[test]
fn the_world_build_assembles_a_dawn_seeded_running_runner() {
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    let peoples = peoples();
    let mut runner = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        0xDA,
    )
    .expect("the production world-build assembles a Runner");

    // The dawn seeded a genome-real population: three plus four founders across the two bands.
    assert_eq!(
        runner.world().unwrap().population(),
        7,
        "the two bands seeded seven founders through the world-build path"
    );

    // Stepping the Runner advances its inner cognition world, not the field alone (the open question
    // the composite closes): the world's own clock tracks the runner's.
    let inner_before = runner.world().unwrap().clock();
    for _ in 0..30 {
        runner.step();
    }
    assert_eq!(runner.clock(), 30, "the runner advanced thirty ticks");
    assert_eq!(
        runner.world().unwrap().clock(),
        inner_before + 30,
        "the composed cognition world ticked every runner step"
    );

    // No mortality hazard was armed and the Earth-year cadence never fires in thirty ticks, so the
    // population is intact: the world-build path shrinks nothing on its own.
    assert_eq!(runner.world().unwrap().population(), 7);
}

#[test]
fn the_world_build_replays_bit_for_bit() {
    let run = |seed: u64| {
        let manifest = manifest();
        let resolution = a_scenario(None).resolve(&manifest).unwrap();
        let map = a_map(0xB0);
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &peoples(),
            seed,
        )
        .unwrap();
        for _ in 0..30 {
            runner.step();
        }
        runner.state_hash()
    };
    assert_eq!(run(0xABC), run(0xABC), "the same seed replays bit for bit");
    assert_ne!(
        run(0xABC),
        run(0xDEF),
        "a different seed builds a different world"
    );
}

#[test]
fn the_field_derives_from_the_medium_through_the_whole_assembly() {
    // Two worlds identical but for their ambient medium (one names water, one the default air) build
    // through the same path from the same seed and map. Only the field's diffusion coefficient
    // tracks the medium (k/(rho*c)), so the dawn worlds are bit-identical while the runners diverge:
    // the medium flows end to end through the world-build path, not just at the FieldCalib layer.
    let manifest = manifest();
    let map = a_map(0xB0);
    let peoples = peoples();

    let build = |medium: Option<&str>| {
        let resolution = a_scenario(medium).resolve(&manifest).unwrap();
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &peoples,
            0xF1E1,
        )
        .unwrap();
        for _ in 0..30 {
            runner.step();
        }
        runner
    };

    let air = build(None);
    let water = build(Some("water"));

    // The dawn (cognition) side is identical: the medium does not touch the world, only the field.
    assert_eq!(
        air.world().unwrap().state_hash(),
        water.world().unwrap().state_hash(),
        "the medium leaves the dawn population untouched"
    );
    // The runners diverge: air conducts heat faster than water, so their temperature fields differ
    // after stepping, and the field folds into the runner's hash.
    assert_ne!(
        air.state_hash(),
        water.state_hash(),
        "air and water fields diverge through the assembled runner"
    );
}

/// A sex-determined test race: `a_race`'s cognition genes plus a sex-determination gene whose
/// heterozygote is the heterogametic sex, a three-locus biallelic pool at one half, and the binary
/// anisogamous breeding system, so mature founders express two sexes and compatible pairs can breed.
/// Labelled fixtures, not owner data.
fn a_sexed_race(id: u32) -> Race {
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
            GeneDef {
                id: GeneId(2),
                effects: vec![GeneEffect {
                    channel: Channel::SexDetermination,
                    weight: Fixed::ONE,
                }],
                dominance: DominanceMode {
                    a: Fixed::ZERO,
                    d: Fixed::ONE,
                    kind: DominanceKind::Complete,
                },
            },
        ],
    };
    let pool = GenePool::new(
        SchemeId(0),
        20,
        vec![
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
            Fixed::from_ratio(1, 2),
        ],
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

/// A dawn of one sex-determined race on one band of a dozen founders, no mortality hazard, so a
/// multi-generation run grows.
fn sexed_peoples() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_sexed_race(0));
    let bands = vec![BandSpec {
        race: RaceId(0),
        place: 10,
        members: 12,
    }];
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands,
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: None,
        biosphere: None,
    }
}

#[test]
fn the_dawn_runner_reproduces_and_drifts_under_the_census_ne_across_generations() {
    // Real-world unification step 1: build_dawn_runner arms reproduction and post-dawn generational
    // drift, and the life-cadence reset windows the census per generation. Over several generations the
    // population grows, each race's pool effective size becomes the census-derived Ne (not the authored
    // founder size, retiring deviation 23), the run replays bit for bit, the scheduled order matches
    // the pinned order, and the whole trace is bit-identical across worker widths.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    // Build, then override the life cadence to a small value and age the founders past maturity so a
    // multi-generation run fits a test budget (the manifest cadence is one Earth year in ticks).
    let build = |seed: u64, workers: usize| {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &sexed_peoples(),
            seed,
        )
        .expect("a sex-determined dawn assembles");
        {
            let w = runner.world_mut().unwrap();
            w.set_life_cadence(4);
            w.set_workers(workers);
            let ids = w.being_ids();
            for id in ids {
                w.set_age(id, 20);
            }
        }
        runner
    };

    // Run one seed serially, tracing the state hash each tick.
    let mut runner = build(0x5EED_0A11, 1);
    let before = runner.world().unwrap().population();
    let founder_ne = runner
        .world()
        .unwrap()
        .gene_pool(RaceId(0))
        .unwrap()
        .effective_size;
    assert_eq!(founder_ne, 20, "the founder pool carries its authored size");
    let mut trace = Vec::new();
    for _ in 0..40 {
        runner.step();
        trace.push(runner.state_hash());
    }
    let w = runner.world().unwrap();
    assert!(
        w.population() > before,
        "the population grew across generations: {before} -> {}",
        w.population()
    );
    // The pool's effective size is now the census-derived Ne (set inside drift_pools before the
    // per-generation reset clears the window), not the authored founder size: deviation 23 retired for
    // the post-dawn tier. A positive Ne means the census tracked real breeders this generation.
    let drifted_ne = w.gene_pool(RaceId(0)).unwrap().effective_size;
    assert!(
        drifted_ne != 20 && drifted_ne > 0,
        "the census-derived Ne replaced the authored founder size (deviation 23): {drifted_ne}"
    );

    // Bit-for-bit replay.
    let mut replay = build(0x5EED_0A11, 1);
    let mut trace2 = Vec::new();
    for _ in 0..40 {
        replay.step();
        trace2.push(replay.state_hash());
    }
    assert_eq!(
        trace, trace2,
        "the multi-generation dawn replays bit for bit"
    );

    // The scheduled order matches the pinned order through the reproduction and drift beats.
    let mut pinned = build(0x5EED_0A11, 1);
    let mut scheduled = build(0x5EED_0A11, 1);
    for _ in 0..40 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the scheduled order stays bit-identical with reproduction and drift armed"
        );
    }

    // Worker-width invariance: the whole trace is bit-identical at widths 2, 3, 8.
    for workers in [2usize, 3, 8] {
        let mut wide = build(0x5EED_0A11, workers);
        let mut wtrace = Vec::new();
        for _ in 0..40 {
            wide.step();
            wtrace.push(wide.state_hash());
        }
        assert_eq!(
            trace, wtrace,
            "the dawn trace diverged at {workers} workers: a beat leaked the thread schedule"
        );
    }
}

/// A mobile development body plan (the thermal-coupling fixture), so a founder's walker has an anatomy
/// to derive its physiology and thermoregulate from. Labelled fixture, not owner data.
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

/// A dawn of one sex-determined race that carries a body plan, and an embodiment genesis, so the
/// world-build embodies each founder as a located body sharing its mind's id.
fn embodied_peoples() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), a_sexed_race(0).with_body_plan(mobile_body()));
    let bands = vec![BandSpec {
        race: RaceId(0),
        place: 10,
        members: 12,
    }];
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands,
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_thermal(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

/// A sexed race carrying a controller taxis gene block (base-level liveliness step 1), so its founders
/// express a thermotaxis reaction norm and MOVE along the temperature gradient. Built by appending the
/// full controller substrate to the sexed race's genes and seeding the pool spine with the taxis
/// magnitudes (move-activation bias and heading gain), mirroring the run harness. Ploidy two, so the
/// seeded weights express deterministically at the dawn; a valid Gaussian stamp lets the spine promote.
fn moving_race() -> Race {
    let mut race = a_sexed_race(0).with_body_plan(mobile_body());
    let layout = ControllerLayout::new(
        &HomeostaticRegistry::dev_thermal(),
        &AffordanceRegistry::dev_default(),
        0,
    );
    let seeds = taxis_move_weights(&layout, 0, 0, Fixed::ONE, Fixed::ONE);
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    append_controller_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        layout.weight_count(),
        &seeds,
    );
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });
    race
}

/// One band of the moving race and a matching embodiment genesis, so the founders embody and disperse.
fn dispersing_peoples() -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), moving_race());
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_thermal(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

#[test]
fn founders_with_a_taxis_block_disperse_and_a_blank_race_does_not() {
    // Base-level liveliness step 1: a founding race carrying a controller taxis gene block expresses a
    // thermotaxis reaction norm, so its founders leave their single dawn cell and disperse along the
    // temperature gradient the runner senses, through build_dawn_runner with no change to the tick. The
    // control: the same dawn with a blank-controller race (no taxis block) stays put, so the movement is
    // the seeded controller, not the mechanism (Principle 9: no authored heading).
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let run_off = |peoples: &DawnPeoples, seed: u64| -> (usize, u128) {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            peoples,
            seed,
        )
        .unwrap();
        let dawn: std::collections::BTreeSet<(i32, i32)> = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .map(|w| {
                let c = w.coord();
                (c.x, c.y)
            })
            .collect();
        assert_eq!(
            dawn.len(),
            1,
            "the band spawns its founders on one dawn cell"
        );
        for _ in 0..60 {
            runner.step();
        }
        let off = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .filter(|w| {
                let c = w.coord();
                !dawn.contains(&(c.x, c.y))
            })
            .count();
        (off, runner.state_hash())
    };

    let (moved, hash_a) = run_off(&dispersing_peoples(), 0x515E);
    assert!(
        moved > 0,
        "founders carrying a taxis block leave their dawn cell (dispersal): {moved} moved"
    );

    // The control: a blank-controller race stays on its dawn cell (no movement without a taxis block).
    let (still, _) = run_off(&embodied_peoples(), 0x515E);
    assert_eq!(
        still, 0,
        "a blank-controller race does not disperse: {still} moved (movement is the seeded controller)"
    );

    // Determinism: the dispersing run replays bit for bit.
    let (_, hash_b) = run_off(&dispersing_peoples(), 0x515E);
    assert_eq!(hash_a, hash_b, "the dispersing run replays bit for bit");
}

#[test]
fn the_dawn_runner_embodies_each_founder_as_a_mind_and_a_body() {
    // Real-world unification step 3: with an embodiment genesis, build_dawn_runner returns one runner
    // carrying both minds and bodies, and every founder is at once a cognition mind and a located,
    // thermoregulating body sharing one id. The composite replays bit for bit and the scheduled order
    // matches the pinned order with the embodiment coupled.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    let peoples = embodied_peoples();
    let runner = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        0x0B0D1,
    )
    .expect("an embodied dawn assembles");
    assert!(
        runner.embodiment().is_some(),
        "the embodied dawn returns a runner carrying bodies"
    );
    let ids = runner.world().unwrap().being_ids();
    assert_eq!(ids.len(), 12, "twelve founders seeded");
    for &id in &ids {
        assert!(
            runner.body_temp(id).is_some(),
            "founder {id:?} is at once a mind and a located body"
        );
    }
    // The birth-path evolve-channel expression is INERT on a genome that carries no exploration or
    // deliberation locus (this founder pool is unseeded): the express block reads the genome, sums over zero
    // matching loci, and leaves the walker founder-zero, so slice 1b adds no world content and every existing
    // scenario replays bit for bit. A scenario that ARMS the discovery loop (a later slice) seeds the loci, and
    // a drifted descendant is the first to express a propensity. The primed-wiring test below proves the same
    // block expresses a seeded locus onto the walker.
    for w in runner.embodiment().unwrap().walkers() {
        assert_eq!(
            w.exploration,
            Fixed::ZERO,
            "an unseeded founder expresses zero exploration (the birth-path expression is inert)"
        );
        assert_eq!(
            w.deliberation,
            Fixed::ZERO,
            "an unseeded founder expresses zero deliberation (the birth-path expression is inert)"
        );
    }

    // Tick: the composite advances (cognition world plus the body-thermal coupling), and the founders
    // remain located bodies.
    let build = |seed: u64| {
        let mut r = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &peoples,
            seed,
        )
        .unwrap();
        for _ in 0..30 {
            r.step();
        }
        r
    };
    let ran = build(0x0B0D1);
    assert_eq!(ran.clock(), 30, "the embodied runner advanced thirty ticks");
    assert_eq!(
        ran.world().unwrap().clock(),
        30,
        "the composed cognition world ticked every step"
    );
    for &id in &ids {
        assert!(
            ran.body_temp(id).is_some(),
            "founder {id:?} remains a body after ticking"
        );
    }

    // Bit-for-bit replay and seed sensitivity.
    assert_eq!(
        build(0x0B0D1).state_hash(),
        build(0x0B0D1).state_hash(),
        "the embodied dawn replays bit for bit"
    );
    assert_ne!(
        build(0x0B0D1).state_hash(),
        build(0x0B0D2).state_hash(),
        "a different seed builds a different embodied world"
    );

    // The scheduled order matches the pinned order with the embodiment coupled (the RES_BEING edge).
    let mut pinned = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        0x0B0D1,
    )
    .unwrap();
    let mut scheduled = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        0x0B0D1,
    )
    .unwrap();
    for _ in 0..20 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the scheduled order stays bit-identical with the embodiment coupled"
        );
    }
}

#[test]
fn a_founder_expresses_a_primed_evolve_channel_at_the_birth_path() {
    // Slice 1b's WIRE claim, proven with a PRIMED value. On every shipped scenario the evolve-channel loci are
    // unseeded (founder-zero), so the birth-path expression is inert and the run replays bit for bit; that
    // inertness cannot show the block actually READS the genome. Here one founder pool PRIMES the exploration
    // locus (homozygous, a positive additive effect: a drifted descendant's state stood up at the dawn) and
    // leaves deliberation unseeded, so the birth path expresses a POSITIVE exploration propensity and a ZERO
    // deliberation one onto the walker. This proves the express block reads the RIGHT channel off the genome
    // (a coded default could not tell exploration from deliberation), so a drifted locus reaches the body.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    // A mobile race carrying the taxis controller block and both evolve-channels, with EXPLORATION primed
    // (frequency one, a positive additive effect) and DELIBERATION left at the unseeded founder-zero append.
    let mut race = a_sexed_race(0).with_body_plan(mobile_body());
    let layout = ControllerLayout::new(
        &HomeostaticRegistry::dev_thermal(),
        &AffordanceRegistry::dev_default(),
        0,
    );
    let seeds = taxis_move_weights(&layout, 0, 0, Fixed::ONE, Fixed::ONE);
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    append_controller_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        layout.weight_count(),
        &seeds,
    );
    let expl = effects.len();
    append_scalar_channel(&mut genes, &mut freqs, &mut effects, Channel::Exploration);
    append_scalar_channel(&mut genes, &mut freqs, &mut effects, Channel::Deliberation);
    // Force the exploration locus homozygous with a positive additive effect, so every founder expresses it;
    // the deliberation locus keeps its founder-zero balanced frequency and zero effect.
    freqs[expl] = Fixed::ONE;
    effects[expl] = Fixed::from_ratio(1, 10);
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });

    let mut races = BTreeMap::new();
    races.insert(RaceId(0), race);
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    let peoples = DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_thermal(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    };

    let runner = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        0x0B0D1,
    )
    .expect("a primed embodied dawn assembles");
    let embodiment = runner.embodiment().unwrap();
    assert_eq!(
        embodiment.walkers().len(),
        8,
        "the primed band embodied its eight founders"
    );
    for w in embodiment.walkers() {
        assert!(
            w.exploration > Fixed::ZERO,
            "a founder of the primed pool expresses a positive exploration propensity (the birth path read the genome)"
        );
        assert_eq!(
            w.deliberation,
            Fixed::ZERO,
            "the unseeded deliberation locus stays founder-zero (the block reads the right channel, not a default)"
        );
    }
}

#[test]
fn the_unified_dawn_pairs_a_body_to_every_newborn_across_generations() {
    // Real-world unification step 3c (lifecycle pairing): with reproduction armed on the unified dawn
    // runner, a World birth mints a paired body, so a child of embodied parents is itself an embodied
    // being. Over several generations the population grows and EVERY mind stays paired to a body (a
    // body_temp entry and a walker at the same id), since the only race carries a body plan. The whole
    // trace replays bit for bit, matches the scheduled order, and is bit-identical across worker widths.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    // Build the embodied dawn, then override the life cadence small and age the founders past maturity so
    // reproduction fires within a test budget (the manifest cadence is one Earth year in ticks).
    let build = |seed: u64, workers: usize| {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &embodied_peoples(),
            seed,
        )
        .expect("an embodied sex-determined dawn assembles");
        {
            let w = runner.world_mut().unwrap();
            w.set_life_cadence(4);
            w.set_workers(workers);
            let ids = w.being_ids();
            for id in ids {
                w.set_age(id, 20);
            }
        }
        runner
    };

    // Every mind is a body and every body is a mind: the two id sets agree exactly at each tick a birth
    // could have fired. A newborn (a mind not among the founders) must gain a body_temp entry and a
    // walker sharing its id, or the paired halves have desynced.
    let founders = build(0x5EED1, 1).world().unwrap().being_ids().len();
    let mut runner = build(0x5EED1, 1);
    let mut trace = Vec::new();
    for _ in 0..40 {
        runner.step();
        trace.push(runner.state_hash());
        let minds: std::collections::BTreeSet<StableId> =
            runner.world().unwrap().being_ids().into_iter().collect();
        let walkers: std::collections::BTreeSet<StableId> = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .map(|w| w.id)
            .collect();
        assert_eq!(
            minds, walkers,
            "every mind is paired to a body and every body to a mind (no desync)"
        );
        for &id in &minds {
            assert!(
                runner.body_temp(id).is_some(),
                "being {id:?} carries a body temperature (an embodied newborn or founder)"
            );
        }
    }
    let grown = runner.world().unwrap().being_ids().len();
    assert!(
        grown > founders,
        "the embodied population grew across generations: {founders} -> {grown}"
    );

    // Bit-for-bit replay.
    let mut replay = build(0x5EED1, 1);
    let mut trace2 = Vec::new();
    for _ in 0..40 {
        replay.step();
        trace2.push(replay.state_hash());
    }
    assert_eq!(
        trace, trace2,
        "the embodied multi-generation dawn replays bit for bit"
    );

    // The scheduled order matches the pinned order through the birth-and-pairing beat.
    let mut pinned = build(0x5EED1, 1);
    let mut scheduled = build(0x5EED1, 1);
    for _ in 0..40 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the scheduled order stays bit-identical with lifecycle pairing armed"
        );
    }

    // Worker-width invariance: the whole trace is bit-identical at widths 2, 3, 8.
    for workers in [2usize, 3, 8] {
        let mut wide = build(0x5EED1, workers);
        let mut wtrace = Vec::new();
        for _ in 0..40 {
            wide.step();
            wtrace.push(wide.state_hash());
        }
        assert_eq!(
            trace, wtrace,
            "the embodied dawn trace diverged at {workers} workers: a beat leaked the thread schedule"
        );
    }
}

#[test]
fn the_unified_dawn_retires_a_body_when_its_mind_is_culled() {
    // Real-world unification step 3c (the death half): when world mortality culls a mind, the lifecycle
    // pairing retires its paired body, so no dead being's body keeps metabolizing (referential
    // integrity, design Part 58). With a rising age hazard armed, the population turns over: minds die
    // and their bodies are retired, while the two id sets stay in exact agreement every tick. The trace
    // still replays bit for bit and is bit-identical across worker widths, so the retirement is
    // deterministic and worker-count independent.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    // A rising raw-age hazard (certain survival young, certain death by age 100), armed on the embodied
    // dawn so mortality culls minds within the run.
    let mortal_peoples = || {
        let mut p = embodied_peoples();
        p.mortality_hazard = Some(Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::from_int(100), Fixed::ONE),
        ]));
        p
    };
    let build = |seed: u64, workers: usize| {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &mortal_peoples(),
            seed,
        )
        .expect("a mortal embodied dawn assembles");
        {
            let w = runner.world_mut().unwrap();
            w.set_life_cadence(4);
            w.set_workers(workers);
            let ids = w.being_ids();
            // Age the founders near the lethal end of the hazard so mortality culls some each generation.
            for id in ids {
                w.set_age(id, 70);
            }
        }
        runner
    };

    let start = build(0x0DEAD, 1).world().unwrap().being_ids().len();
    let mut runner = build(0x0DEAD, 1);
    let mut trace = Vec::new();
    let mut min_seen = start;
    for _ in 0..40 {
        runner.step();
        trace.push(runner.state_hash());
        let minds: std::collections::BTreeSet<StableId> =
            runner.world().unwrap().being_ids().into_iter().collect();
        let walkers: std::collections::BTreeSet<StableId> = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .map(|w| w.id)
            .collect();
        // The pairing invariant holds through death as it does through birth: a culled mind's body is
        // retired in the same tick, so the two sets never diverge and no dead being keeps a body
        // temperature.
        assert_eq!(
            minds, walkers,
            "a culled mind's body is retired in lockstep (no orphaned body)"
        );
        assert_eq!(
            runner.embodiment().unwrap().walkers().len(),
            minds.len(),
            "the walker count tracks the living mind count exactly"
        );
        min_seen = min_seen.min(minds.len());
    }
    assert!(
        min_seen < start,
        "the age hazard culled some minds (and retired their bodies): {start} fell to {min_seen}"
    );

    // Bit-for-bit replay and worker-width invariance through the culling.
    let mut replay = build(0x0DEAD, 1);
    let mut trace2 = Vec::new();
    for _ in 0..40 {
        replay.step();
        trace2.push(replay.state_hash());
    }
    assert_eq!(trace, trace2, "the culling dawn replays bit for bit");
    for workers in [2usize, 3, 8] {
        let mut wide = build(0x0DEAD, workers);
        let mut wtrace = Vec::new();
        for _ in 0..40 {
            wide.step();
            wtrace.push(wide.state_hash());
        }
        assert_eq!(
            trace, wtrace,
            "the culling dawn trace diverged at {workers} workers"
        );
    }
}

#[test]
fn the_scheduled_order_matches_the_pinned_order() {
    // The determinism guard: the deterministic scheduler runs the field and cognition phases in a
    // conflict-free order that is bit-identical to the hand-pinned step order, even with a full dawn
    // world composed on. Any new beat added to the composite must preserve this equality.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);
    let make = || {
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &peoples(),
            0x5CED,
        )
        .unwrap()
    };
    let mut pinned = make();
    let mut scheduled = make();
    for _ in 0..20 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the scheduled order stays bit-identical to the pinned order"
        );
    }
}

/// A survivable body with NO catalog locomotor: [`mobile_body`] with its locomotion parts removed, so the
/// CATALOG affords no movement. Its metabolism still runs (the dev thermal registry's one axis is derived,
/// unit-capacity, so an organ-less body carries its reserve), which isolates the grown run-body as the sole
/// source of any locomotion the founder shows.
fn rooted_body() -> BodyPlan {
    let mut b = mobile_body();
    b.locomotion = vec![];
    b
}

/// A morphogen race: a sexed race whose CATALOG body is rooted ([`rooted_body`]), carrying the same taxis
/// controller block the dispersing race carries (so a founder WANTS to move along the temperature gradient),
/// plus a morphogen program and a morphogen gene block. When `limbed`, the block is seeded so the founder
/// GROWS a limb (a section modulus and arm length under a bony yield, a blunt tip so it is no weapon), which
/// reads LOCOMOTE from its grown physics; when not, the block grows nothing (a single rootless segment whose
/// tiny section buckles under the propulsive load, LOCOMOTE zero). So the ONLY difference between the two
/// founders is the body their genome grows, and any dispersal is that grown limb, a locomotor the catalog
/// body never carried.
fn morphogen_race(limbed: bool) -> Race {
    let program = MorphogenProgram::dev_default();
    let mut race = a_sexed_race(0)
        .with_body_plan(rooted_body())
        .with_morphogen(program.clone());
    let layout = ControllerLayout::new(
        &HomeostaticRegistry::dev_thermal(),
        &AffordanceRegistry::dev_default(),
        0,
    );
    let seeds = taxis_move_weights(&layout, 0, 0, Fixed::ONE, Fixed::ONE);
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    append_controller_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        layout.weight_count(),
        &seeds,
    );
    // The morphogen block, appended after the controller block (its loci take the next gene indices), so a
    // founder's body grows from its own genome exactly as its controller expresses from it. A limbed founder
    // seeds a real section modulus, arm length, and bony yield with a blunt tip; a rooted one seeds nothing.
    let morph_seeds: Vec<(MorphogenParamId, Fixed)> = if limbed {
        vec![
            (MorphogenParamId(0), Fixed::ONE), // contact_area frac 1: a blunt tip, no weapon
            (MorphogenParamId(1), Fixed::from_ratio(1, 2)), // section_modulus frac
            (MorphogenParamId(2), Fixed::from_ratio(2, 5)), // arm_length frac 0.4
            (MorphogenParamId(9), Fixed::from_ratio(3, 4)), // yield_strength frac 0.75: a bony limb
        ]
    } else {
        vec![]
    };
    append_morphogen_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        program.param_count(),
        &morph_seeds,
    );
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });
    race
}

/// One band of the morphogen race and a matching embodiment genesis, so the founders embody and grow.
fn morphogen_peoples(limbed: bool) -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), morphogen_race(limbed));
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_thermal(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

#[test]
fn a_founders_genome_grows_a_locomotor_the_catalog_body_lacks() {
    // Emergent-anatomy Step 2 (B2b), the arc's thesis, blind-verified on the dawn run: a founder's body is
    // GROWN from its own genome at genesis, and the run reads the grown body. A race whose CATALOG body is
    // rooted (no locomotor) but whose GENOME grows a limb disperses from its dawn cell, because the run reads
    // the grown limb, a locomotor the catalog body never carried. The control, the same race whose genome
    // grows no limb, stays put. So the locomotion is the grown genome, never an authored catalog part
    // (Principle 8: order emerges) and never a RaceId branch (Principle 9).
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let build = |limbed: bool, seed: u64| {
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &morphogen_peoples(limbed),
            seed,
        )
        .expect("a morphogen dawn assembles")
    };

    // The grown run-body is what the walker carries: the limbed founder's structure reads LOCOMOTE from its
    // grown physics, though its catalog body carries no locomotion part.
    let fns = FunctionLawRegistry::dev_seed();
    let refs = CapabilityRefs::dev_refs();
    let caps = CapabilityCaps {
        pressure: Fixed::from_int(150_000),
        depth: Fixed::from_int(100),
    };
    let limbed = build(true, 0x11B);
    let walkers = limbed.embodiment().unwrap().walkers();
    let w = &walkers[0];
    assert!(
        w.body.locomotion.is_empty(),
        "the catalog body carries no locomotor: the run limb is the genome's, not the catalog's"
    );
    let structure = w
        .structure
        .as_ref()
        .expect("a morphogen founder carries a grown structure");
    assert!(
        structure.max_capability(FunctionLawRegistry::ID_LOCOMOTE, &fns, &refs, &caps)
            > Fixed::ZERO,
        "the grown body reads LOCOMOTE from its physics: a limb the catalog never had"
    );

    // On the run: the limbed founders disperse from their single dawn cell; the rooted ones do not.
    let run_off = |limbed: bool, seed: u64| -> (usize, u128) {
        let mut runner = build(limbed, seed);
        let dawn: std::collections::BTreeSet<(i32, i32)> = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .map(|w| {
                let c = w.coord();
                (c.x, c.y)
            })
            .collect();
        assert_eq!(
            dawn.len(),
            1,
            "the band spawns its founders on one dawn cell"
        );
        for _ in 0..60 {
            runner.step();
        }
        let off = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .filter(|w| {
                let c = w.coord();
                !dawn.contains(&(c.x, c.y))
            })
            .count();
        (off, runner.state_hash())
    };

    let (moved, hash_a) = run_off(true, 0x515E);
    assert!(
        moved > 0,
        "a founder whose genome grew a limb disperses on that grown locomotor: {moved} moved"
    );
    let (still, _) = run_off(false, 0x515E);
    assert_eq!(
        still, 0,
        "a founder whose genome grew no limb stays put (the catalog body is rooted): {still} moved"
    );

    // Determinism: the grown-limb run replays bit for bit.
    let (_, hash_b) = run_off(true, 0x515E);
    assert_eq!(hash_a, hash_b, "the grown-limb run replays bit for bit");

    // The grown structure is a pure function of the founder's (pool, seed, id): a rebuild reproduces each
    // founder's grown body bit for bit, which is what a two-tier reload relies on (the walker is regrown
    // from the re-minted genome, never serialized, so a save/load reproduces it exactly as a rebuild does).
    let a = build(true, 0x11B);
    let b = build(true, 0x11B);
    let sa = a.embodiment().unwrap().walkers();
    let sb = b.embodiment().unwrap().walkers();
    assert_eq!(sa.len(), sb.len());
    for (wa, wb) in sa.iter().zip(sb.iter()) {
        assert_eq!(wa.id, wb.id, "the founders rebuild in the same id order");
        assert_eq!(
            wa.structure, wb.structure,
            "each founder's grown body is reproduced bit for bit on a rebuild"
        );
    }

    // A separate proof that regrowth from the re-minted genome is byte-identical, the reload guarantee made
    // explicit: promote the founder's genome from the pool and regrow the structure from it, matching the
    // walker the runner grew at genesis.
    let regrown_race = morphogen_race(true);
    let id0 = sa[0].id;
    let genome = a
        .world()
        .unwrap()
        .genome_of(id0)
        .expect("the founder has a genome");
    let params = express_program(
        regrown_race.morphogen.as_ref().unwrap(),
        &regrown_race.genes,
        genome,
    );
    let regrown = grow(
        regrown_race.morphogen.as_ref().unwrap(),
        &params,
        0x11B,
        id0,
    );
    assert_eq!(
        sa[0].structure.as_ref(),
        Some(&regrown),
        "regrowing from the re-minted genome reproduces the founder's grown body exactly"
    );

    // Worker invariance: the grown-limb dawn matches the scheduled order with the embodiment coupled.
    let mut pinned = build(true, 0x515E);
    let mut scheduled = build(true, 0x515E);
    for _ in 0..20 {
        pinned.step();
        scheduled.step_scheduled(&[]);
        assert_eq!(
            pinned.state_hash(),
            scheduled.state_hash(),
            "the grown-body dawn stays bit-identical under the scheduler"
        );
    }
}

/// A homeostatic registry for the viability cull: the thermal axis PLUS a derived integrity axis (emergent-
/// anatomy Step 3). The integrity axis does not self-drain; its level is set each tick from the grown body's
/// whole-body functional viability, so a body reading no viable function falls through the death floor. The
/// death floor is the reserved minimum whole-body capability a grown body needs to be viable. Labelled dev
/// fixture, RESERVED-with-basis (the minimum functional capability below which a body sustains no life
/// function, a small positive fraction), never owner data.
fn integrity_thermal_registry() -> HomeostaticRegistry {
    let mut reg = HomeostaticRegistry::dev_thermal();
    reg.axes.push(HomeostaticAxisDef {
        id: INTEGRITY,
        name: "integrity".to_string(),
        backing_component: None,
        capacity_per_mass: Fixed::ONE,
        base_drain: Fixed::ZERO,
        exertion_drain: Fixed::ZERO,
        death_floor: Fixed::from_ratio(1, 32),
        draw_set: Vec::new(),
    });
    reg
}

/// A morphogen race for the viability cull with NO controller block (a blank reaction norm, so the founders
/// sit still and the test isolates the cull from movement): the sexed race, a catalog body for the LOD-0
/// metabolic tier, a morphogen program, and a morphogen block seeded to grow a viable LIMB (`limbed`) or
/// nothing (an inert all-zero body). The grown body is what the run reads; a limbed founder reads a viable
/// function, an inert one none.
fn viability_race(limbed: bool) -> Race {
    let program = MorphogenProgram::dev_default();
    let mut race = a_sexed_race(0)
        .with_body_plan(mobile_body())
        .with_morphogen(program.clone());
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    let morph_seeds: Vec<(MorphogenParamId, Fixed)> = if limbed {
        vec![
            (MorphogenParamId(0), Fixed::ONE),
            (MorphogenParamId(1), Fixed::from_ratio(1, 2)),
            (MorphogenParamId(2), Fixed::from_ratio(2, 5)),
            (MorphogenParamId(9), Fixed::from_ratio(3, 4)),
        ]
    } else {
        vec![]
    };
    append_morphogen_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        program.param_count(),
        &morph_seeds,
    );
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });
    race
}

/// One band of the viability race and an embodiment genesis carrying the integrity-bearing registry.
fn viability_peoples(limbed: bool) -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), viability_race(limbed));
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: integrity_thermal_registry(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

#[test]
fn an_incoherent_grown_body_is_culled_through_the_ordinary_reserve_floor_path() {
    // Emergent-anatomy Step 3 (the viability cull): selection prunes incoherent grown bodies through the
    // EXISTING reserve-floor death path, never a new seed-time gate. A race whose genome grows a viable LIMB
    // survives; the SAME race (same id, same registry, same everything else) whose genome grows an INERT body
    // (all-zero morphogen, a degenerate structure reading no viable function) has its derived integrity
    // reserve set to that zero viability each tick, falls through the reserved viability floor, and dies
    // through the same is_alive -> reconcile_lifecycle cull as any other death, with no code that inspects
    // morphology to reject it (Principle 8). The opposite verdicts for one race id prove the cull reads the
    // grown physics, never a RaceId or label (Principle 9).
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let run = |limbed: bool, seed: u64| -> (usize, u128) {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &viability_peoples(limbed),
            seed,
        )
        .expect("a morphogen dawn assembles");
        assert_eq!(
            runner.world().unwrap().being_ids().len(),
            8,
            "the band seeds its founders as living bodies"
        );
        for _ in 0..20 {
            runner.step();
        }
        let alive = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .filter(|w| w.alive)
            .count();
        (alive, runner.state_hash())
    };

    let (coherent_alive, hash_a) = run(true, 0x5111);
    assert!(
        coherent_alive > 0,
        "a founder whose genome grew a viable limb reads a viable function and survives: {coherent_alive} alive"
    );

    let (inert_alive, _) = run(false, 0x5111);
    assert_eq!(
        inert_alive, 0,
        "a founder whose genome grew an inert body reads no viable function and is culled: {inert_alive} alive"
    );

    // Determinism: the viability-cull run replays bit for bit.
    let (_, hash_b) = run(true, 0x5111);
    assert_eq!(hash_a, hash_b, "the viability-cull run replays bit for bit");
}

// --- R-AGING (c) FUNCTIONAL CHECK: aging and death EMERGE from the per-segment first-passage accumulator ---

/// A homeostatic registry for the aging functional check: the grazer metabolic axes (ENERGY backed by
/// `bio.energy_density`, WATER, TEMPERATURE, CONDITION) PLUS the derived INTEGRITY axis. The aging accrual
/// keys its repair funding on the `bio.energy_density`-backed axis, so the grazer registry (not the
/// temperature-only one) is required. INTEGRITY is set each tick from the aged whole-body viability, so a
/// body worn down by accumulated damage falls through the death floor. RESERVED-with-basis death floor
/// (the minimum viable whole-body capability), a labelled dev fixture.
fn aging_homeo() -> HomeostaticRegistry {
    let mut reg = HomeostaticRegistry::dev_grazer();
    reg.axes.push(HomeostaticAxisDef {
        id: INTEGRITY,
        name: "integrity".to_string(),
        backing_component: None,
        capacity_per_mass: Fixed::ONE,
        base_drain: Fixed::ZERO,
        exertion_drain: Fixed::ZERO,
        death_floor: Fixed::from_ratio(1, 8),
        draw_set: Vec::new(),
    });
    reg
}

/// A morphogen race that ARMS aging: the viability-cull race's limbed grown body (so its segments read a
/// LOCOMOTE capability and slide a positive distance each tick, the wear insult's drive), extended so its
/// grown tissue carries the tribology axes the Archard wear insult reads. The three added axes are
/// DEV-FIXTURE test constants (non-canonical, never owner data): they are grown with `lo == hi` so every
/// segment carries them deterministically without seeding new genome loci, and `mat.turnover_rate` is
/// intentionally omitted so repair is zero and net damage is pure wear (fast, monotone aging for the
/// check). The `mat.wear_coefficient` is on the axis's `x1e6` storage scale (matching
/// `PhysicsRegistry::ground()`). The only per-race difference is `fracture_energy`, which sets each
/// segment's failure tolerance (`fracture_energy * contact_area`), so a tougher race lives longer purely
/// as an output of the tolerance-versus-wear balance.
fn aging_race(fracture_energy: Fixed) -> Race {
    let mut program = MorphogenProgram::dev_default();
    let fixed = |axis: &str, v: Fixed| AxisSpec {
        axis: axis.to_string(),
        lo: v,
        hi: v,
    };
    program
        .material_axes
        .push(fixed("mat.fracture_energy", fracture_energy));
    // A soft tissue (low indentation hardness) worn by an aggressive tribological pair, so the Archard wear
    // is fast enough that first-passage aging kills within the test window before the metabolic reserves
    // (seeded full below) run down. Pinning hardness overrides the dev_default range so the wear rate is
    // deterministic rather than genome-expressed. All dev-fixture, non-canonical test constants.
    program
        .material_axes
        .push(fixed("mat.indentation_hardness", Fixed::ONE));
    program
        .material_axes
        .push(fixed("mat.wear_coefficient", Fixed::from_int(30)));
    program
        .material_axes
        .push(fixed("mat.specific_cut_energy", Fixed::ONE));

    // A FULLY GROWN race (no catalog body): its metabolic reserves source from its grown tissue (the B2b
    // metabolic-tier grow), so the energy-density and water-fraction seeded below back real energy and
    // water reserves off the grown structure, and the being does not start empty and instantly starve.
    let mut race = a_sexed_race(0).with_morphogen(program.clone());
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    // The same limb seed the viability race grows (a section modulus, arm length, and bony yield with a
    // blunt tip), so the founder reads LOCOMOTE and its wear drive (force and slide distance) is positive;
    // PLUS a full metabolic tissue (energy density and water fraction) so the founders carry ample energy and
    // water reserves and do not starve or dehydrate before aging kills them (isolating the first-passage
    // cause). The composition axes (convective_surface, fracture_strength, specific_heat, energy_density,
    // water_fraction) are addressed through the program's own `composition_param` accessor, so this fixture
    // stays correct as more param categories are appended (the stroke-rate actuator block now follows
    // composition; a `param_count() - 2` / `- 1` address would point at it, not at energy/water). Composition
    // index 1 IS physiology::MUSCLE_STRENGTH (fracture_strength), so seeding it gives the being a positive
    // muscle force (the Archard wear drive; a zero-muscle body exerts no load and never wears); the last two
    // composition axes are energy density and water fraction.
    let comp = program.composition_axes.len();
    let muscle = MorphogenParamId(program.composition_param(1) as u32); // fracture_strength
    let energy_density = MorphogenParamId(program.composition_param(comp - 2) as u32);
    let water_fraction = MorphogenParamId(program.composition_param(comp - 1) as u32);
    let morph_seeds: Vec<(MorphogenParamId, Fixed)> = vec![
        (MorphogenParamId(0), Fixed::ONE),
        (MorphogenParamId(1), Fixed::from_ratio(1, 2)),
        (MorphogenParamId(2), Fixed::from_ratio(2, 5)),
        (MorphogenParamId(9), Fixed::from_ratio(3, 4)),
        (muscle, Fixed::ONE),
        (energy_density, Fixed::ONE),
        (water_fraction, Fixed::ONE),
    ];
    append_morphogen_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        program.param_count(),
        &morph_seeds,
    );
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });
    race
}

/// One band of the aging race and an embodiment genesis carrying the aging-bearing registry.
fn aging_peoples(fracture_energy: Fixed) -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), aging_race(fracture_energy));
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: aging_homeo(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

#[test]
fn aging_kills_by_first_passage_and_a_tougher_body_lives_longer() {
    // R-AGING (c) FUNCTIONAL CHECK, end to end in the run loop: a grown-body race that arms aging dies
    // from FIRST-PASSAGE per-part damage (its segments accrue Archard wear against their own tolerance,
    // the aged whole-body viability falls, and the being is culled through the EXISTING reserve-floor
    // path, INTEGRITY, with no vital-part predicate), and the size-longevity relation is a pure OUTPUT: a
    // tougher race (higher grown fracture energy, so a higher per-segment failure tolerance) lives longer,
    // never a written size-to-duration law. The tribology values are dev fixtures; the real per-tissue
    // wear and turnover stay reserved-with-basis.
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    // Run a race to `ticks` and return, per tick, the (integrity_deaths, other_deaths) counts, plus the
    // final state hash for the determinism check.
    let run = |fracture_energy: Fixed, seed: u64, ticks: u32| -> (Vec<(u32, u32)>, u128) {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &aging_peoples(fracture_energy),
            seed,
        )
        .expect("an aging dawn assembles");
        // Arm the aging runtime the harness leaves unset (physiology is already installed): the material
        // registry (for the mat.wear_coefficient storage scale) and the wear caps.
        let emb = runner.embodiment_mut().unwrap();
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        emb.set_wear(civsim_foundation::material::WearParams::dev_fixture());

        let mut timeline = Vec::new();
        for _ in 0..ticks {
            runner.step();
            let deaths = runner.take_obs_deaths();
            let integ = deaths.iter().filter(|a| **a == INTEGRITY).count() as u32;
            let other = deaths.len() as u32 - integ;
            if integ > 0 || other > 0 {
                timeline.push((integ, other));
            }
        }
        (timeline, runner.state_hash())
    };

    let frail_fe = Fixed::from_int(1);
    let tough_fe = Fixed::from_int(8);
    let (frail, frail_hash) = run(frail_fe, 0x5111, 40);
    let (tough, _) = run(tough_fe, 0x5111, 40);

    let frail_integ: u32 = frail.iter().map(|(i, _)| i).sum();
    let frail_other: u32 = frail.iter().map(|(_, o)| o).sum();
    let tough_integ: u32 = tough.iter().map(|(i, _)| i).sum();

    // (1) Aging kills: the frail race suffers first-passage INTEGRITY deaths.
    assert!(
        frail_integ > 0,
        "a loaded, aging-armed body dies of first-passage per-part damage (INTEGRITY): frail timeline {frail:?}"
    );
    // (2) The death is aging, not a metabolic confound: the INTEGRITY deaths dominate the frail run.
    assert!(
        frail_integ >= frail_other,
        "the deaths are aging (INTEGRITY), not starvation/exposure: {frail_integ} integrity vs {frail_other} other"
    );
    // (3) The longevity relation is an emergent OUTPUT: the tougher race (higher failure tolerance) suffers
    // strictly fewer aging deaths over the same window, purely from the tolerance-versus-wear balance.
    assert!(
        tough_integ < frail_integ,
        "a tougher body lives longer, an emergent output never an authored law: tough {tough_integ} vs frail {frail_integ} integrity deaths"
    );

    // Determinism: the aging run replays bit for bit.
    let (_, frail_hash_b) = run(frail_fe, 0x5111, 40);
    assert_eq!(
        frail_hash, frail_hash_b,
        "the aging run replays bit for bit"
    );
}

/// A FULLY GROWN race (emergent-anatomy Step 3, the metabolic-tier grow): a developmental program and NO
/// catalog body, so it founds embodied members whose metabolism is sourced from their grown tissue. The
/// morphogen block grows a limb (a coherent body) always, plus energy and water tissue when `nourished`, so
/// the ONLY difference between the two variants is whether the grown tissue backs a metabolic reserve. No
/// controller block: a blank reaction norm, so the founders sit still and the test isolates metabolic survival.
fn fully_grown_race(nourished: bool, physiological: bool) -> Race {
    let program = MorphogenProgram::dev_default();
    // NO with_body_plan: race.body stays None, the fully grown case.
    let mut race = a_sexed_race(0).with_morphogen(program.clone());
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    // A limb (a coherent body); the composition params sit after spawn, the physiology axes (convective
    // surface, muscle strength, specific heat) leading the energy density and water fraction the reserves
    // back. `nourished` seeds the reserves; `physiological` additionally seeds the derived-physiology tissue,
    // so a body grows an exchange surface, muscle, and thermal mass the run reads. Addressed through the
    // program's own `composition_param` accessor (not `param_count()`-relative, which the appended actuator
    // block would shift), so the fixture stays correct as more param categories are added.
    let comp = program.composition_axes.len();
    let surface = program.composition_param(comp - 5);
    let muscle = program.composition_param(comp - 4);
    let specific_heat = program.composition_param(comp - 3);
    let energy = program.composition_param(comp - 2);
    let water = program.composition_param(comp - 1);
    let mut morph_seeds: Vec<(MorphogenParamId, Fixed)> = vec![
        (MorphogenParamId(0), Fixed::ONE),
        (MorphogenParamId(1), Fixed::from_ratio(1, 2)),
        (MorphogenParamId(2), Fixed::from_ratio(2, 5)),
        (MorphogenParamId(9), Fixed::from_ratio(3, 4)),
    ];
    if nourished {
        morph_seeds.push((MorphogenParamId(energy as u32), Fixed::from_ratio(1, 2)));
        morph_seeds.push((MorphogenParamId(water as u32), Fixed::from_ratio(1, 2)));
    }
    if physiological {
        morph_seeds.push((MorphogenParamId(surface as u32), Fixed::from_ratio(1, 2)));
        morph_seeds.push((MorphogenParamId(muscle as u32), Fixed::from_ratio(1, 2)));
        morph_seeds.push((
            MorphogenParamId(specific_heat as u32),
            Fixed::from_ratio(1, 2),
        ));
    }
    append_morphogen_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        program.param_count(),
        &morph_seeds,
    );
    race.genes = GeneSet { genes };
    race.pool = GenePool::new(SchemeId(0), 20, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });
    race
}

/// One band of the fully grown race and an embodiment genesis over a metabolizing registry (energy and
/// water, both backed by grown tissue, plus temperature and condition), so the reserve sourced from grown
/// tissue is exercised and an energy-less body starves.
fn fully_grown_peoples(nourished: bool, physiological: bool) -> DawnPeoples {
    let mut races = BTreeMap::new();
    races.insert(RaceId(0), fully_grown_race(nourished, physiological));
    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));
    DawnPeoples {
        races,
        bands: vec![BandSpec {
            race: RaceId(0),
            place: 10,
            members: 8,
        }],
        breeding,
        personality: PersonalityRegistry::new(),
        mortality_hazard: None,
        language: None,
        embodiment: Some(EmbodimentGenesis {
            homeostatic: HomeostaticRegistry::dev_grazer(),
            affordances: AffordanceRegistry::dev_default(),
            locomotion: LocomotionParams::dev_default(),
            organs: BodyPlanRegistry::dev_default(),
            tolerances: Default::default(),
            controller_hidden: 0,
            resource_features: civsim_sim::perceivable_feature::PerceivableFeatureRegistry::empty(),
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
        biosphere: None,
    }
}

#[test]
fn a_fully_grown_race_founds_bodies_with_no_catalog_body_and_starves_without_grown_tissue() {
    // Emergent-anatomy Step 3, the metabolic-tier grow: a race that declares a developmental program and NO
    // catalog body founds embodied members whose metabolic reserves are sourced from their GROWN tissue, so
    // the catalog metabolic body is retired and a grown race needs no catalog body. A nourished grown founder
    // (its tissue carries energy and water) survives on its grown reserves; the SAME race whose tissue carries
    // none is born with no reserve and starves through the ordinary reserve-floor cull, the metabolic
    // viability read from grown physics, never a catalog body (Principle 8) and never a RaceId (Principle 9).
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let run = |nourished: bool, seed: u64| -> (usize, u128) {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &fully_grown_peoples(nourished, false),
            seed,
        )
        .expect("a fully grown dawn assembles");
        // The founders embody though their race declares NO catalog body, on their grown metabolism.
        assert_eq!(
            runner.world().unwrap().being_ids().len(),
            8,
            "the band seeds its founders"
        );
        let emb = runner.embodiment().unwrap();
        assert!(
            emb.walkers().iter().all(|w| w.structure.is_some()),
            "every founder carries a grown body"
        );
        assert!(
            emb.walkers().iter().all(|w| w.body.organs.is_empty()),
            "and no catalog organs: the metabolic body is grown, not catalog"
        );
        for _ in 0..20 {
            runner.step();
        }
        let alive = runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .filter(|w| w.alive)
            .count();
        (alive, runner.state_hash())
    };

    let (nourished_alive, hash_a) = run(true, 0x11B);
    assert!(
        nourished_alive > 0,
        "a fully grown founder whose tissue carries energy and water survives on its grown reserves: {nourished_alive} alive"
    );

    let (starved_alive, _) = run(false, 0x11B);
    assert_eq!(
        starved_alive, 0,
        "a fully grown founder whose tissue carries no energy is born with no reserve and starves: {starved_alive} alive"
    );

    // Determinism: the fully grown metabolic run replays bit for bit.
    let (_, hash_b) = run(true, 0x11B);
    assert_eq!(
        hash_a, hash_b,
        "the fully grown metabolic run replays bit for bit"
    );
}

#[test]
fn a_fully_grown_bodys_derived_physiology_is_read_from_its_grown_tissue() {
    // Emergent-anatomy Step 3, the derived-physiology grow: a fully grown body reads its exchange surface,
    // muscle strength, and thermal mass off its OWN grown tissue, not the empty digest's safe zeros. Proven
    // two ways. (1) Structurally at genesis: a physiology-seeded fully grown founder's grown structure reads
    // a positive convective surface, muscle strength, and specific heat directly (composition_sum / _mean),
    // with no catalog organs. (2) On the run: that grown exchange surface is LOAD-BEARING, a physiological
    // body pays the thermoregulatory drain its surface incurs against the medium and does not outlast a
    // 20-tick run, while the same fully grown race with the reserves but NO grown surface survives. The
    // grown surface changing the run outcome proves the run's derived drain reads the grown tissue, never a
    // catalog organ (Principle 8), never a RaceId (Principle 9, same race id, opposite outcomes).
    let manifest = manifest();
    let resolution = a_scenario(None).resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let build = |physiological: bool| {
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &fully_grown_peoples(true, physiological),
            0x11B,
        )
        .expect("a fully grown dawn assembles")
    };

    // (1) The grown structure reads its physiology composition directly, with no catalog organs.
    let physiological = build(true);
    let founder = &physiological.embodiment().unwrap().walkers()[0];
    assert!(
        founder.body.organs.is_empty(),
        "no catalog organs: the physiology is grown, not catalog"
    );
    let s = founder
        .structure
        .as_ref()
        .expect("a fully grown founder carries a grown structure");
    assert!(
        s.composition_sum("bio.convective_surface") > Fixed::ZERO,
        "a grown body reads its exchange surface off its tissue (not thermally inert)"
    );
    assert!(
        s.composition_sum("mat.fracture_strength") > Fixed::ZERO,
        "and its muscle strength (not exertion-free)"
    );
    assert!(
        s.composition_mean("therm.specific_heat") > Fixed::ZERO,
        "and its thermal mass"
    );

    // (2) The grown surface is load-bearing on the run: it drives a thermoregulatory drain the surfaceless
    // body does not pay.
    let survivors = |physiological: bool| -> usize {
        let mut runner = build(physiological);
        for _ in 0..20 {
            runner.step();
        }
        runner
            .embodiment()
            .unwrap()
            .walkers()
            .iter()
            .filter(|w| w.alive)
            .count()
    };
    assert!(
        survivors(false) > 0,
        "a fully grown body with the reserves but no grown exchange surface pays no thermoregulatory drain and survives"
    );
    assert_eq!(
        survivors(true),
        0,
        "the same race whose genome grows an exchange surface pays the thermoregulatory drain the run reads off that grown tissue, and does not outlast the run"
    );
}

#[test]
fn arming_being_percept_wires_the_being_block_changes_the_run_and_replays() {
    // The being-percept keystone, step 6 (the live wire): a scenario that declares `being_percept` arms the
    // being-directed perception on the embodiment fail-loud from the manifest, so a being perceives other
    // beings and can steer toward or away from them through its founder-zero freely-signed weight. This proves
    // three things: the feature ARMS the controller being block (the layout carries it, off does not), it is
    // NOT a no-op (an armed run's state diverges from an unarmed one), and it REPLAYS bit-for-bit (determinism,
    // Principle 3). The 12-member co-located band means the founders emit and perceive each other's thermal
    // signal, so the perceive and learn wire is exercised end to end, not merely present.
    let manifest = manifest();
    let map = a_map(0xB0);

    let on = Scenario::from_toml_str(
        "[scenario]\nid = \"w\"\nname = \"W\"\n[features]\nbeing_percept = true\n",
    )
    .unwrap()
    .resolve(&manifest)
    .unwrap();
    let off = a_scenario(None).resolve(&manifest).unwrap();

    let build = |res: &ScenarioResolution| {
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            res,
            &map,
            &embodied_peoples(),
            0x2E,
        )
        .expect("the embodied dawn assembles")
    };

    // The feature ARMS the controller being block when declared, and leaves it absent when not (the opt-in
    // that keeps a feature-off world byte-identical to the pre-keystone build).
    let runner_on = build(&on);
    let runner_off = build(&off);
    assert!(
        runner_on
            .embodiment_layout()
            .expect("the armed embodiment carries a layout")
            .n_being()
            > 0,
        "declaring being_percept arms the controller being block"
    );
    assert_eq!(
        runner_off
            .embodiment_layout()
            .expect("the embodiment carries a layout")
            .n_being(),
        0,
        "a world that does not declare being_percept carries no being block"
    );

    // The armed run is NOT a no-op: its state after a run of ticks diverges from the unarmed run (the being
    // block widens the controller and the perceive/learn wire fires on the co-located band).
    let hash_after = |res: &ScenarioResolution, ticks: usize| -> u128 {
        let mut r = build(res);
        for _ in 0..ticks {
            r.step();
        }
        r.state_hash()
    };
    const TICKS: usize = 30;
    let armed = hash_after(&on, TICKS);
    let unarmed = hash_after(&off, TICKS);
    assert_ne!(
        armed, unarmed,
        "arming being-percept changes the run (the wire is live, not inert)"
    );

    // Determinism: the armed run replays bit-for-bit from one seed (the perceive and learn wire draws no RNG
    // and folds deterministically).
    assert_eq!(
        hash_after(&on, TICKS),
        armed,
        "the armed being-percept run replays bit-for-bit from one seed"
    );
}

#[test]
fn build_dawn_runner_arms_only_the_learners_the_scenario_declares() {
    // The loader arc, gap b: the world-build path arms each opt-in learner fail-loud from the manifest,
    // GATED by the scenario [features] block, so which learners a world runs is scenario DATA. A scenario
    // that declares a feature arms its learner; one that declares none arms nothing, which is what keeps a
    // feature-off world byte-identical to the pre-loader build.
    let manifest = manifest();
    let map = a_map(0xB0);

    // Declares all three calibration learners: the build succeeds (the dev fixtures carry their values) and
    // each learner is armed.
    let on = Scenario::from_toml_str(
        "[scenario]\nid = \"w\"\nname = \"W\"\n\
         [features]\nreward = true\ndiscovery = true\nconvictions = true\n",
    )
    .unwrap()
    .resolve(&manifest)
    .unwrap();
    let runner_on = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &on,
        &map,
        &peoples(),
        0x2E,
    )
    .expect("a features-declaring dawn assembles under Development");
    assert!(
        runner_on.reward_learning().is_some(),
        "the reward learner is armed when declared"
    );
    assert!(
        runner_on.discovery().is_some(),
        "the discovery sampler is armed when declared"
    );
    assert!(
        runner_on.felt_conviction_learning().is_some(),
        "the felt-conviction learner is armed when declared"
    );

    // Declares no features: nothing is armed on the canonical path (the gating that keeps a feature-off
    // world byte-identical).
    let off = a_scenario(None).resolve(&manifest).unwrap();
    let runner_off = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &off,
        &map,
        &peoples(),
        0x2E,
    )
    .unwrap();
    assert!(
        runner_off.reward_learning().is_none(),
        "the reward learner is NOT armed when the scenario declares no feature"
    );
    assert!(
        runner_off.discovery().is_none(),
        "the discovery sampler is NOT armed when undeclared"
    );
    assert!(
        runner_off.felt_conviction_learning().is_none(),
        "the felt-conviction learner is NOT armed when undeclared"
    );

    // Each learner is gated INDEPENDENTLY: declaring reward alone arms only the reward learner, leaving
    // discovery and convictions unarmed (proving the three `if` gates are separate, not coupled).
    let reward_only = Scenario::from_toml_str(
        "[scenario]\nid = \"w\"\nname = \"W\"\n[features]\nreward = true\n",
    )
    .unwrap()
    .resolve(&manifest)
    .unwrap();
    let runner_reward = build_dawn_runner(
        &manifest,
        &channels(),
        Profile::Development,
        &reward_only,
        &map,
        &peoples(),
        0x2E,
    )
    .unwrap();
    assert!(
        runner_reward.reward_learning().is_some(),
        "reward alone arms the reward learner"
    );
    assert!(
        runner_reward.discovery().is_none() && runner_reward.felt_conviction_learning().is_none(),
        "reward alone leaves discovery and convictions unarmed (independent gating)"
    );
}

#[test]
fn build_dawn_runner_propagates_a_reserved_feature_calib_fail_loud() {
    // The feature-block fail-loud, end to end (Arc 1 gap b): arming a feature whose OWN calibration value is
    // reserved refuses at the feature arming itself, not at an earlier gate. The manifest is the dev fixtures
    // with just `felt_conviction.retention` blanked (reserved), so every pre-feature read still succeeds and
    // the build reaches the convictions arming, where FeltConvictionCalib::from_manifest fails loud. The SAME
    // manifest builds fine when the scenario declares no features (the reserved value is never read), which is
    // the gating that keeps a feature-off world building.
    let fixtures = std::fs::read_to_string(FIXTURES).unwrap();
    // `0.9375` is felt_conviction.retention's unique value in the dev fixtures; blanking it reserves the key.
    let retention_reserved = fixtures.replace("value = \"0.9375\"", "value = \"\"");
    assert_ne!(
        retention_reserved, fixtures,
        "the retention value must exist to blank"
    );
    let manifest = CalibrationManifest::from_toml_str(&retention_reserved).unwrap();
    let map = a_map(0xB0);

    // convictions declared: the build reaches FeltConvictionCalib::from_manifest and refuses on the reserved
    // retention (never fabricating a default).
    let armed = Scenario::from_toml_str(
        "[scenario]\nid = \"w\"\nname = \"W\"\n[features]\nconvictions = true\n",
    )
    .unwrap()
    .resolve(&manifest)
    .unwrap();
    assert!(
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &armed,
            &map,
            &peoples(),
            0x2E,
        )
        .is_err(),
        "arming convictions with a reserved felt_conviction.retention fails loud at the feature block"
    );

    // convictions NOT declared: the reserved retention is never read, so the same manifest builds.
    let unarmed = a_scenario(None).resolve(&manifest).unwrap();
    assert!(
        build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &unarmed,
            &map,
            &peoples(),
            0x2E,
        )
        .is_ok(),
        "a feature-off world never reads the reserved feature value, so it builds"
    );
}

#[test]
fn a_scenario_refuses_to_build_under_calibrated_while_values_are_reserved() {
    // The handoff's Arc-1 proof step: a [features]-declaring scenario built under Profile::Calibrated against
    // the real reserved manifest REFUSES while any required value it reads is still reserved (never fabricating
    // one), which is exactly what maps the remaining calibration work to Arc 2. HONEST SCOPE (do not overclaim):
    // the refusal here fires at the FIRST gate, World::from_manifest's base required set (evidence/gossip/orbit/
    // tick), which precedes the feature arming, so this test proves the loader refuses under Calibrated but NOT
    // that the feature block itself is fail-loud; that end-to-end claim is proven separately by
    // `build_dawn_runner_propagates_a_reserved_feature_calib_fail_loud` (which reaches the feature arming) and
    // by the per-calib unit tests. The same scenario builds under Development, where the dev fixtures supply
    // labelled placeholders, so the refusal is the calibration gate, not a structural failure.
    let reserved = CalibrationManifest::load(RESERVED).expect("the reserved manifest loads");
    let features_on = Scenario::from_toml_str(
        "[scenario]\nid = \"w\"\nname = \"W\"\n\
         [features]\nreward = true\ndiscovery = true\nconvictions = true\n",
    )
    .unwrap();
    let map = a_map(0xB0);
    let refused = build_dawn_runner(
        &reserved,
        &channels(),
        Profile::Calibrated,
        &features_on.resolve(&reserved).unwrap(),
        &map,
        &peoples(),
        0x2E,
    );
    assert!(
        refused.is_err(),
        "Profile::Calibrated refuses to build while required values are reserved (the fail-loud gate that maps Arc 2)"
    );

    // The identical features-declaring scenario builds under Development against the dev fixtures.
    let dev = manifest();
    assert!(
        build_dawn_runner(
            &dev,
            &channels(),
            Profile::Development,
            &features_on.resolve(&dev).unwrap(),
            &map,
            &peoples(),
            0x2E,
        )
        .is_ok(),
        "the same scenario builds under Development, so the Calibrated refusal is the calibration gate"
    );
}
