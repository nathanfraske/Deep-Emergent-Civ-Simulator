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
//! a channel's row carries, so a new delivery channel is a row (and, where a new law is needed, one
//! kernel on the floor), never by editing a `match channel { Kinetic => ..., Electrical => ... }`. Kinetic
//! is the first (Terran, mass-bearing) instance; the law-set is small, fixed, and extensible.
//!
//! What a kernel READS is the acting part's own data, addressed BY ROLE NAME through the SINGLE-SOURCE
//! [`civsim_compose::AxisBinding`] (a role-name to floor-axis-id map). The kinetic kernel reads the part's actuating
//! force (its `actuating_strength` role over its `cross_section` role) and its stroke distance (its `stroke` role),
//! so a stronger, thicker, or longer-stroked part delivers more energy, keyed on the being's own body, never a
//! per-species number and never a world-global swing speed. The resolve reads those grown values through the
//! `geo`/`mat` ACCESSOR closures the caller passes (an axis id to its grown value, the same closure form
//! [`civsim_compose::derive_capabilities`] reads a part's function through) and derives the force and stroke
//! itself, so the substrate stays a pure law dispatch with no dependency on the body's representation type (it
//! reads axis-id-to-value closures, never a concrete body struct).
//!
//! The binding is the grade law's OWN binding, read directly: the grade-binding unification (the gate's #129
//! arc, owner-decisions R15) put both the capability GRADE path ([`civsim_compose::FunctionLawDef`]) and this
//! delivery path on ONE shared [`civsim_compose::AxisBinding`] type, and Slice C made it a SINGLE SOURCE. A
//! delivery row does not own a binding copy; it names the `grade_law` whose binding it shares
//! ([`ContactTransfer::grade_law`], the mechanical family names IMPACT), and [`resolve_delivered_energy`] reads
//! that one binding (fetched from the grade registry by the caller). So an alien actuation names its own axes once,
//! in the grade law's binding, and the grade and the delivery read the SAME map: a role-to-axis mapping desync is
//! impossible by construction, never two copies that could diverge. A dangling `grade_law` is a fail-loud load
//! error at [`ContactTransferRegistry::validate`].

use std::collections::BTreeMap;

use civsim_compose::{AxisBinding, FunctionLawId, FunctionLawRegistry};
use civsim_core::Fixed;
use civsim_physics::laws;

/// The transfer-law kernel a contact channel delivers energy by. The kernel SET is fixed Rust code (the
/// mechanism); which kernel a channel uses is data (the registry row). [`TransferKernel::Kinetic`] (the rigid
/// `F d`), [`TransferKernel::ElasticRecoil`] (the elastic strain-energy release), and [`TransferKernel::Hydraulic`]
/// (the fluid-pressure `P dV`) are the three members of the SHARED-METABOLIC-SOURCE mechanical family: a segment's
/// delivered mechanical energy is the run-all-gate-to-zero MAX over them (all draw the one metabolic work source,
/// so which contributes DERIVES from the segment's own continuous grown axes, never a selector; see
/// [`resolve_delivered_energy`]). A non-mechanical channel (an electrical discharge, a chemical or thermal touch, a
/// mana coupling) drawing a SEPARATE independent reserve is the flagged floor extension, so a new VARIANT there is
/// a deliberate floor addition with its own law AND its own combine (additive across independent reserves, not
/// folded into the mechanical MAX), never an authored per-channel branch.
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
    /// The HYDRAULIC mechanical path: the delivered energy is the pressure-over-volume-change work `integral P dV`
    /// of a working-fluid actuator (a muscular hydrostat, an octopus arm, a spider's hydraulic leg, a water jet).
    /// For an INCOMPRESSIBLE fluid at a constant driving pressure this integral COMPOSES from the existing floor
    /// laws with NO new kernel: `P dV = P (A d) = (P A) d = F d`, the actuator work of the force `F = P A`, where
    /// `P` is the part's `fluid.driving_pressure` (a pressure the megapascal-to-newton bridge promotes over the
    /// piston cross-section exactly as it promotes a material strength, [`laws::stress_force`]) and `d` the stroke.
    /// So this member is [`laws::actuator_work`] of [`laws::stress_force`] read off the FLUID driving pressure
    /// rather than a solid strength, keyed on the part's own axes. A part with no driving pressure delivers zero and
    /// this path self-gates. For a METABOLICALLY-charged hydrostat it shares the source with [`Self::Kinetic`] (the
    /// fluid charge IS the work that pressurized it), so it aggregates by MAX, never SUM; but a SOURCE-INDEPENDENT
    /// hydrostat (an osmotic or turgor charge, not a muscle stroke) is under-counted by that MAX, the flagged
    /// future coupling on [`resolve_delivered_energy`] (source-independence is orthogonal to this variant, so it is
    /// not caught by adding a variant). TWO ASSUMPTIONS on record: `F = P A` is the NET piston force under the
    /// DELTA-P reading of `fluid.driving_pressure` (the axis documents delta-P OR an absolute pressure; under the
    /// absolute reading the net force subtracts the ambient back-pressure, not modelled here), and the piston area
    /// is the reused structural `cross_section_axis`, a PROXY for the fluid chamber's own bore (a dedicated
    /// piston-area / `fluid.channel_radius`-derived axis is the reserved refinement). The COMPRESSIBLE case (a gas
    /// expanding as it drives, so `P` falls over the stroke, the true varying-`P` integral reading
    /// `fluid.bulk_modulus` and an equation of state) does NOT reduce to `F d`: the kernel does not read
    /// `fluid.bulk_modulus` to detect it, so a compressible fluid must NOT be bound to this member (no runtime
    /// guard), it is the flagged FUTURE kernel, a genuine new floor law.
    Hydraulic,
}

