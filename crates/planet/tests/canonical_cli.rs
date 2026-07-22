use std::process::{Command, Output};

use civsim_planet::{run_planet, sealed_absolute_physics_floor};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_run_planet"))
        .args(arguments)
        .output()
        .expect("the canonical planet binary runs")
}

#[test]
fn the_no_argument_binary_enters_the_floor_only_runner_and_refuses() {
    let output = run(&[]);
    let stdout = String::from_utf8(output.stdout).expect("receipt output is UTF-8");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(stdout.starts_with("receipt=civsim.planet.run.v10\ncomplete=false\n"));
    assert!(stdout.contains("absolute_floor_entries=3\n"));
    assert!(stdout.contains("representation.schema=\"civsim.units.si-representation.v1\"\n"));
    assert!(stdout.contains("event_count=6\n"));
    assert!(stdout.contains("event.0004.kind=stage_entered\n"));
    assert!(stdout.contains("event.0004.stage=star_disk_system\n"));
    assert!(stdout.contains("event.0005.kind=refused\n"));
    assert!(stdout.contains("refusal.0000.requirement=\"stellar_birth.realization_measure\"\n"));
    assert!(stdout.contains("refusal.0000.open_requirement_count=2\n"));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0000.id=\"stellar_birth.joint_physical_measure\"\n"
    ));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0001.id=\"stellar_birth.realization_coordinate_law\"\n"
    ));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0000.obligation.0009=\"gap_law.chaos_protocol\"\n"
    ));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0000.obligation.0016=\"open_stellar_state_and_projection_coverage\"\n"
    ));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0001.obligation.0006=\"gap_law.chaos_protocol\"\n"
    ));
    assert!(stdout
        .contains("refusal.0000.open_requirement.0001.obligation.0014=\"derivation_census\"\n"));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0001.obligation.0023=\"coordinate_totality_over_joint_support\"\n"
    ));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0001.obligation.0024=\"measure_consistent_push_forward\"\n"
    ));
    assert!(stdout.contains("refusal.0000.open_requirement.0000.analysis_count=1\n"));
    assert!(stdout.contains(
        "refusal.0000.open_requirement.0000.analysis.0000.kind=exact_dimensional_census\n"
    ));
    assert!(
        stdout.contains("refusal.0000.open_requirement.0000.analysis.0000.closure_effect=none\n")
    );
    assert!(
        stdout.contains("refusal.0000.open_requirement.0000.analysis.0000.coverage_claim=false\n")
    );
    assert!(stdout.contains("refusal.0000.open_requirement.0001.analysis_count=0\n"));
    assert!(stdout.contains(".exhaustion.gap.chaos_protocol=not_applicable\n"));
    assert!(stdout.contains("transcript=civsim.planet.transcript.v8\n"));
    assert!(!stdout.contains(".kind=contingency\n"));
    assert!(!stdout.contains(".kind=written_state\n"));
}

#[test]
fn repeating_the_front_door_is_byte_identical() {
    let first = run(&[]);
    let second = run(&[]);

    assert_eq!(first.status.code(), Some(2));
    assert_eq!(second.status.code(), Some(2));
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[test]
fn readiness_is_a_distinct_zero_floor_refusal() {
    let output = run(&["--readiness"]);
    let stdout = String::from_utf8(output.stdout).expect("receipt output is UTF-8");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert!(stdout.contains("absolute_floor_entries=0\n"));
    assert!(stdout.contains("refusal.0000.code=absolute_floor_required\n"));
    assert!(stdout.contains("refusal.0000.requirement=\"absolute_physics_floor\"\n"));
    assert!(!stdout.contains("stage.star_disk_system=refused\n"));
}

#[test]
fn caller_world_selectors_are_rejected_before_the_runner() {
    for arguments in [
        &["--seed", "1"][..],
        &["--profile", "local"][..],
        &["--world", "familiar"][..],
    ] {
        let output = run(arguments);
        let stderr = String::from_utf8(output.stderr).expect("diagnostic output is UTF-8");

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(stderr.contains("accepts no caller-authored world input"));
        assert!(stderr.contains(arguments[0]));
    }
}

#[test]
fn help_exposes_no_world_value_surface() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8(output.stdout).expect("help output is UTF-8");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert!(stdout.starts_with("run_planet [--readiness]\n"));
    assert!(stdout.contains("accepts no world values, profile, identity, or seed"));
}

