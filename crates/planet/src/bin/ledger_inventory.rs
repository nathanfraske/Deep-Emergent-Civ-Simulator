use civsim_planet::audited_substrate_ledger;
use std::path::Path;

const INVENTORY_PATH: &str = "docs/working/CANONICAL_LEDGER_INVENTORY.txt";

fn rendered_inventory() -> Result<String, String> {
    audited_substrate_ledger()
        .map(|ledger| ledger.inventory().to_string())
        .map_err(|error| format!("canonical ledger refused: {error}"))
}

fn main() {
    let mode = std::env::args().nth(1);
    if matches!(mode.as_deref(), Some("--help" | "-h")) {
        println!(
            "ledger_inventory [--write|--check]\n\nPrint, regenerate, or verify {INVENTORY_PATH}."
        );
        return;
    }

    let inventory = match rendered_inventory() {
        Ok(inventory) => inventory,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    match mode.as_deref() {
        None => print!("{inventory}"),
        Some("--write") => {
            if let Err(error) = std::fs::write(Path::new(INVENTORY_PATH), &inventory) {
                eprintln!("could not write {INVENTORY_PATH}: {error}");
                std::process::exit(1);
            }
            println!("wrote {INVENTORY_PATH}");
        }
        Some("--check") => match std::fs::read_to_string(Path::new(INVENTORY_PATH)) {
            Ok(current) if current == inventory => {
                println!("canonical ledger inventory current");
            }
            Ok(_) => {
                eprintln!(
                    "canonical ledger inventory stale at {INVENTORY_PATH}; run ledger_inventory --write"
                );
                std::process::exit(1);
            }
            Err(error) => {
                eprintln!("could not read {INVENTORY_PATH}: {error}");
                std::process::exit(1);
            }
        },
        Some(other) => {
            eprintln!("unknown argument: {other}");
            std::process::exit(2);
        }
    }
}
