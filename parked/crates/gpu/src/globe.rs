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

//! NON-CANON viewer globe shading on the GPU (design Part 14, Principle 10). This is the ONLY float
//! kernel in the crate, and it is FENCED OFF from the canonical bit-identical Stage 0 / field /
//! transcendental kernels on purpose: it produces an observer's FRAMEBUFFER, never canonical simulation
//! state, so it carries no bit-identity contract. Two observers' framebuffers need not agree to the bit
//! (Principle 10), which is exactly what frees this kernel from the exact-integer fixed-point discipline
//! the canonical kernels live under and lets it shade in plain `f32`.
//!
//! What it is: the per-pixel body of `civsim_viewer`'s `render::draw_globe` (the ~1M-pixel globe disk
//! shade) moved onto the GPU. Per screen pixel it rebuilds the sphere normal, rotates it into the body
//! frame by the globe orientation, finds the surface CELL the pixel samples, reads that cell's
//! precomputed shading inputs (base colour, terrain normal, self-emitted lava add), applies the
//! sun-direction Lambert term, the display tile-grid seam, the lava emission, and the fresh-crater
//! IMPACT FLASH (summed per pixel over the few active flashes), and packs one `0x00RRGGBB` word.
//!
//! Where the work lives. The heavy DERIVED per-cell inputs (the analytic hillshade normal, the melt
//! glow) are computed ONCE per epoch on the host with the same verified CPU helpers `draw_globe` uses
//! and uploaded as a per-cell cache, so a surface-zoom or deep-time frame no longer pays the
//! O(pixels x craters) analytic gradient the CPU renderer pays. The base colour, the normal, and the
//! lava are held at the cell centre (a cell is smaller than a pixel when zoomed out): the one
//! approximation of the GPU path, which is why its frame is VISUALLY equal to `draw_globe`, not
//! byte-equal (a non-canon display allowance, Principle 10). The impact FLASH is the exception: it has a
//! steep `1/x^3` falloff and is what the owner watches land, so it is summed PER PIXEL in the kernel over
//! the small active-flash array (a handful of craters), pixel-accurate rather than per-cell.
//!
//! The only in-kernel loop is that flash sum, whose accumulator is inline (it calls no `#[cube]`
//! function), so it sidesteps the DSL's accumulator-across-`#[cube]`-loop hazard the `worldgen` note
//! records; the per-cell crater GRADIENT sum is done on the host.

use cubecl::prelude::*;
use cubecl::server::Handle;

/// How the flat per-cell cache maps to sphere directions, mirroring `civsim_viewer`'s
/// `render::SurfaceParam` (kept as a plain enum here so the GPU crate carries no viewer dependency).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobeParam {
    /// Equirectangular `cols` x `rows` grid (row-major), the living-world / fixture cache.
    LatLon { cols: usize, rows: usize },
    /// Six-face equi-angular cube-sphere, each face `face_res` x `face_res`, face-major, the
    /// derived-planet cache (`index = face * face_res^2 + t_row * face_res + s_col`).
    CubeSphere { face_res: usize },
}

impl GlobeParam {
    /// The cache cell count this parameterization addresses.
    pub fn cells(&self) -> usize {
        match *self {
            GlobeParam::LatLon { cols, rows } => cols * rows,
            GlobeParam::CubeSphere { face_res } => 6 * face_res * face_res,
        }
    }
    fn kind(&self) -> u32 {
        match self {
            GlobeParam::LatLon { .. } => 0,
            GlobeParam::CubeSphere { .. } => 1,
        }
    }
    fn a(&self) -> u32 {
        match *self {
            GlobeParam::LatLon { cols, .. } => cols as u32,
            GlobeParam::CubeSphere { face_res } => face_res as u32,
        }
    }
    fn b(&self) -> u32 {
        match *self {
            GlobeParam::LatLon { rows, .. } => rows as u32,
            GlobeParam::CubeSphere { .. } => 0,
        }
    }
}

