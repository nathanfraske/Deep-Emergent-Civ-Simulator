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
use civsim_physics::PhysicsRegistry;
use civsim_sim::affordance_percept::{
    AffordancePerceptKind, AffordancePerceptRefs, AffordancePerceptRegistry,
};
use civsim_sim::anatomy::{
    BodyPlan, BodyPlanRegistry, OrganKindDef, Part, Temperament, TissueComposition,
};
use civsim_sim::calibration::{CalibrationManifest, Profile};
use civsim_sim::decompose::DecomposerDriverRegistry;
use civsim_sim::discovery::DiscoveryCalib;
use civsim_sim::edibility::ToleranceRegistry;
use civsim_sim::environ::{AbioticField, AbioticSourceRegistry};
use civsim_sim::genesis::{genesis, GenesisParams};
use civsim_sim::homeostasis::{
    AffordanceRegistry, HomeostaticAxisDef, HomeostaticAxisId, HomeostaticRegistry, CRAFT, DIG,
    ENERGY, EXTRACT, GEOPHAGE, GRASP, RELEASE, SHELTER, TEMPERATURE, WATER,
};
use civsim_sim::langmod::PerceptualParams;
use civsim_sim::language::{ConceptId, FeatureDimId, ProductionModalityId, Word};
use civsim_sim::learn::{RewardLearningCalib, HARMS, HARM_ATTR, REWARDS, REWARD_ATTR};
use civsim_sim::locomotion::LocomotionParams;
use civsim_sim::material::{MaterialField, MatterCycleCalib};
use civsim_sim::percept::PerceptRegistry;
use civsim_sim::physiology::{ENERGY_DENSITY, SALINITY};
use civsim_sim::planning::plan_toward;
use civsim_sim::runner::{ReproductiveVigorCalib, Runner};
use civsim_sim::scenario::{Scenario, ScenarioResolution};
use civsim_sim::sensorium::SenseChannelId;
use civsim_sim::tom::AccessChannelRegistry;
use civsim_sim::{
    append_controller_block, append_scalar_channel, build_dawn_runner, forage_taxis_weights,
    nsm_gloss, Articulation, Axiom, AxiomAxisId, BandSpec, BreedingSystem, BreedingSystemId,
    BreedingSystemRegistry, Channel, CognitionChannel, ControllerLayout, Curve, DawnPeoples,
    Direction, DominanceKind, DominanceMode, EmbodimentGenesis, EpistemicStance, EvidenceRing,
    ForageGains, GeneDef, GeneEffect, GeneId, GenePool, GeneSet, GeneticScheme, IntrinsicBeliefs,
    LanguageGenesis, PersonalityProfile, PersonalityRegistry, Race, RaceId, ReproductionMode,
    SchemeId, SourceModeId, ToleranceAxisId, TraitAxisId, TraitDef, ValueAxisId, ValueProfile,
    World,
};
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, WorldgenParams};

// --- dev-harness scaffolding (documented departures from the pure world-build path) ---

/// The dev-harness life cadence in ticks: how many ticks make one generation here (the manifest
/// cadence of one Earth year in ticks is not tickable in a demonstration, so the world-build tests
/// and this harness override it). Sized so a generation has real room for the cognition beats: the
/// naming game coordinates words over many ticks, and each generation adds a cohort of newborns that
/// start with empty lexicons (the world-build birth does not yet copy a parent's lexicon), so the
/// naming game needs ticks to fold them into their band's consensus before the next cohort arrives.
const GEN_TICKS: u64 = 128;
/// The dev-run world extent (a bounded plane), reported by the migration reader so a reader can tell a
/// small footprint in a large world from a filled one.
const WORLD_W: i32 = 32;
const WORLD_H: i32 = 24;
const WORLD_CELLS: i32 = WORLD_W * WORLD_H;

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
/// The INGEST affordance's activation output index in the dev-default affordance layout (INGEST is the
/// next affordance id after MOVE's three outputs, so its scalar activation is output 3).
const INGEST_OUTPUT: usize = 3;
/// DEV FIXTURE: the founding move-activation bias, set decisively to one so the clamped MOVE activation
/// saturates and a founder wants to move (basis: the activation magnitude at which MOVE beats a resting
/// zero and the being leaves its cell; reserved as `controller.taxis.move_bias`).
const TAXIS_MOVE_BIAS: Fixed = Fixed::ONE;
/// DEV FIXTURE: the founding heading gain on a source-direction and gradient percept, set to one so the
/// MOVE heading follows the unit direction (basis: the heading-follow strength; reserved as
/// `controller.taxis.heading_gain`).
const TAXIS_HEADING_GAIN: Fixed = Fixed::ONE;
/// DEV FIXTURE: the founding suppression of MOVE when a forage source is underfoot, set to one so a
/// being stops on food rather than wandering off it (basis: the here-flag suppression strength; reserved
/// as `controller.taxis.here_suppress`).
const TAXIS_HERE_SUPPRESS: Fixed = Fixed::ONE;
/// DEV FIXTURE: the founding INGEST drive from a forage source underfoot, set to one so a being eats
/// what it stands on (basis: the ingest-activation strength; reserved as `controller.taxis.ingest_drive`).
const TAXIS_INGEST_DRIVE: Fixed = Fixed::ONE;
/// DEV FIXTURE: the founding salinity-tolerance additive effect on the tolerance locus (base-level
/// liveliness step 4), seeded so the pool expresses a moderate salt tolerance with standing spread (basis:
/// the salt tolerance a naive-to-halophile founding pool spans; reserved as `tolerance.salinity_baseline`).
/// Selection near a salt flat and mutation carry the heritable adaptation from here.
const TOLERANCE_SEED_EFFECT: Fixed = Fixed::ONE;
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
/// DEV FIXTURE (viability DEMONSTRATION knob, labelled, non-canonical): the `viability` scenario runs the
/// genome HOTTER so the heritable exploration propensity bootstraps off founder-zero in a watchable span of
/// generations rather than the hundreds pure drift would take (the owner cleared dev values for speed-to-
/// visible). The emergence line holds: the mutation is a mean-zero symmetric step that authors no direction,
/// founders still start at exploration zero, and selection does the lifting. Every other scenario keeps
/// `mutation_rate`/`mutation_step`, so nothing else changes. Basis: the point rate and additive step set so
/// a positive-exploration tail is sampled each generation, wide enough for the seed-hunger squeeze to select
/// on without swamping selection.
fn viability_mutation_rate() -> Fixed {
    Fixed::from_ratio(1, 8)
}
fn viability_mutation_step() -> Fixed {
    Fixed::from_ratio(1, 4)
}
/// DEV FIXTURE: the per-being developmental-environment variance half-width, opened off zero so
/// littermates vary developmentally (basis: the reserved per-race `environment_variance`; small).
fn env_variance() -> Fixed {
    Fixed::from_ratio(1, 20)
}

