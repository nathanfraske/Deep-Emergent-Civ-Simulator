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
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::scenario::Scenario;
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    build_dawn_runner, Axiom, AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId,
    BreedingSystemRegistry, Channel, CognitionChannel, Curve, DawnPeoples, DominanceMode,
    EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme,
    IntrinsicBeliefs, PersonalityProfile, PersonalityRegistry, Race, RaceId, ReproductionMode,
    SchemeId, SourceModeId, TraitAxisId, TraitDef, ValueAxisId, ValueProfile,
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
