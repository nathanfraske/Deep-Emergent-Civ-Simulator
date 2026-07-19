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

"""THE SOURCES GATE: a vendored source is checksummed, archived, scoped, slimmed, licence-cleared, and
cross-checked, or it does not land.

WHY THIS EXISTS. The project vendors sources well and vendored them in five different shapes, so the
discipline lived in the head of whichever agent ran the fetch. Measured before this gate: 27 sources, all
27 with a sha256, but only 13 with an archive URL, only 5 with a scope, and ZERO with a slim record or any
statement of what the licence permits. A pipeline nobody enforces is a suggestion, and the fields that were
missing are exactly the ones a person forgets when the fetch is going well.

WHAT IT CHECKS, one per owner rule (docs/working/FETCH_PIPELINE_PLAN.md):

  sha256     the receipt. md5 alone does not pass: md5 collisions are practically constructible and a
             collision-forgeable receipt is not tamper-evident, which is the one property a receipt has.
  archive    a link is not provenance because a link rots. Either an archive URL or an explicit
             `archive_pending` with a stated reason, which is the honest form when a host bot-walls the
             crawler (PR #201 hit exactly this on two IOP entries).
  scope      the regime the source's values apply to. A number without its scope gets used outside it.
  slim       what was kept and what was dropped, so a slim is never mistaken for the whole document.
             `slim` for held bytes, `extract` for a witness whose bytes are not held.
  licence    THE REDISTRIBUTION QUESTION. Not "is it paywalled" but "does the licence permit THIS use":
             an open-source, educational, non-profit project redistributing the bytes. A gold open-access
             paper behind a publisher page but carrying CC-BY is fine to hold. A free-to-download paper
             reserving all rights is not, however easy the bytes are to get. So a BYTE-HOLDING entry must
             record either that it is redistributable, or a free open route that is.
  secondary  a claim carrying only a primary has one witness. Two independent witnesses are what let an
             erratum or an author-contact case be settled, and what makes a transcription error visible.

Plus the two integrity checks a registry needs to be trustworthy at all: ids are unique across the two
registry files, and a `holding` path that claims in-tree bytes has those bytes with a matching checksum.

Plus THE HOP, which is the point of the whole exercise: a `// @sources:` marker on a deriving function, or
a `source_id =` reference on a manifest row, must resolve to a real registry id. With it, a walk from a
derived quantity reaches a held, checksummed, archived primary, and "is this substrate traceable to a
vendored primary?" becomes a question a gate answers rather than a person reading both ends.

GRANDFATHERED, so it ratchets. The 27 pre-existing sources take rows in scripts/sources_baseline.tsv
rather than blocking every commit until someone audits all of them. A baseline row is an ADMISSION THAT
NOBODY HAS LOOKED, never a claim the entry is fine. A NEW source gets no rows and must satisfy every check.

THE HONEST CEILING, stated beside the discriminating power. This gate proves an entry was CLASSIFIED, never
that the classification is CORRECT. Someone may record `redistributable = true` for a licence that says
otherwise; no gate can read a licence. It removes the silent case (a source nobody checked) and leaves the
judged case (a source someone judged wrongly) to review.

Usage:
  scripts/sources_gate.py               verify; exit non-zero on an unwaived failure
  scripts/sources_gate.py --update      rewrite the baseline from the current tree (deliberate, reviewed)
  scripts/sources_gate.py --self-test   prove each check convicts synthetic bad input
"""

import hashlib
import pathlib
import re
import sys
import tomllib

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "sources" / "registry.toml"
MIRRORED = ROOT / "sources" / "mirrored.toml"
LICENCES = ROOT / "sources" / "licences.toml"
BASELINE = ROOT / "scripts" / "sources_baseline.tsv"

# Where `// @sources:` markers and `source_id =` row references are scanned for. crates/sim is READ ONLY to
# this lane (another agent is moving modules there); reading it for id resolution is fine, editing is not.
MARKER_SCAN_ROOTS = ["crates"]
MANIFEST_GLOB = "crates/*/data/*/manifest.toml"

