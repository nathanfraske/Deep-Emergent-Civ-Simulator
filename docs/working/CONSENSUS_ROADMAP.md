# Consensus Roadmap: the status board

The lean board: where the work is, one line per item. An entry is a date, a few words, and a pointer (a branch, a PR, a file, or a doc). The detail lives behind the pointer and never on the line.

Edit IN PLACE. When an item moves, move its line; when it lands, tombstone it under recent landings with its pointer; prune landings once they go stale. Do not inline the detail, do not append a dated narrative (that is `HANDOFFS.md`), and do not touch unrelated lines. Keep the file small: the stop-gate fails above 16 KB.

The retired long-form board is `docs/working/CONSENSUS_ROADMAP_HISTORY.md`, preserved verbatim; every item below carries its full history there. Dates are when the line last moved.

## Active arcs

- 2026-07-18 disk slice 2 wire + layer-4 kernels R_1/Omega_star_0/T_core, dormant -> claude/disk-slice2-wire, PR #201
- 2026-07-18 mantle effective-convective-viscosity slice -> claude/seam4-deeptime, crates/physics/src/convective_viscosity.rs
- 2026-07-18 Hayashi-wall grid, the split with Agent C (#198) -> claude/seam4-deeptime, crates/physics/data/hayashi_wall.toml
- 2026-07-17 consolidated surface pipeline, stages 0-5 -> claude/seam4-deeptime, docs/working/CONSOLIDATED_SURFACE_PIPELINE.md
- 2026-07-17 disk-evolution expansion, the stellar-wind chain (#195) -> claude/disk-evolution-arc, docs/working/DISK_EVOLUTION_EXPANSION_SCOPE.md
- 2026-07-16 deep-time run, seam 4 (#69) -> claude/seam4-deeptime, crates/sim/src/deeptime.rs
- 2026-07-16 solar-system generator, the multi-body arc (#72) -> claude/seam4-deeptime, crates/sim/src/planetary_system.rs
- 2026-07-16 gas and ice giant branch (#73) -> worktree-agent-a0d7873cd43809a0f, crates/sim/src/giants.rs
- 2026-07-16 small-body buildout, asteroids and comets (#74) -> claude/seam4-deeptime, crates/sim/src/smallbody.rs
- 2026-07-16 star-planet generative-and-visible capstone -> docs/working/CAPSTONE_PIPELINE_SCOPE.md
- 2026-07-16 genesis-forward foundational arc (#160) -> claude/genesis-forward-scoping, docs/working/GENESIS_FORWARD_PLAN_OF_ATTACK.md
- 2026-07-15 orbital-mechanics arc, the Kepler period (#78) -> claude/seam4-deeptime, crates/sim/src/astro.rs
- 2026-07-15 Stone 0 local-firing provenance gate (#79) -> claude/seam4-deeptime, crates/stone0
- 2026-07-14 kappa_R opacity assembly, the Saha closure (#191) -> claude/kappa-r-assembly, crates/physics/src/saha.rs
- 2026-07-14 kappa_R molecular grid, the AESOPUS redirect (#191) -> claude/kappa-r-assembly
- 2026-07-12 close the founder-zero survival window (#42) -> claude/survival-window-close, docs/working/PRODUCTIVITY_DERIVATION_KICKOFF.md
- 2026-07-12 genesis internal-heat and memory primitives -> claude/genesis-internal-heat-memory, crates/physics/data/geology_floor.toml
- 2026-07-12 provenance-DAG accounting -> claude/genesis-determinism-provenance, crates/sim/src/calibration.rs
- 2026-07-12 derived-output-is-live gate (#43, bridge #158) -> crates/sim/src/derive_gate.rs, docs/working/DERIVED_OUTPUT_LIVE_GATE_SCOPE.md
- 2026-07-11 surface-energy-balance arc (#139) -> laws::surface_balance_temperature
- 2026-07-10 calibration reconciliation of reserved.toml (#122) -> docs/working/RECONCILIATION_BILLBOARD.md
- 2026-07-09 Mirror and Tempest world loaders -> docs/working/MIRROR_CALIBRATION_RESEARCH.md

## Not done / gated

- 2026-07-15 QUARANTINED 56 reserved values with dead provenance, frozen by the CI gate -> docs/working/quarantine_ledger.toml
- 2026-07-12 PARTIAL the four universal constants to CODATA, G now in the floor -> crates/units/src/fundamentals.rs
- 2026-07-11 GATED real biosphere thriving, gated on the biosphere-balance calibration -> worldbuild.rs T3 owner-gate
- 2026-07-11 PARTIAL stroke-rate and limb-biomechanics substrate (#119, #123) -> claude/stroke-rate-substrate
- 2026-07-10 NOT DONE integrated living-world scenario, sequenced behind the landing arcs -> `--scenario full`
- 2026-07-10 NOT DONE geology and geography come alive, in scoping -> the genesis geology arc
- 2026-07-10 NOT DONE R-COEVOLVE, Mirror as early-Earth initial conditions
- 2026-07-10 CHECK LoD-invariant contention when coarse stepping is built (R-TEMPORAL-LOD) -> crates/sim/src/clock.rs
- 2026-07-09 NOT DONE multi-cell and sub-cell extent, gated on the coordinate and field model
- 2026-07-09 NOT DONE affordance-percept contents, Tier C must emerge through the learner (#111) -> crates/sim/src/discovery.rs
- 2026-07-09 NOT DONE affordance-percept ceilings, floor-axis coverage and the closed combinator set (#111)
- 2026-07-09 NOT DONE multi-channel perception, vision and hearing as data rows -> ChannelReachRegistry
- 2026-07-09 NOT DONE genome-derived thermal setpoint, today a fixed per-race 310 K baseline
- 2026-07-09 NOT DONE emergent pollution and climate, gated on industry and the Venus wiring
- 2026-07-09 PARTIAL pick a plant for its PARTS, gated on multi-cell extent
- 2026-07-09 PARTIAL perceiver-relative FracturePotential reference (#111) -> crates/sim/src/affordance_percept.rs
- 2026-07-09 PARTIAL affordance composition, Tier B built and Tier C deferred (#111)
- 2026-07-09 PARTIAL scenario name shaping structure, magic structure remains -> crates/world/src/structure.rs
- 2026-07-08 NOT DONE chop down a tree, the biosphere-meets-made-world seam
- 2026-07-08 NOT DONE war and raiding, `conflict_pressure` has zero run-loop consumers
- 2026-07-08 NOT DONE Venus greenhouse and radiative balance, the floor kernels have no consumers
- 2026-07-08 NOT DONE Europa volumetric z-stacked medium and a tidal-heating law
- 2026-07-08 NOT DONE Arcanum magic system, Part 34 is unbuilt pseudocode and there is no mana field
- 2026-07-08 NOT DONE temporal LoD, agent exec, GPU offload, save schema, view elaboration
- 2026-07-08 NOT DONE the made world's deep tiers, Part 60 stages 6 to 13
- 2026-07-08 Crucible RUNS as substrate, its design needs patchy-basin terrain and the war mechanism

## Recent landings (tombstoned; prune when old)

- 2026-07-18 LANDED disk-gas mean molecular weight derives per world -> crates/sim/src/astro.rs
- 2026-07-16 DONE pipeline literature-fetch values gathered -> docs/working/PIPELINE_FETCHES.md
- 2026-07-16 LANDED post-main-sequence stellar track (#77) -> crates/sim/src/stellar_evolution.rs
- 2026-07-16 LANDED composition-draw generator links 0 to 2 -> crates/materials/src/disk_composition.rs
- 2026-07-16 DONE R-UNITS-PIN Tier-2 representation, merged #130 -> docs/working/UNITS_TIER2_SLICE_PLAN.md
- 2026-07-15 FIXED massive-star T_eff on both inversion paths (#76) -> crates/sim/src/astro.rs
- 2026-07-15 PROVENANCE FIX AGSS09 abundances re-declared a cross-checker -> crates/physics/src/solar_abundances.rs
- 2026-07-15 DONE capstone seam 2, the iron dark-crust optics -> viewer and materials
- 2026-07-15 LANDED accretion primitives, hill radius and isolation mass, toward #72 -> crates/sim/src/astro.rs
- 2026-07-15 FINDING GPU i128 is a CubeCL limit rather than a CUDA one -> crates/gpu
- 2026-07-13 DONE Stage-4 disposer first cut (#186) -> claude/materials-buildout
- 2026-07-13 DONE materials-arc design specs committed at source -> docs/working/VERDICT_KERNEL_CONTRACT.md
