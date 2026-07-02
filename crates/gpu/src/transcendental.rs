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

//! The pinned Q32.32 transcendentals as CubeCL `#[cube]` kernels (R-GPU-CANON-PIN, the transcendental
//! stage). Each reproduces the `crates/core` `Fixed` oracle (`Fixed::exp` and its siblings) bit-for-bit
//! by running the same integer algorithm, with every `Fixed::mul`/`div` replaced by the limb
//! primitive that is itself bit-identical to the oracle (`q32_mul`, `q32_div`), and the adds, shifts,
//! and constants mirrored exactly. Bit-identity then follows by composition: same algorithm, each
//! primitive proven equal to the oracle op, so the whole function is equal.
//!
//! Like the diffusion field kernel, these use native `i64` at the boundary (the per-kernel layout the
//! proposal leaves to the author), so the bit-identity is proven on the CUDA backend and cross-vendor
//! identity on a backend without native 64-bit is a Stage 0 gate matter. No float appears anywhere.
//!
//! One composition caveat: where the oracle narrows an index to `i32` (the exp multiplier `k`, the
//! sin/cos quadrant `n`, the ln exponent `e`), these kernels keep it as `i64`. The result is
//! bit-identical because those values provably fit `i32` within each function's guarded domain, a
//! range bound the composition argument relies on rather than the ops being literally the same.

use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

use crate::prim::{isqrt_u96, q32_div, q32_mul};
use crate::stage0::CudaClient;

// The Q32.32 constants are the oracle's exact bit patterns (crates/core/src/fixed.rs), inlined as
// `let` literals inside each `#[cube]` function so they lift to DSL values.

// The limb primitives q32_mul / q32_div live in crate::prim (shared with the field kernel).

/// Variable left shift by a runtime amount (< 64), as a barrel shifter of constant shifts, since the
/// DSL supports only compile-time-constant shift amounts. Conditions each stage on a bit of `amt`.
#[cube]
fn shl_var(v: i64, amt: u32) -> i64 {
    let mut s = v;
    s = select((amt & 1u32) != 0u32, s << 1u32, s);
    s = select((amt & 2u32) != 0u32, s << 2u32, s);
    s = select((amt & 4u32) != 0u32, s << 4u32, s);
    s = select((amt & 8u32) != 0u32, s << 8u32, s);
    s = select((amt & 16u32) != 0u32, s << 16u32, s);
    s = select((amt & 32u32) != 0u32, s << 32u32, s);
    s
}

/// Variable arithmetic right shift by a runtime amount (< 64), the barrel counterpart of [`shl_var`].
#[cube]
fn shr_var(v: i64, amt: u32) -> i64 {
    let mut s = v;
    s = select((amt & 1u32) != 0u32, s >> 1u32, s);
    s = select((amt & 2u32) != 0u32, s >> 2u32, s);
    s = select((amt & 4u32) != 0u32, s >> 4u32, s);
    s = select((amt & 8u32) != 0u32, s >> 8u32, s);
    s = select((amt & 16u32) != 0u32, s >> 16u32, s);
    s = select((amt & 32u32) != 0u32, s >> 32u32, s);
    s
}

/// Multiply a Q32.32 value by `2^k` (k signed), the oracle's `scale_pow2`: saturating on a positive
/// overflow, flooring (arithmetic right shift) on a negative shift. The i128 range check is expressed
/// with i64 shifts: `v << k` overflows exactly when `v > (i64::MAX >> k)` or `v < (i64::MIN >> k)`.
#[cube]
fn scale_pow2(v: i64, k: i64) -> i64 {
    let maxv = 9223372036854775807i64; // i64::MAX
    let minv = (0i64 - 9223372036854775807i64) - 1i64; // i64::MIN
    let kpos = k >= 0i64;
    let ku = u32::cast_from(select(kpos, k, 0i64 - k)); // |k|
    let hi_lim = shr_var(maxv, ku);
    let lo_lim = shr_var(minv, ku);
    let shifted = shl_var(v, ku);
    let left = select(v > hi_lim, maxv, select(v < lo_lim, minv, shifted));
    let right = shr_var(v, ku);
    select(kpos, left, right)
}

