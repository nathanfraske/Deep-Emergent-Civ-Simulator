#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# Cargo build-artifact ring buffer. Cargo never garbage-collects a target directory, so a repository
# worked by many parallel agent worktrees accumulates one full target per worktree and grows without
# bound (this box reached 244 GB of artifacts across eighteen worktrees, which is what pushed the WSL
# volume into the compression attempt that corrupted it). This script bounds the total.
#
# The policy is least-recently-built eviction under two caps: a total size cap in gigabytes, and a cap on
# how many worktree targets are kept at all. Eviction is graduated, cheapest loss first: the pure
# rebuild cache (`incremental/`) goes before any compiled artifact, because dropping it costs only
# incremental-compile speed and never a rebuild from source.
#
# Three safety rules, each of which skips work rather than risking a break:
#   1. If any cargo or rustc process is running, the script does nothing. Deleting a target under a live
#      build corrupts that build.
#   2. A locked worktree is never touched. The harness locks the worktree of a RUNNING agent, so a lock
#      means someone is building there right now.
#   3. The main workspace target is never deleted outright, only incremental-pruned. It is the active
#      tree, and a full rebuild there costs the working session several minutes.
#
# Usage:
#   scripts/target_gc.sh                      # enforce the defaults
#   scripts/target_gc.sh --dry-run            # report what would be freed, delete nothing
#   scripts/target_gc.sh --cap-gb 40 --keep 2 # tighter caps

set -uo pipefail

CAP_GB="${TARGET_GC_CAP_GB:-60}"
KEEP="${TARGET_GC_KEEP:-3}"
STALE_DAYS="${TARGET_GC_STALE_DAYS:-7}"
DRY_RUN=0
VERBOSE=0

while [ $# -gt 0 ]; do
  case "$1" in
    --cap-gb) CAP_GB="$2"; shift 2 ;;
    --keep) KEEP="$2"; shift 2 ;;
    --stale-days) STALE_DAYS="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --verbose|-v) VERBOSE=1; shift ;;
    -h|--help) sed -n '6,30p' "$0"; exit 0 ;;
    *) echo "target_gc: unknown argument $1" >&2; exit 2 ;;
  esac
done

ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
cd "$ROOT" || exit 0
CAP_MB=$((CAP_GB * 1024))

say() { [ "$VERBOSE" = 1 ] && echo "$@"; return 0; }
act() { [ "$DRY_RUN" = 1 ] && echo "WOULD $*" || eval "$@"; }

# Safety rule 1: never garbage-collect under a live build. A dry run deletes nothing, so it still reports.
if pgrep -x cargo >/dev/null 2>&1 || pgrep -x rustc >/dev/null 2>&1; then
  if [ "$DRY_RUN" = 1 ]; then
    echo "target_gc: a build is running; reporting only (a real run would stand down here)."
  else
    say "target_gc: a build is running; standing down."
    exit 0
  fi
fi

# Safety rule 2: collect the locked worktrees, which are the ones with running agents.
locked_paths=""
current_wt=""
while read -r line; do
  case "$line" in
    /*) current_wt="${line%% *}" ;;
    *locked*) locked_paths="$locked_paths $current_wt" ;;
  esac
done < <(git worktree list --porcelain 2>/dev/null | sed 's/^worktree //')
# The porcelain form puts the path and the "locked" marker on separate lines, so re-read plainly too.
while read -r p rest; do
  case "$rest" in *locked*) locked_paths="$locked_paths $p" ;; esac
done < <(git worktree list 2>/dev/null)

# A lock protects a worktree only while the agent that took it is alive. A crash or a reboot leaves the
# lock file behind naming a process that is gone (this box rebooted mid-session and did exactly that), and
# a ring buffer that honours a stale lock protects dead worktrees forever, which is the accumulation this
# script exists to stop. So a lock counts as live only when its recorded pid still exists; when the lock
# names no pid we keep the conservative reading and treat it as live.
is_locked() {
  case " $locked_paths " in
    *" $1 "*) ;;
    *) return 1 ;;
  esac
  local wt_name lock_file pid want_start have_start
  wt_name="$(basename "$1")"
  lock_file="$ROOT/.git/worktrees/$wt_name/locked"
  [ -f "$lock_file" ] || return 0
  pid="$(sed -n 's/.*pid \([0-9]\{1,\}\).*/\1/p' "$lock_file" 2>/dev/null | head -1)"
  [ -n "$pid" ] || return 0
  if ! kill -0 "$pid" 2>/dev/null; then
    say "target_gc: lock on $wt_name is stale (pid $pid is gone); treating it as collectable."
    return 1
  fi
  # The pid existing is not enough. After a reboot the kernel hands out low pids again, so the agent's pid
  # is very likely alive as something else entirely (here pid 1794 came back as an unrelated worker). The
  # lock records the process start time for exactly this reason: compare it against field 22 of
  # /proc/<pid>/stat, and only a match means the original process is the one still holding the lock.
  want_start="$(sed -n 's/.*start \([0-9]\{1,\}\).*/\1/p' "$lock_file" 2>/dev/null | head -1)"
  [ -n "$want_start" ] || return 0
  have_start="$(awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)"
  [ -n "$have_start" ] || return 0
  if [ "$want_start" = "$have_start" ]; then
    return 0
  fi
  say "target_gc: lock on $wt_name is stale (pid $pid was reused: start $have_start, lock recorded $want_start); collectable."
  return 1
}

MAIN_TARGET="$ROOT/target"
freed_mb=0

