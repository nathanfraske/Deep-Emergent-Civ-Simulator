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

//! The contact-energy-transfer registry (hunt-kill strike arc, piece 1): the data-defined binding from a
//! contact channel to the physics-floor law by which an acting part delivers energy into what it contacts.
//! It is the harden-to-registry sibling of the channel reach registry ([`crate::perception_reach`]): the
//! kernel SET is fixed Rust (the mechanism), and the membership (which contact channels exist and which
//! transfer law each delivers by) is data that grows with the world (Principle 11).
//!
//! The strike framing panel caught the seam this registry fixes: computing the delivered energy solely
//! through the kinetic law (mass and velocity) hardcodes which PHYSICS a contact may hurt through, so a being
//! whose contact attack is electrical, chemical, thermal, or a non-Terran channel with no Earth analogue
//! could not be expressed by plugging numbers into one mass-velocity function; it would need a new function,
//! a rewrite rather than a data row. Here the delivered energy is resolved by dispatching on the NAMED kernel
//! a channel's row carries, so a new delivery channel is a row (and, where a genuinely new law is needed, one
//! kernel on the floor), never by editing a `match channel { Kinetic => ..., Electrical => ... }`. Kinetic
//! is the first (Terran, mass-bearing) instance; the law-set is small, fixed, and extensible.
//!
//! What a kernel READS is the acting part's own data. The kinetic kernel reads the part's actuating force (its
//! strength stress over its cross-section) and its stroke distance (its own grown `mech.stroke_length`), so a
//! stronger, thicker, or longer-stroked part delivers more energy, keyed on the being's own body, never a
//! per-species number and never a world-global swing speed. The caller derives the force and stroke from the
//! axes the row declares and passes them (as [`crate::perception_reach::resolve_reach`] takes the emitted power
//! as a parameter), so this substrate stays a pure law dispatch with no body-representation dependency.

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_physics::laws;

/// The transfer-law kernel a contact channel delivers energy by. The kernel SET is fixed Rust code (the
/// mechanism); which kernel a channel uses is data (the registry row). Today only [`TransferKernel::Kinetic`]
/// is built: the general mass-bearing contact law. A non-kinetic channel (an electrical discharge, a chemical
/// or thermal touch, a mana coupling) is the flagged floor extension, so a new VARIANT here is a deliberate
/// floor addition with its own law, never an authored per-channel branch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TransferKernel {
    /// Mass-bearing contact: the delivered energy is the ACTUATOR WORK that brought the acting part to speed
    /// ([`civsim_physics::laws::actuator_work`], force times stroke distance), the work-energy form of the
    /// kinetic energy. The swing-speed intermediate is retired because it only round-trips to this work
    /// (substituting `v = sqrt(2 F d / m)` into `1/2 m v^2` cancels the mass and returns `F d`), so the delivered
    /// energy is the actuating force over the stroke, read from the part's own strength, cross-section, and
    /// grown stroke geometry, never a world-global swing speed.
    Kinetic,
}

/// One contact channel's transfer binding as data: the law its energy delivers by (dispatched by this kernel
/// id, never by channel identity) and the physics-floor axes the kinetic (actuator-work) kernel reads the
/// acting part's actuating force and stroke from. Every field is data (Principle 11); the resolve is fixed Rust
/// that consumes derived inputs. The axes are floor axis id strings, the same string-keyed floor reference the
/// reach and percept substrates use, so the floor stays the one authored place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactTransfer {
    /// The contact channel this row binds.
    pub channel: ContactChannelId,
    /// The transfer law the channel delivers by (dispatched by this id, never by channel identity).
    pub kernel: TransferKernel,
    /// The physics-floor MATERIAL axis id the acting part's actuating STRESS is read from (its strength per unit
    /// cross-section). Named as DATA so the caller reads the axis the row declares rather than a hardcoded field
    /// id (Principle 11): a Terran actuator names `mat.fracture_strength` (the axis the physiology already reads
    /// as muscle strength), an alien actuator its own strength axis. The caller multiplies this stress by the
    /// cross-section axis below to form the actuating force.
    pub strength_axis: String,
    /// The physics-floor GEOMETRY axis id the acting part's load-bearing CROSS-SECTION is read from. The
    /// actuating force is the strength (above) over this area (`mat.fracture_strength * mech.cross_section_area`,
    /// an N). Named as data so an alien body names its own force geometry.
    pub cross_section_axis: String,
    /// The physics-floor GEOMETRY axis id the acting part's STROKE distance is read from (the distance the
    /// actuating force acts over, the acting part's own grown `mech.stroke_length`). The delivered energy is the
    /// actuator work, force times this stroke ([`civsim_physics::laws::actuator_work`]). Grown independently of
    /// the segment length so the acting-distance-to-length ratio is per-body data, never a fixed one (the
    /// value-authoring fix). Named as data so an alien actuator names its own stroke geometry.
    pub stroke_axis: String,
}

