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

use crate::material::{GeodynamicColumn, GeodynamicField};

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
    /// The representable convective-stress cap (an engine bound), the ceiling on the driving stress the
    /// lid-mobilization read compares to `mat.yield_strength`.
    pub stress_max: Fixed,
    /// The tick duration.
    pub dt: Fixed,
}

/// One convection-evolution step: compose the merged floor law-forms into the next column state.
// @derives[column_convection]: the interior column temperature and convection-onset state <- the merged floor law-forms (thermal_density_anomaly, rayleigh_number, threshold_latch, stokes_velocity, heat_advection, internal_heat_evolution, conduction) over the column's own physical parameters; no authored convection knob (Ra_crit is the derived marginal-stability eigenvalue, the Stokes coefficient the derived 2/9, the buoyancy the real material thermal expansion). A NEW derivation (not a retired-floor replacement), now covered by the liveness gate broadened to any derived output and any input source (task #46): the derive_gate registry carries a column_convection row (category new-derivation) whose probe perturbs the ColumnParams heat_production (a resident-field input) and asserts the stepped temperature responds.
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

/// The continuous interior read-outs of one column, the CONTINUOUS state the resident contract stores (gate
/// ruling, #176): the stepped interior temperature, the Rayleigh number (the convective vigor a consumer
/// reads to derive whether the column convects), and the convective driving stress (the lid-mobilization
/// quantity). No discrete "convecting" flag is carried: the discrete condition is derived from the Rayleigh
/// number against the critical value at each consumer site, so convection can begin and, on a cooling world,
/// cease.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnReadout {
    /// The stepped interior temperature (K).
    pub temperature: Fixed,
    /// The Rayleigh number (dimensionless), the continuous convective vigor.
    pub rayleigh: Fixed,
    /// The convective driving stress (Pa) the interior flow exerts on the base of the lithosphere.
    pub convective_stress: Fixed,
}

