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

//! The experiential associative-learning substrate: a being correlates its own interoceptive harm
//! signal with the raw feature of the ground it stands on, and forms the belief "this feature harms
//! me" for itself (harm-learning arc slice b; Principles 8, 9, 11).
//!
//! This is the retirement of the injected `HAZARD` Observe. The old run authored the belief: it read a
//! high-level fact (a salinity dose over a reserved threshold) and pushed a pre-formed "a hazard is
//! present" observation into every being over that dose, so the being reasoned nothing. Here the being
//! reasons it. Each tick it feels whether a reserve fell beyond the ordinary metabolic-drain noise
//! (its OWN raw interoceptive signal, [`crate::homeostasis::is_harm_tick`] over the reserve delta), and
//! it senses the raw feature of the cell it stands on ([`crate::percept`]). It then contributes one
//! piece of evidence per present feature toward "this feature harms me" (a harm tick) or "this feature
//! is benign" (a harm-free tick), keyed on a per-feature belief subject.
//!
//! The associative learner IS the existing evidence engine ([`crate::evidence::InferenceFrame`] through
//! [`crate::agent::Mind::consider`]), keyed differently: the one global hazard subject becomes a
//! per-feature subject minted from the quantized feature the being senses, and the evidence weight is
//! the general [`crate::evidence::good_weight`] of two reserved likelihoods. Because the belief is then
//! an ordinary `(subject, attr)` frame, the shipped overhearing transmission carries it for free, and
//! the identical loop learns any ground-kind, any good place, any food that sickens, with zero
//! per-hazard code. Nothing reads a dose threshold, a hazard label, or a race id: the sign comes from
//! the being's own reserve falling, the subject from a raw quantized percept, and "this feature harms
//! me" emerges from the correlation over selection (Principles 8, 9).

use civsim_core::{Fixed, StableId};
use civsim_world::Coord3;

use crate::agent::Mind;
use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::evidence::{good_weight, AttrKindId, InferenceParams, ValueId};
use crate::locomotion::ResourceField;
use crate::percept::{feature_bucket, PerceptRegistry};

/// The generic attribute every experientially-learned feature belief is ABOUT: "does standing on this
/// feature harm me". One attribute for all features (the feature identity lives in the subject), a
/// reserved-high id disjoint from every other belief attribute so a harm belief never aliases another.
pub const HARM_ATTR: AttrKindId = AttrKindId(u32::MAX - 2);

/// The value meaning "this feature harms me": the belief a being forms when harm correlates with the
/// feature.
pub const HARMS: ValueId = 1;
/// The value meaning "this feature is benign": the competing hypothesis, reinforced on harm-free ticks
/// so the belief is defeasible for free (a feature that stops harming is un-learned).
pub const BENIGN: ValueId = 0;

/// The reserved base of the per-feature belief-subject band. A feature subject is minted at or above
/// this base, packed with the feature channel and its quantized bucket, so "salinity-bucket-2 ground
/// harms me" is a belief about a FEATURE-KIND and can never alias a belief about a being (being ids are
/// minted incrementally from zero and stay far below this base) or a reserved-high landmark id (the
/// packed subject stays below `1 << 63`). The band is the sibling of the conversational-cell place band
/// (`CELL_PLACE_BASE`): a stable function of the percept, folding into no stream on its own.
const FEATURE_SUBJECT_BASE: u64 = 1 << 62;
/// The bit position the feature channel is packed at, above the 32-bit bucket field.
const FEATURE_CHANNEL_SHIFT: u32 = 32;
/// The mask for the 32-bit bucket field.
const FEATURE_BUCKET_MASK: u64 = 0xFFFF_FFFF;

/// Mint the belief subject for a perceived feature: the `(channel, bucket)` pair packed into the
/// reserved feature-subject band. Two cells whose feature amount lands in the same bucket mint the SAME
/// subject, so a belief learned on one ground applies to another of the same perceived kind (the
/// generalisation the quantization buys); a different channel or a different bucket is a different
/// subject. The bucket is clamped non-negative (a feature amount is non-negative) and into the 32-bit
/// field, so the packing never overflows the band.
pub fn feature_subject(channel: u16, bucket: i64) -> StableId {
    let bucket = (bucket.max(0) as u64) & FEATURE_BUCKET_MASK;
    StableId(FEATURE_SUBJECT_BASE | ((channel as u64) << FEATURE_CHANNEL_SHIFT) | bucket)
}

