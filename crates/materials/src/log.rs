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

//! The coverage-audit log: counts-are-queries over the verdicts a world produces.
//!
//! Every verdict is recorded, so the honesty accounting is a filter over the log rather than a hand-kept
//! tally: "how many authored draws did this world consume" is [`VerdictLog::authored_draw_count`], the Gap-Law
//! prospecting map (the near-degenerate verdicts) is [`VerdictLog::near_degenerate_count`], and the
//! ceremony-avoidance audit (a flood of trivials where a real selection was expected) is
//! [`VerdictLog::trivial_count`]. This is the coverage-law payoff the owner's contract names; the first REAL
//! verdicts arrive with the disposer instantiation, and the synthetic-verdict tests here prove the queries.

use crate::verdict::{ProvenanceKey, TieSlot, Verdict};

/// Which kind of verdict a log record captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictKind {
    /// A decided winner (`delta >= resolution_s`).
    Decided,
    /// A sub-resolution escalation (no winner).
    Escalate,
    /// A content-hash-keyed seeded draw (a counted authored draw).
    SeededDraw,
    /// A single-candidate trivial verdict.
    Trivial,
}

/// One logged verdict: its kind, its opaque provenance key, and the contingency slot it occupied (present for
/// the verdicts that carry one).
#[derive(Debug, Clone)]
pub struct VerdictRecord {
    /// The verdict kind.
    pub kind: VerdictKind,
    /// The opaque provenance key (`sim` resolves it against the seven-tag joined register).
    pub provenance_key: ProvenanceKey,
    /// The contingency slot, for the verdicts that carry one (a trivial verdict has none).
    pub tie_slot: Option<TieSlot>,
}

/// The coverage-audit log: every verdict a world produces, so the honesty accounting is a query.
#[derive(Debug, Clone, Default)]
pub struct VerdictLog {
    records: Vec<VerdictRecord>,
}

impl VerdictLog {
    /// An empty log.
    pub fn new() -> Self {
        VerdictLog {
            records: Vec::new(),
        }
    }

    /// Record a verdict.
    pub fn record<C>(&mut self, verdict: &Verdict<C>) {
        let record = match verdict {
            Verdict::Decided(d) => VerdictRecord {
                kind: VerdictKind::Decided,
                provenance_key: d.provenance_key(),
                tie_slot: Some(d.tie_slot()),
            },
            Verdict::Escalate(e) => VerdictRecord {
                kind: VerdictKind::Escalate,
                provenance_key: e.provenance_key(),
                tie_slot: Some(e.tie_slot()),
            },
            Verdict::SeededDraw(s) => VerdictRecord {
                kind: VerdictKind::SeededDraw,
                provenance_key: s.provenance_key(),
                tie_slot: Some(s.tie_slot()),
            },
            Verdict::Trivial(t) => VerdictRecord {
                kind: VerdictKind::Trivial,
                provenance_key: t.provenance_key(),
                tie_slot: None,
            },
        };
        self.records.push(record);
    }

    /// The logged records, in the order they were produced.
    pub fn records(&self) -> &[VerdictRecord] {
        &self.records
    }

    /// The number of logged verdicts.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// The counted AUTHORED DRAWS: the seeded draws, the layer-4 contingency consumption a world drew, each an
    /// authored pick nature did not derive. This is the counts-are-queries honesty tally.
    pub fn authored_draw_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.kind == VerdictKind::SeededDraw)
            .count()
    }

    /// The NEAR-DEGENERATE verdicts (escalations and seeded draws): the Gap-Law prospecting map, the sites
    /// where the deciding model could not cleanly separate the top candidates.
    pub fn near_degenerate_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| matches!(r.kind, VerdictKind::Escalate | VerdictKind::SeededDraw))
            .count()
    }

    /// The TRIVIAL verdicts: the ceremony-avoidance audit. A flood where a real selection was expected is a
    /// coverage smell (the proposer collapsed the morphospace to one candidate too eagerly).
    pub fn trivial_count(&self) -> usize {
        self.records
            .iter()
            .filter(|r| r.kind == VerdictKind::Trivial)
            .count()
    }

    /// The records for a given provenance key, so the honesty query can attribute draws to a register entry.
    pub fn records_for(&self, key: ProvenanceKey) -> impl Iterator<Item = &VerdictRecord> {
        self.records.iter().filter(move |r| r.provenance_key == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::{dispose, seeded_draw, trivial, Candidate, ProvenanceKey, TieSlot};
    use civsim_core::{Fixed, StateHasher};

    #[derive(Debug, Clone)]
    struct Cand(u64, Fixed);

    impl Candidate for Cand {
        fn feed_content(&self, hasher: &mut StateHasher) {
            hasher.write_u64(self.0);
        }
    }

    #[test]
    fn the_counts_are_queries_over_the_synthetic_verdict_log() {
        let mut log = VerdictLog::new();
        let pk = ProvenanceKey(3);
        let slot = TieSlot(1);

        // A decided verdict (clear winner).
        let decided = dispose(
            vec![Cand(1, Fixed::from_int(10)), Cand(2, Fixed::from_int(3))],
            |c: &Cand| c.1,
            Fixed::from_int(1),
            pk,
            slot,
        );
        log.record(&decided);

        // An escalation (sub-resolution gap).
        let escalated = dispose(
            vec![Cand(3, Fixed::from_int(5)), Cand(4, Fixed::from_int(5))],
            |c: &Cand| c.1,
            Fixed::from_int(2),
            pk,
            slot,
        );
        log.record(&escalated);

        // Two seeded draws (authored draws).
        let d1 = seeded_draw(
            vec![Cand(5, Fixed::ZERO), Cand(6, Fixed::ZERO)],
            slot,
            pk,
            1,
        );
        let d2 = seeded_draw(
            vec![Cand(7, Fixed::ZERO), Cand(8, Fixed::ZERO)],
            slot,
            pk,
            2,
        );
        log.record(&Verdict::SeededDraw(d1));
        log.record(&Verdict::SeededDraw(d2));

        // A trivial verdict.
        log.record(&trivial(Cand(9, Fixed::ZERO), pk));

        assert_eq!(log.len(), 5);
        assert_eq!(
            log.authored_draw_count(),
            2,
            "two seeded draws are the authored-draw tally"
        );
        assert_eq!(
            log.near_degenerate_count(),
            3,
            "one escalation plus two seeded draws are near-degenerate"
        );
        assert_eq!(
            log.trivial_count(),
            1,
            "one trivial verdict, the ceremony-avoidance count"
        );
        assert_eq!(
            log.records_for(pk).count(),
            5,
            "every record is attributed to the provenance key"
        );
    }
}
