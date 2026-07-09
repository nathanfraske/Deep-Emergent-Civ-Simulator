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

//! The knowledge-transmission substrate (design Parts 20, 23, 25, 41).
//!
//! A culture knows things (techniques, designs) and passes them on. This substrate models
//! that passing without an authored technique catalogue: the transmitted unit is an opaque
//! content-addressed [`DesignId`], the future `CompositionNode.id` the Part 41 evaluator
//! will mint, never a closed enum of technique kinds (Principle 4). The substrate exposes
//! exactly the drift and loss rates that `compose.transmission_stability` derives from
//! (Part 41): how far a copy drifts from its origin, and how fast an under-practised design
//! is lost.
//!
//! Three laws, all closed-form fixed-point, deterministic, and race-blind (they read
//! per-race data, they never branch on race identity):
//!
//! - [`transmit`]: a learner copies a design from a holder at a fidelity-scaled proficiency
//!   plus a bounded, mean-zero copy perturbation (drift). Both the fidelity and the effective
//!   drift are supplied by the caller as functions of the copier's own per-race perception and
//!   memory (Parts 20, 25; see [`copy_fidelity`] and [`copy_drift`]), so nothing about how
//!   faithfully a race copies is authored per race (Principle 9). The kernel itself is
//!   race-blind.
//! - [`erode_and_cull`]: each tick, a design held by fewer than the reserved minimum-viable
//!   practitioner count erodes, and when its proficiency crosses the structural floor it drops
//!   from the known set (the technique is lost, a dark age). A culture that still holds it can
//!   re-transmit it for free (rediscovery), because the content address is stable.
//! - [`is_stabilised`]: whether a design has outrun both loss and drift. This read is where
//!   `compose.transmission_stability` derives: the stability span is a function of the loss
//!   rate ([`stability_span`]) and the drift-similarity radius a function of the drift rate
//!   ([`drift_similarity_radius`]).
//!
//! The perturbation is symmetric about zero, so its expectation over a uniform draw is exactly
//! zero and no direction is authored (Principle 9). The cull floor is [`Fixed::ZERO`], a
//! structural boundary (zero proficiency is not-known), not a reserved tuneable, so it
//! fabricates no owner value. The three rates the substrate needs are reserved and fail loud
//! ([`TransmissionParams::from_manifest`]).
//!
//! Follow-on (deferred, not a small addition): registering the holder count of a design as a
//! conserved projection in the [`crate::conservation::ConservationRegistry`] (R-TIER-CONSIST).
//! That registry conserves a quantity across the promote and demote of
//! [`crate::lod::TwoTierWorld`], which today carries population and wealth across the tier
//! boundary but not knowledge. Holder count is not conserved under transmission or loss (those
//! laws are meant to add and remove holders: rediscovery and cull); it would be conserved only
//! when an individual holding a design is promoted to or demoted from a pool, which first
//! requires attaching [`Knowledge`] to individuals and pools and carrying it across the crossing.
//! That is a change to the two-tier world, out of scope here, so it is left as a follow-on rather
//! than over-scoped into this substrate.

use std::collections::{BTreeMap, BTreeSet};

use crate::calibration::{CalibrationError, CalibrationManifest};
use civsim_core::{DrawKey, Fixed, Phase, Rng, StableId};

/// An opaque content-addressed design identity: the address of a piece of transmissible
/// know-how in the composition morphospace (the future `CompositionNode.id` of Part 41). It is
/// a bare `u64`, never an index into an authored technique catalogue, so the world's know-how
/// grows without an enumerated list of kinds (Principle 4). Origination (which content gets
/// which address) is the Part 41 evaluator's job; this substrate only transmits, erodes, and
/// stabilises addresses that already exist.
pub type DesignId = u64;

/// The structural cull floor: a proficiency at or below zero means the design is no longer
/// practised, so it drops from the known set. Zero is a boundary of the representation
/// (not-known), not a reserved owner value, so nothing here is fabricated.
const CULL_THRESHOLD: Fixed = Fixed::ZERO;

/// What one holder knows: the set of designs it holds and its proficiency at each. Both are
/// ordered containers (R-CANON-WALK), so any walk over a holder's knowledge, and thus any state
/// hash or parallel fold over it, is deterministic. A design in `known` always has an entry in
/// `proficiency`; the two are kept in step by [`Knowledge::originate`] and the laws below.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Knowledge {
    /// The designs this holder can practise, by content address, in address order.
    pub known: BTreeSet<DesignId>,
    /// This holder's proficiency at each held design, in `[0, ONE]`. Address-keyed and ordered.
    pub proficiency: BTreeMap<DesignId, Fixed>,
}

