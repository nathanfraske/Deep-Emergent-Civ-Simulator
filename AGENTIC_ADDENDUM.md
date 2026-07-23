# Agentic Addendum: Panels, Hooks, MCP, and Memory Persistence

This addendum specifies the agentic infrastructure for continuing the project inside a coding-agent harness. The reference harness is Claude Code, whose hook lifecycle, MCP configuration, and `CLAUDE.md` auto-loading are its native mechanisms; the requirements are stated so they map to any equivalent harness (the Claude Agent SDK exposes the same lifecycle as in-process callbacks, and Codex CLI ports the same JSON-on-stdin protocol). Nothing here touches the simulation engine's determinism: hooks and servers operate on the documents and the repository, never on the canonical core.

The goal is narrow and practical. The customs in `CLAUDE.md` are enforced by the agent's judgment; this addendum adds a mechanical safety net so a customs violation or a half-finished consolidation cannot slip through, and so the review panels have real data to render.

---

## 1. Memory persistence: the three files

Memory is persisted in three plain Markdown files at the repository root, each with a distinct role and lifecycle.

**`CLAUDE.md` (static; the operating manual).** Claude Code loads `CLAUDE.md` automatically into every session as project memory, so the customs, workflows, and verification suite are always in context. It changes rarely, only when a custom or workflow changes in substance, and any such change is the owner's call. Treat it as read-mostly.

**`HANDOFFS.md` (rolling; the session log).** An append-only, reverse-chronological log. Each session adds one dated entry: what was done, what changed in the documents, where the session stopped, and what is queued next. It is the first thing the next session reads to recover state without re-deriving it. Never rewrite history in it; append.

**`TODOS.md` (live; the canonical planetary queue).** A lean, parseable queue for the abiotic seven-stage runpath, floor-admission work, evidence gates, and repository cleanup. Biology, civilization, dawn, compose, scenarios, and calibration queues live under `parked/` and are never injected as canonical work. It is updated every session: completed items move out, new gaps move in, and order is adjusted. It is the source the backlog panel parses, so its format stays stable (see Section 5).

The persistence loop ties these to the hooks: `SessionStart` reads `HANDOFFS.md` and `TODOS.md` into context and establishes a clean verification baseline; the agent updates `HANDOFFS.md` and `TODOS.md` as part of finishing work; `Stop` refuses to end until they are current and the documents are clean.

---

## 2. Start and stop hooks

Hooks live in `.claude/settings.json` (project-committed, so the team shares them) under the `hooks` key. The events used are `SessionStart`, `PreToolUse`, `PostToolUse`, and `Stop`, with `SessionEnd` optional. Handler type is `command` throughout (a shell script reading JSON on stdin), except where a `prompt` or `mcp_tool` handler is called out.

### 2a. SessionStart: load memory and establish a clean baseline

`SessionStart` fires on startup and re-runs on resume with `source` set to `"resume"`, so it is the right place to inject context that must be fresh. It injects the tail of `HANDOFFS.md`, the open `TODOS.md`, and the result of the verification suite, so the agent begins knowing the current state and whether the documents are already clean. Context is returned to the agent through `hookSpecificOutput.additionalContext` (capped at 10,000 characters, so the script injects the handoff tail and the verify summary, not whole files).

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/session-start.sh\"" }
        ]
      }
    ]
  }
}
```

`session-start.sh` runs `scripts/verify.sh` (Section 4 of the runbook), then emits the last `HANDOFFS.md` entry, the open `TODOS.md` items, and the verify pass/fail summary as `additionalContext`. If the baseline is not clean, it says so loudly, so the agent knows a prior session left the documents in a bad state before it adds to them.

### 2b. PreToolUse: the customs guard (prevention)

`PreToolUse` fires before a tool runs and can block it by exiting 2. Matched to the file-writing tools, it is the hard guard that a customs violation cannot be written into the maintained documents in the first place. It inspects `tool_input` on stdin: if the target is the design document or the audit log and the incoming content contains an em dash or one of the banned adverbs, it blocks and returns the reason, which Claude Code feeds back to the agent.

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit|str_replace",
        "hooks": [
          { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/customs-guard.sh\"" }
        ]
      }
    ]
  }
}
```

`customs-guard.sh` parses `tool_input.file_path` and the new content; if the path is a maintained document and the content contains an em dash or one of the banned adverbs in adverb position, it exits 2 with a message naming the violation. It leaves the archived research papers and other files alone. A `PreToolUse` `deny` blocks the tool even under bypass mode, so this guard holds regardless of permission settings.

### 2c. PostToolUse: per-edit verification (reaction)

