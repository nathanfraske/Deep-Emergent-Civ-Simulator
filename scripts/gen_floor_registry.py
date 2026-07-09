#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
#
# Generates docs/working/PHYSICS_FLOOR_REGISTRY.md: the canonical list of every ABSOLUTE
# physics-floor value, the ONLY legitimate authoring places (Principle 9, the value-authoring
# line). An agent auditing derive-vs-author checks a quantity against this list: if it is already
# a floor AXIS or a named universal constant here, reading it is derive-clean; if it is NOT here,
# it must DERIVE from the floor and the situation, never be authored.
#
# The floor TOMLs carry three block kinds, and the registry keeps them distinct because they are
# not the same authoring category:
#   [[axis]]      the quantity AXES: the authored floor axes (THE floor for derive-vs-author).
#   [[substance]] reference materials (iron, oak): authored real-material DATA, vectors over the
#                 axes; authored, but a datum populating the axes, not an axis.
#   [[law]]       law kernels: fixed Rust, not an authored value (a constant a law needs is a
#                 universal constant or a reserved calibration value, listed elsewhere).
#
# Generated from crates/physics/data/*.toml with each entry's file:line, so the list never drifts;
# the floor-registry stop-gate regenerates this file to a temp and blocks if the committed one is
# stale. Deterministic output (file order, no timestamps) so regenerate-and-diff is clean equality.

import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = (
    os.path.abspath(sys.argv[1])
    if len(sys.argv) > 1
    else os.path.join(ROOT, "docs/working/PHYSICS_FLOOR_REGISTRY.md")
)

# The universal law constants: the authored constant floor (not a per-world taste value). Curated
# here because they are few and fundamental; each names where it is set or read.
UNIVERSAL = [
    ("Stefan-Boltzmann sigma", "5.670374e-8 W/(m^2 K^4)", "radiant-emission law; `metabolism.stefan_boltzmann` in calibration/reserved.toml; laws::radiant_emission"),
    ("Coulomb constant k", "8.9875517873681764e9 N m^2/C^2", "electrostatic-force law; em floor; laws (electricity wave)"),
    ("Vacuum permeability mu_0", "1.25663706212e-6 H/m", "magnetostatic law; em floor"),
    ("Gas constant R", "8.314462618 J/(mol K)", "ideal-gas / thermochemistry; chem-optics floor"),
    ("Gravitational constant G", "6.67430e-11 m^3/(kg s^2)", "forward-declared; no consumer until orbital mechanics (little g is per-world data, not this floor)"),
    ("Reducible fundamentals (c, k_B, h, e, eps_0, N_A)", "CODATA", "the constants the four above reduce to; pinned in crates/units / crates/physics where a law needs them directly"),
]


def parse_floor(path):
    """Return (axes, substances, law_count). axes/substances are lists of (id, line, unit, measures);
    each id is classified by the most recent [[kind]] header above it."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    axes, substances, law_count = [], [], 0
    kind = None
    for i, ln in enumerate(lines):
        h = re.match(r"^\[\[([a-z_]+)\]\]", ln)
        if h:
            kind = h.group(1)
            if kind == "law":
                law_count += 1
            continue
        m = re.match(r'^id = "([^"]+)"', ln)
        if not m:
            continue
        aid = m.group(1)
        unit = ""
        measures = ""
        for j in range(i, min(i + 8, len(lines))):
            mu = re.match(r'^unit = "(.*)"\s*$', lines[j])
            if mu and not unit:
                unit = mu.group(1)
            mm = re.match(r'^measures = "(.*)"\s*$', lines[j])
            if mm and not measures:
                measures = mm.group(1)
        entry = (aid, i + 1, unit, measures)
        if kind == "axis":
            axes.append(entry)
        elif kind == "substance":
            substances.append(entry)
    return axes, substances, law_count


def render_entries(body, rel, entries):
    for aid, line, unit, measures in entries:
        u = f" [{unit}]" if unit else ""
        meas = (measures[:150] + "...") if len(measures) > 150 else measures
        tail = f": {meas}" if meas else ""
        body.append(f"- `{aid}`{u} ({rel}:{line}){tail}")


def main():
    floors = sorted(glob.glob(os.path.join(ROOT, "crates/physics/data/*.toml")))
    parsed = [(os.path.relpath(f, ROOT), *parse_floor(f)) for f in floors]
    n_axes = sum(len(a) for _, a, _, _ in parsed)
    n_sub = sum(len(s) for _, _, s, _ in parsed)
    n_law = sum(lc for _, _, _, lc in parsed)

    body = []
    body.append("# Physics floor registry (the authored places; everything else must derive)")
    body.append("")
    body.append(
        "This is the canonical list of every ABSOLUTE physics-floor value, the ONLY legitimate "
        "authoring places under Principle 9 and the value-authoring line: the material and quantity "
        "AXES, and the universal law constants. It is the enforced reference for the derive-vs-author "
        "audit. The rule it makes concrete: a value in the path of world content that is on this list "
        "may be READ (authoring it there is legitimate); a value that is NOT on this list must DERIVE "
        "from the floor and the situation, never be authored. Per-world and per-race DATA (a world's "
        "orbital period, a race's Kleiber normalization, a control set point) is a separate authored "
        "category, the \"datum the engine models no deeper\"; it lives in calibration/reserved.toml with "
        "its own basis and is not this floor."
    )
    body.append("")
    body.append(
        f"There are {n_axes} material and quantity floor AXES (the floor proper) across {len(floors)} "
        f"floor files, plus the universal law constants below. The same files also carry {n_sub} "
        "reference SUBSTANCES (real materials as authored data vectors over the axes, listed separately "
        f"since a substance is a datum, not an axis) and {n_law} LAW kernels (fixed Rust, not authored "
        "values; a constant a law needs is a universal constant or a reserved calibration value)."
    )
    body.append("")
    body.append(
        "The lists below are GENERATED from `crates/physics/data/*.toml` by "
        "`scripts/gen_floor_registry.py`, with each entry's `file:line`, so they never drift. Do NOT "
        "edit this file by hand: change the floor data, then run the generator (the floor-registry "
        "stop-gate regenerates and blocks if this file is stale). To ADD a floor axis is to add an "
        "authored place, a deliberate act; the diff to this registry is the record of it."
    )
    body.append("")
    body.append("## Universal law constants (the authored constant floor)")
    body.append("")
    for name, value, where in UNIVERSAL:
        body.append(f"- **{name}** = {value} ({where})")
    body.append("")
    body.append("## Material and quantity floor axes (the floor proper)")
    body.append("")
    for rel, axes, _, _ in parsed:
        if not axes:
            continue
        body.append(f"### `{rel}` ({len(axes)} axes)")
        body.append("")
        render_entries(body, rel, axes)
        body.append("")
    body.append("## Reference substances (authored real-material data, not axes)")
    body.append("")
    for rel, _, subs, _ in parsed:
        if not subs:
            continue
        body.append(f"### `{rel}` ({len(subs)} substances)")
        body.append("")
        render_entries(body, rel, subs)
        body.append("")
    with open(OUT, "w", encoding="utf-8") as fh:
        fh.write("\n".join(body).rstrip() + "\n")
    print(f"wrote {os.path.relpath(OUT, ROOT)}: {n_axes} axes, {n_sub} substances, {n_law} laws")


if __name__ == "__main__":
    main()
