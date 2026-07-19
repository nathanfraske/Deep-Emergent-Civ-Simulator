#!/usr/bin/env python3
# CUSTODY + TRANSCRIPTION + SCHEMA. NOT VALIDATION, AND ONE CHECK IS WEAKER THAN IT LOOKS.
#
# WHAT THIS PROVES.
#   CUSTODY, partly. Both sources are citation-plus-witness: their bytes are NOT in this repository, so
#     there is nothing here to hash. What is proved offline is that each receipt is a well-formed SHA-256,
#     that each archive URL is a syntactically valid Wayback capture URL naming the source URL it stands
#     for, that each entry records a `witness_verified` statement, and that a licence finding exists for
#     each source id in the overlay. When a local-custody copy happens to be present (the machine that ran
#     the fetch), its bytes ARE hashed against the receipt, which is real custody and is why the check is
#     written that way rather than skipped.
#   TRANSCRIPTION. That the q column still carries the values read out of the held tables, per the owner's
#     pre-registered fingerprints, and that its schema holds: every value has a band, an anchor naming a
#     table and page, a scope, and a grade.
#   ONE CROSS-FILE CHECK THAT IS NOT CIRCULAR. Each row claims its q's own fit reproduces the gamma_0
#     already banked in gruneisen.toml. That claim is checked against gruneisen.toml itself, with the
#     expected gamma_0 values taken from THIS AGENT'S READING OF THE HELD 2005 TABLE rather than from
#     either file under test. If someone edits gruneisen.toml's gamma_eos_debye, the pairing claim here
#     breaks and this test fails, which is the whole point: q and gamma_0 are correlated parameters of one
#     inversion and a silent drift in one orphans the other.
#
# WHAT THIS DOES NOT PROVE, stated because the checklist audit of 2026-07-19 found this battery's siblings
# described as more than they were.
#   NOT that either compilation is RIGHT. These are fitted compilation values with real disagreement
#     between the two papers (periclase, enstatite), and this test asserts the transcription, never the
#     physics.
#   NOT that the archive URLs RESOLVE. The test is offline by requirement, so it checks their syntax and
#     the recorded `witness_verified` finding. Resolution was verified at fetch time on 2026-07-19: both
#     captures were re-fetched and hashed BYTE-IDENTICAL to the receipts below. A test cannot re-prove that
#     without a network, and claiming otherwise would be the overstatement this header exists to avoid.
#   NOT that the italic-versus-roman grade read is correct. That was a second-channel read of the PDF font
#     metadata at fetch time; here only its RESULT is asserted, in the q_grade fields.
#   NOT anything about hematite, which has no row. Its absence is checked as an absence, not as a value.
#
# Dry-run provenance battery for the Gruneisen volume-exponent q cited [M] column. No network. Mirrors
# scripts/gruneisen_provenance_test.py (receipt + fingerprint + schema idiom).
#
# Usage:
#   scripts/thermoelastic_anchors_provenance_test.py              verify; exit non-zero on failure
#   scripts/thermoelastic_anchors_provenance_test.py --self-test  prove each check convicts bad input
import hashlib
import os
import re
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DATA = os.path.join(ROOT, "crates", "physics", "data")
HELD_DIR = os.path.join(DATA, "thermoelastic_anchors")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(DATA, "thermoelastic_anchors.toml")
GRUNEISEN = os.path.join(DATA, "gruneisen.toml")
LICENCES = os.path.join(ROOT, "sources", "licences.toml")
# Citation-plus-witness (bronze open access, no reuse grant): bytes live outside the tree. Present only on
# the machine that fetched them, so hashing is best-effort defence in depth; the receipt plus the archive
# carries the entry in CI.
LOCAL_STORES = (
    os.path.expanduser("~/.claude/vendored-sources/thermoelastic_anchors"),
    os.path.expanduser("~/.claude/vendored-sources/gruneisen"),
)

TOL = 1e-9
# The registry id each manifest source is mirrored under (scripts/gen_sources.py derive_id: a directory
# with several sources keys them `<dir>.<name>`). The licence overlay is keyed by that id.
MIRROR_PREFIX = "thermoelastic_anchors"


def load(path):
    with open(path, "rb") as fh:
        return tomllib.load(fh)


# ---------------------------------------------------------------------------------------------------
# 1. RECEIPTS
# ---------------------------------------------------------------------------------------------------

