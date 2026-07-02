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

//! Homeostatic physiology and morphological affordances (design Part 15, Part 20, Part 8, Part
//! 25.14, Part 58; R-PHYS-BIO; Principles 9 and 11). Stage 1 of R-BEHAVIOR-EVOLVE, the evolved-
//! behaviour work whose design pass is `docs/emergent_behavior_design.md`.
//!
//! This is the physical substrate an evolved controller will read and act through, built so that a
//! being's needs and options are consequences of its body rather than an authored menu (Principle
//! 9). Two data-defined registries, so neither the set of needs nor the set of options is a closed
//! enum in the mechanism (Principle 11), the same substrate-registry pattern the value (Part 21),
//! semantic (Part 33), and institution-function (Part 36) layers use.
//!
//! A homeostatic axis is a reserve the body must keep within a viable band: energy, water, and, for
//! an exotic creature, whatever else a world declares (an arcane charge, a heat store). Each axis is
//! a [`civsim_core`]-fixed-point [`Stock`] (design Part 15), the compartment abstraction the ecology
//! already uses, with no self-regeneration: it drains by metabolism, which is a consequence of the
//! body's physics, and is restored only by taking matter in, whose yield the resolved biology floor
//! measures (R-PHYS-BIO, `crate::edibility`). When any axis falls through its floor the body dies.
//! There is no "thirst drive" here; there is a water level, and low water is a physical state.
//!
//! An affordance is a physical operation the body's morphology permits: moving, if it bears a
//! locomotion organ (the walking-tree rule of `crate::locomotion`, mobility from the body, not the
//! kingdom); ingesting, which any body does. The affordance set is data and each affordance is
//! gated by a requirement over the body plan's anatomy categories (Part 25.14, Part 35), which are
//! authored physical anatomy, not behaviour. What is authored is what a body physically can do;
//! which operation it issues when is the evolved controller's, built in the stages that follow.
//!
//! Everything here is integer, fixed-point, and draws no randomness, so a being's physiology is a
//! pure function of its body and its intake and reproduces bit for bit (Principle 3). Every rate and
//! band is reserved with its basis in the development fixtures and is the owner's to set, never
//! fabricated (Principle 11).

use std::collections::BTreeMap;

use civsim_core::Fixed;

use crate::anatomy::{BodyPlan, BodyPlanRegistry};
use crate::stocks::Stock;

/// A homeostatic axis id, minted through the registry (extensible, never a closed enum). The
/// numeric values are stable ids folded into no canonical stream on their own; they key the
/// per-being reserves.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HomeostaticAxisId(pub u16);

/// One homeostatic axis as data: how a body's reserve of this quantity drains and what keeps it
/// viable. Membership grows with the world (Principle 11): a world adds an axis without touching the
/// mechanism, so a mana-fed or a thermovore creature is expressible as data.
#[derive(Clone, Debug)]
pub struct HomeostaticAxisDef {
    /// The axis id.
    pub id: HomeostaticAxisId,
    /// A legibility handle, never read by the mechanism.
    pub name: String,
    /// The biology-floor axis id whose tissue this reserve is backed by (`bio.energy_density`,
    /// `bio.water_fraction`, and the rest the floor declares), or `None` for a derived non-draining axis
    /// (integrity, temperature) whose capacity is a fixed unit and whose level is sourced each tick from
    /// elsewhere. On the canonical anatomy-derived path ([`Homeostasis::new`]) the reserve's capacity is
    /// the development-weighted sum over the being's organs of their composition on this floor axis: a
    /// being carries this reserve because it bears energy-dense (or water-rich) tissue, its function
    /// DERIVED from composition, never a tag on an organ kind. The id is data (a floor axis, the
    /// `Substance::vector` key convention), so a reserve backed by protein or a respiratory-surface axis
    /// (R-MEDIUM) is a data edit, not a code change (Principle 11).
    pub backing_component: Option<String>,
    /// The reserve capacity as a multiple of body mass, used ONLY by the labelled development fallback
    /// [`Homeostasis::from_mass`], never by the canonical anatomy-derived path. RESERVED. Basis: the
    /// reserve size relative to body mass from the Part 20 physiology; retained so tests and fixtures
    /// that do not model organs still run, not as a production default.
    pub capacity_per_mass: Fixed,
    /// The base drain per tick, as a fraction of capacity, from resting metabolism. RESERVED.
    /// Basis: the basal metabolic rate of Part 20 mapped onto the base tick the owner set (one
    /// in-world second), per axis (water is lost slower than energy is burnt).
    pub base_drain: Fixed,
    /// How much body exertion (a unit-interval activity signal) adds to this axis's drain per tick.
    /// RESERVED. Basis: the movement-and-work energy cost of Part 20; energy couples strongly to
    /// exertion, water less, a mana charge perhaps not at all, which is why it is per-axis data.
    pub exertion_drain: Fixed,
    /// The viable floor as a fraction of capacity: at or below it the body fails on this axis and
    /// dies. RESERVED. Basis: the physiological reserve at which the body can no longer function,
    /// per axis (Part 20 death conditions).
    pub death_floor: Fixed,
}

