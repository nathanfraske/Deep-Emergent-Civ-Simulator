#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Bound Cargo artifacts without deleting under a live build. The Windows bridge
# uses one native-WSL target per clone, shared by its worktrees. Old per-worktree
# targets remain eviction candidates until they have been drained.

set -uo pipefail

CAP_GB="${TARGET_GC_CAP_GB:-32}"
CAP_MB_OVERRIDE="${TARGET_GC_CAP_MB:-}"
KEEP="${TARGET_GC_KEEP:-1}"
STALE_DAYS="${TARGET_GC_STALE_DAYS:-7}"
MIN_INTERVAL_HOURS="${TARGET_GC_MIN_INTERVAL_HOURS:-12}"
DRY_RUN=0
IF_DUE=0
VERBOSE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --cap-gb|--cap-mb|--keep|--stale-days)
      option="$1"
      if [ $# -lt 2 ]; then
        echo "target_gc: $option requires a value" >&2
        exit 2
      fi
      value="$2"
      case "$option" in
        --cap-gb) CAP_GB="$value" ;;
        --cap-mb) CAP_MB_OVERRIDE="$value" ;;
        --keep) KEEP="$value" ;;
        --stale-days) STALE_DAYS="$value" ;;
      esac
      shift 2
      ;;
    --if-due) IF_DUE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --verbose|-v) VERBOSE=1; shift ;;
    -h|--help)
      sed -n '5,30p' "$0"
      exit 0
      ;;
    *) echo "target_gc: unknown argument $1" >&2; exit 2 ;;
  esac
done

for value in "$CAP_GB" "$KEEP" "$STALE_DAYS" "$MIN_INTERVAL_HOURS"; do
  case "$value" in ''|*[!0-9]*) echo "target_gc: limits must be nonnegative integers" >&2; exit 2;; esac
done
if [ -n "$CAP_MB_OVERRIDE" ]; then
  case "$CAP_MB_OVERRIDE" in ''|*[!0-9]*) echo "target_gc: --cap-mb must be a nonnegative integer" >&2; exit 2;; esac
  CAP_MB="$CAP_MB_OVERRIDE"
else
  CAP_MB=$((CAP_GB * 1024))
fi
[ "$CAP_MB" -gt 0 ] || { echo "target_gc: size cap must be positive" >&2; exit 2; }

ROOT="${TARGET_GC_REPO_ROOT:-${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}}"
ROOT="$(realpath -m "$ROOT")"
cd "$ROOT" 2>/dev/null || {
  echo "target_gc: repository root is unavailable: $ROOT" >&2
  exit 2
}

# Direct WSL callers receive the same native target layout as scripts/dev.ps1.
if [ -n "${WSL_DISTRO_NAME:-}" ] && [ -f "$ROOT/scripts/wsl_dev_env.sh" ] && [ -z "${CIVSIM_MANAGED_TARGET_ROOT:-}" ]; then
  # shellcheck source=wsl_dev_env.sh
  source "$ROOT/scripts/wsl_dev_env.sh" --quiet || exit 2
fi

COMMON_GIT="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"
COMMON_GIT="$(realpath -m "${COMMON_GIT:-$ROOT/.git}")"
STATE_DIR="${CIVSIM_MAINTENANCE_DIR:-$COMMON_GIT/civsim-cache/maintenance}"
mkdir -p "$STATE_DIR" 2>/dev/null || {
  echo "target_gc: cannot create maintenance state at $STATE_DIR" >&2
  exit 2
}
STATE_LEXICAL="$(realpath -ms "$STATE_DIR")" || exit 2
STATE_RESOLVED="$(realpath -m "$STATE_DIR")" || exit 2
if [ "$STATE_LEXICAL" != "$STATE_RESOLVED" ] || [ ! -d "$STATE_LEXICAL" ]; then
  echo "target_gc: refusing linked or non-directory maintenance state" >&2
  exit 2
fi
STATE_DIR="$STATE_LEXICAL"
STAMP="$STATE_DIR/target-gc.stamp"

say() {
  [ "$VERBOSE" = 1 ] && printf '%s\n' "$*"
  return 0
}

