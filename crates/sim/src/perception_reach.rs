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

use civsim_core::Fixed;
use civsim_physics::laws;
use civsim_world::Coord3;

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

/// The reach of a signal to a perceiver: the geometrically-spread magnitude and the accumulated path
/// optical depth. Both are reported and the `exp(-tau)` transmission transform is DEFERRED (the codebase's
/// report-the-indicator, defer-the-transcendental convention, as [`civsim_physics::laws::optical_depth`]
/// itself defers `exp`): a consumer applies its own perception threshold, and a large optical depth is an
/// occluded, blocked signal. Occlusion emerges from a strongly-absorbing medium along the path, never an
/// authored line-of-sight rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Reach {
    /// The geometrically-spread received magnitude before medium attenuation.
    pub spread: Fixed,
    /// The accumulated optical depth of the medium along the path (larger is more attenuated; the
    /// occlusion limit).
    pub optical_depth: Fixed,
}

/// The spreading dimensionality a signal propagates through, READ from the world's own spatial-axis
/// count rather than an authored constant: the world coordinate is [`Coord3`], a packed tuple of `i32`
/// spatial axes, so its size in `i32` units IS the number of axes an unconfined signal spreads in
/// (three today), and a differently-dimensioned coordinate would follow automatically. The engine models
/// one unconfined bulk medium (no confinement or waveguide substrate), so an unconfined signal spreads in
/// all its axes: D = 3.
///
/// Because D = 3 for every path today, the general kernel does not yet bite: [`received_reach`] spreads
/// exactly as `inverse_square_falloff` (the general kernel reproduces it at D=3), so the four run_world
/// pins hold and nothing existing moves. The generality becomes real only when a medium-confinement
/// substrate lands and sets D below 3 for a surface-guided (D=2) or ducted (D=1) signal; that substrate
/// is the flagged sibling refinement, and [`civsim_physics::laws::geometric_spread`] already handles
/// D=2 and D=1 for when it does. Reading D structurally here, not as a per-channel `3`, keeps the
/// derivation explicit and the confinement follow-on a data-and-substrate change rather than an edit here.
pub const fn spreading_dimensionality() -> u32 {
    // The number of i32 spatial axes the coordinate carries; the effective confined dimensionality of the
    // traversed medium would be read here instead once a confinement substrate exists.
    (core::mem::size_of::<Coord3>() / core::mem::size_of::<i32>()) as u32
}

// The structural read above is a byte-size axis count, correct only while Coord3 is a packed tuple of i32
// spatial axes. This compile-time assertion turns a shape change (a non-i32 field, an added axis, or
// padding) into a BUILD ERROR rather than a silently-wrong dimensionality (gate hardening note): whoever
// changes Coord3 is forced to update the derivation here.
const _: () = assert!(
    core::mem::size_of::<Coord3>() == 3 * core::mem::size_of::<i32>(),
    "Coord3 must be three packed i32 axes for the structural spreading-dimensionality read; \
     update spreading_dimensionality() if its shape changes",
);

/// The largest 3D squared separation the distance is formed exactly for; a source farther than this is
/// negligible (its geometric spread underflows to zero), which keeps the fixed-point square root inside
/// its representable range without a fabricated cutoff (the physics already sends a far source to zero).
const MAX_REPRESENTABLE_SEP2: i128 = 1 << 30;