impl Knowledge {
    /// Empty knowledge.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether this holder practises a design.
    pub fn holds(&self, design: DesignId) -> bool {
        self.known.contains(&design)
    }

    /// This holder's proficiency at a design, or [`Fixed::ZERO`] if it does not hold it.
    pub fn proficiency_of(&self, design: DesignId) -> Fixed {
        self.proficiency
            .get(&design)
            .copied()
            .unwrap_or(Fixed::ZERO)
    }

    /// Originate a design at a proficiency: the holder now practises it. Origination proper (the
    /// minting of a fresh content address and its viability score) is the Part 41 evaluator's;
    /// this is the caller entry point that seeds a known design so the transmission and loss laws
    /// have something to carry. The proficiency is clamped into `[0, ONE]`.
    pub fn originate(&mut self, design: DesignId, proficiency: Fixed) {
        self.known.insert(design);
        self.proficiency
            .insert(design, proficiency.clamp(Fixed::ZERO, Fixed::ONE));
    }
}

/// The reserved calibrations the transmission substrate needs. Read from the manifest; until
/// set, reading them fails loud rather than running on a fabricated default (mirrors
/// [`crate::world::GossipParams::from_manifest`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TransmissionParams {
    /// The base copy-drift magnitude: the half-width of the mean-zero proficiency perturbation a
    /// maximally unfaithful copier incurs. The effective drift of a real copier is this base
    /// scaled by the copier's own per-race fidelity (see [`copy_drift`]); this scalar is only the
    /// reserved base scale, not a per-race table.
    pub drift_rate: Fixed,
    /// The minimum-viable practitioner count: a design held by fewer than this many practitioners
    /// erodes each tick.
    pub loss_practitioner_floor: u32,
    /// The per-tick erosion rate a below-floor design loses proficiency at, in expectation.
    pub loss_rate: Fixed,
}

impl TransmissionParams {
    /// Read the transmission calibrations from the manifest, failing loud while reserved.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        let floor_raw = m.require_i64("transmission.loss_practitioner_floor")?;
        let loss_practitioner_floor =
            u32::try_from(floor_raw).map_err(|_| CalibrationError::BadValue {
                id: "transmission.loss_practitioner_floor".to_string(),
                detail: format!("must be a non-negative practitioner count, got {floor_raw}"),
            })?;
        Ok(TransmissionParams {
            drift_rate: m.require_fixed("transmission.drift_rate")?,
            loss_practitioner_floor,
            loss_rate: m.require_fixed("transmission.loss_rate")?,
        })
    }
}

/// The copy fidelity of a copier, a race-blind function of its own expressed memory and
/// perception (Parts 20, 25). Faithful copying needs both retention (memory) and resolution
/// (perception); the product is monotone in each and stays in `[0, ONE]` for inputs in
/// `[0, ONE]`. This reads per-entity data and never branches on race identity, so two races
/// differ here only through their memory and perception values: fidelity is a function of data,
/// not an authored per-race table (Principle 9). The exact functional form is where the Part 20
/// and Part 25 perception model plugs in; the product is the interim, and Weber's roughly 3%
/// just-noticeable difference is one calibration point for the drift scale, not the law.
pub fn copy_fidelity(memory: Fixed, perception: Fixed) -> Fixed {
    memory
        .clamp(Fixed::ZERO, Fixed::ONE)
        .mul(perception.clamp(Fixed::ZERO, Fixed::ONE))
}

/// The effective copy drift of a copier: the reserved base `drift_rate` scaled by the copier's
/// infidelity (`ONE - copy_fidelity`). A faithful copier drifts little; a poor one drifts up to
/// the base rate. This makes the drift a copier experiences a function of its own per-race
/// perception and memory (Principle 9), with `transmission.drift_rate` only the base scale, so
/// nothing about how far a race's copies drift is authored per race.
pub fn copy_drift(base_drift_rate: Fixed, memory: Fixed, perception: Fixed) -> Fixed {
    let infidelity = Fixed::ONE - copy_fidelity(memory, perception);
    base_drift_rate.mul(infidelity)
}

