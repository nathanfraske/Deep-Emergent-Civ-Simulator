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

//! Named world structures: the worldgen generation spec and biome set a scenario's declared
//! structure resolves to (the loader arc, gap a).
//!
//! A scenario's world STRUCTURE (how its terrain is generated and which biomes classify it) is a
//! selected input, not an emergent outcome: terrain is a gameplay input on the Principle-9 authored
//! side, like the physics floor and founder band placement (the owner's ruling, OWNER_DECISIONS_LOG
//! R1), not something the simulation must grow. So the STRUCTURE DEFINITIONS are fixed-Rust builders,
//! and which one a world generates on is scenario DATA (the structure name). This registry is the one
//! seam a new structure (Crucible's patchy contested basins, Europa's z-stacked ice-over-ocean) is
//! added at, so the map a world builds is scenario-selected and extensible rather than hardcoded to
//! one default regardless of scenario (the gap [`WorldStructure`] closes: the map was
//! [`WorldgenParams::dev_default`] for every scenario alike).
//!
//! HONEST LIMIT (flagged, not hidden): the Earth structure's biome bands and worldgen params are
//! still the labelled DEV FIXTURE ([`WorldgenParams::dev_default`], [`BiomeSet::dev_default`]), not
//! owner-reserved manifest values. This slice makes the structure SELECTION scenario data; moving the
//! biome/worldgen MAGNITUDES into the calibration manifest as reserved-with-basis values (so a
//! `Profile::Calibrated` world reads them fail-loud like its other dials) is a follow-on, sibling to
//! the Arc-2 calibration pass. And [`crate::worldgen::AXIS_ELEVATION`]'s role-to-ordinal binding stays
//! an engine convention, so a truly alien axis layout is not yet a data row here (the worldgen module
//! flags this).

use crate::terrain::BiomeSet;
use crate::worldgen::WorldgenParams;

/// The canonical Earth structure name: the grounded elevation/moisture/temperature triad that Mirror,
/// Tempest, and every temperate world generate on. A scenario that declares no structure defaults to
/// it, so the neutral map is unchanged (byte-identical to the pre-loader [`WorldgenParams::dev_default`]
/// generation).
pub const EARTH_STRUCTURE: &str = "earth";

/// A named world structure: the worldgen generation spec plus the biome set a scenario's declared
/// structure resolves to. The pairing is deliberate: a structure's biome classifier reads the axes its
/// worldgen spec generates, so the two travel together as one selected unit.
#[derive(Clone, Debug)]
pub struct WorldStructure {
    /// The tile-axis generation spec (fractal params and per-axis [`crate::worldgen::AxisGenSpec`]).
    pub worldgen: WorldgenParams,
    /// The biome set that classifies the generated axes.
    pub biomes: BiomeSet,
}

impl WorldStructure {
    /// Resolve a scenario's declared structure name to its worldgen spec and biome set, or `None` if no
    /// structure by that name is registered. A caller treats `None` as a dangling reference and fails
    /// loud (the same discipline the scenario loader applies to a dangling medium or dial), so a typo or
    /// an unbuilt structure refuses rather than silently defaulting.
    ///
    /// `"earth"` ([`EARTH_STRUCTURE`], and the default when a scenario declares none) is the grounded
    /// triad, resolving to the labelled dev-fixture worldgen and biome set (see the module honest limit).
    /// A world needing exotic terrain (Crucible's patchy basins, Europa's z-stack) is added here as a new
    /// arm; those are flagged substrate arcs (`docs/working/WORLD_SUBSTRATE_READINESS.md`), not built yet.
    pub fn resolve(name: &str) -> Option<WorldStructure> {
        match name {
            EARTH_STRUCTURE => Some(WorldStructure {
                worldgen: WorldgenParams::dev_default(),
                biomes: BiomeSet::dev_default(),
            }),
            _ => None,
        }
    }

    /// Whether a structure by this name is registered (the caller's fail-loud check).
    pub fn is_registered(name: &str) -> bool {
        WorldStructure::resolve(name).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::FlatBounded;
    use crate::worldgen::TileMap;

    #[test]
    fn the_earth_structure_resolves_and_the_default_matches_dev_default() {
        // The Earth structure must generate the byte-identical map the pre-loader path did (which built
        // the map from `dev_default` regardless of scenario), so declaring `structure = "earth"` (or
        // declaring none) leaves every existing world's terrain hash unchanged.
        let s = WorldStructure::resolve(EARTH_STRUCTURE).expect("earth is registered");
        let topo = FlatBounded::new(24, 16, 1);
        let earth = TileMap::generate(0xABCD, topo, &s.biomes, &s.worldgen);
        let devd = TileMap::generate(
            0xABCD,
            topo,
            &BiomeSet::dev_default(),
            &WorldgenParams::dev_default(),
        );
        assert_eq!(
            earth.state_hash(),
            devd.state_hash(),
            "the earth structure is the dev-default triad, byte-identical"
        );
    }

    #[test]
    fn an_unregistered_structure_is_a_dangling_reference() {
        // A structure the registry does not carry resolves to None so the caller fails loud rather than
        // silently defaulting, the same discipline the scenario loader applies to a dangling medium.
        assert!(WorldStructure::resolve("mirror-basins").is_none());
        assert!(!WorldStructure::is_registered("europa-zstack"));
        assert!(WorldStructure::is_registered(EARTH_STRUCTURE));
    }
}
