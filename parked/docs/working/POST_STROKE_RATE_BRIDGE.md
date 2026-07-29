# Post-stroke-rate bridge: Agent A's next-arc kickoff (doc-only)

This is a doc-only bridge PR off current `main` (`bcd7d66`). It authors no mechanism and moves
no value. Its purpose is to keep Agent A's watch alive after the stroke-rate substrate merged
(PR #123, step 1 plus step-1b), so the gate can reach this session and sequence the next work.
The build lands on this branch when the gate calls it.

## What just merged (context, not this PR's work)

The STROKE-RATE / LIMB-BIOMECHANICS SUBSTRATE is complete on `main`:

- Step 1 (`laws::actuator_work` plus `laws::stress_force`): the delivered strike energy is the
  ACTUATOR WORK `F d`, the actuating force (strength stress over cross-section, promoted to
  newtons by the megapascal-to-newton bridge) times the acting part's own grown stroke, read
  per-body. The world-global `swing_velocity` scalar is retired.
- Step-1b (`MorphogenProgram::actuator_axes`): the body-development program grows
  `mech.cross_section_area` and `mech.stroke_length` per-segment, so a grown body delivers a
  non-zero `F d` blow derived from its own body. Byte-neutral (the four run_world pins hold),
  section-9-hardened, and merged non-destructively over the #121 R-SOURCE-VECTOR lift.

## Next work, in the gate's order

1. **Priority, gate-triggered: the independent blind-bias-check of the reconciliation census
   on #122.** When Agent C's category field lands and the gate calls it, Agent A blind-audits
   the three-way-test census for confirmation-bias and steering (AGENTIC_ADDENDUM sections 9 and
   10), verifying every classification against source rather than rubber-stamping it. This is the
   owner's standing anti-confirmation-bias requirement, not a per-arc choice.

2. **Then, as its own arc on this branch: STEP 2, the actuation-kind substrate.** A per-segment
   actuation-kind axis plus kernel dispatch, so a non-rigid striker (a whip, a jet, a hydrostat)
   is a data row rather than a code change. This retires the one honest alien limit the step-1b
   section-9 audit named: the IMPACT capability kernel names the two rigid-actuator axis ids, so a
   grown alien whose actuator geometry carries different ids is invisible to IMPACT until a kernel
   variant is added. Step 2 is the general fix. It is emergence-critical, so it goes through the
   full frame-blind discipline (the section-11 smoke then the section-10 panel) before any code,
   and the framing is posted for the gate's ruling. Not started until the reconciliation settles.

3. **Flagged sibling seam (a separate arc): the deep-`Body` wound path `body.rs:1108`** still
   authors a swing velocity, coupled to the body-to-Structure bridge. It belongs to the
   body-to-Structure unification arc, not to step 2.

## Flagged from the step-1b section-9 second pass (post-merge nits, for step-2 cleanup)

A second independent section-9 pass over the corrected, merged source returned nineteen clean-notes
and two NITS (both verified against source, neither a correctness defect, byte-neutrality confirmed).
They are recorded here to clean up when step 2 next touches these files, rather than expanded into
this doc-only bridge:

1. `crates/sim/tests/world_build.rs` `fully_grown_race` addresses composition with
   `composition_param(comp - 5)`. `comp` is a `usize`, so `comp - 5` underflows (a debug panic) if a
   program ever carries fewer than five composition axes. Unreachable today (this fixture uses
   `dev_default`, which declares exactly five), but the robust form is the leading index
   `composition_param(0..2)`, mirroring how the physiology axes lead the block. The `comp - 5` form
   is attributable to the step-1b diff.
2. Two pre-existing doc comments (`crates/sim/src/morphogen.rs` near the module head and near the
   `hash_into` note) assert a grown structure "folds into `state_hash`". The runner folds no
   `walker.structure`, and `Structure::hash_into` has no run-path caller (a unit test only), so the
   comments contradict the actual runner. Pre-existing (the step-1b diff neither introduced nor
   fixed them); correct them when morphogen.rs is next touched.

The accurate byte-neutrality rationale of record (the roadmap now states it this way): the four pins
hold not because a grown structure is unhashed on the pinned path, but because no run_world scenario
installs a morphogen program, so `grow()` never runs on the pinned path, and a grown body's effects
reach `state_hash` only through the dynamic state it drives (position, reserves), never a structure
fold.

## Discipline this branch holds to

Per-slice frame-blind before emergence-critical code; the section-9 five-lens plus adversarial
verify before every push; byte-neutral-or-stated-and-sequenced against the four run_world pins
(default `4bbf6b59`, full `1db633b3`, discovery `c9d5cc17`, viability `ad69f2bf`); never fabricate
a value (surface it reserved-with-basis); derive over author; admit the alien as data; and prove
it before trusting it, most of all when the conclusion is Agent A's own.
