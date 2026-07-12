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

//! The surface-transport DRIVER KERNELS (genesis-forward Stage 3, the surface lane). Design in
//! `docs/working/GENESIS_STAGE3_SURFACE_TRANSPORT_SUBSTRATE.md`. Each kernel reads the continuous transport
//! physics of one driver and produces the per-column elevation change the snapshot-apply reconciliation
//! ([`crate::surface_transport::reconcile_column`]) writes as the geological delta. The kernels are the fixed
//! Rust the data-defined driver rows ([`crate::surface_transport::DriverRow`]) invoke by
//! [`crate::surface_transport::TransportKernelId`]. Off the run path until a genesis pass arms a driver, so
//! declaring the kernels is byte-neutral.
//!
//! The four build-now kernels land one at a time. GRAVITY-DOWNSLOPE ([`hillslope_diffuse`]) is the first, and it
//! sets the local slope the fluid-shear and solid-solvent drivers read. FLUID-SHEAR ([`fluid_shear`]) is the
//! second, the moving-fluid entrainment and transport driver, the SOURCE half of the fluvial budget.
//! THERMAL-CHEMICAL ALTERATION ([`thermal_chemical_alter`]) is the third, the in-place weathering that dissolves
//! rock into the dissolved-load reservoir and fractures it into the mobile grains the transport drivers move (the
//! grain source they presuppose). DEPOSITION ([`deposit`]) is the fourth, the conservation SINK that settles a
//! transported source where transport capacity drops.
//!
//! The conservation each kernel proves is LOCAL to its own pass: the hillslope flux conserves total elevation
//! exactly, `deposit` conserves the `entrained` source it is GIVEN (total deposited equals the total of its input
//! source), and each reservoir transfer conserves the four-account total. The CROSS-WRITER budget closure (many
//! drivers removing from one contested column) does NOT follow automatically from those local identities: a
//! source driver bounds its entrainment by the slope drop and routes that full mass downstream, while
//! [`crate::surface_transport::reconcile_column`] clamps the contested column's honored removal to its snapshot
//! mass, so the arming step MUST route only the reconciled (honored) removal into the sink, never the raw
//! pre-reconcile demand. That composition contract (reconcile the removals against the snapshot first, feed the
//! honored removal as the deposition source) is specified in the design and is the arming step's responsibility;
//! it is not yet built or tested here, so the whole-budget closure across contending writers is a stated
//! obligation, not a proven property of these byte-neutral kernels.

use civsim_core::Fixed;
use civsim_world::flood::priority_flood;
use civsim_world::solve::{fixed_cap_solve, SolveOutcome};

use crate::calibration::CalibrationError;

/// The largest stable per-step diffusion number for the four-connected explicit scheme. The explicit update
/// keeps a cell's self-coefficient `1 - factor * n` non-negative only for `factor <= 1 / n`; the four-connected
/// interior (`n = 4`) is the tightest, so a factor above `1/4` grows the residual rather than relaxing it.
fn max_diffusion_factor() -> Fixed {
    Fixed::from_ratio(1, 4)
}

/// The GRAVITY-DOWNSLOPE hillslope-diffusion kernel: gravity moves regolith down a slope with no transporting
/// fluid, smoothing the terrain toward the diffusive steady state over deep genesis time. It relaxes the
/// elevation field by the flux-based discrete diffusion (each adjacent pair of columns exchanges sediment in
/// proportion to their elevation difference), driven to a bounded steady state by C's
/// [`fixed_cap_solve`]. This kernel SETS the local slope the fluid-shear and solid-solvent drivers read.
///
/// It CONSERVES total elevation EXACTLY under fixed-point arithmetic: each undirected edge computes ONE rounded
/// flux and subtracts it from one column while adding the same value to the other, so what one column loses
/// another gains with no rounding leak, and no flux crosses the grid boundary (a reflecting, no-flux edge). The
/// solid-column mass is redistributed, never created or lost.
///
/// `factor` is the dimensionless per-step diffusion number `D * dt / dx^2`: the reserved hillslope diffusivity
/// `D` (a per-material transport coefficient with an error band, calibrated against measured hillslope
/// relaxation, surfaced-with-basis and never fabricated), over the genesis timestep and the grid spacing. It
/// must lie in `(0, 1/4]` for the explicit scheme to stay bounded; a factor outside that range is refused
/// fail-loud (a [`CalibrationError::BadValue`]), never a silent clamp. The upper end `1/4` is the neutral-
/// stability boundary: the field stays bounded and deterministic there, but the highest-frequency (checkerboard)
/// mode is undamped rather than relaxing, so strict monotone relaxation of every mode wants `factor < 1/4`, and
/// the boundary is admitted (not forbidden) because it is bounded, not because it relaxes fastest. The nonlinear
/// threshold-failure
/// (landslide) branch above a reserved critical-slope angle is a deferred refinement of this linear creep core.
///
/// Deterministic fixed-point arithmetic (Principle 3); the residual is the exact integer max elevation change so
/// the fixed-cap solve is worker-invariant and replays bit-for-bit (Principle 10). `elevation` is the row-major
/// field of `width * height` columns; a length mismatch is refused fail-loud.
pub fn hillslope_diffuse(
    elevation: Vec<Fixed>,
    width: usize,
    height: usize,
    factor: Fixed,
    cap: u32,
    threshold: u64,
) -> Result<SolveOutcome<Vec<Fixed>>, CalibrationError> {
    if factor <= Fixed::ZERO || factor > max_diffusion_factor() {
        return Err(CalibrationError::BadValue {
            id: "surface.hillslope_diffusion_factor".to_string(),
            detail: format!(
                "the diffusion number D*dt/dx^2 must lie in (0, 1/4] for stability; got {}",
                factor.to_f64_lossy()
            ),
        });
    }
    if elevation.len() != width.saturating_mul(height) {
        return Err(CalibrationError::BadValue {
            id: "surface.hillslope_diffusion_grid".to_string(),
            detail: format!(
                "the elevation length {} must equal width*height {}",
                elevation.len(),
                width.saturating_mul(height)
            ),
        });
    }
    Ok(fixed_cap_solve(
        elevation,
        cap,
        threshold,
        |z| diffusion_step(z, width, height, factor),
        |a, b| max_change_bits(a, b),
    ))
}

/// One explicit hillslope-diffusion pass, flux-based so it conserves total elevation exactly. Every east and
/// south edge (visited once) exchanges the flux `factor * (z_high - z_low)` computed from the SNAPSHOT `z`, not
/// the partially-updated field, so the pass is order-independent (the Jacobi scheme). The flux is subtracted
/// from the higher column and added to the lower, so the total is unchanged bit-for-bit.
fn diffusion_step(z: &[Fixed], width: usize, height: usize, factor: Fixed) -> Vec<Fixed> {
    let mut next = z.to_vec();
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            if x + 1 < width {
                let j = i + 1;
                let flux = factor * (z[i] - z[j]);
                next[i] -= flux;
                next[j] += flux;
            }
            if y + 1 < height {
                let j = i + width;
                let flux = factor * (z[i] - z[j]);
                next[i] -= flux;
                next[j] += flux;
            }
        }
    }
    next
}

/// The exact integer residual for the fixed-cap solve: the maximum absolute change in any column's raw
/// fixed-point bits between the previous and the new field. An integer comparison, so the convergence decision
/// carries no platform-dependent float tolerance.
fn max_change_bits(a: &[Fixed], b: &[Fixed]) -> u64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.to_bits().abs_diff(y.to_bits()))
        .max()
        .unwrap_or(0)
}

/// The largest grid the fluid-shear kernel accepts, so a cell's drainage area (a count bounded by the cell
/// total) converts to `Fixed` through `from_ratio(area, 1)` without the `i128 << 32` intermediate exceeding the
/// `i64` mantissa. `2^31` cells is a 46340-square grid, far above any genesis pass, so the bound is a
/// correctness guard, never a live limit. It bounds ONLY the area-to-`Fixed` conversion; the transport PRODUCTS
/// (the shear proxy and the capacity) are guarded separately by [`mul_or_overflow`], because a `Fixed` multiply
/// ends in a narrowing cast that would wrap silently rather than fail loud.
const MAX_FLUID_SHEAR_CELLS: usize = 1usize << 31;

/// Multiply two `Fixed` values, failing loud on a Q32.32 overflow. The `*` operator's `Fixed::mul` ends in an
/// `as i64` narrowing cast, and a cast is NOT caught by overflow-checks, so an out-of-range product would wrap
/// SILENTLY to a wrong value rather than panic. The transport laws can reach extreme magnitudes on a pathological
/// input (a huge drainage area times a steep slope, an enormous reserved coefficient), and a silent wrap would
/// corrupt the deterministic result, so every driver product routes through this checked form and surfaces an
/// overflow as a [`CalibrationError`] the caller must handle.
fn mul_or_overflow(a: Fixed, b: Fixed, id: &str) -> Result<Fixed, CalibrationError> {
    a.checked_mul(b).ok_or_else(|| CalibrationError::BadValue {
        id: id.to_string(),
        detail: format!(
            "the product overflowed Q32.32: {} * {}",
            a.to_f64_lossy(),
            b.to_f64_lossy()
        ),
    })
}

/// Divide two `Fixed` values, failing loud on a zero divisor or a Q32.32 overflow. Like [`mul_or_overflow`], this
/// guards the narrowing cast in `Fixed::div` that would otherwise wrap a division by a near-zero denominator
/// (a sub-microkelvin temperature in the Arrhenius exponent) into a silent wrong value.
fn div_or_overflow(a: Fixed, b: Fixed, id: &str) -> Result<Fixed, CalibrationError> {
    a.checked_div(b).ok_or_else(|| CalibrationError::BadValue {
        id: id.to_string(),
        detail: format!(
            "the quotient overflowed Q32.32 or divided by zero: {} / {}",
            a.to_f64_lossy(),
            b.to_f64_lossy()
        ),
    })
}

