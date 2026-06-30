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
//! [`SenseChannelId`], a being carries a [`Sensorium`] of the channels it reads and its acuity
//! per channel, and perception requires the being to read the trace's channel, so a being
//! blind to a channel never perceives a trace on it and an alien sense is a point in the same
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

/// A being's sensorium: the channels it reads and its acuity on each, the heritable and
/// damageable set the gap describes (here a data set; deriving it from the genome and anatomy
/// and clamping it under injury are follow-ons). A being with an empty sensorium that is never
/// installed falls back to reading every channel, so perception is gated only where a sensorium
/// is declared.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Sensorium {
    channels: BTreeMap<SenseChannelId, Fixed>,
}

impl Sensorium {
    /// An empty sensorium (reads no channel; a being that reads nothing perceives nothing once
    /// a sensorium is installed).
    pub fn new() -> Self {
        Sensorium::default()
    }

    /// A sensorium over channel-acuity pairs.
    pub fn with(pairs: impl IntoIterator<Item = (SenseChannelId, Fixed)>) -> Self {
        Sensorium {
            channels: pairs.into_iter().collect(),
        }
    }

    /// Grant or update a channel's acuity.
    pub fn grant(&mut self, channel: SenseChannelId, acuity: Fixed) {
        self.channels.insert(channel, acuity);
    }

    /// The acuity with which this sensorium reads a channel, or `None` if it cannot read it.
    pub fn reads(&self, channel: SenseChannelId) -> Option<Fixed> {
        self.channels.get(&channel).copied()
    }

    /// The channels this sensorium reads, in canonical order.
    pub fn channels(&self) -> impl Iterator<Item = (SenseChannelId, Fixed)> + '_ {
        self.channels.iter().map(|(&c, &a)| (c, a))
    }

    /// Whether the sensorium reads no channel.
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
}
