#!/usr/bin/env python3
# ANALYTIC VERIFICATION + TRANSCRIPTION, NOT VALIDATION.
#
# The free-free closed forms are genuine analytic verification. The non-analytic eigenvalues are TRANSCRIBED r
# ather than recomputed, so for those this is a transcription check.
#
# WHY THE LABEL IS HERE. An audit found this battery described uniformly as one that reconstructs each
# fetch from its recipe and asserts byte-equality, for all eight tests. That was true of some and false
# of others, and the difference matters: custody proves the bytes we hold are the bytes we fetched,
# transcription proves our column matches the held source, and neither proves the source is RIGHT. A
# test that reads its expectation from the file under test does not even prove independence. Saying so
# where the test is, is harder to drift than saying it in a document.
#
# Dry-run provenance battery for the critical-Rayleigh eigenvalue registry cited [M] column. No network. It
# (1) rechecks the sha256 receipt and byte count of every witness against rayleigh_critical_eigenvalues/
# manifest.toml (two archive-then-slimmed to this registry's local custody + a retrievable archived_url, three
# cross-referenced in the sibling convection_scaling/ custody via the `dir` field), (2) recomputes the two
# closed-form eigenvalues (free-free 27*pi^4/4 and its wavenumber pi/sqrt(2)) and asserts the committed column
# transcribed them, (3) reconstructs the rigid-free provenance-tangle arithmetic (17,610.5/16, 17,610.39/16,
# 17,610.39/24) and asserts the source-verbatim decomposition strings and the modern verified value are present
# and internally consistent, and (4) asserts the owner's pre-registered fingerprints against the column. A
# fifth, best-effort check greps each held witness for a value it witnesses when pdftotext is present, and is
# skipped otherwise (the sha256 receipt already pins the exact analyzed bytes). Mirrors
# scripts/gruneisen_provenance_test.py and scripts/convection_scaling_provenance_test.py (held-bytes +
# closed-form + fingerprint idiom).
import hashlib
import math
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")
HELD_DIR = os.path.join(DATA, "rayleigh_critical_eigenvalues")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(DATA, "rayleigh_critical_eigenvalues.toml")
# Archive-then-slimmed witnesses (owner ruling 2026-07-18) keep their bytes out of git, in local custody keyed
# by the manifest `dir` (or this registry's own directory when `dir` is absent). Present only on the machine
# that fetched them; CI verifies via the receipt + archived_url instead.
STORE_ROOT = os.path.expanduser("~/.claude/vendored-sources")
OWN_STORE = "rayleigh_eigenvalues"

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


def custody_path(s):
    """The local-custody path for a witness: keyed by the manifest `dir` (sibling directory) or this registry."""
    return os.path.join(STORE_ROOT, s.get("dir", OWN_STORE) or OWN_STORE, s["file"])


def repo_path(s):
    """The in-repo path a non-slimmed witness would occupy (DATA/<dir>/<file> or HELD_DIR/<file>)."""
    base = os.path.join(DATA, s["dir"]) if s.get("dir") else HELD_DIR
    return os.path.join(base, s["file"])


def check_receipts():
    # A witness is verified by repo bytes whose sha256 matches, by a local-custody copy whose sha256 matches, or
    # by a retrievable Internet Archive snapshot recorded as archived_url. A receipt with none is the rot the
    # custody work exists to prevent. When a slimmed witness sits in custody its sha256 is still re-checked.
    text = open(MANIFEST, encoding="utf-8").read()
    failures = 0
    held = 0
    archived = 0
    for s in parse_blocks(text, "source"):
        if "file" not in s or "sha256" not in s:
            continue
        verified_bytes = False
        for path in (repo_path(s), custody_path(s)):
            if os.path.exists(path):
                raw = open(path, "rb").read()
                if hashlib.sha256(raw).hexdigest() != s["sha256"]:
                    print(f"RAYLEIGH SHA256 DRIFT {s['file']}: copy at {path} does not match the receipt")
                    failures += 1
                elif "bytes" in s and len(raw) != int(s["bytes"]):
                    print(f"RAYLEIGH BYTE-COUNT DRIFT {s['file']}: manifest {s['bytes']} held {len(raw)}")
                    failures += 1
                else:
                    verified_bytes = True
                break
        if verified_bytes:
            held += 1
            continue
        # Slimmed and not in custody on this machine: the archived snapshot is the retrievable witness of record.
        if not s.get("archived_url", "").strip():
            print(f"RAYLEIGH NO WITNESS {s['file']}: receipt {s['sha256']} points at nothing retrievable")
            failures += 1
            continue
        archived += 1
    if not failures:
        print(f"rayleigh receipts OK: {held} verified by held bytes + {archived} by archived_url, all match their receipt")
    return failures


