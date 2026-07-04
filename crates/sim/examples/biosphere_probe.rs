// A read-only probe over the pre-dawn biosphere genesis: a longer, richer, magical world, with each
// surviving species dumped in full (trophic label, temperament, weapons, covering, senses, locomotion,
// mass, and diet), so a reader can see how detailed the emergent ecology actually is. Not canonical
// state, a development inspection tool; float is used for display only.

use std::collections::BTreeMap;

use civsim_core::Fixed;
use civsim_sim::anatomy::{temperament_word, BodyPlanRegistry, WorldProfile};
use civsim_sim::biosphere::{trophic_label, SourceRef, Species};
use civsim_sim::epoch::EpochParams;
use civsim_sim::genesis::{genesis, GenesisParams};
use civsim_sim::lineage::SpeciesId;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let seed: u64 = argv
        .get(1)
        .and_then(|s| {
            s.strip_prefix("0x")
                .and_then(|h| u64::from_str_radix(h, 16).ok())
                .or_else(|| s.parse().ok())
        })
        .unwrap_or(0x5CA1E);
    let gens: u64 = argv.get(2).and_then(|s| s.parse().ok()).unwrap_or(160);

    let mut params = GenesisParams::dev_default();
    params.width = 80;
    params.height = 48;
    params.epoch = EpochParams {
        generations: gens,
        ..params.epoch
    };
    params.profile = WorldProfile::magical();

    let world = genesis(seed, &params);
    let reg = BodyPlanRegistry::dev_default();

    println!(
        "seed 0x{seed:X}  world {}x{}  regions {}  generations {gens}  species {} (alive {})  hash {:032x}",
        params.width, params.height, world.regions.len(), world.species(), world.alive(), world.state_hash()
    );

    // Tallies + a curated interesting set.
    let mut labels: BTreeMap<&str, u32> = BTreeMap::new();
    let mut locos: BTreeMap<String, u32> = BTreeMap::new();
    let mut magical = 0u32;
    let mut interesting: Vec<String> = Vec::new();
    let magic_names = [
        "mana-lash",
        "curse-touch",
        "ember-breath",
        "frost-fang",
        "mana-ward",
        "stone-skin",
        "phase-hide",
        "mana-sight",
        "aura-sense",
        "levitate",
        "blink",
    ];

    for ((rx, ry), rb) in &world.regions {
        let map: BTreeMap<SpeciesId, Species> = rb
            .biosphere
            .species
            .ids()
            .map(|id| (id, rb.biosphere.species.get(id).unwrap().clone()))
            .collect();
        for (&id, sp) in &map {
            if sp.extinct {
                continue;
            }
            let label = trophic_label(&map, id);
            *labels.entry(label).or_default() += 1;
            let bp = &sp.body_plan;
            let weapons: Vec<&str> = bp
                .weapons
                .iter()
                .map(|p| BodyPlanRegistry::name(&reg.weapons, p.kind))
                .collect();
            let senses: Vec<&str> = bp
                .senses
                .iter()
                .map(|p| BodyPlanRegistry::name(&reg.senses, p.kind))
                .collect();
            let loco: Vec<&str> = bp
                .locomotion
                .iter()
                .map(|&k| BodyPlanRegistry::name(&reg.locomotion, k))
                .collect();
            for l in &loco {
                *locos.entry(l.to_string()).or_default() += 1;
            }
            let covering = BodyPlanRegistry::name(&reg.coverings, bp.covering.kind);
            let arms = if weapons.is_empty() {
                "unarmed".to_string()
            } else {
                weapons.join("+")
            };
            let mass = format!("{:.2}", Fixed::to_f64_lossy(bp.body_mass));
            // Diet: what it draws on.
            let mut diet = Vec::new();
            for s in &sp.draws_on {
                match s {
                    SourceRef::Abiotic(a) => diet.push(format!("abiotic#{}", a)),
                    SourceRef::Species(d) => {
                        let pl = trophic_label(&map, *d);
                        diet.push(format!("sp#{}({})", d.0, pl));
                    }
                }
            }
            let has_magic = weapons
                .iter()
                .chain(senses.iter())
                .chain(loco.iter())
                .any(|n| magic_names.contains(n))
                || magic_names.contains(&covering);
            if has_magic {
                magical += 1;
            }
            let line = format!(
                "r({rx},{ry}) sp#{:<3} {:<18} {:<7} mass {mass:>6}  arms:{arms}  cover:{covering}  senses:{}  moves:{}  eats:{}",
                id.0, label, temperament_word(bp.temperament.boldness), senses.join("/"), loco.join("/"), diet.join(",")
            );
            // Curate the interesting: carnivorous plants, walking plants, magical, flyers, apex predators.
            let rooted_only = loco.len() == 1 && loco[0] == "rooted";
            let mobile_plant = label.contains("plant") && !rooted_only;
            let flyer = loco
                .iter()
                .any(|l| *l == "fly" || *l == "glide" || *l == "levitate" || *l == "blink");
            let swimmer = loco.contains(&"swim");
            if label == "carnivorous plant"
                || mobile_plant
                || has_magic
                || flyer
                || (label == "carnivore" && swimmer)
            {
                interesting.push(line);
            }
        }
    }

    println!("\n== tallies ==\n  by kingdom/diet: {:?}\n  by locomotion:  {:?}\n  species with a magical trait: {magical}", labels, locos);
    println!(
        "\n== a sample of the interesting ({} found; first 40) ==",
        interesting.len()
    );
    for l in interesting.iter().take(40) {
        println!("  {l}");
    }
}
