use super::*;

fn band(lower: i64, upper: i64) -> ExactClosedLevelBand {
    ExactClosedLevelBand {
        lower: BigRat::from_i64(lower),
        upper: BigRat::from_i64(upper),
    }
}

fn threshold(channel: u8, lower: i64, upper: i64) -> SeparationThreshold {
    SeparationThreshold {
        channel_identity: [channel; 32],
        level: band(lower, upper),
    }
}

fn coverage(channels: &[u8]) -> ThresholdCoverageProof {
    ThresholdCoverageProof {
        covered_channels: channels.iter().map(|channel| [*channel; 32]).collect(),
        _seal: ThresholdCoverageSeal,
    }
}

#[test]
fn exact_below_threshold_disposition_is_permutation_independent() {
    let candidate = band(2, 3);
    let ordered = [threshold(41, 8, 9), threshold(17, 5, 7)];
    let reversed = ordered.iter().cloned().rev().collect::<Vec<_>>();
    let proof = coverage(&[41, 17]);
    let expected = BoundStateThresholdDisposition::StrictlyBelowAllThresholds {
        limiting_channel: [17; 32],
    };

    for thresholds in [&ordered[..], &reversed[..]] {
        let report = inspect_bound_state_thresholds(&candidate, thresholds, &proof).unwrap();
        assert_eq!(report.disposition, expected);
        assert_eq!(report.authority_effect(), "none");
    }
}

#[test]
fn touching_or_overlapping_threshold_stays_unresolved() {
    for candidate in [band(2, 5), band(4, 6), band(5, 5)] {
        let report =
            inspect_bound_state_thresholds(&candidate, &[threshold(71, 5, 7)], &coverage(&[71]))
                .unwrap();
        assert_eq!(
            report.disposition,
            BoundStateThresholdDisposition::TouchesOrOverlapsThreshold { channel: [71; 32] }
        );
    }
}

#[test]
fn exact_level_above_a_covered_threshold_exposes_open_channel() {
    let report = inspect_bound_state_thresholds(
        &band(8, 9),
        &[threshold(93, 2, 7), threshold(24, 11, 12)],
        &coverage(&[24, 93]),
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        BoundStateThresholdDisposition::EnergeticallyOpenChannel { channel: [93; 32] }
    );
}

#[test]
fn overlap_takes_precedence_over_an_open_channel() {
    let report = inspect_bound_state_thresholds(
        &band(8, 10),
        &[threshold(93, 2, 7), threshold(24, 10, 12)],
        &coverage(&[24, 93]),
    )
    .unwrap();
    assert_eq!(
        report.disposition,
        BoundStateThresholdDisposition::TouchesOrOverlapsThreshold { channel: [24; 32] }
    );
}

#[test]
fn missing_or_malformed_coverage_refuses() {
    let candidate = band(2, 3);
    assert_eq!(
        inspect_bound_state_thresholds(&candidate, &[], &coverage(&[])),
        Err(BoundStateThresholdRefusal::EmptyThresholdSet)
    );
    assert_eq!(
        inspect_bound_state_thresholds(&candidate, &[threshold(4, 5, 6)], &coverage(&[])),
        Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch)
    );
    assert_eq!(
        inspect_bound_state_thresholds(&candidate, &[threshold(4, 5, 6)], &coverage(&[4, 4])),
        Err(BoundStateThresholdRefusal::ThresholdCoverageMismatch)
    );
}

#[test]
fn invalid_bands_duplicate_channels_and_capacity_refuse() {
    assert_eq!(
        inspect_bound_state_thresholds(&band(3, 2), &[threshold(4, 5, 6)], &coverage(&[4])),
        Err(BoundStateThresholdRefusal::InvalidCandidateBand)
    );
    assert_eq!(
        inspect_bound_state_thresholds(
            &band(2, 3),
            &[threshold(91, 9, 8), threshold(4, 6, 5)],
            &coverage(&[91, 4])
        ),
        Err(BoundStateThresholdRefusal::InvalidThresholdBand([4; 32]))
    );
    assert_eq!(
        inspect_bound_state_thresholds(
            &band(2, 3),
            &[threshold(4, 5, 6), threshold(4, 7, 8)],
            &coverage(&[4])
        ),
        Err(BoundStateThresholdRefusal::DuplicateThresholdChannel(
            [4; 32]
        ))
    );

    let oversized = (0..=MAX_SEPARATION_THRESHOLDS)
        .map(|index| {
            let mut identity = [0_u8; 32];
            identity[..8].copy_from_slice(&(index as u64).to_be_bytes());
            SeparationThreshold {
                channel_identity: identity,
                level: band(5, 6),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        inspect_bound_state_thresholds(&band(2, 3), &oversized, &coverage(&[])),
        Err(BoundStateThresholdRefusal::ThresholdCapacityExceeded)
    );
}

#[test]
fn independent_algorithms_agree_over_small_exact_band_grid() {
    for candidate_lower in -3..=3 {
        for candidate_upper in candidate_lower..=3 {
            let candidate = band(candidate_lower, candidate_upper);
            for first_lower in -3..=3 {
                for first_upper in first_lower..=3 {
                    for second_lower in -3..=3 {
                        for second_upper in second_lower..=3 {
                            let thresholds = [
                                threshold(11, first_lower, first_upper),
                                threshold(207, second_lower, second_upper),
                            ];
                            let proof = coverage(&[207, 11]);
                            assert_eq!(
                                producer::disposition(&candidate, &thresholds, &proof),
                                watchdog::disposition(&candidate, &thresholds, &proof)
                            );
                        }
                    }
                }
            }
        }
    }
}
