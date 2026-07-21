# CLAUDE.md: Operating Manual for Continuing This Project

This file is the entry point for any agent working on the canonical abiotic planet and stellar-system pipeline. Read it in full before changing code, research custody, or an authority document. It encodes the project customs, value boundary, workflows, and verification suite so work continues without methodological drift.

The owner is Nathan M. Fraske. The project is his. Every contested design call and every resolution of a research item belongs to him, not to the agent. No owner decision can admit an arbitrary magnitude to the canonical runpath.

---

## 1. What this project is

The canonical product is the deterministic abiotic planet and stellar-system pipeline in `civsim-planet`. It begins at the sealed absolute physics floor, advances through the seven causal stages, and hands an immutable completed snapshot to observer-only viewers. Biology, civilization, dawn, compose, the causal viewer, their scenarios, and their calibration methodology are parked legacy work.

`parked/docs/design.md`, `parked/docs/audit.md`, and `parked/docs/research/` preserve the earlier civilization design history. The verification hooks still protect their historical structure and prose, but they do not define the canonical runpath, authorize a value, or supply planetary readiness evidence. Current canonical state and order live in `HANDOFFS.md`, `TODOS.md`, `docs/working/CONSENSUS_ROADMAP.md`, and `docs/working/REPOSITORY_CLEANUP_PLAN.md`.

The eleven design principles govern everything. The three that come up most: Principle 3 (determinism), Principle 8 (order emerges, never templated), Principle 9 (physics may be an authored cultural input, cultural outcomes may not, enforced by the Steering Audit), Principle 10 (observer independence), and Principle 11 (data-driven by default; a hardcoded constant in the path of world content is a defect until it earns its place). When in doubt, read the actual part before relying on memory of it.

---

## 2. Prime directives (the owner's standing instructions)

These are absolute. They override convenience, and they override the agent's own confidence.

1. **Prove it before you trust it, and most of all when "it" is your own conclusion.** Never sign off on a claim, a compatibility check, or a generalization until it has been verified against the real specified system. Read the actual parts. Do not reason against a hypothetical version of the design.
2. **Audit the input, not the output alone.** When handed a research report, a request, or a prior decision, check the premises, not the result alone. The most valuable catches in this project have come from auditing the input.
3. **Never fabricate or reserve a canonical value.** A required magnitude must derive from the admitted absolute floor. If every derive-first route fails, the candidate may enter only through the structured floor-admission process in Section 7. Otherwise the run refuses.
4. **Emergence over templates.** Any design pattern that imposes order from outside the simulation rather than letting it arise from rules is a red flag. A closed enum or a lookup table sitting where world content should emerge is a defect. The sharp, testable form, a gate on every proposed substrate: if a substrate reads a high-level social or emergent fact (genetic relatedness, family or group membership, a named emotion, a skill or status level) to produce a behaviour, it is authoring, period. Replace it with the general causal primitive plus a proxy that correlates, and let the outcome emerge from selection. The template case is kin-biased cooperation: it is not wired by reading relatedness, which authors Hamilton's rule as a mechanism, but emerges because a being helps the familiar and nearby, which correlate with relatedness through local reproduction, so the rule becomes a description of the outcome, never a coded shortcut.
5. **Do not assume the owner is right either.** Present real data, verified sources, and honest tradeoffs. Disagree when the evidence supports it. The owner rewards a found seam over a smooth yes.
6. **The value-bearing input line is absolute.** The sealed absolute physics floor is the only canonical value-bearing input. Provenance marks and ledger tiers account for a value but never admit it. Calibration manifests, profiles, caller-authored world rows, arbitrary cited inputs, written state, and contingency do not enter the canonical path. Written state and contingency are generated within the run.
7. **Every mechanism must admit the alien.** No Earth, Mirror, Sun, familiar composition, fixed orbit, handpicked body count, or hindcast target may be an implicit world. The same causal stages must accept a physically different stellar system or planet without a special code path. Unsupported closure refuses rather than falling back to a familiar case.

**Mandatory audits.** Every end-of-arc or before-merge audit MUST run the standing panel lenses in `AGENTIC_ADDENDUM.md` section 9 (the fully-blind confirmation-bias catcher, the derive-versus-author catcher, the alien-feasibility catcher, the Terran-bias catcher, and the steering and Principles catcher) alongside the correctness lenses, each an independent panelist, every finding verified against source before it is trusted. These are the owner's standing requirement, not a per-arc choice.

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

## 4. Historical document anatomy and conventions

This section applies only when preserving the legacy civilization design record. It has no value-admission authority over the canonical planet path.

**The design document.**
- Parts are top-level `## Part N: Title`, numbered 0 through 63, gapless. Subsections are `### N.M` (for example `### 33.1`, `### 62.9`).
- A part that uses bold lead-in paragraphs (Part 41 is the model) keeps that style; a part that is prose-only (Part 54) keeps that; a part that specifies structures in `rust` code fences (Parts 9, 20, 25, 33, 36, 37) keeps that. Match the part you are editing.
- A resolved historical research item carries, at the site of its mechanism, a decision blockquote (Section 6), a record in Part 62, and a bibliography group in Part 63.
- An open research item carries, at its site, a `> Needs research, item R-XXX in the research backlog. ... Flagged, not changed.` blockquote.

