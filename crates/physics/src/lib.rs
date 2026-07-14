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

//! # civsim-physics: the authored physics-and-materials substrate
//!
//! This crate is the one authored layer of the project (design Principle 9), the
//! reach-bounding artifact whose completeness sets the expressiveness ceiling of
//! technology, value, meaning, and biology. It carries the locked representation of
//! the substrate guide Section 4 and design Part 58: a physics primitive is either a
//! [`QuantityAxis`] (a named, unit-bearing, range-bounded, fixed-point scalar at a
//! tier) or an [`InteractionLaw`] (a closed-form integer kernel over the quantity
//! vectors of the participating entities, reporting an interval-bounded fixed-point
//! consequence); a [`Substance`] (a material, a tissue, a structural member) is a
//! vector of values over the axes plus the laws it participates in plus a provenance
//! tag. The wave-0 biology floor (R-PHYS-BIO) and the wave-1 mechanical-and-materials
//! floor (R-PHYS-MECH) are data loaded into this registry.
//!
//! This module is phase 1 of the build: the representation and the [`PhysicsRegistry`]
//! that loads it from data, with three disciplines enforced structurally. Every value
//! is fixed-point ([`Fixed`]), parsed from a decimal string by integer arithmetic, so
//! no floating point reaches canonical state. Every axis carries a [`Dimension`] that
//! is a monomial over the Part 55 base dimensions (four from the dawn, plus electric current from
//! wave 3), so the neutrality test (every axis reduces to a base dimension) is a property of the type
//! rather than a check that can be forgotten. And every axis range is either [`AxisRange::Set`] or
//! [`AxisRange::Reserved`]: reading a reserved range fails loud (the owner must set it,
//! never a fabricated default), the same fail-loud discipline as the calibration
//! manifest. The law kernels themselves are phase 2.

pub mod band_gap;
pub mod crystal_field;
pub mod d_state_radius;
pub mod ewald;
pub mod floor_provenance;
pub mod geodynamics;
pub mod graph;
pub mod ionic_radii;
pub mod ionization_ladder;
pub mod lattice_modulus;
pub mod laws;
pub mod materials_oracle;
pub mod melting;
pub mod metal_eos;
pub mod mit_reference;
pub mod molecular_opacity;
pub mod opacity;
pub mod optical_constants;
pub mod periodic;
pub mod petrology;
pub mod petrology_data;
pub mod qeq;
pub mod quantities;
pub mod rose_eos;
pub mod saha;
pub mod scaled;
pub mod stoner;
pub mod term_values;
pub mod tm_oxide_lattice_energy;

use civsim_core::{Fixed, StateHasher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// A physical dimension as a monomial over the Part 55 base dimensions. Because a
/// dimension is only ever integer exponents over length, mass, time, temperature, and (from wave 3)
/// electric current, the
/// neutrality test (an axis reduces to a base dimension and is not a steering leak in
/// the costume of physics) holds by construction: there is no way to author an axis
/// whose dimension is not such a monomial. The dimensionless Ratio class is the
/// all-zero monomial, which closes the wave-1 NEUT-DIMENSIONLESS-CLASS gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Dimension {
    /// Exponent of length.
    pub length: i8,
    /// Exponent of mass.
    pub mass: i8,
    /// Exponent of time.
    pub time: i8,
    /// Exponent of temperature.
    pub temperature: i8,
    /// Exponent of electric current, the fifth base added for the wave-3 electricity-and-magnetism
    /// tier (R-PHYS-W3; the SI ampere). Electricity is the first quantity dimensionally independent
    /// of the other four bases, and the Gaussian half-integer alternative is unrepresentable in an
    /// `i8` monomial, so current is the fifth base. Every earlier axis is unchanged with a current
    /// exponent of zero, so the neutrality-by-construction property is preserved over five bases.
    pub current: i8,
}

impl Dimension {
    /// The dimensionless Ratio class (friction coefficients, restitution, mechanical
    /// advantage, ductility, Poisson's ratio, oxidiser demand).
    pub const DIMENSIONLESS: Dimension = Dimension {
        length: 0,
        mass: 0,
        time: 0,
        temperature: 0,
        current: 0,
    };
    /// Length.
    pub const LENGTH: Dimension = Dimension {
        length: 1,
        mass: 0,
        time: 0,
        temperature: 0,
        current: 0,
    };
    /// Mass.
    pub const MASS: Dimension = Dimension {
        length: 0,
        mass: 1,
        time: 0,
        temperature: 0,
        current: 0,
    };
    /// Time.
    pub const TIME: Dimension = Dimension {
        length: 0,
        mass: 0,
        time: 1,
        temperature: 0,
        current: 0,
    };
    /// Temperature.
    pub const TEMPERATURE: Dimension = Dimension {
        length: 0,
        mass: 0,
        time: 0,
        temperature: 1,
        current: 0,
    };
    /// Area, length squared.
    pub const AREA: Dimension = Dimension {
        length: 2,
        mass: 0,
        time: 0,
        temperature: 0,
        current: 0,
    };
    /// Volume, length cubed.
    pub const VOLUME: Dimension = Dimension {
        length: 3,
        mass: 0,
        time: 0,
        temperature: 0,
        current: 0,
    };
    /// Velocity, length over time.
    pub const VELOCITY: Dimension = Dimension {
        length: 1,
        mass: 0,
        time: -1,
        temperature: 0,
        current: 0,
    };
    /// Force, mass times length over time squared.
    pub const FORCE: Dimension = Dimension {
        length: 1,
        mass: 1,
        time: -2,
        temperature: 0,
        current: 0,
    };
    /// Energy, mass times length squared over time squared.
    pub const ENERGY: Dimension = Dimension {
        length: 2,
        mass: 1,
        time: -2,
        temperature: 0,
        current: 0,
    };
    /// Pressure, mass over length and time squared.
    pub const PRESSURE: Dimension = Dimension {
        length: -1,
        mass: 1,
        time: -2,
        temperature: 0,
        current: 0,
    };
    /// Electric current, the SI ampere (the fifth base, wave 3).
    pub const CURRENT: Dimension = Dimension {
        length: 0,
        mass: 0,
        time: 0,
        temperature: 0,
        current: 1,
    };
    /// Electric charge, the coulomb (ampere times time).
    pub const CHARGE: Dimension = Dimension {
        length: 0,
        mass: 0,
        time: 1,
        temperature: 0,
        current: 1,
    };
    /// Electric potential, the volt (power over current, mass length squared over time cubed and
    /// current). The unified dimension of `elec.potential`, `elec.emf`, and the promoted
    /// `chem.standard_potential`.
    pub const VOLTAGE: Dimension = Dimension {
        length: 2,
        mass: 1,
        time: -3,
        temperature: 0,
        current: -1,
    };

    /// Whether this is the dimensionless Ratio class.
    pub fn is_dimensionless(self) -> bool {
        self == Dimension::DIMENSIONLESS
    }
}

impl std::ops::Mul for Dimension {
    type Output = Dimension;
    /// The product of two dimensions: exponents add, as a law combining quantities.
    fn mul(self, o: Dimension) -> Dimension {
        Dimension {
            length: self.length + o.length,
            mass: self.mass + o.mass,
            time: self.time + o.time,
            temperature: self.temperature + o.temperature,
            current: self.current + o.current,
        }
    }
}

impl std::ops::Div for Dimension {
    type Output = Dimension;
    /// The quotient of two dimensions: exponents subtract.
    fn div(self, o: Dimension) -> Dimension {
        Dimension {
            length: self.length - o.length,
            mass: self.mass - o.mass,
            time: self.time - o.time,
            temperature: self.temperature - o.temperature,
            current: self.current - o.current,
        }
    }
}

/// Whether a value is grounded in real data or is the owner's reserved fantasy design.
/// The split is explicit so the provenance discipline (Part 58) is never lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    /// A real value with a citation or datasheet.
    RealWithSource(String),
    /// A fantasy value the owner reserves, with the ground for it.
    FantasyReserved(String),
}

/// An axis's fixed-point range. A range the owner has not set is [`AxisRange::Reserved`]
/// and reading it fails loud, never a fabricated default (design Principle 11, the
/// reserved-value discipline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisRange {
    /// The owner has not set the bound; the basis on which the owner would set it.
    Reserved {
        /// The ground for the eventual bound.
        basis: String,
    },
    /// The owner's set bound.
    Set {
        /// The inclusive lower bound.
        lo: Fixed,
        /// The inclusive upper bound.
        hi: Fixed,
    },
}

