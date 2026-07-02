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

use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

use crate::stage0::CudaClient;

// The Q32.32 constants are the oracle's exact bit patterns (crates/core/src/fixed.rs), inlined as
// `let` literals inside each `#[cube]` function so they lift to DSL values.

// --- The limb primitives, bit-identical to Fixed::mul / Fixed::div ---

/// The pinned Q32.32 multiply on `i64` operands (bits [32, 96) of the exact signed 128-bit product,
/// floor). Same sign-magnitude u16-partial accumulation as the Stage 0 `emu_mul`, decomposed at the
/// i64 boundary with `cast_from`.
#[cube]
fn q32_mul(a: i64, b: i64) -> i64 {
    let alo = u32::cast_from(a);
    let ahi = u32::cast_from(a >> 32u32);
    let blo = u32::cast_from(b);
    let bhi = u32::cast_from(b >> 32u32);

    let a_neg = ahi >> 31u32;
    let b_neg = bhi >> 31u32;
    let neg = a_neg ^ b_neg;

    let na_lo = (!alo) + 1u32;
    let ca = select(alo == 0u32, 1u32, 0u32);
    let na_hi = (!ahi) + ca;
    let ma_lo = select(a_neg == 1u32, na_lo, alo);
    let ma_hi = select(a_neg == 1u32, na_hi, ahi);

    let nb_lo = (!blo) + 1u32;
    let cb = select(blo == 0u32, 1u32, 0u32);
    let nb_hi = (!bhi) + cb;
    let mb_lo = select(b_neg == 1u32, nb_lo, blo);
    let mb_hi = select(b_neg == 1u32, nb_hi, bhi);

    let mut aa = Array::<u32>::new(4usize);
    aa[0usize] = ma_lo & 0xFFFFu32;
    aa[1usize] = ma_lo >> 16u32;
    aa[2usize] = ma_hi & 0xFFFFu32;
    aa[3usize] = ma_hi >> 16u32;
    let mut bb = Array::<u32>::new(4usize);
    bb[0usize] = mb_lo & 0xFFFFu32;
    bb[1usize] = mb_lo >> 16u32;
    bb[2usize] = mb_hi & 0xFFFFu32;
    bb[3usize] = mb_hi >> 16u32;

    let mut acc = Array::<u32>::new(8usize);
    #[unroll]
    for i in 0usize..8usize {
        acc[i] = 0u32;
    }
    #[unroll]
    for i in 0usize..4usize {
        #[unroll]
        for j in 0usize..4usize {
            let p = aa[i] * bb[j];
            acc[i + j] = acc[i + j] + (p & 0xFFFFu32);
            acc[i + j + 1usize] = acc[i + j + 1usize] + (p >> 16u32);
        }
    }
    let mut carry = 0u32;
    #[unroll]
    for d in 0usize..8usize {
        let t = acc[d] + carry;
        acc[d] = t & 0xFFFFu32;
        carry = t >> 16u32;
    }
    let w0 = acc[0usize] | (acc[1usize] << 16u32);
    let w1 = acc[2usize] | (acc[3usize] << 16u32);
    let w2 = acc[4usize] | (acc[5usize] << 16u32);

    let v0 = !w0;
    let s0 = v0 + 1u32;
    let k0 = select(s0 < v0, 1u32, 0u32);
    let v1 = !w1;
    let s1 = v1 + k0;
    let k1 = select(s1 < v1, 1u32, 0u32);
    let v2 = !w2;
    let s2 = v2 + k1;

    let use_neg = neg == 1u32;
    let lo = select(use_neg, s1, w1);
    let hi = select(use_neg, s2, w2);
    (i64::cast_from(hi) << 32u32) | i64::cast_from(lo)
}

/// The pinned Q32.32 divide on `i64` operands (sign-magnitude 96-step restoring long division,
/// truncate toward zero). Same algorithm as the Stage 0 `emu_div`, decomposed at the i64 boundary.
/// The divisor must be non-zero (the oracle precondition).
#[cube]
fn q32_div(a: i64, b: i64) -> i64 {
    let alo = u32::cast_from(a);
    let ahi = u32::cast_from(a >> 32u32);
    let blo = u32::cast_from(b);
    let bhi = u32::cast_from(b >> 32u32);

    let a_neg = ahi >> 31u32;
    let b_neg = bhi >> 31u32;
    let neg = a_neg ^ b_neg;

    let na_lo = (!alo) + 1u32;
    let ca = select(alo == 0u32, 1u32, 0u32);
    let na_hi = (!ahi) + ca;
    let malo = select(a_neg == 1u32, na_lo, alo);
    let mahi = select(a_neg == 1u32, na_hi, ahi);

    let nb_lo = (!blo) + 1u32;
    let cb = select(blo == 0u32, 1u32, 0u32);
    let nb_hi = (!bhi) + cb;
    let mdlo = select(b_neg == 1u32, nb_lo, blo);
    let mdhi = select(b_neg == 1u32, nb_hi, bhi);

    let mut num = Array::<u32>::new(3usize);
    num[0usize] = 0u32;
    num[1usize] = malo;
    num[2usize] = mahi;
    let mut q = Array::<u32>::new(3usize);
    q[0usize] = 0u32;
    q[1usize] = 0u32;
    q[2usize] = 0u32;
    let mut r0 = 0u32;
    let mut r1 = 0u32;
    let mut r2 = 0u32;
    #[unroll]
    for step in 0usize..96usize {
        let ii = comptime!(95usize - step);
        let widx = comptime!(ii / 32usize);
        let sh = comptime!((ii % 32usize) as u32);
        let bit = (num[widx] >> sh) & 1u32;
        r2 = (r2 << 1u32) | (r1 >> 31u32);
        r1 = (r1 << 1u32) | (r0 >> 31u32);
        r0 = (r0 << 1u32) | bit;
        let r2nz = select(r2 != 0u32, 1u32, 0u32);
        let hi_gt = select(r1 > mdhi, 1u32, 0u32);
        let hi_eq = select(r1 == mdhi, 1u32, 0u32);
        let lo_ge = select(r0 >= mdlo, 1u32, 0u32);
        let ge = (r2nz | hi_gt | (hi_eq & lo_ge)) == 1u32;
        let borrow0 = select(r0 < mdlo, 1u32, 0u32);
        let sub_r0 = r0 - mdlo;
        let b1a = select(r1 < mdhi, 1u32, 0u32);
        let t1 = r1 - mdhi;
        let b1b = select(t1 < borrow0, 1u32, 0u32);
        let sub_r1 = t1 - borrow0;
        let sub_r2 = r2 - (b1a | b1b);
        r0 = select(ge, sub_r0, r0);
        r1 = select(ge, sub_r1, r1);
        r2 = select(ge, sub_r2, r2);
        q[widx] = q[widx] | select(ge, 1u32 << sh, 0u32);
    }
    let q0 = q[0usize];
    let q1 = q[1usize];
    let nq_lo = (!q0) + 1u32;
    let cq = select(q0 == 0u32, 1u32, 0u32);
    let nq_hi = (!q1) + cq;
    let use_neg = neg == 1u32;
    let out_lo = select(use_neg, nq_lo, q0);
    let out_hi = select(use_neg, nq_hi, q1);
    (i64::cast_from(out_hi) << 32u32) | i64::cast_from(out_lo)
}

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
pub fn gpu_exp(client: &CudaClient, x: &[i64]) -> Vec<i64> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let xin = client.create_from_slice(i64::as_bytes(x));
    let out = client.empty(core::mem::size_of_val(x));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        exp_kernel::launch::<CudaRuntime>(
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
