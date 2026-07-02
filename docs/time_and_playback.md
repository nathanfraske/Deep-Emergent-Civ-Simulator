# Time, tick rate, and the observer's playback

This note records how simulated time works in the engine, what is built for an arbitrary tick
rate and a live view, and what the full vision (watching people walk around at one in-world
second per real second, then speeding up to years per second) still requires. It is grounded in
the design document (Part 14, Part 32, Part 54, and Principles 3, 9, 10) and in the code as it
stands on the `claude/arbitrary-tickrate-step` branch.

## The one fact everything rests on

The engine has no running random-number stream and no wall-clock in canonical state. Every draw is
`DrawKey{region, locus, locus2, tick, phase, slot}.rng(seed).at(counter)`, a pure function of its
coordinate, and the tick is always one of those coordinates (`crates/core/src/keys.rs`,
`crates/core/src/rng.rs`). So the world state at tick N is a pure function of the seed and N, and
of nothing else: not of how many render frames passed, not of how fast the observer was playing,
not of where the camera pointed. This is Principle 3 (determinism) and Principle 10 (observer
independence), and the design states the consequence directly: "camera position, zoom, and chosen
timescale select what is rendered and how fast it plays ... but they never alter canonical state"
(Part 0, Principle 10), and "The observer's timescale control is a playback speed over that
canonical timeline rather than a change to it" (Part 54, design.md line 3058).

The practical result is that an arbitrary tick rate, pause, single-step, and fast-forward are
determinism-safe by construction, on one condition: the simulation advances only by whole ticks,
and real time never enters state. An adversarial audit of the tick path confirmed this and
surfaced two provisos, recorded below.

## What is built now

Three pieces, all determinism-safe and tested.

The time-control spine (`crates/sim/src/clock.rs`). A `Steppable` trait (advance one whole
canonical tick), a `SimClock` that carries the reserved base-tick duration for the world-time
readout, and a `PlaybackDriver`: a view-side accumulator that turns real elapsed seconds and a
chosen speed into a whole number of ticks to run, with pause, single-step, speed control, and a
per-frame catch-up cap that surfaces any surplus as `lod_debt` rather than running or dropping it
silently. The driver reads no clock of its own (the caller feeds it the real delta), so it is a
pure function of its inputs and is unit-tested, including the property that the total ticks over
many small frames matches the total over one big frame within a tick of float rounding, so the
observed advance is essentially independent of frame pacing.

The one-generation radiation stepper (`crates/sim/src/epoch.rs`). The pre-dawn radiation was a
batch loop that ran every generation in one call. It is now `step_generation` plus a `Radiation`
driver that owns a region's biosphere and advances exactly one generation per step, implementing
`Steppable`. The batch `run` is a thin wrapper over it. A test proves that stepping one generation
at a time is bit-identical to the batch, in both the summary report and the full allele-frequency
state, which is the determinism heart of the live view: watching the radiation unfold never
diverges from the canonical result.

The staged world genesis and the live viewer (`crates/sim/src/genesis.rs`,
`crates/viewer/src/main.rs`). A `WorldGenesis` driver runs worldgen and seeds the founders up
front, then advances every region's radiation one generation per step, and can produce a
`LivingWorld` snapshot of the current state at any point. Stepped to completion it is bit-identical
to the one-shot `genesis`. The windowed viewer now holds this driver and advances it through the
`PlaybackDriver` at the observer's chosen speed, decoupled from the render frame rate (the window
redraws at its own fps while the simulation advances by whole generations banked from real time).
Controls: space pauses, `.` and `,` speed up and slow down, `n` single-steps a generation, on top
of the existing pan and zoom. A HUD shows the generation reached, the playback speed, the alive
count, and any temporal-LOD debt. A headless `--radiate` mode prints the ecology unfolding
generation by generation and confirms the final state matches batch genesis, for inspection
without a display.

This realizes the coarse, deep-time end of the vision: you watch the pre-dawn ecology radiate,
species appearing and going extinct, at a speed you control, and it is the same world a one-shot
genesis produces.

## The two determinism provisos

From the adversarial audit of the tick path, a live loop must respect two rules. Both are about
what a live loop must not do, not about the mechanism as written.

Advance only by whole ticks. Varying the tick rate, pausing, single-stepping, and
fast-forwarding by running ticks faster are all safe. Skipping or coarsening a canonically-active
tick is not, because a coarse step that reproduces fine stepping exactly is the unsolved
temporal-LOD problem (Part 32). Cheap fast-forward is valid only through quiet, already-coarse
spans, never through busy ones.

