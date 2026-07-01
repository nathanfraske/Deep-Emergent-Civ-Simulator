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

//! The structured anatomy of a generated creature (design Part 25.14, grounded in Part 35's
//! data-defined body model, Part 20's temperament, and Part 34's magic).
//!
//! A creature is not a flat vector of magnitudes: it has a body plan of typed parts drawn
//! from data-defined registries (which natural weapons it bears, what it is clad in, which
//! senses it has, how it moves) and a temperament personality, both sampled at generation and
//! heritable. The registries are the fixed mechanism; the membership (claws, teeth, horns, and
//! the magical kinds) is data that grows with the world, sibling to the biome, value, and
//! trait-axis substrates (Principle 11). Magical kinds are `fantasy` and are drawn only when
//! the [`WorldProfile`] carries magic, so a grounded world (Mirror, Tempest) generates none
//! and a magical world (Arcanum, Confluence) does (Part 34, the test worlds). The full
//! per-part Part 35 body (tissues, wounds, fluids) is the promoted-tier form built from this
//! plan; the wound model and the sentient build stay the open R-WOUND and R-BUILD-PHYS.

use civsim_core::{Fixed, Rng};

/// A body-plan trait kind (a natural weapon, a covering, a sense, or a locomotion mode). The
/// membership of each registry is data; `fantasy` gates a kind on a magical world profile.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindDef {
    pub id: u16,
    pub name: String,
    pub fantasy: bool,
}

/// The data-defined registries a body plan is drawn from. A development fixture supplies a
/// starting real set plus the magical kinds; a TOML loader is the next increment, the way the
/// biome set grew.
#[derive(Clone, Debug)]
pub struct BodyPlanRegistry {
    pub weapons: Vec<KindDef>,
    pub coverings: Vec<KindDef>,
    pub senses: Vec<KindDef>,
    pub locomotion: Vec<KindDef>,
}

fn defs(entries: &[(&str, bool)]) -> Vec<KindDef> {
    entries
        .iter()
        .enumerate()
        .map(|(i, &(name, fantasy))| KindDef {
            id: i as u16,
            name: name.to_string(),
            fantasy,
        })
        .collect()
}

impl BodyPlanRegistry {
    /// A labelled DEVELOPMENT FIXTURE registry: the real natural-body kinds plus a set of
    /// magical ones (gated on a magic world profile). Not owner-reserved canon; the
    /// authoritative membership is a data choice, and the magical kinds are the author's
    /// Principle-9 affordance for the high-magic worlds.
    pub fn dev_default() -> BodyPlanRegistry {
        BodyPlanRegistry {
            weapons: defs(&[
                ("claws", false),
                ("teeth", false),
                ("horns", false),
                ("antlers", false),
                ("tusks", false),
                ("sting", false),
                ("beak", false),
                ("spines", false),
                ("talons", false),
                ("mandibles", false),
                // Magical (Part 34, gated on a magic profile).
                ("mana-lash", true),
                ("curse-touch", true),
                ("ember-breath", true),
                ("frost-fang", true),
            ]),
            coverings: defs(&[
                ("bare hide", false),
                ("fur", false),
                ("feathers", false),
                ("scales", false),
                ("chitin carapace", false),
                ("bony plates", false),
                ("shell", false),
                // Magical.
                ("mana-ward", true),
                ("stone-skin", true),
                ("phase-hide", true),
            ]),
            senses: defs(&[
                ("vision", false),
                ("smell", false),
                ("hearing", false),
                ("vibration", false),
                ("echolocation", false),
                ("electroreception", false),
                // Magical.
                ("mana-sight", true),
                ("aura-sense", true),
            ]),
            locomotion: defs(&[
                ("rooted", false), // sessile: the mark of a plant, not a locomotion mode as such
                ("walk", false),
                ("run", false),
                ("climb", false),
                ("swim", false),
                ("fly", false),
                ("glide", false),
                ("burrow", false),
                ("slither", false),
                // Magical.
                ("levitate", true),
                ("blink", true),
            ]),
        }
    }

    /// The name of a kind in a registry list, or `"?"` if unknown.
    pub fn name(list: &[KindDef], id: u16) -> &str {
        list.iter().find(|k| k.id == id).map_or("?", |k| k.name.as_str())
    }
}

