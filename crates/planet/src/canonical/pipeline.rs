use super::{
    floor_magnitudes::AuditedFloorView, preflight, star_disk_system, OpenRequirement,
    PlanetSnapshot, Refusal, RunReceipt, RunTranscript, Stage,
};
use civsim_ledger::AbsolutePhysicsFloor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetRunOutcome {
    state: PlanetRunState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlanetRunState {
    #[expect(
        dead_code,
        reason = "armed for the first complete canonical planet snapshot"
    )]
    Complete(PlanetSnapshot),
    Refused(RunReceipt),
}

impl PlanetRunOutcome {
    fn refused(receipt: RunReceipt) -> Self {
        Self {
            state: PlanetRunState::Refused(receipt),
        }
    }

    pub fn receipt(&self) -> &RunReceipt {
        match &self.state {
            PlanetRunState::Complete(snapshot) => snapshot.receipt(),
            PlanetRunState::Refused(receipt) => receipt,
        }
    }

    /// Immutable state is present only for a completed physical closure.
    pub fn snapshot(&self) -> Option<&PlanetSnapshot> {
        match &self.state {
            PlanetRunState::Complete(snapshot) => Some(snapshot),
            PlanetRunState::Refused(_) => None,
        }
    }

    pub const fn is_complete(&self) -> bool {
        matches!(&self.state, PlanetRunState::Complete(_))
    }

    pub const fn is_refused(&self) -> bool {
        matches!(&self.state, PlanetRunState::Refused(_))
    }

    /// Seal this outcome behind the immutable observer boundary.
    pub const fn observation(&self) -> super::PlanetObservation<'_> {
        match &self.state {
            PlanetRunState::Complete(snapshot) => super::PlanetObservation::from_snapshot(snapshot),
            PlanetRunState::Refused(receipt) => super::PlanetObservation::from_refusal(receipt),
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
        return PlanetRunOutcome::refused(RunReceipt::refused(floor.len(), refusals));
    }

    // Seal the noncausal representation before constructing the execution view.
    // If it fails, do not ask the units layer to project it again through that
    // view; emit the one typed, value-free refusal instead.
    let mut transcript = match RunTranscript::empty(floor.len()) {
        Ok(transcript) => transcript,
        Err(error) => {
            return PlanetRunOutcome::refused(RunReceipt::representation_unavailable(
                floor.len(),
                error.to_string(),
            ));
        }
    };

    let floor_view = match AuditedFloorView::from_floor(floor) {
        Ok(floor_view) => floor_view,
        Err(error) => {
            return PlanetRunOutcome::refused(RunReceipt::refused(
                floor.len(),
                vec![Refusal::floor_magnitude_unavailable(error.to_string())],
            ));
        }
    };
    debug_assert_eq!(floor_view.len(), floor.len());

    if let Err(error) = transcript.append_audited_floor(floor, &floor_view) {
        return PlanetRunOutcome::refused(RunReceipt::refused(
            floor.len(),
            vec![Refusal::transcript_invariant(error.to_string())],
        ));
    }
    if let Err(error) = transcript.enter_stage(Stage::StarDiskSystem) {
        return PlanetRunOutcome::refused(RunReceipt::refused(
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
        return PlanetRunOutcome::refused(close_refused_transcript(
            floor.len(),
            transcript,
            Stage::StarDiskSystem,
            vec![refusal],
        ));
    }

    PlanetRunOutcome::refused(close_refused_transcript(
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
    fn a_refused_run_exposes_no_snapshot() {
        let outcome = run_planet(&physical_floor());
        assert!(outcome.snapshot().is_none());
        assert_eq!(
            outcome.receipt().refusals()[0].code(),
            RefusalCode::MissingStageRequirement
        );
    }

    #[test]
    fn unavailable_representation_still_has_a_visible_refusal_outcome() {
        let outcome = PlanetRunOutcome::refused(RunReceipt::representation_unavailable(
            3,
            "SI representation projection failed: injected pipeline failure".to_owned(),
        ));
        let observation = outcome.observation();

        assert!(outcome.is_refused());
        assert!(!outcome.is_complete());
        assert!(outcome.snapshot().is_none());
        assert!(!observation.is_complete());
        assert!(observation.snapshot().is_none());
        assert!(std::ptr::eq(observation.receipt(), outcome.receipt()));
        assert_eq!(
            observation
                .refusal_receipt()
                .expect("the representation failure remains observable")
                .refusals()[0]
                .code(),
            RefusalCode::RepresentationUnavailable
        );
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
        assert_eq!(refusal.open_requirements()[0].analyses().len(), 2);
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
    fn floor_arrival_order_cannot_steer_the_receipt_or_observer_bytes() {
        let canonical = physical_floor();
        let mut entries = canonical.entries().cloned().collect::<Vec<_>>();
        entries.reverse();
        let receipts = canonical
            .entries()
            .map(|entry| {
                canonical
                    .receipt(&entry.id)
                    .expect("each sealed floor leaf has one receipt")
                    .clone()
            })
            .collect::<Vec<_>>();
        let reordered = AbsolutePhysicsFloor::admit(
            Ledger::build(entries).expect("reordered floor remains structurally valid"),
            receipts,
        )
        .expect("arrival order is not a physical input");

        let expected = run_planet(&canonical);
        let observed = run_planet(&reordered);

        assert_eq!(expected.receipt(), observed.receipt());
        assert_eq!(
            expected.observation().receipt().to_string(),
            observed.observation().receipt().to_string()
        );
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
