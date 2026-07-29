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

//! The belief inference engine (design Part 9.10, the resolved R-EVIDENCE work).
//!
//! An agent combines evidence into an inferred belief by a deterministic integer
//! rule. For a question (a subject and an attribute), the agent holds a small frame
//! of candidate hypotheses and an explicit unknown. Each piece of evidence adds a
//! signed integer weight, read from a data weight table, to one hypothesis's running
//! log-odds total. The belief commits to the leading hypothesis when its total
//! clears a reserved threshold and beats the runner-up by a reserved margin;
//! otherwise it stays unknown.
//!
//! Three properties matter and are enforced here. The combination is
//! order-independent: totals accumulate as plain integer addition in 128-bit space
//! and the certainty clamp is applied only when the belief is read, never per step,
//! because a per-step clamp would make the result depend on the order evidence
//! arrived in (the same trap the parallel-reduction audit caught). The engine
//! authors no outcome: it only sums weights that are data and reports the leader,
//! so it is steering-neutral. And it is per-mind: a sharper acuity scales the weight
//! extracted from each observation, and the epistemic stance sets the prior, the
//! threshold, and the margin, so two minds reach different beliefs from the same
//! evidence without either being more correct.
//!
//! The weights, the hypothesis frames, and the attribute kinds are data (Part 40);
//! the thresholds are the owner's reserved calibrations, read from the manifest and
//! failing loud until set. The engine is fixed Rust.

use civsim_core::{Fixed, StableId};
use civsim_foundation::calibration::{CalibrationError, CalibrationManifest};

/// A data-defined attribute kind (which question is being inferred). An identifier,
/// not a closed enum, so a world can ask questions the engine's authors never
/// enumerated (Principle 11).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct AttrKindId(pub u32);

/// A candidate value of an attribute, a data-defined identifier. The explicit
/// unknown outcome is the absence of a commit, not a value in this space.
pub type ValueId = u32;

/// A record of one piece of evidence that fed a total, kept for provenance so a
/// wrong inference is still fully traceable (design Part 9.10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct EvidenceRef {
    /// Who or what the evidence came from.
    pub from: StableId,
    /// The signed weight applied (after acuity scaling).
    pub weight: Fixed,
    /// The hypothesis value it was applied toward.
    pub toward: ValueId,
}

/// The reserved calibrations the commit test needs. Read from the calibration
/// manifest; until the owner sets them, reading them fails loud rather than running
/// on a fabricated default (runbook section 4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InferenceParams {
    /// Maximum admissible certainty: each total is clamped to plus or minus this.
    pub clamp: Fixed,
    /// The total a leading hypothesis must reach to commit.
    pub commit_threshold: Fixed,
    /// The lead over the runner-up a hypothesis must hold to commit.
    pub margin: Fixed,
}

impl InferenceParams {
    /// Read the parameters from the calibration manifest. Returns the fail-loud
    /// `Reserved` error while any of the three is still reserved, so a build cannot
    /// run tuned inference on unset numbers.
    pub fn from_manifest(m: &CalibrationManifest) -> Result<Self, CalibrationError> {
        Ok(InferenceParams {
            clamp: m.require_fixed("evidence.log_odds_clamp")?,
            commit_threshold: m.require_fixed("evidence.commit_threshold")?,
            margin: m.require_fixed("evidence.runner_up_margin")?,
        })
    }
}

/// One inference: a frame of candidate hypotheses over a question, with an additive,
/// order-independent log-odds total per hypothesis.
#[derive(Clone, Debug)]
pub struct InferenceFrame {
    /// The subject the question is about.
    pub subject: StableId,
    /// The attribute being inferred.
    pub attr: AttrKindId,
    hyps: Vec<ValueId>,
    totals: Vec<i128>, // raw log-odds bits, unclamped; the clamp is applied at read
    support: Vec<EvidenceRef>,
}