/// The canonical draw stream for one transmission copy: keyed on the LEARNER (the region
/// coordinate), the holder and the design's content address (the two loci), the tick, and
/// [`Phase::TRANSMIT`] (R-RNG-COORD). Folding the learner in is what makes N learners copying the
/// same design from the same holder on the same tick draw N DISTINCT perturbations; keying on
/// (holder, design, tick) alone gave every learner the identical stream. A copy-of-a-copy still
/// replays bit for bit, since the learner, holder, design, and tick are all a deterministic function
/// of canonical state.
pub fn transmit_draw(
    learner: StableId,
    holder: StableId,
    design: DesignId,
    tick: u64,
    seed: u64,
) -> Rng {
    DrawKey::pair(holder.0, design, tick, Phase::TRANSMIT)
        .in_region(learner.0)
        .rng(seed)
}

/// A learner copies a design from a holder (the transmission law). The learner takes the holder's
/// proficiency scaled by `copier_fidelity` (the fidelity-and-trust scaling, mirroring the gossip
/// step's `told_weight * trust`), plus a bounded, mean-zero copy perturbation of magnitude
/// `drift_rate`. Both `copier_fidelity` and `drift_rate` are supplied by the caller as functions
/// of the copier's own per-race perception and memory ([`copy_fidelity`], [`copy_drift`]), so the
/// kernel is race-blind and nothing about copying is authored per race (Principle 9).
///
/// The design's content address is not mutated here: origination (which content owns which
/// address) is the Part 41 evaluator's, so a copy of design `d` is still design `d`. What drifts
/// is the learner's proficiency at `d`. The perturbation is symmetric about zero (its expectation
/// over a uniform `draw` is `-2^-32`, one fixed-point ULP below zero, since the unit grid is the
/// half-open `[0, ONE)`; the residual bias is one ULP times `drift_rate`, negligible against the
/// drift scale), so it authors no direction.
///
/// The copy takes only if it lands above the structural cull floor: a high-fidelity copy lands at
/// high proficiency and the design ratchets in; a low-fidelity copy, whose scaled proficiency is
/// small and whose drift is large, often lands at or below zero and fails to implant, so a
/// low-fidelity culture stays shallow. Returns the resulting proficiency (whether or not it
/// implanted), for chaining a copy-of-a-copy. Deterministic: `draw` is the counter-keyed stream
/// from [`transmit_draw`].
pub fn transmit(
    learner: &mut Knowledge,
    design: DesignId,
    holder_proficiency: Fixed,
    copier_fidelity: Fixed,
    drift_rate: Fixed,
    draw: Rng,
) -> Fixed {
    let fidelity = copier_fidelity.clamp(Fixed::ZERO, Fixed::ONE);
    // The fidelity-scaled copy of the holder's proficiency (like told_weight * trust).
    let copied = holder_proficiency.mul(fidelity);
    // A bounded, near-mean-zero copy perturbation. `2 * unit - 1` lies in [-1, 1) with expectation
    // -2^-32 (one ULP below zero, since the unit grid is the half-open [0, ONE)); scaling by
    // drift_rate authors magnitude, and the one-ULP residual is negligible against that scale.
    let signed = Fixed::from_int(2).mul(draw.unit_fixed(0)) - Fixed::ONE;
    let perturb = signed.mul(drift_rate);
    let prof = (copied + perturb).clamp(Fixed::ZERO, Fixed::ONE);
    if prof > CULL_THRESHOLD {
        learner.known.insert(design);
        learner.proficiency.insert(design, prof);
    }
    prof
}

/// The erosion-and-loss pass over one population (a band, a culture). Each design held by fewer
/// than `loss_practitioner_floor` practitioners erodes this tick, and a design whose proficiency
/// crosses the structural floor drops from its holders' known sets (the technique is lost). A
/// design still held by at least the floor never erodes, so a culture that retains it can
/// re-transmit it for free later (rediscovery), because the content address is stable.
///
/// The pass reads the pre-tick holder counts (read-old) and then erodes (write-new), the
/// gather-then-apply shape the gossip step uses, so a cull this tick does not change another
/// design's fate this tick and the pass is order-independent given the frozen counts. Erosion is
/// one design-level, counter-keyed forgetting roll per design per tick ([`Phase::KNOW_LOSS`]),
/// whose expectation is `loss_rate` and which is always non-negative, so proficiency only erodes
/// and every below-floor holder of a design erodes in lockstep. Returns the culled
/// `(holder, design)` pairs, in canonical holder-then-design order, for inspection.
pub fn erode_and_cull(
    population: &mut BTreeMap<StableId, Knowledge>,
    params: &TransmissionParams,
    tick: u64,
    seed: u64,
) -> Vec<(StableId, DesignId)> {
    // Read-old: holder counts from the pre-tick snapshot.
    let mut holders: BTreeMap<DesignId, u32> = BTreeMap::new();
    for k in population.values() {
        for &d in &k.known {
            *holders.entry(d).or_insert(0) += 1;
        }
    }
    let mut culled = Vec::new();
    // Write-new: erode and cull, each holder reading only the frozen counts and its own state.
    for (&id, k) in population.iter_mut() {
        let below: Vec<DesignId> = k
            .known
            .iter()
            .copied()
            .filter(|d| holders.get(d).copied().unwrap_or(0) < params.loss_practitioner_floor)
            .collect();
        for d in below {
            // One forgetting draw per (design, tick): erosion = 2 * loss_rate * unit, with unit in
            // [0, ONE), so its expectation is loss_rate and it is never negative. Keyed on the
            // design and the tick, so it replays bit for bit and is race-blind.
            let unit = DrawKey::entity(d, tick, Phase::KNOW_LOSS)
                .rng(seed)
                .unit_fixed(0);
            let erosion = Fixed::from_int(2).mul(params.loss_rate).mul(unit);
            let prof = k.proficiency.get(&d).copied().unwrap_or(Fixed::ZERO);
            let new_prof = prof - erosion;
            if new_prof <= CULL_THRESHOLD {
                k.known.remove(&d);
                k.proficiency.remove(&d);
                culled.push((id, d));
            } else {
                k.proficiency.insert(d, new_prof);
            }
        }
    }
    culled
}