/// One contact channel's transfer binding as data: the law FAMILY its energy delivers by (dispatched by this
/// kernel id, never by channel identity) and a REFERENCE to the grade law whose [`AxisBinding`] the delivery
/// kernels read the acting part's axes from BY ROLE NAME. Every field is data (Principle 11); the resolve is
/// fixed Rust that consumes derived inputs.
///
/// SLICE C of the grade-binding unification made the binding a SINGLE SOURCE: the delivery row does NOT own a
/// binding copy, it names the `grade_law` whose binding it shares (the mechanical family names IMPACT), and
/// [`resolve_delivered_energy`] reads that ONE binding (fetched from the grade registry by the caller). So a
/// grade/delivery role-to-axis MAPPING desync is impossible by CONSTRUCTION: there is one map, not two copies
/// that could diverge, retiring the "equal by convention" fragility the earlier slices left. The referenced grade
/// law's binding was validated at its own construction ([`civsim_compose::FunctionLawDef::with_binding`]), so the
/// delivery inherits a validated binding; a dangling `grade_law` (not in the grade registry) is a fail-loud load
/// error at the wire boundary ([`ContactTransferRegistry::validate`]), the real "missing key at load".
///
/// What the shared binding names: the rigid path reads `actuating_strength` (the strength stress), `cross_section`
/// (the load-bearing area, so the actuating force is the stress over the area, an N), and `stroke` (the distance
/// the force acts over, the actuator work `F d`); the elastic path reads `yield_strength`, `elastic_modulus`, and
/// the swept strained VOLUME `cross_section * stroke` (the two geometry roles reused, a PROXY for the elastic
/// element's own volume; a dedicated `mech.elastic_element_volume` axis is the reserved-with-basis refinement);
/// the hydraulic path reads `driving_pressure` over `cross_section` (reused as the piston area) over `stroke`. An
/// alien actuator names its own axis id per role in the grade law's binding (Principle 11); a role a part does not
/// grow reads zero (the absence convention), so a rigid actuator (no yield, no modulus, no driving pressure) reads
/// exactly the rigid `F d` and the elastic and hydraulic paths self-gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactTransfer {
    /// The contact channel this row binds.
    pub channel: ContactChannelId,
    /// The transfer-law FAMILY the channel delivers by (dispatched by this id, never by channel identity). The
    /// mechanical family runs the run-all-gate-to-zero MAX over {rigid, elastic, hydraulic}.
    pub kernel: TransferKernel,
    /// The grade law whose [`AxisBinding`] this delivery SHARES, the single source of the role-to-axis mapping.
    /// The mechanical family names [`civsim_compose::FunctionLawRegistry::ID_IMPACT`] (the delivery family and the
    /// IMPACT grade are the one shared-metabolic-source mechanical family, the gate's lockstep ruling,
    /// owner-decisions R15). [`resolve_delivered_energy`] reads the referenced grade law's binding, so the delivery
    /// and the grade cannot map a role to two different axes: there is one map, and a grade/delivery desync is a
    /// type impossibility rather than a convention. A dangling reference (a `grade_law` not in the grade registry)
    /// is a fail-loud load error at [`ContactTransferRegistry::validate`].
    pub grade_law: FunctionLawId,
}

