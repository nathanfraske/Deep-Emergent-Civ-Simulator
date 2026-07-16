#!/usr/bin/env python3
"""THE DIAMOND GATE: find quantities with two providers and no registered arbitration.

WHY THIS EXISTS. Three times in one week, the same defect shape turned up and was caught only because a
person happened to run one grep before building on it:

  1. k/kappa: `ColumnParams.thermal_diffusivity` STORES a number while `laws::thermal_diffusivity` DERIVES
     the same quantity from three fields the same struct carries. They disagree by 20x. Nothing compared them.
  2. Slack/Hofmeister: two physical models of thermal conductivity, which would have disagreed ~5x on the
     target class had a census not run first.
  3. The A-source: the viewer re-declares `kernel_conductivity = 2` and `kernel_density = 1` locally, the SAME
     fixture cluster `ColumnParams` carries, at a site that can drift from it independently.

A person running one line is not a control. This is the script.

THE PREDICATE IS *NOT* "TWO PROVIDERS". A diamond is not a defect; an UNARBITRATED diamond is. Two providers
with a REGISTERED LADDER ORDER and an overlap sentinel are the conductivity ladder, legal by construction and
the best thing the geotherm arc built. Two providers with neither are k/kappa. So the gate convicts
PROVIDERS-WITHOUT-REGISTERED-ARBITRATION, or it would convict the very design that fixed the defect it exists
to find.

WHAT IT MAY AND MAY NOT DO. It may FALSIFY, never AUTHOR. It reports candidates for a human to rule on, and it
is deliberately CONSERVATIVE by design: it demands no completeness, and a quantity it cannot see is not thereby blessed.
Auto-authored edges are prohibited outright (a call graph masquerading as a derivation graph is this project's
signature defect in another coat), so this script never writes a marker, never edits a registry, and never
decides. Human authors the claim; the machine referees it.

RENDER-ON-DEMAND: every finding ships with its LOCAL SUBGRAPH drawn, because the moment a diamond fires the
first thing anyone wants is the neighbourhood, not a browser.

Usage:  python3 scripts/diamond_gate.py [--strict]
        --strict exits non-zero on any unarbitrated diamond (the CI gate posture).
"""

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LAWS = ROOT / "crates" / "physics" / "src" / "laws.rs"

# A quantity whose providers are registered as a LADDER: an ordered dispatch (measured before estimator) with
# an overlap sentinel that computes and reports the rungs' disagreement wherever both evaluate. These are LEGAL
# by construction. The registry is HAND-AUTHORED here on purpose: registering a ladder is a claim a human makes
# and defends, never something a script infers from shape.
REGISTERED_LADDERS = {
    # quantity -> (module holding the dispatch, the overlap sentinel that reports rung disagreement)
    "thermal_conductivity": (
        "crates/materials/src/conductivity.rs",
        "rung_disagreement_ratio",
    ),
}


def law_providers():
    """Every `pub fn` in laws.rs: a DERIVING provider of the quantity its name states."""
    out = {}
    for i, line in enumerate(LAWS.read_text().splitlines(), 1):
        m = re.match(r"pub fn (\w+)\s*\(", line)
        if m:
            out[m.group(1)] = f"crates/physics/src/laws.rs:{i}"
    return out


def stored_providers():
    """Every `pub <name>: <type>` struct field under crates/, a STORING provider of that quantity.

    Fields declared inside laws.rs itself are EXCLUDED: a law's own output struct naming its output is not a
    second provider, it is the same one. That exclusion is the difference between a gate and a noise machine.
    """
    out = defaultdict(list)
    try:
        res = subprocess.run(
            ["grep", "-rn", "--include=*.rs", r"^\s*pub \w\+: \w", str(ROOT / "crates")],
            capture_output=True,
            text=True,
        )
    except Exception:
        return out
    for line in res.stdout.splitlines():
        m = re.match(r"(.*?):(\d+):\s*pub (\w+): (\w+)", line)
        if not m:
            continue
        path, lineno, name, ty = m.groups()
        rel = str(Path(path).relative_to(ROOT))
        if rel.endswith("physics/src/laws.rs"):
            continue  # a law's own output struct: the same provider, not a second one
        out[name].append((f"{rel}:{lineno}", ty))
    return out


