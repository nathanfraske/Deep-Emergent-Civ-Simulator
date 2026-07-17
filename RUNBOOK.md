# Runbook: Repository Standup, Build Scope, and Reserved Values

This runbook stands the project up as a repository, says exactly what can be built and tested today versus what is held for the owner's calls, and specifies how reserved values and tuneables work, how they are surfaced, and how they are decided. The owner and copyright holder is Nathan M. Fraske; the license is Apache License 2.0.

---

## 1. Repository standup

### 1a. Directory layout

```
<repo-root>/
  LICENSE                      # verbatim Apache License 2.0 text
  NOTICE                       # copyright notice (below)
  README.md                    # what this is, how to build, where the docs are
  CLAUDE.md                    # the operating manual (agent entry point)
  HANDOFFS.md                  # rolling session log
  TODOS.md                     # live backlog mirror + reserved-values queue
  AGENTIC_ADDENDUM.md          # panels, hooks, MCP, memory mechanics
  RUNBOOK.md                   # this file
  Cargo.toml                   # Rust workspace
  rust-toolchain.toml          # pinned toolchain for reproducibility
  .gitignore
  .mcp.json                    # project MCP servers (filesystem, projectops)
  .claude/
    settings.json              # hooks (SessionStart, PreToolUse, PostToolUse, Stop)
    hooks/                      # session-start.sh, customs-guard.sh, post-edit-check.sh, stop-gate.sh
  docs/
    design.md                  # the design document (the 64-part specification)
    audit.md                   # the audit and remediation log
    research/                  # the archived research papers, verbatim
  calibration/
    reserved.toml              # the calibration manifest (Section 4)
  scripts/
    verify.sh                  # the verification suite as one callable script
  tools/
    projectops_server.py       # the projectops MCP server (addendum Section 3)
  crates/
    core/                      # the determinism core (buildable now)
    sim/                       # the simulation systems (built in staged order)
```

The two maintained documents move into `docs/` under shorter names; the verification suite and the manual's commands point at `docs/design.md` and `docs/audit.md`. The archived research papers go into `docs/research/` verbatim and are never rewritten to the prose customs.

### 1b. License and copyright

Add the verbatim Apache License 2.0 text from `http://www.apache.org/licenses/LICENSE-2.0` as `LICENSE`. Add a `NOTICE` file:

```
<Project Name>
Copyright 2026 Nathan M. Fraske

This product includes software authored by Nathan M. Fraske.
```

Every Rust source file carries the standard Apache header:

```rust
// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
```

Set the workspace metadata so the license and author propagate:

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/core", "crates/sim"]
resolver = "2"

