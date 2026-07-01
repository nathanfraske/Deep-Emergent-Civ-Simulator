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

//! The located-identity join, a tile to its occupants (design Parts 6, 56; the map program's
//! phase 4).
//!
//! The simulation ([`crate`]) models beings and biosphere pools; the spatial layer
//! ([`civsim_world`]) is the map they inhabit. This module is the join between them: a
//! [`LocationIndex`] mapping a world [`civsim_world::Coord3`] to the occupants there, and each
//! occupant back to its coordinate, so the superfine zoom can ask a tile who stands on it and
//! a being can be found on the map. It is deliberately entity-world-agnostic: an occupant is a
//! plain [`OccupantId`] keyed by a stable id, so the index works whether occupants are later
//! held in an archetypal entity world (hecs) or grown structure-of-arrays. That choice (the
//! hecs-versus-grow fork) stays open past this phase.
//!
//! Every map is a [`std::collections::BTreeMap`] and every returned list is in canonical
//! (id or coordinate) order, so a walk over occupants is reproducible rather than hash-ordered
//! (R-CANON-WALK), and the index writes no randomness and reads no camera, so it is part of
//! canonical state without perturbing it (Principle 10).

use std::collections::{BTreeMap, BTreeSet};

use civsim_core::StableId;
use civsim_world::{ChunkCoord, Coord3, CHUNK};

/// A located occupant: a being or a promoted biosphere organism, keyed by a stable id and
/// tagged by kind so beings and organisms share one index without colliding. The kind is a
/// small fixed tag (an identity discriminator), not world content, so it is an enum.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct OccupantId {
    pub kind: OccupantKind,
    pub id: StableId,
}

/// What an occupant is. A fixed identity discriminator (not emergent world content).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum OccupantKind {
    /// A modelled being (a person, an animal).
    Being,
    /// A promoted biosphere organism (a notable plant or creature drawn from a pool).
    Organism,
}

impl OccupantId {
    /// A being occupant.
    pub fn being(id: StableId) -> OccupantId {
        OccupantId {
            kind: OccupantKind::Being,
            id,
        }
    }

    /// A promoted-organism occupant.
    pub fn organism(id: StableId) -> OccupantId {
        OccupantId {
            kind: OccupantKind::Organism,
            id,
        }
    }
}

/// The two-way index between coordinates and occupants. A coordinate maps to the set of
/// occupants on it; an occupant maps to its single coordinate.
#[derive(Clone, Debug, Default)]
pub struct LocationIndex {
    at: BTreeMap<Coord3, BTreeSet<OccupantId>>,
    of: BTreeMap<OccupantId, Coord3>,
}

impl LocationIndex {
    /// An empty index.
    pub fn new() -> LocationIndex {
        LocationIndex::default()
    }

    /// Place or move an occupant to a coordinate, returning its previous coordinate if it was
    /// already placed. Idempotent for an unchanged coordinate.
    pub fn place(&mut self, occ: OccupantId, coord: Coord3) -> Option<Coord3> {
        let prev = self.of.insert(occ, coord);
        if let Some(p) = prev {
            if p == coord {
                return Some(p);
            }
            if let Some(set) = self.at.get_mut(&p) {
                set.remove(&occ);
                if set.is_empty() {
                    self.at.remove(&p);
                }
            }
        }
        self.at.entry(coord).or_default().insert(occ);
        prev
    }

    /// Remove an occupant from the map, returning its coordinate if it was placed.
    pub fn remove(&mut self, occ: OccupantId) -> Option<Coord3> {
        let coord = self.of.remove(&occ)?;
        if let Some(set) = self.at.get_mut(&coord) {
            set.remove(&occ);
            if set.is_empty() {
                self.at.remove(&coord);
            }
        }
        Some(coord)
    }

    /// The coordinate of an occupant, if placed.
    pub fn coord_of(&self, occ: OccupantId) -> Option<Coord3> {
        self.of.get(&occ).copied()
    }