/// `e^x`, the oracle's `Fixed::exp`: range-reduce `x = k*ln2 + r`, an 18-term Maclaurin Horner on `r`,
/// then scale by `2^k`. Outside about `[-22, 22]` it saturates to the maximum or to zero.
#[cube]
fn fixed_exp(x: i64) -> i64 {
    let ln2 = 2977044472i64; // ln 2
    let inv_ln2 = 6196328019i64; // 1 / ln 2
    let one = 4294967296i64; // from_int(1) = ONE
    let zero = 0i64;
    let pos22 = 94489280512i64; // from_int(22)
    let neg22 = 0i64 - 94489280512i64; // from_int(-22)
    let maxv = 9223372036854775807i64; // i64::MAX
                                       // k = to_int(x * (1/ln2)) as i64 (fits, |k| < 64); r = x - from_int(k)*ln2.
    let k = q32_mul(x, inv_ln2) >> 32u32;
    let k64 = k << 32u32;
    let r = x - q32_mul(k64, ln2);
    // acc = 1 + r*acc/from_int(i), i = 18 down to 1. Manually unrolled straight-line (the DSL rejects
    // an accumulator carried across an `#[unroll]` loop whose body calls a `#[cube]` fn); the divisors
    // are from_int(18)..from_int(1).
    let acc = one + q32_div(q32_mul(r, one), 77309411328i64);
    let acc = one + q32_div(q32_mul(r, acc), 73014444032i64);
    let acc = one + q32_div(q32_mul(r, acc), 68719476736i64);
    let acc = one + q32_div(q32_mul(r, acc), 64424509440i64);
    let acc = one + q32_div(q32_mul(r, acc), 60129542144i64);
    let acc = one + q32_div(q32_mul(r, acc), 55834574848i64);
    let acc = one + q32_div(q32_mul(r, acc), 51539607552i64);
    let acc = one + q32_div(q32_mul(r, acc), 47244640256i64);
    let acc = one + q32_div(q32_mul(r, acc), 42949672960i64);
    let acc = one + q32_div(q32_mul(r, acc), 38654705664i64);
    let acc = one + q32_div(q32_mul(r, acc), 34359738368i64);
    let acc = one + q32_div(q32_mul(r, acc), 30064771072i64);
    let acc = one + q32_div(q32_mul(r, acc), 25769803776i64);
    let acc = one + q32_div(q32_mul(r, acc), 21474836480i64);
    let acc = one + q32_div(q32_mul(r, acc), 17179869184i64);
    let acc = one + q32_div(q32_mul(r, acc), 12884901888i64);
    let acc = one + q32_div(q32_mul(r, acc), 8589934592i64);
    let acc = one + q32_div(q32_mul(r, acc), 4294967296i64);
    let scaled = scale_pow2(acc, k);
    let hi = select(x > pos22, maxv, scaled);
    select(x < neg22, zero, hi)
}

/// Elementwise `e^x` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::exp` on CUDA.
#[cube(launch)]
fn exp_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_exp(x_in[pos]);
    }
}

