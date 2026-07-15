#!/usr/bin/env python3
# Dry-run provenance battery for the thermo-fetch vendored [M] columns (JANAF standard potentials + the Lodders 2003
# condensation battery). No network: it re-checks the recorded receipts against the vendored bytes, so a drifted or
# swapped table fails the build. Mirrors scripts/aesopus_provenance_test.py's check_optical_manifest idiom.
#
# JANAF: for every [[species]] in data/janaf/manifest.toml, md5 the named .txt and assert it matches the manifest
# receipt (the recipe/receipt immutability rule: the md5 is the immutable receipt of the fetched source bytes), and
# cross-check the file header's phase against the manifest phase so a swapped file is caught.
#
# LODDERS: the source is a transcription of a PDF table (no byte-identical source column to md5), so the battery
# re-checks the owner's pre-registered fingerprint rows against the transcribed TOML (the same rows the Rust #[test]
# guards) and confirms the cited source-PDF md5 receipt is recorded in the header.
import hashlib
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")


def check_janaf_manifest():
    text = open(os.path.join(DATA, "janaf", "manifest.toml"), encoding="utf-8").read()
    failures = 0
    n = 0
    for block in re.split(r"\[\[species\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"((?:[^"\\]|\\.)*)"', block))
        # The header comment mentions the literal "[[species]]"; only a real block carries a file field.
        if "file" not in g or "md5" not in g:
            continue
        n += 1
        path = os.path.join(DATA, "janaf", g["file"])
        if not os.path.exists(path):
            print("JANAF MISSING", g["file"])
            failures += 1
            continue
        raw = open(path, "rb").read()
        got = hashlib.md5(raw).hexdigest()
        if got != g["md5"]:
            print("JANAF MD5 DRIFT", g["file"], "manifest", g["md5"], "file", got)
            failures += 1
        # Cross-check: the file header's second field (e.g. "H2O1(g)") carries the phase in parentheses; it must
        # match the manifest phase, so a file swapped for the wrong phase/species is caught.
        head = raw.decode("utf-8", "replace").splitlines()[0]
        m = re.search(r"\(([a-z,]+)\)\s*$", head)
        file_phase = m.group(1) if m else "?"
        if file_phase != g["phase"]:
            print("JANAF PHASE MISMATCH", g["file"], "manifest", g["phase"], "file", file_phase)
            failures += 1
    assert failures == 0, f"{failures} JANAF provenance mismatch(es)"
    print(f"PASS: {n} JANAF species match their manifest md5 and phase")


# The owner's pre-registered transcription fingerprints for Lodders 2003 Table 8 (50% TC, K; water ice = H onset).
LODDERS_FINGERPRINTS = {
    "Al": ("t50_k", 1653),
    "Ca": ("t50_k", 1517),
    "Ti": ("t50_k", 1582),
    "Fe": ("t50_k", 1334),
    "Mg": ("t50_k", 1336),
    "Si": ("t50_k", 1310),
    "S": ("t50_k", 664),
    "H": ("t_first_k", 182),
}


def parse_condensation_rows(text):
    rows = {}
    for block in re.split(r"\[\[condensation\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"([^"]*)"', block))
        if "element" in g:
            rows[g["element"]] = g
    return rows


def check_lodders():
    path = os.path.join(DATA, "condensation_lodders2003.toml")
    text = open(path, encoding="utf-8").read()
    # The cited source-PDF md5 receipt must be recorded in the header.
    assert re.search(r"md5 [0-9a-f]{32}", text), "Lodders header records no source-PDF md5 receipt"
    rows = parse_condensation_rows(text)
    assert len(rows) == 83, f"expected 83 Lodders rows, got {len(rows)}"
    failures = 0
    for el, (col, want) in LODDERS_FINGERPRINTS.items():
        row = rows.get(el)
        if not row or col not in row:
            print("LODDERS MISSING", el, col)
            failures += 1
            continue
        got = float(row[col])
        if abs(got - want) > 1.0:
            print("LODDERS FINGERPRINT DRIFT", el, col, "want", want, "got", got)
            failures += 1
    assert failures == 0, f"{failures} Lodders fingerprint mismatch(es)"
    print(f"PASS: {len(rows)} Lodders rows, all {len(LODDERS_FINGERPRINTS)} fingerprints reproduce")


def main():
    check_janaf_manifest()
    check_lodders()
    print("PASS: thermo-fetch provenance battery")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAIL:", e)
        sys.exit(1)
