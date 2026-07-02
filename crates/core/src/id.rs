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

//! Process-wide identity and the stable-id registry (design Part 2.1, Part 11).
//!
//! There are two id concepts and conflating them causes pain later, so they are
//! separated from the start. A [`StableId`] names a conceptual entity for its
//! entire existence and beyond; it never changes and is never reused, so the event
//! log, belief provenance, and relationship edges can reference it across
//! promotion, demotion, save, and load. A live ECS entity handle (here
//! [`EntityHandle`], `hecs::Entity` in the full build) is fast but unstable across
//! promotion and not suitable for serialization. The [`Registry`] bridges them.

use crate::hash::StateHasher;
#[allow(clippy::disallowed_types)] // R-CANON-WALK opt-out, justified below
use std::collections::HashMap;

/// A process-wide, monotonically assigned id that names a conceptual entity (a
/// person, an artifact, a building, a culture) for its entire existence and beyond.
/// It never changes and is never reused.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct StableId(pub u64);

/// A pool of aggregated, anonymous individuals (design Part 11.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PoolId(pub u32);

/// A live entity handle in the ECS. In the full build this is `hecs::Entity`; the
/// bedrock keeps an opaque handle so the registry and the identity rules can be
/// built and tested before the ECS crate is wired in.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityHandle(pub u64);

/// Where a [`StableId`] currently lives (design Part 2.1).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum EntityLocation {
    /// Currently a full ECS entity.
    Promoted(EntityHandle),
    /// Currently summarized inside an aggregate pool.
    Pooled { pool: PoolId, slot: u32 },
    /// Recorded in history, with no live representation.
    Retired,
}

/// The component a promoted entity carries so the common direction (live entity to
/// stable id) is a component read with no map lookup (design Part 2.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StableRef(pub StableId);

/// The stable-id registry: mints ids and tracks each id's current location.
#[derive(Default)]
pub struct Registry {
    next_id: u64,
    // The registry is walked only via entries_sorted / hash_into, which iterate in
    // StableId order; a bare walk of this map never reaches a state hash (R-CANON-WALK).
    #[allow(clippy::disallowed_types)]
    locations: HashMap<StableId, EntityLocation>,
}

impl Registry {
    /// A fresh registry. Ids begin at 0.
    pub fn new() -> Self {
        Registry::default()
    }

    /// Mint a new id. Ids are monotonic and never reused, even after an id is
    /// retired, which is what lets log references and edges resolve forever.
    pub fn mint(&mut self) -> StableId {
        let id = StableId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("StableId space exhausted");
        id
    }

    /// Record where an id currently lives.
    pub fn set_location(&mut self, id: StableId, loc: EntityLocation) {
        self.locations.insert(id, loc);
    }

    /// The current location of an id, if the registry has seen it.
    pub fn locate(&self, id: StableId) -> Option<EntityLocation> {
        self.locations.get(&id).copied()
    }

    /// Whether an id resolves to any recorded location. This is the weak form: it
    /// confirms the id is tracked, not that the location it names is still live. A
    /// two-tier subsystem that merges or removes pools should also check that a
    /// `Pooled` location names a pool that still exists (see the two-tier world's
    /// liveness check); use [`Registry::repoint_pool`] to keep such locations valid.
    pub fn resolves(&self, id: StableId) -> bool {
        self.locations.contains_key(&id)
    }

    /// Repoint every `Pooled` location from one pool to another, used when pools
    /// merge so that a demoted entity's location does not name a pool that has been
    /// removed (the referential-integrity invariant of Part 58).
    pub fn repoint_pool(&mut self, from: PoolId, to: PoolId) {
        for loc in self.locations.values_mut() {
            if let EntityLocation::Pooled { pool, .. } = loc {
                if *pool == from {
                    *pool = to;
                }
            }
        }
    }

    /// Number of ids the registry is tracking.
    pub fn tracked(&self) -> usize {
        self.locations.len()
    }

    /// The next id that would be minted, for snapshotting.
    pub fn next_raw(&self) -> u64 {
        self.next_id
    }

    /// Every tracked id and its location, in ascending `StableId` order. This is
    /// the canonical-walk accessor: it is the only sanctioned way to iterate the
    /// registry for a hash or a canonical fold, because the backing map's own
    /// iteration order is a determinism trap (design Part 3.5, R-CANON-WALK).
    pub fn entries_sorted(&self) -> Vec<(StableId, EntityLocation)> {
        let mut v: Vec<(StableId, EntityLocation)> =
            self.locations.iter().map(|(k, v)| (*k, *v)).collect();
        v.sort_by_key(|(id, _)| *id);
        v
    }

