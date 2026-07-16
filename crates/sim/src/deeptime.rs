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
//! The STAR-AGING slice (this): the star ages on the SAME deep-time clock and BRIGHTENS. A main-sequence star's
//! core hydrogen fuses to helium, the core's mean molecular weight rises, so the core contracts and heats and the
//! luminosity CLIMBS over the star's life. The brightening is DERIVED, never an authored curve: the star's
//! main-sequence lifetime `t_ms` is the nuclear timescale (fuel over burn rate, `t_sun * (M/M_sun) / (L_zams/L_sun)`)
//! from the star's mass and its zero-age luminosity, which the stellar front-end ([`crate::stellar`]) already
//! gives, and the current luminosity is the ZAMS anchor climbed by the Gough factor `1 / (1 - c * t/t_ms)`. The
//! surface warmth reads [`DeepTimeState::stellar_luminosity_ratio`] each step and warms as it climbs. Past `t_ms`
//! the star LEAVES the main sequence (the post-main-sequence giant branch is a separate stellar-evolution arc), so
//! the brightening caps there rather than extrapolating a regime this law does not describe.
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
/// crust, so provinces emerge from columns that evolve differently, never an authored arrangement. The star's age
/// rides this same clock (`star_age_start_myr + elapsed_myr`); the impact record joins the state in a later slice.
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
    /// The star's main-sequence AGE (megayears) when this run began: the star and the planet share one clock, so
    /// the star's current age is this start age plus `elapsed_myr` ([`DeepTimeState::star_age_myr`]). A fresh
    /// planet co-forming with a zero-age star begins at zero; a run beginning around an already-aged star carries
    /// the star's prior age here. This is the per-run start age the brightening is measured against, not an
    /// authored map.
    pub star_age_start_myr: Fixed,
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
            star_age_start_myr: Fixed::ZERO,
        }
    }

    /// The same young planet, but around a star that already carries `age_myr` of main-sequence life at the run's
    /// start (a run beginning mid-life rather than at the star's zero-age main sequence). The planet's own history
    /// still begins fresh (uniform columns, no crust); only the star's clock is offset. Returns the state so it
    /// chains off [`DeepTimeState::young`].
    pub fn with_star_start_age(mut self, age_myr: Fixed) -> Self {
        self.star_age_start_myr = age_myr;
        self
    }

    /// The star's CURRENT main-sequence age (megayears): its age when the run began plus the geological time
    /// elapsed since. The star and the planet share one deep-time clock, so as the run steps the star ages with it.
    pub fn star_age_myr(&self) -> Fixed {
        self.star_age_start_myr.saturating_add(self.elapsed_myr)
    }

    /// The CURRENT stellar luminosity ratio `L(t)/L_sun` the deep-time surface reads at this run's present age.
    /// This is the thread from the star's aging to the planet's warmth: the surface reads this each step and warms
    /// as it climbs ([`current_luminosity_ratio`]). `None` once the star has left the main sequence at `t_ms`, the
    /// post-main-sequence regime being a separate stellar-evolution arc.
    pub fn stellar_luminosity_ratio(&self, aging: &StarAgingParams) -> Option<Fixed> {
        current_luminosity_ratio(aging, self.star_age_myr())
    }
}

