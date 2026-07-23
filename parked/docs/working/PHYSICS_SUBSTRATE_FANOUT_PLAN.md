# The Physics-and-Materials Substrate Fan-Out: Execution Target

**Working target for the full resolution of the physics-and-materials substrate (R-DEEPTECH-PHYSICS and the physics-grounded cluster), the reach-bounding authored layer of the project.** This document is the plan; the substrate guide (`PHYSICS_SUBSTRATE_GUIDE.md`) is the locked representation and the open-question frame it executes against. The biology-and-composition slice has already been run as wave 0 (see `BIOSPHERE_SUBSTRATE_SCOPED_PROPOSAL.md`); this records the larger effort it is the first wave of, so when we take it up next the structure is set rather than improvised.

## 1. Why this is its own effort

Three properties make the substrate categorically unlike the other open items, all three stated in the guide and confirmed by the biosphere trace. It is the one authored layer (Principle 9), so a flaw cannot be selected against by emergence: it becomes an authored bias in every downstream outcome. It is reach-bounding: the expressiveness ceiling of technology, value, meaning, and biology is exactly the reach of the axes and laws it carries. And it is the shared upstream of materials (Part 19), fluids, weather, build-physics, catastrophes, wounds (the physics-grounded cluster), the composition interface (Part 41), and the projected world-axes of value and meaning (Parts 21, 33). Doing it as one coordinated fan-out keeps that cluster consistent and amortizes the work, which is why it is planned as a whole even though it is executed in waves.

## 2. What is already locked (built against, not re-decided)

From the substrate guide, Section 4, overridable only on evidence. A physics primitive is either a quantity axis (`QuantityAxis { id, unit, fixed_point_range, tier }`, a named unit-bearing range-bounded fixed-point scalar at a tier) or an interaction law (`InteractionLaw { id, inputs: [AxisId], kernel (closed-form integer), output_measure, interval_bound, tier }`). A substance or organism is a vector of values over the axes plus the laws it participates in plus a provenance tag (`RealWithSource` or `FantasyReserved`). The floor is bounded but extensible: a new axis is a deliberate, audited Principle-9 act, never silent. Magic rides the same floor as extension axes. Determinism binds all of it: fixed-point Q32.32, closed-form integer kernels with interval bounds, content-addressed stable ids, no float in canon. The fan-out populates the membership; it does not reopen the representation.

## 3. The fan-out structure

The work parallelizes across two axes at once: the catalogue domains (breadth) and the dependent consumers (the reach contracts). Each domain runs a green build and a red Steering Audit; each consumer runs an interface check.

**Green domain teams (one per catalogue domain, Section 6 of the guide).** Each proposes, with cited bases, the domain's quantity axes (unit, fixed-point range, tier, provenance) and interaction laws (inputs, closed-form integer kernel, output measure, interval bound, tier), surfaced as reserved-with-basis, never set. The domains:

1. Mechanics: force, momentum, contact area, leverage, friction, and the laws over them (the cutting, fracture, and impact consequences technology reads).
2. Materials and their properties: hardness, toughness, fracture behaviour, density, elasticity, the bulk material axes Part 19 substances are points over.
3. Energy and thermal: thermal transfer, capacity, phase behaviour, combustion, the energy economy.
4. Fluids: the axes and laws the fluid systems (R-FLUID) and weather (R-WEATHER) read.
5. Chemistry and reactions: composition and transformation laws, the reaction substrate production and metallurgy stand on.
6. Optics and signal: emission, transmission, reflectance, the light field (Part 5) and the perception-gating laws read.
7. Biology and anatomy substrate: the physical axes the body, wound (R-WOUND), and the biosphere composition model read. (Wave 0, run; see the scoped proposal.)
8. Magic and exotic axes: the simulated reality's own physics, owner-reserved fantasy design kept on the same floor and laws.

