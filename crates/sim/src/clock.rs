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

//! The simulation time model and the observer's playback control (design Part 14.6, Part 32,
//! Part 54, Principles 3 and 10).
//!
//! The engine has one notion of canonical time: a base tick, the finest canonical timestep.
//! Everything canonical advances by whole ticks, and every random draw folds the tick as a
//! coordinate ([`civsim_core::DrawKey`]), so the world state at tick N is a pure function of the
//! seed and N. That is the property this module leans on: because a step never carries wall-clock
//! time into state, an observer may run the simulation at any speed, pause it, or single-step it,
//! and the canonical timeline is unchanged (Principle 10: "fast-forwarding does not change what
//! happens"). The observer's timescale is a playback speed over the timeline, never a change to it.
//!
//! Two pieces live here. [`Steppable`] is the shape a driver exposes so the observer can advance
//! it one canonical tick at a time (the pre-dawn radiation and the dawn tick are both steppable).
//! [`PlaybackDriver`] is the view-side accumulator that turns real elapsed seconds and a chosen
//! speed into a whole number of ticks to run. It is deliberately non-canonical: it holds a
//! floating-point accumulator (permitted view-side by Part 14.6), it reads no clock of its own
//! (the caller feeds it the real delta, so it stays a pure function of its inputs and is
//! testable), and it only ever asks a [`Steppable`] to run an integer number of whole ticks, so
//! it cannot perturb determinism.
//!
//! The honest limit the design names (Part 32) is surfaced rather than hidden: when the chosen
//! speed asks for more ticks in one frame than the per-frame budget allows, the surplus is
//! recorded as [`PlaybackDriver::lod_debt`] rather than silently run or silently dropped. That
//! debt is the exact signal that fine per-tick simulation cannot keep pace and that coarse
//! statistical stepping (temporal level of detail, Part 32) would have to take over. Cheap
//! fast-forward through busy, canonically-active time remains the open problem the design keeps
//! in the research tier; this module makes the boundary visible instead of pretending it away.

use civsim_core::Fixed;
use civsim_world::{OrbitalElements, PlanetaryBody};

use crate::calibration::{CalibrationError, CalibrationManifest};

/// A steppable simulation: something the observer can advance one canonical tick at a time. A
/// step is one base tick (for the dawn) or one generation (for the pre-dawn radiation); the
/// contract is that a step is always a whole canonical unit, never a fraction, so replay is
/// bit-identical regardless of how the observer paced the stepping.
pub trait Steppable {
    /// Advance exactly one canonical tick. Deterministic and self-contained: it reads no
    /// wall-clock and takes no view-time input, so the same starting state always yields the
    /// same next state (Principle 3).
    fn step(&mut self);

    /// The current canonical tick (or generation) count, for the observer's time readout.
    fn now(&self) -> u64;
}

/// A labelled DEVELOPMENT and TEST fallback for the life-cadence period in base ticks: how often
/// aging and the mortality roll beat, the unit the age-hazard curve is scaled to (design Part 20,
/// R-AGING). This is Earth's year at the one-second base tick (365 days of 86,400 seconds), one
/// option among many, NOT the canonical per-world value: on the canonical path the life cadence
/// derives from the world's own orbit, [`ticks_from_seconds`] over the orbital year and the base
/// tick (see [`crate::world::World::from_manifest_with_orbital`]), so a fast world and a slow
/// world beat aging on their own years. This constant is what [`crate::world::World::new`] falls
/// back to when no manifest and orbit are supplied, so tests and tools have a concrete cadence
/// before the owner sets the two per-world orbital scalars.
pub const LIFE_CADENCE_TICKS: u64 = 31_536_000;

