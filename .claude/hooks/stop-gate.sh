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

# Roadmap living-document gate, with the FEATURE STATUS BOARD as its point. If source
# (crates/**) or the design/roadmap documents changed this session but the consensus
# roadmap was not touched, block: a segment that changed source almost always moved an
# item on the status board (a NOT DONE to DONE, a GATE cleared, a new arc flagged), and the
# board must never go stale. Only fires when the roadmap exists, so it is inert until then.
roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  work_changed="$(git -C "$ROOT" status --porcelain -- crates docs/design.md docs/audit.md ROADMAP.md 2>/dev/null)"
  roadmap_changed="$(git -C "$ROOT" status --porcelain -- "$roadmap_file" 2>/dev/null)"
  if [ -n "$work_changed" ] && [ -z "$roadmap_changed" ]; then
    echo "stop-gate: source or design files changed but the CONSENSUS_ROADMAP was not updated." >&2
    echo "Edit ONLY the 'Feature status board' item(s) this segment moved, IN PLACE, and nothing else: sign off a done feature (tombstone it where it stands, flip its status to DONE, and point to where the work landed by a file or test), or deprecate/re-classify a section, or add a newly-flagged item citing its gate (an R-item, a Part number, or a file). Do NOT append a new dated section, do NOT rewrite the whole board, and do NOT touch unrelated segments. The board is a living in-place tracker, not an append log; that is the point." >&2
    exit 2
  fi
fi

# Floor-registry gate. The physics floor registry (docs/working/PHYSICS_FLOOR_REGISTRY.md) is the
# enforced reference for the derive-vs-author line: the generated list of every authored floor axis.
# It is generated from the floor data, so if it is stale (a floor axis or the generator changed and
# the registry was not regenerated) the reference is wrong and an audit against it is unsound. The
# gate regenerates to a temp and compares, never touching the working tree. Inert until both the
# generator and the registry exist.
reg="docs/working/PHYSICS_FLOOR_REGISTRY.md"
gen="scripts/gen_floor_registry.py"
if [ -f "$ROOT/$gen" ] && [ -f "$ROOT/$reg" ]; then
  tmpreg="$(mktemp)"
  if python3 "$ROOT/$gen" "$tmpreg" >/dev/null 2>&1; then
    if ! diff -q "$tmpreg" "$ROOT/$reg" >/dev/null 2>&1; then
      rm -f "$tmpreg"
      echo "stop-gate: the physics floor registry is stale (a floor axis or the generator changed but it was not regenerated)." >&2
      echo "Run: python3 scripts/gen_floor_registry.py   then commit docs/working/PHYSICS_FLOOR_REGISTRY.md. The registry is the enforced derive-vs-author reference; a stale one is a wrong reference." >&2
      exit 2
    fi
  fi
  rm -f "$tmpreg"
fi

exit 0
