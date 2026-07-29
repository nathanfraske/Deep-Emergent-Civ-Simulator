# Codex operating instructions

`CLAUDE.md`, `AGENTIC_ADDENDUM.md`, and `.claude/settings.json` are the
authoritative project protocol. This file makes that protocol discoverable to
Codex. If this summary and the authoritative files differ, follow the
authoritative files.

## Start of every session

1. Read `CLAUDE.md` and `AGENTIC_ADDENDUM.md` in full.
2. Read the current top entry in `HANDOFFS.md`, the open queue in `TODOS.md`,
   and all of `docs/working/CONSENSUS_ROADMAP.md`.
3. Run the baseline verifier. On Windows use
   `pwsh -NoProfile -File scripts/dev.ps1 doctor`. On Linux or WSL use
   `just doctor`.
4. Treat a failing baseline as inherited state and keep it distinct from new
   findings.

## Project rules that bind every change

- The owner decides contested design calls, research scope, and research
  resolution. No owner choice admits an arbitrary canonical magnitude.
- Verify every agent or reviewer finding against the source before trusting it.
- Canonical abiotic consumers accept value-bearing inputs only from the sealed
  absolute physics floor after derive-first exhaustion and floor admission.
  Provenance types and tier labels are accounting only; they do not admit a
  value. Calibration manifests, profiles, caller-authored world rows, arbitrary
  cited inputs, written state, and contingency do not enter the canonical path.
  Written state and contingency are generated, and an unsupported value
  refuses. Read `docs/working/PHYSICS_FLOOR_REGISTRY.md` before any value or
  law proposal.
- Read the relevant canonical mechanism record before changing a mechanism.
  The parked civilization design is historical evidence only. Do not infer the
  contract from tests, comments, or memory.
- Preserve determinism, refusal behavior, byte receipts, provenance, and the
  eleven design principles.
- The abiotic planetary pipeline is the canonical main runpath. Biology,
  civilization, and dawn fixtures are legacy, non-blocking work. Do not use a
  legacy fixture or workspace result as planetary evidence.
- Follow the prose customs in `CLAUDE.md` in maintained prose and chat replies.

## Research and vendoring

Before any fetch subagent is started, its prompt must include
`AGENTIC_ADDENDUM.md` section 12 and
`docs/working/VENDORING_CHECKLIST.md`. Check licence and paywall status first.
Openly redistributable sources require held bytes, a 64-hex SHA256, custody,
licence, slimming record, exact anchors, source-stated uncertainty or band, and
scope. Restricted material uses citation plus witness with a resolving archive
URL, licence reason, and exact extract. Read primary figures and tables. Omit a
value that cannot be sourced. A relayed number is only a fetch-spec seed.
Evidence custody never grants canonical admission; the value must still derive
or pass the complete absolute-floor admission process.

## Agents and panels

Before an audit, review, lens, blind, or adversarial agent panel, read
`AGENTIC_ADDENDUM.md` sections 7 through 11. Every load-bearing panel first
needs the strongest-model input-bias smoke test, which fails closed. Every
world-content audit needs independent correctness panelists plus all five
section 9 lenses: confirmation bias, derive versus author, alien feasibility,
Terran bias, and steering and Principles. Verify each finding against source.

## Shell, edits, and completion

- Never pipe a verifier into a filter unless `pipefail` or the originating
  status is preserved. Capture and report the real exit code.
- After each edit to `parked/docs/design.md` or `parked/docs/audit.md`, run the
  legacy archive customs and fence checks required by `CLAUDE.md`.
- Use the shared entrypoints in `justfile`. On Windows, use `scripts/dev.ps1`;
  `Makefile` is a thin alias layer. Unqualified planet commands fail closed
  until their canonical targets exist; old commands carry a `legacy` suffix.
- Keep `HANDOFFS.md`, `TODOS.md`, and the lean consensus roadmap current as
  required by the session ritual and Stop gate.
- Before the final response, let the Codex Stop hook run. If hooks are not
  loaded, run `pwsh -NoProfile -File scripts/dev.ps1 stop-gate` on Windows or
  `just stop-gate` on Linux or WSL, and continue working on any failure.
