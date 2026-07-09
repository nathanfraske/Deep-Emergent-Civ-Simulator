# Owner decisions log

Decisions the owner needs to make, accumulated during autonomous work and surfaced at the end (owner
directive 2026-07-08: derive value gates and move on with dev values; leave decisions for the end, do not
stop mid-work to ask). Each entry states the decision, what I did in the interim (derived or dev-set), and
the basis, so the owner can confirm or override in one pass. This is not a blocker: work continues past every
entry using a derived or dev-set value.

## Open

R7. **The world is 3D (`Coord3`), not 2D; perception must key on it. OWNER DIRECTIVE + CORRECTION 2026-07-09.**
While framing the reach wire (Arc 3 slice 1), I stated the world was a 2D grid and proposed a 2D `1/r` falloff.
The owner corrected it: the world should be 2.5D minimum (space above) with subsurface (things in the ground).
Verified in source: the world coordinate is `Coord3 { x, y, z }` (`locomotion.rs:698`), z vertical, the material
field carrying subsurface strata at negative z (`material.rs`, hematite at z:-2); a being carries a `Coord3`.
The 2D I saw is only the perception/place projection: the perception path keys on an opaque flat `PlaceId`
(`world.rs:83`), and the environmental fields are a 2D surface grid. Consequence: `inverse_square_falloff`'s
`P/(4*pi*r^2)` is the correct 3D law over the 3D `Coord3` separation, so the reach wire keys distance on `Coord3`
directly and bypasses `PlaceId`. Owner ruling: build the perception-substrate framework out first with the reach
wire scoped on `Coord3` directly (the small change), and SCOPE the 3D perception-place lift (raise the whole
perception place model from `PlaceId` to `Coord3`) now and do it NEXT after the framework. Captured in
`PERCEPTION_SUBSTRATE_ARC_PLAN.md` (slice 1 and the sequence section).

RESOLVED slice-1 (reach wire) framing, 2026-07-09 (blind section-11 then section-10 panel, 6/6
significant-flaw-fixable, unanimous; gate accepted fork (a)). The reach wire is: for a signal on channel c
from a source `Coord3` to a perceiver `Coord3`, a received physical scalar computed as a general
dimensionality-parameterized geometric-spreading kernel (D derives from the traversed path/medium geometry,
3D bulk to `1/d^2`, a 2D surface to `1/d`, a duct to no spreading; D reserved fail-loud with its geometry
basis where it cannot yet derive, never fabricated) applied to the emitted power and the 3D separation, then
attenuated by the medium's own `opt.absorption_coefficient` sampled along the `Coord3` segment (so occlusion
emerges from the strata, no authored line-of-sight). Channel c is a data-registry row naming its kernel and
axis ids (dispatch by named id, never a code branch on channel identity). The received value is a pure
per-perceiver read (P10). Five build conditions from the gate: D derives or is reserved fail-loud; the general
kernel is byte-identical to `inverse_square_falloff` at D=3; absorption reads the medium axis, never a label;
the registry dispatches by id; the non-geometric propagation-law-as-data stays flagged as the deeper
substrate. The general geometric-spreading kernel at any integer dimensionality is legitimate floor authoring
(physics is an authored floor input, Principle 9), subsuming inverse-square. Section-11 caveat: an earlier
frequency clause over-committed (stated body-resonance as settled); the body-resonance-reserved ruling for the
acoustic frequency stands, judged alien-safe on the clean axis.