/// The reserved calibrations of the associative learner (Principle 11): the numbers that set when a
/// reserve fall counts as harm, how coarse the feature percept is, and how strong a single correlation
/// observation is. The mechanism is fixed Rust; these are the owner's to set, surfaced with a basis,
/// never fabricated. They REPLACE the retired `hazard_dose_threshold` and `hazard_weight`, which
/// authored the belief the being now forms for itself.
#[derive(Clone, Copy, Debug)]
pub struct HarmLearningCalib {
    /// The harm-noise floor: a per-tick reserve fall no deeper than this is ordinary metabolic drain,
    /// not harm ([`crate::homeostasis::is_harm_tick`]). RESERVED. Basis: the largest per-tick reserve
    /// fall a resting, unharmed body incurs (the resting `base_drain` scaled to the tick), so only a
    /// supra-drain fall registers as harm.
    pub harm_noise_floor: Fixed,
    /// The feature granularity: the quantization step that buckets a raw feature amount into a perceived
    /// kind ([`crate::percept::feature_bucket`]). RESERVED. Basis: the sensorium's per-class
    /// just-noticeable difference for the sensed substance, coarse enough that ordinary spatial
    /// variation in a hazard reads as one or two kinds rather than a continuum.
    pub feature_granularity: Fixed,
    /// P(the harm signal fires | this feature harms me): the base rate of a felt reserve fall on a cell
    /// whose feature is truly harmful. RESERVED. Basis: the fraction of ticks a naive being on a harmful
    /// cell is worn faster than it heals, the dose-and-physics harm rate the floor implies, set as
    /// `trace.rs` sets an observation reliability.
    pub p_harm_given_harms: Fixed,
    /// P(the harm signal fires | this feature is benign): the base rate of a spurious felt fall on a
    /// benign cell. RESERVED. Basis: the false-attribution rate of a transient reserve dip unrelated to
    /// the feature, set as `trace.rs` sets a false-attribution rate; low, so a benign cell rarely earns
    /// harm evidence.
    pub p_harm_given_benign: Fixed,
    /// The certainty clamp the per-observation weight is bounded to. RESERVED. Basis: the evidence
    /// engine's log-odds clamp (`evidence.log_odds_clamp`), set equal to it so a single correlation
    /// observation cannot exceed the engine's maximum admissible certainty.
    pub certainty_clamp: Fixed,
}

impl HarmLearningCalib {
    /// Read the learner calibrations fail-loud from the manifest (Principle 11): a reserved value left
    /// unset refuses to build rather than running on a fabricated default. The certainty clamp reads the
    /// same `evidence.log_odds_clamp` the inference engine uses, so the two agree by construction.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<HarmLearningCalib, CalibrationError> {
        Ok(HarmLearningCalib {
            harm_noise_floor: m.require_fixed("harm.noise_floor")?,
            feature_granularity: m.require_fixed("harm.feature_granularity")?,
            p_harm_given_harms: m.require_fixed("harm.p_harm_given_harms")?,
            p_harm_given_benign: m.require_fixed("harm.p_harm_given_benign")?,
            certainty_clamp: m.require_fixed("evidence.log_odds_clamp")?,
        })
    }

    /// A labelled DEVELOPMENT FIXTURE standing up the same magnitudes the manifest would carry, for the
    /// test and harness paths that build a runner without a manifest. A noise floor of a hundredth (above
    /// the energy base drain, below the salt-flat condition harm), a unit granularity (a salt-flat dose
    /// range buckets into one or two kinds), likelihoods of nine-tenths and a tenth (so a single harm
    /// observation carries ln(9) of evidence and a naive being commits in a couple of harm ticks), and
    /// the engine's fifty-nat log-odds clamp.
    pub fn dev_default() -> HarmLearningCalib {
        HarmLearningCalib {
            harm_noise_floor: Fixed::from_ratio(1, 100),
            feature_granularity: Fixed::ONE,
            p_harm_given_harms: Fixed::from_ratio(9, 10),
            p_harm_given_benign: Fixed::from_ratio(1, 10),
            certainty_clamp: Fixed::from_int(50),
        }
    }

    /// The unsigned per-observation weight a single correlation carries: the I.J. Good weight of evidence
    /// of the two reserved likelihoods, `ln(P(harm|harms) / P(harm|benign))`, clamped to the certainty
    /// bound. General over the two probabilities, reading no kind, race, or feature (the feature identity
    /// is in the subject, not the weight).
    pub fn observation_weight(&self) -> Fixed {
        good_weight(
            self.p_harm_given_harms,
            self.p_harm_given_benign,
            self.certainty_clamp,
        )
    }
}