`PostToolUse` fires after a successful edit and cannot undo it, but it can surface feedback. Matched to edits of the maintained documents, it runs the fast checks on the changed file (em dashes, banned adverbs, fence balance) and, when something is off, returns a message so the agent fixes it immediately rather than discovering it at the end.

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit|str_replace",
        "hooks": [
          { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/post-edit-check.sh\"" }
        ]
      }
    ]
  }
}
```

### 2d. Stop: the completion gate

`Stop` fires when the agent is about to finish a turn and can force it to keep working by exiting 2. This is the gate that enforces the whole verification contract: the turn cannot end while the documents are dirty or the memory files are stale. The script runs the full verification suite and checks that, if either maintained document changed this session, `HANDOFFS.md` and `TODOS.md` were updated. If anything fails, it exits 2 with the specific failure; if all passes, it exits 0.

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "\"$CLAUDE_PROJECT_DIR/.claude/hooks/stop-gate.sh\"" }
        ]
      }
    ]
  }
}
```

A `Stop` hook that exits 2 makes the agent continue, which can loop, so `stop-gate.sh` first checks the `stop_hook_active` field on stdin and exits 0 if it is already set, allowing the agent to stop on the second pass. The gate is a backstop, not a substitute for the agent running the suite itself (Section 8 of `CLAUDE.md`).

### 2e. SessionEnd (optional): persist a reminder

`SessionEnd` fires once as the session closes. A light `command` handler can append a timestamped marker to `HANDOFFS.md` or log session statistics. It does not replace the agent's own handoff entry, which carries the substance.

---

## 3. MCP server standup

Two MCP servers are worth standing up. Both are configured in `.mcp.json` at the repository root (project-scoped, committed) as stdio servers; their tools are addressed as `mcp__<server>__<tool>`.

**A filesystem server scoped to the repository.** Optional under Claude Code, which has native file tools, but useful for a custom harness or for restricting access to the docs tree. Point it at the repository root.

**A custom `projectops` server (the high-value one).** A small server that turns the verification suite and the project's structured data into callable tools and resources, so hooks and panels consume structured results rather than re-deriving them from raw greps. Suggested surface:

- Tool `verify`: runs the full suite and returns structured JSON (each check with name, pass or fail, and the offending lines). This is what the `Stop` gate's logic and the verification panel both call. A hook can invoke it directly as an `mcp_tool` handler, addressed `mcp__projectops__verify`.
- Tool `backlog`: parses the bounded canonical section of `TODOS.md` into the planetary work list with order, dependencies, and status.
- Tool `floor_admission`: renders the repository-owned accounting inventory and admission state: tier, provenance mark, derivation ancestry, exhaustion receipt, Buckingham-Pi budget, Gap Law with its typed Chaos Protocol branch, Residual Law obligations, and admitted or refused status. It exposes no editable value field.
- Tool `consolidation_check`: given an item id, confirms the resolution is complete: flag replaced, record present in Part 62, bibliography group present in Part 63, backlog bullet rewritten, counts consistent.

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "${CLAUDE_PROJECT_DIR}"]
    },
    "projectops": {
      "command": "python3",
      "args": ["${CLAUDE_PROJECT_DIR}/tools/projectops_server.py"],
      "env": { "REPO_ROOT": "${CLAUDE_PROJECT_DIR}" }
    }
  }
}
```

A note that matters: a panel backed by MCP must be a real server. The visualizer renders static HTML and cannot execute calls to an MCP URL from inside an artifact, so the panels in Section 5 read from the `projectops` tools through the harness, not from a `fetch()` embedded in rendered HTML.

---

## 4. Panels and the features they require

Panels are review surfaces the harness renders for the maintainer. Each needs one concrete thing implemented to work, and that thing is the same in every case: structured data from `projectops` plus stable, parseable document formats. The panels worth having:

**Verification panel.** Shows live pass or fail of the suite: em dashes, banned adverbs, part gaplessness, fence balance, the open backlog count, duplicate-struct guard, and stale-reference scan. Requires the `projectops verify` tool to emit structured results; without it the panel has nothing to render but raw grep output. This is the single most important feature to implement, because it is what the `Stop` gate and the maintainer both rely on.

**Backlog panel.** Shows the canonical planetary items in working order, with dependencies and status. Requires `TODOS.md` to keep a stable bounded format (Section 5) and the `projectops backlog` tool to parse it. Parked research queues are excluded.

**Floor-admission panel.** Shows the complete four-tier by seven-mark accounting inventory and every admission or refusal receipt. For every non-derived floor candidate at every tier it shows derivation attempts, the Buckingham-Pi budget, Gap Law evidence including the typed Chaos Protocol branch, Residual Law evidence, and residual-slot identity. Evidence custody is visible but cannot switch a candidate to admitted. The panel has no editable magnitude field.

**Consolidation panel.** Shows, for the item worked this session, whether its resolution is complete: flag replaced, record added to Part 62, bibliography group added to Part 63, backlog bullet rewritten, counts moved by the right amount. Requires the `projectops consolidation_check` tool. This catches the most common incomplete-resolution mistakes before the session ends.

The common requirement, stated once: for any panel to work, the verification suite must exist as a callable script or tool emitting structured output (not ad-hoc greps), and the parseable artifacts (`TODOS.md`, the canonical ledger inventory, admission receipts, and the historical part and record headings) must hold stable formats. Those two things are the agentic features that make panels function; everything else is rendering.

---

## 5. Stable formats the panels depend on

So the tools can parse without guesswork, hold these conventions:

- **Part and record headings** stay `## Part N: Title` and `### N.M`, gapless, as the verification suite already assumes.
- **Historical research bullets** retain their old `R-XXX` format under `parked/`; they are not a canonical queue.
- **`TODOS.md`** contains a bounded `## Canonical abiotic queue` section. Each open item is one `- **P-XXX.**` bullet with the identifier first, a one-line outcome, dependencies, and its refusal or acceptance receipt.
- **The ledger inventory and admission receipts** are generated artifacts. Each entry has a stable identity, tier, provenance mark, ancestry, and admitted or refused state. An irreducible candidate also carries its exhaustion and law receipts. Neither artifact carries an operator-editable magnitude.

