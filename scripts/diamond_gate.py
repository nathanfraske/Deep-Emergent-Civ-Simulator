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

THIS GATE'S BLINDNESS SET, STATED BESIDE ITS DISCRIMINATING POWER, because a gate that knows what it cannot see
is a gate and a gate that does not is a comfort. Measured against the THREE instances that motivated building
it, THIS GATE CATCHES ONE:

  CATCHES: a quantity with a laws.rs kernel AND a pub struct field OF THE SAME NAME (k/kappa, found).

  BLIND 1, DIFFERENT-NAMED PROVIDERS: Slack (`lattice_thermal_conductivity_w_per_m_k`) versus Hofmeister
    (`hofmeister_lattice_conductivity`) are two models of one quantity under two names, and NEITHER is a laws.rs
    kernel. This gate cannot see that collision at all. COVERED BY: the census-before-build habit, which is what
    caught it in the event, and by the ladder registry below once a quantity is registered. NOT covered mechanically.

  BLIND 2, LOCAL-VARIABLE DUPLICATES: the viewer re-declares `kernel_conductivity = 2` and `kernel_density = 1`
    as LOCALS, the same fixture cluster ColumnParams carries. Not a pub field, so this gate cannot see it.
    COVERED BY: nothing mechanical today. It was found by a one-line check a human ran, which is precisely the
    control this gate exists to replace and does not yet replace here.

  BLIND 3, SHARED-SOURCE ERRORS: this gate certifies that two providers are ARBITRATED. It says NOTHING about
    whether either provider is RIGHT. Two rungs that agree because they share a wrong input pass silently, the
    same structural blindness a conservation gate has to an error in its opening (which cancels on both sides of
    the identity: misquadrature a profile 2x and the books balance while the world is wrong). COVERED BY: the
    derivation and source-truth tests, which are DIFFERENT MACHINERY guarding a DIFFERENT CLAIM, and which is
    why they are never redundant with a bookkeeping gate.

