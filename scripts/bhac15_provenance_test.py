#!/usr/bin/env python3
# Dry-run provenance battery for the BHAC15 Hayashi-wall vendored [M] column. No network: it re-checks the recorded
# sha256 receipt against the vendored source bytes, and re-derives the extracted wall grid from those same bytes and
# asserts it equals the committed cited-data column, so a drifted source OR a drifted transcription fails the build.
# Mirrors scripts/janaf_provenance_test.py's held-bytes idiom (sha256 primary since 2026-07-17).
#
# BHAC15: sha256 the held bhac15/BHAC15_tracks+structure and assert it matches bhac15/manifest.toml; then re-parse
# the wall grid (wall T_eff at the earliest tabulated age, drift band over the first 2 Myr) and assert every row
# matches hayashi_wall.toml (the audit-the-input cross-check, permanent and offline).
import hashlib
import math
import os
import re
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")
HELD = os.path.join(DATA, "bhac15", "BHAC15_tracks+structure")
MANIFEST = os.path.join(DATA, "bhac15", "manifest.toml")
COLUMN = os.path.join(DATA, "hayashi_wall.toml")


def parse_scalar(text, key):
    m = re.search(rf'^\s*{key}\s*=\s*"?([^"\n]+)"?', text, re.MULTILINE)
    return m.group(1).strip() if m else None


def check_receipt():
    manifest = open(MANIFEST, encoding="utf-8").read()
    want = parse_scalar(manifest, "sha256")
    raw = open(HELD, "rb").read()
    got = hashlib.sha256(raw).hexdigest()
    if got != want:
        print(f"BHAC15 SHA256 DRIFT: manifest {want} held {got}")
        return 1
    want_bytes = int(parse_scalar(manifest, "bytes"))
    if len(raw) != want_bytes:
        print(f"BHAC15 BYTE-COUNT DRIFT: manifest {want_bytes} held {len(raw)}")
        return 1
    print(f"BHAC15 receipt OK: sha256 {got}, {len(raw)} bytes")
    return 0


def derive_grid():
    bymass = defaultdict(list)
    for line in open(HELD, encoding="utf-8"):
        p = line.split()
        if len(p) < 3:
            continue
        try:
            m, logt, teff = float(p[0]), float(p[1]), float(p[2])
        except ValueError:
            continue
        if 0.0 < m <= 1.5 and 4.0 < logt < 11.0 and 1000.0 < teff < 8000.0:
            bymass[round(m, 3)].append((logt, teff))
    two_myr = math.log10(2e6)
    grid = {}
    for m in sorted(bymass):
        trk = sorted(bymass[m])
        wall = trk[0][1]
        win = [t for (lt, t) in trk if lt <= two_myr + 1e-9]
        grid[m] = (wall, min(win), max(win))
    return grid


def read_column():
    text = open(COLUMN, encoding="utf-8").read()
    rows = {}
    for block in re.split(r"\[\[wall\]\]", text)[1:]:
        # The header comment names the literal "[[wall]]"; only a real block carries a mass field.
        mass_match = re.search(r"mass_msun\s*=\s*([\d.]+)", block)
        if mass_match is None:
            continue
        m = float(mass_match.group(1))
        w = float(re.search(r"wall_teff_k\s*=\s*([\d.]+)", block).group(1))
        lo = float(re.search(r"drift_lo_k\s*=\s*([\d.]+)", block).group(1))
        hi = float(re.search(r"drift_hi_k\s*=\s*([\d.]+)", block).group(1))
        rows[round(m, 3)] = (w, lo, hi)
    return rows


def check_transcription():
    derived = derive_grid()
    column = read_column()
    if set(derived) != set(column):
        print(f"BHAC15 GRID MASS MISMATCH: source {sorted(derived)} column {sorted(column)}")
        return 1
    failures = 0
    for m in sorted(derived):
        dw, dlo, dhi = derived[m]
        cw, clo, chi = column[m]
        if abs(dw - cw) > 0.5 or abs(dlo - clo) > 0.5 or abs(dhi - chi) > 0.5:
            print(f"BHAC15 GRID DRIFT at {m}: source {(dw, dlo, dhi)} column {(cw, clo, chi)}")
            failures += 1
    if not failures:
        print(f"BHAC15 wall grid OK: {len(derived)} rows re-derived from held bytes, all match the column")
    return failures


def main():
    failures = check_receipt() + check_transcription()
    if failures:
        print(f"\nBHAC15 PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\nBHAC15 provenance OK")


if __name__ == "__main__":
    main()