/// The per-cell shading cache the kernel samples: one entry per surface cell, all DERIVED on the host
/// with the same helpers `draw_globe` uses, in cache order. Every field is display `f32`/packed RGB
/// (Principle 10), never canonical state.
pub struct GlobeCells<'a> {
    /// Packed `0x00RRGGBB` base albedo per cell (the material tint or relief swatch `draw_globe` picks).
    pub base_rgb: &'a [u32],
    /// The body-frame terrain NORMAL per cell (interleaved `nx, ny, nz`), the sphere normal tilted by the
    /// DERIVED slope: the analytic hillshade normal at the cell centre for the derived globe, the cache
    /// finite difference for the living-world globe. Length `3 * cells`.
    pub normal: &'a [f32],
    /// The self-emitted lava add per cell (interleaved `r, g, b`, already `emission * intensity * gain`),
    /// or empty for a world with no melt record. Length `3 * cells` or `0`.
    pub lava_add: &'a [f32],
}

/// One fresh-crater IMPACT FLASH, the display-f32 form the kernel sums per pixel (the same fields
/// `render::ImpactFlash` carries): the crater centre as a body-frame unit vector, its angular rim radius
/// (radians), the cosine of its reach cone (the cheap reject), and its current time-decayed intensity in
/// `[0, 1]`. Uploaded per frame (a handful of flashes), never a per-cell cache.
#[derive(Clone, Copy, Debug)]
pub struct GlobeFlash {
    /// The crater centre as a body-frame unit vector.
    pub center: [f32; 3],
    /// The crater's rim radius as an angle on the sphere (radians).
    pub angular_radius: f32,
    /// `cos(reach cone)`: a pixel whose centre-dot is below this lies outside the bloom.
    pub cos_reach: f32,
    /// The flash brightness in `[0, 1]` at this epoch (the crater's own time decay).
    pub intensity: f32,
}

/// The per-frame camera + lighting + style scalars (everything that changes on rotate / zoom / the
/// day-night sweep, distinct from the per-epoch per-cell cache). Plain display values (Principle 10).
#[derive(Clone, Copy, Debug)]
pub struct GlobeFrame {
    /// Framebuffer width (pixels).
    pub w: usize,
    /// Framebuffer height (pixels).
    pub h: usize,
    /// Globe disk centre x (pixels).
    pub cx: i32,
    /// Globe disk centre y (pixels).
    pub cy: i32,
    /// Globe disk radius (pixels).
    pub radius_px: usize,
    /// Globe orientation longitude spin (radians), the `GlobeOrientation::rot_lon`.
    pub rot_lon: f32,
    /// Globe orientation latitude tilt (radians), the `GlobeOrientation::rot_lat`.
    pub rot_lat: f32,
    /// The DERIVED body-frame sun direction (the hillshade Lambert dots the terrain normal with this).
    pub star_dir_body: [f32; 3],
    /// The view-frame light direction (`normalize(body_to_view(star_dir))`), used by the bare-sphere
    /// (relief-off) Lambert term so it matches `draw_globe`'s screen-space dot.
    pub light_view: [f32; 3],
    /// The sunlight tint per channel in `[0, 1]` (the star's blackbody colour scaled to unit).
    pub tint: [f32; 3],
    /// The night-side ambient floor (`draw_globe`'s `AMBIENT`).
    pub ambient: f32,
    /// The impact-flash emission colour (`draw_globe`'s `FLASH_COLOR`, each channel `0..255`).
    pub flash_color: [f32; 3],
    /// The impact-flash brightness gain (`draw_globe`'s `FLASH_EMISSION_GAIN`).
    pub flash_gain: f32,
    /// The display tile-grid seam `(cols, rows)`, or `(0, 0)` for no grid.
    pub grid: (usize, usize),
    /// Whether the sun-direction hillshade is on (else the bare-sphere normal lights the disk).
    pub hillshade_on: bool,
}

/// A resident GPU globe renderer over a CubeCL runtime `R` (CUDA on the 5090; the CPU backend for the
/// device-free parity test). It holds the compute client and the per-cell cache resident on the device
/// across frames, re-uploaded only when [`upload_cells`](Self::upload_cells) is called (on an epoch
/// change); [`render`](Self::render) then uploads only the small per-frame scalars, the flash array, and
/// the framebuffer, launches the shade over the pixel grid, and reads the frame back. NON-CANON
/// (Principle 10): it writes pixels, never canonical state, so it needs no bit-identity gate.
pub struct GlobeRenderer<R: Runtime> {
    client: ComputeClient<R>,
    resident: Option<Resident>,
    /// The caller's epoch tag for the resident cache, so it re-uploads only when the epoch changes (a deep-time
    /// step or a scene switch): a rotate / zoom / sweep keeps the same tag and reuses the resident cache.
    tag: Option<u64>,
}

