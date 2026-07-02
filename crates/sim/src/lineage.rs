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

//! The species-lineage tree, the container speciation records into (design Parts 25, 17;
//! R-BIOSPHERE fork F7).
//!
//! A [`Lineage`] is a parent-pointer tree of [`SpeciesId`]s: a founder species is a root,
//! and a declared speciation event forks a daughter that points at its parent, so the whole
//! phylogeny of a world is one navigable, append-only history. This is the settled container
//! (F7: a dedicated species module with a parent-pointer lineage tree, keeping the genome
//! engine pure mechanism); the payload each node carries (the gene pool, the composition
//! vector, the climate envelope) and the rule that decides when to speciate are the
//! R-BIOSPHERE dive's to fill, so the container is generic over its payload `T` and holds no
//! ecology of its own.
//!
//! Ids are minted from a monotone counter, so a deterministic epoch (the seed-keyed
//! pre-dawn radiation) that founds and speciates in a fixed order produces the same ids on
//! any machine, and the tree never reuses an id even after a lineage goes extinct (a species
//! stays in the history; extinction is a state its payload carries, not a deletion). Nodes
//! iterate in id order through a [`std::collections::BTreeMap`], so any walk over the
//! lineage is canonical rather than hash-ordered (R-CANON-WALK).

use std::collections::BTreeMap;

/// A species identifier, an index into a [`Lineage`], minted in creation order. Never a
/// closed enum: which species exist is generated world content.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpeciesId(pub u32);

/// One node of the lineage: its payload, its parent (absent for a founder), and its
/// daughters in creation order.
#[derive(Clone, Debug)]
struct Node<T> {
    payload: T,
    parent: Option<SpeciesId>,
    children: Vec<SpeciesId>,
}

/// A parent-pointer tree of species, generic over the per-species payload `T`. Founders are
/// roots; speciation appends daughters. The tree is append-only: extinction is a property a
/// payload carries, so the phylogeny stays a complete history.
#[derive(Clone, Debug)]
pub struct Lineage<T> {
    nodes: BTreeMap<SpeciesId, Node<T>>,
    next: u32,
}

impl<T> Default for Lineage<T> {
    fn default() -> Self {
        Lineage {
            nodes: BTreeMap::new(),
            next: 0,
        }
    }
}

impl<T> Lineage<T> {
    /// An empty lineage.
    pub fn new() -> Lineage<T> {
        Lineage::default()
    }

    /// Found a new root species with a payload, returning its fresh id.
    pub fn found(&mut self, payload: T) -> SpeciesId {
        self.insert(payload, None)
    }

    /// Fork a daughter of `parent` (a declared speciation event), returning its fresh id.
    /// Returns `None` if `parent` is not in the lineage.
    pub fn speciate(&mut self, parent: SpeciesId, payload: T) -> Option<SpeciesId> {
        if !self.nodes.contains_key(&parent) {
            return None;
        }
        let id = self.insert(payload, Some(parent));
        self.nodes
            .get_mut(&parent)
            .expect("parent presence checked above")
            .children
            .push(id);
        Some(id)
    }

    fn insert(&mut self, payload: T, parent: Option<SpeciesId>) -> SpeciesId {
        let id = SpeciesId(self.next);
        self.next += 1;
        self.nodes.insert(
            id,
            Node {
                payload,
                parent,
                children: Vec::new(),
            },
        );
        id
    }

    /// The payload of a species, if present.
    pub fn get(&self, id: SpeciesId) -> Option<&T> {
        self.nodes.get(&id).map(|n| &n.payload)
    }

    /// The mutable payload of a species, if present (for the epoch to drift a pool in place).
    pub fn get_mut(&mut self, id: SpeciesId) -> Option<&mut T> {
        self.nodes.get_mut(&id).map(|n| &mut n.payload)
    }