/// A world profile that gates content: whether magic is present (from the test worlds, Part
/// 34). Mirror and Tempest carry no magic; Arcanum and Confluence do.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct WorldProfile {
    pub magic: bool,
}

impl WorldProfile {
    /// A grounded world (no magic): Mirror, Tempest.
    pub fn grounded() -> WorldProfile {
        WorldProfile { magic: false }
    }
    /// A magical world: Arcanum, Confluence.
    pub fn magical() -> WorldProfile {
        WorldProfile { magic: true }
    }
}

/// A temperament personality, the Part 20 palette: each axis in `[0, ONE]`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Temperament {
    /// 0 shy, ONE bold.
    pub boldness: Fixed,
    /// 0 cautious, ONE exploratory.
    pub exploration: Fixed,
    /// 0 sluggish, ONE active.
    pub activity: Fixed,
    /// 0 solitary, ONE social.
    pub sociability: Fixed,
    /// 0 placid, ONE aggressive.
    pub aggression: Fixed,
}

/// A part a creature bears: which kind (a registry id) and how developed it is `[0, ONE]`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Part {
    pub kind: u16,
    pub development: Fixed,
}

/// A creature's structured body plan (design 25.14): the scalar life-history traits plus the
/// typed parts and the temperament. This is the aggregate-tier anatomy; a promoted individual
/// builds a full Part 35 body from it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BodyPlan {
    /// Body mass, the master size trait.
    pub body_mass: Fixed,
    /// Encephalization, the intelligence axis that gates a mind.
    pub encephalization: Fixed,
    /// Diet breadth: 0 specialist, ONE generalist.
    pub diet_breadth: Fixed,
    /// The natural weapons it bears (kind and development).
    pub weapons: Vec<Part>,
    /// Its primary covering (kind and thickness).
    pub covering: Part,
    /// The senses it has (kind and acuity).
    pub senses: Vec<Part>,
    /// The locomotion modes it moves by (registry ids).
    pub locomotion: Vec<u16>,
    /// Its temperament personality.
    pub temperament: Temperament,
}

/// The gated kinds of a registry list for a profile (real always, fantasy only under magic).
fn gated(list: &[KindDef], profile: WorldProfile) -> Vec<&KindDef> {
    list.iter().filter(|k| !k.fantasy || profile.magic).collect()
}

/// Pick `n` distinct kinds from a gated registry list, each with a development drawn from the
/// rng, keyed off a base counter so the picks are reproducible and do not collide.
fn pick(rng: &Rng, base: u64, list: &[KindDef], profile: WorldProfile, n: usize) -> Vec<Part> {
    let pool = gated(list, profile);
    if pool.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut out: Vec<Part> = Vec::new();
    for slot in 0..n {
        let idx = rng.range_u32(base + slot as u64 * 2, pool.len() as u32) as usize;
        let kind = pool[idx].id;
        if out.iter().any(|p| p.kind == kind) {
            continue; // a repeat pick collapses, so a creature can bear fewer than n
        }
        out.push(Part {
            kind,
            development: rng.unit_fixed(base + slot as u64 * 2 + 1),
        });
    }
    out
}