BUILT + AUDITED slice-1 (reach wire), 2026-07-09 (Agent A, PR #109 `claude/liveliness-arc`). The reach wire
is built in four byte-neutral off-path segments: the general `geometric_spread` kernel (`physics/laws.rs`,
byte-identical to `inverse_square_falloff` at D=3, proven by test), the channel reach registry
(`perception_reach.rs`, dispatch by the row's named kernel id, never channel identity), the reach read
(`received_reach`: 3D `Coord3` separation with the vertical z, structural dimensionality, medium-sampled
optical depth), and the run-path resolver (`resolve_reach`/`absorption_along`: reads the row, samples the
medium's own absorption along the `Coord3` line, dispatches by kernel id). All four run_world pins hold
bit-exact (default 2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82), corroborated by a caller
sweep (zero run-path callers). The mandatory section-9 five-lens audit ran (6 blind panelists across 3 types
and 3 models, adversarial per-finding verify): 13 findings, all verified against source and hardened. The one
MAJOR: `resolve_reach` silently ignored the row's `frequency_dependent` field, so the acoustic dev row would
resolve to a frequency-independent read of the OPTICAL absorption axis. Hardened to implement the framing's
already-approved "reserved fail-loud": `resolve_reach` now asserts a row is not `frequency_dependent` (the
emitter body-resonance frequency source and the acoustic-law application are a reserved follow-on, not wired
in slice 1), so a frequency-dependent row fails loudly rather than reading the medium axis as if it were
frequency-independent; no shipped row sets the flag; a `#[should_panic]` test proves the fail-loud. The nits
hardened: `MAX_REPRESENTABLE_SEP2` derived from the `i32` cast bound rather than a magic `1<<30` (and the
comment corrected: the guard is a representability clamp, and the D=3 kernel already overflows its own
denominator to zero far below it, so it clips no result); the optical-depth accumulator uses `saturating_add`
before the cap (an unchecked `Fixed +` could overflow a large `tau_max`); the medium-aggregation is flagged
as the volume-mean-only limit with the aggregation-kernel-as-data follow-on named; the fluid-medium limit
reworded medium-agnostic (a fluid-dweller's dominant occluder); the endpoint sampling convention documented
and pinned by test; two doc precisions (byte-neutrality is from the absent caller, not the D=3 identity; the
`bulk_axis` doc de-Terran-ised). Reserved for calibration, surfaced not fabricated: the acoustic absorption
axis the floor does not yet carry (the dev acoustic row reuses the optical axis as a labelled stand-in, a
flagged floor gap); the confinement substrate that would set D below 3 (the geometric kernel already handles
D=2/D=1); the frequency-dependent absorption path and its body-resonance frequency source.

R1. **Founder band placement is an AUTHORED gameplay input, NOT an engine-solved cultural outcome. RESOLVED
   by the owner 2026-07-08.** The CONTINUED-4 living-world finding reported a seed-dependent collapse (a band
   spawning on a dry corner far from water starves) and surfaced "habitability-aware placement" as a candidate
   engine fix. The owner ruled: band placement is where the gameplay comes in, so it is legitimately authored
   (a player/scenario choice), a Principle-9 INPUT like the physics floor, not a cultural OUTCOME. If a band
   is placed somewhere barely survivable and dies, that is the player's choice faithfully simulated, not a bug.
   If a band claws through the hardship and, say, comes to resent its god for the suffering it endured, that
   emergent narrative is the fun. So the engine must NOT auto-place founders for survivability, and the
   seed-dependent death is correct behaviour, not a defect. Consequence for the finding: the ONLY genuine
   remaining engine gap is forage COMPETENCE (the recurrent controller freezing one tile short of REACHABLE
   known food, so a being dies where it should survive), because that corrupts the hardship signal: death must
   come from real scarcity, not from a controller that cannot walk to visible food. The emergent payoff the
   owner named (sustained material hardship shaping a band's beliefs, e.g. resentment of its god) is the
   demonstration target that rides on the axiom/belief kernel (Parts 21/28) plus the harm-learning felt-
   experience path. Keep the recurrent controller as the emergent foundation; the run-matched dawn bootstrap
   (train forage competence under partial knowledge on real terrain, alien-general food matter) is the work
   that makes survival reflect genuine hardship rather than a forage bug.

R2. **How first-hand felt experience feeds a being's convictions: the LEARNED-COUPLING framing. RESOLVED by
   the owner 2026-07-08 via a blind framing panel.** Before wiring "hardship shapes belief" (the resent-god
   loop), the proposed framing was: felt hardship enters the axiom-update kernel as generic SIGNED EVIDENCE on
   a conviction axis, a magnitude plus a DIRECTION toward a pole, the resulting shift emerging from the being's
   disposition. A fully-blind panel (six agents, three types, three models, each seeing only the guiding
   principles and the de-narrativized statement, none seeing the author's or owner's conclusion,
   AGENTIC_ADDENDUM.md section 10) UNANIMOUSLY and independently caught the seam: the DIRECTION clause authors
   the exact coupling it claims to forbid. Deciding that hardship bears on providence and points to its
   negative pole reads the high-level MEANING of the experience to produce the outcome, the kin-template
   violation (Hamilton's rule as a mechanism) relocated from kinship to belief. Verified against source: the
   magnitude side is floor-derivable and clean; the axis-and-pole selection is the authored crux; the alien
   test confirms it (magnitude generalizes to any being with a reserve, an authored direction needs
   per-species authoring). THE RESOLVED FRAMING to build: first-hand felt experience emits ONLY a magnitude
   (interoceptive salience of the reserve-delta) and a valence sign (the floor sign of the delta), with NO
   axis and NO pole; which conviction it bears on, and in which direction, is a per-being LEARNED COUPLING
   (the same associative/credit-assignment primitive that already lets a being learn "this ground harms me"
   from felt reserve-fall, extended so a conviction can be credited), with the pole following from whether the
   felt outcome confirmed or disconfirmed the stance the being was acting on; where no association exists the
   felt event changes no conviction (the honest default). So "hardship erodes faith" is a DESCRIPTION of a
   learned outcome for some beings and its opposite for others, never a coded route. CONCRETE TRAP recorded:
   do NOT reuse `affect.rs`'s `AppraisalBinding` (a per-race `DriveId -> AffectAxisId` table) for axioms; it is
   legitimate for affect (a felt-coloring layer close to an innate disposition) but would be the violation for
   axioms (which hold emergent cultural content). This is the framing for the next build; the design-doc Part
   28 consolidation happens when it is built. Reserved: the interoceptive-signal-to-salience scale (basis: the
   existing salience range of social events), owner's to set.

R5. **Branch-2 (credit-assignment) framing CORRECTED by a third blind framing panel (2026-07-08); one OPEN
   fork surfaced for the owner.** The move half of the R2 substrate. The first-cut framing decided the pole a
   conviction moves toward by comparing the sign of its accumulated felt-experience association A (Branch 1) to
   the sign of its current stance s (same sign = confirm, strengthen toward the current pole; opposite = erode,
   flip toward the other). A fully-blind six-agent framing panel (three types, three models, de-narrivatized,
   none seeing the conclusion) UNANIMOUSLY (6 of 6, none "sound-as-is") caught it, and I verified the algebra
   against source: the comparison is VACUOUS. In both branches the kernel target reduces to `sign(A)` (same-sign:
   target = sign(s) = sign(A); opposite-sign: target = -sign(s) = sign(A)), so s never affects the direction.
   The actual rule is "reserves rose this span -> move the stance toward the numerically +1 pole; reserves fell
   -> toward -1," for every axis and every world. That AUTHORS the axis's meaning: it fixes the +numbered pole
   as the thriving/good pole everywhere, the exact "read the meaning of a symbol/axis" the template case forbids
   (P9 / the value-authoring line). The panel's FORMAL TEST (the load-bearing tool): the move must be INVARIANT
   under relabeling an axis (negating which pole is +1 must leave every being's physical trajectory unchanged);
   the first-cut fails it, a correct move must pass it. THE CORRECTED FRAMING (mandatory, all six): make the move
   relabel-invariant by making the association POLE-REFERENCED. Branch 1's accumulator changes its engagement
   weight from `|stance|` to the SIGNED stance, so `A = sum(felt.valence * intensity * stance)` and `sign(A)` is
   a per-being LEARNED, relabel-invariant fact ("which pole was good to hold, for me"); Branch 2 then feeds the
   AGM kernel `toward = sign(A)`'s pole, magnitude `|A|`, gated by entrenchment. Under relabel, stance -> -stance
   so A -> -A and the target tracks the SAME physical pole (invariant, meaning-free). This captures the owner's
   target: a being that HELD "providence is benevolent" and SUFFERED accrues A pointing away from that pole and
   erodes/flips toward its opposite (resentment), whichever numeric sign the world gave the poles. THE OPEN FORK
   (gp-sonnet's deeper catch, surfaced for the owner): even relabel-invariant, `target = sign(A)` hardcodes ONE
   epistemic polarity, "felt-good confirms a conviction, felt-bad erodes it," which forecloses the real cultural
   mode where felt HARDSHIP VALIDATES a conviction (asceticism, martyrdom, costly-signal belief). The fix is a
   per-being (or per-race) epistemic-polarity disposition `p` mediating the move (`target = sign(p * A)`): p>0
   hedonic, p<0 ascetic, so which epistemology a being/lineage has EMERGES rather than being authored. Options
   the owner is deciding between: (a) p as a per-race innate EPISTEMIC DISPOSITION (a field on `EpistemicStance`,
   the sibling of dogmatism, P9-legal authored input, hedonic default, per-race variable, seeded and inherited
   like the axiom seeds); (b) p as a per-individual HERITABLE genome-expressed trait, founder-zero, selected
   (gp-sonnet's strict reading, fullest emergence, but the coupling only fires once selection lifts p); (c) a
   hedonic always-on base now (like the floor-grounded harm/reward learners) with the ascetic-p generalization
   flagged as a follow-on. Reserved either way: the move threshold/rate (basis: the axiom kernel's existing
   entrenchment and plasticity), and p's default and range. The relabel-invariance fix is not optional and is
   built regardless; only the polarity fork awaits the owner. OWNER CHOSE (a) 2026-07-08: p is a per-race innate
   EPISTEMIC DISPOSITION (a field on `EpistemicStance`, the sibling of dogmatism), a P9-legal authored input
   with a hedonic default and per-race variation, seeded at the dawn and inherited like the axiom seeds, so a
   hedonic race and an ascetic race are data rows and asceticism is representable without foreclosure. Built on
   this basis.

R6. **The experiential-conviction arc PASSED the mandatory five-lens audit; follow-on refinements surfaced.**
   The whole arc (Prereq A felt_salience, Prereq B the conviction-percept threading, Branch 1 the correlation
   record, Branch 2 the conviction move) was audited by the five standing lenses plus correctness (six blind
   panelists, then adversarial per-finding verification, 19 agents), each finding re-verified against source.
   VERDICT: SOUND. The load-bearing claims held under blind scrutiny (relabel-invariance of the Branch-2 move,
   byte-neutral opt-in, weight-agnostic Branch 1, felt_salience alien-clean, the rising-reserve valence
   convention is the floor-wide normalized-level convention not a Terran more-is-better assumption). Confirmed
   findings were fixed (a determinism-contract RES_WORLD phase-access omission; the felt-move overflow/abs
   robustness; the retention doc's inaccurate basis; the flat-gate "entrenchment" overclaim) or answered with
   tests (the per-race polarity proven through the real dawn-seeding and inheritance path; a hedonic-being
   reinforce test showing the move turns on the lived correlation, not only the polarity knob). FOLLOW-ON
   REFINEMENTS the owner may want, none blocking (recorded here so they are not lost): (a) the ENTRENCHMENT-RANK
   -SCALED felt move gate (today the felt move uses a flat gate and does not read the axiom's entrenchment rank,
   so a labile and a calcified conviction are equally movable by felt experience; the rank-scaled gate over the
   reserved entrenchment curve is the faithful refinement); (b) a from_manifest fail-loud calibration read for
   FeltConvictionCalib (the shared follow-on with the other opt-in learners' dev-fixture calibs, due when a
   Calibrated production scenario first arms the learner; none does today); (c) felt_salience is a NET
   reserve-health primitive, so reserve-neutral churn (trading one reserve for another) reads calm and the felt
   intensity scales with how many reserve axes a world declares; a churn-sensitive or per-axis-normalized felt
   measure is a refinement of the Prereq-A primitive; (d) run_world's dev races all use the hedonic polarity
   default (the ascetic path is proven by test through the dawn, but no dev scenario yet declares an ascetic
   race to watch it in a run). The Part 28 design-doc consolidation (a Part 62 record and the Decided-and-
   reserved blockquote, per R2's "the design-doc Part 28 consolidation happens when it is built") is the
   remaining formal step, surfaced for when the owner wants the arc folded into the canonical design document.

R4. **Branch-1 (correlation-record) framing CORRECTED by a second blind framing panel (2026-07-08).** The R2
   substrate's Branch 1. The first-cut framing defined a conviction's eligibility for felt-experience
   association as (evolved controller weight x stance value), glossed as "the degree the conviction influenced
   behaviour this tick." A fully-blind six-agent framing panel (three types, three models, de-narrivatized,
   none seeing the conclusion) caught the SAME seam UNANIMOUSLY (6 of 6, none "sound-as-is"), verified against
   source: (1) STATICNESS: both factors are constant within a life (controller weights change only across
   generations; stance is read-only on the run path, moving only by cadence-gated enculturation that run_world
   does not even arm), so weight x stance is a per-being CONSTANT MASK recomputed to the same value every tick,
   the "decaying trace" does no credit assignment, and it becomes non-defeasible glue between a persistent trait
   and an outcome (a template-case / P9 violation, indistinguishable from authoring). (2) MOTILITY-PARASITISM:
   gating on the behaviour weight means a founder's unweighted convictions, and any conviction on a
   sessile/immobile being, can never form an association however strongly their value tracks the felt swings, so
   the substrate's coverage is silently contingent on motility (an admit-the-alien / P8 violation) and can only
   rediscover what selection already wired into movement. THE CORRECTED FRAMING (derived, preserves R2's core):
   drop the behaviour-weight coupling; mirror R2's own cited precedent, the harm learner (which correlates
   felt-harm with the varying perceived feature weight-agnostically, defeasible via a BENIGN counter-signal).
   Branch 1 is a per-being DECAYING SIGNED ACCUMULATOR per HELD conviction axis: each felt event folds the felt
   summary (`physiology::felt_salience` valence x intensity) into the accumulator of each conviction the being
   currently holds, engagement-weighted by the conviction's strength (|stance|), with a retention decay so it
   tracks RECENT lived valence and can un-form (defeasible). It reads NO behaviour weight (weight-agnostic,
   alien-clean); it changes no conviction and no behaviour (inert recording; Branch 2 consumes it); a being that
   holds no conviction or a world that does not arm the learner records nothing (byte-neutral honest default).
   Per-axis divergence emerges from WHEN each conviction was held relative to the being's lived valence (a
   conviction held through hard years accumulates negative, one acquired after fortunes improved accumulates
   positive), so the selectivity is lived-contingent, not seed-pinned. DIVERGENCE FROM R2's WORDING, surfaced
   for owner review (not blocking, derivable): R2 said the pole follows "whether the felt outcome confirmed or
   disconfirmed the stance the being was ACTING ON"; the panel found the literal behaviour-coupled reading of
   "acting on" IS the defect, so the corrected framing uses "held while experiencing" (weight-agnostic), which
   is MORE faithful to R2's own cited harm-learner primitive and preserves R2's core (felt experience emits only
   magnitude + valence; which conviction it bears on is a learned per-being coupling). HONEST LIMIT (unchanged,
   panel-confirmed): across-conviction attribution is DIFFUSE at the controller-percept tier (a felt outcome
   folds into every held conviction, not one specific conviction); crisp per-conviction attribution needs the
   deliberative tier, a future refinement. Reserved: the association retention/decay rate (basis: the
   eligibility-decay and evidence-ring rates the reward and harm learners already use, set equal for
   consistency) and the engagement weighting (|stance| the interim; uniform the alternative), owner's to confirm.

R3. **Lifespan must be DERIVED from anatomy, not authored. OWNER DIRECTIVE 2026-07-08, a note to honor when
   the lifespan/R-AGING work is built (not built now, surfaced so it is recalled then).** Today `Race.lifespan_years`
   and `maturity_years` are authored per-race numbers (a plain owner-set count, `crates/sim/src/race.rs`,
   design Part 20), and the individual-tier mortality rolls each being against an owner-supplied age-hazard
   curve (`World::apply_mortality`). This is data-driven and per-race differentiable (a short-lived and a
   long-lived race are different data rows, the R-AGING keystone, built and tested), but the lifespan is not
   grown from the body. The owner's directive: it must be DERIVED from the race's own anatomy and physiology,
   the way `physiology::derive_base_drain` derives metabolism from the body, so lifespan follows from body mass
   and metabolic rate (the mass-longevity and rate-of-living scalings), organ integrity and repair capacity,
   and the body's own physics, and a large slow-metabolism race lives for decades or centuries while a small
   fast one lives briefly BECAUSE of its body rather than a typed value. The authored number is the interim
   scaffold; the target is a senescence law that reads the being's own body (the derive-not-author line: author
   only in the physics floor, grow the rest), so a magical / silicon / photosynthetic race gets its lifespan as
   a data row from its own anatomy, and medicine (design Part 22/34) later modifies it. Recorded durably at the
   `lifespan_years` field doc-comment and the R-AGING design flag (`docs/audit.md`) so the builder sees it.
   Not a blocker for the current arc; a directive for the lifespan build.

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
