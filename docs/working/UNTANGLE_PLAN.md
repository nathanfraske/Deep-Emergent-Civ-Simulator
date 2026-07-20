# Untangle plan: ownership, canonical execution, and meaningful pins

Status: planning document. This document proposes sequenced work; it does not declare any proposed mechanism implemented.

Evidence date: 2026-07-20, against the inspected repository snapshot. Measurements involving `calibration/reserved.toml` are a point-in-time count and will change as owner calibration lands.

## Decision

The owner's three problems are one architectural fault with three visible effects. The missing boundary is a canonical, library-owned abiotic world pipeline with an observable execution contract.

```text
No canonical abiotic owner or runner
    -> planet evolution is assembled in the viewer or left as disconnected kernels
    -> the dawn/biology fixture remains the only convenient watchable run
    -> its hashes are described as evidence for changes it cannot reach
    -> large files accrete orchestration, mechanisms, tests, and policy under historical homes
```

File size is therefore a symptom, not the root cause. Splitting files before assigning ownership would make the tree tidier while preserving the false runner and could move viewer-authored values into a canonical-looking module. The sequence must first make the evidence truthful, then establish an abiotic ownership boundary, then extract and split mechanisms inside that boundary.

The answer to the pin question is layered:

1. Keep the two current hashes as dawn-fixture identity checks, rename them, and make the comparison real.
2. Add reachability receipts to say what a run exercised. A receipt explains a pin's scope; it does not validate code the run never reached.
3. Add direct, library-owned golden-state harnesses for each physics subsystem being refactored. These are the smallest pins that can mean something for physics work now.
4. Add a completed canonical-planet receipt and digest once a `Profile::Calibrated` physics run can finish from owner-approved inputs.

This is pins plus reachability plus per-subsystem goldens. More final hashes from `run_world` would repeat the present mistake. Replacing hashes with reachability receipts would detect wiring while losing output regression. A canonical end-to-end digest alone would be too coarse for locating a broken invariant and cannot exist as a completed-state pin while the selected world's required calibration closure refuses.

Principle 3 remains absolute throughout: the same model version, input, and seed must replay bit for bit. Seed ensembles add a population-level physical comparison for chaotic domains; they never replace exact replay for each member.

## Verified baseline and corrections

### File measurements

All requested line counts match the current files exactly:

| File | Lines |
| --- | ---: |
| `crates/sim/src/runner.rs` | 15,357 |
| `crates/sim/src/astro.rs` | 9,125 |
| `crates/physics/src/moment_equivalence.rs` | 7,211 |
| `crates/physics/src/laws.rs` | 7,180 |
| `crates/viewer/src/main.rs` | 6,856 |
| `crates/sim/src/world.rs` | 6,155 |
| `crates/viewer/src/render.rs` | 6,028 |
| `crates/sim/tests/physiology_embodiment.rs` | 5,121 |
| `crates/sim/src/environ.rs` | 4,863 |
| `crates/sim/src/geodynamics.rs` | 4,090 |
| `crates/physics/src/flexure.rs` | 3,712 |
| `crates/sim/src/locomotion.rs` | 3,645 |
| `crates/sim/src/deeptime.rs` | 3,505 |
| `crates/physics/src/opacity.rs` | 3,273 |
| `crates/sim/src/giants.rs` | 2,952 |

Size overstates some production tangles because many files contain large inline test suites. The first main test modules begin at line 7,943 in `runner.rs`, 4,456 in `astro.rs`, 3,051 in `moment_equivalence.rs`, 4,035 in `laws.rs`, 4,275 in `world.rs`, 2,875 in `environ.rs`, 2,251 in `geodynamics.rs`, 1,991 in `locomotion.rs`, 1,639 in `deeptime.rs`, 1,775 in `opacity.rs`, and 1,558 in `giants.rs`. `flexure.rs` begins test/golden material at line 1,666. Test extraction is an early low-risk reduction, but it cannot substitute for production boundaries.

The audit's proposed materials trees also need scale context. `thermoelastic.rs` is 1,301 lines with tests beginning at 860. `conductivity.rs` is 2,691 lines with tests beginning at 1,498. Nine production files for the former and six for the latter would create more navigation than separation at their current sizes.

### Pin and quarantine facts

The `run_world` measurements are confirmed:

- `crates/sim/examples/run_world.rs` is 2,730 lines and declares itself a quarantined, non-canonical development fixture.
- It loads `Profile::Development`, uses `dev-fixtures.toml` or the Mirror fixture, and installs the stated life-cadence and founder-ageing overrides.
- Direct lexical reachability is zero for `geodynamics`, `deeptime`, `flexur`, `thermoelastic`, `conductivity`, `province`, and `moment_equivalence`. Its call path builds `build_dawn_runner`; inspection found no indirect abiotic-physics orchestration that would overturn that lexical result.
- The current release binaries reproduce `40fe8a7269ee4da8974eb1787338c3a0` for default and `be94e3100b9db82f7c1aea1d8091956d` for `--scenario living`.

One owner premise needs correction. In the checked-in repository at the start of this investigation, the two hashes did not mechanically gate every change. The `just pins` recipe printed the expected strings, ran each fixture, and grepped for `final state_hash`; it never compared the emitted value with the expectation. A wrong hash satisfied the grep. During drafting, a concurrent `justfile` change landed an exact comparison and corrected the fixture label. That edit closes the local mismatch hole, but CI still does not invoke `just pins`, `run_world`, or another comparison script, and the inline checker has no failing self-test. The hashes therefore remain outside the per-change gate until that wiring lands. Several comments also refer to four or five pins while the recipe lists two, which is further contract drift.

`docs/QUARANTINE.md` and the `run_world` banner agree that examples are fixtures and that canonical code must be library code plus a real binary. The document's manual list is stale: `crates/sim/examples` contains 17 examples, while the list names 10. The missing seven are `aging_demo`, `biosphere_probe`, `creatures_react_demo`, `planet_live`, `planet_relief`, `planet_volcanism`, and `run_world`. A discovered inventory is safer than another hand-maintained list.

The dawn quarantine is farther along than the problem statement implies. `crates/sim/src/dawn_harness.rs` already carries a strong quarantine banner, and `crates/sim/tests/dawn_harness_quarantine.rs` build-enforces that canonical `sim/src` modules do not call it. Its direct executable consumers are the `aging_demo` and `run_world` examples; integration coverage lives in `mirror_calibrated_boot.rs` and `world_build.rs`. This work should be retained and extended, not recreated.

