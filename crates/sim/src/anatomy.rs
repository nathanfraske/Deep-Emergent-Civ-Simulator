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
///
/// A kind carries crude GEOMETRY (form-axis values, `mech.*`) and MATERIAL (mechanical-floor axis
/// values, `mat.*`), the same string-keyed sorted-walk composition shape [`TissueComposition`] uses, so a
/// part's FUNCTION is read from its own physics through the compose function-law dispatch
/// (`civsim_compose::derive_capabilities`) rather than an authored tag (emergent-anatomy arc, step one).
/// These are LABELLED DEVELOPMENT-FIXTURE values grounded in the cited floor axes, not owner canon: a
/// claw is a small-area hard point because that is what makes it a weapon in physics, not because a
/// person tagged it `F_STRIKE`. Growth stays crude in step one (the values are per-kind stand-ins); the
/// developmental program of step two grows them from the genome. A kind that carries no geometry or
/// material simply reads no capability (the zero-for-absent accessor is the natural gate).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KindDef {
    pub id: u16,
    pub name: String,
    pub fantasy: bool,
    /// The crude geometry the kind's part carries, keyed by form/geometry axis id (`mech.contact_area`,
    /// `mech.edge_radius`, and the rest), sorted for a deterministic walk. Absent axis reads zero.
    pub geometry: std::collections::BTreeMap<String, Fixed>,
    /// The crude material the kind's part is made of, keyed by mechanical-floor axis id
    /// (`mat.indentation_hardness`, and the rest), sorted for a deterministic walk. Absent axis reads zero.
    pub material: std::collections::BTreeMap<String, Fixed>,
}

impl KindDef {
    /// The kind's value on a geometry axis, or zero if it carries none (the substrate absence convention).
    pub fn geo(&self, axis: &str) -> Fixed {
        self.geometry.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// The kind's value on a material axis, or zero if it carries none.
    pub fn mat(&self, axis: &str) -> Fixed {
        self.material.get(axis).copied().unwrap_or(Fixed::ZERO)
    }
}

/// An organ's tissue composition: its value on each biology-floor material axis, keyed by the floor
/// axis id (`bio.energy_density`, `bio.water_fraction`, `bio.protein_fraction`, and the rest the floor
/// declares in `crates/physics/data/biology_floor.toml`). This is the same string-keyed, sorted-walk
/// composition-over-the-floor shape the physics substrate's `Substance::vector` uses (`crates/physics`),
/// so the component vocabulary is the floor's DATA and grows with it at zero code cost (Principle 11):
/// a reserve backed by protein or a toxin-load tissue is a data edit, not an enum change. The organ's
/// physiological function is NOT tagged: which reserve it backs is DERIVED from this composition against
/// the floor (an energy-dense tissue is an energy store, a water-rich one a water store), so nobody
/// authors "fat-body is metabolic"; the mechanism reads the composition. A respiratory-surface axis
/// (R-MEDIUM) enters here as another floor axis id, again with no code change. Labelled development-
/// fixture compositions here, grounded in the cited floor axes, not owner canon.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TissueComposition {
    /// The value on each biology-floor axis, keyed by axis id and sorted for a deterministic walk (the
    /// `Substance::vector` convention). An axis the organ bears none of is simply absent.
    pub components: std::collections::BTreeMap<String, Fixed>,
}

impl TissueComposition {
    /// The organ's value on one composition component, named by its biology-floor axis id. An absent
    /// axis reads as zero (the organ bears none of that tissue), the substrate's absence convention.
    pub fn component(&self, axis: &str) -> Fixed {
        self.components.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// Build a composition from (biology-floor axis id, value) pairs.
    pub fn from_pairs(pairs: &[(&str, Fixed)]) -> TissueComposition {
        TissueComposition {
            components: pairs.iter().map(|&(a, v)| (a.to_string(), v)).collect(),
        }
    }
}

/// One organ kind as data: an id, a legibility name, a fantasy gate, and its tissue composition. The
/// name is cosmetic; the mechanism reads only the composition, from which the organ's reserve-backing
/// function is derived. Membership is data and grows with the world (Principle 11); the composition is
/// grounded in the biology-floor material axes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct OrganKindDef {
    pub id: u16,
    pub name: String,
    pub fantasy: bool,
    pub composition: TissueComposition,
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
    /// The organ kinds a body may bear, each a tissue composition. Which reserve an organ backs is
    /// derived from its composition against the floor, never a tag.
    pub organs: Vec<OrganKindDef>,
}

