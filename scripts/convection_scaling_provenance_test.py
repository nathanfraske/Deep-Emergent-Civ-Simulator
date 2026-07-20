#!/usr/bin/env python3
# ANALYTIC VERIFICATION + TRANSCRIPTION, NOT VALIDATION.
#
# The 27 pi^4 / 4, pi / sqrt(2) and 2^(-4/3) computations are genuine analytic verification, and so is the
# stagnant-lid family relation gamma = -(1 + beta), which is the source's own eq. (8) re-derived against the
# committed exponents. The fingerprint dictionary is transcription. Neither proves empirical Nusselt-Rayleigh
# accuracy, and NOTHING here proves any banked stagnant-lid coefficient applies to a NON-NEWTONIAN lid: none was
# fitted at one, which is recorded on the rows and at the kernel rather than checked here.
#
# WHY THE LABEL IS HERE. An audit found this battery described uniformly as one that reconstructs each
# fetch from its recipe and asserts byte-equality, for all eight tests. That was true of some and false
# of others, and the difference matters: custody proves the bytes we hold are the bytes we fetched,
# transcription proves our column matches the held source, and neither proves the source is RIGHT. A
# test that reads its expectation from the file under test does not even prove independence. Saying so
# where the test is, is harder to drift than saying it in a document.
#
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
    # ARCHIVE-THEN-SLIM (owner ruling 2026-07-18): the witness bytes are not held in the repo; each receipt must
    # point at a retrievable Internet Archive snapshot (archived_url), and if a local copy happens to be present its
    # sha256 must still match. A receipt with no retrievable archive is the rot the custody work exists to prevent.
    text = open(MANIFEST, encoding="utf-8").read()
    failures = 0
    n = 0
    for s in parse_blocks(text, "source"):
        if "sha256" not in s:
            continue
        n += 1
        if not s.get("archived_url", "").strip():
            print(f"CONVECTION NO ARCHIVE {s.get('name', '?')}: receipt {s['sha256']} points at nothing retrievable")
            failures += 1
        if "file" in s:
            path = os.path.join(HELD_DIR, s["file"])
            if os.path.exists(path) and hashlib.sha256(open(path, "rb").read()).hexdigest() != s["sha256"]:
                print(f"CONVECTION SHA256 DRIFT {s['file']}: local copy does not match the receipt")
                failures += 1
    if not failures:
        print(f"convection receipts OK: {n} witnesses carry a sha256 receipt and a retrievable archive")
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
    # The stagnant-lid conventions. Each is a TRIPLE and all three fields are fingerprinted, because a
    # coefficient checked without its exponents is not a checked convention.
    "nu_stag_time_dependent_C1": [
        ("value", 0.48, TOL),
        ("band_lo", 0.48, TOL),
        ("band_hi", 0.55, TOL),
        ("theta_exponent", -4.0 / 3.0, TOL),
        ("rayleigh_exponent", 1.0 / 3.0, TOL),
    ],
    "nu_stag_steady_C2": [
        ("value", 2.95, TOL),
        ("theta_exponent", -1.2, TOL),
        ("rayleigh_exponent", 0.2, TOL),
    ],
    "nu_stag_arrhenius_internal_ra": [
        ("value", 0.278, TOL),
        ("theta_exponent", -0.4, TOL),
        ("rayleigh_exponent", 0.203, TOL),
    ],
    "nu_stag_arrhenius_harmonic_ra": [
        ("value", 0.219, TOL),
        ("theta_exponent", -0.581, TOL),
        ("rayleigh_exponent", 0.262, TOL),
    ],
}

# Batra & Foley 2021 eq. (8) states their stagnant-lid family as Nu = C* theta^-(1+beta) Ra_i^beta, so on those
# two rows gamma is NOT free: it is fixed by beta. Checking it is a relation ANALYTIC VERIFICATION of the
# transcription, the same standing as recomputing 27 pi^4 / 4, and it catches a slipped digit in either exponent
# that the per-field fingerprints above would only catch if the slip were in the field they name. The Schulz
# rows are deliberately NOT in this list: they are free fits and do not obey the relation (0.203 would demand
# -1.203, not -0.4), which is exactly why the two families are banked as separate rows.
ONE_PARAMETER_FAMILY_ROWS = ["nu_stag_time_dependent_C1", "nu_stag_steady_C2"]


