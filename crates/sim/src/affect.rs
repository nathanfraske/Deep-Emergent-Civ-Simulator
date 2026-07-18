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

//! Transient affect: the fast, event-driven emotional state of a being (the R-EMOTION gap).
//!
//! Affect is distinct from the two layers already modelled. It is not the stable personality
//! of the being model (Part 20, deliberately rank-order-stable across a life), and it is not a
//! drive (a need that rises while unmet). It is the moment-to-moment colouring an event leaves
//! on a mind, which decays back toward a baseline or, when strong enough, hardens that baseline
//! (trauma). The owner's two decisions shape this build, each non-final: affect is a separate
//! data-driven layer (an affect-axis registry, sibling to the drive and value substrates, not a
//! closed emotion enum), and the appraisal is derived from the agent's own drives rather than an
//! authored event-to-emotion table, so the engine encodes no designer-preferred reaction (the
//! anti-steering choice the gap flags).
//!
//! Everything here is integer and fixed-point, a pure function of its inputs, so a mind's
//! affective history is bit-identical on replay. The numeric calibrations (the appraisal gains,
//! the decay rate, the trauma threshold and hardening fraction) are reserved owner values,
//! supplied by the caller, never invented here. Wiring affect into the decision considerations
//! (Part 8) so it modulates choice is the named follow-on; this brick is the substrate and the
//! derived appraisal.

use std::collections::BTreeMap;

use civsim_core::Fixed;

use civsim_bio::decision::DriveId;

/// A data-defined affect-axis identifier (a dimension of feeling a race carries, Part 40). The
/// axis set is a registry, not a closed list of named emotions, so an alien affect space is a
/// point in the same substrate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AffectAxisId(pub u32);

/// A being's transient affective state: a current value and a baseline per affect axis, each
/// in the bipolar range `[-1, 1]`. Affect decays from the current toward the baseline; trauma
/// drifts the baseline itself. An axis with no entry reads zero.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AffectState {
    current: BTreeMap<AffectAxisId, Fixed>,
    baseline: BTreeMap<AffectAxisId, Fixed>,
}

impl AffectState {
    /// A flat affective state at the zero baseline.
    pub fn new() -> Self {
        AffectState::default()
    }

    /// The current felt level on an axis (the baseline if there is no transient value, zero if
    /// the axis is untouched).
    pub fn level(&self, axis: AffectAxisId) -> Fixed {
        self.current
            .get(&axis)
            .or_else(|| self.baseline.get(&axis))
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// The baseline an axis decays toward.
    pub fn baseline(&self, axis: AffectAxisId) -> Fixed {
        self.baseline.get(&axis).copied().unwrap_or(Fixed::ZERO)
    }

    /// Apply an affective delta to an axis (an event's appraised impact), clamped to range.
    pub fn apply(&mut self, axis: AffectAxisId, delta: Fixed) {
        let next = (self.level(axis) + delta).clamp(Fixed::ZERO - Fixed::ONE, Fixed::ONE);
        self.current.insert(axis, next);
    }

    /// Decay every transient value toward its baseline by `rate` (0 leaves it, 1 snaps it to
    /// baseline): `current += (baseline - current) * rate`. A pure relaxation, so affect fades
    /// deterministically between events.
    pub fn decay(&mut self, rate: Fixed) {
        let rate = rate.clamp(Fixed::ZERO, Fixed::ONE);
        let axes: Vec<AffectAxisId> = self.current.keys().copied().collect();
        for axis in axes {
            let base = self.baseline(axis);
            let cur = self.level(axis);
            let next = cur + (base - cur).mul(rate);
            self.current.insert(axis, next);
        }
    }

    /// Harden the baseline under a sustained strong feeling (trauma): when the current value's
    /// deviation from the baseline exceeds `threshold`, the baseline drifts toward the current
    /// by `fraction` of the excess, so an overwhelming or repeated experience leaves a
    /// persistent residue the ordinary decay no longer erases. Returns whether the baseline
    /// moved. The threshold and fraction are reserved owner values.
    pub fn harden(&mut self, axis: AffectAxisId, threshold: Fixed, fraction: Fixed) -> bool {
        let base = self.baseline(axis);
        let cur = self.level(axis);
        let dev = cur - base;
        if dev.abs() <= threshold {
            return false;
        }
        let frac = fraction.clamp(Fixed::ZERO, Fixed::ONE);
        let new_base = (base + dev.mul(frac)).clamp(Fixed::ZERO - Fixed::ONE, Fixed::ONE);
        self.baseline.insert(axis, new_base);
        true
    }
}

/// How a change in one drive appraises into affect (the derived appraisal law). Data, so the
/// engine authors no fixed reaction: which affect axis a drive feeds, the gain mapping the
/// drive change to an affect delta (reserved), and whether relief (a fall in the drive) reads
/// positive on the axis or a rise does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DriveAppraisal {
    /// The affect axis a change in this drive feeds.
    pub axis: AffectAxisId,
    /// The gain mapping drive-change magnitude to an affect delta (reserved).
    pub gain: Fixed,
    /// Whether a fall in the drive (relief) is positive on the axis; if false, a rise is.
    pub relief_positive: bool,
}

