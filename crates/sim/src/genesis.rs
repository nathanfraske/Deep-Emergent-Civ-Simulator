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

//! The world-genesis sequence, the full R-BIOSPHERE simulation end to end (design Part 25.12).
//!
//! This is the capstone that composes the pieces: worldgen (the [`civsim_world`] `TileMap`)
//! becomes region environmental profiles, each region generates a closed-food-web biosphere
//! ([`crate::biosphere`]), the pre-dawn epoch ([`crate::epoch`]) radiates the founders over
//! deep time, and a representative surviving organism of each species is promoted onto the map
//! through the located-identity join ([`crate::located`]). The sequence is worldgen, then the
//! pre-dawn biosphere epoch, then the dawn-ready living world, all before any people arrive,
//! so a world's ecology is mature and self-made when play begins.
//!
//! The whole sequence is a pure function of one world seed: worldgen, generation, radiation,
//! and promotion each key their randomness through the canonical draw schema, no float enters
//! canonical state, and [`LivingWorld::state_hash`] folds the map, the per-region biospheres,
//! and the occupant placements in canonical order, so the same seed yields a bit-identical
//! living world on any machine. The soil-fertility field is derived here from moisture as a
//! stand-in until the soil `Stock` worldgen field lands (a named build prerequisite).

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId, StateHasher};
use civsim_world::{
    BiomeSet, Coord3, FlatBounded, OrbitalElements, TileMap, TopologySpace, WorldgenParams,
};
use rayon::prelude::*;

use crate::biosphere::{
    generate, representative_structure, Biosphere, EnvProfile, GeneratorParams, Region, SourceRef,
    Species,
};
use crate::clock::Steppable;
use crate::environ::AbioticSourceRegistry;
use crate::epoch::{run, EpochParams, EpochReport, Radiation};
use crate::located::{LocationIndex, OccupantId};
use crate::morphogen::MorphogenProgram;
use crate::physiology::whole_body_composition_vector;
use civsim_bio::anatomy::{temperament_word, BodyPlanRegistry, WorldProfile};
use civsim_bio::genome::IncompatibilityTable;
use civsim_bio::lineage::SpeciesId;

/// The parameters of the whole sequence: the world size, the region block side, and the
/// generator and epoch parameters. DEVELOPMENT FIXTURE via [`GenesisParams::dev_default`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GenesisParams {
    pub width: i32,
    pub height: i32,
    /// The side, in tiles, of a region block (the map is partitioned into these).
    pub region_side: i32,
    pub generator: GeneratorParams,
    pub epoch: EpochParams,
    /// The world profile that gates content (whether magic is present), from the test worlds.
    pub profile: WorldProfile,
    /// The world's orbital elements: its year and day lengths in world-seconds (design Part 14.6,
    /// Part 32). Owner-set per world; a labelled Earth fixture in development. The canonical time
    /// cadences derive from these, and they fold into [`LivingWorld::state_hash`].
    pub orbital: OrbitalElements,
}

impl GenesisParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values (a grounded, no-magic world on an Earth
    /// orbit).
    pub fn dev_default() -> GenesisParams {
        GenesisParams {
            width: 48,
            height: 32,
            region_side: 16,
            generator: GeneratorParams::dev_default(),
            epoch: EpochParams::dev_default(),
            profile: WorldProfile::grounded(),
            orbital: OrbitalElements::dev_earth(),
        }
    }
}

/// A region's biosphere after the epoch: the environment it was fit to, the lineage, and the
/// epoch report.
#[derive(Clone, Debug)]
pub struct RegionBiosphere {
    pub region: Region,
    pub biosphere: Biosphere,
    pub report: EpochReport,
}

/// What a placed occupant is, so a view can draw it in its individual form: which species,
/// its trophic layer (0 producers, the plants; higher the animals), and its region. A read
/// of canon, never authored per occupant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OccupantInfo {
    pub species: SpeciesId,
    pub layer: u16,
    pub region: (i32, i32),
    /// Body mass, so a view can size the individual without a species lookup.
    pub body_mass: Fixed,
}

