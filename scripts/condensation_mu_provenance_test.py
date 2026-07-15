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


def check_ice():
    import math

    path = os.path.join(DATA, "ice_sublimation_murphy_koop.toml")
    text = open(path, encoding="utf-8").read()
    coeff = dict(re.findall(r'(eq_[abcd])\s*=\s*"(-?[0-9.]+)"', text))
    for k in ("eq_a", "eq_b", "eq_c", "eq_d"):
        assert k in coeff, f"ice header records no {k} coefficient (the recipe receipt)"
    a, b, c, d = (float(coeff[k]) for k in ("eq_a", "eq_b", "eq_c", "eq_d"))

    def p_eq(T):
        # Murphy-Koop 2005 ice equation (natural log), recomputed from the recorded coefficients.
        return math.exp(a + b / T + c * math.log(T) + d * T)

    rows = [r for r in parse_rows(text, "point") if "t_k" in r]
    assert len(rows) == 10, f"expected 10 ice points (130-220 K), got {len(rows)}"
    failures = 0
    for r in rows:
        T = float(r["t_k"])
        want = p_eq(T)
        got = float(r["p_sat_pa"])
        # The stored value is a 3-sig-fig rounding of the equation; allow 1% relative drift.
        if abs(got - want) / want > 0.01:
            print("ICE RECOMPUTE DRIFT", T, "want", want, "got", got)
            failures += 1
    # The 180 K snow-line fingerprint.
    p180 = next(float(r["p_sat_pa"]) for r in rows if float(r["t_k"]) == 180.0)
    if abs(p180 - 5.40e-3) > 1e-4:
        print("ICE FINGERPRINT DRIFT 180 K want 5.40e-3 got", p180)
        failures += 1
    assert failures == 0, f"{failures} ice sublimation mismatch(es)"
    print(f"PASS: {len(rows)} ice points recompute from the Murphy-Koop coefficients, 180 K fingerprint holds")


# Robie-Hemingway 1995 CaTiO3 perovskite fingerprints (kJ/mol), the owner's pre-registered spot rows.
PEROVSKITE_FINGERPRINTS = {
    # temperature (K): (delta_f_g_kj, delta_f_h_kj or None)
    "298.15": (-1574.8, -1660.6),
    "1200": (-1323.9, -1664.1),
    "1300": (-1295.6, -1662.5),
    "1400": (-1267.5, -1660.8),
    "1500": (-1239.4, -1658.9),
    "1600": (-1211.8, -1655.3),
    "1700": (-1184.1, -1654.4),
}


def check_perovskite():
    path = os.path.join(DATA, "perovskite_robie1995.toml")
    text = open(path, encoding="utf-8").read()
    # The cited source-PDF md5 receipt must be recorded in the header.
    assert re.search(r"md5 [0-9a-f]{32}", text), "perovskite header records no source-PDF md5 receipt"
    rows = {r["t_k"]: r for r in parse_rows(text, "point") if "t_k" in r}
    assert len(rows) == 7, f"expected 7 perovskite rows (298.15 + 1200-1700), got {len(rows)}"
    failures = 0
    for T, (wg, wh) in PEROVSKITE_FINGERPRINTS.items():
        row = rows.get(T)
        if not row:
            print("PEROVSKITE MISSING", T)
            failures += 1
            continue
        gg = float(row["delta_f_g_kj"])
        if abs(gg - wg) > 0.1:
            print("PEROVSKITE delta_f_G DRIFT", T, "want", wg, "got", gg)
            failures += 1
        if wh is not None and abs(float(row["delta_f_h_kj"]) - wh) > 0.1:
            print("PEROVSKITE delta_f_H DRIFT", T, "want", wh, "got", row["delta_f_h_kj"])
            failures += 1
    # The refractory fingerprint: delta_f H(298) is the -1660 kJ/mol class.
    if abs(float(rows["298.15"]["delta_f_h_kj"]) - -1660.6) > 1.0:
        print("PEROVSKITE FINGERPRINT delta_f_H(298) not -1660 class")
        failures += 1
    assert failures == 0, f"{failures} perovskite mismatch(es)"
    print(f"PASS: {len(rows)} perovskite rows, all {len(PEROVSKITE_FINGERPRINTS)} fingerprints reproduce")


def main():
    check_agss09()
    check_ice()
    check_perovskite()
    print("PASS: condensation mu-fetch provenance battery")


if __name__ == "__main__":
    try:
        main()
    except AssertionError as e:
        print("FAIL:", e)
        sys.exit(1)