/// An EXACT-ROOT EXPONENT: a rational power `num/den` restricted to the family that reduces to a proven
/// GPU-canon-deterministic kernel today. The stream-power incision law reads its area and slope exponents as this
/// data (on the driver row when a genesis pass arms it), so a non-Earth fluid whose incision law has a different
/// exponent in the buildable family is a data row rather than a rewrite (Principle 11, admit-the-alien). The
/// hardcoded Terran fluvial pair `m = 1/2`, `n = 1` becomes the reserved [`ExactRootExponent::SQRT`] and
/// [`ExactRootExponent::LINEAR`] default, a per-world and per-driver datum. The general arbitrary-exponent
/// fractional power is the deferred GPU-canon primitive (task #45), which extends the buildable set inside
/// [`apply_exact_root`] without touching the driver kernels, the same fixed-mechanism / growing-membership shape
/// the value and semantic substrates use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactRootExponent {
    /// The exponent numerator.
    pub num: u32,
    /// The exponent denominator (never zero).
    pub den: u32,
}

impl ExactRootExponent {
    /// The linear (identity) exponent `1/1`, the first-power slope term.
    pub const LINEAR: ExactRootExponent = ExactRootExponent { num: 1, den: 1 };
    /// The square-root exponent `1/2`, the half-power discharge term (the exact integer [`Fixed::sqrt`]).
    pub const SQRT: ExactRootExponent = ExactRootExponent { num: 1, den: 2 };

    /// Build an exponent `num/den`. A zero denominator is not a valid exponent, so it is refused fail-loud; the
    /// buildability of the resulting power is checked at application in [`apply_exact_root`], not here (a row may
    /// carry an exponent the current kernel cannot yet build, which surfaces as a fail-loud arming error rather
    /// than a silent fallback).
    pub fn new(num: u32, den: u32) -> Result<ExactRootExponent, CalibrationError> {
        if den == 0 {
            return Err(CalibrationError::BadValue {
                id: "surface.exact_root_exponent".to_string(),
                detail: format!("the exponent denominator must be non-zero; got {num}/{den}"),
            });
        }
        Ok(ExactRootExponent { num, den })
    }
}

/// Apply an [`ExactRootExponent`] to a non-negative `Fixed` base, over the family that reduces to a proven
/// GPU-canon-deterministic kernel today: the identity (`1/1`), the exact integer square root (`1/2`), and the
/// small integer powers (`2/1`, `3/1`) via the checked multiply. An exponent outside that buildable family (a
/// cube root, an arbitrary fractional power) is refused fail-loud as deferred to the GPU-canon fractional-power
/// primitive (task #45), never silently approximated: this is where #45 will EXTEND the family, so the driver
/// kernels that call this stay unchanged. A negative base under a root has no real value and is refused
/// fail-loud; the transport bases (a drainage area, a slope) are non-negative.
pub fn apply_exact_root(
    base: Fixed,
    exponent: ExactRootExponent,
) -> Result<Fixed, CalibrationError> {
    let ExactRootExponent { num, den } = exponent;
    if den > 1 && base < Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.exact_root_negative_base".to_string(),
            detail: format!(
                "a root (den {den}) of a negative base {} has no real value",
                base.to_f64_lossy()
            ),
        });
    }
    match (num, den) {
        (1, 1) => Ok(base),
        (1, 2) => Ok(base.sqrt()),
        (2, 1) => mul_or_overflow(base, base, "surface.exact_root_square"),
        (3, 1) => {
            let sq = mul_or_overflow(base, base, "surface.exact_root_cube")?;
            mul_or_overflow(sq, base, "surface.exact_root_cube")
        }
        _ => Err(CalibrationError::BadValue {
            id: "surface.exact_root_unbuildable".to_string(),
            detail: format!(
                "the exponent {num}/{den} is not in the GPU-canon-buildable exact-root family (1/1, 1/2, 2/1, 3/1); deferred to the general fractional-power primitive (task #45)"
            ),
        }),
    }
}

/// The stream-power INCISION PROCESS MODEL: the physical rule by which flow incises rock. Composed with the
/// channel hydraulic geometry and the flow resistance it FIXES the stream-power exponents `m` and `n`, so those
/// exponents DERIVE from the physics rather than being authored data (the owner's short-reserved-list bar). The
/// three standard detachment-limited models (Whipple and Tucker 1999). This is a fixed FLOOR set of physically
/// grounded incision rules, the discrete choice the exponent derivation reads; a nonlinear incision exponent or a
/// different flow-resistance law is a floor extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncisionProcessModel {
    /// Incision linear in the bed SHEAR STRESS (`tau = rho * g * h * S`) with Manning flow resistance. Derives
    /// `m = c * (1 - b) * 3/5`, `n = 7/10` (for Mirror `b = 1/2`, `c = 1`: `m = 3/10`, `n = 7/10`, outside the
    /// exact-root family so deferred to task #45 at application until the general fractional power lands).
    ShearStressManning,
    /// Incision linear in the UNIT STREAM POWER (`omega = rho * g * Q * S / w`, power per unit bed area). Derives
    /// `m = c * (1 - b)`, `n = 1` (Mirror: `m = 1/2`, `n = 1`, the exact-root fluvial default).
    UnitStreamPower,
    /// Incision linear in the TOTAL STREAM POWER (`Omega = rho * g * Q * S`, power per unit channel length).
    /// Derives `m = c`, `n = 1` (Mirror: `m = 1`, `n = 1`).
    TotalStreamPower,
}

/// The greatest common divisor, for reducing a derived exponent to lowest terms.
fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Reduce a rational `num/den` to an [`ExactRootExponent`] in lowest terms, failing loud on a zero denominator or
/// a term that does not fit `u32`.
fn reduce_exponent(num: u64, den: u64, id: &str) -> Result<ExactRootExponent, CalibrationError> {
    if den == 0 {
        return Err(CalibrationError::BadValue {
            id: id.to_string(),
            detail: "the derived exponent has a zero denominator".to_string(),
        });
    }
    let g = gcd(num, den).max(1);
    let (n, d) = (num / g, den / g);
    if n > u32::MAX as u64 || d > u32::MAX as u64 {
        return Err(CalibrationError::BadValue {
            id: id.to_string(),
            detail: format!("the derived exponent {n}/{d} does not fit u32"),
        });
    }
    ExactRootExponent::new(n as u32, d as u32)
}

/// DERIVE the stream-power exponents `(m, n)` from the incision PROCESS MODEL and the channel scalings, so the
/// driver-row exponents are derived rather than authored. `b` is the width-discharge hydraulic-geometry exponent
/// (`w ~ Q^b`, near 1/2) and `c` is the discharge-area exponent (`Q ~ A^c`, near 1 under uniform runoff), each a
/// rational `(num, den)`. The bed shear stress, water conservation, the flow resistance, and these two scalings
/// reduce the incision rule to `E = K * A^m * S^n` with `m` and `n` fixed by the model (Whipple and Tucker 1999):
/// so the exponents leave the per-world reserved list, and the only residual is the discrete process-model choice
/// plus the near-universal `b` and `c` (themselves a deeper-derivation candidate through channel equilibrium).
///
/// The Mirror unit-stream-power case with `b = 1/2`, `c = 1` derives `(SQRT, LINEAR)`, reproducing the fluvial
/// default, so wiring this derivation is byte-neutral. The shear-stress case derives exponents outside the
/// buildable exact-root family (Mirror: 3/10, 7/10), correct as data but refused fail-loud at kernel application
/// ([`apply_exact_root`]) until the general fractional-power primitive (task #45) lands. `b` must lie in `(0, 1)`
/// and `c` must be positive; otherwise the scaling is unphysical and refused fail-loud.
pub fn stream_power_exponents(
    model: IncisionProcessModel,
    b: (u32, u32),
    c: (u32, u32),
) -> Result<(ExactRootExponent, ExactRootExponent), CalibrationError> {
    let (b_num, b_den) = (u64::from(b.0), u64::from(b.1));
    let (c_num, c_den) = (u64::from(c.0), u64::from(c.1));
    if b_den == 0 || b_num == 0 || b_num >= b_den {
        return Err(CalibrationError::BadValue {
            id: "surface.stream_power_hydraulic_geometry".to_string(),
            detail: format!(
                "the width-discharge exponent b = {}/{} must lie in (0, 1)",
                b.0, b.1
            ),
        });
    }
    if c_den == 0 || c_num == 0 {
        return Err(CalibrationError::BadValue {
            id: "surface.stream_power_discharge".to_string(),
            detail: format!(
                "the discharge-area exponent c = {}/{} must be positive",
                c.0, c.1
            ),
        });
    }
    // (1 - b) = (b_den - b_num) / b_den, positive because b lies in (0, 1).
    let one_minus_b_num = b_den - b_num;
    let id = "surface.stream_power_exponent";
    match model {
        // m = c * (1 - b), n = 1.
        IncisionProcessModel::UnitStreamPower => Ok((
            reduce_exponent(c_num * one_minus_b_num, c_den * b_den, id)?,
            ExactRootExponent::LINEAR,
        )),
        // m = c, n = 1.
        IncisionProcessModel::TotalStreamPower => Ok((
            reduce_exponent(c_num, c_den, id)?,
            ExactRootExponent::LINEAR,
        )),
        // m = c * (1 - b) * 3/5, n = 7/10 (Manning flow resistance).
        IncisionProcessModel::ShearStressManning => Ok((
            reduce_exponent(c_num * one_minus_b_num * 3, c_den * b_den * 5, id)?,
            reduce_exponent(7, 10, id)?,
        )),
    }
}