/// One piece of evidence a being contributes this tick: the per-feature subject to key it on, the value
/// it points toward (`HARMS` on a harm tick, `BENIGN` otherwise), and the signed weight. Fed straight
/// into [`crate::agent::Mind::consider`] (which scales it by the mind's acuity and accumulates it into
/// the `(subject, HARM_ATTR)` frame), so the belief commits at read past the engine's threshold and
/// margin with no learner-specific commit logic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FeatureObservation {
    /// The per-feature belief subject ([`feature_subject`]).
    pub subject: StableId,
    /// The value this observation points toward (`HARMS` or `BENIGN`).
    pub toward: ValueId,
    /// The signed evidence weight (the observation weight, scaled by the being's plasticity).
    pub weight: Fixed,
}

/// The observations a being makes this tick: one per PRESENT feature of the cell it stands on
/// (a channel whose amount is positive), toward `HARMS` if it felt harm this tick and `BENIGN`
/// otherwise, each weighted by the reserved observation weight scaled by the being's belief plasticity
/// (its heritable learning rate, `Mind::plasticity`, neutral at one). A cell with no present feature
/// yields nothing (there is nothing to correlate). The harm bit is the being's OWN interoceptive signal
/// (whether any reserve fell beyond the noise floor); the feature is a raw percept; nothing reads a dose
/// threshold, a hazard label, or a race id, so the belief emerges from correlation (Principles 8, 9).
pub fn feature_observations(
    harm: bool,
    features: &[Fixed],
    plasticity: Fixed,
    calib: &HarmLearningCalib,
) -> Vec<FeatureObservation> {
    let base = calib.observation_weight();
    let weight = base.checked_mul(plasticity).unwrap_or(base);
    let toward = if harm { HARMS } else { BENIGN };
    features
        .iter()
        .enumerate()
        .filter(|(_, &amount)| amount > Fixed::ZERO)
        .map(|(channel, &amount)| {
            let bucket = feature_bucket(amount, calib.feature_granularity);
            FeatureObservation {
                subject: feature_subject(channel as u16, bucket),
                toward,
                weight,
            }
        })
        .collect()
}

