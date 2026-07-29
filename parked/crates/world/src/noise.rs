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

//! Deterministic fixed-point fractal value noise for worldgen (design Part 12).
//!
//! Pure integer and fixed-point. A lattice value is a counter-RNG draw keyed by the
//! canonical [`civsim_core::DrawKey`] schema (the octave as the region, the lattice
//! corner as the two loci, the field as the draw-site slot, and no tick because worldgen
//! is genesis-time), and interpolation is smoothstep in [`Fixed`]. The same seed yields
//! the same field on any machine, with no float and no platform-dependent transcendental,
//! so the square-root and divide question of R-GPU-CANON-PIN never enters this path.

use civsim_core::{DrawKey, Fixed, Phase, ABSENT};

/// The field a noise sample is for. It is the draw-site slot, so the three fields draw
/// independent streams at the same lattice point.
pub const FIELD_ELEVATION: u32 = 0;
/// Moisture noise field.
pub const FIELD_MOISTURE: u32 = 1;
/// Temperature noise field.
pub const FIELD_TEMPERATURE: u32 = 2;

/// A lattice value in `[0, ONE)`, a counter-RNG draw at one lattice corner for one octave
/// and field. The corner coordinates are non-negative on a `FlatBounded` world.
#[inline]
fn lattice(seed: u64, octave: u32, gx: i32, gy: i32, field: u32) -> Fixed {
    DrawKey::pair(gx as u64, gy as u64, ABSENT, Phase::WORLDGEN)
        .in_region(octave as u64)
        .slot(field)
        .rng(seed)
        .unit_fixed(0)
}

/// Smoothstep, `3t^2 - 2t^3 = t*t*(3 - 2t)`, for `t` in `[0, ONE]`.
#[inline]
fn smoothstep(t: Fixed) -> Fixed {
    let three = Fixed::from_int(3);
    let two = Fixed::from_int(2);
    t.mul(t).mul(three - two.mul(t))
}

/// Linear interpolation `a + (b - a) * t`.
#[inline]
fn lerp(a: Fixed, b: Fixed, t: Fixed) -> Fixed {
    a + (b - a).mul(t)
}

/// One octave of value noise at `(x, y)` with lattice spacing `period` tiles, bilinearly
/// interpolated with smoothstep weights. Result in `[0, ONE)`.
fn octave_value(seed: u64, x: i32, y: i32, period: i32, octave: u32, field: u32) -> Fixed {
    let period = period.max(1);
    let gx = x.div_euclid(period);
    let gy = y.div_euclid(period);
    let fx = Fixed::from_ratio(x.rem_euclid(period) as i64, period as i64);
    let fy = Fixed::from_ratio(y.rem_euclid(period) as i64, period as i64);
    let v00 = lattice(seed, octave, gx, gy, field);
    let v10 = lattice(seed, octave, gx + 1, gy, field);
    let v01 = lattice(seed, octave, gx, gy + 1, field);
    let v11 = lattice(seed, octave, gx + 1, gy + 1, field);
    let sx = smoothstep(fx);
    let sy = smoothstep(fy);
    let top = lerp(v00, v10, sx);
    let bot = lerp(v01, v11, sx);
    lerp(top, bot, sy)
}

/// Fractal value noise: octaves summed with halving amplitude and halving period,
/// normalised to `[0, ONE)`. `base_period` is the coarsest lattice spacing in tiles.
pub fn fractal(seed: u64, x: i32, y: i32, field: u32, base_period: i32, octaves: u32) -> Fixed {
    let mut acc = Fixed::ZERO;
    let mut total = Fixed::ZERO;
    for o in 0..octaves.max(1) {
        let amp = Fixed::from_ratio(1, 1i64 << o);
        let period = base_period >> o;
        acc += amp.mul(octave_value(seed, x, y, period, o, field));
        total += amp;
    }
    if total == Fixed::ZERO {
        Fixed::ZERO
    } else {
        acc.div(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractal_is_deterministic_and_in_unit_range() {
        for y in 0..32 {
            for x in 0..32 {
                let a = fractal(0xA11CE, x, y, FIELD_ELEVATION, 16, 4);
                let b = fractal(0xA11CE, x, y, FIELD_ELEVATION, 16, 4);
                assert_eq!(a, b, "same seed and coordinate reproduce the same value");
                assert!(
                    a >= Fixed::ZERO && a < Fixed::ONE,
                    "value out of [0,1): {a:?}"
                );
            }
        }
    }

    #[test]
    fn fields_and_seeds_give_different_streams() {
        let elev = fractal(7, 5, 9, FIELD_ELEVATION, 16, 4);
        let moist = fractal(7, 5, 9, FIELD_MOISTURE, 16, 4);
        let other = fractal(8, 5, 9, FIELD_ELEVATION, 16, 4);
        assert_ne!(elev, moist, "the field slot separates the streams");
        assert_ne!(elev, other, "the seed separates the streams");
    }

    #[test]
    fn lattice_corners_are_exact() {
        // At a lattice corner the fractional offset is zero, so the value is the lattice
        // draw itself for the coarsest octave, blended over octaves but reproducible.
        let a = fractal(1, 0, 0, FIELD_ELEVATION, 16, 1);
        let b = fractal(1, 0, 0, FIELD_ELEVATION, 16, 1);
        assert_eq!(a, b);
    }
}
