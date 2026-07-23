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

//! The shared Q32.32 i64-boundary limb primitives: the pinned multiply `q32_mul` and divide `q32_div`.
//! Both reproduce the `crates/core` `Fixed::mul`/`Fixed::div` oracle bit-for-bit by running the same
//! integer algorithm as the Stage 0 `emu_mul`/`emu_div` kernels, decomposed at the `i64` boundary with
//! `cast_from` and a signed `>> 32` rather than taking and returning `u32` limbs. Consolidated here so
//! the diffusion field ([`crate::field`]) and the transcendentals ([`crate::transcendental`]) share one
//! definition instead of carrying private copies. These use native `i64` at the boundary (the
//! per-kernel layout the proposal leaves to the author), so bit-identity is proven on the CUDA backend
//! and cross-vendor identity on a backend without native 64-bit is a Stage 0 gate matter. No float.
//!
//! Why the sign-magnitude limb product rather than the one-line `((a as i128) * (b as i128)) >> 32`
//! that `Fixed::mul` uses on the host: CubeCL has no 128-bit integer type. Its `IntKind` is
//! `I8`/`I16`/`I32`/`I64` only (verified in cubecl-ir 0.10 and on the current cubecl `main`), and `i128`
//! implements neither `CubePrimitive` nor `CubeType`, so `a as i128` inside a `#[cube]` kernel fails to
//! compile ("the trait bound `i128: CubePrimitive` is not satisfied"). This is a CubeCL plumbing gap, not
//! a hardware limit: NVRTC itself compiles `__int128` device code when passed `--device-int128` (the
//! direct Q32.32 multiply lowers to a `mul.wide` + `mul.lo` + carry chain in PTX), but CubeCL 0.10 has no
//! 128-bit kind in its IR, no frontend trait impls for it, and no 128-bit path in any of its three
//! codegen backends (and SPIR-V has no native 128-bit integer at all). Wiring i128 would mean forking
//! that dependency stack for no correctness gain, since the limb product already matches `Fixed::mul`
//! bit-for-bit and stays backend-general. The limb form is load-bearing, not a workaround to retire.

use cubecl::prelude::*;

/// The pinned Q32.32 multiply on `i64` operands: bits [32, 96) of the exact signed 128-bit product
/// (arithmetic floor + two's-complement narrow), matching `Fixed::mul`. Sign-magnitude u16-partial
/// digit accumulation (the product itself uses only u32 arithmetic, no i128), decomposing each `i64`
/// operand into `u32` limbs and recomposing the `i64` result. Proven bit-identical to `Fixed::mul`
/// over corners + a 1M sweep on CUDA (`gpu_fixed_mul` in `tests/stage0_gate`).
#[cube]
pub(crate) fn q32_mul(a: i64, b: i64) -> i64 {
    let alo = u32::cast_from(a);
    let ahi = u32::cast_from(a >> 32u32);
    let blo = u32::cast_from(b);
    let bhi = u32::cast_from(b >> 32u32);

    let a_neg = ahi >> 31u32;
    let b_neg = bhi >> 31u32;
    let neg = a_neg ^ b_neg;

    // magnitudes
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

    // 16-bit sub-limbs
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

    // 16 partials into 8 digit slots, then one normalization pass
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

    // 128-bit negate of words 0..2 when signs differ (enough for the [32, 96) result)
    let v0 = !w0;
    let s0 = v0 + 1u32;
    let k0 = select(s0 < v0, 1u32, 0u32);
    let v1 = !w1;
    let s1 = v1 + k0;
    let k1 = select(s1 < v1, 1u32, 0u32);
    let v2 = !w2;
    let s2 = v2 + k1;

    let use_neg = neg == 1u32;
    let lo = select(use_neg, s1, w1); // bits [32, 64)
    let hi = select(use_neg, s2, w2); // bits [64, 96)
    (i64::cast_from(hi) << 32u32) | i64::cast_from(lo)
}

