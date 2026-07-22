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
    assert!(stdout.starts_with("receipt=civsim.planet.run.v8\ncomplete=false\n"));
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
        "refusal.0000.open_requirement.0001.obligation.0006=\"gap_law.chaos_protocol\"\n"
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
    assert!(stdout.contains("transcript=civsim.planet.transcript.v6\n"));
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
    assert_eq!(census.coverage_gap_ids().len(), 6);
    let radiation = census
        .variables()
        .find(|variable| variable.id() == "stellar_birth.radiation_flux_spectrum")
        .expect("the radiation coordinate is visible");
    assert_eq!(
        radiation.carrier_id(),
        "spectral_flux_density_per_log_frequency_field"
    );
    let mass_history = census
        .phenomena()
        .find(|phenomenon| phenomenon.id() == "stellar_birth.phenomenon.enclosed_mass_history")
        .expect("the mass-history gap is visible");
    assert!(mass_history.derivation_attempts().any(|attempt| {
        attempt
            .missing_dependency_ids()
            .iter()
            .any(|id| id == "collapse.initial_enclosed_mass_or_integration_boundary")
    }));
}
