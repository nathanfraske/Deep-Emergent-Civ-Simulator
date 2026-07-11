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
//! not the authored set. The one exception is the [`fundamentals`] module: the closed
//! table of CODATA fundamental constants, which ARE the one authored universal layer
//! the value-authoring line permits (distinct from any owner or per-world value).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod bignum;
pub mod compute;
pub mod fundamentals;
pub mod plan;
pub mod tier2;

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
    /// policy. A single pairwise add is deterministic. Note that a fold of `add` over a
    /// sequence is NOT order-independent under `Saturate`, because saturating addition
    /// is commutative but not associative (`(MAX + 1) - 1` saturates to `MAX - 1`,
    /// while `(MAX - 1) + 1` reaches `MAX`). To reduce a set of magnitudes use
    /// [`AbsoluteQuantity::sum`], which accumulates exactly and applies the policy once
    /// at read, so the canonical path stays order-independent. Panics on a quantity
    /// mismatch, which is a dimensional error.
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

    /// Reduce a sequence of magnitudes of one quantity to a single magnitude,
    /// accumulating exactly in 128-bit space and applying the quantity's overflow
    /// policy once, at read. The result is therefore independent of the order the
    /// magnitudes arrive in, which is what a fold of [`AbsoluteQuantity::add`] cannot
    /// guarantee under `Saturate`, and it is the canonical reduction the determinism
    /// harness relies on (the clamp-at-read discipline of the evidence engine and the
    /// order-independent reductions of design Part 57, R-REDUCE-ORDER). An empty input
    /// is the zero magnitude of the quantity. Panics on a quantity mismatch.
    ///
    /// The 128-bit accumulator holds the exact sum of any realistic number of `i64`
    /// terms (over 18 quintillion of them before it could overflow), so the only
    /// rounding is the single policy step at the end.
    pub fn sum(
        quantity: u32,
        items: impl IntoIterator<Item = AbsoluteQuantity>,
        reg: &QuantityRegistry,
    ) -> AbsoluteQuantity {
        let def = reg.get(quantity).expect("unknown quantity");
        let mut acc: i128 = 0;
        for it in items {
            assert_eq!(
                it.quantity, quantity,
                "cannot sum magnitudes of different quantities"
            );
            acc += it.bits as i128;
        }
        let bits = match def.overflow {
            // Clamp once, at read: the exact total is order-independent, so its clamp is.
            OverflowPolicy::Saturate => acc.clamp(i64::MIN as i128, i64::MAX as i128) as i64,
            // The wrapped total is the low 64 bits of the exact sum, also order-independent.
            OverflowPolicy::Wrap => acc as i64,
        };
        AbsoluteQuantity { quantity, bits }
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
        rescale_bits(self.bits, from_def.scale_bits, to_def.scale_bits)
            .map(|bits| AbsoluteQuantity { quantity: to, bits })
    }
}

/// Integer division rounded to nearest, ties to even, for a positive divisor. The
/// same rule the canonical quantizer uses, so rescaling is deterministic. Shared with the
/// Tier-2 scaled arithmetic (`crate::tier2`), which rounds ONCE per result through it.
pub(crate) fn idiv_round_half_even(num: i128, den: i128) -> i128 {
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

/// Rescale a raw fixed-point magnitude from one scale (fractional-bit count) to another: up-scale by
/// a checked left shift, down-scale by a round-half-to-even division, in 128-bit space, returning
/// `None` on an out-of-`i64` result. This is the one raw-bit primitive the whole quantity system
/// rescales through: it is the bridge from a `Fixed` value (its scale is its `FRAC_BITS`) to an
/// [`AbsoluteQuantity`] at a quantity's scale and back, and the engine of [`AbsoluteQuantity::checked_convert`].
/// A `scale_bits == to == from` rescale is the identity, so a quantity stored at the canonical
/// thirty-two fractional bits bridges to and from `Fixed` with no change.
pub fn rescale_bits(bits: i64, from_scale_bits: u32, to_scale_bits: u32) -> Option<i64> {
    let s1 = from_scale_bits as i32;
    let s2 = to_scale_bits as i32;
    let out: i128 = if s2 >= s1 {
        // Up-scale by a left shift. A non-zero value shifted by 63 or more already exceeds the i64
        // range, so report it out of range rather than overflow the i128 intermediate.
        let shift = (s2 - s1) as u32;
        if bits == 0 {
            0
        } else if shift >= 63 {
            return None;
        } else {
            (bits as i128) << shift
        }
    } else {
        // Down-scale by a rounded division; bound the shift so the divisor stays a positive power of
        // two, past which the result rounds to zero anyway.
        let shift = (s1 - s2) as u32;
        if shift >= 127 {
            return None;
        }
        idiv_round_half_even(bits as i128, 1i128 << shift)
    };
    if out < i64::MIN as i128 || out > i64::MAX as i128 {
        None
    } else {
        Some(out as i64)
    }
}

/// The scale a quantity's envelope derives to, and whether the envelope had to be windowed (its
/// low-end significance reduced below the target) to fit the sixty-three magnitude bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DerivedScale {
    /// The fractional-bit count the quantity is stored at.
    pub scale_bits: u32,
    /// True when the sixty-three-bit budget could not hold the full significance target, so the
    /// low end carries fewer significant bits than requested (the capacitance and resistivity case).
    pub windowed: bool,
}