CHECKS = ["sha256", "witness-receipt", "licence-reason", "archive", "scope", "slim", "licence", "secondary"]

CLASSIFICATIONS = {
    "sha256-unreviewed",
    "archive-unreviewed",
    "scope-unreviewed",
    "slim-unreviewed",
    "licence-unreviewed",
    "secondary-unreviewed",
    "witness-receipt-unreviewed",
    "licence-reason-unreviewed",
}


def read_toml(path):
    with open(path, "rb") as fh:
        return tomllib.load(fh)


def archive_of(block):
    """PR #201 spells it `archive_url`; the older manifests spell it `archived_url`. Accept both, so
    neither lane has to rename on a live branch."""
    return block.get("archived_url") or block.get("archive_url")


def infer_custody(block):
    """Where the bytes are. An explicit `custody` wins; otherwise infer from the holding path, because the
    27 migrated entries predate the field.

    in_repo  a holding path that resolves to real in-tree bytes
    external holding named but the bytes are not in the tree (the ~/.claude/vendored-sources model)
    witness  no holding at all: the sha256 is a re-fetch receipt and the archive is the public witness
    """
    explicit = block.get("custody")
    if explicit:
        return explicit
    holding = block.get("holding")
    if not holding:
        return "witness"
    return "in_repo" if (ROOT / holding).is_file() else "external"


def check_source(block, custody):
    """Every failing check name for one source entry. Pure, so the self-test can drive it directly.

    THE COMPLETENESS PREDICATE BRANCHES ON CUSTODY (owner ruling, 2026-07-18). A source with a public link
    and no redistribution licence is held as CITATION PLUS WITNESS: the citation, the licence finding, the
    public URL, the Wayback witness, the scope, and a checksum where one can be computed without
    redistributing. It does not hold the bytes, and it must not be failed for lacking a checksum OF bytes
    it is forbidden to keep. That would punish the entry for obeying the rule.

    So the receipt requirement splits:
      bytes-held (in_repo, external): a sha256 OF THE HELD BYTES, which is what makes the holding verifiable
                                     offline with no network.
      witness:                       a RESOLVING archive URL and a recorded licence reason. The sha256 is
                                     kept when it can be computed (it stays a valid re-fetch receipt), but
                                     its absence is not the failure.
      neither:                       the failure case. An entry with no held checksum AND no archive
                                     witness is a claim with nothing behind it.
    """
    failures = []

    if custody == "witness":
        # The witness IS the receipt here. A witness with no retrievable archive and no checksum is the
        # "entry with neither" case the ruling names as the failure.
        if not archive_of(block) and not block.get("sha256"):
            failures.append("witness-receipt")
        if not str(block.get("licence", "")).strip():
            failures.append("licence-reason")
    elif not block.get("sha256"):
        failures.append("sha256")

    if not archive_of(block) and not str(block.get("archive_pending", "")).strip():
        failures.append("archive")

    if not str(block.get("scope", "")).strip():
        failures.append("scope")

    # A slim record. Held bytes declare what was kept and dropped; a witness declares the extract that
    # stands in for the bytes it does not hold.
    if custody == "witness":
        if not str(block.get("extract", "")).strip():
            failures.append("slim")
    elif not str(block.get("slim", "")).strip():
        failures.append("slim")

    # THE LICENCE RULE. Only a BYTE-HOLDING entry has to answer it: a witness redistributes nothing, so it
    # needs a citation and a public archive rather than a redistribution right. A holding must either be
    # redistributable itself, or record a free open route that is.
    if custody in ("in_repo", "external"):
        licence_recorded = bool(str(block.get("licence", "")).strip())
        redistributable = block.get("redistributable")
        has_open_alternative = bool(str(block.get("free_route", "")).strip())
        # Two conditions, and the first is the one that keeps this honest. The licence must be RECORDED
        # (the owner's first bullet: state the terms and whether they permit this use), and the entry must
        # then either be redistributable or name a free open route. Requiring only the second would let any
        # restricted bytes be held by pointing at a free-to-READ url, which is the exact conflation between
        # free-to-read and free-to-redistribute that this rule exists to stop.
        if not licence_recorded or (redistributable is not True and not has_open_alternative):
            failures.append("licence")

    return failures


