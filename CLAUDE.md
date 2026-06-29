# CLAUDE.md: Operating Manual for Continuing This Project

This file is the entry point for any agent picking up the emergent fantasy civilization simulator design project. Read it in full before touching either working document. It encodes the customs, the prose style, the workflows, and the verification suite that the project has been maintained under, so that work continues in the same voice and to the same standard without drift.

The owner is Nathan M. Fraske. The project is his. Every reserved number, every contested design call, and every resolution of a research item belongs to him, not to the agent.

---

## 1. What this project is

A single large Markdown design document for a custom Rust engine: a deterministic, emergent fantasy civilization simulator, a hybrid in the spirit of Dwarf Fortress and Songs of Syx. Simulation comes first; the visible game is a thin glyph view onto a deep world. The world is generated, every individual is modelled, and everything of consequence emerges rather than being authored: language, technology, money, governance, religion, cities, artifacts, beliefs. The deliverable is the full vision captured in prose and Rust-shaped pseudocode, with no minimum-viable-product gating.

The canonical knowledge lives in two maintained documents, plus a research record:

- **The design document** (`emergent_civ_simulator_design.md`): the specification, 64 gapless parts (Part 0 through Part 63). Part 62 holds the research records (62.1 onward), Part 63 holds the bibliography. This is the source of truth.
- **The audit and remediation log** (`AUDIT_AND_REMEDIATION.md`): the companion ledger. Section 1 is the consolidation history (1a onward), Section 2 lists every part that carries an open research flag, Section 3 is the research backlog grouped thematically, Section 4 is the inconsistency list, and the tail holds the queue and the limitation note with the running resolved/open counts.
- **The research papers**: standalone research reports behind the resolved items, archived for reference. Their reasoning is consolidated into the design document; the papers are the long-form source.

The eleven design principles govern everything. The three that come up most: Principle 3 (determinism), Principle 8 (order emerges, never templated), Principle 9 (physics may be an authored cultural input, cultural outcomes may not, enforced by the Steering Audit), Principle 10 (observer independence), and Principle 11 (data-driven by default; a hardcoded constant in the path of world content is a defect until it earns its place). When in doubt, read the actual part before relying on memory of it.

---

## 2. Prime directives (the owner's standing instructions)

These are absolute. They override convenience, and they override the agent's own confidence.

1. **Prove it before you trust it, and most of all when "it" is your own conclusion.** Never sign off on a claim, a compatibility check, or a generalization until it has been verified against the real specified system. Read the actual parts. Do not reason against a hypothetical version of the design.
2. **Audit the input, not the output alone.** When handed a research report, a request, or a prior decision, check the premises, not the result alone. The most valuable catches in this project have come from auditing the input.
3. **Never fabricate a value.** When a mechanism needs a specific number (a threshold, a rate, a weight), it is surfaced to the owner as reserved, with the basis for the decision stated, never invented. This is non-negotiable and is the subject of Section 7.
4. **Emergence over templates.** Any design pattern that imposes order from outside the simulation rather than letting it arise from rules is a red flag. A closed enum or a lookup table sitting where world content should emerge is a defect.
5. **Do not assume the owner is right either.** Present real data, verified sources, and honest tradeoffs. Disagree when the evidence supports it. The owner rewards a found seam over a smooth yes.

---

## 3. Prose customs (apply to all maintained prose, including the agent's own chat replies)

These govern the design document, the audit log, every research record, and the agent's messages. They do **not** apply retroactively to the archived research papers, which were produced as external research outputs and are preserved verbatim.

