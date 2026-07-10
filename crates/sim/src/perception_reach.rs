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
use civsim_physics::{laws, PhysicsRegistry};
use civsim_world::Coord3;

use crate::material::MaterialField;
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
    /// Whether the absorption coefficient forms from the signal's frequency (the acoustic case, where the
    /// coefficient is [`civsim_physics::laws::acoustic_absorption`] of a frequency that derives from the
    /// emitter's own body resonance) rather than being read from the medium directly. This
    /// frequency-dependent absorption path is a RESERVED follow-on: neither the emitter body-resonance
    /// frequency source nor the acoustic-law application is wired in slice 1. Until that segment lands the
    /// field is honoured fail-loud, not silently: [`resolve_reach`] asserts a row is not
    /// `frequency_dependent`, so a row that declares it fails loudly rather than reading the medium axis as
    /// if the channel were frequency-independent (Prime Directive 3, reserved fail-loud). No shipped row
    /// sets it until the frequency source and the acoustic-law application are built.
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
    /// law (the dimensionality derives from the path, not the channel), and both read their path
    /// absorption directly from the medium axis. Not owner data; the minimum a reach run needs to
    /// exercise the two floor source-power axes. The real channel set is the world's data.
    ///
    /// Two honest fixture limits. First, both rows set `frequency_dependent: false`: the
    /// frequency-dependent acoustic absorption path (the coefficient forming from the signal frequency) is
    /// a reserved follow-on not wired in slice 1, and a fixture that shipped `frequency_dependent: true`
    /// would trip the [`resolve_reach`] fail-loud assert, so the fixture stays resolvable. Second, the
    /// acoustic row reuses the OPTICAL absorption axis (`opt.absorption_coefficient`): the physics floor
    /// carries no acoustic absorption axis yet, so this dev channel stands in with the one absorption axis
    /// the floor has. The real acoustic channel binds its own acoustic absorption axis once the floor
    /// grows one (a flagged floor gap, not a slice-1 deliverable).
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
            // The floor carries no acoustic absorption axis yet; this dev channel reuses the optical one
            // as a labelled stand-in (a flagged floor gap). A real acoustic channel binds its own axis.
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: false,
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
/// (three today). The read is `size_of::<Coord3>() / size_of::<i32>()`, so it tracks the coordinate's own
/// axis count rather than a literal `3`. The compile-time assert below pins the packed-`i32` ASSUMPTION
/// the read depends on (that the coordinate is N packed `i32` axes with no padding or non-`i32` field): if
/// the coordinate's shape changes, the assert makes it a build error, a deliberate re-check of this
/// derivation rather than a silent miscount. A pure axis-count change (an added spatial axis) would then
/// be a one-line assert bump, not a rewrite of the perception path. The engine models one unconfined bulk
/// medium (no confinement or waveguide substrate), so an unconfined signal spreads in all its axes: D = 3.
///
/// Because D = 3 for every path today, the general kernel does not yet bite: [`received_reach`] spreads
/// exactly as `inverse_square_falloff` (the general kernel reproduces it at D=3). The generality becomes
/// real only when a medium-confinement substrate lands and sets D below 3 for a surface-guided (D=2) or
/// ducted (D=1) signal; that substrate is the flagged sibling refinement, and
/// [`civsim_physics::laws::geometric_spread`] already handles D=2 and D=1 for when it does. Reading D
/// structurally here, not as a per-channel `3`, keeps the derivation explicit and the confinement
/// follow-on a data-and-substrate change rather than an edit here.
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

/// The largest 3D squared separation the distance is formed for, DERIVED from the fixed-point cast bound
/// rather than authored: the squared separation is cast to `i32` before [`Fixed::from_int`], so its
/// representable ceiling is `i32::MAX`. This is a numerical representability guard, not a perception
/// horizon: at the D=3 spreading the whole engine runs today, the geometric-spread denominator
/// (`sphere_coeff * distance^2`) already overflows its own `checked_mul` and returns zero far below this
/// ceiling (empirically around a squared separation of 2^28 at `sphere_coeff = 4*pi`), so clamping a
/// larger separation to zero here changes no D=3 result: the physics has already sent that source to zero.
/// A confinement substrate that lets D fall below 3 (a ducted D=1 signal does not attenuate geometrically)
/// would revisit this bound alongside that substrate; it is not yet reached, since D=3 for every path.
const MAX_REPRESENTABLE_SEP2: i128 = i32::MAX as i128;

