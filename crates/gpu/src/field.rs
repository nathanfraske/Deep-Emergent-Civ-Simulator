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
use cubecl::server::Handle;
use std::thread::ThreadId;
use std::time::Duration;

use crate::prim::q32_mul;

use crate::stage0::CudaClient;

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

/// One canonical diffusion-and-relaxation step over a clamped-Neumann (zero-flux) grid, matching
/// `civsim_sim`'s `Field::step` (`crates/sim/src/runner.rs`): `g = c + diffusion*lap + relaxation*(baseline
/// - c)`, where `lap = up + dn + lf + rt - 4*c` and an edge neighbour clamps to the cell itself rather
/// than wrapping. This is the runner's field, distinct from [`diffuse_kernel`] above, which is toroidal
/// and diffusion-only (the `diffusion_bench` shape). Field and baseline values are `i64` Q32.32 bits;
/// the two coefficient multiplies are the pinned `q32_mul` and the `4*c` is the exact shift.
#[cube(launch)]
fn field_step_kernel(
    f: &Array<i64>,
    baseline: &Array<i64>,
    g: &mut Array<i64>,
    width: u32,
    height: u32,
    diffusion: i64,
    relaxation: i64,
) {
    let x = ABSOLUTE_POS_X;
    let y = ABSOLUTE_POS_Y;
    if x < width && y < height {
        // Clamped-Neumann: an edge neighbour is the cell itself (zero flux), not a wrap-around.
        let xl = select(x == 0u32, x, x - 1u32);
        let xr = select(x == width - 1u32, x, x + 1u32);
        let yu = select(y == 0u32, y, y - 1u32);
        let yd = select(y == height - 1u32, y, y + 1u32);

        let idx = (y * width + x) as usize;
        let c = f[idx];
        let up = f[(yu * width + x) as usize];
        let dn = f[(yd * width + x) as usize];
        let lf = f[(y * width + xl) as usize];
        let rt = f[(y * width + xr) as usize];

        let lap = up + dn + lf + rt - (c << 2u32); // 4*c is an exact shift on the bit pattern
        let relax = baseline[idx] - c;
        g[idx] = c + q32_mul(diffusion, lap) + q32_mul(relaxation, relax);
    }
}

/// Run `iters` canonical diffusion-and-relaxation steps on the GPU over a `width` x `height`
/// clamped-Neumann field, matching `Field::step`. `initial` and `baseline` are row-major `i64` Q32.32
/// bit patterns (each `width * height` long); `diffusion` and `relaxation` are the two coefficients
/// (Q32.32 bits). The baseline stays resident on the device across every iteration (only the field
/// ping-pongs), which is the residency the offload map calls for. Returns the final field, bit-identical
/// to the CPU `Field::step` on CUDA.
#[allow(clippy::too_many_arguments)]
pub fn gpu_field_step(
    client: &CudaClient,
    initial: &[i64],
    baseline: &[i64],
    width: u32,
    height: u32,
    iters: u32,
    diffusion: i64,
    relaxation: i64,
) -> Vec<i64> {
    let n = (width as usize) * (height as usize);
    assert_eq!(
        initial.len(),
        n,
        "gpu_field_step: initial field must cover width*height cells"
    );
    assert_eq!(
        baseline.len(),
        n,
        "gpu_field_step: baseline must cover width*height cells"
    );
    if n == 0 {
        return Vec::new();
    }
    let mut f_h = client.create_from_slice(i64::as_bytes(initial));
    let mut g_h = client.empty(n * core::mem::size_of::<i64>());
    // The baseline is a constant forcing, uploaded once and held resident across the iteration loop.
    let base_h = client.create_from_slice(i64::as_bytes(baseline));

    let tile = 16u32;
    let bx = width.div_ceil(tile);
    let by = height.div_ceil(tile);
    for _ in 0..iters {
        unsafe {
            field_step_kernel::launch::<CudaRuntime>(
                client,
                CubeCount::Static(bx, by, 1),
                CubeDim::new_3d(tile, tile, 1),
                ArrayArg::from_raw_parts(f_h.clone(), n),
                ArrayArg::from_raw_parts(base_h.clone(), n),
                ArrayArg::from_raw_parts(g_h.clone(), n),
                width,
                height,
                diffusion,
                relaxation,
            );
        }
        core::mem::swap(&mut f_h, &mut g_h);
    }
    let bytes = client.read_one_unchecked(f_h);
    i64::from_bytes(&bytes).to_vec()
}