/// The shared checked Q32.32 limb multiply behind [`fn@sat_mul`] and [`fn@checked_mul_zero`]: it returns a
/// 3-element array `[fit, overflow, use_neg]`. `fit` is the wrapped [32, 96) result (the correct value
/// when the product fits `i64`); `overflow` is 1 when the true Q32.32 product does not fit `i64`; and
/// `use_neg` is 1 when the result is negative. Same sign-magnitude limb product as [`fn@q32_mul`], but it
/// also keeps the top word `w3` (bits [96, 128) of the product magnitude, which `q32_mul` discards) so
/// an overflow of the [32, 96) result window is detectable: the Q32.32 magnitude is the 96-bit value
/// `(w3 : w2 : w1)`, and it fits iff `w3 == 0` and the sign bit is not forced by `w2`. No `i128`.
/// Consolidating the overflow decision here is a correctness guard: the negative-side boundary is subtle
/// (a blind audit caught a missing `w0 == 0` conjunct in it once), and both callers share this logic.
#[cube]
fn mul_checked(a: i64, b: i64) -> Array<i64> {
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
    let w1 = acc[2usize] | (acc[3usize] << 16u32); // bits [32, 64) of the magnitude
    let w2 = acc[4usize] | (acc[5usize] << 16u32); // bits [64, 96)
    let w3 = acc[6usize] | (acc[7usize] << 16u32); // bits [96, 128), overflow beyond the result window

    // The fitting narrowed result (two's complement of words 0..2 when signs differ), as in q32_mul.
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
    let fit = (i64::cast_from(hi) << 32u32) | i64::cast_from(lo);

    // Overflow: the shifted result is `product >> 32`, whose magnitude for a negative product is
    // ceil(|product| / 2^32), so the discarded low word `w0` counts. A positive result fits iff the full
    // magnitude <= 2^63 - 1, i.e. w3 == 0 and the top bit of w2 (bit 63) is clear. A negative result fits
    // iff the full magnitude <= 2^63; the sole extra value it admits is |product| == 2^95 EXACTLY (w3 ==
    // 0, w2 == 0x80000000, w1 == 0, AND w0 == 0), which shifts to -2^63 = i64::MIN and is already produced
    // by `fit`. Any nonzero w0 pushes the ceiling to 2^63 + 1 (below i64::MIN), so it must overflow: the
    // `w0 == 0` conjunct is required (a converged blind-audit finding, since without it a product like
    // (2^32 + 1) * -(2^63 - 2^31 + 1) returns i64::MAX instead of saturating to i64::MIN).
    let hibit = w2 >> 31u32;
    let at_min = (w2 == 0x80000000u32) && (w1 == 0u32) && (w0 == 0u32);
    let over_pos = (w3 != 0u32) || (hibit == 1u32);
    let over_neg = (w3 != 0u32) || (hibit == 1u32 && !at_min);
    let overflow = select(use_neg, over_neg, over_pos);

    let mut r = Array::<i64>::new(3usize);
    r[0usize] = fit;
    r[1usize] = select(overflow, 1i64, 0i64);
    r[2usize] = select(use_neg, 1i64, 0i64);
    r
}

/// Saturating Q32.32 multiply: the Fixed product when it fits `i64`, else the signed extreme (`i64::MIN`
/// on differing signs, `i64::MAX` on agreeing signs), matching the controller's `sat_mul`.
#[cube]
pub(crate) fn sat_mul(a: i64, b: i64) -> i64 {
    let r = mul_checked(a, b);
    let i64_max = 9223372036854775807i64;
    let i64_min = -9223372036854775807i64 - 1i64; // i64::MIN, split so the literal does not overflow
    let sat = select(r[2usize] == 1i64, i64_min, i64_max);
    select(r[1usize] == 1i64, sat, r[0usize])
}

/// Checked Q32.32 multiply returning ZERO on overflow rather than saturating, matching the physiology
/// path's `Fixed::checked_mul(...).unwrap_or(Fixed::ZERO)` (the homeostasis drain and the logistic
/// regen). Shares [`fn@mul_checked`]'s overflow decision with [`fn@sat_mul`], so the two cannot drift.
#[cube]
pub(crate) fn checked_mul_zero(a: i64, b: i64) -> i64 {
    let r = mul_checked(a, b);
    select(r[1usize] == 1i64, 0i64, r[0usize])
}

/// Saturating i64 add: `a + b`, saturating to the signed extreme on overflow rather than wrapping,
/// matching `Fixed::saturating_add`. Overflow occurs only when the operands share a sign and the wrapped
/// result's sign flips; the saturation direction is that shared sign.
#[cube]
pub(crate) fn sat_add(a: i64, b: i64) -> i64 {
    let s = a + b; // wrapping i64 add
    let same_sign = (a ^ b) >= 0i64;
    let flipped = (a ^ s) < 0i64;
    let overflow = same_sign && flipped;
    let i64_max = 9223372036854775807i64;
    let i64_min = -9223372036854775807i64 - 1i64;
    let sat = select(a < 0i64, i64_min, i64_max);
    select(overflow, sat, s)
}