/// The world-physics caps a reach computation reads beyond the source, perceiver, emitted power, and
/// medium samples: the geometric sphere coefficient for the derived dimensionality, the irradiance cap the
/// geometric spread saturates at, and the optical-depth cap the accumulated medium attenuation saturates
/// at (the occlusion limit). Grouped into one value so a reach call passes a single physics-caps argument
/// rather than three same-typed `Fixed` caps that a caller could transpose. Each field is a physics-floor
/// datum the caller supplies (Principle 11); nothing here is authored by the reach code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReachBounds {
    /// The geometric-spread sphere coefficient for the derived dimensionality (for example `4*pi` at
    /// D=3), the surface term the emitted power divides across.
    pub sphere_coeff: Fixed,
    /// The irradiance cap the geometric spread saturates at (a co-located source reads this).
    pub irrad_max: Fixed,
    /// The optical-depth cap the accumulated medium attenuation saturates at (the occlusion limit).
    pub tau_max: Fixed,
}

/// The received reach of a signal from a source location to a perceiver location, given the emitted power,
/// the world-physics [`ReachBounds`], and the medium absorption samples along the path. Pure and OFF the
/// run path (no live caller): the being-percept keystone consumes it, so this is byte-neutral by
/// construction. The separation is the 3D Euclidean distance over the FULL [`Coord3`] (all three axes, the
/// vertical `z` included, unlike the 2D horizontal index metric); the spreading is the general
/// [`civsim_physics::laws::geometric_spread`] kernel at the derived dimensionality; and the path
/// attenuation accumulates the medium's OWN absorption (each `(coefficient, length)` sample read from the
/// medium, never a per-channel or per-medium-label constant), so occlusion emerges from the strata rather
/// than an authored line-of-sight rule.
pub fn received_reach(
    emitted_power: Fixed,
    source: Coord3,
    perceiver: Coord3,
    bounds: ReachBounds,
    absorption_samples: &[(Fixed, Fixed)],
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
            bounds.sphere_coeff,
            bounds.irrad_max,
        )
    };
    // Accumulate the path optical depth from the medium's own absorption samples, each capped, the total
    // capped: a strongly-absorbing medium drives the depth to the cap, the occlusion limit. The running
    // sum uses `saturating_add` before the cap because `Fixed`'s `+` is unchecked: a large `tau_max` cap
    // could overflow the raw i64 accumulator before the `.min` clamps it, so saturate then clamp.
    let mut optical_depth = Fixed::ZERO;
    for &(coefficient, length) in absorption_samples {
        let segment = laws::optical_depth(coefficient, length, bounds.tau_max);
        optical_depth = optical_depth.saturating_add(segment).min(bounds.tau_max);
    }
    Reach {
        spread,
        optical_depth,
    }
}

