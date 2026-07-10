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

//! The affordance-percept registry (ideation / experiential-discovery arc, piece 2, slice 2a): what a
//! being SENSES about the nearby MATTER's potential for ACTION, the raw physics-derived scalars an
//! evolved controller and the discovery loop read to propose and score candidate actions. It is the
//! sibling of [`crate::percept::PerceptRegistry`], which senses the substance CLASSES underfoot for the
//! edibility and harm floors: where that answers "what is here", this answers "what could I DO with what
//! is here", how breakable the matter is, and (as later percepts land) how sharp a loose piece is and how
//! graspable.
//!
//! Each channel is a `[0, 1]` scalar DERIVED from the physics floor over the matter's own material axes,
//! never an `IsTree` or `IsAxe` kind tag (Principle 9), keyed by the physics quantity the kernel reads.
//! The kernel SET is fixed Rust ([`AffordancePerceptKind`], a closed enum in the style of
//! `civsim_compose::CapabilityKernel`); which of those kernels a world's beings SELECT and in what order is
//! the registry's data, declarable by kernel NAME ([`AffordancePerceptRegistry::from_names`], Principle 11).
//! The honest bound, from the blind framing panel: naming makes the SELECTION among the existing kernels
//! world-declarable; it does NOT make the affordance space or the kernels' CONTENTS world-declared or
//! alien-clean, and adding a NEW kind of derivation is a code change (a new variant plus its physics). A
//! being whose native affordance no existing kernel computes needs that code change, tracked as the
//! composition-substrate seam in `docs/working/CONSENSUS_ROADMAP.md`. Empty by default, so a world that
//! declares none carries no affordance-percept block and every run hash is unchanged (the opt-in,
//! hash-neutral pattern the feature block established).
//!
//! This slice is READ only: the derivation and the registry sit off the run path (nothing perceives yet,
//! and `state_hash` folds nothing), so every existing scenario replays bit-for-bit. Piece 2's binding
//! graph (slice 2b) reads these scalars to sample candidate actions, and the WIRE (slice 2c) feeds them
//! into the controller alongside the feature and appetitive blocks.

use civsim_compose::{
    derive_capabilities, CapabilityCaps, CapabilityKernel, CapabilityRefs, CompositionNode,
    FunctionLawId, FunctionLawRegistry, IntentRef, NodeBody, PortVector,
};
use civsim_core::Fixed;
use civsim_physics::PhysicsRegistry;

use crate::material::{SubstanceMix, WieldedTool};

/// The fracture-strength material axis the fracture-potential kernel reads: the stress a substance
/// fractures at, the same axis the extraction contest gates on ([`crate::material`]).
const AXIS_FRACTURE: &str = "mat.fracture_strength";

/// The contact-area geometry axis the sharpness kernel reads: the working area a tool's edge presses
/// over, the intrinsic geometry a shaped object carries and loose matter does not ([`WieldedTool`]).
const AXIS_CONTACT_AREA: &str = "mech.contact_area";

/// The mass axis the percussion IMPACT kernel reads: the tool's extensive mass (its retained volume times
/// its substance density), the datum only a carried object supplies, exposed to the capability dispatch.
const AXIS_MASS: &str = "mech.mass";

/// A perceived affordance channel's id: its slot in the affordance-percept block, in registry order,
/// exactly as [`crate::percept::PerceptId`] slots the raw-feature block.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct AffordancePerceptId(pub u16);

/// The reserved reference levels the affordance-percept derivations read (RESERVED, fail-loud from the
/// manifest, none fabricated). The sibling of `civsim_compose::CapabilityRefs`. The mechanism is fixed
/// Rust; these are the owner's numbers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AffordancePerceptRefs {
    /// RESERVED. The reference actionable contact stress the fracture-potential is measured against: the
    /// contact stress an ORDINARY being delivers to matter it works (the extraction contest's reference
    /// force over a reference contact area, [`crate::material`]'s `ExtractionParams`), so the percept
    /// reads "how breakable is this to an ordinary actor" rather than against the theoretical strongest
    /// material the axis admits (which would read every ordinary rock and soil as equally, trivially
    /// breakable and discriminate nothing). Surfaced with its basis, never fabricated: the value graduates
    /// from the manifest once the owner sets it, and the derivation stays inert until it does.
    ///
    /// SEAM (flagged, roadmap "World liveliness and agency"; R-XXX candidate): this reference is
    /// PERCEIVER-INDEPENDENT, one embodiment-wide "ordinary actor" scale rather than the perceiving being's
    /// own delivered stress, so a mite and a giant sense identical breakability. To admit the alien it should
    /// key on the PERCEIVER's own data; a gated, behaviour-changing piece (not byte-neutral), not built here.
    pub reference_stress: Fixed,
    /// The reserved capability reference levels the sharpness kernel's Pierce read uses (the reference
    /// strike force, target hardness, and penetration references `civsim_compose::CapabilityRefs` carries),
    /// the SAME references a body part's Pierce capability is derived against, so a tool's sensed sharpness
    /// and its worked capability read one scale. Reserved-with-basis in the compose layer, never fabricated.
    pub capability: CapabilityRefs,
}

impl AffordancePerceptRefs {
    /// A labelled dev fixture for the unit tests and the pre-wire scenarios: a reference actionable stress
    /// in the tens-of-megapascals range an ordinary limb-and-stone actor delivers, so soft matter reads a
    /// high fracture-potential and hard rock a low one, alongside the compose dev capability references the
    /// Pierce read uses. The manifest values are reserved; this is only the fixture, never the canonical
    /// numbers.
    pub fn dev_refs() -> AffordancePerceptRefs {
        AffordancePerceptRefs {
            reference_stress: Fixed::from_int(10),
            capability: CapabilityRefs::dev_refs(),
        }
    }
}