/// The mature, dawn-ready living world: the generated map, the per-region biospheres (keyed
/// by region-grid coordinate), the located occupants promoted onto the map, what each occupant
/// is (for the superfine view), and the body-plan registry (so a view can name the parts).
#[derive(Clone, Debug)]
pub struct LivingWorld {
    pub map: TileMap,
    pub regions: BTreeMap<(i32, i32), RegionBiosphere>,
    pub occupants: LocationIndex,
    pub occupant_info: BTreeMap<OccupantId, OccupantInfo>,
    pub registry: BodyPlanRegistry,
    /// The world's orbital elements: its year and day lengths in world-seconds (design Part 14.6).
    /// Carried from [`GenesisParams`] and folded into [`LivingWorld::state_hash`], so the orbit is
    /// canonical state: two worlds with the same seed but different orbits are different worlds.
    pub orbital: OrbitalElements,
    /// The world's abiotic-source registry (Arc 5 T1): which abiotic sources exist, where each is present
    /// (its availability bands over the env axes), and how the extract-deplete cycle draws each. The regions'
    /// `abiotic` sets were derived from it at generation, so it is the world's own data (a Terran world uses
    /// [`AbioticSourceRegistry::earth_dev`]; an alien world its own), and the run arms it from here rather
    /// than a hand-written literal, closing the generation-to-run consistency gap.
    pub abiotic: AbioticSourceRegistry,
    /// The world's grown-body context (Arc 6), if it grows its biosphere bodies from their genomes. `None` is
    /// the catalog-tier world (every species carries a sampled `BodyPlan`, byte-identical to pre-Arc-6).
    /// `Some` is the grown-tier world: each species' body is regrown on demand from its pool against the
    /// shared program (diversity comes from the per-species genome, not per-species programs). Bundles the
    /// program, ploidy, and world seed the consumers that read a species' composition and mass need to regrow
    /// its representative Structure.
    pub grown: Option<GrownBodies>,
}

/// The world-scoped context for growing biosphere bodies from their genomes (Arc 6): the shared morphogen
/// program, the ploidy the pools were built with, and the world seed. Held on [`LivingWorld`] so a consumer
/// can regrow any species' representative [`crate::morphogen::Structure`] from its pool
/// ([`crate::biosphere::representative_structure`]); the niche-loci prefix a species carries is read off its
/// own pool, so it is not stored here.
#[derive(Clone, Debug)]
pub struct GrownBodies {
    pub program: MorphogenProgram,
    pub ploidy: usize,
    pub seed: u64,
}

impl LivingWorld {
    /// A species' whole-body composition VECTOR (Arc 6), forking on the body tier: a catalog species reads
    /// its sampled [`civsim_bio::anatomy::BodyPlan`] against the registry (byte-identical to pre-Arc-6); a grown species regrows
    /// its representative [`crate::morphogen::Structure`] from its pool against the world's shared program and
    /// reads its composition directly off the grown tissue. Fail-loud if a grown species is present without
    /// the world's grown-body context (an invariant violation, never a silent Terran default).
    fn species_composition_vector(
        &self,
        species_id: SpeciesId,
        species: &Species,
    ) -> BTreeMap<String, Fixed> {
        match &species.body_plan {
            Some(bp) => whole_body_composition_vector(bp, &self.registry),
            None => self
                .representative_structure_of(species_id, species)
                .whole_body_composition_vector(),
        }
    }

    /// Regrow a grown-tier species' representative Structure from its pool and the world's grown-body context
    /// (Arc 6). The niche-loci prefix is read off the species' own pool (`pool.loci() - program.param_count()`)
    /// so the morphogen block aligns. Panics if the world carries no grown-body context (a grown species
    /// cannot exist without one), matching the fail-loud discipline over a silent default.
    fn representative_structure_of(
        &self,
        species_id: SpeciesId,
        species: &Species,
    ) -> crate::morphogen::Structure {
        let g = self
            .grown
            .as_ref()
            .expect("a grown-tier species requires the world's morphogen grown-body context");
        let niche_loci = species.pool.loci().saturating_sub(g.program.param_count());
        representative_structure(
            species_id,
            &species.pool,
            &g.program,
            niche_loci,
            g.ploidy,
            g.seed,
        )
    }

    /// The located biomass of every PRODUCER occupant, for seeding the run world's food field (the
    /// biosphere-into-run arc): each producer token contributes `pop_capacity * its niche suitability` in
    /// its region, so a plant stands as food where the climate suits it and the founders graze real located
    /// producers rather than a uniform climate number. A producer is identified by the food-web PRIMITIVE
    /// `draws_on` (drawing on an `Abiotic` source means an autotroph, the same key `trophic_label` reads),
    /// never the trophic-LAYER tag (Principle 8), so the producer/consumer split is a reading of the web and
    /// a carnivorous plant (which also draws on an abiotic source) still counts as a food producer. Walks
    /// occupants in canonical coordinate order (`occupied` is `Coord3`-sorted, `occupants` id-sorted), so
    /// the result is deterministic and worker-invariant. `pop_capacity` is the epoch's own reconstitution
    /// scalar (a reserved value), which places the biomass on the same `[0, 1]`-ish scale as the climate
    /// `biomass_from` it replaces.
    pub fn producer_biomass(&self, pop_capacity: Fixed) -> Vec<(Coord3, Fixed)> {
        let mut out = Vec::new();
        for coord in self.occupants.occupied() {
            for occ in self.occupants.occupants(coord) {
                let Some(info) = self.occupant_info.get(&occ) else {
                    continue;
                };
                let Some(rb) = self.regions.get(&info.region) else {
                    continue;
                };
                let Some(species) = rb.biosphere.species.get(info.species) else {
                    continue;
                };
                let autotroph = species
                    .draws_on
                    .iter()
                    .any(|s| matches!(s, SourceRef::Abiotic(_)));
                if !autotroph {
                    continue;
                }
                let suit = species.niche.suitability(&rb.region.env);
                let biomass = pop_capacity.checked_mul(suit).unwrap_or(Fixed::ZERO);
                if biomass > Fixed::ZERO {
                    out.push((coord, biomass));
                }
            }
        }
        out
    }

