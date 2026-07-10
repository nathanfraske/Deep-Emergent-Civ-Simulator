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
//! per-species number and never a world-global swing speed. The resolve reads those grown values through the
//! `geo`/`mat` ACCESSOR closures the caller passes (an axis id to its grown value, the same closure form
//! [`civsim_compose::derive_capabilities`] reads a part's function through) and derives the force and stroke
//! itself, so the substrate stays a pure law dispatch keyed on axis-id strings with no dependency on the body's
//! representation type (it reads axis-id-to-value closures, never a concrete body struct).

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_physics::laws;

/// The transfer-law kernel a contact channel delivers energy by. The kernel SET is fixed Rust code (the
/// mechanism); which kernel a channel uses is data (the registry row). [`TransferKernel::Kinetic`] (the rigid
/// `F d`) and [`TransferKernel::ElasticRecoil`] (the elastic strain-energy release) are the two members of the
/// SHARED-METABOLIC-SOURCE mechanical family: a segment's delivered mechanical energy is the run-all-gate-to-zero
/// MAX over them (both draw the one muscle-work source, so which contributes DERIVES from the segment's own
/// continuous grown axes, never a selector; see [`resolve_delivered_energy`]). A non-mechanical channel (an
/// electrical discharge, a chemical or thermal touch, a mana coupling) drawing a SEPARATE independent reserve is
/// the flagged floor extension, so a new VARIANT here is a deliberate floor addition with its own law AND its own
/// combine (additive across independent reserves, not folded into the mechanical MAX), never an authored
/// per-channel branch.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TransferKernel {
    /// The RIGID mechanical path: the delivered energy is the ACTUATOR WORK that brought the acting part to speed
    /// ([`civsim_physics::laws::actuator_work`], force times stroke distance), the work-energy form of the
    /// kinetic energy. The swing-speed intermediate is retired because it only round-trips to this work
    /// (substituting `v = sqrt(2 F d / m)` into `1/2 m v^2` cancels the mass and returns `F d`), so the delivered
    /// energy is the actuating force over the stroke, read from the part's own strength, cross-section, and
    /// grown stroke geometry, never a world-global swing speed. The rigid limit of the mechanical family.
    Kinetic,
    /// The ELASTIC mechanical path: the delivered energy is the elastic STRAIN ENERGY a springy actuator stores
    /// up to yield and releases in a recoil blow ([`civsim_physics::laws::elastic_recoil_energy`], the modulus of
    /// resilience `yield^2 / (2 E)` over the strained volume), a whip tip or a trap-jaw latch. The elastic sibling
    /// of the rigid `F d`, on the same joule currency, so the two combine on one scale. A rigid or fluid actuator
    /// (no yield strength, no elastic modulus) stores none and this path self-gates to zero, so the mechanical MAX
    /// falls back to the rigid limit. Shares the metabolic source with [`Self::Kinetic`] (a spring's stored energy
    /// IS the muscle work that loaded it), so the two aggregate by MAX, never SUM.
    ElasticRecoil,
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
    /// value-authoring fix). Named as data so an alien actuator names its own stroke geometry. The elastic path
    /// also reuses this axis (with `cross_section_axis`) as the swept strained VOLUME (see `yield_axis`).
    pub stroke_axis: String,
    /// The physics-floor MATERIAL axis id the acting part's YIELD STRENGTH is read from, the elastic path's first
    /// material input (the stress up to which the actuator stores elastic strain energy). Named as DATA, the
    /// elastic sibling of `strength_axis`, so a springy alien binding names its own yield axis (Principle 11): a
    /// Terran springy tissue names `mat.yield_strength`. A part with no yield strength stores no elastic energy and
    /// the elastic path self-gates to zero (the absence convention), so a rigid actuator reads exactly the rigid
    /// `F d`.
    pub yield_axis: String,
    /// The physics-floor MATERIAL axis id the acting part's ELASTIC MODULUS is read from, the elastic path's second
    /// material input (the stiffness `E` in the modulus of resilience `yield^2 / (2 E)`). Named as data so an alien
    /// names its own stiffness axis. The strained VOLUME the elastic energy density integrates over is the SWEPT
    /// actuator volume `cross_section_axis * stroke_axis` (the two geometry axes the rigid path already reads,
    /// reused with no new axis, the gate's slice-3 ruling). That swept volume is a PROXY for the elastic element's
    /// own volume, exact only where they correlate; a dedicated `mech.elastic_element_volume` floor axis read
    /// directly is the reserved-with-basis REFINEMENT for a world whose element volume diverges from its sweep,
    /// surfaced not authored, not built now.
    pub elastic_modulus_axis: String,
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
            // The Terran springy tissue: the elastic path reads yield strength and elastic modulus off the floor's
            // own material axes and the swept volume off the (shared) cross-section and stroke geometry. A rigid
            // Terran actuator grows no yield or modulus, so the elastic path reads zero and the rigid `F d` stands.
            yield_axis: "mat.yield_strength".to_string(),
            elastic_modulus_axis: "mat.elastic_modulus".to_string(),
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
/// mechanical energy is resolved over the SHARED-METABOLIC-SOURCE mechanical kernel family (the rigid `F d`
/// [`TransferKernel::Kinetic`] and the elastic recoil [`TransferKernel::ElasticRecoil`]), each member contributing
/// the energy its own grounded floor law delivers and reading ZERO where the part carries none of that law's axes
/// (the absence convention). So which member contributes DERIVES from the part's continuous grown physics, never a
/// grown categorical actuation-kind selector and never an authored threshold: a rigid lever and an elastic recoil
/// differ only in which continuous axes are nonzero (strength and stroke, or yield and modulus), an emergent
/// DESCRIPTION of where a part lands in axis space, mirroring how [`civsim_compose::derive_capabilities`] already
/// runs every capability law blind to id and zeroes the inapplicable. `energy_max` is the floor representability
/// cap each law saturates at.
///
/// AGGREGATION is the MAX over the family (the gate's slice-3 ruling, confirmed): the members are alternative
/// delivery PATHS for ONE metabolically-sourced energy (a spring's stored recoil energy IS the muscle work that
/// loaded it), so SUM would double-count that shared source and MAX selects the dominant path. Byte-neutral at
/// the rigid limit: a part growing only the rigid axes (yield and modulus at floor-low) reads exactly the rigid
/// `F d`, because the elastic member self-gates to zero and `MAX(F d, 0) = F d`.
///
/// FLAGGED FUTURE COUPLING (the gate's slice-3 ruling, ON RECORD, not folded in): the exception is keyed on the
/// energy SOURCE being independent, NOT on the channel being non-mechanical. A kernel drawing a SEPARATE,
/// INDEPENDENT reserve is ADDITIVE with the mechanical family rather than a shared path, so the MAX here
/// UNDER-counts it. That independent source can be a non-mechanical channel (an electrical discharge fuelled apart
/// from the stroke) OR a MECHANICALLY-elastic actuator charged from a pathway that is not the muscle stroke (a
/// section-9 alien-lens catch): a turgor- or photosynthesis-charged seed-pod spring (Impatiens, the squirting
/// cucumber), a light- or mana-charged latch, stores elastic strain energy the being did not load with a muscle
/// stroke, so for it the elastic recoil is a second joule, not the same one the rigid `F d` carries. Such a case
/// is NOT a shared-source mechanical-family member and is NOT MAX-folded; when a world grows one it joins as a new
/// [`TransferKernel`] variant with an additive cross-family combine, a deliberate decision. The exhaustive match
/// below makes that unmissable: a new variant outside the mechanical arm fails to compile until its family and
/// combine are chosen. (Today the floor models no independent elastic-charging pathway, so the Terran case where a
/// spring's recoil IS the loading work holds and MAX is correct; the independent-source case is the flagged gap.)
///
/// The DERIVED TOOL-GEOMETRY follow-on (the arc's flagged additive payoff (b), a longer wielded tool extends the
/// effective stroke and a heavier one the sustainable force) drops in at the CALLER, which holds the wielded
/// tool: the caller augments the `geo`/`mat` closure it passes (a wrapped accessor that adds the tool's stroke
/// on the stroke axis), an additive read on the SAME laws and the same axes, never a re-foundation here.
///
/// Off the run path (the strike wire is opt-in and no pinned scenario arms it), so byte-neutral by construction.
pub fn resolve_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    row: &ContactTransfer,
    energy_max: Fixed,
) -> Fixed {
    // The channel's kernel names its delivery FAMILY (its representative member); the resolve runs the whole
    // family's run-all-gate-to-zero, not the one named member, so which law contributes derives from the grown
    // axes, never this tag. Today the two members are the one shared-metabolic-source mechanical family, so the
    // resolve is MAX over the rigid `F d` and the elastic recoil. A future independent-reserve variant would fall
    // outside this arm and fail to compile until its additive cross-family combine is chosen (the flagged coupling).
    match row.kernel {
        TransferKernel::Kinetic | TransferKernel::ElasticRecoil => {
            let rigid = kinetic_delivered_energy(geo, mat, row, energy_max);
            let elastic = elastic_recoil_delivered_energy(geo, mat, row, energy_max);
            rigid.max(elastic)
        }
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

/// The ELASTIC-RECOIL delivered-energy law (the elastic member of the shared-source mechanical family): the
/// elastic strain energy a springy actuator stores up to yield and releases in a recoil blow,
/// [`laws::elastic_recoil_energy`] of the part's yield strength (`row.yield_axis`), elastic modulus
/// (`row.elastic_modulus_axis`), and strained VOLUME. The volume is the SWEPT actuator volume
/// `cross_section_axis * stroke_axis` (the two geometry axes the rigid path already reads, reused with no new
/// axis, the gate's slice-3 ruling). A part with no yield strength or no elastic modulus (a rigid or fluid
/// actuator) stores no elastic energy and reads zero (the absence convention, [`laws::elastic_recoil_energy`]
/// gates), so the elastic member self-gates and the mechanical MAX falls back to the rigid `F d`. Reads only the
/// part's own grown axes named as data on the row (admit-the-alien: a springy binding names its own axes).
///
/// The swept volume is a PROXY for the elastic element's own volume, exact only where they correlate (the gate's
/// slice-3 limit ON RECORD); the dedicated `mech.elastic_element_volume` floor axis read directly is the
/// reserved-with-basis REFINEMENT, surfaced not authored, not built now.
fn elastic_recoil_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    row: &ContactTransfer,
    energy_max: Fixed,
) -> Fixed {
    // The swept strained volume, the two rigid-path geometry axes reused. A volume beyond the representable range
    // saturates to `energy_max` as a numeric sentinel; the law gates on the material first (no yield or modulus
    // reads zero however large the volume), and re-caps a present-material product at `energy_max`, so the sentinel
    // never fabricates energy for a rigid actuator.
    let volume = geo(&row.cross_section_axis)
        .checked_mul(geo(&row.stroke_axis))
        .unwrap_or(energy_max);
    laws::elastic_recoil_energy(
        mat(&row.yield_axis),
        mat(&row.elastic_modulus_axis),
        volume,
        energy_max,
    )
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
        // the actuating force and stroke off, and the yield and modulus axes the elastic path reads, all data
        // (Principle 11).
        assert_eq!(kinetic.strength_axis, "mat.fracture_strength");
        assert_eq!(kinetic.cross_section_axis, "mech.cross_section_area");
        assert_eq!(kinetic.stroke_axis, "mech.stroke_length");
        assert_eq!(kinetic.yield_axis, "mat.yield_strength");
        assert_eq!(kinetic.elastic_modulus_axis, "mat.elastic_modulus");
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
            yield_axis: "a".to_string(),
            elastic_modulus_axis: "a".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: ContactChannelId(1),
            kernel: TransferKernel::Kinetic,
            strength_axis: "b".to_string(),
            cross_section_axis: "b".to_string(),
            stroke_axis: "b".to_string(),
            yield_axis: "b".to_string(),
            elastic_modulus_axis: "b".to_string(),
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
            yield_axis: "first".to_string(),
            elastic_modulus_axis: "first".to_string(),
        });
        reg.insert(ContactTransfer {
            channel: DEV_KINETIC,
            kernel: TransferKernel::Kinetic,
            strength_axis: "second".to_string(),
            cross_section_axis: "second".to_string(),
            stroke_axis: "second".to_string(),
            yield_axis: "second".to_string(),
            elastic_modulus_axis: "second".to_string(),
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

    /// A part's grown-axis accessors for the FULL mechanical family: the kinetic strength/cross-section/stroke plus
    /// the elastic yield strength and elastic modulus; every other axis reads zero (the absence convention). Lets a
    /// test grow a rigid part (strength, no yield/modulus), a springy part (yield/modulus, no strength), or both.
    fn full_part_axes(
        strength: Fixed,
        cross_section: Fixed,
        stroke: Fixed,
        yield_s: Fixed,
        modulus: Fixed,
    ) -> (impl Fn(&str) -> Fixed, impl Fn(&str) -> Fixed) {
        let geo = move |a: &str| match a {
            "mech.cross_section_area" => cross_section,
            "mech.stroke_length" => stroke,
            _ => Fixed::ZERO,
        };
        let mat = move |a: &str| match a {
            "mat.fracture_strength" => strength,
            "mat.yield_strength" => yield_s,
            "mat.elastic_modulus" => modulus,
            _ => Fixed::ZERO,
        };
        (geo, mat)
    }

    #[test]
    fn the_mechanical_resolve_is_the_max_of_the_rigid_and_elastic_paths_and_a_springy_part_reads_its_recoil(
    ) {
        // Slice-3b core (the gate's slice-3 ruling): the delivered mechanical energy is the run-all-gate-to-zero
        // MAX over the rigid `F d` and the elastic recoil, each self-gating on the part's own grown axes. This is
        // the ADVERSARIAL PROBE the byte-neutral kinetic tests cannot be: reverting the resolve to the rigid path
        // alone reads ZERO for the springy-only part below (it grows no rigid strength), flipping the assertion.
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let cap = Fixed::from_int(1_000_000);
        let m = |n: i32| Fixed::from_ratio(n as i64, 1_000_000); // cross-section on the m^2 scale
        let deliver = |strength: Fixed, yield_s: Fixed, modulus: Fixed| {
            let (geo, mat) = full_part_axes(strength, m(1), Fixed::from_int(1), yield_s, modulus);
            resolve_delivered_energy(&geo, &mat, &row, cap)
        };
        // A RIGID part (strength 200 MPa, no yield or modulus): the elastic path self-gates to zero, so the resolve
        // is EXACTLY the rigid `F d` (byte-neutral at the rigid limit). Cross-checked against the kinetic-only
        // accessors and the floor laws directly.
        let rigid_only = deliver(Fixed::from_int(200), Fixed::ZERO, Fixed::ZERO);
        let (kgeo, kmat) = part_axes(Fixed::from_int(200), m(1), Fixed::from_int(1));
        assert_eq!(
            rigid_only,
            resolve_delivered_energy(&kgeo, &kmat, &row, cap),
            "a part with no yield or modulus reads exactly the rigid F d (the elastic member self-gates)"
        );
        let expected_rigid = laws::actuator_work(
            laws::stress_force(Fixed::from_int(200), m(1), cap),
            Fixed::from_int(1),
            cap,
        );
        assert_eq!(rigid_only, expected_rigid, "the rigid F d, 200 N over 1 m");
        // A SPRINGY-ONLY part (no fracture strength, yield 200 / modulus 2000): the rigid path self-gates and the
        // resolve is the elastic recoil, resilience 200^2/(2*2000)=10 (MPa) * C_PA * swept volume (1e-6 m^3). It is
        // POSITIVE, so a kinetic-only revert would read zero here (the mutation the probe catches).
        let springy_only = deliver(Fixed::ZERO, Fixed::from_int(200), Fixed::from_int(2000));
        let vol = m(1).checked_mul(Fixed::from_int(1)).unwrap();
        let expected_elastic =
            laws::elastic_recoil_energy(Fixed::from_int(200), Fixed::from_int(2000), vol, cap);
        assert_eq!(
            springy_only, expected_elastic,
            "a springy part with no rigid strength delivers its elastic recoil"
        );
        assert!(
            springy_only > Fixed::ZERO,
            "the elastic recoil is a positive blow (the kinetic-only revert reads zero here): {springy_only:?}"
        );
        // MAX selects the DOMINANT path: a weak rigid strength (1 MPa) under the same springy tissue reads the
        // larger elastic energy; a strong rigid strength (200 MPa) reads the larger rigid energy.
        let elastic_dominates = deliver(
            Fixed::from_int(1),
            Fixed::from_int(200),
            Fixed::from_int(2000),
        );
        assert_eq!(
            elastic_dominates, springy_only,
            "MAX picks the elastic path when it exceeds the weak rigid path"
        );
        let rigid_dominates = deliver(
            Fixed::from_int(200),
            Fixed::from_int(200),
            Fixed::from_int(2000),
        );
        assert_eq!(
            rigid_dominates, rigid_only,
            "MAX picks the rigid path when it exceeds the elastic path"
        );
        assert!(
            rigid_dominates > elastic_dominates,
            "the strong-rigid part outdelivers the springy-only one ({rigid_dominates:?} vs {elastic_dominates:?})"
        );
        // Both absent: no blow (run-all-gate-to-zero, the absence convention on both members).
        assert_eq!(
            deliver(Fixed::ZERO, Fixed::ZERO, Fixed::ZERO),
            Fixed::ZERO,
            "no rigid strength and no elastic tissue: no blow"
        );
        // Deterministic (Principle 3).
        assert_eq!(
            springy_only,
            deliver(Fixed::ZERO, Fixed::from_int(200), Fixed::from_int(2000))
        );
    }

    #[test]
    fn the_impact_grade_binding_and_the_delivery_row_name_the_same_axes_in_lockstep() {
        // The grade/delivery LOCKSTEP the gate ruled (slice-3 ruling iv), PINNED by a test (a section-9
        // correctness/steering-lens catch: the two paths coincide only BY CONSTRUCTION today, the grade path
        // binding its axes POSITIONALLY (`FunctionLawDef` Vec order) and the delivery path by NAMED fields
        // (`ContactTransfer`), with nothing in code linking them, so a reorder of either could silently desync the
        // grade from the delivery with no compile error). This test fails if the IMPACT kernel's positional axis
        // contract or the canonical delivery row's named axes drift apart, so the by-convention lockstep is no
        // longer untested. The role map: grade `material_axes` [strength, yield, modulus] and `geometry_axes`
        // [cross_section, stroke] must name the delivery row's `strength_axis` / `yield_axis` /
        // `elastic_modulus_axis` / `cross_section_axis` / `stroke_axis`. The full named-vs-positional UNIFICATION
        // (so the lockstep is mechanically enforced, not test-pinned) stays the gate-deferred follow-on.
        use civsim_compose::{CapabilityKernel, FunctionLawDef, FunctionLawRegistry};
        let grade = FunctionLawDef::new(
            FunctionLawRegistry::ID_IMPACT,
            "impact",
            CapabilityKernel::Impact,
        );
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        assert_eq!(
            grade.material_axes.first().map(String::as_str),
            Some(row.strength_axis.as_str()),
            "grade strength axis (material_axes[0]) matches the delivery row strength_axis"
        );
        assert_eq!(
            grade.geometry_axes.first().map(String::as_str),
            Some(row.cross_section_axis.as_str()),
            "grade cross-section axis (geometry_axes[0]) matches the delivery row cross_section_axis"
        );
        assert_eq!(
            grade.geometry_axes.get(1).map(String::as_str),
            Some(row.stroke_axis.as_str()),
            "grade stroke axis (geometry_axes[1]) matches the delivery row stroke_axis"
        );
        assert_eq!(
            grade.material_axes.get(1).map(String::as_str),
            Some(row.yield_axis.as_str()),
            "grade yield axis (material_axes[1]) matches the delivery row yield_axis"
        );
        assert_eq!(
            grade.material_axes.get(2).map(String::as_str),
            Some(row.elastic_modulus_axis.as_str()),
            "grade modulus axis (material_axes[2]) matches the delivery row elastic_modulus_axis"
        );
    }
}
