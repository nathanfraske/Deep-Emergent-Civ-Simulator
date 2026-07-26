// panels-reviewed
// input-bias-smoke-cleared
// panel-smoke-protocol-v1
//
// The section-10 BLIND FRAMING PANEL (AGENTIC_ADDENDUM.md). Diverse isolated agents critique one
// design-framing STATEMENT against the guiding principles alone. Pass the statement and the minimal neutral
// mechanism facts as `args`:
//   args = { statement: "<the raw, de-narrivatized framing to attack>",
//            mechanismFacts: "<minimal neutral context, traced to source, no conclusion>",
//            hopedConclusion: "<the outcome the designer hopes the panel reaches>",
//            smokeHistory: [{ defect_class, correction, verdict_consequence,
//                             materiality_disputed }] } // required; use [] on the first run
// The statement MUST carry no author or owner conclusion and no hint of any suspected flaw. Returns each
// panelist's verdict; YOU verify the decisive claim against source and synthesize the corrected framing.
export const meta = {
  name: 'blind-framing-panel',
  description: 'Diverse isolated panelists critique one design-framing statement against the guiding principles alone',
  phases: [
    { title: 'Smoke', detail: 'fail-closed genericization and construction audit' },
    { title: 'Blind critique', detail: 'independent panelists, same neutral packet, no shared context' },
  ],
}

// The harness may deliver `args` as a JSON STRING rather than a parsed object; normalize either way so the
// statement and neutral mechanism facts actually reach the panelists (a silent empty packet critiques nothing).
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { A = {} }
}

const STATEMENT = (A && A.statement) || '(no statement provided in args.statement)'
const MECHANISM = (A && A.mechanismFacts) || '(no neutral mechanism facts provided in args.mechanismFacts)'

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

if (!A || !Array.isArray(A.smokeHistory))
  return { status: 'SMOKE_HISTORY_REQUIRED', panelists: 0 }
const correction_history = A.smokeHistory
if (!correction_history.every((entry) =>
  entry
  && typeof entry.defect_class === 'string'
  && entry.defect_class.trim()
  && typeof entry.correction === 'string'
  && typeof entry.verdict_consequence === 'string'
  && typeof entry.materiality_disputed === 'boolean'
))
  return { status: 'SMOKE_HISTORY_MALFORMED', panelists: 0 }
if (correction_history.some((entry) => entry.correction === 'PENDING_CORRECTION'))
  return { status: 'SMOKE_CORRECTION_REQUIRED', panelists: 0, next_smoke_history: correction_history }
if (correction_history.some((entry) => entry.materiality_disputed))
  return { status: 'OWNER_REVIEW', panelists: 0, next_smoke_history: correction_history }
const SMOKE_SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    verdict: { type: 'string', enum: ['CLEAR', 'BLOCK', 'OWNER_REVIEW'] },
    genericized_input: { type: 'string' },
    neutralized_notes: { type: 'array', items: { type: 'string' } },
    material_defects: {
      type: 'array',
      items: {
        type: 'object',
        additionalProperties: false,
        properties: {
          defect_class: { type: 'string' },
          evidence: { type: 'string' },
          verdict_consequence: { type: 'string' },
          required_change: { type: 'string' },
          substantial: { type: 'boolean' },
          repeats_prior_class: { type: 'boolean' },
          new_consequence: { type: 'boolean' },
          materiality_disputed: { type: 'boolean' },
        },
        required: [
          'defect_class', 'evidence', 'verdict_consequence', 'required_change', 'substantial',
          'repeats_prior_class', 'new_consequence', 'materiality_disputed',
        ],
      },
    },
  },
  required: ['verdict', 'genericized_input', 'neutralized_notes', 'material_defects'],
}

const SMOKE_PROMPT = `
You are the strongest-model section-11 input-bias smoke gate. Audit this framing-panel construction itself,
including the raw statement, neutral facts, guiding-principle packet, every panel prompt, the roster, and the
conclusion its designer hopes to see. Your charge is the negation of the designer's: identify what a hostile
outsider would need to falsify the framing that this setup omits.

First GENERICIZE. Produce genericized_input containing the complete neutral packet the panel should see.
Remove or neutralize minor wording, surplus context, harmless over-inclusion, motivating narrative, and
non-load-bearing metadata yourself. Record each neutralization in neutralized_notes. Those issues are CLEAR
notes and do not require the author to rebuild the packet.

Return BLOCK only when a defect cannot be neutralized and requires a substantial correction because a
load-bearing fact or falsifier is absent, panel-visible steering remains, source traceability is broken, or
the lens roster cannot reach a consequential failure class. Every material defect must name its defect_class,
direct evidence, the concrete substantive panel verdict it could change, and the required change. Mark
substantial false for anything you can genericize. For a defect class already present in correction_history,
set repeats_prior_class true. Such a repeat may block only with a new concrete verdict_consequence and
new_consequence true. If materiality is disputed, return OWNER_REVIEW rather than requesting another
semantically equivalent packet.

RAW CONSTRUCTION:
${PACKET}

ROSTER:
${JSON.stringify(ROSTER)}

DESIGNER'S HOPED-FOR CONCLUSION:
${(A && A.hopedConclusion) || '(not supplied; treat omission as visible context, not a desired answer)'}

CORRECTION HISTORY:
${JSON.stringify(correction_history)}
`

