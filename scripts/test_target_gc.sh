#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

# Focused canaries for bounded target retention and the WSL environment layout.

set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf -- "$TMP"' EXIT

REPO="$TMP/repo"
CACHE_BASE="$TMP/civsim-dev"
KEY="$(printf 'synthetic\0' | sha256sum | cut -c1-16)"
MANAGED="$CACHE_BASE/targets/$KEY"
STATE="$CACHE_BASE/maintenance"
mkdir -p "$REPO" "$MANAGED/canonical/debug/incremental" "$STATE"
git -C "$REPO" init -q
git -C "$REPO" config user.email target-gc-test@example.invalid
git -C "$REPO" config user.name target-gc-test
printf 'fixture\n' > "$REPO/fixture.txt"
git -C "$REPO" add fixture.txt
git -C "$REPO" commit -qm fixture
SIBLING="$TMP/sibling-worktree"
git -C "$REPO" worktree add -q --detach "$SIBLING"

FAKE_BIN="$TMP/fake-bin"
mkdir -p "$FAKE_BIN"
printf '%s\n' '#!/usr/bin/env bash' 'exit 1' > "$FAKE_BIN/pgrep"
chmod +x "$FAKE_BIN/pgrep"
export PATH="$FAKE_BIN:$PATH"
printf 'schema=1\nrepository=synthetic\nkey=%s\n' "$KEY" > "$MANAGED/.civsim-managed-target-v1"

LOCK_SENTINEL="$TMP/lock-sentinel"
printf 'DO NOT TRUNCATE\n' > "$LOCK_SENTINEL"
ln -s "$LOCK_SENTINEL" "$STATE/artifacts.lock"

export TARGET_GC_REPO_ROOT="$REPO"
export CIVSIM_WSL_CACHE_ROOT="$CACHE_BASE"
export CIVSIM_MANAGED_TARGET_ROOT="$MANAGED"
export CIVSIM_MAINTENANCE_DIR="$STATE"
export CARGO_TARGET_DIR="$MANAGED/canonical"

if bash "$ROOT/scripts/target_gc.sh" --cap-mb >/dev/null 2>&1; then
  echo "target GC self-test: missing option value was accepted" >&2
  exit 1
fi
if TARGET_GC_REPO_ROOT="$TMP/missing-repository" \
  bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 >/dev/null 2>&1; then
  echo "target GC self-test: missing repository was reported as success" >&2
  exit 1
fi

dd if=/dev/zero of="$MANAGED/canonical/debug/incremental/warm.bin" bs=1M count=1 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10
test -f "$MANAGED/canonical/debug/incremental/warm.bin"
grep -qx 'DO NOT TRUNCATE' "$LOCK_SENTINEL"

mkdir -p "$SIBLING/target" "$SIBLING/parked/target"
printf 'retired\n' > "$SIBLING/target/sentinel"
printf 'retired\n' > "$SIBLING/parked/target/sentinel"
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10
test ! -e "$SIBLING/target"
test ! -e "$SIBLING/parked/target"

LINKED_PARKED="$TMP/linked-parked-user-data"
mkdir -p "$LINKED_PARKED/target"
printf 'keep\n' > "$LINKED_PARKED/target/sentinel"
rmdir "$SIBLING/parked"
ln -s "$LINKED_PARKED" "$SIBLING/parked"
if bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10 >/dev/null 2>&1; then
  echo "target GC self-test: linked parked ancestry was accepted" >&2
  exit 1
fi
test -f "$LINKED_PARKED/target/sentinel"
rm "$SIBLING/parked"

PRUNABLE="$TMP/prunable-worktree"
git -C "$REPO" worktree add -q --detach "$PRUNABLE"
rm -rf -- "$PRUNABLE"
mkdir -p "$PRUNABLE/target"
printf 'keep\n' > "$PRUNABLE/target/sentinel"
git -C "$REPO" worktree list --porcelain | grep -q '^prunable '
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10
test -f "$PRUNABLE/target/sentinel"
rm -rf -- "$PRUNABLE"
git -C "$REPO" worktree prune

