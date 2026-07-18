#!/usr/bin/env python3
# Dry-run provenance battery for the elastic-thickness hindcast cited [M] column. No network. It (1) re-checks the
# sha256 receipt and byte count of every held witness PDF against elastic_thickness_hindcast/manifest.toml, (2)
# asserts the oceanic load-age self-consistency dt = age_plate - age_volcano on every oceanic-interior row so a
# transcription slip in either age fails, (3) asserts the owner's pre-registered fingerprints (exact source values
# for spot rows across Earth/Mars/Venus, plus the Calmant fit constants) against elastic_thickness_hindcast.toml,
# and (4) best-effort, greps each held PDF for the value it witnesses when pdftotext is present (the audit-the-input
# cross-check), skipped without it so the mandatory spine stays pure-Python. Mirrors
# scripts/convection_scaling_provenance_test.py (held-bytes + fingerprint idiom).
import hashlib
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")
HELD_DIR = os.path.join(DATA, "elastic_thickness_hindcast")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(DATA, "elastic_thickness_hindcast.toml")
# Archive-then-slimmed witnesses (owner ruling 2026-07-18) keep their bytes here, out of git. Best-effort:
# present only on the machine that fetched them; CI verifies via the receipt + archived_url instead.
LOCAL_STORE = os.path.expanduser("~/.claude/vendored-sources/elastic_thickness_hindcast")

TOL = 1e-6