if [ "$IF_DUE" = 1 ] && [ -f "$STAMP" ]; then
  now=$(date +%s)
  then=$(stat -c %Y "$STAMP" 2>/dev/null || echo 0)
  if [ $((now - then)) -lt $((MIN_INTERVAL_HOURS * 3600)) ]; then
    say "target_gc: maintenance is not due"
    exit 0
  fi
fi

command -v flock >/dev/null 2>&1 || {
  echo "target_gc: flock is required for build-safe collection" >&2
  exit 2
}
exec 9< "$STATE_DIR" || {
  echo "target_gc: cannot open the artifact lock" >&2
  exit 2
}
if ! flock -n 9; then
  say "target_gc: a build or another collector holds the artifact lock"
  exit 0
fi

tmp="$(mktemp)" || {
  echo "target_gc: could not allocate measurement state" >&2
  exit 2
}
worktree_tmp="$(mktemp)" || {
  rm -f -- "$tmp"
  echo "target_gc: could not allocate worktree discovery state" >&2
  exit 2
}
cleanup() {
  rm -f -- "$tmp" "$worktree_tmp"
}
trap cleanup EXIT
trap 'exit 130' INT TERM

build_running() {
  pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1
}

if build_running; then
  if [ "$DRY_RUN" = 1 ]; then
    echo "target_gc: a build is running; dry-run reporting continues"
  else
    say "target_gc: a build is running; standing down"
    exit 0
  fi
fi

locked_paths=""
current_worktree=""
current_locked=0
current_prunable=0
WORKTREES=()
finish_worktree_record() {
  [ -n "$current_worktree" ] || return 0
  if [ "$current_prunable" = 0 ]; then
    WORKTREES+=("$current_worktree")
    [ "$current_locked" = 0 ] || locked_paths="$locked_paths|$current_worktree|"
  fi
  current_worktree=""
  current_locked=0
  current_prunable=0
}
if ! git -C "$ROOT" worktree list --porcelain > "$worktree_tmp" 2>/dev/null; then
  echo "target_gc: could not enumerate repository worktrees" >&2
  exit 2
fi
while IFS= read -r line; do
  case "$line" in
    "worktree "*)
      finish_worktree_record
      current_worktree="${line#worktree }"
      case "$current_worktree" in
        [A-Za-z]:/*)
          current_worktree="$(wslpath -u "$current_worktree" 2>/dev/null)" || {
            echo "target_gc: could not translate worktree path ${line#worktree }" >&2
            exit 2
          }
          ;;
      esac
      current_worktree="$(realpath -m "$current_worktree")"
      ;;
    locked*) current_locked=1 ;;
    prunable*) current_prunable=1 ;;
    "") finish_worktree_record ;;
  esac
done < "$worktree_tmp"
finish_worktree_record
[ "${#WORKTREES[@]}" -gt 0 ] || {
  echo "target_gc: repository worktree inventory is empty" >&2
  exit 2
}

is_locked() {
  local worktree="$1" name lock_file pid wanted actual
  case "$locked_paths" in *"|$worktree|"*) ;; *) return 1;; esac
  lock_file="$(git -C "$worktree" rev-parse --path-format=absolute --git-path locked 2>/dev/null || true)"
  if [ -z "$lock_file" ]; then
    name=$(basename "$worktree")
    lock_file="$COMMON_GIT/worktrees/$name/locked"
  fi
  [ -f "$lock_file" ] || return 0
  pid=$(sed -n 's/.*pid \([0-9][0-9]*\).*/\1/p' "$lock_file" 2>/dev/null | head -1)
  [ -n "$pid" ] || return 0
  kill -0 "$pid" 2>/dev/null || return 1
  wanted=$(sed -n 's/.*start \([0-9][0-9]*\).*/\1/p' "$lock_file" 2>/dev/null | head -1)
  [ -n "$wanted" ] || return 0
  actual=$(awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
  [ -n "$actual" ] && [ "$wanted" = "$actual" ]
}

worktree_owned() {
  local worktree="$1" top common
  top="$(git -C "$worktree" rev-parse --path-format=absolute --show-toplevel 2>/dev/null)" || return 1
  common="$(git -C "$worktree" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || return 1
  top="$(realpath -m "$top")" || return 1
  common="$(realpath -m "$common")" || return 1
  [ "$top" = "$worktree" ] && [ "$common" = "$COMMON_GIT" ]
}

