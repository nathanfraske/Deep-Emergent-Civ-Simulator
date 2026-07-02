# Agentic Addendum: Panels, Hooks, MCP, and Memory Persistence

This addendum specifies the agentic infrastructure for continuing the project inside a coding-agent harness. The reference harness is Claude Code, whose hook lifecycle, MCP configuration, and `CLAUDE.md` auto-loading are its native mechanisms; the requirements are stated so they map to any equivalent harness (the Claude Agent SDK exposes the same lifecycle as in-process callbacks, and Codex CLI ports the same JSON-on-stdin protocol). Nothing here touches the simulation engine's determinism: hooks and servers operate on the documents and the repository, never on the canonical core.

The goal is narrow and practical. The customs in `CLAUDE.md` are enforced by the agent's judgment; this addendum adds a mechanical safety net so a customs violation or a half-finished consolidation cannot slip through, and so the review panels have real data to render.

---

## 1. Memory persistence: the three files

Memory is persisted in three plain Markdown files at the repository root, each with a distinct role and lifecycle.

**`CLAUDE.md` (static; the operating manual).** Claude Code loads `CLAUDE.md` automatically into every session as project memory, so the customs, workflows, and verification suite are always in context. It changes rarely, only when a custom or workflow changes in substance, and any such change is the owner's call. Treat it as read-mostly.

**`HANDOFFS.md` (rolling; the session log).** An append-only, reverse-chronological log. Each session adds one dated entry: what was done, what changed in the documents, where the session stopped, and what is queued next. It is the first thing the next session reads to recover state without re-deriving it. Never rewrite history in it; append.

**`TODOS.md` (live; the backlog mirror).** A parseable mirror of the research backlog (the open items in audit Section 3, in working order, with their couplings) and the reserved-values review queue. It is updated every session: resolved items move out, new flags move in, order is adjusted. It is the source the backlog panel parses, so its format stays stable (see Section 5).

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
- Tool `backlog`: parses `TODOS.md` and audit Section 3 into the open-items list with order, couplings, and status.
- Tool `reserved`: parses the calibration manifest (runbook Section on reserved values) into the review queue: each value with its id, basis, status, and source document reference.
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

**Backlog panel.** Shows the open research items in working order, with their couplings and their resolved-or-open status, and the reserved-values review queue alongside. Requires `TODOS.md` to keep a stable format (Section 5) and the `projectops backlog` and `reserved` tools to parse it and the audit Section 3.

**Reserved-values panel.** Shows the calibration manifest as a queue: each value with its id, basis, status (reserved or set), and the document reference where its mechanism lives. Requires the manifest to be a parseable file with a fixed schema (runbook), and the `projectops reserved` tool. This is the surface the owner uses to work through the numbers the agent has surfaced.

**Consolidation panel.** Shows, for the item worked this session, whether its resolution is complete: flag replaced, record added to Part 62, bibliography group added to Part 63, backlog bullet rewritten, counts moved by the right amount. Requires the `projectops consolidation_check` tool. This catches the most common incomplete-resolution mistakes before the session ends.

The common requirement, stated once: for any panel to work, the verification suite must exist as a callable script or tool emitting structured output (not ad-hoc greps), and the parseable artifacts (`TODOS.md`, the calibration manifest, the audit Section 3, the part and record headings) must hold stable formats. Those two things are the agentic features that make panels function; everything else is rendering.

---

## 5. Stable formats the panels depend on

So the tools can parse without guesswork, hold these conventions:

- **Part and record headings** stay `## Part N: Title` and `### N.M`, gapless, as the verification suite already assumes.
- **Open backlog bullets** start `- **R-XXX.`; resolved bullets start with a plain word and contain `(R-XXX): resolved.`. This is the convention the open-count grep and the backlog tool both rely on.
- **`TODOS.md`** lists each open item as a single bullet with the identifier first, the one-line question, the working-order rank, and the couplings, in a fixed order, so the backlog tool reads it deterministically.
- **The calibration manifest** is a structured file (the runbook specifies the schema): one entry per reserved value, each with id, basis, status, value when set, who set it, and the source document reference.

