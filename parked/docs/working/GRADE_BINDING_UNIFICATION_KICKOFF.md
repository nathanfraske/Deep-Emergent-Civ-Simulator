# Grade-binding unification: Agent A's next-arc kickoff (doc-only)

This is a doc-only bridge PR off current `main`. It authors no mechanism and moves no value.
Its purpose is to keep Agent A's watch alive after the stroke-rate STEP 2 arc merges (PR #124),
so the gate can reach this session and sequence the next work. The build lands on this branch
when the gate calls it, after #124 merges (so the branch carries step 2's grade and delivery
kernels).

## What just completed (context, not this PR's work)

The STROKE-RATE / LIMB-BIOMECHANICS SUBSTRATE step 2 (the actuation-kind substrate) is complete
and signed off on #124: a segment's delivered mechanical energy is the run-all-gate-to-zero MAX
over three shared-source mechanical kernels, the rigid `F d` (Kinetic), the elastic recoil
(ElasticRecoil), and the hydraulic `P dV` (Hydraulic, which composes from the existing floor laws
with no new kernel). Which member fires derives from which continuous grown axes are nonzero,
never a categorical selector. Byte-neutral (the four pins hold), section-9-hardened across four
slices.

## The arc: the grade-binding unification (the gate's priority follow-on, ruled at slice 3)

The seam three section-9 lenses flagged across step 2: the capability GRADE path binds its axis
ids POSITIONALLY (`FunctionLawDef`'s `geometry_axes` / `material_axes` `Vec<String>` order), while
the delivery path binds them by NAMED fields (`ContactTransfer`'s `strength_axis`,
`cross_section_axis`, `stroke_axis`, `yield_axis`, `elastic_modulus_axis`, `pressure_axis`). The
two coincide only BY CONSTRUCTION, with nothing in code linking the positional order of one to the
named fields of the other. Two consequences on record:

- A hand-built or alien IMPACT binding that omits an absent leading axis (a purely-springy
  actuator writing `[yield_id, modulus_id]` with no rigid strength) is positionally mis-read, its
  yield landing in the strength slot and fabricating a rigid blow. Admit-the-alien fails for the
  natural authoring; the caller must know to supply a zero-reading placeholder at each absent slot.
- The grade-to-delivery lockstep (the gate's slice-3 ruling iv) is pinned only by a test
  (`the_impact_grade_binding_and_the_delivery_row_name_the_same_axes_in_lockstep`) plus a
  positional-contract doc caveat, not mechanically enforced; a reorder of either side silently
  desyncs the grade from the delivery with no compile error.

Only the IMPACT kernel was lifted to data bindings in step 2 (slice 2); the other five capability
kernels (Pierce, Locomote, Refract, Shear, Crush) still read their hardcoded axis contract. The
gate ruled the clean fix (deferred from slice 2, raised to PRIORITY at slice 3 by step 2's fourth
positional axis): unify ALL SIX capability-grade kernels to named-field bindings (or one shared
binding type) in a single pass, so the lockstep is MECHANICALLY enforced and the positional-contract
caveat retires. Doing it for IMPACT alone would leave it the odd kernel out.

## Scope (not mechanism, for the gate's ruling before code)

1. Lift the six capability-grade kernels off the positional `Vec<String>` axis contract onto NAMED
   bindings, either named fields per axis role or one shared binding type both paths use, so the
   grade path and the delivery-path `ContactTransfer` row reference the same axis-role names.
2. Retire the positional-contract caveat in `capability.rs::impact` and the drift-test in
   `contact_transfer.rs` once the lockstep is mechanically enforced (a shared binding type makes a
   desync a type error, not a test failure).
3. Byte-neutral: the default bindings equal today's positional contract, so every capability score
   and the four pins are bit-identical; the change is a data-representation refactor with no new
   mechanism and no value.

## Discipline

This is a data-representation refactor (no new floor axis, no new law, no value in the world-content
path), so on first read it does not need a frame-blind (it authors no emergence-critical design; it
hardens an existing substrate). I will confirm that read with the gate before code: if the gate
agrees, the arc is byte-neutral build plus the mandatory section-9 five-lens over the diff; if the
gate wants a frame-blind on the binding-type choice, it goes through section-11 then section-10
first. No code until the gate rules the scope and the frame-blind question.

## Flagged siblings (out of this arc, on record)

- The tool-delivery arc (POUND on the WIELDER's delivered energy concentrated over the tool's
  contact area, `mech.mass` a live read there): the gate ruled it the arc AFTER this one.
- The source-independence datum (so an osmotic/turgor-charged hydrostat or spring routes additively
  rather than being MAX-folded), the compressible/pneumatic `P dV` kernel, and the dedicated
  fluid-channel / elastic-element volume axes: the step-2 reserved refinements.
- The resistance-kernel registry (the wound-side sibling of the delivery-path kernel set) and the
  deep-`Body` `body.rs:1108` swing-velocity seam.

No mechanism, no value moved. Details above; the build lands here when the gate calls it.
