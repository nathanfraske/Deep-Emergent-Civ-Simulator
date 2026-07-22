use super::{
    floor_magnitudes::AuditedFloorView, preflight, star_disk_system, OpenRequirement,
    PlanetSnapshot, Refusal, RunReceipt, RunTranscript, Stage,
};
use civsim_ledger::AbsolutePhysicsFloor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanetRunOutcome {
    Complete {
        receipt: RunReceipt,
        snapshot: PlanetSnapshot,
    },
    Refused(RunReceipt),
}

impl PlanetRunOutcome {
    pub fn receipt(&self) -> &RunReceipt {
        match self {
            Self::Complete { receipt, .. } | Self::Refused(receipt) => receipt,
        }
    }

    /// Immutable state is present only for a completed physical closure.
    pub fn snapshot(&self) -> Option<&PlanetSnapshot> {
        match self {
            Self::Complete { snapshot, .. } => Some(snapshot),
            Self::Refused(_) => None,
        }
    }
}

/// Run the canonical pipeline from an admitted absolute floor or return a
/// visible refusal.
///
/// The signature makes a generic accounting ledger, caller-supplied world state, an authored seed, and
/// unadmitted residue unavailable at this boundary. Written state and contingency may be generated only
/// inside the run. The current audited floor has no stellar-birth realization measure, so a valid floor
/// enters Stage 1 and returns that named refusal without producing a partial snapshot.
pub fn run_planet(floor: &AbsolutePhysicsFloor) -> PlanetRunOutcome {
    let refusals = preflight(floor);
    if !refusals.is_empty() {
        return PlanetRunOutcome::Refused(RunReceipt::refused(floor.len(), refusals));
    }

    let floor_view = match AuditedFloorView::from_floor(floor) {
        Ok(floor_view) => floor_view,
        Err(error) => {
            return PlanetRunOutcome::Refused(RunReceipt::refused(
                floor.len(),
                vec![Refusal::floor_magnitude_unavailable(error.to_string())],
            ));
        }
    };
    debug_assert_eq!(floor_view.len(), floor.len());

    let mut transcript = match RunTranscript::from_audited_floor(floor, &floor_view) {
        Ok(transcript) => transcript,
        Err(error) => {
            return PlanetRunOutcome::Refused(RunReceipt::refused(
                floor.len(),
                vec![Refusal::transcript_invariant(error.to_string())],
            ));
        }
    };
    if let Err(error) = transcript.enter_stage(Stage::StarDiskSystem) {
        return PlanetRunOutcome::Refused(RunReceipt::refused(
            floor.len(),
            vec![Refusal::transcript_invariant(error.to_string())],
        ));
    }

    if let Err(reason) = star_disk_system::require_birth_measure(&floor_view) {
        let open_requirements: Vec<_> = reason
            .open_frontier()
            .iter()
            .map(|requirement| {
                let obligations: Vec<_> = requirement
                    .obligations()
                    .iter()
                    .map(|obligation| obligation.id())
                    .collect();
                OpenRequirement::with_analyses(
                    requirement.requirement_id(),
                    &obligations,
                    requirement.analyses().to_vec(),
                )
            })
            .collect();
        let refusal = Refusal::missing_stage_requirement_frontier(
            Stage::StarDiskSystem,
            reason.requirement_id(),
            open_requirements,
        );
        return PlanetRunOutcome::Refused(close_refused_transcript(
            floor.len(),
            transcript,
            Stage::StarDiskSystem,
            vec![refusal],
        ));
    }

    PlanetRunOutcome::Refused(close_refused_transcript(
        floor.len(),
        transcript,
        Stage::StarDiskSystem,
        vec![Refusal::pipeline_incomplete(
            Stage::StarDiskSystem,
            "the star, disk, and system implementation is not complete",
        )],
    ))
}

fn close_refused_transcript(
    absolute_floor_entries: usize,
    transcript: RunTranscript,
    stage: Stage,
    refusals: Vec<Refusal>,
) -> RunReceipt {
    match RunReceipt::refused_with_transcript(
        absolute_floor_entries,
        transcript,
        Some(stage),
        refusals,
    ) {
        Ok(receipt) => receipt,
        Err(error) => RunReceipt::refused(
            absolute_floor_entries,
            vec![Refusal::transcript_invariant(error.to_string())],
        ),
    }
}

