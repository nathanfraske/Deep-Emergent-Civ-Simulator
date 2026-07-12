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

//! The interior convection-evolution subsystem (genesis-forward geology arc): it composes the merged mantle
//! floor law-forms (`crates/physics/src/laws.rs`) into one per-step column update and drives it to a bounded
//! steady state with C's fixed-cap iterative solver (`civsim_world::solve::fixed_cap_solve`). The floor stays
//! a set of single physical relations; the composition into a per-step evolution lives here, the same split
//! the productivity and matter-cycle subsystems use.
//!
//! One step reads a column's temperature contrast with its cold reference, forms the buoyancy source
//! ([`laws::thermal_density_anomaly`]) and the Rayleigh number ([`laws::rayleigh_number`]), latches the
//! convection onset ([`laws::threshold_latch`], so convection fires once the Rayleigh number crosses the
//! derived critical value and stays on), and evolves the column temperature ([`laws::internal_heat_evolution`])
//! under the radiogenic heat production minus the conductive surface loss (the Fourier flux
//! [`laws::conduction`] over the column mass, so the loss grows with the contrast and gives the restoring
//! force the steady state relaxes onto) plus, once convecting, the convective heat the buoyant flow carries
//! out ([`laws::stokes_velocity`] feeding [`laws::heat_advection`]). No authored convection knob: the onset
//! is the derived critical Rayleigh number, the flow the derived Stokes 2/9, the buoyancy the real material
//! thermal expansion. Determinism holds by construction: fixed-point kernels, a monotone latch, and C's
//! bounded integer-residual solve (never an unbounded until-converged spin), so the solve tolerance and cap
//! are a determinism bound, not a physical knob.
//!
//! Byte-neutral: this subsystem is defined and unit-tested against a SYNTHETIC column state but armed by no
//! scenario, so the canonical pins hold. The resident-field wiring (reading and writing A's `GeodynamicColumn`)
//! and the plate-domain identity (C's `civsim_world::label` connected-components) are the follow-on slices,
//! sequenced behind A's contract reaching main.

use civsim_core::Fixed;
use civsim_physics::laws;
use civsim_world::solve::{fixed_cap_solve, SolveOutcome};

/// The resident state of one interior column the convection solve evolves: its temperature and whether
/// convection has begun. The convection flag is the one-way Rayleigh-onset latch (once the Rayleigh number
/// has crossed the critical value it stays set), so the state records that the column has entered the
/// convecting regime, a relic the memoryless present-to-present kernels could not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnState {
    /// The column temperature (K).
    pub temperature: Fixed,
    /// Whether convection has begun (the one-way Rayleigh-onset latch).
    pub convecting: bool,
}

/// One interior column's fixed physical parameters for a convection step. Every field is a floor value or a
/// per-world datum the runner would read from the resident geodynamic state; here they are supplied by the
/// caller (the synthetic column under test, or a future `GeodynamicColumn` read). The engine bounds
/// (`ra_max`, `v_max`, `flux_max`) are representable caps and the solve tolerance is a determinism bound,
/// not physical knobs.
#[derive(Clone, Copy, Debug)]
pub struct ColumnParams {
    /// The cold reference temperature the column loses heat toward and its buoyancy contrast is taken against.
    pub reference_temperature: Fixed,
    /// Bulk density (kg/m^3).
    pub density: Fixed,
    /// Thermal conductivity (W/(m*K)).
    pub thermal_conductivity: Fixed,
    /// Volumetric thermal expansion (ppm/K), the real material value.
    pub thermal_expansion_ppm: Fixed,
    /// Gravity (m/s^2).
    pub gravity: Fixed,
    /// The convecting-layer depth (representable-scaled length; raw SI mantle depth overflows Q32.32).
    pub depth: Fixed,
    /// The buoyant parcel radius.
    pub radius: Fixed,
    /// Dynamic viscosity (representable-scaled Pa*s).
    pub viscosity: Fixed,
    /// Thermal diffusivity (m^2/s), k/(rho*c).
    pub thermal_diffusivity: Fixed,
    /// Specific heat capacity (J/(kg*K)).
    pub specific_heat: Fixed,
    /// Radiogenic heat production (W/kg), the source term.
    pub heat_production: Fixed,
    /// The derived critical Rayleigh number (marginal-stability eigenvalue), the onset threshold.
    pub ra_crit: Fixed,
    /// The representable Rayleigh cap (an engine bound).
    pub ra_max: Fixed,
    /// The representable velocity cap (an engine bound).
    pub v_max: Fixed,
    /// The representable conductive-flux cap (an engine bound).
    pub flux_max: Fixed,
    /// The tick duration.
    pub dt: Fixed,
}

