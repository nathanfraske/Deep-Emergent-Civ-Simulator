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

//! THE HINDCAST COMPARISON, in rigidity space, against the cited elastic-thickness rows.
//!
//! # THIS IS THE STEP `elastic_thickness_rows` DELIBERATELY STOPPED SHORT OF
//!
//! The rows module builds the observations and refuses to compare, because deciding what a miss means is
//! constitutional (RUNBOOK section 12). This module is that decision, made: a derived rigidity band is compared
//! against a cited row's own rigidity band, they AGREE when the bands overlap, and a disjoint pair is a MISS that
//! fires the residual law rather than any fit. It is still DORMANT: nothing on a run path calls it, no pin moves,
//! and the derived band is read exactly as the solve produced it and is never nudged toward a row.
//!
//! # THE COMPARISON IS IN RIGIDITY SPACE, AND ONLY THERE
//!
//! A published `T_e` is conditioned on an assumed `(E, nu)` the literature rarely states at the point of quotation,
//! and `T_e ~ (1/E)^(1/3)`, so comparing a derived `T_e` against a published one compares the world's plate against
//! a fictitious 80 GPa plate. Every conversion here goes through the ROW'S OWN modulus pair
//! ([`crate::moment_equivalence::RigidityBand::from_hindcast_thickness_interval`]), so both sides are rigidities and
//! neither imports the other's modulus. Where the row states no pair the comparison REFUSES with a named reason
//! rather than borrowing a modulus, which would manufacture agreement.
//!
//! # WHAT THE CURRENT ROWS CAN AND CANNOT DO, HONESTLY
//!
//! With the rows as cited, every real comparison refuses, for two reasons already in the rows module's blindness
//! set. Earth is the one Mirror-class row and is an age-rate relation with no fixed `T_e`: converting it needs a
//! loading age and a decision on the fitted-rate SD, which is the consumer's constitutional call, so it refuses
//! with [`ComparisonRefusal::RateRelationNeedsLoadingAge`]. Mars and Venus are out-of-sample with an absent
//! `(E, nu)`, so they refuse with [`ComparisonRefusal::ModuliAbsent`] until the pair is sourced. The mechanism is
//! built and general; the two blockages are named, not hidden, and the same mechanism compares the moment either is
//! lifted. The overlap, miss, and partition logic is proven on synthetic rows the way an unreachable arm always is.
//!
//! # THE OUTPUT IS FIT TO OBSERVATION NEVER (RUNBOOK section 12)
//!
//! No result carries a corrected or target `T_e`, no field a derivation should hit, and a miss is the residual law
//! firing, reported, never absorbed. The partition ([`crate::elastic_thickness_rows::SampleRole`]) rides every
//! result and is read from the row, never chosen here, because a row's calibration eligibility was declared in the
//! data before any fit. The overlap test carries no authored tolerance: both bands' widths are their own sources'.

use crate::elastic_thickness_rows::{ElasticThicknessRow, ObservedElasticThickness, SampleRole};
use crate::moment_equivalence::RigidityBand;

/// WHY A COMPARISON COULD NOT BE MADE IN RIGIDITY SPACE, named rather than papered over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonRefusal {
    /// The row states no `(E, nu)` ([`crate::elastic_thickness_rows::ModulusPair::Absent`]), so its `T_e` cannot be
    /// converted to a rigidity (Mars, Venus). The out-of-sample comparison is blocked until the pair is sourced from
    /// the underlying admittance primaries; borrowing a modulus would manufacture the agreement.
    ModuliAbsent,
    /// The row is an age-rate relation ([`ObservedElasticThickness::AgeRateRelation`]) with no fixed `T_e`.
    /// Converting it needs a loading age `dt`, and the row's own doc reserves to the consumer both the choice of
    /// `dt` and how to treat the fitted-rate SD (which is NOT a locality band). Held rather than invented.
    RateRelationNeedsLoadingAge,
    /// The flexural rigidity refused at an endpoint (a non-representable or non-positive rigidity from the row's own
    /// pair and thickness). A structural refusal, distinct from the two data-blockage refusals above.
    RigidityRefused,
}

