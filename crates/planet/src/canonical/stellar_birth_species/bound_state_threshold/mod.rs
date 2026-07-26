//! Exact, non-authorizing disposition of solver-produced bound-state levels.
//!
//! A composite candidate is physically bound only when its complete level
//! band lies strictly below every covered separation threshold. Touching or
//! overlapping a threshold remains unresolved, and a level strictly above a
//! threshold exposes an energetically open channel. The arithmetic here is
//! exact and familiarity-independent.
//!
//! This module cannot prove that a threshold set is complete, construct its
//! coverage proof, admit a law premise, or mint a species. Production has no
//! constructor for [`ThresholdCoverageProof`]. The two algorithms therefore
//! emit diagnostic evidence with `authority_effect=none` until an upstream
//! claim-scoped authority supplies solver, coverage, validity, and ancestry
//! receipts and the pair is enrolled in the authority watchdog.

mod producer;
mod watchdog;

#[cfg(test)]
mod tests;

use civsim_units::bignum::BigRat;

const MAX_SEPARATION_THRESHOLDS: usize = 4_096;

#[derive(Debug, Clone)]
struct ExactClosedLevelBand {
    lower: BigRat,
    upper: BigRat,
}

#[derive(Debug, Clone)]
struct SeparationThreshold {
    channel_identity: [u8; 32],
    level: ExactClosedLevelBand,
}

#[derive(Debug, Clone)]
struct ThresholdCoverageProof {
    covered_channels: Vec<[u8; 32]>,
    _seal: ThresholdCoverageSeal,
}

#[derive(Debug, Clone, Copy)]
struct ThresholdCoverageSeal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundStateThresholdDisposition {
    StrictlyBelowAllThresholds { limiting_channel: [u8; 32] },
    TouchesOrOverlapsThreshold { channel: [u8; 32] },
    EnergeticallyOpenChannel { channel: [u8; 32] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundStateThresholdRefusal {
    InvalidCandidateBand,
    EmptyThresholdSet,
    ThresholdCapacityExceeded,
    InvalidThresholdBand([u8; 32]),
    DuplicateThresholdChannel([u8; 32]),
    ThresholdCoverageMismatch,
    CheckerDisagreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundStateThresholdReport {
    disposition: BoundStateThresholdDisposition,
}

impl BoundStateThresholdReport {
    const fn authority_effect(self) -> &'static str {
        "none"
    }
}

fn inspect_bound_state_thresholds(
    candidate: &ExactClosedLevelBand,
    thresholds: &[SeparationThreshold],
    coverage: &ThresholdCoverageProof,
) -> Result<BoundStateThresholdReport, BoundStateThresholdRefusal> {
    let produced = producer::disposition(candidate, thresholds, coverage);
    let watched = watchdog::disposition(candidate, thresholds, coverage);
    match (produced, watched) {
        (Ok(produced), Ok(watched)) if produced == watched => Ok(BoundStateThresholdReport {
            disposition: produced,
        }),
        (Err(produced), Err(watched)) if produced == watched => Err(produced),
        _ => Err(BoundStateThresholdRefusal::CheckerDisagreement),
    }
}