---

## 6. The lifecycle, end to end

A session runs: `SessionStart` loads the handoff tail and canonical planetary queue and establishes a clean verification baseline, re-running on resume so the baseline is never stale. The agent works under the `PreToolUse` customs guard, which blocks an em dash or a banned adverb from ever entering a maintained document, and the `PostToolUse` check, which surfaces a slip the moment it happens. As it finishes, the agent updates `HANDOFFS.md` and `TODOS.md`. The `Stop` gate then refuses to let the turn end until the full suite passes and the memory files are current, guarding against a loop with the `stop_hook_active` check. `SessionEnd` optionally persists a closing marker. The panels read structured results from `projectops` throughout, so the maintainer sees verification status, the planetary queue, the floor-admission state, and consolidation completeness without leaving the harness.

The division of labour is deliberate: the agent's judgment does the work and runs the suite itself, and the hooks are the mechanical backstop that makes a customs violation or a half-finished consolidation impossible to ship rather than merely discouraged.

---

## 7. The fully-blind audit (for uncontaminated correctness and value verdicts)

Reach for this when a verdict must not be contaminated by the repository's own framing: verifying that kernels, formulas, or floor-admission claims are correct, where a shared blind spot between the code and its own tests could hide a defect. A test written to match a buggy output, a comment that rationalizes an error, or a prior sighted review that inherited the same assumption will all pass a normal review, because they were written against the same premise. A blind auditor, given only what the code claims and what it is built on, reaches its verdict from first principles and does not inherit that premise. This method found scale bugs in merged, reviewed kernels (a missing megapascal-to-pascal promotion that made a buckling load a million times too small) that the sighted red-teams and the passing test suite both missed, because the tests encoded the same error.

**The packet.** The auditor sees only a self-contained packet, written to the scratchpad outside the repository, with three sections. Section A is the substrate contract: the exact semantics of every type and primitive the code calls (bit widths, scales, rounding direction, which operations panic or wrap or saturate, the determinism rules), so totality, overflow, and rounding are judgeable without the source; A also states the audit checklist (physical faithfulness, dimensional soundness, totality, precision discipline, fabrication and steering) and any non-obvious unit or scale conventions the code assumes. Section B is the code under audit, the functions only, with the tests, the prior audits, the design docs, the cross-references, and the code's own self-assessing comments all left out (a comment that asserts the property under audit, such as "so all sections survive the cap," is exactly the premise-laden artifact this method exists to defeat: leave it in and a blind auditor can echo the claim back as a verdict instead of deriving it). Section C is the declared specification: what each unit is supposed to compute, its inputs with their units, dimensions, and ranges, and its declared output and bound. The packet carries the code and what it claims, and nothing of the repository's proof that it is correct.

**The protocol.** First pilot with one agent: is the packet sufficient to reach a conclusion with zero repository access? The pilot audits a spanning sample and reports, precisely, any information the packet lacked. Fold those gaps into the packet before spending on a panel, so the panel is not blocked on a missing contract. Then run the full panel: several independent auditors, each blind, each reading only the packet, none aware of the others or of any prior conclusion, each classifying per the checklist. To test whether they converge (the point of the exercise), double-cover the arithmetic-heavy slices or run independent passes and compare. Then verify every flagged finding against the real code yourself before trusting it: the auditor is blind and may misread an intended convention, so a finding is a lead to prove, not a verdict to accept (Prime Directive 1). Reconcile: convergence across independent blind auditors is strong evidence, and a finding that survives your own check against the source is a real defect to surface and fix.