- **No em dashes, ever.** Use commas, colons, parentheses, or semicolons. This is the single most-checked rule.
- **Never use the words "genuinely", "honestly", or "actually"** in their adverb forms. The adjectives "genuine", "honest", and "actual" are fine ("honest limits", "the actual system"). Only the -ly adverbs are banned.
- **Minimize the "it is not just X, it is Y" construction**, and likewise "it is just X, not Y" and "not only X but Y". Keep them to an absolute minimum. Rephrase with "rather than", "over", a plain clause, or a colon.
- **Reasonably concise.** Disclaimers and caveats stay short; the substance carries the response.
- **Prose over bullets for explanations.** Reports, specifications, and explanations are written as prose. Lists, checklists, and code blocks are reserved for truly multifaceted reference material (a verification checklist, a step list, a data schema), which this manual and the audit log use deliberately.
- **Voice.** Plain, precise, declarative. State what the mechanism does and why, name the honest limits, cite the grounding. Avoid hype and filler.

---

## 4. Document anatomy and conventions

**The design document.**
- Parts are top-level `## Part N: Title`, numbered 0 through 63, gapless. Subsections are `### N.M` (for example `### 33.1`, `### 62.9`).
- A part that uses bold lead-in paragraphs (Part 41 is the model) keeps that style; a part that is prose-only (Part 54) keeps that; a part that specifies structures in `rust` code fences (Parts 9, 20, 25, 33, 36, 37) keeps that. Match the part you are editing.
- A resolved research item carries, at the site of its mechanism, a `> Decided and reserved.` blockquote (Section 6), a record in Part 62, and a bibliography group in Part 63.
- An open research item carries, at its site, a `> Needs research, item R-XXX in the research backlog. ... Flagged, not changed.` blockquote.

**The audit log.**
- Section 1 consolidation blocks are `### 1a`, `### 1b`, and so on, each a single dense paragraph ending with a pointer to where the mechanism, record, and sources live, and the note "Calibrations are reserved, not invented (below)." Subsections are separated by `---`.
- Section 3 backlog entries are bullets. An **open** item's bullet starts `- **R-XXX.` so it is counted. A **resolved** item's bullet is rewritten to start with a plain word, for example `- **Composition (R-DEEPTECH-COMPOSE): resolved.**`, so it drops out of the open count while staying findable.
- The limitation note at the end carries the running counts ("Fourteen research questions are resolved ... and eighteen remain open ...") and must be updated on every resolve or flag.

**Research item identifiers.** `R-` plus a domain tag, sometimes two parts: `R-EVIDENCE`, `R-VALUE-METRIC`, `R-DEEPTECH-COMPOSE`. New items follow the same shape.

---

## 5. The two core workflows

### 5a. Resolving a research item (after the owner signs off)

The owner signs off, almost always with the caveat that the result be **broadly generalizable** and, where it applies, **per-race differentiable** and **data-driven**. The flow:

1. **Ground.** Read the actual parts the item touches. Do not work from memory.
2. **Audit the report for generalization seams before integrating.** This step has caught a real seam in every resolution so far. Look specifically for a closed list authored one level down: a fixed enum of kinds, a fixed set of axes, a fixed set of combinators, a hardcoded list of conserved quantities. Harden each into a data-defined, extensible substrate or registry, sibling to the value substrate (Part 21), the semantic substrate (Part 33), and the institution-function substrate (Part 36): the mechanism is fixed Rust, the membership is data and grows with the world. Also confirm the result keys off per-entity or per-race data rather than a hardcoded notion (for example identity-permanence rather than "sentient").
3. **Consolidate the hardened, general form.** Replace the flag blockquote with the mechanism, written in the style of the part (prose, bold lead-ins, or code, as the part uses). Add the `> Decided and reserved.` blockquote (Section 6). Add a four-paragraph record to Part 62 (the question; what was found; the decision taken, signed off and generalized; what remains to be proven). Add a bibliography group to Part 63 with precisely cited sources. Reconcile every cross-reference that deferred to the item.
4. **Update the audit log.** Add a Section 1 consolidation block. Rewrite the Section 3 backlog bullet to the resolved form (so it leaves the open count). Update Section 2, the queue, and the limitation counts.
5. **Verify** (Section 8) and present.