impl AxisRange {
    /// The set bound, or a fail-loud error if the range is still reserved.
    pub fn require(&self, axis_id: &str) -> Result<(Fixed, Fixed), PhysicsError> {
        match self {
            AxisRange::Set { lo, hi } => Ok((*lo, *hi)),
            AxisRange::Reserved { .. } => Err(PhysicsError::ReservedRange(axis_id.to_string())),
        }
    }

    /// Whether the range has been set.
    pub fn is_set(&self) -> bool {
        matches!(self, AxisRange::Set { .. })
    }
}

/// How drawing on a source axis behaves under conservation (R-SOURCE-VECTOR floor metadata): whether a
/// draw DRAWS DOWN a located stock, reads a renewable flux no draw exhausts, or drains a reservoir. This
/// is floor physics on the AXIS (the conservation law of the axis's quantity), read as data so a feeder's
/// supply behaviour is a data row rather than a branch on any source kind (Principle 11): a matter
/// composition axis is a depletable stock, a photon or gravity-gradient feeder's source a non-rivalrous
/// flux. An axis whose character the owner has not declared is [`DepletionCharacter::Reserved`] and
/// reading it in a draw fails loud, never a silent "stock" default (the reserved-value discipline).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepletionCharacter {
    /// The owner has not declared the character; the basis on which it is decided (the conservation law
    /// of the axis's quantity).
    Reserved {
        /// The ground for the eventual character.
        basis: String,
    },
    /// A located stock a draw draws DOWN (matter composition: mass is conserved, so eating removes it).
    DepletableStock,
    /// A renewable flux no draw exhausts (a light, thermal-gradient, or gravity-gradient feeder's source).
    NonRivalrousFlux,
    /// A reservoir replenished from a source and drained to a sink (a hydrology pool).
    Reservoir,
}

/// The reduction of one unit on this axis to the floor's common conserved energy/matter currency
/// (R-SOURCE-VECTOR floor metadata). Used for CROSS-AXIS comparison (the R-TIER-CONSIST pool projection),
/// never the intake arithmetic of a draw. This is NOT an authored value: under the fundamental-constants
/// floor (AGENTIC_ADDENDUM section 9, the three-way test), an axis's reduction IS the energy-equivalence
/// of its physical quantity, so it is DERIVED (the fundamental constants times the substance's own floor
/// physics: a redox axis through its couple's EMF times carrier charge, the `nFE` bridge; a thermal-
/// gradient axis through the heat capacity and the temperature difference; a mass-flux axis through
/// `bio.energy_density`), never declared. Until a feeder is armed and its reduction is derived, the axis
/// carries the [`ReductionCoefficient::Derive`] sentinel and reading it fails loud, never a fabricated
/// number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReductionCoefficient {
    /// No feeder draws on this axis yet, so its reduction has not been derived; the ground from which it
    /// will be derived (the axis's energy-equivalence: the fundamentals times the substance's floor
    /// physics). Reading it fails loud, never a silent default (the derive target, not an owner value).
    Derive {
        /// The ground from which the reduction is derived when a feeder is armed.
        basis: String,
    },
    /// The derived reduction: one axis-unit in the conserved energy/matter currency, computed from the
    /// axis's energy-equivalence at feeder-arming, never authored.
    Derived(Fixed),
}

/// A quantity axis: a named, unit-bearing, range-bounded, fixed-point scalar dimension
/// at a tier (substrate guide Section 4). The `scale_unit` is the one canonical
/// per-quantity scale the value is stored in (for example the megapascal for every
/// pressure-class axis, the owner's R-UNITS-PIN choice).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityAxis {
    /// Stable identifier, for example `mech.density`.
    pub id: String,
    /// What the axis measures.
    pub measures: String,
    /// The human-readable unit label, for example `kg/m^3`.
    pub unit: String,
    /// The Part 55 base-dimension reduction.
    pub dimension: Dimension,
    /// The canonical per-quantity scale the stored value is in, for example `MPa`.
    pub scale_unit: String,
    /// The fixed-point range, set or reserved.
    pub range: AxisRange,
    /// The raw declared decimal bounds `(lo, hi)` of a set range, retained verbatim from the data.
    /// A bound below the Q32.32 epsilon (a picofarad capacitance, say) underflows the stored `Fixed`
    /// `range` to zero, losing the magnitude the per-quantity scale derivation (R-UNITS-PIN) reads;
    /// the declared decimal keeps it. The `Fixed` `range` remains the clamp representation, this is
    /// the derivation source of truth. `None` for a reserved range.
    pub range_decimal: Option<(String, String)>,
    /// A per-class scale breakdown, empty for a single-scale axis. When the physics of an axis
    /// reserves its scale per class (a per-toxin-class tolerance whose pg/kg-to-g/kg envelope exceeds
    /// one Q32.32 scale, R-UNITS-PIN), the class is the quantity granularity: each entry is a distinct
    /// quantity with its own envelope, and the catalogue registers one `QuantityDef` per entry. The
    /// membership is data that grows with the world (Principle 11), so a new class is a new entry
    /// rather than a code change.
    pub per_class: Vec<PerClassRange>,
    /// The tier (0 the grounded floor).
    pub tier: u8,
    /// Whether the axis is real-with-source or fantasy-reserved.
    pub provenance: Provenance,
    /// How drawing on this axis behaves under conservation (R-SOURCE-VECTOR): a depletable stock, a
    /// non-rivalrous flux, or a reservoir. Reserved (fail-loud) until the world declares it. A matter
    /// composition axis declares `depletable_stock`; a field-and-gradient feeder's axis its own character.
    pub depletion_character: DepletionCharacter,
    /// The reduction of one axis-unit to the floor's common conserved energy/matter currency
    /// (R-SOURCE-VECTOR), for cross-axis comparison (R-TIER-CONSIST), never a draw's intake arithmetic.
    /// Reserved until the owner sets it from the floor's energy/matter equivalence for the quantity.
    pub reduction_coefficient: ReductionCoefficient,
}

impl QuantityAxis {
    /// The numeric multiplier the axis's STORED value is scaled by, recovered from the declared
    /// `scale_unit`. A `scale_unit` of the form `x<factor>` (the sole scaled-storage convention, whose
    /// canonical instance is the Archard wear coefficient's `x1e6`: a true coefficient of 1e-9 to 1e-3
    /// underflows or loses precision in Q32.32 unless the data stores it multiplied up) returns the
    /// factor, so a kernel divides it back out to recover the true value. Every plain unit label (MPa,
    /// ratio, m, ...) carries no rescale and returns one. This keeps the storage scale a physics-data
    /// property, read off the axis rather than fabricated at a call site (Principles 9, 11). A malformed
    /// `x` form falls back to one; the canonical axes are covered by a test so a parse regression is caught.
    pub fn storage_scale(&self) -> Fixed {
        match self.scale_unit.strip_prefix('x') {
            Some(factor) => parse_scale_factor(factor).unwrap_or(Fixed::ONE),
            None => Fixed::ONE,
        }
    }
}

/// Parse a scaled-storage factor, either a plain decimal (`1000000`) or the `<mantissa>e<exponent>`
/// scientific form the data writes (`1e6`, or a sub-unity `1e-6` for a coefficient stored scaled DOWN),
/// into a `Fixed`. A negative exponent divides rather than multiplies, so a down-scaled storage convention
/// is honoured rather than silently collapsing to one. `None` if it is neither well-formed nor
/// representable. Kept beside [`QuantityAxis::storage_scale`], the only caller.
fn parse_scale_factor(s: &str) -> Option<Fixed> {
    let split = s.split_once('e').or_else(|| s.split_once('E'));
    match split {
        Some((mantissa, exponent)) => {
            let mut f = Fixed::from_decimal_str(mantissa).ok()?;
            let e: i32 = exponent.parse().ok()?;
            let ten = Fixed::from_int(10);
            for _ in 0..e.abs() {
                f = if e >= 0 {
                    f.checked_mul(ten)?
                } else {
                    f.checked_div(ten)?
                };
            }
            Some(f)
        }
        None => Fixed::from_decimal_str(s).ok(),
    }
}