def check_claim(claim):
    """A claim needs a primary AND at least one secondary: two independent witnesses for one number."""
    failures = []
    if not claim.get("primary"):
        failures.append("primary")
    if not claim.get("secondary"):
        failures.append("secondary")
    return failures


def parse_baseline(text):
    """(source_id, check) -> classification."""
    base = {}
    for raw in text.splitlines():
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        parts = raw.split("\t")
        if len(parts) < 3:
            continue
        base[(parts[0].strip(), parts[1].strip())] = parts[2].strip()
    return base


def scan_source_markers(read_lines, roots=None):
    """Every `// @sources: a, b` marker, as (rel_path, lineno, [ids]).

    Scanned by BRACE DEPTH for test modules the way derives_gate.py does, because a latch on the first
    `#[cfg(test)]` leaves every line below it unscanned. That hole was found by live-firing that gate.
    """
    out = []
    for root in roots or MARKER_SCAN_ROOTS:
        base = ROOT / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            rel = str(path.relative_to(ROOT))
            for i, ln in enumerate(read_lines(path), start=1):
                m = re.search(r"//\s*@sources:\s*(.+)$", ln)
                if not m:
                    continue
                ids = [t.strip() for t in re.split(r"[,\s]+", m.group(1)) if t.strip()]
                out.append((rel, i, ids))
    return out


def scan_row_references(read_toml_fn, paths):
    """Every `source_id = "<id>"` on a manifest row, as (rel_path, id).

    THE FIELD IS `source_id`, NOT `source`, and that is deliberate. `source = "..."` is an ESTABLISHED
    convention in this repo's floor data holding a PROSE citation ("Robie & Hemingway 1995, USGS Bulletin
    2131 (298.15 K, 1 bar)"), 196 of them across crates/physics/data/*.toml. None sits inside a scanned
    manifest today, so there is no live collision, but keying the registry reference on `source` would
    plant one: the first prose citation written into a manifest, or the first widening of this scan to the
    floor data files, would convict 196 correct prose citations as unresolvable ids. A distinct field name
    costs nothing and removes the trap.

    Those 196 prose citations are themselves a consolidation target: each names a source that could resolve
    to a registry id instead. That is a follow-on, not this change.
    """
    out = []
    for path in paths:
        try:
            data = read_toml_fn(path)
        except Exception:
            continue
        rel = str(path.relative_to(ROOT))
        for blocks in data.values():
            if not isinstance(blocks, list):
                continue
            for block in blocks:
                if not isinstance(block, dict):
                    continue
                ref = block.get("source_id")
                if isinstance(ref, str) and ref.strip():
                    out.append((rel, ref.strip()))
    return out


def load_entries(read_toml_fn):
    """Both registry halves as one namespace, with the licence overlay merged on.

    The overlay (sources/licences.toml) carries the redistribution finding for each source. It is kept
    apart from the fetch record because the two have different lifetimes: bytes and checksums are immutable
    forever, while a licence finding is dated, revisable, and may be superseded by a permission grant.
    Merged here so a finding in the overlay is exactly as enforceable as a field written inline.
    """
    entries = []
    problems = []
    seen = {}
    for path, label in ((REGISTRY, "registry"), (MIRRORED, "mirrored")):
        if not path.exists():
            continue
        data = read_toml_fn(path)
        for block in data.get("source", []):
            sid = block.get("id")
            if not sid:
                problems.append(f"{label}: a source block has no `id`")
                continue
            if sid in seen:
                problems.append(
                    f"duplicate source id '{sid}' in both {seen[sid]} and {label} "
                    "(an id lives in exactly one half)"
                )
                continue
            seen[sid] = label
            entries.append((sid, block))

    if LICENCES.exists():
        overlay = read_toml_fn(LICENCES).get("licence", [])
        by_id = {sid: block for sid, block in entries}
        for record in overlay:
            oid = record.get("id")
            if not oid:
                problems.append("licences overlay: a record has no `id`")
                continue
            target = by_id.get(oid)
            if target is None:
                # A stale overlay is a wrong licence claim about a source that no longer exists, which is
                # worse than none: a reader would trust it.
                problems.append(
                    f"licences overlay: '{oid}' matches no source in the registry (stale finding)"
                )
                continue
            for key, value in record.items():
                if key != "id":
                    target[key] = value
    return entries, problems


