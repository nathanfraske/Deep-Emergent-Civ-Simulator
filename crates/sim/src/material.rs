//! The material substrate: the located, structured MATTER that sits in the ground, the third kind of
//! world state beside the environmental FIELDS ([`crate::environ`], [`crate::runner`]'s temperature
//! field) and the ORGANISMS ([`crate::locomotion::Walker`]). A cell carries a MIXTURE of physics
//! [`Substance`](civsim_physics::Substance)s by volume, and its bulk mechanical properties (density,
//! indentation hardness) are DERIVED by reading the [`PhysicsRegistry`], never stored on the tile,
//! mirroring how [`crate::locomotion::ResourceField`] derives axis-presence from a
//! `HomeostaticRegistry` rather than tagging a tile.
//!
//! This is the base of the material-substrate arc (cascade item 1). Everything downstream reads a
//! located material with a derived hardness, weight, or fuel value: extraction reads its hardness,
//! carry weighs its density, crafting works its material, digging fractures it, fire burns its fuel,
//! and the matter cycle deposits into its soil classes. So the located substance mixture comes first
//! and the rest layer onto it.
//!
//! Steering posture. A matter KIND is a [`Substance`](civsim_physics::Substance) row in the physics
//! TOML, never a `Material{Rock,Soil,Ore,...}` enum (Principle 8): the field keys off substance ids
//! and grows with the world at zero code cost (Principle 11). A cell's mechanical properties are read
//! from the registry's authored floor data, never authored as scalars on the tile (Principle 11), and
//! the derivation reads only substance ids and floor axis ids, never a race, species, kind, or role
//! (Principle 9). The bulk aggregation over a mixture is a volume-weighted mean, stated below as the
//! modelling choice; it reduces to a pure substance's own value for a single-substance cell (the
//! common worldgen case).
//!
//! This module is off the run path: nothing reads the field or folds [`SubstanceMix::hash_into`] into
//! `state_hash` yet, so declaring the substrate leaves every existing scenario byte-identical. The
//! hash-changing wiring (worldgen population, the `state_hash` fold, and pointing an extraction
//! contest at a cell's derived hardness) lands in later opt-in slices.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StateHasher};
use civsim_physics::{laws, PhysicsRegistry};
use civsim_world::Coord3;

/// The bulk mass-per-unit-volume axis of the mechanical floor (`mechanical_floor.toml`), read to
/// derive a cell's density and the mass its matter carries.
const AXIS_DENSITY: &str = "mat.density";

/// The contact-pressure-a-surface-resists axis of the mechanical floor, read to derive the hardness an
/// extraction contest works against.
const AXIS_HARDNESS: &str = "mat.indentation_hardness";

/// The stress-a-substance-fractures-at axis of the mechanical floor, read to derive the resistance the
/// extraction contest must overcome to break matter loose (material-substrate arc, cascade item 4). This
/// is the axis the fracture law (`laws::fracture_onset`) gates on, distinct from the indentation hardness
/// (`AXIS_HARDNESS`) a plastic-indentation cut gates on.
const AXIS_FRACTURE: &str = "mat.fracture_strength";

/// The substance a single cell is made of: a mixture keyed by physics [`Substance`] id, each carrying
/// the VOLUME of that substance present in the cell (a fantasy or real substance alike, whatever the
/// registry declares). A substance the cell bears none of is simply absent (reads as zero, the
/// substrate absence convention shared with [`crate::edibility::Composition`] and
/// [`crate::locomotion::ResourceField`]). The mechanical properties are never stored here; they are
/// derived on demand by reading the [`PhysicsRegistry`] (see [`SubstanceMix::bulk_density`],
/// [`SubstanceMix::bulk_hardness`], [`SubstanceMix::mass`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubstanceMix {
    /// The volume of each substance present, keyed by its physics-floor substance id, walked in
    /// canonical (sorted) id order.
    volumes: BTreeMap<String, Fixed>,
}

impl SubstanceMix {
    /// An empty mixture (a void cell, no matter).
    pub fn new() -> SubstanceMix {
        SubstanceMix::default()
    }

    /// Whether the cell holds no matter at all.
    pub fn is_empty(&self) -> bool {
        self.volumes.is_empty()
    }

    /// Set the volume of one substance (overwriting any prior). A non-positive volume removes the
    /// substance, so a present entry always carries a real amount and the canonical walk stays minimal.
    pub fn set(&mut self, substance: &str, volume: Fixed) {
        if volume > Fixed::ZERO {
            self.volumes.insert(substance.to_string(), volume);
        } else {
            self.volumes.remove(substance);
        }
    }

    /// Add to the volume of one substance (a deposit: mined tailings, a dropped load, a decayed
    /// remains). Accumulates onto any present amount.
    pub fn add(&mut self, substance: &str, volume: Fixed) {
        if volume <= Fixed::ZERO {
            return;
        }
        let entry = self
            .volumes
            .entry(substance.to_string())
            .or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(volume);
    }

    /// The volume of one substance present; an absent substance reads as zero (the absence convention).
    pub fn volume(&self, substance: &str) -> Fixed {
        self.volumes.get(substance).copied().unwrap_or(Fixed::ZERO)
    }

    /// The total volume of all matter in the cell.
    pub fn total_volume(&self) -> Fixed {
        Fixed::saturating_sum(self.volumes.values().copied())
    }

    /// The substances present, in canonical (sorted) id order: the extraction and deposit draw walks
    /// this order so competition is an id-sorted sequential draw with no fresh randomness.
    pub fn substances(&self) -> impl Iterator<Item = (&String, &Fixed)> + '_ {
        self.volumes.iter()
    }

    /// Remove up to `want` volume of one substance, returning what was removed (never more than is
    /// present, never negative): the extraction draw, mirroring [`crate::locomotion::ResourceField::take`].
    /// A pruned-to-zero substance leaves the mixture so the canonical walk stays minimal. Reads and
    /// writes only the substance id's volume, no identity (Principle 9).
    pub fn take(&mut self, substance: &str, want: Fixed) -> Fixed {
        let Some(volume) = self.volumes.get_mut(substance) else {
            return Fixed::ZERO;
        };
        let taken = want.clamp(Fixed::ZERO, *volume);
        *volume -= taken;
        if *volume <= Fixed::ZERO {
            self.volumes.remove(substance);
        }
        taken
    }

    /// The value of a floor axis for one substance, read from the registry; an unregistered substance
    /// or one carrying no value on the axis reads as zero (the absence convention). This is the one
    /// registry read the whole derivation rests on, keyed by substance id and floor axis id alone.
    fn axis_value(reg: &PhysicsRegistry, substance: &str, axis: &str) -> Fixed {
        reg.substance(substance)
            .and_then(|s| s.vector.get(axis).copied())
            .unwrap_or(Fixed::ZERO)
    }

    /// The extensive volume-weighted sum of a floor axis over the mixture: sum of volume times the
    /// substance's axis value, overflow-saturating so a pathological volume cannot panic. For the
    /// density axis this is the cell's total matter MASS; the intensive per-volume form is [`bulk`].
    fn weighted_sum(&self, reg: &PhysicsRegistry, axis: &str) -> Fixed {
        Fixed::saturating_sum(self.volumes.iter().map(|(substance, volume)| {
            let value = SubstanceMix::axis_value(reg, substance, axis);
            volume.checked_mul(value).unwrap_or(Fixed::MAX)
        }))
    }

    /// The intensive volume-weighted mean of a floor axis over the mixture: the extensive
    /// [`weighted_sum`] divided by the total volume. An empty cell reads zero (the absence convention).
    /// This is the bulk property a contest works against (a bulk density, a bulk hardness); for a
    /// single-substance cell it is exactly that substance's own registry value.
    fn bulk(&self, reg: &PhysicsRegistry, axis: &str) -> Fixed {
        let total = self.total_volume();
        if total <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        self.weighted_sum(reg, axis)
            .checked_div(total)
            .unwrap_or(Fixed::ZERO)
    }

    /// The intensive volume-weighted mean of ANY named floor axis over the cell's mixture, for a reader
    /// outside this module. Modality-agnostic: it reads whatever axis the caller names, on any substance.
    /// Its first caller is the reach substrate ([`crate::perception_reach`]), which samples a cell's bulk
    /// absorption on a channel's own absorption axis along a signal's path (a strongly-absorbing cell
    /// occludes, an empty cell is transparent), but nothing here is specific to that axis or that use. An
    /// empty cell reads zero (the absence convention). Delegates to the internal bulk mean, so the
    /// derivation stays in one place.
    pub fn bulk_axis(&self, reg: &PhysicsRegistry, axis: &str) -> Fixed {
        self.bulk(reg, axis)
    }

    /// The total mass of matter in the cell: the sum over substances of volume times the substance's
    /// [`AXIS_DENSITY`], read from the registry. This is the load an extraction yields and a carry
    /// weighs (through `laws::weight`). Derived, never stored (Principle 11).
    pub fn mass(&self, reg: &PhysicsRegistry) -> Fixed {
        self.weighted_sum(reg, AXIS_DENSITY)
    }

    /// The load force this matter exerts under gravity: its [`mass`](SubstanceMix::mass) times the local
    /// gravitational acceleration, capped at the physics force ceiling
    /// ([`civsim_physics::laws::weight`]). This is the weight a carrier's grown strength is contested
    /// against when it lifts a load (material-substrate arc, cascade item 3): a being can take up matter
    /// whose weight its whole-body muscle force covers and no more, so the carry limit is grown strength
    /// versus derived weight, never a per-race carry table. The gravity is the world's reserved value,
    /// passed in (the same datum the buoyancy and weight physics already read).
    pub fn weight(&self, reg: &PhysicsRegistry, gravity: Fixed, force_max: Fixed) -> Fixed {
        laws::weight(self.mass(reg), gravity, force_max)
    }

    /// The cell's bulk density: the volume-weighted mean of [`AXIS_DENSITY`] over its mixture, read
    /// from the registry. An empty cell reads zero.
    pub fn bulk_density(&self, reg: &PhysicsRegistry) -> Fixed {
        self.bulk(reg, AXIS_DENSITY)
    }

    /// The cell's bulk indentation hardness: the volume-weighted mean of [`AXIS_HARDNESS`] over its
    /// mixture, read from the registry. This is the hardness an extraction force contests
    /// (`cut_penetrate` / `fracture_onset`); an empty cell reads zero.
    pub fn bulk_hardness(&self, reg: &PhysicsRegistry) -> Fixed {
        self.bulk(reg, AXIS_HARDNESS)
    }

    /// The greatest value of a floor axis over the mixture's constituents (the MAXIMUM, not the
    /// volume-weighted mean), read from the registry. An empty cell reads zero. This is the aggregation a
    /// FRACTURE-GATING property takes: a composite does not fracture until the force clears its STRONGEST
    /// load-bearing constituent, so the resistance is the hardest phase, never the average (see
    /// [`SubstanceMix::fracture_hardness`]).
    fn constituent_max(&self, reg: &PhysicsRegistry, axis: &str) -> Fixed {
        self.volumes
            .keys()
            .map(|substance| SubstanceMix::axis_value(reg, substance, axis))
            .fold(Fixed::ZERO, |acc, v| if v > acc { v } else { acc })
    }

    /// The cell's FRACTURE-GATING hardness: the GREATEST [`AXIS_FRACTURE`] among its constituents (the
    /// hardest load-bearing phase), read from the registry (material-substrate arc, cascade item 4). This
    /// is the resistance the extraction contest must clear to break any matter loose, and it is NOT the
    /// volume-weighted mean [`bulk_hardness`] uses: hardness does not average linearly, and a composite
    /// breaks at its strongest bond, so ore embedded in granite breaks at the granite, not at a blend of
    /// the two. A single-substance cell reads that substance's own fracture strength; a cell of a substance
    /// carrying no fracture datum reads zero (the absence convention: matter with no declared fracture
    /// resistance offers none). An empty cell reads zero.
    ///
    /// The honest limit: the strongest constituent is the proxy for the load-bearing matrix. Where the
    /// continuous phase that holds a composite together is not its hardest constituent, a matrix-phase
    /// datum would refine this; until a substance declares which phase is load-bearing, the hardest
    /// constituent is the defensible read and delivers the ore-in-rock outcome the physics needs.
    pub fn fracture_hardness(&self, reg: &PhysicsRegistry) -> Fixed {
        self.constituent_max(reg, AXIS_FRACTURE)
    }

    /// Fold the mixture into a hash in canonical (substance-id, volume) order. Defined for the wiring
    /// slice that folds the material layer into `state_hash`; nothing calls it on the run path yet, so
    /// declaring the substrate is hash-neutral. The `BTreeMap` walks in sorted id order, so the fold is
    /// reproducible and thread-invariant (the [`crate::locomotion::ResourceField::hash_into`]
    /// discipline).
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (substance, volume) in &self.volumes {
            for b in substance.as_bytes() {
                h.write_u32(*b as u32);
            }
            h.write_fixed(*volume);
        }
    }
}

/// The volume of matter a contact FORCE detaches from a cell in one extraction stroke (material-substrate
/// arc, cascade item 4, the extraction contest). It composes two mechanical floor laws, never a bespoke
/// rule: the being's `force` spread over its `working_area` is a contact pressure
/// ([`laws::contact_pressure`]); that pressure works against the cell's FRACTURE-gating hardness
/// ([`SubstanceMix::fracture_hardness`], the hardest constituent), and [`laws::cut_penetrate`] returns the
/// penetration depth, which is ZERO unless the pressure clears the hardness (the rock holds) and otherwise
/// sizes with the delivered work over the material's specific cutting energy; the detached volume is that
/// depth over the working area. So a being too weak to raise its pressure over the rock's fracture
/// strength mines nothing, a stronger one breaks the same rock, and a harder rock yields less to the same
/// force, all from physics against substance DATA, never a "miner" branch or a per-race yield table
/// (Principles 8, 9). A pure fixed-point read with no randomness, saturating rather than panicking on a
/// pathological product (the caller bounds the result by what the cell in fact holds). Every input is
/// derived or reserved-with-basis at the call site: `force` from the being's grown physiology, the areas
/// and the specific cutting energy from geometry and the substance floor, the caps from the physics floor.
#[allow(clippy::too_many_arguments)]
pub fn extraction_yield(
    force: Fixed,
    working_area: Fixed,
    fracture_hardness: Fixed,
    delivered_energy: Fixed,
    specific_cut_energy: Fixed,
    pressure_max: Fixed,
    depth_max: Fixed,
) -> Fixed {
    let pressure = laws::contact_pressure(force, working_area, pressure_max);
    let depth = laws::cut_penetrate(
        pressure,
        fracture_hardness,
        delivered_energy,
        specific_cut_energy,
        working_area,
        depth_max,
    );
    depth.checked_mul(working_area).unwrap_or(Fixed::MAX)
}

