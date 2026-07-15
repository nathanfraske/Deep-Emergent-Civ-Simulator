#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
#
# Stone 0 pre-push hook: scan the pushed commit range for the LIVE override password (and its base64)
# and for every RETIRED tombstone phrase, before the push leaves the machine. This is the local
# counterpart of the CI tombstone grep: it closes the window where a phrase is laundered and pushed
# before any CI run. It is a tripwire for banal, convenience-driven persistence, not a boundary against
# deliberate obfuscation (a split or a stronger encoding defeats a literal-string scan).
#
# ROBUSTNESS: this hook fails OPEN. Any operational problem (git error, no secrets file) allows the push;
# it blocks ONLY on a positive detection. It NEVER prints, logs, or writes the password: patterns are fed
# to grep through a process substitution (a /dev/fd pipe, never a disk file, never a command-line
# argument), and matches are found with `grep -q` so no matched content is ever emitted.
#
# INSTALL (do this once, per clone; it does NOT change the repo's tracked config):
#   git config core.hooksPath scripts/githooks
#   mkdir -p scripts/githooks
#   ln -sf ../../scripts/stone0-pre-push-hook.sh scripts/githooks/pre-push
# Or, if you keep the default hooks path:
#   ln -sf ../../scripts/stone0-pre-push-hook.sh .git/hooks/pre-push
# Verify it is executable: chmod +x scripts/stone0-pre-push-hook.sh

SECRETS="${STONE0_SECRETS_PATH:-/mnt/e/Secrets/stone0-override.pass}"
TOMBSTONES="$(git rev-parse --show-toplevel 2>/dev/null)/calibration/stone0_tombstones.txt"
ZERO="0000000000000000000000000000000000000000"

# The live password: the first non-empty, non-CANARY line of the secrets file, trimmed. Kept only in a
# shell variable (memory), never written anywhere.
pw=""
if [ -f "$SECRETS" ]; then
  pw="$(grep -v '^CANARY=' "$SECRETS" 2>/dev/null | grep -m1 -v '^[[:space:]]*$' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"
fi
pw_b64=""
if [ -n "$pw" ]; then
  pw_b64="$(printf '%s' "$pw" | base64 2>/dev/null | tr -d '\n')"
fi

# Emit the fixed-string patterns to scan for: the live password, its base64, and each tombstone phrase.
# The password reaches grep only through this function's stdout inside a process substitution.
emit_patterns() {
  [ -n "$pw" ] && printf '%s\n' "$pw"
  [ -n "$pw_b64" ] && printf '%s\n' "$pw_b64"
  if [ -f "$TOMBSTONES" ]; then
    grep -v '^[[:space:]]*#' "$TOMBSTONES" 2>/dev/null | grep -v '^[[:space:]]*$' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
  fi
}

# Nothing to scan for: allow the push (fail-open).
if [ "$(emit_patterns | grep -c .)" -eq 0 ]; then
  exit 0
fi

empty_tree="$(git hash-object -t tree /dev/null 2>/dev/null)"
fail=0
while read -r local_ref local_sha remote_ref remote_sha; do
  # A ref deletion pushes a zero local sha; nothing to scan.
  [ "$local_sha" = "$ZERO" ] && continue
  if [ "$remote_sha" = "$ZERO" ]; then
    # A new branch: diff against the empty tree so the whole pushed tree is scanned.
    base="$empty_tree"
  else
    base="$remote_sha"
  fi
  [ -z "$base" ] && continue
  # Exclude the tombstone list itself: it legitimately holds every retired phrase, and scanning it would
  # always self-trigger (the same exclusion the native gate applies).
  if git diff --no-color "$base" "$local_sha" -- . ':(exclude)calibration/stone0_tombstones.txt' 2>/dev/null | grep -q -F -f <(emit_patterns); then
    echo "stone0 pre-push: BLOCKED. A live override password or a retired tombstone phrase appears in the"
    echo "  pushed range ($base..$local_sha) for $local_ref. The matched content is NOT shown."
    echo "  Remove it before pushing. If it was the live password, tell Nathan to rotate it out of band."
    fail=1
  fi
done

exit "$fail"
