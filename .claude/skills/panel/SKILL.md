---
name: panel
description: >
  Run a standing BLIND PANEL, the active counterpart to the panel-audit catalog in AGENTIC_ADDENDUM.md.
  Two modes. `framing`: the section-10 blind FRAMING panel, several isolated diverse-type/diverse-model
  agents critique a design-framing STATEMENT against the guiding principles alone (no author or owner
  conclusion, de-narrivatized), to catch an authored coupling hiding inside a plausible sentence BEFORE it is
  built. `audit`: the section-9 five mandatory lenses (confirmation-bias, derive-vs-author, alien-feasibility,
  Terran-bias, steering/Principles) plus correctness over a change, each an independent panelist with
  per-finding adversarial verification against source. Use `framing` before committing any emergence-critical
  design framing (any wire from a physical or felt input into a cultural, social, or belief outcome; any "this
  KIND of thing causes that KIND of thing" mechanism) to the canonical path. Use `audit` at every end-of-arc
  or before-merge on any world-content change (the owner's standing requirement). Invoke as `/panel framing`
  or `/panel audit`.
---

# panel: run a standing blind panel

This skill operationalizes the panel-audit types documented in `AGENTIC_ADDENDUM.md` sections 7 through 11, so
they run correctly and consistently rather than being re-derived each time. Read `AGENTIC_ADDENDUM.md`
sections 7-11 for the full rationale and the method's own limits; this file is the procedure.

Two hard rules hold for both modes (Prime Directive 1 and the independence guard):
1. **Verify every surviving finding against the real source yourself before you trust it.** A blind panelist
   is a lead generator, not a verdict. A framing describes a mechanism, so confirm the kernel behaves as the
   panel assumes.
2. **Independence needs diversity, not just isolation.** Same-model, same-prompt panelists make correlated
   errors, so their agreement is one voice, not several. Spread panelists across agent TYPES and MODELS.

**Use the two-stage JavaScript templates.** Both shipped templates now run the strongest available model at
the highest supported effort over the whole construction, the hoped-for conclusion, and correction history
before any panelist launches. Only a structured `CLEAR` supplies the genericized packet consumed by the
diverse panel. A missing, malformed, unreadable, or timed-out decision fails closed. The PreToolUse guard
checks for the structured protocol and rejects marker-only or no-op gate scripts. Its committed ten-case
self-test covers
non-panel, missing-gate, structured-gate, marker-only, malformed-input, unreadable-script, and resumed-script
cases, a decoy `CLEAR` check, and both active templates.

**No loopback semantics stall.** The smoke judges whether the bounded panel can falsify the claim, not whether
the packet is a perfect clone of the repository or whether every transitive process-control dependency is in
scope. It must separate a missing production surface or falsifier from an irrelevant omission, a broken
source-traceability path from surplus metadata, and a leading presupposition from wording it can neutralize
itself. After a material correction and rerun, a repeated
same-class objection must name a new concrete way the remaining construction could change a lens verdict.
Otherwise the workflow itself demotes it to a nonblocking note under `CLEAR`. If that materiality call
remains disputed, the workflow returns `OWNER_REVIEW` and stops recursion; do not keep repackaging equivalent
inputs. Supply each prior defect class, correction, claimed verdict consequence, and materiality disposition
through `smokeHistory` on a rerun so the rule is mechanically reachable rather than dependent on agent
memory. The first run must pass an explicit empty array. A block returns the next history with a pending
correction; the workflow will not rerun until the caller replaces that pending marker with the correction
made. Consequence comparison uses normalized prior text rather than trusting the smoke's boolean alone.

## Mode: framing (`/panel framing`)

For a design-framing STATEMENT, before it is built. The seam this catches is an authored coupling the words
claim to forbid: a high-level fact (the MEANING of an experience, a category, a status) read to produce an
outcome that must emerge. Its first run unanimously caught exactly that in a proposed experience-to-belief
framing (the "DIRECTION toward a pole" clause).

Steps:
1. **Build the sealed packet** (the load-bearing craft). Three parts, nothing else:
   - The guiding principles: P8, P9, P10, P11, the value-authoring line, admit-the-alien, and the template
     case, stated in full (copy from the template).
   - The minimal NEUTRAL mechanism facts: only enough to make the statement intelligible, traced to source,
     stripped of any conclusion.
   - The RAW statement, de-narrivatized (no vivid example that steers; write "a conviction axis", not "god";
     "hardship", not "resent"), phrased as a claim to attack, carrying NO author conclusion, NO owner
     conclusion, and NO hint of the flaw you suspect. If you suspect a flaw, keep it out of the packet.
2. **Run the smoke test, then the panel.** Author a two-stage Workflow from sections 10 and 11. Give the smoke
   agent the sealed packet, every panelist prompt, the lens roster, the correction history, and the
   conclusion the designer hopes to see. Only its explicit `CLEAR` over genericized inputs may launch six
   isolated panelists across diverse agent types and models. Use
   `templates/framing-panel.js`.
3. **Verify and synthesize.** Read the verdicts. Verify the decisive technical claim against source. Report
   where they converge, where they split (escalate a split, take the more severe reading), and the corrected
   framing that survives your own check against the principles. Record the resolved framing in
   `docs/working/OWNER_DECISIONS_LOG.md`.

## Mode: audit (`/panel audit`)

The five mandatory lenses plus correctness over a change (a diff, a slice, an arc). Required on every
world-content change.

Steps:
1. **Scope and stage the packet.** Capture the diff (`git diff`) and name the files/mechanisms under audit.
   For a correctness verdict that must not be contaminated by the repo's own tests and comments, build the
   section-7 blind packet (substrate contract + code only, no tests or docs) instead of pointing at the tree.
2. **Run the smoke test, then the panel.** Author a two-stage Workflow from sections 9 and 11. Give the smoke
   agent the packet, all six prompts, the model and agent-type roster, correction history, and the conclusion
   the designer hopes to see. Only its explicit `CLEAR` over genericized inputs may launch the five section-9
   lenses plus a correctness lens as diverse independent panelists. Use
   `templates/lens-audit.js`. Adversarially verify every finding, defaulting to refuted unless the cited
   source supports it.
3. **Verify and harden.** Verify each surviving finding against source. Fix the real defects; log the honest
   limits. A world-content change is not audited until the five lenses have run, their findings verified, and
   the real defects hardened.

## When NOT to use this

- A framing or value that touches ONLY the physics floor (the one authored place): no panel needed.
- A settled, built mechanism: that is `audit` over the code, not `framing`.
- "Is a built concept alive in the running world?": that is the section-8 blind concept verification (judge
  from one run log alone), a different method; author it directly from the AGENTIC_ADDENDUM section-8 recipe.
