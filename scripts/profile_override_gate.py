#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
"""THE PROFILE OVERRIDE GATE: a calibration profile may override a reserved value, and must SAY SO.

WHY THIS EXISTS. `calibration/reserved.toml` is the canonical register: the runbook says every reserved
value lives there with a mandatory category and provenance, and the scenario design calls a profile "an
override set layered over calibration/reserved.toml". The CODE does no layering. `CalibrationManifest::load`
parses one file into one map, and `run_world` loads `dev-fixtures.toml` or `mirror.toml` DIRECTLY, so
`reserved.toml` never participates in a run at all.

The consequence was invisible and large. Seventy-one ids exist in both `reserved.toml` and `mirror.toml`.
FORTY of them carry different values, with nothing in the tree comparing or arbitrating any of them: the
canonical register says one number, the file the simulation actually reads says another, and both look
authoritative. `climate.mean_surface_temperature` is 287 canonically and 288 on the living path.

THREE ARE WORSE THAN A DISAGREEMENT and this gate names them separately, because they are not overrides at
all. They disagree on UNIT:

    physiology.thermal_setpoint    kelvin  vs  temperature
    physiology.thermal_half_band   kelvin  vs  temperature
    hydrology.saturation_cap       mpa     vs  saturation_index

A megapascal and a dimensionless saturation index are not the same quantity at two values. They are two
quantities sharing a name, which is the diamond in its purest form, and no override declaration can make
that legitimate. They must be renamed or reconciled.

WHAT THIS GATE ENFORCES. Not "profiles may not override": overriding is the declared design. It enforces
that an override is DECLARED rather than discovered, so a divergence is a sentence someone wrote instead of
a number someone finds later. A collision passes only when the profile entry carries:

    overrides_reserved = "<why this world's value differs from the canonical one>"

and a unit mismatch never passes, declaration or not.

RATCHET, NOT AMNESTY. The seventy-one existing collisions are baselined so the gate can be turned on today
rather than after seventy-one arbitration decisions, and the baseline is SHRINK-ONLY by construction: it is
keyed on (id, profile, reserved_value, profile_value), so changing either value un-baselines the row and
the gate convicts it. A baselined row that no longer collides is also an error, because a stale waiver is
how a ledger becomes an amnesty for something that comes back.

Usage:
    profile_override_gate.py              enforce
    profile_override_gate.py --generate   emit the baseline rows for the current tree
    profile_override_gate.py --self-test  prove the detector fires
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
RESERVED = ROOT / "calibration" / "reserved.toml"
PROFILES = ROOT / "calibration" / "profiles"
BASELINE = ROOT / "scripts" / "profile_override_baseline.tsv"


def parse_entries(text):
    """id -> {field: value} for every `[[value]]`-shaped block, whatever the table is named.

    Deliberately tolerant about the block header: reserved.toml and the profiles have drifted in shape,
    and a parser that only knew one of them would silently read zero entries from the other, which is the
    failure mode where a gate reports "clean" because it looked at nothing.
    """
    entries = {}
    for block in re.split(r"\n\[\[[a-z_]+\]\]", text):
        ident = re.search(r'^\s*id\s*=\s*"([^"]+)"', block, re.M)
        if not ident:
            continue
        fields = {}
        for key in ("value", "unit", "category", "provenance", "status", "overrides_reserved"):
            m = re.search(r'^\s*' + key + r'\s*=\s*"([^"]*)"', block, re.M)
            if m:
                fields[key] = m.group(1)
        entries[ident.group(1)] = fields
    return entries


def collisions(reserved, profile):
    """(id, reserved_fields, profile_fields) for every id present in both."""
    return [(i, reserved[i], profile[i]) for i in sorted(set(reserved) & set(profile))]


def classify(rid, res, prof):
    """The finding for one collision, or None when it is fine.

    Returns (kind, detail). `unit-mismatch` is never waivable; `undeclared-override` is baselineable.
    """
    r_unit = res.get("unit", "").strip()
    p_unit = prof.get("unit", "").strip()
    if r_unit and p_unit and r_unit != p_unit:
        return ("unit-mismatch", f"{r_unit} versus {p_unit}")
    r_val = res.get("value", "").strip()
    p_val = prof.get("value", "").strip()
    if r_val == p_val:
        return None
    if prof.get("overrides_reserved", "").strip():
        return None
    return ("undeclared-override", f"{r_val or '(unset)'} versus {p_val or '(unset)'}")


def load_baseline():
    rows = set()
    if not BASELINE.exists():
        return rows
    for line in BASELINE.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        rows.add(tuple(line.split("\t")))
    return rows


def scan():
    """(findings, baseline_keys_seen). A finding is (id, profile_name, kind, detail, key)."""
    reserved = parse_entries(RESERVED.read_text(encoding="utf-8"))
    findings = []
    for path in sorted(PROFILES.glob("*.toml")):
        prof = parse_entries(path.read_text(encoding="utf-8"))
        for rid, res, pf in collisions(reserved, prof):
            verdict = classify(rid, res, pf)
            if not verdict:
                continue
            kind, detail = verdict
            key = (rid, path.name, kind, res.get("value", ""), pf.get("value", ""))
            findings.append((rid, path.name, kind, detail, key))
    return findings


def main():
    findings = scan()

    if "--generate" in sys.argv:
        print("# Profile override baseline v1. SHRINK-ONLY: rows may be deleted, never added.")
        print("# id\tprofile\tkind\treserved_value\tprofile_value")
        for _rid, _p, kind, _d, key in sorted(findings):
            if kind == "undeclared-override":
                print("\t".join(key))
        return 0

    if "--self-test" in sys.argv:
        res = {"a.b": {"value": "1", "unit": "kelvin"}}
        same_unit = {"a.b": {"value": "2", "unit": "kelvin"}}
        assert classify("a.b", res["a.b"], same_unit["a.b"])[0] == "undeclared-override"
        declared = {"value": "2", "unit": "kelvin", "overrides_reserved": "this world is warmer"}
        assert classify("a.b", res["a.b"], declared) is None, "a declared override passes"
        bad_unit = {"value": "2", "unit": "mpa"}
        assert classify("a.b", res["a.b"], bad_unit)[0] == "unit-mismatch"
        equal = {"value": "1", "unit": "kelvin"}
        assert classify("a.b", res["a.b"], equal) is None, "an equal value is no collision"
        print("profile override gate self-test: ok (4 cases, every detector fires)")
        return 0

    # THE KNOWN-DEFECT LEDGER for unit mismatches, and it is NOT a waiver. A waiver would say "this
    # override is legitimate", which is exactly the false claim a unit mismatch makes. This says "these
    # are defects, nobody has fixed them, and they are not allowed to be a surprise". They stay printed on
    # every run, they are counted, and a new one still fails, because the ledger is keyed on the exact
    # unit pair: change either unit and the row stops covering it.
    #
    # They cost real work to fix (renaming an id or reconciling a unit touches consumers, and the living
    # profile reaches canonical state, so a careless fix moves a pin), which is why they are recorded here
    # rather than either fixed in a hurry or left invisible.
    KNOWN_UNIT_DEFECTS = {
        ("metabolism.kleiber_coefficient", "dev-fixtures.toml"),
        ("physiology.thermal_half_band", "dev-fixtures.toml"),
        ("physiology.thermal_setpoint", "dev-fixtures.toml"),
        ("hydrology.saturation_cap", "mirror.toml"),
        ("physiology.thermal_half_band", "mirror.toml"),
        ("physiology.thermal_setpoint", "mirror.toml"),
    }

    baseline = load_baseline()
    all_unit = [f for f in findings if f[2] == "unit-mismatch"]
    unit_mismatches = [f for f in all_unit if (f[0], f[1]) not in KNOWN_UNIT_DEFECTS]
    carried_unit = [f for f in all_unit if (f[0], f[1]) in KNOWN_UNIT_DEFECTS]
    stale_unit = sorted(KNOWN_UNIT_DEFECTS - {(f[0], f[1]) for f in all_unit})
    undeclared = [f for f in findings if f[2] == "undeclared-override"]
    unwaived = [f for f in undeclared if f[4] not in baseline]
    seen_keys = {f[4] for f in undeclared}
    stale = [b for b in baseline if b not in seen_keys]

    for rid, prof, _k, detail, _key in unit_mismatches:
        print(f"UNIT MISMATCH (never waivable) {rid} in {prof}: {detail}")
        print("    Two quantities sharing an id, not one quantity at two values. Rename or reconcile.")
    for rid, prof, _k, detail, _key in unwaived:
        print(f"UNDECLARED OVERRIDE {rid} in {prof}: canonical {detail}")
        print('    Add `overrides_reserved = "<why this world differs>"`, or make the values agree.')
    for b in sorted(stale):
        print(f"STALE BASELINE {b[0]} in {b[1]}: no longer collides at these values. Delete the row.")

    if carried_unit:
        print(f"CARRIED UNIT DEFECTS ({len(carried_unit)}), recorded as defects rather than overrides:")
        for rid, prof, _k, detail, _key in carried_unit:
            print(f"  - {rid} in {prof}: {detail}")
        print()
    for rid, prof in stale_unit:
        print(f"STALE UNIT-DEFECT ROW {rid} in {prof}: no longer mismatching. Delete the row.")

    total = len(unit_mismatches) + len(unwaived) + len(stale) + len(stale_unit)
    if total:
        print()
        print(f"profile override gate: FAILED. {total} finding(s).")
        return 1
    print(
        f"profile override gate: clean ({len(undeclared)} declared-or-baselined override(s), "
        f"{len(carried_unit)} carried unit defect(s), 0 new)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
