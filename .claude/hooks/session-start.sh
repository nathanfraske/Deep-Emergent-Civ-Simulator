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

# Gather each block raw (no byte truncation here; Python does the codepoint-safe
# clip). Preserve the verifier's status before reading its bounded output. The
# former pipeline through `head` discarded a failing verifier status.
verify_tmp="$(mktemp "${TMPDIR:-/tmp}/civsim-session-verify.XXXXXX" 2>/dev/null || true)"
if [ -z "$verify_tmp" ]; then
  verify_out="STATUS: FAILED TO ALLOCATE VERIFICATION OUTPUT"
elif bash scripts/verify.sh >"$verify_tmp" 2>&1; then
  verify_out="STATUS: CLEAN
$(head -c 6000 "$verify_tmp")"
else
  verify_status=$?
  verify_out="STATUS: DIRTY (exit $verify_status)
$(head -c 6000 "$verify_tmp")"
fi
[ -n "$verify_tmp" ] && rm -f "$verify_tmp"
handoff="$(sed -n '1,120p' HANDOFFS.md 2>/dev/null | head -c 16000)"
if ! todos="$(python3 - "$ROOT/TODOS.md" <<'PY'
import re, sys

text = open(sys.argv[1], encoding="utf-8").read()
match = re.search(
    r"^## Canonical abiotic queue\s*$\n(.*?)(?=^## |\Z)",
    text,
    re.M | re.S,
)
if not match:
    raise SystemExit("missing bounded canonical abiotic queue")
pattern = re.compile(r"^- \*\*(P-[A-Z0-9-]+)\.\*\*\s*(.*)$")
items = [(m.group(1), line) for line in match.group(1).splitlines() if (m := pattern.match(line))]
ids = [item for item, _line in items]
duplicates = sorted({item for item in ids if ids.count(item) > 1})
print("CANONICAL QUEUE: items=%d, duplicates=%s" % (len(items), duplicates or "none"))
for _item, line in items:
    print(line)
PY
)"; then
  todos="CANONICAL QUEUE: FAILED TO PARSE TODOS.md; read the file before working"
fi

roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  # The roadmap is now the lean FEATURE STATUS BOARD and nothing else (the deeper reconciliation
  # and history live in parked/docs/working/ROADMAP_REFERENCE.md). Surface the whole board on every session start so
  # it is READ first; the Stop gate enforces editing it IN PLACE when a segment makes progress, so
  # it never goes stale. The Python step below clips it to the roadmap budget on a codepoint
  # boundary with a truncation marker, so the agent knows to read the file for the rest.
  # Bound the raw byte size BEFORE export: the env var is passed to python3, and an unbounded
  # roadmap (its top line alone runs tens of thousands of characters) overflows ARG_MAX so python3
  # never starts and the hook falls back silently. head -c caps the bytes; Python then does the
  # codepoint-safe clip to the roadmap budget, well inside this cap, so a split trailing byte is
  # discarded by that clip and never reaches the output.
  roadmap="$(head -c 16000 "$roadmap_file" 2>/dev/null)"
else
  roadmap="(not present; create it when the ground-truth order is next reviewed)"
fi

method="Method available: the fully-blind audit (AGENTIC_ADDENDUM.md section 7). When a correctness or floor-admission verdict must not be contaminated by the repo's own tests, comments, or prior reviews, assemble a scratchpad packet (substrate contract + code + declared spec, no tests or docs) and run repo-walled independent auditors; pilot one agent for sufficiency first, then verify every finding against the source. Panelists on the cheapest model that fits (Sonnet; Haiku for mass; Opus for the hardest cases)."

