#!/usr/bin/env python3
# CUSTODY + TRANSCRIPTION + ANALYTIC, NOT VALIDATION.
#
# WHAT THIS PROVES, stated before what it checks, because the audit of 2026-07-19 found this battery's
# siblings described uniformly as reconstructing each fetch and asserting byte-equality, which was true of
# some and false of others.
#
#   CUSTODY      the 13 files held in crates/physics/data/phase_conductivity/ hash to the receipts the
#                manifest records for them. This is real and offline: it proves the bytes in the tree are
#                the bytes the fetch recorded.
#   TRANSCRIPTION every [[claim]] in the manifest agrees with the value sitting in the data column
#                itself, and every source id a claim cites resolves in sources/registry.toml. This proves
#                the read record and the column did not drift apart.
#   ANALYTIC     the one DERIVED step in the whole column is recomputed from the source's own printed
#                coefficient: hematite's 18.20 W/(m*K) is 1/(1.844e-4 * 298) from Akiyama et al. 1992
#                equation (2). It also recomputes the ADJACENT Fe3O4 branch and asserts the banked value
#                is not that one, which is the guard against having read the wrong row of a two-row block.
#                And it recomputes atoms_per_primitive_cell from Z, the formula content and the centring
#                for all eight phases.
#   OMISSION     spinel still carries no anchor. An omission that no test watches is an omission that
#                gets quietly filled by the next agent who wants the refusal to stop.
#
# WHAT IT DOES NOT PROVE. That Akiyama's measurement is right, or that 18.20 W/(m*K) is the thermal
# conductivity of hematite. That would need an independent determination, and the honest state of the
# literature reached is that none exists in-frame: the row is banked with no band for exactly that reason.
# It also cannot check the three WITNESS sources' bytes, because the licence forbids holding them (see the
# manifest header): for those it verifies the receipt is well formed and an archive URL is recorded, which
# is a schema check and not custody. No network is used anywhere.
import hashlib
import os
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
HELD_DIR = os.path.join(ROOT, "crates", "physics", "data", "phase_conductivity")
MANIFEST = os.path.join(HELD_DIR, "manifest.toml")
COLUMN = os.path.join(ROOT, "crates", "physics", "data", "phase_conductivity.toml")
REGISTRY = os.path.join(ROOT, "sources", "registry.toml")
MIRRORED = os.path.join(ROOT, "sources", "mirrored.toml")

# Akiyama, T., Ohta, H., Takahashi, R., Waseda, Y. & Yagi, J., 1992, ISIJ International 32(7), 829-837,
# equation (2), printed page 831, read from a 300 dpi render. Both branches are carried so the test can
# convict a read of the wrong one; only the Fe2O3 branch is banked.
ISIJ_FE2O3_A = 1.844e-4
ISIJ_FE2O3_T_LOW = 298.0
ISIJ_FE3O4_A = 1.693e-4
# The three sources the 2026-07-20 hematite fetch added, none of which may hold bytes in this tree.
WITNESS_IDS = (
    "akiyama_1992_isij_iron_oxides",
    "akiyama_1991_tetsu_dense_iron_oxides",
    "tprc_vol2_nonmetallic_solids",
)

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def load(path):
    with open(path, "rb") as handle:
        return tomllib.load(handle)


manifest = load(MANIFEST)
column = load(COLUMN)
registry = load(REGISTRY)
mirrored = load(MIRRORED) if os.path.isfile(MIRRORED) else {"source": []}
known_ids = {s["id"] for s in registry.get("source", [])} | {
    s["id"] for s in mirrored.get("source", [])
}
rows = {r["name"]: r for r in column.get("conductivity", [])}
claims = {c["id"]: c for c in manifest.get("claim", [])}

# ---------------------------------------------------------------------------------------------------
# CUSTODY: the held bytes hash to their recorded receipts.
# ---------------------------------------------------------------------------------------------------
holdings = manifest.get("holding", [])
check(len(holdings) >= 13, f"expected at least 13 held files, manifest lists {len(holdings)}")
for holding in holdings:
    path = os.path.join(HELD_DIR, holding["file"])
    if not os.path.isfile(path):
        failures.append(f"custody: held file missing from the tree: {holding['file']}")
        continue
    with open(path, "rb") as handle:
        digest = hashlib.sha256(handle.read()).hexdigest()
    check(
        digest == holding["sha256"],
        f"custody: {holding['file']} hashes {digest}, manifest records {holding['sha256']}",
    )
    check(
        holding["source_id"] in known_ids,
        f"custody: {holding['file']} cites unknown source id {holding['source_id']}",
    )

# ---------------------------------------------------------------------------------------------------
# SCHEMA, not custody: the witness sources hold no bytes, so only their receipts can be checked.
# ---------------------------------------------------------------------------------------------------
by_id = {s["id"]: s for s in registry.get("source", [])}
for wid in WITNESS_IDS:
    entry = by_id.get(wid)
    if entry is None:
        failures.append(f"witness: {wid} is not registered in sources/registry.toml")
        continue
    check(entry.get("custody") == "witness", f"witness: {wid} must be custody = witness")
    check(
        entry.get("redistributable") is False,
        f"witness: {wid} must record redistributable = false, so no later agent holds its bytes",
    )
    sha = str(entry.get("sha256", ""))
    check(
        len(sha) == 64 and all(c in "0123456789abcdef" for c in sha),
        f"witness: {wid} has a malformed sha256 receipt",
    )
    archive = str(entry.get("archived_url", ""))
    check(archive.startswith("http"), f"witness: {wid} records no archive URL")
    check(
        bool(str(entry.get("extract", "")).strip()),
        f"witness: {wid} holds no bytes, so it must carry an extract",
    )
    # The bytes must NOT be in the tree. A licence finding that nobody enforces is a comment.
    for root, _dirs, files in os.walk(HELD_DIR):
        for name in files:
            check(
                wid not in name,
                f"witness: {wid} appears to have bytes held at {os.path.join(root, name)}",
            )