/// One convection-evolution step: compose the merged floor law-forms into the next column state.
// @derives: the interior column temperature and convection-onset state <- the merged floor law-forms (thermal_density_anomaly, rayleigh_number, threshold_latch, stokes_velocity, heat_advection, internal_heat_evolution, conduction) over the column's own physical parameters; no authored convection knob (Ra_crit is the derived marginal-stability eigenvalue, the Stokes coefficient the derived 2/9, the buoyancy the real material thermal expansion). A bare @derives marker (no [id] token): this is a NEW derivation, not a retired-floor replacement, so it lands on the deriving-substrate billboard but stays out of the retired-floor-derivation registry (derive_gate.rs) and its [id] cross-check; broadening the liveness gate to any derived output is C's #43 slices 3+4, not this lane.
pub fn convection_step(state: &ColumnState, p: &ColumnParams) -> ColumnState {
    // The column's temperature contrast with its cold reference drives buoyancy, conduction, and advection.
    let delta_t = state.temperature - p.reference_temperature;

    // Buoyancy source: the thermal density excess (negative, and rising, when the column is hotter).
    let delta_rho = laws::thermal_density_anomaly(p.density, p.thermal_expansion_ppm, delta_t);

    // The Rayleigh number and the one-way convection-onset latch (fires once Ra crosses the derived Ra_crit).
    let rayleigh = laws::rayleigh_number(
        delta_rho,
        p.gravity,
        p.depth,
        p.viscosity,
        p.thermal_diffusivity,
        p.ra_max,
    );
    let convecting = laws::threshold_latch(rayleigh, p.ra_crit, state.convecting);

    // Conductive surface loss as specific power: the Fourier flux over the column mass per area, so the loss
    // grows with the contrast, the restoring force the steady state relaxes onto.
    let flux = laws::conduction(
        p.thermal_conductivity,
        Fixed::ONE,
        state.temperature,
        p.reference_temperature,
        p.depth,
        p.flux_max,
    );
    let mass_per_area = p.density.checked_mul(p.depth).unwrap_or(Fixed::MAX);
    let conductive_loss = if mass_per_area > Fixed::ZERO {
        flux.checked_div(mass_per_area).unwrap_or(Fixed::ZERO)
    } else {
        Fixed::ZERO
    };

    // Convective loss: once convecting, the buoyant flow carries heat out, augmenting conduction.
    let convective_loss = if convecting {
        let velocity = laws::stokes_velocity(delta_rho, p.gravity, p.radius, p.viscosity, p.v_max);
        laws::heat_advection(velocity, p.specific_heat, delta_t, p.depth)
    } else {
        Fixed::ZERO
    };

    let total_loss = conductive_loss.saturating_add(convective_loss);
    let temperature = laws::internal_heat_evolution(
        state.temperature,
        p.heat_production,
        total_loss,
        p.specific_heat,
        p.dt,
    );
    ColumnState {
        temperature,
        convecting,
    }
}

