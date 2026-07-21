use civsim_planet::{
    readiness_receipt, run_planet, sealed_absolute_physics_floor, PlanetRunOutcome,
};

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--help" | "-h") if args.next().is_none() => {
            println!(
                "run_planet [--readiness]\n\nThe canonical run accepts no world values, profile, identity, or seed. With no arguments it constructs the repository-owned sealed absolute physics floor and enters the seven-stage runner. --readiness reports the missing-floor boundary without entering a stage."
            );
            return;
        }
        Some("--readiness") if args.next().is_none() => {
            print!("{}", readiness_receipt());
            std::process::exit(2);
        }
        Some(argument) => {
            eprintln!(
                "run_planet accepts no caller-authored world input; unsupported argument: {argument}"
            );
            std::process::exit(2);
        }
        None => {}
    }

    let floor = match sealed_absolute_physics_floor() {
        Ok(floor) => floor,
        Err(error) => {
            eprintln!("could not construct the repository-owned absolute physics floor: {error}");
            std::process::exit(2);
        }
    };
    let outcome = run_planet(&floor);
    print!("{}", outcome.receipt());
    if matches!(outcome, PlanetRunOutcome::Refused(_)) {
        std::process::exit(2);
    }
}
