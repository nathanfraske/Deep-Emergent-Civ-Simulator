# The consolidated fetch and vendoring pipeline (plan, then the build)

The project vendors sources well and vendors them in five different shapes. This plan consolidates the
shapes into one registry, states the licence and repo-size policies that decide whether bytes may be held
at all, designs the two missing hops that connect a derived quantity to the primary it rests on, and gates
the result so the discipline survives the agent who wrote it.

Written 2026-07-18. Scope: docs, scripts and data. No `crates/sim` edits (another agent is moving modules
there) and no edits to `crates/physics/data/disk_arc_literature/` (in flight on PR #201, read only).

---

## 1. What exists today, measured

Counted rather than remembered, because the design depends on the split.

**The manifests.** 167 TOML blocks across 18 `manifest.toml` files under `crates/physics/data/*/`. That
number is four different things, and conflating them is the first thing to fix:

| Block kind | Count | What it is |
| --- | --- | --- |
| `[[species]]` | 89 | cited data rows (janaf 34, optical_constants_aesopus 45, oxide_thermochemistry 10) |
| `[[value]]` | 40 | a value read out of a source, with its page or figure anchor |
| `[[source]]` | 27 | an actual vendored artifact: bytes, checksum, citation |
| `[[grid]]` | 11 | grid axis definitions (aesopus_lowt) |

So there are **27 sources** and **140 rows that cite a source**. The registry has to hold those as two
different kinds of record, because a source has custody and a checksum while a row has an anchor and a
scope.

**Field coverage across the 27 sources**, which is where the gaps are:

| Field | Present | Missing |
| --- | --- | --- |
| `sha256` | 27 | 0 |
| `archived_url` | 13 | 14 |
| `scope` | 5 | 22 |
| slim record | 0 | 27 |
| licence or terms | 0 | 27 |
| free-route record | 0 | 27 |

`md5` appears 92 times as the legacy receipt on the `[[species]]` rows, superseded by sha256 as primary
from 2026-07-17 and retained as a secondary.

**The other four homes.** `docs/working/VENDORING_CHECKLIST.md` holds ONE entry, `flexure_tafi`, in prose.
It is the fullest-discipline record in the repo and the model this plan generalizes: it carries the
licence posture, both checksums (full fetch and held slim), the recipe, the archived URL, an explicit
kept-versus-dropped slim record, the block kind, the grade, and the read channels. `crates/physics/data/disk_arc_literature/manifest.toml`
on PR #201 holds 31 `[[source]]` entries in a consistent schema of its own. `docs/design.md` Part 63 is
the prose bibliography, 303 lines. `~/.claude/vendored-sources/` holds 28 MB of bytes for 5 sources
outside the repo entirely.

**The bytes.** 45.4 MB of documents in-tree across 10 files, against a 226 MB `.git`. Two files are 81%
of it: `robie_hemingway_1995/report.pdf` at 20.76 MB and `wachtman_1960_corundum/jresv64An3p213_corundum.pdf`
at 16.20 MB. Neither is slimmed. The other eight total 8.4 MB and six of those are under 1 MB.

**Schema conflicts to reconcile, found by reading both sides:**

- PR #201 spells it `archive_url`; the existing manifests spell it `archived_url`. The registry must
  accept both or the in-flight work breaks on landing.
- PR #201 carries `extract` (the load-bearing passage, verbatim) and `used_by` (the code that consumes
  it). Neither exists in the older manifests, and both are good: `extract` is a slim record for a source
  whose bytes are not held, and `used_by` is the marker-to-source edge already in embryo.
- PR #201 carries `archive_pending` with a stated reason on 2 entries whose hosts served a bot-wall to
  the Wayback crawler. That is the honest form of a missing archive and the registry should keep it
  rather than force a false URL.

---

## 2. The consolidation: one registry, two record kinds

**One place, correctly read.** The instinct is a single file holding all 167 entries. That is the wrong
move and the data says so: the `[[species]]` rows ARE the loaders' column index and they sit beside the
bytes they index, and PR #201 deliberately landed a separate directory to avoid colliding with the
physics lane. Collapsing them would break loaders and clobber in-flight work.

"One place" means one place to look a source up and one place citations live. The split that delivers it
is the split the data already wants:

