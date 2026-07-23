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

//! The integer-Gaussian approximation (design 25.10): determinism, the first two moments,
//! symmetry, and the tail bound. The stamped world identity is `SumOfUniforms { k: 12 }`,
//! whose unit-variance scale is exactly one and whose deviate is bounded to `+/- 6`.

use civsim_core::{gaussian, gaussian_unit, DrawKey, Fixed, GaussApprox, Phase, Rng};

const K12: GaussApprox = GaussApprox::SumOfUniforms { k: 12 };

fn stream(seed: u64, locus: u64) -> Rng {
    const TEST_PHASE: Phase = Phase(0xA11C_E001);
    DrawKey::entity(locus, 0, TEST_PHASE).rng(seed)
}

#[test]
fn a_deviate_is_a_pure_function_of_its_stream_and_counter() {
    let r = stream(0xD1CE, 7);
    let a = gaussian_unit(&r, 0, K12);
    let b = gaussian_unit(&r, 0, K12);
    assert_eq!(
        a, b,
        "the same stream and counter reproduce the same deviate"
    );
    // A different base counter draws a different (but reproducible) deviate.
    let c = gaussian_unit(&r, 12, K12);
    assert_eq!(c, gaussian_unit(&r, 12, K12));
    assert_ne!(a, c, "a different base counter is a different draw");
}

#[test]
fn k12_scale_is_exactly_one() {
    // For k = 12 the unit-variance scale sqrt(12/k) is exactly one, so the deviate is the
    // centred sum with no rescaling: (sum of 12 unit draws) - 6.
    let r = stream(0x5CA1E, 3);
    let mut sum = Fixed::ZERO;
    for i in 0..12u64 {
        sum += r.unit_fixed(i);
    }
    let expected = sum - Fixed::from_int(6);
    assert_eq!(gaussian_unit(&r, 0, K12), expected);
}

#[test]
fn the_sample_mean_is_near_zero_and_the_variance_near_one() {
    // Draw many independent deviates (one per stream) and check the first two moments against
    // the unit-Gaussian target within a fixed tolerance.
    let n: i64 = 20_000;
    let mut sum_bits: i128 = 0;
    let mut sumsq_bits: i128 = 0; // accumulate x^2 in Q32.32 bits
    for locus in 0..n as u64 {
        let x = gaussian_unit(&stream(0xA11CE, locus), 0, K12);
        sum_bits += x.to_bits() as i128;
        sumsq_bits += x.mul(x).to_bits() as i128;
    }
    let mean = Fixed::from_bits((sum_bits / n as i128) as i64);
    let mean_sq = Fixed::from_bits((sumsq_bits / n as i128) as i64);
    // Var = E[x^2] - E[x]^2, with E[x] ~ 0 so E[x^2] ~ Var.
    let var = mean_sq - mean.mul(mean);
    assert!(
        mean.abs() < Fixed::from_ratio(1, 20),
        "sample mean near zero: {mean}"
    );
    assert!(
        (var - Fixed::ONE).abs() < Fixed::from_ratio(1, 10),
        "sample variance near one: {var}"
    );
}

#[test]
fn the_distribution_is_symmetric() {
    // Over many streams, deviates above and below zero are balanced.
    let n: i64 = 20_000;
    let mut above = 0i64;
    let mut below = 0i64;
    for locus in 0..n as u64 {
        let x = gaussian_unit(&stream(0x5EED, locus), 0, K12);
        if x > Fixed::ZERO {
            above += 1;
        } else if x < Fixed::ZERO {
            below += 1;
        }
    }
    let diff = (above - below).abs();
    assert!(
        diff < n / 20,
        "roughly balanced above/below zero: above={above} below={below}"
    );
}

#[test]
fn the_tail_bound_is_never_exceeded() {
    // The sum-of-uniforms deviate at k = 12 is bounded to +/- 6: the honest limit of the
    // approximation. No draw over a large sample crosses it.
    let bound = Fixed::from_int(6);
    for locus in 0..100_000u64 {
        let x = gaussian_unit(&stream(0xB0117, locus), 0, K12);
        assert!(x.abs() <= bound, "deviate within +/- 6: {x}");
    }
}

#[test]
fn gaussian_applies_mean_and_std() {
    // gaussian(mean, std) = mean + std * gaussian_unit.
    let r = stream(0xF00D, 11);
    let unit = gaussian_unit(&r, 0, K12);
    let mean = Fixed::from_int(5);
    let std = Fixed::from_int(2);
    let expected = mean + std.mul(unit);
    assert_eq!(gaussian(&r, 0, mean, std, K12), expected);
    // A zero std collapses to the mean, whatever the stream.
    assert_eq!(gaussian(&r, 0, mean, Fixed::ZERO, K12), mean);
}

#[test]
#[should_panic(expected = "unset")]
fn the_unset_sentinel_panics_rather_than_choosing() {
    let r = stream(1, 1);
    // The Default is the loud-fail sentinel k = 0; drawing on it must panic.
    let _ = gaussian_unit(&r, 0, GaussApprox::default());
}

#[test]
#[should_panic(expected = "unavailable")]
fn the_unavailable_inverse_cdf_table_panics() {
    let r = stream(1, 1);
    let _ = gaussian_unit(&r, 0, GaussApprox::InvCdfTable { bits: 16 });
}
