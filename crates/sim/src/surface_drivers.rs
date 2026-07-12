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
//! This slice is the first build-now kernel, GRAVITY-DOWNSLOPE ([`hillslope_diffuse`]), which sets the local
//! slope the fluid-shear and solid-solvent drivers read.

use civsim_core::Fixed;
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
}
