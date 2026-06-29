#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PreToolUse customs guard (AGENTIC_ADDENDUM.md section 2b). The hard guard that an
# em dash or a banned adverb cannot be written into a maintained document in the
# first place. It inspects tool_input on stdin: if the target is docs/design.md or
# docs/audit.md and the incoming content carries a violation, it exits 2 with the
# reason, which the harness feeds back to the agent. It leaves the archived research
# papers and every other file alone. A PreToolUse deny blocks even under bypass
# mode, so this holds regardless of permission settings.
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
fp = ti.get("file_path", "") or ""

# Only the two maintained documents are guarded.
if not (fp.endswith("docs/design.md") or fp.endswith("docs/audit.md")):
    sys.exit(0)

# Gather the candidate new content across the file-writing tools.
parts = []
for key in ("content", "new_string", "new_str"):
    v = ti.get(key)
    if isinstance(v, str):
        parts.append(v)
blob = "\n".join(parts)

violations = []
if "—" in blob:
    violations.append("em dash (U+2014)")
m = re.search(r"genuinely|honestly|\bactually\b", blob, re.I)
if m:
    violations.append("banned adverb \"%s\"" % m.group(0))

if violations:
    sys.stderr.write(
        "customs-guard blocked a write to a maintained document: "
        + "; ".join(violations)
        + ". Use commas, colons, parentheses, or semicolons instead of an em dash, "
        + "and avoid genuinely / honestly / actually in adverb form (CLAUDE.md section 3).\n"
    )
    sys.exit(2)

sys.exit(0)
'
