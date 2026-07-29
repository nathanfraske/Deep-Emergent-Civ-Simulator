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

//! The quantitative breeding-value tier (design 25.8, 25.10, on the owner's stamped decisions):
//! tier consistency (a promoted cohort reconstructs the pool's additive genetic variance), the
//! unbiased promote-then-demote round trip, narrow-sense heritability as the derived read
//! V_A/(V_A+V_E), the non-steering property (the tier keys off the per-race effect vector, never a
//! race label, Principle 9), the additive mutation step as the sole lever that grows the spine, and
//! the locked-representation change (effects and the stamped approximation) folded into identity.

use civsim_bio::genome::{
    Allele, AlleleState, GenePool, GeneticScheme, Genome, Haplotype, LinkageGroup,
    ReproductionMode, SchemeId,
};
use civsim_core::{Fixed, GaussApprox, StateHasher};

const SCHEME: SchemeId = SchemeId(0);
const K12: GaussApprox = GaussApprox::SumOfUniforms { k: 12 };
const SEED: u64 = 0xB2EED;

fn f(num: i64, den: i64) -> Fixed {
    Fixed::from_ratio(num, den)
}

/// A pool with an explicit breeding-value spine: state-1 frequencies, per-locus effects alpha_i,
/// an effective size, and the stamped k=12 approximation.
fn pool(freqs: Vec<Fixed>, effects: Vec<Fixed>, ne: u32) -> GenePool {
    GenePool::new(SCHEME, ne, freqs).with_additive(effects, K12)
}

/// A promoted individual's additive breeding value: the additive spine summed over every allele
/// (what `express()` reads for a channel with unit weight and no environment).
fn breeding_value(g: &Genome) -> Fixed {
    let bits: i128 = g
        .haps
        .iter()
        .flat_map(|h| h.alleles.iter())
        .map(|a| a.additive.to_bits() as i128)
        .sum();
    Fixed::from_bits(bits as i64)
}

/// The sample variance of a slice of fixed-point values, accumulated in 128-bit bit space so the
/// order and magnitude are safe: mean, then the mean of squared deviations.
fn sample_variance(xs: &[Fixed]) -> Fixed {
    let n = xs.len() as i128;
    assert!(n > 1);
    let sum: i128 = xs.iter().map(|x| x.to_bits() as i128).sum();
    let mean = Fixed::from_bits((sum / n) as i64);
    let mut sq: i128 = 0;
    for &x in xs {
        let d = x - mean;
        sq += d.mul(d).to_bits() as i128;
    }
    Fixed::from_bits((sq / n) as i64)
}

/// The sample variance of the breeding values of `n` individuals promoted from a pool.
fn promoted_spread(p: &GenePool, n: u64, id_base: u64) -> Fixed {
    let bvs: Vec<Fixed> = (0..n)
        .map(|i| breeding_value(&p.promote(SEED, id_base + i, 2)))
        .collect();
    sample_variance(&bvs)
}

#[test]
fn a_promoted_cohort_reconstructs_the_pools_additive_variance() {
    // Tier consistency (R-TIER-CONSIST): the sample additive variance of a promoted cohort's
    // breeding values reconstructs GenePool::additive_variance() within a fixed tolerance.
    let p = pool(
        vec![f(1, 2), f(3, 10), f(7, 10), f(2, 5)],
        vec![Fixed::ONE, Fixed::from_int(2), f(1, 2), f(3, 2)],
        200,
    );
    let va = p.additive_variance();
    let spread = promoted_spread(&p, 6000, 0);
    let tol = va.div(Fixed::from_int(6)); // within ~17%: sampling noise, not systematic bias
    assert!(
        (spread - va).abs() <= tol,
        "promoted spread {spread} reconstructs V_A {va} (tol {tol})"
    );
}

#[test]
fn promote_then_demote_leaves_the_additive_mean_unchanged() {
    // The additive fold is unbiased: promote centres a per-locus additive sum on the pool mean, so
    // demoting the promoted individuals leaves the per-locus effects unchanged within fixed
    // rounding (a large Ne makes each fold a light nudge).
    let effects = vec![Fixed::ONE, Fixed::from_int(2), f(3, 4)];
    let p = pool(vec![f(1, 2), f(2, 5), f(3, 5)], effects.clone(), 10_000);
    let mut folded = p.clone();
    for i in 0..400u64 {
        let ind = p.promote(SEED, i, 2);
        folded.demote(&ind);
    }
    let tol = f(1, 20);
    for locus in 0..effects.len() {
        let before = p.effect(locus).unwrap();
        let after = folded.effect(locus).unwrap();
        assert!(
            (after - before).abs() <= tol,
            "additive mean at locus {locus} unchanged: {before} -> {after}"
        );
    }
}