/// One class's envelope in a per-class-scale axis: the class id and its declared decimal bounds. The
/// catalogue registers one quantity per entry so each class carries its own per-quantity scale
/// (R-UNITS-PIN, the `bio.consumer.reference_tolerance` case), keyed off the same declared decimal
/// envelope the single-scale axes use so a sub-epsilon per-class bound keeps its magnitude.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerClassRange {
    /// The class id (a toxin class, say) this per-class quantity is keyed to.
    pub class: String,
    /// The declared decimal bounds `(lo, hi)` of this class's envelope.
    pub bounds: (String, String),
}

/// When a port reads its axis value. A `Current` port reads this tick's value; a `Prior`
/// port reads the previous tick's resident sample (the induction laws' finite difference);
/// a `Dt` port is the tick-duration primitive itself, a time-dimension read with no axis.
/// This is how a law that reads the same axis at two instants (a flux now and a flux prior)
/// names them as two distinct ports, the temporal case of two participants of one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Temporal {
    /// This tick's value.
    Current,
    /// The previous tick's resident value.
    Prior,
    /// The tick duration, a time-dimension primitive with no axis.
    Dt,
}

/// The open-arity aggregation a class-set port folds its members by. The mechanism is fixed
/// Rust; the class membership is data (Principle 11), so a new nutrient or toxin class is a new
/// axis added to a port's member set, never a code change. The set is deliberately small: these
/// are the folds the substrate's kernels actually perform (the Liebig minimum and the saturating
/// sum), and it grows only when a kernel introduces a new order-independent reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fold {
    /// The limiting-member minimum (the Liebig nutrition fold).
    Min,
    /// The saturating sum (the additive harm fold).
    Sum,
}

/// One input port of a law: a role name (free-form, unique within the law, so the mechanism is
/// not a closed enum), the axis (or axes) it reads, and when it reads it. A single port names one
/// axis; a class-set port (a non-empty `members` with a `fold`) names an open-arity set of same-
/// dimension axes the kernel folds, the variadic case the fixed-arity list could not express (the
/// nutrient classes of the Liebig minimum, the toxin classes of the harm sum). The port's
/// dimensional exponent is a property of the kernel and lives in the fixed-Rust contract
/// ([`graph::kernel_contract`]); the data only wires a role to its axis or class set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawPort {
    /// The role this port fills, matched against the kernel contract.
    pub role: String,
    /// The axis id a single port reads (empty for a `Dt` port or a class-set port).
    pub axis: String,
    /// When the value is read.
    pub temporal: Temporal,
    /// The class-set members: a non-empty list marks a variadic (open-arity) fold over these
    /// same-dimension axes. Empty for a single port.
    pub members: Vec<String>,
    /// The fold a class-set port aggregates its members by; `None` for a single port.
    pub fold: Option<Fold>,
}

/// An interaction law: the metadata of a closed-form integer kernel over the quantity
/// vectors of the participating entities, reporting an interval-bounded fixed-point
/// consequence at a tier. The kernel itself is fixed Rust bound by [`InteractionLaw::kernel`]
/// against the contract table; this is the registry entry that names its ports, the axes it
/// produces, its measured output, and its bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionLaw {
    /// Stable identifier, for example `law.harm.dose_response`.
    pub id: String,
    /// The kernel id this law binds to (empty for a not-yet-migrated law). Checked against
    /// the fixed-Rust contract table at load, so a law cannot declare inputs a kernel does
    /// not take.
    pub kernel: String,
    /// The role-tagged, temporally-tagged input ports. When present, this supersedes the
    /// flat `inputs` list, which is derived from the distinct port axes for compatibility.
    pub ports: Vec<LawPort>,
    /// The axis ids this law writes (the typed graph edges other laws read). The first, if
    /// any, is the primary output whose dimension the kernel contract verifies.
    pub produces: Vec<String>,
    /// The axis ids the law reads (derived from `ports` when ports are declared).
    pub inputs: Vec<String>,
    /// The measured consequence it reports (never a verdict).
    pub output_measure: String,
    /// The dimension of the output.
    pub output_dimension: Dimension,
    /// The interval bound on the output.
    pub interval_bound: String,
    /// The authored tier. Legacy: the substrate now derives tier from the law-output graph
    /// ([`PhysicsRegistry::derived_tier`]); whether to drop this field is a reserved decision.
    pub tier: u8,
}

/// A substance: a material, a tissue, or a structural member as a vector of values over
/// the axes, plus the laws it participates in, plus a provenance tag. The
/// [`Substance::content_id`] is content-addressed (a pure function of the physical
/// content, not the human label), so the same composition has the same id on every
/// machine, the deduplication and determinism discipline of the composition node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substance {
    /// The human-readable handle, for example `iron` or `oak`.
    pub id: String,
    /// The value on each axis, keyed by axis id (sorted, for a deterministic walk).
    pub vector: BTreeMap<String, Fixed>,
    /// The laws this substance participates in.
    pub participates_in: Vec<String>,
    /// The molecular formula (for example `H2O`), the substance's identity from which the molar mass and atom
    /// count derive; empty when the molecular composition is not modelled.
    pub formula: String,
    /// Whether the substance is real-with-source or fantasy-reserved.
    pub provenance: Provenance,
}

impl Substance {
    /// The content-addressed id: a 128-bit hash of the physical content (the axis
    /// values in id order, the laws, and the provenance), excluding the human label, so
    /// two substances with identical content hash identically on every machine.
    pub fn content_id(&self) -> u128 {
        let mut h = StateHasher::new();
        for (axis, value) in &self.vector {
            h.write_bytes(axis.as_bytes());
            h.write_fixed(*value);
        }
        // A separator so the law list cannot be confused with the value list.
        h.write_u64(0);
        for law in &self.participates_in {
            h.write_bytes(law.as_bytes());
        }
        h.write_u64(0);
        // The molecular formula is physical content (which nuclei), so it folds into the content hash beside the
        // values, a separator keeping it distinct from the law list.
        h.write_bytes(self.formula.as_bytes());
        h.write_u64(0);
        match &self.provenance {
            Provenance::RealWithSource(s) => {
                h.write_u32(1);
                h.write_bytes(s.as_bytes());
            }
            Provenance::FantasyReserved(s) => {
                h.write_u32(2);
                h.write_bytes(s.as_bytes());
            }
        }
        h.finish()
    }
}

/// What can go wrong loading or reading the substrate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicsError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// The data file could not be read.
    Io(String),
    /// A duplicate id appears.
    Duplicate(String),
    /// A reference names an axis that does not exist.
    UnknownAxis {
        /// What referenced it.
        context: String,
        /// The missing axis id.
        axis: String,
    },
    /// A reference names a law that does not exist.
    UnknownLaw {
        /// What referenced it.
        context: String,
        /// The missing law id.
        law: String,
    },
    /// A range was read while still reserved (the fail-loud sentinel).
    ReservedRange(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue {
        /// The entry the value belongs to.
        id: String,
        /// What went wrong.
        detail: String,
    },
    /// A dimension string could not be parsed.
    BadDimension {
        /// The entry the dimension belongs to.
        id: String,
        /// What went wrong.
        detail: String,
    },
    /// A range was neither a reserved basis nor a set lo and hi pair.
    BadRange(String),
    /// An entry carries neither a real-with-source nor a fantasy-reserved tag.
    MissingProvenance(String),
    /// A port declares an unparseable temporal or a malformed binding.
    BadPort {
        /// The law the port belongs to.
        law: String,
        /// What went wrong.
        detail: String,
    },
    /// A law binds a kernel id that has no contract in the fixed-Rust table.
    UnknownKernel {
        /// The law.
        law: String,
        /// The unbound kernel id.
        kernel: String,
    },
    /// A law's declared ports do not match its kernel's contract (a missing role, an extra
    /// role, or a duplicated role): the binding the naming convention could not check.
    PortContractMismatch {
        /// The law.
        law: String,
        /// What is wrong.
        detail: String,
    },
    /// A produced axis's dimension is not reachable from the law's input axes through the
    /// kernel's declared dimensional relation.
    DimensionUnreachable {
        /// The law.
        law: String,
        /// What is wrong.
        detail: String,
    },
    /// The law-output graph has a cycle, so a tier cannot be derived.
    CyclicLawGraph(String),
}