/// Read the continuous interior quantities of a column and its stepped temperature, composing the same floor
/// law-forms [`convection_step`] threads, plus [`civsim_physics::laws::convective_stress`] for the
/// lid-driving stress. The Rayleigh number and the stress are evaluated at the input state (the buoyancy the
/// step responds to), and the temperature is the stepped result, so the three form the column's continuous
/// state going out of the tick. Deterministic fixed-point.
pub fn column_readout(state: &ColumnState, p: &ColumnParams) -> ColumnReadout {
    let delta_t = state.temperature - p.reference_temperature;
    let delta_rho = laws::thermal_density_anomaly(p.density, p.thermal_expansion_ppm, delta_t);
    let rayleigh = laws::rayleigh_number(
        delta_rho,
        p.gravity,
        p.depth,
        p.viscosity,
        p.thermal_diffusivity,
        p.ra_max,
    );
    let velocity = laws::stokes_velocity(delta_rho, p.gravity, p.radius, p.viscosity, p.v_max);
    // The shear length is the thermal BOUNDARY LAYER thickness, DERIVED (gate ruling, #176): the boundary
    // layer thins with convective vigor as `depth * Ra^(-1/3)`, so a vigorous mantle (Ra of order 1e6) shears
    // over a layer about a hundredth of its depth, concentrating the driving stress. The cube root is the
    // deterministic fixed-point `Fixed::powf(1/3)` the merged Sherwood and surface-tension laws already use
    // (task #45 is a later GPU-shader-parity refinement of `powf`, not a blocker for this CPU derivation, and
    // the interior is a deep-time cold path). Written as `depth / Ra^(1/3)` (`powf` takes a positive
    // exponent), clamped to at most the layer depth (a boundary layer cannot exceed its own layer, a geometric
    // bound) and falling back to the depth when Ra is non-positive (no convection, where the stress is zero
    // regardless).
    let ra_cube_root = rayleigh.powf(Fixed::from_ratio(1, 3));
    let length_scale = if ra_cube_root > Fixed::ZERO {
        p.depth
            .checked_div(ra_cube_root)
            .unwrap_or(p.depth)
            .min(p.depth)
    } else {
        p.depth
    };
    let convective_stress =
        laws::convective_stress(p.viscosity, velocity, length_scale, p.stress_max);
    let next = convection_step(state, p);
    ColumnReadout {
        temperature: next.temperature,
        rayleigh,
        convective_stress,
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

/// A column's decaying radiogenic heat source paired with its thermal state, for the secular thermal
/// history: over geological time the heat-producing isotope reservoir spends down (the memory primitive
/// [`laws::radiogenic_decay`]), so the radiogenic heat production ([`laws::radiogenic_heat`] over the
/// reservoir) falls and the interior cools, the spent-world relaxation the static-source convection step
/// cannot express on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SecularState {
    /// The column's thermal state.
    pub column: ColumnState,
    /// The heat-producing isotope reservoir (concentration), spending down over the clock.
    pub reservoir: Fixed,
}

/// One secular step over a world-clock tick: decay the isotope reservoir, recompute the radiogenic heat
/// production it now supplies, and apply the convection step under that (falling) production. `decay_constant`
/// is the isotope's per-tick decay rate and `specific_heat_production` its heat per unit reservoir; the
/// step reuses [`convection_step`] with the recomputed heat production, so the whole convection composition
/// (buoyancy, onset, flow, advection, conduction) runs under a source that dies over deep time.
// @derives: the interior column's secular thermal history <- radiogenic_decay (the isotope reservoir spending down over the world clock) feeding radiogenic_heat (the falling heat production) into the convection step, so the interior warms under radiogenic heating and cools as the sources decay; no authored cooling knob, the source history is the decaying reservoir
pub fn secular_step(
    state: &SecularState,
    p: &ColumnParams,
    decay_constant: Fixed,
    specific_heat_production: Fixed,
) -> SecularState {
    let reservoir = laws::radiogenic_decay(state.reservoir, decay_constant, p.dt);
    let heat_production = laws::radiogenic_heat(reservoir, specific_heat_production);
    let column = convection_step(
        &state.column,
        &ColumnParams {
            heat_production,
            ..*p
        },
    );
    SecularState { column, reservoir }
}

/// March the secular step over `ticks` world-clock ticks, returning the interior's state after that span of
/// geological time. Deterministic and bounded by `ticks` (never an unbounded spin), the time-marching
/// counterpart to the fixed-`H` relaxation [`convection_solve`].
pub fn secular_history(
    initial: SecularState,
    p: &ColumnParams,
    decay_constant: Fixed,
    specific_heat_production: Fixed,
    ticks: u64,
) -> SecularState {
    let mut state = initial;
    for _ in 0..ticks {
        state = secular_step(&state, p, decay_constant, specific_heat_production);
    }
    state
}

/// Populate one column's INTERIOR fields on A's [`GeodynamicColumn`] contract from the interior chain,
/// SNAPSHOT-APPLY (gate ruling, #176): read the start-of-tick `snapshot` column and return the end-of-tick
/// column. The interior reads only the snapshot (its resident `temperature` and the surface lane's
/// `crustal_density`), so whichever lane evaluates first reads the same values and the boundary is
/// order-independent, no cross-lane evaluation order pinned. The interior writes its continuous state
/// (`temperature`, `rayleigh`, `convective_stress`) and the `isostatic_elevation` it derives by floating the
/// surface-written crust on the world's mantle; the surface lane's own fields pass through unchanged. A
/// missing density or thickness yields the zero-default elevation (the absence convention), never a
/// fabricated one.
///
/// `mantle_density` is DERIVED, never authored (gate ruling from the owner, #176): it is A's petrology kernel
/// [`derive_mantle_density`] over the world's mantle COMPOSITION at the mantle's temperature and pressure, so
/// no density is a bare per-world number. The caller passes the derived value (the derivation is threaded so
/// the boundary stays snapshot-clean). Byte-neutral: no scenario calls this yet, so it is a dormant capability
/// (the interior law-forms' pattern), and the arming (a scenario running it and the surface reading the
/// result) is the separately-sequenced step.
pub fn populate_interior_column(
    snapshot: GeodynamicColumn,
    p: &ColumnParams,
    mantle_density: Fixed,
) -> GeodynamicColumn {
    // The resident interior state carries only the continuous temperature; the discrete convecting condition is
    // derived inside the step from the Rayleigh number (a false prior latch makes the onset reversible, so a
    // cooling column can stop convecting), never a stored flag.
    let state = ColumnState {
        temperature: snapshot.temperature,
        convecting: false,
    };
    let readout = column_readout(&state, p);
    let isostatic_elevation = civsim_physics::geodynamics::airy_isostatic_elevation(
        snapshot.crustal_density,
        mantle_density,
        snapshot.crustal_thickness,
    )
    .unwrap_or(Fixed::ZERO);
    GeodynamicColumn {
        // The surface lane's fields pass through (the interior does not write them, snapshot-apply).
        crustal_density: snapshot.crustal_density,
        crustal_thickness: snapshot.crustal_thickness,
        // The interior lane's writes.
        isostatic_elevation,
        temperature: readout.temperature,
        convective_stress: readout.convective_stress,
        rayleigh: readout.rayleigh,
    }
}

/// Snapshot-apply the interior population over a whole [`GeodynamicField`]: read the start-of-tick `snapshot`
/// field and return the end-of-tick field, each column populated against the snapshot (order-independent, gate
/// ruling #176). The per-world interior parameters and mantle density are supplied by the caller (a future
/// scenario). A column with no resident state is not walked, so an EMPTY field yields an empty field and the
/// pass is byte-neutral over an unarmed geology; the walk is canonical `Coord3` order, so the fold is
/// reproducible and thread-invariant. Called by no scenario yet.
pub fn step_interior_field(
    snapshot: &GeodynamicField,
    p: &ColumnParams,
    mantle_density: Fixed,
) -> GeodynamicField {
    let mut next = GeodynamicField::new();
    for (coord, column) in snapshot.iter() {
        next.set(coord, populate_interior_column(column, p, mantle_density));
    }
    next
}

/// Derive the mantle density from the world's mantle COMPOSITION, never an authored number (gate ruling from
/// the owner, #176): A's petrology kernel ([`civsim_physics::petrology::crustal_density`], a GENERAL
/// composition-to-density derivation despite the crust-specific name) minimizes the stable mineral assemblage
/// of the composition at the mantle's temperature and pressure and reads its mass over volume, so the density
/// is what the material IS under its conditions, neither a fundamental constant nor a bare per-world scalar.
/// The mantle temperature is the interior heat chain's own thermal state (the column temperature the
/// convection evolution carries), and the pressure is the lithostatic pressure at the mantle's depth; a
/// reference-pressure first pass breaks the mild density-depends-on-pressure self-consistency (a short
/// fixed-point refinement is the follow-on, both derivations, nothing authored). Returns `None` when the
/// composition reaches no assemblage or a phase is missing from the data (fail-loud, never a fabricated
/// density). The isostasy floats the crust on this derived mantle density.
pub fn derive_mantle_density(
    mantle_composition: &[(String, Fixed)],
    mantle_temperature: Fixed,
    reference_pressure_bar: Fixed,
    registry: &civsim_physics::petrology_data::PhaseRegistry,
    table: &civsim_physics::periodic::PeriodicTable,
) -> Option<Fixed> {
    civsim_physics::petrology::crustal_density(
        mantle_composition,
        mantle_temperature,
        reference_pressure_bar,
        registry,
        table,
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
            stress_max: Fixed::from_int(1_000_000),
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

    #[test]
    fn the_reservoir_decays_and_the_source_dies_over_the_thermal_history() {
        let (column0, params) = column(Fixed::ZERO);
        let decay = Fixed::from_ratio(1, 4); // 25% per tick, exactly representable
        let specific_heat_production = Fixed::from_int(4);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        // One step: the reservoir spends down first-order, 100 -> 75.
        let s1 = secular_step(&initial, &params, decay, specific_heat_production);
        assert_eq!(
            s1.reservoir,
            Fixed::from_int(75),
            "the isotope reservoir spends down first-order"
        );
        // It keeps falling, monotone.
        let s2 = secular_step(&s1, &params, decay, specific_heat_production);
        assert!(
            s2.reservoir < s1.reservoir,
            "the reservoir decays monotonically"
        );
        // Over a long history the source is spent (the reservoir approaches zero).
        let late = secular_history(initial, &params, decay, specific_heat_production, 200);
        assert!(
            late.reservoir < Fixed::from_ratio(1, 100),
            "the heat source is spent after long geological time"
        );
    }

    #[test]
    fn a_decaying_source_leaves_the_interior_cooler_than_a_sustained_one() {
        // A decaying reservoir loses its heat source over time, so the interior cools below what a
        // sustained (non-decaying) source would hold: the spent-world relaxation.
        let (column0, params) = column(Fixed::ZERO);
        let specific_heat_production = Fixed::from_int(4);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        let decaying = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            specific_heat_production,
            300,
        );
        // Zero decay constant: the source never spends down, so it sustains a warmer interior.
        let sustained =
            secular_history(initial, &params, Fixed::ZERO, specific_heat_production, 300);
        assert!(
            decaying.column.temperature < sustained.column.temperature,
            "the interior with a decaying source ends cooler than one with a sustained source"
        );
    }

    #[test]
    fn the_secular_history_is_deterministic() {
        let (column0, params) = column(Fixed::ZERO);
        let initial = SecularState {
            column: column0,
            reservoir: Fixed::from_int(100),
        };
        let a = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            Fixed::from_int(4),
            100,
        );
        let b = secular_history(
            initial,
            &params,
            Fixed::from_ratio(1, 4),
            Fixed::from_int(4),
            100,
        );
        assert_eq!(
            a, b,
            "the same synthetic thermal history reproduces the same outcome"
        );
    }

    // --- The interior column-wiring (#176) ---

    #[test]
    fn the_readout_exposes_the_continuous_state_and_a_hot_column_convects() {
        // A hot column (well above the reference) reaches a super-critical Rayleigh number, so the derived
        // convecting condition (Rayleigh against the critical value) is on, and the readout carries the
        // continuous quantities the contract stores.
        let (state, params) = column(Fixed::from_int(1)); // a low Ra_crit, so the hot column convects
        let readout = column_readout(&state, &params);
        assert!(
            readout.rayleigh > params.ra_crit,
            "the hot column is super-critical"
        );
        assert!(
            readout.convective_stress > Fixed::ZERO,
            "a convecting column exerts a driving stress"
        );
        // The stepped temperature is the convection_step result (the readout reuses it).
        assert_eq!(
            readout.temperature,
            convection_step(&state, &params).temperature
        );
    }

    #[test]
    fn convection_is_reversible_a_cold_column_does_not_convect() {
        // The gate's ruling: no stored convecting flag, so convection can CEASE on a cooling world. A column at
        // the reference temperature has no buoyancy, a sub-critical Rayleigh number, and no convective stress.
        let (_, params) = column(Fixed::from_int(1000));
        let cold = ColumnState {
            temperature: params.reference_temperature,
            convecting: false,
        };
        let readout = column_readout(&cold, &params);
        assert_eq!(
            readout.rayleigh,
            Fixed::ZERO,
            "no contrast, no convective vigor"
        );
        assert_eq!(
            readout.convective_stress,
            Fixed::ZERO,
            "a still interior drives no stress"
        );
    }

    #[test]
    fn populate_writes_the_interior_fields_and_the_snapshot_isostasy() {
        // The interior populates its continuous fields and the isostatic elevation, reading the surface lane's
        // crustal_density from the SNAPSHOT (snapshot-apply). The mantle density here stands in for the
        // petrology-derived value (derive_mantle_density over the mantle composition); the test supplies it
        // directly to isolate the wiring.
        let (_, params) = column(Fixed::from_int(1));
        let mantle_density = Fixed::from_ratio(33, 10); // a derived-density stand-in for the wiring test
        let snapshot = GeodynamicColumn {
            crustal_density: Fixed::from_ratio(265, 100), // written by the surface lane (felsic)
            crustal_thickness: Fixed::from_int(35_000),
            temperature: Fixed::from_int(400),
            ..GeodynamicColumn::default()
        };
        let next = populate_interior_column(snapshot, &params, mantle_density);
        // The surface field passes through unchanged (the interior does not write it).
        assert_eq!(next.crustal_density, snapshot.crustal_density);
        // The interior wrote its continuous state and the isostatic elevation from the snapshot crust.
        assert!(next.rayleigh > Fixed::ZERO);
        assert!(
            next.isostatic_elevation > Fixed::ZERO,
            "a felsic column floats above the reference"
        );
        let expected = civsim_physics::geodynamics::airy_isostatic_elevation(
            snapshot.crustal_density,
            mantle_density,
            snapshot.crustal_thickness,
        )
        .unwrap();
        assert_eq!(
            next.isostatic_elevation, expected,
            "the isostasy reads the snapshot crust and mantle"
        );
    }

    #[test]
    fn the_wiring_convection_is_reversible_a_cooled_column_ceases() {
        // The latch guardrail (gate ruling, #176): the resident contract stores no convecting flag, so a column
        // that once convected does NOT stay convecting against a fallen Rayleigh number. Populate a hot column
        // (it convects, stress positive), then feed its result back cooled to the reference, and the re-populated
        // column reads zero stress: the stress keys off the CURRENT Rayleigh number, reversibly, never a
        // persisted onset latch overriding it.
        let (_, params) = column(Fixed::from_int(1));
        let hot = GeodynamicColumn {
            temperature: Fixed::from_int(2000),
            ..GeodynamicColumn::default()
        };
        let convecting = populate_interior_column(hot, &params, Fixed::from_ratio(33, 10));
        assert!(
            convecting.convective_stress > Fixed::ZERO,
            "the hot column convects and drives a stress"
        );
        // Now the column has cooled to its reference (no contrast): re-populate against that snapshot.
        let cooled = GeodynamicColumn {
            temperature: params.reference_temperature,
            ..convecting
        };
        let ceased = populate_interior_column(cooled, &params, Fixed::from_ratio(33, 10));
        assert_eq!(
            ceased.convective_stress,
            Fixed::ZERO,
            "a cooled column ceases convecting, no latch keeps the stress alive"
        );
        assert_eq!(
            ceased.rayleigh,
            Fixed::ZERO,
            "the vigor falls with the contrast"
        );
    }

    #[test]
    fn the_boundary_layer_thins_with_vigor_so_a_hotter_column_drives_more_stress() {
        // The derived boundary layer L = depth * Ra^(-1/3): a more vigorous column has a thinner boundary layer
        // and a higher driving stress, the derive-clean thinning (not the depth reference-pass).
        let (_, params) = column(Fixed::from_int(1));
        let warm = column_readout(
            &ColumnState {
                temperature: Fixed::from_int(600),
                convecting: false,
            },
            &params,
        );
        let hot = column_readout(
            &ColumnState {
                temperature: Fixed::from_int(2000),
                convecting: false,
            },
            &params,
        );
        assert!(
            hot.rayleigh > warm.rayleigh,
            "the hotter column is more vigorous"
        );
        assert!(
            hot.convective_stress > warm.convective_stress,
            "a thinner boundary layer under higher vigor concentrates more driving stress"
        );
    }

    #[test]
    fn an_empty_field_step_is_byte_neutral() {
        // Snapshot-apply over an unarmed geology walks no columns, so it yields an empty field (folds nothing
        // into state_hash), the dormant byte-neutral guarantee.
        let (_, params) = column(Fixed::from_int(1));
        let empty = GeodynamicField::new();
        let next = step_interior_field(&empty, &params, Fixed::from_ratio(33, 10));
        assert!(
            next.is_empty(),
            "an unarmed geology stays empty and byte-neutral"
        );
    }
}