/// The belief-derived expected-harm AVOIDANCE gradient for a being at `here` (harm-learning arc slice
/// c): the summed inverse-distance repulsion away from every cell within `sense_range` whose feature-kind
/// the being holds a committed HARMS belief about. Each believed-harmful cell contributes a vector
/// pointing from the cell toward the being, weighted by inverse distance (nearer harmful ground repels
/// harder), so the raw sum points away from the bulk of believed harm; a being that believes nothing
/// nearby harms it gets a zero gradient. The caller normalises the raw sum to a unit percept, exactly as
/// the temperature gradient is normalised.
///
/// This is a PERCEPT, not a heading. The runner feeds it into the reserve axis's direction slot the
/// evolved controller reads, and ONLY a heritable weight lifted off founding-zero by selection turns it
/// into avoidance, so the flight behaviour emerges rather than being authored (Principle 9): the
/// mechanism never subtracts a harm term from the heading itself. It reads the being's own beliefs and
/// the raw features it senses, never a hazard label or a race id (Principle 8). Pure and RNG-free.
pub fn avoidance_gradient(
    mind: &Mind,
    here: Coord3,
    resources: &ResourceField,
    percepts: &PerceptRegistry,
    sense_range: i64,
    granularity: Fixed,
    params: &InferenceParams,
) -> (Fixed, Fixed) {
    if percepts.is_empty() {
        return (Fixed::ZERO, Fixed::ZERO);
    }
    let mut ax = Fixed::ZERO;
    let mut ay = Fixed::ZERO;
    let r = sense_range.max(0) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx == 0 && dy == 0 {
                continue;
            }
            let cell = Coord3::ground(here.x + dx, here.y + dy);
            let features = percepts.perceive(resources.composition(cell));
            let believes_harm = features.iter().enumerate().any(|(channel, &amount)| {
                amount > Fixed::ZERO && {
                    let subject =
                        feature_subject(channel as u16, feature_bucket(amount, granularity));
                    mind.belief(subject, HARM_ATTR, params) == Some(HARMS)
                }
            });
            if believes_harm {
                // A vector from the harmful cell toward the being (away from the harm), weighted by
                // inverse distance: (-dx, -dy) / (dx^2 + dy^2). No square root; nearer harm repels harder.
                let d2 = Fixed::from_int(dx * dx + dy * dy);
                if let (Some(cx), Some(cy)) = (
                    Fixed::from_int(-dx).checked_div(d2),
                    Fixed::from_int(-dy).checked_div(d2),
                ) {
                    ax = ax.saturating_add(cx);
                    ay = ay.saturating_add(cy);
                }
            }
        }
    }
    (ax, ay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::homeostasis::{HomeostaticRegistry, CONDITION};
    use crate::physiology::SALINITY;

    fn params() -> InferenceParams {
        // The evidence engine's dev thresholds: clamp 50, commit at 3 nats, margin 1.
        InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        }
    }

    #[test]
    fn a_feature_subject_is_disjoint_from_beings_and_landmarks_and_keys_by_channel_and_bucket() {
        let s = feature_subject(0, 2);
        // Above every being id (minted from zero) and below the reserved-high landmark ids.
        assert!(s.0 >= FEATURE_SUBJECT_BASE);
        assert!(s.0 < u64::MAX - 1);
        // Same channel and bucket mint the same subject (the generalisation); a different bucket or
        // channel is a different subject.
        assert_eq!(feature_subject(0, 2), s);
        assert_ne!(feature_subject(0, 3), s);
        assert_ne!(feature_subject(1, 2), s);
        // A negative bucket clamps to zero rather than wrapping the packing.
        assert_eq!(feature_subject(0, -5), feature_subject(0, 0));
    }

    #[test]
    fn only_present_features_earn_an_observation_pointed_by_the_harm_bit() {
        let calib = HarmLearningCalib::dev_default();
        // Two channels: the first absent (zero), the second present (a salt dose of two).
        let features = vec![Fixed::ZERO, Fixed::from_int(2)];
        let harm = feature_observations(true, &features, Fixed::ONE, &calib);
        assert_eq!(
            harm.len(),
            1,
            "only the present feature earns an observation"
        );
        assert_eq!(harm[0].toward, HARMS, "a harm tick points toward HARMS");
        assert_eq!(
            harm[0].subject,
            feature_subject(
                1,
                feature_bucket(Fixed::from_int(2), calib.feature_granularity)
            )
        );
        assert!(
            harm[0].weight > Fixed::ZERO,
            "the observation carries positive evidence"
        );
        // A harm-free tick on the same cell points the same subject toward BENIGN.
        let benign = feature_observations(false, &features, Fixed::ONE, &calib);
        assert_eq!(benign[0].toward, BENIGN);
        assert_eq!(benign[0].subject, harm[0].subject);
        // A cell with no present feature yields nothing to correlate.
        assert!(feature_observations(true, &[Fixed::ZERO], Fixed::ONE, &calib).is_empty());
    }

    #[test]
    fn a_being_repeatedly_harmed_on_a_feature_commits_harms_on_its_own() {
        // The milestone core: a mind fed repeated harm-while-on-the-feature commits a HARMS belief about
        // that feature-kind, with no injected observation, purely from the correlation.
        let calib = HarmLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)]; // one present feature (the salt dose)
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(1), Fixed::ONE); // neutral acuity

        // Before any experience, no belief.
        assert_eq!(mind.belief(subject, HARM_ATTR, &params()), None);
        // Feel harm on the feature for three ticks.
        for _ in 0..3 {
            for obs in feature_observations(true, &features, Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, HARM_ATTR, &params()),
            Some(HARMS),
            "the being forms the HARMS belief about the feature from its own repeated harm"
        );
    }

    #[test]
    fn a_being_never_on_the_feature_never_forms_the_belief() {
        let calib = HarmLearningCalib::dev_default();
        let salt_subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(2), Fixed::ONE);
        // The being only ever stands on plain ground (no present feature), and is harmed by nothing.
        for _ in 0..10 {
            for obs in feature_observations(false, &[Fixed::ZERO], Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(salt_subject, HARM_ATTR, &params()),
            None,
            "a being that never senses the salt feature never forms a belief about it"
        );
    }

    #[test]
    fn the_run_path_forms_the_belief_and_reads_no_injected_hazard() {
        // A structural guarantee that the retirement is complete (the sibling of the metabolism
        // substrate's identity-blindness check): the runner's canonical path no longer authors a hazard
        // belief off a dose threshold, and it forms the belief through the associative learner instead.
        // A future edit that reaches back for the injected hazard subject fails this build.
        let src = include_str!("runner.rs");
        assert!(
            !src.contains("HAZARD_SUBJECT") && !src.contains("HAZARD_ATTR"),
            "the injected hazard belief is retired: no HAZARD_* constant remains on the canonical path"
        );
        assert!(
            src.contains("feature_observations("),
            "the runner forms the belief through the experiential associative learner"
        );
    }

    #[test]
    fn harm_free_ticks_on_a_feature_self_correct_toward_benign() {
        // Absence self-correction, the falsifier for free: a being that stands on the feature but is not
        // harmed (a tolerant halophile, or the hazard removed) accumulates BENIGN and commits it.
        let calib = HarmLearningCalib::dev_default();
        let features = vec![Fixed::from_int(2)];
        let subject = feature_subject(
            0,
            feature_bucket(Fixed::from_int(2), calib.feature_granularity),
        );
        let mut mind = Mind::new(StableId(3), Fixed::ONE);
        for _ in 0..3 {
            for obs in feature_observations(false, &features, Fixed::ONE, &calib) {
                mind.consider(
                    obs.subject,
                    HARM_ATTR,
                    [HARMS, BENIGN],
                    obs.toward,
                    obs.weight,
                    mind.id,
                );
            }
        }
        assert_eq!(
            mind.belief(subject, HARM_ATTR, &params()),
            Some(BENIGN),
            "harm-free experience on the feature commits BENIGN, so the belief is defeasible"
        );
    }

    fn salt_field(cell: Coord3, dose: Fixed) -> ResourceField {
        let mut f = ResourceField::new();
        let mut toxins = std::collections::BTreeMap::new();
        toxins.insert(SALINITY.to_string(), dose);
        f.set(
            cell,
            crate::edibility::Composition {
                nutrients: std::collections::BTreeMap::new(),
                toxins,
            },
        );
        f
    }

    #[test]
    fn the_avoidance_gradient_points_away_from_a_believed_harmful_cell_and_is_zero_without_the_belief(
    ) {
        // Harm-learning arc slice c, the belief-to-behaviour percept: a being that has learned the salt
        // harms it reads an avoidance gradient pointing AWAY from the believed-harmful ground; a being
        // that has learned nothing reads none. The gradient is a percept the controller weights, so no
        // flight is authored here (that emerges when selection lifts the weight).
        let _ = (HomeostaticRegistry::dev_default(), CONDITION); // the axis the gradient routes into
        let calib = HarmLearningCalib::dev_default();
        let percepts = PerceptRegistry::dev_salinity();
        let here = Coord3::ground(5, 5);
        let salt = Coord3::ground(7, 5); // two tiles due east of the being
        let dose = Fixed::from_int(2);
        let field = salt_field(salt, dose);
        let subject = feature_subject(0, feature_bucket(dose, calib.feature_granularity));

        // A being that believes nothing harmful nearby has no avoidance gradient.
        let mut mind = Mind::new(StableId(1), Fixed::ONE);
        assert_eq!(
            avoidance_gradient(
                &mind,
                here,
                &field,
                &percepts,
                4,
                calib.feature_granularity,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
            "no learned harm nearby, no avoidance gradient"
        );

        // Teach it that the salt harms (commit HARMS about the salt feature-kind).
        for _ in 0..3 {
            mind.consider(
                subject,
                HARM_ATTR,
                [HARMS, BENIGN],
                HARMS,
                calib.observation_weight(),
                mind.id,
            );
        }
        assert_eq!(mind.belief(subject, HARM_ATTR, &params()), Some(HARMS));
        let (gx, gy) = avoidance_gradient(
            &mind,
            here,
            &field,
            &percepts,
            4,
            calib.feature_granularity,
            &params(),
        );
        // The salt is to the EAST (+x), so the avoidance gradient points WEST (-x), away from it, with
        // no north-south component (the salt is due east).
        assert!(
            gx < Fixed::ZERO,
            "the gradient points away from the salt to the east: gx={gx:?}"
        );
        assert_eq!(
            gy,
            Fixed::ZERO,
            "the salt is due east, so no north-south pull"
        );
    }

    #[test]
    fn a_world_with_no_percepts_reads_no_avoidance_gradient() {
        // The opt-out short-circuit: without a declared percept the being senses no feature and forms no
        // avoidance, so the gradient is zero and the run is unchanged.
        let mind = Mind::new(StableId(1), Fixed::ONE);
        let field = salt_field(Coord3::ground(6, 5), Fixed::from_int(2));
        assert_eq!(
            avoidance_gradient(
                &mind,
                Coord3::ground(5, 5),
                &field,
                &PerceptRegistry::empty(),
                4,
                Fixed::ONE,
                &params()
            ),
            (Fixed::ZERO, Fixed::ZERO),
        );
    }
}
