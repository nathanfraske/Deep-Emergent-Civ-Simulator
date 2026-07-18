#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Q1 Stone 1, items 2 and 3: the sealed-Fixed-constructor ratchet.

Machine-enforce derive-do-not-author: a value in the path of world content must be derived from the
floor and the situation, or read as world data, never authored inline. The seal is a CI ratchet, not
a compile seal, because the split between an authored world-content value and a legitimate
engine-mechanics constant is SEMANTIC, not syntactic: both are `Fixed::from_ratio(...)`, and the
determinism crates carry the constructors in the low thousands, dominated by the documented
Principle-11 engine-mechanics exemption (divisors, exponents, unit-bridge ratios, math constants).
A hard compile seal over that population would token-wrap thousands of legitimate sites; the ratchet
gates MAIN at merge, exactly where an authored value must not land.

Two enforcement strengths, one mechanism:
  - HARD RATCHET (fails CI) over the raw-representation constructors `::from_bits(` and the
    decimal-parse constructor `::from_decimal_str(` in the non-exempt modules. These are the narrow,
    audited populations. Every occurrence is grandfathered in scripts/constructor_baseline.tsv with a
    one-time AUDIT CLASSIFICATION (legitimate-mechanics, bit-arithmetic, deserialization, test, or
    labelled dev-fixture), never a blanket freeze; a row classified "authored, should derive" is a
    surfaced DEFECT for a floor read. The gate fails on any count above its baseline (a new site to
    classify) or below it (update the baseline when a site is removed).
  - MODULE EXEMPTION (wholesale): a true kernel module never holds an inline world-content value (its
    world-content arrives as an argument read from the floor), and a loader legitimately parses cited
    floor data, so the modules in EXEMPT_MODULES seal wholesale and need no per-site reason.
  - ADVISORY (reports, never fails) over `::from_int(` and `::from_ratio(`: publishes the per-crate
    distribution so the leaf-first typed-migration line the owner is sizing has its live numbers. It
    does not gate: the migration of that population is the owner-scoped work, not a day-one hard gate.

`--generate` emits the observed non-exempt from_bits / from_decimal_str counts as a baseline skeleton
(fill the reason column from the audit). `--self-test` proves a synthetic new `from_bits` is caught.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
# The crates whose source is on the path of world content. Out of scope: `core` (the `Fixed` type's
# own home, where its raw-bit transcendental table legitimately lives) and `units` (the constants
# quarantine, the one authored place, where the fundamentals and measured floor constants live).
CRATES = ["crates/bio/src", "crates/physics/src", "crates/sim/src", "crates/world/src"]
HARD = ["::from_bits(", "::from_decimal_str("]
ADVISORY = ["::from_int(", "::from_ratio("]
BASELINE = ROOT / "scripts" / "constructor_baseline.tsv"

# Modules exempt wholesale (no per-site reason): kernel modules that carry only argument-read
# world-content and engine-mechanics constants, and the floor and manifest loaders that parse cited
# data. Each is a true kernel or a loader, verified to hold no inline world-content value.
EXEMPT_MODULES = {
    "crates/physics/src/laws.rs",       # kernel: math constants (pi^2, von Mises) and argument-read law arithmetic
    "crates/physics/src/lib.rs",        # the physics registry loader (parses cited axis/substance data)
    "crates/physics/src/periodic.rs",   # the periodic-table loader (parses cited atomic weights)
    "crates/physics/src/petrology_data.rs", # the phase-registry loader (parses cited phase thermodynamics)
    "crates/physics/src/melting_data.rs", # the melting-endmember loader (parses cited [M] T_m/dH_fus/dV_fus signatures)
    "crates/physics/src/ionic_radii.rs", # the ionic-radii loader (parses cited Shannon 1976 crystal radii)
    "crates/physics/src/ionization_ladder.rs", # the ionization-ladder loader (parses cited NIST successive IEs)
    "crates/physics/src/d_state_radius.rs", # the d-state-radius loader (parses cited Clementi-Raimondi Zeff)
    "crates/physics/src/metal_eos.rs",  # the metal-EOS-anchor loader (parses cited WebElements molar volume + B_0)
    "crates/physics/src/rose_eos.rs",   # the Rose UBER EOS law (cited floor constants: Avogadro + eV unit conversions)
    "crates/physics/src/tm_oxide_lattice_energy.rs", # the TM-oxide Born-Haber loader (parses cited [M] lattice energies)
    "crates/physics/src/band_gap.rs",   # the band-gap loader (parses cited [M] gaps + compute-once HYBRID/GW eigenvalues)
    "crates/physics/src/term_values.rs", # the term-value loader (parses cited Herman-Skillman eps_s/eps_p, gated on the fetch)
    "crates/physics/src/crystal_field.rs", # the crystal-field loader (parses cited Jorgensen f/g, Racah B, oxide Delta_o)
    "crates/physics/src/stoner.rs",     # the Stoner loader (parses cited Janak 1977 I and nonmagnetic-band N)
    "crates/physics/src/quantities.rs", # quantity definitions and the wide-decimal doc reference
    "crates/bio/src/calibration.rs",    # the calibration-manifest loader (parses the owner's reserved values)
    "crates/sim/src/astro.rs",          # the stellar-flux derivation (parses cited astronomical anchors L_sun/AU)
    "crates/physics/src/opacity.rs",    # the disk-opacity generator (parses cited fundamentals e/eps_0/m_e/c for the Thomson-scattering derivation)
    "crates/physics/src/optical_constants.rs", # the optical-constants loader (parses cited per-species n,k tables)
}