/// Derive a whole-tick cadence from a span of world-time, the canonical bridge from seconds to
/// ticks (design Part 14.6, Parts 20 and 54). It divides a span in world-seconds by the base-tick
/// duration and floors to whole ticks, entirely in fixed-point so no float enters the result
/// (Principle 3). This is the derivation the life-cadence beat runs through, so a world's year in
/// ticks is a function of its orbit and its base tick rather than a hardcoded constant. It fails
/// loud rather than panicking, wrapping, or returning a zero cadence: the base tick and the span
/// must both be positive, the quotient must stay in fixed-point range, and the floored result
/// must be at least one tick (a span shorter than one base tick would beat every tick).
// @derives[clock_calendar_cell]: a world's year/day/season in TICKS, and the cell area in metres <- the world's orbit (world-seconds) divided by the base tick (1 tick = 1 world-second, reserved). The calendar is NOT a hardcoded 365 days; it falls out of the orbit and the tick. The cell edge derives as a reference creature's real ground speed (m/s) x 1 s/tick (see locomotion base_speed), cross-checked by NPP density x cell area = standing crop.
// @derives[world_time_cadence]: the DAY cadence in TICKS (the rotation-derived beat: aging, drift, the diurnal calendar) <- the ROTATION period floored over the base tick, this same kernel; repointed here off the celestial.rs passthrough (OrbitalElements is a manifest read, not a derivation) and differentiated from clock_calendar_cell (which floors the ORBITAL period, the year), so the two temporal cadences are distinct derivations, day versus year (gate ruling, #168). The rotation cadence is real on the run path: DiurnalSky::rotation_period_ticks drives the diurnal cycle.
pub fn ticks_from_seconds(
    seconds: Fixed,
    base_tick_seconds: Fixed,
) -> Result<u64, CalibrationError> {
    if base_tick_seconds <= Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "time.base_tick_seconds".to_string(),
            detail: "the base-tick duration must be positive to derive a tick cadence".to_string(),
        });
    }
    if seconds <= Fixed::ZERO {
        return Err(CalibrationError::BadValue {
            id: "time.derived_cadence".to_string(),
            detail: "the world-time span must be positive to derive a tick cadence".to_string(),
        });
    }
    let quotient =
        seconds
            .checked_div(base_tick_seconds)
            .ok_or_else(|| CalibrationError::BadValue {
                id: "time.derived_cadence".to_string(),
                detail: "the tick cadence overflows fixed-point range for this span and base tick"
                    .to_string(),
            })?;
    // The quotient is strictly positive here (both inputs are positive), so the arithmetic
    // shift right by the fractional bits is an exact floor toward zero and the cast to u64
    // cannot wrap into a huge value.
    let ticks = (quotient.to_bits() >> Fixed::FRAC_BITS) as u64;
    if ticks == 0 {
        return Err(CalibrationError::BadValue {
            id: "time.derived_cadence".to_string(),
            detail: "the span is shorter than one base tick, which would beat every tick"
                .to_string(),
        });
    }
    Ok(ticks)
}

/// The base-tick duration as a fixed-point value, read live from the manifest
/// (`time.base_tick_seconds`). This is the canonical [`Fixed`] source the tick-cadence
/// derivation ([`ticks_from_seconds`]) uses, distinct from the non-canonical f64
/// [`SimClock::world_seconds_per_tick`], which stays a view-side display value and must never
/// carry the canonical division. Fails loud while the value is reserved.
pub fn base_tick_seconds_fixed(m: &CalibrationManifest) -> Result<Fixed, CalibrationError> {
    m.require_fixed("time.base_tick_seconds")
}

/// Read a world's orbital elements from the two reserved owner scalars
/// (`world.orbital_period_seconds`, `world.rotation_period_seconds`), failing loud while either
/// is reserved (never fabricating a value). The two periods are owner-set per world: Earth's
/// values are one option among many (see [`OrbitalElements::dev_earth`]), never a silent default,
/// so a reserved manifest correctly refuses to hand back a fabricated orbit rather than defaulting
/// to Earth. (A free function rather than an inherent `OrbitalElements::from_manifest` because the
/// type lives in `civsim-world`, which cannot depend on the manifest in `civsim-sim`.)
pub fn orbital_from_manifest(m: &CalibrationManifest) -> Result<OrbitalElements, CalibrationError> {
    Ok(OrbitalElements {
        orbital_period_seconds: m.require_fixed("world.orbital_period_seconds")?,
        rotation_period_seconds: m.require_fixed("world.rotation_period_seconds")?,
    })
}

/// Read a world's planetary body from the two reserved owner scalars (`world.planet_radius` in metres,
/// `world.mean_density` the whole-planet mean in kg/m^3), failing loud while either is reserved (never
/// fabricating a value). The two data are the per-world geometry the surface gravity derives from
/// (`g = (4/3) * pi * G * R * rhobar`); Earth's values are one option among many (see
/// [`PlanetaryBody::dev_earth`]), never a silent default, so a reserved manifest correctly refuses to hand
/// back a fabricated body rather than defaulting to Earth. A free function rather than an inherent
/// `PlanetaryBody::from_manifest` because the type lives in `civsim-world`, which cannot depend on the
/// manifest in `civsim-sim` (the same split [`orbital_from_manifest`] uses).
pub fn planetary_from_manifest(m: &CalibrationManifest) -> Result<PlanetaryBody, CalibrationError> {
    Ok(PlanetaryBody {
        radius_meters: m.require_fixed("world.planet_radius")?,
        mean_density: m.require_fixed("world.mean_density")?,
    })
}

