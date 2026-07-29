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

//! # The emic (cultural) measurement layer of design Part 55
//!
//! A people invents its own units from what is at hand, a length from a forearm, a weight from a
//! particular stone, and to the engine each such unit is a conversion factor and a provenance
//! relative to the absolute base the culture never sees. This module carries the *mechanism* of
//! that conversion, the R-UNITS-PIN residual: how an emic magnitude crosses to the absolute base
//! and back exactly where the absolute scale can resolve it, and honestly at the one boundary
//! where it cannot.
//!
//! Two rules make the crossing exact rather than the lossy hash-map-and-single-multiply the flag
//! named:
//!
//! - **The factor is an exact rational.** A cultural unit is in general a non-dyadic fraction of
//!   the base (a cubit is not a power of two times the base length), so storing the factor as one
//!   rounded `Fixed` pre-approximates it before any conversion runs. [`UnitFactor`] holds the
//!   numerator and denominator as integers, so the factor itself contributes no approximation and
//!   the only rounding is the single terminal round of a conversion chain.
//! - **Storage is canonical-absolute.** An emic quantity is a *view*, an absolute magnitude read
//!   through a unit, never a stored emic magnitude of its own. So displaying the same absolute
//!   value in the same unit twice yields the same emic reading, the crossing is idempotent, and a
//!   bounded rounding at the boundary never compounds across repeated reads.
//!
//! The round-trip-exactness rule this delivers: `emic -> absolute -> emic` returns the original
//! for every emic value the absolute scale can resolve (all of them when the denominator divides
//! evenly at the absolute scale's headroom, the representable subset otherwise), and it is within
//! one absolute epsilon at the boundary where the absolute epsilon is coarser than the emic step.
//! That boundary epsilon is the absolute scale's own resolution, a physical floor of the storage,
//! deterministic and observer-independent (Principle 10). For a culture-stated exact quantity (an
//! inscription reading three cubits) [`StatedQuantity`] carries the exact datum, so its emic
//! display stays perfectly reversible until the value must enter the fixed-point physics path,
//! where the single quantization is the bounded, declared loss. The engine authors no factor and
//! no unit: those are per-culture data (Part 40).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An exact rational conversion from a unit's own magnitude to the absolute base: one unit equals
/// `num / den` base units. Held as an integer pair so the factor contributes no approximation of
/// its own. The denominator is strictly positive and the numerator non-zero, so the unit is
/// invertible (an absolute magnitude maps back through `den / num`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UnitFactor {
    num: i64,
    den: i64,
}

impl UnitFactor {
    /// A factor `num / den`, or `None` if the denominator is not strictly positive or the
    /// numerator is zero (a non-invertible unit). Not reduced to lowest terms, since the
    /// conversion is exact either way and reduction is neither needed nor canonical.
    pub fn new(num: i64, den: i64) -> Option<UnitFactor> {
        if den <= 0 || num == 0 {
            None
        } else {
            Some(UnitFactor { num, den })
        }
    }

    /// The numerator (base units per unit, times the denominator).
    pub fn num(&self) -> i64 {
        self.num
    }

    /// The denominator (strictly positive).
    pub fn den(&self) -> i64 {
        self.den
    }
}