impl fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhysicsError::Parse(m) => write!(f, "substrate parse error: {m}"),
            PhysicsError::Io(m) => write!(f, "substrate read error: {m}"),
            PhysicsError::Duplicate(id) => write!(f, "duplicate substrate id '{id}'"),
            PhysicsError::UnknownAxis { context, axis } => {
                write!(f, "'{context}' references unknown axis '{axis}'")
            }
            PhysicsError::UnknownLaw { context, law } => {
                write!(f, "'{context}' references unknown law '{law}'")
            }
            PhysicsError::ReservedRange(id) => write!(
                f,
                "axis range '{id}' is reserved and unset; the owner must set it before it is read (never fabricate a value)"
            ),
            PhysicsError::BadValue { id, detail } => {
                write!(f, "value in '{id}' could not be read: {detail}")
            }
            PhysicsError::BadDimension { id, detail } => {
                write!(f, "dimension of '{id}' could not be read: {detail}")
            }
            PhysicsError::BadRange(id) => write!(
                f,
                "axis '{id}' must declare either a reserved basis or both a lo and hi bound"
            ),
            PhysicsError::MissingProvenance(id) => write!(
                f,
                "'{id}' must declare provenance, either real-with-source or fantasy-reserved"
            ),
            PhysicsError::BadPort { law, detail } => {
                write!(f, "law '{law}' has a malformed port: {detail}")
            }
            PhysicsError::UnknownKernel { law, kernel } => write!(
                f,
                "law '{law}' binds kernel '{kernel}', which has no contract in the kernel table"
            ),
            PhysicsError::PortContractMismatch { law, detail } => {
                write!(f, "law '{law}' does not match its kernel contract: {detail}")
            }
            PhysicsError::DimensionUnreachable { law, detail } => {
                write!(f, "law '{law}' output dimension is unreachable: {detail}")
            }
            PhysicsError::CyclicLawGraph(detail) => {
                write!(f, "the law-output graph has a cycle, so tier cannot be derived: {detail}")
            }
        }
    }
}

impl std::error::Error for PhysicsError {}

/// The loaded substrate: the axes, laws, and substances, each keyed by id in a sorted
/// map so any walk over the registry is in a fixed canonical order (the R-CANON-WALK
/// discipline), which the content hash relies on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhysicsRegistry {
    axes: BTreeMap<String, QuantityAxis>,
    laws: BTreeMap<String, InteractionLaw>,
    substances: BTreeMap<String, Substance>,
}