/// The per-world melt-and-crust parameters the deep-time volcanism reads: the seam-6 adiabatic-melt-column
/// closure's inputs (the solidus surface value and slope, the adiabat slope, the melt productivity, the source
/// density, and gravity), plus the mantle PROCESSING TIME over which one melt column's worth of crust is
/// delivered to the surface. The solidus surface value and slope are DERIVED from the world's own endmember
/// signatures upstream ([`civsim_materials::surface_composition::SurfaceComposition::solidus_surface_k`]), never
/// an authored peridotite value; the adiabat, productivity, source density, and gravity are floor values or
/// per-world data. The processing time is a flagged DERIVE-DOWN, not an independent knob: it is the overturn /
/// melt-delivery timescale `convecting_depth / convective_velocity`, and BOTH ingredients already exist in the
/// convection kernel ([`crate::geodynamics::convecting_mantle_depth_m`] and the Stokes velocity
/// [`civsim_physics::laws::stokes_velocity`]), so it derives the moment those run at physical SI scale. It stays
/// reserved-with-basis only because the kernel currently runs on the representable-scaled interim operating point
/// (the Stokes velocity is physical only in SI), so the derive-down is coupled to the same SI / Tier-2 units
/// wiring that retires that operating point ([`province_column_params`]); ~100 Myr to 1 Gyr for a silicate
/// mantle is its surfaced-basis value, not a fabricated one.
#[derive(Clone, Copy, Debug)]
pub struct MeltParams {
    /// The solidus surface temperature (K), the seam-6 melt-column input. DERIVED upstream from the world's own
    /// endmember signatures, never an authored solidus.
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
    /// A flagged DERIVE-DOWN (`convecting_depth / convective_velocity`, both in the convection kernel), reserved
    /// only until the SI / Tier-2 units wiring lets the Stokes velocity run at physical scale; not an independent knob.
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
        // The star's start age is a per-run constant; its CURRENT age advances through `elapsed_myr` above, so the
        // star ages on the same clock as the interior without a separate step term.
        star_age_start_myr: state.star_age_start_myr,
    })
}

/// The star-aging parameters the deep-time run reads to derive the main-sequence brightening: the star's ZAMS
/// ANCHOR (its zero-age luminosity ratio and its mass ratio, both the stellar front-end's outputs at age 0), plus
/// the two reserved-with-basis constants of the nuclear-timescale-and-brightening law. The mechanism derives the
/// lifetime and the current luminosity from these four values; nothing here is an authored brightening curve. Both
/// constants are per-star DATA (a field, not a hardcoded inline value), so a world's star with different nuclear
/// physics is a data row, never a rewrite (admit-the-alien).
#[derive(Clone, Copy, Debug)]
pub struct StarAgingParams {
    /// `L_zams / L_sun`, the star's ZERO-AGE main-sequence luminosity ratio: the stellar front-end's output at age
    /// 0 ([`crate::stellar::luminosity_ratio`]), the anchor the brightening climbs from and recovers at age 0.
    pub zams_luminosity_ratio: Fixed,
    /// `M / M_sun`, the star's mass ratio: the FUEL term of the nuclear timescale (hydrogen fuel scales with mass).
    pub mass_ratio: Fixed,
    /// The SOLAR nuclear timescale (megayears): `t_sun`, the fuel-over-luminosity constant that sets the Sun's
    /// main-sequence lifetime, so every star's lifetime scales from it. Reserved-with-basis (the basis: the Sun's
    /// hydrogen fuel divided by its luminosity, the classic nuclear-timescale figure of order ten gigayears, cited
    /// rather than fabricated). Surfaced, not set here.
    pub solar_nuclear_timescale_myr: Fixed,
    /// The Gough brightening COEFFICIENT (dimensionless): the fraction the rising core mean-molecular-weight drives
    /// the luminosity up over a full main-sequence lifetime, expected positive (a main-sequence star brightens).
    /// Reserved-with-basis (the basis: the Gough 1981 solar-luminosity form `L(t)/L_zams = 1 / (1 - c * t/t_ms)`,
    /// with `c` of order 0.4 reproducing the ~30 to 40 percent zero-age-to-present solar brightening; per-star so a
    /// differently structured star can carry its own). Surfaced, not set here.
    pub brightening_coefficient: Fixed,
}

/// The star's main-sequence LIFETIME (megayears), DERIVED from the nuclear timescale: `t_ms = t_sun * (M/M_sun) /
/// (L_zams/L_sun)`, the fuel (proportional to mass) divided by the burn rate (the luminosity), anchored to the
/// Sun's `t_sun`. Because the ZAMS luminosity climbs steeply with mass (the mass-luminosity relation the stellar
/// front-end derives, `L ~ M^alpha` with `alpha ~ 3.5`), a heavier star has a SHORTER main-sequence life: it holds
/// more fuel but burns it far faster, so the lifetime shrinks as `M / M^alpha`. The lifetime is thus not authored:
/// it emerges from the mass and the ZAMS luminosity the stellar model already supplies, times the one reserved
/// nuclear-timescale anchor. `None` if the mass or the ZAMS luminosity ratio is non-positive (no fuel, or no burn
/// rate to divide by), or the product overflows.
pub fn main_sequence_lifetime_myr(aging: &StarAgingParams) -> Option<Fixed> {
    if aging.mass_ratio <= Fixed::ZERO || aging.zams_luminosity_ratio <= Fixed::ZERO {
        return None;
    }
    aging
        .solar_nuclear_timescale_myr
        .checked_mul(aging.mass_ratio)
        .and_then(|fuel| fuel.checked_div(aging.zams_luminosity_ratio))
}