#[test]
fn heritability_is_the_derived_va_over_va_plus_ve() {
    // narrow_sense_heritability(env_var) is exactly V_A/(V_A+V_E), and exactly one half when
    // V_E == V_A: the graduated read that replaces the authored constant.
    let p = pool(
        vec![f(1, 2), f(3, 10), f(2, 5)],
        vec![Fixed::ONE, Fixed::from_int(2), f(3, 2)],
        100,
    );
    let va = p.additive_variance();
    let env = f(3, 4);
    assert_eq!(
        p.narrow_sense_heritability(env),
        va.div(va + env),
        "h2 is the derived V_A/(V_A+V_E)"
    );
    // V_E set equal to V_A gives exactly one half.
    assert_eq!(
        p.narrow_sense_heritability(va),
        f(1, 2),
        "h2 is one half when V_E equals V_A"
    );
    // A flat pool has no heritable spread: h2 is zero, not a divide by zero.
    let flat = GenePool::new(SCHEME, 100, vec![f(1, 2), f(1, 2)]);
    assert_eq!(flat.additive_variance(), Fixed::ZERO);
    assert_eq!(flat.narrow_sense_heritability(env), Fixed::ZERO);
    assert_eq!(flat.narrow_sense_heritability(Fixed::ZERO), Fixed::ZERO);
}

#[test]
fn the_effect_vector_alone_authors_the_spine_not_the_race_label() {
    // The non-steering test (Principle 9): two pools with identical frequencies and Ne but
    // different per-race effect vectors (A dense small, B few large) diverge in V_A, heritability,
    // and promoted spread; relabelling swaps the outputs identically; and the same alpha with a
    // different id range gives the identical V_A. The kernel keys off the effect data, never a
    // label.
    let freqs = vec![f(1, 2); 8];
    let ne = 200;
    let env = f(1, 2);
    let dense_small = vec![f(1, 2); 8]; // eight small effects
    let mut few_large = vec![Fixed::ZERO; 8];
    few_large[0] = Fixed::from_int(2);
    few_large[1] = Fixed::from_int(2); // two large effects

    let a = pool(freqs.clone(), dense_small.clone(), ne);
    let b = pool(freqs.clone(), few_large.clone(), ne);

    let va_a = a.additive_variance();
    let va_b = b.additive_variance();
    assert_ne!(va_a, va_b, "different effect vectors give different V_A");
    assert_ne!(
        a.narrow_sense_heritability(env),
        b.narrow_sense_heritability(env),
        "different effect vectors give different heritability"
    );
    let spread_a = promoted_spread(&a, 4000, 0);
    let spread_b = promoted_spread(&b, 4000, 0);
    assert_ne!(
        spread_a, spread_b,
        "different effect vectors, different spread"
    );

    // Relabelling: swap which pool carries which alpha. The outputs swap identically, because V_A
    // is a pure function of (freqs, effects) and nothing else.
    let a_relabelled = pool(freqs.clone(), few_large.clone(), ne);
    let b_relabelled = pool(freqs.clone(), dense_small.clone(), ne);
    assert_eq!(a_relabelled.additive_variance(), va_b);
    assert_eq!(b_relabelled.additive_variance(), va_a);

    // Same alpha, different id range: identical V_A (id-independent) and a matching spread.
    let a2 = pool(freqs.clone(), dense_small, ne);
    assert_eq!(a2.additive_variance(), va_a, "V_A is id-independent");
    let spread_a_other = promoted_spread(&a2, 4000, 5_000_000);
    let tol = va_a.div(Fixed::from_int(5));
    assert!(
        (spread_a_other - spread_a).abs() <= tol,
        "the same alpha promotes the same spread from a different id range \
         ({spread_a} vs {spread_a_other}): the tier is race-blind"
    );
}

// --- The additive mutation step: the sole lever that grows the spine ---

/// A sexual-diploid scheme over `loci` loci that mutates every locus every reproduction (rate one),
/// with the given continuous additive step-size standard deviation and the stamped k=12 stamp.
fn step_scheme(loci: usize, step: Fixed) -> GeneticScheme {
    GeneticScheme {
        id: SCHEME,
        reproduction: ReproductionMode::SexualDiploid,
        linkage_groups: vec![LinkageGroup {
            loci: (0..loci as u32).collect(),
            recombination: vec![Fixed::ZERO; loci.saturating_sub(1)],
        }],
        mutation_rate: Fixed::ONE,
        additive_mutation_step: step,
        gauss: K12,
    }
}

