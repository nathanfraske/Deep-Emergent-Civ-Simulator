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

//! The production world-build path: assembling a whole [`Runner`] from a resolved scenario and a
//! declared dawn population (design Part 28; the world-wiring handoff, section 4). Increment 1
//! ([`FieldCalib::from_resolution`]) gave the field its medium-derived calibration but had no
//! top-level caller; [`World::seed_dawn_populations`] founds a genome-real population but ran only in
//! tests; and a generated world left its language and drift state uninstalled. This module is the
//! seam that closes all three: [`build_dawn_runner`] takes a resolved scenario, seeds the dawn, arms
//! the language and drift calibrations, derives the field from the selected medium, and hands the
//! composed world to [`Runner::with_world`], so a running world exists that the later increments
//! (reproduction, enculturation, the derived articulation substrate) hang their beats on.
//!
//! A world's declared peoples (its races, their founding bands, and the breeding and personality
//! registries the dawn seeding reads) are an input, not a product of this path: the race-genesis that
//! would grow sentient races from the pre-dawn biosphere (design Part 28) is a later arc, so a caller
//! supplies [`DawnPeoples`]. Everything else (the belief, gossip, orbital, field, and ring
//! calibrations) is read fail-loud from the manifest, so a reserved value refuses to build rather
//! than fabricating a number (Principle 11); the assembly adds no RNG draw and no authored behaviour
//! repertoire, so [`Runner::with_world`]'s Principle-9 steering boundary holds by construction and
//! the composite replays bit for bit (Principle 3).

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId};
use civsim_world::{Coord3, TileMap};

use crate::anatomy::BodyPlanRegistry;
use crate::axiom::RingCapacityLaw;
use crate::breeding::BreedingSystemRegistry;
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::controller::Controller;
use crate::conviction_experience::FeltConvictionCalib;
use crate::decision::Curve;
use crate::discovery::DiscoveryCalib;
use crate::edibility::{Physiology, ToleranceRegistry};
use crate::environ::{EnvironCalib, EnvironFields};
use crate::genesis::LivingWorld;
use crate::homeostasis::{AffordanceRegistry, Homeostasis, HomeostaticRegistry};
use crate::langmod::{
    articulated_geometry, form_system_from_values, phoneme_priors, producible_values,
    PerceptualParams,
};
use crate::language::{
    DriftParams, FeatureDimId, FormSystem, LangId, Language, LanguageParams, ProductionModalityId,
};
use crate::learn::{HarmLearningCalib, RewardLearningCalib};
use crate::locomotion::{LocomotionParams, Walker};
use crate::morphogen::{express_program, grow};
use crate::percept::PerceptRegistry;
use crate::personality::PersonalityRegistry;
use crate::primes::nsm_concept_ids;
use crate::race::{Articulation, BandSpec, Race};
use crate::runner::{
    BeingThermal, EmbodiedPhysiology, Embodiment, Field, FieldCalib, LifecycleKit, LivelinessCalib,
    Runner,
};
use crate::scenario::ScenarioResolution;
use crate::sensorium::SenseChannelId;
use crate::tom::AccessChannelRegistry;
use crate::value::RaceId;
use crate::world::{PlaceId, ReproductionParams, World};

/// The declared peoples of a world at the dawn of sentience (design Part 28): the race records, the
/// founding band placements, and the two registries the dawn seeding reads (the breeding systems a
/// founder's gene-fed sex is expressed through, and the per-race personality profiles the life
/// cadence drifts). A world's peoples are an input to the world-build path rather than a product of
/// it: the race-genesis that would grow these from the pre-dawn biosphere is a later arc, so a caller
/// (a scenario author, a genesis step, or a test) supplies them. The optional mortality hazard arms
/// the mortality half of the life cadence; without it the population ages but is never culled.
#[derive(Clone, Debug, Default)]
pub struct DawnPeoples {
    /// The races the world knows, by id.
    pub races: BTreeMap<RaceId, Race>,
    /// The founding bands, each a race seeded in numbers onto one place.
    pub bands: Vec<BandSpec>,
    /// The breeding-system registry a founder's gene-fed sex is expressed through (design Part 25,
    /// R-REPRO). Install-before-seed: the seeding reads it, so a registry supplied here reaches the
    /// census, and an empty one leaves founders with no recorded sex.
    pub breeding: BreedingSystemRegistry,
    /// The per-race personality registry the life-cadence personality beat drifts (design Part 20,
    /// R-BEING-REP). Install-before-seed: the seeding gives each founder whose race carries a profile
    /// a birth-neutral trait instance, so an empty registry leaves the personality beat inert.
    pub personality: PersonalityRegistry,
    /// The optional life-hazard curve on the raw-age domain; when present it arms the mortality half
    /// of the life cadence, when absent the population only ages. Reserved owner data (a `Curve`),
    /// never a fabricated default: a world without a supplied hazard does not cull.
    pub mortality_hazard: Option<Curve>,
    /// The optional derived-language genesis (increment 2e): when present, the founder step derives
    /// each articulating race's phonetic form system from the 2b-2d pipeline and installs a per-band
    /// lineage, so the naming game and drift run live. When absent, the language calibrations are
    /// armed but inert (no lineage carries a form system), so a world without it seeds and ticks
    /// exactly as before.
    pub language: Option<LanguageGenesis>,
    /// The optional embodiment genesis (real-world unification step 3): when present, the founder step
    /// gives each founder whose race carries a [`crate::race::Race::body`] a located, metabolizing body
    /// sharing its mind's [`StableId`], and the world-build returns one runner carrying both minds and
    /// bodies ([`crate::runner::Runner::with_world_and_embodiment`]). When absent, the world-build
    /// returns a world-only runner exactly as before.
    pub embodiment: Option<EmbodimentGenesis>,
    /// The living biosphere seeded into the run world (the biosphere-into-run arc, `--scenario full`): the
    /// genesis-generated regions, occupants, and species, whose PRODUCER occupants seed the food field so
    /// the founders forage over real located plants rather than a uniform climate number. `None` (the
    /// default and every scenario but `full`) leaves the run byte-identical.
    pub biosphere: Option<LivingWorld>,
}

