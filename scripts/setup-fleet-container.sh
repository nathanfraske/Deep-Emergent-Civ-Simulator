#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
set -euo pipefail

# The digest-pinned official Rust image supplies Rust, C/C++, Git, curl,
# pkg-config and Python 3.11. This is minifb's upstream Linux setup recipe:
# https://github.com/emoon/rust_minifb#build-instructions
# It executes inside the disposable job container, never on the physical host.
test -f /.dockerenv
test "$(id -u)" = 0
python3 -c 'import sys,tomllib; assert sys.version_info >= (3,11)'

# The runner owns the checkout directory; this container runs as root. Checkout
# sets safe.directory only in its temporary HOME. Build scripts invoke Git from
# crate directories later, so persist trust for this exact job checkout and prove
# that Git resolves both locations to the expected source before compiling.
if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  workspace="$(realpath "${GITHUB_WORKSPACE:?}")"
  test "$workspace" = "$(pwd -P)"
  git config --global --add safe.directory "$workspace"
  test "$(git rev-parse --show-toplevel)" = "$workspace"
  test "$(git -C crates/sim rev-parse --show-toplevel)" = "$workspace"
  test "$(git rev-parse HEAD)" = "${GITHUB_SHA:?}"
fi
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  libxkbcommon-dev libwayland-cursor0 libwayland-dev

# Cache keys and compiler fanout are infrastructure settings, not simulation
# calibrations. The cache is deliberately separate from all other repositories.
python3 scripts/test_fleet_cache.py
python3 scripts/fleet-cache.py