/// Sample a creature's body plan for its trophic layer, from the registries and gated by the
/// world profile. A `sessile` organism (a producer, a plant) is rooted and bears few weapons
/// (structural defenses); a mobile one (a consumer, an animal) bears more weapons at higher
/// trophic layers and moves by one or two mobile modes. Keyed off the `base` counter of the
/// species' sample draw, so a species' anatomy is a reproducible point over the registries.
pub fn sample_body_plan(
    rng: &Rng,
    layer: u16,
    sessile: bool,
    reg: &BodyPlanRegistry,
    profile: WorldProfile,
    base: u64,
) -> BodyPlan {
    let body_mass = rng.unit_fixed(base);
    let encephalization = rng.unit_fixed(base + 1);
    let diet_breadth = rng.unit_fixed(base + 2);
    let temperament = Temperament {
        boldness: rng.unit_fixed(base + 3),
        exploration: rng.unit_fixed(base + 4),
        activity: rng.unit_fixed(base + 5),
        sociability: rng.unit_fixed(base + 6),
        aggression: rng.unit_fixed(base + 7),
    };
    // Weapon count: a sessile producer bears at most one (structural, like spines); a mobile
    // consumer bears more at higher trophic layers.
    let want_weapons = if sessile { (rng.range_u32(base + 19, 2)) as usize } else { (layer as usize).min(3) };
    let weapons = pick(rng, base + 20, &reg.weapons, profile, want_weapons);
    // One primary covering (always present; real coverings include bare hide).
    let covering = pick(rng, base + 40, &reg.coverings, profile, 1)
        .into_iter()
        .next()
        .unwrap_or(Part {
            kind: 0,
            development: Fixed::from_ratio(1, 2),
        });
    // One to three senses.
    let want_senses = 1 + (rng.range_u32(base + 60, 3)) as usize;
    let senses = pick(rng, base + 62, &reg.senses, profile, want_senses);
    // Locomotion: a sessile organism is rooted (the plant mark, registry id 0); a mobile one
    // moves by one or two of the mobile modes (the non-rooted kinds).
    let locomotion = if sessile {
        vec![0]
    } else {
        let mobile: Vec<KindDef> = reg.locomotion.iter().filter(|k| k.id != 0).cloned().collect();
        let want_loco = 1 + (rng.range_u32(base + 80, 2)) as usize;
        pick(rng, base + 82, &mobile, profile, want_loco)
            .into_iter()
            .map(|p| p.kind)
            .collect()
    };
    BodyPlan {
        body_mass,
        encephalization,
        diet_breadth,
        weapons,
        covering,
        senses,
        locomotion,
        temperament,
    }
}

/// A plain-language word for a normalised temperament axis, low to high, for the inspector.
pub fn temperament_word(boldness: Fixed) -> &'static str {
    if boldness < Fixed::from_ratio(1, 3) {
        "shy"
    } else if boldness < Fixed::from_ratio(2, 3) {
        "wary"
    } else {
        "bold"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_core::{DrawKey, Phase};

    fn rng() -> Rng {
        DrawKey::entity(42, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1)
    }

    #[test]
    fn a_grounded_profile_bears_no_magical_kinds() {
        let reg = BodyPlanRegistry::dev_default();
        let plan = sample_body_plan(&rng(), 2, false, &reg, WorldProfile::grounded(), 200);
        for w in &plan.weapons {
            assert!(!reg.weapons.iter().find(|k| k.id == w.kind).unwrap().fantasy, "no magic weapon in a grounded world");
        }
        assert!(!reg.coverings.iter().find(|k| k.id == plan.covering.kind).unwrap().fantasy);
    }

    #[test]
    fn a_predator_bears_more_weapons_than_a_plant() {
        let reg = BodyPlanRegistry::dev_default();
        let plant = sample_body_plan(&rng(), 0, true, &reg, WorldProfile::grounded(), 200);
        let predator = sample_body_plan(&rng(), 3, false, &reg, WorldProfile::grounded(), 200);
        assert!(plant.weapons.len() <= 1, "a plant bears at most one weapon");
        assert!(predator.weapons.len() >= plant.weapons.len(), "a predator bears more");
    }

    #[test]
    fn a_body_plan_is_deterministic() {
        let reg = BodyPlanRegistry::dev_default();
        let a = sample_body_plan(&rng(), 2, false, &reg, WorldProfile::magical(), 200);
        let b = sample_body_plan(&rng(), 2, false, &reg, WorldProfile::magical(), 200);
        assert_eq!(a, b, "same key, same body plan");
        assert!(!a.senses.is_empty() && !a.locomotion.is_empty(), "a creature has senses and moves");
    }

    #[test]
    fn a_magical_world_can_bear_magical_kinds() {
        // Over many species, a magical world should produce at least one fantasy part.
        let reg = BodyPlanRegistry::dev_default();
        let mut saw_magic = false;
        for s in 0..80u64 {
            let r = DrawKey::entity(s, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1);
            let plan = sample_body_plan(&r, 3, false, &reg, WorldProfile::magical(), 200);
            let magic_weapon = plan
                .weapons
                .iter()
                .any(|w| reg.weapons.iter().find(|k| k.id == w.kind).unwrap().fantasy);
            let magic_cover = reg.coverings.iter().find(|k| k.id == plan.covering.kind).unwrap().fantasy;
            if magic_weapon || magic_cover {
                saw_magic = true;
                break;
            }
        }
        assert!(saw_magic, "a magical world produces magical anatomy");
    }
}
