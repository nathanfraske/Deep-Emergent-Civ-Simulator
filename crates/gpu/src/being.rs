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

//! The resident being-kernel (offload-map step 3): per-being physiology run over a device-resident
//! being buffer, co-resident with the field so the field never reads back per tick, the
//! resident-being-over-resident-field pattern step 3 is built around. Each kernel is bit-identical to
//! its CPU oracle on CUDA. Four pieces are here:
//! - [`gpu_body_thermal`]: the body-thermal exchange (the runner's `phase_body_exchange`), discrete
//!   Newton cooling of each being toward the field temperature at its own cell (the pinned `q32_mul`).
//! - [`gpu_metabolize`]: the homeostasis drain (`Homeostasis::metabolize` then `Stock::step`), a
//!   logistic regeneration toward capacity then a drain, held in `[0, capacity]`.
//! - [`gpu_activate`]: the controller's activation (`activate`), `clamp(sum sat_mul(w, x), -1, 1)`, the
//!   saturating sum carried in a two-limb signed 128-bit accumulator.
//! - [`gpu_sat_mul`]: the saturating Q32.32 multiply the controller and drain both use, exposed for
//!   its own gate.
//!
//! The remaining being-kernel work is the reaction-norm matvec (looping `activate` over the outputs of
//! a per-being weight matrix) and fusing all four over one resident being buffer.

use crate::prim::{checked_mul_zero, q32_div, q32_mul, sat_add, sat_mul};
use crate::stage0::CudaClient;
use cubecl::cuda::CudaRuntime;
use cubecl::prelude::*;

/// Clamp `v` into `[0, cap]` (a Fixed clamp with `cap >= 0`): `0` if `v < 0`, `cap` if `v > cap`, else
/// `v`. The reserve and the exertion both stay in such a band.
#[cube]
fn clamp_cap(v: i64, cap: i64) -> i64 {
    let lo = select(v < 0i64, 0i64, v);
    select(lo > cap, cap, lo)
}

/// The logistic per-step regeneration increment for one reserve, matching `Stock::regen_increment`:
/// zero if the capacity or the amount is non-positive; else `ratio = amount / capacity`,
/// `gap = 1 - ratio`, and the increment is `regen_rate * (amount * gap)`, each product checked (zero on
/// overflow), which keeps the intermediate `amount * gap <= amount` representable before the rate scales
/// it. The divide uses the pinned `q32_div`; a safe divisor stands in when the guard zeroes the result.
#[cube]
fn regen_increment(amount: i64, capacity: i64, regen_rate: i64) -> i64 {
    let one = 4294967296i64; // ONE
    let guard = (capacity <= 0i64) || (amount <= 0i64);
    let denom = select(capacity == 0i64, one, capacity); // never divide by zero when guarded off
    let ratio = q32_div(amount, denom);
    let gap = one - ratio;
    let occupied = checked_mul_zero(amount, gap);
    let inc = checked_mul_zero(regen_rate, occupied);
    select(guard, 0i64, inc)
}

/// One being-and-axis homeostasis drain, matching `Homeostasis::metabolize` followed by `Stock::step`:
/// the reserve regenerates logistically toward capacity, then a drain of `(base + exertion_coupling) *
/// capacity` is applied, all held in `[0, capacity]`. `idx` runs over `(being, axis)` pairs row-major
/// with `n_axes` axes per being, so `axis = idx % n_axes` and `being = idx / n_axes`. Per-axis inputs
/// (`base_drain`, `exertion_drain`) are indexed by axis, `exertion` by being, and the per-reserve state
/// (`amount`, `capacity`, `regen_rate`) by `idx`.
#[cube(launch)]
#[allow(clippy::too_many_arguments)]
fn metabolize_kernel(
    base_drain: &Array<i64>,
    exertion_drain: &Array<i64>,
    exertion: &Array<i64>,
    amount: &Array<i64>,
    capacity: &Array<i64>,
    regen_rate: &Array<i64>,
    n_axes: u32,
    out: &mut Array<i64>,
) {
    let idx = ABSOLUTE_POS;
    if idx < out.len() {
        let na = n_axes as usize;
        let axis = idx % na;
        let being = idx / na;
        let one = 4294967296i64;
        // metabolize: the drain fraction of capacity, then the draw.
        let ex = clamp_cap(exertion[being], one); // clamp(exertion, 0, 1)
        let coupling = checked_mul_zero(exertion_drain[axis], ex);
        let frac = sat_add(base_drain[axis], coupling);
        let cap = capacity[idx];
        let draw = checked_mul_zero(frac, cap);
        // stock.step: regenerate, then apply the (non-negative) draw, staying in [0, cap].
        let amt = amount[idx];
        let regen = regen_increment(amt, cap, regen_rate[idx]);
        let after_regen = clamp_cap(sat_add(amt, regen), cap);
        let drawn = select(draw < 0i64, 0i64, draw);
        out[idx] = clamp_cap(after_regen - drawn, cap);
    }
}