/// The device-resident per-cell cache handles (held across frames).
struct Resident {
    base_rgb: Handle,
    normal: Handle,
    lava_add: Handle,
    cells: usize,
    has_lava: bool,
    param: GlobeParam,
}

impl<R: Runtime> GlobeRenderer<R> {
    /// Build a renderer on an explicit client (used by the parity test with the CPU backend, and by the
    /// viewer with a CUDA client).
    pub fn from_client(client: ComputeClient<R>) -> Self {
        GlobeRenderer {
            client,
            resident: None,
            tag: None,
        }
    }

    /// Whether a per-cell cache is resident.
    pub fn has_cells(&self) -> bool {
        self.resident.is_some()
    }

    /// The epoch tag of the resident cache, if any (the value passed to the last [`upload_cells`](Self::upload_cells)).
    /// The caller compares its current epoch against this to decide whether a re-upload is needed.
    pub fn tag(&self) -> Option<u64> {
        self.tag
    }

    /// The parameterization of the resident cache, if any.
    pub fn resident_param(&self) -> Option<GlobeParam> {
        self.resident.as_ref().map(|r| r.param)
    }

    /// Upload the per-cell shading cache and hold it resident across frames, tagged with the caller's epoch `tag`.
    /// Call on an epoch change (a deep-time step, or a fresh scene); a rotate / zoom / sweep reuses the resident
    /// cache (its `tag` unchanged) and only [`render`](Self::render) runs. `param` says how the flat cache maps to
    /// sphere directions and fixes the cell count. Panics if any supplied slice length disagrees with
    /// `param.cells()`.
    pub fn upload_cells(&mut self, param: GlobeParam, cells: GlobeCells, tag: u64) {
        self.tag = Some(tag);
        let n = param.cells();
        assert_eq!(cells.base_rgb.len(), n, "globe: base_rgb must cover cells");
        assert_eq!(cells.normal.len(), 3 * n, "globe: normal is 3 per cell");
        let has_lava = !cells.lava_add.is_empty();
        if has_lava {
            assert_eq!(cells.lava_add.len(), 3 * n, "globe: lava_add is 3 per cell");
        }
        // A one-element dummy stands in for an absent emissive layer so the kernel always has a valid
        // buffer to bind (the `has_lava` flag gates whether it is read).
        let dummy = [0.0f32];
        let lava_src = if has_lava { cells.lava_add } else { &dummy };
        self.resident = Some(Resident {
            base_rgb: self.client.create_from_slice(u32::as_bytes(cells.base_rgb)),
            normal: self.client.create_from_slice(f32::as_bytes(cells.normal)),
            lava_add: self.client.create_from_slice(f32::as_bytes(lava_src)),
            cells: n,
            has_lava,
            param,
        });
    }

