# GPU-vs-CPU compute split: the offload map (scope)

A design scope for owner sign-off, produced by a subsystem survey (five agents classifying every
engine compute workload against a consistent GPU-fit framework, grounded in the code) and a synthesis.
It answers what can and should be offloaded to the GPU versus kept on the parallelised CPU. The
standing constraint is bit-identity: every device path reproduces the `crates/core` `Fixed` oracle
bit-for-bit (R-GPU-CANON-PIN), and the counter RNG is draw-keyed (R-RNG-COORD), so the RNG is never the
discriminator.


Note on the input first: the inventory carries several near-duplicate entries from separate subsystem surveys (the temperature field appears four times, perceive and converse twice each, aggregate mortality twice). I consolidated each to one workload. One classification is self-correcting and worth flagging: gossip is introduced as a perceive-style hybrid, but its own analysis demotes it to CPU-parallel because its gather is pointer-chasing (only the lone addressee roll is flat). I keep it CPU-parallel and name that seam. Everything below reproduces the CPU oracle bit-for-bit as the standing constraint, so RNG is never the discriminator: the counter-RNG (`rng.rs:94`, `keys.rs:181`) is draw-keyed with no shared stream, which is what makes the mortality, drift, and perceive rolls device-viable in the first place.

## 1. Decision framework (the rule of thumb)

Five properties decide the verdict. Read them in order and the class falls out.

1. **Domain size and independence.** A large, independent domain (grid W*H, or population N in the thousands-to-millions) is a prerequisite for the device. Low-N events (a strike, a beam, a circuit) and one-shot control planes never clear launch overhead.
2. **Flattenability.** Already-SoA grids and dense per-item columns go to the device. Ragged `BTreeMap`-of-`Vec` graphs (belief maps, lexicons, genomes, food webs) do not, and the per-tick extract-and-scatter usually costs more than the work it feeds.
3. **Control-flow uniformity.** Uniform per-element arithmetic vectorises; branchy dispatch (enum matches, early-return layer walks, `Option` skips) diverges under SIMT.
4. **Arithmetic intensity vs transfer.** Compute-bound (noise, controller, mortality rolls, drift) and bandwidth-bound-at-scale stencils (diffusion, eikonal, advection) beat the bus; trivial per-item arithmetic loses to launch+transfer unless fused into a resident pass.
5. **Order-dependence of the reduction/apply.** The reduction must be order-independent to be canonical on a device or safe to merge across workers: an associative i128 sum, a monotone-min-to-fixed-point, an integer count. Anything serial-by-contract (FNV fold, command apply, naming-game consensus, cross-tick finite differences) cannot be reordered without changing the result.

The verdict:

- **GPU** = large independent domain, flattens already, uniform, and either compute-bound or bandwidth-dominated at scale, with an order-independent reduction if any.
- **HYBRID** = the workload splits into a draw-keyed / arithmetic GATHER that flattens (device) and a branchy, ragged, order-sensitive APPLY over graph-shaped state (host). This is the perceive shape. The gather earns the device only riding data already resident, at density.
- **CPU-PARALLEL** = independent across items (beings, regions, lineages) so it fans across workers with a canonical merge, but per-item work is branchy/ragged or too trivial to launch. Order-independent, so the merge is safe.
- **CPU-SERIAL** = order-dependent by contract, or a graph traversal, or low-N control plane. No safe reordering exists, so neither device nor worker split applies.

Two facts cut across all four and gate every GPU verdict: **residency** (a field or a light per-being kernel wins only if its buffer stays on the device across ticks; per-tick PCIe round-trips swamp the compute; design Part 5.6) and **scale** (the dev map is 48x24 at `worldgen.rs:200` and dev populations are tiny, where the CPU is correct; the GPU win is a large-world argument).

## 2. What goes where