/// The closed set of physics-derived affordance scalars a being can perceive over nearby matter. Fixed
/// Rust, one variant per kernel, mirroring `civsim_compose::CapabilityKernel`; which of these kernels a
/// world's beings SELECT, and in what order, is the registry's data below (by name via
/// [`AffordancePerceptRegistry::from_names`]).
///
/// SEAM (flagged, roadmap "World liveliness and agency"; R-XXX candidate): this CLOSED type-set caps the
/// emergent affordance/technique/culture space, because the discovery loop can only propose actions built
/// from the percepts this set contains, so a photosynthetic, mana, or redox being cannot express its native
/// affordances as DATA today (Principle 8/11, admit-the-alien). Opening it needs a data-expressible
/// DERIVATION substrate (a composable algebra over floor axes so a new affordance formula is data, sibling
/// to the function-law catalogue), a deeper piece NOT built here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AffordancePerceptKind {
    /// FRACTURE-POTENTIAL: how breakable the matter is. Its greatest fracture strength (the hardest
    /// constituent phase, the one the extraction contest gates on, [`SubstanceMix::fracture_hardness`])
    /// sets the resistance; the reference actionable stress ([`AffordancePerceptRefs::reference_stress`])
    /// over that resistance, clamped to `[0, 1]`, is the potential, so the WEAKEST present matter reads
    /// near one (an ordinary actor's stress overwhelms it) and matter far stronger than the reference
    /// reads near zero. Matter absent reads zero (nothing to break). Reads only `mat.fracture_strength`
    /// and the reserved reference, never a kind id (Principle 9); it is the graded percept of the same
    /// contest the extraction gate resolves as a boolean.
    FracturePotential,
    /// SHARPNESS: how keen an EDGE the being's wielded tool presents, the existing Pierce capability score
    /// ([`civsim_compose::CapabilityKernel::Pierce`]) run over the tool's own intrinsic geometry (its
    /// working contact area, [`WieldedTool::contact_area`]) and material (its `mat.indentation_hardness`
    /// through the physics registry). A small hard edge scores high, a blunt or soft one low. Read over the
    /// SHAPED tool rather than the loose cell matter, because a keen edge is a property of geometry (which a
    /// knapped tool carries and a pile of matter does not), so a being with no tool, or one whose tool has
    /// no edge, reads zero. It senses the edge's geometric keenness (the Pierce concentration of force at a
    /// small contact area), the same intrinsic geometry the tool's worked capability reads; whether that edge
    /// affords a CUT is its SHEAR capability and whether the cut BITES is the enact's physics, so a being
    /// senses the sharpness of what it holds on physics, never an `IsAxe` tag (Principle 9).
    ///
    /// SEAM (flagged, roadmap "World liveliness and agency"; R-XXX candidate): this kernel is a
    /// purpose-laden COMPOSITE (it runs the Pierce capability over the tool's geometry and hardness), so
    /// "piercing is a salient affordance" is AUTHORED as a percept rather than EMERGING from primitives
    /// (contact area, hardness, applied stress) composed by the ideation loop (Principle 8, admit-the-alien).
    /// It also reads only a WIELDED tool and returns zero for a being whose pierce-capable geometry is its own
    /// body (claws, teeth), a further tool-user assumption folded into the same seam. To fix, bar or relocate
    /// it so the salience emerges; a gated piece, not built here.
    Sharpness,
}

impl AffordancePerceptKind {
    /// Derive this scalar in `[0, 1]` over the nearby matter and the being's wielded tool, through the
    /// physics registry and the reserved reference levels. Pure and RNG-free. Each kernel reads the source
    /// its physics lives on: a material-property kernel (fracture-potential) reads the cell `matter`, a
    /// shape-dependent one (sharpness) reads the shaped `tool`; a kernel whose source is absent reads zero,
    /// the clean degrade.
    pub fn perceive(
        self,
        matter: Option<&SubstanceMix>,
        tool: Option<&WieldedTool>,
        reg: &PhysicsRegistry,
        refs: &AffordancePerceptRefs,
    ) -> Fixed {
        match self {
            AffordancePerceptKind::FracturePotential => {
                let Some(matter) = matter.filter(|m| !m.is_empty()) else {
                    return Fixed::ZERO; // no matter to break
                };
                let strength = matter.fracture_hardness(reg);
                if strength <= Fixed::ZERO {
                    return Fixed::ZERO; // no fracturable matter present
                }
                refs.reference_stress
                    .checked_div(strength)
                    .unwrap_or(Fixed::ONE)
                    .clamp(Fixed::ZERO, Fixed::ONE)
            }
            AffordancePerceptKind::Sharpness => {
                let Some(tool) = tool else {
                    return Fixed::ZERO; // no tool in hand, no edge to sense
                };
                // The tool's intrinsic geometry (its working contact area) and its material (read from the
                // registry by substance id), the two closures the Pierce kernel reads, exactly as a body
                // part's geo/mat feed its capability. The physics ceilings derive from the registry's own
                // axis ranges (Principle 11, data not a fabricated constant).
                let contact_area = tool.contact_area;
                let geo = |axis: &str| {
                    if axis == AXIS_CONTACT_AREA {
                        contact_area
                    } else {
                        Fixed::ZERO
                    }
                };
                let mat = |axis: &str| {
                    reg.substance(&tool.substance)
                        .and_then(|s| s.vector.get(axis).copied())
                        .unwrap_or(Fixed::ZERO)
                };
                let caps = CapabilityCaps::derive(reg);
                CapabilityKernel::Pierce.capability(&geo, &mat, &refs.capability, &caps)
            }
        }
    }

    /// The physics axis this kernel keys off, so the belief-subject key (slice 2b) and a diagnostic can
    /// name the quantity the percept reads, never a hardcoded label (Principle 11).
    pub fn axis(self) -> &'static str {
        match self {
            AffordancePerceptKind::FracturePotential => AXIS_FRACTURE,
            AffordancePerceptKind::Sharpness => AXIS_CONTACT_AREA,
        }
    }

    /// The full closed kernel set, in canonical order, so [`Self::from_name`] can derive from [`Self::name`]
    /// over it (the two cannot diverge) and a total round-trip test covers every kernel. A new variant must be
    /// added here; [`Self::name`]'s exhaustive match forces it a name at the same time.
    pub const ALL: [AffordancePerceptKind; 2] = [
        AffordancePerceptKind::FracturePotential,
        AffordancePerceptKind::Sharpness,
    ];

    /// This kernel's STABLE STRING NAME, the id world data resolves against to SELECT this kernel into a
    /// registry ([`AffordancePerceptRegistry::from_names`]), in the style of `civsim_compose::FunctionLawDef`'s
    /// `name`. The name is a RESOLUTION KEY only: it is consumed when the registry is built and is NOT stored
    /// on the resulting [`AffordancePerceptDef`], which carries the opaque slot id and the kernel handle, so
    /// no downstream consumer (the controller, the discovery loop) can branch on it (the opaque-slot
    /// invariant, the template case: a high-level name must never drive behaviour). Renaming a kernel is a
    /// data-vocabulary change, never a behaviour change. The match is exhaustive, so a new variant forces a
    /// name here, and [`Self::from_name`] derives from this over [`Self::ALL`], so the two cannot diverge.
    pub fn name(self) -> &'static str {
        match self {
            AffordancePerceptKind::FracturePotential => "fracture_potential",
            AffordancePerceptKind::Sharpness => "sharpness",
        }
    }

    /// Resolve a kernel from its stable string name, the inverse of [`Self::name`] that
    /// [`AffordancePerceptRegistry::from_names`] uses. Derived by matching the name against [`Self::name`] over
    /// [`Self::ALL`], so `name` and `from_name` cannot disagree by construction (no hand-kept parallel arm).
    /// `None` for an unknown name, so the registry constructor FAILS LOUD rather than silently drop a percept a
    /// world declared (never a silent plausible default). A name is not a new kernel: the kernel set is closed
    /// Rust, so a name that resolves to nothing is a data error, not a request to author a kernel.
    pub fn from_name(name: &str) -> Option<AffordancePerceptKind> {
        AffordancePerceptKind::ALL
            .into_iter()
            .find(|kind| kind.name() == name)
    }
}

