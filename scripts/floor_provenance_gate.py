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
    """Split a floor manifest into (kind, id, has_provenance) per `[[kind]]` block."""
    blocks = []
    kind = None
    idv = None
    has_prov = False
    started = False

    def flush():
        if started and kind is not None:
            blocks.append({"kind": kind, "id": idv, "has_provenance": has_prov})

    for line in text.splitlines():
        m = re.match(r"^\[\[(\w+)\]\]", line)
        if m:
            flush()
            kind = m.group(1)
            idv = None
            has_prov = False
            started = True
            continue
        if not started:
            continue
        mid = re.match(r'^id = "([^"]*)"', line)
        if mid and idv is None:
            idv = mid.group(1)
        mp = re.match(r'^(real|fantasy) = "(.*)"', line)
        if mp and mp.group(2).strip():
            has_prov = True
    flush()
    return blocks


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
    """Run every floor manifest; return (total_counted, {manifest: [missing ids]})."""
    total = 0
    failures = {}
    for path in sorted(FLOOR_DIR.glob("*.toml")):
        if path.name in DERIVED_PROVENANCE_MANIFESTS:
            continue
        missing, counted = check_manifest(path)
        total += counted
        if missing:
            failures[path.name] = missing
    return total, failures


def main():
    total, failures = check()
    if failures:
        print("floor-provenance gate: value entries carry NO real/fantasy provenance:")
        for name, ids in failures.items():
            for i in ids:
                print(f"  - {name}: {i}")
        print(
            "floor-provenance gate: FAILED (every floor axis/substance/element must declare real or fantasy;"
            " see docs/working/PROVENANCE_PHASE2_FLOOR_UNIFICATION.md)"
        )
        return 1
    print(f"floor-provenance gate: clean ({total} floor value entries, all born provenance-tagged)")
    return 0


def self_test():
    """Prove a synthetic missing-provenance block is caught, and a law block is exempt."""
    good = '[[axis]]\nid = "x.a"\ntier = 0\nreal = "a source"\n'
    blocks = parse_blocks(good)
    assert blocks == [{"kind": "axis", "id": "x.a", "has_provenance": True}], blocks
    bad = '[[axis]]\nid = "x.b"\ntier = 0\n'
    blocks = parse_blocks(bad)
    assert blocks[0]["has_provenance"] is False, blocks
    # A law block declares no provenance and is exempt (not a PROVENANCE_KIND).
    law = '[[law]]\nid = "law.z"\nkernel = "k"\ntier = 0\n'
    assert parse_blocks(law)[0]["kind"] == "law"
    # An empty real string does not count as provenance.
    empty = '[[substance]]\nid = "s.c"\nreal = ""\n'
    assert parse_blocks(empty)[0]["has_provenance"] is False
    print("floor-provenance gate self-test: passed")
    return 0


if __name__ == "__main__":
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    sys.exit(main())