    /// The parent of a species, or `None` for a founder or an unknown id.
    pub fn parent(&self, id: SpeciesId) -> Option<SpeciesId> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }

    /// The daughters of a species in creation order, or an empty slice for an unknown id.
    pub fn children(&self, id: SpeciesId) -> &[SpeciesId] {
        self.nodes.get(&id).map_or(&[], |n| n.children.as_slice())
    }

    /// The ancestors of a species from its parent up to its root founder, in that order.
    pub fn ancestors(&self, id: SpeciesId) -> Vec<SpeciesId> {
        let mut out = Vec::new();
        let mut cur = self.parent(id);
        while let Some(p) = cur {
            out.push(p);
            cur = self.parent(p);
        }
        out
    }

    /// The founder root of a species' lineage (itself if it is a founder), or `None` for an
    /// unknown id.
    pub fn root(&self, id: SpeciesId) -> Option<SpeciesId> {
        if !self.nodes.contains_key(&id) {
            return None;
        }
        Some(self.ancestors(id).last().copied().unwrap_or(id))
    }

    /// The founder roots, in id order.
    pub fn founders(&self) -> Vec<SpeciesId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.parent.is_none())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Every species id in canonical (id) order.
    pub fn ids(&self) -> impl Iterator<Item = SpeciesId> + '_ {
        self.nodes.keys().copied()
    }

    /// The number of species (living and extinct) in the lineage.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the lineage holds no species.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn founders_get_sequential_ids_and_no_parent() {
        let mut lin: Lineage<&str> = Lineage::new();
        let a = lin.found("fern");
        let b = lin.found("moss");
        assert_eq!((a, b), (SpeciesId(0), SpeciesId(1)));
        assert_eq!(lin.parent(a), None);
        assert_eq!(lin.founders(), vec![a, b]);
        assert_eq!(lin.get(a), Some(&"fern"));
    }

    #[test]
    fn speciation_records_the_parent_and_child() {
        let mut lin: Lineage<u32> = Lineage::new();
        let root = lin.found(0);
        let d1 = lin.speciate(root, 1).unwrap();
        let d2 = lin.speciate(root, 2).unwrap();
        assert_eq!(lin.parent(d1), Some(root));
        assert_eq!(lin.children(root), &[d1, d2]);
        assert_eq!(
            lin.speciate(SpeciesId(999), 9),
            None,
            "unknown parent is rejected"
        );
    }

    #[test]
    fn ancestors_and_root_walk_to_the_founder() {
        let mut lin: Lineage<u32> = Lineage::new();
        let a = lin.found(0);
        let b = lin.speciate(a, 1).unwrap();
        let c = lin.speciate(b, 2).unwrap();
        assert_eq!(lin.ancestors(c), vec![b, a]);
        assert_eq!(lin.root(c), Some(a));
        assert_eq!(lin.root(a), Some(a), "a founder is its own root");
    }

    #[test]
    fn ids_iterate_in_canonical_order() {
        let mut lin: Lineage<u32> = Lineage::new();
        let a = lin.found(0);
        let b = lin.speciate(a, 1).unwrap();
        let c = lin.found(2);
        assert_eq!(lin.ids().collect::<Vec<_>>(), vec![a, b, c]);
    }

    #[test]
    fn ids_are_never_reused() {
        // Even though the container never deletes, confirm the counter is monotone so a
        // future extinction (a payload flag) cannot collide a new species onto an old id.
        let mut lin: Lineage<u32> = Lineage::new();
        let a = lin.found(0);
        let b = lin.speciate(a, 1).unwrap();
        let c = lin.found(2);
        assert_eq!([a.0, b.0, c.0], [0, 1, 2]);
        assert_eq!(lin.len(), 3);
    }

    #[test]
    fn the_same_call_sequence_builds_the_same_tree() {
        let build = || {
            let mut lin: Lineage<u32> = Lineage::new();
            let a = lin.found(10);
            let b = lin.speciate(a, 11).unwrap();
            lin.speciate(b, 12);
            lin.found(13);
            lin.ids()
                .map(|id| (id, *lin.get(id).unwrap(), lin.parent(id).map(|p| p.0)))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            build(),
            build(),
            "a deterministic epoch builds a reproducible phylogeny"
        );
    }
}