### 5b. Flagging a new seam (surfaced from a probe, not yet researched)

When a sharp question exposes a real gap:

1. **Ground the gap in the actual part.** Confirm it is a real underspecification, not a misreading.
2. **Write the research question** as a `> Needs research, item R-XXX ... Flagged, not changed.` blockquote at the site, scoping the sub-questions, the grounding to survey, the determinism and data-driven constraints, and the couplings.
3. **Add it to the backlog.** A Section 3 bullet starting `- **R-XXX.` (so it is counted), plus a Section 2 entry, a queue note, and the limitation count bumped.
4. **No record and no bibliography** until the item is resolved.
5. **Verify** and present.

---

## 6. The "Decided and reserved" blockquote

Every resolution ends with a blockquote of this shape, in prose, no em dashes:

> Decided and reserved. The mechanism is settled and signed off, and on the owner's condition it is general and per-race differentiable. [One or two sentences stating the mechanism.] The mechanism is fixed Rust; [the data-defined parts] are data (Principle 11). What is reserved for your calibration, surfaced rather than fabricated, with its basis given: [each reserved value, each with the basis for deciding it]. The reserved list is in the audit log. The honest limits stand: [the real limits, plainly]. [Any couplings or remaining open siblings.]

The reserved list inside it is the contract with the owner: every number the mechanism needs, each named, each with the basis on which the owner would set it, none invented.

---

## 7. Reserved values and tuneables (how the owner's numbers work)

This is the operational heart of "never fabricate a value". The mechanism is fixed and lives in code and prose; the numbers it needs are the owner's to set, and until he sets them they are **reserved**, not guessed.

- **Where reserved values are recorded.** Three places, kept consistent: the `> Decided and reserved.` blockquote at the mechanism's site, the per-item Section 1 consolidation block in the audit log, and the research record in Part 62. Each lists every reserved value with its basis.
- **What "with its basis given" means.** Never just "this threshold is reserved". Always the ground on which the owner would decide it: "the failure boundary the material and physics data already define, such as pressure exceeding yield", or "the drift and loss rates the transmission subsystem already uses, set equal to them for consistency", or "the per-tick budget and the depth at which marginal gain falls below noise, a performance bound rather than a realism one". The basis tells the owner how to choose, so the choice is informed and not arbitrary.
- **How they reach code.** When the engine is built, every reserved value is a named constant in a calibration manifest (see the runbook), defaulting to a sentinel that fails loudly if used unset, never a silent plausible default. Nothing is hardcoded inline.
- **How they graduate.** The owner reviews the basis, sets the value, then playtests or calibrates against the stated target (for example "the population mean reconstructs the pool knowledge level within fixed-point tolerance"). Once set and validated, the value moves from reserved to set in the manifest, with who set it and when. The design document's reserved list and the manifest stay in step.
- **The agent never sets one.** If a task seems to require a concrete number, the agent surfaces it as reserved with its basis and stops there. Surfacing a number for review is the correct output; inventing one is a defect.

---

## 8. The verification suite

Run after every consolidation or flag, in **small, separate** bash commands. Long chained python-plus-awk-plus-grep pipelines time out; keep each check its own command. Let `F` be the design document and `A` the audit log.

- **Em dashes, both files (must be 0):**
  `grep -cP '\x{2014}' "$F"` and `grep -cP '\x{2014}' "$A"` (the em-dash codepoint; equivalent to greppping the literal character)
- **Banned adverbs, both files (must be 0):**
  `grep -ciE 'genuinely|honestly|[^a-z]actually[^a-z]' "$F"` and the same on `"$A"`
- **Parts gapless (must print "parts OK 64"):**
  `grep "^## Part" "$F" | grep -oP "Part \K[0-9]+" | awk 'NR!=$1+1{print "GAP"; b=1} END{if(!b) print "parts OK "NR}'`