/// One kind development-fixture entry with geometry and material: (name, fantasy, geometry pairs as
/// (form-axis id, decimal), material pairs as (mat-axis id, decimal)). The geometry and material are the
/// crude per-kind physics a part's function is derived from; labelled fixtures grounded in the floor axes.
type KindFixture<'a> = (
    &'a str,
    bool,
    &'a [(&'a str, &'a str)],
    &'a [(&'a str, &'a str)],
);

/// Build kinds carrying crude geometry and material, so a part's function is a physics read (step one).
/// The decimal literals are parsed the same exact way the compose form fixtures are; a malformed literal
/// is a programming error in the fixture, never runtime input.
fn kinds(entries: &[KindFixture]) -> Vec<KindDef> {
    entries
        .iter()
        .enumerate()
        .map(|(i, &(name, fantasy, geo, mat))| KindDef {
            id: i as u16,
            name: name.to_string(),
            fantasy,
            geometry: geo.iter().map(|&(a, v)| (a.to_string(), dec(v))).collect(),
            material: mat.iter().map(|&(a, v)| (a.to_string(), dec(v))).collect(),
        })
        .collect()
}

/// A decimal-string to `Fixed` for the labelled kind fixtures. Panics on a malformed literal (a fixture
/// programming error, never runtime input).
fn dec(s: &str) -> Fixed {
    Fixed::from_decimal_str(s).expect("kind-fixture decimal literal")
}

/// One development-fixture organ entry: (name, fantasy gate, composition as (biology-floor axis id,
/// value) pairs).
type OrganFixture<'a> = (&'a str, bool, &'a [(&'a str, Fixed)]);

