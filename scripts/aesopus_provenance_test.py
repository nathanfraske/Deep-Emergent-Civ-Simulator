#!/usr/bin/env python3
# Dry-run provenance battery for the AESOPUS fetch tool: reconstruct each vendored POST body from its recorded
# query (via the shared build_fields) and assert byte-equality against the recorded full_post_fields, WITHOUT
# touching the service. The pull tool is load-bearing provenance infrastructure, so this gives it the same
# regression coverage as the physics: any refactor that silently changes query semantics fails here. The recorded
# full_post_fields is the canonical reproducible body for the query; the actual-fetch receipts (our_md5, the
# service banner, the byte count) are the separate result provenance and are never rewritten. Run with --regen once
# after a deliberate, reviewed builder change to renormalize the recorded bodies to the canonical builder.
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


if __name__ == "__main__":
    main()
