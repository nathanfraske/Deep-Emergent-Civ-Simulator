#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Configure one bounded, native-filesystem build cache per repository clone.
# Source this file before invoking Cargo from a checkout mounted through /mnt.

civsim_dev_env_main() {
  local print_values=0
  if [ "${1:-}" = "--print" ]; then
    print_values=1
  elif [ "${1:-}" != "" ] && [ "${1:-}" != "--quiet" ]; then
    echo "wsl_dev_env: unknown argument ${1:-}" >&2
    return 2
  fi

  local root common key cache_base repo_cache managed marker old_umask env_lock_fd target_status maintenance_lexical maintenance_resolved
  root="${CIVSIM_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
  common="$(git -C "$root" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || {
    echo "wsl_dev_env: repository common directory is unavailable" >&2
    return 2
  }
  common="$(realpath -m "$common")"
  case "$common" in
    *$'\n'*|*$'\r'*)
      echo "wsl_dev_env: repository paths containing line breaks are unsupported" >&2
      return 2
      ;;
  esac
  key="$(printf '%s\0' "$common" | sha256sum | cut -c1-16)"
  cache_base="${CIVSIM_WSL_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/civsim-dev}"
  cache_base="$(realpath -m "$cache_base")"
  case "$cache_base" in
    /|"$HOME")
      echo "wsl_dev_env: refusing unsafe cache root $cache_base" >&2
      return 2
      ;;
  esac

  repo_cache="$cache_base/state/$key"
  managed="$cache_base/targets/$key"
  marker="$managed/.civsim-managed-target-v1"
  old_umask="$(umask)"
  umask 077
  if ! mkdir -p "$repo_cache/gates" "$cache_base/maintenance" \
    "$cache_base/sccache" "$cache_base/targets"; then
    umask "$old_umask"
    return 2
  fi
  maintenance_lexical="$(realpath -ms "$cache_base/maintenance")" || {
    umask "$old_umask"
    return 2
  }
  maintenance_resolved="$(realpath -m "$cache_base/maintenance")" || {
    umask "$old_umask"
    return 2
  }
  if [ "$maintenance_lexical" != "$maintenance_resolved" ] || [ ! -d "$maintenance_lexical" ]; then
    echo "wsl_dev_env: refusing linked or non-directory maintenance state" >&2
    umask "$old_umask"
    return 2
  fi
  command -v flock >/dev/null 2>&1 || {
    echo "wsl_dev_env: flock is required for build-safe target setup" >&2
    umask "$old_umask"
    return 2
  }
  if ! exec {env_lock_fd}< "$maintenance_lexical"; then
    echo "wsl_dev_env: cannot acquire the artifact setup lock" >&2
    umask "$old_umask"
    return 2
  fi
  if ! flock -s "$env_lock_fd"; then
    echo "wsl_dev_env: cannot acquire the artifact setup lock" >&2
    exec {env_lock_fd}>&-
    umask "$old_umask"
    return 2
  fi
  target_status=0
  (
    marker_tmp=""
    if [ -L "$managed" ] || { [ -e "$managed" ] && [ ! -d "$managed" ]; }; then
      echo "wsl_dev_env: refusing non-directory or linked managed target $managed" >&2
      exit 2
    fi
    if [ ! -e "$managed" ] && ! mkdir "$managed"; then
      exit 2
    fi
    if [ -L "$marker" ]; then
      echo "wsl_dev_env: refusing linked managed target marker $marker" >&2
      exit 2
    elif [ -f "$marker" ]; then
      if ! cmp -s "$marker" <(printf 'schema=1\nrepository=%s\nkey=%s\n' "$common" "$key"); then
        echo "wsl_dev_env: managed target marker does not match this repository" >&2
        exit 2
      fi
    else
      first_entry="$(find "$managed" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" || exit 2
      if [ -n "$first_entry" ]; then
        echo "wsl_dev_env: refusing to adopt nonempty unmarked target $managed" >&2
        exit 2
      fi
      marker_tmp="$(mktemp "$managed/.civsim-managed-target-v1.XXXXXX")" || exit 2
      if ! printf 'schema=1\nrepository=%s\nkey=%s\n' "$common" "$key" > "$marker_tmp" || \
        ! mv -f -- "$marker_tmp" "$marker"; then
        rm -f -- "$marker_tmp"
        exit 2
      fi
    fi
    mkdir -p "$managed/canonical" "$managed/parked"
  ) || target_status=$?
  if ! flock -u "$env_lock_fd"; then
    target_status=2
  fi
  exec {env_lock_fd}>&-
  umask "$old_umask"
  [ "$target_status" = 0 ] || return "$target_status"

  export CIVSIM_REPO_CACHE_DIR="$repo_cache"
  export CIVSIM_GATE_CACHE_DIR="$repo_cache/gates"
  # Collection spans every marked clone target, so its lock and throttle state
  # must be global too. Per-clone locks could otherwise prune the same cache at
  # the same time.
  export CIVSIM_MAINTENANCE_DIR="$maintenance_lexical"
  export CIVSIM_MANAGED_TARGET_ROOT="$managed"
  export CARGO_TARGET_DIR="$managed/canonical"
  export CIVSIM_PARKED_TARGET_DIR="$managed/parked"
  export TARGET_GC_CAP_GB="${TARGET_GC_CAP_GB:-32}"
  export TARGET_GC_KEEP="${TARGET_GC_KEEP:-1}"
  export TARGET_GC_STALE_DAYS="${TARGET_GC_STALE_DAYS:-7}"
  export TARGET_GC_MIN_INTERVAL_HOURS="${TARGET_GC_MIN_INTERVAL_HOURS:-12}"
  export CIVSIM_GATE_JOBS="${CIVSIM_GATE_JOBS:-4}"
  touch "$repo_cache/last-used" || return 2

  if command -v sccache >/dev/null 2>&1; then
    export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
    export SCCACHE_DIR="${SCCACHE_DIR:-$cache_base/sccache}"
    export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-8G}"
  fi

  if [ "$print_values" = 1 ]; then
    printf 'repository cache: %s\n' "$CIVSIM_REPO_CACHE_DIR"
    printf 'canonical target: %s\n' "$CARGO_TARGET_DIR"
    printf 'parked target:    %s\n' "$CIVSIM_PARKED_TARGET_DIR"
    printf 'target hard cap:  %s GiB\n' "$TARGET_GC_CAP_GB"
    printf 'gate workers:     %s\n' "$CIVSIM_GATE_JOBS"
    if [ -n "${RUSTC_WRAPPER:-}" ]; then
      printf 'compiler cache:   %s (%s max)\n' "$SCCACHE_DIR" "$SCCACHE_CACHE_SIZE"
    else
      printf 'compiler cache:   unavailable (sccache is optional)\n'
    fi
  fi
}

civsim_dev_env_main "$@"