### Calibration correction

The statement that any `Profile::Calibrated` run refuses while any reserved value remains is too broad. `CalibrationManifest::ensure_all_set` checks the enabled requirement set, not every entry in the global registry. On the current working tree, `reserved.toml` contains 232 entries, of which 118 remain `reserved` and 114 are `set`; despite that, `mirror_calibrated_boot` passes and boots the dawn fixture under `Profile::Calibrated` because its enabled closure is satisfied.

The correct rule is: a canonical physics run must enumerate its required closure, enable it, and refuse if any member is unset or if a required substrate has no registered value path. The planned physics runner is expected to refuse until that closure is complete, but `Profile::Calibrated` by itself does not guarantee refusal. The dawn assembly also installs `dialogue::dev_substrate` unconditionally, so a successful calibrated dawn test does not make that path canonical.

### Viewer and gate facts

The viewer finding is confirmed. The causal scene-building region spans roughly 2,122 lines and holds about 69 physics-chain constants under the audit's classification. It constructs deep-time provinces, steps them, and builds composition-bearing derived scene state. The live viewer caller still uses its local retired isolation-mass derivation while `crates/sim/src/astro.rs` contains the successor.

`DEEP_TIME_MYR_PER_TICK` is passed into `province_column_params`, `step_deep_time`, and impact evolution. It is therefore a physical integration interval on that path even though its prose calls it display cadence. It cannot cross into canonical library code under that label or value. Canonical integration time and viewer sampling cadence must become separate inputs.

The three gate blind spots are confirmed:

- `constructor_gate.py` scans `bio`, `foundation`, `physics`, `sim`, and `world`, but not `viewer`.
- `determinism_gate.py` scans `core`, `physics`, `bio`, `foundation`, `sim`, and `world`, but not `viewer`.
- Stone 0's provenance input hash covers the same canonical families and data, but not `viewer`.

The constructor statement is also confirmed in its intended reading: the viewer has zero `::from_bits(` calls and two `::from_decimal_str(` calls, both in CLI parsing. The advisory surface is much larger, with 301 `::from_int(` and 130 `::from_ratio(` occurrences across production and tests. Those counts are not all defects, but they show why copying the viewer block before classifying each causal input would bypass the value-authoring line.

### `runner.rs` is not an obsolete biology file

`runner.rs` contains 7,942 lines of production code followed by 7,415 lines of tests. Its production half contains four distinct live responsibilities:

- fixed-grid field state, calibration, and thermal stencil operations;
- body/environment and material interaction through `Embodiment`;
- lifecycle configuration and creature bootstrap;
- the `Runner` tick scheduler, which orders world, environment, bodies, learners, lifecycle, and hashing.

`Runner`, `Field`, and `Embodiment` are used by integration tests, examples, evolution code, and the dawn assembly. This is live simulation infrastructure with quarantined fixture callers. Deleting or parking the whole file would discard current behavior rather than isolate it. The correct action is to remove its accidental claim on the physics run, then split its live responsibilities while preserving tick order and digest coverage.

### RNG and chaos facts

The audit's semantic-coordinate proposal is partly present already. `civsim-core::keys::DrawKey` keys draws by region/locus/locus2/tick/phase/slot, and several physics paths use content or physical coordinates directly. Some abiotic paths call `Rng::for_coords` without a registered `Phase`, so namespace observability is incomplete, but the repository is not based solely on call-order RNG.

The Chaos Protocol in `docs/working/R_ASSEMBLY_RESEARCH_QUESTION.md` distinguishes a seed's exact realization from the derived stationary measure for Lyapunov-sensitive assembly. It requires exact seed discipline, conservation projection, and a sourced measure. `DeepTimeState::realization_digest()` already supplies a useful exact realization boundary. The plan should instrument current keys before considering any rekey. Changing coordinates or namespace layout changes realization bytes and requires an explicit model-version decision.

No runner-level model-version field was found in the current pin contract. A stable model identifier must land before the first semantic coupling or physics re-pin; the crate package version is not a substitute for a simulation-model identity.

## Evidence contracts for development

The overloaded phrase `byte-neutral` should leave review vocabulary. Every change should claim one or more of the following, with evidence named beside the claim.

| Claim | What it means | Minimum evidence |
| --- | --- | --- |
| Fixture bit identity | A named fixture produced the same bytes under the same model version and input | Enforced exact comparison against that fixture's expected digest |
| Causal dormancy | None of the changed causal units, state paths, or RNG namespaces ran in the named fixture | A coverage-complete reachability receipt plus a source-to-causal-unit map |
| Semantic equivalence | Old and new implementations preserve the contract over the twinned domain | Values or bands, branch and refusal identity, conservation residuals, provenance, and exact bytes where the mathematics is unchanged |
| Seed-ensemble equivalence | A chaotic mechanism preserves its declared measure within sourced acceptance bands | Exact replay for every seed, then a predeclared ensemble and acceptance rule derived from the Chaos Protocol |
| Intended model change | A physical measure, branch, input basis, or RNG coordinate changed on purpose | Model-version change, reviewed provenance, old/new deltas, and new exact pins |

A fixture identity result cannot support a causal-dormancy claim. A dormancy receipt cannot support semantic equivalence. A seed ensemble cannot waive exact replay.

### What a meaningful pin should contain

The first meaningful direct physics golden should call an existing physics mechanism from library code with existing banked or already-declared fixture inputs. It must not copy constants from the viewer or invent a new tolerance. The smallest useful initial target is the existing column-to-deep-time path because it already has deterministic state, typed refusals, conservation checks, and `realization_digest`. Its golden record should include:

- fixture and provenance identity;
- input digest;
- declared and reached stages;
- checkpoint state digests, including at least the initial and terminal states and every existing physically named transition already exposed by the mechanism;
- branch or refusal identifiers;
- conservation residuals;
- RNG namespace and coordinate identity when draws occur;
- terminal realization digest.

Moment-equivalence and flexure need sibling goldens because a one-dimensional deep-time column cannot observe their load integration and support response. Thermoelastic and conductivity need a property-column golden that feeds the same states the geodynamic path consumes. This creates a small coverage matrix rather than one magic digest:

| Golden | Required reach |
| --- | --- |
| Property column | thermoelastic branch, conductivity branch, validity/refusal, provenance |
| Deep-time column | geodynamic property input, convection, thermal/crust ledger, impacts if enabled, realization digest |
| Load equivalence | yield envelope, moment integral, solver branch, residual |
| Flexural response | load profile, validity regime, response kernel, support/conservation residual |