/// Run `Fixed::exp` on the GPU over a slice of `i64` Q32.32 bit patterns. Bit-identical to the CPU
/// `Fixed::exp` oracle (CUDA backend).
pub fn gpu_exp<R: Runtime>(client: &ComputeClient<R>, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        exp_kernel::launch::<R>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// `ln(x)`, the oracle's `Fixed::ln`: normalize `x = m*2^e` (m in [1,2)) by a leading-bit scan, then
/// `ln(x) = e*ln2 + ln(m)` with `ln(m)` from the atanh series on `(m-1)/(m+1)`. A non-positive input
/// returns `Fixed::MIN` (the fail-loud sentinel), matching the oracle.
#[cube]
fn fixed_ln(x: i64) -> i64 {
    let ln2 = 2977044472i64;
    let one = 4294967296i64; // from_int(1)
    let two = 8589934592i64; // from_int(2)
    let minv = (0i64 - 9223372036854775807i64) - 1i64; // i64::MIN sentinel
                                                       // e = msb(x) - 32; m = x >> e (e >= 0) or x << -e, so m is in [1, 2) as Q32.32.
    let lz = x.leading_zeros();
    let msb = 63i64 - i64::cast_from(lz);
    let e = msb - 32i64;
    let epos = e >= 0i64;
    let eu = u32::cast_from(select(epos, e, 0i64 - e));
    let m = select(epos, shr_var(x, eu), shl_var(x, eu));
    let u = q32_div(m - one, m + one);
    let w = q32_mul(u, u);
    // acc = sum_{j=0}^{12} w^j / from_int(2j+1), Horner from j=12 down; divisors from_int(25)..(1).
    let acc = q32_div(one, 107374182400i64);
    let acc = q32_mul(acc, w) + q32_div(one, 98784247808i64);
    let acc = q32_mul(acc, w) + q32_div(one, 90194313216i64);
    let acc = q32_mul(acc, w) + q32_div(one, 81604378624i64);
    let acc = q32_mul(acc, w) + q32_div(one, 73014444032i64);
    let acc = q32_mul(acc, w) + q32_div(one, 64424509440i64);
    let acc = q32_mul(acc, w) + q32_div(one, 55834574848i64);
    let acc = q32_mul(acc, w) + q32_div(one, 47244640256i64);
    let acc = q32_mul(acc, w) + q32_div(one, 38654705664i64);
    let acc = q32_mul(acc, w) + q32_div(one, 30064771072i64);
    let acc = q32_mul(acc, w) + q32_div(one, 21474836480i64);
    let acc = q32_mul(acc, w) + q32_div(one, 12884901888i64);
    let acc = q32_mul(acc, w) + q32_div(one, 4294967296i64);
    let ln_m = q32_mul(q32_mul(two, u), acc);
    let e64 = e << 32u32; // from_int(e)
    let main = q32_mul(e64, ln2) + ln_m;
    select(x <= 0i64, minv, main)
}

/// Elementwise `ln(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::ln` on CUDA.
#[cube(launch)]
fn ln_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_ln(x_in[pos]);
    }
}

/// Run `Fixed::ln` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_ln(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        ln_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// `x^y = exp(y * ln x)` for `x > 0`, the oracle's `Fixed::powf`; a non-positive base returns zero.
#[cube]
fn fixed_powf(base: i64, y: i64) -> i64 {
    let lnx = fixed_ln(base);
    let e = fixed_exp(q32_mul(y, lnx));
    select(base <= 0i64, 0i64, e)
}

/// Elementwise real power `base^y` over `i64` Q32.32 bit patterns on the GPU, bit-identical to
/// `Fixed::powf` on CUDA.
#[cube(launch)]
fn powf_kernel(base_in: &Array<i64>, y_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_powf(base_in[pos], y_in[pos]);
    }
}