/// The reserved parameters of the extraction contest (material-substrate arc, cascade item 4). The
/// mechanism that reads them is fixed Rust; the working area is the owner's to set, surfaced with a basis,
/// never fabricated (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractionParams {
    /// The bearing AREA (m^2) a being's working surface presses against the rock, over which its grown
    /// force becomes a contact pressure ([`laws::contact_pressure`]). RESERVED. Basis: the contact patch of
    /// the body's working surface, a bare limb's tip or a hafted tool's edge, a body-and-tool geometry
    /// datum the anatomy arc will derive per-part; a smaller area concentrates the same force to a higher
    /// pressure (a pick bites where a fist cannot), so it sets how hard a rock a given strength can break.
    /// A performance-and-realism bound, surfaced for the owner, never invented.
    pub working_area: Fixed,
    /// The physics pressure ceiling the contact-pressure law saturates at (the mechanical floor's pressure
    /// axis maximum). A representability cap, not an authored quantity, mirroring the carry weight's force
    /// ceiling.
    pub pressure_max: Fixed,
}

impl ExtractionParams {
    /// A labelled DEVELOPMENT FIXTURE: a working area of 0.001 m^2 (a ~10 cm^2 working surface) and the
    /// pressure cap. Not owner canon; a stand-in so the extraction contest can run until the owner sets the
    /// working area against its basis. Under the calibrated profile the manifest supplies the set value and
    /// a scenario passes it in; the fixture keeps the fail-loud sentinel from blocking a dev run.
    pub fn dev_fixture() -> ExtractionParams {
        ExtractionParams {
            working_area: Fixed::from_ratio(1, 1000),
            pressure_max: Fixed::from_int(150_000),
        }
    }
}

/// The reserved parameters of the crafting contest (material-substrate arc, cascade item 4, knapping). The
/// mechanism that reads them is fixed Rust; the numbers are the owner's to set, surfaced with a basis,
/// never fabricated (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CraftParams {
    /// The VOLUME of carried matter a tool consumes to make (m^3). RESERVED. Basis: the material a shaped
    /// tool of the working size embodies; a being that carries less than this cannot make the tool. A
    /// geometry-and-scale datum, surfaced for the owner. The working-edge AREA is no longer carried here: it
    /// is DERIVED from the worked stone's own fracture strength under the being's forming force
    /// ([`crate::runner::Embodiment::craft_from_carried`] over [`civsim_physics::laws::edge_area_at`]), so a
    /// hard tough stone holds a fine edge and a soft one a blunt one, by physics not a reserved constant.
    pub tool_volume: Fixed,
    /// The characteristic LENGTH (m) of the shaped tool's body, the long dimension a being knaps (the
    /// tool-geometry expansion, root R2). RESERVED. Basis: the reach-scale length of a hand tool (a blade, a
    /// haft, a pick shaft), a geometry datum set from the being's reach scale like the wear stroke distance;
    /// with the retained volume it fixes the tool's body CROSS-SECTION (`volume / length`), so it decides
    /// whether the shaped tool is slender (weak in buckling and bending) or stout. Surfaced for the owner,
    /// never invented. A tool crafted with a non-positive length carries no body geometry and its
    /// geometry-reading failures are skipped.
    pub tool_length: Fixed,
}

impl CraftParams {
    /// A labelled DEVELOPMENT FIXTURE: a modest tool volume and a hand-tool length. Not owner canon; a stand-in
    /// so the crafting contest can run until the owner sets the values against their basis. The edge is
    /// derived, not fixtured.
    pub fn dev_fixture() -> CraftParams {
        CraftParams {
            tool_volume: Fixed::from_int(1),
            tool_length: Fixed::from_int(1),
        }
    }
}

/// The reserved parameters of tool WEAR (the made-world arc, tool-use, Section D). The mechanism (the Archard
/// wear law [`civsim_physics::laws::wear`] over the tool's own volume, its coefficient the tool material's
/// `mat.wear_coefficient` axis) is fixed Rust; these are the owner's numbers, surfaced with a basis, never
/// fabricated (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WearParams {
    /// The characteristic sliding DISTANCE (m) of one tool stroke, the length the working edge slides against
    /// the matter it works in a single use. RESERVED. Basis: the reach of one hand stroke a being makes with a
    /// hand tool (a knife draw, a scrape, a mining blow's slide), a kinematics datum set from the being's reach
    /// scale; the Archard wear volume is proportional to it. Surfaced for the owner, never invented.
    pub stroke_distance: Fixed,
    /// The physics worn-volume ceiling the wear law saturates at per use, a representability cap (not a
    /// behavioural quantity) that bounds a single stroke's material loss to the fixed-point range, mirroring
    /// the extraction and combustion ceilings.
    pub wear_max: Fixed,
}

impl WearParams {
    /// A labelled DEVELOPMENT FIXTURE: a hand-stroke sliding distance and the representability ceiling. Not
    /// owner canon; a stand-in so the wear step can run until the owner sets the stroke distance against its
    /// basis.
    pub fn dev_fixture() -> WearParams {
        WearParams {
            stroke_distance: Fixed::from_ratio(1, 10),
            wear_max: Fixed::from_int(1_000_000),
        }
    }
}

/// The reserved parameters of a percussion STRIKE (the made-world arc, tool-use, Section G). The mechanism is
/// fixed Rust: the acting being's greatest ACTUATOR WORK ([`civsim_physics::laws::actuator_work`], its strength
/// over its cross-section times its own grown stroke, `F d`) is the delivered energy, which fractures matter
/// whose Griffith energy the blow exceeds ([`civsim_physics::laws::fracture_onset`]'s energy limb). The
/// per-being swing speed the delivered energy once rode on is retired: `F`, the cross-section, and the stroke
/// are read from the acting part's own grown body, so no world-global swing speed remains (the stroke-rate
/// substrate). Only the representability ceiling stays a reserved number, surfaced with a basis, never
/// fabricated (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrikeParams {
    /// The physics energy ceiling the actuator-work law saturates at, a representability cap (not a
    /// behavioural quantity) bounding a single blow's delivered energy to the fixed-point range.
    pub energy_max: Fixed,
}

impl StrikeParams {
    /// A labelled DEVELOPMENT FIXTURE: the representability ceiling. Not owner canon; a stand-in so the strike
    /// step can run. The delivered energy is now derived from the acting part's own strength, cross-section, and
    /// grown stroke, so no swing-speed stand-in remains.
    pub fn dev_fixture() -> StrikeParams {
        StrikeParams {
            energy_max: Fixed::from_int(1_000_000),
        }
    }
}

/// The reserved parameters of the combustion contest (material-substrate arc, cascade item 6, live fire). The
/// mechanism that reads them is fixed Rust (the resolved combustion law over the fuel a cell holds); these
/// numbers are the owner's to set, surfaced with a basis, never fabricated (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombustionCalib {
    /// The fraction of a burning cell's fuel that combusts per tick, the reaction rate. RESERVED. Basis: the
    /// characteristic mass-loss rate of a burning fuel bed over the base tick (a fuel burns down over a
    /// timescale, not instantly), a reaction time constant set from the combustion literature's burn-rate
    /// data against the world's tick; the combustion law would otherwise consume all a cell's fuel in one
    /// tick. A larger fraction burns a fire out faster and hotter. A realism-and-rate bound, surfaced for
    /// the owner, never invented.
    pub burn_rate: Fixed,
    /// The physics released-energy ceiling the combustion law saturates at per cell per tick. A
    /// representability cap, not an authored quantity, mirroring the extraction pressure ceiling: it bounds a
    /// cell's per-tick energy release to the fixed-point range so a large fuel mass cannot overflow the field.
    pub energy_cap: Fixed,
    /// The fraction of the released combustion energy retained as SENSIBLE HEAT that raises the burning
    /// cell's temperature, the rest lost to radiation and carried off by the hot combustion gas. RESERVED.
    /// Basis: the combustion efficiency and the radiative-and-convective loss fraction of an open fire, so a
    /// real flame settles at its adiabatic-minus-losses temperature rather than the full adiabatic flame
    /// temperature the complete-combustion fuel value implies; a combustion-thermodynamics datum set from the
    /// flame-temperature literature. It sets how hot a fire runs and so how readily it spreads. A
    /// realism-and-coupling bound, surfaced for the owner, never invented.
    pub heat_fraction: Fixed,
    /// The oxidiser MASS a cell's medium supplies to a fire per unit respirable content per tick, the term
    /// that makes fire need air: the combustion law's oxidiser mass is this times the cell medium's
    /// respirable content, so an oxygen-demanding fuel burns in open air and starves in a sealed or anoxic
    /// space. RESERVED. Basis: the oxidiser mass the cell's air volume holds and replenishes at full
    /// respirable concentration over the base tick, an atmosphere-supply datum set from the cell scale and
    /// the air's oxygen content; a cell with no medium field reads full concentration (open atmosphere). A
    /// physics-and-supply bound, surfaced for the owner, never invented.
    pub oxidiser_supply: Fixed,
}

impl CombustionCalib {
    /// A labelled DEVELOPMENT FIXTURE: a burn-rate fraction, the energy cap, the heat-retention fraction, and
    /// the oxidiser supply. Not owner canon; a stand-in so the combustion beat can run until the owner sets
    /// the reserved values against their bases. The values exercise the mechanism (a hot fuel cell burns down
    /// over several ticks, stays hot, heats its neighbours enough to spread, and needs air) without standing
    /// for calibrated rates. The oxidiser supply is large, so open air is fuel-limited and only a near-anoxic
    /// medium starves the fire.
    pub fn dev_fixture() -> CombustionCalib {
        CombustionCalib {
            burn_rate: Fixed::from_ratio(1, 4),
            energy_cap: Fixed::from_int(1_000_000_000),
            heat_fraction: Fixed::from_ratio(1, 10),
            oxidiser_supply: Fixed::from_int(1_000_000_000),
        }
    }
}

/// The reserved parameter of shelter (material-substrate arc, cascade item 7): how strongly the insulating
/// matter enclosing a being attenuates its thermal exchange with the ambient field. The mechanism that reads
/// it is fixed Rust (the being's exchange rate divided by one plus the enclosing matter's thermal resistance
/// times this coupling); the number is the owner's to set, surfaced with a basis, never fabricated
/// (Principle 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShelterCalib {
    /// The coupling from the enclosing matter's derived thermal RESISTANCE (its volume over its conductivity,
    /// the barrier R-value per unit cell area) to the dimensionless attenuation of the being's convective
    /// exchange rate. RESERVED. Basis: the ratio of the barrier's conductive resistance to the being's
    /// convective (surface) resistance, the series-thermal-resistance form, so a roof of a given material and
    /// thickness slows the body-to-air coupling in proportion to how much better an insulator it is than open
    /// air; a heat-transfer datum set from the conduction-versus-convection balance. A larger coupling makes a
    /// given shelter buffer more. A physics-and-coupling bound, surfaced for the owner, never invented.
    pub insulation_coupling: Fixed,
}

impl ShelterCalib {
    /// A labelled DEVELOPMENT FIXTURE: an insulation coupling that exercises the mechanism (a roof of a
    /// low-conductivity material visibly buffers a being from a harsh field) without standing for a
    /// calibrated heat-transfer ratio. Not owner canon.
    pub fn dev_fixture() -> ShelterCalib {
        ShelterCalib {
            insulation_coupling: Fixed::from_ratio(1, 10),
        }
    }
}

/// The reserved parameters of the matter cycle (material-substrate arc, cascade item 8): how a cell's
/// organic matter decomposes over time. The mechanism that reads them is fixed Rust; these numbers are the
/// owner's to set, surfaced with a basis, never fabricated (Principle 11).
///
/// The decomposition BARRIER (the thermal gate) and the RATE are read per-substance from the substance's
/// own physics-floor axes (`bio.decomposition_barrier`, cited to the freezing point of the tissue water;
/// `bio.decomposition_rate`, the reserved per-substance timescale), so decomposition varies by the organic
/// matter's own physics rather than one global pair (the read-substance-physics direction). This calib
/// carries only the GLOBAL FALLBACK rate, used for an organic substance that does not yet declare its own
/// `bio.decomposition_rate`; the barrier has no global fallback, so a substance with no barrier axis does
/// not decompose (the barrier is the substance's physical gate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatterCycleCalib {
    /// The fallback fraction of a decomposing substance's volume that breaks down per tick, used when the
    /// substance carries no per-substance `bio.decomposition_rate`. RESERVED. Basis: the characteristic
    /// decomposition timescale of organic matter over the base tick (a carcass rots over a timescale, not
    /// instantly); a decomposition-ecology datum. A larger fraction rots matter faster. Superseded per
    /// substance once the owner sets each organic substance's own `bio.decomposition_rate`.
    pub decomposition_rate: Fixed,
    /// The soil-fertility scale (slice C2): the soil-nutrient supply a cell gains per unit of deposited
    /// decomposed mass, the coupling from the matter cycle's soil store to the productivity soil factor.
    /// RESERVED. Basis: the decomposed nutrient mass that raises a soil-limited cell's soil supply by one
    /// soil requirement (`productivity.soil_requirement`), so a carcass fully fertilises the ground it fell
    /// on; a soil-ecology coupling relating deposited nutrient mass to plant-available supply. Larger makes
    /// a given carcass fertilise more strongly. Only read where the matter cycle is armed, so it stays off
    /// the calibrated worldbuild path until a later slice wires the matter cycle onto it.
    pub fertility_scale: Fixed,
}

impl MatterCycleCalib {
    /// A labelled DEVELOPMENT FIXTURE: a fallback rate and a fertility scale that exercise the mechanism (a
    /// warm carcass visibly rots over several ticks and enriches the ground it fell on) without standing for
    /// calibrated values. The barrier is the substance's own (`bio.decomposition_barrier`), no longer a
    /// fixture field. Not owner canon.
    pub fn dev_fixture() -> MatterCycleCalib {
        MatterCycleCalib {
            decomposition_rate: Fixed::from_ratio(1, 10),
            fertility_scale: Fixed::from_ratio(1, 1000),
        }
    }
}