/// The capability a WIELDED TOOL reads on one function law, derived from the tool's own working geometry (its
/// contact area) and material (its substance's axes through the physics registry) on the SAME capability
/// dispatch a body part is derived on ([`derive_capabilities`], the exact call [`crate::homeostasis`]'s
/// `body_capability` uses). So a wielded tool enters the afford derivation exactly as an extra body part
/// would: a keen hard edge reads PIERCE and thus can afford a cut, by physics, never an `IsAxe` tag
/// (Principle 9). This is the same tool-edge physics the [`AffordancePerceptKind::Sharpness`] percept senses,
/// generalised from Pierce to any law so the afford gate (the made-world arc, tool-contributed affordances)
/// can read it. Reads the tool against the SAME `refs`/`caps` a body's capability uses, so the tool and the
/// body are derived on one scale. Pure and RNG-free.
pub fn tool_capability(
    tool: &WieldedTool,
    reg: &PhysicsRegistry,
    refs: &CapabilityRefs,
    caps: &CapabilityCaps,
    law: FunctionLawId,
) -> Fixed {
    let fns = FunctionLawRegistry::dev_seed();
    let contact_area = tool.contact_area;
    // The tool's MASS is the extensive datum (its retained volume times its substance density) that a
    // percussion IMPACT read needs and the registry's intensive axes cannot supply; exposed to the capability
    // dispatch as `mech.mass` (the made-world arc, the tool-geometry expansion, GATE 2), so a HEAVY tool reads
    // an impact a light one does not, by physics. Derived, never stored.
    let mass = tool.mass(reg);
    let geo = |axis: &str| {
        if axis == AXIS_CONTACT_AREA {
            contact_area
        } else if axis == AXIS_MASS {
            mass
        } else {
            Fixed::ZERO
        }
    };
    let mat = |axis: &str| {
        reg.substance(&tool.substance)
            .and_then(|s| s.vector.get(axis).copied())
            .unwrap_or(Fixed::ZERO)
    };
    derive_capabilities(&fns, &geo, &mat, refs, caps).score(law)
}

/// One declared affordance percept: its slot id and the physics kernel it reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AffordancePerceptDef {
    /// The channel's slot in the affordance-percept block (registry order).
    pub id: AffordancePerceptId,
    /// The physics-derived scalar this channel senses.
    pub kind: AffordancePerceptKind,
}

/// The affordance-percept registry: which physics-derived affordance scalars a world's beings sense over
/// nearby matter, in canonical id order. EMPTY by default, so a world that declares none is bit-identical
/// (opt-in, hash-neutral), exactly as an empty [`crate::percept::PerceptRegistry`] is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AffordancePerceptRegistry {
    percepts: Vec<AffordancePerceptDef>,
}

impl AffordancePerceptRegistry {
    /// The empty registry: no affordance percepts, the opt-in default that folds nothing and reads as a
    /// zero-width block.
    pub fn empty() -> AffordancePerceptRegistry {
        AffordancePerceptRegistry {
            percepts: Vec::new(),
        }
    }

    /// Build from a list of kinds, ids assigned by position (canonical order), exactly as
    /// [`crate::percept::PerceptRegistry::from_classes`] slots the raw-feature channels. Which of the fixed
    /// kernels a world SELECTS and in what order is data (see [`Self::from_names`] for the name path); adding a
    /// NEW kind of kernel is a code change, not a data edit, tracked as the composition-substrate seam.
    pub fn from_kinds(kinds: &[AffordancePerceptKind]) -> AffordancePerceptRegistry {
        AffordancePerceptRegistry {
            percepts: kinds
                .iter()
                .enumerate()
                .map(|(i, &kind)| AffordancePerceptDef {
                    id: AffordancePerceptId(i as u16),
                    kind,
                })
                .collect(),
        }
    }

    /// Build from a list of stable kernel NAMES, resolving each against the fixed kernel set
    /// ([`AffordancePerceptKind::from_name`]) and assigning ids by position (canonical order), the
    /// DATA-SOURCE path the enum-valued [`Self::from_kinds`] lacks: today a world can only name its percepts
    /// in Rust, so this lets world DATA declare which of the existing kernels its beings SELECT and in what
    /// order (Principle 11). FAILS LOUD on an unknown name (the error names the offending kernel) rather than
    /// dropping it, so a typo in world data cannot silently reduce a being's perception.
    ///
    /// The scope, stated plainly (the blind framing panel caught the overclaim it corrects): this makes the
    /// SELECTION and ORDER among the EXISTING kernels world-declarable data. It does NOT make the affordance
    /// space or the kernels' CONTENTS world-declared or alien-clean: the kernel SET stays fixed Rust, so a
    /// being whose native affordance no existing kernel computes still needs a new kernel (a code change),
    /// tracked as the composition-substrate seam in the roadmap. The names resolve to enum kinds and are then
    /// DISCARDED; the result is byte-identical to the registry [`Self::from_kinds`] builds for the same
    /// kernels in the same order, so no name survives into what the run path reads (the opaque-slot invariant).
    pub fn from_names(names: &[&str]) -> Result<AffordancePerceptRegistry, String> {
        let mut kinds = Vec::with_capacity(names.len());
        for &name in names {
            match AffordancePerceptKind::from_name(name) {
                Some(kind) => kinds.push(kind),
                None => {
                    return Err(format!(
                        "unknown affordance-percept kernel name {name:?}; the kernels are fixed Rust and a \
                         new one is a code change, not a data name"
                    ))
                }
            }
        }
        Ok(AffordancePerceptRegistry::from_kinds(&kinds))
    }

    /// The declared percepts, in canonical id order.
    pub fn percepts(&self) -> &[AffordancePerceptDef] {
        &self.percepts
    }

    /// The number of declared affordance-percept channels (the width of the affordance-percept block).
    pub fn len(&self) -> usize {
        self.percepts.len()
    }

    /// Whether the registry declares no affordance percepts (the opt-in default).
    pub fn is_empty(&self) -> bool {
        self.percepts.is_empty()
    }

