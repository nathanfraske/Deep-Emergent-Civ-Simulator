#!/usr/bin/env python3
# RECIPE RECONSTRUCTION, NOT VALIDATION.
#
# Proves the POST recipe reconstructs and the manifest is self-consistent. It does NOT hash the gas .dat files
#  against a receipt, and it does NOT validate the opacities physically.
#
# WHY THE LABEL IS HERE. An audit found this battery described uniformly as one that reconstructs each
# fetch from its recipe and asserts byte-equality, for all eight tests. That was true of some and false
# of others, and the difference matters: custody proves the bytes we hold are the bytes we fetched,
# transcription proves our column matches the held source, and neither proves the source is RIGHT. A
# test that reads its expectation from the file under test does not even prove independence. Saying so
# where the test is, is harder to drift than saying it in a document.
#
# Dry-run provenance battery for the AESOPUS fetch tool: reconstruct each vendored POST body from its recorded
# query (via the shared build_fields) and assert byte-equality against the recorded full_post_fields, WITHOUT
# touching the service. The pull tool is load-bearing provenance infrastructure, so this gives it the same
# regression coverage as the physics: any refactor that silently changes query semantics fails here. The recorded
# full_post_fields is the canonical reproducible body for the query; the actual-fetch receipts (our_md5, the
# service banner, the byte count) are the separate result provenance and are never rewritten. Run with --regen once
# after a deliberate, reviewed builder change to renormalize the recorded bodies to the canonical builder.
#
# THE RECIPE/RECEIPT IMMUTABILITY RULE (engine law): query provenance (full_post_fields, the recipe) may be
# canonicalized only when the service-side semantics are PROVEN identical -- the byte-identical result md5s prove it
# for the "0.7" vs "0.70" xhmax skew here -- whereas result provenance (our_md5, banner, bytes, the receipt) is
# immutable forever, no exceptions, because the receipts are the artifacts and the queries are the recipes. A recipe
# can be re-notated; a receipt cannot be touched. --regen rewrites full_post_fields ONLY and never the receipts.
import sys, json, glob, os

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from aesopus_fields import build_fields

FORM = os.path.join(HERE, "aesopus_1.0_form.html")
DATA = os.path.join(HERE, "..", "crates", "physics", "data", "aesopus_lowt")


def reconstruct(prov, form_html):
    q = prov["query"]
    return build_fields(
        form_html, q["solmix"], q["zeta_ref"], q["xhmin"], q["fco1"], q["fc1"], q["fn1"]
    )


# The opacity manifest's composition must be DERIVABLE from the fetch POST parameters (keyed on the recipe, not the
# filename): X = xhmin, Z = zeta_ref, C/O = (C/O)_ref * 10^fco1. This cross-check keeps the manifest from drifting
# away from the queries it claims to describe.
REF_CO = {"7": 0.54954}  # AGSS09 (C/O)sun; all vendored pulls use solmix=7


def parse_manifest():
    import re

    text = open(os.path.join(DATA, "manifest.toml")).read()
    entries = {}
    for block in re.split(r"\[\[grid\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"([^"]*)"', block))
        entries[g["file"]] = g
    return entries


def check_manifest(provs):
    manifest = parse_manifest()
    failures = 0
    for p in provs:
        prov = json.load(open(p))
        q = prov["query"]
        fname = "aesopus1.0_gasonly_%s.dat" % prov["label"]
        m = manifest.get(fname)
        if not m:
            print("MANIFEST MISSING", fname)
            failures += 1
            continue
        fco1 = float(q["fco1"]) if q["fco1"] not in ("", "-") else 0.0
        want = {
            "hydrogen_mass_fraction": float(q["xhmin"]),
            "metallicity": float(q["zeta_ref"]),
            "carbon_to_oxygen": REF_CO[q["solmix"]] * (10**fco1),
        }
        for k, v in want.items():
            if abs(float(m[k]) - v) > 1e-4 * max(1.0, abs(v)):
                print("MANIFEST DRIFT", fname, k, "manifest", m[k], "recipe", v)
                failures += 1
    assert failures == 0, f"{failures} manifest/recipe mismatch(es)"
    print(f"PASS: {len(provs)} manifest compositions derive from their POST parameters")


def check_optical_manifest():
    # The optical-constants fixture manifest's md5 (the receipt) must match the vendored .dat it names, so a corrupted
    # or swapped n,k file fails the integrity gate. Same fixture-manifest idiom as the opacity grids.
    import re, hashlib

    oc = os.path.join(HERE, "..", "crates", "physics", "data", "optical_constants_aesopus")
    text = open(os.path.join(oc, "manifest.toml")).read()
    failures = 0
    n = 0
    for block in re.split(r"\[\[species\]\]", text)[1:]:
        g = dict(re.findall(r'(\w+)\s*=\s*"((?:[^"\\]|\\.)*)"', block))
        n += 1
        path = os.path.join(oc, g["file"])
        if not os.path.exists(path):
            print("OPTICAL MISSING", g["file"])
            failures += 1
            continue
        got = hashlib.md5(open(path, "rb").read()).hexdigest()
        if got != g["md5"]:
            print("OPTICAL MD5 DRIFT", g["file"], "manifest", g["md5"], "file", got)
            failures += 1
    assert failures == 0, f"{failures} optical-fixture md5 mismatch(es)"
    print(f"PASS: {n} optical-fixture species match their manifest md5")


def main():
    regen = "--regen" in sys.argv
    form_html = open(FORM).read()
    provs = sorted(glob.glob(os.path.join(DATA, "*.provenance.json")))
    assert provs, "no provenance records found"
    failures = 0
    for p in provs:
        prov = json.load(open(p))
        rebuilt = reconstruct(prov, form_html)
        if regen:
            prov["full_post_fields"] = rebuilt
            json.dump(prov, open(p, "w"), indent=2)
            print("regen", os.path.basename(p))
            continue
        recorded = prov["full_post_fields"]
        if rebuilt != recorded:
            failures += 1
            diff = {
                k: (recorded.get(k), rebuilt.get(k))
                for k in set(recorded) | set(rebuilt)
                if recorded.get(k) != rebuilt.get(k)
            }
            print("MISMATCH", os.path.basename(p), diff)
        else:
            print("OK", os.path.basename(p))
    if not regen:
        assert failures == 0, f"{failures} provenance byte-equality mismatch(es)"
        print(f"PASS: {len(provs)} provenance records reproduce byte-identically")
        check_manifest(provs)
        check_optical_manifest()


if __name__ == "__main__":
    main()
