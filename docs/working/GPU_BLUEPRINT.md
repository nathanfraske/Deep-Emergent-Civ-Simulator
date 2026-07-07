# GPU integration blueprint and the Rayon parallelization plan (arc 4, 2026-07-07)

This is the design record for arc 4 (parallelization and GPU offload), produced by an exhaustive
multi-agent map of the simulation's hot loops plus an adversarially-verified Rayon plan. It captures what
is already built, the determinism contract every parallel path must honour, the GPU offload targets, and
the CPU Rayon parallelization that lands now. The governing invariant is unchanged: the simulation is
DETERMINISTIC and must produce a bit-identical `state_hash` across runs AND across worker or thread widths.

## The determinism contract (the bedrock every parallel path rests on)

- Integer math only. Every canonical value is `civsim_core::Fixed` (Q32.32); `Fixed::mul` is
  `((a as i128) * (b as i128)) >> 32`, a pinned floor, and there is no float on the canonical path (a
  `canon_guard` source-scan rejects float tokens in kernels). Because each cell or being is a fixed
  straight-line integer op sequence, ANY compute order (thread, lane, tile, workgroup) yields bit-identical
  results: there is no FMA contraction and no lane-order rounding to perturb. This is what makes the
  stencils and per-being kernels safe to reorder.
- Fixed WRITE POSITION. Compute order is free, but every kernel writes its result into a pre-sized buffer
  indexed by cell (row-major) or being (`StableId` order), never in thread-completion order. The
  `state_hash` fold then walks that buffer in the unchanged canonical order (field row-major, body_temp in
  id order, walkers sorted by id), so the byte stream is a pure function of the produced set at any width.
- RNG keyed by coordinate, never by thread. Every draw goes through a `DrawKey` (region, locus, entity, or
  pair, plus tick and phase), so the stream a cell/being/locus draws is fully determined by its id and the
  tick, not by which thread ran it. `overflow-checks = true` in the release profile means any parallel
  arithmetic overflow PANICS loudly rather than wrapping, an extra guard against a silent divergence.

## What is already built (crates/gpu, CubeCL)

The GPU crate exists and is kept out of `default-members` (a plain `cargo build`/`test` stays lean and
device-free; it is built explicitly with `-p civsim-gpu`). It already carries CubeCL kernels and, for each,
a BIT-IDENTITY GATE test against the exact same computation written in `Fixed` (the CPU oracle), asserting a
zero mismatch count over the whole buffer at non-power-of-two sizes (so the clamped boundary and the linear
index both bite):
- `gpu_field_step` (field.rs) + `field_step_gate` / `diffusion_gate`: the thermal-diffusion stencil.
- `gpu_activate`, `gpu_body_thermal`, `gpu_metabolize` (being.rs) + `being_gate`: the controller matvec, the
  Newton-cooling body exchange, and the per-being metabolic drains.
- `gpu_notice` (perceive.rs) + `perceive_gate`: the perceive notice roll, one thread per (being, trace).
- worldgen (worldgen.rs) + `worldgen_gate`; `stage0` + `stage0_gate`; `transcendental` + gate;
  `cross_backend`; `canon_guard` (the float-token scan). The `prim.rs` q32 ops are the pinned primitives.

So the GPU BLUEPRINT is realised in code: the CPU `Fixed` path is the reference oracle at all times
(R-GPU-CANON-PIN), the GPU path is a consumer of that oracle never the reverse, and every kernel is gated
bit-for-bit. The staging is opt-in by construction (the crate self-skips device tests unless `CIVSIM_GPU`
is set).

## GPU offload targets (ranked), the remaining wiring

The offload targets the map surfaced, each a data-parallel kernel that fits the GPU and already has (or
mirrors) a gated CPU oracle:
1. `Field::step` thermal-diffusion stencil (the densest per-tick loop, ~1M cells at the 1024x1024 target):
   `gpu_field_step` ports it exactly, one thread per cell. The remaining wiring is a runner-level toggle
   that routes `step_field` through the GPU kernel, holding the CPU stencil as the reference.