**Enforcing the blind.** Instruct each auditor to read only the packet file and to not read, grep, glob, or open any file under the repository. If an auditor feels it cannot judge a kernel without the repository, it must not go and get it: it records exactly what the packet was missing, which sharpens the packet rather than breaking the blind.

**Model tiering.** Panelists run on the cheapest model that accomplishes the goal: Sonnet for reasoning-level correctness audits, Haiku when the work is sheer mass over many simple units, and Opus reserved for the hardest kernels that need full depth. The pilot and the per-finding verification are the same tiering call.

The vehicle of a run is the packet plus the panel outputs, kept in the scratchpad; a durable finding graduates into a fix and, if it reveals a class of defect, into a checklist item the next packet's Section A names by default.

**The method's own limits (surfaced by running this method against itself).** The blind protects the auditors from the repository's framing, but it moves the single point of contamination onto the packet, which a sighted and often non-independent party writes: a wrong, curated, or incomplete Section A or C launders the very shared premise the panel exists to catch, and because the final safety net only rechecks findings that were flagged, a false negative from a slanted packet reaches none of the steps. Five guards follow. First, build the packet to a fidelity discipline: trace every contract fact and spec claim to the source rather than to memory or to the code's own comments, and where the stakes justify it have a second party (or a separate blind agent) check the packet for completeness and for framing that presumes the conclusion before the panel runs. Second, the final verify-against-source step decides what is trusted, so recompute each finding from first principles rather than re-reading the code's rationale, and cross-check a dismissal of a converged finding rather than waving it off. Third, non-convergence is not a null result: when some auditors flag and others do not, or classifications differ, escalate the split to the verify step and, unresolved, to a higher tier or the owner, taking the more severe classification when split, never silently dropping it. Fourth, independence is more than mutual unawareness: same-model, same-prompt panelists make correlated errors, so their agreement is one strong opinion rather than several independent ones, and a load-bearing verdict wants diversity across models and varied prompt framing. Fifth, prefer a hard blind to an instructed one where the harness allows it (run the auditors with the repository tools or mount withheld), since the instruction is a compliance control, not a barrier. Scope: the unit-only packet audits a unit against its declared contract, so a defect that lives at the boundary with the unit's real callers, or a drift between the unit and a spec the packet restates, is out of its reach by construction.

## 8. Blind concept verification (for whether a concept is realized in the running world)

Reach for this when the question is not whether the code is correct but whether a CONCEPT is realized in the world it produces: does the running simulation exhibit the thing we meant to build, or is it present in code and inert, mismeasured, or unobservable? A normal test, and even the fully-blind code audit of section 7, can pass while the concept is dead, because both judge the code and not the world it makes: a conversation step can run every tick over an empty belief map, a reader can report the wrong quantity (an inherited disposition where the concept is about live transmission), and a co-designed assertion checks the number the author chose to emit. A blind verifier, given only the concept as a claim and one run of the world, must find the concept in the behaviour or report that it is not there. Run against the wired world's conversation concept, this method independently returned "not realized, and not even measured by this output," and pinned the decisive evidence a sighted reviewer had waved past: the belief stances were frozen and clustered exactly by race, the fingerprint of inheritance, the impostor the concept was written to be distinct from.

**The sealed inputs.** The verifier sees three things and nothing else. The concept: a plain-language statement of what the thing is and does, framed as a claim to be tested rather than a fact, with no named mechanisms, no implementation hints, and no description of what to look for, so the verifier cannot echo the design back as a verdict. The principles: the prime directives (prove it, audit the input, order emerges rather than imposed, observer independence, name the honest limits), so it judges by the project's own standard. The evidence packet: a raw log of one deterministic run with its seeds recorded, and nothing of the source, the design, the tests, or any expected output. The packet carries what the world did and nothing of what it was supposed to do.

**The protocol.** First the verifier derives, from the concept and the principles alone and before it reads the log, the observable signatures a fully realized version would necessarily produce over time, and it distinguishes the genuine article from its impostors (for a transmission concept, transfer against inheritance and independent drift); it generates these itself and is never handed them, which is what keeps the search unbiased by the implementation. Then it searches the log for each signature, evidence for and against. Then it self-checks adversarially: could an apparent positive be something other than the concept, and does the log even measure the quantity the concept is about or only a proxy or a different quantity, since a log that does not measure the thing is itself the finding. Then it renders a verdict, fully realized, partially realized, not realized, or not observable from this output, with the specific log evidence and no hedge into a false yes. To make a load-bearing verdict trustworthy, run a small panel of independent verifiers and escalate a split to a higher tier, and include a positive control, a concept known to be realized, so a reflexive naysayer is caught.