/// A worked object a being wields as a tool (material-substrate arc, cascade item 4, crafting). This is the
/// CONTEST INTERFACE of a tool, the two things an extraction or cut reads: its working GEOMETRY (the
/// contact area its edge or point presses over) and its MATERIAL (the substance it is made of, whose
/// hardness the registry supplies). A crafted tool derived from a `FormDef` geometry and a `Substance`
/// populates exactly these, so this struct is the forward-compatible view a tool presents to a contest,
/// never a closed tool catalog: a sharper edge is a smaller `contact_area`, a harder tool a harder
/// `substance`, and the physics does the rest (a small hard point concentrates the same force into a higher
/// pressure and breaks harder rock, a soft one blunts before it reaches that pressure). The mass and
/// hardness are DERIVED from the registry by the substance id, never stored (Principle 11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WieldedTool {
    /// The contact area (m^2) the tool's working surface presses over, its intrinsic geometry: a smaller
    /// area concentrates the being's force into a higher pressure.
    pub contact_area: Fixed,
    /// The VOLUME of matter the tool retains (m^3), the extensive quantity the registry's intensive axes
    /// cannot supply, set when the tool is shaped from the stock it consumed. It makes the tool's MASS
    /// recoverable ([`WieldedTool::mass`], `mat.density` times this volume, the extensive datum an impact or a
    /// swing reads) and is the stock a wear step decrements and a re-work reshapes, so a tool that loses matter
    /// grows blunt and eventually spends out. Without it the struct's own claim that mass is derived is empty.
    pub volume: Fixed,
    /// The tool's characteristic LENGTH (m), the long dimension of its shaped body, set when it is shaped from
    /// the stock (the made-world arc, tool-use, the tool-geometry expansion, root R2). It is the lever arm a
    /// pry reads, the bending span a sideways load reads, and (with the retained volume) it fixes the tool's
    /// BODY CROSS-SECTION ([`WieldedTool::cross_section`], `volume / length` for a prism), the area a
    /// transverse crack runs through and the section that buckles under an axial load. So a long thin tool is
    /// slender (a small cross-section, weak in buckling and bending) and a short thick one is stout, the
    /// geometry tradeoff that the tool's material choice trades against. Zero-safe: a tool shaped with no
    /// length carries no body geometry, so the geometry-reading failures skip it (the absence convention).
    pub length: Fixed,
    /// The substance the tool is made of; its hardness (the pressure it sustains before it blunts) and its
    /// other properties are read from the [`PhysicsRegistry`] by this id.
    pub substance: String,
}

impl WieldedTool {
    /// The tool's mass (kg): its substance's `mat.density` (an intensive axis in the registry) times its
    /// retained [`WieldedTool::volume`] (the extensive quantity the registry cannot supply), the datum a swing
    /// or an impact reads. Zero when the substance declares no density (the absence convention). Derived, never
    /// stored (Principle 11).
    pub fn mass(&self, reg: &PhysicsRegistry) -> Fixed {
        let density = reg
            .substance(&self.substance)
            .and_then(|s| s.vector.get("mat.density").copied())
            .unwrap_or(Fixed::ZERO);
        // Checked, like every mechanics law: a dense stone times a large volume can overflow Q32.32, and an
        // unchecked multiply would panic in debug and wrap in release, a cross-build determinism hazard. Both
        // density and volume are non-negative physical quantities, so an overflowing mass saturates to the
        // fixed-point maximum rather than wrapping.
        density.checked_mul(self.volume).unwrap_or(Fixed::MAX)
    }

    /// The tool's BODY cross-section (m^2): the retained volume over the characteristic length, the prism
    /// relation `A = V / L`. This is the area a transverse crack runs through and the section that resists an
    /// axial or a buckling load, derived from the two extensive geometry data the tool carries, no cube root
    /// and no shape catalog. Zero when the tool has no length (the absence convention: a tool with no body
    /// geometry has no derivable cross-section, so the geometry-reading failures skip it).
    pub fn cross_section(&self) -> Fixed {
        if self.length <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        self.volume.checked_div(self.length).unwrap_or(Fixed::ZERO)
    }

    /// Fold the tool into a hash, its geometry, its retained volume and length, then its substance id, in
    /// canonical order (the material fold discipline). Called by the runner's per-walker `state_hash` fold when
    /// a being wields a tool; a being with no tool folds nothing, so the wielded slot is opt-in and hash-neutral
    /// by default.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_fixed(self.contact_area);
        h.write_fixed(self.volume);
        h.write_fixed(self.length);
        for b in self.substance.as_bytes() {
            h.write_u32(*b as u32);
        }
    }
}

/// The matter that sits in each z-cell of the world, the ground truth of what the world is made of: a
/// per-cell [`SubstanceMix`] keyed by [`Coord3`], a Coord3-keyed sibling of
/// [`crate::locomotion::ResourceField`] of the same sparse shape. A cell with no entry is void (no
/// matter). The mechanical properties of a cell are DERIVED by reading a [`PhysicsRegistry`], never
/// stored, so the mechanical floor's authored data is the single source of a substance's density and
/// hardness (Principle 11), and the world never hardcodes what a tile is made of: worldgen fills the
/// column from substance rows, and every downstream contest reads the registry.
#[derive(Clone, Debug, Default)]
pub struct MaterialField {
    matter: BTreeMap<Coord3, SubstanceMix>,
}

impl MaterialField {
    /// An empty field: a world with no material layer declared (every cell void).
    pub fn new() -> MaterialField {
        MaterialField::default()
    }

    /// Whether the field holds no matter at any cell: the opt-out state a scenario that declares no
    /// material layer stays in, so its `state_hash` fold (when wired) folds nothing and it replays
    /// bit-for-bit.
    pub fn is_empty(&self) -> bool {
        self.matter.is_empty()
    }

    /// Set the whole substance mixture on a cell (overwriting any prior). An empty mixture clears the
    /// cell back to void.
    pub fn set_cell(&mut self, coord: Coord3, mix: SubstanceMix) {
        if mix.is_empty() {
            self.matter.remove(&coord);
        } else {
            self.matter.insert(coord, mix);
        }
    }

    /// The substance mixture on a cell, if the cell holds any matter.
    pub fn cell(&self, coord: Coord3) -> Option<&SubstanceMix> {
        self.matter.get(&coord)
    }

    /// Every cell that holds matter, in canonical [`Coord3`] order, for a field process that reads the whole
    /// substrate (the combustion beat scanning for combustible cells, cascade item 6). A pure read; the
    /// `BTreeMap` walk is canonical, so a process built on it is reproducible and thread-invariant.
    pub fn cells(&self) -> impl Iterator<Item = (&Coord3, &SubstanceMix)> + '_ {
        self.matter.iter()
    }

    /// The total mass of all located matter over every cell, derived from the registry (the material term of
    /// the matter-cycle conservation ledger, chemistry arc Arc 4): what decomposition moves OUT of the located
    /// substances and into the soil store. Saturating; an empty field reads zero.
    pub fn total_mass(&self, reg: &PhysicsRegistry) -> Fixed {
        Fixed::saturating_sum(self.matter.values().map(|mix| mix.mass(reg)))
    }

    /// Add a volume of one substance to a cell (a deposit: mined spoil dropped, a corpse's remains, an
    /// ash residue). Creates the cell's mixture if it was void.
    pub fn deposit(&mut self, coord: Coord3, substance: &str, volume: Fixed) {
        if volume <= Fixed::ZERO {
            return;
        }
        self.matter.entry(coord).or_default().add(substance, volume);
    }

    /// Remove up to `want` volume of one substance from a cell, returning what was removed. A cell left
    /// empty is pruned to void so the canonical walk stays minimal. An absent cell or substance is a
    /// no-op returning zero.
    pub fn take(&mut self, coord: Coord3, substance: &str, want: Fixed) -> Fixed {
        let Some(mix) = self.matter.get_mut(&coord) else {
            return Fixed::ZERO;
        };
        let taken = mix.take(substance, want);
        if mix.is_empty() {
            self.matter.remove(&coord);
        }
        taken
    }

    /// The volume of one substance on a cell; an absent cell or substance reads as zero.
    pub fn volume(&self, coord: Coord3, substance: &str) -> Fixed {
        self.matter
            .get(&coord)
            .map(|mix| mix.volume(substance))
            .unwrap_or(Fixed::ZERO)
    }

    /// The total mass of matter on a cell, derived from the registry; an empty cell reads zero.
    pub fn mass(&self, coord: Coord3, reg: &PhysicsRegistry) -> Fixed {
        self.matter
            .get(&coord)
            .map(|mix| mix.mass(reg))
            .unwrap_or(Fixed::ZERO)
    }

    /// The bulk density of a cell's matter, derived from the registry; an empty cell reads zero.
    pub fn bulk_density(&self, coord: Coord3, reg: &PhysicsRegistry) -> Fixed {
        self.matter
            .get(&coord)
            .map(|mix| mix.bulk_density(reg))
            .unwrap_or(Fixed::ZERO)
    }

    /// The bulk indentation hardness of a cell's matter, derived from the registry: what an extraction
    /// force contests. An empty cell reads zero (no matter to break).
    pub fn bulk_hardness(&self, coord: Coord3, reg: &PhysicsRegistry) -> Fixed {
        self.matter
            .get(&coord)
            .map(|mix| mix.bulk_hardness(reg))
            .unwrap_or(Fixed::ZERO)
    }

    /// The FRACTURE-GATING hardness of a cell's matter, derived from the registry: the greatest fracture
    /// strength among its constituents, the resistance an extraction contest must clear to break matter
    /// loose (material-substrate arc, cascade item 4). This is the hardest load-bearing phase, not the
    /// bulk mean [`bulk_hardness`] takes, so ore in rock breaks at the rock (see
    /// [`SubstanceMix::fracture_hardness`]). An empty cell reads zero (no matter to break).
    pub fn fracture_hardness(&self, coord: Coord3, reg: &PhysicsRegistry) -> Fixed {
        self.matter
            .get(&coord)
            .map(|mix| mix.fracture_hardness(reg))
            .unwrap_or(Fixed::ZERO)
    }

    /// Fold the material layer into a hash in canonical (Coord3, substance-id, volume) order. Defined
    /// for the wiring slice that folds it into the runner's `state_hash` beside
    /// [`crate::locomotion::ResourceField::hash_into`]; nothing calls it on the run path yet, so the
    /// substrate is hash-neutral until it is wired. The `BTreeMap`s walk in canonical key order, so the
    /// fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (coord, mix) in &self.matter {
            h.write_i64(coord.x as i64);
            h.write_i64(coord.y as i64);
            h.write_i64(coord.z as i64);
            mix.hash_into(h);
        }
    }
}

/// The per-column EARTHWORK DELTA: how far a being's digging and depositing has moved a surface column's
/// elevation from the worldgen baseline (material-substrate arc, cascade item 5, modifiable terrain). A
/// sparse field keyed by the ground column (a [`Coord3`] at z zero), empty by default (no cell reworked),
/// a sibling of [`MaterialField`]. The physics reads the EFFECTIVE elevation as the worldgen elevation plus
/// the delta, so a dug pit (a negative delta) pools water and a raised mound (a positive delta) sheds it,
/// the hydrology's downhill target recomputing from the effective elevation rather than a one-time
/// worldgen precompute. Digging is a fracture contest over the ground's derived hardness (the extraction
/// contest, reused), the removed matter conserved as a carried load and set down elsewhere as a mound; this
/// field is the elevation bookkeeping that lets the dig and the deposit reshape the terrain, not only move
/// matter between a cell and a carrier. Off the run path until the dig affordance wires it, so declaring it
/// leaves every scenario byte-identical (the opt-in empty-default pattern).
///
/// Two SOURCES accumulate onto the same effective elevation, kept in separate maps so a being can dig into
/// geologically-lifted crust and each source stays legible: the being EARTHWORK delta (dig and mound, the
/// original source), and the GEOLOGICAL delta (seed crust plus isostatic relaxation, and later the interior
/// uplift read, the genesis-forward Stage-3 surface lane). The physics reads the sum ([`Self::total_delta`]);
/// both maps are empty by default, so a world that arms neither is byte-identical to the pre-ledger baseline
/// (the opt-in empty-default pattern holds for the added geological source too).
#[derive(Clone, Debug, Default)]
pub struct EarthworkField {
    /// The being-earthwork elevation delta at each reworked column, keyed by its ground [`Coord3`] (z zero). A
    /// column not present reads a zero delta (the absence convention). A column driven back to a zero delta is
    /// pruned, so the canonical walk stays minimal.
    deltas: BTreeMap<Coord3, Fixed>,
    /// The GEOLOGICAL elevation delta at each column (seed crust, isostatic relaxation, and the interior
    /// uplift read), keyed by its ground [`Coord3`]. Empty by default and off the run path until a scenario
    /// arms the geology, so declaring it leaves every scenario byte-identical. The genesis-forward Stage-3
    /// surface source writes it; the physics reads it summed with the being delta.
    geological: BTreeMap<Coord3, Fixed>,
}

impl EarthworkField {
    /// An empty field: worldgen terrain everywhere, nothing dug or mounded.
    pub fn new() -> EarthworkField {
        EarthworkField::default()
    }