/// The inputs the founder step embodies the dawn population from (real-world unification step 3): the
/// data-defined registries a body's reserves, affordances, and movement read against, the organ
/// registry a body plan's tissue composition is scored on, the controller hidden width, and the
/// submerged and emergent medium ids the per-cell medium field folds the map into. A founder's own
/// body diverges through its race's
/// [`crate::race::Race::body`] plan and its genome-expressed controller, never a `RaceId` branch
/// (Principle 9); these registries are the shared substrate the divergence is read against. The
/// physiology and thermal-band reserved values are read fail-loud from the manifest at assembly.
#[derive(Clone, Debug)]
pub struct EmbodimentGenesis {
    /// The homeostatic axis registry a body's reserves are sized against (carrying TEMPERATURE).
    pub homeostatic: HomeostaticRegistry,
    /// The morphological affordance registry the body's actions read.
    pub affordances: AffordanceRegistry,
    /// The locomotion parameters the located movement reads.
    pub locomotion: LocomotionParams,
    /// The organ registry a body plan's tissue composition (surface, specific heat, energy density,
    /// respiratory surface) is scored against, the same registry the physiology derivation reads.
    pub organs: BodyPlanRegistry,
    /// The toxin-tolerance registry a founder's (and its descendants') heritable per-toxin-class
    /// tolerance is expressed from the genome through (base-level liveliness step 4): which toxin classes
    /// the world runs (`bio.salinity` and the like), and which gene channel and Hill each carries. Empty
    /// leaves every being with no tolerance (the harm sink stays inert), so a world with no environmental
    /// toxin runs exactly as before.
    pub tolerances: ToleranceRegistry,
    /// The controller hidden width (zero is the reaction-norm controller; a positive width is the
    /// recurrent graduation).
    pub controller_hidden: usize,
    /// The submerged medium id a cell below the reserved submersion elevation holds (for example
    /// `"medium.water"`), the manifest profile the per-cell medium field folds into a water cell.
    pub submerged_medium_id: String,
    /// The emergent medium id a cell at or above the submersion elevation holds (for example
    /// `"medium.air"`), the manifest profile the per-cell medium field folds into a land cell. A
    /// single-medium world passes the same id for both, folding to a uniform field.
    pub emergent_medium_id: String,
}

