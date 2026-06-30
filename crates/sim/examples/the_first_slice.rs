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

//! The first end-to-end emergent slice in one run: a generated world, a band seeded onto
//! it, the naming game played over the semantic primes so the band coins its own words,
//! and then an emergent conversation in which one member's witnessed knowledge grounds
//! across the band as first-class move events. Run with:
//! `cargo run -p civsim-sim --example the_first_slice`.
//!
//! This is the milestone the roadmap aims at, stitched from the parts built so far: M1 the
//! map, the lean M2 placement (the band's place is its map cell), M3 the naming game over
//! all sixty-five NSM primes, and M4 modelled dialogue. Every number is a labelled
//! fixture, never an owner value, and the whole run replays bit for bit from its seed.
//!
//! One honest seam is visible here and is the next build (the roadmap's deep M2): the
//! conversation's content is rendered through the deterministic English gist, not the
//! band's coined words, because a belief value is not yet grounded as a concept the naming
//! game named. The band's emergent lexicon is shown alongside; tying a witnessed fact to
//! the word the band coined for it is the `ConceptId`-as-substrate-region work still ahead.

use civsim_core::{Fixed, StableId};
use civsim_sim::dialogue::{
    conversation_of, ContentRef, EffectSign, ForceEffectDef, ForceEffectId, ForceFloor, ForceKind,
    Move, MoveKindDef, MoveKindId, MoveRegistry,
};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::language::{ArticulationSubstrate, LanguageParams};
use civsim_sim::primes::{nsm_concept_ids, nsm_gloss};
use civsim_sim::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_sim::world::{GossipParams, Stimulus, TickInput, World};
use civsim_sim::AttrKindId;
use civsim_world::{BiomeSet, Coord3, FlatBounded, TileMap, TopologySpace, WorldgenParams};

const WITNESSED: AccessChannelId = AccessChannelId(1);
const SAID: AccessChannelId = AccessChannelId(3);
const HERD: StableId = StableId(900);
const RANGE: AttrKindId = AttrKindId(0);
const NORTH: u32 = 10;
const SOUTH: u32 = 20;

const SYLLABLES: [&str; 12] = [
    "ka", "lo", "mi", "tu", "ne", "sa", "ri", "wo", "ha", "du", "pe", "go",
];

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(2),
    }
}

fn range_name(v: u32) -> &'static str {
    match v {
        NORTH => "the north meadows",
        SOUTH => "the south flats",
        _ => "somewhere",
    }
}