impl ContactTransfer {
    /// Build a transfer row referencing the grade law whose binding it shares. Infallible: the row names a
    /// `grade_law`, and whether that id resolves in the grade registry is checked at the wire boundary
    /// ([`ContactTransferRegistry::validate`]), where the grade registry is available. The referenced grade law's
    /// binding was validated at ITS construction, so the delivery inherits a validated binding: there is no second
    /// binding to validate here.
    pub fn new(
        channel: ContactChannelId,
        kernel: TransferKernel,
        grade_law: FunctionLawId,
    ) -> ContactTransfer {
        ContactTransfer {
            channel,
            kernel,
            grade_law,
        }
    }
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

    /// Validate every row's grade-law REFERENCE against the grade registry at LOAD: each row's `grade_law` must
    /// resolve in `grade_laws`, else the first offending channel is returned. The referenced grade law's binding
    /// was validated at its own construction ([`civsim_compose::FunctionLawDef::with_binding`]), so a resolved
    /// reference inherits a validated binding; a dangling reference is the fail-loud load error that makes a
    /// grade/delivery mapping desync impossible (there is no second binding to diverge, only a reference that must
    /// resolve). The walk is in canonical channel-id order, so the first-reported offender is deterministic. This
    /// is the wire-boundary check [`crate::runner::Embodiment::set_contact_transfer`] runs against the Embodiment's
    /// grade registry.
    pub fn validate(&self, grade_laws: &FunctionLawRegistry) -> Result<(), String> {
        for (channel, row) in self.channels.iter() {
            if grade_laws.get(row.grade_law).is_none() {
                return Err(format!(
                    "contact channel {}: grade_law {:?} is not in the grade registry",
                    channel.0, row.grade_law
                ));
            }
        }
        Ok(())
    }