/// DERIVE the entrainment threshold, the critical bed SHEAR STRESS at which a grain begins to move, from the
/// Shields relation `tau_c = shields_number * (rho_s - rho_f) * g * d`. This is the physical threshold behind the
/// fluid-shear driver's `theta`, derived rather than authored: the grain density `rho_s` and the fluid density
/// `rho_f` are composition-derived (the petrology and fluid-property data), the gravity `g` is planetary geometry,
/// the grain size `d` is the fracturing driver's product, and the ONLY reserved input is the dimensionless
/// `shields_number`, a universal function of the grain Reynolds number near 0.045 in the rough-turbulent limit,
/// reserved once with basis rather than per-world. So the entrainment threshold leaves the per-world reserved
/// list. The liquid-versus-gas difference is data here: a dense liquid and a thin gas differ only through their
/// `rho_f`, not a code branch.
///
/// The buoyant excess density `rho_s - rho_f` must be positive: a grain no denser than its fluid is not entrained
/// from the bed by shear (it floats, a different transport mode), so that case is refused fail-loud rather than
/// returning a nonsensical non-positive threshold. The reserved number, the gravity, and the grain size must be
/// positive. The products are checked so an extreme input fails loud rather than wrapping silently. The result is
/// a critical shear STRESS; mapping it to the driver's shear-proxy `theta` is the arming step, where the
/// proxy-to-stress constants are known.
pub fn shields_critical_shear_stress(
    shields_number: Fixed,
    grain_density: Fixed,
    fluid_density: Fixed,
    gravity: Fixed,
    grain_size: Fixed,
) -> Result<Fixed, CalibrationError> {
    if shields_number <= Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.shields_number".to_string(),
            detail: format!(
                "the dimensionless Shields number must be positive; got {}",
                shields_number.to_f64_lossy()
            ),
        });
    }
    if gravity <= Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.shields_gravity".to_string(),
            detail: format!(
                "the surface gravity must be positive; got {}",
                gravity.to_f64_lossy()
            ),
        });
    }
    if grain_size <= Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.shields_grain_size".to_string(),
            detail: format!(
                "the grain size must be positive; got {}",
                grain_size.to_f64_lossy()
            ),
        });
    }
    if grain_density <= fluid_density {
        return Err(CalibrationError::BadValue {
            id: "surface.shields_buoyancy".to_string(),
            detail: format!(
                "a grain no denser than its fluid is not bed-entrained by shear (grain {} <= fluid {}); a buoyant grain is a different transport mode",
                grain_density.to_f64_lossy(),
                fluid_density.to_f64_lossy()
            ),
        });
    }
    let excess_density = grain_density - fluid_density;
    let t1 = mul_or_overflow(shields_number, excess_density, "surface.shields_threshold")?;
    let t2 = mul_or_overflow(t1, gravity, "surface.shields_threshold")?;
    mul_or_overflow(t2, grain_size, "surface.shields_threshold")
}

/// The result of one [`fluid_shear`] pass: the flow routing, the drainage area, the entrained mass each column
/// gives up, and the sediment each cell carries downstream. The kernel is the SOURCE half of the fluvial budget:
/// it does not lower the elevation ledger (the snapshot-apply reconciliation applies `entrained` as the column
/// delta) and it does not settle the load (deposition, a later driver, is the sink that consumes `carried_load`).
/// Its conservation is the exact bookkeeping identity the pass guarantees: the total `entrained` mass equals the
/// total `carried_load` arriving at the drainage outlets, so no mass is created or lost while it is in transit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidShearPass {
    /// The downstream receiver of each cell (from `priority_flood`); an outlet receives itself.
    pub receiver: Vec<usize>,
    /// The drainage area of each cell, the count of cells draining through it (itself included), the discharge
    /// proxy `A` the transport law reads. Uniform runoff (each cell contributes one unit); a spatially-varying
    /// runoff is the same accumulation over a per-cell runoff field, a data extension, not a rewrite.
    pub drainage_area: Vec<i64>,
    /// The mass each column gives up to entrainment this pass (`E_i`), the amount the reconciliation lowers the
    /// column by. Non-negative and capped at the column's drop to its receiver (the base-level guard).
    pub entrained: Vec<Fixed>,
    /// The sediment each cell carries downstream: its own entrained mass plus every upstream cell's, accumulated
    /// along the receivers. The value at an outlet is the total sediment its basin delivers, the load deposition
    /// settles. Summing over the outlets recovers the total entrained mass exactly (the conservation identity).
    pub carried_load: Vec<Fixed>,
}

/// Accumulate a per-cell quantity downstream along the `receiver` forest: the returned value at each cell is its
/// own initial value plus every upstream cell's, gathered along the flow network. Deterministic and order-
/// independent by construction (Principle 3, Principle 10): it processes cells in Kahn topological order (a cell
/// is folded into its receiver only once ALL of its own contributors are folded in, so its value is final at
/// that point), and the fold is a commutative, exact `Add`, so the result is a pure function of `initial` and
/// `receiver` regardless of the ready-queue order. The receiver forest has no cycle (a `priority_flood`
/// guarantee), so every cell is processed exactly once.
fn accumulate_downstream<T>(initial: Vec<T>, receiver: &[usize]) -> Vec<T>
where
    T: Copy + std::ops::Add<Output = T>,
{
    let n = receiver.len();
    let mut indegree = vec![0usize; n];
    for (i, &r) in receiver.iter().enumerate() {
        if r != i {
            indegree[r] += 1;
        }
    }
    let mut acc = initial;
    // The ridge cells (nothing drains into them) are the roots of the topological order, collected in index
    // order so the processing is a fixed, reproducible sequence.
    let mut ready: Vec<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut head = 0;
    while head < ready.len() {
        let c = ready[head];
        head += 1;
        let r = receiver[c];
        if r != c {
            acc[r] = acc[r] + acc[c];
            indegree[r] -= 1;
            if indegree[r] == 0 {
                ready.push(r);
            }
        }
    }
    // The caller (fluid_shear) always supplies a priority_flood receiver, which is acyclic, so every cell is
    // processed; the assert documents that invariant (a cycle would leave cells unfolded and the accumulation
    // incomplete). The public deposit kernel, which takes a caller-supplied receiver, guards this fail-loud.
    debug_assert_eq!(
        head, n,
        "the receiver forest must be acyclic so every cell accumulates"
    );
    acc
}

/// The FLUID-SHEAR driver: a moving fluid (a condensed liquid or an ambient gas) exerting boundary shear that
/// entrains grains and carries them downstream, the SOURCE half of the fluvial mass budget. One kernel spans the
/// liquid and the gas cases: the exact-root stream-power form is identical, and the liquid-versus-gas difference
/// lives in the reserved data (a dense liquid entrains at a lower threshold than a thin gas) and in the routing
/// forcing, never in a hardcoded fluid branch. This pass uses the gravity-and-topography routing
/// (`priority_flood`), so it is the fluvial (and gravity-driven density-current) case; the wind-routed aeolian
/// case is the same kernel with a pressure-driven routing forcing, deferred until the atmospheric-flow field
/// lands. Keying the threshold on the fluid's property data admits the alien: a Titan methane river is this
/// kernel with a different reserved threshold and fluid-density property, not a rewrite.
///
/// The transport law is the exact-root stream-power incision, built entirely in the GPU-canon exact-root
/// exponents (no arbitrary fractional power). Each cell's shear proxy is `sqrt(A) * S`: the drainage area `A` (a
/// discharge proxy under uniform runoff) under the exact integer `Fixed::sqrt` (the resolved half power), times
/// the local slope `S` to the receiver (the exact first power). Entrainment happens only above the reserved
/// threshold `theta` (the Shields or Bagnold critical shear, in the shear-proxy units), and the entrained mass is
/// `erodibility * (sqrt(A) * S - theta)` capped at the column's drop to its receiver (the base-level guard, so
/// erosion never inverts the slope and opens a new pit). The entrained mass is routed downstream into
/// `carried_load`, and the total carried to the outlets equals the total entrained (the conservation identity).
///
/// The reserved values, surfaced-with-basis and never fabricated: `erodibility` (the stream-power coefficient,
/// an empirical rate with a notoriously wide error band, per fluid and lithology, calibrated against measured
/// incision rates) and `theta` (the entrainment threshold, the Shields or Bagnold critical shear folded into the
/// shear-proxy units, per fluid, so the liquid-versus-gas difference is data). Both are read from the driver
/// row's parameters by name on the run path, failing loud on an unset value. Both must be non-negative; a
/// negative value is refused fail-loud ([`CalibrationError::BadValue`]), never a silent clamp. `zero` erodibility
/// or an unreachable threshold is a valid inert opt-out (the pass entrains nothing).
///
/// Deterministic fixed-point arithmetic (Principle 3); the routing, the accumulation, and the law are pure
/// functions of the inputs and worker-invariant (Principle 10). `elevation` is the row-major field of
/// `width * height` columns; a length mismatch or an over-large grid is refused fail-loud.
///
/// This is the fluvial default (`m = 1/2`, `n = 1`); [`fluid_shear_with_exponents`] takes the exponents as data
/// for a fluid whose incision law differs.
pub fn fluid_shear(
    elevation: &[Fixed],
    width: usize,
    height: usize,
    erodibility: Fixed,
    theta: Fixed,
) -> Result<FluidShearPass, CalibrationError> {
    fluid_shear_with_exponents(
        elevation,
        width,
        height,
        erodibility,
        theta,
        ExactRootExponent::SQRT,
        ExactRootExponent::LINEAR,
    )
}

