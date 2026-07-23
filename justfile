# Task entry points for the Emergent Civilization Simulator.
#
# Install the runner once (https://github.com/casey/just):
#   cargo install just
# Then `just` on its own lists every recipe.
#
# Linux is the reference development environment. On Windows, use `scripts/dev.ps1` so recipes run
# inside WSL with the same Bash, Python, Rust, and GNU-tool assumptions as Linux.

canonical_packages := "-p civsim-core -p civsim-ledger -p civsim-units -p civsim-physics -p civsim-materials -p civsim-world -p civsim-planet -p civsim-viewer -p civsim-gpu -p civsim-stone0"
parked_target := env_var_or_default("CIVSIM_PARKED_TARGET_DIR", "target/parked")
cargo_dev := "bash scripts/cargo_dev.sh"

# Show the available recipes.
default:
    @just --list

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------

# Enter the canonical planet front door; incomplete physical closure emits a structured refusal.
run *args:
    {{cargo_dev}} run -p civsim-planet --bin run_planet -- {{args}}

# Compatibility name for the former derived view. This enters the same floor-only library runner and no viewer.
run-derived:
    {{cargo_dev}} run -p civsim-planet --bin run_planet

# Report the missing-floor boundary without entering a physical stage. An incomplete receipt exits non-zero.
readiness:
    {{cargo_dev}} run -p civsim-planet --bin run_planet -- --readiness

# Regenerate the centralized four-tier by seven-tag inventory from the audited catalog.
ledger-inventory:
    {{cargo_dev}} run -p civsim-planet --bin ledger_inventory -- --write

# Fail when the checked-in centralized ledger inventory does not match the audited catalog.
ledger-inventory-check:
    {{cargo_dev}} run -p civsim-planet --bin ledger_inventory -- --check

# Run the quarantined legacy dawn development fixture to its final state hash.
run-dawn-legacy:
    {{cargo_dev}} run --release --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" -p civsim-sim --example run_world

# Run the legacy living-world scenario.
run-living-legacy:
    {{cargo_dev}} run --release --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" -p civsim-sim --example run_world -- --scenario living

# Enter the snapshot-only viewer; it refuses until immutable PlanetSnapshot transport is wired.
view:
    {{cargo_dev}} run -p civsim-viewer

# Open the parked causal viewer under an explicit legacy name.
view-living-legacy:
    {{cargo_dev}} run --release --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" -p civsim-viewer-legacy

# Refuse until a canonical snapshot-only GPU adapter exists.
view-gpu:
    #!/usr/bin/env bash
    echo "No snapshot-only planetary GPU viewer exists yet. Use 'just view-living-gpu-legacy' only to inspect the old causal viewer." >&2
    exit 2

# Open the current causal viewer with GPU globe shading under an explicit legacy name.
view-living-gpu-legacy:
    {{cargo_dev}} run --release --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" -p civsim-viewer-legacy --features gpu

# ---------------------------------------------------------------------------
# Verify
# ---------------------------------------------------------------------------

# Configure this clone to use the tracked Stone 0 pre-push entrypoint.
hooks-install:
    #!/usr/bin/env bash
    set -euo pipefail
    test -x scripts/githooks/pre-push
    test -x scripts/stone0-pre-push-hook.sh
    git config --local core.hooksPath scripts/githooks
    just hooks-check

# Verify that this clone will execute the tracked pre-push entrypoint.
hooks-check:
    #!/usr/bin/env bash
    set -euo pipefail
    configured="$(git config --local --get core.hooksPath || true)"
    if [ "$configured" != "scripts/githooks" ]; then
        echo "Stone 0 pre-push hook is not installed for this clone. Run: just hooks-install" >&2
        exit 1
    fi
    test -x scripts/githooks/pre-push
    test -x scripts/stone0-pre-push-hook.sh
    printf 'Stone 0 pre-push hook: installed (%s)\n' "$configured"

# Print one tier's ordered structural gate ids from the declarative authority.
gates-list tier="pr":
    @python3 scripts/gate_runner.py list --tier {{tier}} --ids-only

# Run one declarative structural gate tier.
gates-run tier="pr":
    python3 scripts/gate_runner.py run --tier {{tier}}

# Run every declared self-test available in one structural gate tier.
gates-self-tests tier="canonical":
    python3 scripts/gate_runner.py self-tests --tier {{tier}}

# Verify developer setup, both workspace manifests, generated registries, and canonical boundaries without waiving owner gates.
doctor:
    #!/usr/bin/env bash
    set -euo pipefail
    just hooks-check
    python3 scripts/gate_runner.py --self-test
    python3 scripts/test_gate_runner.py
    bash scripts/test_target_gc.sh
    python3 scripts/gate_runner.py run --tier doctor --phase pre
    {{cargo_dev}} metadata --locked --no-deps --format-version 1 >/dev/null
    {{cargo_dev}} metadata --locked --manifest-path parked/Cargo.toml --no-deps --format-version 1 >/dev/null
    python3 scripts/gate_runner.py self-tests --tier doctor
    {{cargo_dev}} run -q -p civsim-stone0 --bin stone0-gate -- --self-test
    {{cargo_dev}} run -q -p civsim-stone0 --bin stone0-gate -- --ci
    python3 scripts/gate_runner.py run --tier doctor --phase post

# Test the canonical abiotic package set. Parked and legacy compatibility packages are intentionally separate.
test:
    {{cargo_dev}} test {{canonical_packages}} --all-targets

