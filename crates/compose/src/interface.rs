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

//! The interface substrate: the axes a composed design exposes as composition ports.
//!
//! An [`InterfaceRegistry`] is an OPEN DATA REGISTRY of interface axes (the owner's decision, baked
//! in). Each [`InterfaceAxisDef`] binds an interface-axis id to a physics [`civsim_physics::QuantityAxis`] id (the
//! LawPort role-to-axis pattern of the physics substrate), tags it with a [`Direction`] (whether the
//! design offers capacity on the axis or demands it) and a free-form `role`, and names the
//! [`CombinatorKey`] topology by which a composite folds that axis across its children. The registry is
//! ORDERED, so its length is the fixed width of a [`PortVector`]: every design under one interface
//! substrate carries the same-width vector, one slot per interface axis.
//!
//! This is where Principle 8 lives at the composition layer. The interface-axis MEMBERSHIP is data
//! with a labelled dev seed and owner-set membership, never a hardcoded closed list. Two peoples with
//! different interface substrates (one exposing an exotic axis the other cannot) produce
//! different-width vectors, so the SAME intent stream under ONE physics yields a different technology
//! library for each. The evaluate mechanism over the vector is fixed Rust; which axes exist is data.

use crate::combinator::CombinatorKey;
use crate::interval::Interval;
use civsim_core::StateHasher;

/// Whether a port offers capacity on an axis or demands it. An offer is what the design provides; a
/// demand is what it requires. A composite is viable on an axis when the aggregated offer covers the
/// aggregated demand; the shortfall drives the interface-mismatch penalty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The design provides capacity on the axis.
    Offer,
    /// The design requires capacity on the axis.
    Demand,
}

/// An interface-axis id: a stable handle, not an index, so the axis survives the registry growing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterfaceAxisId(pub u32);

/// One interface axis: the LawPort role-to-axis binding of the composition layer. It names the
/// physics quantity the port reads (for provenance and units), the direction it is exposed as, its
/// role, the combinator topology that folds it across children, and whether it is an additive
/// conserved quantity (a mass, an envelope, an energy budget) that must satisfy parent equals the sum
/// of children exactly across a tier boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceAxisDef {
    /// The interface-axis id.
    pub id: InterfaceAxisId,
    /// The human-readable name.
    pub name: String,
    /// The physics [`QuantityAxis`](civsim_physics::QuantityAxis) id this port is bound to, for units
    /// and provenance. The evaluator confirms the axis exists in the physics registry.
    pub physics_axis: String,
    /// The free-form role, matched by the whole-system proxies against the ports they read.
    pub role: String,
    /// The direction the axis is exposed as.
    pub direction: Direction,
    /// The combinator topology that folds this axis across a composite's children.
    pub combinator: CombinatorKey,
    /// Whether the axis is an additive conserved quantity (parent equals the exact sum of children).
    pub additive: bool,
}

/// The interface-axis catalogue. Ordered by id, so its length is the fixed [`PortVector`] width and
/// every walk is deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterfaceRegistry {
    axes: Vec<InterfaceAxisDef>,
}

