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

use civsim_compose::{
    derive_capabilities, CapabilityCaps, CapabilityRefs, FunctionLawId, FunctionLawRegistry,
};

use crate::anatomy::{BodyPlan, BodyPlanRegistry, KindDef};
use crate::morphogen::Structure;
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

    /// A DEVELOPMENT FIXTURE for a located, foraging grazer (base-level liveliness step 3): the two
    /// metabolic reserves (energy and water, backed by tissue and drained by metabolism) plus the
    /// temperature axis the field-coupled runner requires (non-draining, set each tick from the body
    /// core temperature through the comfort-band map). This is the minimum a being that walks, thermo-
    /// regulates, and eats needs: the temperature axis lets it couple to the field, and the energy and
    /// water axes give it hunger and thirst, so a depletable resource loop can bound its lineage. The
    /// axis set is data, so an alien creature with an arcane charge or a thermovore reserve adds another
    /// backed axis without touching the mechanism (Principle 11). Not owner values.
    pub fn dev_grazer() -> HomeostaticRegistry {
        let mut reg = HomeostaticRegistry::dev_default(); // energy and water, both organ-backed
        reg.axes.push(HomeostaticAxisDef {
            id: TEMPERATURE,
            name: "temperature".to_string(),
            backing_component: None,
            capacity_per_mass: Fixed::ONE,
            base_drain: Fixed::ZERO,
            exertion_drain: Fixed::ZERO,
            death_floor: Fixed::ZERO,
        });
        // The condition reserve the environmental-harm sink drains (base-level liveliness step 4): a
        // non-draining, unit-capacity axis degraded only by the measured net_harm of the cell's toxin
        // dose, so a salt flat kills a naive being through the reserve-through-floor cull.
        reg.axes.push(HomeostaticAxisDef {
            id: CONDITION,
            name: "condition".to_string(),
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
/// The respiration axis: the body's respirable-gas reserve (an oxygen buffer). It drains by metabolism
/// and is replenished by gas exchange with the ambient medium through the Fick membrane law (R-MEDIUM,
/// [`crate::medium`]). A being with no respiratory organ presents no exchange surface, takes up nothing,
/// and suffocates, whatever the medium.
pub const RESPIRATION: HomeostaticAxisId = HomeostaticAxisId(4);
/// The condition axis: a derived, non-draining, unit-capacity reserve degraded by ENVIRONMENTAL HARM
/// (base-level liveliness step 4). It does not self-drain (its base and exertion draws are zero) and,
/// at the mass-only Walker tier, is not refreshed from a body (unlike [`INTEGRITY`], which the Body tier
/// sources from `crate::body::Body::integrity`), so it is the condition reserve the per-tick harm sink
/// drains by the measured `net_harm` of the cell's toxin dose against the being's heritable tolerances.
/// When it falls through its floor the body dies of exposure, the emergent cull that makes a salt flat
/// lethal to a naive lineage and livable to a heritable halophile line (Principle 8: death is the
/// reserve-through-floor cull, never a fixed-dose exclusion gate).
pub const CONDITION: HomeostaticAxisId = HomeostaticAxisId(5);

/// A per-being DERIVED drain for one homeostatic axis: the resting and the exertion
/// fraction-of-capacity-per-tick the physics derivation produced from the body's mass, tissue, and
/// medium ([`crate::physiology::derive_base_drain`], [`crate::physiology::derive_exertion_coupling`]),
/// consumed by [`Homeostasis::metabolize_derived`]. These are the derived siblings of the authored
/// [`HomeostaticAxisDef::base_drain`] and [`HomeostaticAxisDef::exertion_drain`] scalars, computed per
/// body against the physics rather than read from a hardcoded axis field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DerivedDrain {
    /// The resting drain, a fraction of capacity per tick (the Kleiber basal rate plus the
    /// thermoregulatory replacement, bridged to a reserve fraction).
    pub base: Fixed,
    /// The added drain per unit of exertion, a fraction of capacity per tick (the work power bridged to
    /// a reserve fraction), scaled by the being's exertion signal in `metabolize_derived`.
    pub exertion: Fixed,
}

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

    /// A being at full reserves, capacities sourced from a GROWN body structure rather than a catalog organ
    /// set (emergent-anatomy Step 3, the metabolic-tier grow): the metabolic sibling of the affordance and
    /// speed direct reads. For each backed axis, the capacity is [`Structure::backed_capacity`], the grown
    /// segments' `bio.*` tissue composition summed directly, with no organ kind id and no registry. A
    /// non-backed axis (integrity, temperature) is unit capacity, level set each tick, exactly as in
    /// [`Homeostasis::new`]. So a grown body sources its own metabolism from its grown tissue: a body whose
    /// tissue backs no energy carries no energy reserve and starves at birth, leaning on the same
    /// reserve-floor cull as any other death (Principle 8), never a morphology gate. This lets a grown race
    /// found a body with no catalog `BodyPlan` at all, the catalog metabolic tier retired.
    pub fn from_structure(reg: &HomeostaticRegistry, structure: &Structure) -> Homeostasis {
        let mut reserves = BTreeMap::new();
        for axis in &reg.axes {
            let cap = match &axis.backing_component {
                Some(axis_id) => structure.backed_capacity(axis_id),
                // A derived, non-stored axis (integrity, temperature): unit capacity, level set each tick.
                None => Fixed::ONE,
            };
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

    /// Advance one tick of metabolism from a per-being DERIVED drain vector rather than the axis defs'
    /// authored scalar drain fields. Each entry is a [`DerivedDrain`] (a resting and an exertion
    /// fraction-of-capacity) the physics derivation produced for this body from its mass, tissue, and
    /// medium ([`crate::physiology`]), so the drain a body pays follows its physics and not a
    /// hardcoded per-axis number (freeing `base_metabolic_drain` and `exertion_drain_coupling`). An axis
    /// absent from the vector does not drain (a derived non-metabolic axis, integrity or temperature,
    /// contributes nothing), so the same walk serves an embodied being carrying those axes. Walks
    /// `reg.axes` in canonical order (R-CANON-WALK) and draws no randomness; returns whether the body is
    /// still alive after the drain. The scalar-field [`Homeostasis::metabolize`] is kept as the labelled
    /// fixture path so the dev fixtures and the `from_mass` tests still run.
    pub fn metabolize_derived(
        &mut self,
        reg: &HomeostaticRegistry,
        drains: &BTreeMap<HomeostaticAxisId, DerivedDrain>,
        exertion: Fixed,
    ) -> bool {
        let exertion = exertion.clamp(Fixed::ZERO, Fixed::ONE);
        for axis in &reg.axes {
            let Some(drain) = drains.get(&axis.id) else {
                continue;
            };
            if let Some(stock) = self.reserves.get_mut(&axis.id) {
                let cap = stock.capacity();
                // Drain is a fraction of capacity: (base + exertion_coupling * exertion) * capacity,
                // exactly the shape metabolize uses, but with base and coupling DERIVED per being.
                let frac = drain
                    .base
                    .saturating_add(drain.exertion.checked_mul(exertion).unwrap_or(Fixed::ZERO));
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

    /// Adjust an axis's reserve by a signed delta: deposit when positive, take when negative, held in
    /// `[0, capacity]`. The medium respiration coupling ([`crate::medium::respire`]) uses this to apply
    /// the signed Fick gas flux, which is uptake in a richer medium and loss in a poorer one. A no-op
    /// for an unregistered axis.
    pub fn adjust(&mut self, axis: HomeostaticAxisId, delta: Fixed) {
        if let Some(stock) = self.reserves.get_mut(&axis) {
            if delta >= Fixed::ZERO {
                stock.deposit(delta);
            } else {
                stock.take(Fixed::ZERO - delta);
            }
        }
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

/// One affordance as data: a physical operation, the physics CAPABILITY that gates it, and the shape of
/// its parameter. The set and the gates are data (Principle 11); the gate is now a DERIVED capability
/// (emergent-anatomy step one), not an authored anatomy category: a body can walk because a part of it
/// reads the LOCOMOTE capability, strike because a part reads PIERCE, read from the parts' own geometry
/// and material through the function-law dispatch, never a `MorphCategory` label. Membership is what
/// makes a body able to walk or to strike, never a rule about when it should.
#[derive(Clone, Debug)]
pub struct AffordanceDef {
    /// The affordance id.
    pub id: AffordanceId,
    /// A legibility handle.
    pub name: String,
    /// The physics capability the body must bear a part reading (at or above `min_capability`) to perform
    /// this operation, or `None` for an operation any body performs (taking matter in). Retires the
    /// authored `MorphCategory`: the gate is a derived capability, not an anatomy category.
    pub requires: Option<FunctionLawId>,
    /// The minimum capability a part must read to afford the operation, in `[0, ONE]`. Ignored for `None`.
    pub min_capability: Fixed,
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
                    requires: Some(FunctionLawRegistry::ID_LOCOMOTE),
                    min_capability: Fixed::ZERO,
                    param: AffordanceParam::Directional,
                },
                AffordanceDef {
                    id: INGEST,
                    name: "ingest".to_string(),
                    requires: None,
                    min_capability: Fixed::ZERO,
                    param: AffordanceParam::Scalar,
                },
            ],
        }
    }

    /// A DEVELOPMENT FIXTURE for a combat-capable world: move, ingest, and strike (gated on the PIERCE
    /// capability). Kept distinct from [`AffordanceRegistry::dev_default`] so the foraging behaviour tests
    /// keep their two-affordance layout; a predator's controller layout carries the extra strike
    /// output and can evolve to use it, closing predator-prey (R-BEHAVIOR-EVOLVE).
    pub fn dev_predator() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: STRIKE,
            name: "strike".to_string(),
            requires: Some(FunctionLawRegistry::ID_PIERCE),
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Directional,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a matter-carrying world: move, ingest, and grasp (material-substrate arc,
    /// cascade item 3, the driver). Kept distinct from [`AffordanceRegistry::dev_default`] so the foraging
    /// behaviour tests keep their two-affordance layout; a carrier's controller layout carries the extra
    /// grasp output and can evolve to use it, the emergent decision to pick matter up. Grasp is a scalar
    /// operation (taking the matter underfoot, no aim) and, like ingest, is unconditional (`requires:
    /// None`): any body may attempt to lift, and the physical gating is the strength-versus-weight bound the
    /// enactment applies ([`crate::runner::Embodiment::pick_up`]), never a capability label. A world that
    /// wires a manipulator capability adds a `requires` here as data, the way item 4's extraction contest
    /// will gate on contact pressure.
    pub fn dev_carrier() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: GRASP,
            name: "grasp".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a matter-mining world: move, ingest, and extract (material-substrate arc,
    /// cascade item 4, the extraction contest). Kept distinct from [`AffordanceRegistry::dev_default`] so
    /// the foraging tests keep their two-affordance layout; a miner's controller layout carries the extra
    /// extract output and can evolve to use it, the emergent decision to break bonded matter loose. Extract
    /// is a scalar operation (working the matter underfoot, no aim) and, like ingest and grasp, is
    /// unconditional (`requires: None`): any body may attempt to break matter, and the physical gating is
    /// the fracture contest the enactment applies (the being's contact pressure against the cell's
    /// fracture-gating hardness, [`crate::runner::Embodiment::extract_underfoot`]), never a capability
    /// label. A world that wires a mining-tool capability adds a `requires` here as data.
    pub fn dev_miner() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: EXTRACT,
            name: "extract".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a mineral-eating world: move, ingest, and geophage (material-substrate
    /// arc, cascade item 4, the force-and-manipulation extension, INGEST-FOR-COMPOSITION). Kept distinct
    /// from [`AffordanceRegistry::dev_default`] so the foraging tests keep their two-affordance layout; a
    /// geophage's controller layout carries the extra geophage output and can evolve to use it, the emergent
    /// decision to eat the matter underfoot for a reserve that needs it (a mineral, salt, grit). Geophage is
    /// a scalar operation (eating the matter underfoot, no aim) and, like ingest, is unconditional
    /// (`requires: None`): any body may attempt to eat matter, and what it gains is its own physiology's
    /// assimilation of the substance against its reserve's need ([`crate::runner::Embodiment::geophage`]),
    /// never a capability label. This is the need-side complement to harm-learning: the same cell
    /// composition a being learns to AVOID for a harm, another being SEEKS for a nutrient it lacks.
    pub fn dev_geophage() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: GEOPHAGE,
            name: "geophage".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a tool-making world: move, ingest, extract, and craft (material-substrate
    /// arc, cascade item 4, the knapping that shapes a tool from mined stone). A toolmaker can mine stone
    /// (EXTRACT) and shape its carried stone into a wielded tool (CRAFT), the two halves of the recursive
    /// tool loop: mine harder rock with a made tool, made from the rock it mined. Craft is a scalar
    /// operation (working the carried matter, no aim) and, like the other matter actions, is unconditional
    /// (`requires: None`): any body may attempt to shape what it carries, and what results is a tool whose
    /// function the crafting seam's cut read derives from its geometry and material
    /// ([`crate::runner::Embodiment::craft_from_carried`]), never a recipe catalog.
    pub fn dev_toolmaker() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: EXTRACT,
            name: "extract".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg.affordances.push(AffordanceDef {
            id: CRAFT,
            name: "craft".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a terrain-shaping world: move, ingest, and dig (material-substrate arc,
    /// cascade item 5, modifiable terrain). A digger excavates the ground underfoot in the same fracture
    /// contest the extraction uses, but the removed matter LOWERS the column (a pit) as well as loading the
    /// carrier, so digging reshapes the terrain rather than only mining it. Dig is a scalar operation
    /// (working the ground underfoot, no aim) and, like the other matter actions, is unconditional
    /// (`requires: None`): any body may attempt to dig, and the physical gating is the fracture contest the
    /// enactment applies ([`crate::runner::Embodiment::dig_underfoot`]), never a capability label.
    pub fn dev_digger() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: DIG,
            name: "dig".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// A DEVELOPMENT FIXTURE for a terrain-shaping world: move, ingest, dig, and release (material-substrate
    /// arc, cascade item 5, modifiable terrain, the deposit-and-mound half). An earthmover can dig a pit
    /// (DIG lowers a column and loads the carrier) and set the spoil down elsewhere (RELEASE deposits the
    /// carried load and raises that column), so terracing, a mound beside a pit, EMERGES from sequencing the
    /// two primitives, no MOUND verb. Release is the inverse of grasp, a scalar operation (setting the load
    /// down underfoot, no aim), unconditional (`requires: None`): any body may open its grasp, and the
    /// consequence is the deposited matter and the raised column ([`crate::runner::Embodiment::release_underfoot`]).
    pub fn dev_earthmover() -> AffordanceRegistry {
        let mut reg = AffordanceRegistry::dev_default();
        reg.affordances.push(AffordanceDef {
            id: DIG,
            name: "dig".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg.affordances.push(AffordanceDef {
            id: RELEASE,
            name: "release".to_string(),
            requires: None,
            min_capability: Fixed::ZERO,
            param: AffordanceParam::Scalar,
        });
        reg
    }

    /// The affordances a given body can perform, in canonical id order, DERIVED from the capabilities its
    /// parts read (emergent-anatomy step one). A rooted body cannot move (no part reads LOCOMOTE); a body
    /// bearing a load-bearing limb can, whatever its kingdom, by physics not by an authored category.
    pub fn afforded(
        &self,
        body: &BodyPlan,
        organs: &BodyPlanRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> Vec<AffordanceId> {
        self.affordances
            .iter()
            .filter(|a| affords(body, a, organs, refs, caps))
            .map(|a| a.id)
            .collect()
    }

    /// Whether a body affords a specific operation, derived from its parts' capabilities.
    pub fn affords_id(
        &self,
        body: &BodyPlan,
        id: AffordanceId,
        organs: &BodyPlanRegistry,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> bool {
        self.affordances
            .iter()
            .find(|a| a.id == id)
            .map(|a| affords(body, a, organs, refs, caps))
            .unwrap_or(false)
    }

    /// The affordances a GROWN body can perform, in canonical id order, read from its [`Structure`]
    /// DIRECTLY (emergent-anatomy Step 2): the required capability is the greatest any grown segment reads
    /// through the function-law dispatch, so a grown body strikes because a segment reads PIERCE and moves
    /// because one reads LOCOMOTE, with no catalog kind id and no organ-registry lookup. This is the
    /// grown-body counterpart to [`Self::afforded`]; a body carries one or the other (catalog parts read
    /// against the shared registry, or its own grown structure), never both.
    pub fn afforded_structure(
        &self,
        structure: &Structure,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
    ) -> Vec<AffordanceId> {
        self.affordances
            .iter()
            .filter(|a| affords_structure(structure, a, refs, caps))
            .map(|a| a.id)
            .collect()
    }
}

/// The move affordance of the development fixture.
pub const MOVE: AffordanceId = AffordanceId(0);
/// The ingest affordance of the development fixture.
pub const INGEST: AffordanceId = AffordanceId(1);
/// The strike affordance (a natural-weapon attack), in the combat fixture only.
pub const STRIKE: AffordanceId = AffordanceId(2);
/// The grasp affordance (picking the matter underfoot up into the carried load), in the carrier fixture
/// only (material-substrate arc, cascade item 3, the driver).
pub const GRASP: AffordanceId = AffordanceId(3);
/// The extract affordance (breaking bonded matter underfoot loose in a fracture contest and taking it), in
/// the miner fixture only (material-substrate arc, cascade item 4, the extraction contest).
pub const EXTRACT: AffordanceId = AffordanceId(4);
/// The geophage affordance (eating the matter underfoot for a reserve backed by that substance), in the
/// geophage fixture only (material-substrate arc, cascade item 4, INGEST-FOR-COMPOSITION).
pub const GEOPHAGE: AffordanceId = AffordanceId(5);
/// The craft affordance (shaping the carried matter into a wielded tool), in the toolmaker fixture only
/// (material-substrate arc, cascade item 4, crafting, the knapping that makes a tool from mined stone).
pub const CRAFT: AffordanceId = AffordanceId(6);
/// The dig affordance (excavating the ground underfoot: a fracture contest that lowers the column and
/// yields spoil), in the digger fixture only (material-substrate arc, cascade item 5, modifiable terrain).
pub const DIG: AffordanceId = AffordanceId(7);
/// The release affordance (setting the carried load down underfoot, the inverse of grasp: it deposits the
/// matter and raises the column), in the earthmover fixture only (material-substrate arc, cascade item 5).
pub const RELEASE: AffordanceId = AffordanceId(8);

/// The maximum capability the body's parts read on one function law, DERIVED from each part's geometry
/// and material through the function-law dispatch (emergent-anatomy step one), blind to any kind or race
/// id. A body affords an operation when one of its parts reads the required capability, so a weapon body
/// strikes and a limbed one moves, by physics not by an authored anatomy category.
fn body_capability(
    body: &BodyPlan,
    organs: &BodyPlanRegistry,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
    law: FunctionLawId,
) -> Fixed {
    let fns = FunctionLawRegistry::dev_seed();
    let cap_of = |k: &KindDef| -> Fixed {
        let geo = |axis: &str| k.geo(axis);
        let mat = |axis: &str| k.mat(axis);
        derive_capabilities(&fns, &geo, &mat, refs, caps).score(law)
    };
    let mut best = Fixed::ZERO;
    for p in &body.weapons {
        if let Some(k) = organs.weapons.iter().find(|k| k.id == p.kind) {
            best = best.max(cap_of(k));
        }
    }
    for p in &body.senses {
        if let Some(k) = organs.senses.iter().find(|k| k.id == p.kind) {
            best = best.max(cap_of(k));
        }
    }
    for &m in &body.locomotion {
        if let Some(k) = organs.locomotion.iter().find(|k| k.id == m) {
            best = best.max(cap_of(k));
        }
    }
    best
}

/// Whether a body affords an operation, DERIVED from its parts' capabilities: an unconditional operation
/// (no required capability) any body performs; a gated one needs a part reading the required capability at
/// or above the threshold.
fn affords(
    body: &BodyPlan,
    a: &AffordanceDef,
    organs: &BodyPlanRegistry,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> bool {
    match a.requires {
        None => true,
        Some(law) => {
            body_capability(body, organs, refs, caps, law) >= a.min_capability.max(MIN_CAP)
        }
    }
}

/// Whether a GROWN body affords an operation, read from its [`Structure`] directly: an unconditional
/// operation any body performs; a gated one needs a grown segment reading the required capability at or
/// above the threshold, the greatest any segment reads through the function-law dispatch.
fn affords_structure(
    structure: &Structure,
    a: &AffordanceDef,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> bool {
    match a.requires {
        None => true,
        Some(law) => {
            let fns = FunctionLawRegistry::dev_seed();
            structure.max_capability(law, &fns, refs, caps) >= a.min_capability.max(MIN_CAP)
        }
    }
}

/// The floor a derived capability must clear to count (a zero-capability part does not afford), so an
/// affordance with a zero `min_capability` still requires a positive reading, not merely a present part.
const MIN_CAP: Fixed = Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 20)); // ~1e-6

/// A being's memory of its reserve levels at the previous tick, the substrate of the interoceptive
/// DELTA percept (harm-learning arc slice a; Principles 3, 9). It retains the per-axis normalised
/// level so the per-tick change `delta(axis) = level_now - level_prev` is a pure fixed-point
/// subtraction: the raw interoceptive signal a being feels, a reserve FALLING felt as harm and rising
/// as relief, read with no cause attached. CONDITION's per-tick change already nets
/// `condition_recovery - harm` (`crate::locomotion`), so its delta is the net environmental-harm
/// signal for free, and keying off the homeostatic registry generalises the signal past CONDITION to
/// any reserve (a food that sickens is an energy or toxin-backed reserve falling after ingestion).
///
/// This is new per-being DYNAMIC state: it must fold into `state_hash` alongside the reserve levels
/// (in canonical axis order), and it draws no randomness, so a run stays bit-identical across worker
/// widths. It is populated only where the harm-learning path runs; an empty memory folds nothing and
/// leaves a run's hash unchanged, so the delta percept is opt-in (the emergent-anatomy pattern).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReserveMemory {
    prev: BTreeMap<HomeostaticAxisId, Fixed>,
}

impl ReserveMemory {
    /// An empty memory: no prior levels retained, so the first tick feels no change (the clean
    /// degrade) and nothing folds into the hash until the first snapshot.
    pub fn new() -> ReserveMemory {
        ReserveMemory {
            prev: BTreeMap::new(),
        }
    }

    /// The per-tick change in an axis's normalised level since the last snapshot: `level_now -
    /// level_prev`. An axis never snapshotted reads a zero delta (no prior, so no felt change), so a
    /// being's first tick on that axis feels nothing rather than a spurious jump from zero.
    pub fn delta(&self, axis: HomeostaticAxisId, homeo: &Homeostasis) -> Fixed {
        let now = homeo.level(axis);
        let prev = self.prev.get(&axis).copied().unwrap_or(now);
        now - prev
    }

    /// The retained previous level of an axis, or zero if none is held (for folding into the hash in
    /// the runner's canonical axis walk).
    pub fn prev_level(&self, axis: HomeostaticAxisId) -> Fixed {
        self.prev.get(&axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// Snapshot the current normalised level of every registered axis, to be the previous levels next
    /// tick. Called once per body-tick after the reserves settle, in the registry's canonical axis
    /// order, drawing no randomness.
    pub fn snapshot(&mut self, reg: &HomeostaticRegistry, homeo: &Homeostasis) {
        self.prev.clear();
        for axis in &reg.axes {
            self.prev.insert(axis.id, homeo.level(axis.id));
        }
    }

    /// Whether any prior levels are held (an unsnapshotted memory folds nothing into the hash).
    pub fn is_empty(&self) -> bool {
        self.prev.is_empty()
    }
}

/// Whether this tick's interoceptive `delta` counts as HARM: a reserve falling (a negative delta)
/// whose magnitude exceeds the harm-noise floor, so ordinary metabolic drain and measurement jitter
/// are not read as harm. Pure and RNG-free; the associative learner (slice b) calls it to decide
/// whether a cell's feature earns evidence toward "this ground harms me" this tick.
///
/// RESERVED: the harm-noise floor. Basis: the noise floor of normal metabolic drain the physiology
/// defines, the largest per-tick reserve fall a resting, unharmed body incurs (`base_drain` scaled to
/// the tick), so only a fall beyond ordinary living registers as harm. Surfaced for the owner, never
/// fabricated.
pub fn is_harm_tick(delta: Fixed, harm_noise_floor: Fixed) -> bool {
    delta < Fixed::ZERO && delta.abs() > harm_noise_floor
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

    /// The labelled capability context an affordance derive reads in these tests: the dev organ registry
    /// (the part kinds a body's ids index), the dev references, and the mechanical floor's own physics
    /// ceilings (pressure 150000 MPa, length 100 m), matching the compose crate's derive.
    fn cap_ctx() -> (BodyPlanRegistry, CapabilityRefs, CapabilityCaps) {
        (
            BodyPlanRegistry::dev_default(),
            CapabilityRefs::dev_refs(),
            CapabilityCaps {
                pressure: Fixed::from_int(150_000),
                depth: Fixed::from_int(100),
            },
        )
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
    fn reserve_delta_is_the_signed_change_since_the_last_snapshot() {
        let reg = HomeostaticRegistry::dev_default();
        let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
        let mut mem = ReserveMemory::new();
        // An unsnapshotted memory folds nothing and feels no change on the first read.
        assert!(mem.is_empty());
        assert_eq!(
            mem.delta(ENERGY, &h),
            Fixed::ZERO,
            "no prior, no felt change"
        );

        mem.snapshot(&reg, &h);
        assert!(!mem.is_empty());
        // A drain since the snapshot is felt as a negative delta of exactly that fall.
        h.adjust(ENERGY, Fixed::ZERO - Fixed::from_ratio(1, 4));
        assert_eq!(
            mem.delta(ENERGY, &h),
            Fixed::ZERO - Fixed::from_ratio(1, 4),
            "a quarter-capacity fall reads as a -0.25 delta"
        );
        // A gain reads as a positive delta.
        let mut h2 = Homeostasis::from_mass(&reg, Fixed::ONE);
        h2.adjust(ENERGY, Fixed::ZERO - Fixed::from_ratio(1, 2));
        let mut mem2 = ReserveMemory::new();
        mem2.snapshot(&reg, &h2);
        h2.adjust(ENERGY, Fixed::from_ratio(1, 4));
        assert_eq!(
            mem2.delta(ENERGY, &h2),
            Fixed::from_ratio(1, 4),
            "relief is positive"
        );
    }

    #[test]
    fn a_harm_tick_is_a_fall_beyond_the_noise_floor_only() {
        let floor = Fixed::from_ratio(1, 100);
        // A fall bigger than the floor is harm.
        assert!(is_harm_tick(Fixed::ZERO - Fixed::from_ratio(1, 10), floor));
        // A fall smaller than the floor (ordinary drain) is not.
        assert!(!is_harm_tick(
            Fixed::ZERO - Fixed::from_ratio(1, 1000),
            floor
        ));
        // A rise (relief) is never harm however large.
        assert!(!is_harm_tick(Fixed::from_ratio(1, 2), floor));
        // Exactly at the floor magnitude is not harm (strict).
        assert!(!is_harm_tick(Fixed::ZERO - floor, floor));
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
    fn metabolize_derived_drains_from_the_derived_vector_not_the_axis_fields() {
        // The derived path drains each axis by a per-being DerivedDrain, not by the axis def's authored
        // base_drain / exertion_drain scalars. An axis absent from the vector does not drain, and the
        // derived rate governs the pace.
        let reg = HomeostaticRegistry::dev_default();
        let mut h = Homeostasis::from_mass(&reg, Fixed::ONE);
        let mut drains = BTreeMap::new();
        drains.insert(
            ENERGY,
            DerivedDrain {
                base: Fixed::from_ratio(1, 10),
                exertion: Fixed::ZERO,
            },
        );
        // WATER absent from the vector: it must not drain at all on the derived path.
        let e0 = h.level(ENERGY);
        let w0 = h.level(WATER);
        assert!(h.metabolize_derived(&reg, &drains, Fixed::ZERO));
        assert!(h.level(ENERGY) < e0, "the derived energy drain applies");
        assert_eq!(h.level(WATER), w0, "an axis absent from the vector holds");
        // Ten resting ticks at a tenth of capacity per tick empties and kills on energy.
        for _ in 0..12 {
            h.metabolize_derived(&reg, &drains, Fixed::ZERO);
        }
        assert!(!h.is_alive(&reg), "the derived drain eventually kills");
        assert_eq!(
            h.dead_axis(&reg),
            Some(ENERGY),
            "it dies on the drained axis"
        );
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
        let (organs, refs, caps) = cap_ctx();
        // The rooted mark (kind id 0) bears no limb geometry, so it reads no LOCOMOTE capability.
        let rooted = body((1, 2), vec![0]);
        let afforded = reg.afforded(&rooted, &organs, &refs, &caps);
        assert!(
            !afforded.contains(&MOVE),
            "a rooted body affords no movement"
        );
        assert!(afforded.contains(&INGEST), "but it still takes matter in");
        assert!(!reg.affords_id(&rooted, MOVE, &organs, &refs, &caps));
    }

    #[test]
    fn a_mobile_body_affords_movement_whatever_its_kingdom() {
        let reg = AffordanceRegistry::dev_default();
        let (organs, refs, caps) = cap_ctx();
        let walking_tree = body((1, 2), vec![3]); // an autotroph body with a load-bearing limb (climb)
        let afforded = reg.afforded(&walking_tree, &organs, &refs, &caps);
        assert!(
            afforded.contains(&MOVE),
            "a body with a load-bearing limb can move"
        );
        assert!(afforded.contains(&INGEST));
    }

    #[test]
    fn the_capability_move_gate_reproduces_the_retired_rooted_mark_gate() {
        // Hash-neutrality of the MorphCategory retirement: the derived-capability MOVE gate must return
        // the identical verdict the retired `m != rooted-mark` proxy did for EVERY registry locomotion
        // mode, so a run over these modes decides movement identically and its canonical state hash is
        // unchanged (this slice retires the anatomy category, not the run's behaviour). Every non-rooted
        // mode bears a limb that reads a LOCOMOTE capability, so it affords MOVE; the rooted mark (kind
        // id 0) bears none, so it does not, by physics rather than by a mode id.
        let reg = AffordanceRegistry::dev_default();
        let (organs, refs, caps) = cap_ctx();
        for k in &organs.locomotion {
            let b = body((1, 2), vec![k.id]);
            let capability_gate = reg.afforded(&b, &organs, &refs, &caps).contains(&MOVE);
            let retired_rooted_mark_gate = k.id != 0;
            assert_eq!(
                capability_gate, retired_rooted_mark_gate,
                "mode {} ({}): the capability MOVE gate must match the retired rooted-mark gate",
                k.id, k.name
            );
        }
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
    /// tests can show reserves do NOT track mass. Locomotion is the rooted mark, kind id 0 (irrelevant here).
    fn organ_body(mass: (i64, i64), organs: Vec<Part>) -> BodyPlan {
        let mut b = body(mass, vec![0]);
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
