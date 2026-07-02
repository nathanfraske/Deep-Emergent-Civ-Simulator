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

//! The per-part body, wounds, and fluids (design Part 35; R-BUILD-PHYS, R-WOUND, R-FLUID; reads the
//! resolved mechanical-and-materials floor R-PHYS-MECH and biology floor R-PHYS-BIO; Principles 3, 9,
//! 11).
//!
//! A body is an assembly of typed parts made of layered tissue materials, promoted from the aggregate
//! anatomy vector (Part 25.14, [`crate::anatomy::BodyPlan`]) on demand (Principle 1, the aggregate
//! stays statistics until an individual warrants a full body). A wound is not a stored kind: it is the
//! measured consequence of a physical insult meeting a part's tissue, computed by the resolved floor
//! laws ([`civsim_physics::laws`]) and recorded as the part's condition change plus a measured outcome,
//! the on-ideology endpoint sibling to edibility-as-a-measured-relation (design.md Part 25.13). The
//! damage-mode taxonomy is a data registry rather than a closed enum, so a magical world adds
//! corrosion or a curse as data (the R-WOUND ask); the measurement family the engine runs against the
//! tissue is fixed Rust, the same split as the affordance registry (membership is data, enactment is
//! physics). A body fluid is a data-defined kind with a composition and a conserved volume stock, its
//! loss a wound consequence (the R-FLUID ask), so a race need not bleed blood or bleed at all.
//!
//! There is no separate health scalar (Part 35): a body is alive while no vital part is destroyed and
//! no fluid pool is past its critical fraction, and [`Body::integrity`] is the derived aggregate any
//! decision layer reads, never a competing store. Everything here is integer fixed-point with no float
//! in canonical state and no randomness (a wound is a deterministic function of the insult and the
//! tissue), so a body's condition reproduces bit for bit (Principle 3). What the physics needs is
//! reserved with its basis (`body.*` in the calibration manifest) and defaulted only by a labelled
//! development fixture; the tissue properties, the damage caps, and the fluid criticals are the
//! owner's to set, never fabricated (Principle 11).

use civsim_core::Fixed;
use civsim_physics::laws;

use crate::anatomy::BodyPlan;
use crate::stocks::Stock;

// ============================================================================================
// Tissue materials: data over the resolved material floor axes (mat.*/therm.*).
// ============================================================================================

/// A tissue-material id, minted through the registry (extensible, never a closed enum).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TissueMaterialId(pub u16);

/// A tissue material: the property vector over the resolved R-PHYS-MECH material axes that the wound
/// laws read (`crates/physics/data/mechanical_floor.toml`). A tissue is a material with properties
/// (Part 19, Part 41); which materials exist is data, the laws that read them are fixed. All values
/// are on the floor's scales (strengths in MPa, energies on the floor's energy scale).
#[derive(Clone, Debug)]
pub struct TissueMaterial {
    pub id: TissueMaterialId,
    /// A legibility handle, never read by a law.
    pub name: String,
    /// Indentation hardness (MPa): the pressure a cut must exceed to bite (`mat.indentation_hardness`).
    pub hardness: Fixed,
    /// Fracture strength (MPa): the stress at which a brittle part breaks (`mat.fracture_strength`).
    pub fracture_strength: Fixed,
    /// Fracture energy on the floor scale: the energy a crack area needs (`mat.fracture_energy`).
    pub fracture_energy: Fixed,
    /// Specific cutting energy on the floor scale: the work per swept volume to cut
    /// (`mat.specific_cut_energy`).
    pub specific_cut_energy: Fixed,
    /// Elastic modulus (MPa), for the thermal-stress law (`mat.elastic_modulus`).
    pub elastic_modulus: Fixed,
    /// Linear thermal-expansion coefficient (`therm.expansion`).
    pub expansion: Fixed,
}

/// The set of tissue materials a world runs, data-defined and extensible.
#[derive(Clone, Debug, Default)]
pub struct TissueRegistry {
    pub materials: Vec<TissueMaterial>,
}

/// The dev-fixture tissue materials (leaf ids). Not owner canon; the authoritative values are
/// reserved (`body.tissue_*`), these only let the model run and be tested.
pub const HIDE: TissueMaterialId = TissueMaterialId(0);
pub const FLESH: TissueMaterialId = TissueMaterialId(1);
pub const BONE: TissueMaterialId = TissueMaterialId(2);
pub const ORGAN: TissueMaterialId = TissueMaterialId(3);

impl TissueRegistry {
    /// A labelled DEVELOPMENT FIXTURE registry of tissue materials, the layers a vertebrate-like body
    /// runs on, with reserved-with-basis properties (real datasheets: cortical bone is hard and
    /// strong, flesh soft, hide tough-but-soft). Not owner values.
    pub fn dev_default() -> TissueRegistry {
        let m = |id, name: &str, h, fs, fe, sce, em, ex| TissueMaterial {
            id,
            name: name.to_string(),
            hardness: h,
            fracture_strength: fs,
            fracture_energy: fe,
            specific_cut_energy: sce,
            elastic_modulus: em,
            expansion: ex,
        };
        TissueRegistry {
            materials: vec![
                // hide: low hardness, tough (moderate cut energy), low fracture strength.
                m(
                    HIDE,
                    "hide",
                    Fixed::from_ratio(2, 1),
                    Fixed::from_ratio(10, 1),
                    Fixed::from_ratio(3, 1),
                    Fixed::from_ratio(4, 1),
                    Fixed::from_ratio(50, 1),
                    Fixed::from_ratio(1, 100),
                ),
                // flesh: soft, easily cut.
                m(
                    FLESH,
                    "flesh",
                    Fixed::from_ratio(1, 1),
                    Fixed::from_ratio(3, 1),
                    Fixed::from_ratio(1, 1),
                    Fixed::from_ratio(2, 1),
                    Fixed::from_ratio(10, 1),
                    Fixed::from_ratio(2, 100),
                ),
                // bone: hard, strong, brittle (fractures rather than cuts).
                m(
                    BONE,
                    "bone",
                    Fixed::from_ratio(120, 1),
                    Fixed::from_ratio(150, 1),
                    Fixed::from_ratio(8, 1),
                    Fixed::from_ratio(40, 1),
                    Fixed::from_ratio(18000, 1000),
                    Fixed::from_ratio(1, 100),
                ),
                // organ: very soft.
                m(
                    ORGAN,
                    "organ",
                    Fixed::from_ratio(1, 2),
                    Fixed::from_ratio(2, 1),
                    Fixed::from_ratio(1, 2),
                    Fixed::from_ratio(1, 1),
                    Fixed::from_ratio(5, 1),
                    Fixed::from_ratio(2, 100),
                ),
            ],
        }
    }