def check_duplicate_documents(entries):
    """The SAME document carried under two ids, detected by identical sha256.

    WHY THIS IS NOT COSMETIC. The per-directory manifests cross-reference a sibling's bytes by COPYING the
    record (the gruneisen manifest's `dir` idiom), so one document can appear as `zha_1996` and again as
    `gruneisen.zha_1996_forsterite`. A licence finding recorded against one id then does not reach the
    other, and that is exactly what happened here: the AGU finding landed on `zha_1996` while its twin
    carried nothing. A registry whose facts do not propagate to every copy of the same document will report
    a source as cleared while an identical copy of it sits un-cleared.

    Returns [(sha256, [ids], findings_disagree)] so the caller can report the disagreeing ones loudest.
    """
    by_sha = {}
    for sid, block in entries:
        sha = block.get("sha256")
        if sha and len(str(sha)) == 64:
            by_sha.setdefault(sha, []).append((sid, block))
    out = []
    for sha, group in sorted(by_sha.items()):
        if len(group) < 2:
            continue
        findings = {b.get("redistributable") for _, b in group}
        licences = {bool(str(b.get("licence", "")).strip()) for _, b in group}
        disagree = len(findings) > 1 or len(licences) > 1
        out.append((sha, [sid for sid, _ in group], disagree))
    return out


def check_collection_coverage(tracked_data_files):
    """A vendored data COLLECTION must have a source record.

    THE HOLE THIS CLOSES, found by counting rather than by reading: 89 tracked data files (janaf 34,
    optical_constants_aesopus 45, oxide_thermochemistry 10) are vendored bytes with per-row citations and
    NO `[[source]]` block at all, so the source registry does not cover them and no licence question was
    ever asked of them. That is exactly where the largest licence problem in the tree turned out to sit:
    the JANAF tables, whose terms were never captured at fetch time at all. (An earlier draft of this
    comment called them copyrighted by statute under 15 USC 290e. That was wrong: the Act AUTHORIZES the
    Secretary to secure copyright in Standard Reference Data, which is not evidence any was secured over
    these tables. The entry is UNRESOLVED, not restricted.)

    A per-row md5 proves a file was not corrupted. It does not say who may redistribute it, what regime it
    holds in, or what was dropped when it was slimmed. Those are source-level questions and they need a
    source-level record.
    """
    return [d for d, (n_files, n_sources) in sorted(tracked_data_files.items()) if n_files and not n_sources]


def verify_holdings(entries):
    """A holding that claims in-tree bytes must have those bytes, with a matching checksum."""
    problems = []
    for sid, block in entries:
        holding = block.get("holding")
        if not holding:
            continue
        path = ROOT / holding
        if not path.is_file():
            continue  # `external` custody: bytes held outside the tree, checked by its archive instead
        declared = block.get("sha256")
        if not declared:
            continue  # already convicted by the sha256 check
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != declared:
            problems.append(
                f"{sid}: holding '{holding}' has sha256 {actual[:16]}... but the entry declares "
                f"{declared[:16]}... (the bytes and the receipt disagree)"
            )
    return problems


def run_checks(entries, baseline, claims, markers, row_refs):
    """(unwaived_failures, waived_count, unresolved_ids). Pure over its inputs."""
    unwaived = []
    waived = 0
    for sid, block in entries:
        custody = infer_custody(block)
        for check in check_source(block, custody):
            if (sid, check) in baseline:
                waived += 1
            else:
                unwaived.append((sid, check))

    for label, claim in claims:
        for failure in check_claim(claim):
            cid = claim.get("id", "(unnamed claim)")
            key = (f"{label}:{cid}", "secondary")
            if key in baseline:
                waived += 1
            else:
                unwaived.append((f"{label}:{cid}", f"claim-{failure}"))

    known = {sid for sid, _ in entries}
    unresolved = []
    for rel, lineno, ids in markers:
        for i in ids:
            if i not in known:
                unresolved.append(f"{rel}:{lineno}: @sources id '{i}' is not in the registry")
    for rel, ref in row_refs:
        if ref not in known:
            unresolved.append(f"{rel}: row `source_id = \"{ref}\"` is not in the registry")
    return unwaived, waived, unresolved


