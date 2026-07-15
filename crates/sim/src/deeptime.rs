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

//! THE DEEP-TIME RUN (seam 4, the wiring): the derived planet EVOLVES over geological time. The ruling was that
//! the physics is already built and dormant (the interior convection, the melt rung, the impact chain, the star
//! aging); seam 4 steps it on a clock so the surface CHANGES and the provinces write themselves onto that clock,
//! never an authored map. This module is the run state and the tick; the viewer's time control drives it and the
//! surface re-derives and redraws each step.
//!
//! Slice 1 (this): the INTERIOR THERMAL EVOLUTION. A lateral field of mantle columns steps its convection
//! ([`convection_step`]) each tick, so the interior cools and convects over deep time. Lateral variation EMERGES
//! rather than being painted: a column with more radiogenic heat or a hotter start relaxes to a different thermal
//! state than its neighbour, so the provinces are the written record of each column's own history. Nothing is
//! authored: the columns start uniform (a fresh planet has no thermal history yet) and diverge only by their own
//! parameters and the convection law.
//!
//! The later slices wire onto this same clock: the melt-driven VOLCANISM (the interior temperature crossing the
//! seam-6 solidus extracts melt that emplaces crust, thickening it where the mantle is hot, which retires the
//! authored crustal-thickness fixture as the thickness becomes the accumulated melt), the IMPACT CHAIN (the
//! accretion-tail flux draws impacts that crater and blanket the surface), and the STAR AGING (the main-sequence
//! luminosity climbing over the star's life warms the surface). Each is a step term added here; slice 1 is the
//! interior spine they hang on.
//!
//! Determinism (Principle 3, Principle 10): [`convection_step`] is a pure function and the columns are walked in
//! index order, so the tick is a pure function of the state and the parameters, worker-invariant. Dormant: no
//! scenario or viewer drives it yet (the time control is the next slice), so it is byte-neutral over the run
//! path.

use crate::geodynamics::{convection_step, ColumnParams, ColumnState};
use civsim_core::Fixed;

/// The deep-time state of a derived planet: a lateral field of interior mantle columns (one per surface cell or
/// province) and the geological time elapsed. The field is the spatial extent the lateral variation lives in:
/// each column carries its own thermal state, so provinces emerge from columns that evolve differently, never an
/// authored arrangement. The crust, the impact record, and the star's age join this state in the later slices.
#[derive(Clone, Debug, PartialEq)]
pub struct DeepTimeState {
    /// The interior mantle columns, one per lateral cell, each evolving by its own convection over deep time.
    pub columns: Vec<ColumnState>,
    /// The geological time elapsed (megayears), the clock the provinces are written against.
    pub elapsed_myr: Fixed,
}

impl DeepTimeState {
    /// The initial state of a fresh planet: `n_cells` mantle columns all at the same starting temperature, none
    /// convecting yet (the Rayleigh-onset latch fires per column as each steepens). A young planet has NO lateral
    /// thermal history, so the field starts uniform; the variation is what the deep-time run produces, not an
    /// authored initial map.
    pub fn young(n_cells: usize, initial_temperature: Fixed) -> Self {
        DeepTimeState {
            columns: vec![
                ColumnState {
                    temperature: initial_temperature,
                    convecting: false,
                };
                n_cells
            ],
            elapsed_myr: Fixed::ZERO,
        }
    }
}