/// Derive a quantity's fixed-point scale from its declared envelope. The mechanism is fixed; the
/// envelope and the targets are the owner's reserved numbers (R-UNITS-PIN), provided by the caller:
/// `hi_log2` is the floor-base-2 logarithm of the envelope's largest bound magnitude, `lo_log2` the
/// floor-base-2 logarithm of its smallest non-zero bound magnitude (both negative for a value below
/// one, and computed from the physical decimal envelope by the caller, since a bound like `1e-12`
/// underflows the canonical fixed-point epsilon and cannot round-trip through a `Fixed`); `sig_target`
/// is the significant bits the low end must retain; `guard` is the integer headroom above the top;
/// and `canonical_scale` is the default scale a fitting quantity keeps (thirty-two for the physics
/// Q32.32 substrate). The rule (design.md Part 55, the units proposal): the scale defaults to
/// `canonical_scale` when the top fits its integer field and `canonical_scale` fractional bits
/// already resolve the bottom to `sig_target` significant bits; otherwise a wide envelope derives a
/// scale that holds the top and gives the bottom as much significance as the sixty-three-bit budget
/// allows, reducing the significance target (`windowed`) when even that will not fit. The crate
/// authors no scale; it computes one from the owner's envelope and targets.
pub fn derive_scale_bits(
    hi_log2: i32,
    lo_log2: i32,
    sig_target: u32,
    guard: u32,
    canonical_scale: u32,
) -> DerivedScale {
    const MAG_BITS: u32 = 63; // one sign bit, sixty-three magnitude bits in i64
    let default_int_bits = MAG_BITS.saturating_sub(canonical_scale); // 31 for the canonical Q32.32

    // Integer bits to hold the top's integer part (none when the top is below one), plus the guard.
    // Saturating throughout so a non-physical extreme argument returns a bounded result rather than
    // panicking or wrapping; real envelope log2 bounds and a small guard never approach these limits.
    let integer_bits = (hi_log2.saturating_add(1).max(0) as u32).saturating_add(guard);
    // Significant bits of the bottom at scale f are about lo_log2 + f, so f >= sig_target - lo_log2.
    // Computed in i64 so the u32-minus-i32 cannot overflow.
    let frac_needed = (sig_target as i64 - lo_log2 as i64).clamp(0, MAG_BITS as i64) as u32;

    if integer_bits <= default_int_bits && frac_needed <= canonical_scale {
        return DerivedScale {
            scale_bits: canonical_scale,
            windowed: false,
        };
    }

    let budget = MAG_BITS.saturating_sub(integer_bits);
    let scale_bits = frac_needed.min(budget).min(62);
    // Windowed when the low end carries fewer significant bits than requested, which is exactly when
    // the final scale falls below what the significance target needed (whether the 63-bit budget or
    // the 62-bit cap is what bound it), or the top alone overruns the magnitude bits.
    let windowed = integer_bits > MAG_BITS || frac_needed > scale_bits;
    DerivedScale {
        scale_bits,
        windowed,
    }
}

impl AbsoluteQuantity {
    /// Bridge a `Fixed` value (whose scale is `fixed_frac_bits`) to this quantity's scale, or `None`
    /// on an out-of-range result. For a quantity stored at `fixed_frac_bits` this is the identity, so
    /// the canonical Q32.32 quantities cross with no change.
    pub fn from_fixed_bits(
        quantity: u32,
        fixed_bits: i64,
        fixed_frac_bits: u32,
        reg: &QuantityRegistry,
    ) -> Option<AbsoluteQuantity> {
        let def = reg.get(quantity)?;
        rescale_bits(fixed_bits, fixed_frac_bits, def.scale_bits)
            .map(|bits| AbsoluteQuantity { quantity, bits })
    }

