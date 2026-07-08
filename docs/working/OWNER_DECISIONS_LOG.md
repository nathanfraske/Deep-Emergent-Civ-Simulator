# Owner decisions log

Decisions the owner needs to make, accumulated during autonomous work and surfaced at the end (owner
directive 2026-07-08: derive value gates and move on with dev values; leave decisions for the end, do not
stop mid-work to ask). Each entry states the decision, what I did in the interim (derived or dev-set), and
the basis, so the owner can confirm or override in one pass. This is not a blocker: work continues past every
entry using a derived or dev-set value.

## Open

0. **The `--scenario full` collapse: RE-DIAGNOSED 2026-07-08 (my earlier soil-draw diagnosis was WRONG,
   corrected here).** With the edibility grounding, default/discovery/viability all THRIVE; only `--scenario
   full` collapses. EARLIER (incorrect) claim: the producer food-override drives the extract cycle to
   over-draw the soil. FALSIFIED by controlled A/B: zeroing `draw_fraction` (no soil/water draw at all) still
   collapses full identically, so the extract DRAW is NOT the cause. THE REAL CAUSE (instrumented via
   `take_obs_deaths`): the full/viability founders are a GRAZER + OILSEED HYBRID (viability_homeostatic =
   `dev_grazer` energy(0)/water(1)/temperature reserves PLUS the oilseed seed reserves), and they die of
   THIRST (death axis 1 = WATER), not starvation. `set_producer` writes each real plant's biomass as the
   ENERGY food only at PRODUCER cells, which makes those cells an energy ATTRACTOR: the founders' forage taxis
   pulls them to congregate on producer cells, and where those cells are dry they die of thirst. This is why
   `MAX(producer, climate)` and bumping the producer biomass did NOT help (the cells stay an energy attractor)
   but disabling `set_producer` DID (uniform energy, the founders spread out and reach water). So it is a
   SPATIAL food-versus-water foraging coupling, tangled with the confused hybrid founder food setup, NOT a
   metabolism-rate or soil-depletion issue. IMPLICATION for the owner's "not just authored oilseed eaters"
   question: the clean fix is not a rate tweak, it is to RATIONALIZE the full-scenario founder food (forage
   the real biosphere producers cleanly like the DEFAULT grazer world already does, retiring the oilseed
   hybrid) and ensure the real producer food does not create a dry-cell thirst trap (a spatial energy/water
   balance). This IS the food-web integration (`docs/working/FOODWEB_INTEGRATION_PLAN.md` slices D + I), now
   understood to be the actual `--scenario full` fix, not a separate biosphere-balance rate pass.

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

9. **Arc 7 (creatures-have-simpler-minds) first slice BUILT (`45b269e`), creature-SURVIVAL is the follow-on.**
   Behind `--creatures` (requires `--scenario full`), 131 biosphere consumers spawn as living `Walker`-agents
   riding the founder embodiment loop, byte-neutral off (full unchanged 1c7cf2f2), worker-invariant. The
   runner-lifecycle trap is fixed (a creature is retired when IT dies, a founder when its MIND dies) and the
   creature id namespace is provably disjoint (asserted). HONEST LIMIT the owner should know: the creatures
   spawn with FULL reserves but DIE within the first tick, because the metabolic Kleiber drain exceeds their
   reserve at the small biosphere body scale (body_mass ~0.06 to 0.9) in the oilseed-based dev food world.
   This is the SAME metabolism-calibration class as R-UNITS-PIN and the item-0 `--scenario full` collapse, and
   it is the crux of the owner's "not just authored oilseed eaters" question: making creatures (and the whole
   real biosphere) survive is a metabolism/food BALANCE, surfaced for a dedicated pass, being scoped in
   `docs/working/FOODWEB_INTEGRATION_PLAN.md`. Two further deferred Arc-7 slices stay: a being-perception
   percept (so flee/chase can EMERGE, the controller perceives no other beings today, a re-pinning
   `ControllerLayout` change) and creature reproduction/selection (so good foragers are selected). All must
   stay a general percept + evolved controller + selection so predator-avoidance EMERGES, never authored.
