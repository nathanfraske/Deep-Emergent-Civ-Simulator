#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# PreToolUse guard on the Agent tool: a FETCH agent must be briefed with the vendoring protocol.
#
# WHY THIS EXISTS. The standing rule (AGENTIC_ADDENDUM.md section 12, docs/working/VENDORING_CHECKLIST.md)
# is that a load-bearing number must have its source vendored AT FETCH TIME: the document downloaded, a
# SHA256 taken, the bytes or a licensed citation-plus-witness held behind a manifest, the value's anchor
# and scope carried. A URL citation is not provenance.
#
# That rule lived in the dispatcher's head. Every fetch agent that got it got it because whoever wrote the
# prompt happened to remember, and an agent briefed without it produces a number with a hyperlink behind
# it, which is the exact defect the rule exists to prevent. Tonight's provenance audit found the cost of
# that: a source certified with `sha256 = "anything"`, 34 held tables with no receipt at all, and four
# claims naming one document as both of their two independent witnesses.
#
# So the brief is not a habit any more. A fetch-shaped Agent dispatch that does not carry the protocol is
# BLOCKED, and the block prints the protocol, so the fix is to paste it rather than to remember it. This is
# the same move as every fix in the provenance arc: a defence carried only in prose is one that gets
# dropped, so it becomes a refusal instead.
#
# It is deliberately conservative. It fires only when the prompt is clearly about acquiring external data
# (fetch/vendor/download/cite a source, a DOI, a paper, a table) AND lacks the protocol markers. A
# read-only research or search agent is not a fetch agent and is not blocked.

set -uo pipefail
payload="$(cat)"

tool="$(printf '%s' "$payload" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("tool_name",""))' 2>/dev/null || echo "")"
[ "$tool" = "Agent" ] || exit 0

prompt="$(printf '%s' "$payload" | python3 -c '
import sys, json
d = json.load(sys.stdin)
print(d.get("tool_input", {}).get("prompt", ""))
' 2>/dev/null || echo "")"
[ -n "$prompt" ] || exit 0

lower="$(printf '%s' "$prompt" | tr "[:upper:]" "[:lower:]")"

# Is this a FETCH dispatch? Needs an acquisition verb AND an external-source noun, so "search the codebase
# for a citation" does not trip it.
verb_re='(fetch|vendor|download|retrieve|acquire|obtain|source (the|a|its))'
noun_re='(doi|arxiv|paper|primary source|literature|journal|nist|iapws|janaf|handbook|dataset|data file|table from|published)'
printf '%s' "$lower" | grep -qE "$verb_re" || exit 0
printf '%s' "$lower" | grep -qE "$noun_re" || exit 0

# Does it already carry the protocol? Four markers, because a brief that mentions only one of them is a
# brief that will produce three of the four failures.
missing=()
printf '%s' "$lower" | grep -qE 'sha256|checksum'                   || missing+=("a SHA256/checksum requirement")
printf '%s' "$lower" | grep -qE 'paywall|licen[cs]e|redistribut'     || missing+=("the paywall/licence check")
printf '%s' "$lower" | grep -qE 'wayback|internet archive|archived_url|witness' || missing+=("the archive-witness fallback")
printf '%s' "$lower" | grep -qE 'anchor|table|figure|page|locator'   || missing+=("the per-value anchor (table/figure/page)")

[ "${#missing[@]}" -eq 0 ] && exit 0

{
  echo "fetch-brief-guard: BLOCKED. This looks like a FETCH agent dispatch and its brief is missing:"
  for m in "${missing[@]}"; do echo "    - $m"; done
  echo
  echo "A fetch agent briefed without the vendoring protocol produces a number whose provenance is a"
  echo "hyperlink, which is the defect the protocol exists to prevent. Paste this into the prompt:"
  echo
  echo "  ## The vendoring protocol (docs/working/VENDORING_CHECKLIST.md, AGENTIC_ADDENDUM.md section 12)"
  echo "  - Check the PAYWALL first. If paywalled with no free legitimate copy, do NOT vendor bytes: use"
  echo "    CITATION-PLUS-WITNESS (custody = \"witness\", a RESOLVING Wayback archived_url, a licence field"
  echo "    recording the terms and why bytes are not held, and an extract quoting the exact table cell)."
  echo "  - If openly licensed, hold the bytes: sha256 (64 hex, no exceptions), custody = \"in_repo\","
  echo "    licence, redistributable, and a slim field saying what was kept and dropped."
  echo "  - Every value carries its band or uncertainty FROM THE SOURCE'S OWN STATEMENT (never invented),"
  echo "    its anchor (table/figure/page/equation), and the scope/regime it is valid in."
  echo "  - Read the primary's figures and tables, not the abstract."
  echo "  - A value you cannot source, you OMIT. An absent row is correct and expected; an invented row,"
  echo "    a periodic-table trend, or a related mineral's value is the worst outcome and will be caught."
  echo "  - Deliver the data column, its manifest, and an OFFLINE provenance test that re-checks every"
  echo "    receipt. Label the test with what it PROVES (custody / transcription / analytic) and what it"
  echo "    does not."
  echo
  echo "If this is NOT a fetch agent (a read-only search or an analysis pass), say so in the prompt: the"
  echo "guard only fires on an acquisition verb plus an external-source noun."
} >&2
exit 2
