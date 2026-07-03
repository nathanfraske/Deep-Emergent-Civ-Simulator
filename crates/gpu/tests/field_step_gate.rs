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

//! The GPU field-step gate: `gpu_field_step` must reproduce the runner's `Field::step` (clamped-Neumann
//! diffusion plus relaxation to a baseline) bit-for-bit over the whole field after many steps. The CPU
//! reference here is the exact same stencil written against `civsim_core::Fixed`, so a per-cell match
//! proves the runner's field is canonical on the GPU (distinct from the toroidal `diffusion_bench` that
//! `gpu_diffuse` already gates). Self-skips unless `CIVSIM_GPU` is set.

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_field_step};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

fn idx(x: i32, y: i32, w: i32) -> usize {
    (y * w + x) as usize
}

/// The exact CPU reference: `civsim_sim`'s `Field::step`, a clamped-Neumann five-point stencil plus a
/// relaxation term toward the baseline, iterated `iters` times.
fn cpu_field_step(
    initial: &[i64],
    baseline: &[i64],
    w: i32,
    h: i32,
    iters: u32,
    diffusion: Fixed,
    relaxation: Fixed,
) -> Vec<i64> {
    let mut temp: Vec<Fixed> = initial.iter().map(|&b| Fixed::from_bits(b)).collect();
    let base: Vec<Fixed> = baseline.iter().map(|&b| Fixed::from_bits(b)).collect();
    for _ in 0..iters {
        let mut next = temp.clone();
        for y in 0..h {
            for x in 0..w {
                let i = idx(x, y, w);
                let cur = temp[i];
                let up = temp[idx(x, if y > 0 { y - 1 } else { y }, w)];
                let dn = temp[idx(x, if y < h - 1 { y + 1 } else { y }, w)];
                let lf = temp[idx(if x > 0 { x - 1 } else { x }, y, w)];
                let rt = temp[idx(if x < w - 1 { x + 1 } else { x }, y, w)];
                let lap = up + dn + lf + rt - Fixed::from_int(4).mul(cur);
                let relax = base[i] - cur;
                next[i] = cur + diffusion.mul(lap) + relaxation.mul(relax);
            }
        }
        temp = next;
    }
    temp.iter().map(|f| f.to_bits()).collect()
}

#[test]
fn field_step_is_bit_identical_to_the_runner_field() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping field-step gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    // Non-square, non-power-of-two so the clamped boundary and the linear indexing both bite.
    let (w, h) = (37i32, 23i32);
    // A spread initial field and a distinct baseline, so diffusion AND relaxation both act every step.
    let (mut initial, mut baseline) = (Vec::new(), Vec::new());
    for y in 0..h {
        for x in 0..w {
            initial.push(Fixed::from_ratio(((x * 7 + y * 3) % 20) as i64, 4).to_bits());
            baseline.push(Fixed::from_ratio(((x + 2 * y) % 15) as i64, 5).to_bits());
        }
    }
    let diffusion = Fixed::from_ratio(1, 8);
    let relaxation = Fixed::from_ratio(1, 16);
    let iters = 50u32;

    let got = gpu_field_step(
        &client,
        &initial,
        &baseline,
        w as u32,
        h as u32,
        iters,
        diffusion.to_bits(),
        relaxation.to_bits(),
    );
    let want = cpu_field_step(&initial, &baseline, w, h, iters, diffusion, relaxation);
    assert_eq!(got.len(), want.len());
    let mism = got.iter().zip(&want).filter(|(a, b)| a != b).count();
    assert_eq!(
        mism,
        0,
        "GPU field step must equal the CPU Field::step over all {} cells after {iters} iters",
        want.len()
    );
}