/// Run the homeostasis drain on the GPU for a located being population, bit-identical to
/// `Homeostasis::metabolize` + `Stock::step` on CUDA. Per-axis `base_drain`/`exertion_drain` (length
/// `n_axes`), per-being `exertion`, and per-`(being, axis)` `amount`/`capacity`/`regen_rate` (length
/// `n_beings * n_axes`, row-major by being then axis). Returns the updated reserve amounts.
#[allow(clippy::too_many_arguments)]
pub fn gpu_metabolize(
    client: &CudaClient,
    base_drain: &[i64],
    exertion_drain: &[i64],
    exertion: &[i64],
    amount: &[i64],
    capacity: &[i64],
    regen_rate: &[i64],
    n_axes: u32,
) -> Vec<i64> {
    let n = amount.len();
    assert!(n_axes >= 1, "gpu_metabolize: need at least one axis");
    let na = n_axes as usize;
    assert_eq!(
        base_drain.len(),
        na,
        "gpu_metabolize: base_drain is per axis"
    );
    assert_eq!(
        exertion_drain.len(),
        na,
        "gpu_metabolize: exertion_drain is per axis"
    );
    assert_eq!(
        capacity.len(),
        n,
        "gpu_metabolize: capacity is per (being, axis)"
    );
    assert_eq!(
        regen_rate.len(),
        n,
        "gpu_metabolize: regen_rate is per (being, axis)"
    );
    assert_eq!(
        n % na,
        0,
        "gpu_metabolize: amount length must be a multiple of n_axes"
    );
    assert_eq!(
        exertion.len(),
        n / na,
        "gpu_metabolize: exertion is per being"
    );
    if n == 0 {
        return Vec::new();
    }
    let bd = client.create_from_slice(i64::as_bytes(base_drain));
    let ed = client.create_from_slice(i64::as_bytes(exertion_drain));
    let ex = client.create_from_slice(i64::as_bytes(exertion));
    let am = client.create_from_slice(i64::as_bytes(amount));
    let ca = client.create_from_slice(i64::as_bytes(capacity));
    let rr = client.create_from_slice(i64::as_bytes(regen_rate));
    let out_h = client.empty(core::mem::size_of_val(amount));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        metabolize_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(bd.clone(), na),
            ArrayArg::from_raw_parts(ed.clone(), na),
            ArrayArg::from_raw_parts(ex.clone(), n / na),
            ArrayArg::from_raw_parts(am.clone(), n),
            ArrayArg::from_raw_parts(ca.clone(), n),
            ArrayArg::from_raw_parts(rr.clone(), n),
            n_axes,
            ArrayArg::from_raw_parts(out_h.clone(), n),
        );
    }
    i64::from_bytes(&client.read_one_unchecked(out_h)).to_vec()
}

/// The maximum term count one `activate` sums (a literal unroll bound; the launcher clamps `n_terms`).
/// The controller's largest activation is `n_in + hidden` terms; 32 covers the dev layout with headroom.
const MAX_TERMS: u32 = 32;

/// Clamp a signed 128-bit accumulator `(hi, lo)` (value `V = hi * 2^64 + lo`, two's complement) to the
/// unit interval `[-1, 1]` in Q32.32 bits, matching `acc.clamp(-1, 1)` after the controller's
/// `saturating_sum`: return `ONE` when `V > ONE`, `-ONE` when `V < -ONE`, else `V` (which then fits
/// `i64`, and its low word reinterpreted as `i64` is the value). Because the final interval is inside
/// `i64`, clamping the 128-bit sum straight to `[-1, 1]` subsumes `saturating_sum`'s prior clamp to the
/// `i64` range.
#[cube]
fn clamp_unit(hi: i64, lo: u64) -> i64 {
    let one = 4294967296i64; // ONE = 1 << 32
    let neg_one = -4294967296i64;
    let one_u = 4294967296u64; // ONE as u64
    let neg_thresh = 0xFFFF_FFFF_0000_0000u64; // 2^64 - 2^32: |V| > ONE on the negative side below this
    let lo_i = i64::cast_from(lo); // the low word reinterpreted as i64 (= V when V fits [-ONE, ONE])
    let pos_over = (hi > 0i64) || (hi == 0i64 && lo > one_u);
    let neg_over = (hi < -1i64) || (hi == -1i64 && lo < neg_thresh);
    select(pos_over, one, select(neg_over, neg_one, lo_i))
}