**`sources/registry.toml`** at the repo root is THE source registry. One `[[source]]` block per vendored
artifact, each with a stable `id`. It is project-wide by position, a sibling of `calibration/reserved.toml`,
because sources are cited from physics, bio and world alike and none of those should own the others'
citations.

**Per-directory manifests keep their rows** and each row gains `source_id = "<id>"` pointing into the
registry. The rows keep their anchors, their scopes and their loader semantics. Nothing moves, nothing is
rewritten, and every existing citation, checksum, archive URL and scope survives verbatim because the
migration is additive.

**The bibliography is generated, never hand-kept.** `scripts/gen_sources.py` renders `docs/SOURCES.md`
from the registry, so the centralized bibliography with proper citations cannot drift from the held
bytes. It follows `gen_floor_registry.py`, which the stop-gate already keeps never-stale by regenerating
to a temp and diffing.

### The registry entry

```toml
[[source]]
id = "byerlee_1978"                 # stable, referenced from rows, markers and the floor register
citation = "Byerlee, J., 1978, Friction of rocks, Pure and Applied Geophysics 116, 615-626, DOI 10.1007/BF00876528"
url = "https://earthquake.usgs.gov/static/lfs/research/rockphysics/Friction_of_rocks.pdf"
archived_url = "https://web.archive.org/web/.../..."   # `archive_url` accepted as an alias
sha256 = "995adf14a816e517bd037936d45fa4237e312c0aa1f278781525bd6b202d2306"
bytes = 459016
recipe = "GET the open USGS copy"
scope = "brittle crust, normal stress 0 to 2 GPa, room temperature"
grade = "measured"

# CUSTODY: where the bytes are, which the licence decides (section 3)
custody = "in_repo"                 # in_repo | witness | external
holding = "crates/physics/data/byerlee_1978/Friction_of_rocks.pdf"

# LICENCE: the redistribution question, answered rather than assumed
licence = "US Government work (17 USC 105), USGS-authored"
redistributable = true              # may THIS project redistribute the bytes
licence_evidence = "https://www.usgs.gov/information-policies-and-instructions/copyrights-and-credits"
free_route = "https://earthquake.usgs.gov/static/lfs/research/rockphysics/Friction_of_rocks.pdf"
free_route_kind = "publisher-open"  # publisher-open | preprint | author-copy | repository | gov-work

# SLIM: what was kept and what was dropped, so a slim is never mistaken for the whole document
slim = "full document held, 10 pages, 0.44 MB, under the per-file cap"
```

A source whose bytes are not held carries `custody = "witness"`, no `holding`, and the `extract` field
instead: the load-bearing passage verbatim, which is what PR #201 already does and is a legitimate slim
when the bytes may not be redistributed.

### The claim record

The owner's rule 5 is that a fetch covers a PRIMARY and at least one SECONDARY for the same claim, both
through the same pipeline, so they cross-check and so an erratum has two witnesses. That needs a record
of the claim itself, which today exists only as the `[[value]]` rows' informal shape:

```toml
[[claim]]
id = "friction.high_stress_coefficient"
quantity = "0.6"
primary = ["byerlee_1978"]
secondary = ["kohlstedt_1995"]
anchor = "p.621 sec.5, tau = 50 + 0.6 sigma_n above 200 MPa"
channel = "T"                       # T text, F figure, TF dual-channel agreement
scope = "normal stress above 200 MPa"
```

Claims live beside their values in the per-directory manifests, not in the central registry, because a
claim is about a value and the value's file is where a reader looks. The registry holds sources; the
manifests hold claims; the gate enforces that every claim's source ids resolve.

---

## 3. The licence policy (rule 1, as corrected)

The test is not whether a source is paywalled. The test is **whether the licence permits this use**: an
open-source, educational, non-profit project redistributing the bytes. A gold open-access paper sitting
behind a publisher page while carrying CC-BY is fine to vendor. A free-to-download paper whose terms
reserve all rights is not, however easy the bytes are to obtain.

Free-to-read and free-to-redistribute are different questions and the registry asks both. Each entry
records the licence or terms, whether they permit redistribution for this use, the evidence URL for that
finding, and whether a free route to the bytes exists and of what kind. Recording "we checked the licence
and it permits this" is itself provenance worth holding, so the fields are required even when the answer
is yes.

**The three custody classes** follow from the answer:

- **`in_repo`**: the licence permits redistribution and the bytes fit the size policy. Bytes in-tree,
  sha256 verifiable offline with no network. This is the target state for rule 4 and the strongest form
  of provenance, because the DAG's leaves are in-tree.
