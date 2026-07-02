#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# SessionStart hook (AGENTIC_ADDENDUM.md section 2a). Injects the fully-blind-audit
# method pointer, the consensus-roadmap frontier, the tail of HANDOFFS.md, the open
# TODOS.md items, and the verification baseline as additionalContext, so the agent begins
# knowing the current state and whether the documents are already clean.
#
# Truncation discipline (the failures a fully-blind audit of this hook found, section 7):
# every section is clipped to its OWN budget, including the verification output, so no one
# block can starve the others; each clip happens once, in Python, on a UTF-8 codepoint
# boundary (never mid-character, so a split multi-byte char can never corrupt the JSON
# payload); and every clip leaves an explicit ...[truncated] marker, so a section is never
# SILENTLY cut and the agent always knows to read the file for the rest. If Python fails
# for any reason the hook emits a minimal but VALID JSON object rather than nothing.

set -u
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT" || exit 0

# Gather each block RAW (no byte truncation here; Python does the codepoint-safe clip).
verify_out="$(bash scripts/verify.sh 2>&1)"
handoff="$(sed -n '1,120p' HANDOFFS.md 2>/dev/null)"
todos="$(grep '^- \*\*R-' TODOS.md 2>/dev/null | head -40)"

roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  # The header grep is bounded (head -20) so a roadmap that grows many sections cannot
  # crowd out the Tier-B frontier that follows.
  roadmap="Sections:
$(grep '^## ' "$roadmap_file" 2>/dev/null | head -20)

Near-term frontier (Tier B):
$(sed -n '/^## Tier B/,/^## Tier C/p' "$roadmap_file" 2>/dev/null | head -28)"
else
  roadmap="(not present; create it when the ground-truth order is next reviewed)"
fi

method="Method available: the fully-blind audit (AGENTIC_ADDENDUM.md section 7). When a correctness or reserved-value verdict must not be contaminated by the repo's own tests, comments, or prior reviews, assemble a scratchpad packet (substrate contract + code + declared spec, no tests or docs) and run repo-walled independent auditors; pilot one agent for sufficiency first, then verify every finding against the source. Panelists on the cheapest model that fits (Sonnet; Haiku for mass; Opus for the hardest cases)."

# The raw sections are passed by environment (never re-expanded, so quotes/backticks in the
# content are inert) and clipped once in Python on codepoint boundaries.
export SS_METHOD="$method" SS_ROADMAP="$roadmap" SS_HANDOFF="$handoff" SS_TODOS="$todos" SS_VERIFY="$verify_out"

out="$(python3 <<'PY'
import json, os
LIMIT = 9500  # safety margin under the harness's 10000-character additionalContext cap
# Per-section budgets: each block is clipped on its own so none can starve another. The
# top HANDOFFS entry is the priority (recover state), so it gets the largest share.
BUDGETS = {"method": 600, "roadmap": 1500, "handoff": 4000, "todos": 1600, "verify": 800}

def clip(s, n):
    s = s or ""
    if len(s) <= n:
        return s
    return s[:n].rstrip() + "\n...[truncated %d chars; read the file for the rest]" % (len(s) - n)

sec = {k: clip(os.environ.get("SS_" + k.upper(), ""), b) for k, b in BUDGETS.items()}
ctx = (
    "Session memory loaded by the SessionStart hook.\n\n"
    + sec["method"] + "\n\n"
    + "=== docs/working/CONSENSUS_ROADMAP.md (the ground-truth order of work; keep it current) ===\n"
    + sec["roadmap"] + "\n\n"
    + "=== HANDOFFS.md (read the top entry first to recover state) ===\n" + sec["handoff"] + "\n\n"
    + "=== TODOS.md (open research items) ===\n" + sec["todos"] + "\n\n"
    + "=== verification baseline (scripts/verify.sh) ===\n" + sec["verify"]
)
if len(ctx) > LIMIT:
    ctx = ctx[:LIMIT].rstrip() + "\n...[truncated to the context cap]"
print(json.dumps({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": ctx}}))
PY
)"

# Guard the Python step: on any failure (including a crash on unusual content) emit a
# minimal valid object, never nothing and never malformed JSON.
if [ $? -ne 0 ] || [ -z "$out" ]; then
  printf '%s\n' '{"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": "SessionStart hook: context assembly failed; read HANDOFFS.md and TODOS.md and run scripts/verify.sh manually."}}'
  exit 0
fi
printf '%s\n' "$out"
exit 0
