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
# INSTALL (do this once, per clone; it does NOT change tracked repository data):
#   just hooks-install
# The tracked scripts/githooks/pre-push entrypoint delegates here. `just doctor`
# verifies that the clone still points at that directory.

if [ -n "${STONE0_SECRETS_PATH:-}" ]; then
  SECRETS="$STONE0_SECRETS_PATH"
else
  # WSL mounts Windows drives below /mnt, while Git for Windows exposes the
  # same drive at /e inside Git Bash. Prefer the first path that exists so the
  # local pre-push tripwire does not disappear when a clone switches shells.
  SECRETS="/mnt/e/Secrets/stone0-override.pass"
  for candidate in \
    /mnt/e/Secrets/stone0-override.pass \
    /e/Secrets/stone0-override.pass
  do
    if [ -f "$candidate" ]; then
      SECRETS="$candidate"
      break
    fi
  done
fi
TOMBSTONES="$(git rev-parse --show-toplevel 2>/dev/null)/scripts/stone0_tombstones.txt"
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

fail=0
while read -r local_ref local_sha remote_ref remote_sha; do
  # A ref deletion pushes a zero local sha; nothing to scan.
  [ "$local_sha" = "$ZERO" ] && continue

  commits=""
  if [ "$remote_sha" = "$ZERO" ]; then
    # A new ref has no remote endpoint. Scan every commit reachable from the
    # local ref that is not already reachable from a tracking ref for this
    # remote. If the clone has no tracking refs yet, this becomes the complete
    # reachable history, which is the safe first-push behavior.
    remote_name="${1:-}"
    remote_tips=()
    if [ -n "$remote_name" ]; then
      while IFS= read -r tip; do
        [ -n "$tip" ] && remote_tips+=("$tip")
      done < <(git for-each-ref --format='%(objectname)' "refs/remotes/$remote_name/" 2>/dev/null)
    fi
    rev_args=("$local_sha")
    if [ "${#remote_tips[@]}" -gt 0 ]; then
      rev_args+=(--not "${remote_tips[@]}")
    fi
    if ! commits="$(git rev-list "${rev_args[@]}" 2>/dev/null)"; then
      echo "stone0 pre-push: NOTICE. Could not enumerate the new-ref history for $local_ref; allowing the push under the fail-open contract." >&2
      continue
    fi
  else
    # This is a commit set, not an endpoint diff. Scanning every snapshot in
    # the set catches a password that was added in one commit and removed in a
    # later commit before the same push.
    if ! commits="$(git rev-list "$remote_sha..$local_sha" 2>/dev/null)"; then
      echo "stone0 pre-push: NOTICE. Could not enumerate $remote_sha..$local_sha for $local_ref; allowing the push under the fail-open contract." >&2
      continue
    fi
  fi

  for commit in $commits; do
    # Exclude the tombstone list itself: it legitimately holds every retired
    # phrase. `git grep <commit>` searches the full committed snapshot, including
    # binary blobs, without exposing the matched bytes.
    git grep -q -F -f <(emit_patterns) "$commit" -- . ':(exclude)scripts/stone0_tombstones.txt' 2>/dev/null
    grep_status=$?
    if [ "$grep_status" -eq 0 ]; then
      echo "stone0 pre-push: BLOCKED. A live override password or a retired tombstone phrase appears in commit $commit"
      echo "  within the pushed history for $local_ref. The matched content is NOT shown."
      echo "  Remove it from every commit before pushing. If it was the live password, tell Nathan to rotate it out of band."
      fail=1
      break
    fi
    if [ "$grep_status" -gt 1 ]; then
      echo "stone0 pre-push: NOTICE. Could not scan commit $commit for $local_ref; allowing that scan under the fail-open contract." >&2
      break
    fi
  done
done

exit "$fail"