/// The diurnal-cycle tick periods `(rotation_period_ticks, orbital_period_ticks)` for a chosen
/// day-sampling resolution, DERIVED from the world's own orbit rather than authored as tick counts.
/// `ticks_per_day` is the visibility SAMPLING RATE (how many ticks render one rotation), an observer
/// or demo choice about time resolution and NOT a world datum, so a world with any rotation renders a
/// day in the same number of ticks. Both returned periods derive from the world's rotation and
/// orbital periods in world-seconds through the canonical seconds-to-ticks bridge
/// ([`ticks_from_seconds`]) at the sampling base-tick that resolution implies (`rotation_period /
/// ticks_per_day` world-seconds per tick), so the year in ticks is the world's own days-per-year
/// (`orbital_period / rotation_period`) times the sampling rate, never a hardcoded 365. A fast world
/// and a slow world render their day-night and seasons on their own orbits (admit-the-alien: the
/// orbit is the data, the sampling rate is the observer's dial). Fails loud rather than fabricating:
/// the sampling rate must be at least one tick and the derived base-tick and periods must be positive
/// and in fixed-point range.
pub fn diurnal_periods_at_sampling(
    orbital: &OrbitalElements,
    ticks_per_day: u64,
) -> Result<(u64, u64), CalibrationError> {
    if ticks_per_day == 0 {
        return Err(CalibrationError::BadValue {
            id: "time.diurnal_sampling".to_string(),
            detail: "the day-sampling resolution must be at least one tick".to_string(),
        });
    }
    let ticks_per_day_fixed = i32::try_from(ticks_per_day)
        .map(Fixed::from_int)
        .map_err(|_| CalibrationError::BadValue {
            id: "time.diurnal_sampling".to_string(),
            detail: "the day-sampling resolution is too large to represent in fixed-point"
                .to_string(),
        })?;
    // The sampling base-tick renders one rotation in `ticks_per_day` ticks: base = rotation / n
    // world-seconds per tick. Both the day and the year then run through the same canonical bridge,
    // so the day comes back to `ticks_per_day` and the year is that times the world's days-per-year.
    let sampling_base_tick = orbital
        .rotation_period_seconds
        .checked_div(ticks_per_day_fixed)
        .ok_or_else(|| CalibrationError::BadValue {
            id: "time.diurnal_sampling".to_string(),
            detail: "the sampling base-tick overflows fixed-point range for this rotation period"
                .to_string(),
        })?;
    let rotation_period_ticks =
        ticks_from_seconds(orbital.rotation_period_seconds, sampling_base_tick)?;
    let orbital_period_ticks =
        ticks_from_seconds(orbital.orbital_period_seconds, sampling_base_tick)?;
    Ok((rotation_period_ticks, orbital_period_ticks))
}

/// In-world years one pre-dawn radiation generation represents, for the deep-time readout only,
/// never canonical state. Owner-set (2026-07-01, `time.years_per_generation`) to 10_000 years, a
/// deep speciation timescale, so the default forty-generation radiation spans about 400_000 years.
pub const YEARS_PER_GENERATION: u64 = 10_000;

/// The base-tick duration: how much world-time one canonical tick represents. This is a reserved
/// owner calibration value (design Part 54; `time.base_tick_seconds` in the manifest), owner-set
/// (2026-07-01) to one in-world second per tick. The mechanism above does not depend on its value,
/// since advancing by whole ticks is deterministic whatever a tick means in world-time; it is used
/// only to label the observer's speed and time readout. One second per tick matches the design's
/// definition of the base tick as short enough for smooth movement (line 3058) and R-VIEW-ELAB's
/// near-one-second view target, and makes playback speed 1.0 play one in-world second per real
/// second.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SimClock {
    /// World-seconds represented by one base tick. Owner-set to 1.0 (`time.base_tick_seconds`).
    pub world_seconds_per_tick: f64,
}

impl SimClock {
    /// The clock at the owner-set base-tick duration, one in-world second per tick.
    pub fn dev_default() -> SimClock {
        SimClock {
            world_seconds_per_tick: 1.0,
        }
    }

    /// The world-time, in seconds, that `ticks` base ticks represent.
    pub fn world_seconds(&self, ticks: u64) -> f64 {
        ticks as f64 * self.world_seconds_per_tick
    }

