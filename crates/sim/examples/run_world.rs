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

//! QUARANTINED DEV-FIXTURE RUN HARNESS (not canonical). A whole scenario world run end to end so its
//! emergence can be observed: a declared dawn population (index-varied races on isolated founding
//! bands) is assembled by [`civsim_sim::build_dawn_runner`] into ONE runner whose founders are at
//! once cognition minds and located, metabolizing bodies, then ticked over a number of life-cadence
//! generations while emergence readers print what arises: population and lineage counts, a language
//! signal (within-band word consensus and cross-band divergence), a belief signal (per-band axiom
//! stance and its spread), the census-derived effective population size, and a physiology signal
//! (mean body temperature, births and deaths per window).
//!
//! Every number here is a labelled DEV FIXTURE loaded from `calibration/profiles/dev-fixtures.toml`
//! under [`Profile::Development`], never the owner's calibration (design Principle 11, the
//! reserved-value discipline). A production run uses [`Profile::Calibrated`] against
//! `calibration/reserved.toml` and refuses to start while anything it needs is reserved. This
//! harness exists only to exercise the mechanisms and watch them run.
//!
//! The manifest life cadence is one Earth year in ticks (31_536_000 at one second per tick), which is
//! not tickable in a demonstration, so this harness overrides it to a small dev cadence exactly as the
//! world-build tests do ([`GEN_TICKS`] ticks per generation) and ages the founders past maturity at the
//! dawn so reproduction fires within the run. These two overrides are dev-harness scaffolding, not
//! canon, and are the only departures from the pure world-build path.
//!
//! Run: `cargo run --release --example run_world -- --seed <u64> --races <n> --bands <n>
//! --generations <n> [--scenario <name>]`. Same arguments reproduce the same `state_hash` (Principle
//! 3), printed at every snapshot and at the end so a run is reproducible.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use civsim_core::{Fixed, GaussApprox, StableId};
use civsim_sim::anatomy::{BodyPlan, BodyPlanRegistry, Part, Temperament};
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::homeostasis::{AffordanceRegistry, HomeostaticRegistry};
use civsim_sim::langmod::PerceptualParams;
use civsim_sim::language::{ConceptId, FeatureDimId, ProductionModalityId, Word};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::runner::Runner;
use civsim_sim::scenario::{Scenario, ScenarioResolution};
use civsim_sim::sensorium::SenseChannelId;
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    append_controller_block, build_dawn_runner, nsm_gloss, taxis_move_weights, Articulation, Axiom,
    AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId, BreedingSystemRegistry, Channel,
    CognitionChannel, ControllerLayout, Curve, DawnPeoples, Direction, DominanceKind,
    DominanceMode, EmbodimentGenesis, EpistemicStance, EvidenceRing, GeneDef, GeneEffect, GeneId,
    GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs, LanguageGenesis, PersonalityProfile,
    PersonalityRegistry, Race, RaceId, ReproductionMode, SchemeId, SourceModeId, TraitAxisId,
    TraitDef, ValueAxisId, ValueProfile, World,
};
use civsim_world::{BiomeSet, FlatBounded, TileMap, WorldgenParams};

// --- dev-harness scaffolding (documented departures from the pure world-build path) ---

/// The dev-harness life cadence in ticks: how many ticks make one generation here (the manifest
/// cadence of one Earth year in ticks is not tickable in a demonstration, so the world-build tests
/// and this harness override it). Sized so a generation has real room for the cognition beats: the
/// naming game coordinates words over many ticks, and each generation adds a cohort of newborns that
/// start with empty lexicons (the world-build birth does not yet copy a parent's lexicon), so the
/// naming game needs ticks to fold them into their band's consensus before the next cohort arrives.
const GEN_TICKS: u64 = 128;

/// Founding members per band, so the founding population scales by the band count. Enough per band
/// that a gene-fed two-sex cohort holds a compatible pair to breed.
const MEMBERS_PER_BAND: usize = 6;

/// The voice reception channel the founding races hear speech on (labelled fixture).
const VOICE: SenseChannelId = SenseChannelId(1);

// --- base-level liveliness step 1: the founding thermotaxis reaction norm and the selection dials.
// Labelled DEV FIXTURES (the harness convention). Their reserved-with-basis home for a canonical race
// genesis is `controller.taxis.*`, `genome.mutation_rate`, `genome.additive_mutation_step`, and the
// per-race `environment_variance` in the manifest; here they are stood up as dev values so the run
// moves. ---

