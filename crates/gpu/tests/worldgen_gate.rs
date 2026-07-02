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

//! The GPU worldgen-noise gate: `gpu_worldgen_noise` must reproduce `civsim_world`'s `noise::fractal`
//! bit-for-bit for every cell and every field (elevation, moisture, temperature). The CPU reference is
//! the real oracle function (not a reimplementation), so a per-cell match proves the fractal noise,
//! its draw-keyed lattice fold, and the smoothstep/lerp/normalise chain are all canonical on the GPU.
//! Self-skips unless `CIVSIM_GPU` is set.

use civsim_gpu::{cuda_client, gpu_worldgen_noise};
use civsim_world::noise::{fractal, FIELD_ELEVATION, FIELD_MOISTURE, FIELD_TEMPERATURE};

fn gpu_enabled() -> bool {
    std::env::var("CIVSIM_GPU").is_ok()
}

#[test]
fn worldgen_noise_is_bit_identical_to_the_cpu_fractal() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping worldgen gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let (w, h) = (48u32, 24u32); // the dev map dimensions
    let seed = 0x5EED_1234_ABCD_0001u64;
    let base_period = 16i32; // periods 16, 8, 4, 2, 1 across the octaves exercise the halving + max(1)
    let octaves = 5u32;

    let (gelev, gmoist, gtemp) =
        gpu_worldgen_noise(&client, w, h, seed, base_period as i64, octaves);
    assert_eq!(gelev.len(), (w * h) as usize);

    let mut mism = 0u64;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            for (field, got) in [
                (FIELD_ELEVATION, &gelev),
                (FIELD_MOISTURE, &gmoist),
                (FIELD_TEMPERATURE, &gtemp),
            ] {
                let want = fractal(seed, x as i32, y as i32, field, base_period, octaves).to_bits();
                if got[i] != want {
                    mism += 1;
                    if mism <= 8 {
                        eprintln!(
                            "noise mismatch x={x} y={y} field={field} got={} want={want}",
                            got[i]
                        );
                    }
                }
            }
        }
    }
    assert_eq!(
        mism,
        0,
        "GPU worldgen noise must equal noise::fractal over all {} cells x 3 fields",
        w * h
    );
}