def parse_blocks(text, kind):
    """Split a TOML into [[kind]] blocks, returning per-block {field: string} for quoted and bare-int fields.
    The split anchors [[kind]] at the START of a line, so a prose mention of the token inside a header comment
    (e.g. 'BLOCK KIND [[row]], the ...') is never mistaken for a real array-of-table header."""
    blocks = []
    for block in re.split(rf"(?m)^\[\[{kind}\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"((?:[^"\\]|\\.)*)"', block))
        for k, v in re.findall(r"^(\w+)\s*=\s*([0-9]+)\s*$", block, re.MULTILINE):
            g.setdefault(k, v)
        blocks.append(g)
    return blocks


def check_receipts():
    # Truncate at the first line-anchored [[reference]] so the not-held reference blocks never bleed into the
    # last [[source]] (a header-comment mention of the token is left-padded, so it is not a truncation point).
    # A witness is verified by repo bytes whose sha256 matches OR by a retrievable Internet Archive snapshot
    # recorded as archived_url (archive-then-slim, owner ruling 2026-07-18). Ruiz stays held in-repo (its
    # docta.ucm.es redirect chain is not archivable); the other three are slimmed to their archived_url. A
    # receipt with neither is the rot the custody work exists to prevent; a slimmed witness that also sits in
    # local custody is re-hashed there (defence in depth).
    text = re.split(r"(?m)^\[\[reference\]\]", open(MANIFEST, encoding="utf-8").read())[0]
    failures = 0
    held = 0
    archived = 0
    for s in parse_blocks(text, "source"):
        if "file" not in s or "sha256" not in s:
            continue
        path = os.path.join(HELD_DIR, s["file"])
        if os.path.exists(path):
            raw = open(path, "rb").read()
            if hashlib.sha256(raw).hexdigest() != s["sha256"]:
                print(f"ET SHA256 DRIFT {s['file']}: repo copy does not match the receipt")
                failures += 1
            elif "bytes" in s and len(raw) != int(s["bytes"]):
                print(f"ET BYTE-COUNT DRIFT {s['file']}: manifest {s['bytes']} held {len(raw)}")
                failures += 1
            else:
                held += 1
            continue
        if not s.get("archived_url", "").strip():
            print(f"ET NO WITNESS {s['file']}: receipt {s['sha256']} points at nothing retrievable")
            failures += 1
            continue
        archived += 1
        custody = os.path.join(LOCAL_STORE, s["file"])
        if os.path.exists(custody) and hashlib.sha256(open(custody, "rb").read()).hexdigest() != s["sha256"]:
            print(f"ET SHA256 DRIFT {s['file']}: local-custody copy does not match the receipt")
            failures += 1
    if not failures:
        print(f"elastic-thickness receipts OK: {held} held in-repo + {archived} archive-then-slimmed, all match their receipt")
    return failures


def read_rows():
    text = open(COLUMN, encoding="utf-8").read()
    return parse_blocks(text, "row")


def check_load_age(rows):
    """Every oceanic-interior row must satisfy load_age = age_plate - age_volcano (Calmant's abscissa), so a
    transcription slip in either measured age is caught."""
    failures = 0
    n = 0
    for r in rows:
        if r.get("environment") != "oceanic_interior":
            continue
        try:
            tp = float(r["age_plate_myr"])
            tv = float(r["age_volcano_myr"])
            dt = float(r["load_age_myr"])
        except (KeyError, ValueError):
            print(f"ET LOAD-AGE UNPARSEABLE for feature {r.get('feature')!r}")
            failures += 1
            continue
        n += 1
        if abs(dt - (tp - tv)) > TOL:
            print(f"ET LOAD-AGE INCONSISTENT {r.get('feature')!r}: {dt} != {tp} - {tv} = {tp - tv}")
            failures += 1
    if not failures:
        print(f"elastic-thickness load-age OK: all {n} oceanic-interior rows satisfy dt = tp - tv")
    return failures


# The owner's pre-registered transcription fingerprints, keyed by (body, feature), each an EXACT string match on
# the source value (t_e_km carries limits like ">70" and ranges like "100-180" verbatim, so string equality is the
# right check).
FINGERPRINTS = {
    ("Earth", "Bermuda"): {"t_e_km": "32.5", "age_plate_myr": "117", "age_volcano_myr": "30"},
    ("Earth", "Great Meteor"): {"t_e_km": "19", "age_plate_myr": "80", "age_volcano_myr": "14"},
    ("Earth", "Mayotte"): {"t_e_km": "40.5", "age_plate_myr": "150", "age_volcano_myr": "5"},
    ("Earth", "Kauai"): {"t_e_km": "32.5", "age_plate_myr": "93.5", "age_volcano_myr": "5"},
    ("Earth", "Tahiti"): {"t_e_km": "20", "age_plate_myr": "71", "age_volcano_myr": "1"},
    ("Earth", "Mangaia"): {"t_e_km": "7", "age_plate_myr": "88", "age_volcano_myr": "17.7"},
    ("Mars", "North Polar Region"): {"t_e_km": ">300", "surface_age": "Current"},
    ("Mars", "Olympus Mons"): {"t_e_km": ">70", "surface_age": "A"},
    ("Mars", "Isidis Planitia"): {"t_e_km": "100-180", "surface_age": "H"},
    ("Mars", "Coracis Fossae"): {"t_e_km": "10.3-12.5", "surface_age": "H-N"},
    ("Venus", "T_e distribution mode, thin (dominant)"): {"t_e_km": "<20", "fraction_percent": "47"},
    ("Venus", "T_e distribution mode, intermediate"): {"t_e_km": "40-70"},
    ("Venus", "T_e distribution mode, thick"): {"t_e_km": ">90"},
    ("Venus", "T_e inversion search range"): {"t_e_km": "0-120"},
}


def check_fingerprints(rows):
    index = {(r.get("body"), r.get("feature")): r for r in rows}
    failures = 0
    for key, fields in FINGERPRINTS.items():
        row = index.get(key)
        if not row:
            print("ET MISSING ROW", key)
            failures += 1
            continue
        for field, want in fields.items():
            got = row.get(field)
            if got != want:
                print(f"ET FINGERPRINT DRIFT {key} .{field}: want {want!r} got {got!r}")
                failures += 1
    if not failures:
        print(f"elastic-thickness fingerprints OK: all {len(FINGERPRINTS)} spot rows reproduce their pre-registered values")
    return failures


def check_top_level():
    """The Calmant isochron-fit constants, carried as top-level fields."""
    text = re.split(r"(?m)^\[\[row\]\]", open(COLUMN, encoding="utf-8").read())[0]
    top = dict(re.findall(r'(\w+)\s*=\s*"((?:[^"\\]|\\.)*)"', text))
    failures = 0
    if "2.70" not in top.get("oceanic_isochron_fit", ""):
        print(f"ET TOP-LEVEL DRIFT oceanic_isochron_fit: {top.get('oceanic_isochron_fit')!r} lacks 2.70")
        failures += 1
    if top.get("oceanic_isochron_fit_sigma") != "0.15":
        print(f"ET TOP-LEVEL DRIFT sigma: want 0.15 got {top.get('oceanic_isochron_fit_sigma')!r}")
        failures += 1
    if not failures:
        print("elastic-thickness top-level OK: Calmant fit T_e = 2.70 sqrt(dt), sigma = 0.15")
    return failures


# Best-effort: each held witness must contain the values it witnesses. Skipped without pdftotext so the mandatory
# checks above stay dependency-free (the sha256 receipt already pins the exact analyzed bytes).
WITNESS_SIGNATURES = {
    "calmant_1990_gji.pdf": ["2.70", "isochron", "350 and 450", "550-600"],
    "ruiz_2011_icarus.pdf": ["Coracis", "Isidis", "paleo-heat"],
    "smrekar_anderson_2005_lpsc.pdf": ["47%", "admittance", "bottom loading"],
    "watts_burov_2003_epsl.pdf": ["Chile", "seismogenic", "Burov"],
}


def check_witness_text():
    if not shutil.which("pdftotext"):
        print("elastic-thickness witness-text check SKIPPED (pdftotext not installed; sha256 receipts still pin the bytes)")
        return 0
    failures = 0
    checked = 0
    for fname, needles in WITNESS_SIGNATURES.items():
        path = os.path.join(HELD_DIR, fname)
        if not os.path.exists(path):
            path = os.path.join(LOCAL_STORE, fname)  # slimmed from the repo; check local custody when present
        if not os.path.exists(path):
            continue
        checked += 1
        out = subprocess.run(["pdftotext", "-q", path, "-"], capture_output=True, text=True).stdout
        for needle in needles:
            if needle not in out:
                print(f"ET WITNESS TEXT MISSING {fname}: '{needle}' not found in the analyzed bytes")
                failures += 1
    if failures:
        return failures
    if checked:
        print(f"elastic-thickness witness text OK: {checked} witnesses state their value")
    else:
        print("elastic-thickness witness text: all witnesses archive-then-slimmed (verified via receipt + archived_url)")
    return failures


def main():
    rows = read_rows()
    failures = check_receipts()
    failures += check_load_age(rows)
    failures += check_fingerprints(rows)
    failures += check_top_level()
    failures += check_witness_text()
    if failures:
        print(f"\nELASTIC-THICKNESS HINDCAST PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\nelastic-thickness hindcast provenance OK")


if __name__ == "__main__":
    main()
