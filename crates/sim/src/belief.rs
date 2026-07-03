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

//! The belief facet-strength and prevailing-belief substrate (design Part 9.2 and Part 54).
//!
//! This is the belief half of the two-tier lifting and restriction operators the Part 54
//! keystone resolved. A prevailing belief at the aggregate tier carries two coupled
//! quantities on the owner's decision: an INTENSIVE `knowledge_level`, a `Fixed` in `[0, 1]`
//! that reads as the pool's derived mean conviction, and an EXTENSIVE belief `mass`, the
//! raw Q32.32 bit-sum of its members' facet strengths held as an exact-associative `i128`
//! (the same exact-addition contract `total_wealth` relies on in the two-tier world). The
//! intensive level is the mass divided by the member count; the extensive mass is what the
//! conserved-projection registry balances bit for bit across every tier crossing.
//!
//! Two operators cross the tier boundary and conserve belief mass by construction. Lifting
//! (promotion) mints a facet strength for each promoting member from the pool's level through
//! a reserved monotone curve plus a small counter-seeded per-mind dispersion, and subtracts
//! exactly the minted bits from the pool. Restriction (re-summarize) folds the id-ordered
//! facet strengths back by adding exactly their bits. Because both directions move the same
//! bits, total belief mass (pool plus promoted) is conserved in both directions with no
//! tolerance, which is what lets it register as a conserved projection alongside population
//! and wealth (design Part 58, R-PROJ-REGISTER).
//!
//! Everything here is content-blind (Principle 9). The diffusion and the lift key on no
//! belief's identity: two beliefs differing only in their [`BeliefKey`] diffuse and lift by
//! the identical mechanism, and two races diverge only through their per-race curve data,
//! never through a hardcoded belief-kind branch. The mechanism is fixed Rust; the curve, the
//! dispersion magnitude, and the diffusion rate are the owner's reserved calibrations, read
//! from the manifest and failing loud until set (Principle 11).
//!
//! The honest limit the owner signed off on: because the level-to-strength curve may be
//! nonlinear and the dispersion is a finite sample, the intensive level drifts slightly on a
//! lift while the extensive mass stays exact. That the population mean reconstructs the pool
//! level within a fixed-point tolerance is a calibration target on the curve and dispersion,
//! not an identity the mechanism enforces (the deliberately-lossy lift direction of Part 54,
//! corrected on the next re-summarize).

use crate::calibration::{CalibrationError, CalibrationManifest};
use crate::decision::Curve;
use crate::evidence::{self, AttrKindId, ValueId};
use civsim_core::{DrawKey, Fixed, Phase, StableId, StateHasher};
use std::collections::BTreeMap;

/// A `[0, 1]`-clamped conviction: the Part 9.2 belief strength as a standalone scalar,
/// decoupled from any belief store. A promoted mind carries one per prevailing belief it
/// holds; a pool carries their extensive bit-sum rather than the individual scalars.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub struct FacetStrength(pub Fixed);

impl FacetStrength {
    /// A facet strength, clamped into `[0, 1]`. Conviction is a probability-like quantity and
    /// cannot leave the unit interval, so a curve output or a dispersed value past either end
    /// saturates rather than wrapping.
    #[inline]
    pub fn new(value: Fixed) -> Self {
        FacetStrength(value.clamp(Fixed::ZERO, Fixed::ONE))
    }

    /// The underlying `Fixed` conviction.
    #[inline]
    pub fn get(self) -> Fixed {
        self.0
    }

    /// The raw Q32.32 bit pattern, the contribution this strength makes to a pool's extensive
    /// belief mass.
    #[inline]
    pub fn to_bits(self) -> i64 {
        self.0.to_bits()
    }
}

/// A belief's question and asserted value: what the belief is about (a subject and one of its
/// attributes) and which value it commits to. Reuses the evidence engine's identifiers
/// ([`AttrKindId`], [`ValueId`]) so a prevailing belief keys off the same data-defined
/// question space the individual-tier inference engine does, not a second enum (Principle 11).
/// `Ord`-derived so a [`BeliefPool`] walks in one canonical order (R-CANON-WALK).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BeliefKey {
    /// The subject the belief is about.
    pub subject: StableId,
    /// The attribute the belief is about.
    pub attr: AttrKindId,
    /// The value the belief commits to.
    pub value: ValueId,
}

