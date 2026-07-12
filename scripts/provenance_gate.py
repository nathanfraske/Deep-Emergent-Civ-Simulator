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

"""The provenance-register gate (docs/PROVENANCE_LEDGER.md, the owner's mandated-constant register).

The fast structural sibling of the born-provenance `#[test]` in crates/sim/src/calibration.rs, in the
family of constructor_gate / determinism_gate. It fails CI the moment a reserved value ships without one
of the seven provenance tags, so the register can never silently drift back to empty (the failure that
made it mandatory: a designed-but-optional ledger stayed 0-of-228 tagged).

The seven tags: derived, measured, estimator, closure, authored, written_state, contingency. This gate
checks the STRUCTURAL invariant (every entry carries one of the seven, and a derived value declares its
DAG edges); the Rust test carries the semantic invariant (the acyclic DAG, the worst-case join, and the
category-provenance consistency), because those need the loader. Two gates, one register.

The honesty number (the count of closure-plus-authored on the authoring surface, after DAG reachability)
is reported by the Rust `authoring_surface` query, not here; this gate only proves the register is full.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "calibration" / "reserved.toml"

SEVEN_TAGS = {
    "derived",
    "measured",
    "estimator",
    "closure",
    "authored",
    "written_state",
    "contingency",
}


def parse_entries(text):
    """Split the TOML into [[reserved]] blocks and pull each id, provenance, and inputs presence."""
    entries = []
    for block in re.split(r"\n\[\[reserved\]\]\n", text):
        m = re.search(r'^id = "([^"]+)"', block, re.M)
        if not m:
            continue
        idv = m.group(1)
        prov = re.search(r'^provenance = "([^"]*)"', block, re.M)
        prov = prov.group(1).strip() if prov else ""
        has_inputs = re.search(r"^inputs = \[", block, re.M) is not None
        entries.append({"id": idv, "provenance": prov, "has_inputs": has_inputs})
    return entries


def check(text):
    entries = parse_entries(text)
    untagged = [e["id"] for e in entries if e["provenance"] == ""]
    unknown = [
        (e["id"], e["provenance"])
        for e in entries
        if e["provenance"] != "" and e["provenance"] not in SEVEN_TAGS
    ]
    derived_no_inputs = [
        e["id"] for e in entries if e["provenance"] == "derived" and not e["has_inputs"]
    ]
    non_derived_with_inputs = [
        e["id"]
        for e in entries
        if e["provenance"] in SEVEN_TAGS
        and e["provenance"] != "derived"
        and e["has_inputs"]
    ]
    return entries, untagged, unknown, derived_no_inputs, non_derived_with_inputs


def main():
    text = MANIFEST.read_text()
    entries, untagged, unknown, derived_no_inputs, non_derived_with_inputs = check(text)
    ok = True
    if untagged:
        ok = False
        print(f"provenance gate: {len(untagged)} reserved value(s) carry NO provenance tag:")
        for i in untagged:
            print(f"  - {i}")
    if unknown:
        ok = False
        print("provenance gate: unknown provenance tag(s) (not one of the seven):")
        for i, p in unknown:
            print(f"  - {i}: '{p}'")
    if derived_no_inputs:
        ok = False
        print("provenance gate: derived value(s) with no `inputs` DAG edges:")
        for i in derived_no_inputs:
            print(f"  - {i}")
    if non_derived_with_inputs:
        ok = False
        print("provenance gate: non-derived value(s) that declare `inputs` (only derived has DAG edges):")
        for i in non_derived_with_inputs:
            print(f"  - {i}")
    if ok:
        tagged = len(entries)
        print(f"provenance gate: clean ({tagged} reserved values, all born provenance-tagged)")
        return 0
    print("provenance gate: FAILED (the register must be fully tagged; see docs/PROVENANCE_LEDGER.md)")
    return 1


def self_test():
    """Prove a synthetic untagged entry is caught."""
    bad = '\n[[reserved]]\nid = "sample.untagged"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\n'
    _, untagged, _, _, _ = check(bad)
    assert untagged == ["sample.untagged"], f"self-test failed: {untagged}"
    good = '\n[[reserved]]\nid = "sample.tagged"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\nprovenance = "closure"\n'
    _, untagged, unknown, _, _ = check(good)
    assert untagged == [] and unknown == [], f"self-test failed: {untagged} {unknown}"
    print("provenance gate self-test: passed")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    sys.exit(main())