    /// Shade the globe disk into `buf` (a `w * h` `0x00RRGGBB` framebuffer) on the GPU, summing the `flashes`
    /// per pixel. `buf` arrives with the background and star already drawn; the kernel overwrites only the disk
    /// pixels and leaves the rest as uploaded, so the caller composites the atmosphere limb over the result
    /// exactly as `draw_globe_scene` does. A no-op (leaving `buf` untouched) if no cache is resident or the radius
    /// is zero. Determinism of the CANON is untouched: this reads no canonical state and writes only pixels.
    pub fn render(&self, buf: &mut [u32], frame: &GlobeFrame, flashes: &[GlobeFlash]) {
        let Some(res) = self.resident.as_ref() else {
            return;
        };
        if frame.radius_px == 0 || frame.w == 0 || frame.h == 0 {
            return;
        }
        assert_eq!(buf.len(), frame.w * frame.h, "globe: buf must be w*h");
        let nf = flashes.len();
        // Flatten the flash array into a centre buffer (3 per flash) and a geometry buffer (angular_radius,
        // cos_reach, intensity per flash); a one-element dummy stands in when there are no flashes.
        let (fc, fg): (Vec<f32>, Vec<f32>) = if nf == 0 {
            (vec![0.0], vec![0.0])
        } else {
            let mut fc = Vec::with_capacity(3 * nf);
            let mut fg = Vec::with_capacity(3 * nf);
            for f in flashes {
                fc.extend_from_slice(&f.center);
                fg.push(f.angular_radius);
                fg.push(f.cos_reach);
                fg.push(f.intensity);
            }
            (fc, fg)
        };
        let ip: Vec<u32> = vec![
            frame.w as u32,
            frame.h as u32,
            res.cells as u32,
            res.param.kind(),
            res.param.a(),
            res.param.b(),
            frame.grid.0 as u32,
            frame.grid.1 as u32,
            u32::from(frame.hillshade_on),
            u32::from(res.has_lava),
            nf as u32,
        ];
        let fp: Vec<f32> = vec![
            frame.cx as f32,
            frame.cy as f32,
            frame.radius_px as f32,
            frame.rot_lon,
            frame.rot_lat,
            frame.star_dir_body[0],
            frame.star_dir_body[1],
            frame.star_dir_body[2],
            frame.light_view[0],
            frame.light_view[1],
            frame.light_view[2],
            frame.tint[0],
            frame.tint[1],
            frame.tint[2],
            frame.ambient,
            frame.flash_color[0],
            frame.flash_color[1],
            frame.flash_color[2],
            frame.flash_gain,
        ];
        let out_h = self.client.create_from_slice(u32::as_bytes(buf));
        let ip_h = self.client.create_from_slice(u32::as_bytes(&ip));
        let fp_h = self.client.create_from_slice(f32::as_bytes(&fp));
        let fc_h = self.client.create_from_slice(f32::as_bytes(&fc));
        let fg_h = self.client.create_from_slice(f32::as_bytes(&fg));
        let n_pix = buf.len();
        let lava_len = if res.has_lava { 3 * res.cells } else { 1 };
        let tile = 16u32;
        let bx = (frame.w as u32).div_ceil(tile);
        let by = (frame.h as u32).div_ceil(tile);
        unsafe {
            globe_kernel::launch::<R>(
                &self.client,
                CubeCount::Static(bx, by, 1),
                CubeDim::new_3d(tile, tile, 1),
                ArrayArg::from_raw_parts(out_h.clone(), n_pix),
                ArrayArg::from_raw_parts(res.base_rgb.clone(), res.cells),
                ArrayArg::from_raw_parts(res.normal.clone(), 3 * res.cells),
                ArrayArg::from_raw_parts(res.lava_add.clone(), lava_len),
                ArrayArg::from_raw_parts(fc_h.clone(), fc.len()),
                ArrayArg::from_raw_parts(fg_h.clone(), fg.len()),
                ArrayArg::from_raw_parts(ip_h.clone(), ip.len()),
                ArrayArg::from_raw_parts(fp_h.clone(), fp.len()),
            );
        }
        let bytes = self.client.read_one_unchecked(out_h);
        buf.copy_from_slice(u32::from_bytes(&bytes));
    }
}