### GPU (canonical device offload)
- **Temperature diffusion + relaxation field step** (`runner.rs:232`, `field.rs:89`): bandwidth-bound grid stencil on an already-resident SoA; the diffusion half is proven, the clamped-Neumann boundary + relaxation term is the remaining kernel.
- **Worldgen fractal value noise** (`noise.rs:80`): heaviest per-cell arithmetic in the set, coordinate-keyed so input transfer is zero, one-shot.
- **Biome classification** (`terrain.rs:112`): fuse into the noise kernel; a handful of integer band comparisons on values already in registers.
- **Evolved controller forward pass** (`controller.rs:381`): batched dense fixed-point matvec, one weight vector per being, uniform, clamp activation.
- **Homeostasis reserve updates** (`homeostasis.rs:312`): uniform per-axis saturating arithmetic, free work fused into the being-kernel.
- **Body-thermal exchange** (`runner.rs:593`): per-being gather-then-mul over the resident field, fused.
- **Pool-tier aggregate mortality** (`demography.rs:193`): population-scale draw-keyed count reduction; trivial transfer, order-independent count.
- **Wright-Fisher drift + selection** (`genome.rs:611`): the same count-reduction kernel, pre-dawn epoch; arithmetic dwarfs the small state.
- **Counter RNG / splitmix64** (`rng.rs:94`): the pinned building block, always fused into a consumer kernel, never launched alone.
- **Eikonal flow-field solve** (`design.md:1250`, todo!()): monotone-min-to-fixed-point is order-independent, so a parallel sweep converges to the unique result and is canonical.
- **Medium / moisture / wind advection** (`medium.rs:123`, not built): the same low-intensity stencil class as temperature once a transport step exists.
- **Physics scalar / flux / radiant fields once fielded** (`laws.rs:648`, `1143`, `1316`, and siblings, no grid driver yet): per-cell pinned mul/div; they join the resident field pass when a per-cell driver is built.

### HYBRID (device gather + host apply)
- **Perceive notice roll** (`world.rs:2226`, gather `2306`): dense draw-keyed roll on the device (built, bit-identical), ragged belief apply on the host.
- **Field thermal gradient sampling** (`runner.rs:214`): integer central-difference rides the resident field; unit-normalise + evolved controller on the host.
- **Individual-tier mortality roll** (`world.rs:1234`): per-being death mask on the device (only if being state is resident); `remove_being` structural prune on the host.
- **Locomotion per-being step** (`locomotion.rs:385`): the controller+metabolism+gradient core on the device (the GPU workloads above); affordance dispatch and memory-set update on the host.
- **Respiration membrane flux** (`laws.rs:1058`): three-scalar flux gather when population is resident; homeostasis apply on the host.
- **Optics attenuation gather** (`laws.rs:1364`): pairwise inverse-square falloff (the perception reach), device gather; notice/apply on the host.
- **Poiseuille conductance** (`laws.rs:1087`): per-channel conductance gather on the device; the node flow-conservation solve on the host.
- **Reacting-field chemistry** (`laws.rs:759`): grid/pair margin gather on the device; the `Limiter`-enum consequence apply on the host.
- **Archard wear** (`laws.rs:600`): batched gather at scale, but bit-identity needs a NEW wide-limb primitive first (see limits).

### CPU-PARALLEL (worker fan-out, canonical merge)
- **decide utility-AI argmax** (`world.rs:2189`): no RNG and uniform, but trivial per-agent arithmetic over nested-`BTreeMap` drives; parallelise now, latent GPU only at large N with a resident drive matrix.
- **converse dialogue turns** (`world.rs:1620`): branchy speech-act reasoning over per-mind belief/ToM graphs.
- **gossip** (`world.rs:2087`): the gather is pointer-chasing (only the addressee roll is flat), so it misses the perceive-style device bar. This is the seam vs perceive.
- **language drift** (`world.rs:1555`): per-lineage uniform rewrite, but rare cadence and ragged lexicons keep transfer above the win.
- **Medium respiration, organism side** (`medium.rs:77`): cheap grid gather feeding branchy variable-length organ sums.
- **LOD quadtree biome reduction** (`lod.rs:98`): mode/argmax over a category set with lowest-id tiebreak, a poor SIMT fit, one-shot.
- **Edibility / nutrition / harm** (`laws.rs:100`): Hill dose-response with data-dependent exponent and ragged nutrient/toxin classes diverge.
- **Conserved-projection census sums** (`lod.rs:116`): associative i128 reduction, but transfer-bound at the individual-tier size; a GPU win only as a fused tail on already-resident columns.

