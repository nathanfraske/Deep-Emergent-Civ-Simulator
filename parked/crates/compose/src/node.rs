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
//! A [`CompositionNode`] is a [`NodeBody::Leaf`] (geometric primitives of one material, joined
//! one way), a [`NodeBody::Composite`] (child designs, referenced by content id, assembled with an
//! assembly material and join), or a [`NodeBody::Transduction`] (an OPAQUE, domain-serialized
//! primitive: `compose` does not interpret its bytes, it only content-addresses them so a distinct
//! transduction mints a distinct node that is promoted, folded, and selected through discovery like
//! any design). Its [`CompositionNode::content_id`] is PURE-CONTENT: a `StateHasher`
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
    /// A transduction: an OPAQUE, domain-serialized primitive affordance. The `compose` substrate is
    /// domain-agnostic and never interprets these bytes; it folds them whole into the content id so a
    /// distinct transduction mints a distinct content-addressed node and CAN be promoted, folded, and
    /// selected through discovery like any other design node. This is the SUBSTRATE for a multi-axis
    /// affordance (a `compose` tree of transduction leaves); the bundling of leaves into a purpose-laden
    /// affordance is left to EMERGE through the discovery loop under selection, not authored or proven
    /// here (the Tier-C seam, see `civsim-sim`'s affordance-percept module). The domain (`civsim-sim`)
    /// serializes its affordance transduction
    /// into these canonical bytes; the serialization is the domain's contract and MUST be deterministic
    /// and stable (fixed field order, fixed-point-exact, no map-iteration-order dependence) so identical
    /// transductions mint identical ids across runs and workers, the same discipline `state_hash`
    /// holds. `compose` asserts nothing about a sensor's physical fitness: the physics evaluator returns
    /// an empty interface vector and the zero-load structural viability (a safety of one, the
    /// unconstrained maximum, because a sensor bears no structural load), so the structural promotion
    /// gate never rejects it AS a structure; its sensory fitness is evaluated in the domain, not here.
    Transduction {
        /// The domain's canonical serialization of the transduction. Never interpreted by `compose`;
        /// folded whole (length-prefixed) into the content id.
        canonical: Vec<u8>,
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
                            // Sort children by their FULL per-child content digest (target, transform, AND overrides)
                            // so a permuted assembly canonicalizes to one id even when two children share a target
                            // and transform but carry different overrides. Keying the sort on (target, transform)
                            // alone left such children in arrival order, so swapping them changed the id; the digest
                            // key folds the overrides in, so order is immaterial and two children that differ only in
                            // overrides still hash to distinct ids.
            let mut sorted: Vec<(u128, &ComponentRef)> =
                children.iter().map(|c| (child_key(c), c)).collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (_, c) in sorted {
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
        NodeBody::Transduction { canonical } => {
            h.write_u32(3); // transduction tag
                            // Length-prefix the opaque bytes so the byte stream is unambiguous (no two
                            // distinct byte strings share a fold), then fold them verbatim. compose does
                            // not parse them; the domain's canonical serialization carries the meaning.
            h.write_u64(canonical.len() as u64);
            h.write_bytes(canonical);
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

/// The full per-child content digest a composite orders its children by: the target, the transform,
/// and every override, in fixed order. Two children that differ in ANY of these get distinct keys,
/// so the child sort is a total canonical order over distinct children and a permuted assembly folds
/// identically. Distinct from the child's own `target` (a child's content id): this also folds the
/// per-assembly transform and overrides, which are not part of the child design's own address.
fn child_key(c: &ComponentRef) -> u128 {
    let mut h = StateHasher::new();
    write_u128(&mut h, c.target);
    h.write_u32(c.transform.0);
    for o in &c.overrides {
        h.write_fixed(*o);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interface::PortVector;

    fn leaf(material: u128) -> CompositionNode {
        CompositionNode::new(
            IntentRef(0),
            NodeBody::Leaf {
                primitives: vec![FormId(1)],
                material,
                joining: JoinId(0),
            },
            PortVector::from_slots(vec![]),
            vec![],
        )
    }

    fn child(target: u128, transform: u32, overrides: Vec<Fixed>) -> ComponentRef {
        ComponentRef {
            target,
            transform: TransformId(transform),
            overrides,
        }
    }

    fn composite(children: Vec<ComponentRef>) -> CompositionNode {
        CompositionNode::new(
            IntentRef(7),
            NodeBody::Composite {
                children,
                assembly_material: 99,
                assembly_join: JoinId(0),
            },
            PortVector::from_slots(vec![]),
            vec![],
        )
    }

    #[test]
    fn permuting_same_target_same_transform_children_with_distinct_overrides_is_canonical() {
        // Regression (audit defect 3): two children sharing a target and transform but carrying
        // DIFFERENT overrides used to keep their arrival order in the content fold, so swapping them
        // changed the id. Folding the full per-child content (overrides included) into the ordering
        // key makes the two assembly orders canonicalize to one id.
        let a = child(0xAA, 3, vec![Fixed::from_int(1)]);
        let b = child(0xAA, 3, vec![Fixed::from_int(2)]);
        let forward = composite(vec![a.clone(), b.clone()]);
        let reversed = composite(vec![b, a]);
        assert_eq!(
            forward.content_id(),
            reversed.content_id(),
            "child order does not change the content id"
        );
    }

    #[test]
    fn children_differing_only_in_overrides_produce_distinct_ids() {
        // The other half: overrides are content. Two assemblies with the same single child target and
        // transform but different overrides must hash differently.
        let one = composite(vec![child(0xBB, 1, vec![Fixed::from_int(1)])]);
        let two = composite(vec![child(0xBB, 1, vec![Fixed::from_int(5)])]);
        assert_ne!(
            one.content_id(),
            two.content_id(),
            "a different override is a different design"
        );
    }

    #[test]
    fn the_material_content_ids_are_unaffected_by_the_leaf_helper() {
        // A sanity anchor so the fixtures are meaningful: two leaves of different material differ.
        assert_ne!(leaf(1).content_id(), leaf(2).content_id());
    }

    fn transduction(canonical: Vec<u8>) -> CompositionNode {
        CompositionNode::new(
            IntentRef(0),
            NodeBody::Transduction { canonical },
            PortVector::from_slots(vec![]),
            vec![],
        )
    }

    #[test]
    fn distinct_transduction_bytes_mint_distinct_ids_and_identical_bytes_agree() {
        // The content-addressing contract: a distinct opaque serialization is a distinct node, and the
        // same bytes (the domain's stable canonical form) recompute to the same id.
        let a = transduction(vec![1, 2, 3]);
        let b = transduction(vec![1, 2, 4]);
        assert_ne!(
            a.content_id(),
            b.content_id(),
            "different bytes, different id"
        );
        assert_eq!(
            a.content_id(),
            transduction(vec![1, 2, 3]).content_id(),
            "identical bytes recompute to the identical id"
        );
        assert_eq!(a.id, a.content_id(), "stored id matches recomputed");
    }

    #[test]
    fn the_length_prefix_disambiguates_byte_boundaries() {
        // Length-prefixing the opaque bytes means no two distinct serializations can collide by sharing
        // a fold: [1] followed by [2,3] cannot masquerade as [1,2] followed by [3]. Distinct byte
        // strings mint distinct ids even when a naive concatenation would run together.
        let ab = transduction(vec![1, 2, 3]);
        let cd = transduction(vec![1, 2, 3, 0]);
        assert_ne!(ab.content_id(), cd.content_id());
    }

    #[test]
    fn a_transduction_leaf_never_collides_with_a_geometric_leaf() {
        // The tag byte partitions the three bodies: an empty-byte transduction and a geometric leaf are
        // distinct designs even though both are "leaves", because tag 3 differs from tag 1.
        assert_ne!(transduction(vec![]).content_id(), leaf(0).content_id());
    }
}