    /// Whether no column has been reworked by EITHER source (being earthwork or geology), the opt-out state a
    /// scenario that arms neither stays in, so its `state_hash` fold folds nothing and it replays bit-for-bit.
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty() && self.geological.is_empty()
    }

    /// The BEING-earthwork elevation delta at a column (positive a mound, negative a pit); an unreworked
    /// column reads zero. The column is the ground [`Coord3`] (`Coord3::ground(x, y)`); the caller passes the
    /// surface column. This is the dig-and-mound source alone; [`Self::total_delta`] adds the geological one.
    pub fn delta(&self, column: Coord3) -> Fixed {
        self.deltas.get(&column).copied().unwrap_or(Fixed::ZERO)
    }

    /// The GEOLOGICAL elevation delta at a column (seed crust, isostatic relaxation, the interior uplift read);
    /// an unlifted column reads zero. The genesis-forward Stage-3 surface source writes this.
    pub fn geological_delta(&self, column: Coord3) -> Fixed {
        self.geological.get(&column).copied().unwrap_or(Fixed::ZERO)
    }

    /// The EFFECTIVE elevation delta the physics reads: the being earthwork plus the geological delta, so a
    /// dug pit in lifted crust reads both. Saturating so an extreme sum stays representable. When the geology
    /// is unarmed (the geological map empty) this equals [`Self::delta`], so the read is byte-neutral.
    pub fn total_delta(&self, column: Coord3) -> Fixed {
        self.delta(column)
            .saturating_add(self.geological_delta(column))
    }

    /// Move a column's elevation by `change` (negative to dig down, positive to mound up), accumulating onto
    /// any prior rework. A column driven back to zero is pruned to keep the canonical walk minimal. A pure
    /// deterministic bookkeeping write with no randomness, the earthwork sibling of
    /// [`MaterialField::deposit`].
    pub fn adjust(&mut self, column: Coord3, change: Fixed) {
        if change == Fixed::ZERO {
            return;
        }
        let entry = self.deltas.entry(column).or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(change);
        if *entry == Fixed::ZERO {
            self.deltas.remove(&column);
        }
    }

    /// Move a column's GEOLOGICAL elevation by `change` (negative subsidence, positive uplift), accumulating
    /// onto any prior geological rework. A column driven back to zero is pruned to keep the canonical walk
    /// minimal. The genesis-forward Stage-3 surface source (seed crust, isostatic relaxation, the interior
    /// uplift read) writes through this, the geological sibling of [`Self::adjust`]; a pure deterministic
    /// bookkeeping write with no randomness.
    pub fn adjust_geological(&mut self, column: Coord3, change: Fixed) {
        if change == Fixed::ZERO {
            return;
        }
        let entry = self.geological.entry(column).or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(change);
        if *entry == Fixed::ZERO {
            self.geological.remove(&column);
        }
    }

    /// Fold the earthwork into a hash for the runner's `state_hash` beside [`MaterialField::hash_into`]. Both
    /// sources fold in canonical key order, the being deltas then the geological deltas, each entry written
    /// without a length prefix so an EMPTY map folds nothing: a scenario that arms neither source is
    /// hash-unchanged, and one that arms only the being earthwork is identical to before the geological source
    /// was added. The `BTreeMap` walks in canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (column, delta) in &self.deltas {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(*delta);
        }
        for (column, delta) in &self.geological {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(*delta);
        }
    }
}

/// The per-column geodynamic interface state lives in the SHARED [`civsim_physics::geodynamics`] contract, so
/// the surface elevation-ledger lane (here) and the interior convection lane (the geology floor) import the
/// same typed boundary rather than a private copy; re-exported here for the resident field below.
pub use civsim_physics::geodynamics::GeodynamicColumn;

/// The sparse per-column [`GeodynamicColumn`] field, the resident interface between the interior and surface
/// geodynamics lanes. Empty by default and off the run path until a genesis pass arms the geology, so
/// declaring it leaves every scenario byte-identical (the opt-in empty-default pattern, the sibling of
/// [`EarthworkField`]). A column not present reads the zero default.
#[derive(Clone, Debug, Default)]
pub struct GeodynamicField {
    columns: BTreeMap<Coord3, GeodynamicColumn>,
}

impl GeodynamicField {
    /// An empty field: no column carries geodynamic state.
    pub fn new() -> GeodynamicField {
        GeodynamicField::default()
    }

    /// Whether no column carries geodynamic state (the opt-out state a scenario that arms no geology stays in,
    /// so its `state_hash` fold folds nothing and it replays bit-for-bit).
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// The geodynamic state at a column; an unset column reads the zero default (the absence convention). The
    /// column is the ground [`Coord3`] (`Coord3::ground(x, y)`).
    pub fn get(&self, column: Coord3) -> GeodynamicColumn {
        self.columns.get(&column).copied().unwrap_or_default()
    }

    /// Walk the columns that carry geodynamic state, in canonical [`Coord3`] key order (the `BTreeMap` walk),
    /// so a consumer that folds over them (the surface isostatic relaxation) is reproducible and
    /// thread-invariant. An empty field yields nothing, so a consumer over an unarmed geology does no work and
    /// stays byte-neutral.
    pub fn iter(&self) -> impl Iterator<Item = (Coord3, GeodynamicColumn)> + '_ {
        self.columns.iter().map(|(coord, state)| (*coord, *state))
    }

    /// Set a column's geodynamic state. An all-zero state is pruned to keep the canonical walk minimal, so a
    /// column driven back to the default drops out (the same discipline as the earthwork prune).
    pub fn set(&mut self, column: Coord3, state: GeodynamicColumn) {
        if state == GeodynamicColumn::default() {
            self.columns.remove(&column);
        } else {
            self.columns.insert(column, state);
        }
    }

    /// Fold the field into a hash beside [`EarthworkField::hash_into`], each entry written without a length
    /// prefix so an EMPTY field folds nothing (an unarmed geology is hash-unchanged). The `BTreeMap` walks in
    /// canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (column, state) in &self.columns {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(state.crustal_density);
            h.write_fixed(state.crustal_thickness);
            h.write_fixed(state.isostatic_elevation);
        }
    }
}

/// The per-cell FIRE INTENSITY (material-substrate arc, cascade item 6, LIVE FIRE): the combustion energy a
/// cell releases this tick, keyed by its [`Coord3`], sparse over the burning cells. A cell holding a
/// combustible substance (a substance carrying `therm.fuel_value`) that stands at or above its
/// `therm.ignition_temperature` combusts through the resolved combustion law ([`civsim_physics::laws::combustion`]),
/// consuming a bounded fraction of its fuel and releasing that fuel's chemical energy, which this field
/// records so the world can read where and how hard it is burning (the render tint, the harm dose, the heat
/// the later slice injects into the temperature field so fire spreads). It is RECOMPUTED each tick, not
/// accumulated: a cell that runs out of fuel or cools below its ignition temperature drops out, so the field
/// always reflects the current combustion. Off the run path until the combustion beat sources it, so
/// declaring it leaves every scenario byte-identical (the opt-in empty-default pattern, the sibling of
/// [`EarthworkField`]).
#[derive(Clone, Debug, Default)]
pub struct FireField {
    /// The released combustion energy at each burning cell this tick, keyed by its [`Coord3`]. A cell not
    /// present is not burning (intensity zero, the absence convention). A cell whose intensity falls to zero
    /// is pruned, so the canonical walk stays minimal.
    intensities: BTreeMap<Coord3, Fixed>,
}

impl FireField {
    /// An unlit field: nothing burning anywhere.
    pub fn new() -> FireField {
        FireField::default()
    }

    /// Whether nothing is burning (the opt-out state a scenario that lights no fire stays in, so its
    /// `state_hash` fold folds nothing and it replays bit-for-bit).
    pub fn is_empty(&self) -> bool {
        self.intensities.is_empty()
    }

    /// The fire intensity at a cell (the released combustion energy this tick); an unlit cell reads zero.
    pub fn intensity(&self, cell: Coord3) -> Fixed {
        self.intensities.get(&cell).copied().unwrap_or(Fixed::ZERO)
    }

    /// Set a cell's fire intensity to this tick's released combustion energy. A zero (a cell that stopped
    /// burning) is pruned to keep the canonical walk minimal. A pure deterministic write with no randomness;
    /// the combustion beat rebuilds the field from an empty one each tick, so no stale entry survives.
    pub fn set(&mut self, cell: Coord3, intensity: Fixed) {
        if intensity <= Fixed::ZERO {
            self.intensities.remove(&cell);
        } else {
            self.intensities.insert(cell, intensity);
        }
    }

    /// Fold the fire field into a hash in canonical (cell, intensity) order, for the runner's `state_hash`
    /// beside [`EarthworkField::hash_into`]. An empty (unlit) field folds nothing, so an opted-out run is
    /// unchanged. The `BTreeMap` walks in canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (cell, intensity) in &self.intensities {
            h.write_i64(cell.x as i64);
            h.write_i64(cell.y as i64);
            h.write_i64(cell.z as i64);
            h.write_fixed(*intensity);
        }
    }
}

/// The per-cell SOIL NUTRIENT store the matter cycle deposits decomposed matter into (material-substrate
/// arc, cascade item 8, slice C, the re-materialisation). When a cell's organic matter decomposes, its lost
/// mass does not vanish to a scalar sink: it re-materialises HERE, located at the cell and split into
/// nutrient CLASSES by the decomposing substance's own composition (its mineral-ash fraction to a mineral
/// class, the remainder to an organic class), so the ground where a carcass rots is enriched. Mass-valued
/// (not volume), so a deposit is EXACT: no volume-quantisation rounding, and the split conserves the
/// decomposed mass bit for bit, because the organic share is the remainder after the mineral share (mineral
/// plus organic equals the loss exactly, whatever the mineral multiply rounds to). The productivity
/// derivation reads a cell's total nutrient mass as its soil fertility (slice C2), closing the matter cycle
/// into the food web. Off the run path until the matter cycle deposits into it, so declaring it leaves every
/// scenario byte-identical (the opt-in empty-default pattern, the sibling of [`FireField`]).
#[derive(Clone, Debug, Default)]
pub struct SoilNutrientField {
    /// The accumulated nutrient mass at each cell, keyed by [`Coord3`] then by nutrient-class id (the source
    /// composition axis for the mineral share, a residual id for the organic share). A cell or class not
    /// present holds no nutrient (the absence convention).
    cells: BTreeMap<Coord3, BTreeMap<String, Fixed>>,
}

impl SoilNutrientField {
    /// A barren store: no cell holds any deposited nutrient.
    pub fn new() -> SoilNutrientField {
        SoilNutrientField::default()
    }

    /// Whether nothing has been deposited anywhere (the opt-out state a scenario with no matter cycle stays
    /// in, so its `state_hash` fold folds nothing and it replays bit-for-bit).
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Deposit nutrient mass of a class at a cell, accumulating with what is already there. A non-positive
    /// mass is a no-op, so no empty entry is created and an unused store stays empty.
    pub fn deposit(&mut self, cell: Coord3, class: &str, mass: Fixed) {
        if mass <= Fixed::ZERO {
            return;
        }
        let entry = self
            .cells
            .entry(cell)
            .or_default()
            .entry(class.to_string())
            .or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(mass);
    }

    /// Remove up to `want` of a class's nutrient mass at a cell (the extract-and-deplete sibling of
    /// [`Self::deposit`]): the draw a producer makes on the located soil-nutrient store as it fixes biomass,
    /// so soil is finite and a heavily-worked column depletes rather than reading an infinite well (the
    /// closed nutrient cycle). Clamped to the present mass, never negative; the class entry is removed when
    /// it reaches zero and an emptied cell is dropped, so a drawn-then-emptied cell is byte-identical to a
    /// never-deposited one and the `hash_into` fold is unperturbed. Returns the mass actually removed. Reads
    /// and writes only the class string's mass, never an identity (Principle 9), so the drawn class is
    /// whatever the world's chemistry declares, not an authored nutrient.
    pub fn take(&mut self, cell: Coord3, class: &str, want: Fixed) -> Fixed {
        let Some(classes) = self.cells.get_mut(&cell) else {
            return Fixed::ZERO;
        };
        let Some(have) = classes.get_mut(class) else {
            return Fixed::ZERO;
        };
        let taken = want.clamp(Fixed::ZERO, *have);
        *have -= taken;
        if *have <= Fixed::ZERO {
            classes.remove(class);
            if classes.is_empty() {
                self.cells.remove(&cell);
            }
        }
        taken
    }

    /// The nutrient mass of a class at a cell; an unenriched cell or an absent class reads zero.
    pub fn mass(&self, cell: Coord3, class: &str) -> Fixed {
        self.cells
            .get(&cell)
            .and_then(|m| m.get(class))
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// The total nutrient mass at a cell, summed over its classes (the soil fertility the productivity
    /// derivation reads). An unenriched cell reads zero. Saturating, so an over-enriched cell caps rather
    /// than wrapping.
    pub fn cell_total(&self, cell: Coord3) -> Fixed {
        self.cells
            .get(&cell)
            .map(|m| {
                m.values()
                    .fold(Fixed::ZERO, |acc, v| acc.saturating_add(*v))
            })
            .unwrap_or(Fixed::ZERO)
    }

    /// The total deposited nutrient mass over every cell and class (the matter the decomposition has moved
    /// out of located substances and into the soil, the sink side of the conservation the
    /// [`crate::conservation::ConservationRegistry`] guards). Saturating.
    pub fn total(&self) -> Fixed {
        self.cells.values().fold(Fixed::ZERO, |acc, m| {
            m.values().fold(acc, |a, v| a.saturating_add(*v))
        })
    }

    /// Each enriched cell with its total nutrient mass, in canonical [`Coord3`] order (the productivity
    /// fertility read walks this to fill its per-cell soil supply). An empty store yields nothing.
    pub fn cell_totals(&self) -> impl Iterator<Item = (Coord3, Fixed)> + '_ {
        self.cells.iter().map(|(cell, classes)| {
            let total = classes
                .values()
                .fold(Fixed::ZERO, |acc, v| acc.saturating_add(*v));
            (*cell, total)
        })
    }

    /// Fold the soil store into a hash in canonical (cell, class, mass) order, beside
    /// [`FireField::hash_into`]. An empty store folds nothing, so an opted-out run is unchanged. The
    /// `BTreeMap`s walk in canonical key order, so the fold is reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (cell, classes) in &self.cells {
            for (class, mass) in classes {
                h.write_i64(cell.x as i64);
                h.write_i64(cell.y as i64);
                h.write_i64(cell.z as i64);
                for b in class.as_bytes() {
                    h.write_u32(*b as u32);
                }
                h.write_fixed(*mass);
            }
        }
    }
}

/// A data-defined MASS-CONSTITUENT registry (Principle 11): how the decomposed mass of a substance or a body
/// is apportioned into located soil-nutrient CLASSES by the decomposing matter's OWN composition, generalizing
/// the fixed ash-plus-organic pair to k=N constituents. Each constituent names a FRACTION AXIS (a value in
/// [0, 1] on the matter's composition giving the share of the decomposed mass that is that constituent) and the
/// soil CLASS it deposits into; whatever the constituents do not claim is the RESIDUAL, deposited to the
/// residual class, so the split is mass-EXACT (the constituent shares plus the residual equal the whole loss
/// bit for bit, the residual absorbing every fixed-point rounding). The membership is DATA: a world adds a
/// constituent by adding a row, so the split never authors a closed Earth ash-and-humus pair in the
/// decomposition hot path. Read against the decomposing matter's own composition (a substance's `vector` or a
/// decayed body parcel's composition), so the SAME split serves the material leg (rock, carrion) and the
/// tissue leg (a decayed body), symmetric: a body re-materialises into the soil by ITS OWN axes, not a
/// hardcoded organic bucket.
#[derive(Clone, Debug)]
pub struct ConstituentRegistry {
    constituents: Vec<Constituent>,
    residual_class: String,
}

/// One mass constituent: the [0, 1] fraction axis on the decomposing matter's composition and the soil class
/// its share deposits into.
#[derive(Clone, Debug)]
struct Constituent {
    fraction_axis: String,
    deposit_class: String,
}