#[test]
fn refusal_details_have_a_typed_read_only_api() {
    let floor = sealed_absolute_physics_floor().expect("the repository floor seals");
    let outcome = run_planet(&floor);
    let analysis = &outcome.receipt().refusals()[0].open_requirements()[0].analyses()[0];
    let census = analysis
        .exact_dimensional_census_view()
        .expect("the open joint measure carries an exact census");

    assert!(census.is_computed());
    assert_eq!(census.variables().len(), 31);
    assert_eq!(census.phenomena().len(), 7);
    assert_eq!(census.coverage_gap_ids().len(), 7);
    assert_eq!(
        census.structure_schema_id(),
        Some("civsim.planet.stellar-birth-structure.v2")
    );
    let component_registry = census
        .component_registry_schema()
        .expect("the component registry contract is visible");
    assert_eq!(
        component_registry.cardinality_rule_id(),
        "realization_coordinate_defined_from_joint_measure_support"
    );
    assert_eq!(
        component_registry.symmetry_rule_id(),
        "permutation_equivariant_multiset"
    );
    assert_eq!(
        component_registry.topology_label_authority_rule_id(),
        "derived_physical_relation_only"
    );
    let species_registry = census
        .species_registry_schema()
        .expect("the species registry contract is visible");
    assert_eq!(
        species_registry.membership_rule_id(),
        "floor_derived_only_or_named_refusal"
    );
    let stellar_state = census
        .stellar_state_schema()
        .expect("the open stellar-state contract is visible");
    assert_eq!(stellar_state.schema_id(), "civsim.planet.stellar-state.v1");
    assert_eq!(
        stellar_state.value_authority_rule_id(),
        "sealed_absolute_floor_and_derived_state_only"
    );
    assert_eq!(
        stellar_state.numeric_rule_id(),
        "deterministic_integer_or_exact_rational_causal_float_confirmation_only"
    );
    assert_eq!(
        stellar_state.trajectory_rule_id(),
        "lineage_preserving_complete_physical_state_history"
    );
    assert_eq!(
        stellar_state.lineage_rule_id(),
        "physical_birth_death_merge_split_lineage_or_named_refusal"
    );
    let coordinates = stellar_state.state_coordinate_registry();
    assert_eq!(
        coordinates.dimension_rule_id(),
        "exact_exponent_vector_bound_to_active_floor_basis"
    );
    assert_eq!(
        coordinates.identity_rule_id(),
        "canonical_semantics_basis_dimension_domain_tensor_normalization_reference_sector_dependency_digest"
    );
    assert_eq!(
        coordinates.collision_rule_id(),
        "complete_descriptor_collision_is_named_refusal"
    );
    let dimension_basis = coordinates.dimension_basis_registry();
    assert_eq!(
        dimension_basis.cardinality_rule_id(),
        "variable_cardinality"
    );
    assert_eq!(
        dimension_basis.exponent_encoding_rule_id(),
        "exact_signed_integer_vector_matching_basis_cardinality"
    );
    let sectors = stellar_state.interaction_sector_registry();
    assert_eq!(
        sectors.admission_rule_id(),
        "derive_first_buckingham_pi_gap_chaos_residual_unique_or_refusal"
    );
    assert_eq!(
        sectors.conservation_rule_id(),
        "global_cross_sector_transfer_ledger_balances"
    );
    assert_eq!(
        sectors.execution_rule_id(),
        "identity_blind_common_execution_path"
    );
    assert_eq!(
        sectors.identity_rule_id(),
        "canonical_law_dependency_basis_field_charge_coupling_validity_conservation_digest"
    );
    assert_eq!(
        sectors.collision_rule_id(),
        "complete_artifact_collision_is_named_refusal"
    );
    let regimes = stellar_state.physical_regime_registry();
    assert_eq!(
        regimes.predicate_admission_rule_id(),
        "every_entry_formula_threshold_and_use_derive_first_or_refusal"
    );
    assert_eq!(
        regimes.dispatch_rule_id(),
        "proof_identity_never_name_label_ordinal_or_priority"
    );
    let classifications = stellar_state.classification_registry();
    assert_eq!(classifications.causal_authority_rule_id(), "none");
    assert_eq!(
        classifications.boundary_rule_id(),
        "read_only_total_projection_from_one_immutable_causal_transcript"
    );
    assert_eq!(
        classifications.display_name_rule_id(),
        "nonidentifying_metadata_with_no_cross_registry_equivalence"
    );
    assert_eq!(census.index_domains().len(), 6);
    let log_frequency = census
        .index_domains()
        .find(|domain| domain.id() == "stellar_birth.domain.log_frequency")
        .expect("the log-frequency domain is visible");
    assert_eq!(
        log_frequency.reference_rule_id(),
        "log_ratio_reference_is_gauge_shift"
    );
    assert_eq!(log_frequency.support_rule_id(), "joint_measure_defined");
    assert_eq!(
        log_frequency.resolution_rule_id(),
        "convergence_derived_or_named_refusal"
    );
    assert_eq!(
        log_frequency.capacity_rule_id(),
        "engine_limit_is_named_refusal"
    );
    assert_eq!(census.carrier_schemas().len(), 11);
    let composition_carrier = census
        .carrier_schemas()
        .find(|carrier| carrier.id() == "species_number_fraction_simplex")
        .expect("the composition carrier is visible");
    assert_eq!(
        composition_carrier.measure_semantics_id(),
        "number_fraction_over_complete_species_registry"
    );
    assert_eq!(
        composition_carrier.support_rule_id(),
        "complete_registry_support_or_named_refusal"
    );
    let radiation = census
        .variables()
        .find(|variable| variable.id() == "stellar_birth.radiation_flux_spectrum")
        .expect("the radiation coordinate is visible");
    assert_eq!(
        radiation.carrier_id(),
        "spectral_flux_density_per_log_frequency_field"
    );
    let composition = census
        .variables()
        .find(|variable| variable.id() == "stellar_birth.composition.species_number_fraction_field")
        .expect("the number-fraction coordinate is visible");
    assert_eq!(composition.carrier_id(), "species_number_fraction_simplex");
    let mass_history = census
        .phenomena()
        .find(|phenomenon| {
            phenomenon.id() == "stellar_birth.phenomenon.material_element_mass_history"
        })
        .expect("the mass-history gap is visible");
    assert!(mass_history.derivation_attempts().any(|attempt| {
        attempt
            .missing_dependency_ids()
            .iter()
            .any(|id| id == "collapse.initial_material_mass_or_integration_boundary")
    }));
}

