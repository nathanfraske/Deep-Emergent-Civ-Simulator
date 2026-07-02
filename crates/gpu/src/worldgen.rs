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

//! Deterministic fixed-point fractal worldgen noise on the GPU (design Part 12, offload-map step 2),
//! the CubeCL counterpart of `civsim_world`'s `noise::fractal`. It is the cleanest standalone GPU
//! offload in the engine: each cell's field value is a pure function of its coordinates through the
//! draw-keyed counter RNG (R-RNG-COORD), so there is zero input to upload (the thread index IS the
//! input), only the result reads back, and it is genesis-time one-shot with no per-tick transfer. Per
//! cell it folds a `DrawKey::pair(gx, gy, ABSENT, WORLDGEN).in_region(octave).slot(field)` lattice draw
//! at four corners per octave, interpolates with a Q32.32 smoothstep, sums octaves with halving
//! amplitude and period, and normalises with the pinned `q32_div`. Every step is the oracle's
//! arithmetic through the pinned primitives (`splitmix64`, `q32_mul`, `q32_div`), so it is bit-identical
//! to `noise::fractal` on CUDA (`tests/worldgen_gate`).

use crate::prim::{q32_div, q32_mul, splitmix64};
use crate::stage0::CudaClient;
use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

/// A lattice value in `[0, ONE)`: the counter-RNG draw at one lattice corner for one octave and field,
/// bit-identical to `noise::lattice`. Folds `[octave, gx, gy, ABSENT, WORLDGEN, field]` (the
/// `DrawKey::pair(...).in_region(octave).slot(field)` coordinate order) through `splitmix64` with
/// `rotate_left((i % 63) + 1)`, then `unit_fixed(0) = at(0) >> 32`.
#[allow(clippy::manual_rotate)] // the DSL exposes no rotate_left intrinsic
#[cube]
fn lattice(seed: u64, octave: u64, gx: u64, gy: u64, field: u64) -> i64 {
    let mut key = splitmix64(seed);
    let absent = 0xFFFF_FFFF_FFFF_FFFFu64; // ABSENT = u64::MAX (the tick coordinate)
    key = splitmix64(key ^ ((octave << 1u32) | (octave >> 63u32))); // i=0, region = octave
    key = splitmix64(key ^ ((gx << 2u32) | (gx >> 62u32))); // i=1, locus = gx
    key = splitmix64(key ^ ((gy << 3u32) | (gy >> 61u32))); // i=2, locus2 = gy
    key = splitmix64(key ^ ((absent << 4u32) | (absent >> 60u32))); // i=3, tick = ABSENT
    let worldgen = 16u64; // Phase::WORLDGEN = 0x10
    key = splitmix64(key ^ ((worldgen << 5u32) | (worldgen >> 59u32))); // i=4, phase
    key = splitmix64(key ^ ((field << 6u32) | (field >> 58u32))); // i=5, slot = field
    let at0 = splitmix64(key);
    i64::cast_from(at0 >> 32u32)
}

/// Smoothstep `3t^2 - 2t^3 = t*t*(3 - 2t)` on Q32.32 bits, bit-identical to `noise::smoothstep`.
#[cube]
fn smoothstep(t: i64) -> i64 {
    let three = 12884901888i64; // 3 << 32
    let two = 8589934592i64; // 2 << 32
    q32_mul(q32_mul(t, t), three - q32_mul(two, t))
}

/// Linear interpolation `a + (b - a) * t` on Q32.32 bits, bit-identical to `noise::lerp`.
#[cube]
fn lerp(a: i64, b: i64, t: i64) -> i64 {
    a + q32_mul(b - a, t)
}

/// One octave of value noise at `(x, y)` with lattice spacing `period`, bit-identical to
/// `noise::octave_value`. The coordinates are non-negative (a `FlatBounded` world), so `div_euclid` /
/// `rem_euclid` are plain integer division and remainder.
#[cube]
fn octave_value(seed: u64, x: i64, y: i64, period: i64, octave: u64, field: u64) -> i64 {
    let p = select(period < 1i64, 1i64, period); // period.max(1)
    let gx = x / p;
    let gy = y / p;
    let rx = x - gx * p; // rem_euclid for non-negative x
    let ry = y - gy * p;
    let fx = (rx << 32u32) / p; // from_ratio(rx, p) = (rx << 32) / p
    let fy = (ry << 32u32) / p;
    let gxu = u64::cast_from(gx);
    let gyu = u64::cast_from(gy);
    let v00 = lattice(seed, octave, gxu, gyu, field);
    let v10 = lattice(seed, octave, gxu + 1u64, gyu, field);
    let v01 = lattice(seed, octave, gxu, gyu + 1u64, field);
    let v11 = lattice(seed, octave, gxu + 1u64, gyu + 1u64, field);
    let sx = smoothstep(fx);
    let sy = smoothstep(fy);
    let top = lerp(v00, v10, sx);
    let bot = lerp(v01, v11, sx);
    lerp(top, bot, sy)
}

/// The maximum octave count the manually-unrolled `fractal` covers. The launcher clamps `octaves` to
/// this, and octaves beyond the live count contribute zero. Worldgen uses about five.
const MAX_OCTAVES: u32 = 8;

