// panels-reviewed
// input-bias-smoke-cleared
// panel-smoke-protocol-v1
//
// The section-9 MANDATORY LENS AUDIT (AGENTIC_ADDENDUM.md) plus correctness, over a change. Pass the change
// context as `args`:
//   args = { context: "<what changed, the files/mechanisms under audit, the diff path, the byte-neutrality
//                       and determinism claims to check, established facts to verify not assume>",
//            hopedConclusion: "<the outcome the designer hopes the panel reaches>",
//            smokeHistory: [{ defect_class, correction, verdict_consequence,
//                             materiality_disputed }] } // required; use [] on the first run
// The strongest-model smoke genericizes the construction first. Six diverse blind panelists then run only
// over its neutral packet, followed by adversarial verification per finding.
export const meta = {
  name: 'mandatory-lens-audit',
  description: 'The five mandatory lenses + correctness over a change, each independent, with per-finding verify',
  phases: [
    { title: 'Smoke', detail: 'fail-closed genericization and construction audit' },
    { title: 'Review', detail: 'six independent diverse lens panelists' },
    { title: 'Verify', detail: 'adversarial per-finding verification against source' },
  ],
}

// The harness may deliver `args` as a JSON STRING rather than a parsed object; normalize either way so the
// audit context actually reaches the panelists (a silent empty context makes them audit nothing).
let A = args
if (typeof A === 'string') {
  try { A = JSON.parse(A) } catch (e) { A = {} }
}

const RAW_CONTEXT = `
You are auditing a change to a deterministic emergent-world simulator (Rust). Read the ACTUAL source; do not
trust any summary. Report only findings you can tie to a specific file:line in the current source; if you
cannot substantiate a finding against source, do not report it.

CHANGE UNDER AUDIT:
${(A && A.context) || '(no context provided in args.context; run `git diff` and read the changed files)'}
`

const FINDING_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    lens: { type: 'string' },
    findings: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        properties: {
          severity: { type: 'string', enum: ['critical', 'major', 'minor', 'nit', 'clean-note'] },
          file: { type: 'string' }, line: { type: 'integer' },
          claim: { type: 'string' }, evidence: { type: 'string' }, why_it_matters: { type: 'string' },
        },
        required: ['severity', 'file', 'claim', 'evidence'],
      },
    },
  },
  required: ['lens', 'findings'],
}

// The five owner-standing lenses (AGENTIC_ADDENDUM section 9) plus correctness.
const LENSES = [
  { key: 'confirmation-bias', prompt: `LENS: FULLY-BLIND CONFIRMATION-BIAS CATCHER. Blind to the author's APPROACH and its rationale. Find where the author confirmed a chosen approach rather than tested it: an assumption carried unquestioned from design into code and tests alike, a test written to pass the approach rather than break it, a "clearly correct" step never adversarially probed, any value/threshold tuned to make a result come out right. Attack the approach, not the implementation of it.` },
  { key: 'derive-vs-author', prompt: `LENS: DERIVE-VERSUS-AUTHOR. A value may be authored ONLY in the physics FLOOR (material axes and law constants), nowhere else. Flag every constant, threshold, rate, weight, fraction, or table in the path of world content: is it read from the floor, or should it be DERIVED from the floor and the situation? A value that is neither floor data nor a derivation is a defect; when the floor cannot yet supply it, flag the gap and propose growing the floor, never accept it as authored.` },
  { key: 'alien-feasibility', prompt: `LENS: ALIEN-FEASIBILITY. Construct a concrete adversarial alien (a photosynthetic mind, a redox- or mana-fed metabolism, a silicon body, a creature with no analogue of the modelled organ). Does the mechanism admit it as DATA, or silently assume the Terran / single-kind case? Any step hardcoding one kingdom, chemistry, body plan, energy pathway, or sensory modality where a world's creature could differ is a defect; the mechanism must key on the being's own data so the alien is a data row.` },
  { key: 'terran-bias', prompt: `LENS: TERRAN-BIAS CATCHER. Hunt Earth-specific chemistry, biology, physics, or naming baked into the mechanism where world-declared data should decide it. Ignore any comment claiming Terran-cleanness; audit the actual decision path. A hardcoded string naming a floor axis the substance itself declares is acceptable (the floor is the authored place); a hardcoded string that decides a world outcome independent of the substance's own data is a defect.` },
  { key: 'steering-principles', prompt: `LENS: STEERING & PRINCIPLES (8 and 9). Does any substrate read a high-level social or emergent fact (relatedness, family or group membership, a named emotion, a skill or status level, a trophic or kingdom tag) to produce a behaviour, rather than a general causal primitive plus a proxy that correlates and lets the outcome emerge? Does any physics-input author a cultural or emergent outcome? A closed enum or lookup table where world content should emerge is a red flag.` },
  { key: 'correctness', prompt: `LENS: CORRECTNESS. Verify the change does what it claims: determinism (seed-keyed, worker-invariant, no wall-clock/rand), byte-neutrality of any opt-in default, conservation/overflow/panic/unwrap risks, and that every claimed invariant holds against source. Flag real defects with a concrete failing input.` },
]

