#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# SessionStart hook (AGENTIC_ADDENDUM.md section 2a). Injects the tail of
# HANDOFFS.md, the open TODOS.md items, the verification baseline, and the
# consensus-roadmap frontier as additionalContext, so the agent begins knowing the
# current state and whether the documents are already clean. If the baseline is
# dirty it says so loudly.

set -u
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT" || exit 0

verify_out="$(bash scripts/verify.sh 2>&1)"
# Bound each block on its own so all sections survive the overall cap below: the
# HANDOFFS entries and the TODOS bullets are long, and left unbounded either one alone
# fills the budget and silently drops the sections after it. The top HANDOFFS entry is
# the priority (recover state), so it gets the largest share.
handoff="$(sed -n '1,60p' HANDOFFS.md 2>/dev/null | head -c 3500)"
todos="$(grep '^- \*\*R-' TODOS.md 2>/dev/null | head -40 | head -c 2500)"

# The consensus roadmap is the ground-truth order of work. Inject its section
# headers and near-term frontier so the agent starts oriented and remembers to keep
# it current. Bounded so the whole context stays under the additionalContext limit.
roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  roadmap="Sections:
$(grep '^## ' "$roadmap_file" 2>/dev/null)

Near-term frontier (Tier B):
$(sed -n '/^## Tier B/,/^## Tier C/p' "$roadmap_file" 2>/dev/null | head -28)"
else
  roadmap="(not present; create it when the ground-truth order is next reviewed)"
fi

# The roadmap block is bounded on its own so it survives the overall cap below,
# since the HANDOFFS and TODOS blocks can be large.
roadmap="$(printf '%s' "$roadmap" | head -c 2000)"

ctx="$(cat <<EOF
Session memory loaded by the SessionStart hook.

Method available: the fully-blind audit (AGENTIC_ADDENDUM.md section 7). When a correctness or reserved-value verdict must not be contaminated by the repo's own tests, comments, or prior reviews, assemble a scratchpad packet (substrate contract + code + declared spec, no tests or docs) and run repo-walled independent auditors; pilot one agent for sufficiency first, then verify every finding against the source. Panelists on the cheapest model that fits (Sonnet; Haiku for mass; Opus for the hardest cases).

=== docs/working/CONSENSUS_ROADMAP.md (the ground-truth order of work; review it at the start, and as work is done keep it current: cite each item's gate as an R-item, a Part number, or a file, and tombstone a completed or removed item by rewriting it in place, never by deleting it silently) ===
$roadmap

=== HANDOFFS.md (read the top entry first to recover state) ===
$handoff

=== TODOS.md (open research items) ===
$todos

=== verification baseline (scripts/verify.sh) ===
$verify_out
EOF
)"

# Cap at well under the 10,000-character additionalContext limit.
ctx="$(printf '%s' "$ctx" | head -c 9500)"

python3 - "$ctx" <<'PY'
import json, sys
print(json.dumps({
    "hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": sys.argv[1],
    }
}))
PY
exit 0