impl BeliefKey {
    /// A stable content hash of the key, folded through the canonical [`StateHasher`], for use
    /// as a draw coordinate. Two beliefs with distinct keys draw distinct per-mind dispersions,
    /// so a mind promoted holding several beliefs perturbs each independently, exactly as the
    /// transmission substrate keys a copy on its content-addressed design id. The mechanism
    /// never branches on this value; it only decorrelates the dispersion draw.
    #[inline]
    pub fn key_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_stable(self.subject);
        h.write_u32(self.attr.0);
        h.write_u32(self.value);
        h.finish() as u64
    }

    /// Fold the key into a state hash in a fixed field order (canonical-walk contribution).
    #[inline]
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_stable(self.subject);
        h.write_u32(self.attr.0);
        h.write_u32(self.value);
    }
}

/// A prevailing belief at the aggregate tier: a [`BeliefKey`], its EXTENSIVE `mass` (the raw
/// Q32.32 bit-sum of its members' facet strengths, an exact-associative `i128`), and its
/// member `count`. The INTENSIVE knowledge level is the mass divided by the count. Mass, not
/// level, is the conserved quantity, so a lift and a restriction move bits and balance bit for
/// bit (design Part 54).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PrevailingBelief {
    /// What the belief is about and commits to.
    pub key: BeliefKey,
    /// The extensive belief mass: the raw Q32.32 bit-sum of the members' facet strengths, held
    /// in `i128` so addition is exact and associative and the conserved projection balances
    /// regardless of fold order or thread count (the `total_wealth` contract).
    pub mass: i128,
    /// How many members hold this belief.
    pub count: u32,
}

impl PrevailingBelief {
    /// An empty prevailing belief over a key (mass zero, count zero).
    pub fn empty(key: BeliefKey) -> Self {
        PrevailingBelief {
            key,
            mass: 0,
            count: 0,
        }
    }

    /// A prevailing belief whose members all sit exactly at `level`: mass is `level` scaled by
    /// the count, so [`Self::knowledge_level`] reads back `level` exactly. The setup and test
    /// entry point for a pool at a known conviction.
    pub fn seeded(key: BeliefKey, level: Fixed, count: u32) -> Self {
        PrevailingBelief {
            key,
            mass: level.to_bits() as i128 * count as i128,
            count,
        }
    }

    /// The INTENSIVE knowledge level: the extensive mass divided by the member count, a single
    /// integer divide of the raw bit-sum (the `confidence_weighted_mean` shape: accumulate in
    /// 128-bit space, divide once). An empty belief reads as zero rather than dividing by zero.
    #[inline]
    pub fn knowledge_level(&self) -> Fixed {
        if self.count == 0 {
            return Fixed::ZERO;
        }
        Fixed::from_bits((self.mass / self.count as i128) as i64)
    }

    /// Advance diffusion one step: raise the level toward saturation (a fully-held belief at
    /// one) and carry the extensive mass with it. The increment per member is
    /// `rate * distance * (1 - level)`, so a belief far from saturation gains fastest and one
    /// already held by everyone barely moves. Because every member gains the same increment,
    /// the mass gains `increment_bits * count`, which keeps `mass / count` equal to the raised
    /// level exactly.
    ///
    /// Content-blind (Principle 9): the `rate` is identical for every belief, read once from
    /// the reserved `evidence.aggregate_diffusion_rate`; the only per-belief difference is
    /// `distance`, the normalized diffusion coupling in `[0, 1]` the caller derives from the
    /// spatial or social distance the idea has to travel (a nearer, better-connected site
    /// passes a value closer to one). Nothing here reads the belief's key, so two beliefs
    /// differing only in their [`BeliefKey`] diffuse identically.
    pub fn advance_diffusion(&mut self, rate: Fixed, distance: Fixed) {
        if self.count == 0 {
            return;
        }
        let level = self.knowledge_level();
        let headroom = Fixed::ONE - level;
        let coupling = distance.clamp(Fixed::ZERO, Fixed::ONE);
        let increment = rate.mul(coupling).mul(headroom);
        let inc_bits = increment.to_bits() as i128;
        self.mass += inc_bits * self.count as i128;
    }