const smoke = await agent(SMOKE_PROMPT, {
  label: 'smoke:input-bias',
  phase: 'Smoke',
  schema: SMOKE_SCHEMA,
  agentType: 'general-purpose',
  model: 'opus',
  effort: 'max',
})

if (
  !smoke
  || !['CLEAR', 'BLOCK', 'OWNER_REVIEW'].includes(smoke.verdict)
  || typeof smoke.genericized_input !== 'string'
  || !Array.isArray(smoke.neutralized_notes)
  || !smoke.neutralized_notes.every((note) => typeof note === 'string')
  || !Array.isArray(smoke.material_defects)
)
  return { status: 'SMOKE_MALFORMED', panelists: 0 }

const normalizeConsequence = (value) =>
  String(value || '').trim().toLowerCase().replace(/\s+/g, ' ')
const priorClasses = new Set(correction_history.map((entry) => entry.defect_class))
const priorConsequences = new Map()
for (const entry of correction_history) {
  if (!priorConsequences.has(entry.defect_class))
    priorConsequences.set(entry.defect_class, new Set())
  priorConsequences.get(entry.defect_class).add(normalizeConsequence(entry.verdict_consequence))
}
const ownerDisputed = new Set(
  correction_history
    .filter((entry) => entry && entry.materiality_disputed === true)
    .map((entry) => entry.defect_class)
)
const blockers = []
for (const defect of smoke.material_defects) {
  if (
    !defect
    || typeof defect.defect_class !== 'string'
    || !defect.defect_class.trim()
    || typeof defect.evidence !== 'string'
    || typeof defect.verdict_consequence !== 'string'
    || typeof defect.required_change !== 'string'
    || typeof defect.substantial !== 'boolean'
    || typeof defect.repeats_prior_class !== 'boolean'
    || typeof defect.new_consequence !== 'boolean'
    || typeof defect.materiality_disputed !== 'boolean'
  )
    return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }
  const repeats = priorClasses.has(defect.defect_class)
  if (defect.repeats_prior_class !== repeats)
    return { status: 'SMOKE_HISTORY_MISMATCH', panelists: 0, smoke }
  if (defect.materiality_disputed || ownerDisputed.has(defect.defect_class)) {
    const next_smoke_history = [
      ...correction_history,
      {
        defect_class: defect.defect_class,
        correction: 'OWNER_REVIEW_REQUIRED',
        verdict_consequence: defect.verdict_consequence,
        materiality_disputed: true,
      },
    ]
    return { status: 'OWNER_REVIEW', panelists: 0, smoke, next_smoke_history }
  }
  if (!defect.substantial) {
    smoke.neutralized_notes.push(`[genericized:${defect.defect_class}] ${defect.evidence}`)
    continue
  }
  const normalizedConsequence = normalizeConsequence(defect.verdict_consequence)
  const consequenceIsNew = defect.new_consequence
    && normalizedConsequence
    && !priorConsequences.get(defect.defect_class).has(normalizedConsequence)
  if (repeats && !consequenceIsNew) {
    smoke.neutralized_notes.push(
      `[repeat-without-new-consequence:${defect.defect_class}] ${defect.evidence}`
    )
    continue
  }
  if (!defect.verdict_consequence.trim() || !defect.required_change.trim())
    return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }
  blockers.push(defect)
}

if (smoke.verdict === 'OWNER_REVIEW')
  return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }
if (smoke.verdict === 'BLOCK' && smoke.material_defects.length === 0)
  return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }
if (smoke.verdict === 'CLEAR' && blockers.length > 0)
  return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }

smoke.effective_verdict = blockers.length > 0 ? 'BLOCK' : 'CLEAR'
if (smoke.effective_verdict !== 'CLEAR') {
  const next_smoke_history = [
    ...correction_history,
    ...blockers.map((defect) => ({
      defect_class: defect.defect_class,
      correction: 'PENDING_CORRECTION',
      verdict_consequence: defect.verdict_consequence,
      materiality_disputed: false,
    })),
  ]
  return { status: 'SMOKE_BLOCK', panelists: 0, smoke, blockers, next_smoke_history }
}
if (!smoke.genericized_input.trim())
  return { status: 'SMOKE_MALFORMED', panelists: 0, smoke }

const GENERICIZED_PACKET = smoke.genericized_input
const verdicts = await parallel(
  ROSTER.map((r) => () =>
    agent(GENERICIZED_PACKET, { label: r.label, phase: 'Blind critique', schema: SCHEMA, agentType: r.agentType, model: r.model, effort: 'high' })
      .then((v) => (v ? { panelist: r.label, ...v } : null))
  )
)

return {
  smoke: {
    verdict: smoke.effective_verdict,
    neutralized_notes: smoke.neutralized_notes,
    smoke_history: correction_history,
  },
  panelists: ROSTER.length,
  verdicts: verdicts.filter(Boolean),
}
