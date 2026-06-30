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

//! A minimal two-tier world: individuals and aggregate pools (design Parts 11, 54).
//!
//! This is the smallest model that exercises the conservation and referential
//! integrity invariants of Part 58. Promotion turns an anonymous pool member into a
//! full individual; demotion folds an individual back into a pool; pools merge and
//! split. Every operation moves population and wealth between tiers without creating
//! or destroying any, and every cross-tier reference stays valid because it uses
//! [`StableId`], which the registry keeps resolvable across the crossing.
//!
//! The behaviour here (who promotes when, how wealth is apportioned) is not the
//! point and is not calibrated; the point is that the structural invariants hold.

use civsim_core::{EntityHandle, EntityLocation, Fixed, PoolId, Registry, StableId, StateHasher};
use serde::{Deserialize, Serialize};

/// A promoted, fully represented entity holding some wealth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Individual {
    /// Stable identity, valid across promotion and demotion.
    pub id: StableId,
    /// This individual's share of wealth.
    pub wealth: Fixed,
}

/// An aggregate pool of anonymous individuals, tracked as statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pool {
    /// Pool identity.
    pub id: PoolId,
    /// How many anonymous individuals the pool represents.
    pub count: u32,
    /// The pool's total wealth.
    pub wealth: Fixed,
}

/// A world of two tiers, plus a set of relationship edges that must never dangle.
pub struct TwoTierWorld {
    /// The stable-id registry that keeps references resolvable across tier crossings.
    pub reg: Registry,
    /// The promoted individuals.
    pub individuals: Vec<Individual>,
    /// The aggregate pools.
    pub pools: Vec<Pool>,
    /// Relationship edges between individuals, by stable id. A consequential test of
    /// referential integrity: these must resolve after promotion, demotion, merge,
    /// and split.
    pub edges: Vec<(StableId, StableId)>,
    next_pool: u32,
    next_handle: u64,
}

impl TwoTierWorld {
    /// An empty world.
    pub fn new() -> Self {
        TwoTierWorld {
            reg: Registry::new(),
            individuals: Vec::new(),
            pools: Vec::new(),
            edges: Vec::new(),
            next_pool: 0,
            next_handle: 0,
        }
    }

    /// Total population across both tiers (the conserved population projection).
    pub fn population(&self) -> i128 {
        self.individuals.len() as i128 + self.pools.iter().map(|p| p.count as i128).sum::<i128>()
    }

    /// Total wealth across both tiers, as summed fixed-point bits (the conserved
    /// wealth projection). Fixed addition is exact, so this total is conserved
    /// exactly across structural change.
    pub fn total_wealth(&self) -> i128 {
        let ind: i128 = self
            .individuals
            .iter()
            .map(|i| i.wealth.to_bits() as i128)
            .sum();
        let pool: i128 = self.pools.iter().map(|p| p.wealth.to_bits() as i128).sum();
        ind + pool
    }

    /// Add an aggregate pool, returning its id.
    pub fn add_pool(&mut self, count: u32, wealth: Fixed) -> PoolId {
        let id = PoolId(self.next_pool);
        self.next_pool += 1;
        self.pools.push(Pool { id, count, wealth });
        id
    }

    fn pool_index(&self, id: PoolId) -> usize {
        self.pools
            .iter()
            .position(|p| p.id == id)
            .unwrap_or_else(|| panic!("no pool {id:?}"))
    }

    /// Add a relationship edge between two ids.
    pub fn add_edge(&mut self, a: StableId, b: StableId) {
        self.edges.push((a, b));
    }

    /// Promote one member of a pool into a full individual carrying `share` of the
    /// pool's wealth. Conserves population and wealth.
    pub fn promote(&mut self, pool: PoolId, share: Fixed) -> StableId {
        let pi = self.pool_index(pool);
        assert!(self.pools[pi].count >= 1, "promoting from an empty pool");
        assert!(share >= Fixed::ZERO, "a promoted share cannot be negative");
        assert!(
            self.pools[pi].wealth >= share,
            "share exceeds the pool's wealth; the partition would go negative"
        );
        self.pools[pi].count -= 1;
        self.pools[pi].wealth -= share;

        let id = self.reg.mint();
        let handle = EntityHandle(self.next_handle);
        self.next_handle += 1;
        self.reg.set_location(id, EntityLocation::Promoted(handle));
        self.individuals.push(Individual { id, wealth: share });
        id
    }

    /// Demote an individual back into a pool. Conserves population and wealth, and
    /// keeps the id resolvable (now `Pooled`), so edges referencing it stay valid.
    pub fn demote(&mut self, id: StableId, into: PoolId) {
        let idx = self
            .individuals
            .iter()
            .position(|i| i.id == id)
            .unwrap_or_else(|| panic!("no individual {id:?}"));
        let ind = self.individuals.swap_remove(idx);
        let pi = self.pool_index(into);
        self.pools[pi].count += 1;
        self.pools[pi].wealth += ind.wealth;
        self.reg.set_location(
            id,
            EntityLocation::Pooled {
                pool: into,
                slot: self.pools[pi].count - 1,
            },
        );
    }

