#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
#
# Stone 0 CI tombstone scan: grep the pushed commit range for every retired tombstone phrase, and grep
# the FULL history whenever the tombstone file itself changed in the push. A retired phrase is
# declassified (it opens nothing), so nothing secret lives in the cloud and matched phrases may be named
# here (unlike the pre-push hook, which also handles the LIVE password and stays quiet). This catches a
# laundering committed while a password was live, retroactively, on the first run after that phrase is
# retired into the tombstone list.
#
# Inputs (set by the workflow from the push event): BEFORE (github.event.before), AFTER (github.sha).
# The tracked-tree state is already covered by the `stone0-gate --ci` step; this adds the range and
# history dimensions. Fails OPEN on an operational error; fails CLOSED only on a detection.

set -o pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
TOMBSTONES="$ROOT/scripts/stone0_tombstones.txt"
EXCLUDE=':(exclude)scripts/stone0_tombstones.txt'
ZERO="0000000000000000000000000000000000000000"

if [ ! -f "$TOMBSTONES" ]; then
  echo "stone0-ci: no tombstone list; nothing to scan."
  exit 0
fi

# The retired phrases: non-comment, non-empty lines, trimmed.
mapfile -t PHRASES < <(grep -v '^[[:space:]]*#' "$TOMBSTONES" 2>/dev/null | grep -v '^[[:space:]]*$' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
if [ "${#PHRASES[@]}" -eq 0 ]; then
  echo "stone0-ci: tombstone list is empty; nothing to scan."
  exit 0
fi

# Decide the scope. If we have a real before-sha and it exists, the range is BEFORE..AFTER; otherwise
# (new branch, first push, or a missing base) fall back to the full history, which is a superset.
AFTER="${AFTER:-$(git rev-parse HEAD 2>/dev/null)}"
have_range=0
if [ -n "$BEFORE" ] && [ "$BEFORE" != "$ZERO" ] && git cat-file -e "$BEFORE^{commit}" 2>/dev/null; then
  have_range=1
fi

# Whether the tombstone file itself changed in this push (a new phrase was retired), which triggers a
# full-history re-check for that phrase. If we cannot compute the range, scan full history to be safe.
tombstone_changed=1
if [ "$have_range" -eq 1 ]; then
  if git diff --name-only "$BEFORE..$AFTER" 2>/dev/null | grep -qx 'scripts/stone0_tombstones.txt'; then
    tombstone_changed=1
  else
    tombstone_changed=0
  fi
fi

fail=0
for phrase in "${PHRASES[@]}"; do
  [ -z "$phrase" ] && continue
  if [ "$tombstone_changed" -eq 1 ] || [ "$have_range" -eq 0 ]; then
    scope=(--all)
    where="full history"
  else
    scope=("$BEFORE..$AFTER")
    where="pushed range $BEFORE..$AFTER"
  fi
  # git's pickaxe (-S, literal by default) finds every commit that added or removed the phrase in any
  # file except the tombstone list itself.
  hits="$(git log "${scope[@]}" -S"$phrase" --oneline -- . "$EXCLUDE" 2>/dev/null)"
  if [ -n "$hits" ]; then
    echo "stone0-ci: BLOCKED. Retired tombstone phrase laundered in $where:"
    echo "  phrase: $phrase"
    echo "$hits" | sed 's/^/    /'
    echo "  A retired phrase must appear nowhere but the tombstone list. Remove the laundered copy."
    fail=1
  fi
done

if [ "$fail" -eq 0 ]; then
  echo "stone0-ci: clean (no retired tombstone phrase laundered in the scanned scope)."
fi
exit "$fail"
