#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PostToolUse per-edit check (AGENTIC_ADDENDUM.md section 2c). After a successful
# edit of a maintained document, it runs the fast checks on the changed file (em
# dashes, banned adverbs, fence balance) and surfaces any slip so the agent fixes it
# immediately rather than discovering it at the end. It cannot undo the edit; it
# reports. The hook JSON is read into an environment variable for the same reason as
# the customs guard.

set -u
HOOK_INPUT="$(cat)" python3 -c '
import json, os, re, sys

try:
    data = json.loads(os.environ.get("HOOK_INPUT", "") or "{}")
except Exception:
    sys.exit(0)

ti = data.get("tool_input", {}) or {}
fp = ti.get("file_path", "") or ""
if not (fp.endswith("docs/design.md") or fp.endswith("docs/audit.md")):
    sys.exit(0)

try:
    with open(fp, encoding="utf-8") as fh:
        text = fh.read()
except OSError:
    sys.exit(0)

issues = []
em = text.count("—")
if em:
    issues.append("%d em dash(es)" % em)
adv = re.findall(r"genuinely|honestly|\bactually\b", text, re.I)
if adv:
    uniq = ", ".join(sorted({a.lower() for a in adv}))
    issues.append("banned adverb(s): " + uniq)
if text.count("```") % 2:
    issues.append("unbalanced code fences")

if issues:
    sys.stderr.write(
        "post-edit-check found issues in " + fp + ": " + "; ".join(issues)
        + ". Fix before finishing (CLAUDE.md section 8).\n"
    )
    sys.exit(2)

sys.exit(0)
'
