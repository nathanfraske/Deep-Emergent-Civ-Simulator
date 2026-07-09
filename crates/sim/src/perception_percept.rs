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

//! The sensorium-gated magnitude percept (perception-substrate arc, slice 2): a being turns a received
//! signal magnitude into a percept only on a channel its own sensorium senses, through the being's OWN
//! transduction (a physics-floor response law parameterized by the being's data) and its OWN discrimination
//! law, gated by the being's OWN detection threshold. The percept it forms is the transduced physical
//! magnitude and the discrete bucket the being discriminates it into, never a valence: what a signal MEANS
//! to the being is learned receiver-side (slice 3), never stamped here.
//!
//! The derive-vs-author line the slice-2 framing panel drew (5/5 unanimous): the response and discrimination
//! SHAPE is the being's data, never the mechanism. The floor supplies the law FAMILY
//! ([`civsim_physics::laws::ResponseLaw`], [`civsim_physics::laws::DiscriminationLaw`]); the being supplies
//! the selection and the parameters, which segment 3 derives from its genome and anatomy. A linear response,
//! no threshold, and a uniform absolute discrimination step are DEGENERATE DEFAULTS of the family, so a
//! logarithmic, power-law, thresholded, or Weber-scaled sense is a data row rather than a code rewrite
//! (admit-the-alien).
//!
//! Perceptibility is ONE being-derived quantity (the gate's condition 3): the read/not-read gate and the
//! sensitivity are not two mechanisms. A being that carries no transduction for a channel does not sense it
//! (absence is zero sensitivity), and a signal that carries a transduction but does not clear the being's
//! detection threshold forms no percept. The pre-sensorium default-open convention (a being with no
//! sensorium at all senses every channel, so a world that predates the sensorium is unchanged) is a distinct
//! explicit case a caller supplies, not conflated with a declared-but-negligible sense.

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_physics::laws::{self, DiscriminationLaw, ResponseLaw};

use crate::sensorium::SenseChannelId;

/// One channel's transduction as the being's OWN data: the response law and its parameters, the
/// discrimination law and its step, and the detection threshold below which the being forms no percept.
/// Every field is the being's data (Principle 11); the physics floor supplies the law family, the being
/// supplies the selection and the parameters. Segment 3 derives these from the being's genome and anatomy
/// through the same expression machinery that produces its other bodily traits; until then a world declares
/// them as data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ChannelTransduction {
    /// The response law the being's channel transduces a received magnitude by.
    pub response: ResponseLaw,
    /// The response gain (the being's sensitivity on the channel).
    pub gain: Fixed,
    /// The response shape parameter (the Stevens exponent, or the Fechner compression; ignored by
    /// [`ResponseLaw::Linear`]).
    pub shape: Fixed,
    /// The discrimination law the being quantizes the transduced activation by.
    pub discrimination: DiscriminationLaw,
    /// The discrimination step (the just-noticeable difference, absolute or magnitude-relative per the law).
    pub step: Fixed,
    /// The detection threshold on the transduced activation: below it the being forms no percept. This is
    /// the SINGLE perceptibility quantity (condition 3), so the read/not-read gate is not a separate
    /// mechanism: a being that senses the channel but whose signal falls below this threshold perceives
    /// nothing, and a channel a being carries no transduction for is simply not sensed.
    pub threshold: Fixed,
}

/// The percept a being forms of a signal on a channel it senses: the transduced activation (the physical
/// magnitude after the being's own response law) and the discrete bucket the being's discrimination law
/// places it in (the stable key a downstream per-feature belief is minted from, so which signals count as
/// the same perceived kind derives from the being's own sense, never an authored taxonomy). It carries NO
/// valence, category label, or meaning: what the signal MEANS is learned receiver-side (slice 3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MagnitudePercept {
    /// The transduced activation (the physical magnitude after the being's own response).
    pub activation: Fixed,
    /// The discrete bucket the being's discrimination law places the activation in.
    pub bucket: i64,
}

