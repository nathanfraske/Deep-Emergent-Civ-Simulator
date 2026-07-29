#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PreToolUse customs guard (AGENTIC_ADDENDUM.md section 2b). The hard guard that an
# em dash or a banned adverb cannot be written into a maintained document in the
# first place. It inspects tool_input on stdin: if the target is
# parked/docs/design.md or parked/docs/audit.md and the incoming content carries a violation, it exits 2 with the
# reason, which the harness feeds back to the agent. It leaves the archived research
# papers and every other file alone. A PreToolUse deny blocks even under bypass
# mode, so this holds regardless of permission settings.
#
# The hook JSON is piped STRAIGHT INTO Python's stdin. The script itself rides in
# argv via -c, so stdin is free for the payload; the heredoc problem this guard used
# to dodge (a heredoc feeds the SCRIPT to stdin, leaving nowhere for the payload) does
# not apply to -c at all.
#
# IT USED TO PASS THE JSON IN AN ENVIRONMENT VARIABLE, AND THAT MADE THIS GUARD FAIL
# OPEN ON EXACTLY THE EDITS IT MATTERED MOST FOR. An env var is capped at
# MAX_ARG_STRLEN (32 pages, 131072 bytes), so any tool_input past ~128 KB made the
# exec fail with E2BIG, "argument list too long". The hook then exited 126, and a
# PreToolUse hook blocks ONLY on exit 2, so a non-2 exit is a non-blocking error: the
# guard vanished and the edit proceeded unchecked. Measured before the fix: a 147-byte
# payload carrying an em dash was blocked, a 25 KB payload was blocked, and a 200 KB
# payload FAILED OPEN and would have landed the em dash. The claim "a PreToolUse deny
# blocks even under bypass mode, so this holds regardless of permission settings" was
# true of every edit except the long ones, which are the ones most likely to hide a
# violation. A pipe has no such limit. Read from stdin; never from the environment.

set -u
python3 -c '
import json, re, sys

try:
    data = json.loads(sys.stdin.read() or "{}")
except Exception:
    sys.exit(0)

ti = data.get("tool_input", {}) or {}
def maintained(path):
    normalized = str(path).replace("\\", "/")
    return normalized.endswith(("parked/docs/design.md", "parked/docs/audit.md"))

parts = []
fp = ti.get("file_path", "") or ""
if maintained(fp):
    for key in ("content", "new_string", "new_str"):
        v = ti.get(key)
        if isinstance(v, str):
            parts.append(v)

# Codex apply_patch carries a multi-file patch in tool_input.command. Inspect
# only added lines while the active patch file is one of the maintained docs.
command = ti.get("command")
if isinstance(command, str):
    current = ""
    for line in command.splitlines():
        m = re.match(r"^\*\*\* (?:Add|Update|Delete) File:\s*(.+?)\s*$", line)
        if m:
            current = m.group(1)
            continue
        m = re.match(r"^\*\*\* Move to:\s*(.+?)\s*$", line)
        if m:
            current = m.group(1)
            continue
        if line == "*** End Patch":
            current = ""
            continue
        if maintained(current) and line.startswith("+") and not line.startswith("+++"):
            parts.append(line[1:])

if not parts:
    sys.exit(0)

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