/// The per-pixel globe shade, the GPU port of `render::draw_globe`'s inner body. Each thread owns one
/// screen pixel `(x, y)`; it shades only pixels inside the globe disk and leaves the rest as the caller
/// uploaded (the background and star), matching `draw_globe`'s "pixels outside the disk are untouched".
#[cube(launch)]
#[allow(clippy::too_many_arguments)]
fn globe_kernel(
    out: &mut Array<u32>,
    base_rgb: &Array<u32>,
    normal: &Array<f32>,
    lava_add: &Array<f32>,
    flash_c: &Array<f32>,
    flash_g: &Array<f32>,
    ip: &Array<u32>,
    fp: &Array<f32>,
) {
    let x = ABSOLUTE_POS_X;
    let y = ABSOLUTE_POS_Y;
    let w = ip[0];
    let h = ip[1];
    if x < w && y < h {
        let ncells = ip[2];
        let param_kind = ip[3];
        let pa = ip[4];
        let pb = ip[5];
        let grid_cols = ip[6];
        let grid_rows = ip[7];
        let hillshade_on = ip[8];
        let has_lava = ip[9];
        let nf = ip[10];

        let cx = fp[0];
        let cy = fp[1];
        let r = fp[2];
        let rot_lon = fp[3];
        let rot_lat = fp[4];
        let sx = fp[5];
        let sy = fp[6];
        let sz = fp[7];
        let lvx = fp[8];
        let lvy = fp[9];
        let lvz = fp[10];
        let tr = fp[11];
        let tg = fp[12];
        let tb = fp[13];
        let ambient = fp[14];
        let flash_r = fp[15];
        let flash_g_col = fp[16];
        let flash_b = fp[17];
        let flash_gain = fp[18];

        let pidx = (y * w + x) as usize;

        let nxf = (f32::cast_from(x) - cx) / r;
        let nyf = (f32::cast_from(y) - cy) / r;
        let d2 = nxf * nxf + nyf * nyf;
        if d2 <= 1.0 {
            let nzf = (1.0 - d2).sqrt();
            // view point p = [nxf, -nyf, nzf], carried to the body frame by view_to_body(rot_lon, rot_lat).
            let pvx = nxf;
            let pvy = -nyf;
            let pvz = nzf;
            let clat = rot_lat.cos();
            let slat = rot_lat.sin();
            let clon = rot_lon.cos();
            let slon = rot_lon.sin();
            // q = rot_x(p, -rot_lat)
            let qx = pvx;
            let qy = clat * pvy + slat * pvz;
            let qz = -slat * pvy + clat * pvz;
            // b = rot_y(q, -rot_lon)
            let bx = clon * qx - slon * qz;
            let by = qy;
            let bz = slon * qx + clon * qz;

            // body_to_uv
            let lat = by.clamp(-1.0, 1.0).asin();
            let lon = bx.atan2(bz);
            let pi = core::f32::consts::PI;
            let tau = core::f32::consts::TAU;
            let uu0 = (lon + pi) / tau;
            let uu = uu0 - uu0.floor();
            let vv = (0.5 - lat / pi).clamp(0.0, 1.0);

            // cell index
            let cell = cell_index(param_kind, pa, pb, bx, by, bz, uu, vv, ncells);
            let c3 = cell * 3u32;

            // base albedo
            let word = base_rgb[cell as usize];
            let base_r = f32::cast_from((word >> 16u32) & 255u32);
            let base_g = f32::cast_from((word >> 8u32) & 255u32);
            let base_b = f32::cast_from(word & 255u32);

            // Lambert: hillshade dots the terrain normal with the body sun dir; the bare-sphere path
            // dots the view normal [nxf, -nyf, nzf] with the view light (the same value when flat).
            let nnx = normal[c3 as usize];
            let nny = normal[(c3 + 1u32) as usize];
            let nnz = normal[(c3 + 2u32) as usize];
            let dot_hs = nnx * sx + nny * sy + nnz * sz;
            let dot_flat = pvx * lvx + pvy * lvy + pvz * lvz;
            let lambert_raw = select(hillshade_on == 1u32, dot_hs, dot_flat);
            let lambert = lambert_raw.max(0.0);

            let day_r = ambient + (1.0 - ambient) * lambert * tr;
            let day_g = ambient + (1.0 - ambient) * lambert * tg;
            let day_b = ambient + (1.0 - ambient) * lambert * tb;
            let mut cr = (base_r * day_r).clamp(0.0, 255.0);
            let mut cg = (base_g * day_g).clamp(0.0, 255.0);
            let mut cb = (base_b * day_b).clamp(0.0, 255.0);

            // display tile-grid seam (per pixel, matching draw_globe)
            if grid_cols > 0u32 && grid_rows > 0u32 {
                let gc = f32::cast_from(grid_cols);
                let gr = f32::cast_from(grid_rows);
                let gu = uu * gc;
                let gv = vv * gr;
                let du = (gu - (gu + 0.5).floor()).abs();
                let dv = (gv - (gv + 0.5).floor()).abs();
                let half_u = (gc / (2.0 * r) * 0.6).min(0.25);
                let half_v = (gr / (2.0 * r) * 0.6).min(0.25);
                let seam = du < half_u || dv < half_v;
                cr = select(seam, cr * 0.45, cr);
                cg = select(seam, cg * 0.45, cg);
                cb = select(seam, cb * 0.5, cb);
            }

            // self-emitted lava glow (adds over the shaded crust, survives on the night side)
            if has_lava == 1u32 {
                cr = (cr + lava_add[c3 as usize]).clamp(0.0, 255.0);
                cg = (cg + lava_add[(c3 + 1u32) as usize]).clamp(0.0, 255.0);
                cb = (cb + lava_add[(c3 + 2u32) as usize]).clamp(0.0, 255.0);
            }

            // self-emitted IMPACT FLASH, summed PER PIXEL over the active flashes (the port of
            // crater_flash_emission): full over the excavation bowl (x <= 1), the x^-3 ejecta falloff beyond.
            if nf > 0u32 {
                let mut e = 0.0f32;
                for k in 0..nf {
                    let k3 = k * 3u32;
                    let fcx = flash_c[k3 as usize];
                    let fcy = flash_c[(k3 + 1u32) as usize];
                    let fcz = flash_c[(k3 + 2u32) as usize];
                    let far = flash_g[k3 as usize];
                    let fcr = flash_g[(k3 + 1u32) as usize];
                    let fin = flash_g[(k3 + 2u32) as usize];
                    let dot = bx * fcx + by * fcy + bz * fcz;
                    let inside = dot >= fcr && far > 0.0;
                    let ccx = by * fcz - bz * fcy;
                    let ccy = bz * fcx - bx * fcz;
                    let ccz = bx * fcy - by * fcx;
                    let sang = (ccx * ccx + ccy * ccy + ccz * ccz).sqrt().clamp(0.0, 1.0);
                    let ang = sang.asin();
                    let safe_far = select(far > 0.0, far, 1.0);
                    let xr = ang / safe_far;
                    let prof = select(xr <= 1.0, 1.0, 1.0 / (xr * xr * xr));
                    let contrib = select(inside, fin * prof, 0.0);
                    e += contrib;
                }
                cr = (cr + flash_r * e * flash_gain).clamp(0.0, 255.0);
                cg = (cg + flash_g_col * e * flash_gain).clamp(0.0, 255.0);
                cb = (cb + flash_b * e * flash_gain).clamp(0.0, 255.0);
            }

            let ir = u32::cast_from(cr);
            let ig = u32::cast_from(cg);
            let ib = u32::cast_from(cb);
            out[pidx] = (ir << 16u32) | (ig << 8u32) | ib;
        }
    }
}