/// The star's CURRENT luminosity ratio `L(t)/L_sun` at a main-sequence age `star_age_myr`, DERIVED from the ZAMS
/// anchor and the Gough brightening: `L(t)/L_sun = (L_zams/L_sun) / (1 - c * t/t_ms)`, with `t` the star's age,
/// `t_ms` its derived main-sequence lifetime ([`main_sequence_lifetime_myr`]), and `c` the Gough brightening
/// coefficient. As core hydrogen fuses to helium the core's mean molecular weight rises, the core contracts and
/// heats, and the luminosity climbs; the burnt fraction `f = t/t_ms` drives the climb, so the brightening EMERGES
/// from the age against the derived lifetime, never an authored curve. At age 0 the factor is exactly one, so the
/// ZAMS anchor is recovered. `None` once `t >= t_ms`: the star has exhausted its core hydrogen and LEAVES the main
/// sequence, the post-main-sequence giant branch being a separate stellar-evolution arc, capped here rather than
/// extrapolating a regime this law does not describe. Also `None` on a negative age, an undefined lifetime, or a
/// coefficient so large the denominator would hit zero before `t_ms` (a divergent regime the first-grade law does
/// not cover).
pub fn current_luminosity_ratio(aging: &StarAgingParams, star_age_myr: Fixed) -> Option<Fixed> {
    if star_age_myr < Fixed::ZERO {
        return None;
    }
    let t_ms = main_sequence_lifetime_myr(aging)?;
    if t_ms <= Fixed::ZERO || star_age_myr >= t_ms {
        // The star has left the main sequence (or has no defined lifetime): the post-main-sequence arc, not here.
        return None;
    }
    let fraction = star_age_myr.checked_div(t_ms)?; // f = t/t_ms, in [0, 1)
    let drop = aging.brightening_coefficient.checked_mul(fraction)?; // c * f
    let denom = Fixed::ONE.checked_sub(drop)?; // 1 - c * f
    if denom <= Fixed::ZERO {
        // A coefficient large enough to zero the denominator before t_ms diverges: outside the first-grade law.
        return None;
    }
    aging.zams_luminosity_ratio.checked_div(denom)
}

/// The number of convection PROVINCES that span a lateral distance, DERIVED from the convective cell
/// width. Rayleigh-Benard convection cells have a horizontal width of order the convecting-layer DEPTH
/// (an aspect ratio of order one), so the province width is `mantle_depth_m * cell_aspect` and the count
/// spanning `span_m` is `span_m / width`, floored, and at least one. The lateral province SCALE is thus
/// DERIVED from the convective physics, never a hand-set grid size: a deeper mantle makes fewer, wider
/// provinces and a larger planet more of them, so the texture's spatial scale is what the convection IS.
/// `cell_aspect` (the convective cell aspect ratio) is the CALLER's reserved-with-basis value (its basis:
/// the Rayleigh-Benard critical-mode cell aspect, order one, set by the mantle's convective boundary
/// regime), threaded in, never authored here. `None` on a non-physical input (a non-positive span, depth,
/// or aspect, or an overflow).
pub fn provinces_across(span_m: Fixed, mantle_depth_m: Fixed, cell_aspect: Fixed) -> Option<usize> {
    if span_m <= Fixed::ZERO || mantle_depth_m <= Fixed::ZERO || cell_aspect <= Fixed::ZERO {
        return None;
    }
    let width = mantle_depth_m.checked_mul(cell_aspect)?;
    let count = span_m.checked_div(width)?;
    Some(count.to_int().max(1) as usize)
}