def gather(read_toml_fn=read_toml, read_lines=None):
    if read_lines is None:

        def read_lines(path):
            with open(path, encoding="utf-8", errors="replace") as fh:
                return fh.readlines()

    entries, problems = load_entries(read_toml_fn)
    manifest_paths = sorted(ROOT.glob(MANIFEST_GLOB))
    claims = []
    for path in manifest_paths:
        try:
            data = read_toml_fn(path)
        except Exception:
            continue
        rel = str(path.relative_to(ROOT))
        for claim in data.get("claim", []):
            claims.append((rel, claim))
    markers = scan_source_markers(read_lines)
    row_refs = scan_row_references(read_toml_fn, manifest_paths)

    # Collection coverage: per vendored data directory, how many data files are tracked by git against how
    # many `[[source]]` records the manifest declares. Uses the checked-in file list rather than the
    # working tree, so a stray local download cannot make a directory look covered or uncovered.
    collections = {}
    for path in manifest_paths:
        if path.parent.name in ("disk_arc_literature",):
            continue
        try:
            data = read_toml_fn(path)
        except Exception:
            continue
        n_sources = len(data.get("source", []))
        n_files = sum(
            1
            for f in path.parent.iterdir()
            if f.is_file() and f.name != "manifest.toml" and not f.name.startswith(".")
        )
        collections[path.parent.name] = (n_files, n_sources)

    return entries, problems, claims, markers, row_refs, collections


def render_baseline(entries, uncovered=()):
    lines = [
        "# The sources-gate baseline (scripts/sources_gate.py).",
        "#",
        "# One row per (source, check) that the pre-existing vendored population does not yet satisfy, so",
        "# the gate RATCHETS FORWARD instead of blocking every commit until someone audits all 27 sources",
        "# at once. Exactly the shape scripts/derives_baseline.tsv and scripts/constructor_baseline.tsv use.",
        "#",
        "# A ROW IS AN ADMISSION THAT NOBODY HAS LOOKED. It is not a claim that the entry is fine, and it is",
        "# not permission to add another. A NEW source gets no rows and must satisfy every check: sha256, an",
        "# archive record, a scope, a slim record, a licence answer if it holds bytes, and for a claim a",
        "# secondary witness beside the primary.",
        "#",
        "# Resolve a row by fixing the entry, then deleting the row. The gate reports a row whose source is",
        "# gone, so the baseline cannot rot.",
        "#",
        "# source_id\tcheck\tclassification\treason",
    ]
    for sid, block in entries:
        custody = infer_custody(block)
        reviewed = block.get("reviewed")
        restricted = block.get("redistributable") is False
        for check in check_source(block, custody):
            # A row must say which of the two it is. `-unreviewed` means nobody has looked. A REVIEWED row
            # means somebody looked, found a defect, and it is waived pending a ruling: that must read as a
            # known defect rather than an unexamined one, or the waiver buries the finding it records.
            if check == "licence" and restricted:
                lines.append(
                    f"{sid}\tlicence\tlicence-DEFECT-owner-ruling\t"
                    f"REVIEWED {reviewed}: recorded NOT redistributable while bytes are held ({custody}). "
                    "Waived pending owner ruling D6, not because nobody looked"
                )
            else:
                lines.append(
                    f"{sid}\t{check}\t{check}-unreviewed\t"
                    f"predates the sources gate; {custody} custody; classify when next touched"
                )
    for directory in uncovered:
        lines.append(
            f"collection:{directory}\tcoverage\tcoverage-unreviewed\t"
            "vendored data files with no [[source]] record; needs a collection-level source entry"
        )
    return "\n".join(lines) + "\n"