# ---------------------------------------------------------------------------------------------------
# TRANSCRIPTION: claims resolve, and the kappa claims agree with the column.
# ---------------------------------------------------------------------------------------------------
for cid, claim in claims.items():
    cited = list(claim.get("primary", [])) + list(claim.get("secondary", []))
    check(bool(cited), f"transcription: claim {cid} cites no source")
    for source_id in cited:
        check(
            source_id in known_ids,
            f"transcription: claim {cid} cites unresolved source id {source_id}",
        )
    if not claim.get("secondary"):
        check(
            bool(str(claim.get("single_witness_reason", "")).strip()),
            f"transcription: claim {cid} has one witness and no stated reason",
        )

# The seven anchored phases, each claim's number against the column's own field.
ANCHORED = {
    "quartz": None,  # anisotropic: the column states principal values, the loader derives the scalar
    "corundum": "36.0",
    "periclase": "48.4",
    "forsterite": "5.158",
    "fayalite": "3.161",
    "enstatite": "4.961",
    "hematite": "18.20",
}
for phase, expected in ANCHORED.items():
    row = rows.get(phase)
    check(row is not None, f"transcription: phase {phase} is missing from the column")
    if row is None:
        continue
    check(
        f"conductivity.kappa_298.{phase}" in claims,
        f"transcription: phase {phase} carries an anchor with no claim in the manifest",
    )
    if expected is None:
        check(
            "kappa_298_parallel_c" in row and "kappa_298_perpendicular_c" in row,
            f"transcription: {phase} should state principal values",
        )
        continue
    check(
        row.get("kappa_298_w_per_m_k") == expected,
        f"transcription: {phase} column value {row.get('kappa_298_w_per_m_k')} != {expected}",
    )
    check(
        expected in claims[f"conductivity.kappa_298.{phase}"]["quantity"],
        f"transcription: {phase} claim quantity does not carry {expected}",
    )

# ---------------------------------------------------------------------------------------------------
# ANALYTIC: recompute the one derived value, and the crystallographic counts.
# ---------------------------------------------------------------------------------------------------
recomputed = 1.0 / (ISIJ_FE2O3_A * ISIJ_FE2O3_T_LOW)
banked = float(rows["hematite"]["kappa_298_w_per_m_k"])
check(
    abs(recomputed - banked) < 5e-3,
    f"analytic: 1/(1.844e-4 * 298) = {recomputed:.4f} but the column banks {banked}",
)
# THE ADJACENT-ROW GUARD. Akiyama's equation (2) prints the Fe2O3 branch immediately above the Fe3O4
# branch, and the two differ only in the third digit of the coefficient. Reading the wrong one is the
# single most likely transcription failure for this value, so it is convicted explicitly.
magnetite = 1.0 / (ISIJ_FE3O4_A * ISIJ_FE2O3_T_LOW)
check(
    abs(magnetite - banked) > 1.0,
    f"analytic: the banked hematite value {banked} matches the Fe3O4 branch {magnetite:.4f}, "
    "which means the magnetite row was read instead of the hematite row",
)
# The hematite row is stated at the fit's own lower bound, not at the 300 K the other rows use.
check(
    rows["hematite"].get("kappa_298_temperature_k") == "298",
    "analytic: hematite must state 298 K, the lower bound of the range its fit declares",
)

for name, row in rows.items():
    z = int(row["formula_units_per_conventional_cell"])
    per_formula = int(row["atoms_per_formula_unit"])
    points = int(row["lattice_points_per_conventional_cell"])
    stated = int(row["atoms_per_primitive_cell"])
    conventional = z * per_formula
    check(
        conventional % points == 0,
        f"analytic: {name} conventional content {conventional} is not divisible by centring {points}",
    )
    check(
        conventional // points == stated,
        f"analytic: {name} reconstructs to {conventional // points}, column states {stated}",
    )

# ---------------------------------------------------------------------------------------------------
# OMISSION: spinel is still unanchored, and the record still says why.
# ---------------------------------------------------------------------------------------------------
spinel = rows.get("spinel")
check(spinel is not None, "omission: spinel row is missing entirely")
if spinel is not None:
    for field in ("kappa_298_w_per_m_k", "kappa_298_parallel_c", "kappa_298_perpendicular_c"):
        check(
            field not in spinel,
            f"omission: spinel now carries {field}. No source reached measures the stoichiometric "
            "phase; if one has been found, this test is what should be updated alongside it.",
        )
    check(
        "conductivity.kappa_298.spinel" not in claims,
        "omission: a spinel kappa claim exists with no value in the column",
    )
manifest_text = open(MANIFEST, encoding="utf-8").read()
check(
    "MgO . 3.5 Al2O3" in manifest_text,
    "omission: the manifest no longer records that Slack's 1962 spinel is the alumina-rich "
    "MgO . 3.5 Al2O3, which is the finding that keeps a later agent from banking his value",
)

if failures:
    print(f"phase_conductivity provenance: {len(failures)} FAILURE(S)")
    for failure in failures:
        print(f"  - {failure}")
    sys.exit(1)
print(
    f"phase_conductivity provenance OK: {len(holdings)} held files hashed, "
    f"{len(claims)} claims resolved, {len(rows)} cell counts recomputed, "
    "hematite recomputed from Akiyama eq. (2), spinel omission intact"
)
