# Determinism-engine next-phase plan (R-CMD-ORDER adoption, wake layer, temporal-LOD prerequisites)

Status: PLAN, awaiting owner direction. No implementation is authorized by this document; it records the research state and the ordered options for the next phase so the owner can choose.

Date: 2026-07-02. Branch: `claude/physics-substrate-fanout`.

Traceability. This plan synthesizes three inputs produced in the same session:
- The committed converse adoption of the `CommandBuffer` barrier (commit `81f718c`, "feat(sim): adopt the CommandBuffer barrier in the live tick; parallel converse read stage"), building on the R-CMD-ORDER mechanism (`crates/core/src/command.rs`, commit `5b9168c`) and the R-HARNESS-COVER harness (commit `14ebc64`).
- Two parallel scouting reports: the R-AGENT-EXEC wake layer, and the two named prerequisites of R-TEMPORAL-LOD.
- An adversarial review fan-out over the converse-adoption diff. Its finders completed (seven findings); its three-vote verify phase was interrupted by a worker-process restart before it recorded verdicts, so the findings below are finder-raised and, for the load-bearing one, re-derived by hand rather than workflow-verified.

Grounding documents: `docs/temporal_lod_research.md`, `docs/event_driven_execution_research.md`, `docs/deterministic_scheduler_design.md`; design Parts 3.5, 4.1, 4.3, 54, 57, 58; audit items R-CMD-ORDER, R-HARNESS-COVER, R-PROJ-REGISTER, R-AGENT-EXEC, R-TEMPORAL-LOD, R-GPU-CANON-PIN.

---

## 1. Where things stand

R-CMD-ORDER's mechanism is complete and unit-proven in isolation (`CommandKey` total order, `content_id`, `EventQueue`, `CommandBuffer` barrier). The converse (dialogue) read stage has been adopted onto that barrier and committed: the read pass runs on worker threads (the ActionStage of design Part 4.1), and the produced moves are re-ordered at the barrier by `CommandKey` before application, with a `World::set_workers` execution width. The sim determinism harness runs the dawn tick through a worker sweep at widths 1, 2, 3, and 8 and asserts the full-tick hashes are identical across widths.

The intended reading was that this closes R-CMD-ORDER's adoption and R-HARNESS-COVER's last gap, leaving only consolidation. Section 2 shows why that reading is premature.

## 2. The load-bearing finding: the worker-sweep proof is vacuous as written

The dawn fixture (`crates/sim/tests/determinism_harness.rs`, `dawn_world`) spawns and promotes minds and places them in bands, but it seeds no beliefs, no perceptual traces, and no observations, and the tick is driven with an empty stimulus batch. The converse phase emits a dialogue move only when a mind holds a committed belief (the INFORM path) or an open question (the INQUIRE path). With no evidence ever entering a mind, neither condition is reachable, so the converse phase emits zero dialogue moves for the entire run.

The consequence: the state-hash changes across the run are produced by the naming game (`converse_language`) and language drift, which are separate phases, not by the dialogue barrier the sweep parallelizes. The worker sweep therefore parallelizes an empty converse read stage and proves it identical across widths trivially. The parallel machinery executes, but the R-CMD-ORDER property it exists to stress, that a non-trivial set of dialogue commands applies in a total order independent of the producing thread, is not exercised.