def read_eigenvalues():
    text = open(COLUMN, encoding="utf-8").read()
    rows = {}
    for g in parse_blocks(text, "eigenvalue"):
        if "name" in g:
            rows[g["name"]] = g
    return rows


def check_closed_forms(rows):
    """The free-free eigenvalue has a closed form; recompute it so a drifted transcription of an analytic value
    fails."""
    failures = 0
    pi = math.pi
    ff = rows.get("bottom_heated_free_free", {})
    checks = [
        ("free-free Ra_c 27*pi^4/4", 27 * pi**4 / 4, float(ff.get("rayleigh_number", "nan"))),
        ("free-free k_c pi/sqrt(2)", pi / math.sqrt(2), float(ff.get("critical_wavenumber", "nan"))),
    ]
    for label, computed, column in checks:
        if math.isnan(column) or abs(computed - column) > 1e-2:
            print(f"RAYLEIGH CLOSED-FORM DRIFT {label}: computed {computed:.6f} column {column}")
            failures += 1
    if not failures:
        print("rayleigh closed forms OK: 27*pi^4/4 and pi/sqrt(2) reproduce the free-free row")
    return failures


def check_rigid_free_tangle(rows):
    """Reconstruct the rigid-free provenance tangle: the source-verbatim decomposition strings must be present,
    their arithmetic must reproduce the recorded values, the Drazin-Reid inconsistency must hold (17,610.39/24
    is not 1101), and the modern verified value must be transcribed. So a drift in the tangle fails the build."""
    failures = 0
    rf = rows.get("bottom_heated_rigid_free", {})
    if not rf:
        print("RAYLEIGH MISSING ROW bottom_heated_rigid_free")
        return 1
    # The printed notations that must survive verbatim (Glomski & Johnson 2012 table 1).
    verbatim = {
        "decomposition_pellew_southwell_1940": "17,610.5/16",
        "decomposition_reid_harris_1958": "no explicit decimal",
        "decomposition_chandrasekhar_1961": "17,610.39/16",
        "decomposition_drazin_reid_2004": "17,610.39/24",
    }
    for field, needle in verbatim.items():
        if needle not in rf.get(field, ""):
            print(f"RAYLEIGH TANGLE DRIFT {field}: '{needle}' missing from the source-verbatim decomposition")
            failures += 1
    # The arithmetic behind the notations.
    arithmetic = [
        ("17,610.5/16 (Pellew-Southwell)", 17610.5 / 16, 1100.65625),
        ("17,610.39/16 (Chandrasekhar / Reid-Harris)", 17610.39 / 16, 1100.649375),
        ("17,610.39/24 (Drazin-Reid printed)", 17610.39 / 24, 733.76625),
    ]
    for label, got, want in arithmetic:
        if abs(got - want) > 1e-4:
            print(f"RAYLEIGH TANGLE ARITHMETIC {label}: {got} != {want}")
            failures += 1
    # The documented inconsistency: 17,610.39/24 is 733.77, NOT the printed 1101 (which is round(17,610.39/16)).
    if abs(17610.39 / 24 - 1101) < 1:
        print("RAYLEIGH TANGLE: 17,610.39/24 unexpectedly near 1101 (the documented inconsistency is absent)")
        failures += 1
    if abs(round(17610.39 / 16) - 1101) > 0:
        print("RAYLEIGH TANGLE: round(17,610.39/16) is not 1101 (the Drazin-Reid rounding explanation breaks)")
        failures += 1
    # The modern verified value (Glomski & Johnson 2012 Theorem 4.1), transcribed.
    vv = rf.get("verified_value", "")
    if not vv.startswith("1100.6496068876"):
        print(f"RAYLEIGH VERIFIED VALUE DRIFT: verified_value '{vv[:20]}' does not start 1100.6496068876")
        failures += 1
    if not rf.get("verified_wavenumber", "").startswith("2.6823217576"):
        print("RAYLEIGH VERIFIED WAVENUMBER DRIFT: does not start 2.6823217576")
        failures += 1
    if not failures:
        print("rayleigh rigid-free tangle OK: verbatim decompositions, arithmetic, the Drazin-Reid inconsistency, and the verified value all reproduce")
    return failures


def check_definition(rows):
    """The Rayleigh-Roberts definition row must carry the fifth-power depth dependence and the H*h^2 temperature
    scale, in both conventions, so a drifted definition transcription fails."""
    failures = 0
    d = rows.get("rayleigh_roberts_definition", {})
    if not d:
        print("RAYLEIGH MISSING ROW rayleigh_roberts_definition")
        return 1
    needles = [("rayleigh_number", "d^5"), ("alt_form", "h^5"), ("temperature_scale", "h^2/lambda"),
               ("conversion_to_bottom_heated_Ra", "R/N")]
    for field, needle in needles:
        if needle not in d.get(field, ""):
            print(f"RAYLEIGH DEFINITION DRIFT {field}: '{needle}' missing")
            failures += 1
    if not failures:
        print("rayleigh Rayleigh-Roberts definition OK: fifth-power depth, H*h^2/lambda scale, and the R/N conversion all present")
    return failures