    /// Perceive the being's surroundings and equipment: the `[0, 1]` physics scalar for each declared
    /// percept, in canonical id order. A pure read of the matter's and the tool's own physics against the
    /// reserved references (Principles 9, 10), RNG-free. `matter` is the [`SubstanceMix`] of the cell the
    /// being stands on (or reaches), for the material-property percepts; `tool` is the [`WieldedTool`] the
    /// being holds, for the shape-dependent ones. Either source `None` reads its dependent channels as zero
    /// (the clean degrade), so a world with no material layer and a being with no tool perceive a flat-zero
    /// vector and an opted-out world is unchanged.
    pub fn perceive(
        &self,
        matter: Option<&SubstanceMix>,
        tool: Option<&WieldedTool>,
        reg: &PhysicsRegistry,
        refs: &AffordancePerceptRefs,
    ) -> Vec<Fixed> {
        self.percepts
            .iter()
            .map(|p| p.kind.perceive(matter, tool, reg, refs))
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------------------
// Composition-substrate arc, Tier B: the single-axis TRANSDUCTION, the data-declarable "sensor" form.
//
// The blind framing panel (docs/working/AFFORDANCE_COMPOSITION_SUBSTRATE_FRAMING.md) split the affordance
// substrate along the authoring line. A SINGLE-AXIS transduction reads ONE floor axis of the perceived
// target and normalizes it against a reference sourced from the PERCEIVER's own body or a cited physics-floor
// constant. That is declaring a SENSOR (a P9 sensory-physics disposition a world may author as data), never
// an affordance. The MULTI-AXIS COMPOSITION that would bundle transductions into a purpose-laden percept must
// EMERGE from a learner over these primitives under selection (the template case: the bundling is the
// correlational fact that must arise, never be authored); that is Tier C, routed through the discovery loop
// (discovery.rs:61-63, "NO coded primitive-to-affordance pairing ... a technique emerges"), NOT built here.
// Valence and meaning are receiver-side, never in the transduction.
//
// HONEST BOUNDS (roadmap R-XXX candidates, not closed here): this touches only the P11 "which fold over
// EXISTING axes" ceiling. It does NOT touch the READABLE-AXIS ceiling (a being whose native affordance needs
// an axis the floor does not model still forces the develop-the-floor path), nor the COMBINATOR-SET ceiling
// (the closed kernel set below). Alien-admission stays gated by floor-axis coverage.
// ---------------------------------------------------------------------------------------------------------

/// Where a transduction's reference level comes from. A typed enum with NO numeric-literal field, so a free
/// literal terminal is UNREPRESENTABLE (the value-authoring line enforced by the TYPE, not by a runtime
/// check): a reference is EITHER the perceiver's own body axis (keyed on the being's own data, so a small body
/// and a large body reference differently and an alien body differs as data) OR a cited physics-floor
/// constant (read from the floor, never typed in). Each variant holds an id (an axis or constant name), never
/// a [`Fixed`] value, so no author can write a bare number into a declared transduction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ReferenceSource {
    /// The perceiver's OWN body axis, read from the perceiving being's own data (its delivered-stress
    /// capacity, a limb dimension, and so on). Holds the axis id, resolved against the being's body at
    /// evaluation; never a stored value. This is what closes the perceiver-independent-reference seam.
    PerceiverBodyAxis(String),
    /// A cited physics-floor constant (a named floor law constant or axis reference), resolved against the
    /// floor at evaluation. Holds the constant id, never a stored value, so the floor stays the one authored
    /// place and a transduction cannot smuggle a literal.
    FloorConstant(String),
}

/// The fixed-Rust transform a single-axis transduction applies. A CLOSED set (the combinator-set ceiling,
/// flagged as a roadmap seam): the MATH is Rust, which kernel a transduction uses is data. One kernel today,
/// the general form of the existing fracture-potential read; the set grows only when a new physically
/// grounded transform is added, exactly as `AffordancePerceptKind` and `civsim_compose::CombinatorKernel` are
/// closed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransductionKernel {
    /// The reference OVER the target-axis magnitude, clamped to `[0, 1]`: the general form of
    /// [`AffordancePerceptKind::FracturePotential`] with the reference keyed on the perceiver rather than a
    /// shared scale, so a target whose axis is small relative to the perceiver's reference reads near one and
    /// one far larger reads near zero. An absent target axis reads zero (nothing to sense).
    ReferenceOverAxis,
}

/// A single-axis affordance transduction, declared as DATA: which floor axis to read on the target, where its
/// reference comes from ([`ReferenceSource`], never a literal), and which fixed-Rust [`TransductionKernel`] to
/// apply. Declaring one of these is declaring a SENSOR, not an affordance (see the module note above).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SingleAxisTransduction {
    /// The floor axis read on the perceived target (the matter underfoot or a held object).
    pub target_axis: String,
    /// Where the reference level is sourced (the perceiver's own body, or a cited floor constant). Never a
    /// literal: the type forbids it.
    pub reference: ReferenceSource,
    /// The fixed-Rust transform applied to the target magnitude and the reference.
    pub kernel: TransductionKernel,
}

impl SingleAxisTransduction {
    /// Evaluate this transduction to a `[0, 1]` scalar. Pure and RNG-free (Principles 9, 10). The three
    /// readers supply the only values that enter: `target` reads a floor axis on the perceived matter or
    /// object; `own_body` reads an axis on the PERCEIVER's own body (so the reference keys on the being's own
    /// data, and an alien body differs as data); `floor_const` resolves a cited floor constant. No value is
    /// stored on the transduction itself, so nothing an author typed in can reach the result: the reference is
    /// always a resolved read, never a baked number (the value-authoring line, enforced by the type). The
    /// type forbids a STORED literal; it does not constrain what the resolver closures return, so the sensor's
    /// honesty also rests on the live-wire contract that the closures read floor and own-body data (a
    /// deferred, coordinated `runner.rs` follow-on, verified when wired).
    ///
    /// A zero target magnitude reads zero (the clean degrade, "nothing to sense"), which conflates an axis
    /// the target does not carry with an axis genuinely valued at zero, matching the existing
    /// [`AffordancePerceptKind::FracturePotential`] convention; a resolver returns [`Fixed::ZERO`] for both.
    pub fn transduce(
        &self,
        target: &impl Fn(&str) -> Fixed,
        own_body: &impl Fn(&str) -> Fixed,
        floor_const: &impl Fn(&str) -> Fixed,
    ) -> Fixed {
        let magnitude = target(&self.target_axis);
        let reference = match &self.reference {
            ReferenceSource::PerceiverBodyAxis(axis) => own_body(axis),
            ReferenceSource::FloorConstant(id) => floor_const(id),
        };
        match self.kernel {
            TransductionKernel::ReferenceOverAxis => {
                if magnitude <= Fixed::ZERO {
                    return Fixed::ZERO; // no target-axis magnitude present, nothing to sense
                }
                // magnitude > 0 here, so a `checked_div` None is an OVERFLOW (an out-of-range quotient),
                // never a zero divisor, and the true quotient's sign is the reference's. Saturate toward the
                // clamp bound in the true direction: a large positive ratio to one, a large negative to zero.
                // The bounds are the clamp's own, not an authored value.
                let saturated = if reference >= Fixed::ZERO {
                    Fixed::ONE
                } else {
                    Fixed::ZERO
                };
                reference
                    .checked_div(magnitude)
                    .unwrap_or(saturated)
                    .clamp(Fixed::ZERO, Fixed::ONE)
            }
        }
    }

    /// The canonical serialization of this transduction: the domain's stable, deterministic byte form,
    /// the input a `civsim_compose::NodeBody::Transduction` content-addresses. Fixed field order (the
    /// target axis, then the reference, then the kernel), every string length-prefixed (a u32
    /// little-endian byte count) so no two distinct transductions can share a byte stream, every enum a
    /// fixed discriminant byte read from the type here rather than from `#[repr]` (so a variant reorder
    /// cannot silently shift an id). There is no map or set in a transduction, so there is no
    /// iteration-order dependence: the bytes are a pure function of the field values, and identical
    /// transductions serialize identically on every machine and across runs, the same stability
    /// discipline `state_hash` holds. `compose` never parses these bytes; it folds them whole into the
    /// content id, so a distinct sensor mints a distinct design.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_canonical_str(&mut bytes, &self.target_axis);
        match &self.reference {
            ReferenceSource::PerceiverBodyAxis(axis) => {
                bytes.push(REF_TAG_PERCEIVER_BODY_AXIS);
                write_canonical_str(&mut bytes, axis);
            }
            ReferenceSource::FloorConstant(id) => {
                bytes.push(REF_TAG_FLOOR_CONSTANT);
                write_canonical_str(&mut bytes, id);
            }
        }
        match self.kernel {
            TransductionKernel::ReferenceOverAxis => bytes.push(KERNEL_TAG_REFERENCE_OVER_AXIS),
        }
        bytes
    }

    /// This transduction as a content-addressed `civsim_compose` node: an opaque tag-3 leaf carrying its
    /// canonical bytes, so a distinct sensor mints a distinct design id and can be promoted, folded, and
    /// selected through discovery like any other node. The leaf is the SUBSTRATE for a multi-axis
    /// affordance (a `compose` tree of these leaves); the bundling itself is the Tier-C emergent step the
    /// discovery loop performs under selection, not authored here (see the module note above). The
    /// `intent` is opaque provenance (never folded into the id); the leaf declares no
    /// interface ports and no scalar params, so the content id is a pure function of the canonical bytes.
    pub fn to_composition_node(&self, intent: IntentRef) -> CompositionNode {
        CompositionNode::new(
            intent,
            NodeBody::Transduction {
                canonical: self.canonical_bytes(),
            },
            PortVector::from_slots(Vec::new()),
            Vec::new(),
        )
    }
}

