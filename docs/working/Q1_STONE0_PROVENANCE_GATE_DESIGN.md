# Q1 Stone 0: the local-firing provenance gate, human-password override

This is the design-first opener for Stone 0, the owner-directed enforcement arc that converts "never fabricate a value" from a discipline a builder must remember, and a CI check that has been missed, into a rule that fires on the everyday local command an agent runs (`cargo build`, `cargo check`, `cargo test`, `cargo clippy`) and cannot be worked around without the owner. It is numbered Stone 0 rather than Stone 4 because it sits UNDER the other Stones: Stone 1 (the sealed `Fixed` constructor), Stone 2 (observation-schedule invariance), and Stone 3 (the conserved-quantity ledger) each enforce one principle, and Stone 0 is the meta-gate that makes the no-fabricated-values discipline itself un-bypassable at the local inner loop.

## Why it exists (the motivating miss)

A mantle-column depth reached the deep-time code as `Fixed::ONE`, a placeholder fixture, and it compiled clean. The constructor gate did not catch it (the value was inside a `#[cfg(test)]` fixture and under the audited baseline), the provenance gate did not catch it (it was not a `calibration/reserved.toml` entry), and the run path had not yet read it. The owner's ruling: "That should not have compiled without a fail loud somewhere in the stack, and loudly flagged if it ever hit runpath." A CI-only backstop is not enough, because CI has been missed before and is not a hard gate on the inner loop. The gate must fire where the work happens.

## What Stone 0 is (the mechanism)

Three parts, settled by the owner's steer, plus one part still out to the owner-proxy consult.

### Part A: the gate fires on every local cargo command (settled)

A small workspace crate `stone0` exposes a `run_gate()` entry point intended for build scripts. Each gated crate (the world-content crates: `sim`, `world`, `materials`, `physics`) carries a three-line `build.rs` that calls `stone0::run_gate()`, with `stone0` as a build-dependency. Because `cargo build`, `cargo check`, `cargo test`, and `cargo clippy` all compile and run a crate's build script, the gate runs on every one of those, for both the workspace form and the `-p <crate>` form. A finding makes the build script fail, so the cargo command fails with a formatted report and the agent is blocked. `rerun-if-changed` is emitted for every scanned source directory, the calibration manifest, and the gate policy, so the scan reruns exactly when the content it guards changes and is otherwise cached (the inner loop stays fast).

The scan itself is the existing provenance discipline, consolidated: the fixture and un-provenanced-value detection that `scripts/constructor_gate.py` and `scripts/provenance_gate.py` already encode, extended to catch the class that slipped through (a placeholder numeric literal standing in for a reserved material or physical value, in dormant or test-fixture code that is on its way to the run path). The scan is the single source of truth, run identically here and in CI.

### Part B: human-password override, checked against the local secrets directory (settled)

When an agent is legitimately blocked (a false positive, or a value the owner has judged acceptable to proceed on), only the owner can authorize the build to proceed. The override credential is a single memorable password the owner keeps in their head, stored in exactly one place: `/mnt/e/Secrets/` (the local, out-of-repo, untracked secrets directory the project already uses, alongside `cec-bot.env` and `cec-sudo.env`), in a file `stone0-override.pass`. The password does not persist anywhere else.

The flow: a blocked agent must obtain the password from the owner out of band (the owner types it to them). The agent supplies it ephemerally for a single command, as the environment variable `STONE0_OVERRIDE`. The gate reads `/mnt/e/Secrets/stone0-override.pass` and compares; on a match it lets that one build proceed and prints a loud OVERRIDDEN notice; on a mismatch or an absent secrets file it fails. The password is never written into the repository, and the override is per-invocation, not a persistent disable. The agent is never given the password by the gate and cannot read the secrets file's contents through any gate output.

The owner creates the password file; the agent (me) never writes it, never asks for its contents, and never persists it (the same standing rule as the sudo password). This document specifies the path and format; the value is the owner's alone.

### Part C: laundering detection (resolved by the owner-proxy consult: rotate-on-grant plus a tombstone list)

The threat: an agent, once given the password, saves it somewhere in the repository so it can reuse the override later without asking the owner again. The Fable 5 Max consult (repo-blind, run `wf_e527bc39`) established the hard result and the resolution.

