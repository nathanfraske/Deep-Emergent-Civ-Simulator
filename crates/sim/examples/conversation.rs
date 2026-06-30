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

//! A watchable conversation: a promoted band grounds a shared belief through modelled
//! dialogue, and the move log is narrated. Run with:
//! `cargo run -p civsim-sim --example conversation`.
//!
//! Ada has seen where the water is. Over a few ticks the promoted band talks: each move is
//! a first-class event (an assertion, an agreement), and the conversation is reassembled
//! from the log rather than stored. The scene surfaces the gating factors the design
//! names: the content gate the substrate passes at load, co-location and channel reach,
//! felicity (a command misfires with no role to back it), promotion to move-by-move
//! fidelity, and grounding as said evidence with no manufactured common ground. Every
//! number here is a labelled fixture, never an owner value, and the run replays from its
//! seed.
//!
//! The canonical record is the move sequence and its consequences. Each move is printed
//! twice: the deterministic gist (the gloss handle and the content the engine holds), and
//! a clearly marked non-canon flavor line, the optional interpretation layer dressing real
//! moves in a voice. The flavor invents no move and changes nothing.

use civsim_core::{Fixed, StableId};
use civsim_sim::dialogue::{
    conversation_of, ContentRef, EffectSign, FelicityCond, ForceEffectDef, ForceEffectId,
    ForceFloor, ForceKind, Move, MoveKindDef, MoveKindId, MoveRegistry, ResolvedBand,
};
use civsim_sim::evidence::InferenceParams;
use civsim_sim::tom::{AccessChannelDef, AccessChannelId, AccessChannelRegistry, AccessWeights};
use civsim_sim::world::{GossipParams, Stimulus, TickInput, World};
use civsim_sim::AttrKindId;

const WITNESSED: AccessChannelId = AccessChannelId(1);
const SAID: AccessChannelId = AccessChannelId(3);
const WATER: AttrKindId = AttrKindId(0);
const WATERSHED: StableId = StableId(99);
const SPRING: u32 = 10;
const WASH: u32 = 20;

fn params() -> InferenceParams {
    InferenceParams {
        clamp: Fixed::from_int(50),
        commit_threshold: Fixed::from_int(3),
        margin: Fixed::from_int(1),
    }
}

fn place_name(v: u32) -> &'static str {
    match v {
        SPRING => "the north spring",
        WASH => "the dry wash",
        _ => "somewhere",
    }
}

/// The starter dialogue substrate: the etic force floor and a small recognised repertoire.
/// Membership is data; the glosses are the legibility handles, never read by the force.
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
            ForceEffectDef {
                id: ForceEffectId(4),
                kind: ForceKind::RaiseInquiry,
                sign: EffectSign::Neutral,
                name: "ask".to_string(),
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
            MoveKindDef {
                id: MoveKindId(4),
                name: "question".to_string(),
                force: vec![ForceEffectId(4)],
                expects: vec![MoveKindId(1)],
                sincerity_judged: false,
                felicity: vec![],
                gloss: "asks".to_string(),
            },
        ],
    };
    (floor, registry)
}

fn band_world() -> World {
    let mut w = World::new(
        params(),
        params(),
        AccessWeights::from_pairs([(WITNESSED, Fixed::from_int(4)), (SAID, Fixed::from_int(2))]),
    )
    .with_seed(0x00C0_FFEE);
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
    let (floor, registry) = substrate();
    w.set_dialogue(registry, floor)
        .expect("the dialogue substrate passes the content gate");
    w
}