### CPU-SERIAL (order-dependent, graph, or control plane)
- **belief update Mind::consider** (`agent.rs:166`): ragged, dynamically grown `BTreeMap`-of-`Vec` mutation with tiny per-call work.
- **theory-of-mind Mind::model** (`agent.rs:200`): same ragged shape plus the anti-projection admit gate.
- **naming game converse_language** (`world.rs:1485`): sequential consensus; the within-tick read-write chain is the mechanism.
- **reproduction + mate-choice** (`mate_choice.rs:122`): small candidate sets, argmax/truncation-sort and Dobzhansky-Muller table lookups.
- **biosphere stocks-and-flows** (`stocks.rs:129`): trophic flows couple stocks along a food-web graph, order-dependent.
- **combat / wound mechanics** (`laws.rs:213`): branchy mutable body-graph apply over sparse events.
- **structural members / machines** (`laws.rs:405`): one-shot per-artifact scalar evaluations, low parallelism.
- **Coulomb / Lorentz EM** (`laws.rs:1476`): modelled at device scale, no charged-particle population today.
- **circuit / device EM solve** (`laws.rs:1518`): topology-graph network solve with a cross-tick induction dependency.
- **age-distribution census** (`demography.rs:143`): a sum over a few hundred distinct ages, no launch pays.
- **state-hash canonical walk** (`hash.rs:44`): non-associative FNV recurrence; the StableId-sorted walk order IS the determinism contract.
- **CommandBuffer ActionApply barrier** (`command.rs:231`): single-threaded serializer by contract, the source of worker-count independence.
- **deterministic scheduler** (`schedule.rs:98`): control plane deriving the batch schedule once, not a data workload.

## 3. Prioritised offload roadmap (value x feasibility)

**Already landed.** Diffusion field kernel, bit-identical on CUDA, tile-invariant, ~22.6x on 1024x1024x200 (record 62.23). Perceive notice-roll spike, bit-identical on the 5090 this session.

Then, in build order:

1. **Finish the temperature field kernel** (small; high value). Extend `diffuse_kernel` from toroidal-diffusion-only to clamped-Neumann + the `relaxation*(baseline-cur)` term with a resident baseline buffer, so it matches `Field::step` rather than `diffusion_bench`, under its own Stage-0 gate. One added pinned `q32_mul` and a branchless boundary clamp. SEAM (residency): the runner samples the field on the CPU every tick (`phase_body_exchange` at `runner.rs:596`, `gradient_at` per being), so this amortises only once the sampling co-locates on the device (step 3). Until then it is a correctness-complete kernel waiting on its consumers.

2. **Worldgen noise + fused biome classify** (medium; high value, no residency problem). Coordinate-keyed, zero input transfer, one output readback, heaviest per-cell arithmetic in the set. Build the splitmix64 `for_coords` fold plus octave lerp with pinned muls and the final divide, fuse the ordered-band biome scan into the same pass. The cleanest standalone new win because the one-shot shape escapes the per-tick transfer fight.

3. **The fused resident being-kernel** (medium; highest architectural value). One device-resident being buffer carrying weights, drives, reserves, `body_temp`, and cell-index, running the controller forward pass + homeostasis drain + body-thermal exchange + the integer thermal-gradient gather in a single pass, co-resident with the field from step 1 so the field never reads back. This turns four too-light-to-launch workloads into one amortised pass and is what unlocks the field offload. SEAM (SoA-extraction + residency, Part 5.6): `drive_levels`, `reserves` (`BTreeMap<AxisId,Stock>`), `body_temp` (`BTreeMap<StableId,Fixed>`), and the coord-to-cell lookup extract to columns once and stay resident across ticks. This fusion is the core of the whole split.

4. **Pool-tier aggregate mortality** (medium; high throughput value). Push `(age, n, chance)` triples (chance precomputed host-side with the pinned `Curve::eval` to keep the kernel pure integer roll+compare), launch over `(bucket, member)`, reduce deaths per bucket, pull one u64 per bucket. The bulk population lives in pools, so this is the largest parallel domain in the set. SEAM (SoA-extraction, light): the sparse `BTreeMap<age,count>` extracts to a few hundred triples cheaply.

5. **Wright-Fisher drift + selection** (medium). Reuse the step-4 count-reduction kernel for the per-locus Bernoulli resample, plus the pinned `q32_mul`/`q32_div` rational selection update; leave the epoch orchestration (extinction culling, speciation, incompatibility rolls) on the CPU driver. It rides step 4's gate.

6. **Individual-tier mortality roll** (medium; hybrid). Fold the per-being hazard roll into the resident being-kernel (step 3) as a death-mask output; keep `remove_being` on the host. Modest value (once per life cadence) and it pays only because step 3 already has the state resident.

7. **Perceive gather to production** (medium). Promote the spike: SoA-extract `minds`/`place_of`/`sensorium` each tick, roll the notice bitmap over the `(being, trace)` pair domain, stream-compact hits, drain to belief maps on the host. SEAM (stream-compaction, Part 5.6): the variable-length hit set must return in canonical trace-then-being order so the serial apply sees the exact sequence (tag each hit with its trace index, as the existing merge at `world.rs:2281` already does). Gate: dense scenes only.