- **`witness`**: the licence does not permit redistribution, or the bytes cannot be slimmed under the
  cap. No bytes held. The sha256 is a re-fetch receipt, the archived URL is the public witness, and the
  `extract` carries the load-bearing passage. This is PR #201's model and it is the correct handling for
  a restrictively-licensed source.
- **`external`**: bytes in local custody outside the repo (`~/.claude/vendored-sources/`), sha256 plus a
  retrievable archive snapshot. The current gruneisen and flexure_tafi model. This class is a way station:
  every `external` entry is a candidate to become `in_repo` once slimmed, or `witness` if the licence
  says no.

### What the licence review found

A licence pass ran against the publishers' own terms, with a quoted term and an evidence URL per source.
Every locally-checkable claim it made was re-verified against this tree before being recorded: the
ten-tracked-PDF list, the absence of any tracked `.djvu`, the 34 JANAF tables, and the repo's Apache-2.0
licence all confirmed. The legal reading itself is not verified by this agent and is not a lawyer's
opinion, so the remediation is an owner ruling (D6) and nothing has been deleted. The findings are
recorded as data in `sources/licences.toml`, merged by the gate and shown in the bibliography.

**The Goldsmith 2001 case, settled.** The article carries "© 2001. The American Astronomical Society. All
rights reserved." and IOP labels it "Free article". The AAS 2022 open-access transition made pre-2022 back
content free to READ without relicensing it: CC-BY applies only to articles accepted after 11 October 2021,
and for older articles "permission to reuse content accepted before that date will still be required". No
arXiv preprint exists, established four independent ways (an exact-phrase API search, two author-and-title
searches, and ADS's own eprint gateway resolving to the publisher PDF rather than to arXiv), which is
notable because Goldsmith posted five other papers to arXiv in 2000 to 2002. His 2001 affiliation was
Cornell, so no government-work argument applies. **The correct handling is citation plus public witness,
never a byte vendoring**: free to read, all rights reserved, no open route in existence. Under the old rule
it should not have been byte-vendored, and under the licence rule it still should not be, for a sharper
reason: the absence of a permitting licence rather than the presence of a paywall.

IOP states the governing distinction in its own words, which is worth carrying as the pipeline's motto:
**"Free to view is not the same as free to reuse."** A generalization that matters for every future fetch:
IOP instructs authors uploading to arXiv to "select the 'non-exclusive licence to distribute' and not an
open access or Creative Commons Licence", so an arXiv copy of an IOP paper normally carries a licence that
lets arXiv distribute and does not grant redistribution rights. **An arXiv link is not automatically an
open licence.** The per-submission licence has to be checked each time, and the registry records which
kind it is.

**The existing holdings, triaged.** Of the ten tracked PDFs, four are clear and six are not:

| Source | Finding | Basis |
| --- | --- | --- |
| `byerlee_1978` | redistributable | US Government work, 17 USC 105, USGS-authored |
| `robie_hemingway_1995` | redistributable | USGS Bulletin 2131, public domain |
| `wachtman_1960_corundum` | redistributable | J. Res. NBS "not subject to copyright in the U.S." |
| `mit_ocw_12108` | redistributable, CONFLICT | CC BY-NC-SA 4.0 against this repo's Apache-2.0 |
| `fan_2019` | NOT redistributable | MSA, "open access does not mean un-copyrighted" |
| `jackson_1999_enstatite` | NOT redistributable | MSA; a better open route exists at MSA's own archive |
| `speziale_2004` | NOT redistributable | AGU, permission "does not extend to public posting" |
| `zha_1996` | NOT redistributable | AGU, same term |
| `yoneda_1990` | NOT redistributable | J-STAGE, approval required |
| `heyliger_2003_quartz` | NOT redistributable | "© 2003 Acoustical Society of America" |

Two more findings sit outside that table. **The JANAF entry is NOT ESTABLISHED and is corrected here
(owner challenge, 2026-07-19).** The original wording called it a surprise: 34 tracked files whose licence
is not the usual NIST public domain, on the ground that the Standard Reference Data Act (15 USC 290e)
LETS Commerce secure copyright in Standard Reference Data compilations and JANAF is SRD 13.

That inferred a restriction from the mere EXISTENCE OF THE AUTHORITY to restrict. It is not the same
claim, and the difference is the whole finding. The statute permitting NIST to secure copyright is not
evidence that NIST did so for these tables, and the owner reports finding nothing stating anything other
than the NIST open licence. Our own manifest captured no terms at fetch time: it records 34
`janaf.nist.gov` URLs and no licence statement, so there is no local evidence of a restriction either.

The burden runs the other way. A restrictive finding needs an ASSERTION to point at, not an authority that
was never shown to be exercised. So this entry stands as UNRESOLVED, not as not-redistributable, and it
carries no remediation: nothing is converted, nothing is deleted, and the tables remain as they are.
Closing it needs one checkable thing, the actual terms on the NIST distribution page captured and held the
way any other licence finding is. Until then the honest record is "no terms captured, no assertion found".

Kept because it remains true and useful: individual numeric values are facts and uncopyrightable, so
citing values is safe regardless of how the compilation is licensed. And the
**geokniga** source behind the Gruneisen handbook is a shadow library that disclaims all rights in its own
words. No bytes were ever committed, verified, so there is nothing to take down; the defect is that the
manifest calls it "the open GeoKniga DjVu", and labelling a shadow library open is a false provenance claim
a later reader would rely on.

**Remediation is a ratchet, not a stop-the-world audit**, and the baseline distinguishes the two states
that matter. A `-unreviewed` row means nobody has looked. A `licence-DEFECT-owner-ruling` row means
somebody looked, found a defect, and it is waived pending a ruling. The gate additionally prints the
restricted-but-held set on every run whether or not it passes, so a waiver can never make a known defect
invisible, which is the failure mode grandfathering invites.

### The ruling, and what it changed (owner, 2026-07-18)

**A source with a public link and no redistribution licence is held as citation plus witness**: the
citation, the licence finding, the public URL, the Wayback witness, the scope, and a checksum where one
can be computed without redistributing. It does not hold the bytes. This confirmed what was proposed for
Goldsmith 2001 and extended it to the other five.

**Six entries converted, and the conversion cost nothing.** Before removing any bytes, each source's
Wayback capture was re-fetched and hashed against the receipt on record. **Five of six were byte-identical
to the bytes we held.** The sixth, `jackson_1999_enstatite`, had no capture at all, so one was requested
from the Wayback save endpoint, then re-fetched and hashed, and it too came back byte-identical. So every
converted entry now points at a public artifact whose bytes match its retained sha256: the receipt still
verifies, the provenance is unbroken, and what was lost by deleting the local copies is nothing beyond the
copies themselves. That verification is recorded per entry in `witness_verified`.

A caution on method, because it nearly produced a false finding. The first hash pass used Python's urllib
and reported all six as DIFFERING. Those were 10 to 12 KB interstitial pages rather than the documents;
`curl -L` with a browser user-agent returned the real PDFs at the correct sizes. The differing verdict was
an artifact of the fetch, not a fact about the archive, and reporting it would have been wrong.

**The gate's completeness predicate now branches on custody**, which the ruling requires: a witness must
not be failed for lacking a checksum of bytes it is forbidden to keep. A bytes-held entry needs a
held-bytes sha256; a witness needs a resolving archive URL and a recorded licence reason; an entry with
neither is the failure case. Both directions are covered in the self-test.

**JANAF cannot be converted as a data change, and that is the finding.** The 34 tables are not inert
document witnesses like the six PDFs: they are `include_str!`d into the physics crate at 35 sites in
`crates/physics/src/janaf.rs`. Deleting them is a compile error rather than a conversion, and they are
load-bearing for the condensation and thermochemistry work. The legal nuance offers the path, since values
are uncopyrightable facts and the compilation is what is restricted: replace the verbatim NIST files with a
derived data file carrying only the numeric columns the loader consumes, cited to Chase 1998 with the
statute noted. That is an engineering arc against a buildable tree with pins verified either side, because
any transcription difference moves them. The cheaper route the statute itself names is a written permission
request to NIST, which settles it with no code change. The entry is recorded in `sources/registry.toml`
rather than in the JANAF manifest, precisely because that manifest is a compiled input and this branch
cannot verify pins. It remains the one source the gate reports as restricted-but-held.

**A hole the conversion exposed.** Removing the bytes broke three cross-references in the Gruneisen
manifest, which reaches sibling directories' bytes by copying the record. The generator now emits a
`holding` claim only when the bytes are present, so those corrected themselves and no entry claims
a file that is gone. Underneath sat a real defect: **six documents are carried under two ids each**
(identical sha256), and the AGU and MSA findings had landed on one id of each pair and not the other, so an
identical copy read as un-reviewed. The gate now detects same-checksum-different-id and flags the pairs
whose findings disagree; the three disagreeing pairs were closed, and the duplication itself is D8.

---

## 4. The repo-size policy (rule 4)

Vendoring bytes into the repo is right, because a provenance DAG whose leaves are outside the tree cannot
be verified by a checkout. The consequence is measurable and must be bounded: 45.4 MB of documents against
a 226 MB `.git`, with 81% of it in two un-slimmed files.

**The policy.** Slim first, then hold. A document is slimmed to the pages that carry the load-bearing
content before it is vendored, and the entry records what was kept and what was dropped, exactly as
`flexure_tafi` does (2.21 MB and 11 pages down to 0.90 MB and 4 pages). A slimmed document under the
per-file cap is held `in_repo`. One that cannot be slimmed under the cap becomes `witness` or `external`,
with the archived URL carrying retrievability.

**The cap is an owner decision and is surfaced, not invented** (section 8, D1). The measured basis: six
of the ten held documents are already under 1 MB, eight are under 3.3 MB, and the one worked slim in the
repo landed at 0.90 MB. The two outliers at 20.76 MB and 16.20 MB are exactly the two that were never
slimmed. So the evidence puts the natural boundary in the 1 to 2 MB region, and the two files that
exceed any cap in that range are the two the policy is meant to catch.

**The two outliers are remediation candidates, not emergencies.** Both are US government works by
authorship (a USGS Bulletin and the NBS Journal of Research), so the licence question is unlikely to be
the problem; size is. Slimming `robie_hemingway_1995` to the tables the code reads would return most of
20 MB. That work is queued behind the owner's cap decision rather than done speculatively, because
slimming is irreversible against the held checksum and re-slimming to a different page set means a new
receipt.

---

## 5. The two hops: connecting a derived quantity to its held primary

The candidate-evidence DAG already exists and this plan consumes it rather than rebuilding it.
`FloorCandidateRecord` in `crates/physics/src/floor_provenance.rs` carries
`derived_from: Vec<String>`, enforced by the registry's own test (`"{} is derived but names no inputs"`).
It is an evidence-custody graph only. It is not joined to calibration, cannot admit an absolute-floor
entry, and is not reachable from the observer-only viewer.

The graph traces a derived quantity back to its input quantities and then stops. Two hops are missing.

### Hop A: a candidate leaf to the source that vendors it

`FloorCandidateRecord` has `id`, `status`, `derived_from`, `derive_first_defect`, `unsettled`, and
`sources`. A candidate leaf can therefore point at the manifest entry holding the paper it came from.
That final hop establishes evidence custody; it does not convert the candidate to `[M]` or authorize a
runpath magnitude.

**The hop is a `sources` key on the existing `[[grade]]` blocks in `crates/physics/data/floor_provenance.toml`:**

```toml
[[grade]]
id = "therm.heat_capacity.forsterite"
grade = "measured"
sources = ["robie_hemingway_1995"]
```

Put in the file that already exists rather than a new sidecar keyed by the same id, because two files
keyed by one id can disagree and this repo has a `diamond_gate.py` for exactly that failure.

**Implemented.** `FloorCandidateRecord::sources` is a default-empty list, and
`floor_provenance_gate.py` keeps parsing the historical `[[grade]]` data spelling. Empty means the source
link has not yet been migrated, never that the row is admitted or source-free.

### Hop B: a deriving function to its sources

A `// @derives:` marker names its inputs in prose with no machine-readable link. The marker is already
parsed by two consumers, `scripts/derives_gate.py` and `crates/sim/tests/derived_output_live.rs`, and the
latter enforces that a bracketed `@derives[id]` is unique across sites. Changing the marker's own grammar
would risk both parsers.

**The hop is a sibling line:**

```rust
// @derives: a rock's Gruneisen gamma <- the per-phase cited gamma table over the rock's modal census
// @sources: gruneisen_ahrens_1995, gruneisen_stixrude_2005
```

A separate `// @sources:` line cannot break either existing parser, and the ids resolve into
`sources/registry.toml`. With both hops in place, a walk from a derived quantity reaches a held,
checksummed, archived primary, and "is this substrate traceable to a vendored primary?" becomes a
question a gate answers rather than a person.

PR #201's `used_by` field is the same edge traversed from the source end. Both directions are kept: the
marker points down to sources, `used_by` points up to consumers, and the gate can cross-check them once
both populations are non-trivial.

---

## 6. The gate

`scripts/sources_gate.py`, in the family of `constructor_gate.py` and `derives_gate.py`, with the same
`--self-test` and `--update` verbs and the same grandfathering ratchet via `scripts/sources_baseline.tsv`.

**It fails on**, per the owner's list:

1. an entry missing `sha256`;
2. an entry missing an archive record (neither `archived_url`/`archive_url` nor `archive_pending` with a
   stated reason);
3. an entry missing `scope`;
4. an entry missing a slim record (`slim` for held bytes, `extract` for a witness);
5. a byte-holding (`custody = "in_repo"` or `"external"`) whose licence does not permit redistribution
   and which records no open alternative;
6. a claim carrying only a primary with no secondary;
7. a `@sources:` marker or a row `source_id =` id that does not resolve in the registry (the hop, enforced);
8. a registry id that is duplicated, and a `holding` path that does not exist or whose sha256 does not
   match, when the bytes are in-tree.

Checks 1 through 6 are the discipline; 7 is what makes the DAG walk real; 8 is integrity.

**Grandfathering.** `scripts/sources_baseline.tsv` carries one row per pre-existing source with the
classification of what has not been reviewed, following `derives_baseline.tsv` exactly:

```
# source_id	field	classification	reason
byerlee_1978	licence	licence-unreviewed	predates the licence policy; classify when next touched
janaf	archive	archive-unreviewed	predates the archive requirement
```

The existing population ratchets forward instead of demanding a 167-entry audit up front. A NEW source
gets no baseline row and must satisfy every check. The honest ceiling, stated the way `derives_gate.py`
states its own: this gate proves an entry was CLASSIFIED, never that the classification is correct.
Someone may record a licence finding wrongly. It removes the silent case and leaves the judged case to
review.

**Live-fire is mandatory, not optional.** Both gates written tonight had real defects that only
live-firing exposed (a `#[cfg(test)]` latch that blinded a whole file below it, in `derives_gate.py`).
The gate must be shown convicting a real bad entry in the real tree and going clean on revert, for each
of the failure modes, and the evidence recorded.

**Wiring.** A step in `.github/workflows/ci.yml` beside the other gates, running `--self-test` then the
gate, and a block in `.claude/hooks/stop-gate.sh` guarded on the script and baseline existing so it stays
inert until both are committed.

---

## 7. Migration, and what PR #201 would need to change

The migration is additive and staged so that old-form and new-form entries coexist, which is what lets
the concurrent agent keep working.

**Stage 1 (this change).** Create `sources/registry.toml` with the 27 existing sources migrated in,
citation and checksum and archive and scope carried over verbatim. Create the gate, the baseline and the
generator. Wire CI and the stop-gate. Prove nothing was lost by count and by spot-check. The existing
manifests are untouched at this stage, so nothing can break.

**Stage 2 (follow-on).** Add `source_id = "<id>"` to the manifest rows and `sources = [...]` to the floor
register grades, turning the grandfathered entries green as each is touched.

**Stage 3 (follow-on).** Add `@sources:` lines to marked deriving functions, closing hop B on the
population that has markers.

**PR #201 accommodation.** Its 31 entries are already close to the target schema and the registry accepts
them with two small changes on its side, neither urgent and neither blocking its merge:

- `archive_url` is accepted as an alias for `archived_url`, so **no rename is required**. The gate
  reads either.
- Its entries need `scope` and the licence fields (`licence`, `redistributable`, `licence_evidence`,
  `free_route`). Until then they take baseline rows exactly like the older 27, so the PR merges green.
- Its `custody` is `witness` for all 31 (bytes deliberately not committed, about 82 MB), which is a
  first-class class in this design and needs no change. Its `extract` field IS the slim record for that
  class.
- Its 2 `archive_pending` entries are accepted as-is, because a stated reason for a missing archive is
  the honest form.

The one thing that agent should adopt going forward is the `id` field name (its entries use `name`),
which the gate also accepts as an alias to avoid a rename on a live branch.

---

## 8. Owner decisions, surfaced rather than taken

**D1. The per-file size cap for in-repo held bytes.** Basis: six of ten held documents are under 1 MB,
eight under 3.3 MB, the one worked slim landed at 0.90 MB, and the two files that would exceed any cap in
the 1 to 2 MB region are precisely the two that were never slimmed (20.76 MB and 16.20 MB, together 81%
of 45.4 MB against a 226 MB `.git`). The decision is where to draw it and whether the two outliers are
slimmed now, converted to `witness`, or grandfathered indefinitely.

**D2 resolved.** `FloorCandidateRecord` now carries `sources: Vec<String>`. The candidate registry remains
off the canonical planet runpath and the viewer remains snapshot-only. A source link is evidence custody,
not admission.

**D3. Remediation order for the 27 grandfathered sources.** The ratchet resolves them as they are touched.
If the owner wants a deliberate sweep instead, the natural order is the 14 missing an archive URL first
(a link that rots is the failure the archive exists to prevent), then the licence review, then scope.

**D4. Whether `docs/design.md` Part 63 becomes generated.** Part 63 is the prose bibliography and
`docs/SOURCES.md` will be generated from the registry. Two bibliographies is a diamond. The options are
to leave Part 63 as the design document's own narrative bibliography and let `SOURCES.md` be the held-bytes
index (they answer different questions), or to generate Part 63's groups from the registry too. The first
is assumed here; the second is a larger change to a maintained document and is not taken unilaterally.

**D5. The inherited build block.** Independent of this work and blocking byte-pin verification for
everyone on this branch: `crates/physics/src/gruneisen.rs:236` uses `Fixed::from_decimal_str(raw.trim())`
inside a TOML field parser, and `constructor_gate.py` fails on it because it has no row in
`scripts/constructor_baseline.tsv`. It arrived with commit `8f18516` and is not on `origin/main`. The
working tree is clean and the gate still fails, so the defect is entirely inherited. The site reads as a
textbook `deserialization` classification, but it is another agent's commit and classifying someone
else's constructor is not this agent's call.

**D6. What remains after the ruling: JANAF and the OCW conflict.** The six restricted PDF holdings are
DONE, converted to citation-plus-witness with byte-identical witnesses verified. Two items remain and both
are yours. **JANAF**: the 34 tables cannot be converted as a data change (35 `include_str!` sites), so the
options are a permission request to NIST under the route 15 USC 290e itself names, an engineering arc
replacing the compilation with a derived numeric column cited to Chase 1998, or a deliberate accepted risk
recorded as the decision. **`mit_ocw_12108`**: CC BY-NC-SA inside an Apache-2.0 repo overclaims for that
file, and wants an isolated directory with its own licence note plus a `NOTICE` entry. `NOTICE` currently
carries no third-party attributions at all, against four remaining vendored PDFs.

**D7. The 89-file registry coverage hole.** Three directories vendor bulk data with per-row citations and
no source-level record at all (`janaf` 34 files, `optical_constants_aesopus` 45, `oxide_thermochemistry`
10, plus `aesopus_lowt` 22 grid files). A per-row md5 proves a file is uncorrupted; it does not say who may
redistribute it, what regime it holds in, or what was dropped in slimming. The gate now flags these four
collections and grandfathers them. The decision is whether each gets one collection-level `[[source]]`
entry (cheap, and it is where the JANAF licence problem would have surfaced years earlier) or stays
row-cited by exception.

**D8. Six documents are carried under two ids each.** Detected by identical sha256: three Gruneisen
cross-references to sibling directories, and three shared between `convection_scaling` and
`rayleigh_critical_eigenvalues`. The immediate consequence was real and is now closed (a licence finding on
one id did not reach its twin, so an identical copy read as un-reviewed), and the gate flags any future
recurrence. The remaining question is whether the cross-reference idiom should reference a single registry
id rather than copy the record, which would make the duplication structurally impossible.

**D9. Git history still contains the removed bytes.** The six PDFs are gone from HEAD, so a checkout no
longer redistributes them, but they remain reachable in history. Removing them from history needs a rewrite
(`git filter-repo`) that rewrites every commit hash and breaks every open branch and PR, including the
concurrent lanes. That is destructive and coordination-heavy, so it is surfaced rather than attempted, with
the honest note that "not in HEAD" is a weaker statement than "not in the repository".