/// The stability span: the number of ticks a design must persist to have outrun loss, derived
/// from the loss rate. It is the natural loss timescale, the ticks over which erosion at
/// `loss_rate` would consume a full unit of proficiency (`ceil(ONE / loss_rate)`). A design that
/// persists at least this long has demonstrably beaten the loss process. This is the span half of
/// `compose.transmission_stability`, derived from `transmission.loss_rate` rather than fabricated
/// (the Part 41 coupling). A non-positive loss rate has no finite loss timescale and returns
/// [`u64::MAX`] as a fail-safe; the manifest `loss_rate` is a set, positive value.
pub fn stability_span(loss_rate: Fixed) -> u64 {
    if loss_rate <= Fixed::ZERO {
        return u64::MAX;
    }
    let q = Fixed::ONE.div(loss_rate);
    let bits = q.to_bits();
    let floor = bits >> Fixed::FRAC_BITS;
    let has_frac = (bits & ((1i64 << Fixed::FRAC_BITS) - 1)) != 0;
    let span = floor + if has_frac { 1 } else { 0 };
    (span.max(1)) as u64
}

/// The drift-similarity radius: the proficiency spread within which two re-converged copies count
/// as the same design, derived from the drift rate. Two surviving copies each within one drift
/// step of a shared origin differ by up to two drift steps, so the radius is `2 * drift_rate`.
/// This is the drift half of `compose.transmission_stability`, derived from
/// `transmission.drift_rate` rather than fabricated (the Part 41 coupling).
///
/// The convergence metric this radius bounds is a local proficiency-spread metric (see
/// [`is_stabilised`]). The compose morphospace metric (the distance between two designs in the
/// composition space) is owned by the unbuilt Part 41 evaluator and will replace the local
/// stand-in; the radius derivation carries over unchanged.
pub fn drift_similarity_radius(drift_rate: Fixed) -> Fixed {
    Fixed::from_int(2).mul(drift_rate)
}

/// The persistence-and-convergence history of one design, the input to [`is_stabilised`]. The
/// caller accumulates it: how many consecutive ticks the design has been held by at least the
/// practitioner floor (`persisted_ticks`), and a sample of its current copies' proficiencies
/// (`copies`), the local convergence metric.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DesignHistory {
    /// Consecutive ticks the design has persisted at or above the practitioner floor.
    pub persisted_ticks: u64,
    /// A sample of the design's current copies' proficiencies, for the convergence read.
    pub copies: Vec<Fixed>,
}