/// The set of contact-transfer bindings a world runs, keyed by [`ContactChannelId`] in canonical (ascending)
/// order so any walk is reproducible and the registry has one representation for one membership. Data-defined
/// and extensible: a new channel is covered the moment it registers its row. EMPTY by default, so a world that
/// declares no transfer bindings runs no contact-energy resolve (the substrate is opt-in and off the run path
/// until the strike wire consumes it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContactTransferRegistry {
    channels: BTreeMap<ContactChannelId, ContactTransfer>,
}

impl ContactTransferRegistry {
    /// An empty registry: no contact channel delivers, so no transfer resolve fires. The default and the
    /// opt-out.
    pub fn empty() -> ContactTransferRegistry {
        ContactTransferRegistry {
            channels: BTreeMap::new(),
        }
    }

    /// Insert or replace a channel's transfer binding, keyed by its own channel id, so the store stays
    /// canonical.
    pub fn insert(&mut self, transfer: ContactTransfer) {
        self.channels.insert(transfer.channel, transfer);
    }

    /// The transfer binding for a channel, if one is registered. The resolve dispatches on the returned row's
    /// kernel id, never on the channel id itself.
    pub fn get(&self, channel: ContactChannelId) -> Option<&ContactTransfer> {
        self.channels.get(&channel)
    }

    /// Iterate the rows in canonical (ascending channel id) order.
    pub fn iter(&self) -> impl Iterator<Item = (&ContactChannelId, &ContactTransfer)> {
        self.channels.iter()
    }

    /// Whether the registry declares no channel (the opt-out).
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    /// A labelled DEVELOPMENT FIXTURE: the one contact channel the physics floor already carries a law for, a
    /// kinetic (actuator-work) channel that reads the acting part's actuating force off `mat.fracture_strength`
    /// over `mech.cross_section_area` and its stroke off the grown `mech.stroke_length`. Not owner data; the
    /// minimum a contact resolve needs to exercise the actuator-work law. The real channel set is the world's
    /// data, and non-kinetic channels are the flagged floor extension.
    pub fn dev_terran() -> ContactTransferRegistry {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            // The Terran actuator: strength stress over cross-section is the force, the grown stroke length the
            // distance it acts over. A body names its own strength, cross-section, and stroke axes.
            strength_axis: "mat.fracture_strength".to_string(),
            cross_section_axis: "mech.cross_section_area".to_string(),
            stroke_axis: "mech.stroke_length".to_string(),
        });
        reg
    }
}

/// The kinetic dev-fixture contact channel (a leaf id, not special-cased in any mechanism).
pub const DEV_KINETIC: ContactChannelId = ContactChannelId(1);

/// A contact channel id: an opaque leaf id keying a transfer binding, never read for its value by any
/// mechanism (the resolve dispatches on the row's kernel, not this id).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ContactChannelId(pub u16);