**Consumer-interface agents (one per dependent system).** Each confirms its system reads exactly the axes and laws it needs and no fabricated field: composition (Part 41) and its interface-axis and emergent-proxy set; the value and semantic substrates (Parts 21, 33) and their projected world-axes; materials (Part 19) as substance vectors; and the cluster (R-FLUID, R-WEATHER, R-BUILD-PHYS, R-CATASTROPHE, R-WOUND), each reading a subset. The interface agents are how the reach-bounding is made explicit per consumer and how subset consistency is held.

**Red Steering-Audit teams (adversarial, per domain and across).** The red job is the one the authored layer exists to guard: find the law or descriptor choice that quietly encodes a preferred outcome, the axis set that illuminates only the dimensions producing the designer's expected result, the proxy that smuggles in an outcome, the axis whose granularity makes one race "naturally" superior. Plus the determinism attack (a kernel that is not closed-form fixed-point with a bound, an overflow surface, an order-dependent reduction) and the reach attack (a ceiling discovered too late). A domain's axes and laws are not accepted until the red team has run against them.

## 4. The waves (two horizons, the floor first)

The guide's two horizons set the sequence. Wave 0 is the minimal grounded floor a domain needs for the first proof to run, populated and proven first. Later waves are the deepening, discoverable tiers, staged.

- **Wave 0 (run): the biology-and-composition floor.** The nutrition, toxin, consumer-physiology, and edibility-law slice the living-world build needs, run as scoped facets, output in `BIOSPHERE_SUBSTRATE_SCOPED_PROPOSAL.md`, reserved-with-basis and awaiting the red pass and owner sign-off. It is wave 0 of the Biology domain and a template for the rest.
- **Wave 1: the mechanical-and-materials floor.** Mechanics, Materials, Energy/thermal at Tier 0, the floor the first technology convergence proof (Part 41, Stage 14) selects against. This is the next-largest unblock after biology.
- **Wave 2: the fluid, chemistry, and optics floors.** The cluster consumers (R-FLUID, R-WEATHER), reactions, and the light field.
- **Wave 3 and beyond: the deepening tiers.** The discoverable, tier-gated depth that bounds the upper technology climb, staged over the far horizon, each tier with its completeness criterion and Steering Audit.
- **The magic and exotic axes** run alongside whichever wave their consumer needs them, on the same floor, owner-reserved.

## 5. The per-facet output schema

Every green facet returns, for the owner to set, never fabricated: for each axis, `{ name, what it measures, unit, plausible fixed-point range with basis, tier, provenance (real-with-source plus citation, or fantasy-reserved-with-basis) }`; for each law, `{ inputs (axis ids), the closed-form integer kernel expressed over the Fixed ops that exist, output measure, interval bound, tier, scientific basis plus citation, every reserved constant with its basis }`; a determinism feasibility note per law; and the steering seams the facet flags rather than smooths. The red team returns confirmed steering or determinism or reach findings with the attack that produced each.

## 6. The gates (a domain is not done until)

A domain's output passes only when: every axis is unit-bearing, range-bounded, fixed-point, tier-tagged, and provenance-split; every law is closed-form integer with an interval bound and an order-independent reduction; the completeness criterion for the tier is stated and met (the test that the floor is complete-enough for that tier); the Steering Audit has run and its findings are folded; and the consumer-interface agents confirm their systems read what they need. Then it consolidates into the design document (Part 41, Part 58, Part 19) under the prose customs and the verification suite, with the design document remaining the source of truth and the reserved values feeding the calibration manifest.

## 7. Cost, sequencing, and the standing instruction

This is the single largest fan-out in the project, in breadth (many physics and engineering disciplines and every dependent consumer) and in stakes (an authored bias in the one authored layer). It is taken in waves so the floor is proven before the deepening, and so each wave unblocks a concrete consumer (wave 0 unblocks the living world, wave 1 the technology proof). The owner's standing instruction holds: nothing chosen is final, every axis range and law constant rides the fidelity-versus-compute scale and is retuned at scale, and a value set now is a hypothesis awaiting calibration. No value is fabricated here or by the agent; the fan-out surfaces, the owner sets.
