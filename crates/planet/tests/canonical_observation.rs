use civsim_ledger::{Provenance, Tier};
use civsim_planet::{
    run_planet, sealed_absolute_physics_floor, RefusalCode, RunEventKind, Stage, StageStatus,
};

#[test]
fn public_observation_exposes_one_typed_floor_to_refusal_chain() {
    let floor = sealed_absolute_physics_floor().expect("the repository floor seals");
    let outcome = run_planet(&floor);
    let observation = outcome.observation();
    let receipt = observation.receipt();
    let transcript = receipt.transcript();

    assert!(outcome.is_refused());
    assert!(!outcome.is_complete());
    assert!(!observation.is_complete());
    assert!(outcome.snapshot().is_none());
    assert!(observation.snapshot().is_none());
    assert!(std::ptr::eq(receipt, outcome.receipt()));
    assert!(std::ptr::eq(
        observation
            .refusal_receipt()
            .expect("a refused observation exposes its sealed receipt"),
        outcome.receipt()
    ));

    assert_eq!(receipt.absolute_floor_entries(), 3);
    assert_eq!(transcript.declared_floor_entries(), 3);
    assert!(transcript.is_closed());
    assert!(receipt.realization_id().is_none());
    assert!(transcript.realization_id().is_none());
    assert_eq!(transcript.contingency_draws().count(), 0);
    assert_eq!(transcript.events().len(), 6);
    assert!(matches!(
        transcript.events()[0].kind(),
        RunEventKind::FloorValue(_)
    ));
    assert!(matches!(
        transcript.events()[1].kind(),
        RunEventKind::FloorValue(_)
    ));
    assert!(matches!(
        transcript.events()[2].kind(),
        RunEventKind::FloorValue(_)
    ));
    assert!(matches!(
        transcript.events()[3].kind(),
        RunEventKind::DerivedValue(_)
    ));
    assert!(matches!(
        transcript.events()[4].kind(),
        RunEventKind::StageEntered {
            stage: Stage::StarDiskSystem
        }
    ));
    assert!(matches!(
        transcript.events()[5].kind(),
        RunEventKind::Refused {
            stage: Some(Stage::StarDiskSystem),
            ..
        }
    ));

    let floor_records = transcript
        .events()
        .iter()
        .filter_map(|event| match event.kind() {
            RunEventKind::FloorValue(record) => Some(record),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(floor_records.len(), 3);
    assert_eq!(
        floor_records
            .iter()
            .map(|record| record.quantity_id())
            .collect::<Vec<_>>(),
        ["fundamental.alpha", "fundamental.G", "fundamental.m_e"]
    );
    for record in floor_records {
        assert_eq!(record.tier(), Tier::Universal);
        assert_eq!(record.provenance(), Provenance::Measured);
        assert_eq!(record.provenance().bracket_tag(), Some("[M]"));
        assert!(record.ancestry().is_none());

        let measurement = record
            .measurement()
            .expect("every measured floor record exposes source custody");
        assert!(!measurement.source_id().is_empty());
        assert!(!measurement.source_anchor().is_empty());
        assert!(!measurement.source_decimal().is_empty());
        assert!(!measurement.uncertainty_kind().is_empty());
        assert!(!measurement.uncertainty_decimal().is_empty());
        assert_eq!(measurement.source_sha256().len(), 64);
        assert!(measurement
            .source_sha256()
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit()));

        let exhaustion = record
            .exhaustion()
            .expect("every irreducible floor record exposes derive-first exhaustion");
        assert_eq!(exhaustion.entry_id, record.quantity_id());
        assert!(!exhaustion.phenomenon.is_empty());
        assert!(!exhaustion.derivation_attempts.is_empty());
        assert!(!exhaustion.residual_slot.is_empty());
        assert!(exhaustion.buckingham_pi_groups > 0);
        assert!(!exhaustion.gap_law.reference_validity.is_empty());
        assert!(!exhaustion.gap_law.gap_dispatch.is_empty());
        assert!(!exhaustion.gap_law.smooth_systematics.is_empty());
        assert!(!exhaustion.gap_law.scale_free_limit.is_empty());
        assert_eq!(
            exhaustion.gap_law.chaos_protocol.kind_id(),
            "not_applicable"
        );
        assert!(!exhaustion.gap_law.chaos_protocol.evidence().is_empty());
        assert!(!exhaustion.residual_law.conservation.is_empty());
        assert!(!exhaustion.residual_law.disequilibrium.is_empty());
        assert!(!exhaustion.residual_law.fluctuation_dissipation.is_empty());
        assert!(!exhaustion.residual_law.dimensional_analysis.is_empty());
    }

    let derived_records = transcript
        .events()
        .iter()
        .filter_map(|event| match event.kind() {
            RunEventKind::DerivedValue(record) => Some(record),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(derived_records.len(), 1);
    let permittivity = derived_records[0];
    assert_eq!(permittivity.quantity_id(), "derived.eps_0");
    assert_eq!(permittivity.tier(), Tier::Universal);
    assert_eq!(permittivity.provenance(), Provenance::Derived);
    assert_eq!(permittivity.provenance().bracket_tag(), Some("[D]"));
    assert!(permittivity.measurement().is_none());
    assert!(permittivity.exhaustion().is_none());
    let ancestry = permittivity
        .ancestry()
        .expect("a derived record exposes its direct causal ancestry");
    assert_eq!(ancestry.law_id(), "units.execution.eps_0.definition");
    assert!(ancestry.expression().is_some_and(|value| !value.is_empty()));
    assert!(ancestry.evaluation_id().is_some());
    assert_eq!(
        ancestry
            .input_ids()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "representation.e",
            "fundamental.alpha",
            "representation.h",
            "representation.c",
        ]
    );

    assert!(transcript.events().iter().all(|event| !matches!(
        event.kind(),
        RunEventKind::Contingency(_) | RunEventKind::WrittenState(_)
    )));

    assert_eq!(
        receipt
            .stages()
            .iter()
            .map(|stage| (stage.stage(), stage.status()))
            .collect::<Vec<_>>(),
        [
            (Stage::StarDiskSystem, StageStatus::Refused),
            (Stage::AssemblyComposition, StageStatus::NotReached),
            (Stage::OrbitalSecularMoons, StageStatus::NotReached),
            (Stage::YoungThermalMaterials, StageStatus::NotReached),
            (Stage::GeodynamicsDeepTime, StageStatus::NotReached),
            (Stage::LoadFlexure, StageStatus::NotReached),
            (Stage::Snapshot, StageStatus::NotReached),
        ]
    );
    let stage_receipt = &receipt.stages()[0];
    let entered_id = stage_receipt
        .entered_event()
        .expect("Stage 1 records its entry event");
    let terminal_id = stage_receipt
        .terminal_event()
        .expect("Stage 1 records its refusal event");
    let entered_event = transcript
        .events()
        .iter()
        .find(|event| event.id() == entered_id)
        .expect("the stage entry points into the transcript");
    assert!(matches!(
        entered_event.kind(),
        RunEventKind::StageEntered {
            stage: Stage::StarDiskSystem
        }
    ));
    let terminal_event = transcript
        .events()
        .iter()
        .find(|event| event.id() == terminal_id)
        .expect("the stage terminal points into the transcript");

    assert_eq!(receipt.refusals().len(), 1);
    let refusal = &receipt.refusals()[0];
    assert_eq!(refusal.code(), RefusalCode::MissingStageRequirement);
    assert_eq!(refusal.stage(), Some(Stage::StarDiskSystem));
    assert_eq!(
        refusal.requirement_id(),
        Some("stellar_birth.realization_measure")
    );
    match terminal_event.kind() {
        RunEventKind::Refused {
            stage: Some(Stage::StarDiskSystem),
            refusals,
        } => assert_eq!(refusals, receipt.refusals()),
        other => panic!("Stage 1 terminal event is not its typed refusal: {other:?}"),
    }

    let frontier = refusal.open_requirements();
    assert_eq!(
        frontier
            .iter()
            .map(|requirement| requirement.requirement_id())
            .collect::<Vec<_>>(),
        [
            "stellar_birth.joint_physical_measure",
            "stellar_birth.realization_coordinate_law",
        ]
    );
    assert_eq!(
        frontier[0]
            .obligations()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "derivation_census",
            "buckingham_pi_census",
            "evidence_custody",
            "typed_support",
            "normalization",
            "conditioning",
            "correlation_preservation",
            "uncertainty_propagation",
            "gap_law",
            "gap_law.chaos_protocol",
            "residual_law",
            "unique_residual_slot_if_irreducible",
            "absolute_floor_binding",
            "artifact_schema_version",
            "semantic_checker_version",
            "dependency_digest",
            "open_stellar_state_and_projection_coverage",
            "dimensional_closure",
            "dependency_admission",
            "validity_domain_proof",
            "global_conservation_proof",
            "observer_independence",
            "ordering_independence",
            "presentation_identity_and_taxonomy_independence",
        ]
    );
    assert_eq!(
        frontier[1]
            .obligations()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "versioned_coordinate_semantics",
            "canonical_content_coordinate",
            "observer_independence",
            "ordering_independence",
            "exact_replay",
            "gap_law",
            "gap_law.chaos_protocol",
            "residual_law",
            "unique_residual_slot_if_irreducible",
            "absolute_floor_binding",
            "artifact_schema_version",
            "semantic_checker_version",
            "dependency_digest",
            "joint_measure_binding",
            "derivation_census",
            "buckingham_pi_census",
            "evidence_custody",
            "typed_support",
            "dimensional_closure",
            "dependency_admission",
            "validity_domain_proof",
            "global_conservation_proof",
            "open_stellar_state_and_projection_coverage",
            "coordinate_totality_over_joint_support",
            "measure_consistent_push_forward",
            "presentation_identity_and_taxonomy_independence",
        ]
    );
}