/// Resolve the mechanical energy an acting part delivers on a channel's [`ContactTransfer`] row from the
/// part's OWN grown axes, read through the `geo` and `mat` accessors (an axis id to its grown value, the same
/// closure form [`civsim_compose::derive_capabilities`] reads a part's function through, so this stays a pure
/// law dispatch with no body-representation dependency). Off the run path (the strike wire is opt-in and no
/// pinned scenario arms it), so byte-neutral by construction.
///
/// RUN-ALL-GATE-TO-ZERO (the stroke-rate step-2 substrate, gate-signed-off, owner-decisions R15): the delivered
/// mechanical energy is resolved over the registered delivered-energy kernel set (exactly one today,
/// [`TransferKernel::Kinetic`], dispatched below), each kernel contributing the energy its own grounded floor
/// law delivers and reading ZERO where the part carries none of that law's axes (the absence convention). So
/// which law contributes DERIVES from the part's continuous grown physics, never a grown categorical
/// actuation-kind selector and never an authored threshold:
/// a rigid lever, an elastic recoil, and a hydraulic jet differ only in which continuous axes are nonzero, an
/// emergent DESCRIPTION of where a part lands in axis space, mirroring how the capability laws already emerge by
/// physics not a tag. Today the kernel set is exactly [`TransferKernel::Kinetic`], the ACTUATOR WORK `F d`
/// (strength stress over cross-section, the acting distance the grown stroke), the rigid limit; a new grounded
/// delivered-energy law (an elastic-recoil `1/2 k x^2`, a hydraulic `integral P dV`) is a new kernel gated on
/// its own axes. `energy_max` is the floor representability cap each law saturates at.
///
/// AGGREGATION across kernels is a single-kernel identity today and is DEFERRED to the slice that lands the
/// second kernel: summing the per-kernel energies would double-count where they share an energy source (a
/// spring's stored elastic energy IS the muscle work that loaded it), so the cross-kernel combine (a max over
/// the dominant delivery mechanism, or a partition that avoids the shared-source double-count) is settled with
/// the gate when kernel two lands, not pre-authored here. With one kernel the aggregate is that kernel's energy.
///
/// The DERIVED TOOL-GEOMETRY follow-on (the arc's flagged additive payoff (b), a longer wielded tool extends the
/// effective stroke and a heavier one the sustainable force) drops in at the CALLER, which holds the wielded
/// tool: the caller augments the `geo`/`mat` closure it passes (a wrapped accessor that adds the tool's stroke
/// on the stroke axis), an additive read on the SAME `F d` law and the same axis, never a re-foundation here.
pub fn resolve_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    row: &ContactTransfer,
    energy_max: Fixed,
) -> Fixed {
    // Run-all-gate-to-zero over the registered delivered-energy kernels. One kernel today, so the aggregate is
    // its energy; a new kernel adds a gated term here and the cross-kernel combine is settled with the gate then.
    match row.kernel {
        TransferKernel::Kinetic => kinetic_delivered_energy(geo, mat, row, energy_max),
    }
}

