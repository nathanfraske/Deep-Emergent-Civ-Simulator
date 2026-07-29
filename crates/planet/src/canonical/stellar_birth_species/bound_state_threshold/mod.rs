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

use super::physical_registry::neutral_bound_profile::{
    threshold_binding_sha256, ExactInterval, ThresholdCoverageCapability,
};
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
pub(in crate::canonical::stellar_birth_species) enum BoundStateThresholdRefusal {
    InvalidCandidateBand,
    EmptyThresholdSet,
    ThresholdCapacityExceeded,
    InvalidThresholdBand([u8; 32]),
    DuplicateThresholdChannel([u8; 32]),
    ThresholdCoverageMismatch,
    CheckerDisagreement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::canonical::stellar_birth_species) struct BoundStateThresholdReport {
    disposition: BoundStateThresholdDisposition,
}

impl BoundStateThresholdReport {
    pub(in crate::canonical::stellar_birth_species) const fn authority_effect(
        &self,
    ) -> &'static str {
        "none"
    }

    pub(in crate::canonical::stellar_birth_species) const fn disposition_id(&self) -> &'static str {
        match self.disposition {
            BoundStateThresholdDisposition::StrictlyBelowAllThresholds { .. } => {
                "strictly_below_all_thresholds"
            }
            BoundStateThresholdDisposition::TouchesOrOverlapsThreshold { .. } => {
                "touches_or_overlaps_threshold"
            }
            BoundStateThresholdDisposition::EnergeticallyOpenChannel { .. } => {
                "energetically_open_channel"
            }
        }
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

/// Consume the claim-local coverage capability minted by the neutral-profile
/// pair, then run the pre-existing independent exact threshold algorithms.
///
/// The capability binds one candidate band and one covered separation channel.
/// It cannot admit a premise or member, and the resulting report keeps
/// `authority_effect=none`.
pub(in crate::canonical::stellar_birth_species) fn inspect_authorized_thresholds(
    candidate_lower: &BigRat,
    candidate_upper: &BigRat,
    thresholds: &[([u8; 32], BigRat, BigRat)],
    covered_channels: &[[u8; 32]],
    capability: &ThresholdCoverageCapability,
) -> Result<BoundStateThresholdReport, BoundStateThresholdRefusal> {
    if thresholds.len() != 1 || covered_channels.len() != 1 {
        return Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch);
    }
    let (channel_identity, threshold_lower, threshold_upper) = &thresholds[0];
    let expected_binding = threshold_binding_sha256(
        &ExactInterval {
            lower: candidate_lower.clone(),
            upper: candidate_upper.clone(),
        },
        *channel_identity,
        &ExactInterval {
            lower: threshold_lower.clone(),
            upper: threshold_upper.clone(),
        },
    )
    .map_err(|_| BoundStateThresholdRefusal::ThresholdCoverageMismatch)?;
    if capability.binding_sha256() != expected_binding
        || capability.producer_receipt_sha256() == [0; 32]
        || capability.watchdog_receipt_sha256() == [0; 32]
        || capability.producer_receipt_sha256() == capability.watchdog_receipt_sha256()
        || covered_channels[0] != *channel_identity
    {
        return Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch);
    }
    let candidate = ExactClosedLevelBand {
        lower: candidate_lower.clone(),
        upper: candidate_upper.clone(),
    };
    let separation_thresholds = thresholds
        .iter()
        .map(|(channel_identity, lower, upper)| SeparationThreshold {
            channel_identity: *channel_identity,
            level: ExactClosedLevelBand {
                lower: lower.clone(),
                upper: upper.clone(),
            },
        })
        .collect::<Vec<_>>();
    let coverage = ThresholdCoverageProof {
        covered_channels: covered_channels.to_vec(),
        _seal: ThresholdCoverageSeal,
    };
    inspect_bound_state_thresholds(&candidate, &separation_thresholds, &coverage)
}