**Enforcing the blind.** The verifier reads only the run log; reading, grepping, or opening any source, design, or test file is a protocol failure, because seeing the mechanism lets it confirm the concept from the code rather than from the behaviour, which is the one thing the method exists to prevent. If it feels it cannot judge without the source, it records what the log did not show, which is a request for a reader rather than a reason to break the blind.

**The method's own limits.** It tests implemented and observable together, not implemented alone: a concept live inside the engine but surfaced by no reader scores not observable, which for a world meant to be watched is the honest verdict (an emergence nobody can see is not delivered) but means the right response to that verdict is often to build the reader and re-run, not to conclude the mechanism is broken. Absence of evidence in one log is not proof the mechanism is absent, only that this run did not demonstrate it, and the verifier must say so. The verdict is only as good as the concept statement: too vague and the derived signatures are weak, too leading and the blind is broken, so writing the concept plainly and hint-free is the load-bearing craft. And a single verifier on a single model is one opinion; a verdict that will gate work wants a panel across models and varied framing, the same independence discipline the blind audit of section 7 requires, since agreement among correlated agents is one voice, not several.

## 9. The mandatory audit-panel lenses (a standing, inter-agent requirement)

Every end-of-arc or before-merge audit, by any agent, MUST run the following lenses as independent panelists, in addition to the correctness lenses the work suggests (conservation, determinism, byte-neutrality, overflow). These are the owner's standing requirement, not a per-arc choice, and they exist because the author's own framing and the code's own tests share the author's blind spots. Each lens is a separate agent so no one prompt dilutes another, and every finding is verified against the real source before it is trusted (Prime Directive 1). Verify findings against source yourself; a blind panelist is a lead generator, not a verdict.

1. **The fully-blind confirmation-bias catcher.** An agent blind not only to the tests and comments (section 7) but to the AUTHOR'S APPROACH and its rationale: it is given the problem and the code's behaviour, never the author's account of why the approach is right, and it is told to find where the author has confirmed a chosen approach rather than tested it. It hunts the shape of confirmation bias: an assumption carried unquestioned from the design into the code and the tests alike, a test written to pass the approach rather than to break it, a "clearly correct" step that was never adversarially probed. Its job is to attack the approach, not to grade the implementation of it.

2. **The derive-versus-admit catcher.** An agent whose sole charge is the canonical value boundary: the sealed absolute physics floor is the only value-bearing input. It flags every constant, threshold, rate, weight, fraction, table, profile, manifest, and caller row in the causal path and asks for a complete derivation ancestry. If derive-first fails, every non-derived floor candidate at every tier needs nonempty attempts, a per-phenomenon Buckingham-Pi budget, Gap Law evidence including its typed Chaos Protocol branch, Residual Law evidence, and a unique residual slot before floor admission. For dynamical candidates it requires an explicit regime partition and transition law, rejects fixed-path integration when sub-resolution divergence exceeds admitted input bands, and requires a derived stationary measure plus a versioned content-derived realization coordinate for each such regime. Mixed or changing dynamics cannot be compressed into one label. The seven provenance marks and four tiers are accounting only. A citation, owner choice, scenario, or convenient test value cannot grant admission. The lens checks `docs/working/PHYSICS_FLOOR_REGISTRY.md` for discovery coverage and `docs/working/CANONICAL_LEDGER_INVENTORY.txt` for admitted membership.

3. **The alien-system feasibility catcher.** An agent that constructs an adversarial stellar and planetary system: a non-solar star, unfamiliar elemental inventory, multiple stars, unusual disk structure, different body counts, non-Earth geodynamics, or a system outside familiar hindcast targets. It checks whether the same seven stages can derive or refuse that case without an Earth, Mirror, Sun, fixed-orbit, fixed-composition, or fixed-body-count branch. The alien must be a realization of the same physics, never a hand-authored scenario row.

4. **The Terran and Solar bias catcher.** An agent hunting for Earth-specific, Solar-System-specific, or local-Milky-Way values and names in the causal decision path. A measured reference may test or bound a mechanism, but it cannot choose the realization. The lens traces every familiar anchor to confirm it is either derived from the admitted floor, used only as noncausal evidence, or causes a structured refusal.

5. **The steering and observer-independence catcher.** An agent applying Principles 8, 9, and 10 directly: does a hindcast target, desired planet class, viewer preference, display cadence, or authored classification feed back into physical construction or evolution? Does the seed stand in for a missing physical measure? Does the viewer mutate a snapshot or advance physical time? A canonical mechanism may generate state and contingency, but it cannot accept them from an observer or profile.

