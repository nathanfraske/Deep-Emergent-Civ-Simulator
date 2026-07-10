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

//! The function-law dispatch: derive a part's FUNCTION from its own geometry and material through the
//! physics floor, never from a label (emergent-anatomy arc, step one).
//!
//! The existing leaf ([`crate::eval::eval_leaf`]) already reads a designed artifact's STRUCTURAL
//! viability from physics over geometry and material, blind to any substance name. This module extends
//! the same move to CAPABILITY: a part is a weapon because a hard point at small contact area drives a
//! cut into a reference target, a sense because a transducer couples a field, locomotion because a vane
//! yields thrust, read from the part's own geometry and material by running the floor laws, never from
//! an authored `F_STRIKE`/`F_SIGHT` tag or a catalog kind. The name a part carries is then an emergent
//! read of the winning physics the way [`civsim_biosphere`-style trophic labels] are read from what a
//! thing eats, or a pure cosmetic with no behavioural read.
//!
//! The dispatch follows the physics-registry pattern the [`crate::combinator::CombinatorRegistry`] and
//! [`crate::proxy::ProxyRegistry`] use: the kernel set ([`CapabilityKernel`]) is fixed Rust, closed and
//! `Fixed`-only, and the MEMBERSHIP (which function laws a world runs) is data in a
//! [`FunctionLawRegistry`] that grows with the world (Principle 11). A kernel reads only geometry-axis
//! and material-axis values by id (never a substance name, a kind id, or a race id), plus the
//! reserved-with-basis reference levels the caller supplies from the manifest ([`CapabilityRefs`],
//! fail-loud while reserved, never fabricated in this crate). Every read is a pure fixed-point function,
//! no float and no RNG, so a capability is a deterministic read of grown physics.
//!
//! Step one seeds the one PIERCE law (the weapon read, retiring `F_STRIKE`); the registry is built to
//! grow to the resonance (voice and ear), refraction (sight), aerodynamic (glide), insulation
//! (covering), and respiration laws whose kernels the physics floor already carries, each a data entry
//! plus its reserved references, never a new authored branch.

use crate::interval::sat_sub;
use civsim_core::Fixed;
use civsim_physics::{laws, AxisRange, Dimension, PhysicsRegistry, QuantityAxis};
use std::collections::BTreeMap;

/// A function-law id: a stable handle for one function class, so a law survives the registry growing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionLawId(pub u32);

/// The closed set of function-class kernels, fixed Rust. Each reads a part's geometry and material
/// through the physics floor and returns a dimensionless capability in `[0, 1]`: zero means the part
/// cannot perform the function (its geometry or material does not clear the physics), one means it
/// performs it as well as the reserved reference. The set grows only by the owner adding a variant bound
/// to a physics kernel the floor already carries, never by a per-kind or per-race branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityKernel {
    /// PIERCE, the weapon read (retires `F_STRIKE`). A reference striking force concentrated over the
    /// part's contact area is a contact pressure, bounded by the part's own indentation hardness (a
    /// point softer than the pressure it would carry blunts before it reaches it), and if that effective
    /// pressure drives a cut into a reference target through [`laws::cut_penetrate`] the part is a
    /// weapon, graded by the penetration depth. Reads `mech.contact_area` and `mat.indentation_hardness`;
    /// the strike force, the target hardness and cut energy, the delivered energy, and the reference
    /// depth are reserved. A blunt point (large area, low pressure) or a soft one (low hardness, capped
    /// pressure) does not clear the target hardness and reads zero: not a weapon, by physics not by tag.
    Pierce,
    /// LOCOMOTE, the limb read (retires `F_LOCOMOTION` and the `MorphCategory::Locomotion` gate). A limb
    /// bears a reference propulsive load without structural failure: the bending stress the load raises
    /// over the limb's section modulus and length ([`laws::bend_stress`]) stays below the material's yield,
    /// so the limb can push off rather than buckle. The capability is one minus the bending utilization (a
    /// limb far from yield is a strong locomotor, a slender or weak one near yield reads low). Reads
    /// `mech.section_modulus`, `mech.arm_length`, and `mat.yield_strength`; the reference locomotor load is
    /// reserved. A part carrying no section modulus (an organ, a hide) bears no load and reads zero: not a
    /// limb, by physics not by tag.
    Locomote,
    /// REFRACT, the optical-sense read (retires `F_SIGHT` and the `MorphCategory::Sense` gate for the
    /// visual channel). A light-transducing tissue focuses light when its refractive index exceeds the
    /// medium's ([`laws::refractive_contrast`]), so a contrast above the medium is an eye. The capability
    /// is the refractive contrast over the reference. Reads `opt.refractive_index`; the medium index and
    /// the reference contrast are reserved. A tissue matching the medium (no contrast) does not focus and
    /// reads zero. The honest limit: this is the optical channel only; the acoustic, chemical, and field
    /// senses are their own kernels (a documented follow-on), so a non-optical sense reads zero here.
    Refract,
    /// SHEAR, the sever read (the made-world arc, tool-use, root R1): the first NON-PIERCING action kernel,
    /// so an affordance can require a shear law rather than only the pierce one, opening the sever/divide
    /// action space. An edge pressing a reference force over its contact area imposes a shear stress
    /// ([`laws::shear`]), bounded by the part's OWN shear strength (a part cannot deliver a shear beyond what
    /// it withstands before it shears itself, an isotropic ductile part deriving that limit from its yield by
    /// the von Mises ratio). If that effective shear clears a reference target's shear resistance the part can
    /// sever, graded by the effective shear over the reference. Reads `mech.contact_area`, `mat.shear_strength`,
    /// and `mat.yield_strength`; the reference force and the reference shear resistance are reserved. A part
    /// with no edge (no contact area) or no shear strength (and no yield to derive one) delivers no shear and
    /// reads zero: not a sever tool, by physics not a tag. Where PIERCE measures a normal-stress penetration,
    /// SHEAR measures the tangential parting a cut physically is, so a cut affordance gates on this rather than
    /// the pierce proxy.
    Shear,
    /// CRUSH, the compressive-failure read (the made-world arc, tool-use, Section G): the second non-piercing
    /// action, so a blunt strong tool can afford a crush a keen edge is not distinguished for. A part pressing
    /// a reference force over its contact area imposes a compressive stress ([`laws::contact_pressure`]),
    /// bounded by the part's OWN compressive strength (a part cannot deliver a compressive stress beyond what
    /// it withstands before it crushes itself). If that effective stress clears a reference target's
    /// compressive resistance the part can crush, graded over the reference. Reads `mech.contact_area` and
    /// `mat.compressive_strength`; the reference force and the reference compressive resistance are reserved. A
    /// part with no face (no contact area) or no compressive strength delivers nothing and reads zero. Where
    /// SHEAR measures the tangential parting a cut is, CRUSH measures the compressive failure a hammer or a
    /// mill is: a material weak in compression but tough in shear is crushed, not cut, and one weak in shear
    /// but strong in compression is cut, not crushed, diverging on the target's own resistance axes by physics
    /// not a tag.
    Crush,
    /// IMPACT, the percussion read (the made-world arc, tool-use, Section G): a part's ACTUATOR WORK delivers a
    /// blow. Its actuating force (its strength stress `mat.fracture_strength` over its cross-section
    /// `mech.cross_section_area`, promoted to newtons by [`laws::stress_force`]) over its own grown stroke
    /// `mech.stroke_length` is the energy it delivers ([`laws::actuator_work`], `F d`, a joule). If that clears a
    /// reference strike energy the part is a percussion tool, graded over the reference. So a STRONG, thick,
    /// long-stroked part reads a high impact where a weak or short-stroked one reads none, the distinction
    /// derived from the part's own body rather than a world-global swing speed (the stroke-rate substrate). Reads
    /// `mat.fracture_strength`, `mech.cross_section_area`, `mech.stroke_length`; the reference strike energy is
    /// reserved. A part with no actuating strength or no grown stroke delivers no blow and reads zero.
    Impact,
}

