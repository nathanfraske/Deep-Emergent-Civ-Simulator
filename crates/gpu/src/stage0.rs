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

//! The Stage 0 pinned arithmetic (R-GPU-CANON-PIN), as CubeCL `#[cube]` kernels. These are the GPU
//! side of the contract the CPU oracle in `crates/core/src/fixed.rs` defines and the portable
//! software gate in `crates/core/tests/gpu_emulation.rs` proves: the pinned Q32.32 sign-magnitude
//! limb multiply (bits [32, 96) of the exact 128-bit product, arithmetic floor + two's-complement
//! narrow, matching `Fixed::mul`) and the sign-magnitude 96-step restoring divide (truncate toward
//! zero, matching `Fixed::div`).
//!
//! The kernels are confined to an op set with identical defined semantics on every CubeCL backend:
//! `u32` wrapping add and subtract, `u32 * u32` where both inputs are below 2^16 (so the product is
//! below 2^32 and no overflow behaviour is relied on), bitwise AND/OR/NOT/XOR, shifts by a
//! compile-time-constant amount strictly below 32, `u32` comparison, and the branchless `select`. No
//! native 64-bit type, no native divide or modulo, and no float appear anywhere on this path. Inputs
//! and outputs cross the boundary as the low and high `u32` limbs of the `i64` Fixed bit patterns.
//!
//! Bit-identity to the oracle follows from the unique-result argument: each kernel computes an exact
//! integer product or quotient (one correct mathematical value), rounds exactly once at the pinned
//! step, and wraps exactly once at the pinned narrowing, so any correct implementation over the
//! confined op set agrees bit-for-bit. The device gate (`tests/stage0_gate.rs`) is the empirical
//! confirmation and the guard against an implementation bug in the emulation.

use cubecl::cpu::{CpuDevice, CpuRuntime};
use cubecl::cuda::{CudaDevice, CudaRuntime};
use cubecl::prelude::*;

/// The concrete CUDA compute client type used by the launchers in this crate.
pub type CudaClient = ComputeClient<CudaRuntime>;

/// A CUDA compute client for the default device (index 0). Assumes a working CUDA device and NVRTC;
/// call only when a device is present (the gate tests self-skip unless `CIVSIM_GPU` is set).
pub fn cuda_client() -> CudaClient {
    CudaRuntime::client(&CudaDevice::default())
}

/// The concrete CubeCL CPU-backend compute client type.
pub type CpuClient = ComputeClient<CpuRuntime>;

/// A CubeCL CPU-backend compute client. It runs the same `#[cube]` kernels through a completely
/// independent codegen path (MLIR/LLVM, no GPU), so agreement between this and the CUDA backend is
/// cross-backend bit-identity evidence (the multi-vendor Stage 0 residual). Needs no device.
pub fn cpu_client() -> CpuClient {
    CpuRuntime::client(&CpuDevice)
}

/// The pinned Q32.32 multiply, `emu_mul` (see `crates/core/tests/gpu_emulation.rs`) as a `#[cube]`
/// kernel. Sign-magnitude, unsigned 64x64->128 schoolbook product over 16-bit sub-limbs, negate the
/// 128-bit product when the signs differ, then keep bits [32, 96). The 16 partial products are
/// accumulated into eight 16-bit digit slots and normalized once (a fully unrolled, data-independent
/// schedule with no dynamic index): the same exact product as the oracle, so bit-identical.
#[cube(launch)]
fn emu_mul_kernel(
    a_lo: &Array<u32>,
    a_hi: &Array<u32>,
    b_lo: &Array<u32>,
    b_hi: &Array<u32>,
    out_lo: &mut Array<u32>,
    out_hi: &mut Array<u32>,
) {
    let pos = ABSOLUTE_POS;
    if pos < out_lo.len() {
        let alo0 = a_lo[pos];
        let ahi0 = a_hi[pos];
        let blo0 = b_lo[pos];
        let bhi0 = b_hi[pos];

        let a_neg = ahi0 >> 31u32; // 0 or 1
        let b_neg = bhi0 >> 31u32;
        let neg = a_neg ^ b_neg;

        // magnitudes: branchless two's-complement negate + select
        let na_lo = (!alo0) + 1u32;
        let ca = select(alo0 == 0u32, 1u32, 0u32);
        let na_hi = (!ahi0) + ca;
        let ma_lo = select(a_neg == 1u32, na_lo, alo0);
        let ma_hi = select(a_neg == 1u32, na_hi, ahi0);

        let nb_lo = (!blo0) + 1u32;
        let cb = select(blo0 == 0u32, 1u32, 0u32);
        let nb_hi = (!bhi0) + cb;
        let mb_lo = select(b_neg == 1u32, nb_lo, blo0);
        let mb_hi = select(b_neg == 1u32, nb_hi, bhi0);

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

        // accumulate the 16 partial products into 8 16-bit digit slots (each u32 slot holds the
        // pre-normalized sum without overflow: <= 8 * (2^16 - 1) < 2^20).
        let mut acc = Array::<u32>::new(8usize);
        #[unroll]
        for i in 0usize..8usize {
            acc[i] = 0u32;
        }
        #[unroll]
        for i in 0usize..4usize {
            #[unroll]
            for j in 0usize..4usize {
                let p = aa[i] * bb[j]; // < 2^32
                acc[i + j] = acc[i + j] + (p & 0xFFFFu32);
                acc[i + j + 1usize] = acc[i + j + 1usize] + (p >> 16u32);
            }
        }
        // single normalization pass to clean 16-bit digits
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

        // 128-bit two's-complement negate of words 0..2 (enough for the [32, 96) output).
        let v0 = !w0;
        let s0 = v0 + 1u32;
        let k0 = select(s0 < v0, 1u32, 0u32);
        let v1 = !w1;
        let s1 = v1 + k0;
        let k1 = select(s1 < v1, 1u32, 0u32);
        let v2 = !w2;
        let s2 = v2 + k1;

        let use_neg = neg == 1u32;
        out_lo[pos] = select(use_neg, s1, w1); // bits [32, 64)
        out_hi[pos] = select(use_neg, s2, w2); // bits [64, 96)
    }
}

