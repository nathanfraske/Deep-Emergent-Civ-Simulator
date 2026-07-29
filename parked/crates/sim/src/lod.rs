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

use crate::census::ReproductiveMoments;
use crate::demography::AgeHistogram;
use crate::institution::{AggregateInstitution, Institution};
use civsim_bio::belief::{BeliefKey, BeliefParams, BeliefPool, FacetStrength};
use civsim_core::{
    EntityHandle, EntityLocation, EventId, Fixed, InstId, PoolId, Registry, StableId, StateHasher,
};
use civsim_foundation::breeding::SexClass;
use civsim_foundation::decision::Curve;
use serde::{Deserialize, Serialize};

/// A promoted, fully represented entity holding some wealth.
///
/// Not `Copy`: a promoted mind carries the facet strengths it was lifted with (one per
/// prevailing belief), so demotion can fold exactly those bits back and total belief mass is
/// conserved across the crossing (design Part 54, R-PROJ-REGISTER).
#[derive(Debug, Clone, PartialEq)]
pub struct Individual {
    /// Stable identity, valid across promotion and demotion.
    pub id: StableId,
    /// This individual's share of wealth.
    pub wealth: Fixed,
    /// This individual's age in life-cadence steps, if it was promoted from an age-tracked
    /// pool. `None` for an age-untracked individual, so a world that does not model age is
    /// unchanged. Carried across the tier boundary so promotion and demotion conserve the
    /// pool's age distribution (design Parts 20, 54; R-AGING pool tier, R-PROJ-REGISTER).
    pub age: Option<u32>,
    /// The facet strengths this individual was lifted with, one per prevailing belief it holds,
    /// keyed by [`BeliefKey`]. Empty for an individual promoted from a belief-free pool, so
    /// existing two-tier worlds are unchanged. Restriction (demotion) folds exactly these bits
    /// back into the target pool, so the belief mass the lift removed returns exactly.
    pub beliefs: Vec<(BeliefKey, FacetStrength)>,
}

/// An aggregate pool of anonymous individuals, tracked as statistics. A pool may carry an
/// age distribution ([`AgeHistogram`]); when it does, `count` equals `ages.total()` and the
/// pool ages and suffers mortality statistically (the aggregate-tier demography of design
/// Parts 20, 25). An age-untracked pool leaves `ages` empty and behaves as a plain head
/// count, so existing two-tier worlds are unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct Pool {
    /// Pool identity.
    pub id: PoolId,
    /// How many anonymous individuals the pool represents. Equals `ages.total()` whenever the
    /// pool is age-tracked.
    pub count: u32,
    /// The pool's total wealth.
    pub wealth: Fixed,
    /// The pool's age distribution, empty for an age-untracked pool.
    pub ages: AgeHistogram,
    /// The pool's reproductive-moment accumulator for the current census window (design Parts 25,
    /// 54; the R-REPRO census tier). It carries the sex split of this window's breeders and the two
    /// reproductive moments, so the pool derives an effective population size Ne from
    /// [`Pool::effective_size`] without holding any individual. A window accumulator, not persisted
    /// canonical state: it is empty by default and rebuilt as births flow in through
    /// [`Pool::add_births`], so it stays out of the snapshot and the state hash exactly as the
    /// individual-tier census stays out of the world hash.
    pub repro: ReproductiveMoments,
    /// The pool's prevailing beliefs (design Part 9.14, Part 54): each an extensive belief mass
    /// and a member count, from which the intensive knowledge level derives. Empty for a
    /// belief-free pool, so existing two-tier worlds are unchanged. A lift moves belief mass from
    /// here into a promoted mind's facet strengths and a restriction folds it back, both exact,
    /// so total belief mass is a conserved projection across the tier boundary.
    pub beliefs: BeliefPool,
    /// The compact aggregate institution this pool crystallized, if any (design Part 36): a feature
    /// vector, a legitimacy mass, and a count rather than explicit roles and members. Promotion
    /// materializes an explicit [`Institution`] whose feature signature reproduces this vector, and
    /// the feature mass and legitimacy mass are conserved projections across the tier boundary
    /// exactly as belief mass is. `None` for a pool that has crystallized no institution, so
    /// existing two-tier worlds are unchanged.
    pub institution: Option<AggregateInstitution>,
}

