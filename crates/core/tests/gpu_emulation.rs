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

//! The Stage 0 emulation check for R-GPU-CANON-PIN, in software. A canonical GPU kernel has no native
//! 64-by-64-to-128 multiply and no i128, so the proposal pins a 32-bit-limb sign-magnitude emulation.
//! This test implements that emulation using ONLY the confined op set a shader has (u32 wrapping add
//! and subtract, u16-by-u16-to-u32 multiply, bitwise ops, shifts by a constant below 32, and u32
//! comparisons; no i64 or i128 multiply anywhere in `emu_mul`), and proves it is bit-identical to the
//! `Fixed::mul` oracle over a random-and-corner sweep. Bit-identity of this integer emulation to the
//! oracle is the hard, backend-general half of the Stage 0 gate; a device run on a specific vendor is
//! then a confirmation, since integer arithmetic is exact and lane-order-independent. When a GPU
//! toolchain is available (a CUDA toolkit, a glsl-to-SPIR-V compiler, or wgpu), the identical limb
//! algorithm ports to the shader and is gated the same way against this oracle.

use civsim_core::Fixed;

// --- The GPU-op-set emulation of Fixed::mul, from 32-bit limbs (no i64/i128 multiply) ---

fn split(x: i64) -> (u32, u32) {
    let u = x as u64;
    (u as u32, (u >> 32) as u32)
}
fn join(lo: u32, hi: u32) -> i64 {
    (((hi as u64) << 32) | lo as u64) as i64
}
fn is_neg(hi: u32) -> bool {
    hi & 0x8000_0000 != 0
}

/// 64-bit two's-complement negation over (lo, hi) limbs, u32 ops only. Maps the i64::MIN pattern to
/// 2^63 exactly (the corner with no arithmetic negation).
fn neg64(lo: u32, hi: u32) -> (u32, u32) {
    let nlo = (!lo).wrapping_add(1);
    let carry = if nlo == 0 { 1 } else { 0 };
    let nhi = (!hi).wrapping_add(carry);
    (nlo, nhi)
}

/// Unsigned 64-by-64-to-128 product as four u32 words, schoolbook over 16-bit limbs so every partial
/// product `u16 * u16` fits a u32 and no high-half or 32-bit-overflow behaviour is relied on.
fn umul_64_128(alo: u32, ahi: u32, blo: u32, bhi: u32) -> [u32; 4] {
    let a = [alo & 0xFFFF, alo >> 16, ahi & 0xFFFF, ahi >> 16];
    let b = [blo & 0xFFFF, blo >> 16, bhi & 0xFFFF, bhi >> 16];
    let mut acc = [0u32; 8]; // eight normalized 16-bit digits
    for i in 0..4 {
        let mut carry = 0u32;
        for j in 0..4 {
            // a[i]*b[j] <= (2^16-1)^2; + acc digit (<=2^16-1) + carry (<=2^16-1) still fits u32.
            let t = a[i] * b[j] + acc[i + j] + carry;
            acc[i + j] = t & 0xFFFF;
            carry = t >> 16;
        }
        // propagate the row's final carry into the higher digits (never past digit 7).
        let mut k = i + 4;
        while carry > 0 {
            let t = acc[k] + carry;
            acc[k] = t & 0xFFFF;
            carry = t >> 16;
            k += 1;
        }
    }
    [
        acc[0] | (acc[1] << 16),
        acc[2] | (acc[3] << 16),
        acc[4] | (acc[5] << 16),
        acc[6] | (acc[7] << 16),
    ]
}

/// 128-bit two's-complement negation over four u32 words, u32 ops only (add-with-carry by comparison).
fn neg128(w: [u32; 4]) -> [u32; 4] {
    let mut r = [0u32; 4];
    let mut carry = 1u32;
    for k in 0..4 {
        let v = !w[k];
        let s = v.wrapping_add(carry);
        // carry out of adding carry in {0,1}: the sum wrapped below v only on overflow.
        carry = if s < v { 1 } else { 0 };
        r[k] = s;
    }
    r
}

/// The pinned emulation of `Fixed::mul`: sign-magnitude, unsigned limb product, negate the full
/// 128-bit product when signs differ, then take bits [32, 96) (the middle two words). This is
/// `floor(a*b / 2^32) mod 2^64` interpreted as i64, matching the oracle's arithmetic-shift floor and
/// two's-complement narrowing.
fn emu_mul(a: i64, b: i64) -> i64 {
    let (alo, ahi) = split(a);
    let (blo, bhi) = split(b);
    let neg = is_neg(ahi) ^ is_neg(bhi);
    let (malo, mahi) = if is_neg(ahi) { neg64(alo, ahi) } else { (alo, ahi) };
    let (mblo, mbhi) = if is_neg(bhi) { neg64(blo, bhi) } else { (blo, bhi) };
    let prod = umul_64_128(malo, mahi, mblo, mbhi);
    let signed = if neg { neg128(prod) } else { prod };
    join(signed[1], signed[2])
}