/// The exponent-configurable form of [`fluid_shear`]: the stream-power incision law with its AREA exponent `m`
/// and SLOPE exponent `n` read as [`ExactRootExponent`] data rather than the hardcoded `m = 1/2`, `n = 1`. The
/// arming step passes the driver row's reserved exponents here, so a non-Earth fluid whose incision law has a
/// different exponent in the GPU-canon-buildable exact-root family is a data row. The shear proxy is
/// `A^m * S^n` through [`apply_exact_root`]; an exponent outside the buildable family fails loud (deferred to the
/// general fractional-power primitive, task #45). The Mirror fluvial default `SQRT`, `LINEAR` reproduces the
/// original `sqrt(A) * S` exactly, so [`fluid_shear`] is byte-identical.
#[allow(clippy::too_many_arguments)]
pub fn fluid_shear_with_exponents(
    elevation: &[Fixed],
    width: usize,
    height: usize,
    erodibility: Fixed,
    theta: Fixed,
    area_exponent: ExactRootExponent,
    slope_exponent: ExactRootExponent,
) -> Result<FluidShearPass, CalibrationError> {
    if erodibility < Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.fluid_shear_erodibility".to_string(),
            detail: format!(
                "the stream-power erodibility must be non-negative; got {}",
                erodibility.to_f64_lossy()
            ),
        });
    }
    if theta < Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "surface.fluid_shear_entrainment_threshold".to_string(),
            detail: format!(
                "the entrainment threshold must be non-negative; got {}",
                theta.to_f64_lossy()
            ),
        });
    }
    let n = width.saturating_mul(height);
    if elevation.len() != n {
        return Err(CalibrationError::BadValue {
            id: "surface.fluid_shear_grid".to_string(),
            detail: format!(
                "the elevation length {} must equal width*height {}",
                elevation.len(),
                n
            ),
        });
    }
    if n >= MAX_FLUID_SHEAR_CELLS {
        return Err(CalibrationError::BadValue {
            id: "surface.fluid_shear_grid".to_string(),
            detail: format!(
                "the grid cell count {n} must be below 2^31 so a drainage area converts to Fixed exactly"
            ),
        });
    }

    // Route the flow: priority_flood over the raw fixed-point bits (a monotone key, so the ordering is the
    // elevation ordering) gives each cell's downstream receiver, filling pits so flow is defined everywhere.
    let bits: Vec<i64> = elevation.iter().map(|z| z.to_bits()).collect();
    let receiver = priority_flood(width, height, &bits).receiver;

    // The drainage area: accumulate a unit per cell downstream (the discharge proxy under uniform runoff).
    let drainage_area = accumulate_downstream(vec![1i64; n], &receiver);

    // The entrainment per cell: erodibility * (A^m * S^n - theta), above the threshold, capped at the drop to the
    // receiver so erosion never inverts the slope. The area exponent `m` and the slope exponent `n` are the
    // driver row's [`ExactRootExponent`] data; the Mirror fluvial default (m = 1/2 via the exact integer
    // `Fixed::sqrt`, n = 1 linear) reproduces the standard stream-power form. An exponent outside the buildable
    // family fails loud (deferred to the general fractional-power primitive, task #45).
    let mut entrained = vec![Fixed::ZERO; n];
    for i in 0..n {
        let r = receiver[i];
        // The slope to the receiver on the original terrain; zero at an outlet and inside a filled pit (a lake
        // does not incise), so entrainment there is zero.
        let drop = elevation[i] - elevation[r];
        if drop <= Fixed::ZERO {
            continue;
        }
        let area = Fixed::from_ratio(drainage_area[i], 1);
        // A^m * S^n, both exact-root exponents; the product is checked so an extreme area-slope cannot wrap.
        let a_term = apply_exact_root(area, area_exponent)?;
        let s_term = apply_exact_root(drop, slope_exponent)?;
        let shear = mul_or_overflow(a_term, s_term, "surface.fluid_shear_shear_proxy")?;
        if shear <= theta {
            continue;
        }
        let capacity = mul_or_overflow(erodibility, shear - theta, "surface.fluid_shear_capacity")?;
        // The base-level guard: never erode below the receiver in one pass (never open a new pit).
        entrained[i] = if capacity < drop { capacity } else { drop };
    }

    // Route the entrained mass downstream: each cell carries its own plus all upstream entrainment to its outlet.
    let carried_load = accumulate_downstream(entrained.clone(), &receiver);

    Ok(FluidShearPass {
        receiver,
        drainage_area,
        entrained,
        carried_load,
    })
}

/// The result of one [`thermal_chemical_alter`] pass: the two limbs of in-place bedrock alteration. `dissolved`
/// is the mass each column gives up into solution (the `ColumnSolid` to `DissolvedLoad` reservoir move the
/// snapshot-apply reconciliation and the [`crate::surface_transport::SurfaceMassBudget`] transfer carry out).
/// `grains_produced` is the intact bedrock each column converts to mobile grains by thermal and frost
/// fracturing; that mass stays in the solid column (no reservoir move) but becomes the transportable stock the
/// gravity-downslope and fluid-shear drivers entrain, so this limb is the grain SOURCE those drivers presuppose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermalChemicalPass {
    /// The mass dissolved from each column into the dissolved-load reservoir this pass (non-negative, capped at
    /// the column's available mass).
    pub dissolved: Vec<Fixed>,
    /// The intact bedrock each column converts to mobile grains this pass (non-negative, capped at the column's
    /// available mass). It stays in the solid column; it is the transportable stock, not a reservoir move.
    pub grains_produced: Vec<Fixed>,
}

/// The THERMAL-CHEMICAL ALTERATION driver: it alters the bedrock in place and is the SOURCE the transport drivers
/// presuppose (they move grains that something must make). It has two limbs. The DISSOLUTION limb removes mass
/// into solution on a thermally-activated (Arrhenius) kinetic, the rate `coefficient * exp(-activation_temperature
/// / T)`, so a hotter surface dissolves faster; that mass moves from the solid column to the dissolved-load
/// reservoir. The FRACTURING limb produces mobile grains by thermal and frost cracking, the rate
/// `fracture_coefficient * diurnal_range`, so a larger day-night temperature swing makes more grains; that mass
/// stays in the solid column as the transportable stock.
///
/// The Arrhenius form is the general temperature dependence of a reaction rate, and the driver keys it on DATA:
/// the `dissolution_coefficient` folds the reserved pre-factor, the solvent's chemical aggressiveness, and the
/// lithology's solubility (surfaced separately in the driver row, armed as their product), and the
/// `activation_temperature` is the reserved activation energy over the floor gas constant. So a non-water solvent
/// is a data row with its own coefficient and activation temperature, not a rewrite. The honest limit the design
/// flags stands: the Arrhenius form assumes thermally-activated kinetics, so a solvent whose dissolution is not
/// thermally activated (a purely mechanical or a radiolytic process) needs a different kernel form (a floor
/// extension) rather than different data alone.
///
/// The reserved values, surfaced-with-basis and never fabricated: `dissolution_coefficient` (the combined
/// pre-exponential, whose factors are the cited Arrhenius pre-factor per lithology and solvent, the solvent
/// chemical aggressiveness, and the lithology solubility), `activation_temperature` (the cited activation energy
/// over the floor gas constant, in Kelvin), and `fracture_coefficient` (a per-material fatigue rate folded with
/// the solvent's freeze expansion, keyed on the diurnal range). All must be non-negative; a negative is refused
/// fail-loud ([`CalibrationError::BadValue`]), never a silent clamp. A zero coefficient is a valid inert opt-out
/// of that limb. A non-positive temperature is refused fail-loud (Kelvin is positive, and the Arrhenius exponent
/// divides by it).
///
/// Deterministic fixed-point arithmetic over the pinned integer-only `Fixed::exp` (Principle 3), a pure function
/// of the inputs and worker-invariant (Principle 10). The fields are per-column and equal length; a mismatch is
/// refused fail-loud.
#[allow(clippy::too_many_arguments)]
pub fn thermal_chemical_alter(
    column_mass: &[Fixed],
    temperature: &[Fixed],
    diurnal_range: &[Fixed],
    dissolution_coefficient: Fixed,
    activation_temperature: Fixed,
    fracture_coefficient: Fixed,
) -> Result<ThermalChemicalPass, CalibrationError> {
    for (id, v) in [
        (
            "surface.thermal_chemical_dissolution_coefficient",
            dissolution_coefficient,
        ),
        (
            "surface.thermal_chemical_activation_temperature",
            activation_temperature,
        ),
        (
            "surface.thermal_chemical_fracture_coefficient",
            fracture_coefficient,
        ),
    ] {
        if v < Fixed::ZERO {
            return Err(CalibrationError::BadValue {
                id: id.to_string(),
                detail: format!(
                    "the thermal-chemical rate constant must be non-negative; got {}",
                    v.to_f64_lossy()
                ),
            });
        }
    }
    let n = column_mass.len();
    if temperature.len() != n || diurnal_range.len() != n {
        return Err(CalibrationError::BadValue {
            id: "surface.thermal_chemical_grid".to_string(),
            detail: format!(
                "the column_mass ({}), temperature ({}), and diurnal_range ({}) fields must be equal length",
                n,
                temperature.len(),
                diurnal_range.len()
            ),
        });
    }

    let mut dissolved = vec![Fixed::ZERO; n];
    let mut grains_produced = vec![Fixed::ZERO; n];
    for i in 0..n {
        if temperature[i] <= Fixed::ZERO {
            return Err(CalibrationError::BadValue {
                id: "surface.thermal_chemical_temperature".to_string(),
                detail: format!(
                    "the surface temperature must be positive Kelvin; cell {} is {}",
                    i,
                    temperature[i].to_f64_lossy()
                ),
            });
        }
        let available = if column_mass[i] > Fixed::ZERO {
            column_mass[i]
        } else {
            Fixed::ZERO
        };
        // The Arrhenius dissolution rate: coefficient * exp(-activation_temperature / T). The exponent is a
        // dimensionless ratio; a large barrier (cold surface) drives it toward zero rate (exp saturates). The
        // rate is gated only by temperature, not by per-cell solvent PRESENCE: the `dissolution_coefficient` is
        // uniform over the pass (it folds the solvent's aggressiveness and the lithology's solubility), so a
        // world with a patchy solvent (a locally wet arid world) needs a per-cell solvent-availability forcing to
        // gate dissolution to where the solvent is present, the analogue of the fluid-shear flow routing, a
        // deferred extension. The division and the product are checked so a near-zero temperature or an extreme
        // coefficient fails loud rather than wrapping silently.
        let exponent = div_or_overflow(
            activation_temperature,
            temperature[i],
            "surface.thermal_chemical_arrhenius",
        )?;
        let rate = mul_or_overflow(
            dissolution_coefficient,
            (-exponent).exp(),
            "surface.thermal_chemical_dissolution_rate",
        )?;
        dissolved[i] = if rate < available { rate } else { available };
        // The remaining solid mass after dissolution is the stock the fracturing limb can convert to grains.
        let after_dissolution = available - dissolved[i];
        // Thermal and frost fracturing: linear in the temperature-cycling amplitude (`diurnal_range` on a
        // rotating world; any cyclic thermal forcing, tidal or orbital, on an alien), a larger swing cracks more
        // grains.
        let produced = if diurnal_range[i] > Fixed::ZERO {
            mul_or_overflow(
                fracture_coefficient,
                diurnal_range[i],
                "surface.thermal_chemical_fracture_rate",
            )?
        } else {
            Fixed::ZERO
        };
        grains_produced[i] = if produced < after_dissolution {
            produced
        } else {
            after_dissolution
        };
    }

    Ok(ThermalChemicalPass {
        dissolved,
        grains_produced,
    })
}

