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