/// Canonical serialization discriminant for [`ReferenceSource::PerceiverBodyAxis`].
const REF_TAG_PERCEIVER_BODY_AXIS: u8 = 0;
/// Canonical serialization discriminant for [`ReferenceSource::FloorConstant`].
const REF_TAG_FLOOR_CONSTANT: u8 = 1;
/// Canonical serialization discriminant for [`TransductionKernel::ReferenceOverAxis`].
const KERNEL_TAG_REFERENCE_OVER_AXIS: u8 = 0;

/// Append a length-prefixed UTF-8 string to a canonical byte buffer: a u64 little-endian byte count, then
/// the bytes. The prefix disambiguates field boundaries, so no two distinct field sequences collide. The
/// count is u64 (never u32), so the "no two distinct transductions share a byte stream" property holds for
/// any string a machine can hold, with no >4 GiB truncation caveat; it matches the u64 length prefix
/// `civsim_compose::NodeBody::Transduction` folds the whole serialization under.
fn write_canonical_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u64).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// The transduction registry: which single-axis transductions a world's beings run, in canonical order, the
/// data-declarable sibling of [`AffordancePerceptRegistry`] for the Tier-B sensor form. EMPTY by default
/// (opt-in, hash-neutral): a world that declares none carries no transduction block. This is the ISOLATED
/// mechanism; the live run-path wire that feeds a being's own body into the read (a `runner.rs` signature and
/// call-site change, behaviour-changing) is the coordinated follow-on, not built here.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TransductionRegistry {
    transductions: Vec<SingleAxisTransduction>,
}

impl TransductionRegistry {
    /// The empty registry (the opt-in default).
    pub fn empty() -> TransductionRegistry {
        TransductionRegistry {
            transductions: Vec::new(),
        }
    }

    /// Build from a list of declared transductions, in canonical order.
    pub fn from_transductions(transductions: Vec<SingleAxisTransduction>) -> TransductionRegistry {
        TransductionRegistry { transductions }
    }

    /// The declared transductions, in canonical order.
    pub fn transductions(&self) -> &[SingleAxisTransduction] {
        &self.transductions
    }

    /// The number of declared transductions.
    pub fn len(&self) -> usize {
        self.transductions.len()
    }

    /// Whether the registry declares none (the opt-in default).
    pub fn is_empty(&self) -> bool {
        self.transductions.is_empty()
    }

    /// Evaluate every declared transduction against one target and the perceiver's own body, in canonical
    /// order. Pure and RNG-free; the readers are the only inputs (see [`SingleAxisTransduction::transduce`]).
    pub fn transduce_all(
        &self,
        target: &impl Fn(&str) -> Fixed,
        own_body: &impl Fn(&str) -> Fixed,
        floor_const: &impl Fn(&str) -> Fixed,
    ) -> Vec<Fixed> {
        self.transductions
            .iter()
            .map(|t| t.transduce(target, own_body, floor_const))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal physics floor: the fracture and hardness material axes, a length axis (so the Pierce
    // capability caps derive), and three substances (hard granite, softer soil, and peat carrying neither
    // fracture nor hardness), enough to exercise the fracture-potential and sharpness derivations over real
    // substance data.
    const FLOOR: &str = r#"
[[axis]]
id = "mat.density"
measures = "mass per unit volume"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
real = "test fixture"

[[axis]]
id = "mat.fracture_strength"
measures = "the stress a substance fractures at"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mat.indentation_hardness"
measures = "the contact pressure a surface resists before plastic indentation"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "1"
range_hi = "150000"
real = "test fixture"

[[axis]]
id = "mech.length"
measures = "a characteristic length"
unit = "m"
dimension = "1,0,0,0"
scale = "m"
tier = 0
range_lo = "0"
range_hi = "100"
real = "test fixture"

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "20" },
  { axis = "mat.indentation_hardness", value = "5000" },
]

[[substance]]
id = "soil"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1500" },
  { axis = "mat.fracture_strength", value = "3" },
  { axis = "mat.indentation_hardness", value = "100" },
]