/// The result of one [`deposit`] pass: the mass settled into each column and the sediment flux leaving each cell
/// downstream. The kernel is the SINK half of the fluvial budget: `deposited` is the mass added to each column
/// (the snapshot-apply reconciliation applies it as a positive delta, raising the elevation ledger), and the pass
/// conserves the source it is GIVEN: the total deposited equals the total of the `entrained` source array (every
/// source unit settles somewhere). Whether that source array is the reconciled (honored) removal or a raw
/// pre-reconcile demand is the ARMING step's responsibility, not this kernel's: the cross-writer budget closes
/// only when the arming feeds the honored removal here (see the module note).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepositionPass {
    /// The mass settled into each column this pass (non-negative). Where the sediment flux exceeds the local
    /// transport capacity the excess drops out here; a drainage outlet settles its whole remaining load.
    pub deposited: Vec<Fixed>,
    /// The sediment flux leaving each cell for its receiver (non-negative). Zero at a drainage outlet (which
    /// deposits its whole load rather than exporting it off-grid, so the budget stays column-to-column).
    pub carried_out: Vec<Fixed>,
}

/// The DEPOSITION driver: the conservation SINK that closes the fluvial budget, the negative half of the
/// fluid-shear source. It walks the drainage network from upstream to downstream, and at each cell the sediment
/// flux available to carry (the flux arriving from upstream plus the cell's own entrained source) is limited by
/// the local transport `capacity`: the flux the capacity can hold continues downstream, and the excess SETTLES
/// into the column here (raising the elevation ledger). Where the fluid slows or the slope flattens the capacity
/// drops, so the load settles, exactly the depositional behaviour the budget needs. A drainage outlet settles its
/// whole remaining load (the river-mouth deposit), so no mass exports off-grid and the total deposited equals the
/// total of the `entrained` source it is given. This closes the budget column-to-column only when the caller
/// feeds the RECONCILED (honored) removal as the source: on a contested column the source drivers demand more
/// than the snapshot holds, [`crate::surface_transport::reconcile_column`] clamps the honored removal to the
/// snapshot, and the arming step must route only that honored mass here, or the ledger would gain net mass
/// (deposited from the raw demand exceeding the clamped removal). That composition contract is the arming step's,
/// not this kernel's.
///
/// The `capacity` is the exact-root transport capacity (formed as `transport_coefficient * sqrt(A) * S`, the same
/// GPU-canon exact-root shear proxy the fluid-shear entrainment reads, with a reserved transport coefficient), so
/// the caller supplies it in that form and the kernel is the pure settling operator over it. The grain-size
/// sorting the design names (coarse grains settling first, fines carried further, each grain class with its own
/// settling capacity in the exact-root settling form) is a refinement over this bulk-settling core, a per-class
/// capacity extension deferred, so the built pass settles the bulk excess and closes the budget without yet
/// fractionating the load by grain size.
///
/// Deterministic and worker-invariant (Principle 3, Principle 10): the walk is the same Kahn topological order as
/// the drainage accumulation (a cell settles only once all its upstream flux has arrived, so its deposit is
/// final), and the downstream fold is a commutative exact add, so `deposited` and `carried_out` are pure
/// functions of the inputs. The fields are per-cell and equal length; a mismatch, a negative capacity, or a
/// negative entrained source is refused fail-loud ([`CalibrationError::BadValue`]).
pub fn deposit(
    entrained: &[Fixed],
    receiver: &[usize],
    capacity: &[Fixed],
) -> Result<DepositionPass, CalibrationError> {
    let n = entrained.len();
    if receiver.len() != n || capacity.len() != n {
        return Err(CalibrationError::BadValue {
            id: "surface.deposition_grid".to_string(),
            detail: format!(
                "the entrained ({}), receiver ({}), and capacity ({}) fields must be equal length",
                n,
                receiver.len(),
                capacity.len()
            ),
        });
    }
    for i in 0..n {
        if entrained[i] < Fixed::ZERO {
            return Err(CalibrationError::BadValue {
                id: "surface.deposition_entrained".to_string(),
                detail: format!("the entrained source must be non-negative; cell {i} is negative"),
            });
        }
        if capacity[i] < Fixed::ZERO {
            return Err(CalibrationError::BadValue {
                id: "surface.deposition_capacity".to_string(),
                detail: format!(
                    "the transport capacity must be non-negative; cell {i} is negative"
                ),
            });
        }
        // Each receiver must be a valid cell index, so a malformed caller-supplied routing fails loud here rather
        // than panicking on an out-of-range index inside the walk.
        if receiver[i] >= n {
            return Err(CalibrationError::BadValue {
                id: "surface.deposition_receiver".to_string(),
                detail: format!(
                    "receiver[{i}] = {} is out of range for {n} cells",
                    receiver[i]
                ),
            });
        }
    }

    let mut indegree = vec![0usize; n];
    for (i, &r) in receiver.iter().enumerate() {
        if r != i {
            indegree[r] += 1;
        }
    }
    let mut incoming = vec![Fixed::ZERO; n];
    let mut deposited = vec![Fixed::ZERO; n];
    let mut carried_out = vec![Fixed::ZERO; n];
    let mut ready: Vec<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut head = 0;
    while head < ready.len() {
        let c = ready[head];
        head += 1;
        let available = incoming[c] + entrained[c];
        let carried = if available < capacity[c] {
            available
        } else {
            capacity[c]
        };
        deposited[c] = available - carried;
        let r = receiver[c];
        if r != c {
            carried_out[c] = carried;
            incoming[r] += carried;
            indegree[r] -= 1;
            if indegree[r] == 0 {
                ready.push(r);
            }
        } else {
            // A drainage outlet settles its whole remaining load (the river-mouth deposit), so nothing exports
            // off-grid and the column-to-column budget closes exactly.
            deposited[c] += carried;
            carried_out[c] = Fixed::ZERO;
        }
    }

    // Every cell must have been processed. A `priority_flood` receiver forest is acyclic (each cell drains to an
    // outlet in finitely many steps), so the Kahn walk reaches all `n`; if a caller passes a receiver with a
    // CYCLE, the cells in it never reach in-degree zero and would be skipped, and their entrained mass would be
    // SILENTLY LOST (breaking the total-deposited == total-entrained identity). Refuse that fail-loud rather than
    // return a quietly non-conserving result.
    if head != n {
        return Err(CalibrationError::BadValue {
            id: "surface.deposition_receiver_cycle".to_string(),
            detail: format!(
                "the receiver forest has a cycle: only {head} of {n} cells drain to an outlet, so mass would be silently lost"
            ),
        });
    }

    Ok(DepositionPass {
        deposited,
        carried_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(z: &[Fixed]) -> Fixed {
        z.iter().fold(Fixed::ZERO, |acc, &v| acc + v)
    }

    #[test]
    fn a_flat_terrain_is_unchanged() {
        // No slope, no flux: a flat field is its own diffusive steady state.
        let z = vec![Fixed::from_int(5); 9];
        let out =
            hillslope_diffuse(z.clone(), 3, 3, Fixed::from_ratio(1, 4), 50, 0).expect("valid");
        assert_eq!(out.state, z);
    }

    #[test]
    fn a_spike_diffuses_and_conserves_total_elevation_exactly() {
        // A central spike spreads to its neighbours; the total elevation is conserved bit-for-bit through the
        // flux-based step (the load-bearing conservation invariant).
        let mut z = vec![Fixed::ZERO; 9];
        z[4] = Fixed::from_int(9); // the centre of a 3x3 grid
        let opening = total(&z);
        let out = hillslope_diffuse(z, 3, 3, Fixed::from_ratio(1, 5), 100, 0).expect("valid");
        assert_eq!(
            total(&out.state),
            opening,
            "the diffusion conserves total elevation exactly"
        );
        // The peak fell and the neighbours rose: the spike diffused.
        assert!(out.state[4] < Fixed::from_int(9), "the peak eroded");
        assert!(out.state[1] > Fixed::ZERO, "a neighbour received sediment");
    }

    #[test]
    fn the_terrain_relaxes_toward_the_flat_mean_steady_state() {
        // With reflecting (no-flux) boundaries the diffusive steady state is the constant field at the mean
        // elevation; a generous solve relaxes toward it while conserving the total.
        let mut z = vec![Fixed::ZERO; 9];
        z[4] = Fixed::from_int(9);
        let opening = total(&z);
        let out = hillslope_diffuse(z, 3, 3, Fixed::from_ratio(1, 5), 1000, 1).expect("valid");
        assert_eq!(
            total(&out.state),
            opening,
            "conserved through the whole relaxation"
        );
        // The spread field is far flatter than the spike: the max-minus-min collapsed toward zero.
        let max = out
            .state
            .iter()
            .copied()
            .fold(Fixed::ZERO, |a, v| if v > a { v } else { a });
        let min = out
            .state
            .iter()
            .copied()
            .fold(Fixed::from_int(9), |a, v| if v < a { v } else { a });
        assert!(
            (max - min) < Fixed::from_int(2),
            "the terrain relaxed toward flat"
        );
    }

    #[test]
    fn an_out_of_range_diffusion_factor_is_refused_fail_loud() {
        let z = vec![Fixed::ZERO; 4];
        assert!(hillslope_diffuse(z.clone(), 2, 2, Fixed::ZERO, 10, 0).is_err());
        // Above 1/4 the explicit scheme is unstable: refused rather than run to a growing residual.
        assert!(hillslope_diffuse(z, 2, 2, Fixed::from_ratio(1, 3), 10, 0).is_err());
    }

    #[test]
    fn a_length_mismatch_is_refused_fail_loud() {
        let z = vec![Fixed::ZERO; 5];
        assert!(hillslope_diffuse(z, 3, 3, Fixed::from_ratio(1, 4), 10, 0).is_err());
    }

    #[test]
    fn the_diffusion_is_a_pure_function_of_its_inputs() {
        // Same inputs, bit-identical outcome (Principle 3, Principle 10).
        let mut z = vec![Fixed::from_int(1); 12];
        z[5] = Fixed::from_int(7);
        let a = hillslope_diffuse(z.clone(), 4, 3, Fixed::from_ratio(1, 5), 40, 2).expect("valid");
        let b = hillslope_diffuse(z, 4, 3, Fixed::from_ratio(1, 5), 40, 2).expect("valid");
        assert_eq!(a.state, b.state);
        assert_eq!(a.iterations, b.iterations);
    }

    // A west-to-east ramp: elevation equals the column x, so flow drains toward the low west edge and the
    // interior cells have a positive slope to their receivers.
    fn west_ramp(width: usize, height: usize) -> Vec<Fixed> {
        (0..width * height)
            .map(|i| Fixed::from_int((i % width) as i32))
            .collect()
    }

    fn outlet_load(pass: &FluidShearPass) -> Fixed {
        (0..pass.receiver.len())
            .filter(|&i| pass.receiver[i] == i)
            .map(|i| pass.carried_load[i])
            .fold(Fixed::ZERO, |acc, v| acc + v)
    }

    #[test]
    fn apply_exact_root_covers_the_gpu_canon_buildable_family() {
        // The identity, the exact square root, and the small integer powers, each exact.
        assert_eq!(
            apply_exact_root(Fixed::from_int(7), ExactRootExponent::LINEAR).unwrap(),
            Fixed::from_int(7)
        );
        assert_eq!(
            apply_exact_root(Fixed::from_int(9), ExactRootExponent::SQRT).unwrap(),
            Fixed::from_int(3),
            "sqrt(9) = 3 exactly"
        );
        assert_eq!(
            apply_exact_root(Fixed::from_int(5), ExactRootExponent::new(2, 1).unwrap()).unwrap(),
            Fixed::from_int(25),
            "5^2 = 25"
        );
        assert_eq!(
            apply_exact_root(Fixed::from_int(4), ExactRootExponent::new(3, 1).unwrap()).unwrap(),
            Fixed::from_int(64),
            "4^3 = 64"
        );
    }

    #[test]
    fn the_default_fluvial_exponents_are_the_reserved_sqrt_and_linear() {
        // The Mirror fluvial default m=1/2, n=1: the current hardcoded stream-power behaviour as data.
        assert_eq!(
            ExactRootExponent::SQRT,
            ExactRootExponent { num: 1, den: 2 }
        );
        assert_eq!(
            ExactRootExponent::LINEAR,
            ExactRootExponent { num: 1, den: 1 }
        );
    }

    #[test]
    fn an_unbuildable_exponent_is_refused_deferred_to_the_gpu_canon_primitive() {
        // A cube root (1/3) and an arbitrary fractional power are outside today's buildable family: refused
        // fail-loud as deferred to task #45, never silently approximated.
        assert!(
            apply_exact_root(Fixed::from_int(8), ExactRootExponent::new(1, 3).unwrap()).is_err()
        );
        assert!(
            apply_exact_root(Fixed::from_int(8), ExactRootExponent::new(2, 3).unwrap()).is_err()
        );
        assert!(
            apply_exact_root(Fixed::from_int(8), ExactRootExponent::new(5, 7).unwrap()).is_err()
        );
    }

    #[test]
    fn a_zero_denominator_and_a_negative_base_root_are_refused_fail_loud() {
        assert!(
            ExactRootExponent::new(1, 0).is_err(),
            "a zero denominator is not an exponent"
        );
        // A root of a negative base has no real value.
        assert!(apply_exact_root(Fixed::from_int(-4), ExactRootExponent::SQRT).is_err());
        // A negative base under the identity (den 1) is fine.
        assert_eq!(
            apply_exact_root(Fixed::from_int(-4), ExactRootExponent::LINEAR).unwrap(),
            Fixed::from_int(-4)
        );
    }

    #[test]
    fn an_overflowing_power_fails_loud() {
        // A large base cubed overflows Q32.32: refused rather than wrapping silently.
        assert!(apply_exact_root(
            Fixed::from_int(2_000_000),
            ExactRootExponent::new(3, 1).unwrap()
        )
        .is_err());
    }

    #[test]
    fn the_unit_stream_power_model_derives_the_mirror_fluvial_default() {
        // The Mirror hydraulic geometry (b = 1/2, c = 1) under the unit-stream-power model derives m = 1/2,
        // n = 1, the fluvial default: the exponents fall out of the physics, not a reserved datum.
        let (m, n) =
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (1, 2), (1, 1)).unwrap();
        assert_eq!(m, ExactRootExponent::SQRT);
        assert_eq!(n, ExactRootExponent::LINEAR);
    }

    #[test]
    fn the_total_stream_power_model_derives_linear_in_area() {
        // Total stream power derives m = c = 1, n = 1.
        let (m, n) =
            stream_power_exponents(IncisionProcessModel::TotalStreamPower, (1, 2), (1, 1)).unwrap();
        assert_eq!(m, ExactRootExponent::LINEAR);
        assert_eq!(n, ExactRootExponent::LINEAR);
    }

    #[test]
    fn the_shear_stress_model_derives_unbuildable_exponents_deferred_to_45() {
        // The shear-stress model with Manning resistance derives m = 3/10, n = 7/10 for the Mirror geometry:
        // correct as data, but outside the buildable exact-root family, so the kernel refuses them fail-loud
        // (deferred to the general fractional-power primitive).
        let (m, n) =
            stream_power_exponents(IncisionProcessModel::ShearStressManning, (1, 2), (1, 1))
                .unwrap();
        assert_eq!(m, ExactRootExponent { num: 3, den: 10 });
        assert_eq!(n, ExactRootExponent { num: 7, den: 10 });
        assert!(
            apply_exact_root(Fixed::from_int(4), m).is_err(),
            "the derived shear-stress exponent is not buildable yet"
        );
    }

    #[test]
    fn the_derived_unit_stream_power_exponents_reproduce_the_fluvial_kernel_byte_for_byte() {
        // The byte-neutral proof of the derivation: deriving the exponents from the unit-stream-power model and
        // feeding them to the kernel reproduces the fluvial default exactly, so wiring the derivation is
        // byte-neutral.
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let (m, n) =
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (1, 2), (1, 1)).unwrap();
        let derived = fluid_shear_with_exponents(&z, w, h, Fixed::from_int(1), Fixed::ZERO, m, n)
            .expect("valid");
        let default = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        assert_eq!(derived, default);
    }

    #[test]
    fn a_different_hydraulic_geometry_derives_a_different_area_exponent() {
        // The exponent tracks the physics: a steeper width-discharge scaling (b = 2/3) under unit stream power
        // derives m = 1/3, a smaller area exponent than the b = 1/2 case.
        let (m, _n) =
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (2, 3), (1, 1)).unwrap();
        assert_eq!(m, ExactRootExponent { num: 1, den: 3 });
    }

    #[test]
    fn an_unphysical_hydraulic_geometry_is_refused_fail_loud() {
        // b outside (0, 1) and a non-positive c are unphysical scalings, refused rather than deriving a nonsense
        // exponent.
        assert!(
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (1, 1), (1, 1)).is_err()
        );
        assert!(
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (3, 2), (1, 1)).is_err()
        );
        assert!(
            stream_power_exponents(IncisionProcessModel::UnitStreamPower, (1, 2), (0, 1)).is_err()
        );
    }

    #[test]
    fn the_shields_threshold_derives_from_the_excess_density_gravity_and_grain_size() {
        // tau_c = shields * (rho_s - rho_f) * g * d. With shields = 1/2, excess = 2, g = 10, d = 1:
        // 0.5 * 2 * 10 * 1 = 10, an exact check on the derivation.
        let tau = shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(3),
            Fixed::from_int(1),
            Fixed::from_int(10),
            Fixed::from_int(1),
        )
        .unwrap();
        assert_eq!(tau, Fixed::from_int(10));
    }

    #[test]
    fn a_denser_grain_a_stronger_gravity_or_a_larger_grain_raises_the_threshold_a_denser_fluid_lowers_it(
    ) {
        let base = shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(3),
            Fixed::from_int(1),
            Fixed::from_int(10),
            Fixed::from_int(1),
        )
        .unwrap();
        let denser_grain = shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(4),
            Fixed::from_int(1),
            Fixed::from_int(10),
            Fixed::from_int(1),
        )
        .unwrap();
        let denser_fluid = shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(3),
            Fixed::from_int(2),
            Fixed::from_int(10),
            Fixed::from_int(1),
        )
        .unwrap();
        let stronger_gravity = shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(3),
            Fixed::from_int(1),
            Fixed::from_int(20),
            Fixed::from_int(1),
        )
        .unwrap();
        assert!(
            denser_grain > base,
            "a denser grain entrains at a higher shear"
        );
        assert!(
            denser_fluid < base,
            "a denser fluid lowers the threshold (the liquid-versus-gas difference is data)"
        );
        assert!(
            stronger_gravity > base,
            "stronger gravity holds the grain down"
        );
    }

    #[test]
    fn a_buoyant_grain_is_refused_fail_loud() {
        // A grain no denser than its fluid is not bed-entrained by shear (it floats): refused rather than a
        // non-positive threshold.
        assert!(shields_critical_shear_stress(
            Fixed::from_ratio(1, 2),
            Fixed::from_int(1),
            Fixed::from_int(2),
            Fixed::from_int(10),
            Fixed::from_int(1)
        )
        .is_err());
    }

    #[test]
    fn a_non_positive_shields_input_is_refused_fail_loud() {
        let ok = (
            Fixed::from_ratio(1, 2),
            Fixed::from_int(3),
            Fixed::from_int(1),
        );
        assert!(
            shields_critical_shear_stress(Fixed::ZERO, ok.1, ok.2, Fixed::from_int(10), Fixed::ONE)
                .is_err(),
            "a non-positive Shields number is refused"
        );
        assert!(
            shields_critical_shear_stress(ok.0, ok.1, ok.2, Fixed::ZERO, Fixed::ONE).is_err(),
            "a non-positive gravity is refused"
        );
        assert!(
            shields_critical_shear_stress(ok.0, ok.1, ok.2, Fixed::from_int(10), Fixed::ZERO)
                .is_err(),
            "a non-positive grain size is refused"
        );
    }

    #[test]
    fn the_shields_derivation_is_a_pure_function() {
        let a = shields_critical_shear_stress(
            Fixed::from_ratio(45, 1000),
            Fixed::from_ratio(265, 100),
            Fixed::ONE,
            Fixed::from_ratio(98, 10),
            Fixed::from_ratio(1, 100),
        );
        let b = shields_critical_shear_stress(
            Fixed::from_ratio(45, 1000),
            Fixed::from_ratio(265, 100),
            Fixed::ONE,
            Fixed::from_ratio(98, 10),
            Fixed::from_ratio(1, 100),
        );
        assert_eq!(a, b);
        assert!(
            a.unwrap() > Fixed::ZERO,
            "an Earth-like grain has a positive threshold"
        );
    }

    #[test]
    fn a_flat_terrain_entrains_nothing() {
        // No slope, no shear: a flat field entrains and carries no mass.
        let z = vec![Fixed::from_int(7); 9];
        let pass = fluid_shear(&z, 3, 3, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        assert!(pass.entrained.iter().all(|&e| e == Fixed::ZERO));
        assert!(pass.carried_load.iter().all(|&c| c == Fixed::ZERO));
    }

    #[test]
    fn the_entrained_mass_all_reaches_the_outlets_the_conservation_identity() {
        // A ramp entrains on the interior, and every entrained unit is carried downstream to an outlet, so the
        // total entrained equals the total delivered at the drainage outlets (no mass created or lost in transit,
        // the load-bearing conservation identity of the source half).
        let (w, h) = (4, 4);
        let z = west_ramp(w, h);
        let pass = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        let total_entrained = total(&pass.entrained);
        assert_eq!(
            total_entrained,
            outlet_load(&pass),
            "all entrained mass reaches the outlets"
        );
        assert!(
            total_entrained > Fixed::ZERO,
            "the slope entrains some mass"
        );
        // A carried load is never below the cell's own entrainment (it is its own plus the upstream load).
        for i in 0..w * h {
            assert!(pass.carried_load[i] >= pass.entrained[i]);
            assert!(pass.entrained[i] >= Fixed::ZERO);
        }
    }

    #[test]
    fn every_cell_drains_to_exactly_one_outlet_so_the_areas_partition_the_grid() {
        // The drainage area of the outlets partitions the grid: every cell drains to exactly one boundary outlet,
        // so the outlet areas sum to the cell total (the check on the downstream accumulation).
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let pass = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        let outlet_area: i64 = (0..w * h)
            .filter(|&i| pass.receiver[i] == i)
            .map(|i| pass.drainage_area[i])
            .sum();
        assert_eq!(
            outlet_area as usize,
            w * h,
            "the outlet areas partition the grid"
        );
        // Every cell's area is at least one (itself) and at most the whole grid.
        assert!(pass
            .drainage_area
            .iter()
            .all(|&a| a >= 1 && a as usize <= w * h));
    }

    #[test]
    fn a_bowl_does_not_incise_a_filled_pit_has_no_slope_to_entrain() {
        // A rim at 5 around a central pit of 0: the pit fills and drains up over the rim, so its slope to the
        // receiver on the original terrain is negative (a lake does not incise), and the flat rim has no slope,
        // so nothing entrains.
        let z = vec![
            Fixed::from_int(5),
            Fixed::from_int(5),
            Fixed::from_int(5),
            Fixed::from_int(5),
            Fixed::ZERO,
            Fixed::from_int(5),
            Fixed::from_int(5),
            Fixed::from_int(5),
            Fixed::from_int(5),
        ];
        let pass = fluid_shear(&z, 3, 3, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        assert_eq!(
            pass.entrained[4],
            Fixed::ZERO,
            "the filled pit does not incise"
        );
        assert!(pass.entrained.iter().all(|&e| e == Fixed::ZERO));
    }

    #[test]
    fn a_high_threshold_entrains_nothing() {
        // Above the reserved entrainment threshold nothing moves: a threshold beyond any cell's shear proxy
        // leaves the field inert (the Shields/Bagnold critical-shear cutoff).
        let (w, h) = (4, 4);
        let z = west_ramp(w, h);
        let pass = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::from_int(1000)).expect("valid");
        assert!(pass.entrained.iter().all(|&e| e == Fixed::ZERO));
    }

    #[test]
    fn zero_erodibility_is_an_inert_opt_out() {
        // A zero erodibility entrains nothing: a valid inert opt-out, not a refused value.
        let (w, h) = (4, 4);
        let z = west_ramp(w, h);
        let pass = fluid_shear(&z, w, h, Fixed::ZERO, Fixed::ZERO).expect("valid");
        assert!(pass.entrained.iter().all(|&e| e == Fixed::ZERO));
        assert!(outlet_load(&pass) == Fixed::ZERO);
    }

    #[test]
    fn a_negative_reserved_value_is_refused_fail_loud() {
        let z = west_ramp(3, 3);
        assert!(
            fluid_shear(&z, 3, 3, Fixed::from_int(-1), Fixed::ZERO).is_err(),
            "a negative erodibility is refused"
        );
        assert!(
            fluid_shear(&z, 3, 3, Fixed::from_int(1), Fixed::from_int(-1)).is_err(),
            "a negative threshold is refused"
        );
    }

    #[test]
    fn a_length_mismatch_is_refused_fail_loud_for_fluid_shear() {
        let z = vec![Fixed::ZERO; 5];
        assert!(fluid_shear(&z, 3, 3, Fixed::from_int(1), Fixed::ZERO).is_err());
    }

    #[test]
    fn the_fluid_shear_pass_is_a_pure_function_of_its_inputs() {
        // Same inputs, bit-identical routing, area, entrainment, and carried load (Principle 3, Principle 10).
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let a = fluid_shear(&z, w, h, Fixed::from_int(2), Fixed::from_ratio(1, 2)).expect("valid");
        let b = fluid_shear(&z, w, h, Fixed::from_int(2), Fixed::from_ratio(1, 2)).expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn the_default_exponents_reproduce_the_fluvial_kernel_byte_for_byte() {
        // fluid_shear delegates to fluid_shear_with_exponents with the Mirror fluvial default (SQRT, LINEAR), so
        // the two are bit-identical: the exponent field is byte-neutral by construction.
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let default = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        let explicit = fluid_shear_with_exponents(
            &z,
            w,
            h,
            Fixed::from_int(1),
            Fixed::ZERO,
            ExactRootExponent::SQRT,
            ExactRootExponent::LINEAR,
        )
        .expect("valid");
        assert_eq!(default, explicit);
    }

    #[test]
    fn a_different_area_exponent_shapes_the_entrainment() {
        // Below the base-level cap (a small erodibility), the area exponent shapes the entrainment: a linear
        // exponent (m = 1) entrains differently from the sqrt default (m = 1/2), so the exponent is a live datum.
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let k = Fixed::from_ratio(1, 100);
        let sqrt_m = fluid_shear_with_exponents(
            &z,
            w,
            h,
            k,
            Fixed::ZERO,
            ExactRootExponent::SQRT,
            ExactRootExponent::LINEAR,
        )
        .expect("valid");
        let linear_m = fluid_shear_with_exponents(
            &z,
            w,
            h,
            k,
            Fixed::ZERO,
            ExactRootExponent::LINEAR,
            ExactRootExponent::LINEAR,
        )
        .expect("valid");
        assert_ne!(
            sqrt_m.entrained, linear_m.entrained,
            "the area exponent shapes the entrainment where the cap does not bind"
        );
    }

    #[test]
    fn an_unbuildable_fluid_shear_exponent_fails_loud() {
        // A cube-root area exponent is outside the buildable family, so the kernel refuses it fail-loud (deferred
        // to the general fractional-power primitive) rather than silently approximating.
        let (w, h) = (3, 3);
        let z = west_ramp(w, h);
        let cube_root = ExactRootExponent::new(1, 3).unwrap();
        assert!(fluid_shear_with_exponents(
            &z,
            w,
            h,
            Fixed::from_int(1),
            Fixed::ZERO,
            cube_root,
            ExactRootExponent::LINEAR
        )
        .is_err());
    }

    #[test]
    fn a_hotter_surface_dissolves_faster_the_arrhenius_kinetic() {
        // The Arrhenius rate rises with temperature: two cells with the same abundant column and no diurnal
        // range, the hotter one dissolves more (exp(-activation_temperature / T) is larger for a larger T).
        let column = vec![Fixed::from_int(100), Fixed::from_int(100)];
        let temperature = vec![Fixed::from_int(300), Fixed::from_int(600)];
        let no_range = vec![Fixed::ZERO, Fixed::ZERO];
        let pass = thermal_chemical_alter(
            &column,
            &temperature,
            &no_range,
            Fixed::from_int(10),  // coefficient
            Fixed::from_int(600), // activation temperature
            Fixed::ZERO,          // no fracturing this test
        )
        .expect("valid");
        assert!(
            pass.dissolved[1] > pass.dissolved[0],
            "the hotter cell dissolves faster"
        );
        assert!(pass.dissolved[0] > Fixed::ZERO, "some dissolution occurs");
        assert!(pass.grains_produced.iter().all(|&g| g == Fixed::ZERO));
    }

    #[test]
    fn a_larger_diurnal_range_produces_more_grains() {
        // Thermal and frost fracturing is linear in the diurnal range: the cell with the wider day-night swing
        // makes more mobile grains.
        let column = vec![Fixed::from_int(100), Fixed::from_int(100)];
        let temperature = vec![Fixed::from_int(288), Fixed::from_int(288)];
        let range = vec![Fixed::from_int(5), Fixed::from_int(40)];
        let pass = thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::ZERO,              // no dissolution this test
            Fixed::from_int(100),     // activation temperature (unused with zero coefficient)
            Fixed::from_ratio(1, 10), // fracture coefficient
        )
        .expect("valid");
        assert!(
            pass.grains_produced[1] > pass.grains_produced[0],
            "the wider swing makes more grains"
        );
        assert!(pass.dissolved.iter().all(|&d| d == Fixed::ZERO));
    }

    #[test]
    fn alteration_never_exceeds_the_available_column_mass() {
        // A thin column with aggressive dissolution and fracturing: the dissolved plus the grains produced never
        // exceeds the column, and neither limb alone over-draws it (the fail-loud cap).
        let column = vec![Fixed::from_int(1)];
        let temperature = vec![Fixed::from_int(1000)];
        let range = vec![Fixed::from_int(50)];
        let pass = thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(1000),
            Fixed::from_ratio(1, 1),
            Fixed::from_int(1000),
        )
        .expect("valid");
        assert!(
            pass.dissolved[0] <= Fixed::from_int(1),
            "dissolution capped"
        );
        assert!(
            pass.dissolved[0] + pass.grains_produced[0] <= Fixed::from_int(1),
            "the two limbs together never exceed the column"
        );
    }

    #[test]
    fn zero_coefficients_are_inert_opt_outs_for_thermal_chemical() {
        let column = vec![Fixed::from_int(100); 4];
        let temperature = vec![Fixed::from_int(500); 4];
        let range = vec![Fixed::from_int(30); 4];
        let pass = thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::ZERO,
            Fixed::ZERO,
            Fixed::ZERO,
        )
        .expect("valid");
        assert!(pass.dissolved.iter().all(|&d| d == Fixed::ZERO));
        assert!(pass.grains_produced.iter().all(|&g| g == Fixed::ZERO));
    }

    #[test]
    fn a_negative_thermal_chemical_constant_is_refused_fail_loud() {
        let column = vec![Fixed::from_int(10)];
        let temperature = vec![Fixed::from_int(300)];
        let range = vec![Fixed::from_int(10)];
        assert!(thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(-1),
            Fixed::from_int(100),
            Fixed::from_int(1)
        )
        .is_err());
        assert!(thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(1),
            Fixed::from_int(100),
            Fixed::from_int(-1)
        )
        .is_err());
    }

    #[test]
    fn a_non_positive_temperature_is_refused_fail_loud() {
        // Kelvin is positive and the Arrhenius exponent divides by it: a zero or negative temperature is a bad
        // input, refused rather than run to a divide-by-zero.
        let column = vec![Fixed::from_int(10)];
        let range = vec![Fixed::from_int(10)];
        assert!(thermal_chemical_alter(
            &column,
            &[Fixed::ZERO],
            &range,
            Fixed::from_int(1),
            Fixed::from_int(100),
            Fixed::from_int(1)
        )
        .is_err());
    }

    #[test]
    fn a_length_mismatch_is_refused_fail_loud_for_thermal_chemical() {
        let column = vec![Fixed::from_int(10); 3];
        let temperature = vec![Fixed::from_int(300); 2];
        let range = vec![Fixed::from_int(10); 3];
        assert!(thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(1),
            Fixed::from_int(100),
            Fixed::from_int(1)
        )
        .is_err());
    }

    #[test]
    fn the_thermal_chemical_pass_is_a_pure_function_of_its_inputs() {
        let column = vec![
            Fixed::from_int(50),
            Fixed::from_int(80),
            Fixed::from_int(20),
        ];
        let temperature = vec![
            Fixed::from_int(280),
            Fixed::from_int(320),
            Fixed::from_int(400),
        ];
        let range = vec![Fixed::from_int(12), Fixed::from_int(8), Fixed::from_int(25)];
        let a = thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(5),
            Fixed::from_int(500),
            Fixed::from_ratio(1, 4),
        )
        .expect("valid");
        let b = thermal_chemical_alter(
            &column,
            &temperature,
            &range,
            Fixed::from_int(5),
            Fixed::from_int(500),
            Fixed::from_ratio(1, 4),
        )
        .expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn all_entrained_mass_settles_the_conservation_identity() {
        // A chain 0->1->2->3 (3 the outlet), a source at the head, an abundant capacity: the whole load carries
        // to the mouth and settles there, so the total deposited equals the total entrained (the load-bearing
        // conservation identity of the sink half).
        let entrained = vec![Fixed::from_int(10), Fixed::ZERO, Fixed::ZERO, Fixed::ZERO];
        let receiver = vec![1usize, 2, 3, 3];
        let capacity = vec![Fixed::from_int(100); 4];
        let pass = deposit(&entrained, &receiver, &capacity).expect("valid");
        assert_eq!(
            total(&pass.deposited),
            total(&entrained),
            "all mass settles"
        );
        assert_eq!(
            pass.deposited[3],
            Fixed::from_int(10),
            "an abundant capacity carries the whole load to the mouth"
        );
        assert!(
            pass.carried_out[3] == Fixed::ZERO,
            "an outlet exports nothing"
        );
    }

    #[test]
    fn a_capacity_drop_settles_the_excess_upstream() {
        // The same chain but the head cell's capacity is small: the excess above capacity settles at the head,
        // and the rest carries to the mouth. The total still equals the entrained (nothing created or lost).
        let entrained = vec![Fixed::from_int(10), Fixed::ZERO, Fixed::ZERO, Fixed::ZERO];
        let receiver = vec![1usize, 2, 3, 3];
        let capacity = vec![
            Fixed::from_int(2),
            Fixed::from_int(100),
            Fixed::from_int(100),
            Fixed::from_int(100),
        ];
        let pass = deposit(&entrained, &receiver, &capacity).expect("valid");
        assert_eq!(
            pass.deposited[0],
            Fixed::from_int(8),
            "the excess above the capacity settles at the head"
        );
        assert_eq!(
            pass.deposited[3],
            Fixed::from_int(2),
            "the carried remainder settles at the mouth"
        );
        assert_eq!(total(&pass.deposited), total(&entrained), "still conserved");
    }

    #[test]
    fn zero_entrained_deposits_nothing() {
        let entrained = vec![Fixed::ZERO; 4];
        let receiver = vec![1usize, 2, 3, 3];
        let capacity = vec![Fixed::from_int(100); 4];
        let pass = deposit(&entrained, &receiver, &capacity).expect("valid");
        assert!(pass.deposited.iter().all(|&d| d == Fixed::ZERO));
    }

    #[test]
    fn a_negative_deposition_input_is_refused_fail_loud() {
        let receiver = vec![1usize, 1];
        assert!(
            deposit(
                &[Fixed::from_int(-1), Fixed::ZERO],
                &receiver,
                &[Fixed::ZERO; 2]
            )
            .is_err(),
            "a negative entrained source is refused"
        );
        assert!(
            deposit(
                &[Fixed::ZERO; 2],
                &receiver,
                &[Fixed::from_int(-1), Fixed::ZERO]
            )
            .is_err(),
            "a negative capacity is refused"
        );
    }

    #[test]
    fn a_length_mismatch_is_refused_fail_loud_for_deposition() {
        assert!(deposit(&[Fixed::ZERO; 3], &[0usize, 1], &[Fixed::ZERO; 3]).is_err());
    }

    #[test]
    fn a_cyclic_receiver_is_refused_fail_loud_so_mass_is_never_silently_lost() {
        // A 2-cycle (cell 0 -> 1 -> 0, neither an outlet): the Kahn walk can process nothing, so the entrained
        // mass would be silently dropped and the conservation identity broken. The kernel refuses it fail-loud
        // rather than return a quietly non-conserving Ok (the hardened acyclicity precondition).
        let entrained = vec![Fixed::from_int(5), Fixed::ZERO];
        let cyclic = vec![1usize, 0];
        let capacity = vec![Fixed::ZERO; 2];
        assert!(deposit(&entrained, &cyclic, &capacity).is_err());
    }

    #[test]
    fn an_out_of_range_receiver_is_refused_fail_loud_not_a_panic() {
        // A receiver index beyond the grid is a malformed routing: refused with a clean error rather than a
        // panic on the out-of-range index inside the walk.
        let entrained = vec![Fixed::ZERO; 2];
        let bad = vec![9usize, 1];
        let capacity = vec![Fixed::ZERO; 2];
        assert!(deposit(&entrained, &bad, &capacity).is_err());
    }

    #[test]
    fn an_overflowing_transport_product_fails_loud_rather_than_wrapping_silently() {
        // The shear proxy sqrt(A) * S and the capacity K * excess route through the checked multiply, so an
        // extreme reserved coefficient surfaces as a fail-loud error rather than a silent Q32.32 wrap. A huge
        // erodibility on a steep, high-drainage ramp overflows the capacity product.
        let (w, h) = (4, 4);
        let z = west_ramp(w, h);
        let huge = Fixed::from_int(i32::MAX);
        assert!(
            fluid_shear(&z, w, h, huge, Fixed::ZERO).is_err(),
            "an overflowing capacity product is refused, not wrapped"
        );
    }

    #[test]
    fn the_deposition_pass_is_a_pure_function_of_its_inputs() {
        let entrained = vec![
            Fixed::from_int(4),
            Fixed::from_int(1),
            Fixed::from_int(6),
            Fixed::ZERO,
        ];
        let receiver = vec![2usize, 2, 3, 3];
        let capacity = vec![
            Fixed::from_int(3),
            Fixed::from_int(3),
            Fixed::from_int(5),
            Fixed::from_int(100),
        ];
        let a = deposit(&entrained, &receiver, &capacity).expect("valid");
        let b = deposit(&entrained, &receiver, &capacity).expect("valid");
        assert_eq!(a, b);
    }

    #[test]
    fn the_fluid_shear_source_and_deposition_sink_conserve_together() {
        // The full erosion-transport-deposition cycle over the two drivers: fluid-shear entrains from a ramp, and
        // deposition settles that source, so the total deposited equals the total entrained (the drivers compose
        // into a mass-conserving pass, the whole point of the source-and-sink split).
        let (w, h) = (5, 4);
        let z = west_ramp(w, h);
        let shear = fluid_shear(&z, w, h, Fixed::from_int(1), Fixed::ZERO).expect("valid");
        let capacity = vec![Fixed::from_int(1000); w * h]; // abundant, so the load routes to the outlets
        let dep = deposit(&shear.entrained, &shear.receiver, &capacity).expect("valid");
        assert_eq!(
            total(&dep.deposited),
            total(&shear.entrained),
            "the source and the sink conserve together"
        );
        assert!(
            total(&shear.entrained) > Fixed::ZERO,
            "the ramp entrained some mass to deposit"
        );
    }
}
