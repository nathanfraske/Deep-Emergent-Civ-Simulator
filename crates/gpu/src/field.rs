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

//! The canonical fixed-point diffusion field kernel (design Part 5.5), the CubeCL counterpart of the
//! CPU `diffusion_bench` (`crates/core/examples`) and `civsim_sim`'s `Field::step`. It is the first
//! real physics field workload on the GPU: the per-cell op sequence is fixed (a toroidal five-point
//! Jacobi stencil), the neighbour combine is exact integer addition, and the single coefficient
//! multiply is the pinned Q32.32 multiply (the same floor-rounded result the Stage 0 gate pins). It is
//! proven bit-identical to the CPU `Fixed` reference on the CUDA backend (`tests/diffusion_gate`), and
//! workgroup- and tile-size independent by construction (no reduction, no transcendental).
//!
//! Op-set note (the honest boundary). Unlike [`crate::stage0`], which stays inside the u32-limb
//! confined op set and is therefore backend-general by the unique-result argument, this kernel holds
//! field values as native `i64` and uses native i64 add and sub, a signed `<< 2` for the `4*c` term,
//! and the i64<->u32 limb split (via `cast_from` and a signed `>> 32`) inside `q32_mul`. That is the
//! per-kernel layout choice the proposal leaves to the kernel author (native 64-bit where the backend
//! has it), not part of the u32-only set the Stage 0 gate proves across vendors. So CUDA bit-identity
//! is proven here; cross-vendor bit-identity on a backend without native 64-bit (base WGSL) is a Stage
//! 0 gate matter for that backend rather than something the unique-result argument already implies. The
//! `4*c` shift equals `Fixed::from_int(4).mul(cur)` for both signs, so the only rounding is the single
//! floor inside `q32_mul`.
//!
//! Two preconditions match the CPU only inside the intended regime: the neighbour sum uses wrapping
//! i64 add, so a field grown past the i64 range would wrap on the GPU where the CPU `Fixed` add panics
//! (a stable diffusion coefficient keeps a bounded field far from this); and the linear index is a
//! `u32` product, so a grid with `width * height >= 2^32` would wrap the index.

use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

use crate::stage0::CudaClient;

/// The pinned Q32.32 multiply on `i64` operands: bits [32, 96) of the exact signed 128-bit product
/// (arithmetic floor + two's-complement narrow), matching `Fixed::mul`. The core is the same
/// sign-magnitude u16-partial digit accumulation as the Stage 0 `emu_mul` kernel (no i128, and the
/// product itself uses only u32 arithmetic), differing only at the boundary: it decomposes each `i64`
/// operand into `u32` limbs with `cast_from` and a signed `>> 32`, and recomposes the `i64` result,
/// rather than taking and returning the limbs as the Stage 0 kernel does. Proven bit-identical to
/// `Fixed::mul` over corners + a 1M sweep on CUDA (`gpu_fixed_mul` in `tests/stage0_gate`).
#[cube]
fn q32_mul(a: i64, b: i64) -> i64 {
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

/// Elementwise wrapper over `q32_mul`, so the diffusion kernel's coefficient multiply can be gated
/// over the full corner and sweep range against the oracle (not only over the narrow field values a
/// diffusion run visits). See [`gpu_fixed_mul`].
#[cube(launch)]
fn q32_mul_kernel(a: &Array<i64>, b: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = q32_mul(a[pos], b[pos]);
    }
}

/// Elementwise pinned Q32.32 multiply using the diffusion kernel's `q32_mul` (the i64-boundary form,
/// distinct from [`crate::stage0::gpu_mul`]'s u32-limb form). `a`, `b`, and the result are raw `i64`
/// Fixed bit patterns; bit-identical to `Fixed::mul` on CUDA. Exists so the field multiply is proven
/// over the same corner + sweep range as the Stage 0 multiply. `a` and `b` must have equal length.
pub fn gpu_fixed_mul(client: &CudaClient, a: &[i64], b: &[i64]) -> Vec<i64> {
    assert_eq!(a.len(), b.len(), "gpu_fixed_mul: mismatched input lengths");
    let n = a.len();
    if n == 0 {
        return Vec::new();
    }
    let a_h = client.create_from_slice(i64::as_bytes(a));
    let b_h = client.create_from_slice(i64::as_bytes(b));
    let out_h = client.empty(core::mem::size_of_val(a));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        q32_mul_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(a_h.clone(), n),
            ArrayArg::from_raw_parts(b_h.clone(), n),
            ArrayArg::from_raw_parts(out_h.clone(), n),
        );
    }
    let bytes = client.read_one_unchecked(out_h);
    i64::from_bytes(&bytes).to_vec()
}