/// The MOVE affordance's activation output index in the dev-default affordance layout (MOVE is the
/// lowest affordance id, so its activation is output 0, its heading components outputs 1 and 2).
const MOVE_OUTPUT: usize = 0;
/// The TEMPERATURE axis's input block base in the dev-thermal registry (its only axis, so index 0): its
/// level, here-flag, two source-direction components, then its signed slot.
const TEMPERATURE_INPUT_BASE: usize = 0;
/// DEV FIXTURE: the founding move-activation bias, set decisively to one so the clamped MOVE activation
/// saturates and a founder wants to move (basis: the activation magnitude at which MOVE beats a resting
/// zero and the being leaves its cell; reserved for a canonical genesis).
const TAXIS_MOVE_BIAS: Fixed = Fixed::ONE;
/// DEV FIXTURE: the founding heading gain on the temperature source-direction percept, set to one so
/// the MOVE heading follows the unit gradient direction (basis: the heading-follow strength; reserved).
const TAXIS_HEADING_GAIN: Fixed = Fixed::ONE;
/// DEV FIXTURE: the per-locus per-generation structural mutation rate, opened off zero so the founding
/// controller weights drift and the movement-dependent fitness a later step gives them has a heritable
/// gradient to select on (basis: the reserved `genome.mutation_rates` baseline; small so it explores
/// without swamping selection).
fn mutation_rate() -> Fixed {
    Fixed::from_ratio(1, 100)
}
/// DEV FIXTURE: the additive mutation step, opened off zero so a controller weight can drift its
/// magnitude across generations (basis: the reserved additive-step end; small).
fn mutation_step() -> Fixed {
    Fixed::from_ratio(1, 20)
}
/// DEV FIXTURE: the per-being developmental-environment variance half-width, opened off zero so
/// littermates vary developmentally (basis: the reserved per-race `environment_variance`; small).
fn env_variance() -> Fixed {
    Fixed::from_ratio(1, 20)
}

/// The controller layout the founding thermotaxis block is sized against: the same registries the
/// embodiment genesis installs (the dev-thermal homeostatic axes and the dev-default affordances, a
/// reaction norm at hidden width zero), so `weight_count` and the taxis weight indices match the
/// controller a founder expresses.
fn dawn_layout() -> ControllerLayout {
    ControllerLayout::new(
        &HomeostaticRegistry::dev_thermal(),
        &AffordanceRegistry::dev_default(),
        0,
    )
}

/// The concepts the language reader samples, the first few NSM semantic primes (the anchor meanings a
/// band coordinates words for first). Kept short so a snapshot line stays legible.
const SAMPLED_CONCEPTS: [u32; 6] = [1, 2, 3, 4, 5, 6];

// --- command-line interface ---

/// The resolved run configuration. `races`, `bands`, and `generations` are counts; `diversity_step`
/// and `pool_ne` are the scenario nudges (a neutral baseline when no scenario is named).
struct Config {
    seed: u64,
    races: usize,
    bands: usize,
    generations: u64,
    scenario: Option<String>,
    /// The per-race divergence step (tenths): how far each successive race is pushed off the shared
    /// baseline in its genes, its vocal tract, and its innate belief. Larger under a high-diversity
    /// scenario posture.
    diversity_step: i64,
    /// The founder gene-pool effective size Ne (the dawn drift strength before the census-derived Ne
    /// takes over). Smaller under an effective-population-size low posture.
    pool_ne: u32,
    /// The ambient medium name, if the scenario selects one the dev-fixtures manifest carries.
    medium: Option<String>,
}

/// Parse the arguments simply, with sane defaults, then fold in any named scenario's postures.
/// Precedence: an explicit flag wins over a scenario posture, which wins over the neutral default.
fn parse_config() -> Config {
    let mut seed: Option<u64> = None;
    let mut races: Option<usize> = None;
    let mut bands: Option<usize> = None;
    let mut generations: Option<u64> = None;
    let mut scenario: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--seed" => seed = args.next().and_then(|v| v.parse().ok()),
            "--races" => races = args.next().and_then(|v| v.parse().ok()),
            "--bands" => bands = args.next().and_then(|v| v.parse().ok()),
            "--generations" => generations = args.next().and_then(|v| v.parse().ok()),
            "--scenario" => scenario = args.next(),
            "--help" | "-h" => {
                eprintln!(
                    "usage: run_world --seed <u64> --races <n> --bands <n> --generations <n> \
                     [--scenario <name>]"
                );
                std::process::exit(0);
            }
            other => eprintln!("run_world: ignoring unknown argument {other}"),
        }
    }

    // Load the named scenario's postures (parse only, never resolve its dials against the dev-fixtures
    // manifest: those dial ids are not in the fixtures profile, so a full resolve would fail loud; the
    // postures we read here are separate from that). A missing file falls back to the neutral baseline.
    let loaded = scenario.as_ref().and_then(|name| {
        let path = format!("{}/../../scenarios/{name}.toml", env!("CARGO_MANIFEST_DIR"));
        match Scenario::load(&path) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("run_world: could not load scenario {name} ({e:?}), using baseline");
                None
            }
        }
    });

    // The scenario nudges: count -> race count, diversity -> divergence step, the
    // effective-population-size dial -> founder pool Ne, and the selected medium (only when the
    // dev-fixtures manifest carries a profile for it).
    let (posture_races, diversity_step, pool_ne, medium) = match &loaded {
        Some(s) => {
            let races_from_count = match s.races.count.as_str() {
                "few" => 2,
                "several" | "some" => 3,
                "many" => 5,
                _ => 3,
            };
            let step = if s.races.diversity == "high" { 2 } else { 1 };
            let ne = match s.dial("genome.effective_population_size") {
                Some(Direction::Low) => 8,
                Some(Direction::High) => 40,
                _ => 20,
            };
            let medium = s
                .scenario
                .medium
                .clone()
                .filter(|m| m == "water" || m == "air");
            (Some(races_from_count), step, ne, medium)
        }
        None => (None, 1, 20, None),
    };

    Config {
        seed: seed.unwrap_or(1),
        races: races.or(posture_races).unwrap_or(3).max(1),
        bands: bands.unwrap_or(4).max(1),
        generations: generations.unwrap_or(20).max(1),
        scenario,
        diversity_step,
        pool_ne,
        medium,
    }
}