    /// Lift one promoting member out: mint a facet strength for it at the current level and
    /// subtract exactly its bits from the pool, dropping the count by one. Returns `None` for
    /// an empty belief (nothing to lift). The minted strength is the counterpart the individual
    /// tier carries, so the total belief mass is unchanged by the crossing.
    pub fn lift_one(
        &mut self,
        curve: &Curve,
        dispersion: Fixed,
        being: StableId,
        tick: u64,
        seed: u64,
    ) -> Option<FacetStrength> {
        if self.count == 0 {
            return None;
        }
        let level = self.knowledge_level();
        let s = instantiate_strength(level, curve, dispersion, being, self.key, tick, seed);
        self.mass -= s.to_bits() as i128;
        self.count -= 1;
        Some(s)
    }

    /// Lift a whole promoting cohort at once, in canonical id order, minting every member's
    /// facet strength from the SAME captured level (so the cohort all instantiate from the pool
    /// as it stood before any of them left), then subtracting exactly the summed minted bits and
    /// dropping the count by the cohort size. The sum is a 128-bit accumulate, so the mass moved
    /// is independent of the order the cohort is walked. Panics if the cohort is larger than the
    /// member count.
    pub fn lift_cohort(
        &mut self,
        beings_id_ordered: &[StableId],
        curve: &Curve,
        dispersion: Fixed,
        tick: u64,
        seed: u64,
    ) -> Vec<FacetStrength> {
        assert!(
            self.count as usize >= beings_id_ordered.len(),
            "lifting more members than the belief holds"
        );
        let level = self.knowledge_level();
        let mut minted = Vec::with_capacity(beings_id_ordered.len());
        let mut sum: i128 = 0;
        for &being in beings_id_ordered {
            let s = instantiate_strength(level, curve, dispersion, being, self.key, tick, seed);
            sum += s.to_bits() as i128;
            minted.push(s);
        }
        self.mass -= sum;
        self.count -= beings_id_ordered.len() as u32;
        minted
    }

    /// Fold one demoting member's facet strength back in: add exactly its bits and raise the
    /// count by one (restriction, the exact inverse move to [`Self::lift_one`]).
    pub fn fold_one(&mut self, strength: FacetStrength) {
        self.mass += strength.to_bits() as i128;
        self.count += 1;
    }

    /// Fold an id-ordered cohort of facet strengths back into the belief: add exactly the summed
    /// bits (a 128-bit accumulate, order-independent) and raise the count by the cohort size
    /// (design.md:3113, the id-ordered mean fold, in the accumulate-then-single-divide shape of
    /// `axiom::confidence_weighted_mean`; the divide is deferred to [`Self::knowledge_level`]).
    pub fn fold_back(&mut self, strengths_id_ordered: &[FacetStrength]) {
        let sum: i128 = strengths_id_ordered
            .iter()
            .map(|s| s.to_bits() as i128)
            .sum();
        self.mass += sum;
        self.count += strengths_id_ordered.len() as u32;
    }

    /// Fold the belief into a state hash: its key, then the extensive mass (both 64-bit limbs,
    /// so the full `i128` is unambiguous), then the count.
    pub fn hash_into(&self, h: &mut StateHasher) {
        self.key.hash_into(h);
        h.write_u64((self.mass as u128 >> 64) as u64);
        h.write_u64(self.mass as u128 as u64);
        h.write_u32(self.count);
    }
}

/// A pool's prevailing beliefs, keyed by [`BeliefKey`] in a `BTreeMap` so every walk (the state
/// hash, the lift order, the mass sum) is canonical and insertion-order independent
/// (R-CANON-WALK). Attached to an aggregate pool; the promoted tier carries the individual
/// facet strengths instead.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BeliefPool(BTreeMap<BeliefKey, PrevailingBelief>);

impl BeliefPool {
    /// An empty belief pool.
    pub fn new() -> Self {
        BeliefPool(BTreeMap::new())
    }

    /// Whether the pool holds no prevailing beliefs.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many distinct prevailing beliefs the pool holds.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// The prevailing belief for a key, if present.
    pub fn get(&self, key: &BeliefKey) -> Option<&PrevailingBelief> {
        self.0.get(key)
    }

    /// The prevailing belief for a key mutably, if present.
    pub fn get_mut(&mut self, key: &BeliefKey) -> Option<&mut PrevailingBelief> {
        self.0.get_mut(key)
    }