[workspace.package]
authors = ["Nathan M. Fraske"]
license = "Apache-2.0"
edition = "2021"
```

Pin the toolchain (`rust-toolchain.toml`) so builds are reproducible across machines, which matters for a project whose whole premise is determinism.

### 1c. Hooks, MCP, and the verification suite

Install `.claude/settings.json` and `.mcp.json` per the agentic addendum, and write `scripts/verify.sh` as the single callable form of the verification suite (the same checks the manual lists, emitting a clear pass-or-fail summary and, with a flag, structured JSON for the panels). Wire `scripts/verify.sh` into continuous integration so every push re-runs it on the documents.

---

## 2. What can be stood up and tested now

The dividing line is reserved values and design status. Anything that is pure mechanism with no reserved number and a settled design is buildable and testable today. Anything that needs a number the owner has not set, or a design that is still an open research item, is held.

| Layer | Status | Build now? | Test now? |
| --- | --- | --- | --- |
| Determinism core (fixed-point Q32.32 newtype; SplitMix64 counter-RNG; ECS scaffolding; append-only event log with never-reused StableId; the typed canonical-state boundary) | Settled mechanism, no reserved values | Yes | Yes, fully |
| Audited invariants (the determinism harness; the conservation and referential-integrity harness; the conserved-projection registry plumbing) | Settled mechanism, no reserved values | Yes | Yes, fully |
| Intra-tick coordination channel; the render or view boundary that quarantines float and the language model | Settled mechanism | Yes | Yes |
| Data schemas and loaders for resolved substrates (value substrate, semantic substrate, institution-function substrate, composition node with its interface-axis substrate, combinator registry, and emergent-proxy set, genome loci, language tables, trace and evidence and absence registries) | Settled schema, reserved content and numbers | Yes, the schema and loader and manifest plumbing | Round-trip and load tests yes; behaviour no |
| Resolved system mechanisms (belief and evidence, theory-of-mind structure, institutions, value-distance metric, genome and inheritance, language and meaning, tier consistency, recursive composition) | Settled mechanism, reserved calibrations | Yes, the mechanism, in staged dependency order | Structural and determinism tests yes; tuned behaviour waits on the numbers |
| The reserved calibration values themselves | Held for owner | No | No |
| Open research items (the eighteen in the backlog) | Not yet designed | No | No |

### 2a. Build and test today (the bedrock)

The determinism core is the foundation and carries no reserved numbers, so it is built and tested in full now: the fixed-point newtype with its arithmetic and property tests; the counter-based RNG keyed on the master seed, the entity id, and a phase, with tests that the same key yields the same draw on any machine and at any thread count; the event log with stable, never-reused identifiers; and the two standing harnesses, one proving that the same seed yields the same world bit-for-bit across platforms and thread counts, the other proving that promotion and demotion conserve every declared quantity and leave no dangling reference. These harnesses are themselves the project's correctness contract and should exist before any system is built on them.

The data schemas and loaders for the resolved substrates are also built now: the structures are settled, so the loader, the round-trip tests, and the calibration-manifest plumbing are all implementable. What is inert until the owner acts is the content (the specific axes, leaves, templates) and the numbers.

The resolved system mechanisms are implementable now in the staged order the design document lays out, each proven at small scale before it is trusted. The first proof to stand up is convergence without a target in the technology layer (one conceived intent, a part-based representation, fixed-point physics proxies, the per-culture search, two isolated cultures), confirming they reach the same physical attractor from different seeds before the transmission layer or deeper composition is added. The non-templatedness test (two seeds growing technology graphs of different shape under one physics) is the acceptance test for the composition layer once it exists.

### 2b. Held for the owner's calls

Held are: every reserved calibration value, which is surfaced with its basis and decided by the owner (Section 4); the eighteen open research items, which are not yet designed and are taken in the order the audit queue sets, several deliberately far-horizon (the four remaining deep-technology questions wait on the technology layer being built and proven at small scale; the most-wired items are taken last; the non-authoritative view elaboration waits on the level-of-detail model being built); and Inconsistency 5, the technique-origination disagreement, which is an owner decision the composition layer's promotion gate now leans on.

---

## 3. Order of operations for the first build

1. Stand up the repository: layout, license and notice, workspace, toolchain pin, `.gitignore`, the documents into `docs/`, the hooks and MCP config, `scripts/verify.sh`, and CI running the verification suite.
2. Build the determinism core and its tests: fixed-point newtype, counter-RNG, event log, the typed canonical-state boundary.
3. Build the two audited harnesses (determinism reproducibility; conservation and referential integrity) and run them in CI. Nothing else is trusted until these pass.
4. Build the substrate schemas, their loaders, round-trip tests, and the calibration-manifest plumbing, with every reserved value loading as a fail-loud sentinel.
5. Stand up the first small-scale proof (convergence without a target) and confirm it before adding transmission or composition depth.
6. From there, implement the resolved systems in staged order, surfacing each reserved value into the manifest as it is reached, and bring the owner the review queue when numbers are needed to tune.

---

## 4. Reserved values and tuneables: how they work, are surfaced, and are decided

This is the operational form of the prime directive that the project never fabricates a value. A mechanism is fixed and lives in code and the design document; the numbers it needs are the owner's, and until he sets them they are reserved, not guessed.

### 4a. The calibration manifest

Every reserved value is one entry in `calibration/reserved.toml`. The schema:

```toml
[[reserved]]
id      = "compose.max_depth"
basis   = "the per-tick budget and the depth at which marginal proxy gain falls below noise; a determinism-and-performance bound, not a realism one"
status  = "reserved"          # reserved | set
value   = ""                   # the owner's number; empty while reserved
unit    = "levels"
set_by  = ""                   # who set it, once set
set_date = ""                  # when, once set
source  = "Part 41 composition mechanism; record 62.10; audit section 1l"
```

One entry per value, the id namespaced by system (`compose.*`, `evidence.*`, `tier.*`, `value_metric.*`, and so on). The `basis` is copied from the design document's reserved list so the manifest and the document say the same thing. The `source` points back to the mechanism, so a reviewer can read the full context before deciding.

### 4b. How a value is surfaced

When a system is designed or built and needs a number, the agent does three things and stops: states the number as reserved in the `> Decided and reserved.` blockquote at the mechanism's site, with its basis; lists it in the audit consolidation block and the Part 62 record; and adds the entry to `calibration/reserved.toml`. Surfacing the number for review is the correct output. Inventing one is a defect. The basis is mandatory and concrete: it tells the owner the ground on which to choose, for example "the failure boundary the material and physics data already define, such as pressure exceeding yield", or "set equal to the drift and loss rates the transmission subsystem already uses, for consistency", so the decision is informed rather than arbitrary.

### 4c. How a value is decided and graduates

The owner reviews the entry through the reserved-values panel, which renders the manifest as a queue. He sets the value on the stated basis, then validates it by playtest or calibration against the target the basis names (for example "the population mean reconstructs the pool knowledge level within fixed-point tolerance", or "the aggregate undertaking rate matches the detailed tier's for comparable conditions", or "two seeds produce technology graphs of different shape"). Once set and validated, the entry's `status` becomes `set`, with `value`, `set_by`, and `set_date` filled, and the design document's reserved list is annotated to match. The manifest and the document never drift: a value is reserved in both or set in both.

### 4d. How the code treats a reserved value

The loader reads `calibration/reserved.toml` at startup. A `reserved` entry with an empty value loads as a sentinel, and any system that reads an unset required value fails loudly rather than running on a silent default. Nothing is hardcoded inline; every tuneable is a named manifest entry. Two build profiles follow from this: a development profile, in which a system whose required values are still reserved is gated off and its tests run only at the structural and determinism level; and a calibrated profile, in which the build refuses to start if any enabled system has a required value still reserved. Continuous integration lists every `status = "reserved"` entry as the standing review queue, so the set of decisions waiting on the owner is always visible.

### 4e. Why this discipline

The reserved-value mechanism is what lets the engine be both fully data-driven and openly incomplete. The mechanism is provable and testable now without the numbers; the numbers are the owner's calibration, surfaced with their basis and decided deliberately, never invented to make a thing appear finished. A number that is guessed to fill a gap is exactly the failure the project audits for; a number that is surfaced, reasoned, reviewed, set, and validated is the project working as intended.

## 5. The numerical-twin rule for analytic derivatives (owner-ruled, 2026-07-13)

Every analytic derivative in the substrate ships with its NUMERICAL TWIN in the test battery. It is the differential form of the g-factor lesson: a closed form is re-evaluated against an independent computation, never trusted because it was transcribed correctly. When a mechanism carries an analytic derivative (the Rose EOS curvature that recovers the bulk modulus is the first instance; the QEq hardness matrix, the elastic stiffness `C_ij` from strain, and the `dG/dT` entropies are the same shape as they are built), a test evaluates the quantity BOTH ways and requires agreement within a stated tolerance, so a sign slip or a unit error in the closed form fails the build.

The twin has its own hygiene, which the test must pin so the validator cannot pass on noise: use central differences, and sweep the step size to confirm the error sits in the second-order (`h`-squared) plateau. Too large a step leaves truncation error, too small a step leaves floating-point or fixed-point roundoff error; only the plateau in between measures the derivative, so the sweep proves the check is reading the derivative rather than one of the two error regimes. The Rose EOS twin already does this in spirit (a numerical second derivative of `E(V)` recovers the cited `B0`, which the analytic length scale was built from, so the twin exercises the whole unit chain); as the derivative-carrying mechanisms grow, each carries the same twin-plus-sweep.

Siting rule (learned on the rate-law kernel, 2026-07-13): a numerical-twin test that uses float (`to_f64_lossy` for the finite-difference arithmetic) lives in a test FILE, never inline in a canonical-path module, because the integer-only steering scan (`the_canonical_kernel_path_is_integer_only`) reads the whole `laws.rs` source, test module included, and rejects any `f32`/`f64` there. The two disciplines pair cleanly once the twin is sited right: the kernel PATH stays integer-only, and the twin's float lives in `crates/physics/tests/*.rs` where it is sanctioned. A twin whose finite differences can be formed in `Fixed` may stay inline, but the float form is usually clearer for the differentiation and belongs in the test file regardless. The rate-law twin (`crates/physics/tests/rate_law.rs`, recovering `d ln(rate)/d(1/T) = -E*/k_B`) is the first sited this way.

---

## 6. Tool verdicts are read through the machine interface, never by grepping their prose (owner-ruled, 2026-07-16)

A tool's verdict is consumed through its MACHINE INTERFACE: exit codes, deny lists, structured output. NEVER by pattern-matching its human-readable text. This is source-verbatim entry's sibling for tooling: the hand between a source and a struct has a cousin, the regex between a tool and a decision, and both get REMOVED rather than supervised.

The rule was earned. Six narrow-grep convictions landed in twenty-four hours, which is a class rather than a streak. The one that named the rule: a sweep of `rustdoc`'s broken-link findings was counted with `grep -c "unresolved link"`, and the count came back 149. The lint emits 151. Two findings (`field_step_kernel`, "private or doc(hidden)", and `promote`, "is both a function and a module") ARE `broken_intra_doc_links` firings that do not carry that phrase, so the grep missed exactly the cases whose wording was unusual, which is the population a grep is least able to enumerate and most likely to be asked about. Both would have blocked the gate.

So: deny the lint and read `$?`. Do not grep its output. The same applies to `cargo test` (read the exit code, not "test result: ok"), to `clippy` (`-D warnings` and the exit code, never a grep for "^error"), and to any gate script (an exit code, never a phrase in its report). A grep is a fine instrument for EXPLORING. It is never the instrument for DECIDING, because a tool's prose is prose, and prose greps are now six-time losers.

NEVER PIPE A VERDICT YOU INTEND TO READ (added 2026-07-17, the cloud agent's catch, one layer deeper than the grep rule). `cargo test --workspace 2>&1 | tail -N` reports TAIL'S exit code, not cargo's, because A PIPELINE'S EXIT STATUS IS ITS LAST STAGE'S. So a passing `tail` masked a failing `cargo`, and a false green almost reached a merge. A pipe is a grep with extra steps: it discards the machine interface just as thoroughly. Redirect and read `$?` (`cmd >/dev/null 2>&1; echo $?`), or consume the runner's own structured output, but the moment a verdict enters a pipe its exit code belongs to whatever ends the pipe. This is the same failure as the grep-of-diff and the grep-of-lint: the verdict was consumed through rendered text (`tail`'s output) while its real channel (cargo's exit code) was thrown away by the pipe.

AND TWO SCOPING TRAPS IN THE TEST COMMAND ITSELF, both of which HIDE FAILURES BY NOT RUNNING THEM. `--lib` runs only lib tests, so it SKIPS integration tests (`crates/*/tests/*.rs`) and doc tests; a binding test or an oracle test living in `tests/` is invisible to `cargo test --lib`. And `--workspace` versus plain `cargo test` is environment-dependent: `--workspace` builds EVERY crate including `civsim-gpu`, whose CUDA build-dependency is proxy-blocked in a network-restricted sandbox, so there `--workspace` fails to build and PLAIN `cargo test` (the 9 default-members, gpu excluded) is both the working command and the more complete one for the non-gpu crates. The lesson generalizes past this repo: THE MOST COMPLETE TEST COMMAND THE ENVIRONMENT CAN RUN is the gate, and every flag that narrows it (`--lib`, `-p CRATE`, a filter string) is a scoping that can hide a red crate the way a crate-scoped run hid the Fe row for a full session. Confirm what the command truly RAN (the printed per-crate `test result` lines), rather than only that it exited 0.

THE GATE MUST MIRROR EVERY CI JOB STEP, not the test step alone (added 2026-07-17, and it convicts this gate's own operator). CI here is "build, FORMAT, lint, test". A gate that runs build, lint, test, and the pins but NOT `cargo fmt --all --check` PASSES A COMMIT CI WILL REJECT: it happened, an agent's cherry-picked commit was fmt-dirty, I called it GATE PASSED because format was the one step my gate omitted, and the cloud agent independently found the identical gap in its own local gate the same hour. A GATE IS INCOMPLETE BY EXACTLY THE CHECKS IT DROPS RELATIVE TO CI. Enumerate CI's steps and run all of them. Note `cargo fmt --all --check` (VERIFY, read the exit code) is distinct from `cargo fmt --all` (REFORMAT): running the reformatter before a commit is why an author's own commits are clean, and it is exactly the step a cherry-pick of someone else's commit skips, so a gate over cherry-picked work must add the check back.

AND THE QUIETER LESSON FROM THE SAME HOUR: RUN A GATE CHECK IN A CLEAN TREE, and when a check contradicts what you expect, RE-VERIFY IN A CLEAN STATE BEFORE ACTING. A fmt check run inside a gate worktree that had been juggled through several checkouts and a stash reported a clean branch as dirty; the branch was clean, confirmed from the main checkout at exit 0. A phantom reformat of a non-problem was one step away, stopped only by `cargo fmt --all` producing an empty diff and then the clean-tree recheck. A CONTAMINATED WORKING TREE MAKES EVERY TOOL LIE IN UNISON: the machine interface is only as honest as the tree it reads.

The sibling failure, same day, same class: a search for a convective velocity ran `grep "fn .*(velocity|overturn|timescale)"` over the laws, read its output, and concluded no convective velocity existed. `laws::stokes_velocity` WAS IN THAT OUTPUT. It is the convective velocity, named for its physics rather than for its role. So the second half of the rule: SEARCH FOR THE PHYSICS NAME, NOT THE WORD YOU EXPECT, and when a search returns "absent", ask whether the thing is present under a name you did not think of.

---

## 7. Mutation testing on the tests that guard physics (owner-ruled, 2026-07-16)

Mutation testing runs STANDING on evaluator and gate tests, the tests that guard physics, as a PRE-MERGE TIER for every new evaluator. A test that has never been shown to FAIL has not been shown to test anything.

Two incidents on one day settled it, and neither was caught by review:

1. The ductile evaluator's parallel-property test ("the composite is weaker than either row alone") was VACUOUS: the property is true of the derived lower bracket BY CONSTRUCTION, so the test passed WITH THE BISECTION DELETED. Mutation said so; reading it did not.
2. Its first replacement re-summed the rows with `ln_sum_exp`, THE SAME FUNCTION UNDER TEST, so a max-only sum satisfied it. That is the self-comparing sentinel rebuilt by accident, on the same day the phantom E_coh sentinel was killed for being one.
3. The convective strain rate's BINDING TEST survived a mutant that dropped the magnitude and returned a signed rate, because every fixture had a rising flow. The binding was blind to the exact convention it existed to bind, and a signed rate breaks `tau = eta * eps_dot` for every sinking parcel.

THE TWIN-INDEPENDENCE RULE APPLIES TO TESTS EXPLICITLY. A test whose EXPECTED VALUE shares a route with the code under test asserts nothing. Pinned expectations come from OUTSIDE: the ductile evaluator's come from Hirth and Kohlstedt's own worked examples, which the primary computed by hand decades before this codebase existed, which is the perfect independent route. When no external number exists, pin the expectation from a root computed by a different construction (two rows sharing an intercept give `sigma^3.5 + sigma = 13.313708` at `sigma = 2`), and say in the test which route the expectation came from.

A SURVIVING MUTANT IS NOT AUTOMATICALLY A DEFECT. Some survive BY CONSTRUCTION and must be stated rather than chased: a 1-ULP mutant survives the reassociation binding test because THE BOUND IS THE RESIDUE, so a deviation below it is indistinguishable from the reassociation the test licenses. Report those in the blindness set. Chase the rest.

---

## 8. When a doc link breaks, ask whether the prose survived its target (owner-ruled, 2026-07-16)

THE TRIAGE QUESTION for every broken doc link, and it decides the repair:

- **Prose whose target merely MOVED gets a RELINK.** The mechanism is alive, the name changed. Point at the real item and prove the target (a rename verified at the commit that did it, never a plausible-looking neighbour).
- **Prose that OUTLIVED ITS MECHANISM gets a REWRITE, with a TOMBSTONE.** The doc describes something the project no longer does, and often something it DELETED ON PURPOSE.

The founding case for the second branch: `sim/material.rs` described the tool edge as derived from the worked stone's fracture strength UNDER THE BEING'S FORMING FORCE, over `laws::edge_area_at`. That function was purged in `8f34b31` because "at USE the same force cancelled ... a dependence on the maker with no physical basis at use time". A RELINK WOULD HAVE BEEN THE WORST REPAIR AVAILABLE: it would have pointed rejected reasoning at the new mechanism, so the doc would teach maker-dependence while linking to the intrinsic scale. Rewrite teaches the living mechanism; the tombstone keeps the purge rationale FINDABLE rather than re-derivable, because "an edge that remembers its maker's force is history where physics should be" is itself a keeper lesson, and the next person to reach for a formation dependence that cancels at use time should find it already answered.

The gate that surfaces these (`RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" cargo doc --workspace --no-deps --document-private-items`) runs WITH PRIVATE ITEMS IN SCOPE, because a gate blind to privacy modifiers audits the API surface rather than the codebase: the founding phantom (`ductile_strength_mpa`) fired only by the accident of being public, and twenty-four more broken links were hiding inside private items when the gate was first measured.

---

## 9. A mechanism may never calibrate against the value it retires (owner-ruled, 2026-07-16)

THE REPLACEMENT-CIRCULARITY RULE. When a derivation replaces a reserved value, it may NOT be calibrated or validated against that value. Replacements validate against THE RETIREE'S OWN SOURCES, or against INDEPENDENT data, and never against the retiree.

The founding case, caught by the agent building the replacement rather than by a reviewer: the disk-evolution arc derives `tau_disk` in order to RETIRE the reserved `disk_gas_lifetime_myr` that the #73 giant gate races today. Calibrating the derived clock against that reserved lifetime would have validated the replacement against the thing it replaces, and it would have passed, because a value agrees with itself.

THE REFUSED VALUE STILL HAS A HOME, and naming it is half the rule, because a value with no home gets wired in sideways. The Haisch-Lada-Lada and Mamajek disk-fraction-versus-age data behind that reserved lifetime is the POPULATION-LEVEL HINDCAST for the DERIVED `tau_disk` DISTRIBUTION once the dispersal race lands: an OUTPUT VALIDATOR ACROSS THE ENSEMBLE, never a calibration input. And NEVER ITS MEDIAN: a ~1 to 10 Myr range collapsed to a point is the statistic-with-a-hidden-conditioning-variable class this project has now convicted three times (the limiting isotherm's age convention, the elastic thickness's load and moduli, the disk lifetime's sample).

THE SIBLING PROHIBITION, ratified the same day: DO NOT REACH FOR THE NEAREST AVAILABLE NUMBER OF ROUGHLY THE RIGHT KIND. `FORMATION_TIME_MYR` (1 Myr, the oligarchic isolation-mass growth time) sits one file away from a disk arc needing a formation epoch, and it is a DIFFERENT PHYSICAL EVENT with a DIFFERENT CLOCK ZERO (when the embryo accreted, against when the 1 AU midplane cooled to the condensation front). Both are "early". That is not an identity. Proximity is not provenance.

THE PAYOFF, and the reason this rule is not merely a prohibition: refusing the circular calibration usually reveals that the quantity was DERIVABLE ALL ALONG. The same ruling dissolved two reserved epochs into physics. A basis line reading "the epoch the 1 AU midplane sits at that front" is an IMPLICIT EQUATION, `T_mid(1 AU, t) = T_condensation`, whose root IS the epoch, solvable on machinery the engine already runs. A basis line reading "the observed class-II value" names a SAMPLE, and samples have AGES, so the epoch is a fetchable chord endpoint rather than a reserved number. READ A BASIS LINE AS PHYSICS RATHER THAN AS PROSE, and the value it justifies often stops being a value.

AND THE DEGENERACY CAN SHARPEN WITH TIME, so a landmark refused today may referee tomorrow. The `0.19` formation rate was a PARTITION SHARE, never a measurement: its own basis says the 1400 K landmark fixes only the PRODUCT of rate, dust column, and opacity. That made it useless as a referee, since a hindcast on it passes on a compensating dust error and fails on a correct rate. But the engine has SINCE DERIVED two of those three factors through the composition wire and the opacity machinery, so the same physical condition has sharpened from an under-determined product into a genuine referee. When a landmark is refused for degeneracy, RECORD WHICH FACTORS WERE FREE, because deriving them is what promotes the constraint back to a gate.

---

## 10. The premise line binds BUILDERS, not designs (owner correction, 2026-07-17)

A correction earned by overapplying section 6's sibling. THE PREMISE LINE EXISTS TO STOP A BUILDER WIRING AN UNBUILT UPSTREAM. It does not exist to convict a design of being a design.

The distinction, and it is the whole rule: RULING PROSE AND EXPLORATION PROSE ARE WRITTEN AT THE SPEC LEVEL, and a spec is where a north star belongs. When a scoping brief says the engine "already carries" a thing that lives in the design documents, that is A RUNG TO PRICE, not a false claim to refute. The right output is A FLAG ON WHAT MUST BE BUILT, worded so a builder cannot mistake it for a shelf to reach for. The wrong output is "REFUTED AS BUILT", which is what this file's author wrote about a temporal-LoD north star that was never claimed as code.

SO THE TEST IS THE CONSEQUENCE, NOT THE TENSE. Ask what the claim is about to license:

- IF A BUILDER IS ABOUT TO WIRE IT, the premise line fires at full strength: verify in the code, and STOP if it is absent. Designed-exists does not imply built-exists, and this project has paid for that lesson repeatedly (an arbitration scheduled against a provider that never provided; a condition claimed realized by a function that existed nowhere but its own sentence; a value called "banked" that lived only in a contingency-vector spec).
- IF A DESIGN IS BEING PRICED, the same finding is a RUNG: name it, price it, say plainly that it is spec-real and code-absent, and move on. Nothing is being authored into being, so nothing needs convicting.

The failure mode this corrects is a real one and it has a cost: an agent that debunks its owner's exploration prose spends the channel's goodwill on a defect that was never a defect, and it trains the owner to write less freely, which is the opposite of what the check is for. VERIFY EVERY PREMISE; CONVICT ONLY THE ONES A BUILD WOULD REST ON.

---

## 11. A published value that fails to reproduce ships re-derived, with its erratum beside it (owner-ruled, 2026-07-17)

THE STANDARD. A published value that FAILS TO REPRODUCE FROM ITS OWN SOURCE'S DEFINITIONS ships as THE RE-DERIVED VALUE, with the erratum documented beside it. NEVER silently. NEVER from memory. ALWAYS from the source's own mathematics. Three conditions must ALL hold before the re-derivation ships:

1. THE RE-DERIVATION IS INDEPENDENTLY TWINNED. Two routes that share no code (two libraries, or a closed form against a numerical twin).
2. THE SURROUNDING CONTROLS VALIDATE. The same machinery reproduces the source's OTHER values, so THE METHOD STANDS WHILE THE CONSTANT FALLS. A re-derivation that cannot reproduce the source's controls is a bug in the reader, not an erratum in the source.
3. THE SLIP'S CAUSE IS NAMED OR MARKED UNATTRIBUTED. "Unattributed" is an honest finding; a guessed cause is a second error.

THE FOUNDING CASE. McNutt and Menard 1982 print `K(x_0) = -0.0289`. Their own printed definition applied to their own printed equation (A8) gives `0.0389`, confirmed through two independent libraries plus a finite-difference twin, WITH THE LINE-LOAD CONTROLS REPRODUCING TO 0.05 PERCENT. It propagates into their `C_2`, so their published seamount curvatures run about 26 PERCENT LOW. The cause is UNATTRIBUTED: no natural reading at `x_0` produces 0.0289. The re-derived constant ships; the 26 percent correction applies wherever their curvature-derived quantities enter a row; BOTH VALUES ARE CARRIED.

ITS SIBLING, RESOLVED THE SAME WAY BY THE SOURCE'S OWN INTERNAL CONSISTENCY: the primary prints `tau = 80 + 600 sigma_n (MPa)`, and three channels agree the page says 600, while the paper's OWN equations (7) and (8) back-solve to `mu ~ 0.6` and `S_0 = 80 MPa`, excluding `S_0 = 50` outright at 177 against a printed 283. THE SELF-CONSISTENT READING IS `tau = 80 + 0.6 sigma_n`: the published value lost a decimal point, and the source settles it against itself.

AND THE THIRD SHAPE, WHICH IS A UNIT RATHER THAN A DIGIT: Calmant's Table 1 prints `E = 10^12 N/m^2`, refuted by their own `D` range against their own Table 2. The value is right in dyn/cm^2. A printed unit is a claim like any other and gets checked against the source's own numbers.

AN UNEXPLAINED RESIDUAL SHIPS AS EXACTLY THAT. Their `C_1` carries an unexplained 2 percent, and it ships as a DECLARED RESIDUAL IN THE BAND, because AN UNEXPLAINED TWO PERCENT STATED IS HONEST AND AN UNEXPLAINED TWO PERCENT ABSORBED IS THE SILENT-PARAMETER CLASS.

WHY THIS IS NOT LICENCE TO EDIT SOURCES. The standard reads the source's OWN definitions against the source's OWN printed results, and the arbiter is always the source's internal consistency, never our preference, never a modern value, never a memory. Condition 2 is what keeps it honest: if the controls do not reproduce, the reader is wrong. This is the same instrument as the citation-coverage discipline pointed the other way: there, a value wearing a citation it could not cash was retired (the Fe row, cited to a CODATA table containing no iron); here, a value contradicting its own paper's mathematics is corrected. BOTH ASK ONE QUESTION: does the number keep the promise its source made about it?

AND THE COROLLARY THE Fe ROW PAID FOR: RE-VALUING IS INCOMPLETE UNTIL ITS PINS MOVE WITH IT, IN THE SAME COMMIT, FOR THE SAME CITED REASON. That commit named its live consumer in its own message and did not check it, so the suite asserted the retired value until a stranger's failing test surfaced it. NAMING A CONSUMER IS NOT CHECKING IT. And watch the quiet half: a sibling assertion pinned the Rose ratio at 2.895 with a tolerance of 0.01 while the correction moved it 0.0058, so THE TOLERANCE ABSORBED THE REPAIR and that test sat green throughout while its comment kept teaching the retired number. A tolerance wide enough to absorb a change is blind to it, which is legitimate for a physical band, and is exactly why THE COMMENT rather than the assertion is what the next reader inherits.

---

## 12. T_e is a verdict: the output is never fit, the input rarely, through one licensed channel (owner-ruled, 2026-07-17)

This is a constitutional section and the distinction it draws carries the whole constitution. It generalizes past `T_e` to EVERY DERIVED VERDICT the engine emits.

### The output is fit to observation NEVER

Constraining a verdict to match observation is AUTHORING THE OUTCOME, and it destroys the two things the engine exists to produce:

- INFORMATION. A derived `T_e` that misses the rows is the RESIDUAL LAW FIRING: a residual points at a defect, a dropped term, or a band that needs honest widening. Absorbing the miss by nudging until it lands DELETES THE SIGNAL that something upstream is wrong.
- THE ALIENS. The moment `T_e` is fit to Earth, every other world's elastic thickness becomes an extrapolation of an Earth fit WEARING A DERIVATION'S CLOTHES: Terran bias injected at the deepest structural level, invisible in every render forever after.

So the Mirror rule applies at full strength: THE COMPARISON IS RUN, THE MISS IS REPORTED, AND THE DERIVATION STAYS UNTOUCHED, exactly as the 41-planet assembly count was reported and never gated.

### The input is inverted from observation RARELY, and only third in line

There is a legitimate inversion channel, because it is WHAT THE FIELD ITSELF DOES: flexure observations are how humanity constrains lithospheric rheology at all, McNutt's program WAS an inversion, and the lab-to-field strength dispute exists precisely because field data measures something the lab bands only bracket. When a derivation INPUT carries a DECLARED IGNORANCE BAND from independent sources (the `V*` spread, the 418-to-530 effective-Q gap), the hindcast can act as a MEASUREMENT of that input. INFERENCE WITHIN DECLARED IGNORANCE IS NOT TUNING.

But the channel is THIRD IN LINE, and the order is load-bearing:

1. FIRST, TREAT THE MISS AS A DEFECT HUNT, because this project's entire history says the miss usually IS one: a dropped term, a units frame, a chord variable.
2. SECOND, NARROW THE BAND BY MORE INDEPENDENT MEASUREMENT. The conditioning-fields filter (the deep `V*` band's covering-set selection over matching-condition determinations) is exactly this: honest narrowing with NO observation spent.
3. ONLY WHEN THE BAND IS IRREDUCIBLY WIDE AND THE MISS SURVIVES THE HUNT does inversion earn its turn.

### The six conditions on the licensed inversion, all load-bearing

1. PRIOR BAND, FIT STRICTLY WITHIN. The parameter must have a prior band from lab or theory, and the fit selects strictly inside it. A FIT DEMANDING A VALUE OUTSIDE THE BAND IS NOT A CALIBRATION, IT IS THE CHAIN SURFACING A DEFECT (the in-population rule from the MMSN pin, generalized).
2. SEPARATE TAGGED ENTRY, LAB ROWS NEVER EDITED. The inferred value is a separate, tagged, field-calibrated entry; the lab rows stand untouched (modality discipline), so the two knowledge sources stay distinguishable forever.
3. THE CONSTRAINT FLOWS TO THE INPUT AND ONLY THE INPUT. The engine may store "effective `V*` is Y with band, inferred from row Z" and MAY NEVER STORE "`T_e` should be X". Every world's `T_e` is re-derived through unchanged machinery.
4. THE PROVENANCE DAG MARKS THE INFERENCE. Any consumer of any world's `T_e` can see the Earth-inferred parameter in its ancestry, and THE PURE-LAB-BAND DERIVATION REMAINS COMPUTABLE IN PARALLEL for anyone who wants the uninfected answer. (Verified as built: `sim/calibration.rs` carries a provenance tag per value and its derives-from ids.)
5. MIRROR-CLASS INSTANCE ONLY. The inversion is licensed only on Earth, because Earth is the one world whose OTHER inputs are independently pinned, making the inversion well-posed with ONE unknown. On an alien row it would be a degenerate fit of everything at once.
6. A ROW SPENT ON CALIBRATION IS SPENT. It can NEVER again be claimed as validation, so the partition is declared BEFORE the fit, preregistered-holdout style: Earth's seamounts calibrate if anything does, Mars and Venus stay OUT-OF-SAMPLE, and the engine's predictive claims rest only on rows it never touched.

### The institution: the calibration ledger

Against creeping calibration (one licensed inference per year until the engine is secretly a fit of Earth with derivation theater), every field-inferred parameter is registered in `docs/working/CALIBRATION_LEDGER.md` with its spent row, its band, and the owner's signature, the same ceremony as a re-pin window, published in the audit and eventually the paper. TWO INVARIANTS RIDE IT: the validation set STRICTLY EXCEEDS the calibration set at all times, and the ledger's LENGTH is a reported honesty metric. IF THAT LIST EVER NEEDS A SECOND PAGE, THE ANSWER HAS QUIETLY BECOME NO, and the ledger is what makes the quiet loud.

The ledger is empty today, which is the state to preserve.