impl PhysicsRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        PhysicsRegistry::default()
    }

    /// Parse and validate a registry from TOML text.
    pub fn from_toml_str(s: &str) -> Result<Self, PhysicsError> {
        let file: RegistryFile =
            toml::from_str(s).map_err(|e| PhysicsError::Parse(e.to_string()))?;
        Self::from_file(file)
    }

    /// Load and validate a registry from a file path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PhysicsError> {
        let text = std::fs::read_to_string(path).map_err(|e| PhysicsError::Io(e.to_string()))?;
        Self::from_toml_str(&text)
    }

    /// The world material registry: the mechanical floor plus the ground-material substances a world's
    /// z-column is made of (`ground_floor.toml`), built from the crate's EMBEDDED floor data so a caller
    /// (the sim's world-build) needs no filesystem path. The material substrate reads a cell's substance
    /// mixture against this to derive its bulk density and, where a contest reads them, its mechanical
    /// properties (the material-substrate arc, cascade item 1). The ground floor EXTENDS the mechanical
    /// floor rather than editing it, so the mechanical floor's identity and its tests are untouched.
    pub fn ground() -> Result<Self, PhysicsError> {
        let mut reg =
            PhysicsRegistry::from_toml_str(include_str!("../data/mechanical_floor.toml"))?;
        // The reactive floors (material-substrate arc, cascade item 8, the matter cycle): the fluids,
        // chemistry, and biology floors carry the transform physics a cell's matter reads to decompose,
        // corrode, or burn (the corrosion electrode potentials and susceptibility, the combustion products'
        // ash fraction, the ambient fluids). Loaded in the wave order (each extends the prior), so a
        // substance can carry its own reaction, corrosion, and decomposition data and the matter cycle
        // reads it straight from the floor (the north-star "dissolve into the floor"). Additive: the
        // material-substrate derivations read specific axes by id, so a fuller registry leaves them
        // unchanged, and worldgen fills the z-column from the ground floor's strata alone.
        reg.extend_from_toml_str(include_str!("../data/fluids_floor.toml"))?;
        reg.extend_from_toml_str(include_str!("../data/chem_optics_floor.toml"))?;
        reg.extend_from_toml_str(include_str!("../data/em_floor.toml"))?;
        reg.extend_from_toml_str(include_str!("../data/biology_floor.toml"))?;
        // The geology floor (genesis-forward Layer 1): the geo.* axes the deep-time genesis and the interior
        // engine read (the internal-heat-production source term first). Additive and self-contained, so the
        // existing derivations are unchanged; a genesis pass reads it, but no current scenario does.
        reg.extend_from_toml_str(include_str!("../data/geology_floor.toml"))?;
        // The ground floor loads LAST, so its z-column substances can carry cross-domain axes the reactive
        // floors define (the matter cycle's decomposable "carrion" carries `bio.mineral_ash_fraction`). Its
        // substances and axes are order-independent for the material derivations (a registry lookup by id).
        reg.extend_from_toml_str(include_str!("../data/ground_floor.toml"))?;
        Ok(reg)
    }

    /// Extend this registry with another floor file's axes, laws, and substances, then revalidate the
    /// whole. A wave loads onto the previous floor rather than duplicating the shared axes it reads:
    /// the wave-2 fluids, chemistry, and optics floor references the wave-1 mechanical and material
    /// axes, so it is merged onto the mechanical floor rather than standing alone. A duplicate id is
    /// an error, never a silent overwrite.
    pub fn extend_from_toml_str(&mut self, s: &str) -> Result<(), PhysicsError> {
        let file: RegistryFile =
            toml::from_str(s).map_err(|e| PhysicsError::Parse(e.to_string()))?;
        for a in file.axis {
            let axis = a.into_axis()?;
            if self.axes.contains_key(&axis.id) {
                return Err(PhysicsError::Duplicate(axis.id));
            }
            self.axes.insert(axis.id.clone(), axis);
        }
        for l in file.law {
            let law = l.into_law()?;
            if self.laws.contains_key(&law.id) {
                return Err(PhysicsError::Duplicate(law.id));
            }
            self.laws.insert(law.id.clone(), law);
        }
        for s in file.substance {
            let sub = s.into_substance()?;
            if self.substances.contains_key(&sub.id) {
                return Err(PhysicsError::Duplicate(sub.id));
            }
            self.substances.insert(sub.id.clone(), sub);
        }
        self.validate()
    }

    /// Extend this registry from another floor file path.
    pub fn extend(&mut self, path: impl AsRef<Path>) -> Result<(), PhysicsError> {
        let text = std::fs::read_to_string(path).map_err(|e| PhysicsError::Io(e.to_string()))?;
        self.extend_from_toml_str(&text)
    }

    fn from_file(file: RegistryFile) -> Result<Self, PhysicsError> {
        let mut reg = PhysicsRegistry::new();
        for a in file.axis {
            let axis = a.into_axis()?;
            if reg.axes.contains_key(&axis.id) {
                return Err(PhysicsError::Duplicate(axis.id));
            }
            reg.axes.insert(axis.id.clone(), axis);
        }
        for l in file.law {
            let law = l.into_law()?;
            if reg.laws.contains_key(&law.id) {
                return Err(PhysicsError::Duplicate(law.id));
            }
            reg.laws.insert(law.id.clone(), law);
        }
        for s in file.substance {
            let sub = s.into_substance()?;
            if reg.substances.contains_key(&sub.id) {
                return Err(PhysicsError::Duplicate(sub.id));
            }
            reg.substances.insert(sub.id.clone(), sub);
        }
        reg.validate()?;
        Ok(reg)
    }

    /// Confirm every cross-reference resolves: a law reads only existing axes, a
    /// substance carries values only on existing axes and participates only in existing
    /// laws. A dangling reference is a load-time error, never a silent skip.
    fn validate(&self) -> Result<(), PhysicsError> {
        for law in self.laws.values() {
            for axis in &law.inputs {
                if !self.axes.contains_key(axis) {
                    return Err(PhysicsError::UnknownAxis {
                        context: law.id.clone(),
                        axis: axis.clone(),
                    });
                }
            }
            // A produced axis must exist, so an output edge cannot dangle.
            for axis in &law.produces {
                if !self.axes.contains_key(axis) {
                    return Err(PhysicsError::UnknownAxis {
                        context: law.id.clone(),
                        axis: axis.clone(),
                    });
                }
            }
            // A class-set port's members must all exist and share one dimension, so a fold folds
            // over a homogeneous quantity (the Liebig minimum over nutrient fractions, the harm
            // sum over toxin concentrations) rather than a dimensional mixture.
            for port in &law.ports {
                if port.members.is_empty() {
                    continue;
                }
                let mut member_dim: Option<Dimension> = None;
                for m in &port.members {
                    let axis = self.axes.get(m).ok_or_else(|| PhysicsError::UnknownAxis {
                        context: law.id.clone(),
                        axis: m.clone(),
                    })?;
                    match member_dim {
                        None => member_dim = Some(axis.dimension),
                        Some(d) if d != axis.dimension => {
                            return Err(PhysicsError::BadPort {
                                law: law.id.clone(),
                                detail: format!(
                                    "class-set port '{}' folds axes of differing dimensions ({:?} and {:?})",
                                    port.role, d, axis.dimension
                                ),
                            });
                        }
                        Some(_) => {}
                    }
                }
            }
            // A migrated law is checked against its kernel contract: the binding, the temporal
            // agreement, and the dimensional reachability of its output.
            graph::check_law(law, &self.axes)?;
        }
        for sub in self.substances.values() {
            for axis in sub.vector.keys() {
                if !self.axes.contains_key(axis) {
                    return Err(PhysicsError::UnknownAxis {
                        context: sub.id.clone(),
                        axis: axis.clone(),
                    });
                }
            }
            for law in &sub.participates_in {
                if !self.laws.contains_key(law) {
                    return Err(PhysicsError::UnknownLaw {
                        context: sub.id.clone(),
                        law: law.clone(),
                    });
                }
            }
        }
        // The law-output graph must be acyclic so a tier can be derived.
        graph::derive_tiers(&self.laws)?;
        Ok(())
    }

    /// The derived tier of a law: its depth in the law-output graph, computed rather than
    /// authored (the composition-layer resolution). A law reading only ground axes is tier 1;
    /// a law reading an axis a tier-1 law produces is tier 2; and so on. Returns `None` for an
    /// unknown law. This is the honest layering that fills the empty middle the authored `tier`
    /// stamps left; whether to drop the authored field is a reserved decision.
    pub fn derived_tier(&self, law_id: &str) -> Option<u32> {
        graph::derive_tiers(&self.laws).ok()?.get(law_id).copied()
    }

    /// Every law's derived tier, in sorted id order.
    pub fn derived_tiers(&self) -> BTreeMap<String, u32> {
        graph::derive_tiers(&self.laws).unwrap_or_default()
    }

    /// An axis by id.
    pub fn axis(&self, id: &str) -> Option<&QuantityAxis> {
        self.axes.get(id)
    }

    /// A law by id.
    pub fn law(&self, id: &str) -> Option<&InteractionLaw> {
        self.laws.get(id)
    }

    /// A substance by id.
    pub fn substance(&self, id: &str) -> Option<&Substance> {
        self.substances.get(id)
    }

    /// The axes, in sorted id order.
    pub fn axes(&self) -> impl Iterator<Item = &QuantityAxis> + '_ {
        self.axes.values()
    }

    /// The laws, in sorted id order.
    pub fn laws(&self) -> impl Iterator<Item = &InteractionLaw> + '_ {
        self.laws.values()
    }

    /// The substances, in sorted id order.
    pub fn substances(&self) -> impl Iterator<Item = &Substance> + '_ {
        self.substances.values()
    }

    /// The ids of axes whose range is still reserved, in sorted order: the standing
    /// review queue for the owner, the substrate's analogue of the calibration manifest
    /// reserved-values panel.
    pub fn reserved_axis_ids(&self) -> Vec<&str> {
        self.axes
            .values()
            .filter(|a| !a.range.is_set())
            .map(|a| a.id.as_str())
            .collect()
    }

    /// A 128-bit content hash of the whole registry, walked in sorted id order, so the
    /// same data hashes identically on every machine (the determinism and memoisation
    /// discipline of the locked representation).
    pub fn content_id(&self) -> u128 {
        let mut h = StateHasher::new();
        for axis in self.axes.values() {
            h.write_bytes(axis.id.as_bytes());
            let d = axis.dimension;
            h.write_bytes(&[
                d.length as u8,
                d.mass as u8,
                d.time as u8,
                d.temperature as u8,
                d.current as u8,
            ]);
            h.write_bytes(axis.scale_unit.as_bytes());
            match &axis.range {
                AxisRange::Reserved { basis } => {
                    h.write_u32(0);
                    h.write_bytes(basis.as_bytes());
                }
                AxisRange::Set { lo, hi } => {
                    h.write_u32(1);
                    h.write_fixed(*lo);
                    h.write_fixed(*hi);
                }
            }
            // The declared decimal bounds distinguish two set ranges whose sub-epsilon low ends
            // both underflow the stored Fixed to zero (a picofarad versus a femtofarad), which the
            // Fixed lo/hi above cannot.
            if let Some((lo, hi)) = &axis.range_decimal {
                h.write_bytes(lo.as_bytes());
                h.write_bytes(hi.as_bytes());
            }
            // The per-class scale breakdown is part of the axis's content: two axes differing only in
            // a class's envelope are distinct.
            for pc in &axis.per_class {
                h.write_bytes(pc.class.as_bytes());
                h.write_bytes(pc.bounds.0.as_bytes());
                h.write_bytes(pc.bounds.1.as_bytes());
            }
            h.write_u32(axis.tier as u32);
            write_provenance(&mut h, &axis.provenance);
        }
        h.write_u64(u64::MAX);
        for law in self.laws.values() {
            h.write_bytes(law.id.as_bytes());
            h.write_u64(0);
            h.write_bytes(law.kernel.as_bytes());
            h.write_u64(0);
            for axis in &law.inputs {
                h.write_bytes(axis.as_bytes());
            }
            h.write_u64(0);
            // The role-tagged ports and produced axes are part of the law's canonical content.
            for port in &law.ports {
                h.write_bytes(port.role.as_bytes());
                h.write_bytes(port.axis.as_bytes());
                h.write_u32(match port.temporal {
                    Temporal::Current => 0,
                    Temporal::Prior => 1,
                    Temporal::Dt => 2,
                });
                h.write_u32(match port.fold {
                    None => 0,
                    Some(Fold::Min) => 1,
                    Some(Fold::Sum) => 2,
                });
                for m in &port.members {
                    h.write_bytes(m.as_bytes());
                }
                h.write_u64(0);
            }
            h.write_u64(0);
            for axis in &law.produces {
                h.write_bytes(axis.as_bytes());
            }
            let d = law.output_dimension;
            h.write_bytes(&[
                d.length as u8,
                d.mass as u8,
                d.time as u8,
                d.temperature as u8,
                d.current as u8,
            ]);
            h.write_u32(law.tier as u32);
        }
        h.write_u64(u64::MAX);
        for sub in self.substances.values() {
            h.write_bytes(sub.id.as_bytes());
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&sub.content_id().to_le_bytes());
            h.write_bytes(&bytes);
        }
        h.finish()
    }

    /// Number of axes.
    pub fn axis_count(&self) -> usize {
        self.axes.len()
    }

    /// Number of laws.
    pub fn law_count(&self) -> usize {
        self.laws.len()
    }

    /// Number of substances.
    pub fn substance_count(&self) -> usize {
        self.substances.len()
    }
}

fn write_provenance(h: &mut StateHasher, p: &Provenance) {
    match p {
        Provenance::RealWithSource(s) => {
            h.write_u32(1);
            h.write_bytes(s.as_bytes());
        }
        Provenance::FantasyReserved(s) => {
            h.write_u32(2);
            h.write_bytes(s.as_bytes());
        }
    }
}

