# The Primitive Physics Substrate: Research and Population Guide

**Working vehicle for R-DEEPTECH-PHYSICS, and the shared upstream coordinator for the physics-grounded cluster (R-FLUID, R-WEATHER, R-BUILD-PHYS, R-CATASTROPHE, R-WOUND).**

Status: a guide, not a settled specification. It records the decisions the existing architecture already forces, the open questions that the fan-out research and the owner's reserved decisions must answer, and the references that govern both. Every locked decision here is held only because it follows from a principle already settled in the design document, and any of it yields to evidence: a flaw found in research overrides it. Nothing in the open part is guessed here; the catalogue is populated by research, by datasheets, and by the owner, not by this document.

---

## 1. Authority and how to read this

This document carries two kinds of content, kept visibly separate. The first is the locked representation (Section 4): decisions that are derivable now because they follow from Principle 9, the completeness requirement, the determinism core, and the bounded-floor pattern the project already uses three times over. These are stable enough to build against, and they are overridable only if research shows the governing principle is itself wrong for physics. The second is the open research (Sections 5 and 6): the contents, the criteria, and the values, which the fan-out across disciplines and the owner's reserved decisions populate, and which this document deliberately does not fill in.

The resolved output of this work consolidates into the design document (the Part 41 physics substrate, Part 58, and the materials of Part 19) under the same prose customs and the same verification suite. The design document stays the source of truth; this guide is the working space. When a locked decision is overridden by evidence, the override is marked and dated here and propagated to the design document.

## 2. What already governs this

The research is not starting from nothing. The following are settled and constrain every answer.

- **Principle 9 (Part 0): physics is the one authored layer**, the single bounded place the designer's hand may enter, subject to the Steering Audit. Everything cultural emerges from it. This is the root reason the physics set is the highest-leverage authored artifact in the project.
- **The completeness requirement (Part 41)** states the stakes in the design's own words: the physical model must be "as complete as it can be made", it is "the single most important and most demanding piece of content in the project", "the reach of the whole emergent-technology system is exactly the reach of its physics", and it "must therefore be assembled deliberately and, by Principle 9, audited for hidden outcome bias, since a physical law that quietly encodes the designer's preferred answer is a steering leak wearing the costume of physics". Functional outcomes (chopping, penetration, reach) are measured consequences of physics, never stored fields.
- **The descriptor-space caveat (Part 41 honest limits):** the choice of descriptor space is a subtle authoring decision, since the search illuminates only the dimensions it is given, so descriptors are kept to physical quantities rather than designer aesthetics.
- **Materials are data (Part 19),** which this guide sharpens into materials-as-points-over-axes (Section 4).
- **Magic is a grounded system (Part 34), with its laws defined as data under Part 40, both under Principle 9,** which places the magic axes inside the same extensible floor as the rest of the physics (Section 4). Part 34, "Magic as a Grounded System", is itself the precedent: magic that reduces to grounded laws and axes.
- **The foundational substrates (Part 58)** are where the resolved physics structures live, alongside the conserved-projection registry.
- **The semantic substrate (Part 33) and the value substrate (Part 21)** both project their world-axes from physics and world state, so the physics axes are upstream of meaning and value. The semantic substrate's bounded primitive floor (the NSM primes) is the precedent for the bounded-but-extensible floor used here.
- **The composition layer (Part 41, R-DEEPTECH-COMPOSE)** reads physics through its interface-axis substrate and emergent-proxy set; those are the technology-facing projection of this layer, and the composition leaf floor is a second precedent for the bounded floor.
- **The determinism core:** fixed-point Q32.32, counter-based RNG, content-addressing with stable ids, and the interval-bounded integer evaluation established in the composition work. Physical quantities and laws live inside these constraints; float stays quarantined to the view.
- **The reserved-value process (runbook):** every constant is surfaced with its basis, never fabricated, and split between real-with-source and fantasy-reserved-with-basis.

## 3. Stakes and the two horizons

