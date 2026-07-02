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

//! The device gate for the canonical fixed-point diffusion field kernel (Part 5.5): the GPU stencil
//! must reproduce the CPU reference bit-for-bit over the whole field after many steps. The CPU
//! reference here is the exact same toroidal Jacobi step written against `civsim_core::Fixed`, so a
//! per-cell match proves the GPU field is canonical. Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_diffuse, gpu_diffuse_tiled};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

/// The initial field of the bench: `((x*7 + y*13) % 100)` as a whole Q32.32 value, row-major.
fn initial_field(w: usize, h: usize) -> Vec<i64> {
    let mut f = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            f.push(Fixed::from_int(((x * 7 + y * 13) % 100) as i32).to_bits());
        }
    }
    f
}

/// The exact CPU reference for `diffuse_kernel`: a toroidal five-point Jacobi step,
/// `g = c + k*lap`, `lap = up + dn + lf + rt - 4*c`, all over `Fixed` (the oracle).
fn cpu_diffuse(initial: &[i64], w: usize, h: usize, iters: usize, k: Fixed) -> Vec<i64> {
    let mut f: Vec<Fixed> = initial.iter().map(|&b| Fixed::from_bits(b)).collect();
    let mut g = f.clone();
    for _ in 0..iters {
        for y in 0..h {
            let yu = (y + h - 1) % h;
            let yd = (y + 1) % h;
            for x in 0..w {
                let xl = (x + w - 1) % w;
                let xr = (x + 1) % w;
                let c = f[y * w + x];
                let four_c = Fixed::from_bits(c.to_bits() << 2); // 4*c is an exact shift
                let lap = f[yu * w + x] + f[yd * w + x] + f[y * w + xl] + f[y * w + xr] - four_c;
                g[y * w + x] = c + k.mul(lap);
            }
        }
        std::mem::swap(&mut f, &mut g);
    }
    f.iter().map(|v| v.to_bits()).collect()
}

#[test]
fn diffusion_field_is_bit_identical_cpu_vs_gpu() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping diffusion device gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let (w, h) = (64usize, 64usize);
    let iters = 50usize;
    let k = Fixed::from_ratio(1, 5); // 0.2, below the 0.25 stability bound
    let initial = initial_field(w, h);

    let cpu = cpu_diffuse(&initial, w, h, iters, k);
    let client = cuda_client();
    let gpu = gpu_diffuse(
        &client,
        &initial,
        w as u32,
        h as u32,
        iters as u32,
        k.to_bits(),
    );

    assert_eq!(gpu.len(), cpu.len());
    let mut mism = 0u64;
    for i in 0..cpu.len() {
        if gpu[i] != cpu[i] {
            mism += 1;
            if mism <= 5 {
                eprintln!("cell {i}: gpu={:#x} cpu={:#x}", gpu[i], cpu[i]);
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "GPU diffusion field must match the CPU reference in every one of {} cells",
        cpu.len()
    );
}

#[test]
fn diffusion_is_invariant_to_tile_size() {
    // A canonical kernel must give the same bits regardless of workgroup/tile size, or autotune could
    // make the result hardware-dependent (Stage 0 requirement). The kernel is a per-cell op sequence
    // with no cross-cell reduction, so every tile edge must agree bit-for-bit.
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping diffusion tile-invariance gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let (w, h) = (70usize, 66usize); // deliberately not a multiple of any tested tile edge
    let iters = 30usize;
    let k = Fixed::from_ratio(1, 5);
    let initial = initial_field(w, h);
    let client = cuda_client();

    let base = gpu_diffuse(
        &client,
        &initial,
        w as u32,
        h as u32,
        iters as u32,
        k.to_bits(),
    );
    for tile in [1u32, 4, 8, 32] {
        let other = gpu_diffuse_tiled(
            &client,
            &initial,
            w as u32,
            h as u32,
            iters as u32,
            k.to_bits(),
            tile,
        );
        assert_eq!(
            other, base,
            "diffusion result changed with tile edge {tile}"
        );
    }
}