WAYBACK = re.compile(r"^https://web\.archive\.org/web/(\d{14})(id_)?/(https?://\S+)$")


def check_receipts(manifest, licences, stores=LOCAL_STORES):
    """Every source: a well-formed receipt, a well-formed archive witness, a recorded verification, and a
    licence finding in the overlay. Local-custody bytes, when present, are hashed against the receipt."""
    failures = []
    hashed = 0
    witnessed = 0
    for src in manifest.get("source", []):
        name = src.get("name", "<unnamed>")
        custody = src.get("custody", "")
        if custody != "witness":
            failures.append(f"{name}: custody is {custody!r}, expected 'witness' (bronze OA, no reuse grant)")

        sha = str(src.get("sha256", "")).strip()
        if len(sha) != 64 or any(c not in "0123456789abcdef" for c in sha.lower()):
            failures.append(f"{name}: sha256 {sha!r} is not 64 hex characters")

        archive = str(src.get("archived_url", "")).strip()
        m = WAYBACK.match(archive)
        if not m:
            failures.append(f"{name}: archived_url is not a well-formed Wayback capture URL: {archive!r}")
        else:
            witnessed += 1
            # The capture must stand for the source URL the entry cites, or the witness witnesses
            # something else. Compared scheme-insensitively: Wayback records whichever scheme it crawled.
            cited = str(src.get("url", "")).strip()
            if cited and _strip_scheme(m.group(3)) != _strip_scheme(cited):
                failures.append(
                    f"{name}: archived_url captures {m.group(3)!r} but the entry cites {cited!r}"
                )

        if not str(src.get("witness_verified", "")).strip():
            failures.append(f"{name}: no witness_verified record (the re-fetch-and-hash finding)")

        # THE LICENCE REASON, which for a witness is the whole justification for not holding the bytes.
        sid = f"{MIRROR_PREFIX}.{name}"
        rec = licences.get(sid)
        if rec is None:
            failures.append(f"{name}: no licence finding for id {sid!r} in sources/licences.toml")
        elif not str(rec.get("licence", "")).strip():
            failures.append(f"{name}: licence finding for {sid!r} records no licence reason")

        # Defence in depth: hash whatever local custody holds.
        fname = src.get("file")
        if fname:
            for store in stores:
                path = os.path.join(store, fname)
                if not os.path.exists(path):
                    continue
                raw = open(path, "rb").read()
                if hashlib.sha256(raw).hexdigest() != sha:
                    failures.append(f"{name}: local-custody copy at {store} does not match the receipt")
                elif "bytes" in src and len(raw) != int(src["bytes"]):
                    failures.append(f"{name}: byte-count drift, manifest {src['bytes']} held {len(raw)}")
                else:
                    hashed += 1
                break

    if not failures:
        print(
            f"thermoelastic_anchors receipts OK: {witnessed} citation-plus-witness source(s), "
            f"{hashed} verified against local-custody bytes, all licence-cleared"
        )
    return failures


def _strip_scheme(url):
    return re.sub(r"^https?://", "", url).rstrip("/")


# ---------------------------------------------------------------------------------------------------
# 2. SCHEMA: every value carries a band, an anchor, a scope and a grade
# ---------------------------------------------------------------------------------------------------

ANCHOR_HINT = re.compile(r"\bTable\b", re.IGNORECASE)
PAGE_HINT = re.compile(r"\bpage\b", re.IGNORECASE)


