# Owner decisions log

Decisions the owner needs to make, accumulated during autonomous work and surfaced at the end (owner
directive 2026-07-08: derive value gates and move on with dev values; leave decisions for the end, do not
stop mid-work to ask). Each entry states the decision, what I did in the interim (derived or dev-set), and
the basis, so the owner can confirm or override in one pass. This is not a blocker: work continues past every
entry using a derived or dev-set value.

## Open

0. **The `--scenario full` biosphere-balance collapse (a PRE-EXISTING issue, diagnosed, not the edibility
   grounding).** With the edibility grounding, the default, discovery, and viability scenarios all THRIVE
   (they were all going extinct before); only `--scenario full` still collapses, and it collapsed identically
   before the grounding. DIAGNOSIS (controlled A/B): the culprit is the producer food-OVERRIDE
   (`EnvironFields::set_producer`, biosphere-into-run). Where a biosphere plant stands, its biomass sets the
   food capacity, and the extract-deplete cycle draws the soil down in proportion to that capacity; the higher
   producer capacity draws the soil faster than weathering plus decomposition replenish it, so the soil (and
   the climate food the founders forage) depletes and the world crashes (disabling `set_producer` alone lets
   full thrive: pop 33 -> 42 -> 45 -> 48). This is a multi-value nutrient-cycle balance (`draw_fraction`,
   `weathering_rate`, `biomass_per_stock`, `pop_capacity`, the decomposition inputs), not a single derivable
   gate, so it is surfaced for a dedicated biosphere-balance pass rather than dev-hacked here. Candidate
   fixes to weigh: bound the producer draw to the soil's actual inflow (weathering + decomposition), or make
   the producer biomass a genuine standing stock the plant regrows rather than a fixed capacity the extract
   over-draws.

