#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Ratchet nondeterminism vectors at the canonical and parked boundaries.

The default mode scans only the active deterministic substrate: core, ledger, materials, physics,
planet, units, and world. It fails if a new wall-clock read, thread-identity read, unordered-container
type, or dependency on the observer crate appears. Every accepted occurrence has an inspected reason
in ``scripts/determinism_baseline.tsv``.

``--parked`` scans the retired biology, compose, foundation, and sim packages against
``parked/scripts/determinism_baseline.tsv``. That mode preserves diagnostic coverage after the move;
it is not canonical evidence or runpath admission. The legacy viewer is intentionally outside this
state-determinism scan because its wall-clock reads drive rendering and playback. The import pattern
still prevents any scanned state crate from depending on a viewer.

Both baselines are exact ratchets. A count above baseline is a new site to inspect, and a count below
baseline is a stale exemption to remove. Rayon and ``par_iter`` remain outside this grep gate because
parallel reduction order requires semantic review rather than a substring rule. Use ``--self-test``
in either mode to prove a synthetic occurrence is caught by that mode's live scan.
"""

import pathlib
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
CANONICAL_CRATES = [
    "crates/core/src",
    "crates/ledger/src",
    "crates/materials/src",
    "crates/physics/src",
    "crates/planet/src",
    "crates/planet-substrate/src",
    "crates/units/src",
    "crates/world/src",
]
PARKED_CRATES = [
    "parked/crates/bio/src",
    "parked/crates/compose/src",
    "parked/crates/foundation/src",
    "parked/crates/sim/src",
]
# The scanned vectors. Each is a plain substring; the baseline captures the exact occurrence count
# per file (imports included), so the gate is a pure introduction ratchet.
# `civsim_viewer` is the read-and-render crate (Q1 Stone 2, the viewer-import ratchet): the
# determinism crates must never import it, so an observer's render path cannot feed back into
# canonical state. Its baseline is zero (the boundary is structural today), so any import fails.
PATTERNS = [
    "Instant::now",
    "SystemTime",
    "thread::current",
    "ThreadId",
    "HashMap",
    "HashSet",
    "civsim_viewer",
]
CANONICAL_BASELINE = ROOT / "scripts" / "determinism_baseline.tsv"
PARKED_BASELINE = ROOT / "parked" / "scripts" / "determinism_baseline.tsv"


def scan(root: pathlib.Path, crates: list[str]) -> dict:
    """Count each pattern per file across the determinism crates. Returns {(pattern, relpath): count}."""
    counts = {}
    for crate in crates:
        for path in sorted((root / crate).rglob("*.rs")):
            text = path.read_text(encoding="utf-8")
            rel = path.relative_to(root).as_posix()
            for pat in PATTERNS:
                n = text.count(pat)
                if n:
                    counts[(pat, rel)] = n
    return counts


def load_baseline(path: pathlib.Path) -> dict:
    """Parse the baseline TSV into {(pattern, relpath): count}, skipping comments and blanks."""
    base = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) < 4 or not fields[3].strip():
            raise SystemExit(f"malformed baseline row (need pattern<TAB>path<TAB>count<TAB>reason): {line!r}")
        pat, rel, count = fields[0], fields[1], int(fields[2])
        if pat not in PATTERNS:
            raise SystemExit(f"unknown determinism pattern in baseline: {pat!r}")
        if count <= 0:
            raise SystemExit(f"baseline count must be positive: {line!r}")
        if (pat, rel) in base:
            raise SystemExit(f"duplicate determinism baseline row: {pat!r} in {rel}")
        base[(pat, rel)] = count
    return base


def check(root: pathlib.Path, crates: list[str], baseline_path: pathlib.Path) -> list:
    """Return a list of human-readable violations (empty when the tree matches the baseline)."""
    violations = []
    missing = [crate for crate in crates if not (root / crate).is_dir()]
    for crate in missing:
        violations.append(f"missing determinism scan root: {crate}")
    if not baseline_path.is_file():
        violations.append(f"missing determinism baseline: {baseline_path.relative_to(root).as_posix()}")
        return violations

    observed = scan(root, crates)
    baseline = load_baseline(baseline_path)
    baseline_rel = baseline_path.relative_to(root).as_posix()
    for key in sorted(observed.keys() | baseline.keys()):
        pat, rel = key
        got = observed.get(key, 0)
        want = baseline.get(key, 0)
        if got > want:
            violations.append(
                f"NEW nondeterminism vector: '{pat}' in {rel} ({got} occurrence(s), baseline {want}). "
                f"If inspection proves it deterministic, add it to {baseline_rel} with the reason; "
                f"otherwise remove it (use a counter-based source or a BTree* for anything iterated)."
            )
        elif got < want:
            violations.append(
                f"stale baseline: '{pat}' in {rel} ({got} occurrence(s), baseline {want}). "
                f"A site was removed; lower or delete its row in {baseline_rel}."
            )
    return violations


def self_test(
    root: pathlib.Path,
    crates: list[str],
    baseline_path: pathlib.Path,
    mode: str,
) -> int:
    """Prove this mode is live in an isolated fixture, never by mutating the repository worktree."""
    baseline_relative = baseline_path.relative_to(root)
    with tempfile.TemporaryDirectory(prefix=f"determinism-{mode}-") as tmp:
        fixture_root = pathlib.Path(tmp)
        for crate in crates:
            (fixture_root / crate).mkdir(parents=True, exist_ok=True)
        fixture_baseline = fixture_root / baseline_relative
        fixture_baseline.parent.mkdir(parents=True, exist_ok=True)
        fixture_baseline.write_text("", encoding="utf-8")
        probe = fixture_root / crates[0] / "__determinism_gate_probe.rs"
        probe.write_text(
            "let _ = std::time::Instant::now();\n",
            encoding="utf-8",
        )
        violations = check(fixture_root, crates, fixture_baseline)
    hit = any("__determinism_gate_probe.rs" in v for v in violations)
    if hit:
        print(f"determinism gate {mode} self-test: PASS (a synthetic new Instant::now is caught)")
        return 0
    print(f"determinism gate {mode} self-test: FAIL (the gate did not catch a synthetic new vector)")
    return 1


def main() -> int:
    args = set(sys.argv[1:])
    unknown = args - {"--parked", "--self-test"}
    if unknown:
        print(f"determinism gate: unknown argument(s): {', '.join(sorted(unknown))}")
        return 2

    parked = "--parked" in args
    mode = "parked" if parked else "canonical"
    crates = PARKED_CRATES if parked else CANONICAL_CRATES
    baseline_path = PARKED_BASELINE if parked else CANONICAL_BASELINE
    if "--self-test" in args:
        return self_test(ROOT, crates, baseline_path, mode)

    violations = check(ROOT, crates, baseline_path)
    if violations:
        print(f"determinism gate ({mode}): FAIL")
        for v in violations:
            print(f"  - {v}")
        return 1
    qualifier = "active canonical" if not parked else "retired parked"
    print(f"determinism gate ({mode}): clean ({qualifier} crates match their inspected baseline)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
