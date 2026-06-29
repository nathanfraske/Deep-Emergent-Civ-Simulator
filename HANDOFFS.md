# HANDOFFS.md: Rolling Session Log

Reverse-chronological. Each session appends one entry at the top: what was done, what changed in the documents, where it stopped, and what is queued next. Read the top entry first to recover state. Never rewrite past entries; append. Full detail for earlier sessions lives in the session transcripts.

---

## 2026-06-29: Applied the design-level audit findings (owner-approved)

**What was done.** On the owner's direction, the three design-level outcomes of the determinism audit were written into the maintained documents. C-08: the stale camera-drives-promotion text in Parts 6.2, 11.2, and 14.5 was reconciled to the governing Part 54 and the already-corrected Principles 6 and 10, so canonical promotion is significance-driven and a zoom requests a non-authoritative elaboration that never writes back. C-10: the EventId-dense-index versus Part 7.3 truncation conflict was recorded as Inconsistency 6 in the audit and noted at the Part 7.3 site. Ten determinism and reproducibility research flags were opened following the flagging workflow: a `> Needs research` blockquote at each site (R-RNG-COORD at 3.2, R-CANON-WALK and R-HARNESS-COVER at 3.5, R-CMD-ORDER at 4.3, R-GPU-CANON-PIN at 5.4, R-SAVE-SCHEMA at 7.3, R-LANG-DET at 33, R-UNITS-PIN at 55, R-REDUCE-ORDER at 57, R-PROJ-REGISTER at 58), a new Section 3 backlog subsection with ten counted bullets, and updates to audit Section 2, Section 7, and the Limitation note. TODOS gained the ten items and Inconsistency 6.

**Counts.** Fourteen resolved, twenty-eight open (eighteen plus the ten new flags). The audit backlog grep reports twenty-eight. No item was resolved; these are new open flags, so the resolved count holds.

**Verification.** The suite is clean: zero em dashes, zero banned adverbs, sixty-four gapless parts, balanced fences, records sequential, backlog count twenty-eight. The Rust workspace is untouched and still green.

**Where it stopped.** The maintained documents now carry the audit's design-level outcomes. Ready to open the pull request for the standup, the audit, the bedrock remediation, and these design edits.

**Queued next.** Owner sets the order on the ten new determinism-hardening items; they are best taken as the determinism, scheduling, and level-of-detail foundations are built. Inconsistency 6 is reconciled before event compaction is implemented.

---

## 2026-06-29: Determinism red/green audit and bedrock remediation

**What was done.** Ran a fan-out red and green team audit of the determinism, reproducibility, conservation, referential integrity, and observer independence invariants across fifteen documented subsystems, as a background workflow (thirty-one agents). Each subsystem had a red agent construct concrete divergence or leak scenarios and a green agent adversarially verify each against the actual spec and code, then add what red missed. The consolidated report is in `audits/determinism-redteam-2026-06-29.md`. It returned ten confirmed findings, ten design-level gaps, and seven soundly defended attacks.

**Confirmed bedrock defects, reproduced then fixed (red to green).** Each got a failing regression test first, then a fix. C-02: `Rng::range_i32` overflowed on a span wider than `i32::MAX`; the addend is now widened to `i64`. C-03: `quantize_unit` rounded the unit count then truncated the final division, contradicting its round-half-to-even contract; it now rounds the final placement half-to-even through a new integer helper. C-09: the event log double-indexed an id that was both actor and subject; `append` now deduplicates referenced ids. C-05 and C-04: a parallel `Fixed` reduction could panic in one chunking and succeed in another at an intermediate overflow; added `Fixed::sum_bits`, `checked_sum`, and `saturating_sum`, which accumulate in 128-bit space and are partition-independent, with the `+` operator and `Sum` documented as fail-loud and for bounded quantities, and the determinism harness extended with a near-overflow associativity case. C-06: merge and split left `Pooled` registry locations stale; `merge_pools` now repoints them through a new `Registry::repoint_pool`, and `referential_integrity_ok` checks location liveness rather than mere key presence. C-07: promote and split accepted arbitrary shares; they now reject negative or over-budget shares, and a `partition_lowest_id` helper implements the settled lowest-id remainder rule exactly.

**Guardrail added.** C-01 (the most severe finding): an entity stream keyed on an allocation-order `StableId` would make promotion order, which the camera can influence, change canonical ids and the state hash, breaking observer independence. Added `Rng::for_coords` (an observer-safe coordinate-keyed stream) and documented the constraint on `Rng::for_entity`. The deeper fix, minting canonical ids from camera-independent coordinates, is design-level and surfaced for the owner.

