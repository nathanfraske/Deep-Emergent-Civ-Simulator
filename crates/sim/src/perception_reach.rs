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

//! The channel reach registry (perception-substrate arc, slice 1, segment 2): the data-defined binding
//! from a sense channel to the physics-floor law and axes its signal reaches by. It is the
//! harden-to-registry sibling of the value substrate ([`crate::value`]), the semantic substrate, and
//! the percept substrate ([`crate::percept`]): the kernel SET is fixed Rust (the mechanism), the
//! membership (which channels exist and which law and axes each reaches by) is data and grows with the
//! world (Principle 11).
//!
//! The reach read (segment 3) never branches on channel identity. It looks up channel c's row and
//! dispatches on the NAMED kernel id the row carries, so a mana-field, redox-gradient, or seismic
//! channel is added by inserting a row (and, where a genuinely new law is needed, one kernel on the
//! floor), never by editing a `match channel { Optical => ..., Acoustic => ... }`. That closed-enum
//! dispatch on channel identity is the Principle-8/11 defect the slice-1 framing panel caught, and this
//! registry is its fix.
//!
//! What this registry does NOT carry is the propagation GEOMETRY: the reach spreads by the general
//! [`civsim_physics::laws::geometric_spread`] kernel whose dimensionality DERIVES from the geometry of
//! the space the signal traverses (segment 3), never a per-channel constant. A channel whose
//! propagation is not geometric at all (a network, gradient, or threshold field) needs the deeper
//! data-expressible propagation-law form, the flagged sibling substrate, not built here.

use std::collections::BTreeMap;

use crate::sensorium::SenseChannelId;

/// The spreading law a channel's signal propagates by. The kernel SET is fixed Rust code (the
/// mechanism); which law a channel uses is data (the registry row). Today only [`SpreadKernel::Geometric`]
/// is built: the general geometric-spread law parameterized by the dimensionality derived from the path.
/// A non-geometric propagation is the flagged deeper substrate, so a new VARIANT here is a deliberate
/// floor extension, never an authored per-channel branch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SpreadKernel {
    /// Geometric spreading over the surface of a `(D-1)`-sphere, `E = power / (sphere_coeff * d^(D-1))`
    /// ([`civsim_physics::laws::geometric_spread`]); the dimensionality derives from the traversed
    /// geometry.
    Geometric,
}

/// One channel's reach binding as data: the law its signal spreads by, the physics-floor axis its
/// emitted power reads, the medium axis its path absorption reads, and whether that absorption
/// coefficient forms from the signal's frequency. Every field is data (Principle 11); the reach read is
/// fixed Rust that consumes this row. The axes are floor axis id strings (for example `opt.source_power`,
/// `opt.absorption_coefficient`, `acoustic.source_power`), the same string-keyed floor reference the
/// percept and material substrates use, so the floor stays the one authored place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelReach {
    /// The channel this row binds.
    pub channel: SenseChannelId,
    /// The spreading law the channel propagates by (dispatched by this id, never by channel identity).
    pub spreading: SpreadKernel,
    /// The physics-floor axis id the channel's emitted power reads (for example `opt.source_power`).
    pub source_power_axis: String,
    /// The physics-floor medium axis id the channel's path absorption reads (for example
    /// `opt.absorption_coefficient`), sampled along the traversed segment so occlusion emerges from the
    /// medium rather than an authored line-of-sight rule.
    pub absorption_axis: String,
    /// Whether the absorption coefficient forms from the signal's frequency (the acoustic case, via
    /// [`civsim_physics::laws::acoustic_absorption`]) rather than being read from the medium directly.
    /// The frequency itself derives from the emitter's own body resonance (segment 3), never an authored
    /// channel constant.
    pub frequency_dependent: bool,
}

/// The set of channel reach bindings a world runs, keyed by [`SenseChannelId`] in canonical (ascending)
/// order so any walk is reproducible and the registry has one representation for one membership.
/// Data-defined and extensible: a new channel is covered the moment it registers its row. EMPTY by
/// default, so a world that declares no reach bindings leaves every run hash unchanged (the reach read
/// is opt-in).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelReachRegistry {
    channels: BTreeMap<SenseChannelId, ChannelReach>,
}

