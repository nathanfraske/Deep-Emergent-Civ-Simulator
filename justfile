# Task entry points for the Emergent Civilization Simulator.
#
# Install the runner once (https://github.com/casey/just):
#   cargo install just
# Then `just` on its own lists every recipe.
#
# The cargo recipes below run identically on Linux, macOS, and Windows. The few that shell out to a
# script or a pipeline are marked UNIX and expect bash (on Windows use WSL or Git Bash for those).

# Show the available recipes.
default:
    @just --list

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

# Run the canonical simulation to its final state hash.
run:
    cargo run --release --example run_world -p civsim-sim

# Run the living-world scenario.
run-living:
    cargo run --release --example run_world -p civsim-sim -- --scenario living

# Open the desktop viewer on a generated world (CPU shading).
view:
    cargo run --release -p civsim-viewer

# Open the viewer with GPU globe shading. Needs a CUDA toolkit and an NVIDIA card;
# on WSL2 it also needs NVRTC and CUDA_PATH set.
view-gpu:
    cargo run --release -p civsim-viewer --features gpu

# ---------------------------------------------------------------------------
# Verify
# ---------------------------------------------------------------------------

# The whole workspace test suite.
test:
    cargo test --workspace

# Formatting gate (CI enforces this; run it before every commit).
fmt:
    cargo fmt --all

# Formatting check without writing.
fmt-check:
    cargo fmt --all --check

# Lints, all targets, warnings denied.
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# The documentation verification suite (em dashes, banned adverbs, part numbering, fences). UNIX.
verify:
    bash scripts/verify.sh

# Everything CI enforces, in the order that fails cheapest first. UNIX.
check-all: fmt-check lint test verify

# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------

# The two canonical byte pins. Both must print their expected hash; a change to either is a
# re-pin, which is the owner's call and never a silent edit. UNIX (uses a pipeline).
#   default expects 40fe8a7269ee4da8974eb1787338c3a0
#   living  expects be94e3100b9db82f7c1aea1d8091956d
pins:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release --example run_world -p civsim-sim
    echo "default (expect 40fe8a7269ee4da8974eb1787338c3a0):"
    ./target/release/examples/run_world | grep 'final state_hash'
    echo "living  (expect be94e3100b9db82f7c1aea1d8091956d):"
    ./target/release/examples/run_world --scenario living | grep 'final state_hash'

# ---------------------------------------------------------------------------
# Housekeeping
# ---------------------------------------------------------------------------

# Bound the cargo build artifacts (LRU ring buffer under a size and worktree-count cap). UNIX.
gc:
    bash scripts/target_gc.sh --verbose

# Report what the artifact GC would remove, without removing it. UNIX.
gc-dry:
    bash scripts/target_gc.sh --dry-run --verbose