**Surfaced for the owner, not changed.** The design-level findings stay the owner's call: C-08 (stale camera-drives-promotion text in Parts 11.2 and 14.5 contradicting the governing Part 54), C-10 (EventId as a dense storage index versus Part 7.3 truncation), and ten proposed research flags from the gaps (canonical-walk ordering, command ordering, GPU bit-identity pinning, RNG coordinate keying, conserved-projection registration, reduction ordering, save schema versioning, harness coverage, the unit system, and language determinism). None were written into the maintained documents.

**Where it stopped.** The full suite passes (sixty-seven tests plus the ignored Stage-14 steering placeholder), `cargo fmt` and `cargo clippy -D warnings` are clean, and the document verification stays clean. The maintained documents are unchanged, so the resolved and open counts hold.

**Queued next.** The owner decides whether to apply the design-level findings: reconcile the stale promotion text against Part 54, resolve the EventId contradiction, and open any of the ten proposed research flags (which would edit `docs/audit.md` and `docs/design.md` and move the counts).

---

## 2026-06-29: Repository standup from THE_BOOK

**What was done.** Stood the project up as a buildable repository per the runbook. THE_BOOK.md was unbound into its parts with no change to content: the design document into `docs/design.md` (Parts 0 through 63), the audit and remediation log into `docs/audit.md`, and the eight standalone research papers into `docs/research/` verbatim, em dashes intact. The five memory and manual files (`CLAUDE.md`, `HANDOFFS.md`, `TODOS.md`, `AGENTIC_ADDENDUM.md`, `RUNBOOK.md`) are at the repository root. The Apache license was already present; added `NOTICE`, `README.md`, `.gitignore`, the Cargo workspace, and the pinned `rust-toolchain.toml`.

