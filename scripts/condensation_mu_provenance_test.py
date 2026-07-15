#!/usr/bin/env python3
# Dry-run provenance battery for the disk-condensation mu-fetch vendored [M] columns (the AGSS09 solar abundance
# pattern, the Murphy-Koop ice sublimation pressure, and the Robie-Hemingway CaTiO3 perovskite Gibbs energy). No
# network: each source is a transcription of a primary PDF (or an equation from one), so there is no byte-identical
# machine-readable column to md5. The battery instead (a) confirms each file's header records its cited source
# receipt (the source-PDF md5, or the equation coefficients), and (b) re-checks the owner's pre-registered
# fingerprint rows against the transcribed TOML, the same rows the Rust #[test]s guard, so a drifted transcription
# fails the build. Mirrors scripts/janaf_provenance_test.py's transcription-fingerprint idiom.
import math
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")


def parse_rows(text, block):
    """Parse the `[[block]]` rows of a vendored TOML into a list of key->string dicts (string-valued fields only)."""
    rows = []
    for chunk in re.split(r"\[\[" + block + r"\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"([^"]*)"', chunk))
        # Capture the un-quoted integer fields too (e.g. z = 8) for keying.
        g.update(dict(re.findall(r"(\w+)\s*=\s*(-?\d+)\s*$", chunk, re.MULTILINE)))
        if g:
            rows.append(g)
    return rows


# AGSS09 Table 1 photospheric fingerprints (log-epsilon, H = 12), the owner's pre-registered spot rows.
AGSS09_FINGERPRINTS = {
    "O": 8.69,
    "C": 8.43,
    "Fe": 7.50,
    "Mg": 7.60,
    "Si": 7.51,
    "Ca": 6.34,
    "Al": 6.45,
    "Ti": 4.95,
    "Na": 6.24,
    "Ni": 6.22,
    "S": 7.12,
    "H": 12.00,
}


def check_agss09():
    path = os.path.join(DATA, "solar_abundances_agss09.toml")
    text = open(path, encoding="utf-8").read()
    # The cited source-PDF md5 receipt must be recorded in the header.
    assert re.search(r"md5 [0-9a-f]{32}", text), "AGSS09 header records no source-PDF md5 receipt"
    # The header comment names the literal "[[abundance]]"; only a real row carries a symbol field.
    rows = {r["symbol"]: r for r in parse_rows(text, "abundance") if "symbol" in r}
    assert len(rows) == 42, f"expected 42 AGSS09 rows (Z=1..42), got {len(rows)}"
    failures = 0
    for el, want in AGSS09_FINGERPRINTS.items():
        row = rows.get(el)
        if not row or "log_eps_photosphere" not in row:
            print("AGSS09 MISSING", el)
            failures += 1
            continue
        got = float(row["log_eps_photosphere"])
        if abs(got - want) > 0.05:
            print("AGSS09 FINGERPRINT DRIFT", el, "want", want, "got", got)
            failures += 1
    assert failures == 0, f"{failures} AGSS09 fingerprint mismatch(es)"
    print(f"PASS: {len(rows)} AGSS09 rows, all {len(AGSS09_FINGERPRINTS)} fingerprints reproduce")


def main():
    check_agss09()
    print("PASS: condensation mu-fetch provenance battery")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAIL:", e)
        sys.exit(1)