    /// Merge pool `b` into pool `a`, removing `b`. Conserves population and wealth,
    /// and repoints any registry location that named `b` so a demoted entity's
    /// reference does not dangle to a removed pool (audit C-06).
    pub fn merge_pools(&mut self, a: PoolId, b: PoolId) -> PoolId {
        let bi = self.pool_index(b);
        let moved = self.pools.remove(bi);
        let ai = self.pool_index(a);
        self.pools[ai].count += moved.count;
        self.pools[ai].wealth += moved.wealth;
        self.reg.repoint_pool(b, a);
        a
    }

    /// Split a pool, moving `take_count` members and `take_wealth` into a new pool.
    /// Conserves population and wealth, and rejects a share that would drive the
    /// source pool negative (audit C-07).
    pub fn split_pool(&mut self, src: PoolId, take_count: u32, take_wealth: Fixed) -> PoolId {
        let si = self.pool_index(src);
        assert!(
            self.pools[si].count >= take_count,
            "splitting more than the pool holds"
        );
        assert!(
            take_wealth >= Fixed::ZERO,
            "a split share cannot be negative"
        );
        assert!(
            self.pools[si].wealth >= take_wealth,
            "split wealth exceeds the pool's wealth"
        );
        self.pools[si].count -= take_count;
        self.pools[si].wealth -= take_wealth;
        self.add_pool(take_count, take_wealth)
    }

    /// Partition a total into `n` exact integer shares (in fixed-point bits) with the
    /// remainder assigned to the lowest-id share, the settled
    /// `tier.partition_remainder_rule` (design Part 54). The shares sum to the total
    /// exactly, so a partition conserves wealth bit for bit (audit C-07).
    pub fn partition_lowest_id(total: Fixed, n: u32) -> Vec<Fixed> {
        assert!(n >= 1, "cannot partition into zero shares");
        let bits = total.to_bits();
        let share = bits / n as i64;
        let remainder = bits - share * n as i64;
        (0..n)
            .map(|i| {
                let extra = if i == 0 { remainder } else { 0 };
                Fixed::from_bits(share + extra)
            })
            .collect()
    }

    /// Whether the location named by a reference is still live: the id is tracked,
    /// and a `Promoted` location has a matching individual while a `Pooled` location
    /// names a pool that still exists. This is the strong form the weak
    /// [`Registry::resolves`] does not provide (audit C-06).
    fn location_valid(&self, id: StableId) -> bool {
        match self.reg.locate(id) {
            None => false,
            Some(EntityLocation::Promoted(_)) => self.individuals.iter().any(|i| i.id == id),
            Some(EntityLocation::Pooled { pool, .. }) => self.pools.iter().any(|p| p.id == pool),
            Some(EntityLocation::Retired) => true,
        }
    }

    /// Whether every edge endpoint still names a live location. A dangling reference,
    /// including one to a pool removed by a merge, is the failure this guards against
    /// (design Part 58).
    pub fn referential_integrity_ok(&self) -> bool {
        self.edges
            .iter()
            .all(|(a, b)| self.location_valid(*a) && self.location_valid(*b))
    }

