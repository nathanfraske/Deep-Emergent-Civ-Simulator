# Affordance composition substrate: the blind framing report (Agent B, sibling piece off Arc 3)

This is the doc-only opener for the composition-substrate framing, the deepest of the four seams flagged by the `AffordancePerceptKind` hardening (PR #110), assigned to Agent B by the gate. It also serves as Agent B's own gating channel (self-subscribed), the bridge that survives the #110 merge. No code is built here; the gate rules on the framing before any code.

## The piece as scoped

The `AffordancePerceptKind` hardening tracked a composition substrate as the deepest seam: a data-expressible affordance-derivation form so a new affordance formula is DATA rather than a new Rust kernel. It subsumes the two deferred deeper builds (Tier B, the parametrized single-axis reduction with a perceiver-keyed reference; Tier C, barring or relocating the composite pierce kernel). The gate directed framing it blind, doc-only, before any code.

## What was framed, and how

The framing was tested blind before any proposal: the section-11 input-bias smoke test (fail-closed, strongest model at maximum reasoning) on the panel's own construction, then the section-10 blind framing panel across isolated diverse panelists.

The smoke test BLOCKED the first construction: it caught that all my questions probed the combinator layer, where I already suspected weakness, and none probed the readable-leaf/floor-axis coverage, where the real ceiling lives, and that two falsifiers were missing (the alien the tree cannot express, and whether the substrate lets more emerge or only improves ergonomics). The construction was corrected (the readable-axis-coverage fact folded in, two falsifier questions added, the leading analogies stripped) and the smoke test then returned CLEAR. Only then did the panel run.

The panel returned five verdicts (six ran; one panelist was lost to a model-side safeguard flag, leaving five isolated voices across three agent types and two models, a strong diverse set). The raw statement put to them: an affordance-percept derivation is a DATA-defined expression tree over floor axes, its nodes a fixed closed combinator set, its references read from the perceiver's own data, emitting a neutral magnitude with valence receiver-side, so the derivation SET is world-declarable data (P11) and, the claim to test, admits the alien.

## The panel result: unanimous reframe, and a template-case violation caught

All five panelists returned `reframe-needed` or `significant-flaw-fixable`, converging on the same core: the framing conflates a P11 ergonomics win with a P8 emergence win, and "admits the alien" is oversold. Each finding is verified against source (Prime Directive 1).

**1. Two distinct ceilings, and the substrate touches only one.** Ceiling A is which fold combines the axes (the closed combinator set); moving that from bespoke-Rust-per-affordance to data-selected is a real P11 win. Ceiling B is which axes exist to read at all (the readable-leaf set, whatever the floor models); that ceiling is untouched. Alien-admission is bounded by ceiling B, so the "admits the alien" claim is inherited from the floor, not delivered by the substrate: a being whose native affordance reads an axis the floor never modeled gets nothing from the tree and still forces the develop-the-floor path.

**2. The template-case violation (the sharpest finding).** A hand-declared MULTI-AXIS composite tree selects which floor axes co-determine one percept. That bundling, "these axes together constitute an affordance", is exactly the correlational feature the template case says must EMERGE from a learner over primitives, never be authored. A declared composite tree is the affordance-analogue of reading relatedness: it hands the evolved controller a pre-formed composite fact, and if no author declares the tree, that affordance can never be perceived, never enters the discovery loop, and cannot emerge. So the declarable-tree SET bounds the emergent affordance space exactly as the bespoke-kernel set did, one in Rust, one in data: the P8 posture is unchanged and only the P11 ergonomics improved. Verified against source: the current `Sharpness` kernel is precisely such a composite (the Pierce capability over tool geometry and hardness), the seam the hardening already flagged.

**3. The leaf-smuggling seam (fixable).** "References read from the perceiver's own data or derived from the floor" is asserted in prose but not enforced. A data tree can still smuggle a literal terminal (normalize an axis against a typed-in constant), which crosses the value-authoring line through data rather than through a kernel.

**Verified feasibility of the fix.** The reframe routes composition through the existing discovery loop, and that path is grounded: `discovery.rs:61-63` states "there is NO coded primitive-to-affordance pairing ... selection keeps the combinations that pay off, so a technique emerges as a learned belief path rather than a designer's recipe (Principle 8)." The codebase already composes actions from primitives times percepts under selection, so routing affordance composition through a learner over axis-primitives is consistent with the established P8 discipline, not a novel leap.

## The corrected framing (survives my own check against the principles)

Split the substrate along the authoring line:

**A. Single-axis transduction, authorable as data (the clean P9/P11 win, and Tier B).** An affordance-percept LEAF reads one floor axis (of the matter, a held object, or the perceiver's OWN body) and normalizes it against a reference read from the perceiver's own data or derived from the floor. This is a sensory-physics-floor input, a "sensor" or organ a world declares as data, an innate P9 disposition, and it closes the perceiver-independent-reference seam by keying the reference on the perceiving being's own body. The transform kernel MATH stays fixed Rust; which axis a transduction reads, and against which own-data reference, is data.

**B. Multi-axis composition MUST EMERGE (the actual P8 fix, and Tier C).** Which single-axis primitives bundle into a composite affordance must ARISE from a learner over the primitives and the fixed combinator set, scored by selection through the discovery and controller loop, so a composite affordance (edibility, graspability, piercing-salience) is a DESCRIPTION of what emerged, never an authored tree. An author-declared tree is at most a seed or a diagnostic label, never the ceiling on what can be perceived.

**C. The value-line as a hard schema constraint, not prose.** Every leaf terminal is a floor-axis read, an own-data read, or a cited physics-floor law constant. No free numeric literals anywhere in a node (weights, exponents, saturation caps, the operand of a normalize). This is what enforces "derived from the floor" structurally.

**D. The honest bounds, stated plainly and tracked.** This substrate closes ceiling A (which fold combines EXISTING axes, the P11 win) and, through the emergent-composition path, the template-case seam for composites over existing axes. It does NOT touch ceiling B: alien-admission stays gated by floor-axis coverage, and a being whose native affordance reads an axis the floor never modeled still forces the develop-the-floor path. Track the combinator-set ceiling and the axis-set ceiling as first-class seams (R-XXX candidates), as the kind-enum ceiling was tracked.

## The decision this forces (surfaced, not assumed, Prime Directive 5)

The panel revises the piece from "a world declares affordance trees as data" (P11 only, and a template-case violation) to the split above. Per Prime Directive 5 I do not proceed on my original framing. The fork for the gate:

- **Fork (a): adopt the split.** Build the single-axis-transduction-as-data layer first (the clean P9/P11 slice that also closes the perceiver-independent-reference seam, Tier B), with the value-line hard schema; then the emergent-multi-axis-composition path (routing composition through the discovery loop under selection, which closes the template-case composite seam, Tier C, and subsumes barring the pierce composite). Frame each slice blind before code. State the ceiling-B bound plainly and track both ceilings.
- **Fork (b): scope down.** Build only the single-axis-transduction-as-data layer now (Tier B, the clean win), and open the emergent-multi-axis-composition path as its own gated design piece, since it touches the discovery and controller loop (a larger surface, and one that may intersect Agent A's later percept-wiring work).

My recommendation is fork (b) sequencing: the single-axis transduction is the clean, isolable, admit-the-alien-honest slice (a sensor keyed on the being's own data), and it directly closes a tracked seam; the emergent-composition path is the deeper P8 build and deserves its own framing and its own coordination against the discovery-loop surface. Either way, the "declare trees as data" overclaim is retired: the honest headline is a P11 win over existing axes plus an emergent-composition path for the P8 fix, with alien-admission gated by floor-axis coverage. The gate rules.
