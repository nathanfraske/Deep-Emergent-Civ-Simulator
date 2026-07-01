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

/// The base-tick duration: how much world-time one canonical tick represents. This is a RESERVED
/// owner calibration value, not a fabricated constant (design Part 54 reserves the base-tick
/// duration; the runbook manifest carries it). The mechanism above does not depend on its value,
/// since advancing by whole ticks is deterministic whatever a tick means in world-time; it is
/// used only to label the observer's speed and time readout. The development fixture below is the
/// owner's stated anchor, one in-world second per tick, which matches the design's near-one-second
/// resolution target for smooth movement and view elaboration (Part 32, R-VIEW-ELAB); the
/// authoritative value is the owner's to set.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SimClock {
    /// World-seconds represented by one base tick. RESERVED: owner value, dev fixture 1.0.
    pub world_seconds_per_tick: f64,
}

impl SimClock {
    /// A labelled DEVELOPMENT FIXTURE (one in-world second per tick), the owner's stated anchor,
    /// not an authoritative calibration.
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
        assert!((sum - expected).abs() <= 1, "chunked total tracks rate*time");
        assert!((one - expected).abs() <= 1, "single-shot total tracks rate*time");
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
        assert!(d.lod_debt() > 0, "the surplus is recorded as temporal-LOD debt");
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
        assert!(d.rate() >= MIN_RATE, "zero is clamped up to the floor, not left at zero");
        d.set_rate(1.0e30);
        assert!(d.rate() <= MAX_RATE, "an enormous rate is clamped to the ceiling");
        d.scale_rate(0.0); // ignored (non-positive)
        assert!(d.rate() <= MAX_RATE);
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