    /// The evolved abiotic SOURCE id each producer occupant draws on, for the extract-deplete cycle: a
    /// producer fixes biomass from the specific abiotic source its niche evolved to close on (its
    /// `SourceRef::Abiotic(id)`), and the run consults a DATA-DEFINED source binding to learn which field
    /// that id reads and whether it is a depletable stock or a renewable flux. The id's MEANING is never
    /// interpreted here (no `id == 2` soil switch, which would re-author a closed Earth triad): it is read
    /// off the evolved food web and passed opaque to the registry, so an alien source (geothermal, a redox
    /// gradient, a mana field) is one data row, not code. Canonical order, deterministic.
    pub fn producer_sources(&self) -> Vec<(Coord3, Vec<u16>)> {
        let mut out = Vec::new();
        for coord in self.occupants.occupied() {
            for occ in self.occupants.occupants(coord) {
                let Some(info) = self.occupant_info.get(&occ) else {
                    continue;
                };
                let Some(rb) = self.regions.get(&info.region) else {
                    continue;
                };
                let Some(species) = rb.biosphere.species.get(info.species) else {
                    continue;
                };
                // EVERY abiotic edge the niche evolved to draw on (its Liebig factor set), not just the
                // first: a producer limited by light AND soil closes on both, and the extract beat takes the
                // minimum over the set. Species order is canonical; a producer with one edge yields a
                // one-element list (the scalar case, unchanged).
                let sources: Vec<u16> = species
                    .draws_on
                    .iter()
                    .filter_map(|s| match s {
                        SourceRef::Abiotic(id) => Some(*id),
                        _ => None,
                    })
                    .collect();
                if !sources.is_empty() {
                    out.push((coord, sources));
                }
            }
        }
        out
    }

    /// The located BODY MATTER of every CONSUMER occupant (a non-autotroph animal), for predation: each
    /// consumer's body is its own physics-derived `whole_body_composition_vector` (the SAME fold a corpse
    /// uses, content-addressed, no minted substance), scaled to a volume by its body mass and the located
    /// population. Consumer versus producer is read from the food-web PRIMITIVE draws_on (no Abiotic edge
    /// means a consumer), never a trophic tag. Deterministic canonical order. A founder eats one through the
    /// geophage by the same edibility that grazes plants; the matter cycle decomposes it back to soil.
    pub fn consumer_bodies(
        &self,
        pop_capacity: Fixed,
    ) -> Vec<(Coord3, std::collections::BTreeMap<String, Fixed>, Fixed)> {
        let mut out = Vec::new();
        for coord in self.occupants.occupied() {
            for occ in self.occupants.occupants(coord) {
                let Some(info) = self.occupant_info.get(&occ) else {
                    continue;
                };
                let Some(rb) = self.regions.get(&info.region) else {
                    continue;
                };
                let Some(species) = rb.biosphere.species.get(info.species) else {
                    continue;
                };
                let autotroph = species
                    .draws_on
                    .iter()
                    .any(|s| matches!(s, SourceRef::Abiotic(_)));
                if autotroph {
                    continue;
                }
                let composition = self.species_composition_vector(info.species, species);
                if composition.is_empty() {
                    continue;
                }
                let suit = species.niche.suitability(&rb.region.env);
                let volume = info
                    .body_mass
                    .checked_mul(pop_capacity.checked_mul(suit).unwrap_or(Fixed::ZERO))
                    .unwrap_or(Fixed::ZERO);
                if volume > Fixed::ZERO {
                    out.push((coord, composition, volume));
                }
            }
        }
        out
    }