Three properties make this categorically unlike the other open items. It is the only authored layer, so its correctness is asymmetric: a flaw cannot be selected against by emergence, it becomes an authored bias in every outcome. It is reach-bounding, so populating it decides the expressiveness ceiling of the whole simulation. And it is the shared upstream dependency for materials, fluids, weather, build-physics, catastrophes, wounds, and the projected world-axes of value and meaning, so doing it as a coordinating document keeps that cluster consistent and amortizes the work.

It also spans two horizons, and the document holds both. The minimal grounded floor is needed early, because the first build proof (convergence without a target) cannot run without a small physics to select against. The tiered, deepening, discoverable substrate is the far-horizon climb. The floor is populated and proven first; the deepening is staged.

## 4. The locked representation (decided now, overridable on evidence)

These decisions follow from Section 2 and are stable enough to build against.

**Primitives are axes and laws; substances are points over them.** A physics primitive is either a quantity axis (a named, unit-bearing, range-bounded, fixed-point scalar dimension, carrying a tier) or an interaction law (a closed-form integer kernel that takes the quantity vectors of the participating entities and reports a measured consequence as an interval-bounded fixed-point value, also tier-tagged). The functional outcomes a player would name are outputs of laws, never stored categories, exactly as the completeness requirement demands.

**A material is a data point, not a primitive.** A substance is a vector of values over the quantity axes, plus the set of laws it participates in, plus a provenance tag. The engine authors the axes (real physics) and the laws; materials are points over them. This is what leaves room for materials with no basis in our reality: you never author materials into the primitive set at all. Iron draws its vector from a datasheet; an invented alloy draws its vector from the owner's reserved decisions, over the same axes. The engine treats them identically; the only difference is the provenance tag and the source of the numbers.

```
QuantityAxis   { id, unit, fixed_point_range, tier }
InteractionLaw { id, inputs: [AxisId], kernel (closed-form integer),
                 output_measure, interval_bound, tier }
Substance      { id, vector: [Fixed over AxisId], participates_in: [LawId],
                 provenance: RealWithSource(citation) | FantasyReserved(basis) }
```

Everything is content-addressed with stable ids for determinism and memoisation, the same discipline as the composition node.

**The floor is bounded but extensible.** The axis set and the law set are a bounded floor, the structural analogue of the NSM semantic primes and the composition leaf primitives. Everything above the floor is points and compositions over it. A fantasy material is representable if and only if its behaviour decomposes onto the axes the substrate has: a light, strong material is a vector (low density, high toughness); a material that glows when struck participates in an emission-on-impact law the substrate carries; a material that channels magic is nonzero on a magic-coupling axis. Inventing a material whose behaviour needs an interaction the substrate has no axis for requires extending the floor with a new axis, and that extension is a deliberate Principle-9 affordance that is audited for steering. Extending the floor is a designed act, never a silent one.

**Magic lives on the same floor, as extension axes.** Because magic is already a grounded system (Part 34) with its laws defined as data under Part 40, both under Principle 9, the magic and exotic-physics axes are part of this extensible floor. The reach of the magic system is bounded by the magic axes the substrate carries, exactly as the reach of technology is bounded by the physical axes. Magic that reduces to axes and laws is representable, deterministic, and able to ground emergent outcomes; magic that cannot be reduced to axes and laws would be an authored outcome, a steering leak, and would break both the determinism and the emergence premise. This is the boundary that lets a simulated-reality physics exist without abandoning the principles.

