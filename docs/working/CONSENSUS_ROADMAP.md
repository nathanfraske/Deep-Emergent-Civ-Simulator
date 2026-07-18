# Consensus Roadmap: the task board

A LEAN board and nothing else: where are we, what landed, where did the work go. One short line per item: a date, a few words, and a POINTER (a commit, a file, a doc, an agent worktree). Follow the pointer for detail; never inline it here. This is not HANDOFFS (the rolling narrative) and not CLAUDE.md (the manual). When a task lands, tombstone it (mark DONE with its landing pointer); prune tombstones once they are old history. The retired ballooned version is archived at `CONSENSUS_ROADMAP_HISTORY.md`, and the arc-after-the-crust plan is `PHYSICS_SUBSTRATE_ROADMAP.md`.

Editing rule (the stop-gate enforces it): edit IN PLACE, keep every line a few words plus a pointer, and keep this file small. If a line wants to grow, that detail belongs behind the pointer, not here.

## Active arcs

- 2026-07-18  Mountains / mid-band (increment 3) -> branch `claude/topology-increment3`. Next: conditioned Ra_crit row, block-3 re-derivations, sub-step D, Seams C/D. Detail in HANDOFFS.
- 2026-07-18  Viewer cadence, deep-time watchable (open-young, pace advance, decouple flash) -> viewer agent, non-canon, viewer-side.
- 2026-07-18  GPU viewer render (112x surface-zoom, byte-neutral, feature `gpu`) -> `crates/gpu/src/globe.rs`, worktree `agent-afd7905b...`; integrate at the render layer.
- 2026-07-18  Arc AFTER the crust (perpetual dynamics, atmosphere keystone, hydrosphere) -> plan in `PHYSICS_SUBSTRATE_ROADMAP.md`.

## Recent landings (tombstoned; prune when old)

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
