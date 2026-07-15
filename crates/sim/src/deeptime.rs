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
use civsim_physics::melting::adiabatic_melt_column;

/// The deep-time state of a derived planet: a lateral field of interior mantle columns (one per surface cell or
/// province), the crust each column has BUILT so far, and the geological time elapsed. The field is the spatial
/// extent the lateral variation lives in: each column carries its own thermal state and its own accumulated
/// crust, so provinces emerge from columns that evolve differently, never an authored arrangement. The impact
/// record and the star's age join this state in the later slices.
#[derive(Clone, Debug, PartialEq)]
pub struct DeepTimeState {
    /// The interior mantle columns, one per lateral cell, each evolving by its own convection over deep time.
    pub columns: Vec<ColumnState>,
    /// The crust each column has BUILT (kilometres), one per lateral cell, accumulated from the melt its
    /// interior has delivered over the run so far. This is the DERIVED crustal thickness (the seam-6
    /// adiabatic-melt column integrated over the deep-time history), the field that retires the authored 30 km
    /// crustal-thickness fixture: a hot, long-lived column builds thick crust, a cold one thin, the provinces.
    pub crust_thickness_km: Vec<Fixed>,
    /// The geological time elapsed (megayears), the clock the provinces are written against.
    pub elapsed_myr: Fixed,
}

impl DeepTimeState {
    /// The initial state of a fresh planet: `n_cells` mantle columns all at the same starting temperature, none
    /// convecting yet (the Rayleigh-onset latch fires per column as each steepens), and no crust built yet (a
    /// fresh planet has no accumulated crust, it is what the melt history produces). A young planet has NO lateral
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
            crust_thickness_km: vec![Fixed::ZERO; n_cells],
            elapsed_myr: Fixed::ZERO,
        }
    }
}

/// The per-world melt-and-crust parameters the deep-time volcanism reads: the seam-6 adiabatic-melt-column
/// closure's inputs (the solidus surface value and slope, the adiabat slope, the melt productivity, the source
/// density, and gravity), plus the mantle PROCESSING TIME over which one melt column's worth of crust is
/// delivered to the surface. Every field is a floor value or a per-world datum; the processing time is
/// reserved-with-basis (the mantle overturn / melt-delivery timescale that converts the column's total crust to
/// a per-tick production rate, ~100 Myr to 1 Gyr for a silicate mantle), surfaced not fabricated.
#[derive(Clone, Copy, Debug)]
pub struct MeltParams {
    /// The solidus surface temperature (K), the seam-6 melt-column input.
    pub solidus_surface_k: Fixed,
    /// The solidus slope (K per GPa).
    pub solidus_slope_k_per_gpa: Fixed,
    /// The adiabat slope (K per GPa).
    pub adiabat_slope_k_per_gpa: Fixed,
    /// The melt productivity (per GPa).
    pub productivity_per_gpa: Fixed,
    /// The melt source density (kg per cubic metre).
    pub source_density_kg_per_m3: Fixed,
    /// Gravity (m per second squared).
    pub gravity_m_per_s2: Fixed,
    /// The mantle processing time (megayears), the overturn timescale one melt column's crust is delivered over.
    pub processing_time_myr: Fixed,
}

/// THE VOLCANISM: the crust a column's interior melt delivers in one tick. The column's current interior
/// temperature is the mantle potential temperature the seam-6 [`adiabatic_melt_column`] (McKenzie-Bickle)
/// crosses against the solidus; the crust a full melt column makes is delivered over the mantle processing time,
/// so a tick of `dt_myr` adds `crust_thickness_km * dt_myr / processing_time`. A sub-solidus column (a mantle
/// colder than its surface solidus) makes no melt and no crust: zero, never negative, never a fabricated crust.
/// This is derive-first (the crust is the melt the interior produces, keyed on the column's own temperature) and
/// the province-builder: a hotter, longer-lived column accumulates more crust than its neighbour, so the surface
/// crust becomes the written readout of the interior's melt history. Returns the crust increment (kilometres).
pub fn crust_growth(potential_temperature_k: Fixed, melt: &MeltParams, dt_myr: Fixed) -> Fixed {
    if melt.processing_time_myr <= Fixed::ZERO || dt_myr <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    match adiabatic_melt_column(
        potential_temperature_k,
        melt.solidus_surface_k,
        melt.solidus_slope_k_per_gpa,
        melt.adiabat_slope_k_per_gpa,
        melt.productivity_per_gpa,
        melt.source_density_kg_per_m3,
        melt.gravity_m_per_s2,
    ) {
        Some(column) => column
            .crust_thickness_km
            .checked_mul(dt_myr)
            .and_then(|c| c.checked_div(melt.processing_time_myr))
            .unwrap_or(Fixed::ZERO),
        None => Fixed::ZERO,
    }
}

