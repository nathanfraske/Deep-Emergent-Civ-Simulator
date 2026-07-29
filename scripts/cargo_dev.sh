#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Keep every local Cargo entrypoint on the bounded native WSL target. Native
# Linux and CI retain Cargo's ordinary target selection. A collector runs after
# WSL commands so direct Just sessions receive the same cap enforcement
# as agent session hooks.

set -euo pipefail

ROOT="${CIVSIM_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
if [ -n "${WSL_DISTRO_NAME:-}" ]; then
  cache_base="${CIVSIM_WSL_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/civsim-dev}"
  maintenance_dir="${CIVSIM_MAINTENANCE_DIR:-$(realpath -m "$cache_base")/maintenance}"
  mkdir -p "$maintenance_dir" || exit 2
  maintenance_lexical="$(realpath -ms "$maintenance_dir")" || exit 2
  maintenance_resolved="$(realpath -m "$maintenance_dir")" || exit 2
  if [ "$maintenance_lexical" != "$maintenance_resolved" ] || [ ! -d "$maintenance_lexical" ]; then
    echo "cargo_dev: refusing linked or non-directory maintenance state" >&2
    exit 2
  fi
  maintenance_dir="$maintenance_lexical"
  command -v flock >/dev/null 2>&1 || {
    echo "cargo_dev: flock is required for build-safe collection" >&2
    exit 2
  }
  # Lock the validated directory itself. A separate lock file could be a stale
  # symlink, and shell output redirection would truncate its destination.
  exec 8< "$maintenance_dir" || exit 2
  flock -s 8 || exit 2
  # Always recompute and validate the repository marker while the shared build
  # lock is held. An inherited environment can outlive a collected target.
  # shellcheck source=wsl_dev_env.sh
  source "$ROOT/scripts/wsl_dev_env.sh" --quiet
fi

set +e
"${CARGO:-cargo}" "$@"
status=$?
set -e

if [ -n "${WSL_DISTRO_NAME:-}" ] && [ -f "$ROOT/scripts/target_gc.sh" ]; then
  maintenance_status=0
  flock -u 8 || maintenance_status=2
  exec 8>&-
  bash "$ROOT/scripts/target_gc.sh" || maintenance_status=$?
  if [ "$status" = 0 ] && [ "$maintenance_status" != 0 ]; then
    status="$maintenance_status"
  fi
fi

exit "$status"