    /// The material for an id, if registered.
    pub fn material(&self, id: TissueMaterialId) -> Option<&TissueMaterial> {
        self.materials.iter().find(|m| m.id == id)
    }
}

// ============================================================================================
// Damage modes: a data registry keyed to a fixed physics measurement family.
// ============================================================================================

/// The physics measurement the engine runs to turn an insult into damage. This is the fixed engine
/// interface (the ways the floor knows how to measure harm), on the same footing as the affordance
/// enactment or the RNG phase set: the family is fixed Rust, the membership of the [`DamageModeRegistry`]
/// is data (Principle 11). A mode whose measurement is [`MeasureKind::Exotic`] has no built law and is
/// a reserved, audited floor extension (the fantasy modes corrosion, freeze, curse), never a fabricated
/// effect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeasureKind {
    /// Penetration: contact pressure then cut/penetration depth (cut and pierce, geometry-differentiated).
    Penetration,
    /// Fracture: the dual stress-and-energy fracture criterion (blunt force).
    Fracture,
    /// Thermal: constrained thermal stress from a temperature rise (burn).
    Thermal,
    /// Exotic: a mode whose floor law is not yet built (a reserved, audited extension). Measured as no
    /// damage rather than a fabricated one.
    Exotic,
}

/// A damage-mode id, minted through the registry.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DamageModeId(pub u16);

/// One damage mode as data: which measurement family the engine runs against the tissue.
#[derive(Clone, Debug)]
pub struct DamageModeDef {
    pub id: DamageModeId,
    pub name: String,
    pub measure: MeasureKind,
}

/// The set of damage modes a world runs, data-defined and extensible.
#[derive(Clone, Debug, Default)]
pub struct DamageModeRegistry {
    pub modes: Vec<DamageModeDef>,
}

/// The dev-fixture damage modes.
pub const CUT: DamageModeId = DamageModeId(0);
pub const PIERCE: DamageModeId = DamageModeId(1);
pub const BLUNT: DamageModeId = DamageModeId(2);
pub const BURN: DamageModeId = DamageModeId(3);

impl DamageModeRegistry {
    /// A labelled DEVELOPMENT FIXTURE: the grounded modes (cut, pierce, blunt, burn), each mapped to
    /// the resolved floor measurement. A magical world adds corrosion, freeze, or a curse as data with
    /// [`MeasureKind::Exotic`] (a reserved floor extension). Cut and pierce share the penetration
    /// measurement, differing only by the insult's contact area (the crush-versus-pierce geometry).
    pub fn dev_default() -> DamageModeRegistry {
        let d = |id, name: &str, measure| DamageModeDef {
            id,
            name: name.to_string(),
            measure,
        };
        DamageModeRegistry {
            modes: vec![
                d(CUT, "cut", MeasureKind::Penetration),
                d(PIERCE, "pierce", MeasureKind::Penetration),
                d(BLUNT, "blunt", MeasureKind::Fracture),
                d(BURN, "burn", MeasureKind::Thermal),
            ],
        }
    }

    /// The measurement family of a mode, or [`MeasureKind::Exotic`] for an unregistered one (so an
    /// unknown mode degrades to a reserved no-op rather than a panic).
    pub fn measure(&self, id: DamageModeId) -> MeasureKind {
        self.modes
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.measure)
            .unwrap_or(MeasureKind::Exotic)
    }
}

// ============================================================================================
// Body functions.
// ============================================================================================

/// A data-defined body function a part provides (grip, sight, locomotion, a natural weapon). Losing
/// the part costs its functions. Membership is data (Part 40); the id is the handle the rest of the
/// engine reads.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FunctionId(pub u16);

/// The dev-fixture functions.
pub const F_LOCOMOTION: FunctionId = FunctionId(0);
pub const F_SIGHT: FunctionId = FunctionId(1);
pub const F_STRIKE: FunctionId = FunctionId(2);
pub const F_VITAL_CORE: FunctionId = FunctionId(3);

// ============================================================================================
// Fluids: a data-defined kind with a composition and a conserved volume stock (R-FLUID).
// ============================================================================================

/// A fluid-kind id, minted through the registry (extensible, never a closed enum). Replaces the old
/// `FluidKind` enum; a race runs on blood, ichor, sap, or nothing, as data.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FluidKindId(pub u16);

/// One fluid kind as data: what a race runs on, as a composition over the biology floor plus the
/// dynamics its loss and interactions need. The composition (its water and fuel fractions) drives
/// interactions through the resolved floor (ignition through the combustion law); clotting is a
/// reserved rate; corrosion waits on wave-2 chemistry.
#[derive(Clone, Debug)]
pub struct FluidDef {
    pub id: FluidKindId,
    pub name: String,
    /// The fraction of the pool that may be lost before the body fails on this fluid (Part 35's
    /// critical fraction). RESERVED. Basis: the survivable fractional loss of the fluid's function
    /// (for a mammal, roughly a third of blood volume).
    pub critical_fraction: Fixed,
    /// How fast a breach seals per tick, netted against the bleed rate. RESERVED. Basis: the
    /// coagulation timescale of the fluid, per race.
    pub clot_rate: Fixed,
    /// The fuel value of the fluid's composition (`therm.fuel_value`), for the ignition interaction.
    /// Zero for a non-combustible fluid.
    pub fuel_value: Fixed,
}

/// The set of fluid kinds a world runs, data-defined and extensible.
#[derive(Clone, Debug, Default)]
pub struct FluidRegistry {
    pub fluids: Vec<FluidDef>,
}

/// The dev-fixture fluid: blood (a critical fraction and a clot rate, reserved-with-basis).
pub const BLOOD: FluidKindId = FluidKindId(0);