impl ChannelReachRegistry {
    /// An empty registry: no channel reaches, so no reach read fires and the run is bit-identical to a
    /// world without the reach substrate. The default and the opt-out.
    pub fn empty() -> ChannelReachRegistry {
        ChannelReachRegistry {
            channels: BTreeMap::new(),
        }
    }

    /// Insert or replace a channel's reach binding, keyed by its own channel id, so the store stays
    /// canonical.
    pub fn insert(&mut self, reach: ChannelReach) {
        self.channels.insert(reach.channel, reach);
    }

    /// The reach binding for a channel, if one is registered. The reach read dispatches on the returned
    /// row's kernel id, never on the channel id itself.
    pub fn get(&self, channel: SenseChannelId) -> Option<&ChannelReach> {
        self.channels.get(&channel)
    }

    /// Iterate the rows in canonical (ascending channel id) order.
    pub fn iter(&self) -> impl Iterator<Item = (&SenseChannelId, &ChannelReach)> {
        self.channels.iter()
    }

    /// Whether the registry declares no channel (the opt-out: no reach read fires).
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// A labelled DEVELOPMENT FIXTURE: the two channels the physics floor already carries source-power
    /// axes for, an optical channel and an acoustic channel. Both spread by the same general geometric
    /// law (the dimensionality derives from the path, not the channel); the optical absorption reads
    /// the medium axis directly, the acoustic absorption forms from the signal frequency. Not owner
    /// data; the minimum a reach run needs to exercise the two floor source-power axes. The real
    /// channel set is the world's data.
    pub fn dev_terran() -> ChannelReachRegistry {
        let mut reg = ChannelReachRegistry::empty();
        reg.insert(ChannelReach {
            channel: DEV_OPTICAL,
            spreading: SpreadKernel::Geometric,
            source_power_axis: "opt.source_power".to_string(),
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: false,
        });
        reg.insert(ChannelReach {
            channel: DEV_ACOUSTIC,
            spreading: SpreadKernel::Geometric,
            source_power_axis: "acoustic.source_power".to_string(),
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: true,
        });
        reg
    }
}

/// The optical dev-fixture channel (a leaf id, not special-cased in any mechanism).
pub const DEV_OPTICAL: SenseChannelId = SenseChannelId(1);
/// The acoustic dev-fixture channel.
pub const DEV_ACOUSTIC: SenseChannelId = SenseChannelId(2);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_is_the_opt_out() {
        let reg = ChannelReachRegistry::empty();
        assert!(reg.is_empty());
        assert!(reg.get(DEV_OPTICAL).is_none());
    }

    #[test]
    fn a_channel_reach_is_looked_up_by_id_and_carries_its_law_and_axes_as_data() {
        let reg = ChannelReachRegistry::dev_terran();
        // Dispatch is by channel id into the registry, then by the row's kernel id: never a code branch
        // on channel identity.
        let optical = reg.get(DEV_OPTICAL).expect("optical row present");
        assert_eq!(optical.spreading, SpreadKernel::Geometric);
        assert_eq!(optical.source_power_axis, "opt.source_power");
        assert!(!optical.frequency_dependent);
        let acoustic = reg.get(DEV_ACOUSTIC).expect("acoustic row present");
        assert_eq!(acoustic.source_power_axis, "acoustic.source_power");
        assert!(
            acoustic.frequency_dependent,
            "the acoustic absorption forms from the signal frequency",
        );
        assert!(reg.get(SenseChannelId(99)).is_none());
    }

    #[test]
    fn the_registry_walks_in_canonical_channel_id_order() {
        let reg = ChannelReachRegistry::dev_terran();
        let ids: Vec<u32> = reg.iter().map(|(c, _)| c.0).collect();
        assert_eq!(ids, vec![1, 2], "canonical ascending channel id order");
    }

    #[test]
    fn a_later_insert_replaces_a_row_keyed_by_channel() {
        let mut reg = ChannelReachRegistry::empty();
        reg.insert(ChannelReach {
            channel: DEV_OPTICAL,
            spreading: SpreadKernel::Geometric,
            source_power_axis: "opt.source_power".to_string(),
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: false,
        });
        reg.insert(ChannelReach {
            channel: DEV_OPTICAL,
            spreading: SpreadKernel::Geometric,
            source_power_axis: "opt.source_power".to_string(),
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: true,
        });
        assert_eq!(reg.iter().count(), 1, "one row per channel id");
        assert!(reg.get(DEV_OPTICAL).unwrap().frequency_dependent);
    }
}
