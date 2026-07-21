# Deep Emergent Civilization Simulator

The canonical product in this repository is now the deterministic abiotic
planet and stellar-system pipeline. Its contract derives a complete system
through seven causal stages and hands an immutable snapshot to observer-only
viewers; the present implementation truthfully refuses in Stage 1.
Biology, civilization, dawn, compose, scenarios, calibration, and the old
causal viewer were built under a retired methodology and are preserved under
`parked/`.

Copyright 2026 Nathan M. Fraske. Licensed under the Apache License, Version
2.0; see `LICENSE` and `NOTICE`.

## Canonical architecture

```text
sealed floor + ledger + units
              |
      canonical planet stages
              |
       immutable snapshot
              |
            viewer

active candidate substrate: core, physics, materials, world, and private
planet modules; reachable by canonical stages only through typed adapters

parked compatibility: biology, civilization, authored worldgen, causal
presentation, scenarios, and the retired runner
```

`civsim-planet` owns the seven stages:

1. star, disk, and system;
2. assembly and composition;
3. orbital, secular, and moon evolution;
4. young thermal and material state;
5. geodynamics and deep time;
6. loads and flexure;
7. immutable snapshot.

The sealed absolute physics floor is the only value-bearing input. The four
ledger tiers and seven provenance marks, `[D]`, `[M]`, `[E]`, `[C]`,
`[A]`, `[W]`, and `[X]`, are accounting only. A citation or tag cannot
admit a value. Derive first. Every irreducible non-derived floor leaf, at any
tier, needs the complete derivation-exhaustion, Buckingham-Pi, Gap Law with
typed Chaos Protocol, and Residual Law receipt. Otherwise the stage refuses. Written state and
contingency are generated inside the run.

The current audited physical floor contains three Universal `[M]` invariants:
`alpha`, `G`, and `m_e`. Seven exact SI definitions live in a separate,
versioned representation receipt and carry no provenance mark. `eps_0` is a
runtime `[D]` value derived from `alpha`, `e`, `h`, and `c`; `sigma`, `R`, and
the atomic-volume conversion are representation-derived execution values. The
runner enters Stage 1 and refuses on the missing stellar-birth realization
measure. It never promotes that refusal to a snapshot.

## Developer entry points

Linux and WSL use Just:

```sh
just
just doctor
just run-derived
just readiness
just ledger-inventory-check
just test
just check-pr
```

Windows uses the WSL bridge:

```powershell
pwsh -NoProfile -File scripts/dev.ps1 doctor
pwsh -NoProfile -File scripts/dev.ps1 run-derived
pwsh -NoProfile -File scripts/dev.ps1 readiness
pwsh -NoProfile -File scripts/dev.ps1 test
```

`run-derived` is a compatibility name for the former derived globe workflow.
It enters the same sealed-floor library runner as `run` and never enters a
viewer. Legacy commands carry an explicit `legacy` or `parked` suffix and use
`parked/Cargo.toml`. Their results do not supply planetary readiness.

## Current records

- `HANDOFFS.md`: current state first, followed by preserved session history.
- `TODOS.md`: the lean canonical planetary queue.
- `docs/working/CONSENSUS_ROADMAP.md`: the live status board.
- `docs/working/REPOSITORY_CLEANUP_PLAN.md`: dependency-ordered cleanup plan.
- `docs/working/CANONICAL_LEDGER_INVENTORY.txt`: generated four-by-seven accounting view.
- `docs/working/ABIOTIC_EVIDENCE_DEBT.md`: unadmitted evidence and derivation obligations.
- `docs/working/PHYSICS_FLOOR_REGISTRY.md`: discovery map for floor and derivation audits.
- `docs/working/VENDORING_CHECKLIST.md`: research custody protocol.
- `parked/docs/`, `parked/audits/`, and `parked/TODOS_LEGACY.md`: preserved civilization-era records.

The strict Diamond gate is green: `Fixed::log_sum_exp` is the sole provider,
and compatibility functions delegate to it. The current physical blocker is
the named Stage 1 realization-measure refusal.