impl FluidRegistry {
    /// A labelled DEVELOPMENT FIXTURE with one fluid, blood. A world supplies ichor, sap, or none.
    pub fn dev_default() -> FluidRegistry {
        FluidRegistry {
            fluids: vec![FluidDef {
                id: BLOOD,
                name: "blood".to_string(),
                critical_fraction: Fixed::from_ratio(1, 3),
                clot_rate: Fixed::from_ratio(1, 200),
                fuel_value: Fixed::ZERO,
            }],
        }
    }

    pub fn fluid(&self, id: FluidKindId) -> Option<&FluidDef> {
        self.fluids.iter().find(|f| f.id == id)
    }
}

/// A body's pool of one fluid: its kind, its conserved volume as a Part 15 [`Stock`], and the
/// accumulated bleed rate from breaches. A pool drained past its critical fraction is fatal.
#[derive(Clone, Debug)]
pub struct FluidPool {
    pub kind: FluidKindId,
    pub volume: Stock,
    /// The current bleed rate (volume per tick) from unsealed breaches; clotting nets it down.
    pub bleed_rate: Fixed,
}

impl FluidPool {
    /// Whether the pool has fallen past its critical fraction (fatal on this fluid).
    pub fn past_critical(&self, def: &FluidDef) -> bool {
        self.volume.occupancy() <= def.critical_fraction
    }
}

// ============================================================================================
// The body: parts of layered tissue, and fluid pools.
// ============================================================================================

/// One layer of a part, from outside in: a tissue material and its thickness (in the floor's length
/// units). An insult eats through the layers from the surface.
#[derive(Clone, Copy, Debug)]
pub struct TissueLayer {
    pub material: TissueMaterialId,
    pub thickness: Fixed,
}

/// A part's accumulated condition: its structural integrity in `[0, ONE]` (one intact, zero
/// destroyed) and whether it has been severed. Derived from the wounds applied to it; there is no
/// separate health value.
#[derive(Clone, Copy, Debug)]
pub struct PartCondition {
    pub integrity: Fixed,
    pub severed: bool,
}

impl Default for PartCondition {
    fn default() -> Self {
        PartCondition {
            integrity: Fixed::ONE,
            severed: false,
        }
    }
}

/// One body part: its kind (from the data anatomy), its layered tissues, its mass, whether its loss
/// is lethal, the functions it provides, its condition, and the fluid vessel it carries (which bleeds
/// when a wound breaches it).
#[derive(Clone, Debug)]
pub struct BodyPart {
    /// The anatomy part-kind id (a limb, an organ, a carapace segment; Part 25.14, Part 40).
    pub kind: u16,
    pub name: String,
    /// The tissue layers, outer first.
    pub tissues: Vec<TissueLayer>,
    /// The part's mass (a share of the body mass), for the strike energetics.
    pub mass: Fixed,
    /// Destruction is lethal (the torso, the head).
    pub vital: bool,
    /// The functions this part provides (lost with it).
    pub functions: Vec<FunctionId>,
    pub condition: PartCondition,
    /// The fluid vessel this part carries, breached by a deep wound.
    pub carries_fluid: Option<FluidKindId>,
}

impl BodyPart {
    /// The total tissue depth of the part (the sum of its layer thicknesses), the depth a cut must
    /// reach to sever it.
    pub fn depth(&self) -> Fixed {
        let mut total = Fixed::ZERO;
        for layer in &self.tissues {
            total = total.saturating_add(layer.thickness);
        }
        total
    }

    /// The outermost intact tissue material (what an insult meets first), or `None` for a part with no
    /// tissue.
    pub fn surface(&self) -> Option<TissueMaterialId> {
        self.tissues.first().map(|l| l.material)
    }

    /// Whether the part is destroyed (severed or at zero integrity).
    pub fn destroyed(&self) -> bool {
        self.condition.severed || self.condition.integrity <= Fixed::ZERO
    }
}

/// A being's physical body: its parts and its fluid pools. Promoted from the aggregate anatomy vector
/// on demand (R-BUILD-PHYS).
#[derive(Clone, Debug)]
pub struct Body {
    pub parts: Vec<BodyPart>,
    pub fluids: Vec<FluidPool>,
}

/// The reserved parameters of body promotion and the wound mapping. The mechanism is fixed; these
/// numbers are the owner's to set. The development fixture lets the model run and be tested.
#[derive(Clone, Copy, Debug)]
pub struct BodyParams {
    /// Contact pressure cap (MPa) passed to the floor laws. RESERVED. Basis: the floor's pressure
    /// axis maximum (`mat`/`mech` scale).
    pub pressure_max: Fixed,
    /// Delivered-energy cap on the floor scale. RESERVED. Basis: the floor's energy axis maximum.
    pub energy_max: Fixed,
    /// Stress cap (MPa). RESERVED. Basis: the floor's stress axis maximum.
    pub stress_max: Fixed,
    /// The integrity a clean fracture costs a part, in `[0, ONE]`. RESERVED. Basis: the loss of
    /// structural function when a bone breaks (near-total for a load-bearing part).
    pub fracture_damage: Fixed,
    /// The temperature rise (floor units) that a full-severity burn corresponds to. RESERVED. Basis:
    /// the tissue's thermal-damage threshold (protein denaturation).
    pub burn_scale: Fixed,
    /// The blood-loss volume a full-depth breach opens per tick, as a fraction of the vessel volume.
    /// RESERVED. Basis: the haemorrhage rate of a severed major vessel.
    pub breach_bleed: Fixed,
    /// The fraction of body mass the torso carries; the remainder splits over head and limbs.
    /// RESERVED. Basis: real body-segment mass fractions.
    pub torso_mass_fraction: Fixed,
    /// The base tissue thickness of the torso at unit body mass. RESERVED. Basis: real tissue depths
    /// scaled allometrically.
    pub base_thickness: Fixed,
}

impl BodyParams {
    /// A labelled DEVELOPMENT FIXTURE, not owner values.
    pub fn dev_default() -> BodyParams {
        BodyParams {
            pressure_max: Fixed::from_int(1000),
            energy_max: Fixed::from_int(1000),
            stress_max: Fixed::from_int(1000),
            fracture_damage: Fixed::from_ratio(9, 10),
            burn_scale: Fixed::from_int(200),
            breach_bleed: Fixed::from_ratio(1, 20),
            torso_mass_fraction: Fixed::from_ratio(1, 2),
            base_thickness: Fixed::from_ratio(1, 20),
        }
    }
}

