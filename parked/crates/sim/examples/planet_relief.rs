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

//! A PLANET-SCALE feasibility check of the derived surface-elevation pipeline: does the same
//! composition-to-terrain chain the visible spine proved on a small field still produce varied,
//! geologically-plausible relief when the crust composition varies GEOGRAPHICALLY across a whole planet,
//! rather than in three horizontal bands?
//!
//! The chain under test is DERIVED end to end and unchanged from the spine: each tile's crustal density is
//! the stable assemblage its composition minimizes to (`civsim_physics::petrology::crustal_density`), its
//! elevation is the Airy isostatic flotation of that crust on the mantle
//! (`civsim_physics::geodynamics::airy_isostatic_elevation`), the relief datum is the field mean
//! (`civsim_world::terrain::relief_datum`), and each tile's relief class crosses that datum and the
//! sea-level reference (`civsim_world::terrain::classify_relief`). A lighter or thicker crust floats
//! higher; a denser or thinner one sits lower; nothing about the terrain is painted.
//!
//! The ONE authored input is the per-tile composition and crustal thickness, exactly as
//! `slice0_demo_field` authors its three-band palette. Here that stand-in is a plate-tectonic pattern
//! rather than bands: a deterministic Voronoi tessellation into plates, each plate assigned one of four
//! crust compositions by a hash of its index, with crustal thickening toward plate interiors (stable
//! cores) and along convergent boundaries (collision belts).
//!
//! The four crust types are named by their COMPOSITION, not by any Earth tectonic setting: a light silica
//! crust, a magnesium-olivine crust, an iron-bearing olivine crust, and an iron-oxide crust, spanning a
//! range of derived densities. This is deliberately not Terran-keyed. The engine underneath admits the
//! alien at the mechanism level: `crustal_density` keys on the elemental composition and minimizes Gibbs
//! free energy over the phase REGISTRY, so a world of a different chemistry is a different element vector
//! plus different registry rows (a data row, never a rewrite), and it is fail-loud on a chemistry with no
//! matching phase. The registry currently holds six silicate/oxide phases, so this is a silicate-world map
//! by DATA, not by any assumption in the mechanism; an ice-crust or metal-crust world renders through the
//! identical pipeline once its phases are added. This whole stand-in retires when the accretion,
//! differentiation, and tectonic chains derive the composition field. It is fully deterministic (a fixed
//! hash, no RNG), so the map replays byte for byte.
//!
//! Run: `cargo run -p civsim-sim --example planet_relief`. It writes a compact grid of the derived
//! elevation, province, and relief class to stdout for the renderer.

use civsim_core::Fixed;
use civsim_physics::geodynamics::airy_isostatic_elevation;
use civsim_physics::periodic::PeriodicTable;
use civsim_physics::petrology::crustal_density;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_world::terrain::{classify_relief, relief_datum, TerrainRelief};

const WIDTH: usize = 220;
const HEIGHT: usize = 110;
const PLATES: usize = 18;
const SEED: u32 = 0x5EED_1234;

/// A deterministic integer finalizer (Murmur3-style), the reproducible stand-in for the plate scatter and
/// province assignment. No RNG state: a pure function of its input, so the whole field replays identically.
fn hash32(mut x: u32) -> u32 {
    x ^= SEED;
    x = x.wrapping_mul(0x9E37_79B1);
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    x
}

/// The four crust types the plate stand-in assigns, named by COMPOSITION rather than any Earth tectonic
/// setting. Each maps to a real element-count vector the phase registry resolves and a base crustal
/// thickness; the elevation falls out of the derived density and the isostasy. The `Silica` crust is the
/// lightest (floats high, forms the thick standing masses); the others grow denser through the olivines to
/// the iron oxide (sit lower to founder).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Crust {
    Silica,
    Olivine,
    FerroanOlivine,
    IronOxide,
}