def check_anchors(column):
    """Every numeric q in the column has its band, its anchor (naming a table AND a page), the row's scope,
    and a grade. An assumed value must additionally be flagged unusable."""
    failures = []
    values = 0
    for row in column.get("anchor", []):
        name = row.get("name", "<unnamed>")
        if not str(row.get("scope", "")).strip():
            failures.append(f"{name}: no scope")
        # Each channel present must be complete: value, band, grade, and an anchor for that channel.
        for field, anchor_field in (("q", "anchor"), ("q_slb2011", "anchor_slb2011")):
            if field not in row:
                continue
            values += 1
            band = f"{field}_band"
            grade = f"{field}_grade" if field != "q" else "q_grade"
            if band not in row:
                failures.append(f"{name}.{field}: no {band}")
            else:
                try:
                    float(row[band])
                except (TypeError, ValueError):
                    failures.append(f"{name}.{band}: not numeric ({row[band]!r})")
            if not str(row.get(grade, "")).strip():
                failures.append(f"{name}.{field}: no {grade}")
            text = str(row.get(anchor_field, "")).strip()
            if not text:
                failures.append(f"{name}.{field}: no {anchor_field}")
            elif not (ANCHOR_HINT.search(text) and PAGE_HINT.search(text)):
                failures.append(
                    f"{name}.{anchor_field}: does not name both a Table and a page ({text[:60]!r})"
                )
            try:
                float(row[field])
            except (TypeError, ValueError):
                failures.append(f"{name}.{field}: not numeric ({row[field]!r})")

        # AN ASSUMED VALUE MAY NOT PASS AS AN ANCHOR. This is the check that would have caught an
        # italic q = 1 entering wearing a measured grade.
        grades = {str(row.get(k, "")) for k in ("q_grade", "q_slb2011_grade")}
        if "unverified_measurement_candidate" in grades:
            if str(row.get("usable_as_anchor", "")).strip().lower() != "false":
                failures.append(
                    f"{name}: graded unverified_measurement_candidate but usable_as_anchor is not 'false'"
                )

    if not failures:
        print(f"thermoelastic_anchors schema OK: {values} value(s), each with a band, an anchor and a grade")
    return failures


# ---------------------------------------------------------------------------------------------------
# 3. TRANSCRIPTION FINGERPRINTS
# ---------------------------------------------------------------------------------------------------

# The owner's pre-registered fingerprints: (primary q, band, grade, successor q from the 2011 table).
# `None` for the primary means the phase exists only in the successor compilation.
FINGERPRINTS = {
    "periclase": (1.5, 0.2, "fit", 1.7),
    "corundum": (1.3, 0.2, "fit", 1.3),
    "spinel": (2.8, 0.6, "fit", 2.7),
    "forsterite": (2.1, 0.2, "fit", 2.1),
    "fayalite": (3.6, 1.0, "fit", 3.6),
    "enstatite": (7.8, 1.1, "fit", 3.4),
    "quartz": (None, None, "unverified_measurement_candidate", 1.0),
}

# Phases in the registry that this column deliberately does NOT carry. An absent row is the correct result
# and it is asserted as such, so a later well-meaning fill-in fails rather than passing quietly.
DELIBERATELY_ABSENT = {"hematite"}


def check_fingerprints(column):
    failures = []
    rows = {r.get("name"): r for r in column.get("anchor", [])}
    for name, (q, band, grade, q2011) in FINGERPRINTS.items():
        row = rows.get(name)
        if row is None:
            failures.append(f"missing row {name}")
            continue
        if q is None:
            if "q" in row:
                failures.append(f"{name}: carries a primary q, but only the successor compilation has it")
        else:
            if abs(float(row.get("q", "nan")) - q) > TOL:
                failures.append(f"{name}.q: want {q} got {row.get('q')}")
            if abs(float(row.get("q_band", "nan")) - band) > TOL:
                failures.append(f"{name}.q_band: want {band} got {row.get('q_band')}")
        if abs(float(row.get("q_slb2011", "nan")) - q2011) > TOL:
            failures.append(f"{name}.q_slb2011: want {q2011} got {row.get('q_slb2011')}")
        got_grades = {str(row.get(k, "")) for k in ("q_grade", "q_slb2011_grade")}
        if grade not in got_grades:
            failures.append(f"{name}: expected grade {grade!r} on some channel, got {sorted(got_grades)}")

    for name in DELIBERATELY_ABSENT:
        if name in rows:
            failures.append(
                f"{name}: has a row, but it is recorded as unsourceable (no constant q exists in any "
                "primary read). A value here must arrive with its own source, not by filling the gap."
            )
    if not failures:
        print(
            f"thermoelastic_anchors fingerprints OK: {len(FINGERPRINTS)} row(s) reproduce their "
            f"pre-registered q, and {len(DELIBERATELY_ABSENT)} deliberate absence(s) are still absent"
        )
    return failures


# ---------------------------------------------------------------------------------------------------
# 4. THE gamma_0 PAIRING, checked across files against an expectation from the held source
# ---------------------------------------------------------------------------------------------------