def self_test():
    """Prove EVERY check convicts. A gate whose failure paths were never fired is a comfort, not a gate."""
    ok_block = {
        "id": "good",
        "sha256": "a" * 64,
        "archived_url": "https://web.archive.org/web/x",
        "scope": "300 K, 1 bar",
        "slim": "pages 3-5 kept, rest dropped",
        "licence": "CC-BY 4.0",
        "redistributable": True,
        "holding": "some/path.pdf",
    }
    assert check_source(ok_block, "in_repo") == [], "a complete entry must pass"

    def without(key):
        b = dict(ok_block)
        b.pop(key)
        return b

    assert check_source(without("sha256"), "in_repo") == ["sha256"], "missing sha256 must convict"
    assert check_source(without("archived_url"), "in_repo") == ["archive"], "missing archive must convict"
    assert check_source(without("scope"), "in_repo") == ["scope"], "missing scope must convict"
    assert check_source(without("slim"), "in_repo") == ["slim"], "missing slim must convict"

    # md5 alone is NOT a receipt.
    md5_only = dict(ok_block)
    md5_only.pop("sha256")
    md5_only["md5"] = "b" * 32
    assert "sha256" in check_source(md5_only, "in_repo"), "an md5-only entry must still convict on sha256"

    # THE LICENCE RULE, both directions.
    no_licence = without("redistributable")
    assert "licence" in check_source(no_licence, "in_repo"), (
        "a byte-holding with no redistribution answer must convict"
    )
    assert "licence" in check_source(no_licence, "external"), (
        "an external byte-holding must answer the licence question too"
    )
    assert "licence" not in check_source(no_licence, "witness"), (
        "a witness redistributes nothing, so it must NOT be convicted on licence"
    )
    open_route = dict(no_licence)
    open_route["free_route"] = "https://arxiv.org/abs/1234.5678"
    assert "licence" not in check_source(open_route, "in_repo"), (
        "a recorded free open route satisfies the licence rule"
    )
    # An unrecorded licence convicts even when a free route is named: free-to-read is not free-to-redistribute.
    route_but_no_licence = dict(open_route)
    route_but_no_licence.pop("licence")
    assert "licence" in check_source(route_but_no_licence, "in_repo"), (
        "a free route must NOT excuse an unrecorded licence"
    )
    not_redistributable = dict(ok_block)
    not_redistributable["redistributable"] = False
    assert "licence" in check_source(not_redistributable, "in_repo"), (
        "an explicitly non-redistributable byte-holding must convict"
    )

    # THE CUSTODY BRANCH (owner ruling). A witness must NOT be failed for lacking a held-bytes checksum,
    # and must be failed when it has neither a checksum nor an archive.
    wit_ok = {
        "id": "w",
        "archived_url": "https://web.archive.org/web/x",
        "scope": "300 K",
        "extract": "the passage",
        "licence": "all rights reserved; free to read only",
    }
    assert check_source(wit_ok, "witness") == [], (
        "a citation-plus-witness entry with an archive and a licence reason must PASS with no held sha256"
    )
    naked = {k: v for k, v in wit_ok.items() if k != "archived_url"}
    got = check_source(naked, "witness")
    assert "witness-receipt" in got and "archive" in got, (
        f"a witness with neither checksum nor archive is the failure case, got {got}"
    )
    with_sha = dict(naked)
    with_sha["sha256"] = "e" * 64
    assert "witness-receipt" not in check_source(with_sha, "witness"), (
        "a witness carrying a re-fetch receipt satisfies the receipt requirement"
    )
    no_reason = {k: v for k, v in wit_ok.items() if k != "licence"}
    assert "licence-reason" in check_source(no_reason, "witness"), (
        "a witness must record WHY it holds no bytes"
    )
    # A bytes-held entry still needs its held-bytes checksum: the branch must not weaken that side.
    assert "sha256" in check_source(without("sha256"), "in_repo"), (
        "the custody branch must not excuse a byte-holding from its own receipt"
    )

    # A witness needs an extract as its slim record.
    witness = {k: v for k, v in ok_block.items() if k not in ("slim", "holding")}
    assert check_source(witness, "witness") == ["slim"], "a witness with no extract must convict"
    witness["extract"] = "the load-bearing passage"
    assert check_source(witness, "witness") == [], "a witness with an extract must pass"

    # `archive_pending` with a reason is the honest form of a missing archive.
    pending = without("archived_url")
    pending["archive_pending"] = "host bot-walls the crawler"
    assert "archive" not in check_source(pending, "in_repo"), "a stated archive_pending must pass"
    empty_pending = without("archived_url")
    empty_pending["archive_pending"] = "   "
    assert "archive" in check_source(empty_pending, "in_repo"), (
        "an EMPTY archive_pending must not buy a pass"
    )

    # PR #201's spelling must be accepted.
    pr201 = without("archived_url")
    pr201["archive_url"] = "https://web.archive.org/web/y"
    assert "archive" not in check_source(pr201, "in_repo"), "`archive_url` must be accepted as an alias"

    # CLAIMS: a primary with no secondary is the defect the rule exists for.
    assert check_claim({"primary": ["a"], "secondary": ["b"]}) == [], "a two-witness claim passes"
    assert check_claim({"primary": ["a"]}) == ["secondary"], "a primary with no secondary must convict"
    assert check_claim({"secondary": ["b"]}) == ["primary"], "a claim with no primary must convict"

    # THE HOP: an unresolvable marker id must be caught, from a marker AND from a manifest row.
    markers = [("a.rs", 1, ["known_source", "ghost_source"])]
    _, _, unresolved = run_checks(
        [("known_source", ok_block)], {}, [], markers, [("m.toml", "another_ghost")]
    )
    assert len(unresolved) == 2, f"both unresolvable ids must be reported, got {unresolved}"
    assert any("ghost_source" in u for u in unresolved), "the marker ghost must be named"
    assert any("another_ghost" in u for u in unresolved), "the row ghost must be named"

    # The marker regex itself, against a real line shape.
    line = "    // @sources: gruneisen.ahrens_1995_handbook, zha_1996\n"
    m = re.search(r"//\s*@sources:\s*(.+)$", line)
    assert m, "the marker regex must match an indented marker"
    ids = [t.strip() for t in re.split(r"[,\s]+", m.group(1)) if t.strip()]
    assert ids == ["gruneisen.ahrens_1995_handbook", "zha_1996"], f"id split: {ids}"

    # WAIVERS: a baselined failure is waived, an unbaselined one is not.
    bad = ("bad", without("sha256"))
    unwaived, waived, _ = run_checks([bad], {("bad", "sha256"): "sha256-unreviewed"}, [], [], [])
    assert unwaived == [] and waived == 1, f"a baselined failure must be waived: {unwaived} {waived}"
    unwaived2, _, _ = run_checks([bad], {}, [], [], [])
    assert unwaived2 == [("bad", "sha256")], f"an unbaselined failure must convict: {unwaived2}"

    print("sources gate: self-test OK (every check fired)")
    return 0