Existing values may be factored out of current tests into named fixtures without changing their bits. If a required datum exists only as a viewer literal or has no permitted basis, that golden stops with a named missing-input refusal. The plan never fills the gap with a plausible number.

### Reachability receipt design

The audit's receipt is directionally sound, but a list of Merkle roots is too opaque for the first implementation. A root says two runs differ; it does not show which stage or state path differed. Hash width is also secondary to causal coverage. Start with canonically sorted readable records and derive a digest from them using the repository's deterministic hashing substrate. Include a receipt-schema identifier so the hashing representation can change without pretending the world model changed.

The completed shape should carry at least:

```text
RunReceipt
  receipt_schema
  model_version
  runner_id
  profile
  normalized_input_digest
  declared_stages
  reached_causal_nodes
  written_state_paths
  rng_namespaces_and_coordinate_schemas
  checkpoint_digest
  outcome
    Completed { final_state_digest, conservation_residuals, provenance_digest }
    Refused { sorted_refusals, stage, provenance_or_missing_basis }
```

The readable vectors are the audit artifact. A compact digest is derived from the whole record for pinning. `declared_stages` is necessary: a run that silently omits flexure must differ from one that declares flexure and refuses there.

Instrumentation must be explicit and outside hashable simulation state. A test must run the same input with tracing enabled and disabled and compare every simulation digest. Stable causal IDs should name physical stages or contracts, not Rust helper symbols, so a file move does not force a model re-pin. State-path IDs should name semantic state fields. RNG namespace records should expose the existing `DrawKey::phase` routes and named adapters around direct `Rng::for_coords` sites while preserving the current coordinate vector exactly.

Written paths cannot be inferred only by diffing state before and after a stage because a same-value write would disappear. Record them at controlled mutation boundaries. If a state type still permits untracked public mutation, the receipt must declare that coverage gap and cannot support a complete written-path claim yet.

A receipt is proof only over instrumented coverage. Before calling it a `PinReachabilityReceipt`, add a gate that maps every canonical pipeline stage and RNG entry to a registered causal ID and rejects an unregistered stage. Stage-level coverage is enough to prove that dawn fixtures never entered the abiotic crate after the crate split. Symbol-level dormancy claims inside a reached stage require finer registration or a semantic twin. A grep count may remain a diagnostic, but it is not that proof.

### Chaotic domains

For a deterministic refactor, old and new code must first replay every chosen seed exactly. If a later model change intentionally alters a chaotic realization, compare the declared observables across a seed ensemble after bumping the model version. The ensemble size, seed-selection rule, statistics, and acceptance bands must come from the Chaos Protocol's measure definition, a power analysis, or cited physical uncertainty. They cannot be selected to make a patch pass.

The ensemble report should keep per-seed input and output digests, conservation results, refusal counts, and aggregate observables. A changed RNG key schema belongs in the model-version record even if the ensemble remains within its physical bands.

## Minimum viable canonical runner

### Ownership boundary

Create a library crate provisionally named `civsim-planet`, with the invariant: it owns abiotic world construction and deterministic planetary evolution, and it cannot depend on `civsim-sim`, `civsim-bio`, or `civsim-viewer`. `civsim-sim` may depend on it and temporarily re-export moved modules to preserve public paths. The viewer should eventually depend on it directly.

This boundary is preferable to placing a new binary in the current `civsim-sim` package. The present sim crate is the embodied civilization runtime and still exposes the quarantined dawn assembly. A canonical physics runner housed there would preserve the ambiguity the plan is meant to remove.

The minimum runner surface is library first:

```text
crates/planet/
  src/
    canonical/
      spec.rs          # typed world/run inputs, no fixture defaults
      preflight.rs     # enabled requirement closure and capability checks
      pipeline.rs      # fixed stage order
      receipt.rs       # completed/refused structured outcome
    bin/
      run_planet.rs    # thin CLI over the library
```

The binary name is provisional; the ownership and dependency rule are the decision. Every new canonical root must be added in the same slice to the constructor gate, determinism gate, derives/floor coverage where applicable, Stone 0 provenance input hash, workspace CI, and a dependency-boundary gate with a failing self-test.

### Inputs and stage contract

The binary runs only `Profile::Calibrated`. It has no development fallback and no implicit world. Its `CanonicalPlanetRunSpec` reads an owner-designated per-world manifest containing world identity, seed, initial per-world data, termination target, and any numerical-policy data the owner rules to be per-world. Universal inputs come only from the fundamental-constant floor. Derived quantities are computed. Unresolved quantities are reserved with their basis and cause refusal.

Mirror may become the first world only after the owner explicitly designates it and the current profile-override differences are resolved as canonical world data. The passing Mirror dawn test is not evidence for that designation.

The minimum declared physics arc is:

1. star, disk, and planetary-system inputs;
2. planet assembly and composition;
3. young thermal state and material property column, including thermoelastic and conductivity branches;
4. geodynamic and deep-time evolution with thermal, crust, impact, and relief ledgers as enabled by the world;
5. load moment equivalence and flexural response for the declared surface-load representation;
6. a render-free `PlanetEvolutionSnapshot` and completed run receipt.

Preflight must distinguish `enabled`, `reached`, `refused`, and `skipped-by-world-data`. A required stage cannot silently degrade to a display fallback. If the finite-load or field adapter needed for the declared planet is still absent, the arc refuses with that capability named. A periodic one-dimensional diagnostic must not be promoted as a planetary surface result.

`DEEP_TIME_MYR_PER_TICK` does not move into this spec. The canonical physical step must derive from an integration stability/error substrate, or be read as owner-approved numerical/per-world data with a basis. Viewer frames sample or interpolate canonical state on a separate cadence. Until that ruling and substrate exist, preflight exposes the missing physical-step basis.

### What is worth pinning

A final-state digest alone is too weak for a dissipative system because distinct trajectories can converge. The completed pin is the digest of the full receipt, including normalized inputs, declared/reached stages, checkpoint trajectory, final state, conservation residuals, RNG namespaces, refusal-free outcome, and provenance. `World::state_hash` should retain its deliberately documented state boundary; runner input identity belongs beside it in the receipt rather than being smuggled into that hash.

The model version changes when a physical mapping, branch measure, calibrated input, state interpretation, or RNG coordinate schema changes. A receipt-schema-only change does not change the model version if a twin proves the simulated state bytes and behavior unchanged. A path-only file move with compatibility re-exports is likewise a structural change, though Stone 0's source-input fingerprint may need its own expected update because source paths changed.