mkdir -p "$REPO/target"
printf 'busy\n' > "$REPO/target/busy-sentinel"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' > "$FAKE_BIN/pgrep"
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0
test -f "$REPO/target/busy-sentinel"
printf '%s\n' '#!/usr/bin/env bash' 'exit 1' > "$FAKE_BIN/pgrep"
rm -rf -- "$REPO/target"

FAIL_GIT_BIN="$TMP/fail-git-bin"
mkdir -p "$FAIL_GIT_BIN" "$REPO/target"
printf '%s\n' '#!/usr/bin/env bash' 'exit 1' > "$FAIL_GIT_BIN/git"
chmod +x "$FAIL_GIT_BIN/git"
printf 'keep\n' > "$REPO/target/git-failure-sentinel"
if PATH="$FAIL_GIT_BIN:$PATH" bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0 >/dev/null 2>&1; then
  echo "target GC self-test: failed worktree enumeration was reported as success" >&2
  exit 1
fi
test -f "$REPO/target/git-failure-sentinel"
rm -rf -- "$REPO/target"

EXTERNAL="$TMP/external-user-data"
mkdir -p "$EXTERNAL"
printf 'keep\n' > "$EXTERNAL/sentinel"
ln -s "$EXTERNAL" "$REPO/target"
if bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10 >/dev/null 2>&1; then
  echo "target GC self-test: linked repo target was accepted" >&2
  exit 1
fi
test -f "$EXTERNAL/sentinel"
rm "$REPO/target"

LINK_KEY="3333333333333333"
ln -s "$EXTERNAL" "$CACHE_BASE/targets/$LINK_KEY"
if bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10 >/dev/null 2>&1; then
  echo "target GC self-test: linked managed child was accepted" >&2
  exit 1
fi
test -f "$EXTERNAL/sentinel"
rm "$CACHE_BASE/targets/$LINK_KEY"

OLD_KEY="$(printf 'old\0' | sha256sum | cut -c1-16)"
OLD_MANAGED="$CACHE_BASE/targets/$OLD_KEY"
mkdir -p "$OLD_MANAGED/canonical"
printf 'schema=1\nrepository=old\nkey=%s\n' "$OLD_KEY" > "$OLD_MANAGED/.civsim-managed-target-v1"
dd if=/dev/zero of="$OLD_MANAGED/canonical/old.bin" bs=1M count=2 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 0
test ! -e "$OLD_MANAGED"
test -f "$MANAGED/canonical/debug/incremental/warm.bin"

mkdir -p "$REPO/target"
dd if=/dev/zero of="$REPO/target/old.bin" bs=1M count=3 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 2 --keep 0
test ! -e "$REPO/target"
test -f "$MANAGED/canonical/debug/incremental/warm.bin"
test -f "$MANAGED/.civsim-managed-target-v1"

mkdir -p "$MANAGED/canonical"
dd if=/dev/zero of="$MANAGED/canonical/oversized.bin" bs=1M count=4 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0 --dry-run >/dev/null
test -f "$MANAGED/canonical/oversized.bin"
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0
test ! -e "$MANAGED/canonical/oversized.bin"
test -f "$MANAGED/.civsim-managed-target-v1"

rm -f "$MANAGED/.civsim-managed-target-v1"
mkdir -p "$MANAGED/canonical"
dd if=/dev/zero of="$MANAGED/canonical/unmarked.bin" bs=1M count=2 status=none
if bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0 2> "$TMP/unmarked.err"; then
  echo "target GC self-test: unmarked managed target was accepted" >&2
  exit 1
fi
test -f "$MANAGED/canonical/unmarked.bin"
grep -q 'unmarked managed-target-shaped' "$TMP/unmarked.err"
rm -f "$MANAGED/canonical/unmarked.bin"
printf 'schema=1\nrepository=synthetic\nkey=%s\n' "$KEY" > "$MANAGED/.civsim-managed-target-v1"

touch "$STATE/target-gc.stamp"
mkdir -p "$REPO/target"
dd if=/dev/zero of="$REPO/target/not-due.bin" bs=1M count=2 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0 --if-due
test -f "$REPO/target/not-due.bin"
rm -rf -- "$REPO/target"