    /// The standing-food COMPOSITION of every PRODUCER occupant (an autotroph), for the food web's write side
    /// (chemistry arc, T3): a producer's standing food is its OWN physics-derived `whole_body_composition_vector`
    /// (the SAME fold a corpse and a consumer body use), so the food a grazer eats carries the plant's own
    /// chemistry rather than one minted `bio.energy_density` class. Producer versus consumer is read from the
    /// food-web PRIMITIVE draws_on (an Abiotic edge means an autotroph), never a kingdom tag, so a walking,
    /// prey-eating autotroph is still a producer here. The composition is the fixed per-unit-biomass density
    /// vector; the standing biomass VOLUME it scales is the environ's logistic productivity stock, so grazing
    /// draws the volume and the composition rides fixed (the read-back stays a single volume, not N stocks).
    /// The consumer of this ([`crate::environ::EnvironFields::set_producer_food`]) carries it at its REAL per-axis
    /// magnitudes (CORRECTED-T3), no longer a sum-to-one simplex, so an energy-dense plant feeds more than a woody
    /// one. Deterministic canonical order; a cell with two producer occupants keeps the last (as
    /// `set_producer_food` overwrites). Mirrors [`Self::consumer_bodies`].
    pub fn producer_compositions(
        &self,
    ) -> Vec<(Coord3, std::collections::BTreeMap<String, Fixed>)> {
        let mut out = Vec::new();
        for coord in self.occupants.occupied() {
            for occ in self.occupants.occupants(coord) {
                let Some(info) = self.occupant_info.get(&occ) else {
                    continue;
                };
                let Some(rb) = self.regions.get(&info.region) else {
                    continue;
                };
                let Some(species) = rb.biosphere.species.get(info.species) else {
                    continue;
                };
                let autotroph = species
                    .draws_on
                    .iter()
                    .any(|s| matches!(s, SourceRef::Abiotic(_)));
                if !autotroph {
                    continue;
                }
                let composition = self.species_composition_vector(info.species, species);
                if composition.is_empty() {
                    continue;
                }
                out.push((coord, composition));
            }
        }
        out
    }

    /// The total surviving species across all regions.
    pub fn alive(&self) -> u32 {
        self.regions.values().map(|r| r.report.alive).sum()
    }

    /// The total species (living and extinct) across all regions.
    pub fn species(&self) -> usize {
        self.regions.values().map(|r| r.biosphere.len()).sum()
    }

    /// A one-line description of a placed occupant for the superfine inspector: its derived
    /// trophic label (from what it eats, not a stored type), temperament, natural weapons,
    /// covering, and senses, all named from the body-plan registry.
    pub fn describe(&self, occ: OccupantId) -> String {
        let info = match self.occupant_info.get(&occ) {
            Some(i) => i,
            None => return "unknown".to_string(),
        };
        let region = match self.regions.get(&info.region) {
            Some(r) => r,
            None => return "unknown".to_string(),
        };
        let bio = &region.biosphere;
        let sp = match bio.species.get(info.species) {
            Some(s) => s,
            None => return "unknown".to_string(),
        };
        // Derive the kingdom-and-diet label cheaply, without cloning the region (fork F11).
        // Kingdom is autotrophy, not diet: a producer is a plant whatever it eats.
        let is_producer = sp
            .draws_on
            .iter()
            .any(|s| matches!(s, SourceRef::Abiotic(_)));
        let mut eats_species = false;
        let mut eats_animal = false;
        let mut eats_plant = false;
        for src in &sp.draws_on {
            if let SourceRef::Species(dep) = src {
                eats_species = true;
                if let Some(prey) = bio.species.get(*dep) {
                    if prey
                        .draws_on
                        .iter()
                        .any(|s| matches!(s, SourceRef::Abiotic(_)))
                    {
                        eats_plant = true;
                    }
                    if prey
                        .draws_on
                        .iter()
                        .any(|s| matches!(s, SourceRef::Species(_)))
                    {
                        eats_animal = true;
                    }
                }
            }
        }
        let label = if is_producer {
            if eats_species {
                "carnivorous plant"
            } else {
                "plant"
            }
        } else if eats_animal && eats_plant {
            "omnivore"
        } else if eats_animal {
            "carnivore"
        } else {
            "herbivore"
        };
        match &sp.body_plan {
            // Catalog tier: name the typed parts (byte-identical to pre-Arc-6).
            Some(bp) => {
                let reg = &self.registry;
                let weapons: Vec<&str> = bp
                    .weapons
                    .iter()
                    .map(|p| BodyPlanRegistry::name(&reg.weapons, p.kind))
                    .collect();
                let senses: Vec<&str> = bp
                    .senses
                    .iter()
                    .map(|p| BodyPlanRegistry::name(&reg.senses, p.kind))
                    .collect();
                let covering = BodyPlanRegistry::name(&reg.coverings, bp.covering.kind);
                let arms = if weapons.is_empty() {
                    "unarmed".to_string()
                } else {
                    weapons.join("+")
                };
                format!(
                    "{label}#{}  {}  {arms}  {covering}  senses:{}",
                    info.species.0,
                    temperament_word(bp.temperament.boldness),
                    senses.join("/")
                )
            }
            // Grown tier (Arc 6): a grown body has no named parts (function is read off the structure, not a
            // catalog), so describe it by its grown physics: segment count and grown energy density. Flagged
            // as reduced-fidelity, not a placeholder default.
            None => {
                let s = self.representative_structure_of(info.species, sp);
                format!(
                    "{label}#{}  grown  segments:{}  e_density:{}",
                    info.species.0,
                    s.segment_count(),
                    s.whole_body_energy_density().to_bits()
                )
            }
        }
    }