### Calibrated refusal and the proposed refusal digest

A completed canonical state pin cannot exist for a world whose enabled requirement closure refuses. The plan should say that plainly rather than pin a development run under a canonical name.

A structured refusal receipt is coherent as a diagnostic and change-review artifact. Its readable content should include sorted refusal IDs, stage, missing basis or provenance, declared/reached stages, input identity, receipt schema, and model version. A digest over that record makes artifacts comparable and detects accidental drift.

It is a poor acceptance pin. A green test that requires the current real world to keep refusing would turn a known stop into expected behavior. The recommended policy is:

- test the refusal machinery and canonical ordering with synthetic manifests;
- make the real `run_planet` command exit with a documented nonzero status on refusal and print the readable receipt;
- expose the current receipt in a clearly named readiness report or status check;
- do not call its digest a canonical world pin and do not make exact persistence of the current refusal set a passing regression condition;
- land the first canonical completion pin only when the selected world's required closure is set and every declared stage completes.

Whether the readiness status is merge-blocking while calibration remains open is an owner CI-policy ruling. The two acceptable choices are a required red readiness gate or a visible non-required status. Folding refusal into an ordinary green test is excluded. In either policy, a newly added refusal and a removed refusal must be readable without comparing opaque hashes.

## Retiring the dawn-band and biology entanglement

Here, "retire" means remove a path's authority and dependency role. It does not mean delete behavior that still has callers or unique tests.

### Current status by category

| Category | Items | Finding | Planned treatment |
| --- | --- | --- | --- |
| Live canonical-capable mechanisms | `Runner`, `Field`, `Embodiment`, `World`, `EnvironFields`, locomotion, physiology kernels, and their supporting state | Used by current tests, examples, or evolution paths; many are intended civilization runtime | Keep, split by invariant, and make them consumers of canonical planet snapshots later |
| Live fixture assembly | `dawn_harness`, `run_world`, `aging_demo`, `world_build` tests, Mirror dawn boot | Executes and supplies regression coverage, but reads quarantined values and development substrate | Preserve in a fixture-only lane with enforced exact hashes and an explicit noncanonical name |
| Quarantined live debt | The ledgered anatomy, conviction-experience, discovery, edibility, homeostasis, morphogen, and broader parked biology values | Code still runs in fixtures; values owe derive-or-cite work | Freeze under the existing ledger and prevent canonical dependency; graduate one concept only when its substrate and provenance are ready |
| Dormant but intended | Opt-in branches and later civilization mechanisms that compile and have tests but have no canonical caller yet | Lack of reach is expected staging, not deletion evidence | Keep their tests and ownership; mark activation requirements and avoid claiming coverage from the planet runner |
| Retired model still executing | The viewer's local isolation-mass derivation | A successor exists in `sim::astro`, but the old derivation remains on the live viewer path | Twin, replace as a semantic model change, then delete the retired implementation |
| Dead code | None established by this investigation | Physics nonreachability does not prove a biology mechanism has no callers, future owner, or unique regression value | Require a zero-caller scan, no declared future ownership, and no unique test before removal |

The six modules named by the dawn banner are not six dead modules. They are live or fixture-live modules containing frozen weak-provenance values. Their quarantine status is about value authority, not reachability or usefulness.

### Definition of "no longer load-bearing for physics"

The dawn stack is out of the physics path only when all of these are measurable:

1. `civsim-planet` and its canonical binary have no dependency on `civsim-sim`, `civsim-bio`, the dawn harness, or a viewer crate. A dependency-boundary gate enforces this.
2. Physics PR checks run direct property, deep-time, moment, and flexure goldens regardless of whether a dawn fixture is run.
3. The canonical run and its receipt originate in planet library code. No `examples/` target is described as canonical in recipes, CI, or docs.
4. The viewer consumes `PlanetEvolutionSnapshot` or another render-free planet result and does not assemble causal planetary state.
5. Dawn hashes and integration tests remain in a separately named fixture lane. Their pass or failure says nothing about physics coverage unless a receipt names a shared reached mechanism.
6. A future full-civilization runner composes an owner-approved planet state with the civilization runtime through a typed boundary. It does not call `build_dawn_runner` as a shortcut.

### Preserve the existing regression coverage

Do not delete the two known dawn hashes. Convert them into enforced fixture pins, retain the default and `living` scenarios, and keep the current world-build and calibrated-manifest integration tests. Keep slow evolutionary tests in the existing nightly lane. Add explicit fixture identity to failure output so a developer sees which noncanonical scenario changed.

After the planet boundary is established, move `dawn_harness.rs` and its direct executable/test consumers into a workspace package such as `civsim-dawn-fixtures`. That package depends on `civsim-sim`; no canonical package may depend on it. This is stronger than a public re-export guarded by a textual test and makes the dependency graph carry the quarantine. Preserve the two run results exactly during the move.

This extraction is conditional on a public-API audit. If it would require exposing sim internals solely for the fixture, keep the harness feature-gated inside `civsim-sim` for one transition slice, move the executable consumers first, and record the remaining private seam. Do not weaken encapsulation to make the directory move look complete. The end state still removes the harness from the default sim library surface.

When `run_world.rs` next needs source changes, keep a thin example or binary entry point and split its fixture-only helpers into `cli`, `fixture_inputs`, `dawn_setup`, and `report` modules under its fixture package. This is a pure move with the invariant that no helper becomes canonical library API. Its risk is changing argument defaults or output formatting that the pin parser reads; the enforced fixture checks catch both.

The eventual name `Runner` may be too broad once `run_planet` exists. A rename to `LifeRunner` or `CivilizationRunner` is an API change and should wait until the planet boundary and dawn-fixture package make the two roles clear. It is not required to unblock physics.

## Sequenced execution slices

Every commit in these slices must compile the affected workspace tree. Every slice ends with the standard repository checks plus the measurements listed for that slice. Structural and semantic work never share a commit. A structural move means the simulation state and branch/refusal results are byte-identical, even if source paths and a source-provenance fingerprint change.

### Slice 1: finish and enforce the current fixture pin contract

This is the single first move.

Rename the contract from canonical pins to dawn-fixture pins, place the two expected digests in one machine-readable fixture manifest, and add a checker that compares exact emitted values. Preserve the landed inline comparison, then extract its contract so CI and local recipes call the same checker. It must reject a missing result, duplicate result, malformed result, nonzero fixture exit, and digest mismatch. Give the checker a synthetic failing self-test and run it from CI. A compatibility `just pins` alias may remain briefly, but its output must name the dawn fixture and direct callers to the precise recipe.

