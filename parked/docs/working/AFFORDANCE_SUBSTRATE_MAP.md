# Affordance/composition substrate map: the bedrock study (Agent B, PR #111)

This is the read-only bedrock study the gate commissioned under the owner's "loop everything to bedrock" directive: map the affordance/composition stack layer by layer until the layer that truly needs building, and name the real build target with file:line, or establish that an existing wired substrate already suffices. It is the affordance sibling of the perception-gaps study. No code is built here.

## How it ran

The construction was gated by the section-11 input-bias smoke test (fail-closed, Opus at maximum reasoning) BEFORE the mappers ran; it blocked twice and was corrected each time. The first block: the mappers only probed the affordance-percept and discovery layers, where the author already suspected a missing substrate, and never sent a mapper to the evolved controller's combining core, so the "the controller already composes channels under selection, nothing new needed" alternative had permission to be concluded but no evidence path to affirm it; and the six-way classification lacked a bucket for "an existing wired substrate that already suffices." Both were fixed (a dedicated controller-core mapper with the null as a first-class verdict, an "existing-wired-adequate" class, an expressiveness verify lens balancing the absence-proof lens) and the smoke test then cleared.

The cleared study ran as a workflow: four looping mappers (single-axis transduction, emergent composition, discovery/selection consumption, the controller combine-core), each adversarially verified against source (default-refuted, matched lenses), then a synthesis and a completeness critic. Every claim carries a file:line; the load-bearing anchors were re-verified from source, and the decisive correction below was verified by hand.

## The layer map

- **M1, single-axis transduction (Tier B).** The `SingleAxisTransduction` substrate (`affordance_percept.rs:451-561`) is the un-authored, perceiver-keyed primitive floor a composite would build over: one floor axis of the target in, normalized against the perceiver's own body axis or a cited floor constant, one neutral scalar out, valence-free, with a literal terminal made unrepresentable by the type. It is class-4 (an existing but unwired substrate): zero live callers, its resolvers supplied only by tests, the live wire a deferred `runner.rs` follow-on.
- **M2, emergent multi-axis composition.** No wired mechanism in `crates/sim` produces an emergent multi-axis composite affordance over transduction primitives.
- **M3, discovery/selection consumption.** The discovery loop's reward-belief selection (`discovery.rs` `candidate_bindings`, `sample_candidate`, the TD-lambda eligibility trace) is the correct and sufficient learner, and it composes primitive-times-single-channel under selection with no coded pairing. But the belief-subject key cannot conjoin channels: `SequenceStep` holds one primitive, one `target_bucket` channel, one `param_bucket` (`learn.rs:187-194`), verified by hand. The affordance percepts collapse through the disjunctive `appetitive_salience` `.any()` (`learn.rs:737-739`) into one per-primitive value before reaching the network.
- **M4, the controller combine-core.** The evolved controller's `activate` (`controller.rs:108-112`) is a hard-saturating linear unit that with a negative bias weight represents a bounded-AND, and its recurrent form represents arbitrary conjunctions, all from heritable founder-inert weights selection tunes. So behavioural conjunction over the channels the controller RECEIVES is already served (class-6). But the input layout has no per-affordance-axis block (`controller.rs:135-198`), so the controller conjoins appetitive/feature/conviction channels, never two affordance axes: the axes never reach it separately.

The sim-only synthesis therefore landed "mixed": single-axis selection and behavioural conjunction are served; an explicit emergent multi-axis COMPOSITE affordance (a conjunction like choppable = fracture-potential and sharpness sensed jointly on one target, reifiable as a shareable belief) is representable in no wired `crates/sim` mechanism.

## The decisive correction (the completeness critic, verified by hand)

The four mappers and the synthesis confined themselves to `crates/sim` and never read `crates/compose`, the crate named "the emergent technology-composition evaluator (design Part 41)" and the resolved research item R-DEEPTECH-COMPOSE. That crate already implements emergent-under-selection multi-axis composition, and I verified each load-bearing claim against source:

- It is the resolved composition substrate (`crates/compose/src/lib.rs:15`).
- A recursive `CompositionNode` DAG (`node.rs:50-91`): a leaf is geometric form primitives of one material with a join, a composite is child designs assembled with an assembly material and join.
- The data-defined combinator fold registry (`combinator.rs`, four physics-grounded kernels).
- The memoised bottom-up interval evaluator (`eval.rs`).
- A three-gate promotion into a per-culture, content-addressed, transmission-stabilized SHAREABLE library (`promote.rs:138-162`): viability at the collapse boundary, transmission-stability gated on the transmission substrate's own loss and drift rates, and reuse-compression.
- It is entirely unwired on the sim live path: `evaluate_node` / `promote` / `CompositionNode` have zero callers outside compose's own tests; only the capability LEAF read (`derive_capabilities` / `CapabilityKernel`) is live, at `affordance_percept.rs:198` (which is itself a wired multi-axis capability read the sim-only mappers over-generalized past).

So the synthesis's proposed "build a wholly new Tier-C composer" would duplicate a built, resolved substrate. That is the exact failure the completeness critic exists to catch, and the loop-to-bedrock method found the honest bottom is deeper than the enum.

## The layer-identity adjudication (a dedicated compose mapper, verified by hand)