# Compile and test the complete parked workspace, including biology and civilization.
test-legacy:
    {{cargo_dev}} test --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" --all-targets

# Run retired calibration, profile, and quarantine ratchets against parked work only.
audit-parked:
    #!/usr/bin/env bash
    set -euo pipefail
    python3 scripts/gate_runner.py self-tests --tier legacy
    python3 scripts/gate_runner.py run --tier legacy

# Format the canonical workspace.
fmt:
    {{cargo_dev}} fmt --all

# Format the separately parked workspace.
fmt-legacy:
    {{cargo_dev}} fmt --manifest-path parked/Cargo.toml --all

# Canonical formatting check without writing.
fmt-check:
    {{cargo_dev}} fmt --all --check

# Lint the canonical abiotic package set.
lint:
    {{cargo_dev}} clippy {{canonical_packages}} --all-targets -- -D warnings

# Check formatting in the complete parked workspace.
fmt-check-legacy:
    {{cargo_dev}} fmt --manifest-path parked/Cargo.toml --all --check

# Lint the complete parked workspace, including biology and civilization.
lint-legacy:
    {{cargo_dev}} clippy --manifest-path parked/Cargo.toml --target-dir "{{parked_target}}" --all-targets -- -D warnings

# The documentation verification suite (em dashes, banned adverbs, part numbering, fences). UNIX.
verify:
    bash scripts/verify.sh

# Run one canonical quality tier around the declarative structural gates.
_ci tier:
    #!/usr/bin/env bash
    set -euo pipefail
    just fmt-check
    python3 scripts/gate_runner.py --self-test
    python3 scripts/test_gate_runner.py
    bash scripts/test_target_gc.sh
    python3 scripts/gate_runner.py run --tier {{tier}} --phase pre
    python3 scripts/gate_runner.py self-tests --tier {{tier}}
    {{cargo_dev}} run -q -p civsim-stone0 --bin stone0-gate -- --self-test
    {{cargo_dev}} run -q -p civsim-stone0 --bin stone0-gate -- --ci
    python3 scripts/gate_runner.py run --tier {{tier}} --phase post
    just lint
    just test
    RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links" {{cargo_dev}} doc {{canonical_packages}} --no-deps --document-private-items
    {{cargo_dev}} test {{canonical_packages}} --doc

# Run the canonical PR tier, including the strict single-provider Diamond scan.
ci:
    just _ci pr

# Fast, non-certifying developer loop. The PR tier remains the merge receipt.
check-fast:
    #!/usr/bin/env bash
    set -euo pipefail
    just fmt-check
    python3 scripts/gate_runner.py --self-test
    python3 scripts/test_gate_runner.py
    bash scripts/test_target_gc.sh
    python3 scripts/gate_runner.py run --tier stop --phase pre
    {{cargo_dev}} check {{canonical_packages}} --all-targets
    python3 scripts/gate_runner.py run --tier stop --phase post

# Run the same canonical recipe used by CI through the portable local wrapper.
ci-local:
    bash scripts/ci_local.sh

# Show the canonical CI recipe without running it. UNIX.
ci-list:
    bash scripts/ci_local.sh --list

# Run all parked checks; this result never supplies a canonical planetary readiness receipt.
ci-legacy: check-legacy audit-parked

# Show the legacy aggregate recipe without running it. UNIX.
ci-list-legacy:
    just --show ci-legacy

# The future required planetary pull-request tier.
check-pr: ci

# Public alias for the required planetary pull-request tier.
check: check-pr

# The complete canonical CPU tier currently runs the same quality commands through its distinct gate tier.
check-full:
    just _ci full

# Scheduled canonical checks use their distinct gate tier; parked ignored tests remain a separate CI job.
check-nightly:
    just _ci nightly

# Common checks over the legacy workspace. This is not CI parity or planetary readiness. UNIX.
check-legacy: fmt-check-legacy lint-legacy test-legacy verify

# Run the repository Stop hook against the current tree. UNIX.
stop-gate:
    #!/usr/bin/env bash
    set -euo pipefail
    printf '%s\n' '{"stop_hook_active":false}' | bash .claude/hooks/stop-gate.sh

# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------

# The two legacy fixture digests. Both must MATCH their expected hash when this compatibility check is
# invoked; neither digest is evidence for planetary readiness. A change to either is a re-pin,
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
# Compare the two legacy fixture digests; this is not planetary evidence.
pins-dawn-legacy:
    #!/usr/bin/env bash
    set -euo pipefail
    target_dir="{{parked_target}}"
    {{cargo_dev}} build --release --manifest-path parked/Cargo.toml --target-dir "$target_dir" -p civsim-sim --example run_world
    fail=0
    check() {
        local label="$1" expected="$2"; shift 2
        local got
        got="$("$target_dir/release/examples/run_world" "$@" | sed -n 's/.*final state_hash: \([0-9a-f]*\).*/\1/p' | tail -1)"
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

# Show the bounded native-WSL target and receipt locations.
cache-info:
    bash scripts/wsl_dev_env.sh --print

# Bound the native Cargo target and drain retired per-worktree targets. UNIX.
gc:
    bash scripts/target_gc.sh --verbose

# Report what the artifact GC would remove, without removing it. UNIX.
gc-dry:
    bash scripts/target_gc.sh --dry-run --verbose

# Issue an online WSL filesystem trim without stopping the distribution.
trim-wsl:
    bash scripts/wsl_trim.sh --force --verbose