    /// A deterministic hash of the whole two-tier world, walked in canonical order
    /// (individuals and pools by ascending id, edges sorted, then the registry in
    /// id order). Built only through the sorted accessors, never hash-map order, so
    /// it is bit-identical across runs and machines (design Part 3.5, R-CANON-WALK).
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        let mut inds: Vec<&Individual> = self.individuals.iter().collect();
        inds.sort_by_key(|i| i.id);
        for i in inds {
            h.write_stable(i.id);
            h.write_fixed(i.wealth);
        }
        let mut pools: Vec<&Pool> = self.pools.iter().collect();
        pools.sort_by_key(|p| p.id.0);
        for p in pools {
            h.write_u32(p.id.0);
            h.write_u32(p.count);
            h.write_fixed(p.wealth);
        }
        let mut edges = self.edges.clone();
        edges.sort();
        for (a, b) in edges {
            h.write_stable(a);
            h.write_stable(b);
        }
        self.reg.hash_into(&mut h);
        h.finish()
    }

    /// Capture the full canonical state as a serializable snapshot, including the id
    /// high-water marks, so a reload reproduces the world exactly and never reuses an
    /// id (R-SAVE-SCHEMA groundwork). Wealth is stored as fixed-point bits so no
    /// float enters the saved state.
    pub fn to_snapshot(&self) -> WorldSnapshot {
        WorldSnapshot {
            schema_version: WorldSnapshot::VERSION,
            next_id: self.reg.next_raw(),
            next_pool: self.next_pool,
            next_handle: self.next_handle,
            individuals: self
                .individuals
                .iter()
                .map(|i| IndRepr {
                    id: i.id.0,
                    wealth_bits: i.wealth.to_bits(),
                })
                .collect(),
            pools: self
                .pools
                .iter()
                .map(|p| PoolRepr {
                    id: p.id.0,
                    count: p.count,
                    wealth_bits: p.wealth.to_bits(),
                })
                .collect(),
            edges: self
                .edges
                .iter()
                .map(|(a, b)| EdgeRepr { a: a.0, b: b.0 })
                .collect(),
            locations: self
                .reg
                .entries_sorted()
                .into_iter()
                .map(LocRepr::from_location)
                .collect(),
        }
    }

    /// Rebuild a world from a snapshot. Restores the id high-water marks
    /// authoritatively from the snapshot rather than inferring them from the live
    /// entries.
    pub fn from_snapshot(s: &WorldSnapshot) -> Self {
        let reg = Registry::restore(s.next_id, s.locations.iter().map(LocRepr::to_location));
        TwoTierWorld {
            reg,
            individuals: s
                .individuals
                .iter()
                .map(|i| Individual {
                    id: StableId(i.id),
                    wealth: Fixed::from_bits(i.wealth_bits),
                })
                .collect(),
            pools: s
                .pools
                .iter()
                .map(|p| Pool {
                    id: PoolId(p.id),
                    count: p.count,
                    wealth: Fixed::from_bits(p.wealth_bits),
                })
                .collect(),
            edges: s
                .edges
                .iter()
                .map(|e| (StableId(e.a), StableId(e.b)))
                .collect(),
            next_pool: s.next_pool,
            next_handle: s.next_handle,
        }
    }
}

/// A serializable snapshot of a two-tier world. Carries an explicit schema version
/// and the id high-water marks (R-SAVE-SCHEMA). The full rkyv and bincode wiring of
/// design Part 7.3 is the remaining work; this captures the canonical state losslessly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub schema_version: u32,
    pub next_id: u64,
    pub next_pool: u32,
    pub next_handle: u64,
    pub individuals: Vec<IndRepr>,
    pub pools: Vec<PoolRepr>,
    pub edges: Vec<EdgeRepr>,
    pub locations: Vec<LocRepr>,
}

impl WorldSnapshot {
    /// The current snapshot schema version.
    pub const VERSION: u32 = 1;
}

/// A promoted individual in a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndRepr {
    pub id: u64,
    pub wealth_bits: i64,
}

/// An aggregate pool in a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolRepr {
    pub id: u32,
    pub count: u32,
    pub wealth_bits: i64,
}

/// A relationship edge in a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRepr {
    pub a: u64,
    pub b: u64,
}

/// A registry location entry in a snapshot. `kind` is 0 promoted, 1 pooled, 2 retired.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocRepr {
    pub id: u64,
    pub kind: u8,
    pub handle: u64,
    pub pool: u32,
    pub slot: u32,
}

impl LocRepr {
    fn from_location(entry: (StableId, EntityLocation)) -> Self {
        let (id, loc) = entry;
        match loc {
            EntityLocation::Promoted(h) => LocRepr {
                id: id.0,
                kind: 0,
                handle: h.0,
                pool: 0,
                slot: 0,
            },
            EntityLocation::Pooled { pool, slot } => LocRepr {
                id: id.0,
                kind: 1,
                handle: 0,
                pool: pool.0,
                slot,
            },
            EntityLocation::Retired => LocRepr {
                id: id.0,
                kind: 2,
                handle: 0,
                pool: 0,
                slot: 0,
            },
        }
    }

    fn to_location(&self) -> (StableId, EntityLocation) {
        let loc = match self.kind {
            0 => EntityLocation::Promoted(EntityHandle(self.handle)),
            1 => EntityLocation::Pooled {
                pool: PoolId(self.pool),
                slot: self.slot,
            },
            _ => EntityLocation::Retired,
        };
        (StableId(self.id), loc)
    }
}