/// Resolve the reach of a signal on a channel's [`ChannelReach`] row from a source to a perceiver: sample
/// the medium's OWN absorption along the 3D `Coord3` path from the material field, and dispatch the
/// spreading law by the row's kernel id, never by channel identity (the condition-4 harden-to-registry
/// contract, so adding a kernel or a channel is a match-arm or data change, never a channel-identity
/// branch). Pure and off the run path (no live caller): the being-percept keystone consumes it.
///
/// The emitted power is a PARAMETER: it is the source's own emission on the channel (a being's body, a
/// fire's material), which the keystone resolves from the source's own material through the row's
/// `source_power_axis`. Reading it from the cell the source stands on would conflate a being's emission
/// with the ground under it, so it is not read here (surfaced to the gate as the emitter-power sub-fork).
///
/// Fail-loud on a reserved path: a `frequency_dependent` row cannot be resolved in slice 1 (the emitter
/// body-resonance frequency source and the [`civsim_physics::laws::acoustic_absorption`] application are
/// not wired), so this asserts the row is frequency-independent rather than silently reading its medium
/// axis as if it were (Prime Directive 3, reserved fail-loud). The assert never fires today: no shipped
/// row sets `frequency_dependent`, and the function has no live caller.
pub fn resolve_reach(
    row: &ChannelReach,
    emitted_power: Fixed,
    source: Coord3,
    perceiver: Coord3,
    field: &MaterialField,
    reg: &PhysicsRegistry,
    bounds: ReachBounds,
) -> Reach {
    assert!(
        !row.frequency_dependent,
        "frequency-dependent absorption is a reserved follow-on (the emitter body-resonance frequency \
         source and laws::acoustic_absorption are not wired in slice 1); a channel row must set \
         frequency_dependent = false until that segment lands"
    );
    let samples = absorption_along(source, perceiver, field, reg, &row.absorption_axis);
    match row.spreading {
        SpreadKernel::Geometric => {
            received_reach(emitted_power, source, perceiver, bounds, &samples)
        }
    }
}

