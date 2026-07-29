//! Reverse map reconstruction and exact-threshold disposition.

use super::{
    BoundStateThresholdDisposition, BoundStateThresholdRefusal, ExactClosedLevelBand,
    SeparationThreshold, ThresholdCoverageProof, MAX_SEPARATION_THRESHOLDS,
};
use std::{cmp::Ordering, collections::BTreeMap};

pub(super) fn disposition(
    candidate: &ExactClosedLevelBand,
    thresholds: &[SeparationThreshold],
    coverage: &ThresholdCoverageProof,
) -> Result<BoundStateThresholdDisposition, BoundStateThresholdRefusal> {
    if candidate.upper.cmp_rat(&candidate.lower) == Ordering::Less {
        return Err(BoundStateThresholdRefusal::InvalidCandidateBand);
    }
    let threshold_count = thresholds.len();
    if threshold_count == 0 {
        return Err(BoundStateThresholdRefusal::EmptyThresholdSet);
    }
    if threshold_count > MAX_SEPARATION_THRESHOLDS {
        return Err(BoundStateThresholdRefusal::ThresholdCapacityExceeded);
    }

    let mut by_channel = BTreeMap::new();
    for threshold in thresholds.iter().rev() {
        if by_channel
            .insert(threshold.channel_identity, threshold)
            .is_some()
        {
            return Err(BoundStateThresholdRefusal::DuplicateThresholdChannel(
                threshold.channel_identity,
            ));
        }
    }
    let mut covered = BTreeMap::new();
    for identity in coverage.covered_channels.iter().rev() {
        if covered.insert(*identity, ()).is_some() {
            return Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch);
        }
    }
    if covered.len() != by_channel.len()
        || covered
            .keys()
            .zip(by_channel.keys())
            .any(|(left, right)| left != right)
    {
        return Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch);
    }

    let mut overlaps = Vec::new();
    let mut opens = Vec::new();
    let mut below = Vec::new();
    let mut invalid = Vec::new();
    for (identity, threshold) in by_channel.iter().rev() {
        if threshold.level.upper.cmp_rat(&threshold.level.lower) == Ordering::Less {
            invalid.push(*identity);
            continue;
        }
        let threshold_ends_before_candidate =
            threshold.level.upper.cmp_rat(&candidate.lower) == Ordering::Less;
        let threshold_starts_after_candidate =
            threshold.level.lower.cmp_rat(&candidate.upper) == Ordering::Greater;
        if threshold_starts_after_candidate {
            below.push((*identity, &threshold.level.lower));
        } else if threshold_ends_before_candidate {
            opens.push(*identity);
        } else {
            overlaps.push(*identity);
        }
    }

    if let Some(identity) = invalid.into_iter().min() {
        return Err(BoundStateThresholdRefusal::InvalidThresholdBand(identity));
    }
    if let Some(channel) = overlaps.into_iter().min() {
        return Ok(BoundStateThresholdDisposition::TouchesOrOverlapsThreshold { channel });
    }
    if let Some(channel) = opens.into_iter().min() {
        return Ok(BoundStateThresholdDisposition::EnergeticallyOpenChannel { channel });
    }
    let (limiting_channel, _) = below
        .into_iter()
        .min_by(
            |(left_identity, left_lower), (right_identity, right_lower)| {
                let order = left_lower.cmp_rat(right_lower);
                if order == Ordering::Equal {
                    left_identity.cmp(right_identity)
                } else {
                    order
                }
            },
        )
        .ok_or(BoundStateThresholdRefusal::EmptyThresholdSet)?;
    Ok(BoundStateThresholdDisposition::StrictlyBelowAllThresholds { limiting_channel })
}