# THE EXEMPTION THAT WAS TRIED AND REVERTED, recorded because the next person will propose it again.
#
# The first sweep produced two false-positive classes, and one looked mechanically fixable: `convective_stress`
# fired, but its doc reads "([`crate::laws::convective_stress`]) ... Written by the INTERIOR lane", so it STORES
# the law's output and says so. A cache, not a rival. The obvious refinement: exempt any field whose doc cites
# the law computing it.
#
# THAT REFINEMENT SILENCED THE ONE TRUE POSITIVE. `ColumnParams.thermal_diffusivity` also cites
# `laws::thermal_diffusivity`, because its doc is the tag explaining that the field CONFLICTS with that law by
# 20x. A cache cites its law. A documented conflict cites its law too. The citation does not distinguish them,
# so the exemption made the gate blind to the exact defect it exists to find, and it did so QUIETLY: the sweep
# just got shorter and cleaner-looking.
#
# THE RULE THIS EARNS: a refinement that makes a gate quieter must be PROVEN not to make it blind, against the
# gate's own known true positives, before it ships. A gate's noise is visible and annoying; a gate's blindness
# is invisible and comfortable, which is why blindness is the failure mode that survives review.


def render_subgraph(quantity, law_site, field_sites):
    """The finding's LOCAL SUBGRAPH, drawn. This is the render-on-demand path: the neighbourhood at the moment
    the gate fires, which is when anyone wants it."""
    lines = [f"    ({quantity})", "     |"]
    lines.append(f"     +-- [DERIVED]  laws::{quantity}()            {law_site}")
    for site, ty in field_sites:
        lines.append(f"     +-- [STORED]   pub {quantity}: {ty:<8}  {site}")
    lines.append("     |")
    lines.append("     +-- ARBITRATION: none registered  <-- the defect")
    return "\n".join(lines)


def main():
    strict = "--strict" in sys.argv
    laws = law_providers()
    fields = stored_providers()

    findings = []
    for quantity, law_site in sorted(laws.items()):
        sites = fields.get(quantity)
        if not sites:
            continue
        if quantity in REGISTERED_LADDERS:
            continue  # a registered ladder: legal by construction, not a defect
        findings.append((quantity, law_site, sites))

    print("THE DIAMOND GATE: quantities with a DERIVING law and a STORING field, unarbitrated.")
    print(f"scanned {len(laws)} laws.rs kernels against every pub struct field under crates/")
    print(f"registered ladders (exempt, legal by construction): {sorted(REGISTERED_LADDERS)}")
    print()

    if not findings:
        print("no unarbitrated diamonds found.")
        return 0

    for quantity, law_site, sites in findings:
        print(f"UNARBITRATED DIAMOND: `{quantity}`")
        print(render_subgraph(quantity, law_site, sites))
        print()

    print(f"{len(findings)} unarbitrated diamond(s).")
    print()
    print("Each is a CANDIDATE for a human to rule on, never a verdict: this gate falsifies, it does not")
    print("author. A finding is discharged by one of three moves, and by nothing else:")
    print("  1. RETIRE the stored field, deriving the quantity (the k/kappa ruling: replacement, not")
    print("     arbitration, because a bare value with no declared scale carries no correctness, only a value).")
    print("  2. REGISTER A LADDER: an ordered dispatch (measured before estimator) plus an overlap sentinel")
    print("     that computes and reports the rungs' disagreement wherever both evaluate.")
    print("  3. SHOW IT IS NOT ONE: the name collides but the quantities differ. Then rename, because a name")
    print("     that reads as a diamond will be read as one by the next person too.")
    return 1 if strict else 0


if __name__ == "__main__":
    sys.exit(main())
