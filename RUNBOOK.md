# Runbook: Canonical Planet Development

This runbook is the short operational entry point for the abiotic planet and
stellar-system pipeline. The retired civilization runbook is preserved at
`parked/RUNBOOK_LEGACY.md`.

## 1. Start a session

Read `CLAUDE.md`, `AGENTIC_ADDENDUM.md`, the top entry in `HANDOFFS.md`,
the canonical section of `TODOS.md`, and all of
`docs/working/CONSENSUS_ROADMAP.md`.

On Windows:

```powershell
pwsh -NoProfile -File scripts/dev.ps1 doctor
```

On Linux or WSL:

```sh
just doctor
```

Record inherited failures separately from new findings. Do not baseline a new
failure merely because an older branch carried it.

## 2. Value and state boundary

The sealed absolute physics floor is the only canonical value-bearing input.
The four tiers and seven provenance marks are accounting, not admission.

For every required magnitude:

1. search the complete derivation substrate;
2. derive it from admitted ancestry when possible;
3. if every derivation fails, prepare the complete exhaustion receipt,
   Buckingham-Pi budget, Gap Law evidence with a typed Chaos Protocol branch,
   Residual Law evidence, and unique residual slot for the non-derived floor
   candidate, regardless of tier;
4. refuse the stage when that admission is absent or incomplete.

Calibration manifests, profiles, scenarios, caller-authored world rows,
arbitrary cited inputs, written state, and contingency do not enter the
canonical path. Written state and contingency are generated. No caller seed or
identity enters; an internal realization cannot exist until its physical
measure exists.

The root workspace also retains tested abiotic candidate mechanisms. Their
public raw parameter APIs are not canonical admission. Canonical stage source
may reach physics, materials, world, or private pre-migration planet mechanisms
only through a dedicated typed adapter that binds every magnitude to its floor
ancestry or returns a structured refusal. The separate `parked/` workspace owns
biology, civilization, authored world generation, and causal presentation.

## 3. Build and test

The shared commands are:

```sh
just run-derived
just readiness
just ledger-inventory-check
just fmt-check
just lint
just test
just check-pr
```

`just run-derived` is expected to enter Stage 1 and return a structured refusal
until the stellar-birth realization measure derives or is admitted and Stage 1
is wired. `just readiness` inspects the no-floor boundary and therefore returns
the separate `absolute_floor_required` refusal. A refusal is a truthful result,
not a completed-world pin.

Use explicit parked commands only for compatibility:

```sh
just check-legacy
just audit-parked
```

Neither command supplies canonical evidence.

## 4. Change discipline

Read the relevant mechanism record before editing. Preserve determinism,
conservation, refusal identity, provenance ancestry, and admit-the-alien.
Viewer code may consume only immutable snapshots and receipts.

Every physical slice needs focused tests for:

- repeated-run bit identity;
- stage reachability and downstream non-reachability;
- structured refusals;
- conservation and residuals;
- perturbation sensitivity;
- provenance and derivation ancestry;
- an adversarial non-Earth system.

Formula changes, ownership moves, and file splits are separate slices. Split
by invariant only after direct goldens exist.

Test constants are classified by role. A fixture is arbitrary and must remain
labelled and physically non-authoritative. An oracle point is frozen together
with its independently computed expectation until the mapping changes. A
physical claim belongs in hindcast or validation machinery, never in a unit-test
literal. Sensitivity claims name the variables held fixed.

## 5. Research and vendoring

Before a fetch, read `AGENTIC_ADDENDUM.md` section 12 and
`docs/working/VENDORING_CHECKLIST.md`. Check licence and paywall status
first. Hold open bytes with SHA256 and exact anchors; use the restricted
citation-plus-witness route where required.

Vendoring closes evidence custody only. It does not grant canonical admission.
A fetched number must still derive or pass the complete floor-admission
process. A relayed number is a fetch-spec seed and may not ride canonical code
as a provisional.

## 6. Finish a session

Update `HANDOFFS.md`, `TODOS.md`, and only the roadmap lines moved by the
work. Run the exact Stop hook:

```powershell
pwsh -NoProfile -File scripts/dev.ps1 stop-gate
```

or:

```sh
just stop-gate
```

Do not override a failing gate. Resolve every in-scope failure or report the
specific owner decision that blocks it.
