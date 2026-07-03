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

//! The absence-window substrate: how long a being is absent before it is presumed dead (design
//! Part 9.9, Part 20; the derive half of R-EVIDENCE `absence_windows`).
//!
//! The evidence engine once carried the presumed-dead window as an owner calibration anchored to a
//! human legal convention (the seven-year Anglo-American death-in-absentia rule, scaled to a human
//! lifespan). That anchor has no meaning for a race that lives a month or a millennium, or one
//! whose members are hard to see coming and going. The window is a function of two things a race
//! already owns: how visible its members are (a less visible race's absence takes longer to
//! confirm, so the window is longer) and how long it characteristically lives (no one is presumed
//! merely absent for longer than their own lifespan would allow). This module derives the window
//! from those, through a fixed function reading per-race DATA and never a concrete
//! [`crate::value::RaceId`] (Principle 9).
//!
//! The absence STAGES are a race-independent universal set (a recent absence, a prolonged one, a
//! long one), each a multiplier on the base window; a race enters the same stages, and only its
//! own visibility and characteristic lifespan set how long each stage runs for it. The stage
//! multipliers are the reserved inputs, surfaced as the labelled dev fixture below.
//!
//! Scope: [`absence_window`] is a pure standalone derivation. Wiring it into the running world (a
//! per-being absence clock, and the `PlaceId`/cadence-to-tick bridge through
//! `time.life_cadence_ticks` that would convert the characteristic-lifespan cap into exact ticks)
//! is a NAMED FOLLOW-ON, deferred so this build stays additive.

use civsim_core::Fixed;

use crate::base_rates::RaceBaseRates;
use crate::decision::Curve;
use crate::demography::hazard_age;

/// One absence stage: a label and a multiplier on the base window. Race-independent (a universal
/// set every race passes through); how long the stage runs for a given race comes from that race's
/// own visibility and characteristic lifespan, not from the stage.
#[derive(Clone, Debug)]
pub struct AbsenceStage {
    /// A human-readable label for the stage (diagnostic; not keyed on).
    pub label: String,
    /// The multiplier this stage applies to the base window. RESERVED. Basis: how much longer a
    /// deeper stage of absence runs before presumption, a monotone escalation, not a per-race value.
    pub multiplier: Fixed,
}

/// A data-defined absence-schedule identifier (a newtype), so a world can carry more than one
/// schedule (Principle 11).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AbsenceScheduleId(pub u32);

/// An absence schedule: the ordered universal set of stages a being passes through while absent.
#[derive(Clone, Debug)]
pub struct AbsenceScheduleDef {
    /// The schedule's identifier.
    pub id: AbsenceScheduleId,
    /// The stages, in escalating order.
    pub stages: Vec<AbsenceStage>,
}

impl AbsenceScheduleDef {
    /// A labelled DEVELOPMENT FIXTURE, not owner values, so the derivation runs and can be tested
    /// now. Three escalating stages (recent, prolonged, long-lost). The multipliers are a fixture,
    /// never a fabricated calibration; the real escalation is the owner's data.
    pub fn dev_default() -> AbsenceScheduleDef {
        AbsenceScheduleDef {
            id: AbsenceScheduleId(0),
            stages: vec![
                AbsenceStage {
                    label: "recent".to_string(),
                    multiplier: Fixed::ONE,
                },
                AbsenceStage {
                    label: "prolonged".to_string(),
                    multiplier: Fixed::from_int(2),
                },
                AbsenceStage {
                    label: "long_lost".to_string(),
                    multiplier: Fixed::from_int(4),
                },
            ],
        }
    }
}

/// The per-cadence hazard at or above which an age counts as the characteristic-lifespan mark. This
/// is a DEFINITIONAL percentile convention (the age at which a member faces even odds of dying
/// within one cadence, a characteristic scale in the sense a physical half-life is), NOT an authored
/// biology value: it reads a fixed fraction off whatever hazard curve the race supplies, so the
/// characteristic lifespan is derived from the race's own life table.
pub const LIFESPAN_HAZARD_THRESHOLD: Fixed = Fixed::from_bits(1i64 << (Fixed::FRAC_BITS - 1));