/// The pinned Q32.32 divide on `i64` operands (sign-magnitude 96-step restoring long division,
/// truncate toward zero). Same algorithm as the Stage 0 `emu_div`, decomposed at the i64 boundary.
/// The divisor must be non-zero (the oracle precondition).
#[cube]
pub(crate) fn q32_div(a: i64, b: i64) -> i64 {
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

/// Integer square root (floor) of a 96-bit value held as three `u32` limbs (`n = n0 + n1*2^32 +
/// n2*2^64`), bit-by-bit with no multiply and no native 64-bit: the confined op set, so it is
/// backend-general like the Stage 0 arithmetic. Matches `u128::isqrt` (hence `Fixed::sqrt`, whose
/// radicand `(bits as u128) << 32` is below 2^95). The single trial bit sits at a compile-time
/// position each of the 48 unrolled steps, so `res + bit` and the limb it lands in are comptime.
#[cube]
pub(crate) fn isqrt_u96(n0: u32, n1: u32, n2: u32) -> i64 {
    let mut m0 = n0; // remaining radicand
    let mut m1 = n1;
    let mut m2 = n2;
    let mut r0 = 0u32; // result accumulator
    let mut r1 = 0u32;
    let mut r2 = 0u32;
    #[unroll]
    for step in 0usize..48usize {
        let pos = comptime!(94usize - 2usize * step); // trial bit position, 94 down to 0
        let bit0 = comptime!(if pos / 32usize == 0usize {
            1u32 << ((pos % 32usize) as u32)
        } else {
            0u32
        });
        let bit1 = comptime!(if pos / 32usize == 1usize {
            1u32 << ((pos % 32usize) as u32)
        } else {
            0u32
        });
        let bit2 = comptime!(if pos / 32usize == 2usize {
            1u32 << ((pos % 32usize) as u32)
        } else {
            0u32
        });

        // t = res + bit  (3-limb add; bit has a single set limb)
        let t0 = r0 + bit0;
        let tc0 = select(t0 < r0, 1u32, 0u32);
        let t1a = r1 + bit1;
        let tc1a = select(t1a < r1, 1u32, 0u32);
        let t1 = t1a + tc0;
        let tc1b = select(t1 < t1a, 1u32, 0u32);
        let tc1 = tc1a | tc1b;
        let t2 = r2 + bit2 + tc1;

        // ge = (m >= t) as an unsigned 96-bit compare
        let gt2 = select(m2 > t2, 1u32, 0u32);
        let eq2 = select(m2 == t2, 1u32, 0u32);
        let gt1 = select(m1 > t1, 1u32, 0u32);
        let eq1 = select(m1 == t1, 1u32, 0u32);
        let ge0 = select(m0 >= t0, 1u32, 0u32);
        let ge = (gt2 | (eq2 & (gt1 | (eq1 & ge0)))) == 1u32;

        // m - t  (3-limb subtract with borrow)
        let d0 = m0 - t0;
        let db0 = select(m0 < t0, 1u32, 0u32);
        let d1a = m1 - t1;
        let db1a = select(m1 < t1, 1u32, 0u32);
        let d1 = d1a - db0;
        let db1b = select(d1a < db0, 1u32, 0u32);
        let db1 = db1a | db1b;
        let d2 = m2 - t2 - db1;

        // res >> 1
        let sr0 = (r0 >> 1u32) | (r1 << 31u32);
        let sr1 = (r1 >> 1u32) | (r2 << 31u32);
        let sr2 = r2 >> 1u32;

        // (res >> 1) + bit
        let u0 = sr0 + bit0;
        let uc0 = select(u0 < sr0, 1u32, 0u32);
        let u1a = sr1 + bit1;
        let uc1a = select(u1a < sr1, 1u32, 0u32);
        let u1 = u1a + uc0;
        let uc1b = select(u1 < u1a, 1u32, 0u32);
        let uc1 = uc1a | uc1b;
        let u2 = sr2 + bit2 + uc1;

        m0 = select(ge, d0, m0);
        m1 = select(ge, d1, m1);
        m2 = select(ge, d2, m2);
        r0 = select(ge, u0, sr0);
        r1 = select(ge, u1, sr1);
        r2 = select(ge, u2, sr2);
    }
    // the result is below 2^48, so its bits sit in r0 (low 32) and r1 (next 16); r2 is zero.
    (i64::cast_from(r1) << 32u32) | i64::cast_from(r0)
}