**The audit log.**
- Section 1 consolidation blocks are `### 1a`, `### 1b`, and so on, each a single dense paragraph ending with a pointer to where the mechanism, record, and sources live. Retain old calibration wording only as quoted historical record. Subsections are separated by `---`.
- Section 3 backlog entries are bullets. An **open** item's bullet starts `- **R-XXX.` so it is counted. A **resolved** item's bullet is rewritten to start with a plain word, for example `- **Composition (R-DEEPTECH-COMPOSE): resolved.**`, so it drops out of the open count while staying findable.
- The limitation note at the end carries the running counts (of the form "N research questions are resolved ... and M remain open ...") and must be updated on every legacy resolve or flag. Read the current numbers from `parked/docs/audit.md` itself rather than from any count quoted elsewhere: a restated count goes stale silently, and three documents in this repo have carried a stale one.

**Research item identifiers.** `R-` plus a domain tag, sometimes two parts: `R-EVIDENCE`, `R-VALUE-METRIC`, `R-DEEPTECH-COMPOSE`. New items follow the same shape.

---

## 5. Historical research-document workflows

### 5a. Resolving a research item (after the owner signs off)

The owner signs off, almost always with the caveat that the result be **broadly generalizable** and, where it applies, **per-race differentiable** and **data-driven**. The flow:

1. **Ground.** Read the actual parts the item touches. Do not work from memory.
2. **Audit the report for generalization seams before integrating.** This step has caught a real seam in every resolution so far. Look specifically for a closed list authored one level down: a fixed enum of kinds, a fixed set of axes, a fixed set of combinators, a hardcoded list of conserved quantities. Harden each into a data-defined, extensible substrate or registry, sibling to the value substrate (Part 21), the semantic substrate (Part 33), and the institution-function substrate (Part 36): the mechanism is fixed Rust, the membership is data and grows with the world. Also confirm the result keys off per-entity or per-race data rather than a hardcoded notion (for example identity-permanence rather than "sentient").
3. **Consolidate the hardened, general form.** Replace the flag blockquote with the mechanism, written in the style of the part (prose, bold lead-ins, or code, as the part uses). Add the decision blockquote from Section 6. Add a four-paragraph record to Part 62 (the question; what was found; the decision taken, signed off and generalized; what remains to be proven). Add a bibliography group to Part 63 with precisely cited sources. Reconcile every cross-reference that deferred to the item.
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

## 6. The decision and admission blockquote

Every new canonical resolution ends with a blockquote of this shape, in prose, with no em dashes:

> Decided. [State the mechanism and causal scope.] Every value-bearing input comes from the admitted absolute physics floor. [Name the derivation ancestry.] If an irreducible candidate remains after derive-first exhaustion, [name the Buckingham-Pi budget, derivation attempts, Gap Law evidence, Residual Law evidence, and unique residual slot]. Unsupported closure refuses. Written state and contingency are generated. [State the alien-system falsifier and the remaining limits.]

The blockquote records a derivation or a refusal. It never reserves an owner-set magnitude and never treats a citation, tier, or provenance mark as admission.

---

## 7. Absolute-floor admission and refusal

This is the operational heart of "never fabricate a value." The canonical run has two routes: a value derives from the sealed floor, or the run refuses. An irreducible reference or residue is still part of the floor and is admitted only after the derive-first process proves why no derivation closes it.

- **Accounting.** The seven marks are `[D]` Derived, `[M]` Measured, `[E]` Estimator, `[C]` Closure, `[A]` Authored, `[W]` Written state, and `[X]` Contingency. The four tiers are Universal, Reference, Residue, and Contingency. Counts and tags describe the ledger; they do not authorize a magnitude.
- **Direct floor.** Universal irreducible leaves are measured fundamentals. Composite constants derive from named ancestry. A generic API may not bind a caller magnitude to an admitted identity.
- **Exhaustion route.** Every non-derived floor leaf, at any tier, requires nonempty derivation attempts, a per-phenomenon Buckingham-Pi input budget, Gap Law evidence, Residual Law evidence, and a unique residual slot. Vendored evidence supports the receipt but does not grant admission.
- **Generated categories.** Closure, written state, and contingency are not caller-supplied initial inputs. State and contingency arise inside the causal run and carry their generated provenance.
- **Failure.** If the required derivation or admission receipt is absent, incomplete, over budget, or outside the repository-owned audited catalog, the stage returns a structured refusal. No profile, manifest, scenario row, fixture, citation, or owner-selected number fills the gap.

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

- Do not invent, reserve, or accept a caller-authored canonical number. Derive it, admit an irreducible floor entry through the complete receipt, or refuse.
- Do not sign off on your own conclusion without proving it against the real parts.
- Do not author a closed enum or a lookup table where world content should emerge.
- Do not add em dashes or the banned adverbs, and do not let a "not just X, it is Y" construction past.
- Do not rewrite the archived research papers to the prose customs; they are preserved verbatim.
- Do not let the resolved and open counts, the part numbering, or the cross-references fall out of sync.
- Do not reach for a new top-level Part number lightly; the document has held at 64 parts, and renumbering breaks the gapless check. Extend an existing part or use a subsection unless a new part is clearly warranted and the counts are updated everywhere.
- Do not psychoanalyze, pad, or hype. State the work, prove it, name its limits.