    /// A compact humanised world-time for a tick count (years, days, hours, minutes, seconds),
    /// for the observer's readout. Presentation only.
    pub fn format_world_time(&self, ticks: u64) -> String {
        let mut secs = self.world_seconds(ticks);
        if !secs.is_finite() || secs < 0.0 {
            return "0s".to_string();
        }
        const YEAR: f64 = 365.0 * 24.0 * 3600.0;
        const DAY: f64 = 24.0 * 3600.0;
        const HOUR: f64 = 3600.0;
        const MIN: f64 = 60.0;
        let years = (secs / YEAR).floor();
        secs -= years * YEAR;
        let days = (secs / DAY).floor();
        secs -= days * DAY;
        let hours = (secs / HOUR).floor();
        secs -= hours * HOUR;
        let mins = (secs / MIN).floor();
        secs -= mins * MIN;
        if years > 0.0 {
            format!("{years:.0}y {days:.0}d")
        } else if days > 0.0 {
            format!("{days:.0}d {hours:.0}h")
        } else if hours > 0.0 {
            format!("{hours:.0}h {mins:.0}m")
        } else if mins > 0.0 {
            format!("{mins:.0}m {secs:.0}s")
        } else {
            format!("{secs:.0}s")
        }
    }
}

/// The lowest and highest playback speeds the driver clamps to (ticks per real second at the
/// current step granularity). These bound the observer's control, not the simulation: a floor
/// above zero (pause is a separate flag) and a ceiling that keeps the accumulator arithmetic
/// well away from overflow. They are view-side UI bounds, not canonical values.
const MIN_RATE: f64 = 1.0 / 64.0;
const MAX_RATE: f64 = 1.0e12;

/// The view-side playback accumulator (design Part 14.6). It converts real elapsed seconds and a
/// chosen speed into a whole number of canonical ticks to run this frame, banking the fractional
/// remainder so no time is lost or invented across frames. It is a pure function of its inputs:
/// it reads no clock (the caller passes the real delta), and it only ever yields an integer tick
/// count, so it cannot perturb determinism (Principle 10). It never advances a [`Steppable`]
/// itself; it tells the caller how many whole steps to run.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlaybackDriver {
    /// The chosen speed: canonical ticks to advance per real second. With the base-tick anchor
    /// of one world-second per tick, a rate of 1.0 plays one in-world second per real second;
    /// larger rates fast-forward (days, then years, per real second).
    rate: f64,
    /// Real seconds banked toward the next whole tick (the fixed-timestep accumulator).
    accumulator: f64,
    /// Whether playback is paused. A paused driver banks no real time and runs no ticks except
    /// forced single steps.
    paused: bool,
    /// Whole ticks requested out of band (single-step), always run before the rate-driven ticks
    /// and independent of pause.
    pending: u32,
    /// The most ticks this driver will hand back in one `advance`, so a long stall or a very high
    /// speed cannot spiral into an unbounded catch-up that freezes the caller.
    max_ticks_per_advance: u32,
    /// Ticks the chosen speed asked for beyond the per-frame budget, accumulated. This is the
    /// honest temporal-LOD signal (Part 32): a non-zero debt means fine stepping cannot keep pace
    /// at this speed and coarse stepping would be needed to fast-forward cheaply.
    ///
    /// CONTENTION-LOD-INVARIANCE FLAG (owner-flagged, check this BEFORE building coarse stepping
    /// here): when coarse temporal-LOD stepping is built, resource-contention resolution MUST stay
    /// LOD-invariant (Principle 10: the canonical outcome may not depend on whether a region is
    /// fine- or coarse-stepped). The FIELD-DRAW mode is safe unconditionally, since the proportional
    /// split equals the depletion integral over the interval, so a coarse step and a fine step agree
    /// exactly. The FIRST-COME mode (discrete grabbable items resolved by physical arrival order) is
    /// the OPEN one: it stays invariant only if the arrival-time ordering is computed the same at
    /// both resolutions, and arrival time is a function of each agent's trajectory over the interval,
    /// which a coarse step only approximates. So before trusting first-come across the LOD boundary,
    /// verify (an advisor consult against the concrete solver) that the arrival-time and trajectory
    /// computation is itself LOD-invariant. Grounding: the contention-resolver design (the
    /// apportionment-contention record; field-draw proportional split versus first-come-by-arrival,
    /// mode derived from the resource's physical type).
    lod_debt: u64,
}

impl PlaybackDriver {
    /// A driver at a chosen starting rate (ticks per real second), running (not paused), with a
    /// sensible per-frame catch-up cap.
    pub fn new(rate: f64) -> PlaybackDriver {
        PlaybackDriver {
            rate: rate.clamp(MIN_RATE, MAX_RATE),
            accumulator: 0.0,
            paused: false,
            pending: 0,
            max_ticks_per_advance: 4096,
            lod_debt: 0,
        }
    }

