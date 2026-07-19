#!/usr/bin/env python3
"""SCHEMA + RECEIPT COMPLETENESS, NOT VALIDATION.

Proves every one of 31 sources carries a complete, well-formed receipt. It does NOT prove any extract or grade is scientifically correct.

WHY THE LABEL IS IN THE FILE. An audit found this battery described uniformly as one that "reconstructs
each fetch from its recipe and asserts byte-equality", for all eight tests. That was true of some and
false of others, and the difference matters: a custody check proves the bytes we hold are the bytes we
fetched, a transcription check proves our column matches the held source, and neither proves the source
is RIGHT. A test that reads its expectation from the file under test does not even prove independence.
Saying so here, where the test is, is harder to drift than saying it in a document.

Provenance dry-run for the disk-arc literature vendoring manifest.

This is the SLIMMED-landing check (crates/physics/data/disk_arc_literature/manifest.toml). The full source
PDFs are not byte-held in the repo (about 82 MB across the entries), the standing practice for paper sources,
so this test cannot re-checksum bytes offline the way the BHAC15 held-bytes test does. What it enforces is that
every receipt is COMPLETE and WELL-FORMED, so a receipt cannot go missing or malform quietly: each source
carries a citation, a recipe url, a 64-hex sha256, a positive byte count, a known grade, a non-empty verbatim
extract, a used-by pointer, and either a well-formed web.archive.org snapshot (archive_url) or a non-empty
archive_pending reason (a link rots; the snapshot is how the recipe survives the host moving), with no
duplicate names. When the physics lane recovers and an entry graduates
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
_WAYBACK = re.compile(r"^https://web\.archive\.org/web/\d{14}/")


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
        # Archive capture: each entry must carry a well-formed Wayback snapshot OR a
        # non-empty archive_pending reason (a link rots; the snapshot is the recipe's survival).
        has_archive = "archive_url" in s
        has_pending = str(s.get("archive_pending", "")).strip() != ""
        if has_archive:
            if not _WAYBACK.match(str(s.get("archive_url", ""))):
                problems.append(f"{tag}: archive_url is not a web.archive.org/web/<14-digit-ts>/ snapshot")
        elif not has_pending:
            problems.append(f"{tag}: missing archive_url and no archive_pending reason given")
    return problems


def self_test():
    """Prove the checker fires on each defect class."""
    ok_archive = "https://web.archive.org/web/20260718203609/https://example.org/x"
    bad_cases = [
        {"source": []},
        {"source": [{"name": "x"}]},  # missing fields
        {"source": [
            {"name": "a", "url": "u", "citation": "c", "sha256": "z" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w", "archive_url": ok_archive}
        ]},  # sha256 not hex
        {"source": [
            {"name": "d", "url": "u", "citation": "c", "sha256": "a" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w", "archive_url": ok_archive},
            {"name": "d", "url": "u", "citation": "c", "sha256": "b" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w", "archive_url": ok_archive},
        ]},  # duplicate name
        {"source": [
            {"name": "n", "url": "u", "citation": "c", "sha256": "a" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w"}
        ]},  # no archive_url and no archive_pending
        {"source": [
            {"name": "m", "url": "u", "citation": "c", "sha256": "a" * 64,
             "bytes": 1, "grade": "theory", "extract": "e", "used_by": "w",
             "archive_url": "https://example.org/not-wayback"}
        ]},  # archive_url malformed
    ]
    for j, case in enumerate(bad_cases):
        if not check(case):
            print(f"SELF-TEST FAIL: bad case {j} was not caught")
            return 1
    good = {"source": [{"name": "g", "url": "u", "citation": "c", "sha256": "a" * 64,
                        "bytes": 10, "grade": "observation", "extract": "e", "used_by": "w",
                        "archive_url": ok_archive}]}
    if check(good):
        print("SELF-TEST FAIL: a clean manifest was flagged")
        return 1
    pending = {"source": [{"name": "p", "url": "u", "citation": "c", "sha256": "a" * 64,
                           "bytes": 10, "grade": "observation", "extract": "e", "used_by": "w",
                           "archive_pending": "bot-walled, held bytes are the witness"}]}
    if check(pending):
        print("SELF-TEST FAIL: an archive_pending entry was flagged")
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