impl Body {
    /// Promote a full per-part body from the aggregate anatomy vector (R-BUILD-PHYS): a vital torso
    /// carrying the fluid vessel, a vital head bearing the senses, a limb per locomotion mode, and a
    /// part per natural weapon, each with layered tissues (an outer covering, flesh, and a bone core)
    /// and a mass share of the body. Deterministic, a pure function of the plan (Principle 1: the
    /// aggregate anatomy expands to this only when the individual is promoted). The tissue thicknesses
    /// and mass shares are reserved-with-basis; the covering's development thickens the outer layer.
    pub fn from_body_plan(plan: &BodyPlan, fluid: FluidKindId, params: &BodyParams) -> Body {
        let mass = plan.body_mass.clamp(Fixed::ZERO, Fixed::ONE);
        // Tissue thickness scales with body mass; the covering thickens the outer hide layer.
        let base = params
            .base_thickness
            .mul(mass.saturating_add(Fixed::from_ratio(1, 4)));
        let hide_t = base.mul(
            Fixed::from_ratio(1, 4)
                .saturating_add(plan.covering.development.mul(Fixed::from_ratio(1, 2))),
        );
        let flesh_t = base;
        let bone_t = base.mul(Fixed::from_ratio(1, 2));

        let torso_layers = vec![
            TissueLayer {
                material: HIDE,
                thickness: hide_t,
            },
            TissueLayer {
                material: FLESH,
                thickness: flesh_t,
            },
            TissueLayer {
                material: ORGAN,
                thickness: bone_t,
            }, // vital organs at the core
        ];
        let limb_layers = vec![
            TissueLayer {
                material: HIDE,
                thickness: hide_t,
            },
            TissueLayer {
                material: FLESH,
                thickness: flesh_t.mul(Fixed::from_ratio(1, 2)),
            },
            TissueLayer {
                material: BONE,
                thickness: bone_t,
            },
        ];
        let head_layers = vec![
            TissueLayer {
                material: HIDE,
                thickness: hide_t,
            },
            TissueLayer {
                material: BONE,
                thickness: bone_t,
            }, // the skull
            TissueLayer {
                material: ORGAN,
                thickness: flesh_t.mul(Fixed::from_ratio(1, 2)),
            },
        ];

        let torso_mass = mass.mul(params.torso_mass_fraction);
        let rest = mass - torso_mass;
        let limb_count = plan.locomotion.iter().filter(|&&m| m != 0).count().max(1);
        let weapon_count = plan.weapons.len();
        let divisor = Fixed::from_int((1 + limb_count + weapon_count) as i32);
        let each = if divisor > Fixed::ZERO {
            rest.div(divisor)
        } else {
            Fixed::ZERO
        };

        let mut parts = Vec::new();
        parts.push(BodyPart {
            kind: 0,
            name: "torso".to_string(),
            tissues: torso_layers,
            mass: torso_mass,
            vital: true,
            functions: vec![F_VITAL_CORE],
            condition: PartCondition::default(),
            carries_fluid: Some(fluid),
        });
        parts.push(BodyPart {
            kind: 1,
            name: "head".to_string(),
            tissues: head_layers,
            mass: each,
            vital: true,
            functions: plan.senses.iter().map(|_| F_SIGHT).take(1).collect(),
            condition: PartCondition::default(),
            carries_fluid: Some(fluid),
        });
        for (i, &m) in plan.locomotion.iter().enumerate() {
            if m == 0 {
                continue; // the rooted mark is not a limb
            }
            parts.push(BodyPart {
                kind: 10 + i as u16,
                name: format!("limb{i}"),
                tissues: limb_layers.clone(),
                mass: each,
                vital: false,
                functions: vec![F_LOCOMOTION],
                condition: PartCondition::default(),
                carries_fluid: None,
            });
        }
        for (i, w) in plan.weapons.iter().enumerate() {
            parts.push(BodyPart {
                kind: 20 + i as u16,
                name: format!("weapon{i}"),
                tissues: vec![TissueLayer {
                    material: BONE,
                    thickness: bone_t.mul(w.development.clamp(Fixed::ZERO, Fixed::ONE)),
                }],
                mass: each.mul(Fixed::from_ratio(1, 4)),
                vital: false,
                functions: vec![F_STRIKE],
                condition: PartCondition::default(),
                carries_fluid: None,
            });
        }

        let vessel = Stock::new(Fixed::ONE, Fixed::ONE, Fixed::ZERO);
        Body {
            parts,
            fluids: vec![FluidPool {
                kind: fluid,
                volume: vessel,
                bleed_rate: Fixed::ZERO,
            }],
        }
    }

    /// Whether the body is alive: no vital part destroyed and no fluid pool past its critical
    /// fraction (Part 35's liveness rule, derived not stored).
    pub fn is_alive(&self, fluids: &FluidRegistry) -> bool {
        if self.parts.iter().any(|p| p.vital && p.destroyed()) {
            return false;
        }
        for pool in &self.fluids {
            if let Some(def) = fluids.fluid(pool.kind) {
                if pool.past_critical(def) {
                    return false;
                }
            }
        }
        true
    }

    /// The derived aggregate condition in `[0, ONE]` (Part 35: never a stored competing value). It is
    /// the least of every vital part's integrity and every fluid pool's headroom above its critical
    /// fraction, so it reaches zero exactly when the body dies, and it is the reading the decision
    /// layer and the homeostatic integrity axis consume.
    pub fn integrity(&self, fluids: &FluidRegistry) -> Fixed {
        let mut worst = Fixed::ONE;
        for p in &self.parts {
            if p.vital {
                let v = if p.destroyed() {
                    Fixed::ZERO
                } else {
                    p.condition.integrity
                };
                if v < worst {
                    worst = v;
                }
            }
        }
        for pool in &self.fluids {
            if let Some(def) = fluids.fluid(pool.kind) {
                // Headroom: 1 at full, 0 at the critical fraction, clamped.
                let span = Fixed::ONE - def.critical_fraction;
                let head = if span > Fixed::ZERO {
                    (pool.volume.occupancy() - def.critical_fraction).div(span)
                } else {
                    pool.volume.occupancy()
                };
                let head = head.clamp(Fixed::ZERO, Fixed::ONE);
                if head < worst {
                    worst = head;
                }
            }
        }
        worst
    }