/// The step, in age units, at which [`characteristic_lifespan`]'s scan advances when called from
/// [`absence_window`], and the ceiling it scans to. These are a DETERMINISM-AND-PERFORMANCE scan
/// resolution and bound (a finite, reproducible scan), not biology values; a finer step is more
/// accurate at more cost.
const ABSENCE_PROBE_STEP: u32 = 1;
const ABSENCE_PROBE_MAX: u32 = 4096;

/// The characteristic lifespan of a race, in age units: the first probed age at which its
/// per-cadence hazard reaches [`LIFESPAN_HAZARD_THRESHOLD`], scanning from zero by `probe_step` up
/// to `probe_max`. A hazard that never reaches the threshold within the bound reads `probe_max` (the
/// scan cap). Deterministic: a fixed scan over [`Curve::eval`], no float and no RNG.
///
/// This tracks the hazard curve's own shape: stretch the curve horizontally and the crossing age,
/// and so the characteristic lifespan, stretches with it proportionally.
pub fn characteristic_lifespan(hazard: &Curve, probe_step: u32, probe_max: u32) -> u32 {
    let step = probe_step.max(1); // a zero step would not advance; keep the scan finite
    let mut age: u32 = 0;
    loop {
        let h = hazard.eval(hazard_age(age)).clamp(Fixed::ZERO, Fixed::ONE);
        if h >= LIFESPAN_HAZARD_THRESHOLD || age >= probe_max {
            return age.min(probe_max);
        }
        age = age.saturating_add(step).min(probe_max);
    }
}

/// The absence window, in the same tick units as `base_check_interval`, that a being of the race
/// whose base rates are `race` runs for before `stage`'s presumption applies.
///
/// The window is `base_check_interval * (stage.multiplier / race.visibility)`: a deeper stage and a
/// less visible race both lengthen it, since an absence that is harder to observe takes longer to
/// confirm. It derives from the race's own visibility, never an authored per-race window. It is
/// then capped so no one is presumed merely absent for longer than their own characteristic
/// lifespan allows: the cap is `base_check_interval * characteristic_lifespan(...)`, one check
/// interval per characteristic-lifespan step. A zero-visibility (unobservable) race saturates to
/// the cap. Deterministic integer arithmetic throughout.
pub fn absence_window(stage: &AbsenceStage, race: &RaceBaseRates, base_check_interval: u64) -> u64 {
    let char_life = characteristic_lifespan(
        &race.natural_mortality,
        ABSENCE_PROBE_STEP,
        ABSENCE_PROBE_MAX,
    );
    let cap = base_check_interval.saturating_mul(char_life as u64);
    if race.visibility <= Fixed::ZERO {
        // An unobservable race's absence can never be confirmed early: saturate to the cap.
        return cap;
    }
    // stretch = multiplier / visibility (a Fixed >= 0); lower visibility lengthens it.
    let stretch = match stage.multiplier.checked_div(race.visibility) {
        Some(s) => s,
        // An unrepresentably large stretch saturates to the cap.
        None => return cap,
    };
    let window = scale_u64_by_fixed(base_check_interval, stretch);
    window.min(cap)
}

