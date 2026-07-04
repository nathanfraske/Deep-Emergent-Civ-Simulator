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

use civsim_core::{Fixed, GaussApprox};
use civsim_sim::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::homeostasis::{AffordanceRegistry, HomeostaticRegistry};
use civsim_sim::langmod::PerceptualParams;
use civsim_sim::language::{ConceptId, FeatureDimId, ProductionModalityId};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::scenario::Scenario;
use civsim_sim::sensorium::SenseChannelId;
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    build_dawn_runner, Articulation, Axiom, AxiomAxisId, BandSpec, BreedingSystem,
    BreedingSystemId, BreedingSystemRegistry, Channel, CognitionChannel, Curve, DawnPeoples,
    DominanceKind, DominanceMode, EmbodimentGenesis, EpistemicStance, EvidenceRing, GeneDef,
    GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs, LanguageGenesis,
    PersonalityProfile, PersonalityRegistry, Race, RaceId, ReproductionMode, SchemeId,
    SourceModeId, TraitAxisId, TraitDef, ValueAxisId, ValueProfile,
};
use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../calibration/profiles/dev-fixtures.toml"
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
            controller_hidden: 0,
            submerged_medium_id: "medium.water".to_string(),
            emergent_medium_id: "medium.air".to_string(),
        }),
    }
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
