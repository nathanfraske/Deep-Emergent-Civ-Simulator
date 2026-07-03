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

//! The aggregate-tier age distribution and mortality (design Parts 20, 25, 54; the
//! pool-tier half of the R-AGING lifespan keystone).
//!
//! The individual tier already carries a life-process loop: a being's age advances one
//! step per life cadence ([`crate::world::World::age_step`]) and a mortality pass rolls
//! each being against an age-hazard curve ([`crate::world::World::apply_mortality`]).
//! The aggregate (pool) tier could not carry the same demography: a [`crate::lod::Pool`]
//! tracks only a head count, so a coarse population has no age structure and cannot age
//! or die statistically. This module is the missing aggregate-tier mechanism, an age
//! histogram that ages and suffers mortality the same way the individual tier does, so a
//! quiet region advancing in cheap coarse steps still has a survivorship curve rather
//! than a static count.
//!
//! The guiding star (Principle 9, the Steering Audit): physics and biology enter as data,
//! emergent demography comes out, and the mechanism authors no outcome. The age-hazard
//! curve is the authored biology input, exactly as a material yield stress or a gene
//! effect size is authored; it is supplied as a [`Curve`], never built into this code. The
//! demographic outcomes (the age structure, the survivorship curve, which cohorts thin and
//! which persist) are never authored: they emerge from the supplied hazard and the seed
//! alone. A flat zero hazard kills nobody, so the mechanism adds no mortality of its own,
//! and swapping two populations' hazard curves swaps their expected demography, so the
//! hazard is the sole author. The tests below are the audit of that property.
//!
//! Tier consistency (design Part 54, record 62.9) is conservation plus distributional
//! agreement, never identical member-by-member outcomes, which is mathematically
//! unattainable for a statistical and a per-agent model. So mortality here is the exact
//! per-member Bernoulli sum the aggregate genome tier already uses
//! ([`crate::genome::GenePool::drift`]): each of a bucket's members is rolled against the
//! bucket's age hazard, keyed by counter-RNG, so the count is conserved exactly and the
//! pool death fraction agrees in expectation with the individual tier's. The exact sum is
//! O(count) per bucket, the same cost the drift sampler carries; the O(1) binomial (tau-
//! leaping) fast path for very large pools is a reserved owner decision (25.10, the same
//! large-population approximation the drift sampler reserves, and the binomial coarse
//! advance the temporal-LOD work reserves), surfaced rather than fabricated, so only the
//! exact reference sampler is built here.

use crate::breeding::SexClass;
use crate::census::ReproductiveMoments;
use crate::decision::Curve;
use civsim_core::{DrawKey, Fixed, Phase, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Convert an age in life-cadence steps to the [`Fixed`] value a hazard curve is evaluated
/// at, clamped to the representable positive range. An age beyond `i32::MAX` steps (roughly
/// two billion cadences, far past any lifespan) reads the oldest curve point through the
/// curve's monotone end clamp, rather than wrapping negative through a raw `age as i32` cast
/// and reading the youngest, least-lethal point. A cast must never author the demographic
/// outcome, only the hazard may (Principle 9). Both the aggregate tier here and the
/// individual tier ([`crate::world::World::apply_mortality`]) evaluate the hazard through
/// this one conversion, so the two never disagree at the age ceiling.
pub fn hazard_age(age: u32) -> Fixed {
    Fixed::from_int(age.min(i32::MAX as u32) as i32)
}

/// An aggregate-tier age distribution: a count of anonymous members per age, where age is
/// measured in life-cadence steps exactly as the individual tier measures it. The map is
/// keyed by age so every walk is canonical (ascending age, never hash-map order, design
/// Part 3.5, R-CANON-WALK), and a zero-count age is never stored, so the histogram has one
/// representation for one distribution and hashes identically however it was assembled.
///
/// A member count is a `u64` per age. The conservation guarantees below are exact for any
/// total within `u64` per bucket, which is beyond reach at realistic populations (order 1e11
/// against a ceiling near 1.8e19); a bucket driven past that ceiling saturates rather than
/// wrapping, so the failure mode at that unreachable scale is a capped total, never a
/// wrapped one.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct AgeHistogram {
    /// Age in life-cadence steps to the count of members at that age. Never holds a zero.
    counts: BTreeMap<u32, u64>,
}

