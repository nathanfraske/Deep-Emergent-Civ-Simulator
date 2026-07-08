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

This skill operationalizes the panel-audit types documented in `AGENTIC_ADDENDUM.md` sections 7 through 10, so
they run correctly and consistently rather than being re-derived each time. Read `AGENTIC_ADDENDUM.md`
sections 7-10 for the full rationale and the method's own limits; this file is the procedure.

Two hard rules hold for both modes (Prime Directive 1 and the independence guard):
1. **Verify every surviving finding against the real source yourself before you trust it.** A blind panelist
   is a lead generator, not a verdict. A framing describes a mechanism, so confirm the kernel behaves as the
   panel assumes.
2. **Independence needs diversity, not just isolation.** Same-model, same-prompt panelists make correlated
   errors, so their agreement is one voice, not several. Spread panelists across agent TYPES and MODELS.

The workflow both modes author carries a `panels-reviewed` marker, so the PreToolUse panel-reminder hook
(`.claude/hooks/workflow-panel-reminder.sh`) passes: invoking this skill IS the consultation the hook guards.

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
2. **Run the panel** via the Workflow tool using `.claude/skills/panel/templates/framing-panel.js` (pass the
   statement and the neutral mechanism facts as `args`). Six panelists across three types and three models,
   each isolated, each set adversarial (attack the weakest point; is a high-level fact read to produce an
   outcome; does it admit the alien as data).
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
2. **Run the panel** via the Workflow tool using `.claude/skills/panel/templates/lens-audit.js`. The five
   section-9 lenses plus a correctness lens, each an independent panelist, then an adversarial verify per
   finding (default REFUTED unless substantiated at the cited file:line).
3. **Verify and harden.** Verify each surviving finding against source. Fix the real defects; log the honest
   limits. A world-content change is not audited until the five lenses have run, their findings verified, and
   the real defects hardened.

## When NOT to use this

- A framing or value that touches ONLY the physics floor (the one authored place): no panel needed.
- A settled, built mechanism: that is `audit` over the code, not `framing`.
- "Is a built concept alive in the running world?": that is the section-8 blind concept verification (judge
  from one run log alone), a different method; author it directly from the AGENTIC_ADDENDUM section-8 recipe.
