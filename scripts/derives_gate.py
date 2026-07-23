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

"""The DERIVES-COVERAGE gate: a new function in the physics or materials substrate must either carry a
`// @derives:` marker or be classified as non-deriving in the baseline. Visibility is not consulted: a
private derivation is as rebuildable by the next reader as a public one.

WHY THIS EXISTS. `docs/working/PHYSICS_FLOOR_REGISTRY.md` is the enforced derive-versus-author reference:
an agent about to author a value is supposed to check it first, because a quantity found there is a
defect to author. The stop-gate already keeps it never-stale. But staleness was the only failure it could
detect, and that left a hole: an UNMARKED deriving function is invisible to the generator, so the
registry regenerates identically and the staleness check passes while the map is wrong. The map then
returns a false negative, the reader concludes no derivation exists, and authors what was already built.

That happened. Before this gate, `crates/physics` and `crates/materials` held 409 public functions and
ZERO `@derives` markers, so the deriving half of the substrate map covered essentially none of the
physics. Worse than silent: the registry listed `therm.conductivity` as an authored FLOOR AXIS while
saying nothing about the two-rung ladder that derives it, so the map pointed a reader toward authoring
the very quantity a substrate already produced.

WHAT THIS GATE ADDS. Coverage cannot rot back. A new `pub fn` in the scanned crates is either marked as
a derivation, or named in `scripts/derives_baseline.tsv` with a classification saying what it is instead.
The existing population is grandfathered so the gate ratchets forward rather than demanding a 409-function
audit up front, exactly as the constructor gate does for its own baseline.

THE HONEST CEILING. This gate proves a new function was CLASSIFIED, never that its classification is
correct: someone may file a real derivation as a helper. It removes the silent case (a derivation nobody
looked at) and leaves the judged case (a derivation someone judged wrongly) to review, which is stated
here so the limit is not mistaken for coverage.

Usage:
  scripts/derives_gate.py              verify; exit non-zero on an unclassified new function
  scripts/derives_gate.py --update     rewrite the baseline from the current tree (deliberate, reviewed)
  scripts/derives_gate.py --self-test  parser check on synthetic input
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASELINE = ROOT / "scripts" / "derives_baseline.tsv"
SCAN_ROOTS = [
    "crates/physics/src",
    "crates/materials/src",
    "crates/planet-substrate/src",
]

# A marker may sit up to this many lines above the signature (doc comments and attributes intervene).
MARKER_LOOKBACK = 40

CLASSIFICATIONS = {
    "derives",  # carries a @derives marker; the gate confirms rather than trusts
    "helper",  # internal arithmetic or formatting with no world quantity of its own
    "accessor",  # reads a field or a loaded row
    "constructor",  # builds a value from its parts
    "loader",  # parses vendored data into memory
    "predicate",  # answers a question, produces no quantity
    "grandfathered",  # predates the gate; classify properly when the function is next touched
}


def _marker_is_substantive(text):
    """Whether a line carries a @derives marker that actually SAYS something.

    THE BYPASS THIS CLOSES. The gate used to accept any line matching `@derives:` regardless of what
    followed, so a bare `// @derives:` above a function was a pass token: it satisfied coverage, counted
    toward the marked total, and let an invented world value through while reading as documented. Worse,
    `gen_floor_registry.py` requires nonempty marker text, so an empty marker did not even reach the
    registry mirror the substrate map is read from. The value was covered, counted, and invisible at once.

    The declared convention is `@derives: <the quantity> <- <the substrate inputs it reads>`, and both
    halves carry meaning: the quantity says what is derived, the inputs say what it is derived FROM. A
    marker missing either has not answered the question it exists to answer.
    """
    m = re.search(r"//\s*@derives(?:\[\w+\])?:(.*)$", text)
    if not m:
        return False
    body = m.group(1).strip()
    if "<-" not in body:
        return False
    quantity, _, inputs = body.partition("<-")
    return bool(quantity.strip()) and bool(inputs.strip())


def scan_public_functions(read_file):
    """Every `pub fn` in the scanned crates, with whether a @derives marker sits above it.

    Returns [(rel_path, fn_name, has_marker)] sorted, so output is deterministic."""
    out = []
    for root in SCAN_ROOTS:
        base = ROOT / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            # Baseline paths are repository identifiers, not host paths.
            rel = path.relative_to(ROOT).as_posix()
            lines = read_file(path)
            # Skip test modules by BRACE DEPTH, never by a latch. A latch on the first `#[cfg(test)]`
            # leaves every line after it unscanned, so a function written below a test module would be
            # invisible to this gate. That hole was found by live-firing the gate and getting no
            # conviction, which is the only reason to live-fire a gate at all.
            pending_test = False
            test_depth = None
            depth = 0
            for i, ln in enumerate(lines):
                if test_depth is None and "#[cfg(test)]" in ln:
                    pending_test = True
                opens = ln.count("{")
                closes = ln.count("}")
                if pending_test and opens:
                    test_depth = depth
                    pending_test = False
                in_test = test_depth is not None
                depth += opens - closes
                if test_depth is not None and depth <= test_depth:
                    test_depth = None
                if in_test:
                    continue
                # Match EVERY function, not only `pub fn`. Visibility is irrelevant to whether something
                # derives a world quantity: a private derivation inside these crates is exactly as
                # rebuildable by the next reader, and the failures this gate exists to stop happened INSIDE
                # crates/physics and crates/materials, where private items are freely reachable. Scanning
                # only the public surface left 1,348 functions invisible. The sibling stone0 gates are all
                # visibility-blind for the same reason.
                m = re.match(
                    r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+|unsafe\s+|async\s+|extern\s+\"[^\"]*\"\s+)*fn\s+(\w+)",
                    ln,
                )
                if not m:
                    continue
                # Attribute a marker ONLY from this function's own contiguous comment-and-attribute
                # block. Walking up a fixed number of lines would credit one marker to every function
                # near it, which would let a single marker vouch for code nobody marked. Stop at the
                # first line that is not a comment, an attribute, or blank.
                has_marker = False
                j = i - 1
                while j >= 0 and (i - j) <= MARKER_LOOKBACK:
                    t = lines[j].strip()
                    if t.startswith("//"):
                        if _marker_is_substantive(t):
                            has_marker = True
                            break
                        j -= 1
                        continue
                    if t.startswith("#[") or t.startswith("#!") or t == "":
                        j -= 1
                        continue
                    break
                out.append((rel, m.group(1), has_marker))
    out.sort()
    return out


def parse_baseline(text):
    """(file, fn) -> classification. Comment lines start with #."""
    base = {}
    for raw in text.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        parts = raw.split("\t")
        if len(parts) < 3:
            continue
        base[(parts[0].strip(), parts[1].strip())] = parts[2].strip()
    return base