/// Sample the medium's bulk absorption on `axis` along the 3D `Coord3` path from `source` to `perceiver`,
/// one sample per cell the line crosses (the grid's own cell is the resolution, an engine bound, not a
/// fabricated sampling rate). Each sample is `(absorption_coefficient, segment_length)`: the coefficient
/// is the bulk axis mean of the material at that cell (an empty cell reads zero, transparent), and the
/// length is the path length divided evenly across the steps. Occlusion emerges as a run of
/// strongly-absorbing cells (rock at negative z) drives the accumulated optical depth up, never an
/// authored line-of-sight rule.
///
/// Endpoint convention: the walk samples cells `i` in `1..=steps`, so it includes the perceiver's own
/// cell (`i = steps`) and excludes the source's own cell (`i = 0`). The signal is attenuated by the medium
/// it arrives THROUGH, up to and including the cell it arrives at; the source's own cell is where the
/// signal originates, so its medium is not on the arrival path. This is a modelling convention, pinned by
/// test, not a physical inevitability.
///
/// Honest limits (flagged, two):
/// - MEDIUM KIND: this reads the [`MaterialField`], substances in a 3D `Coord3` grid, so a signal through
///   rock occludes and through empty cells passes; the fluid medium's own absorption (the ambient
///   [`crate::medium::MediumField`]) is not sampled here. So a being whose dominant occluder IS its fluid
///   (a water-dweller, an atmosphere-swimmer) has that fluid treated as transparent, which understates its
///   occlusion. The full volumetric medium is the flagged z-stacked-medium follow-on.
/// - AGGREGATION: the per-cell coupling is the volume-weighted bulk mean ([`crate::material::SubstanceMix::bulk_axis`]), the
///   linear mixing the material substrate uses everywhere. A channel whose occlusion is NOT a linear
///   volume-mean of the cell (a threshold occluder, a saturating or max-dominated medium) cannot be
///   expressed by the row today, which carries no aggregation selector. The data-expressible
///   aggregation-kernel form is the flagged sibling of the [`SpreadKernel`] registry, not built here.
fn absorption_along(
    source: Coord3,
    perceiver: Coord3,
    field: &MaterialField,
    reg: &PhysicsRegistry,
    axis: &str,
) -> Vec<(Fixed, Fixed)> {
    let dx = perceiver.x as i128 - source.x as i128;
    let dy = perceiver.y as i128 - source.y as i128;
    let dz = perceiver.z as i128 - source.z as i128;
    let sep2 = dx * dx + dy * dy + dz * dz;
    let steps = dx.abs().max(dy.abs()).max(dz.abs());
    if steps == 0 || sep2 > MAX_REPRESENTABLE_SEP2 {
        // Co-located (no path), or beyond the representable separation where the spread is negligible so
        // the path attenuation does not matter: no samples.
        return Vec::new();
    }
    let r = Fixed::from_int(sep2 as i32).sqrt();
    let seg_len = r
        .checked_div(Fixed::from_int(steps as i32))
        .unwrap_or(Fixed::ZERO);
    let mut samples = Vec::with_capacity(steps as usize);
    for i in 1..=steps {
        let cx = (source.x as i128 + dx * i / steps) as i32;
        let cy = (source.y as i128 + dy * i / steps) as i32;
        let cz = (source.z as i128 + dz * i / steps) as i32;
        let coefficient = field
            .cell(Coord3 {
                x: cx,
                y: cy,
                z: cz,
            })
            .map(|mix| mix.bulk_axis(reg, axis))
            .unwrap_or(Fixed::ZERO);
        samples.push((coefficient, seg_len));
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::SubstanceMix;

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
            !acoustic.frequency_dependent,
            "the dev acoustic row is a resolvable direct-medium-read channel; the frequency-dependent \
             absorption path is a reserved follow-on, so no shipped row sets it",
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

    // The D=3 sphere coefficient, derived from the exact `Fixed::PI` constant rather than a truncated
    // decimal literal. Passed identically to both kernels in the byte-identity tests, so it drives no
    // absolute-value assertion; deriving it from PI keeps the test constant honest.
    fn four_pi() -> Fixed {
        Fixed::from_int(4)
            .checked_mul(Fixed::PI)
            .expect("4*pi is representable")
    }

    /// Build [`ReachBounds`] with the D=3 sphere coefficient `4*pi`, the given irradiance cap, and the
    /// given optical-depth cap.
    fn bounds(irrad_max: Fixed, tau_max: Fixed) -> ReachBounds {
        ReachBounds {
            sphere_coeff: four_pi(),
            irrad_max,
            tau_max,
        }
    }

    #[test]
    fn received_reach_spreads_by_the_general_kernel_over_the_3d_separation() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 3, y: 4, z: 0 }; // sep2 = 9 + 16 = 25, r = 5
        let reach = received_reach(power, src, per, bounds(cap, Fixed::from_int(1000)), &[]);
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
            bounds(cap, tau),
            &[],
        );
        let vertical = received_reach(
            power,
            src,
            Coord3 { x: 0, y: 0, z: 5 },
            bounds(cap, tau),
            &[],
        );
        assert_eq!(horizontal.spread, vertical.spread);
        // A vertical offset changes the reach; the 2D horizontal index metric would drop it.
        let above = received_reach(
            power,
            src,
            Coord3 { x: 3, y: 0, z: 4 },
            bounds(cap, tau),
            &[],
        ); // sep2 = 25
        let flat = received_reach(
            power,
            src,
            Coord3 { x: 3, y: 0, z: 0 },
            bounds(cap, tau),
            &[],
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
            bounds(cap, tau_max),
            &[(Fixed::from_int(100), Fixed::from_int(10))],
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
            bounds(cap, tau_max),
            &[(Fixed::ZERO, Fixed::from_int(10))],
        );
        assert_eq!(clear.optical_depth, Fixed::ZERO);
    }

    #[test]
    fn received_reach_at_zero_separation_reads_the_geometric_cap() {
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let c = Coord3 { x: 7, y: -2, z: 1 };
        let reach = received_reach(power, c, c, bounds(cap, Fixed::from_int(1000)), &[]);
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
        // A separation whose square exceeds the representable cast bound is clamped to zero; well within
        // that bound the geometric-spread denominator has already overflowed its checked_mul and returned
        // zero on its own, so a far source reads zero either way.
        let far = Coord3 {
            x: 100_000,
            y: 0,
            z: 0,
        };
        let reach = received_reach(power, src, far, bounds(cap, Fixed::from_int(1000)), &[]);
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
        // Byte-NEUTRALITY of this slice comes from the module having no live caller, not from this
        // identity (the run_world pins hold because nothing calls this code). What this identity buys is
        // FORWARD safety: when the being-percept keystone wires this in at D=3, the reach spreads bit-for-
        // bit as inverse_square_falloff, so it introduces no new spreading behaviour versus the law the
        // rest of the engine already uses. With D=3 everywhere (no confinement substrate) the general
        // kernel reproduces inverse-square exactly.
        let cap = Fixed::from_int(1_000_000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 6, y: 8, z: 0 }; // sep2 = 100, r = 10
        let reach = received_reach(power, src, per, bounds(cap, Fixed::from_int(1000)), &[]);
        let r = Fixed::from_int(100).sqrt();
        assert_eq!(
            reach.spread,
            laws::inverse_square_falloff(power, r, four_pi(), cap),
            "at D=3 the reach spread equals inverse_square_falloff (pins hold)",
        );
    }

    // --- The run-path resolver (segment 4): read the row, sample the medium along the Coord3 path,
    // dispatch the kernel by the row's id ---

    /// A minimal self-contained floor for the resolver tests: the one optical-absorption axis the reach
    /// samples along a path, and one strongly-absorbing substance ("rock") carrying it. Ranges and values
    /// are stand-in test data, not owner values (the real axis lives on the chem/optics floor).
    const TEST_FLOOR: &str = r#"
