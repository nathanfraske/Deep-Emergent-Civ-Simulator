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

"""The quarantine gate: keeps the living-world quarantine ledger honest against the source.

The ledger (docs/working/quarantine_ledger.toml) inventories the parked living-world reserved values that
carry no provenance or deprecated/outdated provenance under the value-authoring line, isolated from active
development until each owes its derive-or-cite. The values are NOT removed (the parked world still runs on
them); they are quarantined: named in one register, flagged by defect class, and frozen so active work does
not build on them as if they were sound.

This gate is the honesty check on that register. It fails CI when the ledger drifts out of sync with the
code: a malformed entry, an unknown verdict or defect class, a file that no longer exists, or an anchor that
no longer resolves in its file. The last case is the useful one: when someone edits or removes a quarantined
site (paying the debt, or moving the value), the anchor stops resolving and the gate fails, forcing the
ledger entry to be updated or retired rather than left stale. It does not (and cannot cheaply) prove that no
NEW un-provenanced value has entered the living world; that stays a review responsibility, stated here so the
limit is honest.

Usage:
  scripts/quarantine_gate.py             verify the ledger against the source; exit non-zero on drift
  scripts/quarantine_gate.py --self-test parser + resolver self-check on a synthetic ledger
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LEDGER = ROOT / "docs" / "working" / "quarantine_ledger.toml"

REQUIRED_FIELDS = ["id", "file", "anchor", "value", "shapes", "verdict", "defect_class", "owed"]
VERDICTS = {"no-provenance", "deprecated-provenance"}
DEFECT_CLASSES = {
    "bare-literal-no-basis",
    "basis-without-citation",
    "earth-or-terran-tuned",
    "interim-placeholder-label",
    "authored-should-derive-from-body-or-floor",
    "terran-biology-assumption",
    "other",
}


def parse_entries(text):
    """Parse the `[[entry]]` blocks into a list of dicts, matching the repo's hand-rolled toml convention
    (no tomllib dependency, the same shape as scripts/provenance_gate.py). A `key = "value"` line inside a
    block sets that key; blank lines and `#` comments are ignored."""
    entries = []
    current = None
    for raw in text.splitlines():
        line = raw.strip()
        if line.startswith("#") or not line:
            continue
        if line == "[[entry]]":
            current = {}
            entries.append(current)
            continue
        if current is None:
            continue
        m = re.match(r'^(\w+)\s*=\s*"(.*)"\s*$', line)
        if m:
            current[m.group(1)] = m.group(2)
    return entries


def verify(entries, file_reader):
    """Return a list of human-readable problems (empty means clean). `file_reader(path)` returns the file's
    text or None if it does not exist, so the resolver is testable without touching disk."""
    problems = []
    seen_ids = set()
    for i, e in enumerate(entries):
        tag = e.get("id", f"entry#{i + 1}")
        for field in REQUIRED_FIELDS:
            if not e.get(field):
                problems.append(f"{tag}: missing required field '{field}'")
        if e.get("id"):
            if e["id"] in seen_ids:
                problems.append(f"{tag}: duplicate id")
            seen_ids.add(e["id"])
        if e.get("verdict") and e["verdict"] not in VERDICTS:
            problems.append(f"{tag}: unknown verdict '{e['verdict']}'")
        if e.get("defect_class") and e["defect_class"] not in DEFECT_CLASSES:
            problems.append(f"{tag}: unknown defect_class '{e['defect_class']}'")
        # Resolve the anchor (or, failing that, the value) in the named file: a quarantined site that no
        # longer resolves is a stale entry the ledger owes an update.
        if e.get("file"):
            text = file_reader(e["file"])
            if text is None:
                problems.append(f"{tag}: file '{e['file']}' does not exist")
            else:
                anchor = e.get("anchor", "")
                value = e.get("value", "")
                if anchor and anchor not in text and (not value or value not in text):
                    problems.append(
                        f"{tag}: anchor no longer resolves in {e['file']} "
                        f"(the site changed; update or retire the entry)"
                    )
    return problems


def summarize(entries):
    by_verdict = {}
    by_class = {}
    for e in entries:
        by_verdict[e.get("verdict", "?")] = by_verdict.get(e.get("verdict", "?"), 0) + 1
        by_class[e.get("defect_class", "?")] = by_class.get(e.get("defect_class", "?"), 0) + 1
    return by_verdict, by_class


def self_test():
    good = """
# a synthetic ledger
[[entry]]
id = "q-demo-1"
file = "SYNTH.rs"
anchor = "const GROWTH_RATE"
value = "Fixed::from_ratio(3, 10)"
shapes = "the plant growth rate"
verdict = "no-provenance"
defect_class = "bare-literal-no-basis"
owed = "derive-from-photosynthesis-flux"
"""
    entries = parse_entries(good)
    assert len(entries) == 1, "one entry parses"
    reader_ok = lambda p: "const GROWTH_RATE: Fixed = Fixed::from_ratio(3, 10);" if p == "SYNTH.rs" else None
    assert verify(entries, reader_ok) == [], "a resolving entry is clean"
    reader_gone = lambda p: "// the growth rate now derives" if p == "SYNTH.rs" else None
    stale = verify(entries, reader_gone)
    assert any("no longer resolves" in p for p in stale), "a removed anchor is caught as stale"
    reader_missing = lambda p: None
    assert any("does not exist" in p for p in verify(entries, reader_missing)), "a missing file is caught"
    bad = parse_entries('[[entry]]\nid = "x"\nfile = "SYNTH.rs"\nverdict = "made-up"\n')
    assert any("unknown verdict" in p for p in verify(bad, reader_ok)), "an unknown verdict is caught"
    assert any("missing required field" in p for p in verify(bad, reader_ok)), "a missing field is caught"
    print("quarantine gate self-test: ok")


def main():
    if "--self-test" in sys.argv:
        self_test()
        return 0
    if not LEDGER.exists():
        print(f"quarantine gate: ledger not found at {LEDGER}")
        return 1
    entries = parse_entries(LEDGER.read_text())

    def reader(path):
        p = ROOT / path
        return p.read_text() if p.exists() else None

    problems = verify(entries, reader)
    by_verdict, by_class = summarize(entries)
    if problems:
        print(f"quarantine gate: {len(problems)} problem(s) against {len(entries)} ledger entries:")
        for p in problems:
            print(f"  - {p}")
        return 1
    verdicts = ", ".join(f"{k} {v}" for k, v in sorted(by_verdict.items()))
    classes = ", ".join(f"{k} {v}" for k, v in sorted(by_class.items()))
    print(f"quarantine gate: clean ({len(entries)} quarantined living-world values; {verdicts})")
    print(f"  by defect class: {classes}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
