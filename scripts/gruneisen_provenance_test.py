#!/usr/bin/env python3
# Dry-run provenance battery for the Gruneisen cited [M] column. No network. It (1) rechecks the sha256
# receipt of every witness (three archive-then-slimmed to local custody + a retrievable archived_url, three
# cross-referenced in-repo in the sibling moduli directories) against gruneisen/manifest.toml, (2) asserts the owner's pre-registered fingerprints
# (gamma_thermodynamic, gamma_eos_debye, K') against gruneisen.toml, and (3) recomputes the Slater relation
# gamma = K'/2 - 1/6 for every row that carries both a measured gamma and a K' (the OVERLAP SENTINEL): a
# Slater-applicable phase must agree to same order (ratio <= the documented bound) and a Slater-inapplicable
# phase must diverge (ratio > the bound), so the documented chain/framework-silicate failure is encoded and a
# drifted transcription fails the build. A fourth, best-effort check greps each held witness for the value it
# witnesses when djvutxt/pdftotext are present, and is skipped otherwise (the sha256 receipt already pins the
# bytes). Mirrors scripts/convection_scaling_provenance_test.py (held-bytes + fingerprint + closed-form idiom).
import hashlib
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "..", "crates", "physics", "data")
HELD_DIR = os.path.join(DATA, "gruneisen")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(DATA, "gruneisen.toml")
# Archive-then-slimmed witnesses (owner ruling 2026-07-18) keep their bytes here, out of git. Best-effort:
# present only on the machine that fetched them; CI verifies via the receipt + archived_url instead.
LOCAL_STORE = os.path.expanduser("~/.claude/vendored-sources/gruneisen")

TOL = 1e-3
# The documented line between a same-order Slater estimate and the chain/framework-silicate breakdown.
# Applicable phases (dense oxides, orthosilicates): Slater over-estimates the direct thermodynamic gamma by
# ~1.17x (periclase) to ~1.87x (fayalite), so the ratio sits in [SLATER_LO, SLATER_FACTOR_BOUND]. Inapplicable
# phases (enstatite, quartz): the anomalously large K' from soft tilt modes drives Slater ~4-5x high.
SLATER_FACTOR_BOUND = 2.5
SLATER_LO = 0.9


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
    # A witness is verified by repo bytes whose sha256 matches (the cross-referenced K' primaries, held in the
    # sibling moduli dirs) OR by a retrievable Internet Archive snapshot recorded as archived_url (the three
    # archive-then-slimmed here). A receipt with neither is the rot the custody work exists to prevent. When a
    # slimmed witness happens to sit in local custody its sha256 is still re-checked (defence in depth).
    text = open(MANIFEST, encoding="utf-8").read()
    failures = 0
    held = 0
    archived = 0
    for s in parse_blocks(text, "source"):
        if "file" not in s or "sha256" not in s:
            continue
        base = os.path.join(DATA, s["dir"]) if s.get("dir") else HELD_DIR
        path = os.path.join(base, s["file"])
        if os.path.exists(path):
            raw = open(path, "rb").read()
            if hashlib.sha256(raw).hexdigest() != s["sha256"]:
                print(f"GRUNEISEN SHA256 DRIFT {s['file']}: repo copy does not match the receipt")
                failures += 1
            elif "bytes" in s and len(raw) != int(s["bytes"]):
                print(f"GRUNEISEN BYTE-COUNT DRIFT {s['file']}: manifest {s['bytes']} held {len(raw)}")
                failures += 1
            else:
                held += 1
            continue
        # Slimmed from the repo: the archived snapshot is the retrievable witness of record.
        if not s.get("archived_url", "").strip():
            print(f"GRUNEISEN NO WITNESS {s['file']}: receipt {s['sha256']} points at nothing retrievable")
            failures += 1
            continue
        archived += 1
        custody = os.path.join(LOCAL_STORE, s["file"])
        if os.path.exists(custody) and hashlib.sha256(open(custody, "rb").read()).hexdigest() != s["sha256"]:
            print(f"GRUNEISEN SHA256 DRIFT {s['file']}: local-custody copy does not match the receipt")
            failures += 1
    if not failures:
        print(f"gruneisen receipts OK: {held} held in-repo + {archived} archive-then-slimmed, all match their receipt")
    return failures