Do not touch `run_world.rs` in this slice. The current bytes already reproduce the documented values. Update only the contract, recipe, checker, CI wiring, and prose that calls the two values canonical.

Measurements:

- both current scenarios pass exact comparison;
- changing one expected hexadecimal digit makes the checker fail;
- replacing the output with a different well-formed digest makes it fail;
- CI contains an invocation of the checker;
- the emitted fixture hashes remain the two measured values;
- a repository search finds no remaining claim that these two values are canonical physics pins.

Why first: it is small, needs no physical value or ownership ruling, preserves current regression value, and prevents every later refactor from citing a check that does not enforce its expectation. It does not make a physics pin meaningful by itself. Slice 2 does that.

### Slice 2: establish direct physics evidence before moving code

Build the four golden-state harnesses described above around current library APIs. Start with the column/deep-time golden, then add property, moment, and flexure goldens. Reuse current banked inputs or factor named inputs out of existing tests without changing bits. Where an input lacks an allowed source, emit a named refusal and leave that golden incomplete rather than authoring a replacement.

In the same slice family, define the receipt vocabulary and a versioned readable serialization. Add stage-level tracing to these harnesses first. Add a trace on/off identity test. Record the current direct `Rng::for_coords` coordinate shapes without changing them. Add a discovered quarantine inventory gate so every example is either declared fixture/performance tooling or fails the scan; this closes the stale `QUARANTINE.md` list.

Classification: evidence instrumentation and tests. It must be simulation-byte preserving. If adding tracing changes a state digest, stop the slice.

Measurements:

- every completed golden reports its expected causal stages and no undeclared stage;
- perturbation tests show each recorded state field affects its component digest;
- trace-enabled and trace-disabled runs have identical state and realization digests;
- a changed branch or refusal ID fails the appropriate golden;
- deleting a stage registration or adding an unregistered RNG entry fails a self-test;
- the dawn receipt, once stage tracing is available, reports no abiotic stage; until then, only the lexical finding may be cited.

### Slice 3: establish the abiotic crate boundary

First move the derived Stefan-Boltzmann composite function from `sim::physiology` to its natural nonliving owner in `civsim-units`, preserving its exact Q32.32 and fine-scale bits. Re-export it temporarily from physiology so current callers and docs continue to compile. This removes the one production dependency by which `astro` and `stellar` reach into physiology.

Create `civsim-planet`, add it to every canonical scan and provenance input set, and install an allowed-dependency gate. Then move the abiotic modules in dependency order, retaining `civsim_sim::<module>` compatibility modules while downstream callers migrate:

1. move the tightly linked `astro`, `stellar`, `stellar_evolution`, `planetary_system`, and `giants` group together so runtime and rustdoc paths never break;
2. move `planet`, `moons`, `secular`, and `geodynamics_surface` as their dependencies permit;
3. move `smallbody`, then `planetary_assembly`;
4. extract the rigid-rigid convection constants to a shared convection contract, move `geodynamics`, then move `deeptime` and `flexural_field`;
5. move inline tests with their owning mechanisms, converting only cross-module private tests that cannot follow into named integration tests.

Before the first move, generate and check in the measured module dependency graph as a migration receipt. Doc-only edges and test-only edges must be marked so they do not dictate false production architecture. Each module or tightly coupled group is one compiling commit.

Classification: pure moves and one exact derived-constant ownership move. No formula, branch order, iteration order, constant bits, state hash fold, RNG coordinate, or refusal variant changes. Moving source may update Stone 0's source-input fingerprint; that is recorded separately and is not a world re-pin.

Measurements for every move:

- `cargo check --workspace --all-targets` and the affected crate tests pass before the next module moves;
- old `civsim_sim` paths and new `civsim_planet` paths produce identical values in a twin test;
- all four physics state/outcome goldens and both dawn-fixture pins are unchanged; any source-path fingerprint update is reviewed separately;
- the dependency gate rejects a synthetic `planet -> sim`, `planet -> bio`, or `planet -> viewer` edge;
- constructor, determinism, derives/floor, and Stone 0 scans report the new root;
- after compatibility callers migrate, no viewer physics caller needs `civsim-sim` merely to reach abiotic modules.

### Slice 4: land the canonical front door in refusal-capable form

Add `CanonicalPlanetRunSpec`, preflight, fixed pipeline ordering, structured receipts, and the thin `run_planet` binary. Select no implicit world. Wire `Profile::Calibrated` only. Enumerate the required calibration and capability closure for each declared stage.

The pipeline may land while the real world refuses. That is a useful front door because it makes missing substrates and owner values concrete, but it is not yet a completed canonical pin. Use synthetic manifests to prove completed and refused receipt determinism without changing the visibility of the real refusal.

Classification: new orchestration over existing mechanisms. Treat any first live coupling between previously dormant stages as semantic even when individual kernels are unchanged, because it changes the simulated world. Such activation requires a model version, conservation checks, and review of every input path.

Measurements:

- Development data cannot be selected through the binary or library canonical entry point;
- each reserved member of the enabled closure produces a readable refusal at its stage;
- an unrelated reserved registry entry does not block the run;
- permuting manifest file order does not change the receipt;
- tracing on/off leaves simulated bytes unchanged;
- a declared stage cannot disappear without changing or refusing the receipt;
- the real command is nonzero and visibly refused until its selected world is ready.

### Slice 5: remove dawn assembly from the default runtime surface

Create the fixture package and move `dawn_harness`, `run_world`, `aging_demo`, and the direct dawn integration tests into it, subject to the public-API audit above. Retain the quarantine ledger and its anchor gate in the crates that own the values. Replace the textual no-import test with a package dependency rule once the stronger boundary exists.

Classification: pure simulation move with API and command-path changes. No model bump if the two fixture bytes and all branch/refusal results remain identical. Removing the old public re-export is an announced API break and should be isolated from physics changes.

Measurements:

- the two fixture hashes remain exact;
- all moved world-build and Mirror tests pass in the fixture package;
- default `civsim-sim` compilation does not compile or export the dawn assembly;
- no canonical package depends on the fixture package;
- every quarantine-ledger anchor still resolves;
- the slow biology lane remains scheduled.

### Slice 6: classify every viewer causal input before extraction

