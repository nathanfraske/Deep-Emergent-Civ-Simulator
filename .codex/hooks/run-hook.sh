#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

set -uo pipefail

hook="${1:-}"
case "$hook" in
  substrate-first|session-start|customs-guard|workflow-panel-reminder|pipeline-status-guard|fetch-brief-guard|post-edit-check|stop-gate) ;;
  *)
    echo "Codex hook bridge: unknown hook '$hook'." >&2
    exit 2
    ;;
esac

root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "Codex hook bridge: no Git root is available." >&2
  exit 2
}
payload="$(cat)"
script="$root/.claude/hooks/$hook.sh"

if [ ! -f "$script" ]; then
  echo "Codex hook bridge: missing $script." >&2
  exit 2
fi

output="$(printf '%s' "$payload" | CLAUDE_PROJECT_DIR="$root" bash "$script")"
status=$?
if [ -n "$output" ]; then
  printf '%s\n' "$output"
elif [ "$hook" = "stop-gate" ] && [ "$status" -eq 0 ]; then
  printf '%s\n' '{"continue":true}'
fi
exit "$status"