[[axis]]
id = "opt.absorption_coefficient"
measures = "medium absorption per unit path (the optical-depth lever)"
unit = "1/m"
dimension = "-1,0,0,0"
scale = "1/m"
tier = 0
range_lo = "0"
range_hi = "1000000"
real = "test fixture"

[[substance]]
id = "rock"
participates_in = []
real = "test fixture"
values = [
  { axis = "opt.absorption_coefficient", value = "50" },
]
"#;

    fn test_reg() -> PhysicsRegistry {
        PhysicsRegistry::from_toml_str(TEST_FLOOR).expect("test floor parses")
    }

    fn rock_cell() -> SubstanceMix {
        let mut m = SubstanceMix::new();
        m.set("rock", Fixed::from_int(1));
        m
    }

    #[test]
    fn resolve_reach_dispatches_by_the_row_and_an_empty_field_is_transparent() {
        let reg = test_reg();
        let field = MaterialField::new();
        let row = ChannelReachRegistry::dev_terran()
            .get(DEV_OPTICAL)
            .expect("optical row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let tau_max = Fixed::from_int(1000);
        let power = Fixed::from_int(100);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 3, y: 4, z: 0 }; // sep2 = 25, r = 5
        let reach = resolve_reach(&row, power, src, per, &field, &reg, bounds(cap, tau_max));
        // An empty field reads zero absorption at every cell along the path: transparent, no occlusion.
        assert_eq!(
            reach.optical_depth,
            Fixed::ZERO,
            "an empty material field is transparent"
        );
        // The spread is the geometric-kernel reach dispatched by the Geometric row, independent of the
        // (empty) medium: it equals the direct received_reach.
        let direct = received_reach(power, src, per, bounds(cap, tau_max), &[]);
        assert_eq!(
            reach.spread, direct.spread,
            "the Geometric row dispatches to the geometric-spread reach",
        );
    }

    #[test]
    fn resolve_reach_occlusion_emerges_from_absorbing_cells() {
        let reg = test_reg();
        let mut field = MaterialField::new();
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 4, y: 0, z: 0 }; // Chebyshev steps = 4, cells (1,0,0)..(4,0,0)
                                               // Fill every cell the path crosses with strongly-absorbing rock.
        for x in 1..=4 {
            field.set_cell(Coord3 { x, y: 0, z: 0 }, rock_cell());
        }
        let row = ChannelReachRegistry::dev_terran()
            .get(DEV_OPTICAL)
            .expect("optical row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let tau_max = Fixed::from_int(1000);
        let power = Fixed::from_int(100);
        let blocked = resolve_reach(&row, power, src, per, &field, &reg, bounds(cap, tau_max));
        // A run of absorbing cells accumulates optical depth: occlusion emerges from the medium data, with
        // no authored line-of-sight rule.
        assert!(
            blocked.optical_depth > Fixed::ZERO,
            "a rock-filled path occludes (optical depth accumulates)"
        );
        // With the same geometry but an empty field, the path is transparent: the difference is the medium.
        let empty = MaterialField::new();
        let clear = resolve_reach(&row, power, src, per, &empty, &reg, bounds(cap, tau_max));
        assert_eq!(clear.optical_depth, Fixed::ZERO);
        assert_eq!(
            blocked.spread, clear.spread,
            "occlusion changes the optical depth, not the geometric spread"
        );
    }

    #[test]
    fn absorption_along_is_empty_for_a_co_located_source() {
        let reg = test_reg();
        let field = MaterialField::new();
        let c = Coord3 { x: 2, y: 2, z: 2 };
        let samples = absorption_along(c, c, &field, &reg, "opt.absorption_coefficient");
        assert!(
            samples.is_empty(),
            "a co-located source has no path to sample"
        );
    }

    #[test]
    fn absorption_along_samples_one_coefficient_per_crossed_cell() {
        let reg = test_reg();
        let mut field = MaterialField::new();
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 4, y: 0, z: 0 };
        // Only the second crossed cell (2,0,0) carries rock; the rest are empty (transparent).
        field.set_cell(Coord3 { x: 2, y: 0, z: 0 }, rock_cell());
        let samples = absorption_along(src, per, &field, &reg, "opt.absorption_coefficient");
        assert_eq!(
            samples.len(),
            4,
            "one sample per Chebyshev step along the path"
        );
        // Exactly the rock cell reads its bulk coefficient; the empty cells read zero.
        let nonzero: Vec<Fixed> = samples
            .iter()
            .map(|&(c, _)| c)
            .filter(|&c| c > Fixed::ZERO)
            .collect();
        assert_eq!(nonzero.len(), 1, "only the rock cell absorbs");
        assert_eq!(nonzero[0], Fixed::from_int(50), "rock's bulk absorption");
    }

    #[test]
    fn absorption_along_includes_the_perceiver_cell_and_excludes_the_source_cell() {
        // Pin the endpoint convention: cells i in 1..=steps, so the perceiver's own cell is sampled and
        // the source's own cell is not. Rock in only the source cell reads transparent; rock in only the
        // perceiver cell occludes.
        let reg = test_reg();
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 4, y: 0, z: 0 };

        let mut source_only = MaterialField::new();
        source_only.set_cell(src, rock_cell());
        let s = absorption_along(src, per, &source_only, &reg, "opt.absorption_coefficient");
        assert!(
            s.iter().all(|&(c, _)| c == Fixed::ZERO),
            "the source's own cell is excluded from the path samples"
        );

        let mut perceiver_only = MaterialField::new();
        perceiver_only.set_cell(per, rock_cell());
        let p = absorption_along(
            src,
            per,
            &perceiver_only,
            &reg,
            "opt.absorption_coefficient",
        );
        assert_eq!(
            p.iter().filter(|&&(c, _)| c > Fixed::ZERO).count(),
            1,
            "the perceiver's own cell is included in the path samples"
        );
    }

    #[test]
    #[should_panic(expected = "frequency-dependent absorption is a reserved follow-on")]
    fn resolve_reach_fails_loud_on_a_frequency_dependent_row() {
        // The reserved fail-loud: a row that declares frequency_dependent cannot be resolved in slice 1
        // (no frequency source, no acoustic-law wiring), so resolve_reach asserts rather than silently
        // reading the medium axis as if the channel were frequency-independent.
        let reg = test_reg();
        let field = MaterialField::new();
        let row = ChannelReach {
            channel: DEV_ACOUSTIC,
            spreading: SpreadKernel::Geometric,
            source_power_axis: "acoustic.source_power".to_string(),
            absorption_axis: "opt.absorption_coefficient".to_string(),
            frequency_dependent: true,
        };
        let cap = Fixed::from_int(1_000_000);
        let src = Coord3 { x: 0, y: 0, z: 0 };
        let per = Coord3 { x: 3, y: 4, z: 0 };
        let _ = resolve_reach(
            &row,
            Fixed::from_int(100),
            src,
            per,
            &field,
            &reg,
            bounds(cap, Fixed::from_int(1000)),
        );
    }
}
