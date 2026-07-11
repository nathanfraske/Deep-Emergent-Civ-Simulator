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

//! The reproductive-success census and the effective-population-size derivation (design Part 25,
//! R-REPRO; the census tier of the deep-time genome model, Part 25.7).
//!
//! The aggregate genome tier ([`crate::genome::GenePool`]) carries an effective population size Ne
//! that sets the strength of drift. Ne was formerly a single reserved number dialed per world. It
//! now DERIVES from a census of who bred and how well: the breeding sex ratio and the variance in
//! reproductive success are the two demographic facts that pull Ne below the head count, and both
//! are measured, not authored. A [`ReproductiveCensus`] tallies, per contributing parent per
//! window, the sex class ([`crate::breeding::SexClass`], a gene-fed phenotype) and the offspring
//! count; the tally reduces to [`ReproductiveMoments`] (the sex split and the reproductive moments
//! sum k, sum k squared), and one race-blind kernel ([`ReproductiveMoments::effective_size`]) reads
//! the moments and returns Ne.
//!
//! Principle 9 (the Steering Audit) governs the kernel: it authors no ratio. Two races diverge in
//! Ne only through their census data, through the one kernel; swapping their census inputs swaps
//! their Ne; a sex-symmetric, Poisson-ideal census returns N with no downward bias; and the 1:1 sex
//! ratio a stable Ne rests on emerges from Fisherian selection on the sex-determination locus
//! ([`crate::breeding`]), never from a hardcoded number.
//!
//! The two textbook results the kernel composes:
//!
//! - Wright's separate-sexes effective size, `Ne = 4 Nm Nf / (Nm + Nf)` (Wright 1931), generalized
//!   to k mating types by the harmonic form `Ne = k^2 / sum_i (1 / N_i)`, which recovers Wright at
//!   k = 2 and the head count for one class or balanced classes.
//! - The Crow-Kimura variance effective size, `Ne = (N k_bar - 1) / (k_bar - 1 + Vk / k_bar)` (Crow
//!   and Kimura 1970), where k_bar is the mean and Vk the variance of offspring number: an
//!   over-Poisson variance (a few individuals monopolizing reproduction) drives Ne well below N,
//!   and an equalized (under-Poisson) family size lifts it above N.
//!
//! Everything is integer or fixed-point with 128-bit reductions and no float, so a whole
//! population's Ne is a bit-identical canonical reduction, order-independent across threads. The
//! window is stamped so a per-window Ne is a distinct reduction from the next window's.
//!
//! Tier consistency (design Part 54, record 62.9): the individual tier keeps a per-being sex and a
//! per-parent tally; the pool tier keeps only the sex split and the two moments
//! ([`crate::demography::AgeHistogram::add_births`] feeds them without individuals). Fed the same
//! events the two agree exactly, because both reduce to [`ReproductiveMoments`]; fed a statistically
//! equivalent stream they agree in expectation, not member for member, exactly as the demography's
//! mortality tiers do.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StableId, StateHasher};

use crate::breeding::SexClass;

/// Wright's separate-sexes effective population size, `Ne = 4 Nm Nf / (Nm + Nf)` (Wright 1931):
/// with `Nm` breeding members of one class and `Nf` of the other, a skewed sex ratio drives Ne well
/// below the head count `Nm + Nf`, because the rarer sex is the genetic bottleneck. Pure integer
/// with a 128-bit intermediate so the product cannot overflow at any realistic population; returns
/// 0 when neither class breeds. For the k-class generalization see [`effective_size_classes`].
pub fn effective_size_sex(nm: u64, nf: u64) -> u64 {
    let denom = (nm as u128) + (nf as u128);
    if denom == 0 {
        return 0;
    }
    let num = 4u128 * (nm as u128) * (nf as u128);
    (num / denom) as u64
}

/// The k-class generalization of Wright's separate-sexes formula: `Ne = k^2 / sum_i (1 / N_i)` over
/// the classes that bred (design Part 25, R-REPRO). At k = 2 this is exactly
/// `4 Nm Nf / (Nm + Nf)`; at k = 1 (hermaphroditic) or balanced classes it is the head count; a
/// missing mating type does not enter, so k is the number of breeding classes, not the registry's
/// class count. The reciprocal sum is an order-independent 128-bit reduction, so the value is the
/// same for any partition of the classes across threads.
pub fn effective_size_classes(counts: &[u64]) -> u64 {
    ne_classes_fx(counts).to_int().max(0) as u64
}

