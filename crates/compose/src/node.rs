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

//! The composition node: a design as a tree over the form-and-join floor and the material substrate.
//!
//! A [`CompositionNode`] is either a [`NodeBody::Leaf`] (geometric primitives of one material, joined
//! one way) or a [`NodeBody::Composite`] (child designs, referenced by content id, assembled with an
//! assembly material and join). Its [`CompositionNode::content_id`] is PURE-CONTENT: a `StateHasher`
//! digest over the canonical body (the sorted primitive and child ids, the material content ids, the
//! form and join ids, the transforms and overrides, and the params in fixed order), with the
//! `master_seed` OMITTED. This matches the physics [`Substance`](civsim_physics::Substance) content-id
//! pattern (a pure function of the physical content, not the human label or the world seed), so the
//! same design has the same id on every machine and across worlds, the property the cross-world
//! deduplication and the evaluator's memoisation both rely on.
//!
//! The [`IntentRef`] is opaque, non-authoritative provenance (which desire the design was tried
//! against). It is NEVER read into the port vector and NEVER folded into the content id, so two
//! designs that are physically identical but were reached from different intents deduplicate to one
//! design. (Owner ratification flag: the pure-content, seed-omitted choice is surfaced for sign-off,
//! matching `Substance::content_id`.)

use crate::form::{FormId, JoinId};
use crate::interface::PortVector;
use civsim_core::{Fixed, StateHasher};

/// Opaque, non-authoritative provenance: the intent a design was tried against. Never read into the
/// vector, never folded into the content id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IntentRef(pub u128);

/// A transform id applied to a child in a composite (a placement or orientation), a data-defined
/// handle. Folded into the content id so a different placement is a different design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransformId(pub u32);

/// A reference to a child design by its content id, with the transform placing it and any parameter
/// overrides it carries in the assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRef {
    /// The child's content id (the pure-content address).
    pub target: u128,
    /// The placement transform.
    pub transform: TransformId,
    /// Parameter overrides applied to the child in this assembly, positional, in fixed order.
    pub overrides: Vec<Fixed>,
}

/// The body of a node: a leaf primitive or a composite of children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeBody {
    /// A leaf: geometric primitives of one material, joined one way.
    Leaf {
        /// The geometric primitives (form ids).
        primitives: Vec<FormId>,
        /// The material, a physics [`Substance`](civsim_physics::Substance) content id.
        material: u128,
        /// The join binding the primitives.
        joining: JoinId,
    },
    /// A composite: child designs assembled with an assembly material and join.
    Composite {
        /// The children, referenced by content id.
        children: Vec<ComponentRef>,
        /// The material of the assembly itself (bracketry, weld, matrix), a `Substance` content id.
        assembly_material: u128,
        /// The join binding the children.
        assembly_join: JoinId,
    },
}

/// A composition node: one design in the technology library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionNode {
    /// The content id, set to [`CompositionNode::content_id`] at construction. It is the pure-content
    /// address the memo and the child references key on.
    pub id: u128,
    /// Opaque provenance, never folded into the content id or the vector.
    pub intent: IntentRef,
    /// The body.
    pub body: NodeBody,
    /// The declared interface contract: which axes the design offers and demands, and the demand
    /// intervals it commits to. The evaluator overwrites the offer slots with the physics-computed
    /// achieved values; the demand slots are the design's declared draws. Derived from the interface
    /// substrate and the params, so it carries no content the params do not, and it is not folded
    /// into the content id.
    pub ports: PortVector,
    /// The design's scalar parameters, in fixed order (the reference-load bounds, the declared
    /// envelope mass, and any further declared quantities). Folded into the content id.
    pub param: Vec<Fixed>,
}

impl CompositionNode {
    /// Build a node, computing and storing its pure-content id. The `ports` contract is taken as
    /// given (typically the interface substrate's template with the declared demands filled).
    pub fn new(intent: IntentRef, body: NodeBody, ports: PortVector, param: Vec<Fixed>) -> Self {
        let id = compute_content_id(&body, &param);
        CompositionNode {
            id,
            intent,
            body,
            ports,
            param,
        }
    }

    /// The pure-content id: a `StateHasher` digest over the canonical body and params, with the
    /// intent and the master seed omitted. Recomputing it always agrees with the stored `id`.
    pub fn content_id(&self) -> u128 {
        compute_content_id(&self.body, &self.param)
    }
}

/// The canonical content fold. Primitives and children are sorted (by id, by target) so a permuted
/// assembly canonicalizes to the same id; overrides and params are positional, folded in their given
/// order. The intent and the master seed are never folded.
fn compute_content_id(body: &NodeBody, param: &[Fixed]) -> u128 {
    let mut h = StateHasher::new();
    match body {
        NodeBody::Leaf {
            primitives,
            material,
            joining,
        } => {
            h.write_u32(1); // leaf tag
            let mut forms: Vec<u32> = primitives.iter().map(|f| f.0).collect();
            forms.sort_unstable();
            for f in forms {
                h.write_u32(f);
            }
            h.write_u64(u64::MAX); // separator
            write_u128(&mut h, *material);
            h.write_u32(joining.0);
        }
        NodeBody::Composite {
            children,
            assembly_material,
            assembly_join,
        } => {
            h.write_u32(2); // composite tag
                            // Sort children by target so a permuted assembly hashes identically; the overrides ride
                            // with their target, so a re-sort cannot detach an override from its child.
            let mut sorted: Vec<&ComponentRef> = children.iter().collect();
            sorted.sort_by(|a, b| {
                a.target
                    .cmp(&b.target)
                    .then(a.transform.0.cmp(&b.transform.0))
            });
            for c in sorted {
                write_u128(&mut h, c.target);
                h.write_u32(c.transform.0);
                h.write_u64(0); // separator
                for o in &c.overrides {
                    h.write_fixed(*o);
                }
                h.write_u64(u64::MAX); // override terminator
            }
            h.write_u64(u64::MAX); // separator
            write_u128(&mut h, *assembly_material);
            h.write_u32(assembly_join.0);
        }
    }
    h.write_u64(u64::MAX); // param separator
    for p in param {
        h.write_fixed(*p);
    }
    h.finish()
}

#[inline]
fn write_u128(h: &mut StateHasher, v: u128) {
    h.write_bytes(&v.to_le_bytes());
}