[[substance]]
id = "peat"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "400" },
]
"#;

    fn floor() -> PhysicsRegistry {
        PhysicsRegistry::from_toml_str(FLOOR).expect("test floor parses")
    }

    fn mix(pairs: &[(&str, i32)]) -> SubstanceMix {
        let mut m = SubstanceMix::new();
        for (s, v) in pairs {
            m.set(s, Fixed::from_int(*v));
        }
        m
    }

    fn tool(substance: &str, contact_area: Fixed) -> WieldedTool {
        WieldedTool {
            contact_area,
            volume: Fixed::ONE,
            length: Fixed::ONE,
            substance: substance.to_string(),
        }
    }

    #[test]
    fn fracture_potential_reads_weak_matter_high_and_strong_matter_low() {
        // Slice 2a: the fracture-potential percept is the graded, matter-derived reading of the same
        // contest the extraction gate resolves as a boolean. Weak soil (fracture strength 3) reads a high
        // potential (an ordinary actor's reference stress overwhelms it), hard granite (fracture strength
        // 20) reads a lower one, and both are strictly ordered by their own physics, never a kind tag.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs(); // reference stress 10
        let granite = mix(&[("granite", 4)]);
        let soil = mix(&[("soil", 4)]);

        let granite_pot =
            AffordancePerceptKind::FracturePotential.perceive(Some(&granite), None, &reg, &refs);
        let soil_pot =
            AffordancePerceptKind::FracturePotential.perceive(Some(&soil), None, &reg, &refs);
        // Reference stress 10 over granite's fracture strength 20 is one half; over soil's 3 it saturates
        // to one (the clamp), so soft matter reads maximal fracture-potential and hard rock reads half.
        assert_eq!(granite_pot, Fixed::from_ratio(1, 2));
        assert_eq!(soil_pot, Fixed::ONE);
        assert!(
            soil_pot > granite_pot,
            "the weaker matter reads the higher fracture-potential, by its own fracture strength"
        );
    }

    #[test]
    fn matter_with_no_fracture_axis_and_no_matter_read_zero() {
        // The clean degrade: matter that carries no fracture strength (peat, no such axis) and an empty
        // cell both read zero potential, so there is nothing spurious to break where the physics is silent.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs();
        let peat = mix(&[("peat", 4)]);
        assert_eq!(
            AffordancePerceptKind::FracturePotential.perceive(Some(&peat), None, &reg, &refs),
            Fixed::ZERO,
            "matter with no fracture strength is not fracturable"
        );
        // The registry-level perceive over an absent or empty material layer reads a flat zero vector.
        let registry =
            AffordancePerceptRegistry::from_kinds(&[AffordancePerceptKind::FracturePotential]);
        assert_eq!(
            registry.perceive(None, None, &reg, &refs),
            vec![Fixed::ZERO]
        );
        assert_eq!(
            registry.perceive(Some(&SubstanceMix::new()), None, &reg, &refs),
            vec![Fixed::ZERO]
        );
    }

    #[test]
    fn sharpness_reads_a_keen_tool_edge_and_zero_without_a_tool() {
        // Slice 2a: sharpness is the Pierce capability score over the being's WIELDED tool, its own edge
        // geometry (a small working contact area concentrates force) and material, the same read the tool's
        // worked capability is derived from. A keen hard edge scores positive; a being with no tool reads
        // zero (no edge to sense), because a keen edge is a property of a shaped object, not of loose matter.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs();
        // A knapped granite point: a tiny working area, a hard material.
        let point = tool("granite", Fixed::from_ratio(1, 1_000_000)); // 1e-6 m^2, a fine knapped edge
        let keen = AffordancePerceptKind::Sharpness.perceive(None, Some(&point), &reg, &refs);
        assert!(
            keen > Fixed::ZERO,
            "a keen hard tool edge presents a positive sharpness"
        );
        assert!(keen <= Fixed::ONE, "sharpness is a [0, 1] capability score");
        // No tool in hand: nothing sharp to sense.
        assert_eq!(
            AffordancePerceptKind::Sharpness.perceive(None, None, &reg, &refs),
            Fixed::ZERO,
            "a being with no tool senses no sharpness"
        );
        // A blunt tool (a wide working area spreads the same force thin) is no sharper than the keen point,
        // and typically less, so the percept ranks a concentrated edge over a spread one by geometry alone.
        let blunt = tool("granite", Fixed::ONE); // a 1 m^2 slab face
        let dull = AffordancePerceptKind::Sharpness.perceive(None, Some(&blunt), &reg, &refs);
        assert!(
            keen >= dull,
            "a concentrated edge is at least as sharp as a spread one, by the Pierce geometry"
        );
    }

    #[test]
    fn a_keen_edged_tool_reads_the_pierce_capability_a_body_lacks() {
        // The made-world arc, tool-use, slice 1: a wielded tool's capability enters the SAME dispatch a body
        // part's does, so the afford gate can read what a tool grants. A keen edge (a small working contact
        // area concentrates force) reads a positive PIERCE the gate can grant a CUT on; a broad blunt face of
        // the same substance spreads the force and reads no more, so the reading tracks the edge geometry, by
        // physics not an `IsAxe` tag. What the tool's MATERIAL hardness distinguishes (a keen granite edge cuts
        // where a soft point of the same geometry deforms) is not the AFFORD gate but the cut's EFFECTIVENESS,
        // gated in the enact next slice: this Pierce kernel reads a tiny-area edge as maximally piercing
        // whatever its hardness, so a tool that AFFORDS a cut is one with an edge, and whether the cut BITES is
        // the enact's physics.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs();
        let caps = CapabilityCaps::derive(&reg);
        let pierce = |t: &WieldedTool| {
            tool_capability(
                t,
                &reg,
                &refs.capability,
                &caps,
                FunctionLawRegistry::ID_PIERCE,
            )
        };

        let keen = tool("granite", Fixed::from_ratio(1, 1_000_000)); // a fine knapped edge
        let blunt = tool("granite", Fixed::ONE); // a broad slab face

        assert!(
            pierce(&keen) > Fixed::ZERO,
            "a keen edge reads a positive pierce the afford gate can grant a cut on"
        );
        assert!(
            pierce(&keen) >= pierce(&blunt),
            "a concentrated edge reads at least as much pierce as a spread blunt face, by geometry"
        );
    }

    #[test]
    fn the_registry_is_opt_in_and_canonically_ordered() {
        // The opt-in default: an empty registry declares no percepts and perceives an empty vector, so a
        // world that names none carries no affordance-percept block (hash-neutral). A populated one slots
        // its kinds by position in canonical id order, exactly as the feature registry does, and its
        // material-property and shape-dependent channels read their own sources side by side.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs();
        let empty = AffordancePerceptRegistry::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty
            .perceive(Some(&mix(&[("soil", 4)])), None, &reg, &refs)
            .is_empty());

        let registry = AffordancePerceptRegistry::from_kinds(&[
            AffordancePerceptKind::FracturePotential,
            AffordancePerceptKind::Sharpness,
        ]);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.percepts()[0].id, AffordancePerceptId(0));
        assert_eq!(registry.percepts()[1].id, AffordancePerceptId(1));
        assert_eq!(
            registry.percepts()[0].kind.axis(),
            "mat.fracture_strength",
            "the fracture percept names the material axis it keys off, not a label"
        );
        assert_eq!(
            registry.percepts()[1].kind.axis(),
            "mech.contact_area",
            "the sharpness percept names the geometry axis it keys off, not a label"
        );
        // A being standing on soft soil with a keen granite point in hand reads both channels lit: a high
        // fracture-potential underfoot and a positive sharpness in hand, the pair a binding graph (slice 2b)
        // will bind into a strike.
        let out = registry.perceive(
            Some(&mix(&[("soil", 4)])),
            Some(&tool("granite", Fixed::from_ratio(1, 1_000_000))),
            &reg,
            &refs,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            Fixed::ONE,
            "soft soil reads maximal fracture-potential"
        );
        assert!(
            out[1] > Fixed::ZERO,
            "the keen point reads positive sharpness"
        );
    }

    #[test]
    fn oilseed_reads_a_positive_fracture_potential_and_carries_energy_density() {
        // Ideation viability arc, slice A: the `oilseed` substance the viability world uses to close the
        // discovery loop is at once FRACTURABLE (a positive FracturePotential an ordinary forager can act
        // on) and ENERGY-DENSE (bio.energy_density, the kernel's assimilable energy). Proven against the
        // real embedded ground floor, so the authored environment carries both halves the loop needs: the
        // percept that makes the being propose to break it, and the energy its extraction-and-ingestion
        // pays off with (the ingest coupling in slice B turns the energy into a reserve rise).
        let reg = PhysicsRegistry::ground().expect("the embedded ground floor loads");
        let refs = AffordancePerceptRefs::dev_refs();
        let oilseed = mix(&[("oilseed", 4)]);
        let potential =
            AffordancePerceptKind::FracturePotential.perceive(Some(&oilseed), None, &reg, &refs);
        assert!(
            potential > Fixed::ZERO,
            "oilseed reads a positive fracture-potential, so the discovery loop proposes acting on it"
        );
        let energy = reg
            .substance("oilseed")
            .and_then(|s| s.vector.get("bio.energy_density").copied())
            .unwrap_or(Fixed::ZERO);
        assert!(
            energy > Fixed::ZERO,
            "oilseed carries a positive bio.energy_density, the nutrition the loop's payoff draws on"
        );
    }

    #[test]
    fn from_names_selects_the_existing_kernels_and_is_byte_identical_to_from_kinds() {
        // The data-source path: a world NAMES which of the fixed kernels its beings perceive, and the result
        // is the SAME registry the enum-valued constructor builds, so the string name is a resolution key
        // only and nothing of it survives into the stored registry (the name-does-not-persist half of the
        // opaque-slot invariant; the downstream-reads-by-slot half holds in discovery.rs, not here).
        let by_name = AffordancePerceptRegistry::from_names(&["fracture_potential", "sharpness"])
            .expect("known kernel names resolve");
        let by_kind = AffordancePerceptRegistry::from_kinds(&[
            AffordancePerceptKind::FracturePotential,
            AffordancePerceptKind::Sharpness,
        ]);
        assert_eq!(
            by_name, by_kind,
            "the name path yields the identical opaque-slot registry the enum path does"
        );
        // The declared ORDER is the slot order: a different declared order is a different registry, so the
        // world's ordering choice is data, and it maps into the canonical slot ids by position.
        let reordered = AffordancePerceptRegistry::from_names(&["sharpness", "fracture_potential"])
            .expect("known names resolve");
        assert_ne!(
            reordered, by_kind,
            "a different declared order assigns different slots, so order is world-declared data"
        );
    }

    #[test]
    fn from_names_fails_loud_on_an_unknown_name() {
        // Never a silent plausible default: a name that resolves to no fixed kernel is an error naming the
        // offending kernel, so a typo in world data cannot silently drop a percept. A name is not a request
        // to author a kernel; the kernel set is closed Rust.
        let err =
            AffordancePerceptRegistry::from_names(&["fracture_potential", "levitation_potential"])
                .expect_err("an unknown kernel name fails loud");
        assert!(
            err.contains("levitation_potential"),
            "the error names the offending kernel, not a silent drop: {err}"
        );
    }

    #[test]
    fn every_kernel_name_round_trips_and_the_def_carries_no_name_to_branch_on() {
        // name() and from_name() are inverse over the WHOLE kernel set: the loop runs over
        // AffordancePerceptKind::ALL, from which from_name() derives, so the coverage is total over the
        // declared set and a name()/from_name() mismatch on a future kernel is impossible by construction. The
        // resolved def exposes only the opaque slot id and the kernel handle, never the name string, so a
        // downstream consumer has no name to branch on (the template-case guard, enforced by the type).
        for kind in AffordancePerceptKind::ALL {
            assert_eq!(
                AffordancePerceptKind::from_name(kind.name()),
                Some(kind),
                "{} round-trips through name()/from_name()",
                kind.name()
            );
        }
        let reg = AffordancePerceptRegistry::from_names(&["sharpness"]).expect("resolves");
        let def = reg.percepts()[0];
        assert_eq!(
            def.id,
            AffordancePerceptId(0),
            "the def carries the opaque slot id"
        );
        assert_eq!(
            def.kind,
            AffordancePerceptKind::Sharpness,
            "the def carries the kernel handle, the only thing perceive() reads; there is no name field"
        );
    }

    #[test]
    fn perceiver_keyed_reference_makes_a_large_body_read_differently_than_a_small_one() {
        // Tier B, the perceiver-keyed reference proven: the SAME matter transduces DIFFERENTLY for a small
        // body and a large body, because the reference is read from the perceiving being's OWN body, not a
        // shared embodiment-wide scale. This is what closes the perceiver-independent-reference seam and
        // admits the alien as data (a different body is a different reference, no rewrite).
        let t = SingleAxisTransduction {
            target_axis: "mat.fracture_strength".to_string(),
            reference: ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
            kernel: TransductionKernel::ReferenceOverAxis,
        };
        let rock = |axis: &str| {
            if axis == "mat.fracture_strength" {
                Fixed::from_int(20)
            } else {
                Fixed::ZERO
            }
        };
        let small_body = |axis: &str| {
            if axis == "body.delivered_stress" {
                Fixed::from_int(5)
            } else {
                Fixed::ZERO
            }
        };
        let large_body = |axis: &str| {
            if axis == "body.delivered_stress" {
                Fixed::from_int(20)
            } else {
                Fixed::ZERO
            }
        };
        let no_const = |_: &str| Fixed::ZERO;

        let small = t.transduce(&rock, &small_body, &no_const);
        let large = t.transduce(&rock, &large_body, &no_const);
        // reference over strength: small body 5/20 is one quarter; large body 20/20 saturates to one (the
        // giant finds the rock fully actionable, the small body only a quarter), monotone in the perceiver's
        // own capacity, by its own data.
        assert_eq!(small, Fixed::from_ratio(1, 4));
        assert_eq!(large, Fixed::ONE);
        assert!(
            large > small,
            "a stronger body reads the same matter as more actionable, keyed on its own body"
        );
    }

    #[test]
    fn the_reference_is_a_resolved_read_never_a_stored_literal() {
        // The value-authoring line enforced by the TYPE: [`ReferenceSource`] holds an id, never a `Fixed`, so
        // a reference is always RESOLVED at evaluation, never a baked number. Proven behaviourally here: the
        // same transduction against two different floor-constant resolvers yields two different results, so no
        // literal is stored on the transduction. The ABSENCE of a literal is a compile-time guarantee: no
        // `ReferenceSource` variant and no field accepts a `Fixed`, so an author cannot write a bare number.
        let t = SingleAxisTransduction {
            target_axis: "mat.fracture_strength".to_string(),
            reference: ReferenceSource::FloorConstant("floor.reference_stress".to_string()),
            kernel: TransductionKernel::ReferenceOverAxis,
        };
        let rock = |axis: &str| {
            if axis == "mat.fracture_strength" {
                Fixed::from_int(20)
            } else {
                Fixed::ZERO
            }
        };
        let no_body = |_: &str| Fixed::ZERO;
        let floor_low = |id: &str| {
            if id == "floor.reference_stress" {
                Fixed::from_int(5)
            } else {
                Fixed::ZERO
            }
        };
        let floor_high = |id: &str| {
            if id == "floor.reference_stress" {
                Fixed::from_int(20)
            } else {
                Fixed::ZERO
            }
        };

        let low = t.transduce(&rock, &no_body, &floor_low);
        let high = t.transduce(&rock, &no_body, &floor_high);
        assert_eq!(low, Fixed::from_ratio(1, 4));
        assert_eq!(high, Fixed::ONE);
        assert!(
            high > low,
            "the reference is read from the floor resolver, not baked into the transduction"
        );
    }

    #[test]
    fn transduce_degrades_cleanly_and_stays_in_the_unit_interval() {
        // Absent target axis (magnitude zero) reads zero (nothing to sense); a reference far exceeding the
        // axis clamps to one; every result stays in [0, 1].
        let t = SingleAxisTransduction {
            target_axis: "mat.fracture_strength".to_string(),
            reference: ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
            kernel: TransductionKernel::ReferenceOverAxis,
        };
        let strong = |axis: &str| {
            if axis == "body.delivered_stress" {
                Fixed::from_int(1000)
            } else {
                Fixed::ZERO
            }
        };
        let absent = |_: &str| Fixed::ZERO; // no target axis present
        let soft = |axis: &str| {
            if axis == "mat.fracture_strength" {
                Fixed::from_int(1)
            } else {
                Fixed::ZERO
            }
        };
        let none_const = |_: &str| Fixed::ZERO;

        assert_eq!(
            t.transduce(&absent, &strong, &none_const),
            Fixed::ZERO,
            "no target-axis magnitude, nothing to sense"
        );
        let saturated = t.transduce(&soft, &strong, &none_const);
        assert_eq!(
            saturated,
            Fixed::ONE,
            "a reference far above the axis saturates to one"
        );
        assert!(
            saturated >= Fixed::ZERO && saturated <= Fixed::ONE,
            "the transduction stays in the unit interval"
        );
    }

    #[test]
    fn reference_over_axis_saturation_is_sign_aware_on_overflow() {
        // The overflow fallback saturates toward the clamp bound in the TRUE direction: a large positive ratio
        // (a positive reference over a near-zero magnitude, whose quotient overflows the fixed-point range)
        // reads one, and a large NEGATIVE ratio (a negative reference, which the type permits from a signed
        // axis) reads zero, never a spurious one. This exercises the `checked_div` None (overflow) branch the
        // other tests do not reach, and pins the sign-aware saturation.
        let tiny = Fixed::from_ratio(1, 1_000_000_000); // small enough that a small reference overflows the ratio
        let target = move |_: &str| tiny;
        let t = SingleAxisTransduction {
            target_axis: "mat.fracture_strength".to_string(),
            reference: ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
            kernel: TransductionKernel::ReferenceOverAxis,
        };
        let none_const = |_: &str| Fixed::ZERO;
        let positive_ref = |_: &str| Fixed::from_int(10);
        let negative_ref = |_: &str| Fixed::from_int(-10);
        assert_eq!(
            t.transduce(&target, &positive_ref, &none_const),
            Fixed::ONE,
            "a large positive ratio saturates to one"
        );
        assert_eq!(
            t.transduce(&target, &negative_ref, &none_const),
            Fixed::ZERO,
            "a large negative ratio saturates to zero, not a spurious one (sign-aware overflow)"
        );
    }

    #[test]
    fn the_transduction_registry_is_opt_in_and_canonically_ordered() {
        // Opt-in default: an empty registry transduces an empty vector (hash-neutral). A populated one
        // evaluates its declared transductions in canonical order, each keyed on its own reference source.
        let empty = TransductionRegistry::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        let no = |_: &str| Fixed::ZERO;
        assert!(empty.transduce_all(&no, &no, &no).is_empty());

        let reg = TransductionRegistry::from_transductions(vec![
            SingleAxisTransduction {
                target_axis: "mat.fracture_strength".to_string(),
                reference: ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
                kernel: TransductionKernel::ReferenceOverAxis,
            },
            SingleAxisTransduction {
                target_axis: "mat.compressive_strength".to_string(),
                reference: ReferenceSource::FloorConstant("floor.reference_stress".to_string()),
                kernel: TransductionKernel::ReferenceOverAxis,
            },
        ]);
        assert_eq!(reg.len(), 2);
        let target = |axis: &str| match axis {
            "mat.fracture_strength" => Fixed::from_int(20),
            "mat.compressive_strength" => Fixed::from_int(10),
            _ => Fixed::ZERO,
        };
        let body = |axis: &str| {
            if axis == "body.delivered_stress" {
                Fixed::from_int(20)
            } else {
                Fixed::ZERO
            }
        };
        let konst = |id: &str| {
            if id == "floor.reference_stress" {
                Fixed::from_int(5)
            } else {
                Fixed::ZERO
            }
        };
        let out = reg.transduce_all(&target, &body, &konst);
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            Fixed::ONE,
            "fracture: own-body reference 20 over strength 20 saturates to one"
        );
        assert_eq!(
            out[1],
            Fixed::from_ratio(1, 2),
            "compressive: floor reference 5 over strength 10 is one half"
        );
    }

    fn td(target: &str, reference: ReferenceSource) -> SingleAxisTransduction {
        SingleAxisTransduction {
            target_axis: target.to_string(),
            reference,
            kernel: TransductionKernel::ReferenceOverAxis,
        }
    }

    #[test]
    fn canonical_bytes_are_stable_and_mint_a_stable_node_id() {
        // The stability contract the composer leaf rests on: an identical transduction serializes to the
        // identical bytes, and those bytes mint the identical content id, on every construction.
        let a = td(
            "mat.fracture_strength",
            ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
        );
        let b = td(
            "mat.fracture_strength",
            ReferenceSource::PerceiverBodyAxis("body.delivered_stress".to_string()),
        );
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(
            a.to_composition_node(IntentRef(0)).content_id(),
            b.to_composition_node(IntentRef(0)).content_id(),
        );
    }

    #[test]
    fn the_intent_is_opaque_provenance_and_never_shifts_the_content_id() {
        // The tag-3 leaf keeps the pure-content, intent-omitted contract: two nodes for the same
        // transduction reached from different intents deduplicate to one design.
        let t = td(
            "mat.fracture_strength",
            ReferenceSource::FloorConstant("floor.reference_stress".to_string()),
        );
        assert_eq!(
            t.to_composition_node(IntentRef(1)).content_id(),
            t.to_composition_node(IntentRef(999)).content_id(),
        );
    }

    #[test]
    fn a_different_field_mints_a_different_node_and_the_reference_kind_disambiguates() {
        // Distinctness across every field, and the discriminant guard: the SAME id string under a body
        // axis versus a floor constant is two different sensors, so the reference kind must separate them.
        let base = td(
            "mat.fracture_strength",
            ReferenceSource::PerceiverBodyAxis("x".to_string()),
        );
        let other_axis = td(
            "mat.compressive_strength",
            ReferenceSource::PerceiverBodyAxis("x".to_string()),
        );
        let other_ref_id = td(
            "mat.fracture_strength",
            ReferenceSource::PerceiverBodyAxis("y".to_string()),
        );
        let other_ref_kind = td(
            "mat.fracture_strength",
            ReferenceSource::FloorConstant("x".to_string()),
        );
        let id = |t: &SingleAxisTransduction| t.to_composition_node(IntentRef(0)).content_id();
        assert_ne!(id(&base), id(&other_axis), "different target axis");
        assert_ne!(id(&base), id(&other_ref_id), "different reference id");
        assert_ne!(
            id(&base),
            id(&other_ref_kind),
            "same id string, different reference kind, is a different sensor"
        );
    }

    #[test]
    fn the_length_prefix_stops_a_field_boundary_collision() {
        // Without the length prefix, moving a character across the target/reference boundary would leave
        // the concatenation unchanged. The prefix makes ("ab", "c") and ("a", "bc") distinct sensors.
        let split_one = td("ab", ReferenceSource::PerceiverBodyAxis("c".to_string()));
        let split_two = td("a", ReferenceSource::PerceiverBodyAxis("bc".to_string()));
        assert_ne!(split_one.canonical_bytes(), split_two.canonical_bytes());
    }
}
