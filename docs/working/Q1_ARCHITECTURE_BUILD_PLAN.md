# Q1 architecture build plan: machine-enforced local buildability

Owner directive: stand this up next, once the in-flight Mirror arcs (the physical-hydrology arc and the threat/flee arc) land. Q1 is the machine-enforcement architecture that converts the derive-first discipline and cross-module consistency from conventions the builder must remember into rules the compiler and gates enforce, so a forgetful builder cannot violate them. This is the buildable form of the converged Opus and Fable-5 consult (see the north-star memory and this session's transcript for the full plan and enforcement classes).

This document scopes each of Fable's five moves against the ACTUAL repo (exists versus new), sequences them into discrete, independently-valuable, testable steps, and flags the one design fork that needs an owner ruling. Enforcement class is named for every rule: COMPILER (ill-formed code fails `cargo check`), GATE (a CI check blocks merge), LINT (blocked with a fix hint), or PACKET (an auto-generated artifact hands the constraint to the builder as input).

## Grounding: what the repo already gives us

- Workspace crates: `core`, `physics`, `units`, `compose`, `sim`, `world`, `viewer`, `gpu`. The determinism-critical tick-state crates are `core`, `physics`, `sim`, `world`.
- `core::Fixed` is the Q32.32 fixed-point canonical numeric type (`crates/core/src/fixed.rs`); `canonical.rs` holds the units-per-one grid.
- `units` (design Part 55) already has a data-defined base-dimension REGISTRY and a `Dimension` type (a runtime vector of integer exponents over the base dimensions, canonical sorted), plus a quantity registry carrying each quantity's dimension and per-quantity scale, and the Tier-2 wide-magnitude representation (i64, `MAG_BITS = 63`). Dimensions are a RUNTIME value, not a compile-time type parameter.
- `viewer` (the read/render side) depends on `sim` one-way; `sim` does NOT depend on `viewer`, and `viewer` holds no hashable simulation state. So the dynamics/read separation Fable's Q2 wants is already the crate shape.
- The `Runner::step`, `step_with_world_inputs(&[TickInput])`, and `step_scheduled(&[TickInput])` entry points already take world inputs, close to `step(&mut World, &Inputs)`.
- The billboard exists: `scripts/gen_floor_registry.py` scanning `// @derives:` markers (currently a handful, in `locomotion`, `celestial`, `decompose`, `physiology`, `environ`, `clock`) into `docs/working/PHYSICS_FLOOR_REGISTRY.md`.
- Gate infrastructure exists: `.github/workflows/ci.yml` and the hooks `customs-guard.sh`, `post-edit-check.sh`, `stop-gate.sh`, `substrate-first.sh`. New gates land as CI steps or hooks.

## The design fork that needs an owner ruling (before Move 2b)

Fable's highest-leverage move is dimensional and scale TYPES enforced by the compiler (`Fixed<Dim, Scale>`), so a unit or scale mismatch is a `cargo check` error. But our `units` crate deliberately keeps the base-dimension SET as DATA (owner-extensible, Principle 9, so an alien field is a data row). A fully type-level `Dim` (const-generic exponent vector) needs a FIXED base-dimension set at compile time, which conflicts with the data-defined set. The fork:

- Option A: FIX the base-dimension set as a closed compile-time const (the SI base dimensions plus a small reserved set for alien field-quantities), enabling full const-generic `Fixed<Dim, Scale>` and pure COMPILER enforcement. Cost: the base-dimension set stops being runtime data; adding a base dimension becomes a code change, not a data row.
- Option B: KEEP the runtime `Dimension` and enforce dimensional consistency at module boundaries by a GATE (every declared module input/output carries its `Dimension`, and a check proves each edge's dimensions match), plus a type-tag wrapper for the common quantities. Cost: consistency is a gate, not a compile error, so it is caught at merge rather than at `cargo check`, and it is weaker than Fable's ideal.

Note the SEALED CONSTRUCTOR (below, the derive-first enforcement, the actual fix for the forgetting problem) does NOT depend on this fork: it works with either option. So Stone 1 proceeds regardless; the fork only gates the full type-level dimension enforcement (Move 2b).

## Build sequence

The first three stones are each independently valuable, harden the CURRENT engine before any contract machinery, and are the recommended start order.

### Stone 1 (FIRST): the sealed-constructor `Fixed` (the machine derive-first). Move 2a.
- Enforcement: COMPILER plus a belt-and-suspenders GATE.
- What exists: `core::Fixed` (Q32.32); `units` holds the authored primitive and measured constants.
- What is new: make `Fixed`'s raw-integer constructor `pub(crate)` (or sealed behind a `units`-crate token), so a `Fixed` can be fabricated from a bare number ONLY inside the constants quarantine; everywhere else a value must come from a typed operation or a floor read. Add a GATE (extend `post-edit-check.sh` or a clippy lint) that greps the non-quarantine crates for raw-integer-to-`Fixed` construction and fails.
- Builder receives it: the packet and the compiler; a builder that types a bare number where a value belongs gets a compile error pointing at the sealed constructor.
- Unrepresentable: authoring a value outside the quarantine does not compile. This is the derivation-hunter as a type rule.
- Acceptance: the tree compiles; an attempt to construct a `Fixed` from a literal in `sim`/`physics`/`world` fails to compile (a red test proving it); all existing tests and the four pins hold (pure refactor, byte-neutral).

### Stone 2: dynamics/read observer-independence hardening. (Q2 foundation, buildable now.)
- Enforcement: COMPILER (crate boundary, already present) plus a GATE plus a property test.
- What exists: the one-way `sim` to `viewer` dependency; the `step(&mut, &Inputs)` shape; the determinism pins.
- What is new: (1) a GATE that greps the determinism crates (`core`/`physics`/`sim`/`world`) for `Instant::now`, `SystemTime`, thread-id reads, unordered-container iteration, and any read-crate import, and fails (the `customs-guard`/`post-edit` hooks are the seed); (2) the OBSERVATION-SCHEDULE INVARIANCE test: run the same seed and inputs twice under two observation/render schedules and assert the world-trajectory hash is bit-identical (extend the existing pin harness to a second schedule that observes/disaggregates). It passes trivially now, and standing it up before the LoD tower means the tower is built against a live guard.
- Acceptance: the grep gate is green on the current tree; the two-schedule invariance test passes bit-identical; an intentionally-added `Instant::now` in `sim` fails the gate (a red test).

### Stone 3: the conserved-quantity ledger. Move 3.
- Enforcement: COMPILER (move semantics) plus a per-step GATE.
- What exists: the matter cycle (mass conservation in `decompose`/biosphere), and Fable's apportionment resolver (task #21) as the exact-integer residue handler.
- What is new: `Conserved<Q>` (no `Clone`, no public arithmetic) plus the ledger API (`transfer`/`create`/`destroy` with source/sink tags); a per-step sum-to-zero harness as a GATE. Wrap the single most-touched conserved quantity first (matter/mass, the matter cycle).
- Builder receives it: the packet lists which conserved quantities the module touches and the ledger tags it may use; there is no by-hand conservation path.
- Unrepresentable: creating or dropping a conserved quantity off-ledger is a borrow-check error; an unbalanced create is caught by the sum-to-zero gate. Task #21's apportionment carries the indivisible residue.
- Acceptance: matter moves only through the ledger; the sum-to-zero gate is green each step; a hand-written duplication of a `Conserved` value fails to compile (a red test); byte-neutral.

### Then the contract layer (Moves 1, 4, 5), building on the stones.
- Move 1 (contracts): define a `Module` trait with typed `In`/`Out` structs and a `CONTRACT` const; convert ONE leaf physics law as the reference; a GATE scans every `CONTRACT` (every module has one, no duplicate outputs, no dangling inputs) advisory-first; convert modules leaf-first up the derive index; flip the gate to blocking on the last. The `@derives` markers are the seed; contracts are the typed superset. Enforcement: COMPILER (typed In/Out make an undeclared read a missing-field error) plus GATE (the scanner).
- Move 4 (DAG packet plus fan-in budget): evolve `gen_floor_registry.py` into the packet generator that, from the contract registry, emits each module's one-hop upstream contracts, downstream assumption suites, ledger tags, typed aliases, harvested golden fixtures, and the acceptance checklist, ending in a completeness line. Add the fan-in/fan-out budget GATE (advisory, publish the current distribution, set the cap just above the worst offender, ratchet down over merges) and the acyclicity-except-declared-loops GATE. Enforcement: PACKET plus GATE.
- Move 5 (tests): the own-contract property-test harness (run over the harvested fixtures); the consumer-assumption suites plus the producer-change re-run that emits the blast-radius queue; the two metamorphic global tests (conserved-scaling, and layout-permutation-hash-invariance, the latter close to the existing determinism pins). Enforcement: GATE.

### Cross-move stand-up order (smallest-first, each independently valuable)
Stone 1 (sealed `Fixed`), then Stone 2 (observer-independence gate plus invariance test), then Stone 3 (ledger on matter), then Move 1 (Module trait plus one reference module plus the advisory scanner), then Move 4 step 1 (emit packets from the existing index, advisory), then Move 5 step 1 (own-contract tests as modules convert), then flip the scanner and fan-in gates to blocking, then the assumption suites and the phase types (Move 2c) and the metamorphic tests. Each step compiles the tree and keeps the gates green, so the coordinator hands any single step to an implementation agent and verifies it in isolation.

## Q2 note
Q2 (the LoD tower) rides on this foundation: the ledger makes coarsen/reconstruct conserve exactly, the contract registry supplies the sufficient-statistics union, the observation-schedule invariance test is the observer-independence guard, and the apportionment resolver is the reconstruction primitive. Q2 is a separate arc after Q1's stones; the observer-independence hardening (Stone 2) is the piece of Q2 worth landing inside Q1 because it hardens the current engine immediately.

## Owner decisions in this plan
1. The base-dimension fork (Option A fixed compile-time set for full type-level `Fixed<Dim,Scale>`, versus Option B runtime `Dimension` plus a boundary gate). Stone 1 does not wait on this.
2. Confirmation that the first three stones are the right start order (they are pure additions or refactors, each byte-neutral and independently valuable).

---

## Fable-5 critique adopted: the fork resolved and the final ship order (supersedes the sequence above)

Round-3 Fable-5 consult (verified, repo-blind) resolved the fork and corrected the ordering. This section is authoritative where it differs from the sequence above.

### The base-dimension fork: RESOLVED, decouple Dim from Scale.
The fork conflated two quantities with opposite extensibility profiles. The SCALE (Q-format) is chosen per-law at LOAD time by the existing scale planner, so a const-generic `Scale` would fight the planner: keep Scale RUNTIME, checked at module boundaries (Option B). The base-dimension SET is a floor concern, small and known before a run, so it takes Option (i), BUILD-TIME CODEGEN: the base-dimension manifest stays authored as data (one row per dimension, stable ID), and a deterministic build step (sorted by stable ID so exponent indices never shift, never hand-edited) generates the const-generic dimension-type width, aliases, and algebra. The set is therefore data at the manifest AND compile-time types at the enforcement layer; "runtime data versus compile-time fixed" is a false dichotomy here. The recompile-to-add-a-base-dimension cost is correctly placed: a base dimension is a floor axis, the one layer where a recompile-gated change is legitimate, while materials, agents, and cultural content never need a recompile. A world declares its base dimensions before the seed runs, so the binary is built with the manifest union; runtime base-dimension addition is not a real requirement. Option A (fixed closed set) is rejected (it makes an alien field-quantity a code change, an admit-the-alien hit); Option (ii) (fixed core plus escape hatch) is rejected as primary (the escape hatch is exactly where the alien lives, so the most novel quantities would get the weakest enforcement, backwards) and kept only as a fallback if codegen is judged too heavy, gated by a hard lint on the hatch.

### The final ship order.
- **(0) Static determinism grep gate.** No `Instant::now`/`SystemTime`, thread-id reads, or unordered-container iteration in the determinism crates (`core`/`physics`/`sim`/`world`). Near-zero cost, TOTAL static coverage, and it protects the reproducibility baseline every later byte-neutral claim rests on (the existing dynamic pins only catch nondeterminism on the path a fixture exercises). First, or parallel with (1). Verify `core::rng` is already counter-based and order-free here.
- **(1) The seal-the-constructor MINI-ARC** (not one atomic step): add the sealed `Fixed` constructor plus the constants quarantine; add a LINT flagging raw construction outside quarantine (advisory); inventory the violation set; migrate site by site with each byte-diff reviewed (legitimate sites move to a derive path byte-neutral, actual defects change bytes correctly); FLIP the seal to hard once the count hits zero. Only the flip is byte-neutral by construction. Fork-independent; mechanizes the owner's top-priority derive-first rule.
- **(1b) The DAG-packet generator in READ-ONLY advisory mode** against the current thin `@derives` index. This is the most important resequencing: the packet must be the builder's mandatory entry point from day one, so the later contract conversion is done FROM packets, not by consolidating the old way. It fills out as contracts arrive.
- **(2) The observation-schedule invariance test while it is still green** (before any LoD), so a future red is attributable to the change under review, not a pre-existing latent dependency.
- **(3) The conserved-quantity ledger** on the most-touched conserved quantity (matter), with the sum-to-zero gate; this also seeds the contract work by revealing that quantity's producers and consumers.
- **Then the contract layer**: `Module` trait with typed `In`/`Out` (dimension via the codegen types), the fan-in budget and acyclicity gates, and the cross-module tests, leaf-first and advisory-then-blocking, with the golden-fixture harvester landing beside the first cross-module property tests (tests over imagined domains miss the narrow fixed-point failure bands). Phase types trail the dimension types (lower urgency, real hazard).

### Relabel: byte-neutral only if the invariant already holds.
Stones (0), (2), and (3) may LIGHT UP a latent bug on landing (an unordered-iteration nondeterminism, a conservation leak). Fixing it is not byte-neutral, and that is a SUCCESS: the stone found a real determinism or conservation bug, and its fix is a deliberate reviewed byte change. Budget for it; a byte change here is the stone working, not failing.
