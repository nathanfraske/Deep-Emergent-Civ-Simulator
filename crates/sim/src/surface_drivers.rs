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
//! The build-now kernels land one at a time. GRAVITY-DOWNSLOPE ([`hillslope_diffuse`]) is the first, and it
//! sets the local slope the fluid-shear and solid-solvent drivers read. FLUID-SHEAR ([`fluid_shear`]) is the
//! second, the moving-fluid entrainment and transport driver, the SOURCE half of the fluvial budget that hands a
//! conserved carried-load field to deposition (the sink, a later slice).

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
/// must lie in `(0, 1/4]` for the explicit scheme to relax rather than grow; a factor outside that range is
/// refused fail-loud (a [`CalibrationError::BadValue`]), never a silent clamp. The nonlinear threshold-failure
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
/// correctness guard, never a live limit.
const MAX_FLUID_SHEAR_CELLS: usize = 1usize << 31;

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
pub fn fluid_shear(
    elevation: &[Fixed],
    width: usize,
    height: usize,
    erodibility: Fixed,
    theta: Fixed,
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

    // The entrainment per cell: erodibility * (sqrt(A) * S - theta), above the threshold, capped at the drop to
    // the receiver so erosion never inverts the slope.
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
        let shear = area.sqrt() * drop; // sqrt(A) * S, both exact-root exponents
        if shear <= theta {
            continue;
        }
        let capacity = erodibility * (shear - theta);
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
        // dimensionless ratio; a large barrier (cold surface) drives it toward zero rate (exp saturates).
        let exponent = -(activation_temperature / temperature[i]);
        let rate = dissolution_coefficient * exponent.exp();
        dissolved[i] = if rate < available { rate } else { available };
        // The remaining solid mass after dissolution is the stock the fracturing limb can convert to grains.
        let after_dissolution = available - dissolved[i];
        // Thermal and frost fracturing: linear in the diurnal range (a larger swing cracks more grains).
        let produced = if diurnal_range[i] > Fixed::ZERO {
            fracture_coefficient * diurnal_range[i]
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
}