/// The pinned Q32.32 divide, `emu_div` (see `crates/core/tests/gpu_emulation.rs`) as a `#[cube]`
/// kernel. Sign-magnitude restoring long division of the numerator magnitude `|a| << 32` (a 96-bit
/// value) by the divisor magnitude `|b|`, MSB-first over a fixed 96 steps, keeping the low 64
/// quotient bits and applying the sign, which gives truncation toward zero overall (the oracle). The
/// per-step subtract is branchless (computed then `select`ed on the remainder-greater-or-equal test).
/// Divide by zero is a caller precondition (mirroring the CPU `Fixed::div` panic); the gate feeds only
/// non-zero divisors.
#[cube(launch)]
fn emu_div_kernel(
    a_lo: &Array<u32>,
    a_hi: &Array<u32>,
    b_lo: &Array<u32>,
    b_hi: &Array<u32>,
    out_lo: &mut Array<u32>,
    out_hi: &mut Array<u32>,
) {
    let pos = ABSOLUTE_POS;
    if pos < out_lo.len() {
        let alo0 = a_lo[pos];
        let ahi0 = a_hi[pos];
        let blo0 = b_lo[pos];
        let bhi0 = b_hi[pos];

        let a_neg = ahi0 >> 31u32;
        let b_neg = bhi0 >> 31u32;
        let neg = a_neg ^ b_neg;

        // numerator magnitude (limbs malo, mahi)
        let na_lo = (!alo0) + 1u32;
        let ca = select(alo0 == 0u32, 1u32, 0u32);
        let na_hi = (!ahi0) + ca;
        let malo = select(a_neg == 1u32, na_lo, alo0);
        let mahi = select(a_neg == 1u32, na_hi, ahi0);

        // divisor magnitude (limbs mdlo, mdhi)
        let nb_lo = (!blo0) + 1u32;
        let cb = select(blo0 == 0u32, 1u32, 0u32);
        let nb_hi = (!bhi0) + cb;
        let mdlo = select(b_neg == 1u32, nb_lo, blo0);
        let mdhi = select(b_neg == 1u32, nb_hi, bhi0);

        // Numerator = |a| << 32, little-endian words [0, malo, mahi]; low word all zero. The quotient
        // bit at position ii lands in the same word and offset as the numerator bit, so one comptime
        // word index and shift serve both. q has three words: q[2] catches bits >= 64 and is
        // discarded (the oracle keeps the low 64 quotient bits). No branch: only runtime `select`s.
        let mut num = Array::<u32>::new(3usize);
        num[0usize] = 0u32;
        num[1usize] = malo;
        num[2usize] = mahi;
        let mut q = Array::<u32>::new(3usize);
        q[0usize] = 0u32;
        q[1usize] = 0u32;
        q[2usize] = 0u32;
        let mut r0 = 0u32; // 65-bit running remainder (r0, r1, r2)
        let mut r1 = 0u32;
        let mut r2 = 0u32;
        #[unroll]
        for step in 0usize..96usize {
            let ii = comptime!(95usize - step); // MSB-first numerator bit index
            let widx = comptime!(ii / 32usize);
            let sh = comptime!((ii % 32usize) as u32);
            let bit = (num[widx] >> sh) & 1u32;
            // shift the remainder left by one and bring in the numerator bit
            r2 = (r2 << 1u32) | (r1 >> 31u32);
            r1 = (r1 << 1u32) | (r0 >> 31u32);
            r0 = (r0 << 1u32) | bit;
            // ge = (r2, r1, r0) >= (0, mdhi, mdlo), computed as u32 flags (no bool logic ops)
            let r2nz = select(r2 != 0u32, 1u32, 0u32);
            let hi_gt = select(r1 > mdhi, 1u32, 0u32);
            let hi_eq = select(r1 == mdhi, 1u32, 0u32);
            let lo_ge = select(r0 >= mdlo, 1u32, 0u32);
            let ge = (r2nz | hi_gt | (hi_eq & lo_ge)) == 1u32;
            // branchless conditional subtract of the divisor magnitude
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
            // set the quotient bit when ge (q[2] holds discarded bits >= 64)
            q[widx] = q[widx] | select(ge, 1u32 << sh, 0u32);
        }
        let q0 = q[0usize];
        let q1 = q[1usize];
        // apply the sign to the low-64 quotient (two's-complement negate when signs differ)
        let nq_lo = (!q0) + 1u32;
        let cq = select(q0 == 0u32, 1u32, 0u32);
        let nq_hi = (!q1) + cq;
        let use_neg = neg == 1u32;
        out_lo[pos] = select(use_neg, nq_lo, q0);
        out_hi[pos] = select(use_neg, nq_hi, q1);
    }
}