    /// A deterministic 128-bit hash of the whole living world: the map, then each region in
    /// canonical grid order (its environment, its species count and epoch report), then the
    /// occupant placements. The same seed hashes identically on any machine.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        let map_hash = self.map.state_hash();
        h.write_u64((map_hash >> 64) as u64);
        h.write_u64(map_hash as u64);
        // The world's orbit folds in at a pinned position: right after the map hash and before the
        // regions, orbital period then rotation period, order fixed. A change to the year or day
        // length changes the world hash deterministically (Principle 3), so the orbit is canonical
        // state rather than a display value.
        h.write_fixed(self.orbital.orbital_period_seconds);
        h.write_fixed(self.orbital.rotation_period_seconds);
        for ((rx, ry), rb) in &self.regions {
            h.write_u32(*rx as u32);
            h.write_u32(*ry as u32);
            for &field in &rb.region.env.fields {
                h.write_fixed(field);
            }
            h.write_u32(rb.biosphere.len() as u32);
            h.write_u32(rb.report.daughters);
            h.write_u32(rb.report.extinctions);
            h.write_u32(rb.report.alive);
            h.write_u32(rb.report.incompatibilities);
            h.write_u32(rb.biosphere.empty_niches);
        }
        for coord in self.occupants.occupied() {
            h.write_u32(coord.x as u32);
            h.write_u32(coord.y as u32);
            for occ in self.occupants.occupants(coord) {
                h.write_u64(occ.id.0);
            }
        }
        h.finish()
    }
}

/// Run the whole world-genesis sequence deterministically from one world seed.
pub fn genesis(
    seed: u64,
    params: &GenesisParams,
    abiotic: &AbioticSourceRegistry,
    morphogen: Option<&MorphogenProgram>,
) -> LivingWorld {
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(params.width, params.height, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );

    let mut regions: BTreeMap<(i32, i32), RegionBiosphere> = BTreeMap::new();
    let mut occupants = LocationIndex::new();
    let mut occupant_info: BTreeMap<OccupantId, OccupantInfo> = BTreeMap::new();
    let registry = BodyPlanRegistry::dev_default();
    let side = params.region_side.max(1);
    let cols = (params.width + side - 1) / side;
    let rows = (params.height + side - 1) / side;
    // The world's grown-body context (Arc 6), built once from the shared program, the generator ploidy, and
    // the seed; passed to promotion and carried on the world so a consumer can regrow any species' body.
    let grown = morphogen.map(|p| GrownBodies {
        program: p.clone(),
        ploidy: params.generator.ploidy,
        seed,
    });

    for ry in 0..rows {
        for rx in 0..cols {
            let x0 = rx * side;
            let y0 = ry * side;
            let region = derive_region(&map, x0, y0, side, params.generator.env_axes, abiotic);
            // A stable per-region id folds the grid coordinate; used to key the region's draws.
            let region_id = ((rx as u64) << 32) | (ry as u64 & 0xffff_ffff);
            let mut biosphere = generate(
                seed,
                &region,
                region_id,
                &params.generator,
                &registry,
                params.profile,
                morphogen,
            );
            // The pre-dawn biosphere declares no Dobzhansky-Muller incompatibilities yet, so the
            // speciation gate reads an empty table and falls back to the frequency-distance rule; a
            // world that declares a table is a data addition (design 25.7, WP1).
            let report = run(
                seed,
                &mut biosphere,
                &region,
                &params.epoch,
                &IncompatibilityTable::new(),
            );

            // The dawn: promote a representative surviving organism of each species onto a
            // tile in the region, so the located-identity join carries the living world.
            place_survivors(
                &mut occupants,
                &mut occupant_info,
                &biosphere,
                region_id,
                (rx, ry),
                x0,
                y0,
                side,
                &map,
                grown.as_ref(),
            );

            regions.insert(
                (rx, ry),
                RegionBiosphere {
                    region,
                    biosphere,
                    report,
                },
            );
        }
    }

    LivingWorld {
        map,
        regions,
        occupants,
        occupant_info,
        registry,
        orbital: params.orbital,
        abiotic: abiotic.clone(),
        grown,
    }
}

/// One region held by the staged world-genesis driver: the block it covers on the map and its
/// running radiation (which owns the region's biosphere and its epoch state).
#[derive(Clone, Debug)]
struct StagedRegion {
    coord: (i32, i32),
    region_id: u64,
    x0: i32,
    y0: i32,
    radiation: Radiation,
}

