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
//! What a kernel READS is the acting part's own data. The kinetic kernel reads the part's mass (the caller
//! derives it from the part's own density and geometry) and the contact velocity, so a denser or faster
//! contacting part delivers more energy, keyed on the being's own body, never a per-species number. The caller
//! passes the derived inputs (as [`crate::perception_reach::resolve_reach`] takes the emitted power as a
//! parameter), so this substrate stays a pure law dispatch with no body-representation dependency.

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
    /// Mass-bearing contact: the delivered energy is the kinetic energy of the acting part
    /// ([`civsim_physics::laws::kinetic_energy`], one-half m v-squared), from the part's own mass and the
    /// contact velocity.
    Kinetic,
}

/// One contact channel's transfer binding as data: the law its energy delivers by (dispatched by this kernel
/// id, never by channel identity) and the physics-floor material axis the kinetic kernel reads the acting
/// part's delivery mass from. Every field is data (Principle 11); the resolve is fixed Rust that consumes this
/// row. The axis is a floor axis id string, the same string-keyed floor reference the reach and percept
/// substrates use, so the floor stays the one authored place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactTransfer {
    /// The contact channel this row binds.
    pub channel: ContactChannelId,
    /// The transfer law the channel delivers by (dispatched by this id, never by channel identity).
    pub kernel: TransferKernel,
    /// The physics-floor axis id the acting part's delivery mass is read from. Named as DATA so the caller reads
    /// the axis the row declares rather than a hardcoded field id (Principle 11): a world binds its channel's
    /// mass source here, and the strike wire reads the acting part's mass off THIS axis
    /// ([`crate::runner::Embodiment::strike_occupant`] reads `seg.geo(source_axis)`). The run-path Segment
    /// carries the EXTENSIVE mass datum `mech.mass` (volume times density, integrated at growth), so a Terran
    /// channel names `mech.mass` and the caller reads it directly; a world whose body carries an intensive
    /// density axis instead names that and the caller integrates over geometry (the reserved density-to-mass
    /// read), so the mass source stays the row's data, never a caller constant.
    pub source_axis: String,
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
    /// kinetic (mass-bearing) channel that reads the acting part's delivery mass off the extensive `mech.mass`
    /// axis. Not owner data; the minimum a contact resolve needs to exercise the kinetic law. The real channel
    /// set is the world's data, and non-kinetic channels are the flagged floor extension.
    pub fn dev_terran() -> ContactTransferRegistry {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            // The extensive mass datum a grown part carries (volume times density), the axis the strike wire
            // reads the delivery mass off. A Terran mass-bearing channel names it directly.
            source_axis: "mech.mass".to_string(),
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
/// The delivery inputs are PARAMETERS the caller derives from the acting part's OWN body: `part_mass` is the
/// part's mass (its material density integrated over its geometry) and `contact_velocity` is the approach
/// speed of the contact. The kinetic kernel is [`civsim_physics::laws::kinetic_energy`] of those; a future
/// non-kinetic kernel reads its own channel-appropriate inputs. `energy_max` is the physics-floor
/// representability cap the law saturates at. Keying on the part's own mass and velocity, a denser or faster
/// part delivers more energy, never a per-species constant (admit-the-alien: a massless energy-being would
/// carry a different channel and kernel, a data row).
pub fn resolve_transfer(
    row: &ContactTransfer,
    part_mass: Fixed,
    contact_velocity: Fixed,
    energy_max: Fixed,
) -> Fixed {
    match row.kernel {
        TransferKernel::Kinetic => laws::kinetic_energy(part_mass, contact_velocity, energy_max),
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
    fn a_transfer_is_looked_up_by_id_and_carries_its_law_and_axis_as_data() {
        let reg = ContactTransferRegistry::dev_terran();
        // Dispatch is by channel id into the registry, then by the row's kernel id: never a code branch on
        // channel identity.
        let kinetic = reg.get(DEV_KINETIC).expect("kinetic row present");
        assert_eq!(kinetic.kernel, TransferKernel::Kinetic);
        // The Terran kinetic channel names the extensive mass axis the strike wire reads the delivery mass off.
        assert_eq!(kinetic.source_axis, "mech.mass");
        assert!(reg.get(ContactChannelId(99)).is_none());
    }

    #[test]
    fn the_registry_walks_in_canonical_channel_id_order() {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer {
            channel: ContactChannelId(2),
            kernel: TransferKernel::Kinetic,
            source_axis: "a".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: ContactChannelId(1),
            kernel: TransferKernel::Kinetic,
            source_axis: "b".to_string(),
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
            source_axis: "first".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            source_axis: "second".to_string(),
        });
        assert_eq!(reg.iter().count(), 1, "one row per channel id");
        assert_eq!(reg.get(DEV_KINETIC).unwrap().source_axis, "second");
    }

    #[test]
    fn kinetic_resolve_is_the_kinetic_energy_of_the_parts_mass_and_velocity() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let mass = Fixed::from_int(2);
        let velocity = Fixed::from_int(3);
        // The resolve dispatches the Kinetic kernel to the floor kinetic-energy law over the given mass and
        // velocity: the substrate adds no arithmetic of its own, it selects the law.
        assert_eq!(
            resolve_transfer(&row, mass, velocity, cap),
            laws::kinetic_energy(mass, velocity, cap),
        );
    }

    #[test]
    fn a_denser_or_faster_part_delivers_more_energy() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let base = resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(3), cap);
        // A more massive contacting part delivers more energy (linear in mass), keyed on the part's own body.
        let heavier = resolve_transfer(&row, Fixed::from_int(4), Fixed::from_int(3), cap);
        // A faster contact delivers more energy (quadratic in velocity), keyed on the contact kinematics.
        let faster = resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(6), cap);
        assert!(heavier > base && faster > base && base > Fixed::ZERO);
        // Deterministic: identical inputs give the identical bit-exact energy (Principle 3).
        assert_eq!(
            base,
            resolve_transfer(&row, Fixed::from_int(2), Fixed::from_int(3), cap)
        );
    }
}
