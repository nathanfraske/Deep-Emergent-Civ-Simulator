//! R-AGING (c) WATCHABLE DEMO (quarantined dev harness, not canonical): a grown-body race that arms aging
//! run through the real world-backed loop ([`build_dawn_runner`]), printing the per-tick fall of a founder's
//! INTEGRITY as its grown tissue wears, and the first-passage deaths when the aged whole-body viability drops
//! through the reserve-floor cull. It then contrasts a frail race with a race whose grown tissue carries 8x
//! the fracture energy, so the watcher sees the size-longevity relation emerge as a pure OUTPUT of the
//! tolerance-versus-wear balance: the tougher body ages slower and outlives the frail one, with no authored
//! size-to-duration law. This mirrors the integration test `aging_kills_by_first_passage_and_a_tougher_body_
//! lives_longer`; it is a new example, so it is byte-neutral by construction (it touches no shipped path).
//!
//! Run: `cargo run --example aging_demo -p civsim-sim`. The tribology values are DEV FIXTURES (non-canonical,
//! never owner data); the real per-tissue wear and turnover stay reserved-with-basis.

use std::collections::BTreeMap;

use civsim_core::{Fixed, GaussApprox};
use civsim_sim::anatomy::BodyPlanRegistry;
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::homeostasis::{
    AffordanceRegistry, HomeostaticAxisDef, HomeostaticRegistry, INTEGRITY,
};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::material::WearParams;
use civsim_sim::scenario::Scenario;
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    append_morphogen_block, build_dawn_runner, Axiom, AxiomAxisId, AxisSpec, BandSpec,
    BreedingSystem, BreedingSystemId, BreedingSystemRegistry, Channel, CognitionChannel,
    DawnPeoples, DominanceKind, DominanceMode, EmbodimentGenesis, EpistemicStance, EvidenceRing,
    GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs,
    MorphogenParamId, MorphogenProgram, PersonalityRegistry, Race, RaceId, ReproductionMode,
    SchemeId, SourceModeId, ValueAxisId, ValueProfile,
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

fn a_map(seed: u64) -> TileMap {
    let topo = FlatBounded::new(16, 12, 1);
    let biomes = BiomeSet::dev_default();
    TileMap::generate(seed, topo, &biomes, &WorldgenParams::dev_default())
}

fn a_scenario() -> Scenario {
    Scenario::from_toml_str("[scenario]\nid = \"w\"\nname = \"W\"\n").unwrap()
}

/// A sexed race (two cognition genes and a sex-determination gene), a three-locus biallelic pool, an innate
/// disposition, and a binary breeding system. The base a grown-body race is assembled from.
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

/// The aging homeostatic registry: the grazer metabolic axes (ENERGY backed by `bio.energy_density`, the
/// repair-funding key) plus the derived INTEGRITY axis the aged viability sets each tick.
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

/// A fully grown race that arms aging: its grown tissue carries the tribology axes the Archard wear insult
/// reads, plus a full metabolic tissue so it does not starve before aging kills it, plus muscle so it exerts
/// a wear load. The only per-race difference is `fracture_energy` (the per-segment failure tolerance). All
/// tribology values are dev fixtures, non-canonical.
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
    program
        .material_axes
        .push(fixed("mat.indentation_hardness", Fixed::ONE));
    program
        .material_axes
        .push(fixed("mat.wear_coefficient", Fixed::from_int(30)));
    program
        .material_axes
        .push(fixed("mat.specific_cut_energy", Fixed::ONE));

    let mut race = a_sexed_race(0).with_morphogen(program.clone());
    let mut genes = race.genes.genes.clone();
    let mut freqs = vec![Fixed::from_ratio(1, 2); 3];
    let mut effects = vec![Fixed::ZERO; 3];
    // Address composition axes through the program's own accessor, so this stays correct as more param
    // categories are appended (the stroke-rate actuator block follows composition; `param_count() - 2` / `- 1`
    // would point at it, not at energy/water).
    let comp = program.composition_axes.len();
    let muscle = MorphogenParamId(program.composition_param(1) as u32); // fracture_strength = MUSCLE_STRENGTH
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

fn main() {
    let manifest = manifest();
    let resolution = a_scenario().resolve(&manifest).unwrap();
    let map = a_map(0xB0);

    let ticks = 40u32;
    println!("R-AGING (c) demo: aging emerges from per-segment first-passage wear on the run-path body.\n");

    for (label, fracture_energy) in [
        ("FRAIL  (fracture_energy 1)", Fixed::from_int(1)),
        ("TOUGH  (fracture_energy 8)", Fixed::from_int(8)),
    ] {
        let mut runner = build_dawn_runner(
            &manifest,
            &channels(),
            Profile::Development,
            &resolution,
            &map,
            &aging_peoples(fracture_energy),
            0x5111,
        )
        .expect("an aging dawn assembles");
        // Arm the aging runtime the harness leaves unset (physiology is already installed).
        let emb = runner.embodiment_mut().unwrap();
        emb.set_material_registry(civsim_physics::PhysicsRegistry::ground().unwrap());
        emb.set_wear(WearParams::dev_fixture());

        println!("=== {label} ===");
        let mut total_integrity_deaths = 0u32;
        let mut total_other_deaths = 0u32;
        let mut first_death_tick: Option<u32> = None;
        for t in 0..ticks {
            // Watch a living founder's integrity fall as its tissue wears.
            let sample = runner
                .embodiment()
                .and_then(|e| e.walkers().iter().find(|w| w.alive))
                .map(|w| w.homeostasis.level(INTEGRITY).to_f64_lossy());
            runner.step();
            let deaths = runner.take_obs_deaths();
            let integ = deaths.iter().filter(|a| **a == INTEGRITY).count() as u32;
            let other = deaths.len() as u32 - integ;
            total_integrity_deaths += integ;
            total_other_deaths += other;
            if integ > 0 && first_death_tick.is_none() {
                first_death_tick = Some(t);
            }
            if let Some(integ_level) = sample {
                let alive = runner
                    .embodiment()
                    .map(|e| e.walkers().iter().filter(|w| w.alive).count())
                    .unwrap_or(0);
                let bar = "#".repeat(((integ_level.max(0.0) * 40.0) as usize).min(40));
                println!(
                    "  tick {t:>2}: integrity {integ_level:.3} |{bar:<40}| alive {alive:>2}  \
                     deaths(aging {integ}, other {other})"
                );
            }
            if runner
                .embodiment()
                .map(|e| e.walkers().iter().all(|w| !w.alive))
                .unwrap_or(true)
            {
                println!("  (all founders have died)");
                break;
            }
        }
        match first_death_tick {
            Some(t) => println!(
                "  -> first first-passage aging death at tick {t}; {total_integrity_deaths} aging deaths, \
                 {total_other_deaths} other, over the run.\n"
            ),
            None => println!(
                "  -> no aging death in {ticks} ticks ({total_integrity_deaths} aging, \
                 {total_other_deaths} other); this body outlives the window.\n"
            ),
        }
    }

    println!(
        "The tougher tissue (8x the failure tolerance) ages slower and outlives the frail one: the \
         size-longevity relation is a pure OUTPUT of the tolerance-versus-wear balance, never an authored law."
    );
}