/// The KINETIC (rigid-actuator) delivered-energy law: the actuator work `F d`, where the force is the acting
/// part's strength stress (`row.strength_axis`) over its cross-section (`row.cross_section_axis`) promoted to
/// newtons by the floor's [`laws::stress_force`] (its megapascal-to-newton bridge), and the distance is the
/// part's own grown stroke (`row.stroke_axis`). The rigid limit of the run-all-gate-to-zero set: a part with no
/// strength or no stroke delivers zero (the absence convention, [`laws::actuator_work`] returns zero), so this
/// kernel self-gates. Reads only the part's own grown axes, no per-species constant and no world-global swing
/// speed (admit-the-alien: an actuator on a different physics carries a different kernel gated on its own axes).
fn kinetic_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    row: &ContactTransfer,
    energy_max: Fixed,
) -> Fixed {
    let force = laws::stress_force(
        mat(&row.strength_axis),
        geo(&row.cross_section_axis),
        energy_max,
    );
    laws::actuator_work(force, geo(&row.stroke_axis), energy_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_registry_is_the_opt_out() {
        let reg = ContactTransferRegistry::empty();
        assert!(reg.is_empty());
        assert!(reg.get(DEV_KINETIC).is_none());
    }

    #[test]
    fn a_transfer_is_looked_up_by_id_and_carries_its_law_and_axes_as_data() {
        let reg = ContactTransferRegistry::dev_terran();
        // Dispatch is by channel id into the registry, then by the row's kernel id: never a code branch on
        // channel identity.
        let kinetic = reg.get(DEV_KINETIC).expect("kinetic row present");
        assert_eq!(kinetic.kernel, TransferKernel::Kinetic);
        // The Terran kinetic channel names the strength, cross-section, and stroke axes the strike wire reads
        // the actuating force and stroke off, all data (Principle 11).
        assert_eq!(kinetic.strength_axis, "mat.fracture_strength");
        assert_eq!(kinetic.cross_section_axis, "mech.cross_section_area");
        assert_eq!(kinetic.stroke_axis, "mech.stroke_length");
        assert!(reg.get(ContactChannelId(99)).is_none());
    }

    #[test]
    fn the_registry_walks_in_canonical_channel_id_order() {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer {
            channel: ContactChannelId(2),
            kernel: TransferKernel::Kinetic,
            strength_axis: "a".to_string(),
            cross_section_axis: "a".to_string(),
            stroke_axis: "a".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: ContactChannelId(1),
            kernel: TransferKernel::Kinetic,
            strength_axis: "b".to_string(),
            cross_section_axis: "b".to_string(),
            stroke_axis: "b".to_string(),
        });
        let ids: Vec<u16> = reg.iter().map(|(c, _)| c.0).collect();
        assert_eq!(ids, vec![1, 2], "canonical ascending channel id order");
    }

    #[test]
    fn a_later_insert_replaces_a_row_keyed_by_channel() {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            strength_axis: "first".to_string(),
            cross_section_axis: "first".to_string(),
            stroke_axis: "first".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            strength_axis: "second".to_string(),
            cross_section_axis: "second".to_string(),
            stroke_axis: "second".to_string(),
        });
        assert_eq!(reg.iter().count(), 1, "one row per channel id");
        assert_eq!(reg.get(DEV_KINETIC).unwrap().stroke_axis, "second");
    }

    /// A part's grown-axis accessors for the kinetic kernel: a strength stress on `mat.fracture_strength`, a
    /// cross-section on `mech.cross_section_area`, and a stroke on `mech.stroke_length`; every other axis reads
    /// zero (the absence convention). The cross-section is on the 1e-6 m^2 scale that keeps a 200 MPa strength a
    /// modest newton force through the floor's megapascal-to-newton bridge, well under the representability cap.
    fn part_axes(
        strength: Fixed,
        cross_section: Fixed,
        stroke: Fixed,
    ) -> (impl Fn(&str) -> Fixed, impl Fn(&str) -> Fixed) {
        let geo = move |a: &str| match a {
            "mech.cross_section_area" => cross_section,
            "mech.stroke_length" => stroke,
            _ => Fixed::ZERO,
        };
        let mat = move |a: &str| match a {
            "mat.fracture_strength" => strength,
            _ => Fixed::ZERO,
        };
        (geo, mat)
    }

    #[test]
    fn kinetic_resolve_is_the_actuator_work_of_the_parts_own_axes() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let strength = Fixed::from_int(200); // MPa
        let cross_section = Fixed::from_ratio(1, 1_000_000); // m^2
        let stroke = Fixed::from_int(1); // m
        let (geo, mat) = part_axes(strength, cross_section, stroke);
        // The resolve reads the part's axes and dispatches the Kinetic kernel to the floor laws: the actuating
        // force is the strength stress over the cross-section (`stress_force`, its megapascal-to-newton bridge),
        // and the delivered energy the actuator work of that force over the stroke. It adds no arithmetic of its own.
        let force = laws::stress_force(strength, cross_section, cap);
        assert_eq!(
            resolve_delivered_energy(&geo, &mat, &row, cap),
            laws::actuator_work(force, stroke, cap),
        );
    }

    #[test]
    fn a_stronger_or_longer_stroked_part_delivers_more_energy() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let m = |n: i32| Fixed::from_ratio(n as i64, 1_000_000); // cross-section on the m^2 scale
        let deliver = |strength: Fixed, cross_section: Fixed, stroke: Fixed| {
            let (geo, mat) = part_axes(strength, cross_section, stroke);
            resolve_delivered_energy(&geo, &mat, &row, cap)
        };
        let base = deliver(Fixed::from_int(200), m(1), Fixed::from_int(1)); // 200 N over 1 m: 200 J
                                                                            // A greater actuating strength delivers more energy (linear in force), keyed on the part's own material.
        let stronger = deliver(Fixed::from_int(400), m(1), Fixed::from_int(1));
        // A longer stroke delivers more energy (linear in distance), keyed on the part's own grown geometry.
        let longer = deliver(Fixed::from_int(200), m(1), Fixed::from_int(2));
        assert!(stronger > base && longer > base && base > Fixed::ZERO);
        // The kernel self-gates: a part with no strength, no cross-section, or no stroke delivers no blow (the
        // absence convention, so a part that grew none of the kinetic axes contributes zero, run-all-gate-to-zero).
        assert_eq!(deliver(Fixed::ZERO, m(1), Fixed::from_int(1)), Fixed::ZERO);
        assert_eq!(
            deliver(Fixed::from_int(200), Fixed::ZERO, Fixed::from_int(1)),
            Fixed::ZERO
        );
        assert_eq!(
            deliver(Fixed::from_int(200), m(1), Fixed::ZERO),
            Fixed::ZERO
        );
        // Deterministic: identical inputs give the identical bit-exact energy (Principle 3).
        assert_eq!(
            base,
            deliver(Fixed::from_int(200), m(1), Fixed::from_int(1))
        );
    }
}
