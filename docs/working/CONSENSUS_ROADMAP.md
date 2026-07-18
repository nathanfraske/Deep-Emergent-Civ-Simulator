# Consensus Roadmap: the task board

A LEAN board and nothing else: where are we, what landed, where did the work go. One short line per item: a date, a few words, and a POINTER (a commit, a file, a doc, an agent worktree). Follow the pointer for detail; never inline it here. This is not HANDOFFS (the rolling narrative) and not CLAUDE.md (the manual). When a task lands, tombstone it (mark DONE with its landing pointer); prune tombstones once they are old history. The retired ballooned version is archived at `CONSENSUS_ROADMAP_HISTORY.md`, and the arc-after-the-crust plan is `PHYSICS_SUBSTRATE_ROADMAP.md`.

Editing rule (the stop-gate enforces it): edit IN PLACE, keep every line a few words plus a pointer, and keep this file small. If a line wants to grow, that detail belongs behind the pointer, not here.

## Active arcs

- 2026-07-18  Mountains / mid-band (increment 3) -> branch `claude/topology-increment3`. Next: block-3 re-derivations, sub-step D, Seams C/D. Detail in HANDOFFS.
- 2026-07-18  Remote agent #201 (disk-evolution wire) lands large and often -> rebase-onto-main integration, keep viewer/physics lanes disjoint.
- 2026-07-18  Arc AFTER the crust (perpetual dynamics, atmosphere keystone, hydrosphere) -> plan in `PHYSICS_SUBSTRATE_ROADMAP.md`.

## Recent landings (tombstoned; prune when old)

- 2026-07-18  DONE  Build speedups: build-script opt-level 3 (Stone 0 gate 4.4s -> 1.4s) + dev line-tables-only (9.9G -> 4.5G); pins held -> `Cargo.toml`.
- 2026-07-18  DONE  CI doc gate unbroken: two links cited a nonexistent `derive_deep_time_cap` -> `crates/viewer/src/main.rs`.
- 2026-07-18  DONE  Biology parked out of `civsim-sim` (10 modules, 9,388 lines, both pins bit-exact) -> `crates/bio`.
- 2026-07-18  FLAG  `calibration` sits in `civsim-bio` but 23 sim modules read it; wants its own shared crate -> `crates/bio/src/lib.rs`.
- 2026-07-18  DONE  TAFI verdict: source correct, our transcription wrong; primary vendored + archived -> `VENDORING_CHECKLIST.md` (`flexure_tafi`).
- 2026-07-18  DONE  Retired boundary-layer formula purged from 9 restatement sites (the diamond) -> `laws.rs`, `moment_equivalence.rs`, `geodynamics.rs`.
- 2026-07-18  DONE  Stale-claim sweep, 32 verified findings (flexure Green's-function doc corrected to the cited form) -> `flexure.rs`.
- 2026-07-18  DONE  Physics realization digest (first determinism instrumentation on the physics path) -> `deeptime.rs::realization_digest`.
- 2026-07-18  DONE  Dawn harness parked, quarantine build-enforced (`worldbuild.rs` -> `dawn_harness.rs`) -> `crates/sim/tests/dawn_harness_quarantine.rs`.
- 2026-07-18  DONE  Lean board landed on main (archive byte-identical, 16KB cap) -> PR #202, `9ae14a4b`.
- 2026-07-18  DONE  GPU globe shading on the 5090 (CubeCL f32 kernel, non-canon, feature `gpu`) -> `crates/gpu/src/globe.rs`.
- 2026-07-18  DONE  Viewer cadence watchable (opens young, derived 1 tick/frame, held impact bloom, pole-smooth glow) -> `d8add16`.
- 2026-07-18  DONE  Cargo artifact ring buffer (LRU under size+count caps, auto at SessionStart) -> `scripts/target_gc.sh`.
- 2026-07-18  DONE  WSL copyback recovery (branch ref repaired to `b2ebbb4`, cargo PATH restored to `.bashrc`, 180G reclaimed).
- 2026-07-18  DONE  Conditioned Ra_crit row (reads the registry, rigid-rigid DEFAULTS-TAKEN, dispatch stubbed) -> `crates/physics/src/rayleigh_critical.rs`.
- 2026-07-18  DONE  Critical-Rayleigh eigenvalue registry (boundary_class x heating_mode) -> `30fdd86`.
- 2026-07-18  DONE  Rigid-rigid eigenvalue diamond fixed (1708 -> 1707.762, one cited row + two sentinels) -> `9036c5e`.
- 2026-07-18  DONE  Block-2 receipt: all-brittle-thin-lid unreachable, measured -> `e0941f9`.
- 2026-07-18  DONE  T_e hindcast retired from the floor (duplicated the wired referee) -> `9bab42a`.
- 2026-07-18  DONE  Sub-step B: physical boundary layer `d(Ra_crit/Ra)^(1/3)` -> `0e8e4f6`.
- 2026-07-18  DONE  Grueneisen ladder -> `ee89ce1`; Earth/Mars/Venus T_e referee -> `1ea64e5`; watchable-impacts flash -> `5f93509`.
- 2026-07-18  DONE  Physics-substrate audit (3-lane map, dormant inventory) -> `PHYSICS_SUBSTRATE_ROADMAP.md`.

## Top dormant-but-built (one wire from alive; detail in PHYSICS_SUBSTRATE_ROADMAP)

- Secular cooling: `geodynamics::secular_step` built; `step_deep_time` calls the static `convection_step` instead, so the interior freezes and volcanism dies.
- Star brightening: `deeptime::stellar_luminosity_ratio` built; the viewer ages the clock but never reads it.
- Relief collapse: `relax_to_support_bound` built; the viewer uses a hardcoded yield-strength flag.
- Multi-body system generator: `assemble_system_with_giants` fully dark; the viewer samples independent single worlds.

## Biggest missing vectors (detail in PHYSICS_SUBSTRATE_ROADMAP)

- Atmosphere radiative-balance / greenhouse closure (`wien_peak`, `interface_split` have no consumer; nothing derives surface temperature from the air). Highest leverage; gates Venus, climate, hydrology.
- Hydrosphere + weather + erosion (R-HYDROSPHERE-WEATHER; needs a Chaos-Protocol climate ruling).
- Interior recycling (crust is monotonic; no subduction, no overturn).
