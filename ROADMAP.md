# ROADMAP.md: The Path to a Living, Talking World

This roadmap covers one target milestone: the first end-to-end emergent slice. A generated visual map the owner can look at; seeded bands of sentient people placed on it at the dawn of sentience, each with intrinsic beliefs, race parameters, and a genome; those bands playing the naming game over the full set of about sixty-five Natural Semantic Metalanguage primes to coordinate a shared starter lexicon; and from there the people acting emergently through the resolved modelled dialogue (R-CONVERSE), the belief and evidence engine, theory of mind, and the utility-AI decision layer, with conversations as first-class events.

It was synthesised from a six-lens read of the design document and the codebase. It is a working plan, not a contract; the owner sets the order and the priorities, and every reserved value below is surfaced, never invented.

---

## Where we are

The simulation's reasoning core is built and deterministic; its world and its body are not. Two of the four milestone components already have a working core, and two start from nothing.

Built and running (on an abstract place model): the deterministic core (`Fixed` Q32.32, `Rng::for_coords`, `StableId`/`Registry`, `StateHasher`, `EventLog`, the `Canonical`/`NonCanonical` boundary), the serial `World::tick` spine, belief facets, the evidence and inference engine, recursive theory of mind with the anti-projection wall, the utility-AI decision layer, the naming game with drift and lineages, and the calibration manifest with its profiles.

Absent: there is no worldgen, terrain, biome, spatial, or render code. A place is a bare `PlaceId = u32` with no geometry. A `Mind` carries only an id and an acuity, with no race, genome, or axioms attached. `ConceptId` is an opaque `u32`, and the naming game has only ever run over a single concept. R-CONVERSE is resolved in the design but not implemented in code (only the one-pass gossip fallback exists), and the `EventLog` is wired but empty, since nothing emits into it.

The headline: this is overwhelmingly engineering, not research. Worldgen carries no research item at all (it is unbuilt, not unresolved). The being model, the prime inventory, and the dialogue layer are resolved designs awaiting code. There is one genuine open research blocker on the critical path, R-RNG-COORD, plus a handful of owner calibration numbers to set. Everything else in the determinism cluster is deferrable while the slice stays serial, single-stream, promoted-tier, CPU, and regenerated from seed.

---

## The critical path

Determinism keying contract, then a generated CPU map, then seeded bands at the dawn, then the naming game over the primes, then the R-CONVERSE move-event layer, then the emergent loop.

---

## Milestones

### M0: Lock the determinism keying contract

The one genuine research item on the path.

- Resolve R-RNG-COORD: put the canonical tick into every draw coordinate, and replace the hand-assigned `PHASE_*` tags in `world.rs` with a phase and draw-site registry, so two draw sites cannot collide on counter zero. Pin R-REDUCE-ORDER for the two combines the slice uses (the gossip-conflict apply and the weighted pick).
- Why first: the serial tick already replays bit for bit today, so this is not what stops the work from starting. But every later milestone multiplies draw sites (per-cell worldgen noise, per-locus genome sampling, sixty-five-prime coordination, the four dialogue draws), and locking the key now is far cheaper than renamespacing later. Treat any replay seeds produced before this lands as throwaway.
- Exit: the same seed yields a bit-identical world and history across reruns and thread counts, on the real tick phases (a slice-scoped extension of the determinism harness).

### M1: A generated visual map (CPU, headless)

The long pole and the largest unscoped engineering, with no research dependency.

- Implement: a topology module (`Coord3`, a `TopologySpace` trait with a `FlatBounded` implementation, fixed-point distance so no square root is needed); a data-driven terrain and biome substrate (named records with field thresholds, never a closed enum); worldgen pass one in fixed point (layered integer noise through `for_coords` to an elevation field, then rainfall, temperature, and drainage, then biome classification); a light or stubbed hydrology pass; the chunk grid and quadtree with bottom-up summaries; and a headless glyph frame (a `Camera`, `build_glyph_frame`, a small data-driven glyph table, emitting ANSI text or a PPM or PNG image). No GPU.
- Defer: R-GPU-CANON-PIN, R-WEATHER, R-CATASTROPHE, R-SAVE-SCHEMA (the map regenerates from seed), R-UNITS-PIN (internal Fixed units for now).
- Exit: a command generates a viewable map from a seed, bit-identical across runs.

### M2: Seed bands at the dawn of sentience

The heaviest data-model work; the genome chain alone is three extra-large pieces. The data model can proceed in parallel with M1 against a placeholder place id; only the final placement step needs the map.