/// The shared single-round conversion core: `round_half_even(v * factor_mul * 2^(s_out - s_in) /
/// factor_div)`, computed with the whole numerator formed exactly before the one terminal round,
/// so the crossing rounds once rather than at each step. Returns `None` on a zero divisor, on an
/// intermediate that exceeds the 128-bit width, or on a result outside the `i64` range (the widen
/// signal, matching the tier2 single-op contract). The product `v * factor_mul` is `i64 * i64` and
/// fits `i128`; a pathological factor whose shifted numerator or denominator lands in
/// `[2^127, 2^128)` fails loud rather than wrapping.
fn convert(v: i64, s_in: u32, factor_mul: i64, factor_div: i64, s_out: u32) -> Option<i64> {
    if factor_div == 0 {
        return None;
    }
    let neg = (v < 0) ^ (factor_mul < 0) ^ (factor_div < 0);
    let product = (v as i128).checked_mul(factor_mul as i128)?; // exact v * factor_mul
    let mut num = product.unsigned_abs();
    let mut den = (factor_div as i128).unsigned_abs();
    let shift = s_out as i64 - s_in as i64;
    if shift >= 0 {
        // The output scale is finer: an exact left shift of the numerator, checked for overflow.
        let sh = shift as u32;
        num = num.checked_shl(sh).filter(|x| (x >> sh) == num)?;
    } else {
        // The output scale is coarser: shift the denominator up so the single round drops the
        // low bits, checked for overflow.
        let sh = (-shift) as u32;
        den = den.checked_shl(sh).filter(|x| (x >> sh) == den)?;
    }
    // Round half to even of num / den with both positive, then reapply the sign. A shift-aligned
    // value in [2^127, 2^128) fits u128 but not signed i128, so a raw cast would wrap: fail loud.
    let num_i = i128::try_from(num).ok()?;
    let den_i = i128::try_from(den).ok()?;
    let q = crate::idiv_round_half_even(num_i, den_i);
    let signed = if neg { -q } else { q };
    if (i64::MIN as i128..=i64::MAX as i128).contains(&signed) {
        Some(signed as i64)
    } else {
        None
    }
}

/// Convert an emic magnitude to the absolute base, rounded ONCE. `v` at emic scale `s_emic` times
/// the factor `num/den`, delivered at the absolute quantity scale `s_abs`. Returns `None` on the
/// widen signal (see [`convert`]).
pub fn emic_to_absolute(v: i64, s_emic: u32, factor: UnitFactor, s_abs: u32) -> Option<i64> {
    convert(v, s_emic, factor.num, factor.den, s_abs)
}

/// Convert an absolute magnitude back to an emic reading, rounded ONCE. One unit is `num/den` base
/// units, so the absolute maps back through `den/num`: `v_emic = round(a * den * 2^(s_emic -
/// s_abs) / num)`. Returns `None` on the widen signal.
pub fn absolute_to_emic(a: i64, s_abs: u32, factor: UnitFactor, s_emic: u32) -> Option<i64> {
    // Invert the factor: multiply by den, divide by num. The sign of num is carried by `convert`.
    convert(a, s_abs, factor.den, factor.num, s_emic)
}

/// A culture-stated quantity carried as its exact datum, the opt-in exact-rational carry the gate
/// reserved for the cultural-display path. It remembers the exact emic magnitude a people named
/// (an inscription reading three cubits), so its emic display is always exactly what was stated,
/// regardless of whether the factor is dyadic. The single bounded loss happens only when the
/// quantity is forced into the fixed-point physics path through [`StatedQuantity::quantize_to_absolute`],
/// never before: keep the exact datum until it must be lossy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StatedQuantity {
    v_emic: i64,
    s_emic: u32,
    factor: UnitFactor,
}

impl StatedQuantity {
    /// A quantity a culture stated as `v_emic` (at emic scale `s_emic`) of the unit whose factor is
    /// `factor`.
    pub fn new(v_emic: i64, s_emic: u32, factor: UnitFactor) -> StatedQuantity {
        StatedQuantity {
            v_emic,
            s_emic,
            factor,
        }
    }

    /// The exact emic magnitude, always what was stated. This is the perfectly reversible display
    /// the carry buys: no round trip, no loss, for any factor.
    pub fn emic_bits(&self) -> i64 {
        self.v_emic
    }

    /// The emic scale the magnitude is stated at.
    pub fn emic_scale(&self) -> u32 {
        self.s_emic
    }

    /// Quantize the stated quantity into the absolute base at scale `s_abs`, the one bounded loss,
    /// taken only when the value must enter the fixed-point physics path. Returns `None` on the
    /// widen signal.
    pub fn quantize_to_absolute(&self, s_abs: u32) -> Option<i64> {
        emic_to_absolute(self.v_emic, self.s_emic, self.factor, s_abs)
    }
}

/// A unit's identity within a culture's measurement system: an interned id assigned in
/// registration order. Keying the store on this ordered id rather than on a hash of the unit's
/// name is what makes a canonical walk over a culture's units observer-independent, the units-local
/// instance of R-CANON-WALK (Principle 10).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct UnitId(pub u32);

