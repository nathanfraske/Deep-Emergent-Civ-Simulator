#!/usr/bin/env python3
# Dry-run provenance battery for the convection-scaling cited [M] column. No network. It (1) re-checks the sha256
# receipt and byte count of every held witness PDF against convection_scaling/manifest.toml, (2) recomputes the two
# closed-form stability eigenvalues (free-free 27*pi^4/4 and its wavenumber pi/sqrt(2), plus the symmetric-layer
# prefactor 2^(-4/3)) and asserts the committed column transcribed them correctly, and (3) asserts the owner's
# pre-registered fingerprints against convection_scaling.toml. A fourth, best-effort check greps each held PDF for
# the value it witnesses when pdftotext is present (the audit-the-input cross-check), and is skipped without it so
# the mandatory spine stays pure-Python. Mirrors scripts/janaf_provenance_test.py (held-bytes + fingerprint idiom).
import hashlib
import math
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")
HELD_DIR = os.path.join(DATA, "convection_scaling")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(DATA, "convection_scaling.toml")

TOL = 1e-3


def parse_blocks(text, kind):
    """Split a TOML into [[kind]] blocks, returning per-block {field: string} for quoted and bare-int fields."""
    blocks = []
    for block in re.split(rf"\[\[{kind}\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"((?:[^"\\]|\\.)*)"', block))
        for k, v in re.findall(r"^(\w+)\s*=\s*([0-9]+)\s*$", block, re.MULTILINE):
            g.setdefault(k, v)
        blocks.append(g)
    return blocks


def check_receipts():
    text = open(MANIFEST, encoding="utf-8").read()
    failures = 0
    n = 0
    for s in parse_blocks(text, "source"):
        if "file" not in s or "sha256" not in s:
            continue
        n += 1
        path = os.path.join(HELD_DIR, s["file"])
        if not os.path.exists(path):
            print("CONVECTION MISSING", s["file"])
            failures += 1
            continue
        raw = open(path, "rb").read()
        got = hashlib.sha256(raw).hexdigest()
        if got != s["sha256"]:
            print(f"CONVECTION SHA256 DRIFT {s['file']}: manifest {s['sha256']} held {got}")
            failures += 1
        if "bytes" in s and len(raw) != int(s["bytes"]):
            print(f"CONVECTION BYTE-COUNT DRIFT {s['file']}: manifest {s['bytes']} held {len(raw)}")
            failures += 1
    if not failures:
        print(f"convection receipts OK: {n} held witnesses match their sha256 and byte count")
    return failures


def read_constants():
    text = open(COLUMN, encoding="utf-8").read()
    rows = {}
    for g in parse_blocks(text, "constant"):
        if "name" in g:
            rows[g["name"]] = g
    return rows


def check_closed_forms(rows):
    """The two eigenvalues that have a closed form, plus the symmetric prefactor, recomputed and matched to the
    committed column so a drifted transcription of an analytic value fails."""
    failures = 0
    pi = math.pi
    ff = rows.get("ra_crit_free_free", {})
    checks = [
        ("free-free Ra_crit 27*pi^4/4", 27 * pi**4 / 4, float(ff.get("value", "nan"))),
        ("free-free wavenumber pi/sqrt(2)", pi / math.sqrt(2), float(ff.get("critical_wavenumber", "nan"))),
        (
            "symmetric prefactor 2^(-4/3)",
            2 ** (-4 / 3),
            float(rows.get("nu_ra_prefactor_a", {}).get("band_lo", "nan")),
        ),
    ]
    for label, computed, column in checks:
        if math.isnan(column) or abs(computed - column) > TOL:
            print(f"CONVECTION CLOSED-FORM DRIFT {label}: computed {computed:.6f} column {column}")
            failures += 1
    if not failures:
        print("convection closed forms OK: 27*pi^4/4, pi/sqrt(2), 2^(-4/3) all reproduce the column")
    return failures


# The owner's pre-registered transcription fingerprints for convection_scaling.toml (field, value, tolerance).
FINGERPRINTS = {
    "nu_ra_prefactor_a": [("value", 1.0, TOL), ("band_lo", 0.397, TOL), ("band_hi", 1.0, TOL)],
    "nu_ra_bare_coefficient_C": [("value", 0.294, TOL), ("band_lo", 0.1, TOL), ("band_hi", 0.294, TOL)],
    "ra_crit_free_free": [("value", 657.511, 1e-2), ("critical_wavenumber", 2.2214, 1e-2)],
    "ra_crit_rigid_rigid": [("value", 1707.762, 1e-2), ("critical_wavenumber", 3.117, 1e-2)],
    "ra_crit_rigid_free": [("value", 1100.65, 1e-2), ("critical_wavenumber", 2.682, 1e-2)],
}


def check_fingerprints(rows):
    failures = 0
    for name, fields in FINGERPRINTS.items():
        row = rows.get(name)
        if not row:
            print("CONVECTION MISSING ROW", name)
            failures += 1
            continue
        for field, want, tol in fields:
            if field not in row:
                print("CONVECTION MISSING FIELD", name, field)
                failures += 1
                continue
            got = float(row[field])
            if abs(got - want) > tol:
                print(f"CONVECTION FINGERPRINT DRIFT {name}.{field}: want {want} got {got}")
                failures += 1
    if not failures:
        print(f"convection fingerprints OK: all {len(FINGERPRINTS)} rows reproduce their pre-registered values")
    return failures


# Best-effort: each held witness must contain the value it witnesses. Skipped without pdftotext so the
# mandatory checks above stay dependency-free (the sha256 receipt already pins the exact analyzed bytes).
WITNESS_SIGNATURES = {
    "bodenschatz_pesch_ahlers_2000_arfm.pdf": ["1708", "3.117"],
    "ricard_geodynamics_convection_notes.pdf": ["657"],
    "komacek_abbot_2016_arxiv_1609.04786.pdf": ["Racrit"],
    "foley_bercovici_2014_arxiv_1410.7652.pdf": ["Solomatov"],
    "glomski_johnson_2012_ams.pdf": ["1100.65"],
}


def check_witness_text():
    if not shutil.which("pdftotext"):
        print("convection witness-text check SKIPPED (pdftotext not installed; sha256 receipts still pin the bytes)")
        return 0
    failures = 0
    for fname, needles in WITNESS_SIGNATURES.items():
        path = os.path.join(HELD_DIR, fname)
        if not os.path.exists(path):
            continue
        out = subprocess.run(["pdftotext", "-q", path, "-"], capture_output=True, text=True).stdout
        for needle in needles:
            if needle not in out:
                print(f"CONVECTION WITNESS TEXT MISSING {fname}: '{needle}' not found in held bytes")
                failures += 1
    if not failures:
        print(f"convection witness text OK: all {len(WITNESS_SIGNATURES)} held PDFs state their value")
    return failures


def main():
    rows = read_constants()
    failures = check_receipts()
    failures += check_closed_forms(rows)
    failures += check_fingerprints(rows)
    failures += check_witness_text()
    if failures:
        print(f"\nCONVECTION-SCALING PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\nconvection-scaling provenance OK")


if __name__ == "__main__":
    main()