/// Fractal value noise at `(x, y)` for `field`, bit-identical to `noise::fractal`: octaves summed with
/// halving amplitude and period, normalised by the amplitude total with the pinned `q32_div`. `octaves`
/// is at least one and at most `MAX_OCTAVES` (the launcher clamps), so the total is never zero.
///
/// The octaves are manually unrolled straight-line rather than a loop: the DSL drops an accumulator
/// carried across an `#[unroll]` loop whose body calls a `#[cube]` fn (see `transcendental::exp`), which
/// silently zeroed `acc`. Each octave `o` uses `amp = ONE >> o` and `period = base_period >> o` (literal
/// shifts), and a dead octave (`o >= octaves`) contributes zero through `select`, so the sum matches the
/// CPU's `for o in 0..octaves` for any live count.
#[cube]
fn fractal(seed: u64, x: i64, y: i64, field: u64, base_period: i64, octaves: u32) -> i64 {
    let one = 4294967296i64; // ONE = 1 << 32
    let mut acc = 0i64;
    let mut total = 0i64;

    let ov0 = octave_value(seed, x, y, base_period, 0u64, field);
    acc += select(0u32 < octaves, q32_mul(one, ov0), 0i64);
    total += select(0u32 < octaves, one, 0i64);

    let ov1 = octave_value(seed, x, y, base_period >> 1u32, 1u64, field);
    acc += select(1u32 < octaves, q32_mul(one >> 1u32, ov1), 0i64);
    total += select(1u32 < octaves, one >> 1u32, 0i64);

    let ov2 = octave_value(seed, x, y, base_period >> 2u32, 2u64, field);
    acc += select(2u32 < octaves, q32_mul(one >> 2u32, ov2), 0i64);
    total += select(2u32 < octaves, one >> 2u32, 0i64);

    let ov3 = octave_value(seed, x, y, base_period >> 3u32, 3u64, field);
    acc += select(3u32 < octaves, q32_mul(one >> 3u32, ov3), 0i64);
    total += select(3u32 < octaves, one >> 3u32, 0i64);

    let ov4 = octave_value(seed, x, y, base_period >> 4u32, 4u64, field);
    acc += select(4u32 < octaves, q32_mul(one >> 4u32, ov4), 0i64);
    total += select(4u32 < octaves, one >> 4u32, 0i64);

    let ov5 = octave_value(seed, x, y, base_period >> 5u32, 5u64, field);
    acc += select(5u32 < octaves, q32_mul(one >> 5u32, ov5), 0i64);
    total += select(5u32 < octaves, one >> 5u32, 0i64);

    let ov6 = octave_value(seed, x, y, base_period >> 6u32, 6u64, field);
    acc += select(6u32 < octaves, q32_mul(one >> 6u32, ov6), 0i64);
    total += select(6u32 < octaves, one >> 6u32, 0i64);

    let ov7 = octave_value(seed, x, y, base_period >> 7u32, 7u64, field);
    acc += select(7u32 < octaves, q32_mul(one >> 7u32, ov7), 0i64);
    total += select(7u32 < octaves, one >> 7u32, 0i64);

    q32_div(acc, total)
}

/// Per-cell worldgen kernel: `elev`, `moist`, `temp` are the three fractal fields (slots 0, 1, 2) at
/// each cell, row-major, `i64` Q32.32 bits. The thread's `(x, y)` is the sole input (coordinate-keyed),
/// so no field uploads, only the results read back.
#[cube(launch)]
fn noise_kernel(
    elev: &mut Array<i64>,
    moist: &mut Array<i64>,
    temp: &mut Array<i64>,
    width: u32,
    height: u32,
    seed: u64,
    base_period: i64,
    octaves: u32,
) {
    let x = ABSOLUTE_POS_X;
    let y = ABSOLUTE_POS_Y;
    if x < width && y < height {
        let idx = (y * width + x) as usize;
        let xi = i64::cast_from(x);
        let yi = i64::cast_from(y);
        elev[idx] = fractal(seed, xi, yi, 0u64, base_period, octaves);
        moist[idx] = fractal(seed, xi, yi, 1u64, base_period, octaves);
        temp[idx] = fractal(seed, xi, yi, 2u64, base_period, octaves);
    }
}

/// Generate the three worldgen noise fields on the GPU over a `width` x `height` grid, bit-identical to
/// `civsim_world`'s `noise::fractal` per cell on CUDA. Returns `(elevation, moisture, temperature)`,
/// each row-major `i64` Q32.32 bits. `octaves` is clamped to at least one (the CPU `octaves.max(1)`).
pub fn gpu_worldgen_noise(
    client: &CudaClient,
    width: u32,
    height: u32,
    seed: u64,
    base_period: i64,
    octaves: u32,
) -> (Vec<i64>, Vec<i64>, Vec<i64>) {
    let n = (width as usize) * (height as usize);
    if n == 0 {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let octaves = octaves.max(1);
    assert!(
        octaves <= MAX_OCTAVES,
        "gpu_worldgen_noise: octaves {octaves} exceeds the unroll bound {MAX_OCTAVES}"
    );
    let elev_h = client.empty(n * core::mem::size_of::<i64>());
    let moist_h = client.empty(n * core::mem::size_of::<i64>());
    let temp_h = client.empty(n * core::mem::size_of::<i64>());
    let tile = 16u32;
    let bx = width.div_ceil(tile);
    let by = height.div_ceil(tile);
    unsafe {
        noise_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(bx, by, 1),
            CubeDim::new_3d(tile, tile, 1),
            ArrayArg::from_raw_parts(elev_h.clone(), n),
            ArrayArg::from_raw_parts(moist_h.clone(), n),
            ArrayArg::from_raw_parts(temp_h.clone(), n),
            width,
            height,
            seed,
            base_period,
            octaves,
        );
    }
    let elev = i64::from_bytes(&client.read_one_unchecked(elev_h)).to_vec();
    let moist = i64::from_bytes(&client.read_one_unchecked(moist_h)).to_vec();
    let temp = i64::from_bytes(&client.read_one_unchecked(temp_h)).to_vec();
    (elev, moist, temp)
}
