# Consensus Roadmap: the status board

The lean board: where the work is, one line per item. An entry is a date, a few words, and a pointer (a branch, a PR, a file, or a doc). The detail lives behind the pointer and never on the line.

Edit IN PLACE. When an item moves, move its line; when it lands, tombstone it under recent landings with its pointer; prune landings once they go stale. Do not inline the detail, do not append a dated narrative (that is `HANDOFFS.md`), and do not touch unrelated lines. Keep the file small: the stop-gate fails above 16 KB.

The retired long-form board is `docs/working/CONSENSUS_ROADMAP_HISTORY.md`, preserved verbatim; every item below carries its full history there. Dates are when the line last moved.

## Active arcs

- 2026-07-19  Mountains / mid-band (increment 3) -> #205 HELD, the seven-field frontier is NOT closed: the ambient-frame rows were read at interior state, so the cluster now REFUSES a mixed-frame request (#208 steering). ALL 9 audit findings fixed, each as a refusal or an unconstructible pairing rather than a comment. BLOCKED: the wire cannot activate because the bundle correctly refuses at interior state; unblocking needs the state-resolved thermoelastic provider -> `docs/working/THERMOELASTIC_STATE_FRAME_DERIVE_FIRST_STEERING.md`. Next: thread the census through `build_deep_time_provinces`, move the 7 fields + their 3 uncompared copies, re-baseline via `realization_digest`, then Seams C/D. RETRACTED in HANDOFFS: my "the full SI lift is infeasible" claim was WRONG (audit-caught, I tested SI-LINEAR not the ruled SI/LOG-SPACE); the ruled plan stands, heat carries as per-tick energy or a signed log.
- 2026-07-18  Remote agent #201 (disk-evolution wire) lands large and often -> rebase-onto-main integration, keep viewer/physics lanes disjoint.
- 2026-07-18  Arc AFTER the crust (perpetual dynamics, atmosphere keystone, hydrosphere) -> plan in `PHYSICS_SUBSTRATE_ROADMAP.md`.

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

- 2026-07-19 FLAGGED provenance system: 6 CRITICAL + 8 HIGH from an adversarial audit; a ghost source passes `sources_gate.py` (live-fired), 4 claims have primary == secondary, 0 `@sources` markers -> `$CLAUDE_JOB_DIR/tmp/codex_provenance_audit.txt`, blocks R-ABIOGENESIS-2.
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

- 2026-07-19  DONE  #207 merged to main (`0de27ab`): derive-first abiogenesis research spec + closure-formalism addendum -> `docs/working/ABIOGENESIS_*.md`.
- 2026-07-19  DONE  #201 merged to main (`402e00d`): disk slice 2 run-path wire, Mdot_0 derived, the L_bol hold dissolved -> `git log 402e00d`.
- 2026-07-19  DONE  #203 merged to main (`b1b9ed0`): two crates, five live-fired gates, 225 files, pins held -> `git log b1b9ed0`.
- 2026-07-19  DONE  Local/CI command parity: the check list is PARSED from the workflow, not copied (six classes had never run locally) -> `scripts/ci_local.sh`.
- 2026-07-19  DONE  Phase-conductivity column fetched (8 phases, cell counts as reconstructions + kappa_298 x5) -> `crates/physics/data/phase_conductivity.toml`.
- 2026-07-19  DONE  Crate layering un-inverted: `calibration`+`decision` bio -> foundation; foundation drops its bio lib dep -> `crates/foundation`.
- 2026-07-19  DONE  Language cycle cut: `PlaceId` lifted to a leaf, 7-module cluster -> 3 -> `crates/foundation/src/located.rs`.
- 2026-07-19  DONE  World ejecta docs: 7 stale claims, one pointing at a route ruled out in PR #177 -> `crates/world`.
- 2026-07-19  FLAG  Last cycle (`environ`/`genesis`/`runner`) is NOT a small lift: `Field::step` calls `derive_field_diffusion` inside `runner.rs`.
- 2026-07-18  HELD  Fixture-cluster re-pin BLOCKED on one absent cited column (`atoms_per_primitive_cell`, no data file carries it); rho/c_p/alpha measured and deriving, k/kappa/eta gated; thermostat diamond collapsed byte-neutral, both pins unmoved -> `GEOTHERM_ARC_SCOPE.md`.
- 2026-07-18  DONE  Rock conductivity aggregate: Bruggeman EMT (fetched numerical benchmark rejects the geometric mean), pins held -> `crates/materials/src/conductivity.rs`.
- 2026-07-18  DONE  Build speedups: build-script opt-level 3 (Stone 0 gate 4.4s -> 1.4s) + dev line-tables-only (9.9G -> 4.5G); pins held -> `Cargo.toml`.
- 2026-07-18  DONE  Pipeline-status guard hardcoded (a habit is not a defense) -> `.claude/hooks/pipeline-status-guard.sh`.
- 2026-07-18  DONE  Stone 0 pre-push hook INSTALLED; derives gate widened past `pub` (818 -> 1099 fns).
- 2026-07-18  DONE  Provenance ratchet now fires at TURN scope (package-scoped cargo skipped it) -> `.claude/hooks/stop-gate.sh`.
- 2026-07-18  DONE  Two load-bearing stale claims retired (render.rs T_e, stone0 wiring) -> `render.rs`, `crates/stone0/src/lib.rs`.
- 2026-07-18  DONE  Derives-coverage gate + 8 physics/materials markers (map 11 -> 19 substrates; CI + stop-gate wired) -> `scripts/derives_gate.py`.
- 2026-07-18  DONE  Gruneisen loader + census aggregator (rock gamma DERIVED, uncited phase refused) -> `crates/physics/src/gruneisen.rs`.
- 2026-07-18  DONE  CI doc gate unbroken: two links cited a nonexistent `derive_deep_time_cap` -> `crates/viewer/src/main.rs`.
- 2026-07-18  DONE  Biology parked out of `civsim-sim` (10 modules, 9,388 lines, both pins bit-exact) -> `crates/bio`.
- 2026-07-18  DONE  Leaf substrates split out of `civsim-sim` (20 out-degree-zero modules, 12,677 lines, both pins bit-exact) -> `crates/foundation`.
- 2026-07-18  DONE  `learn`/`locomotion` cycle broken by lifting 2 shared types; 3 SCCs -> 2 -> `crates/foundation/src/sequence.rs`.
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
- 2026-07-18  DONE  Vendoring pipeline consolidated + gated (29-source registry, licence overlay, `@sources` hop) -> `FETCH_PIPELINE_PLAN.md`, `scripts/sources_gate.py`.
- 2026-07-18  DONE  Owner licence ruling applied: 6 restricted PDFs -> citation-plus-witness (witnesses byte-identical); JANAF blocked by `include_str!` -> plan D6.
- Secular cooling: `geodynamics::secular_step` built; `step_deep_time` calls the static `convection_step` instead, so the interior freezes and volcanism dies.
- Star brightening: `deeptime::stellar_luminosity_ratio` built; the viewer ages the clock but never reads it.
- Relief collapse: `relax_to_support_bound` built; the viewer uses a hardcoded yield-strength flag.
- Multi-body system generator: `assemble_system_with_giants` fully dark; the viewer samples independent single worlds.
- Atmosphere radiative-balance / greenhouse closure (`wien_peak`, `interface_split` have no consumer; nothing derives surface temperature from the air). Highest leverage; gates Venus, climate, hydrology.
- Hydrosphere + weather + erosion (R-HYDROSPHERE-WEATHER; needs a Chaos-Protocol climate ruling).
- Interior recycling (crust is monotonic; no subduction, no overturn).

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
