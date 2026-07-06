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
//! `civsim_compose::CapabilityKernel`); the MEMBERSHIP (which scalars a world perceives) is the registry's
//! data, so a new sensible affordance percept is a data edit, never a code branch (Principle 11). Empty by
//! default, so a world that declares none carries no affordance-percept block and every run hash is
//! unchanged (the opt-in, hash-neutral pattern the feature block established).
//!
//! This slice is READ only: the derivation and the registry sit off the run path (nothing perceives yet,
//! and `state_hash` folds nothing), so every existing scenario replays bit-for-bit. Piece 2's binding
//! graph (slice 2b) reads these scalars to sample candidate actions, and the WIRE (slice 2c) feeds them
//! into the controller alongside the feature and appetitive blocks.

use civsim_core::Fixed;
use civsim_physics::PhysicsRegistry;

use crate::material::SubstanceMix;

/// The fracture-strength material axis the fracture-potential kernel reads: the stress a substance
/// fractures at, the same axis the extraction contest gates on ([`crate::material`]).
const AXIS_FRACTURE: &str = "mat.fracture_strength";

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
    pub reference_stress: Fixed,
}

impl AffordancePerceptRefs {
    /// A labelled dev fixture for the unit tests and the pre-wire scenarios: a reference actionable stress
    /// in the tens-of-megapascals range an ordinary limb-and-stone actor delivers, so soft matter reads a
    /// high fracture-potential and hard rock a low one. The manifest value is reserved; this is only the
    /// fixture, never the canonical number.
    pub fn dev_refs() -> AffordancePerceptRefs {
        AffordancePerceptRefs {
            reference_stress: Fixed::from_int(10),
        }
    }
}

/// The closed set of physics-derived affordance scalars a being can perceive over nearby matter. Fixed
/// Rust, one variant per kernel, mirroring `civsim_compose::CapabilityKernel`; which kernels a world runs
/// is the registry's data below.
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
}

impl AffordancePerceptKind {
    /// Derive this scalar in `[0, 1]` over the nearby matter through the physics registry and the reserved
    /// reference levels. Pure and RNG-free. Zero where there is no matter of the kernel's axis to read,
    /// the clean degrade.
    pub fn perceive(
        self,
        matter: &SubstanceMix,
        reg: &PhysicsRegistry,
        refs: &AffordancePerceptRefs,
    ) -> Fixed {
        match self {
            AffordancePerceptKind::FracturePotential => {
                let strength = matter.fracture_hardness(reg);
                if strength <= Fixed::ZERO {
                    return Fixed::ZERO; // no fracturable matter present
                }
                refs.reference_stress
                    .checked_div(strength)
                    .unwrap_or(Fixed::ONE)
                    .clamp(Fixed::ZERO, Fixed::ONE)
            }
        }
    }

    /// The material axis this kernel reads, so the belief-subject key (slice 2b) and a diagnostic can name
    /// the physics quantity the percept keys off, never a hardcoded label (Principle 11).
    pub fn axis(self) -> &'static str {
        match self {
            AffordancePerceptKind::FracturePotential => AXIS_FRACTURE,
        }
    }
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
    /// [`crate::percept::PerceptRegistry::from_classes`] slots the raw-feature channels. The membership is
    /// data: a world adds a sensible affordance percept by naming another kind, never by a code change.
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

    /// Perceive the nearby matter: the `[0, 1]` physics scalar for each declared percept, in canonical id
    /// order, zero where no matter is present. A pure read of the matter's own physics against the reserved
    /// references (Principles 9, 10), RNG-free. `matter` is the [`SubstanceMix`] of the cell the being
    /// stands on (or reaches), `None` (or empty) where the world declares no material layer, in which case
    /// every channel reads zero (the clean degrade, so an opted-out world is unchanged).
    pub fn perceive(
        &self,
        matter: Option<&SubstanceMix>,
        reg: &PhysicsRegistry,
        refs: &AffordancePerceptRefs,
    ) -> Vec<Fixed> {
        self.percepts
            .iter()
            .map(|p| match matter {
                Some(m) if !m.is_empty() => p.kind.perceive(m, reg, refs),
                _ => Fixed::ZERO,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal physics floor: the fracture axis plus three substances of falling fracture strength
    // (granite the strongest, soil weaker, peat carrying no fracture axis at all), enough to exercise the
    // fracture-potential derivation over real substance data.
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

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.fracture_strength", value = "20" },
]

[[substance]]
id = "soil"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1500" },
  { axis = "mat.fracture_strength", value = "3" },
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

        let granite_pot = AffordancePerceptKind::FracturePotential.perceive(&granite, &reg, &refs);
        let soil_pot = AffordancePerceptKind::FracturePotential.perceive(&soil, &reg, &refs);
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
            AffordancePerceptKind::FracturePotential.perceive(&peat, &reg, &refs),
            Fixed::ZERO,
            "matter with no fracture strength is not fracturable"
        );
        // The registry-level perceive over an absent or empty material layer reads a flat zero vector.
        let registry =
            AffordancePerceptRegistry::from_kinds(&[AffordancePerceptKind::FracturePotential]);
        assert_eq!(registry.perceive(None, &reg, &refs), vec![Fixed::ZERO]);
        assert_eq!(
            registry.perceive(Some(&SubstanceMix::new()), &reg, &refs),
            vec![Fixed::ZERO]
        );
    }

    #[test]
    fn the_registry_is_opt_in_and_canonically_ordered() {
        // The opt-in default: an empty registry declares no percepts and perceives an empty vector, so a
        // world that names none carries no affordance-percept block (hash-neutral). A populated one slots
        // its kinds by position in canonical id order, exactly as the feature registry does.
        let reg = floor();
        let refs = AffordancePerceptRefs::dev_refs();
        let empty = AffordancePerceptRegistry::empty();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        assert!(empty
            .perceive(Some(&mix(&[("soil", 4)])), &reg, &refs)
            .is_empty());

        let registry =
            AffordancePerceptRegistry::from_kinds(&[AffordancePerceptKind::FracturePotential]);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.percepts()[0].id, AffordancePerceptId(0));
        assert_eq!(
            registry.percepts()[0].kind,
            AffordancePerceptKind::FracturePotential
        );
        assert_eq!(
            registry.percepts()[0].kind.axis(),
            "mat.fracture_strength",
            "the percept names the physics axis it keys off, not a label"
        );
    }
}