/// The staged form of world genesis, so the pre-dawn radiation can be watched unfolding rather
/// than only seen as the finished [`LivingWorld`]. Where [`genesis`] runs worldgen, then every
/// region's whole radiation, then the dawn placement in one call, this driver runs worldgen and
/// the founder generation up front, then advances every region's radiation one generation per
/// [`crate::clock::Steppable::step`], and can produce a [`LivingWorld`] snapshot of the current state at any
/// point. Stepped to completion it yields a living world bit-identical to [`genesis`], since the
/// radiation stepper reproduces the batch epoch exactly and the placement is a pure function of
/// the matured biospheres. It is a driver over canonical state, not a view: it holds no camera and
/// writes no randomness beyond the deterministic epoch (Principle 10).
#[derive(Clone, Debug)]
pub struct WorldGenesis {
    params: GenesisParams,
    map: TileMap,
    registry: BodyPlanRegistry,
    regions: Vec<StagedRegion>,
    side: i32,
    gen: u64,
    abiotic: AbioticSourceRegistry,
    grown: Option<GrownBodies>,
}

impl WorldGenesis {
    /// Begin a staged world genesis: run worldgen and seed every region's founders (generation 0),
    /// leaving the radiation to be stepped. Deterministic from the one world seed, exactly as
    /// [`genesis`].
    pub fn new(
        seed: u64,
        params: &GenesisParams,
        abiotic: &AbioticSourceRegistry,
        morphogen: Option<&MorphogenProgram>,
    ) -> WorldGenesis {
        let biomes = BiomeSet::dev_default();
        let map = TileMap::generate(
            seed,
            FlatBounded::new(params.width, params.height, 1),
            &biomes,
            &WorldgenParams::dev_default(),
        );
        let registry = BodyPlanRegistry::dev_default();
        let side = params.region_side.max(1);
        let cols = (params.width + side - 1) / side;
        let rows = (params.height + side - 1) / side;

        let mut regions = Vec::new();
        for ry in 0..rows {
            for rx in 0..cols {
                let x0 = rx * side;
                let y0 = ry * side;
                let region = derive_region(&map, x0, y0, side, params.generator.env_axes, abiotic);
                let region_id = ((rx as u64) << 32) | (ry as u64 & 0xffff_ffff);
                let biosphere = generate(
                    seed,
                    &region,
                    region_id,
                    &params.generator,
                    &registry,
                    params.profile,
                    morphogen,
                );
                let radiation = Radiation::new(
                    seed,
                    biosphere,
                    region,
                    params.epoch,
                    IncompatibilityTable::new(),
                );
                regions.push(StagedRegion {
                    coord: (rx, ry),
                    region_id,
                    x0,
                    y0,
                    radiation,
                });
            }
        }

        WorldGenesis {
            params: *params,
            map,
            registry,
            regions,
            side,
            gen: 0,
            abiotic: abiotic.clone(),
            grown: morphogen.map(|p| GrownBodies {
                program: p.clone(),
                ploidy: params.generator.ploidy,
                seed,
            }),
        }
    }

    /// Advance every region's radiation by one generation, if any remain. Returns whether a
    /// generation was run (false once the whole radiation is complete).
    pub fn step_once(&mut self) -> bool {
        if self.gen >= self.params.epoch.generations {
            return false;
        }
        // DETERMINISTIC data-parallelism (arc 4): each region owns a DISJOINT radiation (its own biosphere,
        // species pools, and stocks); no region reads or writes another's state, so `par_iter_mut` hands each
        // closure an exclusive `&mut` element, and every draw inside `step_once` keys through a DrawKey on the
        // region/pool id (not the thread), so the result is bit-identical at any thread count.
        self.regions.par_iter_mut().for_each(|sr| {
            sr.radiation.step_once();
        });
        self.gen += 1;
        true
    }

    /// Generations run so far across the radiation.
    pub fn generation(&self) -> u64 {
        self.gen
    }

    /// The planned total generations.
    pub fn generations_planned(&self) -> u64 {
        self.params.epoch.generations
    }

    /// Whether the whole radiation has run.
    pub fn is_complete(&self) -> bool {
        self.gen >= self.params.epoch.generations
    }

    /// The living species across all regions at the generation reached so far.
    pub fn alive(&self) -> u32 {
        self.regions
            .iter()
            .map(|sr| sr.radiation.report().alive)
            .sum()
    }

    /// The total species (living and extinct) across all regions so far.
    pub fn species(&self) -> usize {
        self.regions
            .iter()
            .map(|sr| sr.radiation.biosphere().len())
            .sum()
    }

    /// The generated map (fixed for the life of the driver).
    pub fn map(&self) -> &TileMap {
        &self.map
    }

