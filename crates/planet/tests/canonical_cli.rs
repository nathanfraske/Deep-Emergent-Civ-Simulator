use std::process::{Command, Output};

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
    assert!(stdout.starts_with("receipt=civsim.planet.run.v5\ncomplete=false\n"));
    assert!(stdout.contains("absolute_floor_entries=3\n"));
    assert!(stdout.contains("representation.schema=\"civsim.units.si-representation.v1\"\n"));
    assert!(stdout.contains("event_count=6\n"));
    assert!(stdout.contains("event.0004.kind=stage_entered\n"));
    assert!(stdout.contains("event.0004.stage=star_disk_system\n"));
    assert!(stdout.contains("event.0005.kind=refused\n"));
    assert!(stdout.contains(
        "refusal.missing_stage_requirement.requirement=\"stellar_birth.realization_measure\"\n"
    ));
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
    assert!(
        stdout.contains("refusal.absolute_floor_required.requirement=\"absolute_physics_floor\"\n")
    );
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