- Implement: the being registries and a `Race` definition loaded from data (drives, traits, value axes, genes, axiom axes, source modes); the `Genome` and the Part 25 genotype-to-phenotype map in fixed point with per-locus counter-RNG; the allele-frequency `Pool` with Hardy-Weinberg promotion; intrinsic-belief seeding (value profile, axioms, epistemic stance) pushed into priors through the existing `seed_prior` hook; the one fixed axiom and value update kernel; a `spawn_being` assembly; and `seed_dawn_populations` placing bands onto habitable map cells, with `PlaceId` bridged to a `Coord3`.
- Calibration: add clearly-labelled development fixtures for the axiom, genome, and value-metric values so the step runs under the Development profile, with the fail-loud gate refusing them under Calibrated until the owner sets them.
- Defer: R-BUILD-PHYS (primitive build stats now). Register the genome and axiom conserved projections as the pool is built, ahead of R-PROJ-REGISTER.
- Exit: a seed places race-typed, genome-bearing, axiom-seeded bands on the map; replay is bit-identical; the existing belief, theory-of-mind, and decision layers run spatially with no change.

### M3: The naming game over the primes

- Implement: load the roughly sixty-five Natural Semantic Metalanguage primes as data (stable ids plus the one sanctioned hardcoding, the English gloss lemma); a thin `ConceptId` and lemma representation (the full substrate-region grounding is a later increment); convert `converse_language` from its current unbounded per-tick loop into the bounded once-per-culture dawn dynamic with a round cap and id-ordered aggregation; and give each band its own form system so bands diverge.
- Calibration: `lang.dawn_round_cap` (reserved; the owner sets it, or a labelled development fixture runs the demo).
- Defer: R-LANG-TYPOLOGY (grammar is not exercised by coordinating words).
- Exit: each band converges a shared starter lexicon over the full prime set, bands diverge into cognate families, and the run replays bit for bit.

### M4: Emergent dialogue (implement R-CONVERSE) and the loop

The design is resolved (Part 9.5, record 62.14); this is the build.

- Implement, in order: the `MoveKindDef` and `ForceEffect` registries (the etic floor as typed dispatch into existing mechanisms, no new behaviour); the `FelicityCond` gate and the content gate (affordance or outcome at load, refusing any graded persuasion or fidelity weight); the move-as-event encoding (which fills the empty `EventLog`); the conversation query; the speak-as-intent `ActionDef`; the per-move response loop reusing the Part 37 single-utterance update; grounding as said-evidence, with its two-colluder regression test written first; repair within the 33.7 error budget; the level-of-detail significance gate; the four determinism pins on interim keying; the reserved values as fail-loud manifest constants; and the two Steering Audit invariants as harness checks. Also start emitting events from perception, decision, and gossip, and broaden decision considerations beyond drive levels.
- Defer: the deeper couplings (deliberation to Part 36, persuasion to Parts 21 and 28, negotiation to Parts 37, 24, and 8).
- Exit (the milestone itself): seed bands on the map, run the prime naming game to a shared lexicon, then run promoted bands through move-by-move conversations as first-class events, with the false-belief and seen-through-lie battery passing through a real dialogue exchange and bit-identical replay.

---

## Research to resolve, calibration to set, and what waits

Genuine research on the critical path: only R-RNG-COORD, plus pinning R-REDUCE-ORDER for the two combines the slice touches.

Owner calibration to set, surfaced not invented: `lang.dawn_round_cap`; the dawn axiom, genome, and value-metric fixtures; and the dozen-plus R-CONVERSE values. Development fixtures let the slice run; the Calibrated gate fails loud until the authoritative numbers are set.

Safely deferred: R-GPU-CANON-PIN, R-SAVE-SCHEMA, R-WEATHER, R-CATASTROPHE, R-UNITS-PIN, R-CMD-ORDER, the full forms of R-CANON-WALK, R-PROJ-REGISTER, and R-HARNESS-COVER, R-LANG-TYPOLOGY, R-BUILD-PHYS, and the three deferred dialogue couplings.

---

## Biggest risks

- The map is entirely unbuilt and gates the whole milestone; it is the single largest piece of engineering, though it needs no research dive.
- The genome chain carries the strictest determinism bar (per-locus counter-RNG, a fixed-point Gaussian segregation draw with no float log or square root); one float slip forks the timeline.
- The grounding-as-said-evidence wall is the highest-risk correctness item: a single accidental write of a common-ground prior to a nested store reopens the two-colluder corruption, so it is gated by its regression test before any other dialogue feature is trusted.
- Stay serial. The moment any phase is parallelised for performance, R-CMD-ORDER, R-REDUCE-ORDER, and the full R-CANON-WALK become live blockers at once.
- Tier consistency: the axiom kernel must run identically on a promoted individual and on a pool representative.

---

## Sequencing recommendation

Run M0 and M1 in parallel (lock the keying contract while building the map, the long pole), and begin the M2 being-data-model work against the placeholder place id at the same time, since only its final placement step waits on the map. M3 is small once bands exist. M4 is the second-largest build after the map but reuses the resolved reasoning core almost entirely.