**The canonical two-route rule (owner-ruled 2026-07-20, clarified 2026-07-21).** Every value-bearing quantity either derives from the sealed absolute floor or the stage refuses. Every non-derived floor entry, at any tier, enters only after structured derive-first exhaustion under the Buckingham-Pi, Gap Law with typed Chaos Protocol, and Residual Law receipts. There is no calibration route, per-world authored row, development fallback, or owner-selected magnitude.

**The accounting rule.** The seven marks are `[D]`, `[M]`, `[E]`, `[C]`, `[A]`, `[W]`, and `[X]`; the four tiers are Universal, Reference, Residue, and Contingency. They describe origin and position in the ledger. They never grant value admission. Universal measured fundamentals and composites derived from them form the present audited floor. Closure, written state, and contingency are generated rather than supplied as initial inputs.

**The evidence rule.** A vendored source, citation, checksum, provenance mark, or source-stated band establishes evidence custody. It does not by itself make a value canonical. The value must still derive or pass the complete admission process, and an unsupported value must remain absent while the stage refuses.

These five plus the correctness lenses are the panel. A change that touches the canonical planet path is not audited until they have run, their findings have been verified against source, and the real defects have been hardened.

## 10. The blind framing panel (for a design-framing STATEMENT, before it is built)

Reach for this when the thing to check is neither the code (section 7) nor the running world (section 8) but a FRAMING: the sentence or paragraph that will govern how a mechanism works, written before there is any code to audit. A framing can read as principled and self-consistent and still author the very coupling it claims to forbid, because the authoring hides inside a plausible word. The other two methods cannot catch this: there is no kernel to trace and no run to read, only the design intent. A blind framing panel, given only the guiding principles and the raw statement, judges the intent from first principles and finds the authored coupling before it reaches the canonical path. Run against a proposed "felt experience enters belief-update as signed evidence with a DIRECTION toward a pole of a conviction axis," this method returned, unanimously and independently across three agent types and three models, that the DIRECTION clause authored a kind-of-experience to kind-of-conviction map, the kin-template violation wearing belief instead of kinship: deciding that hardship bears on providence and points to its negative pole reads the high-level meaning of the experience to produce the outcome. The corrected framing the panel converged on emits only a magnitude and a valence sign from the floor, and lets the coupling to a conviction be LEARNED per-being (a primitive plus a correlating proxy), so the outcome is a description of a learned result and never a coded route.

**The sealed input.** Each panelist sees three things and nothing else. The guiding principles: Principles 8, 9, 10, 11, the value-authoring line, admit-the-alien, and the template case stated in full, so it judges by the project's own standard. The minimal neutral mechanism facts: only enough context to make the statement intelligible (what the kernel does, what signal exists), stripped of any conclusion. The raw statement: phrased as a claim to attack, de-narrativized (no vivid motivating example that steers, no "god" or "resentment" where "a conviction axis" will do), and carrying NO author conclusion, NO owner conclusion, and NO hint of the flaw the author already suspects. The packet carries what the framing proposes and nothing of anyone's belief that it is right.

