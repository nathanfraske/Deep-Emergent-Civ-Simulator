// panels-reviewed (this template IS the section-10 blind framing panel; the reminder hook passes)
//
// The section-10 BLIND FRAMING PANEL (AGENTIC_ADDENDUM.md). Diverse isolated agents critique one
// design-framing STATEMENT against the guiding principles alone. Pass the statement and the minimal neutral
// mechanism facts as `args`:
//   args = { statement: "<the raw, de-narrivatized framing to attack>",
//            mechanismFacts: "<minimal neutral context, traced to source, no conclusion>" }
// The statement MUST carry no author or owner conclusion and no hint of any suspected flaw. Returns each
// panelist's verdict; YOU verify the decisive claim against source and synthesize the corrected framing.
export const meta = {
  name: 'blind-framing-panel',
  description: 'Diverse isolated panelists critique one design-framing statement against the guiding principles alone',
  phases: [{ title: 'Blind critique', detail: 'independent panelists, same statement, no shared context, no leaked conclusion' }],
}

const STATEMENT = (args && args.statement) || '(no statement provided in args.statement)'
const MECHANISM = (args && args.mechanismFacts) || '(no neutral mechanism facts provided in args.mechanismFacts)'

const PACKET = `
You are one of several independent reviewers, each working ALONE. You cannot see the others and there is no
"correct" answer handed to you. Judge the STATEMENT below against the GUIDING PRINCIPLES alone. Find the
strongest possible objection to the framing and, if you can, propose a BETTER framing. Do not assume the
statement is right. Attack its weakest point first.

=== GUIDING PRINCIPLES (a deterministic emergent-world simulator; these govern everything) ===
- P8 (emergence, never templated): order must ARISE from rules, never be imposed from outside the simulation.
  A closed enum, a lookup table, or an authored rule where world content should emerge is a defect.
- P9: the world's PHYSICS may be an authored input; a CULTURAL OUTCOME may not be authored. An innate
  disposition or a physical constant is a legitimate authored input; a specific belief a being ends up holding
  must emerge, never be scripted.
- P10: observer independence (readers never write canonical state).
- P11: data-driven by default; a hardcoded constant in the path of world content is a defect until it earns
  its place. Membership (which axes, which categories) is data; only the mechanism is fixed code.
- THE VALUE-AUTHORING LINE (absolute): a value may be authored ONLY in the physics floor (material axes and
  law constants). Everywhere else it must be DERIVED from the floor and the situation, or read as world data.
- ADMIT THE ALIEN: every mechanism must be feasible for a non-Terran / magical / silicon / photosynthetic
  being as DATA, not a rewrite. Key on the being's OWN data, never a fixed category.
- THE TEMPLATE CASE (the sharpest test): kin-biased cooperation must NOT be produced by reading genetic
  relatedness (that authors Hamilton's rule as a MECHANISM). It must EMERGE because a being helps the familiar
  and nearby, which merely CORRELATE with relatedness, so the rule becomes a DESCRIPTION of the outcome, never
  a coded shortcut. General form: if a mechanism reads a high-level fact (relatedness, group membership, a
  named emotion, a status level) to produce a behaviour, it is authoring. Replace it with a general causal
  PRIMITIVE plus a PROXY that correlates, and let the outcome emerge from selection or learning.

=== MECHANISM CONTEXT (neutral facts, no conclusions) ===
${MECHANISM}

=== THE STATEMENT TO EVALUATE ===
${STATEMENT}

=== YOUR TASK ===
Assess ONLY this statement against ONLY the principles above. In particular: does anything quietly AUTHOR a
coupling the principles say must emerge (a hidden lookup, a fixed category, a high-level fact read to produce
an outcome)? Would it hold for an alien being as data? Where is it weakest, and what is the single best
improvement? Give your verdict and, if you have one, your improved framing. Be concrete and terse; commit to
your strongest objection.
`

const SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    verdict: { type: 'string', enum: ['sound-as-is', 'minor-improvement', 'significant-flaw-fixable', 'reframe-needed'] },
    strongest_objection: { type: 'string' },
    principle_most_at_risk: { type: 'string' },
    alien_test: { type: 'string' },
    proposed_framing: { type: 'string' },
  },
  required: ['verdict', 'strongest_objection', 'principle_most_at_risk', 'proposed_framing'],
}

// Diverse roster: different agent TYPES and MODELS, each isolated, each seeing the identical packet.
const ROSTER = [
  { agentType: 'general-purpose', model: 'opus', label: 'panel:gp-opus' },
  { agentType: 'general-purpose', model: 'sonnet', label: 'panel:gp-sonnet' },
  { agentType: 'Plan', model: 'opus', label: 'panel:plan-opus' },
  { agentType: 'Plan', model: 'sonnet', label: 'panel:plan-sonnet' },
  { agentType: 'claude', model: 'fable', label: 'panel:claude-fable' },
  { agentType: 'claude', model: 'sonnet', label: 'panel:claude-sonnet' },
]

const verdicts = await parallel(
  ROSTER.map((r) => () =>
    agent(PACKET, { label: r.label, phase: 'Blind critique', schema: SCHEMA, agentType: r.agentType, model: r.model, effort: 'high' })
      .then((v) => (v ? { panelist: r.label, ...v } : null))
  )
)

return { panelists: ROSTER.length, verdicts: verdicts.filter(Boolean) }