/// The set of homeostatic axes a world runs, data-defined and extensible.
#[derive(Clone, Debug, Default)]
pub struct HomeostaticRegistry {
    pub axes: Vec<HomeostaticAxisDef>,
}

impl HomeostaticRegistry {
    /// A labelled DEVELOPMENT FIXTURE (an energy axis and a water axis), not owner values, so the
    /// physiology runs and can be tested now. The two axes are the minimum a mobile heterotroph
    /// needs; a world adds others as data.
    pub fn dev_default() -> HomeostaticRegistry {
        HomeostaticRegistry {
            axes: vec![
                HomeostaticAxisDef {
                    id: ENERGY,
                    name: "energy".to_string(),
                    backing_component: Some("bio.energy_density".to_string()),
                    capacity_per_mass: Fixed::ONE,
                    base_drain: Fixed::from_ratio(1, 400),
                    exertion_drain: Fixed::from_ratio(1, 100),
                    death_floor: Fixed::ZERO,
                },
                HomeostaticAxisDef {
                    id: WATER,
                    name: "water".to_string(),
                    backing_component: Some("bio.water_fraction".to_string()),
                    capacity_per_mass: Fixed::from_ratio(6, 10),
                    base_drain: Fixed::from_ratio(1, 300),
                    exertion_drain: Fixed::from_ratio(1, 400),
                    death_floor: Fixed::ZERO,
                },
            ],
        }
    }

    /// A DEVELOPMENT FIXTURE for an embodied being: energy and water (metabolic reserves) plus
    /// integrity (bodily condition, refreshed from the per-part body rather than drained). Integrity
    /// does not self-drain (its base and exertion draws are zero); it is set each tick from
    /// [`crate::body::Body::integrity`]. Not owner values.
    pub fn dev_embodied() -> HomeostaticRegistry {
        let mut reg = HomeostaticRegistry::dev_default();
        reg.axes.push(HomeostaticAxisDef {
            id: INTEGRITY,
            name: "integrity".to_string(),
            backing_component: None,
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::ZERO,
            exertion_drain: Fixed::ZERO,
            death_floor: Fixed::ZERO,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for the thermal coupling: a single temperature axis whose reserve is a
    /// two-sided comfort band. Like integrity, it does not self-drain (its base and exertion draws are
    /// zero); its level is set each tick from the located body core temperature through the comfort-band
    /// map (`crate::runner`), so a temperature outside the viable band is a physical state the evolved
    /// controller reads, and a body carried a full half-band past its set point (a zero comfort
    /// fraction) has fallen through the floor and dies. Isolating temperature (no energy or water axis)
    /// lets the thermal coupling be exercised and tested without a metabolic-starvation confound. Not
    /// owner values.
    pub fn dev_thermal() -> HomeostaticRegistry {
        HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: TEMPERATURE,
                name: "temperature".to_string(),
                backing_component: None,
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            }],
        }
    }

    /// The axis definition for an id, if registered.
    pub fn axis(&self, id: HomeostaticAxisId) -> Option<&HomeostaticAxisDef> {
        self.axes.iter().find(|a| a.id == id)
    }
}

/// The energy axis of the development fixture (a leaf id, not special-cased in the mechanism).
pub const ENERGY: HomeostaticAxisId = HomeostaticAxisId(0);
/// The water axis of the development fixture.
pub const WATER: HomeostaticAxisId = HomeostaticAxisId(1);
/// The integrity axis: bodily condition, an axis whose level is refreshed each tick from the per-part
/// body ([`crate::body::Body::integrity`], R-WOUND) rather than drained by metabolism, so a wound is a
/// state the evolved controller reads and a destroyed body is a death. Derived, never a competing
/// store (design Part 35).
pub const INTEGRITY: HomeostaticAxisId = HomeostaticAxisId(2);
/// The temperature axis: core temperature, a two-sided survivable band. The metabolic side is a
/// reserve; the environmental exchange (through the resolved thermal floor) is the reserved coupling
/// that waits on the located world.
pub const TEMPERATURE: HomeostaticAxisId = HomeostaticAxisId(3);

/// A being's homeostatic state: one reserve [`Stock`] per axis, keyed by axis id in canonical
/// order so a walk over the reserves is reproducible (R-CANON-WALK). The reserves do not self-
/// regenerate; they drain by metabolism and are restored only by intake.
#[derive(Clone, Debug)]
pub struct Homeostasis {
    reserves: BTreeMap<HomeostaticAxisId, Stock>,
}