/// The starter dialogue substrate: the etic force floor and a small recognised repertoire,
/// content-gated at install. The glosses are legibility handles, never read by the force.
fn substrate() -> (ForceFloor, MoveRegistry) {
    let floor = ForceFloor {
        effects: vec![
            ForceEffectDef {
                id: ForceEffectId(1),
                kind: ForceKind::TellEvidence,
                sign: EffectSign::Neutral,
                name: "assert".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(2),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Positive,
                name: "accept".to_string(),
            },
            ForceEffectDef {
                id: ForceEffectId(3),
                kind: ForceKind::RegisterUptake,
                sign: EffectSign::Negative,
                name: "refuse".to_string(),
            },
        ],
    };
    let registry = MoveRegistry {
        moves: vec![
            MoveKindDef {
                id: MoveKindId(1),
                name: "assertion".to_string(),
                force: vec![ForceEffectId(1)],
                expects: vec![MoveKindId(2), MoveKindId(3)],
                sincerity_judged: true,
                felicity: vec![],
                gloss: "tells".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(2),
                name: "acceptance".to_string(),
                force: vec![ForceEffectId(2)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "agrees".to_string(),
            },
            MoveKindDef {
                id: MoveKindId(3),
                name: "refusal".to_string(),
                force: vec![ForceEffectId(3)],
                expects: vec![],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "doubts it".to_string(),
            },
        ],
    };
    (floor, registry)
}

fn main() {
    let seed = 0x5_11CEu64;

    // 1. Generate the world (M1) and find the habitable land cell nearest the centre.
    let topo = FlatBounded::new(64, 28, 1);
    let biomes = BiomeSet::dev_default();
    let map = TileMap::generate(seed, topo, &biomes, &WorldgenParams::dev_default());
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

    // 2. Build the world and seed a band on the home cell (place = tile index, the lean M2
    //    bridge). Install language (innovation off, for a clean lexicon) and the dialogue
    //    substrate.
    let mut w = World::new(params(), params(), {
        AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (SAID, Fixed::from_int(2))])
    })
    .with_seed(seed);
    w.set_channels(AccessChannelRegistry {
        channels: vec![
            AccessChannelDef {
                id: WITNESSED,
                name: "witnessed".to_string(),
            },
            AccessChannelDef {
                id: SAID,
                name: "said".to_string(),
            },
        ],
    });
    w.set_gossip(GossipParams {
        told_weight: Fixed::from_int(3),
        trust_baseline: Fixed::ONE,
        trust_penalty: Fixed::from_ratio(1, 2),
    });
    w.set_concepts(nsm_concept_ids());
    let (substr, forms) = ArticulationSubstrate::syllabic(SYLLABLES.map(String::from), 2, 3);
    w.set_form_system(forms);
    w.set_language(LanguageParams {
        innovation_rate: Fixed::ZERO,
    });
    let (floor, registry) = substrate();
    w.set_dialogue(registry.clone(), floor)
        .expect("the dialogue substrate passes the content gate");

    let names = [" Syl", "Bavo", "Cira", "Ravn"];
    let name_of = |id: StableId| -> &str { names.get(id.0 as usize).copied().unwrap_or("?") };
    let band: Vec<StableId> = (0..4).map(|_| w.spawn(Fixed::ONE)).collect();
    let place = home.y as u32 * topo.width as u32 + home.x as u32;
    for &m in &band {
        w.set_place(m, place);
    }

    println!(
        "A generated world ({}x{}, seed {seed:#x}). A band of {} settles the nearest",
        topo.width,
        topo.height,
        band.len()
    );
    println!(
        "habitable land to the centre: {home_biome} at ({}, {}), marked @.\n",
        home.x, home.y
    );

    // 3. Phase one: the naming game, until the band shares a word for every prime.
    let concepts = nsm_concept_ids();
    let converged = |w: &World| {
        concepts.iter().all(|&c| {
            let first = w.word_for(band[0], c);
            first.is_some() && band.iter().all(|&m| w.word_for(m, c) == first)
        })
    };
    let mut naming_ticks = 0;
    while naming_ticks < 6000 && !converged(&w) {
        w.tick(&[]);
        naming_ticks += 1;
    }
    println!(
        "After {naming_ticks} ticks the band has coined a shared word for all {} primes.",
        concepts.len()
    );
    println!("A few of its first words (English gist, coined word):");
    for &c in concepts.iter().take(6) {
        let gloss = nsm_gloss(c).unwrap_or("?");
        let word = w
            .word_for(band[0], c)
            .map(|word| substr.render(&word))
            .unwrap_or_else(|| "-".to_string());
        println!("  {gloss:<8} {word}");
    }

    // 4. Phase two: a scout returns having witnessed where the herd ranges, and the band
    //    talks. Promote the band to move-by-move dialogue, then seed the witnessed belief.
    for &m in &band {
        w.promote(m);
    }
    let move_floor = w.events().len();
    w.tick(&[TickInput {
        mind: band[0],
        ordinal: 0,
        stim: Stimulus::Observe {
            subject: HERD,
            attr: RANGE,
            hyps: vec![NORTH, SOUTH],
            toward: NORTH,
            weight: Fixed::from_int(5),
            from: band[0],
        },
    }]);
    for _ in 0..5 {
        w.tick(&[]);
    }

    // 5. Render the map with the band marked.
    println!("\nThe band's home on the generated world:\n");
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

    // 6. Narrate the conversation from the move log (canonical gist).
    let bp = *w.belief_params();
    println!(
        "\n{} returns having seen the herd, and the band talks it over (first moves):",
        name_of(band[0])
    );
    let mut shown = 0;
    for e in w.events().iter() {
        if (e.id.0 as usize) < move_floor {
            continue;
        }
        let mv = match Move::from_event(e) {
            Some(m) => m,
            None => continue,
        };
        if shown >= 8 {
            break;
        }
        let gloss = registry
            .move_kind(mv.force)
            .map(|m| m.gloss.as_str())
            .unwrap_or("?");
        let speaker = name_of(mv.speaker);
        match mv.content {
            ContentRef::Belief { subject, attr } => {
                let val = w
                    .mind(mv.speaker)
                    .and_then(|m| m.belief(subject, attr, &bp));
                let to: Vec<&str> = mv.addressees.iter().map(|a| name_of(*a)).collect();
                println!(
                    "  {speaker} {gloss} {}: the herd ranges in {}.",
                    to.join(", "),
                    val.map(range_name).unwrap_or("somewhere")
                );
            }
            _ => println!("  {speaker} {gloss}."),
        }
        shown += 1;
    }

    if let Some(first) = w
        .events()
        .iter()
        .find(|e| (e.id.0 as usize) >= move_floor)
        .map(|e| e.id)
    {
        if let Some(conv) = conversation_of(w.events(), first, 100) {
            let who: Vec<&str> = conv.participants.iter().map(|p| name_of(*p)).collect();
            println!(
                "\nReassembled from the log as one conversation: {} moves among {}.",
                conv.event_ids.len(),
                who.join(", ")
            );
        }
    }

    // 7. Where the witnessed knowledge landed, and the grounding it built.
    println!("\nWhat the band now believes about the herd:");
    for &m in &band {
        let b = w.mind(m).and_then(|mind| mind.belief(HERD, RANGE, &bp));
        println!(
            "  {:<5} the herd ranges in {}",
            name_of(m),
            b.map(range_name).unwrap_or("nowhere known yet")
        );
    }
    let mp = *w.meta_params();
    let grounded = w
        .mind(band[0])
        .and_then(|m| m.modeled_belief(band[1], RANGE, &mp));
    println!(
        "\nGrounding (said evidence, no common ground): {} models {} as believing {}.",
        name_of(band[0]),
        name_of(band[1]),
        grounded.map(range_name).unwrap_or("nothing settled yet"),
    );

    // 8. Determinism: the whole slice replays from the seed.
    println!("\nDeterminism: map and band both replay from seed {seed:#x}.");
    println!("  map state hash   : {:032x}", map.state_hash());
    println!("  world state hash : {:032x}", w.state_hash());
}