TOMB="$CACHE_BASE/targets/.civsim-gc-$OLD_KEY-999"
mkdir -p "$TOMB/canonical"
printf 'schema=1\norigin=%s\n' "$OLD_MANAGED" > "$TOMB/.civsim-gc-tomb-v1"
printf 'schema=1\nrepository=old\nkey=%s\n' "$OLD_KEY" > "$TOMB/.civsim-managed-target-v1"
dd if=/dev/zero of="$TOMB/canonical/interrupted.bin" bs=1M count=2 status=none
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10 --dry-run --verbose > "$TMP/tomb-dry.out"
grep -q 'physical.*tomb' "$TMP/tomb-dry.out"
grep -q 'projected.*after tomb recovery' "$TMP/tomb-dry.out"
test -d "$TOMB"
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10
test ! -e "$TOMB"

NESTED_ORIGIN="$MANAGED/canonical/debug/incremental"
NESTED_TOMB="$MANAGED/canonical/debug/.civsim-gc-incremental-777"
mkdir -p "$NESTED_TOMB"
printf 'schema=1\norigin=%s\n' "$NESTED_ORIGIN" > "$NESTED_TOMB/.civsim-gc-tomb-v1"
printf 'interrupted\n' > "$NESTED_TOMB/sentinel"
bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10
test ! -e "$NESTED_TOMB"

LOOKALIKE="$CACHE_BASE/targets/.civsim-gc-unmarked-lookalike"
mkdir -p "$LOOKALIKE"
printf 'keep\n' > "$LOOKALIKE/sentinel"
if bash "$ROOT/scripts/target_gc.sh" --cap-mb 10 --keep 10 >/dev/null 2>&1; then
  echo "target GC self-test: unmarked tomb lookalike was accepted" >&2
  exit 1
fi
test -f "$LOOKALIKE/sentinel"
rm -rf -- "$LOOKALIKE"

mkdir -p "$REPO/target"
dd if=/dev/zero of="$REPO/target/locked.bin" bs=1M count=2 status=none
exec 7< "$STATE"
flock -s 7
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0
test -f "$REPO/target/locked.bin"
flock -u 7
exec 7>&-
bash "$ROOT/scripts/target_gc.sh" --cap-mb 1 --keep 0
test ! -e "$REPO/target"

FAKE_CARGO="$TMP/fake-cargo"
printf '%s\n' \
  '#!/usr/bin/env bash' \
  'set -euo pipefail' \
  'mkdir -p "$CARGO_TARGET_DIR"' \
  'dd if=/dev/zero of="$CARGO_TARGET_DIR/post-build.bin" bs=1M count=4 status=none' \
  > "$FAKE_CARGO"
chmod +x "$FAKE_CARGO"
touch "$STATE/target-gc.stamp"
export CIVSIM_REPO_ROOT="$ROOT"
unset CIVSIM_MANAGED_TARGET_ROOT CARGO_TARGET_DIR
source "$ROOT/scripts/wsl_dev_env.sh" --quiet
WRAPPER_MANAGED="$CIVSIM_MANAGED_TARGET_ROOT"
WSL_DISTRO_NAME=synthetic CARGO="$FAKE_CARGO" TARGET_GC_CAP_MB=1 \
  bash "$ROOT/scripts/cargo_dev.sh" check
test ! -e "$WRAPPER_MANAGED/canonical/post-build.bin"
test -f "$WRAPPER_MANAGED/.civsim-managed-target-v1"

# An inherited managed-target variable must not authorize Cargo to recreate a
# collected directory without its repository-bound marker.
rm -rf -- "$WRAPPER_MANAGED"
WSL_DISTRO_NAME=synthetic CARGO="$FAKE_CARGO" TARGET_GC_CAP_MB=10 \
  bash "$ROOT/scripts/cargo_dev.sh" check
test -f "$WRAPPER_MANAGED/.civsim-managed-target-v1"

unset TARGET_GC_REPO_ROOT CIVSIM_WSL_CACHE_ROOT CIVSIM_MANAGED_TARGET_ROOT CIVSIM_MAINTENANCE_DIR CARGO_TARGET_DIR
export XDG_CACHE_HOME="$TMP/xdg"
export CIVSIM_REPO_ROOT="$ROOT"
common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir)"
common="$(realpath -m "$common")"
env_key="$(printf '%s\0' "$common" | sha256sum | cut -c1-16)"
unmarked_env_target="$XDG_CACHE_HOME/civsim-dev/targets/$env_key"
mkdir -p "$unmarked_env_target"
printf 'keep\n' > "$unmarked_env_target/sentinel"
before_umask="$(umask)"
if source "$ROOT/scripts/wsl_dev_env.sh" --quiet; then
  echo "target GC self-test: WSL environment adopted an unmarked target" >&2
  exit 1