**The protocol.** Run several isolated panelists across DIVERSE agent types and DIVERSE models (independence is more than mutual unawareness: same-model, same-prompt agents make correlated errors, so their agreement is one voice, not several, the fourth guard of section 7). Each is set adversarial: attack the weakest point first, do not assume the statement is right, and answer specifically whether anything quietly authors a coupling that must emerge (a hidden lookup, a fixed category, a high-level fact read to produce an outcome) and whether the framing holds for an alien being as data. Collect their verdicts and their proposed reframings independently. Then verify the decisive claim against source yourself (Prime Directive 1): a framing describes a mechanism, so confirm the kernel behaves as the panel assumes (here, that the accommodation kernel moves toward the event's own direction, so an event-side authored direction cannot itself produce the divergence the framing promised). Convergence across independent diverse panelists on the same seam is strong evidence; the reframing they agree on, once it survives your own check against the principles, is the framing to build.

**When to use it, and when not.** Use it before committing an emergence-critical framing to the canonical path, above all where an authored kind-to-kind coupling could hide inside a sentence: any wire from a physical or felt input into a cultural, social, or belief outcome; any "this kind of thing causes that kind of thing" mechanism; any place the owner or the author feels the framing is important and is not certain it is clean. It is cheap relative to building the wrong mechanism into the belief or culture path and unwinding it later. Do not use it for a framing that only touches the physics floor (the one authored place), for a settled mechanism already built (that is section 7 or the section 9 lenses over the code), or for a question about whether a built concept is alive in the world (that is section 8).

**The method's own limits.** It judges the WORDS, so a framing that reads clean can still be built wrong: the framing panel gates the design intent, and the code that realizes it still needs the section 9 lenses and, where a kernel or a scale is load-bearing, the section 7 blind audit. Its verdict is only as good as the statement: a leading or narrativized statement breaks the blind (the same load-bearing craft as section 8), and a statement that omits the mechanism fact the flaw depends on can launder the seam past the panel, so trace the neutral mechanism facts to the source before sealing them. And a framing panel that returns "sound" is a licence to build the intent, not a proof the build will be faithful.

## 11. The input-bias smoke test (the strongest-model check on the panel's own construction)

Run this on every load-bearing panel, of any type above, before its verdict is trusted. The blind protects the panelists from the repository's framing, but it moves the single point of contamination onto the panel's own construction: the packet, the sealed statement, the prompts, the lens set, and above all the CHOICE of what to look for, all written by the same sighted party whose blind spot the panel exists to catch. A construction that shares that blind spot confirms the bias in silence and returns a false "all clear" that reads as validation, which is worse than running no panel, because it launders the premise past every later step (the final verify rechecks only the findings that were flagged, so a false negative from a slanted setup reaches nothing). Sections 7 and 10 name this risk as a guard to remember; this section makes it a required, named step so it is never left to memory, because a panel pointed only at what its designer wants to check will share that designer's blind spot and report clean.

**The trick.** Before the panel's verdict is trusted, a dedicated agent on the STRONGEST available model (Opus at maximum reasoning) audits the panel's own CONSTRUCTION, never the artifact under review. It is handed the whole setup: the packet or sealed statement, every panelist prompt, the lens definitions, and, stated plainly, what the designer is hoping or expecting the panel to conclude. It has two jobs, a generic-ifier pass then a bias audit, and its central question is one: where does this construction lead the witness or share the designer's blind spot? The strongest model earns its place here, because catching subtle shaping in one's own setup is a hard adversarial task and a weaker model shares more of the designer's convenient assumptions.

**The generic-ifier pass (its first job).** The smoke test strips the inputs to their neutral core: it removes every comment, gloss, narrative aside, motivating example, and framing line that is not either what the code does in operation or what the concept at its core is. A comment that asserts the property under audit, a variable name or label that editorializes the intended reading, a sentence that motivates the conclusion the designer wants, each is a place bias sneaks in through the text rather than through the structure, and each is deleted. What the panel then sees is the mechanism and the claim, and nothing of anyone's belief that it is right. This makes active the discipline section 7 states for its Section B (leave the code's self-assessing comments out) and section 10 for its sealed statement (de-narrativized, carrying no author conclusion): the smoke test performs that stripping itself rather than trusting the designer to have done it, and it reports what it removed so the removal is auditable and a load-bearing fact is never stripped along with the steering.

**What it hunts (its second job, the bias audit).** A leading frame: a statement or prompt phrased to elicit the answer the designer wants, or a presupposition baked in. An omitted falsifier: the evidence, code, context, or measured quantity a panelist would need to DISPROVE the hypothesis and that the packet leaves out (a run log that never measures the concept is itself the finding). A curated input: an excerpt or spec restatement selected to support the wanted conclusion. And the sharpest one, the lens set that shares the blind spot: a panel told to look only for the failures the designer already suspects can never find the failure the designer cannot see. So the smoke test's load-bearing question is the negation of the designer's: what would a hostile outsider who does NOT share this goal check that this construction omits, and what failure mode does the setup never name?

**The protocol.** Run the smoke test on the strongest model against the full construction before the panel runs, or in parallel with a hold on trusting the verdict. A construction it clears may run and its verdict be trusted; a construction it flags is fixed, the missing falsifier folded in, the leading frame neutralized, the omitted lens added, the steering comment stripped, and the panel re-run. A panel verdict of "sound" or "clean" is not trusted until the smoke test has cleared its inputs. For the most load-bearing runs, run the smoke test across diverse models too, since it is itself a construction and one strong agent is one voice, the same independence discipline the other sections require.

**It fails closed.** The smoke test is a hard gate, and only an explicit CLEAR verdict on genericized inputs lets the panel proceed. Every other outcome blocks it: the agent cannot decide, the model errors or times out, the output is missing or malformed, or the construction cannot be genericized without losing a fact the panel needs. On any of these the workflow does NOT continue, the construction is reworked, and the smoke test is re-run, and the panel is never launched on inputs the smoke test did not clear. A check that did not run or could not decide is read as a block, never as a pass, because a silent allow-through on a failed or absent meta-check is the exact failure this whole method exists to prevent, and a workflow that treats an errored smoke test as a green light has defeated its own guard. Where the harness allows it, make the block mechanical (the panel step refuses to launch until the smoke test has emitted CLEAR) rather than a matter of the author remembering.