impl Crust {
    fn from_plate(plate: usize) -> Self {
        match hash32(plate as u32 ^ 0xA11CE) % 100 {
            0..=33 => Crust::Silica,
            34..=66 => Crust::Olivine,
            67..=90 => Crust::FerroanOlivine,
            _ => Crust::IronOxide,
        }
    }

    /// True for the light silica crust, the one that floats high enough to build thick standing masses (so
    /// two adjacent light-crust plates thicken into a collision belt rather than subside).
    fn is_light(self) -> bool {
        matches!(self, Crust::Silica)
    }

    fn code(self) -> u8 {
        match self {
            Crust::Silica => 0,
            Crust::Olivine => 1,
            Crust::FerroanOlivine => 2,
            Crust::IronOxide => 3,
        }
    }

    /// The crust composition, an element-count vector the registry minimizes to a stable assemblage.
    fn composition(self) -> Vec<(String, Fixed)> {
        match self {
            // Silica (quartz), the lightest rock-former, floats high.
            Crust::Silica => vec![
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(2)),
            ],
            // Forsterite (Mg-olivine), denser, near mantle density.
            Crust::Olivine => vec![
                ("Mg".to_string(), Fixed::from_int(2)),
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(4)),
            ],
            // A Mg-Fe olivine, denser still, founders into deeper basins.
            Crust::FerroanOlivine => vec![
                ("Mg".to_string(), Fixed::from_int(1)),
                ("Fe".to_string(), Fixed::from_int(1)),
                ("Si".to_string(), Fixed::from_int(1)),
                ("O".to_string(), Fixed::from_int(4)),
            ],
            // Hematite, the densest, sinks to the deepest floor.
            Crust::IronOxide => vec![
                ("Fe".to_string(), Fixed::from_int(2)),
                ("O".to_string(), Fixed::from_int(3)),
            ],
        }
    }
}

/// The wrapped east-west distance and plain north-south distance from a tile to a plate seed, on a cylinder
/// (longitude wraps, latitude does not). Returns the squared distance in tile units.
fn dist2(tx: usize, ty: usize, sx: usize, sy: usize) -> i64 {
    let raw = (tx as i64 - sx as i64).abs();
    let dx = raw.min(WIDTH as i64 - raw);
    let dy = ty as i64 - sy as i64;
    dx * dx + dy * dy
}

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut r = (v as f64).sqrt() as i64;
    while r * r > v {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= v {
        r += 1;
    }
    r
}