A focused mapper read `crates/compose` end to end to adjudicate whether a composite AFFORDANCE is the same morphospace as an artifact `CompositionNode`. The verdict, with the crux verified by hand:

A `CompositionNode`'s operands are a designed object's parts: form primitives, a material substance, a join, and child designs referenced by content id (`node.rs:50-91`, verified). There is NO perceiver operand anywhere in the node, the port vector, or the eval path. A composite affordance is a perceiver-and-target relation whose operands are the target's floor axes PLUS the perceiver's own body axes, exactly the shape Tier-B `SingleAxisTransduction` already encodes (`affordance_percept.rs:422-460`). The two are distinct morphospaces: the artifact DAG has no slot for a perceiver body axis, so it cannot represent "choppable sensed jointly by this body." Their only overlap is the capability leaf both already call.

On the alien: compose's interface substrate is an open data registry (`interface.rs:76-93`), so a fold over axes the floor already models is a data row (the combinator-set ceiling is closable as data). But each interface axis must bind to a physics axis the floor carries (`interface.rs:61-63`) and the leaf physics dispatch reads a fixed structural axis set (`eval.rs:320-328`), so the readable-axis ceiling stays: a redox or photosynthetic axis the floor does not model is still floor-development. Wiring compose closes the "which fold" ceiling, not the alien-axis one.

## The named bedrock target

Neither of the study's first two framings is right. The honest target is: **build the Tier-C affordance composer by REUSING compose's resolved primitives, adapted to the perceiver-and-target operand**, rather than a from-scratch build (which would duplicate compose) or a drop-in wire (which the operand mismatch forbids):

- The LEAF is the built Tier-B `SingleAxisTransduction` (`affordance_percept.rs:451`), keyed on the target axes and the perceiver's own body.
- The FOLD is compose's `CombinatorKernel::fold` (`combinator.rs:90`), a data-defined typed-combinator registry (no new fold math).
- The EMERGENCE-AND-REIFICATION gate is compose's `promote` / `promoted_library` (`promote.rs:138,153`), adapted: gates 1 and 2 (viability, transmission-stability) transfer; gate 3 (reuse-compression) presumes "referenced as a component by other designs" and has no affordance analogue as written, so it needs an affordance-side reuse signal or a different third gate.
- The EMERGENCE SURFACE is the discovery loop (`discovery.rs` `candidate_bindings`, the "no coded primitive-to-affordance pairing" learner): the composite must EMERGE under the existing reward-belief selection, never be an authored primitive-to-affordance table. This requires the belief-subject key to represent a conjunction (`SequenceStep`, `learn.rs:187-194`) or the latent multi-step subject to be wired, and the Tier-B transduction primitives fed into the percept channel in place of the closed authored percepts (the deferred `runner.rs` wire, on Agent A's surface).

This is not a new learner: the reward-belief selection loop is the correct and sufficient learner. It is a composer that reuses compose's fold and promotion kernels over the affordance operand, emerging through the discovery loop under selection.

## The intersection with Agent A

The target intersects Agent A's being-percept valence learner at the shared bedrock, and the intersection is exact: both are one machine, correlate a primitive or percept with a felt outcome under selection. Agent A's valence learner IS the discovery/reward-belief loop this study localizes (`candidate_bindings` proposes the candidate, `candidate_weight` reads the being's own felt-reward belief, the eligibility trace assigns delayed felt-outcome credit). The composite build does not add a second learner; it extends the same loop's subject key from a single-channel atom to a conjunction over un-authored transduction primitives. Agent A supplies the valence signal that credits a belief; this target supplies the richer percept-side subject the same learner correlates against. So the discovery/controller loop is the convergence point, and this build must be sequenced with Agent A's percept and learner work on that surface.

## The owner-blocker (reserved, surfaced with its basis)

A genuine design-intent fork, the owner's call, not the agent's: **is a composite affordance the SAME morphospace as an artifact `CompositionNode`, or a DISTINCT sibling substrate?**

- UNIFY: extend compose's node leaf to carry a perceiver operand and a perceiver-keyed reference, so one substrate serves both designed objects and sensed affordances. One library, one promotion path, at the cost of widening the artifact node with a perceiver slot it does not otherwise need.
- DISTINCT SIBLING: an affordance composer that reuses compose's fold, promote, and open-registry kernels but keeps its own perceiver-and-target node shape. Two node shapes, shared kernels, cleaner separation of the artifact and affordance morphospaces.

The two seams that decide it: promote gate 3 (reuse-compression presumes "referenced as a component by other designs," which an affordance-percept has no analogue of) and the missing perceiver operand in the artifact node. The code supports either. Held for the owner in the register.

## Honest bounds carried forward

- The READABLE-AXIS ceiling stays (an alien affordance needing an axis the floor does not model is floor-development, `affordance_percept.rs:411-413`); wiring compose does not close it.
- The combinator-set ceiling is closable as data via compose's open interface registry, so it need not stay a closed one-kernel set.
- The `SEQ_FIELD_BITS` packing ceiling (`learn.rs:162-174`, four bits per field, 16 values) is a not-byte-neutral refinement the composite build may force once the primitive alphabet crosses 16, and it changes every existing belief subject, so it is an owner-call before the composer is wired.