    /// Bridge this quantity's magnitude back to a `Fixed` value's raw bits at `fixed_frac_bits`, or
    /// `None` on an out-of-range result.
    pub fn to_fixed_bits(self, fixed_frac_bits: u32, reg: &QuantityRegistry) -> Option<i64> {
        let def = reg.get(self.quantity)?;
        rescale_bits(self.bits, def.scale_bits, fixed_frac_bits)
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

    #[test]
    fn derive_scale_bits_defaults_to_the_canonical_and_derives_for_wide_envelopes() {
        // A quantity whose envelope fits Q32.32 keeps scale 32 (modulus [1, 1.2e6], P=16, guard=1).
        assert_eq!(
            derive_scale_bits(20, 0, 16, 1, 32),
            DerivedScale {
                scale_bits: 32,
                windowed: false
            }
        );
        // A symmetric signed envelope keeps 32: no tiny low bound (potential [-1e8, 1e8]).
        assert_eq!(derive_scale_bits(26, 26, 16, 1, 32).scale_bits, 32);
        // Charge [1e-9, 1e5] does not fit: the low end forces Q17.46, the design-of-record's worked
        // case, without windowing.
        assert_eq!(
            derive_scale_bits(16, -30, 16, 0, 32),
            DerivedScale {
                scale_bits: 46,
                windowed: false
            }
        );
        // Capacitance [1e-12, 1e3] exceeds the 63-bit budget at P=16, so it windows to Q10.53 (the
        // low end carries about thirteen significant bits, fewer than the sixteen requested).
        assert_eq!(
            derive_scale_bits(9, -40, 16, 0, 32),
            DerivedScale {
                scale_bits: 53,
                windowed: true
            }
        );
        // A truly over-wide envelope stays bounded and is flagged windowed.
        let wide = derive_scale_bits(40, -27, 16, 0, 32);
        assert!(wide.windowed && wide.scale_bits <= 62);
        // Windowed is flagged even when the 62-bit fractional cap (not the budget) is what drops the
        // low end below the target: here scale caps at 62 but the target wanted 63 fractional bits.
        let capped = derive_scale_bits(-1, -47, 16, 0, 32);
        assert_eq!(
            capped,
            DerivedScale {
                scale_bits: 62,
                windowed: true
            }
        );
        // Non-physical extreme arguments return a bounded result rather than panicking or wrapping.
        let ex = derive_scale_bits(i32::MAX, i32::MIN, u32::MAX, u32::MAX, 32);
        assert!(ex.scale_bits <= 62);
    }

    #[test]
    fn rescale_bits_bridges_fixed_scales_and_is_identity_at_the_same_scale() {
        let five = 5i64 << 32; // 5.0 as Q32.32 raw
        assert_eq!(
            rescale_bits(five, 32, 32),
            Some(five),
            "same scale is identity"
        );
        assert_eq!(
            rescale_bits(five, 32, 46),
            Some(5i64 << 46),
            "up-scale is a left shift"
        );
        assert_eq!(
            rescale_bits(5i64 << 46, 46, 32),
            Some(five),
            "down-scale round-trips a representable value"
        );
        // A shift past the i64 range is reported, not wrapped.
        assert_eq!(rescale_bits(1, 0, 63), None);
    }

    #[test]
    fn the_fixed_bridge_is_a_no_op_for_a_canonical_quantity() {
        let mut q = QuantityRegistry::new();
        let q32 = q.register(QuantityDef {
            name: "at32".to_string(),
            dimension: Dimension::dimensionless(),
            scale_bits: 32,
            overflow: OverflowPolicy::Saturate,
        });
        let q46 = q.register(QuantityDef {
            name: "at46".to_string(),
            dimension: Dimension::dimensionless(),
            scale_bits: 46,
            overflow: OverflowPolicy::Saturate,
        });
        let five = 5i64 << 32;
        // A canonical Q32.32 quantity bridges to and from Fixed unchanged.
        let a = AbsoluteQuantity::from_fixed_bits(q32, five, 32, &q).unwrap();
        assert_eq!(a.bits, five);
        assert_eq!(a.to_fixed_bits(32, &q), Some(five));
        // A finer-scaled quantity holds a value the Fixed scale underflows: charge ~1e-9 at scale 46
        // is ~70369 raw, but through the Q32.32 bridge it degrades to ~4 raw (why the finer scale
        // exists), and re-deriving from that coarse value loses the low bits.
        let charge_raw_46 = 70369i64; // round(1e-9 * 2^46)
        let c = AbsoluteQuantity::new(q46, charge_raw_46);
        let through_fixed = c.to_fixed_bits(32, &q).unwrap();
        assert_eq!(
            through_fixed, 4,
            "1e-9 underflows the Q32.32 grid to ~4 raw"
        );
        let back = AbsoluteQuantity::from_fixed_bits(q46, through_fixed, 32, &q).unwrap();
        assert!(
            back.bits < charge_raw_46,
            "round-tripping through Q32.32 loses charge's low end"
        );
    }
}