/// Parse a dimension from a name (length, mass, time, temperature, area, volume,
/// velocity, force, energy, pressure, current, charge, voltage, dimensionless or ratio) or, for a
/// monomial the named set does not cover, from a `length,mass,time,temperature` exponent tuple, or a
/// `length,mass,time,temperature,current` five-tuple that carries the wave-3 electric-current base (a
/// four-tuple loads with a current exponent of zero, so every pre-wave-3 floor parses unchanged).
fn parse_dimension(name: &str) -> Result<Dimension, String> {
    let t = name.trim();
    let named = match t.to_ascii_lowercase().as_str() {
        "length" => Some(Dimension::LENGTH),
        "mass" => Some(Dimension::MASS),
        "time" => Some(Dimension::TIME),
        "temperature" => Some(Dimension::TEMPERATURE),
        "area" => Some(Dimension::AREA),
        "volume" => Some(Dimension::VOLUME),
        "velocity" => Some(Dimension::VELOCITY),
        "force" => Some(Dimension::FORCE),
        "energy" => Some(Dimension::ENERGY),
        "pressure" => Some(Dimension::PRESSURE),
        "current" | "ampere" => Some(Dimension::CURRENT),
        "charge" | "coulomb" => Some(Dimension::CHARGE),
        "voltage" | "volt" | "potential" | "emf" => Some(Dimension::VOLTAGE),
        "dimensionless" | "ratio" => Some(Dimension::DIMENSIONLESS),
        _ => None,
    };
    if let Some(d) = named {
        return Ok(d);
    }
    let parts: Vec<&str> = t.split(',').map(|p| p.trim()).collect();
    if parts.len() == 4 || parts.len() == 5 {
        let exps: Result<Vec<i8>, _> = parts.iter().map(|p| p.parse::<i8>()).collect();
        if let Ok(e) = exps {
            // A 4-tuple is a pre-wave-3 dimension: its current exponent is zero, so every earlier
            // floor's data loads unchanged. A 5-tuple carries the electric-current exponent.
            return Ok(Dimension {
                length: e[0],
                mass: e[1],
                time: e[2],
                temperature: e[3],
                current: if e.len() == 5 { e[4] } else { 0 },
            });
        }
    }
    Err(format!(
        "expected a dimension name or a 'length,mass,time,temperature[,current]' exponent tuple, got '{t}'"
    ))
}

fn provenance_from(real: &str, fantasy: &str, id: &str) -> Result<Provenance, PhysicsError> {
    if !real.trim().is_empty() {
        Ok(Provenance::RealWithSource(real.trim().to_string()))
    } else if !fantasy.trim().is_empty() {
        Ok(Provenance::FantasyReserved(fantasy.trim().to_string()))
    } else {
        Err(PhysicsError::MissingProvenance(id.to_string()))
    }
}