impl CapabilityKernel {
    /// The geometry axes this kernel reads, documenting its input surface. The read itself gates on axis
    /// PRESENCE through the zero-for-absent accessor (a part carrying no contact area reads zero area and
    /// so zero capability), so this list is the declared contract, not a runtime guard.
    pub fn geometry_axes(self) -> &'static [&'static str] {
        match self {
            CapabilityKernel::Pierce => &["mech.contact_area"],
            CapabilityKernel::Locomote => &["mech.section_modulus", "mech.arm_length"],
            CapabilityKernel::Refract => &[],
            CapabilityKernel::Shear => &["mech.contact_area"],
            CapabilityKernel::Crush => &["mech.contact_area"],
            CapabilityKernel::Impact => &["mech.cross_section_area", "mech.stroke_length"],
        }
    }

    /// The material axes this kernel reads.
    pub fn material_axes(self) -> &'static [&'static str] {
        match self {
            CapabilityKernel::Pierce => &["mat.indentation_hardness"],
            CapabilityKernel::Locomote => &["mat.yield_strength"],
            CapabilityKernel::Refract => &["opt.refractive_index"],
            CapabilityKernel::Shear => &["mat.shear_strength", "mat.yield_strength"],
            CapabilityKernel::Crush => &["mat.compressive_strength"],
            CapabilityKernel::Impact => &["mat.fracture_strength"],
        }
    }

    /// The dimensionless capability in `[0, 1]` the part's geometry and material yield for this function
    /// against the reserved references. A pure fixed-point read: no float, no id, no RNG, so a capability
    /// is a deterministic function of the grown physics and the reserved references.
    /// `geometry_axes` and `material_axes` are the law's DATA-declared axis-id bindings (from its
    /// [`FunctionLawDef`] row). The IMPACT kernel reads its axes from them (the grade-path parallel of the
    /// delivery-path contact-transfer row); the other kernels read their hardcoded contract and ignore the
    /// lists (the flagged follow-on lifts them the same way).
    pub fn capability(
        self,
        geo: &dyn Fn(&str) -> Fixed,
        mat: &dyn Fn(&str) -> Fixed,
        refs: &CapabilityRefs,
        caps: &CapabilityCaps,
        geometry_axes: &[String],
        material_axes: &[String],
    ) -> Fixed {
        match self {
            CapabilityKernel::Pierce => pierce(geo, mat, refs, caps),
            CapabilityKernel::Locomote => locomote(geo, mat, refs, caps),
            CapabilityKernel::Refract => refract(mat, refs),
            CapabilityKernel::Shear => shear(geo, mat, refs, caps),
            CapabilityKernel::Crush => crush(geo, mat, refs, caps),
            CapabilityKernel::Impact => impact(geo, mat, refs, geometry_axes, material_axes),
        }
    }
}