# The raw sections are passed by environment (never re-expanded, so quotes/backticks in the
# content are inert) and clipped once in Python on codepoint boundaries.
fetch="Fetch discipline, a STANDING RULE read at session start (AGENTIC_ADDENDUM.md section 12): if you produce a load-bearing number you MUST vendor its source at fetch time. Download the document or data file, SHA256 it, hold it behind the manifest (docs/working/VENDORING_CHECKLIST.md); a URL citation is NOT provenance. Read primary figures and tables, and carry each value's anchor, dual-channel agreement where required, and scope. Evidence custody is not canonical admission: the candidate must still derive or pass the complete absolute-floor admission process."

export SS_METHOD="$method" SS_FETCH="$fetch" SS_ROADMAP="$roadmap" SS_HANDOFF="$handoff" SS_TODOS="$todos" SS_VERIFY="$verify_out"

out="$(python3 <<'PY'
import json, os
LIMIT = 9500  # safety margin under the harness's 10000-character additionalContext cap
# Per-section budgets: each block is clipped on its own so none can starve another. The
# top HANDOFFS entry is the priority (recover state), so it gets the largest share.
BUDGETS = {"method": 500, "fetch": 650, "roadmap": 3200, "handoff": 3200, "todos": 1300, "verify": 500}

def clip(s, n):
    s = s or ""
    if len(s) <= n:
        return s
    return s[:n].rstrip() + "\n...[truncated %d chars; read the file for the rest]" % (len(s) - n)

sec = {k: clip(os.environ.get("SS_" + k.upper(), ""), b) for k, b in BUDGETS.items()}
ctx = (
    "Session memory loaded by the SessionStart hook.\n\n"
    + sec["method"] + "\n\n"
    + sec["fetch"] + "\n\n"
    + "=== docs/working/CONSENSUS_ROADMAP.md (the ground-truth order of work; keep it current) ===\n"
    + sec["roadmap"] + "\n\n"
    + "=== HANDOFFS.md (read the top entry first to recover state) ===\n" + sec["handoff"] + "\n\n"
    + "=== TODOS.md (canonical abiotic queue) ===\n" + sec["todos"] + "\n\n"
    + "=== parked legacy archive verification baseline (scripts/verify.sh) ===\n" + sec["verify"]
)
if len(ctx) > LIMIT:
    ctx = ctx[:LIMIT].rstrip() + "\n...[truncated to the context cap]"
print(json.dumps({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": ctx}}))
PY
)"

# Cargo artifacts once reached 244 GB across per-agent targets. Session maintenance now shares one
# native-WSL target per clone, enforces its hard cap, drains old worktree targets, and issues a throttled
# online fstrim. It is detached because this hook's stdout must remain one valid JSON object.
if [ -f "$ROOT/scripts/session_maintenance.sh" ]; then
  # Keep detached maintenance output diagnostic but bounded. Trimming before
  # launch avoids an ever-growing ignored file across long-lived clones.
  maintenance_log="$ROOT/.claude/target_gc.log"
  if [ -f "$maintenance_log" ] && [ "$(wc -c < "$maintenance_log" 2>/dev/null || echo 0)" -gt 1048576 ]; then
    maintenance_tail="$(mktemp "${TMPDIR:-/tmp}/civsim-maintenance-log.XXXXXX" 2>/dev/null || true)"
    if [ -n "$maintenance_tail" ]; then
      tail -c 262144 "$maintenance_log" > "$maintenance_tail" 2>/dev/null && mv "$maintenance_tail" "$maintenance_log"
      rm -f "$maintenance_tail"
    fi
  fi
  ( setsid bash "$ROOT/scripts/session_maintenance.sh" >> "$ROOT/.claude/target_gc.log" 2>&1 & ) >/dev/null 2>&1 </dev/null
fi

# Guard the Python step: on any failure (including a crash on unusual content) emit a
# minimal valid object, never nothing and never malformed JSON.
if [ $? -ne 0 ] || [ -z "$out" ]; then
  printf '%s\n' '{"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": "SessionStart hook: context assembly failed; read HANDOFFS.md and TODOS.md and run scripts/verify.sh manually."}}'
  exit 0
fi
printf '%s\n' "$out"
exit 0
