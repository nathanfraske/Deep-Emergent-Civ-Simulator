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

//! The sensorium: data-driven, channel-gated perception (the R-SENSORIUM gap, the buildable
//! half).
//!
//! The deeper move the gap names is to make perception fully data-driven: a sense is a reader
//! of a physical channel, an event carries a channel signature, and a being perceives the
//! event only when one of its senses reads that channel above threshold. This brick builds
//! that channel gate over the existing place-based witnessing: a trace carries a
//! [`SenseChannelId`], a being carries a [`Sensorium`] of the channels it reads and, per channel,
//! two distinct quantities (an acuity gate and a discrimination resolution, the R-SENSORIUM split
//! documented on [`Sensorium`]), and perception requires the being to read the trace's channel, so a
//! being blind to a channel never perceives a trace on it and an alien sense is a point in the same
//! data space rather than a new code path.
//!
//! What this brick does not build is spatial propagation: the sim places beings and traces by
//! an abstract place tag, not a coordinate, so a stimulus carrying a distance and attenuating
//! with range has nothing to compute over yet. The reach-and-attenuation half waits on a
//! coordinate and field model (the physics-substrate medium and field set), the named
//! follow-on; this brick is the channel reader. The channel set itself is the physics-channel
//! substrate, so a novel sense is a deliberate physics extension, never an authored sense enum.

use std::collections::BTreeMap;

use civsim_core::Fixed;

/// A data-defined perception channel: a physical channel a sense reads (sight, sound, scent, a
/// thermal or mana channel, a channel no one else reads). A registry id rather than a closed
/// enum, drawn from the physics-channel substrate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SenseChannelId(pub u32);

impl SenseChannelId {
    /// The default channel a trace carries when none is specified, and the one a being with no
    /// declared sensorium implicitly reads (so existing place-based perception is unchanged).
    pub const DEFAULT: SenseChannelId = SenseChannelId(0);
}

/// A being's sensorium: per channel, two DISTINCT perceptual quantities the gap describes (here a
/// data set; deriving them from the genome and anatomy and clamping them under injury are
/// follow-ons). ACUITY is the sensitivity gate in `[0, ONE]` (higher is a keener sense, absent is
/// blind); RESOLUTION is the discrimination sharpness as a just-noticeable difference on the
/// channel's own physical scale (for a voice channel, a frequency difference in hertz), where a
/// SMALLER value is a sharper sense. These are different physical quantities (sensitivity versus
/// discrimination), so they are sibling maps rather than one number: a keen-but-coarse sense (high
/// acuity, large JND) and a faint-but-sharp one (low acuity, small JND) are both expressible, the
/// perception gate reads the acuity while the phonetic contrast geometry reads the resolution, and
/// neither reader can corrupt the other's quantity (the R-SENSORIUM acuity/resolution split; before
/// it, one field carried both and a valid acuity of one read as an implausibly sharp one-hertz JND).
/// A being with an empty sensorium that is never installed falls back to reading every channel, so
/// perception is gated only where a sensorium is declared.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Sensorium {
    /// Per-channel acuity in `[0, ONE]`: the sensitivity gate. Read by the perception gate (the
    /// perceive beat of `civsim_sim::world::World`) and the production-perception Liebig half
    /// (`civsim_sim::langmod::capability_halves`). A channel absent here is one the being does not
    /// perceive.
    channels: BTreeMap<SenseChannelId, Fixed>,
    /// Per-channel resolution: the just-noticeable difference on the channel's physical scale
    /// (smaller is sharper). Read by `civsim_sim::langmod::perceptual_geometry` to set the phonetic
    /// contrast budget and confusability. A distinct physical quantity from acuity, keyed by the same
    /// channel; absent for a channel whose discrimination is unspecified.
    resolutions: BTreeMap<SenseChannelId, Fixed>,
}

impl Sensorium {
    /// An empty sensorium (reads no channel; a being that reads nothing perceives nothing once
    /// a sensorium is installed).
    pub fn new() -> Self {
        Sensorium::default()
    }

    /// A sensorium over channel-acuity pairs (the sensitivity gate). Its resolution map starts empty;
    /// add per-channel just-noticeable differences with [`Sensorium::and_resolution`] or
    /// [`Sensorium::grant_resolution`].
    pub fn with(pairs: impl IntoIterator<Item = (SenseChannelId, Fixed)>) -> Self {
        Sensorium {
            channels: pairs.into_iter().collect(),
            resolutions: BTreeMap::new(),
        }
    }