def main():
    args = sys.argv[1:]
    if "--self-test" in args:
        return self_test()

    if not REGISTRY.exists() and not MIRRORED.exists():
        print(
            "sources gate: no registry. Run scripts/gen_sources.py once, review the file, and commit it.",
            file=sys.stderr,
        )
        return 2

    entries, problems, claims, markers, row_refs, collections = gather()
    uncovered = check_collection_coverage(collections)

    if "--update" in args:
        BASELINE.write_text(render_baseline(entries, uncovered), encoding="utf-8")
        rows = sum(1 for ln in BASELINE.read_text().splitlines() if ln and not ln.startswith("#"))
        print(f"sources gate: baseline rewritten, {len(entries)} source(s), {rows} waived check(s)")
        return 0

    if not BASELINE.exists():
        print(
            "sources gate: no baseline. Run scripts/sources_gate.py --update once, review it, and commit.",
            file=sys.stderr,
        )
        return 2

    baseline = parse_baseline(BASELINE.read_text(encoding="utf-8"))
    unwaived, waived, unresolved = run_checks(entries, baseline, claims, markers, row_refs)
    problems += verify_holdings(entries)

    for directory in uncovered:
        if (f"collection:{directory}", "coverage") in baseline:
            waived += 1
        else:
            unwaived.append((f"collection:{directory}", "coverage"))

    # THE LOUD LINE. A source recorded as NOT redistributable whose bytes are nonetheless held is a real
    # defect, and it is currently WAIVED by the baseline pending the owner's ruling. A waiver must never
    # make a known defect invisible, so it is reported every run whether or not the gate passes. This is
    # the difference between grandfathering (nobody has looked) and burying (somebody looked, found a
    # problem, and the gate went quiet).
    held_but_restricted = [
        sid
        for sid, block in entries
        if block.get("redistributable") is False and infer_custody(block) == "in_repo"
    ]

    live = {sid for sid, _ in entries}
    stale = sorted({sid for (sid, _), _ in baseline.items() if sid not in live and ":" not in sid})

    ok = True
    if problems:
        ok = False
        print(f"sources gate: FAILED. {len(problems)} registry integrity problem(s):", file=sys.stderr)
        for p in problems[:25]:
            print(f"  {p}", file=sys.stderr)
    if unresolved:
        ok = False
        print(
            f"sources gate: FAILED. {len(unresolved)} source reference(s) do not resolve in the registry:",
            file=sys.stderr,
        )
        for u in unresolved[:25]:
            print(f"  {u}", file=sys.stderr)
        print(
            "\nThe id must name a `[[source]]` in sources/registry.toml or sources/mirrored.toml. This is "
            "THE HOP: it is what lets a walk from a derived quantity reach a held, checksummed, archived "
            "primary.",
            file=sys.stderr,
        )
    if unwaived:
        ok = False
        print(
            f"sources gate: FAILED. {len(unwaived)} unwaived check failure(s) on vendored source(s):",
            file=sys.stderr,
        )
        for sid, check in unwaived[:25]:
            print(f"  {sid}: {check}", file=sys.stderr)
        print(
            "\nA new source must carry: sha256 (the receipt), an archive URL or a stated archive_pending "
            "(a link rots), a scope (the regime its values hold in), a slim record (`slim` for held bytes, "
            "`extract` for a witness), and if it HOLDS bytes an answer to the licence question "
            "(`redistributable = true`, or a `free_route` to an openly licensed copy). A claim needs a "
            "primary AND a secondary, so one number has two independent witnesses.\n"
            "Grandfathering an existing entry is scripts/sources_baseline.tsv; a NEW source may not take a "
            "row.",
            file=sys.stderr,
        )
    if stale:
        ok = False
        print(
            f"sources gate: FAILED. {len(stale)} baseline row(s) name a source that no longer exists:",
            file=sys.stderr,
        )
        for sid in stale[:25]:
            print(f"  {sid}", file=sys.stderr)

    dupes = check_duplicate_documents(entries)
    disagreeing = [(s, ids) for s, ids, dis in dupes if dis]
    if dupes:
        print(
            f"sources gate: NOTICE. {len(dupes)} document(s) are carried under more than one id "
            "(identical sha256), so a licence finding on one id does not reach its twin:"
        )
        for sha, ids, dis in dupes:
            flag = "  <-- LICENCE FINDINGS DISAGREE" if dis else ""
            print(f"  {' == '.join(ids)}{flag}")

    if held_but_restricted:
        print(
            f"sources gate: NOTICE. {len(held_but_restricted)} source(s) are recorded as NOT "
            "redistributable while their bytes are held in-tree. Waived pending the owner's ruling "
            "(FETCH_PIPELINE_PLAN.md D6), reported every run so a waiver never hides a known defect:"
        )
        for sid in held_but_restricted:
            print(f"  {sid}")

    if not ok:
        return 1

    print(
        f"sources gate: clean ({len(entries)} source(s), {len(claims)} claim(s), "
        f"{len(markers)} @sources marker(s), {waived} waived check(s) in the baseline)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