impl Homeostasis {
    /// A being at full reserves, capacities derived from its ANATOMY (the canonical path). For each
    /// axis backed by a biology-floor composition axis, the reserve capacity is the development-weighted
    /// sum over the being's organs of that organ's composition on the axis: `Σ organ.development *
    /// organs.organ_composition(organ.kind).component(axis_id)`. So a reserve exists to the extent the
    /// body bears tissue that stores it, function DERIVED from composition against the biology floor,
    /// never a tag (Principle 8). A huge, mostly-armored creature that rolled few or small organs holds
    /// small metabolic reserves; a body with no organ contributing to an axis has zero capacity there
    /// and fails that axis at once (the armored-giant case the owner raised). A non-backed axis
    /// (integrity, temperature; `backing_component == None`) is a unit-capacity derived reserve whose
    /// level is sourced each tick from elsewhere, not stored in tissue.
    pub fn new(
        reg: &HomeostaticRegistry,
        plan: &BodyPlan,
        organs: &BodyPlanRegistry,
    ) -> Homeostasis {
        let mut reserves = BTreeMap::new();
        for axis in &reg.axes {
            let cap = match &axis.backing_component {
                Some(axis_id) => {
                    let mut sum = Fixed::ZERO;
                    for organ in &plan.organs {
                        let share = organs
                            .organ_composition(organ.kind)
                            .map(|comp| comp.component(axis_id))
                            .unwrap_or(Fixed::ZERO);
                        let backed = organ.development.checked_mul(share).unwrap_or(Fixed::ZERO);
                        sum = sum.saturating_add(backed);
                    }
                    sum
                }
                // A derived, non-stored axis (integrity, temperature): unit capacity, level set each tick.
                None => Fixed::ONE,
            };
            // A reserve holds [0, cap], starts full, and does not regenerate on its own (rate 0).
            reserves.insert(axis.id, Stock::new(cap, cap, Fixed::ZERO));
        }
        Homeostasis { reserves }
    }

    /// A labelled DEVELOPMENT FALLBACK: capacities set from body mass alone (`capacity_per_mass *
    /// body_mass`), for tests and fixtures that do not model organs. This is the pre-anatomy path and
    /// is NOT the production constructor: sourcing a reserve from body mass leaks size back into the
    /// reserve, which is exactly what the anatomy-derived [`Homeostasis::new`] removes. Retained so the
    /// physiology, locomotion, and thermal fixtures still run without building a full organ endowment.
    pub fn from_mass(reg: &HomeostaticRegistry, body_mass: Fixed) -> Homeostasis {
        let mass = body_mass.clamp(Fixed::ZERO, Fixed::ONE);
        let mut reserves = BTreeMap::new();
        for axis in &reg.axes {
            let cap = axis
                .capacity_per_mass
                .checked_mul(mass)
                .unwrap_or(Fixed::ZERO);
            // A reserve holds [0, cap], starts full, and does not regenerate on its own (rate 0).
            reserves.insert(axis.id, Stock::new(cap, cap, Fixed::ZERO));
        }
        Homeostasis { reserves }
    }

    /// The current level of an axis as a fraction of its capacity, in `[0, ONE]`, the normalised
    /// read a controller and a view see. An unregistered axis reads as zero.
    pub fn level(&self, axis: HomeostaticAxisId) -> Fixed {
        self.reserves
            .get(&axis)
            .map(|s| s.occupancy())
            .unwrap_or(Fixed::ZERO)
    }

    /// The raw reserve amount of an axis (for intake accounting).
    pub fn amount(&self, axis: HomeostaticAxisId) -> Fixed {
        self.reserves
            .get(&axis)
            .map(|s| s.amount())
            .unwrap_or(Fixed::ZERO)
    }

    /// The capacity of an axis's reserve (for sizing a fractional intake against a bite yield). An
    /// unregistered axis reads as zero.
    pub fn capacity(&self, axis: HomeostaticAxisId) -> Fixed {
        self.reserves
            .get(&axis)
            .map(|s| s.capacity())
            .unwrap_or(Fixed::ZERO)
    }

    /// Set a derived axis's level to a fraction of its capacity, for an axis whose value is sourced
    /// from elsewhere each tick rather than drained by metabolism (integrity, refreshed from the
    /// per-part body; design Part 35's derived, never-stored condition). A no-op for an unregistered
    /// axis. The fraction is clamped to `[0, ONE]`.
    pub fn set_level(&mut self, axis: HomeostaticAxisId, fraction: Fixed) {
        if let Some(stock) = self.reserves.get_mut(&axis) {
            let target = fraction
                .clamp(Fixed::ZERO, Fixed::ONE)
                .checked_mul(stock.capacity())
                .unwrap_or(Fixed::ZERO);
            let current = stock.amount();
            if target >= current {
                stock.deposit(target - current);
            } else {
                stock.take(current - target);
            }
        }
    }