/// WHICH RUNG OF THE RESIDUAL LAW a miss lands on. NEVER a verdict, NEVER a fit: a miss is the residual law firing,
/// reported. The rung names the next ACTION in order, and the action is analysis, never a code correction toward the
/// row. A comparison always reports the FIRST rung; walking the ladder is the analyst's step, not this function's.
///
/// THE LICENSE (ratified on review): encoding the FIRST rung is LAW, not judgment. Every miss begins as a defect
/// hunt by the Residual Law, so machine-encoding that entry point prevents the hurried-consumer failure where a
/// miss reads as an invitation to calibrate. The later rungs (band-narrowing, then the licensed channel) stay
/// correctly EXTERNAL, because they are the analyst's judgment rather than the law's fixed first move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualRung {
    /// FIRST: hunt a defect. A miss is a claim about the engine or the transcribed row before it is a claim about
    /// the world, so the first move is always to look for the mistake, not to accept the residual.
    DefectHunt,
    /// THEN: narrow the band. If no defect survives, either band may be wider than its own sources force; a tighter
    /// band can turn a miss into an overlap without any fit, purely by removing slack the sources did not license.
    BandNarrow,
    /// THEN: ledger it. A miss that survives the defect hunt and the narrowing is a standing residual, recorded as a
    /// fact about the world's data against the engine, never absorbed into a calibration.
    Ledger,
}

/// THE RESULT of comparing a derived rigidity band against one cited row.
///
/// It is a sum of exactly three outcomes: the bands overlap (consistent), the bands are disjoint (a miss carrying
/// the residual law's first rung), or the row cannot be brought into rigidity space (a named refusal). There is no
/// fourth outcome and, deliberately, no `Corrected` or `Target` anywhere in the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HindcastComparison {
    /// The derived band and the row's own band overlap: the world's plate is consistent with the observation, at no
    /// authored tolerance. The partition rides so a Mirror-class agreement is distinguishable from an out-of-sample.
    Consistent { sample_role: SampleRole },
    /// The bands are disjoint: a MISS. It carries the residual law's first rung and the partition, and it carries no
    /// corrected `T_e`, because absorbing the miss deletes the signal the residual law exists to keep.
    Miss {
        sample_role: SampleRole,
        rung: ResidualRung,
    },
    /// The comparison could not be made in rigidity space; the reason is named so a blockage is a finding rather than
    /// a silent pass.
    Refused { reason: ComparisonRefusal },
}