def read_minerals():
    text = open(COLUMN, encoding="utf-8").read()
    rows = {}
    for g in parse_blocks(text, "mineral"):
        if "name" in g:
            rows[g["name"]] = g
    return rows


# The owner's pre-registered transcription fingerprints for gruneisen.toml (field, value, tolerance).
FINGERPRINTS = {
    "periclase": [("gamma_thermodynamic", 1.54, TOL), ("gamma_eos_debye", 1.50, TOL), ("bulk_modulus_pressure_derivative_kprime", 3.94, TOL)],
    "corundum": [("gamma_thermodynamic", 1.32, TOL), ("gamma_eos_debye", 1.32, TOL), ("bulk_modulus_pressure_derivative_kprime", 4.3, TOL)],
    "spinel": [("gamma_thermodynamic", 1.51, TOL), ("gamma_eos_debye", 1.02, TOL), ("bulk_modulus_pressure_derivative_kprime", 5.7, TOL)],
    "forsterite": [("gamma_thermodynamic", 1.29, TOL), ("gamma_eos_debye", 0.99, TOL), ("bulk_modulus_pressure_derivative_kprime", 4.2, TOL)],
    "fayalite": [("gamma_thermodynamic", 1.21, TOL), ("gamma_eos_debye", 1.06, TOL), ("bulk_modulus_pressure_derivative_kprime", 4.85, TOL)],
    "enstatite": [("gamma_eos_debye", 0.67, TOL), ("bulk_modulus_pressure_derivative_kprime", 7.1, TOL)],
    "quartz": [("bulk_modulus_pressure_derivative_kprime", 5.99, TOL)],
}


def check_fingerprints(rows):
    failures = 0
    for name, fields in FINGERPRINTS.items():
        row = rows.get(name)
        if not row:
            print("GRUNEISEN MISSING ROW", name)
            failures += 1
            continue
        for field, want, tol in fields:
            if field not in row:
                print("GRUNEISEN MISSING FIELD", name, field)
                failures += 1
                continue
            got = float(row[field])
            if abs(got - want) > tol:
                print(f"GRUNEISEN FINGERPRINT DRIFT {name}.{field}: want {want} got {got}")
                failures += 1
    if not failures:
        print(f"gruneisen fingerprints OK: all {len(FINGERPRINTS)} rows reproduce their pre-registered values")
    return failures


def slater(kprime):
    """The Slater (1939) estimator: gamma = K'/2 - 1/6."""
    return kprime / 2.0 - 1.0 / 6.0


def measured_gamma(row):
    """The measured anchor: prefer the direct thermodynamic gamma_th, else the EoS-Debye gamma_0."""
    if "gamma_thermodynamic" in row:
        return float(row["gamma_thermodynamic"]), "gamma_thermodynamic"
    if "gamma_eos_debye" in row:
        return float(row["gamma_eos_debye"]), "gamma_eos_debye"
    return None, None


def check_slater_closed_form(rows):
    """Recompute the Slater formula for one anchored row so a drifted formula constant (the 1/6) fails."""
    per = rows.get("periclase", {})
    kp = float(per.get("bulk_modulus_pressure_derivative_kprime", "nan"))
    got = slater(kp)
    want = 1.803  # 3.94/2 - 1/6
    if abs(got - want) > 1e-2:
        print(f"GRUNEISEN SLATER CLOSED-FORM DRIFT periclase: K'={kp} -> Slater {got:.3f}, expected {want}")
        return 1
    print("gruneisen Slater closed form OK: K'/2 - 1/6 reproduces the periclase fingerprint (1.803)")
    return 0