So the honest scorecard is one of three, and the two it misses were both caught by a person running a grep. This
gate narrows the class; it does not close it. Claiming otherwise would make it a comfort.

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
    "cohesive_energy": (
        "ladder-order",
        "ANCHOR RUNG BUILT: crates/physics/src/materials_oracle.rs::phase_cohesive_energy. "
        "ESTIMATOR RUNG PENDING (not built): the Miedema-class prediction of delta_f_H.",
        "RE-REGISTERED 2026-07-16 AFTER THE PRIOR REGISTRATION WAS REFUTED. The correction is recorded here "
        "rather than deleted, because the next reader will otherwise re-derive the phantom from the same "
        "plausible reasoning. WHAT WAS WRONG: the prior entry claimed 'only Rose's EOS route provides E_coh' and "
        "scheduled an arbitration against it. Rose never provided E_coh. MetallicRoute::cohesive_energy has, "
        "since its birth commit, READ the measured atomization column (its body is an anchor-presence check whose "
        "value is discarded, then table.element(symbol).atomization_enthalpy), and metallic.rs's own doc says so: "
        "'the metallic cohesive energy is the banked atomization enthalpy' and 'at its equilibrium volume the "
        "Rose EOS reproduces E_coh BY CONSTRUCTION (the depth of the binding well)'. Rose CONSUMES E_coh. The "
        "declared overlap set (elemental metals) does not exist as phases in phase_registry.toml, which holds "
        "only oxides and silicates; and an element's delta_f_H is zero in its reference state, so the Hess route "
        "returns the column itself for an element. The sentinel would have compared the column against its own "
        "reflection and agreed to the bit forever. It was billed as the fetch's referee and could not have caught "
        "the one defect the fetch had (the Fe row cited to a CODATA table containing no iron row, caught by a "
        "human census). A TEST THAT CANNOT FAIL, BILLED AS A REFEREE, IS THE COMFORT CLASS IN ITS PUREST FORM. "
        "THE TRUE LADDER, with each rung's code state verified at the site rather than asserted: ANCHOR RUNG "
        "(BUILT) Hess over banked rows, E_coh = sum(elemental atomization) - delta_f_H(compound), measured "
        "thermochemistry throughout. ESTIMATOR RUNG (NOT BUILT, named blocker) the Miedema-class prediction of "
        "delta_f_H for compounds carrying no measured formation-enthalpy row: its form and its carbide extension "
        "are fetched and primary-cited (docs/working/CAPSTONE_FETCH_VALUES_2.md item 4, Niessen and de Boer 1981), "
        "but its phi* and n_ws element parameters are book-pinned and unsourced, which metal_eos.rs records as a "
        "deliberate absence rather than an oversight. OVERLAP SENTINEL (NOT RUNNABLE YET, falsifiable trigger) "
        "MEASURED CARBIDES: SiC and TiC each carry a measured formation-enthalpy row AND a Miedema prediction, "
        "which are two independent derivations of one quantity, so the sentinel becomes runnable the day a carbide "
        "phase and a Miedema prediction both exist. Neither exists today: phase_registry.toml carries no carbide. "
        "BLINDNESS SET, stated beside the discriminating power per the standing rule. The sentinel DISCRIMINATES "
        "errors in delta_f_H (a calorimetric row against a model prediction from element parameters). It is BLIND "
        "to errors in the ATOMIZATION COLUMN, which BOTH rungs read and which therefore cancels out of the "
        "comparison: the same shared-source blindness that killed the phantom, now named rather than hidden. So "
        "WHO REFEREES THE ATOMIZATION COLUMN remains OPEN and is not served by this ladder. Its only independent "
        "twin today is the diatomic per-atom check (doubling each banked O/N/H gas row recovers its molecule's "
        "dissociation enthalpy, a total sourced outside the column); Mg, Si, Al, Ca, Ti and Fe, the elements "
        "silicate E_coh most needs, have none. ROSE reclassifies to what the code always had it as: a form fed by "
        "the column, never a provider of it.",
    ),
    "creep_rate": (
        "ladder-order",
        "crates/physics/src/creep_rows.rs",
        "H&K measured rows (anchor rung) before the MBD form in civsim_materials::creep (estimator rung); "
        "their prefactors are NOT interchangeable, so the ladder is the arbitration",
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
    """Every `pub fn` in laws.rs: a DERIVING provider. The quantity it provides is ITS NAME by default, or the
    quantity an `@provides` annotation in its doc comment names when it carries one.

    WHY THE ANNOTATION EXISTS (owner ruling 2026-07-16). This gate's UNIT OF ANALYSIS IS THE
    QUANTITY-TO-PROVIDER MAPPING, and a function's NAME is only the default GUESS at the quantity it supplies.
    That guess is what made the gate blind to its own worst miss: Slack and Hofmeister are two providers of ONE
    quantity under TWO names, so a name-keyed gate cannot see the collision at all. `@provides` decouples the
    two, so a quantity's providers can be counted even when they disagree about what to call themselves.

    The route considered and rejected was a `[[law]]` block, which is the floor data's marker. It cannot carry
    these: a law block's ports are AXES, and the `[direct]` extractions take only caller-composed values, which
    is precisely what `[direct]` MEANS in the floor registry. There is no portless precedent, and inventing one
    would have forced portless functions into a marker built for axes. Authorship stays human (a human writes
    `@provides` and defends it); the checking stays mechanical.
    """
    out = {}
    pending = None
    for i, line in enumerate(LAWS.read_text().splitlines(), 1):
        ann = re.search(r"@provides\s+(\w+)", line)
        if ann:
            pending = ann.group(1)
            continue
        m = re.match(r"pub fn (\w+)\s*\(", line)
        if m:
            out.setdefault(pending or m.group(1), []).append(
                (m.group(1), f"crates/physics/src/laws.rs:{i}")
            )
            pending = None
        elif line.strip() and not line.strip().startswith("///"):
            # The annotation binds to the NEXT `pub fn`, so any non-doc line between them breaks the binding.
            # An annotation that could drift onto a function it does not describe would be worse than none.
            pending = None
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
    for quantity, providers in sorted(laws.items()):
        sites = fields.get(quantity)
        if not sites:
            continue
        if quantity in registry:
            continue  # a registered relationship of any kind: legal by construction, not a defect
        findings.append((quantity, providers[0][1], sites))
    return findings


def detect_twin_providers(laws, registry):
    """THE SECOND SHAPE, and the one this gate was BLIND to until `@provides` made the mapping explicit: a
    quantity with TWO OR MORE DERIVING providers, under any names, and no registered arbitration.

    The gate's honest scorecard named this miss: Slack's `lattice_thermal_conductivity_w_per_m_k` and
    `hofmeister_lattice_conductivity` are two models of ONE quantity under TWO names, and a name-keyed sweep
    cannot see the collision at all. It was covered by the census-before-build habit and by nothing mechanical.
    A quantity whose providers declare themselves through `@provides` can be COUNTED, so the habit gains a
    machine that does not get tired.

    ITS OWN BLINDNESS, stated beside its power: it sees only providers that DECLARE themselves, so an
    undeclared twin is still invisible, and the annotation is hand-authored, so this converts a silent
    collision into a visible one ONLY where a human wrote the claim down. It reads `laws.rs` alone, so a
    provider in another crate (which is exactly where Slack and Hofmeister live) is still outside its reach.
    The census habit remains the guard for both; this narrows the class, it does not close it.
    """
    out = []
    for quantity, providers in sorted(laws.items()):
        if len(providers) < 2 or quantity in registry:
            continue
        out.append((quantity, providers))
    return out


def canary_check():
    """Run the detector against the SYNTHETIC canary fixture. Returns a list of failures.

    A failure here means THE GATE IS BLIND, which is strictly worse than the gate being noisy, so it is reported
    before any sweep result and it fails the run.
    """
    failures = []
    for name, law_site, field_sites, must_flag, why in CANARIES:
        # One provider per canary quantity: the fixture predates `@provides` and exercises the
        # deriving-plus-storing shape, which is the shape it was written to pin.
        laws = {name: [(name, law_site)]}
        fields = {name: field_sites} if field_sites else {}
        flagged = bool(detect(laws, fields, {}))
        if flagged != must_flag:
            verb = "FAILED TO FLAG" if must_flag else "WRONGLY FLAGGED"
            failures.append(f"{verb} `{name}`: {why}")

    # THE TWIN CANARY, and it is prove-ALWAYS like its sibling: a quantity with two declared providers and no
    # arbitration MUST flag, and the same pair MUST fall silent once registered. A detector nobody has watched
    # fail is a detector nobody has watched.
    twin_laws = {
        "canary_twin_quantity": [
            ("canary_provider_alpha", "fixture:1"),
            ("canary_provider_beta", "fixture:2"),
        ]
    }
    if not detect_twin_providers(twin_laws, {}):
        failures.append(
            "FAILED TO FLAG `canary_twin_quantity`: two declared providers of one quantity, unregistered, "
            "is the different-named-providers class this detector exists for"
        )
    if detect_twin_providers(twin_laws, {"canary_twin_quantity": ("ladder-order", "fixture", "why")}):
        failures.append(
            "WRONGLY FLAGGED `canary_twin_quantity`: a registered arbitration is legal by construction"
        )
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
    twins = detect_twin_providers(laws, REGISTERED_RELATIONSHIPS)

    print("THE DIAMOND GATE: quantities with a DERIVING law and a STORING field, unarbitrated.")
    print(f"canary fixture: {len(CANARIES)} known cases, all behaved (the gate can still see)")
    print(f"scanned {len(laws)} laws.rs kernels against every pub struct field under crates/")
    aliased = sum(1 for q, ps in laws.items() for fn, _ in ps if fn != q)
    print(
        f"quantity-to-provider mapping: {sum(len(p) for p in laws.values())} providers over {len(laws)} "
        f"quantities ({aliased} supplying a quantity under a name other than their own)"
    )
    for q, (kind, where, why) in sorted(REGISTERED_RELATIONSHIPS.items()):
        print(f"registered [{kind}] {q}: {why}")
    print()

    report_twins(twins)
    print()

    if not findings:
        print("no unarbitrated diamonds found.")
        return 1 if (strict and twins) else 0

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


def report_twins(twins):
    """The second finding shape, reported beside the first: one quantity, two declared providers."""
    if not twins:
        print("no twin providers: every `@provides` quantity has exactly one declaring kernel.")
        return
    for quantity, providers in twins:
        print(f"UNARBITRATED TWIN PROVIDERS: `{quantity}`")
        print(f"    ({quantity})")
        print("     |")
        for fn, site in providers:
            print(f"     +-- [DERIVED]  laws::{fn}()            {site}")
        print("     |")
        print("     +-- ARBITRATION: none registered  <-- the defect")
        print()
    print(f"{len(twins)} quantity(s) with two or more declared providers and no arbitration.")


if __name__ == "__main__":
    sys.exit(main())
