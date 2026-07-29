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
set -o pipefail
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

verify_tmp="$(mktemp "${TMPDIR:-/tmp}/civsim-stop-verify.XXXXXX")" || {
  echo "stop-gate: could not allocate verification output." >&2
  exit 2
}
trap 'rm -f "$verify_tmp"' EXIT

if ! python3 scripts/gate_runner.py run --tier stop --phase pre >"$verify_tmp" 2>&1; then
  echo "stop-gate: the maintained documents are not clean; the turn cannot end." >&2
  echo "Run the Stop pre-phase gates and fix the FAIL lines, then finish." >&2
  tail -25 "$verify_tmp" >&2
  exit 2
fi

# Every implementation or repository-entrypoint segment updates both memory
# surfaces. Checking them independently prevents an edit to one from masking a
# stale other file.
memory_trigger_changed="$(git -C "$ROOT" status --porcelain -- \
  crates Cargo.toml Cargo.lock parked/Cargo.toml parked/Cargo.lock parked/crates parked/README.md \
  justfile Makefile scripts .github .claude docs/working AGENTS.md CLAUDE.md RUNBOOK.md README.md \
  2>/dev/null)"
handoff_changed="$(git -C "$ROOT" status --porcelain -- HANDOFFS.md 2>/dev/null)"
todos_changed="$(git -C "$ROOT" status --porcelain -- TODOS.md 2>/dev/null)"
if [ -n "$memory_trigger_changed" ] && { [ -z "$handoff_changed" ] || [ -z "$todos_changed" ]; }; then
  echo "stop-gate: repository work changed but HANDOFFS.md and TODOS.md were not both updated." >&2
  echo "Append the current HANDOFFS entry and update the bounded canonical queue before finishing." >&2
  exit 2
fi

# Roadmap living-document gate, with the LEAN POINTER BOARD as its point. If source
# (crates/**) or the design/roadmap documents changed this session but the consensus
# roadmap was not touched, block: a segment that changed source almost always moved an
# item on the board (an arc advanced, a gate cleared, a landing tombstoned, a new arc
# flagged), and the board must never go stale. Only fires when the roadmap exists.
roadmap_file="docs/working/CONSENSUS_ROADMAP.md"
if [ -f "$roadmap_file" ]; then
  work_changed="$(git -C "$ROOT" status --porcelain -- \
    crates Cargo.toml Cargo.lock justfile Makefile scripts .github .claude \
    AGENTS.md CLAUDE.md RUNBOOK.md README.md docs/working \
    ':(exclude)docs/working/CONSENSUS_ROADMAP.md' 2>/dev/null)"
  roadmap_changed="$(git -C "$ROOT" status --porcelain -- "$roadmap_file" 2>/dev/null)"
  if [ -n "$work_changed" ] && [ -z "$roadmap_changed" ]; then
    echo "stop-gate: source or design files changed but the CONSENSUS_ROADMAP was not updated." >&2
    echo "Edit ONLY the line(s) this segment moved, IN PLACE, and nothing else. Every entry is ONE short line: a date, a few words, and a pointer (a branch, a PR, a file, or a doc). Move a line when its item moves, tombstone a landed item under 'Recent landings' with its pointer, prune stale landings, or add a newly-flagged item citing its gate. The detail lives BEHIND the pointer: do NOT inline it, do NOT append a dated narrative (that is HANDOFFS.md), do NOT rewrite the whole board, and do NOT touch unrelated lines. The board is a lean in-place pointer board, not an append log; that is the point." >&2
    exit 2
  fi
fi

# Roadmap size guard, the other half of the lean form. The board balloons whenever a segment
# pastes its detail inline instead of pointing at it, which is how the old board reached half a
# megabyte and made every parallel agent collide on it. A hard byte cap keeps the board a board.
if [ -f "$roadmap_file" ]; then
  # Count the repository form. A Windows CRLF checkout must not consume the
  # board budget with one extra byte per line.
  roadmap_bytes="$(python3 -c 'import sys; print(len(open(sys.argv[1], "rb").read().replace(b"\r\n", b"\n")))' "$ROOT/$roadmap_file" 2>/dev/null || echo 0)"
  if [ "$roadmap_bytes" -gt 16384 ]; then
    echo "stop-gate: the CONSENSUS_ROADMAP is $roadmap_bytes bytes, over the 16384-byte cap." >&2
    echo "Trim it back to one short line per item (a date, a few words, a pointer). Move any inlined detail behind its pointer, and prune tombstoned landings that have gone stale. The retired long-form board is docs/working/CONSENSUS_ROADMAP_HISTORY.md; do not grow the live board back into it." >&2
    exit 2
  fi
fi

# Provenance ratchet, at TURN scope rather than build scope. Package-scoped
# cargo commands do not prove the complete canonical boundary, and the boundary
# gate also reads Cargo manifests and viewer sources. Membership, order,
# arguments, timeouts, hashes, and self-test metadata come from gates.toml.
repo_changed="$(git -C "$ROOT" status --porcelain -- . 2>/dev/null)"
if [ -n "$repo_changed" ]; then
  if ! out="$(cd "$ROOT" && python3 scripts/gate_runner.py --self-test 2>&1)"; then
    echo "stop-gate: the declarative gate authority failed its synthetic self-test." >&2
    printf '%s\n' "$out" | tail -20 >&2
    exit 2
  fi
  if ! out="$(cd "$ROOT" && python3 scripts/gate_runner.py self-tests --tier stop 2>&1)"; then
    echo "stop-gate: a declared detector self-test failed." >&2
    printf '%s\n' "$out" | tail -30 >&2
    exit 2
  fi
  if [ -n "$(git -C "$ROOT" status --porcelain -- crates/stone0 2>/dev/null)" ]; then
    if ! out="$(cd "$ROOT" && bash scripts/cargo_dev.sh run -q -p civsim-stone0 --bin stone0-gate -- --self-test 2>&1)"; then
      echo "stop-gate: the Stone 0 native self-test failed." >&2
      printf '%s\n' "$out" | tail -20 >&2
      exit 2
    fi
  fi
  if ! out="$(cd "$ROOT" && python3 scripts/gate_runner.py run --tier stop --phase provenance 2>&1)"; then
    echo "stop-gate: the declarative provenance ratchet failed." >&2
    echo "This runs at turn scope because a package-scoped cargo command can skip canonical boundary inputs." >&2
    printf '%s\n' "$out" | tail -30 >&2
    exit 2
  fi
fi

# Canonical ledger accounting inventory. Counts and memberships are generated from the audited catalog; a hand-edited
# report or a catalog change without regeneration is a stale central ledger view and blocks completion.
if ! out="$(cd "$ROOT" && python3 scripts/gate_runner.py run --tier stop --phase post 2>&1)"; then
  echo "stop-gate: a declarative Stop post-phase gate failed." >&2
  printf '%s\n' "$out" | tail -20 >&2
  exit 2
fi

exit 0
