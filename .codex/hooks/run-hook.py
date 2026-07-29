#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

"""Run an authoritative Claude hook under WSL for native Windows Codex."""

from __future__ import annotations

import os
import pathlib
import shutil
import subprocess
import sys


HOOKS = {
    "substrate-first",
    "session-start",
    "customs-guard",
    "workflow-panel-reminder",
    "pipeline-status-guard",
    "fetch-brief-guard",
    "post-edit-check",
    "stop-gate",
}


def main() -> int:
    if len(sys.argv) != 2 or sys.argv[1] not in HOOKS:
        print("Codex hook bridge: unknown or missing hook.", file=sys.stderr)
        return 2

    wsl = shutil.which("wsl.exe")
    if not wsl:
        print("Codex hook bridge: WSL is required on Windows.", file=sys.stderr)
        return 2

    hook = sys.argv[1]
    root = pathlib.Path(__file__).resolve().parents[2]
    env = os.environ.copy()
    env["CLAUDE_PROJECT_DIR"] = str(root)
    inherited = env.get("WSLENV", "")
    entries = [entry for entry in inherited.split(":") if entry]
    if "CLAUDE_PROJECT_DIR/p" not in entries:
        entries.append("CLAUDE_PROJECT_DIR/p")
    env["WSLENV"] = ":".join(entries)

    payload = sys.stdin.buffer.read()
    code = 'exec bash "$CLAUDE_PROJECT_DIR/.claude/hooks/$1.sh"'
    result = subprocess.run(
        [wsl, "-e", "bash", "-c", code, "bash", hook],
        cwd=root,
        env=env,
        input=payload,
        stdout=subprocess.PIPE,
        check=False,
    )
    if result.stdout:
        sys.stdout.buffer.write(result.stdout)
    elif hook == "stop-gate" and result.returncode == 0:
        sys.stdout.write('{"continue":true}\n')
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
