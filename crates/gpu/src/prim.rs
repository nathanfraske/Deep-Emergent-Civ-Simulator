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

use cubecl::prelude::*;

/// The oracle's `splitmix64` counter mixer (`crates/core/src/rng.rs`), on native `u64`. CUDA `u64` add
/// and multiply wrap, matching the oracle's `wrapping_add`/`wrapping_mul`. Shared by the draw-keyed GPU
/// consumers (the perceive notice roll and the worldgen noise fold) so there is one definition.
#[cube]
pub(crate) fn splitmix64(x: u64) -> u64 {
    let mut z = x + 0x9E3779B97F4A7C15u64; // the SplitMix64 golden gamma
    z = (z ^ (z >> 30u32)) * 0xBF58476D1CE4E5B9u64;
    z = (z ^ (z >> 27u32)) * 0x94D049BB133111EBu64;
    z ^ (z >> 31u32)
}

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