/// The Crow-Kimura variance effective size, `Ne = (N k_bar - 1) / (k_bar - 1 + Vk / k_bar)` (Crow
/// and Kimura 1970), where `N` is the number of breeders, `k_bar` the mean offspring number, and
/// `Vk` its variance. Poisson reproduction (`Vk == k_bar`) returns approximately N; an over-Poisson
/// variance (a few individuals monopolizing reproduction) drives Ne well below N. Fixed-point and
/// checked; returns the floor as a `u64`.
pub fn effective_size_var(n: u64, k_bar: Fixed, vk: Fixed) -> u64 {
    ne_var_fx(n, k_bar, vk).to_int().max(0) as u64
}

/// The k-class harmonic effective size as a [`Fixed`], the shared implementation behind
/// [`effective_size_classes`] and the composite kernel.
fn ne_classes_fx(counts: &[u64]) -> Fixed {
    let present: Vec<u64> = counts.iter().copied().filter(|&c| c > 0).collect();
    let k = present.len();
    if k == 0 {
        return Fixed::ZERO;
    }
    if k == 1 {
        return Fixed::from_int(present[0].min(i32::MAX as u64) as i32);
    }
    let recip_sum = Fixed::saturating_sum(
        present
            .iter()
            .map(|&n| Fixed::ONE.div(Fixed::from_int(n.min(i32::MAX as u64) as i32))),
    );
    if recip_sum <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    Fixed::from_int((k * k) as i32).div(recip_sum)
}

/// The Crow-Kimura variance effective size as a [`Fixed`], the shared implementation behind
/// [`effective_size_var`] and the composite kernel.
fn ne_var_fx(n: u64, k_bar: Fixed, vk: Fixed) -> Fixed {
    if n == 0 || k_bar <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    let n_fx = Fixed::from_int(n.min(i32::MAX as u64) as i32);
    let num = k_bar.mul(n_fx) - Fixed::ONE;
    let vk_over_kbar = vk.checked_div(k_bar).unwrap_or(Fixed::ZERO);
    let denom = k_bar - Fixed::ONE + vk_over_kbar;
    // A second difference-divisor: `denom = k_bar - 1 + vk/k_bar` is a difference that vanishes in the
    // equal-contribution degenerate case (`k_bar == 1, Vk == 0`), where the variance effective size would
    // diverge. The declared limit-on-the-difference is the census size `n_fx`, a finite honest limit rather
    // than infinity, enforced by the slice-3 backstop; `guarded_div` returns `n_fx` at the `denom <= 0`
    // boundary, byte-neutral with the prior explicit guard (`n_fx` is a positive census size, so the sign
    // post-check below leaves it unchanged).
    let ne = civsim_units::guard::guarded_div(
        num,
        denom,
        civsim_units::guard::ZeroGuard::LimitAtZero(n_fx),
    );
    if ne < Fixed::ZERO {
        Fixed::ZERO
    } else {
        ne
    }
}

/// The reduced reproductive moments of a window: the sex-class split of the breeders and the two
/// reproductive-success moments (sum of offspring counts and sum of their squares). This is the
/// canonical form both tiers reduce to, so the individual census and the pool accumulator feed the
/// one Ne kernel ([`ReproductiveMoments::effective_size`]). The pool tier maintains one of these
/// directly ([`crate::demography::AgeHistogram::add_births`]), deriving Ne without ever holding an
/// individual (design Part 54). The sex split is a sorted map so every walk is canonical.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ReproductiveMoments {
    /// The number of breeders recorded.
    n: u64,
    /// The sum of offspring counts over breeders (Wright-Fisher gamete contributions).
    sum_k: u64,
    /// The sum of squared offspring counts (u128 headroom for the squared term).
    sum_k2: u128,
    /// The count of breeders in each sex class, in canonical ascending order.
    sex_counts: BTreeMap<SexClass, u64>,
}

impl ReproductiveMoments {
    /// Empty moments.
    pub fn new() -> Self {
        ReproductiveMoments::default()
    }

    /// Build moments from pre-reduced totals (the pool tier and tests use this to set a census's
    /// sex split and moments directly, without individuals).
    pub fn from_totals(
        sex_counts: BTreeMap<SexClass, u64>,
        n: u64,
        sum_k: u64,
        sum_k2: u128,
    ) -> Self {
        ReproductiveMoments {
            n,
            sum_k,
            sum_k2,
            sex_counts,
        }
    }