Never trigger a birth or a promotion from a view-time path. Reproduction and promotion draws are
keyed on allocation-order ids (`crates/core/src/id.rs`, and the hazard notes in
`crates/sim/src/world.rs`). Those ids are observer-safe only while birth and promotion order is a
deterministic function of canonical state. They sit outside `World::tick` today, so the current
tick is pure, but any live loop that ever drives a birth or a promotion from the camera, the
wall-clock, or the render cadence would leak observer order into those streams. Births and
promotions must be driven by canonical tick logic alone.

## What dictates walking around: physics in, everything else emergent

Walking is not authored, and it is not a view animation. Under Principle 9 (the Steering Audit),
physics may be an authored input; a cultural or behavioural outcome may not. So what an engine may
put in for locomotion is the physics of it: the body's morphology and gait from the anatomy layer
(a heavy carnivore and a small forager move differently because their bodies differ, Part 25.14,
Part 35), the terrain's passability and cost (slope, water, biome, the subtile fractional
coordinates the design already reserves for smooth within-tile movement, design.md line 640), the
metabolic cost of moving, and the physical laws the physics crate floors. That is the "physics
in."

Everything above that emerges. Where a being goes and why is the spatial expression of its drives
resolving against what it believes and what its body and the ground allow. The pieces the engine
already has for this are the drive and utility-AI decision layer (Part 8, `crates/sim/src/
decision.rs`), the belief and perception core (Part 9, where a being's idea of where food, water,
kin, or danger is comes from what it has witnessed and been told), and the located-identity join
that can already move an occupant from tile to tile (`crates/sim/src/located.rs`). A hungry being
moves toward the forage it believes is there; a thirsty one toward water; a threatened one away
from what frightens it; pathfinding (Part 13) turns "I want to be there" into a route over passable
ground, and the subtile position carries it smoothly between tiles at the base tick. No wander
script, no migration table, no authored gait. Even seasonal migration is the climate fields plus
carrying capacity plus drives, not a rule that says "this race goes south in winter" (Part 17). The
Steering Audit is what forbids the authored version: you may make deep water block a non-swimmer,
you may not make a people migrate by decree.

This is why the fine "walking around" end is a build, not a viewer toggle. It is the emergent
locomotion path (drives to destination to path to a per-tick position update), done the emergent
way with only physics authored in, plus the bridge that puts the running dawn world's beings on
the map the viewer draws. The current dawn world (`World`) holds minds and language on an abstract
place token, not a map coordinate, and runs no movement; the biosphere occupants the viewer shows
are placed once and stand still. Building the fine end means giving beings map positions that
change per tick as their drives move them, and connecting that world to the viewer, all while
keeping births and promotions out of the view-time path per the proviso above.

## The full spectrum, and what each end needs

