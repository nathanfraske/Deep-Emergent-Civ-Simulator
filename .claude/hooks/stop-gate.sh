#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# Stop completion gate (AGENTIC_ADDENDUM.md section 2d). The turn cannot end while
# the documents are dirty or the memory files are stale. It runs the full
# verification suite and checks that, if either maintained document changed, the
# memory files were updated, and that, if source or design files changed, the
# consensus roadmap was kept current. To avoid a loop it first honours
# stop_hook_active.

set -u
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
input="$(cat)"

active="$(printf '%s' "$input" | python3 -c '
import json, sys
try:
    print(json.load(sys.stdin).get("stop_hook_active", False))
except Exception:
    print(False)
' 2>/dev/null)"
if [ "$active" = "True" ]; then
  exit 0
fi

cd "$ROOT" || exit 0

if ! bash scripts/verify.sh >/tmp/civsim_stop_verify.out 2>&1; then
  echo "stop-gate: the maintained documents are not clean; the turn cannot end." >&2
  echo "Run scripts/verify.sh and fix the FAIL lines, then finish." >&2
  tail -25 /tmp/civsim_stop_verify.out >&2
  exit 2
fi

docs_changed="$(git -C "$ROOT" status --porcelain -- docs/design.md docs/audit.md 2>/dev/null)"
mem_changed="$(git -C "$ROOT" status --porcelain -- HANDOFFS.md TODOS.md 2>/dev/null)"
if [ -n "$docs_changed" ] && [ -z "$mem_changed" ]; then
  echo "stop-gate: a maintained document changed but HANDOFFS.md / TODOS.md were not updated." >&2
  echo "Append a dated HANDOFFS entry and update TODOS before finishing (CLAUDE.md section 10)." >&2
  exit 2
fi

# Roadmap living-document gate. If source (crates/**) or the design and roadmap
# documents changed this session but the consensus roadmap was not touched, prompt
# an update. Only fires when the roadmap exists, so it is inert until the roadmap is
# in the tree.
roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  work_changed="$(git -C "$ROOT" status --porcelain -- crates docs/design.md docs/audit.md ROADMAP.md 2>/dev/null)"
  roadmap_changed="$(git -C "$ROOT" status --porcelain -- "$roadmap_file" 2>/dev/null)"
  if [ -n "$work_changed" ] && [ -z "$roadmap_changed" ]; then
    echo "stop-gate: source or design files changed but $roadmap_file was not updated." >&2
    echo "Review the roadmap and update it as work is done: cite each item's gate (an R-item, a Part number, or a file), and tombstone a completed or removed item by rewriting it in place rather than deleting it." >&2
    exit 2
  fi
fi

exit 0