impl InferenceFrame {
    /// A fresh frame over a set of candidate values. Totals start at zero (a flat
    /// prior); a stance prior is applied through [`InferenceFrame::seed_prior`].
    pub fn new(
        subject: StableId,
        attr: AttrKindId,
        hyps: impl IntoIterator<Item = ValueId>,
    ) -> Self {
        let mut hyps: Vec<ValueId> = hyps.into_iter().collect();
        // Canonical candidate set: sorted and deduplicated, so the frame's hypothesis order is a
        // pure function of the value ids (R-CANON-WALK), the state hash walks a canonical order,
        // and merge_hyps can union new hypotheses in by binary search.
        hyps.sort_unstable();
        hyps.dedup();
        let totals = vec![0i128; hyps.len()];
        InferenceFrame {
            subject,
            attr,
            hyps,
            totals,
            support: Vec::new(),
        }
    }

    /// Union additional candidate hypotheses into the frame, keeping the candidate set sorted and
    /// its parallel `totals` aligned, each new hypothesis entering at a zero total. A belief's
    /// hypothesis space is the union of every candidate set asserted about the question, so the
    /// committed belief is a pure function of the evidence set rather than of which informant
    /// spoke first: the hypothesis space emerges from what the world asserts, and the gossip apply
    /// is order-independent (R-REDUCE-ORDER) instead of first-writer-wins.
    pub fn merge_hyps(&mut self, hyps: impl IntoIterator<Item = ValueId>) {
        for v in hyps {
            if let Err(idx) = self.hyps.binary_search(&v) {
                self.hyps.insert(idx, v);
                self.totals.insert(idx, 0i128);
            }
        }
    }

    /// Seed a per-hypothesis prior (the epistemic stance's starting position). Added,
    /// so it composes with evidence order-independently.
    pub fn seed_prior(&mut self, value: ValueId, prior: Fixed) {
        if let Some(idx) = self.hyps.iter().position(|h| *h == value) {
            self.totals[idx] += prior.to_bits() as i128;
        }
    }

    /// Add a piece of evidence toward a hypothesis. The weight is scaled by the
    /// mind's reasoning acuity (a sharper mind extracts more from the same
    /// observation). The total is a plain sum, so the result is independent of the
    /// order evidence is added. Evidence toward a value not in the frame is ignored.
    pub fn add_evidence(&mut self, toward: ValueId, weight: Fixed, acuity: Fixed, from: StableId) {
        let effective = weight.mul(acuity);
        if let Some(idx) = self.hyps.iter().position(|h| *h == toward) {
            self.totals[idx] += effective.to_bits() as i128;
            self.support.push(EvidenceRef {
                from,
                weight: effective,
                toward,
            });
        }
    }

    /// The clamped log-odds total for a hypothesis value, or `None` if it is not in
    /// the frame. The clamp is applied here, at read, so accumulation stays
    /// order-independent.
    pub fn clamped_total(&self, value: ValueId, params: &InferenceParams) -> Option<Fixed> {
        let idx = self.hyps.iter().position(|h| *h == value)?;
        Some(Fixed::from_bits(
            clamp_i128(self.totals[idx], params.clamp.to_bits() as i128) as i64,
        ))
    }

    /// Commit to the leading hypothesis if it clears the threshold and beats the
    /// runner-up by the margin; otherwise return `None`, the explicit unknown. Ties
    /// break by ascending hypothesis index, so the result is deterministic, and a
    /// genuine tie cannot commit because its margin over the runner-up is zero.
    pub fn commit(&self, params: &InferenceParams) -> Option<ValueId> {
        if self.hyps.is_empty() {
            return None;
        }
        let clamp = params.clamp.to_bits() as i128;
        let clamped: Vec<i128> = self.totals.iter().map(|t| clamp_i128(*t, clamp)).collect();

        // Leading and runner-up by clamped total, ties broken by lowest index.
        let mut lead = 0usize;
        for i in 1..clamped.len() {
            if clamped[i] > clamped[lead] {
                lead = i;
            }
        }
        let mut runner: Option<i128> = None;
        for (i, c) in clamped.iter().enumerate() {
            if i == lead {
                continue;
            }
            runner = Some(match runner {
                Some(r) => r.max(*c),
                None => *c,
            });
        }
        let runner = runner.unwrap_or(i128::MIN);

        let threshold = params.commit_threshold.to_bits() as i128;
        let margin = params.margin.to_bits() as i128;
        if clamped[lead] >= threshold && clamped[lead].saturating_sub(runner) >= margin {
            Some(self.hyps[lead])
        } else {
            None
        }
    }