/// The pinned emulation of `Fixed::div`: sign-magnitude restoring long division of `(|a| << 32)` by
/// `|b|` over 96 bits, taking the low 64 quotient bits and applying the sign, matching the oracle's
/// truncation toward zero. u32 ops only (wrapping subtract, comparison, shifts); no i64/i128 divide.
fn emu_div(a: i64, b: i64) -> i64 {
    let (alo, ahi) = split(a);
    let (blo, bhi) = split(b);
    let neg = is_neg(ahi) ^ is_neg(bhi);
    let (malo, mahi) = if is_neg(ahi) { neg64(alo, ahi) } else { (alo, ahi) };
    let (mdlo, mdhi) = if is_neg(bhi) { neg64(blo, bhi) } else { (blo, bhi) };
    // numerator magnitude = |a| << 32, little-endian 96-bit words.
    let num = [0u32, malo, mahi];
    let (mut r0, mut r1, mut r2) = (0u32, 0u32, 0u32); // 65-bit remainder
    let (mut q0, mut q1) = (0u32, 0u32); // low 64 quotient bits (the oracle narrows to i64)
    let mut i = 95i32;
    while i >= 0 {
        let bit = (num[(i / 32) as usize] >> (i % 32) as u32) & 1;
        r2 = (r2 << 1) | (r1 >> 31);
        r1 = (r1 << 1) | (r0 >> 31);
        r0 = (r0 << 1) | bit;
        let ge = r2 != 0 || r1 > mdhi || (r1 == mdhi && r0 >= mdlo);
        if ge {
            let borrow0 = (r0 < mdlo) as u32;
            r0 = r0.wrapping_sub(mdlo);
            let borrow1a = (r1 < mdhi) as u32;
            let t1 = r1.wrapping_sub(mdhi);
            let borrow1b = (t1 < borrow0) as u32;
            r1 = t1.wrapping_sub(borrow0);
            r2 = r2.wrapping_sub(borrow1a | borrow1b);
            if i < 32 {
                q0 |= 1u32 << i;
            } else if i < 64 {
                q1 |= 1u32 << (i - 32);
            }
        }
        i -= 1;
    }
    if neg {
        let (nlo, nhi) = neg64(q0, q1);
        join(nlo, nhi)
    } else {
        join(q0, q1)
    }
}

// --- The Stage 0 oracle-match gate (in software) ---

fn corners() -> Vec<i64> {
    vec![
        0,
        1,
        -1,
        2,
        -2,
        i64::MAX,
        i64::MIN,
        i64::MIN + 1,
        i64::MAX - 1,
        1 << 32,        // 1.0
        -(1 << 32),     // -1.0
        (1 << 32) + 1,
        1 << 31,        // 0.5
        -(1 << 31),
        (1i64 << 62),
        -(1i64 << 62),
        0x0000_0001_FFFF_FFFF,
        -0x0000_0001_0000_0000,
    ]
}

#[test]
fn the_limb_multiply_emulation_is_bit_identical_to_the_oracle() {
    let mut mismatches = 0u64;
    let mut checked = 0u64;
    // Every corner-by-corner pair.
    let cs = corners();
    for &a in &cs {
        for &b in &cs {
            checked += 1;
            let want = Fixed::from_bits(a).mul(Fixed::from_bits(b)).to_bits();
            let got = emu_mul(a, b);
            if got != want {
                mismatches += 1;
                if mismatches <= 5 {
                    eprintln!("mismatch: a={a:#x} b={b:#x} got={got:#x} want={want:#x}");
                }
            }
        }
    }
    // A deterministic pseudo-random sweep (xorshift64, fixed seed) across the full i64 range.
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s as i64
    };
    for _ in 0..200_000 {
        let a = next();
        let b = next();
        checked += 1;
        let want = Fixed::from_bits(a).mul(Fixed::from_bits(b)).to_bits();
        let got = emu_mul(a, b);
        if got != want {
            mismatches += 1;
            if mismatches <= 5 {
                eprintln!("mismatch: a={a:#x} b={b:#x} got={got:#x} want={want:#x}");
            }
        }
    }
    assert_eq!(
        mismatches, 0,
        "Stage 0: the u32-limb emulation must match Fixed::mul bit-for-bit over all {checked} cases"
    );
}

#[test]
fn the_limb_divide_emulation_is_bit_identical_to_the_oracle() {
    let mut mismatches = 0u64;
    let mut checked = 0u64;
    let cs = corners();
    let check = |a: i64, b: i64, mism: &mut u64| {
        if b == 0 {
            return;
        }
        let want = Fixed::from_bits(a).div(Fixed::from_bits(b)).to_bits();
        let got = emu_div(a, b);
        if got != want {
            *mism += 1;
            if *mism <= 5 {
                eprintln!("div mismatch: a={a:#x} b={b:#x} got={got:#x} want={want:#x}");
            }
        }
    };
    for &a in &cs {
        for &b in &cs {
            checked += 1;
            check(a, b, &mut mismatches);
        }
    }
    let mut s: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s as i64
    };
    for _ in 0..200_000 {
        let a = next();
        let b = next();
        checked += 1;
        check(a, b, &mut mismatches);
    }
    assert_eq!(
        mismatches, 0,
        "Stage 0: the u32-limb emulation must match Fixed::div bit-for-bit over {checked} cases"
    );
}

#[test]
fn the_named_multiply_corners_match() {
    // The specific corners the proposal calls out.
    assert_eq!(emu_mul(i64::MIN, i64::MIN), Fixed::from_bits(i64::MIN).mul(Fixed::from_bits(i64::MIN)).to_bits());
    assert_eq!(emu_mul(i64::MIN, i64::MIN), 0, "mul(MIN, MIN) = 0");
    assert_eq!(emu_mul(i64::MIN, -1), 1i64 << 31, "mul(MIN, -1 bits) = 2^31");
    // 1.5 * 2.0 = 3.0 in Q32.32.
    assert_eq!(emu_mul(3 << 31, 2 << 32), 3 << 32, "1.5 * 2.0 = 3.0");
}