One in-world second per real second, everyone walking around, is the fine end. It needs the base
tick to be short enough for smooth movement (the design's own definition, line 3058), the emergent
locomotion path above, and the dawn-world-to-map bridge. It is the largest remaining build and is
mostly new simulation, not new plumbing; the time control this branch adds already drives it once
it exists.

Years per second is the coarse end. It cannot mean running tens of millions of base ticks per
second at full fidelity, and it cannot mean skipping active ticks. The design's answer, which this
note follows, is temporal level of detail (Part 32): quiet regions advance in coarse statistical
steps over many base ticks, so fast-forward through quiet time is cheap, while any region that is
canonically active is bounded by the cost of its own activity. To still see people moving in a
region that is being advanced coarsely, the view elaborates ephemeral per-tick motion that never
writes canon (R-VIEW-ELAB, seeded by region, canonical time, and pool seed so the same look shows
the same thing). Both Part 32 and R-VIEW-ELAB are open research in the design, not settled
mechanisms. The radiation view this branch ships is the coarse end for the pre-dawn epoch
specifically, where a generation is already a coarse deep-time step, which is why it works today
without the general temporal-LOD machinery.

The `lod_debt` the `PlaybackDriver` surfaces is the honest seam between the two ends: when the
chosen speed asks for more base ticks in a frame than the fine simulation can run, the debt is the
exact amount that coarse stepping would have to absorb. Today it is only reported; absorbing it is
the Part 32 work.

## Reserved values (surfaced, not set)

None of the mechanism above depends on these to run; it runs on labelled development fixtures. They
are the numbers that anchor world-time and the life cycle to real meaning, surfaced with a
recommendation and a basis, for the owner to set and confirm against a test. The agent does not set
them.

- Base-tick duration (world-seconds per base tick). Recommendation: 1.0, one in-world second per
  tick. Basis: the design's own definition of the base tick as the finest canonical timestep short
  enough that people and animals move smoothly (line 3058), and R-VIEW-ELAB's near-one-second view
  target (line 3078); it is already reserved in Part 54. Dev fixture: `SimClock::dev_default` =
  1.0. Confirm: at 1.0, playback speed 1.0 reads one in-world second per real second.

- Life-cadence period (base ticks per aging and mortality beat). Recommendation: derive from the
  base-tick duration and the Part 20 life-process rate (for example, a beat per in-world day is
  86400 ticks at a one-second base tick). Basis: the aging step is written to run once per this
  cadence and is deliberately not yet wired into the tick, waiting on this value rather than a
  fabricated one (`crates/sim/src/world.rs`, `age_step`/`apply_mortality`). Confirm: a cohort ages
  and faces mortality once per cadence and replays bit for bit.

- Generation-to-years mapping (in-world years per pre-dawn radiation generation). Recommendation:
  tie to the modelled organism generation time of Part 25 (the Wright-Fisher generation the drift
  step already uses), so the radiation view can label deep time in years rather than generations.
  Basis: the generation time is a biological property the owner sets per the world, not a display
  convenience to invent. Confirm: the radiation HUD's year readout equals generations times the set
  value.

The view-layer playback knobs are not reserved calibration values, because they never enter canon:
the default playback speed (dev default 4 generations per second in the viewer), the speed step
factor (2x), the rate bounds, and the per-frame catch-up cap. They are free to tune with signoff,
like the ecology development fixtures. Separately, how much visibly happens per generation in the
radiation view is governed by the existing R-BIOSPHERE epoch fixtures (the speciation cadence, the
species cap, the generation count); the current dev fixtures reach the species cap within about a
dozen generations, after which the view holds steady, so a livelier deep-time view is a matter of
those reserved epoch calibrations.

## Emergent locomotion, built, and three corrections toward the principle

The fine end's first slice is built (`crates/sim/src/locomotion.rs`, `examples/walkers.rs`):
positioned beings walk the map at a physics-bounded speed and settle where their needs are met.
Three corrections, each an application of "physics in, everything else emergent," shaped it and are
worth recording because they are the pattern the rest of the fine end must follow.

Mobility is the body, not the kingdom. A first pass gated movement on "is it a plant." That
templates the outcome: it forbids a walking tree. The fix is that walking is gated on the body's
locomotion organ, a morphological fact, and whether a body has one is itself an emergent draw, not a
rule keyed on trophic role. The generator's hard `sessile = producer` was replaced by a
`rooted_prior` draw, a strong tendency (an autotroph favours staying in the light) that is never
absolute, so a mobile autotroph and a sessile filter-feeder can both arise. The priors are reserved.

A being is not a god. A first pass let a being head for the nearest resource within a wide radius,
which is omniscience: it knew where water was without ever having seen it. The fix is that a being
knows only what it has perceived within a small true sensory range and remembered, and it navigates
by that belief; knowing of no satisfier, it explores to discover one, on a heading keyed on the
seed, the being, and the tick, so it earns its knowledge by moving through the world rather than
reading the map. Being told of a place it has not seen is the next layer (Part 9 gossip and
language).

Behaviour itself must not be authored, and this one is not yet fixed, only fixed in place as a
placeholder and flagged. The being still chooses from an authored list of drives and actions
(the decision layer): it has a thirst, and a way to relieve it is to seek water, because that menu
was written. A fixed behavioural repertoire chosen from outside the simulation is steering at the
level of behaviour. The end goal is that the policy is not authored at all: a being's homeostatic
state (energy, water, integrity) is a consequence of its body's physics, its motor options are the
affordances of its morphology, and the mapping from state to motion is a heritable policy expressed
from its genome that evolves under the pre-dawn epoch's selection, with fitness a consequence of
homeostatic survival rather than an authored objective. Seeking water when dry becomes a behaviour
the lineage came to have because the ones that did survived. This is the emergent-behaviour work,
R-BEHAVIOR-EVOLVE in the backlog; it grounds in the genome and the epoch's selection that already
exist, and it is the layer beneath everything above. The movement physics this slice builds is the
substrate such a policy would drive; the authored decision layer is the placeholder to replace.