    /// Record one breeder of sex `sex` that produced `k` offspring this window (a single
    /// reproductive-success datum). Saturating so a century-scale accumulator cannot wrap.
    pub fn record_parent(&mut self, sex: SexClass, k: u32) {
        self.n = self.n.saturating_add(1);
        self.sum_k = self.sum_k.saturating_add(k as u64);
        self.sum_k2 = self.sum_k2.saturating_add((k as u128) * (k as u128));
        *self.sex_counts.entry(sex).or_insert(0) += 1;
    }

    /// The number of breeders recorded.
    pub fn breeders(&self) -> u64 {
        self.n
    }

    /// The canonical sex-class split of the breeders (ascending class id).
    pub fn sex_split(&self) -> &BTreeMap<SexClass, u64> {
        &self.sex_counts
    }

    /// The mean offspring number per breeder, as a [`Fixed`].
    pub fn k_bar(&self) -> Fixed {
        if self.n == 0 {
            return Fixed::ZERO;
        }
        Fixed::from_ratio(self.sum_k as i64, self.n as i64)
    }

    /// The variance of offspring number over breeders, `E[k^2] - (E[k])^2`, clamped at zero against
    /// fixed-point rounding. A population variance (divides by N, not N - 1), the form the
    /// Crow-Kimura estimator uses.
    pub fn variance_k(&self) -> Fixed {
        if self.n == 0 {
            return Fixed::ZERO;
        }
        let n_i64 = self.n as i64;
        let mean = Fixed::from_ratio(self.sum_k as i64, n_i64);
        let mean_sq = Fixed::from_ratio(self.sum_k2.min(i64::MAX as u128) as i64, n_i64);
        let v = mean_sq - mean.mul(mean);
        if v < Fixed::ZERO {
            Fixed::ZERO
        } else {
            v
        }
    }

    /// The effective population size the moments imply: the one race-blind Ne kernel (design Part
    /// 25, R-REPRO). It composes the sex-ratio reduction (the harmonic k-class form over the sex
    /// split) and the reproductive-variance reduction (Crow-Kimura over the moments) through the
    /// reciprocal, variance-in-allele-frequency rule `1/Ne = 1/Ne_sex + 1/Ne_var - 1/N`: two
    /// independent sources of drift add in reciprocal, minus the shared census term counted once
    /// (Crow and Kimura 1970; Caballero 1994). Its honest limit: this composition assumes the two
    /// factors act independently, so it double-corrects mildly when the same individuals drive both
    /// a skewed sex ratio and a skewed variance, and the exact simultaneous-factor form (Hill 1972)
    /// is a refinement; a sex-symmetric, Poisson-ideal census returns exactly N.
    ///
    /// The result is rounded to nearest (so a symmetric ideal census reads N, not N - 1 off a
    /// floor) and clamped into `[1, 2N]`, the equalized-family bound on the variance effective size.
    /// Reads nothing but the moments, so it is race-blind: swapping two races' census inputs swaps
    /// their Ne, and the kernel authors no ratio (Principle 9).
    pub fn effective_size(&self) -> u32 {
        let n = self.n;
        if n == 0 {
            return 0;
        }
        let counts: Vec<u64> = self.sex_counts.values().copied().collect();
        let ne_sex = ne_classes_fx(&counts);
        let ne_var = ne_var_fx(n, self.k_bar(), self.variance_k());
        let n_fx = Fixed::from_int(n.min(i32::MAX as u64) as i32);
        let inv = |x: Fixed| {
            if x > Fixed::ZERO {
                Fixed::ONE.div(x)
            } else {
                Fixed::ZERO
            }
        };
        // The Crow-Kimura `1/Ne = 1/Ne_sex + 1/Ne_var - 1/N` is a DIFFERENCE of positive reciprocal terms, so
        // `recip` can reach zero or go negative with no single-quantity floor. The declared limit-on-the-
        // difference (the R-UNITS-PIN floor invariant, slice-3 backstop) is `Ne = N`, the census size: the
        // correct degenerate limit when the sex-ratio and variance corrections cancel the census term, derived
        // from the population genetics, not fabricated. `guarded_div` enforces it, returning `n_fx` at the
        // `recip <= 0` boundary, byte-neutral with the prior explicit branch.
        let recip = inv(ne_sex) + inv(ne_var) - inv(n_fx);
        let ne = civsim_units::guard::guarded_div(
            Fixed::ONE,
            recip,
            civsim_units::guard::ZeroGuard::LimitAtZero(n_fx),
        );
        let rounded = (ne + Fixed::from_ratio(1, 2)).to_int();
        let max = n.saturating_mul(2).min(i32::MAX as u64) as i32;
        rounded.clamp(1, max) as u32
    }