    /// Insert or replace a prevailing belief.
    pub fn insert(&mut self, belief: PrevailingBelief) {
        self.0.insert(belief.key, belief);
    }

    /// Seed a prevailing belief whose members all sit at `level` (setup and test helper).
    pub fn seed(&mut self, key: BeliefKey, level: Fixed, count: u32) {
        self.insert(PrevailingBelief::seeded(key, level, count));
    }

    /// The mutable prevailing belief for a key, creating an empty one if absent (the restriction
    /// target: a demoting member may fold into a belief the pool did not yet carry).
    pub fn entry_or_default(&mut self, key: BeliefKey) -> &mut PrevailingBelief {
        self.0
            .entry(key)
            .or_insert_with(|| PrevailingBelief::empty(key))
    }

    /// The keys in canonical order, materialized so a caller can lift while borrowing the pool
    /// mutably one belief at a time.
    pub fn keys_in_order(&self) -> Vec<BeliefKey> {
        self.0.keys().copied().collect()
    }

    /// Walk the prevailing beliefs in canonical key order.
    pub fn iter(&self) -> impl Iterator<Item = (&BeliefKey, &PrevailingBelief)> {
        self.0.iter()
    }

    /// The pool's total extensive belief mass: the sum of its prevailing beliefs' masses, in
    /// 128-bit space so it is exact and order-independent.
    pub fn total_mass(&self) -> i128 {
        self.0.values().map(|b| b.mass).sum()
    }

    /// Fold the whole belief pool into a state hash, length-prefixed and walked in canonical key
    /// order, so two pools that reached the same beliefs by different insertion orders hash the
    /// same and a belief boundary is unambiguous.
    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_u64(self.0.len() as u64);
        // `values()` walks the BTreeMap in key order, the canonical order (R-CANON-WALK).
        for belief in self.0.values() {
            belief.hash_into(h);
        }
    }
}

/// The reserved calibrations the belief substrate needs, read from the manifest and failing
/// loud while any is still reserved (Principle 11). The single source of truth the two retired
/// mappings (`tier.belief_level_to_strength` and `evidence.knowledge_to_strength`) now derive
/// through: one monotone level-to-strength curve, one per-mind dispersion magnitude, and one
/// aggregate diffusion rate.
#[derive(Clone, Debug)]
pub struct BeliefParams {
    /// The monotone level-to-strength curve (`tier.belief_level_to_strength`): a pool knowledge
    /// level maps through it to a promoted mind's base facet strength.
    pub level_to_strength: Curve,
    /// The per-mind dispersion magnitude (`tier.belief_dispersion`): the half-width of the
    /// symmetric, counter-seeded deviation added around the curve output so promoted minds vary.
    pub dispersion: Fixed,
    /// The aggregate diffusion rate (`evidence.aggregate_diffusion_rate`): how fast a prevailing
    /// belief's level climbs toward saturation per step, before the per-belief distance coupling.
    pub diffusion_rate: Fixed,
}

impl BeliefParams {
    /// Read the belief calibrations from the manifest. Returns the fail-loud `Reserved` error
    /// while any of the curve, the dispersion, or the diffusion rate is still reserved, so a
    /// build cannot run belief lifting or diffusion on unset numbers. The diffusion rate is read
    /// through [`evidence::aggregate_diffusion_rate`] so the `evidence`-namespaced value is
    /// obtained in one place.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(BeliefParams {
            level_to_strength: m.require_curve("tier.belief_level_to_strength")?,
            dispersion: m.require_fixed("tier.belief_dispersion")?,
            diffusion_rate: evidence::aggregate_diffusion_rate(m)?,
        })
    }
}

