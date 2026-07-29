#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# The local CI entrypoint is intentionally thin. GitHub Actions invokes the same
# `just ci` recipe, so there is no second command list to drift or a partial YAML
# extractor that can silently omit a multiline step.

set -euo pipefail
ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$ROOT"

if [ "${1:-}" = "--list" ]; then
  printf '%s\n' '# canonical ci invokes this recipe with tier=pr'
  just --show _ci
  exit 0
fi
if [ "${1:-}" = "--list-gates" ]; then
  tier="${2:-pr}"
  if [ "$#" -gt 2 ]; then
    echo "usage: scripts/ci_local.sh [--list | --list-gates [tier]]" >&2
    exit 2
  fi
  exec python3 scripts/gate_runner.py list --tier "$tier" --ids-only
fi
if [ "$#" -ne 0 ]; then
  echo "usage: scripts/ci_local.sh [--list | --list-gates [tier]]" >&2
  exit 2
fi

exec just ci