/// Advance the deep-time state by one tick: step every interior column's convection ([`convection_step`]) against
/// its parameters and accumulate the elapsed geological time. `params` is either ONE entry broadcast to every
/// column (a laterally uniform world) or one per column (each cell's own composition and radiogenic budget, the
/// source of lateral variation); any other length is a caller mismatch and the column falls back to the first
/// entry so the tick never panics. `dt_myr` is the tick's geological duration, the clock bookkeeping (the
/// physical timestep the convection kernel integrates is the parameters' own `dt`). Returns the next state.
/// Deterministic and worker-invariant (a pure per-column map in index order). `None` if `params` is empty (there
/// is nothing to step the columns against), fail-loud rather than a silent no-op.
pub fn step_deep_time(
    state: &DeepTimeState,
    params: &[ColumnParams],
    dt_myr: Fixed,
) -> Option<DeepTimeState> {
    if params.is_empty() {
        return None;
    }
    let per_column = params.len() == state.columns.len();
    let columns = state
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let p = if per_column { &params[i] } else { &params[0] };
            convection_step(col, p)
        })
        .collect();
    Some(DeepTimeState {
        columns,
        elapsed_myr: state.elapsed_myr.saturating_add(dt_myr),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representable-scaled mantle column parameter set, mirroring the convection tests: the values are the
    // engine-scaled illustrative mantle (depth and viscosity are the representable-scaled forms the convection
    // kernel documents), enough to exercise the deep-time evolution deterministically. `ra_crit` high keeps a
    // column conductive so its relaxation is monotone and easy to assert; `heat_production` is the per-column
    // knob the lateral-variation test varies.
    fn mantle_params(heat_production: i32) -> ColumnParams {
        ColumnParams {
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
            heat_production: Fixed::from_int(heat_production),
            ra_crit: Fixed::from_int(1_000_000_000),
            ra_max: Fixed::from_int(1_000_000),
            v_max: Fixed::from_int(1_000_000),
            flux_max: Fixed::from_int(1_000_000),
            stress_max: Fixed::from_int(1_000_000),
            dt: Fixed::ONE,
        }
    }

    #[test]
    fn the_interior_evolves_over_deep_time() {
        // A hot young mantle relaxes toward its conductive steady state over successive ticks: the temperature
        // MOVES (the world is not static), and the clock accumulates.
        let start = DeepTimeState::young(4, Fixed::from_int(2000));
        let params = [mantle_params(5)];
        let mut state = start.clone();
        for _ in 0..5 {
            state = step_deep_time(&state, &params, Fixed::from_int(100)).expect("steps");
        }
        assert!(
            state.columns[0].temperature != start.columns[0].temperature,
            "the interior temperature evolves over deep time, not static"
        );
        assert!(
            state.columns[0].temperature < start.columns[0].temperature,
            "a hot mantle with modest radiogenic heat cools toward its steady state"
        );
        assert_eq!(
            state.elapsed_myr,
            Fixed::from_int(500),
            "the deep-time clock accumulates (5 ticks of 100 Myr)"
        );
    }

    #[test]
    fn lateral_variation_emerges_from_per_column_heat() {
        // The provinces: two columns start identical but carry different radiogenic budgets, so after deep time
        // they hold different temperatures. The lateral variation is the WRITTEN record of each column's history,
        // never an authored map (the field started uniform).
        let start = DeepTimeState::young(2, Fixed::from_int(2000));
        assert_eq!(
            start.columns[0], start.columns[1],
            "a fresh planet starts laterally uniform"
        );
        let params = [mantle_params(5), mantle_params(400)];
        let mut state = start;
        for _ in 0..8 {
            state = step_deep_time(&state, &params, Fixed::from_int(100)).expect("steps");
        }
        assert!(
            state.columns[0].temperature != state.columns[1].temperature,
            "columns with different radiogenic heat diverge into provinces, got {} vs {}",
            state.columns[0].temperature.to_f64_lossy(),
            state.columns[1].temperature.to_f64_lossy()
        );
        assert!(
            state.columns[1].temperature > state.columns[0].temperature,
            "the more radiogenic column stays hotter"
        );
    }

    #[test]
    fn the_tick_is_deterministic() {
        let start = DeepTimeState::young(6, Fixed::from_int(1800));
        let params = [mantle_params(50)];
        let a = step_deep_time(&start, &params, Fixed::from_int(100)).expect("a");
        let b = step_deep_time(&start, &params, Fixed::from_int(100)).expect("b");
        assert_eq!(
            a, b,
            "the tick is a pure function of the state and parameters"
        );
    }

    #[test]
    fn an_empty_parameter_set_fails_loud() {
        let start = DeepTimeState::young(3, Fixed::from_int(1500));
        assert!(
            step_deep_time(&start, &[], Fixed::from_int(100)).is_none(),
            "no parameters to step against fails loud, never a silent no-op"
        );
    }
}