# gamma_0 as printed in the held Stixrude & Lithgow-Bertelloni 2005 Table 1, read from the source at fetch
# time. This is the INDEPENDENT expectation: it comes from the paper, not from either file under test, so
# the check convicts a drift in gruneisen.toml as well as one here.
SLB2005_GAMMA_0 = {
    "periclase": 1.50,
    "corundum": 1.32,
    "spinel": 1.02,
    "forsterite": 0.99,
    "fayalite": 1.06,
    "enstatite": 0.67,
}


def check_gamma0_pairing(column, gruneisen):
    """q and gamma_0 are correlated parameters of ONE inversion, so a row claiming its q pairs with the
    banked gamma_0 must still be true of what gruneisen.toml carries today."""
    failures = []
    banked = {}
    for m in gruneisen.get("mineral", []):
        if "gamma_eos_debye" in m:
            banked[m.get("name")] = float(m["gamma_eos_debye"])
    checked = 0
    for row in column.get("anchor", []):
        name = row.get("name")
        claim = str(row.get("gamma_0_matches_banked", "")).strip().lower()
        if claim == "true":
            expected = SLB2005_GAMMA_0.get(name)
            if expected is None:
                failures.append(f"{name}: claims a banked gamma_0 match, but no 2005 gamma_0 is registered")
                continue
            if name not in banked:
                failures.append(
                    f"{name}: claims gamma_0_matches_banked = true, but gruneisen.toml carries no "
                    "gamma_eos_debye for it"
                )
                continue
            if abs(banked[name] - expected) > 1e-6:
                failures.append(
                    f"{name}: pairing broken. The 2005 fit's gamma_0 is {expected}, but gruneisen.toml "
                    f"now carries gamma_eos_debye = {banked[name]}. The banked gamma_0 and this q are no "
                    "longer from one inversion."
                )
                continue
            checked += 1
        elif claim == "no-banked-gamma_0":
            if name in banked:
                failures.append(
                    f"{name}: recorded as having no banked gamma_0, but gruneisen.toml now carries "
                    f"gamma_eos_debye = {banked[name]}. Re-examine the pairing."
                )
    if not failures:
        print(
            f"thermoelastic_anchors gamma_0 pairing OK: {checked} row(s) still pair with the banked "
            "gruneisen.toml gamma_eos_debye from the same 2005 inversion"
        )
    return failures


# ---------------------------------------------------------------------------------------------------
# 5. CLAIMS
# ---------------------------------------------------------------------------------------------------


def check_claims(manifest, column):
    """One claim per carried value, each naming a source the manifest itself defines. A claim with
    no secondary must SAY WHY, which is the honest form when the only other reading is the same authors'
    successor inversion rather than an independent group."""
    failures = []
    known = {f"{MIRROR_PREFIX}.{s.get('name')}" for s in manifest.get("source", [])}
    claims = {c.get("id"): c for c in manifest.get("claim", [])}
    for row in column.get("anchor", []):
        cid = f"q.{row.get('name')}"
        if cid not in claims:
            failures.append(f"no claim {cid!r} for the row it carries")
    for cid, claim in claims.items():
        primary = list(claim.get("primary") or [])
        secondary = list(claim.get("secondary") or [])
        if not primary:
            failures.append(f"{cid}: no primary")
        for ref in primary + secondary:
            if ref not in known:
                failures.append(f"{cid}: cites {ref!r}, which this manifest does not define")
        if primary and secondary and set(primary) == set(secondary):
            failures.append(f"{cid}: primary and secondary name the same source (one witness twice)")
        if not secondary and not str(claim.get("single_witness_reason", "")).strip():
            failures.append(f"{cid}: single witness with no stated reason")
    if not failures:
        print(f"thermoelastic_anchors claims OK: {len(claims)} claim(s), each resolving and each witnessed")
    return failures


# ---------------------------------------------------------------------------------------------------


def run(manifest, column, gruneisen, licences):
    failures = []
    failures += check_receipts(manifest, licences)
    failures += check_anchors(column)
    failures += check_fingerprints(column)
    failures += check_gamma0_pairing(column, gruneisen)
    failures += check_claims(manifest, column)
    return failures


def load_licences():
    try:
        return {r["id"]: r for r in load(LICENCES).get("licence", []) if r.get("id")}
    except FileNotFoundError:
        return {}