    /// The occupants on a coordinate, in canonical id order.
    pub fn occupants(&self, coord: Coord3) -> Vec<OccupantId> {
        self.at
            .get(&coord)
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// The occupants anywhere in a chunk, in canonical (coordinate, then id) order. This is the
    /// query the located-identity layer keys on (the Part 6 chunk a tile belongs to).
    pub fn occupants_in_chunk(&self, chunk: ChunkCoord) -> Vec<OccupantId> {
        let (x0, y0) = (chunk.cx * CHUNK, chunk.cy * CHUNK);
        let mut out = Vec::new();
        // Bounded scan over the chunk's coordinate range on the occupant's own layer set; the
        // BTreeMap keeps the walk canonical.
        for (&coord, set) in &self.at {
            if coord.x >= x0 && coord.x < x0 + CHUNK && coord.y >= y0 && coord.y < y0 + CHUNK {
                out.extend(set.iter().copied());
            }
        }
        out
    }

    /// The number of placed occupants.
    pub fn len(&self) -> usize {
        self.of.len()
    }

    /// Whether no occupant is placed.
    pub fn is_empty(&self) -> bool {
        self.of.is_empty()
    }

    /// Every occupied coordinate, in canonical order.
    pub fn occupied(&self) -> impl Iterator<Item = Coord3> + '_ {
        self.at.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(n: u64) -> OccupantId {
        OccupantId::being(StableId(n))
    }

    #[test]
    fn place_and_query_a_tile() {
        let mut idx = LocationIndex::new();
        let c = Coord3::ground(3, 4);
        assert_eq!(idx.place(b(1), c), None);
        assert_eq!(idx.place(b(2), c), None);
        assert_eq!(idx.occupants(c), vec![b(1), b(2)], "canonical id order");
        assert_eq!(idx.coord_of(b(1)), Some(c));
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn moving_updates_both_directions() {
        let mut idx = LocationIndex::new();
        let a = Coord3::ground(0, 0);
        let d = Coord3::ground(9, 9);
        idx.place(b(1), a);
        assert_eq!(idx.place(b(1), d), Some(a), "returns the previous coordinate");
        assert_eq!(idx.occupants(a), vec![], "left the old tile");
        assert_eq!(idx.occupants(d), vec![b(1)], "arrived at the new tile");
        assert_eq!(idx.coord_of(b(1)), Some(d));
    }

    #[test]
    fn removing_clears_the_tile_and_the_reverse() {
        let mut idx = LocationIndex::new();
        let c = Coord3::ground(2, 2);
        idx.place(b(1), c);
        assert_eq!(idx.remove(b(1)), Some(c));
        assert!(idx.occupants(c).is_empty());
        assert_eq!(idx.coord_of(b(1)), None);
        assert!(idx.is_empty());
        assert_eq!(idx.remove(b(1)), None, "removing an absent occupant is None");
    }

    #[test]
    fn beings_and_organisms_share_the_index_without_colliding() {
        let mut idx = LocationIndex::new();
        let c = Coord3::ground(1, 1);
        let being = OccupantId::being(StableId(7));
        let organism = OccupantId::organism(StableId(7));
        idx.place(being, c);
        idx.place(organism, c);
        assert_eq!(idx.occupants(c).len(), 2, "same raw id, different kind, both present");
    }

    #[test]
    fn chunk_query_gathers_the_block() {
        let mut idx = LocationIndex::new();
        // Two occupants in chunk (0,0), one in chunk (1,0).
        idx.place(b(1), Coord3::ground(0, 0));
        idx.place(b(2), Coord3::ground(CHUNK - 1, CHUNK - 1));
        idx.place(b(3), Coord3::ground(CHUNK, 0));
        let c0 = idx.occupants_in_chunk(ChunkCoord { cx: 0, cy: 0 });
        assert_eq!(c0, vec![b(1), b(2)], "the block gathers its two, canonically");
        let c1 = idx.occupants_in_chunk(ChunkCoord { cx: 1, cy: 0 });
        assert_eq!(c1, vec![b(3)]);
    }
}