    /// Set the per-frame catch-up cap (the most whole ticks one `advance` will return).
    pub fn with_max_ticks_per_advance(mut self, cap: u32) -> PlaybackDriver {
        self.max_ticks_per_advance = cap.max(1);
        self
    }

    /// The number of whole ticks to run this frame, given the real seconds elapsed since the last
    /// call. Forced single steps run first (up to the cap), then, if not paused, the rate-driven
    /// ticks from the accumulator. Any rate-driven surplus beyond the cap is banked as `lod_debt`
    /// rather than run or dropped silently. Pure: same inputs, same result.
    pub fn advance(&mut self, real_dt_seconds: f64) -> u32 {
        let cap = self.max_ticks_per_advance;
        // Forced single steps first, bounded by the cap.
        let forced = self.pending.min(cap);
        self.pending -= forced;
        let mut ticks = forced;

        if !self.paused && real_dt_seconds.is_finite() && real_dt_seconds > 0.0 {
            self.accumulator += real_dt_seconds * self.rate;
            let whole_f = self.accumulator.floor();
            let whole = if whole_f.is_finite() && whole_f >= 0.0 {
                whole_f as u64
            } else {
                0
            };
            self.accumulator -= whole as f64;
            let room = (cap - ticks) as u64;
            let run = whole.min(room);
            ticks += run as u32;
            if whole > room {
                self.lod_debt += whole - room;
            }
        }
        ticks
    }

    /// Run each of `steps` steps through a [`Steppable`], as a convenience for the caller. Returns
    /// how many steps were run (the same value `advance` returned).
    pub fn drive<S: Steppable>(&mut self, sim: &mut S, real_dt_seconds: f64) -> u32 {
        let steps = self.advance(real_dt_seconds);
        for _ in 0..steps {
            sim.step();
        }
        steps
    }

    /// Queue `n` forced single steps, run on the next `advance` whether or not the driver is
    /// paused. Single-stepping is the observer's frame-by-frame control.
    pub fn request_steps(&mut self, n: u32) {
        self.pending = self.pending.saturating_add(n);
    }

    /// Pause playback (forced single steps still run; the accumulator banks no real time).
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume playback.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggle the pause flag, returning the new state.
    pub fn toggle_pause(&mut self) -> bool {
        self.paused = !self.paused;
        self.paused
    }

    /// Whether playback is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// The current playback rate (ticks per real second).
    pub fn rate(&self) -> f64 {
        self.rate
    }

    /// Set the playback rate directly, clamped to the view-side bounds.
    pub fn set_rate(&mut self, rate: f64) {
        self.rate = rate.clamp(MIN_RATE, MAX_RATE);
    }

    /// Multiply the rate by `factor`, clamped. Speeding up (factor > 1) and slowing down
    /// (factor < 1) are the observer's coarse speed controls.
    pub fn scale_rate(&mut self, factor: f64) {
        if factor.is_finite() && factor > 0.0 {
            self.set_rate(self.rate * factor);
        }
    }

    /// The accumulated temporal-LOD debt: ticks the chosen speed asked for beyond the per-frame
    /// budget. Non-zero means fine stepping cannot keep pace at this speed (Part 32).
    pub fn lod_debt(&self) -> u64 {
        self.lod_debt
    }

