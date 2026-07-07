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