impl ConstituentRegistry {
    /// An empty registry whose whole loss goes to `residual_class` (the k=1 case). Push constituents to carve
    /// fraction axes out of the residual.
    pub fn new(residual_class: &str) -> ConstituentRegistry {
        ConstituentRegistry {
            constituents: Vec::new(),
            residual_class: residual_class.to_string(),
        }
    }

    /// Add a constituent: the [0, 1] `fraction_axis` on the decomposing matter's composition gives the share
    /// of its decomposed mass that is this constituent, deposited to `deposit_class`. Push order fixes only
    /// the canonical deposit sequence (the residual always follows), never the mass total.
    pub fn push(&mut self, fraction_axis: &str, deposit_class: &str) {
        self.constituents.push(Constituent {
            fraction_axis: fraction_axis.to_string(),
            deposit_class: deposit_class.to_string(),
        });
    }

    /// The UNCONFIGURED default split (the opt-in sibling of the decomposer's `None` -> unconditional-rate
    /// default): the decomposing matter's own mineral-ash fraction to a mineral class, the remainder to an
    /// organic class, the pre-chemistry-arc behaviour exactly. Named after Earth axes only as a labelled
    /// default a world overrides by arming its own [`ConstituentRegistry`]; the mechanism reads the matter's
    /// OWN composition, never a per-species table (residue F1: a Terran-clean world names its own axes here).
    pub fn terran_default() -> ConstituentRegistry {
        let mut r = ConstituentRegistry::new("bio.organic_residue");
        r.push("bio.mineral_ash_fraction", "bio.mineral_ash_fraction");
        r
    }

    /// Apportion `mass` into (deposit_class, mass) shares by the decomposing matter's composition (`axis`, a
    /// lookup returning the matter's value on a named floor axis, zero for an absent one). Each constituent
    /// claims `min(remaining, mass * fraction)`, the residual takes the final remaining, so the shares sum to
    /// `mass` EXACTLY (the residual absorbs all fixed-point rounding). A non-positive mass or a zero share is
    /// dropped (no empty deposit, so an armed-but-unmatched body still lands its whole mass in the residual).
    /// Canonical: the constituents in push order, then the residual, so the deposit sequence is reproducible
    /// and thread-invariant.
    pub fn split(&self, mass: Fixed, axis: impl Fn(&str) -> Fixed) -> Vec<(String, Fixed)> {
        if mass <= Fixed::ZERO {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut remaining = mass;
        for c in &self.constituents {
            if remaining <= Fixed::ZERO {
                break;
            }
            let frac = axis(&c.fraction_axis);
            if frac <= Fixed::ZERO {
                continue;
            }
            let share = mass.checked_mul(frac).unwrap_or(Fixed::ZERO).min(remaining);
            if share <= Fixed::ZERO {
                continue;
            }
            out.push((c.deposit_class.clone(), share));
            remaining -= share;
        }
        if remaining > Fixed::ZERO {
            out.push((self.residual_class.clone(), remaining));
        }
        out
    }
}

/// A TISSUE PARCEL: a quantity of an organism's own MATTER, deposited where it fell, carried as a raw
/// COMPOSITION VECTOR (its value on each biology and mechanical floor axis, keyed by axis id) rather than a
/// registered [`civsim_physics::Substance`] id. This is how a generated organism becomes located, usable
/// matter without minting a per-species substance or authoring a species-to-substance map (Principle 8): the
/// parcel's physics IS its own body's composition, derived by a development-weighted fold over its body plan
/// ([`crate::physiology::whole_body_composition_vector`]), and every consumer reads a named axis off the
/// vector exactly as it reads one off a substance, with the same absence-is-zero convention. The composition
/// shares the string-keyed sorted-walk shape of [`crate::anatomy::TissueComposition`] and
/// [`civsim_physics::Substance`]'s `vector`, so a consumer that reads `mat.fracture_strength` or
/// `bio.decomposition_barrier` off a substance reads it off a parcel unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TissueParcel {
    /// The located volume of this matter.
    pub volume: Fixed,
    /// The parcel's value on each floor axis it carries, keyed by axis id (an absent axis is zero).
    pub composition: BTreeMap<String, Fixed>,
}

impl TissueParcel {
    /// The parcel's value on a named floor axis; an axis it does not carry reads zero (the absence
    /// convention the material substrate uses throughout).
    pub fn axis(&self, id: &str) -> Fixed {
        self.composition.get(id).copied().unwrap_or(Fixed::ZERO)
    }
}

/// A parcel's CONTENT IDENTITY: its composition as sorted `(axis, value)` pairs. Two parcels share a key
/// exactly when their compositions are byte-identical, so the key IS the content (collision-free), never a
/// hash of it. Derived from an organism's own body, never authored, so two byte-identical corpses accumulate
/// and any difference stays distinct.
type TissueKey = Vec<(String, Fixed)>;

/// The per-cell ORGANISM-TISSUE field: the located matter a dead organism leaves, keyed by [`Coord3`] then by
/// content identity, with the accumulated volume as the value. A sibling of [`SoilNutrientField`] and
/// [`crate::decompose::DecomposerStockField`] with the same empty-default-folds-nothing discipline, so an
/// unarmed or unseeded field folds no bytes into `state_hash` and a scenario with no corpse matter is
/// byte-identical. Reads (fracture hardness, per-axis presence) walk the parcels; the consumers
/// (extraction, cutting, the matter cycle) union this with the located substance mixture, so an organism's
/// matter is worked and rots by the SAME mechanisms and the SAME axes as any other matter.
#[derive(Clone, Debug, Default)]
pub struct TissueField {
    cells: BTreeMap<Coord3, BTreeMap<TissueKey, Fixed>>,
}

impl TissueField {
    /// A field holding no tissue anywhere.
    pub fn new() -> TissueField {
        TissueField::default()
    }

    /// Whether no tissue has been deposited anywhere (the opt-out state a scenario with no corpse matter
    /// stays in, so its `state_hash` fold folds nothing and it replays bit-for-bit).
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Deposit a parcel of an organism's matter at a cell: a non-positive volume is a no-op (no empty entry
    /// is created), and a deposit whose composition is byte-identical to one already present accumulates its
    /// volume onto that parcel (content identity).
    pub fn deposit(&mut self, cell: Coord3, composition: BTreeMap<String, Fixed>, volume: Fixed) {
        if volume <= Fixed::ZERO {
            return;
        }
        // A BTreeMap iterates in sorted key order, so the collected pairs are canonical and the key is stable.
        let key: TissueKey = composition.into_iter().collect();
        let entry = self
            .cells
            .entry(cell)
            .or_default()
            .entry(key)
            .or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(volume);
    }

    /// Remove up to `want` of AXIS `axis` from the located body matter at a cell (the eaten-and-deplete
    /// sibling of [`Self::deposit`]): a bite of a body, drawn through the SAME edibility a grazer uses on
    /// standing food, so predation is grazing on a located depleting body and needs no new verb. Each parcel
    /// yields up to `volume * composition[axis]` of the axis; removing that reduces the parcel's VOLUME
    /// proportionally, so the whole body shrinks together with its composition unchanged (a fraction of the
    /// carcass is eaten, not one axis leached out of it). Walks parcels in canonical `TissueKey` order
    /// (matching [`Self::parcels`] and the `hash_into` fold), drops an emptied parcel and an emptied cell so
    /// a fully-eaten cell is byte-identical to a never-deposited one, and returns the axis mass removed
    /// (which equals the body's loss, so the eater's gain is conservation-honest). Keyed off the axis string
    /// alone, never an identity: what a body yields is whatever its physics-floor composition carries, so an
    /// alien tissue enters as data (Principle 9).
    pub fn take(&mut self, cell: Coord3, axis: &str, want: Fixed) -> Fixed {
        if want <= Fixed::ZERO {
            return Fixed::ZERO;
        }
        let Some(parcels) = self.cells.get_mut(&cell) else {
            return Fixed::ZERO;
        };
        let keys: Vec<TissueKey> = parcels.keys().cloned().collect();
        let mut remaining = want;
        let mut removed = Fixed::ZERO;
        for key in keys {
            if remaining <= Fixed::ZERO {
                break;
            }
            let density = key
                .iter()
                .find(|(a, _)| a.as_str() == axis)
                .map(|(_, v)| *v)
                .unwrap_or(Fixed::ZERO);
            if density <= Fixed::ZERO {
                continue;
            }
            let volume = *parcels.get(&key).unwrap();
            let available = volume.checked_mul(density).unwrap_or(Fixed::MAX);
            let take_axis = remaining.min(available);
            let vol_removed = take_axis
                .checked_div(density)
                .unwrap_or(Fixed::ZERO)
                .min(volume);
            let axis_removed = vol_removed.checked_mul(density).unwrap_or(Fixed::ZERO);
            let new_volume = volume - vol_removed;
            if new_volume <= Fixed::ZERO {
                parcels.remove(&key);
            } else {
                parcels.insert(key, new_volume);
            }
            removed = removed.saturating_add(axis_removed);
            remaining -= axis_removed;
        }
        if parcels.is_empty() {
            self.cells.remove(&cell);
        }
        removed
    }

    /// Take a single whole-body BITE of up to `want` VOLUME from the located body matter at a cell (predation's
    /// whole-body bite, chemistry arc Arc 2), returning the mass of EVERY axis in the removed volume. Unlike
    /// [`Self::take`] (which draws one named axis and removes the volume that carried it), one bite removes one
    /// volume ONCE and credits every nutrient the eater assimilates, so a multi-axis carcass is not
    /// over-depleted (the earlier per-axis draw removed a volume per axis, a declared-open-biomass leak). Walks
    /// parcels in canonical order, shrinks each proportionally, drops an emptied parcel and an emptied cell so
    /// a fully-eaten cell is byte-identical to a never-deposited one, and returns the summed per-axis mass
    /// removed. A non-positive want or an empty cell returns an empty map. Keyed off no identity: what the bite
    /// yields is whatever the body's own composition carries (Principle 9).
    pub fn bite(&mut self, cell: Coord3, want: Fixed) -> BTreeMap<String, Fixed> {
        let mut removed_axes: BTreeMap<String, Fixed> = BTreeMap::new();
        if want <= Fixed::ZERO {
            return removed_axes;
        }
        let Some(parcels) = self.cells.get_mut(&cell) else {
            return removed_axes;
        };
        let keys: Vec<TissueKey> = parcels.keys().cloned().collect();
        let mut remaining = want;
        for key in keys {
            if remaining <= Fixed::ZERO {
                break;
            }
            let volume = *parcels.get(&key).unwrap();
            let take_vol = remaining.min(volume);
            if take_vol <= Fixed::ZERO {
                continue;
            }
            // Every axis in the removed volume: its mass is removed_volume * density (the body's own value).
            for (axis, density) in &key {
                let m = take_vol.checked_mul(*density).unwrap_or(Fixed::MAX);
                let acc = removed_axes.entry(axis.clone()).or_insert(Fixed::ZERO);
                *acc = acc.saturating_add(m);
            }
            let new_volume = volume - take_vol;
            if new_volume <= Fixed::ZERO {
                parcels.remove(&key);
            } else {
                parcels.insert(key, new_volume);
            }
            remaining -= take_vol;
        }
        if parcels.is_empty() {
            self.cells.remove(&cell);
        }
        removed_axes
    }

    /// The total of AXIS `axis` available in the located body matter at a cell (the read the ingest measures
    /// its bite against): the sum over parcels of `volume * composition[axis]`. Zero where no body lies.
    pub fn axis_supply(&self, cell: Coord3, axis: &str) -> Fixed {
        let Some(parcels) = self.cells.get(&cell) else {
            return Fixed::ZERO;
        };
        let mut total = Fixed::ZERO;
        for (key, &volume) in parcels {
            let density = key
                .iter()
                .find(|(a, _)| a.as_str() == axis)
                .map(|(_, v)| *v)
                .unwrap_or(Fixed::ZERO);
            total = total.saturating_add(volume.checked_mul(density).unwrap_or(Fixed::MAX));
        }
        total
    }

    /// The IDENTITY-BLIND, valence-blind total matter mass located at a cell (the tissue analogue of
    /// [`crate::locomotion::ResourceField::cell_content`], so the resource-density percept can union the two
    /// matter pools and a being is drawn to a corpse exactly as to a plant, per the creature-selection-loop
    /// slice-2 ruling). The sum over parcels of `volume * sum-of-composition-densities`, which equals
    /// [`Self::axis_supply`] summed over every axis: a body's total mass across all substances it carries, read
    /// without inspecting which substances they are. Zero where no body lies. A pure read: it moves no matter,
    /// so the two intake paths (the forage INGEST over [`crate::locomotion::ResourceField`] and the whole-body
    /// bite over this field) each still eat their own pool once, with no double-count.
    pub fn cell_content(&self, cell: Coord3) -> Fixed {
        let Some(parcels) = self.cells.get(&cell) else {
            return Fixed::ZERO;
        };
        let mut total = Fixed::ZERO;
        for (key, &volume) in parcels {
            let density_sum = key
                .iter()
                .fold(Fixed::ZERO, |acc, (_, v)| acc.saturating_add(*v));
            total = total.saturating_add(volume.checked_mul(density_sum).unwrap_or(Fixed::MAX));
        }
        total
    }

    /// Rot every parcel by a fraction of its volume, returning per parcel the (cell, its own COMPOSITION, mass
    /// removed) for the soil deposit (the tissue -> soil RETURN leg of the nutrient cycle): the located body
    /// matter is fed back to the soil the producers draw, re-materialised into the soil by the body's OWN
    /// composition axes (T5), symmetric with the material leg's per-substance split. Each parcel carries its
    /// own composition so a caller ([`crate::material::ConstituentRegistry::split`]) apportions its lost mass
    /// by that composition, never a hardcoded organic bucket. Canonical order; an emptied parcel/cell is
    /// dropped so the fold stays reproducible. The removed volume is the mass returned (a unit tissue density
    /// until a reserved density lands).
    pub fn decay(&mut self, rate: Fixed) -> Vec<(Coord3, BTreeMap<String, Fixed>, Fixed)> {
        if rate <= Fixed::ZERO {
            return Vec::new();
        }
        let mut out = Vec::new();
        let cells: Vec<Coord3> = self.cells.keys().copied().collect();
        for cell in cells {
            let parcels = self.cells.get_mut(&cell).unwrap();
            let keys: Vec<TissueKey> = parcels.keys().cloned().collect();
            for key in keys {
                let volume = *parcels.get(&key).unwrap();
                let d = volume.checked_mul(rate).unwrap_or(Fixed::ZERO).min(volume);
                if d <= Fixed::ZERO {
                    continue;
                }
                let nv = volume - d;
                let composition: BTreeMap<String, Fixed> = key.iter().cloned().collect();
                if nv <= Fixed::ZERO {
                    parcels.remove(&key);
                } else {
                    parcels.insert(key, nv);
                }
                out.push((cell, composition, d));
            }
            if parcels.is_empty() {
                self.cells.remove(&cell);
            }
        }
        out
    }