/// Run `Fixed::powf` on the GPU: `out[i] = base[i]^y[i]`, bit-identical to `Fixed::powf` (CUDA). `base`
/// and `y` must have equal length.
pub fn gpu_powf(client: &CudaClient, base: &[i64], y: &[i64]) -> Vec<i64> {
    assert_eq!(base.len(), y.len(), "gpu_powf: mismatched input lengths");
    let n = base.len();
    if n == 0 {
        return Vec::new();
    }
    let bin = client.create_from_slice(i64::as_bytes(base));
    let yin = client.create_from_slice(i64::as_bytes(y));
    let out = client.empty(core::mem::size_of_val(base));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        powf_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(bin.clone(), n),
            ArrayArg::from_raw_parts(yin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// Fill a 32-entry local array with the CORDIC `atan(2^-i)` table (the oracle's `CORDIC_ATAN`).
#[cube]
fn cordic_atan_table() -> Array<i64> {
    let mut a = Array::<i64>::new(32usize);
    a[0usize] = 3373259426i64;
    a[1usize] = 1991351318i64;
    a[2usize] = 1052175346i64;
    a[3usize] = 534100635i64;
    a[4usize] = 268086748i64;
    a[5usize] = 134174063i64;
    a[6usize] = 67103403i64;
    a[7usize] = 33553749i64;
    a[8usize] = 16777131i64;
    a[9usize] = 8388597i64;
    a[10usize] = 4194303i64;
    a[11usize] = 2097152i64;
    a[12usize] = 1048576i64;
    a[13usize] = 524288i64;
    a[14usize] = 262144i64;
    a[15usize] = 131072i64;
    a[16usize] = 65536i64;
    a[17usize] = 32768i64;
    a[18usize] = 16384i64;
    a[19usize] = 8192i64;
    a[20usize] = 4096i64;
    a[21usize] = 2048i64;
    a[22usize] = 1024i64;
    a[23usize] = 512i64;
    a[24usize] = 256i64;
    a[25usize] = 128i64;
    a[26usize] = 64i64;
    a[27usize] = 32i64;
    a[28usize] = 16i64;
    a[29usize] = 8i64;
    a[30usize] = 4i64;
    a[31usize] = 2i64;
    a
}

/// CORDIC vectoring: `atan(y0/x0)` for `x0 > 0`, driving `y` to zero and accumulating the angle. The
/// oracle's `cordic_vectoring`, an `#[unroll]` loop of shift-add with the sign branch made branchless
/// by `select` (no `#[cube]` call in the body, so the loop-carried i64 state is allowed).
#[cube]
fn cordic_vectoring(x0: i64, y0: i64) -> i64 {
    let atan = cordic_atan_table();
    let mut x = x0;
    let mut y = y0;
    let mut z = 0i64;
    #[unroll]
    for i in 0usize..32usize {
        let sh = comptime!(i as u32);
        let dx = x >> sh;
        let dy = y >> sh;
        let ypos = y >= 0i64;
        let ai = atan[i];
        x = select(ypos, x + dy, x - dy);
        y = select(ypos, y - dx, y + dx);
        z = select(ypos, z + ai, z - ai);
    }
    z
}

/// `atan(x)` in radians, the oracle's `Fixed::atan` (CORDIC vectoring from `(1, x)`), saturating toward
/// the right angle for a very large magnitude.
#[cube]
fn fixed_atan(x: i64) -> i64 {
    let one = 4294967296i64;
    let half_pi = 6746518852i64;
    let bound = 1152921504606846976i64; // from_int(1 << 28) = 2^60
    let v = cordic_vectoring(one, x);
    let hi = select(x > bound, half_pi, v);
    select(x < (0i64 - bound), 0i64 - half_pi, hi)
}

/// Elementwise `atan(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::atan`.
#[cube(launch)]
fn atan_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_atan(x_in[pos]);
    }
}

/// Run `Fixed::atan` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_atan(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        atan_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// CORDIC circular rotation: for an angle in about `[-pi/4, pi/4]`, returns `[cos, sin]` (a 2-element
/// array to avoid a tuple return). The oracle's `cordic_rotation`, an `#[unroll]` shift-add loop with
/// the sign branch made branchless by `select`.
#[cube]
fn cordic_rotation(theta: i64) -> Array<i64> {
    let atan = cordic_atan_table();
    let mut x = 2608131496i64; // CORDIC_INV_GAIN (the prescale 1/A_32)
    let mut y = 0i64;
    let mut z = theta;
    #[unroll]
    for i in 0usize..32usize {
        let sh = comptime!(i as u32);
        let dx = x >> sh;
        let dy = y >> sh;
        let zpos = z >= 0i64;
        let ai = atan[i];
        x = select(zpos, x - dy, x + dy);
        y = select(zpos, y + dx, y - dx);
        z = select(zpos, z - ai, z + ai);
    }
    let mut out = Array::<i64>::new(2usize);
    out[0usize] = x; // cos
    out[1usize] = y; // sin
    out
}