const ROSTER = [
  { agentType: 'general-purpose', model: 'opus', effort: 'high' },
  { agentType: 'general-purpose', model: 'sonnet', effort: 'high' },
  { agentType: 'Plan', model: 'opus', effort: 'high' },
  { agentType: 'Plan', model: 'sonnet', effort: 'high' },
  { agentType: 'claude', model: 'fable', effort: 'high' },
  { agentType: 'claude', model: 'sonnet', effort: 'high' },
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
You are the strongest-model section-11 input-bias smoke gate. Audit this panel construction itself, including
the evidence packet, every lens prompt, the diverse roster, and the conclusion its designer hopes to see.
Your charge is the negation of the designer's: identify what a hostile outsider would need to falsify the
claim that this setup omits.

First GENERICIZE. Produce genericized_input containing the complete neutral packet the panel should see.
Remove or neutralize minor wording, surplus context, harmless over-inclusion, motivating narrative, and
non-load-bearing metadata yourself. Record each neutralization in neutralized_notes. Those issues are CLEAR
notes and do not require the author to rebuild the packet.

Return BLOCK only when a defect cannot be neutralized and requires a substantial correction because a
load-bearing production surface or falsifier is absent, panel-visible steering remains, source traceability
is broken, or the lens set cannot reach a consequential failure class. Every material defect must name its
defect_class, direct evidence, the concrete substantive lens verdict it could change, and the required
change. Mark substantial false for anything you can genericize. For a defect class already present in
correction_history, set repeats_prior_class true. Such a repeat may block only with a new concrete
verdict_consequence and new_consequence true. If materiality is disputed, return OWNER_REVIEW rather than
requesting another semantically equivalent packet.

RAW CONSTRUCTION:
${RAW_CONTEXT}

LENSES AND PROMPTS:
${JSON.stringify(LENSES)}

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

const GENERICIZED_CONTEXT = smoke.genericized_input
const CONTEXT = `
You are auditing a change to a deterministic emergent-world simulator (Rust). Read the ACTUAL source; do not
trust any summary. Report only findings you can tie to a specific file:line in the current source.

GENERICIZED CHANGE UNDER AUDIT:
${GENERICIZED_CONTEXT}
`

const reviews = await parallel(
  LENSES.map((l, index) => () =>
    agent(`${CONTEXT}\n\n${l.prompt}\n\nReturn your lens key ("${l.key}") and your findings.`,
      {
        label: `review:${l.key}`,
        phase: 'Review',
        schema: FINDING_SCHEMA,
        ...ROSTER[index],
      })
  )
)

const allFindings = [], cleanNotes = []
for (const r of reviews.filter(Boolean))
  for (const f of r.findings || [])
    (f.severity === 'clean-note' ? cleanNotes : allFindings).push({ ...f, lens: r.lens })

const VERDICT_SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    verdict: { type: 'string', enum: ['CONFIRMED', 'REFUTED', 'PARTIAL'] },
    reasoning: { type: 'string' },
    corrected_severity: { type: 'string', enum: ['critical', 'major', 'minor', 'nit', 'invalid'] },
  },
  required: ['verdict', 'reasoning', 'corrected_severity'],
}

const verified = await parallel(
  allFindings.map((f) => () =>
    agent(`${CONTEXT}\n\nADVERSARIALLY VERIFY this finding against the ACTUAL source. Default to REFUTED if you cannot substantiate it at the cited file:line.\n\nLENS: ${f.lens}\nSEVERITY(claimed): ${f.severity}\nFILE: ${f.file}:${f.line || '?'}\nCLAIM: ${f.claim}\nEVIDENCE(claimed): ${f.evidence}\n\nRead the file, confirm or refute, give corrected severity.`,
      { label: `verify:${(f.file || '').split('/').pop()}:${f.line || 0}`, phase: 'Verify', schema: VERDICT_SCHEMA })
      .then((v) => ({ ...f, ...v }))
  )
)

const survived = verified.filter(Boolean).filter((f) => f.verdict !== 'REFUTED' && f.corrected_severity !== 'invalid')
const order = { critical: 0, major: 1, minor: 2, nit: 3 }
return {
  smoke: {
    verdict: smoke.effective_verdict,
    neutralized_notes: smoke.neutralized_notes,
    smoke_history: correction_history,
  },
  panelists: reviews.filter(Boolean).length,
  clean_notes: cleanNotes.map((c) => `[${c.lens}] ${c.claim}`),
  confirmed_findings: survived.sort((a, b) => (order[a.corrected_severity] ?? 9) - (order[b.corrected_severity] ?? 9)),
}