**Conditional / later (behind unbuilt substrate or a new primitive).** Eikonal flow-field solve (large): write `solve_flow_field` (today `todo!()`), reserve the convergence count, gate the monotone-min sweep. Medium advection and the physics scalar/flux/radiant fields (medium each): all wait on a per-cell field driver, then join the resident field pass as sibling stencils. Archard wear and wide-i128 EM (large): both need the NEW wide-limb primitive before any bit-identical gate, and EM additionally needs a charged-particle population that does not exist.

The resident-buffer / SoA-extraction / stream-compaction triad recurs at exactly three load-bearing points: residency at steps 1, 3, 6; SoA-extraction at steps 3, 4, 7; stream-compaction at step 7 (and any future hybrid returning a variable-length hit set).

## 4. What stays on the parallelised CPU, and why

Two families stay off the device by their nature.

**The branchy cognition.** converse (`world.rs:1620`), gossip (`world.rs:2087`), belief update (`agent.rs:166`), theory-of-mind (`agent.rs:200`), and decide's storage (`world.rs:2189`) are speech-act and inference reasoning over per-mind `BTreeMap` belief and model graphs. They pointer-chase, they diverge, and they do not flatten to SoA; the per-tick extract-and-scatter would cost more than the work. They are already parallelised the right way: across turn owners or minds on worker threads over a frozen `&World`, with the results drained through the canonical barrier. Their draw-keyed tie-breaks are individually pinned, but there is no dense numeric kernel to reproduce.

**The canonical merges and serial barriers.** The CommandBuffer apply (`command.rs:231`), the FNV state-hash walk (`hash.rs:44`), the naming-game consensus (`world.rs:1485`), and the scheduler (`schedule.rs:98`) are order-dependent by contract. The whole point of the barrier is that id minting and reply resolution happen in one canonical CommandKey order independent of worker count; the FNV digest is defined by the sorted walk; the naming game reaches consensus precisely because a later speaker sees an earlier one's adoption in the same tick. Reordering any of these onto a device would produce a different (wrong) result. They run off the hot path (once per tick or checkpoint) and are cheap serial CPU work. The sub-hashes and per-mind shards can fan across workers where the code does not already, but each shard walks a graph, not a grid, so it is a CPU-parallel refinement, never a kernel.

## 5. Honest limits and the cross-cutting gates

**Transfer cost is the master constraint.** Every GPU verdict here is contingent on the data being resident. The memory-bound fields (diffusion, advection, the physics scalar fields) want to stay on the device across ticks; the light per-being kernels (controller, metabolism, body-thermal, individual mortality) have near-one-op-per-byte intensity and lose to a launch unless fused into one resident being-pass; the census sums (`lod.rs:116`) are associative and device-shaped in the abstract but transfer-bound at the individual-tier's real size, earning the device only as a fused tail on columns a prior GPU phase already produced.

**A profile gates each offload.** The dev map (48x24) and dev populations sit below every launch threshold, and the CPU is correct there. Each offload is a scale argument: build the kernel, prove bit-identity, then route through it above a measured crossover, not on assertion. The value ordering above is value x feasibility, but the routing decision is per-deployment and profile-driven.

**Bit-identity is the price of a canonical offload.** Every device path must reproduce the CPU oracle bit-for-bit through the pinned primitives (R-GPU-CANON-PIN: splitmix64, `unit_fixed`, `q32_mul`, `q32_div`, `isqrt`) with its own Stage-0 gate. Two facts qualify this. First, two workloads need a NEW primitive before any gate: Archard wear (`laws.rs:600`) and the wide-i128 EM forms (`coulomb_force` at `laws.rs:1476`) evaluate a full-width raw (unshifted) i128 product and a truncating i128-by-i64 divide to hold sub-2^-32 low bits, which the pinned `q32_mul`/`q32_div` (fixed shift windows) do not cover. Second, cross-vendor without native i64/u64 falls back to u32-limb emulation for splitmix and the field muls, a separate Stage-0 matter noted against the field and noise kernels.

**Residency and scale together, not either alone.** An offload pays only when the data is resident AND the domain is large; one without the other is a loss. This is why the roadmap front-loads the two workloads that escape the double gate (the diffusion field, resident by design, and worldgen noise, one-shot with no per-tick transfer) and treats the fused being-kernel as the enabling infrastructure that makes the rest of the per-tick physics-to-physiology chain amortise.

**Some verdicts are "GPU when built," not "GPU now."** The eikonal solve is `todo!()` (`solve_flow_field`), the medium advection and the atmosphere/hydrology/thermal/radiant physics laws have no per-cell field driver, and the N-body EM form has no particle population. Those verdicts are contingent on the substrate existing and are kept behind their prerequisites in the roadmap rather than counted as available wins.