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

//! Explicitly parked fixtures used only by the legacy viewer.

use civsim_core::Fixed;
use civsim_sim::geodynamics::{generate_derived_tiles, DerivedTile};

/// A labelled Slice-0 DEMONSTRATION crust field, the visible spine's scenario stand-in: a `cols` by `rows` grid
/// whose per-tile composition is one of three real rock-forming compositions (light silica SiO2, forsterite
/// Mg2SiO4, and denser periclase MgO), banded lightest-at-the-top. The per-tile composition is the ONLY authored
/// input, and it is a stand-in for what accretion and differentiation will derive in a later slice: this function
/// retires when that chain lands. Everything downstream is DERIVED end to end: each tile's density
/// ([`civsim_physics::petrology::crustal_density`], the stable assemblage the composition minimizes to, never an
/// authored per-rock density), its elevation (Airy isostasy), the field datum
/// ([`civsim_sim::geodynamics::relief_datum`], the field mean), and its relief
/// ([`civsim_sim::geodynamics::classify_relief`], crossing the datum and the sea-level reference). So a lighter
/// crust floats higher and a denser one sits lower from its chemistry alone, and the viewer can show a frame whose
/// terrain is what the material IS. The isostasy fixtures (a synthetic mantle density, a representable column
/// thickness, the surface conditions, and the sea-level datum) are named below; they are Slice-0 fixtures retiring
/// when the accretion and water-budget chains derive them. `None` if the registry or table fails to load or a
/// composition reaches no assemblage (fail-loud, never a fabricated tile).
pub fn slice0_demo_field(cols: usize, rows: usize) -> Option<Vec<DerivedTile>> {
    let registry = civsim_physics::petrology_data::PhaseRegistry::standard().ok()?;
    let table = civsim_physics::periodic::PeriodicTable::standard().ok()?;
    // The three real compositions, lightest to densest. Each resolves through the registry to its stable
    // assemblage; the density falls out of the chemistry.
    let silica = vec![
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(2)),
    ]; // SiO2 (quartz)
    let forsterite = vec![
        ("Mg".to_string(), Fixed::from_int(2)),
        ("Si".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(4)),
    ]; // Mg2SiO4
    let periclase = vec![
        ("Mg".to_string(), Fixed::from_int(1)),
        ("O".to_string(), Fixed::from_int(1)),
    ]; // MgO
    let palette = [silica, forsterite, periclase];
    // Band the palette across the rows, lightest at the top: a demonstration arrangement (the authored, labelled
    // stand-in), so the derived elevations vary down the field and the relief bands emerge from the derived
    // references, never a painted terrain.
    let rows = rows.max(1);
    let cols = cols.max(1);
    let mut field = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        let band = (r * palette.len()) / rows; // 0..palette.len()-1, top to bottom
        for _ in 0..cols {
            field.push(palette[band].clone());
        }
    }
    // Slice-0 isostasy fixtures (retire when the accretion and water-budget chains derive them):
    let mantle_density = Fixed::from_ratio(33, 10); // 3.3 g/cm^3, a synthetic mantle reference
    let crustal_thickness = Fixed::from_int(30); // 30 km, a representable column
    let temperature_k = Fixed::from_int(300); // surface conditions
    let pressure_bar = Fixed::from_int(1);
    let sea_level = Fixed::ZERO; // the ocean/land datum fixture
    generate_derived_tiles(
        &field,
        mantle_density,
        crustal_thickness,
        temperature_k,
        pressure_bar,
        sea_level,
        &registry,
        &table,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_world::terrain::TerrainRelief;

    #[test]
    fn the_slice0_demo_field_derives_all_three_relief_bands_from_real_compositions() {
        // THE VISIBLE SPINE'S SCENARIO: the banded demo field (light silica over forsterite over denser periclase)
        // derives three ordered elevations, so at the sea-level datum the frame shows all three relief classes: the
        // light silica floats to Upland, the forsterite sits Lowland, and the dense periclase sinks below the datum
        // to Submarine. The terrain is what the material is, never painted. All derived from the composition alone.
        let field = slice0_demo_field(4, 6).expect("the demo field derives");
        assert_eq!(field.len(), 24, "a 4 by 6 grid");
        // Deterministic: the same demo replays byte for byte.
        assert_eq!(
            field,
            slice0_demo_field(4, 6).expect("replays"),
            "a pure derivation replays"
        );
        // Elevation falls monotonically down the bands (lighter crust higher): the top row outfloats the bottom.
        assert!(
            field[0].elevation > field[field.len() - 1].elevation,
            "the light top band floats above the dense bottom band"
        );
        // All three relief classes are present: the derived terrain spans ocean, lowland, and upland.
        let has = |r: TerrainRelief| field.iter().any(|t| t.relief == r);
        assert!(has(TerrainRelief::Upland), "the light band is upland");
        assert!(has(TerrainRelief::Lowland), "the middle band is lowland");
        assert!(has(TerrainRelief::Submarine), "the dense band is submarine");
        // A degenerate request still yields a valid (non-empty) field.
        assert!(slice0_demo_field(1, 1).is_some());
    }
}