    /// The parcels at a cell, reconstructed as [`TissueParcel`]s in canonical content order. An empty cell
    /// yields nothing.
    pub fn parcels(&self, cell: Coord3) -> Vec<TissueParcel> {
        self.cells
            .get(&cell)
            .map(|m| {
                m.iter()
                    .map(|(key, &volume)| TissueParcel {
                        volume,
                        composition: key.iter().cloned().collect(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every cell holding tissue, in canonical [`Coord3`] order (for a reader or a consumer that walks the
    /// deposited matter). An empty field yields nothing.
    pub fn cells(&self) -> impl Iterator<Item = Coord3> + '_ {
        self.cells.keys().copied()
    }

    /// The total tissue volume at a cell, summed over its parcels. A cell with no tissue reads zero.
    /// Saturating, so an over-deposited cell caps rather than wrapping.
    pub fn volume_at(&self, cell: Coord3) -> Fixed {
        self.cells
            .get(&cell)
            .map(|m| {
                m.values()
                    .fold(Fixed::ZERO, |acc, v| acc.saturating_add(*v))
            })
            .unwrap_or(Fixed::ZERO)
    }

    /// The total tissue volume over every cell and parcel (the tissue term of the matter-cycle conservation
    /// ledger, chemistry arc Arc 4): the located body matter that decomposition returns to the soil store, at
    /// the unit tissue density the decay leg uses. Saturating; an empty field reads zero.
    pub fn total_volume(&self) -> Fixed {
        self.cells.values().fold(Fixed::ZERO, |acc, parcels| {
            parcels.values().fold(acc, |a, v| a.saturating_add(*v))
        })
    }

    /// The fracture hardness a being must overcome to work the tissue at a cell: the greatest
    /// `mat.fracture_strength` any parcel there carries (an absent axis reads zero). A cell with no tissue
    /// reads zero, so it contributes nothing to a consumer's `.max(...)` union with the located substance
    /// mixture, and the run is byte-identical where no corpse lies.
    pub fn fracture_hardness(&self, cell: Coord3) -> Fixed {
        self.cells
            .get(&cell)
            .map(|m| {
                m.keys().fold(Fixed::ZERO, |acc, key| {
                    let strength = key
                        .iter()
                        .find(|(axis, _)| axis == "mat.fracture_strength")
                        .map(|(_, v)| *v)
                        .unwrap_or(Fixed::ZERO);
                    acc.max(strength)
                })
            })
            .unwrap_or(Fixed::ZERO)
    }

    /// Fold the tissue field into a hash in canonical (cell, composition pairs, volume) order, beside
    /// [`SoilNutrientField::hash_into`]. An empty field folds nothing, so an opted-out run is unchanged. The
    /// `BTreeMap`s walk in canonical key order (the `Coord3` then the sorted composition), so the fold is
    /// reproducible and thread-invariant.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (cell, parcels) in &self.cells {
            for (key, volume) in parcels {
                h.write_i64(cell.x as i64);
                h.write_i64(cell.y as i64);
                h.write_i64(cell.z as i64);
                for (axis, value) in key {
                    for b in axis.as_bytes() {
                        h.write_u32(*b as u32);
                    }
                    h.write_fixed(*value);
                }
                h.write_fixed(*volume);
            }
        }
    }
}

/// One horizontal stratum of a ground profile: a substance filling every cell in an inclusive z-band
/// at a fixed volume. The membership is data (a substance id and a z-band), so a world's stratigraphy
/// is data-defined, never a `Rock`/`Soil`/`Ore` enum (Principle 8 and 11): a new stratum is a data
/// edit, and the substance is any [`civsim_physics::Substance`] the ground floor declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundLayer {
    /// The physics substance id filling this stratum.
    pub substance: String,
    /// The lowest z the stratum fills, inclusive; z counts down from the surface at zero.
    pub z_lo: i32,
    /// The highest z the stratum fills, inclusive.
    pub z_hi: i32,
    /// The volume of the substance deposited in each cell of the stratum.
    pub volume: Fixed,
}

/// A ground profile: the stratigraphy a world's z-column is filled from, a list of [`GroundLayer`]s.
/// Overlapping strata accumulate in a cell (an ore band within bedrock is the two substances mixed),
/// so a mixed cell's bulk properties are the volume-weighted mean of its strata, derived from the
/// registry. The profile is data (Principle 11); the fill is fixed Rust and deterministic.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroundProfile {
    /// The strata; deposits accumulate, so the fill result is independent of their order.
    pub layers: Vec<GroundLayer>,
}

impl GroundProfile {
    /// A labelled DEVELOPMENT FIXTURE stratigraphy, not owner values: a loam topsoil at the surface, a
    /// granite bedrock below it, and a hematite ore band within the bedrock, so a filled world exercises
    /// both a single-substance cell and a mixed (ore-in-rock) cell. The substances are ground-floor rows
    /// ([`civsim_physics::PhysicsRegistry::ground`]); this fixture lets a material world be filled and
    /// seen now, the way the worldgen fixtures let the map be generated before the owner sets the
    /// authoritative worldgen calibration.
    pub fn dev() -> GroundProfile {
        GroundProfile {
            layers: vec![
                GroundLayer {
                    substance: "loam".to_string(),
                    z_lo: 0,
                    z_hi: 0,
                    volume: Fixed::ONE,
                },
                GroundLayer {
                    substance: "granite".to_string(),
                    z_lo: -3,
                    z_hi: -1,
                    volume: Fixed::ONE,
                },
                GroundLayer {
                    substance: "hematite".to_string(),
                    z_lo: -2,
                    z_hi: -2,
                    volume: Fixed::ONE,
                },
            ],
        }
    }

    /// Fill a material layer from this profile over a `width` by `height` surface extent, depositing
    /// each stratum's substance into every cell of its z-band in canonical (x, y, z) order with no
    /// randomness. This is the deterministic worldgen population of the ground (cascade item 1): a
    /// material-declaring world builds its z-column here, and every derived hardness, weight, and fuel
    /// value downstream reads the substances placed. A non-positive extent yields an empty field (the
    /// opt-out). Off the default assembly path, so a scenario that does not fill the ground stays empty
    /// and byte-identical.
    pub fn fill(&self, width: i32, height: i32) -> MaterialField {
        let mut field = MaterialField::new();
        for x in 0..width {
            for y in 0..height {
                for layer in &self.layers {
                    for z in layer.z_lo..=layer.z_hi {
                        field.deposit(Coord3 { x, y, z }, &layer.substance, layer.volume);
                    }
                }
            }
        }
        field
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constituent_split_is_mass_exact_and_generalises_the_ash_pair_to_k_axes() {
        // Chemistry arc, T5: the decomposed mass is apportioned into soil classes by the decomposing matter's
        // OWN composition (data-defined), generalizing the fixed ash-plus-organic pair to k=N constituents,
        // and the split is mass-EXACT (the constituent shares plus the residual equal the whole loss bit for
        // bit, whatever the per-share fixed-point multiplies round to).
        let mass = Fixed::from_int(100);
        // A k=3 registry: three fraction axes plus a residual. The fractions do NOT sum to a round number, so
        // the residual has to absorb the rounding for the sum to stay exact.
        let mut reg = ConstituentRegistry::new("residual");
        reg.push("ash", "soil.ash");
        reg.push("lignin", "soil.lignin");
        reg.push("labile", "soil.labile");
        let comp: BTreeMap<String, Fixed> = [
            ("ash".to_string(), Fixed::from_ratio(7, 100)),
            ("lignin".to_string(), Fixed::from_ratio(23, 100)),
            ("labile".to_string(), Fixed::from_ratio(31, 100)),
        ]
        .into_iter()
        .collect();
        let split = reg.split(mass, |a| comp.get(a).copied().unwrap_or(Fixed::ZERO));
        // Four shares: the three constituents plus the residual (39% of the mass, whatever rounds up).
        assert_eq!(split.len(), 4, "k=3 constituents plus the residual");
        let total = split
            .iter()
            .fold(Fixed::ZERO, |acc, (_, m)| acc.saturating_add(*m));
        assert_eq!(
            total, mass,
            "the shares sum to the whole loss EXACTLY (mass-exact)"
        );
        // The classes are the matter's OWN chemistry, not a hardcoded Earth bucket.
        let classes: Vec<&str> = split.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(
            classes,
            vec!["soil.ash", "soil.lignin", "soil.labile", "residual"]
        );

        // The terran_default reproduces the pre-arc k=2 split exactly (mineral-ash to a mineral class, the
        // remainder to an organic class), the byte-neutral fallback an unarmed matter cycle uses.
        let carrion: BTreeMap<String, Fixed> = [(
            "bio.mineral_ash_fraction".to_string(),
            Fixed::from_ratio(5, 100),
        )]
        .into_iter()
        .collect();
        let d = ConstituentRegistry::terran_default();
        let s = d.split(mass, |a| carrion.get(a).copied().unwrap_or(Fixed::ZERO));
        // The expected shares are computed the SAME way the pre-arc code did (mineral = mass * ash via
        // checked_mul, organic = the exact remainder), so this proves equivalence without assuming 0.05 is
        // exactly representable in fixed point. The remainder construction is what keeps the sum exact.
        let ash = Fixed::from_ratio(5, 100);
        let mineral = mass.checked_mul(ash).unwrap();
        let organic = mass - mineral;
        assert_eq!(
            s,
            vec![
                ("bio.mineral_ash_fraction".to_string(), mineral),
                ("bio.organic_residue".to_string(), organic),
            ],
            "the default is the mineral-ash + organic-remainder pair (the pre-arc split)"
        );
        assert_eq!(
            mineral.saturating_add(organic),
            mass,
            "and the pair still sums to the whole loss exactly"
        );

        // A body carrying NONE of the fraction axes lands its WHOLE mass in the residual (the tissue leg's
        // byte-neutral case: a body with no declared constituent decays to the organic residual entire).
        let body: BTreeMap<String, Fixed> = [("mat.density".to_string(), Fixed::from_int(1000))]
            .into_iter()
            .collect();
        let sb = reg.split(mass, |a| body.get(a).copied().unwrap_or(Fixed::ZERO));
        assert_eq!(
            sb,
            vec![("residual".to_string(), mass)],
            "a body with no constituent axis returns its whole mass to the residual (mass-exact)"
        );
    }

    #[test]
    fn a_tissue_field_accumulates_by_content_and_folds_order_free() {
        let cell = Coord3::ground(2, 3);
        let mut field = TissueField::new();
        assert!(field.is_empty());
        // Two byte-identical compositions accumulate onto one parcel; a different one stays distinct.
        let soft: BTreeMap<String, Fixed> = [
            ("mat.fracture_strength".to_string(), Fixed::from_int(3)),
            ("bio.energy_density".to_string(), Fixed::from_int(5)),
        ]
        .into_iter()
        .collect();
        let hard: BTreeMap<String, Fixed> =
            [("mat.fracture_strength".to_string(), Fixed::from_int(30))]
                .into_iter()
                .collect();
        field.deposit(cell, soft.clone(), Fixed::from_int(2));
        field.deposit(cell, soft.clone(), Fixed::from_int(1)); // same content, accumulates
        field.deposit(cell, hard.clone(), Fixed::from_int(4)); // distinct content, new parcel
        field.deposit(cell, soft.clone(), Fixed::ZERO); // non-positive, a no-op
        let parcels = field.parcels(cell);
        assert_eq!(
            parcels.len(),
            2,
            "two distinct compositions, the identical ones merged"
        );
        assert_eq!(
            field.volume_at(cell),
            Fixed::from_int(7),
            "2 + 1 (merged) + 4 = 7 total volume"
        );
        // The fracture hardness a being must overcome is the greatest fracture strength present.
        assert_eq!(
            field.fracture_hardness(cell),
            Fixed::from_int(30),
            "the hardest parcel sets the fracture gate"
        );
        // A cell with no tissue reads zero, so it adds nothing to a consumer's max-union.
        assert_eq!(field.fracture_hardness(Coord3::ground(0, 0)), Fixed::ZERO);

        // The hash is order-free: depositing the same parcels in a different order folds identically.
        let mut other = TissueField::new();
        other.deposit(cell, hard, Fixed::from_int(4));
        other.deposit(cell, soft.clone(), Fixed::from_int(1));
        other.deposit(cell, soft, Fixed::from_int(2));
        let mut h1 = StateHasher::new();
        field.hash_into(&mut h1);
        let mut h2 = StateHasher::new();
        other.hash_into(&mut h2);
        assert_eq!(
            h1.finish(),
            h2.finish(),
            "deposit order does not change the fold"
        );
    }

    #[test]
    fn tissue_cell_content_is_the_identity_blind_total_matter_mass() {
        // The percept-union read (creature-selection-loop slice 2): the cell's total matter mass, summed over
        // every substance a corpse carries, without inspecting which substances they are. It equals
        // `axis_supply` summed over all axes, so a being senses a corpse by its total mass exactly as it senses
        // a plant patch by its total content.
        let cell = Coord3::ground(2, 3);
        let mut field = TissueField::new();
        assert_eq!(field.cell_content(cell), Fixed::ZERO, "no body, no content");
        // A two-substance corpse (one axis a structural material, one a metabolic store) and a distinct
        // single-substance parcel: the read must not privilege either axis, so it sums all of them.
        let soft: BTreeMap<String, Fixed> = [
            ("mat.fracture_strength".to_string(), Fixed::from_int(3)),
            ("bio.energy_density".to_string(), Fixed::from_int(5)),
        ]
        .into_iter()
        .collect();
        let hard: BTreeMap<String, Fixed> =
            [("mat.fracture_strength".to_string(), Fixed::from_int(30))]
                .into_iter()
                .collect();
        field.deposit(cell, soft.clone(), Fixed::from_int(2));
        field.deposit(cell, soft, Fixed::from_int(1)); // same content, accumulates to volume 3
        field.deposit(cell, hard, Fixed::from_int(4));
        // soft: volume 3 * (3 + 5) = 24; hard: volume 4 * 30 = 120; total 144.
        assert_eq!(field.cell_content(cell), Fixed::from_int(144));
        // The union read agrees with `axis_supply` summed over every axis present (the identity-blind identity).
        let by_axis = field
            .axis_supply(cell, "mat.fracture_strength")
            .saturating_add(field.axis_supply(cell, "bio.energy_density"));
        assert_eq!(field.cell_content(cell), by_axis);
        // A never-touched cell stays zero, so an unarmed or corpse-free scenario adds nothing to the union.
        assert_eq!(field.cell_content(Coord3::ground(9, 9)), Fixed::ZERO);
    }

    /// A minimal mechanical floor: the two axes the material layer reads, and three substances that
    /// exercise the derivation (a hard dense rock, a soft light soil, and a fuel with no hardness axis
    /// so the absence path is covered). Ranges and values are stand-in test data, not owner values.
    const FLOOR: &str = r#"
[[axis]]
id = "mat.density"
measures = "bulk mass per unit volume"
unit = "kg/m^3"
dimension = "-3,1,0,0"
scale = "kg/m^3"
tier = 0
range_lo = "0.08"
range_hi = "23000"
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
  { axis = "mat.indentation_hardness", value = "5000" },
  { axis = "mat.fracture_strength", value = "20" },
]

[[substance]]
id = "soil"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1500" },
  { axis = "mat.indentation_hardness", value = "100" },
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
    fn a_wielded_tools_mass_is_its_substance_density_times_its_retained_volume() {
        // The tool as matter (the made-world arc, root R2): its mass is recoverable now that it retains its
        // VOLUME (the extensive quantity the registry's intensive density cannot supply), density times volume,
        // derived not stored. This is the datum a swing or an impact reads, and the stock a wear step decrements.
        let reg = floor();
        let tool = WieldedTool {
            contact_area: Fixed::from_ratio(1, 1000),
            volume: Fixed::from_int(2),
            length: Fixed::ONE,
            substance: "granite".to_string(),
        };
        // granite density 2700 times retained volume 2 is 5400 kg.
        assert_eq!(tool.mass(&reg), Fixed::from_int(5400));
        // A substance the registry does not carry (no density) reads zero mass, the absence convention.
        let void_tool = WieldedTool {
            contact_area: Fixed::from_ratio(1, 1000),
            volume: Fixed::from_int(2),
            length: Fixed::ONE,
            substance: "nonexistent".to_string(),
        };
        assert_eq!(void_tool.mass(&reg), Fixed::ZERO);
    }

    #[test]
    fn the_earthwork_field_accumulates_digs_and_mounds_and_folds_canonically() {
        let mut ew = EarthworkField::new();
        assert!(ew.is_empty());
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);
        // An unreworked column reads a zero delta.
        assert_eq!(ew.delta(a), Fixed::ZERO);
        // Digging lowers the column; a second dig accumulates.
        ew.adjust(a, Fixed::from_int(-2));
        ew.adjust(a, Fixed::from_int(-1));
        assert_eq!(ew.delta(a), Fixed::from_int(-3), "digs accumulate");
        // Mounding another column raises it.
        ew.adjust(b, Fixed::from_int(4));
        assert_eq!(ew.delta(b), Fixed::from_int(4));
        assert!(!ew.is_empty());
        // Filling a pit back to the baseline prunes the column (a zero delta is void).
        ew.adjust(a, Fixed::from_int(3));
        assert_eq!(
            ew.delta(a),
            Fixed::ZERO,
            "a column back at baseline reads zero"
        );
        // The hash is canonical: two fields with the same deltas built in different orders fold identically.
        let mut c1 = EarthworkField::new();
        c1.adjust(Coord3::ground(0, 0), Fixed::from_int(1));
        c1.adjust(Coord3::ground(9, 9), Fixed::from_int(-1));
        let mut c2 = EarthworkField::new();
        c2.adjust(Coord3::ground(9, 9), Fixed::from_int(-1));
        c2.adjust(Coord3::ground(0, 0), Fixed::from_int(1));
        let hash = |ew: &EarthworkField| {
            let mut h = StateHasher::new();
            ew.hash_into(&mut h);
            h.finish()
        };
        assert_eq!(
            hash(&c1),
            hash(&c2),
            "the fold is insertion-order-independent"
        );
        // An empty field folds nothing (opting out is hash-neutral).
        let mut h = StateHasher::new();
        EarthworkField::new().hash_into(&mut h);
        let h0 = StateHasher::new();
        assert_eq!(h.finish(), h0.finish(), "an empty earthwork folds no bytes");
    }

    #[test]
    fn the_geological_source_sums_with_the_being_earthwork_and_stays_byte_neutral_when_unarmed() {
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);
        let hash = |ew: &EarthworkField| {
            let mut h = StateHasher::new();
            ew.hash_into(&mut h);
            h.finish()
        };
        // A field with only being earthwork is byte-identical to one built by the old delta-only path: the
        // empty geological map folds no bytes, so adding the geological source did not perturb the hash.
        let mut being_only = EarthworkField::new();
        being_only.adjust(a, Fixed::from_int(-3));
        being_only.adjust(b, Fixed::from_int(4));
        assert!(
            being_only.geological_delta(a) == Fixed::ZERO
                && being_only.geological_delta(b) == Fixed::ZERO
        );
        assert_eq!(
            being_only.total_delta(a),
            being_only.delta(a),
            "with the geology unarmed the effective delta is the being delta alone"
        );
        // The geological source accumulates, prunes to zero, and is a distinct source from the being delta.
        let mut ew = EarthworkField::new();
        ew.adjust(a, Fixed::from_int(-3)); // a being pit
        ew.adjust_geological(a, Fixed::from_int(10)); // geological uplift under it
        ew.adjust_geological(a, Fixed::from_int(2)); // more uplift, accumulating
        assert_eq!(
            ew.geological_delta(a),
            Fixed::from_int(12),
            "geological uplift accumulates"
        );
        assert_eq!(
            ew.delta(a),
            Fixed::from_int(-3),
            "the being delta is untouched by the geology"
        );
        assert_eq!(
            ew.total_delta(a),
            Fixed::from_int(9),
            "the physics reads the sum: a pit dug into lifted crust"
        );
        // A geological-only field is not empty, and folds canonically (insertion-order-independent).
        let mut g1 = EarthworkField::new();
        g1.adjust_geological(Coord3::ground(0, 0), Fixed::from_int(1));
        g1.adjust_geological(Coord3::ground(9, 9), Fixed::from_int(-1));
        assert!(!g1.is_empty(), "an armed geology is not the opt-out state");
        let mut g2 = EarthworkField::new();
        g2.adjust_geological(Coord3::ground(9, 9), Fixed::from_int(-1));
        g2.adjust_geological(Coord3::ground(0, 0), Fixed::from_int(1));
        assert_eq!(
            hash(&g1),
            hash(&g2),
            "the geological fold is insertion-order-independent"
        );
        // Driving the geological uplift back to zero prunes it and returns the field to empty.
        let mut p = EarthworkField::new();
        p.adjust_geological(a, Fixed::from_int(5));
        p.adjust_geological(a, Fixed::from_int(-5));
        assert!(
            p.is_empty(),
            "a geology relaxed back to baseline is the opt-out state again"
        );
    }

