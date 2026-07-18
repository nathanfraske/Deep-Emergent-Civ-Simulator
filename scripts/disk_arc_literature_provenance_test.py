#!/usr/bin/env python3
"""Provenance dry-run for the disk-arc literature vendoring manifest.

This is the SLIMMED-landing check (crates/physics/data/disk_arc_literature/manifest.toml). The full source
PDFs are not byte-held in the repo (about 82 MB across the entries), the standing practice for paper sources,
so this test cannot re-checksum bytes offline the way the BHAC15 held-bytes test does. What it enforces is that
every receipt is COMPLETE and WELL-FORMED, so a receipt cannot go missing or malform quietly: each source
carries a citation, a recipe url, a 64-hex sha256, a positive byte count, a known grade, a non-empty verbatim
extract, and a used-by pointer, with no duplicate names. When the physics lane recovers and an entry graduates
into the coordinator's held-bytes idiom (a run-path data column with an offline byte re-checksum), that entry's
stronger test lands there; until then this keeps the ledger honest.

Run: python3 scripts/disk_arc_literature_provenance_test.py
Self-test: python3 scripts/disk_arc_literature_provenance_test.py --self-test
"""
import sys
import re

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover (older interpreters)
    import tomli as tomllib

MANIFEST = "crates/physics/data/disk_arc_literature/manifest.toml"
GRADES = {"theory", "observation", "review"}
REQUIRED = ("name", "url", "citation", "sha256", "bytes", "grade", "extract", "used_by")
_HEX64 = re.compile(r"^[0-9a-f]{64}$")


def check(doc):
    """Return a list of problems (empty means the manifest is clean)."""
    problems = []
    sources = doc.get("source", [])
    if not sources:
        return ["manifest holds no [[source]] entries"]
    seen = set()
    for i, s in enumerate(sources):
        tag = s.get("name", f"#{i}")
        for field in REQUIRED:
            if field not in s:
                problems.append(f"{tag}: missing '{field}'")
        name = s.get("name")
        if name in seen:
            problems.append(f"{tag}: duplicate name")
        seen.add(name)
        sha = s.get("sha256", "")
        if not _HEX64.match(str(sha)):
            problems.append(f"{tag}: sha256 is not 64 lowercase hex chars")
        if not isinstance(s.get("bytes"), int) or s.get("bytes", 0) <= 0:
            problems.append(f"{tag}: bytes must be a positive integer")
        if s.get("grade") not in GRADES:
            problems.append(f"{tag}: grade '{s.get('grade')}' not in {sorted(GRADES)}")
        if not str(s.get("extract", "")).strip():
            problems.append(f"{tag}: extract is empty (the slimmed held data)")
        if not str(s.get("citation", "")).strip():
            problems.append(f"{tag}: citation is empty")
    return problems


def self_test():
    """Prove the checker fires on each defect class."""
    bad_cases = [
        {"source": []},
        {"source": [{"name": "x"}]},  # missing fields
        {"source": [
            {"name": "a", "url": "u", "citation": "c", "sha256": "z" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w"}
        ]},  # sha256 not hex
        {"source": [
            {"name": "d", "url": "u", "citation": "c", "sha256": "a" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w"},
            {"name": "d", "url": "u", "citation": "c", "sha256": "b" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w"},
        ]},  # duplicate name
    ]
    for j, case in enumerate(bad_cases):
        if not check(case):
            print(f"SELF-TEST FAIL: bad case {j} was not caught")
            return 1
    good = {"source": [{"name": "g", "url": "u", "citation": "c", "sha256": "a" * 64,
                        "bytes": 10, "grade": "observation", "extract": "e", "used_by": "w"}]}
    if check(good):
        print("SELF-TEST FAIL: a clean manifest was flagged")
        return 1
    print("disk_arc_literature provenance self-test: PASS")
    return 0


def main():
    if "--self-test" in sys.argv:
        return self_test()
    with open(MANIFEST, "rb") as fh:
        doc = tomllib.load(fh)
    problems = check(doc)
    if problems:
        print("disk_arc_literature provenance FAIL:")
        for p in problems:
            print(f"  - {p}")
        return 1
    n = len(doc["source"])
    print(f"disk_arc_literature provenance OK: {n} vendored sources, receipts complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