- **Code fences balanced (count must be even):**
  `n=$(grep -c $'\x60\x60\x60' "$F"); python3 -c "print('BAL' if $n%2==0 else 'UNBAL')"` (the escape is three backtick characters)
- **Open backlog count (the running number of open items):**
  `grep -c '^- \*\*R-' "$A"`
- **No duplicate struct definitions** for any struct you added:
  `grep -c 'pub struct YourStruct' "$F"` (expect 1)
- **No stale references:** grep the doc for any sentence that still calls a now-resolved item "open" or "unsolved", and for the old flag text, and reconcile.
- **Discouraged constructions in new content:** `grep -ciE "not just|not only|it'?s just"` scoped to the block you wrote.
- **Records sequential:** `grep -oE '^### 62\.[0-9]+' "$F"` should be a clean run with no gaps.

A consolidation is not done until em dashes are 0, banned adverbs are 0, parts are gapless, fences are balanced, the backlog count moved by exactly the right amount, and no stale reference remains.

---

## 9. Editing mechanics and known gotchas

- **`str_replace` line shifts.** After any edit, line numbers below it move. Re-grep for anchors rather than reusing stale line numbers. The awk-filtered line numbers from a part-scoped read are relative to that stream, not the file; use `grep -n` on the whole file for true line numbers.
- **The struct-splice sweep.** When two structs or a struct and an enum share one `rust` fence, a Python splice that targets the fence can sweep an unintended definition out with it. After any splice near code, run the duplicate-struct and fence-balance checks, and confirm the struct you meant to keep is still present (`grep -c`).
- **Whole-part rewrites.** Write the new content to a temp file with a quoted heredoc (`cat > /tmp/x.md << 'EOF'`), grep it for em dashes, banned adverbs, and discouraged constructions, fix in place, then splice it in its own bash command.
- **The backlog-count trick.** `grep -c '^- \*\*R-'` counts only bullets that start with `- **R-`. A resolved bullet rewritten to start `- **Word (R-XXX): resolved.**` drops from the count while staying searchable by its identifier. Use this to move an item from open to resolved.
- **Separators.** Audit Section 1 and Section 3 subsections are separated by `---`. Preserve them when inserting.
- **`present_files`.** Present the most relevant file first. Keep the closing message short; do not append a long postamble after sharing files.
- **Do not recommend installing Claude Code or the desktop app.** That has been covered.

---

## 10. Session ritual

1. **Orient.** Read this file, then `HANDOFFS.md` for the current state and the last session's stopping point, then `TODOS.md` for what is queued. If a transcript of prior sessions exists, consult it incrementally for detail rather than loading it whole.
2. **Confirm scope with the owner** before a research dive or a resolution. The owner sets the target and the order.
3. **Do the work** under the workflows above.
4. **Verify** with the suite.
5. **Update memory.** Append a dated entry to `HANDOFFS.md` (what was done, what changed, where it stopped, what is next). Update `TODOS.md` (move resolved items out, add new flags, reorder). Keep both honest and current; they are how the next session avoids repeating work.
6. **Present** the changed files with a short summary that names the seam caught, the mechanism, and what remains.

---

## 11. What not to do

- Do not invent a number. Surface it as reserved with its basis.
- Do not sign off on your own conclusion without proving it against the real parts.
- Do not author a closed enum or a lookup table where world content should emerge.
- Do not add em dashes or the banned adverbs, and do not let a "not just X, it is Y" construction past.
- Do not rewrite the archived research papers to the prose customs; they are preserved verbatim.
- Do not let the resolved and open counts, the part numbering, or the cross-references fall out of sync.
- Do not reach for a new top-level Part number lightly; the document has held at 64 parts, and renumbering breaks the gapless check. Extend an existing part or use a subsection unless a new part is clearly warranted and the counts are updated everywhere.
- Do not psychoanalyze, pad, or hype. State the work, prove it, name its limits.
