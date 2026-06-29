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

//! # civsim-units: the absolute unit and dimensional system (design Part 55)
//!
//! This crate is the foundation the runbook lists as buildable now and the
//! structure the R-UNITS-PIN flag calls for. It carries the *mechanism* only:
//!
//! - A base-dimension registry. The set of base dimensions is data the owner and
//!   the physics fan-out provide (R-DEEPTECH-PHYSICS), not authored here, because
//!   the physics catalogue is the one authored layer and is the owner's to populate
//!   (Principle 9). A [`Dimension`] is a vector of integer exponents over those base
//!   dimensions, kept in a canonical sorted form, so a derived dimension (force,
//!   energy) is a computed composition rather than an authored entry, and every
//!   quantity mechanically reduces to base dimensions, which is the descriptor
//!   neutrality the steering audit wants.
//! - A quantity registry. Each quantity carries its dimension, its per-quantity
//!   fixed-point scale, and an explicit saturate-or-wrap overflow policy. The scales
//!   are the owner's reserved numbers, provided in data; the crate ships none.
//! - Deterministic integer arithmetic and conversion. Magnitudes are `i64` at a
//!   quantity's scale. No floating point appears anywhere, so nothing here can
//!   perturb a canonical result, and overflow follows the quantity's declared
//!   policy rather than an accident (the discipline Part 55 requires).
//!
//! What this crate deliberately does not contain: any base dimension, any quantity,
//! or any scale. Those are the authored physics catalogue and the owner's reserved
//! values; the tests use a small fixture catalogue, clearly marked as a fixture and
//! not the authored set.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An exponent on a base dimension. Small signed integer; real physical dimensions
/// stay well within this range.
pub type DimExp = i8;

/// A dimension as a canonical, sorted vector of `(base index, exponent)` terms with
/// no zero exponents. Two dimensions are equal exactly when their canonical vectors
/// are equal, so dimensional checks are exact and deterministic.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Dimension {
    terms: Vec<(u16, DimExp)>,
}

impl Dimension {
    /// The dimensionless quantity (a pure number, a ratio).
    pub fn dimensionless() -> Self {
        Dimension { terms: Vec::new() }
    }

    /// A single base dimension raised to the first power.
    pub fn base(index: u16) -> Self {
        Dimension {
            terms: vec![(index, 1)],
        }
    }

    /// Build a dimension from arbitrary terms, reducing to canonical form: like
    /// indices are combined, zero exponents dropped, and the result sorted by index.
    pub fn from_terms(terms: impl IntoIterator<Item = (u16, DimExp)>) -> Self {
        let mut acc: HashMap<u16, i32> = HashMap::new();
        for (idx, exp) in terms {
            *acc.entry(idx).or_insert(0) += exp as i32;
        }
        let mut v: Vec<(u16, DimExp)> = acc
            .into_iter()
            .filter(|(_, e)| *e != 0)
            .map(|(idx, e)| (idx, e as DimExp))
            .collect();
        v.sort_by_key(|(idx, _)| *idx);
        Dimension { terms: v }
    }

    /// Whether this is the dimensionless quantity.
    pub fn is_dimensionless(&self) -> bool {
        self.terms.is_empty()
    }

    /// The product of two dimensions (exponents add): the dimension of a product of
    /// two quantities.
    pub fn mul(&self, other: &Dimension) -> Dimension {
        Dimension::from_terms(self.terms.iter().chain(other.terms.iter()).copied())
    }

    /// The reciprocal dimension (exponents negate).
    pub fn inv(&self) -> Dimension {
        Dimension::from_terms(self.terms.iter().map(|(i, e)| (*i, -e)))
    }

    /// The quotient of two dimensions.
    pub fn div(&self, other: &Dimension) -> Dimension {
        self.mul(&other.inv())
    }

    /// The canonical terms, for inspection and hashing.
    pub fn terms(&self) -> &[(u16, DimExp)] {
        &self.terms
    }
}

/// The ordered set of base dimensions. The membership is data (the owner's authored
/// physics catalogue under Principle 9), so this registry is populated from a world
/// definition rather than hardcoded. The index of a base dimension is its position,
/// which the [`Dimension`] exponent vectors reference.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BaseDimensionRegistry {
    names: Vec<String>,
}

impl BaseDimensionRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        BaseDimensionRegistry::default()
    }

    /// Register a base dimension by name, returning its index. Re-registering an
    /// existing name returns its existing index, so the registry is idempotent.
    pub fn register(&mut self, name: &str) -> u16 {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return i as u16;
        }
        let i = self.names.len();
        assert!(i <= u16::MAX as usize, "base-dimension space exhausted");
        self.names.push(name.to_string());
        i as u16
    }

    /// The index of a registered base dimension.
    pub fn index_of(&self, name: &str) -> Option<u16> {
        self.names.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// The name of a base dimension by index.
    pub fn name(&self, index: u16) -> Option<&str> {
        self.names.get(index as usize).map(|s| s.as_str())
    }

    /// Number of base dimensions.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// What to do when a quantity's accumulation would exceed its fixed-point range.
/// This is an engine mechanic (how the simulation runs), so it is a closed choice;
/// which policy a given quantity uses is data (a field on its definition).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OverflowPolicy {
    /// Clamp to the representable bound. The default discipline for a quantity a gate
    /// thresholds, so a runaway accumulation reads as the bound rather than wrapping.
    Saturate,
    /// Wrap modularly. Only for a quantity whose modular accumulation is intended.
    Wrap,
}