    /// A labelled DEVELOPMENT FIXTURE: the one contact channel the physics floor already carries a law for, a
    /// kinetic (actuator-work) channel that SHARES the IMPACT grade law's binding by naming
    /// [`FunctionLawRegistry::ID_IMPACT`] as its `grade_law`. The delivery reads the IMPACT grade's own binding
    /// (`actuating_strength` to `mat.fracture_strength`, `cross_section` to `mech.cross_section_area`, `stroke` to
    /// `mech.stroke_length`, `yield_strength` to `mat.yield_strength`, `elastic_modulus` to `mat.elastic_modulus`,
    /// `driving_pressure` to `fluid.driving_pressure`, the byte-neutral default), so grade and delivery read from
    /// ONE map and a mapping desync is impossible by construction. Not owner data; the minimum a contact resolve
    /// needs to exercise the mechanical family. The real channel set is the world's data, and a source-independent
    /// or non-mechanical channel is the flagged floor extension.
    pub fn dev_terran() -> ContactTransferRegistry {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer::new(
            DEV_KINETIC,
            TransferKernel::Kinetic,
            FunctionLawRegistry::ID_IMPACT,
        ));
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
/// [`TransferKernel::Kinetic`], the elastic recoil [`TransferKernel::ElasticRecoil`], and the hydraulic `P dV`
/// [`TransferKernel::Hydraulic`]), each member contributing the energy its own grounded floor law delivers and
/// reading ZERO where the part carries none of that law's axes (the absence convention). So which member
/// contributes DERIVES from the part's continuous grown physics, never a grown categorical actuation-kind selector
/// and never an authored threshold: a rigid lever, an elastic recoil, and a hydraulic jet differ only in which
/// continuous axes are nonzero (strength and stroke, or yield and modulus, or driving pressure), an emergent
/// DESCRIPTION of where a part lands in axis space, mirroring how [`civsim_compose::derive_capabilities`] already
/// runs every capability law blind to id and zeroes the inapplicable. `energy_max` is the floor representability
/// cap each law saturates at. The hydraulic member adds NO new law: for an incompressible fluid at constant
/// pressure `integral P dV = F d` COMPOSES from the existing [`laws::stress_force`] and [`laws::actuator_work`]
/// (the driving pressure over the piston cross-section is the force), so the family grows without a new grounded
/// law; the compressible gas-expansion case is the flagged future kernel that would need one.
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
/// is NOT a shared-source mechanical-family member and is NOT MAX-folded. The same case exists for a HYDRAULIC
/// actuator: an osmotically or turgor-charged hydrostat (the squirting cucumber's fluid pressure, a plant cell's
/// turgor, a hydrostatic skeleton pressurized by an ion pump) drives a `P dV` blow whose driving pressure the being
/// did not raise with a muscle stroke, so its hydraulic energy is a second joule too. HONEST LIMIT of the
/// enforcement (a section-9 catch, correcting an earlier over-claim): energy-SOURCE independence is ORTHOGONAL to
/// the kernel VARIANT (which physics). A source-independent actuator REUSES an existing mechanical variant (a
/// turgor hydrostat IS a [`TransferKernel::Hydraulic`] `P dV`, a turgor spring an [`TransferKernel::ElasticRecoil`]),
/// so it lands in the mechanical MAX arm and is SILENTLY MAX-folded; the exhaustive match below catches only a
/// NEW variant (a new physics or channel, an electrical discharge), NOT a source-independent reuse of an existing
/// one. So the real future fix is a SOURCE-INDEPENDENCE DATUM (a way for a world to declare that a driving
/// pressure or a stored strain energy is independently charged) plus its additive combine, not merely a new
/// variant. Today the floor carries no such datum: `fluid.driving_pressure` and the elastic axes do not declare
/// their charging source, so every modeled mechanical actuator is metabolic-sourced and MAX is correct; the
/// source-independent case is the flagged gap, needing the source datum before it can be routed additively.
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
    binding: &AxisBinding,
    energy_max: Fixed,
) -> Fixed {
    // The channel's kernel names its delivery FAMILY (its representative member); the resolve runs the whole
    // family's run-all-gate-to-zero, not the one named member, so which law contributes derives from the grown
    // axes, never this tag. Today the three members are the one shared-metabolic-source mechanical family, so the
    // resolve is MAX over the rigid `F d`, the elastic recoil, and the hydraulic `P dV`. A future independent-reserve
    // variant would fall outside this arm and fail to compile until its additive cross-family combine is chosen (the
    // flagged coupling). The `binding` is the SINGLE SOURCE the caller fetched from the grade registry by the row's
    // `grade_law` (Slice C), so grade and delivery read the same map by construction.
    match row.kernel {
        TransferKernel::Kinetic | TransferKernel::ElasticRecoil | TransferKernel::Hydraulic => {
            let rigid = kinetic_delivered_energy(geo, mat, binding, energy_max);
            let elastic = elastic_recoil_delivered_energy(geo, mat, binding, energy_max);
            let hydraulic = hydraulic_delivered_energy(geo, mat, binding, energy_max);
            rigid.max(elastic).max(hydraulic)
        }
    }
}

/// Read a ROLE's floor-axis value through the GEOMETRY accessor (the role names a geometry quantity: a
/// cross-section, a stroke). An unbound role reads zero (the absence convention); a load-validated row always
/// carries the mechanical-family roles, so a required role never reads zero for absence, only for an axis the
/// part grew to zero. The delivery-path sibling of `civsim_compose`'s `role_geo`.
fn role_geo(geo: &dyn Fn(&str) -> Fixed, binding: &AxisBinding, role: &str) -> Fixed {
    binding.axis(role).map(geo).unwrap_or(Fixed::ZERO)
}

/// Read a ROLE's floor-axis value through the MATERIAL accessor (the role names a material quantity: a strength,
/// a yield, a modulus, a driving pressure). The material sibling of [`role_geo`].
fn role_mat(mat: &dyn Fn(&str) -> Fixed, binding: &AxisBinding, role: &str) -> Fixed {
    binding.axis(role).map(mat).unwrap_or(Fixed::ZERO)
}

/// The KINETIC (rigid-actuator) delivered-energy law: the actuator work `F d`, where the force is the acting
/// part's strength stress (the `actuating_strength` role) over its cross-section (the `cross_section` role)
/// promoted to newtons by the floor's [`laws::stress_force`] (its megapascal-to-newton bridge), and the distance
/// is the part's own grown stroke (the `stroke` role). The rigid limit of the run-all-gate-to-zero set: a part
/// with no strength or no stroke delivers zero (the absence convention, [`laws::actuator_work`] returns zero), so
/// this kernel self-gates. Reads its inputs by ROLE NAME through the row's [`AxisBinding`], no per-species constant
/// and no world-global swing speed (admit-the-alien: an actuator on a different physics names its own axis id per
/// role, a data row not a rewrite).
fn kinetic_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    binding: &AxisBinding,
    energy_max: Fixed,
) -> Fixed {
    let force = laws::stress_force(
        role_mat(mat, binding, "actuating_strength"),
        role_geo(geo, binding, "cross_section"),
        energy_max,
    );
    laws::actuator_work(force, role_geo(geo, binding, "stroke"), energy_max)
}

