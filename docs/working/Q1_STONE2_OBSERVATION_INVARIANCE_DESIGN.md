# Q1 Stone 2: the observation-schedule invariance test, grounded design opener

Design-first opener for Q1 Stone 2, the observation-schedule invariance test, the go the gate ruled after Stone 1. Its purpose is to convert Principle 10 (the world's canonical state is independent of when and how it is observed) from a claim into a standing test, and to stand that test up while it is green, before any level-of-detail tower, so the tower is built against a live guard rather than one added after a latent dependency has crept in. No test lands until the gate gates this design.

## What the plan says Stone 2 is

Ship-order item (2): "the OBSERVATION-SCHEDULE INVARIANCE test: run the same seed and inputs twice under two observation/render schedules and assert the world-trajectory hash is bit-identical (extend the existing pin harness to a second schedule that observes/disaggregates). It passes trivially now, and standing it up before the LoD tower means the tower is built against a live guard." The plan also flags a determinism-gate extension forbidding a read-crate import in the determinism crates, and the relabel note budgets for the stone lighting up a latent bug (a fix that is not byte-neutral is the stone working).

## Grounding: what the repo already gives us

- The determinism harness (`crates/sim/tests/determinism_harness.rs`) already proves two neighbouring invariances: INPUT-DELIVERY-schedule invariance (`the_unified_runner_step_matches_step_scheduled`, `the_runner_tick_runs_through_the_scheduler_bit_identically`: the same inputs delivered eagerly versus through the scheduler produce a bit-identical `state_hash`), and RELOAD invariance (a snapshot round-trip reproduces the live `state_hash`). It hashes the trajectory as a sequence of `(state_hash, event_log_hash)` per tick, driven by a `seed_observations` input schedule.
- The OBSERVATION surface, the observer-called read paths, is present and read-only: `lod::to_snapshot() -> WorldSnapshot` (the coarsened observation the plan names), `conservation::snapshot(world)` (reads the conserved-quantity ledger), `genesis::snapshot() -> LivingWorld` (the living-world projection), and the viewer render (`crates/viewer/src`, which depends on `sim` one-way and holds no hashable state). `state_hash` itself is a pure read.
- The crate boundary is structural: `sim` does not depend on `viewer`, so a render cannot feed back into canonical state by construction. That is the shape Stone 2 makes a standing test rather than a convention.

A grounding correction I made before proposing this (Prime Directive 1): `homeostasis::snapshot(&mut self)` is NOT an observation despite its name; it is a canonical tick operation (called once per body-tick to carry the reserve levels to the next tick's interoceptive delta), so it is out of scope. The observer-called reads above are the ones the test schedules, and each is `&self` read-only.

## The design

- **The invariant.** The world-trajectory hash, the per-tick sequence of `state_hash`, is bit-identical regardless of the OBSERVATION SCHEDULE applied to the run. An observation (a snapshot, a projection, a render, a hash read) is a pure read of the canonical state and must not perturb it, so interleaving observations at any cadence, or none, leaves the trajectory unchanged.
- **The test.** Run several worlds from the same seed and the same `seed_observations` input schedule, each under a different OBSERVATION schedule, and assert every trajectory-hash sequence is identical:
  - schedule 0 (baseline): tick only, record the trajectory, observe nothing;
  - schedule 1: observe every tick (call `to_snapshot`, the conservation snapshot, the living-world projection, and a render-to-buffer, discarding each);
  - schedule 2: observe every third tick;
  - schedule 3: observe on a scrambled tick set (the same thread-scrambled style the harness already uses for inputs).
  All four trajectory hashes must match tick for tick.
- **The red test (non-vacuous guard).** A mock observation that intentionally perturbs canonical state (a test-only observer that writes through where a real observer only reads) must make the invariance FAIL, so the test proves it is live rather than trivially green.
- **The optional gate line.** Extend the determinism gate (Stone 1 item 0) with a ratchet forbidding a read-or-render-crate import (`civsim_viewer`) inside the determinism crates. The crate boundary already enforces this structurally, so the gate is belt-and-suspenders; it is worth adding now so an accidental future `viewer` import in `sim` fails at merge.

## Honest status and the light-up budget

The test PASSES TRIVIALLY today, because every observation op is read-only, so no cadence of reads can change the trajectory. Its value is the STANDING GUARD: when the level-of-detail tower is built (Q2), a coarsen-then-reconstruct that is not exact, or a cached observation that feeds back, would change the trajectory under a different observation schedule and this test would go red at exactly the change under review. Standing it up now, before the tower, is what makes that red attributable. If it lights up a latent dependency on the CURRENT tree (an observation op that is not as read-only as it looks), that is the stone working, and its fix is a deliberate reviewed byte change, budgeted per the plan.

## As built (the corrected per-surface form, gate-approved)

A grounding correction (Prime Directive 1) reshaped the design at build: the four observer reads do NOT compose on one world. `TwoTierWorld::to_snapshot`, the generic `ConservationRegistry::snapshot`, and `WorldGenesis::snapshot` are distinct subsystems, and the harness `Runner`/`World` composes none of them, so there is no single bed a cadence loop can observe through all four. The gate approved the corrected coverage: a full cadence-invariance test where a canonical tick exists, and a read-only assertion where it does not, because cadence-invariance cannot be tested on a subsystem that has no cadence yet. The built form (`crates/sim/tests/observation_invariance.rs`):

- The RICH cadence test on the `WorldGenesis` ecological driver (`step_once` ticks, `snapshot` yields the `LivingWorld` the viewer renders, `LivingWorld::state_hash` hashes it): the per-step trajectory is bit-identical under the observation schedules (none, every step, every third, every eighth), each observing through `snapshot` and a render-equivalent `LivingWorld` read, so the render path is covered with no `viewer` dependency. The recorded object is the per-step trajectory, not the final state, because the radiation runs to a fixed endpoint (a final-state hash would be invariant even to a perturbation, a vacuous target).
- The read-only assertions on the two static surfaces (`to_snapshot`, `conservation`): each proves the observation leaves its source `state_hash` unchanged and is idempotent, catching an interior-mutability side-effect.
- The red control: a state-perturbing observation shifts the trajectory, so the guard is non-vacuous, plus a discrimination control (distinct seeds hash differently).
- The `viewer`-import ratchet: `civsim_viewer` is a forbidden pattern in the determinism grep gate, so a read-or-render-crate import in the determinism crates fails at merge.

**What is guarded now versus at the LoD arc (the deferred cadence coverage).** Now: the genesis driver's full cadence-invariance, and the read-only-ness of `to_snapshot` and `conservation`. Deferred: the CADENCE-invariance of `to_snapshot` (and any future `conservation` tick) rides the level-of-detail arc, because those subsystems gain a canonical tick only when the tower is built. That is exactly where a non-exact coarsen-then-reconstruct goes red, so this test is the standing guard the tower is built against, and the cadence loop extends to the LoD read the moment it ticks.

## Discipline

Design-first: no test until the gate gates it. Byte-neutral (a new test plus an optional gate line, no canonical source change), so all five pins hold trivially unless the stone lights up a real dependency, in which case the fix is a reviewed byte change and a success. Section-9 once by me. This is the observer-independence guarantee the deep-time level-of-detail rests on, stood up while it is green.