2. `Controller::evaluate` reaction-norm / recurrent matvec (per being per tick, per genome per episode
   sub-tick): `gpu_activate`, one thread per activation.
3. `phase_body_exchange` Newton cooling and `Homeostasis::metabolize` + `Stock::step` (per being):
   `gpu_body_thermal` / `gpu_metabolize`, the resident-being-over-resident-field pattern.
4. `EnvironFields::step_productivity` Liebig-minimum biomass fold (pure pointwise per cell): a new pointwise
   `#[cube]` kernel over the pinned `q32` primitives.
5. `perceive` notice roll (already CPU-parallel): `gpu_notice`, one thread per (being, trace) pair.
6. `GenePool::drift` / `found` Wright-Fisher Bernoulli sampling (the aggregate-tier inner cost at large Ne):
   a new grid launch, one thread per (pool, locus), the 2*Ne count from a `DrawKey`-keyed RNG.

The staging: each target ships as an opt-in toggle with the CPU oracle as the reference, a CPU-vs-GPU
bit-identity gate at non-power-of-two sizes as the acceptance test, so nothing regresses. The headless test
plan is the gate template (`field_step_gate`): `cpu_kernel` in `Fixed` vs the GPU kernel over N iters,
`assert_eq!(mismatch, 0)` over the whole buffer, at a size like 37x23 chosen so the boundary and the index
both bite.

## The CPU Rayon parallelization (landing now)

Before the GPU, the CPU tick has NO data parallelism on the runner side (only `world.rs` `perceive` and
`converse` use a hand-rolled `std::thread::scope`; there is no `rayon` in the workspace). The map found six
loops that are embarrassingly parallel AND provably bit-identity-safe (each adversarially verified): every
one either collects into an id-keyed `BTreeMap` (order-free), writes disjoint indexed slots, or maps into an
indexed `Vec`, and each draws RNG only through a `DrawKey` keyed by the item's id, never the thread. These
land as `rayon` `par_iter`/`par_iter_mut`/`par_chunks_mut` with the collection or write position unchanged,
so the result is identical to the serial version at any thread count:
1. `Field::step` stencil: `next.par_chunks_mut(w)` over the double buffer (read `self.temp`, write `next`),
   disjoint indexed writes, no RNG. The highest per-tick value.
2. `step_embodiment`'s six pre-compute maps (field_dirs, field_signed, appetitive, attraction,
   discovery_choices, drains): `emb.walkers.par_iter()` collecting into the same `BTreeMap<StableId, _>`
   (order-free). The heaviest per-being work when the discovery/attraction/appetitive opt-ins are armed.
3. `phase_body_exchange`: a gather-then-apply, `ids.par_iter().filter_map(...).collect::<Vec<_>>()` then a
   serial insert, so no read-after-write during the parallel gather.
4. `evolve_with` per-genome scoring: `pop.par_iter().enumerate().map(express + scorer).collect()`, each
   score a pure function of (genome, episode_ticks, seed) on local state.
5. `GenePool::drift` (and `found`): `self.freqs.par_iter_mut().enumerate()`, per-locus independent, the 2*Ne
   Bernoulli stream keyed by `DrawKey::pair(pool, locus, generation, EVOLVE)`.
6. `WorldGenesis::step_once` regions: `self.regions.par_iter_mut().for_each(|sr| sr.radiation.step_once())`,
   disjoint region state, draws keyed by region.

The acceptance gate for each: `run_world` stays `a465919e`, the determinism harness stays green, and the
hash is invariant under `RAYON_NUM_THREADS` (1 vs many), since the parallel result equals the serial one by
construction. Because rayon is a workspace dependency the sim gains, the parallelism is on by default but
determinism-neutral; a future toggle can pin the pool size for a fully reproducible wall-clock if wanted.