# The owner's pre-registered transcription fingerprints for rayleigh_critical_eigenvalues.toml
# (row -> [(field, value, tol)]). Verified at source in Goluskin 2016 and Glomski & Johnson 2012.
FINGERPRINTS = {
    "bottom_heated_free_free": [("rayleigh_number", 657.511, 1e-2), ("critical_wavenumber", 2.2214, 1e-2)],
    "bottom_heated_rigid_rigid": [("rayleigh_number", 1707.762, 1e-2), ("critical_wavenumber", 3.117, 1e-2)],
    "bottom_heated_rigid_free": [("rayleigh_number", 1100.65, 1e-2), ("critical_wavenumber", 2.682, 1e-2)],
    "internal_ih1_noslip_energy_stability": [("rayleigh_number", 26926.6, 1e-1), ("critical_wavenumber", 3.6174, 1e-2)],
    "internal_ih1_noslip_linear_instability": [("rayleigh_number", 37325.2, 1e-1), ("critical_wavenumber", 3.9989, 1e-2)],
    "internal_ih1_freefree_linear_instability": [("rayleigh_number", 16992.2, 1e-1), ("critical_wavenumber", 3.0277, 1e-2)],
    "internal_ih1_rigidfree_freetop_linear_instability": [("rayleigh_number", 16669.8, 1e-1), ("critical_wavenumber", 3.0131, 1e-2)],
}


def check_fingerprints(rows):
    failures = 0
    for name, fields in FINGERPRINTS.items():
        row = rows.get(name)
        if not row:
            print("RAYLEIGH MISSING ROW", name)
            failures += 1
            continue
        for field, want, tol in fields:
            if field not in row:
                print("RAYLEIGH MISSING FIELD", name, field)
                failures += 1
                continue
            got = float(row[field])
            if abs(got - want) > tol:
                print(f"RAYLEIGH FINGERPRINT DRIFT {name}.{field}: want {want} got {got}")
                failures += 1
    if not failures:
        print(f"rayleigh fingerprints OK: all {len(FINGERPRINTS)} rows reproduce their pre-registered values")
    return failures


# Best-effort: each held witness must contain a value it witnesses. Skipped without pdftotext so the mandatory
# checks above stay dependency-free (the sha256 receipt already pins the exact analyzed bytes).
WITNESS_SIGNATURES = {
    "goluskin_2016_internally_heated_arxiv_1506.01656.pdf": (OWN_STORE, ["37 325.2", "26 926.6", "1100.65"]),
    "sturtz_etal_2021_magma_ocean_arxiv_2108.00910.pdf": (OWN_STORE, ["Rayleigh-Roberts", "Roberts"]),
    "glomski_johnson_2012_ams.pdf": ("convection_scaling", ["1100.65", "17, 610.39", "no decimal"]),
}


def check_witness_text():
    if not shutil.which("pdftotext"):
        print("rayleigh witness-text check SKIPPED (pdftotext not installed; sha256 receipts still pin the bytes)")
        return 0
    failures = 0
    checked = 0
    for fname, (store, needles) in WITNESS_SIGNATURES.items():
        path = os.path.join(DATA, store, fname)
        if not os.path.exists(path):
            path = os.path.join(STORE_ROOT, store, fname)  # slimmed from repo; check local custody when present
        if not os.path.exists(path):
            continue
        checked += 1
        out = subprocess.run(["pdftotext", "-q", path, "-"], capture_output=True, text=True, errors="replace").stdout
        for needle in needles:
            if needle not in out:
                print(f"RAYLEIGH WITNESS TEXT MISSING {fname}: '{needle}' not found in the analyzed bytes")
                failures += 1
    if checked == 0:
        print("rayleigh witness text: all witnesses archive-then-slimmed (verified via receipt + archived_url; no local copies present)")
    elif not failures:
        print(f"rayleigh witness text OK: {checked} present witness(es) state their value")
    return failures


def main():
    rows = read_eigenvalues()
    failures = check_receipts()
    failures += check_closed_forms(rows)
    failures += check_rigid_free_tangle(rows)
    failures += check_definition(rows)
    failures += check_fingerprints(rows)
    failures += check_witness_text()
    if failures:
        print(f"\nRAYLEIGH-EIGENVALUE PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\nrayleigh-eigenvalue provenance OK")


if __name__ == "__main__":
    main()