/// Drive the convection step to a bounded steady state with C's fixed-cap iterative solve: at most `cap`
/// steps, stopping the moment the integer temperature-change residual falls to or below `threshold`. Both
/// `cap` and `threshold` are determinism bounds (the solver terminates by construction, never an unbounded
/// until-converged spin), not physical knobs. Returns the solve outcome (the final state, the iteration
/// count, and whether the residual crossed the threshold within the cap).
pub fn convection_solve(
    initial: ColumnState,
    p: &ColumnParams,
    cap: u32,
    threshold: u64,
) -> SolveOutcome<ColumnState> {
    fixed_cap_solve(
        initial,
        cap,
        threshold,
        |s| convection_step(s, p),
        |a, b| a.temperature.to_bits().abs_diff(b.temperature.to_bits()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // A synthetic column: hot relative to a cold reference, with representable-scaled parameters (so the
    // Rayleigh intermediates fit Q32.32). The Rayleigh onset is switched by ra_crit in each test.
    fn column(ra_crit: Fixed) -> (ColumnState, ColumnParams) {
        let state = ColumnState {
            temperature: Fixed::from_int(400),
            convecting: false,
        };
        let params = ColumnParams {
            reference_temperature: Fixed::from_int(300),
            density: Fixed::ONE,
            thermal_conductivity: Fixed::from_int(2),
            thermal_expansion_ppm: Fixed::from_int(30),
            gravity: Fixed::from_int(10),
            depth: Fixed::ONE,
            radius: Fixed::ONE,
            viscosity: Fixed::ONE,
            thermal_diffusivity: Fixed::from_ratio(1, 100),
            specific_heat: Fixed::from_int(10),
            heat_production: Fixed::from_int(100),
            ra_crit,
            ra_max: Fixed::from_int(1_000_000),
            v_max: Fixed::from_int(1_000_000),
            flux_max: Fixed::from_int(1_000_000),
            dt: Fixed::ONE,
        };
        (state, params)
    }

    #[test]
    fn a_subcritical_column_stays_conductive_and_relaxes() {
        // Ra_crit above any Rayleigh number the column reaches: it never convects and relaxes to the
        // conductive steady state (where radiogenic production balances the Fourier loss).
        let (state, params) = column(Fixed::from_int(1_000_000_000));
        let outcome = convection_solve(state, &params, 10_000, 1);
        assert!(
            outcome.converged,
            "the conductive column relaxes to a steady state"
        );
        assert!(
            !outcome.state.convecting,
            "a subcritical column never enters the convecting regime"
        );
        // It cooled from 400 toward the reference (production below the loss at 400).
        assert!(outcome.state.temperature < Fixed::from_int(400));
        assert!(outcome.state.temperature > Fixed::from_int(300));
    }

    #[test]
    fn a_supercritical_column_convects_and_relaxes_cooler() {
        // Ra_crit at zero: the column convects immediately, so the buoyant flow adds a convective loss and
        // the steady state is cooler than pure conduction.
        let (state, conv_params) = column(Fixed::ZERO);
        let convecting = convection_solve(state, &conv_params, 10_000, 1);
        assert!(
            convecting.state.convecting,
            "a supercritical column convects"
        );
        assert!(convecting.converged);

        let (state, cond_params) = column(Fixed::from_int(1_000_000_000));
        let conductive = convection_solve(state, &cond_params, 10_000, 1);

        assert!(
            convecting.state.temperature < conductive.state.temperature,
            "convection carries extra heat out, so the convecting steady state is cooler"
        );
    }

    #[test]
    fn the_convection_solve_is_deterministic() {
        let (state, params) = column(Fixed::ZERO);
        let a = convection_solve(state, &params, 5_000, 1);
        let b = convection_solve(state, &params, 5_000, 1);
        assert_eq!(
            a.state, b.state,
            "the same synthetic column reproduces the same outcome"
        );
        assert_eq!(a.iterations, b.iterations);
    }

    #[test]
    fn the_convection_onset_latch_is_one_way() {
        // A column already convecting stays convecting even after it has cooled below the onset contrast:
        // the latch never un-fires, so the recorded convecting regime is stable.
        let (_, params) = column(Fixed::from_int(1_000_000_000)); // a high Ra_crit it will not re-cross
        let already = ColumnState {
            temperature: Fixed::from_int(301), // barely above the reference, a tiny Rayleigh number
            convecting: true,
        };
        let next = convection_step(&already, &params);
        assert!(
            next.convecting,
            "the convection latch holds once set, even below the onset threshold"
        );
    }
}