**Its own limit, and where the recursion stops.** The smoke test is subject to the very flaw it hunts: it is a construction, and a designer who frames it to be reassured gets reassurance. Two things bound this. The negation charge is structural rather than a checklist: it is told to find what the setup omits, so it works against the designer's frame instead of within it. And the recursion bottoms out at the owner, who is the backstop for any load-bearing verdict; a smoke test that clears a panel is a licence to trust the panel, never a proof, and a result the owner would want to see is surfaced, not buried under a green panel.

## 12. The fetch sub-agent's standing rule: vendor the source, or the number has no provenance (owner-ruled, 2026-07-17)

EVERY FETCH SUB-AGENT READS THIS AT SESSION START, before it produces a single load-bearing number, and the agent that spawns a fetch sub-agent points it here in the spawn prompt. This is a standing, inter-agent requirement, the same standing as the section 9 lenses.

THE RULE. A fetch that produces a number MUST VENDOR ITS SOURCE: download the actual document or data file, checksum it (SHA256, since md5 collisions are practically constructible and a collision-forgeable receipt is not tamper-evident, the one property a receipt exists to have; an md5 may ride as a legacy secondary), and hold it in the repo behind the manifest (`docs/working/VENDORING_CHECKLIST.md`), at fetch time, by the agent who knows what the artifact is. A URL citation is NOT provenance. A link rots, a page moves, a server retires, and the number is then a claim with no held witness. The receipt is a byte, never a promise.

THE LICENSING CARVE-OUT, the runbook's Phase-0 logic applied at fetch time. A paywalled or restricted-license document gets the full receipt, hash, citation, and anchor recorded like any other, but its bytes are held in the RESTRICTED STORE and never committed to the public tree, migrating into the vault mechanism when the custody runbook lands. The provenance is complete either way; only the bytes' custody changes to honor the license. An open document (the Byerlee case) is vendored into the public tree directly.

WHY THIS IS THE WHOLE POINT, not a new burden. The project's provenance discipline exists so every value traces to a byte a later reader can open. A fetch that records "read from [URL]" and enters the number has produced a value whose provenance is a hyperlink, which is the exact gap the discipline closes. THE FOUNDING CASE: the Byerlee 1978 friction fetch recorded a USGS URL and read the coefficients from the paper's text, but never vendored the PDF; when the low-stress band was later needed, no held page existed to read, and the work had to re-fetch and hash the source (md5 `cced4f8db5ee5a9e78c0459f0b5f0b3e`) before it could proceed. The numbers were right; their provenance was a promise. That is the failure this rule ends.

THE FETCH'S OUTPUT, in full, for every value: the VENDORED source (by md5), held behind the manifest; the ANCHOR, the exact page, figure, table, or equation the value is read from; DUAL-CHANNEL where the standard requires it, two independent reads (text and figure, or two methods) that agree, so a single mis-read cannot pass; and the SCOPE, the regime the value applies to, read from the primary's own structure, because a value's domain is part of the value. Byerlee's low-stress scatter (coefficient 0.3 to 10) is the sub-5-MPa regime, not the whole sub-200-MPa branch, and a fetch that shipped the envelope without its scope would have seated a scatter forty times too deep.

READ THE PRIMARY, NOT THE ABSTRACT. The headline in an abstract is a summary; the figures and tables carry the scope, the scatter, and the regime boundaries. A fetch that reads only the abstract produces a number stripped of its qualifiers, which is a number stripped of its truth. A fetch sub-agent that has not vendored its source, anchored its value, and read its scope has not finished its fetch.

THE CHANNEL COROLLARY (owner-ruled 2026-07-18, tightened 2026-07-20): a number that arrives through the CHANNEL, relayed by a spawning agent, a prior summary, or a research report rather than read from held bytes, is a FETCH-SPEC SEED, never a citation and never canonical code. It scopes the fetch (which source to open, which figure to read, what value to expect) but may not ride the canonical path as a provisional. VENDORED BYTES CLOSE EVIDENCE CUSTODY ONLY: the source must be downloaded, checksummed, held behind the manifest, and read at its exact anchor, then the candidate must still derive or pass the complete absolute-floor admission process. The founding Larson-Penston case remains useful history for custody, but its vendored number alone would not grant canonical admission under the current rule.

THE EXTERNAL-CLAIM COROLLARY (owner-ruled 2026-07-22): finding a discrepancy does not authorize an accusation or author contact. Record parallel values, exact quotations, unreadable glyphs, and project refusals neutrally. Any assertion that an external author, publication, dataset, table, identifier, or result is erroneous, invalid, misattributed, retracted, or needs correction, including a contact phrased as a question, must pass `docs/working/EXTERNAL_ADVERSE_CLAIM_RULE.md`. The release requires five independent evidence lineages beyond the subject artifact, exact text and destination digests, an owner signature, and agreement from the independent watchdog. There is no waiver or single-witness route.