    /// Fold the registry into a state hash in canonical order: the high-water mark
    /// first (so a reload that restores it cannot reuse an id), then each id and
    /// location in ascending id order.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u64(self.next_id);
        for (id, loc) in self.entries_sorted() {
            h.write_stable(id);
            loc.hash_into(h);
        }
    }

    /// Rebuild a registry from a snapshot, restoring the id high-water mark
    /// authoritatively rather than inferring it from the live entries, so a reload
    /// never reuses an id (the save half of the Part 58 foundation, R-SAVE-SCHEMA).
    pub fn restore(
        next_id: u64,
        entries: impl IntoIterator<Item = (StableId, EntityLocation)>,
    ) -> Self {
        #[allow(clippy::disallowed_types)] // R-CANON-WALK opt-out, justified below
        let mut locations = HashMap::new();
        for (id, loc) in entries {
            locations.insert(id, loc);
        }
        Registry { next_id, locations }
    }
}

impl EntityLocation {
    /// Fold a location into a state hash deterministically (a discriminant then the
    /// fields), so a location participates in the canonical walk.
    pub fn hash_into(&self, h: &mut StateHasher) {
        match self {
            EntityLocation::Promoted(handle) => {
                h.write_u32(0);
                h.write_u64(handle.0);
            }
            EntityLocation::Pooled { pool, slot } => {
                h.write_u32(1);
                h.write_u32(pool.0);
                h.write_u32(*slot);
            }
            EntityLocation::Retired => {
                h.write_u32(2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::StateHasher;

    #[test]
    fn canonical_walk_is_insertion_order_independent() {
        // R-CANON-WALK: the registry hash must not depend on the order locations
        // were inserted, only on the sorted id order.
        let entries = [
            (StableId(2), EntityLocation::Promoted(EntityHandle(20))),
            (
                StableId(0),
                EntityLocation::Pooled {
                    pool: PoolId(1),
                    slot: 3,
                },
            ),
            (StableId(5), EntityLocation::Retired),
            (StableId(1), EntityLocation::Promoted(EntityHandle(7))),
        ];
        let mut a = Registry::new();
        for (id, loc) in entries {
            a.set_location(id, loc);
        }
        let mut b = Registry::new();
        for (id, loc) in entries.iter().rev() {
            b.set_location(*id, *loc);
        }
        // entries_sorted is identical regardless of insertion order.
        assert_eq!(a.entries_sorted(), b.entries_sorted());
        // and so is the canonical hash (next_id matches here, both 0).
        let mut ha = StateHasher::new();
        a.hash_into(&mut ha);
        let mut hb = StateHasher::new();
        b.hash_into(&mut hb);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn restore_carries_the_high_water_mark() {
        // R-SAVE-SCHEMA: a restored registry keeps the next-id high-water mark, so a
        // reload never reuses an id even when fewer ids are live than were minted.
        let mut reg = Registry::new();
        let _a = reg.mint();
        let b = reg.mint();
        let _c = reg.mint(); // next_id is now 3
        reg.set_location(b, EntityLocation::Retired);

        let restored = Registry::restore(reg.next_raw(), reg.entries_sorted());
        let next = {
            let mut r = restored;
            r.mint()
        };
        assert_eq!(next, StableId(3), "restore preserves the high-water mark");
    }

    #[test]
    fn ids_are_monotonic_and_unique() {
        let mut reg = Registry::new();
        let a = reg.mint();
        let b = reg.mint();
        let c = reg.mint();
        assert_eq!((a, b, c), (StableId(0), StableId(1), StableId(2)));
        assert!(a < b && b < c);
    }

    #[test]
    fn retired_ids_are_never_reused() {
        let mut reg = Registry::new();
        let a = reg.mint();
        reg.set_location(a, EntityLocation::Retired);
        let b = reg.mint();
        assert_ne!(a, b, "minting never reuses a retired id");
        // A retired id still resolves, so any reference to it stays valid.
        assert!(reg.resolves(a));
        assert_eq!(reg.locate(a), Some(EntityLocation::Retired));
    }

    #[test]
    fn location_transitions_track() {
        let mut reg = Registry::new();
        let id = reg.mint();
        reg.set_location(id, EntityLocation::Promoted(EntityHandle(99)));
        assert_eq!(
            reg.locate(id),
            Some(EntityLocation::Promoted(EntityHandle(99)))
        );
        reg.set_location(
            id,
            EntityLocation::Pooled {
                pool: PoolId(3),
                slot: 5,
            },
        );
        assert_eq!(
            reg.locate(id),
            Some(EntityLocation::Pooled {
                pool: PoolId(3),
                slot: 5
            })
        );
        assert!(reg.resolves(id));
        assert!(!reg.resolves(StableId(777)), "unknown id does not resolve");
    }
}