impl Default for TwoTierWorld {
    fn default() -> Self {
        TwoTierWorld::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_toml_round_trip_preserves_state() {
        // R-SAVE-SCHEMA: the canonical state survives a real serialize and parse
        // through a format, and the rebuilt world hashes identically.
        let mut w = TwoTierWorld::new();
        let pa = w.add_pool(8, Fixed::from_int(80));
        let pb = w.add_pool(4, Fixed::from_int(40));
        let x = w.promote(pa, Fixed::from_int(5));
        let y = w.promote(pb, Fixed::from_int(3));
        w.add_edge(x, y);
        w.demote(x, pb);
        let before = w.state_hash();

        let snap = w.to_snapshot();
        let text = toml::to_string(&snap).expect("snapshot serializes");
        let parsed: WorldSnapshot = toml::from_str(&text).expect("snapshot parses");
        assert_eq!(parsed, snap, "the snapshot survives the format round trip");
        let restored = TwoTierWorld::from_snapshot(&parsed);
        assert_eq!(
            restored.state_hash(),
            before,
            "restored world hashes identically"
        );
        assert!(restored.referential_integrity_ok());
        assert_eq!(
            restored.reg.next_raw(),
            w.reg.next_raw(),
            "high-water mark restored"
        );
    }

    #[test]
    #[should_panic]
    fn promote_beyond_pool_wealth_is_rejected() {
        // Regression for the determinism audit C-07: a share larger than the pool
        // holds would drive pool wealth negative and must be rejected.
        let mut w = TwoTierWorld::new();
        let p = w.add_pool(1, Fixed::from_int(1));
        w.promote(p, Fixed::from_int(10_000));
    }

    #[test]
    fn dangling_pool_location_is_detected() {
        // Regression for the determinism audit C-06: a reference whose location
        // names a pool that no longer exists must fail the integrity check, not pass
        // merely because the id is still a key in the registry.
        let mut w = TwoTierWorld::new();
        let p = w.add_pool(1, Fixed::ZERO);
        let x = w.promote(p, Fixed::ZERO);
        w.add_edge(x, x);
        w.reg.set_location(
            x,
            EntityLocation::Pooled {
                pool: PoolId(999),
                slot: 0,
            },
        );
        assert!(
            !w.referential_integrity_ok(),
            "a Pooled reference to a non-existent pool must be caught"
        );
    }

    #[test]
    fn merge_repoints_pooled_locations_and_keeps_integrity() {
        // C-06 green: after a merge, an edge to an entity demoted into the removed
        // pool stays valid because its location is repointed to the survivor.
        let mut w = TwoTierWorld::new();
        let a = w.add_pool(1, Fixed::from_int(10));
        let b = w.add_pool(1, Fixed::from_int(10));
        let x = w.promote(b, Fixed::from_int(5));
        w.demote(x, b);
        let other = w.add_pool(1, Fixed::from_int(10));
        let y = w.promote(other, Fixed::ZERO);
        w.add_edge(y, x);

        w.merge_pools(a, b);
        assert!(
            matches!(w.reg.locate(x), Some(EntityLocation::Pooled { pool, .. }) if pool == a),
            "the demoted entity's location follows the merge"
        );
        assert!(
            w.referential_integrity_ok(),
            "no reference dangles after the merge"
        );
    }

    #[test]
    fn partition_lowest_id_is_exact_and_conserves() {
        // C-07 green: the remainder goes to the lowest id and the shares sum exactly.
        let shares = TwoTierWorld::partition_lowest_id(Fixed::from_int(10), 3);
        assert_eq!(shares.len(), 3);
        let total_bits: i128 = Fixed::sum_bits(shares.iter().copied());
        assert_eq!(
            total_bits,
            Fixed::from_int(10).to_bits() as i128,
            "shares sum to the total"
        );
        assert!(
            shares[0] >= shares[1] && shares[1] == shares[2],
            "remainder lands on the lowest id"
        );
    }

    #[test]
    fn promote_then_demote_conserves_locally() {
        let mut w = TwoTierWorld::new();
        let p = w.add_pool(10, Fixed::from_int(100));
        let pop0 = w.population();
        let wealth0 = w.total_wealth();

        let id = w.promote(p, Fixed::from_int(10));
        assert_eq!(w.population(), pop0, "promotion conserves population");
        assert_eq!(w.total_wealth(), wealth0, "promotion conserves wealth");

        w.demote(id, p);
        assert_eq!(w.population(), pop0);
        assert_eq!(w.total_wealth(), wealth0);
        assert!(w.reg.resolves(id), "a demoted id still resolves");
    }

    #[test]
    fn merge_and_split_conserve() {
        let mut w = TwoTierWorld::new();
        let a = w.add_pool(5, Fixed::from_int(50));
        let b = w.add_pool(7, Fixed::from_int(70));
        let pop0 = w.population();
        let wealth0 = w.total_wealth();

        let merged = w.merge_pools(a, b);
        assert_eq!(w.population(), pop0);
        assert_eq!(w.total_wealth(), wealth0);
        assert_eq!(w.pools.len(), 1);

        w.split_pool(merged, 4, Fixed::from_int(40));
        assert_eq!(w.population(), pop0);
        assert_eq!(w.total_wealth(), wealth0);
        assert_eq!(w.pools.len(), 2);
    }
}