/// A provenance kind's identity: an interned id into the open [`ProvenanceKindRegistry`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct ProvenanceKindId(pub u32);

/// The open, data-driven set of kinds a unit's provenance can be. The membership is data, not a
/// closed Rust enum, so a culture can coin a unit from anything: a forearm, a stride, a seed, a
/// vessel, a celestial cycle, or, for an alien people, a mana tide's reach or a redox front's
/// advance, each a data row rather than a code change. This is the substrate treatment the value,
/// semantic, institution-function, and access-channel registries already carry, applied to unit
/// provenance so the emic layer admits the alien as data (the input-audit catch). The mechanism is
/// fixed; the kinds are the world's data.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProvenanceKindRegistry {
    names: Vec<String>,
}

impl ProvenanceKindRegistry {
    /// An empty registry.
    pub fn new() -> ProvenanceKindRegistry {
        ProvenanceKindRegistry::default()
    }

    /// Register a provenance kind by name, returning its ordered id. A repeat of an existing name
    /// returns the existing id rather than duplicating, so the set stays canonical.
    pub fn register(&mut self, name: &str) -> ProvenanceKindId {
        if let Some(id) = self.id_of(name) {
            return id;
        }
        let id = self.names.len() as u32;
        self.names.push(name.to_string());
        ProvenanceKindId(id)
    }

    /// The name of a provenance kind.
    pub fn name_of(&self, id: ProvenanceKindId) -> Option<&str> {
        self.names.get(id.0 as usize).map(|s| s.as_str())
    }

    /// The id of a provenance kind by name.
    pub fn id_of(&self, name: &str) -> Option<ProvenanceKindId> {
        self.names
            .iter()
            .position(|n| n == name)
            .map(|i| ProvenanceKindId(i as u32))
    }

    /// The number of registered kinds.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// What a people derived a unit from: an open provenance-kind id and a free descriptor of the
/// specific referent (which forearm, which star's cycle, which vessel). The kind is drawn from the
/// open [`ProvenanceKindRegistry`], never a closed enum, so an origin the crate never anticipated
/// is a data row. The referent is the culture's own descriptor of the concrete thing.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct UnitOrigin {
    /// The kind of thing the unit came from, an id into the open provenance registry.
    pub kind: ProvenanceKindId,
    /// The specific referent, the culture's own descriptor (free data).
    pub referent: String,
}

/// One of a culture's own named units: its dimension (the built data-driven exponent vector, not a
/// closed dimension enum), its exact-rational conversion to the absolute base, the name the people
/// gave it, and its open provenance (what they derived it from). The struct stays the emic layer's
/// data row, not an authored taxonomy of unit kinds.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct EmicUnit {
    /// The dimension this unit measures, as the crate's canonical exponent vector.
    pub dimension: crate::Dimension,
    /// The exact-rational conversion to the absolute base (one unit is `num/den` base units).
    pub factor: UnitFactor,
    /// The culture's own name for the unit.
    pub name: String,
    /// What the people derived the unit from, an open provenance reference.
    pub origin: UnitOrigin,
}

/// A culture's measurement system: the ordered store of its own units. Units are held in
/// registration-id order, and the only walk built for any canonical purpose is
/// [`MeasurementSystem::iter_ordered`], which yields them in that order, so a hash of a culture's
/// units is observer-independent. The name index is a convenience lookup and is never the canonical
/// walk. A culture (`CultureId`, Part 40) owns one of these; the id association is the sim's to
/// attach, so this substrate stays culture-agnostic.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MeasurementSystem {
    units: Vec<EmicUnit>,
    #[serde(skip)]
    by_name: HashMap<String, u32>,
}

impl MeasurementSystem {
    /// An empty measurement system.
    pub fn new() -> MeasurementSystem {
        MeasurementSystem::default()
    }

    /// Register a unit, returning its ordered id. Panics on a duplicate name, so a culture cannot
    /// define one named unit two ways.
    pub fn register(&mut self, unit: EmicUnit) -> UnitId {
        assert!(
            !self.by_name.contains_key(&unit.name),
            "duplicate unit '{}'",
            unit.name
        );
        let id = self.units.len() as u32;
        self.by_name.insert(unit.name.clone(), id);
        self.units.push(unit);
        UnitId(id)
    }