/// Build organ kinds from (name, fantasy, composition-pairs) tuples, each pair a (biology-floor axis
/// id, value). The compositions are LABELLED DEVELOPMENT FIXTURE values grounded in the cited floor
/// axes, not owner canon.
fn organ_defs(entries: &[OrganFixture]) -> Vec<OrganKindDef> {
    entries
        .iter()
        .enumerate()
        .map(|(i, &(name, fantasy, comp))| OrganKindDef {
            id: i as u16,
            name: name.to_string(),
            fantasy,
            composition: TissueComposition::from_pairs(comp),
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
            // Weapons carry crude PIERCE geometry (a small contact area, a sharp edge radius) and material
            // (indentation hardness), so weapon-ness is derived from physics (a hard small-area point cuts)
            // rather than an authored tag. Labelled fixtures inside the mechanical floor's axis ranges,
            // grounded in real weapon materials (keratin/bone ~200-400 MPa, enamel ~3000 MPa, sclerotized
            // chitin ~500 MPa). The magical weapons carry a placeholder mechanical form so they still read
            // as weapons until the thaumic track derives their true magical function.
            weapons: kinds(&[
                (
                    "claws",
                    false,
                    &[
                        ("mech.contact_area", "0.00000005"),
                        ("mech.edge_radius", "0.0001"),
                    ],
                    &[("mat.indentation_hardness", "300")],
                ),
                (
                    "teeth",
                    false,
                    &[
                        ("mech.contact_area", "0.0000001"),
                        ("mech.edge_radius", "0.0002"),
                    ],
                    &[("mat.indentation_hardness", "3000")],
                ),
                (
                    "horns",
                    false,
                    &[
                        ("mech.contact_area", "0.0000005"),
                        ("mech.edge_radius", "0.001"),
                    ],
                    &[("mat.indentation_hardness", "250")],
                ),
                (
                    "antlers",
                    false,
                    &[
                        ("mech.contact_area", "0.0000008"),
                        ("mech.edge_radius", "0.002"),
                    ],
                    &[("mat.indentation_hardness", "200")],
                ),
                (
                    "tusks",
                    false,
                    &[
                        ("mech.contact_area", "0.0000006"),
                        ("mech.edge_radius", "0.0015"),
                    ],
                    &[("mat.indentation_hardness", "250")],
                ),
                (
                    "sting",
                    false,
                    &[
                        ("mech.contact_area", "0.00000002"),
                        ("mech.edge_radius", "0.00005"),
                    ],
                    &[("mat.indentation_hardness", "150")],
                ),
                (
                    "beak",
                    false,
                    &[
                        ("mech.contact_area", "0.0000003"),
                        ("mech.edge_radius", "0.0005"),
                    ],
                    &[("mat.indentation_hardness", "300")],
                ),
                (
                    "spines",
                    false,
                    &[
                        ("mech.contact_area", "0.00000003"),
                        ("mech.edge_radius", "0.00008"),
                    ],
                    &[("mat.indentation_hardness", "200")],
                ),
                (
                    "talons",
                    false,
                    &[
                        ("mech.contact_area", "0.00000005"),
                        ("mech.edge_radius", "0.0001"),
                    ],
                    &[("mat.indentation_hardness", "350")],
                ),
                (
                    "mandibles",
                    false,
                    &[
                        ("mech.contact_area", "0.0000002"),
                        ("mech.edge_radius", "0.0003"),
                    ],
                    &[("mat.indentation_hardness", "500")],
                ),
                // Magical (Part 34, gated on a magic profile); placeholder mechanical form until the thaumic track.
                (
                    "mana-lash",
                    true,
                    &[
                        ("mech.contact_area", "0.0000004"),
                        ("mech.edge_radius", "0.0008"),
                    ],
                    &[("mat.indentation_hardness", "200")],
                ),
                (
                    "curse-touch",
                    true,
                    &[("mech.contact_area", "0.0000004")],
                    &[("mat.indentation_hardness", "100")],
                ),
                (
                    "ember-breath",
                    true,
                    &[("mech.contact_area", "0.0000005")],
                    &[("mat.indentation_hardness", "100")],
                ),
                (
                    "frost-fang",
                    true,
                    &[
                        ("mech.contact_area", "0.0000001"),
                        ("mech.edge_radius", "0.0002"),
                    ],
                    &[("mat.indentation_hardness", "1000")],
                ),
            ]),
            // Coverings carry the SURFACE emissivity (`opt.emissivity`, the chem/optics floor axis) their
            // outermost layer radiates at, so the metabolism's radiant thermoregulatory term reads the
            // being's OWN covering datum ([`crate::physiology::covering_emissivity`]) rather than a
            // duplicate global manifest scalar (the retired `metabolism.surface_emissivity`, which
            // duplicated this floor axis; derive-vs-author, Principle 6). A LABELLED development fixture:
            // uniform at 0.95 to match the retired scalar (so the radiant term is byte-identical), grounded
            // in biological-tissue emissivity (~0.95); per-covering differentiation (bare skin ~0.98, fur or
            // scales lower) is now DATA the mechanism supports, an owner/world calibration, never authored
            // here. Empty geometry (a covering carries no mechanical form axis).
            // Each covering carries `mat.fracture_energy` (predation-integration slice): the covering is the
            // OUTERMOST tissue a whole-body strike meets, so the catalog-wound coarse branch reads this via the
            // being's own body plan (`emb.organs.kind(covering.kind)`) for its Griffith tolerance, the coarse
            // analogue of a grown Segment's own `mat.fracture_energy`. The VALUE of each is a REAL floor value
            // grounded off the nearest existing tissue material (`body.rs` dev tissues: hide=3, bone=8), never a
            // fabricated magnitude: soft/keratin coverings read `hide`=3, rigid/mineral coverings read `bone`=8.
            // A per-substance datum (three-way test category 2). RESERVED-with-basis: the only authored part is
            // the per-covering-to-tissue ANALOGUE MAPPING (which tissue a covering resembles), the owner's to
            // confirm; the magnitude is the analogue tissue's own floor value. Byte-neutral: nothing reads it
            // until the coarse wound branch, armed only in `full --creatures`.
            coverings: kinds(&[
                (
                    "bare hide",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
                (
                    "fur",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
                (
                    "feathers",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
                (
                    "scales",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
                (
                    "chitin carapace",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "8")],
                ),
                (
                    "bony plates",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "8")],
                ),
                (
                    "shell",
                    false,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "8")],
                ),
                // Magical (placeholder mechanical form until the thaumic track derives their function): a soft
                // ward reads hide, a stone-skin reads bone, so a magically-covered being is still woundable.
                (
                    "mana-ward",
                    true,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
                (
                    "stone-skin",
                    true,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "8")],
                ),
                (
                    "phase-hide",
                    true,
                    &[],
                    &[("opt.emissivity", "0.95"), ("mat.fracture_energy", "3")],
                ),
            ]),
            // Senses carry crude optical material (a refractive index) so an optical sense's SIGHT function
            // is derived from physics (a lens denser than the medium focuses). The optical channel is the
            // one wired in step one; the acoustic, chemical, and field senses read no optical contrast and
            // are their own kernels (a documented follow-on), so they carry a placeholder index for now.
            senses: kinds(&[
                ("vision", false, &[], &[("opt.refractive_index", "1.4")]),
                ("smell", false, &[], &[("opt.refractive_index", "1.05")]),
                ("hearing", false, &[], &[("opt.refractive_index", "1.05")]),
                ("vibration", false, &[], &[("opt.refractive_index", "1.05")]),
                (
                    "echolocation",
                    false,
                    &[],
                    &[("opt.refractive_index", "1.05")],
                ),
                (
                    "electroreception",
                    false,
                    &[],
                    &[("opt.refractive_index", "1.05")],
                ),
                // Magical.
                ("mana-sight", true, &[], &[("opt.refractive_index", "1.3")]),
                ("aura-sense", true, &[], &[("opt.refractive_index", "1.05")]),
            ]),
            // Locomotion modes carry crude LIMB geometry (a section modulus and length) and material (a
            // bony yield strength), so a mode's LOCOMOTION function is derived from physics (a limb that
            // bears its propulsive load can push off). The rooted mark carries none (not a limb), so it
            // reads no locomotor capability. Labelled fixtures inside the floor axis ranges (cortical bone
            // ~150 MPa yield).
            locomotion: kinds(&[
                ("rooted", false, &[], &[]), // sessile: the mark of a plant, no limb
                (
                    "walk",
                    false,
                    &[
                        ("mech.section_modulus", "0.0001"),
                        ("mech.arm_length", "0.3"),
                    ],
                    &[("mat.yield_strength", "150")],
                ),
                (
                    "run",
                    false,
                    &[
                        ("mech.section_modulus", "0.00012"),
                        ("mech.arm_length", "0.35"),
                    ],
                    &[("mat.yield_strength", "150")],
                ),
                (
                    "climb",
                    false,
                    &[
                        ("mech.section_modulus", "0.00008"),
                        ("mech.arm_length", "0.25"),
                    ],
                    &[("mat.yield_strength", "150")],
                ),
                (
                    "swim",
                    false,
                    &[
                        ("mech.section_modulus", "0.00006"),
                        ("mech.arm_length", "0.4"),
                    ],
                    &[("mat.yield_strength", "80")],
                ),
                (
                    "fly",
                    false,
                    &[
                        ("mech.section_modulus", "0.00004"),
                        ("mech.arm_length", "0.5"),
                    ],
                    &[("mat.yield_strength", "60")],
                ),
                (
                    "glide",
                    false,
                    &[
                        ("mech.section_modulus", "0.00004"),
                        ("mech.arm_length", "0.5"),
                    ],
                    &[("mat.yield_strength", "60")],
                ),
                (
                    "burrow",
                    false,
                    &[
                        ("mech.section_modulus", "0.0001"),
                        ("mech.arm_length", "0.15"),
                    ],
                    &[("mat.yield_strength", "150")],
                ),
                (
                    "slither",
                    false,
                    &[
                        ("mech.section_modulus", "0.00005"),
                        ("mech.arm_length", "0.6"),
                    ],
                    &[("mat.yield_strength", "40")],
                ),
                // Magical.
                (
                    "levitate",
                    true,
                    &[
                        ("mech.section_modulus", "0.00004"),
                        ("mech.arm_length", "0.5"),
                    ],
                    &[("mat.yield_strength", "60")],
                ),
                (
                    "blink",
                    true,
                    &[
                        ("mech.section_modulus", "0.00004"),
                        ("mech.arm_length", "0.5"),
                    ],
                    &[("mat.yield_strength", "60")],
                ),
            ]),
            organs: organ_defs(&[
                // (name, fantasy, &[(biology-floor axis id, value)]). Function is derived from the
                // composition: an energy-dense tissue backs the energy reserve, a water-rich one the
                // hydration reserve, both to the extent of their composition. Nothing here is tagged,
                // and the axis ids are the floor's own (`bio.*`), so a new reserve type keys off an
                // existing or new floor axis with no code change.
                (
                    "fat-body",
                    false,
                    &[
                        ("bio.energy_density", Fixed::ONE),
                        ("bio.water_fraction", Fixed::from_ratio(1, 10)),
                    ][..],
                ),
                (
                    "glycogen-store",
                    false,
                    &[
                        ("bio.energy_density", Fixed::from_ratio(3, 4)),
                        ("bio.water_fraction", Fixed::from_ratio(1, 4)),
                    ][..],
                ),
                (
                    "water-store",
                    false,
                    &[
                        ("bio.energy_density", Fixed::ZERO),
                        ("bio.water_fraction", Fixed::ONE),
                    ][..],
                ),
                (
                    "generalist-viscera",
                    false,
                    &[
                        ("bio.energy_density", Fixed::from_ratio(1, 2)),
                        ("bio.water_fraction", Fixed::from_ratio(1, 2)),
                    ][..],
                ),
                // Muscle (Part 35, real-world unification step 5): a strength-bearing tissue whose
                // mat.fracture_strength composition the whole-body work force integrates over, so a body
                // that lists a muscle organ exerts force from its anatomy rather than a raw mass proxy. A
                // labelled dev fixture value (the FLESH 3 MPa the individual-tier Body::strength uses).
                (
                    "muscle",
                    false,
                    &[("mat.fracture_strength", Fixed::from_int(3))][..],
                ),
                // Magical (Part 34): a mana-storing tissue, a fixture stand-in until an arcane floor
                // grounds its composition.
                (
                    "mana-sac",
                    true,
                    &[
                        ("bio.energy_density", Fixed::from_ratio(1, 2)),
                        ("bio.water_fraction", Fixed::from_ratio(1, 4)),
                    ][..],
                ),
            ]),
        }
    }

    /// The name of a kind in a registry list, or `"?"` if unknown.
    pub fn name(list: &[KindDef], id: u16) -> &str {
        list.iter()
            .find(|k| k.id == id)
            .map_or("?", |k| k.name.as_str())
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
    /// The internal organs it bears (kind and development, where development is the organ's size, the
    /// capacity-bearing quantity), each drawn independently of body mass, covering, and trophic layer.
    /// Its reserve capacities derive from these organs' compositions, not from body mass: a huge,
    /// mostly-armored creature that rolls few or small organs holds small metabolic reserves, and a
    /// body with no organ contributing to a reserve has none of it (the owner-directed, composition-
    /// derived anatomy-derived reserves).
    pub organs: Vec<Part>,
    /// Its temperament personality.
    pub temperament: Temperament,
}