/// Form the percept of a received magnitude through a being's OWN channel transduction, or `None` if the
/// signal does not clear the being's detection threshold (the being senses the channel, but this signal is
/// too faint to perceive). Pure and off the run path (no live caller): the being-percept keystone consumes
/// it, so byte-neutral by construction. The activation is [`civsim_physics::laws::transduce`] under the
/// being's response law and parameters; the bucket is [`civsim_physics::laws::discriminate`] under the
/// being's discrimination law and step. The threshold is the sole perceptibility gate (condition 3).
pub fn sense(
    magnitude: Fixed,
    transduction: &ChannelTransduction,
    activation_max: Fixed,
) -> Option<MagnitudePercept> {
    let activation = laws::transduce(
        magnitude,
        transduction.response,
        transduction.gain,
        transduction.shape,
        activation_max,
    );
    if activation < transduction.threshold {
        return None;
    }
    let bucket = laws::discriminate(activation, transduction.discrimination, transduction.step);
    Some(MagnitudePercept { activation, bucket })
}

/// The set of channel transductions a being carries, keyed by [`SenseChannelId`] in canonical (ascending)
/// order so any walk is reproducible and the store has one representation for one membership. Data-defined
/// and extensible: a being senses a channel the moment it carries a transduction for it. EMPTY by default,
/// so a being that declares no transductions senses only through the pre-sensorium default-open convention
/// a caller supplies (see [`perceive`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TransductionRegistry {
    channels: BTreeMap<SenseChannelId, ChannelTransduction>,
}

impl TransductionRegistry {
    /// An empty registry: the being carries no channel transduction of its own.
    pub fn empty() -> TransductionRegistry {
        TransductionRegistry {
            channels: BTreeMap::new(),
        }
    }

    /// Insert or replace a channel's transduction, keyed by its channel id, so the store stays canonical.
    pub fn insert(&mut self, channel: SenseChannelId, transduction: ChannelTransduction) {
        self.channels.insert(channel, transduction);
    }

    /// The being's transduction for a channel, if it carries one (else the being does not sense the
    /// channel of its own).
    pub fn get(&self, channel: SenseChannelId) -> Option<&ChannelTransduction> {
        self.channels.get(&channel)
    }

    /// Iterate the transductions in canonical (ascending channel id) order.
    pub fn iter(&self) -> impl Iterator<Item = (&SenseChannelId, &ChannelTransduction)> {
        self.channels.iter()
    }