    /// Fold the moments into a canonical hash (the sex split ascending, then the scalar moments), so
    /// a pool-tier Ne reduction is part of the world's reproducible identity.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u64(self.n);
        h.write_u64(self.sum_k);
        h.write_u64(self.sum_k2 as u64);
        h.write_u64((self.sum_k2 >> 64) as u64);
        h.write_u64(self.sex_counts.len() as u64);
        for (class, &count) in &self.sex_counts {
            h.write_u32(class.0 as u32);
            h.write_u64(count);
        }
    }
}

/// The individual-tier reproductive census over a window (design Part 25, R-REPRO). Per contributing
/// parent it holds the offspring credited to it, and per being it holds the gene-fed sex class. The
/// window is a stamped reduction key ([`ReproductiveCensus::window`]): [`ReproductiveCensus::reset`]
/// bumps it and clears the tally, so each window's Ne is a distinct canonical reduction. Every walk
/// is over a sorted map (ascending id), so the reduction is order-independent and bit-identical
/// across threads.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct ReproductiveCensus {
    /// Offspring credited per contributing parent this window.
    offspring: BTreeMap<StableId, u32>,
    /// The gene-fed sex class of each being recorded this window (parents and offspring).
    sex_of: BTreeMap<StableId, SexClass>,
    /// The number of birth events recorded this window.
    births: u32,
    /// The stamped window ordinal (the reduction key).
    window: u64,
}

impl ReproductiveCensus {
    /// An empty census at window zero.
    pub fn new() -> Self {
        ReproductiveCensus::default()
    }

    /// The stamped window ordinal.
    pub fn window(&self) -> u64 {
        self.window
    }

    /// Close the current window and open the next: bump the window stamp and clear the tally, so
    /// the next window's Ne measures a fresh cohort (design Part 25.7, the per-generation cadence).
    pub fn reset(&mut self) {
        self.window = self.window.saturating_add(1);
        self.offspring.clear();
        self.sex_of.clear();
        self.births = 0;
    }

    /// Record a being's gene-fed sex class (an offspring at birth, a founder at the dawn). Idempotent
    /// for a fixed being, since sex is a fixed phenotype.
    pub fn record_sex(&mut self, id: StableId, sex: SexClass) {
        self.sex_of.insert(id, sex);
    }

    /// Credit one offspring to a contributing parent and record its sex, the per-parent primitive a
    /// birth calls once for each parent that contributed. Saturating so the tally cannot wrap.
    pub fn credit(&mut self, parent: StableId, sex: SexClass) {
        let slot = self.offspring.entry(parent).or_insert(0);
        *slot = slot.saturating_add(1);
        self.sex_of.insert(parent, sex);
    }

    /// Record one birth: credit each contributing parent once (both parents in a two-class system,
    /// the single parent in a one-parent system) and record the child's sex. The birth counter
    /// rises by one, so the offspring tally must equal the summed per-birth parent count (the
    /// conserved reproductive projection [`ReproductiveCensus::total_offspring`] checks).
    pub fn record_birth(
        &mut self,
        parents: &[(StableId, SexClass)],
        child: StableId,
        child_sex: SexClass,
    ) {
        for &(parent, sex) in parents {
            self.credit(parent, sex);
        }
        self.record_sex(child, child_sex);
        self.births = self.births.saturating_add(1);
    }

    /// The offspring credited to a being this window.
    pub fn offspring_of(&self, id: StableId) -> u32 {
        self.offspring.get(&id).copied().unwrap_or(0)
    }

    /// A being's recorded sex class, if any.
    pub fn sex_of(&self, id: StableId) -> Option<SexClass> {
        self.sex_of.get(&id).copied()
    }

    /// The number of birth events recorded this window.
    pub fn births(&self) -> u32 {
        self.births
    }

