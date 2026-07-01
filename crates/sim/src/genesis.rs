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
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, TopologySpace, WorldgenParams};

use crate::biosphere::{generate, Biosphere, EnvProfile, GeneratorParams, Region};
use crate::epoch::{run, EpochParams, EpochReport};
use crate::located::{LocationIndex, OccupantId};

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
}

impl GenesisParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values.
    pub fn dev_default() -> GenesisParams {
        GenesisParams {
            width: 48,
            height: 32,
            region_side: 16,
            generator: GeneratorParams::dev_default(),
            epoch: EpochParams::dev_default(),
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

/// The mature, dawn-ready living world: the generated map, the per-region biospheres (keyed
/// by region-grid coordinate), and the located occupants promoted onto the map.
#[derive(Clone, Debug)]
pub struct LivingWorld {
    pub map: TileMap,
    pub regions: BTreeMap<(i32, i32), RegionBiosphere>,
    pub occupants: LocationIndex,
}

impl LivingWorld {
    /// The total surviving species across all regions.
    pub fn alive(&self) -> u32 {
        self.regions.values().map(|r| r.report.alive).sum()
    }

    /// The total species (living and extinct) across all regions.
    pub fn species(&self) -> usize {
        self.regions.values().map(|r| r.biosphere.len()).sum()
    }

    /// A deterministic 128-bit hash of the whole living world: the map, then each region in
    /// canonical grid order (its environment, its species count and epoch report), then the
    /// occupant placements. The same seed hashes identically on any machine.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        let map_hash = self.map.state_hash();
        h.write_u64((map_hash >> 64) as u64);
        h.write_u64(map_hash as u64);
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
pub fn genesis(seed: u64, params: &GenesisParams) -> LivingWorld {
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(params.width, params.height, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );

    let mut regions: BTreeMap<(i32, i32), RegionBiosphere> = BTreeMap::new();
    let mut occupants = LocationIndex::new();
    let side = params.region_side.max(1);
    let cols = (params.width + side - 1) / side;
    let rows = (params.height + side - 1) / side;

    for ry in 0..rows {
        for rx in 0..cols {
            let x0 = rx * side;
            let y0 = ry * side;
            let region = derive_region(&map, x0, y0, side, params.generator.env_axes);
            // A stable per-region id folds the grid coordinate; used to key the region's draws.
            let region_id = ((rx as u64) << 32) | (ry as u64 & 0xffff_ffff);
            let mut biosphere = generate(seed, &region, region_id, &params.generator);
            let report = run(seed, &mut biosphere, &region, &params.epoch);

            // The dawn: promote a representative surviving organism of each species onto a
            // tile in the region, so the located-identity join carries the living world.
            place_survivors(&mut occupants, &biosphere, region_id, x0, y0, side, &map);

            regions.insert((rx, ry), RegionBiosphere { region, biosphere, report });
        }
    }

    LivingWorld {
        map,
        regions,
        occupants,
    }
}

/// Derive a region's environmental profile from the map tiles in its block: the mean of each
/// terrain field over the block, plus a soil-fertility field (a moisture-derived stand-in
/// until the soil stock lands). Abiotic sources present: light always, water when the block is
/// moist enough for a producer to ground on.
fn derive_region(map: &TileMap, x0: i32, y0: i32, side: i32, env_axes: usize) -> Region {
    let topo = map.topo();
    let x1 = (x0 + side).min(topo.width);
    let y1 = (y0 + side).min(topo.height);
    let mut elev = Vec::new();
    let mut moist = Vec::new();
    let mut temp = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(t) = map.tile(Coord3::ground(x, y)) {
                elev.push(t.elevation);
                moist.push(t.moisture);
                temp.push(t.temperature);
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

    let mut abiotic = std::collections::BTreeSet::new();
    abiotic.insert(0u16); // light, always available above ground
    if m_moist >= Fixed::from_ratio(3, 10) {
        abiotic.insert(1u16); // water, where the block is moist enough
    } else {
        abiotic.insert(2u16); // a dryland soil-nutrient source, so producers can still ground
    }

    Region {
        env: EnvProfile::new(fields),
        abiotic,
    }
}

/// Promote a representative surviving organism of each species onto a tile in the region and
/// index it, spreading them across the block deterministically so the superfine zoom finds
/// occupants. The organism id folds the region id and the species id, so it is stable.
fn place_survivors(
    occupants: &mut LocationIndex,
    biosphere: &Biosphere,
    region_id: u64,
    x0: i32,
    y0: i32,
    side: i32,
    map: &TileMap,
) {
    let topo = map.topo();
    let mut slot = 0i32;
    for id in biosphere.species.ids() {
        let sp = biosphere.species.get(id).unwrap();
        if sp.extinct {
            continue;
        }
        // Spread across the block in a fixed row-major order, staying on the map.
        let dx = slot % side;
        let dy = (slot / side) % side;
        let coord = Coord3::ground(x0 + dx, y0 + dy);
        if topo.contains(coord) {
            let occ_id = StableId((region_id << 20) ^ id.0 as u64);
            occupants.place(OccupantId::organism(occ_id), coord);
            slot += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_replays_bit_identically() {
        let p = GenesisParams::dev_default();
        let a = genesis(0x11FE, &p);
        let b = genesis(0x11FE, &p);
        assert_eq!(a.state_hash(), b.state_hash(), "the same seed yields the same living world");
        let c = genesis(0x2222, &p);
        assert_ne!(a.state_hash(), c.state_hash(), "a different seed, a different world");
    }

    #[test]
    fn the_living_world_has_a_populated_ecology() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p);
        assert!(!w.regions.is_empty(), "the map is partitioned into regions");
        assert!(w.species() > 0, "species were generated");
        assert!(w.alive() > 0, "some species survive to the dawn");
        assert!(!w.occupants.is_empty(), "the dawn placed organisms on the map");
        // The map is still a normal generated map.
        assert_eq!(w.map.topo().width, p.width);
    }

    #[test]
    fn the_epoch_radiated_the_founders() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p);
        let daughters: u32 = w.regions.values().map(|r| r.report.daughters).sum();
        assert!(daughters > 0, "the pre-dawn epoch radiated daughter species");
    }

    #[test]
    fn occupants_are_findable_at_the_superfine_zoom() {
        let p = GenesisParams::dev_default();
        let w = genesis(0x11FE, &p);
        // Some occupied tile has a promoted organism the located join can return.
        let coord = w.occupants.occupied().next().expect("an occupied tile");
        assert!(!w.occupants.occupants(coord).is_empty(), "a tile returns its occupants");
    }
}