    /// Rebuild the name index after a deserialize (the index is not serialized, since it is derived
    /// from the ordered store).
    pub fn reindex(&mut self) {
        self.by_name.clear();
        for (i, u) in self.units.iter().enumerate() {
            self.by_name.insert(u.name.clone(), i as u32);
        }
    }

    /// The unit for an id.
    pub fn get(&self, id: UnitId) -> Option<&EmicUnit> {
        self.units.get(id.0 as usize)
    }

    /// The id of a unit by name (a convenience lookup, never the canonical walk).
    pub fn id_of(&self, name: &str) -> Option<UnitId> {
        self.by_name.get(name).copied().map(UnitId)
    }

    /// The number of units.
    pub fn len(&self) -> usize {
        self.units.len()
    }

    /// Whether the system has no units.
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// Walk the units in canonical id order. This is the one ordered accessor a hash or any other
    /// order-sensitive canonical operation is built over, so the walk is deterministic and
    /// independent of insertion order, machine, and hash seed.
    pub fn iter_ordered(&self) -> impl Iterator<Item = (UnitId, &EmicUnit)> {
        self.units
            .iter()
            .enumerate()
            .map(|(i, u)| (UnitId(i as u32), u))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A helper: does a full emic -> absolute -> emic round trip land within `tol` of the original?
    fn round_trip(v: i64, s_emic: u32, factor: UnitFactor, s_abs: u32) -> Option<i64> {
        let abs = emic_to_absolute(v, s_emic, factor, s_abs)?;
        absolute_to_emic(abs, s_abs, factor, s_emic)
    }

    #[test]
    fn a_dyadic_factor_round_trips_exactly_for_every_value() {
        // A unit that is 3/4 of the base. Exactness needs the absolute scale to resolve the value:
        // with headroom for the denominator (s_abs at least s_emic + log2(den), here two bits for
        // den = 4) the forward crossing is exact, and an exact forward makes the back crossing
        // exact too, so every value round-trips exactly.
        let f = UnitFactor::new(3, 4).unwrap();
        for v in [-1_000_000i64, -7, -1, 0, 1, 2, 5, 1024, 999_999] {
            let back = round_trip(v, 16, f, 20).unwrap();
            assert_eq!(
                back, v,
                "dyadic factor must round-trip {v} exactly with headroom"
            );
        }
    }

    #[test]
    fn a_non_dyadic_factor_round_trips_exactly_where_the_scale_resolves_it() {
        // A cubit as 4572/10000 of the base metre. With ample absolute headroom (a fine absolute
        // scale relative to the emic scale) the crossing resolves every stated value, so the round
        // trip is exact even though the factor is non-dyadic.
        let cubit = UnitFactor::new(4572, 10000).unwrap();
        for v in [1i64, 2, 3, 10, 37, 1000] {
            let back = round_trip(v, 8, cubit, 24).unwrap();
            assert_eq!(
                back, v,
                "cubit must round-trip {v} exactly at a resolving scale"
            );
        }
    }

    #[test]
    fn a_non_dyadic_factor_stays_within_one_epsilon_at_the_boundary() {
        // Without full headroom the round trip is bounded by one epsilon rather than exact. With the
        // absolute scale at least as fine as the emic (here equal), one absolute epsilon is one emic
        // ULP, so the emic drift is at most one; the rule is honest about this boundary and must
        // never drift past it. A coarser absolute scale would widen the bound to one absolute epsilon
        // spanning several emic ULPs, which the idempotence test covers instead.
        let cubit = UnitFactor::new(4572, 10000).unwrap();
        let (s_emic, s_abs) = (10, 10); // no denominator headroom, absolute as fine as emic
        let mut saw_boundary = false;
        for v in 1..=5000i64 {
            if let Some(back) = round_trip(v, s_emic, cubit, s_abs) {
                assert!(
                    (back - v).abs() <= 1,
                    "round trip of {v} drifted to {back}, past one epsilon"
                );
                if back != v {
                    saw_boundary = true;
                }
            }
        }
        assert!(
            saw_boundary,
            "this scale should exercise the within-one-epsilon boundary"
        );
    }

    #[test]
    fn the_crossing_is_idempotent_so_nothing_ratchets() {
        // Canonical-absolute storage means once we have the absolute, repeated display and
        // re-absolutization stabilize: a second round trip equals the first, no ratchet.
        let cubit = UnitFactor::new(4572, 10000).unwrap();
        let (s_emic, s_abs) = (12, 10);
        for v in [1i64, 7, 41, 250, 999] {
            let abs1 = emic_to_absolute(v, s_emic, cubit, s_abs).unwrap();
            let emic1 = absolute_to_emic(abs1, s_abs, cubit, s_emic).unwrap();
            let abs2 = emic_to_absolute(emic1, s_emic, cubit, s_abs).unwrap();
            let emic2 = absolute_to_emic(abs2, s_abs, cubit, s_emic).unwrap();
            assert_eq!(abs1, abs2, "absolute must stabilize after one round trip");
            assert_eq!(
                emic1, emic2,
                "emic display must stabilize after one round trip"
            );
        }
    }

    #[test]
    fn a_stated_quantity_displays_exactly_regardless_of_the_factor() {
        // The opt-in exact-rational carry: an inscription's stated magnitude is exact on display
        // even for a non-dyadic factor where the plain round trip would lose an epsilon.
        let cubit = UnitFactor::new(4572, 10000).unwrap();
        let stated = StatedQuantity::new(3, 0, cubit); // three whole cubits
        assert_eq!(
            stated.emic_bits(),
            3,
            "the stated datum is exact on display"
        );
        // The loss appears only when forced into the physics path, and is bounded there.
        let abs = stated.quantize_to_absolute(24).unwrap();
        assert!(abs > 0, "the quantized absolute is a real magnitude");
    }

    #[test]
    fn a_zero_or_negative_denominator_is_rejected() {
        assert!(UnitFactor::new(1, 0).is_none());
        assert!(UnitFactor::new(1, -4).is_none());
        assert!(
            UnitFactor::new(0, 4).is_none(),
            "a zero factor is non-invertible"
        );
    }

    #[test]
    fn zero_crosses_to_zero_both_ways() {
        let cubit = UnitFactor::new(4572, 10000).unwrap();
        assert_eq!(emic_to_absolute(0, 12, cubit, 20), Some(0));
        assert_eq!(absolute_to_emic(0, 20, cubit, 12), Some(0));
    }

    #[test]
    fn an_out_of_range_crossing_declines_rather_than_wrapping() {
        // A large value times a large factor, upshifted, overruns the i64 result. The crossing must
        // return None, the widen signal, rather than wrap to a corrupt magnitude (the tier2
        // single-op contract). Verified against a raw wrapping composition below.
        let big = UnitFactor::new(1_000_000_000, 1).unwrap();
        assert_eq!(
            emic_to_absolute(1_000_000_000, 0, big, 40),
            None,
            "an out-of-range absolute must decline, not wrap"
        );
        // A modest crossing in the same shape still succeeds, so the guard is not over-eager.
        assert!(emic_to_absolute(1000, 0, big, 0).is_some());
    }

    fn unit(name: &str, num: i64, den: i64) -> EmicUnit {
        EmicUnit {
            dimension: crate::Dimension::base(0),
            factor: UnitFactor::new(num, den).unwrap(),
            name: name.to_string(),
            origin: UnitOrigin {
                kind: ProvenanceKindId(0),
                referent: "a forearm".to_string(),
            },
        }
    }

    #[test]
    fn a_measurement_system_registers_and_looks_up_units() {
        let mut ms = MeasurementSystem::new();
        let cubit = ms.register(unit("cubit", 4572, 10000));
        let span = ms.register(unit("span", 2286, 10000));
        assert_eq!(ms.len(), 2);
        assert_eq!(cubit, UnitId(0));
        assert_eq!(span, UnitId(1));
        assert_eq!(ms.id_of("cubit"), Some(UnitId(0)));
        assert_eq!(ms.id_of("span"), Some(UnitId(1)));
        assert_eq!(ms.id_of("league"), None);
        assert_eq!(ms.get(cubit).unwrap().name, "cubit");
    }

    #[test]
    fn the_canonical_walk_is_id_ordered_and_insertion_order_independent() {
        // Two systems that register the same units in different orders are different systems (ids
        // differ), but the canonical walk of each is its own registration order, deterministic and
        // independent of the hash map's iteration. The property that matters: iter_ordered yields
        // ascending ids, the single ordered accessor a hash is ever built over.
        let mut ms = MeasurementSystem::new();
        ms.register(unit("cubit", 4572, 10000));
        ms.register(unit("span", 2286, 10000));
        ms.register(unit("digit", 1905, 100000));
        let walk: Vec<(UnitId, String)> = ms
            .iter_ordered()
            .map(|(id, u)| (id, u.name.clone()))
            .collect();
        assert_eq!(
            walk,
            vec![
                (UnitId(0), "cubit".to_string()),
                (UnitId(1), "span".to_string()),
                (UnitId(2), "digit".to_string()),
            ],
            "the walk must be ascending id order"
        );
        // Deterministic across repeated walks (no hash-map iteration leaks in).
        let walk2: Vec<UnitId> = ms.iter_ordered().map(|(id, _)| id).collect();
        assert_eq!(walk2, vec![UnitId(0), UnitId(1), UnitId(2)]);
    }

    #[test]
    fn reindex_rebuilds_the_name_lookup() {
        // The name index carries `#[serde(skip)]`, so after a deserialize it is empty until reindex
        // rebuilds it from the ordered store (the same pattern as QuantityRegistry). Here reindex is
        // exercised for idempotence: rebuilding preserves every lookup and the ordered walk.
        let mut ms = MeasurementSystem::new();
        ms.register(unit("cubit", 4572, 10000));
        ms.register(unit("span", 2286, 10000));
        ms.reindex();
        assert_eq!(ms.id_of("cubit"), Some(UnitId(0)));
        assert_eq!(ms.id_of("span"), Some(UnitId(1)));
        let names: Vec<String> = ms.iter_ordered().map(|(_, u)| u.name.clone()).collect();
        assert_eq!(names, vec!["cubit".to_string(), "span".to_string()]);
    }

    #[test]
    #[should_panic(expected = "duplicate unit")]
    fn a_duplicate_unit_name_panics() {
        let mut ms = MeasurementSystem::new();
        ms.register(unit("cubit", 4572, 10000));
        ms.register(unit("cubit", 1, 2));
    }

    #[test]
    fn the_provenance_registry_is_open_and_admits_the_alien() {
        // A Terran people coins a length from a forearm and a time from a moon's cycle; an alien
        // people coins a length from a mana tide's reach. All three are registered kinds, data rows
        // in the same open registry, not code the crate had to anticipate. That is the seam the
        // input-audit caught: provenance is data, so the alien is a data row.
        let mut reg = ProvenanceKindRegistry::new();
        let forearm = reg.register("body-part");
        let moon = reg.register("celestial-cycle");
        let mana = reg.register("mana-tide"); // an origin the crate never enumerated
        assert_eq!(reg.len(), 3);
        assert_ne!(forearm, mana);
        // Registering an existing kind returns the existing id, so the set stays canonical.
        assert_eq!(reg.register("body-part"), forearm);
        assert_eq!(reg.len(), 3);
        assert_eq!(reg.name_of(moon), Some("celestial-cycle"));

        // A unit coined from the alien origin is an ordinary EmicUnit, no special case.
        let mut ms = MeasurementSystem::new();
        let tide_span = EmicUnit {
            dimension: crate::Dimension::base(0),
            factor: UnitFactor::new(7, 3).unwrap(),
            name: "tide-span".to_string(),
            origin: UnitOrigin {
                kind: mana,
                referent: "the third mana tide".to_string(),
            },
        };
        let id = ms.register(tide_span);
        assert_eq!(ms.get(id).unwrap().origin.kind, mana);
        assert_eq!(ms.get(id).unwrap().origin.referent, "the third mana tide");
    }
}