fn main() {
    let names = ["Ada", "Bero", "Cael", "Zev"];
    let name_of =
        |id: StableId| -> &'static str { names.get(id.0 as usize).copied().unwrap_or("?") };
    let (_floor, registry) = substrate();

    println!("A band at a watering ground. Ada has seen where the water is.\n");
    println!("Content gate: the move substrate loaded, so no move kind or felicity");
    println!("condition smuggles a graded persuasion or fidelity weight.\n");

    let mut w = band_world();
    let band: Vec<StableId> = (0..3).map(|_| w.spawn(Fixed::ONE)).collect(); // Ada, Bero, Cael
    let zev = w.spawn(Fixed::ONE);
    for &m in &band {
        w.set_place(m, 1); // the watering ground
        w.promote(m); // promoted to move-by-move dialogue
    }
    w.set_place(zev, 2); // a ridge away, out of earshot
    w.promote(zev);

    println!(
        "Co-location: {}, {}, {} share the watering ground (place 1); {} is a ridge away (place 2).",
        name_of(band[0]),
        name_of(band[1]),
        name_of(band[2]),
        name_of(zev),
    );
    println!("Promotion: all four are promoted to move-by-move dialogue.");

    // Ada has seen the water; Bero and Cael wonder where it is but cannot answer, so they
    // will ask. Curiosity is an inquiry goal (design 9.13), seeded here.
    w.set_wondering(band[1], WATERSHED, WATER);
    w.set_wondering(band[2], WATERSHED, WATER);
    println!(
        "Curiosity: {} and {} wonder where the water is; only {} has seen it.\n",
        name_of(band[1]),
        name_of(band[2]),
        name_of(band[0]),
    );

    // Ada witnesses the water at the north spring; the band talks over the next ticks.
    w.tick(&[TickInput {
        mind: band[0],
        ordinal: 0,
        stim: Stimulus::Observe {
            subject: WATERSHED,
            attr: WATER,
            hyps: vec![SPRING, WASH],
            toward: SPRING,
            weight: Fixed::from_int(5),
            from: band[0],
        },
    }]);
    // Run until the talk falls silent (no new moves), so the convergence is visible.
    let mut prev = w.events().len();
    let mut quiet = 0;
    let mut ticks = 0;
    while ticks < 40 && quiet < 3 {
        w.tick(&[]);
        ticks += 1;
        let now = w.events().len();
        if now == prev {
            quiet += 1;
        } else {
            quiet = 0;
        }
        prev = now;
    }

    // Narrate the move log: the canonical gist, then a non-canon flavor line.
    let bp = *w.belief_params();
    println!("The talk, move by move (canonical gist, then non-canon flavor):\n");
    for e in w.events().iter() {
        let mv = match Move::from_event(e) {
            Some(m) => m,
            None => continue,
        };
        let gloss = registry
            .move_kind(mv.force)
            .map(|m| m.gloss.as_str())
            .unwrap_or("?");
        let speaker = name_of(mv.speaker);
        let addressees: Vec<&str> = mv.addressees.iter().map(|a| name_of(*a)).collect();
        let to = addressees.join(", ");
        let (gist, flavor) = match mv.content {
            ContentRef::Belief { subject, attr } => {
                let val = w
                    .mind(mv.speaker)
                    .and_then(|m| m.belief(subject, attr, &bp));
                let where_ = val.map(place_name).unwrap_or("somewhere");
                (
                    format!("  {speaker} {gloss} {to}: the water is at {where_}."),
                    format!(
                        "    (flavor) {speaker} points past the rocks, miming the cold rush of {where_}."
                    ),
                )
            }
            ContentRef::Inquiry { .. } => (
                format!("  {speaker} {gloss} {to}: where is the water?"),
                format!("    (flavor) {speaker} spreads empty hands toward {to}."),
            ),
            ContentRef::PriorMove { .. } => {
                let line = format!("  {speaker} {gloss}.");
                let flav = if gloss == "agrees" {
                    format!("    (flavor) {speaker} nods, already turning toward the water.")
                } else {
                    format!("    (flavor) {speaker} frowns and stays put.")
                };
                (line, flav)
            }
            _ => (format!("  {speaker} {gloss}."), String::new()),
        };
        println!("{gist}");
        if !flavor.is_empty() {
            println!("{flavor}");
        }
    }
    println!("\nThe talk fell silent after {ticks} ticks: once each models the others as",);
    println!("knowing, there is nothing left to tell and no open question, so it stops.");

    // The conversation is a query over the log, not a stored object.
    if let Some(first) = w.events().iter().next().map(|e| e.id) {
        if let Some(conv) = conversation_of(w.events(), first, 100) {
            let who: Vec<&str> = conv.participants.iter().map(|p| name_of(*p)).collect();
            println!(
                "\nReassembled from the log as one conversation: {} moves among {}.",
                conv.event_ids.len(),
                who.join(", ")
            );
        }
    }

    // Where the belief landed.
    println!("\nWhere the belief landed:");
    for &m in &band {
        let b = w
            .mind(m)
            .and_then(|mind| mind.belief(WATERSHED, WATER, &bp));
        println!(
            "  {:<5} believes the water is at {}",
            name_of(m),
            b.map(place_name).unwrap_or("nowhere yet")
        );
    }
    let z = w
        .mind(zev)
        .and_then(|mind| mind.belief(WATERSHED, WATER, &bp));
    println!(
        "  {:<5} (a ridge away) believes the water is at {}  <- co-location gate: heard nothing",
        name_of(zev),
        z.map(place_name).unwrap_or("nowhere yet")
    );

    // Grounding as said evidence: Ada comes to model a bandmate as agreeing, purely from
    // their acceptances over the said channel, with no co-witnessed common-ground prior.
    let mp = *w.meta_params();
    let modelled = w
        .mind(band[0])
        .and_then(|m| m.modeled_belief(band[1], WATER, &mp));
    println!(
        "\nGrounding (said evidence, no common ground): {} models {} as believing {}.",
        name_of(band[0]),
        name_of(band[1]),
        modelled.map(place_name).unwrap_or("nothing settled yet"),
    );

    // Felicity gates the act, never its force: a command needs a role to back it.
    let command = MoveKindDef {
        id: MoveKindId(9),
        name: "command".to_string(),
        force: vec![ForceEffectId(1)],
        expects: vec![],
        sincerity_judged: false,
        felicity: vec![FelicityCond {
            dimension: "role.command".to_string(),
            band: "felicity.command.role".to_string(),
        }],
        gloss: "orders".to_string(),
    };
    let role_band = ResolvedBand {
        lo: Fixed::ONE,
        hi: Fixed::from_int(10),
    };
    let bands = |k: &str| (k == "felicity.command.role").then_some(role_band);
    let has_role = command.felicitous(|d| (d == "role.command").then(|| Fixed::from_int(3)), bands);
    let no_role = command.felicitous(|d| (d == "role.command").then_some(Fixed::ZERO), bands);
    println!("\nFelicity gate: a command lands only with a role to back it.");
    println!("  with a commanding role: lands = {has_role}");
    println!("  with no such role:      lands = {no_role}  <- misfires as a bare attempt");

    // Determinism: the same seed reproduces the whole exchange.
    let replay = {
        let mut w2 = band_world();
        let band2: Vec<StableId> = (0..3).map(|_| w2.spawn(Fixed::ONE)).collect();
        let zev2 = w2.spawn(Fixed::ONE);
        for &m in &band2 {
            w2.set_place(m, 1);
            w2.promote(m);
        }
        w2.set_place(zev2, 2);
        w2.promote(zev2);
        w2.tick(&[TickInput {
            mind: band2[0],
            ordinal: 0,
            stim: Stimulus::Observe {
                subject: WATERSHED,
                attr: WATER,
                hyps: vec![SPRING, WASH],
                toward: SPRING,
                weight: Fixed::from_int(5),
                from: band2[0],
            },
        }]);
        for _ in 0..5 {
            w2.tick(&[]);
        }
        w2.state_hash()
    };
    println!("\nDeterminism: the conversation replays from its seed.");
    println!("  state hash this run : {:032x}", w.state_hash());
    println!("  same seed replayed  : {replay:032x}");
    println!("  matches: {}", w.state_hash() == replay);
}