/// One controller activation per thread: `clamp(sum_t sat_mul(weights[t], inputs[t]), -1, 1)`, the
/// oracle's `activate`. `base = act * n_terms` locates this activation's contiguous term block. Two
/// passes keep the DSL happy: pass one writes the saturating products into a per-thread array (distinct
/// slots, so no carried accumulator crosses the `sat_mul` call); pass two sums them into a signed
/// 128-bit accumulator `(hi, lo)` with no `#[cube]` call in the loop, so the carried accumulator
/// survives. A dead term (`t >= n_terms`) contributes zero and reads a clamped index.
#[cube(launch)]
fn activate_kernel(weights: &Array<i64>, inputs: &Array<i64>, n_terms: u32, out: &mut Array<i64>) {
    let act = ABSOLUTE_POS;
    if act < out.len() {
        let nt = n_terms as usize;
        let base = act * nt;
        let mut prod = Array::<i64>::new(32usize);
        #[unroll]
        for t in 0usize..32usize {
            let live = t < nt;
            let tt = select(live, t, 0usize);
            let p = sat_mul(weights[base + tt], inputs[base + tt]);
            prod[t] = select(live, p, 0i64);
        }
        let mut hi = 0i64;
        let mut lo = 0u64;
        #[unroll]
        for t in 0usize..32usize {
            let p = prod[t];
            let p_lo = u64::cast_from(p);
            let p_hi = p >> 63u32; // sign extension: 0 or -1
            let new_lo = lo + p_lo;
            let carry = select(new_lo < lo, 1i64, 0i64);
            hi = hi + p_hi + carry;
            lo = new_lo;
        }
        out[act] = clamp_unit(hi, lo);
    }
}

/// Run a batch of controller activations on the GPU, bit-identical to the oracle `activate` on CUDA:
/// `out[a] = clamp(sum_t sat_mul(weights[a*n_terms + t], inputs[a*n_terms + t]), -1, 1)`. `weights` and
/// `inputs` are row-major `i64` Q32.32 bits, `n_terms` term pairs per activation. Returns one Q32.32
/// value per activation. `n_terms` is clamped to at most `MAX_TERMS`.
pub fn gpu_activate(
    client: &CudaClient,
    weights: &[i64],
    inputs: &[i64],
    n_terms: u32,
) -> Vec<i64> {
    assert_eq!(
        weights.len(),
        inputs.len(),
        "gpu_activate: weights and inputs differ"
    );
    assert!(
        n_terms <= MAX_TERMS,
        "gpu_activate: n_terms {n_terms} exceeds the unroll bound {MAX_TERMS}"
    );
    if n_terms == 0 || weights.is_empty() {
        return Vec::new();
    }
    let n_acts = weights.len() / (n_terms as usize);
    assert_eq!(
        n_acts * (n_terms as usize),
        weights.len(),
        "gpu_activate: length must be a multiple of n_terms"
    );
    let w_h = client.create_from_slice(i64::as_bytes(weights));
    let x_h = client.create_from_slice(i64::as_bytes(inputs));
    let out_h = client.empty(n_acts * core::mem::size_of::<i64>());
    let threads = 256u32;
    let blocks = (n_acts as u32).div_ceil(threads);
    unsafe {
        activate_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(w_h.clone(), weights.len()),
            ArrayArg::from_raw_parts(x_h.clone(), inputs.len()),
            n_terms,
            ArrayArg::from_raw_parts(out_h.clone(), n_acts),
        );
    }
    i64::from_bytes(&client.read_one_unchecked(out_h)).to_vec()
}