/// Instantiate a promoting mind's facet strength from a pool's knowledge level: the reserved
/// monotone curve read at the level, plus a symmetric mean-zero per-mind dispersion,
/// `clamp01(curve.eval(level) + (unit_draw * 2 - 1) * dispersion_mag)`. The unit draw is keyed
/// on the being, the belief's content hash, the tick, and [`Phase::BELIEF_LIFT`], so it replays
/// bit for bit and each mind and belief draws independently (the structure of
/// `axiom::inherit_seed`, whose mutation is a keyed deviate around a blended base).
///
/// Content-blind (Principle 9): the curve and dispersion are identical for every belief, and
/// the only per-belief input to the draw is the key's content hash, which decorrelates the
/// dispersion without steering it. Two races diverge only by supplying different `curve` data;
/// there is no belief-kind branch anywhere in this function.
pub fn instantiate_strength(
    level: Fixed,
    curve: &Curve,
    dispersion_mag: Fixed,
    being_id: StableId,
    key: BeliefKey,
    tick: u64,
    seed: u64,
) -> FacetStrength {
    let base = curve.eval(level);
    let unit = DrawKey::pair(being_id.0, key.key_hash(), tick, Phase::BELIEF_LIFT)
        .rng(seed)
        .unit_fixed(0);
    // Map the unit draw in [0, 1) to a symmetric deviate in [-1, 1) by exact addition, then
    // scale by the dispersion half-width. Addition is exact, so no rounding enters the deviate.
    let deviate = (unit + unit) - Fixed::ONE;
    let dispersion = deviate.mul(dispersion_mag);
    FacetStrength::new(base + dispersion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: ValueId) -> BeliefKey {
        BeliefKey {
            subject: StableId(7),
            attr: AttrKindId(3),
            value,
        }
    }

    /// A fixture identity curve (level maps to itself), never an owner calibration: it tests the
    /// mechanism while the manifest path stays fail-loud until the owner sets the real shape.
    fn identity_curve() -> Curve {
        Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)])
    }

    fn ids(n: u64) -> Vec<StableId> {
        (0..n).map(StableId).collect()
    }

    #[test]
    fn knowledge_level_is_the_intensive_mean_of_the_mass() {
        let b = PrevailingBelief::seeded(key(1), Fixed::from_ratio(3, 5), 40);
        assert_eq!(b.knowledge_level(), Fixed::from_ratio(3, 5));
        // The mass is extensive: level scaled by the count.
        assert_eq!(b.mass, Fixed::from_ratio(3, 5).to_bits() as i128 * 40);
        // An empty belief reads as zero rather than dividing by zero.
        assert_eq!(
            PrevailingBelief::empty(key(1)).knowledge_level(),
            Fixed::ZERO
        );
    }

    #[test]
    fn lift_then_fold_conserves_belief_mass_exactly() {
        // The conservation crux at the belief level: lifting a cohort out and folding it back in
        // returns the mass bit for bit, in both directions, whatever the curve or dispersion.
        let curve = identity_curve();
        let dispersion = Fixed::from_ratio(1, 10);
        let mut b = PrevailingBelief::seeded(key(1), Fixed::from_ratio(1, 2), 20);
        let mass0 = b.mass;
        let cohort = ids(8);

        let minted = b.lift_cohort(&cohort, &curve, dispersion, 5, 0xBEEF);
        assert_eq!(b.count, 12, "the cohort left the count");
        let carried: i128 = minted.iter().map(|s| s.to_bits() as i128).sum();
        // Total belief mass (pool remainder plus the minted counterparts) is unchanged.
        assert_eq!(b.mass + carried, mass0, "lift conserves total belief mass");

        b.fold_back(&minted);
        assert_eq!(b.count, 20, "the cohort returned to the count");
        assert_eq!(b.mass, mass0, "fold-back restores the mass bit for bit");
    }

    #[test]
    fn round_trip_level_fidelity_reconstructs_the_pool_level() {
        // Calibration target (not a mechanism-enforced identity): promoting N minds from a pool
        // at level L, the id-ordered mean of their facet strengths reconstructs L within a stated
        // tolerance. The mean drift is bounded by the finite-sample spread of a mean-zero draw.
        let curve = identity_curve();
        let dispersion = Fixed::from_ratio(1, 10);
        let level = Fixed::from_ratio(3, 5);
        let n = 256u64;
        let mut b = PrevailingBelief::seeded(key(1), level, n as u32);
        let minted = b.lift_cohort(&ids(n), &curve, dispersion, 0, 0x51E7);

        let sum: i128 = minted.iter().map(|s| s.to_bits() as i128).sum();
        let mean = Fixed::from_bits((sum / n as i128) as i64);
        let tolerance = Fixed::from_ratio(1, 100); // stated tolerance: 0.01
        assert!(
            (mean - level).abs() <= tolerance,
            "population mean {mean:?} reconstructs level {level:?} within {tolerance:?}"
        );
    }

    #[test]
    fn the_dispersion_draw_is_independent_of_cohort_iteration_order() {
        // Determinism: each member's minted strength is a pure function of its id, so lifting the
        // cohort in a scrambled order gives every member the identical strength, and the mass
        // moved (a 128-bit sum) is identical too.
        let curve = identity_curve();
        let dispersion = Fixed::from_ratio(1, 5);
        let level = Fixed::from_ratio(1, 2);

        let ordered = ids(16);
        let mut scrambled = ordered.clone();
        scrambled.reverse();

        let mut a = PrevailingBelief::seeded(key(2), level, 16);
        let mut c = PrevailingBelief::seeded(key(2), level, 16);
        let minted_a = a.lift_cohort(&ordered, &curve, dispersion, 9, 0xABCD);
        let minted_c = c.lift_cohort(&scrambled, &curve, dispersion, 9, 0xABCD);

        // Pair each id to its strength; the two maps must agree.
        let map_a: BTreeMap<StableId, i64> = ordered
            .iter()
            .zip(minted_a.iter())
            .map(|(id, s)| (*id, s.to_bits()))
            .collect();
        let map_c: BTreeMap<StableId, i64> = scrambled
            .iter()
            .zip(minted_c.iter())
            .map(|(id, s)| (*id, s.to_bits()))
            .collect();
        assert_eq!(
            map_a, map_c,
            "a member's strength is a pure function of its id"
        );
        assert_eq!(a.mass, c.mass, "the mass moved is order-independent");
    }

    #[test]
    fn fold_back_mean_is_order_independent() {
        // The id-ordered fold is a 128-bit sum, so folding the same strengths in any order gives
        // the identical mass and identical knowledge level.
        let strengths: Vec<FacetStrength> = [3i64, 9, 1, 7, 5]
            .iter()
            .map(|n| FacetStrength::new(Fixed::from_ratio(*n, 10)))
            .collect();
        let mut forward = PrevailingBelief::empty(key(1));
        forward.fold_back(&strengths);
        let mut reversed_vec = strengths.clone();
        reversed_vec.reverse();
        let mut reversed = PrevailingBelief::empty(key(1));
        reversed.fold_back(&reversed_vec);
        assert_eq!(forward.mass, reversed.mass);
        assert_eq!(forward.knowledge_level(), reversed.knowledge_level());
    }

    #[test]
    fn params_from_manifest_fail_loud_while_reserved() {
        // Reading the curve, the dispersion, or the diffusion rate from a still-reserved manifest
        // returns the fail-loud Reserved error, never a fabricated shape or number.
        let toml = r#"
[[reserved]]
id = "tier.belief_level_to_strength"
basis = "the monotone level-to-strength curve"
status = "reserved"
source = "Part 54"
[[reserved]]
id = "tier.belief_dispersion"
basis = "the per-mind dispersion half-width"
status = "reserved"
source = "Part 54"
[[reserved]]
id = "evidence.aggregate_diffusion_rate"
basis = "the aggregate diffusion rate"
status = "reserved"
source = "Part 9"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        assert_eq!(
            BeliefParams::from_manifest(&m).unwrap_err(),
            CalibrationError::Reserved("tier.belief_level_to_strength".to_string())
        );
    }

    #[test]
    fn params_from_manifest_read_once_set() {
        let toml = r#"
[[reserved]]
id = "tier.belief_level_to_strength"
basis = "curve"
status = "set"
value = "0=0,1=1"
source = "Part 54"
[[reserved]]
id = "tier.belief_dispersion"
basis = "dispersion"
status = "set"
value = "0.1"
source = "Part 54"
[[reserved]]
id = "evidence.aggregate_diffusion_rate"
basis = "rate"
status = "set"
value = "0.25"
source = "Part 9"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let p = BeliefParams::from_manifest(&m).unwrap();
        assert_eq!(p.dispersion, Fixed::from_ratio(1, 10));
        assert_eq!(p.diffusion_rate, Fixed::from_ratio(1, 4));
        assert_eq!(p.level_to_strength.eval(Fixed::ONE), Fixed::ONE);
    }

    #[test]
    fn diffusion_is_content_blind() {
        // Two beliefs differing ONLY in their BeliefKey, at the same level and count, diffuse to
        // the identical mass and level under the same rate and distance: the mechanism reads no
        // belief identity.
        let rate = Fixed::from_ratio(1, 4);
        let distance = Fixed::from_ratio(3, 4);
        let level = Fixed::from_ratio(2, 5);
        let mut a = PrevailingBelief::seeded(key(1), level, 30);
        let mut b = PrevailingBelief::seeded(key(2), level, 30);
        a.advance_diffusion(rate, distance);
        b.advance_diffusion(rate, distance);
        assert_eq!(a.mass, b.mass, "two keys diffuse to the identical mass");
        assert_eq!(a.knowledge_level(), b.knowledge_level());
        // Diffusion raises the level toward saturation, never past it.
        assert!(a.knowledge_level() > level);
        assert!(a.knowledge_level() < Fixed::ONE);
    }

    #[test]
    fn lift_is_content_blind_in_the_curve() {
        // With no dispersion, two beliefs differing only in BeliefKey lift the SAME being to the
        // identical strength (the curve output at the level), proving the deterministic lift keys
        // on the level and not on the belief's identity. With dispersion the two differ only by a
        // mean-zero keyed decorrelation, not by a steered mechanism.
        let curve = identity_curve();
        let level = Fixed::from_ratio(1, 2);
        let being = StableId(11);
        let s1 = instantiate_strength(level, &curve, Fixed::ZERO, being, key(1), 4, 0xF00D);
        let s2 = instantiate_strength(level, &curve, Fixed::ZERO, being, key(2), 4, 0xF00D);
        assert_eq!(s1, s2, "content-blind lift ignores the key");
        assert_eq!(s1.get(), curve.eval(level));
    }

    #[test]
    fn belief_id_permutation_leaves_the_hash_invariant() {
        // Two prevailing beliefs with identical content differing only in their key: inserting
        // them in either order gives the identical canonical hash (the BTreeMap walk is
        // key-ordered, not insertion-ordered), and the total mass matches. A permutation of which
        // key carries the (identical) content cannot be observed.
        let level = Fixed::from_ratio(3, 5);
        let mut forward = BeliefPool::new();
        forward.seed(key(1), level, 12);
        forward.seed(key(2), level, 12);
        let mut reverse = BeliefPool::new();
        reverse.seed(key(2), level, 12);
        reverse.seed(key(1), level, 12);

        let mut hf = StateHasher::new();
        forward.hash_into(&mut hf);
        let mut hr = StateHasher::new();
        reverse.hash_into(&mut hr);
        assert_eq!(
            hf.finish(),
            hr.finish(),
            "insertion order does not move the hash"
        );
        assert_eq!(forward.total_mass(), reverse.total_mass());
    }

    #[test]
    fn divergence_comes_only_from_per_race_curve_data() {
        // Two races supply divergent level-to-strength curves; the same being, key, and draw
        // coordinate yield divergent promoted strengths, a pure function of the curve data. There
        // is no belief-kind branch: swapping the two curves swaps the two outputs exactly.
        let level = Fixed::from_ratio(1, 2);
        let being = StableId(5);
        // Race A: steep (high conviction at mid level). Race B: shallow.
        let race_a = Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)]);
        let race_b = Curve::new([
            (Fixed::ZERO, Fixed::ZERO),
            (Fixed::ONE, Fixed::from_ratio(1, 4)),
        ]);
        let s_a = instantiate_strength(level, &race_a, Fixed::ZERO, being, key(1), 0, 1);
        let s_b = instantiate_strength(level, &race_b, Fixed::ZERO, being, key(1), 0, 1);
        assert_ne!(s_a, s_b, "divergent curves give divergent strengths");
        // The divergence is exactly the curve data: each strength is its own curve at the level.
        assert_eq!(s_a.get(), race_a.eval(level));
        assert_eq!(s_b.get(), race_b.eval(level));
        // Swap-test: relabelling the curves swaps the outputs identically, so nothing keys on a
        // hidden race or belief identity.
        let s_a_swapped = instantiate_strength(level, &race_b, Fixed::ZERO, being, key(1), 0, 1);
        let s_b_swapped = instantiate_strength(level, &race_a, Fixed::ZERO, being, key(1), 0, 1);
        assert_eq!(s_a_swapped, s_b);
        assert_eq!(s_b_swapped, s_a);
    }
}