/// COMPARE a derived rigidity band against one cited row, in rigidity space, honoring RUNBOOK section 12.
///
/// The derived band is read as the solve produced it and is never nudged. The row is converted through its OWN
/// modulus pair, or the comparison refuses. A two-sided interval uses exact band overlap; a one-sided bound compares
/// against the bound directly and never invents the open side; an age-rate relation refuses for a loading age; an
/// absent pair refuses. A disjoint pair is a miss at the residual law's first rung, carrying the partition.
///
/// EDGE INCLUSIVITY: the intervals are CLOSED, so TOUCHING bands overlap (the operators are `<=` and `>=`
/// throughout: [`RigidityBand::overlaps`] and the bound arms below). This is the honest reading because a rigidity
/// band is an inclusive interval, and a ulp-marginal miss costs only an investigation by design (the residual
/// law's first rung), never a fit, so erring toward overlap at the boundary spends nothing.
pub fn compare_to_row(derived: RigidityBand, row: &ElasticThicknessRow) -> HindcastComparison {
    // The row's own moduli, or refuse. An absent pair cannot be brought into rigidity space (Mars, Venus), and
    // borrowing one would manufacture agreement, so the blockage is reported rather than crossed.
    let (e, nu) = match row.modulus_pair.operative() {
        Some(pair) => pair,
        None => {
            return HindcastComparison::Refused {
                reason: ComparisonRefusal::ModuliAbsent,
            }
        }
    };
    // THE ONE-SIDED-BOUND LICENSE (ratified on review): every arm below is the SAME rule, band intersection over
    // the row's STATED SUPPORT, never a variant. A two-sided interval claims both edges; an upper-bound row claims
    // exactly one edge and the comparison honors exactly that claim, inconsistent only when the derived band lies
    // entirely beyond it, with the open side never invented. That is band intersection applied to the row's actual
    // epistemic shape, not a different test for bounds.
    let consistent = match row.observed {
        // A TWO-SIDED INTERVAL: exact band overlap through the row's own pair.
        ObservedElasticThickness::Interval { low_km, high_km } => {
            match RigidityBand::from_hindcast_thickness_interval(low_km, high_km, e, nu) {
                Some(observed) => derived.overlaps(observed),
                None => {
                    return HindcastComparison::Refused {
                        reason: ComparisonRefusal::RigidityRefused,
                    }
                }
            }
        }
        // AN UPPER BOUND `T_e <= max`. Rigidity is monotone in thickness, so this is `D <= D(max)`: the observed
        // constraint is the half-line at or below `D(max)`, open below. The derived band is consistent unless it
        // lies ENTIRELY above `D(max)` (its low edge exceeds the ceiling). The open lower side is never invented.
        ObservedElasticThickness::UpperBound { max_km } => {
            match crate::flexure::flexural_rigidity(e, nu, max_km) {
                Some(d_max) => derived.low() <= d_max,
                None => {
                    return HindcastComparison::Refused {
                        reason: ComparisonRefusal::RigidityRefused,
                    }
                }
            }
        }
        // A LOWER BOUND `T_e >= min`, i.e. `D >= D(min)`: the half-line at or above `D(min)`, open above. Consistent
        // unless the derived band lies ENTIRELY below `D(min)` (its high edge falls under the floor).
        ObservedElasticThickness::LowerBound { min_km } => {
            match crate::flexure::flexural_rigidity(e, nu, min_km) {
                Some(d_min) => derived.high() >= d_min,
                None => {
                    return HindcastComparison::Refused {
                        reason: ComparisonRefusal::RigidityRefused,
                    }
                }
            }
        }
        // AN AGE-RATE RELATION has no fixed `T_e`; it refuses for a loading age rather than inventing one.
        ObservedElasticThickness::AgeRateRelation { .. } => {
            return HindcastComparison::Refused {
                reason: ComparisonRefusal::RateRelationNeedsLoadingAge,
            }
        }
    };
    if consistent {
        HindcastComparison::Consistent {
            sample_role: row.sample_role,
        }
    } else {
        HindcastComparison::Miss {
            sample_role: row.sample_role,
            rung: ResidualRung::DefectHunt,
        }
    }
}