**Engine built (runbook section 2a bedrock).** `crates/core` is the determinism core with no external dependencies: the `Fixed` Q32.32 newtype with its arithmetic and property tests, the SplitMix64 counter-based RNG keyed on `(seed, entity, phase, counter)`, `StableId` and the registry, arena and slab allocators with a generational guard, the cache-line wrapper, the append-only event log with a stable-id provenance index, the typed canonical-state boundary, and the deterministic state hash. `crates/sim` holds the calibration manifest loader (reserved values load as fail-loud sentinels, with development and calibrated profiles), the conserved-projection registry, a minimal two-tier LOD world, and a data-driven substrate loader. Both standing harnesses run as tests: determinism reproducibility (the same seed at one, four, and the machine's worker count yields a bit-identical state hash) and conservation with referential integrity (promotion, demotion, merge, and split conserve every declared projection and leave no dangling reference). The full suite passes: 35 core unit tests, 4 determinism-harness tests, 16 sim tests, 3 invariants-harness tests, with `cargo fmt` and `cargo clippy -D warnings` clean.

**Agentic infrastructure.** `.claude/settings.json` wires the SessionStart, PreToolUse, PostToolUse, and Stop hooks to four scripts under `.claude/hooks/`. `scripts/verify.sh` is the verification suite as one callable script with a `--json` mode; the em-dash check matches the U+2014 bytes directly because this grep build rejects the code-point form. `.mcp.json` configures the filesystem and projectops servers; `tools/projectops_server.py` is a standard-library-only MCP stdio server exposing verify, backlog, reserved, and consolidation_check. `calibration/reserved.toml` is seeded with fifty reserved values surfaced from the design document's blockquotes (forty-nine reserved, one set: the lowest-id partition rule the design already settled). A GitHub Actions workflow runs the verification suite, the reserved-values review queue, and the Rust build, format, lint, and test.

**Seams handled.** The event log is kept schema-open: the kind is a data-defined identifier rather than a closed `EventKind` enum, honouring the open R-EVENT flag and Principle 4 instead of baking the taxonomy the design has not yet decided. `Fixed` is implemented as the newtype Part 58 requires for the compile-time canonical-state boundary, not the illustrative `i64` alias of Part 3.1. The steering and convergence-without-a-target harness is left as an honest ignored placeholder, since it depends on the technology design space and reserved physics proxies that do not exist yet; it is not faked. Every reserved value is surfaced from the design with its basis, none invented. A stdin-handling bug in two hooks (a heredoc consuming Python's stdin in place of the hook payload) was caught and fixed during testing.

**What changed in the maintained documents.** Nothing in substance: `docs/design.md` and `docs/audit.md` are the unbound forms of THE_BOOK Parts II and III, verifying clean. No research item was resolved or flagged this session, so the counts hold at fourteen resolved and eighteen open.

**Where it stopped.** The repository is stood up, builds, and tests green. No design work is mid-flight.

**Queued next.** Owner sets the order. The reserved-values review queue (fifty entries in `calibration/reserved.toml`) is ready for the panel. The research backlog is unchanged: ready items are R-TOM-UPDATE, R-INFRA, R-LANG-MODALITY, and the Part 35 pair R-WOUND and R-FLUID. The next engine milestone the runbook names is the first small-scale proof, convergence without a target (Stage 14), which waits on the technology layer and its reserved numbers.

---

## Current state (most recent)

**Status.** Fourteen research items resolved, eighteen open. The design document holds at 64 gapless parts; records run 62.1 through 62.10; the bibliography is Part 63. Both maintained documents verify clean: no em dashes, no banned adverbs, parts gapless, fences balanced, counts consistent.

**Resolved (14).** R-BEING-REP (Part 20), R-VALUE-METRIC (Part 21), R-AXIOM (Part 28), R-GENOME (Part 25), the six language-and-meaning items (Part 33), R-EVIDENCE (Part 9), R-INST (Part 36), R-TIER-CONSIST (Part 54), R-DEEPTECH-COMPOSE (Part 41). Each carries a `> Decided and reserved.` blockquote at its mechanism, a record in Part 62, a bibliography group in Part 63, and a consolidation block in audit Section 1. Three of these (R-VALUE-METRIC, R-INST, R-TIER-CONSIST) were consolidated directly into the design document; their standalone research reports are in the session transcripts rather than in `docs/research/`.

**Most recent work.** R-DEEPTECH-COMPOSE (recursive technology composition) was researched and resolved. The generalization audit hardened three seams before integrating: the conceived intent was changed from a closed enum to the emergent need-driven intent of the Part 41 lifecycle; the four evaluation combinators were changed from a fixed set into a data-defined extensible combinator registry grounded in the physics substrate; and the interface axes, leaf floor, combinator registry, and emergent-proxy set were framed as the etic grounding floor of composition, sibling to the value, semantic, and institution-function substrates and extensible with the physics. The mechanism is a content-addressed composition node, a three-gate promotion criterion (physics validation, transmission stability, compressive reuse), and a memoised interval-bounded interface-gated bottom-up evaluation. It lives in Part 41, record 62.10, audit section 1l.

**Open flag added this session.** R-TOM-UPDATE (recursive theory-of-mind nested-model update) was flagged from a probe of Part 37: the part specifies the mental-model structure and consumers but not the rule that populates the nested store from second-order evidence so it diverges from projection, plus the depth-bound fallback. Flagged at Part 37, audit Section 2, Section 3 (a new cognitive subsection), the queue, and the limitation count. No record or bibliography until it is resolved. Its substrate is the resolved evidence engine (R-EVIDENCE) applied recursively, so it is marked ready.

**Also produced this session.** A bound book of the two documents and the archived research papers; this operating manual (`CLAUDE.md`); the agentic addendum (panels, hooks, MCP, memory persistence); this runbook (repository standup under Apache 2.0 with Nathan M. Fraske as copyright holder, the buildable-now versus held-for-calls matrix, and the reserved-values process); and the seeded `TODOS.md`.

**Where it stopped.** The documentation and handoff package is complete. No design work is mid-flight.

**Queued next.** The owner sets the order. Ready to take when he chooses: R-TOM-UPDATE (substrate resolved), R-INFRA (reuses the etic-and-emic pattern from R-INST), R-LANG-MODALITY (the signed and non-vocal generalization), and the Part 35 pair R-WOUND and R-FLUID. Far-horizon by standing instruction: the four remaining deep-technology questions (wait on the technology layer being built and proven at small scale), R-EVENT and R-RELATION (most-wired, taken last; R-RELATION waits on the graph substrate), R-VIEW-ELAB (waits on the level-of-detail model), and R-COMMS (mail half is reachable from resolved R-INST plus open R-INFRA, the signal half waits on deep technology). Inconsistency 5 (technique origination) remains an owner decision that the composition promotion gate now leans on.

---

## How to add an entry

At the top of this file, above "Current state", add a dated heading and a short entry naming what was done, what changed in `docs/design.md` and `docs/audit.md`, the seam caught if any, where the session stopped, and what is queued. Keep it honest and current; it is how the next session avoids repeating work.