/// The flat cache index of a body-frame direction under the cache parameterization, the `#[cube]` port of
/// `render::surface_cell_index` (LatLon + equi-angular cube-sphere). `param_kind` is `0` for LatLon
/// (`pa` cols, `pb` rows) and `1` for CubeSphere (`pa` face_res).
#[cube]
#[allow(clippy::too_many_arguments)]
fn cell_index(
    param_kind: u32,
    pa: u32,
    pb: u32,
    bx: f32,
    by: f32,
    bz: f32,
    uu: f32,
    vv: f32,
    ncells: u32,
) -> u32 {
    // LatLon cell (also the harmless default the CubeSphere path selects away).
    let cols = pa;
    let rows_safe = select(pb > 0u32, pb, 1u32);
    let cu = min_u32(
        u32::cast_from(uu.clamp(0.0, 0.9999) * f32::cast_from(cols)),
        cols - 1u32,
    );
    let cv = min_u32(
        u32::cast_from(vv.clamp(0.0, 0.9999) * f32::cast_from(rows_safe)),
        rows_safe - 1u32,
    );
    let idx_latlon = cv * cols + cu;

    // CubeSphere: the equi-angular inverse map (cube_dir_to_face_st), branchless.
    let face_res = pa;
    let ax = bx.abs();
    let ay = by.abs();
    let az = bz.abs();
    let cond_x = ax >= ay && ax >= az;
    let cond_y = ay >= az;
    let face = select(
        cond_x,
        select(bx > 0.0, 0u32, 1u32),
        select(
            cond_y,
            select(by > 0.0, 2u32, 3u32),
            select(bz > 0.0, 4u32, 5u32),
        ),
    );
    let dx = select(
        cond_x,
        select(bx > 0.0, -bz, bz),
        select(cond_y, bx, select(bz > 0.0, bx, -bx)),
    );
    let dy = select(cond_x, by, select(cond_y, select(by > 0.0, -bz, bz), by));
    let dz = select(
        cond_x,
        select(bx > 0.0, bx, -bx),
        select(cond_y, select(by > 0.0, by, -by), select(bz > 0.0, bz, -bz)),
    );
    let inv = select(dz != 0.0, 1.0 / dz, 0.0);
    let frac_pi_2 = core::f32::consts::FRAC_PI_2;
    let s = (dx * inv).atan() / frac_pi_2 + 0.5;
    let t = (dy * inv).atan() / frac_pi_2 + 0.5;
    let sc = s.clamp(0.0, 0.9999);
    let tc = t.clamp(0.0, 0.9999);
    let ci = min_u32(
        u32::cast_from(sc * f32::cast_from(face_res)),
        face_res - 1u32,
    );
    let cj = min_u32(
        u32::cast_from(tc * f32::cast_from(face_res)),
        face_res - 1u32,
    );
    let idx_cube = face * face_res * face_res + cj * face_res + ci;

    let idx = select(param_kind == 0u32, idx_latlon, idx_cube);
    min_u32(idx, ncells - 1u32)
}