Create a machine-readable ledger for the causal scene-building region. For each of its physical constants, fallbacks, and CLI bridges, record one disposition:

- fundamental constant already owned by the floor;
- per-world data with an owner-approved source;
- derived from an existing substrate;
- missing substrate to build;
- reserved with basis and therefore refusing;
- display-only value that cannot enter causal state;
- development-fixture input retained only by the old viewer adapter.

Give `DEEP_TIME_MYR_PER_TICK`, the default crust thickness, atmosphere/quench values, formation and melt inputs, impact inputs, radiogenic inputs, and young-thermal inputs explicit rows. Resolve duplicate or retired formulas by naming old and successor versions; do not silently choose one.

Classification: analysis, schemas, and fail-loud surfacing. Any replacement of a live viewer value or formula is deferred to a semantic slice.

Measurements:

- every causal `Fixed` constructor and every fallback in the region resolves to one ledger row;
- no ledger row says only "reasonable", "typical", or another unsupported magnitude claim;
- removing a source expression or adding a new causal value makes the ledger gate fail;
- display cadence and physical integration interval have separate typed paths;
- unresolved rows become preflight refusals, not defaults.

This slice is blocked on owner rulings for values whose correct class cannot be derived from repository evidence. The ledger itself is not blocked.

### Slice 7: extract viewer physics through a semantic twin

Define render-free planet input and output types. Move mechanisms, not viewer constants: parameterize the library mechanisms over typed inputs, and let a clearly named development adapter continue to supply the viewer's old fixture values while twins are established. The canonical adapter reads calibrated/per-world data and refuses missing inputs.

Move `build_deep_time_provinces`, `step_provinces`, and the physical part of `build_derived_scene_with_composition` behind the planet API. Keep color, camera, `SurfaceParam`, cache, interpolation, and playback decisions in the viewer. Compare the old and extracted development paths over the same input before deleting the old implementation.

Only after the exact extraction passes should a separate semantic slice replace the retired isolation-mass derivation with its successor. That slice changes the model version and records downstream state, band, branch, conservation, provenance, and ensemble effects where the assembly protocol requires them.

Classification: first a pure extraction using explicit old fixture inputs; then one or more semantic changes. Never combine them.

Measurements:

- the old and new development adapters are byte-identical at every recorded checkpoint before semantic replacement;
- the viewer receives no mutable causal planet state and owns no physical integration loop afterward;
- a gate rejects causal planet construction in `crates/viewer/src`;
- the viewer's rendered frame derived from a fixed snapshot remains identical for the pure extraction;
- the isolation-mass successor slice shows the intended delta under a new model version and passes the Chaos Protocol evidence if the affected assembly is sensitive;
- constructor and determinism gates cover the new library location.

### Slice 8: complete and pin the first calibrated planet

Resolve the selected world's required closure through owner-set per-world data or built derivations. Close the open physical capabilities required by the declared stage list. Run the canonical pipeline to completion and check in the readable completed receipt plus its derived digest.

Classification: semantic activation. This is the first canonical physics pin and must carry a model version.

Measurements:

- no development or fixture profile appears in the normalized input receipt;
- the outcome is `Completed`, with an empty refusal set;
- every declared required stage is reached;
- exact reruns, clean builds, and supported serial/parallel execution produce identical receipts;
- conservation and validity residuals lie within their already-declared contracts;
- every state field covered by the final digest has a perturbation test;
- the four subsystem goldens remain useful and continue to run beside the end-to-end pin.

### Slice 9 and onward: finish structural decomposition in the final homes

Apply the file map below one invariant at a time. Test-only extractions may begin after Slice 1 when they do not collide with active work. Production modules that are moving to `civsim-planet` should be split after relocation so source history moves once and the final owner receives the clean tree. High-fanout `laws.rs` is handled one domain per commit. Every touched large file follows the standing modularization instruction.

## File decomposition by invariant

### Common move discipline

For every pure split:

1. establish or identify an exact golden before moving code;
2. create the child module and compatibility re-exports;
3. move one invariant with its tests and docs, without cleanup or formula edits;
4. run targeted tests, exact fixture/golden comparisons, `cargo check --workspace --all-targets`, clippy, rustdoc, and repository gates;
5. remove compatibility exports only in a later API slice.

Do not reorder `BTreeMap` walks, iterator reductions, branch tests, fixed-point operations, hash folds, event phases, or RNG calls while moving code. Formatting and import changes are allowed; arithmetic expression cleanup is a semantic risk and waits.

### Proposed split map