The hard result, stated plainly: as literally posed the requirement cannot be met. For cloud CI to detect the LIVE password it must evaluate a predicate that carries password-distinguishing information, and on a hosted runner everything the detection job can reach is agent-reachable (a push-triggered workflow runs the workflow file from the pushed ref with that job's secret context), so any reachable recognizer of a low-entropy memorable password is an offline brute-force oracle. There are exactly two concessions: bring the check to the machine that holds the secret, or change the secret's lifecycle so what CI greps for is no longer secret. The consult recommends the second, and it is what this design adopts.

The resolution, rotate-on-grant with a committed tombstone list, has these parts:

The LOCAL detector (un-brute-forceable, nothing committed). On every build the gate already reads the secrets file, so it also greps the working tree, the index, and, from a pre-push hook, the pushed commit range, for the literal LIVE password and its base64 (the laziest encoding). A hit hard-fails with a distinct "laundered credential" report, independent of any override. There is no offline brute-force surface because nothing derived from the password is committed.

Rotate on grant (the declassification move). When the owner grants an override, then in the same sitting the owner retires that phrase and memorizes a fresh unrelated one, a one-line edit to the single secrets file. Retirement DECLASSIFIES: a retired password opens nothing, so it may persist anywhere, which means the retired phrase is appended to a plaintext TOMBSTONE list committed in the repo (`calibration/stone0_tombstones.txt`). Any copy an agent laundered while the phrase was live goes stale the moment it is retired.

The CI detector (now trivial, no cloud secret). Cloud CI greps the pushed range for every tombstone on every run, and greps full history whenever the tombstone file itself changes. A laundering committed while the password was live is caught retroactively on the first run after that phrase is retired, and CI halts for the owner. Nothing secret lives in the cloud: CI only ever greps for already-public retired phrases.

The garnish (cheap supplements, adopted, never the whole answer). The gate greps committed files for the override environment-variable NAME being set (catches baking the override into a script without knowing the value), and a canary line planted in the secrets file fails the gate if it ever appears in the repo (proof an agent read the file directly).

The named concession, stated plainly: cloud CI can never see laundering of the CURRENTLY live password. That blindness is bounded by the grant-to-rotation window (minutes, and covered locally by the next build) and is self-healing at rotation. If the owner ever wants that window closed against a direct push that skips local builds, the single least-bad addition is a read-only WATCHER on a machine that can read the secrets file (a cron that pulls on push, greps the new commits without executing them, and posts a required commit status cloud CI blocks on); it is optional standing infrastructure, not part of the core.

### Part D: the integrity anchor (the cheaper attack is neutering the gate, not laundering)

The consult flagged that an agent with write access has a cheaper move than laundering the password: neuter the gate itself (an early return, a dropped build-dependency). No local mechanism closes this, so the anchor lives outside agent reach: platform branch protection with required status checks (agents lack admin), a `CODEOWNERS` entry making the owner the required reviewer on the gate, the workflow, and the tombstone paths, and the owner's own eyes on any diff that touches them. A second surface the consult named: the password, once told to an agent, also survives in that agent's session transcript and shell history on disk, which no repo scanner can reach; rotation is what actually delivers "cannot reuse without asking," because it staleness-kills the transcript copy too.

## The honest limits

Local checks are tamper-evident, not tamper-proof: an agent with write access can edit a `build.rs` or drop the build-dependency, and that is a tracked change the owner sees in the diff and that CI and the CODEOWNERS gate catch. Literal-string scanning catches only verbatim storage; base64 is covered, but a split across two files or a stronger encoding defeats it, so detection is a tripwire for banal convenience-driven persistence (the realistic agent failure), never a boundary against deliberate obfuscation. The gate catches the fixture CLASS the scan encodes; a fabricated value dressed as a legitimate engine-mechanics constant (the Stone 1 semantic problem) is not separable syntactically and stays the province of the derivation-hunter audit, not this gate.

## Build order

1. The `civsim-stone0` crate and its `gate()`: the provenance-scan invocation (the existing gates as the violation predicate), the `STONE0_OVERRIDE` check against the secrets file, the native local laundering scan (worktree, index, plus the tombstone and override-env-name and canary greps), the content-hash cache, and the `rerun-if-changed` emissions. Fail-OPEN on any operational error (missing python, absent secrets dir), fail-CLOSED only on a positive detection, so a gate bug never bricks the build. With tests. Runs as a bin first, not yet firing on build. (Parts A, B, C-local, D-scan.)
2. `calibration/stone0_tombstones.txt` (empty, headed), the pre-push range hook, and the CI step that greps the pushed range for every tombstone (and full history when the tombstone file changes). Prove it catches a planted tombstone and has no false positive on the current tree. (Part C-CI.)
3. `.github/CODEOWNERS` on the gate, workflow, and tombstone paths, plus the owner-ritual and branch-protection documentation. (Part D anchor.)
4. Wire `build.rs` into the gated crates, verify the byte pins still compute and the inner-loop cost is acceptable. This is the invasive step that makes the gate fire at clippy-time; it lands last, once the scan is proven false-positive-free. (Part A firing.)
