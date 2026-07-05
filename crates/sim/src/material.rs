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
use civsim_physics::PhysicsRegistry;
use civsim_world::Coord3;

/// The bulk mass-per-unit-volume axis of the mechanical floor (`mechanical_floor.toml`), read to
/// derive a cell's density and the mass its matter carries.
const AXIS_DENSITY: &str = "mat.density";

/// The contact-pressure-a-surface-resists axis of the mechanical floor, read to derive the hardness an
/// extraction contest works against.
const AXIS_HARDNESS: &str = "mat.indentation_hardness";

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

[[substance]]
id = "granite"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "2700" },
  { axis = "mat.indentation_hardness", value = "5000" },
]

[[substance]]
id = "soil"
participates_in = []
real = "test fixture"
values = [
  { axis = "mat.density", value = "1500" },
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
    fn an_unregistered_substance_and_an_empty_cell_read_zero() {
        let reg = floor();
        let mystery = mix(&[("orichalcum", 5)]);
        assert_eq!(mystery.bulk_density(&reg), Fixed::ZERO);
        assert_eq!(mystery.bulk_hardness(&reg), Fixed::ZERO);
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
}
