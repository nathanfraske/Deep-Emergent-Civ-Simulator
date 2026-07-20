# Untangle plan: ownership, canonical execution, and meaningful pins

Status: planning document. This document proposes sequenced work; it does not declare any proposed mechanism implemented.

Evidence date: 2026-07-20, against the current working tree. Measurements involving `calibration/reserved.toml` describe that working tree, which already contains owner edits.

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

One owner premise needs correction. In this repository the two hashes do not mechanically gate every change. The `just pins` recipe prints the expected strings, runs each fixture, and greps for `final state_hash`; it never compares the emitted value with the expectation. A wrong hash still satisfies the grep. The CI workflow does not invoke `just pins`, `run_world`, or another comparison script. The hashes are documented expectations today, not enforced pins. Several comments also refer to four or five pins while the recipe lists two, which is further contract drift.

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