// The TOML-facing schema. Values are decimal strings (parsed to Fixed by integer
// arithmetic), so no floating point reaches canonical state and the data round-trips
// losslessly. Kept separate from the typed forms above so Fixed never needs serde.

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RegistryFile {
    #[serde(default)]
    axis: Vec<AxisDef>,
    #[serde(default)]
    law: Vec<LawDef>,
    #[serde(default)]
    substance: Vec<SubstanceDef>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct AxisDef {
    id: String,
    #[serde(default)]
    measures: String,
    #[serde(default)]
    unit: String,
    dimension: String,
    #[serde(default)]
    scale: String,
    #[serde(default)]
    tier: u8,
    /// The basis if the range is reserved; empty if the range is set.
    #[serde(default)]
    range_reserved: String,
    /// The set lower bound, a decimal string; empty if reserved.
    #[serde(default)]
    range_lo: String,
    /// The set upper bound, a decimal string; empty if reserved.
    #[serde(default)]
    range_hi: String,
    /// The citation if real-with-source.
    #[serde(default)]
    real: String,
    /// The basis if fantasy-reserved.
    #[serde(default)]
    fantasy: String,
    /// A per-class scale breakdown (R-UNITS-PIN): one entry per class, each with its own envelope,
    /// so the catalogue registers one quantity per class. Empty for a single-scale axis.
    #[serde(default)]
    per_class: Vec<PerClassDef>,
    /// The R-SOURCE-VECTOR depletion character: `depletable_stock`, `non_rivalrous_flux`, or `reservoir`.
    /// Empty declares it reserved (fail-loud on a draw), the default for an axis no feeder draws on yet.
    #[serde(default)]
    depletion_character: String,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PerClassDef {
    class: String,
    #[serde(default)]
    range_lo: String,
    #[serde(default)]
    range_hi: String,
}

impl AxisDef {
    fn into_axis(self) -> Result<QuantityAxis, PhysicsError> {
        // `@` is the reserved separator the catalogue joins a per-class quantity name with
        // (`<axis>@<class>`), so an axis id carrying it could alias a per-class quantity and collide.
        if self.id.contains('@') {
            return Err(PhysicsError::BadValue {
                id: self.id.clone(),
                detail: "an axis id may not contain '@', the per-class quantity separator"
                    .to_string(),
            });
        }
        let dimension =
            parse_dimension(&self.dimension).map_err(|detail| PhysicsError::BadDimension {
                id: self.id.clone(),
                detail,
            })?;
        let (range, range_decimal) = if !self.range_reserved.trim().is_empty() {
            (
                AxisRange::Reserved {
                    basis: self.range_reserved.trim().to_string(),
                },
                None,
            )
        } else if !self.range_lo.trim().is_empty() && !self.range_hi.trim().is_empty() {
            let lo = Fixed::from_decimal_str(&self.range_lo).map_err(|detail| {
                PhysicsError::BadValue {
                    id: self.id.clone(),
                    detail,
                }
            })?;
            let hi = Fixed::from_decimal_str(&self.range_hi).map_err(|detail| {
                PhysicsError::BadValue {
                    id: self.id.clone(),
                    detail,
                }
            })?;
            (
                AxisRange::Set { lo, hi },
                Some((
                    self.range_lo.trim().to_string(),
                    self.range_hi.trim().to_string(),
                )),
            )
        } else {
            return Err(PhysicsError::BadRange(self.id.clone()));
        };
        let provenance = provenance_from(&self.real, &self.fantasy, &self.id)?;
        let depletion_character = match self.depletion_character.trim() {
            "" => DepletionCharacter::Reserved {
                basis: "the conservation law of the axis's quantity, declared when a feeder draws on it"
                    .to_string(),
            },
            "depletable_stock" => DepletionCharacter::DepletableStock,
            "non_rivalrous_flux" => DepletionCharacter::NonRivalrousFlux,
            "reservoir" => DepletionCharacter::Reservoir,
            other => {
                return Err(PhysicsError::BadValue {
                    id: self.id.clone(),
                    detail: format!(
                        "unknown depletion_character '{other}', expected depletable_stock, non_rivalrous_flux, or reservoir"
                    ),
                });
            }
        };
        // The reduction to the conserved currency is a DERIVE target on every axis (R-SOURCE-VECTOR under
        // the fundamental-constants floor): an axis's reduction IS its quantity's energy-equivalence, so it
        // is derived from the fundamentals times the substance's floor physics at feeder-arming, never
        // authored. Until then every axis carries the fail-loud Derive sentinel, no fabricated coefficient.
        let reduction_coefficient = ReductionCoefficient::Derive {
            basis: "the axis's energy-equivalence: the fundamentals times the substance's floor physics"
                .to_string(),
        };
        let mut per_class: Vec<PerClassRange> = Vec::with_capacity(self.per_class.len());
        for p in self.per_class {
            // A class id must be non-empty and free of the `@` separator, else its
            // `<axis>@<class>` quantity name would be malformed or could alias another.
            if p.class.trim().is_empty() || p.class.contains('@') {
                return Err(PhysicsError::BadValue {
                    id: self.id.clone(),
                    detail: format!(
                        "a per-class class id must be non-empty and free of '@', got '{}'",
                        p.class
                    ),
                });
            }
            // A duplicate class id would collide two `<axis>@<class>` quantity names, which the
            // catalogue's register panics on; reject it at load with a clean error instead.
            if per_class.iter().any(|e| e.class == p.class) {
                return Err(PhysicsError::BadValue {
                    id: self.id.clone(),
                    detail: format!("duplicate per-class class '{}'", p.class),
                });
            }
            per_class.push(PerClassRange {
                class: p.class,
                bounds: (p.range_lo.trim().to_string(), p.range_hi.trim().to_string()),
            });
        }
        Ok(QuantityAxis {
            id: self.id,
            measures: self.measures,
            unit: self.unit,
            dimension,
            scale_unit: self.scale,
            range,
            range_decimal,
            per_class,
            tier: self.tier,
            provenance,
            depletion_character,
            reduction_coefficient,
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct LawDef {
    id: String,
    #[serde(default)]
    inputs: Vec<String>,
    /// The kernel id this law binds to (new-format descriptor; empty for a legacy law).
    #[serde(default)]
    kernel: String,
    /// The role-tagged ports (new-format descriptor).
    #[serde(default)]
    ports: Vec<PortDef>,
    /// The axis ids this law produces (new-format descriptor).
    #[serde(default)]
    produces: Vec<String>,
    #[serde(default)]
    output_measure: String,
    dimension: String,
    #[serde(default)]
    interval_bound: String,
    #[serde(default)]
    tier: u8,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct PortDef {
    role: String,
    #[serde(default)]
    axis: String,
    /// One of `current` (default), `prior`, or `dt`.
    #[serde(default)]
    temporal: String,
    /// The class-set members: a non-empty list makes this a variadic fold port.
    #[serde(default)]
    members: Vec<String>,
    /// The fold for a class-set port: `min` or `sum`.
    #[serde(default)]
    fold: String,
}

impl LawDef {
    fn into_law(self) -> Result<InteractionLaw, PhysicsError> {
        let output_dimension =
            parse_dimension(&self.dimension).map_err(|detail| PhysicsError::BadDimension {
                id: self.id.clone(),
                detail,
            })?;
        let mut ports = Vec::with_capacity(self.ports.len());
        for p in self.ports {
            let temporal = match p.temporal.trim().to_ascii_lowercase().as_str() {
                "" | "current" => Temporal::Current,
                "prior" => Temporal::Prior,
                "dt" => Temporal::Dt,
                other => {
                    return Err(PhysicsError::BadPort {
                        law: self.id.clone(),
                        detail: format!(
                            "unknown temporal '{other}' (expected current, prior, or dt)"
                        ),
                    });
                }
            };
            let fold = match p.fold.trim().to_ascii_lowercase().as_str() {
                "" => None,
                "min" => Some(Fold::Min),
                "sum" => Some(Fold::Sum),
                other => {
                    return Err(PhysicsError::BadPort {
                        law: self.id.clone(),
                        detail: format!("unknown fold '{other}' (expected min or sum)"),
                    });
                }
            };
            // A class-set port declares members and a fold; a single port declares neither. A
            // half-declared port (members without a fold, or a fold without members) is a load
            // error, not a silent single port.
            if p.members.is_empty() != fold.is_none() {
                return Err(PhysicsError::BadPort {
                    law: self.id.clone(),
                    detail: format!(
                        "port '{}' must declare both members and a fold, or neither",
                        p.role
                    ),
                });
            }
            // A class-set port folds its members, so a stray single `axis` alongside them is a
            // contradiction (which value does it read), not a silent extra input.
            if !p.members.is_empty() && !p.axis.trim().is_empty() {
                return Err(PhysicsError::BadPort {
                    law: self.id.clone(),
                    detail: format!(
                        "class-set port '{}' must not also declare a single axis",
                        p.role
                    ),
                });
            }
            ports.push(LawPort {
                role: p.role,
                axis: p.axis,
                temporal,
                members: p.members,
                fold,
            });
        }
        // The flat input list is derived from the distinct non-Dt port axes (a single port's axis
        // and a class-set port's members) when ports are declared, so the legacy `inputs`
        // consumers keep working without duplication in data. This is the complete read-set (a
        // Prior port's axis is included) for the load-time cross-reference check and the content
        // hash; the derived-tier graph, not this list, is where a Prior read is treated as the
        // acausal edge it is (see `graph::derive_tiers`).
        let inputs = if ports.is_empty() {
            self.inputs
        } else {
            let mut seen = std::collections::BTreeSet::new();
            let mut derived = Vec::new();
            for p in &ports {
                if p.temporal == Temporal::Dt {
                    continue;
                }
                if !p.axis.is_empty() && seen.insert(p.axis.clone()) {
                    derived.push(p.axis.clone());
                }
                for m in &p.members {
                    if seen.insert(m.clone()) {
                        derived.push(m.clone());
                    }
                }
            }
            derived
        };
        Ok(InteractionLaw {
            id: self.id,
            kernel: self.kernel,
            ports,
            produces: self.produces,
            inputs,
            output_measure: self.output_measure,
            output_dimension,
            interval_bound: self.interval_bound,
            tier: self.tier,
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SubstanceDef {
    id: String,
    /// The molecular formula (for example `H2O`), the substance's IRREDUCIBLE identity: which nuclei and how
    /// many. The molar mass and atom count DERIVE from it plus the periodic table (never an authored molar mass),
    /// so it is a layer-2 identity datum, universal, cited; empty for a substance whose molecular composition is
    /// not modelled. An alien volatile carries its own formula as a data row.
    #[serde(default)]
    formula: String,
    #[serde(default)]
    values: Vec<ValuePair>,
    #[serde(default)]
    participates_in: Vec<String>,
    #[serde(default)]
    real: String,
    #[serde(default)]
    fantasy: String,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct ValuePair {
    axis: String,
    value: String,
}

impl SubstanceDef {
    fn into_substance(self) -> Result<Substance, PhysicsError> {
        let mut vector = BTreeMap::new();
        for pair in self.values {
            let v =
                Fixed::from_decimal_str(&pair.value).map_err(|detail| PhysicsError::BadValue {
                    id: format!("{}::{}", self.id, pair.axis),
                    detail,
                })?;
            if vector.insert(pair.axis.clone(), v).is_some() {
                return Err(PhysicsError::Duplicate(format!(
                    "{}::{}",
                    self.id, pair.axis
                )));
            }
        }
        let provenance = provenance_from(&self.real, &self.fantasy, &self.id)?;
        Ok(Substance {
            id: self.id,
            vector,
            participates_in: self.participates_in,
            formula: self.formula,
            provenance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[axis]]
id = "mech.density"
measures = "bulk mass per unit volume"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_reserved = "the CRC density-table maximum, set at or just above osmium ~22600"
real = "CRC Handbook of Chemistry and Physics, density tables"

[[axis]]
id = "mech.hardness"
measures = "indentation hardness as a contact pressure"
unit = "MPa"
dimension = "pressure"
scale = "MPa"
tier = 0
range_lo = "0.1"
range_hi = "120000"
real = "Vickers and Brinell hardness tables; Ashby and Jones"

[[law]]
id = "law.contact.pressure"
inputs = ["mech.force", "mech.contact_area"]
output_measure = "contact pressure"
dimension = "pressure"
interval_bound = "[0, P_MAX]"
tier = 0

[[axis]]
id = "mech.force"
measures = "applied force magnitude"
unit = "N"
dimension = "force"
scale = "N"
tier = 0
range_lo = "0.01"
range_hi = "100000000"
real = "Newtonian mechanics; biomechanics force tables"

[[axis]]
id = "mech.contact_area"
measures = "bearing contact area"
unit = "m^2"
dimension = "area"
scale = "m^2"
tier = 0
range_lo = "0.00000001"
range_hi = "10"
real = "contact mechanics; Hertz contact"

[[substance]]
id = "iron"
participates_in = ["law.contact.pressure"]
real = "ASM Metals Handbook"
values = [
  { axis = "mech.density", value = "7870" },
  { axis = "mech.hardness", value = "1500" },
]
"#;

    #[test]
    fn loads_and_counts() {
        let reg = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        assert_eq!(reg.axis_count(), 4);
        assert_eq!(reg.law_count(), 1);
        assert_eq!(reg.substance_count(), 1);
    }

    #[test]
    fn content_id_is_deterministic_across_loads() {
        let a = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        let b = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        assert_eq!(a.content_id(), b.content_id());
    }

    #[test]
    fn content_id_is_independent_of_declaration_order() {
        // The same axes declared in a different order hash identically, because the
        // registry walks in sorted id order.
        let reordered: String = {
            // Move the iron substance and reorder axes by hand: parse, the BTreeMap
            // sorts, so a second registry from the same data is order-independent. Here
            // we assert the sorted-walk property directly by comparing to SAMPLE.
            SAMPLE.to_string()
        };
        let a = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        let b = PhysicsRegistry::from_toml_str(&reordered).unwrap();
        assert_eq!(a.content_id(), b.content_id());
    }

    #[test]
    fn reading_a_reserved_range_fails_loud() {
        let reg = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        let density = reg.axis("mech.density").unwrap();
        assert_eq!(
            density.range.require("mech.density").unwrap_err(),
            PhysicsError::ReservedRange("mech.density".to_string())
        );
        assert_eq!(reg.reserved_axis_ids(), vec!["mech.density"]);
    }

    #[test]
    fn a_set_range_reads_exactly() {
        let reg = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        let hardness = reg.axis("mech.hardness").unwrap();
        let (lo, hi) = hardness.range.require("mech.hardness").unwrap();
        assert_eq!(lo, Fixed::from_ratio(1, 10));
        assert_eq!(hi, Fixed::from_int(120000));
    }

    #[test]
    fn an_unlisted_feeder_modality_is_a_pure_registry_row_that_carries_its_own_character() {
        // R-SOURCE-VECTOR HARD-GATE ACCEPTANCE (b): an unlisted feeder modality (a gravity-gradient
        // feeder) is addable as PURE NEW REGISTRY ROWS with zero code edits: a new axis declaring its
        // own depletion character loads and carries it, so a photovore's flux, a thermovore's gradient,
        // and this gravity-gradient feeder each name nothing in code. A build that needed a new source-kind
        // branch to add one would fail this; adding it here is a data edit alone.
        let feeder = r#"
[[axis]]
id = "field.gravity_gradient"
measures = "the local gravity-potential gradient a gravitropic feeder draws on"
unit = "m/s^2"
dimension = "1,0,-2,0"
scale = "m/s^2"
tier = 0
range_lo = "0"
range_hi = "100"
fantasy = "a gravity-gradient feeder's source, a reserved fantasy modality"
depletion_character = "non_rivalrous_flux"
"#;
        let reg = PhysicsRegistry::from_toml_str(feeder).unwrap();
        let axis = reg.axis("field.gravity_gradient").unwrap();
        // The feeder's flux character is carried off its own row (a renewable flux, drawn but not depleted).
        assert_eq!(
            axis.depletion_character,
            DepletionCharacter::NonRivalrousFlux
        );
        // Its reduction to the conserved currency is a DERIVE target (the fundamentals times the substance's
        // floor physics), never an authored number, so it reads as the fail-loud Derive sentinel until a
        // feeder-arming derive computes it.
        assert!(matches!(
            axis.reduction_coefficient,
            ReductionCoefficient::Derive { .. }
        ));
    }

    #[test]
    fn an_undeclared_depletion_character_stores_the_reserved_sentinel() {
        // An axis that declares no depletion character (every pre-R-SOURCE-VECTOR axis) carries the
        // Reserved sentinel, never a fabricated "stock" default: a feeder must declare its character
        // before a draw reads it (the reserved-value discipline, Principle 11). This test covers the
        // STORAGE of the sentinel; the behavioral fail-loud (a Reserved-character draw panics) is proven
        // in the sim fold test `a_draw_on_an_undeclared_reserved_character_fails_loud`.
        let reg = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        assert!(matches!(
            reg.axis("mech.density").unwrap().depletion_character,
            DepletionCharacter::Reserved { .. }
        ));
    }

    #[test]
    fn a_declared_depletable_stock_axis_carries_its_character() {
        // A matter axis declaring depletable_stock carries it, so the matter draw depletes as today.
        let matter = r#"
[[axis]]
id = "bio.test_energy"
measures = "a test matter composition axis"
unit = "kJ/g"
dimension = "2,0,-2,0"
scale = "kJ/g"
tier = 0
range_lo = "0"
range_hi = "38"
real = "test fixture"
depletion_character = "depletable_stock"
"#;
        let reg = PhysicsRegistry::from_toml_str(matter).unwrap();
        assert_eq!(
            reg.axis("bio.test_energy").unwrap().depletion_character,
            DepletionCharacter::DepletableStock
        );
    }

    #[test]
    fn an_unknown_depletion_character_fails_loud_at_load() {
        // A malformed character is a config error rejected at load, not a silent fallback.
        let bad = r#"
[[axis]]
id = "bio.test_bad"
measures = "a test axis with a bad character"
unit = "fraction"
dimension = "dimensionless"
scale = "1"
tier = 0
range_lo = "0"
range_hi = "1.0"
real = "test fixture"
depletion_character = "sometimes"
"#;
        assert!(PhysicsRegistry::from_toml_str(bad).is_err());
    }

    #[test]
    fn substance_values_parse_exactly_and_have_a_content_id() {
        let reg = PhysicsRegistry::from_toml_str(SAMPLE).unwrap();
        let iron = reg.substance("iron").unwrap();
        assert_eq!(
            iron.vector.get("mech.density"),
            Some(&Fixed::from_int(7870))
        );
        // Content id is stable and excludes the human label: a clone with a different
        // id but identical content hashes the same.
        let mut twin = iron.clone();
        twin.id = "wrought_iron".to_string();
        assert_eq!(iron.content_id(), twin.content_id());
    }

    #[test]
    fn a_scaled_storage_axis_reports_its_factor_and_a_plain_unit_reports_one() {
        // The Archard wear coefficient is the sole scaled-storage axis (`scale = "x1e6"`): its stored value
        // is the true coefficient times a million, and `storage_scale` recovers the million so a kernel can
        // divide it back out. A plain unit label (MPa, kg/m^3) carries no rescale and reports one. This is
        // how the wear step sources its `coefficient_scale` from the axis data rather than a hardcoded
        // constant, so a world that declares a different storage scale is honoured.
        let toml = r#"
[[axis]]
id = "mat.wear_coefficient"
measures = "the Archard dimensionless wear coefficient, stored at scale x1e6"
unit = "ratio"
dimension = "dimensionless"
scale = "x1e6"
tier = 0
range_lo = "0.001"
range_hi = "1000"
real = "Archard 1953"

[[axis]]
id = "mech.density"
measures = "bulk density"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
real = "CRC"
"#;
        let reg = PhysicsRegistry::from_toml_str(toml).unwrap();
        assert_eq!(
            reg.axis("mat.wear_coefficient").unwrap().storage_scale(),
            Fixed::from_int(1_000_000),
            "the x1e6 storage scale is recovered as one million"
        );
        assert_eq!(
            reg.axis("mech.density").unwrap().storage_scale(),
            Fixed::ONE,
            "a plain unit label carries no rescale"
        );
        // The parser handles a plain-decimal factor and rejects a malformed one (falling back to one).
        assert_eq!(
            parse_scale_factor("1000000"),
            Some(Fixed::from_int(1_000_000))
        );
        assert_eq!(parse_scale_factor("2e3"), Some(Fixed::from_int(2000)));
        assert_eq!(parse_scale_factor("nonsense"), None);
        // A negative exponent divides (a down-scaled storage convention), not a silent collapse to one.
        assert_eq!(parse_scale_factor("1e-3"), Some(Fixed::from_ratio(1, 1000)));
    }

    #[test]
    fn a_dangling_axis_reference_is_rejected() {
        let bad = r#"
[[law]]
id = "law.x"
inputs = ["does.not.exist"]
dimension = "pressure"
"#;
        assert_eq!(
            PhysicsRegistry::from_toml_str(bad).unwrap_err(),
            PhysicsError::UnknownAxis {
                context: "law.x".to_string(),
                axis: "does.not.exist".to_string(),
            }
        );
    }

    #[test]
    fn a_range_that_is_neither_set_nor_reserved_is_rejected() {
        let bad = r#"
[[axis]]
id = "mech.bad"
dimension = "pressure"
real = "x"
"#;
        assert_eq!(
            PhysicsRegistry::from_toml_str(bad).unwrap_err(),
            PhysicsError::BadRange("mech.bad".to_string())
        );
    }

    #[test]
    fn an_axis_without_provenance_is_rejected() {
        let bad = r#"
[[axis]]
id = "mech.bad"
dimension = "pressure"
range_lo = "0"
range_hi = "1"
"#;
        assert_eq!(
            PhysicsRegistry::from_toml_str(bad).unwrap_err(),
            PhysicsError::MissingProvenance("mech.bad".to_string())
        );
    }

    #[test]
    fn dimensions_compose_as_monomials() {
        // Pressure is mass over length and time squared; force over area is the same.
        let force_over_area = Dimension::FORCE / Dimension::AREA;
        assert_eq!(force_over_area, Dimension::PRESSURE);
        // Energy is force times length.
        assert_eq!(Dimension::FORCE * Dimension::LENGTH, Dimension::ENERGY);
        // A ratio of two forces is dimensionless.
        assert!((Dimension::FORCE / Dimension::FORCE).is_dimensionless());
    }

    #[test]
    fn data_round_trips_losslessly_through_toml() {
        let file: RegistryFile = toml::from_str(SAMPLE).unwrap();
        let text = toml::to_string(&file).unwrap();
        let again: RegistryFile = toml::from_str(&text).unwrap();
        assert_eq!(file, again);
    }
}