    /// Clear the accumulated LOD debt (for a fresh per-frame or per-readout window).
    pub fn clear_lod_debt(&mut self) {
        self.lod_debt = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trivial steppable counter, to exercise the driver's stepping without a real simulation.
    struct Counter(u64);
    impl Steppable for Counter {
        fn step(&mut self) {
            self.0 += 1;
        }
        fn now(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn rate_one_plays_one_tick_per_real_second() {
        let mut d = PlaybackDriver::new(1.0);
        // One real second at rate 1 yields exactly one tick.
        assert_eq!(d.advance(1.0), 1);
        // Half a second banks, the next half completes the second tick.
        assert_eq!(d.advance(0.5), 0);
        assert_eq!(d.advance(0.5), 1);
    }

    #[test]
    fn faster_rate_runs_more_ticks() {
        let mut d = PlaybackDriver::new(60.0);
        // A minute per second: one real second is sixty ticks.
        assert_eq!(d.advance(1.0), 60);
    }

    #[test]
    fn chunked_time_conserves_total_ticks_within_float_tolerance() {
        // The fixed-timestep property: the total ticks over many small frames matches the total
        // over one big frame, because the accumulator carries the fractional remainder rather than
        // discarding it. This is what makes the observed count essentially independent of frame
        // pacing, the heart of the feature. The accumulator is floating point (a view-side value
        // permitted by Part 14.6), so chunked and single-shot may differ by at most one tick of
        // rounding, never more, and neither invents or loses ticks in bulk. Determinism is
        // untouched regardless: the simulation only ever advances by whole ticks either way.
        let rate = 37.0;
        let total_time = 10.0;
        let mut chunked = PlaybackDriver::new(rate);
        let mut sum = 0i64;
        for _ in 0..1000u32 {
            sum += chunked.advance(total_time / 1000.0) as i64;
        }
        let mut single = PlaybackDriver::new(rate);
        let one = single.advance(total_time) as i64;
        assert!(
            (sum - one).abs() <= 1,
            "chunked ({sum}) and single-shot ({one}) agree within one tick of float rounding"
        );
        let expected = (rate * total_time) as i64;
        assert!(
            (sum - expected).abs() <= 1,
            "chunked total tracks rate*time"
        );
        assert!(
            (one - expected).abs() <= 1,
            "single-shot total tracks rate*time"
        );
    }

    #[test]
    fn pause_runs_no_ticks_but_single_step_still_advances() {
        let mut d = PlaybackDriver::new(10.0);
        d.pause();
        assert_eq!(d.advance(5.0), 0, "paused banks no time");
        d.request_steps(1);
        assert_eq!(d.advance(5.0), 1, "a forced single step runs while paused");
        assert_eq!(d.advance(5.0), 0, "and only once");
    }

    #[test]
    fn resume_after_pause_does_not_replay_banked_time() {
        let mut d = PlaybackDriver::new(10.0);
        d.pause();
        let _ = d.advance(100.0); // a long pause
        d.resume();
        // Resuming does not dump the paused seconds; only the real delta after resume counts.
        assert_eq!(d.advance(0.1), 1);
    }

    #[test]
    fn the_catch_up_cap_bounds_a_frame_and_records_lod_debt() {
        let mut d = PlaybackDriver::new(1.0e9).with_max_ticks_per_advance(1000);
        // A billion ticks per second asked for in one second: only the cap runs this frame, the
        // rest is surfaced as LOD debt rather than run or dropped.
        let ran = d.advance(1.0);
        assert_eq!(ran, 1000, "the cap bounds the frame");
        assert!(
            d.lod_debt() > 0,
            "the surplus is recorded as temporal-LOD debt"
        );
        assert_eq!(d.lod_debt(), 1_000_000_000 - 1000);
    }

    #[test]
    fn drive_steps_a_steppable_the_reported_number_of_times() {
        let mut d = PlaybackDriver::new(5.0);
        let mut c = Counter(0);
        let n = d.drive(&mut c, 1.0);
        assert_eq!(n, 5);
        assert_eq!(c.now(), 5, "the steppable advanced exactly n steps");
    }

    #[test]
    fn rate_is_clamped_to_the_view_bounds() {
        let mut d = PlaybackDriver::new(0.0);
        assert!(
            d.rate() >= MIN_RATE,
            "zero is clamped up to the floor, not left at zero"
        );
        d.set_rate(1.0e30);
        assert!(
            d.rate() <= MAX_RATE,
            "an enormous rate is clamped to the ceiling"
        );
        d.scale_rate(0.0); // ignored (non-positive)
        assert!(d.rate() <= MAX_RATE);
    }

    #[test]
    fn the_life_cadence_derives_from_the_orbit_and_base_tick() {
        // The life cadence is a derivation, not a hardcoded constant: a world's year in
        // world-seconds divided by the base-tick duration, floored to whole ticks. For the
        // labelled Earth fixture at the owner-set one-second base tick this reproduces today's
        // interim of 31,536,000 ticks (one Earth year of seconds), the same value the dev fallback
        // LIFE_CADENCE_TICKS carries, so the derivation and the fallback agree on Earth. The point
        // is that the value now comes out of the orbit rather than being written down.
        let earth = OrbitalElements::dev_earth();
        let base_tick = Fixed::from_int(1); // time.base_tick_seconds, one world-second per tick
        let cadence = ticks_from_seconds(earth.orbital_period_seconds, base_tick).unwrap();
        assert_eq!(
            cadence, 31_536_000,
            "Earth's year of seconds at a one-second tick"
        );
        assert_eq!(
            cadence, LIFE_CADENCE_TICKS,
            "the derivation reproduces the dev fallback on Earth"
        );
        // The other owner-set time confirmations still hold.
        assert_eq!(SimClock::dev_default().world_seconds_per_tick, 1.0);
        assert_eq!(YEARS_PER_GENERATION, 10_000);
    }

    #[test]
    fn a_fast_world_and_a_slow_world_derive_different_cadences() {
        // The non-steering property at the derivation level: one formula, two orbits, two
        // cadences. A fast world (a short year) and a slow, long-year world get different life
        // cadences from the same function, and neither is the hardcoded Earth constant, so the
        // cadence is a property of the world's orbit rather than an authored per-world number. The
        // life cadence a real world runs on (`World::from_manifest_with_orbital`) is exactly this
        // derivation, so what holds here holds for `World::life_cadence_ticks`.
        let base_tick = Fixed::from_int(1);
        // A fast world: a year of one Earth day (86,400 world-seconds).
        let fast = ticks_from_seconds(Fixed::from_int(86_400), base_tick).unwrap();
        // A slow, long-year world (a Venus-scale slow spin at world scale): about four Earth years,
        // well inside the Q32.32 range, so its cadence is much longer than Earth's.
        let slow = ticks_from_seconds(Fixed::from_int(126_144_000), base_tick).unwrap();
        assert_ne!(fast, slow, "different orbits derive different cadences");
        assert_eq!(fast, 86_400);
        assert_eq!(slow, 126_144_000);
        assert_ne!(fast, 31_536_000, "the fast world is not the Earth constant");
        assert_ne!(slow, 31_536_000, "the slow world is not the Earth constant");
    }

    #[test]
    fn deriving_a_cadence_fails_loud_on_bad_inputs() {
        // The derivation never panics, wraps, or returns a zero cadence. A non-positive base tick
        // or a non-positive orbital span is a fail-loud CalibrationError, and a span shorter than
        // one base tick (which would beat every tick) is refused rather than floored to zero.
        assert!(ticks_from_seconds(Fixed::from_int(31_536_000), Fixed::ZERO).is_err());
        assert!(ticks_from_seconds(Fixed::from_int(31_536_000), Fixed::from_int(-1)).is_err());
        assert!(ticks_from_seconds(Fixed::ZERO, Fixed::from_int(1)).is_err());
        assert!(ticks_from_seconds(Fixed::from_int(-5), Fixed::from_int(1)).is_err());
        // Half a second of world-time at a one-second base tick floors to zero ticks: refused.
        assert!(ticks_from_seconds(Fixed::from_ratio(1, 2), Fixed::from_int(1)).is_err());
        // A span of exactly one base tick is the smallest admissible cadence, one tick.
        assert_eq!(
            ticks_from_seconds(Fixed::from_int(1), Fixed::from_int(1)).unwrap(),
            1
        );
    }

    #[test]
    fn the_diurnal_periods_derive_from_the_orbit_at_a_chosen_sampling() {
        // The day-night render periods are a derivation from the world's own orbit, not authored
        // tick counts. At a 128-tick day sampling the Earth fixture reproduces exactly the tick
        // counts the retired run_world literal wrote by hand (128 for the day, 128*365 for the year),
        // but now the year comes out of the world's days-per-year (orbital / rotation period) rather
        // than a hardcoded 365: the demo picks only the sampling rate, the orbit supplies the rest.
        let earth = OrbitalElements::dev_earth();
        let (rotation, orbit) = diurnal_periods_at_sampling(&earth, 128).unwrap();
        assert_eq!(
            rotation, 128,
            "the day renders in the chosen sampling ticks"
        );
        assert_eq!(
            orbit,
            128 * 365,
            "the year is the sampling times the world's days-per-year"
        );
    }

    #[test]
    fn a_different_orbit_derives_different_diurnal_periods_at_the_same_sampling() {
        // The non-steering property: one sampling rate, two orbits, two year-lengths. A world whose
        // year is 400 of its own days gets a 400-day year in ticks at the same day sampling, never
        // Earth's 365, so the calendar is a property of the world's orbit. The day always renders in
        // the sampling rate (the observer's dial), whatever the world's rotation period is.
        let sampling = 100;
        let alien = OrbitalElements {
            rotation_period_seconds: Fixed::from_int(50_000),
            orbital_period_seconds: Fixed::from_int(50_000 * 400),
        };
        let (rotation, orbit) = diurnal_periods_at_sampling(&alien, sampling).unwrap();
        assert_eq!(
            rotation, sampling,
            "the day renders in the sampling ticks for any rotation"
        );
        assert_eq!(
            orbit,
            sampling * 400,
            "the year is the sampling times this world's days-per-year"
        );
        assert_ne!(orbit, sampling * 365, "not Earth's calendar");
    }

    #[test]
    fn the_diurnal_sampling_fails_loud_on_a_zero_rate() {
        // A zero day-sampling would divide by zero to find the base tick: refused fail-loud rather
        // than panicking or fabricating a period.
        let earth = OrbitalElements::dev_earth();
        assert!(diurnal_periods_at_sampling(&earth, 0).is_err());
    }

    #[test]
    fn orbital_and_base_tick_read_live_and_fail_loud_while_reserved() {
        // The two orbital scalars are owner-set per world; a reserved manifest hands back a
        // fail-loud error, never a fabricated Earth orbit. A set manifest reads both back exactly,
        // and the base-tick reader supplies the canonical Fixed the cadence math divides by (never
        // the non-canonical f64 world_seconds_per_tick).
        let reserved = r#"
[[reserved]]
id = "world.orbital_period_seconds"
basis = "b"
status = "reserved"
source = "s"
[[reserved]]
id = "world.rotation_period_seconds"
basis = "b"
status = "reserved"
source = "s"
[[reserved]]
id = "time.base_tick_seconds"
basis = "b"
status = "set"
value = "1"
source = "s"
"#;
        let m = CalibrationManifest::from_toml_str(reserved).unwrap();
        assert!(matches!(
            orbital_from_manifest(&m).unwrap_err(),
            CalibrationError::Reserved(_)
        ));
        assert_eq!(base_tick_seconds_fixed(&m).unwrap(), Fixed::from_int(1));

        let set = r#"
[[reserved]]
id = "world.orbital_period_seconds"
basis = "b"
status = "set"
value = "86400"
source = "s"
[[reserved]]
id = "world.rotation_period_seconds"
basis = "b"
status = "set"
value = "3600"
source = "s"
[[reserved]]
id = "time.base_tick_seconds"
basis = "b"
status = "set"
value = "1"
source = "s"
"#;
        let m = CalibrationManifest::from_toml_str(set).unwrap();
        let orbital = orbital_from_manifest(&m).unwrap();
        assert_eq!(orbital.orbital_period_seconds, Fixed::from_int(86_400));
        assert_eq!(orbital.rotation_period_seconds, Fixed::from_int(3_600));
        // The whole pipeline: manifest to orbit to cadence, all fixed-point and fail-loud.
        let base_tick = base_tick_seconds_fixed(&m).unwrap();
        let cadence = ticks_from_seconds(orbital.orbital_period_seconds, base_tick).unwrap();
        assert_eq!(cadence, 86_400);
    }

    #[test]
    fn planetary_from_manifest_reads_the_two_geometry_scalars_and_fails_loud_while_reserved() {
        // Reserved: the reader refuses to fabricate a body, failing loud on the first reserved scalar.
        let reserved = r#"
[[reserved]]
id = "world.planet_radius"
basis = "b"
status = "reserved"
source = "s"
[[reserved]]
id = "world.mean_density"
basis = "b"
status = "reserved"
source = "s"
"#;
        let m = CalibrationManifest::from_toml_str(reserved).unwrap();
        assert!(matches!(
            planetary_from_manifest(&m).unwrap_err(),
            CalibrationError::Reserved(_)
        ));

        // Set: the reader reads the two owner scalars into a PlanetaryBody (Earth's values here, one option).
        let set = r#"
[[reserved]]
id = "world.planet_radius"
basis = "b"
status = "set"
value = "6371000"
source = "s"
[[reserved]]
id = "world.mean_density"
basis = "b"
status = "set"
value = "5514"
source = "s"
"#;
        let m = CalibrationManifest::from_toml_str(set).unwrap();
        let body = planetary_from_manifest(&m).unwrap();
        assert_eq!(body.radius_meters, Fixed::from_int(6_371_000));
        assert_eq!(body.mean_density, Fixed::from_int(5514));
    }

    #[test]
    fn sim_clock_reads_world_time() {
        let c = SimClock::dev_default();
        assert_eq!(c.world_seconds(60), 60.0);
        // Formatting is presentation only; check it produces the expected coarse buckets.
        assert_eq!(c.format_world_time(0), "0s");
        assert_eq!(c.format_world_time(90), "1m 30s");
        // One year of seconds reads in years.
        let one_year_ticks = (365.0 * 24.0 * 3600.0) as u64;
        assert!(c.format_world_time(one_year_ticks).ends_with('d'));
        assert!(c.format_world_time(one_year_ticks).starts_with("1y"));
    }
}