    /// Advance one tick of metabolism: every reserve drains by its resting rate plus its exertion
    /// coupling times the body's current exertion (a unit-interval signal, for example the fraction
    /// of top speed a mover is using). Deterministic; draws no randomness. Returns whether the body
    /// is still alive after the drain.
    pub fn metabolize(&mut self, reg: &HomeostaticRegistry, exertion: Fixed) -> bool {
        let exertion = exertion.clamp(Fixed::ZERO, Fixed::ONE);
        for axis in &reg.axes {
            if let Some(stock) = self.reserves.get_mut(&axis.id) {
                let cap = stock.capacity();
                // Drain is a fraction of capacity: (base + exertion_coupling * exertion) * capacity.
                let frac = axis.base_drain.saturating_add(
                    axis.exertion_drain
                        .checked_mul(exertion)
                        .unwrap_or(Fixed::ZERO),
                );
                let draw = frac.checked_mul(cap).unwrap_or(Fixed::ZERO);
                stock.step(draw);
            }
        }
        self.is_alive(reg)
    }

    /// Take matter in on one axis: deposit `amount` into that reserve, capped at capacity, returning
    /// what the reserve could hold. The `amount` is the yield the biology floor measured for this
    /// axis (net nutrition for energy, water content for water; R-PHYS-BIO, `crate::edibility`), so
    /// the physiology stays floor-agnostic and the caller resolves what a bite is worth.
    pub fn ingest(&mut self, axis: HomeostaticAxisId, amount: Fixed) -> Fixed {
        self.reserves
            .get_mut(&axis)
            .map(|s| s.deposit(amount))
            .unwrap_or(Fixed::ZERO)
    }

    /// Whether every reserve is above its death floor. A body fails the moment one axis falls
    /// through its floor (you die of thirst though your energy is full).
    pub fn is_alive(&self, reg: &HomeostaticRegistry) -> bool {
        self.dead_axis(reg).is_none()
    }

    /// The first axis (in canonical id order) that has fallen to or below its death floor, if any,
    /// so a caller can record the cause of death.
    pub fn dead_axis(&self, reg: &HomeostaticRegistry) -> Option<HomeostaticAxisId> {
        for axis in &reg.axes {
            if let Some(stock) = self.reserves.get(&axis.id) {
                let floor = axis
                    .death_floor
                    .checked_mul(stock.capacity())
                    .unwrap_or(Fixed::ZERO);
                if stock.amount() <= floor {
                    return Some(axis.id);
                }
            }
        }
        None
    }
}

/// Whether a body plan's anatomy can sustain life at birth: built at full reserves from its organs
/// ([`Homeostasis::new`]), no backed axis starts at or below its death floor. A body with no organ
/// contributing to a backed reserve (the extreme of the armored-giant case: a creature that stores no
/// energy at all) is birth-nonviable and this returns false.
///
/// This is a QUERY over physics, not a generator gate. The owner's decision on the birth-nonviable
/// case is to LEAN ON THE EXISTING CLOSURE CULL rather than add a seed-time reject: in the running
/// sim a birth-nonviable organism dies at once (its reserve is already through the floor), so its
/// aggregate pool draws no sustaining return and collapses under the Part 15 stock dynamics
/// ([`crate::stocks::Stock`]), the same over-harvest cull that removes an under-supplied pool. Nothing
/// here rejects a species at seed time. The cull it leans on reads only the food web and supply, never
/// morphology, and birth-viability is a pure function of the organ set, independent of body mass,
/// covering, or weaponry, so leaning on the cull removes only the physically-impossible and steers no
/// morphological outcome (proven in `crates/sim/tests/biosphere_steering.rs`, Principle 9).
pub fn birth_viable(reg: &HomeostaticRegistry, plan: &BodyPlan, organs: &BodyPlanRegistry) -> bool {
    Homeostasis::new(reg, plan, organs).is_alive(reg)
}

/// An affordance id, minted through the registry (extensible, never a closed enum of behaviours).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AffordanceId(pub u16);

/// The anatomy category a morphological requirement reads. These are the authored physical body-
/// plan groups of Part 25.14 and Part 35 (a body has weapons, coverings, senses, and locomotion
/// organs), fixed physics rather than world content, so a small discriminator here references the
/// body plan's own shape; the affordance set that references it is open data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MorphCategory {
    /// No organ required: an operation any body performs (taking matter in).
    Any,
    /// A locomotion organ that is not merely the rooted mark (the walking-tree rule).
    Locomotion,
    /// A natural weapon.
    Weapon,
    /// A sense organ.
    Sense,
}