    /// Whether the being carries no channel transduction of its own.
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

/// Form a being's percept on a channel, resolving perceptibility as ONE being-derived quantity (condition
/// 3). The being senses the channel through its own transduction if it carries one, and the percept forms
/// only if the signal clears the being's threshold. `None` in three cases: the being carries a transduction
/// for the channel but the signal is sub-threshold; the being carries some transductions but none for this
/// channel (absence is zero sensitivity, so it does not sense it); or the being carries no transductions at
/// all and no `default_open` is supplied.
///
/// The pre-sensorium default-open convention is a DISTINCT explicit case: a being whose registry is empty
/// senses every channel through the `default_open` transduction a caller passes (so a world that predates
/// the sensorium is unchanged). Passing `None` there means a being with no senses perceives nothing. A being
/// with a NON-empty registry never falls back to the default: its declared senses are its whole sensorium.
pub fn perceive(
    registry: &TransductionRegistry,
    channel: SenseChannelId,
    magnitude: Fixed,
    activation_max: Fixed,
    default_open: Option<&ChannelTransduction>,
) -> Option<MagnitudePercept> {
    let transduction = match registry.get(channel) {
        Some(transduction) => transduction,
        None => {
            if registry.is_empty() {
                default_open?
            } else {
                return None;
            }
        }
    };
    sense(magnitude, transduction, activation_max)
}

// --- Segment 3: deriving a being's transduction from its body, or reserving it fail-loud (condition 2) ---

/// The per-sense transduction parameters that have NO anatomy derivation yet, RESERVED for the owner and
/// surfaced with basis rather than fabricated (Prime Directive 3). The anatomy today derives only the
/// optical GAIN (from the eye's refractive focusing, [`derive_optical_transduction`]); the response SHAPE,
/// the discrimination law and its step, and the detection threshold need a per-channel anatomy-to-sense
/// transduction kernel that is not built (the gate's condition-5 deeper derive target), so a world supplies
/// them as reserved values until that kernel lands. Each carries its basis in the audit log.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReservedSenseParams {
    /// Reserved: the response law a sense organ's transduction follows. Basis: established sensory
    /// psychophysics for the modality (Terran vision is near power-law or logarithmic), read from the
    /// per-channel anatomy-transduction kernel once built.
    pub response: ResponseLaw,
    /// Reserved: the response shape (the Stevens exponent or Fechner compression). Basis: the modality's
    /// measured response curve.
    pub shape: Fixed,
    /// Reserved: the discrimination law. Basis: Weber's law holds across most senses, so the magnitude-
    /// relative step is the usual case, the absolute step the degenerate one.
    pub discrimination: DiscriminationLaw,
    /// Reserved: the discrimination step (the just-noticeable difference). Basis: the sensorium already
    /// carries a per-channel resolution; the step derives from it once the sense's resolution itself
    /// derives from anatomy.
    pub step: Fixed,
    /// Reserved: the detection threshold. Basis: the minimum activation the sense registers, a floor the
    /// anatomy-transduction kernel sets from the organ's noise.
    pub threshold: Fixed,
}

/// The marker that a channel's transduction cannot be produced: its anatomy-to-sense derivation is not
/// built, so the transduction is RESERVED fail-loud. A caller that receives this must not fabricate a
/// transduction (and in particular must not borrow the sense's placeholder optical index, the Terran
/// default condition 2 forbids); the per-channel anatomy-transduction kernel is the deeper build.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TransductionUnbuilt;

/// Derive the OPTICAL channel's transduction from the being's own body: the gain is the being's optical
/// focusing capability (the REFRACT read of its eye's refractive contrast against the medium, a
/// physics-derived sensitivity that is zero for a being with no optical transducer and rises with a keener
/// lens), and the remaining parameters are the reserved per-sense values. This is the one channel whose
/// transduction the anatomy supplies a physics derivation for today (condition 2, condition-5's "wire
/// optical now"); the gain flows from the eye's genome-expressed material, so a being's optical sensitivity
/// arises from its evolved body, never an authored per-being number. Pure and off the run path.
pub fn derive_optical_transduction(
    optical_capability: Fixed,
    reserved: ReservedSenseParams,
) -> ChannelTransduction {
    ChannelTransduction {
        response: reserved.response,
        // DERIVED: the eye's normalized focusing power is its sensitivity on the optical channel.
        gain: optical_capability,
        shape: reserved.shape,
        discrimination: reserved.discrimination,
        step: reserved.step,
        threshold: reserved.threshold,
    }
}