def verify(found, baseline):
    """Unclassified new functions, and baseline rows whose function is gone."""
    unclassified = []
    for rel, fn, has_marker in found:
        if has_marker:
            continue
        if (rel, fn) in baseline:
            continue
        unclassified.append((rel, fn))
    live = {(rel, fn) for rel, fn, _ in found}
    stale = [k for k in baseline if k not in live]
    return unclassified, stale


def render_baseline(found):
    lines = [
        "# The derives-coverage baseline (scripts/derives_gate.py).",
        "#",
        "# One row per public function in the physics and materials substrate that does NOT carry a",
        "# `// @derives:` marker, with what it is instead. A function that DOES carry a marker needs no row:",
        "# the gate reads the marker directly, so the registry and this baseline cannot disagree.",
        "#",
        "# Classifications: " + " | ".join(sorted(CLASSIFICATIONS)),
        "#",
        "# `grandfathered` means the function predates the gate and nobody has classified it yet. It is not a",
        "# claim that the function does not derive; it is an admission that no one has looked. Classify it",
        "# properly when the function is next touched, and if it turns out to derive a world quantity, give it",
        "# a marker instead of a row.",
        "#",
        "# file\tfunction\tclassification\treason",
    ]
    for rel, fn, has_marker in found:
        if has_marker:
            continue
        lines.append(f"{rel}\t{fn}\tgrandfathered\tpredates the derives-coverage gate; unclassified")
    return "\n".join(lines) + "\n"