impl InterfaceRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        InterfaceRegistry::default()
    }

    /// Add an interface axis, keeping the axes sorted by id so the width order is canonical.
    pub fn insert(&mut self, def: InterfaceAxisDef) {
        match self.axes.binary_search_by_key(&def.id.0, |a| a.id.0) {
            Ok(i) => self.axes[i] = def,
            Err(i) => self.axes.insert(i, def),
        }
    }

    /// The fixed width: the number of interface axes, and the length of every [`PortVector`] built
    /// against this registry.
    pub fn width(&self) -> usize {
        self.axes.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty()
    }

    /// The axes, in id order (the slot order of a [`PortVector`]).
    pub fn axes(&self) -> impl Iterator<Item = &InterfaceAxisDef> + '_ {
        self.axes.iter()
    }

    /// The axis at a slot index.
    pub fn axis_at(&self, slot: usize) -> Option<&InterfaceAxisDef> {
        self.axes.get(slot)
    }

    /// The slot index of an interface-axis id, if present.
    pub fn slot_of(&self, id: InterfaceAxisId) -> Option<usize> {
        self.axes.binary_search_by_key(&id.0, |a| a.id.0).ok()
    }

    /// The first axis whose role matches, if any (the proxies read ports by role).
    pub fn slot_of_role(&self, role: &str) -> Option<usize> {
        self.axes.iter().position(|a| a.role == role)
    }

    /// An empty (all-zero) port vector of this registry's width, the identity a leaf fills in.
    pub fn empty_vector(&self) -> PortVector {
        PortVector {
            slots: self
                .axes
                .iter()
                .map(|a| PortSlot {
                    axis: a.id,
                    direction: a.direction,
                    interval: Interval::ZERO,
                })
                .collect(),
        }
    }

    /// A stable content hash of the interface substrate, so two peoples with different substrates are
    /// distinguishable and a promoted library can be keyed to the substrate it emerged under.
    pub fn content_id(&self) -> u128 {
        let mut h = StateHasher::new();
        for a in &self.axes {
            h.write_u32(a.id.0);
            h.write_bytes(a.name.as_bytes());
            h.write_bytes(a.physics_axis.as_bytes());
            h.write_bytes(a.role.as_bytes());
            h.write_u32(match a.direction {
                Direction::Offer => 0,
                Direction::Demand => 1,
            });
            h.write_u32(a.combinator.0);
            h.write_u32(a.additive as u32);
        }
        h.finish()
    }

    /// The dev-seed axis-id constants, so the leaf dispatch and the tests can name a port without a
    /// magic number.
    /// The structural-stress-margin offer axis (bound to `mat.fracture_strength`).
    pub const AXIS_MARGIN: InterfaceAxisId = InterfaceAxisId(0);
    /// The toughness (absorbable-energy) margin offer axis (bound to `mat.fracture_energy`).
    pub const AXIS_TOUGHNESS: InterfaceAxisId = InterfaceAxisId(1);
    /// The envelope-mass demand axis (bound to `mech.mass`), additive and conserved.
    pub const AXIS_MASS: InterfaceAxisId = InterfaceAxisId(2);
    /// The transmission-efficiency offer axis (bound to `mech.restitution`, a dimensionless ratio).
    pub const AXIS_EFFICIENCY: InterfaceAxisId = InterfaceAxisId(3);
    /// The exotic stiffness-resonance offer axis (bound to `mat.elastic_modulus`), present only in
    /// the exotic-people dev seed.
    pub const AXIS_STIFFNESS: InterfaceAxisId = InterfaceAxisId(4);

    /// A labelled DEV SEED of a base people's interface substrate: the stress margin, the toughness
    /// margin, the envelope mass, and the transmission efficiency. Not owner-authored production
    /// membership. The two structural-margin axes are what a hard-brittle and a tough-ductile material
    /// trade off on, so the same design diverges by material.
    pub fn dev_seed_base() -> Self {
        use crate::combinator::CombinatorRegistry as C;
        let mut reg = InterfaceRegistry::new();
        reg.insert(InterfaceAxisDef {
            id: Self::AXIS_MARGIN,
            name: "stress_margin".to_string(),
            physics_axis: "mat.fracture_strength".to_string(),
            role: "margin".to_string(),
            direction: Direction::Offer,
            combinator: C::KEY_LIMITING_MIN,
            additive: false,
        });
        reg.insert(InterfaceAxisDef {
            id: Self::AXIS_TOUGHNESS,
            name: "toughness_margin".to_string(),
            physics_axis: "mat.fracture_energy".to_string(),
            role: "toughness".to_string(),
            direction: Direction::Offer,
            combinator: C::KEY_LIMITING_MIN,
            additive: false,
        });
        reg.insert(InterfaceAxisDef {
            id: Self::AXIS_MASS,
            name: "envelope_mass".to_string(),
            physics_axis: "mech.mass".to_string(),
            role: "budget".to_string(),
            direction: Direction::Demand,
            combinator: C::KEY_CONSERVED_BUDGET,
            additive: true,
        });
        reg.insert(InterfaceAxisDef {
            id: Self::AXIS_EFFICIENCY,
            name: "transmission_efficiency".to_string(),
            physics_axis: "mech.restitution".to_string(),
            role: "chain_efficiency".to_string(),
            direction: Direction::Offer,
            combinator: C::KEY_EFFICIENCY_PRODUCT,
            additive: false,
        });
        reg
    }

    /// A labelled DEV SEED of an EXOTIC people's interface substrate: the base axes plus one exotic
    /// stiffness-resonance axis the base people cannot expose. This is the Principle 8 demonstration
    /// surface: the same intent stream under one physics yields a different library for this people.
    pub fn dev_seed_exotic() -> Self {
        use crate::combinator::CombinatorRegistry as C;
        let mut reg = Self::dev_seed_base();
        reg.insert(InterfaceAxisDef {
            id: Self::AXIS_STIFFNESS,
            name: "stiffness_resonance".to_string(),
            physics_axis: "mat.elastic_modulus".to_string(),
            role: "resonance_input".to_string(),
            direction: Direction::Offer,
            combinator: C::KEY_SATURATING_SUM,
            additive: false,
        });
        reg
    }
}