# Pass 1, the free win: drop every incremental cache. This is a pure rebuild cache, so it costs
# incremental-compile speed and never an artifact. Run it before any eviction, since it often brings the
# total under the cap on its own.
for inc in "$MAIN_TARGET"/*/incremental "$ROOT"/.claude/worktrees/*/target/*/incremental; do
  [ -d "$inc" ] || continue
  wt_root="${inc%%/target/*}"
  if [ "$wt_root" != "$ROOT" ] && is_locked "$wt_root"; then
    say "target_gc: skipping locked $wt_root"
    continue
  fi
  sz=$(du -sm "$inc" 2>/dev/null | cut -f1)
  [ -n "${sz:-}" ] || continue
  [ "$sz" -lt 1 ] && continue
  act "rm -rf '$inc'"
  freed_mb=$((freed_mb + sz))
  say "target_gc: pruned ${sz}M of incremental cache in $inc"
done

# Pass 2: measure what remains, oldest build first. The modification time of the target directory tracks
# the last build into it, which is the recency signal the ring buffer evicts on.
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
total_mb=0
for t in "$MAIN_TARGET" "$ROOT"/.claude/worktrees/*/target; do
  [ -d "$t" ] || continue
  wt_root="${t%/target}"
  is_locked "$wt_root" && continue
  sz=$(du -sm "$t" 2>/dev/null | cut -f1)
  [ -n "${sz:-}" ] || continue
  mtime=$(stat -c %Y "$t" 2>/dev/null || echo 0)
  total_mb=$((total_mb + sz))
  # The main target is protected from outright deletion, so it is measured but never queued.
  [ "$t" = "$MAIN_TARGET" ] && continue
  printf '%s\t%s\t%s\n' "$mtime" "$sz" "$t" >> "$tmp"
done

count=$(wc -l < "$tmp" | tr -d ' ')
say "target_gc: ${total_mb}M across $((count + 1)) target(s); cap ${CAP_MB}M, keep ${KEEP} worktree target(s)."

# Pass 3: evict least-recently-built worktree targets until both caps are satisfied.
kept=0
while read -r mtime sz path; do
  over_size=$([ "$total_mb" -gt "$CAP_MB" ] && echo 1 || echo 0)
  over_count=$([ "$count" -gt "$KEEP" ] && echo 1 || echo 0)
  if [ "$over_size" = 0 ] && [ "$over_count" = 0 ]; then
    break
  fi
  act "rm -rf '$path'"
  total_mb=$((total_mb - sz))
  freed_mb=$((freed_mb + sz))
  count=$((count - 1))
  echo "target_gc: evicted ${sz}M  $path"
done < <(sort -n "$tmp")
kept=$count

# Pass 4: the main target is protected from deletion but still accumulates, because cargo never removes
# the artifacts of a dependency version, a feature permutation, or a test binary that no longer exists.
# When the total is still over cap, sweep the compiled outputs no build has touched in STALE_DAYS. Cargo
# rebuilds a missing output on demand, so this costs rebuild time for the stale part alone and never
# correctness. Fingerprints are left in place; cargo notices the absent output and redoes just that unit.
if [ "$total_mb" -gt "$CAP_MB" ] && [ -d "$MAIN_TARGET" ]; then
  swept_mb=0
  for sub in deps examples build; do
    for prof in debug release; do
      d="$MAIN_TARGET/$prof/$sub"
      [ -d "$d" ] || continue
      sz_before=$(du -sm "$d" 2>/dev/null | cut -f1)
      # An artifact counts as stale only when it was BOTH built long ago and not read since. Either clock
      # alone lies: mtime alone condemns an old artifact that recent builds still link against, and atime
      # alone is defeated by anything that walks the tree (a filesystem restore rewrites every atime,
      # which is exactly what the copyback did here). Requiring both is the conservative reading, and it
      # degrades to sweeping nothing rather than to sweeping something live.
      if [ "$DRY_RUN" = 1 ]; then
        n=$(find "$d" -maxdepth 1 -type f -atime +"$STALE_DAYS" -mtime +"$STALE_DAYS" 2>/dev/null | wc -l)
        [ "$n" -gt 0 ] && echo "WOULD sweep $n stale file(s) from $prof/$sub"
        continue
      fi
      find "$d" -maxdepth 1 -type f -atime +"$STALE_DAYS" -mtime +"$STALE_DAYS" -delete 2>/dev/null
      sz_after=$(du -sm "$d" 2>/dev/null | cut -f1)
      swept_mb=$((swept_mb + sz_before - sz_after))
    done
  done
  if [ "$swept_mb" -gt 0 ]; then
    freed_mb=$((freed_mb + swept_mb))
    total_mb=$((total_mb - swept_mb))
    echo "target_gc: swept ${swept_mb}M of artifacts unused for ${STALE_DAYS}+ days from the main target."
  fi
fi

if [ "$freed_mb" -gt 0 ]; then
  echo "target_gc: freed $((freed_mb / 1024))G (${freed_mb}M); ${total_mb}M remains across the main target and ${kept} worktree target(s)."
else
  say "target_gc: already within caps; nothing to free."
fi
# Report honestly when the protected main target alone cannot fit under the cap: that is a deliberate
# `cargo clean` decision for the owner, never something this script takes on its own.
if [ "$total_mb" -gt "$CAP_MB" ]; then
  echo "target_gc: still ${total_mb}M over the ${CAP_MB}M cap, and the remainder is the protected main target. Run 'cargo clean' deliberately to reclaim it (it costs one full rebuild)."
fi
exit 0
