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

"""Q1 Stone 1, item 0: the determinism grep gate.

Fail if a NEW nondeterminism vector appears in the determinism-critical crates (core, physics, sim,
world): a wall-clock read (Instant::now, SystemTime), a thread-identity read (thread::current,
ThreadId), or an unordered-container type (HashMap, HashSet) whose iteration order is process-random
and would leak into canonical state if the container were iterated.

The current tree is provably deterministic (the five canonical pins reproduce bit-exact across
separate runs, which a state-feeding wall-clock or hash-order iteration would break), yet it carries
a small baseline of these patterns that are determinism-safe for a documented reason: timing
instrumentation whose elapsed time is reported and never stored, and lookup-only maps never iterated
into state. That baseline lives in scripts/determinism_baseline.tsv with the reason per site. This
gate ratchets INTRODUCTION: it fails on any count above the baseline (a new vector to review) and on
any count below it (update the baseline, with a reason, when a site is legitimately removed).

Rayon / par_iter is deliberately not scanned: the engine's parallel reductions are order-independent
by design (the sanctioned parallelism, proven by the width-invariant pins), not a hazard. The gate
ratchets the introduction of a vector; it does not prove a grandfathered map stays lookup-only, which
stays the reviewer's job at the site. Run `--self-test` to prove the gate fails on a synthetic new
occurrence.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CRATES = [
    "crates/core/src",
    "crates/physics/src",
    "crates/bio/src",
    "crates/foundation/src",
    "crates/sim/src",
    "crates/world/src",
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
BASELINE = ROOT / "scripts" / "determinism_baseline.tsv"


def scan(root: pathlib.Path) -> dict:
    """Count each pattern per file across the determinism crates. Returns {(pattern, relpath): count}."""
    counts = {}
    for crate in CRATES:
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
        if len(fields) < 3:
            raise SystemExit(f"malformed baseline row (need pattern<TAB>path<TAB>count<TAB>reason): {line!r}")
        pat, rel, count = fields[0], fields[1], int(fields[2])
        base[(pat, rel)] = count
    return base


def check(root: pathlib.Path) -> list:
    """Return a list of human-readable violations (empty when the tree matches the baseline)."""
    observed = scan(root)
    baseline = load_baseline(BASELINE)
    violations = []
    for key in sorted(observed.keys() | baseline.keys()):
        pat, rel = key
        got = observed.get(key, 0)
        want = baseline.get(key, 0)
        if got > want:
            violations.append(
                f"NEW nondeterminism vector: '{pat}' in {rel} ({got} occurrence(s), baseline {want}). "
                f"If it is determinism-safe (timing reported not stored, a lookup-only map never "
                f"iterated into state), add it to scripts/determinism_baseline.tsv with the reason; "
                f"otherwise remove it (use a counter-based source or a BTree* for anything iterated)."
            )
        elif got < want:
            violations.append(
                f"stale baseline: '{pat}' in {rel} ({got} occurrence(s), baseline {want}). "
                f"A site was removed; lower or delete its row in scripts/determinism_baseline.tsv."
            )
    return violations


def self_test(root: pathlib.Path) -> int:
    """Prove the gate is live: a synthetic new `Instant::now` in a determinism crate must fail."""
    probe = root / "crates" / "core" / "src" / "__determinism_gate_probe.rs"
    probe.write_text("let _ = std::time::Instant::now();\n", encoding="utf-8")
    try:
        violations = check(root)
    finally:
        probe.unlink()
    hit = any("__determinism_gate_probe.rs" in v for v in violations)
    if hit:
        print("determinism gate self-test: PASS (a synthetic new Instant::now is caught)")
        return 0
    print("determinism gate self-test: FAIL (the gate did not catch a synthetic new vector)")
    return 1


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test(ROOT)
    violations = check(ROOT)
    if violations:
        print("determinism gate: FAIL")
        for v in violations:
            print(f"  - {v}")
        return 1
    print("determinism gate: clean (the determinism crates match the proven-safe baseline)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
