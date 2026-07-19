#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PIPELINE-STATUS GUARD (PreToolUse on Bash).
#
# A shell pipeline reports the exit status of its LAST command. So
#
#     cargo test --workspace 2>&1 | tail -25
#
# exits 0 whenever `tail` succeeds, no matter what cargo did. On 2026-07-18 that produced a false report
# in this project twice in one turn: the toolchain was not on PATH, cargo exited 127 with
# "cargo: command not found", `tail` exited 0, and the agent reported the workspace suite as passing when
# it had never run. The same shape then hid a clean `cargo doc` result behind a SIGPIPE-truncated `head`
# in a sweep, which produced an unsound "zero broken links" finding.
#
# The correction was written down as a habit ("capture the status directly from now on"). A habit is not a
# defense: agent habits are precisely what the gates in this repo exist to catch. So it is hardcoded here.
#
# WHAT IS BLOCKED: piping a VERIFIER (cargo, python gate scripts, scripts/verify.sh) into a filter
# (head/tail/grep/wc/sed/awk/cut/sort/uniq/jq) when nothing in the command preserves the real status.
#
# WHAT IS ACCEPTED, because each genuinely preserves it:
#   - `set -o pipefail` anywhere in the command
#   - an explicit ${PIPESTATUS[...]} read
#   - redirecting to a file and testing $? afterwards (no pipe from the verifier at all)
#
# The honest limit: this is a textual guard on one well-understood shape. It cannot catch every way a
# status can be dropped (a subshell that swallows it, a `|| true`, a wrapper script). It removes the shape
# that actually caused a false green here, and it is stated plainly so it is not mistaken for coverage.

set -uo pipefail
input="$(cat)"

cmd="$(printf '%s' "$input" | python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
    print(d.get("tool_input", {}).get("command", ""))
except Exception:
    print("")
' 2>/dev/null)"

[ -z "$cmd" ] && exit 0

# Already safe: any of these preserves the real status.
case "$cmd" in
  *pipefail*|*PIPESTATUS*) exit 0 ;;
esac

# A verifier whose status is load-bearing.
verifier_re='(^|[;&|(]|[[:space:]])(cargo|python3?[[:space:]]+scripts/[a-z0-9_]+\.py|bash[[:space:]]+scripts/verify\.sh|scripts/verify\.sh)([[:space:]]|$)'
# A filter that would mask it.
filter_re='\|[[:space:]]*(head|tail|grep|wc|sed|awk|cut|sort|uniq|jq)([[:space:]]|$)'

# Judge PER STATEMENT, never across the whole command. A command often redirects a verifier safely and
# then pipes something unrelated (`grep ... | cut`) in a later statement; matching the two patterns
# anywhere in one string blocks that, which is a false positive. A false positive is not a harmless gate:
# it teaches the reader to route around the guard, which is how a gate decays into a suggestion. So split
# on statement separators and require the verifier and the masking filter to sit in the SAME statement.
offending=0
while IFS= read -r stmt; do
  [ -z "$stmt" ] && continue
  case "$stmt" in
    *pipefail*|*PIPESTATUS*) continue ;;
  esac
  # A verifier whose output is REDIRECTED rather than piped keeps its status; that is the safe form.
  if printf '%s' "$stmt" | grep -qE "$verifier_re" && printf '%s' "$stmt" | grep -qE "$filter_re"; then
    offending=1
    break
  fi
done <<EOF
$(printf '%s' "$cmd" | sed -e 's/&&/\n/g' -e 's/||/\n/g' -e 's/;/\n/g')
EOF

if [ "$offending" = 1 ]; then
  cat >&2 <<'MSG'
pipeline-status-guard: BLOCKED. This pipes a verifier into a filter, so the exit status you read back
belongs to the FILTER, not to the verifier.

    cargo test --workspace 2>&1 | tail -25      # exits 0 whenever tail succeeds

This exact shape reported a passing test suite in this repo while cargo was exiting 127 with
"cargo: command not found", and separately hid a truncated cargo doc run behind head, producing an
unsound finding. It is blocked rather than discouraged because a habit is not a defense.

Use one of these instead:

    out=$(mktemp); cargo test --workspace > "$out" 2>&1; status=$?
    echo "EXIT: $status"; tail -25 "$out"

    set -o pipefail; cargo test --workspace 2>&1 | tail -25

    cargo test --workspace 2>&1 | tail -25; echo "EXIT: ${PIPESTATUS[0]}"

If the status truly does not matter here (a read-only survey, not a verification), add `set -o pipefail`
anyway, or state the intent by reading ${PIPESTATUS[0]}.
MSG
  exit 2
fi

exit 0
