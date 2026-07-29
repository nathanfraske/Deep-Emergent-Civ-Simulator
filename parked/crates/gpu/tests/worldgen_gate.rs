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

use civsim_core::Fixed;
use civsim_gpu::{cuda_client, gpu_worldgen, gpu_worldgen_noise};
use civsim_world::noise::{fractal, FIELD_ELEVATION, FIELD_MOISTURE, FIELD_TEMPERATURE};
use civsim_world::terrain::{BiomeDef, BiomeId, BiomeSet, Rgb};

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

#[test]
fn worldgen_biome_is_bit_identical_to_classify() {
    if !gpu_enabled() {
        eprintln!("civsim-gpu: skipping biome gate (set CIVSIM_GPU=1 to run)");
        return;
    }
    let client = cuda_client();
    let (w, h) = (48u32, 24u32);
    let seed = 0x5EED_1234_ABCD_0001u64;
    let base_period = 16i32;
    let octaves = 5u32;

    // Four biomes whose bands tile the [0, 1) field cube, so first-match order and every branch of the
    // classify scan are exercised and no cell falls to the fallback.
    let f = |n: i64, d: i64| Fixed::from_ratio(n, d);
    let band = |a: i64, b: i64| (f(a, 10), f(b, 10));
    let mk = |id: u16, el: (Fixed, Fixed), mo: (Fixed, Fixed), te: (Fixed, Fixed)| BiomeDef {
        id: BiomeId(id),
        name: format!("b{id}"),
        glyph: '#',
        color: Rgb::default(),
        elevation: el,
        moisture: mo,
        temperature: te,
    };
    let biomes = vec![
        mk(1, band(0, 3), band(0, 10), band(0, 10)),
        mk(2, band(3, 10), band(0, 5), band(0, 10)),
        mk(3, band(3, 10), band(5, 10), band(0, 5)),
        mk(4, band(3, 10), band(5, 10), band(5, 10)),
    ];
    let fallback = BiomeId(99);
    let set = BiomeSet::new(biomes.clone(), fallback);

    let (mut lo, mut hi, mut ids) = (Vec::new(), Vec::new(), Vec::new());
    for b in &biomes {
        lo.push(b.elevation.0.to_bits());
        hi.push(b.elevation.1.to_bits());
        lo.push(b.moisture.0.to_bits());
        hi.push(b.moisture.1.to_bits());
        lo.push(b.temperature.0.to_bits());
        hi.push(b.temperature.1.to_bits());
        ids.push(b.id.0 as u32);
    }

    let (_e, _m, _t, gbiome) = gpu_worldgen(
        &client,
        w,
        h,
        seed,
        base_period as i64,
        octaves,
        &lo,
        &hi,
        &ids,
        fallback.0 as u32,
    );

    let mut mism = 0u64;
    let mut nonfallback = 0u64;
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let el = fractal(
                seed,
                x as i32,
                y as i32,
                FIELD_ELEVATION,
                base_period,
                octaves,
            );
            let mo = fractal(
                seed,
                x as i32,
                y as i32,
                FIELD_MOISTURE,
                base_period,
                octaves,
            );
            let te = fractal(
                seed,
                x as i32,
                y as i32,
                FIELD_TEMPERATURE,
                base_period,
                octaves,
            );
            let want = set.classify(el, mo, te).0 as u32;
            if want != fallback.0 as u32 {
                nonfallback += 1;
            }
            if gbiome[i] != want {
                mism += 1;
                if mism <= 8 {
                    eprintln!("biome mismatch x={x} y={y} got={} want={want}", gbiome[i]);
                }
            }
        }
    }
    assert!(
        nonfallback > 0,
        "the bands must claim some cells (else vacuous)"
    );
    assert_eq!(
        mism,
        0,
        "GPU biome must equal BiomeSet::classify over all {} cells",
        w * h
    );
}
