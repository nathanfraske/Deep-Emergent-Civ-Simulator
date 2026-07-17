#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.
#
# Generates docs/working/PHYSICS_FLOOR_REGISTRY.md: the canonical, actual-truth map of the physics
# substrate. An agent auditing derive-vs-author checks a quantity against this list: if it is already
# a floor AXIS or a named universal constant here, reading it is derive-clean; if it is NOT here,
# it must DERIVE from the floor and the situation, never be authored. It is also the "where do I look
# for this law" index, so it lists every floor LAW, both the ones declared in the floor data and the
# direct kernels implemented in laws.rs, each with its file:line.
#
# The floor TOMLs carry three block kinds, and the registry keeps them distinct because they are
# not the same authoring category:
#   [[axis]]      the quantity AXES: the authored floor axes (THE floor for derive-vs-author).
#   [[substance]] reference materials (iron, oak): authored real-material DATA, vectors over the
#                 axes; authored, but a datum populating the axes, not an axis.
#   [[law]]       declared law kernels: fixed Rust, not an authored value (a constant a law needs is
#                 a universal constant or a reserved calibration value, listed elsewhere).
# A fourth source is scanned so nothing hides from the map:
#   crates/physics/src/laws.rs   the DIRECT law kernels (every `pub fn`), some backed by a declared
#                 [[law]] block, some called straight by the sim (the spreading law, the transduction
#                 family). A direct kernel adds no authored value (it reads existing axes/constants),
#                 but it IS a floor law mechanism, so it belongs on the map for findability.
#
# Generated with each entry's file:line, so the list never drifts; the floor-registry stop-gate
# regenerates this file to a temp and blocks if the committed one is stale. Deterministic output
# (file order, no timestamps) so regenerate-and-diff is clean equality.

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
LAWS_RS_REL = "crates/physics/src/laws.rs"

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
    """Return (axes, substances, laws). axes/substances are (id, line, unit, measures); laws are
    (id, line, kernel). Each id is classified by the most recent [[kind]] header above it."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    axes, substances, laws = [], [], []
    kind = None
    for i, ln in enumerate(lines):
        h = re.match(r"^\[\[([a-z_]+)\]\]", ln)
        if h:
            kind = h.group(1)
            continue
        m = re.match(r'^id = "([^"]+)"', ln)
        if not m:
            continue
        aid = m.group(1)
        unit = ""
        measures = ""
        kernel = ""
        for j in range(i, min(i + 8, len(lines))):
            mu = re.match(r'^unit = "(.*)"\s*$', lines[j])
            if mu and not unit:
                unit = mu.group(1)
            mm = re.match(r'^measures = "(.*)"\s*$', lines[j])
            if mm and not measures:
                measures = mm.group(1)
            mk = re.match(r'^kernel = "(.*)"\s*$', lines[j])
            if mk and not kernel:
                kernel = mk.group(1)
        if kind == "axis":
            axes.append((aid, i + 1, unit, measures))
        elif kind == "substance":
            substances.append((aid, i + 1, unit, measures))
        elif kind == "law":
            laws.append((aid, i + 1, kernel))
    return axes, substances, laws


def parse_laws_rs(path):
    """Return [(name, line, summary)] for every `pub fn` in laws.rs, in file order (deterministic).
    summary is the first DESCRIPTIVE line of the doc comment immediately above the fn, if any.

    DESCRIPTIVE, not merely FIRST, and the difference is a defect this file already suffered. Two consumers
    independently claimed the position "first doc line": this reader took it as the excerpt, and the diamond
    gate's `@provides` annotation was written there. The collision made the enforced derive-vs-author reference
    stop describing two kernels and recite their annotations back ("@provides thermal_boundary_layer" as the
    summary of `thermal_boundary_layer`). That is the POSITIONAL-CONVENTION COLLISION class: two conventions
    agreeing on a slot and disagreeing about its meaning, which the stop gate caught and no other check could.

    The repair is SEMANTIC rather than positional (owner ruling): annotation lines are SKIPPED rather than
    reserved a position, so "the excerpt is the first descriptive line" holds no matter where an annotation is
    placed, and the NEXT annotation convention cannot re-break it. Placing `@provides` last is the current
    convention and is now a convenience rather than a load-bearing requirement.
    """
    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    kernels = []
    summary = None
    for i, ln in enumerate(lines):
        s = ln.strip()
        mdoc = re.match(r"^///\s?(.*)", s)
        if mdoc:
            text = mdoc.group(1).strip()
            # An annotation is machinery for another reader, never a description of the kernel. Skipping it
            # keeps this excerpt semantic; reserving it a position would only move the collision.
            if summary is None and text and not text.startswith("@"):
                summary = text
            continue
        mfn = re.match(r"^pub fn ([a-z0-9_]+)", ln)
        if mfn:
            kernels.append((mfn.group(1), i + 1, summary or ""))
            summary = None
            continue
        if s == "" or s.startswith("#["):
            # blank lines and attributes sit between a doc block and its fn; keep the pending summary.
            continue
        summary = None  # any other line ends the pending doc block
    return kernels


def scan_derives(root):
    """Scan every .rs under crates/ for `// @derives:` markers, the deriving-substrate index.

    A marker sits at a derivation entry point (a subsystem that produces a world quantity from the
    floor and the situation) and reads `// @derives[<id>]: <quantity> <- <from these substrate
    inputs>`, or the bare `// @derives:` form. The optional `[<id>]` token is the machine handle the
    derived-output-is-live gate keys its `Derivation` registry to (tasks #43, #46); it is
    ignored here, so both forms land on the billboard.
    The generator emits every marker with its file:line, so the deriving half of the substrate map
    (orbital mechanics, hydrology, metabolism, the matter cycle, the time-space anchors: everything
    OUTSIDE crates/physics) is on the billboard and the stop-gate keeps it never-stale, exactly as
    it does the authored floor. Returns [(rel, line, text)] sorted by (file, line) for deterministic
    output. This is the derive-FROM list: an agent about to author a value checks here first, and a
    quantity found here is a defect to author because a substrate already derives it."""
    out = []
    for path in sorted(glob.glob(os.path.join(root, "crates/**/*.rs"), recursive=True)):
        rel = os.path.relpath(path, root)
        with open(path, encoding="utf-8") as fh:
            for i, ln in enumerate(fh):
                m = re.search(r"//\s*@derives(?:\[\w+\])?:\s*(.*\S)", ln)
                if m:
                    out.append((rel, i + 1, m.group(1).strip()))
    out.sort(key=lambda t: (t[0], t[1]))
    return out


def render_entries(body, rel, entries):
    for aid, line, unit, measures in entries:
        u = f" [{unit}]" if unit else ""
        meas = (measures[:150] + "...") if len(measures) > 150 else measures
        tail = f": {meas}" if meas else ""
        body.append(f"- `{aid}`{u} ({rel}:{line}){tail}")


def main():
    # floor_provenance.toml is the seven-tag grade accounting sidecar (provenance register Phase 2), keyed
    # by entry id, not a physics floor manifest of axes/laws/substances, so it is excluded from the floor
    # registry generation (the derive-vs-author reference is the axes/laws/substances, not their grades).
    floors = sorted(
        f
        for f in glob.glob(os.path.join(ROOT, "crates/physics/data/*.toml"))
        if os.path.basename(f) != "floor_provenance.toml"
    )
    parsed = [(os.path.relpath(f, ROOT), *parse_floor(f)) for f in floors]
    n_axes = sum(len(a) for _, a, _, _ in parsed)
    n_sub = sum(len(s) for _, _, s, _ in parsed)
    n_law = sum(len(lw) for _, _, _, lw in parsed)
    laws_rs = parse_laws_rs(os.path.join(ROOT, LAWS_RS_REL))
    declared_kernels = {k for _, _, _, lw in parsed for _, _, k in lw if k}
    n_kernels = len(laws_rs)
    n_direct = sum(1 for name, _, _ in laws_rs if name not in declared_kernels)
    derives = scan_derives(ROOT)
    n_derives = len(derives)

    body = []
    body.append("# Physics substrate registry (what you may author, and what you must derive FROM)")
    body.append("")
    body.append(
        "This is the canonical, actual-truth map of the physics substrate, in two halves. The FLOOR half "
        "is every ABSOLUTE physics-floor value (the ONLY legitimate authoring places under Principle 9 and "
        "the value-authoring line: the material and quantity AXES and the universal law constants), plus "
        "every floor LAW with its file:line so this doubles as the \"where do I look for this law\" index. "
        "The DERIVING-SUBSTRATE half (the section right below the constants) is every subsystem OUTSIDE the "
        "physics crate that derives a world quantity from the floor and the situation: orbital mechanics, "
        "the hydrology cycle, productivity, metabolism, the matter cycle, the time-space anchors. The "
        "derive-vs-author rule it makes concrete: a value on the axis/constant lists may be READ "
        "(authoring it there is legitimate); a value that is NOT must DERIVE, and the deriving-substrate "
        "list is where you look for the substrate that derives it BEFORE you ever author a number. If a "
        "quantity you were about to author appears in the deriving-substrate list, authoring it is a "
        "defect. Per-world and per-race DATA (a world's orbital period once it is read from the orbit, a "
        "race's Kleiber normalization, a control set point) is a separate authored category, the \"datum "
        "the engine models no deeper\"; it lives in calibration/reserved.toml with its own basis and is not "
        "this floor."
    )
    body.append("")
    body.append(
        f"There are {n_axes} material and quantity floor AXES (the floor proper) across {len(floors)} "
        f"floor files, plus the universal law constants below. The same files carry {n_sub} reference "
        "SUBSTANCES (real materials as authored data vectors over the axes, a datum not an axis). The "
        f"floor laws are {n_law} DECLARED laws (the `[[law]]` blocks in the floor data) plus {n_kernels} "
        f"direct kernels in `{LAWS_RS_REL}` ({n_direct} of them not backed by a declared block, called "
        "straight by the sim), all listed below. A law kernel is fixed Rust, not an authored value; a "
        "constant a law needs is a universal constant or a reserved calibration value."
    )
    body.append("")
    body.append(
        "The lists below are GENERATED from `crates/physics/data/*.toml`, `" + LAWS_RS_REL + "`, and every "
        "`// @derives:` marker across `crates/**/*.rs` by `scripts/gen_floor_registry.py`, with each "
        "entry's `file:line`, so they never drift. Do NOT edit this file by hand: change the floor data, "
        "add the law kernel, or place the `// @derives:` marker at the deriving subsystem, then run the "
        "generator (the floor-registry stop-gate regenerates and blocks if this file is stale). To ADD a "
        "floor axis, a law kernel, or a deriving substrate is a deliberate act; the diff to this registry "
        "is the record of it. When you build a new subsystem that derives a world quantity, mark it with "
        "`// @derives: <quantity> <- <from these inputs>` so it lands on this billboard and no future "
        "agent authors from the ether what your subsystem already derives."
    )
    body.append("")
    body.append("## Universal law constants (the authored constant floor)")
    body.append("")
    for name, value, where in UNIVERSAL:
        body.append(f"- **{name}** = {value} ({where})")
    body.append("")
    body.append("## Deriving substrates (check here BEFORE authoring: what the world derives, and where)")
    body.append("")
    body.append(
        f"The {n_derives} deriving subsystems below live OUTSIDE the authored floor. Each produces a "
        "world quantity from the floor and the situation, so its output must never be authored: if the "
        "value you need appears here, read or extend the subsystem, do not set a number. This is the "
        "list that stops `1 year = 365 days` from being authored when orbital mechanics already derives "
        "it. Generated from the `// @derives:` markers in the code; a subsystem missing its marker is a "
        "gap in this map, so mark every derivation entry point."
    )
    body.append("")
    if derives:
        cur = None
        for rel, line, text in derives:
            if rel != cur:
                body.append(f"### `{rel}`")
                body.append("")
                cur = rel
            body.append(f"- {text} (`{rel}:{line}`)")
        body.append("")
    else:
        body.append("_No `// @derives:` markers found yet. Place them at each deriving subsystem._")
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
    body.append("## Law kernels (fixed Rust mechanisms, not authored values)")
    body.append("")
    body.append(
        "Two kinds, kept distinct: a DECLARED law is a `[[law]]` block in the floor data (it names the "
        "kernel it runs); a DIRECT kernel is a `pub fn` in `" + LAWS_RS_REL + "`. Every direct kernel is "
        "listed, tagged `[direct]` when no declared block backs it (the sim calls it straight, for "
        "example the general spreading law and the transduction family). This section is the law index: "
        "to find where a law lives, look here."
    )
    body.append("")
    body.append("### Declared laws (`[[law]]` blocks in the floor data)")
    body.append("")
    for rel, _, _, laws in parsed:
        if not laws:
            continue
        body.append(f"#### `{rel}` ({len(laws)} laws)")
        body.append("")
        for lid, line, kernel in laws:
            k = f" -> `{kernel}`" if kernel else ""
            body.append(f"- `{lid}`{k} ({rel}:{line})")
        body.append("")
    body.append(f"### Direct kernels (`{LAWS_RS_REL}`, {n_kernels} `pub fn`, {n_direct} unbacked)")
    body.append("")
    for name, line, summary in laws_rs:
        tag = "" if name in declared_kernels else " [direct]"
        s = (summary[:150] + "...") if len(summary) > 150 else summary
        tail = f": {s}" if s else ""
        body.append(f"- `{name}`{tag} ({LAWS_RS_REL}:{line}){tail}")
    body.append("")
    with open(OUT, "w", encoding="utf-8") as fh:
        fh.write("\n".join(body).rstrip() + "\n")
    print(
        f"wrote {os.path.relpath(OUT, ROOT)}: {n_axes} axes, {n_sub} substances, "
        f"{n_law} declared laws, {n_kernels} laws.rs kernels ({n_direct} direct), "
        f"{n_derives} deriving substrates"
    )


if __name__ == "__main__":
    main()
