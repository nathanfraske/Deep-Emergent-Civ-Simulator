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

use civsim_world::TileMap;

use crate::axiom::RingCapacityLaw;
use crate::breeding::BreedingSystemRegistry;
use crate::calibration::{CalibrationError, CalibrationManifest, Profile};
use crate::decision::Curve;
use crate::language::{DriftParams, LanguageParams};
use crate::personality::PersonalityRegistry;
use crate::primes::nsm_concept_ids;
use crate::race::{BandSpec, Race};
use crate::runner::{Field, FieldCalib, Runner};
use crate::scenario::ScenarioResolution;
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
/// The language and drift calibrations are armed here, but the per-race articulation substrate the
/// naming game coins from (the [`crate::language::FormSystem`] derived from a race's phoneme priors)
/// is a follow-on increment: until a lineage carries a form system, the naming game and the drift
/// beat are no-ops, so this founder step arms the calibrations and the later increment supplies the
/// derived articulation content that makes them live.
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
    world.seed_dawn_populations(&peoples.races, &peoples.bands, &ring_law);

    // Arm the language and drift calibrations. The concept set is the NSM semantic primes (the anchor
    // meanings a band coordinates words for first); the innovation and sound-change rates are
    // fail-loud manifest reads. The per-race form system is a follow-on, so these stay inert until a
    // lineage carries one.
    world.set_concepts(nsm_concept_ids());
    world.set_language(LanguageParams::from_manifest(manifest)?);
    world.set_drift(DriftParams::from_manifest(manifest)?);

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
