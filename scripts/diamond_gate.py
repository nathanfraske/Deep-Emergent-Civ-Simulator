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

Usage:  python3 scripts/diamond_gate.py [--strict] [--self-test]
        --self-test proves the detector still FIRES on its known-bad fixture (the project's standing gate
          convention, matching determinism_gate/constructor_gate/provenance_gate/stone0: "prove every
          detector fires"). The canary also runs on every normal invocation, so the gate cannot report a
          sweep it is too blind to trust.
        --strict exits non-zero on any unarbitrated diamond. NOT the current CI posture: the sweep still
          reports output caches and cross-domain name collisions, so strict would cry wolf.
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
REGISTERED_RELATIONSHIPS = {
    # quantity -> (kind, where the relationship is realized, what makes it legal)
    #
    # THE REGISTRY IS THE SIGNAL, and it is the answer to "output caches need a signal that is not citation".
    # A citation cannot distinguish a cache from a conflict (both cite the law; that is what blinded the first
    # refinement). A REGISTRATION can, because it is a claim a human makes and defends, not a shape a script
    # infers. Any registered relationship is exempt; an unregistered one is convicted. No doc convention, no
    # citation parsing, nothing for the next refactor to silently break.
    "thermal_conductivity": (
        "ladder-order",
        "crates/materials/src/conductivity.rs",
        "measured rung before estimator rung, with rung_disagreement_ratio as the overlap sentinel",
    ),
    "convective_stress": (
        "cache-of",
        "crates/physics/src/laws.rs::convective_stress",
        "the interior lane writes the law's own output onto the column state; one provider, cached, not two",
    ),
}

# THE PERMANENT CANARY: known-bad cases the gate MUST ALWAYS FLAG, checked on EVERY run.
#
# PROVE-ONCE BECOMES PROVE-ALWAYS. The first refinement to this gate silenced its one true positive and did it
# quietly (the sweep just got shorter). That was caught because someone happened to look. Happening-to-look is
# not a control, which is the same reasoning that built this gate in the first place, so the same fix applies to
# the gate itself: a refinement that re-blinds it now FAILS LOUDLY on the next run.
#
# The canaries are SYNTHETIC on purpose. Using the live `thermal_diffusivity` diamond would mean the canary dies
# the day k/kappa is fixed, and prove-always would quietly decay into prove-until-fixed. A synthetic fixture
# outlives every repair.
CANARIES = [
    # (name, law_site, [(field_site, type)], must_flag, why)
    (
        "canary_stored_and_derived",
        "crates/physics/src/laws.rs:1",
        [("crates/sim/src/canary.rs:1", "Fixed")],
        True,
        "the k/kappa shape: one law derives it, one field stores it, nothing arbitrates",
    ),
    (
        "canary_law_only",
        "crates/physics/src/laws.rs:2",
        [],
        False,
        "a law with no storing field is not a diamond",
    ),
]


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


def detect(laws, fields, registry):
    """THE DETECTOR, pure so the canary can run it against a fixture. A quantity with a deriving law AND a
    storing field, and no registered relationship, is an unarbitrated diamond."""
    findings = []
    for quantity, law_site in sorted(laws.items()):
        sites = fields.get(quantity)
        if not sites:
            continue
        if quantity in registry:
            continue  # a registered relationship of any kind: legal by construction, not a defect
        findings.append((quantity, law_site, sites))
    return findings


def canary_check():
    """Run the detector against the SYNTHETIC canary fixture. Returns a list of failures.

    A failure here means THE GATE IS BLIND, which is strictly worse than the gate being noisy, so it is reported
    before any sweep result and it fails the run.
    """
    failures = []
    for name, law_site, field_sites, must_flag, why in CANARIES:
        laws = {name: law_site}
        fields = {name: field_sites} if field_sites else {}
        flagged = bool(detect(laws, fields, {}))
        if flagged != must_flag:
            verb = "FAILED TO FLAG" if must_flag else "WRONGLY FLAGGED"
            failures.append(f"{verb} `{name}`: {why}")
    return failures


def main():
    strict = "--strict" in sys.argv
    self_test = "--self-test" in sys.argv

    # THE CANARY RUNS FIRST, ALWAYS. If the gate is blind, nothing it says about the codebase means anything.
    blind = canary_check()
    if blind:
        print("THE DIAMOND GATE IS BLIND. Its canary fixture did not behave:")
        for f in blind:
            print(f"  {f}")
        print()
        print("A refinement has broken the detector. A gate's noise is visible and annoying; a gate's blindness")
        print("is invisible and comfortable, which is why blindness is the failure mode that survives review.")
        print("Fix the detector before trusting any sweep below.")
        return 2

    if self_test:
        print(f"diamond gate self-test: {len(CANARIES)} known cases, every detector fires.")
        return 0

    laws = law_providers()
    fields = stored_providers()
    findings = detect(laws, fields, REGISTERED_RELATIONSHIPS)

    print("THE DIAMOND GATE: quantities with a DERIVING law and a STORING field, unarbitrated.")
    print(f"canary fixture: {len(CANARIES)} known cases, all behaved (the gate can still see)")
    print(f"scanned {len(laws)} laws.rs kernels against every pub struct field under crates/")
    for q, (kind, where, why) in sorted(REGISTERED_RELATIONSHIPS.items()):
        print(f"registered [{kind}] {q}: {why}")
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