/// The shape of the parameter a physical operation takes, a structural property of the operation
/// itself (a move is aimed somewhere, an ingestion is not), not a behaviour. It sets how many
/// controller outputs the operation reads (R-BEHAVIOR-EVOLVE, [`crate::controller`]): a directional
/// operation reads an activation and a heading, a scalar one reads only an activation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AffordanceParam {
    /// The operation takes no aim (ingesting the matter on the current tile, resting).
    Scalar,
    /// The operation is aimed by a two-component heading on the map (moving, and later striking or
    /// grasping toward a target).
    Directional,
}

/// One affordance as data: a physical operation, the morphological requirement that gates it, and
/// the shape of its parameter. The set and the gates are data (Principle 11); the categories the
/// gates read are the authored anatomy (Part 25.14). Membership is what makes a body able to walk
/// or to strike, never a rule about when it should.
#[derive(Clone, Debug)]
pub struct AffordanceDef {
    /// The affordance id.
    pub id: AffordanceId,
    /// A legibility handle.
    pub name: String,
    /// The anatomy category the body must bear (above `min_development`) to perform this operation.
    pub requires: MorphCategory,
    /// The minimum development the required organ must have, in `[0, ONE]`. Ignored for `Any`.
    pub min_development: Fixed,
    /// The shape of the operation's parameter (how many controller outputs it reads).
    pub param: AffordanceParam,
}

/// The set of affordance primitives a world runs, data-defined and extensible.
#[derive(Clone, Debug, Default)]
pub struct AffordanceRegistry {
    pub affordances: Vec<AffordanceDef>,
}

impl AffordanceRegistry {
    /// A labelled DEVELOPMENT FIXTURE: move (gated on a locomotion organ) and ingest (unconditional).
    /// A world adds affordances (strike, grasp) as data as the body plan grows the organs for them.
    pub fn dev_default() -> AffordanceRegistry {
        AffordanceRegistry {
            affordances: vec![
                AffordanceDef {
                    id: MOVE,
                    name: "move".to_string(),
                    requires: MorphCategory::Locomotion,
                    min_development: Fixed::ZERO,
                    param: AffordanceParam::Directional,
                },
                AffordanceDef {
                    id: INGEST,
                    name: "ingest".to_string(),
                    requires: MorphCategory::Any,
                    min_development: Fixed::ZERO,
                    param: AffordanceParam::Scalar,
                },
            ],
        }
    }

    /// A DEVELOPMENT FIXTURE for a combat-capable world: move, ingest, and strike (gated on a natural
    /// weapon). Kept distinct from [`AffordanceRegistry::dev_default`] so the foraging behaviour tests
    /// keep their two-affordance layout; a predator's controller layout carries the extra strike
    /// output and can evolve to use it, closing predator-prey (R-BEHAVIOR-EVOLVE).
    pub fn dev_predator() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: STRIKE,
            name: "strike".to_string(),
            requires: MorphCategory::Weapon,
            min_development: Fixed::ZERO,
            param: AffordanceParam::Directional,
        });
        reg
    }

    /// The affordances a given body can perform, in canonical id order, reading its morphology. A
    /// rooted body cannot move; a body bearing a locomotion organ can, whatever its kingdom.
    pub fn afforded(&self, body: &BodyPlan) -> Vec<AffordanceId> {
        self.affordances
            .iter()
            .filter(|a| affords(body, a))
            .map(|a| a.id)
            .collect()
    }

    /// Whether a body affords a specific operation.
    pub fn affords_id(&self, body: &BodyPlan, id: AffordanceId) -> bool {
        self.affordances
            .iter()
            .find(|a| a.id == id)
            .map(|a| affords(body, a))
            .unwrap_or(false)
    }
}

/// The move affordance of the development fixture.
pub const MOVE: AffordanceId = AffordanceId(0);
/// The ingest affordance of the development fixture.
pub const INGEST: AffordanceId = AffordanceId(1);
/// The strike affordance (a natural-weapon attack), in the combat fixture only.
pub const STRIKE: AffordanceId = AffordanceId(2);

/// The rooted locomotion mark: a locomotion list holding only this is not a mobile organ (the
/// walking-tree rule, matching `crate::locomotion`).
const ROOTED_MODE: u16 = 0;

