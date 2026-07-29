#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# The verification suite as one callable script (CLAUDE.md section 8, runbook
# section 1c). It checks the two maintained legacy documents
# (parked/docs/design.md and parked/docs/audit.md) against the prose customs and
# document invariants. The archived
# research papers under parked/docs/research/ are deliberately not checked: they predate
# the customs and keep their em dashes verbatim.
#
# Usage:
#   scripts/verify.sh           human-readable pass-or-fail summary; exit 0 if clean
#   scripts/verify.sh --json    structured JSON for the projectops server and panels
#
# The em-dash check matches the UTF-8 bytes of U+2014 directly, because some grep
# builds reject the \x{2014} code-point form.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
F="$ROOT/parked/docs/design.md"
A="$ROOT/parked/docs/audit.md"
EMDASH=$'\xe2\x80\x94'

MODE="human"
if [ "${1:-}" = "--json" ]; then MODE="json"; fi

names=(); passes=(); details=()
overall=0

record() {
  # record <name> <pass:0|1> <detail>
  names+=("$1"); passes+=("$2"); details+=("$3")
  if [ "$2" -ne 1 ]; then overall=1; fi
}

count_lines() { # count_lines <pattern> <file>  (matching lines; never fails the script)
  grep -c "$1" "$2" 2>/dev/null || true
}

# 1, 2: em dashes must be 0 in each maintained document.
for pair in "design:$F" "audit:$A"; do
  label="${pair%%:*}"; file="${pair#*:}"
  n=$(grep -c "$EMDASH" "$file" 2>/dev/null || true)
  if [ "$n" -eq 0 ]; then p=1; else p=0; fi
  record "em_dashes_$label" "$p" "$n line(s) with an em dash"
done

# 3, 4: banned adverbs (genuinely, honestly, actually) must be 0.
for pair in "design:$F" "audit:$A"; do
  label="${pair%%:*}"; file="${pair#*:}"
  n=$(grep -ciE 'genuinely|honestly|\bactually\b' "$file" 2>/dev/null || true)
  if [ "$n" -eq 0 ]; then p=1; else p=0; fi
  record "banned_adverbs_$label" "$p" "$n line(s) with a banned adverb"
done

# 5: parts gapless 0..63 in the design document. Use one POSIX awk pass rather
# than grep -P: Git for Windows ships a grep build whose PCRE mode rejects its
# own UTF-8 locale, which made the real document read as "parts OK 0".
parts=$(awk '
  /^## Part [0-9]+:/ {
    part = $3
    sub(/:$/, "", part)
    count++
    if (count != part + 1) {
      print "GAP at index " count " got " part
      bad = 1
    }
  }
  END { if (!bad) print "parts OK " count }
' "$F")
if [ "$parts" = "parts OK 64" ]; then p=1; else p=0; fi
record "parts_gapless" "$p" "$parts"

# 6: code fences balanced (even count of ``` lines) in the design document.
fences=$(grep -c '```' "$F" 2>/dev/null || true)
if [ $((fences % 2)) -eq 0 ]; then p=1; else p=0; fi
record "fences_balanced" "$p" "$fences fence line(s)"

# 7: research records 62.1..62.N sequential with no gaps.
recs=$(grep -oE '^### 62\.[0-9]+' "$F" | grep -oE '[0-9]+$' \
  | awk 'NR!=$1{print "GAP at "NR" got "$1; bad=1} END{if(!bad) print "records OK "NR}')
if echo "$recs" | grep -q "records OK"; then p=1; else p=0; fi
record "records_sequential" "$p" "$recs"

# 8: open backlog count (informational; the running number of open research items).
backlog=$(grep -c '^- \*\*R-' "$A" 2>/dev/null || true)
record "open_backlog_count" 1 "$backlog open item(s)"

# 9: every "Needs research" flag in the design corresponds to an open backlog item
#    and no resolved item still carries one (soft consistency, informational).
flags=$(grep -c '> Needs research' "$F" 2>/dev/null || true)
resolved=$(grep -c '> Decided and reserved' "$F" 2>/dev/null || true)
record "flags_vs_resolved" 1 "$flags open flag(s), $resolved resolved blockquote(s)"

if [ "$MODE" = "json" ]; then
  printf '{\n  "ok": %s,\n  "checks": [\n' "$([ $overall -eq 0 ] && echo true || echo false)"
  last=$(( ${#names[@]} - 1 ))
  for i in "${!names[@]}"; do
    pb=$([ "${passes[$i]}" -eq 1 ] && echo true || echo false)
    comma=","; [ "$i" -eq "$last" ] && comma=""
    printf '    {"name": "%s", "pass": %s, "detail": "%s"}%s\n' \
      "${names[$i]}" "$pb" "${details[$i]}" "$comma"
  done
  printf '  ]\n}\n'
else
  echo "legacy archive verification over parked/docs/design.md and parked/docs/audit.md"
  echo "------------------------------------------------------------"
  for i in "${!names[@]}"; do
    if [ "${passes[$i]}" -eq 1 ]; then tag="PASS"; else tag="FAIL"; fi
    printf '  [%s] %-22s %s\n' "$tag" "${names[$i]}" "${details[$i]}"
  done
  echo "------------------------------------------------------------"
  if [ $overall -eq 0 ]; then echo "RESULT: clean"; else echo "RESULT: dirty (see FAIL lines above)"; fi
fi

exit $overall