def self_test():
    """Each check must CONVICT synthetic bad input. A check that cannot fail is decoration."""
    good_src = {
        "name": "slb_2005_mantle_minerals_i",
        "custody": "witness",
        "sha256": "95e1cd9b58c28874b2617e68937a61c22ad78032ba4bc2b5a2305a104dc54604",
        "url": "https://example.org/a.pdf",
        "archived_url": "https://web.archive.org/web/20260718151531/https://example.org/a.pdf",
        "witness_verified": "re-fetched and hashed identical",
    }
    lic = {f"{MIRROR_PREFIX}.slb_2005_mantle_minerals_i": {"licence": "bronze OA, no reuse grant"}}
    cases = []

    # Receipts.
    cases.append(("bad sha256", check_receipts({"source": [dict(good_src, sha256="deadbeef")]}, lic, ())))
    cases.append(("bad archive url", check_receipts({"source": [dict(good_src, archived_url="http://x")]}, lic, ())))
    cases.append(("archive of another document",
                  check_receipts({"source": [dict(good_src, url="https://example.org/OTHER.pdf")]}, lic, ())))
    cases.append(("no witness_verified", check_receipts({"source": [dict(good_src, witness_verified="")]}, lic, ())))
    cases.append(("no licence finding", check_receipts({"source": [good_src]}, {}, ())))
    cases.append(("wrong custody", check_receipts({"source": [dict(good_src, custody="in_repo")]}, lic, ())))

    # Schema.
    row = {
        "name": "x", "q": "1.5", "q_band": "0.2", "q_grade": "fit",
        "scope": "ambient", "anchor": "Table 1, page 30",
    }
    cases.append(("no band", check_anchors({"anchor": [{k: v for k, v in row.items() if k != "q_band"}]})))
    cases.append(("no scope", check_anchors({"anchor": [dict(row, scope="")]})))
    cases.append(("anchor without a page", check_anchors({"anchor": [dict(row, anchor="Table 1")]})))
    cases.append(("no grade", check_anchors({"anchor": [dict(row, q_grade="")]})))
    cases.append(("assumed value passing as an anchor",
                  check_anchors({"anchor": [dict(row, q_grade="unverified_measurement_candidate")]})))

    # Fingerprints.
    good_col = load(COLUMN)
    drifted = {"anchor": [dict(r) for r in good_col["anchor"]]}
    drifted["anchor"][0]["q"] = "9.9"
    cases.append(("drifted q", check_fingerprints(drifted)))
    filled = {"anchor": [dict(r) for r in good_col["anchor"]] + [{"name": "hematite", "q": "1.0"}]}
    cases.append(("hematite filled in", check_fingerprints(filled)))

    # gamma_0 pairing.
    cases.append(("banked gamma_0 drifted", check_gamma0_pairing(
        {"anchor": [{"name": "periclase", "gamma_0_matches_banked": "true"}]},
        {"mineral": [{"name": "periclase", "gamma_eos_debye": "1.99"}]})))
    cases.append(("banked gamma_0 vanished", check_gamma0_pairing(
        {"anchor": [{"name": "periclase", "gamma_0_matches_banked": "true"}]}, {"mineral": []})))

    # Claims.
    cases.append(("claim citing an unknown source", check_claims(
        {"source": [good_src], "claim": [{"id": "q.x", "primary": ["ghost"],
                                          "single_witness_reason": "r"}]}, {"anchor": []})))
    cases.append(("single witness with no reason", check_claims(
        {"source": [good_src], "claim": [{"id": "q.x", "primary": [f"{MIRROR_PREFIX}.slb_2005_mantle_minerals_i"]}]},
        {"anchor": []})))
    cases.append(("row with no claim", check_claims(
        {"source": [good_src], "claim": []}, {"anchor": [{"name": "periclase"}]})))

    silent = [label for label, found in cases if not found]
    if silent:
        print("\nSELF-TEST FAILED: these checks did not convict bad input:")
        for label in silent:
            print(f"  {label}")
        return 1
    print(f"\nthermoelastic_anchors self-test OK: {len(cases)} synthetic defect(s), all convicted")
    return 0


def main():
    if "--self-test" in sys.argv[1:]:
        return self_test()
    failures = run(load(MANIFEST), load(COLUMN), load(GRUNEISEN), load_licences())
    if failures:
        print(f"\nTHERMOELASTIC ANCHORS PROVENANCE FAILED ({len(failures)})")
        for f in failures:
            print(f"  {f}")
        return 1
    print("\nthermoelastic_anchors provenance OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