/// The rigid-rigid Rayleigh-Benard marginal-stability CRITICAL RAYLEIGH NUMBER, the onset threshold for a mantle
/// layer bounded by rigid (no-slip) top and bottom. Paired with [`RIGID_RIGID_CRITICAL_WAVENUMBER`] as ONE
/// boundary regime: the classical rigid-rigid eigenvalue is {Ra_crit ~ 1707.76, a_c ~ 3.117}. This is an
/// analytic eigenvalue, not an authored knob (Chandrasekhar 1961, Hydrodynamic and Hydromagnetic Stability, ch.
/// II). A future marginal-stability solver would derive {Ra_crit, a_c, regime} jointly from the mantle's actual
/// mechanical boundaries; here the two are the cited rigid-rigid pair, kept together so the onset and the cell
/// scale can never drift into different regimes.
pub const RIGID_RIGID_RA_CRIT: Fixed = Fixed::from_int(1708);

/// The rigid-rigid Rayleigh-Benard marginal-stability CRITICAL WAVENUMBER a_c (inverse layer depths), the pair
/// mate of [`RIGID_RIGID_RA_CRIT`]. The horizontal mode that goes unstable first has a_c ~ 3.117 for rigid
/// boundaries (versus a_c = pi/sqrt(2) ~ 2.221 for free-free), so the convecting-cell half-wavelength is `pi /
/// a_c` ~ 1.008 layer depths (versus ~1.414 free-free). The province lateral SCALE reads `pi / a_c` for the cell
/// aspect, so the aspect and the onset threshold are the SAME regime by construction. Cited: Chandrasekhar (1961).
pub const RIGID_RIGID_CRITICAL_WAVENUMBER: Fixed = Fixed::from_int(3117).div(Fixed::from_int(1000));