    /// Advance bleeding one tick: each pool with an open breach loses its bleed rate (netted against
    /// its fluid's clot rate) from the volume stock, and the bleed rate decays toward sealed. A pure
    /// deterministic drain; death from a pool past critical is read by [`Body::is_alive`].
    pub fn bleed(&mut self, fluids: &FluidRegistry) {
        for pool in &mut self.fluids {
            let clot = fluids
                .fluid(pool.kind)
                .map(|f| f.clot_rate)
                .unwrap_or(Fixed::ZERO);
            if pool.bleed_rate > Fixed::ZERO {
                let net = pool.bleed_rate;
                pool.volume.step(net); // draw the bled volume
                                       // The wound seals a little each tick.
                pool.bleed_rate = sub_floor(pool.bleed_rate, clot);
            }
        }
    }

    /// The physics-derived reach of the body: the summed limb-segment lengths through the resolved
    /// `reach` law (R-BUILD-PHYS: a mechanical stat measured from the body, not an authored one). The
    /// limb tissue depth stands in for the segment length here.
    pub fn reach(&self) -> Fixed {
        let segments: Vec<Fixed> = self
            .parts
            .iter()
            .filter(|p| p.functions.contains(&F_LOCOMOTION) && !p.destroyed())
            .map(|p| p.depth())
            .collect();
        laws::reach(&segments)
    }

    /// The physics-derived effective strength: the muscle-bearing mass (the mass of the intact parts
    /// carrying a flesh layer) scaled by the flesh material's strength (R-BUILD-PHYS: a mechanical
    /// stat read from the body and its materials, not an authored number, so a larger body of a
    /// stronger tissue is stronger and losing a limb weakens it). The raw morphology is the primitive.
    pub fn strength(&self, tissues: &TissueRegistry) -> Fixed {
        let flesh_strength = tissues
            .material(FLESH)
            .map(|m| m.fracture_strength)
            .unwrap_or(Fixed::ZERO);
        let mut muscle = Fixed::ZERO;
        for p in &self.parts {
            if p.destroyed() {
                continue;
            }
            if p.tissues.iter().any(|l| l.material == FLESH) {
                muscle = muscle.saturating_add(p.mass);
            }
        }
        muscle.checked_mul(flesh_strength).unwrap_or(Fixed::ZERO)
    }
}

/// The record of one measured wound: the mode that caused it, the measured severity in `[0, ONE]`,
/// and the structural consequences read from the physics. Not a stored damage kind; the canonical
/// state is the part's condition and the fluids, this is the outcome the caller reads and history can
/// narrate.
#[derive(Clone, Copy, Debug)]
pub struct WoundRecord {
    pub mode: DamageModeId,
    /// The measured severity, the integrity the insult removed from the part, in `[0, ONE]`.
    pub severity: Fixed,
    pub severed: bool,
    pub fractured: bool,
    pub vessel_breached: bool,
}

/// A physical insult: a mode, a force over a contact area, a delivered energy, and a temperature rise
/// (for a thermal mode). The contact area is the geometry that makes a cut, a pierce, or a crush of
/// the same force and energy.
#[derive(Clone, Copy, Debug)]
pub struct Insult {
    pub mode: DamageModeId,
    pub force: Fixed,
    pub contact_area: Fixed,
    pub delivered_energy: Fixed,
    pub delta_t: Fixed,
}

