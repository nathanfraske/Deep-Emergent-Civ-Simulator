#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# SessionStart hook (AGENTIC_ADDENDUM.md section 2a). Injects the tail of
# HANDOFFS.md, the open TODOS.md items, and the verification baseline as
# additionalContext, so the agent begins knowing the current state and whether the
# documents are already clean. If the baseline is dirty it says so loudly.

set -u
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT" || exit 0

verify_out="$(bash scripts/verify.sh 2>&1)"
handoff="$(sed -n '1,60p' HANDOFFS.md 2>/dev/null)"
todos="$(grep '^- \*\*R-' TODOS.md 2>/dev/null | head -40)"

ctx="$(cat <<EOF
Session memory loaded by the SessionStart hook.

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