MANAGED_BASE="$(realpath -m "${CIVSIM_WSL_CACHE_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/civsim-dev}/targets")"
MANAGED_TARGET="${CIVSIM_MANAGED_TARGET_ROOT:-}"
DISCOVERY_ERROR=0
if [ -n "$MANAGED_TARGET" ]; then
  if [ -L "$MANAGED_TARGET" ]; then
    echo "target_gc: refusing linked managed target $MANAGED_TARGET" >&2
    DISCOVERY_ERROR=1
    MANAGED_TARGET=""
  else
    MANAGED_TARGET="$(realpath -m "$MANAGED_TARGET")"
    case "$MANAGED_TARGET" in
      "$MANAGED_BASE"/*) ;;
      *)
        echo "target_gc: managed target is outside the managed base" >&2
        DISCOVERY_ERROR=1
        ;;
    esac
  fi
fi
ACTIVE_TARGET="$(realpath -m "${CARGO_TARGET_DIR:-$ROOT/target}")"
PROTECTED_TARGET="$ACTIVE_TARGET"
if [ -n "$MANAGED_TARGET" ]; then
  case "$ACTIVE_TARGET/" in "$MANAGED_TARGET/"*) PROTECTED_TARGET="$MANAGED_TARGET";; esac
fi

TARGETS=()
TOMBS=()
add_target() {
  local candidate="$1" lexical resolved known
  lexical="$(realpath -ms "$candidate")" || {
    echo "target_gc: cannot normalize target candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  }
  resolved="$(realpath -m "$candidate")" || {
    echo "target_gc: cannot resolve target candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  }
  if [ "$lexical" != "$resolved" ]; then
    echo "target_gc: refusing target with linked ancestry $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  candidate="$lexical"
  if [ -L "$candidate" ]; then
    echo "target_gc: refusing linked target candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  if [ -e "$candidate" ] && [ ! -d "$candidate" ]; then
    echo "target_gc: refusing non-directory target candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  [ -d "$candidate" ] || return 0
  for known in "${TARGETS[@]:-}"; do
    [ "$known" = "$candidate" ] && return 0
  done
  TARGETS+=("$candidate")
}

managed_marker_valid_for_key() {
  local target="$1" expected_key="$2" marker repository key lines derived_key
  marker="$target/.civsim-managed-target-v1"
  [ -f "$marker" ] && [ ! -L "$marker" ] || return 1
  lines=$(wc -l < "$marker" 2>/dev/null) || return 1
  [ "$lines" = 3 ] || return 1
  grep -qx 'schema=1' "$marker" || return 1
  repository=$(sed -n 's/^repository=//p' "$marker")
  key=$(sed -n 's/^key=//p' "$marker")
  [ -n "$repository" ] || return 1
  printf '%s\n' "$key" | grep -Eq '^[0-9a-f]{16}$' || return 1
  derived_key="$(printf '%s\0' "$repository" | sha256sum | cut -c1-16)"
  [ "$derived_key" = "$key" ] && [ "$expected_key" = "$key" ]
}

managed_marker_valid() {
  managed_marker_valid_for_key "$1" "$(basename "$1")"
}

target_origin_safe() {
  local target="$1" worktree
  if [ "$(dirname "$target")" = "$MANAGED_BASE" ]; then
    managed_marker_valid "$target"
    return
  fi
  for worktree in "${WORKTREES[@]}"; do
    if [ "$target" = "$worktree/target" ] || [ "$target" = "$worktree/parked/target" ]; then
      worktree_owned "$worktree"
      return
    fi
  done
  return 1
}

tomb_origin_safe() {
  local tomb="$1" origin="$2" worktree key target
  [ "$origin" = "$ROOT/target" ] && return 0
  [ "$origin" = "$ROOT/parked/target" ] && return 0
  for worktree in "${WORKTREES[@]}"; do
    if [ "$origin" = "$worktree/target" ] || [ "$origin" = "$worktree/parked/target" ]; then
      worktree_owned "$worktree"
      return
    fi
  done
  case "$origin" in
    "$MANAGED_BASE"/*)
      if [ "$(dirname "$origin")" = "$MANAGED_BASE" ]; then
        key=$(basename "$origin")
        printf '%s\n' "$key" | grep -Eq '^[0-9a-f]{16}$' || return 1
        managed_marker_valid_for_key "$tomb" "$key"
        return
      fi
      ;;
  esac
  # Incremental pruning retires a directory beside its origin. Accept only the
  # exact nested name that the collector itself discovers beneath a known,
  # already guarded target root.
  if [ "$(basename "$origin")" = incremental ]; then
    for target in "${TARGETS[@]:-}"; do
      case "$origin/" in "$target/"*) target_origin_safe "$target"; return;; esac
    done
  fi
  return 1
}

add_tomb() {
  local candidate="$1" lexical resolved marker origin lines known expected_prefix
  [ -e "$candidate" ] || [ -L "$candidate" ] || return 0
  lexical="$(realpath -ms "$candidate")" || {
    echo "target_gc: cannot normalize tomb candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  }
  resolved="$(realpath -m "$candidate")" || {
    echo "target_gc: cannot resolve tomb candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  }
  if [ "$lexical" != "$resolved" ]; then
    echo "target_gc: refusing tomb with linked ancestry $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  candidate="$lexical"
  if [ -L "$candidate" ] || [ ! -d "$candidate" ]; then
    echo "target_gc: refusing linked or non-directory tomb candidate $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  marker="$candidate/.civsim-gc-tomb-v1"
  lines=$(wc -l < "$marker" 2>/dev/null) || lines=0
  origin=$(sed -n 's/^origin=//p' "$marker" 2>/dev/null)
  expected_prefix="$(dirname "$origin")/.civsim-gc-$(basename "$origin")-"
  if [ "$lines" != 2 ] || ! grep -qx 'schema=1' "$marker" 2>/dev/null || \
    [ -z "$origin" ] || [[ "$candidate" != "$expected_prefix"* ]] || \
    ! tomb_origin_safe "$candidate" "$origin"; then
    echo "target_gc: preserving unmarked or invalid tomb lookalike $candidate" >&2
    DISCOVERY_ERROR=1
    return 0
  fi
  for known in "${TOMBS[@]:-}"; do
    [ "$known" = "$candidate" ] && return 0
  done
  TOMBS+=("$candidate")
}

add_target "$ROOT/target"
add_target "$ROOT/parked/target"
for worktree in "${WORKTREES[@]}"; do
  add_target "$worktree/target"
  add_target "$worktree/parked/target"
  for tomb in "$worktree"/.civsim-gc-*; do
    add_tomb "$tomb"
  done
  for tomb in "$worktree/parked"/.civsim-gc-*; do
    add_tomb "$tomb"
  done
done
for tomb in "$ROOT/parked"/.civsim-gc-* "$MANAGED_BASE"/.civsim-gc-* "$MANAGED_BASE"/*.civsim-gc-*; do
  add_tomb "$tomb"
done
[ -n "$MANAGED_TARGET" ] && add_target "$MANAGED_TARGET"

# All clones share one global target budget. Only exact, direct-child markers
# created by wsl_dev_env are eligible for collection.
for target in "$MANAGED_BASE"/*; do
  [ -e "$target" ] || [ -L "$target" ] || continue
  case "$(basename "$target")" in
    *.civsim-gc-*) continue ;;
  esac
  if [ -L "$target" ]; then
    echo "target_gc: refusing linked managed-cache child $target" >&2
    DISCOVERY_ERROR=1
  elif [ -f "$target/.civsim-managed-target-v1" ]; then
    if managed_marker_valid "$target"; then
      add_target "$target"
    else
      echo "target_gc: preserving managed target with an invalid marker $target" >&2
      DISCOVERY_ERROR=1
    fi
  elif printf '%s\n' "$(basename "$target")" | grep -Eq '^[0-9a-f]{16}$'; then
    echo "target_gc: preserving unmarked managed-target-shaped directory $target" >&2
    DISCOVERY_ERROR=1
  fi
done

# Interrupted incremental deletion tombs live inside a guarded target rather
# than beside it. Discover them without following links and validate their
# exact marker and origin before they become deletion candidates.
for target in "${TARGETS[@]:-}"; do
  if ! find "$target" -mindepth 1 -name '.civsim-gc-*' -prune -print0 > "$worktree_tmp" 2>/dev/null; then
    echo "target_gc: could not enumerate nested deletion tombs in $target" >&2
    DISCOVERY_ERROR=1
    continue
  fi
  while IFS= read -r -d '' tomb; do
    add_tomb "$tomb"
  done < "$worktree_tmp"
done

if [ "$DISCOVERY_ERROR" != 0 ]; then
  echo "target_gc: unsafe candidates were found; no destructive action taken" >&2
  exit 2
fi

freed_mb=0
tomb_total_mb=0
tomb_count=0
for tomb in "${TOMBS[@]}"; do
  tomb_origin=$(sed -n 's/^origin=//p' "$tomb/.civsim-gc-tomb-v1" 2>/dev/null)
  if [ -z "$tomb_origin" ] || ! tomb_origin_safe "$tomb" "$tomb_origin"; then
    echo "target_gc: tomb ownership changed before deletion: $tomb" >&2
    exit 2
  fi
  tomb_size=$(du -sm "$tomb" 2>/dev/null | cut -f1) || {
    echo "target_gc: could not measure interrupted deletion $tomb" >&2
    exit 2
  }
  [ -n "$tomb_size" ] || exit 2
  tomb_total_mb=$((tomb_total_mb + tomb_size))
  tomb_count=$((tomb_count + 1))
  if [ "$DRY_RUN" = 1 ]; then
    printf 'WOULD retry interrupted deletion %s\n' "$tomb"
  elif build_running; then
    say "target_gc: a direct build appeared; standing down"
    exit 0
  elif rm -rf -- "$tomb" && [ ! -e "$tomb" ]; then
    freed_mb=$((freed_mb + tomb_size))
    printf 'target_gc: completed interrupted deletion %s\n' "$tomb"
  else
    echo "target_gc: could not complete interrupted deletion $tomb" >&2
    exit 2
  fi
done

target_locked() {
  local target="$1" worktree
  for worktree in "${WORKTREES[@]}"; do
    if [ "$target" = "$worktree/target" ] || [ "$target" = "$worktree/parked/target" ]; then
      is_locked "$worktree"
      return
    fi
  done
  return 1
}

deletion_path_safe() {
  local path="$1" lexical resolved target
  lexical="$(realpath -ms "$path")" || return 1
  resolved="$(realpath -m "$path")" || return 1
  [ "$lexical" = "$resolved" ] || return 1
  for target in "${TARGETS[@]:-}"; do
    if [ "$path" = "$target" ]; then
      target_origin_safe "$target"
      return
    fi
    case "$path/" in
      "$target/"*)
        target_origin_safe "$target"
        return
        ;;
    esac
  done
  return 1
}

target_safe() {
  local target="$1" known
  [ -d "$target" ] || return 1
  [ ! -L "$target" ] || return 1
  case "$target" in /|"$HOME"|"$ROOT") return 1;; esac
  for known in "${TARGETS[@]:-}"; do
    [ "$known" = "$target" ] && return 0
  done
  return 1
}

measure_targets() {
  : > "$tmp" || return 2
  total_mb=0
  evictable_count=0
  target_count=0
  local target size mtime protected locked key last_used
  for target in "${TARGETS[@]:-}"; do
    [ -d "$target" ] || continue
    size=$(du -sm "$target" 2>/dev/null | cut -f1) || {
      echo "target_gc: could not measure $target" >&2
      return 2
    }
    [ -n "$size" ] || {
      echo "target_gc: measurement returned no size for $target" >&2
      return 2
    }
    mtime=""
    if [ "$(dirname "$target")" = "$MANAGED_BASE" ] && managed_marker_valid "$target"; then
      key=$(sed -n 's/^key=//p' "$target/.civsim-managed-target-v1")
      last_used="$(dirname "$MANAGED_BASE")/state/$key/last-used"
      [ -f "$last_used" ] && mtime=$(stat -c %Y "$last_used" 2>/dev/null || true)
    fi
    [ -n "$mtime" ] && : || mtime=$(stat -c %Y "$target" 2>/dev/null) || {
      echo "target_gc: could not read the age of $target" >&2
      return 2
    }
    protected=0
    locked=0
    [ "$target" = "$PROTECTED_TARGET" ] && protected=1
    target_locked "$target" && locked=1
    total_mb=$((total_mb + size))
    target_count=$((target_count + 1))
    if [ "$protected" = 0 ] && [ "$locked" = 0 ]; then
      evictable_count=$((evictable_count + 1))
    fi
    printf '%s\t%s\t%s\t%s\t%s\n' "$mtime" "$size" "$protected" "$locked" "$target" >> "$tmp" || return 2
  done
}

remove_directory() {
  local path="$1" tomb marker marker_tmp
  [ -d "$path" ] || return 0
  deletion_path_safe "$path" || return 4
  if [ "$DRY_RUN" = 1 ]; then
    return 0
  fi
  build_running && return 3
  tomb="$(dirname "$path")/.civsim-gc-$(basename "$path")-$$"
  [ ! -e "$tomb" ] && [ ! -L "$tomb" ] || return 2
  case "$path" in *$'\n'*|*$'\r'*) return 2;; esac
  marker="$path/.civsim-gc-tomb-v1"
  marker_tmp="$(mktemp "$path/.civsim-gc-tomb-v1.XXXXXX")" || return 2
  if ! printf 'schema=1\norigin=%s\n' "$path" > "$marker_tmp" || ! mv -f -- "$marker_tmp" "$marker"; then
    rm -f -- "$marker_tmp"
    return 2
  fi
  if ! mv -- "$path" "$tomb"; then
    rm -f -- "$marker"
    return 2
  fi
  rm -rf -- "$tomb" && [ ! -e "$tomb" ]
}

handle_remove_failure() {
  local path="$1" status="$2"
  if [ "$status" = 3 ]; then
    say "target_gc: a direct build appeared; cleanup stopped"
    exit 0
  fi
  if [ "$status" = 4 ]; then
    echo "target_gc: ownership or link state changed before deleting $path" >&2
    exit 2
  fi
  echo "target_gc: failed to remove $path; any retired tree remains marked for retry" >&2
  exit 2
}

stamp_success() {
  [ "$DRY_RUN" = 1 ] && return 0
  touch "$STAMP" || {
    echo "target_gc: could not write the success stamp" >&2
    exit 2
  }
}

# Once the marked native target is active, repo-local targets are retired. They
# cannot accelerate a later bridged build, so drain each unlocked tree without
# paying for a recursive size walk first. Locked legacy worktrees are deferred.
if [ -n "$MANAGED_TARGET" ] && [ "$PROTECTED_TARGET" = "$MANAGED_TARGET" ]; then
  retained_targets=()
  for target in "${TARGETS[@]:-}"; do
    case "$target" in
      "$MANAGED_BASE"/*)
        retained_targets+=("$target")
        continue
        ;;
    esac
    if [ "$target" = "$MANAGED_TARGET" ]; then
      retained_targets+=("$target")
      continue
    fi
    if target_locked "$target"; then
      say "target_gc: deferred locked retired target $target"
      retained_targets+=("$target")
      continue
    fi
    target_safe "$target" || continue
    if remove_directory "$target"; then
      if [ "$DRY_RUN" = 1 ]; then
        printf 'WOULD drain retired target %s\n' "$target"
      else
        printf 'target_gc: drained retired target %s\n' "$target"
      fi
    else
      status=$?
      handle_remove_failure "$target" "$status"
    fi
  done
  TARGETS=("${retained_targets[@]:-}")
fi

measure_targets || exit 2
if [ "$DRY_RUN" = 1 ] && [ "$tomb_total_mb" -gt 0 ]; then
  freed_mb=$((freed_mb + tomb_total_mb))
  say "target_gc: $((total_mb + tomb_total_mb))M physical across ${target_count} target(s) plus ${tomb_count} tomb(s); projected ${total_mb}M after tomb recovery; cap ${CAP_MB}M, keep ${KEEP} evictable target(s)"
else
  say "target_gc: ${total_mb}M across ${target_count} target(s); cap ${CAP_MB}M, keep ${KEEP} evictable target(s)"
fi

# A count-only excess is resolved by dropping old clone targets. Do not throw
# away warm incremental state from the protected target to solve a count bound.
if [ "$total_mb" -le "$CAP_MB" ] && [ "$evictable_count" -gt "$KEEP" ]; then
  while IFS=$'\t' read -r _ size protected locked target; do
    [ "$protected" = 1 ] && continue
    [ "$locked" = 1 ] && continue
    [ "$evictable_count" -le "$KEEP" ] && break
    target_safe "$target" || continue
    if remove_directory "$target"; then
      total_mb=$((total_mb - size))
      evictable_count=$((evictable_count - 1))
      freed_mb=$((freed_mb + size))
      if [ "$DRY_RUN" = 1 ]; then
        printf 'WOULD evict %sM  %s\n' "$size" "$target"
      else
        printf 'target_gc: evicted %sM  %s\n' "$size" "$target"
      fi
    else
      status=$?
      handle_remove_failure "$target" "$status"
    fi
  done < <(sort -n "$tmp")
fi

if [ "$total_mb" -le "$CAP_MB" ] && [ "$evictable_count" -le "$KEEP" ]; then
  if [ "$freed_mb" -gt 0 ]; then
    if [ "$DRY_RUN" = 1 ]; then
      printf 'target_gc: would free approximately %sM\n' "$freed_mb"
    else
      printf 'target_gc: freed %sM; approximately %sM remains\n' "$freed_mb" "$total_mb"
    fi
  fi
  say "target_gc: within bounds; warm incremental artifacts were preserved"
  stamp_success
  exit 0
fi

prune_incrementals() {
  local wanted_protected="$1" protected locked target incremental size find_tmp status
  while IFS=$'\t' read -r _ _ protected locked target; do
    [ "$locked" = 1 ] && continue
    [ "$protected" = "$wanted_protected" ] || continue
    find_tmp="$(mktemp)" || {
      echo "target_gc: could not allocate incremental discovery state" >&2
      return 2
    }
    if ! find "$target" -type d -name incremental -prune -print0 > "$find_tmp" 2>/dev/null; then
      rm -f -- "$find_tmp"
      echo "target_gc: could not enumerate incrementals in $target" >&2
      return 2
    fi
    while IFS= read -r -d '' incremental; do
      size=$(du -sm "$incremental" 2>/dev/null | cut -f1) || {
        rm -f -- "$find_tmp"
        echo "target_gc: could not measure incremental directory $incremental" >&2
        return 2
      }
      if remove_directory "$incremental"; then
        freed_mb=$((freed_mb + size))
        if [ "$DRY_RUN" = 1 ]; then
          say "WOULD prune ${size}M from $incremental"
        else
          say "target_gc: pruned ${size}M from $incremental"
        fi
      else
        status=$?
        rm -f -- "$find_tmp"
        handle_remove_failure "$incremental" "$status"
      fi
    done < "$find_tmp"
    rm -f -- "$find_tmp"
  done < "$tmp"
}

# Prefer loss from inactive clone caches before touching the active target.
prune_incrementals 0 || exit $?

measure_targets || exit 2

# Remove the oldest inactive target trees until both the size and count bounds hold.
while IFS=$'\t' read -r _ size protected locked target; do
  [ "$protected" = 1 ] && continue
  [ "$locked" = 1 ] && continue
  if [ "$total_mb" -le "$CAP_MB" ] && [ "$evictable_count" -le "$KEEP" ]; then
    break
  fi
  target_safe "$target" || continue
  if remove_directory "$target"; then
    total_mb=$((total_mb - size))
    evictable_count=$((evictable_count - 1))
    freed_mb=$((freed_mb + size))
    if [ "$DRY_RUN" = 1 ]; then
      printf 'WOULD evict %sM  %s\n' "$size" "$target"
    else
      printf 'target_gc: evicted %sM  %s\n' "$size" "$target"
    fi
  else
    status=$?
    handle_remove_failure "$target" "$status"
  fi
done < <(sort -n "$tmp")

# Only an excess that survived old-target eviction may consume the active
# incremental cache. This keeps the everyday compile loop warm when possible.
if [ "$total_mb" -gt "$CAP_MB" ]; then
  measure_targets || exit 2
  prune_incrementals 1 || exit $?
  measure_targets || exit 2
fi

# The native shared target is disposable and marker-bound. If it alone exceeds
# the hard cap, replace it atomically. This is the final safety valve against
# an unbounded VHD even when Cargo has accumulated many feature permutations.
if [ "$total_mb" -gt "$CAP_MB" ] && [ -n "$MANAGED_TARGET" ] && [ "$PROTECTED_TARGET" = "$MANAGED_TARGET" ] && [ -d "$MANAGED_TARGET" ]; then
  allowed_base="$MANAGED_BASE"
  marker="$MANAGED_TARGET/.civsim-managed-target-v1"
  case "$MANAGED_TARGET/" in
    "$allowed_base/"*) managed_safe=1 ;;
    *) managed_safe=0 ;;
  esac
  if [ "$managed_safe" = 1 ] && managed_marker_valid "$MANAGED_TARGET"; then
    managed_size=$(du -sm "$MANAGED_TARGET" 2>/dev/null | cut -f1) || {
      echo "target_gc: could not measure the managed target" >&2
      exit 2
    }
    [ -n "$managed_size" ] || exit 2
    if [ "$managed_size" -le "$CAP_MB" ]; then
      say "target_gc: active target is within its cap; another protected target holds the excess"
    elif [ "$DRY_RUN" = 1 ]; then
      printf 'WOULD replace over-cap managed target %s\n' "$MANAGED_TARGET"
    elif build_running; then
      say "target_gc: a direct build appeared; standing down"
      exit 0
    else
      marker_contents=$(cat "$marker") || {
        echo "target_gc: could not read the managed target marker" >&2
        exit 2
      }
      tomb="$(dirname "$MANAGED_TARGET")/.civsim-gc-$(basename "$MANAGED_TARGET")-$$"
      tomb_marker="$MANAGED_TARGET/.civsim-gc-tomb-v1"
      tomb_marker_tmp="$(mktemp "$MANAGED_TARGET/.civsim-gc-tomb-v1.XXXXXX")" || {
        echo "target_gc: could not allocate the managed replacement marker" >&2
        exit 2
      }
      if ! printf 'schema=1\norigin=%s\n' "$MANAGED_TARGET" > "$tomb_marker_tmp" || \
        ! mv -f -- "$tomb_marker_tmp" "$tomb_marker"; then
        rm -f -- "$tomb_marker_tmp"
        echo "target_gc: could not mark the managed target for replacement" >&2
        exit 2
      fi
      if ! mv -- "$MANAGED_TARGET" "$tomb"; then
        rm -f -- "$tomb_marker"
        echo "target_gc: could not retire the over-cap managed target" >&2
        exit 2
      fi
      if mkdir -p "$MANAGED_TARGET" && \
        printf '%s\n' "$marker_contents" > "$MANAGED_TARGET/.civsim-managed-target-v1"; then
        if ! rm -rf -- "$tomb" || [ -e "$tomb" ]; then
          echo "target_gc: replacement succeeded but the discoverable old target could not be deleted" >&2
          exit 2
        fi
        freed_mb=$((freed_mb + managed_size))
        printf 'target_gc: reset over-cap managed target (%sM)\n' "$managed_size"
      else
        rm -rf -- "$MANAGED_TARGET"
        if mv -- "$tomb" "$MANAGED_TARGET"; then
          rm -f -- "$MANAGED_TARGET/.civsim-gc-tomb-v1"
          echo "target_gc: managed target replacement failed and was rolled back" >&2
        else
          echo "target_gc: managed target replacement failed; preserved tree remains at $tomb" >&2
        fi
        exit 2
      fi
    fi
  else
    echo "target_gc: over-cap managed target failed its path or marker guard; no destructive action taken" >&2
  fi
fi

measure_targets || exit 2
if [ "$freed_mb" -gt 0 ]; then
  if [ "$DRY_RUN" = 1 ]; then
    printf 'target_gc: would free approximately %sM\n' "$freed_mb"
  else
    printf 'target_gc: freed %sM; approximately %sM remains\n' "$freed_mb" "$total_mb"
  fi
else
  say "target_gc: no collectable artifacts were removed"
fi
if [ "$total_mb" -gt "$CAP_MB" ]; then
  echo "target_gc: ${total_mb}M remains above the ${CAP_MB}M cap because active or locked targets are protected" >&2
  [ "$DRY_RUN" = 1 ] && exit 0
  exit 2
fi
stamp_success
exit 0
