# Emergent Civilization Simulator: Engine and Simulation Design Document

**Target:** A custom Rust engine simulating a fantasy world from the beginning of its history, across a huge generated map, with civilizations that form, fight, migrate, build, worship, and remember. Every promoted individual has a name, a personality, a goal set, a life history, and a body of beliefs that can diverge from the truth. Folklore, religion, and culture emerge from accumulated transmission rather than authoring. Gameplay is observation through a glyph-based multi-scale view.

**Language:** Rust. **ECS:** hecs. **CPU parallelism:** Rayon plus a custom pinned pool for partitioned passes. **GPU compute:** dual backend, CUDA when present, Vulkan/Metal/DX12 via wgpu otherwise. **Rendering:** instanced glyph renderer on wgpu, sharing the GPU device with compute. **Cross-OS:** Windows, Linux, macOS. **ARM:** a nice-to-have, kept reachable through NEON-capable SIMD and wgpu's Metal backend.

This document is the build reference. It specifies the data structures, the memory model, the function-level shape of each subsystem, and the order to build them.

---

## Part 0: Design Principles

The whole project lives or dies on one tension: maximum performance and maximum per-individual depth pull against each other. A handful of principles resolve that tension, and every later decision derives from them.

**1. Detail is allocated, not uniform, and identity is held separate from processing.** Two different things vary across the world, and conflating them causes trouble. Processing fidelity is how deeply and how often an entity is updated, from full per-tick cognition, through coarse updates every N ticks, down to statistical advancement of a whole population at once. Identity is whether an entity is an individual with a name, a lineage, a history, and a belief store, or an anonymous member of an aggregate pool. These are not the same axis. Sentient beings are always individuals and are never demoted to anonymity: every person who has ever lived keeps a stable identity and history from birth to death, whether or not anyone is watching and whether or not they are doing anything important (Part 54). What scales for a sentient is processing fidelity, not identity, so the quiet majority run a cheap approximation while the active and the significant run full cognition, and belief stores stay sparse. Non-sentient creatures and plants use the full range, existing as aggregate pools until something makes one matter and promotes it to an individual (Part 11). Depth of processing is spent where in-world significance is, a war, a notable figure, an active settlement, and that significance is a property of the world and the seed, not of where the camera points (Principle 10).

**2. Detail is generated lazily and then frozen.** An anonymous member of a population pool has no personality, name, or history until something narratively load-bearing happens to it. At that moment it is promoted to a full entity, its detail is generated deterministically from the pool's statistics and the triggering event, and it is persisted. The illusion of a fully-detailed world holds because anything you inspect becomes detailed on inspection and stays that way.

**3. The canonical state is CPU-authoritative, deterministic, and integer-valued.** The same seed produces the same world on any machine, regardless of thread count. This is achieved with fixed-point and integer math for all authoritative state, per-entity counter-based RNG, and disciplined avoidance of order-dependent parallel reductions. Determinism is a feature: shareable seeds, replayable histories, and tractable debugging.

**4. The GPU owns approximate fields, never canonical truth.** Grid-shaped environmental simulation (water, heat, erosion, terrain genesis, flow-field pathfinding) runs on the GPU. Its results are approximate and non-reproducible across drivers, so they are quarantined: a GPU field influences canonical state only after quantization at the boundary. The GPU is treated like a renderer's approximation that the simulation samples, similar to how a physics preview informs but does not define authoritative positions.

**5. History is a byproduct of logging, not a thing simulated continuously.** Artifacts with stories, buildings with histories, lineages, and rivalries are all queries over a single append-only event log. An artifact's saga is the chronological set of events tagged with its stable id. This costs almost nothing to store and is rich to read.

A sixth, structural observation ties the new rendering requirement into everything above, and it is worth stating as a principle in its own right.

**6. Zoom level, simulation LOD, and spatial partition share one hierarchy, but the camera views it rather than driving it.** The overworld glyph grid, the quadtree that partitions space for parallelism, and the fidelity tiers are one hierarchy, not three separate systems. A glyph at the overworld scale is the aggregate summary of an entire subtree; descending the zoom expands that subtree into finer glyphs and, at the bottom, into individually rendered people, creatures, and plants. The correction this principle now carries, which reconciles it with Principle 3, is that the camera selects what to view and render over that hierarchy and may request a non-authoritative detailed elaboration of a quiet region for viewing, but it never drives canonical simulation fidelity. What runs at high fidelity in canonical truth is decided by in-world significance and the seed (Principle 10, Part 54), so the same hierarchy serves viewing and simulation without the act of looking changing what is simulated. Building these as one structure removes enormous duplication; keeping the camera a viewer and not a cause is what preserves reproducibility.

**7. The world begins at the dawn of sentience.** There is no starting civilization. Every race crosses into sentience at once as scattered bands holding nothing but their intrinsic capabilities and a seeded character of intrinsic beliefs (Part 28), and everything beyond that, language, technique, society, money, law, government, religion, cities, and artifacts, is developed by the races themselves through the emergent systems. The whole arc from a few sentient bands to an empire with a written tongue and a remembered history is produced by the simulation, not handed to it.

**8. Order emerges from behaviour; it is not templated.** Governments, currencies, guilds, legal codes, religious hierarchies, and economic classes are forms the races work out for themselves from their values and their pressures (Part 36), not systems sitting in code waiting to be unlocked. Different cultures invent different forms, or never invent a given form at all, and the engine models the substrate from which institutions crystallize rather than predefining the institutions. A history in which a people never invents money or never centralizes power is valid and is allowed to happen.

**9. Physics may be authored; outcomes may not, and the Steering Audit overrides the designer.** Everything the engine encodes falls on one side of a single boundary. A physical affordance is a fact true of the world regardless of any culture: a material property, a force or contact mechanic, a geometric or thermodynamic consequence, what physically happens when one thing meets another. A cultural outcome is anything a people is meant to reach: a specific design, a name, a method, a category of tool, a recipe, even the knowledge that a given problem can be solved a given way. Physical affordances may be authored, because physics is the one acknowledged and bounded place our own bias is permitted to enter. Cultural outcomes may not be authored, and must emerge from agents conceiving, evaluating, transmitting, and making them (Part 41). The boundary is enforced rather than merely intended: a content gate refuses authored cultural outcomes, every artifact carries a causal provenance back to the agent who conceived it (Part 7), and a convergence audit checks that cultures converge only where physics forces it and diverge everywhere else. This principle exists to override the designer, the author of this document included, whenever a vision is being pressed onto the world that the world should have been left to find for itself.

**10. Observer independence: canonical truth never depends on who is watching.** The seed and the world's own state determine the entire canonical timeline, including which regions and entities run at which fidelity and when. The observer is a viewer: camera position, zoom, and chosen timescale select what is rendered and how fast it plays, and may trigger non-authoritative elaboration for viewing, but they never alter canonical state or fidelity. Looking at a thing does not change its fate, and fast-forwarding does not change what happens. This is what lets the reproducibility of Principle 3 coexist with the detailed, zoomable view of Principle 6, and it is enforced the way the other boundaries are, by keeping the rendering and elaboration layers structurally unable to write canonical state (Part 54, Part 58).

**11. Data-driven by default; hardcoding is the exception that must justify itself.** Anything about the world that is even remotely worth tweaking is defined in data, not fixed in engine code: the races and their drives, traits, values, and genes (Part 20); the creatures and plants and what they feed on (Parts 16, 17); the materials, anatomies, deities, and laws of magic (Part 40); the events, relations, wounds, and institutions a history is made of. A closed enum or a hardcoded constant sitting in the path of world content is treated as a defect until it earns its place. The only admissible exceptions are things whose data-driving would carry massive implications or tradeoffs: the engine mechanics that make the simulation run and stay deterministic (the scheduler, the fixed-point and RNG core, the storage layout), and the single authored physics substrate that Principle 9 already designates as the one bounded place the designer's hand may enter. Every other exception is argued explicitly at its site, never left silent, because a hardcoded assumption presented as settled is precisely the failure this project audits for. The practical test when adding anything: if a designer, a modder, or a different world would reasonably want it different, it is data. This is also why the research backlog exists rather than a sweep of instant conversions: the items left as code are exactly the ones whose conversion has large enough implications to deserve a deliberate session first.

---

## Part 1: Architectural Overview

The engine is four cooperating layers.

```
+-----------------------------------------------------------+
|  Observation layer (thin)                                 |
|  - glyph renderer (wgpu, instanced quads)                 |
|  - reads spatial hierarchy at current zoom level          |
|  - reads GPU field buffers directly for overlays          |
|  - camera focus drives LOD promotion                      |
+-----------------------------------------------------------+
|  Deterministic CPU core (authoritative)                   |
|  - hecs world (fast physical components, SoA)             |
|  - stable-id registry (promotion/demotion coherence)      |
|  - knowledge / belief / culture subsystems (side stores)  |
|  - append-only event log (ground truth history)           |
|  - fixed-point math, per-entity counter RNG               |
|  - phase scheduler: Rayon + pinned partitioned pool       |
+-----------------------------------------------------------+
|  GPU compute (approximate fields)                         |
|  - CubeCL: one #[cube] kernel source per workload         |
|  - JIT to CUDA (NVIDIA) / Vulkan,Metal,DX12 (wgpu) / CPU  |
|  - water, heat, erosion, terrain noise, flow fields       |
|  - grid buffers resident across ticks                     |
|  - async readback of agent-relevant slice only            |
+-----------------------------------------------------------+
|  Persistence                                              |
|  - rkyv zero-copy snapshots (mmap-able world state)       |
|  - bincode streaming event log                            |
|  - compaction: full records only for promoted entities   |
+-----------------------------------------------------------+
```

Data flows in a fixed direction each tick. The GPU advances the environmental fields. The CPU reads back only the slice its agents need. High-LOD agents perceive, think, and act; their actions are recorded as events in the log and applied to canonical state at a single-threaded sync point. Belief propagation runs over the new events. Bookkeeping decays beliefs, checks demotion triggers, and swaps the double buffer. The renderer, running at its own slower cadence, reads the spatial hierarchy and the GPU field buffers to draw glyphs.

The remainder of the document specifies each box.

---

## Part 2: Core Data Model and Memory Architecture

### 2.1 The two kinds of identity

There are two id concepts and conflating them causes pain later, so they are separated from the start.

A **`StableId`** is a process-wide, monotonically assigned `u64` that names a conceptual entity (a person, an artifact, a building, a culture) for its entire existence and beyond. It never changes and is never reused. The event log, belief provenance pointers, and relationship edges all reference `StableId`s, because those references must survive promotion, demotion, save, and load.

A **`hecs::Entity`** is hecs's own generational index, valid only while an entity is promoted and resident in the ECS. It is fast and cache-local but unstable across promotion/demotion and not suitable for serialization.

The bridge between them is the **stable-id registry**.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StableId(pub u64);

pub enum EntityLocation {
    /// Currently a full ECS entity.
    Promoted(hecs::Entity),
    /// Currently summarized inside an aggregate pool.
    Pooled { pool: PoolId, slot: u32 },
    /// Recorded in history, no live representation.
    Retired,
}

pub struct Registry {
    next_id: u64,
    locations: HashMap<StableId, EntityLocation>,
    // Reverse map for the hot path: given a live ECS entity, recover its StableId.
    // Stored as a component on the entity instead (see StableRef below) to avoid a lookup.
}

impl Registry {
    pub fn mint(&mut self) -> StableId {
        let id = StableId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn locate(&self, id: StableId) -> EntityLocation { /* ... */ }
    pub fn set_location(&mut self, id: StableId, loc: EntityLocation) { /* ... */ }
}
```

Every promoted entity carries its own `StableId` as a component, so the common direction (live entity to stable id) is a component read with no map lookup:

```rust
#[derive(Clone, Copy)]
pub struct StableRef(pub StableId);
```

The rare direction (stable id to current location) goes through `Registry::locate`, which is only needed when resolving a log reference or a relationship edge whose target may have been demoted.

### 2.2 hecs as the fast-component store

hecs holds only the components that are iterated in bulk every tick and that benefit from contiguous SoA layout. These are the "physical" facts of an entity. Rich, variable-length, churn-heavy data (beliefs, memories, cultural membership detail) does not live here; it lives in side subsystems referenced by handle (Section 2.4).

The component catalog for a high-LOD person:

```rust
#[derive(Clone, Copy)]
pub struct Position { pub x: i32, pub y: i32, pub z: i16 }

#[derive(Clone, Copy)]
pub struct Velocity { pub dx: Fixed, pub dy: Fixed }   // Fixed defined in Part 3

// Aliveness and age. A being's physical condition is not a single health scalar; it is the
// part-level body (Part 35), built from the race's data-defined anatomy (Part 40) and reached by
// BodyRef. `alive` is the cached result of the part-level check (a vital part destroyed, or a
// fluid pool past its critical fraction), and any aggregate "how hurt" figure the AI or the
// abstract battle model needs is derived from the body rather than stored as a separate truth.
#[derive(Clone, Copy)]
pub struct Vitals { pub age_days: u32, pub alive: bool }

/// What the agent is doing right now; drives the action system.
#[derive(Clone, Copy)]
pub struct Activity { pub kind: ActionId, pub target: Option<StableId>, pub progress: Fixed }

/// Handles into the side subsystems. Small and copyable.
/// A being's drives, personality traits, and value leanings are NOT fixed structs of named
/// fields. They are per-axis columns in the attribute store, sized by the race's data-defined
/// attribute sets (Parts 20, 40), reached through this handle. A race that has no hunger has no
/// hunger column; a race with no "extraversion" axis has no such trait. Storing them as columns
/// keeps the per-axis decay and scoring passes the SoA loops the utility AI wants (Part 8).
#[derive(Clone, Copy)]
pub struct AttributeRef(pub u32);     // index into the per-axis attribute store (drives, traits, values)
#[derive(Clone, Copy)]
pub struct BodyRef(pub u32);          // index into the part-level body store (Part 35), built from anatomy
#[derive(Clone, Copy)]
pub struct KnowledgeRef(pub u32);     // index into the belief arena
#[derive(Clone, Copy)]
pub struct SocialRef(pub u32);        // index into the relationship adjacency store
```

Spawning a person assembles the archetype once:

```rust
fn spawn_person(world: &mut hecs::World, reg: &mut Registry, knowledge: &mut KnowledgeArena,
                attrs: &mut AttributeStore, bodies: &mut BodyStore,
                race: &Race, seed_ctx: SeedCtx) -> (StableId, hecs::Entity) {
    let id = reg.mint();
    let kref = knowledge.alloc_store();
    // Allocate this being's attribute columns from its race's data-defined drive, trait, and value
    // sets (Part 20), and its body from the race's anatomy (Parts 35, 40), both seeded
    // deterministically (Part 3). A race without a given axis gets no column.
    let aref = attrs.alloc_for_race(race, seed_ctx);
    let bref = bodies.alloc_from_anatomy(race, seed_ctx);
    let entity = world.spawn((
        StableRef(id),
        Position { /* from spawn context */ x: 0, y: 0, z: 0 },
        Velocity { dx: 0, dy: 0 },
        Vitals { age_days: 0, alive: true },
        Activity { kind: ActionId::IDLE, target: None, progress: 0 },
        aref,
        bref,
        kref,
        SocialRef(/* allocate */ 0),
    ));
    reg.set_location(id, EntityLocation::Promoted(entity));
    (id, entity)
}
```

hecs is archetypal, so an entity with this exact component set lives in one contiguous table and iterates at full speed. Adding or removing a component moves the entity between tables; we avoid doing that on the hot path. Transient states that would otherwise churn the archetype (a temporary curse, a combat state) are handled in one of two ways: a small bitflag field inside an existing component for cheap binary states, or a side sparse map keyed by `StableId` for richer transient data. This sidesteps hecs's lack of per-component sparse storage without giving up archetypal iteration speed where it matters.

### 2.3 Arena and slab allocation

Hot subsystem data is allocated in arenas for contiguous layout, cheap bulk reset, and predictable addresses for SIMD. The pattern, used by the knowledge arena, the event log, and the per-chunk scratch buffers:

```rust
pub struct Arena<T> {
    blocks: Vec<Box<[MaybeUninit<T>]>>, // grown in large fixed-size blocks
    len: usize,
    block_size: usize,
}

impl<T> Arena<T> {
    pub fn with_block_size(n: usize) -> Self { /* ... */ }
    pub fn push(&mut self, value: T) -> u32 { /* returns a stable index */ }
    pub fn get(&self, idx: u32) -> &T { /* ... */ }
    pub fn get_mut(&mut self, idx: u32) -> &mut T { /* ... */ }
    /// Free everything at once (end of a transient scope).
    pub fn reset(&mut self) { self.len = 0; }
}
```

Per-chunk arenas align with the spatial partition (Section 6) and with NUMA affinity (Part 4): a chunk's data is allocated by the worker that owns that chunk's CCD, so it stays L3-resident.

For slots that are freed and reused individually (the knowledge stores of demoted entities, for example), a slab with a free list and generational guard is used instead, so a stale handle fails a generation check rather than aliasing a reused slot:

```rust
pub struct Slab<T> {
    entries: Vec<SlabEntry<T>>,
    free_head: Option<u32>,
}
struct SlabEntry<T> { generation: u32, slot: Slot<T> }
enum Slot<T> { Occupied(T), Vacant(Option<u32> /* next free */) }

#[derive(Clone, Copy)]
pub struct SlabHandle { index: u32, generation: u32 }
```

### 2.4 The knowledge subsystem (where the sparse instinct lives)

An individual's beliefs are a variable-length, graph-structured, per-entity database. This is not an ECS component in either archetypal or sparse form; it is a side store, referenced from the ECS by a `KnowledgeRef` handle. Inside the store we have total freedom of layout, which is exactly where churn-heavy, heterogeneous belief data belongs. The full structure is specified in Part 9; here we only fix the handle and the allocation surface so the rest of the model can reference it.

```rust
pub struct KnowledgeArena {
    stores: Slab<BeliefStore>, // one per promoted entity that holds beliefs
}
impl KnowledgeArena {
    pub fn alloc_store(&mut self) -> KnowledgeRef { /* returns handle */ }
    pub fn store(&self, r: KnowledgeRef) -> &BeliefStore { /* ... */ }
    pub fn store_mut(&mut self, r: KnowledgeRef) -> &mut BeliefStore { /* ... */ }
    pub fn free_store(&mut self, r: KnowledgeRef) { /* on demotion, after summarizing */ }
}
```

### 2.5 Memory layout rules

Three rules govern hot data placement. First, anything iterated in bulk is structure-of-arrays, which hecs gives us per archetype. Second, SoA arrays consumed by SIMD are aligned to 32 bytes for AVX2 and 64 bytes for AVX-512, so loads are aligned. Third, data written independently by different worker threads is padded to a 64-byte cache line to prevent false sharing, using a small wrapper:

```rust
#[repr(align(64))]
pub struct CacheLine<T>(pub T);
```

Per-thread accumulators, per-chunk dirty flags, and any other write-hot per-worker scalar are wrapped this way so two CCDs never contend on one line.

---

## Part 3: Determinism Architecture

Determinism is the property that one seed yields one world, bit for bit, on any machine and at any thread count. It is fragile and must be engineered from the first commit, because retrofitting it means auditing every subsystem.

### 3.1 Fixed-point canonical math

All authoritative numeric state is fixed-point or integer, on the CPU and on the GPU alike. Floating point is permitted only in non-authoritative work: non-authoritative GPU kernels (render fields, overlays, view-time elaboration) and the renderer, neither of which feeds back into canonical state. Canonical GPU field kernels are fixed-point integer exactly like the CPU core, which is what lets them hold authoritative state while staying bit-identical across machines (Section 5.4). A `Q32.32` fixed-point type backed by `i64` gives ample range and resolution for bounded quantities like needs, health, positions-as-subtile, strengths, and probabilities, and it is the default representation for those. It is not the only scale: quantities with a much wider range or that accumulate over centuries (wealth, wear, deposited sediment) carry their own per-quantity fixed-point scale with an explicit overflow discipline under the unit and dimensional system of Part 55, so Q32.32 is the common case rather than a single global format.

```rust
pub type Fixed = i64;
pub const FRAC_BITS: u32 = 32;
pub const ONE: Fixed = 1 << FRAC_BITS;

#[inline] pub fn fx_from_int(i: i32) -> Fixed { (i as i64) << FRAC_BITS }
#[inline] pub fn fx_to_int(x: Fixed) -> i32 { (x >> FRAC_BITS) as i32 }

#[inline]
pub fn fx_mul(a: Fixed, b: Fixed) -> Fixed {
    // 128-bit intermediate avoids overflow, then shift back.
    (((a as i128) * (b as i128)) >> FRAC_BITS) as i64
}
#[inline]
pub fn fx_div(a: Fixed, b: Fixed) -> Fixed {
    (((a as i128) << FRAC_BITS) / (b as i128)) as i64
}
```

The rule for contributors: if a value can influence canonical state, it is `Fixed` or an integer. No `f32` or `f64` crosses into authoritative components.

### 3.2 Per-entity counter-based RNG

There is no shared global RNG, because shared state makes results depend on the order draws happen, which depends on scheduling. Instead, each draw is a pure function of a coordinate `(master_seed, entity, phase, counter)`. The same entity, in the same phase, asking for its k-th random number, always gets the same number, no matter which thread serves it or when. The mixing uses a SplitMix64 finalizer (a hash), not XOR, so that nearby entity ids do not produce correlated streams.

```rust
#[inline]
fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// A cheap, stateless stream. `counter` is advanced by the caller per draw.
#[derive(Clone, Copy)]
pub struct Rng { key: u64 }

impl Rng {
    pub fn for_entity(master_seed: u64, id: StableId, phase: u32) -> Self {
        let k = splitmix64(master_seed ^ id.0.rotate_left(17));
        Rng { key: splitmix64(k ^ ((phase as u64) << 1)) }
    }
    #[inline] pub fn at(&self, counter: u64) -> u64 { splitmix64(self.key ^ counter) }

    #[inline] pub fn unit_fixed(&self, counter: u64) -> Fixed {
        // 0..ONE
        ((self.at(counter) >> 32) as i64).wrapping_mul(ONE >> 32)
    }
    #[inline] pub fn range_u32(&self, counter: u64, n: u32) -> u32 {
        // Lemire's bounded method, deterministic.
        (((self.at(counter) as u128) * (n as u128)) >> 64) as u32
    }
}
```

Because the stream is indexed by an explicit counter, a system that needs several draws for one entity uses `at(0)`, `at(1)`, and so on, and replays identically. No draw-count bookkeeping is stored.

### 3.3 The parallel reduction hazard

Floating-point addition is not associative, so summing a quantity over many agents in parallel can give different totals depending on how the work was chunked across threads. This is the single most common way parallel simulations lose determinism. Three defenses, applied together:

Canonical accumulators are integer or `Fixed`, where addition is associative and the total is independent of order. A parallel sum of `Fixed` values is safe.

Where a reduction must be floating-point (only ever in non-authoritative GPU work, never in a canonical kernel), the result is treated as approximate and quantized before it can affect canonical state (Section 3.4).

Any reduction whose order could vary is replaced with a fixed-order fold, for example a deterministic tree reduction over a fixed partition independent of the runtime thread count, or a sequential fold when the cost is acceptable.

### 3.4 GPU work: canonical in fixed-point, quarantined in float

The GPU runs in one of two modes, and the determinism rule differs by mode. The distinction was settled by the exploration recorded in Part 62 and is detailed for the compute layer in Section 5.4.

A canonical GPU kernel computes an authoritative field in fixed-point integer only. Integer addition is associative and integer arithmetic is exact, so the result is independent of how the hardware schedules its lanes and is bit-identical across vendors and drivers, the same property GPU cryptography relies on to produce identical hashes on different cards. A field computed this way holds authoritative state directly, with no quantizer, because there is no floating-point variance to contain, and a seed reproduces it identically on any machine. The hard rule is that no floating-point value appears anywhere in a canonical kernel, the GPU counterpart of the typed canonical-state boundary of Part 58; the transcendentals physics needs are implemented in fixed-point (an exact integer square root, an exponential by fixed-point series) at a small fixed precision cost that is the same on every machine.

A non-authoritative GPU kernel is anything whose output only reaches the screen: render fields, overlays, and the view-time elaboration of a watched region. These may use floating point freely, because the typed boundary makes them structurally unable to write canonical state (Principle 10), so their nondeterminism does no harm. Where such a float field is nonetheless read by the CPU to inform a decision, its value crosses into canonical state only through a quantizer that snaps it to an integer canonical unit, identical across machines for the same quantized input.

```rust
/// A non-authoritative float field read back to inform a decision: water depth
/// as f32 metres becomes canonical millimetres through a stable quantizer.
#[inline]
pub fn quantize_depth(metres: f32) -> i32 {
    (metres * 1000.0).round_ties_even() as i32 // round-half-to-even, then clamp
}
```

Which fields can be canonical on the GPU is decided empirically, not assumed: the Stage 0 spike confirms that each candidate kernel produces bit-identical output across the CUDA, Vulkan, and Metal backends, and any operation that diverges or is too costly in fixed-point falls back to the CPU, which is still deterministic, or stays a non-authoritative quantized field.

### 3.5 The determinism test harness

A test rig is built on day one and run in CI. It executes the same seed at one thread, four threads, and the worker count of the machine, hashes the full canonical state at fixed checkpoints, and asserts the hashes match. A divergence is a regression caught immediately, when the offending change is small.

```rust
fn determinism_check(seed: u64, ticks: u64) {
    let a = run_headless(seed, ticks, Threads(1));
    let b = run_headless(seed, ticks, Threads(4));
    let c = run_headless(seed, ticks, Threads::auto());
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.state_hash(), c.state_hash());
}
```

`state_hash` walks canonical state in a fixed order (sorted by `StableId`, not by hash-map iteration order, which is itself a determinism trap) and folds it into a 128-bit hash.

---

## Part 4: Concurrency and Scheduling

### 4.1 The tick as an ordered sequence of phases

A tick is a fixed sequence of phases. Phases run in order; within a phase, work is parallel where the access pattern allows. The order is part of the determinism contract.

```rust
pub enum Phase {
    EnvironmentGpu,    // dispatch GPU field kernels; no CPU state touched
    Readback,          // collect the previous tick's async readback slice
    Perception,        // CPU, parallel: each agent reads world + field slice -> percepts
    Cognition,         // CPU, parallel: utility AI scores actions, picks one
    ActionStage,       // CPU, parallel: produce action intents into per-worker command buffers
    ActionApply,       // CPU, single-threaded: drain command buffers in StableId order
    Events,            // append resulting events to the log (single-threaded append)
    Gossip,            // CPU, parallel over speakers: belief propagation
    Culture,           // CPU, periodic: recompute culture aggregates (not every tick)
    Bookkeeping,       // decay beliefs, check demotion/promotion triggers
    Swap,              // swap double buffer, advance tick counter
}
```

The split between `ActionStage` (parallel, pure, produces intents) and `ActionApply` (single-threaded, mutates the shared graph in a fixed order) is the key to safe parallel agents. Agents never mutate shared state mid-tick; they emit intents that are applied deterministically afterward.

### 4.2 Double-buffered world state

Mutable per-tick state that agents read from each other is double-buffered. Agents read the immutable previous buffer and write only their own slot in the next buffer. Because each agent's update depends only on the frozen snapshot and writes only to its own slot, the result is identical regardless of which thread computes which agent. This is what makes the parallel `Perception`, `Cognition`, and `ActionStage` phases both safe and deterministic.

```rust
pub struct DoubleBuffer<T> { front: T, back: T }
impl<T> DoubleBuffer<T> {
    pub fn read(&self) -> &T { &self.front }
    pub fn write(&mut self) -> &mut T { &mut self.back }
    pub fn swap(&mut self) { std::mem::swap(&mut self.front, &mut self.back); }
}
```

Not all state is double-buffered. The event log is append-only and written only in the single-threaded `Events` phase. The belief stores are mutated in `Gossip`, which is parallelized over speakers with disjoint write sets (Section 9.5 explains the partitioning that keeps that safe).

### 4.3 Command buffers for structural change

Structural changes (spawn, despawn, add or remove a component, promote, demote) cannot happen concurrently with iteration. They are deferred into per-worker command buffers and applied single-threaded at `ActionApply`, drained in `StableId` order so the application is deterministic.

```rust
pub enum Command {
    Spawn { id: StableId, blueprint: Blueprint },
    Despawn { id: StableId },
    SetActivity { id: StableId, activity: Activity },
    Promote { pool: PoolId, slot: u32, reason: PromotionReason },
    Demote { id: StableId },
    EmitEvent(Event),
}

#[derive(Default)]
pub struct CommandBuffer { cmds: Vec<Command> }
impl CommandBuffer {
    pub fn push(&mut self, c: Command) { self.cmds.push(c); }
}

/// Applied single-threaded, sorted for determinism.
fn apply_commands(world: &mut hecs::World, reg: &mut Registry,
                  log: &mut EventLog, mut buffers: Vec<CommandBuffer>) {
    let mut all: Vec<Command> = buffers.drain(..).flat_map(|b| b.cmds).collect();
    all.sort_by_key(command_sort_key); // stable, StableId-based
    for c in all { apply_one(world, reg, log, c); }
}
```

### 4.4 Two schedulers for two shapes of work

The tick mixes two work shapes, and each gets the scheduler that fits it.

Bulk irregular data-parallel passes, where per-item cost varies a lot (iterating every high-LOD agent, whose cognition cost depends on its situation), use Rayon. Its work-stealing load-balances automatically, stealing from busy workers, which is exactly right when items are uneven. The work is chunked coarsely so Rayon's coordination overhead stays amortized.

```rust
use rayon::prelude::*;

fn cognition_phase(agents: &[AgentSlice], out: &mut [Decision], ctx: &TickCtx) {
    agents.par_iter().zip(out.par_iter_mut()).for_each(|(a, d)| {
        *d = score_and_choose(a, ctx);   // pure: reads snapshot, writes own slot
    });
}
```

Statically partitioned passes, where the map is divided into regions and we want each region's data to stay resident on one CCD's L3, use a custom pinned pool. Work-stealing would move a region's work to a core whose cache does not hold that region, so for these passes we assign region R to a fixed worker pinned to R's CCD and run a fixed schedule. This also yields a reproducible region-to-worker mapping. The pool is built on `crossbeam` and `std::thread`, with affinity set through `core_affinity`. A custom Rayon `ThreadPool` with a pinning spawn handler is the pragmatic middle option if we want to keep `par_iter` while gaining affinity, accepting that work-stealing still moves tasks across the pinned threads.

```rust
pub struct PinnedPool {
    workers: Vec<std::thread::JoinHandle<()>>,
    submit: crossbeam::channel::Sender<RegionJob>,
    done: crossbeam::channel::Receiver<RegionResult>,
}

impl PinnedPool {
    pub fn new(core_for_region: impl Fn(RegionId) -> CoreId) -> Self {
        // spawn one worker per region-group; each pins itself with core_affinity
        // to the core/CCD that owns its regions' chunk arenas.
        todo!()
    }
    /// Run one fixed-order pass over all regions; blocks until complete.
    pub fn run_pass(&self, regions: &[RegionId], f: impl Fn(RegionId) + Sync) { todo!() }
}
```

The graduation rule: start every pass on Rayon. Move a pass to the pinned pool only when profiling shows either work-stealing coordination eating the tick budget or cross-CCD memory traffic hurting that pass. Building the pinned pool before that evidence exists is premature.

### 4.5 Async I/O isolation

Save streaming and any networking run on their own threads or a dedicated runtime, never interleaved with the compute pools. Rayon and an async runtime must not share threads; a blocking call inside a Rayon job on a runtime thread can deadlock or abort. Subsystems communicate with the I/O thread through channels: the simulation hands finished snapshot chunks and event batches to a writer that serializes them off the hot path.

---

## Part 5: GPU Compute Backend

The GPU runs grid-shaped, divergence-free workloads. Agent cognition never goes here: CubeCL's execution model still has planes (warps on CUDA, subgroups on WebGPU, SIMD-groups on Metal) whose units execute the same instruction in lockstep, so the branchy, pointer-chasing nature of agent brains would serialize against that model exactly as it would on raw CUDA or Vulkan. CubeCL does not make divergent code fast; it makes one kernel source target every backend. Agent brains stay on the CPU.

Rather than maintain separate CUDA and Vulkan backends, the engine uses **CubeCL**. A kernel is written once as a `#[cube]`-annotated Rust function and just-in-time compiled on demand to CUDA PTX on NVIDIA, to SPIR-V (Vulkan) or Metal or DX12 through the wgpu runtime elsewhere (including macOS, which has neither native Vulkan nor CUDA), to AMD HIP/ROCm where present, and to CPU SIMD as a first-class fallback, each using the best instructions that platform offers. One kernel source, every target.

### 5.1 Why CubeCL replaces the hand-rolled backends

The dual-backend plan, a CUDA path and a Vulkan path behind a hand-written trait, was the single largest maintenance cost in the original design: every kernel had to exist twice, in CUDA C and in WGSL, and stay in sync. CubeCL removes that. A single `#[cube]` function is the source of truth, type-checked and borrow-checked like ordinary Rust, and CubeCL's just-in-time compiler lowers it to each backend's native form at first launch. It is the compute layer beneath the Burn machine-learning framework, so it is exercised in real production, and it sits deliberately between low-level wrappers like wgpu and cudarc and high-level frameworks, which is exactly the layer this engine wants. It provides automatic vectorization, compile-time specialization through `comptime`, autotuning of launch parameters at first run, and a memory-management strategy built around heavy buffer reuse to avoid per-dispatch allocation. Its own runtime abstraction (`Runtime`, `ComputeClient`, `ComputeServer`, `Channel`) already wraps the wgpu and CUDA runtimes, so it subsumes the compute-backend trait the original design would have hand-written. The whole `compute-cuda` and `compute-wgpu` split collapses into one `compute` crate of `#[cube]` kernels.

The tradeoffs are real and stated plainly. CubeCL is a restricted DSL: it currently supports functions, generics, and structs, with partial support for traits, methods, and type inference, so kernels are written in a subset of Rust rather than arbitrary Rust. It is a young project from a small team and still has rough edges, so versions are pinned and breaking changes are expected. Neither outweighs deleting an entire parallel kernel codebase.

### 5.2 The topology model

CubeCL describes hardware as a hierarchy of units inside cubes inside a hyper-cube (a cube maps to a CUDA block, the hyper-cube to a CUDA grid), with planes (warps, subgroups) of units executing in lockstep, across several orthogonal axes of parallelism. Topology values are constants inside a kernel and use Rust constant syntax in capitals: `ABSOLUTE_POS` for a unit's global index (equivalent to `CUBE_POS * CUBE_DIM + UNIT_POS`), plus `CUBE_DIM`, `UNIT_POS`, `PLANE_DIM`, and the rest. The one rule worth internalizing is that good kernels read these maxima at `comptime` and adapt rather than hardcoding them. Writing `PLANE_DIM == 32` is a CUDA-ism that breaks on AMD, where a wavefront is 64, and is meaningless on the CPU target, where there is no plane and `PLANE_DIM` is 1. Adaptive kernels are why CPU is a real target and not a bolt-on, and why autotune helps: it searches the comptime-resolved choices along these axes.

### 5.3 Kernel authoring and launch

A kernel is an ordinary-looking Rust function annotated `#[cube(launch)]`, or `#[cube(launch_unchecked)]` to skip bounds-check insertion, taking CubeCL `Array`, `Tensor`, and scalar handles. Helper functions reused across kernels are plain `#[cube]`. The same function compiles to CUDA on the NVIDIA workstation and to Vulkan or Metal through wgpu on the AMD and Apple machines. (The API below matches CubeCL's current examples; pin the version, because the surface still moves.)

```rust
use cubecl::prelude::*;

// Each unit handles one 2x2 Margolus block of the water grid.
#[cube(launch)]
fn water_margolus(src: &Array<u32>, dst: &mut Array<u32>, #[comptime] dims: GridDims, parity: u32) {
    let idx = ABSOLUTE_POS;
    if idx < src.len() {
        // derive the four cell indices of this block from idx, dims, and parity,
        // read their water amounts from `src`, redistribute by the block rule,
        // and write the result into `dst`.
    }
}
```

The GPU-facing code is generic over the runtime `R: Runtime`; only startup names a concrete backend, which is exactly how CubeCL's own examples select a runtime in `main`. Both monomorphizations are compiled into the binary behind feature flags, and a runtime capability check picks which to instantiate, so one build runs natively on every target.

```rust
use cubecl::prelude::*;

// Created once at startup for the detected runtime, then reused every tick.
fn make_client<R: Runtime>(device: &R::Device) -> ComputeClient<R::Server, R::Channel> {
    R::client(device)
}

// A per-tick environment step, generic over the backend.
fn env_step<R: Runtime>(client: &ComputeClient<R::Server, R::Channel>,
                        src: &Handle, dst: &Handle, dims: GridDims, parity: u32, n: usize) {
    unsafe {
        water_margolus::launch::<R>(
            client,
            CubeCount::Static((n as u32).div_ceil(64), 1, 1),
            CubeDim::new(64, 1, 1),
            ArrayArg::from_raw_parts::<u32>(src, n, 1),
            ArrayArg::from_raw_parts::<u32>(dst, n, 1),
            dims,
            ScalarArg::new(parity),
        );
    }
}

// The single place the backend is chosen.
fn main() {
    if cfg!(feature = "cuda") && cuda_device_present() {
        run_engine::<cubecl::cuda::CudaRuntime>(&Default::default());
    } else {
        run_engine::<cubecl::wgpu::WgpuRuntime>(&Default::default()); // Vulkan/Metal/DX12
    }
}
```

Grid buffers are CubeCL handles created with `client.create(bytes)` (or `client.empty(size)`) that stay resident on the device across ticks, exactly as the resident-buffer rule (Section 5.6) requires. Readback is `client.read(handle.binding())`, kept to the agent-relevant slice and issued asynchronously per that same section. Vectorization is a launch-time factor on the array arguments, so the same kernel body lowers to the best packed-SIMD instruction on each backend without per-target code.

### 5.4 Canonical GPU work and the float boundary

The GPU is used in two modes, and which mode a kernel is in determines whether it can hold authoritative state. The reasoning behind this split is recorded in Part 62.

A canonical kernel computes an authoritative field and is written in fixed-point integer with no floating point at all. Because integer addition is associative and integer arithmetic is exact, the result does not depend on how the hardware schedules its lanes and is bit-identical across vendors, drivers, and backends, which is the property that lets the field hold canonical state while a seed still reproduces it on any machine. This is the GPU counterpart of the typed canonical-state boundary (Part 58): no float crosses into a canonical kernel. The transcendentals the physics needs are implemented in fixed-point rather than reached for from the hardware float unit, an exact integer square root for magnitudes and distances, an exponential by fixed-point series for decay and diffusion kernels, at a small fixed precision cost that is identical on every machine. The Margolus-block fluids, heat and smoke diffusion, and hydraulic and thermal erosion of Section 5.5 are all expressible this way.

A non-authoritative kernel computes something that only reaches the screen, a render field, an overlay, or the view-time elaboration of a watched region, and is free to use floating point at full speed, because the typed boundary and Principle 10 make it structurally unable to write canonical state, so its nondeterminism does no harm. Its float result is never read back into canonical state except through the quantizer of Section 3.4.

Which fields are canonical on the GPU is settled by measurement, not assumption. The Stage 0 spike (Part 60) confirms that each candidate kernel produces bit-identical output across the CUDA, Vulkan, and Metal backends and the CPU backend used as an oracle, finds any operation where that breaks, and weighs the throughput of the integer kernel against a float baseline. An operation that cannot be made bit-identical across backends, or that is too slow in fixed-point to be worth it, falls back to the CPU, which is still deterministic, or remains a non-authoritative quantized field. The existence proof that this is achievable at all is GPU cryptography, which produces identical hashes across different vendors' hardware using exactly this integer-only, no-hardware-transcendental discipline.

### 5.5 The workload catalog

Everything on the GPU is a stencil: each cell reads its neighbours, applies a rule, writes the result. All of the following fit:

Water flow and pressure, heat and fire and smoke diffusion, hydraulic and thermal erosion, rainfall and wind, soil fertility, and pollution or disease modelled as diffusion. Terrain genesis (simplex and worley noise, then hydraulic erosion and river carving) is embarrassingly parallel per cell and produces the starting world. Flow-field pathfinding solves a parallel wavefront (eikonal) over the grid to produce a vector field that any number of agents sample for free, which is the scalable alternative to per-agent A*.

The classic falling-sand and water race (two cells trying to move into one) is resolved with block cellular automata using the Margolus neighbourhood: the grid is partitioned into two-by-two blocks, and the block grid is shifted diagonally on alternate ticks, so each cell is updated within exactly one block per tick and conflicts cannot occur. This is the standard GPU technique for cellular fluids, and it is the `water_margolus` `#[cube]` kernel shown in Section 5.3. Heat, smoke, and disease diffusion are simpler stencils on the same pattern; erosion couples a few fields (water, sediment, elevation) but is still a per-cell stencil.

### 5.6 The CPU to GPU seam

PCIe has high latency and limited bandwidth relative to on-GPU memory, so the seam is governed by strict rules. Grid state stays resident in GPU storage buffers across ticks; the whole grid is never round-tripped each frame. The CPU reads back only the slices its agent brains need (the cells under high-LOD agents, fired events, summary statistics), and it does so asynchronously and double-buffered: it maps a staging buffer and reads the previous tick's copy while the GPU computes the next. Synchronous readback and per-agent-per-tick small transfers are forbidden, because latency dominates and stalls both processors. All agent queries for a tick are batched into one readback region.

```rust
// Each tick: launch advances the resident field in place; we only pull the agent slice.
fn environment_phase<R: Runtime>(client: &ComputeClient<R::Server, R::Channel>,
                                 fields: &mut FieldBuffers, agents_bbox: GridRect, parity: u32) {
    // src and dst are resident handles that we ping-pong; nothing leaves the device here.
    env_step::<R>(client, &fields.water_src, &fields.water_dst, fields.dims, parity, fields.n);
    env_step::<R>(client, &fields.heat_src,  &fields.heat_dst,  fields.dims, parity, fields.n);
    fields.swap(); // src <-> dst handles
    // Issue the async readback of just the agent-relevant slice; collect it next tick.
    fields.pending = Some(client.read_async(fields.water_src.slice(agents_bbox.byte_range())));
}
```

### 5.7 CPU-side SIMD

The CPU's own numeric hot loops (needs decay, distance checks, the aggregate population update) are SIMD over SoA arrays. The `wide` crate gives portable SIMD on stable Rust and lowers to AVX on x86 and NEON on ARM from one source, which keeps the ARM target reachable. Structure-of-arrays layout is the precondition: process eight agents per AVX2 instruction, sixteen per AVX-512. AVX-512 width is an unambiguous win on AMD Zen 4 and Zen 5 desktop parts, which have no meaningful frequency penalty; on Intel parts before Ice Lake the 256-bit AVX2 path is preferred because of the historic downclocking, and on Zen 5 mobile (Strix Point) the vector unit is narrower. The width is therefore chosen at runtime by feature detection rather than baked in.

```rust
use wide::f32x8; // illustrative; canonical accumulators stay integer/Fixed

// Needs decay over a contiguous SoA column, eight lanes at a time.
fn decay_hunger(col: &mut [Fixed], rate: Fixed) {
    for chunk in col.chunks_mut(8) {
        for h in chunk { *h = fx_mul(*h, rate); } // scalar fallback shown;
        // the SIMD path packs 8 Fixed values, multiplies, and stores aligned.
    }
}
```

---

## Part 6: Spatial Architecture

Space is one hierarchy, and it is the same hierarchy that drives both parallelism and the zoom-based renderer (Part 14). It has four levels of granularity, coarsest to finest.

### 6.1 The four levels

A **region** is the coarsest partition, a large square of the map assigned to one CCD-pinned worker for the partitioned passes (Part 4). A region owns its chunk arenas so its data stays L3-resident. A **chunk** is a fixed-size square of tiles, the unit of loading, simulation, and persistence, and an independent work item. A **tile** is one cell of the world grid, carrying terrain, an environmental field sample index, and the occupancy of whatever stands on it. A **subtile** position lets agents move smoothly within a tile using fixed-point fractional coordinates.

```rust
pub const CHUNK: usize = 64;        // 64x64 tiles per chunk
pub const REGION_CHUNKS: usize = 16; // 16x16 chunks per region

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord { pub cx: i32, pub cy: i32, pub cz: i16 }

pub struct Chunk {
    terrain: [Terrain; CHUNK * CHUNK],     // SoA-friendly, one byte-ish each
    field_index: [u32; CHUNK * CHUNK],     // into GPU field buffers
    occupants: SmallVec<[StableId; 8]>,    // entities currently in this chunk
    site: Option<SiteId>,                  // a settlement or structure here
    dirty: CacheLine<bool>,                // for incremental updates, padded
}
```

### 6.2 The quadtree (and octree for z)

Above the chunk grid sits a quadtree (an octree where vertical z-levels matter) used for two purposes that turn out to be the same purpose: multi-resolution level of detail, and variable-density spatial queries. Each node summarizes its subtree: dominant terrain, total population, presence of a site, the highest-salience event in the subtree. The renderer reads exactly one node per visible cell at the current zoom depth and draws one glyph from that node's summary. Descending the zoom descends the tree. The simulation reads the tree to decide which regions are near the camera and should be promoted.

```rust
pub struct QuadNode {
    bounds: GridRect,
    summary: NodeSummary,             // dominant terrain, pop, site flag, top event
    children: Option<Box<[QuadNode; 4]>>, // None at leaf (chunk) level
}

pub struct NodeSummary {
    pub dominant_terrain: Terrain,
    pub population: u32,
    pub site: Option<SiteId>,
    pub top_event: Option<EventId>,   // most salient recent event for the glyph hint
    pub field_avg: FieldAverages,     // mean water/heat/etc for overlay tinting
}
```

The summary is recomputed bottom-up only for dirty subtrees, so a quiet region costs nothing to keep current.

### 6.3 Spatial hashing for proximity and line of sight

The checks that dominate a Dwarf-Fortress-style simulation are not pathfinding but per-unit proximity and line-of-sight tests. Those use a spatial hash from tile coordinate to a small bucket of occupant ids, giving average constant-time neighbour queries and cheap per-tick updates as entities move.

```rust
pub struct SpatialHash {
    cells: HashMap<(i32, i32, i16), SmallVec<[StableId; 8]>>,
}
impl SpatialHash {
    pub fn insert(&mut self, id: StableId, pos: Position) { /* ... */ }
    pub fn moved(&mut self, id: StableId, from: Position, to: Position) { /* ... */ }
    pub fn neighbours(&self, pos: Position, radius: i32) -> impl Iterator<Item = StableId> + '_ { /* ... */ todo!() }
}
```

### 6.4 Region partitioning and NUMA

Regions are the boundary at which parallelism, memory locality, and the LOD tier all align. A region is owned by one worker pinned to one CCD; that worker allocates the region's chunk arenas, so the region's hot data lives in that CCD's L3. The high, medium, and aggregate fidelity tiers are assigned per region based on distance from any camera focus and on activity level, so the same partition that gives cache locality also gives the LOD decision.

---

## Part 7: Event Sourcing and History

History is not simulated continuously; it is logged, and then queried. Every meaningful occurrence becomes an immutable event carrying who, where, when, and which entities it concerns. An entity's life story, an artifact's provenance, a building's history, and who-killed-whom are all queries over the log filtered by `StableId`. A consequential event may also emit physical traces into the world (a corpse, spilled fluid, a dropped item, disturbed ground), data-defined entities that carry the event's provenance and afford later perception, which is how a belief about the event can form from evidence long after it happened rather than being known the moment it occurs (Part 9).

### 7.1 The event record

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub u64);

pub struct Event {
    pub id: EventId,
    pub tick: u64,
    pub kind: EventKind,
    pub location: Position,
    pub actors: SmallVec<[StableId; 2]>,    // who did it
    pub subjects: SmallVec<[Subject; 4]>,   // what it concerns (artifact, building, victim, ...)
    pub payload: EventPayload,              // kind-specific data
}

pub enum Subject {
    Person(StableId),
    Artifact(StableId),
    Building(StableId),
    Site(SiteId),
    Culture(StableId),
}

pub enum EventKind {
    Birth, Death, Kill, Forge, Build, Marry, Migrate, Battle,
    FoundSite, ConvertReligion, CoinTerm, ComposeWork, /* ... */
}
```

> Needs research, item R-EVENT in the research backlog; the openness question raised here is the largest of the set, so it is not converted in place. The `/* ... */` already concedes this list was never meant to be closed, and the open question is whether events should have a hardcoded type taxonomy at all. A session should weigh a fully data-defined event-schema registry (a kind is a named record of typed fields, defined in data) against a structural event with a generic typed payload and no kind enum, meaning carried by which entities and attributes it references. The choice shapes the entire history and dramaturg layer (Parts 29, 30), so it is decided deliberately, not guessed.

### 7.2 Log structure and indices

The log is append-only and stored in arena blocks. To answer "show me everything about entity X" without scanning the whole log, a secondary index maps each `StableId` to the list of `EventId`s that reference it, updated on append.

```rust
pub struct EventLog {
    events: Arena<Event>,                       // append-only storage
    by_entity: HashMap<StableId, Vec<EventId>>, // provenance index
    next: u64,
}
impl EventLog {
    pub fn append(&mut self, mut e: Event) -> EventId {
        let id = EventId(self.next); self.next += 1; e.id = id;
        for s in subjects_and_actors(&e) { self.by_entity.entry(s).or_default().push(id); }
        self.events.push(e);
        id
    }
    pub fn history_of(&self, id: StableId) -> impl Iterator<Item = &Event> + '_ {
        self.by_entity.get(&id).into_iter().flatten().map(move |eid| self.get(*eid))
    }
    pub fn get(&self, id: EventId) -> &Event { self.events.get(id.0 as u32) }
}
```

Appends happen only in the single-threaded `Events` phase, so no locking is needed on the log itself.

### 7.3 Persistence and save bloat

Dwarf Fortress saves balloon because full records are kept for every historical figure and event across centuries. The policy here is to keep full append-only records only for promoted entities. Aggregate pools persist as compact statistics. Demoted entities have their fine-grained events compacted: the run of small events is snapshotted into a summary record, and the originals are truncated. Two formats are used: rkyv for the large, mmap-able world snapshot, which deserializes zero-copy and is fastest to load, and bincode (or postcard for size sensitivity) for the streaming event log. JSON is reserved for human-readable legends-style exports, never for canonical state.

```rust
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub seed: u64,
    pub chunks: Vec<ChunkSnapshot>,
    pub promoted: Vec<EntitySnapshot>,  // full records
    pub pools: Vec<PoolStats>,          // statistics, not individuals
    pub registry: RegistrySnapshot,
}
```

---

## Part 8: Entity AI and Decision Making

Decision making is tiered to match the LOD system. High-LOD agents run full utility AI every tick; the rare narratively critical agent additionally runs a planner; medium-LOD agents run coarse utility every N ticks; aggregate pools run no per-agent AI at all and instead generate events stochastically over population statistics.

### 8.1 Utility AI as the default

Each candidate action is scored from zero to one by curves over the agent's drives, traits, and world state, all of them data-defined per race (Parts 20, 40), and the highest scorer is chosen. Utility AI scales well, produces smooth behaviour, needs no hand-authored priority ordering, and is naturally data-oriented because scoring is a pass over SoA columns. It is the cheapest path to good behaviour at thousands of agents.

```rust
// Actions and considerations are defined in data (Part 40), not in code, so a world or a race
// can have actions the engine's authors never enumerated, and an action that satisfies a drive
// exists only where that drive does. A consideration reads a named input (a drive level, a trait,
// a value axis, or a world fact) through a curve, both resolved from the definitions.
pub struct Consideration {
    pub input: InputId,   // a drive level, a trait, a value axis, or a world fact
    pub curve: CurveId,   // a data-defined response curve
}
pub struct ActionDef {
    pub id: ActionId,
    pub considerations: SmallVec<[Consideration; 4]>,
    pub weight: Fixed,
    pub satisfies: SmallVec<[DriveId; 2]>, // which of the agent's drives it reduces, if any
}

fn score_action(a: &AgentView, w: &WorldView, defs: &Defs, def: &ActionDef) -> Fixed {
    let mut s = def.weight;
    for c in &def.considerations {
        let x = defs.curve(c.curve, defs.resolve_input(c.input, a, w)); // each in 0..ONE
        s = fx_mul(s, x);                                               // product of considerations
    }
    s
}

fn choose(a: &AgentView, w: &WorldView, defs: &Defs, actions: &[ActionDef]) -> ActionId {
    actions.iter()
        .map(|d| (d.id, score_action(a, w, defs, d)))
        .max_by_key(|(_, s)| *s)
        .map(|(k, _)| k)
        .unwrap_or(ActionId::IDLE)
}
```

A race's traits feed the curves where it has them: if a race defines a caution trait, agents high in it weight self-protective actions more steeply; if it defines a sociability trait, agents high in it weight company-seeking actions higher. The trait-to-behaviour link is the same numeric mechanism Talk of the Town used for conversation volume, but the axes are the race's own, not a fixed human set.

### 8.2 Planners for the load-bearing few

Agents whose choices carry narrative weight (a leader plotting a war, a legendary smith pursuing a masterwork) can additionally run GOAP, an A* search over actions that assembles multi-step plans, or HTN, which scales better than flat GOAP because methods constrain the search. These are expensive per agent and are reserved for the high-LOD, narratively critical handful. The output of a planner is a queued sequence of activities that the utility layer can still interrupt if a need becomes urgent.

### 8.3 LOD mapping

High-LOD agents run the full utility pass every tick, plus a planner if flagged. Medium-LOD agents run a reduced consideration set every N ticks and otherwise coast on their last decision. Aggregate pools run no per-agent logic; a pool-level system samples its statistics each period and emits events (a birth, a death, a feud, a small migration) with probabilities derived from the pool's composition, promoting an individual only when an event is load-bearing enough to warrant it. The stochastic events a pool emits are demographic and social, a birth, a death, a feud, a migration. A collective decision is a different thing and is not a draw over pool composition: undertaking a public work, going to war, or enacting a law is an act of the settlement's evolved governance and its legitimacy (Part 36), so even at aggregate fidelity it is attributed to and constrained by that authority rather than conjured from the ether. Doing this cheaply at the aggregate tier, and consistently with the detailed tier that would deliberate it agent by agent (Parts 42, 43), is resolved by the tier-consistency mechanism (R-TIER-CONSIST, Part 54): a collective undertaking is emitted through the aggregate representation of the settlement's emergent governing institution and its norms (Part 36), a coarse projection of the same authority the detailed tier deliberates through, rather than a draw over pool composition.

---

## Part 9: Knowledge, Belief, and Folklore (the differentiator)

This is the subsystem that exceeds Dwarf Fortress, which spreads essentially no false rumours and does not model memory fallibility. Here, an individual's belief about a past event can differ from the truth, distortion accumulates at every transmission, and folklore forms by accumulated copying error. The design layers directly on the event log and draws its architecture from Talk of the Town's belief facets and Gossamer's four-phase gossip loop.

### 9.1 Ground truth versus belief

The event log is ground truth, deterministic and immutable. A belief is a lossy projection of an event into one entity's mind, and it can be wrong. The two are never conflated: a legends view can show both what truly happened and what a given character believes happened, side by side, by reading the log against that character's belief store.

### 9.2 The belief facet

A belief store is a per-entity database of facets. Each facet is one entity's potentially-false belief about one attribute of one subject. The facet carries its value, a pointer to the prior belief it replaced (a perfect personal history), pointers to the source facets that spawned it (full provenance, so any rumour's trajectory can be reconstructed), accumulating evidence, a strength that decays with time, and the truth at observation for accuracy tracking.

```rust
pub struct BeliefStore {
    facets: Slab<BeliefFacet>,
    by_subject: HashMap<(StableId, AttrKind), FacetId>, // newest belief per attribute
}

#[derive(Clone, Copy)] pub struct FacetId(u32);

pub struct BeliefFacet {
    pub subject: StableId,        // who or what this is about
    pub attr: AttrKind,          // which attribute (hair colour, whereabouts, who-killed-whom...)
    pub value: AttrValue,        // the believed value, possibly false
    pub truth_at_obs: AttrValue, // ground truth when first formed, for accuracy metrics
    pub predecessor: Option<FacetId>,        // the belief this one replaced
    pub parents: SmallVec<[FacetSource; 2]>, // sources that produced this belief
    pub evidence: SmallVec<[Evidence; 4]>,   // accumulating support
    pub strength: Fixed,                     // decays over time; sum of evidence strengths
    pub formed_tick: u64,
    pub last_reinforced: u64,
}

pub struct FacetSource { pub from: StableId, pub their_facet: FacetId, pub hop: u16 }
pub enum Evidence { Observed, Told { by: StableId }, Inferred, Confabulated,
                    Absent { since: u64 } } // Absent: formed from absence of an expected observation (9.11)
```

The `hop` count on a source is what lets folklore drift be measured: a belief seven hops from the witness has passed through seven potential distortions.

### 9.3 The eleven knowledge phenomena

The store supports the same phenomena Talk of the Town modelled, grouped into five categories. Origination covers observation (a witness forms a true facet), reflection (forming a belief about oneself), transference (mistakenly attributing one person's attribute to another), confabulation (inventing a plausible value), lying (asserting a value one knows is false), and implantation (a lie that takes root in the hearer). Reinforcement is declaration: retelling strengthens the teller's own belief, so a character who repeats a lie often enough can come to believe it. Propagation is statement and eavesdropping. Deterioration is mutation. Termination is forgetting. Each is a function over the store.

### 9.4 The belief mutation graph

Mutation is not random noise; it is biased toward plausible neighbours. An authored transition table per attribute gives, for each value, a weighted distribution over what it can mutate into (hair "brown" mutates to "black" or "red" far more readily than to "white"). Confabulation samples instead from the population's actual distribution of that attribute, so an invented hair colour matches the demographics. The probability that a given facet deteriorates on a given tick is a function of the holder's memory attribute, the facet type (a whereabouts belief mutates more readily than a first-name belief), and the current strength (weaker beliefs deteriorate more easily).

```rust
pub struct MutationGraph { tables: HashMap<AttrKind, TransitionTable> }
pub struct TransitionTable { rows: HashMap<AttrValue, Vec<(AttrValue, Fixed)>> }

fn maybe_mutate(f: &mut BeliefFacet, g: &MutationGraph, mem: Fixed, rng: Rng, ctr: u64) {
    let p_decay = deterioration_prob(mem, f.attr, f.strength); // 0..ONE
    if rng.unit_fixed(ctr) < p_decay {
        if let Some(row) = g.tables.get(&f.attr).and_then(|t| t.rows.get(&f.value)) {
            f.value = weighted_pick(row, rng, ctr + 1);
            f.strength = fx_mul(f.strength, /* weaken */ ONE / 2);
        }
    }
}
```

### 9.5 The four-phase gossip loop

Following Gossamer, gossip is four phases, and distortion can enter at two of them, so every witness and every relay is a potential distortion point.

Witness determines who perceived an event, possibly mutates their perception on intake (a misunderstanding at the source), and decides whether and how strongly to record it. Reflection assembles an entity's remembered events into communicable microstories. Propagation decides which microstories an entity shares with whom, possibly mutates the underlying belief in transmission, and sets the recipient's initial belief strength. Decay reduces memory weights over time and culls facets below a threshold, bounding memory growth and prioritizing recent events.

The probability of faithful recording, retention, and transmission is gated by salience, modelled as familiarity: the normalized count of references to a subject within the holder's own store. Famous subjects are recorded and transmitted more faithfully, which is how reputations come to precede characters.

```rust
fn gossip_phase(world: &WorldView, stores: &mut KnowledgeArena,
                pairs: &[(StableId, StableId)], log: &EventLog, mgraph: &MutationGraph, t: u64) {
    // pairs are (speaker, listener) chosen this tick from co-located, socially linked agents.
    // Parallelized over disjoint speaker partitions so write sets do not overlap.
    for &(speaker, listener) in pairs {
        let micro = reflect(stores.store(handle(speaker)));        // pick something to tell
        if let Some(story) = micro {
            let distorted = transmit(story, mgraph, salience(stores, speaker, story.subject));
            integrate(stores.store_mut(handle(listener)), distorted, speaker, t);
        }
    }
}
```

The partition that makes the parallel form safe: speakers are grouped so that no two speakers in the same parallel batch write to the same listener's store. Cross-batch conflicts are resolved by processing conflicting pairs in a later sub-batch, keeping each batch's write sets disjoint.

### 9.6 Lies, and how beliefs reconnect to truth

A lie is an injected belief whose value diverges from ground truth while its source and time metadata are preserved, so the provenance chain still records who started it and when. Because each facet keeps its predecessor and parents, the true trajectory of any rumour can be reconstructed even after many mutations, which is what powers a legends view that shows the real event, the chain of tellers, and the drifted version each generation believed. Belief strength plus decay gives confidence-weighted, time-decaying beliefs that can oscillate when contradictory evidence arrives.

### 9.7 Scale and the LOD restriction

Talk of the Town spent roughly a minute per knowledge timestep for only 300 to 500 fully-modelled characters, each holding hundreds of facets. Full belief modelling is therefore restricted to promoted entities. The masses spread rumours statistically at the aggregate-pool level: a pool tracks a small set of prevailing beliefs and their strengths, drifts them with the same mutation rules applied once per pool rather than per person, and only instantiates an individual belief store when a member is promoted. This keeps the deep simulation affordable while preserving emergent folklore at population scale.

### 9.8 Belief forms from evidence, about any question

Part 9 so far moves a belief between minds and lets it drift. It does not yet say how a belief about the world forms in the first place, and the perfect-crime case shows why that matters: a hidden death is ground truth in the event log, but no mind should believe it until a mind perceives a trace of it or infers it. The settled half was propagation; this is the open half, origination from evidence, and it is built as one general engine rather than a murder system. A mind forms a belief about a question, where a question is a subject and an attribute with a set of candidate values (a hypothesis frame), by accumulating evidence from four sources, observation of physical traces, testimony, inference, and the absence of an expected observation, until one hypothesis is both well enough supported and clearly better than its rivals to commit. Who killed whom is one instance of this engine. The same engine answers where the good ore lies from surface signs and assays, who fathered a child from resemblance and presence, how strong an enemy host is from observed banners and scout reports, whether a trader is reliable from remembered dealings, where a lost caravan went, and what causes a blight from observations that correlate it with the wet season or the eastern wind, which is the same accumulation toward a causal hypothesis that seeds emergent science (Part 41, R-DEEPTECH-SCIENCE). The mechanism is fixed Rust; the trace kinds, the evidence weights, the hypothesis frames, and the absence schedules are all data registries (Part 40), siblings of the belief mutation graph (9.4), so the engine is general and nothing about murder is special-cased.

### 9.9 Physical traces

A trace is a world-placed, perceptible consequence of an event, ground truth that affords observation rather than a belief. When a consequential physical event occurs, its handler emits one or more traces into the focus-scale local simulation (Parts 42, 43), each linked to the event that produced it (Part 7 provenance). What kinds of trace exist, how perceptible each is and how that perceptibility changes over time, and which belief each proposes when perceived, are all data (the trace-kind registry, Part 40), so a corpse, a pool of spilled fluid, a wound on a body, a dropped weapon, and disturbed earth are entries alongside a felled tree, a footprint, cooling ash, a flood line, or a worn tool. This is the engine analogue of the forensic principle that a contact can leave a trace, with the rider that a trace must persist and be perceived to matter.

```rust
pub struct Trace {
    pub id: StableId,
    pub kind: TraceKindId,           // a data registry entry (Part 40), not a fixed enum
    pub pos: Position,
    pub origin_event: EventId,        // links to the ground-truth event (Part 7)
    pub created_tick: u64,
    pub salience: SalienceCurve,      // perceptibility over time: a corpse decays, a fluid dries
    pub concealment: Fixed,           // 0..ONE perceptibility multiplier set by concealment (9.12)
}

// What perceiving a trace of this kind proposes as a belief; pure data.
pub struct TraceKindDef {
    pub id: TraceKindId,
    pub implies: SmallVec<[TraceImplication; 2]>, // (attr, value, weight) facets to propose
    pub default_salience: SalienceCurve,
}
pub struct TraceImplication { pub attr: AttrKind, pub value: AttrValue, pub weight: Fixed }
```

Perceiving a trace forms an observed belief through the existing Witness phase (9.5): an agent within a trace's current perceptibility rolls perception against the trace's salience scaled by its own acuity, and on success writes a `BeliefFacet` with `Evidence::Observed`, the implied subject, attribute, and value, `truth_at_obs` read from the log, and an initial strength from the implication weight. Perception is fallible exactly as the witness step already distorts, so a misperceived value can mutate on intake through the mutation graph (9.4). Because salience is evaluated only when an agent is co-located, there is no per-trace per-tick cost, and a trace that has decayed or been concealed simply fails the roll.

### 9.10 The inference engine

An agent combines evidence into an inferred belief by a deterministic integer rule. For a question, the agent holds a small frame of candidate hypotheses (drawn from the attribute's value space or a data-defined frame, and always including an explicit unknown), and each piece of evidence adds a signed integer weight, read from a data weight table, to each hypothesis's running total. The totals are kept as log-odds in fixed point, which makes combination a plain integer addition that is associative and independent of the order evidence arrives in, and each total is clamped to a reserved bound so no finite evidence reaches absolute certainty. The agent commits an inferred facet for the leading hypothesis when its total clears a reserved commit threshold and beats the runner-up by a reserved margin, the second test guarding against committing to the best of a poor set; if nothing clears, the belief stays unknown, which is how the engine represents ignorance without the unstable normalisation that a Dempster-Shafer combination would require on the authoritative path. The committed facet records every trace and facet that fed it, so a wrong inference is still a fully traceable belief.

```rust
pub struct InferenceFrame {
    pub subject: StableId,
    pub attr: AttrKind,
    pub hyps: SmallVec<[AttrValue; 4]>,   // candidate values, including an explicit Unknown
    pub logodds: SmallVec<[Fixed; 4]>,    // additive, order-independent, clamped to +/- clamp
    pub support: SmallVec<[EvidenceRef; 8]>, // provenance: what fed each total
    pub clamp: Fixed,             // RESERVED: max certainty
    pub commit_threshold: Fixed,  // RESERVED
    pub margin: Fixed,            // RESERVED: lead over runner-up to commit
}
pub struct EvidenceRef { pub from: StableId, pub weight: Fixed, pub toward: u8 }
```

The same engine yields different beliefs in different minds because two existing systems parametrise it: a mind's reasoning acuity (the genome phenotype, Part 25) scales the weight it extracts from evidence and can lower its commit threshold, so a sharper mind needs less to conclude but is not thereby more correct, since it can commit a confident wrong inference sooner; and a mind's epistemic stance (Part 28) sets the prior and the margin, so a skeptic starts near unknown and demands a wider lead while a credulous mind commits readily. Once committed, an inferred facet is an ordinary belief: it propagates and mutates through the gossip loop like any other, carrying its provenance so the legends view can show a drifted inference beside the truth, and it is defeasible, because later evidence of the opposite sign is simply added and can carry the total back across the threshold so a new facet supersedes the old. This is the non-monotonic, revisable belief the design wants, realised as integer accumulation. The only step that is not exact integer arithmetic is converting a total to a displayed confidence, which uses a precomputed integer lookup and is needed only for the view or for a single threshold test that is instead done directly in log-odds space, so it never touches the authoritative path. The normalised Dempster-Shafer rule is rejected for the canonical combiner because its conflict normalisation divides by a data-dependent near-zero quantity that is neither stable nor exact in fixed point; an unnormalised conjunctive form is held in reserve as an option for a specific frame that needs richer ignorance semantics and can afford it.

### 9.11 Inference from absence

The absence of an expected observation is itself evidence, and it is the one piece here that is architecturally novel, because a non-event drives a belief. A subject that minds refer to carries a last-seen tick, and whenever it is observed that tick updates and a single re-check is scheduled into an ordered timer queue at the last-seen tick plus a window; there is no scanning of subjects, the queue simply pops due checks in canonical order, and a subject seen again before its check makes the stale check a cheap no-op. The window scales with how visible and socially embedded the subject normally is, computed from the same familiarity signal the gossip loop already uses, so a prominent figure whose absence is loud is missed in days while an obscure one is missed in years or never, which is the closed-world intuition that absence of an expected sighting is informative in proportion to how expected it was. When a check fires and the subject has been unseen past a schedule's threshold, an escalation schedule, which is data, advances the belief and writes an inferred facet of low and decaying strength: the death schedule runs from whereabouts-unknown to missing to presumed-dead, and other schedules express an overdue caravan presumed lost, a silent ally presumed estranged, or a vein that has stopped yielding presumed worked out. The presumption is deliberately fragile and defeasible: a later sighting contributes heavy opposite-sign evidence and collapses it, while a found body upgrades the same belief from inferred to observed. The formal model is negation as failure under a locally closed world, the people assuming what they cannot observe, which is exactly the rebuttable presumption that the law of declaring a person dead in absentia encodes, and the windows are reserved with that waiting-period spread as their basis.

```rust
pub struct AbsenceTracker {
    pub subject: StableId,
    pub last_seen: u64,
    pub schedule: AbsenceScheduleId, // data (Part 40): the escalation states and thresholds
    pub state: u8,                   // index into the schedule's states
    pub window: u64,                 // visibility-scaled (RESERVED scaling)
}
// ordered (due_tick, StableId) queue; no per-tick per-subject scan
```

### 9.12 Concealment

An agent can suppress the formation of a belief by acting on the traces that would seed it, which is distinct from lying. Lying (Part 37) asserts a false value into a belief that is being communicated; concealment denies the world the input that would form a belief at all, by removing, relocating, diminishing, or disguising a trace, a small set of trace operations any agent can apply to any trace and whose use is utility-driven from data. A hidden corpse, a cleaned fluid pool, a buried cache, a covered track, a disposed weapon all lower or zero a trace's perceptibility so the perception roll of 9.9 fails and the inference engine of 9.10 is starved. Partial concealment leaves a faint residual that a skilled searcher can still find at reduced odds, which is the residue that makes a perfect crime imperfect, and concealment interoperates with absence, since a fully hidden change leaves only the slow absence path to belief, which is why a concealed death becomes a missing-person presumption rather than a known murder. Concealment is itself witnessable: the act emits its own event and can leave its own trace, so an agent who sees another bury something forms an observed belief that feeds the inference engine, the seen-burying-something clue, and an agent who both conceals a trace and lies about it can be undone by either path.

### 9.13 Inquiry and investigation

Resolving a question on purpose is a goal-directed evidence-gathering behaviour in the utility AI (Part 8) that any agent can run when it holds a question it is motivated to answer, and the motive is data: a grievance, a duty, an economic interest, curiosity, or suspicion. The behaviour reuses primitives the engine already has: move to the places the answer might live so perception runs and manufactures observed facets, solicit testimony from others in a directed gossip interaction that yields told evidence, and combine it all through the inference engine until a hypothesis commits or the trail goes cold. An investigator's skill and acuity raise perception on faint traces, raise what interviews yield, and sharpen the margin test. Failure is first-class and intended: too little evidence leaves a cold case, and a misleading set of traces or a grudge-driven false testimony can cross the threshold and commit a wrong conclusion, a false accusation that then spreads and mutates as an ordinary belief, traceable through its provenance and correctable by later evidence. A kin seeking a killer or a legal authority (Part 36) prosecuting one is one instance; a prospector seeking a lode, a scout assessing a host, a merchant vetting a partner, and a scholar pursuing the cause of a blight are the same behaviour pointed at a different question.

### 9.14 The aggregate tier

For the masses none of this runs per individual. A demographic event such as a death is canonical at its pool the moment it happens (Part 8), but the pool's knowledge of it is not instant: each pool carries, per prevailing belief, a knowledge level that rises by integer diffusion over time as a function of distance from the origin, advanced once per pool rather than per agent, so a death in a distant province becomes known there only after a delay set by distance and a reserved diffusion rate. Absence at this tier is just another diffusing belief or is deferred until a subject is load-bearing enough to promote, and concealment degrades to a reserved factor that slows the diffusion or stalls it below saturation. On promotion a member instantiates an individual belief store seeded from the pool's prevailing beliefs at their current knowledge level mapped to facet strength, so a promoted mind knows what its pool knew, and on demotion its beliefs fold back into the pool statistics; the level-to-strength mapping is reserved and is specified by the tier-consistency mechanism (R-TIER-CONSIST, Part 54) as a monotone fixed-point curve with a counter-seeded per-mind dispersion conditioned so the population mean reconstructs the pool knowledge level, the id-ordered mean folding back on demotion to conserve total belief mass.

### 9.15 Determinism

Every authoritative step is integer and fixed-point. Evidence weights, the log-odds totals and their clamp, thresholds, margins, decay, and the aggregate diffusion are all fixed-point add, compare, and shift, with no canonical division; the optional unnormalised conjunctive combiner is multiply-only with a fixed shift. The one inexact conversion, a total to a displayed confidence, uses a precomputed integer lookup and stays out of the authoritative path, since the threshold test is done in log-odds space. The absence check is an ordered queue keyed by due tick and stable id, with stale checks as deterministic no-ops, so it needs no per-tick scan and does not depend on iteration order. Every roll, a perception success, a low-acuity jump to a conclusion below threshold, a confabulation draw, keys counter-based RNG on a hash of the master seed, the perceiver, the object, the tick, and a phase, and all co-located observers, contributing evidence, and pool updates are enumerated in stable-id order before they combine, so the result is bit-identical across machines and thread counts.

> Decided and reserved. The mechanism is settled and signed off, and it is general rather than a whodunit system: a belief forms about any question, a subject and attribute with a hypothesis frame, by accumulating evidence from observation of traces, testimony, inference, and absence, and who-killed-whom is one configured instance alongside prospecting, parentage, scouting, trade trust, lost caravans, and the causal inquiry that seeds emergent science. A trace is a perceptible, decaying, event-linked world entity whose kind, perceptibility, and implied belief are data; perception writes observed facets through the existing Witness phase. The inference rule is an integer log-odds accumulator, additive and order-independent and clamped, with an explicit unknown and a best-and-clearly-better commit test, parametrised by a mind's genome acuity and epistemic stance, committing defeasible inferred facets that propagate and revise like any belief; the normalised Dempster-Shafer rule is rejected on the authoritative path for its unstable conflict normalisation, with an unnormalised conjunctive form reserved as an option. Absence is first-class evidence detected by a lazy last-seen tick and an ordered timer queue with a visibility-scaled window, escalating through a data-defined schedule (death is one schedule) and reconciled by later observation. Concealment is a utility-driven suppression of trace perceptibility, distinct from lying, partial and itself witnessable. Investigation is a utility goal any motivated agent runs, reusing move, perceive, interview, and combine, with false conclusions an intended emergent outcome. The aggregate tier diffuses knowledge with delay and seeds promoted minds for tier consistency. The mechanism is fixed Rust; the trace-kind registry, the evidence-weight tables, the hypothesis frames, and the absence schedules are data (Part 40), siblings of the mutation graph, so the engine generalises to any evidence-gathering task by data alone. What is reserved for your calibration, surfaced rather than fabricated, with its basis given: the per-implication evidence weights (basis: the weight-of-evidence of each observation type, a fresh corpse far heavier than a stale stain); the commit threshold and the runner-up margin (basis: the intended balance of false conclusions against cold cases); the log-odds clamp (basis: the maximum admissible certainty); the trace salience and decay curves (basis: real decay and fading timescales at the tick rate); the absence windows and their visibility scaling (basis: the spread of death-in-absentia waiting periods mapped onto the prominence axis); the presumption strengths and decay; the concealment perceptibility multipliers and their skill and time costs; the genome-acuity and epistemic-stance couplings; and the aggregate diffusion rate, the concealment suppression factor, and the knowledge-level-to-strength mapping. The reserved list is in the audit log. The honest limits stand: the bounded log-odds frame approximates a full forensic network and cannot carry rich dependencies between traces; the locally closed world can let a community confidently presume a living traveller dead, which is realistic and intended; and wrongful conclusions from misleading traces or grudge-driven testimony are a designed emergent feature, not a defect.

---

## Part 10: Culture and Religion

Cultures and religions are first-class entities, not authored constants. Individuals reference them through membership edges, learn from them, pass them to their children, and in aggregate define them. The feedback loop between individual belief and group definition is what makes customs, practices, and religions emerge, diverge, and mutate across centuries.

### 10.1 Culture and religion as entities

```rust
pub struct Culture {
    pub id: StableId,
    pub values: SmallVec<[(ValueAxisId, i8); 16]>, // profile over the world's value axes (Parts 20, 40)
    pub customs: Vec<Custom>,              // practices, taboos, rites
    pub aesthetic: AestheticMarkers,       // naming style, motifs, palette for glyph hints
    pub language: LanguageMarkers,         // affects coined terms and names
    pub member_count: u32,
    pub parent: Option<StableId>,          // cultural lineage when one splits from another
}

pub struct Religion {
    pub id: StableId,
    pub deities: SmallVec<[Deity; 4]>,
    pub tenets: Vec<Tenet>,                // moral rules, weighted
    pub rites: Vec<Rite>,
    pub origin_myth: Option<EventId>,      // the real (or believed) founding event
    pub follower_count: u32,
    pub parent: Option<StableId>,          // schism lineage
}
```

Membership is a relationship edge. hecs has no native relationships, so a thin adjacency store maps each membership kind to its edges; this is the one place where flecs's native relationships would have saved work, weighed earlier against the customization ceiling.

```rust
pub enum Relation { MemberOf, Follows, ChildOf, Owns, Allegiance, Rivalry }

pub struct RelationStore {
    // forward and reverse adjacency for O(1) "who belongs to culture C"
    edges: HashMap<(Relation, StableId), SmallVec<[StableId; 8]>>, // target -> sources
    of:    HashMap<(Relation, StableId), StableId>,                // source -> target (exclusive kinds)
}
```

> Needs research, item R-RELATION in the research backlog. `Relation` is a closed set of edge kinds, and relationship types are world content a mod or an exotic society could need to extend (a patronage tie, a blood debt, a binding oath). This couples to the unified dynamic-graph substrate that is already a flagged open foundation (Part 58), so it is researched together with that rather than converted alone.

### 10.2 The three transmission flows

Learning from culture and environment runs each relevant period: an agent samples its culture's values and customs and writes them, probabilistically and weighted by the agent's own traits, age, and exposure, into the agent's facets and belief store. A young, high-openness agent absorbs more; an old, low-openness agent barely shifts.

```rust
fn enculturate(agent: AttributeRef, attrs: &mut AttributeStore, culture: &Culture, rng: Rng, t: u64) {
    let plasticity = plasticity_of(attrs, agent /* a plasticity trait, if the race has one, plus age */);
    // pull the agent's value level toward the culture's on each value axis they share. This straight
    // per-axis step is exact when the race's value structure is Independent; for a structured space the
    // pull should follow the structure's geodesic (Part 21), which is a reserved choice (R-VALUE-METRIC).
    for (axis, culture_level) in &culture.values {
        if let Some(agent_level) = attrs.value_mut(agent, *axis) {
            let pull = *culture_level as i32 - *agent_level as i32;
            let step = fx_mul(fx_from_int(pull), plasticity);
            *agent_level = (*agent_level as i32 + fx_to_int(step)).clamp(-128, 127) as i8;
        }
    }
}
```

Parent to child transmission runs at birth or maturation: the child's belief store and facets are seeded from the parents', as a lossy copy through the same per-hop distortion machinery, so children inherit a drifted version of their parents' worldview rather than a perfect copy. Over generations this is how oral tradition mutates.

Person to person transmission is the gossip loop of Part 9, which already carries cultural and religious assertions as ordinary beliefs about subjects.

### 10.3 The emergence feedback loop

A culture's current definition is the aggregate of its members. Periodically (not every tick, this is a slow cadence) the culture's value profile and prevailing customs are recomputed from the distribution of its members' facets and beliefs. New members then learn from that updated definition. Because each member's beliefs drift through accumulated copying error, the aggregate drifts with them, and the culture as a whole evolves across generations with no authored change.

```rust
fn recompute_culture(culture: &mut Culture, members: &[StableId], world: &WorldView) {
    // the culture's profile is the mean (or mode) of its members' levels on each value axis it has;
    // customs whose support among members falls below a threshold fade, and emergent practices
    // that gain support are added.
    for (axis, level) in culture.values.iter_mut() {
        *level = mean_value(members, *axis, world);
    }
    update_customs_from_support(culture, members, world);
}
```

Divergence and schism fall out of the same machinery. When a subpopulation becomes isolated (geographic separation tracked through the spatial hierarchy) or when its belief distribution drifts far enough from the parent's, the subpopulation recomputes as a distinct culture or religion entity, with `parent` set to record the lineage. Religions splinter exactly this way: a faction whose tenet weights diverge past a threshold becomes a new `Religion` with the original as parent, and the schism itself is logged as an event, becoming part of the history both groups will later remember (and misremember).

---

## Part 11: LOD Promotion and Demotion

The boundary between aggregate pools and full entities is crossed constantly, and keeping it coherent is the machinery that makes "a fully detailed world" affordable. The stable-id registry (Part 2) is what makes references survive the crossing.

### 11.1 Aggregate pools

A pool represents many anonymous individuals as statistics: counts by role, distributions over age, personality, and skill, a small set of prevailing beliefs, and membership in a culture and religion. A pool runs no per-agent logic; it emits events stochastically and tracks only enough to generate a coherent individual on demand.

```rust
pub struct Pool {
    pub id: PoolId,
    pub culture: StableId,
    pub religion: Option<StableId>,
    pub count: u32,
    pub age_dist: Distribution,
    pub personality_dist: PersonalityDistribution,
    pub skill_dist: SkillDistribution,
    pub prevailing_beliefs: SmallVec<[(BeliefKey, Fixed); 8]>,
    pub seed: u64,                 // base seed for deterministic member generation
}
```

### 11.2 Promotion

When a pool member becomes load-bearing (it kills a noble, forges an artifact, founds a site), it is promoted: a full entity is instantiated, its name, personality, skills, and backstory are generated deterministically from `(pool.seed, slot_id)` and the triggering event, the pool count is decremented, and the new entity is persisted with a back-reference to its pool. Determinism of generation means a re-promotion of the same slot reproduces the same individual.

```rust
fn promote(pool: &mut Pool, slot: u32, trigger: &Event, reg: &mut Registry,
           world: &mut hecs::World, knowledge: &mut KnowledgeArena) -> StableId {
    let ctx = SeedCtx::from(pool.seed, slot);
    let (id, _entity) = spawn_person(world, reg, knowledge, ctx);
    // backstory consistent with pool stats AND the triggering event:
    generate_backstory(id, &ctx, pool, trigger, knowledge);
    pool.count -= 1;
    id
}
```

The camera drives promotion as well: when the viewer zooms into a region (Part 14), that region's near-camera pools promote a sample of members to high LOD so there is something detailed to watch, and demote them again when the camera leaves.

### 11.3 Demotion

When a promoted entity is no longer load-bearing and no camera is near, it is demoted: a compacted life-summary record and its key relationships are written to the persistent log, its fine-grained recent events are compacted into that summary, its belief store is summarized into the pool's prevailing beliefs and then freed, and its hot ECS components are despawned. The append-only history is never deleted; only the live representation goes away. The registry marks the id `Pooled` or `Retired`, so any log reference or relationship edge still resolves.

```rust
fn demote(id: StableId, reg: &mut Registry, world: &mut hecs::World,
          log: &mut EventLog, knowledge: &mut KnowledgeArena, pools: &mut PoolSet) {
    write_life_summary(id, world, log);                 // permanent record
    summarize_beliefs_into_pool(id, knowledge, pools);  // fold beliefs into stats
    knowledge.free_store(store_of(id, world));
    despawn(world, id);
    reg.set_location(id, EntityLocation::Pooled { /* ... */ pool: PoolId(0), slot: 0 });
}
```

### 11.4 Coherence rule

Every cross-entity reference anywhere in the engine uses `StableId`, never `hecs::Entity`. This single rule is what allows promotion and demotion to churn freely while the event log, belief provenance, relationship edges, and save files all remain valid.

---

## Part 12: Worldgen Pipeline

Worldgen mirrors Dwarf Fortress's structure and is fully reproducible from a seed. It runs in three passes, the first two on the GPU and the third as an abstract statistical simulation on the CPU.

Terrain genesis produces elevation from layered noise, then rainfall, temperature, drainage, and volcanism fields, then a biome classification derived from elevation, temperature, rainfall, and drainage. This is per-cell and embarrassingly parallel, so it runs as GPU kernels writing the resident field buffers the simulation will later use.

Erosion and hydrology run GPU hydraulic and thermal erosion and carve rivers from high ground to oceans and lakes, shaping the terrain the civilizations will inhabit.

The third pass does not build civilizations; it populates the natural world and lights the spark. It seeds flora and fauna across the biomes (Parts 16, 17) and places scattered proto-populations of each race at the dawn of sentience (Part 28), each carrying only its intrinsic capabilities and intrinsic beliefs. There are no starting tribes with histories, no founded sites, no artifacts, and no language yet; those are things the races will develop. From this primordial state the historical simulation runs forward at aggregate LOD, the races growing language, technique, society, belief, and eventually civilizations through the emergent systems (Parts 23, 33, 36), recording every occurrence into the event log, with detailed local simulation beginning only where the observer later focuses. Because every step is seeded and integer-valued, the same seed yields the same world and the same emergent history on any machine, the seed-to-identical-world property applied to a world that writes its own past.

```rust
fn worldgen<R: Runtime>(seed: u64, dims: GridDims, client: &ComputeClient<R::Server, R::Channel>) -> World {
    let fields = generate_terrain(seed, dims, gpu);   // GPU noise -> elevation, climate
    erode_and_carve(&fields, gpu);                    // GPU erosion + rivers
    let mut world = World::from_fields(fields);
    seed_ecology(&mut world, seed);                   // flora and fauna across biomes (Parts 16, 17)
    seed_dawn_populations(&mut world, seed);          // proto-races at the dawn of sentience (Part 28)
    simulate_emergent_history(&mut world, seed);      // history grows from nothing; logs all events
    world
}
```

---

## Part 13: Pathfinding

The first move is to reframe the problem. Profiling of Dwarf Fortress reported by its developers shows pathfinding is only a small share of processing, roughly three to six percent of a tick, while per-unit turn processing dominates (over sixty percent in large forts) and line-of-sight and proximity checks are a larger algorithmic share than pathfinding. The structural fix is therefore the LOD and aggregate-pool design (fewer active per-tick units), not a fancier path algorithm. Profile before optimizing pathfinding; if it is under ten percent of tick time, spend the effort on turn processing and line-of-sight instead, which is why the spatial hash of Part 6 exists.

With that established, movement uses a hierarchy. Flow fields, solved as a parallel eikonal wavefront on the GPU, give a vector field that any number of agents sharing a destination sample at constant per-agent cost, which is ideal when crowds share goals; the cost is memory and that a single agent on a huge map is served less efficiently than by A*. HPA*, hierarchical A* over a coarse graph of region portals, paths at the abstract level and then refines, which scales to huge, changing worlds and is the canonical large-map answer. The two combine: HPA* for the global route across regions, flow-field tiles for local crowd movement within them. The world is grid-based and mutable (digging and construction change it constantly), so a grid plus an HPA* hierarchy is more practical than a navmesh, which is costlier to update under frequent terrain change.

```rust
pub enum PathRequest {
    Crowd { goal: Position },        // served by a shared flow field
    Individual { from: Position, to: Position }, // HPA* refine, for the rare lone long path
}

pub struct FlowField { dir: Vec<Dir8>, /* per tile, sampled O(1) by agents */ }
fn solve_flow_field<R: Runtime>(goal: Position, region: RegionId, client: &ComputeClient<R::Server, R::Channel>) -> FlowField { todo!() }
```

---

## Part 14: Rendering and the Multi-Scale Zoom System

The visual target is one step up from Dwarf Fortress ASCII: simple glyphs on a grid, where every glyph stands for a thing in the world. The defining feature is scale. The top scale is the overworld, where one glyph summarizes an entire region of terrain. Zooming in expands each cell into its children, revealing finer terrain, then sites, then buildings, and at the bottom individual people, creatures, and plants. The renderer is thin, because the GPU is busy with simulation compute, and it shares one wgpu device with the compute backend.

### 14.1 The renderer is a view over the spatial hierarchy

The crucial design choice, stated as Principle 6, is that the zoom levels are the quadtree levels are the LOD tiers. The renderer does not maintain its own scene. It reads the quadtree (Part 6) at a depth determined by the current zoom and draws exactly one glyph per visible node, using that node's summary. Descending the zoom descends the tree; the glyph at each level is a summary of the subtree beneath it.

```rust
pub struct Camera {
    pub center: WorldPos,   // where we are looking, in world tiles
    pub zoom_depth: u8,     // 0 = whole world (root), deeper = finer
    pub viewport: ScreenDims,
}

/// Returns one draw item per visible cell at the camera's zoom depth.
fn build_glyph_frame(tree: &QuadTree, world: &World, cam: &Camera) -> Vec<GlyphInstance> {
    let nodes = tree.visible_nodes_at_depth(cam.center, cam.zoom_depth, cam.viewport);
    nodes.iter().map(|node| glyph_for_node(node, world, cam.zoom_depth)).collect()
}
```

### 14.2 What each scale draws

At shallow depth, a node is a region or chunk summary, and its glyph is the dominant terrain of the subtree, tinted by the field averages (blue where water depth is high, red where heat is high), with an overlaid marker if the subtree contains a site. At medium depth, individual sites and their broad layout appear: a settlement glyph, walls, roads. At deeper depth, individual buildings and tiles appear. At the deepest depth, the leaf is a single tile and the glyph is whatever occupies it: a person, a creature, a plant, an item, or the bare terrain. The mapping from a thing to a glyph and colour is a small table.

```rust
pub struct GlyphInstance {
    pub cell: [i32; 2],     // grid cell on screen
    pub glyph: u16,         // index into the glyph atlas
    pub fg: [u8; 4],        // foreground colour
    pub bg: [u8; 4],        // background colour (used for field overlays)
}

fn glyph_for_node(node: &QuadNode, world: &World, depth: u8) -> GlyphInstance {
    if node.is_leaf() {
        // deepest scale: draw the actual occupant or terrain
        match leaf_occupant(node, world) {
            Occupant::Person(id)   => glyph_for_person(id, world),
            Occupant::Creature(id) => glyph_for_creature(id, world),
            Occupant::Plant(p)     => glyph_for_plant(p),
            Occupant::Item(it)     => glyph_for_item(it),
            Occupant::None         => glyph_for_terrain(node.summary.dominant_terrain),
        }
    } else {
        // summary scale: dominant terrain tinted by field averages, site marker if present
        let mut g = glyph_for_terrain(node.summary.dominant_terrain);
        g.bg = field_overlay_colour(node.summary.field_avg);
        if node.summary.site.is_some() { g.glyph = SITE_GLYPH; }
        g
    }
}
```

### 14.3 Rendering tech: instanced glyph quads

Glyphs are drawn with a single instanced draw call. The glyph atlas is one texture holding every glyph (a curated tileset, the "one step up from ASCII"). Each `GlyphInstance` becomes one instance of a unit quad; the vertex shader places it on the grid and the fragment shader samples the atlas and applies foreground and background colour. Tens of thousands of glyphs draw in one call, which is why the renderer can stay cheap.

```rust
pub struct GlyphRenderer {
    pipeline: wgpu::RenderPipeline,
    atlas: wgpu::Texture,           // the tileset
    instances: wgpu::Buffer,        // GlyphInstance array, updated per frame
    quad: wgpu::Buffer,             // unit quad vertices
}
impl GlyphRenderer {
    fn draw(&mut self, frame: &[GlyphInstance], encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        // upload `frame` into self.instances, then one instanced draw:
        // render_pass.draw(0..4, 0..frame.len()) with the quad and instance buffers bound.
    }
}
```

### 14.4 Field overlays with no CPU round-trip

Because the environmental fields live in GPU buffers already (Part 5), the renderer reads them directly to tint glyph backgrounds for water, heat, fire, and similar, with no copy back to the CPU. The same device holds both the compute storage buffers and the render pipeline, so a visualization shader binds the field buffer as read-only and colours cells from it. This is the payoff of sharing one device between compute and render.

### 14.5 The camera drives simulation LOD

Zooming in is the act that promotes a region to high-LOD simulation. When the camera descends to a depth where individuals would be visible, the regions in view raise their fidelity tier and their near-camera pools promote a sample of members so there is detail to watch (Part 11). When the camera leaves, those regions demote again. The viewer's attention is therefore the allocator of compute, which is the cleanest possible realization of Principle 1.

### 14.6 Cadence and headless mode

The renderer runs at its own frame rate, decoupled from and usually slower than the simulation tick, and it never feeds back into canonical state, so it may use floating point freely. For long worldgen and history runs there is a headless mode with no renderer at all, which frees the entire GPU for compute; a thin glyph view attaches only when a human wants to watch. This is appropriate because, for now, gameplay is observation.

```rust
pub enum RunMode {
    Headless,                 // batch worldgen / history, full GPU for compute
    Observed { cam: Camera }, // attach the glyph renderer at its own cadence
}
```

---

## Part 15: Stocks, Flows, and Ecological Feedback

The ecological, material, and social systems in this world are one kind of thing wearing many costumes: stocks that flow between compartments, each with an amount, a capacity, and a regeneration rate. Vegetation biomass, soil fertility, fresh water, game density, fish, timber, ore, population, even accumulated knowledge are all stocks. An actor draws from a stock; the stock regenerates toward its capacity at some rate (zero for non-renewing stocks like ore); when sustained draw exceeds regeneration, the stock falls toward collapse, and collapse propagates into the stocks coupled to it and into what the population believes and does next. This part defines the shared abstraction; the parts that follow are specific instances of it.

A stock renews logistically toward its capacity, so recovery is fast when the stock is depleted and slows as it nears the ceiling, which is the standard model for both populations and renewable resources.

```rust
#[derive(Clone, Copy)]
pub struct Stock {
    pub amount: Fixed,
    pub capacity: Fixed,
    pub regen_rate: Fixed,   // fraction per period; 0 for non-renewing stocks (ore, gems, stone)
}

impl Stock {
    /// Logistic regeneration toward capacity, then subtract what was drawn this period.
    pub fn step(&mut self, drawn: Fixed) {
        let headroom = ONE - fx_div(self.amount, self.capacity.max(ONE));
        let regen = fx_mul(self.regen_rate, fx_mul(self.amount, headroom));
        self.amount = (self.amount + regen - drawn).clamp(0, self.capacity);
    }
    pub fn depleted(&self) -> bool { self.amount <= 0 }
    pub fn pressure(&self) -> Fixed { ONE - fx_div(self.amount, self.capacity.max(ONE)) }
}
```

Stocks live at two resolutions, and which one a region uses is the level-of-detail decision from Principle 1, applied to ecology exactly as it is applied to people. Spatial stocks (vegetation biomass, soil fertility, water, game density) are layers of the field grid, advanced on the GPU per cell where a region is near the camera or otherwise high detail, and collapsed to a handful of per-region scalars where it is not. Population and social stocks live in the aggregate pools. The same world is therefore simulated everywhere, at the grain each region warrants, which is what lets an entire living planet run at once.

The feedback that makes the world feel alive is that draws come from behaviour, behaviour comes from need and belief, and the consequences of depletion change both. A region's herbivores draw on its vegetation; its predators draw on its herbivores; its sentient population draws on game, timber, water, and soil; and when any of these crosses its collapse threshold the shock ripples through the coupled stocks and is witnessed, recorded in the event log, and folded into belief. Every later part wires a specific set of these couplings.

---

## Part 16: Flora and Vegetation

Vegetation is a stock layer over the grid with ecological structure on top. Each cell holds a mix of plant species with their biomass and a successional stage; each species carries a climate envelope, a growth rate, a successional role, a fire response, a dispersal range, and a set of mutualist animals. The field advances on the GPU as a stencil, because growth, competition, and dispersal are all local, while collapse and sentient impact are resolved on the CPU and logged.

```rust
pub struct PlantSpecies {
    pub id: SpeciesId,
    pub climate_envelope: ClimateEnvelope,    // temperature, rainfall, soil-fertility tolerances
    pub growth_rate: Fixed,
    pub max_biomass: Fixed,
    pub draws_on: SmallVec<[GrowthInput; 4]>, // data-defined: what nourishes it, so growth and
                                              // competition read its needs, not a fixed sun+water+soil
    pub successional_role: Succession,        // Pioneer | MidSuccession | Climax
    pub fire_response: FireResponse,          // FireKilled | FireTolerant | FireDependent
    pub dispersal_range: u8,                  // cells per period
    pub mutualists: SmallVec<[SpeciesId; 4]>, // pollinators, seed dispersers
    pub water_retention: Fixed,               // how much this cover holds soil and moisture
    pub scheme: SchemeId,                     // its genetic and reproductive scheme (Part 25); plants
                                              // often run the clonal or haploid variant
    pub glyph: GlyphHint,
}

// What a plant species draws on to grow, read by the growth and competition steps instead of a
// hardcoded sun-water-soil assumption. Parallel to fauna FoodSource (Part 17), so a carnivorous
// plant, a mana-fern, or a heat-feeder is a data entry rather than an engine change.
pub enum GrowthInput {
    Light,                 // sunlight reaching the cell, after canopy competition (Part 5)
    Water,                 // soil moisture and the water field
    SoilNutrient,          // the cell's fertility stock
    Resource(ResourceId),  // any data-defined stock: an arcane field, ambient heat, a mineral...
    Prey(SpeciesId),       // a carnivorous plant drawing on an animal pool
    Carrion,               // drawing on dead biomass
}

pub struct VegCell {
    pub stand: SmallVec<[(SpeciesId, Fixed /*biomass*/); 4]>,
    pub stage: Succession,
    pub soil: Stock,    // fertility, drawn by growth and farming, regenerates slowly
}
```

Four local processes govern a cell. Growth moves each species' biomass toward its maximum at its growth rate, scaled by how well the cell's climate and soil match that species' envelope. Competition allocates each cell's limited resources among the species present, where the resources in contention are whatever those species draw on by their data-defined inputs, the usual light, water, and soil nutrients for most plants but a prey pool or an arcane field for the exotic ones, so a climax species eventually shades out pioneers. Succession advances the cell's stage over time from bare ground through pioneer cover to a climax community, and resets toward bare ground after a disturbance such as fire, clear-cut, or flood. Dispersal spreads each species into neighbouring cells within its range, the stencil step that lets forests march and meadows creep.

The couplings are where this stops being decoration. Foundation and keystone species are flagged so their loss cascades: when a foundation species' biomass in a region falls below a threshold, the species that depended on its canopy or structure lose their own viability and the cell's stage regresses. Plant and animal mutualisms tie flora to fauna (Part 17): a plant whose pollinators or seed dispersers have collapsed loses its growth and dispersal, so an animal extinction can quietly doom a plant a generation later. Soil fertility is a stock drawn by growth and, harder, by farming (Part 19), so sustained agriculture in a cell drains it toward exhaustion unless rotation or fallow restores it.

The collapse modes are the scars sentients and pressure leave on the world. Deforestation is over-draw of the timber stock (Part 19) faster than regrowth, and it strips water-retaining cover. Overgrazing is herbivore or herded-animal draw (Part 17) exceeding vegetation regeneration. Soil exhaustion is farming draw exceeding fertility regeneration. Salinization is irrigation raising soil salts past tolerance. Desertification is the runaway feedback among these: cover loss reduces water retention, which dries the cell, which kills more cover, a positive loop that, once tipped, is slow to reverse, and it is the engine behind the Dust Bowl scenario when it couples to the weather layer (Part 18). Fire is its own coupling: fire-dependent biomes regress and lose species when fire is suppressed, fire spreads on the existing fire field, and slash-and-burn is a sentient tool that clears cover at the cost of igniting that field. Blight is a plant pathogen (Part 22) spreading as diffusion through stands of one species, which is what makes the monoculture of intensive farming dangerous.

Heritable variation lets a species adapt rather than only persist or perish. A stand carries allele frequencies over its heritable channels, its climate tolerances, growth rate, fire response, and pathogen resistance, which drift and respond to selection over generations under the same model as every other living thing (Part 25), so a species tracks a shifting climate toward a colder or drier margin, a forest under a recurring blight selects for resistance, and local ecotypes form where one species holds different biomes. Plants run this at the population tier, since they live as biomass stocks rather than individuals, and a plant becomes an explicit genome only when a notable organism is promoted, a great or ancient tree the world remembers.

---

## Part 17: Fauna and the Food Web

Animals run on the same fidelity tiers as sentients but live mostly as per-species, per-region population pools, with herds, packs, and flocks as the unit and individual promotion reserved for a named beast, a specific hunt, or a creature a high-detail agent is interacting with. A legendary dragon or a pack's alpha is a promoted entity with a belief store and a history; the ten thousand deer in a region are a pool with a count and an age distribution.

```rust
pub struct AnimalSpecies {
    pub id: SpeciesId,
    pub trophic_level: TrophicLevel,
    pub feeds_on: SmallVec<[FoodSource; 4]>, // data-defined: which stocks or species it draws on, so a
                                             // new feeding pattern is a data entry, not an engine change
    pub climate_envelope: ClimateEnvelope,
    pub biomes: SmallVec<[BiomeId; 4]>,
    pub repro_rate: Fixed,
    pub food_need: Fixed,
    pub water_need: Fixed,
    pub intelligence: Fixed,           // 0 = mindless .. high = great beast (the personality tiers, Part 20)
    pub trait_axes: SmallVec<[TraitId; 5]>, // its temperament axes from the world's TraitDef registry;
                                            // the cross-species palette (boldness, exploration, activity,
                                            // sociability, aggressiveness) is the default for a plain animal
    pub domesticable: bool,
    pub engineer_effect: Option<EngineerEffect>, // edits fields directly (beaver -> water)
    pub scheme: SchemeId,              // its genetic and reproductive scheme (Part 25); usually sexual diploid
    pub glyph: GlyphHint,
}

// What a species draws on, read directly by the food web (below) instead of switched on a category.
// The set is deliberately small and mostly data-referencing, so an exotic feeder (a mana-grazer, a
// stone-eater) is added as a Resource rather than by changing engine logic.
pub enum FoodSource {
    Vegetation,           // the plant biomass layer (Part 16)
    Species(SpeciesId),   // another animal pool, that is, predation
    Carrion,              // dead biomass, that is, scavenging
    Resource(ResourceId), // any data-defined stock: water, a mineral lick, an arcane field...
}

pub struct AnimalPop {
    pub species: SpeciesId,
    pub region: RegionId,
    pub count: Stock,                  // capacity = local carrying capacity
    pub age_dist: Distribution,
}
```

The spine is the food web as a coupled set of stocks. Producers are the vegetation layer; herbivore pools draw on it; carnivore pools draw on their prey pools; apex predators sit at the top. Predator and prey counts are stocks coupled so that the classic oscillation emerges on its own: abundant prey lets predators grow, growing predators crash the prey, the crash starves the predators, and the cycle turns. Carrying capacity per pool is set by the region's available food and water stocks, so exceeding it produces starvation, die-off, or the decision to migrate.

The trophic cascade is the headline coupling into flora. Removing an apex predator (through over-hunting, Part 19, or climate) lets its prey herbivores boom, the herbivores overgraze the vegetation field, and the biome shifts or collapses, the wolves-and-Yellowstone pattern made mechanical. Ecosystem engineers couple the other way: a flagged species edits the field grid directly, so beavers raise the water field and create wetlands that change which plants and animals a region can hold.

Migration is flow between region pools, driven two ways. Seasonal migration follows the climate fields as the seasons cycle (Part 18), animals tracking temperature, water, and forage across the map. Pressure migration is triggered by carrying-capacity overshoot, depletion, predation, or sentient encroachment, moving a fraction of a pool into an adjacent region with capacity. Domestication turns a wild pool into a managed one held by a civilization, with selective breeding shifting its trait distribution over generations (Part 25), directional selection on the herd's allele frequencies, toward docility, yield, or strength.

Extinction is permanent and written into history. It arrives through over-hunting beyond regeneration, habitat loss when a biome collapses and removes a species' niche, the collapse of a specialist predator's only prey, or a climate envelope that no longer exists anywhere on the map after deep-time drift (Part 18). An extinction is an event in the log, and a last-of-its-kind can be promoted to an individual for narrative weight. The intelligence field is a dial, not a wall, and it gates how much personality a creature carries (Part 20): a mindless species lives as pure pool statistics, a plain animal carries the dispositional-trait layer alone so its members differ in temperament, and a great beast high on the dial additionally carries values and goals and, when promoted, a name and a belief store. This is where werewolves, dragons, intelligent monsters, and the borderline creatures between animal and race live, holding a grudge and a hoard without being a full person.

---
## Part 18: Climate and Weather

Two layers, kept separate. The climate baseline is the static, long-term average produced by worldgen (Part 12): each cell's mean temperature, mean rainfall, and drainage, set by latitude, elevation, and prevailing wind. Weather is the dynamic, short-term realization of that baseline, simulated as fields on the GPU that fluctuate around the baseline day to day and season to season.

```rust
pub struct ClimateBaseline {        // per cell, from worldgen, slow-changing
    pub temp_mean: Fixed,
    pub rainfall_mean: Fixed,
    pub drainage: Fixed,
}

pub struct WeatherFields {          // dynamic GPU field layers, resident across ticks
    pub temp: FieldLayer,
    pub humidity: FieldLayer,
    pub wind: FieldLayer,           // vector field
    pub precip: FieldLayer,
    pub pressure: FieldLayer,
}
```

The water cycle is modeled as a closed loop so that a perturbation anywhere propagates everywhere. Evaporation from open water and transpiration from vegetation feed humidity; humidity under the right pressure and temperature becomes precipitation; precipitation becomes runoff and river flow in the water field; rivers return water to lakes and seas, where it evaporates again. Because vegetation feeds the loop through transpiration, forest cover raises local humidity and rainfall, which is real (a large forest waters itself), and stripping that cover dries the region. That is the other half of the desertification feedback from Part 16, and the full Dust Bowl chain runs: remove deep-rooted cover for farming, soil loosens and dries, wind in the weather field lifts the topsoil, dust storms strip more cover, and the region tips into a drier climate state.

Extreme events emerge when a field crosses a threshold and a stochastic trigger fires, rather than being placed by hand. Floods come from saturated soil plus heavy precipitation plus river surge overflowing into adjacent cells, damaging sites and crops. Droughts come from sustained sub-baseline rainfall, stressing vegetation, failing harvests, and pushing animal migration. Storms, blizzards, heat waves, and hail are threshold crossings in temperature, pressure, and precipitation. Lightning is a stochastic strike weighted by storm intensity that can ignite the fire field and kill or injure an exposed individual. Tornadoes are a rare high-gradient event that carves a path of destruction across cells.

```rust
pub enum WeatherEvent {
    Flood { cells: Vec<GridCell>, depth: Fixed },
    Drought { region: RegionId, severity: Fixed, since_tick: u64 },
    Storm { region: RegionId, intensity: Fixed },
    LightningStrike { cell: GridCell, ignites: bool },
    Tornado { path: Vec<GridCell> },
    Blizzard { region: RegionId },
    HeatWave { region: RegionId },
    ArcaneStorm { region: RegionId, source_event: Option<EventId> },
}
```

> Needs research, item R-WEATHER in the research backlog. The `ArcaneStorm` variant shows the seam already strained once for magic; a closed enum will keep straining as worlds add phenomena. A session should decide whether weather events become data-defined (a phenomenon named by which fields it perturbs and how) so a world's own climate and magic can introduce events the engine never enumerated.

Seasons are the climate fields cycling annually, with amplitude set by latitude, driving plant growth cycles (Part 16), animal migration (Part 17), and the agricultural calendar (Part 19). Fantasy weather rides the same field machinery as additional layers: arcane storms over a magic field, blights, an unnatural darkness or perpetual winter tied to a historical event such as a great evil's rise, and elemental phenomena. Over deep time the climate baseline itself drifts, warm ages and cold ages and the occasional volcanic winter (Part 26) shifting biomes across the whole map and forcing the mass migrations and collapses that punctuate a world's history.

---

## Part 19: Materials, Resources, and Economy

Resources come in two classes, and the distinction drives the whole economic and much of the historical arc. Stock resources are finite and effectively non-renewing on the timescale of a civilization: ore, gems, building stone. Flow resources renew if harvested below their regeneration rate: timber, game (Part 17), fish, soil fertility (Part 16), fresh water, wild forage. The flow resources are the interesting half, because each is also part of the ecology, so a forest is simultaneously a timber stock, a biome, and a climate regulator, and over-cutting it strikes all three at once.

```rust
pub enum ResourceClass {
    Stock,   // ore, gems, stone: regen_rate = 0
    Flow,    // timber, game, fish, water: regen_rate > 0
}

pub struct Deposit {
    pub material: MaterialId,
    pub class: ResourceClass,
    pub stock: Stock,        // amount, capacity, regen
    pub location: ChunkCoord,
}

pub struct Material {
    pub id: MaterialId,
    pub props: MaterialProps, // density, hardness, sharpness, value density, fuel value, ...
}
```

Over-harvest is the tragedy of the commons made literal: when extraction from a flow resource exceeds its regeneration, the stock slides toward collapse, and for timber that collapse is not merely a shortage but biome loss, erosion, and local drying, the cascade from Parts 16 and 18. This is real history, from Easter Island to the Maya to Mesopotamian deforestation, and it is the engine behind a civilization's rise and fall.

The response to depletion is where belief and culture (Parts 9, 10, 20) produce divergent fates from the same pressure. A culture whose values include stewardship and long-term thinking discovers and adopts sustainable management, crop rotation, fallow fields, harvest quotas, planted forestry, sometimes only after a collapse teaches the lesson and the survivors encode "do not over-cut" as a taboo carried forward by the transmission system (Part 23). A culture whose values prize expansion or the immediate term responds to local depletion by moving outward for more, which drives colonization, migration, and war over resource-rich land (Part 24). A culture that neither manages nor relocates collapses into a ruin with a story, the people who felled their last tree, a site whose history the legends remember.

Materials feed three further systems. They set artifact quality: different ores yield different metals with different properties, so a masterwork blade of a rare hard metal is materially better and more prized, and its provenance is tracked in the event log (Part 7). They drive trade: a region rich in ore and poor in timber becomes interdependent with its mirror, and that interdependence is either alliance or leverage and a cause of war when a route is threatened. And they generate emergent value: a material's worth is set by supply and demand across the trade network rather than fixed, so a famine makes grain priceless, a gold strike crashes gold locally, and a blockade spikes the price of whatever it cuts off.

```rust
pub struct Market {
    pub site: SiteId,
    pub price: HashMap<MaterialId, Fixed>,   // emergent from local supply and demand
    pub supply: HashMap<MaterialId, Fixed>,
    pub demand: HashMap<MaterialId, Fixed>,
}

pub struct TradeRoute {
    pub from: SiteId,
    pub to: SiteId,
    pub path: Vec<ChunkCoord>,               // through real geography (Part 13)
    pub goods: SmallVec<[(MaterialId, Fixed); 4]>,
    pub volume: Fixed,
}
```

A trade route is also a vector for disease (Part 22), knowledge (Part 23), belief, and religion (Parts 9, 10), so the economic network is at once the contagion network, the cultural-diffusion network, and the rumour network, which is why severing or opening one reshapes far more than prices.

---

## Part 20: Sentient Race Parameters

A race is not a special case in the engine; it is a parameter set over every system already described. This is what lets many races coexist with distinct natures without bespoke code per race.

```rust
pub struct Race {
    pub id: RaceId,
    pub name: String,

    // Time and memory
    pub lifespan_years: u32,
    pub maturity_years: u32,
    pub memory: Fixed,            // governs belief deterioration probability (Part 9)
    pub belief_plasticity: Fixed, // how readily beliefs and values change with input

    // Physical build and reproduction
    pub build: PhysicalBuild,
    pub repro_rate: Fixed,

    // Data-defined being attributes: this race's selections from the world's registries (Part 40).
    // A race has only the drives, traits, value axes, and genes it is given. Nothing here is
    // universal: a race with no hunger simply omits that drive, a race with no notion of
    // "extraversion" simply does not list it as a trait.
    pub drives:     SmallVec<[DriveInstance; 8]>,  // what it needs; omit any it lacks
    pub traits:     SmallVec<[TraitInstance; 8]>,  // its personality axes, not a fixed Big Five
    pub value_axes: SmallVec<[ValueAxisId; 16]>,   // the value dimensions its culture reasons over
    pub genes:      SmallVec<[GeneId; 16]>,        // its heritable axes
    pub scheme:     SchemeId,                      // how those genes are inherited (Part 25); the default
                                                   // is sexual diploid, but a race may run another
    pub producible_sounds: ProducibleSounds,       // the phonemes its anatomy can voice (Parts 33, 35, 25);
                                                   // a race may hold sounds others cannot make or perceive

    // The imbued and the innate
    pub imbued:    ImbuedTraits,        // magic affinity, disease immunity, regeneration, night vision...
    pub intrinsic: IntrinsicBeliefs,    // innate value profile + axioms + epistemic stance, seeded at the dawn (Part 28)
}

pub struct PhysicalBuild {
    pub size: Fixed,
    pub strength: Fixed,
    pub speed: Fixed,
    pub climate_tolerance: ClimateEnvelope,          // which biomes are comfortable
    pub locomotion: SmallVec<[LocomotionMode; 2]>,   // data-defined modes, not a closed enum
}
```

The being attributes are defined in data, never in code. The world definition (Part 40) carries a registry of drives, traits, value axes, genes, and actions, and a race is a selection over those registries with per-race magnitudes. This is the same mechanism as the drive system: an attribute is a named axis defined once in data, a race lists the axes it has, and a being stores its levels in per-axis columns referenced by a handle (Part 2). What a race eats is now expressed as the satisfaction source of whatever hunger-type drive it has, so the old `Diet` enum folds into the data-defined drive and satisfaction model rather than being a fixed category.

```rust
// Defined in the WorldDefinition (Part 40), selected per race above.
pub struct DriveDef    { pub id: DriveId, pub name: String, pub dynamics: DriveDynamics,
                         pub satisfied_by: SmallVec<[SatisfactionSource; 4]>, pub urgency: CurveId,
                         pub lethal_if_unmet: bool }
pub struct TraitDef {
    pub id: TraitId,
    pub name: String,                 // a race's own temperament axis, named for text
    pub lexicon: Lexicon,             // high / mid / low descriptor phrases for generated prose
    pub species_median: Fixed,        // the race's central tendency on this axis
    pub heritable_fraction: Fixed,    // h2 for this axis; a reserved calibration (see the note below)
    pub plasticity_curve: CurveId,    // how plastic this axis is by age; a reserved calibration
    pub correlations: SmallVec<[(TraitId, Fixed); 4]>, // axes need not be orthogonal
}
pub struct ValueAxisDef{ pub id: ValueAxisId, pub name: String /* e.g. honour, novelty, kinship */ }
// GeneDef (the heritable axes a race carries) is defined in full in Part 25, with its GeneEffect
// reach, dominance, linkage, and mutation regime; a race lists the GeneIds it carries, and selects
// a GeneticScheme (Part 25) for how those genes are inherited.

pub enum DriveDynamics { Accumulate { rate: Fixed },   // rises while unmet (hunger, fatigue)
                         EventDriven,                  // changes only on events, no passive drift
                         FieldCoupled { field: FieldId } } // tracks an environmental field (sunlight)
pub struct DriveInstance { pub drive: DriveId, pub scale: Fixed, pub initial: Fixed }
pub struct TraitInstance {
    pub trait_id:   TraitId,
    pub setpoint:   Fixed,  // mu: the stable central tendency, rank-order-stable across the lifespan
    pub reactivity: Fixed,  // sigma-like spread: how far momentary state swings with the situation
}
```

Personality follows the current synthesis in personality science rather than a flat vector, in three layers, and the layering is the fixed mechanism while every axis, rate, and lexicon is data (Principle 11). The reasoning and evidence are recorded in Part 62.2 and the sources in Part 63.

Dispositional traits are distributions, not points. A being does not have a single value on a trait; it has a stable central tendency and a characteristic spread of momentary states around it, which is the density-distribution finding that resolved the old person-versus-situation debate. Each trait instance therefore stores two fixed-point numbers, a `setpoint` and a `reactivity`, not one. The setpoint is what stays rank-order-stable across a life; the reactivity is how far behaviour swings with the situation and is itself a stable individual difference. The axis set a race has is its own data (`TraitDef`), so two beings with identical setpoints but different reactivity behave visibly differently, and a race carries as few or as many axes as its nature wants.

```rust
// The behaviour shown now is a deterministic draw from the distribution given the situation.
// The cheap path (low LOD) reads the setpoint; the full path (high LOD) draws the momentary state.
fn expressed(t: &TraitInstance, situation: Fixed, rng: Rng, ctr: u64) -> Fixed {
    t.setpoint + fx_mul(t.reactivity, situation_response(situation, rng.unit(ctr)))
}

// Layer 2, characteristic adaptations: more malleable than dispositional traits, enculturated
// faster and further (Part 10). Values already live in their own axis set (Parts 10, 21); goals and
// learned if-then situation-response signatures complete the layer. The if-then list is kept only
// for the significant few, where situational coherence ("bold in trade, cautious in war") earns its
// storage.
pub struct Adaptations {
    pub values:  SmallVec<[(ValueAxisId, i8); 16]>, // the value profile, the separate-but-linked layer
    pub goals:   SmallVec<[Goal; 8]>,
    pub if_then: SmallVec<[Contingency; 8]>,        // significant few only
}
pub struct Contingency { pub when: SituationTag, pub bias: (ActionId, i16) }

// Layer 3, narrative identity: the most malleable layer, the autobiographical through-line, kept as
// a sparse tag set only for promoted, load-bearing beings so a generated biography reads as a life.
pub struct Narrative { pub themes: SmallVec<[NarrativeTag; 4]>, pub defining: SmallVec<[EventId; 8]> }
```

Personality changes lawfully over a life, and the change is a deterministic integer pull. Real personalities drift in a maturity direction with age, stabilize in rank order as they age, deepen the traits that drew a being into its roles, and jump at major life events. All of this is reproduced by pulling each trait's setpoint toward a moving target by a fraction that shrinks with age, evaluated at the being's LOD cadence. The mechanism is fixed, audited Rust; its rates and curves are data and are reserved for calibration (below), since the science returns them as review values that must not be fabricated into constants.

```rust
// Mechanism is fixed; targets, the plasticity curve, and weights are data. Runs at LOD cadence.
fn age_personality(t: &mut TraitInstance, def: &TraitDef, ctx: &LifeCtx, rng: Rng, ctr: u64) {
    let target = blend(&[
        ctx.maturity_target(def),      // the race's maturation direction (the maturity principle)
        ctx.enculturation_target(def), // the culture's profile on this axis (Part 10)
        ctx.role_target(def),          // current life roles: work, partnership, parenthood
        ctx.corresponsive_bias(def),   // deepening the traits that selected the being into its situation
    ]);
    let plasticity = curve(def.plasticity_curve, ctx.age); // high in youth, low plateau in adulthood
    let delta = fx_mul(fx_mul(target - t.setpoint, plasticity), ctx.role_weight);
    t.setpoint = clamp_trait(t.setpoint + delta);          // range is the trait scale (a calibration)
}

// A child's setpoint on an axis is a heritable blend toward the population mean, plus the dominant
// non-shared term, plus a small mutation. The heritable fraction is per-axis data; selection acts on
// the heritable substrate across deep time (Part 25). The broader gene model is still open (R-GENOME).
fn inherit_trait(parents: (Fixed, Fixed), def: &TraitDef, pop_mean: Fixed, rng: Rng, ctr: u64) -> Fixed {
    let midparent = (parents.0 + parents.1) >> 1;
    let h2 = def.heritable_fraction;                        // centres near one half (a reserved value)
    let genetic = fx_mul(h2, midparent) + fx_mul(ONE - h2, pop_mean);
    genetic + nonshared_noise(rng, ctr) + mutation_drift(rng, ctr)
}
```

Life events fire discrete impulses (a one-time jump to a setpoint and a temporary rise in plasticity), and the rare deliberate self-change a being attempts is a bounded burst that decays. Rank-order stability falls out because the pulls are small against the spread between beings; mean-level drift falls out because beings of the same race in the same roles share targets. At the statistical tier the same rule advances pool aggregates rather than individuals.

Animals and great beasts run the same machinery with fewer axes and fewer layers, gated by the intelligence dial (Part 17). A mindless creature carries no personality and lives as pure pool statistics. An animal carries the dispositional-trait layer only, drawn from a small, evidence-based temperament palette (boldness, exploration, activity, sociability, aggressiveness, the axes that recur across species in animal-personality research), so a wolf pack has bold and timid members and a promoted beast has a temperament that reads in text. A great beast, the tier between animal and sentient where dragons and intelligent monsters live, additionally carries the adaptations layer (values and goals) and, when promoted, a name and a belief store, so it can hold a grudge and a hoard without being a full person. The same `TraitDef` registry and the same change and inheritance mechanisms serve all three; what differs is how many axes a species declares and how many layers its intelligence unlocks.

> Decided and reserved. The representation above is settled and signed off: three layers (dispositional-trait distributions, characteristic adaptations, narrative identity), the deterministic integer change mechanism, the heritable-plus-noise inheritance rule, and the same machinery for animals and great beasts at reduced axis count and layer depth. What is deliberately left for your call, because the research returns them as review values from human and mostly-WEIRD samples that should not be fabricated into engine constants, is the numeric calibration: the per-axis heritable fraction (the literature centres near one half), the shape of the plasticity-by-age curve (so cumulative drift and the rising rank-order stability match the targets in Part 62.2), the maturity-target directions and magnitudes, the life-event impulse and self-change burst sizes, and which contested mechanisms (the social-investment role effect above all) are enabled and how strongly per race. The value-distance metric that pairs with this layer's values is now resolved (R-VALUE-METRIC, decision in Part 21, record in Part 62.3). The axiomatic-belief stance representation that pairs with this layer is now resolved (R-AXIOM, decision in Part 28, record in Part 62.4). The broader gene model the trait-inheritance rule plugs into is now resolved (R-GENOME, decision in Part 25, record in Part 62.5), and it confirms this layer composes with it, since the inheritance rule above is the additive reduction of the fuller genome. One connected representation question stays open as its own item: which physical-build stats are primitive versus physics-derived (R-BUILD-PHYS, Parts 41, 55). The reserved list is collected in the audit log.



The parameters wire into the systems in ways that make each race occupy a different part of the world and the resource web. Lifespan sets the rate of generational turnover, and through it the speed of cultural drift (Part 10): a long-lived race holds living memory across centuries and its folklore drifts slowly, while a short-lived race turns over fast and mutates its oral tradition more per century. Memory is exactly the memory attribute that governs how quickly belief facets deteriorate (Part 9), so a strong-memory race keeps an accurate, stable history and a forgetful race generates rich, unreliable folklore. Belief plasticity is the dial on how readily enculturation and gossip move a member's values and beliefs, which is the belief mutability the design calls for, made per race.

Physical build places a race in the map and the food web. Whether a drive's magnitude relates to body size is a per-race data choice, not an engine rule: a race may tie its hunger or thirst to size, leave them independent of it, or not have those drives at all. Where a race does couple eating to size, a large member presses harder on carrying capacity and is pushed sooner toward expansion or conflict; a race that does not hunger sidesteps that pressure entirely. Locomotion and climate tolerance decide which biomes a race can hold: a race with an aquatic locomotion mode lives in and around water tiles, a winged one ignores terrain pathing costs (Part 13), a burrowing one exploits the underground (Part 26), a cold-adapted one thrives in tundra others avoid, so different races settle different regions and meet mainly at the seams. A race's drives and their satisfaction sources decide which stocks it draws on and therefore who it competes with, so a race whose hunger is satisfied only by meat contends with apex predators for game while a race that hungers for nothing competes for none of it. Reproduction rate decides how fast a race's pools expand and so how hard it pushes outward, a primary driver of which races spread and which stay small. Imbued traits layer special interactions on top: magic affinity opens the arcane fields, disease immunity removes a race from an epidemic's host set (Part 22), regeneration and night vision change combat and behaviour.

---
## Part 21: Values, Difference, and Emergent Conflict

There is no alignment axis and no evil flag anywhere in the data. A race or culture that media would call evil is, in this engine, one whose values and practices differ from, and often oppose, its neighbours', and whose conflicts with them emerge from that difference rather than from an authored disposition. This is both more honest and far more generative than a morality bit.

Every race and culture carries a value profile over the world's data-defined value axes, the same axes its individuals carry (Parts 20, 40). How opposed two profiles are is not a naive Euclidean distance, because value axes are not independent: in a human-style structure some axes are near-synonyms and others are opposites, and other races organize value space entirely differently, as a ring, a set of near-independent foundations, a hierarchy, or a lattice. The structure is therefore per-race data and the distance respects it, with the mechanism fixed and the topology chosen by the data (R-VALUE-METRIC, recorded in Part 62.3). Conflict pressure between two groups is then a function of four emergent quantities, none of which either group possesses on its own.

```rust
type ValueProfile = SmallVec<[(ValueAxisId, i8); 16]>; // the axis-tagged profile used everywhere:
                                                       // on a culture, a deity, and inside a mental
                                                       // model as a believed profile (Part 37)

// Value structure is per-race data, because the human value circle is one structure among many.
// The engine fixes the distance mechanism and lets the data pick the topology.
pub enum ValueStructure {
    Independent,                          // axes orthogonal; the distance reduces to the old Euclidean
    Relationship(Matrix),                 // a PSD weight matrix of compatible and opposing axis pairs
    Graph(SmallVec<[(ValueAxisId, ValueAxisId, u32); 32]>), // weighted edges; a circle or tree is a
                                                            // special graph the compiler recognizes
}

// Compiled offline from a ValueStructure into an exact integer all-pairs ground-metric table (shortest
// paths over integer edge weights are exact integers), so the runtime is table lookups plus a weighted
// sum and determinism is automatic. dist[i*k + j] is the structural distance between axes i and j.
pub struct GroundMetric { dist: Box<[Fixed]>, k: usize }

// The distance between two value profiles. A PURE function of the two profiles and the structure, so
// it is callable on a believed profile from a mental model (Part 37) exactly as on a true one, at any
// theory-of-mind depth. Partial profiles, where an agent knows only some of another's values, are
// handled by summing over the axes present, which is why this weighted form, not a normalized
// transport distance, is the primary runtime path. Independent structure reproduces the old Euclidean.
pub fn value_distance(a: &ValueProfile, b: &ValueProfile, g: &GroundMetric) -> Fixed {
    structural_distance(a, b, g) // weighted by g: near-synonym axes count once, opposing axes far
}

// Cross-race comparison goes through a shared etic substrate, because two races' emic axes are not
// directly comparable. Each race's axes carry an authored projection onto the substrate; distance is
// computed there. An emic value that projects to nothing is untranslatable, which is also a theory-of-
// mind blind spot: an agent cannot model a value it has no axis for. A baseline incommensurability
// term (reserved) reflects that even an aligned alien unsettles.
pub struct EticSubstrate  { pub axes: SmallVec<[EticAxisId; 8]> }
pub struct EmicProjection { pub onto: SmallVec<[(EticAxisId, Fixed); 4]> } // per emic axis: weights

pub fn cross_race_distance(a: &ValueProfile, ra: RaceId, b: &ValueProfile, rb: RaceId,
                           w: &WorldView) -> Fixed {
    let (ea, eb) = (project_to_etic(a, ra, w), project_to_etic(b, rb, w));
    structural_distance(&ea, &eb, w.etic_ground_metric()) + w.tuning.incommensurability_floor
}

pub enum ContactOrigin { Trade, Aid, Raid, Border, FirstBlood }

pub struct IntergroupRelation {
    pub a: GroupId,
    pub b: GroupId,
    pub grievance: Fixed,        // accumulated historical wrong, decays slowly
    pub trust: Fixed,            // accumulated good dealing
    pub niche_overlap: Fixed,    // competition for the same stocks and territory
    pub origin: ContactOrigin,   // how first contact went
    pub history: Vec<EventId>,   // the real record both sides will remember differently
}

// value_dist is the structural value_distance within a race, or cross_race_distance between races.
// How distance maps to pressure (the coefficient on value_dist, and whether it is linear) is a
// reserved calibration (R-VALUE-METRIC), not asserted here.
pub fn conflict_pressure(r: &IntergroupRelation, value_dist: Fixed) -> Fixed {
    fx_mul(value_dist, ONE + r.niche_overlap) + r.grievance - r.trust
}
```

> Needs research, item R-CONTACT in the research backlog. `ContactOrigin` is a closed five-way summary of how two peoples first met, and how first contact went deserves to be richer and extensible (a meeting through pilgrimage, through famine-driven migration, through a shared enemy). A session should decide whether origin becomes data-defined or is derived from the actual founding events rather than a fixed label.

The same `value_distance` serves every consumer that asks how aligned two value profiles are: conflict pressure here, the enculturation pull that moves a member toward its culture (Part 10), and the favour a deity extends to cultures whose values match its own (Part 38). It stays cheap because the structure-dependent work is precomputed once per value space into the integer ground-metric table, leaving the runtime a weighted sum over table lookups, which matters because recursive theory of mind (Part 37) calls it repeatedly, once per modelled mind per nesting level, when an agent judges who is friend, who is foe, and who is feigning alignment. Because the function is pure and reads whatever profile it is handed rather than ground truth, an agent's plot turns on the gap between what a target believes about the plotter's values and what they are, and the same distance measures that gap.

> Decided and reserved. The mechanism is settled and signed off: value structure is per-race data (independent axes, a relationship matrix, or a weighted graph, with a ring or tree recognized for the fast exact forms), compiled offline to an exact integer ground-metric table, and compared by a pure structural distance that reduces to the old Euclidean when axes are independent and that runs on believed profiles for theory of mind. Cross-race comparison goes through a shared etic substrate with authored emic projections, and an untranslatable value is a theory-of-mind blind spot. What is reserved for your call, because the research returns them as design choices or as values with no empirical anchor for non-human minds, is the numeric calibration: the default compatibility and opposition weights on the axis relationships, the coefficient mapping value distance to conflict pressure, the enculturation pull rate and whether the pull follows the structure's geodesic or a straight line, the deity favour curve and whether it weighs direction or intensity, the cross-race incommensurability floor, and the membership of the etic substrate axes themselves. The reserved list is in the audit log.

A group whose values hold raiding to be honourable, or whose religion demands expansion, or whose survival diet requires what a neighbour herds, will tend toward high conflict pressure, yet to itself it is acting virtuously, not wickedly. Whether two peoples fight or cooperate is then decided by the contingent specifics: how compatible their value profiles turn out to be, whether their resource niches collide or complement, how first contact happened (a meeting through trade or aid seeds trust and positive beliefs, a meeting through a raid seeds grievance), and how the folklore system hardens the trajectory, since a single old atrocity can mythologize into an eternal grudge and propaganda can demonize a neighbour who never wronged anyone.

Two consequences follow, and both are features. Worlds where the supposedly evil race gets along arise whenever the dice fall toward complementary niches and a peaceful first contact, and worlds of endless war arise when niches collide and folklore calcifies, both from one mechanism with nothing scripted. And a supposedly good race can be the historical villain of a particular world, a desperate people raiding a peaceful neighbour in a famine, with its own legends casting it as the hero throughout. Because each culture remembers the shared history through its own distorting belief stores (Part 9), the legends view can present the same war twice, once from each side's account, neither of them the whole truth. Morality in this world is emergent, contested, and perspective-dependent, which is the entire point.

---

## Part 22: Disease and Epidemics

Disease is modeled as a contagion over populations of every kind, sentient, animal, and plant, using the diffusion and transmission machinery the rest of the engine already runs. A pathogen has a transmissibility, a lethality, an incubation period, a host set, and a set of resistance traits that remove an individual or race from its reach.

```rust
pub struct Pathogen {
    pub id: PathogenId,
    pub transmissibility: Fixed,
    pub lethality: Fixed,
    pub incubation: u32,
    pub hosts: SmallVec<[SpeciesId; 4]>,            // which species and races it infects
    pub resistance_traits: SmallVec<[TraitId; 2]>, // imbued immunity removes a host (Part 20)
    pub vector: Option<SpeciesId>,                 // zoonotic carrier, if any
}

pub struct Infection {
    pub pathogen: PathogenId,
    pub region: RegionId,
    pub susceptible: Fixed,    // SIR-style compartments over the pool
    pub infected: Fixed,
    pub recovered: Fixed,
}
```

Spread is density-dependent, so dense settlements breed and sustain plague where scattered populations do not, and it is carried along trade routes (Part 19), which makes the economic network the contagion network and lets a disease leap between distant cities with their caravans. Zoonotic pathogens jump from an animal vector to a sentient host, opening new plagues out of the fauna layer (Part 17). Infection runs as SIR compartments over each affected pool, with the infected fraction draining the population stock at the pathogen's lethality and the recovered fraction gaining immunity.

The historical weight is large, because a plague reshapes everything coupled to population: a die-off creates labour shortage, which stalls the economy, depopulates sites, and triggers social upheaval, all of it written into the event log and the legends. Resistance is heritable and selectable (Part 25), so a population that survives a plague is more resistant to its return, and an imbued-immune race (Part 20) is simply absent from the host set, which can make it a suspected carrier in the eyes of a grieving, dying neighbour, feeding the conflict system (Part 21) through false belief (Part 27).

---

## Part 23: Knowledge, Technique, and the Reversible Tech Web

Beyond beliefs about facts (Part 9), a culture holds techniques, the practical knowledge of metallurgy, agriculture, sustainable forestry, construction, medicine, navigation, and magic. Techniques are a stock a culture accumulates, transmits, and, crucially, can lose, which makes the technological history of the world a reversible web rather than a one-way unlock ladder.

```rust
pub struct Technique {
    pub id: TechniqueId,
    pub domain: TechDomain,
    pub prereqs: SmallVec<[TechniqueId; 4]>,
    pub discovery_pressure: fn(&Culture, &WorldView) -> Fixed, // necessity raises the odds
}

pub struct CultureKnowledge {
    pub known: HashSet<TechniqueId>,
    pub proficiency: HashMap<TechniqueId, Fixed>, // erodes as practitioners are lost
}
```

Discovery is stochastic and pressure-weighted, so necessity drives invention: a timber shortage (Part 19) raises the odds of discovering forestry or coal, a plague (Part 22) raises the odds of medical technique, a military threat (Part 24) raises the odds of metallurgy and fortification. Transmission uses the same cultural channels as belief, so techniques pass between members, down to children, and across cultures through trade, conquest, and migration, the historical pattern of technological diffusion.

The distinctive feature is loss. When a culture collapses or its population crashes below the practitioners a technique requires, proficiency erodes and the technique can vanish entirely, leaving ruins its descendants can no longer build, an aqueduct or a forged alloy that becomes a mystery and a legend. Dark ages and rediscovery emerge naturally from this, and a technique lost in one place may survive in another and diffuse back later, or be reinvented under fresh pressure. This rides entirely on the transmission and pool machinery already built, pointed at techniques instead of rumours.

> Inconsistency with Part 41, flagged for reconciliation and not silently resolved here. Part 41 specifies technology as an emergent design space in which no technology is authored: its content gate explicitly rejects any attempt to define a technology type or hand a culture a finished recipe, because the designer can place only physics, and conception consults no catalogue because none exists. The origination model in this part predates that and contradicts it: the `Technique` struct is an authored catalogue node with hand-written prerequisites and a per-technique `discovery_pressure` function, and the prose has cultures discovering named, predefined techniques (forestry, coal, metallurgy, fortification), which is exactly the authored cultural outcome Part 41 forbids. What survives and is consistent is this part's transmission-and-loss machinery, the known set, the proficiency that erodes, diffusion across cultures, reversibility and dark ages, which Part 41 reuses by reference. What must change is the origination half: a technique must be conceived from need and physics (Part 41), not unlocked from an authored web. The reconciliation also has to confront a coverage gap, since Part 41's emergent model is framed around artifacts (a form, a material, a joining technique) while most techniques here are processes and methods (metallurgy, agriculture, navigation, medicine) whose emergent conception Part 41 does not fully specify and partly lists among its honest limits. Resolving this is a design decision reserved for the owner, not invented here.

---
## Part 24: War as a System

War is a system with its own dynamics, not a single rolled outcome, and it couples to nearly every other part, which is why most of a world's dramatic history is woven on it. It begins when conflict pressure (Part 21) crosses a threshold, and it proceeds through mobilization, logistics, battle, conquest, and aftermath.

```rust
pub struct War {
    pub belligerents: SmallVec<[GroupId; 4]>,
    pub cause: WarCause,        // Resource | ValueClash | Grievance | Succession | Religious
    pub start_tick: u64,
    pub theaters: Vec<RegionId>,
}

pub struct Army {
    pub owner: GroupId,
    pub strength: Fixed,        // drawn from the labour pool, which the economy then misses
    pub supply: Stock,          // food and materiel; runs down with distance and time
    pub location: ChunkCoord,
    pub commander: StableId,    // a promoted individual with goals and a reputation
}
```

Mobilization draws fighting strength from the population's labour pool, so a war at scale hurts the economy it is meant to defend, trading farmers and miners for soldiers. Logistics ties an army to food and geography (Parts 13, 19): it consumes supply that runs down with distance from home, so campaigns over hostile terrain or long supply lines wither, and controlling routes and chokepoints matters as much as winning fields. Battles resolve between armies with their commanders, strengths, supply, and terrain as inputs, and they produce the most heavily mythologized entries in the event log, since a decisive battle is exactly the salient, threat-related event the folklore system transmits and distorts most vigorously (Part 9).

```rust
fn resolve_battle(a: &Army, b: &Army, terrain: &TerrainView, log: &mut EventLog) -> BattleOutcome {
    // strength, supply, terrain, and commander skill -> casualties and victor;
    // logs the battle, may kill or promote individuals, shifts morale and grievance.
    todo!()
}
```

Conquest transfers territory and its resource stocks and subjugates or assimilates the losing population, which either folds into the victor's culture over generations (Part 10) or smolders as a grievance for a future revolt (Part 21). Aftermath generates refugees who migrate (and carry their beliefs and diseases with them), grievances that seed the next war, ruined sites that become the ruins later peoples explore (Part 26), and plundered artifacts that change hands and continue their recorded provenance somewhere new (Part 7). A long enough run of this produces the layered military history, the rivalries, the reconquests, and the legendary commanders a chronicle of the world will be full of.

---

## Part 25: Genetics and Deep-Time Evolution

Because the world runs from the beginning of its history, its living things evolve across that history rather than stay fixed. Individuals carry heritable variation, selection and drift act on it across generations, and the peoples, beasts, and forests of the world come to be visibly shaped by what they have survived. The model is one fixed, audited set of mechanisms with the genetic system itself held as data: a multi-locus quantitative spine, where many small-effect genes sum to a heritable value, with an optional Mendelian layer of explicit allele pairs and dominance laid on per gene where discrete, hideable characters are wanted, and the whole wrapped in a per-race scheme that selects how reproduction and inheritance work. The mechanism is fixed Rust; which genes exist, what each reaches, the per-race scheme, and every magnitude and rate are data (Principle 11). It is the broader substrate the personality inheritance rule of Part 20 plugs into, and it applies to all life, sentient races, animals, and plants alike, differing only in the genes they carry and the scheme they run (R-GENOME, recorded in Part 62.5). All of it is integer and fixed-point with per-entity counter-based RNG, so a lineage is bit-identical across machines and thread counts.

### 25.1 The gene and the genome

A gene is a registry entry (Part 40) that names what it affects and how it is inherited; a being's genome is a vector over the genes its race or species carries, diploid where the scheme says so. A gene may be pleiotropic, feeding more than one phenotype, and what a gene reaches is deliberately bounded to what genetics plausibly touches: personality trait setpoints (the axes of Part 20), the cognitive-capacity channels (below), physical build, imbued traits, and life history. Anatomy itself, which body parts and fluids a body plan has, is reserved as its own question bound up with the data-driven anatomy (Parts 35, 40); the interface is noted but not designed here.

```rust
pub struct GeneDef {
    pub id: GeneId,
    pub effects: SmallVec<[GeneEffect; 2]>, // the phenotypic channels it feeds; may be pleiotropic
    pub dominance: DominanceMode,           // how an allele pair resolves under a diploid scheme
    pub linkage: LinkageSite,               // its linkage group and integer map position
    pub mutation: MutationRegime,           // per-gene mutation regime (rates reserved)
    pub dm_partners: SmallVec<[GeneId; 2]>, // optional Dobzhansky-Muller incompatibility partners
}

pub enum GeneEffect {
    TraitSetpoint { axis: TraitId, weight: Fixed },      // an additive push on a Part 20 trait setpoint
    Cognition     { channel: CognitionChannel, weight: Fixed },
    Build         { channel: BuildChannel, weight: Fixed },     // size, strength, speed, climate tol., locomotion
    Imbued        { channel: ImbuedChannel, weight: Fixed },    // magic affinity, disease immunity, regen, nightvision
    LifeHistory   { channel: LifeHistoryChannel, weight: Fixed },// lifespan, reproduction
    // Anatomy is intentionally absent; see the reserved interface above.
}

pub enum CognitionChannel { ReasoningAcuity, MemoryCapacity, BeliefPlasticity }

// Falconer's parameterization, in fixed point: a is half the difference between the two homozygotes,
// d the heterozygote's deviation from their midpoint, and d/a the degree of dominance.
pub struct DominanceMode { pub a: Fixed, pub d: Fixed, pub kind: DominanceKind }
pub enum DominanceKind { Additive, Incomplete, Complete, Over, Co }

// An allele carries a small-effect additive value (the quantitative view) and, where the gene is
// Mendelian, a discrete state (the hideable view), plus a tag used for distance and incompatibility.
pub struct Allele { pub additive: Fixed, pub state: AlleleState, pub origin: u32 }

pub struct Genome { pub scheme: SchemeId, pub haps: SmallVec<[Haplotype; 2]> } // 2 diploid, 1 haploid/clonal
pub struct Haplotype { pub alleles: SmallVec<[Allele; 32]> } // indexed by the carried gene order
```

### 25.2 The per-race scheme, and how genetics is tuned

How a race or species reproduces and inherits is itself data. A scheme selects among fixed mechanism variants, defaulting to the standard sexual diploid model that ordinary creatures share, so that tuning genetics for a people is a data edit rather than a code change, and an exotic race can run a different system entirely without the engine hardcoding one.

```rust
pub struct GeneticScheme {
    pub id: SchemeId,
    pub reproduction: ReproductionMode,
    pub ploidy: Ploidy,
    pub dominance_default: DominanceKind,            // for genes that do not override it
    pub linkage_groups: SmallVec<[LinkageGroup; 8]>, // ordered loci and per-interval recombination fractions
    pub mutation_default: MutationRegime,
    pub isolation: IsolationParams,                  // the distance-to-fertility curve (reserved)
}

pub enum ReproductionMode {
    SexualDiploid,                              // the common default: Mendelian segregation plus the spine
    Haploid,
    Clonal,                                     // offspring is the parent genome plus mutation
    Eusocial { caste_rule: CasteRuleId },       // queen-and-caste inheritance
    MagicallyDetermined { rule: MagicRuleId },  // exotic, non-allelic inheritance (escape hatch)
}
```

Most creatures select the default; a magical or non-biological race selects another. `MagicallyDetermined` is an honest escape hatch, a named rule dispatching to its own audited function, because non-biological inheritance (ritual-determined, lineage-cursed, environmentally attuned) cannot be expressed as allele segregation; the engine still guarantees determinism and the data-selection of which rule applies.

### 25.3 From genotype to phenotype, and why the Part 20 rule is a special case

A phenotype channel fed by a set of genes is the sum of their additive contributions, plus the within-locus dominance deviations, plus a bounded gene-gene interaction term, plus an environmental offset, all in fixed point.

```
breeding_value  = sum over the channel's loci of each locus's additive contribution
genotypic_value = breeding_value + sum of dominance deviations + epistasis_term
phenotype       = genotypic_value + environment
```

This reconciles exactly with the personality inheritance rule already adopted in Part 20. Under the additive limit, with many small-effect loci and no dominance or epistasis, the offspring's genetic value is distributed about the midparent value with a within-family spread equal to half the parental additive variance (the infinitesimal model; Fisher 1918, made rigorous by Barton, Etheridge, and Veber 2017). The Part 20 rule, child_setpoint equals heritable_fraction times the midparent value plus one minus that fraction times the population mean plus a non-shared noise term plus mutation drift, is that limit term for term: the heritable fraction is narrow-sense heritability (the regression of offspring on midparent), the non-shared noise is the segregation draw with half the additive variance, the pull toward the population mean is the standard regression when heritability is below one, and the mutation term is the per-generation mutational input. So when every gene feeding a channel is additive, the full model collapses to the Part 20 equation; dominance, epistasis, linkage, and explicit alleles are structure laid on top. The personality rule is a clean reduction, and the two compose without contradiction.

How many loci feed a channel is reserved: more loci give smoother, more drift-stable, more nearly continuous variation (the infinitesimal regime), fewer give discrete, major-gene behaviour with visible single-gene jumps. Continuous channels (build, cognition, trait setpoints) lean many; discrete imbued or Mendelian characters lean few.

### 25.4 Dominance, linkage, and epistasis, in deterministic integer form

Dominance resolves per locus from the gene's `DominanceMode`: complete, incomplete, and co-dominance are all the one fixed-point expression of midpoint plus additive part plus the heterozygote deviation, and a recessive character hides because a heterozygote expresses the dominant allele while the recessive one persists in the haplotype and resurfaces on a homozygous pairing. Linkage is carried by ordering loci within groups: forming a gamete walks a group, and between two adjacent loci a crossover fires when a counter-RNG draw falls below that interval's recombination fraction, a fixed-point constant stored per interval. Genes in different groups assort independently. Recombination fractions are chosen by the owner at design time (the Haldane and Kosambi mapping functions are documentation-time tools, never runtime code, since they involve transcendental functions), so linkage disequilibrium and genetic hitchhiking emerge for free and deterministically. Epistasis is a bounded interaction lookup over the genotypes at interacting loci, the same mechanism that carries non-additive trait architecture and the Dobzhansky-Muller incompatibilities of deep time (25.7); it is bounded so its cost is in the number of interacting pairs, never combinatorial.

### 25.5 Mutation

A point mutation fires per locus per reproduction when a counter-RNG draw falls below the gene's rate, perturbing the additive value by a fixed-point step or flipping the discrete state; a structural mutation, where a scheme opts in, duplicates or deletes a locus or rearranges a linkage group, the rare substrate by which schemes themselves drift over very deep time. Every mutation is a pure function of the master seed, the parent's id, the phase, the locus, and an ordinal, so it is reproducible and machine-independent. Under neutral theory the rate of substitution in a population equals the per-individual mutation rate independent of population size (Kimura), so the owner's chosen rate directly sets the molecular-clock pace at which lineages diverge.

### 25.6 Intelligence, resolved

The cognitive-capacity question is settled here. Reasoning acuity is its own heritable, polygenic phenotype produced by the genotype-to-phenotype map, not a free-floating attribute and not a mere data-trait. It is the channel that gates cognitive events such as technology conception (Part 41) and sets perception and inference quality (the knowledge-formation work, R-EVIDENCE), and it is distinct from memory, which governs belief deterioration (Part 9), and from belief plasticity, which governs how readily beliefs update (Part 20), both of which remain their own channels and their own race parameters. The three may share loci, since pleiotropy is allowed and realistic, but they are not one axis. This matches the genetics of general cognitive ability, which is highly polygenic with very-small-effect loci and a heritability that rises through development (Haworth et al. 2010; Plomin and von Stumm 2018), and the empirical separability of reasoning from working memory (Conway, Kane, and Engle 2003). Intelligence therefore inherits by the same machinery as any other quantitative channel, and a race tunes it through the genes it carries and their effect sizes, all reserved.

### 25.7 Deep time: drift, selection, divergence, speciation, and hybrids

Over generations a population's heritable composition changes by drift and selection, and given isolation it diverges until it is a distinct people or species. At the statistical tier a pool carries fixed-point allele-frequency vectors. Each generation, selection shifts those frequencies by the breeder's equation, response equals narrow-sense heritability times the selection differential, for quantitative channels, and by genotype-fitness weighting for discrete loci; drift perturbs them by a Wright-Fisher sampling step parameterized by an effective population size, drawn by counter-RNG; mutation adds input; and migration mixes frequencies between pools. Founder effects fall out directly, since a small band carries a non-representative slice of its parent pool and so diverges.

Reproductive isolation is a smooth curve plus a discrete table. Genetic distance between two pools is a fixed-point function of their allele-frequency vectors, a fixation-index or Nei-style measure, and it is computed by reusing the project's configurable structural-distance machinery, the same integer ground-metric pattern that carries value distance (Part 21) and the planned language distance (Part 33), since the continuous component is another instance of it. A fertility function maps that distance to the probability a cross succeeds and to hybrid fertility, a reserved curve. Laid over it is a Dobzhansky-Muller incompatibility table: specific allele pairs, neutral in their own lineage, that combine lethally or sterilizingly in a hybrid regardless of overall distance, which is the discrete part that distance alone cannot capture (Coyne and Orr). Where a scheme has sex chromosomes, Haldane's rule, that the heterogametic sex bears the brunt of hybrid sterility or inviability, is expressible as a data-defined asymmetry in which recessive incompatibilities are unmasked in the hemizygous sex. Speciation is declared, not scripted: when the distance between two diverged pools crosses a reserved threshold, or accumulated incompatibilities exceed a reserved count, the engine records them as distinct species with separate identities. A cross between still-compatible races produces an offspring whose genome is the recombined union of the parents' haplotypes under the child's resolved scheme, whose phenotype follows the same map, and whose fertility comes from the distance curve and the incompatibility table, so a sterile mule-like hybrid is the natural high-distance or incompatibility-triggered outcome and a fertile blended people is the low-distance one.

### 25.8 The two tiers, and crossing between them

The masses live as allele-frequency pools advanced statistically, the same level-of-detail principle the rest of the engine follows, while a promoted being carries an explicit genome. Promotion generates that genome by sampling each locus from the pool's allele frequencies, under the scheme's expected genotype frequencies, by counter-RNG keyed on the new being's id, so the individual is statistically consistent with the pool it came from. Demotion folds the individual's genotype back into the pool's frequency counts by canonically ordered accumulation with fixed rounding. The accepted, documented cost is that linkage disequilibrium and family structure built up among promoted individuals are lost on demotion, since only marginal frequencies survive the fold.

### 25.9 Plants and animals

The model is universal. Animals carry it directly: a species names its scheme, almost always the sexual diploid default, and its temperament axes (Part 17) and its build are gene-fed phenotypes, so a wolf pack's bold and timid members differ at the genes, and the selective breeding of domestication is directional selection on the herd's allele frequencies toward docility and yield, exactly as Part 17 describes. Plants carry it at the population tier: a stand of a species carries allele-frequency distributions over its heritable channels, its climate tolerances, growth rate, fire response, and pathogen resistance, and those frequencies drift and respond to selection, so a species adapts to a colder margin over generations, a forest under a recurring blight (Part 22) selects for resistance, and local ecotypes emerge where one species occupies different biomes. Plant schemes lean on the clonal and haploid variants as often as the sexual one, since much of the plant world reproduces without two parents. A plant becomes an explicit genome only on the rare promotion of a notable organism, a great or ancient tree the world remembers. The disease-resistance and domestication hooks elsewhere in the design (Parts 17, 22) are, under this model, selection differentials applied to the relevant alleles, so the heritable adaptation those systems already promise is grounded in the genome rather than asserted.

### 25.10 Determinism

Every operation here is integer and fixed-point. Allele effects, dominance, epistasis, the breeder's-equation response, and the distance measures are sums and products of fixed-point quantities; every stochastic step, which allele segregates, whether a crossover or mutation fires, the drift sample, and the segregation noise, keys counter-based RNG on a hash of the master seed, the entity id, the phase, the locus, and an ordinal, with no sequential RNG state threaded between loci or beings, which is what makes the result independent of thread count and evaluation order. The one genuine hazard is the Gaussian draw the infinitesimal segregation term and the mutation step need, which must not use a floating-point logarithm or square root; it is computed by a fixed-point inverse-CDF table or an integer sum-of-uniforms approximation, and which approximation and what precision is a reserved decision. Pool aggregation is canonically ordered by id with fixed rounding.

> Decided and reserved. The mechanism is settled and signed off: a multi-locus quantitative spine with an optional Mendelian dominance layer per gene, wrapped in a per-race scheme that selects fixed reproduction and inheritance variants and defaults to sexual diploid, applied uniformly to sentient races, animals, and plants. The genotype-to-phenotype map is settled, and the Part 20 personality inheritance rule is its additive reduction, so they compose. Dominance is Falconer's a and d in fixed point; linkage is ordered loci with stored per-interval recombination fractions; epistasis and Dobzhansky-Muller incompatibilities are a bounded interaction lookup; mutation and all stochastic steps are per-locus counter-RNG. Intelligence is resolved as its own heritable polygenic channel, distinct from memory and belief plasticity, sharing loci by pleiotropy but not collapsed into them. Deep time is Wright-Fisher drift plus breeder's-equation selection over allele-frequency pools, with genetic distance reusing the structural-distance machinery, a reserved fertility curve, a discrete incompatibility table, Haldane's rule as a data-defined asymmetry, and declared rather than scripted speciation. The two tiers cross by Hardy-Weinberg-consistent promotion and frequency-folding demotion. The mechanism is fixed Rust; which genes exist, the per-race schemes, the effect sizes, and the rates are data. What is reserved for your calibration, because the research returns these as design choices or as review values that must not be fabricated into constants, is the numeric set: the per-channel narrow-sense heritability (Part 20 already centres it near one half); the loci-per-channel counts; the allele effect-size scales and the dominance degrees; the per-interval recombination fractions; the point- and structural-mutation rates and the mutation step-size distribution; the effective population size per pool; the selection-differential scaling for domestication and disease; the genetic-distance-to-fertility curve; the speciation distance and incompatibility-count thresholds; the rule for which parent's scheme governs a hybrid; the choice of fixation-index versus Nei distance; and the integer Gaussian approximation and its precision. The reserved list is in the audit log. Two honest limits stand: the statistical tier tracks marginal allele frequencies, not linkage disequilibrium or family structure, so interaction-dependent phenomena are exact only among currently promoted individuals; and magical inheritance resists the single mechanism, so `MagicallyDetermined` is a dispatch to a bespoke audited rule rather than a parameterization of the standard one, which is the boundary of the one-mechanism goal.

---

## Part 26: Geology, the Deep Map, and Catastrophe

Beneath the surface terrain sits a geological model that places resources by history and opens the world downward. Rock type and ore distribution follow the region's geological past, so volcanic provinces carry certain ores and sedimentary basins carry others, and the underground extends into caverns and z-levels for mining, buried things, and subterranean races (Part 20).

```rust
pub struct GeoColumn {                 // per chunk, downward
    pub layers: SmallVec<[GeoLayer; 8]>,
}
pub struct GeoLayer {
    pub rock: RockType,
    pub deposits: SmallVec<[Deposit; 2]>, // ore, gems, stone (Part 19)
    pub depth: i16,
    pub cavern: bool,
}

pub enum Catastrophe {
    Eruption { cell: GridCell, winter_severity: Fixed }, // ash -> volcanic winter (Part 18)
    Earthquake { epicenter: GridCell, magnitude: Fixed },
    GreatFlood { region: RegionId },
    Meteor { cell: GridCell },
}
```

> Needs research, item R-CATASTROPHE in the research backlog. These four are geological; a world with magic and divine action (Parts 34, 38) has catastrophes they do not cover (a magical cataclysm, a god's wrath, a planar rupture). A session should decide whether catastrophes become data-defined by the terrain and field changes they cause, so the set is open to a world's own forces rather than fixed in code.

Catastrophes are rare punctuations that reset regions and bend history. A large eruption injects ash that drives a volcanic winter through the climate baseline (Part 18), cooling the world and forcing migration and collapse, and it reshapes the local terrain. Earthquakes, great floods, and meteor strikes destroy sites, alter rivers and coastlines, and bury the works of earlier peoples. Those buried works become the deep map's reward: ruins covered by time that later civilizations rediscover, with artifacts whose provenance has been lost (Part 7) and whose true history only the event log still holds, an archaeology of the world's own past that feeds new legends.

---

## Part 27: The Belief-Reality Coupling

This part adds no new data; it names the loop that ties every preceding system together and is the thematic heart of the simulation. The belief system (Part 9) is wired in both directions.

Reality shapes belief. Events in the world, a battle, a famine, a forest's fall, a neighbour's raid, are witnessed, and witnessing writes facets into the belief stores of those present, with distortion at intake and at every later retelling. What a population knows of its world is therefore a lossy, drifting projection of what truly happened, and different peoples hold different versions of the same events.

Belief shapes reality. Beliefs drive action through the decision systems (Part 8), and action changes the world's stocks, populations, and politics. A culture that believes a forest sacred will not cut it, and so preserves the biome and the climate it regulates (Parts 16, 18). A culture that believes a neighbour demonic strikes first (Part 24), and the attack makes the enemy real. A famine falsely blamed on a minority (Part 21) becomes a persecution and then a war, on the strength of a belief that was never true. A widely held prophecy becomes self-fulfilling as people act to meet it. A technique believed lost is not sought, and so stays lost (Part 23).

The loop closes: reality to belief to action to reality, with the truth bending at every hop, running across the ecological, material, social, epidemiological, and military systems at once. A region's deforestation changes its weather, which fails its harvests, which starves its people, who blame the gods or a neighbour, which sends them to war or to ruin, which their descendants remember as a myth that shapes how the next civilization treats its own forests. No single part produces that arc; the coupling does. Building the parts so they feed one another, and the belief system so it both records and drives, is what makes a watched world feel as though its history was lived rather than authored, which is the entire aim of the project.

---

## Part 28: The Dawn of Sentience and Intrinsic Beliefs

The world does not begin with civilizations. It begins at the moment every race crosses into sentience at once, as small scattered bands holding nothing but their bodies, their intrinsic capabilities, and a seeded character of intrinsic beliefs. Everything else, language, fire, agriculture, metallurgy, money, law, government, written religion, cities, and artifacts, is developed from there by the races themselves, emergently, and no two worlds develop it the same way.

What a race starts with is exactly two things, both moddable start variables (Part 40). Its intrinsic capabilities are the Race parameter set (Part 20): lifespan, memory, build, diet, reproduction, imbued traits, the physical and cognitive hand it was dealt. Its intrinsic beliefs are a seeded starting disposition, not learned but innate to the race at its dawn: a value profile, a small set of axiomatic beliefs about the world, the self, others, and the sacred, and an epistemic stance for how it decides what is true, all of which colour how it interprets everything it later witnesses.

An axiom is not one number. Position and hold are independent: two races can both hold that outsiders are kin, one as an unquestioned axiom and one as a revisable hunch, and a single scalar cannot tell them apart. So an axiom record separates where the stance sits from how hard it is to move, and carries the epistemic basis that decides which evidence even counts. The set of axiom axes a race has, and every coefficient in the record, is per-race data (Part 40); the mechanism below is one fixed, audited kernel, and races differ only in their data (R-AXIOM, recorded in Part 62.4).

```rust
pub struct IntrinsicBeliefs {
    pub values:    ValueProfile,             // innate profile over the race's value axes (Parts 20, 21)
    pub axioms:    SmallVec<[Axiom; 8]>,     // innate foundational stances, seeded at the dawn
    pub epistemic: EpistemicStance,          // how this mind decides what is true and defends it
}

// Foundational stance: few per race, deep, slow. Position (stance) and hold (strength, confidence,
// entrenchment) are separate fields, after Rokeach's central-peripheral architecture and AGM
// epistemic entrenchment. Every field is fixed-point or an integer rank, so the kernel is deterministic.
pub struct Axiom {
    pub axis:         AxiomAxisId, // which per-race axiom axis (data: e.g. world dangerous..safe)
    pub stance:       Fixed,       // signed position on the bipolar axis
    pub strength:     Fixed,       // conviction magnitude, 0..1
    pub confidence:   Fixed,       // evidential weight behind the stance, 0..1
    pub entrenchment: i32,         // AGM ordering rank: the gate deciding what yields first
    pub salience:     Fixed,       // how often this axiom is invoked in appraisal
    pub stubbornness: Fixed,       // Friedkin-Johnsen anchor weight to the innate seed, 0..1
    pub innate_seed:  Fixed,       // heritable anchor stance, set at birth, immutable (the FJ prejudice)
    pub evidence: ArrayVec<EvidenceTag, AXIOM_EVIDENCE_CAP>, // bounded ring; the B-plus-C slice (cap reserved)
}

pub struct EvidenceTag { pub event: EventId, pub source: SourceModeId, pub pressure: Fixed }

// The epistemic stance parametrizes the update kernel for every OTHER belief, so the engine has one
// kernel whose coefficients are read from the believer's own epistemic axioms. Source modes (tradition,
// evidence, revelation, authority, intuition, or a race's own) are data (Part 40).
pub struct EpistemicStance {
    pub source_weights: SmallVec<[(SourceModeId, Fixed); 8]>, // unit-sum: which evidence counts, by origin
    pub dogmatism: Fixed, // global damping on all accommodation
    pub seizing:   Fixed, // urgency: how fast it locks onto an early answer (need-for-closure)
    pub freezing:  Fixed, // permanence: how strongly it then defends it
    pub certainty: Fixed, // knowledge as fixed vs evolving: the default entrenchment a new belief gets
}

// The per-race registry entry for an axiom axis (Part 40): its lexicon and per-axis dynamics defaults,
// so the rates are data, not engine constants. The numbers are reserved for calibration.
pub struct AxiomAxisDef {
    pub id:            AxiomAxisId,
    pub negative_pole: String,     // e.g. "domination", "hostile", "dangerous"
    pub positive_pole: String,     // e.g. "reverence", "trusting", "safe"
    pub domain_tag:    AxiomDomain, // World | Selfhood | Others | Sacred | Epistemic
    pub lexicon:       Lexicon,     // descriptor phrases for generated prose
    pub heritability:  Fixed,       // heritable fraction of the innate seed (reserved)
    pub plasticity:    CurveId,     // accommodation step by age or context (reserved)
    pub calcify:       Fixed,       // entrenchment gained per quiet phase, to a cap (reserved)
}

pub enum AxiomDomain { World, Selfhood, Others, Sacred, Epistemic }
pub struct SourceModeDef { pub id: SourceModeId, pub name: String } // tradition, evidence, revelation...
```

The update rule is one kernel run at the believer's LOD cadence, with every coefficient read from data or from the believer's own epistemic stance. An incoming event is appraised through the axiom before it can touch the axiom: evidential pressure is the event's salience times the source weight for its provenance times one minus the dogmatism damping. Below an entrenchment-gated threshold the pressure is assimilated, nudging a value or a fast belief facet (Part 9) rather than the axiom, which is how belief perseverance and confirmation bias fall out of the gate rather than being coded as exceptions. Above it the axiom accommodates, moving toward the evidence by a plasticity-scaled step; a rare high-salience event scaled by its source weight, a revelation to a revelation-weighted mind, can clear even a high threshold in one step, the conversion or revelation jump. Enculturation is a Friedkin-Johnsen anchored average: a member moves toward the confidence-weighted mean of the stances it is exposed to but keeps a fixed-weight anchor to its own innate seed, so a society never fully converges and lasting between-group difference emerges without being scripted. Influence is bounded by confidence, a member is moved only by others within a confidence band of its own, and weighted by conformist and prestige bias, so sects fracture when a central axiom's variance crosses a fission threshold and group boundaries sharpen. Inheritance draws a child band's seed as a heritable blend of parental seed and local cultural mean plus a bounded counter-RNG mutation, the same anchor the enculturation rule pulls against. All of this is integer and fixed-point with per-entity counter-based RNG; aggregation is canonically ordered by id and rounding is fixed, so the result is bit-identical across machines and thread counts.

The three belief layers are distinct by role, not by type, which is why the old belief-domain registry becomes the axiom-axis registry and the term stops colliding with the belief facets of Part 9. Axioms are few, deep, and slow, the seed crystal that biases the priors of value adaptation and gates the acceptance of new facts. Value axes (Parts 20, 21) are a moderate, more malleable priority vector that weights appraisal and utility. Belief facets (Part 9) are the many, fast, specific propositions the folklore and gossip layer fills in. Axioms set the prior and the plausibility ceiling for values and act as an appraisal filter on incoming facts; accumulated value drift and a persistent weight of disconfirming facts are the only upward path back to an axiom, and it is deliberately slow. In the appraisal and utility layer (Part 8) axioms and values supply the weights and biases on considerations while belief facets supply the facts, so an axiom is the slowest-moving term in every decision a being makes.

The same kernel runs for an individual, a band, a culture, and a race, but a culture is not a mind sitting above its people with a kernel of its own. The kernel runs on whatever the canonical unit is at the current level of detail (Part 54), and there are two regimes. When a group is made of promoted individuals, each member runs the kernel on the events it personally lived, and the group's axiom profile is a derived summary, the confidence-weighted, canonically-ordered mean of its members, recomputed each phase; an individual is flagged as deviating when its stance leaves a band around that mean (the Dwarf Fortress cyan highlight, generalized). So a culture's belief moving never means a culture-level kernel fired: it means members' own stances moved, or the composition changed through births seeded from parents blended with the local mean and through deaths and migration, or the enculturation feedback tightened the group, since each member is pulled toward the mean and the mean is the aggregate, so the population pulls itself inward until the stubbornness anchors settle it into a persistent spread rather than a single point. When that spread on a central axiom grows too wide, the group is splitting into sects. When a group is too large to simulate per person, it is an aggregate pool carrying a representative profile plus a spread, and the same kernel runs once on that representative, driven by population-scale events, a lost war, a famine, a prophet, a raid, with pools swaying each other by the same bounded-confidence pull weighted by contact; promotion samples an individual from the pool and demotion folds one back into its statistics, so the aggregate path is the affordable compression of the per-individual path, the same math applied once to a representative instead of many times.

Nothing here is per-culture code: the axiom set and every coefficient, the seed stance, the default entrenchment, the heritability, the plasticity, the stubbornness anchor, the confidence band, the conformity and prestige strengths, the calcification rate, and the deviation and fission thresholds, are per-race data or reserved global defaults, so a eusocial hive, a solitary predator, and a trading people each get their own axioms and their own dynamics from data alone.

> Decided and reserved. The representation is settled and signed off: a multi-field axiom record separating position (stance) from hold (strength, confidence, entrenchment) with an epistemic stance that parametrizes one fixed update kernel, plus a bounded evidence ring as the first slice of a justification model whose full provenance graph is a later goal. The dynamics are settled: an AGM entrenchment gate for holding firm versus yielding, a Friedkin-Johnsen anchored average for enculturation, bounded-confidence with conformist and prestige bias for schism and drift, calcification of unchallenged axioms, and heritable-plus-encultured inheritance. The mechanism is fixed Rust; the axiom axes, the source modes, their lexicons, and every coefficient are per-race data, so the model runs across races and not only cultures. What is reserved for your call, because the research returns them as design choices or as review values from human and mostly-WEIRD instruments (the Social Axioms Survey, the Primals inventory, the Need-for-Closure subscales, the conformity coefficient) that must not be fabricated into engine constants, is the numeric calibration: the evidence-ring capacity; the entrenchment-threshold curve and the accommodation step; the calcification rate, its cap, and the brittleness under repeated challenge; the stubbornness anchor and whether it derives from dogmatism and freezing; the confidence band, the conformity and prestige strengths, and the fission and deviation thresholds; the revelation-jump salience threshold; the heritable fraction and the mutation spread; the per-race source-weight, dogmatism, seizing, freezing, and certainty defaults; and how stance and strength scale the bias they place on a consideration; and, raised separately, how the hold fields aggregate to a group, since a culture's entrenchment is probably better derived from the variance of its members, low variance meaning a value held rigidly across the whole people, than from the mean of their entrenchments, and that choice changes how fast a culture moves relative to an individual. The reserved list is in the audit log.

These axioms are the seed crystal for everything that grows. A race whose innate stance toward nature is reverent tends, across centuries, to develop stewardship practices, nature religion, and sustainable resource use (Part 19); a race whose innate stance is domination tends toward extraction, expansion, and the resource wars that follow. A race innately disposed to see outsiders as hostile starts most first contacts (Part 21) from suspicion; one disposed to exchange starts them from trade. The axioms do not script the outcome, they bias the emergent trajectory, and the same axioms under different ecological and historical pressure produce different histories.

The dawn replaces the abstract civilization-placement step of the old worldgen pass (Part 12). Worldgen now builds the natural world and seeds proto-populations of each race at sentience, and the historical simulation begins from nothing but those bands, growing language, technique, society, and belief through the emergent systems (Parts 23, 33, 36) rather than starting from pre-built civilizations. The whole arc of a civilization, from a few sentient bands to an empire with a written language, a coined currency, a state religion, and a legendary history, is a thing the simulation produces, not a thing it is given.

---

## Part 29: The Dramaturg

A world this generative produces far more history than any observer can follow, so the hardest problem for a watching-game is not generating drama but surfacing it. The dramaturg is the subsystem that continuously reads the live event stream, recognizes narratively significant patterns, scores them for dramatic weight, and steers the observer and the cinematic camera (Part 31) toward what matters. It is the difference between a world that generates brilliant history no one can find and one that tells you where to look.

The dramaturg is built around a library of story shapes, each a pattern matched against the unfolding event log (Part 7) and the belief stores (Part 9). A story shape is not a template the world is forced into; it is a recognizer that fires when the emergent history happens to trace a dramatically resonant curve.

```rust
// Story shapes are an open, re-wireable registry, not a fixed enum. The dramaturg loads a list of
// shapes (a dramaturg config alongside the world definition); each names itself and carries a
// recognizer, so shapes can be added, removed, or re-wired without touching engine code. The
// recognizer is still code today (a function over history and belief); a data-defined pattern
// language for recognizers is a later option. What is a lever now is the SET of shapes the
// dramaturg looks for, so what counts as a story can be changed as the world demands.
pub struct StoryShape {
    pub id: ShapeId,
    pub name: String,   // reversal, tragedy, vengeance, dynasty, schism, prophecy... or your own
    /// Returns Some(instance) when the pattern is present in a window of history,
    /// scored by how strongly and completely it matches.
    pub recognize: fn(&HistoryView, &BeliefView, window: TickRange) -> Option<StoryInstance>,
}

pub struct StoryInstance {
    pub shape: ShapeId,
    pub principals: SmallVec<[StableId; 4]>, // the figures, houses, or peoples involved
    pub span: TickRange,
    pub key_events: Vec<EventId>,            // the beats that compose the arc
    pub weight: DramaScore,
}
```

Dramatic weight is a composite score, because not every match deserves the camera. It rises with the magnitude of the stakes (how much changed: a throne, a people's survival, an artifact of legend), with reversal (how far the principals rose or fell), with the salience the world's own people assign it (an event already spreading through the folklore system is, by definition, one that matters to them, Part 9), with personal investment (whether the observer has been following a principal), and with rarity (a pattern the world has not shown lately). The dramaturg reads belief as well as fact, so it can surface a betrayal the victim does not yet know about, or a war fought over something both sides misremember, the dramatic irony only this engine's split between truth and belief makes available.

```rust
pub struct DramaScore {
    pub stakes: Fixed,
    pub reversal: Fixed,
    pub folkloric_salience: Fixed, // how much the world's people are already telling this story
    pub observer_investment: Fixed,
    pub novelty: Fixed,
}
fn rank(s: &DramaScore) -> Fixed { /* weighted sum, tunable */ todo!() }
```

The dramaturg runs continuously at a coarse cadence over recent history, maintaining a ranked, decaying queue of live story instances. New beats update the instances they belong to; completed arcs, a vengeance fulfilled, a dynasty ended, get a final score and pass into the legends as recognized stories. The observer layer (Part 31) draws from the top of this queue to decide where to point the camera, and the legends browser uses the same recognized instances to let a person read the history as stories rather than as a flat log. The dramaturg authors nothing: it invents no events, only finding the shapes already present in what the simulation produced, so everything it surfaces did happen in the world.

---

## Part 30: The Temporal History Database

Beneath the dramaturg, the legends browser, and any follow-this-thread-across-the-centuries feature sits a query problem the snapshot-and-log persistence (Part 7) does not solve: complex, time-ranged, relational questions over millions of historical entities and events. The temporal history database is an embedded query engine over the event log that answers them.

The questions it must answer are concrete. What happened to this animal population, every event touching that pool from its origin to its extinction (Part 17). What wars have been waged over this one ore deposit, every conflict whose cause or theater references that deposit (Parts 19, 24). What became of every descendant of this person on the northern continent between these two centuries, and what each side believed caused the war that consumed them. These are joins across entity, region, time, and event kind, with the history filtered, ordered, and sometimes evaluated as of a past moment.

```rust
pub struct HistoryDb {
    log: EventLog,                       // the append-only ground truth (Part 7)
    by_entity: Index<StableId, EventId>, // every event referencing an entity
    by_region: Index<RegionId, EventId>, // spatial index
    by_kind:   Index<EventKind, EventId>,// typed index
    by_time:   BTreeIndex<u64, EventId>, // ordered for range scans
    lineage:   LineageIndex,             // ancestor/descendant closure (Part 7 relations)
}

pub struct Query {
    pub entities: Option<EntityFilter>,  // a set, a lineage closure, a pool, an artifact, a deposit
    pub region:   Option<RegionFilter>,
    pub kinds:    Option<SmallVec<[EventKind; 8]>>,
    pub time:     Option<TickRange>,
    pub as_of:    Option<u64>,           // evaluate world or belief state as of this tick
    pub viewpoint: Option<StableId>,     // answer from this entity's beliefs, not ground truth
}

impl HistoryDb {
    pub fn run(&self, q: &Query) -> Vec<EventId> { /* intersect indices, scan, order */ todo!() }
    /// State reconstruction: the value of an attribute, or a belief, as it stood at a past tick.
    pub fn as_of(&self, subject: StableId, attr: AttrKind, tick: u64) -> Option<AttrValue> { todo!() }
}
```

Two properties make it more than a log scanner. It is temporal in the bitemporal sense: it answers not only what happened but what was true as of year N and what a given person believed as of year N, reconstructing past state by replaying the relevant slice of the log, which is what the dramaturg and the legends browser both need. And it carries a viewpoint parameter, so any query can be answered from ground truth or from a chosen entity's distorted beliefs (Part 9), which is what lets the legends show the same war from each side's memory. The indices are maintained incrementally on append, the lineage closure reuses the relationship edges (Part 7), and the engine is the substrate that finally makes a world of this size inspectable, both for the player and for you debugging it.

---
## Part 31: The Cinematic Observer

The glyph renderer (Part 14) draws the present; the observer is the framework for navigating deep time and being shown the drama the dramaturg finds. It is what turns a simulation you can look at into a history you can watch.

The intelligent camera takes the dramaturg's ranked queue (Part 29) and decides where to point, framing the principals of the highest-weighted live story, cutting between concurrent arcs the way an editor would, holding on a moment the world considers momentous, and respecting any lock the observer has placed. It moves through the multi-scale hierarchy (Part 14) as the story demands, pulling out to the overworld for a migration or a war's sweep and pushing in to a single tile for a duel or a coronation.

```rust
pub struct Observer {
    pub focus: Focus,
    pub locks: SmallVec<[Focus; 4]>, // threads the person has chosen to follow
    pub time_cursor: TimeCursor,     // live, or scrubbed to a past tick
    pub mode: ObserveMode,
}

pub enum Focus {
    Free,
    Entity(StableId),     // a person, beast, or artifact
    Lineage(StableId),    // a bloodline through time
    Institution(InstId),  // a government, guild, or faith (Part 36)
    Site(SiteId),
    Pool(PoolId),         // an animal population or a people
    Story(StoryId),       // a dramaturg arc, followed beat to beat
}

pub enum ObserveMode {
    Director,   // the camera follows the dramaturg automatically
    Follow,     // locked to a chosen thread
    Browse,     // the legends browser, reading history as story
}
```

Three capabilities sit on top. Lock-on lets a person follow a chosen person, bloodline, artifact, or institution through time, with the camera and the legends staying with that thread. Time-scrubbing and replay run over the history database (Part 30), so the observer can rewind to any past tick and watch it forward, or jump to the moment a surfaced story began. And the legends browser reads the recognized story instances (Part 29) and the history database as narrative, presenting a life, a war, or an artifact's saga as a told story, with a viewpoint toggle so the same events can be read as they truly happened or as a given people remember them (Parts 9, 30). This is navigation of the world's whole history, which is the actual game.

---

## Part 32: Temporal Level of Detail

Spatial level of detail (Principle 1) runs regions near the observer at full tick resolution and the rest as statistics. Temporal level of detail is its twin in time, and it is the capability that would let an entire world be simulated from the dawn across deep time on one machine. It is also the hardest single addition in this document and is recorded here as a direction to research rather than a settled design, because whether it is needed depends on whether a single machine can otherwise hold the full-fidelity world over the spans you want.

The idea is an adaptive, per-region simulation clock. A quiet century in a backwater advances in coarse statistical steps that move its pools, stocks, and aggregate beliefs forward cheaply without simulating individuals. The instant something significant happens there, a war, a discovery, a catastrophe, or the observer turns to look, that region drops to fine tick-by-tick resolution, runs at full fidelity while it matters, and coarsens again when the drama passes.

```rust
pub enum TimeResolution {
    Fine { dt: Ticks },   // full per-tick simulation
    Coarse { dt: Ticks }, // statistical advancement over pools and stocks
    Dormant,              // only woken by an incoming event or the observer
}

pub struct RegionClock {
    pub region: RegionId,
    pub resolution: TimeResolution,
    pub local_tick: u64,  // regions can sit at different points until they interact
}
```

The hard part, and the reason this is research rather than specification, is determinism (Part 3). Variable and per-region timesteps fight the seed-reproducible guarantee directly: the coarse and fine paths must produce the same canonical result for the same elapsed time, regions advancing at different local clocks must reconcile correctly when they interact, and the whole thing must stay bit-identical across runs and machines. Meeting that needs an event-driven core that can promote and demote a region's time resolution while preserving determinism, which is difficult and the right thing to prototype in isolation before committing to it. If a single machine proves able to hold the full-fidelity world over the spans you want without it, it is unnecessary; if not, it is the mechanism that makes the full vision run, and it is the temporal completion of the level-of-detail principle the engine otherwise only half realizes.

---

## Part 33: Procedural Language and Linguistic Drift

Each culture grows its own language, and the growth goes all the way down: agents at the dawn of a people coordinate a sound system and the first form-meaning pairings into existence, then the language drifts and splits over generations by the same accumulated-change mechanism as folklore, applied to words. The hard requirement holds throughout: however alien a language becomes, the reader is always shown an English gloss of the gist, so a name, an inscription, or a recorded saying renders both in its own tongue and in a translation the reader understands. One fixed, audited set of mechanisms carries all of it; the content, which sounds a race can make, which words and concepts a culture forms, its grammar, and its script, emerges and is data, not authored into the engine (Principle 11). The whole pipeline is seeded and deterministic like worldgen (Part 12), integer and fixed-point with per-entity counter-based RNG, so a language is bit-identical across machines and thread counts. The six questions this rested on, what a concept is and how it stays legible under emergence, how language distance is measured, whether multilingualism is modelled per being, whether a language barrier distorts belief, how writing emerges, and the generation and drift internals, are now resolved together (the R-LANG cluster, recorded in Part 62.6), with the dawn of language handled by a hybrid that bootstraps the first anchors through coordination and then hands off to seeded generation (33.9).

### 33.1 The concept and the semantic substrate

A word means something, and that meaning has to be a thing the engine knows even as the surface form drifts, or the legibility guarantee cannot hold. A concept is therefore a deterministic, integer-representable region over a shared semantic substrate, and concepts are emergent: a culture forms them by drawing category boundaries over the entities, events, relations, affordances, and values the world already represents. This is the direct sibling of the value etic substrate (Part 21); cross-culture comparison runs on the shared substrate while each culture's own partition is its emic projection.

```rust
pub struct SemanticSubstrate {              // the fixed grounding floor, authored once, not emergent
    pub primitive_axes: Vec<AxisDef>,       // grounded in semantic primes and molecules (the NSM set)
    pub world_axes: Vec<AxisDef>,           // projected from existing state: material, size, sharpness,
                                            //   affordances (cuts, contains, burns), roles (Part 36),
                                            //   values (Part 21), genome features (Part 25)
    pub relation_axes: Vec<AxisDef>,        // kinship dimensions, spatial frames, taxonomic is-a/part-of
}

pub struct Concept {
    pub id: ConceptId,                      // stable, hashed from (culture_id, birth_event_id)
    pub constraints: Vec<AxisConstraint>,   // a region over the substrate; canonically sorted by AxisId
    pub prototype: Vec<Fixed>,              // a Rosch-style prototype point; graded membership is distance
    pub parent: Option<ConceptId>,          // split and merge lineage
    pub salience: Fixed,                    // usage-weighted; drives retention and drift
    pub gloss: GlossKey,                    // deterministic handle into the gloss machinery (33.2)
}
pub struct AxisConstraint { pub axis: AxisId, pub lo: Fixed, pub hi: Fixed, pub weight: Fixed }
```

The primitive axes are the only innate semantic content in the engine, the grounding floor that stops the symbol-grounding regress: symbols bottom out in non-symbolic categorical features, not in other symbols. The world and relation axes are projected directly from state the engine already holds, so a newly invented tool category, institution, or role automatically exposes a feature signature, and a culture can form a concept over it with no human in the loop. A culture forms a new concept when its agents repeatedly need to distinguish a referent that current concepts fail to separate, a discrimination failure measured as referential ambiguity crossing a reserved threshold over recent salient experience; the engine splits the tightest containing region along the substrate axis that maximises integer information gain, with id-ordered tie-breaks, and the new region lexicalises if it earns enough use under the dawn dynamic. Concepts drift as their prototype migrates with the culture's salient exemplars (semantic shift), split under discrimination pressure (one culture lexicalises ice apart from snow where another holds one word), and merge when two low-salience neighbours fall into disuse (the lower id absorbs the higher). Because each culture runs its own partition over the same substrate, two cultures end with non-aligned concept spaces, which is the source of much of the intrigue: a distinction one people cannot express, another grammaticalises.

### 33.2 The deterministic gloss, and the non-authoritative interpretation layer

The English gist is a deterministic engine-side fact, computed with no human and no model in the loop, so the legibility guarantee never depends on anything non-reproducible. Each substrate axis carries an authored English lemma (this is the one sanctioned hardcoding, justified because the substrate is finite, universal, and mechanism rather than content). The gloss of a concept is then a deterministic function: find the nearest authored anchor among the primitive and world lemmas in fixed point, qualify it with the axes that most distinguish this concept from its parent and siblings, and emit a bounded gist string with a confidence tag (a region near the molecule for tool, with the cuts axis high and the long axis high, glosses as a long cutting tool, blade-like). Any emergent concept gets a legible gloss, including concepts formed over freshly invented artifacts or institutions, because those expose authored axes.

A separate, optional, external interpretation layer may elaborate this for the reader at view time (turning the gist and the concept structure and the culture's flavour into richer prose), but it is strictly downstream and non-authoritative. It can never be read back into canonical state, never feeds distance or any agent's reasoning, and if it is absent the deterministic gist still renders. This confirms the intended separation and hardens it: the elaboration is carried as a view-only type that no canonical-state function accepts, so backfeed is a compile-time impossibility rather than a matter of discipline. The guarantee is the deterministic gloss; the interpretation layer is optional polish on top.

### 33.3 Sounds, and per-race vocalization

A language is built from the sounds its speakers can make, and which sounds those are is itself per-race data, not a fixed human inventory. The phonetic substrate is a data registry of articulatory features, and each race possesses a producible sound set, the phonemes its vocal anatomy can form, determined by the data-driven anatomy (Part 35) and gene-affected through the genome (Part 25, an imbued or build channel for the vocal apparatus). A common baseline covers the human-like sounds most races share, but a race may possess features and sounds beyond it, simultaneous dual tone, stridulation, sub- or ultrasonic registers, resonances tied to magic, click types, whatever its body affords, and so its languages can take shapes other races cannot voice. Because the producible set can itself change as anatomy evolves over deep time, a lineage can gain new sounds and expand the space its language explores. This is the sign-off condition: the mechanism for sounds is fixed, the producible set is per-race data tied to the reserved anatomy interface.

```rust
pub struct PhoneticSubstrate { pub features: Vec<ArticulatoryFeatureDef> } // place, manner, voicing,
                                                                           // height, backness, and any
                                                                           // non-human features a race adds
pub struct ProducibleSounds {              // per race; what its anatomy can voice (Parts 35, 25)
    pub phonemes: Vec<PhonemeId>,          // the sounds available to build a phonology from
    pub perceivable: Vec<PhonemeId>,       // what it can hear but may not be able to produce
}
pub struct Phonology {
    pub consonants: Vec<PhonemeId>,        // a selection from the race's producible set
    pub vowels: Vec<PhonemeId>,
    pub syllable_templates: Vec<SyllableShape>, // phonotactics, e.g. CV, CVC, (C)(C)V(C)
}
```

The split between what a race can produce and what it can perceive matters downstream. A sound one race cannot make raises the phonetic distance to a language that uses it (33.5) and feeds the cross-race incommensurability floor, and it sets a hard production ceiling on learning (33.6): a being cannot speak a phoneme its anatomy cannot form, though it may still understand a tongue that uses it. The mapping from anatomy to producible sounds is the reserved anatomy interface; the baseline substrate contents and each race's producible set are data, surfaced for your calibration rather than fixed here.

> Needs research (R-LANG-MODALITY, surfaced after the cluster sign-off). The system here models one communication channel, the vocal-auditory one, and assumes it in the phonology structs and the producible-sound capability. Generalizing modality to a first-class property, so signed and non-vocal languages exist, an individual can lose hearing or voice by birth or injury, and a race may not vocalize at all, with per-being production and reception channels that are gene-and-anatomy determined (Part 25) and injury-damageable (Part 35), is item R-LANG-MODALITY in the audit log, flagged not designed here.

### 33.4 Generation and drift

At a culture's dawn the generator emits a distinct but typologically plausible language from the culture's seed. Phonology is assembled by selecting a phoneme inventory from the race's producible set, with size and composition sampled from typological priors (consonant and vowel inventory sizes and the implicational and dispersion tendencies real sound systems obey, all reserved with their basis in the typological record), and a phonotactics. Morphology samples a morphological type and an affixation tendency; grammar samples a dominant word order from the cross-linguistic distribution and then its harmonic correlates, so an object-before-verb language tends to postpositions and a verb-before-object language to prepositions, with a small reserved probability of disharmony, since real languages are overwhelmingly harmonic. The lexicon is filled by sampling word-forms under the phonotactics for each concept the culture holds, with the first core anchors set by the dawn dynamic (33.9).

```rust
pub struct GrammarParams { pub word_order: WordOrder, pub head_dir: HeadDir,
                           pub morph_type: MorphType, pub alignment: Alignment }
pub struct Word { pub form: Vec<PhonemeId>, pub concept: ConceptId } // form plus its concept gloss
pub struct Language {
    pub id: LangId, pub parent: Option<LangId>,
    pub phonology: Phonology, pub morphology: Morphology, pub grammar: GrammarParams,
    pub lexicon: Vec<Word>,                       // canonically sorted by ConceptId, not a hashed map
    pub sound_change_log: Vec<SoundChangeRule>,   // the drift history that makes descent reconstructable
}
```

Drift then runs over generations, each operator deterministic. A regular sound change is a rewrite rule, a sound becoming another in a stated phonetic environment, applied at once to every lexicon form that meets it, the regularity that lets descendants be reconstructed and gives true cognate correspondences. Words fall out of use and are replaced, and concept prototypes migrate (semantic shift, 33.1). Content words drift along the one-way grammaticalisation cline toward function words and affixes, changing the morphology over time. When two pools of one language fall below a contact threshold, they accumulate independent sound-change logs from divergent seeds and become sister languages under a shared parent, so the parent pointers and the logs reconstruct a real family tree; contact adds borrowing, a wave over the tree. On contact, high-salience foreign words, for concepts the borrower lacks or for prestige, enter the lexicon adapted to the borrower's phonotactics. The drift operators and their rates are reserved; the procedures are fixed and seeded.

### 33.5 Language distance and mutual intelligibility

How far two languages have diverged is a structural distance over a structured space, and it reuses the value-distance machinery (Part 21) rather than inventing a new metric. It has three fixed-point components. Phonological distance is a feature-based distance between the two inventories and phonotactic profiles over the shared phonetic substrate, with sounds one side cannot produce or perceive contributing to a phonetic incommensurability term. Lexical distance is the dominant component: over the core concepts both cultures hold, it is the normalised integer edit distance between their word-forms (phoneme-index strings), averaged, with cognate recognition falling out of low edit distance and concepts one side lacks adding a capped distance through the incommensurability floor rather than being dropped. Grammatical distance is the distance over the typological parameter vector (word order, head direction, morphological type, alignment). The three combine under reserved weights, with lexical distance weighted highest because it dominates real mutual-intelligibility measurement; aggregation is ordered by concept id and rounding is fixed. Mutual intelligibility is a decreasing function of the combined distance, consumed as a friction and throughput multiplier on the transmission of belief and knowledge across a language barrier (Parts 9, 23): dialects understand each other nearly fully, separate families barely at all.

### 33.6 Multilingualism, interpreters, and learning

Whether a being knows several tongues is modelled at the level of detail the rest of the engine uses: per individual for promoted beings, as an aggregate for the masses. A promoted being holds its native language and a small set of others each at a fixed-point proficiency; a pool carries an aggregate distribution of languages known, and intelligibility stays a population-level friction at that tier, with promotion and demotion mapping between the two by counter-RNG sampling and folding. Acquisition is a deterministic per-tick increment driven by exposure, by a per-being aptitude, by an age-of-acquisition multiplier that steps down past a reserved breakpoint (the empirical record places grammar-learning ability roughly preserved into the late teens and declining after, the breakpoint reserved with that basis), and inversely by the distance to a language the being already knows, so a near tongue is learned faster. A production ceiling applies across races: a being cannot come to speak a sound its anatomy cannot form (33.3), though it may learn to understand the tongue. An interpreter or translator role emerges where a being holds high proficiency in two or more languages (Part 36), and because an interpreter can be low-skilled or disloyal, it becomes a deliberate point of distortion or of lies into the belief system, which is a feature for intrigue rather than a defect.

```rust
pub struct LangKnowledge {                 // promoted beings only
    pub native: LangId,
    pub others: Vec<(LangId, Fixed)>,       // proficiency in each, sorted by LangId
}
```

### 33.7 Mistranslation as belief distortion

Crossing a language barrier adds error to a belief, beyond merely slowing its passage. When a belief facet passes from a speaker of one tongue to a hearer of another, the engine computes an error budget from the language distance and the interpreter's proficiency, and applies it deterministically to the facet via counter-RNG keyed on the transmission event. The error takes three forms: the conveyed concept is snapped to the nearest concept in the hearer's own partition of the substrate, so a distinction the hearer's language lacks is lost; a heard form within edit distance of a different word in the hearer's language can bind the wrong concept, a false-cognate error; and graded nuance on the facet is coarsened in proportion to the budget. The distorted facet then propagates and decays as any belief does (Part 9), and can corrupt one being's model of another specifically through the barrier (Part 37), so mistranslation joins witnessing and retelling as a place where the truth bends. The error-budget constants are reserved; the mechanism is fixed.

### 33.8 Writing and literacy

Writing is invented in the emergent-technology layer as a response to need, but the script itself and its effect on language live here. The technology layer (Part 41) decides whether and when a culture invents writing, triggered by accumulated pressure from trade volume, institutional record-keeping, and large populations crossing a reserved threshold, which matches the historical pattern in which writing was devised independently only a few times and grew from tallies and accounting marks toward fuller script. The kind of script that emerges is a deterministic function of the language's own phonological complexity rather than an authored choice or a forced trajectory: a large syllable inventory and complex phonotactics push toward logographic or syllabic writing, a small and simple one toward syllabic or alphabetic, the position on that continuum reserved as weights. Literacy spreads as a skill, per being for the promoted and as a pool fraction for the masses, through the same diffusion machinery as any technique and gated by institutions such as scribal schools or a priesthood. The payoff is the effect on belief: a written record decays far more slowly than memory, locks its provenance so it resists the per-hop distortion of oral retelling, and can carry across time without a living witness, while oral transmission keeps its higher decay and distortion. This makes the difference between an oral and a literate culture a real difference in how far and how faithfully its past survives, with the decay and provenance modifiers reserved for calibration.

### 33.9 The dawn of language

The first language is neither fully templated nor simulated from nothing at full cost. A thin coordination dynamic runs once per culture at its dawn, over a small set of the most salient anchor concepts and a discrete dispersion game that settles a seed sound system, establishing the first form-meaning pairings and the starter inventory through genuine agent coordination, and then hands the inventory and anchors to the seeded generator (33.4) to fill the full lexicon and to the drift engine thereafter. The dynamic is bounded, a hard cap on rounds with counter-RNG keying and id-ordered aggregation, so it is bit-deterministic by construction, and it runs only at the culture or pool tier, so it is cheap. It captures the property the science establishes, that a shared structured code self-organises from local coordination, by instantiating its outcome rather than paying for the full continuous simulation. Setting the round cap to zero collapses it to pure seeded generation, and growing the anchor set buys more emergence, so the balance between coordinated emergence and cost is a reserved knob. This is the chosen origination model among the three considered (pure seeding, full bootstrapping, and this hybrid); it is the one that honours order emerging from coordination as a real if abstracted mechanism while staying inside the determinism and performance budgets.

### 33.10 Determinism

All canonical language state is integer and fixed-point: every distance, proficiency, salience, prototype point, and threshold is fixed-point, and no floating point touches it. Every stochastic choice, phoneme selection, word-form sampling, sound-change selection, the dawn dynamic's tie-breaks, and the mistranslation draws, is keyed on a hash of the master seed, the culture or entity id, the phase, and an event ordinal, so it is reproducible regardless of evaluation order or thread count. Sounds are integer indices into an inventory, never platform strings, so edit distance, hashing, and the sound-change rewrites are integer operations with defined results, and the edit-distance computation walks a canonical order; the lexicon is a sorted vector, not a hashed map, so iteration is deterministic. The English gloss lemmas are authored constants, and any rich text rendering happens at view time. Exactly one thing is non-deterministic and it is fully quarantined: the optional external interpretation layer, carried as a view-only type that canonical code cannot accept. The honest limits are that the dawn dynamic must be hard-capped, which slightly trims realism for guaranteed determinism; that the typological priors and implicational filters keep generated languages in the plausible region but cannot certify every one as natural; and that a concept far from any authored anchor on the substrate receives a coarse gist rather than a crisp gloss, so legibility is guaranteed but precision is not.

The translatability requirement is structural, not cosmetic. Every word carries its concept, every name and inscription is generated from concepts, and the renderer (Parts 14, 39) always has the underlying meaning, so it can show the alien surface form beside an English rendering of what it means. The language can differ from English in every way, in sound, in structure, and in word, while the reader still gets the gist, which is what keeps a world full of invented tongues legible.

> Decided and reserved. The mechanism is settled and signed off. A concept is a deterministic integer region over a shared semantic substrate grounded in semantic primes plus projected world and relation features; concepts are emergent, formed by each culture's own partition of that substrate, and drift, split, and merge, so cultures carve the world differently while staying comparable on the shared substrate. The English gist gloss is a deterministic engine-side function of a concept's structure, never dependent on the optional external interpretation layer, which is view-only and cannot backfeed (the sign-off condition on the model layer, confirmed and hardened to a type guarantee). Sounds are per-race: the phonetic substrate is data, each race has a producible sound set fixed by its anatomy and genome and may hold sounds others cannot voice or even perceive, so its language takes shapes others cannot, and the producible set can grow as anatomy evolves (the sign-off condition on vocalization). Generation and drift are seeded and deterministic like worldgen, with regular sound change as a rewrite, grammaticalisation, splitting into real family trees by parent and sound-change log, and borrowing on contact. Language distance reuses the value-distance machinery as three fixed-point components (phonology with a cross-race phonetic incommensurability term, the dominant lexical edit distance with an incommensurability floor, and grammar), feeding mutual intelligibility as transmission friction. Multilingualism is per individual for promoted beings and aggregate for pools, with an emergent interpreter role and a cross-race production ceiling on learnable sounds. A language barrier distorts belief, beyond slowing it, by concept-snapping, false-cognate error, and nuance loss into the belief and theory-of-mind systems. Writing is invented in the technology layer (Part 41) but its script and its effect on belief live here, the script type emerging from phonological complexity and a written record cutting belief decay and locking provenance. The dawn of language is the hybrid: a thin bounded coordination dynamic sets the first anchors and sound system, then hands off to seeded generation, collapsing to pure seeding at a zero round cap. The mechanism is fixed Rust; which sounds a race can make, the concepts a culture forms, its grammar, and its script are emergent and data. What is reserved for your calibration, surfaced rather than fabricated, with its basis given: the discrimination and lexicalisation thresholds and the concept drift rate; the phoneme inventory sampling priors and implicational and dispersion tendencies (basis in phonological typology); the baseline phonetic substrate and each race's producible sound set (basis in the reserved anatomy interface); the word-order and morphological-type sampling distributions and the disharmony probability (basis in word-order and harmony typology); word-form length and shape; the drift operator rates (sound change, lexical replacement, grammaticalisation, splitting, borrowing); the three language-distance component weights (basis: lexical distance dominates mutual-intelligibility measurement); the second-language acquisition rate, the aptitude range, and the age-of-acquisition breakpoint (basis in the age record placing the decline after the late teens); the mistranslation error-budget constants; the writing-invention pressure threshold, the script-type continuum weights, the literacy spread rate, and the written-record decay and provenance modifiers; and the dawn dynamic's round cap and anchor-set size (basis: performance budget against emergence richness). The reserved list is collected in the audit log. The honest limits stand as stated in 33.10, and the anatomy-to-sound mapping remains the reserved anatomy interface rather than being designed here.

---

## Part 34: Magic as a Grounded System

Magic is a simulated layer the world obeys, not a flag, and it is grounded: it has a defined magical physics, consistent rules with costs and limits and risks, so that magical effects are as lawful within the world as the water cycle is. It is discovered, practiced, and transmitted through the same real frameworks as any other technique and belief, so a magical tradition spreads, drifts, and can be lost exactly as a craft or a rumour does.

```rust
pub struct MagicLaws {                       // a moddable start variable (Part 40): the world's magical physics
    pub sources: SmallVec<[ManaSource; 4]>,  // where magical energy comes from (the arcane fields, Part 18)
    pub costs: CostModel,                    // what working magic consumes and risks
    pub limits: LimitModel,                  // what it cannot do, and the price of pushing those limits
}

pub struct MagicalTradition {
    pub id: TraditionId,
    pub school: School,                   // the domain and method it works through
    pub techniques: HashSet<TechniqueId>, // discovered and transmitted as techniques (Part 23)
    pub tenets: Vec<Tenet>,               // beliefs about how and why it works (Parts 9, 10)
}
```

Because magic is grounded, it interacts with every other system rather than sitting beside them. It draws on the arcane fields that ride the same GPU machinery as weather (Part 18). Its energy is a stock with a cost, so over-drawing it has consequences the way over-harvesting a forest does (Part 19). Magical agriculture or healing eases carrying capacity and plague (Parts 17, 22); magical war changes battle (Part 35); weather magic perturbs the climate fields; imbuing is what gives certain artifacts their powers and is recorded in their provenance (Part 7). A race with innate magic affinity (Part 20) begins disposed toward it, but the specific traditions are developed, not given.

Transmission through real frameworks is the point you asked for. A magical technique is discovered under pressure, taught between practitioners, passed to apprentices and children, diffused by trade and conquest, and lost when its practitioners are lost (Part 23), while the beliefs about how magic works, which can be true or false, spread and distort through the folklore system like any other belief (Part 9). Whether the underlying laws are real and fixed or whether the practitioners merely believe they understand them is itself a lever, and where gods are real (Part 38) some magic may be divine rather than arcane.

---

## Part 35: Detailed Combat, Anatomy, and Wounds

At the promoted scale, combat is resolved in detail rather than by the abstract battle function (Part 24), because a remembered fight needs real wounds. The framework models bodies as assemblies of parts made of materials, weapons and armour as materials that interact, and injuries as lasting, consequential damage, generalized so that a race need not bleed blood, or bleed in the way a mammal does at all.

```rust
pub struct Body {
    pub parts: Vec<BodyPart>,             // built from the race's data-defined anatomy (Part 40)
    pub fluids: SmallVec<[FluidPool; 4]>, // what this race runs on: blood, ichor, sap, none...
}

pub struct BodyPart {
    pub kind: PartKindId,               // data-defined in the race's anatomy (Part 40): a limb, organ,
                                        // carapace segment, wing, or something with no real-world analog
    pub tissues: SmallVec<[Tissue; 4]>, // layered materials: skin or chitin, muscle, bone, organ
    pub material: MaterialId,           // each tissue is a material with properties (Part 19)
    pub vital: bool,                    // destruction or failure is lethal
    pub functions: SmallVec<[FunctionId; 4]>, // data-defined: what losing it costs (grip, sight, flight)
    pub damage: PartDamage,             // accumulated injury to this part, the part-level condition
    pub contains: SmallVec<[OrganId; 4]>,
}

pub struct FluidPool { pub kind: FluidKind, pub amount: Stock, pub critical: Fixed }

pub enum Wound {
    Cut { part: PartId, depth: Fixed, severed: bool },
    Blunt { part: PartId, fracture: bool },
    Pierce { part: PartId, organ_hit: Option<OrganId> },
    Burn { part: PartId, severity: Fixed },
}
```

> Needs research, item R-WOUND in the research backlog; not resolved here. Two things are open and neither is invented. First, the damage-mode taxonomy above is a closed enum: cut, blunt, pierce, and burn are physical force modes, but a world with magic and exotic matter needs modes that do not exist in reality (corrosion, freezing, disintegration, curse), so modes should be data-defined alongside anatomies and materials. Second, and the reason this is research rather than a quick conversion, is how wounds differ across races whose anatomies differ: what a wound means, how it propagates through tissues and functions, and which injuries are even possible depend on a body plan that is itself data, so the wound model must be defined against the data-driven anatomy rather than assuming one body plan. This needs its own session. The same session should settle the fluid model (item R-FLUID): `FluidKind` is likewise a closed enum, and what a fluid is, how its loss impairs function, and how fluids interact (clotting, ignition, corrosion) should be data-defined against the anatomy rather than fixed in code.

The part-level body above is the canonical model of an individual's physical condition. There is no separate single health value (the old scalar was removed): a being is alive while no vital part is destroyed and no fluid pool is past its critical fraction, and any aggregate "how hurt" figure the decision systems (Part 8) or the abstract battle model (Part 24) need is derived from the part damage and fluids, never stored as a competing truth. As with the rest of the individual, this detail runs at the fidelity the individual warrants (Principle 1): a promoted combatant carries a fully tracked body, and the aggregate masses remain statistics with no per-body model until promoted.

A strike resolves through the materials. A weapon's material and edge meet a body part's tissues and any armour over them, and whether it cuts, fractures, pierces, or is turned aside is a function of those material properties (Part 19), the force behind it, and the wielder's skill. Armour is modeled as material layers that absorb, deflect, or fail under the right attack, so the protective measure matters and the right weapon against the right armour matters. Wounds have lasting consequences: a severed limb is gone and its function with it, a pierced organ impairs or kills, a fluid pool drained past its critical fraction is fatal, and that fluid is whatever the race runs on, so an internal-fluid-loss model that is blood for one race is ichor or sap or something stranger for another, with each race's anatomy and fluids defined in data (Part 40).

Skill with weaponry is a learned, partly heritable capability that improves with use and is taught (Parts 23, 25), so a veteran fights differently from a conscript, and a legendary duelist is mechanically as well as narratively exceptional. This detail runs only for the promoted handful, where a fight will be remembered and mythologized (Part 9), while the masses still resolve through the aggregate battle model (Part 24), which is the level-of-detail principle applied to violence.

---

## Part 36: Emergent Institutions, Governance, and Economy

The social and economic order is emergent in the strong sense: governments, currencies, guilds, legal codes, religious hierarchies, and economic classes are not systems that exist in code waiting for a race to unlock them, they are forms the races work out for themselves, and different cultures with different intrinsic beliefs (Part 28) under different pressures invent different forms, or never invent a given form at all. This is Principle 8 made concrete, and it is the hardest emergence in the engine, so the approach is to model the substrate from which institutions crystallize rather than to predefine the institutions.

The substrate is repeated coordinated behaviour. When agents, pursuing their goals through the decision systems (Part 8) under their culture's values, fall into a stable recurring pattern of coordination, that pattern can crystallize into an institution: a persistent entity that outlives its members, encodes a role structure and a set of rules, constrains the agents inside it, and accrues legitimacy or corruption over time.

```rust
pub struct Institution {
    pub id: InstId,
    // No authored kind tag. Identity is structural and functional, read off the fields below.
    pub roles: Vec<Role>,        // the positions within it and their powers and duties
    pub rules: Vec<Norm>,        // crystallized enforced behaviour, ADICO-shaped (below); taboos -> laws
    pub coordinates: FunctionVec,// emergent intensities over the institution-function substrate (Part 40)
    pub legitimacy: Fixed,       // a belief held by the governed (Part 9); erodes -> revolt
    pub resources: ResourcePool, // treasury, holdings, sacra, whatever the institution controls
    pub members: Vec<StableId>,
    pub founded: EventId,        // provenance to the crystallizing behaviour (Principle 9 gate)
    pub parent: Option<InstId>,  // institutions spawn and reform from others
    pub descriptor: EticDescriptor, // DERIVED, non-authoritative; for legibility only, never an input
}
```

An institution carries no authored category. What it is, is read off its structure, its roles, the norms it enforces, the legitimacy it holds, its lineage, and above all the function it coordinates, and any notion of its kind is recovered from that structure rather than declared. This resolves the one place the old model contradicted itself: a `kind` field commented as emergent but typed as a closed five-way enum (governance, faith, guild, legal, market), which both predefined the categories a people could fall into, in violation of Principle 8, and authored a cultural outcome, in violation of Principle 9. A people that crystallizes a scholarly academy, a military order, a caste, a secret society, a bank, a hospital order, a mercenary company, or a mystery cult now has somewhere to live, because none of those is a slot to be matched: each is a configuration of the same emergent parts. The move is the one the engine already makes for artifacts, which carry an etic description derived from physics and an emic name the culture gives (Part 41), and for concepts, which are emergent regions over a shared semantic substrate (Part 33). An institution's identity is its own emergent emic thing, recovered as a derived etic descriptor only for legibility and comparison, and never used to steer behaviour.

What an institution coordinates is expressed over an institution-function substrate, the etic floor of what coordination can be about, and this is the direct sibling of the value etic substrate (Part 21) and the semantic substrate (Part 33). The substrate is a data registry of function axes (organized force, the sacred and the production of legitimacy, exchange and credit, knowledge and its transmission, care and provisioning, and whatever else a world distinguishes), authored once like the other substrates, and it is an affordance floor rather than a cultural outcome, which is what lets Principle 9 permit authoring it: the dimensions coordination can run along are a structural fact about social worlds, the way the semantic primes are a structural fact about meaning, while which institutions a people forms, what they are called, and how they are built remain emergent. An institution's `coordinates` is an emergent blend over this substrate, a vector of fixed-point intensities, not a single chosen slot, so a body that is high on both the sacred and exchange axes is a temple that also banks, something no single-variant enum could express. The substrate is where per-race difference enters at its deepest: an exotic people whose social life organizes around an axis no human society has, brood-tending for a hive race, mana-channeling for a magical order, a diapause-council for a race that sleeps through seasons, gets that axis in the substrate as data, and its institutions then occupy a region of the space other races cannot, so their crystallized forms are structurally unlike anything the others produce rather than being forced onto a shared human shape.

```rust
// An emergent blend over the institution-function substrate (Part 40). Not a slot.
pub struct FunctionVec { pub intensity: Vec<Fixed> } // one Fixed per substrate axis, AxisId-ordered

// A norm in the Ostrom-Crawford ADICO grammar: the statement's TYPE is itself emergent
// from which components are present (strategy = A.I.C, norm = A.D.I.C, rule = A.D.I.C.O),
// so membership-gating and succession are ordinary norms rather than authored sub-enums.
pub struct Norm {
    pub attribute: AttributeSel,       // to whom it applies (which roles or members)
    pub deontic: Option<Deontic>,      // may / must / must-not; absent means a mere strategy
    pub aim: ActionRef,                // the action prescribed
    pub condition: ConditionExpr,      // when, where, and how it applies
    pub or_else: Option<SanctionRef>,  // the consequence of violation; absent means norm, present means rule
    pub enforcement: Fixed,            // how reliably enforced (crystallization strength)
    pub provenance: EventId,           // the repeated enforced behaviour it crystallized from (Parts 10, 21)
}

// DERIVED for legibility only; recomputed from structure, never a behavioural input.
pub struct EticDescriptor {
    pub best_match: Option<TemplateId>, // nearest recognition template (Part 40), or none -> generic
    pub similarity: Fixed,              // how well it matches; a weak match reads as "weakly X"
}
```

The norms carry the Ostrom-Crawford grammar so that rule, norm, and bare strategy are distinguished by structure rather than by a flag, and so that how an institution gates membership and passes its roles are themselves emergent norms (the condition under which a non-member may join, the rule by which a role passes) rather than two more authored taxonomies. Law is then not a primitive but a crystallization, an institution whose norms are dominated by enforced sanctions over society-wide attributes (Parts 10, 21), and a market, a guild, or a bank is likewise recognized by what it coordinates and how its norms read, never declared.

Any human-readable type is a derived etic descriptor, computed by recognizing the institution's emergent feature signature against a library of descriptive templates. A template is a data-defined prototype, a feature vector with per-feature weights, and recognition is a fixed-point polythetic match, a weighted min-over-max (Tanimoto) similarity summed in canonical feature order, so no single feature is necessary or sufficient and membership in a category is graded (the family-resemblance and prototype view, Needham and Rosch, made integer-exact). The descriptor is the best-matching template only when similarity clears a reserved threshold; otherwise the institution gets a generic structural description built from its dominant axes (a knowledge-coordinating body with initiation-gated membership), and a configuration that matches nothing is never blocked, relabelled by force, or prevented from existing, it simply reads as generic. Templates are the observer's vocabulary, not the world's: they are loaded as data, read only in this recognition pass, and have no path back into the decision systems (Part 8), crystallization, or culture (Part 28), so they recognize a guild-like body without ever causing or constraining one. The owner may ship none, in which case every institution reads by its generic structural description and the engine still runs, which is the safety net against the recognition library being hard to calibrate. The culture's own emic name for the institution is grown from its roles, norms, and coordinated need exactly as a concept's word-form is (Part 33), so the observer renders the emic name beside the etic gloss: the Order of the Silver Hand, a force-and-belief order.

Other systems reason over function and capability rather than over category, which is both what removes the tag cleanly and what makes the model more expressive than the enum it replaces. Taxation asks whether an institution coordinates exchange and holds a treasury, conflict and diplomacy ask whether it commands force, succession reads the norm by which its roles pass, a postal role (Part 41) asks whether it coordinates knowledge and has the reach, and legitimacy reads the legitimacy field directly (Part 9). Because these are questions about what an institution does, the temple that also banks is taxed and granted clerical legitimacy at once, with no contradiction, and the extractive-versus-inclusive character a polity has (whether power and membership are open or closed) falls out as another derived reading over the role and membership norms rather than an authored label. Comparison across cultures reuses the value and language distance machinery (Parts 21, 33) as an institution-distance: a fixed-point structural distance over the shared function substrate and feature signature, so two peoples that both crystallized something guild-like land close even when their emic names and norm details differ, and a people whose academy doubles as a court lands between clusters, the convergence-on-recognizable-forms that institutional sociology observes (the isomorphism of DiMaggio and Powell) measured rather than imposed.

All of this is integer and order-independent. The feature signature is extracted by canonically id-ordered aggregation over roles, members, and norms (Part 58); the recognition and distance sums are fixed-point in canonical feature order, so they are bit-identical across machines and thread counts; any stochastic step in crystallization or a recognition tie-break is keyed on a hash of the master seed, the institution id, and a phase, with id-ordered tie-breaks; and the descriptor, being a pure function of canonical state, is recomputable on demand and need not enter the state hash, the way render state does not. At the aggregate tier an institution is carried as a compact feature vector and a count rather than explicit roles and members, enough to compute descriptors and distances at the pool level, and promotion materializes an explicit institution whose feature signature must reproduce the pool's, which the tier-consistency mechanism (R-TIER-CONSIST, Part 54) carries as one of the declared conserved projections; because the descriptor is derived, it is consistent across promotion and demotion for free as long as the features are conserved. The crystallization substrate itself is unchanged: when agents under their culture's values (Part 28) fall into a stable recurring pattern of coordination (Part 8), that pattern crystallizes into an institution whose roles, norms, and coordinated function are read off the actual behaviour and the need it served, with full provenance to the founding event, and the descriptor is computed afterward for legibility alone.

> Decided and reserved. The mechanism is settled and signed off, and it is built to be maximally emergent and differentiable per race. An institution carries no authored kind; its identity is its emergent structure (roles, ADICO-grammar norms, the function it coordinates, legitimacy, lineage, resources), and any type is recovered only as a derived, non-authoritative etic descriptor for legibility, with the culture's own emic name beside it, the same etic-and-emic split the engine uses for artifacts (Part 41) and concepts (Part 33). What an institution coordinates is an emergent blend over an institution-function substrate, a data registry of function axes that is the etic floor of what coordination can be about, sibling to the value and semantic substrates and authorable under Principle 9 as an affordance rather than an outcome; this is where the deepest per-race difference enters, since an exotic people can carry exotic function axes and so crystallize institutions structurally unlike any other race's. Membership-gating and succession are emergent norms, not authored sub-enums. The derived descriptor is a fixed-point polythetic match against a library of descriptive recognition templates that recognize but never generate or constrain, a configuration matching none simply reads as generic, and the owner may ship no templates at all and the engine still runs. Cross-system reasoning is over function and capability, not category, which is strictly more expressive than the old enum. Comparison reuses the value and language distance machinery as an institution-distance over the shared substrate. The model is integer and order-independent with counter-based RNG, and carries to the aggregate tier as a conserved feature vector. The mechanism is fixed Rust; the function-substrate axes, the recognition templates and their feature weights, and every threshold are data (Principle 11). What is reserved for your calibration, surfaced rather than fabricated, with its basis given: the membership of the institution-function substrate axes themselves (basis: the functional domains a given world distinguishes, with force, the sacred, exchange, knowledge, and care a starting menu and not a fixed fact, and exotic axes added per race); the feature weights in the similarity and distance metrics (basis: which features the owner treats as diagnostic of sameness); the recognition threshold (basis: the intended trade between over-labelling a novel form and falling back to a generic description); the crystallization thresholds and rates by which a recurring coordination pattern becomes an institution (basis: the intended institution-formation cadence in playtest, the hardest of these to set); the recognition template library itself, the guild-like and church-like prototypes (basis: the owner's authored observer vocabulary, flagged as the engine's approximate understanding, never authoritative and never generative); and the legitimacy coefficients, which belong to Part 9 and are referenced here. The reserved list is in the audit log. The honest limits stand: the descriptor can mislabel a novel institution near a threshold, which is why the similarity is surfaced and the generic description is preferred near the boundary and why the descriptor is always the observer's reading and never ground truth (the legibility caution of Scott); calibrating crystallization to fire at the right rate is the real open tuning problem; the function-axis set can have blind spots that collapse distinct institutions together in distance space until the axis set is revised; and this couples to still-open items, R-RELATION for how membership, role, and lineage edges are typed without a back-door taxonomy, R-INFRA for the identical etic-and-emic question applied to the buildings institutions inhabit, R-DOMAIN for the sacred content that feeds the belief axis, and R-TIER-CONSIST (now resolved in Part 54) for the feature-conservation invariant this design leans on, which it carries as a declared conserved projection.

Governance emerges from how a people came to organize power, and the form it takes depends on the culture: a race innately disposed to hierarchy and a race innately disposed to consensus, given the same pressures, settle into different structures, and both can later reform, a chieftainship hardening into a monarchy, a monarchy overthrown into a council, as legitimacy shifts. Legitimacy is itself a belief held by the governed (Part 9), so it can be earned, inherited, propagandized, or lost, and a ruler whose legitimacy collapses faces revolt. Law emerges as a culture's taboos and repeatedly enforced norms (Parts 10, 21) crystallize into an explicit, enforced code.

The economy emerges the same way, past the markets and prices already specified (Part 19). The invention of money itself is a discovery a culture may or may not make, choosing some scarce, durable, divisible good as a medium of exchange once barter's friction grows painful enough; from money come credit and debt, and from those, banking. The division of labour, a labour market, taxation and the public works it funds, and economic classes with the dynamics of wealth concentration all emerge from agents trading, specializing, and accumulating under their institutions, producing booms and busts as a property of the system rather than a scripted event. None of this is guaranteed: a culture that never invents money, or never centralizes power, is a valid and interesting history, and the engine lets it happen rather than forcing every people up the same ladder.

---
## Part 37: Recursive Theory of Mind

The belief system (Part 9) models what an agent believes about the world. Recursive theory of mind models what an agent believes other agents believe, and what they believe about that, nesting belief inside belief, which is the known-hard piece that turns gossip into politics. With it, an agent can hold a model of another's knowledge, intentions, and trust, and reason about it.

```rust
pub struct MentalModel {
    pub of: StableId,                      // whose mind this is a model of
    pub beliefs: BeliefStore,              // what I think THEY believe (recursively, a BeliefStore)
    pub intentions: SmallVec<[Intent; 4]>, // what I think they are trying to do
    pub trust: Fixed,                      // how much I think they trust me, and I them
    pub depth: u8,                         // nesting level, bounded for tractability
}
```

The capabilities this unlocks are the political ones. Deception becomes meaningful, because a liar (Part 9) can model whether the target will believe the lie, and a target can model whether the speaker is likely lying, so a lie can be seen through. Negotiation and alliance become reasoned from modeled intentions rather than raw stats, an agent weighing what another wants and will accept. Trust and alliance also weigh value alignment, computed by the same `value_distance` (Part 21) over the believed profile carried in this model rather than the other's true values, so feigned agreement reads as closeness until evidence (Part 9) corrects the belief and the distance opens back up. Intrigue and spying become possible, an agent acting to learn or to shape what another believes. Nesting is bounded to a small depth, because perfectly recursive belief is intractable, but even two or three levels, I know that you know that I know, is enough for betrayal, brinkmanship, and trust to become real, and it is the layer that makes the social and governance systems (Part 36) feel political rather than mechanical.

> Needs research, item R-TOM-UPDATE in the research backlog. This part specifies the mental-model structure (a nested belief store per modeled mind, bounded depth) and its consumers (deception, negotiation, and trust over the believed profile), but not the rule that populates and updates the nested store, and without that rule the model defaults to projection, attributing the agent's own beliefs or the ground truth to the target, which cannot represent a false belief and collapses under any false-belief or deception probe, the failure of relaying one's own corpus rather than applying recursive theory of mind. A session must specify how the nested store is populated and updated from second-order evidence about the target's epistemic access, what the agent saw the target witness, what the target was told, what the target said, and what the target could reach, reusing the resolved evidence engine (R-EVIDENCE, Part 9) applied to whether the target believes a thing rather than whether the thing is true, so the model provably diverges from projection and supports false belief (the Sally-Anne competence) and seen-through deception; this includes the prior content of a fresh nested store and how it departs from a common-ground default as evidence accumulates, how the gossip and testimony loop (Part 9) updates the hearer's model of the speaker's belief alongside the hearer's own belief, and how the cost is bounded, since an evidence-driven nested store per modeled relationship is expensive and must be level-of-detail-gated like belief itself and conserved across promotion and demotion (R-TIER-CONSIST, Part 54). The session must also pin what the model assumes at the depth bound, beyond two or three levels, so deeper nesting degrades to a principled default, mutually-known common ground or projection from the level above, rather than fabricating deeper beliefs. Ground in the false-belief and theory-of-mind literature and in computational and Bayesian theory of mind, in epistemic logic on bounded recursion and common ground, and in game precedent for nested belief and deception. The update must be integer and fixed-point with counter-based RNG like the first-order engine, and parametrised by the genome acuity and epistemic stance that already shape first-order inference (Parts 25, 28) so a sharper or more skeptical mind models others differently. Flagged, not changed.

---

## Part 38: Gods and Divinity

Whether the gods are real is a tunable starting variable (Part 40), and the choice profoundly changes the belief-reality loop (Part 27). With divinity switched off, all religion is purely emergent belief with no real referent: gods are ideas the cultures invent, and every religious belief is, in ground truth, false, however powerfully it shapes behaviour. With divinity switched on, deities are real entities with domains and powers that can act on the world within tunable limits, and now some religious beliefs are true, divine action is a real causal force, and the cultures are partly right about their cosmos.

```rust
pub struct Pantheon {
    pub exists: bool,                    // the master starting toggle
    pub deities: Vec<Deity>,
}

pub struct Deity {
    pub id: DeityId,
    pub domains: SmallVec<[Domain; 4]>,  // what it governs: storm, harvest, death, war, craft...
    pub power: Fixed,                    // tunable: how much it can do
    pub intervention: InterventionModel, // how often and how directly it acts, and at what cost
    pub disposition: SmallVec<[(ValueAxisId, i8); 16]>, // over the world's value axes: what it rewards
    pub known_as: HashMap<CultureId, StableId>, // the often distorted form each culture worships
}
```

> Needs research, item R-DOMAIN in the research backlog. `Domain` (storm, harvest, death, war, craft...) is a closed enum, yet what a god governs should be as open as the world's own concerns and physics, including domains no real pantheon has. A session should decide whether divine domains become data-defined, and how they relate to the data-defined value axes (Part 20) and belief domains (Part 28), since the three overlap.

When divinity is on, a deity acts through the same channels the rest of the engine uses: its interventions are events in the log (Part 7) and so become history and, distorted, folklore (Part 9); it can grant or withdraw the divine magic that is one source of the magic system (Part 34); it can answer or ignore the rites of its faithful (Part 10); and its disposition shapes which cultures it favours and which it smites, measured by the same `value_distance` (Part 21) between the deity's disposition profile and a culture's, with whether favour weighs the direction of disagreement or only its magnitude left as a reserved choice (R-VALUE-METRIC). The tunable power and intervention parameters let a world range from gods who shaped creation and then went silent, to gods who walk and war, to no gods at all, all from start variables. The deep interest is in the gap between divine truth and mortal belief: even with real gods, the cultures' picture of them is a distorted projection, each culture worshipping its own remembered form, so a people can be both right that their god exists and badly wrong about what it wants, which is the belief-reality loop at its richest.

---

## Part 39: The Text Renderer

Procedural prose from grammars tends toward repetition, and the chronicle of this world is its primary output, so the wording deserves better. The text renderer is a local language model used strictly as a renderer of non-authoritative flavour text, the legends, descriptions, and the wording of rumours and inscriptions, kept entirely out of the deterministic core exactly as the GPU fields are. It writes the prose; it never decides what is true.

The quarantine is the whole design. The simulation produces the facts, an event, a life, a war, an artifact's provenance, a given people's distorted belief about any of them, all of it deterministic and integer-valued in the canonical core (Part 3). The text renderer takes those structured facts as input and produces readable prose, and its output flows only to the screen, never back into canonical state. This mirrors the GPU rule (Part 5): the model is a renderer whose output is approximate and need not be reproducible, so determinism is untouched, the chronicle can be regenerated at any time from the same facts, and a person editing or disabling the renderer changes nothing about the world's history.

```rust
/// Input is structured, sourced from the history database and belief stores; output is prose only.
pub struct RenderRequest {
    pub facts: StoryInstance,             // from the dramaturg (Part 29) or a history query (Part 30)
    pub viewpoint: Option<StableId>,      // render from this people's belief, not ground truth (Part 9)
    pub register: Register,               // saga, chronicle, rumour, inscription, epitaph...
    pub language_surface: Option<LangId>, // names and terms in-world, glossed to English (Part 33)
}
// The renderer is a presentation service; canonical state never reads its output.
```

It pairs with the language framework (Part 33): names, terms, and sayings come through in their in-world language with the English gloss the translatability requirement demands, while the connective prose is written in English, so a saga reads as English narrative studded with the authentic words of the people it concerns. The cost is real, running a model and keeping it strictly quarantined, and the discipline is the same discipline that protects determinism everywhere else, but for a world whose output is its history, the quality of the writing is not a side issue.

---

## Part 40: Data-Driven Definitions and Modding

Everything the world is made of, and every starting variable, is defined in data rather than code: the races and their intrinsic capabilities and intrinsic beliefs (Parts 20, 28), creatures, plants, materials and their properties, the magical laws (Part 34), anatomies and fluids (Part 35), the pantheon and whether it is real (Part 38), the climate and the map seed, and the tuning of every system. The deterministic core reads these definitions; it does not hardcode them.

```rust
pub struct WorldDefinition {              // the complete moddable starting state
    pub seed: u64,
    pub races: Vec<RaceDef>,              // capabilities + IntrinsicBeliefs (Parts 20, 28)
    pub creatures: Vec<AnimalSpeciesDef>, // (Part 17)
    pub plants: Vec<PlantSpeciesDef>,     // (Part 16)
    pub materials: Vec<MaterialDef>,      // (Part 19)
    pub magic: MagicLaws,                 // (Part 34)
    pub anatomies: Vec<AnatomyDef>,       // bodies and fluids (Part 35)
    pub pantheon: PantheonDef,            // gods, real or not, and their tuning (Part 38)
    pub climate: ClimateParams,           // (Part 18)

    // Being-attribute and behaviour registries: nothing about a being is hardcoded. Races (Part 20)
    // select from these; a race has only the drives, traits, value axes, and genes it is given.
    pub drives:        Vec<DriveDef>,        // needs; a race omits any it lacks (Parts 8, 20)
    pub traits:        Vec<TraitDef>,        // personality axes, not a fixed Big Five (Parts 8, 20)
    pub value_axes:    Vec<ValueAxisDef>,    // the value dimensions cultures reason over (Parts 10, 21)
    pub value_struct:  Vec<ValueStructure>,  // per race, parallel to `races`: the topology of its value
                                             //   space, compiled offline to a ground metric; Independent
                                             //   reproduces plain Euclidean distance (Part 21)
    pub etic_substrate:EticSubstrate,        // shared axes for cross-race value distance; each race's
                                             //   emic axes carry an EmicProjection onto it (Part 21)
    pub genes:         Vec<GeneDef>,         // heritable axes (Part 25)
    pub schemes:       Vec<GeneticScheme>,   // per-race genetic and reproductive schemes (Part 25)
    pub semantic_substrate: SemanticSubstrate, // the grounding axes concepts are regions over (Part 33)
    pub phonetic_substrate: PhoneticSubstrate, // the articulatory features sounds are built from (Part 33)
    pub trace_kinds:    Vec<TraceKindDef>,     // perceptible event consequences and the beliefs they imply (Part 9)
    pub evidence_weights: EvidenceWeightTables,// signed weights from each evidence type toward each hypothesis (Part 9)
    pub hypothesis_frames: Vec<HypothesisFrameDef>, // candidate value sets per question kind (Part 9)
    pub absence_schedules: Vec<AbsenceScheduleDef>, // escalation states and thresholds for inference from absence (Part 9)
    pub inst_functions: InstitutionFunctionSubstrate, // the etic axes institutions are emergent blends over (Part 36)
    pub inst_templates: Vec<InstRecognitionTemplate>, // descriptive, non-generative recognizers for observer legibility (Part 36)
    pub axiom_axes:    Vec<AxiomAxisDef>,    // the foundational stance axes races can hold (Part 28)
    pub source_modes:  Vec<SourceModeDef>,   // epistemic source vocabulary: tradition, evidence, ... (Part 28)
    pub locomotion:    Vec<LocomotionDef>,   // movement modes (Part 20)
    pub actions:       Vec<ActionDef>,       // the utility-AI action set and its considerations (Part 8)
    pub curves:        Vec<CurveDef>,        // named response curves referenced by considerations (Part 8)

    pub tuning: SystemTuning,             // rates and thresholds across the systems
}
```

This serves two ends. It speeds your own iteration enormously, because changing a race's lifespan, the lethality of a plague, the magical laws, or whether the gods are real is a data edit rather than a recompile, and it lets you run many different worlds from the same engine by varying only the definition. And it opens the world to modding, which is much of what gave Dwarf Fortress its longevity, since others can define new races, creatures, materials, magics, and starting conditions without touching the engine. Because it touches every system, each reading its definitions from the world definition, it is a stance to commit to early, as retrofitting data-driven definitions onto hardcoded systems is painful. The determinism guarantee extends to it cleanly: a world is reproducible from its definition and seed, which is exactly the property that makes shared worlds and shared mods work.

---

## Part 41: Technology as an Emergent Design Space

This part was a flagged open problem in earlier drafts. It is now specified. The architecture below draws on four bodies of research, design theory's function-behaviour-structure ontology, artificial life's physics-based evolution and quality-diversity search, cultural-evolution theory, and procedural-content-generation practice, and it resolves the hard question at the centre of the project: how to let cultures invent, refine, and converge on technologies without the designer steering them toward predetermined designs. The deepest generative reach of the system remains partly unsolved, and that limit is stated plainly here and in Part 61.

**Physics is the input; culture is the output.** This is the thesis the whole system is built to honour. The engine authors physics, and technology is what the cultures produce as output when their agents press that physics against their needs. The only bias that can enter is humanity's understanding of physics and our implementation of it, which is an acknowledged and bounded source of bias rather than a hidden one, and that is the point: by routing every technological outcome through a physical substrate, designer intent has no other way in. A culture's tools, weapons, and methods are a product of what is physically true and of who the culture is, never of what the author wanted them to build.

**Functions are not handed out; intents are conceived.** The representation literature encodes an artifact by the functions it must perform rather than by its form (Gero 1990), and this project adopts that, with one critical change that answers the steering concern directly. A function is not a slot the engine hands every culture to fill. A people does not begin knowing that a cutting tool exists, or that cutting is a thing one can do to a problem. The engine never seeds that knowledge. What the engine provides is physics; what must emerge, in a mind, is the thought that some makeable thing, used some way, might meet a need. A function, in this engine, is an intent an agent conceives rather than a category the designer enumerated.

**The lifecycle of a technology.** A technology passes through the following stages, and the engine authors only the first and the fourth, both of which are physics.
1. A need arises, emergent from survival, resources, or conflict (Parts 15, 19, 24): a people must kill prey at a distance it cannot reach, or part wood it cannot break by hand.
2. An agent conceives an intent. This is a cognitive innovation event, gated by the agent's intelligence and the pressure of the need, weighted by what it has observed of the world's physics, a sharp stone that once cut, a falling rock that once crushed, and driven by counter-based RNG so it is reproducible (Part 3). The agent forms a hypothesis: a hard thing with a thin worked edge, swung, might part the wood. No catalogue of functions is consulted, because none exists; the intent is constructed from need and observation.
3. The intent is articulated as a design, a composition of form, material, and joining technique with an intended use, and is fabricated through the made-world systems (Parts 42 through 44).
4. Physics evaluates the design. The world simulates the artifact in use and measures whether it performs (Sims 1994): does this edge geometry and this material concentrate enough stress to part the wood. Physics is the teacher, and it can refute as easily as confirm, so a conceived design that does not work is felt to not work.
5. The culture refines and searches. Variants are tried, and the search is divergent rather than goal-directed (below), so the people explore the space of workable designs rather than marching toward a single answer the designer planted.
6. The design and the intent behind it transmit, drift, are lost, and re-converge across peoples (below), through the same machinery as belief and knowledge (Parts 9, 23).
7. The result is made legible to the observer through an objective functional description derived from the measured physics, held separately from the culture's own name and conception (below).

**Representation.** The conceived-intent front end (Gero 1990, reframed as above) emits a combinatorial phenotype: an artifact is a composition of form primitives, a material (Part 19), and a joining technique, integer and enum encoded so it is deterministic-friendly and legible. A generic shape grammar may elaborate form detail, but the grammar is kept primitive, shapes and joins and material assignment, so that it encodes no designer idea of a finished object; the search does the inventing. Indirect neural encodings that compactly capture symmetry and repetition (compositional pattern producing networks and their kin, Stanley 2007; generative representations evolved as parametric L-systems, Hornby and Pollack 2002) are powerful and are noted as an option, but they are floating-point and so are quarantined to offline use (below). The governing tension is expressivity against validity, a space wide enough to surprise yet constrained enough to yield valid, evaluable objects (Togelius et al. 2011; Shaker et al. 2016).

**The physics substrate and the completeness requirement.** Because physics is the only authored layer and the sole source of selection pressure, the engine needs a grounded physical model that is as complete as it can be made: the set of physical quantities and interactions against which any conceived artifact is evaluated. This is the single most important and most demanding piece of content in the project. The functional outcomes a player would name, reach, penetration, chopping, are not categories the engine stores; they are measured consequences of underlying physics, force and momentum and contact area and leverage, material hardness and toughness and fracture, friction and buoyancy and thermal transfer. Chopping effectiveness is not a field; it is what the physics reports when a particular worked edge of a particular material meets a particular wood at a particular speed. The completeness of this physical model bounds what can be invented: a physical interaction the engine does not model is a technology no culture can discover, so the reach of the whole emergent-technology system is exactly the reach of its physics. The substrate must therefore be assembled deliberately and, by Principle 9, audited for hidden outcome bias, since a physical law that quietly encodes the designer's preferred answer is a steering leak wearing the costume of physics.

**Evaluation and convergence without a target.** Performance is scored by physics or use proxies, computed in fixed-point closed form rather than by full finite-element analysis (Part 3): contact pressure as force over edge area, bending stress from a section modulus, buoyancy from displaced volume. The search that explores designs is a quality-diversity or novelty search rather than an objective-driven optimizer. Novelty search abandons the objective and rewards behavioural difference (Lehman and Stanley 2011); MAP-Elites keeps the best design in each cell of a behavioural space and so illuminates the whole space rather than climbing to one peak (Mouret and Clune 2015); the case for why this reaches functional solutions a direct objective would miss is made in full by Stanley and Lehman (2015); and co-evolving the problem with the solution, so the niche is the implicit objective, is the structure of POET (Wang et al. 2019). The reason this yields convergence without a target is the same reason convergent evolution does. Real organisms occupy a small, clustered region of the space of theoretically possible forms because shared functional constraints and selection create attractors (Raup 1966 on shell morphospace; McGhee 2007), and the eye evolved independently on dozens of separate occasions with nothing aiming at an eye (von Salvini-Plawen and Mayr 1977). Define an artifact morphospace the same way, and independent cultures under shared physics and similar needs flow into the same physical attractor, the sword-shaped region, with no target steering them there. This is the proven mechanism for convergence without steering, and it is adopted as such.

**The Steering Audit.** Principle 9 is enforced by three mechanisms, and together they are the in-engine embodiment of auditing the input and not only the output, and of proving a result before trusting it.
1. The content gate. Every entry in a world definition (Part 40) is classified as a physical affordance or a cultural outcome, and the loader refuses authored cultural outcomes. An attempt to define a sword type, to seed a culture with the knowledge that cutting tools exist, or to hand a people a finished recipe, is rejected by the engine. The designer cannot place a technology; the designer can only place physics.
2. The provenance requirement. Every artifact, design, and technique carries a causal provenance back through transmission to the agent who conceived it and the need that drove the conception (Part 7). An artifact that exists with no such provenance appeared from nowhere, and a thing from nowhere is a steering leak the engine can detect and flag.
3. The convergence audit. As a standing test in the spirit of the determinism harness, the engine runs many independent worlds from different seeds under identical physics and measures where cultures converge and diverge in artifact morphospace. The invariant is that convergence must be explained by physics, cultures converging only where physics forces a single answer, and diverging in name, form, and method everywhere physics underdetermines it. Convergence that physics does not force is the signature of a designer-baked attractor, and the audit surfaces it. This mechanism is empowered to override the designer, the author of this document included, because the entire value of the system is lost the moment a human vision is pressed onto the world in place of what the world would have found for itself.

**Cultural transmission of designs.** Designs and the intents behind them spread, drift, are lost, and re-converge through the same transmission machinery as belief and knowledge (Parts 9, 23), which is what produces the cross-cultural sword: independent lineages under shared physics converging on the same workable form. Copying fidelity is not a universal constant. The often-cited three percent figure, the Weber fraction below which a human cannot perceive a difference in a judged length (Eerkens and Lipo 2005), is a grounded reference for one specific case, a human-like maker copying a visually judged dimension, and a reasonable default there, but this world is not that single case. Fidelity must be a function of the copier's perception and memory, which vary by race (Part 20); of the transmission medium, since an oral demonstration, a hands-on apprenticeship, and a written specification (Part 49) carry very different fidelity; of the artifact's complexity, since a many-part design degrades faster than a simple one; and of the skills of teacher and learner (Part 35). Writing collapses copying error toward zero, a short-memory race inflates it, and complexity amplifies it, so the Weber fraction is one calibration point and not the law. The accumulation of small copying errors makes independent lineages diverge over time (Eerkens and Lipo 2005), high-fidelity transmission plus innovation makes traditions ratchet upward in capability across generations (Boyd and Richerson 1985; the ratchet of Tomasello 1999), biased transmission by prestige, conformity, or content decides which variants spread (Boyd and Richerson 1985), and a population that crashes below the practitioners a technique needs can lose it outright, the mechanism behind real technological loss (Henrich 2004 on the Tasmanian case, already reflected in Part 23). An artifact tradition can be recorded and analysed as an evolving lineage with the methods of cultural phylogenetics (Howe and Windram 2011), a way to surface the history of a design for the legends, noted here as an option rather than a dependency.

**Readability: physics in, culture out, made legible.** Every artifact carries two descriptions, and keeping them separate is what makes a world of invented technologies legible. The etic description is objective and is derived from the measured physics: this object concentrates force on a hard worked edge, so it is, as a matter of physics, a cutting implement. The emic description is the culture's own, its name for the thing, its conceived purpose, its form, and its method. The etic tag is what lets the observer, and the engine, understand that a given object is a sword to the people who made it even though they call it a ketz and forge it a way no other culture does. Baseline descriptions are rendered by a deterministic grammar over the structured record (Tracery and its author-focused descendants, Compton et al. 2015), and richer flavour text may be produced by the quarantined language model (Part 39), which reads the same structured record and never writes canonical state. The proven precedent for surfacing generated artifacts in a consistent voice is Caves of Qud, which renders generated history through artifacts in the voice of its hand-written corpus and marks procedural provenance distinctly (Grinblat and Bucklew 2017). The payoff is the thesis restated: physics is the input and culture is the output, the bias is only humanity's understanding of physics and our implementation of it, and that bias is surfaced openly rather than hidden in a catalogue of pre-made things.

**Determinism rules for the design space, adopted verbatim.** The following ruleset governs what runs in the authoritative core and what is quarantined, and it is the same quarantine discipline the GPU fields and the text renderer already follow (Parts 4, 39).
- Deterministic-friendly, kept in the authoritative core: the combinatorial part-based representation, which is enums and integers; the conceived-intent front end, which is symbolic; the physics and use proxies, computed in fixed-point (Part 3); MAP-Elites and novelty search, which are selection, archive bookkeeping, and a behavioural descriptor, all expressible in integer and fixed-point arithmetic and driven by counter-based RNG; and the cultural-transmission layer, which is scaled-integer drift, biased-transmission weights, and diffusion over the contact graph.
- Determinism hazards, quarantined: floating-point physics is non-reproducible across processors, compilers, and SIMD widths, so full rigid-body or finite-element simulation is kept out of the authoritative path in favour of the fixed-point proxies; where richer physics is wanted, it runs on the GPU as a non-authoritative approximate field that is quantized before it can touch canonical state (Part 5), or it runs offline during worldgen with its results baked deterministically into integer data. Neural indirect encodings and the language model are quarantined entirely: used offline to pre-bake design genomes that are then frozen as integer data, or used as non-authoritative presentation whose output is cached by the hash of the structured record and never parsed back into simulation state.

**Recursive composition: a validated module becomes a part.** The representation above encodes one artifact as a composition of leaf primitives, a material, and a joining technique, which is a single level. Open-ended depth, a machine that is a composition of components that are themselves compositions nested arbitrarily deep, requires that a conceived-and-validated artifact or technique be able to become a reusable building block that a later intent references as a component. A technology is therefore a content-addressed composition node, an integer and enum record whose identity is a counter-based hash of its canonical contents, so the same design has the same id on every machine and every node is deduplicated and memoisable. A node is either a leaf, carrying the existing primitive-plus-material-plus-join payload and bottoming out there, or a composite, carrying references to child nodes by stable id, the assembly-level material and join that fasten them, and the parameters bound at this composition. A child is referenced by id rather than by category, so a composite depends on a specific validated module that must already exist and be stable, which is what makes prerequisite depth real rather than declared, while a structural interface signature lets an agent search for any module that satisfies a contract rather than for a named part. The conceived intent the node carries is the emergent need-driven intent of the lifecycle above, a reference to what the agent is trying to achieve, and never an authored enum of allowed technologies, so what can be invented is not capped and two peoples conceive different things.

```rust
pub struct CompositionNode {
    pub id: u64,                       // content address: hash(master_seed, canonical bytes, phase)
    pub intent: IntentRef,             // the emergent need-driven intent (lifecycle above), non-authoritative
    pub body: NodeBody,                // a leaf, or a composite of validated children
    pub ports: PortVector,             // fixed-width interface: what it offers and demands at its boundary
    pub param: SmallVec<[Fixed; 4]>,   // parameters bound at this composition (scale, count, ...)
}
pub enum NodeBody {
    Leaf { primitives: SmallVec<[FormId; 8]>, material: MaterialId, joining: JoinId },
    Composite { children: SmallVec<[ComponentRef; 8]>, assembly_material: MaterialId, assembly_join: JoinId },
}
pub struct ComponentRef { pub target: u64, pub transform: TransformId, pub overrides: SmallVec<[Fixed; 2]> }
```

**The interface substrate, the grounding floor of composition.** The port vector and the evaluation channels are a data-defined, extensible substrate of physical interface axes rather than an authored fixed list, the etic floor of what a connection between modules can be about, sibling to the value substrate (Part 21), the semantic substrate (Part 33), and the institution-function substrate (Part 36), and authorable under Principle 9 as an affordance rather than an outcome. A mechanical mount and load rating, energy in and out by form and rate, signal, thermal flux, mass, and envelope are a starting menu and not a fixed fact; a world with exotic physics or a people who channel forces another race cannot carry exotic interface axes, and the reach of the whole composition system is exactly the reach of this substrate, which couples it to the discoverable physics substrate (R-DEEPTECH-PHYSICS). The leaf primitives, materials, and joins are likewise the data-defined, extensible, physics-grounded floor at which the recursion terminates, the structural analogue of the bounded primitive floor beneath the semantic substrate.

**Promotion to a reusable primitive.** A validated module does not become a referenceable part the instant it works, because promoting every working artifact would grow the alphabet the next level searches over without bound and make the search explode. A module is promoted to a reusable primitive only when three gates are passed together. It must pass the physics and use evaluation above a viability threshold, the necessary floor that is not sufficient alone. It must have stabilised in transmission, having spread and re-converged through the same cultural transmission machinery as belief and knowledge above and survived drift and loss over a reserved span, which is encapsulation as stabilisation, since only a design copied faithfully enough to persist is safe to hide behind an interface and build upon. And its reuse must compress the technology corpus, the module being referenced by enough distinct intents that carrying it as one primitive is cheaper than re-expanding it everywhere, which is the description-length criterion that makes promotion prefer exactly the modules that make the next level cheaper to search. The conjunction is what bounds the explosion: validation alone over-promotes, while the stability and compression gates ensure the alphabet grows only by units that are durable and economical, so the branching factor of the next level is set by the count of promoted modules and not by the far larger count of all working designs. Because the stability gate runs through the cultural transmission model, which is already per-culture, promotion is per-culture for free: a people with faithful transmission ratchets quickly and keeps a deep library, a lossy people loses modules and stays shallow, and a people with exotic interface axes promotes structurally unlike parts, so the technological trajectories of different peoples diverge with nothing authored. A module promoted for one intent can be referenced inside an unrelated intent wherever its interface fits, which is exaptation and a deliberate source of cross-domain depth, and it falls out for free from referencing by id and matching by interface.

**Composed evaluation.** The performance of a multi-level system is not the sum of its parts, because cross-cutting couplings are nonlinear and a system carries properties no part has, a structure's resonance, a powerplant's thermal balance, a control loop's stability. Evaluation is therefore a memoised bottom-up pass over the composition graph that computes for each node a fixed-width vector of integer quantities and a viability score, with three mechanisms. Child vectors combine by typed combinators chosen by the interface topology, and the combinator set is itself a data-defined, extensible registry grounded in the physics substrate rather than a fixed list: a limiting-factor minimum where a load path is as strong as its weakest member, a saturating sum where capacities are redundant, a conserved budget that must fit an envelope or a supply, a product of efficiencies down a conversion chain so loss compounds and depth pays its own diminishing returns, with new topologies registering new rules as the physics grows. A small set of whole-system proxies, each a closed-form integer formula over the already-aggregated quantities, captures the emergent properties no child carries, and that proxy set is data and is exactly where the reach of technology is bounded by the physics substrate. An interface mismatch between a child and its parent contract is a scored penalty within an adaptable range and a floored score beyond it rather than a hard reject, so adapters and bodging become emergent intermediate technologies. Each quantity is carried as an integer interval rather than a point, the combinators propagate the bounds cheaply, and a wide interval flags a composition whose couplings the proxies cannot pin down, so the viability gate can require the lower bound to clear threshold and the design neither pretends to full physical simulation nor trusts a fragile sum. Every node's vector is a pure function of its content-addressed id and is computed once and cached, so evaluating a larger system that references a known module reuses the cached result and the cost is in the new nodes alone, which is the reason encapsulation pays off in computation and the reason the deep-time aggregate tier (R-DEEPTECH-SCALE) can carry a culture's technology as a compact library of promoted ids rather than re-searching it.

**Emergent depth, never an authored tree.** Prerequisite depth is a read-out of these rules and is authored nowhere. A composite cannot be conceived-and-validated until the children it references exist and, to be referenced as primitives, have passed the three gates, so the graph of which module uses which is the prerequisite graph and it is generated rather than declared. Each promoted module is a stabilised rung and the next level reaches only as high as the rungs below it are stable, which is the cultural ratchet applied to structure. And because composed evaluation rewards decomposition only where the physics makes a system near-decomposable, depth appears in the domains the physics favours and stays shallow where a monolith wins, so the shape of the technology graph is selected by physics and reuse rather than scripted. This is the line that separates the design from an authored automation game, whose dependency graph and recipe ratios are written by hand node by node: that tiered shape is the output to be reached emergently here, and the test of success is non-templatedness, two peoples from different seeds growing technology graphs of different shape and depth under one physics. If a feature can be met only by writing a table of which components an intent needs, it is out of scope for composition and belongs in the physics substrate.

> Decided and reserved. The mechanism is settled and signed off, and on the owner's condition it is general and per-race differentiable rather than fitted to one technology or one physics. A technology is a content-addressed composition node, integer and enum, that is a leaf bottoming out at the data-defined primitive-material-join floor or a composite referencing validated child modules by stable id; the conceived intent is the emergent need-driven intent of the lifecycle and never an authored enum of technologies. A validated module is promoted to a reusable primitive only when it passes physics validation, stabilises in transmission through the per-culture cultural ratchet, and compresses the corpus by reuse, the conjunction bounding the combinatorial explosion by growing the next level's alphabet only with durable and economical units, and the per-culture stability gate making technological trajectories diverge by people for free. Evaluation is a memoised bottom-up pass that aggregates fixed-width integer interval vectors by typed combinators drawn from a data-defined extensible registry, adds data-defined whole-system emergent proxies, and charges a graded interface-mismatch penalty, with memoisation on the content id letting the deep-time aggregate tier carry a compact library rather than re-search. The interface-axis substrate, the leaf floor, the combinator registry, and the emergent-proxy set are the etic grounding floor of composition, sibling to the value, semantic, and institution-function substrates and extensible with the physics. The mechanism is fixed Rust; the leaf primitives, the interface and evaluation axes, the combinators, the emergent proxies, and every threshold are data (Principle 11). What is reserved for your calibration, surfaced rather than fabricated, with its basis given: the maximum composition depth and node count per evaluation (basis: the per-tick budget and the depth at which marginal proxy gain falls below noise, a determinism-and-performance bound and not a realism one); the viability threshold (basis: the failure boundary the material and physics data already define, such as pressure exceeding yield); the stability span and drift-similarity radius of the transmission gate (basis: the drift and loss rates the belief and knowledge transmission subsystem already uses, set equal to them for consistency); the reuse-and-compression threshold (basis: the description-length-decrease criterion, with the integer reuse count its cheap surrogate, calibrated so promoted-library growth fits the aggregate-tier memory budget over the long run); the interface-mismatch penalty curve and adaptable range (basis: the loss physics of the relevant interface axis and where an adapter becomes physically impossible); the emergent-proxy weights (basis: the physics substrate's units and criticality for each aggregate quantity, not aesthetic tuning); the interval-width rejection threshold (basis: the approximation error acceptable at the aggregate tier versus the per-tick tier); and the evaluation channel set itself (basis: exactly the quantities the physics substrate can expose, which defines the reach). The reserved list is in the audit log. The honest limits stand: near-decomposability is an assumption the interval widths flag but cannot resolve, so strongly integral technologies are under-served and that is a real reach limit; the compression gate's integer reuse count is a cheap surrogate for an expensive description-length ideal and can promote a near-duplicate or miss a deep shared sub-structure; the reach of the whole system equals the reach of the interface substrate and the emergent-proxy set, so a physical interaction the substrate cannot evaluate is a capability ceiling that is surfaced rather than faked; and the contact with how a new technique first originates is the unresolved Inconsistency 5, since the stability gate presupposes an answer to where a technique comes from, so the promotion of a new technique to the join space may need a technique-specific variant once that inconsistency is settled. This resolves the composition piece of the north-star cluster; the deeper questions of emergent science (R-DEEPTECH-SCIENCE), the discoverable physics substrate (R-DEEPTECH-PHYSICS), emergent prerequisite depth as a standalone proof (R-DEEPTECH-DEPTH), and deep-time aggregate running (R-DEEPTECH-SCALE) remain open.

**Honest limits.** Functional convergence on simple artifacts is well-supported by the literature above. The recursive composition of validated modules into complex, multi-part technologies, a vehicle or a mechanism, is now resolved by the composition mechanism above (R-DEEPTECH-COMPOSE), with the honest caveats that near-decomposability is an assumption rather than a guarantee, so strongly integral technologies are under-served, and that the reach of the whole system equals the reach of its physics substrate and emergent-proxy set; the deeper cluster questions it does not settle, emergent science, the discoverable physics substrate, emergent prerequisite depth as a standalone proof, and deep-time aggregate running, remain open. The choice of descriptor space for the morphospace is itself a subtle authoring decision, since the search illuminates only the dimensions it is given, so descriptors are kept to physical quantities rather than designer aesthetics, and the convergence claim is proven at small scale before the architecture is trusted, per Principle 9 and Stage 14. The full set of caveats is in Part 61.

> Needs research, the north-star cluster (R-DEEPTECH). The climb this limit describes, from simple artifacts to the modern and far-future tier (industry, computers, spaceflight) emerging from the dawn of sentience, is the project's declared north-star ambition, accepted as possibly unreachable and pursued regardless, both for its own sake and because even a partial result is paper-grade. It is documented as a cluster of researchable problems in the audit log: hierarchical and recursive composition so modules become building blocks (R-DEEPTECH-COMPOSE, now resolved in the composition mechanism above and recorded in Part 62.10), the emergence of abstract and mathematical science as an enabler (R-DEEPTECH-SCIENCE), a tiered and deepening discoverable physics substrate (R-DEEPTECH-PHYSICS), emergent rather than authored prerequisite depth (R-DEEPTECH-DEPTH), and running the whole compounding process over deep time at the aggregate tier deterministically (R-DEEPTECH-SCALE). The composition piece is settled; the remaining four are to be taken super-deep when reached.

> Needs research (R-COMMS). A specific instance of this generative-depth limit, and a gap in the belief layer too, is long-range communication: mail, semaphore, the telegraph, the telephone, and a magical sending share one structure, a message moved between distant parties through a medium with a range, latency, capacity, cost, and fidelity, built by institutions (Part 36) and infrastructure (Part 46), feeding information flow into belief (Part 9). The engine has the adjacent pieces but no unifying communication-channel abstraction, so a postal service emerges as transport plus institution while signal networks have no home. This is item R-COMMS in the audit log.

---

## Part 42: The Local Settlement Layer, Jobs, Tasks, and Items

Several of the systems that follow, construction, crafting, mechanisms, logistics, all need the same thing the engine does not yet have: a fine-grained local simulation in which individual agents take jobs, carry and place specific items, and operate workshops, tile by tile, inside a high-LOD settlement. This is the layer Dwarf Fortress and Songs of Syx run, and it is the keystone the made world stands on, because it is what lets things be built and carried rather than abstracted into a statistic. By Principle 1 it runs only in the zoomed-in focus, while the rest of the world stays aggregate.

The atoms are items, jobs, and the agents that do them. An item is a concrete physical object at a location, with a material (Part 19), a kind, a quality, and a history (Part 7). A job is a unit of work that changes the local world, hauling an item, raising a wall segment, working a reaction at a workshop, tending a field, and it has requirements: a worker with the needed skill (Part 35), input items, a tool, a workshop, and a place.

```rust
pub struct Item {
    pub id: ItemId,
    pub kind: ItemKind,
    pub material: MaterialId,            // (Part 19)
    pub quality: Quality,                // from the maker's skill and the design (Part 41)
    pub at: Location,                    // a tile, a stockpile, a pack, an agent's hands
    pub history: SmallVec<[EventId; 2]>, // who made it, who carried it, what it has seen (Part 7)
}

pub struct Job {
    pub kind: JobKind,                   // Haul | Build | React | Farm | Operate | ...
    pub site: Location,
    pub inputs: SmallVec<[ItemReq; 4]>,
    pub tool: Option<ItemReq>,
    pub workshop: Option<WorkshopId>,
    pub skill: Option<SkillReq>,
    pub claimed_by: Option<StableId>,
}
```

The point you care about is that the work is emergent rather than a scripted build order. Jobs are not handed down from a fixed plan; they arise from needs and standing conditions. A hungry settlement generates food jobs, an exposed one generates shelter jobs, a threatened one generates wall and weapon jobs, and a settlement with a surplus and a cultural taste for monument generates the jobs that raise one. What gets built and made is a product of the population's needs, its institutions' decisions (Part 36), and its culture (Parts 10, 28), so two settlements in different places with different values build differently from the same physical possibilities. The decision to raise such a work is itself a governance decision, not a statistic: it is made by whatever authority the settlement evolved (Part 36, through the decision tier of Part 8), so a people with no authority for it never raises one, and the same people under a different government raises something else. A lightweight local scheduler matches available workers to the highest-priority open jobs, and agents path (Part 13) to inputs, carry them, and act, leaving the world and the event log changed.

Because this is the heaviest local system, it is strictly bounded by the focus. A settlement the observer is watching runs its full job and item simulation; a settlement off-screen runs as the aggregate economy (Part 19) and the production statistics (Part 44), its made things and structures summarized rather than placed tile by tile. Promotion and demotion across that boundary use the same machinery as for individuals (Part 11), so zooming into a town instantiates its detailed local state from its summary and zooming out re-summarizes it.

> Resolved (R-TIER-CONSIST), in Part 54. The two canonical tiers of a settlement, the focus-scale job and item simulation here and the aggregate economy off-focus, are reconciled by a lifting operator (instantiate-from-summary) and a restriction operator (re-summarize) under a conservation-plus-seeded-significance-schedule guarantee: re-summarizing a fresh instantiation conserves every declared canonical quantity exactly and is audited, each tier is bit-reproducible given the camera-free significance schedule so watching cannot move the timeline, and a collective undertaking is emitted through the settlement's emergent governing institution (Part 36) rather than a draw over pool demographics. Identical-outcomes agreement is rejected as unattainable for nonlinear dynamics; the guarantee is conservation plus reproducibility, which is what Principles 3 and 10 require. Decided and reserved in Part 54, recorded in Part 62.9.

---

## Part 43: Buildings, Construction, and Local Physics

Buildings are modeled and built at two resolutions, the building analog of Principle 1, and they stand or fall by real structural physics. Off the observer's focus, a structure is a component model: walls, supports, roofs, and rooms as objects with materials and a footprint, light enough to reason about and to hold for a whole world's settlements. In the high-LOD focus, the same structure drops to a tile or voxel model where every tile has a material and a type, which supports digging, mining, fluids, and collapse, at the cost that only the focus can afford. A hybrid that keeps regional structures as components and expands them to tiles only where someone is watching is the fit.

```rust
pub enum Structure {
    Component(ComponentBuilding), // regional scale: walls, supports, rooms as objects
    Tiled(TileGrid),              // focus scale: per-tile material and type; dig, build, collapse
}

pub struct StructuralNode {
    pub material: MaterialId,             // load-bearing properties from the material (Part 19)
    pub supports: SmallVec<[NodeId; 4]>,  // what holds this up
    pub load: Fixed,                      // accumulated load it bears
}
```

Structural integrity is modeled from materials and load. Each piece of a structure has load-bearing properties from its material, requires support, and carries a load, and when load exceeds what the material and its supports can bear, it fails. Undermining by mining (Part 26), siege (Part 24), earthquake (Part 26), or slow decay removes support and brings structures down, which produces the collapses and, later, the buried ruins that Part 26 wants for rediscovery. Stone holds differently from wood, a span needs support beneath it, and a dug-out tunnel can cave in, all from material properties rather than scripted rules.

Construction takes materials, labour, tools, and time. A structure is raised by jobs in the local settlement layer (Part 42): it needs materials delivered, labour with the relevant technique (Part 23), tools, and time, and it progresses as the work is done. What is built emerges from need and culture rather than from a catalogue of building types: a people raises shelter, storage, defense, workshops, places of worship, and monuments because it needs or values them, in forms shaped by its materials, its environment, and its aesthetic (Part 51), so the engine never hands a culture a fixed set of blueprints to unlock. Buildings decay without maintenance, take damage in war and disaster, and are abandoned when a settlement falls, leaving the ruins the world remembers.

Local physics at the focus scale is the tile-resolution counterpart to the regional field grids (Part 5), and it interacts with the built environment in ways the coarse fields cannot. Fluids fill and pressurize dug and built spaces, water flooding a mine, a cistern holding a head of pressure; fire and heat spread through structures by their materials, a timber hall burns where a stone one does not; cave-ins drop unsupported tiles; and items obey enough physics to fall, float, burn, and be swept away. This runs only in the focus, where a flood or a fire or a collapse is being watched, while the rest of the world uses the regional fields, the level-of-detail principle applied to physics.

---

## Part 44: Production, Supply Chains, and Storage

Production is the connective tissue that makes the made world a thing built rather than a statistic: raw deposit becomes processed material becomes component becomes finished good, each step a transformation needing a technique, a tool or workshop, an energy source, and labour. It turns the economy (Part 19) from the trading of raw resources into a production economy, and it is the precondition for designs (Part 41) and buildings (Part 43) to be fabricated. This is the core loop Dwarf Fortress runs, generalized and kept emergent.

A production step is a physical transformation grounded in materials and technique rather than a rung on a progress ladder. The engine describes what is physically possible, ore plus fuel plus a furnace and the smelting technique yields metal, metal plus a forge and the smithing technique yields a tool, and which transformations a culture performs, and at what scale, emerges from its demand, its resources, its techniques (Part 23), and its institutions' choices (Part 36).

```rust
pub struct Recipe {
    pub inputs: SmallVec<[ItemReq; 4]>,  // materials and components (Parts 19, 42)
    pub technique: TechniqueId,          // the know-how it requires (Part 23)
    pub workshop: Option<WorkshopId>,
    pub tool: Option<ItemReq>,
    pub energy: EnergyReq,               // fuel, or mechanical power (Part 45)
    pub labor: Fixed,
    pub outputs: SmallVec<[ItemStack; 2]>,
}
```

A supply chain is the graph of these transformations from raw to finished, and because each step has real inputs, a real workshop, and real labour, the chain creates the interdependence that drives the economy and history: a smith needs ore and fuel and a forge, the ore needs miners, the fuel needs woodcutters or coal, and a break anywhere starves the end of the chain. Where the chain crosses settlements it becomes trade (Part 19) and rides the transportation network (Part 47). The division of labour the economy needs (Part 36) is agents specializing in steps of these chains.

Storage and its failure are part of the same flow. Granaries, warehouses, and stockpiles hold the outputs; goods spoil at rates set by their kind and their conditions; and carrying capacity is bounded by storage and by transport (Part 47). This layer is what makes famine, sieges, and trade disruption bite mechanically rather than abstractly: a city fed by an imported-grain supply chain starves when the route is cut or the granary burns, and that starvation drives the revolts (Part 36), the migrations (Part 17), and the fall of sites the legends record. None of the structure is scripted: which goods flow, how much is stored, and how fragile a settlement's chains are all emerge from what the culture produces and how it has organized itself.

---
## Part 45: Mechanisms and Mechanical Power

Mechanical power is a transmittable resource, and modeling it opens water wheels, windmills, gears, pulleys, mills, pumps, cranes, powered workshops, and siege engines as things civilizations build and depend on. The capability is power sourced, transmitted, and consumed over a network.

```rust
pub struct PowerNetwork {
    pub sources: Vec<PowerSource>, // water wheel (Part 18), windmill (Part 18), muscle or animal (Part 17)
    pub links: Vec<PowerLink>,     // axles, gears, belts that transmit power, with losses
    pub sinks: Vec<PowerSink>,     // mills, pumps, cranes, powered workshops
    pub available: Fixed,          // generated minus transmission losses
}
```

Power is generated from the water system (a wheel in a river, Part 18), the wind (a mill, Part 18), or muscle, animal (Part 17) or sentient, transmitted through mechanical links that lose some of it, and consumed by sinks that do work: a mill grinds grain far faster than hand-querns, a pump moves water for irrigation or drains a mine (Part 43), a crane raises construction loads, a powered workshop multiplies a production step (Part 44), and a siege engine arms a war (Part 24). Because power couples to the water and wind systems, a drought (Part 18) can still a watermill and a calm can still a windmill, tying productivity to climate. Whether and how a culture develops mechanical power emerges from its needs, its techniques (Part 23), and its environment, a river-rich land leaning toward water power and a windy plateau toward wind, so it is another grounded possibility a culture selects within rather than a mandated stage.

---

## Part 46: Infrastructure that Edits the Map

Infrastructure is the class of persistent constructions that change the shared layers of the world: roads, bridges, canals, ports, aqueducts, walls, dikes, and terraces. The capability is agents building durable structures that modify the spatial, hydrological, and movement layers, sentient ecosystem engineering at civilizational scale, the deliberate counterpart to the beaver editing the water field (Part 17).

```rust
pub enum Infrastructure {
    Road { path: Vec<ChunkCoord>, quality: Fixed }, // lowers movement cost (Part 13)
    Bridge { span: Vec<ChunkCoord> },               // crosses a barrier
    Canal { path: Vec<ChunkCoord> },                // moves water and boats (Parts 18, 47)
    Aqueduct { path: Vec<ChunkCoord> },             // carries water to dry land (Part 18)
    Port { at: ChunkCoord },                        // enables sea transport (Part 47)
    Wall { line: Vec<ChunkCoord>, height: Fixed },  // changes siege and defense (Part 24)
    Terrace { region: RegionId },                   // makes slopes farmable (Part 48)
}
```

> Needs research, item R-INFRA in the research backlog. Unlike `Structure` in Part 43, which is only a representation distinction, this is a closed taxonomy of construction types. The same principle the user applied to buildings should apply here: infrastructure forms ideally emerge from need and physics (a people that needs to move water against terrain arrives at a channel) rather than being unlocked from a fixed list. A session should decide whether infrastructure becomes data-defined by the layer edits it performs, or emergent through the design-space machinery (Part 41), and how either stays deterministic.

Each piece edits a layer the rest of the simulation reads. Roads and bridges lower movement cost, which speeds trade volume (Part 19), army movement, and migration (Part 13). Canals and aqueducts move water, coupling to the water cycle and to carrying capacity, so an aqueduct can green a dry but valuable site and a canal can open inland water transport. Ports enable the sea transport ships need (Part 47). Walls change the siege calculus of war (Part 24). Terraces make slopes farmable, raising a region's agricultural capacity (Part 48). All of it is built through the settlement and construction layers (Parts 42, 43), decays without maintenance, and is damaged in war and disaster, leaving the ruined roads and broken aqueducts of fallen civilizations. What infrastructure a people builds emerges from its geography and needs rather than from a checklist: a riverine trading culture digs canals, a mountain culture cuts terraces and roads, a besieged one raises walls.

---

## Part 47: Transportation and Vehicles

Transportation determines what is reachable, and modeling it as vehicles with capacity, speed, and range transforms logistics, trade, war, and exploration. Ships are the highest-impact case, because without them oceans are barriers and the world is a set of isolated landmasses; with them come sea trade, exploration (Part 50), naval war, and overseas colonization, redrawing the map of what civilizations can touch.

```rust
pub struct Vehicle {
    pub kind: VehicleKind,        // Ship | Boat | Cart | Wagon | PackAnimal | Mount | Airship...
    pub capacity: Fixed,          // freight it carries (Part 44 goods, Part 24 troops)
    pub speed: Fixed,
    pub range: Fixed,             // before resupply (Part 44)
    pub medium: Medium,           // Water | Road | OpenLand | Air
    pub built_from: Vec<ItemReq>, // a made thing (Parts 41, 44)
}
```

Vehicles set the capacity and reach of every flow that moves over distance. Ships and boats open water, the sea and the canals of Part 46, carrying bulk freight cheaply and projecting force as navies. Carts and wagons set land freight capacity and want roads (Part 46) to be efficient. Pack animals and mounts come from domestication (Part 17), mounts giving speed and the cavalry that changes battle (Part 24). Fantastical mediums, airships and flying mounts, ride the same capacity, speed, and range model. Because freight capacity and range bound the supply chains and storage of Part 44, transportation decides whether a distant resource is worth exploiting, whether an army can be supplied far from home (Part 24), and how far trade and colonization can reach. Which vehicles a culture develops emerges from its environment and needs, a coastal people building ships and a plains people breeding mounts and building wagons, so the transportation a civilization runs on is selected from grounded possibilities rather than granted.

---

## Part 48: Agriculture as a Developed Practice

The doc has soil and carrying capacity (Part 16) and the domestication of animals (Part 17), but not the invention and improvement of farming itself, which is the hinge the whole arc from the dawn of sentience turns on. The shift from foraging to cultivation produces a food surplus, the surplus frees labour from food production, freed labour specializes (Part 44) and concentrates into settlements, and settlements grow into the cities and civilizations the rest of the made world serves. This may be foundational rather than optional, because it is what lets a few sentient bands (Part 28) become a civilization at all.

Agriculture is a developed practice, grown rather than granted. Crops are wild plants (Part 16) brought under cultivation and improved by selection over generations into higher-yielding cultivars (Part 25), and methods improve from simple cultivation through irrigation (coupling to the water system and to canals, Parts 18, 46), crop rotation that manages soil fertility (Part 16), terracing that opens slopes (Part 46), the plough, and fertilizer.

```rust
pub struct AgriculturalPractice {
    pub crops: Vec<CultivarId>, // domesticated and bred from wild plants (Parts 16, 25)
    pub methods: MethodSet,     // irrigation, rotation, terracing, plough, fertilizer...
    pub yield_per_labor: Fixed, // rises with crops and methods; sets the surplus
}
```

Each improvement raises yield per unit of labour and so the size of the surplus, which sets how much of a population can do something other than farm, and that fraction gates the growth of specialization, institutions (Part 36), and monuments. A culture that invents irrigation in a dry but fertile basin, or terracing on its hillsides, can support a density and a complexity a foraging neighbour cannot, and a blight (Part 22) or a soil collapse (Part 16) that breaks its agriculture can starve a civilization back down. Whether and how a people develops farming, which crops and which methods, emerges from its environment, its plants, and its intrinsic disposition toward nature (Part 28), so even this hinge is selected within a grounded space rather than handed over as a mandatory first unlock.

---
## Part 49: Writing and External Memory

Writing is a lever directly on the belief and knowledge core, and it is a clean, high-payoff addition. As an invented technology it makes information persistent and high-fidelity, sharply cutting the decay and distortion oral transmission suffers (Part 9), so that what is written down resists the per-hop mutation that turns spoken history into folklore. It lets technique (Part 23), law (Part 36), and scripture (Part 10) accumulate across generations instead of eroding, and it lets a civilization keep its own chronicles.

```rust
pub struct WrittenRecord {
    pub medium: RecordMedium, // tablet, scroll, codex, inscription, monument
    pub language: LangId,     // in its own tongue, glossed to English (Part 33)
    pub content: RecordContent, // technique, law, scripture, chronicle, tally, letter
    pub fidelity: Fixed,      // high and slow-decaying, unlike oral belief (Part 9)
    pub at: Location,         // a library, an archive, a wall, a grave
}
```

The thematic fit is strong: a written chronicle is a culture's own account of its past, and it is still biased and selective, the victors' record, sitting alongside the engine's ground-truth event log (Part 7), so the world can hold what truly happened, what people believe happened (Part 9), and what a civilization wrote down that it happened, three versions that need not agree. Writing couples to literacy, which fraction of a people can read, to libraries and archives as objects that concentrate stored knowledge, and to the printing press as a later multiplier that drops the cost of copying and floods a culture with text. It also creates a sharp new failure mode: when a library burns, in war (Part 24) or disaster (Part 26), a mass of accumulated technique and history is lost at once (Part 23), a dark age starting from a single fire. Whether a people develops writing at all, and what it chooses to record, emerges from its needs and its culture rather than arriving on schedule.

---

## Part 50: Exploration and Cartography

The observer sees the whole world; the civilizations in it do not. Cartography is the spatial twin of the belief system: each civilization holds its own geographic knowledge, incomplete and distorted, that must be filled by exploration, and the unknown is populated with rumour and myth (Part 9) until someone goes and looks.

```rust
pub struct WorldKnowledge {
    pub owner: GroupId,
    pub known: HashMap<RegionId, RegionBelief>, // explored, remembered, and distorted
    pub terra_incognita: Vec<RegionId>,         // unknown, filled with rumour and myth (Part 9)
}

pub struct RegionBelief {
    pub geography: GeoSketch, // a belief about the land, which can be wrong
    pub source: SourceKind,   // first-hand, hearsay, a copied map, a traveller's tale
    pub accuracy: Fixed,
}
```

A culture's map is built from where its people have been and what travelers and traders report, so it is accurate where they have gone, vague or wrong at the edges, and blank or mythologized beyond, with the same distortion-by-transmission that history suffers (Part 9) applied to places: a coastline known only from a sailor's tale is drawn wrong, a rich land beyond the mountains becomes a legend before it becomes a destination, and a false map can send a colony or an army somewhere that does not match it. Exploration, by foot, caravan, or above all ship (Part 47), converts terra incognita into known, often distorted, geography, and drives the discovery of new lands, resources (Part 19), and peoples (the first-contact trajectory of Part 21). This lumps cleanly into information propagation and the written word: maps are written records (Part 49) that are copied, drift, and are traded, so geographic knowledge spreads, distorts, and is lost exactly as other knowledge does, and a civilization's picture of the world can be as wrong as its memory of its own past.

---

## Part 51: Material Culture, Art, and Style

Cultures develop distinctive aesthetic styles, the tangible counterpart to the cultural systems (Part 10), and those styles mark their made things and buildings, drift over time, signal identity, and feed value and provenance. This is what makes a culture's objects and architecture recognizable as theirs.

```rust
pub struct Style {
    pub culture: CultureId,
    pub motifs: Vec<Motif>,           // recurring forms, patterns, and symbols
    pub forms: FormPreferences,       // proportions and shapes the culture favours
    pub drifts_from: Option<StyleId>, // styles evolve and split like languages (Part 33)
}
```

A style is a set of motifs and form preferences a culture applies to its buildings (Part 43), tools, clothing, textiles, adornment, and art, and it behaves like the other cultural systems: it drifts over generations, splits when a people splits, and is influenced by contact and conquest, the visual analog of linguistic drift (Part 33). It feeds two systems already present. It feeds value (Part 19), because craftsmanship and a prized style raise an object's worth, so a masterwork in a culture's high style is treasured. And it feeds provenance and legend (Part 7), because a made thing carries its style, so an artifact's origin can be read from its workmanship, and a sword in the eastern style found in a western tomb is a story. Clothing, textiles, and personal adornment fall under the same system, marking status, role, and identity. What styles a culture develops emerges from its history and values rather than being assigned, so a people's aesthetic is as much a product of who they are as their language or their laws.

---

## Part 52: Medicine

Medicine is the developed practice of treating the bodies the combat and anatomy system wounds (Part 35) and the diseases the epidemiology system spreads (Part 22). Like any technique it is invented, improved, transmitted, and lost (Part 23), and it changes mortality, war survival, plague outcomes, and lifespan.

```rust
pub struct MedicalPractice {
    pub techniques: HashSet<TechniqueId>,         // surgery, setting bones, herbalism, midwifery...
    pub materia_medica: Vec<HerbId>,              // remedies drawn from flora (Part 16)
    pub efficacy: HashMap<AfflictionKind, Fixed>, // how well it treats each kind of harm
}
```

Medical practice spans surgery and the setting of wounds (Part 35), herbalism drawn from the flora (Part 16), the treatment of disease (Part 22), prosthetics for lost limbs, and midwifery, and each is a technique a culture may discover under pressure, a plague driving the search for treatment (Part 23), and may lose when its practitioners are lost. Better medicine raises survival from wounds and plague and extends lifespan, which feeds back into demographics, war (Part 24), and the value a culture places on its healers. It competes and combines with magical healing where magic exists (Part 34): a culture may lean on herbs and surgery, on divine or arcane healing, or on a blend, and which it develops emerges from its environment, its techniques, and its beliefs rather than being granted. Medicine can also be wrong, since a practice believed effective may not be, so a culture's medicine is another of its beliefs about the world (Part 9), held with confidence and sometimes mistaken.

---

## Part 53: Households, Kinship, and Inheritance

Between the individual and the institution sits a unit the social systems need: the household, the economic and reproductive atom of a society, organized by culturally variable kinship and marriage and bound by rules of inheritance. It ties together genetics (Part 25), institutions (Part 36), and the economy (Part 19).

```rust
pub struct Household {
    pub members: SmallVec<[StableId; 8]>,
    pub kinship: KinshipStructure, // culturally variable: who counts as family, how lines are reckoned
    pub holdings: Vec<ItemId>,     // property the household owns (Parts 42, 44)
    pub head: Option<StableId>,
}

pub struct InheritanceRule {
    pub property: PropertyDescent, // how holdings pass: to whom, divided or whole
    pub status: StatusDescent,     // how rank and office pass (Part 36)
    pub craft: CraftDescent,       // whether trade and technique pass down a line (Part 23)
}
```

The household is where reproduction (Part 25), property, and craft are organized, and its kinship and marriage structures are a cultural variable rather than a fixed scheme, so different peoples reckon family differently, permit different unions, and form different domestic units, all emerging from their values (Parts 10, 28). Inheritance is the rule by which property, status, and craft pass between generations, and it has large downstream effects: how property descends shapes the distribution of wealth and the economy (Part 36), how status and office descend shapes dynasties and the stability of governance (Part 36), and whether craft and technique pass down a family line shapes how skills (Part 35) and trades persist or scatter. A people that divides holdings among all heirs fragments wealth differently from one that passes everything to a single heir, and these rules, emergent from culture, quietly shape the long-run form of a society without being imposed on it.

---

## Part 54: Tier Consistency and Observer Independence

A contradiction sat unresolved in the early principles, and this part resolves it. Principle 3 says the same seed yields the same world on any machine regardless of how it is watched. The earlier wording of Principle 6 said that zooming the camera into a region is what promotes that region to detailed simulation. These collide, because a region simulated as individuals does not produce the same outcomes as the same region advanced as population statistics, so if the camera drove promotion, then where a person looked would change the canonical timeline and the world would no longer be reproducible from its seed. The engine needs a foundational ruling on how the tiers relate and whether watching can change history, and it needs it before the systems that assume one answer are built, since the answer cannot be retrofitted onto both tiers after they exist.

Four axes are tangled together in the phrase "level of detail," and separating them is half the solution. Identity is whether an entity is an individual with a name, history, and belief store, or an anonymous member of an aggregate pool. Processing fidelity is how deeply an entity is updated, from full cognition down to a cheap behavioural approximation. Temporal fidelity is how often it is updated, from every base tick to one batched statistical step across a long span. The driver is whether fidelity is decided by the observer or by the world itself, its in-world significance and the seed. The observer-dependence problem is specifically the case where the driver is the camera and the effect lands on canonical outcomes, so the ruling turns on the driver.

There are four ways to relate the tiers, with real tradeoffs.

Option A, canonical truth is the aggregate tier and the detailed view is a non-authoritative elaboration. The statistical simulation is the source of truth and runs identically regardless of watching; zooming in instantiates a detailed elaboration consistent with the aggregate state, but nothing the detailed view does writes back. Reproducibility is perfect and the detailed view can be arbitrarily rich. The cost is that the people you watch cannot canonically cause anything: a duel you observe was already decided by the aggregate, and the detailed motion must conform to a result it did not produce, which risks the hollow feeling of detail without consequence and weakens the promise that anything you inspect becomes real.

Option B, canonical truth is the detailed tier and the aggregate is only a summary. Everything is really simulated in detail, everywhere, always, and statistics are a compression for display. Detail is maximal and always consequential, and there is no observer dependence because the detail is always present. The cost is that it does not fit consumer hardware at world scale: a whole planet of agents simulated in full every tick over centuries is not affordable, which is the reason level of detail exists at all.

Option C, the two tiers are engineered to be mutually consistent. The detailed simulation of a region is built to yield the same canonical outcomes the aggregate would have produced, so promotion refines and demotion summarizes while canonical invariants hold, and promotion and demotion become free. The cost is that this is the hardest thing on the list to engineer, because a detailed dynamical simulation and a statistical model are different dynamics and can usually be made to agree only on conserved totals and on seeded determinism given a fixed tier sequence, not on identical outcomes; and if they do not produce identical outcomes, then which tier ran, if that is driven by watching, changes history, and the problem returns.

Option D, canonical fidelity is driven by in-world significance and never by the camera. The simulation decides what to run in detail from the world and the seed, a war, a notable figure, a settlement past a threshold, and the observer is a pure viewer that renders whatever fidelity a region is already running and never causes promotion. Reproducibility is perfect and the promotion pattern is itself deterministic. The cost is that you cannot force arbitrary canonical detail by looking: zooming into a quiet backwater the engine is running coarsely shows coarse truth, unless a non-authoritative elaboration is layered on top for viewing.

The recommended resolution combines D and A, and it is the model the amended Principles 1, 6, and 10 now encode. Canonical fidelity is a deterministic function of in-world significance and the seed, so looking never changes canonical truth (Option D, Principle 10). For viewing a quiet region in fine detail, a non-authoritative elaboration is instantiated that is consistent with the region's canonical state and never writes back (Option A), so a person can watch daily life in a backwater without that watching becoming canonical. The instant something canonically significant happens in that region, its significance, not the camera, promotes it to canonical detailed simulation, deterministically. This keeps reproducibility intact while still letting the observer look closely at anything.

The identity axis is settled separately and decisively: sentient beings are always individuals and are never demoted to anonymous pools. Every person who lives in a world holds a stable identity, lineage, beliefs, and history from birth to death, whether or not anyone watches and whether or not they ever do anything of note. This is the answer to whether all sentients can be permanent standouts while promotion and demotion apply only to other creatures, and the answer is yes, with one necessary distinction. Always an individual is a statement about identity, not about processing. Running full cognition and full belief processing on every sentient in a world every tick is not affordable, since a planet of civilizations holds far more sentient minds than even this engine can deliberate over each tick. So identity is universal and permanent, while processing fidelity still scales with in-world significance: the active and the significant run full cognition and full belief dynamics, the quiet majority run a cheap behavioural and belief approximation or are updated lazily, and belief stores are sparse, a quiet farmer carrying few facets. The gain is large and is the reason to adopt it. With sentient identity always present and sentient fates driven by significance rather than the camera, the layer a person most cares about, the named people and their history, is fully canonical, reproducible, and observer-independent by construction, and the only level of detail left on a sentient is how hard it is being thought about, which never changes who it is or what happened to it. Promotion and demotion in the old anonymous-to-individual sense then apply only to non-sentient creatures and plants (Part 11), where it is acceptable that the ten-thousandth deer has no name until it matters.

The observer's experience of time rides on this without breaking it. There is a base tick, the finest canonical timestep, short enough in world-time that people and animals move smoothly rather than jittering. Canonical temporal fidelity is significance-driven like the rest: active regions advance at or near the base tick, quiet regions advance in coarse batched steps over many base ticks (the temporal level of detail of Part 32). The observer's timescale control is a playback speed over that canonical timeline rather than a change to it. At normal speed a person watches an active region tick by tick and sees smooth movement; speeding up advances the rendering faster, and because quiet regions are already coarse, the simulation keeps pace through quiet spans cheaply, which is exactly the case a person wants to fast-forward. Two honest limits follow. Watching a quiet region at normal speed shows smooth movement only through the non-authoritative elaboration above, since the region's canonical motion is coarse. And fast-forwarding through a span that is canonically active is bounded by the cost of computing that activity at its canonical fidelity, because the active steps cannot be skipped or coarsened without changing their outcomes, which is the hard determinism problem that keeps temporal level of detail (Part 32) in the research tier: cheap fast-forward through quiet time is solved by significance-driven coarsening, but cheap fast-forward through busy time would require coarse stepping that reproduces fine stepping exactly, and that remains unsolved.

The keystone the ruling names, that instantiating a region from its summary and later re-summarizing it must agree, is resolved here, and the agreement it guarantees is precise rather than total. The two operations that cross the tier boundary are a lifting, instantiate-from-summary, which turns a coarse pool plus the canonically significant event that promoted it into detailed individuals and state, and a restriction, re-summarize, which folds detailed state back into the canonical pool. These are exactly the lifting and restriction operators of multiscale computation, where a fine model and a coarse model are coupled by an operator each way, and the consistency the engine needs is a relation between the two operators rather than a hope that two different dynamics will coincide.

The guarantee is conservation plus a seeded, significance-driven schedule, and it is the strongest guarantee that can exist for dynamics like these. Identical outcomes between the detailed and aggregate tiers, the tempting third option, is not merely expensive but mathematically unattainable: exact consistent aggregation of a dynamical system requires linearity (the classical aggregation results of Leontief and Theil), and an emergent civilization is radically nonlinear, so no amount of engineering makes a statistical model and a per-agent model produce the same micro-outcomes. What the engine guarantees instead is four properties. Re-summarizing a fresh instantiation changes no conserved canonical quantity, the restriction-after-lifting identity, audited on every transition rather than assumed. Every lifting and every restriction conserves, exactly and in integer and fixed-point, the quantities that a registry of conserved projections declares: each two-tier subsystem registers what it conserves, and the audit enforces all of them, so population, resource stocks, wealth, the institution feature vector (Part 36), and aggregate belief mass (Part 9) are the present entries and a future two-tier system, a magic-field network or a disease model or a trade-route graph, is covered the moment it declares its own conserved projection, with nothing in the mechanism special-cased to the entries that exist today. Given the schedule, which is a deterministic function of in-world significance and the seed and never of the camera (the Part 54 ruling), and given the master seed, the detailed run and the aggregate run are each bit-identical on every replay, on any machine and at any thread count, regardless of who is watching, because instantiation seeds the same individuals from the same counter-based key and so produces the same subsequent canonical facts. The canonical timeline is therefore a deterministic function of the seed and the world alone, so looking changes nothing canonical, which is Principle 10.

What is deliberately not guaranteed, and must not be, is the other direction: lifting after restriction is not the identity. The micro-detail discarded on demotion, the exact task ordering of creatures that become anonymous again, their precise per-tick positions, is not recovered on a later promotion, and that is correct because it was never canonical at the aggregate tier in the first place. This asymmetry, that restriction-after-lifting is exact on conserved state while lifting-after-restriction is lossy, is the honest statement of what the engine promises. A person might wish for more, that a city which grew while unwatched be identical building-for-building to what the aggregate model would have produced, but that wish conflates two different dynamics and describes something that cannot exist: the aggregate model never produced buildings by name, only conserved totals and event summaries. The invariant keeps the tiers agreeing exactly where agreement is meaningful, the conserved totals, and reproducible everywhere a fact is canonical, while letting the non-conserved micro-detail differ because it carries no canonical weight. This satisfies determinism (Principle 3), which asks that the same seed yield the same world and is met by the reproducibility of the canonical sequence, and observer independence (Principle 10), which asks that watching not change canon and is met by the camera-free schedule and the non-authoritative elaboration, without demanding the impossible identical-outcomes agreement.

Instantiate-from-summary materializes the pool's load-bearing slots in canonical id order, seeding each individual with a counter-based key that is a pure function of the master seed, the region, the canonical tick, the triggering event, and the slot, so the same promotion yields the same individuals every time and on every machine. The sampled distributions, age, skill, personality, and the prevailing beliefs, are conditioned to reproduce the pool's own moments to fixed-point tolerance, so the materialized detail is consistent with the statistics it came from rather than a fresh invention, which is the multiscale principle that a lifting constructs fine states conditioned on the coarse moments. Conservation holds by construction: the instantiated count plus the count left in the pool equals the original, resources and wealth are split by an exact integer partition with the remainder assigned to the lowest canonical id, and the institution materializes so that its feature signature reproduces the pool's (Part 36). This reuses the promotion machinery and its seeding context already specified (Part 11).

Re-summarize folds detailed state back into the new canonical pool by exact id-ordered fixed-point sums of the conserved quantities, re-derives the moment distributions from the individuals, folds the prevailing beliefs back as the id-ordered mean of the corresponding facet strengths, and compacts the run of fine events into a summary record carrying a provenance hash over the compacted span and the demotion event, so the event log stays append-only ground truth and auditable (the boundary with the event-schema item R-EVENT). The pool seed is derived deterministically, and the resulting pool is the new canonical aggregate state going forward, itself reproducible from the seed and the significance schedule.

A collective undertaking, a public work, a war, a law, is emitted at the aggregate tier by the aggregate representation of the settlement's emergent governing institution (Part 36), not by a draw over pool demographics. The aggregate institution carries its legitimacy, its coordinated-function profile, its roles, and its norms in the same ADICO grammar the detailed tier uses, and it emits an undertaking when those norms fire on the pool's current conditions and an accumulated decision propensity crosses a reserved threshold, with the single remaining tie-break keyed on counter-based RNG. This is a coarse projection of the very deliberation the detailed tier would run agent by agent through the same institution's roles and norms (Part 8), so the two tiers share one authority and agree on which authority decides and under which norms, which is the canonical level of the decision, while the tactical execution is detailed-tier micro-detail that carries no canonical weight when aggregated. Demographic and social events, births, deaths, feuds, migrations, remain draws over pool composition because they are not acts of governance. Because institutions are now fully emergent and differ per race (R-INST), this decision mechanism inherits that generality with no further work: whatever authority a people evolved, with whatever roles and norms, is what governs its aggregate tier, so an exotic governance structure drives aggregate decisions exactly as a familiar one does, and nothing here assumes a council or a monarch or any particular shape of rule.

Promotion and demotion in the identity sense, anonymous to individual and back, apply only to entities without permanent identity. An entity that holds a permanent identity is never demoted to a pool and scales only in how hard it is being thought about, and which entities those are is itself per-entity and per-race data rather than a fixed notion: for the standard sentient races it is every person, but a race whose unit of identity is not the individual body, a hive whose colony is the person, or any other exotic identity structure a race may carry, is handled by the same rule keyed on the identity-permanence property rather than on a hardcoded idea of who counts as a person. A permanent-identity entity running the cheap behavioural and belief approximation and then re-thought at full cognition stays consistent because the coarse update is the restriction of the full update onto the conserved facets: its identity, lineage, and any canonical outcome that already happened are preserved exactly, and the fine cognition the coarse update skipped was never canonical. The belief lifting and restriction, the mapping deferred from the evidence work (Part 9), is the same pattern made concrete: on promotion a prevailing belief at the pool's knowledge level maps to a facet strength by a reserved monotone fixed-point curve with a small counter-seeded per-mind dispersion conditioned so that the population mean strength reconstructs the pool knowledge level, and on demotion the id-ordered mean of facet strengths folds back, conserving total belief mass.

All of it is integer and order-independent. The conservation sums, the moment re-derivations, the decision propensity and its scoring, and the belief means are fixed-point and aggregated in canonical id order, so they are independent of evaluation order and thread count; the significance schedule takes no camera or wall-clock input, which is the structural reason watching cannot move the timeline; every stochastic step, a promotion slot, a belief dispersion, a decision tie-break, a demographic draw, is keyed on a hash of the master seed, the entity or region, a phase, and an ordinal, so the same draw is produced regardless of when or whether a region was promoted; and the canonical ids of instantiated entities are derived deterministically so that promotion and demotion preserve ordering across the boundary and the event log stays coherent.

> Decided and reserved. The mechanism is settled and signed off, and it is general across every two-tier system rather than fitted to one. The keystone is resolved as a lifting operator (instantiate-from-summary) and a restriction operator (re-summarize) with a conservation-plus-seeded-schedule guarantee: restriction-after-lifting is exact on conserved state and audited every transition; every transition conserves, in integer and fixed-point, a registry of conserved projections that each two-tier subsystem declares for itself, so present and future two-tier systems are covered without special-casing; each tier is bit-reproducible given the significance-and-seed schedule, which excludes the camera; and the canonical timeline is therefore a function of seed and world alone, satisfying Principles 3 and 10. Lifting-after-restriction is deliberately lossy, because the discarded micro-detail was never canonical, and identical-outcomes agreement is rejected as mathematically unattainable for nonlinear dynamics (Leontief, Theil) rather than merely costly. Collective decisions are emitted at the aggregate tier through the emergent governing institution (Part 36) and its ADICO norms, legitimacy, and coordinated function, a coarse projection of the detailed tier's deliberation through the same authority, so the mechanism inherits the institution model's full per-race generality and assumes no particular shape of rule; demographic and social events stay pool-composition draws. Identity promotion and demotion apply only to entities without permanent identity, with the identity-permanence property per-entity and per-race data so exotic identity structures are handled by the same rule; permanent-identity entities scale only in processing fidelity, the coarse update being the restriction of the full update on conserved facets. The belief level-to-strength lifting and the id-ordered restriction conserve belief mass and resolve the mapping deferred from the evidence work. The mechanism is fixed Rust; the conserved-projection registry is a system declaration, and the thresholds and rates are reserved. What is reserved for your calibration, surfaced rather than fabricated, with its basis given: the significance thresholds that drive promotion and demotion (basis: that canonically consequential events always promote while quiet drift never does, calibrated against whether anything that mattered ran at the wrong fidelity); the decision-propensity threshold and accumulation rate for aggregate collective undertakings (basis: matching the aggregate undertaking rate to the detailed tier's for comparable conditions, per settlement scale); the belief level-to-strength curve and the per-mind dispersion magnitude (basis: that the population mean reconstructs the pool knowledge level within fixed-point tolerance and individual variation reads as plausible); the resource and wealth partition remainder rule (settled as lowest-id, a one-line data choice if another fairness policy is wanted); an optional distributional-agreement tolerance only if a statistical-replay feature is ever added (basis: the fixed-point distance already used elsewhere, default unused); and the base-tick duration insofar as it bounds how fast significance can promote a region (basis: the engine-wide tick already chosen). The reserved list is in the audit log. The honest limits stand: identical-outcomes agreement is not provided and cannot be, so a person who toggles fidelity and scrutinizes non-conserved micro-detail can see the detailed and aggregate dynamics differ in canonically irrelevant ways; the cheap fast-forward through a busy, significant span of time remains the separate open problem of temporal level of detail (Part 32), distinct from this spatial-and-processing tier work, since re-summarization here is for quiet spans rather than busy ones; this couples to the non-authoritative view elaboration (R-VIEW-ELAB), which must stay consistent with the canonical tiers but is a separate mechanism that cannot write canon, and to the event-schema item (R-EVENT), which the demotion compaction touches; and a poorly calibrated aggregate decision proxy could emit undertakings the detailed tier would not, which is a calibration target to cross-validate rather than a flaw in the invariant.

> Needs research, item R-VIEW-ELAB in the research backlog. The ruling above settles that a quiet or aggregate region is shown in detail through a non-authoritative elaboration that cannot write canon, but not how that elaboration is built. A session must specify how an aggregate pool, or a sentient run at coarse or lazy processing, is elaborated into ephemeral per-tick individuals for viewing at the base tick, near a one-second resolution if feasible: seeded by region, canonical time, and pool seed so the same zoom shows the same people doing the same things on every re-look and stays consistent with the pool's population, demographics, prevailing beliefs, and recorded events; with smooth generated motion between coarse canonical updates; differing for the two identity cases, an always-identified sentient elaborated from its existing coarse state versus an invented ephemeral individual for a non-sentient pool; and keeping the one-way boundary so a watched backwater duel never becomes canonical, with significance and not the camera performing any real promotion. It rides on temporal level of detail (Part 32) for quiet spans but is its own mechanism, and the base-tick duration that sets the one-second target is itself a reserved value. Flagged, not changed.

The whole ruling rests on one enforced boundary, the same kind the determinism and steering audits rest on: the rendering layer and the non-authoritative elaboration layer are structurally unable to write canonical state, so observation and viewing-time elaboration cannot leak into the timeline (Principle 10, and the typed canonical-state boundary of Part 58). With that boundary in place, the detailed, zoomable, real-time view of Principle 6 and the seed-reproducibility of Principle 3 hold at the same time.

---

## Part 55: Units, Dimensions, and the Measurement of Physics

Part 41 made physics the authored core and the one bounded place our bias is allowed to enter, which makes the unit system foundational. If mass, length, force, energy, and temperature were each invented per system, the physics could not compose, and combat, structural integrity, mechanical power, and thermodynamics would not share a number line. The unit system also fixes the fixed-point discipline, because a fixed-point value has a finite range and precision and a careless choice overflows or loses resolution over deep time. This part settles both, and it settles the half you asked about: how the cultures come to measure and understand physics as their own, without that understanding being handed to them.

There are two layers, and keeping them apart is the whole design.

The first is the absolute canonical unit system, the engine's ground truth. Physics is authored and computed in one fixed set of base units, an internal metric system of a base length, mass, time, and temperature and the derived units of force, energy, and pressure, each represented in fixed-point at a chosen scale (Part 3). This layer is absolute and is never culture-relative; it is the substrate every physical system shares so they compose. It is also, by Principle 9, exactly and only where humanity's understanding of physics enters: the base units and the constants and laws expressed in them are the acknowledged, bounded bias, and nothing else about technology or measurement is authored.

```rust
pub struct AbsoluteQuantity {
    pub dimension: Dimension,   // Length | Mass | Time | Temperature | Force | Energy | ...
    pub magnitude: Fixed,       // in base units, at this dimension's defined fixed-point scale
}
```

The second is the emic layer, the measurement systems and the understanding of physics that the cultures invent for themselves. A people does not know the absolute units and never sees them. It develops its own units from what is at hand, a length from a forearm or a stride, a weight from a particular stone or seed, a span of time from a celestial cycle, and gives them its own names, and those units are, to the engine, conversion factors and provenance relative to the absolute system that the culture itself does not possess. Just as importantly, a people develops its own understanding of how physics works, and that understanding is a body of belief (Part 9) that can be incomplete or wrong and that evolves. A culture may hold that heavier bodies fall faster, or that heat is a fluid, and may carry that belief for centuries until observation, a craftsman's experience, or a teacher revises it, the same conception-and-transmission machinery that drives technology (Parts 23, 41). Their measurements and their theories improve, drift, spread between peoples, and can be lost, as techniques and rumours do.

```rust
pub struct CultureMeasurementSystem {
    pub culture: CultureId,
    pub units: HashMap<UnitName, EmicUnit>, // the culture's own named units
}
pub struct EmicUnit {
    pub dimension: Dimension,
    pub to_absolute: Fixed,   // conversion the engine knows; the culture does not
    pub origin: UnitOrigin,   // a body part, a seed, a vessel, a celestial cycle
}
pub struct PhysicalUnderstanding {
    pub culture: CultureId,
    pub laws: Vec<BelievedLaw>, // the culture's theories of physics, possibly wrong (Part 9)
    pub accuracy: Fixed,        // how well its understanding matches the absolute laws
}
```

The engine evaluates designs and events in absolute units, because physics is the teacher and physics is absolute; the cultures act on their own measurements and beliefs, which may serve them well or mislead them, and the gap between the two is a source of both error and discovery. A people whose understanding of leverage is good builds better machines (Part 45); a people whose grasp of structural load is wrong builds things that fall (Part 43); a people that has not yet realized two of its units describe the same quantity measures clumsily until someone makes the connection.

The readability layer surfaces quantities in either form. For the observer's own understanding a quantity can be shown in absolute or familiar terms; for flavour and legend it is shown in a culture's own units and a culture's own conception, the etic and emic split that Parts 41 and 39 already use, so an inscription gives a distance in the people's own measure and the observer still gets the gist.

The numeric backbone deserves stating plainly, because it is foundational and easy to get wrong. Each physical quantity is assigned a fixed-point scale chosen for its real range, so that the smallest meaningful difference is representable and the largest plausible value leaves headroom; quantities that accumulate over time, wealth, wear, deposited sediment, are accumulated with an explicit overflow and rescaling discipline rather than summed blindly into one fixed value across centuries, since a Q32.32 value in a 64-bit integer has a bounded range that long accumulation will exceed. The rule is that an absolute quantity carries its dimension and its scale, conversions between scales are explicit, and saturation versus wrap on overflow is a defined per-quantity choice rather than an accident. This is the same discipline that protects determinism elsewhere, applied to the numbers physics is computed in.

---

## Part 56: Coordinate Dimensionality and World Topology

The shape of space is foundational because pathfinding, spatial indexing, rendering, structural physics, and fluid flow all bake in the decision, and adding verticality or changing the macro topology after the spatial stack exists is among the most expensive retrofits a world engine can face. The decisions are settled here so the spatial layer (Part 6) can be built against them.

The world is two-and-a-half dimensional, a stack of two-dimensional tile layers in the Dwarf Fortress model. Every tile has a position and a z-level, every entity has an x, a y, and a z, and the systems that need verticality operate in the stack: mining and caverns descend through z-levels (Part 26), fluids fall and pool and pressurize between them (Part 43), structures rise through them with the load of each level bearing on the one below (Part 43), and burrowing and winged locomotion (Part 20) move through the vertical the way walking moves through the horizontal. This is richer than a flat plane and far cheaper than true volumetric three dimensions, and it matches the fortress-and-underground character the world wants.

Rendering is separate from this and does not touch canonical state. The primary view is the top-down glyph grid per z-level, the Dwarf Fortress presentation, which the multi-scale zoom (Part 14) already assumes. An isometric viewer is noted as an optional addition if it can be built cheaply, and because it is a rendering concern rather than a simulation one, it carries no risk to canonical state and can be added or dropped at any time without touching the world model, so it is left as a stretch rather than a commitment.

The macro topology of the map, its overall shape, is a seeded starting variable (Part 40) rather than a fixed choice. A world may be a bounded flat plane, a plane that wraps in one direction like a cylinder, or a true sphere. A sphere is appealing and would make circumnavigation and the geometry of climate and exploration real, but it is the most expensive option, since tiling a sphere evenly is awkward, the poles are coordinate singularities, and the flat tile view needs a projection. Because the choice is a start variable, the foundational requirement is not which topology ships first but that the spatial layer is built against a topology abstraction: adjacency, distance, and coordinate wrapping go through an interface that a flat, cylindrical, or spherical implementation can satisfy, so that pathfinding (Part 13), spatial hashing, and chunking (Part 6) do not hardcode the assumptions of a flat plane. Build that interface and any topology is selectable; omit it and the world is locked to whichever was assumed, the retrofit to avoid.

```rust
pub struct Coord3 { pub x: i32, pub y: i32, pub z: i16 } // 2.5D stacked tile layers

pub enum Topology { FlatBounded, FlatWrapped, Cylindrical, Spherical } // a seeded start variable

pub trait TopologySpace {           // all spatial code goes through this, never raw flat-plane math
    fn neighbours(&self, c: Coord3) -> SmallVec<[Coord3; 8]>;
    fn distance(&self, a: Coord3, b: Coord3) -> Fixed;
    fn wrap(&self, c: Coord3) -> Coord3;
}
```

---
## Part 57: Deterministic Scheduling and Agent Execution

Two foundations are flagged here for dedicated design passes of their own rather than settled now, because both are the kind of invisible substrate that compounds with every system added, and both deserve a full brainstorm before they are locked. They are recorded with their requirements so the rest of the engine is built to permit the answers.

The deterministic scheduler. Choosing hecs (Part 4) meant choosing to own the loop, which means hecs provides no automatic system scheduler, and the design now has on the order of sixty systems. A framework like Bevy would infer from each system's borrow signature, which data it reads and which it writes, what may run in parallel and in what order; with hecs that pass must be built. Without a principled declaration of what each system reads and writes, plus a deterministic ordering and a parallelization strategy derived from it, system order becomes implicit, and implicit order is a classic source of nondeterminism, a stray order-dependent reduction or a hash-map iteration in a canonical path. The requirements the eventual design must meet are that ordering and parallel grouping are derived from declared data access rather than hand-tuned by guesswork, that the result is deterministic and observer-independent (Principles 3, 10), and, the goal you named, that the scheduler is cheap to tune, so changing how systems are grouped and ordered is a low-cost adjustment rather than a rewrite. This is the single foundation most worth locking early, because it touches every system at once, and it is given its own pass for that reason, not because it is optional.

The agent execution model. How agents are stepped is foundational to both performance and determinism, and it is where a Dwarf-Fortress-class simulation spends most of its time, the per-unit turn processing that dominates the frame far more than pathfinding does (Part 13). The question is whether agents are polled every tick or sleep until an event relevant to them wakes them, and the answer interacts with everything: with the all-sentients-individual decision (Part 54), since a world of permanent sentient individuals cannot poll every mind every tick and must lean on significance-driven processing and event-driven wakeup; with temporal level of detail (Part 32), since a dormant region's agents should not be stepped at all; and with determinism, since an event-driven wake order must be reproducible. The requirements are bounded cost at world scale, reproducible wake ordering, and compatibility with significance-driven processing fidelity, and the design is given its own brainstorm because event-driven agent execution is hard to bolt onto a poll-everything loop after the fact and is worth getting right from the start.

---

## Part 58: Foundational Substrates and Invariants

A set of cross-cutting substrates and invariants belong in the foundation because later systems silently assume them, and each is cheap to design now and painful to retrofit. They are documented here together.

Conservation and referential integrity, as first-class audited invariants. Promotion and demotion (Parts 11, 54) must conserve totals: the count of individuals plus the counts in the aggregate pools must always equal the whole, with no entity created or lost in the transition, or a world develops silent population and resource leaks that read as nondeterminism. What must be conserved is not a fixed list but a registry of conserved projections that each two-tier subsystem declares for itself, with population, resource stocks, wealth, the institution feature vector (Part 36), and aggregate belief mass (Part 9) among the present entries, so the harness enforces every declared quantity and a future two-tier system, a magic-field network or a disease model or a trade-route graph, is covered the moment it registers its own projection, with nothing special-cased to the entries that exist today (this is the conservation half of the tier-consistency guarantee, R-TIER-CONSIST, Part 54). Cross-store references must survive structural change: a belief, a history entry, an institution, a household, or an artifact that points at an entity must remain valid when entities merge or split, as when two cultures fuse into one or a household dissolves and its members and holdings disperse. These are invariants of the same kind as determinism and the steering audit, and they are enforced the same way, with a standing harness that checks conservation and reference validity across promotion, demotion, merges, and splits, run in continuous integration, so a leak or a dangling reference is caught the moment it is introduced rather than discovered as drift centuries into a world.

The typed canonical-state boundary. Determinism should be enforced at compile time rather than asked of the engine's authors to remember. A fixed-point newtype for all authoritative numeric state makes a floating-point value in canonical state a compile error rather than a latent nondeterminism bug, and a crisp type-level distinction between authoritative state and the non-authoritative fields, the non-authoritative GPU fields (Part 5), the language-model output (Part 39), and the viewing-time elaboration (Part 54), makes it impossible to let an approximate or unreproducible value cross into the timeline by accident. The boundary extends onto the GPU as a matching rule: a canonical GPU kernel contains no floating point at all, so authoritative field work stays fixed-point integer and bit-identical across machines (Section 5.4). This is the steering-audit move applied to numbers: turn a rule you must remember into one the compiler keeps.

The transient intra-tick coordination channel. Systems must react to one another within a single tick, a death notifying the belief, household, institution, and economy systems at once, and that in-tick signalling is a different thing from the append-only event log that records history (Part 7). Conflating the two, or wiring the signalling ad hoc between systems, produces ordering and determinism bugs. The foundation is a transient, deterministic, intra-tick channel, distinct from the historical log, over which systems publish and consume the events of the current tick in a defined order, so the cascade of consequences from one occurrence is reproducible and the historical log stays a clean record rather than a communication bus.

Save schema versioning and migration. The zero-copy speed of rkyv (the persistence of Parts 10 and 40) is tied to an exact memory layout, which means that without a migration path designed in from the start, any change to a component layout breaks every existing save, and a player's world of centuries becomes unloadable after an update. The foundation is a versioned schema with an explicit migration path, so the format can evolve as the engine grows without abandoning the long-lived worlds that are the entire point of the project. This is trivial to ignore until it is catastrophic, which is why it belongs in the foundation.

The unified dynamic graph substrate. Many systems are graphs over the same entities, the social network, trade routes (Part 19), contact between peoples (Part 21), kinship (Part 53), and the transmission paths of belief and technique (Parts 9, 23, 41), and because hecs has no native relationships, each system would otherwise build and maintain its own adjacency and the graphs could not be queried together. One shared, queryable, dynamic graph layer, where edges carry a type and can be added and removed as the world changes and a query can cross relationship kinds, lets these networks share storage and be reasoned over jointly, a question like which trading partners of this city are also kin of its ruling household answerable in one traversal rather than by joining several bespoke structures.

---

## Part 59: Crate and Module Organization

A Cargo workspace separates concerns so the compute kernels, the deterministic core, and the renderer build and test independently. A feature flag selects the CubeCL CUDA runtime without forcing it on non-NVIDIA builds.

```
civsim/                     (workspace root)
  crates/
    core/         fixed-point math, Rng, StableId, Registry, arenas, slabs, CacheLine
    units/        absolute unit system, per-quantity fixed-point scales, emic measurement and conversions
    graph/        shared dynamic typed-edge graph (social, trade, kinship, transmission)
    ecs/          hecs glue: component catalog, blueprints, spawn/despawn helpers
    spatial/      chunks, regions, quadtree/octree, SpatialHash, z-levels, TopologySpace (flat/cylindrical/spherical)
    compute/      CubeCL #[cube] kernels, FieldBuffers, runtime selection, the seam
                  (features: "cuda" -> cubecl-cuda, default -> cubecl-wgpu)
    sim/          tick scheduler, phases, double buffer, command buffers, Rayon + PinnedPool
    ai/           utility AI, GOAP/HTN, LOD-tiered decision dispatch
    history/      Event, EventLog, indices, persistence (rkyv/bincode), compaction
    knowledge/    BeliefStore, facets, MutationGraph, the four-phase gossip loop
    society/      Culture, Religion, RelationStore, enculturation, emergence,
                  intergroup relations, value distance, emergent conflict
    races/        Race parameter sets, builds, locomotion, imbued traits
    lod/          Pool, promotion/demotion, deterministic member generation
    worldgen/     terrain genesis, erosion/hydrology, ecology and dawn-of-sentience seeding
    ecology/      Stock and Flow, flora, fauna, the food web, trophic dynamics
    climate/      climate baseline, weather fields, the water cycle, extreme events
    economy/      materials, deposits, markets, trade routes, emergent prices
    epidemiology/ pathogens, SIR infection over pools, contagion on the trade network
    tech/         techniques, pressure-driven discovery, transmission and loss
    warfare/      armies, logistics, battle resolution, conquest, aftermath
    genetics/     genomes, inheritance, selection, founder effects, hybridization
    geology/      geo columns, the deep map, ore placement, catastrophes
    language/     phonology, morphology, lexicon, drift, families, intelligibility
    magic/        magical laws, traditions, schools, arcane fields, imbuing
    combat/       bodies, tissues, fluids, wounds, weapon and armour materials
    institutions/ emergent governance, law, guilds, money, credit, classes
    mind/         recursive theory of mind, mental models, deception, intrigue
    divinity/     pantheon, deities, domains, intervention (tunable, optional)
    data/         WorldDefinition, loaders, the moddable starting variables
    settlement/   jobs, tasks, items, workshops, the focus-scale local sim
    build/        buildings, construction, structural integrity, decay
    localphys/    focus-scale fluids, fire and heat, cave-ins, item physics
    production/   recipes, workshops, supply chains, storage and spoilage
    mechanism/    water, wind, and muscle power over a transmission network
    infra/        roads, bridges, canals, aqueducts, walls; edits the map
    transport/    ships and vehicles, capacity, range, the logistics network
    farming/      crop domestication and methods, yield, the food surplus
    writing/      written records, literacy, libraries, the chronicle
    cartography/  per-civilization geographic knowledge and exploration
    style/        aesthetic styles, motifs, material culture, adornment
    medicine/     surgery, herbalism, disease treatment, prosthetics
    household/    kinship, marriage, households, inheritance of property and craft
    design/       conceived intents, physics-grounded morphospace, QD search, the steering audit
    historydb/    the temporal and relational query engine over the event log
    dramaturg/    story shapes, recognizers, drama scoring, the live story queue
    observer/     intelligent camera, lock-on, time-scrub, the legends browser
    textgen/      the quarantined LLM flavour-text renderer            [optional]
    timelod/      adaptive per-region time resolution                  [research]
    render/       GlyphRenderer, glyph atlas, camera, field overlays           [optional]
    app/          binary: selects the runtime, wires run modes, the main loop
  tests/
    determinism/  the same-seed-different-threads harness, run in CI
    steering/     the convergence audit across independent seeds, run in CI
    invariants/   conservation and referential-integrity checks across promote/demote/merge/split, run in CI
```

Key external crates and their role: `hecs` for the archetypal ECS, `rayon` for bulk data-parallel passes, `crossbeam` and `core_affinity` for the pinned partitioned pool, `wide` for portable stable SIMD on the CPU side (AVX on x86, NEON on ARM), `cubecl` for the single-source GPU compute layer (with the `cuda` and `wgpu` runtime features, the latter covering Vulkan, Metal, and DX12), `wgpu` directly for the glyph renderer's device (shared with CubeCL's wgpu runtime where that backend is active), `rkyv` for zero-copy world snapshots, `bincode` or `postcard` for the streaming event log, and `smallvec` for the many short inline vectors in events, facets, and relations. Pin versions of the fast-moving crates, CubeCL above all, and expect to track breaking releases.

---

## Part 60: Implementation Staging

Build in this order. Each stage is independently testable, and the early stages de-risk the parts most likely to be wrong.

**Stage 0: Spike the GPU seam and the GPU determinism question (one to two weeks).** Before committing to the architecture, prove two things about the GPU at once, because together they decide how large a lever it is. The first is the seam: write a water or heat cellular automaton as one `#[cube]` kernel, resident across ticks, with async readback of one slice, run it under the wgpu runtime and, on the NVIDIA box, the CUDA runtime from the same source, confirm the kernel expresses cleanly in the DSL, and measure readback latency against a tick budget. If async readback of the agent-relevant slice costs under roughly ten to fifteen percent of the tick budget, GPU offload is worth it; if synchronous stalls dominate, keep environmental simulation on the CPU with SIMD for now and revisit. The second is whether that kernel can be canonical while preserving seed-sharing, which is settled in principle by the exploration in Part 62 and tested for real here. Write the kernel in fixed-point integer with no floating point, hash the full field after a fixed number of steps, and confirm the hash is bit-identical run-to-run, invariant to workgroup and tile size (so autotune cannot make the result hardware-dependent), and identical across the CUDA, Vulkan, and Metal backends and the CubeCL CPU backend used as a reference oracle. Then add the determinism-risky operations real physics needs, the square root and the exponential, each in fixed-point, and re-run the cross-backend test to find the operation, if any, where bit-identity breaks; that operation is the boundary of the canonical GPU footprint and falls back to the CPU. Measure the throughput of the deterministic integer kernel against a throwaway float kernel, since that number decides whether a given field is worth offloading at all. Acceptance criterion: the canonical GPU footprint is exactly the set of field kernels proven bit-identical across all target backends at acceptable throughput, with everything else on the CPU or kept as a non-authoritative quantized field.

**Stage 1: Deterministic CPU core and the foundational substrates.** Build the `core` crate (fixed-point, Rng, StableId, Registry, arenas) and, alongside it, the foundations that everything later assumes: the absolute unit and dimensional system with its per-quantity fixed-point scales (Part 55), the typed canonical-state boundary that makes a float in canonical state a compile error (Part 58), the base-tick representation of time that the later temporal level of detail will build on (Parts 32, 54), and the 2.5D stacked coordinate model behind a topology abstraction so flat, cylindrical, or spherical worlds stay selectable (Part 56). Stand up hecs with the component catalog, implement double-buffered state and command buffers, and build the event log and rkyv/bincode persistence with versioned schemas and a migration path now (Part 58), because retrofitting event sourcing or save migration later is painful. Bring up the determinism harness and the conservation and referential-integrity harness in CI from the first tick (Part 58). The deterministic scheduler and the agent execution model (Part 57) are designed in their own passes before the system count grows, since both touch every system at once.

**Stage 2: Spatial hierarchy, LOD, and worldgen.** Implement chunks, regions, and the quadtree (which the renderer will later reuse), and the level-of-detail model of Part 54: sentients are always individuals, promotion and demotion of anonymous-to-individual identity apply only to non-sentient creatures and plants, canonical processing fidelity is driven by in-world significance rather than the camera, and any detailed view of a quiet region is a non-authoritative elaboration that never writes back. Add the worldgen pass that builds the natural world, seeds proto-populations at the dawn of sentience (Part 28), and runs the emergent history that records all events. Establish data-driven definitions (Part 40) here, since every later system reads its content and starting variables from the world definition and retrofitting that is painful. Acceptance test: the same seed and definition yield a bit-identical world and history across machines and thread counts, regardless of how it is viewed.

**Stage 3: Agents and the folklore subsystem.** Add utility AI for high-LOD agents, with GOAP or HTN only for the narratively critical few. Then build the knowledge subsystem: belief facets with ground-truth-versus-belief, evidence, strength, and provenance pointers, the four-phase witness, reflection, propagation, decay loop, the authored mutation graph, and salience-gated distortion per hop. Add the culture and religion entities and the emergence feedback loop. Keep full belief processing gated by in-world significance rather than by who is watching, with a cheap approximation and aggregate statistical rumour spread for the quiet majority (Part 54). This is the differentiator; invest here.

**Stage 4: Pathfinding and spatial scaling.** Implement the spatial hash for the proximity and line-of-sight checks that dominate, chunk and region partitioning with NUMA-affined workers on the pinned pool, and HPA* plus flow-field tiles for movement. Profile before optimizing pathfinding; if it is under ten percent of tick time, spend the effort on turn processing and line-of-sight.

**Stage 5: SIMD, NUMA, and the glyph renderer.** Convert the hot SoA loops to `wide` SIMD, targeting AVX-512 width on AMD Zen 4 and 5 and AVX2 on Intel before Ice Lake, chosen at runtime by feature detection. Pin workers to cores and CCDs. Build the instanced glyph renderer over the quadtree, with field overlays read directly from the GPU buffers and the camera wired to drive LOD promotion. Keep headless mode for batch runs.

**Stage 6: The living world.** Build the Stock and Flow abstraction (Part 15), then layer in ecology (flora, fauna, the food web, Parts 16 and 17), climate and weather (Part 18), materials and the economy (Part 19), and geology and the deep map (Part 26). Wire the couplings: grazing draws on vegetation, vegetation feeds the water cycle, harvest draws on resource stocks, depletion cascades through the coupled stocks. This is the world the civilizations inhabit and act upon.

**Stage 7: Peoples, conflict, and deep history.** Bring up the race parameter sets (Part 20), values and emergent conflict (Part 21), disease and epidemics (Part 22), the reversible tech web (Part 23), war as a system (Part 24), and genetics and deep-time evolution (Part 25). Then close the loop with the belief-reality coupling (Part 27), wiring every system into the belief stores and letting beliefs drive the actions that change the world. With this stage in place the world produces its own deep history.

**Stage 8: The observation layer.** Build the temporal history database over the event log (Part 30), then the dramaturg that recognizes and scores story shapes over the live stream (Part 29), then the cinematic observer that the dramaturg drives, with lock-on, time-scrub, and the legends browser (Part 31). This is what makes the history watchable and queryable, and it is the highest-leverage layer for the watching-game, so build it as soon as the history exists to read.

**Stage 9: Emergent society and the deep frameworks.** Bring up the emergent institutions, governance, and economy (Part 36), where order crystallizes from behaviour rather than from templates, then the deeper world frameworks: procedural language and its drift (Part 33), grounded magic (Part 34), detailed combat and anatomy at the promoted scale (Part 35), recursive theory of mind (Part 37), and divinity where a world enables it (Part 38). Each rides the transmission, belief, and event machinery already built.

**Stage 10: Presentation, modding, and adaptive time.** Add the quarantined text renderer for the chronicle and the language surface (Part 39), finish the data-driven definitions into full moddability of the starting variables (Part 40), and, if a single machine cannot hold the full-fidelity world across the spans you want, research and prototype temporal level of detail (Part 32) in isolation against the determinism harness before committing to it. With these the simulation reaches the social, mental, and historical target this document describes.

**Stage 11: The local settlement layer and the made world.** Build the focus-scale local simulation, jobs, tasks, items, and workshops (Part 42), then buildings and construction with structural integrity and the focus-scale local physics (Part 43), and the production, supply-chain, and storage layer (Part 44). This is the heaviest local capability and the keystone the rest of the made world stands on, and by Principle 1 it runs only where the observer is watching while the world off-focus stays aggregate. Keep every part of it emergent: jobs arise from needs, structures from culture, production from demand, none of it from a scripted plan.

**Stage 12: The civilizational physical layer.** Add mechanical power networks (Part 45), infrastructure that edits the spatial, water, and movement layers (Part 46), transportation and vehicles, ships above all (Part 47), and agriculture as a developed, improvable practice (Part 48). These turn settlements into a connected, productive, map-shaping civilization, and each is selected by a culture from grounded physical possibilities rather than granted on a schedule.

**Stage 13: Knowledge, material culture, and the social atom.** Add writing and external memory and the bias of a culture's own chronicle (Part 49), exploration and cartography as the spatial twin of belief (Part 50), material culture and aesthetic style (Part 51), medicine (Part 52), and households, kinship, and inheritance (Part 53). These extend the belief-versus-truth core into knowledge and space and give the made world its cultural texture and its social atom. With this stage in place the simulation is the full target this document describes.

**Stage 14: The emergent technology design space.** With the made world in place (Stages 11 through 13) and a grounded physics substrate to evaluate against, build the design space of Part 41 in the order the research supports. First prove convergence without a target: one conceived intent, a part-based representation, fixed-point physics proxies, MAP-Elites per culture, and two isolated cultures, and confirm they independently populate the same physical attractor from different seeds and materials. The threshold to proceed is that the convergence is explained by physics and not by authored bias, checked by the steering audit; if the cultures do not converge where physics should force it, the physics proxy or the descriptor space is wrong, and evaluation is fixed before anything is added. Then add the transmission layer (drift, diffusion, loss) and confirm lineages diverge, borrow, re-converge, and occasionally lose a technology under a population crash. Then scale the physics substrate and the range of conceivable intents, holding the per-tick budget with surrogate evaluation and tighter LOD as needed. Finally add readability: the etic functional tags and the grammar renderer, with the quarantined language-model flavour layer only if it stays cached and non-authoritative. The recursive composition of validated modules into complex multi-part technologies is now specified (R-DEEPTECH-COMPOSE, Part 41), and the deepest reach beyond it, emergent science, a deepening discoverable physics substrate, and the whole compounding climb over deep time, is the part of this document that remains partly unsolved (Part 61), so it is built incrementally and proven at small scale before it is trusted.

---

## Part 61: Risks and Caveats

The young, fast-moving crates (CubeCL above all, plus wgpu and hecs's surrounding ecosystem) will see breaking changes; pin versions and budget for upgrades. wgpu compute performance varies across versions and backends, with documented swings of around twenty percent between releases and overhead on Metal via translation, so benchmark on each target rather than assuming portability implies uniform speed.

The folklore subsystem's academic precedents are uneven in maturity. Talk of the Town shipped (in Bad News) but was explicitly not computationally efficient, around a minute per knowledge timestep for only a few hundred fully-modelled characters, which is exactly why full belief modelling must be restricted to promoted entities. Gossamer is a prototype whose impressive emergent behaviours are stated as goals rather than demonstrated outcomes, so treat its four-phase model as a sound architecture to build on, not a proven result to copy.

The Dwarf Fortress profiling figures (pathfinding around three to six percent, line-of-sight around twenty, turn processing over sixty) come from developer statements and community reports rather than a formal publication, and they vary by fort, so use them as direction, not gospel, and profile your own build.

Determinism is fragile under parallelism. Any stray parallel floating-point reduction, any hash-map iteration order in a canonical path, or any thread-count-dependent work split can break reproducibility. The harness that runs the same seed at different thread counts and asserts bit-identical state is not optional; it is the early-warning system that keeps the property alive as the engine grows.

A NUMA-aware Rust crate with a specific current version could not be pinned down at survey time; thread pinning via `core_affinity` is available, but validate the topology behaviour empirically on the AMD and Intel hardware you have access to before relying on it.

The made-world layer is the largest body of new work in this document, and its keystone, the focus-scale local settlement, building, and physics simulation (Parts 42, 43), is a major undertaking on the scale of Dwarf Fortress's own core loop. It is bounded hard by the focus under Principle 1, so it never runs world-wide, but it is still the heaviest single capability here, and the parts that depend on it (construction, crafting, mechanisms, logistics) cannot be finished before it exists. Sequence it accordingly, and prove the focus boundary, detailed local state instantiating cleanly from a settlement's aggregate summary and re-summarizing on zoom-out, before building the systems that ride on it. Separately, the emergent technology design space (Part 41) is now specified rather than deferred, but it carries the deepest residual risk in the document, and the risk is stated plainly. Functional convergence on simple artifacts is well-supported by the open-endedness and quality-diversity literature, but the open-ended invention of complex, multi-part technologies hits the complexity ceilings that literature reports and is partly unsolved. The choice of descriptor space for the morphospace search is itself a subtle form of authoring, since the search illuminates only the dimensions it is given, so descriptors must be physical quantities rather than designer aesthetics, and the steering audit must confirm that convergence is forced by physics and not by that choice. No shipped game has demonstrated the emergent invention of new tool categories as opposed to variants within authored categories, so the architecture is a synthesis of proven components rather than a proven whole, and its central claim, convergence without steering, must be tested at small scale (Stage 14) before the full system is trusted. The rest of the made world is built to function with cultures selecting within a grounded space of physical possibilities, so none of it blocks on the deepest generative layer being solved. That deepest generative layer, the open-ended climb from simple artifacts to the modern and far-future tier emerging from the dawn of sentience, is the project's north-star ambition and is documented as the five-question R-DEEPTECH cluster in the audit log, accepted as possibly unreachable, pursued for its own sake and for the paper-grade material a partial result would yield, and to be taken super-deep when reached.

Finally, the GPU strategy. Adopting CubeCL removes what was the largest single maintenance cost in the original plan, the parallel CUDA and WGSL kernel codebases, by making one `#[cube]` source compile to every backend. The cost that remains is CubeCL's own maturity: it is a restricted subset of Rust (functions, generics, structs, partial trait and method support), it is a young project from a small team with rough edges, and its API still moves, so kernels must fit the DSL and versions must be pinned. The Stage 0 spike exists partly to confirm the kernels you need express cleanly before you depend on it. If CubeCL ever proves too limiting for a specific kernel, the fallback is the reverse of the original plan: drop that one kernel to raw wgpu or cudarc behind a thin trait while keeping every other kernel in CubeCL, rather than maintaining two full backends. You retain AMD and Intel test hardware to validate the wgpu runtime path, and the CUDA runtime gives you raw NVIDIA performance on your own machine, both from the single source.

---

## Part 62: Research Record of Explored Design Questions

This part records design questions that have been explored in depth and provisionally settled, so the reasoning and the evidence are kept and the questions are not reopened from scratch later. Each entry states the question, what was found, the decision taken and where it lives in this document, and what still has to be proven before the decision is fully trusted. The record grows as more questions are worked through.

### 62.1 How much of the GPU can be canonical while keeping seed-sharing

The question. The design wants two things that look like they conflict: a short seed that reproduces an entire world bit for bit on any machine (the cross-machine determinism of Part 3, which is what makes seed-sharing work), and the GPU as a working lever for the environmental field physics rather than an accelerator confined to non-authoritative output. Floating-point arithmetic is non-associative and its results vary across drivers and hardware, and a GPU's lanes reduce in hardware-scheduled order, so float on the canonical path breaks cross-machine reproducibility. The question was how much GPU work could be made canonical without giving up seed-sharing.

What was found. The hazard is floating point, not the GPU. Integer arithmetic is associative and exact, so an integer computation is independent of how lanes are scheduled and is bit-identical across vendors and drivers. Two demonstrations were run to check the claim rather than assume it. The first showed a parallel reduction and a long-running diffusion stencil giving different results across summation orders in float, and for the reduction a flatly wrong answer through absorption, while the fixed-point integer version of both was bit-identical across every order. The second showed that the transcendental functions physics needs do not force float back in: an exact integer square root matched the reference over tens of thousands of values, and an exponential by fixed-point series landed within about one part in one hundred thousand of the real function, with every operation an integer add, multiply, shift, or divide and no float unit touched. The precision cost of fixed-point is a small, fixed, reproducible quantization, the same on every machine, which is a different thing from float's order-dependent divergence. The approach also has a real-world existence proof: GPU cryptography and mining produce identical hashes across different vendors' hardware using exactly this discipline, integer-only and no hardware transcendentals.

The decision taken. The GPU stops being non-authoritative by definition and becomes a lever in two modes. It is canonical for the subset of field physics expressible in bounded fixed-point integer (diffusion, the Margolus-block fluids, erosion, heat), which runs authoritatively on the GPU and stays bit-identical across machines, so seed-sharing holds. It is also free, with no determinism constraint, for everything that never touches canon (rendering, overlays, view-time elaboration), because the typed boundary and Principle 10 firewall that work from canonical state. The hard rule is no floating point in any canonical kernel. This is written into the determinism architecture (Section 3.4), the GPU compute part (Section 5.4), and the typed-boundary substrate (Part 58). A consequence worth noting is that a deterministic integer field is strictly stronger than the old quarantine, since there is no variance to contain and the question of whether quantization leaks nondeterminism through agent feedback simply disappears.

What remains to be proven. The mathematics of integer order-independence is settled, but cross-vendor bit-identity of the actual compiled kernels is an empirical property of compilers and drivers and is not taken on faith. The Stage 0 spike (Part 60) is the gate: it confirms each candidate kernel produces identical output across the CUDA, Vulkan, Metal, and CPU backends, finds the first operation where identity breaks if one does, and measures the throughput of the integer kernel against a float baseline to decide whether each field is worth offloading. The residual risks are that some operation's integer codegen differs across a backend, in which case that field falls back to the CPU, and that a fixed-point transcendental is too slow to be worth it, in which case that field stays on the CPU or stays a non-authoritative approximate field. The lever is therefore as wide as the spike proves it to be, with the CPU as the deterministic fallback for everything outside it.

### 62.2 How to represent personality so it is data-driven, changeable, and works for non-human minds

The question. The being model needs a representation for personality grounded in how personality really works and changes, data-driven per race rather than a hardcoded human template, and extending to minds unlike humans, including a tier of intelligent beasts that have personality without full sentience. The placeholder was a single signed integer per trait on a fixed, human-derived axis set, which the audit flagged as both ungrounded and probably wrong (R-BEING-REP).

What was found. Personality science has moved past the fixed vector. A trait is better modelled as a density distribution of momentary states (Fleeson, 2001): a stable central tendency plus a characteristic spread that is itself a stable individual difference, which dissolves the old person-versus-situation problem. Above the dispositional layer sit characteristic adaptations (values, goals, and learned if-then situation-response signatures; Mischel and Shoda, 1995; McAdams and Pals, 2006) and, on top, a malleable narrative identity. Values are a linked but distinct layer with their own circular structure (Schwartz). The five-factor structure is not universal: it failed to replicate among Tsimane forager-horticulturalists, where a "Big Two" emerged instead (Gurven et al., 2013), and animal-personality research finds a smaller recurring temperament palette (boldness, exploration, activity, sociability, aggressiveness) that generalizes across species, so the axes themselves must be data. Personality change is lawful and quantified: rank-order stability rises with age toward a plateau (Roberts and DelVecchio, 2000; Bleidorn et al., 2022), mean levels drift in a maturity direction, roles and major life events move traits, and the effects are reproducible by a simple pull toward moving targets. Heritability is moderate and mostly additive, near one half, with the non-shared environment dominant (Polderman et al., 2015). Game and simulation precedent (Dwarf Fortress facets-plus-values-plus-goals, The Sims numeric axes, Talk of the Town numeric personality over deep time) confirms a layered integer model renders to legible text and scales.

The decision taken. A three-layer model, signed off: dispositional traits as fixed-point density distributions (a setpoint and a reactivity per axis), a characteristic-adaptations layer holding values, goals, and if-then contingencies, and a sparse narrative layer for the significant few. Personality change is a deterministic integer pull of each setpoint toward a blended target (maturity, enculturation, role, corresponsive), scaled by an age-declining plasticity, with life-event impulses and decaying self-change bursts. Inheritance is a heritable blend toward the population mean plus a dominant non-shared term plus mutation. The mechanism is fixed, audited Rust; the axes, lexicons, correlations, rates, and curves are data (Principle 11). Animals and great beasts run the same machinery with fewer axes and fewer layers, gated by the intelligence dial, which gives the intelligent-beast tier the user wanted. This lives in Part 20, with the fauna tiers in Part 17 and the sources in Part 63.

What remains to be proven. The numeric calibrations are review values from human, mostly-WEIRD samples and are reserved for the owner rather than fabricated: the per-axis heritable fraction, the plasticity-by-age curve, the maturity-target directions and magnitudes, the life-event and self-change sizes, and which contested mechanisms (the social-investment role effect above all, where the evidence is mixed) are enabled per race. Two connected representation questions stay open as their own items: the axiomatic-belief stance (R-AXIOM) and the broader gene model the trait-inheritance rule plugs into (R-GENOME); the value-distance metric that this layer's values feed has since been resolved (R-VALUE-METRIC, recorded in Part 62.3). In-engine, it must be shown that the change dynamics reproduce the rising rank-order-stability curve and roughly one standard deviation of lifespan drift, that populations stay diverse over deep time rather than collapsing onto the maturity target, and that the whole update is bit-identical across thread counts and machines.

---

### 62.3 How to measure the distance between two value profiles, within and across races

The question. Conflict, enculturation, and divine favour all ask how far apart two value profiles are, and the placeholder answered with a naive Euclidean distance over shared axes, which treats every value as independent and orthogonal. The audit flagged this (R-VALUE-METRIC): real value systems are structured, some values are near-synonyms and others are opposites, and a single human-derived structure cannot be hardcoded as universal across unlike minds. The metric also has to work with recursive theory of mind (Part 37), where an agent compares believed profiles, often partial, not just ground truth.

What was found. Human values do carry a structure, the Schwartz circular model, where adjacent motivations are compatible and opposite ones conflict, validated across many cultures (Schwartz, 1992, 2012; Schwartz and Bardi, 2001). But the strict circular form is contested even for humans (Perrinjaquet et al., 2007, reject the strict circumplex by structural-equation test), and other moral-psychology frames are not a ring at all: Moral Foundations is a set of near-independent foundations (Haidt), Hofstede and Inglehart-Welzel are low-dimensional spaces, Rokeach is a rank-ordered list. Structure is real but not single, so it must be data, not a constant. The right mathematics for distance over a structured space is a ground metric plus optimal transport: where full distributions exist, tree and graph forms of earth mover's distance have exact closed or near-closed forms (Le et al., NeurIPS 2019; Villani, 2009), and the ground metric itself is all-pairs shortest paths over the structure (Dijkstra, 1959; Floyd, 1962), which over integer edge weights is exact integers, so determinism is free. For the common case of a single profile per agent, a weighted distance over that same ground metric is the cheap primary form, and it reduces to plain Euclidean exactly when the structure is independent. Across races the construct-equivalence problem applies: two races' emic axes are not directly comparable, so comparison must pass through a shared etic substrate with authored projections, a standard cross-cultural-measurement move. Game precedent confirms the layered, data-driven approach: Dwarf Fortress reasons over dozens of value axes, and Stellaris, Crusader Kings, and RimWorld all attach social consequence to value or trait alignment.

The decision taken, signed off conditional on theory-of-mind compatibility, which was checked and holds. Value structure is per-race data, an enum over independent axes, a relationship matrix, or a weighted graph, with a ring or tree recognized for the fast exact forms. It is compiled offline into an exact integer all-pairs ground-metric table, so the runtime is a weighted sum over table lookups. The distance is a pure function of two profiles and the table, which is what makes it composable with recursive theory of mind: it runs identically on a believed profile inside a mental model, at any nesting depth, and partial profiles are handled by summing over the axes known, which is why the weighted form, not normalized transport, is the primary runtime path. Cross-race distance projects both profiles onto a shared etic substrate, adds a baseline incommensurability term, and treats an untranslatable value as a theory-of-mind blind spot. This lives in Part 21, wired into conflict (Part 21), enculturation (Part 10), and divine favour (Part 38), with the registries in Part 40 and sources in Part 63.

What remains to be proven. The calibrations are reserved for the owner, not fabricated, because the research returns them as design choices or as values with no empirical anchor for non-human minds: the default compatibility and opposition weights, the coefficient mapping distance to conflict pressure, the enculturation pull rate and geodesic-versus-straight choice, the deity favour curve and direction-versus-intensity choice, the cross-race incommensurability floor, and which axes populate the etic substrate. In-engine, it must be shown that the compiled table matches the structure's true shortest paths, that the weighted form and the transport form agree where both apply, that the metric stays cheap enough to call per modelled mind per theory-of-mind level, and that the whole computation is bit-identical across thread counts and machines.

---

### 62.4 How to represent and evolve axiomatic beliefs, for races and not only cultures

The question. A race begins with intrinsic beliefs: a value profile and a small set of axioms about the world, the self, others, and the sacred. The placeholder represented each axiom as a single signed scalar with a strength, which the audit flagged (R-AXIOM): a scalar conflates where a stance sits with how hard it is to move, says nothing about how a mind decides what is true, and cannot be unique across unlike minds. The owner asked that the answer cover both worldview stance and epistemic stance, settle representation first and dynamics second, and run per race with per-race coefficients, not only per culture.

What was found. Belief science separates position from hold at every level: Rokeach's central-to-peripheral architecture, Gardenfors and Makinson's AGM epistemic entrenchment (an ordering that decides which belief yields when two conflict), and the Primal World Beliefs program (Clifton et al., a five-year effort measuring stable primals such as the world being safe, enticing, and alive) all distinguish what is believed from how firmly. There is a direct cross-cultural model of general beliefs about the world, the Social Axioms Survey (Leung and Bond), whose factors recur across dozens of cultures and which is the closest published analog to the engine's axioms. Epistemic stance, how a mind decides what is true, is a distinct and well-mapped construct (personal epistemology: Perry; Hofer and Pintrich; King and Kitchener), with the resistance-to-disconfirmation parameter supplied by dogmatism (Rokeach) and Need for Cognitive Closure (Kruglanski and Webster, who split it into a seizing urgency and a freezing permanence). The dynamics have rigorous, mostly integer-friendly models: AGM entrenchment for what holds firm, the Friedkin-Johnsen opinion model (each agent anchored to its own innate opinion by a stubbornness weight, so a population keeps lasting disagreement rather than collapsing to consensus, with DeGroot consensus as the no-stubbornness special case), bounded-confidence models (Deffuant, Hegselmann-Krause) for schism, and Boyd and Richerson's conformist and prestige transmission biases for cultural drift. Game precedent validates the multi-field, data-driven, per-culture approach (Stellaris paired ethics with attraction drift, Crusader Kings faith tenets and fervor, RimWorld memes and precepts with a per-agent certainty that flips ideology, Dwarf Fortress species-seeded values distinct from facets), but none is bit-deterministic, so their numeric machinery must be re-derived in fixed-point.

The decision taken, signed off. An axiom is a multi-field record that separates position (stance) from hold (strength, confidence, entrenchment rank) and carries salience, a Friedkin-Johnsen stubbornness anchor, an immutable heritable seed, and a bounded evidence ring (Option B with the first bounded slice of the justification model of Option C; the full provenance graph is a later goal). A separate epistemic stance (source weights over tradition, evidence, revelation, authority, and intuition, plus dogmatism, seizing, freezing, and certainty) parametrizes one fixed update kernel for every other belief. Dynamics are an AGM entrenchment gate (sub-threshold pressure is assimilated into values or fast belief facets, so perseverance and confirmation bias emerge from the gate, while supra-threshold pressure accommodates and a high-salience source-weighted event is a revelation jump), a Friedkin-Johnsen anchored average for enculturation, bounded-confidence with conformist and prestige bias for schism, calcification of unchallenged axioms, and heritable-plus-encultured inheritance. The mechanism is fixed Rust; the axiom axes, source modes, lexicons, and every coefficient are per-race data, so the model runs across races and not only cultures, and the old belief-domain registry becomes the axiom-axis registry, ending its collision with the belief facets of Part 9. This lives in Part 28, with the registry in Part 40 and sources in Part 63.

What remains to be proven. The calibrations are reserved for the owner, not fabricated, because the research returns them as design choices or as review values from human and mostly-WEIRD instruments: the evidence-ring capacity, the entrenchment-threshold curve and accommodation step, the calcification rate and brittleness, the stubbornness anchor, the confidence band, the conformity and prestige strengths, the fission and deviation thresholds, the revelation-jump threshold, the heritable fraction and mutation spread, the per-race epistemic defaults, and how stance and strength scale a consideration bias. In-engine, it must be shown that belief perseverance and a clean labile-to-calcified phase transition emerge from the gate, that populations reach persistent disagreement rather than consensus, that sects fracture and child bands resemble both parents and local culture, and that the whole update is bit-identical across thread counts and machines, which requires a fixed rounding convention and canonical aggregation order.

### 62.5 How to model heredity, variation, and deep-time evolution, deterministically and for all life

The question. The being model needed a genome and an inheritance model for the trait rule of Part 20 to plug into, grounded in how heredity really works, data-driven per race rather than one hardcoded biology, extending to magical and non-biological beings, and applying to animals and plants and not only sentient races. The placeholder was a flat vector of signed-integer gene leanings with a blended-inheritance function and a provisional mutation rate, which the audit flagged as ungrounded; a separate question, raised during the knowledge-formation work, was where the agent's intelligence lives, since the design referenced it as a gate on cognition but the race parameter set had no such attribute.

What was found. Quantitative genetics supplies the deep-time-correct spine: Fisher's infinitesimal model, that a trait of many small additive loci yields an offspring genetic value normal about the midparent value, made rigorous under selection, drift, and structure by Barton, Etheridge, and Veber (2017), with Turelli (2017) clarifying that the within-family Gaussian holds even where the population-level distribution does not. The breeder's equation (Falconer and Mackay; Lynch and Walsh), Wright's fixation index and Nei's genetic distance, the Wright-Fisher drift model, and neutral theory (Kimura) give integer-expressible population dynamics; speciation biology (Coyne and Orr, the Dobzhansky-Muller model, Haldane's rule) gives reproductive isolation. General cognitive ability is the textbook polygenic, very-small-effect, rising-heritability trait (Haworth et al. 2010; Plomin and von Stumm 2018) and is empirically separable from working memory (Conway, Kane, and Engle 2003), which settles intelligence as its own channel rather than a duplicate of memory or plasticity. Game precedent gives a clear ledger: Dwarf Fortress per-individual diploid genotypes with list-order dominance and hidden carriers; RimWorld Biotech genes-as-data and xenotypes, but with an order-dependent inheritance bug to avoid; Crusader Kings hidden recessive carriers and inbreeding; Niche an honest Mendelian model on the five pillars of population genetics; and Spore the cautionary tale of form decoupled from function.

The decision taken, signed off. A two-layer model, a multi-locus quantitative spine with an optional Mendelian dominance layer per gene, wrapped in a per-race GeneticScheme that selects fixed reproduction and inheritance variants and defaults to sexual diploid. The genotype-to-phenotype map is additive contributions plus Falconer dominance plus a bounded epistasis lookup plus environment, and the Part 20 inheritance rule is provably its additive reduction, so they compose. Linkage is ordered loci with stored per-interval recombination fractions; mutation and all stochastic steps are per-locus counter-RNG; intelligence is a polygenic cognitive channel distinct from memory and belief plasticity. Deep time is Wright-Fisher drift plus breeder's-equation selection over allele-frequency pools, with genetic distance reusing the structural-distance machinery of Part 21, a reserved fertility curve and a discrete incompatibility table for isolation, and declared rather than scripted speciation. The masses run as allele-frequency pools and a promoted being carries an explicit Hardy-Weinberg-consistent genome, folded back on demotion. The model is uniform across sentient races, animals, and plants, differing only in genes and scheme. This lives in Part 25, with the gene and scheme registries in Part 40 and sources in Part 63.

What remains to be proven. The calibrations are reserved for the owner, not fabricated, because the research returns them as design choices or as review values: the per-channel heritability and loci counts, the allele effect sizes and dominance degrees, the recombination fractions, the mutation rates and step sizes, the effective population size, the selection scaling for domestication and disease, the distance-to-fertility curve, the speciation and incompatibility thresholds, the hybrid-scheme rule, the distance-measure choice, and the integer Gaussian approximation and its precision. In-engine it must be shown that neutral drift matches the Wright-Fisher variance, that a promote-then-demote round trip leaves allele frequencies unbiased, that two isolated pools cross the speciation threshold on the molecular-clock timescale, and that the whole pipeline is bit-identical across thread counts and machines. Two honest limits stand: the statistical tier loses linkage disequilibrium and family structure between promotions, and magical inheritance is a bespoke dispatched rule rather than a parameterization of the standard mechanism.

### 62.6 How a language emerges from the dawn of sentience without steering, and stays legible

The question. The language system had a settled structural spine (per-culture phonology, morphology, grammar, and a drifting lexicon, with a hard observer-legibility guarantee) but six open questions beneath it: what a concept is and how the concept space stays extensible when nothing is authored, how mutual intelligibility is measured, whether multilingualism and interpreters are modelled per being, whether a language barrier distorts belief or only slows it, how writing and literacy emerge and where they live, and the generation and drift internals with their determinism. The hardest was origination: whether the first language is seeded from a generator or bootstrapped from nothing by agents coordinating a code into existence. The owner's conditions were that emergence reach all the way down to sounds and word-forms, that concepts be emergent so cultures carve the world their own way, that an external interpretation layer stay non-authoritative, that each race be able to produce its own sounds, and that the legibility guarantee survive.

What was found. The science establishes that a shared, structured code self-organises from local coordination, and fast: iterated-learning experiments and models (Kirby, Cornish, and Smith) show compositional structure emerging through a learning bottleneck, naming-game and Talking Heads work (Steels and colleagues) shows a population converging a shared lexicon from scratch, vowel systems emerge from imitation games under dispersion pressure (de Boer), and Nicaraguan Sign Language is the real-world proof that a full language emerges fresh from no linguistic input. A ready grounding substrate exists in the Natural Semantic Metalanguage (Wierzbicka and Goddard), a small fixed set of semantic primes lexicalised across languages, which answers the symbol-grounding problem (Harnad) by bottoming meaning out in non-symbolic features; semantic typology (Berlin and Kay and the World Color Survey; Levinson on spatial frames) shows a shared space variably partitioned, which is exactly the emergent-concept model. Language distance is a solved structural-distance problem (the ASJP program's normalised Levenshtein distance over core vocabulary), sound change is regular and so a deterministic rewrite (the Neogrammarian hypothesis; the Zompist and Lexurgy appliers), grammaticalisation gives a one-way drift cline (Hopper and Traugott), and typology gives the plausibility priors (Dryer and WALS on word order and harmony; Maddieson on inventory sizes). Age of acquisition is graded with a late-teen decline (Hartshorne, Tenenbaum, and Pinker). Game precedent confirms the approach: Dwarf Fortress's per-culture languages with an English-root translation layer are beloved but use an authored concept list and are not bit-deterministic, Caves of Qud and Ultima Ratio Regum show gloss-with-gist and generate-then-cache working, and No Man's Sky shows the hollow-name failure when a generated word points at nothing simulated.

The decision taken, signed off. A concept is a deterministic integer region over a shared semantic substrate (primes plus projected world and relation features), emergent and per-culture, drifting and splitting and merging, with a deterministic engine-side English gist gloss that never depends on the external interpretation layer, which is view-only and type-quarantined against backfeed. Sounds are per-race: the phonetic substrate is data and each race has a producible sound set fixed by its anatomy and genome, so it can voice sounds others cannot and its language takes shapes others cannot, the producible set growing as anatomy evolves. Language distance reuses the value-distance machinery as three fixed-point components feeding mutual intelligibility as friction; multilingualism is per individual for the promoted and aggregate for pools with an emergent interpreter role and a cross-race production ceiling; a barrier distorts belief by concept-snapping, false-cognate error, and nuance loss into the belief and theory-of-mind systems; writing is invented in the technology layer but its script and its effect on belief live in the language system, the script type emerging from phonological complexity and a written record cutting decay and locking provenance. Origination is the hybrid: a thin bounded coordination dynamic sets the first anchors and sound system through agent coordination, then hands off to seeded generation and drift, collapsing to pure seeding at a zero round cap. This lives in Part 33, with the substrates and per-race sounds in Parts 40 and 20 and sources in Part 63.

What remains to be proven. The calibrations are reserved, surfaced rather than fabricated, each with a basis: the discrimination and lexicalisation thresholds and the concept drift rate; the phoneme inventory priors and each race's producible set (with the anatomy interface itself reserved); the word-order and morphology sampling and the disharmony probability; the drift operator rates; the three distance weights (lexical dominant); the acquisition rate, aptitude range, and age breakpoint; the mistranslation budget; the writing-invention threshold, script-type weights, literacy spread, and record fidelity modifiers; and the dawn dynamic's round cap and anchor-set size. In-engine it must be shown that two differently-seeded cultures partition the same domain differently while staying comparable on the substrate, that any emergent concept including one over a freshly invented artifact receives a legible gist, that dialect chains show graded intelligibility while separate families sit at the floor, that removing the interpretation layer changes zero canonical state, that descent produces real family trees, and that the whole pipeline is bit-identical across thread counts and machines.

### 62.7 How a belief about the world forms from evidence, generally and data-driven

The question. Part 9 settled how a belief moves between minds and distorts but not how a belief about an event forms from the world at all, which the perfect-crime case exposes: a hidden death is ground truth in the log, yet no mind should know it until evidence or inference produces a belief, and the killer can suppress that by concealing the traces. The owner's condition on accepting the answer was that it not be a whodunit system but a general, data-driven evidence engine that serves any task requiring evidence gathering.

What was found. The right inference rule is an integer log-odds accumulator, additive and order-independent and exactly representable in fixed point, the occupancy-grid update of Elfes and of Thrun, Burgard, and Fox, rather than a normalised Dempster-Shafer combination, whose conflict normalisation divides by a data-dependent near-zero quantity that Zadeh showed is unstable and that is not exact in fixed point; an unnormalised conjunctive form survives as an option. Inference to the best explanation (Peirce; Harman; Lipton) supplies the commit test, best and clearly better than the runner-up, guarding against the best of a poor set. Absence is grounded in negation as failure under a locally closed world (Reiter; Clark) and in the rebuttable presumption of death in absentia, whose waiting periods span seven years at English common law down to three to five by statute and accelerate under evident peril, which is the basis for a visibility-scaled window. Physical traces are the forensic exchange principle made literal. The strongest precedent is Shadows of Doubt, a fully simulated murder with physically placed clues; Dwarf Fortress demonstrates the unwitnessed-crime case and models wrongful conviction from grudge-driven false reports as a feature; Crusader Kings supplies the discoverable-secret loop; and Talk of the Town is the belief model already adopted.

The decision taken, signed off and generalised. A belief forms about any question, a subject and attribute with a hypothesis frame, by accumulating evidence from observation of traces, testimony, inference, and absence. Traces are data-defined perceptible event consequences; the inference engine is an integer log-odds accumulator with an explicit unknown and a best-and-clearly-better commit test, parametrised by genome acuity and epistemic stance, committing defeasible inferred facets that propagate and revise like any belief; absence is first-class evidence detected by a lazy last-seen tick and an ordered timer queue with a visibility-scaled, data-defined escalation schedule; concealment is utility-driven suppression of trace perceptibility, distinct from lying; investigation is a utility goal any motivated agent runs, with false conclusions an intended outcome; and the aggregate tier diffuses knowledge with delay. The trace kinds, evidence weights, hypothesis frames, and absence schedules are all data registries (Part 40), siblings of the mutation graph, so who-killed-whom is one configured instance alongside prospecting, parentage, scouting, trade trust, and the causal inquiry that seeds emergent science. This lives in Part 9, with the registries in Part 40 and sources in Part 63.

What remains to be proven. The calibrations are reserved, surfaced rather than fabricated: the per-implication evidence weights, the commit threshold and runner-up margin, the certainty clamp, the trace salience and decay curves, the absence windows and their visibility scaling, the presumption strengths, the concealment multipliers and costs, the acuity and stance couplings, and the aggregate diffusion rate and knowledge-to-strength mapping. In-engine it must be shown that a witnessed event commits a conclusion quickly while a concealed one reaches at most a missing presumption through absence, that adding and removing tracked subjects and re-sightings never changes results across thread counts, that false accusations occur at a low but intended rate, and that a promoted mind's knowledge is consistent with what its pool knew.

### 62.8 Representing emergent institutions without an authored kind

The question. Part 36 holds that institutions are emergent in the strong sense, yet the Institution struct carried a kind field typed as a closed five-way enum (governance, faith, guild, legal, market). That was the sharpest internal contradiction in the document: it predefined the categories a people could fall into, against Principle 8, and authored a cultural outcome, against Principle 9, leaving a scholarly academy, a military order, a caste, a secret society, or a bank with nowhere to live. The owner asked to trace every read-site of the tag so the cost was known, then to decide between data-defining the kinds and dropping the tag entirely, and signed off on the condition that the result be maximally emergent and differentiable per race.

What was found. The read-surface is thin, so removing the tag is cheap and the real work is replacing the classification function it served, legibility, cross-system reasoning, and cross-cultural comparison. The engine had already solved the underlying problem twice, with the artifact etic-and-emic split (Part 41) and the concept semantic substrate (Part 33), and institutional theory points the same way: Ostrom and Crawford's ADICO grammar gives a structural, emergent representation of a rule in which the statement's type falls out of which components are present; Greif models a guild as a self-enforcing equilibrium of belief and behaviour rather than a labelled box; DiMaggio and Powell explain why recognizable forms recur as isomorphism, an emergent regularity rather than a primitive; and polythetic classification, family resemblance, and prototype theory (Needham, Wittgenstein, Rosch) argue a category is a graded feature-cluster with no necessary-and-sufficient definition, which is a recognizer and not an enum. Scott's legibility critique warns that imposing categories on emergent social order distorts, which is exactly why any type must be a surfaced approximation and never authoritative. Game precedent is uniformly cautionary: across Dwarf Fortress, Victoria 3, Crusader Kings, RimWorld, Songs of Syx, and Caves of Qud the category space is always authored and only instances emerge, so the emergent-recognition path is novel.

The decision taken, signed off and hardened. The authored kind is dropped entirely; an institution's identity is its emergent structure (roles, ADICO-grammar norms, the function it coordinates, legitimacy, lineage, resources), and any type is recovered only as a derived, non-authoritative etic descriptor for legibility, with the culture's own emic name beside it. What an institution coordinates is an emergent blend over an institution-function substrate, a data registry of function axes that is the etic floor of what coordination can be about, sibling to the value and semantic substrates and authorable under Principle 9 as an affordance, and this is where the deepest per-race difference enters, since an exotic people can carry exotic axes and crystallize institutions structurally unlike any other race's. Membership-gating and succession are emergent norms rather than authored sub-enums. The descriptor is a fixed-point polythetic match against a library of descriptive recognition templates that recognize but never generate, a configuration matching none reads as generic, and the owner may ship no templates and the engine still runs. Cross-system reasoning is over function and capability, comparison reuses the value and language distance machinery as an institution-distance, and the model carries to the aggregate tier as a conserved feature vector. The substrate and templates are added to Part 40; the mechanism lives in Part 36 and the sources in Part 63.

What remains to be proven. The calibrations are reserved: the membership of the function-substrate axes, the feature weights in the similarity and distance metrics, the recognition threshold, the crystallization thresholds and rates by which a coordination pattern becomes an institution, and the recognition template library itself. In-engine it must be shown that the five former variants and the cases that had nowhere to go all arise as configurations and are recognized or read generically without ever being blocked, that recognition and distance are bit-identical across machines and thread counts, that a promoted institution reproduces its pool's feature vector, and that crystallization fires at a believable rate, which is the hardest open tuning problem.

### 62.9 The tier-consistency keystone and aggregate decision provenance

The question. A settlement runs at two canonical tiers, detailed focus-scale simulation and aggregate pool statistics, and Part 54 named the agreement between them, that instantiating a region from its summary and re-summarizing it must not depend on whether it was watched, as the keystone to prove before systems are built on it. The architectural ruling was already made (significance and the seed drive canonical fidelity, never the camera, with a non-authoritative elaboration for viewing), so the open work was the mechanism and the exact guarantee, plus the decision-provenance subproblem of how a collective undertaking at the aggregate tier comes from the settlement's evolved authority rather than a draw over demographics. The owner asked to confirm it is broadly generalizable.

What was found. The two boundary operations are exactly the lifting and restriction operators of multiscale computation (the equation-free and heterogeneous multiscale methods of Kevrekidis, Gear, Samaey, and of E and Engquist), where a fine and a coarse model are coupled by an operator each way, and the consistency condition is a relation between the operators, restriction-after-lifting an identity on the coarse state while lifting-after-restriction is lossy. Identical outcomes between the tiers is mathematically unattainable for nonlinear dynamics: exact consistent aggregation requires linearity (the aggregation results of Leontief and Theil), so a statistical model and a per-agent model cannot be made to coincide on micro-outcomes. The established substitute is to preserve invariants rather than trajectories, the conservation-preserving reduction that structure-preserving model-order reduction demonstrates. Counter-based stateless RNG (the Random123 line) plus fixed-point integer math and canonical id ordering give bit-identical results across machines and threads, and seeded coordinate-hash instantiation gives reproducible on-demand generation. Collective decisions need an institutional grammar, and the Ostrom-Crawford ADICO grammar already adopted for institutions is it. Game precedent is cautionary: no surveyed title guarantees both tier-consistency and seed-reproducibility, and Dwarf Fortress's documented cross-tier referential-integrity bugs are exactly what an audited invariant prevents.

The decision taken, signed off and generalized. The keystone is resolved as a lifting operator (instantiate-from-summary) and a restriction operator (re-summarize) under a conservation-plus-seeded-significance-schedule guarantee, not identical outcomes. Restriction-after-lifting is exact on conserved state and audited every transition; every transition conserves, in integer and fixed-point, a registry of conserved projections each two-tier subsystem declares for itself, so present and future two-tier systems are covered without special-casing (the generalization added on the owner's condition); each tier is bit-reproducible given the camera-free significance schedule; and the canonical timeline is a function of seed and world alone, satisfying determinism and observer-independence without the impossible identical-outcomes requirement. Collective undertakings are emitted through the aggregate representation of the emergent governing institution and its ADICO norms, legitimacy, and coordinated function, a coarse projection of the same authority the detailed tier deliberates through, so the mechanism inherits the institution model's full per-race generality and assumes no shape of rule. Identity promotion and demotion apply only to entities without permanent identity, the property per-entity and per-race data so exotic identity structures like a hive whose individual is the colony are handled by the same rule; permanent-identity entities scale only in processing fidelity, the coarse update being the restriction of the full update on conserved facets, and the belief level-to-strength lifting and id-ordered restriction conserve belief mass and resolve the mapping deferred from the evidence work. This lives in Part 54, with the conserved-projection registry in Part 58, the decision coupling in Part 8, and the belief coupling in Part 9.

What remains to be proven. The calibrations are reserved: the significance thresholds that drive promotion and demotion, the decision-propensity threshold and accumulation rate, the belief level-to-strength curve and dispersion, the partition remainder rule, an optional distributional-agreement tolerance if ever wanted, and the base-tick duration. In-engine it must be shown that random promote-then-demote cycles conserve every declared quantity exactly, that a region's canonical tiers are bit-identical across runs differing only in the camera path, that the aggregate institution chooses the same class of undertaking the detailed tier would under the same conditions, and that the audited referential-integrity harness catches a leak or dangling reference the moment it appears. Identical-outcomes agreement is not provided and cannot be, the fast-forward through busy time remains the separate temporal-LOD problem, and the non-authoritative view elaboration (R-VIEW-ELAB) remains open as the sibling mechanism that must stay consistent with the canonical tiers without writing them.

### 62.10 Recursive technology composition (R-DEEPTECH-COMPOSE)

The question. Part 41 encodes a single artifact as a composition of form primitives, a material, and a joining technique, one level deep, and the honest-limits note flagged the open-ended invention of complex multi-part technologies as the partly-unsolved part. For capability to climb without an authored ceiling, a conceived-and-physics-validated artifact or technique must become a reusable building block an agent can reference as a component inside a larger intent, recursively, so a machine is a composition of components that are themselves compositions. Three things had to be decided: the recursive representation, the criterion by which a module is promoted to a reusable primitive, and the method by which physics or use evaluation composes across levels. The owner asked to confirm it is broadly generalizable and per-race differentiable.

What was found. The artificial-life work on generative and compositional encodings is decisive that complexity scales only when validated sub-structures are reused as labelled units: Hornby and Pollack's generative representation, whose abstraction property is the ability to label a compound element and manipulate it as a unit with parameters, gives higher fitness and more regular, scalable structure than a direct encoding that uses each element once. Program synthesis supplies the promotion criterion: DreamCoder grows a library by abstracting out the sub-components that compress the description length of the whole task corpus, the multi-task reuse that bootstraps harder problems. The biology of major evolutionary transitions and exaptation supplies encapsulation, a stabilised lower unit becoming a hidden building block of the next level and a part being repurposed when its interface fits, and engineering modularity theory supplies the interface discipline, Simon's near-decomposability in which stable intermediate forms assemble faster and come to dominate, Parnas's information hiding, Baldwin and Clark's visible-rules-versus-hidden-modules, Ulrich's interface specification, and Suh's independence axiom. Cultural-evolution theory supplies the stability gate, Tomasello's ratchet, in which the hard part is faithful transmission that prevents slippage backward rather than invention. Automation games such as Factorio are the cautionary precedent: their tiered-intermediate dependency graphs and recipe ratios are authored by hand node by node, the shape to reach emergently rather than to author.

The decision taken, signed off and generalized. A technology is a content-addressed composition node, integer and enum, that is a leaf bottoming out at the data-defined primitive-material-join floor or a composite referencing validated child modules by stable id, with a fixed-width interface vector and bound parameters; the conceived intent is the emergent need-driven intent of the Part 41 lifecycle and never an authored enum, which is the first generalization hardening, since the report had drawn it as an enum where Part 41 conceives intents emergently. A validated module is promoted to a reusable primitive only when three gates pass together, physics validation, stabilisation in transmission through the per-culture cultural ratchet, and compression of the corpus by reuse, the conjunction bounding the combinatorial explosion by growing the next level's alphabet only with durable and economical units, and the per-culture stability gate making technological trajectories diverge by people for free, which is the per-race differentiation the owner required. Evaluation is a memoised bottom-up pass that aggregates fixed-width integer interval vectors by typed combinators drawn from a data-defined extensible registry, which is the second and main generalization hardening, since the report had drawn the combinators as a fixed set of four, adds data-defined whole-system emergent proxies for the properties no part carries, and charges a graded interface-mismatch penalty, with memoisation on the content id letting the deep-time aggregate tier carry a compact library rather than re-search. The interface-axis substrate, the leaf floor, the combinator registry, and the emergent-proxy set are framed as the etic grounding floor of composition, sibling to the value, semantic, and institution-function substrates and extensible with the physics, which is the third hardening. This lives in Part 41.

What remains to be proven. The calibrations are reserved: the depth and node-count bounds, the viability threshold, the stability span and drift radius, the reuse-and-compression threshold, the interface-mismatch penalty curve, the emergent-proxy weights, the interval-width rejection threshold, and the evaluation channel set. In-engine it must be shown that two cultures from different seeds grow technology graphs of different shape and depth under one physics (the non-templatedness test), that evaluating a deep system costs in the new nodes alone (the memoisation test), and that the promoted-library size grows sub-linearly in validated-artifact count (the compression-gate test). Near-decomposability remains an assumption the interval widths flag but cannot resolve, the integer reuse count is a cheap surrogate for the description-length ideal, the reach equals the reach of the physics substrate and the proxy set, and the contact with technique-origination is the unresolved Inconsistency 5. The composition piece is settled; the four deeper cluster questions, emergent science, the discoverable physics substrate, emergent prerequisite depth as a standalone proof, and deep-time aggregate scale, remain open.

---

## Part 63: References and Academic Grounding

A starting bibliography for the systems in this document, weighted toward the emergent technology design space (Part 41), the belief and folklore subsystem (Part 9), and the cultural-evolution and procedural-generation foundations the design rests on. It is provided as a research footing, since the project is also of interest as the basis for a paper. The canonical works below are well established, but exact editions, venues, page ranges, and identifiers should be confirmed against the primary sources before formal citation; this list has not been re-verified entry by entry in this pass.

**Design theory and functional representation**
- Gero, J. S. (1990). Design prototypes: a knowledge representation schema for design. AI Magazine, 11(4). The function-behaviour-structure (FBS) ontology underpinning Part 41's conceived-intent representation.
- Gibson, J. J. (1979). The Ecological Approach to Visual Perception. The theory of affordances, relevant to the affordance-versus-outcome boundary of Principle 9.

**Artificial life, evolutionary and generative design**
- Sims, K. (1994). Evolving virtual creatures. SIGGRAPH '94. The founding demonstration of physics-based evolution of morphology and control, the model for physics-as-fitness.
- Hornby, G. S., and Pollack, J. B. (2002). Creating high-level components with a generative representation for body-brain evolution. Artificial Life, 8(3). Generative (indirect) representations outperforming direct encodings.
- Stanley, K. O. (2007). Compositional pattern producing networks: a novel abstraction of development. Genetic Programming and Evolvable Machines, 8(2).
- Stanley, K. O., D'Ambrosio, D. B., and Gauci, J. (2009). A hypercube-based encoding for evolving large-scale neural networks (HyperNEAT). Artificial Life, 15(2).

**Open-endedness, novelty, and quality-diversity search**
- Lehman, J., and Stanley, K. O. (2011). Abandoning objectives: evolution through the search for novelty alone. Evolutionary Computation, 19(2). Novelty search.
- Mouret, J.-B., and Clune, J. (2015). Illuminating search spaces by mapping elites. arXiv:1504.04909. MAP-Elites.
- Stanley, K. O., and Lehman, J. (2015). Why Greatness Cannot Be Planned: The Myth of the Objective. Springer. The argument that ambitious targets are deceptive and functional results come from collecting stepping stones, the philosophical basis for convergence without a target.
- Wang, R., Lehman, J., Clune, J., and Stanley, K. O. (2019). Paired Open-Ended Trailblazer (POET). arXiv:1901.01753. Co-evolution of problems and solutions, the niche as implicit objective.

**Theoretical morphology and convergent evolution**
- Raup, D. M. (1966). Geometric analysis of shell coiling: general problems. Journal of Paleontology, 40(5). The shell morphospace, the model for an artifact design space with attractors.
- McGhee, G. R. (2007). The Geometry of Evolution: Adaptive Landscapes and Theoretical Morphospaces. Cambridge University Press.
- von Salvini-Plawen, L., and Mayr, E. (1977). On the evolution of photoreceptors and eyes. Evolutionary Biology, 10. The independent origins of eyes, the canonical case of convergence without a target.

**Procedural content generation**
- Togelius, J., Yannakakis, G. N., Stanley, K. O., and Browne, C. (2011). Search-based procedural content generation: a taxonomy and survey. IEEE Transactions on Computational Intelligence and AI in Games, 3(3).
- Shaker, N., Togelius, J., and Nelson, M. J. (2016). Procedural Content Generation in Games. Springer. The expressivity-versus-validity framing.

**Cultural evolution and transmission**
- Boyd, R., and Richerson, P. J. (1985). Culture and the Evolutionary Process. University of Chicago Press. Biased transmission (content, prestige, conformity).
- Tomasello, M. (1999). The Cultural Origins of Human Cognition. Harvard University Press. The ratchet effect of cumulative culture.
- Henrich, J. (2004). Demography and cultural evolution: how adaptive cultural processes can produce maladaptive losses, the Tasmanian case. American Antiquity, 69(2). The mechanism of technological loss under population decline.
- Eerkens, J. W., and Lipo, C. P. (2005). Cultural transmission, copying errors, and the generation of variation in material culture and the archaeological record. Journal of Anthropological Archaeology, 24(4). The accumulated-copying-error model and the three percent Weber-fraction reference.
- Mesoudi, A. (2011). Cultural Evolution: How Darwinian Theory Can Explain Human Culture and Synthesize the Social Sciences. University of Chicago Press.
- Howe, C. J., and Windram, H. F. (2011). Phylomemetics: evolutionary analysis beyond the gene. PLoS Biology, 9(5). Treating artifact and text traditions as evolving lineages, noted as an option in Part 41.

**Emergent narrative, character knowledge, and generated-artifact readability**
- Ryan, J. O., Mateas, M., and Wardrip-Fruin, N. (2015). Toward characters who observe, tell, misremember, and lie. AIIDE Workshop on Experimental AI in Games. The Talk of the Town knowledge model behind Part 9.
- Ryan, J. O., et al. (2016). Bad News: a game of death and communication. CHI Extended Abstracts. The playable demonstration of that model.
- Compton, K., Kybartas, B., and Mateas, M. (2015). Tracery: an author-focused generative text tool. International Conference on Interactive Digital Storytelling. The grammar-realization layer for Parts 39 and 41.
- Grinblat, J., and Bucklew, C. B. (2017). Subverting historical cause and effect: generation of mythic biographies in Caves of Qud. Foundations of Digital Games PCG Workshop. The proven precedent for surfacing generated artifacts in a consistent voice.

**Personality, its development, and behaviour genetics (Parts 20, 62.2)**
- Fleeson, W. (2001). Toward a structure- and process-integrated view of personality: traits as density distributions of states. Journal of Personality and Social Psychology, 80(6). The finding that a trait is a distribution of momentary states, behind the setpoint-plus-reactivity representation.
- Fleeson, W., and Jayawickreme, E. (2015). Whole Trait Theory. Journal of Research in Personality, 56. The descriptive distribution joined to explanatory social-cognitive machinery.
- Mischel, W., and Shoda, Y. (1995). A cognitive-affective system theory of personality. Psychological Review, 102(2). The if-then situation-behaviour signatures behind the contingency layer.
- McAdams, D. P., and Pals, J. L. (2006). A new Big Five: fundamental principles for an integrative science of personality. American Psychologist, 61(3). The three-level layering of traits, characteristic adaptations, and narrative identity.
- DeYoung, C. G., Quilty, L. C., and Peterson, J. B. (2007). Between facets and domains: ten aspects of the Big Five. Journal of Personality and Social Psychology, 93(5). The hierarchy from metatraits through aspects to facets.
- Ashton, M. C., and Lee, K. (2007). Empirical, theoretical, and practical advantages of the HEXACO model of personality structure. Personality and Social Psychology Review, 11(2). The sixth Honesty-Humility factor.
- Schwartz, S. H. (1992). Universals in the content and structure of values. Advances in Experimental Social Psychology, 25; and Schwartz et al. (2012). Refining the theory of basic individual values. Journal of Personality and Social Psychology, 103(4). The circular structure of basic values, the separate-but-linked value layer.
- Roberts, B. W., and DelVecchio, W. F. (2000). The rank-order consistency of personality traits from childhood to old age. Psychological Bulletin, 126(1). The rising rank-order stability curve the change mechanism must reproduce.
- Roberts, B. W., Walton, K. E., and Viechtbauer, W. (2006). Patterns of mean-level change in personality traits across the life course. Psychological Bulletin, 132(1). The maturity-direction drift.
- Bleidorn, W., et al. (2022). Personality stability and change: a meta-analysis of longitudinal studies. Psychological Bulletin, 148(7-8). Updated stability-and-change estimates revising the post-25 picture.
- Roberts, B. W., et al. (2017). A systematic review of personality trait change through intervention. Psychological Bulletin, 143(2). The intervention-driven change effect behind the self-change burst.
- Lodi-Smith, J., and Roberts, B. W. (2007). Social investment and personality: a meta-analysis. Personality and Social Psychology Review, 11(1). The role-investment mechanism, flagged as contested.
- Polderman, T. J. C., et al. (2015). Meta-analysis of the heritability of human traits based on fifty years of twin studies. Nature Genetics, 47(7). The near-one-half, mostly-additive heritability behind the inheritance rule.
- Gurven, M., et al. (2013). How universal is the Big Five? Testing the five-factor model of personality variation among forager-farmers in the Bolivian Amazon. Journal of Personality and Social Psychology, 104(2). The Tsimane non-replication, the argument for data-driven axes.
- Gosling, S. D., and John, O. P. (1999). Personality dimensions in nonhuman animals: a cross-species review. Current Directions in Psychological Science, 8(3); with Réale et al. (2007, 2010) on animal personality and pace-of-life syndromes. The cross-species temperament palette behind the animal and great-beast tiers.

**Value structure and metric (Parts 21, 62.3)**
- Schwartz, S. H., and Bardi, A. (2001). Value hierarchies across cultures: taking a similarities perspective. Journal of Cross-Cultural Psychology, 32(3). The pan-cultural similarity in value priorities, alongside the Schwartz circular structure cited in the personality group above.
- Perrinjaquet, A., et al. (2007). A test of the quasi-circumplex structure of human values. Journal of Research in Personality, 41(4). The structural-equation test that rejects the strict circumplex, the reason structure is treated as per-race data rather than a fixed ring.
- Graham, J., Haidt, J., et al. (2013). Moral foundations theory: the pragmatic validity of moral pluralism. Advances in Experimental Social Psychology, 47. The near-independent-foundations alternative to a ring, motivating the Independent structure.
- Hofstede, G. (2001). Culture's Consequences, 2nd ed. Sage. The low-dimensional national-culture value space, another non-ring structure.
- Inglehart, R., and Welzel, C. (2005). Modernization, Cultural Change, and Democracy. Cambridge University Press. The two-axis cultural map.
- Rokeach, M. (1973). The Nature of Human Values. Free Press. Values as a rank-ordered hierarchy, the list rather than the space.
- Fiske, A. P. (1992). The four elementary forms of sociality. Psychological Review, 99(4). Relational structure behind authored cross-race projections.
- Villani, C. (2009). Optimal Transport: Old and New. Springer. The optimal-transport foundation for distance between distributions over a ground metric.
- Le, T., Yamada, M., Fukumizu, K., and Cuturi, M. (2019). Tree-sliced variants of Wasserstein distances. Advances in Neural Information Processing Systems, 32. The closed-form tree earth mover's distance used where full value distributions exist.
- Dijkstra, E. W. (1959). A note on two problems in connexion with graphs. Numerische Mathematik, 1; and Floyd, R. W. (1962). Algorithm 97: shortest path. Communications of the ACM, 5(6). The shortest-path algorithms that compile a value graph into the exact integer ground-metric table.
- Ramdas, A., García Trillos, N., and Cuturi, M. (2017). On Wasserstein two-sample testing and related families of nonparametric tests. Entropy, 19(2). The statistical reading of transport distance between value distributions.

**Axiomatic and worldview belief, epistemic stance, and belief dynamics (Parts 28, 62.4)**
- Rokeach, M. (1960). The Open and Closed Mind. Basic Books. The central-to-peripheral belief architecture and dogmatism, the basis for separating a belief's position from how firmly it is held.
- Leung, K., Bond, M. H., et al. (2002). Social axioms: the search for universal dimensions of general beliefs about how the world functions. Journal of Cross-Cultural Psychology, 33(3); with Leung and Bond (2004) on the pan-cultural structure. The closest published model of the engine's worldview axioms.
- Clifton, J. D. W., et al. (2019). Primal world beliefs. Psychological Assessment, 31(1). The Primals inventory (world as safe, enticing, alive) and the stability and interpretive-lens findings behind the worldview axes.
- Koltko-Rivera, M. E. (2004). The psychology of worldviews. Review of General Psychology, 8(1). The integrative account of worldview dimensions.
- Hofer, B. K., and Pintrich, P. R. (1997). The development of epistemological theories. Review of Educational Research, 67(1); with Perry (1970) and King and Kitchener's Reflective Judgment model. Personal epistemology: the source-of-knowing and certainty dimensions behind the epistemic stance.
- Kruglanski, A. W., and Webster, D. M. (1996). Motivated closing of the mind: seizing and freezing. Psychological Review, 103(2). The decomposition of need for closure into a seizing urgency and a freezing permanence, the resistance-to-disconfirmation parameters.
- Alchourron, C. E., Gardenfors, P., and Makinson, D. (1985). On the logic of theory change: partial meet contraction and revision functions. Journal of Symbolic Logic, 50(2); with Gardenfors and Makinson on epistemic entrenchment. The AGM belief-revision spine and the entrenchment ordering that decides what yields.
- Friedkin, N. E., and Johnsen, E. C. (1990). Social influence and opinions. Journal of Mathematical Sociology, 15(3-4); with DeGroot (1974), Reaching a consensus, Journal of the American Statistical Association, 69(345), as the no-stubbornness special case. The anchored-averaging opinion model with per-agent stubbornness behind the enculturation rule.
- Deffuant, G., et al. (2000). Mixing beliefs among interacting agents. Advances in Complex Systems, 3; and Hegselmann, R., and Krause, U. (2002). Opinion dynamics and bounded confidence. Journal of Artificial Societies and Social Simulation, 5(3). The bounded-confidence mechanism behind schism and clustering.
- Boyd, R., and Richerson, P. J. (1985). Culture and the Evolutionary Process. University of Chicago Press; with Henrich and Boyd (1998) on conformist transmission and Henrich and Gil-White (2001) on prestige bias. The transmission biases behind cultural drift and boundary sharpening.
- Axelrod, R. (1997). The dissemination of culture. Journal of Conflict Resolution, 41(2). The integer feature-and-trait model of homophily and assimilation producing stable distinct cultural regions.

**Language models as non-authoritative renderers**
- Gallotta, R., et al. (2024). Large language models and games: a survey and roadmap. IEEE Transactions on Games. Representative of the recent and fast-moving literature on quarantining model output from canonical game state via function calling and external authoritative stores; treat this group as current-as-of-survey and verify before formal use.

**Genetics, inheritance, and deep-time evolution (Parts 25, 62.5)**
- Fisher, R. A. (1918). The correlation between relatives on the supposition of Mendelian inheritance. Transactions of the Royal Society of Edinburgh, 52. The infinitesimal model: many small additive loci yield a trait normally distributed about the midparent value, the basis for the quantitative spine and its reduction to the Part 20 rule.
- Barton, N. H., Etheridge, A. M., and Veber, A. (2017). The infinitesimal model: definition, derivation, and implications. Theoretical Population Biology, 118; with Turelli, M. (2017), Fisher's infinitesimal model: a story for the ages, same issue. The rigorous derivation that offspring are Gaussian about the midparent with half the parental variance within families, even under selection where the population distribution is not Gaussian.
- Falconer, D. S., and Mackay, T. F. C. (1996). Introduction to Quantitative Genetics, 4th ed. Longman. The a and d dominance parameterization, narrow-sense heritability as the offspring-on-midparent regression, and the breeder's equation, the working equations of the genotype-to-phenotype map and selection.
- Lynch, M., and Walsh, B. (1998). Genetics and Analysis of Quantitative Traits. Sinauer. The comprehensive treatment of additive, dominance, and epistatic variance and multi-locus inheritance behind the map.
- Wright, S. (1949). The genetical structure of populations. Annals of Eugenics, 15; and Nei, M. (1972). Genetic distance between populations. The American Naturalist, 106(949). The fixation index and genetic-distance measures reused through the structural-distance machinery for divergence.
- Kimura, M. (1983). The Neutral Theory of Molecular Evolution. Cambridge University Press. Substitution rate equals the per-individual mutation rate independent of population size, the molecular clock that sets the divergence pace.
- Coyne, J. A., and Orr, H. A. (2004). Speciation. Sinauer; with the Bateson-Dobzhansky-Muller incompatibility model and Haldane, J. B. S. (1922) on hybrid sterility in the heterogametic sex. Reproductive isolation as a distance curve plus a discrete incompatibility table, and the data-defined asymmetry of Haldane's rule.
- Haworth, C. M. A., et al. (2010). The heritability of general cognitive ability increases linearly from childhood to young adulthood. Molecular Psychiatry, 15(11); with Plomin, R., and von Stumm, S. (2018), The new genetics of intelligence, Nature Reviews Genetics, 19(3). Cognitive ability as a highly polygenic, very-small-effect, rising-heritability trait, the basis for treating intelligence as a quantitative channel.
- Conway, A. R. A., Kane, M. J., and Engle, R. W. (2003). Working memory capacity and its relation to general intelligence. Trends in Cognitive Sciences, 7(12). The strong but partial correlation that justifies reasoning acuity and memory as separate channels.
- Game and simulation precedent for heritable variation at scale: Dwarf Fortress (per-individual diploid genotypes, list-order dominance, hidden carriers), RimWorld Biotech (genes-as-data and xenotypes, with an order-dependent inheritance bug to avoid), Crusader Kings (hidden recessive carriers, inbreeding), Niche: a Genetics Survival Game (an explicit Mendelian model on the five population-genetics pillars), Thrive (statistical auto-evolution of unobserved species), and Spore (the cautionary tale of form decoupled from function). Adopted and avoided as noted in Parts 25 and 62.5.

**Procedural language, emergence, and linguistic drift (Parts 33, 62.6)**
- Kirby, S., Cornish, H., and Smith, K. (2008). Cumulative cultural evolution in the laboratory: an experimental approach to the origins of structure in human language. PNAS, 105(31). The iterated-learning demonstration that compositional structure emerges through a transmission bottleneck, the basis for treating structure as emergent from coordination.
- Steels, L. (2011). Modeling the cultural evolution of language. Physics of Life Reviews, 8(4); with the Talking Heads experiment and the naming-game literature (Baronchelli et al. 2006). Self-organisation of a shared lexicon from local interaction, the model behind the dawn coordination dynamic.
- de Boer, B. (2000). Self-organization in vowel systems. Journal of Phonetics, 28(4). Realistic vowel inventories emerging from imitation games under dispersion pressure, the basis for the seed sound system.
- Senghas, A., and Coppola, M. (2001). Children creating language: how Nicaraguan Sign Language acquired a spatial grammar. Psychological Science, 12(4); with Kegl, Senghas, and Coppola (1999). The real-world emergence of a full language from no linguistic input, evidence that fresh emergence is fast and structured.
- Wierzbicka, A., and Goddard, C. (2014). Words and Meanings: Lexical Semantics across Domains, Languages, and Cultures. Oxford University Press; with Wierzbicka (2021) on the closure of the prime set at 65. The Natural Semantic Metalanguage of semantic primes and molecules, the grounding substrate for concepts.
- Harnad, S. (1990). The symbol grounding problem. Physica D, 42(1-3). Why symbols must bottom out in non-symbolic categorical features, the argument for a grounding floor under the concept substrate.
- Berlin, B., and Kay, P. (1969). Basic Color Terms; with the World Color Survey (Kay, Berlin, Maffi, Merrifield, Cook). The constrained variation in how languages partition a shared perceptual space, the model for emergent, cross-culturally varying concepts.
- Levinson, S. C. (2003). Space in Language and Cognition. Cambridge University Press; with Levinson and Meira (2003) on topological spatial categories. A single similarity space variably subdivided across languages, the template for emic partitions over a shared etic space.
- Holman, E. W., Wichmann, S., et al. (2008 onward). The ASJP program and the use of normalised Levenshtein distance over core vocabulary to measure language distance. The integer edit-distance basis for the lexical component of language distance and its inverse relation to mutual intelligibility.
- Osthoff, H., and Brugmann, K. (1878). The Neogrammarian regularity hypothesis, that sound laws admit no exceptions; with Hopper, P., and Traugott, E. (2003), Grammaticalization, Cambridge University Press. Regular sound change as a deterministic rewrite and the one-way grammaticalisation cline, the drift operators.
- Dryer, M. S. (1992). The Greenbergian word order correlations. Language, 68(1); and Dryer, M. S., and Haspelmath, M. (eds.) (2013). The World Atlas of Language Structures Online; with Maddieson, I. on consonant and vowel inventory typology. The word-order distribution, harmony correlations, and inventory-size priors for typologically plausible generation.
- Hartshorne, J. K., Tenenbaum, J. B., and Pinker, S. (2018). A critical period for second language acquisition: evidence from 2/3 million English speakers. Cognition, 177. The graded age-of-acquisition effect with a late-teen decline, the basis for the second-language learning model.
- Ong, W. J. (1982). Orality and Literacy; with Goody, J. (1977), The Domestication of the Savage Mind. How writing transforms cultural memory and transmission fidelity, the basis for written records cutting belief decay and locking provenance.
- Game and simulation precedent for procedural language: Dwarf Fortress (per-culture generated lexicons with an English-root translation layer, but an authored concept list and no bit-determinism), Ultima Ratio Regum (generate-on-demand per-culture languages and scripts), Caves of Qud (readable gibberish with a gloss, history as biased accounts), and No Man's Sky (the hollow-name failure mode of recombined names with no simulated meaning). Adopted and avoided as noted in Parts 33 and 62.6.

**Belief formation from evidence: inference, absence, and traces (Parts 9, 62.7)**
- Elfes, A. (1989). Using occupancy grids for mobile robot perception and navigation. Computer, 22(6); with Thrun, S., Burgard, W., and Fox, D. (2005), Probabilistic Robotics, MIT Press. The log-odds (logit) occupancy update, additive and order-independent and clamped, the basis for the integer evidence accumulator.
- Dempster, A. P. (1968) and Shafer, G. (1976), A Mathematical Theory of Evidence; with Smets, P. (1990) on the unnormalised Transferable Belief Model and Yager, R. (1987). Evidence theory and belief functions over a hypothesis frame, adopted for the explicit-ignorance idea and the optional unnormalised conjunctive combiner.
- Zadeh, L. A. (1986). A simple view of the Dempster-Shafer theory of evidence and its implication for the rule of combination. AI Magazine, 7(2); with Sentz, K., and Ferson, S. (2002), Combination of Evidence in Dempster-Shafer Theory, a Sandia report. The instability of the normalised rule's conflict division, the reason it is rejected on the authoritative path.
- Peirce, C. S., on abduction; with Harman, G. (1965), The inference to the best explanation, Philosophical Review, 74(1); Lipton, P. (2004), Inference to the Best Explanation; and Josephson, J., and Josephson, S. (1994), Abductive Inference. The best-and-clearly-better hypothesis-selection criterion layered on the accumulator, guarding against the best of a poor set (van Fraassen).
- Reiter, R. (1978) on the closed-world assumption and (1980) on default logic; with Clark, K. (1978) on negation as failure. The formal model for treating an absent expected observation as defeasible evidence, kept local rather than global.
- The legal doctrine of presumption of death in absentia (the Cestui Que Vie Act 1666 seven-year period, the Uniform Probate Code five-year standard, shorter state statutes, and acceleration under evident peril). The basis for the visibility-scaled absence window and its escalation.
- Locard, E., the exchange principle that a contact can leave a trace. The basis for emitting perceptible, decaying physical traces from events and requiring that they be perceived to matter.
- Loftus, E., the misinformation effect and eyewitness reliability. The basis for fallible perception at the witness step and corruptible memory thereafter, already modelled as distortion and mutation.
- Ryan, J., and Mateas, M. (Talk of the Town and Bad News; Game AI Pro 3, chapter 37). The per-mind belief-facet model bounded to who learned a fact, the ancestor of Part 9.
- Game and simulation precedent for evidence and detection: Shadows of Doubt (a fully simulated murder with physically placed clues and an evidence-assembly loop, and a real perfect-crime outcome), Dwarf Fortress (unwitnessed crime leaves no suspect, interrogation as a social contest, and wrongful conviction from grudge-driven false reports as a feature), and Crusader Kings (the discoverable-secret and scheme loop). Adopted and avoided as noted in Parts 9 and 62.7.

**Emergent institutions and the classification problem (Parts 36, 62.8)**
- Ostrom, E., and Crawford, S. (1995). A Grammar of Institutions. American Political Science Review, 89(3); with Ostrom, E. (1990), Governing the Commons, Cambridge University Press. The ADICO grammar (Attributes, Deontic, aIm, Conditions, Or-else) as the structural, emergent representation of a norm in which a statement's type emerges from its components, and the design principles of long-enduring self-organized institutions.
- Greif, A. (2006). Institutions and the Path to the Modern Economy: Lessons from Medieval Trade. Cambridge University Press. Institutions as self-enforcing equilibria of belief, norm, and organization, the basis for treating a guild or coalition as emergent coordinated behaviour rather than a labelled box.
- North, D. (1990). Institutions, Institutional Change and Economic Performance. Cambridge University Press; with Acemoglu, D., and Robinson, J. (2012), Why Nations Fail. Institutions as the rules of the game, formal versus informal, and the extractive-versus-inclusive character as a derived reading over structure rather than an authored type.
- DiMaggio, P., and Powell, W. (1983). The Iron Cage Revisited: Institutional Isomorphism and Collective Rationality in Organizational Fields. American Sociological Review, 48(2); with Meyer, J., and Rowan, B. (1977) on institutionalized organizations. Why recognizable institutional forms recur, coercive, mimetic, and normative isomorphism, as an emergent regularity the engine observes rather than imposes.
- Needham, R. (1975). Polythetic Classification: Convergence and Consequences. Man, 10(3); with Wittgenstein, L. (family resemblance, Philosophical Investigations) and Rosch, E. (prototype theory). A category as a graded cluster of features with no necessary-and-sufficient definition, the basis for recognizing an institution by a polythetic, prototype match rather than enumerating kinds.
- Scott, J. C. (1998). Seeing Like a State. Yale University Press; with Service, E. (1962), Primitive Social Organization, and its modern critique. The legibility critique, that imposing simplified categories on emergent social order distorts, the basis for treating any institution type as a surfaced approximation for the observer and never as ground truth, and the caution against a fixed unilineal typology of polities.
- Game and simulation precedent for institutions and factions: Dwarf Fortress (authored entity and position types with procedural instances and worldgen religions), Victoria 3 (an explicit authored institution list with investment levels, plus a fixed interest-group and government-type roster), Crusader Kings 3 (authored government types and faiths assembled from a fixed list of tenets), RimWorld (ideoligions assembled from authored memes and precepts), Songs of Syx (authored services, factions, and population strata), and Caves of Qud (a fixed persistent faction set plus procedurally generated village and cult factions drawn from authored slots). Uniformly cautionary: the category space is always authored and only instances emerge, the pattern this design departs from. Adopted and avoided as noted in Parts 36 and 62.8.

**Tier consistency: multiscale coupling, aggregation, and reproducibility (Parts 54, 58, 62.9)**
- Kevrekidis, I. G., Gear, C. W., Hyman, J. M., Kevrekidis, P. G., Runborg, O., and Theodoropoulos, C. (2003). Equation-free, coarse-grained multiscale computation. Communications in Mathematical Sciences, 1(4); with Kevrekidis, I. G., and Samaey, G. (2009), Equation-free multiscale computation, Annual Review of Physical Chemistry, 60. The lifting (coarse-to-fine) and restriction (fine-to-coarse) operators and their consistency conditions, the direct model for instantiate-from-summary and re-summarize.
- E, W., and Engquist, B. (2003). The heterogeneous multiscale methods. Communications in Mathematical Sciences, 1(1). The companion framework for coupling a fine and a coarse model, adopted for the operator framing.
- Leontief, W. (1947). Introduction to a theory of the internal structure of functional relationships. Econometrica, 15(4); with Theil, H. (1954), Linear Aggregation of Economic Relations, North-Holland. That exact consistent aggregation of a dynamical system requires linearity, the basis for rejecting identical-outcomes agreement between the tiers as mathematically unattainable.
- Structure-preserving model-order reduction (symplectic and energy-preserving reduced models, for example Peng and Mohseni 2016 on Hamiltonian systems, and conservation-preserving reduced methods). That a reduced model standing in for a full one preserves invariants rather than trajectories, the basis for conservation-as-agreement rather than outcome-matching.
- Salmon, J., Moraes, M., Dror, R., and Shaw, D. (2011). Parallel random numbers: as easy as 1, 2, 3. Proceedings of SC11, ACM. Counter-based stateless RNG whose result is a deterministic function of key and counter, the basis for order- and thread-independent reproducibility, the family the engine's SplitMix64 keying belongs to.
- Crawford, S., and Ostrom, E. (1995). A Grammar of Institutions. American Political Science Review, 89(3). The ADICO grammar, reused here as the substrate through which an aggregate collective decision is generated so the two tiers share one authority model (also cited for Part 36).
- Game and simulation precedent for tiered and off-screen simulation: Dwarf Fortress (abstract world-tier history continued off-screen and instantiated into detailed play, with documented cross-tier referential-integrity bugs that an audited invariant exists to prevent), Songs of Syx (large-population strata and aggregates rather than per-agent everywhere), RimWorld (a world-tier abstraction bridged to the loaded detailed map by caravans and events), and Paradox grand strategy (off-focus regions advanced as aggregates, calibrated for play rather than reproducibility). Uniformly cautionary: none guarantees both tier-consistency and seed-reproducibility. Adopted and avoided as noted in Parts 54 and 62.9.

**Recursive technology composition: generative encodings, modularity, and the cultural ratchet (Parts 41, 62.10)**
- Hornby, G. S., and Pollack, J. B. (2002). Creating high-level components with a generative representation for body-brain evolution. Artificial Life, 8(3). The generative representation and its abstraction property, labelling a compound element to manipulate it as a unit with parameters, the basis for the recursive composition node and the reuse of validated modules.
- Hornby, G. S. (2004). Functional scalability through generative representations: the evolution of table designs. Environment and Planning B, 31(4); with Hornby, G. S. (2003), Generative Representations for Evolutionary Design Automation, PhD thesis, Brandeis University, and the related AAAI 2003 Spring Symposium paper. That generative, building-block-reusing representations give higher fitness and more regular structure and scale with complexity, the empirical justification for composition over a flat representation.
- Hornby, G. S., Lipson, H., and Pollack, J. B. (2003). Generative representations for the automated design of modular physical robots. IEEE Transactions on Robotics and Automation, 19(4). Reusable subprocedures letting a design system scale to more complex tasks in fewer steps.
- Stanley, K. O. (2007). Compositional pattern producing networks: a novel abstraction of development. Genetic Programming and Evolvable Machines, 8(2); with Stanley, K. O., and Lehman, J. (2015), Why Greatness Cannot Be Planned, Springer. Compositional encodings that yield regular hierarchical structure, and the stepping-stone view of open-ended search.
- Clune, J., Mouret, J.-B., and Lipson, H. (2013). The evolutionary origins of modularity. Proceedings of the Royal Society B, 280; with Mengistu, H., Huizinga, J., Mouret, J.-B., and Clune, J. (2016), The evolutionary origins of hierarchy, PLoS Computational Biology, 12(6). That a connection-cost pressure drives modularity and the recursive composition of sub-modules into hierarchy, and improves evolvability.
- Ellis, K., Wong, C., Nye, M., Sablé-Meyer, M., Cary, L., Morales, L., Hewitt, L., Solar-Lezama, A., and Tenenbaum, J. B. (2021). DreamCoder: bootstrapping inductive program synthesis with wake-sleep library learning. PLDI 2021 (and Philosophical Transactions of the Royal Society A, 381, 2023). The description-length library-growth criterion, the basis for the compressive-reuse promotion gate that bounds the combinatorial explosion.
- Maynard Smith, J., and Szathmáry, E. (1995). The Major Transitions in Evolution. Oxford University Press. Encapsulation producing new higher-level units, the basis for promotion-as-stabilisation and the hiding of lower-level detail behind a module.
- Gould, S. J., and Vrba, E. S. (1982). Exaptation, a missing term in the science of form. Paleobiology, 8(1). Parts repurposed for roles they were not built for, the basis for exaptation falling out of id-reference plus interface matching.
- Simon, H. A. (1962). The architecture of complexity. Proceedings of the American Philosophical Society, 106(6). Hierarchy and near-decomposability, the watchmaker parable in which stable intermediate forms assemble faster and come to dominate, the basis for depth being selected by physics rather than authored.
- Parnas, D. L. (1972). On the criteria to be used in decomposing systems into modules. Communications of the ACM, 15(12). Information hiding, the basis for the interface that hides a module's internals from the level above.
- Baldwin, C. Y., and Clark, K. B. (2000). Design Rules, Vol. 1: The Power of Modularity. MIT Press; with Ulrich, K. T. (1995), The role of product architecture in the manufacturing firm, Research Policy, 24, and Suh, N. P. (1990, 2001), The Principles of Design and Axiomatic Design, Oxford University Press. Visible design rules versus hidden modules, interface specification, and the independence axiom, the basis for the port-vector interface contract and the typed combinators.
- Tomasello, M. (1999), The Cultural Origins of Human Cognition, Harvard University Press; with Tennie, C., Call, J., and Tomasello, M. (2009), Ratcheting up the ratchet, Philosophical Transactions of the Royal Society B, 364. The ratchet effect and the primacy of faithful transmission over invention, the basis for the stability gate (also cited for the transmission systems).
- Game and simulation precedent for authored technology and assembly graphs: Factorio (Wube Software) and kin such as Satisfactory and Dyson Sphere Program, whose tiered-intermediate recipe graphs and authored recipe ratios are the structural shape this design reaches emergently rather than authoring. Uniformly cautionary: the dependency graph is hand-written node by node. Adopted and avoided as noted in Parts 41 and 62.10.