/// `sin(x)`, the oracle's `Fixed::sin`: reduce the angle to `[-pi/4, pi/4]` by quadrant, CORDIC rotate,
/// and map back. Quadrant `n rem_euclid 4` is `n & 3` in two's complement.
#[cube]
fn fixed_sin(x: i64) -> i64 {
    let half_pi = 6746518852i64;
    let half = 2147483648i64; // 0.5 for round-to-nearest
    let n = (q32_div(x, half_pi) + half) >> 32u32; // round(x / (pi/2))
    let r = x - q32_mul(n << 32u32, half_pi);
    let cs = cordic_rotation(r);
    let c = cs[0usize];
    let s = cs[1usize];
    let neg_s = 0i64 - s;
    let neg_c = 0i64 - c;
    let q = n & 3i64;
    let out = neg_c; // q == 3
    let out = select(q == 2i64, neg_s, out);
    let out = select(q == 1i64, c, out);
    select(q == 0i64, s, out)
}

/// `cos(x)`, the oracle's `Fixed::cos` (the cosine component of the same reduction and rotation).
#[cube]
fn fixed_cos(x: i64) -> i64 {
    let half_pi = 6746518852i64;
    let half = 2147483648i64;
    let n = (q32_div(x, half_pi) + half) >> 32u32;
    let r = x - q32_mul(n << 32u32, half_pi);
    let cs = cordic_rotation(r);
    let c = cs[0usize];
    let s = cs[1usize];
    let neg_s = 0i64 - s;
    let neg_c = 0i64 - c;
    let q = n & 3i64;
    let out = s; // q == 3
    let out = select(q == 2i64, neg_c, out);
    let out = select(q == 1i64, neg_s, out);
    select(q == 0i64, c, out)
}

/// Elementwise `sin(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::sin`.
#[cube(launch)]
fn sin_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_sin(x_in[pos]);
    }
}

/// Elementwise `cos(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::cos`.
#[cube(launch)]
fn cos_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_cos(x_in[pos]);
    }
}

/// Run `Fixed::sin` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_sin(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        sin_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// Run `Fixed::cos` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_cos(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        cos_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// Integer square root of a `u64` (floor), bit-by-bit with no multiply (add/sub/compare/shift only),
/// 32 iterations. Matches `u64::isqrt`, so `Fixed::sqrt` over a `[0, 1]` radicand (the asin case, where
/// the radicand `(1 - x*x) << 32` is below 2^64) is reproduced bit-for-bit. Not a general `Fixed::sqrt`
/// (that needs a u128 radicand); this is the asin-domain sqrt.
#[cube]
fn isqrt_u64(radicand: u64) -> u64 {
    let mut n = radicand;
    let mut res = 0u64;
    let mut bit = 4611686018427387904u64; // 2^62, the largest 4^k below 2^64
    #[unroll]
    for _i in 0usize..32usize {
        let t = res + bit;
        let ge = n >= t;
        n = select(ge, n - t, n);
        res = select(ge, (res >> 1u32) + bit, res >> 1u32);
        bit >>= 2u32;
    }
    res
}

/// `asin(x)` in radians, the oracle's `Fixed::asin` = `atan(x / sqrt(1 - x*x))`, saturating to the
/// right angle outside `[-1, 1]` (the total-internal-reflection boundary).
#[cube]
fn fixed_asin(x: i64) -> i64 {
    let one = 4294967296i64;
    let half_pi = 6746518852i64;
    let neg_one = 0i64 - 4294967296i64; // from_int(-1)
                                        // denom = sqrt(1 - x^2); the radicand (1 - x^2) << 32 is below 2^64 for |x| < 1, x != 0.
    let d = one - q32_mul(x, x);
    let radicand = u64::cast_from(d) << 32u32;
    // When x^2 rounds to zero (x == 0, or |x| small enough that q32_mul(x, x) underflows), d == one
    // and the radicand is exactly 2^64, which overflows u64; the square root is one there. Otherwise
    // the radicand is below 2^64 and the u64 isqrt applies. Note the oracle does not special-case
    // x == 0: asin(0) is cordic_vectoring(one, 0), the small CORDIC residual, not exactly zero.
    let denom = select(d == one, one, i64::cast_from(isqrt_u64(radicand)));
    let v = cordic_vectoring(denom, x);
    let hi = select(x >= one, half_pi, v);
    select(x <= neg_one, 0i64 - half_pi, hi)
}