/// Cross-tick resident context for the field stencil: the split form of [`gpu_field_step`] with
/// `iters = 1` per dispatch. The baseline is uploaded once and held resident; [`dispatch`](Self::dispatch)
/// uploads one temperature snapshot, launches [`field_step_kernel`] once, and returns WITHOUT reading
/// back; [`readback`](Self::readback) is the single CUDA-event-fenced sync point. This is what licenses
/// the cross-tick software pipeline (design `docs/working/GPU_BLUEPRINT.md`): tick N dispatches the frozen
/// post-combustion field, tick N's remaining CPU tail runs concurrently with the GPU stencil, and tick
/// N+1 reads the result at a fixed point. Determinism is unaffected by completion timing: the fence only
/// changes how long the CPU parks, never which bytes it reads (the kernel is a pointwise fixed-point map,
/// gated bit-identical to the CPU `Field::step`). All calls must issue from ONE thread, because the
/// thread-local `StreamId` fixes device order; the owning thread is captured in [`new`](Self::new) and
/// debug-asserted at each call so a dispatch and its readback can never split across two streams.
pub struct FieldResident {
    client: CudaClient,
    /// The baseline forcing, uploaded once in `new` and held resident across every tick.
    base_h: Handle,
    width: u32,
    height: u32,
    n: usize,
    /// `(input upload handle, output handle)` of a dispatched-but-unread step. BOTH are held until the
    /// fence in `readback` proves the kernel finished, so the input device buffer cannot be freed under a
    /// queued-but-not-yet-executed kernel.
    pending: Option<(Handle, Handle)>,
    /// Test-only artificial park before the readback fence (the timing-invariance injection). `None` in
    /// production; it never affects which bytes are read, only how long the CPU waits.
    readback_delay: Option<Duration>,
    /// The single owning thread, captured at construction. `dispatch` and `readback` debug-assert against
    /// it so the pair never splits across two thread-local streams.
    owner: ThreadId,
}

impl FieldResident {
    /// Upload the baseline once and capture the owning thread. `baseline` is row-major `i64` Q32.32 bits,
    /// `width * height` long.
    pub fn new(client: CudaClient, baseline: &[i64], width: u32, height: u32) -> FieldResident {
        let n = (width as usize) * (height as usize);
        assert_eq!(
            baseline.len(),
            n,
            "FieldResident: baseline must cover width*height cells"
        );
        let base_h = client.create_from_slice(i64::as_bytes(baseline));
        FieldResident {
            client,
            base_h,
            width,
            height,
            n,
            pending: None,
            readback_delay: None,
            owner: std::thread::current().id(),
        }
    }

    /// Snapshot upload plus one async launch, no readback. `temp` is the frozen post-combustion field bits;
    /// `diffusion`/`relaxation` are passed fresh from the caller's calib every tick (never cached, so a
    /// future calib change cannot desync the device path). A no-op on an empty field.
    pub fn dispatch(&mut self, temp: &[i64], diffusion: i64, relaxation: i64) {
        debug_assert_eq!(
            std::thread::current().id(),
            self.owner,
            "FieldResident::dispatch off the owning thread"
        );
        // Fail loud on a double dispatch: overwriting `pending` would drop the prior kernel's input handle
        // under a possibly-still-queued kernel, a use-after-free that could feed the readback nondeterministic
        // bytes and break replay. The caller must `readback` between dispatches; today's step_field wiring
        // does (readback at the head, dispatch after combustion), so this guards a future refactor, not a
        // live path. A single Option check per tick, cheap enough to keep in release (a memory-safety pin).
        assert!(
            self.pending.is_none(),
            "FieldResident::dispatch called with a readback still pending (would drop a live device handle)"
        );
        assert_eq!(
            temp.len(),
            self.n,
            "FieldResident::dispatch: temp must cover width*height cells"
        );
        if self.n == 0 {
            return;
        }
        let f_h = self.client.create_from_slice(i64::as_bytes(temp));
        let g_h = self.client.empty(self.n * core::mem::size_of::<i64>());
        let tile = 16u32;
        let bx = self.width.div_ceil(tile);
        let by = self.height.div_ceil(tile);
        unsafe {
            field_step_kernel::launch::<CudaRuntime>(
                &self.client,
                CubeCount::Static(bx, by, 1),
                CubeDim::new_3d(tile, tile, 1),
                ArrayArg::from_raw_parts(f_h.clone(), self.n),
                ArrayArg::from_raw_parts(self.base_h.clone(), self.n),
                ArrayArg::from_raw_parts(g_h.clone(), self.n),
                self.width,
                self.height,
                diffusion,
                relaxation,
            );
        }
        // Flush the stream so the kernel BEGINS executing on the device now, concurrently with this tick's
        // CPU tail, instead of sitting queued until the readback fence triggers it (the missing-overlap the
        // audit flagged). `flush` submits the outstanding launch without waiting for its completion, so
        // determinism is unchanged (the readback still fences on the same event); it only moves WHEN the
        // device starts the work earlier, which is the whole point of the cross-tick pipeline.
        let _ = self.client.flush();
        // Hold BOTH handles until the fence: the input buffer must outlive the queued kernel.
        self.pending = Some((f_h, g_h));
    }

    /// The single blocking sync point: a device-to-host copy plus a CUDA event wait. Returns `None` when
    /// no dispatch is pending (the first tick, or an unarmed run), so the caller falls back to the CPU
    /// step. The output is a fresh row-major `i64` Q32.32 field.
    pub fn readback(&mut self) -> Option<Vec<i64>> {
        debug_assert_eq!(
            std::thread::current().id(),
            self.owner,
            "FieldResident::readback off the owning thread"
        );
        let (in_h, out_h) = self.pending.take()?;
        if let Some(d) = self.readback_delay {
            std::thread::sleep(d); // park BEFORE the fence: proves timing cannot move the hash
        }
        let bytes = self.client.read_one_unchecked(out_h); // the fence: waits kernel + copy
        let v = i64::from_bytes(&bytes).to_vec();
        drop(in_h); // safe now: the fence proved the kernel consumed the input buffer
        Some(v)
    }

    /// Whether a dispatched step is awaiting readback.
    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// Inject an artificial park before the readback fence (test-only, the timing-invariance sweep).
    pub fn set_readback_delay(&mut self, d: Option<Duration>) {
        self.readback_delay = d;
    }
}