    /// Build a [`LivingWorld`] snapshot of the current state: the map, each region's biosphere and
    /// report as they stand, and a fresh dawn placement of the surviving organisms. A pure read of
    /// the driver's canonical state; the driver is unchanged.
    pub fn snapshot(&self) -> LivingWorld {
        let mut regions: BTreeMap<(i32, i32), RegionBiosphere> = BTreeMap::new();
        let mut occupants = LocationIndex::new();
        let mut occupant_info: BTreeMap<OccupantId, OccupantInfo> = BTreeMap::new();
        for sr in &self.regions {
            let biosphere = sr.radiation.biosphere();
            place_survivors(
                &mut occupants,
                &mut occupant_info,
                biosphere,
                sr.region_id,
                sr.coord,
                sr.x0,
                sr.y0,
                self.side,
                &self.map,
                self.grown.as_ref(),
            );
            regions.insert(
                sr.coord,
                RegionBiosphere {
                    region: sr.radiation.region().clone(),
                    biosphere: biosphere.clone(),
                    report: sr.radiation.report(),
                },
            );
        }
        LivingWorld {
            map: self.map.clone(),
            regions,
            occupants,
            occupant_info,
            registry: self.registry.clone(),
            orbital: self.params.orbital,
            abiotic: self.abiotic.clone(),
            grown: self.grown.clone(),
        }
    }

    /// Run the whole radiation to completion and return the mature living world, the batch result.
    pub fn into_living(mut self) -> LivingWorld {
        while self.step_once() {}
        self.snapshot()
    }
}

impl Steppable for WorldGenesis {
    fn step(&mut self) {
        self.step_once();
    }
    fn now(&self) -> u64 {
        self.gen
    }
}

/// Derive a region's environmental profile from the map tiles in its block: the mean of each
/// terrain field over the block, plus a soil-fertility field (a moisture-derived stand-in
/// until the soil stock lands). The abiotic sources PRESENT are a data read of the world's own
/// `sources` registry against the region's env (Arc 5 T1, [`AbioticSourceRegistry::available_in`]),
/// not a hardcoded triad: the Terran fixture reproduces "light always, water where moist, else a
/// dryland soil source", an alien world declares its own presence bands.
fn derive_region(
    map: &TileMap,
    x0: i32,
    y0: i32,
    side: i32,
    env_axes: usize,
    sources: &AbioticSourceRegistry,
) -> Region {
    let topo = map.topo();
    let x1 = (x0 + side).min(topo.width);
    let y1 = (y0 + side).min(topo.height);
    let mut elev = Vec::new();
    let mut moist = Vec::new();
    let mut temp = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(t) = map.tile(Coord3::ground(x, y)) {
                elev.push(t.elevation());
                moist.push(t.moisture());
                temp.push(t.temperature());
            }
        }
    }
    let mean = |v: &[Fixed]| -> Fixed {
        if v.is_empty() {
            Fixed::ZERO
        } else {
            Fixed::saturating_sum(v.iter().copied())
                .checked_div(Fixed::from_int(v.len() as i32))
                .unwrap_or(Fixed::ZERO)
        }
    };
    let m_elev = mean(&elev);
    let m_moist = mean(&moist);
    let m_temp = mean(&temp);
    // Field vector, padded to env_axes: [elevation, moisture, temperature, soil-fertility, ...].
    let mut fields = vec![m_elev, m_moist, m_temp, m_moist];
    fields.truncate(env_axes);
    while fields.len() < env_axes {
        fields.push(Fixed::from_ratio(1, 2));
    }

    // The abiotic sources PRESENT here derive from the world's declared registry against this region's env
    // (Arc 5 T1): each source's availability bands are read over `fields`, so the world's own sources and
    // presence conditions decide the set, never the Earth "light always, water where moist" hardcode. The
    // dev-fixture registry reproduces that Earth triad exactly (light always, water where moist >= 0.3, a
    // dryland soil source where drier), so a canonical Earth world is byte-identical.
    let abiotic = sources.available_in(&fields);

    Region {
        env: EnvProfile::new(fields),
        abiotic,
    }
}