**Provenance is split and explicit.** Every axis value and law constant is tagged either real-with-source (a citation or datasheet) or fantasy-reserved-with-basis (the owner's decision and the ground for it). This is where the real-data discipline gets its largest workout, and it feeds the calibration manifest directly.

**Determinism constraints bind all of it.** Every quantity is fixed-point with a declared range; every law kernel is a closed-form integer function with an interval bound; every entity has a stable content-addressed id; no float enters the canonical state. A law that cannot be expressed in closed-form fixed-point with a bound is out of scope until it can be.

## 5. The open research (populated by fan-out, datasheets, and the owner, not here)

These are the open questions. This document states them and what grounds them; it does not answer them, because a fabricated answer here would be the steering leak the substrate exists to prevent.

- **The axis set:** which quantity axes exist, and at what tier each appears. Grounded in physics and engineering, decided through the fan-out, not guessed.
- **The laws:** the exact closed form of each interaction law, which are real physical and engineering models that must be chosen, converted to fixed-point, and validated against the phenomena they model.
- **The tiering model:** what "discoverable" and "deepening" mean mechanically, how a tier's physics is unlocked, and how a tier's primitives gate the technology and composition reach above them.
- **The completeness criterion:** the test that says the floor is complete-enough for a tier, and the Steering Audit that says its descriptors do not encode preferred outcomes. The design document flags this as hard; it is.
- **The magic and exotic axes:** the specific axes of the simulated reality's physics, and how they stay bounded and deterministic. This is mostly the owner's reserved fantasy-design plus the consistency work to keep it on the floor.
- **The primitive-versus-emergent boundary test:** the operational procedure for deciding, case by case, what is a primitive and what is a measured consequence. The principle is settled (measured consequence, never stored field); the decision procedure is open.

## 6. The catalogue, as a frame to populate

The catalogue is organized by domain and is not filled in here. Each entry, once researched, is an axis or a law with its tier and its provenance tag. The domains, with the kind of axis each holds shown only as illustration and not as the authored set:

- **Mechanics:** force, momentum, contact area, leverage, friction, and the like.
- **Materials and their properties:** hardness, toughness, fracture behaviour, density, and so on.
- **Energy and thermal:** thermal transfer, capacity, phase behaviour.
- **Fluids:** the axes and laws the fluid systems (R-FLUID) read.
- **Chemistry and reactions:** the composition and transformation laws.
- **Optics and signal:** emission, transmission, and the like.
- **Biology and anatomy substrate:** the physical axes the body, wound (R-WOUND), and fluid systems read.
- **Magic and exotic axes:** the simulated reality's own physics, owner-reserved.

The illustrative quantities above are textbook examples of the kind of axis a domain holds; the authoritative axis set, the law forms, the tiers, and every value are the research output, determined through the fan-out and the datasheets, not asserted here.

## 7. Interfaces to the dependent systems

This layer is upstream of much of the engine, and stating the contracts makes the reach-bounding explicit per consumer.

- **Composition (Part 41):** the interface-axis substrate and the emergent-proxy set are the technology-facing projection of these axes and laws; composition reads measured consequences from here, and its reach equals the reach of this substrate.
- **The value and semantic substrates (Parts 21, 33):** their projected world-axes are derived from these physics axes and from world state.
- **Materials (Part 19):** materials are the substance vectors of Section 4.
- **The physics-grounded cluster (R-FLUID, R-WEATHER, R-BUILD-PHYS, R-CATASTROPHE, R-WOUND):** each reads a subset of the axes and laws. This document is their shared upstream, so resolving it unblocks the cluster and keeps the subsets consistent.

## 8. The Steering Audit and the fan-out plan

The Steering Audit for physics is the test that the chosen primitives and descriptors do not author cultural outcomes. The red team's job is to find the physical law or descriptor choice that quietly encodes a preferred answer, the axis set that illuminates only the dimensions that produce the designer's expected result, and the proxy that smuggles in an outcome. This is the attack surface the adversarial pass exists for.

This is the single largest fan-out candidate in the project, for two reasons at once: the breadth (many physics and engineering disciplines, and every dependent consumer) and the failure mode (an authored bias in the one authored layer, an unbounded or nondeterministic substrate, or a reach ceiling discovered too late), which is the confident-but-wrong-with-subtle-consequences shape. The fan-out parallelizes across the catalogue domains and across the dependent consumers; the green team builds and defends a domain's axes, laws, tiers, and the completeness claim, and the red team runs the Steering Audit and the determinism and reach attacks against it.

## 9. Reserved values, override, and consolidation

Every constant is surfaced with its basis and split real-with-source or fantasy-reserved-with-basis, feeding the calibration manifest; no value is fabricated here or by the agent. Any locked decision in Section 4 yields to evidence: if research shows the governing principle is wrong for physics, the decision is overridden, and the override is marked and dated here and propagated to the design document. The resolved output consolidates into the design document (Part 41, Part 58, Part 19) under the prose customs and the verification suite, and the design document remains the source of truth.