| Current file | Proposed modules | Invariant that warrants the boundary | Classification | Main break risk and prerequisite |
| --- | --- | --- | --- | --- |
| `sim/src/runner.rs` | `runner/field`, `runner/thermal_exchange`, `runner/embodiment/{state,material_actions,perception}`, `runner/lifecycle`, `runner/step`, `runner/digest`, and child test modules | `field` owns deterministic lattice state and traversal; thermal exchange owns the body/medium energy transaction; embodiment owns located body actions; lifecycle owns birth/age/death cadence; `step` alone owns phase order; `digest` alone declares hashed runtime state. | Pure move first. Later tick or API redesign is semantic. | Tick phase order, mutable aliasing, hash-field omission, and public re-exports. Extract tests first. Move `digest` only after a field-by-field perturbation matrix proves coverage. |
| `sim/src/astro.rs` | In `planet`: `astro/irradiation_orbits`, `astro/cloud_collapse`, `astro/disk`, `astro/stellar_activity`, `astro/formation`, with a small facade | Each child owns one physical input-to-output regime and its validity/refusal contract. Coupled disk equations remain one invariant rather than becoming formula-per-file fragments. | Pure move; retired-formula replacement is semantic. | Fixed-point evaluation order, domain refusals, doc links to stellar modules, and constants shared with giants. Move the astro/stellar group before splitting. |
| `physics/src/moment_equivalence.rs` | `moment_equivalence/yield_envelope`, `moment_integral`, `line_load`, `axisymmetric_load`, `solver`, `validity`, and tests | The yield envelope owns brittle/ductile selection and transition identity; the integral owns section moment; geometry adapters own load measure; the solver owns convergence and residual; validity owns refusals. | Pure move. Typed `LoadKind` or new geometry is semantic. | Quadrature order, brittle/ductile transition identity, root bracket, refusal variant, and residual. The audit's separate brittle and ductile files would split one branch contract. |
| `physics/src/laws.rs` | Domain modules for nutrition, solid mechanics, thermal transport, fluids, phase equilibrium, kinetics, radiation, physiology, electrochemistry/electromagnetism, and time/geodynamic helpers; shared checked arithmetic remains central | A domain module owns laws sharing dimensions, validity domains, and conservation meaning. Arithmetic helpers remain common only when their contract is domain-neutral. | Pure move one domain at a time. | Highest fanout in the tree, name collisions, doc-link resolution, and accidental duplicate helpers. No mass rename and no formula cleanup during extraction. |
| `viewer/src/main.rs` | First remove causal physics to `planet`; then `viewer/cli`, `commands`, `playback`, and `scene_adapter` | After extraction, each child owns an observer concern: input parsing, command dispatch, presentation time, or immutable snapshot adaptation. Causal evolution has no viewer invariant and therefore gets no viewer child module. | Physics extraction is blocked and mixed as described in Slices 6 and 7. The remaining UI split is pure. | Laundering physical values, display/physics timestep confusion, CLI defaults, and event order. Splitting the causal block into viewer submodules is refused because it would entrench the wrong owner. |
| `sim/src/world.rs` | `world/state`, `construction`, `lifecycle`, `cognition`, `language_dialogue`, `transmission`, `digest`, and child tests | State owns stored civilization facts; construction establishes valid initial relationships; each transition module owns one event domain; digest declares the exact replay boundary. | Pure move; later composition with planet is semantic. | Canonical tick order, event-log order, and deliberate hash boundaries. Digest extraction waits for perturbation coverage. |
| `viewer/src/render.rs` | `render/palette`, `projection`, `surface_field`, `cpu_globe`, `gpu_cache`, `system_map`, `picking`, and tests | Every child is a pure or cached transformation from an immutable snapshot and view state to pixels, geometry, or selection. None may advance physical state. | Pure observer move after causal physics leaves the viewer. | Pixel identity, cache keys, CPU/GPU parity, and accidentally moving physical state into a rendering module. |
| `sim/tests/physiology_embodiment.rs` | Shared test support plus separate physiology, material-actions, tools/earthworks, fire, matter-cycle, and shelter/respiration integration files | Each test file proves one end-to-end biological transaction; shared support only constructs preexisting named fixtures. | Pure test move and safe early work. | Hidden fixture setup order and duplicated helpers drifting. Shared support must contain no new world values. |
| `sim/src/environ.rs` | `environ/fields`, `calibration`, `abiotic_sources`, `hydrology`, `productivity`, `insolation`, `digest`, and child tests | Each process owns one environmental field update and its conservation/source rule; calibration owns data resolution; digest owns replay coverage. | Pure move; changing source coupling is semantic. | Field traversal order, calibration lookup behavior, runner coupling, and hash coverage. Keep all updates to one environmental field with its conservation rule. |
| `sim/src/geodynamics.rs` | In `planet`: `geodynamics/column`, `column_properties`, `convection`, `secular_cooling`, `field_adapter`, and tests | Column state is the stored contract; `column_properties` jointly resolves compatible thermal and rheological properties; convection and secular cooling own distinct state transitions; the field adapter owns spatial projection. | Pure move and split; activation or property-model replacement is semantic. | Do not split thermo and rheology into independent dispatch paths as the audit sketch suggests. Extract the shared critical-convection contract from the current deeptime test dependency first. |
| `physics/src/flexure.rs` | `flexure/validity`, `scaled_math`, `special_functions`, `green_functions`, `plate_response`, `golden`, and tests | Validity owns admissible regimes; scaled and special-function kernels own numerically stable representations; Green functions own impulse response; plate response owns load aggregation and residual. | Pure move. Load representation and finite-disc work are semantic. | Summation/evaluation order, singular-limit branches, validity refusals, and load/support residuals. A separate `units` file is unwarranted unless it owns conversion contracts rather than a handful of constants. |
| `sim/src/locomotion.rs` | `locomotion/params`, `resource_field`, `walker`, `speed`, `navigation`, `derived_taxis`, and tests | Parameters resolve movement data; fields expose identity-blind observations; walker owns state transition; speed owns body/medium capability; navigation and taxis own direction selection from declared observations. | Pure move. New navigation behavior is semantic. | Coordinate ordering, field read timing, controller observation layout, and collision/refusal branches. |
| `sim/src/deeptime.rs` | In `planet`: `deeptime/state_digest`, `thermal_crust`, `impacts`, `relief`, `chronology`, `province_coupling`, and tests | State/digest owns realization identity; thermal and crust transfers stay together under one ledger; impacts own their semantic RNG coordinates; relief owns surface response; chronology owns time mapping; province coupling owns spatial exchange. | Pure move and split. Timestep, melt production, or new coupling is semantic. | RNG coordinates in bombardment, ledger ordering, state-digest coverage, convergence, and the geodynamics link. Province activation is blocked on the timestep and open floor items. |
| `physics/src/opacity.rs` | `opacity/rosseland`, `gas_continuum`, `h_minus`, `grain_mie`, `mixing`, `aggregate`, and tests | Each opacity mechanism owns one transport regime and validity domain; mixing combines same-state contributions; aggregate owns ordered dispatch and refusal. | Pure move. Regime or data changes are semantic. | Dispatch order at regime boundaries, table provenance, interpolation order, and refusal identity. Keep branch selection with its validity contract. |
| `sim/src/giants.rs` | After relocation: `giants/formation`, `disk_budget`, `termination`, `classification`, and tests | Formation owns growth; disk budget owns conserved gas availability; termination owns stop conditions; classification reads the completed state without feeding growth. | Defer unless touched; then pure move. | Production is only about 1,557 lines. Gas-ledger conservation, termination branch, and disk-clock coupling matter more than file count. Avoid splitting every equation. |
| `materials/src/thermoelastic.rs` | For now extract `tests`. If production grows, use `thermoelastic/contract`, `elastic_debye`, and `mgd` | The contract owns dispatch, validity hull, and refusal; the latter two are distinct equation families only when each has enough implementation to stand alone. | Test move now; production split deferred. | The audit's nine-file tree is premature for about 859 production lines and could separate branch hull, validity, and refusal. |
| `materials/src/conductivity.rs` | After the current estimator/phase-conductivity arc has a stable baseline: `conductivity/estimators`, `phase`, `assemblage`, and tests | Estimators own candidate laws and validity; phase owns deterministic law selection for one phase; assemblage owns ordered phase aggregation and provenance. | Pure move, deferred until the semantic arc has its golden baseline. | Dispatch precedence, phase ladder, provenance, aggregation order, and overlap with fresh semantic work in the same substrate. The six-file audit tree is more granular than current invariants require. |
| `sim/examples/run_world.rs` | In the fixture package: thin entry plus `cli`, `fixture_inputs`, `dawn_setup`, and `report` | CLI owns syntax; fixture inputs own labeled development data; dawn setup owns fixture assembly; report owns stable human and machine output. None is canonical API. | Pure fixture move. | Scenario defaults, explicit overrides, output parse contract, and accidentally exposing fixture numbers as library defaults. |