/// Apply an insult to a body part and return the measured wound (R-WOUND). The mode's measurement
/// family (data) selects which resolved floor law computes the damage against the part's surface
/// tissue: penetration (contact pressure then cut depth), fracture (the dual criterion on the
/// delivered energy), or thermal (constrained thermal stress). The measured damage reduces the part's
/// integrity, a full-depth cut severs it, a deep wound breaches its fluid vessel, and an exotic mode
/// with no built law does nothing (a reserved extension, never fabricated). Deterministic and
/// float-free; a pure function of the insult and the tissue.
pub fn apply_insult(
    body: &mut Body,
    part_index: usize,
    insult: &Insult,
    modes: &DamageModeRegistry,
    tissues: &TissueRegistry,
    params: &BodyParams,
) -> WoundRecord {
    let mut rec = WoundRecord {
        mode: insult.mode,
        severity: Fixed::ZERO,
        severed: false,
        fractured: false,
        vessel_breached: false,
    };
    let Some(part) = body.parts.get_mut(part_index) else {
        return rec;
    };
    if part.destroyed() {
        return rec;
    }
    let Some(surface) = part.surface() else {
        return rec;
    };
    let Some(mat) = tissues.material(surface) else {
        return rec;
    };
    let part_depth = part.depth();

    match modes.measure(insult.mode) {
        MeasureKind::Penetration => {
            let pressure =
                laws::contact_pressure(insult.force, insult.contact_area, params.pressure_max);
            // How deep the pressure can reach: walk the layers from the surface, summing the
            // thickness of each layer the pressure can bite, and stop at the first layer whose
            // hardness it does not exceed (a bone core stops a weak cut, the wound propagating
            // through the tissues rather than a single surface read).
            let mut reachable = Fixed::ZERO;
            for layer in &part.tissues {
                match tissues.material(layer.material) {
                    Some(lm) if pressure > lm.hardness => {
                        reachable = reachable.saturating_add(layer.thickness);
                    }
                    _ => break,
                }
            }
            if reachable <= Fixed::ZERO {
                return rec; // turned aside: the surface hardness exceeds the delivered pressure
            }
            let depth = laws::cut_penetrate(
                pressure,
                mat.hardness,
                insult.delivered_energy,
                mat.specific_cut_energy,
                insult.contact_area,
                reachable,
            )
            .min(reachable);
            let frac = if part_depth > Fixed::ZERO {
                depth.div(part_depth).clamp(Fixed::ZERO, Fixed::ONE)
            } else {
                Fixed::ZERO
            };
            rec.severity = frac;
            // A cut clean through the part severs it; a deep one breaches a fluid vessel.
            if depth >= part_depth {
                rec.severed = true;
                part.condition.severed = true;
                part.condition.integrity = Fixed::ZERO;
            } else {
                part.condition.integrity = sub_floor(part.condition.integrity, frac);
            }
            if frac >= Fixed::from_ratio(1, 2) && part.carries_fluid.is_some() {
                rec.vessel_breached = true;
            }
        }
        MeasureKind::Fracture => {
            // Blunt force breaks the strongest structural layer (the bone), transmitted through the
            // softer tissue over it, so the criterion reads the toughest layer's properties.
            let mut fs = mat.fracture_strength;
            let mut fe = mat.fracture_energy;
            for layer in &part.tissues {
                if let Some(lm) = tissues.material(layer.material) {
                    if lm.fracture_strength > fs {
                        fs = lm.fracture_strength;
                        fe = lm.fracture_energy;
                    }
                }
            }
            let stress =
                laws::contact_pressure(insult.force, insult.contact_area, params.pressure_max);
            let (stress_margin, energy_margin) = laws::fracture_onset(
                stress,
                fs,
                fe,
                insult.contact_area,
                insult.delivered_energy,
                params.energy_max,
            );
            if stress_margin < Fixed::ZERO || energy_margin < Fixed::ZERO {
                rec.fractured = true;
                rec.severity = params.fracture_damage;
                part.condition.integrity =
                    sub_floor(part.condition.integrity, params.fracture_damage);
                if part.carries_fluid.is_some() {
                    rec.vessel_breached = true;
                }
            }
        }
        MeasureKind::Thermal => {
            let (_, fractured) = laws::thermal_stress(
                mat.elastic_modulus,
                mat.expansion,
                insult.delta_t,
                Fixed::ONE,
                mat.fracture_strength,
                params.stress_max,
            );
            // Burn severity: the temperature rise against the reserved burn scale, worsened if the
            // thermal stress cracks the tissue.
            let mut sev = if params.burn_scale > Fixed::ZERO {
                insult
                    .delta_t
                    .div(params.burn_scale)
                    .clamp(Fixed::ZERO, Fixed::ONE)
            } else {
                Fixed::ZERO
            };
            if fractured {
                sev = Fixed::ONE;
            }
            rec.severity = sev;
            rec.fractured = fractured;
            part.condition.integrity = sub_floor(part.condition.integrity, sev);
            if sev >= Fixed::from_ratio(3, 4) && part.carries_fluid.is_some() {
                rec.vessel_breached = true;
            }
        }
        MeasureKind::Exotic => {
            // No built law: a reserved, audited floor extension. No damage is fabricated.
        }
    }

    // A breached vessel opens a bleed on the matching pool.
    if rec.vessel_breached {
        if let Some(kind) = part.carries_fluid {
            let bleed = params.breach_bleed.mul(rec.severity);
            for pool in &mut body.fluids {
                if pool.kind == kind {
                    pool.bleed_rate = pool.bleed_rate.saturating_add(bleed);
                }
            }
        }
    }
    rec
}

/// Enact a strike: an attacker's natural weapon meets a target's body part, the wound measured
/// through the same floor laws as any other insult (R-WOUND, and the predator-prey closure the
/// evolved-behaviour work needs, R-BEHAVIOR-EVOLVE Part 8.4). The weapon part's mass and the strike
/// velocity give the delivered energy through the resolved `kinetic_energy` law; the applied force,
/// the contact area (a sharp weapon a small area, a blunt one a large one), and the mode are the
/// strike's geometry. A strike from a body with no such weapon part delivers nothing. Deterministic,
/// float-free, no randomness; a pure function of the two bodies and the blow.
#[allow(clippy::too_many_arguments)]
pub fn strike(
    attacker: &Body,
    weapon_index: usize,
    velocity: Fixed,
    applied_force: Fixed,
    contact_area: Fixed,
    mode: DamageModeId,
    target: &mut Body,
    target_part: usize,
    modes: &DamageModeRegistry,
    tissues: &TissueRegistry,
    params: &BodyParams,
) -> WoundRecord {
    let weapon_mass = attacker
        .parts
        .get(weapon_index)
        .filter(|p| p.functions.contains(&F_STRIKE) && !p.destroyed())
        .map(|p| p.mass)
        .unwrap_or(Fixed::ZERO);
    if weapon_mass <= Fixed::ZERO {
        return WoundRecord {
            mode,
            severity: Fixed::ZERO,
            severed: false,
            fractured: false,
            vessel_breached: false,
        };
    }
    let energy = laws::kinetic_energy(weapon_mass, velocity, params.energy_max);
    let insult = Insult {
        mode,
        force: applied_force,
        contact_area,
        delivered_energy: energy,
        delta_t: Fixed::ZERO,
    };
    apply_insult(target, target_part, &insult, modes, tissues, params)
}