#[test]
fn the_structure_block_is_versioned_and_repeated_identically_in_both_refusals() {
    let output = run(&[]);
    let stdout = String::from_utf8(output.stdout).expect("receipt output is UTF-8");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stdout
            .matches(".structure.schema=\"civsim.planet.stellar-birth-structure.v2\"\n")
            .count(),
        2
    );
    assert_eq!(
        stdout.matches(".structure.index_domain_count=6\n").count(),
        2
    );
    assert_eq!(stdout.matches(".structure.carrier_count=11\n").count(), 2);
    assert!(stdout.contains(
        ".structure.species_registry.membership_rule=floor_derived_only_or_named_refusal\n"
    ));
    assert!(stdout.contains(
        ".structure.component_registry.symmetry_rule=permutation_equivariant_multiset\n"
    ));
    assert_eq!(
        stdout
            .matches(".structure.stellar_state.schema=\"civsim.planet.stellar-state.v1\"\n")
            .count(),
        2
    );
    assert!(stdout.contains(
        ".structure.stellar_state.interaction_sector_registry.execution_rule=identity_blind_common_execution_path\n"
    ));
    assert!(stdout.contains(
        ".structure.stellar_state.state_coordinate_registry.dimension_basis_registry.cardinality_rule=variable_cardinality\n"
    ));
    assert!(stdout.contains(
        ".structure.stellar_state.physical_regime_registry.dispatch_rule=proof_identity_never_name_label_ordinal_or_priority\n"
    ));
    assert!(stdout
        .contains(".structure.stellar_state.classification_registry.causal_authority_rule=none\n"));
    assert!(stdout.contains(".structure.index_domain.0002.support_rule=joint_measure_defined\n"));
    assert!(!stdout.contains(".structure.index_domain.0002.cardinality_rule="));
    assert!(!stdout.contains("collapse_shell"));
    assert!(!stdout.contains("enclosed_mass"));
    for named_class in [
        "magnetar",
        "population_iii",
        "luminosity_class_iii",
        "white_dwarf",
        "thaumic",
    ] {
        assert!(!stdout.contains(named_class));
    }
}