/// Whether a body meets an affordance's morphological requirement.
fn affords(body: &BodyPlan, a: &AffordanceDef) -> bool {
    match a.requires {
        MorphCategory::Any => true,
        MorphCategory::Locomotion => {
            // A mobile organ is any non-rooted locomotion mode; development is not tracked per
            // locomotion mode in the current body plan, so the min-development gate is vacuous here
            // until it is (the honest limit, noted in the design pass).
            body.locomotion.iter().any(|&m| m != ROOTED_MODE)
        }
        MorphCategory::Weapon => body
            .weapons
            .iter()
            .any(|p| p.development >= a.min_development),
        MorphCategory::Sense => body
            .senses
            .iter()
            .any(|p| p.development >= a.min_development),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{Part, Temperament};

    fn body(mass: (i64, i64), locomotion: Vec<u16>) -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(mass.0, mass.1),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: vec![],
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![Part {
                kind: 1,
                development: Fixed::from_ratio(1, 2),
            }],
            locomotion,
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(1, 2),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        }
    }

    #[test]
    fn a_body_starts_at_full_reserves() {
        let reg = HomeostaticRegistry::dev_default();
        let h = Homeostasis::from_mass(&reg, Fixed::ONE);
        assert_eq!(h.level(ENERGY), Fixed::ONE, "energy starts full");
        assert_eq!(h.level(WATER), Fixed::ONE, "water starts full");
        assert!(h.is_alive(&reg));
    }

    #[test]
    fn a_bigger_body_holds_a_larger_reserve() {
        let reg = HomeostaticRegistry::dev_default();
        let big = Homeostasis::from_mass(&reg, Fixed::ONE);
        let small = Homeostasis::from_mass(&reg, Fixed::from_ratio(1, 4));
        // Both start full (occupancy ONE), but the raw amount the big body holds is greater.
        assert!(
            big.amount(ENERGY) > small.amount(ENERGY),
            "size buys a larger energy reserve"
        );
    }

    #[test]
    fn metabolism_drains_and_eventually_kills() {
        let reg = HomeostaticRegistry::dev_default();
        let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
        // Rest (no exertion): the reserves fall over time.
        let mut alive_ticks = 0;
        for _ in 0..100_000 {
            if !h.metabolize(&reg, Fixed::ZERO) {
                break;
            }
            alive_ticks += 1;
        }
        assert!(!h.is_alive(&reg), "unfed, the body eventually dies");
        assert!(alive_ticks > 0, "it lived for a while first");
        assert!(h.dead_axis(&reg).is_some(), "a cause of death is recorded");
    }

    #[test]
    fn exertion_drains_energy_faster_than_rest() {
        let reg = HomeostaticRegistry::dev_default();
        let mut resting = Homeostasis::from_mass(&reg, Fixed::ONE);
        let mut working = Homeostasis::from_mass(&reg, Fixed::ONE);
        for _ in 0..50 {
            resting.metabolize(&reg, Fixed::ZERO);
            working.metabolize(&reg, Fixed::ONE);
        }
        assert!(
            working.level(ENERGY) < resting.level(ENERGY),
            "working burns energy faster"
        );
    }

    #[test]
    fn ingesting_restores_a_reserve() {
        let reg = HomeostaticRegistry::dev_default();
        let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
        for _ in 0..50 {
            h.metabolize(&reg, Fixed::ONE);
        }
        let low = h.level(ENERGY);
        h.ingest(ENERGY, Fixed::from_ratio(1, 2)); // a nutritious bite (yield from the floor)
        assert!(h.level(ENERGY) > low, "eating restores the energy reserve");
    }

    #[test]
    fn death_is_per_axis() {
        // A tiny water capacity relative to its drain kills by thirst though energy is fine.
        let reg = HomeostaticRegistry::dev_default();
        let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
        // Keep energy topped up while never drinking.
        let mut dead_of = None;
        for _ in 0..100_000 {
            h.metabolize(&reg, Fixed::ZERO);
            h.ingest(ENERGY, Fixed::ONE); // always refill energy
            if let Some(a) = h.dead_axis(&reg) {
                dead_of = Some(a);
                break;
            }
        }
        assert_eq!(
            dead_of,
            Some(WATER),
            "it dies of thirst while its energy is kept full"
        );
    }

    #[test]
    fn a_rooted_body_cannot_move_but_can_ingest() {
        let reg = AffordanceRegistry::dev_default();
        let rooted = body((1, 2), vec![ROOTED_MODE]);
        let afforded = reg.afforded(&rooted);
        assert!(
            !afforded.contains(&MOVE),
            "a rooted body affords no movement"
        );
        assert!(afforded.contains(&INGEST), "but it still takes matter in");
        assert!(!reg.affords_id(&rooted, MOVE));
    }

    #[test]
    fn a_mobile_body_affords_movement_whatever_its_kingdom() {
        let reg = AffordanceRegistry::dev_default();
        let walking_tree = body((1, 2), vec![3]); // an autotroph body with a mobile organ
        let afforded = reg.afforded(&walking_tree);
        assert!(
            afforded.contains(&MOVE),
            "a body with a locomotion organ can move"
        );
        assert!(afforded.contains(&INGEST));
    }

    #[test]
    fn physiology_is_deterministic() {
        let reg = HomeostaticRegistry::dev_default();
        let run = || {
            let mut h = Homeostasis::from_mass(&reg, Fixed::from_ratio(3, 4));
            for t in 0..200 {
                let exertion = if t % 2 == 0 { Fixed::ONE } else { Fixed::ZERO };
                h.metabolize(&reg, exertion);
                if t % 10 == 0 {
                    h.ingest(ENERGY, Fixed::from_ratio(1, 5));
                }
            }
            (h.amount(ENERGY).to_bits(), h.amount(WATER).to_bits())
        };
        assert_eq!(run(), run(), "the same body and intake replay bit for bit");
    }

    // The anatomy-derived (composition-derived) path: capacities come from the being's organs, not
    // from body mass. Function is DERIVED from tissue composition against the biology floor, never a
    // tag: an energy-dense organ backs the energy reserve, a water-rich one the hydration reserve.

    /// An organ of a given registry kind and development (its size, the capacity-bearing quantity).
    fn organ(kind: u16, dev: (i64, i64)) -> Part {
        Part {
            kind,
            development: Fixed::from_ratio(dev.0, dev.1),
        }
    }

    /// A body carrying a given organ set. Body mass is set independently and large on purpose, so the
    /// tests can show reserves do NOT track mass. Locomotion is the rooted mark (irrelevant here).
    fn organ_body(mass: (i64, i64), organs: Vec<Part>) -> BodyPlan {
        let mut b = body(mass, vec![ROOTED_MODE]);
        b.organs = organs;
        b
    }

    #[test]
    fn capacity_is_derived_from_organ_composition() {
        // The dev registry: fat-body (id 0) is energy_density ONE, water_fraction 1/10; water-store
        // (id 2) is energy_density ZERO, water_fraction ONE. A body with one full fat-body holds a
        // full energy reserve and only a tenth of a water reserve, straight from the composition.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        let plan = organ_body((1, 2), vec![organ(0, (1, 1))]); // one fat-body at full development
        let h = Homeostasis::new(&reg, &plan, &organs);
        assert_eq!(
            h.capacity(ENERGY),
            Fixed::ONE,
            "a full fat-body backs a full energy reserve (development ONE * energy_density ONE)"
        );
        assert_eq!(
            h.capacity(WATER),
            Fixed::from_ratio(1, 10),
            "the same fat-body backs only a tenth of a water reserve (water_fraction 1/10)"
        );
    }

    #[test]
    fn an_energy_dense_organ_backs_energy_not_water() {
        // A pure water-store (id 2): energy_density ZERO, water_fraction ONE. It backs the water
        // reserve fully and the energy reserve not at all, so a creature of only water-store tissue has
        // no energy reserve and fails the energy axis at once. Function is derived, never tagged.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        let plan = organ_body((1, 2), vec![organ(2, (1, 1))]); // one full water-store
        let h = Homeostasis::new(&reg, &plan, &organs);
        assert_eq!(
            h.capacity(WATER),
            Fixed::ONE,
            "water-store backs water fully"
        );
        assert_eq!(
            h.capacity(ENERGY),
            Fixed::ZERO,
            "and backs energy not at all: no energy-dense tissue, no energy reserve"
        );
        assert_eq!(
            h.dead_axis(&reg),
            Some(ENERGY),
            "with no energy-backing organ it has no energy reserve and dies on that axis"
        );
    }

    #[test]
    fn an_armored_giant_with_few_organs_holds_small_reserves() {
        // The owner's case: a huge, mostly-armored creature that rolled few or small organs holds SMALL
        // metabolic reserves, while a small, organ-rich body holds large ones. Reserves derive from
        // anatomy, not from body mass, so size does not buy endurance on its own.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        // A giant: maximum body mass, but a single tiny fat-body (development 1/8).
        let giant = organ_body((1, 1), vec![organ(0, (1, 8))]);
        // A small body: a quarter the mass, but a full fat-body.
        let small_rich = organ_body((1, 4), vec![organ(0, (1, 1))]);
        let hg = Homeostasis::new(&reg, &giant, &organs);
        let hs = Homeostasis::new(&reg, &small_rich, &organs);
        assert!(
            giant.body_mass > small_rich.body_mass,
            "the giant is by far the larger body"
        );
        assert!(
            hg.capacity(ENERGY) < hs.capacity(ENERGY),
            "yet the giant holds the smaller energy reserve: reserves derive from organs, not mass"
        );
    }

    #[test]
    fn a_body_with_no_organs_has_no_metabolic_reserves() {
        // No organs contributing to a backed axis, no reserve of it. A body with an empty organ set
        // has zero energy and water capacity and fails the first backed axis at once.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        let plan = organ_body((1, 1), vec![]);
        let h = Homeostasis::new(&reg, &plan, &organs);
        assert_eq!(h.capacity(ENERGY), Fixed::ZERO);
        assert_eq!(h.capacity(WATER), Fixed::ZERO);
        assert!(
            !h.is_alive(&reg),
            "an organ-less body carries no metabolic reserve and is not viable"
        );
    }

    #[test]
    fn organ_backed_capacity_sums_over_organs_and_scales_with_development() {
        // Capacity is the development-weighted sum over organs: two energy-dense organs back more
        // energy reserve than one, and a larger organ backs more than a smaller one of the same kind.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        // fat-body (id 0, energy_density ONE) at half development: energy capacity 1/2.
        let one = Homeostasis::new(&reg, &organ_body((1, 2), vec![organ(0, (1, 2))]), &organs);
        // two organs contributing to energy: fat-body 1/2 plus glycogen-store (id 1, energy_density
        // 3/4) at 1/2, energy capacity 1/2 + 3/8 = 7/8.
        let two = Homeostasis::new(
            &reg,
            &organ_body((1, 2), vec![organ(0, (1, 2)), organ(1, (1, 2))]),
            &organs,
        );
        assert_eq!(one.capacity(ENERGY), Fixed::from_ratio(1, 2));
        assert_eq!(two.capacity(ENERGY), Fixed::from_ratio(7, 8));
        assert!(
            two.capacity(ENERGY) > one.capacity(ENERGY),
            "more energy-backing tissue, a larger energy reserve"
        );
    }

    #[test]
    fn a_derived_axis_has_unit_capacity_regardless_of_organs() {
        // A non-backed axis (backing_component None), integrity or temperature, is a unit-capacity
        // derived reserve whose level is sourced each tick from elsewhere. Its capacity does not depend
        // on the organ set, so an organ-less body still carries a full integrity axis to be refreshed.
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_embodied();
        let plan = organ_body((1, 1), vec![]); // no organs at all
        let h = Homeostasis::new(&reg, &plan, &organs);
        assert_eq!(
            h.capacity(INTEGRITY),
            Fixed::ONE,
            "integrity is a derived unit-capacity axis, independent of organs"
        );
        assert_eq!(
            h.capacity(ENERGY),
            Fixed::ZERO,
            "while the organ-backed metabolic axes are empty"
        );
    }

    #[test]
    fn anatomy_derived_reserves_are_deterministic() {
        let organs = BodyPlanRegistry::dev_default();
        let reg = HomeostaticRegistry::dev_default();
        let plan = organ_body((3, 4), vec![organ(0, (1, 2)), organ(2, (1, 4))]);
        let run = || {
            let h = Homeostasis::new(&reg, &plan, &organs);
            (h.capacity(ENERGY).to_bits(), h.capacity(WATER).to_bits())
        };
        assert_eq!(run(), run(), "the same anatomy derives the same reserves");
    }

    #[test]
    fn a_reserve_can_key_off_any_floor_axis_as_pure_data() {
        // The hardening proof (Principle 11): a reserve backed by a biology-floor axis the default
        // fixtures never use (`bio.protein_fraction`) works with DATA ALONE. No enum variant, no match
        // arm, no struct field is touched: the composition and the backing are keyed off floor axis
        // ids, the `Substance::vector` convention, so the reserve vocabulary grows with the floor's
        // data, never a code change. A future respiratory-surface axis (R-MEDIUM) enters the same way.
        use crate::anatomy::{OrganKindDef, TissueComposition};
        let mut organs = BodyPlanRegistry::dev_default();
        organs.organs = vec![OrganKindDef {
            id: 0,
            name: "muscle".to_string(),
            fantasy: false,
            composition: TissueComposition::from_pairs(&[("bio.protein_fraction", Fixed::ONE)]),
        }];
        let reg = HomeostaticRegistry {
            axes: vec![HomeostaticAxisDef {
                id: HomeostaticAxisId(9),
                name: "protein".to_string(),
                backing_component: Some("bio.protein_fraction".to_string()),
                capacity_per_mass: Fixed::ONE,
                base_drain: Fixed::ZERO,
                exertion_drain: Fixed::ZERO,
                death_floor: Fixed::ZERO,
            }],
        };
        let plan = organ_body((1, 2), vec![organ(0, (1, 1))]);
        let h = Homeostasis::new(&reg, &plan, &organs);
        assert_eq!(
            h.capacity(HomeostaticAxisId(9)),
            Fixed::ONE,
            "a protein-backed reserve derives from a protein-rich organ, keyed off the floor axis id"
        );
    }
}