/// Elementwise saturating Q32.32 multiply, so the controller's saturating product (`crate::prim::sat_mul`)
/// can be gated over the full corner and overflow range against the oracle before it is used inside the
/// reaction-norm activation. See [`gpu_sat_mul`].
#[cube(launch)]
fn sat_mul_kernel(a: &Array<i64>, b: &Array<i64>, out: &mut Array<i64>) {
    let pos = ABSOLUTE_POS;
    if pos < out.len() {
        out[pos] = sat_mul(a[pos], b[pos]);
    }
}

/// Elementwise saturating Q32.32 multiply: the Fixed product when it fits, else the signed extreme
/// (`i64::MIN` on differing signs, `i64::MAX` on agreeing signs), matching the controller's `sat_mul`.
/// `a`, `b`, and the result are raw `i64` Fixed bit patterns; bit-identical to the CPU `sat_mul` on
/// CUDA. `a` and `b` must have equal length.
pub fn gpu_sat_mul(client: &CudaClient, a: &[i64], b: &[i64]) -> Vec<i64> {
    assert_eq!(a.len(), b.len(), "gpu_sat_mul: mismatched input lengths");
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
        sat_mul_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(a_h.clone(), n),
            ArrayArg::from_raw_parts(b_h.clone(), n),
            ArrayArg::from_raw_parts(out_h.clone(), n),
        );
    }
    i64::from_bytes(&client.read_one_unchecked(out_h)).to_vec()
}

/// One being's body-thermal exchange, matching the runner's `phase_body_exchange`:
/// `next = bt + exchange * (env - bt)`, where `bt` is the being's body temperature, `env` is the
/// resident field temperature at the being's cell, and `exchange` is the Newton-cooling rate. The
/// multiply is the pinned `q32_mul` and the difference and sum are exact integer arithmetic on the
/// Q32.32 bit patterns.
#[cube(launch)]
fn body_thermal_kernel(
    field: &Array<i64>,
    cell: &Array<u32>,
    body_temp: &mut Array<i64>,
    exchange: i64,
) {
    let pos = ABSOLUTE_POS;
    if pos < body_temp.len() {
        let bt = body_temp[pos];
        let env = field[cell[pos] as usize];
        body_temp[pos] = bt + q32_mul(exchange, env - bt);
    }
}

/// Run the body-thermal exchange on the GPU for a located being population, bit-identical to the CPU
/// `phase_body_exchange` on CUDA. `field` is the resident field (row-major Q32.32 bits), `cell[i]` the
/// row-major cell index of being `i`, `body_temp[i]` its body temperature (Q32.32 bits), and `exchange`
/// the Newton-cooling rate (Q32.32 bits). Returns the updated body temperatures. Each `cell[i]` must be
/// a valid index into `field` (the runner only exchanges for a being the located index places on the
/// grid, so an off-grid being is filtered before this call).
pub fn gpu_body_thermal(
    client: &CudaClient,
    field: &[i64],
    cell: &[u32],
    body_temp: &[i64],
    exchange: i64,
) -> Vec<i64> {
    let n = body_temp.len();
    assert_eq!(cell.len(), n, "gpu_body_thermal: one cell index per being");
    if n == 0 {
        return Vec::new();
    }
    // The kernel does a raw `field[cell[pos]]` load, so an out-of-range cell index is undefined on the
    // device. The runner only exchanges heat for a being the located index places on the grid, so a
    // valid index is the caller's contract; make it loud here rather than a silent device read (a
    // hardening the blind audit of this kernel surfaced).
    assert!(
        cell.iter().all(|&c| (c as usize) < field.len()),
        "gpu_body_thermal: every cell index must be within the field ({} cells)",
        field.len()
    );
    let field_h = client.create_from_slice(i64::as_bytes(field));
    let cell_h = client.create_from_slice(u32::as_bytes(cell));
    let bt_h = client.create_from_slice(i64::as_bytes(body_temp));
    let threads = 256u32;
    let blocks = (n as u32).div_ceil(threads);
    unsafe {
        body_thermal_kernel::launch::<CudaRuntime>(
            client,
            CubeCount::Static(blocks, 1, 1),
            CubeDim::new_1d(threads),
            ArrayArg::from_raw_parts(field_h.clone(), field.len()),
            ArrayArg::from_raw_parts(cell_h.clone(), n),
            ArrayArg::from_raw_parts(bt_h.clone(), n),
            exchange,
        );
    }
    let bytes = client.read_one_unchecked(bt_h);
    i64::from_bytes(&bytes).to_vec()
}
