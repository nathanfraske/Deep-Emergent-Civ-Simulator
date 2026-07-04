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
use civsim_world::TileMap;

use crate::axiom::RingCapacityLaw;
use crate::breeding::BreedingSystemRegistry;
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::decision::Curve;
use crate::langmod::{
    articulated_geometry, form_system_from_values, phoneme_priors, producible_values,
    PerceptualParams,
};
use crate::language::{
    DriftParams, FeatureDimId, FormSystem, LangId, Language, LanguageParams, ProductionModalityId,
};
use crate::personality::PersonalityRegistry;
use crate::primes::nsm_concept_ids;
use crate::race::{Articulation, BandSpec, Race};
use crate::runner::{Field, FieldCalib, Runner};
use crate::scenario::ScenarioResolution;
use crate::sensorium::SenseChannelId;
use crate::tom::AccessChannelRegistry;
use crate::value::RaceId;
use crate::world::World;

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

    // Arm the derived per-race languages at the founder step (increment 2e), if a language genesis is
    // supplied: derive each articulating race's phonetic form system from the base geometry and its
    // own articulation, install a per-band lineage, and assign the band's founders, so the naming game
    // and drift run live. Without a genesis the calibrations above stay inert (no lineage). The
    // founders are returned in band order, skipping any band whose race is not registered, so the
    // per-band grouping mirrors that skip.
    if let Some(genesis) = &peoples.language {
        let mut founders_by_band: Vec<(RaceId, Vec<StableId>)> = Vec::new();
        let mut cursor = 0usize;
        for band in &peoples.bands {
            if !peoples.races.contains_key(&band.race) {
                continue; // seed_dawn_populations skipped this band, so no ids were minted for it
            }
            let end = (cursor + band.members).min(founders.len());
            founders_by_band.push((band.race, founders[cursor..end].to_vec()));
            cursor = end;
        }
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

    // Derive the field from the scenario's selected medium (increment 1, WP2): the diffusion
    // coefficient is the medium's k/(rho*c) over the cell and tick, so a world of air and a world of
    // water conduct heat at their own physics rate rather than a free scalar.
    let field = Field::from_map(map);
    let calib = FieldCalib::from_resolution(manifest, resolution)?;

    // Compose the armed world onto the field runner. with_world refuses a world carrying an authored
    // decision repertoire (Principle 9); this path installs none, so the boundary holds.
    Ok(Runner::with_world(field, calib, world))
}