/// Subtract, flooring at zero (there is no saturating_sub on Fixed).
fn sub_floor(a: Fixed, b: Fixed) -> Fixed {
    let r = a - b;
    if r < Fixed::ZERO {
        Fixed::ZERO
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anatomy::{Part, Temperament};

    fn plan(mass: (i64, i64), legs: usize, weapons: usize) -> BodyPlan {
        BodyPlan {
            body_mass: Fixed::from_ratio(mass.0, mass.1),
            encephalization: Fixed::from_ratio(1, 2),
            diet_breadth: Fixed::from_ratio(1, 2),
            weapons: (0..weapons)
                .map(|i| Part {
                    kind: i as u16,
                    development: Fixed::from_ratio(3, 4),
                })
                .collect(),
            covering: Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            },
            senses: vec![Part {
                kind: 0,
                development: Fixed::from_ratio(1, 2),
            }],
            locomotion: (0..legs).map(|_| 1u16).collect(),
            organs: vec![],
            temperament: Temperament {
                boldness: Fixed::from_ratio(1, 2),
                exploration: Fixed::from_ratio(1, 2),
                activity: Fixed::from_ratio(1, 2),
                sociability: Fixed::from_ratio(1, 2),
                aggression: Fixed::from_ratio(1, 4),
            },
        }
    }

    fn body() -> Body {
        Body::from_body_plan(&plan((3, 4), 4, 1), BLOOD, &BodyParams::dev_default())
    }

    #[test]
    fn a_promoted_body_has_vital_parts_a_vessel_and_starts_intact() {
        let b = body();
        assert!(b.parts.iter().any(|p| p.vital), "it has a vital part");
        assert!(
            b.parts
                .iter()
                .any(|p| p.name == "torso" && p.carries_fluid.is_some()),
            "the torso carries a vessel"
        );
        assert_eq!(
            b.parts
                .iter()
                .filter(|p| p.name.starts_with("limb"))
                .count(),
            4,
            "four legs"
        );
        assert_eq!(
            b.parts
                .iter()
                .filter(|p| p.name.starts_with("weapon"))
                .count(),
            1,
            "one weapon"
        );
        let fr = FluidRegistry::dev_default();
        assert!(b.is_alive(&fr), "a fresh body is alive");
        assert_eq!(b.integrity(&fr), Fixed::ONE, "and at full integrity");
    }

    #[test]
    fn mechanical_stats_are_derived_from_the_body() {
        // R-BUILD-PHYS: reach and strength are read from the promoted body, not authored. A bigger,
        // more-limbed body reaches and strikes harder; losing a limb weakens it.
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let small = Body::from_body_plan(&plan((1, 8), 2, 0), BLOOD, &params);
        let big = Body::from_body_plan(&plan((1, 1), 6, 0), BLOOD, &params);
        assert!(
            big.reach() > small.reach(),
            "the bigger, more-limbed body reaches farther"
        );
        assert!(
            big.strength(&tissues) > small.strength(&tissues),
            "and is stronger"
        );
        // Amputation weakens the body (derived, not stored).
        let mut b = Body::from_body_plan(&plan((1, 1), 4, 0), BLOOD, &params);
        let before = b.strength(&tissues);
        let limb = b
            .parts
            .iter()
            .position(|p| p.name.starts_with("limb"))
            .unwrap();
        b.parts[limb].condition.severed = true;
        assert!(
            b.strength(&tissues) < before,
            "losing a limb lowers derived strength"
        );
    }

    #[test]
    fn a_bigger_body_has_deeper_tissue() {
        let params = BodyParams::dev_default();
        let small = Body::from_body_plan(&plan((1, 8), 4, 0), BLOOD, &params);
        let big = Body::from_body_plan(&plan((1, 1), 4, 0), BLOOD, &params);
        let sd = small.parts[0].depth();
        let bd = big.parts[0].depth();
        assert!(
            bd > sd,
            "a bigger body has a deeper torso ({bd:?} vs {sd:?})"
        );
    }

    #[test]
    fn a_deep_cut_to_a_limb_severs_it_and_costs_its_function() {
        let mut b = body();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let limb = b
            .parts
            .iter()
            .position(|p| p.name.starts_with("limb"))
            .unwrap();
        // A sharp blade: a tiny edge contact area (a high pressure exceeding even the bone) and ample
        // energy cuts clean through.
        let insult = Insult {
            mode: CUT,
            force: Fixed::from_int(2000),
            contact_area: Fixed::from_ratio(1, 100_000),
            delivered_energy: Fixed::from_int(10),
            delta_t: Fixed::ZERO,
        };
        let rec = apply_insult(&mut b, limb, &insult, &modes, &tissues, &params);
        assert!(rec.severity > Fixed::ZERO, "the cut did measurable damage");
        assert!(rec.severed, "a full-depth cut severs the limb");
        assert!(b.parts[limb].destroyed(), "the limb is gone");
        let fr = FluidRegistry::dev_default();
        assert!(b.is_alive(&fr), "but losing a non-vital limb is not fatal");
    }

    #[test]
    fn a_blunt_blow_fractures_bone_by_the_floor_criterion() {
        let mut b = body();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let limb = b
            .parts
            .iter()
            .position(|p| p.name.starts_with("limb"))
            .unwrap();
        // A heavy blunt blow: a broad contact area, high force and energy.
        let insult = Insult {
            mode: BLUNT,
            force: Fixed::from_int(900),
            contact_area: Fixed::from_ratio(1, 100),
            delivered_energy: Fixed::from_int(1),
            delta_t: Fixed::ZERO,
        };
        let rec = apply_insult(&mut b, limb, &insult, &modes, &tissues, &params);
        assert!(
            rec.fractured,
            "a hard enough blow fractures the part (the floor criterion)"
        );
        assert!(
            b.parts[limb].condition.integrity < Fixed::ONE,
            "and lowers its integrity"
        );
    }

    #[test]
    fn a_light_touch_does_no_measurable_harm() {
        let mut b = body();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let limb = b
            .parts
            .iter()
            .position(|p| p.name.starts_with("limb"))
            .unwrap();
        // A feather touch: negligible force spread over a broad area does not exceed the hardness.
        let insult = Insult {
            mode: CUT,
            force: Fixed::from_ratio(1, 100),
            contact_area: Fixed::from_int(10),
            delivered_energy: Fixed::from_ratio(1, 100),
            delta_t: Fixed::ZERO,
        };
        let rec = apply_insult(&mut b, limb, &insult, &modes, &tissues, &params);
        assert_eq!(
            rec.severity,
            Fixed::ZERO,
            "a touch below the tissue hardness does nothing"
        );
        assert_eq!(
            b.parts[limb].condition.integrity,
            Fixed::ONE,
            "the limb is unharmed"
        );
    }

    #[test]
    fn a_torso_wound_bleeds_and_bleeding_out_kills() {
        let mut b = body();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let fr = FluidRegistry::dev_default();
        // Make the breach open fast so the test is short.
        let params = BodyParams {
            breach_bleed: Fixed::from_ratio(1, 2),
            ..BodyParams::dev_default()
        };
        let torso = b.parts.iter().position(|p| p.name == "torso").unwrap();
        // A deep pierce that breaches the vessel but does not run clean through the torso.
        let insult = Insult {
            mode: PIERCE,
            force: Fixed::from_int(2000),
            contact_area: Fixed::from_ratio(1, 100_000),
            delivered_energy: Fixed::from_int(3),
            delta_t: Fixed::ZERO,
        };
        let rec = apply_insult(&mut b, torso, &insult, &modes, &tissues, &params);
        assert!(
            rec.vessel_breached,
            "a deep torso wound breaches the vessel"
        );
        assert!(!rec.severed, "but does not run clean through");
        assert!(b.is_alive(&fr), "not dead yet");
        // Bleed out over time.
        let mut died = false;
        for _ in 0..200 {
            b.bleed(&fr);
            if !b.is_alive(&fr) {
                died = true;
                break;
            }
        }
        assert!(died, "unstaunched, the body bleeds out and dies");
    }

    #[test]
    fn destroying_a_vital_part_is_fatal() {
        let mut b = body();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let fr = FluidRegistry::dev_default();
        let params = BodyParams::dev_default();
        let head = b.parts.iter().position(|p| p.name == "head").unwrap();
        // A massive pierce clean through the head (a pressure past the skull, ample energy).
        let insult = Insult {
            mode: PIERCE,
            force: Fixed::from_int(3000),
            contact_area: Fixed::from_ratio(1, 100_000),
            delivered_energy: Fixed::from_int(20),
            delta_t: Fixed::ZERO,
        };
        apply_insult(&mut b, head, &insult, &modes, &tissues, &params);
        assert!(b.parts[head].destroyed(), "the head is destroyed");
        assert!(!b.is_alive(&fr), "destroying a vital part kills the body");
        assert_eq!(b.integrity(&fr), Fixed::ZERO, "integrity reads zero");
    }

    #[test]
    fn an_exotic_mode_with_no_built_law_does_nothing_rather_than_fabricating() {
        let mut b = body();
        let mut modes = DamageModeRegistry::dev_default();
        // A world adds a curse mode with no built measurement.
        let curse = DamageModeId(99);
        modes.modes.push(DamageModeDef {
            id: curse,
            name: "curse".to_string(),
            measure: MeasureKind::Exotic,
        });
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let torso = b.parts.iter().position(|p| p.name == "torso").unwrap();
        let insult = Insult {
            mode: curse,
            force: Fixed::from_int(999),
            contact_area: Fixed::from_ratio(1, 1000),
            delivered_energy: Fixed::from_int(999),
            delta_t: Fixed::ZERO,
        };
        let rec = apply_insult(&mut b, torso, &insult, &modes, &tissues, &params);
        assert_eq!(
            rec.severity,
            Fixed::ZERO,
            "an unbuilt exotic mode fabricates no damage"
        );
        assert_eq!(
            b.parts[torso].condition.integrity,
            Fixed::ONE,
            "the part is unharmed until the law is built"
        );
    }

    #[test]
    fn the_wound_measurement_replays_bit_identically() {
        let run = || {
            let mut b = body();
            let modes = DamageModeRegistry::dev_default();
            let tissues = TissueRegistry::dev_default();
            let params = BodyParams::dev_default();
            let torso = b.parts.iter().position(|p| p.name == "torso").unwrap();
            let insult = Insult {
                mode: CUT,
                force: Fixed::from_int(300),
                contact_area: Fixed::from_ratio(1, 100_000),
                delivered_energy: Fixed::from_int(2),
                delta_t: Fixed::ZERO,
            };
            let rec = apply_insult(&mut b, torso, &insult, &modes, &tissues, &params);
            (
                rec.severity.to_bits(),
                b.parts[torso].condition.integrity.to_bits(),
            )
        };
        assert_eq!(
            run(),
            run(),
            "the same insult on the same tissue measures identically"
        );
    }

    #[test]
    fn a_predator_strike_wounds_prey_through_its_natural_weapon() {
        // The predator-prey closure: an attacker with a natural weapon strikes a prey's body, the
        // wound measured through the floor. A predator with a sharp, developed weapon and a fast blow
        // draws blood; a body with no weapon delivers nothing.
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let predator = Body::from_body_plan(&plan((1, 1), 4, 1), BLOOD, &params);
        let weapon = predator
            .parts
            .iter()
            .position(|p| p.name.starts_with("weapon"))
            .unwrap();
        let mut prey = Body::from_body_plan(&plan((1, 2), 4, 0), BLOOD, &params);
        let torso = prey.parts.iter().position(|p| p.name == "torso").unwrap();
        let rec = strike(
            &predator,
            weapon,
            Fixed::from_int(30),           // a fast blow (m/s)
            Fixed::from_int(2000),         // the force behind it (N)
            Fixed::from_ratio(1, 100_000), // a sharp point
            PIERCE,
            &mut prey,
            torso,
            &modes,
            &tissues,
            &params,
        );
        assert!(
            rec.severity > Fixed::ZERO,
            "the strike drew a measurable wound"
        );
        assert!(
            prey.parts[torso].condition.integrity < Fixed::ONE,
            "the prey is hurt"
        );

        // A weaponless body strikes nothing.
        let unarmed = Body::from_body_plan(&plan((1, 1), 4, 0), BLOOD, &params);
        let rec2 = strike(
            &unarmed,
            0,
            Fixed::from_int(30),
            Fixed::from_int(2000),
            Fixed::from_ratio(1, 100_000),
            PIERCE,
            &mut prey,
            torso,
            &modes,
            &tissues,
            &params,
        );
        assert_eq!(
            rec2.severity,
            Fixed::ZERO,
            "a body with no weapon part strikes for nothing"
        );
    }

    #[test]
    fn integrity_is_derived_and_falls_with_damage() {
        let mut b = body();
        let fr = FluidRegistry::dev_default();
        let modes = DamageModeRegistry::dev_default();
        let tissues = TissueRegistry::dev_default();
        let params = BodyParams::dev_default();
        let before = b.integrity(&fr);
        let torso = b.parts.iter().position(|p| p.name == "torso").unwrap();
        let insult = Insult {
            mode: CUT,
            force: Fixed::from_int(200),
            contact_area: Fixed::from_ratio(1, 100_000),
            delivered_energy: Fixed::from_int(1),
            delta_t: Fixed::ZERO,
        };
        apply_insult(&mut b, torso, &insult, &modes, &tissues, &params);
        assert!(
            b.integrity(&fr) < before,
            "a wound to a vital part lowers derived integrity"
        );
    }
}