/// The LOCOMOTE read: is the part a load-bearing limb, and how strong a one, from its geometry and material.
fn locomote(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> Fixed {
    let section_modulus = geo("mech.section_modulus");
    let arm_length = geo("mech.arm_length");
    let yield_strength = mat("mat.yield_strength");
    if section_modulus <= Fixed::ZERO || yield_strength <= Fixed::ZERO {
        return Fixed::ZERO; // no section to bear a load, or no strength: not a limb
    }
    // The bending stress the reference propulsive load raises over the limb's section and length, and the
    // margin to yield. A limb whose stress stays well below yield bears the load and can push off; one
    // near or past yield buckles. The capability is one minus the bending utilization (the safety fraction).
    let (sigma, _margin) = laws::bend_stress(
        refs.reference_locomotor_load,
        section_modulus,
        arm_length,
        yield_strength,
        caps.pressure,
    );
    // Utilization sigma/yield, capability = 1 - utilization clamped to [0, 1]. A stress at or past yield
    // reads zero (the limb fails), a stress far below reads near one (a strong locomotor).
    match sigma.checked_div(yield_strength) {
        Some(util) => sat_sub(Fixed::ONE, util).clamp(Fixed::ZERO, Fixed::ONE),
        None => Fixed::ZERO,
    }
}

/// The REFRACT read: is the tissue an optical transducer (an eye), from its refractive index against the
/// medium. Material-only (a lens is a material property); no geometry axis is read.
fn refract(mat: &dyn Fn(&str) -> Fixed, refs: &CapabilityRefs) -> Fixed {
    let n2 = mat("opt.refractive_index");
    if n2 <= Fixed::ZERO {
        return Fixed::ZERO; // no optical tissue: not an eye
    }
    let (contrast, _tir) =
        laws::refractive_contrast(refs.medium_refractive_index, n2, refs.optical_contrast_cap);
    // A contrast at or below one (matching or thinner than the medium) does not focus; the capability is
    // the contrast above the medium, over the reference contrast that reads as a fully capable eye.
    normalize(
        sat_sub(contrast, Fixed::ONE),
        refs.reference_optical_contrast,
    )
}

/// The PIERCE read: is the part a weapon, and how good a one, from its own geometry and material.
fn pierce(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> Fixed {
    let contact_area = geo("mech.contact_area");
    if contact_area <= Fixed::ZERO {
        return Fixed::ZERO; // no tip, no contact: not a weapon
    }
    // The pressure the reference strike force concentrates over the tip, bounded by the part's own
    // material: a part cannot sustain a contact pressure above its own indentation hardness before it
    // plastically blunts, so a soft point caps out low and cannot exceed a hard target's resistance.
    let applied = laws::contact_pressure(refs.reference_strike_force, contact_area, caps.pressure);
    let hardness = mat("mat.indentation_hardness");
    let effective = if hardness > Fixed::ZERO {
        applied.min(hardness)
    } else {
        applied
    };
    // The cut depth into the reference target: zero unless the effective pressure clears the target
    // hardness (the penetration law's own gate), then graded by the delivered energy over the swept
    // resistance. A depth above the reserved reference reads as a fully capable weapon.
    let depth = laws::cut_penetrate(
        effective,
        refs.target_hardness,
        refs.reference_delivered_energy,
        refs.target_specific_cut_energy,
        contact_area,
        caps.depth,
    );
    normalize(depth, refs.reference_penetration_depth)
}

/// The CUT capability of a part's edge against a SPECIFIC target material (material-substrate arc, cascade
/// item 4, crafting, the load-bearing seam). It runs the same contact-pressure-into-penetration contest
/// [`pierce`] does, but the resistance it must defeat is read from the TARGET's own material axes
/// (`mat.indentation_hardness`, `mat.specific_cut_energy`) rather than the one global reference target
/// [`pierce`] measures against. So the same edge cuts soft hide well and hard granite poorly, and a harder
/// sharper edge parts stone a softer blunter one cannot, diverging on the target's substance DATA alone,
/// never a per-material branch: mining flint versus granite and cutting hide versus wood become one
/// contest over different target rows. The part's own `mat.indentation_hardness` still caps the pressure a
/// soft edge sustains before it blunts, so a soft tool cannot exceed a hard target's resistance however
/// sharp; the strike force, delivered energy, and reference depth stay reserved.
///
/// A target axis the substance does not carry reads zero (the absence convention): a target with no
/// declared hardness or cutting energy offers no resistance and cuts fully, the same zero-for-absent rule
/// the leaf read holds. A pure fixed-point read, opt-in beside [`derive_capabilities`] so declaring it
/// changes no existing capability score. The follow-on that folds CUT into [`FunctionLawRegistry`] as a
/// data-bound kernel (retiring [`pierce`]'s global reference target) lands when a consumer reads it.
pub fn cut_capability_against_target(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    target: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> Fixed {
    let contact_area = geo("mech.contact_area");
    if contact_area <= Fixed::ZERO {
        return Fixed::ZERO; // no edge, no contact: nothing to cut with
    }
    let applied = laws::contact_pressure(refs.reference_strike_force, contact_area, caps.pressure);
    let hardness = mat("mat.indentation_hardness");
    let effective = if hardness > Fixed::ZERO {
        applied.min(hardness)
    } else {
        applied
    };
    // The penetration into THIS target: zero unless the effective pressure clears the target's own
    // hardness, then graded by the delivered energy over the target's own cutting energy. Both are read
    // from the target closure, so the divergence between targets is substance data, not code.
    let depth = laws::cut_penetrate(
        effective,
        target("mat.indentation_hardness"),
        refs.reference_delivered_energy,
        target("mat.specific_cut_energy"),
        contact_area,
        caps.depth,
    );
    normalize(depth, refs.reference_penetration_depth)
}

/// The SHEAR read: is the part a sever tool, and how good a one, from its edge geometry and shear
/// strength (the made-world arc, root R1, the first non-piercing action). The reference force over the
/// edge's contact area is a shear stress ([`laws::shear`]), bounded by the part's OWN shear strength
/// (an independent shear strength if the material declares one, else the von Mises reduction of its
/// yield): a part cannot deliver a shear beyond what it withstands before it shears itself. The
/// effective deliverable shear (the applied shear capped at the part's own strength) over the reserved
/// reference shear resistance is the capability. A keen strong edge delivers a high shear and reads high;
/// a blunt one (large area, low stress) or a weak one (low shear strength, capped) reads low; a part with
/// no edge or no shear strength reads zero.
fn shear(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> Fixed {
    let contact_area = geo("mech.contact_area");
    if contact_area <= Fixed::ZERO {
        return Fixed::ZERO; // no edge, no contact: nothing to shear with
    }
    let shear_strength = mat("mat.shear_strength");
    let yield_strength = mat("mat.yield_strength");
    let independent = if shear_strength > Fixed::ZERO {
        Some(shear_strength)
    } else {
        None // an isotropic ductile part derives its shear strength from yield (von Mises), inside the law
    };
    // The applied shear the reference force imposes over the edge, and the margin to the part's own shear
    // strength. The part self-limits: it delivers at most its own shear strength before it shears itself, so
    // the effective shear is the applied capped at that strength (applied + the margin where the margin is
    // negative, i.e. where the applied exceeds the part's own strength). A part with neither an independent
    // shear strength nor a yield reads a zero material strength, so its effective shear cancels to zero.
    let (tau_applied, margin) = laws::shear(
        refs.reference_strike_force,
        contact_area,
        independent,
        yield_strength,
        caps.pressure,
    );
    let effective = tau_applied + margin.min(Fixed::ZERO);
    // The sever threshold: a part that cannot drive its effective shear PAST the reference target's shear
    // resistance cannot part it and reads zero (the shear sibling of PIERCE's clear-the-target gate, so a
    // weak sliver that self-limits below the reference is no sever tool). Above the threshold the capability
    // grades by how far the effective shear exceeds the reference, over one reference of excess as a full
    // parting, exactly the shape the REFRACT read uses for its own contrast-above-the-medium.
    normalize(
        sat_sub(effective, refs.reference_shear_resistance),
        refs.reference_shear_resistance,
    )
}

/// The CRUSH read: is the part a compressive-failure tool, and how good a one, from its face geometry and
/// compressive strength (the made-world arc, Section G, the second non-piercing action). The reference force
/// over the face's contact area is a compressive stress ([`laws::contact_pressure`]), bounded by the part's
/// OWN compressive strength (it crushes itself before it delivers more). The effective deliverable stress
/// (the applied stress capped at the part's own strength) must clear the reserved reference compressive
/// resistance to crush, graded above the threshold, the compressive sibling of the SHEAR read. A strong tool
/// crushes; a weak one (self-limited low), a spread one (low stress), or a part with no face or no
/// compressive strength reads zero.
fn crush(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> Fixed {
    let contact_area = geo("mech.contact_area");
    if contact_area <= Fixed::ZERO {
        return Fixed::ZERO; // no face, no contact: nothing to crush with
    }
    let compressive_strength = mat("mat.compressive_strength");
    // The applied compressive stress the reference force imposes over the face, self-limited at the part's
    // own compressive strength (a part that would carry more than it withstands crushes itself first). A part
    // with no compressive strength (zero) delivers no crushing stress.
    let applied = laws::contact_pressure(refs.reference_strike_force, contact_area, caps.pressure);
    let effective = if compressive_strength > Fixed::ZERO {
        applied.min(compressive_strength)
    } else {
        Fixed::ZERO
    };
    normalize(
        sat_sub(effective, refs.reference_compressive_resistance),
        refs.reference_compressive_resistance,
    )
}

/// The IMPACT read: is the part a percussion tool, and how good a one, from its ACTUATOR WORK (the made-world
/// arc, Section G). The part's actuating force (its strength stress `mat.fracture_strength` over its
/// cross-section `mech.cross_section_area`, an N) over its own grown stroke `mech.stroke_length` is the energy
/// it delivers ([`laws::actuator_work`], `F d`, a J); if that clears the reserved reference strike energy (on
/// the same joule scale) the part strikes, graded above the threshold. A STRONG, thick, long-stroked part reads
/// a high impact where a weak or short-stroked one reads none, the distinction derived from the part's own body
/// rather than a world-global swing speed. A part with no actuating strength or no stroke reads zero.
fn impact(
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    geometry_axes: &[String],
    material_axes: &[String],
) -> Fixed {
    // The actuating-strength, cross-section, and stroke axes are read from the law's DATA-declared bindings (the
    // grade-path parallel of the delivery-path contact-transfer row), so an alien actuator names its own axes on
    // both paths in lockstep: the strength is material-axis 0, the cross-section geometry-axis 0, the stroke
    // geometry-axis 1 (the order the kernel's contract declares them). A binding that names no such axis reads
    // zero through the accessor, so the part self-gates (the absence convention), never a hardcoded id and never
    // a fabricated blow.
    let strength = material_axes.first().map(|a| mat(a)).unwrap_or(Fixed::ZERO);
    let cross_section = geometry_axes.first().map(|a| geo(a)).unwrap_or(Fixed::ZERO);
    let stroke = geometry_axes.get(1).map(|a| geo(a)).unwrap_or(Fixed::ZERO);
    // The actuating force in newtons (strength stress over cross-section, promoted by the megapascal-to-newton
    // bridge), then the actuator work over the grown stroke. Passing the force through `actuator_work` rather
    // than short-circuiting on overflow keeps the stroke guard live: a part with no grown stroke reads zero even
    // when its force would overflow (a representability corner, not a full-impact ceiling).
    let force = laws::stress_force(strength, cross_section, ENERGY_GUARD);
    let delivered = laws::actuator_work(force, stroke, ENERGY_GUARD);
    normalize(
        sat_sub(delivered, refs.reference_strike_energy),
        refs.reference_strike_energy,
    )
}

/// Normalize a raw physics reading to `[0, 1]` against a reserved reference level (the reading that
/// counts as full capability). A non-positive reference reads zero (an unset reference offers no
/// capability rather than a fabricated one); an overflow in the division reads full.
fn normalize(value: Fixed, reference: Fixed) -> Fixed {
    if reference <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    match value.checked_div(reference) {
        Some(r) => r.clamp(Fixed::ZERO, Fixed::ONE),
        None => Fixed::ONE,
    }
}

/// The reserved-with-basis reference levels the capability kernels read, supplied by the caller from the
/// calibration manifest (fail-loud while reserved), never fabricated in this crate. These are the
/// physical stand-ins a capability is measured against: what a reference strike delivers and what a
/// reference target resists, so a part's function is read relative to the world it must act in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRefs {
    /// The reference striking force a part concentrates over its contact area (`capability.strike_force`,
    /// N). Basis: the force a muscle-driven strike delivers at the whole-body muscle-force scale
    /// ([`whole_body_muscle_force`]); larger reads more parts as weapons, smaller fewer.
    pub reference_strike_force: Fixed,
    /// The indentation hardness of the reference target a weapon must defeat (`capability.target_hardness`,
    /// MPa). Basis: the Vickers/Brinell hardness of the reference prey's hide or tissue (the
    /// `mat.indentation_hardness` value for biological soft tissue), the resistance the cut must exceed.
    pub target_hardness: Fixed,
    /// The specific cutting energy of the reference target (`capability.target_cut_energy`, MJ/m^3).
    /// Basis: the energy to part unit volume of the reference target tissue (`mat.specific_cut_energy`,
    /// soft-tissue low end), which sets how far a given delivered energy cuts.
    pub target_specific_cut_energy: Fixed,
    /// The kinetic energy a reference strike delivers (`capability.strike_energy`, J). Basis: the
    /// half-mass-times-velocity-squared of a muscle-driven strike at a reference limb velocity.
    pub reference_delivered_energy: Fixed,
    /// The penetration depth that counts as a fully capable weapon (`capability.reference_depth`, m).
    /// Basis: a wound depth that incapacitates the reference target, a fraction of its characteristic
    /// dimension; the capability is the achieved depth over this reference, clamped to one.
    pub reference_penetration_depth: Fixed,
    /// The reference propulsive load a limb bears in locomotion (`capability.locomotor_load`, N). Basis:
    /// the ground-reaction force a limb carries in a stride at the body-mass scale (a multiple of body
    /// weight); larger reads fewer limbs as strong locomotors, smaller more.
    pub reference_locomotor_load: Fixed,
    /// The refractive index of the medium an optical sense focuses against (`capability.medium_index`,
    /// dimensionless). Basis: the medium's `opt.refractive_index` (near one for air or vacuum, ~1.33 for
    /// water), the index a lens tissue must exceed to bend light.
    pub medium_refractive_index: Fixed,
    /// The refractive-contrast saturation ceiling the optical law caps at (`capability.optical_contrast_cap`,
    /// dimensionless). Basis: the largest `opt.refractive_index` ratio the floor represents.
    pub optical_contrast_cap: Fixed,
    /// The refractive contrast above the medium that counts as a fully capable eye
    /// (`capability.reference_optical_contrast`, dimensionless). Basis: the lens-to-medium index step a
    /// focusing eye needs; the capability is the achieved contrast above one, over this reference.
    pub reference_optical_contrast: Fixed,
    /// The shear resistance a sever tool must overcome to read as fully capable (`capability.reference_shear`,
    /// MPa). Basis: the `mat.shear_strength` of the reference target a cut parts (the soft-tissue or fibre
    /// shear strength a sever must exceed), the sibling of `target_hardness` for the shear kernel; the
    /// capability is the edge's deliverable shear over this reference, clamped to one. Larger reads fewer
    /// edges as sever tools, smaller more; surfaced reserved-with-basis, never fabricated.
    pub reference_shear_resistance: Fixed,
    /// The compressive resistance a crush tool must overcome to read as fully capable
    /// (`capability.reference_compression`, MPa). Basis: the `mat.compressive_strength` of the reference
    /// target a crush must fail (the compressive strength of the matter a hammer or a mill breaks), the
    /// compressive sibling of `reference_shear_resistance`; the capability is the face's deliverable stress
    /// over this reference, clamped to one. Surfaced reserved-with-basis, never fabricated.
    pub reference_compressive_resistance: Fixed,
    /// The reference STRIKE ENERGY a percussion tool must deliver to read as fully capable
    /// (`capability.strike_energy`, on the JOULE scale the actuator-work law reports, `F d`). Basis: the energy
    /// that fractures the reference target (its Griffith energy over the struck area), the energy a fully-capable
    /// blow lands; the capability is the tool's delivered actuator work over this reference, clamped to one.
    /// Surfaced reserved-with-basis, never fabricated.
    pub reference_strike_energy: Fixed,
}

impl CapabilityRefs {
    /// A labelled DEV FIXTURE standing up physically plausible references inside the floor axis ranges,
    /// for the compose tests and any harness that derives a capability without a manifest. NOT
    /// owner-authored calibration: the running engine supplies these fail-loud from the reserved
    /// `capability.*` manifest homes named on each field. A soft blunt part reads zero and a hard sharp
    /// one reads a weapon under these references, so the derive-not-tag thesis is exercised.
    pub fn dev_refs() -> CapabilityRefs {
        CapabilityRefs {
            reference_strike_force: dec("100"), // N, a moderate muscle-driven strike
            target_hardness: dec("5"),          // MPa, biological hide/soft tissue
            target_specific_cut_energy: dec("2"), // MJ/m^3, soft-tissue cutting energy
            reference_delivered_energy: dec("1"), // J, a strike's kinetic energy
            reference_penetration_depth: dec("0.01"), // m, a one-centimetre incapacitating wound
            reference_locomotor_load: dec("50"), // N, a stride ground-reaction load
            medium_refractive_index: dec("1"),  // air/vacuum, the medium a lens focuses against
            optical_contrast_cap: dec("10"),    // the refractive-contrast ceiling
            reference_optical_contrast: dec("0.3"), // a lens-to-air index step that focuses (n~1.3)
            reference_shear_resistance: dec("3"), // MPa, soft-tissue/fibre shear strength a sever parts
            reference_compressive_resistance: dec("5"), // MPa, the compressive strength a crush must fail
            reference_strike_energy: dec("100"), // J, the actuator work a fully-capable blow lands
        }
    }
}

/// The physics saturation ceilings the capability kernels pass to the floor laws, derived from the
/// registry's own axis ranges exactly as [`crate::eval`] derives its caps, so no ceiling is fabricated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityCaps {
    /// The pressure ceiling (the largest pressure-dimension axis hi).
    pub pressure: Fixed,
    /// The depth (length) ceiling (the largest length-dimension axis hi).
    pub depth: Fixed,
}

impl CapabilityCaps {
    /// Derive the ceilings from the physics registry's set axis ranges (a reserved-range axis contributes
    /// none, so a partly-reserved floor still yields a ceiling from its set axes).
    pub fn derive(reg: &PhysicsRegistry) -> CapabilityCaps {
        CapabilityCaps {
            pressure: dim_cap(reg, Dimension::PRESSURE),
            depth: dim_cap(reg, Dimension::LENGTH),
        }
    }
}

/// The energy overflow-guard the IMPACT read passes to the kinetic-energy law (a PURE representability cap,
/// not a behavioural ceiling): far above any muscle-driven blow's kilojoule-scale energy yet clear of the
/// Q32.32 maximum, so a heavy tool's delivered energy saturates safely rather than wrapping. Sibling to the
/// runner's stress guard; the impact capability is bounded by the reference strike energy regardless.
const ENERGY_GUARD: Fixed = Fixed::from_int(1_000_000_000);

fn dim_cap(reg: &PhysicsRegistry, dim: Dimension) -> Fixed {
    reg.axes()
        .filter(|a| a.dimension == dim)
        .filter_map(axis_hi)
        .max()
        .unwrap_or(Fixed::MAX)
}

fn axis_hi(a: &QuantityAxis) -> Option<Fixed> {
    match &a.range {
        AxisRange::Set { hi, .. } => Some(*hi),
        AxisRange::Reserved { .. } => None,
    }
}

/// One function-law entry: an id, a human label, and the kernel it computes. The label is a name for the
/// law, never a per-part authored tag; the part's own function is the derived score, not this string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLawDef {
    /// The law's stable id.
    pub id: FunctionLawId,
    /// The human-readable name of the function class.
    pub name: String,
    /// The kernel it computes.
    pub kernel: CapabilityKernel,
    /// The physics-floor GEOMETRY axis ids the law's kernel reads, as DATA (Principle 11): the grade-path
    /// parallel of the delivery-path contact-transfer row (which carries its `cross_section_axis`/`stroke_axis`
    /// on the row), so an alien actuator names its own axes on BOTH the capability grade and the delivered-energy
    /// paths, in lockstep, never a rewrite of one while the other is data. The IMPACT kernel reads its
    /// cross-section (entry 0) and stroke (entry 1) from here; the other kernels still read their hardcoded
    /// default contract (a flagged follow-on to lift them the same way), so a def whose binding equals
    /// `kernel.geometry_axes()`, as [`FunctionLawDef::new`] populates it, reads byte-identically.
    pub geometry_axes: Vec<String>,
    /// The physics-floor MATERIAL axis ids the law's kernel reads, as DATA, the material sibling of
    /// [`Self::geometry_axes`]. The IMPACT kernel reads its actuating-strength axis from the first entry.
    pub material_axes: Vec<String>,
}

impl FunctionLawDef {
    /// A law entry whose axis bindings are the kernel's own declared contract, the byte-neutral default: the
    /// data-carried axis ids equal [`CapabilityKernel::geometry_axes`] / [`CapabilityKernel::material_axes`], so
    /// a def built this way reads exactly as the hardcoded kernel did. An alien registry overrides the axis ids
    /// by constructing the def with its own lists (the harden-to-registry contract, Principle 11).
    pub fn new(
        id: FunctionLawId,
        name: impl Into<String>,
        kernel: CapabilityKernel,
    ) -> FunctionLawDef {
        FunctionLawDef {
            id,
            name: name.into(),
            kernel,
            geometry_axes: kernel
                .geometry_axes()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            material_axes: kernel
                .material_axes()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// The function-law catalogue. Ordered by id so every walk is deterministic. The structure is fixed
/// Rust; the membership is data that grows with the world (Principle 11).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FunctionLawRegistry {
    defs: BTreeMap<u32, FunctionLawDef>,
}

impl FunctionLawRegistry {
    /// The stable id of the PIERCE law in [`Self::dev_seed`].
    pub const ID_PIERCE: FunctionLawId = FunctionLawId(0);
    /// The stable id of the LOCOMOTE law in [`Self::dev_seed`].
    pub const ID_LOCOMOTE: FunctionLawId = FunctionLawId(1);
    /// The stable id of the REFRACT law in [`Self::dev_seed`].
    pub const ID_REFRACT: FunctionLawId = FunctionLawId(2);
    /// The stable id of the SHEAR law in [`Self::dev_seed`] (the first non-piercing action, root R1).
    pub const ID_SHEAR: FunctionLawId = FunctionLawId(3);
    /// The stable id of the CRUSH law in [`Self::dev_seed`] (the second non-piercing action, compression).
    pub const ID_CRUSH: FunctionLawId = FunctionLawId(4);
    /// The stable id of the IMPACT law in [`Self::dev_seed`] (the percussion read, the mass payoff).
    pub const ID_IMPACT: FunctionLawId = FunctionLawId(5);

    /// An empty registry.
    pub fn new() -> Self {
        FunctionLawRegistry::default()
    }

    /// Add a law. Returns the id.
    pub fn insert(&mut self, def: FunctionLawDef) -> FunctionLawId {
        let id = def.id;
        self.defs.insert(id.0, def);
        id
    }

    /// A law by id.
    pub fn get(&self, id: FunctionLawId) -> Option<&FunctionLawDef> {
        self.defs.get(&id.0)
    }

    /// The laws, in id order.
    pub fn defs(&self) -> impl Iterator<Item = &FunctionLawDef> + '_ {
        self.defs.values()
    }

    /// Number of laws.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// A labelled DEV SEED wiring the step-one function laws (PIERCE the weapon read, LOCOMOTE the limb
    /// read, REFRACT the optical-sense read). Not owner-authored production content: a stand-in so the
    /// derive-from-physics read can be exercised. The membership grows as the owner adds the acoustic,
    /// aerodynamic, insulation, chemoreception, and respiration laws.
    pub fn dev_seed() -> Self {
        let mut reg = FunctionLawRegistry::new();
        // Each law's axis bindings default to its kernel's own contract (`FunctionLawDef::new`), so the IMPACT
        // read (now off the data bindings, the delivery-path parallel) is byte-identical to the hardcoded form.
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_PIERCE,
            "pierce",
            CapabilityKernel::Pierce,
        ));
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_LOCOMOTE,
            "locomote",
            CapabilityKernel::Locomote,
        ));
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_REFRACT,
            "refract",
            CapabilityKernel::Refract,
        ));
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_SHEAR,
            "shear",
            CapabilityKernel::Shear,
        ));
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_CRUSH,
            "crush",
            CapabilityKernel::Crush,
        ));
        reg.insert(FunctionLawDef::new(
            FunctionLawRegistry::ID_IMPACT,
            "impact",
            CapabilityKernel::Impact,
        ));
        reg
    }
}

