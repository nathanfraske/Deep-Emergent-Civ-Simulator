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

# The two fixture digests. Both must MATCH their expected hash; a change to either is a re-pin,
# which is the owner's call and never a silent edit. UNIX.
#
# THIS RECIPE DID NOT COMPARE ANYTHING UNTIL 2026-07-20. It printed the expected hash and then ran
# `run_world | grep 'final state_hash'`, and that grep matches ANY hash, so the recipe exited 0
# whatever the world produced. A wrong digest satisfied it, and the only thing standing between a
# silent re-pin and the repository was whether a human read two hex strings carefully. CI never
# invoked it either. The comparison is real now and a mismatch exits non-zero.
#
# READ THE SCOPE BEFORE TRUSTING A GREEN RESULT. `run_world` is a QUARANTINED dev-fixture harness on
# `Profile::Development`, and its reachability into the physics arc is zero on every symbol
# (`geodynamics`, `deeptime`, `flexur`, `thermoelastic`, `conductivity`, `province`,
# `moment_equivalence`). So a match here proves the dawn fixture replayed, and proves nothing about
# the abiotic physics. See `docs/working/UNTANGLE_PLAN.md` for what would.
pins:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --release --example run_world -p civsim-sim
    fail=0
    check() {
        local label="$1" expected="$2"; shift 2
        local got
        got="$(./target/release/examples/run_world "$@" | sed -n 's/.*final state_hash: \([0-9a-f]*\).*/\1/p' | tail -1)"
        if [ "$got" = "$expected" ]; then
            echo "  $label OK   $got"
        else
            echo "  $label FAIL expected $expected, got ${got:-<none>}"
            fail=1
        fi
    }
    echo "fixture digests (dawn harness; reaches no abiotic physics):"
    check "default" 40fe8a7269ee4da8974eb1787338c3a0
    check "living " be94e3100b9db82f7c1aea1d8091956d --scenario living
    if [ "$fail" -ne 0 ]; then
        echo "A digest moved. That is a re-pin and it is the owner's call: do not edit the expectation to match." >&2
        exit 1
    fi

# ---------------------------------------------------------------------------
# Housekeeping
# ---------------------------------------------------------------------------

# Bound the cargo build artifacts (LRU ring buffer under a size and worktree-count cap). UNIX.
gc:
    bash scripts/target_gc.sh --verbose

# Report what the artifact GC would remove, without removing it. UNIX.
gc-dry:
    bash scripts/target_gc.sh --dry-run --verbose