// --- the declared dawn population, parameterized and index-varied ---

/// A full founding race, varied by index so the races diverge in isolation: two quantitative
/// cognition loci (acuity, memory), a sex-determination locus (so a gene-fed two-sex cohort can
/// breed), a vocal tract scaled off the shared base geometry (so its phonetics and coined words
/// differ), an innate belief stance pushed off the baseline, and a body plan (so each founder
/// embodies as a located, thermoregulating body). Every value is a labelled fixture. The mechanism
/// reads no race id: the races diverge only through this per-index data (Principle 9).
fn full_race(index: usize, cfg: &Config) -> Race {
    let i = index as i64;
    let step = cfg.diversity_step;

    let mut genes = vec![
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
    ];

    // The two cognition loci start at index-shifted allele frequencies (races begin genetically
    // apart), while the sex locus stays balanced so both sexes appear and pairs can breed. The
    // frequencies are clamped to a sane interior band. The cognition and sex loci carry a flat additive
    // spine (effect zero); the controller block below adds its own.
    let freq0 = clamp_tenths(5 + i * step);
    let freq1 = clamp_tenths(5 - i * step);
    let mut freqs = vec![freq0, freq1, Fixed::from_ratio(1, 2)];
    let mut effects = vec![Fixed::ZERO, Fixed::ZERO, Fixed::ZERO];

    // Base-level liveliness step 1: append the founding controller gene block seeding a temperature
    // thermotaxis reaction norm, so a founder MOVES along the temperature gradient the runner senses
    // (the dev-thermal registry's only axis, dev-default's MOVE the directional output 0). The full
    // controller substrate is seeded (a gene per weight, matching `evolve::controller_gene_set`), with
    // the taxis magnitudes carried in the pool additive spine; every other weight starts at zero and can
    // mutate on. Reads no race id: the seeds are the same for every race (Principle 9).
    let layout = dawn_layout();
    let seeds = taxis_move_weights(
        &layout,
        MOVE_OUTPUT,
        TEMPERATURE_INPUT_BASE,
        TAXIS_MOVE_BIAS,
        TAXIS_HEADING_GAIN,
    );
    // SexualDiploid (below), so ploidy two.
    append_controller_block(
        &mut genes,
        &mut freqs,
        &mut effects,
        2,
        layout.weight_count(),
        &seeds,
    );
    // The stamped integer-Gaussian approximation the additive spine draws through (the labelled
    // SumOfUniforms{k=12} default of design 25.10; a canonical build reads genome.gauss_approx). The
    // seeded loci sit at frequency one, so their within-locus deviation is zero and the draw is scaled
    // out, but promote still draws it, so the stamp must be a real one, not the unset sentinel.
    let pool = GenePool::new(SchemeId(0), cfg.pool_ne, freqs)
        .with_additive(effects, GaussApprox::SumOfUniforms { k: 12 });

    // The innate belief stance walks off the baseline by index, so lineages of different races start
    // from different convictions and their per-band means diverge.
    let stance = clamp_tenths(4 + i * step);
    let intrinsic = IntrinsicBeliefs {
        values: ValueProfile::with([(ValueAxisId(0), 3)]),
        axioms: vec![Axiom {
            axis: AxiomAxisId(0),
            stance,
            strength: Fixed::from_ratio(1, 2),
            confidence: Fixed::from_ratio(1, 2),
            entrenchment: 5,
            salience: Fixed::from_ratio(1, 2),
            stubbornness: Fixed::from_ratio(1, 4),
            innate_seed: stance,
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

    // Base-level liveliness step 1: open the selection dials off zero (they were both zero, so a weight
    // could not drift even if a locus existed), so the seeded controller weights mutate and the
    // movement-dependent fitness a later step gives them has a heritable gradient to select on. Mutation
    // uses the counter-keyed genome draw, so the run stays deterministic.
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: mutation_rate(),
        additive_mutation_step: mutation_step(),
        gauss: GaussApprox::SumOfUniforms { k: 12 },
    };

    // The vocal tract is scaled off the shared base geometry by index, so each race derives a
    // different producible sound inventory and coins observably different words. Clamped so a small
    // tract still yields a non-empty inventory (the derivation fails loud otherwise).
    let tract = clamp_tenths(10 - i * step);

    Race::new(
        RaceId(index as u32),
        GeneSet { genes },
        pool,
        scheme,
        intrinsic,
        Fixed::from_int(2),
        env_variance(),
        80,
        18,
    )
    .with_breeding(BreedingSystemId(0))
    .with_articulation(Articulation {
        vocal_tract_scale: tract,
        hearing_resolution: Fixed::from_int(20),
    })
    .with_body_plan(mobile_body())
}

/// Clamp an integer count of tenths into the interior band `[0.3, 0.9]` and return it as a `Fixed`,
/// so an index-varied frequency, stance, or tract scale stays valid however far the divergence step
/// pushes it.
fn clamp_tenths(tenths: i64) -> Fixed {
    Fixed::from_ratio(tenths.clamp(3, 9), 10)
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

/// A one-axis personality profile maturing toward a positive target, so the life-cadence personality
/// beat has something to drift. Labelled fixture.
fn a_personality() -> PersonalityProfile {
    PersonalityProfile::new([TraitDef {
        axis: TraitAxisId(0),
        plasticity_curve: Curve::new([(Fixed::ZERO, Fixed::ONE), (Fixed::ONE, Fixed::ZERO)]),
        maturity_target: Fixed::from_ratio(1, 2),
    }])
}

/// The shared language genesis: candidate sounds on the shared base geometry, air acoustics, engine
/// caps, and reserved thresholds, from which each race derives its own phonetic form system by bending
/// the base with its vocal tract. Labelled fixture (mirrors the world-build test genesis).
fn language_genesis() -> LanguageGenesis {
    LanguageGenesis {
        base_lengths: (12..=16).map(|cm| Fixed::from_ratio(cm, 100)).collect(),
        modality: ProductionModalityId(0),
        dim: FeatureDimId(0),
        sound_speed: Fixed::from_int(340),
        absorption_reference: Fixed::from_ratio(1, 100_000_000),
        path: Fixed::from_int(10),
        perceptual: PerceptualParams {
            modes: 3,
            freq_max: Fixed::from_int(100_000),
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

/// The embodiment genesis: the shared substrate registries a founder's body reads against. Labelled
/// fixture (mirrors the world-build test genesis).
fn embodiment_genesis() -> EmbodimentGenesis {
    EmbodimentGenesis {
        homeostatic: HomeostaticRegistry::dev_thermal(),
        affordances: AffordanceRegistry::dev_default(),
        locomotion: LocomotionParams::dev_default(),
        organs: BodyPlanRegistry::dev_default(),
        controller_hidden: 0,
        submerged_medium_id: "medium.water".to_string(),
        emergent_medium_id: "medium.air".to_string(),
    }
}

/// Assemble the declared peoples from the config: `races` index-varied races, `bands` founding bands
/// round-robined across the races and each placed at a distinct cell (a distinct `PlaceId`, so
/// lineages diverge in isolation), a language genesis and an embodiment genesis so both signals run
/// live, and a mild raw-age mortality hazard so the population turns over and deaths are observable.
fn assemble_peoples(cfg: &Config) -> DawnPeoples {
    let mut races = BTreeMap::new();
    for index in 0..cfg.races {
        races.insert(RaceId(index as u32), full_race(index, cfg));
    }

    // Round-robin the bands across the races and place each at a distinct cell. When bands outnumber
    // races, the early races receive a second band at a separate place, so a race's two isolated bands
    // coin their own words and diverge (the language signal reads exactly this).
    let bands = (0..cfg.bands)
        .map(|b| BandSpec {
            race: RaceId((b % cfg.races) as u32),
            place: ((b + 1) * 10) as u32,
            members: MEMBERS_PER_BAND,
        })
        .collect();

    let mut breeding = BreedingSystemRegistry::new();
    breeding.insert(BreedingSystem::dev_binary_anisogamy(BreedingSystemId(0)));

    let mut personality = PersonalityRegistry::new();
    for index in 0..cfg.races {
        personality.set(RaceId(index as u32), a_personality());
    }

    // A gentle rising raw-age hazard: near-certain survival while young, rising through mid-age to
    // certain death by an age past the founders' lifespan, so the oldest turn over each generation
    // while the young persist. A labelled fixture, the mortality half the DawnPeoples hazard field is
    // built for.
    let mortality_hazard = Some(Curve::new([
        (Fixed::ZERO, Fixed::ZERO),
        (Fixed::from_int(30), Fixed::ZERO),
        (Fixed::from_int(90), Fixed::ONE),
    ]));

    DawnPeoples {
        races,
        bands,
        breeding,
        personality,
        mortality_hazard,
        language: Some(language_genesis()),
        embodiment: Some(embodiment_genesis()),
    }
}

/// A minimal resolved scenario carrying only the selected medium, so the field derives from that
/// medium end to end (a world of water conducts heat at its own rate). The full scenario's dials stay
/// unresolved on purpose (see [`parse_config`]). Falls back to the default air medium if the named
/// medium does not resolve.
fn resolve_medium(manifest: &CalibrationManifest, medium: &Option<String>) -> ScenarioResolution {
    let toml = match medium {
        Some(m) => format!("[scenario]\nid = \"run\"\nname = \"Run\"\nmedium = \"{m}\"\n"),
        None => "[scenario]\nid = \"run\"\nname = \"Run\"\n".to_string(),
    };
    let scenario = Scenario::from_toml_str(&toml).expect("the inline scenario parses");
    scenario.resolve(manifest).unwrap_or_else(|_| {
        Scenario::from_toml_str("[scenario]\nid = \"run\"\nname = \"Run\"\n")
            .unwrap()
            .resolve(manifest)
            .expect("the neutral scenario resolves against the dev-fixtures manifest")
    })
}

// --- fixtures shared with the world-build path ---

fn manifest() -> CalibrationManifest {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../calibration/profiles/dev-fixtures.toml"
    );
    CalibrationManifest::load(path).expect("the dev-fixtures profile loads")
}

fn channels() -> AccessChannelRegistry {
    AccessChannelRegistry::from_toml_str(
        "[[channels]]\nid = 1\nname = \"witnessed\"\nmargin_steps = 1\n\
         [[channels]]\nid = 2\nname = \"told\"\nmargin_steps = 0\n\
         [[channels]]\nid = 3\nname = \"said\"\nmargin_steps = -1\n",
    )
    .unwrap()
}

// --- emergence readers ---

/// A living band grouped for the readers: its frozen dawn place, the race that founded it (read from a
/// member), and its current members (founders plus the newborns that inherited the place).
type Band = (u32, Option<RaceId>, Vec<StableId>);

/// The living bands grouped by their frozen dawn place, in place order, each carrying the race that
/// founded it (read from a member). A newborn inherits its parents' place, so a band grows as its
/// lineage grows and this stays the isolation grouping.
fn bands_by_place(w: &World) -> Vec<Band> {
    let mut grouped: BTreeMap<u32, Vec<StableId>> = BTreeMap::new();
    for id in w.being_ids() {
        if let Some(place) = w.place_of(id) {
            grouped.entry(place).or_default().push(id);
        }
    }
    grouped
        .into_iter()
        .map(|(place, ids)| {
            let race = ids.first().and_then(|&id| w.race_of(id));
            (place, race, ids)
        })
        .collect()
}

/// The within-band consensus fraction over the sampled concepts: for each concept, among a band's
/// members that have coined a word, the fraction agreeing on the most common word; averaged over the
/// concepts any member has named. `None` when no member of the band has coined any sampled word yet.
///
/// A caveat the reader surfaces rather than hides: the world-build birth (`World::birth`) does not yet
/// copy a parent's lexicon or assign a newborn its band lineage (a newborn falls back to `LangId(0)`
/// with an empty lexicon), so a fast-growing band fills with un-converged newborns and this fraction
/// decays as reproduction outruns the naming game. The founder cohort still shows the clean converge/
/// diverge signal (see [`divergence_comparison`]); newborn lexicon inheritance is a real not-yet-wired
/// seam in the world-build path, not a reader defect.
fn band_consensus(w: &World, members: &[StableId]) -> Option<f64> {
    let mut sum = 0.0;
    let mut named = 0;
    for &c in &SAMPLED_CONCEPTS {
        let concept = ConceptId(c);
        let mut counts: BTreeMap<Word, u32> = BTreeMap::new();
        for &id in members {
            if let Some(word) = w.word_for(id, concept) {
                *counts.entry(word).or_insert(0) += 1;
            }
        }
        let total: u32 = counts.values().sum();
        if total == 0 {
            continue;
        }
        let modal = counts.values().copied().max().unwrap_or(0);
        sum += modal as f64 / total as f64;
        named += 1;
    }
    (named > 0).then(|| sum / named as f64)
}

/// The cognate overlap between two band representatives: how many of the sampled concepts they name
/// with the same word. Within a band this is high (co-location coordinates), across separated bands it
/// falls (isolation diverges), and across races it falls further (different phonetics).
fn cognate_overlap(w: &World, a: StableId, b: StableId) -> u32 {
    SAMPLED_CONCEPTS
        .iter()
        .filter(|&&c| {
            let concept = ConceptId(c);
            let wa = w.word_for(a, concept);
            wa.is_some() && wa == w.word_for(b, concept)
        })
        .count() as u32
}

/// Render a coined word as a short readable token by mapping each form segment's first feature value
/// to a syllable, so two lineages' divergent words read differently at a glance. A word with no
/// segments renders as a dash.
fn render_word(word: &Word) -> String {
    const SYLLABLES: [&str; 12] = [
        "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
    ];
    if word.is_empty() {
        return "-".to_string();
    }
    word.segments()
        .iter()
        .map(|seg| {
            seg.features()
                .first()
                .map(|(_, v)| SYLLABLES[v.0 as usize % SYLLABLES.len()])
                .unwrap_or("_")
        })
        .collect()
}

/// The mean and spread of the founding axiom stance within a band (axis 0): the mean is the lineage's
/// current conviction, the spread is how far its members disagree. Reads the intrinsic beliefs
/// directly. `None` for a band whose members carry no such axiom.
fn band_belief(w: &World, members: &[StableId]) -> Option<(f64, f64)> {
    let stances: Vec<f64> = members
        .iter()
        .filter_map(|&id| {
            w.intrinsic_of(id)
                .and_then(|b| b.axioms.iter().find(|a| a.axis == AxiomAxisId(0)))
                .map(|a| a.stance.to_f64_lossy())
        })
        .collect();
    if stances.is_empty() {
        return None;
    }
    let mean = stances.iter().sum::<f64>() / stances.len() as f64;
    let var = stances.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / stances.len() as f64;
    Some((mean, var.sqrt()))
}

/// The mean designs-known per being (the knowledge-depth signal). NOT-YET-OBSERVABLE as nonzero:
/// `knowledge_of` is a live reader, but the world-build path arms no design origination (Part 41), so
/// no design ever enters a being's knowledge and this reads zero every snapshot. Reported honestly as
/// the live-but-inert reader it is, rather than substituting a fabricated number.
fn mean_knowledge(w: &World) -> f64 {
    let ids = w.being_ids();
    if ids.is_empty() {
        return 0.0;
    }
    let total: usize = ids
        .iter()
        .map(|&id| w.knowledge_of(id).map(|k| k.known.len()).unwrap_or(0))
        .sum();
    total as f64 / ids.len() as f64
}

/// The migration and dispersal reader (base-level liveliness step 1): how far the located population has
/// spread from its dawn cells. Reports the distinct occupied cells now (it starts at one per band and
/// grows as founders disperse along the temperature gradient), the count of beings standing off every
/// dawn cell, and the greatest Chebyshev displacement of any being from the nearest dawn cell. Reads the
/// runner's located walkers (a pure read of hashed state, so it never perturbs the run). `None` if the
/// runner carries no embodied population.
fn migration(runner: &Runner, dawn_cells: &BTreeSet<(i32, i32)>) -> Option<(usize, usize, i32)> {
    let emb = runner.embodiment()?;
    let mut occupied: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut off_dawn = 0usize;
    let mut max_disp = 0i32;
    for w in emb.walkers() {
        let c = w.coord();
        let cell = (c.x, c.y);
        occupied.insert(cell);
        if !dawn_cells.contains(&cell) {
            off_dawn += 1;
        }
        let nearest = dawn_cells
            .iter()
            .map(|&(dx, dy)| (c.x - dx).abs().max((c.y - dy).abs()))
            .min()
            .unwrap_or(0);
        max_disp = max_disp.max(nearest);
    }
    Some((occupied.len(), off_dawn, max_disp))
}

/// The distinct cells the embodied population stands on at the dawn, before any tick: one per founding
/// band (each band spawns its members on its band cell), the baseline the migration reader measures
/// dispersal against.
fn dawn_cells(runner: &Runner) -> BTreeSet<(i32, i32)> {
    runner
        .embodiment()
        .map(|emb| {
            emb.walkers()
                .iter()
                .map(|w| {
                    let c = w.coord();
                    (c.x, c.y)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The per-cell field-state reader (base-level liveliness step 2): samples the environmental field
/// stack, reporting the fraction of cells holding standing water, the mean and peak water depth, and the
/// mean producer biomass (the food supply the productivity derives). A pure read of hashed state.
/// `None` if the runner carries no environmental stack.
fn field_state(runner: &Runner) -> Option<(f64, f64, f64, f64)> {
    let env = runner.environ()?;
    let (w, h) = env.dims();
    let n = (w as f64) * (h as f64);
    if n <= 0.0 {
        return None;
    }
    let mut wet = 0.0;
    let mut water_sum = 0.0;
    let mut water_max = 0.0f64;
    let mut biomass_sum = 0.0;
    for y in 0..h {
        for x in 0..w {
            let water = env.water_at(x, y).to_f64_lossy();
            if water > 0.0 {
                wet += 1.0;
            }
            water_sum += water;
            water_max = water_max.max(water);
            biomass_sum += env.biomass_at(x, y).to_f64_lossy();
        }
    }
    Some((wet / n, water_sum / n, water_max, biomass_sum / n))
}

/// The mean body temperature over the living, embodied population, in the manifest's thermal units.
/// `None` if no being carries a body temperature.
fn mean_body_temp(runner: &Runner) -> Option<f64> {
    let w = runner.world()?;
    let temps: Vec<f64> = w
        .being_ids()
        .iter()
        .filter_map(|&id| runner.body_temp(id).map(|t| t.to_f64_lossy()))
        .collect();
    if temps.is_empty() {
        return None;
    }
    Some(temps.iter().sum::<f64>() / temps.len() as f64)
}

/// Print one emergence snapshot: population and lineage counts, the effective sizes, the language,
/// belief, knowledge, and physiology signals, the births and deaths since the previous snapshot, and
/// the current `state_hash`.
fn snapshot(
    label: &str,
    runner: &Runner,
    cfg: &Config,
    prev: &BTreeSet<StableId>,
    dawn: &BTreeSet<(i32, i32)>,
) -> BTreeSet<StableId> {
    let w = runner.world().expect("the unified runner carries a world");
    let bands = bands_by_place(w);
    let current: BTreeSet<StableId> = w.being_ids().into_iter().collect();

    let births = current.difference(prev).count();
    let deaths = prev.difference(&current).count();

    // "Distinct peoples" is read as the distinct races present among the living, and "lineages" as the
    // distinct founding-band places. NOT-YET-OBSERVABLE: a canonical species/cladogenesis count. The
    // World exposes no speciation reader (a race splitting into daughter species is a later arc), so
    // this reports the seeded races and band lineages rather than faking an emergent species tally.
    let distinct_races: BTreeSet<RaceId> = current.iter().filter_map(|&id| w.race_of(id)).collect();

    println!(
        "=== {label} (tick {}, gen {}) ===",
        runner.clock(),
        runner.clock() / GEN_TICKS
    );
    println!(
        "  population {}  |  lineages {} (bands)  |  distinct peoples {} (races present)",
        w.population(),
        bands.len(),
        distinct_races.len(),
    );

    // Per-lineage counts, each band tagged with its founding race.
    let per_lineage: Vec<String> = bands
        .iter()
        .map(|(place, race, ids)| {
            let r = race.map(|r| r.0 as i64).unwrap_or(-1);
            format!("p{place}=r{r}:{}", ids.len())
        })
        .collect();
    println!("  per-lineage: {}", per_lineage.join("  "));

    // Effective population size: the census-derived Ne per race, the value drift_pools set from each
    // race's own reproductive census this generation (it replaces the authored founder pool size after
    // the first generation, retiring audit deviation 23 for the post-dawn tier). The race-blind census
    // window itself is cleared at each generation boundary (reset_census_window fires in the same life
    // beat we snapshot just after), so only its ordinal, not its tally, is meaningful here.
    let per_ne: Vec<String> = (0..cfg.races)
        .map(|index| {
            let ne = w
                .gene_pool(RaceId(index as u32))
                .map(|p| p.effective_size)
                .unwrap_or(0);
            format!("r{index}={ne}")
        })
        .collect();
    println!(
        "  effective size Ne (census-derived, per race): {}  |  census window {}",
        per_ne.join(" "),
        w.census().window(),
    );

    // The language signal: mean within-band consensus, and the within/across-place/across-race cognate
    // overlap over the sampled concepts.
    print_language(w, &bands);

    // The belief signal: per-band mean axiom stance and its spread (lineages diverge in conviction).
    let beliefs: Vec<String> = bands
        .iter()
        .filter_map(|(place, _, ids)| {
            band_belief(w, ids).map(|(mean, spread)| format!("p{place}:{mean:.2}±{spread:.2}"))
        })
        .collect();
    println!("  belief axiom-0 stance: {}", beliefs.join("  "));
    println!("  mean knowledge (designs/being): {:.2}", mean_knowledge(w));

    // The physiology signal: mean body temperature, and the observed births and deaths this window.
    match mean_body_temp(runner) {
        Some(t) => println!(
            "  physiology: mean body_temp {t:.3}  |  births {births}  deaths {deaths} (this window)"
        ),
        None => println!("  physiology: no embodied bodies  |  births {births}  deaths {deaths}"),
    }

    // The field-state signal (step 2): the environmental stack's water and productivity.
    match field_state(runner) {
        Some((wet, mean_water, max_water, mean_biomass)) => println!(
            "  field: {:.0}% cells wet  |  mean water {mean_water:.3} (peak {max_water:.3})  |  \
             mean productivity {mean_biomass:.3}",
            wet * 100.0
        ),
        None => println!("  field: no environmental stack"),
    }

    // The migration signal (step 1): dispersal of the located population from its dawn cells.
    match migration(runner, dawn) {
        Some((cells, off, disp)) => println!(
            "  migration: {cells} distinct cells occupied (from {} dawn cells)  |  {off} beings off \
             their dawn cell  |  max displacement {disp}",
            dawn.len()
        ),
        None => println!("  migration: no located population"),
    }

    println!("  state_hash: {:032x}", runner.state_hash());
    current
}

/// The language section of a snapshot: mean within-band consensus over the sampled concepts, one
/// band's sample words, and the cognate-overlap comparison (within a band, across two bands of one
/// race, across two races) that shows convergence in isolation and divergence between lineages.
fn print_language(w: &World, bands: &[Band]) {
    let consensus: Vec<f64> = bands
        .iter()
        .filter_map(|(_, _, ids)| band_consensus(w, ids))
        .collect();
    if consensus.is_empty() {
        println!("  language: no words coined yet");
        return;
    }
    let mean = consensus.iter().sum::<f64>() / consensus.len() as f64;

    // A sample of the first band's coined words, to show they are real forms.
    let sample: Vec<String> = bands
        .first()
        .map(|(_, _, ids)| {
            SAMPLED_CONCEPTS
                .iter()
                .filter_map(|&c| {
                    let gloss = nsm_gloss(ConceptId(c)).unwrap_or("?");
                    ids.first()
                        .and_then(|&id| w.word_for(id, ConceptId(c)))
                        .map(|word| format!("{gloss}={}", render_word(&word)))
                })
                .collect()
        })
        .unwrap_or_default();
    println!(
        "  language: mean within-band consensus {:.0}%  |  band0 words: {}",
        mean * 100.0,
        sample.join(" ")
    );

    // The divergence comparison: pick a within-band pair, a same-race across-place pair, and a
    // cross-race pair, and report cognate overlap over the sampled concepts.
    if let Some(cmp) = divergence_comparison(w, bands) {
        println!(
            "  language divergence (shared words / {}): {cmp}",
            SAMPLED_CONCEPTS.len()
        );
    }
}

/// Build the cognate-overlap comparison string: within one band, across two bands of the same race,
/// and across two races. Any comparison whose participants do not exist is omitted.
fn divergence_comparison(w: &World, bands: &[Band]) -> Option<String> {
    let mut parts = Vec::new();

    // Within a band: the first band with two members.
    if let Some((place, _, ids)) = bands.iter().find(|(_, _, ids)| ids.len() >= 2) {
        parts.push(format!(
            "within p{place}={}",
            cognate_overlap(w, ids[0], ids[1])
        ));
    }

    // Across two bands of one race at separate places.
    let mut by_race: BTreeMap<u32, Vec<&Band>> = BTreeMap::new();
    for band in bands {
        if let Some(r) = band.1 {
            by_race.entry(r.0).or_default().push(band);
        }
    }
    if let Some((race, group)) = by_race.iter().find(|(_, g)| g.len() >= 2) {
        let a = group[0];
        let b = group[1];
        if let (Some(&ia), Some(&ib)) = (a.2.first(), b.2.first()) {
            parts.push(format!(
                "across-place r{race}(p{}|p{})={}",
                a.0,
                b.0,
                cognate_overlap(w, ia, ib)
            ));
        }
    }

    // Across two races: the first bands of two distinct races.
    let races: Vec<&Band> = {
        let mut seen = BTreeSet::new();
        bands
            .iter()
            .filter(|b| b.1.is_some_and(|r| seen.insert(r.0)))
            .collect()
    };
    if races.len() >= 2 {
        if let (Some(&ia), Some(&ib)) = (races[0].2.first(), races[1].2.first()) {
            parts.push(format!(
                "across-race r{}|r{}={}",
                races[0].1.unwrap().0,
                races[1].1.unwrap().0,
                cognate_overlap(w, ia, ib)
            ));
        }
    }

    (!parts.is_empty()).then(|| parts.join("  "))
}

// --- the run ---

fn main() {
    let cfg = parse_config();
    let manifest = manifest();
    let channels = channels();
    let resolution = resolve_medium(&manifest, &cfg.medium);

    // A generated world large enough that the founding bands land on distinct cells.
    let topo = FlatBounded::new(32, 24, 1);
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(cfg.seed, topo, &biomes, &WorldgenParams::dev_default());

    let peoples = assemble_peoples(&cfg);

    println!(
        "run_world: DEV-FIXTURE HARNESS (Profile::Development, labelled fixtures, not owner canon)"
    );
    println!(
        "  seed {}  races {}  bands {}  generations {}  scenario {}",
        cfg.seed,
        cfg.races,
        cfg.bands,
        cfg.generations,
        cfg.scenario.as_deref().unwrap_or("<baseline>"),
    );
    println!(
        "  founders {} ({} bands x {} members)  gen_ticks {}  pool_ne {}  diversity_step {}  medium {}",
        cfg.bands * MEMBERS_PER_BAND,
        cfg.bands,
        MEMBERS_PER_BAND,
        GEN_TICKS,
        cfg.pool_ne,
        cfg.diversity_step,
        cfg.medium.as_deref().unwrap_or("air"),
    );

    // Build the unified runner. build_dawn_runner already arms reproduction and post-dawn generational
    // drift (worldbuild.rs: set_reproduction + arm_generational_drift), and the life cadence resets the
    // census window each generation, so nothing further is armed here.
    let mut runner = build_dawn_runner(
        &manifest,
        &channels,
        Profile::Development,
        &resolution,
        &map,
        &peoples,
        cfg.seed,
    )
    .expect("the dawn assembles a unified runner");

    // Dev-harness scaffolding (the two documented overrides): a small life cadence so generations map
    // to a tickable number of ticks, and founders aged past maturity so reproduction fires within the
    // run. Both mirror the world-build tests.
    {
        let w = runner.world_mut().expect("the runner carries a world");
        w.set_life_cadence(GEN_TICKS);
        for id in w.being_ids() {
            w.set_age(id, 20);
        }
    }

    let founders: BTreeSet<StableId> = runner.world().unwrap().being_ids().into_iter().collect();
    // The dawn cells the migration reader measures dispersal against (one per founding band).
    let dawn = dawn_cells(&runner);
    println!("  dawn seeded {} founders\n", founders.len());

    let total_ticks = cfg.generations * GEN_TICKS;
    let snapshot_every = (cfg.generations / 10).max(1);

    let start = Instant::now();
    let mut prev = founders;
    for gen in 1..=cfg.generations {
        for _ in 0..GEN_TICKS {
            runner.step();
        }
        // Snapshot at every tenth of the run; the final generation is reported once, by the FINAL
        // block below, so it is not double-printed here.
        if gen % snapshot_every == 0 && gen != cfg.generations {
            prev = snapshot(&format!("SNAPSHOT gen {gen}"), &runner, &cfg, &prev, &dawn);
            println!();
        }
    }
    let elapsed = start.elapsed();

    let _ = snapshot("FINAL", &runner, &cfg, &prev, &dawn);
    println!();
    println!(
        "  ticked {} generations x {} = {} ticks in {:.2}s ({:.1} ms/generation)",
        cfg.generations,
        GEN_TICKS,
        total_ticks,
        elapsed.as_secs_f64(),
        elapsed.as_secs_f64() * 1000.0 / cfg.generations as f64,
    );
    println!("  final state_hash: {:032x}", runner.state_hash());
    println!("  (same arguments reproduce this hash: Principle 3 determinism)");
}