impl Pool {
    /// Whether this pool tracks an age distribution (as opposed to a plain head count).
    pub fn is_age_tracked(&self) -> bool {
        self.ages.total() > 0
    }

    /// The pool-tier birth inflow: a breeder of sex `parent_sex` produced `offspring` young this
    /// window (design Parts 20, 25, 54). The newborns enter the age-zero cohort and the breeder's
    /// reproductive contribution feeds this pool's moment accumulator, so a coarse pool ages, grows,
    /// and derives Ne with no individuals. Delegates to [`AgeHistogram::add_births`].
    pub fn add_births(&mut self, parent_sex: SexClass, offspring: u32) {
        self.ages.add_births(&mut self.repro, parent_sex, offspring);
    }

    /// The effective population size the pool's census window implies, through the same race-blind
    /// kernel the individual tier uses ([`ReproductiveMoments::effective_size`]), so a pool-only
    /// world derives Ne consistently with a modelled one (record 62.9).
    pub fn effective_size(&self) -> u32 {
        self.repro.effective_size()
    }

    /// Whether this pool holds members but no age distribution (a plain head count). Distinct
    /// from an empty pool, which can still become age-tracked. Mixing such a pool with an
    /// age-tracked one would break the `count == ages.total()` invariant, so the operations
    /// that could mix them reject it.
    fn is_untracked_with_members(&self) -> bool {
        self.ages.total() == 0 && self.count > 0
    }
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
    /// The promoted, explicit institutions (design Part 36): a pool's crystallized
    /// [`AggregateInstitution`] materializes into one of these, and demotion folds it back. Feature
    /// mass and legitimacy mass are conserved across the crossing exactly as population and wealth
    /// are. Empty for a world with no promoted institutions, so existing two-tier worlds are
    /// unchanged.
    pub institutions: Vec<Institution>,
    next_pool: u32,
    next_handle: u64,
    next_inst: u32,
}