/// The inputs the founder step derives each race's phonetic form system from (increment 2e): the
/// shared base sound geometry, the medium acoustics, the engine caps, and the reserved thresholds and
/// word-length range. A world's articulating races bend the shared base by their own [`Articulation`]
/// (design Part 33.3), so two races produce different sound inventories from this common genesis
/// through one pipeline with no race id. The base lengths and thresholds are reserved owner data (the
/// `articulation.*` manifest keys), never fabricated; the medium acoustics derive from the world's
/// selected medium, and the caps are engine mechanics.
#[derive(Clone, Debug)]
pub struct LanguageGenesis {
    /// The shared base resonator lengths, one per candidate sound, in feature-value order (reserved
    /// `articulation.base_resonator_lengths`, through [`crate::language::ArticulationSubstrate::phonetic`]).
    pub base_lengths: Vec<Fixed>,
    /// The production modality the derived form system coins in (the phonetic single-modality default
    /// is [`ProductionModalityId`] zero).
    pub modality: ProductionModalityId,
    /// The feature dimension the producible values sit on (the phonetic single-dimension default is
    /// [`FeatureDimId`] zero).
    pub dim: FeatureDimId,
    /// The medium's speed of sound (from [`civsim_physics::laws::speed_of_sound`] over the selected
    /// medium's bulk modulus and density).
    pub sound_speed: Fixed,
    /// The medium's thermoviscous absorption reference beta.
    pub absorption_reference: Fixed,
    /// A typical propagation path over which a contrast blurs.
    pub path: Fixed,
    /// The perceptual-geometry engine caps and mode count.
    pub perceptual: PerceptualParams,
    /// The channel-wide production-and-perception capability, broadcast across the candidate values as
    /// the phoneme-prior gate. At the dawn a founding race's whole voice channel carries one
    /// capability; a per-value producibility model is a follow-on.
    pub capability: Fixed,
    /// The reserved producibility threshold (`articulation.producibility_threshold`): the prior below
    /// which a value does not enter the inventory.
    pub producibility_threshold: Fixed,
    /// The reserved minimum word length (`articulation.word_min_len`).
    pub word_min_len: u32,
    /// The reserved maximum word length (`articulation.word_max_len`).
    pub word_max_len: u32,
    /// The voice reception channel a race hears speech on.
    pub hearing_channel: SenseChannelId,
}

/// Derive one race's phonetic form system from the shared genesis and the race's own articulation
/// (the 2b-2d pipeline in one place): scale the base geometry by the race's vocal tract and read its
/// hearing resolution ([`articulated_geometry`]), weight the candidate sounds by dispersion and the
/// broadcast capability gate ([`phoneme_priors`]), select the producible set at the reserved threshold
/// ([`producible_values`]), and bridge it to a coinable form system ([`form_system_from_values`]).
/// Fails loud on an empty inventory (a race that can produce no reliable sound gets no fabricated
/// language). Reads no race id: two races diverge only through their [`Articulation`] data.
fn derive_race_form_system(
    genesis: &LanguageGenesis,
    articulation: &Articulation,
) -> Result<FormSystem, crate::langmod::FormSystemError> {
    let geo = articulated_geometry(
        &genesis.base_lengths,
        genesis.sound_speed,
        genesis.absorption_reference,
        genesis.path,
        articulation,
        genesis.hearing_channel,
        genesis.perceptual,
    )
    .ok_or(crate::langmod::FormSystemError::EmptyInventory)?;
    let gate = vec![genesis.capability; genesis.base_lengths.len()];
    let priors = phoneme_priors(&geo, &gate);
    let values = producible_values(&priors, genesis.producibility_threshold);
    form_system_from_values(
        genesis.modality,
        genesis.dim,
        &values,
        genesis.word_min_len,
        genesis.word_max_len,
    )
}

/// Arm the derived per-race languages at the founder step (increment 2e): for each founding band, in
/// band order, derive its race's phonetic form system once (cached per race) and install a per-band
/// lineage carrying it, then assign the band's founders to that lineage. Bands of one race share the
/// form system and the drift cadence but are separate lineages, so separated bands coin and converge
/// their own words and then drift apart on their race's cadence. A band whose race declares no
/// articulation is skipped (fail-quiet), so a race with no phonetics simply has no lineage. Reads no
/// race id in the derivation: the divergence is the per-race articulation data (Principle 9). Returns
/// the [`FormSystemError`](crate::langmod::FormSystemError) of the first race whose inventory is empty,
/// so a mis-calibrated threshold refuses rather than installing a silent language.
pub fn arm_dawn_languages(
    world: &mut World,
    founders_by_band: &[(RaceId, Vec<StableId>)],
    races: &BTreeMap<RaceId, Race>,
    genesis: &LanguageGenesis,
) -> Result<(), crate::langmod::FormSystemError> {
    let mut race_form: BTreeMap<RaceId, FormSystem> = BTreeMap::new();
    for (band_index, (race_id, members)) in founders_by_band.iter().enumerate() {
        let Some(race) = races.get(race_id) else {
            continue;
        };
        let Some(articulation) = race.articulation.as_ref() else {
            continue;
        };
        // Derive the race's form system once and reuse it for every band of that race (a Clone, so
        // the pipeline runs once per race, not once per band).
        let form_system = match race_form.get(race_id) {
            Some(fs) => fs.clone(),
            None => {
                let fs = derive_race_form_system(genesis, articulation)?;
                race_form.insert(*race_id, fs.clone());
                fs
            }
        };
        let lang = LangId(band_index as u32);
        world.add_language(Language::new(lang, *race_id, form_system));
        for &member in members {
            world.set_language_of(member, lang);
        }
    }
    Ok(())
}

