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
    echo "It is a LEAN task board: edit IN PLACE, one short line per item (a date, a few words, and a POINTER to the commit/file/doc/worktree). Tombstone a landed item (mark it DONE with its pointer), or add a newly-flagged one citing its gate (an R-item, a Part number, a file). Do NOT inline the work's detail (point to it instead), do NOT append a dated narrative section (that is HANDOFFS), do NOT touch unrelated lines. The board answers where-are-we and where-did-it-go at a glance, nothing more." >&2
    exit 2
  fi
  # Size guard: the board must stay lean. If a line wants to grow, that detail belongs behind its pointer.
  roadmap_bytes="$(wc -c < "$roadmap_file" 2>/dev/null | tr -d ' ')"
  if [ -n "$roadmap_bytes" ] && [ "$roadmap_bytes" -gt 16384 ]; then
    echo "stop-gate: the CONSENSUS_ROADMAP ballooned to ${roadmap_bytes} bytes (lean cap 16384)." >&2
    echo "Trim it: each item a few words plus a pointer, the detail moved behind the pointer (HANDOFFS, a commit, a doc). The retired long-form archive is CONSENSUS_ROADMAP_HISTORY.md; do not grow the board into that again." >&2
    exit 2
  fi
fi

# Floor-registry gate. The physics floor registry (docs/working/PHYSICS_FLOOR_REGISTRY.md) is the
# enforced reference for the derive-vs-author line and the physics-substrate map: the generated list of
# every authored floor axis and substance AND every law kernel (declared in the floor data or direct in
# laws.rs), each with file:line. It is generated from the floor data and laws.rs, so if it is stale (a
# floor axis, a law kernel, or the generator changed and the registry was not regenerated) the reference
# is wrong and an audit against it is unsound. The
# gate regenerates to a temp and compares, never touching the working tree. Inert until both the
# generator and the registry exist.
# Provenance ratchet, at TURN scope rather than build scope. The Stone 0 gate runs from
# crates/sim/build.rs, so it only fires when a build touches civsim-sim. A package-scoped command such as
# `cargo test -p civsim-physics` never fires it, which means a physics-only change can be written,
# verified, and committed without the provenance ratchet ever running. That happened: an unclassified
# from_decimal_str site rode in on a physics-only commit and surfaced later, on an unrelated build.
#
# This closes it without touching the build graph, so no build gets slower and no gate is duplicated. The
# script list is READ FROM the Rust source rather than copied here, so there is ONE list: a gate added to
# PROVENANCE_SCRIPTS is picked up here automatically and the two cannot drift apart.
if [ -n "$(git -C "$ROOT" status --porcelain -- crates 2>/dev/null)" ]; then
  scripts_list="$(sed -n '/const PROVENANCE_SCRIPTS/,/];/p' "$ROOT/crates/stone0/src/lib.rs" 2>/dev/null \
    | grep -oE '"scripts/[a-z0-9_]+\.py"' | tr -d '"')"
  for s in $scripts_list; do
    [ -f "$ROOT/$s" ] || continue
    if ! out="$(cd "$ROOT" && python3 "$s" 2>&1)"; then
      echo "stop-gate: the provenance ratchet failed in $s." >&2
      echo "This runs at turn scope because a package-scoped cargo command does not fire crates/sim/build.rs," >&2
      echo "so a physics-only change would otherwise skip the Stone 0 gate entirely." >&2
      printf '%s\n' "$out" | tail -20 >&2
      exit 2
    fi
  done
fi

# Derives-coverage gate. The registry's staleness check cannot see an UNMARKED deriving function: the
# generator regenerates identically and the check passes while the map is wrong, which is how the physics
# substrate ended up with 818 public functions and no markers at all. This ratchets that shut.
if [ -f "$ROOT/scripts/derives_gate.py" ] && [ -f "$ROOT/scripts/derives_baseline.tsv" ]; then
  if ! python3 "$ROOT/scripts/derives_gate.py" >/tmp/civsim_derives.out 2>&1; then
    echo "stop-gate: the derives-coverage gate failed." >&2
    tail -20 /tmp/civsim_derives.out >&2
    exit 2
  fi
fi

reg="docs/working/PHYSICS_FLOOR_REGISTRY.md"
gen="scripts/gen_floor_registry.py"
if [ -f "$ROOT/$gen" ] && [ -f "$ROOT/$reg" ]; then
  tmpreg="$(mktemp)"
  if python3 "$ROOT/$gen" "$tmpreg" >/dev/null 2>&1; then
    if ! diff -q "$tmpreg" "$ROOT/$reg" >/dev/null 2>&1; then
      rm -f "$tmpreg"
      echo "stop-gate: the physics floor registry is stale (a floor axis, a laws.rs kernel, or the generator changed but it was not regenerated)." >&2
      echo "Run: python3 scripts/gen_floor_registry.py   then commit docs/working/PHYSICS_FLOOR_REGISTRY.md. The registry is the enforced derive-vs-author reference; a stale one is a wrong reference." >&2
      exit 2
    fi
  fi
  rm -f "$tmpreg"
fi

exit 0
