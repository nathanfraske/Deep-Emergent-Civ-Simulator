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
//! Two bands, seeded near but distant on a generated world, each developing its own language
//! over time. Run with: `cargo run -p civsim-sim --example two_bands`.
//!
//! The world is generated deterministically; two bands of minds are placed a short distance
//! apart on the map, each at its own place and speaking its own articulation system (its own
//! sound inventory). They play the naming game over the semantic primes tick after tick. A
//! band aligns internally because its speakers are co-located, and the two bands diverge
//! because they never share a place, so two distinct tongues emerge for the one shared world
//! of meanings. The run is keyed on a seed and replays identically.

use civsim_core::{Fixed, StableId};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, ConceptId, LangId, Language, LanguageParams};
use civsim_sim::located::{LocationIndex, OccupantId};
use civsim_sim::primes::{nsm_concept_ids, nsm_gloss};
use civsim_sim::tom::AccessWeights;
use civsim_sim::value::RaceId;
use civsim_sim::world::World;
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, TopologySpace, WorldgenParams};

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn main() {
    let seed = 0x0DA2_B0DEu64;

    // A generated world for the bands to live on.
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(
        seed,
        FlatBounded::new(64, 48, 1),
        &biomes,
        &WorldgenParams::dev_default(),
    );

    // Two band homes: near but distant (a few tiles apart), on the map.
    let home_a = Coord3::ground(26, 24);
    let home_b = Coord3::ground(34, 28);
    let dist2 = map.topo().distance2(home_a, home_b);
    let biome = |c: Coord3| map.tile(c).map(|t| biomes.name(t.biome)).unwrap_or("edge");

    let mut w = World::new(params(), params(), AccessWeights::from_pairs([])).with_seed(seed);
    w.set_concepts(nsm_concept_ids());
    w.set_language(LanguageParams {
        innovation_rate: Fixed::from_ratio(1, 200), // a touch of innovation, so tongues keep moving
    });

    // Two languages with distinct sound inventories, so the bands sound different from the start.
    let (sub_a, forms_a) = ArticulationSubstrate::syllabic(
        ["ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo"].map(String::from),
        2,
        3,
    );
    let (sub_b, forms_b) = ArticulationSubstrate::syllabic(
        ["zu", "gu", "vi", "ry", "da", "fo", "el", "un"].map(String::from),
        2,
        3,
    );
    w.add_language(Language::new(LangId(1), RaceId(1), forms_a));
    w.add_language(Language::new(LangId(2), RaceId(2), forms_b));

    // Seed the two bands: place A at home A speaking language 1, place B at home B speaking 2.
    let mut located = LocationIndex::new();
    let band = |w: &mut World,
                located: &mut LocationIndex,
                place: u32,
                lang: LangId,
                home: Coord3,
                n: usize|
     -> Vec<StableId> {
        (0..n)
            .map(|i| {
                let m = w.spawn(Fixed::ONE);
                w.set_place(m, place);
                w.set_language_of(m, lang);
                // Place the person on the map beside their band's home.
                located.place(
                    OccupantId::being(m),
                    Coord3::ground(home.x + (i as i32 % 2), home.y + (i as i32 / 2)),
                );
                m
            })
            .collect()
    };
    let band_a = band(&mut w, &mut located, 1, LangId(1), home_a, 5);
    let band_b = band(&mut w, &mut located, 2, LangId(2), home_b, 5);

    let concepts = nsm_concept_ids();
    let converged = |w: &World, b: &[StableId]| {
        concepts.iter().all(|&c| {
            let first = w.word_for(b[0], c);
            first.is_some() && b.iter().all(|&m| w.word_for(m, c) == first)
        })
    };
    // The fraction of concepts on which the two bands use a different word (their divergence).
    let divergence = |w: &World| -> (usize, usize) {
        let mut diff = 0;
        let mut both = 0;
        for &c in &concepts {
            if let (Some(wa), Some(wb)) = (w.word_for(band_a[0], c), w.word_for(band_b[0], c)) {
                both += 1;
                if sub_a.render(&wa) != sub_b.render(&wb) {
                    diff += 1;
                }
            }
        }
        (diff, both)
    };

    println!("Two bands on a {}x{} world, seed {seed:#x}.", 64, 48);
    println!(
        "  band A at ({}, {}) on {}, band B at ({}, {}) on {}; {} tiles apart (squared distance {}).",
        home_a.x, home_a.y, biome(home_a), home_b.x, home_b.y, biome(home_b),
        (dist2 as f64).sqrt() as i64, dist2
    );
    println!(
        "  they share {} meanings (the semantic primes) but each speaks its own sound system.\n",
        concepts.len()
    );

    // Run over time, watching each band settle its own words and the two tongues diverge.
    let mut ticks = 0u64;
    let checkpoints = [50u64, 200, 800, 4000];
    let mut ci = 0;
    while ticks < 6000 && !(converged(&w, &band_a) && converged(&w, &band_b)) {
        w.tick(&[]);
        ticks += 1;
        if ci < checkpoints.len() && ticks == checkpoints[ci] {
            let (diff, both) = divergence(&w);
            println!(
                "  tick {ticks:>4}: band A settled {}/{} words, band B {}/{}; of {both} shared, {diff} differ",
                settled(&w, &band_a, &concepts), concepts.len(),
                settled(&w, &band_b, &concepts), concepts.len()
            );
            ci += 1;
        }
    }

    let (diff, both) = divergence(&w);
    println!(
        "\nAfter {ticks} ticks: both bands settled; of {both} shared meanings, {diff} have different words ({}%).",
        if both > 0 { diff * 100 / both } else { 0 }
    );
    println!("\nThe same meanings, two emergent tongues (English gist, band A word, band B word):");
    for &c in concepts.iter().take(12) {
        let gloss = nsm_gloss(c).unwrap_or("?");
        let wa = w
            .word_for(band_a[0], c)
            .map(|x| sub_a.render(&x))
            .unwrap_or_else(|| "-".into());
        let wb = w
            .word_for(band_b[0], c)
            .map(|x| sub_b.render(&x))
            .unwrap_or_else(|| "-".into());
        println!("  {gloss:<14} {wa:<10} {wb}");
    }
    println!(
        "\n{} people placed on the map across the two homes; the tongues are theirs, and they diverge because the bands are apart.",
        located.len()
    );

    // The reproducibility anchors: the deterministic fingerprints of this run, so the canonical
    // record can be verified to replay bit for bit.
    let mut digest = civsim_core::StateHasher::new();
    for (b, sub) in [(&band_a, &sub_a), (&band_b, &sub_b)] {
        for &c in &concepts {
            if let Some(word) = w.word_for(b[0], c) {
                digest.write_bytes(sub.render(&word).as_bytes());
            }
        }
    }
    println!("\n== canonical fingerprints ==");
    println!("  seed                {seed:#018x}");
    println!("  world (map) hash    {:032x}", map.state_hash());
    println!("  clock (ticks)       {}", w.clock());
    println!(
        "  event-log length    {}  (hash {:032x})",
        w.events().len(),
        w.event_log_hash()
    );
    println!("  lexicon digest      {:032x}", digest.finish());
}

/// How many concepts a band has settled a shared word for.
fn settled(w: &World, band: &[StableId], concepts: &[ConceptId]) -> usize {
    concepts
        .iter()
        .filter(|&&c| {
            let first = w.word_for(band[0], c);
            first.is_some() && band.iter().all(|&m| w.word_for(m, c) == first)
        })
        .count()
}