/// Assemble a running [`Runner`] from a resolved scenario and a declared dawn population (design Part
/// 28; the world-wiring handoff, section 4). The build, in order: construct the [`World`] from the
/// manifest (its life cadence derived from the world's orbit, fail-loud on a reserved orbit or base
/// tick); install the breeding and personality registries the seeding reads; seed the dawn population
/// through [`World::seed_dawn_populations`], sizing each founder's evidence ring from its expressed
/// memory through the manifest [`RingCapacityLaw`]; arm the language and drift calibrations (the
/// concept set is the NSM semantic primes, the innovation and sound-change rates read fail-loud from
/// the manifest); optionally arm the mortality hazard; derive the field from the scenario's selected
/// medium ([`FieldCalib::from_resolution`], so a world of air and a world of water conduct heat at
/// their own physics rate); and compose the world onto the field runner through
/// [`Runner::with_world`].
///
/// Fail-loud throughout (Principle 11): a reserved belief, gossip, orbit, ring, medium, or field
/// calibration refuses to build rather than fabricating a number, so under [`Profile::Calibrated`]
/// this cannot run until the owner has set the values it needs, and under [`Profile::Development`] a
/// fixtures profile supplies labelled placeholders. Determinism-clean (Principle 3): the world is
/// seeded on `seed` before any minting, the dawn's draws are counter-keyed on the minted ids, and the
/// field runner adds no RNG phase, so two identical builds replay bit for bit. Steering-clean
/// (Principle 9): the assembly installs no authored decision repertoire, so the composed world's
/// behaviour source stays the evolved controller and [`Runner::with_world`]'s refusal boundary holds
/// by construction.
///
/// The language and drift calibrations are always armed here. When [`DawnPeoples::language`] carries a
/// [`LanguageGenesis`], the founder step also derives each articulating race's phonetic form system
/// from its own [`Articulation`] through the shared pipeline and installs a per-band lineage
/// ([`arm_dawn_languages`]), so the naming game and drift run live: bands coin and converge words from
/// their race's producible sounds, separated bands diverge, and two races with different articulation
/// speak observably different languages. Without a genesis the calibrations stay inert (no lineage
/// carries a form system), so a world that declares no phonetics seeds and ticks exactly as before.
pub fn build_dawn_runner(
    manifest: &CalibrationManifest,
    channels: &AccessChannelRegistry,
    profile: Profile,
    resolution: &ScenarioResolution,
    map: &TileMap,
    peoples: &DawnPeoples,
    seed: u64,
) -> Result<Runner, CalibrationError> {
    // The world, its life cadence derived from the manifest orbit (fail-loud on a reserved orbit or
    // base tick). Seeded before any minting so the dawn draws are reproducible from the seed alone.
    let mut world = World::from_manifest(manifest, channels, profile)?.with_seed(seed);

    // The registries the dawn seeding reads must be installed BEFORE it runs, or founders silently
    // miss their census sex and personality instance (both fail-quiet by design).
    world.set_breeding_systems(peoples.breeding.clone());
    world.set_personality_registry(peoples.personality.clone());

    // Seed the dawn: mint founders, promote genomes from each race's pool, express minds, size each
    // evidence ring from expressed memory through the manifest ring law, seed intrinsic beliefs,
    // place each band, and record the race registry the drift cadence later reads.
    let ring_law = RingCapacityLaw::from_manifest(manifest)?;
    let founders = world.seed_dawn_populations(&peoples.races, &peoples.bands, &ring_law);

    // Arm the language and drift calibrations. The concept set is the NSM semantic primes (the anchor
    // meanings a band coordinates words for first); the innovation and sound-change rates are
    // fail-loud manifest reads.
    world.set_concepts(nsm_concept_ids());
    world.set_language(LanguageParams::from_manifest(manifest)?);
    world.set_drift(DriftParams::from_manifest(manifest)?);

    // Arm reproduction and post-dawn generational drift (real-world unification, step 1): a mature,
    // compatible pair bears one child per reproductive cadence, and each generation each race's pool
    // drifts under the effective size its own reproductive census implies (census-derived Ne, retiring
    // audit deviation 23 for the post-dawn tier). The mutation spread is the surfaced reserved value,
    // never fabricated inline; the ring law is already built above. Both are inert until the first
    // life cadence fires (the founders are seeded at age zero, so no pair is mature at the dawn), so a
    // short run behaves exactly as before while a multi-generation run grows and drifts.
    world.set_reproduction(ReproductionParams::from_manifest(manifest)?);
    world.arm_generational_drift();

    // Arm the belief-diffusion movers (base-level liveliness step 5), fail-loud from the manifest: the
    // per-generation enculturation of each band's intrinsic axiom stances (`set_stubbornness_split`, the
    // conserved split between a being's own conviction and its band's mean) and the aggregate belief-pool
    // diffusion rate (`set_belief_diffusion_rate`). Enculturation drifts the seeded intrinsic axioms on
    // the life cadence; the pool diffusion is inert until a belief pool is seeded (no dawn pool is, so it
    // is an armed no-op this arc, the honest limit). The gossip spread the movement coupling drives reads
    // `Mind.beliefs` directly and needs neither.
    world.set_stubbornness_split(manifest.require_fixed("belief.enculturation_stubbornness")?);
    world.set_belief_diffusion_rate(manifest.require_fixed("belief.diffusion_rate")?);

    // Install the modelled-dialogue substrate (base-level liveliness promotion policy), so a being the
    // runner promotes into a narrative arc converses move-by-move rather than being silenced (a promoted
    // being is skipped by the aggregate gossip fallback, so without a dialogue substrate it would neither
    // gossip nor converse). The substrate is the labelled dev fixture (design Part 9.5; a canonical
    // substrate is owner data); its content gate fails loud on a malformed load. Without a promoted set
    // the dialogue step stays a no-op, so this is inert until the runner's promotion policy lifts a being.
    let (force_floor, move_registry) = crate::dialogue::dev_substrate();
    world
        .set_dialogue(move_registry, force_floor)
        .map_err(|e| CalibrationError::BadValue {
            id: "dialogue.substrate".to_string(),
            detail: format!("the dev dialogue substrate failed its content gate: {e:?}"),
        })?;

    // Arm the derived per-race languages at the founder step (increment 2e), if a language genesis is
    // supplied: derive each articulating race's phonetic form system from the base geometry and its
    // own articulation, install a per-band lineage, and assign the band's founders, so the naming game
    // and drift run live. Without a genesis the calibrations above stay inert (no lineage). The
    // founders are returned in band order, skipping any band whose race is not registered, so the
    // per-band grouping mirrors that skip.
    if let Some(genesis) = &peoples.language {
        let founders_by_band = group_founders_by_band(&founders, &peoples.bands, &peoples.races);
        arm_dawn_languages(&mut world, &founders_by_band, &peoples.races, genesis).map_err(
            |e| CalibrationError::BadValue {
                id: "articulation.producibility_threshold".to_string(),
                detail: format!(
                    "the founder step could not derive a race's phonetic form system: {e:?} (the \
                     producibility threshold or the base geometry leaves a race with no producible \
                     sound)"
                ),
            },
        )?;
    }

    // Optionally arm the mortality half of the life cadence; without a hazard the population ages but
    // is never culled.
    if let Some(hazard) = &peoples.mortality_hazard {
        world.set_mortality_hazard(hazard.clone());
    }

    // Seed the temperature field from the map, reconciling the worldgen's NORMALISED [0,1] temperature
    // axis to the ABSOLUTE temperature the physics reads (derive-vs-author, Principle 6): the metabolism
    // and hydrology laws need Kelvin (Stefan-Boltzmann's T^4), so the world's mean surface temperature and
    // its full equator-to-pole span are read fail-loud from the manifest and mapped in, rather than
    // leaving a Kelvin-labelled [0,1] field. The dev fixtures carry the IDENTITY (mean 1/2, range 1), so a
    // Development build keeps the raw normalised field byte-for-byte; a calibrated world reads the owner's
    // Kelvin climate data.
    let temp_mean = manifest.require_fixed("climate.mean_surface_temperature")?;
    let temp_range = manifest.require_fixed("climate.latitude_temperature_range")?;
    let field = Field::from_map_absolute(map, temp_mean, temp_range);
    // Derive the field diffusion from the scenario's selected medium (increment 1, WP2): the diffusion
    // coefficient is the medium's k/(rho*c) over the cell and tick, so a world of air and a world of
    // water conduct heat at their own physics rate rather than a free scalar.
    let calib = FieldCalib::from_resolution(manifest, resolution)?;

    // Compose the armed world onto the field runner. Both constructors refuse a world carrying an
    // authored decision repertoire (Principle 9); this path installs none, so the boundary holds. With
    // an embodiment genesis, assemble a located body sharing each founder's mind id and return one
    // runner carrying both minds and bodies (real-world unification step 3); without, a world-only
    // runner exactly as before.
    let mut runner = match &peoples.embodiment {
        Some(genesis) => {
            let (embodiment, kit) =
                assemble_dawn_embodiment(&world, map, peoples, &founders, genesis, manifest, seed)?;
            let mut runner = Runner::with_world_and_embodiment(field, calib, world, embodiment);
            // Arm the lifecycle pairing (real-world unification, step 3c): a World birth now mints a
            // paired body and a death retires it, so a multi-generation embodied run keeps minds and
            // bodies in lockstep. Without arming, the runner ticks minds and bodies but never embodies
            // a newborn (the pre-3c behaviour).
            runner.arm_lifecycle(kit);
            runner
        }
        None => Runner::with_world(field, calib, world),
    };

    // Arm the environmental field stack (base-level liveliness step 2): hydrology and primary
    // productivity advance each tick after the temperature field, and the standing producer biomass is
    // written into the embodiment's resource field so the grazers have supply. Biosphere-ready: the
    // productivity is the default abstract source of the per-cell producer biomass, which the
    // living-biosphere addendum replaces with real producer occupants. Fail-loud on a reserved forcing
    // constant (Principle 11).
    // Seed the run's food field from the living biosphere's PRODUCER occupants when armed (the
    // biosphere-into-run arc): where a real producer stands, its located biomass is the food capacity, so
    // the founders graze real plants rather than a uniform climate number. The reconstitution scalar is
    // `Fixed::ONE`, the epoch's own `pop_capacity` (reserved; it must match the genesis epoch's
    // `pop_capacity` so the run biomass matches what genesis radiated). `None` leaves the producer field
    // all-zero and the run byte-identical.
    let mut environ = EnvironFields::from_map(map);
    if let Some(living) = &peoples.biosphere {
        environ.set_producer(&living.producer_biomass(civsim_core::Fixed::ONE));
        environ.set_producer_source(&living.producer_sources());
        // Arm the run's abiotic-source registry from the SAME object the biosphere was generated against
        // (Arc 5 T1), so the extract-deplete cycle resolves each producer's evolved source id against the
        // world's own bindings rather than a hand-written literal that only agreed with generation by
        // comment convention. Closes the confirmed generation-to-run gap: the canonical build path never
        // armed this registry before, only the run_world example did.
        runner.set_abiotic_sources(living.abiotic.clone());
        // T3 (standing food as the producer's OWN composition) MECHANISM is built and proven
        // (`environ.set_producer_food` carries the plant's REAL per-axis composition magnitudes, CORRECTED-T3, no
        // longer a sum-to-one simplex; the base-liveliness INGEST supersedes the `food_energy_density` anchor per
        // cell where a real composition stands, so food value is the plant's own `bio.energy_density`). Wiring it
        // in the canonical path here stays OWNER-GATED, now on the biosphere-BALANCE calibration rather than a
        // missing mechanism: the real `bio.energy_density` scale (kJ/g, [0, 38]) is far below the placeholder
        // `food_energy_density` (3000) the reserve/Kleiber-drain was calibrated against, so a grazer starves until
        // the owner recalibrates the reserve-drain scale to the real food scale (measured in `--scenario living`:
        // it collapses by starvation; surfaced, never tuned, as no autonomous value may author the survival
        // outcome). Left unwired so the canonical demo stays byte-identical and alive; the call to arm it is
        // `environ.set_producer_food(&living.producer_compositions())`.
    }
    runner.set_environ(environ, EnvironCalib::from_manifest(manifest)?);
    // Arm the base-level liveliness surfacing policy (the arc-promotion magnitudes), fail-loud from the
    // manifest (Principle 11): the values that gate and weight the run-path story hooks are
    // reserved-with-basis, not hardcoded inline constants.
    runner.set_liveliness(LivelinessCalib::from_manifest(manifest)?);
    // Arm the experiential associative learner's calibrations (harm-learning arc slice b) the same way,
    // so a canonical run reads the harm-noise floor, feature granularity, and harm likelihoods fail-loud
    // rather than the dev fixture. Harm-learning is always armed (inert without a declared toxin percept),
    // so it needs no feature flag.
    runner.set_harm_learning(HarmLearningCalib::from_manifest(manifest)?);

    // Arm the OPT-IN learners the scenario declares (the loader arc, gap b), each fail-loud from the manifest
    // (Principle 11): a scenario that declares a feature `true` arms its learner from the reserved values (so
    // under Calibrated it refuses until the owner has set them, which maps the remaining calibration work);
    // a scenario that declares none arms nothing here and its world is byte-identical to the pre-loader build.
    // Each is gated on its own flag, so which learners a world runs is scenario DATA rather than a hardcoded
    // opt-in in the run harness. Reward and discovery normally travel together (the discovery sampler biases
    // by reward beliefs), but each is gated independently so a world can arm the reward learner alone.
    if resolution.features.reward {
        runner.set_reward_learning(RewardLearningCalib::from_manifest(manifest)?);
    }
    if resolution.features.discovery {
        runner.set_discovery(DiscoveryCalib::from_manifest(manifest)?);
    }
    if resolution.features.convictions {
        runner.set_felt_conviction_learning(FeltConvictionCalib::from_manifest(manifest)?);
    }
    // Tool affordances are an embodiment property, so they arm only when the world built an embodiment; a
    // disembodied world declaring the flag simply has nothing to arm it on.
    if resolution.features.tools {
        if let Some(emb) = runner.embodiment_mut() {
            emb.set_tool_affordances(true);
        }
    }
    Ok(runner)
}