### What is blocked and what is not

Pure test extraction, `runner` responsibility splits, `laws` domain extraction, moment-equivalence grouping, opacity grouping, and observer-only render splits need no physical ruling once their exact goldens exist. They are blocked only by missing coverage or overlapping active edits.

The following work is blocked behind an explicit ruling or substrate and must not be disguised as a file move:

| Blocker | Work it blocks | Required resolution |
| --- | --- | --- |
| Canonical world identity and termination data | First completed planet pin | Owner designates per-world data and its provenance; no fixture is promoted implicitly |
| Physical integration interval | Canonical province/deep-time activation and viewer extraction | Build a stability/error derivation, or owner-classify a numerical/per-world value with basis; separate it from frame cadence |
| Viewer causal-input ledger rows | Moving the causal scene builder into canonical library code | Derive, source as per-world, reserve with basis, or keep explicitly fixture-only |
| F3 finite-disc and typed-load questions | Declaring the surface flexure stage complete | Resolve load geometry/field adapter and `LoadKind` ownership; a one-dimensional diagnostic cannot stand in |
| F7 melt-mass basis and conserved-ledger source/sink tags | Canonical crust/melt conservation receipt | Build the missing melt-fraction basis and preserve source/sink provenance in the ledger |
| F6 stagnant-lid production ruling | Activating zero production in the visible province field | Owner decides whether the physically derived zero disables the current field or another substrate is required |
| Model-version policy | Any retired-formula replacement, RNG rekey, or newly live coupling | Define version ownership and re-pin review before semantic work |
| Refusal-status CI policy | Continuous presentation of the incomplete real canonical run | Owner selects required-red or visible-nonrequired status; green expected refusal is excluded |

Moving and splitting `geodynamics` or `deeptime` exactly is not blocked by these physical decisions. Calling their currently incomplete viewer coupling canonical is blocked. Likewise, parameterizing a viewer mechanism over typed inputs can proceed; supplying canonical values for unresolved inputs cannot.

## Standard falsifiers at each slice

The executor should attach a short measurement receipt to every slice:

- exact dawn-fixture results before and after;
- applicable physics golden results before and after;
- model version and receipt schema before and after, with a reason for any change;
- reached causal nodes and RNG namespaces for the exercised harness;
- state-field and refusal/branch twins for structural work;
- conservation residuals and physical/data provenance identity;
- `cargo fmt --all --check`;
- targeted `cargo clippy` and tests for moved crates;
- `cargo check --workspace --all-targets` at every compiling commit;
- full repository CI-equivalent checks at each slice boundary;
- `git diff --check` and gate self-tests.

For a pure move, any simulation digest, branch/refusal, residual, RNG coordinate, or physical/data provenance change falsifies the claim and stops the move. A source-path fingerprint may change and must be reviewed in its own receipt. For a semantic slice, an unchanged model version falsifies the process even if the old dawn hashes remain unchanged.

## What this plan would not do

- It would not add more `run_world` scenarios and call them physics pins. The harness cannot reach the physics arc.
- It would not discard exact replay in favor of a seed ensemble. Ensemble evidence begins after per-seed determinism is proven.
- It would not treat a reachability receipt as output validation. A receipt can show that code ran while the code produced wrong state.
- It would not claim dormancy from a receipt whose instrumentation coverage is unmeasured. Stable stage registration and a coverage gate come first.
- It would not rekey all RNG draws to implement the audit sketch. Semantic coordinate keys already exist. Instrument current coordinates; change them only as a versioned model decision.
- It would not start with Merkle roots or mandate `Hash256`. Readable canonical records and coverage solve the present problem. Hash representation can evolve under a receipt-schema version.
- It would not split brittle and ductile strength into separate ownership units. Together with their transition they form one yield-envelope contract.
- It would not split geodynamic thermal and rheology dispatch so far that one column can be assembled from mutually inconsistent branches. `column_properties` owns their coherent result and refusal.
- It would not create nine thermoelastic production files or six conductivity production files now. Their present production sizes and invariants support fewer modules.
- It would not copy the viewer's 69 classified constants, 30 km fallback, or `DEEP_TIME_MYR_PER_TICK` into `sim` or `planet`. Each value must derive, be fundamental, be per-world data, remain fixture-only, or refuse with basis.
- It would not replace the viewer's retired isolation formula in the same commit that moves it. Exact extraction and semantic retirement need separate evidence.
- It would not scan the whole viewer with canonical constructor rules as a substitute for extraction. CLI and rendering fixtures would add noise while causal code remained in the wrong crate. Move causal ownership, then prohibit it from returning.
- It would not require every global reserved entry to be set before any calibrated run. The required enabled closure is the correct fail-loud boundary.
- It would not make the current real refusal set a green golden. A refusal digest is a diagnostic checksum, not a completed-world pin.
- It would not delete the dawn harness, biology mechanisms, or their slow tests merely because the physics runner should not depend on them.
- It would not place the canonical planet binary in the current sim package. That would preserve the ambiguous ownership boundary.
- It would not fold runner input identity into `World::state_hash` without a separate ruling. The receipt can bind inputs and state while retaining deliberate hash boundaries.
- It would not refactor all large files in one branch. Small, compiling, independently falsifiable moves are the safety mechanism.

## End state

The plan is complete when the repository can answer five questions mechanically:

1. Which world, profile, model version, and inputs ran?
2. Which physical stages, state paths, and RNG namespaces were reached?
3. Did the same input replay exactly?
4. Which local invariant broke if the end-to-end receipt changed?
5. Is a stop a visible refusal, rather than a default, fallback, skipped stage, or expected green limitation?

At that point the dawn hashes still protect dawn behavior, physics goldens protect local mechanisms, the canonical planet receipt protects a real calibrated arc, the viewer observes rather than authors physics, and the large files are split along contracts that can each be tested. The three original problems then disappear together because the ownership boundary and the evidence boundary finally describe the same world.