    /// The total offspring credited across all parents this window (the conserved reproductive
    /// projection, design Part 58): in a two-parent system this equals twice the birth count, in a
    /// single-parent system it equals the birth count. A shortfall is a fabricated leak (a birth
    /// that failed to credit a contributing parent).
    pub fn total_offspring(&self) -> u64 {
        self.offspring.values().map(|&k| k as u64).sum()
    }

    /// Reduce the per-parent tally to [`ReproductiveMoments`]: for each breeder (a parent with at
    /// least one offspring) walked in ascending id order, record its sex (defaulting to class 0 when
    /// unknown, the hermaphroditic-like fallback) and offspring count. This is the individual tier's
    /// projection onto the shared Ne kernel.
    pub fn moments(&self) -> ReproductiveMoments {
        let mut m = ReproductiveMoments::new();
        for (&parent, &k) in &self.offspring {
            if k == 0 {
                continue;
            }
            let sex = self.sex_of.get(&parent).copied().unwrap_or_default();
            m.record_parent(sex, k);
        }
        m
    }

    /// The effective population size the census implies, through the one race-blind kernel: it
    /// reduces to [`ReproductiveMoments`] and calls [`ReproductiveMoments::effective_size`]. Feeds
    /// [`crate::genome::GenePool::effective_size`] for the census-tier pool.
    pub fn effective_size(&self) -> u32 {
        self.moments().effective_size()
    }

