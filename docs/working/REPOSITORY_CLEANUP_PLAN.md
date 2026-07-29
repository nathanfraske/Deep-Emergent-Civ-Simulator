# Repository cleanup plan

Status: execution plan, updated 2026-07-20 on PR #215 head `9c9444f0` and
current uncommitted worktree.

This plan turns the current handoffs, `UNTANGLE_PLAN.md`, the gate audit, and
the module-ownership audit into one dependency order. It is a pointer, not a
replacement for those evidence records. The core rule is simple: establish
truthful receipts and ownership boundaries before moving formulas or splitting
large files.

Owner ruling, 2026-07-20: the abiotic planetary pipeline is the canonical main
runpath. The biology and civilization arc was built under the prior methodology
and is legacy work. It may remain compile-maintained during extraction, but its
fixtures, pins, and behavioral tests do not define planetary readiness and must
not block the canonical run.

## Diagnosis

The three inherited structural failures were:

1. The convenient run command entered a legacy dawn and biology fixture whose
   two pins reached none of the abiotic physics under review.
2. The viewer owned causal planet construction and evolution that belonged in a
   library.
3. CI, Stone 0, the Stop hook, and local scripts carried different gate lists,
   so a green surface did not have one stable meaning.

The workspace, physics boundary, viewer ownership, and final numerical-provider Diamond are now repaired.
The canonical front door constructs the sealed eleven-entry floor, enters Stage 1, and returns the named
`stellar_birth.realization_measure` refusal because the audited floor lacks the
opaque joint physical measure and distinct realization-coordinate law.
`--readiness` remains a separate zero-entry boundary receipt. One declarative
inventory now defines 25 gates across seven tiers for Stone 0, CI, Stop, Just,
Make, PowerShell, local CI, and tracked pre-push clients.

The live `TODOS.md` is now a bounded canonical abiotic queue, with the displaced
queue preserved at `parked/TODOS_LEGACY.md`. `HANDOFFS.md` remains an append-only
history with a current owner-ruling header. The consensus roadmap is now a lean
planetary status board with ample space below its 16 KiB gate.

## Target ownership

```text
sealed floor + ledger + units
              |
      canonical planet stages
              +--> receipts and immutable snapshots
              `--> viewer, as an immutable-observation leaf

active candidate substrate
  core + physics + materials + physical world operators
  private pre-migration planet modules
              |
        typed stage adapters only

parked legacy compatibility, outside readiness