1. **R-UNITS-PIN: the reserve's absolute joule scale** (the `MetabolicAnchors` energy-density-to-joule
   anchor). Dev-set INTERIM value: `LocomotionParams::food_energy_density = 3000` (the forage reconciliation,
   calibrated so the default/discovery/viability worlds thrive). The geophage direct-fill needs no separate
   scale. Owner sets the canonical anchor. TWO HONEST LIMITS the end-of-arc audit confirmed, both surfaced not
   hidden: (a) the value was tuned by watching an AGGREGATE outcome (the population trend at seed 0x5EED,
   0xBEEF, 0xF00D) that is downstream of many OTHER simultaneously-dev-set reserved values, so "the world
   thrives" is a dev-harness calibration that a viable world EXISTS at this point, NOT a validated proof the
   physical model's absolute scale is correct; the owner's calibration replaces it against a real target.
   (b) `food_energy_density` is a SINGLE GLOBAL scale applied to EVERY backing class uniformly (energy, water,
   a mineral, a mana axis all reconciled by the same 3000), a functional simplification: the correct form is
   PER-CLASS content (each food's own per-supply content on each class), which lands NATURALLY when T3 is
   wired (the standing food carries the producer's own composition, so the plant's own `bio.energy_density`
   supersedes the global scale per cell). Until then the global scale is the interim, alien-imperfect (a
   mana-fed world's mana food is scaled by the energy reconciliation); the mechanism keys on the class as
   data, only the reconciliation magnitude is shared.

5. **The per-class physiological REQUIREMENT datum is no longer read on the physical intake path** (the audit
   flagged this). The old satisfaction intake read `laws::satisfaction(supply, assim, requirement)`, using a
   being's per-class per-tick REQUIREMENT to shape the fill. The physical intake fills toward the reserve's
   ROOM (capacity minus amount) instead, so the requirement datum is not gated on the physical path (it is
   still read on the no-physiology fallback path). This is a deliberate model change (a being eats until
   sated, room-bounded, rather than to a per-tick requirement curve), not a silent bug, but the owner should
   confirm the requirement datum's role is subsumed by the reserve capacity or restore it as an intake gate
   if a distinct per-tick need is wanted.

6. **The viability calibration has NO scenario-level CI protection yet** (the audit's confirmation-bias
   finding). No `#[test]` asserts a population survives across generations, so the "world thrives" proof is a
   manual run of the (non-canonical) run_world example, and the known `--scenario full` collapse is unflagged
   by any red test. INTERIM: a unit-level regression guard now ties `food_energy_density` to a survivable
   intake regime (a foraging being's per-tick gain stays a meaningful fraction of its reserve), so a scale
   regression fails CI; a full scenario-level survival test (a foraging cohort holds a population over N
   generations, and `full` marked as a known-collapse) is the follow-on the biosphere-balance pass should add.

2. **The T2/T3/T5 axis-conversion sign-off + Part 62 consolidation** (the chemistry arc, PR #105 merged).
   The mechanism is built and byte-neutral; the design-doc consolidation (a Part 62 record, the
   Decided-and-reserved blockquote, the bibliography, the audit Section 1/2/3 and counts) is the owner's
   resolution step (= R-SOURCE-VECTOR / R-BIO-REGISTRY).

3. **Cluster-I merge checkpoints.** Each Cluster-I arc branch (edibility grounding, Arc 5, 6, 7) is built
   autonomously and pushed as a PR; the owner runs final sims and merges. The chemistry arc (PR #105) was
   merged on the owner's standing authorization; later arcs are queued as PRs for owner review unless the
   owner extends the merge authorization.

4. **The genuine physics-CONSTANT reserved values** surfaced by the grounding, none fabricated: the Kleiber
   coefficient `kleiber_a`, the trophic/assimilation efficiency `ingest_efficiency`, the rock-weathering and
   per-substance decomposition kinetics. These are legitimate physics-floor authored inputs (Principle 9);
   dev-set until the owner calibrates. Not fudges.

7. **Arc 5 T4 residue (byte-changing, flagged not built).** `derive_region` still pads the region ENV vector
   to a fixed four slots with a moisture-DUPLICATE soil-fertility axis (the terrain has three real generated
   axes; the fourth is a moisture copy standing in for a real soil Stock that has not landed). Unifying the
   niche env-axis count with the tile-axis registry and grounding the derived soil axis (a real soil field)
   would drop that duplicate, which re-pins the biosphere, so it is deferred. The floor now carries
   `fluid.moisture_content` with `range_hi = 0.5` (a physics-floor authored bound, Hillel saturation basis);
   the owner may refine it.

8. **Arc 6 grown-body reserved values + the selection follow-on.** `GeneratorParams.ploidy = 2` (the
   sexual-diploid fixture; DATA, so a haploid/clonal alien is a world choice) and `morphogen_gauss` = the
   stamped `SumOfUniforms { k: 12 }` identity (the unset sentinel `k = 0` PANICS on draw, a trap avoided).
   Both reserved-with-basis; the owner sets the world's canonical ploidy and gauss stamp. HONEST LIMIT for the
   owner's awareness: epoch selection applies one uniform per-species fitness scalar across every locus, so a
   grown body's SHAPE is selected only as a side effect of regional niche fit, never because the grown
   Structure's own capability or viability was read. A fitness term reading the Structure (so morphology is
   selected on its own merits) is a natural next research item, surfaced not silently carried.

9. **Arc 7 (creatures-have-simpler-minds) is scoped, not built.** The first slice (spawn biosphere creatures
   as living `Walker`s reusing the founder machinery, byte-neutral behind a new flag) is fully planned in
   `docs/working/ARC7_CREATURE_MINDS_PLAN.md`. It touches the runner lifecycle and mints new StableIds (the
   two highest-risk areas: a creature id must be PROVABLY disjoint from founder ids, and a mind-less creature
   needs a `reconcile_lifecycle` guard), so it is surfaced as a focused dedicated slice rather than
   tail-of-marathon work. KEY finding for the owner: flee/chase CANNOT emerge from the first slice, because
   the evolved controller perceives only its own reserves and the matter field, NOT other beings; a
   being-perception percept (added to the shared `ControllerLayout`, which re-pins every walker) is the
   prerequisite deferred slice, and it must stay a general percept + evolved controller + selection so the
   predator-avoidance EMERGES rather than being an authored rule.
