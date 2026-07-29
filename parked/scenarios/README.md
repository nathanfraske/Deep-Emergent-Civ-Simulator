# Scenarios: the canonical test worlds as data

Each file here is one starting scenario, the data-of-record for a test world defined in `docs/working/TEST_WORLDS.md`. A scenario is a profile over the engine's dials, not a separate engine: the same deterministic rules run in every one, and the file sets only what differs (the race set, the magical physics, the change-and-extremes dials, the deep-time depth).

The schema is provisional, consistent with the working principle that nothing here is final. It will firm up when the scenario loader that reads these into a `World` is built; until then these files are the design-of-record for what each world is.

## Format

- `[scenario]`: the identity and one-line character of the world.
- `[races]`: the seeded sentient-race posture (Part 20), as owner-given categorical choices.
- `[magic]`: the magical-physics posture (Part 34). `laws` is whether a `MagicLaws` is installed at all; the intensity fields (potency, cost, limit looseness, affinity fraction and weight) are posture tokens now and become reserved calibration ids as Part 34 is built.
- `[dials]`: for each dial that drives change and extremes, the direction this scenario pushes it. A direction is a token, not a magnitude: `real` (a plausible real-world analogue, the baseline), `high` (cranked toward volatility), `low` (damped). The magnitude behind each token stays reserved in `calibration/reserved.toml` and resolves through a scenario override once the owner sets it. No magnitude is written here, so no value is fabricated (the prime directive).

A dial named here by its reserved id (for example `genome.mutation_rates`) keys an existing built system. Where the system is not yet built (most magic intensity), the posture lives under `[magic]` and becomes a reserved id later.

## The four worlds

- `mirror.toml`: grounded baseline, no magic. The control.
- `tempest.toml`: grounded, no magic, every change-and-extremes dial cranked high.
- `arcanum.toml`: high-magic stress test, minimally grounded.
- `confluence.toml`: grounded reality with abundant magic, the headline target.