/// Elementwise `asin(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::asin`.
#[cube(launch)]
fn asin_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_asin(x_in[pos]);
    }
}

/// Run `Fixed::asin` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_asin(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        asin_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// `x^n` for an integer `n`, the oracle's `Fixed::powi` by exponentiation-by-squaring (a negative
/// exponent takes the reciprocal first). The 32-step squaring is manually unrolled straight-line (the
/// DSL rejects an accumulator carried across an `#[unroll]` loop that calls a `#[cube]` fn); squaring
/// past the top set bit of `|n|` is harmless because the corresponding bit does not touch `acc`.
/// Precondition: a zero base with a negative exponent divides by zero (the oracle panics; the GPU
/// yields a defined-but-meaningless value), the same divide-by-zero precondition as `q32_div`.
#[cube]
fn fixed_powi(x: i64, n: i32) -> i64 {
    let one = 4294967296i64;
    let e = u32::cast_from(select(n < 0i32, 0i32 - n, n)); // |n|
    let base = select(n < 0i32, q32_div(one, x), x);
    let acc = one;
    let acc = select((e & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 1u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 2u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 3u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 4u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 5u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 6u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 7u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 8u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 9u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 10u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 11u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 12u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 13u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 14u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 15u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 16u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 17u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 18u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 19u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 20u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 21u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 22u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 23u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 24u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 25u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 26u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 27u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 28u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 29u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    let acc = select(((e >> 30u32) & 1u32) == 1u32, q32_mul(acc, base), acc);
    let base = q32_mul(base, base);
    select(((e >> 31u32) & 1u32) == 1u32, q32_mul(acc, base), acc)
}

/// Elementwise integer power `x[i]^n[i]` on the GPU, bit-identical to `Fixed::powi` on CUDA. `x` and
/// `n` must have equal length.
#[cube(launch)]
fn powi_kernel(x_in: &Array<i64>, n_in: &Array<i32>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_powi(x_in[pos], n_in[pos]);
    }
}

/// Run `Fixed::powi` on the GPU: `out[i] = x[i]^n[i]`, bit-identical to `Fixed::powi` (CUDA). `x` and
/// `n` must have equal length.
pub fn gpu_powi(client: &CudaClient, x: &[i64], n: &[i32]) -> Vec<i64> {
    assert_eq!(x.len(), n.len(), "gpu_powi: mismatched input lengths");
    let count = x.len();
    if count == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let nin = client.create_from_slice(i32::as_bytes(n));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (count as u32).div_ceil(threads);
    unsafe {
        powi_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), count),
            ArrayArg::from_raw_parts(nin.clone(), count),
            ArrayArg::from_raw_parts(out.clone(), count),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}

/// `sqrt(x)`, the oracle's `Fixed::sqrt`: `isqrt((x as u128) << 32)` for `x > 0`, else zero. The
/// left shift by 32 zeros the radicand's low limb, so the radicand is `(0, x_lo, x_hi)` fed to the
/// 96-bit limb isqrt. General (any positive `Fixed`), unlike the asin-domain u64 isqrt.
#[cube]
fn fixed_sqrt(v: i64) -> i64 {
    let vlo = u32::cast_from(v);
    let vhi = u32::cast_from(v >> 32u32);
    let r = isqrt_u96(0u32, vlo, vhi);
    select(v <= 0i64, 0i64, r)
}

/// Elementwise `sqrt(x)` over `i64` Q32.32 bit patterns on the GPU, bit-identical to `Fixed::sqrt`.
#[cube(launch)]
fn sqrt_kernel(x_in: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = fixed_sqrt(x_in[pos]);
    }
}

/// Run `Fixed::sqrt` on the GPU over a slice of `i64` Q32.32 bit patterns (CUDA backend).
pub fn gpu_sqrt(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        sqrt_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(xin.clone(), n),
            ArrayArg::from_raw_parts(out.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out);
    i64::from_bytes(&bytes).to_vec()
}