/// The ELASTIC-RECOIL delivered-energy law (the elastic member of the shared-source mechanical family): the
/// elastic strain energy a springy actuator stores up to yield and releases in a recoil blow,
/// [`laws::elastic_recoil_energy`] of the part's yield strength (the `yield_strength` role), elastic modulus
/// (the `elastic_modulus` role), and strained VOLUME. The volume is the SWEPT actuator volume
/// `cross_section * stroke` (the two geometry roles the rigid path already reads, reused with no new
/// role, the gate's slice-3 ruling). A part with no yield strength or no elastic modulus (a rigid or fluid
/// actuator) stores no elastic energy and reads zero (the absence convention, [`laws::elastic_recoil_energy`]
/// gates), so the elastic member self-gates and the mechanical MAX falls back to the rigid `F d`. Reads its inputs
/// by ROLE NAME through the row's [`AxisBinding`] (admit-the-alien: a springy binding names its own axes).
///
/// The swept volume is a PROXY for the elastic element's own volume, exact only where they correlate (the gate's
/// slice-3 limit ON RECORD); the dedicated `mech.elastic_element_volume` floor axis read directly is the
/// reserved-with-basis REFINEMENT, surfaced not authored, not built now.
fn elastic_recoil_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    binding: &AxisBinding,
    energy_max: Fixed,
) -> Fixed {
    // The swept strained volume, the two rigid-path geometry roles reused. A volume beyond the representable range
    // saturates to `energy_max` as a numeric sentinel; the law gates on the material first (no yield or modulus
    // reads zero however large the volume), and re-caps a present-material product at `energy_max`, so the sentinel
    // never fabricates energy for a rigid actuator.
    let volume = role_geo(geo, binding, "cross_section")
        .checked_mul(role_geo(geo, binding, "stroke"))
        .unwrap_or(energy_max);
    laws::elastic_recoil_energy(
        role_mat(mat, binding, "yield_strength"),
        role_mat(mat, binding, "elastic_modulus"),
        volume,
        energy_max,
    )
}

