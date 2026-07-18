# Emergent Civilization Simulator

A custom Rust engine for a deterministic, emergent fantasy civilization simulator, a hybrid in the spirit of Dwarf Fortress and Songs of Syx. Simulation comes first; the visible game is a thin glyph view onto a deep world. The world is generated, every individual is modelled, and everything of consequence emerges from rules rather than being authored: language, technology, money, governance, religion, cities, artifacts, and beliefs.

This repository holds the design, the audit that keeps it honest, the research behind its resolved questions, the operating material for continuing the work, and the engine code as it is stood up in staged order.

Copyright 2026 Nathan M. Fraske. Licensed under the Apache License, Version 2.0; see `LICENSE` and `NOTICE`.

## Where the documents are

- `docs/design.md`: the specification, 64 gapless parts (Part 0 through Part 63). Part 62 holds the research records, Part 63 the bibliography. This is the source of truth.
- `docs/audit.md`: the companion ledger. The consolidation history, the parts carrying open research flags, the research backlog, the inconsistencies, and the running resolved-and-open counts.
- `docs/research/`: the standalone research papers behind the resolved items, archived verbatim. They predate the prose customs and keep their original form (em dashes and all); they are never rewritten.
- `CLAUDE.md`, `AGENTIC_ADDENDUM.md`, `RUNBOOK.md`: the operating manual, the agentic infrastructure, and the standup runbook.
- `HANDOFFS.md`, `TODOS.md`: the rolling session log and the live backlog mirror.

## What is built now

The standup follows the runbook: the determinism core is the foundation and carries no reserved numbers, so it is built and tested in full. The simulation crate carries the calibration-manifest plumbing and the substrate-loader scaffold; behaviour that depends on a reserved value is gated until the owner sets the number.

- `crates/core`: the determinism bedrock. The `Fixed` (Q32.32) newtype with its arithmetic, the SplitMix64 counter-based RNG keyed on `(seed, entity, phase, counter)`, `StableId` and the registry, arena and slab allocators, the cache-line wrapper, the append-only event log with never-reused identifiers, the typed canonical-state boundary, and the deterministic state hash. No external dependencies, so the bedrock is maximally reproducible.
- `crates/sim`: the calibration manifest loader (every reserved value loads as a fail-loud sentinel), the development and calibrated build profiles, the conserved-projection registry, the minimal two-tier LOD model, the data-driven substrate loader, and the conservation-with-referential-integrity harness. The determinism reproducibility harness lives with the core it exercises (`crates/core/tests`).

What is held for the owner's calls: every reserved calibration value (surfaced with its basis in `calibration/reserved.toml`, never invented), and the open research items in the backlog (`docs/audit.md` carries the running count).

## Building and testing

```
cargo test --workspace
```

This runs the unit and property tests of the core, the determinism reproducibility harness (the same seed at one, four, and the machine's worker count must yield a bit-identical state hash), and the conservation and referential-integrity harness (promotion, demotion, merge, and split must conserve every declared projection and leave no dangling reference). Continuous integration runs the same tests plus `scripts/verify.sh`, which checks the prose customs and the document invariants.

```
scripts/verify.sh          # human-readable pass or fail summary
scripts/verify.sh --json   # structured output for the projectops server and the panels
```

## The reserved-value discipline

The engine is fully data-driven and openly incomplete by design. A mechanism is fixed Rust; the numbers it needs are the owner's, surfaced with their basis in `calibration/reserved.toml` and set deliberately, never guessed. A reserved value loads as a sentinel that fails loudly if read unset, so no system runs on a fabricated default. See `RUNBOOK.md` section 4.
