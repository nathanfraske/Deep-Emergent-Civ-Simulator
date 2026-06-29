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

use civsim_core::{EntityHandle, EntityLocation, Fixed, PoolId, Registry, StableId};

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

    /// Merge pool `b` into pool `a`, removing `b`. Conserves population and wealth.
    pub fn merge_pools(&mut self, a: PoolId, b: PoolId) -> PoolId {
        let bi = self.pool_index(b);
        let moved = self.pools.remove(bi);
        let ai = self.pool_index(a);
        self.pools[ai].count += moved.count;
        self.pools[ai].wealth += moved.wealth;
        a
    }

    /// Split a pool, moving `take_count` members and `take_wealth` into a new pool.
    /// Conserves population and wealth.
    pub fn split_pool(&mut self, src: PoolId, take_count: u32, take_wealth: Fixed) -> PoolId {
        let si = self.pool_index(src);
        assert!(
            self.pools[si].count >= take_count,
            "splitting more than the pool holds"
        );
        self.pools[si].count -= take_count;
        self.pools[si].wealth -= take_wealth;
        self.add_pool(take_count, take_wealth)
    }

    /// Whether every edge endpoint still resolves through the registry. A dangling
    /// reference is the failure this guards against (design Part 58).
    pub fn referential_integrity_ok(&self) -> bool {
        self.edges
            .iter()
            .all(|(a, b)| self.reg.resolves(*a) && self.reg.resolves(*b))
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