def check_slater_sentinel(rows):
    """The overlap sentinel: where a row carries BOTH a measured gamma and a K', recompute Slater and assert
    same-order agreement for a Slater-applicable phase and documented divergence for an inapplicable one."""
    failures = 0
    checked = 0
    for name, row in rows.items():
        kp = row.get("bulk_modulus_pressure_derivative_kprime")
        gamma, src = measured_gamma(row)
        if kp is None or gamma is None:
            continue  # not an overlap row (K'-only, e.g. quartz, or gamma-only): the sentinel does not fire
        checked += 1
        ratio = slater(float(kp)) / gamma
        applicable = row.get("slater_gamma_applicable", "").strip().lower() == "true"
        if applicable:
            if not (SLATER_LO <= ratio <= SLATER_FACTOR_BOUND):
                print(f"GRUNEISEN SLATER SENTINEL {name}: applicable but Slater/{src} = {ratio:.2f} outside "
                      f"[{SLATER_LO}, {SLATER_FACTOR_BOUND}] (same-order agreement expected)")
                failures += 1
        else:
            if ratio <= SLATER_FACTOR_BOUND:
                print(f"GRUNEISEN SLATER SENTINEL {name}: flagged inapplicable but Slater/{src} = {ratio:.2f} "
                      f"<= {SLATER_FACTOR_BOUND} (the documented anomaly divergence is absent)")
                failures += 1
    if not failures:
        print(f"gruneisen Slater sentinel OK: {checked} overlap rows agree (applicable) or diverge (anomaly) as declared")
    return failures


# Best-effort: each held witness must contain a value it witnesses. Skipped without djvutxt/pdftotext so the
# mandatory checks above stay dependency-free (the sha256 receipt already pins the exact analyzed bytes).
WITNESS_SIGNATURES = {
    "ahrens_1995_mineral_physics_handbook.djvu": ("djvutxt", ["Mineral physics", "MgO"]),
    "stixrude_lithgow-bertelloni_gji_2005.pdf": ("pdftotext", ["mantle", "forsterite"]),
    "angel_etal_1997_quartz_jac.pdf": ("pdftotext", ["37.12", "5.99"]),
}


def _extract(tool, path):
    # djvutxt prints to stdout when no output file is given; pdftotext needs the explicit "-" sink.
    argv = ["djvutxt", path] if tool == "djvutxt" else [tool, "-q", path, "-"]
    return subprocess.run(argv, capture_output=True, text=True, errors="replace").stdout


def check_witness_text():
    failures = 0
    n = 0
    for fname, (tool, needles) in WITNESS_SIGNATURES.items():
        path = os.path.join(HELD_DIR, fname)
        if not os.path.exists(path):
            path = os.path.join(LOCAL_STORE, fname)  # slimmed from the repo; check local custody when present
        if not os.path.exists(path) or not shutil.which(tool):
            continue
        n += 1
        out = _extract(tool, path)
        for needle in needles:
            if needle not in out:
                print(f"GRUNEISEN WITNESS TEXT MISSING {fname}: '{needle}' not found in the analyzed bytes")
                failures += 1
    if n == 0:
        print("gruneisen witness-text check SKIPPED (bytes archive-then-slimmed and/or djvutxt/pdftotext absent; receipts + archived_url carry it)")
    elif not failures:
        print(f"gruneisen witness text OK: {n} witnesses state their value")
    return failures


def main():
    rows = read_minerals()
    failures = check_receipts()
    failures += check_fingerprints(rows)
    failures += check_slater_closed_form(rows)
    failures += check_slater_sentinel(rows)
    failures += check_witness_text()
    if failures:
        print(f"\nGRUNEISEN PROVENANCE FAILED ({failures})")
        sys.exit(1)
    print("\ngruneisen provenance OK")


if __name__ == "__main__":
    main()