    #[test]
    fn the_geodynamic_field_carries_the_interface_state_prunes_the_default_and_folds_canonically() {
        let mut g = GeodynamicField::new();
        assert!(g.is_empty(), "no column carries geodynamic state at first");
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);
        // An unset column reads the zero default.
        assert_eq!(g.get(a), GeodynamicColumn::default());
        // The surface lane writes a crustal density; the interior lane writes the isostatic elevation and
        // uplift; each reads the other's fields at the same column, the two-way interface.
        let state = GeodynamicColumn {
            crustal_density: Fixed::from_ratio(33, 10),
            crustal_thickness: Fixed::from_int(35_000),
            isostatic_elevation: Fixed::from_int(5),
            ..GeodynamicColumn::default()
        };
        g.set(a, state);
        assert_eq!(g.get(a).crustal_density, Fixed::from_ratio(33, 10));
        assert_eq!(g.get(a).crustal_thickness, Fixed::from_int(35_000));
        assert_eq!(g.get(a).isostatic_elevation, Fixed::from_int(5));
        assert!(!g.is_empty());
        // Setting a column back to the all-zero default prunes it (the absence convention).
        g.set(a, GeodynamicColumn::default());
        assert!(g.is_empty(), "an all-zero state is pruned");
        // The hash is canonical: two fields with the same states built in different orders fold identically.
        let s1 = GeodynamicColumn {
            crustal_density: Fixed::from_int(3),
            crustal_thickness: Fixed::ZERO,
            isostatic_elevation: Fixed::ZERO,
            ..GeodynamicColumn::default()
        };
        let s2 = GeodynamicColumn {
            crustal_density: Fixed::ZERO,
            crustal_thickness: Fixed::ZERO,
            isostatic_elevation: Fixed::from_int(-1),
            ..GeodynamicColumn::default()
        };
        let mut c1 = GeodynamicField::new();
        c1.set(a, s1);
        c1.set(b, s2);
        let mut c2 = GeodynamicField::new();
        c2.set(b, s2);
        c2.set(a, s1);
        let hash = |g: &GeodynamicField| {
            let mut h = StateHasher::new();
            g.hash_into(&mut h);
            h.finish()
        };
        assert_eq!(
            hash(&c1),
            hash(&c2),
            "the fold is insertion-order-independent"
        );
        // An empty field folds nothing (opting out is hash-neutral).
        let mut h = StateHasher::new();
        GeodynamicField::new().hash_into(&mut h);
        assert_eq!(
            h.finish(),
            StateHasher::new().finish(),
            "an empty geodynamic field folds no bytes"
        );
    }

    #[test]
    fn the_fire_field_records_burning_cells_prunes_the_extinguished_and_folds_canonically() {
        let mut fire = FireField::new();
        assert!(fire.is_empty());
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);
        // An unlit cell reads zero intensity.
        assert_eq!(fire.intensity(a), Fixed::ZERO);
        // A burning cell records its released energy.
        fire.set(a, Fixed::from_int(7));
        fire.set(b, Fixed::from_int(3));
        assert_eq!(fire.intensity(a), Fixed::from_int(7), "the cell burns");
        assert!(!fire.is_empty());
        // A cell that stops burning (zero intensity) is pruned, so the field reflects the current combustion.
        fire.set(a, Fixed::ZERO);
        assert_eq!(
            fire.intensity(a),
            Fixed::ZERO,
            "an extinguished cell is unlit"
        );
        // The hash is canonical: two fields with the same intensities built in different orders fold identically.
        let mut c1 = FireField::new();
        c1.set(Coord3::ground(0, 0), Fixed::from_int(1));
        c1.set(Coord3::ground(9, 9), Fixed::from_int(2));
        let mut c2 = FireField::new();
        c2.set(Coord3::ground(9, 9), Fixed::from_int(2));
        c2.set(Coord3::ground(0, 0), Fixed::from_int(1));
        let hash = |f: &FireField| {
            let mut h = StateHasher::new();
            f.hash_into(&mut h);
            h.finish()
        };
        assert_eq!(
            hash(&c1),
            hash(&c2),
            "the fold is insertion-order-independent"
        );
        // An empty (unlit) field folds nothing (opting out is hash-neutral).
        let mut h = StateHasher::new();
        FireField::new().hash_into(&mut h);
        let h0 = StateHasher::new();
        assert_eq!(
            h.finish(),
            h0.finish(),
            "an unlit fire field folds no bytes"
        );
    }

    #[test]
    fn a_pure_cell_derives_its_substances_registry_values_at_any_volume() {
        let reg = floor();
        // A single-substance cell's bulk property is exactly that substance's registry value,
        // independent of how much of it there is; the mass scales with the volume.
        for v in [1, 2, 7] {
            let m = mix(&[("granite", v)]);
            assert_eq!(
                m.bulk_density(&reg),
                Fixed::from_int(2700),
                "density, v={v}"
            );
            assert_eq!(
                m.bulk_hardness(&reg),
                Fixed::from_int(5000),
                "hardness, v={v}"
            );
            assert_eq!(m.mass(&reg), Fixed::from_int(2700 * v), "mass, v={v}");
        }
    }

    #[test]
    fn a_mixture_derives_the_volume_weighted_mean() {
        let reg = floor();
        // granite volume 1 (rho 2700, H 5000) plus soil volume 3 (rho 1500, H 100); total volume 4.
        let m = mix(&[("granite", 1), ("soil", 3)]);
        assert_eq!(m.total_volume(), Fixed::from_int(4));
        // mass = 1*2700 + 3*1500 = 7200; bulk density = 7200 / 4 = 1800.
        assert_eq!(m.mass(&reg), Fixed::from_int(7200));
        assert_eq!(m.bulk_density(&reg), Fixed::from_int(1800));
        // bulk hardness = (1*5000 + 3*100) / 4 = 5300 / 4 = 1325.
        assert_eq!(m.bulk_hardness(&reg), Fixed::from_int(1325));
    }

    #[test]
    fn a_substance_missing_an_axis_contributes_zero_to_that_property() {
        let reg = floor();
        // peat carries a density but NO hardness axis, so it contributes zero to the hardness weighted
        // sum while still counting in the total volume (the absence convention).
        let m = mix(&[("granite", 1), ("peat", 1)]);
        // hardness = (5000 + 0) / 2 = 2500.
        assert_eq!(m.bulk_hardness(&reg), Fixed::from_int(2500));
        // density = (2700 + 400) / 2 = 1550.
        assert_eq!(m.bulk_density(&reg), Fixed::from_int(1550));
    }

    #[test]
    fn the_fracture_gating_hardness_is_the_hardest_constituent_not_the_mean() {
        let reg = floor();
        // A pure cell reads its substance's own fracture strength, at any volume.
        for v in [1, 4, 9] {
            assert_eq!(
                mix(&[("granite", v)]).fracture_hardness(&reg),
                Fixed::from_int(20),
                "pure granite fractures at its own strength, v={v}"
            );
        }
        // Ore in rock: a mostly-soft cell with a little granite fractures at the GRANITE (the hardest
        // constituent, 20), NOT at the volume-weighted mean the bulk read would take. With granite 1 and
        // soil 3 the mean fracture would be (1*20 + 3*3) / 4 = 29/4 ~= 7, far below 20; the contest must
        // clear the granite to break anything loose, so the read is 20. This is the gate's item-4 note:
        // ore embedded in granite breaks at the granite, not at a blend.
        let ore_in_rock = mix(&[("granite", 1), ("soil", 3)]);
        assert_eq!(
            ore_in_rock.fracture_hardness(&reg),
            Fixed::from_int(20),
            "the cell fractures at its hardest constituent (20), not the ~7 the mean would give"
        );
        // A constituent carrying no fracture datum contributes zero to the max (the absence convention),
        // so granite plus peat still fractures at the granite.
        assert_eq!(
            mix(&[("granite", 5), ("peat", 5)]).fracture_hardness(&reg),
            Fixed::from_int(20),
            "peat declares no fracture resistance, so the granite still gates"
        );
        // A cell of only a fracture-datum-less substance offers no fracture resistance; an empty cell
        // likewise reads zero.
        assert_eq!(
            mix(&[("peat", 3)]).fracture_hardness(&reg),
            Fixed::ZERO,
            "matter with no declared fracture strength offers none"
        );
        assert_eq!(SubstanceMix::new().fracture_hardness(&reg), Fixed::ZERO);
    }

    #[test]
    fn the_extraction_contest_gates_on_fracture_strength_and_sizes_on_cutting_energy() {
        // The extraction kernel composes contact_pressure and cut_penetrate: the being's force over its
        // working area is a pressure that must CLEAR the cell's fracture strength to break anything loose
        // (the strength gate), and above the gate the detached volume is sized by the delivered work over
        // the material's specific cutting energy (the energy sizing). The two axes are independent, as the
        // fracture physics is: strength decides whether the rock breaks, energy decides how much comes off.
        let p_max = Fixed::from_int(1_000_000);
        let d_max = Fixed::from_int(1000);
        let area = Fixed::from_ratio(1, 100); // 0.01 m^2, a tool-tip contact
        let energy = Fixed::from_int(1_000_000);
        let cut_energy = Fixed::from_int(100);
        let soft = Fixed::from_int(5);
        let hard = Fixed::from_int(2000);
        // pressure = force / (area * 1e6). A weak force (1000 N over 0.01 m^2 = 0.1 MPa) does not clear
        // even the soft rock, so nothing yields.
        let weak = Fixed::from_int(1000);
        assert_eq!(
            extraction_yield(weak, area, soft, energy, cut_energy, p_max, d_max),
            Fixed::ZERO,
            "a being too weak to raise its pressure over the fracture strength mines nothing"
        );
        // A strong force (1e7 N over 0.01 m^2 = 1000 MPa) clears the soft rock and detaches a positive,
        // exact volume: depth = energy / (cut_energy * area) / 1e6 = 1e6 / 1 / 1e6 = 1.0, volume = depth *
        // area = 0.01.
        let strong = Fixed::from_int(10_000_000);
        let soft_yield = extraction_yield(strong, area, soft, energy, cut_energy, p_max, d_max);
        assert_eq!(
            soft_yield,
            Fixed::from_ratio(1, 100),
            "above the gate the yield is the swept volume of the cut depth"
        );
        // The SAME strong force on a rock whose fracture strength (2000 MPa) exceeds the 1000 MPa pressure
        // yields nothing: it cannot break the rock, however much work it delivers.
        assert_eq!(
            extraction_yield(strong, area, hard, energy, cut_energy, p_max, d_max),
            Fixed::ZERO,
            "a rock too strong for the being's pressure holds"
        );
        // An even stronger force raises the pressure over the hard rock's strength and mines it: a stronger
        // being clears a gate a weaker one cannot (the force controls the strength gate).
        let mighty = Fixed::from_int(100_000_000);
        assert!(
            extraction_yield(mighty, area, hard, energy, cut_energy, p_max, d_max) > Fixed::ZERO,
            "a stronger being breaks the harder rock the weaker one could not"
        );
        // Above the gate, a tougher-to-cut material (higher specific cutting energy) yields LESS to the
        // same force and work: the energy sizes the removal.
        let tough_cut = Fixed::from_int(400);
        assert!(
            extraction_yield(strong, area, soft, energy, tough_cut, p_max, d_max) < soft_yield,
            "a tougher material yields less to the same work"
        );
        // More delivered work yields more, at the same pressure above the gate (the energy sizing is
        // monotone).
        let more_energy = Fixed::from_int(2_000_000);
        assert!(
            extraction_yield(strong, area, soft, more_energy, cut_energy, p_max, d_max)
                > soft_yield,
            "more delivered work detaches more matter"
        );
    }

    #[test]
    fn an_unregistered_substance_and_an_empty_cell_read_zero() {
        let reg = floor();
        let mystery = mix(&[("orichalcum", 5)]);
        assert_eq!(mystery.bulk_density(&reg), Fixed::ZERO);
        assert_eq!(mystery.bulk_hardness(&reg), Fixed::ZERO);
        assert_eq!(mystery.fracture_hardness(&reg), Fixed::ZERO);
        assert_eq!(mystery.mass(&reg), Fixed::ZERO);
        let empty = SubstanceMix::new();
        assert!(empty.is_empty());
        assert_eq!(empty.bulk_density(&reg), Fixed::ZERO);
        assert_eq!(empty.total_volume(), Fixed::ZERO);
    }

    #[test]
    fn take_removes_up_to_what_is_present_and_prunes_to_void() {
        let reg = floor();
        let mut field = MaterialField::new();
        let coord = Coord3 { x: 3, y: -2, z: -1 };
        field.deposit(coord, "granite", Fixed::from_int(5));
        assert_eq!(field.volume(coord, "granite"), Fixed::from_int(5));
        // A partial take returns exactly the want and leaves the remainder.
        assert_eq!(
            field.take(coord, "granite", Fixed::from_int(2)),
            Fixed::from_int(2)
        );
        assert_eq!(field.volume(coord, "granite"), Fixed::from_int(3));
        // An over-take returns only what is present and empties the cell to void.
        assert_eq!(
            field.take(coord, "granite", Fixed::from_int(9)),
            Fixed::from_int(3)
        );
        assert!(field.is_empty());
        assert_eq!(field.bulk_hardness(coord, &reg), Fixed::ZERO);
    }

    #[test]
    fn a_whole_body_bite_removes_one_volume_and_credits_every_axis_where_per_axis_takes_over_deplete(
    ) {
        // Chemistry arc, Arc 2 (FIX-2): one whole-body bite removes ONE volume and returns the mass of EVERY
        // axis in it, so a multi-axis carcass is not over-depleted, where the per-axis `take` (one axis, and
        // the volume that carried it) would remove a volume PER axis, the declared-open-biomass leak the bite
        // fixes. A body carrying two axes (exact binary densities, so the arithmetic is representable).
        let cell = Coord3::ground(1, 1);
        let e_density = Fixed::from_ratio(1, 2); // 0.5
        let m_density = Fixed::from_ratio(1, 4); // 0.25
        let body = || -> BTreeMap<String, Fixed> {
            [
                ("bio.energy_density".to_string(), e_density),
                ("halite".to_string(), m_density),
            ]
            .into_iter()
            .collect()
        };
        // The expected per-axis mass in a 10-volume bite, computed the way the mechanism does.
        let expect_e = Fixed::from_int(10).checked_mul(e_density).unwrap();
        let expect_m = Fixed::from_int(10).checked_mul(m_density).unwrap();

        // One bite of 10 volume: removes 10 volume ONCE, credits BOTH axes.
        let mut field = TissueField::new();
        field.deposit(cell, body(), Fixed::from_int(100));
        let removed = field.bite(cell, Fixed::from_int(10));
        assert_eq!(
            removed.get("bio.energy_density").copied().unwrap(),
            expect_e,
            "the bite credits the energy axis (10 volume * its density)"
        );
        assert_eq!(
            removed.get("halite").copied().unwrap(),
            expect_m,
            "the SAME bite also credits the mineral axis: one bite, every axis"
        );
        assert_eq!(
            field.volume_at(cell),
            Fixed::from_int(90),
            "the body lost exactly ONE bite's volume (10), not one per axis"
        );

        // Contrast: two per-axis takes (the pre-fix predation) remove TWO volumes for the same nutrition, each
        // draw removing the volume that carried its axis (expect_e / e_density = 10, and again for the mineral).
        let mut field2 = TissueField::new();
        field2.deposit(cell, body(), Fixed::from_int(100));
        field2.take(cell, "bio.energy_density", expect_e);
        field2.take(cell, "halite", expect_m);
        assert_eq!(
            field2.volume_at(cell),
            Fixed::from_int(80),
            "two per-axis takes removed TWO volumes (the over-depletion the whole-body bite fixes)"
        );
    }

    #[test]
    fn deposit_accumulates_and_field_reads_derive_from_the_registry() {
        let reg = floor();
        let mut field = MaterialField::new();
        let coord = Coord3 { x: 0, y: 0, z: 0 };
        field.deposit(coord, "granite", Fixed::from_int(1));
        field.deposit(coord, "granite", Fixed::from_int(1));
        field.deposit(coord, "soil", Fixed::from_int(2));
        // granite 2, soil 2: mass = 2*2700 + 2*1500 = 8400; density = 8400/4 = 2100.
        assert_eq!(field.mass(coord, &reg), Fixed::from_int(8400));
        assert_eq!(field.bulk_density(coord, &reg), Fixed::from_int(2100));
    }

    #[test]
    fn the_hash_is_canonical_and_insertion_order_independent() {
        // Two cells built by inserting the same substances in opposite order fold identically, and a
        // field folds the same regardless of the order its cells were set.
        let mut a = SubstanceMix::new();
        a.set("granite", Fixed::from_int(2));
        a.set("soil", Fixed::from_int(5));
        let mut b = SubstanceMix::new();
        b.set("soil", Fixed::from_int(5));
        b.set("granite", Fixed::from_int(2));
        let (mut ha, mut hb) = (StateHasher::new(), StateHasher::new());
        a.hash_into(&mut ha);
        b.hash_into(&mut hb);
        assert_eq!(ha.finish(), hb.finish());

        let c1 = Coord3 { x: 1, y: 1, z: 0 };
        let c2 = Coord3 { x: 1, y: 2, z: 0 };
        let mut fa = MaterialField::new();
        fa.set_cell(c1, a.clone());
        fa.set_cell(c2, b.clone());
        let mut fb = MaterialField::new();
        fb.set_cell(c2, b.clone());
        fb.set_cell(c1, a.clone());
        let (mut hfa, mut hfb) = (StateHasher::new(), StateHasher::new());
        fa.hash_into(&mut hfa);
        fb.hash_into(&mut hfb);
        assert_eq!(hfa.finish(), hfb.finish());
    }

    #[test]
    fn an_empty_field_folds_nothing_so_opting_out_is_hash_neutral() {
        // The opt-out state: an empty material layer folds no bytes, so a scenario that declares no
        // matter is byte-identical to one with no material layer at all (the empty-default discipline).
        let empty = MaterialField::new();
        assert!(empty.is_empty());
        let mut folded = StateHasher::new();
        empty.hash_into(&mut folded);
        let fresh = StateHasher::new();
        assert_eq!(folded.finish(), fresh.finish());
    }

    #[test]
    fn a_ground_profile_fills_the_z_column_deterministically() {
        let profile = GroundProfile::dev();
        let field = profile.fill(2, 2);
        for x in 0..2 {
            for y in 0..2 {
                // Every surface cell carries the loam topsoil.
                assert_eq!(field.volume(Coord3 { x, y, z: 0 }, "loam"), Fixed::ONE);
                // Every bedrock cell (z in -1..-3) carries granite.
                for z in [-1, -2, -3] {
                    assert_eq!(
                        field.volume(Coord3 { x, y, z }, "granite"),
                        Fixed::ONE,
                        "granite at z={z}"
                    );
                }
                // The ore band at z=-2 is a mixed cell: granite plus hematite.
                assert_eq!(field.volume(Coord3 { x, y, z: -2 }, "hematite"), Fixed::ONE);
                let ore = field
                    .cell(Coord3 { x, y, z: -2 })
                    .expect("the ore cell holds matter");
                assert_eq!(ore.total_volume(), Fixed::from_int(2));
            }
        }
        // No matter above the surface or below the profile.
        assert!(field.cell(Coord3 { x: 0, y: 0, z: 1 }).is_none());
        assert!(field.cell(Coord3 { x: 0, y: 0, z: -4 }).is_none());
        // Deterministic: a second fill folds to an identical hash.
        let (mut a, mut b) = (StateHasher::new(), StateHasher::new());
        field.hash_into(&mut a);
        profile.fill(2, 2).hash_into(&mut b);
        assert_eq!(a.finish(), b.finish());
    }

    #[test]
    fn a_filled_ground_derives_real_densities_from_the_ground_registry() {
        // End to end: the substances the profile places read their cited densities off the world ground
        // registry, and a mixed cell reads the volume-weighted mean between them.
        let reg = PhysicsRegistry::ground().expect("the ground registry loads");
        let field = GroundProfile::dev().fill(1, 1);
        // A pure bedrock cell reads granite's own cited density.
        assert_eq!(
            field.bulk_density(Coord3 { x: 0, y: 0, z: -1 }, &reg),
            Fixed::from_int(2700)
        );
        // The ore band (granite 2700 plus hematite 5260, equal volume) reads their mean, 3980.
        assert_eq!(
            field.bulk_density(Coord3 { x: 0, y: 0, z: -2 }, &reg),
            Fixed::from_int(3980)
        );
        // The topsoil reads loam's cited density.
        assert_eq!(
            field.bulk_density(Coord3 { x: 0, y: 0, z: 0 }, &reg),
            Fixed::from_int(1400)
        );
    }

    #[test]
    fn a_carry_load_weighs_its_derived_mass_under_gravity() {
        let reg = floor();
        // A granite load of volume 2 has mass 2*2700 = 5400; at unit gravity it weighs 5400.
        let load = mix(&[("granite", 2)]);
        assert_eq!(load.mass(&reg), Fixed::from_int(5400));
        assert_eq!(
            load.weight(&reg, Fixed::ONE, Fixed::from_int(1_000_000)),
            Fixed::from_int(5400)
        );
        // Doubling the gravity doubles the weight (F = m g).
        assert_eq!(
            load.weight(&reg, Fixed::from_int(2), Fixed::from_int(1_000_000)),
            Fixed::from_int(10800)
        );
        // The physics force ceiling caps a heavy load's weight.
        assert_eq!(
            load.weight(&reg, Fixed::ONE, Fixed::from_int(1000)),
            Fixed::from_int(1000)
        );
        // An empty carrier weighs nothing.
        assert_eq!(
            SubstanceMix::new().weight(&reg, Fixed::ONE, Fixed::from_int(1000)),
            Fixed::ZERO
        );
    }
}