/// A quantity definition: its dimension, its per-quantity fixed-point scale (the
/// number of fractional bits), and its overflow policy. The scale is the owner's
/// reserved number, provided in data (R-UNITS-PIN); the crate authors none.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct QuantityDef {
    /// Stable name within the catalogue.
    pub name: String,
    /// The quantity's dimension as a base-exponent vector.
    pub dimension: Dimension,
    /// Fractional bits of the fixed-point representation for this quantity.
    pub scale_bits: u32,
    /// The overflow discipline for accumulation.
    pub overflow: OverflowPolicy,
}

/// A registry of quantity definitions, indexed by id (registration order) and by
/// name. Iteration for any canonical purpose uses id order, which is deterministic.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QuantityRegistry {
    defs: Vec<QuantityDef>,
    #[serde(skip)]
    by_name: HashMap<String, u32>,
}

impl QuantityRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        QuantityRegistry::default()
    }

    /// Register a quantity definition, returning its id. Panics on a duplicate name,
    /// so the catalogue cannot define one quantity two ways.
    pub fn register(&mut self, def: QuantityDef) -> u32 {
        assert!(
            !self.by_name.contains_key(&def.name),
            "duplicate quantity '{}'",
            def.name
        );
        let id = self.defs.len() as u32;
        self.by_name.insert(def.name.clone(), id);
        self.defs.push(def);
        id
    }

    /// Rebuild the name index after a deserialize (the index is not serialized).
    pub fn reindex(&mut self) {
        self.by_name.clear();
        for (i, d) in self.defs.iter().enumerate() {
            self.by_name.insert(d.name.clone(), i as u32);
        }
    }

    /// The definition for an id.
    pub fn get(&self, id: u32) -> Option<&QuantityDef> {
        self.defs.get(id as usize)
    }

    /// The id of a quantity by name.
    pub fn id_of(&self, name: &str) -> Option<u32> {
        self.by_name.get(name).copied()
    }

    /// Number of quantities.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

/// A magnitude of a specific quantity, stored as an `i64` at that quantity's scale.
/// Integer only, so it is exact and deterministic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbsoluteQuantity {
    /// The quantity id in a [`QuantityRegistry`].
    pub quantity: u32,
    /// The magnitude in the quantity's fixed-point units.
    pub bits: i64,
}

impl AbsoluteQuantity {
    /// Construct a magnitude of a quantity.
    pub fn new(quantity: u32, bits: i64) -> Self {
        AbsoluteQuantity { quantity, bits }
    }

    /// Add two magnitudes of the same quantity, applying that quantity's overflow
    /// policy. Both saturate and wrap are deterministic and order-independent for a
    /// pairwise add. Panics on a quantity mismatch, which is a dimensional error.
    pub fn add(self, other: AbsoluteQuantity, reg: &QuantityRegistry) -> AbsoluteQuantity {
        assert_eq!(
            self.quantity, other.quantity,
            "cannot add magnitudes of different quantities"
        );
        let def = reg.get(self.quantity).expect("unknown quantity");
        let bits = match def.overflow {
            OverflowPolicy::Saturate => self.bits.saturating_add(other.bits),
            OverflowPolicy::Wrap => self.bits.wrapping_add(other.bits),
        };
        AbsoluteQuantity {
            quantity: self.quantity,
            bits,
        }
    }

    /// Convert to another quantity of the same dimension, rescaling the magnitude
    /// from the source scale to the target scale with round-half-to-even, in 128-bit
    /// space so the rescale never overflows mid-way. Returns `None` on a dimension
    /// mismatch or an out-of-range result, so the crossing is checked, not silent.
    pub fn checked_convert(self, to: u32, reg: &QuantityRegistry) -> Option<AbsoluteQuantity> {
        let from_def = reg.get(self.quantity)?;
        let to_def = reg.get(to)?;
        if from_def.dimension != to_def.dimension {
            return None;
        }
        let s1 = from_def.scale_bits as i32;
        let s2 = to_def.scale_bits as i32;
        let bits = if s2 >= s1 {
            (self.bits as i128) << (s2 - s1)
        } else {
            idiv_round_half_even(self.bits as i128, 1i128 << (s1 - s2))
        };
        if bits < i64::MIN as i128 || bits > i64::MAX as i128 {
            return None;
        }
        Some(AbsoluteQuantity {
            quantity: to,
            bits: bits as i64,
        })
    }
}