No legacy consumer sits on the canonical planetary runpath.
```

- `physics` owns stateless laws, numerical kernels, solvers, and unadmitted
  evidence candidates. Its active ground catalog contains no organism,
  language, tool-demo, or display-instance rows.
- `materials` owns candidate-backed material and phase properties. A citation
  does not make a candidate canonical.
- `world` owns physical spatial operators and render-neutral fields. Authored
  biome generation, map seeds, quadtree presentation, and cameras are parked.
- The `planet` crate owns abiotic construction, evolution, preflight,
  refusals, receipts, and snapshots. It must not depend on `sim`, `bio`, or
  `viewer`.
- The canonical binary is a thin front door over `planet`; its only
  value-bearing input is the validated absolute physics floor. Ledger tags are
  accounting, not permission for caller-authored world values. It never falls
  back to development data.
- The whole contract obeys admit-the-alien: no Earth, Mirror, Sun, fixed orbit,
  familiar composition, or hindcast target is an implicit world. Reference data
  may bound a mechanism; it cannot choose a realization.
- `sim`, `bio`, and dawn fixtures are parked legacy work outside the main
  runpath and readiness claim. Any future compatibility consumer must accept a
  supplied immutable snapshot through an audited one-way edge.
- `viewer` renders planet snapshots and receipts. It owns no causal integration
  interval, hidden model constant, biology bootstrap, or planet evolution.

Legacy compatibility is isolated in explicitly named packages under `parked/`.
Canonical crates do not re-export those surfaces.

Core retains only generic deterministic mechanics by default. Retired biology
and civilization phase identifiers compile only under the `legacy-domains`
feature enabled by the parked workspace.

## Stage 0: make the control plane trustworthy

1. DONE IN WORKTREE: cross-platform setup, including the LF policy, exact-byte research exemptions,
   WSL developer bridge, Just and Make entrypoints, Codex instructions, Codex
   hooks, tracked pre-push entrypoint, and project MCP launchers. This clone has
   `core.hooksPath = scripts/githooks`.
2. USER ACTION: start a fresh Codex thread rooted at this repository, trust the
   project if prompted, and approve the eight exact command-hook hashes in
   `/hooks`. Hook trust is per user and cannot be committed or proven by this
   thread, which started above the repository.
3. DONE: prune stale roadmap tombstones and keep the board below 16 KB with useful
   headroom.
4. DONE: rebuild `TODOS.md` as a lean parseable canonical abiotic queue and
   preserve the displaced queue at `parked/TODOS_LEGACY.md`.
5. SAFE, REPAIR OPEN: both shipped panel templates are disabled. Neither may run
   until it has a strongest-model Section 11 input-bias smoke stage that fails
   closed. The lens template must also construct a diverse panel.
6. DONE: make `projectops consolidation_check` require the Part 62 record pointer
   and Part 63 bibliography evidence for the parked historical archive. The live
   `floor_admission` tool exposes accounting and refusal policy without an
   editable magnitude.
7. DONE: `Fixed::log_sum_exp` is the sole N-ary provider. Materials creep and
   Saha reductions consume it; the public binary Saha function is a delegating
   compatibility wrapper. The strict Diamond scan has no exception row and
   reports 110 providers for 110 quantities.

Acceptance receipt: SessionStart reports one canonical queue, the roadmap has
headroom, all hook allow and deny probes pass, the `projectops` MCP server
initializes, and the Stop hook runs the strict Diamond scan without an amnesty
or known-open entry.

## Stage 1: one gate authority

Status: complete in the current worktree.

`scripts/gates.toml` and `scripts/gate_runner.py` are the authority. Each entry
carries a stable id, an argument array, tiers, path triggers, self-test
arguments, timeout, artifact policy, debt status, and cache inputs. The runner
never uses `eval`, rejects unsafe paths and links, hashes its manifest, its own
bytes, file modes, inventory, and contents, and rehashes around execution to
detect races.

These are thin clients of the runner:

- `.github/workflows/ci.yml`
- `crates/stone0`
- `.claude/hooks/stop-gate.sh`
- `scripts/ci_local.sh`
- `justfile`, `Makefile`, and `scripts/dev.ps1`
- the tracked pre-push hook

Stone 0 verdicts remain uncached. Cargo build-script freshness remains the local
compilation cache. The tracked pre-push hook runs the declarative `pr` tier
before its separate pushed-history tombstone scan and preserves Git's argument
and ref-update streams.

The supported interface is:

```text
dev doctor
dev run
dev run-derived
dev readiness
dev check pr
dev check full
dev check nightly
dev check legacy
```

Acceptance receipt: every client prints the same ordered gate ids for a tier;
deleting or changing one registered gate causes CI and local parity tests to
fail. No unqualified `run`, `view`, `test`, `lint`, or `check` command enters or
is blocked by biology. The planetary target now exists and may return a
structured physical refusal; legacy commands remain explicitly named and
separate.

## Stage 2: honest tiers and fixture receipts

The `pr`, `full`, and `nightly` names exist but currently run the same canonical
quality commands and effectively the same structural membership. That is an
explicit coverage debt, not three different readiness claims. There is no
`quick` tier.

- `pr`: current canonical formatting, structural and provenance gates,
  self-tests, Stone 0, Clippy, tests, rustdoc, and doctests.
- `full`: must gain direct nonignored planetary CPU goldens beyond `pr`.
- `nightly`: must gain expensive canonical capstones, broad perturbation and
  support checks, GPU hardware where present, and mutation work. It cannot use
  an authored seed ensemble to select physical state.
- `legacy`: compile maintenance and selected regression checks for the old
  biology, civilization, and dawn-fixture arc. This tier is reported separately
  and never supplies a planetary readiness receipt.

If dawn hashes are retained, move them into a machine-readable legacy fixture
manifest. Build once, extract one structured result per fixture, reject missing
or duplicate results, and add a deliberate-mismatch self-test. Do not treat
them as required evidence for planet work. Until direct goldens exist, add a
required direct canonical evidence. Two moment-equivalence tests remain ignored
pending lid rebaseline; they are named debt and cannot be counted as passing
planet readiness.

Acceptance receipt: required physics paths cannot receive a green PR without
either direct goldens or the temporary capstone job. Any invoked legacy fixture
check still rejects wrong, missing, duplicate, malformed, or nonzero results,
but it is labeled as legacy evidence.

## Stage 3: evidence truth before ownership moves

Implement `UNTANGLE_PLAN.md` Slice 2 without changing model semantics:

1. direct property-kernel golden;
2. deep-time province golden;
3. moment-equivalence golden;
4. flexure-field golden;
5. a stage-reachability vector in each structured receipt;
6. explicit refusal outcomes, conservation residuals, perturbation checks, and
   provenance hops.

Acceptance receipt: each subsystem can be reached and judged without the
viewer, and a refusal is distinct from a successful result.

## Stage 4: establish the planet boundary

Status: boundary and ownership move complete; live stage wiring remains open.

`civsim-planet` now has no caller identity or seed, a validated
absolute-floor input, fixed stage contract, named refusal reasons,
deterministic receipt, and immutable snapshot. The canonical path has no
calibration profile, world-value manifest, or caller-authored contingency
vector. Do not add an implicit default world.

The executable takes no world, identity, profile, seed, or magnitude argument.
With no arguments it constructs the repository-owned sealed floor and calls
the library runner. `just run-derived` is a compatibility name for this same
front door, not the former causal viewer. Every pre-migration planet module is
crate-private, and the boundary gate rejects a public re-export or canonical
import until a typed stage adapter exists. Canonical files also cannot import
raw physics, materials, or world substrates directly.

The fourteen retained modules now live in `civsim-planet-substrate` with their
raw surface private and 372 tests passing. The parked simulation compiles the
same mechanisms through its historical compatibility surface. This does not
make the mechanisms admitted or reachable from the canonical runner.

The floor now has crate-private typed magnitudes for all eight `[M]`
fundamentals and three `[D]` composites. The units tables are candidate
declarations, not admission authority. A separate sealed registry exact-matches
their full fingerprints before constructing the catalog. Every magnitude is
symbol-bound and dimension-typed; every composite proves its formula dimension
and exact ancestry, then evaluates only from the projected input bits published
in the transcript. An unaudited floor, extra candidate, relabeled magnitude, or
caller-paired value cannot construct the view. The full receipt and remaining
debt are recorded in `docs/working/PR215_LIVE_SOURCE_AUDIT.md`.

Ownership moved in this order:

1. `astro`, `stellar`, `stellar_evolution`, `planetary_system`, and `giants`;
2. `planet`, `moons`, `secular`, and `geodynamics_surface`;
3. `smallbody`, then `planetary_assembly`;
4. the shared convection contract, then planet geodynamics;
5. `deeptime`, then `flexural_field`.

Move the Stefan-Boltzmann derivation from `sim::physiology` to `units` first,
with an exact compatibility test. Create a separate dawn-fixture package and a
forbidden-edge gate so canonical crates cannot import it.

All former `crates/sim/examples` and `crates/sim/tests` are parked legacy work.
The substrate package owns the retained abiotic modules and inline tests; none
of the old examples or integration fixtures is a canonical orchestration
receipt.
Physical atmosphere, hydrology, and field mechanisms still need extraction from
mixed legacy modules without bringing productivity, food-web, or organism state
back into canonical scope.

The planet crate has no legacy `foundation` dependency. `GeodynamicColumn` and
`GeodynamicField` now live in `world`; `EarthworkField` remains parked because it
is not an abiotic planet input. Physical values enter only through the validated
absolute-floor interface.

Acceptance receipt: dependency direction matches the target diagram, direct
goldens remain green, and the planet front door either completes every required
stage or returns a named refusal. Legacy fixture drift is reported separately
and does not redefine the planet result.

## Stage 5: remove causal physics from the viewer

Status: ownership separation complete; snapshot transport and display adapters
remain open.

Build a ledger for each viewer-held value: display-only, derived, fixture
input, or misplaced model input. Reach zero unclassified causal values.
`DEEP_TIME_MYR_PER_TICK` is a physical integration interval and cannot be
treated as display cadence.

Preserve the existing causal viewer as historical evidence under `parked/`.
Compatibility edits may make new absence and refusal contracts explicit, but
may not supply a replacement physical value. The root viewer
is a new immutable-observation leaf. It may inspect a typed refusal now and
must refuse rendering until an immutable snapshot input is wired. Rebuild any
useful observer transformations from snapshot fields; do not
promote the old viewer's fixture constants or integration loop into `planet`.

Acceptance receipt: the viewer depends on immutable snapshots and receipts,
contains no causal integration interval, physical state mutation, generated
world input, or route by which observing changes the simulation outcome.

## Stage 6: decompose grab bags by invariant

Split tests first, then production files. Preserve public facades during each
move and keep one responsibility per child module.

Prioritize files that feed the planet path. Defer decomposition of biology,
cognition, embodiment, and civilization grab bags unless a small compatibility
facade is needed to free planetary code.

The world and GPU containment slice is complete. `WorldStructure`, authored
biome and seed-driven world generation, quadtree and camera presentation, GPU
world generation, and globe presentation now live only in parked compatibility
packages. The active world crate retains generic relief and physical spatial
operators; the active GPU crate retains fixed-point Stage 0, fields, and
transcendental kernels.

| Hotspot | First boundaries |
|---|---|
| `planet-substrate/src/astro.rs` | irradiation/orbit, disk, stellar structure/activity, formation, photoevaporation |
| `physics/src/moment_equivalence.rs` | load geometry, yield envelope, integration, fixed-point solver, rigidity/lid referee |
| parked mixed `sim/src/environ.rs` | extract abiotic fields, insolation, atmosphere, and hydrology; leave productivity and organism state parked |
| `planet-substrate/src/geodynamics.rs` | columns/assemblages, convection, secular/interior, surface generation |
| `planet-substrate/src/deeptime.rs` | ledger, impacts, stellar aging, provinces, relief |
| parked `viewer/src/render.rs` | recover display-only optics, globe backends, system map, and picking through snapshot adapters |
| parked `sim/src/runner.rs` | defer; split only where it blocks planet extraction |
| parked `sim/src/world.rs` | defer; split only where it blocks planet extraction |

Do not split `physics/src/laws.rs` until every gate that hardcodes that path has
been generalized to a declared law-module tree with failing self-tests.

Acceptance receipt per split: API parity, gate-coverage parity, unchanged
receipts, and focused tests for the moved invariant group.

## Source and research debt lane

Keep this lane independent from module moves:

- work the candidate-by-candidate obligations in
  `docs/working/ABIOTIC_EVIDENCE_DEBT.md` without promoting custody to
  admission;

- clear the three uncovered collections: `aesopus_lowt`,
  `optical_constants_aesopus`, and `oxide_thermochemistry`;
- remove the two stale source waivers;
- resolve JANAF redistribution, the MIT OCW licence conflict, and both external
  custody transitions;
- alias or explain seven duplicate documents by checksum;
- expand claim-to-source hops beyond the current narrow conductivity use;
- rebuild the IAPWS ice, held-out Slater, and independent Rayleigh validations.

All fetches follow `VENDORING_CHECKLIST.md`; relayed values remain fetch-spec
seeds until held-byte or citation-plus-witness receipts close evidence custody.
Evidence custody does not admit a canonical magnitude.

## Landing sequence

1. LANDED IN WORKTREE: control-plane repair, parked legacy boundary,
   `civsim-planet`, dependency gate, generated ledger inventory, typed sealed
   magnitudes, immutable-observation viewer, sole log-sum-exp provider, and named Stage
   1 refusal. Missing fusion volume and the incomplete H&K pressure range now
   produce structured Gap Law refusals. Active physics, world, GPU, and core
   legacy surfaces are separated from their parked compatibility facades.
2. LANDED IN WORKTREE: extract the fourteen private raw planet modules into
   `civsim-planet-substrate`, preserve 372 tests, and harden package and gate
   dependency boundaries without changing formulas.
3. LANDED IN WORKTREE: separate admission from declaration, bind source and
   uncertainty receipts to all measured floor entries, make composites replay
   from their published ancestry bits, and enforce typed SI dimensions and
   exact-symbol ancestry.
4. CURRENT PHYSICAL SLICE: build the machine-readable derive-first and
   Buckingham-Pi census, then close the opaque joint stellar-birth measure and
   separate realization-coordinate law under
   `docs/working/STELLAR_BIRTH_MEASURE_CONTRACT.md`.
5. THEN: add SI-native collapse and centrifugal-radius adapters, followed by
   the system and assembly chain behind direct physical goldens and
   stage-reachability receipts.

## Owner decisions

Resolved: the abiotic planetary pipeline is the main runpath. Biology,
civilization, and dawn fixtures are legacy and non-blocking.

- whether `N_A` and `A3_per_cm3_mol` remain physical Universal entries or move
  to an explicitly nonphysical metrology and unit bridge;
- canonical world identity and termination rule;
- physical integration interval versus viewer cadence;
- finite-disc load treatment and `LoadKind`;
- melt-mass basis and ledger source/sink tags;
- whether stagnant-lid paths gain production callers or are retired;
- model-version policy for formula replacement;
- capstone refusal policy;
- whether `HANDOFFS.md` may be archived or sharded.
