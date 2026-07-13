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

"""The floor-provenance gate (provenance register Phase 2, docs/working/PROVENANCE_PHASE2_FLOOR_UNIFICATION.md).

The sibling of scripts/provenance_gate.py, one register level down: it fails CI the moment a physics-floor
value entry ships without a provenance, so the floor's mandatory real-versus-fantasy split (Part 58,
`civsim_physics::Provenance::RealWithSource`/`FantasyReserved`, read fail-loud by `provenance_from`) can
never silently drift back to an untagged value. The Rust loader already fails loud on a missing provenance;
this is the fast structural sibling that catches it at CI-lint time across every floor manifest uniformly,
before the build, exactly as provenance_gate.py does for calibration/reserved.toml.

What carries provenance, from the actual substrate (crates/physics/src/lib.rs, periodic.rs): a VALUE entry
does, a DERIVATION does not. An `[[axis]]` (QuantityAxis), a `[[substance]]` (Substance), and an
`[[element]]` (periodic Element) each carry a `provenance` field, so each manifest block of those kinds must
declare a non-empty `real = "..."` or `fantasy = "..."`. A `[[law]]` (InteractionLaw) is a derivation that
computes an output from inputs and carries no provenance value (it is implicitly derived), so it is exempt.
`phase_registry.toml` is exempt entirely: a phase's provenance DERIVES from its constituent substances (the
DAG join one level up, the gate's seam-1 ruling), so a phase authors no real/fantasy field any more than it
authors its density; slice 2 wires that derivation. Phase 2 refines these two tags into the seven-tag
register; this slice-1 gate establishes the enforcement surface, byte-neutral.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FLOOR_DIR = ROOT / "crates" / "physics" / "data"

# The block kinds that carry a `provenance` value in the Rust struct, so their manifest blocks must declare
# real or fantasy. A `law` is a derivation (no provenance value); any other kind is skipped.
PROVENANCE_KINDS = {"axis", "substance", "element"}
# Manifests whose entries derive their provenance rather than declaring it (seam-1 ruling), exempt here.
DERIVED_PROVENANCE_MANIFESTS = {"phase_registry.toml"}


def parse_blocks(text):
    """Split a floor manifest into (kind, id, has_provenance, bucket) per `[[kind]]` block."""
    blocks = []
    kind = None
    idv = None
    has_prov = False
    bucket = None
    started = False

    def flush():
        if started and kind is not None:
            blocks.append(
                {"kind": kind, "id": idv, "has_provenance": has_prov, "bucket": bucket}
            )

    for line in text.splitlines():
        m = re.match(r"^\[\[(\w+)\]\]", line)
        if m:
            flush()
            kind = m.group(1)
            idv = None
            has_prov = False
            bucket = None
            started = True
            continue
        if not started:
            continue
        mid = re.match(r'^id = "([^"]*)"', line)
        if mid and idv is None:
            idv = mid.group(1)
        msym = re.match(r'^symbol = "([^"]*)"', line)
        if msym and idv is None:
            idv = msym.group(1)  # elements key by symbol
        mp = re.match(r'^(real|fantasy) = "(.*)"', line)
        if mp and mp.group(2).strip():
            has_prov = True
            bucket = mp.group(1)
    flush()
    return blocks


# The seven register tags and the two-tag -> seven-tag consistency, the floor's analog of the calibration
# category<->provenance gate. The two are RELATED but not strictly nested: the inline real/fantasy flags
# whether a CITATION is present, and the grade is the refined provenance of the VALUE. A citation can be for
# the value ITSELF (measured, a cited derivation, a cited band) OR for the SHAPE/METHOD/DEFINITION while the
# value is chosen or sampled (the Hill-equation citation whose n is "the owner's per-class call"; the
# definitional Archimedes citation whose immersed volume is a per-object slot). So a `real` entry can carry
# any grade except a written-state (a floor value is never the sim's evolved history). A `fantasy` entry has
# NO observation behind it, so it can never be measured or a banded estimator: fantasy -> authored, closure,
# derived, or contingency. The load-bearing catch is therefore a fantasy value laundered as measured.
SEVEN_GRADES = {"measured", "derived", "estimator", "closure", "authored", "contingency", "written_state"}
CONSISTENT = {
    "real": {"measured", "derived", "estimator", "closure", "authored", "contingency"},
    "fantasy": {"authored", "closure", "derived", "contingency"},
}
GRADE_REGISTER = FLOOR_DIR / "floor_provenance.toml"


def parse_grade_register(text):
    """Parse floor_provenance.toml into {id: {grade, has_derived_from}} per `[[grade]]` block."""
    reg = {}
    idv = grade = None
    has_df = False
    started = False

    def flush():
        if started and idv is not None:
            reg[idv] = {"grade": grade, "has_derived_from": has_df}

    for line in text.splitlines():
        if re.match(r"^\[\[grade\]\]", line):
            flush()
            idv = grade = None
            has_df = False
            started = True
            continue
        if not started:
            continue
        mid = re.match(r'^id = "([^"]*)"', line)
        if mid:
            idv = mid.group(1)
        mg = re.match(r'^grade = "([^"]*)"', line)
        if mg:
            grade = mg.group(1)
        if re.match(r"^derived_from = \[", line):
            has_df = True
    flush()
    return reg


def _grade_problems_for(manifest_entries, reg):
    """The pure cross-validation logic: every manifest value entry has a one-of-seven grade consistent with
    its real/fantasy bucket, a derived grade carries derived_from, and no grade is an orphan. Returns
    (id, problem) pairs. Takes the parsed register so it is testable without the file."""
    problems = []
    manifest_ids = {e["id"] for e in manifest_entries}
    bucket_by_id = {e["id"]: e["bucket"] for e in manifest_entries}
    for e in manifest_entries:
        idv = e["id"]
        g = reg.get(idv)
        if g is None:
            problems.append((idv, "no grade in floor_provenance.toml"))
            continue
        grade = g["grade"]
        if grade not in SEVEN_GRADES:
            problems.append((idv, f"grade '{grade}' is not one of the seven"))
            continue
        bucket = bucket_by_id.get(idv)
        if bucket in CONSISTENT and grade not in CONSISTENT[bucket]:
            problems.append((idv, f"{bucket}-bucketed entry graded '{grade}' (allowed: {sorted(CONSISTENT[bucket])})"))
        if grade == "derived" and not g["has_derived_from"]:
            problems.append((idv, "derived grade with no derived_from"))
    for idv in reg:
        if idv not in manifest_ids:
            problems.append((idv, "grade-register id not present in any floor manifest"))
    return problems


def check_grades(manifest_entries):
    """Cross-validate the real grade register against the manifest value entries. Returns (id, problem)."""
    reg = parse_grade_register(GRADE_REGISTER.read_text())
    return _grade_problems_for(manifest_entries, reg)


def check_manifest(path):
    """Return the ids of provenance-carrying blocks in this manifest that declare no real/fantasy."""
    text = path.read_text()
    missing = []
    counted = 0
    for b in parse_blocks(text):
        if b["kind"] not in PROVENANCE_KINDS:
            continue
        counted += 1
        if not b["has_provenance"]:
            missing.append(b["id"] if b["id"] is not None else "<no id>")
    return missing, counted


def check():
    """Run every floor manifest; return (total_counted, {manifest: [missing ids]}, manifest_entries).
    `manifest_entries` is [{id, bucket}] for every provenance-carrying block, for the grade cross-check."""
    total = 0
    failures = {}
    manifest_entries = []
    for path in sorted(FLOOR_DIR.glob("*.toml")):
        if path.name in DERIVED_PROVENANCE_MANIFESTS or path.name == GRADE_REGISTER.name:
            continue
        text = path.read_text()
        missing = []
        for b in parse_blocks(text):
            if b["kind"] not in PROVENANCE_KINDS:
                continue
            total += 1
            if not b["has_provenance"]:
                missing.append(b["id"] if b["id"] is not None else "<no id>")
            else:
                manifest_entries.append({"id": b["id"], "bucket": b["bucket"]})
        if missing:
            failures[path.name] = missing
    # The candidate phases (phase_registry.toml, seam-1 reconciled by the two-axis distinction): they carry
    # no inline real/fantasy (their thermodynamic data is cited via per-property `source =`), so they are
    # exempt from the real/fantasy completeness check, but each MUST carry a grade in the register keyed
    # "phase.<name>" (measured plus a derive-first defect). Bucket None, so the real/fantasy-to-grade
    # consistency is skipped; only completeness, one-of-seven, and derived-needs-derived_from apply.
    phase_path = FLOOR_DIR / "phase_registry.toml"
    if phase_path.exists():
        for m in re.finditer(r'^name = "([^"]+)"', phase_path.read_text(), re.M):
            total += 1
            manifest_entries.append({"id": f"phase.{m.group(1)}", "bucket": None})
    return total, failures, manifest_entries


def main():
    total, failures, manifest_entries = check()
    ok = True
    if failures:
        ok = False
        print("floor-provenance gate: value entries carry NO real/fantasy provenance:")
        for name, ids in failures.items():
            for i in ids:
                print(f"  - {name}: {i}")
    grade_problems = check_grades(manifest_entries)
    if grade_problems:
        ok = False
        print("floor-provenance gate: grade-register problems (floor_provenance.toml):")
        for i, p in grade_problems:
            print(f"  - {i}: {p}")
    if ok:
        surface = sum(
            1
            for g in parse_grade_register(GRADE_REGISTER.read_text()).values()
            if g["grade"] in ("closure", "authored")
        )
        print(
            f"floor-provenance gate: clean ({total} floor value entries, all born provenance-tagged and"
            f" seven-tag graded; floor authoring surface {surface})"
        )
        return 0
    print(
        "floor-provenance gate: FAILED (every floor axis/substance/element must declare real or fantasy and"
        " a consistent seven-tag grade; see docs/working/PROVENANCE_PHASE2_FLOOR_UNIFICATION.md)"
    )
    return 1


def self_test():
    """Prove a synthetic missing-provenance block is caught, a law block is exempt, and the grade
    consistency check fires on a mislabel."""
    good = '[[axis]]\nid = "x.a"\ntier = 0\nreal = "a source"\n'
    blocks = parse_blocks(good)
    assert blocks == [
        {"kind": "axis", "id": "x.a", "has_provenance": True, "bucket": "real"}
    ], blocks
    bad = '[[axis]]\nid = "x.b"\ntier = 0\n'
    blocks = parse_blocks(bad)
    assert blocks[0]["has_provenance"] is False and blocks[0]["bucket"] is None, blocks
    # A law block declares no provenance and is exempt (not a PROVENANCE_KIND).
    law = '[[law]]\nid = "law.z"\nkernel = "k"\ntier = 0\n'
    assert parse_blocks(law)[0]["kind"] == "law"
    # An empty real string does not count as provenance.
    empty = '[[substance]]\nid = "s.c"\nreal = ""\n'
    assert parse_blocks(empty)[0]["has_provenance"] is False
    # The grade register parses and the consistency check fires: a real-bucketed entry graded closure is
    # inconsistent (real -> {measured, derived, estimator}); an untagged manifest entry is caught.
    reg = parse_grade_register('[[grade]]\nid = "x.a"\ngrade = "closure"\n')
    assert reg == {"x.a": {"grade": "closure", "has_derived_from": False}}, reg
    # Directly exercise the consistency logic without touching the real register file. A real-bucketed
    # closure IS allowed (a cited shape with a chosen value), but a fantasy-bucketed measured is the
    # load-bearing catch (a no-observation value laundered as measured).
    real_closure = _grade_problems_for(
        [{"id": "x.a", "bucket": "real"}], {"x.a": {"grade": "closure", "has_derived_from": False}}
    )
    assert real_closure == [], real_closure
    fantasy_measured = _grade_problems_for(
        [{"id": "x.a", "bucket": "fantasy"}], {"x.a": {"grade": "measured", "has_derived_from": False}}
    )
    assert any("fantasy-bucketed" in p for _, p in fantasy_measured), fantasy_measured
    # A derived grade with no derived_from is caught.
    der = _grade_problems_for(
        [{"id": "x.d", "bucket": "real"}], {"x.d": {"grade": "derived", "has_derived_from": False}}
    )
    assert any("derived_from" in p for _, p in der), der
    # An untagged manifest entry is caught.
    untagged = _grade_problems_for([{"id": "x.z", "bucket": "real"}], {})
    assert any("no grade" in p for _, p in untagged), untagged
    print("floor-provenance gate self-test: passed")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    sys.exit(main())
