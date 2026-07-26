//! Identity-sorted, forward exact-threshold disposition.

use super::{
    BoundStateThresholdDisposition, BoundStateThresholdRefusal, ExactClosedLevelBand,
    SeparationThreshold, ThresholdCoverageProof, MAX_SEPARATION_THRESHOLDS,
};
use std::cmp::Ordering;

pub(super) fn disposition(
    candidate: &ExactClosedLevelBand,
    thresholds: &[SeparationThreshold],
    coverage: &ThresholdCoverageProof,
) -> Result<BoundStateThresholdDisposition, BoundStateThresholdRefusal> {
    if candidate.lower.cmp_rat(&candidate.upper) == Ordering::Greater {
        return Err(BoundStateThresholdRefusal::InvalidCandidateBand);
    }
    if thresholds.is_empty() {
        return Err(BoundStateThresholdRefusal::EmptyThresholdSet);
    }
    if thresholds.len() > MAX_SEPARATION_THRESHOLDS {
        return Err(BoundStateThresholdRefusal::ThresholdCapacityExceeded);
    }

    let mut ordered = thresholds.iter().collect::<Vec<_>>();
    ordered.sort_unstable_by_key(|threshold| threshold.channel_identity);
    for pair in ordered.windows(2) {
        if pair[0].channel_identity == pair[1].channel_identity {
            return Err(BoundStateThresholdRefusal::DuplicateThresholdChannel(
                pair[0].channel_identity,
            ));
        }
    }

    let mut covered = coverage.covered_channels.clone();
    covered.sort_unstable();
    if covered.windows(2).any(|pair| pair[0] == pair[1])
        || covered
            != ordered
                .iter()
                .map(|threshold| threshold.channel_identity)
                .collect::<Vec<_>>()
    {
        return Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch);
    }

    let mut first_overlap = None;
    let mut first_open = None;
    let mut limiting = None;
    for threshold in ordered {
        if threshold.level.lower.cmp_rat(&threshold.level.upper) == Ordering::Greater {
            return Err(BoundStateThresholdRefusal::InvalidThresholdBand(
                threshold.channel_identity,
            ));
        }
        if candidate.upper.cmp_rat(&threshold.level.lower) == Ordering::Less {
            match limiting {
                None => limiting = Some((&threshold.level.lower, threshold.channel_identity)),
                Some((current_lower, current_identity)) => {
                    let order = threshold.level.lower.cmp_rat(current_lower);
                    if order == Ordering::Less
                        || (order == Ordering::Equal
                            && threshold.channel_identity < current_identity)
                    {
                        limiting = Some((&threshold.level.lower, threshold.channel_identity));
                    }
                }
            }
        } else if candidate.lower.cmp_rat(&threshold.level.upper) == Ordering::Greater {
            first_open.get_or_insert(threshold.channel_identity);
        } else {
            first_overlap.get_or_insert(threshold.channel_identity);
        }
    }

    if let Some(channel) = first_overlap {
        Ok(BoundStateThresholdDisposition::TouchesOrOverlapsThreshold { channel })
    } else if let Some(channel) = first_open {
        Ok(BoundStateThresholdDisposition::EnergeticallyOpenChannel { channel })
    } else {
        Ok(BoundStateThresholdDisposition::StrictlyBelowAllThresholds {
            limiting_channel: limiting
                .ok_or(BoundStateThresholdRefusal::EmptyThresholdSet)?
                .1,
        })
    }
}