/// One slot of a port vector: the interface axis it fills, the direction, and the interval-valued
/// quantity on that axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortSlot {
    /// The interface axis this slot is for.
    pub axis: InterfaceAxisId,
    /// The direction (offer or demand).
    pub direction: Direction,
    /// The interval-valued quantity.
    pub interval: Interval,
}

/// A fixed-width vector of `(interface-axis, direction, interval)` slots, one per axis of the
/// interface substrate it was built against. The width is the substrate's width, so a design's whole
/// interface is one vector the combinators fold and the proxies read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortVector {
    slots: Vec<PortSlot>,
}

impl PortVector {
    /// A vector from its slots (assumed already in registry-slot order).
    pub fn from_slots(slots: Vec<PortSlot>) -> Self {
        PortVector { slots }
    }

    /// The number of slots (the substrate width).
    pub fn width(&self) -> usize {
        self.slots.len()
    }

    /// The slots, in order.
    pub fn slots(&self) -> &[PortSlot] {
        &self.slots
    }

    /// A mutable view of the slots, for the leaf to fill.
    pub fn slots_mut(&mut self) -> &mut [PortSlot] {
        &mut self.slots
    }

    /// The interval at a slot index, or the zero interval if out of range.
    pub fn interval_at(&self, slot: usize) -> Interval {
        self.slots
            .get(slot)
            .map(|s| s.interval)
            .unwrap_or(Interval::ZERO)
    }

    /// Fold the vector into a content hash, in slot order.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for s in &self.slots {
            h.write_u32(s.axis.0);
            h.write_u32(match s.direction {
                Direction::Offer => 0,
                Direction::Demand => 1,
            });
            s.interval.hash_into(h);
        }
    }

    /// The content id of the vector alone (the aggregated interface state).
    pub fn content_id(&self) -> u128 {
        let mut h = StateHasher::new();
        self.hash_into(&mut h);
        h.finish()
    }
}

/// A small helper the combinator fold and the tests share: gather one slot's intervals across a set
/// of child vectors, dropping the vectors that are narrower than the slot (a child under a smaller
/// substrate does not carry the exotic axis).
pub(crate) fn gather_slot(children: &[(u128, PortVector)], slot: usize) -> Vec<(u128, Interval)> {
    let mut out = Vec::with_capacity(children.len());
    for (id, v) in children {
        if slot < v.width() {
            out.push((*id, v.interval_at(slot)));
        }
    }
    out
}