---

## 6. The lifecycle, end to end

A session runs: `SessionStart` loads the handoff tail and the open todos and establishes a clean verification baseline, re-running on resume so the baseline is never stale. The agent works under the `PreToolUse` customs guard, which blocks an em dash or a banned adverb from ever entering a maintained document, and the `PostToolUse` check, which surfaces a slip the moment it happens. As it finishes, the agent updates `HANDOFFS.md` and `TODOS.md`. The `Stop` gate then refuses to let the turn end until the full suite passes and the memory files are current, guarding against a loop with the `stop_hook_active` check. `SessionEnd` optionally persists a closing marker. The panels read structured results from `projectops` throughout, so the maintainer sees verification status, the backlog, the reserved-values queue, and consolidation completeness without leaving the harness.

The division of labour is deliberate: the agent's judgment does the work and runs the suite itself, and the hooks are the mechanical backstop that makes a customs violation or a half-finished consolidation impossible to ship rather than merely discouraged.

---

## 7. The fully-blind audit (for uncontaminated correctness and value verdicts)

Reach for this when a verdict must not be contaminated by the repository's own framing: verifying that kernels, formulas, or reserved values are correct, where a shared blind spot between the code and its own tests could hide a defect. A test written to match a buggy output, a comment that rationalizes an error, or a prior sighted review that inherited the same assumption will all pass a normal review, because they were written against the same premise. A blind auditor, given only what the code claims and what it is built on, reaches its verdict from first principles and does not inherit that premise. This method found scale bugs in merged, reviewed kernels (a missing megapascal-to-pascal promotion that made a buckling load a million times too small) that the sighted red-teams and the passing test suite both missed, because the tests encoded the same error.

**The packet.** The auditor sees only a self-contained packet, written to the scratchpad outside the repository, with three sections. Section A is the substrate contract: the exact semantics of every type and primitive the code calls (bit widths, scales, rounding direction, which operations panic or wrap or saturate, the determinism rules), so totality, overflow, and rounding are judgeable without the source; A also states the audit checklist (physical faithfulness, dimensional soundness, totality, precision discipline, fabrication and steering) and any non-obvious unit or scale conventions the code assumes. Section B is the code under audit, the functions only, with the tests, the prior audits, the design docs, and the cross-references all left out. Section C is the declared specification: what each unit is supposed to compute, its inputs with their units, dimensions, and ranges, and its declared output and bound. The packet carries the code and what it claims, and nothing of the repository's proof that it is correct.

**The protocol.** First pilot with one agent: is the packet sufficient to reach a conclusion with zero repository access? The pilot audits a spanning sample and reports, precisely, any information the packet lacked. Fold those gaps into the packet before spending on a panel, so the panel is not blocked on a missing contract. Then run the full panel: several independent auditors, each blind, each reading only the packet, none aware of the others or of any prior conclusion, each classifying per the checklist. To test whether they converge (the point of the exercise), double-cover the arithmetic-heavy slices or run independent passes and compare. Then verify every flagged finding against the real code yourself before trusting it: the auditor is blind and may misread an intended convention, so a finding is a lead to prove, not a verdict to accept (Prime Directive 1). Reconcile: convergence across independent blind auditors is strong evidence, and a finding that survives your own check against the source is a real defect to surface and fix.

**Enforcing the blind.** Instruct each auditor to read only the packet file and to not read, grep, glob, or open any file under the repository. If an auditor feels it cannot judge a kernel without the repository, it must not go and get it: it records exactly what the packet was missing, which sharpens the packet rather than breaking the blind.

**Model tiering.** Panelists run on the cheapest model that accomplishes the goal: Sonnet for reasoning-level correctness audits, Haiku when the work is sheer mass over many simple units, and Opus reserved for the hardest kernels that need full depth. The pilot and the per-finding verification are the same tiering call.

The vehicle of a run is the packet plus the panel outputs, kept in the scratchpad; a durable finding graduates into a fix and, if it reveals a class of defect, into a checklist item the next packet's Section A names by default.
