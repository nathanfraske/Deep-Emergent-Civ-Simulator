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

//! The form-and-join floor: the one content dependency the evaluator needs before it can measure a
//! leaf.
//!
//! A [`FormRegistry`] is a geometry catalogue over the wave-1 geometric axes the mechanical floor
//! already carries (`mech.contact_area`, `mech.section_modulus`, `mech.arm_length`,
//! `mech.edge_radius`): each [`FormDef`] binds a form id to a value on each of those axes, so a leaf's
//! primitives contribute geometry the physics laws read. A [`JoinRegistry`] is the catalogue of the
//! ways two members are joined, each [`JoinDef`] carrying a joint transmission efficiency in `[0, 1]`.
//!
//! Both registries follow the physics-registry pattern exactly (design Part 58, `crates/physics`):
//! the registry structure and the geometric-axis vocabulary are fixed Rust, the MEMBERSHIP is data
//! that grows with the world (Principle 11). The [`FormRegistry::dev_seed`] and
//! [`JoinRegistry::dev_seed`] constructors are labelled DEV SEEDS, the same discipline the trace-kind
//! and decision fixtures use: they are stand-ins so the evaluator can be exercised, not owner-authored
//! production geometry. In the full engine a form's geometry is discovered and emergent, and the owner
//! extends the membership; nothing here is a reserved calibration value.

use civsim_core::Fixed;
use std::collections::BTreeMap;

/// The four wave-1 geometric axes a form carries a value on, the mechanical floor's geometry axes
/// (`crates/physics/data/mechanical_floor.toml`). These are the axis ids a [`FormDef`] keys its
/// geometry map by; the leaf dispatch reads them to feed the physics law kernels.
pub const FORM_AXES: [&str; 4] = [
    "mech.contact_area",
    "mech.section_modulus",
    "mech.arm_length",
    "mech.edge_radius",
];

/// A form id: a stable handle for a geometric primitive. A newtype, not an index, so a form survives
/// the registry growing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FormId(pub u32);

/// One form: an id, a human label, and its value on each geometric axis. A missing axis reads as zero
/// (a form that carries no bending section is not a beam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormDef {
    /// The form's stable id.
    pub id: FormId,
    /// The human-readable name.
    pub name: String,
    /// The value on each geometric axis, keyed by axis id (sorted, for a deterministic walk).
    pub geometry: BTreeMap<String, Fixed>,
}

impl FormDef {
    /// The value on a geometric axis, or zero if the form does not carry it.
    #[inline]
    pub fn geo(&self, axis: &str) -> Fixed {
        self.geometry.get(axis).copied().unwrap_or(Fixed::ZERO)
    }
}

/// The geometry catalogue. Ordered by id so every walk is deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FormRegistry {
    forms: BTreeMap<u32, FormDef>,
}

impl FormRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        FormRegistry::default()
    }

    /// Add a form. Returns the id.
    pub fn insert(&mut self, def: FormDef) -> FormId {
        let id = def.id;
        self.forms.insert(id.0, def);
        id
    }

    /// A form by id.
    pub fn get(&self, id: FormId) -> Option<&FormDef> {
        self.forms.get(&id.0)
    }

    /// The forms, in id order.
    pub fn forms(&self) -> impl Iterator<Item = &FormDef> + '_ {
        self.forms.values()
    }

    /// Number of forms.
    pub fn len(&self) -> usize {
        self.forms.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.forms.is_empty()
    }

    /// A labelled DEV SEED of geometric primitives, each value inside the mechanical floor's axis
    /// ranges. Not owner-authored production geometry: a stand-in so the evaluator can be exercised.
    pub fn dev_seed() -> Self {
        let mut reg = FormRegistry::new();
        reg.insert(form(
            0,
            "beam",
            &[
                ("mech.contact_area", "0.01"),
                ("mech.section_modulus", "0.0001"),
                ("mech.arm_length", "1.0"),
                ("mech.edge_radius", "0.001"),
            ],
        ));
        reg.insert(form(
            1,
            "blade",
            &[
                ("mech.contact_area", "0.0001"),
                ("mech.section_modulus", "0.00001"),
                ("mech.arm_length", "0.3"),
                ("mech.edge_radius", "0.0000001"),
            ],
        ));
        reg.insert(form(
            2,
            "point",
            &[
                ("mech.contact_area", "0.00000005"),
                ("mech.section_modulus", "0.000001"),
                ("mech.arm_length", "0.05"),
                ("mech.edge_radius", "0.00000005"),
            ],
        ));
        reg
    }
}

/// A join id: a stable handle for a way of joining two members.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JoinId(pub u32);

/// One join: an id, a name, and a joint transmission efficiency in `[0, 1]` (the fraction of a load
/// the joint carries across before the joint itself becomes the weak link).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinDef {
    /// The join's stable id.
    pub id: JoinId,
    /// The human-readable name.
    pub name: String,
    /// The joint transmission efficiency in `[0, 1]`.
    pub efficiency: Fixed,
}

/// The join catalogue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JoinRegistry {
    joins: BTreeMap<u32, JoinDef>,
}

impl JoinRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        JoinRegistry::default()
    }

    /// Add a join. Returns the id.
    pub fn insert(&mut self, def: JoinDef) -> JoinId {
        let id = def.id;
        self.joins.insert(id.0, def);
        id
    }

    /// A join by id.
    pub fn get(&self, id: JoinId) -> Option<&JoinDef> {
        self.joins.get(&id.0)
    }

    /// The efficiency of a join, or one (a lossless ideal joint) if the id is unknown, so a missing
    /// join never fabricates a loss.
    pub fn efficiency(&self, id: JoinId) -> Fixed {
        self.joins
            .get(&id.0)
            .map(|j| j.efficiency)
            .unwrap_or(Fixed::ONE)
    }

    /// The joins, in id order.
    pub fn joins(&self) -> impl Iterator<Item = &JoinDef> + '_ {
        self.joins.values()
    }

    /// Number of joins.
    pub fn len(&self) -> usize {
        self.joins.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.joins.is_empty()
    }

    /// A labelled DEV SEED of join kinds. Not owner-authored production data.
    pub fn dev_seed() -> Self {
        let mut reg = JoinRegistry::new();
        reg.insert(JoinDef {
            id: JoinId(0),
            name: "welded".to_string(),
            efficiency: Fixed::from_decimal_str("0.95").unwrap(),
        });
        reg.insert(JoinDef {
            id: JoinId(1),
            name: "riveted".to_string(),
            efficiency: Fixed::from_decimal_str("0.8").unwrap(),
        });
        reg.insert(JoinDef {
            id: JoinId(2),
            name: "lashed".to_string(),
            efficiency: Fixed::from_decimal_str("0.5").unwrap(),
        });
        reg
    }
}

fn form(id: u32, name: &str, geo: &[(&str, &str)]) -> FormDef {
    let mut geometry = BTreeMap::new();
    for (axis, val) in geo {
        geometry.insert(
            (*axis).to_string(),
            Fixed::from_decimal_str(val).expect("dev-seed geometry decimal"),
        );
    }
    FormDef {
        id: FormId(id),
        name: name.to_string(),
        geometry,
    }
}