/// Promote a representative surviving organism of each species onto a tile in the region and
/// index it, spreading them across the block deterministically so the superfine zoom finds
/// occupants. The organism id folds the region id and the species id, so it is stable.
#[allow(clippy::too_many_arguments)]
fn place_survivors(
    occupants: &mut LocationIndex,
    occupant_info: &mut BTreeMap<OccupantId, OccupantInfo>,
    biosphere: &Biosphere,
    region_id: u64,
    region: (i32, i32),
    x0: i32,
    y0: i32,
    side: i32,
    map: &TileMap,
    grown: Option<&GrownBodies>,
) {
    let topo = map.topo();
    for id in biosphere.species.ids() {
        let sp = biosphere.species.get(id).unwrap();
        if sp.extinct {
            continue;
        }
        // Scatter across the region block by a deterministic per-species hash, so the living
        // world reads as a spread ecology rather than a clustered grid. Collisions share a
        // tile, which the location index and the superfine view both handle.
        let dx = (civsim_core::splitmix64(id.0 as u64 ^ region_id) % side.max(1) as u64) as i32;
        let dy = (civsim_core::splitmix64(id.0 as u64 ^ region_id ^ 0x5bd1_e995)
            % side.max(1) as u64) as i32;
        let coord = Coord3::ground(x0 + dx, y0 + dy);
        if topo.contains(coord) {
            let occ = OccupantId::organism(StableId((region_id << 20) ^ id.0 as u64));
            occupants.place(occ, coord);
            occupant_info.insert(
                occ,
                OccupantInfo {
                    species: id,
                    layer: sp.layer,
                    region,
                    // Arc 6: a catalog species reads its sampled plan's mass; a grown species reads the mass
                    // its representative Structure digests to (fail-loud without the world's grown context).
                    body_mass: match &sp.body_plan {
                        Some(bp) => bp.body_mass,
                        None => {
                            let g = grown.expect(
                                "a grown-tier species requires the world's morphogen grown-body context",
                            );
                            let niche_loci =
                                sp.pool.loci().saturating_sub(g.program.param_count());
                            representative_structure(
                                id, &sp.pool, &g.program, niche_loci, g.ploidy, g.seed,
                            )
                            .digest()
                            .body_mass
                        }
                    },
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_replays_bit_identically() {
        let p = GenesisParams::dev_default();
        let a = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        let b = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "the same seed yields the same living world"
        );
        let c = genesis(0x2222, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert_ne!(
            a.state_hash(),
            c.state_hash(),
            "a different seed, a different world"
        );
    }

    #[test]
    fn the_orbit_folds_into_the_world_hash() {
        // The orbit is canonical state (design Part 14.6, Principle 3): the same seed and orbit
        // hash identically, and the same seed under a different orbit hashes differently, so a
        // world's year and day length are part of what makes it that world.
        let mut p = GenesisParams::dev_default();
        let earth = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert_eq!(
            earth.state_hash(),
            genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None).state_hash(),
            "same seed and orbit, same hash"
        );
        // A different orbit, everything else held: a different world.
        p.orbital = OrbitalElements {
            orbital_period_seconds: Fixed::from_int(86_400),
            rotation_period_seconds: Fixed::from_int(3_600),
        };
        let fast = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert_ne!(
            earth.state_hash(),
            fast.state_hash(),
            "a different orbit is a different world"
        );
        // And the different-orbit world is itself reproducible.
        assert_eq!(
            fast.state_hash(),
            genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None).state_hash(),
            "the different-orbit world replays bit-identically"
        );
    }

    #[test]
    fn staged_genesis_matches_batch_genesis() {
        // The end-to-end determinism proof for the live view: the staged driver stepped to
        // completion produces a living world bit-identical to the one-shot batch genesis, so
        // watching the radiation unfold never diverges from the canonical result.
        let p = GenesisParams::dev_default();
        let batch = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        let staged =
            WorldGenesis::new(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None).into_living();
        assert_eq!(
            batch.state_hash(),
            staged.state_hash(),
            "staged genesis stepped to completion matches batch genesis bit for bit"
        );
    }

    #[test]
    fn a_staged_genesis_can_be_watched_step_by_step() {
        let p = GenesisParams::dev_default();
        let mut wg = WorldGenesis::new(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert_eq!(wg.generation(), 0);
        let founders = wg.species();
        assert!(founders > 0, "the founders are seeded before any radiation");
        // A snapshot at generation 0 is already a valid living world (the founders on the map).
        let snap0 = wg.snapshot();
        assert!(
            !snap0.occupants.is_empty(),
            "generation 0 places the founders"
        );
        // Step the whole radiation; progress advances and the ecology grows.
        for _ in 0..p.epoch.generations {
            wg.step_once();
        }
        assert!(wg.is_complete());
        assert_eq!(wg.generation(), p.epoch.generations);
        let snapf = wg.snapshot();
        assert!(
            snapf.species() >= snap0.species(),
            "the radiation grew the lineage"
        );
        assert_eq!(
            snapf.state_hash(),
            genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None).state_hash(),
            "the fully stepped snapshot equals batch genesis"
        );
    }

    #[test]
    fn the_living_world_has_a_populated_ecology() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        assert!(!w.regions.is_empty(), "the map is partitioned into regions");
        assert!(w.species() > 0, "species were generated");
        assert!(w.alive() > 0, "some species survive to the dawn");
        assert!(
            !w.occupants.is_empty(),
            "the dawn placed organisms on the map"
        );
        // The map is still a normal generated map.
        assert_eq!(w.map.topo().width, p.width);
    }

    #[test]
    fn the_epoch_radiated_the_founders() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        let daughters: u32 = w.regions.values().map(|r| r.report.daughters).sum();
        assert!(
            daughters > 0,
            "the pre-dawn epoch radiated daughter species"
        );
    }

    #[test]
    fn occupants_are_findable_at_the_superfine_zoom() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p, &AbioticSourceRegistry::earth_dev(), None);
        // Some occupied tile has a promoted organism the located join can return.
        let coord = w.occupants.occupied().next().expect("an occupied tile");
        assert!(
            !w.occupants.occupants(coord).is_empty(),
            "a tile returns its occupants"
        );
    }
}