fn main() {
    let registry = PhaseRegistry::standard().expect("phase registry loads");
    let table = PeriodicTable::standard().expect("periodic table loads");
    let temperature_k = Fixed::from_int(300);
    let pressure_bar = Fixed::from_int(1);
    let mantle_density = Fixed::from_ratio(33, 10); // 3.3 g/cm^3, the Slice-0 mantle fixture.
    let sea_level = Fixed::from_int(1500); // metres, the labelled ocean/land datum fixture.

    // Plate seeds, scattered deterministically across the grid.
    let seeds: Vec<(usize, usize)> = (0..PLATES)
        .map(|i| {
            let sx = (hash32(i as u32 * 2) as usize) % WIDTH;
            let sy = (hash32(i as u32 * 2 + 1) as usize) % HEIGHT;
            (sx, sy)
        })
        .collect();

    // Memoize the derived density per crust type (a pure function of composition, temperature, pressure).
    let densities: Vec<Fixed> = [
        Crust::Silica,
        Crust::Olivine,
        Crust::FerroanOlivine,
        Crust::IronOxide,
    ]
    .iter()
    .map(|c| {
        crustal_density(
            &c.composition(),
            temperature_k,
            pressure_bar,
            &registry,
            &table,
        )
        .expect("the crust composition resolves to a density")
    })
    .collect();
    let density_of = |c: Crust| densities[c.code() as usize];

    let mut elevations: Vec<Fixed> = Vec::with_capacity(WIDTH * HEIGHT);
    let mut provinces: Vec<u8> = Vec::with_capacity(WIDTH * HEIGHT);

    for ty in 0..HEIGHT {
        for tx in 0..WIDTH {
            // Nearest and second-nearest plate (for the province and the boundary geometry).
            let (mut d1, mut d2) = (i64::MAX, i64::MAX);
            let (mut p1, mut p2) = (0usize, 0usize);
            for (i, &(sx, sy)) in seeds.iter().enumerate() {
                let d = dist2(tx, ty, sx, sy);
                if d < d1 {
                    d2 = d1;
                    p2 = p1;
                    d1 = d;
                    p1 = i;
                } else if d < d2 {
                    d2 = d;
                    p2 = i;
                }
            }
            let crust = Crust::from_plate(p1);
            let neighbour = Crust::from_plate(p2);
            let gap = isqrt(d2) - isqrt(d1); // tiles from the nearest plate boundary.

            // Crustal thickness (metres): the second authored field, a tectonic stand-in. The variation is
            // geological, not noise: stable cores thicken toward light-crust plate interiors, margins thin
            // toward the boundary, collision roots thicken where two light-crust plates converge, and dense
            // floor subsides (thins) with age away from its spreading boundary.
            let g = gap.min(26) as i32;
            let thickness_m: i32 = match crust {
                Crust::Silica => {
                    if neighbour.is_light() && gap < 5 {
                        // Two light-crust plates converging: a collision root, highest at the suture.
                        46_000 + (5 - gap as i32) * 1_400
                    } else {
                        // A thin margin grading to a thick stable core toward the plate interior.
                        20_000 + g * 1_000
                    }
                }
                // Dense floor subsides with age: thick and shallow near the boundary, thin and deep in the
                // old interior.
                Crust::Olivine => 8_000 - gap.min(16) as i32 * 90,
                Crust::FerroanOlivine => 7_000 - gap.min(16) as i32 * 90,
                Crust::IronOxide => 6_000,
            };

            let elevation = airy_isostatic_elevation(
                density_of(crust),
                mantle_density,
                Fixed::from_int(thickness_m),
            )
            .expect("the isostasy resolves");
            elevations.push(elevation);
            provinces.push(crust.code());
        }
    }

    let datum = relief_datum(&elevations).expect("the field has a datum");

    // Emit: a header, then the elevation grid (metres), province grid, and relief grid, row major.
    let (mut lo, mut hi) = (f64::MAX, f64::MIN);
    for e in &elevations {
        let v = e.to_f64_lossy();
        lo = lo.min(v);
        hi = hi.max(v);
    }
    println!(
        "PLANET {} {} datum={:.1} sea_level={:.1} min={:.1} max={:.1}",
        WIDTH,
        HEIGHT,
        datum.to_f64_lossy(),
        sea_level.to_f64_lossy(),
        lo,
        hi
    );
    println!("ELEV");
    for ty in 0..HEIGHT {
        let row: Vec<String> = (0..WIDTH)
            .map(|tx| format!("{:.0}", elevations[ty * WIDTH + tx].to_f64_lossy()))
            .collect();
        println!("{}", row.join(" "));
    }
    println!("PROVINCE");
    for ty in 0..HEIGHT {
        let row: Vec<String> = (0..WIDTH)
            .map(|tx| provinces[ty * WIDTH + tx].to_string())
            .collect();
        println!("{}", row.join(" "));
    }
    println!("RELIEF");
    for ty in 0..HEIGHT {
        let row: Vec<String> = (0..WIDTH)
            .map(|tx| {
                let r = classify_relief(elevations[ty * WIDTH + tx], sea_level, datum);
                match r {
                    TerrainRelief::Submarine => "0",
                    TerrainRelief::Lowland => "1",
                    TerrainRelief::Upland => "2",
                }
                .to_string()
            })
            .collect();
        println!("{}", row.join(" "));
    }
}