/// Split a slice of `i64` Fixed bit patterns into low and high `u32` limbs.
fn split(v: &[i64]) -> (Vec<u32>, Vec<u32>) {
    let lo = v.iter().map(|&x| x as u64 as u32).collect();
    let hi = v.iter().map(|&x| ((x as u64) >> 32) as u32).collect();
    (lo, hi)
}

/// Rejoin low and high `u32` limbs into `i64` Fixed bit patterns.
fn join(lo: &[u32], hi: &[u32]) -> Vec<i64> {
    lo.iter()
        .zip(hi.iter())
        .map(|(&l, &h)| (((h as u64) << 32) | l as u64) as i64)
        .collect()
}

/// Elementwise pinned Q32.32 multiply on the GPU: `out[i]` = Fixed::mul bits of `a[i]` and `b[i]`,
/// with `a`, `b`, and the result carried as raw `i64` Fixed bit patterns. Bit-identical to
/// `Fixed::mul` (the Stage 0 contract). `a` and `b` must have equal length.
pub fn gpu_mul<R: Runtime>(client: &ComputeClient<R>, a: &[i64], b: &[i64]) -> Vec<i64> {
    assert_eq!(a.len(), b.len(), "gpu_mul: mismatched input lengths");
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let (a_lo, a_hi) = split(a);
    let (b_lo, b_hi) = split(b);
    let a_lo_h = client.create_from_slice(u32::as_bytes(&a_lo));
    let a_hi_h = client.create_from_slice(u32::as_bytes(&a_hi));
    let b_lo_h = client.create_from_slice(u32::as_bytes(&b_lo));
    let b_hi_h = client.create_from_slice(u32::as_bytes(&b_hi));
    let out_lo_h = client.empty(n * core::mem::size_of::<u32>());
    let out_hi_h = client.empty(n * core::mem::size_of::<u32>());

    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        emu_mul_kernel::launch::<R>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(a_lo_h.clone(), n),
            ArrayArg::from_raw_parts(a_hi_h.clone(), n),
            ArrayArg::from_raw_parts(b_lo_h.clone(), n),
            ArrayArg::from_raw_parts(b_hi_h.clone(), n),
            ArrayArg::from_raw_parts(out_lo_h.clone(), n),
            ArrayArg::from_raw_parts(out_hi_h.clone(), n),
        );
    }
    let lo = client.read_one_unchecked(out_lo_h);
    let hi = client.read_one_unchecked(out_hi_h);
    join(u32::from_bytes(&lo), u32::from_bytes(&hi))
}

/// Elementwise pinned Q32.32 divide on the GPU: `out[i]` = Fixed::div bits of `a[i]` by `b[i]`,
/// carried as raw `i64` Fixed bit patterns. Bit-identical to `Fixed::div`. Every `b[i]` must be
/// non-zero (the divide-by-zero precondition mirrors `Fixed::div`). `a` and `b` must have equal length.
pub fn gpu_div<R: Runtime>(client: &ComputeClient<R>, a: &[i64], b: &[i64]) -> Vec<i64> {
    assert_eq!(a.len(), b.len(), "gpu_div: mismatched input lengths");
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let (a_lo, a_hi) = split(a);
    let (b_lo, b_hi) = split(b);
    let a_lo_h = client.create_from_slice(u32::as_bytes(&a_lo));
    let a_hi_h = client.create_from_slice(u32::as_bytes(&a_hi));
    let b_lo_h = client.create_from_slice(u32::as_bytes(&b_lo));
    let b_hi_h = client.create_from_slice(u32::as_bytes(&b_hi));
    let out_lo_h = client.empty(n * core::mem::size_of::<u32>());
    let out_hi_h = client.empty(n * core::mem::size_of::<u32>());

    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        emu_div_kernel::launch::<R>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(a_lo_h.clone(), n),
            ArrayArg::from_raw_parts(a_hi_h.clone(), n),
            ArrayArg::from_raw_parts(b_lo_h.clone(), n),
            ArrayArg::from_raw_parts(b_hi_h.clone(), n),
            ArrayArg::from_raw_parts(out_lo_h.clone(), n),
            ArrayArg::from_raw_parts(out_hi_h.clone(), n),
        );
    }
    let lo = client.read_one_unchecked(out_lo_h);
    let hi = client.read_one_unchecked(out_hi_h);
    join(u32::from_bytes(&lo), u32::from_bytes(&hi))
}
