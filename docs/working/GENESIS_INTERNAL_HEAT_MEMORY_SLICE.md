# Genesis-forward speedup slice: the internal-heat axis and the memory primitives

This is the design opener for the gate-carved speedup slice of Agent A's genesis-forward arc (the emergent-geology packet, `docs/working/GEOLOGY_ARC_PACKET.md`). Agent B builds two disjoint-file foundation pieces off `origin/main`: the internal-heat-production floor axis with its radiogenic decay, and the four memory primitives the packet's bias-control audit named as the substrate's biggest missing dimension. Both are Layer-0/Layer-1 floor foundations that later phases (the interior engine, the tectonic regime, the surface-process record) read, so they are on the critical path and unblock rather than wait.

## Piece 1: the internal-heat-production axis and radiogenic decay

The packet (section 3, Phase 1) calls for an internal-heat-production axis in watts per kilogram, a heat-per-mass source the mantle convection and the tectonic regime read, plus the radiogenic decay of the heat-producing isotopes. Two authoring categories, kept distinct per the locked three-way test:

The internal-heat-production axis (W/kg) is a NEW FLOOR AXIS whose per-world VALUE is per-world data. The axis is the authoring place (a heat-per-mass source term); the value a world carries is its isotope abundance, contingent per-world data (Mirror is bulk silicate Earth). A tidal or accretional source feeds the SAME axis as heat-per-mass regardless of origin, so the axis is general (admit-the-alien): a tidally heated moon and a radiogenic planet differ only in the value, not the mechanism.

The radiogenic decay constants (U-238, U-235, Th-232, K-40) are UNIVERSAL REFERENCE DATA: the measured half-lives and specific heat-production rates of the heat-producing isotopes, the same in every world, cited once (the same reference-substance tier that carries water, iron, and granite, and that just absorbed the seventh CODATA fundamental G in #161). The decay-rate law that reads them (the internal heat production falling as the isotope reservoir decays over geological time) is DERIVABLE Rust, a pure function of the reservoir, the decay constants, and the elapsed age. So the decay law is exactly a first consumer of the world clock and the age stamp from Piece 2.

## Piece 2: the memory primitives

The packet's exotic-and-thaumic gap analysis (Part D) found the biggest single catch: the substrate is MEMORYLESS. Every floor law kernel is a pure present-to-present function; the only temporal capability is the one-step `temporal = prior` plus `dt` port in the induction laws. So the substrate cannot express any state that RECORDS a past regime differing from the present, yet a large class of geology IS exactly that relic record (extinct-dynamo remanent magnetization, tidal-budget history, metamorphic pressure-temperature-time paths, inherited radiometric age, the one-way surface-redox transition). The packet's through-line: the substrate needs a temporal and memory dimension as much as it needs the spatial generality. This slice builds the four primitives, each a determinism-clean floor law-form or data-model addition, integer and fixed-point only (Principle 3, no wall-clock, no float convergence):

The world CLOCK: a single monotonic tick counter, the absolute time base the age stamp and the decay law read. It is the one authoritative time source, pinned so replay reproduces it exactly.

The per-parcel AGE STAMP: a formation-time stamp per material parcel (the absolute clock value at which a parcel formed or last re-equilibrated), so a parcel's elapsed age is the clock minus its stamp, the input the radiogenic decay and any age-recorded relic reads.

The ACCUMULATOR: a general integrate-over-time law-form, a resident quantity advanced each tick by a per-tick rate (the strain that builds toward a yield threshold, the dose that builds toward a transition, the tidal budget that spends down). A pure deterministic fold over the clock, distinct from a memoryless kernel because it carries state between ticks.

The one-time irreversible-threshold LATCH: a primitive that fires ONCE when an accumulated quantity crosses a declared threshold and then stays latched (inner-core nucleation, a redox transition, a phase latch), the one-way transition the memoryless present-to-present kernels cannot express. Deterministic and monotone: it never un-fires, so the recorded past is stable.

## Discipline and scope

No fabricated value: the isotope abundances are per-world data surfaced with basis; the decay constants and specific heat-production rates are measured reference data, cited (a lab can refute them without this simulator); the axis and the four primitives are fixed-Rust mechanisms whose membership and per-world values are data. Determinism holds by construction (fixed integer clock, fixed-point accumulators, a monotone latch, no wall-clock or float-convergence gate; Principle 3). Byte-neutral-or-stated: the axis and primitives are opt-in floor additions that no current scenario arms, so the canonical pins hold until a genesis pass switches them on; any pin move is stated with its reason. The files are B's own (the floor axis and the laws.rs law-forms, B first in the laws.rs queue), disjoint from A's Layer-0 fundamentals and C's units domain. Each sub-piece is grounded at source, built byte-neutral-or-stated, and posted; the gate runs the audit lenses before merge.
