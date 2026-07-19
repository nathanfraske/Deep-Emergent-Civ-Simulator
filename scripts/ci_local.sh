#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# Run what CI runs, READ FROM the workflow rather than copied from it.
#
# WHY THIS EXISTS. A 225-file structural change passed twelve local checks and then failed CI on an unused
# import. The cause was drift between two lists that were supposed to be the same one: CI runs
# `cargo clippy --workspace --all-targets -- -D warnings`, and the local habit had been plain
# `cargo clippy --workspace --all-targets`. The local result printed "0" and looked identical while being a
# strictly weaker claim.
#
# It was not one flag. Comparing the workflow against the local habit found SIX classes of check that were
# never being run locally at all: `-D warnings`, `--document-private-items` on the doc gate, the doc TESTS
# (`cargo test --workspace --doc`), every gate's `--self-test` companion, the stone0 binary in `--ci` mode,
# and the tombstone scan.
#
# So this does not restate the commands. It PARSES them out of `.github/workflows/ci.yml` and runs them, so
# there is one definition and a command added to CI is picked up here automatically. A second hardcoded
# list is the diamond pattern this repository keeps paying for.
#
# Honest limits, stated so this is not mistaken for CI:
#   - It runs on your working tree, not a clean checkout, so it cannot catch a missing file that happens to
#     exist locally or a stale build artifact masking a real error.
#   - CI uses `cargo nextest`; if nextest is absent here the run falls back to `cargo test`, which differs
#     in isolation and in how it reports.
#   - Scheduled-only jobs (the nightly full suite) are skipped, exactly as they are on a pull request.
#
# Usage:
#   scripts/ci_local.sh           run every extracted check, report a summary, exit non-zero on any failure
#   scripts/ci_local.sh --list    print the commands it would run and exit

set -uo pipefail
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$ROOT" || exit 1
export PATH="$HOME/.cargo/bin:$PATH"

WORKFLOW=".github/workflows/ci.yml"
[ -f "$WORKFLOW" ] || { echo "ci_local: no $WORKFLOW" >&2; exit 1; }

# Extract every `run:` command from the jobs that fire on a push or pull request. A job guarded by
# `if: github.event_name == 'schedule'` is nightly-only and is skipped here for the same reason CI skips it.
mapfile -t CMDS < <(python3 - "$WORKFLOW" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
# split into top-level jobs so a scheduled-only job can be dropped whole
jobs = re.split(r'\n  (?=[a-z][a-z0-9_-]*:\n)', text)
out = []
for job in jobs:
    if re.search(r"if:\s*github\.event_name == 'schedule'", job):
        continue
    for m in re.finditer(r'^\s+run:\s*(.+)$', job, re.M):
        cmd = m.group(1).strip()
        if cmd.startswith('|'):
            continue                      # heredoc block, not a single command
        if cmd.startswith(('cargo', 'python3 scripts/', 'bash scripts/', 'RUSTDOCFLAGS')):
            out.append(cmd)
    # multi-line run blocks: take their cargo/python/bash lines
    for m in re.finditer(r'^\s+run: \|\s*\n((?:\s{8,}.*\n)+)', job, re.M):
        for line in m.group(1).split('\n'):
            c = line.strip()
            if c.startswith(('cargo', 'python3 scripts/', 'bash scripts/', 'RUSTDOCFLAGS')):
                out.append(c)
seen, uniq = set(), []
for c in out:
    if c not in seen:
        seen.add(c); uniq.append(c)
print('\n'.join(uniq))
PY
)

if [ "${1:-}" = "--list" ]; then
  printf 'ci_local: %d command(s) extracted from %s\n' "${#CMDS[@]}" "$WORKFLOW"
  for c in "${CMDS[@]}"; do echo "  $c"; done
  exit 0
fi

# nextest is CI's runner; fall back if it is not installed, and SAY so rather than silently differing.
HAVE_NEXTEST=1
cargo nextest --version >/dev/null 2>&1 || HAVE_NEXTEST=0

pass=0; fail=0; failed_cmds=()
for c in "${CMDS[@]}"; do
  run="$c"
  if [[ "$c" == *"cargo nextest"* && "$HAVE_NEXTEST" = "0" ]]; then
    run="${c/cargo nextest run/cargo test}"
    run="${run%% -E *}"
    echo "  [substituted, nextest absent] $run"
  fi
  out="$(mktemp)"
  if eval "$run" >"$out" 2>&1; then
    pass=$((pass + 1)); printf '  PASS  %s\n' "${c:0:96}"
  else
    fail=$((fail + 1)); failed_cmds+=("$c")
    printf '  FAIL  %s\n' "${c:0:96}"
    grep -E '^(error|warning: unused|##\[error\])' "$out" | head -5 | sed 's/^/          /'
  fi
  rm -f "$out"
done

echo "  ---"
echo "  ci_local: $pass passed, $fail failed (of ${#CMDS[@]} extracted)"
if [ "$fail" -gt 0 ]; then
  echo "  failing:"
  for c in "${failed_cmds[@]}"; do echo "    $c"; done
  exit 1
fi
[ "$HAVE_NEXTEST" = "0" ] && echo "  NOTE: nextest absent, test runs were substituted with cargo test and differ from CI."
exit 0