    /// A sensorium over channel-resolution (just-noticeable-difference) pairs, its acuity map empty:
    /// the discrimination side alone, for a reader (like `civsim_sim::langmod::perceptual_geometry`) that
    /// consumes only the resolution.
    pub fn with_resolution(pairs: impl IntoIterator<Item = (SenseChannelId, Fixed)>) -> Self {
        Sensorium {
            channels: BTreeMap::new(),
            resolutions: pairs.into_iter().collect(),
        }
    }

    /// Add per-channel resolutions to a sensorium (a builder over [`Sensorium::with`]), so one
    /// sensorium carries both an acuity gate and a discrimination resolution per channel.
    pub fn and_resolution(
        mut self,
        pairs: impl IntoIterator<Item = (SenseChannelId, Fixed)>,
    ) -> Self {
        self.resolutions.extend(pairs);
        self
    }

    /// Grant or update a channel's acuity (the sensitivity gate).
    pub fn grant(&mut self, channel: SenseChannelId, acuity: Fixed) {
        self.channels.insert(channel, acuity);
    }

    /// Grant or update a channel's resolution (the just-noticeable difference; smaller is sharper).
    pub fn grant_resolution(&mut self, channel: SenseChannelId, jnd: Fixed) {
        self.resolutions.insert(channel, jnd);
    }

    /// The acuity with which this sensorium reads a channel (the sensitivity gate), or `None` if it
    /// cannot read it.
    pub fn reads(&self, channel: SenseChannelId) -> Option<Fixed> {
        self.channels.get(&channel).copied()
    }

    /// The resolution (just-noticeable difference) this sensorium discriminates a channel at, or
    /// `None` if none is set. Distinct from [`Sensorium::reads`]: a smaller value is a sharper sense,
    /// on the channel's own physical scale rather than the `[0, ONE]` acuity scale.
    pub fn resolution(&self, channel: SenseChannelId) -> Option<Fixed> {
        self.resolutions.get(&channel).copied()
    }

    /// The channels this sensorium reads (by acuity), in canonical order.
    pub fn channels(&self) -> impl Iterator<Item = (SenseChannelId, Fixed)> + '_ {
        self.channels.iter().map(|(&c, &a)| (c, a))
    }

    /// Whether the sensorium reads no channel (no acuity gate installed).
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIGHT: SenseChannelId = SenseChannelId(1);
    const SCENT: SenseChannelId = SenseChannelId(2);

    #[test]
    fn a_sensorium_reads_only_its_channels() {
        let s = Sensorium::with([(SIGHT, Fixed::ONE), (SCENT, Fixed::from_ratio(1, 2))]);
        assert_eq!(s.reads(SIGHT), Some(Fixed::ONE));
        assert_eq!(s.reads(SCENT), Some(Fixed::from_ratio(1, 2)));
        assert_eq!(
            s.reads(SenseChannelId(99)),
            None,
            "blind to an unread channel"
        );
    }

    #[test]
    fn acuity_and_resolution_are_independent_quantities_on_one_channel() {
        // The R-SENSORIUM acuity/resolution split (WP5): one channel carries a `[0, ONE]` acuity and a
        // Hz-scale just-noticeable difference at once, each read through its own accessor, so a valid
        // acuity of one no longer reads as an implausibly sharp one-hertz JND (the latent conflation).
        let jnd = Fixed::from_int(20);
        let s = Sensorium::with([(SIGHT, Fixed::ONE)]).and_resolution([(SIGHT, jnd)]);
        assert_eq!(s.reads(SIGHT), Some(Fixed::ONE), "the acuity gate is one");
        assert_eq!(
            s.resolution(SIGHT),
            Some(jnd),
            "the resolution is twenty, not one"
        );
        assert_ne!(
            s.reads(SIGHT),
            s.resolution(SIGHT),
            "the two quantities are read from separate fields, so one does not become the other"
        );

        // The two sides gate independently: a channel can carry an acuity but no resolution (perceived
        // but with unspecified discrimination) and the reverse.
        let acuity_only = Sensorium::with([(SCENT, Fixed::from_ratio(1, 2))]);
        assert_eq!(acuity_only.reads(SCENT), Some(Fixed::from_ratio(1, 2)));
        assert_eq!(acuity_only.resolution(SCENT), None, "no resolution set");
        let resolution_only = Sensorium::with_resolution([(SCENT, jnd)]);
        assert_eq!(resolution_only.resolution(SCENT), Some(jnd));
        assert_eq!(resolution_only.reads(SCENT), None, "no acuity gate set");
    }
}
