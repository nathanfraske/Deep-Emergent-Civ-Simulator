#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# PostToolUse per-edit check (AGENTIC_ADDENDUM.md section 2c). After a successful
# edit of a maintained document, it runs the fast checks on the changed file (em
# dashes, banned adverbs, fence balance) and surfaces any slip so the agent fixes it
# immediately rather than discovering it at the end. It cannot undo the edit; it
# reports. The hook JSON is piped straight into Python's stdin. It used to travel in an
# environment variable, which capped it at MAX_ARG_STRLEN (131072 bytes) and made this
# check die with E2BIG on any long edit, reporting "argument list too long" and
# checking nothing. A check that errors without checking is a check that did not run.

set -u
python3 -c '
import json, os, re, sys

try:
    data = json.loads(sys.stdin.read() or "{}")
except Exception:
    sys.exit(0)

ti = data.get("tool_input", {}) or {}
def maintained(path):
    normalized = str(path).replace("\\", "/")
    return normalized.endswith(("parked/docs/design.md", "parked/docs/audit.md"))

paths = []
fp = ti.get("file_path", "") or ""
if maintained(fp):
    paths.append(fp)

# Codex apply_patch may touch several files and supplies their names inside the
# patch command rather than tool_input.file_path.
command = ti.get("command")
if isinstance(command, str):
    for line in command.splitlines():
        m = re.match(r"^\*\*\* (?:Add|Update) File:\s*(.+?)\s*$", line)
        if not m:
            m = re.match(r"^\*\*\* Move to:\s*(.+?)\s*$", line)
        if m and maintained(m.group(1)):
            paths.append(m.group(1))

root = os.environ.get("CLAUDE_PROJECT_DIR", os.getcwd())
reports = []
for fp in dict.fromkeys(paths):
    normalized = fp.replace("\\", "/")
    if re.match(r"^[A-Za-z]:/", normalized):
        normalized = "/mnt/" + normalized[0].lower() + normalized[2:]
    elif not os.path.isabs(normalized):
        normalized = os.path.join(root, normalized)
    try:
        with open(normalized, encoding="utf-8") as fh:
            text = fh.read()
    except OSError:
        continue

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
        reports.append(fp + ": " + "; ".join(issues))

if reports:
    sys.stderr.write(
        "post-edit-check found issues in " + " | ".join(reports)
        + ". Fix before finishing (CLAUDE.md section 8).\n"
    )
    sys.exit(2)

sys.exit(0)
'
