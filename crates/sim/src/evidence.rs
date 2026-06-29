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

use crate::calibration::{CalibrationError, CalibrationManifest};
use civsim_core::{Fixed, StableId};

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
        let hyps: Vec<ValueId> = hyps.into_iter().collect();
        let totals = vec![0i128; hyps.len()];
        InferenceFrame {
            subject,
            attr,
            hyps,
            totals,
            support: Vec::new(),
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

    /// The provenance of every piece of evidence that fed this frame.
    pub fn support(&self) -> &[EvidenceRef] {
        &self.support
    }
}

#[inline]
fn clamp_i128(v: i128, bound: i128) -> i128 {
    v.clamp(-bound, bound)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
