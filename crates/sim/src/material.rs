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
    /// The working-edge contact AREA (m^2) a knapped tool presents, its intrinsic geometry, over which a
    /// wielded tool concentrates a being's force ([`WieldedTool`]). RESERVED. Basis: the contact patch of
    /// the working edge a shaping stroke produces (a knapped point or blade tip), a tool-geometry datum the
    /// crafting-physics refinement will later derive from the fracture of the worked stone; a smaller edge
    /// concentrates the same force into a higher pressure and works harder matter. A performance-and-realism
    /// bound, surfaced for the owner, never invented.
    pub edge_area: Fixed,
    /// The VOLUME of carried matter a tool consumes to make (m^3). RESERVED. Basis: the material a shaped
    /// tool of the working size embodies; a being that carries less than this cannot make the tool. A
    /// geometry-and-scale datum, surfaced for the owner.
    pub tool_volume: Fixed,
}

impl CraftParams {
    /// A labelled DEVELOPMENT FIXTURE: a small working edge and a modest tool volume. Not owner canon; a
    /// stand-in so the crafting contest can run until the owner sets the values against their bases.
    pub fn dev_fixture() -> CraftParams {
        CraftParams {
            edge_area: Fixed::from_ratio(1, 1_000_000),
            tool_volume: Fixed::from_int(1),
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
    /// The substance the tool is made of; its hardness (the pressure it sustains before it blunts) and its
    /// other properties are read from the [`PhysicsRegistry`] by this id.
    pub substance: String,
}

impl WieldedTool {
    /// Fold the tool into a hash, its geometry then its substance id, in canonical order (the material fold
    /// discipline). Called by the runner's per-walker `state_hash` fold when a being wields a tool; a being
    /// with no tool folds nothing, so the wielded slot is opt-in and hash-neutral by default.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_fixed(self.contact_area);
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