impl AgeHistogram {
    /// An empty distribution.
    pub fn new() -> Self {
        AgeHistogram {
            counts: BTreeMap::new(),
        }
    }

    /// A distribution from `(age, count)` pairs. Counts at the same age accumulate, and a
    /// zero count is dropped, so any input yields the canonical representation.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (u32, u64)>) -> Self {
        let mut h = AgeHistogram::new();
        for (age, n) in pairs {
            h.add(age, n);
        }
        h
    }

    /// Add `n` members at `age` (saturating, so a count cannot wrap). Adding zero is a no-op
    /// and never creates an empty bucket.
    pub fn add(&mut self, age: u32, n: u64) {
        if n == 0 {
            return;
        }
        let slot = self.counts.entry(age).or_insert(0);
        *slot = slot.saturating_add(n);
    }

    /// Remove up to `n` members at `age`, returning how many were removed (fewer than `n`
    /// if the bucket held fewer). A bucket emptied to zero is dropped so the
    /// representation stays canonical. This is the per-age primitive a promotion (a member
    /// leaving the pool for the individual tier) and a split use, each conserving because
    /// the removed members are accounted for by the caller.
    pub fn remove(&mut self, age: u32, n: u64) -> u64 {
        let held = match self.counts.get(&age) {
            Some(&c) => c,
            None => return 0,
        };
        let taken = held.min(n);
        let left = held - taken;
        if left == 0 {
            self.counts.remove(&age);
        } else {
            self.counts.insert(age, left);
        }
        taken
    }

    /// The count of members at `age`.
    pub fn count_at(&self, age: u32) -> u64 {
        self.counts.get(&age).copied().unwrap_or(0)
    }

    /// The total member count across all ages: the conserved population projection (design
    /// Part 58, R-PROJ-REGISTER). Returned as `i128` so it composes directly with the
    /// [`crate::conservation::ConservationRegistry`], where addition is exact and
    /// associative, so an age distribution's total is conserved bit for bit across every
    /// structural change (aging, mortality, merge, promotion).
    pub fn total(&self) -> i128 {
        self.counts.values().map(|&c| c as i128).sum()
    }

    /// The number of distinct populated ages (buckets). Diagnostic, not a conserved
    /// quantity.
    pub fn occupied_ages(&self) -> usize {
        self.counts.len()
    }

    /// Iterate the populated `(age, count)` buckets in ascending age order (canonical).
    pub fn buckets(&self) -> impl Iterator<Item = (u32, u64)> + '_ {
        self.counts.iter().map(|(&age, &c)| (age, c))
    }

    /// Advance every cohort by one life cadence: each member ages one step (design Part 20).
    /// Aging is saturating at the numeric ceiling, exactly as the individual tier's
    /// [`crate::world::World::age_step`] saturates, so a long-lived cohort's age never
    /// wraps and two cohorts that both reach the ceiling merge into it rather than being
    /// lost. Conserves the total exactly: no member is created or destroyed by the passage
    /// of time, only relabelled to an older age. Births and promotions are separate inflows
    /// the caller applies; aging alone leaves age zero empty.
    pub fn age_step(&mut self) {
        let mut aged: BTreeMap<u32, u64> = BTreeMap::new();
        for (&age, &n) in &self.counts {
            let older = age.saturating_add(1);
            let slot = aged.entry(older).or_insert(0);
            *slot = slot.saturating_add(n);
        }
        self.counts = aged;
    }

    /// Run one mortality pass against an age-hazard curve, returning the number of deaths
    /// (design Part 20, the R-AGING life-process loop, at the aggregate tier). For each
    /// populated age in ascending order, the curve maps the age to a per-cadence death
    /// probability (a rising-hazard curve is the data-driven default, owner supplied as
    /// `hazard`, never built in here), and each of that bucket's members is rolled against
    /// it by counter-RNG, keyed on the pool, the age, and the cadence ordinal under
    /// [`Phase::MORTALITY`]. The survivors stay, the dead are subtracted, and the count is
    /// conserved exactly (the returned deaths plus the surviving total equal the total
    /// before).
    ///
    /// Deterministic and observer-independent: the roll is a pure function of the seed, the
    /// pool's id, the age, the cadence, and the member's slot, so a pool faces the same
    /// mortality on replay and the pass is independent of thread count. The cadence ordinal
    /// is in the key because a bucket holds a different cohort each cadence (aging rotates
    /// members through it), so without it a persistent bucket would suffer identical deaths
    /// every cadence. The individual tier keys instead on the being's own id and age, since
    /// a named being reaches each age once; the two tiers therefore agree in distribution,
    /// not member for member, which is the exact tier-consistency guarantee of record 62.9.
    pub fn apply_mortality(
        &mut self,
        hazard: &Curve,
        seed: u64,
        pool_id: u64,
        cadence: u64,
    ) -> i128 {
        let mut deaths: i128 = 0;
        let mut survivors: BTreeMap<u32, u64> = BTreeMap::new();
        for (&age, &n) in &self.counts {
            let chance = hazard.eval(hazard_age(age)).clamp(Fixed::ZERO, Fixed::ONE);
            let rng = DrawKey::pair(pool_id, age as u64, cadence, Phase::MORTALITY).rng(seed);
            let mut died: u64 = 0;
            for k in 0..n {
                if rng.unit_fixed(k) < chance {
                    died += 1;
                }
            }
            deaths += died as i128;
            let left = n - died;
            if left > 0 {
                survivors.insert(age, left);
            }
        }
        self.counts = survivors;
        deaths
    }

    /// The pool-tier birth inflow (design Parts 20, 25, 54; the R-REPRO census tier). A breeder of
    /// sex `parent_sex` produced `offspring` young this window: the newborns enter the age-zero
    /// cohort of this distribution (the inflow the aging loop leaves empty), and the breeder's
    /// contribution is recorded into the reproductive-moment accumulator `moments` (its sex and its
    /// offspring count), so the pool tier derives an effective population size Ne without ever
    /// holding an individual ([`ReproductiveMoments::effective_size`]). This is the aggregate
    /// counterpart of the individual tier's [`crate::census::ReproductiveCensus::record_birth`]; fed
    /// the same events the two tiers reduce to the same moments and so to the same Ne (record 62.9).
    /// Adding zero offspring still records the breeder (a parent that reproduced this window),
    /// keeping the sex split and the breeder count consistent with the individual tier. Conserves as
    /// an inflow: the age-zero cohort rises by exactly `offspring`.
    pub fn add_births(
        &mut self,
        moments: &mut ReproductiveMoments,
        parent_sex: SexClass,
        offspring: u32,
    ) {
        self.add(0, offspring as u64);
        moments.record_parent(parent_sex, offspring);
    }

    /// Merge another distribution into this one, adding member counts age by age. Conserves:
    /// the merged total equals the sum of the two totals (the pool-merge primitive of design
    /// Part 11, at the age-structure level). Saturating per bucket, matching [`Self::add`].
    pub fn merge(&mut self, other: &AgeHistogram) {
        for (&age, &n) in &other.counts {
            self.add(age, n);
        }
    }

    /// Fold the distribution into a hash in canonical (ascending-age) order, writing each
    /// populated bucket's age then count. Because the store is keyed by age and holds no
    /// zero, the fold is a pure function of the distribution and independent of how it was
    /// assembled (design Part 3.5, R-CANON-WALK).
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (&age, &n) in &self.counts {
            h.write_u32(age);
            h.write_u64(n);
        }
    }

    /// A standalone canonical hash of the distribution.
    pub fn state_hash(&self) -> u128 {
        let mut h = StateHasher::new();
        self.hash_into(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conservation::ConservationRegistry;

    /// A rising age-hazard curve as data: death probability climbs with age, the
    /// data-driven default the individual tier's tests also use. The shape is a fixture,
    /// never a fabricated calibration; the real curve is a reserved owner value.
    fn rising_hazard() -> Curve {
        Curve::new([
            (Fixed::from_int(0), Fixed::from_ratio(1, 100)),
            (Fixed::from_int(50), Fixed::from_ratio(1, 20)),
            (Fixed::from_int(80), Fixed::from_ratio(1, 2)),
            (Fixed::from_int(120), Fixed::ONE),
        ])
    }

    /// A flat hazard at a constant probability, for the distributional-agreement check.
    fn flat_hazard(num: i64, den: i64) -> Curve {
        Curve::new([(Fixed::from_int(0), Fixed::from_ratio(num, den))])
    }

    #[test]
    fn total_is_the_conserved_projection() {
        let h = AgeHistogram::from_pairs([(10, 5), (20, 3), (10, 2)]);
        assert_eq!(h.count_at(10), 7, "same-age counts accumulate");
        assert_eq!(h.count_at(20), 3);
        assert_eq!(h.total(), 10);
        assert_eq!(h.occupied_ages(), 2);
    }

    #[test]
    fn zero_counts_are_never_stored() {
        let mut h = AgeHistogram::new();
        h.add(5, 0);
        assert_eq!(h.occupied_ages(), 0, "adding zero creates no bucket");
        h.add(5, 4);
        let removed = h.remove(5, 4);
        assert_eq!(removed, 4);
        assert_eq!(h.occupied_ages(), 0, "an emptied bucket is dropped");
        assert_eq!(h.total(), 0);
    }

    #[test]
    fn age_step_conserves_total_and_shifts_ages() {
        let mut h = AgeHistogram::from_pairs([(0, 10), (5, 4)]);
        let before = h.total();
        h.age_step();
        assert_eq!(h.total(), before, "aging conserves the total");
        assert_eq!(h.count_at(0), 0, "aging empties age zero");
        assert_eq!(h.count_at(1), 10, "the age-zero cohort is now age one");
        assert_eq!(h.count_at(6), 4);
    }

    #[test]
    fn age_step_saturates_without_wrap_or_loss() {
        let mut h = AgeHistogram::from_pairs([(u32::MAX, 3), (u32::MAX - 1, 2)]);
        let before = h.total();
        h.age_step();
        assert_eq!(h.total(), before, "aging at the ceiling loses no one");
        assert_eq!(
            h.count_at(u32::MAX),
            5,
            "the ceiling absorbs the cohort aging into it, no wrap to zero"
        );
    }

    #[test]
    fn mortality_conserves_count_exactly() {
        let mut h = AgeHistogram::from_pairs([(30, 100), (90, 100)]);
        let before = h.total();
        let deaths = h.apply_mortality(&rising_hazard(), 0xA6E, 1, 0);
        assert_eq!(
            h.total() + deaths,
            before,
            "survivors plus deaths equal the population before"
        );
        assert!(deaths >= 0 && deaths <= before);
    }

    #[test]
    fn mortality_replays_bit_identically() {
        let start = AgeHistogram::from_pairs([(30, 500), (90, 500)]);
        let mut a = start.clone();
        let mut b = start.clone();
        let da = a.apply_mortality(&rising_hazard(), 0x5EED, 7, 3);
        let db = b.apply_mortality(&rising_hazard(), 0x5EED, 7, 3);
        assert_eq!(da, db, "same key, same deaths");
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "same key, same resulting distribution"
        );
    }

    #[test]
    fn mortality_differs_by_cadence_and_by_pool() {
        // The cohort in a bucket rotates each cadence, so the same bucket must roll
        // differently across cadences; and two distinct pools must roll independently. A
        // multi-age distribution is compared by its full canonical hash, so a decorrelated
        // stream is caught by the whole survivor distribution rather than one bucket's death
        // count, which two independent streams could coincide on.
        let start = AgeHistogram::from_pairs([(30, 2000), (60, 2000), (90, 2000)]);
        let hz = flat_hazard(3, 10);
        let mut c0 = start.clone();
        let mut c1 = start.clone();
        let mut p2 = start.clone();
        c0.apply_mortality(&hz, 0x11, 1, 0);
        c1.apply_mortality(&hz, 0x11, 1, 1);
        p2.apply_mortality(&hz, 0x11, 2, 0);
        assert_ne!(
            c0.state_hash(),
            c1.state_hash(),
            "a different cadence decorrelates the survivor distribution"
        );
        assert_ne!(
            c0.state_hash(),
            p2.state_hash(),
            "a different pool decorrelates the survivor distribution"
        );
    }

    #[test]
    fn mortality_at_the_age_ceiling_reads_the_oldest_hazard() {
        // A cast must never author demography (Principle 9). A cohort at the numeric age
        // ceiling faces the oldest, most-lethal point of a rising hazard through the curve's
        // monotone end clamp, never wrapping negative to read the youngest, least-lethal
        // point. This is the regression guard for the signedness-wrap the red-team found.
        let n = 4000u64;
        let hz = rising_hazard(); // 1% at age 0, certain by age 120
        let mut ancient = AgeHistogram::from_pairs([(u32::MAX, n)]);
        let mut young = AgeHistogram::from_pairs([(0, n)]);
        let ancient_deaths = ancient.apply_mortality(&hz, 0xA9E, 1, 0);
        let young_deaths = young.apply_mortality(&hz, 0xA9E, 1, 0);
        assert_eq!(
            ancient_deaths, n as i128,
            "the age ceiling reads the certain oldest hazard, not the youngest"
        );
        assert!(
            young_deaths < ancient_deaths,
            "a rising hazard takes more at the ceiling than at age zero ({young_deaths} vs {ancient_deaths})"
        );
    }

    #[test]
    fn zero_hazard_kills_nobody() {
        // The Steering Audit: the mechanism adds no mortality of its own. With no authored
        // biology (a flat zero hazard) the population is untouched.
        let mut h = AgeHistogram::from_pairs([(10, 1000), (99, 1000)]);
        let before = h.total();
        let deaths = h.apply_mortality(&flat_hazard(0, 1), 0xDEAD, 1, 0);
        assert_eq!(deaths, 0);
        assert_eq!(h.total(), before, "a zero hazard kills no one");
    }

    #[test]
    fn unit_hazard_kills_everybody() {
        // The other end of the same audit: a certain hazard removes the whole cohort. The
        // outcome tracks the supplied biology exactly, from nothing to everything.
        let mut h = AgeHistogram::from_pairs([(10, 1000), (99, 1000)]);
        let deaths = h.apply_mortality(&flat_hazard(1, 1), 0xBEEF, 1, 0);
        assert_eq!(deaths, 2000);
        assert_eq!(h.total(), 0, "a certain hazard leaves no survivors");
    }

    #[test]
    fn the_hazard_is_the_sole_author_of_demography() {
        // The Steering Audit's core invariant (Principle 9): the demographic outcome is
        // authored by the supplied hazard, never by the mechanism or a population label.
        let start = AgeHistogram::from_pairs([(20, 6000)]);
        let harsh = flat_hazard(4, 10); // 0.40
        let mild = flat_hazard(1, 10); // 0.10
        let seed = 0x77;

        // Direction one: hold the pool key fixed, vary only the hazard. The outcome tracks
        // the biology it is handed, and the harsher curve takes more. The mechanism adds
        // nothing of its own beyond consuming the curve.
        let mut under_harsh = start.clone();
        let mut under_mild = start.clone();
        let harsh_deaths = under_harsh.apply_mortality(&harsh, seed, 1, 0);
        let mild_deaths = under_mild.apply_mortality(&mild, seed, 1, 0);
        assert!(
            harsh_deaths > mild_deaths,
            "with the pool key fixed, a harsher hazard takes more ({harsh_deaths} vs {mild_deaths})"
        );

        // Direction two: hold the hazard fixed, vary only the pool label. No pool is
        // favoured; the label decorrelates the RNG stream but carries no hidden demographic
        // bias, so the death fractions match within sampling noise over a large cohort.
        let mut pool_a = start.clone();
        let mut pool_b = start.clone();
        let a_frac = pool_a.apply_mortality(&harsh, seed, 1, 0) as f64 / 6000.0;
        let b_frac = pool_b.apply_mortality(&harsh, seed, 2, 0) as f64 / 6000.0;
        assert!(
            (a_frac - b_frac).abs() < 0.04,
            "the pool label carries no hidden bias ({a_frac} vs {b_frac})"
        );
    }

    #[test]
    fn tier_symmetry_holds_in_expectation() {
        // Record 62.9: the aggregate and individual tiers agree by conservation and
        // distribution, never member for member. A single-age cohort suffers pool-tier
        // mortality; the same cohort modelled as named individuals (the individual tier's
        // key: the being's id and age) suffers its own; both death fractions land close to
        // the supplied hazard and close to each other. This is distributional agreement, the
        // realisable half of tier consistency, not the unattainable identical-outcomes half.
        let age = 40u32;
        let n = 6000u64;
        let chance = flat_hazard(3, 10); // 0.30
        let seed = 0xC0FFEE;

        let mut pool = AgeHistogram::from_pairs([(age, n)]);
        let pool_deaths = pool.apply_mortality(&chance, seed, 1, 0) as f64;

        // The individual tier: each being has its own id and rolls under Phase::MORTALITY on
        // its (id, age) key, exactly as World::apply_mortality does.
        let p = chance.eval(hazard_age(age));
        let mut indiv_deaths = 0u64;
        for id in 0..n {
            let roll = DrawKey::entity(id, age as u64, Phase::MORTALITY)
                .rng(seed)
                .unit_fixed(0);
            if roll < p {
                indiv_deaths += 1;
            }
        }

        let expected = 0.30 * n as f64;
        let pool_frac = pool_deaths / n as f64;
        let indiv_frac = indiv_deaths as f64 / n as f64;
        // Both are Binomial(n, 0.30) samples; within a few percent of the mean and of each
        // other over 6000 draws.
        assert!(
            (pool_deaths - expected).abs() < 0.03 * n as f64,
            "pool death fraction {pool_frac} tracks the hazard"
        );
        assert!(
            (indiv_deaths as f64 - expected).abs() < 0.03 * n as f64,
            "individual death fraction {indiv_frac} tracks the hazard"
        );
        assert!(
            (pool_frac - indiv_frac).abs() < 0.05,
            "the two tiers agree in expectation ({pool_frac} vs {indiv_frac})"
        );
    }

    #[test]
    fn add_births_feeds_the_moment_accumulator_and_conserves() {
        // The pool-tier birth inflow: newborns enter the age-zero cohort and the breeder's
        // reproductive contribution feeds the moment accumulator, so a coarse pool derives Ne with
        // no individuals. Population is conserved as an inflow (age zero rises by the offspring
        // added), and the moments reduce to a positive effective size.
        use crate::breeding::SexClass;
        let mut ages = AgeHistogram::from_pairs([(20, 30), (40, 20)]);
        let mut moments = ReproductiveMoments::new();
        let before = ages.total();
        // Two breeding classes, a spread of family sizes.
        ages.add_births(&mut moments, SexClass(0), 3);
        ages.add_births(&mut moments, SexClass(1), 1);
        ages.add_births(&mut moments, SexClass(0), 2);
        assert_eq!(
            ages.count_at(0),
            6,
            "the newborns enter the age-zero cohort"
        );
        assert_eq!(
            ages.total(),
            before + 6,
            "the inflow conserves as a pure addition"
        );
        assert_eq!(moments.breeders(), 3, "each breeder is recorded once");
        assert!(
            moments.effective_size() > 0,
            "the pool derives Ne from the moments alone"
        );
    }

    #[test]
    fn merge_conserves_the_total() {
        let mut a = AgeHistogram::from_pairs([(10, 5), (20, 7)]);
        let b = AgeHistogram::from_pairs([(20, 3), (30, 4)]);
        let total = a.total() + b.total();
        a.merge(&b);
        assert_eq!(a.total(), total, "a merge conserves the summed total");
        assert_eq!(a.count_at(20), 10, "same-age members combine");
        assert_eq!(a.count_at(30), 4);
    }

    #[test]
    fn hashing_is_assembly_order_independent() {
        let forward = AgeHistogram::from_pairs([(1, 2), (5, 9), (33, 1)]);
        let shuffled = AgeHistogram::from_pairs([(33, 1), (1, 1), (5, 9), (1, 1)]);
        assert_eq!(
            forward.state_hash(),
            shuffled.state_hash(),
            "one distribution has one hash however it was built"
        );
    }

    #[test]
    fn age_total_registers_as_a_conserved_projection() {
        // The first aggregate-tier age conserved projection (R-PROJ-REGISTER): the age
        // total is declared to the conservation registry, and aging and merging preserve it
        // while a fabricated leak is caught. This is the seam a running two-tier world folds
        // in when it adopts pool-tier demography.
        let mut reg: ConservationRegistry<AgeHistogram> = ConservationRegistry::new();
        reg.register("age_population", |h: &AgeHistogram| h.total());

        let before = AgeHistogram::from_pairs([(0, 40), (10, 25)]);
        let mut after = before.clone();
        after.age_step();
        assert!(
            reg.check(&before, &after).is_ok(),
            "aging conserves the age population"
        );

        let mut merged = before.clone();
        merged.merge(&AgeHistogram::from_pairs([(10, 5)]));
        assert!(
            reg.check(&before, &merged).is_err(),
            "a genuine inflow is not conservation and the registry says so"
        );
    }
}
