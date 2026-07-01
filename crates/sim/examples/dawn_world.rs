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

//! QUARANTINED DEV-FIXTURE HARNESS (not canonical). This example uses authored, dev-fixture numbers
//! (calibrations, seeds, scenario values) to produce a result for demonstration and testing only, and
//! its behaviour is not authoritative (design Principle 11, the reserved-value discipline: an authored
//! constant in the path of world content is a defect until it earns its place). The canonical runner
//! is manifest-driven and fail-loud with zero unapproved authored features; see docs/QUARANTINE.md.
//!
//! The whole thread so far in one scene: a generated world, a band seeded onto a
//! habitable cell of it, and the naming game played there over the semantic primes. Run
//! with: `cargo run -p civsim-sim --example dawn_world`.
//!
//! It generates a map (the M1 worldgen), places a band on the land cell nearest the
//! centre, runs the naming game over all sixty-five NSM primes until the band shares a
//! word for each, then prints the map with the band marked and a sample of the band's
//! first lexicon. Every number is a labelled fixture, never an owner value, and the run
//! replays identically from its seed. This is the lean placement bridge: co-location is
//! still a place token, with the deep being model (genome, axioms, race parameters) and a
//! place-as-map-coordinate still ahead (roadmap M2).

use civsim_core::{Fixed, StableId};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::primes::{nsm_concept_ids, nsm_gloss};
use civsim_sim::tom::AccessWeights;
use civsim_sim::world::World;
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, TopologySpace, WorldgenParams};

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

const SYLLABLES: [&str; 12] = [
    "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
];

fn main() {
    let seed = 0xDA1C0u64;

    // 1. Generate the world (M1).
    let topo = FlatBounded::new(64, 28, 1);
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(seed, topo, &biomes, &WorldgenParams::dev_default());

    // 2. Find a habitable cell: the land cell (not ocean or coast) nearest the centre,
    //    tie-broken by canonical coordinate order, so the choice is deterministic.
    let centre = Coord3::ground(topo.width / 2, topo.height / 2);
    let mut home: Option<(i64, Coord3)> = None;
    for y in 0..topo.height {
        for x in 0..topo.width {
            let c = Coord3::ground(x, y);
            let name = biomes.name(map.tile(c).unwrap().biome);
            if name != "ocean" && name != "coast" {
                let d = topo.distance2(c, centre);
                if home.is_none_or(|(bd, _)| d < bd) {
                    home = Some((d, c));
                }
            }
        }
    }
    let home = home.expect("the generated world has habitable land").1;
    let home_biome = biomes.name(map.tile(home).unwrap().biome);

    // 3. Seed a band on that cell and play the naming game over the primes.
    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_concepts(nsm_concept_ids());
    let (substrate, forms) = ArticulationSubstrate::syllabic(SYLLABLES.map(String::from), 2, 3);
    w.set_form_system(forms);
    w.set_language(LanguageParams {
        innovation_rate: Fixed::ZERO,
    });
    let band: Vec<StableId> = (0..5).map(|_| w.spawn(Fixed::ONE)).collect();
    // The place token is the home cell's tile index, so the band's place is its map cell.
    let place = home.y as u32 * topo.width as u32 + home.x as u32;
    for &m in &band {
        w.set_place(m, place);
    }

    let concepts = nsm_concept_ids();
    let converged = |w: &World| {
        concepts.iter().all(|&c| {
            let first = w.word_for(band[0], c);
            first.is_some() && band.iter().all(|&m| w.word_for(m, c) == first)
        })
    };
    let mut ticks = 0;
    while ticks < 4000 && !converged(&w) {
        w.tick(&[]);
        ticks += 1;
    }

    // 4. Show the world with the band marked at its home cell.
    println!(
        "A generated world ({}x{}, seed {seed:#x}) with a band of {} seeded on it.",
        topo.width,
        topo.height,
        band.len()
    );
    println!(
        "The band settled the nearest habitable land to the centre: {home_biome} at ({}, {}), marked @.\n",
        home.x, home.y
    );
    for y in 0..topo.height {
        let mut line = String::with_capacity(topo.width as usize);
        for x in 0..topo.width {
            let c = Coord3::ground(x, y);
            if c == home {
                line.push('@');
            } else {
                line.push(biomes.glyph(map.tile(c).unwrap().biome));
            }
        }
        println!("{line}");
    }

    // 5. The band's first words for a sample of the primes.
    println!(
        "\nAfter {ticks} ticks the band shares a word for every prime: {}",
        converged(&w)
    );
    println!("A sample of its first lexicon (English gist, coined word):");
    for &c in concepts.iter().take(12) {
        let gloss = nsm_gloss(c).unwrap_or("?");
        let word = w
            .word_for(band[0], c)
            .map(|word| substrate.render(&word))
            .unwrap_or_else(|| "-".to_string());
        println!("  {gloss:<10} {word}");
    }
    println!("  ... and {} more primes.", concepts.len() - 12);

    println!("\nDeterminism: the map and the band both replay from the seed {seed:#x}.");
    println!("  map state hash: {:032x}", map.state_hash());
    println!("  world state hash: {:032x}", w.state_hash());
}