/// Group the returned founders by their band, in band order, skipping any band whose race was not
/// registered (so no ids were minted for it): the founders are a band-order concatenation, so this
/// re-splits them exactly as the seeding minted them (design Part 28). Shared by the language founder
/// step and the embodiment founder step.
fn group_founders_by_band(
    founders: &[StableId],
    bands: &[BandSpec],
    races: &BTreeMap<RaceId, Race>,
) -> Vec<(RaceId, Vec<StableId>)> {
    let mut grouped = Vec::new();
    let mut cursor = 0usize;
    for band in bands {
        if !races.contains_key(&band.race) {
            continue;
        }
        let end = (cursor + band.members).min(founders.len());
        grouped.push((band.race, founders[cursor..end].to_vec()));
        cursor = end;
    }
    grouped
}

/// Assemble the dawn embodiment (real-world unification step 3): give each founder whose race carries a
/// body plan a located, metabolizing body that reuses the founder's mind [`StableId`] (never a second
/// registry mint), so the being is at once a `World` mind and an `Embodiment` walker. A race with no
/// body plan founds minds without bodies (the disembodied case). The body's homeostatic reserves derive
/// from its race's plan and the shared organ registry, its controller from its own genome (else a blank
/// reaction norm), and its comfort band from the reserved thermal set point and half-band, so two races
/// diverge in their bodies from their plan and genome alone, never a `RaceId` branch (Principle 9). The
/// bodies respire the per-cell medium field folded from the worldgen map (`medium.water` below the
/// reserved submersion elevation, `medium.air` above), and the physiology anchors are the fail-loud
/// manifest reads.
#[allow(clippy::too_many_arguments)]
fn assemble_dawn_embodiment(
    world: &World,
    map: &TileMap,
    peoples: &DawnPeoples,
    founders: &[StableId],
    genesis: &EmbodimentGenesis,
    manifest: &CalibrationManifest,
    seed: u64,
) -> Result<(Embodiment, LifecycleKit), CalibrationError> {
    // The reserved comfort band every founder's thermoregulation reads (fail-loud while reserved). The
    // spawn temperature is the set point (a founder is born at its comfort centre; physical state, not
    // a reserved value). The same band is the lifecycle kit's template, so a newborn is born into it too.
    let setpoint = manifest.require_fixed("physiology.thermal_setpoint")?;
    let half_band = manifest.require_fixed("physiology.thermal_half_band")?;
    let thermal = BeingThermal {
        setpoint,
        half_band,
        initial_temp: setpoint,
    };
    // The map extent a band's spawn coordinate is placed within. Coord3 is authoritative and each
    // being's PlaceId stays frozen at its dawn band; a habitability-filtered placement rides step 4's
    // reserved submersion threshold (a documented coupling), so this arc places each band at a
    // deterministic in-bounds cell.
    let topo = map.topo();
    let (mw, mh) = (topo.width.max(1), topo.height.max(1));
    let mut emb = Embodiment::new(
        genesis.homeostatic.clone(),
        genesis.affordances.clone(),
        genesis.locomotion,
        genesis.controller_hidden,
        seed,
    );
    // Declare the perceived-feature registry from the world's harm-relevant toxin classes (harm-learning
    // arc slice b): a being senses the substances its physiology responds to, so it can correlate felt
    // harm with the ground it stands on and form the belief for itself (retiring the injected hazard
    // Observe). This MUST precede the layout clone below, because it rebuilds the controller layout to
    // carry the feature block: the founder controllers are expressed against `layout`, so a layout cloned
    // before this call would express them at the wrong width (their bias and forage weights would land on
    // the pre-feature indices, and a founder would read a feature slot as its move bias and never forage).
    // A world with no declared toxins declares no percepts, so the layout is unchanged and the run is
    // byte-identical.
    emb.set_percepts(PerceptRegistry::from_tolerances(&genesis.tolerances));
    let layout = emb.layout().clone();
    // The per-band spawn map the lifecycle pairing reads to place a newborn at its band's frozen dawn
    // site (real-world unification, step 3c). The grouping filters bands whose race is registered in the
    // same order as this filtered band list, so the two align by index.
    let grouped = group_founders_by_band(founders, &peoples.bands, &peoples.races);
    let filtered_bands: Vec<&BandSpec> = peoples
        .bands
        .iter()
        .filter(|b| peoples.races.contains_key(&b.race))
        .collect();
    let mut spawn_by_place: BTreeMap<PlaceId, Coord3> = BTreeMap::new();
    for (band_index, (race_id, ids)) in grouped.iter().enumerate() {
        let Some(race) = peoples.races.get(race_id) else {
            continue;
        };
        // A race founds embodied members if it declares a catalog body OR a developmental program: a fully
        // grown race (a morphogen program and no catalog body) sources its metabolism from its grown tissue
        // and needs no catalog body (emergent-anatomy Step 3, the metabolic-tier grow).
        if race.body.is_none() && race.morphogen.is_none() {
            continue; // a race with neither founds minds without bodies
        }
        let coord = Coord3::ground((band_index as i32 * 5) % mw, (band_index as i32 * 3) % mh);
        // Record this band's spawn coordinate under its PlaceId, so a newborn of the band (whose place
        // it inherits from its parents) spawns at the same site.
        if let Some(band) = filtered_bands.get(band_index) {
            spawn_by_place.insert(band.place, coord);
        }
        for &id in ids {
            // Grow this member's body from its OWN genome, if the race carries a program (emergent-anatomy
            // Step 2). The run reads the grown segments' physics for affordances and ground speed.
            let structure = match (&race.morphogen, world.genome_of(id)) {
                (Some(program), Some(genome)) => {
                    let params = express_program(program, &race.genes, genome);
                    Some(grow(program, &params, seed, id))
                }
                _ => None,
            };
            // The LOD-0 metabolic body and the reserves. A race that declares a catalog body keeps it as the
            // metabolic aggregate and sources its reserves from its catalog organs, whether or not it also
            // grows a run structure (the B2b hybrid), so those scenarios are byte-identical. A FULLY GROWN
            // race (no catalog body) sources both from its grown structure: the body is the digest and the
            // reserves are summed off the grown tissue directly (emergent-anatomy Step 3, the metabolic-tier
            // grow), so the catalog metabolic body is retired and a grown race needs no catalog body at all.
            let (body, homeostasis) = if let Some(plan) = &race.body {
                (
                    plan.clone(),
                    Homeostasis::new(&genesis.homeostatic, plan, &genesis.organs),
                )
            } else if let Some(s) = &structure {
                (
                    s.digest(),
                    Homeostasis::from_structure(&genesis.homeostatic, s),
                )
            } else {
                continue; // a grown race whose founder has no genome yet: cannot embody this member
            };
            let controller = match world.genome_of(id) {
                Some(genome) => Controller::express(&race.genes, genome, &layout),
                None => Controller::zeros(&layout),
            };
            // The consumer physiology: the nutrient requirements from the registry (the dev fixture)
            // PLUS the heritable per-toxin-class tolerance expressed from the founder's genome through the
            // tolerance registry (base-level liveliness step 4), so a founder carries its own salt (or
            // dust) resistance and a lineage adapts to a gradient by selection. A founder with no genome
            // falls back to the tolerance-free dev fixture.
            let physiology = match world.genome_of(id) {
                Some(genome) => Physiology::express(
                    &genesis.homeostatic,
                    &genesis.tolerances,
                    &race.genes,
                    genome,
                ),
                None => Physiology::dev_for_registry(&genesis.homeostatic),
            };
            let mut walker = Walker::new(id, coord, body, homeostasis, physiology, controller);
            // The founder's heritable EXPLORATION and DELIBERATION propensities, expressed from its own
            // genome through the two unit evolve-channels (the ideation activation). A founder carries the
            // channels' unseeded loci, so it expresses to zero (founder-zero) and the propensities stay
            // dormant until mutation drifts a descendant off zero and the discovery loop is armed; a race
            // whose pool never carried the channels expresses zero too (the express is inert unseeded).
            if let Some(genome) = world.genome_of(id) {
                walker.exploration = race
                    .genes
                    .express_unit(genome, crate::genome::Channel::Exploration);
                walker.deliberation = race
                    .genes
                    .express_unit(genome, crate::genome::Channel::Deliberation);
                walker.social_learning = race
                    .genes
                    .express_unit(genome, crate::genome::Channel::SocialLearning);
            }
            if let Some(s) = structure {
                walker = walker.with_structure(s);
            }
            emb.add(walker, thermal);
        }
    }
    // Arm the tolerance registry on the embodiment so the lifecycle pairing expresses a newborn's
    // heritable tolerance from its own genome the same way (base-level liveliness step 4).
    emb.set_tolerances(genesis.tolerances.clone());
    // Install the world's organ registry so an affordance and the ground speed are derived against the
    // same kinds the physiology reads (emergent-anatomy step one), not the labelled dev fixture.
    emb.set_organs(genesis.organs.clone());
    emb.set_physiology(EmbodiedPhysiology::from_manifest(
        manifest,
        genesis.organs.clone(),
        map,
        &genesis.submerged_medium_id,
        &genesis.emergent_medium_id,
    )?);
    Ok((emb, LifecycleKit::new(thermal, spawn_by_place)))
}