/// Reserve a NON-OPTICAL channel's transduction fail-loud (condition 2). The anatomy-to-sense transduction
/// is optical-only today, so an acoustic, chemical, field, or mana channel has no physics derivation, and
/// its anatomy sense carries a PLACEHOLDER optical index that must not be borrowed as a stand-in sensitivity
/// (the Terran default the value-authoring line forbids). This returns [`TransductionUnbuilt`] rather than a
/// fabricated transduction, so a caller cannot silently produce a wrong non-optical percept. The caller
/// distinguishes an optical channel from a reserved one by the sense's KIND, never by its placeholder index;
/// the per-channel anatomy-transduction kernel that would derive these is the deeper build.
pub fn reserve_nonoptical_transduction() -> Result<ChannelTransduction, TransductionUnbuilt> {
    Err(TransductionUnbuilt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear(gain: Fixed, threshold: Fixed, step: Fixed) -> ChannelTransduction {
        ChannelTransduction {
            response: ResponseLaw::Linear,
            gain,
            shape: Fixed::ZERO,
            discrimination: DiscriminationLaw::AbsoluteStep,
            step,
            threshold,
        }
    }

    const CH: SenseChannelId = SenseChannelId(1);
    const OTHER: SenseChannelId = SenseChannelId(2);

    #[test]
    fn the_linear_absolute_default_is_transduce_then_discriminate() {
        // The degenerate default composes the two floor defaults: a linear response then an absolute
        // bucket, so the percept is exactly `laws::transduce(Linear)` quantized by `laws::discriminate`.
        let cap = Fixed::from_int(1_000_000);
        let step = Fixed::from_ratio(1, 4);
        let t = linear(Fixed::from_int(2), Fixed::ZERO, step);
        let m = Fixed::from_int(5);
        let p = sense(m, &t, cap).expect("clears the zero threshold");
        let activation =
            laws::transduce(m, ResponseLaw::Linear, Fixed::from_int(2), Fixed::ZERO, cap);
        assert_eq!(p.activation, activation);
        assert_eq!(
            p.bucket,
            laws::discriminate(activation, DiscriminationLaw::AbsoluteStep, step)
        );
        // The linear activation is magnitude * gain (byte-identical to the plain product).
        assert_eq!(p.activation, m.mul(Fixed::from_int(2)));
    }

    #[test]
    fn a_sub_threshold_signal_forms_no_percept() {
        // Perceptibility is the single being-derived quantity: a signal whose transduced activation is
        // below the being's detection threshold forms no percept, even though the being senses the channel.
        let cap = Fixed::from_int(1_000_000);
        let t = linear(Fixed::ONE, Fixed::from_int(10), Fixed::ONE);
        assert!(
            sense(Fixed::from_int(4), &t, cap).is_none(),
            "activation 4 is below the threshold 10: no percept"
        );
        let p = sense(Fixed::from_int(20), &t, cap).expect("activation 20 clears threshold 10");
        assert_eq!(p.activation, Fixed::from_int(20));
    }

    #[test]
    fn perceive_absence_of_transduction_is_zero_sensitivity() {
        // A being with a non-empty registry that lacks a channel does NOT sense it (absence is zero
        // sensitivity), and never falls back to the default-open transduction.
        let cap = Fixed::from_int(1_000_000);
        let mut reg = TransductionRegistry::empty();
        reg.insert(CH, linear(Fixed::ONE, Fixed::ZERO, Fixed::ONE));
        let open = linear(Fixed::ONE, Fixed::ZERO, Fixed::ONE);
        assert!(
            perceive(&reg, CH, Fixed::from_int(5), cap, Some(&open)).is_some(),
            "the declared channel is sensed"
        );
        assert!(
            perceive(&reg, OTHER, Fixed::from_int(5), cap, Some(&open)).is_none(),
            "an undeclared channel on a non-empty sensorium is not sensed, no default fallback"
        );
    }

    #[test]
    fn perceive_empty_registry_uses_the_default_open_convention() {
        // A being with an empty registry senses every channel through the default-open transduction (the
        // pre-sensorium convention), and perceives nothing when no default is supplied.
        let cap = Fixed::from_int(1_000_000);
        let reg = TransductionRegistry::empty();
        let open = linear(Fixed::ONE, Fixed::ZERO, Fixed::ONE);
        assert!(
            perceive(&reg, CH, Fixed::from_int(5), cap, Some(&open)).is_some(),
            "an empty sensorium senses every channel through the default-open transduction"
        );
        assert!(
            perceive(&reg, CH, Fixed::from_int(5), cap, None).is_none(),
            "an empty sensorium with no default-open perceives nothing"
        );
    }

    #[test]
    fn the_registry_walks_in_canonical_channel_id_order() {
        let mut reg = TransductionRegistry::empty();
        reg.insert(OTHER, linear(Fixed::ONE, Fixed::ZERO, Fixed::ONE));
        reg.insert(CH, linear(Fixed::ONE, Fixed::ZERO, Fixed::ONE));
        let ids: Vec<u32> = reg.iter().map(|(c, _)| c.0).collect();
        assert_eq!(ids, vec![1, 2], "canonical ascending channel id order");
    }

    #[test]
    fn a_compressive_sense_is_a_data_row_not_a_rewrite() {
        // Admit-the-alien: switching the being's response to a compressive law is a data change on the
        // transduction, no new code path. A log sense forms a smaller activation for a large magnitude than
        // a linear one, so the percept differs by the being's data alone.
        let cap = Fixed::from_int(1_000_000);
        let m = Fixed::from_int(100);
        let lin = ChannelTransduction {
            response: ResponseLaw::Linear,
            gain: Fixed::ONE,
            shape: Fixed::ZERO,
            discrimination: DiscriminationLaw::AbsoluteStep,
            step: Fixed::ONE,
            threshold: Fixed::ZERO,
        };
        let log = ChannelTransduction {
            response: ResponseLaw::LogCompressive,
            shape: Fixed::ONE,
            ..lin
        };
        let pl = sense(m, &lin, cap).unwrap();
        let pg = sense(m, &log, cap).unwrap();
        assert!(
            pg.activation < pl.activation,
            "a compressive (log) sense forms a smaller activation for a large magnitude"
        );
    }

    // --- Segment 3: derivation and reservation ---

    fn reserved() -> ReservedSenseParams {
        ReservedSenseParams {
            response: ResponseLaw::Linear,
            shape: Fixed::ZERO,
            discrimination: DiscriminationLaw::AbsoluteStep,
            step: Fixed::ONE,
            threshold: Fixed::ZERO,
        }
    }

    #[test]
    fn the_optical_gain_derives_from_the_focusing_capability() {
        // The optical channel's gain IS the being's optical focusing capability (the REFRACT read); the
        // remaining parameters are the reserved per-sense values. A keener eye derives a higher gain.
        let dim = derive_optical_transduction(Fixed::from_ratio(1, 4), reserved());
        let keen = derive_optical_transduction(Fixed::from_ratio(3, 4), reserved());
        assert_eq!(
            dim.gain,
            Fixed::from_ratio(1, 4),
            "gain is the focusing capability"
        );
        assert!(keen.gain > dim.gain, "a keener eye derives a higher gain");
        // The reserved parameters pass through unchanged (they are the owner's, not fabricated here).
        assert_eq!(dim.response, ResponseLaw::Linear);
        assert_eq!(dim.discrimination, DiscriminationLaw::AbsoluteStep);
        assert_eq!(dim.step, Fixed::ONE);
    }

    #[test]
    fn a_being_with_no_optical_transducer_derives_zero_gain() {
        // A being whose eye does not focus (zero capability) derives zero optical gain, so it forms no
        // percept on the optical channel: blindness emerges from the anatomy, never an authored flag.
        let t = derive_optical_transduction(Fixed::ZERO, reserved());
        assert_eq!(t.gain, Fixed::ZERO);
        let cap = Fixed::from_int(1_000_000);
        // With a zero gain the activation is zero; a positive threshold then forms no percept.
        let t_thresholded = ChannelTransduction {
            threshold: Fixed::from_ratio(1, 100),
            ..t
        };
        assert!(sense(Fixed::from_int(1000), &t_thresholded, cap).is_none());
    }

    #[test]
    fn a_nonoptical_channel_is_reserved_fail_loud() {
        // Condition 2: a non-optical channel has no anatomy derivation and its placeholder optical index
        // must not be borrowed, so it is reserved fail-loud rather than fabricated.
        assert_eq!(reserve_nonoptical_transduction(), Err(TransductionUnbuilt));
    }
}