def _count(text: str, pat: str) -> int:
    # Exact-constructor match: `::from_bits(` matches `Fixed::from_bits(` / `Self::from_bits(` but not
    # `from_bits_i128(` (the `_i128` breaks the trailing paren) and not a method name like
    # `set_temp_from_bits(` (no `::` before it).
    return text.count(pat)


def scan(root: pathlib.Path, patterns) -> dict:
    counts = {}
    for crate in CRATES:
        for path in sorted((root / crate).rglob("*.rs")):
            rel = path.relative_to(root).as_posix()
            if rel in EXEMPT_MODULES:
                continue
            text = path.read_text(encoding="utf-8")
            for pat in patterns:
                n = _count(text, pat)
                if n:
                    counts[(pat, rel)] = n
    return counts


def load_baseline(path: pathlib.Path) -> dict:
    base = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) < 4:
            raise SystemExit(f"malformed baseline row (need pattern<TAB>path<TAB>count<TAB>class<TAB>reason): {line!r}")
        base[(fields[0], fields[1])] = (int(fields[2]), fields[3])
    return base


def hard_check(root: pathlib.Path) -> tuple:
    """Return (violations, defect_rows). Violations fail the gate; defect_rows are surfaced, not frozen."""
    observed = scan(root, HARD)
    baseline = load_baseline(BASELINE)
    violations = []
    for key in sorted(observed.keys() | baseline.keys()):
        pat, rel = key
        got = observed.get(key, 0)
        want = baseline.get(key, (0, ""))[0]
        if got > want:
            violations.append(
                f"NEW inline `{pat}` in {rel} ({got}, baseline {want}). Classify it in "
                f"scripts/constructor_baseline.tsv: legitimate-mechanics/bit-arithmetic/deser/test/"
                f"dev-fixture is grandfathered with a reason; an authored world-content value must "
                f"instead read from the floor (the physics registry, the calibration manifest, the "
                f"periodic table) or derive from it."
            )
        elif got < want:
            violations.append(f"stale baseline: `{pat}` in {rel} ({got}, baseline {want}). Lower or delete its row.")
    defect_rows = [f"{p} {r}" for (p, r), (_, cls) in sorted(baseline.items()) if cls == "authored-should-derive"]
    return violations, defect_rows


def advisory(root: pathlib.Path) -> None:
    counts = scan(root, ADVISORY)
    per_crate = {}
    for (pat, rel), n in counts.items():
        crate = rel.split("/")[1]
        per_crate.setdefault(crate, {}).setdefault(pat, 0)
        per_crate[crate][pat] += n
    print("advisory: the interleaved from_int / from_ratio distribution (the owner-scoped migration line):")
    for crate in sorted(per_crate):
        parts = ", ".join(f"{pat} x{per_crate[crate][pat]}" for pat in sorted(per_crate[crate]))
        print(f"  {crate}: {parts}")
    print("  (kernel modules are exempt wholesale; this counts only the non-exempt, interleaved modules)")


def generate(root: pathlib.Path) -> int:
    for (pat, rel), n in sorted(scan(root, HARD).items()):
        print(f"{pat}\t{rel}\t{n}\tCLASSIFY\treason")
    return 0


def self_test(root: pathlib.Path) -> int:
    probe = root / "crates" / "sim" / "src" / "__constructor_gate_probe.rs"
    probe.write_text("let _ = Fixed::from_bits(1);\n", encoding="utf-8")
    try:
        violations, _ = hard_check(root)
    finally:
        probe.unlink()
    hit = any("__constructor_gate_probe.rs" in v for v in violations)
    print("constructor gate self-test: " + ("PASS (a synthetic new from_bits is caught)" if hit else "FAIL"))
    return 0 if hit else 1


def main() -> int:
    args = sys.argv[1:]
    if "--generate" in args:
        return generate(ROOT)
    if "--self-test" in args:
        return self_test(ROOT)
    violations, defect_rows = hard_check(ROOT)
    advisory(ROOT)
    print(f"audit: {len(defect_rows)} baselined site(s) classified authored-should-derive (a floor-read defect).")
    for d in defect_rows:
        print(f"  DEFECT: {d}")
    if violations:
        print("constructor gate: FAIL")
        for v in violations:
            print(f"  - {v}")
        return 1
    print("constructor gate: clean (the hard-ratchet constructors match the audited baseline)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