/// The received reach of a signal from a source location to a perceiver location, given the emitted power,
/// the geometric sphere coefficient for the derived dimensionality, and the medium absorption samples
/// along the path. Pure and OFF the run path (no live caller): the being-percept keystone consumes it, so
/// this is byte-neutral by construction. The separation is the 3D Euclidean distance over the FULL
/// [`Coord3`] (all three axes, the vertical `z` included, unlike the 2D horizontal index metric); the
/// spreading is the general [`civsim_physics::laws::geometric_spread`] kernel at the derived
/// dimensionality; and the path attenuation accumulates the medium's OWN absorption (each
/// `(coefficient, length)` sample read from the medium, never a per-channel or per-medium-label constant),
/// so occlusion emerges from the strata rather than an authored line-of-sight rule.
pub fn received_reach(
    emitted_power: Fixed,
    source: Coord3,
    perceiver: Coord3,
    sphere_coeff: Fixed,
    irrad_max: Fixed,
    absorption_samples: &[(Fixed, Fixed)],
    tau_max: Fixed,
) -> Reach {
    let dx = source.x as i128 - perceiver.x as i128;
    let dy = source.y as i128 - perceiver.y as i128;
    let dz = source.z as i128 - perceiver.z as i128;
    let sep2 = dx * dx + dy * dy + dz * dz;
    let spread = if sep2 > MAX_REPRESENTABLE_SEP2 {
        // A source beyond the representable separation is negligible: its geometric spread underflows.
        Fixed::ZERO
    } else {
        // r is the square root of the 3D squared separation, exact on perfect squares and deterministic
        // otherwise; sep2 is bounded above by MAX_REPRESENTABLE_SEP2 < i32::MAX, so the cast is lossless.
        let r = Fixed::from_int(sep2 as i32).sqrt();
        laws::geometric_spread(
            emitted_power,
            r,
            spreading_dimensionality(),
            sphere_coeff,
            irrad_max,
        )
    };
    // Accumulate the path optical depth from the medium's own absorption samples, each capped, the total
    // capped: a strongly-absorbing medium drives the depth to the cap, the occlusion limit.
    let mut optical_depth = Fixed::ZERO;
    for &(coefficient, length) in absorption_samples {
        let segment = laws::optical_depth(coefficient, length, tau_max);
        optical_depth = (optical_depth + segment).min(tau_max);
    }
    Reach {
        spread,
        optical_depth,
    }
}

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

    // --- The reach read ---

    fn four_pi() -> Fixed {
        Fixed::from_ratio(62_832, 5_000)
    }

    #[test]
    fn received_reach_spreads_by_the_general_kernel_over_the_3d_separation() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 3, y: 4, z: 0 }; // sep2 = 9 + 16 = 25, r = 5
        let reach = received_reach(power, src, per, four_pi(), cap, &[], Fixed::from_int(1000));
        let r = Fixed::from_int(25).sqrt();
        assert_eq!(
            reach.spread,
            laws::geometric_spread(power, r, 3, four_pi(), cap),
            "the reach spreads by the general kernel at the derived dimensionality (3)",
        );
        assert_eq!(
            reach.optical_depth,
            Fixed::ZERO,
            "no absorption samples means no path attenuation",
        );
    }

    #[test]
    fn received_reach_includes_the_vertical_z_axis() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let tau = Fixed::from_int(1000);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        // A purely-vertical separation reaches like an equal-magnitude horizontal one: z is a real axis.
        let horizontal = received_reach(
            power,
            src,
            Coord3 { x: 5, y: 0, z: 0 },
            four_pi(),
            cap,
            &[],
            tau,
        );
        let vertical = received_reach(
            power,
            src,
            Coord3 { x: 0, y: 0, z: 5 },
            four_pi(),
            cap,
            &[],
            tau,
        );
        assert_eq!(horizontal.spread, vertical.spread);
        // A vertical offset changes the reach; the 2D horizontal index metric would drop it.
        let above = received_reach(
            power,
            src,
            Coord3 { x: 3, y: 0, z: 4 },
            four_pi(),
            cap,
            &[],
            tau,
        ); // sep2 = 25
        let flat = received_reach(
            power,
            src,
            Coord3 { x: 3, y: 0, z: 0 },
            four_pi(),
            cap,
            &[],
            tau,
        ); // sep2 = 9
        assert_ne!(
            above.spread, flat.spread,
            "a vertical offset changes the reach; z is not dropped"
        );
    }

    #[test]
    fn received_reach_occlusion_emerges_from_a_strongly_absorbing_medium() {
        let cap = Fixed::from_int(1_000_000);
        let tau_max = Fixed::from_int(10);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 10, y: 0, z: 0 };
        // A strongly-absorbing medium drives the accumulated optical depth to the cap: the occlusion
        // limit, emerging from the medium data, with no authored line-of-sight rule.
        let blocked = received_reach(
            power,
            src,
            per,
            four_pi(),
            cap,
            &[(Fixed::from_int(100), Fixed::from_int(10))],
            tau_max,
        );
        assert_eq!(
            blocked.optical_depth, tau_max,
            "a strongly-absorbing medium occludes"
        );
        // A transparent medium (zero absorption coefficient) does not attenuate.
        let clear = received_reach(
            power,
            src,
            per,
            four_pi(),
            cap,
            &[(Fixed::ZERO, Fixed::from_int(10))],
            tau_max,
        );
        assert_eq!(clear.optical_depth, Fixed::ZERO);
    }

    #[test]
    fn received_reach_at_zero_separation_reads_the_geometric_cap() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let c = Coord3 { x: 7, y: -2, z: 1 };
        let reach = received_reach(power, c, c, four_pi(), cap, &[], Fixed::from_int(1000));
        assert_eq!(
            reach.spread, cap,
            "a co-located source reads the cap (zero distance)"
        );
    }

    #[test]
    fn received_reach_a_far_source_is_negligible() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        // A separation whose square exceeds the representable bound is negligible (spread underflows).
        let far = Coord3 {
            x: 100_000,
            y: 0,
            z: 0,
        };
        let reach = received_reach(power, src, far, four_pi(), cap, &[], Fixed::from_int(1000));
        assert_eq!(reach.spread, Fixed::ZERO);
    }

    #[test]
    fn spreading_dimensionality_reads_the_coordinate_axis_count() {
        // Read structurally from Coord3 (three i32 axes), not authored: a differently-dimensioned
        // coordinate would change this without an edit here.
        assert_eq!(spreading_dimensionality(), 3);
    }

    #[test]
    fn received_reach_is_byte_identical_to_inverse_square_today() {
        // With D = 3 everywhere (no confinement substrate), the general kernel reproduces inverse-square
        // exactly, so the reach spreads bit-for-bit as inverse_square_falloff and the pins hold.
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 6, y: 8, z: 0 }; // sep2 = 100, r = 10
        let reach = received_reach(power, src, per, four_pi(), cap, &[], Fixed::from_int(1000));
        let r = Fixed::from_int(100).sqrt();
        assert_eq!(
            reach.spread,
            laws::inverse_square_falloff(power, r, four_pi(), cap),
            "at D=3 the reach spread equals inverse_square_falloff (pins hold)",
        );
    }
}