/// The per-race appraisal binding (data): which affect axis each drive feeds and how. This is
/// the whole of the appraisal law's content, and it is data, so a race's emotional reactions
/// are its own and nothing is hardcoded.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AppraisalBinding {
    per_drive: BTreeMap<DriveId, DriveAppraisal>,
}

impl AppraisalBinding {
    /// An empty binding.
    pub fn new() -> Self {
        AppraisalBinding::default()
    }

    /// Bind a drive to an affect appraisal.
    pub fn bind(&mut self, drive: DriveId, appraisal: DriveAppraisal) {
        self.per_drive.insert(drive, appraisal);
    }

    /// The affect axis and signed delta a change in `drive` evokes (a measured consequence of
    /// how the event moved the drive), or `None` if the race does not appraise that drive. The
    /// delta is `drive_change * gain` with the sign set by `relief_positive`, so a relieved
    /// need and a worsened one push opposite ways without an authored event-to-emotion table.
    pub fn delta(&self, drive: DriveId, drive_change: Fixed) -> Option<(AffectAxisId, Fixed)> {
        let a = self.per_drive.get(&drive)?;
        let directed = if a.relief_positive {
            Fixed::ZERO - drive_change
        } else {
            drive_change
        };
        Some((a.axis, directed.mul(a.gain)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOY: AffectAxisId = AffectAxisId(0);
    const HUNGER: DriveId = DriveId(0);

    #[test]
    fn affect_applies_and_clamps() {
        let mut a = AffectState::new();
        a.apply(JOY, Fixed::from_ratio(1, 2));
        assert_eq!(a.level(JOY), Fixed::from_ratio(1, 2));
        a.apply(JOY, Fixed::ONE); // pushes past the ceiling
        assert_eq!(a.level(JOY), Fixed::ONE, "clamped to the bipolar ceiling");
    }

    #[test]
    fn affect_decays_toward_baseline() {
        let mut a = AffectState::new();
        a.apply(JOY, Fixed::ONE);
        a.decay(Fixed::from_ratio(1, 2)); // halfway to the zero baseline
        assert_eq!(a.level(JOY), Fixed::from_ratio(1, 2));
        a.decay(Fixed::ONE); // snap to baseline
        assert_eq!(a.level(JOY), Fixed::ZERO);
    }

    #[test]
    fn a_strong_feeling_hardens_the_baseline_as_trauma() {
        let mut a = AffectState::new();
        a.apply(JOY, Fixed::ONE);
        // Below threshold: no hardening.
        assert!(!a.harden(JOY, Fixed::from_ratio(2, 1), Fixed::from_ratio(1, 2)));
        assert_eq!(a.baseline(JOY), Fixed::ZERO);
        // Above threshold: the baseline drifts toward the sustained feeling.
        assert!(a.harden(JOY, Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2)));
        assert_eq!(
            a.baseline(JOY),
            Fixed::from_ratio(1, 2),
            "half the excess became permanent"
        );
        // Now decay no longer returns all the way to zero.
        a.decay(Fixed::ONE);
        assert_eq!(
            a.level(JOY),
            Fixed::from_ratio(1, 2),
            "the residue persists"
        );
    }

    #[test]
    fn appraisal_derives_a_signed_delta_from_a_drive_change() {
        let mut b = AppraisalBinding::new();
        // A fall in hunger (relief) reads positive on joy, gain 2.
        b.bind(
            HUNGER,
            DriveAppraisal {
                axis: JOY,
                gain: Fixed::from_int(2),
                relief_positive: true,
            },
        );
        // Hunger fell by 0.25 (relieved): joy rises by 0.25 * 2 = 0.5.
        let (axis, delta) = b
            .delta(HUNGER, Fixed::ZERO - Fixed::from_ratio(1, 4))
            .unwrap();
        assert_eq!(axis, JOY);
        assert_eq!(delta, Fixed::from_ratio(1, 2));
        // Hunger rose by 0.25 (worsened): joy falls by 0.5.
        let (_, delta) = b.delta(HUNGER, Fixed::from_ratio(1, 4)).unwrap();
        assert_eq!(delta, Fixed::ZERO - Fixed::from_ratio(1, 2));
        // An unbound drive does not appraise.
        assert_eq!(b.delta(DriveId(99), Fixed::ONE), None);
    }
}