fi
test -f "$unmarked_env_target/sentinel"
test "$(umask)" = "$before_umask"
rm -rf -- "$XDG_CACHE_HOME/civsim-dev"
source "$ROOT/scripts/wsl_dev_env.sh" --quiet
first_target="$CARGO_TARGET_DIR"
source "$ROOT/scripts/wsl_dev_env.sh" --quiet
test "$CARGO_TARGET_DIR" = "$first_target"
case "$CARGO_TARGET_DIR" in "$TMP/xdg/civsim-dev/targets/"*/canonical) ;; *) exit 1;; esac
test -f "$CIVSIM_MANAGED_TARGET_ROOT/.civsim-managed-target-v1"
test "$TARGET_GC_CAP_GB" = 32
test "$CIVSIM_MAINTENANCE_DIR" = "$TMP/xdg/civsim-dev/maintenance"
test "$(umask)" = "$before_umask"

ENV_MARKER="$CIVSIM_MANAGED_TARGET_ROOT/.civsim-managed-target-v1"
cp "$ENV_MARKER" "$TMP/env-marker.backup"
rm "$ENV_MARKER"
ln -s "$EXTERNAL/sentinel" "$ENV_MARKER"
if source "$ROOT/scripts/wsl_dev_env.sh" --quiet; then
  echo "target GC self-test: linked managed marker was accepted" >&2
  exit 1
fi
test -f "$EXTERNAL/sentinel"
rm "$ENV_MARKER"
mv "$TMP/env-marker.backup" "$ENV_MARKER"
source "$ROOT/scripts/wsl_dev_env.sh" --quiet

FAKE_FAIL="$TMP/fail-command"
printf '%s\n' '#!/usr/bin/env bash' 'exit 7' > "$FAKE_FAIL"
chmod +x "$FAKE_FAIL"
TRIM_STATE="$TMP/trim-state"
mkdir -p "$TRIM_STATE"
if WSL_DISTRO_NAME=synthetic CIVSIM_FSTRIM="$FAKE_FAIL" CIVSIM_WSL_EXE="$FAKE_FAIL" \
  CIVSIM_MAINTENANCE_DIR="$TRIM_STATE" bash "$ROOT/scripts/wsl_trim.sh" --force >/dev/null 2>&1; then
  echo "target GC self-test: failed trim was reported as success" >&2
  exit 1
fi
test ! -e "$TRIM_STATE/wsl-trim.stamp"

if WSL_DISTRO_NAME=synthetic CIVSIM_MAINTENANCE_DIR=/dev/null/state \
  bash "$ROOT/scripts/wsl_trim.sh" --force >/dev/null 2>&1; then
  echo "target GC self-test: trim state failure was reported as success" >&2
  exit 1
fi

if WSL_DISTRO_NAME=synthetic CIVSIM_FSTRIM="$TMP/missing-fstrim" \
  CIVSIM_WSL_EXE="$TMP/missing-wsl" CIVSIM_MAINTENANCE_DIR="$TRIM_STATE" \
  bash "$ROOT/scripts/wsl_trim.sh" --force >/dev/null 2>&1; then
  echo "target GC self-test: unavailable forced trim was reported as success" >&2
  exit 1
fi

if WSL_DISTRO_NAME=synthetic CIVSIM_REPO_ROOT="$TMP/missing-repository" \
  CLAUDE_PROJECT_DIR="$TMP/missing-repository" \
  bash "$ROOT/scripts/session_maintenance.sh" >/dev/null 2>&1; then
  echo "target GC self-test: session environment refusal was reported as success" >&2
  exit 1
fi

echo "target GC self-test: PASS (argument and root refusal, symlink refusal, warm retention, global eviction, tomb recovery, hard cap, build lock, post-build enforcement, marker recovery, throttle, trim failure, stable WSL layout)"
