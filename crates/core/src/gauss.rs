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

//! Deterministic integer-Gaussian candidate mechanisms.
//!
//! The arithmetic supplies reproducible mean-zero deviates without floating point. The
//! approximation shape remains value-bearing method content. This module does not choose
//! or admit one: a canonical caller must derive it, complete a floor-admission receipt, or
//! refuse. Pre-migration planetary assembly carries the choice explicitly so it cannot be
//! hidden in the numeric provider.
//!
//! Everything here is integer and fixed-point over the counter-keyed [`Rng`]: a deviate is
//! a pure function of its stream and counter, so a derived realization is bit-identical
//! across machines and thread counts.
//! There is no float. The one square root the scale can need (`sqrt(12/k)` for a
//! non-canonical `k`) is a single [`Fixed::sqrt`] computed once per call, never inside the
//! sub-draw loop; the stamped `k = 12` returns unit scale without any square root at all.

use crate::fixed::Fixed;
use crate::hash::StateHasher;
use crate::rng::Rng;

/// An integer-Gaussian approximation method carried explicitly by a caller.
///
/// This is a mechanism selector, not an admitted default. Changing it changes the generated
/// realization, so canonical use requires a derived or admitted method identity upstream.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum GaussApprox {
    /// The central-limit sum of `k` counter-keyed unit draws, centred by subtracting `k/2` and
    /// scaled by `sqrt(12/k)` to unit variance. At `k = 12` the scale is exactly one and the
    /// deviate's tails are bounded to `+/- 6`. A larger `k` changes both cost and shape.
    SumOfUniforms { k: u8 },
    /// An inverse-CDF table lookup at `bits` of precision. It is not built;
    /// [`gaussian_unit`] panics rather than fabricating a shape.
    InvCdfTable { bits: u8 },
}

impl Default for GaussApprox {
    /// The loud-fail sentinel for an unset method. `SumOfUniforms { k: 0 }` is not a usable
    /// approximation (a zero-term sum has no variance), so [`gaussian_unit`] panics on it rather
    /// than silently choosing a shape. A pool or scheme that will draw a Gaussian must have its
    /// method supplied upstream; a caller that never draws can retain the sentinel.
    fn default() -> Self {
        GaussApprox::SumOfUniforms { k: 0 }
    }
}

impl GaussApprox {
    /// Fold the method identity into a state hash in a fixed byte order.
    #[inline]
    pub fn hash_into(&self, hasher: &mut StateHasher) {
        match *self {
            GaussApprox::SumOfUniforms { k } => {
                hasher.write_u32(0x5001_0000);
                hasher.write_u32(k as u32);
            }
            GaussApprox::InvCdfTable { bits } => {
                hasher.write_u32(0x1CDF_0000);
                hasher.write_u32(bits as u32);
            }
        }
    }
}

/// The unit-variance scale `sqrt(12/k)` for the sum-of-uniforms approximation. For the stamped
/// `k = 12` this is exactly [`Fixed::ONE`] (`12/12 = 1`), the common path, returned without any
/// square root; for another `k` it is one [`Fixed::sqrt`], computed once per call and never in a
/// sub-draw loop.
#[inline]
fn unit_variance_scale(k: u8) -> Fixed {
    if k == 12 {
        Fixed::ONE
    } else {
        Fixed::from_ratio(12, k as i64).sqrt()
    }
}

/// A mean-zero, unit-variance Gaussian-approximate deviate on a single counter-keyed stream
/// (design 25.10).
///
/// For `SumOfUniforms { k }` it sums the `k` unit draws at counters `base_counter ..
/// base_counter + k` on one stream, subtracts `k/2` to centre the sum, and multiplies by
/// `sqrt(12/k)` to reach unit variance. The result is bounded to `+/- (k/2) * sqrt(12/k)` (for
/// `k = 12`, `+/- 6`), the declared limit of the central-limit approximation. The draw
/// is a pure function of the stream and the base counter, so it reproduces bit for bit on any
/// machine and thread count.
///
/// Panics on the unset sentinel `SumOfUniforms { k: 0 }` and on `InvCdfTable`, rather
/// than fabricating a shape.
pub fn gaussian_unit(rng: &Rng, base_counter: u64, approx: GaussApprox) -> Fixed {
    match approx {
        GaussApprox::SumOfUniforms { k } => {
            assert!(
                k > 0,
                "gauss approximation is unset (sentinel k = 0); an upstream derived or admitted method identity is required before a Gaussian is drawn"
            );
            let mut sum = Fixed::ZERO;
            for i in 0..k as u64 {
                sum += rng.unit_fixed(base_counter + i);
            }
            let centered = sum - Fixed::from_ratio(k as i64, 2);
            centered.mul(unit_variance_scale(k))
        }
        GaussApprox::InvCdfTable { .. } => {
            panic!("InvCdfTable gaussian approximation is unavailable")
        }
    }
}

/// A Gaussian-approximate deviate with the given `mean` and standard deviation `std`:
/// `mean + std * gaussian_unit(rng, base_counter, approx)`. The same determinism and tail-bound
/// guarantees as [`gaussian_unit`] carry through.
#[inline]
pub fn gaussian(
    rng: &Rng,
    base_counter: u64,
    mean: Fixed,
    std: Fixed,
    approx: GaussApprox,
) -> Fixed {
    mean + std.mul(gaussian_unit(rng, base_counter, approx))
}