/// Whether a design has stabilised: it has persisted at least [`stability_span`] ticks (it has
/// outrun loss) and its current copies have re-converged within [`drift_similarity_radius`] (they
/// have outrun drift). This read is where `compose.transmission_stability` derives: both bounds
/// are computed from `params.loss_rate` and `params.drift_rate`, so the compose value is set
/// equal to what this substrate already uses rather than fabricated.
///
/// The convergence metric is the local proficiency spread (max minus min over the sample), a
/// stand-in for the Part 41 morphospace metric; that coupling is noted on
/// [`drift_similarity_radius`]. A sample of fewer than two copies cannot show divergence and is
/// trivially converged. `design` names which design the history belongs to (a caller-side key).
pub fn is_stabilised(
    design: DesignId,
    history: &DesignHistory,
    params: &TransmissionParams,
) -> bool {
    // `design` names which design the history belongs to (a caller-side key); the read itself is
    // over the accumulated history, so the id is not otherwise dereferenced here.
    let _ = design;
    if history.persisted_ticks < stability_span(params.loss_rate) {
        return false;
    }
    if history.copies.len() < 2 {
        return true;
    }
    let mut lo = Fixed::MAX;
    let mut hi = Fixed::MIN;
    for &c in &history.copies {
        if c < lo {
            lo = c;
        }
        if c > hi {
            hi = c;
        }
    }
    (hi - lo) <= drift_similarity_radius(params.drift_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(num: i64, den: i64) -> Fixed {
        Fixed::from_ratio(num, den)
    }

    // A high-fidelity and a low-fidelity race's expressed memory and perception (Parts 20, 25).
    // Only these per-race data values differ between the dual-test races; the kernels never see
    // race identity.
    fn hi_memory() -> Fixed {
        f(95, 100)
    }
    fn hi_perception() -> Fixed {
        f(98, 100)
    }
    fn lo_memory() -> Fixed {
        f(35, 100)
    }
    fn lo_perception() -> Fixed {
        f(50, 100)
    }

    fn params(drift: Fixed, floor: u32, loss: Fixed) -> TransmissionParams {
        TransmissionParams {
            drift_rate: drift,
            loss_practitioner_floor: floor,
            loss_rate: loss,
        }
    }

    // Build a culture as a chain population: entity 0 originates every design at full proficiency,
    // then each design is copied down a chain of `hops` entities under the given fidelity and
    // drift. Returns the population so its retained copies and depths can be measured. The entity
    // ids, design ids, tick, and seed are fixed, so two runs differ only in fidelity and drift.
    fn run_culture(
        n_designs: u64,
        hops: u64,
        fidelity: Fixed,
        drift: Fixed,
        seed: u64,
    ) -> BTreeMap<StableId, Knowledge> {
        let mut pop: BTreeMap<StableId, Knowledge> = BTreeMap::new();
        for h in 0..=hops {
            pop.insert(StableId(h), Knowledge::new());
        }
        for d in 0..n_designs {
            pop.get_mut(&StableId(0)).unwrap().originate(d, Fixed::ONE);
        }
        for h in 0..hops {
            let holder = StableId(h);
            let learner = StableId(h + 1);
            for d in 0..n_designs {
                let hp = pop.get(&holder).unwrap().proficiency_of(d);
                if !pop.get(&holder).unwrap().holds(d) {
                    continue;
                }
                let draw = transmit_draw(learner, holder, d, 0, seed);
                let mut k = pop.remove(&learner).unwrap();
                transmit(&mut k, d, hp, fidelity, drift, draw);
                pop.insert(learner, k);
            }
        }
        pop
    }

    fn total_copies(pop: &BTreeMap<StableId, Knowledge>) -> usize {
        pop.values().map(|k| k.known.len()).sum()
    }

    fn deep_at_frontier(pop: &BTreeMap<StableId, Knowledge>, frontier: u64, depth: Fixed) -> usize {
        pop.get(&StableId(frontier))
            .map(|k| {
                k.known
                    .iter()
                    .filter(|d| k.proficiency_of(**d) >= depth)
                    .count()
            })
            .unwrap_or(0)
    }

    // --- Test 1: replay determinism ---

    fn run_sim(seed: u64) -> BTreeMap<StableId, Knowledge> {
        let p = params(f(3, 100), 3, f(1, 4));
        let mut pop: BTreeMap<StableId, Knowledge> = BTreeMap::new();
        for e in 0..6u64 {
            pop.insert(StableId(e), Knowledge::new());
        }
        // Seed a few designs across a few holders.
        pop.get_mut(&StableId(0))
            .unwrap()
            .originate(100, Fixed::ONE);
        pop.get_mut(&StableId(0)).unwrap().originate(200, f(9, 10));
        pop.get_mut(&StableId(1)).unwrap().originate(100, f(8, 10));
        for tick in 0..8u64 {
            // A transmission sweep: each holder teaches its next neighbour every design it holds.
            let ids: Vec<StableId> = pop.keys().copied().collect();
            for i in 0..ids.len() {
                let holder = ids[i];
                let learner = ids[(i + 1) % ids.len()];
                let designs: Vec<DesignId> =
                    pop.get(&holder).unwrap().known.iter().copied().collect();
                for d in designs {
                    let hp = pop.get(&holder).unwrap().proficiency_of(d);
                    let draw = transmit_draw(learner, holder, d, tick, seed);
                    let mut k = pop.remove(&learner).unwrap();
                    transmit(&mut k, d, hp, f(85, 100), p.drift_rate, draw);
                    pop.insert(learner, k);
                }
            }
            erode_and_cull(&mut pop, &p, tick, seed);
        }
        pop
    }

    #[test]
    fn distinct_learners_from_one_holder_draw_distinct_perturbations() {
        // Regression (audit defect 12): two learners copying the SAME design from the SAME holder on
        // the SAME tick must draw distinct perturbation streams, now that the learner is folded into
        // the draw key. The same learner reproduces its own stream (determinism preserved).
        let holder = StableId(1);
        let design: DesignId = 42;
        let (l1, l2) = (StableId(10), StableId(11));
        let d1 = transmit_draw(l1, holder, design, 5, 0xC0FFEE).unit_fixed(0);
        let d2 = transmit_draw(l2, holder, design, 5, 0xC0FFEE).unit_fixed(0);
        assert_ne!(
            d1, d2,
            "two learners at one holder draw different perturbations"
        );
        let d1_again = transmit_draw(l1, holder, design, 5, 0xC0FFEE).unit_fixed(0);
        assert_eq!(
            d1, d1_again,
            "the same learner reproduces its own perturbation"
        );
    }

    #[test]
    fn replay_is_bit_identical() {
        let a = run_sim(0xABCDEF);
        let b = run_sim(0xABCDEF);
        assert_eq!(
            a, b,
            "same seed reproduces identical known-sets and proficiency"
        );
        let c = run_sim(0x123456);
        assert_ne!(
            a, c,
            "a different seed drives a different (still deterministic) run"
        );
    }

    // --- Test 2: drift accumulates along a copy-of-a-copy chain ---

    // Full fidelity isolates drift: the copied value carries perfectly, so the endpoint is the
    // origin plus the accumulated mean-zero perturbations. The ensemble spread across many designs
    // is due only to drift (the deterministic retention is identical for all), and grows with the
    // chain length H (~H drift steps) and with the drift rate.
    fn ensemble_spread(n_designs: u64, hops: u64, drift: Fixed, seed: u64) -> Fixed {
        // Start each design at 0.5 so the mean-zero walk does not clamp at the bounds.
        let mut finals: Vec<Fixed> = Vec::new();
        for d in 0..n_designs {
            let mut prof = f(1, 2);
            let mut k = Knowledge::new();
            for h in 0..hops {
                let draw = transmit_draw(StableId(h + 1), StableId(h), d, 0, seed);
                prof = transmit(&mut k, d, prof, Fixed::ONE, drift, draw);
            }
            finals.push(prof);
        }
        let mut lo = Fixed::MAX;
        let mut hi = Fixed::MIN;
        for &v in &finals {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
        hi - lo
    }

    #[test]
    fn drift_accumulates_monotone_in_chain_length() {
        let drift = f(1, 100); // 0.01, small enough that a 32-hop walk from 0.5 does not clamp
        let n = 1024;
        let seed = 0xD21F7;
        let s2 = ensemble_spread(n, 2, drift, seed);
        let s8 = ensemble_spread(n, 8, drift, seed);
        let s32 = ensemble_spread(n, 32, drift, seed);
        assert!(
            s2 < s8,
            "spread grows with chain length: s2={s2:?} s8={s8:?}"
        );
        assert!(
            s8 < s32,
            "spread grows with chain length: s8={s8:?} s32={s32:?}"
        );
        // Scales with the drift rate: doubling the base drift widens the spread.
        let s8_double = ensemble_spread(n, 8, f(2, 100), seed);
        assert!(
            s8_double > s8,
            "spread scales with drift rate: s8={s8:?} s8_double={s8_double:?}"
        );
        // Zero drift is a perfect copy: no divergence at all.
        assert_eq!(
            ensemble_spread(n, 32, Fixed::ZERO, seed),
            Fixed::ZERO,
            "with zero drift a copy-of-a-copy is exact"
        );
    }

    // --- Test 3: loss, cull (a dark age), then rediscovery ---

    #[test]
    fn loss_culls_then_a_holding_culture_rediscovers() {
        let p = params(f(3, 100), 5, f(1, 4));
        let design: DesignId = 42;
        // Culture A: two practitioners, below the floor of 5, so the design erodes.
        let mut culture_a: BTreeMap<StableId, Knowledge> = BTreeMap::new();
        for e in 0..2u64 {
            let mut k = Knowledge::new();
            k.originate(design, Fixed::ONE);
            culture_a.insert(StableId(e), k);
        }
        // Culture B: five practitioners, at the floor, so the design is retained.
        let mut culture_b: BTreeMap<StableId, Knowledge> = BTreeMap::new();
        for e in 10..15u64 {
            let mut k = Knowledge::new();
            k.originate(design, Fixed::ONE);
            culture_b.insert(StableId(e), k);
        }
        // Run the loss pass on both for enough ticks that A erodes past the floor and culls.
        let mut a_lost = false;
        for tick in 0..40u64 {
            erode_and_cull(&mut culture_a, &p, tick, 0x10ADED);
            erode_and_cull(&mut culture_b, &p, tick, 0x10ADED);
            if culture_a.values().all(|k| !k.holds(design)) {
                a_lost = true;
                break;
            }
        }
        assert!(
            a_lost,
            "the below-floor design erodes and culls in culture A (a dark age)"
        );
        assert!(
            culture_b.values().all(|k| k.holds(design)),
            "culture B holds the design at the floor, so it never erodes (it is retained)"
        );
        // Rediscovery: a holder in B re-transmits the design to a learner in A for free.
        let b_holder = StableId(10);
        let hp = culture_b.get(&b_holder).unwrap().proficiency_of(design);
        let learner = StableId(0);
        let draw = transmit_draw(learner, b_holder, design, 100, 0x10ADED);
        let mut k = culture_a.remove(&learner).unwrap();
        transmit(&mut k, design, hp, f(9, 10), p.drift_rate, draw);
        culture_a.insert(learner, k);
        assert!(
            culture_a.get(&learner).unwrap().holds(design),
            "the design re-diffuses into culture A from a culture that still holds it (rediscovery)"
        );
    }

    // --- Test 4: the non-steering fidelity dual, and the paired invariant ---

    #[test]
    fn fidelity_dual_diverges_and_identical_fidelity_is_invariant() {
        let base_drift = f(5, 100);
        let n = 12;
        let hops = 6;
        let seed = 0xFDE1;

        // Two races differ ONLY in their per-race memory and perception; fidelity and drift are
        // derived from that data, never authored per race.
        let hi_fid = copy_fidelity(hi_memory(), hi_perception());
        let hi_drift = copy_drift(base_drift, hi_memory(), hi_perception());
        let lo_fid = copy_fidelity(lo_memory(), lo_perception());
        let lo_drift = copy_drift(base_drift, lo_memory(), lo_perception());

        let hi = run_culture(n, hops, hi_fid, hi_drift, seed);
        let lo = run_culture(n, hops, lo_fid, lo_drift, seed);

        // The dual: high fidelity ratchets and deepens; low fidelity loses designs and stays
        // shallow. Nothing about this is authored: the two runs share every draw and differ only
        // through the memory-and-perception-derived fidelity and drift.
        assert!(
            total_copies(&hi) > total_copies(&lo),
            "high fidelity spreads more copies: hi={} lo={}",
            total_copies(&hi),
            total_copies(&lo)
        );
        let hi_deep = deep_at_frontier(&hi, hops, f(1, 2));
        let lo_deep = deep_at_frontier(&lo, hops, f(1, 2));
        assert!(
            hi_deep > lo_deep,
            "high fidelity reaches the frontier at depth; low fidelity does not: hi_deep={hi_deep} lo_deep={lo_deep}"
        );
        assert_eq!(
            lo_deep, 0,
            "the shallow culture holds nothing deep at the frontier"
        );

        // The paired invariant (a fidelity swap, the sibling of the modality-swap invariant): two
        // races whose per-race data DIFFERS (memory and perception swapped) but whose CITED
        // fidelity is identical produce bit-identical transmission outcomes. This proves the
        // outcome keys off the derived fidelity, not the raw per-race values and not race identity:
        // the kernel never sees a race, so equal cited fidelity gives equal results, which no
        // authored per-race table could guarantee.
        let fid_p = copy_fidelity(f(7, 10), f(8, 10));
        let dft_p = copy_drift(base_drift, f(7, 10), f(8, 10));
        let fid_q = copy_fidelity(f(8, 10), f(7, 10)); // swapped per-race data
        let dft_q = copy_drift(base_drift, f(8, 10), f(7, 10));
        assert_eq!(fid_p, fid_q, "the swap leaves the cited fidelity identical");
        assert_eq!(dft_p, dft_q, "the swap leaves the cited drift identical");
        let race_p = run_culture(n, hops, fid_p, dft_p, seed);
        let race_q = run_culture(n, hops, fid_q, dft_q, seed);
        assert_eq!(
            race_p, race_q,
            "identical cited fidelity gives an invariant transmission distribution (no authored table)"
        );

        // And erosion is invariant under the same swap: two identical-fidelity cultures erode
        // identically.
        let p = params(dft_p, 3, f(1, 4));
        let mut ep = race_p.clone();
        let mut eq = race_q.clone();
        for tick in 0..10u64 {
            erode_and_cull(&mut ep, &p, tick, seed);
            erode_and_cull(&mut eq, &p, tick, seed);
        }
        assert_eq!(
            ep, eq,
            "identical-fidelity cultures show invariant loss distributions"
        );
    }

    // --- Test 5: the stability gate derives from the reserved rates ---

    #[test]
    fn stability_gate_derives_from_the_reserved_rates() {
        let loss = f(1, 4); // 0.25 per tick
        let drift = f(3, 100); // 0.03
        let p = params(drift, 5, loss);

        // The span and radius are computed from the rates, not fabricated. These are exactly the
        // values compose.transmission_stability would be set to (it derives, set equal to the
        // transmission subsystem's own rates).
        assert_eq!(
            stability_span(loss),
            4,
            "stability span = ceil(ONE / loss_rate) = ceil(1 / 0.25) = 4"
        );
        // The radius is exactly two drift steps (the derivation compose.transmission_stability
        // would be set to), which is 0.06 within one fixed-point epsilon of rounding.
        assert_eq!(
            drift_similarity_radius(drift),
            drift + drift,
            "drift-similarity radius = 2 * drift_rate exactly"
        );
        assert!(
            (drift_similarity_radius(drift) - f(6, 100)).abs() <= Fixed::from_bits(2),
            "and that is 0.06 to fixed-point tolerance: {:?}",
            drift_similarity_radius(drift)
        );

        let design: DesignId = 7;
        // A design that persisted the full span with converged copies (spread within the radius)
        // reads stabilised.
        let converged = DesignHistory {
            persisted_ticks: 4,
            copies: vec![f(60, 100), f(63, 100), f(65, 100)], // spread 0.05 <= 0.06
        };
        assert!(
            is_stabilised(design, &converged, &p),
            "persisted >= span and copies within the radius reads stabilised"
        );
        // Not yet persisted long enough: not stabilised.
        let too_young = DesignHistory {
            persisted_ticks: 3,
            copies: converged.copies.clone(),
        };
        assert!(
            !is_stabilised(design, &too_young, &p),
            "a design that has not outrun loss is not stabilised"
        );
        // Persisted, but the copies are still too far apart: not stabilised.
        let diverged = DesignHistory {
            persisted_ticks: 10,
            copies: vec![f(20, 100), f(90, 100)], // spread 0.70 > 0.06
        };
        assert!(
            !is_stabilised(design, &diverged, &p),
            "a design whose copies have not re-converged is not stabilised"
        );
    }

    // --- from_manifest: fail-loud while reserved, parses when set ---

    #[test]
    fn params_fail_loud_while_reserved() {
        // drift_rate is now set (Arc 2 Mirror calibration, the base drift half-width); loss_rate and
        // loss_practitioner_floor remain reserved, so from_manifest still fails loud on the first
        // reserved read it makes (loss_practitioner_floor), never fabricating a value.
        let m = CalibrationManifest::load(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../calibration/reserved.toml"
        ))
        .unwrap();
        assert!(m.is_set("transmission.drift_rate"));
        assert!(m.is_reserved("transmission.loss_rate"));
        assert!(m.is_reserved("transmission.loss_practitioner_floor"));
        assert_eq!(
            TransmissionParams::from_manifest(&m).unwrap_err(),
            CalibrationError::Reserved("transmission.loss_practitioner_floor".to_string()),
            "reading a reserved transmission value fails loud (never fabricated)"
        );
    }

    #[test]
    fn params_parse_when_set() {
        let toml = r#"
[[reserved]]
id = "transmission.drift_rate"
basis = "test"
status = "set"
value = "0.03"
source = "test"

[[reserved]]
id = "transmission.loss_rate"
basis = "test"
status = "set"
value = "0.25"
source = "test"

[[reserved]]
id = "transmission.loss_practitioner_floor"
basis = "test"
status = "set"
value = "5"
source = "test"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let p = TransmissionParams::from_manifest(&m).unwrap();
        assert_eq!(p.drift_rate, f(3, 100));
        assert_eq!(p.loss_rate, f(1, 4));
        assert_eq!(p.loss_practitioner_floor, 5);
    }
}