/// The controller layout the founding forage-taxis block is sized against: the same registries the
/// embodiment genesis installs (the dev-grazer homeostatic axes, energy and water and temperature, and
/// the dev-default affordances, a reaction norm at hidden width zero), AND the same perceived-feature
/// registry the world-build declares from the salinity tolerance (harm-learning arc slice b), so
/// `weight_count` and the taxis weight indices match the controller a founder expresses and reads at run
/// time. Sizing the seed against a layout WITHOUT the feature block would misplace every forage weight
/// (a founder would read a feature slot as its move bias and never forage), so this must carry the same
/// percepts the run does.
fn dawn_layout(cfg: &Config) -> ControllerLayout {
    // The layout MUST match the registries the embodiment genesis installs, or the seeded forage weights
    // land in the wrong slots and the founders never forage. The viability embodiment adds the seed-energy
    // axis and the geophage affordance, so its layout is wider; sizing the forage seed against the default
    // layout there misplaces every weight and starves the population by the second generation.
    let (homeostatic, affordances) = if cfg.viability {
        (viability_homeostatic(), AffordanceRegistry::dev_geophage())
    } else {
        (
            HomeostaticRegistry::dev_grazer(),
            AffordanceRegistry::dev_default(),
        )
    };
    ControllerLayout::with_percepts(
        &homeostatic,
        &affordances,
        &PerceptRegistry::from_tolerances(&ToleranceRegistry::dev_salinity()),
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
    /// Whether the dedicated opt-in `discovery` scenario is selected (`--scenario discovery`): the only
    /// run that ARMS the ideation / experiential-discovery loop (the evolve-channel loci seeded at
    /// genesis, an inert fracturable material field, the affordance-percept registry, and the discovery
    /// and reward learners). Every other run leaves the loop unarmed and unseeded and stays byte-identical.
    discovery: bool,
    /// Whether the dedicated opt-in `viability` scenario is selected (`--scenario viability`): the discovery
    /// loop armed over a world where a discovered action PAYS OFF. It arms the loop exactly as `discovery`
    /// does, but deposits the nutritive `oilseed` (not inert granite), gives the founders a seed-energy
    /// reserve backed by oilseed and a seed-storing tissue, and affords GEOPHAGE, so extracting and eating
    /// the seed feeds a reserve rise the reward learner credits and "extract and eat this seed" can emerge
    /// as a rewarded belief under selection. Every other run stays byte-identical.
    viability: bool,
    /// Whether the `full` scenario is selected (`--scenario full`): the viability world PLUS the held-off
    /// mechanisms armed onto the run-path. The biosphere (a dead being's own flesh becomes located matter
    /// that decomposes back to soil nutrient through the matter cycle), social learning (nutrition learning,
    /// observe-and-imitate, and granular beliefs, so a discovered technique can spread by watching a knower
    /// rather than only by lone re-discovery), and tool affordances. Implies `viability`; every other run
    /// leaves these dormant and byte-identical.
    full: bool,
}

impl Config {
    /// Whether this run ARMS the experiential-discovery loop (the `discovery` or `viability` scenario): the
    /// evolve-channel loci are seeded at genesis, matter is deposited, and the loop and reward learner are
    /// installed. Every other run leaves the loop unarmed and byte-identical.
    fn armed(&self) -> bool {
        self.discovery || self.viability
    }
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
        // `full` is the viability world plus more arming, so it reuses viability's postures.
        let file = if name == "full" {
            "viability"
        } else {
            name.as_str()
        };
        let path = format!("{}/../../scenarios/{file}.toml", env!("CARGO_MANIFEST_DIR"));
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

    // The dedicated `discovery`, `viability`, and `full` scenarios are the opt-ins that arm the ideation
    // loop; every other scenario and the baseline leave it unarmed (byte-identical), so the flags key off
    // the name. `full` is the viability world plus the held-off mechanisms, so it implies `viability`.
    let discovery = scenario.as_deref() == Some("discovery");
    let full = scenario.as_deref() == Some("full");
    let viability = scenario.as_deref() == Some("viability") || full;

    Config {
        seed: seed.unwrap_or(1),
        races: races.or(posture_races).unwrap_or(3).max(1),
        bands: bands.unwrap_or(4).max(1),
        generations: generations.unwrap_or(20).max(1),
        scenario,
        diversity_step,
        pool_ne,
        medium,
        discovery,
        viability,
        full,
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

    // Base-level liveliness step 4: a heritable salinity-tolerance gene (locus 3, Channel::Tolerance axis
    // 0), so a founder expresses its own salt resistance and the pool carries standing variation for
    // selection to act on near a salt flat. Seeded at a balanced frequency with a moderate additive
    // effect, so founders range from naive (they die on a salt flat) to halophile (they live on it) and
    // mutation opens the tail; the expressed magnitude is clamped non-negative in `Physiology::express`.
    genes.push(GeneDef {
        id: GeneId(genes.len() as u32),
        effects: vec![GeneEffect {
            channel: Channel::Tolerance(ToleranceAxisId(0)),
            weight: Fixed::ONE,
        }],
        dominance: DominanceMode::additive(),
    });
    freqs.push(Fixed::from_ratio(1, 2));
    effects.push(TOLERANCE_SEED_EFFECT);

    // Base-level liveliness step 3: append the founding controller gene block seeding a FORAGE reaction
    // norm over the dev-grazer registry, so a founder walks toward known food and water, stops on a source
    // to ingest it, and steers along the temperature comfort gradient the runner senses (energy and water
    // are the forage axes, temperature the steer axis; dev-default's MOVE is directional output 0, INGEST
    // scalar output 3). The axis input bases come from the layout, so they follow the registry's data, not
    // a magic constant. The full controller substrate is seeded (a gene per weight), with the taxis
    // magnitudes carried in the pool additive spine; every other weight starts at zero and can mutate on.
    // Reads no race id: the seeds are the same for every race (Principle 9).
    let layout = dawn_layout(cfg);
    let energy_base = layout
        .axis_input_base(ENERGY)
        .expect("the dev-grazer layout carries an energy axis");
    let water_base = layout
        .axis_input_base(WATER)
        .expect("the dev-grazer layout carries a water axis");
    let temp_base = layout
        .axis_input_base(TEMPERATURE)
        .expect("the dev-grazer layout carries a temperature axis");
    let seeds = forage_taxis_weights(
        &layout,
        MOVE_OUTPUT,
        INGEST_OUTPUT,
        &[energy_base, water_base],
        &[temp_base],
        ForageGains {
            move_bias: TAXIS_MOVE_BIAS,
            here_suppress: TAXIS_HERE_SUPPRESS,
            heading_gain: TAXIS_HEADING_GAIN,
            ingest_drive: TAXIS_INGEST_DRIVE,
        },
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
    // Ideation activation slice 2: the ARMED scenarios (`discovery`, `viability`) SEED the two evolve-channel
    // loci at genesis, so a free population evolves exploration and deliberation off founder-zero (the drift
    // reuses the standing additive mutation). Opt-in: ONLY an armed scenario grows the genome, a deliberately
    // different world with its own re-baseline (the B2b opt-in norm); every other run leaves the genome
    // untouched and byte-identical. Founder-zero, so a founder expresses zero and the drive EMERGES by
    // selection, never a coded switch (Principle 9). The birth path expresses these loci onto each walker.
    if cfg.armed() {
        append_scalar_channel(&mut genes, &mut freqs, &mut effects, Channel::Exploration);
        append_scalar_channel(&mut genes, &mut freqs, &mut effects, Channel::Deliberation);
    }
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
    // The `viability` DEMONSTRATION runs the genome hotter (labelled dev knob) so exploration bootstraps in
    // tens of generations; every other scenario keeps the standard dev rates and stays byte-identical.
    let (mut_rate, mut_step) = if cfg.viability {
        (viability_mutation_rate(), viability_mutation_step())
    } else {
        (mutation_rate(), mutation_step())
    };
    let scheme = GeneticScheme {
        id: SchemeId(0),
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: Vec::new(),
        mutation_rate: mut_rate,
        additive_mutation_step: mut_step,
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
    .with_body_plan(if cfg.full {
        diverse_body(index)
    } else if cfg.viability {
        viability_body()
    } else {
        mobile_body()
    })
}

/// Clamp an integer count of tenths into the interior band `[0.3, 0.9]` and return it as a `Fixed`,
/// so an index-varied frequency, stance, or tract scale stays valid however far the divergence step
/// pushes it.
fn clamp_tenths(tenths: i64) -> Fixed {
    Fixed::from_ratio(tenths.clamp(3, 9), 10)
}

/// A mobile development body plan (the grazer fixture), so a founder's walker has an anatomy to derive
/// its physiology and thermoregulate from, and organs that BACK its metabolic reserves: a fat-body (kind
/// 0, energy-dense) and a water-store (kind 2, water-rich) from the dev organ registry, so its energy and
/// water reserve capacities are non-zero (`Homeostasis::new` derives them from organ composition, so an
/// organ-less body would carry no reserves and starve at birth). Labelled fixture, not owner data.
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
        organs: vec![
            Part {
                kind: 0, // fat-body: backs the energy reserve
                development: Fixed::from_ratio(1, 2),
            },
            Part {
                kind: 2, // water-store: backs the water reserve
                development: Fixed::from_ratio(1, 2),
            },
        ],
        temperament: Temperament {
            boldness: Fixed::from_ratio(1, 2),
            exploration: Fixed::from_ratio(1, 2),
            activity: Fixed::from_ratio(3, 4),
            sociability: Fixed::from_ratio(1, 2),
            aggression: Fixed::from_ratio(1, 4),
        },
    }
}

/// The homeostatic axis id for the viability world's SEED-ENERGY reserve (the seed HUNGER), backed by the
/// oilseed substance so eating oilseed refills it. Distinct from the grazer axes (energy 0, water 1,
/// temperature 3, condition 5), a leaf id not special-cased in the mechanism.
const SEED_ENERGY: HomeostaticAxisId = HomeostaticAxisId(10);

/// The homeostatic axis id for the viability world's SEED-CRAVING reserve, also oilseed-backed. This is the
/// felt-REWARD signal, kept distinct from the survival hunger: it drains FAST so it always carries room, so
/// each bite of the seed is a supra-noise RISE the reward learner credits (the record-before-credit path
/// lands it on the geophage that caused it). A being that keeps only the slow hunger reserve topped feels no
/// rise once it is full and forms no belief; the fast craving is what makes eating read as reward even for a
/// well-fed being. Non-lethal (an empty craving is appetite, not starvation). A leaf id.
const SEED_CRAVING: HomeostaticAxisId = HomeostaticAxisId(11);

/// DEV FIXTURE (a viability BALANCE KNOB, surfaced for the owner): the seed-energy reserve's per-tick base
/// drain, the seed HUNGER. Set so the reserve opens room over a handful of ticks, so a being gets hungry
/// between bites and eating-while-hungry is a felt RISE the reward learner credits (the record-before-credit
/// reward path lands that rise on the geophage that caused it), and so a being that never eats empties the
/// reserve over a grace of about two generations and starves. The owner tunes this, the oilseed's nutritive
/// value, and its accessibility to taste; it is set to a sensible viable world, NOT to force the spread.
/// Basis: the grace the ecology needs for a first eater to appear, and a hunger period short enough that
/// eating registers as reward.
fn seed_energy_drain() -> Fixed {
    Fixed::from_ratio(1, 900)
}

/// DEV FIXTURE (the viability demonstration's graded fitness-cost floor): the reproductive vigor a being
/// with a fully-depleted seed-energy reserve keeps, so a chronically seed-hungry being reproduces at this
/// rate rather than at zero (the cost is GRADED, not a hard sterility). Set LOW so an eater that keeps its
/// reserve up strongly out-reproduces a non-eater and selection spreads the eating lineage within tens of
/// generations, but above zero so the founder-zero cohort (which starts seed-full and cannot eat yet) still
/// reproduces while exploration bootstraps. The canonical floor is reserved with its basis (the minimum
/// viable per-generation reproduction rate the demography sustains without the lineage collapsing); this is
/// only the demonstration fixture. The owner tunes it.
fn viability_repro_floor() -> Fixed {
    Fixed::from_ratio(1, 2)
}

/// DEV FIXTURE (a viability BALANCE KNOB): the seed-CRAVING reserve's per-tick base drain, set FAST so the
/// reserve always carries room and each bite is a felt RISE the reward learner credits, so a being that
/// discovers eating forms the "eating pays off" belief within a few generations even when well fed (its slow
/// hunger reserve topped). Basis: fast enough that a bite between the enact gate's intermittent geophages
/// lands as a supra-noise rise, comparable to the sensorium just-noticeable difference.
fn seed_craving_drain() -> Fixed {
    Fixed::from_ratio(1, 64)
}

/// DEV FIXTURE (viability accessibility knob): the modest per-cell oilseed supply the demonstration keeps
/// standing, topped up each generation (a renewable but SCARCE seed crop). Set BELOW the being's per-class
/// requirement so a geophage bite is supply-limited and small: the seed reserve never tops off, so a being
/// stays chronically hungry and each bite is a felt RISE the reward learner credits. A generous hoard
/// instead lets a being pin its reserve full (no room, no rise), so eating reads as nothing and the belief
/// never commits; the modest supply is the food pressure that makes eating pay off. The owner tunes it.
fn viability_oilseed_supply() -> Fixed {
    Fixed::from_int(100)
}

/// The viability world's standing oilseed crop: the modest supply on every ground cell, rebuilt at the top
/// of the run and topped up each generation so the seed is a renewable but scarce food. Labelled fixture.
fn viability_oilseed_field() -> MaterialField {
    let mut material = MaterialField::new();
    for y in 0..WORLD_H {
        for x in 0..WORLD_W {
            material.deposit(Coord3::ground(x, y), "oilseed", viability_oilseed_supply());
        }
    }
    material
}

/// The seed-store organ's kind id in the viability organ registry: the next kind after the dev organs, so
/// the viability body and the viability organ registry agree on it (both derive it from the dev default).
fn seed_store_kind() -> u16 {
    BodyPlanRegistry::dev_default().organs.len() as u16
}

/// The viability world's organ registry: the dev organs plus a SEED-STORE organ whose tissue is composed of
/// the oilseed substance, so the seed-energy reserve (backed by oilseed) has capacity to eat into (a
/// geophage-fed reserve's capacity is summed from tissue carrying its backing axis). Labelled fixture.
fn viability_organs() -> BodyPlanRegistry {
    let mut organs = BodyPlanRegistry::dev_default();
    organs.organs.push(OrganKindDef {
        id: seed_store_kind(),
        name: "seed-store".to_string(),
        fantasy: false,
        composition: TissueComposition::from_pairs(&[("oilseed", Fixed::ONE)]),
    });
    organs
}

/// The viability founder body: the grazer body plus a SMALL SEED-STORE organ, so the founder's seed-energy
/// reserve has capacity (an organ-less reserve would be zero-capacity, so eating oilseed would feed nothing
/// and the loop could not close). The organ is deliberately SMALL so its mass adds a negligible metabolic
/// drain: a half-sized store starves the population by the second generation (the extra mass tips the
/// energy balance), so the seed reserve is a small bonus store, not a body-remaking cost. Labelled fixture.
fn viability_body() -> BodyPlan {
    let mut body = mobile_body();
    body.organs.push(Part {
        kind: seed_store_kind(),
        development: Fixed::from_ratio(1, 20),
    });
    body
}

/// A few DISTINCT founder body plans for the `full` world, cycled by race index, so the peoples differ in
/// ANATOMY and not only in cognition, language, and belief. Each keeps the three reserve-backing organs
/// (fat-body 0, water-store 1, seed-store 2) so its energy, water, and seed-hunger reserves stay non-zero and
/// it is viable; they differ in mass, insulation (covering), organ DEVELOPMENT (so reserve CAPACITY differs),
/// and temperament, so distinct metabolic and behavioural strategies compete over the same scarce food:
/// - index 0, a balanced forager (the viability baseline).
/// - index 1, a HEAVY, well-insulated browser: big mass, thick covering, a large fat-body (a deep energy
///   reserve to ride out scarcity), slow and placid.
/// - index 2, a SMALL, thin-covered, fast and curious forager: little mass, shallow reserves, high activity
///   and exploration, so it works patches quickly and probes more.
///
/// Labelled dev fixtures; each organ's function is still DERIVED from its tissue composition, never a role
/// tag (Principle 8). Opt-in to `full`, so the default and viability determinism pins are unchanged.
fn diverse_body(index: usize) -> BodyPlan {
    let mut body = viability_body();
    match index % 3 {
        0 => body,
        1 => {
            body.body_mass = Fixed::ONE;
            body.covering.development = Fixed::from_ratio(9, 10);
            body.organs[0].development = Fixed::from_ratio(9, 10);
            body.temperament.activity = Fixed::from_ratio(2, 5);
            body.temperament.exploration = Fixed::from_ratio(3, 10);
            body.temperament.aggression = Fixed::from_ratio(1, 5);
            body
        }
        _ => {
            body.body_mass = Fixed::from_ratio(1, 4);
            body.covering.development = Fixed::from_ratio(1, 4);
            body.organs[0].development = Fixed::from_ratio(7, 20);
            body.organs[1].development = Fixed::from_ratio(7, 20);
            body.temperament.activity = Fixed::from_ratio(9, 10);
            body.temperament.exploration = Fixed::from_ratio(4, 5);
            body.temperament.aggression = Fixed::from_ratio(2, 5);
            body
        }
    }
}

/// The viability world's homeostatic registry: the grazer axes plus a SEED-ENERGY reserve BACKED BY the
/// oilseed substance, so eating oilseed lifts it (the felt reward the learner credits). NON-LETHAL (the
/// death floor is out of reach) for the ruling-B demonstration: an empty seed reserve is HUNGER, not death.
/// The seed-hunger couples to REPRODUCTION instead (the graded reproductive-vigor coupling): a being whose
/// seed reserve runs low reproduces LESS, so a being that discovers geophage-on-oilseed keeps its reserve up
/// and OUT-REPRODUCES one that never eats, and the exploration propensity that leads to eating spreads by
/// differential reproduction rather than by a mass-starvation cull. That was the tension of the lethal
/// variant: a lethal seed-hunger crashed the whole founder-zero cohort together, removing the food
/// competition that made the hunger a pressure. The graded reproductive cost keeps the population large and
/// food-pressured while still selecting the eating lineage. Labelled dev fixture; the emergence line holds
/// (a physiology-and-selection gradient, never an authored technique).
fn viability_homeostatic() -> HomeostaticRegistry {
    let mut reg = HomeostaticRegistry::dev_grazer();
    reg.axes.push(HomeostaticAxisDef {
        id: SEED_ENERGY,
        name: "seed-energy".to_string(),
        backing_component: Some("oilseed".to_string()),
        capacity_per_mass: Fixed::ONE,
        base_drain: seed_energy_drain(),
        exertion_drain: Fixed::ZERO,
        // NON-LETHAL: the death floor is out of reach, so an empty seed reserve is HUNGER, not death. The
        // seed-hunger couples to reproduction through the runner's graded reproductive-vigor coupling (keyed
        // on this axis): a being that never eats drains it and reproduces at the floor rate, while one that
        // discovers eating keeps it up and reproduces at full rate, so differential REPRODUCTION selects the
        // exploration propensity that leads to eating (the run path has no fitness scorer). The reproductive
        // cost is graded, so the eating lineage spreads without the mass starvation a lethal hunger caused.
        death_floor: Fixed::from_int(-1000),
    });
    // The seed-CRAVING reserve: also oilseed-backed, so the SAME geophage bite that feeds the survival hunger
    // feeds it, but it drains FAST and never kills. Because it drains fast it always carries room, so each
    // bite is a supra-noise RISE the reward learner credits toward "eating the seed pays off" (the
    // record-before-credit path lands that rise on the geophage that caused it). Without it, a being that
    // eats enough to survive keeps its slow hunger reserve topped, feels no rise, and never forms the belief;
    // the fast craving is what makes eating read as reward even for a well-fed eater. Non-lethal (death_floor
    // out of reach): an empty craving is appetite, not starvation.
    reg.axes.push(HomeostaticAxisDef {
        id: SEED_CRAVING,
        name: "seed-craving".to_string(),
        backing_component: Some("oilseed".to_string()),
        capacity_per_mass: Fixed::ONE,
        base_drain: seed_craving_drain(),
        exertion_drain: Fixed::ZERO,
        death_floor: Fixed::from_int(-1000),
    });
    reg
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
/// fixture (mirrors the world-build test genesis). The VIABILITY scenario swaps in an oilseed-backed
/// seed-energy reserve, the organ registry that stores it, and the geophage affordance so a being can EAT
/// what it extracts; every other run keeps the dev-grazer genesis (byte-identical).
fn embodiment_genesis(cfg: &Config) -> EmbodimentGenesis {
    let (homeostatic, affordances, organs) = if cfg.viability {
        (
            viability_homeostatic(),
            AffordanceRegistry::dev_geophage(),
            viability_organs(),
        )
    } else {
        (
            HomeostaticRegistry::dev_grazer(),
            AffordanceRegistry::dev_default(),
            BodyPlanRegistry::dev_default(),
        )
    };
    EmbodimentGenesis {
        homeostatic,
        affordances,
        locomotion: LocomotionParams::dev_default(),
        organs,
        // The heritable salinity-tolerance class (base-level liveliness step 4), so a founder carries a
        // salt resistance expressed from its genome and a lineage near a salt flat adapts by selection.
        tolerances: ToleranceRegistry::dev_salinity(),
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
        embodiment: Some(embodiment_genesis(cfg)),
        // Seeded at the call site (the biosphere-into-run arc): `--scenario full` grows the living biosphere
        // sized to the run grid; every other scenario leaves it `None` and byte-identical.
        biosphere: None,
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
/// no design ever enters a being's knowledge and this reads zero every snapshot. Reported plainly as
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
fn migration(
    runner: &Runner,
    dawn_cells: &BTreeSet<(i32, i32)>,
) -> Option<(usize, usize, i32, f64)> {
    let emb = runner.embodiment()?;
    let mut occupied: BTreeSet<(i32, i32)> = BTreeSet::new();
    let mut off_dawn = 0usize;
    let mut max_disp = 0i32;
    let mut disp_sum = 0i64;
    let mut n = 0i64;
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
        disp_sum += nearest as i64;
        n += 1;
    }
    // Mean displacement (display only, f64) alongside the max, so a tight clump (mean near max, both
    // small) is distinguishable from a genuine spread (mean well below a large max). The world-cell total
    // it is reported against lets a reader tell a small footprint in a large world from a filled one.
    let mean_disp = if n > 0 {
        disp_sum as f64 / n as f64
    } else {
        0.0
    };
    Some((occupied.len(), off_dawn, max_disp, mean_disp))
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
/// mean productivity capacity (the ceiling the food supply regrows toward). A pure read of hashed state.
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
    let mut capacity_sum = 0.0;
    for y in 0..h {
        for x in 0..w {
            let water = env.water_at(x, y).to_f64_lossy();
            if water > 0.0 {
                wet += 1.0;
            }
            water_sum += water;
            water_max = water_max.max(water);
            capacity_sum += env.capacity_at(x, y).to_f64_lossy();
        }
    }
    Some((wet / n, water_sum / n, water_max, capacity_sum / n))
}

/// The carrying-capacity pressure reader (base-level liveliness step 3): the scarcity that bounds each
/// lineage. Reports the GLOBAL standing food (the grazable `bio.energy_density` stock over the whole
/// map), its occupancy against the productivity capacity, the population the standing food supports per
/// unit, and, crucially, the LOCAL occupancy at the cells beings stand on. The global number is
/// diluted by the empty wilderness (a clustered population grazes only a few of hundreds of cells), so
/// the local occupancy is where the carrying capacity truly bites: it falls as beings graze their own
/// cells down, the scarcity signal that bounds each lineage with no authored cap. A pure read of hashed
/// state; `None` if the runner carries no located resource loop.
fn carrying_capacity(runner: &Runner) -> Option<(f64, f64, f64)> {
    let env = runner.environ()?;
    let emb = runner.embodiment()?;
    let resources = emb.resources();
    let standing = resources.total_supply(ENERGY_DENSITY).to_f64_lossy();
    let (w, h) = env.dims();
    let capacity: f64 = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| env.capacity_at(x, y).to_f64_lossy())
        .sum();
    let occupancy = if capacity > 0.0 {
        standing / capacity
    } else {
        0.0
    };
    // The local pressure: sum the standing food and the capacity over the distinct cells a being stands
    // on. This is where grazing happens, so it shows the scarcity the global average hides.
    let occupied: BTreeSet<(i32, i32)> = emb
        .walkers()
        .iter()
        .map(|wk| {
            let c = wk.coord();
            (c.x, c.y)
        })
        .collect();
    let mut local_standing = 0.0;
    let mut local_capacity = 0.0;
    for &(x, y) in &occupied {
        local_standing += resources
            .supply(Coord3::ground(x, y), ENERGY_DENSITY)
            .to_f64_lossy();
        local_capacity += env.capacity_at(x, y).to_f64_lossy();
    }
    let local_occupancy = if local_capacity > 0.0 {
        local_standing / local_capacity
    } else {
        1.0
    };
    Some((standing, occupancy, local_occupancy))
}

/// The salinity-and-adaptation reader (base-level liveliness step 4): the environmental salt gradient and
/// the population's heritable answer to it. Reports the fraction of cells carrying meaningful salt (the
/// salt flats emerging in endorheic basins), the peak salt mass, and the mean expressed salinity
/// TOLERANCE over the living embodied population (the halophile signal: it rises over generations where a
/// lineage lives near salt, the measured proof that the gradient selects an adaptation rather than
/// excluding a lineage at a fixed dose). A pure read of hashed state; `None` if the runner carries no
/// located population.
fn salinity_state(runner: &Runner) -> Option<(f64, f64, f64)> {
    let env = runner.environ()?;
    let emb = runner.embodiment()?;
    let (w, h) = env.dims();
    let n = (w as f64) * (h as f64);
    if n <= 0.0 {
        return None;
    }
    let mut salty_cells = 0.0;
    let mut salt_max = 0.0f64;
    for y in 0..h {
        for x in 0..w {
            let salt = env.salt_at(x, y).to_f64_lossy();
            if salt > 0.1 {
                salty_cells += 1.0;
            }
            salt_max = salt_max.max(salt);
        }
    }
    let tolerances: Vec<f64> = emb
        .walkers()
        .iter()
        .filter_map(|wk| wk.physiology.tolerance(SALINITY).map(|t| t.to_f64_lossy()))
        .collect();
    let mean_tolerance = if tolerances.is_empty() {
        0.0
    } else {
        tolerances.iter().sum::<f64>() / tolerances.len() as f64
    };
    Some((salty_cells / n, salt_max, mean_tolerance))
}

/// The dynamic belief-spread reader (harm-learning arc slice b): how far a LEARNED "this ground harms
/// me" belief has spread through the population. A being forms this belief for ITSELF when it feels its
/// own condition fall while standing on a salt flat and correlates the felt harm with the salinity it
/// senses underfoot (retiring the injected hazard Observe), then gossip carries the committed belief to
/// whoever shares its live cell, so a migrant that crossed a flat seeds the belief in a band that never
/// did. Reports the count of beings that hold any committed feature-harm belief (a `HARM_ATTR -> HARMS`
/// on a per-feature subject) and the population, so the fraction climbs outward from the flats as the
/// idea rides movement and gossip. This reads the DYNAMIC `Mind.beliefs` inference state (not the
/// intrinsic axiom seed the `band_belief` reader reads). `None` if the runner carries no cognition world.
fn feature_harm_belief_spread(runner: &Runner) -> Option<(usize, usize)> {
    let w = runner.world()?;
    let params = w.belief_params();
    let ids = w.being_ids();
    if ids.is_empty() {
        return None;
    }
    let holders = ids
        .iter()
        .filter(|&&id| {
            w.mind(id).is_some_and(|m| {
                m.committed_beliefs(params)
                    .iter()
                    .any(|b| b.attr == HARM_ATTR && b.value == HARMS)
            })
        })
        .count();
    Some((holders, ids.len()))
}

/// The promoted-set-and-arcs reader (base-level liveliness §4, the generous arc-scoped promotion policy):
/// how many beings are lifted to the individual move-by-move dialogue tier because they are living a
/// narrative arc (a survival struggle, their energy or condition worn low), the rest running the
/// aggregate gossip tier. Reports the promoted count and the population, so the named individuals living
/// their arcs are legible: the count rises when the land presses the population (a lean generation
/// promotes many strugglers) and falls when it is fed. A pure read of hashed state; `None` if the runner
/// carries no cognition world.
fn promoted_arcs(runner: &Runner) -> Option<(usize, usize)> {
    let w = runner.world()?;
    Some((w.promoted_ids().len(), w.population()))
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
    runner: &mut Runner,
    cfg: &Config,
    prev: &BTreeSet<StableId>,
    dawn: &BTreeSet<(i32, i32)>,
) -> BTreeSet<StableId> {
    // Non-canonical observability: drain the cause-of-death log for this window (before any immutable
    // borrow of the runner) and tally it by reserve, so the run reports WHAT killed beings, which the
    // snapshot-diff death count cannot. Aging deaths carry no reserve cause and are the remainder.
    let mut death_cause: BTreeMap<&str, usize> = BTreeMap::new();
    for axis in runner.take_obs_deaths() {
        let cause = match axis.0 {
            0 => "starvation",
            1 => "thirst",
            2 => "incoherence",
            3 => "exposure",
            5 => "wear",
            _ => "other",
        };
        *death_cause.entry(cause).or_default() += 1;
    }
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
    if !death_cause.is_empty() {
        let parts: Vec<String> = death_cause
            .iter()
            .map(|(c, n)| format!("{n} {c}"))
            .collect();
        println!("  cause of death (this window): {}", parts.join(", "));
    }

    // The field-state signal (step 2): the environmental stack's water and productivity.
    match field_state(runner) {
        Some((wet, mean_water, max_water, mean_capacity)) => println!(
            "  field: {:.0}% cells wet  |  mean water {mean_water:.3} (peak {max_water:.3})  |  \
             mean productivity {mean_capacity:.3}",
            wet * 100.0
        ),
        None => println!("  field: no environmental stack"),
    }

    // The carrying-capacity pressure signal (step 3): the standing food, the productivity ceiling it
    // regrows toward, the occupancy (standing over capacity, so a low value is grazing scarcity), and the
    // population the standing food currently feeds. This is the number that shows the population settling
    // where its metabolic draw meets what the land regrows, with no authored cap.
    match carrying_capacity(runner) {
        Some((standing, occupancy, local_occupancy)) => {
            let pop = w.population().max(1);
            println!(
                "  carrying capacity: standing food {standing:.1} (global occupancy {:.0}%)  |  \
                 LOCAL occupancy {:.0}% (grazing pressure {:.0}% where beings graze)  |  {:.2} food/being",
                occupancy * 100.0,
                local_occupancy * 100.0,
                (1.0 - local_occupancy) * 100.0,
                standing / pop as f64,
            );
        }
        None => println!("  carrying capacity: no located resource loop"),
    }

    // The salinity-and-adaptation signal (step 4): the salt gradient and the population's mean heritable
    // salt tolerance (the halophile answer, which rises over generations where a lineage lives near salt).
    match salinity_state(runner) {
        Some((salty, salt_max, mean_tol)) => println!(
            "  salinity: {:.0}% cells salty (peak salt {salt_max:.2})  |  mean salt tolerance {mean_tol:.3} \
             (heritable, selects up near salt)",
            salty * 100.0
        ),
        None => println!("  salinity: no located population"),
    }

    // The belief-spread signal (harm-learning arc slice b): how far a LEARNED "this ground harms me"
    // belief has ridden gossip and migration outward from the beings that formed it for themselves by
    // correlating their own felt harm with the salinity they sensed on a flat.
    match feature_harm_belief_spread(runner) {
        Some((holders, total)) if total > 0 => println!(
            "  belief spread: {holders}/{total} hold a learned feature-harm belief ({:.0}%): a being \
             forms it for itself by correlating its own condition falling with the ground it stands on \
             (no injected observation), then it rides movement-coupled gossip to co-located beings",
            100.0 * holders as f64 / total as f64
        ),
        _ => println!("  belief spread: no cognition world"),
    }

    // The promotion signal (§4): the beings lifted to the individual dialogue tier because they are
    // living a survival arc, the resolution knob on the story turned up on what is already happening.
    match promoted_arcs(runner) {
        Some((promoted, pop)) if pop > 0 => println!(
            "  arcs: {promoted}/{pop} beings promoted to the individual tier (living a survival arc; the \
             aggregate tier carries the rest, generous by default)"
        ),
        _ => println!("  arcs: no cognition world"),
    }

    // The migration signal (step 1): dispersal of the located population from its dawn cells.
    match migration(runner, dawn) {
        Some((cells, off, disp, mean)) => println!(
            "  migration: {cells} of {WORLD_CELLS} world cells occupied (from {} dawn cells)  |  {off} \
             beings off their dawn cell  |  displacement max {disp}, mean {mean:.1}",
            dawn.len()
        ),
        None => println!("  migration: no located population"),
    }

    // The discovery-emergence signal (ideation activation slice 3): what the population is DISCOVERING in an
    // armed scenario (`discovery` or `viability`). Printed only when the loop is armed, so every other run
    // is unchanged. In `viability` the reward-belief count rises off zero as extract-and-eat pays off.
    if cfg.armed() {
        discovery_signal(runner);
    }

    println!("  state_hash: {:032x}", runner.state_hash());
    current
}

/// The discovery-emergence reader (ideation activation slice 3): what the population is DISCOVERING in the
/// armed `discovery` scenario, the ideation analogue of the belief-spread reader. It reports the loop's
/// live signals: how many beings PROPOSE a candidate action off the matter they perceive; how many carry an
/// EXPLORATION or DELIBERATION weight lifted off founder-zero (the evolve-channels rising under selection,
/// the emergent activation); how many ENACT a matter action this tick (a decided affordance that is a
/// matter primitive rather than the controller's own MOVE or INGEST forage); and how many hold a committed
/// REWARDS belief (a learned technique, zero until a discovered action pays off, which is the viability
/// world). A pure read off the embodiment and the minds, folding nothing into `state_hash`; called only in
/// the armed scenario, so every other run is byte-identical.
fn discovery_signal(runner: &Runner) {
    let (Some(emb), Some(world)) = (runner.embodiment(), runner.world()) else {
        println!("  discovery: no embodied cognition world");
        return;
    };
    let walkers = emb.walkers();
    let n = walkers.len();
    if n == 0 {
        println!("  discovery: no living population to run the loop");
        return;
    }
    let params = world.belief_params();
    let proposing = walkers
        .iter()
        .filter(|w| w.proposed_action.is_some())
        .count();
    let exploring = walkers
        .iter()
        .filter(|w| w.exploration > Fixed::ZERO)
        .count();
    let deliberating = walkers
        .iter()
        .filter(|w| w.deliberation > Fixed::ZERO)
        .count();
    // A matter ENACTION: a decided affordance that is a matter primitive (the controller's own forage is
    // MOVE or INGEST), so it came from the exploration or deliberation enact gate acting on the matter the
    // being perceives, the discovery loop closing from percept to act.
    let enacting = walkers
        .iter()
        .filter(|w| {
            matches!(
                w.decided_affordance,
                Some(GRASP | EXTRACT | GEOPHAGE | CRAFT | DIG | RELEASE | SHELTER)
            )
        })
        .count();
    // A being holding a committed REWARDS belief (its planner recalls at least one action it believes pays
    // off). Any positive depth cap detects a held belief; this is an observability read, not the reserved
    // planning depth. Zero until a discovered action pays off (the viability world), and the reader is
    // ready for it.
    let believers = walkers
        .iter()
        .filter(|w| {
            world
                .mind(w.id)
                .is_some_and(|m| !plan_toward(m, REWARD_ATTR, REWARDS, params, 8).is_empty())
        })
        .count();
    println!(
        "  discovery loop: {proposing}/{n} proposing a candidate action  |  evolve-channels off \
         founder-zero: {exploring} exploration, {deliberating} deliberation  |  {enacting} enacting a \
         matter action this tick  |  {believers} hold a reward belief (0 until a discovered action pays \
         off, the viability world)"
    );
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

    // A generated world large enough that the founding bands land on distinct cells. Grid size is
    // overridable for profiling sweeps (CIVSIM_W / CIVSIM_H); it defaults to the fixture size. The founding
    // bands still seed within the WORLD_W x WORLD_H corner, so a larger grid scales the per-cell grid work
    // (field, env, matter cycle) while holding the population fixed, which is the sweep the profile wants.
    let gw = std::env::var("CIVSIM_W")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(WORLD_W);
    let gh = std::env::var("CIVSIM_H")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(WORLD_H);
    let topo = FlatBounded::new(gw, gh, 1);
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(cfg.seed, topo, &biomes, &WorldgenParams::dev_default());

    let mut peoples = assemble_peoples(&cfg);
    // Grow the living biosphere sized to the run grid and seed it into the peoples (the biosphere-into-run
    // arc, `--scenario full`). genesis is a pure function of the seed and builds the SAME map internally
    // (TileMap::generate with cfg.seed over the same grid and dev-default biomes/worldgen), so its regions,
    // occupants, and species land on the run's own coordinate space. Its PRODUCER occupants seed the food
    // field in build_dawn_runner, so the founders forage over real located plants, not a uniform number.
    if cfg.full {
        let living = genesis(
            cfg.seed,
            &GenesisParams {
                width: gw,
                height: gh,
                ..GenesisParams::dev_default()
            },
        );
        let producers = living.producer_biomass(Fixed::ONE);
        println!(
            "  BIOSPHERE SEEDED (opt-in): {} surviving species across {} regions, {} located occupants, of \
             which {} are food-producing plants that seed the food field the founders graze (real located \
             producers, not a uniform number). Grown by genesis from the same seed and grid, so it shares \
             the run's world.",
            living.species(),
            living.regions.len(),
            living.occupants.len(),
            producers.len(),
        );
        peoples.biosphere = Some(living);
    }

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

    // Ideation activation slice 2 + viability arc slice C: the ARMED scenarios (`discovery`, `viability`)
    // ARM the experiential-discovery loop in the free world (opt-in; every other run leaves it unarmed and
    // byte-identical). The arming is ENVIRONMENT plus SENSORY CAPACITY plus the fixed MECHANISM, never
    // authored behaviour: matter on every cell (authored physics, Principle 9) so beings have something to
    // perceive and act on; the affordance-percept registry so they SENSE its fracture-potential; and the
    // discovery and reward learners so a being PROPOSES a candidate action each tick and, as its
    // founder-zero exploration weight drifts off zero under selection, ENACTS one. WHICH action a being
    // enacts still emerges from the exploration weight lifting, never a coded choice.
    //
    // `discovery` deposits INERT granite: breaking it feeds no reserve, so no rewarded technique emerges,
    // correct. `viability` deposits the nutritive `oilseed`, and its founders carry an oilseed-backed
    // seed-energy reserve (the viability embodiment) and afford GEOPHAGE, so extracting-and-eating the seed
    // feeds a reserve RISE the reward learner credits, and "extract and eat this seed" can emerge as a
    // rewarded belief under selection. NO payoff is authored: the energy is the seed's own physics drawn
    // through the runtime edibility laws, and no belief or exploration weight is pre-set. The evolve-channel
    // loci were seeded founder-zero at genesis (full_race) under the same flag.
    if cfg.armed() {
        let material_registry =
            PhysicsRegistry::ground().expect("the embedded ground physics floor loads");
        // `viability` seeds a MODEST renewable oilseed crop (topped up each generation in the run loop), so a
        // being crops it in small bites and stays hungry, the food pressure that makes eating pay off;
        // `discovery` seeds inert granite whose amount is immaterial.
        let material = if cfg.viability {
            viability_oilseed_field()
        } else {
            let mut material = MaterialField::new();
            for y in 0..WORLD_H {
                for x in 0..WORLD_W {
                    material.deposit(Coord3::ground(x, y), "granite", Fixed::from_int(4));
                }
            }
            material
        };
        if let Some(emb) = runner.embodiment_mut() {
            emb.set_material(material);
            emb.set_material_registry(material_registry);
            emb.set_affordance_percepts(
                AffordancePerceptRegistry::from_kinds(&[AffordancePerceptKind::FracturePotential]),
                AffordancePerceptRefs::dev_refs(),
            );
        }
        runner.set_discovery(DiscoveryCalib::dev_default());
        // The `viability` DEMONSTRATION lowers the REWARD-NOISE FLOOR (a labelled dev knob). The default floor
        // is the largest per-tick rise ordinary metabolic recovery produces, so a rise below it is dismissed
        // as recovery noise. But the SEED reserve has no metabolic recovery (it rises ONLY when the being eats
        // the seed), so a small rise is a real reward, not noise; and a being on a gently-draining reserve
        // eats-while-hungry in a small supra-drain step whose rise falls under the default floor, so with the
        // default the reward reads as nothing and the "eating pays off" belief never commits. A low floor lets
        // those real small rises register. It reads the seed's own physics still (a being that never eats
        // feels no rise and forms nothing); it only stops dismissing a small true reward as noise. Every other
        // armed scenario keeps the default reward calib.
        let reward_calib = if cfg.viability {
            RewardLearningCalib {
                reward_noise_floor: Fixed::from_ratio(1, 10_000),
                // A FAST eligibility decay (short TD-lambda), a labelled dev knob: the seed reward is
                // IMMEDIATE (a geophage refills the seed reserve on the tick it is enacted), so with the
                // record-before-credit path the credit lands full-weight on the geophage tick itself; a short
                // decay keeps that just-enacted geophage from accumulating NEUTRAL evidence over the many
                // forage or idle ticks it would otherwise linger in the trace for. So "eating the seed pays
                // off" reads off the eat that paid off, not the whole window around it. The default longer
                // window fits a reward that truly lags its action; the seed reward does not lag.
                eligibility_decay: Fixed::from_ratio(1, 8),
                ..RewardLearningCalib::dev_default()
            }
        } else {
            RewardLearningCalib::dev_default()
        };
        runner.set_reward_learning(reward_calib);
        if cfg.viability {
            // Arm the graded reproductive-vigor coupling (the ruling-B demonstration): a being's per-generation
            // eligibility to pair scales with its SEED-ENERGY reserve, so a being that discovered eating keeps
            // its reserve up and out-reproduces one that never eats, and selection spreads the eating lineage
            // without the mass-starvation crash a lethal seed-hunger caused. Keyed on SEED_ENERGY (the slow
            // survival hunger an eater refills and a non-eater lets drain), floored so the founder-zero cohort
            // still reproduces while exploration bootstraps. A labelled dev fixture; the canonical coupling and
            // its floor are reserved with their basis. The emergence line holds: a physiology-and-selection
            // gradient, not an authored technique.
            runner.set_reproductive_vigor_coupling(ReproductiveVigorCalib {
                axis: SEED_ENERGY,
                floor: viability_repro_floor(),
            });
            println!(
                "  VIABILITY SCENARIO ARMED (opt-in): {} ground cells seeded with fracturable, energy-dense \
                 oilseed; founders carry an oilseed-backed seed-energy reserve and afford geophage; \
                 affordance-percept, discovery, and reward learners installed; evolve-channels seeded \
                 founder-zero. NO payoff authored (the energy is the seed's own physics through the \
                 edibility laws); extract-and-eat can emerge as a rewarded belief under selection. Balance \
                 knobs for the owner: the oilseed's nutritive value (bio.energy_density 25 kJ/g), the \
                 seed-energy drain, and accessibility.\n",
                WORLD_CELLS,
            );
        } else {
            println!(
                "  DISCOVERY SCENARIO ARMED (opt-in): {} ground cells seeded with fracturable granite; \
                 affordance-percept, discovery, and reward learners installed; evolve-channels seeded \
                 founder-zero. No payoff authored (breaking granite feeds no reserve); the `viability` \
                 scenario closes the loop.\n",
                WORLD_CELLS,
            );
        }

        // The `full` scenario arms the mechanisms held off the run-path onto the viability world.
        if cfg.full {
            // Biosphere: a dead being's own flesh becomes located matter (corpse_matter), the matter cycle
            // decomposes that organic matter back to soil nutrient (matter_cycle), and the decomposer drivers
            // set the rate (decomposer). Dev fixtures; the corpse composition is derived per-individual from
            // the body at death, never authored.
            runner.set_matter_cycle(MatterCycleCalib::dev_fixture());
            runner.set_decomposer(DecomposerDriverRegistry::dev_fixture());
            runner.set_corpse_matter(true);
            // Arm the abiotic-source binding registry (the extract-deplete cycle): bind the dev world's
            // evolved source ids to run fields as DATA (0 light -> flux, 1 water -> depletable stock, 2 soil
            // nutrient -> depletable stock of class bio.organic_residue), never an id switch in the hot path;
            // an alien world binds its own sources. The three conversions are labelled dev fixtures, RESERVED
            // with basis (they graduate to the manifest): biomass_per_stock (biomass a unit of drawn stock
            // supports, the reciprocal of the soil fertility_scale), draw_fraction (the nutrient the standing
            // biomass sequesters per tick), weathering_rate (the physical rock-to-nutrient bootstrap seed).
            let mut sources = AbioticSourceRegistry::default();
            sources.insert(0, AbioticField::Flux, "");
            sources.insert(1, AbioticField::WaterStock, "");
            sources.insert(2, AbioticField::SoilStock, "bio.organic_residue");
            sources.biomass_per_stock = Fixed::from_int(4);
            sources.draw_fraction = Fixed::from_ratio(1, 20);
            sources.weathering_rate = Fixed::from_ratio(1, 100);
            runner.set_abiotic_sources(sources);
            // Seed the biosphere CONSUMERS as located body matter (predation + the matter cycle): each animal
            // is its physics-derived body composition, eaten via the geophage by edibility and decomposed back
            // to soil, so the trophic web turns. No minted substance; content-addressed like a corpse.
            if let Some(living) = &peoples.biosphere {
                let bodies = living.consumer_bodies(Fixed::ONE);
                if let Some(emb) = runner.embodiment_mut() {
                    for (coord, composition, volume) in bodies {
                        emb.tissue_mut().deposit(coord, composition, volume);
                    }
                }
            }
            // Social learning + tools (on the embodiment): a discovered technique can spread by watching a
            // knower and by co-located gossip, rather than only by lone re-discovery in each short lifetime,
            // and a worked object can afford an action its raw form does not.
            if let Some(emb) = runner.embodiment_mut() {
                emb.set_nutrition_learning(true);
                emb.set_observe_and_imitate(true);
                emb.set_granular_beliefs(true);
                emb.set_tool_affordances(true);
            }
            println!(
                "  FULL SCENARIO ARMED (opt-in): the viability world PLUS the biosphere (a dead being's own \
                 flesh becomes located matter that decomposes back to soil nutrient), social learning \
                 (nutrition learning, observe-and-imitate, granular beliefs), and tool affordances. A \
                 discovered technique can now spread by watching a knower and outlive its discoverer.\n"
            );
        }
    }

    let founders: BTreeSet<StableId> = runner.world().unwrap().being_ids().into_iter().collect();
    // The dawn cells the migration reader measures dispersal against (one per founding band).
    let dawn = dawn_cells(&runner);
    println!("  dawn seeded {} founders\n", founders.len());

    let total_ticks = cfg.generations * GEN_TICKS;
    let snapshot_every = (cfg.generations / 10).max(1);

    // Harm-learning arc slice b: NO belief is seeded at the dawn. A being forms the "this ground harms
    // me" belief for ITSELF by correlating its own felt condition fall with the salinity it senses on a
    // flat (Runner::couple_conversation, the associative learner), so the belief-spread reader shows an
    // EXPERIENTIALLY-formed idea riding gossip and migration outward rather than an injected discovery.

    // GPU field offload, opt-in behind CIVSIM_GPU_FIELD (and only compiled under --features gpu). Arms the
    // cross-tick stencil pipeline so the field diffusion runs on the device; bit-identical to the CPU path.
    #[cfg(feature = "gpu")]
    if std::env::var("CIVSIM_GPU_FIELD").is_ok() {
        runner.arm_gpu_field(civsim_gpu::cuda_client());
        println!(
            "  GPU FIELD OFFLOAD ARMED (opt-in): the cross-tick field stencil runs on the device."
        );
    }

    let start = Instant::now();
    let mut prev = founders;
    for gen in 1..=cfg.generations {
        for _ in 0..GEN_TICKS {
            runner.step();
        }
        // The viability seed crop is renewable: top the modest oilseed back up on every cell each generation,
        // so the standing supply stays scarce-but-present across the run rather than depleting to nothing.
        // Opt-in; every other scenario leaves the material field untouched.
        if cfg.viability {
            if let Some(emb) = runner.embodiment_mut() {
                emb.set_material(viability_oilseed_field());
            }
        }
        // Snapshot at every tenth of the run; the final generation is reported once, by the FINAL
        // block below, so it is not double-printed here.
        if gen % snapshot_every == 0 && gen != cfg.generations {
            prev = snapshot(
                &format!("SNAPSHOT gen {gen}"),
                &mut runner,
                &cfg,
                &prev,
                &dawn,
            );
            println!();
        }
    }
    let elapsed = start.elapsed();

    let _ = snapshot("FINAL", &mut runner, &cfg, &prev, &dawn);
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
    civsim_sim::profile::report();
}
