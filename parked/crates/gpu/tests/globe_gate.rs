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

//! End-to-end smoke gate for the NON-CANON globe shade kernel (`globe.rs`). It runs the kernel through a
//! real backend and checks the three properties a framebuffer shade must have: the disk is drawn, pixels
//! OUTSIDE the disk keep the uploaded background (the kernel writes only the disk), and the star-facing
//! hemisphere is brighter than the far side (the Lambert term works). It runs on the CubeCL CPU backend
//! (no device needed, so it is part of the default gate), and additionally on CUDA when `CIVSIM_GPU` is
//! set. This is a sanity gate; the pixel-for-pixel parity against the CPU `draw_globe` reference lives in
//! the viewer crate (which owns `draw_globe`).

use civsim_gpu::globe::{GlobeCells, GlobeFrame, GlobeParam, GlobeRenderer};
use cubecl::cpu::{CpuDevice, CpuRuntime};
use cubecl::prelude::*;

/// A small uniform-crust scene: a LatLon cache of one grey material, flat sphere normals, no emission.
fn scene() -> (GlobeParam, Vec<u32>, Vec<f32>) {
    let param = GlobeParam::LatLon { cols: 8, rows: 4 };
    let n = param.cells();
    let base = vec![0x00808088u32; n]; // uniform grey-blue crust
                                       // Flat normals: the bare sphere normal is supplied per cell as (0,0,1) placeholder; the gate uses
                                       // the relief-OFF path (hillshade_on = false), so the kernel ignores these and lights the sphere
                                       // normal directly. Any finite value is fine here.
    let normal = vec![0.0f32; 3 * n]
        .iter()
        .enumerate()
        .map(|(i, _)| if i % 3 == 2 { 1.0 } else { 0.0 })
        .collect();
    (param, base, normal)
}

fn frame(w: usize, h: usize) -> GlobeFrame {
    GlobeFrame {
        w,
        h,
        cx: (w / 2) as i32,
        cy: (h / 2) as i32,
        radius_px: (w.min(h) / 2).saturating_sub(2),
        rot_lon: 0.0,
        rot_lat: 0.0,
        // Sun toward the viewer and to the right: the +x, +z hemisphere is lit.
        star_dir_body: [0.6, 0.0, 0.8],
        light_view: [0.6, 0.0, 0.8],
        tint: [1.0, 1.0, 1.0],
        ambient: 0.10,
        flash_color: [255.0, 246.0, 224.0],
        flash_gain: 2.5,
        grid: (0, 0),
        hillshade_on: false,
    }
}

fn channels(px: u32) -> (u8, u8, u8) {
    (
        ((px >> 16) & 255) as u8,
        ((px >> 8) & 255) as u8,
        (px & 255) as u8,
    )
}

fn luma(px: u32) -> u32 {
    let (r, g, b) = channels(px);
    r as u32 + g as u32 + b as u32
}

fn run<R: Runtime>(client: ComputeClient<R>) {
    let (param, base, normal) = scene();
    let empty: Vec<f32> = Vec::new();
    let mut r = GlobeRenderer::from_client(client);
    r.upload_cells(
        param,
        GlobeCells {
            base_rgb: &base,
            normal: &normal,
            lava_add: &empty,
        },
        0,
    );
    let (w, h) = (200usize, 160usize);
    const BG: u32 = 0x00101018;
    let mut buf = vec![BG; w * h];
    let f = frame(w, h);
    r.render(&mut buf, &f, &[]);

    // 1) A corner pixel (well outside the disk) keeps the uploaded background.
    assert_eq!(buf[0], BG, "outside-disk pixel must keep the background");

    // 2) The disk centre is drawn (not background) and lit.
    let cidx = (h / 2) * w + w / 2;
    assert_ne!(buf[cidx], BG, "disk centre must be shaded");

    // 3) The star-facing side (right of centre) is brighter than the far side (left of centre): the
    //    sun is at +x, so the right hemisphere is lit and the left falls toward the ambient floor.
    let rp = f.radius_px;
    let right = (h / 2) * w + (w / 2 + rp / 2);
    let left = (h / 2) * w + (w / 2 - rp / 2);
    assert!(
        luma(buf[right]) > luma(buf[left]),
        "star-facing side must be brighter: right={} left={}",
        luma(buf[right]),
        luma(buf[left])
    );
    // 4) The lit side clearly exceeds the pure-ambient floor (0.10 * base), so lighting was applied.
    let (br, _, _) = channels(0x00808088);
    let ambient_r = (br as f32 * 0.10) as u32;
    let (rr, _, _) = channels(buf[right]);
    assert!(
        rr as u32 > ambient_r + 5,
        "lit side must exceed the ambient floor: {} vs {}",
        rr,
        ambient_r
    );
}

#[test]
fn globe_shade_runs_on_cpu_backend() {
    run(CpuRuntime::client(&CpuDevice));
}

#[test]
fn globe_shade_runs_on_cuda() {
    if std::env::var("CIVSIM_GPU").is_err() {
        eprintln!("civsim-gpu: skipping globe CUDA smoke (set CIVSIM_GPU; needs a device)");
        return;
    }
    run(civsim_gpu::cuda_client());
}