/// One canonical diffusion step over a toroidal (wrap-around) grid: `g = c + k * lap`, where
/// `lap = up + dn + lf + rt - 4*c`. Neighbour indices wrap by `select` (no modulo), the `4*c` is a
/// shift, and the coefficient multiply is the pinned `q32_mul`. Field values are `i64` Q32.32 bits.
#[cube(launch)]
fn diffuse_kernel(f: &Array<i64>, g: &mut Array<i64>, width: u32, height: u32, k: i64) {
    let x = ABSOLUTE_POS_X;
    let y = ABSOLUTE_POS_Y;
    if x < width && y < height {
        let xl = select(x == 0u32, width - 1u32, x - 1u32);
        let xr = select(x == width - 1u32, 0u32, x + 1u32);
        let yu = select(y == 0u32, height - 1u32, y - 1u32);
        let yd = select(y == height - 1u32, 0u32, y + 1u32);

        let idx = (y * width + x) as usize;
        let c = f[idx];
        let up = f[(yu * width + x) as usize];
        let dn = f[(yd * width + x) as usize];
        let lf = f[(y * width + xl) as usize];
        let rt = f[(y * width + xr) as usize];

        let lap = up + dn + lf + rt - (c << 2u32); // 4*c is an exact shift on the bit pattern
        g[idx] = c + q32_mul(k, lap);
    }
}

/// Run `iters` canonical diffusion steps on the GPU over a `width` x `height` toroidal field, starting
/// from `initial` (row-major `i64` Q32.32 bit patterns), with diffusion coefficient `k` (Q32.32 bits).
/// Returns the final field. Bit-identical to the CPU reference stencil (the Part 5.5 determinism
/// contract). `initial.len()` must equal `width * height`.
pub fn gpu_diffuse(
    client: &CudaClient,
    initial: &[i64],
    width: u32,
    height: u32,
    iters: u32,
    k: i64,
) -> Vec<i64> {
    gpu_diffuse_tiled(client, initial, width, height, iters, k, 16)
}

/// As [`gpu_diffuse`], but with an explicit square workgroup (cube) edge `tile`. Because the kernel
/// is a per-cell integer op sequence with no cross-cell reduction, the result must be identical for
/// every `tile`; this knob exists so the gate can prove that tile-size invariance (a Stage 0
/// requirement: autotune must not make a canonical result hardware-dependent). Prefer [`gpu_diffuse`].
pub fn gpu_diffuse_tiled(
    client: &CudaClient,
    initial: &[i64],
    width: u32,
    height: u32,
    iters: u32,
    k: i64,
    tile: u32,
) -> Vec<i64> {
    let n = (width as usize) * (height as usize);
    assert_eq!(
        initial.len(),
        n,
        "gpu_diffuse: initial field must cover width*height cells"
    );
    assert!(tile > 0, "gpu_diffuse: tile edge must be positive");
    if n == 0 {
        return Vec::new();
    }
    let mut f_h = client.create_from_slice(i64::as_bytes(initial));
    let mut g_h = client.empty(n * core::mem::size_of::<i64>());

    let bx = width.div_ceil(tile);
    let by = height.div_ceil(tile);
    for _ in 0..iters {
        unsafe {
            diffuse_kernel::launch::<CudaRuntime>(
                client,
                CubeCount::Static(bx, by, 1),
                CubeDim::new_3d(tile, tile, 1),
                ArrayArg::from_raw_parts(f_h.clone(), n),
                ArrayArg::from_raw_parts(g_h.clone(), n),
                width,
                height,
                k,
            );
        }
        core::mem::swap(&mut f_h, &mut g_h);
    }
    let bytes = client.read_one_unchecked(f_h);
    i64::from_bytes(&bytes).to_vec()
}