/// The HYDRAULIC delivered-energy law (the hydraulic member of the shared-source mechanical family): the
/// pressure-over-volume-change work `integral P dV` of a working-fluid actuator. For an incompressible fluid at a
/// constant driving pressure this COMPOSES from the existing floor laws with NO new kernel: `P dV = P (A d) = (P A)
/// d = F d`, so the delivered energy is [`laws::actuator_work`] of the force `F = P A`, where the piston force is
/// the part's `driving_pressure` role over its cross-section (the `cross_section` role, reused as the piston area)
/// promoted to newtons by [`laws::stress_force`] (the driving pressure is megapascal-stored, so the same
/// megapascal-to-newton bridge the rigid strength uses applies), over the stroke (the `stroke` role). This is the
/// SAME two laws the rigid [`kinetic_delivered_energy`] uses, keyed on the FLUID driving pressure rather than a
/// solid strength, so a hydrostat actuator is a data row, not a new law. A part with no driving pressure delivers
/// zero (the absence convention, [`laws::stress_force`] returns zero force), so this member self-gates and the
/// mechanical MAX falls back to the rigid `F d`. Reads its inputs by ROLE NAME through the row's [`AxisBinding`].
///
/// Reusing the piston cross-section is a PROXY for the fluid's own channel cross-section, exact where they
/// correlate; a dedicated `fluid.channel_radius`-derived piston area is the reserved-with-basis REFINEMENT. The
/// COMPRESSIBLE gas-expansion case (a gas expanding as it drives, so the pressure falls over the stroke, the true
/// varying-pressure integral reading `fluid.bulk_modulus` and an equation of state) does NOT reduce to `F d` and is
/// the flagged FUTURE kernel, a genuine new floor law, not built now.
fn hydraulic_delivered_energy(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    binding: &AxisBinding,
    energy_max: Fixed,
) -> Fixed {
    // The hydraulic piston force: the driving pressure over the piston cross-section, promoted to newtons by the
    // same megapascal-to-newton bridge the rigid path uses (the driving pressure is megapascal-stored like a
    // material strength). Then the actuator work over the stroke. A part with no driving pressure reads a zero
    // force and so a zero blow (the absence convention), so a non-fluid actuator self-gates.
    let force = laws::stress_force(
        role_mat(mat, binding, "driving_pressure"),
        role_geo(geo, binding, "cross_section"),
        energy_max,
    );
    laws::actuator_work(force, role_geo(geo, binding, "stroke"), energy_max)
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

    /// The IMPACT grade law's binding, the SINGLE SOURCE the delivery reads (Slice C): the dev grade registry's
    /// `ID_IMPACT` binding, exactly what the strike wire fetches by a row's `grade_law`. Both the grade and the
    /// delivery read this one value, so there is no second copy that could diverge.
    fn grade_binding() -> AxisBinding {
        FunctionLawRegistry::dev_seed()
            .get(FunctionLawRegistry::ID_IMPACT)
            .expect("the dev grade registry carries the IMPACT law")
            .binding
            .clone()
    }

    #[test]
    fn a_transfer_is_looked_up_by_id_and_names_its_grade_law() {
        let reg = ContactTransferRegistry::dev_terran();
        // Dispatch is by channel id into the registry, then by the row's kernel id: never a code branch on
        // channel identity.
        let kinetic = reg.get(DEV_KINETIC).expect("kinetic row present");
        assert_eq!(kinetic.kernel, TransferKernel::Kinetic);
        // The delivery row does not own a binding: it NAMES the grade law whose binding it shares (the single
        // source, Slice C), the IMPACT grade for the mechanical family.
        assert_eq!(kinetic.grade_law, FunctionLawRegistry::ID_IMPACT);
        // And that referenced grade law's binding maps each mechanical role to its floor axis (the one map both
        // paths read); the delivery reads exactly this, never a second copy.
        let binding = grade_binding();
        assert_eq!(
            binding.axis("actuating_strength"),
            Some("mat.fracture_strength")
        );
        assert_eq!(
            binding.axis("cross_section"),
            Some("mech.cross_section_area")
        );
        assert_eq!(binding.axis("stroke"), Some("mech.stroke_length"));
        assert_eq!(binding.axis("yield_strength"), Some("mat.yield_strength"));
        assert_eq!(binding.axis("elastic_modulus"), Some("mat.elastic_modulus"));
        assert_eq!(
            binding.axis("driving_pressure"),
            Some("fluid.driving_pressure")
        );
        assert!(reg.get(ContactChannelId(99)).is_none());
    }

    #[test]
    fn the_registry_walks_in_canonical_channel_id_order() {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer::new(
            ContactChannelId(2),
            TransferKernel::Kinetic,
            FunctionLawRegistry::ID_IMPACT,
        ));
        reg.insert(ContactTransfer::new(
            ContactChannelId(1),
            TransferKernel::Kinetic,
            FunctionLawRegistry::ID_IMPACT,
        ));
        let ids: Vec<u16> = reg.iter().map(|(c, _)| c.0).collect();
        assert_eq!(ids, vec![1, 2], "canonical ascending channel id order");
    }

    #[test]
    fn a_later_insert_replaces_a_row_keyed_by_channel() {
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer::new(
            DEV_KINETIC,
            TransferKernel::Kinetic,
            FunctionLawRegistry::ID_IMPACT,
        ));
        // A second insert on the same channel replaces the first; the differing kernel tag proves it is the later
        // row that survives.
        reg.insert(ContactTransfer::new(
            DEV_KINETIC,
            TransferKernel::ElasticRecoil,
            FunctionLawRegistry::ID_IMPACT,
        ));
        assert_eq!(reg.iter().count(), 1, "one row per channel id");
        assert_eq!(
            reg.get(DEV_KINETIC).unwrap().kernel,
            TransferKernel::ElasticRecoil
        );
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
        let binding = grade_binding();
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
            resolve_delivered_energy(&geo, &mat, &row, &binding, cap),
            laws::actuator_work(force, stroke, cap),
        );
    }

    #[test]
    fn a_stronger_or_longer_stroked_part_delivers_more_energy() {
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let binding = grade_binding();
        let cap = Fixed::from_int(1_000_000);
        let m = |n: i32| Fixed::from_ratio(n as i64, 1_000_000); // cross-section on the m^2 scale
        let deliver = |strength: Fixed, cross_section: Fixed, stroke: Fixed| {
            let (geo, mat) = part_axes(strength, cross_section, stroke);
            resolve_delivered_energy(&geo, &mat, &row, &binding, cap)
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
        let binding = grade_binding();
        let cap = Fixed::from_int(1_000_000);
        let m = |n: i32| Fixed::from_ratio(n as i64, 1_000_000); // cross-section on the m^2 scale
        let deliver = |strength: Fixed, yield_s: Fixed, modulus: Fixed| {
            let (geo, mat) = full_part_axes(strength, m(1), Fixed::from_int(1), yield_s, modulus);
            resolve_delivered_energy(&geo, &mat, &row, &binding, cap)
        };
        // A RIGID part (strength 200 MPa, no yield or modulus): the elastic path self-gates to zero, so the resolve
        // is EXACTLY the rigid `F d` (byte-neutral at the rigid limit). Cross-checked against the kinetic-only
        // accessors and the floor laws directly.
        let rigid_only = deliver(Fixed::from_int(200), Fixed::ZERO, Fixed::ZERO);
        let (kgeo, kmat) = part_axes(Fixed::from_int(200), m(1), Fixed::from_int(1));
        assert_eq!(
            rigid_only,
            resolve_delivered_energy(&kgeo, &kmat, &row, &binding, cap),
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
    fn a_hydraulic_part_delivers_its_pressure_work_which_composes_from_the_rigid_laws() {
        // Slice-4 core (the composition proof): the hydraulic member delivers `integral P dV` = `F d` with the force
        // `F = driving_pressure * cross_section`, which COMPOSES from the SAME `stress_force` + `actuator_work` the
        // rigid path uses, keyed on the FLUID driving pressure, so no new law is needed. The adversarial probe: a
        // fluid part with NO solid strength and no springy tissue (only a driving pressure) delivers a positive blow
        // a rigid-only revert reads zero, and it EQUALS the composed actuator work.
        let row = ContactTransferRegistry::dev_terran()
            .get(DEV_KINETIC)
            .expect("kinetic row")
            .clone();
        let binding = grade_binding();
        let cap = Fixed::from_int(1_000_000);
        let m = |n: i32| Fixed::from_ratio(n as i64, 1_000_000); // cross-section on the m^2 scale
                                                                 // A fluid part: a driving pressure on `fluid.driving_pressure`, a piston cross-section and a stroke, and
                                                                 // NO solid strength and NO springy tissue, so only the hydraulic member fires.
        let fluid_geo = |a: &str| match a {
            "mech.cross_section_area" => m(1),
            "mech.stroke_length" => Fixed::from_int(1),
            _ => Fixed::ZERO,
        };
        let fluid_mat = |a: &str| match a {
            // 50 MPa, an imposed hydraulic driving pressure WITHIN the floor's declared 0.0001..100 MPa range
            // (arterial is ~0.016 MPa; industrial heads reach ~100).
            "fluid.driving_pressure" => Fixed::from_int(50),
            _ => Fixed::ZERO,
        };
        let delivered = resolve_delivered_energy(&fluid_geo, &fluid_mat, &row, &binding, cap);
        // FIRST-PRINCIPLES physics check (non-tautological): the hydraulic blow equals the integral `P dV` computed
        // DIRECTLY as pressure (in pascals) times the swept volume, with no reference to the stress_force/actuator_work
        // composition. 50 MPa = 50e6 Pa over a swept volume 1e-6 m^2 * 1 m = 1e-6 m^3 is 50 J. This proves the
        // PHYSICS reduction P dV = F d, not merely that the code matches its own composition.
        let p_pascals = Fixed::from_int(50_000_000); // 50 MPa expressed in Pa
        let swept_volume = m(1).checked_mul(Fixed::from_int(1)).unwrap(); // cross-section * stroke, m^3
        let first_principles_pdv = p_pascals.checked_mul(swept_volume).unwrap(); // P dV, joules
        assert_eq!(
            delivered, first_principles_pdv,
            "the hydraulic blow equals the first-principles integral P dV (50e6 Pa * 1e-6 m^3 = 50 J)"
        );
        // And it equals the actuator work of the pressure force, the derive-first composition (the same two laws the
        // rigid path uses), corroborating that the composition reproduces the first-principles physics.
        let pressure_force = laws::stress_force(Fixed::from_int(50), m(1), cap);
        let composed = laws::actuator_work(pressure_force, Fixed::from_int(1), cap);
        assert_eq!(
            delivered, composed,
            "the hydraulic blow is the actuator work of the driving-pressure force (P dV = F d, no new law)"
        );
        assert!(
            delivered > Fixed::ZERO,
            "a fluid part with no solid strength delivers a positive hydraulic blow (a rigid-only revert reads zero): {delivered:?}"
        );
        // A stronger driving pressure delivers more (linear in pressure), keyed on the part's own fluid. 100 MPa is
        // the floor's declared upper end, still in range.
        let stronger_mat = |a: &str| match a {
            "fluid.driving_pressure" => Fixed::from_int(100),
            _ => Fixed::ZERO,
        };
        assert!(
            resolve_delivered_energy(&fluid_geo, &stronger_mat, &row, &binding, cap) > delivered,
            "a higher driving pressure delivers more hydraulic energy"
        );
        // No driving pressure: the hydraulic member self-gates, so a non-fluid actuator reads zero from it.
        let dry_mat = |_: &str| Fixed::ZERO;
        assert_eq!(
            resolve_delivered_energy(&fluid_geo, &dry_mat, &row, &binding, cap),
            Fixed::ZERO,
            "no driving pressure, no strength, no tissue: no blow (run-all-gate-to-zero)"
        );
        // Deterministic (Principle 3).
        assert_eq!(
            delivered,
            resolve_delivered_energy(&fluid_geo, &fluid_mat, &row, &binding, cap)
        );
    }

    #[test]
    fn the_delivery_reads_the_grade_laws_binding_as_the_one_source() {
        // Slice C, the structural single source: the delivery row does NOT own a binding, it names the grade law
        // whose binding it shares, and the resolve reads THAT binding. So there is one map, and a grade/delivery
        // mapping desync is impossible by construction (no second copy exists to diverge). This test reads the
        // delivered energy through the grade registry's own IMPACT binding, the exact value the strike wire fetches.
        let reg = ContactTransferRegistry::dev_terran();
        let row = reg.get(DEV_KINETIC).expect("kinetic row").clone();
        assert_eq!(row.grade_law, FunctionLawRegistry::ID_IMPACT);
        // The grade registry is the SINGLE SOURCE; the delivery resolve reads its IMPACT binding by the row's
        // grade_law, exactly as `Embodiment::strike_*` does.
        let grade_laws = FunctionLawRegistry::dev_seed();
        let binding = &grade_laws
            .get(row.grade_law)
            .expect("the row's grade_law resolves in the grade registry")
            .binding;
        let cap = Fixed::from_int(1_000_000);
        let (geo, mat) = part_axes(
            Fixed::from_int(200),
            Fixed::from_ratio(1, 1_000_000),
            Fixed::from_int(1),
        );
        // The blow the shared binding yields is the actuator work of the part's own axes read through the grade
        // law's map, 200 N over 1 m: no per-path binding, one source.
        let force = laws::stress_force(Fixed::from_int(200), Fixed::from_ratio(1, 1_000_000), cap);
        assert_eq!(
            resolve_delivered_energy(&geo, &mat, &row, binding, cap),
            laws::actuator_work(force, Fixed::from_int(1), cap),
            "the delivery reads the grade law's binding directly (the single source)"
        );
    }

    #[test]
    fn a_dangling_grade_law_reference_fails_loud_at_registry_validate() {
        // Slice C fail-loud load error: a delivery row naming a grade_law that is NOT in the grade registry is a
        // dangling reference, caught at the wire boundary (`Embodiment::set_contact_transfer` runs this validate).
        // With no second binding to diverge, the only load failure is an unresolved reference, so the grade/delivery
        // lockstep is a load-time impossibility rather than a runtime misread.
        let grade_laws = FunctionLawRegistry::dev_seed();
        // The canonical fixture validates clean: its grade_law (IMPACT) resolves.
        assert!(ContactTransferRegistry::dev_terran()
            .validate(&grade_laws)
            .is_ok());
        // A row naming a grade law id absent from the registry fails validate, naming the offending channel.
        let mut reg = ContactTransferRegistry::empty();
        reg.insert(ContactTransfer::new(
            DEV_KINETIC,
            TransferKernel::Kinetic,
            FunctionLawId(9999),
        ));
        let err = reg
            .validate(&grade_laws)
            .expect_err("a dangling grade_law reference fails validate");
        assert!(
            err.contains("grade_law") && err.contains(&DEV_KINETIC.0.to_string()),
            "the validate error names the offending channel and its dangling reference: {err}"
        );
    }
}