impl TwoTierWorld {
    /// An empty world.
    pub fn new() -> Self {
        TwoTierWorld {
            reg: Registry::new(),
            individuals: Vec::new(),
            pools: Vec::new(),
            edges: Vec::new(),
            institutions: Vec::new(),
            next_pool: 0,
            next_handle: 0,
            next_inst: 0,
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

    /// Total belief mass across both tiers (the conserved belief-mass projection): the summed
    /// extensive masses of every pool's prevailing beliefs, plus the raw bits of every promoted
    /// mind's carried facet strengths. Belief mass is a raw Q32.32 bit-sum, so addition is exact
    /// and associative and this total is conserved bit for bit across a lift and a restriction,
    /// exactly as `total_wealth` is (design Part 54, Part 58, R-PROJ-REGISTER).
    pub fn belief_mass(&self) -> i128 {
        let pool: i128 = self.pools.iter().map(|p| p.beliefs.total_mass()).sum();
        let ind: i128 = self
            .individuals
            .iter()
            .flat_map(|i| i.beliefs.iter().map(|(_, s)| s.to_bits() as i128))
            .sum();
        pool + ind
    }

    /// Total institution feature mass across both tiers (the conserved feature-vector projection of
    /// design Part 36 and R-TIER-CONSIST): the summed feature-signature bits of every pool's
    /// compact aggregate institution plus the extracted feature-signature bits of every promoted
    /// explicit institution. The signature is an extensive bit-sum, so this total is conserved bit
    /// for bit across promotion, demotion, merge, and split, exactly as `belief_mass` is.
    pub fn institution_feature_mass(&self) -> i128 {
        let pool: i128 = self
            .pools
            .iter()
            .filter_map(|p| p.institution.as_ref())
            .map(|a| a.feature_signature.mass_bits())
            .sum();
        let promoted: i128 = self
            .institutions
            .iter()
            .map(|i| i.feature_signature().mass_bits())
            .sum();
        pool + promoted
    }

    /// Total institution legitimacy mass across both tiers (the second conserved projection of the
    /// institution substrate): the summed legitimacy bits of every pool's aggregate institution
    /// plus every promoted institution's legitimacy. Conserved bit for bit across the tier
    /// crossings.
    pub fn institution_legitimacy_mass(&self) -> i128 {
        let pool: i128 = self
            .pools
            .iter()
            .filter_map(|p| p.institution.as_ref())
            .map(|a| a.legitimacy.to_bits() as i128)
            .sum();
        let promoted: i128 = self
            .institutions
            .iter()
            .map(|i| i.legitimacy.to_bits() as i128)
            .sum();
        pool + promoted
    }

    /// Attach a crystallized aggregate institution to a pool (the pool-tier crystallization result;
    /// the live detector that produces it from the running decision layer is a named follow-on). A
    /// pool that already carries one has the new aggregate merged into it, so feature and
    /// legitimacy mass are conserved.
    pub fn set_pool_institution(&mut self, pool: PoolId, agg: AggregateInstitution) {
        let pi = self.pool_index(pool);
        self.pools[pi].institution = Some(match self.pools[pi].institution.take() {
            Some(existing) => existing.merge(&agg),
            None => agg,
        });
    }

    /// Promote a pool's compact aggregate institution into an explicit [`Institution`] (design Part
    /// 36): the aggregate leaves the pool (`institution` becomes `None`) and an explicit institution
    /// whose feature signature reproduces the compact vector enters the promoted tier, minted with a
    /// fresh [`InstId`] and the given founding provenance. Conserves feature mass and legitimacy
    /// mass: the signature and legitimacy move across the tier boundary unchanged. Panics if the
    /// pool carries no institution.
    pub fn promote_institution(&mut self, pool: PoolId, founded: EventId) -> InstId {
        let pi = self.pool_index(pool);
        let agg = self.pools[pi]
            .institution
            .take()
            .expect("promoting an institution from a pool that has none");
        let id = InstId(self.next_inst);
        self.next_inst += 1;
        let inst = agg.materialize(id, founded);
        debug_assert_eq!(
            inst.feature_signature(),
            agg.feature_signature,
            "the materialized institution must reproduce the pool's compact vector"
        );
        self.institutions.push(inst);
        id
    }

    /// Demote an explicit institution back into a pool's compact aggregate form (design Part 36):
    /// the institution folds into the target pool's aggregate (merged if the pool already carries
    /// one), conserving feature mass and legitimacy mass. Panics if no institution with that id is
    /// promoted.
    pub fn demote_institution(&mut self, id: InstId, into: PoolId) {
        let idx = self
            .institutions
            .iter()
            .position(|i| i.id == id)
            .unwrap_or_else(|| panic!("no promoted institution {id:?}"));
        let inst = self.institutions.swap_remove(idx);
        let agg = AggregateInstitution::from_institution(&inst);
        let pi = self.pool_index(into);
        self.pools[pi].institution = Some(match self.pools[pi].institution.take() {
            Some(existing) => existing.merge(&agg),
            None => agg,
        });
    }

    /// Add an age-untracked aggregate pool (a plain head count), returning its id.
    pub fn add_pool(&mut self, count: u32, wealth: Fixed) -> PoolId {
        let id = PoolId(self.next_pool);
        self.next_pool += 1;
        self.pools.push(Pool {
            id,
            count,
            wealth,
            ages: AgeHistogram::new(),
            repro: ReproductiveMoments::new(),
            beliefs: BeliefPool::new(),
            institution: None,
        });
        id
    }

    /// Add an age-tracked aggregate pool from an age distribution, returning its id. The
    /// pool's head count is the distribution's total, so `count == ages.total()` holds by
    /// construction (design Parts 20, 25; the aggregate-tier demography). The head count is a
    /// `u32` in this demonstration model (the whole two-tier model is u32-scaled: `PoolId`,
    /// the pooled slot), so a distribution whose total exceeds `u32::MAX` fails loud here
    /// rather than truncating the count and desyncing it from the distribution. The
    /// `AgeHistogram` substrate itself is u64-capable; a production pool tier would carry a
    /// wider count.
    pub fn add_pool_aged(&mut self, ages: AgeHistogram, wealth: Fixed) -> PoolId {
        let id = PoolId(self.next_pool);
        self.next_pool += 1;
        let count = u32::try_from(ages.total())
            .expect("pool age total exceeds the u32 head-count ceiling of the two-tier model");
        self.pools.push(Pool {
            id,
            count,
            wealth,
            ages,
            repro: ReproductiveMoments::new(),
            beliefs: BeliefPool::new(),
            institution: None,
        });
        id
    }

    /// Advance the aggregate-tier demography one life cadence over every age-tracked pool,
    /// returning the total deaths (design Part 20, the R-AGING life-process loop at the pool
    /// tier). Each tracked pool suffers mortality at each cohort's current age against the
    /// supplied hazard curve, then the survivors age one step (the standard survive-then-age
    /// cohort life-table order). Population is conserved as a sink: the total before equals
    /// the total after plus the returned deaths. The hazard is authored biology entering as
    /// data (Principle 9); the demographic outcome emerges from it and the seed, keyed per
    /// pool so two pools roll independently. An age-untracked pool is left untouched.
    ///
    /// The caller supplies a `cadence` ordinal that must strictly increase across calls (the
    /// same contract [`AgeHistogram::apply_mortality`] documents): a bucket holds a rotated
    /// cohort each cadence, so a repeated cadence would re-roll the same stream. Mortality
    /// only removes members and aging conserves, so the recomputed head count never exceeds
    /// the count set at construction, and the `u32::try_from` below cannot fail once the pool
    /// was admitted by [`Self::add_pool_aged`].
    pub fn age_pools(&mut self, hazard: &Curve, seed: u64, cadence: u64) -> i128 {
        let mut deaths: i128 = 0;
        for pool in &mut self.pools {
            if pool.ages.total() == 0 {
                continue;
            }
            let died = pool
                .ages
                .apply_mortality(hazard, seed, pool.id.0 as u64, cadence);
            pool.ages.age_step();
            pool.count =
                u32::try_from(pool.ages.total()).expect("pool head count exceeds the u32 ceiling");
            deaths += died;
        }
        deaths
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

    /// Promote one member of an age-untracked pool into a full individual carrying `share`
    /// of the pool's wealth. Conserves population and wealth. An age-tracked pool must use
    /// [`Self::promote_at_age`] so the promoted member's age leaves the distribution.
    pub fn promote(&mut self, pool: PoolId, share: Fixed) -> StableId {
        let pi = self.pool_index(pool);
        assert!(
            !self.pools[pi].is_age_tracked(),
            "an age-tracked pool must promote with promote_at_age so the age leaves the distribution"
        );
        assert!(self.pools[pi].count >= 1, "promoting from an empty pool");
        assert!(share >= Fixed::ZERO, "a promoted share cannot be negative");
        assert!(
            self.pools[pi].wealth >= share,
            "share exceeds the pool's wealth; the partition would go negative"
        );
        self.pools[pi].count -= 1;
        self.pools[pi].wealth -= share;
        self.mint_promoted(share, None)
    }

    /// Promote one member of a given age out of an age-tracked pool, carrying `share` of the
    /// pool's wealth and that age into the individual tier. Conserves population, wealth, and
    /// the pool's age distribution: the member leaves its age bucket, so `count` stays equal
    /// to `ages.total()`, and the age travels with the individual so a later demotion returns
    /// it exactly (design Part 54, R-PROJ-REGISTER, the age projection across the tier
    /// boundary).
    pub fn promote_at_age(&mut self, pool: PoolId, share: Fixed, age: u32) -> StableId {
        let pi = self.pool_index(pool);
        assert!(
            self.pools[pi].ages.count_at(age) >= 1,
            "promoting an age the pool holds none of"
        );
        assert!(share >= Fixed::ZERO, "a promoted share cannot be negative");
        assert!(
            self.pools[pi].wealth >= share,
            "share exceeds the pool's wealth; the partition would go negative"
        );
        let removed = self.pools[pi].ages.remove(age, 1);
        debug_assert_eq!(removed, 1);
        self.pools[pi].count -= 1;
        self.pools[pi].wealth -= share;
        self.mint_promoted(share, Some(age))
    }

    /// Mint a promoted individual with the given wealth share and optional age, registering
    /// its location. The shared tail of [`Self::promote`] and [`Self::promote_at_age`].
    fn mint_promoted(&mut self, share: Fixed, age: Option<u32>) -> StableId {
        let id = self.reg.mint();
        let handle = EntityHandle(self.next_handle);
        self.next_handle += 1;
        self.reg.set_location(id, EntityLocation::Promoted(handle));
        self.individuals.push(Individual {
            id,
            wealth: share,
            age,
            beliefs: Vec::new(),
        });
        id
    }

    /// Promote one member of an age-untracked pool into a full individual, lifting the pool's
    /// prevailing beliefs into the new mind's facet strengths as it crosses the tier boundary
    /// (design Part 54, the belief lifting operator). It does the same wealth-and-population
    /// promote [`Self::promote`] does, then for each prevailing belief the pool holds it mints one
    /// facet strength at the belief's current level through the reserved curve and dispersion
    /// (`params`), subtracting exactly the minted bits from the pool and dropping that belief's
    /// count. The minted strengths ride with the individual, so total belief mass is unchanged by
    /// the crossing and a later demotion folds them back exactly.
    pub fn promote_lifting(
        &mut self,
        pool: PoolId,
        share: Fixed,
        params: &BeliefParams,
        tick: u64,
        seed: u64,
    ) -> StableId {
        let pi = self.pool_index(pool);
        assert!(
            !self.pools[pi].is_age_tracked(),
            "an age-tracked pool must promote with promote_at_age so the age leaves the distribution"
        );
        assert!(self.pools[pi].count >= 1, "promoting from an empty pool");
        assert!(share >= Fixed::ZERO, "a promoted share cannot be negative");
        assert!(
            self.pools[pi].wealth >= share,
            "share exceeds the pool's wealth; the partition would go negative"
        );
        self.pools[pi].count -= 1;
        self.pools[pi].wealth -= share;
        let id = self.mint_promoted(share, None);
        // Lift each prevailing belief the pool holds, in canonical key order, into the new mind.
        let keys = self.pools[pi].beliefs.keys_in_order();
        let mut carried: Vec<(BeliefKey, FacetStrength)> = Vec::new();
        for key in keys {
            let lifted = self.pools[pi].beliefs.get_mut(&key).and_then(|b| {
                b.lift_one(&params.level_to_strength, params.dispersion, id, tick, seed)
            });
            if let Some(strength) = lifted {
                carried.push((key, strength));
            }
        }
        // The individual minted last is the one to attach the lifted strengths to.
        if let Some(ind) = self.individuals.last_mut() {
            debug_assert_eq!(ind.id, id);
            ind.beliefs = carried;
        }
        id
    }

    /// Demote an individual back into a pool. Conserves population and wealth, and
    /// keeps the id resolvable (now `Pooled`), so edges referencing it stay valid. An
    /// individual carrying an age (one promoted from an age-tracked pool) returns that age to
    /// the target pool's distribution, so `count` stays equal to `ages.total()`.
    pub fn demote(&mut self, id: StableId, into: PoolId) {
        let idx = self
            .individuals
            .iter()
            .position(|i| i.id == id)
            .unwrap_or_else(|| panic!("no individual {id:?}"));
        let ind = self.individuals.swap_remove(idx);
        let pi = self.pool_index(into);
        // Keep the count-equals-distribution-total invariant: an aged individual may only
        // join an age-tracked (or empty) pool, and an unaged one only an untracked (or empty)
        // pool. Mixing would leave count and ages.total() out of step.
        match ind.age {
            Some(_) => assert!(
                !self.pools[pi].is_untracked_with_members(),
                "cannot demote an age-tracked individual into an untracked pool"
            ),
            None => assert!(
                self.pools[pi].ages.total() == 0,
                "cannot demote an unaged individual into an age-tracked pool"
            ),
        }
        self.pools[pi].count += 1;
        self.pools[pi].wealth += ind.wealth;
        if let Some(age) = ind.age {
            self.pools[pi].ages.add(age, 1);
        }
        // Restrict the mind's facet strengths back into the target pool's prevailing beliefs
        // (design Part 54, the belief restriction operator), folding exactly the bits the lift
        // removed so total belief mass returns. A key the pool did not yet carry is created.
        for (key, strength) in &ind.beliefs {
            self.pools[pi]
                .beliefs
                .entry_or_default(*key)
                .fold_one(*strength);
        }
        self.reg.set_location(
            id,
            EntityLocation::Pooled {
                pool: into,
                slot: self.pools[pi].count - 1,
            },
        );
    }

    /// Merge pool `b` into pool `a`, removing `b`. Conserves population and wealth, combines
    /// their age distributions age by age, and repoints any registry location that named `b`
    /// so a demoted entity's reference does not dangle to a removed pool (audit C-06).
    pub fn merge_pools(&mut self, a: PoolId, b: PoolId) -> PoolId {
        let bi = self.pool_index(b);
        let moved = self.pools.remove(bi);
        let ai = self.pool_index(a);
        // An age-tracked pool and an untracked one cannot merge without breaking the
        // count-equals-distribution-total invariant (an empty pool is compatible with either).
        let mixed = (self.pools[ai].is_age_tracked() && moved.is_untracked_with_members())
            || (moved.is_age_tracked() && self.pools[ai].is_untracked_with_members());
        assert!(
            !mixed,
            "cannot merge an age-tracked pool with an untracked one"
        );
        self.pools[ai].count = self.pools[ai]
            .count
            .checked_add(moved.count)
            .expect("merged pool head count exceeds the u32 ceiling");
        self.pools[ai].wealth += moved.wealth;
        self.pools[ai].ages.merge(&moved.ages);
        // Combine the two pools' aggregate institutions (design Part 36): their feature signatures
        // and legitimacy masses add, so total feature mass and legitimacy mass are conserved.
        self.pools[ai].institution = match (self.pools[ai].institution.take(), moved.institution) {
            (Some(x), Some(y)) => Some(x.merge(&y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };
        self.reg.repoint_pool(b, a);
        a
    }

    /// Split a pool, moving `take_count` members and `take_wealth` into a new pool.
    /// Conserves population and wealth, and rejects a share that would drive the
    /// source pool negative (audit C-07). Age-tracked splitting (partitioning the age
    /// distribution, which needs a rule for which ages move) is a follow-on, so this rejects
    /// an age-tracked source.
    pub fn split_pool(&mut self, src: PoolId, take_count: u32, take_wealth: Fixed) -> PoolId {
        let si = self.pool_index(src);
        assert!(
            !self.pools[si].is_age_tracked(),
            "splitting an age-tracked pool is not yet supported (the age partition rule is a follow-on)"
        );
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
        // Partition the pool's aggregate institution into two halves that recombine exactly (design
        // Part 36): the source keeps the first half and the new pool takes the second, so total
        // feature mass and legitimacy mass are conserved across the split.
        let split_inst = self.pools[si].institution.as_ref().map(|a| a.split_two());
        let new_pool = self.add_pool(take_count, take_wealth);
        if let Some((keep, give)) = split_inst {
            let si = self.pool_index(src);
            self.pools[si].institution = Some(keep);
            let ni = self.pool_index(new_pool);
            self.pools[ni].institution = Some(give);
        }
        new_pool
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
    /// it is bit-identical across runs and machines (design Part 3.5, R-CANON-WALK). Each
    /// variable-length collection is length-prefixed so its boundary is unambiguous and two
    /// different worlds cannot fold to the same bytes.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        let mut inds: Vec<&Individual> = self.individuals.iter().collect();
        inds.sort_by_key(|i| i.id);
        h.write_u64(inds.len() as u64);
        for i in inds {
            h.write_stable(i.id);
            h.write_fixed(i.wealth);
            // A presence flag then the value, so an absent age is distinct from a present
            // age zero without a wrapping collision at the ceiling.
            h.write_u32(i.age.is_some() as u32);
            h.write_u32(i.age.unwrap_or(0));
            // The mind's carried facet strengths, length-prefixed and walked in canonical
            // BeliefKey order (the vector is built in lift order, so it is sorted for the hash).
            let mut bel = i.beliefs.clone();
            bel.sort_by_key(|(k, _)| *k);
            h.write_u64(bel.len() as u64);
            for (k, s) in bel {
                k.hash_into(&mut h);
                h.write_fixed(s.get());
            }
        }
        let mut pools: Vec<&Pool> = self.pools.iter().collect();
        pools.sort_by_key(|p| p.id.0);
        h.write_u64(pools.len() as u64);
        for p in pools {
            h.write_u32(p.id.0);
            h.write_u32(p.count);
            h.write_fixed(p.wealth);
            h.write_u64(p.ages.occupied_ages() as u64);
            p.ages.hash_into(&mut h);
            // The pool's prevailing beliefs, length-prefixed and walked in canonical key order.
            p.beliefs.hash_into(&mut h);
            // The pool's compact aggregate institution, a presence flag then its canonical fold.
            h.write_u32(p.institution.is_some() as u32);
            if let Some(agg) = &p.institution {
                agg.hash_into(&mut h);
            }
        }
        let mut edges = self.edges.clone();
        edges.sort();
        h.write_u64(edges.len() as u64);
        for (a, b) in edges {
            h.write_stable(a);
            h.write_stable(b);
        }
        // The promoted explicit institutions, by ascending InstId. Institution::hash_into folds
        // every authoritative field EXCEPT the derived descriptor (Principle 10), so mutating a
        // descriptor never changes the state hash.
        let mut insts: Vec<&Institution> = self.institutions.iter().collect();
        insts.sort_by_key(|i| i.id.0);
        h.write_u64(insts.len() as u64);
        for i in insts {
            i.hash_into(&mut h);
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
                    age: i.age.map(|a| a as i64).unwrap_or(-1),
                })
                .collect(),
            pools: self
                .pools
                .iter()
                .map(|p| PoolRepr {
                    id: p.id.0,
                    count: p.count,
                    wealth_bits: p.wealth.to_bits(),
                    ages: p.ages.buckets().collect(),
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
                    age: if i.age < 0 { None } else { Some(i.age as u32) },
                    // The snapshot schema does not yet carry belief state (a follow-on to the
                    // rkyv/bincode persistence of R-SAVE-SCHEMA); a reloaded individual starts
                    // belief-free, exactly as the reproductive-moment window does.
                    beliefs: Vec::new(),
                })
                .collect(),
            pools: s
                .pools
                .iter()
                .map(|p| Pool {
                    id: PoolId(p.id),
                    count: p.count,
                    wealth: Fixed::from_bits(p.wealth_bits),
                    ages: AgeHistogram::from_pairs(p.ages.iter().copied()),
                    // The reproductive-moment accumulator is a window transient, not persisted; a
                    // reloaded pool starts a fresh window, exactly as the individual census does.
                    repro: ReproductiveMoments::new(),
                    // Belief state is not yet in the snapshot schema (R-SAVE-SCHEMA follow-on); a
                    // reloaded pool starts belief-free.
                    beliefs: BeliefPool::new(),
                    // Institution persistence is a named follow-on (R-SAVE-SCHEMA); a reloaded
                    // pool starts with no crystallized institution.
                    institution: None,
                })
                .collect(),
            edges: s
                .edges
                .iter()
                .map(|e| (StableId(e.a), StableId(e.b)))
                .collect(),
            // Institution persistence is a named follow-on (R-SAVE-SCHEMA); a reloaded world
            // starts with no promoted institutions, exactly as belief and reproductive-moment
            // state restart.
            institutions: Vec::new(),
            next_pool: s.next_pool,
            next_handle: s.next_handle,
            next_inst: 0,
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
    /// The current snapshot schema version. Version 2 adds per-individual age and per-pool
    /// age distributions (the aggregate-tier demography).
    pub const VERSION: u32 = 2;
}

/// A promoted individual in a snapshot. `age` is the individual's age in life-cadence steps,
/// or `-1` for an age-untracked individual (so the field is always present and TOML-friendly).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndRepr {
    pub id: u64,
    pub wealth_bits: i64,
    pub age: i64,
}

/// An aggregate pool in a snapshot. `ages` is the pool's age distribution as `(age, count)`
/// pairs in ascending-age order, empty for an age-untracked pool. A bucket count is a `u64`,
/// but a TOML integer is signed `i64`, so serializing errors loudly (it does not corrupt)
/// above `i64::MAX`; in a well-formed two-tier world a bucket count cannot exceed the pool's
/// `u32` head count, far below that ceiling, so the save is lossless in reach.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolRepr {
    pub id: u32,
    pub count: u32,
    pub wealth_bits: i64,
    pub ages: Vec<(u32, u64)>,
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

    fn flat_hazard(num: i64, den: i64) -> Curve {
        Curve::new([(Fixed::from_int(0), Fixed::from_ratio(num, den))])
    }

    #[test]
    fn aged_promote_then_demote_restores_the_distribution() {
        // The age crosses the tier boundary and comes back: promoting a 40-year-old removes
        // it from the pool's distribution, and demoting returns it, so the pool's age
        // structure and head count are exactly as they started. (The registry legitimately
        // retains the minted id as Pooled, so this checks the distribution, not the full
        // state hash, the same conservation the untracked promote-then-demote test checks.)
        let mut w = TwoTierWorld::new();
        let ages0 = AgeHistogram::from_pairs([(10, 4), (40, 2)]);
        let p = w.add_pool_aged(ages0.clone(), Fixed::from_int(60));
        let wealth0 = w.total_wealth();

        let x = w.promote_at_age(p, Fixed::from_int(5), 40);
        assert_eq!(
            w.pools[0].ages.count_at(40),
            1,
            "the promoted age left the pool"
        );
        assert_eq!(w.pools[0].count as i128, w.pools[0].ages.total());

        w.demote(x, p);
        assert_eq!(
            w.pools[0].ages, ages0,
            "demotion restores the exact age distribution"
        );
        assert_eq!(w.pools[0].count as i128, w.pools[0].ages.total());
        assert_eq!(
            w.total_wealth(),
            wealth0,
            "wealth is conserved across the round trip"
        );
    }

    #[test]
    fn age_pools_is_a_conserving_sink() {
        let mut w = TwoTierWorld::new();
        w.add_pool_aged(
            AgeHistogram::from_pairs([(30, 500), (60, 500)]),
            Fixed::ZERO,
        );
        let before = w.pools[0].ages.total();
        let deaths = w.age_pools(&flat_hazard(3, 10), 0x5EED, 0);
        assert!(deaths > 0);
        assert_eq!(
            w.pools[0].ages.total() + deaths,
            before,
            "survivors plus deaths equal the population before"
        );
        assert_eq!(
            w.pools[0].count as i128,
            w.pools[0].ages.total(),
            "the head count follows the distribution"
        );
    }

    #[test]
    fn snapshot_round_trips_the_age_distribution() {
        let mut w = TwoTierWorld::new();
        let p = w.add_pool_aged(
            AgeHistogram::from_pairs([(5, 3), (25, 7)]),
            Fixed::from_int(40),
        );
        let x = w.promote_at_age(p, Fixed::from_int(4), 25);
        w.add_edge(x, x);
        let before = w.state_hash();

        let snap = w.to_snapshot();
        assert_eq!(snap.schema_version, WorldSnapshot::VERSION);
        let text = toml::to_string(&snap).expect("aged snapshot serializes");
        let parsed: WorldSnapshot = toml::from_str(&text).expect("aged snapshot parses");
        let restored = TwoTierWorld::from_snapshot(&parsed);
        assert_eq!(
            restored.state_hash(),
            before,
            "the age distribution and the promoted age survive the round trip"
        );
        assert_eq!(restored.pools[0].ages.count_at(5), 3);
        assert_eq!(restored.individuals[0].age, Some(25));
    }

    #[test]
    #[should_panic]
    fn splitting_an_age_tracked_pool_is_rejected() {
        let mut w = TwoTierWorld::new();
        let p = w.add_pool_aged(AgeHistogram::from_pairs([(10, 5)]), Fixed::ZERO);
        w.split_pool(p, 2, Fixed::ZERO);
    }

    #[test]
    #[should_panic]
    fn plain_promote_on_an_age_tracked_pool_is_rejected() {
        let mut w = TwoTierWorld::new();
        let p = w.add_pool_aged(AgeHistogram::from_pairs([(10, 5)]), Fixed::ZERO);
        w.promote(p, Fixed::ZERO);
    }

    #[test]
    #[should_panic]
    fn demoting_an_aged_individual_into_an_untracked_pool_is_rejected() {
        // The invariant guard: an aged individual joining a plain head-count pool would leave
        // count and ages.total() out of step, so it is a loud error, not a silent corruption.
        let mut w = TwoTierWorld::new();
        let tracked = w.add_pool_aged(AgeHistogram::from_pairs([(10, 3)]), Fixed::ZERO);
        let untracked = w.add_pool(5, Fixed::ZERO);
        let x = w.promote_at_age(tracked, Fixed::ZERO, 10);
        w.demote(x, untracked);
    }

    #[test]
    #[should_panic]
    fn merging_a_tracked_and_an_untracked_pool_is_rejected() {
        let mut w = TwoTierWorld::new();
        let tracked = w.add_pool_aged(AgeHistogram::from_pairs([(10, 3)]), Fixed::ZERO);
        let untracked = w.add_pool(5, Fixed::ZERO);
        w.merge_pools(tracked, untracked);
    }

    #[test]
    #[should_panic(expected = "u32 head-count ceiling")]
    fn an_age_total_above_u32_fails_loud_rather_than_truncating() {
        // The red-team seam: a distribution whose total exceeds u32::MAX must not silently
        // truncate the head count and desync it from the distribution. It fails loud.
        let mut w = TwoTierWorld::new();
        w.add_pool_aged(
            AgeHistogram::from_pairs([(30, u32::MAX as u64 + 1)]),
            Fixed::ZERO,
        );
    }
}
