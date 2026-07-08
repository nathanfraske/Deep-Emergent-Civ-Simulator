// panels-reviewed (this template runs the five section-9 mandatory lenses; the reminder hook passes)
//
// The section-9 MANDATORY LENS AUDIT (AGENTIC_ADDENDUM.md) plus correctness, over a change. Pass the change
// context as `args`:
//   args = { context: "<what changed, the files/mechanisms under audit, the diff path, the byte-neutrality
//                       and determinism claims to check, established facts to verify not assume>" }
// Six blind panelists (the five standing lenses + correctness), then an adversarial verify per finding
// (default REFUTED unless substantiated at the cited file:line). YOU verify each survivor against source.
export const meta = {
  name: 'mandatory-lens-audit',
  description: 'The five mandatory lenses + correctness over a change, each independent, with per-finding verify',
  phases: [{ title: 'Review', detail: 'six independent lens panelists' }, { title: 'Verify', detail: 'adversarial per-finding verification against source' }],
}

const CONTEXT = `
You are auditing a change to a deterministic emergent-world simulator (Rust). Read the ACTUAL source; do not
trust any summary. Report only findings you can tie to a specific file:line in the current source; if you
cannot substantiate a finding against source, do not report it.

CHANGE UNDER AUDIT:
${(args && args.context) || '(no context provided in args.context; run `git diff` and read the changed files)'}
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

const reviews = await parallel(
  LENSES.map((l) => () =>
    agent(`${CONTEXT}\n\n${l.prompt}\n\nReturn your lens key ("${l.key}") and your findings.`,
      { label: `review:${l.key}`, phase: 'Review', schema: FINDING_SCHEMA })
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
  panelists: reviews.filter(Boolean).length,
  clean_notes: cleanNotes.map((c) => `[${c.lens}] ${c.claim}`),
  confirmed_findings: survived.sort((a, b) => (order[a.corrected_severity] ?? 9) - (order[b.corrected_severity] ?? 9)),
}