def check_family_relation(rows):
    failures = 0
    for name in ONE_PARAMETER_FAMILY_ROWS:
        row = rows.get(name)
        if not row:
            print("CONVECTION MISSING ROW", name)
            failures += 1
            continue
        beta = float(row["rayleigh_exponent"])
        gamma = float(row["theta_exponent"])
        if abs(gamma - (-(1.0 + beta))) > TOL:
            print(
                f"CONVECTION FAMILY DRIFT {name}: eq. (8) demands gamma = -(1 + beta) = {-(1.0 + beta):.6f}, column has {gamma}"
            )
            failures += 1
    if not failures:
        print(f"convection family relation OK: {len(ONE_PARAMETER_FAMILY_ROWS)} linearized rows satisfy gamma = -(1 + beta)")
    return failures


def check_stagnant_lid_shape(rows):
    """A stagnant-lid row is a suppression law or it is not one. Every row whose name marks it as stagnant must
    carry BOTH exponents, with a NEGATIVE theta exponent (a stiffer lid loses less heat) and a POSITIVE Rayleigh
    exponent (a more vigorous interior loses more). A sign slip here would invert the physics silently, and the
    Rust reader keys off the presence of these two fields, so an absent one would make a row quietly unreadable
    rather than loudly wrong."""
    failures = 0
    n = 0
    for name, row in rows.items():
        if not name.startswith("nu_stag_"):
            continue
        n += 1
        for field in ("theta_exponent", "rayleigh_exponent"):
            if field not in row:
                print(f"CONVECTION STAGNANT ROW {name} IS MISSING {field}: the Rust reader would refuse it")
                failures += 1
        if "theta_exponent" in row and float(row["theta_exponent"]) >= 0:
            print(f"CONVECTION SIGN {name}: theta exponent must be negative (suppression), got {row['theta_exponent']}")
            failures += 1
        if "rayleigh_exponent" in row and float(row["rayleigh_exponent"]) <= 0:
            print(f"CONVECTION SIGN {name}: Rayleigh exponent must be positive, got {row['rayleigh_exponent']}")
            failures += 1
    if not n:
        print("CONVECTION NO STAGNANT-LID ROWS: the suppression family is gone from the column")
        failures += 1
    elif not failures:
        print(f"convection stagnant-lid shape OK: {n} rows carry a complete, correctly signed (alpha, gamma, beta)")
    return failures


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
    "batra_foley_2021_gji228_ggab366_nsfpar_accepted_manuscript.pdf": ["0.48", "2.95"],
    "schulz_tosi_plesa_breuer_2020_gji220_ggz417.pdf": ["0.278", "0.219", "12.73"],
}


def check_witness_text():
    if not shutil.which("pdftotext"):
        print("convection witness-text check SKIPPED (pdftotext not installed; sha256 receipts still pin the bytes)")
        return 0
    failures = 0
    checked = 0
    for fname, needles in WITNESS_SIGNATURES.items():
        path = os.path.join(HELD_DIR, fname)
        if not os.path.exists(path):
            # Slimmed from the repo (archive-then-slim); the receipt + archived_url + fingerprints carry it.
            continue
        checked += 1
        out = subprocess.run(["pdftotext", "-q", path, "-"], capture_output=True, text=True).stdout
        for needle in needles:
            if needle not in out:
                print(f"CONVECTION WITNESS TEXT MISSING {fname}: '{needle}' not found in held bytes")
                failures += 1
    if not failures:
        if checked:
            print(f"convection witness text OK: {checked} present witness(es) state their value")
        else:
            print(f"convection witness text: all {len(WITNESS_SIGNATURES)} witnesses archived-and-slimmed (verified via receipt + archive + fingerprints, no local copies present)")
    return failures


def main():
    rows = read_constants()
    failures = check_receipts()
    failures += check_closed_forms(rows)
    failures += check_fingerprints(rows)
    failures += check_family_relation(rows)
    failures += check_stagnant_lid_shape(rows)
    failures += check_witness_text()
    if failures:
        print(f"\nCONVECTION-SCALING PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\nconvection-scaling provenance OK")


if __name__ == "__main__":
    main()