/// Advance the deep-time state by one tick: step every interior column's convection ([`convection_step`]), grow
/// each column's crust from the melt its interior now delivers ([`crust_growth`]), and accumulate the elapsed
/// geological time. `column_params` is either ONE entry broadcast to every column (a laterally uniform world) or
/// one per column (each cell's own composition and radiogenic budget, the source of lateral variation); any other
/// length is a caller mismatch and the column falls back to the first entry so the tick never panics. `melt` is
/// the shared per-world volcanism parameters. `dt_myr` is the tick's geological duration. The crust ACCUMULATES:
/// a crust the interior once built stays, so the surface thickness is the integrated melt history, hot columns
/// building thicker crust (the provinces, and the derived thickness that retires the 30 km fixture). Returns the
/// next state. Deterministic and worker-invariant (a pure per-column map in index order). `None` if
/// `column_params` is empty (nothing to step against), fail-loud rather than a silent no-op.
pub fn step_deep_time(
    state: &DeepTimeState,
    column_params: &[ColumnParams],
    melt: &MeltParams,
    dt_myr: Fixed,
) -> Option<DeepTimeState> {
    if column_params.is_empty() {
        return None;
    }
    let per_column = column_params.len() == state.columns.len();
    let columns: Vec<ColumnState> = state
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let p = if per_column {
                &column_params[i]
            } else {
                &column_params[0]
            };
            convection_step(col, p)
        })
        .collect();
    // The volcanism: each column's stepped interior temperature delivers crust over the tick, added to the crust
    // it has already built (a made crust stays; it does not un-form when the mantle cools). The accumulated
    // thickness is the derived crust, laterally varying by each column's own melt history.
    let crust_thickness_km: Vec<Fixed> = state
        .crust_thickness_km
        .iter()
        .zip(columns.iter())
        .map(|(prev, col)| prev.saturating_add(crust_growth(col.temperature, melt, dt_myr)))
        .collect();
    Some(DeepTimeState {
        columns,
        crust_thickness_km,
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

    // The melt-and-crust parameters, the McKenzie-Bickle peridotite values the seam-6 melt-column test uses
    // (solidus 1373 K, 130 K/GPa; adiabat 15.5 K/GPa; productivity 0.12/GPa; source 3300 kg/m^3; g ~10), plus a
    // 100 Myr processing time. A column above the ~1373 K surface solidus makes crust; a cooled sub-solidus one
    // makes none.
    fn melt_params() -> MeltParams {
        MeltParams {
            solidus_surface_k: Fixed::from_int(1373),
            solidus_slope_k_per_gpa: Fixed::from_int(130),
            adiabat_slope_k_per_gpa: Fixed::from_ratio(155, 10),
            productivity_per_gpa: Fixed::from_ratio(12, 100),
            source_density_kg_per_m3: Fixed::from_int(3300),
            gravity_m_per_s2: Fixed::from_int(10),
            processing_time_myr: Fixed::from_int(100),
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
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
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
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
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
    fn the_crust_grows_from_volcanism_and_the_provinces_thicken_where_the_mantle_is_hot() {
        // The volcanism: a hot mantle builds crust over deep time (the accumulated derived thickness that retires
        // the 30 km fixture), and a hotter, more-radiogenic column builds THICKER crust than its cooler neighbour,
        // so the crust field is the written record of the interior's melt history, the provinces in relief.
        let start = DeepTimeState::young(2, Fixed::from_int(2000));
        assert_eq!(
            start.crust_thickness_km[0],
            Fixed::ZERO,
            "a fresh planet has no crust yet"
        );
        let params = [mantle_params(5), mantle_params(600)];
        let mut state = start;
        for _ in 0..10 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert!(
            state.crust_thickness_km[0] > Fixed::ZERO,
            "the interior melt builds crust over deep time"
        );
        assert!(
            state.crust_thickness_km[1] > state.crust_thickness_km[0],
            "the hotter, more-radiogenic column builds thicker crust (a province), got {} vs {}",
            state.crust_thickness_km[1].to_f64_lossy(),
            state.crust_thickness_km[0].to_f64_lossy()
        );
    }

    #[test]
    fn a_sub_solidus_mantle_builds_no_crust() {
        // A mantle colder than its surface solidus (1373 K) makes no melt, so it builds no crust: the volcanism is
        // the interior's own readout, not an authored floor of crust.
        assert_eq!(
            crust_growth(Fixed::from_int(1000), &melt_params(), Fixed::from_int(100)),
            Fixed::ZERO,
            "a sub-solidus column makes no melt and no crust"
        );
        assert!(
            crust_growth(Fixed::from_int(1800), &melt_params(), Fixed::from_int(100)) > Fixed::ZERO,
            "a super-solidus column does build crust"
        );
    }

    #[test]
    fn the_tick_is_deterministic() {
        let start = DeepTimeState::young(6, Fixed::from_int(1800));
        let params = [mantle_params(50)];
        let a = step_deep_time(&start, &params, &melt_params(), Fixed::from_int(100)).expect("a");
        let b = step_deep_time(&start, &params, &melt_params(), Fixed::from_int(100)).expect("b");
        assert_eq!(
            a, b,
            "the tick is a pure function of the state and parameters"
        );
    }

    #[test]
    fn an_empty_parameter_set_fails_loud() {
        let start = DeepTimeState::young(3, Fixed::from_int(1500));
        assert!(
            step_deep_time(&start, &[], &melt_params(), Fixed::from_int(100)).is_none(),
            "no parameters to step against fails loud, never a silent no-op"
        );
    }
}