    /// The candidate hypotheses of this frame, in their fixed order. Lets a holder walk
    /// the frame's values for a canonical hash without owning a second copy.
    pub fn hyps(&self) -> &[ValueId] {
        &self.hyps
    }

    /// The provenance of every piece of evidence that fed this frame.
    pub fn support(&self) -> &[EvidenceRef] {
        &self.support
    }
}

#[inline]
fn clamp_i128(v: i128, bound: i128) -> i128 {
    v.clamp(-bound, bound)
}

/// The logistic (sigmoid) transform `1 / (1 + e^-x)`, the log-odds-to-probability map the manifest's
/// unit convention already names (a total `L` maps to `p = 1 / (1 + e^-L)`; Elfes 1989; Thrun,
/// Burgard, Fox 2005). Deterministic and float-free: `Fixed::exp` is the pinned integer series. The
/// denominator `1 + e^-x` is at least one for any `x`, so the divide never hits zero; the result is
/// clamped into `[0, 1]` against fixed-point rounding at the tails.
fn logistic(x: Fixed) -> Fixed {
    let e = (Fixed::ZERO - x).exp();
    Fixed::ONE
        .checked_div(Fixed::ONE.saturating_add(e))
        .unwrap_or(Fixed::ZERO)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

/// The mean-field aggregate belief-diffusion rate: the per-step transmission probability a prevailing
/// belief gains, `sigmoid(told_weight * trust_baseline) * contact_density` (the SI-epidemic and
/// social-diffusion mean field). The individual tier's gossip loop adds `told_weight * trust` in
/// LOG-ODDS (nats) per contact (crates/sim/src/world.rs `apply_assertion`), so that quantity is not a
/// probability and cannot be used as a `[0, 1]` hazard directly; it is first mapped through the
/// logistic (the log-odds-to-probability transform the manifest's unit convention names) to the
/// per-contact transmission probability, then scaled by `contact_density`, the contacts per step. This
/// keeps the rate a true probability in `[0, 1]` that never pins the clamp: a per-contact log-odds of
/// two, which as a raw product would clamp the rate to one and saturate every belief in a single step,
/// maps to a per-contact probability of about 0.88 instead. Clamped to `[0, 1]` as a diffusion rate.
/// Nothing here reads a race id: two worlds diverge only through their gossip parameters and contact
/// density (Principle 9).
pub fn derive_aggregate_diffusion_rate(
    told_weight: Fixed,
    trust_baseline: Fixed,
    contact_density: Fixed,
) -> Fixed {
    let per_contact = logistic(told_weight.mul(trust_baseline));
    per_contact
        .mul(contact_density)
        .clamp(Fixed::ZERO, Fixed::ONE)
}

/// The aggregate-tier belief diffusion rate (`evidence.aggregate_diffusion_rate`), DERIVED from the
/// individual-tier gossip parameters rather than authored: the content-blind base rate is
/// [`derive_aggregate_diffusion_rate`] of `gossip.told_weight` and `gossip.trust_baseline` at unit
/// contact density, so it is the per-contact transmission PROBABILITY the logistic maps the per-contact
/// log-odds to. The per-belief spatial or social coupling enters downstream as the `distance` factor
/// [`crate::belief::PrevailingBelief::advance_diffusion`] applies, and the logistic `level * (1 - level)`
/// term throttles the step, so the aggregate is a proper SI mean field rather than a raw log-odds
/// product used as a hazard. The two gossip parameters are read fail-loud, so the rate cannot run on an
/// unset value (Principle 11), and no independent `evidence.aggregate_diffusion_rate` scalar is
/// authored: it is the same quantity the gossip loop already carries, kept to one source of truth. Read
/// here, in the `evidence`-namespaced module the value belongs to, and consumed by the belief substrate
/// ([`crate::belief::BeliefParams`]).
pub fn aggregate_diffusion_rate(m: &CalibrationManifest) -> Result<Fixed, CalibrationError> {
    let told_weight = m.require_fixed("gossip.told_weight")?;
    let trust_baseline = m.require_fixed("gossip.trust_baseline")?;
    Ok(derive_aggregate_diffusion_rate(
        told_weight,
        trust_baseline,
        Fixed::ONE,
    ))
}

/// I.J. Good's weight of evidence, `W = ln(P(E|H) / P(E|not H)) = ln P(E|H) - ln P(E|not H)`
/// (Good 1950; Jaynes 2003), the log-likelihood ratio a single observation contributes to the
/// additive log-odds total the inference engine sums. It is general over any two probabilities:
/// nothing here reads a trace kind, a race, or an attribute, so the same primitive serves the
/// mortality-implication weight, a corroboration weight, or any other likelihood contrast.
///
/// A zero probability has no finite log. Rather than introduce a fabricated floor-probability
/// value, the weight saturates to the certainty clamp the engine already reserves
/// (`evidence.log_odds_clamp`, passed in as `clamp`): a zero numerator (`P(E|H) = 0`, the
/// observation is impossible if the hypothesis holds) drives the weight to `-clamp`, and a zero
/// denominator (`P(E|not H) = 0`, impossible unless the hypothesis holds, so decisive for it)
/// drives it to `+clamp`. Both zero is no evidence either way, exactly zero. The finite result is
/// clamped into `[-clamp, +clamp]` too, so no likelihood contrast exceeds the maximum admissible
/// certainty. Deterministic: `Fixed::ln` is the pinned integer CORDIC log, no float.
pub fn good_weight(p_given_true: Fixed, p_given_false: Fixed, clamp: Fixed) -> Fixed {
    let neg_clamp = Fixed::ZERO - clamp;
    match (p_given_true <= Fixed::ZERO, p_given_false <= Fixed::ZERO) {
        (true, true) => Fixed::ZERO,
        (true, false) => neg_clamp,
        (false, true) => clamp,
        (false, false) => {
            // Both logs lie in the representable window (probabilities give a log in roughly
            // [-22.2, 21.5] nats), so the difference cannot overflow; clamp it to the certainty
            // bound to match the engine's log-odds ceiling.
            let w = p_given_true.ln() - p_given_false.ln();
            w.clamp(neg_clamp, clamp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_aggregate_rate_is_the_sigmoid_of_the_gossip_log_odds_and_never_pins_the_clamp() {
        // The mean field maps the per-contact log-odds through the sigmoid to a probability, then
        // scales by contact density: sigmoid(told_weight * trust) * contact_density.
        let told = Fixed::from_int(2);
        let a = derive_aggregate_diffusion_rate(told, Fixed::from_ratio(1, 2), Fixed::ONE);
        // told * trust = 1 nat; sigmoid(1) is about 0.731 at unit density.
        assert!(
            (a.to_bits() - Fixed::from_ratio(731, 1000).to_bits()).abs()
                < Fixed::ONE.to_bits() / 100,
            "sigmoid(1) is about 0.731 ({a:?})"
        );
        // A second world with lower trust has a smaller log-odds increment, so a lower rate (the
        // sigmoid is monotone), purely from its gossip parameter.
        let b = derive_aggregate_diffusion_rate(told, Fixed::from_ratio(1, 5), Fixed::ONE);
        assert!(
            b < a,
            "lower trust gives a lower per-contact probability ({b:?} < {a:?})"
        );
        // The units fix: a per-contact log-odds the OLD raw product would clamp to one (told 5 *
        // trust 1 = 5 nats) instead maps through the sigmoid to a probability STRICTLY below one, so
        // the rate never pins the clamp and a belief cannot saturate in a single step.
        let strong = derive_aggregate_diffusion_rate(Fixed::from_int(5), Fixed::ONE, Fixed::ONE);
        assert!(
            strong < Fixed::ONE,
            "a strong log-odds no longer pins the rate at one ({strong:?})"
        );
        assert!(
            strong > Fixed::from_ratio(1, 2),
            "but a strong positive log-odds is still a high probability ({strong:?})"
        );
        // Contact density scales the per-contact probability linearly: half the contacts, half the rate.
        let sparse =
            derive_aggregate_diffusion_rate(told, Fixed::from_ratio(1, 2), Fixed::from_ratio(1, 2));
        assert!(
            (sparse.to_bits() - a.to_bits() / 2).abs() < 4,
            "contact density scales the rate ({sparse:?})"
        );
    }

    #[test]
    fn the_aggregate_diffusion_tracks_the_real_threshold_gated_gossip_loop() {
        // Non-circular validation (the old test reimplemented advance_diffusion's own formula so it
        // passed by construction): the individual tier here is the REAL log-odds threshold-gated
        // mechanism world.rs's gossip loop runs, `Mind::consider` accumulating log-odds and
        // `Mind::belief` committing when they cross the reserved threshold, not a reimplementation of
        // the aggregate step. The aggregate `advance_diffusion` is validated as the SI logistic mean
        // field of that loop. Gossip fixtures (labelled, not owner values): one nat per convinced
        // contact against a commit threshold of three, so a listener needs several convinced contacts
        // to commit, the threshold gating the toy tier lacked.
        use crate::agent::Mind;
        use crate::belief::{BeliefKey, PrevailingBelief};
        use civsim_core::{DrawKey, Phase};

        let params = InferenceParams {
            clamp: Fixed::from_int(50),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(1),
        };
        let told_weight = Fixed::ONE;
        let trust = Fixed::ONE;
        let per_contact = told_weight.mul(trust); // one nat per convinced contact
        const SUBJECT: StableId = StableId(1);
        const ATTR: AttrKindId = AttrKindId(0);
        const VALUE: ValueId = 7;
        const OTHER: ValueId = 8;
        let hyps = [VALUE, OTHER];
        let convinced = |m: &Mind| m.belief(SUBJECT, ATTR, &params) == Some(VALUE);

        // Threshold gating (the property the toy test lacked): one told contact does NOT commit; the
        // reserved threshold takes three.
        let mut fresh = Mind::new(StableId(9_999), Fixed::ONE);
        fresh.consider(SUBJECT, ATTR, hyps, VALUE, per_contact, SUBJECT);
        assert!(
            !convinced(&fresh),
            "one told contact does not commit (log-odds gated)"
        );
        fresh.consider(SUBJECT, ATTR, hyps, VALUE, per_contact, SUBJECT);
        fresh.consider(SUBJECT, ATTR, hyps, VALUE, per_contact, SUBJECT);
        assert!(convinced(&fresh), "three told contacts clear the threshold");

        const N: usize = 1500;
        let seed = 0x0D1FF_u64;
        let mut minds: Vec<Mind> = (0..N as u64)
            .map(|i| Mind::new(StableId(i + 1000), Fixed::ONE))
            .collect();
        // Seed about eight percent as convinced with strong first-hand evidence.
        let seed_count = N / 12;
        for m in minds.iter_mut().take(seed_count) {
            m.consider(SUBJECT, ATTR, hyps, VALUE, Fixed::from_int(5), SUBJECT);
        }
        let start = minds.iter().filter(|m| convinced(m)).count();
        assert!(start > 0 && start < N, "a small seed is convinced");

        // The SI contagion over the REAL mechanism: each step every unconvinced mind contacts one
        // random other (counter-keyed draw), and hears the belief at one nat only if that partner was
        // convinced at the step-start snapshot (a synchronous update, so a mind convinced this step
        // does not also transmit this step).
        let mut individual = vec![start];
        for t in 0..40u64 {
            let snapshot: Vec<bool> = minds.iter().map(&convinced).collect();
            for i in 0..N {
                if snapshot[i] {
                    continue;
                }
                let partner = DrawKey::entity(i as u64, t, Phase::GOSSIP)
                    .slot(0)
                    .rng(seed)
                    .range_u32(0, N as u32) as usize;
                if partner != i && snapshot[partner] {
                    minds[i].consider(
                        SUBJECT,
                        ATTR,
                        hyps,
                        VALUE,
                        per_contact,
                        StableId(partner as u64 + 1000),
                    );
                }
            }
            individual.push(minds.iter().filter(|m| convinced(m)).count());
        }
        // The real loop spreads the belief monotonically to most of the population.
        for w in individual.windows(2) {
            assert!(w[1] >= w[0], "the convinced set only grows");
        }
        assert!(
            *individual.last().unwrap() > N * 4 / 5,
            "the belief saturates through the real threshold-gated gossip loop ({:?})",
            individual.last()
        );

        // The aggregate advance_diffusion, seeded at the same initial fraction and run at the DERIVED
        // rate, is the SI logistic mean field of that loop: it rises monotonically from the seed to
        // near saturation and does NOT jump to saturation in one step (the units bug the old
        // raw-product rate caused: a pinned rate of one drove the level to one in a single step).
        let rate = derive_aggregate_diffusion_rate(told_weight, trust, Fixed::ONE);
        let key = BeliefKey {
            subject: SUBJECT,
            attr: ATTR,
            value: VALUE,
        };
        let start_level = Fixed::from_ratio(start as i64, N as i64);
        let mut agg = PrevailingBelief::seeded(key, start_level, N as u32);
        let mut agg_curve = vec![agg.knowledge_level()];
        for _ in 0..40 {
            agg.advance_diffusion(rate, Fixed::ONE);
            agg_curve.push(agg.knowledge_level());
        }
        for w in agg_curve.windows(2) {
            assert!(w[1] >= w[0], "the aggregate level only grows");
        }
        assert!(
            agg_curve[1] < Fixed::from_ratio(1, 2),
            "the aggregate does not saturate in one step ({:?})",
            agg_curve[1]
        );
        assert!(
            *agg_curve.last().unwrap() > Fixed::from_ratio(9, 10),
            "the aggregate logistic reaches saturation ({:?})",
            agg_curve.last()
        );
    }

    fn params() -> InferenceParams {
        // Fixture calibration, not the owner's reserved values: this tests the
        // mechanism, while the manifest path stays fail-loud until the owner sets
        // the real numbers.
        InferenceParams {
            clamp: Fixed::from_int(10),
            commit_threshold: Fixed::from_int(3),
            margin: Fixed::from_int(2),
        }
    }

    const ONE_ACUITY: Fixed = Fixed::ONE;

    #[test]
    fn combination_is_order_independent() {
        // The crux property: the same evidence in any order yields the same belief.
        let subj = StableId(1);
        let attr = AttrKindId(0);
        let w = Fixed::from_int(2);

        let mut a = InferenceFrame::new(subj, attr, [10u32, 20, 30]);
        a.add_evidence(10, w, ONE_ACUITY, StableId(2));
        a.add_evidence(20, w, ONE_ACUITY, StableId(3));
        a.add_evidence(10, w, ONE_ACUITY, StableId(4));

        let mut b = InferenceFrame::new(subj, attr, [10u32, 20, 30]);
        b.add_evidence(10, w, ONE_ACUITY, StableId(4));
        b.add_evidence(20, w, ONE_ACUITY, StableId(3));
        b.add_evidence(10, w, ONE_ACUITY, StableId(2));

        assert_eq!(a.commit(&params()), b.commit(&params()));
        assert_eq!(a.commit(&params()), Some(10), "10 leads by 4 to 2");
    }

    #[test]
    fn clamp_at_read_keeps_order_independence_past_the_bound() {
        // Even when a prefix would exceed the certainty clamp, the clamp-at-read rule
        // makes the committed result independent of order (the C-05-style hazard).
        let subj = StableId(1);
        let attr = AttrKindId(0);
        let huge = Fixed::from_int(100); // far past the clamp of 10
        let neg = Fixed::from_int(-100);

        let mut a = InferenceFrame::new(subj, attr, [1u32, 2]);
        a.add_evidence(1, huge, ONE_ACUITY, StableId(9));
        a.add_evidence(1, neg, ONE_ACUITY, StableId(9));
        a.add_evidence(1, Fixed::from_int(5), ONE_ACUITY, StableId(9));

        let mut b = InferenceFrame::new(subj, attr, [1u32, 2]);
        b.add_evidence(1, Fixed::from_int(5), ONE_ACUITY, StableId(9));
        b.add_evidence(1, neg, ONE_ACUITY, StableId(9));
        b.add_evidence(1, huge, ONE_ACUITY, StableId(9));

        assert_eq!(a.clamped_total(1, &params()), b.clamped_total(1, &params()));
        assert_eq!(a.commit(&params()), b.commit(&params()));
    }

    #[test]
    fn stays_unknown_below_threshold_or_margin() {
        let subj = StableId(1);
        let attr = AttrKindId(0);
        // Below threshold: a single weight of 2 does not reach the threshold of 3.
        let mut weak = InferenceFrame::new(subj, attr, [1u32, 2]);
        weak.add_evidence(1, Fixed::from_int(2), ONE_ACUITY, StableId(2));
        assert_eq!(weak.commit(&params()), None);

        // Over threshold but the margin over the runner-up is too small.
        let mut close = InferenceFrame::new(subj, attr, [1u32, 2]);
        close.add_evidence(1, Fixed::from_int(4), ONE_ACUITY, StableId(2));
        close.add_evidence(2, Fixed::from_int(3), ONE_ACUITY, StableId(3));
        assert_eq!(
            close.commit(&params()),
            None,
            "lead of 1 is below the margin of 2"
        );
    }

    #[test]
    fn belief_is_defeasible() {
        let subj = StableId(1);
        let attr = AttrKindId(0);
        let mut f = InferenceFrame::new(subj, attr, [1u32, 2]);
        f.add_evidence(1, Fixed::from_int(5), ONE_ACUITY, StableId(2));
        assert_eq!(f.commit(&params()), Some(1));
        // Heavy opposite-sign evidence carries the total back and flips the belief.
        f.add_evidence(1, Fixed::from_int(-6), ONE_ACUITY, StableId(3));
        f.add_evidence(2, Fixed::from_int(5), ONE_ACUITY, StableId(4));
        assert_eq!(f.commit(&params()), Some(2));
    }

    #[test]
    fn acuity_scales_the_weight_extracted() {
        let subj = StableId(1);
        let attr = AttrKindId(0);
        let w = Fixed::from_int(2); // below the threshold of 3 at unit acuity

        let mut dull = InferenceFrame::new(subj, attr, [1u32, 2]);
        dull.add_evidence(1, w, ONE_ACUITY, StableId(2));
        assert_eq!(dull.commit(&params()), None);

        let mut sharp = InferenceFrame::new(subj, attr, [1u32, 2]);
        sharp.add_evidence(1, w, Fixed::from_int(2), StableId(2)); // acuity 2 doubles it
        assert_eq!(
            sharp.commit(&params()),
            Some(1),
            "a sharper mind commits sooner"
        );
    }

    #[test]
    fn params_from_manifest_fail_loud_while_reserved() {
        let toml = r#"
[[reserved]]
id = "evidence.log_odds_clamp"
basis = "max certainty"
status = "reserved"
source = "Part 9"
[[reserved]]
id = "evidence.commit_threshold"
basis = "balance of false conclusions against cold cases"
status = "reserved"
source = "Part 9"
[[reserved]]
id = "evidence.runner_up_margin"
basis = "lead over runner-up"
status = "reserved"
source = "Part 9"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let err = InferenceParams::from_manifest(&m).unwrap_err();
        assert_eq!(
            err,
            CalibrationError::Reserved("evidence.log_odds_clamp".to_string())
        );
    }

    #[test]
    fn params_from_manifest_read_once_set() {
        let toml = r#"
[[reserved]]
id = "evidence.log_odds_clamp"
basis = "max certainty"
status = "set"
value = "10"
source = "Part 9"
[[reserved]]
id = "evidence.commit_threshold"
basis = "balance"
status = "set"
value = "3"
source = "Part 9"
[[reserved]]
id = "evidence.runner_up_margin"
basis = "lead"
status = "set"
value = "2"
source = "Part 9"
"#;
        let m = CalibrationManifest::from_toml_str(toml).unwrap();
        let p = InferenceParams::from_manifest(&m).unwrap();
        assert_eq!(p.clamp, Fixed::from_int(10));
        assert_eq!(p.commit_threshold, Fixed::from_int(3));
        assert_eq!(p.margin, Fixed::from_int(2));
    }

    #[test]
    fn good_weight_is_symmetric_at_equal_probabilities() {
        // Equal likelihoods carry no evidence: ln(p/p) = 0, exactly, for any probability.
        let clamp = Fixed::from_int(20);
        for (n, d) in [(1, 10), (1, 2), (9, 10), (3, 4)] {
            let p = Fixed::from_ratio(n, d);
            assert_eq!(
                good_weight(p, p, clamp),
                Fixed::ZERO,
                "equal probabilities give zero weight"
            );
        }
    }

    #[test]
    fn good_weight_is_monotonic_in_p_true() {
        // Holding P(E|not H) fixed, a larger P(E|H) is stronger evidence for H.
        let clamp = Fixed::from_int(20);
        let p_false = Fixed::from_ratio(1, 2);
        let low = good_weight(Fixed::from_ratio(1, 5), p_false, clamp);
        let mid = good_weight(Fixed::from_ratio(1, 2), p_false, clamp);
        let high = good_weight(Fixed::from_ratio(4, 5), p_false, clamp);
        assert!(low < mid, "weight rises with P(E|H) ({low:?} < {mid:?})");
        assert!(mid < high, "weight rises with P(E|H) ({mid:?} < {high:?})");
        assert_eq!(mid, Fixed::ZERO, "the equal-probability case sits at zero");
    }

    #[test]
    fn good_weight_zero_probability_saturates_to_the_clamp_exactly() {
        // A zero has no finite log; the weight saturates to the certainty clamp, not a fabricated
        // floor. A zero numerator is decisive against H, a zero denominator decisive for it.
        let clamp = Fixed::from_int(7);
        let p = Fixed::from_ratio(3, 10);
        assert_eq!(
            good_weight(Fixed::ZERO, p, clamp),
            Fixed::ZERO - clamp,
            "an impossible observation under H hits -clamp exactly"
        );
        assert_eq!(
            good_weight(p, Fixed::ZERO, clamp),
            clamp,
            "an observation impossible except under H hits +clamp exactly"
        );
        assert_eq!(
            good_weight(Fixed::ZERO, Fixed::ZERO, clamp),
            Fixed::ZERO,
            "both impossible is no evidence either way"
        );
    }
}
