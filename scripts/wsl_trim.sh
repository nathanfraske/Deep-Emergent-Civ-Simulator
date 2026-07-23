#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Tell WSL which ext4 blocks are unused. This is online and non-destructive.
# It does not force sparse-VHD mode or stop the distribution.

set -uo pipefail

INTERVAL_HOURS="${WSL_TRIM_MIN_INTERVAL_HOURS:-168}"
FORCE=0
DRY_RUN=0
VERBOSE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --force) FORCE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --verbose|-v) VERBOSE=1; shift ;;
    -h|--help) sed -n '5,18p' "$0"; exit 0 ;;
    *) echo "wsl_trim: unknown argument $1" >&2; exit 2 ;;
  esac
done

say() {
  [ "$VERBOSE" = 1 ] && printf '%s\n' "$*"
  return 0
}

case "$INTERVAL_HOURS" in ''|*[!0-9]*) echo "wsl_trim: interval must be a nonnegative integer" >&2; exit 2;; esac
if [ -z "${WSL_DISTRO_NAME:-}" ]; then
  say "wsl_trim: not running under WSL; skipped"
  exit 0
fi
if pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1; then
  say "wsl_trim: a Rust build is running; skipped"
  exit 0
fi

STATE_DIR="${CIVSIM_MAINTENANCE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/civsim-dev/maintenance}"
mkdir -p "$STATE_DIR" 2>/dev/null || {
  echo "wsl_trim: cannot create maintenance state at $STATE_DIR" >&2
  exit 2
}
STATE_LEXICAL="$(realpath -ms "$STATE_DIR")" || exit 2
STATE_RESOLVED="$(realpath -m "$STATE_DIR")" || exit 2
if [ "$STATE_LEXICAL" != "$STATE_RESOLVED" ] || [ ! -d "$STATE_LEXICAL" ]; then
  echo "wsl_trim: refusing linked or non-directory maintenance state" >&2
  exit 2
fi
STATE_DIR="$STATE_LEXICAL"
STAMP="$STATE_DIR/wsl-trim.stamp"
if [ "$FORCE" = 0 ] && [ -f "$STAMP" ]; then
  now=$(date +%s)
  then=$(stat -c %Y "$STAMP" 2>/dev/null || echo 0)
  if [ $((now - then)) -lt $((INTERVAL_HOURS * 3600)) ]; then
    say "wsl_trim: maintenance is not due"
    exit 0
  fi
fi

WSL_EXE="${CIVSIM_WSL_EXE:-/mnt/c/Windows/System32/wsl.exe}"
FSTRIM="${CIVSIM_FSTRIM:-/usr/sbin/fstrim}"
if [ "$DRY_RUN" = 1 ]; then
  printf 'WOULD trim WSL distribution %s\n' "$WSL_DISTRO_NAME"
  exit 0
fi

if [ "$(id -u)" = 0 ] && [ -x "$FSTRIM" ]; then
  "$FSTRIM" -v / || {
    echo "wsl_trim: fstrim failed" >&2
    exit 2
  }
elif [ -x "$WSL_EXE" ] && [ -x "$FSTRIM" ]; then
  "$WSL_EXE" -d "$WSL_DISTRO_NAME" -u root -e "$FSTRIM" -v / || {
    status=$?
    echo "wsl_trim: root-capable WSL trim failed" >&2
    exit "$status"
  }
else
  if [ "$FORCE" = 1 ]; then
    echo "wsl_trim: forced trim requires a root-capable WSL launcher" >&2
    exit 2
  fi
  say "wsl_trim: root-capable WSL launcher is unavailable; skipped"
  exit 0
fi
touch "$STAMP" || {
  echo "wsl_trim: trim succeeded but its success stamp could not be written" >&2
  exit 2
}