Verification status: raised independently by two review finders, and re-derived here by reading the fixture and the converse preconditions. The three-vote verify pass that would have triple-checked it did not record verdicts before the restart, so before acting on this the confirmation should be reproduced (the direct check is to assert a non-zero emitted-move count in the fixture and watch it fail on today's fixture).

Implication for the ledger: consolidating R-CMD-ORDER as resolved, or declaring R-HARNESS-COVER's tick sweep complete, is not honest until the proof runs over a converse phase that emits moves. This is a correctness hole in a proof, not in the shipping mechanism; the `CommandBuffer` and `CommandKey` primitives remain sound.

## 3. The two lower findings (finder-raised, unverified)

- `CommandBuffer`'s barrier drain surfaces a duplicate `CommandKey` only as a `Some` return on `schedule`; the `into_ordered`/`apply_ordered` path sorts stably, so a colliding key degrades to producer order rather than failing loud. A determinism-contract violation should be a loud failure, not a silent degrade. Severity: low to medium.
- `content_id` truncates the 128-bit FNV-1a state hash to its low 64 bits while its doc promises a "never-reused identifier"; two distinct canonical contents can collide at 64 bits. The doc overstates the guarantee. Severity: low (documentation and, at extreme scale, a collision boundary to state plainly).

## 4. Scout synthesis: the R-AGENT-EXEC wake layer

The single biggest scoping fact: there are two independent drive systems, and they live in different places. The homeostatic reserves that drain each tick (`crates/sim/src/homeostasis.rs`) are stepped only inside `locomotion::step`, and `World` does not own a reserve map at all. The utility drives that rise each tick (`crates/sim/src/decision.rs`) are the ones inside `World::tick`. The research doc's "draining reserve whose crossing schedules a wake" is the former. So the first build decision is where reserves live: a `World`-owned homeostasis map is the prerequisite for scheduling reserve-crossing wakes on the tick.

The crossing tick is exactly solvable in integer arithmetic, but with a raw-bit ceiling divide in i128, not `Fixed::div` (which is scaled by 2^32 and truncates toward zero, firing one tick early). The idle reserve trajectory is bit-exact linear because the reserves carry a zero regeneration rate, so `amount_n = amount_0 - n * draw` in raw bits with a single one-time truncation forming `draw`. That is the clean saving.

Two hazards to design around. Affect decay is geometric relaxation toward a baseline; `Fixed::powi` does not reproduce the sequential per-tick loop bit-for-bit, so affect is the one accumulation that resists a bit-exact closed form and needs the reserved re-check-interval fallback rather than a single scheduled wake. And there is no discrete action threshold in the code today, only the death floor; an action-threshold value is a new reserved calibration, surfaced with its basis, not invented.

The substrate is ready: the `EventQueue` `drain_due(tick)` primitive is exactly the wake-queue drain the staged plan wants, and `colocated_index` is the reactive spatial subscription.

## 5. Scout synthesis: the temporal-LOD prerequisites

Ordering is fixed by the research doc: R-PROJ-REGISTER and the fixed-point sampler come first, and both are hard prerequisites rather than siblings.

R-PROJ-REGISTER. The registry type is already built (`crates/sim/src/conservation.rs`, exact i128 totals) and the exact-integer-partition-with-remainder-to-lowest-id primitive already exists (`crates/sim/src/lod.rs`, `partition_lowest_id`). The gap is that only population and wealth are registered, in a test, over the toy two-tier world. Genetics (`genome.rs` demote) and belief mass (`axiom.rs` confidence-weighted mean) both fold through a truncating divide, so they are lossy and non-conserving and cannot be registered as exact projections until reshaped to the partition form. Language and institutions have no fold-back operator built at all. There is no continuous-integration gate asserting the registered set matches the declared-present list. The minimal build: reshape each lossy fold to the `partition_lowest_id` form, co-locate a conserved-projection declaration with each subsystem's lift and restrict operators, and add the completeness gate on the pattern the `PhysicsRegistry` validate step already uses. Reserved for the owner: the merged-strength rule for the concept-snap case (Part 33), and the declared-present catalogue of which subsystems must register versus are fine-only.

The fixed-point sampler. The uniform counter-based RNG and the R-GPU-CANON-PIN transcendentals (`exp`, `ln`, `sqrt`, `pow`) are built, and an exact binomial-by-Bernoulli loop already runs inline in `genome.rs`. Missing: a reusable Poisson and binomial primitive, an O(1) large-mean path (the per-trial loop is O(N), and `exp` underflows to zero below a reduced argument around negative twenty-two, precisely the large-count regime), and a `DrawKey` extension for the channel and span coordinates the coarse leap keys on. This work is gated on the owner's R-GPU-CANON-PIN precision sign-off, because the exp and ln series degrees are reserved values and a Poisson inversion built on `exp` inherits an un-ratified precision until then.

## 6. The plan, ordered

Phase 0, repair the proof before any consolidation. Seed the harness fixture with observations so minds hold committed beliefs and open questions and dialogue moves flow; assert a non-zero emitted-move count so the sweep can never silently go vacuous again; re-prove the worker sweep bit-identical over a non-empty converse, and re-prove serial bit-identity against the pre-adoption behaviour. Fold in the two hardenings: fail loud on a duplicate `CommandKey` at the barrier, and correct the `content_id` doc to state the 64-bit boundary. Only after this is consolidating R-CMD-ORDER, or closing R-HARNESS-COVER's tick sweep, an honest step. This is small and it repairs a proof that is currently overstated.

Then one of two forward branches, both fully scoped:
- Branch A, the R-AGENT-EXEC wake layer, the scale payoff. First settle where reserves live (a `World`-owned homeostasis map), then build the raw-bit ceiling-divide crossing solver and the `EventQueue` wake schedule, with affect's geometric decay on the re-check fallback and the action-threshold reserved value surfaced.
- Branch B, the temporal-LOD prerequisites. Reshape the genetics and belief folds to exact partitions and register them plus a completeness CI gate (R-PROJ-REGISTER); the sampler waits on the R-GPU-CANON-PIN precision sign-off.

Recommendation: Phase 0 first, because it repairs a proof that overstates what is proven and it is a small, bounded change. Then R-PROJ-REGISTER (Branch B's first half) as the higher-leverage forward step, since it unblocks temporal LOD and the exact-partition discipline hardens genetics and belief regardless of when the coarse tier is built. The wake layer (Branch A) is the larger and more visible build, and it opens the "where do reserves live" question, which deserves its own decision rather than being folded into a determinism-hardening pass.

---

## Traceability index

- Reviewed commit: `81f718c` (converse adoption). Mechanism commits: `5b9168c` (`CommandBuffer`), `14ebc64` (harness).
- Research docs: `docs/temporal_lod_research.md` (R-TEMPORAL-LOD scope, build factors, prerequisites), `docs/event_driven_execution_research.md` (R-AGENT-EXEC scope), `docs/deterministic_scheduler_design.md` (the hold-off on Rayon).
- Code sites named: `crates/sim/tests/determinism_harness.rs` (`dawn_world`), `crates/sim/src/world.rs` (converse, `set_workers`), `crates/core/src/command.rs` (`CommandBuffer`, `content_id`), `crates/sim/src/homeostasis.rs`, `crates/sim/src/locomotion.rs`, `crates/sim/src/decision.rs`, `crates/sim/src/conservation.rs`, `crates/sim/src/lod.rs` (`partition_lowest_id`), `crates/sim/src/genome.rs`, `crates/sim/src/axiom.rs`, `crates/core/src/fixed.rs` (transcendentals), `crates/core/src/rng.rs`, `crates/core/src/keys.rs` (`DrawKey`).
- Open backlog items in scope: R-CMD-ORDER, R-HARNESS-COVER, R-PROJ-REGISTER, R-AGENT-EXEC, R-TEMPORAL-LOD, R-GPU-CANON-PIN.