impl BodyPlanRegistry {
    /// The tissue composition of an organ kind, by registry id, if known.
    pub fn organ_composition(&self, kind: u16) -> Option<&TissueComposition> {
        self.organs
            .iter()
            .find(|o| o.id == kind)
            .map(|o| &o.composition)
    }
}

/// The gated kinds of a registry list for a profile (real always, fantasy only under magic).
fn gated(list: &[KindDef], profile: WorldProfile) -> Vec<&KindDef> {
    list.iter()
        .filter(|k| !k.fantasy || profile.magic)
        .collect()
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

/// Pick `n` distinct organ kinds, each with an independently drawn development (its size, the capacity-
/// bearing quantity). Mirrors [`pick`] over organ kinds. Keyed off a base counter with NO functional
/// coupling to body mass, covering, or trophic layer, so a body's organ endowment is an independent
/// draw: the load-bearing anti-steering invariant that lets a huge armored creature roll few or small
/// organs and so hold small metabolic reserves (Principle 8).
fn pick_organs(
    rng: &Rng,
    base: u64,
    list: &[OrganKindDef],
    profile: WorldProfile,
    n: usize,
) -> Vec<Part> {
    let pool: Vec<&OrganKindDef> = list
        .iter()
        .filter(|k| !k.fantasy || profile.magic)
        .collect();
    if pool.is_empty() || n == 0 {
        return Vec::new();
    }
    let mut out: Vec<Part> = Vec::new();
    for slot in 0..n {
        let idx = rng.range_u32(base + slot as u64 * 2, pool.len() as u32) as usize;
        let kind = pool[idx].id;
        if out.iter().any(|p| p.kind == kind) {
            continue; // a repeat pick collapses, so a body can bear fewer than n organs, or none
        }
        out.push(Part {
            kind,
            development: rng.unit_fixed(base + slot as u64 * 2 + 1),
        });
    }
    out
}

/// Sample a creature's body plan for its trophic layer, from the registries and gated by the
/// world profile. Whether the body is rooted or mobile is an emergent morphological outcome, drawn
/// against a `rooted_prior` bias, not a rule keyed on the organism's kingdom (Principle 9, physics
/// in and behaviour out). The prior is a strong tendency, not a law: autotrophy favours staying in
/// the light and heterotrophy favours moving to the food, so a producer is passed a high prior and
/// a consumer a low one, but neither is absolute, so a mobile autotroph (a walking tree) and a
/// sessile consumer (a coral, a barnacle) can both arise. A rooted body bears few, structural
/// weapons; a mobile one bears more at higher trophic layers and moves by one or two mobile modes.
/// Keyed off the `base` counter of the species' sample draw, so a species' anatomy is a
/// reproducible point over the registries.
pub fn sample_body_plan(
    rng: &Rng,
    layer: u16,
    rooted_prior: Fixed,
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
    // Rooted or mobile is drawn against the prior, not set by kingdom: below the prior the body is
    // rooted, above it the body moves. A high prior (a producer) is usually but not always rooted.
    let is_rooted = rng.unit_fixed(base + 79) < rooted_prior;
    // Weapon count: a rooted body bears at most one (structural, like spines); a mobile one bears
    // more at higher trophic layers. This keys off the drawn morphology, not the kingdom.
    let want_weapons = if is_rooted {
        (rng.range_u32(base + 19, 2)) as usize
    } else {
        (layer as usize).min(3)
    };
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
    // Locomotion: a rooted body carries the rooted mark (registry id 0) and does not walk; a mobile
    // one moves by one or two of the mobile modes (the non-rooted kinds). The outcome is the drawn
    // morphology, so a mobile autotroph carries real locomotion and a walking tree can exist.
    let locomotion = if is_rooted {
        vec![0]
    } else {
        let mobile: Vec<KindDef> = reg
            .locomotion
            .iter()
            .filter(|k| k.id != 0)
            .cloned()
            .collect();
        let want_loco = 1 + (rng.range_u32(base + 80, 2)) as usize;
        pick(rng, base + 82, &mobile, profile, want_loco)
            .into_iter()
            .map(|p| p.kind)
            .collect()
    };
    // Organs: one to four, drawn independently of body mass, covering, and layer (the anti-steering
    // invariant), so a body's reserve endowment is not a function of its size.
    let want_organs = 1 + (rng.range_u32(base + 100, 4)) as usize;
    let organs = pick_organs(rng, base + 102, &reg.organs, profile, want_organs);
    BodyPlan {
        body_mass,
        encephalization,
        diet_breadth,
        weapons,
        covering,
        senses,
        locomotion,
        organs,
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
        let plan = sample_body_plan(&rng(), 2, Fixed::ZERO, &reg, WorldProfile::grounded(), 200);
        for w in &plan.weapons {
            assert!(
                !reg.weapons.iter().find(|k| k.id == w.kind).unwrap().fantasy,
                "no magic weapon in a grounded world"
            );
        }
        assert!(
            !reg.coverings
                .iter()
                .find(|k| k.id == plan.covering.kind)
                .unwrap()
                .fantasy
        );
    }

    #[test]
    fn a_predator_bears_more_weapons_than_a_plant() {
        let reg = BodyPlanRegistry::dev_default();
        let rooted = sample_body_plan(&rng(), 0, Fixed::ONE, &reg, WorldProfile::grounded(), 200);
        let predator =
            sample_body_plan(&rng(), 3, Fixed::ZERO, &reg, WorldProfile::grounded(), 200);
        assert!(
            rooted.weapons.len() <= 1,
            "a rooted body bears at most one, structural weapon"
        );
        assert!(
            predator.weapons.len() >= rooted.weapons.len(),
            "a mobile predator bears more"
        );
    }

    #[test]
    fn mobility_is_the_body_not_the_kingdom() {
        // A producer is usually rooted but not by law: with a prior below one, a producer body can
        // draw mobile, a walking tree, and a consumer can draw rooted, a sessile filter-feeder. The
        // outcome is the drawn morphology, never the kingdom (Principle 9). Over many species a
        // high-but-not-one producer prior yields at least one mobile autotroph.
        let reg = BodyPlanRegistry::dev_default();
        let prior = Fixed::from_ratio(90, 100); // high, not absolute
        let mut mobile_producers = 0;
        for s in 0..200u64 {
            let r = DrawKey::entity(s, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1);
            let body = sample_body_plan(&r, 0, prior, &reg, WorldProfile::grounded(), 200);
            // A body whose locomotion is not just the rooted mark can walk (a walking tree).
            if body.locomotion.iter().any(|&m| m != 0) {
                mobile_producers += 1;
            }
        }
        assert!(
            mobile_producers > 0,
            "a walking tree can emerge: mobility is drawn, not decreed"
        );
    }

    #[test]
    fn a_body_plan_is_deterministic() {
        let reg = BodyPlanRegistry::dev_default();
        let a = sample_body_plan(&rng(), 2, Fixed::ZERO, &reg, WorldProfile::magical(), 200);
        let b = sample_body_plan(&rng(), 2, Fixed::ZERO, &reg, WorldProfile::magical(), 200);
        assert_eq!(a, b, "same key, same body plan");
        assert!(
            !a.senses.is_empty() && !a.locomotion.is_empty(),
            "a creature has senses and moves"
        );
    }

    #[test]
    fn a_body_plan_draws_organs_from_the_registry() {
        let reg = BodyPlanRegistry::dev_default();
        let plan = sample_body_plan(&rng(), 2, Fixed::ZERO, &reg, WorldProfile::grounded(), 200);
        assert!(!plan.organs.is_empty(), "a body draws at least one organ");
        for o in &plan.organs {
            assert!(
                reg.organs.iter().any(|k| k.id == o.kind),
                "every drawn organ is a registered kind"
            );
            assert!(
                reg.organ_composition(o.kind).is_some(),
                "and its composition is readable, so a reserve can derive its capacity"
            );
        }
    }

    #[test]
    fn organ_composition_reads_the_registry() {
        let reg = BodyPlanRegistry::dev_default();
        // The fat-body fixture (id 0) is energy-dense (ONE) and nearly dry (1/10). Its function is not
        // tagged anywhere; the mechanism reads this composition to derive which reserve it backs.
        let fat = reg.organ_composition(0).expect("fat-body is registered");
        assert_eq!(
            fat.component("bio.energy_density"),
            Fixed::ONE,
            "the composition reads its energy-density off the floor axis id"
        );
        assert_eq!(
            fat.component("bio.water_fraction"),
            Fixed::from_ratio(1, 10)
        );
        assert_eq!(
            fat.component("bio.protein_fraction"),
            Fixed::ZERO,
            "an axis the organ bears none of reads as zero (the substrate absence convention)"
        );
        assert!(
            reg.organ_composition(9999).is_none(),
            "an unknown organ kind has no composition"
        );
    }

    #[test]
    fn a_grounded_world_draws_no_magical_organs() {
        // The mana-sac (fantasy) organ is gated on a magic profile, like every other fantasy kind.
        let reg = BodyPlanRegistry::dev_default();
        for s in 0..80u64 {
            let r = DrawKey::entity(s, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1);
            let plan = sample_body_plan(&r, 2, Fixed::ZERO, &reg, WorldProfile::grounded(), 200);
            for o in &plan.organs {
                assert!(
                    !reg.organs.iter().find(|k| k.id == o.kind).unwrap().fantasy,
                    "no magical organ in a grounded world"
                );
            }
        }
    }

    #[test]
    fn a_magical_world_can_bear_magical_kinds() {
        // Over many species, a magical world should produce at least one fantasy part.
        let reg = BodyPlanRegistry::dev_default();
        let mut saw_magic = false;
        for s in 0..80u64 {
            let r = DrawKey::entity(s, 0, Phase::BIOSPHERE_SAMPLE).rng(0xB0D1);
            let plan = sample_body_plan(&r, 3, Fixed::ZERO, &reg, WorldProfile::magical(), 200);
            let magic_weapon = plan
                .weapons
                .iter()
                .any(|w| reg.weapons.iter().find(|k| k.id == w.kind).unwrap().fantasy);
            let magic_cover = reg
                .coverings
                .iter()
                .find(|k| k.id == plan.covering.kind)
                .unwrap()
                .fantasy;
            if magic_weapon || magic_cover {
                saw_magic = true;
                break;
            }
        }
        assert!(saw_magic, "a magical world produces magical anatomy");
    }
}