/// A diploid genome over `loci` loci with every allele at zero additive and state zero.
fn zero_genome(loci: usize) -> Genome {
    let hap = || Haplotype {
        alleles: (0..loci)
            .map(|_| Allele {
                additive: Fixed::ZERO,
                state: AlleleState(0),
                origin: 0,
            })
            .collect(),
    };
    Genome {
        scheme: SCHEME,
        haps: vec![hap(), hap()],
    }
}

/// The sample variance of the breeding values of `n` lineages, each a self-reproduction chain of
/// `generations` steps from a zero founder under the given step scheme.
fn lineage_spread(loci: usize, step: Fixed, generations: u64, n: u64) -> Fixed {
    let scheme = step_scheme(loci, step);
    let founder = zero_genome(loci);
    let bvs: Vec<Fixed> = (0..n)
        .map(|lineage| {
            let mut ind = founder.clone();
            let mut pid = lineage * 4 + 1;
            for g in 0..generations {
                ind = scheme.reproduce(&ind, pid, &ind, pid + 1, loci, SEED, g);
                pid += 2;
            }
            breeding_value(&ind)
        })
        .collect();
    sample_variance(&bvs)
}

#[test]
fn the_additive_step_grows_variance_and_zero_freezes_the_spine() {
    let loci = 4;
    let step = f(1, 5);
    // One generation of mutation from a flat founder gives the spine a positive variance close to
    // the reserved rate: each of the 2*loci alleles gains an independent step*g, so Var ~
    // 2*loci*step^2.
    let var1 = lineage_spread(loci, step, 1, 4000);
    let expected1 = step.mul(step).mul(Fixed::from_int(2 * loci as i32));
    assert!(
        var1 > Fixed::ZERO,
        "the additive step grows variance: {var1}"
    );
    let tol = expected1.div(Fixed::from_int(4));
    assert!(
        (var1 - expected1).abs() <= tol,
        "one-generation variance {var1} tracks the reserved rate {expected1}"
    );
    // The variance keeps growing over generations (a random walk of steps): two generations exceed
    // one.
    let var2 = lineage_spread(loci, step, 2, 4000);
    assert!(
        var2 > var1,
        "additive variance grows with generations ({var1} -> {var2})"
    );
    // A zero step freezes the spine: no additive variation appears however many generations run.
    let frozen = lineage_spread(loci, Fixed::ZERO, 3, 2000);
    assert_eq!(
        frozen,
        Fixed::ZERO,
        "step-size zero freezes the additive spine"
    );
}

// --- The locked-representation change is visible in identity ---

#[test]
fn effects_and_the_stamp_are_folded_into_pool_identity() {
    fn hash(p: &GenePool) -> u128 {
        let mut h = StateHasher::new();
        p.hash_into(&mut h);
        h.finish()
    }
    let base = pool(
        vec![f(1, 2), f(1, 3)],
        vec![Fixed::ONE, Fixed::from_int(2)],
        100,
    );
    // Identical pools hash identically.
    let same = pool(
        vec![f(1, 2), f(1, 3)],
        vec![Fixed::ONE, Fixed::from_int(2)],
        100,
    );
    assert_eq!(hash(&base), hash(&same));
    // A different effect vector changes identity (the locked-representation change).
    let other_effects = pool(
        vec![f(1, 2), f(1, 3)],
        vec![Fixed::ONE, Fixed::from_int(3)],
        100,
    );
    assert_ne!(
        hash(&base),
        hash(&other_effects),
        "the additive spine is part of pool identity"
    );
    // A different stamped approximation changes identity, and differs from the flat default.
    let other_stamp = GenePool::new(SCHEME, 100, vec![f(1, 2), f(1, 3)]).with_additive(
        vec![Fixed::ONE, Fixed::from_int(2)],
        GaussApprox::SumOfUniforms { k: 6 },
    );
    assert_ne!(
        hash(&base),
        hash(&other_stamp),
        "the stamped approximation is part of pool identity"
    );
    let flat = GenePool::new(SCHEME, 100, vec![f(1, 2), f(1, 3)]);
    assert_ne!(hash(&base), hash(&flat));
}