/// A command-line readiness receipt when no admitted absolute floor was
/// supplied.
pub fn readiness_receipt() -> RunReceipt {
    RunReceipt::refused(0, vec![Refusal::absolute_floor_required()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::{sealed_absolute_physics_floor, RefusalCode, RunEventKind, StageStatus};
    use civsim_ledger::{
        ChaosProtocolReceipt, DerivationExhaustionReceipt, Entry, GapLawReceipt, Ledger,
        Provenance, ResidualLawReceipt, Tier,
    };

    fn physical_floor() -> AbsolutePhysicsFloor {
        sealed_absolute_physics_floor().expect("the physical catalog is admissible")
    }

    #[test]
    fn readiness_without_an_absolute_floor_is_never_success() {
        let receipt = readiness_receipt();
        assert!(!receipt.is_complete());
        assert_eq!(
            receipt.refusals()[0].code(),
            RefusalCode::AbsoluteFloorRequired
        );
    }

    #[test]
    fn a_refused_run_exposes_no_snapshot() {
        let outcome = PlanetRunOutcome::Refused(readiness_receipt());
        assert!(outcome.snapshot().is_none());
    }

    #[test]
    fn no_authored_identity_or_seed_enters_a_refused_run() {
        let outcome = run_planet(&physical_floor());

        assert!(outcome.receipt().realization_id().is_none());
        assert_eq!(
            outcome.receipt().transcript().contingency_draws().count(),
            0
        );
        let refusal = &outcome.receipt().refusals()[0];
        assert_eq!(
            refusal.requirement_id(),
            Some("stellar_birth.realization_measure")
        );
        assert_eq!(
            refusal
                .open_requirements()
                .iter()
                .map(OpenRequirement::requirement_id)
                .collect::<Vec<_>>(),
            vec![
                "stellar_birth.joint_physical_measure",
                "stellar_birth.realization_coordinate_law",
            ]
        );
        assert!(refusal.open_requirements().iter().all(|requirement| {
            requirement
                .obligations()
                .iter()
                .any(|obligation| obligation == "gap_law.chaos_protocol")
        }));
        assert_eq!(refusal.open_requirements()[0].analyses().len(), 1);
        assert!(refusal.open_requirements()[1].analyses().is_empty());
        assert_eq!(outcome.receipt().stages()[0].status(), StageStatus::Refused);
        assert!(outcome.receipt().stages()[1..]
            .iter()
            .all(|stage| stage.status() == StageStatus::NotReached));
        let events = outcome.receipt().transcript().events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event.kind(), RunEventKind::FloorValue(_)))
                .count(),
            3
        );
        assert!(matches!(
            events[4].kind(),
            RunEventKind::StageEntered {
                stage: Stage::StarDiskSystem
            }
        ));
        assert!(matches!(
            events[5].kind(),
            RunEventKind::Refused {
                stage: Some(Stage::StarDiskSystem),
                ..
            }
        ));
        assert!(outcome.snapshot().is_none());
    }

    #[test]
    fn universal_only_catalog_refuses_at_the_missing_stellar_birth_measure() {
        let floor = physical_floor();
        let outcome = run_planet(&floor);

        assert_eq!(outcome.receipt().absolute_floor_entries(), floor.len());
        let refusal = &outcome.receipt().refusals()[0];
        assert_eq!(
            refusal.requirement_id(),
            Some("stellar_birth.realization_measure")
        );
        assert_eq!(refusal.open_requirements().len(), 2);
    }

    #[test]
    fn the_front_door_exposes_no_seed_that_could_bypass_the_missing_physical_measure() {
        let floor = physical_floor();
        let first = run_planet(&floor);
        let second = run_planet(&floor);

        assert_eq!(first.receipt().refusals(), second.receipt().refusals());
        assert_eq!(first.receipt().stages(), second.receipt().stages());
        assert_eq!(first.receipt().transcript(), second.receipt().transcript());
        assert!(first.snapshot().is_none());
        assert!(second.snapshot().is_none());
    }

    #[test]
    fn repeating_the_same_refused_run_is_bit_readable_and_identical() {
        let floor = physical_floor();
        let first = run_planet(&floor);
        let second = run_planet(&floor);

        assert_eq!(first.receipt(), second.receipt());
        assert_eq!(first.receipt().to_string(), second.receipt().to_string());
    }

    #[test]
    fn receipt_and_transcript_share_one_open_requirement_wire_shape() {
        let text = run_planet(&physical_floor()).receipt().to_string();
        let receipt_prefix = "refusal.0000.open_requirement.";
        let transcript_prefix = "event.0005.reason.0000.open_requirement.";
        let receipt_payload = text
            .lines()
            .filter_map(|line| line.strip_prefix(receipt_prefix))
            .collect::<Vec<_>>();
        let transcript_payload = text
            .lines()
            .filter_map(|line| line.strip_prefix(transcript_prefix))
            .collect::<Vec<_>>();

        assert!(!receipt_payload.is_empty());
        assert_eq!(receipt_payload, transcript_payload);
    }

    #[test]
    fn structurally_admitted_but_unaudited_floor_cannot_run() {
        let ledger = Ledger::build([Entry {
            id: "fundamental.cited_knob".into(),
            tier: Tier::Universal,
            provenance: Provenance::Measured,
            inputs: vec![],
        }])
        .unwrap();
        let receipt = DerivationExhaustionReceipt {
            entry_id: "fundamental.cited_knob".into(),
            phenomenon: "fixture".into(),
            derivation_attempts: vec!["derive-first fixture attempt".into()],
            residual_slot: "fixture.slot".into(),
            buckingham_pi_groups: 1,
            gap_law: GapLawReceipt {
                reference_validity: "fixture evidence".into(),
                gap_dispatch: "fixture evidence".into(),
                smooth_systematics: "fixture evidence".into(),
                scale_free_limit: "fixture evidence".into(),
                chaos_protocol: ChaosProtocolReceipt::NotApplicable {
                    basis: "fixture has no dynamical branch".into(),
                },
            },
            residual_law: ResidualLawReceipt {
                conservation: "fixture evidence".into(),
                disequilibrium: "fixture evidence".into(),
                fluctuation_dissipation: "fixture evidence".into(),
                dimensional_analysis: "fixture evidence".into(),
            },
        };
        let floor = AbsolutePhysicsFloor::admit(ledger, [receipt])
            .expect("structural admission does not claim catalog authority");
        let outcome = run_planet(&floor);

        assert_eq!(outcome.receipt().refusals().len(), 1);
        assert_eq!(
            outcome.receipt().refusals()[0].code(),
            RefusalCode::FloorCatalogMismatch
        );
        assert!(outcome
            .receipt()
            .stages()
            .iter()
            .all(|stage| stage.status() == StageStatus::NotReached));
        assert!(outcome.snapshot().is_none());
    }
}