/// COMPARE a derived band against every cited row, pairing each row's `(body, region)` with its result. Dormant: it
/// reads no run path. With the rows as cited, every result is a refusal, and the reasons partition as one
/// [`ComparisonRefusal::RateRelationNeedsLoadingAge`] (Earth) and sixteen [`ComparisonRefusal::ModuliAbsent`] (Mars
/// and Venus), which is the honest state of the out-of-sample comparison until those two blockages are lifted.
pub fn compare_to_all_cited_rows(
    derived: RigidityBand,
) -> Vec<(&'static str, &'static str, HindcastComparison)> {
    crate::elastic_thickness_rows::all_hindcast_rows()
        .iter()
        .map(|row| (row.body, row.region, compare_to_row(derived, row)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elastic_thickness_rows::{
        all_hindcast_rows, calmant_oceanic_seamounts, ElasticThicknessKind, HindcastMethod,
        ModulusPair,
    };
    use civsim_core::Fixed;

    // A SYNTHETIC row with a stated pair, so the overlap and miss arms (unreachable through the cited rows, which all
    // refuse) are exercised directly. The moduli and observation are chosen by the test, never cited.
    fn synthetic_row(
        observed: ObservedElasticThickness,
        sample_role: SampleRole,
        youngs_modulus_gpa: Fixed,
        poisson_ratio: Fixed,
    ) -> ElasticThicknessRow {
        ElasticThicknessRow {
            body: "Synthetic",
            region: "test",
            observed,
            kind: ElasticThicknessKind::TeElastic,
            method: HindcastMethod::ThreeDNumericalPlateFit,
            modulus_pair: ModulusPair::Stated {
                youngs_modulus_gpa,
                poisson_ratio,
            },
            sample_role,
            load_class: "synthetic",
            epoch: "synthetic",
            citation: "synthetic test row, not cited",
            conditioning: &[],
        }
    }

    fn e_nu() -> (Fixed, Fixed) {
        (Fixed::from_int(80), Fixed::from_ratio(1, 4))
    }

    // The rigidity of a thickness through the test pair, for placing a derived band relative to an observed one.
    fn d_at(te_km: i32) -> Fixed {
        let (e, nu) = e_nu();
        crate::flexure::flexural_rigidity(e, nu, Fixed::from_int(te_km))
            .expect("a positive rigidity")
    }

    #[test]
    fn an_overlapping_interval_row_is_consistent_and_carries_its_partition() {
        let (e, nu) = e_nu();
        let row = synthetic_row(
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(20),
                high_km: Fixed::from_int(40),
            },
            SampleRole::OutOfSample,
            e,
            nu,
        );
        // A derived band straddling the observed interval overlaps it.
        let derived = RigidityBand::new(d_at(30), d_at(50)).expect("a band");
        assert_eq!(
            compare_to_row(derived, &row),
            HindcastComparison::Consistent {
                sample_role: SampleRole::OutOfSample
            },
            "bands that overlap are consistent, and the partition rides the result"
        );
    }

    #[test]
    fn a_disjoint_interval_row_is_a_miss_at_the_defect_hunt_rung() {
        let (e, nu) = e_nu();
        let row = synthetic_row(
            ObservedElasticThickness::Interval {
                low_km: Fixed::from_int(20),
                high_km: Fixed::from_int(40),
            },
            SampleRole::MirrorCalibratable,
            e,
            nu,
        );
        // A derived band entirely stiffer than the observed interval misses it.
        let derived = RigidityBand::new(d_at(60), d_at(80)).expect("a band");
        assert_eq!(
            compare_to_row(derived, &row),
            HindcastComparison::Miss {
                sample_role: SampleRole::MirrorCalibratable,
                rung: ResidualRung::DefectHunt,
            },
            "disjoint bands are a miss at the residual law's first rung, carrying the partition, never a verdict"
        );
    }

    #[test]
    fn an_upper_bound_row_admits_a_soft_plate_and_refuses_a_stiff_one() {
        let (e, nu) = e_nu();
        // Te <= 12 km: the observed ceiling is D(12).
        let row = synthetic_row(
            ObservedElasticThickness::UpperBound {
                max_km: Fixed::from_int(12),
            },
            SampleRole::OutOfSample,
            e,
            nu,
        );
        // A soft derived band (below the ceiling) is consistent: the open lower side cannot be contradicted.
        let soft = RigidityBand::new(d_at(4), d_at(10)).expect("a band");
        assert_eq!(
            compare_to_row(soft, &row),
            HindcastComparison::Consistent {
                sample_role: SampleRole::OutOfSample
            },
            "a plate at or below the observed ceiling is consistent with an upper bound"
        );
        // A stiff derived band entirely above the ceiling misses.
        let stiff = RigidityBand::new(d_at(20), d_at(30)).expect("a band");
        assert_eq!(
            compare_to_row(stiff, &row),
            HindcastComparison::Miss {
                sample_role: SampleRole::OutOfSample,
                rung: ResidualRung::DefectHunt,
            },
            "a plate entirely stiffer than the observed ceiling misses an upper bound"
        );
    }

    #[test]
    fn a_lower_bound_row_admits_a_stiff_plate_and_refuses_a_soft_one() {
        let (e, nu) = e_nu();
        // Te >= 90 km: the observed floor is D(90).
        let row = synthetic_row(
            ObservedElasticThickness::LowerBound {
                min_km: Fixed::from_int(90),
            },
            SampleRole::OutOfSample,
            e,
            nu,
        );
        // A stiff derived band (above the floor) is consistent: the open upper side cannot be contradicted.
        let stiff = RigidityBand::new(d_at(100), d_at(120)).expect("a band");
        assert_eq!(
            compare_to_row(stiff, &row),
            HindcastComparison::Consistent {
                sample_role: SampleRole::OutOfSample
            },
            "a plate at or above the observed floor is consistent with a lower bound"
        );
        // A soft derived band entirely below the floor misses.
        let soft = RigidityBand::new(d_at(40), d_at(70)).expect("a band");
        assert_eq!(
            compare_to_row(soft, &row),
            HindcastComparison::Miss {
                sample_role: SampleRole::OutOfSample,
                rung: ResidualRung::DefectHunt,
            },
            "a plate entirely softer than the observed floor misses a lower bound"
        );
    }

    #[test]
    fn an_absent_modulus_row_refuses_rather_than_borrowing_a_pair() {
        // A Mars row has an absent pair, so it cannot enter rigidity space; the blockage is a finding.
        let mars = all_hindcast_rows()
            .into_iter()
            .find(|r| r.body == "Mars")
            .expect("a Mars row");
        let derived = RigidityBand::new(d_at(20), d_at(40)).expect("a band");
        assert_eq!(
            compare_to_row(derived, &mars),
            HindcastComparison::Refused {
                reason: ComparisonRefusal::ModuliAbsent
            },
            "an absent (E, nu) refuses; the out-of-sample comparison awaits the sourced pair"
        );
    }

    #[test]
    fn the_earth_rate_relation_refuses_for_a_loading_age() {
        // Earth's cited row is an age-rate relation with a present pair but no fixed Te.
        let earth = calmant_oceanic_seamounts();
        let derived = RigidityBand::new(d_at(20), d_at(40)).expect("a band");
        assert_eq!(
            compare_to_row(derived, &earth),
            HindcastComparison::Refused {
                reason: ComparisonRefusal::RateRelationNeedsLoadingAge
            },
            "the rate relation needs a loading age and an SD decision, both the consumer's call, so it refuses"
        );
    }

    #[test]
    fn every_cited_row_refuses_today_and_the_reasons_partition_one_and_sixteen() {
        // THE HONEST STATE of the out-of-sample comparison: with the rows as cited, no comparison can be made, and
        // the two documented blockages account for every refusal. One rate relation (Earth), sixteen absent moduli
        // (Mars and Venus). If a real comparison ever becomes possible, this count changes and this test with it.
        let derived = RigidityBand::new(d_at(20), d_at(40)).expect("a band");
        let results = compare_to_all_cited_rows(derived);
        assert_eq!(results.len(), 1 + 13 + 3);
        let rate = results
            .iter()
            .filter(|(_, _, c)| {
                matches!(
                    c,
                    HindcastComparison::Refused {
                        reason: ComparisonRefusal::RateRelationNeedsLoadingAge
                    }
                )
            })
            .count();
        let moduli = results
            .iter()
            .filter(|(_, _, c)| {
                matches!(
                    c,
                    HindcastComparison::Refused {
                        reason: ComparisonRefusal::ModuliAbsent
                    }
                )
            })
            .count();
        assert_eq!(
            rate, 1,
            "Earth's rate relation is the one loading-age refusal"
        );
        assert_eq!(
            moduli, 16,
            "Mars and Venus are the sixteen absent-moduli refusals"
        );
        assert_eq!(
            rate + moduli,
            results.len(),
            "every cited row refuses today, for one of exactly two documented reasons"
        );
    }
}