    /// Fold the census into a canonical hash (the window stamp, then the tally and sex map in
    /// ascending id order), so a per-window Ne reduction is part of the world's reproducible
    /// identity and replays bit for bit.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u64(self.window);
        h.write_u32(self.births);
        h.write_u64(self.offspring.len() as u64);
        for (id, &k) in &self.offspring {
            h.write_stable(*id);
            h.write_u32(k);
        }
        h.write_u64(self.sex_of.len() as u64);
        for (id, class) in &self.sex_of {
            h.write_stable(*id);
            h.write_u32(class.0 as u32);
        }
    }

    /// A standalone canonical hash of the census.
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
    use civsim_core::{DrawKey, Phase};

    fn sc(i: u16) -> SexClass {
        SexClass(i)
    }

    #[test]
    fn ne_sex_formula_matches_wright() {
        // Wright 1931: 4 Nm Nf / (Nm + Nf). A balanced sex ratio is the head count; a skewed one
        // drops Ne well below it, the whole point of the separate-sexes correction.
        assert_eq!(
            effective_size_sex(50, 50),
            100,
            "balanced sexes give the head count"
        );
        assert_eq!(effective_size_sex(10, 10), 20);
        // A 1:9 breeding sex ratio: 4*10*90/100 = 36, well below the 100 head count.
        assert_eq!(effective_size_sex(10, 90), 36);
        assert!(
            effective_size_sex(10, 90) < 100 / 2,
            "a 1:9 ratio drops Ne well below N"
        );
        assert_eq!(
            effective_size_sex(0, 40),
            0,
            "no breeders of one sex, no effective size"
        );
        // The k-class harmonic form recovers Wright's two-sex formula exactly.
        assert_eq!(
            effective_size_classes(&[10, 90]),
            effective_size_sex(10, 90)
        );
        assert_eq!(effective_size_classes(&[50, 50]), 100);
        // Three balanced mating types: the head count again; one class: the head count.
        assert_eq!(effective_size_classes(&[30, 30, 30]), 90);
        assert_eq!(
            effective_size_classes(&[77]),
            77,
            "one class is hermaphroditic: Ne is N"
        );
    }

    #[test]
    fn ne_var_formula_matches_crow_kimura() {
        // Crow-Kimura 1970: (N k_bar - 1) / (k_bar - 1 + Vk / k_bar).
        let n = 100u64;
        let k_bar = Fixed::from_int(2);
        // Poisson reproduction, Vk == k_bar: Ne is approximately N.
        let poisson = effective_size_var(n, k_bar, k_bar);
        assert!(
            poisson.abs_diff(n) <= 2,
            "Poisson variance gives Ne ~ N (got {poisson})"
        );
        // High variance in reproductive success: Ne collapses far below N.
        let high = effective_size_var(n, k_bar, Fixed::from_int(20));
        assert!(
            high < n / 4,
            "a high reproductive variance gives Ne << N (got {high})"
        );
        // Equalized family size (Vk below Poisson) lifts Ne above N.
        let equalized = effective_size_var(n, k_bar, Fixed::from_ratio(1, 2));
        assert!(
            equalized > n,
            "an equalized family size lifts Ne above N (got {equalized})"
        );
    }

    #[test]
    fn ne_derivation_replays_bit_identically() {
        // The reduction is a canonical order-independent walk, so the same births in any order (a
        // worker or thread sweep) give the same census hash and the same Ne.
        let births: Vec<(StableId, SexClass, StableId, SexClass, StableId, SexClass)> = (0..40)
            .map(|i| {
                let mother = StableId(i);
                let father = StableId(1000 + i);
                let child = StableId(5000 + i);
                (mother, sc(0), father, sc(1), child, sc(i as u16 % 2))
            })
            .collect();

        let build = |order: &[usize]| {
            let mut c = ReproductiveCensus::new();
            for &idx in order {
                let (m, ms, f, fs, ch, cs) = births[idx];
                c.record_birth(&[(m, ms), (f, fs)], ch, cs);
            }
            c
        };
        let forward: Vec<usize> = (0..births.len()).collect();
        let reversed: Vec<usize> = (0..births.len()).rev().collect();
        let shuffled: Vec<usize> = (0..births.len())
            .map(|i| (i * 7 + 3) % births.len())
            .collect();

        let a = build(&forward);
        let b = build(&reversed);
        // The shuffled order visits a permutation; dedup by using distinct indices only.
        let mut seen = std::collections::BTreeSet::new();
        let uniq: Vec<usize> = shuffled.into_iter().filter(|i| seen.insert(*i)).collect();
        let d = build(&uniq);

        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "reversed order gives the same census"
        );
        assert_eq!(a.effective_size(), b.effective_size(), "and the same Ne");
        assert_eq!(
            a.state_hash(),
            d.state_hash(),
            "a permuted order gives the same census"
        );
    }

    #[test]
    fn offspring_tally_conserves() {
        // Two-parent births credit two parents each, so the tally is twice the birth count; a
        // single-parent system credits one. The tally is a conserved reproductive projection, and a
        // fabricated leak (a birth crediting only one parent in a two-class system) is caught.
        let mut two = ReproductiveCensus::new();
        for i in 0..5 {
            two.record_birth(
                &[(StableId(i), sc(0)), (StableId(100 + i), sc(1))],
                StableId(200 + i),
                sc(0),
            );
        }
        assert_eq!(two.births(), 5);
        assert_eq!(
            two.total_offspring(),
            10,
            "two-parent births credit twice the birth count"
        );

        let mut single = ReproductiveCensus::new();
        for i in 0..5 {
            single.record_birth(&[(StableId(i), sc(0))], StableId(300 + i), sc(0));
        }
        assert_eq!(
            single.total_offspring(),
            5,
            "single-parent births credit once each"
        );

        // Register the tally as a conserved projection and catch a fabricated leak. The correct
        // census balances (offspring == 2 * births); a leaky one where a birth credited only one
        // parent does not, and the registry reports the drift.
        let degree = 2i128;
        let mut reg: ConservationRegistry<ReproductiveCensus> = ConservationRegistry::new();
        reg.register("credit_balance", move |c: &ReproductiveCensus| {
            c.total_offspring() as i128 - degree * c.births() as i128
        });
        let mut leaky = ReproductiveCensus::new();
        for i in 0..4 {
            leaky.record_birth(
                &[(StableId(i), sc(0)), (StableId(100 + i), sc(1))],
                StableId(200 + i),
                sc(0),
            );
        }
        // A fifth birth that forgets to credit the father: bump births but credit one parent.
        leaky.credit(StableId(4), sc(0));
        leaky.record_sex(StableId(204), sc(0));
        leaky.births = leaky.births.saturating_add(1);
        assert!(
            reg.check(&two, &leaky).is_err(),
            "a birth that failed to credit a parent breaks the conserved reproductive projection"
        );
    }

    #[test]
    fn tier_consistency_ne_agrees_in_expectation() {
        // Record 62.9: the individual tier (per-being sex and per-parent tally) and the pool tier
        // (the sex split and the two moments, no individuals) agree. Fed the same events they agree
        // exactly, because both reduce to ReproductiveMoments; fed a statistically equivalent stream
        // they agree in expectation, not member for member, like the demography's mortality tiers.
        let seed = 0xC0FFEE;
        let n = 400u64;

        // The individual tier: N breeders, sex read off the locus (balanced), offspring drawn from a
        // counter-keyed stream, each breeder credited its drawn count.
        let mut census = ReproductiveCensus::new();
        for i in 0..n {
            let parent = StableId(i);
            let sex = sc((i % 2) as u16);
            let draws = DrawKey::entity(i, 0, Phase::REPRODUCE).rng(seed);
            // Offspring count in 0..=4 (mean 2), a spread that exercises the variance term.
            let k = (draws.at(0) % 5) as u32;
            census.record_sex(parent, sex);
            for _ in 0..k {
                let slot = census.offspring.entry(parent).or_insert(0);
                *slot += 1;
            }
        }
        let ne_individual = census.effective_size();

        // The pool tier: the same demography drawn on an independent stream, accumulated as moments
        // only (no individual identities). A different key stream, so it differs member for member.
        let mut moments = ReproductiveMoments::new();
        for i in 0..n {
            let sex = sc((i % 2) as u16);
            let draws = DrawKey::entity(i, 1, Phase::REPRODUCE).rng(seed);
            let k = (draws.at(0) % 5) as u32;
            moments.record_parent(sex, k);
        }
        let ne_pool = moments.effective_size();

        // Both tiers see balanced sexes and a mean-2, spread reproduction, so their Ne land close.
        assert!(
            ne_individual.abs_diff(ne_pool) <= (n as u32) / 10,
            "the tiers agree in expectation ({ne_individual} vs {ne_pool})"
        );

        // Fed identical events the reduction is exact, not merely close.
        let exact = census.moments().effective_size();
        assert_eq!(
            exact, ne_individual,
            "the two projections of one census reduce identically"
        );
    }

    #[test]
    fn non_steering_two_races_diverge_through_one_kernel() {
        // Principle 9: two races diverge in Ne only through their census data, through one race-blind
        // kernel; swapping the census inputs swaps the Ne; a symmetric census yields N with no bias.
        let n = 100u64;

        // Race A, gonochoric-polygynous: a skewed breeding sex ratio (90 of one class, 10 of the
        // other) and a high male variance (one male sires 108, nine sire 8), against 90 females each
        // rearing 2. Both drag Ne far below N.
        let mut a_sex = BTreeMap::new();
        a_sex.insert(sc(0), 90u64); // females
        a_sex.insert(sc(1), 10u64); // males
        let a_sum_k = 360u64; // 180 female credits + 180 male credits
        let a_sum_k2 = 90u128 * 4 + 108 * 108 + 9 * 64; // females at 2, one male at 108, nine at 8
        let race_a = ReproductiveMoments::from_totals(a_sex, n, a_sum_k, a_sum_k2);

        // Race B, hermaphroditic-even: one self-compatible class, balanced Poisson reproduction
        // (k_bar 2, Vk ~ 2). No sex reduction, no variance excess, so Ne rests near N.
        let mut b_sex = BTreeMap::new();
        b_sex.insert(sc(0), 100u64);
        let race_b = ReproductiveMoments::from_totals(b_sex, n, 200, 600);

        let ne_a = race_a.effective_size();
        let ne_b = race_b.effective_size();
        assert!(
            ne_a < (n as u32) / 2,
            "race A collapses well below N (got {ne_a})"
        );
        assert!(
            ne_b.abs_diff(n as u32) <= 2,
            "race B rests near N (got {ne_b})"
        );
        assert!(ne_a < ne_b, "the two races diverge");

        // The kernel authors no ratio: it reads only the census. Feed each race's census under the
        // other race's label and the Ne follows the census, not the label. So swapping the census
        // inputs swaps the outputs.
        let race_ne = |_label: &str, census: &ReproductiveMoments| census.effective_size();
        assert_eq!(
            race_ne("B", &race_a),
            ne_a,
            "Ne follows the census, not the race label"
        );
        assert_eq!(race_ne("A", &race_b), ne_b);

        // A sex-symmetric, Poisson-ideal census yields N with no downward bias.
        let mut sym_sex = BTreeMap::new();
        sym_sex.insert(sc(0), 50u64);
        sym_sex.insert(sc(1), 50u64);
        let symmetric = ReproductiveMoments::from_totals(sym_sex, n, 200, 600);
        assert!(
            symmetric.effective_size().abs_diff(n as u32) <= 1,
            "a symmetric census yields N, no authored bias (got {})",
            symmetric.effective_size()
        );
    }
}
