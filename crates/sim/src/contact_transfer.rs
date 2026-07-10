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

/// Resolve the energy an acting part delivers on a channel's [`ContactTransfer`] row, dispatching the transfer
/// law by the row's kernel id, never by channel identity (the harden-to-registry contract, so adding a kernel
/// or a channel is a match-arm or data change, never a channel-identity branch). Pure and off the run path (no
/// live caller): the hunt-kill strike wire consumes it, so this is byte-neutral by construction.
///
/// The delivery inputs are PARAMETERS the caller derives from the acting part's OWN body: `actuator_force` is
/// the actuating force (the part's strength stress over its cross-section, read off the row's `strength_axis`
/// and `cross_section_axis`) and `stroke_distance` is the distance the force acts over (the part's own grown
/// `mech.stroke_length`, read off the row's `stroke_axis`). The kinetic kernel is
/// [`civsim_physics::laws::actuator_work`] of those (force times distance, the delivered energy directly); a
/// future non-kinetic kernel reads its own channel-appropriate inputs. `energy_max` is the physics-floor
/// representability cap the law saturates at. Keying on the part's own strength, cross-section, and stroke, a
/// stronger, thicker, or longer-stroked part delivers more energy, never a per-species constant and never a
/// world-global swing speed (admit-the-alien: a massless energy-being would carry a different channel and
/// kernel, a data row).
pub fn resolve_transfer(
    row: &ContactTransfer,
    actuator_force: Fixed,
    stroke_distance: Fixed,
    energy_max: Fixed,
) -> Fixed {
    match row.kernel {
        TransferKernel::Kinetic => laws::actuator_work(actuator_force, stroke_distance, energy_max),
    }
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

    #[test]
    fn kinetic_resolve_is_the_actuator_work_of_the_parts_force_and_stroke() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let force = Fixed::from_int(2);
        let stroke = Fixed::from_int(3);
        // The resolve dispatches the Kinetic kernel to the floor actuator-work law over the given force and
        // stroke: the substrate adds no arithmetic of its own, it selects the law.
        assert_eq!(
            resolve_transfer(&row, force, stroke, cap),
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
        let base = resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(3), cap);
        // A greater actuating force delivers more energy (linear in force), keyed on the part's own strength and
        // cross-section.
        let stronger = resolve_transfer(&row, Fixed::from_int(4), Fixed::from_int(3), cap);
        // A longer stroke delivers more energy (linear in distance), keyed on the part's own grown geometry.
        let longer = resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(6), cap);
        assert!(stronger > base && longer > base && base > Fixed::ZERO);
        // A zero-strength actuator delivers no blow (the absence convention).
        assert_eq!(
            resolve_transfer(&row, Fixed::ZERO, Fixed::from_int(3), cap),
            Fixed::ZERO
        );
        // Deterministic: identical inputs give the identical bit-exact energy (Principle 3).
        assert_eq!(
            base,
            resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(3), cap)
        );
    }
}