/// A part's derived function: the capability score on each function law, keyed by law id (sorted, for a
/// deterministic walk). Never stored on the part; a pure read of its geometry and material, recomputed
/// or cached by content id the way the leaf eval is (a cached capability must equal the recomputed one to
/// the bit, the same cache-soundness discipline).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityVector {
    scores: BTreeMap<u32, Fixed>,
}

impl CapabilityVector {
    /// The capability on a law, or zero if the vector does not carry it.
    pub fn score(&self, id: FunctionLawId) -> Fixed {
        self.scores.get(&id.0).copied().unwrap_or(Fixed::ZERO)
    }

    /// The scores, in law-id order.
    pub fn scores(&self) -> impl Iterator<Item = (FunctionLawId, Fixed)> + '_ {
        self.scores.iter().map(|(&k, &v)| (FunctionLawId(k), v))
    }

    /// Number of scored laws.
    pub fn len(&self) -> usize {
        self.scores.len()
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

/// Derive a part's full capability vector: run every function law over the part's geometry and material,
/// blind to any id. A pure fixed-point read, so the derived function is a deterministic function of the
/// grown physics and the reserved references, with no layer, kingdom, or race key (the Principle-9
/// steering guarantee the leaf already holds, extended to capability). The caller derives the physics
/// ceilings once with [`CapabilityCaps::derive`] and passes them, so a per-part derive draws no registry
/// scan.
pub fn derive_capabilities(
    fns: &FunctionLawRegistry,
    geo: &dyn Fn(&str) -> Fixed,
    mat: &dyn Fn(&str) -> Fixed,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
) -> CapabilityVector {
    let mut scores = BTreeMap::new();
    for def in fns.defs() {
        scores.insert(
            def.id.0,
            def.kernel
                .capability(geo, mat, refs, caps, &def.geometry_axes, &def.material_axes),
        );
    }
    CapabilityVector { scores }
}

/// A decimal-string to `Fixed`, for the labelled dev references. Panics on a malformed literal, which is
/// a programming error in the fixture, never runtime input.
fn dec(s: &str) -> Fixed {
    Fixed::from_decimal_str(s).expect("dev-refs decimal literal")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// A geometry accessor over a fixed test map (zero for an absent axis, the leaf convention).
    fn geo_of(map: BTreeMap<&'static str, &'static str>) -> impl Fn(&str) -> Fixed {
        let m: BTreeMap<String, Fixed> = map
            .into_iter()
            .map(|(k, v)| (k.to_string(), dec(v)))
            .collect();
        move |axis: &str| m.get(axis).copied().unwrap_or(Fixed::ZERO)
    }

    fn mat_of(map: BTreeMap<&'static str, &'static str>) -> impl Fn(&str) -> Fixed {
        geo_of(map)
    }

    /// The physics ceilings from the mechanical floor's own pressure and length ranges (hi 150000 MPa,
    /// 100 m), so the kernel caps match the real [`CapabilityCaps::derive`] without loading the floor.
    fn test_caps() -> CapabilityCaps {
        CapabilityCaps {
            pressure: dec("150000"),
            depth: dec("100"),
        }
    }

    #[test]
    fn a_hard_sharp_point_reads_as_a_weapon_and_a_soft_blunt_one_does_not() {
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();

        // A hard, sharp point: tiny contact area concentrates the strike, hard material sustains it.
        let claw_geo = geo_of([("mech.contact_area", "0.00000005")].into_iter().collect());
        let claw_mat = mat_of([("mat.indentation_hardness", "500")].into_iter().collect());
        let claw = derive_capabilities(&fns, &claw_geo, &claw_mat, &refs, &caps);
        let claw_pierce = claw.score(FunctionLawRegistry::ID_PIERCE);
        assert!(
            claw_pierce > Fixed::ZERO,
            "a hard sharp point is a weapon by its physics: {claw_pierce:?}"
        );

        // A soft, blunt surface: broad contact area spreads the strike below the target hardness.
        let hide_geo = geo_of([("mech.contact_area", "0.01")].into_iter().collect());
        let hide_mat = mat_of([("mat.indentation_hardness", "5")].into_iter().collect());
        let hide = derive_capabilities(&fns, &hide_geo, &hide_mat, &refs, &caps);
        let hide_pierce = hide.score(FunctionLawRegistry::ID_PIERCE);
        assert_eq!(
            hide_pierce,
            Fixed::ZERO,
            "a soft blunt surface is not a weapon: it does not clear the target hardness"
        );
    }

    #[test]
    fn a_keen_strong_edge_reads_a_shear_capability_a_blunt_or_weak_or_ductileless_one_does_not() {
        // The made-world arc, root R1, the first non-piercing action: the SHEAR kernel reads whether a part
        // can sever, from its edge geometry and its own shear strength, never a tag. A keen edge of a material
        // whose shear strength clears the reference resistance parts it and reads a positive capability; a keen
        // edge of a material too weak to reach the reference cannot part it and reads zero; a blunt edge of the
        // strong material spreads the stress below the reference and reads zero; a part with no edge or with
        // neither a shear strength nor a yield to derive one reads zero. A ductile material with only a yield
        // reads a positive shear through the von Mises reduction, so the strength is derived where the axis is
        // silent. This is the SEVER-threshold shape: the effective shear must clear the reference to sever.
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs(); // reference force 100 N, reference shear resistance 3 MPa
        let caps = test_caps();
        let keen = geo_of([("mech.contact_area", "0.00000005")].into_iter().collect());
        let blunt = geo_of([("mech.contact_area", "0.01")].into_iter().collect());
        let strong = mat_of([("mat.shear_strength", "200")].into_iter().collect());
        let weak = mat_of([("mat.shear_strength", "2")].into_iter().collect()); // below the 3 MPa reference
        let ductile = mat_of([("mat.yield_strength", "300")].into_iter().collect()); // no shear axis
        let inert = mat_of([("mat.density", "2500")].into_iter().collect()); // neither shear nor yield

        let shear_of = |geo: &dyn Fn(&str) -> Fixed, mat: &dyn Fn(&str) -> Fixed| {
            derive_capabilities(&fns, geo, mat, &refs, &caps).score(FunctionLawRegistry::ID_SHEAR)
        };

        let keen_strong = shear_of(&keen, &strong);
        let keen_weak = shear_of(&keen, &weak);
        let blunt_strong = shear_of(&blunt, &strong);
        let keen_ductile = shear_of(&keen, &ductile);
        let keen_inert = shear_of(&keen, &inert);
        let no_edge = shear_of(&geo_of(BTreeMap::new()), &strong);

        assert!(
            keen_strong > Fixed::ZERO,
            "a keen strong edge severs: {keen_strong:?}"
        );
        assert!(
            keen_strong > keen_weak,
            "a strong-shear material severs where a too-weak one cannot ({keen_strong:?} vs {keen_weak:?})"
        );
        assert_eq!(
            keen_weak,
            Fixed::ZERO,
            "a material whose shear strength is below the reference resistance cannot sever it"
        );
        assert_eq!(
            blunt_strong,
            Fixed::ZERO,
            "a spread blunt edge drives its shear below the reference and severs nothing"
        );
        assert!(
            keen_ductile > Fixed::ZERO,
            "a ductile edge with only a yield reads a shear through the von Mises reduction: {keen_ductile:?}"
        );
        assert_eq!(
            keen_inert,
            Fixed::ZERO,
            "an edge with neither shear strength nor yield delivers no shear"
        );
        assert_eq!(no_edge, Fixed::ZERO, "no edge, no shear");
    }

    #[test]
    fn a_strong_faced_tool_reads_a_crush_capability_a_weak_or_spread_or_strengthless_one_does_not()
    {
        // The made-world arc, Section G, the second non-piercing action: the CRUSH kernel reads whether a part
        // can fail matter in compression, from its face geometry and its own compressive strength. A face of a
        // strong-compression material clears the reference and crushes; a too-weak material self-limits below
        // the reference; a spread (large-area) face drives its stress below the reference; a part with no face
        // or no compressive strength reads zero. The compressive sibling of the SHEAR sever threshold.
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs(); // reference force 100 N, reference compressive resistance 5 MPa
        let caps = test_caps();
        let keen = geo_of([("mech.contact_area", "0.00000005")].into_iter().collect());
        let spread = geo_of([("mech.contact_area", "0.01")].into_iter().collect());
        let strong = mat_of([("mat.compressive_strength", "200")].into_iter().collect());
        let weak = mat_of([("mat.compressive_strength", "2")].into_iter().collect()); // below the 5 MPa ref
        let strengthless = mat_of([("mat.density", "2500")].into_iter().collect()); // no compressive axis

        let crush_of = |geo: &dyn Fn(&str) -> Fixed, mat: &dyn Fn(&str) -> Fixed| {
            derive_capabilities(&fns, geo, mat, &refs, &caps).score(FunctionLawRegistry::ID_CRUSH)
        };

        assert!(
            crush_of(&keen, &strong) > Fixed::ZERO,
            "a strong-faced tool crushes: {:?}",
            crush_of(&keen, &strong)
        );
        assert_eq!(
            crush_of(&keen, &weak),
            Fixed::ZERO,
            "a material whose compressive strength is below the reference cannot fail it"
        );
        assert_eq!(
            crush_of(&spread, &strong),
            Fixed::ZERO,
            "a spread face drives its compressive stress below the reference and crushes nothing"
        );
        assert_eq!(
            crush_of(&keen, &strengthless),
            Fixed::ZERO,
            "a part with no compressive strength delivers no crush"
        );
        assert_eq!(
            crush_of(&geo_of(BTreeMap::new()), &strong),
            Fixed::ZERO,
            "no face, no crush"
        );
    }

    #[test]
    fn a_strong_long_stroked_part_reads_an_impact_a_weak_or_strengthless_one_does_not() {
        // The made-world arc, Section G: the IMPACT kernel reads whether a part is a percussion tool, from its
        // ACTUATOR WORK (its strength stress over its cross-section, over its own grown stroke, F d), not its
        // mass. A strong, long-stroked part delivers energy above the reference strike energy and reads a full
        // impact; a weak one of the same geometry delivers too little and reads zero; a strengthless part, or one
        // with no grown stroke, reads zero. The per-body strength, cross-section, and stroke replace the retired
        // world-global swing speed.
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs(); // reference strike energy 100 J
        let caps = test_caps();
        // The actuating geometry: a 1e-6 m^2 cross-section the force acts over and a grown stroke it acts across.
        let geo = geo_of(
            [
                ("mech.cross_section_area", "0.000001"),
                ("mech.stroke_length", "1"),
            ]
            .into_iter()
            .collect(),
        );
        // A STRONG actuator (fracture_strength 200 MPa): force 200 N over a 1 m stroke (the stress_force
        // megapascal-to-newton bridge) delivers 200 J, above the 100 J reference. A WEAK one (0.1): 0.1 J, below.
        let strong = mat_of([("mat.fracture_strength", "200")].into_iter().collect());
        let weak = mat_of([("mat.fracture_strength", "0.1")].into_iter().collect());
        let strengthless = mat_of(BTreeMap::new());

        let impact_of = |mat: &dyn Fn(&str) -> Fixed| {
            derive_capabilities(&fns, &geo, mat, &refs, &caps).score(FunctionLawRegistry::ID_IMPACT)
        };

        assert!(
            impact_of(&strong) > Fixed::ZERO,
            "a strong long-stroked part reads a percussion impact: {:?}",
            impact_of(&strong)
        );
        assert_eq!(
            impact_of(&weak),
            Fixed::ZERO,
            "a weak actuator of the same geometry delivers too little energy and reads no impact"
        );
        assert_eq!(
            impact_of(&strengthless),
            Fixed::ZERO,
            "a strengthless part delivers no blow"
        );
        // A part with no grown stroke reads no impact even when strong (the absence convention).
        let no_stroke = geo_of([("mech.cross_section_area", "1")].into_iter().collect());
        assert_eq!(
            derive_capabilities(&fns, &no_stroke, &strong, &refs, &caps)
                .score(FunctionLawRegistry::ID_IMPACT),
            Fixed::ZERO,
            "a part with no grown stroke delivers no blow"
        );
    }

    #[test]
    fn a_cut_reads_the_targets_own_material_so_the_same_edge_diverges_by_target() {
        // The crafting seam (material-substrate item 4): a cut contest reads the TARGET's material, so the
        // same edge parts a soft target and stalls on a hard one, and a harder sharper edge parts stone a
        // softer one cannot, all from the target's substance data with no per-material branch.
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();
        // A modest flint edge: a small contact area, a moderate own hardness.
        let edge_geo = geo_of([("mech.contact_area", "0.00000005")].into_iter().collect());
        let edge_mat = mat_of([("mat.indentation_hardness", "500")].into_iter().collect());
        // A soft target (hide): low hardness, low cutting energy. A hard target (granite): high both.
        let hide = mat_of(
            [
                ("mat.indentation_hardness", "5"),
                ("mat.specific_cut_energy", "2"),
            ]
            .into_iter()
            .collect(),
        );
        let granite = mat_of(
            [
                ("mat.indentation_hardness", "5000"),
                ("mat.specific_cut_energy", "1000"),
            ]
            .into_iter()
            .collect(),
        );
        let on_hide = cut_capability_against_target(&edge_geo, &edge_mat, &hide, &refs, &caps);
        let on_granite =
            cut_capability_against_target(&edge_geo, &edge_mat, &granite, &refs, &caps);
        assert!(
            on_hide > Fixed::ZERO,
            "the edge parts soft hide: {on_hide:?}"
        );
        assert_eq!(
            on_granite,
            Fixed::ZERO,
            "the same edge cannot part granite: its pressure does not clear the harder target"
        );

        // A harder, sharper edge (a smaller contact patch, a harder own material) raises its effective
        // pressure over granite's hardness and parts it: a better tool works harder matter, the crafting
        // payoff, from geometry and material alone.
        let pick_geo = geo_of([("mech.contact_area", "0.000000005")].into_iter().collect());
        let pick_mat = mat_of([("mat.indentation_hardness", "6000")].into_iter().collect());
        let pick_on_granite =
            cut_capability_against_target(&pick_geo, &pick_mat, &granite, &refs, &caps);
        assert!(
            pick_on_granite > Fixed::ZERO,
            "a harder sharper edge parts the granite the softer one could not: {pick_on_granite:?}"
        );

        // The contest reads the target: the same edge on two targets differs only because the target data
        // differs, no branch on a material name.
        assert!(
            on_hide > on_granite,
            "the divergence is the target's substance data, not the edge"
        );
    }

    #[test]
    fn the_derived_capability_is_a_pure_function_of_geometry_and_material_with_no_id_key() {
        // The Principle-9 guarantee: two parts with identical geometry and material read the identical
        // capability, whatever else differs, because the read keys only on axis values. Recomputing is
        // bit-identical (the cache-soundness property the leaf oracle proves, here trivially since the
        // read is a pure function).
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();
        let g = geo_of([("mech.contact_area", "0.0001")].into_iter().collect());
        let m = mat_of([("mat.indentation_hardness", "300")].into_iter().collect());
        let a = derive_capabilities(&fns, &g, &m, &refs, &caps);
        let b = derive_capabilities(&fns, &g, &m, &refs, &caps);
        assert_eq!(a, b, "the capability read is deterministic and pure");
    }

    #[test]
    fn a_part_with_no_contact_area_reads_no_weapon_capability() {
        // Presence-gating: a part carrying no contact-area geometry reads zero area and so zero pierce,
        // the natural gate the zero-for-absent accessor gives, no explicit has-axis branch.
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();
        let g = geo_of(BTreeMap::new());
        let m = mat_of([("mat.indentation_hardness", "500")].into_iter().collect());
        let v = derive_capabilities(&fns, &g, &m, &refs, &caps);
        assert_eq!(v.score(FunctionLawRegistry::ID_PIERCE), Fixed::ZERO);
    }

    #[test]
    fn a_stout_limb_reads_as_a_locomotor_and_a_bodiless_organ_does_not() {
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();

        // A stout limb: a real section modulus and a bony yield strength, so the stride load stays far
        // below yield and the limb bears it.
        let limb_geo = geo_of(
            [
                ("mech.section_modulus", "0.0001"),
                ("mech.arm_length", "0.3"),
            ]
            .into_iter()
            .collect(),
        );
        let limb_mat = mat_of([("mat.yield_strength", "150")].into_iter().collect());
        let limb = derive_capabilities(&fns, &limb_geo, &limb_mat, &refs, &caps);
        let limb_move = limb.score(FunctionLawRegistry::ID_LOCOMOTE);
        assert!(
            limb_move > Fixed::ZERO,
            "a stout limb bears its propulsive load and is a locomotor: {limb_move:?}"
        );

        // An organ carrying no section modulus bears no load: not a limb, by physics.
        let organ_geo = geo_of(BTreeMap::new());
        let organ_mat = mat_of([("mat.yield_strength", "3")].into_iter().collect());
        let organ = derive_capabilities(&fns, &organ_geo, &organ_mat, &refs, &caps);
        assert_eq!(
            organ.score(FunctionLawRegistry::ID_LOCOMOTE),
            Fixed::ZERO,
            "an organ with no section modulus is no limb"
        );
    }

    #[test]
    fn an_optical_lens_reads_as_an_eye_and_a_medium_matched_tissue_does_not() {
        let fns = FunctionLawRegistry::dev_seed();
        let refs = CapabilityRefs::dev_refs();
        let caps = test_caps();
        let no_geo = geo_of(BTreeMap::new());

        // A lens tissue: refractive index well above the medium (air ~1), so it focuses light.
        let lens = mat_of([("opt.refractive_index", "1.4")].into_iter().collect());
        let eye = derive_capabilities(&fns, &no_geo, &lens, &refs, &caps);
        assert!(
            eye.score(FunctionLawRegistry::ID_REFRACT) > Fixed::ZERO,
            "a lens denser than the medium focuses light and is an eye"
        );

        // A tissue matching the medium index (no contrast) does not focus: not an eye.
        let clear = mat_of([("opt.refractive_index", "1")].into_iter().collect());
        let blind = derive_capabilities(&fns, &no_geo, &clear, &refs, &caps);
        assert_eq!(
            blind.score(FunctionLawRegistry::ID_REFRACT),
            Fixed::ZERO,
            "a tissue matching the medium reads no optical contrast: not an eye"
        );
    }
}