/// One province's convection [`ColumnParams`], composed from the DERIVED per-planet inputs and the
/// convection kernel's REPRESENTABLE-SCALED operating point. The DERIVED inputs are threaded in where they
/// are physical and safe: the temperatures are real kelvin (so the solidus comparison the volcanism makes
/// is physical), `surface_gravity_m_s2` is the planet's DERIVED surface gravity, `convecting_depth_mm` is
/// the DERIVED convecting-mantle depth ([`crate::geodynamics::convecting_mantle_depth_m`]) expressed in
/// megametres (an O(1) length the depth-cubed Rayleigh term does not overflow, RETIRING the depth = 1
/// fixture), and `heat_production` is the per-province radiogenic budget (its lateral spread is what makes
/// the provinces diverge). The kernel's remaining DYNAMICAL quantities (viscosity, diffusivity, the
/// representable caps) are engine-scaled illustrative values, the documented interim the units plan retires
/// with the SI / Tier-2 units wiring: the raw SI mantle viscosity and the depth-cubed Rayleigh term overflow
/// Q32.32, so the kernel runs on a self-consistent scaled operating point rather than SI (a labelled fixture,
/// not an authored world-content value). This scaled operating point retires TOGETHER with the same SI /
/// Tier-2 arc that unblocks the mantle PROCESSING-TIME derive-down (`MeltParams::processing_time_myr`): both
/// wait on the SI-valued Stokes velocity, so the two are one unit-wiring dependency, not two independent knobs.
/// `ra_crit` and `ra_crit_wavenumber` are the classical Rayleigh-Benard critical PAIR (the marginal-stability
/// eigenvalue for rigid boundaries, {~1708, a_c ~ 3.117}, from [`RIGID_RIGID_RA_CRIT`] and
/// [`RIGID_RIGID_CRITICAL_WAVENUMBER`]), so the onset threshold and the lateral cell scale share one regime; as
/// the units plan lifts the operating point to SI a hot radiogenic province crosses `ra_crit` and convects
/// while a cold one conducts, the bifurcation that amplifies the seed. Deterministic (a pure function of its
/// inputs).
pub fn province_column_params(
    convecting_depth_mm: Fixed,
    surface_gravity_m_s2: Fixed,
    heat_production: Fixed,
    reference_temperature_k: Fixed,
    dt: Fixed,
) -> ColumnParams {
    ColumnParams {
        reference_temperature: reference_temperature_k,
        density: Fixed::ONE,
        thermal_conductivity: Fixed::from_int(2),
        thermal_expansion_ppm: Fixed::from_int(30),
        gravity: surface_gravity_m_s2,
        depth: convecting_depth_mm,
        radius: Fixed::ONE,
        viscosity: Fixed::ONE,
        thermal_diffusivity: Fixed::from_ratio(1, 100),
        specific_heat: Fixed::from_int(10),
        heat_production,
        ra_crit: RIGID_RIGID_RA_CRIT,
        ra_crit_wavenumber: RIGID_RIGID_CRITICAL_WAVENUMBER,
        ra_max: Fixed::from_int(1_000_000),
        v_max: Fixed::from_int(1_000_000),
        flux_max: Fixed::from_int(1_000_000),
        stress_max: Fixed::from_int(1_000_000),
        dt,
    }
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
            ra_crit_wavenumber: RIGID_RIGID_CRITICAL_WAVENUMBER,
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

    // The star-aging parameters. The two constants are ILLUSTRATIVE stand-ins for the owner's reserved values,
    // chosen at the physically-anchored figures only to exercise the mechanism: `t_sun` = 10 Gyr (10,000 Myr, the
    // solar nuclear timescale) and `c` = 0.4 (the Gough 1981 coefficient). The ZAMS luminosity and mass ratios are
    // the per-star anchors the caller feeds from the stellar front-end.
    fn aging_params(zams_luminosity_ratio: Fixed, mass_ratio: Fixed) -> StarAgingParams {
        StarAgingParams {
            zams_luminosity_ratio,
            mass_ratio,
            solar_nuclear_timescale_myr: Fixed::from_int(10_000),
            brightening_coefficient: Fixed::from_ratio(4, 10),
        }
    }

    #[test]
    fn the_star_brightens_monotonically_over_its_main_sequence_life() {
        // The core burns hydrogen, its mean molecular weight rises, and the luminosity climbs: at every later age
        // the star is brighter than before, the surface-warming drive of the deep-time run.
        let star = aging_params(Fixed::ONE, Fixed::ONE); // Sun-like: t_ms = 10,000 Myr
        let mut prev = current_luminosity_ratio(&star, Fixed::ZERO).expect("on the main sequence");
        for age_myr in [500, 1000, 2000, 4000, 6000, 8000, 9000, 9500, 9900] {
            let l = current_luminosity_ratio(&star, Fixed::from_int(age_myr))
                .expect("still on the main sequence before t_ms");
            assert!(
                l > prev,
                "the star brightens as it ages: at {age_myr} Myr L/L_sun = {} was not above the prior {}",
                l.to_f64_lossy(),
                prev.to_f64_lossy()
            );
            prev = l;
        }
    }

    #[test]
    fn the_zams_anchor_is_recovered_at_age_zero() {
        // At age 0 the Gough factor is exactly one, so the current luminosity is the stellar front-end's ZAMS
        // output unchanged: the brightening climbs FROM the anchor, it does not replace it.
        let star = aging_params(Fixed::from_ratio(7, 4), Fixed::from_ratio(6, 5));
        assert_eq!(
            current_luminosity_ratio(&star, Fixed::ZERO),
            Some(Fixed::from_ratio(7, 4)),
            "at age 0 the ZAMS anchor is recovered exactly"
        );
    }

    #[test]
    fn a_more_massive_star_leaves_the_main_sequence_faster() {
        // t_ms ~ M / L and L climbs steeply with M (the mass-luminosity relation the stellar front-end derives), so
        // a heavier star holds more fuel but burns it far faster and leaves the main sequence sooner. The ZAMS
        // luminosities come from the real stellar model to show the derivation composes end to end.
        use crate::stellar::luminosity_ratio;
        let alpha = Fixed::from_ratio(35, 10); // illustrative mass-luminosity slope (~3.5), as the stellar tests use
        let lambda = Fixed::ZERO; // solar metallicity: the metallicity factor is one
        let sun_l = luminosity_ratio(Fixed::ONE, Fixed::ONE, alpha, lambda).expect("sun L");
        let heavy_l =
            luminosity_ratio(Fixed::from_int(2), Fixed::ONE, alpha, lambda).expect("heavy L");
        let sun = aging_params(sun_l, Fixed::ONE);
        let heavy = aging_params(heavy_l, Fixed::from_int(2));
        let sun_life = main_sequence_lifetime_myr(&sun).expect("sun lifetime");
        let heavy_life = main_sequence_lifetime_myr(&heavy).expect("heavy lifetime");
        assert!(
            heavy_life < sun_life,
            "a heavier star burns its fuel faster and leaves the main sequence sooner, got heavy {} vs sun {}",
            heavy_life.to_f64_lossy(),
            sun_life.to_f64_lossy()
        );
        // And the heavier star, having a shorter t_ms, brightens FASTER: at the same age it is a larger fraction of
        // the way through its life, so a larger fraction of the way up the brightening curve.
        let age = Fixed::from_int(1000);
        let sun_f = current_luminosity_ratio(&sun, age)
            .expect("sun still on MS")
            .checked_div(sun_l)
            .expect("sun brightening factor");
        let heavy_f = current_luminosity_ratio(&heavy, age)
            .expect("heavy still on MS")
            .checked_div(heavy_l)
            .expect("heavy brightening factor");
        assert!(
            heavy_f > sun_f,
            "the shorter-lived heavier star brightens faster per unit time, got {} vs {}",
            heavy_f.to_f64_lossy(),
            sun_f.to_f64_lossy()
        );
    }

    #[test]
    fn the_star_leaves_the_main_sequence_at_its_lifetime() {
        // Just before t_ms the star is still on the main sequence; at and past t_ms it has exhausted its core
        // hydrogen and LEAVES, so the brightening caps to None (the post-main-sequence branch is a separate arc).
        let star = aging_params(Fixed::ONE, Fixed::ONE); // t_ms = 10,000 Myr
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(9_999)).is_some(),
            "just before t_ms the star is still on the main sequence"
        );
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(10_000)).is_none(),
            "at t_ms the star leaves the main sequence"
        );
        assert!(
            current_luminosity_ratio(&star, Fixed::from_int(12_000)).is_none(),
            "past t_ms is the post-main-sequence regime, not extrapolated here"
        );
    }

    #[test]
    fn the_brightening_is_deterministic() {
        // A pure derivation replays bit-for-bit, the determinism the canon requires.
        let star = aging_params(Fixed::from_ratio(3, 2), Fixed::from_ratio(5, 4));
        let a = current_luminosity_ratio(&star, Fixed::from_int(3_000));
        let b = current_luminosity_ratio(&star, Fixed::from_int(3_000));
        assert_eq!(a, b, "the brightening replays deterministically");
    }

    #[test]
    fn the_star_ages_on_the_deep_time_clock() {
        // The star and the planet share one clock: as the run steps, the star's age advances through elapsed_myr
        // and its luminosity climbs, the drive that warms the surface over the run.
        let star = aging_params(Fixed::ONE, Fixed::ONE);
        let params = [mantle_params(5)];
        let mut state = DeepTimeState::young(2, Fixed::from_int(2000));
        let l0 = state
            .stellar_luminosity_ratio(&star)
            .expect("on the main sequence at the start");
        assert_eq!(
            l0,
            Fixed::ONE,
            "a fresh run begins at the star's ZAMS anchor"
        );
        for _ in 0..20 {
            state = step_deep_time(&state, &params, &melt_params(), Fixed::from_int(100))
                .expect("steps");
        }
        assert_eq!(
            state.star_age_myr(),
            Fixed::from_int(2000),
            "the star's age tracks the deep-time clock (20 ticks of 100 Myr)"
        );
        let l1 = state
            .stellar_luminosity_ratio(&star)
            .expect("still on the main sequence");
        assert!(
            l1 > l0,
            "the star brightens as the deep-time run advances, warming the surface, got {} vs {}",
            l1.to_f64_lossy(),
            l0.to_f64_lossy()
        );
    }

    #[test]
    fn a_run_can_begin_with_an_already_aged_star() {
        // The per-run start age: a run beginning mid-life carries the star's prior age, so at the same elapsed time
        // its star is further up the brightening curve than a run around a zero-age star.
        let star = aging_params(Fixed::ONE, Fixed::ONE);
        let fresh = DeepTimeState::young(1, Fixed::from_int(1800));
        let aged = fresh.clone().with_star_start_age(Fixed::from_int(5000));
        assert_eq!(
            aged.star_age_myr(),
            Fixed::from_int(5000),
            "the run begins with the star already aged"
        );
        assert!(
            aged.stellar_luminosity_ratio(&star).expect("aged, on MS")
                > fresh.stellar_luminosity_ratio(&star).expect("fresh, on MS"),
            "the already-aged star is brighter at the same elapsed time"
        );
    }

    #[test]
    fn a_massless_or_dark_star_has_no_defined_lifetime() {
        // Fail-loud guards: a non-positive mass (no fuel) or a non-positive ZAMS luminosity (no burn rate) has no
        // defined main-sequence lifetime, routed to None rather than a fabricated value.
        assert_eq!(
            main_sequence_lifetime_myr(&aging_params(Fixed::ONE, Fixed::ZERO)),
            None,
            "no mass, no fuel, no lifetime"
        );
        assert_eq!(
            main_sequence_lifetime_myr(&aging_params(Fixed::ZERO, Fixed::ONE)),
            None,
            "no luminosity, no burn rate, no lifetime"
        );
        assert_eq!(
            current_luminosity_ratio(&aging_params(Fixed::ONE, Fixed::ONE), Fixed::from_int(-100)),
            None,
            "a negative age is not a main-sequence age"
        );
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

    #[test]
    fn the_province_count_derives_from_the_convective_scale() {
        // The lateral province scale is DERIVED, not a hand-set grid: the count spanning a distance is the
        // distance over the convective cell width (mantle depth times the aspect ratio). A deeper mantle
        // makes fewer, wider provinces; a larger span more of them.
        let depth = Fixed::from_int(1_500_000); // ~1500 km convecting mantle
        let aspect = Fixed::from_ratio(141, 100); // ~sqrt(2), the free-free critical-mode aspect
        let circumference = Fixed::from_int(21_000_000); // ~2*pi*3340 km
        let n = provinces_across(circumference, depth, aspect).expect("derives");
        assert!(
            (7..=12).contains(&n),
            "the province count is circumference / (depth * aspect), got {n}"
        );
        // A deeper mantle (wider cells) yields strictly fewer provinces across the same span.
        let deeper = provinces_across(circumference, Fixed::from_int(3_000_000), aspect).unwrap();
        assert!(
            deeper < n,
            "a deeper mantle makes fewer, wider provinces, got {deeper} vs {n}"
        );
        // Degenerate inputs fail loud rather than fabricating a scale.
        assert!(provinces_across(Fixed::ZERO, depth, aspect).is_none());
        assert!(provinces_across(circumference, Fixed::ZERO, aspect).is_none());
        assert!(provinces_across(circumference, depth, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_province_column_wires_the_derived_depth_gravity_and_heat() {
        // The derived per-planet inputs thread into the column params (the depth retires the depth = 1
        // fixture), and a hotter radiogenic budget is carried through so the provinces can diverge.
        let depth_mm = Fixed::from_ratio(15, 10); // 1.5 Mm derived convecting depth
        let g = Fixed::from_ratio(37, 10); // Mars-class surface gravity
        let cool = province_column_params(
            depth_mm,
            g,
            Fixed::from_int(5),
            Fixed::from_int(300),
            Fixed::ONE,
        );
        let hot = province_column_params(
            depth_mm,
            g,
            Fixed::from_int(400),
            Fixed::from_int(300),
            Fixed::ONE,
        );
        assert_eq!(
            cool.depth, depth_mm,
            "the derived depth is wired in, not a fixture"
        );
        assert_eq!(cool.gravity, g, "the derived surface gravity is wired in");
        assert!(
            hot.heat_production > cool.heat_production,
            "the per-province radiogenic budget varies"
        );
        // The two share the same operating point apart from the varied heat, so a step diverges them.
        let cool_state = convection_step(
            &ColumnState {
                temperature: Fixed::from_int(1588),
                convecting: false,
            },
            &cool,
        );
        let hot_state = convection_step(
            &ColumnState {
                temperature: Fixed::from_int(1588),
                convecting: false,
            },
            &hot,
        );
        assert!(
            hot_state.temperature > cool_state.temperature,
            "the more-radiogenic province stays hotter after a step, got {} vs {}",
            hot_state.temperature.to_f64_lossy(),
            cool_state.temperature.to_f64_lossy()
        );
    }
}
