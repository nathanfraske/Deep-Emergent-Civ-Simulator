#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PreToolUse panel-types reminder for the Workflow tool (AGENTIC_ADDENDUM.md
# sections 7 through 10). The agent's panel-audit catalog is documentation it reads
# on demand, not context loaded verbatim at launch, so it is easy to author a panel
# workflow that forgets a standing type (one of the five section 9 lenses, or the
# section 10 blind framing panel). This hook surfaces the catalog at exactly the
# moment a panel workflow is authored.
#
# It fires ONLY for audit/review/panel-shaped Workflow calls (keyed on the script or
# name text, and on the file at scriptPath when a run is resumed), and it blocks once
# with the catalog, the same hard-guard idiom customs-guard and stop-gate use. A
# script that already carries the `panels-reviewed` acknowledgment passes untouched,
# so re-invoking after the reminder does not loop, and every non-panel workflow (a
# migration, a research sweep, a fan-out map) passes silently.
#
# The hook JSON is read from stdin into an environment variable so the Python below
# can be passed with -c; piping the JSON straight into a heredoc would feed the
# script to Python's stdin instead of the payload.

set -u
HOOK_INPUT="$(cat)" python3 -c '
import json, os, re, sys

try:
    data = json.loads(os.environ.get("HOOK_INPUT", "") or "{}")
except Exception:
    sys.exit(0)

ti = data.get("tool_input", {}) or {}
blob = ""
for key in ("script", "name"):
    v = ti.get(key)
    if isinstance(v, str):
        blob += "\n" + v
# A resumed run passes scriptPath instead of the inline script; read that file so the
# keyword and acknowledgment checks see the real content.
sp = ti.get("scriptPath")
if isinstance(sp, str) and sp:
    try:
        with open(sp, "r", encoding="utf-8", errors="ignore") as fh:
            blob += "\n" + fh.read()
    except Exception:
        pass

low = blob.lower()

# Only remind for audit / review / panel-shaped workflows.
if not re.search(r"audit|panel|lens|blind|adversar|confirmation.bias|framing", low):
    sys.exit(0)

# Already acknowledged: let it through so re-invoking never loops.
if "panels-reviewed" in low:
    sys.exit(0)

sys.stderr.write(
    "STANDING PANEL-AUDIT TYPES (AGENTIC_ADDENDUM.md; consult before a panel workflow):\n"
    "  section 7  fully-blind audit: code correctness from a packet, uncontaminated by the "
    "repo tests, comments, or prior reviews.\n"
    "  section 8  blind concept verification: is a concept ALIVE in the running world, judged "
    "from one run log alone.\n"
    "  section 9  the FIVE mandatory lenses (REQUIRED on every world-content change): "
    "confirmation-bias, derive-vs-author, alien-feasibility, Terran-bias, steering/Principles.\n"
    "  section 10 the blind FRAMING panel: critique a design-framing STATEMENT against the "
    "principles alone (diverse types + models, blind, de-narrativized) BEFORE it is built.\n"
    "  section 11 the INPUT-BIAS SMOKE TEST (required on any load-bearing panel): a dedicated agent "
    "on the STRONGEST model (Opus, maximum reasoning) audits the panel construction ITSELF, the packet "
    "or statement, the prompts, and the lens set, plus what you are hoping it concludes, for "
    "confirmation-bias shaping. Its charge is the negation of yours: what would a hostile outsider "
    "check that this setup omits? A panel that only looks for what you want shares your blind spot and "
    "reports clean, so do not trust a sound verdict until the smoke test has cleared the inputs.\n"
    "Confirm your workflow covers the relevant standing types (and, for a world-content audit, "
    "all five section 9 lenses run as independent panelists with per-finding verify; for any "
    "load-bearing panel, the section 11 smoke test on the construction before the verdict is trusted). "
    "Then add a comment line containing `panels-reviewed` near the top of the script and re-invoke.\n"
)
sys.exit(2)
'
