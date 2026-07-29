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

//! Cross-crate binding for the sole `Fixed::log_sum_exp` provider. Physics retains a binary compatibility
//! wrapper and materials retains its physical composition API, but both must return the shared provider's bits.

use civsim_core::Fixed;
use civsim_materials::creep::{creep_total_log_rate, CreepComposition};
use civsim_physics::saha::log_sum_exp;

fn fixture(nums: &[i64], den: i64) -> Vec<Fixed> {
    nums.iter().map(|&n| Fixed::from_ratio(n, den)).collect()
}

#[test]
fn domain_entrypoints_delegate_to_the_shared_provider() {
    let cases = [
        fixture(&[3, 1], 1),
        fixture(&[7, 3, 3, 1], 1),
        fixture(&[1, 1, 1, 1, 1, 1, 1, 1], 1),
        fixture(&[2, 45], 1),
        fixture(&[0, -3, -6, -9, -12], 1),
        fixture(&[7, 5, 11, 3, 9, 1], 2),
    ];

    for values in cases {
        let shared = Fixed::log_sum_exp(&values).expect("fixtures are non-empty");
        assert_eq!(
            creep_total_log_rate(&values, CreepComposition::Parallel),
            shared,
            "parallel creep must consume the shared provider"
        );

        let negated: Vec<Fixed> = values.iter().map(|&value| -value).collect();
        let shared_sequential =
            -Fixed::log_sum_exp(&negated).expect("negated fixtures are non-empty");
        assert_eq!(
            creep_total_log_rate(&values, CreepComposition::Sequential),
            shared_sequential,
            "sequential creep must consume the shared provider"
        );
    }

    for pair in [
        fixture(&[3, 1], 1),
        fixture(&[2, 45], 1),
        fixture(&[-4, -9], 1),
    ] {
        assert_eq!(
            log_sum_exp(pair[0], pair[1]),
            Fixed::log_sum_exp(&pair).expect("pairs are non-empty"),
            "the Saha compatibility wrapper must delegate without moving a bit"
        );
    }
}

#[test]
fn shared_provider_tracks_the_real_log_domain_sum() {
    let scale = (1i64 << 32) as f64;
    for values in [
        fixture(&[5, 2, 1], 1),
        fixture(&[10, 9, 8, 7, 6, 5], 1),
        fixture(&[7, 5, 11, 3, 9, 1], 2),
    ] {
        let got = Fixed::log_sum_exp(&values).unwrap();
        let max = values
            .iter()
            .map(|value| value.to_f64_lossy())
            .fold(f64::NEG_INFINITY, f64::max);
        let oracle = max
            + values
                .iter()
                .map(|value| (value.to_f64_lossy() - max).exp())
                .sum::<f64>()
                .ln();
        let error_bits = (got.to_bits() as f64 - oracle * scale).abs();
        let derived_kernel_cap = values.len() as f64 * 42.0 + 40.0;
        assert!(
            error_bits <= derived_kernel_cap,
            "shared log-domain sum error {error_bits:.2} bits exceeds its exp-plus-ln kernel bound {derived_kernel_cap:.2}"
        );
    }
}
