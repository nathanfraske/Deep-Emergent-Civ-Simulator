#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Bounded, throttled maintenance used by the session-start hook.

set -uo pipefail
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"

if [ -n "${WSL_DISTRO_NAME:-}" ] && [ -f "$ROOT/scripts/wsl_dev_env.sh" ]; then
  # shellcheck source=wsl_dev_env.sh
  source "$ROOT/scripts/wsl_dev_env.sh" --quiet || {
    echo "session_maintenance: WSL environment validation failed" >&2
    exit 2
  }
fi

status=0
bash "$ROOT/scripts/target_gc.sh" --if-due --verbose || status=$?
trim_status=0
bash "$ROOT/scripts/wsl_trim.sh" --verbose || trim_status=$?
[ "$status" != 0 ] || status="$trim_status"
exit "$status"