/// Integer division rounded to nearest, ties to even, for a positive divisor. The
/// same rule the canonical quantizer uses, so rescaling is deterministic.
fn idiv_round_half_even(num: i128, den: i128) -> i128 {
    debug_assert!(den > 0);
    let q = num.div_euclid(den);
    let r = num.rem_euclid(den);
    let twice = r * 2;
    if twice < den {
        q
    } else if twice > den {
        q + 1
    } else if q % 2 == 0 {
        q
    } else {
        q + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A small FIXTURE catalogue, not the authored physics set. It exists only to
    // exercise the mechanism; the real base dimensions, quantities, and scales are
    // data the owner provides (R-UNITS-PIN, R-DEEPTECH-PHYSICS).
    fn fixture() -> (BaseDimensionRegistry, QuantityRegistry, u32, u32, u32) {
        let mut base = BaseDimensionRegistry::new();
        let length = base.register("length");
        let mass = base.register("mass");
        let time = base.register("time");

        let mut q = QuantityRegistry::new();
        let dist = q.register(QuantityDef {
            name: "distance".to_string(),
            dimension: Dimension::base(length),
            scale_bits: 16,
            overflow: OverflowPolicy::Saturate,
        });
        // force = mass * length / time^2, a computed composition, never authored.
        let force_dim = Dimension::from_terms([(mass, 1), (length, 1), (time, -2)]);
        let force = q.register(QuantityDef {
            name: "force".to_string(),
            dimension: force_dim,
            scale_bits: 16,
            overflow: OverflowPolicy::Saturate,
        });
        // a second distance quantity at a finer scale, same dimension, for conversion.
        let dist_mm = q.register(QuantityDef {
            name: "distance_fine".to_string(),
            dimension: Dimension::base(length),
            scale_bits: 24,
            overflow: OverflowPolicy::Saturate,
        });
        (base, q, dist, force, dist_mm)
    }

    #[test]
    fn dimension_is_canonical_regardless_of_term_order() {
        let a = Dimension::from_terms([(2, -2), (0, 1), (1, 1)]);
        let b = Dimension::from_terms([(1, 1), (0, 1), (2, -2)]);
        assert_eq!(a, b, "term order does not change the canonical dimension");
        // combining like terms and dropping zeros.
        let c = Dimension::from_terms([(0, 1), (0, 1), (3, 2), (3, -2)]);
        assert_eq!(c, Dimension::from_terms([(0, 2)]));
    }

    #[test]
    fn dimension_algebra_composes() {
        let (_b, _q, _d, _f, _m) = fixture();
        let length = 0u16;
        let time = 2u16;
        let velocity = Dimension::base(length).div(&Dimension::base(time));
        let accel = velocity.div(&Dimension::base(time));
        assert_eq!(accel, Dimension::from_terms([(length, 1), (time, -2)]));
        assert!(Dimension::base(length)
            .div(&Dimension::base(length))
            .is_dimensionless());
    }

    #[test]
    fn add_rejects_a_dimension_mismatch() {
        let (_b, q, dist, force, _m) = fixture();
        let a = AbsoluteQuantity::new(dist, 100);
        let f = AbsoluteQuantity::new(force, 100);
        let caught = std::panic::catch_unwind(|| a.add(f, &q));
        assert!(caught.is_err(), "adding distance to force must panic");
    }

    #[test]
    fn add_applies_the_overflow_policy_deterministically() {
        let (_b, q, dist, _f, _m) = fixture();
        let big = AbsoluteQuantity::new(dist, i64::MAX);
        let one = AbsoluteQuantity::new(dist, 1);
        // distance saturates, so it stays at the bound rather than wrapping.
        assert_eq!(big.add(one, &q).bits, i64::MAX);
    }

    #[test]
    fn convert_rescales_within_the_same_dimension() {
        let (_b, q, dist, _f, dist_mm) = fixture();
        // 5 units at scale 16 -> scale 24 is an exact left shift.
        let coarse = AbsoluteQuantity::new(dist, 5i64 << 16);
        let fine = coarse.checked_convert(dist_mm, &q).unwrap();
        assert_eq!(fine.bits, 5i64 << 24);
        // and back again is exact here.
        let back = fine.checked_convert(dist, &q).unwrap();
        assert_eq!(back.bits, coarse.bits);
    }

    #[test]
    fn convert_rejects_a_dimension_mismatch() {
        let (_b, q, dist, force, _m) = fixture();
        let a = AbsoluteQuantity::new(dist, 1 << 16);
        assert!(a.checked_convert(force, &q).is_none());
    }

    #[test]
    fn reindex_rebuilds_the_name_lookup_after_a_deserialize() {
        // The name index is not serialized, so a freshly deserialized registry has an
        // empty lookup until reindex runs. Simulate that post-deserialize state by
        // clearing the skipped field, then prove reindex restores the lookup.
        let (_b, q, _d, _f, _m) = fixture();
        let mut back = q.clone();
        back.by_name.clear();
        assert!(back.id_of("force").is_none(), "lookup empty before reindex");
        back.reindex();
        assert_eq!(back.id_of("force"), q.id_of("force"));
        assert_eq!(back.id_of("distance"), q.id_of("distance"));
        assert_eq!(back.len(), q.len());
    }
}