## Heterogeneous cores (Intel P/E, AMD X3D): topology-agnostic by construction

A standing question: can the Rayon on-ramp classify and exploit asymmetric cores (Intel Performance and
Efficiency cores, AMD dual-CCD parts where one CCD carries the 3D V-Cache)? The answer for correctness is
that it does not need to. Rayon is topology-agnostic: it spawns one work-stealing pool of N workers and does
not pin them, so the OS scheduler (Intel Thread Director, the AMD CPPC driver) places threads and
work-stealing absorbs the speed asymmetry dynamically (a slow E-core steals fewer chunks, a fast P-core
steals more). Crucially, every parallel loop here draws RNG only through a `DrawKey` keyed by the item's id
and writes to a fixed position, so the result is bit-identical whatever core runs whatever chunk: core
placement can move the wall-clock, never the hash. The `RAYON_NUM_THREADS` 1-vs-many gate proves this on any
topology, so no P/E or X3D box is needed to build or validate the engine.

Classification is a later, optional PERFORMANCE tuning, not a correctness need. The two loop families have
different appetites: the field and material grid steps (`Field::step`, `step_matter_cycle`) are cache-bound
and would prefer the V-Cache CCD or the P-cores, while the genome and evolve fan-outs are compute-bound and
run well anywhere including E-cores. Exploiting that would mean: detect topology portably (`hwlocality`, or
read `/sys/devices/system/cpu/*/{type,cache}` plus CPUID for core kind), build two Rayon pools with
`ThreadPoolBuilder::spawn_handler` plus the `core_affinity` crate (a cache pool pinned to V-Cache or P cores,
a throughput pool spanning all cores), and route each phase with `pool.install(...)`. Pinning cannot change
results, since nothing in the sim keys off thread identity, so it is a determinism-safe change to add when
there is a real perf target and a machine to measure on. Recommendation: stay topology-agnostic until then;
work-stealing covers the asymmetry and correctness is already hardware-invariant.

## CPU and GPU interleaving: the cross-tick field-stencil pipeline

The offload map for `Field::step` establishes that the thermal-diffusion stencil is a pure function of the previous tick's temperature buffer plus two run-constant inputs, the field baseline and the diffusion and relaxation coefficients. That purity is what licenses interleaving, and the shape of the tick's dependency graph is what bounds it. The four systems serialize completely in the composed runner: `SYS_FIELD` writes the field bundle, `SYS_BODY` reads the settled field, `SYS_EMBODIMENT` reads it and resolves being mortality, `SYS_WORLD` follows, with write-write conflicts on `RES_BODY` and `RES_BEING` forbidding any intra-tick batch parallelism. There is no same-tick slack to exploit. The one real window is the previous tick's CPU tail, which carries no dependency on this tick's field.

The interleaving model is a depth-one software pipeline over the stencil, driven from the single tick-loop thread. The stencil for tick N+1 is dispatched asynchronously the instant tick N's `step_combustion` commits its `add_heat` write, spliced inside `step_field` between the combustion call and the matter-cycle call. At that point `self.field.temp` is frozen for the rest of tick N: the field's only two writers are `Field::step` and `add_heat`, and nothing after combustion writes it again until the next tick. The dispatch uploads that snapshot, launches the kernel once, and returns without reading back. Tick N's remaining CPU work then runs concurrently with the GPU stencil: the matter cycle, body exchange, embodiment, hydrology recoupling, the world tick, and lifecycle reconciliation, none of which read the device buffer or write the field. The result is consumed at one fixed point, the top of tick N+1's `step_field`, where a blocking readback replaces the synchronous `Field::step` call and copies the finished bytes into the host mirror before `env.step` reads them. If no dispatch is pending, the synchronous CPU path runs as the fallback.