/// Branchless integer minimum for the cell-index clamps.
#[cube]
fn min_u32(a: u32, b: u32) -> u32 {
    select(a < b, a, b)
}

/// The globe renderer on the default CUDA device (the RTX 5090 target).
pub type CudaGlobeRenderer = GlobeRenderer<cubecl::cuda::CudaRuntime>;

/// The globe renderer on the CubeCL CPU backend (MLIR/LLVM, no device). It runs the identical `#[cube]` kernel
/// through an independent codegen path, so it is the device-free way to exercise and gate the shade (the viewer's
/// parity test uses it). Naming this alias keeps callers free of the cubecl runtime types.
pub type CpuGlobeRenderer = GlobeRenderer<cubecl::cpu::CpuRuntime>;

/// A globe renderer on the CubeCL CPU backend (no device needed). Used by the viewer parity test to render the same
/// scene through the GPU kernel and compare it to the CPU `draw_globe`, on any machine.
pub fn cpu_renderer() -> CpuGlobeRenderer {
    GlobeRenderer::from_client(crate::cpu_client())
}

/// Try to build a globe renderer on the default CUDA device, returning `None` when no usable device is present
/// (no driver, no NVRTC, or a launch failure), so the viewer falls back to the CPU renderer GPU-less. The probe is
/// real: it creates the client, uploads a one-cell cache, and shades a tiny framebuffer, all under
/// `catch_unwind`, so a box without a working CUDA stack yields `None` rather than a panic. Safe to call at
/// startup on any machine.
pub fn try_cuda_renderer() -> Option<CudaGlobeRenderer> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let client = crate::cuda_client();
        let mut r = GlobeRenderer::from_client(client);
        // A minimal 1-cell scene: prove upload + launch + readback work on this device.
        let base = [0x00808080u32];
        let normal = [0.0f32, 0.0, 1.0];
        let empty: [f32; 0] = [];
        r.upload_cells(
            GlobeParam::LatLon { cols: 1, rows: 1 },
            GlobeCells {
                base_rgb: &base,
                normal: &normal,
                lava_add: &empty,
            },
            0,
        );
        let mut buf = vec![0u32; 16];
        r.render(
            &mut buf,
            &GlobeFrame {
                w: 4,
                h: 4,
                cx: 2,
                cy: 2,
                radius_px: 2,
                rot_lon: 0.0,
                rot_lat: 0.0,
                star_dir_body: [0.0, 0.0, 1.0],
                light_view: [0.0, 0.0, 1.0],
                tint: [1.0, 1.0, 1.0],
                ambient: 0.1,
                flash_color: [255.0, 246.0, 224.0],
                flash_gain: 2.5,
                grid: (0, 0),
                hillshade_on: false,
            },
            &[],
        );
        r
    }))
    .ok()
}