/// Floor of `n * f` for a non-negative [`Fixed`] `f`, saturating rather than wrapping. `f` in
/// Q32.32 means `n * f = n * f.to_bits() / 2^32`; the product is formed in `u128` (which cannot
/// overflow for `u64 * i64` magnitudes) and shifted back.
fn scale_u64_by_fixed(n: u64, f: Fixed) -> u64 {
    if f <= Fixed::ZERO {
        return 0;
    }
    let prod = (n as u128).saturating_mul(f.to_bits() as u128) >> Fixed::FRAC_BITS;
    prod.min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::RaceId;

    fn race(id: u32, mortality: Curve, visibility: Fixed) -> RaceBaseRates {
        RaceBaseRates {
            race: RaceId(id),
            natural_mortality: mortality,
            visibility,
            decay_multiplier: Fixed::ONE,
        }
    }

    fn rising_hazard() -> Curve {
        Curve::new([
            (Fixed::from_int(0), Fixed::from_ratio(1, 100)),
            (Fixed::from_int(50), Fixed::from_ratio(1, 20)),
            (Fixed::from_int(80), Fixed::from_ratio(1, 2)),
            (Fixed::from_int(120), Fixed::ONE),
        ])
    }

    // === Non-steering swap (2): visibility is the sole author of the absence window ===

    #[test]
    fn a_less_visible_race_gets_a_longer_window_at_every_stage() {
        let schedule = AbsenceScheduleDef::dev_default();
        let base = 1000u64;
        // Two races identical except visibility; the same rising hazard, so the same cap.
        let low = race(0, rising_hazard(), Fixed::from_ratio(1, 4));
        let high = race(1, rising_hazard(), Fixed::from_ratio(3, 4));
        for stage in &schedule.stages {
            let w_low = absence_window(stage, &low, base);
            let w_high = absence_window(stage, &high, base);
            assert!(
                w_low > w_high,
                "the less visible race waits strictly longer at stage {} ({w_low} > {w_high})",
                stage.label
            );
        }
    }

    #[test]
    fn swapping_visibility_swaps_the_window_assignment() {
        let schedule = AbsenceScheduleDef::dev_default();
        let base = 1000u64;
        let stage = &schedule.stages[1];
        let low = race(0, rising_hazard(), Fixed::from_ratio(1, 4));
        let high = race(1, rising_hazard(), Fixed::from_ratio(3, 4));
        let w_low = absence_window(stage, &low, base);
        let w_high = absence_window(stage, &high, base);

        // Swap the visibilities; the assignment swaps and the values track the visibility, not the id.
        let low2 = race(0, rising_hazard(), Fixed::from_ratio(3, 4));
        let high2 = race(1, rising_hazard(), Fixed::from_ratio(1, 4));
        let w_low2 = absence_window(stage, &low2, base);
        let w_high2 = absence_window(stage, &high2, base);
        assert!(
            w_low2 < w_high2,
            "swapping visibility swaps which waits longer"
        );
        assert_eq!(
            w_low, w_high2,
            "the window tracks visibility, not the race label"
        );
        assert_eq!(w_high, w_low2);
    }

    #[test]
    fn a_deeper_stage_runs_longer() {
        let schedule = AbsenceScheduleDef::dev_default();
        let r = race(0, rising_hazard(), Fixed::from_ratio(1, 2));
        let base = 1000u64;
        let w0 = absence_window(&schedule.stages[0], &r, base);
        let w1 = absence_window(&schedule.stages[1], &r, base);
        let w2 = absence_window(&schedule.stages[2], &r, base);
        assert!(
            w0 < w1 && w1 < w2,
            "each escalating stage runs longer ({w0} < {w1} < {w2})"
        );
    }

    #[test]
    fn an_unobservable_race_saturates_to_the_cap() {
        let schedule = AbsenceScheduleDef::dev_default();
        let r = race(0, rising_hazard(), Fixed::ZERO);
        let base = 1000u64;
        let char_life =
            characteristic_lifespan(&r.natural_mortality, ABSENCE_PROBE_STEP, ABSENCE_PROBE_MAX);
        let cap = base * char_life as u64;
        assert_eq!(
            absence_window(&schedule.stages[0], &r, base),
            cap,
            "a zero-visibility race is capped at its characteristic lifespan"
        );
    }

    // === characteristic_lifespan tracks a horizontal stretch of the hazard ===

    #[test]
    fn characteristic_lifespan_scales_with_a_horizontal_stretch() {
        // A hazard that crosses 0.5 at age 50, and the same curve stretched 2x horizontally
        // (crossing at age 100). The characteristic lifespan should double.
        let base = Curve::new([
            (Fixed::from_int(0), Fixed::ZERO),
            (Fixed::from_int(100), Fixed::ONE),
        ]);
        let stretched = Curve::new([
            (Fixed::from_int(0), Fixed::ZERO),
            (Fixed::from_int(200), Fixed::ONE),
        ]);
        let cl_base = characteristic_lifespan(&base, 1, 4096);
        let cl_stretched = characteristic_lifespan(&stretched, 1, 4096);
        assert_eq!(
            cl_base, 50,
            "the base hazard crosses the threshold at age 50"
        );
        assert_eq!(
            cl_stretched, 100,
            "the 2x-stretched hazard crosses at age 100"
        );
        assert_eq!(
            cl_stretched,
            2 * cl_base,
            "the characteristic lifespan stretches proportionally with the hazard"
        );
    }

    #[test]
    fn a_never_crossing_hazard_reads_the_scan_cap() {
        // A flat, low hazard that never reaches the threshold reads the probe cap.
        let flat = Curve::new([(Fixed::from_int(0), Fixed::from_ratio(1, 100))]);
        assert_eq!(characteristic_lifespan(&flat, 1, 500), 500);
    }
}