Determinism holds across the async boundary because completion timing changes only how long the CPU parks at the readback, never which bytes it reads. The kernel is a pointwise fixed-point map with no atomics, no reduction, and no random draw, gated bit-identical to the CPU oracle, so device scheduling cannot perturb any cell. Its inputs are fixed: a deterministically produced temperature snapshot, a baseline set once at construction, and coefficients set once at construction. The single consume point is code-position-fixed and reached unconditionally every tick, never on a timer or a poll, and the readback is a real CUDA event wait rather than a wall-clock guess, so it returns the complete kernel output whenever the GPU finished. Host and device memory are disjoint and the host mirror is overwritten wholesale only at that one point, so no overlapped CPU read can see a torn value, and no field writer runs inside the overlap window to race. All GPU calls issue from one thread on one thread-local stream, so program order fixes device execution order statically.

The win is narrow and single-edged, and this is stated plainly rather than inflated. The pipeline hides only the stencil kernel's own latency, not `env.step`, `regrow_supply`, `step_combustion`, or `step_matter_cycle`, which have no kernel and stay on the critical path. The overlap is bounded above by the kernel's latency and below by zero, realized in full only when the previous tick's CPU tail costs more wall-clock than the stencil, the expected regime once per-being work dominates at nontrivial population. At small population the tail is cheap and the readback parks for a real fraction of the kernel time, though never longer than the synchronous path would have cost. The pipeline is depth one by the combustion-to-stencil-to-combustion recurrence and cannot be deepened; only one stencil is ever in flight.

Staging is gated on a timing-invariance test. The dispatch and readback are split in `crates/gpu` first, keeping the existing single-call launcher and its gate as the reference, then wired behind a feature flag and a Runner toggle off by default, then spliced into `step_field` with the synchronous path as the fallback. The acceptance bar re-runs the CPU-oracle bit-identity gate and the full `run_world` hash check with the toggle off and on, and under a forced-slow or forced-async GPU: an artificial delay is injected before the readback fence resolves and swept across runs, and the per-tick field hash and the final state hash must be bit-identical across every delay setting, at one Rayon thread and many. Hash invariance to injected completion latency is the operational proof that timing cannot move the hash. The intra-phase precompute hoist, which overlaps a small set of field-independent CPU work under the stencil for the small-population case, and the persistent-resident variant with an on-device heat-scatter kernel, are deferred refinements; the scatter kernel is new arithmetic and must carry its own bit-identity gate before it ships.
### Open questions the interleave design surfaced (and one fix already applied)

The design map raised four items to close before or alongside any GPU-pipeline wiring. One was a real
latent seam in the tree, now FIXED: `couple_reproductive_vigor` ran in the pinned `step_inner` before
`world.tick` but the scheduled `run_phase` (the `SYS_WORLD` branch) omitted it, so an armed vigor coupling
would tick the world with a different eligibility map on the two entry points and diverge once reproduction
fires. The scheduled path now calls it in the same place, a no-op when unarmed so `run_world` stays
`a465919e` and the unarmed pinned-versus-scheduled equivalence holds. A biting regression test needs a
breeding-capable race whose reproduce beat takes the per-being vigor draw (the vigor map is not
hashed directly, it acts only through a `MATE_CHOICE`-phase draw), so the dawn harness's non-breeding world
cannot exercise it; a lock built on the `reproduction.rs` sex-determined-race scaffolding is a flagged
follow-on. The remaining three stay open and are not blockers for the Rayon work already landed: whether
dropping an unconsumed final GPU dispatch in the epilogue is resource-clean (host-state-inert is clear,
cleanup is not independently verified); the population balance point that decides whether the cross-tick
overlap is near-full or marginal, a profiling question on real hardware rather than a number to invent; and
the single-CPU-thread-owns-a-Runner's-GPU-calls precondition, which the thread-local `StreamId` determinism
argument rests on and which a future multi-threaded GPU driver would need an explicit stream pin to preserve.
