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
   if a distinct per-tick need is wanted. The edibility grounding makes intake and drain both physical in the reserve's units; the ABSOLUTE
   scale (how a floor `bio.energy_density` in kJ/g maps to the reserve's stored joules) is the one genuine
   units anchor. INTERIM: dev-set to the value that keeps the dev world viable, iterated against the
   survival proof. BASIS: the reserve stored energy is `capacity * body_mass_kg * bio.energy_density`; set so
   a being's daily intake at plausible forage density offsets its Kleiber drain. Owner sets the canonical
   value; the mechanism derives everything else.

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