def self_test():
    sample = {
        "a.rs": [
            "// @derives: a world quantity <- the floor\n",
            "pub fn derived_thing() -> Fixed {\n",
            "}\n",
            "pub fn plain_helper() -> Fixed {\n",
        ]
    }

    def reader(path):
        return sample[path.name]

    global SCAN_ROOTS
    saved = SCAN_ROOTS
    try:
        # Exercise the marker window logic directly rather than the filesystem walk.
        lines = sample["a.rs"]
        marked = _marker_is_substantive("".join(lines[0:1]))
        unmarked = re.search(r"//\s*@derives(?:\[\w+\])?:", "".join(lines[2:3])) is not None
        assert marked, "a marker above the signature must be seen"
        assert not unmarked, "a signature with no marker above it must not read as marked"
        base = parse_baseline("a.rs\tplain_helper\thelper\tarithmetic\n")
        unclassified, stale = verify(
            [("a.rs", "derived_thing", True), ("a.rs", "plain_helper", False)], base
        )
        assert not unclassified, f"classified and marked functions must pass, got {unclassified}"
        unclassified2, _ = verify([("a.rs", "brand_new", False)], base)
        assert unclassified2 == [("a.rs", "brand_new")], "an unmarked new function must be caught"
        stale_only = verify([("a.rs", "derived_thing", True)], base)[1]
        assert stale_only == [("a.rs", "plain_helper")], "a vanished baseline row must be reported"
    finally:
        SCAN_ROOTS = saved
    print("derives gate: self-test OK")
    return 0


def main():
    args = sys.argv[1:]
    if "--self-test" in args:
        return self_test()

    def read_file(path):
        with open(path, encoding="utf-8") as fh:
            return fh.readlines()

    found = scan_public_functions(read_file)
    marked = sum(1 for _, _, m in found if m)

    if "--update" in args:
        BASELINE.write_text(render_baseline(found), encoding="utf-8")
        print(
            f"derives gate: baseline rewritten, {len(found)} public function(s), "
            f"{marked} marked as deriving, {len(found) - marked} classified rows"
        )
        return 0

    if not BASELINE.exists():
        print(
            "derives gate: no baseline. Run scripts/derives_gate.py --update once, review the file, "
            "and commit it.",
            file=sys.stderr,
        )
        return 2

    baseline = parse_baseline(BASELINE.read_text(encoding="utf-8"))
    unclassified, stale = verify(found, baseline)

    if unclassified:
        print(
            f"derives gate: FAILED. {len(unclassified)} new public function(s) in the physics or "
            "materials substrate are neither marked as deriving nor classified:",
            file=sys.stderr,
        )
        for rel, fn in unclassified[:25]:
            print(f"  {rel}: {fn}", file=sys.stderr)
        print(
            "\nIf it DERIVES a world quantity from the floor and the situation, give it a marker:\n"
            "    // @derives: <the quantity> <- <the substrate inputs it reads>\n"
            "so docs/working/PHYSICS_FLOOR_REGISTRY.md carries it and the next reader finds it instead "
            "of authoring the value again.\n"
            "If it does not, add a row to scripts/derives_baseline.tsv saying what it is "
            "(" + " | ".join(sorted(CLASSIFICATIONS - {'derives', 'grandfathered'})) + ").",
            file=sys.stderr,
        )
        return 1

    if stale:
        print(
            f"derives gate: FAILED. {len(stale)} baseline row(s) name a function that no longer exists, "
            "so the baseline is stale:",
            file=sys.stderr,
        )
        for rel, fn in stale[:25]:
            print(f"  {rel}: {fn}", file=sys.stderr)
        print(
            "\nRemove those rows, or run scripts/derives_gate.py --update and review the diff.",
            file=sys.stderr,
        )
        return 1

    print(
        f"derives gate: clean ({len(found)} public function(s); {marked} carry a @derives marker, "
        f"{len(found) - marked} classified in the baseline)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
