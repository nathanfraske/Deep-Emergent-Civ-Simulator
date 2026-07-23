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

"""Retired provenance-register diagnostic for ``parked/calibration/reserved.toml``.

This preserves the old register's structural audit after biology, civilization, and calibration were
moved out of the canonical runpath. It does not define canonical provenance, authorize an input, or
admit a value because it has a tag or citation. Canonical seven-tag and four-tier accounting lives in
``civsim-ledger`` and is subordinate to the absolute-floor admission boundary.

The parked register historically required one of the seven provenance tags and allowed the migration
sentinel ``unverified_measurement_candidate``. Its old DAG rule also allowed inputs only on a derived
entry. This diagnostic retains those historical rules so parked drift remains visible; it does not
export them as current runpath policy.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "parked" / "calibration" / "reserved.toml"

CANONICAL_TAGS = {
    "derived",
    "measured",
    "estimator",
    "closure",
    "authored",
    "written_state",
    "contingency",
}
HISTORICAL_SENTINELS = {"unverified_measurement_candidate"}
ALLOWED_TAGS = CANONICAL_TAGS | HISTORICAL_SENTINELS


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
        if e["provenance"] != "" and e["provenance"] not in ALLOWED_TAGS
    ]
    non_derived_with_inputs = [
        e["id"]
        for e in entries
        if e["provenance"] in ALLOWED_TAGS
        and e["provenance"] != "derived"
        and e["has_inputs"]
    ]
    return entries, untagged, unknown, non_derived_with_inputs


def main():
    if not MANIFEST.is_file():
        print(f"retired provenance gate: missing parked register: {MANIFEST.relative_to(ROOT)}")
        return 1
    text = MANIFEST.read_text(encoding="utf-8")
    entries, untagged, unknown, non_derived_with_inputs = check(text)
    ok = True
    if untagged:
        ok = False
        print(f"retired provenance gate: {len(untagged)} reserved value(s) carry NO provenance tag:")
        for i in untagged:
            print(f"  - {i}")
    if unknown:
        ok = False
        print("retired provenance gate: unknown provenance tag(s):")
        for i, p in unknown:
            print(f"  - {i}: '{p}'")
    if non_derived_with_inputs:
        ok = False
        print("retired provenance gate: non-derived value(s) declare `inputs` under the old DAG rule:")
        for i in non_derived_with_inputs:
            print(f"  - {i}")
    if ok:
        tagged = len(entries)
        print(f"retired provenance gate: clean ({tagged} parked values carry an allowed historical tag)")
        return 0
    print("retired provenance gate: FAILED (the parked historical register drifted)")
    return 1


def self_test():
    """Prove a synthetic untagged entry is caught."""
    bad = '\n[[reserved]]\nid = "sample.untagged"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\n'
    _, untagged, _, _ = check(bad)
    assert untagged == ["sample.untagged"], f"self-test failed: {untagged}"
    good = '\n[[reserved]]\nid = "sample.tagged"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\nprovenance = "closure"\n'
    _, untagged, unknown, non_der = check(good)
    assert untagged == [] and unknown == [], f"self-test failed: {untagged} {unknown}"
    # A derived value with no manifest inputs is accepted (it derives from code-level substrate quantities).
    der0 = '\n[[reserved]]\nid = "sample.derived"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\nprovenance = "derived"\n'
    _, untagged, unknown, non_der = check(der0)
    assert untagged == [] and unknown == [] and non_der == [], f"self-test failed: {untagged} {unknown} {non_der}"
    # A non-derived value that declares inputs is caught (a leaf has no DAG edges).
    leaf_edges = '\n[[reserved]]\nid = "sample.leaf"\nbasis = "b"\nstatus = "reserved"\nsource = "s"\nprovenance = "measured"\ninputs = ["x"]\n'
    _, _, _, non_der = check(leaf_edges)
    assert non_der == ["sample.leaf"], f"self-test failed: {non_der}"
    # The old migration sentinel remains accepted here without becoming an eighth canonical tag.
    sentinel = '\n[[reserved]]\nid = "sample.sentinel"\nprovenance = "unverified_measurement_candidate"\n'
    _, untagged, unknown, non_der = check(sentinel)
    assert untagged == [] and unknown == [] and non_der == [], f"self-test failed: {untagged} {unknown} {non_der}"
    print("retired provenance gate self-test: passed")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    sys.exit(main())
