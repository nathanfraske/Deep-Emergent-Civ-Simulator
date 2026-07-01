# Temporal level of detail: a research and scoping pass

This is a research and scoping pass on Part 32 (Temporal Level of Detail), the design's own "hardest
single addition," recorded there as a direction to research rather than a settled design. It is
written for the owner to weigh before any build. It preserves the existing documentation and intent:
nothing in Part 32, Part 54 (R-TIER-CONSIST), or the R-VIEW-ELAB flag is changed by this pass; it
decomposes the problem, maps each piece to the engine's existing machinery or to a known external
technique, and separates what is achievable from the one part the design already marks as
open. The binding commitments it honors, quoted from the design so they are not drifted:
identical-outcomes agreement between a coarse and a fine tier is "rejected as mathematically
unattainable for nonlinear dynamics (Leontief, Theil) rather than merely costly" (Part 54); "cheap
fast-forward through busy time would require coarse stepping that reproduces fine stepping exactly,
and that remains unsolved" (Part 54); and the tier schedule is significance-and-seed, not the camera
(Principle 10), with the base-tick duration a reserved value that bounds how fast significance can
promote a region.

## Why this is worth researching now

The profiling of the dawn tick (`docs` handoff, the `tick_bench` harness) made the need concrete.
After a 5x optimization, one hundred beings run at about 85 microseconds per tick, and at the
owner-set base tick of one in-world second that puts one hundred in-world years at full per-tick
fidelity at roughly seventy-five hours of wall time for one hundred people, before a whole planet of
civilizations is considered. Full fidelity does not reach deep time on one machine. Either the world
is run at reduced fidelity where nothing is watching or at stake (temporal level of detail), or the
spans the vision wants are out of reach. This pass is the honest investigation of whether, and how,
the reduced-fidelity path can be made deterministic.

## The reframe: temporal LOD is not one problem

Part 32 reads as a single hard research direction. Decomposed against what the engine already has, it
is several sub-problems, and most of them are either already built here or map to a known technique
with a settled determinism story. One sub-problem is hard, and the design already names it
and marks it bounded rather than solved. Separating them is most of the work, because it turns "the
hardest single addition" into a mostly-tractable build plus one honest limit, rather than a monolith
to be feared.

The five sub-problems:

1. The coarse step: how a quiet region advances its pools, stocks, and aggregate state over a long
   span without simulating individuals per tick.
2. The event statistics: how the demographic and social events a quiet region still emits (births,
   deaths, feuds, migrations) are produced over a long leap without stepping each one.
3. The coarse-fine relationship: what a coarse step is allowed to promise relative to the fine steps
   it stands in for, and therefore what "the same result for the same elapsed time" can mean.
4. The cross-region reconciliation: how regions sitting at different local clocks interact correctly
   and deterministically when a migration, a trade, an army, or a rumour crosses between them.
5. The schedule: how a region is promoted to fine resolution when something matters and demoted when
   it does not, deterministically and driven by significance rather than the camera.

## Sub-problem 1: the coarse step already exists in part

A coarse statistical advancement of populations is not hypothetical here; it is built and proven. The
pre-dawn radiation epoch (`crates/sim/src/epoch.rs`) advances species as allele-frequency pools by
Wright-Fisher generations, not by stepping individuals, and the biosphere's stocks (Part 15,
`crates/sim/src/stocks.rs`) step logistically toward carrying capacity in closed-form fixed-point
arithmetic. That is exactly a coarse temporal tier for the ecology: a generation, or a stock step, is
a coarse advance over a span that a fine per-individual simulation would take many ticks to cover.
The pieces temporal LOD needs for the coarse step are therefore, for the ecology, present. What is
open is generalizing the coarse step to the other subsystems a quiet region carries (aggregate
beliefs and their diffusion, culture drift, demographics, the aggregate institution state of Part 36)
and defining each subsystem's coarse step as the temporal analogue of the spatial restrict operator
R-TIER-CONSIST already provides. The coarse step is not a new invention so much as a registry of
per-subsystem coarse advances, sibling to the conserved-projection registry Part 58 already carries.

## Sub-problem 2: the event statistics are a leap, and the leap is a known technique

A quiet region still has a history: a birth here, a death there, a feud, a small migration. Part 8.3
already specifies that an aggregate pool "runs no per-agent logic; a pool-level system samples its
statistics each period and emits events with probabilities derived from the pool's composition." Over
a long coarse span, stepping each event is exactly what temporal LOD wants to avoid. The technique for
this is well established in stochastic simulation: rather than simulate each event, sample how many
events of each type occurred over the whole leap from the event rates, and apply them at once. This is
tau-leaping (Gillespie), and its faithfulness has a stated condition (the leap condition: the leap is
valid while the event rates do not change appreciably over the span), with variants that keep the
counts physical (binomial and multinomial tau-leaping bound the number of events by the population
available to undergo them). For this engine the mapping is direct and determinism-friendly: the coarse
leap draws its event counts from counter-based RNG keyed on the region, the canonical span, the seed,
and a phase, so the coarse leap and the fine steps it replaces draw from the same reproducible source
and the leap replays bit for bit. The honest boundary is the leap condition itself: when a region's
rates are changing fast (a population crashing, a war igniting), the leap is no longer faithful, which
is precisely the signal that the region should be at fine resolution, so the boundary of the technique
coincides with the boundary of when coarsening is appropriate.

## Sub-problem 3: the coarse-fine relationship is already ruled, and it is conservation, not identity

This is where Part 32's phrasing needs to be read against Part 54, and the reading resolves a tension.
Part 32 asks that "the coarse and fine paths must produce the same canonical result for the same
elapsed time." Taken literally that is the thing Part 54 proves impossible: identical-outcomes
agreement between tiers is mathematically unattainable for nonlinear dynamics. The resolution is that
temporal LOD inherits R-TIER-CONSIST's guarantee unchanged: a coarse step is the temporal analogue of
the spatial restrict operator, and what it promises is that it conserves, in integer and fixed-point,
the registry of declared projections each subsystem carries, not that it reproduces the fine
trajectory. The population total, the accumulated knowledge level, the conserved belief mass, the
stock quantities are conserved exactly across a coarse span; the exact identity and micro-history of
who did what to whom is not, because it was never canonical at coarse fidelity. This is the same
lossy-lift, exact-restrict contract Part 54 already audits every transition against, applied to the
time axis. Reframing Part 32's "same result" as "conserves the declared projections" is not a
weakening; it is aligning Part 32 with the ruling Part 54 already settled, and it is what makes the
quiet-span coarse step both cheap and deterministic.

## Sub-problem 4: cross-region reconciliation is a solved field (with a determinism caveat)

Regions advancing at different local clocks that must reconcile when they interact is not a novel
problem; it is the central problem of parallel discrete-event simulation, studied since the late
nineteen-seventies, and it has two families of answer with opposite determinism properties. The
optimistic family (Time Warp, Jefferson) lets each region run ahead and rolls back when a late message
from another region arrives out of order; rollback makes the outcome depend on timing and is therefore
a determinism hazard this engine cannot adopt. The conservative family (Chandy-Misra-Bryant) never
rolls back: a region only advances its local clock as far as it can prove no other region can send it
an earlier-timestamped event, using lookahead, the minimum delay before one region can affect another.
For this engine lookahead is not an abstraction to invent; it is a physical quantity the world already
has, the travel or propagation time of the thing that crosses a border. A migration, an army, a trade
caravan, or a rumour takes a bounded minimum time to cross from one region to another, and that bound
is the lookahead that lets a quiet region safely advance without waiting. Conservative synchronization
keyed on canonical event order (the R-CMD-ORDER and R-REDUCE-ORDER disciplines already flagged) gives
a bit-reproducible reconciliation independent of how the regions were scheduled. So the cross-region
sub-problem has a settled technique whose determinism properties are exactly the ones this engine
requires, and its one input (lookahead) is already a physical fact of the world.

## Sub-problem 5: the schedule is R-TIER-CONSIST's schedule, on the time axis

A region drops to fine resolution when something significant happens (a war, a discovery, a
catastrophe) and coarsens when the drama passes. This is the significance-and-seed schedule Part 54
already defines for spatial and processing fidelity, applied to temporal fidelity, and it inherits
Part 54's guarantees: promotion and demotion are driven by canonical significance and the seed, never
by the camera, and every demotion is a conserving restrict. The camera's role is bounded to
non-authoritative view elaboration (R-VIEW-ELAB), which shows a watcher a coarsely-run region in
apparent per-tick detail without ever writing canon, so looking at a backwater cannot promote it. The
base-tick duration, already a reserved value, bounds how fast significance can promote a region. So the
schedule is not a new mechanism; it is the existing significance schedule extended to time, with
R-VIEW-ELAB the sibling that handles the camera's separate, non-canonical wish to watch.

## What is open, and the honest limit

Two things remain real research, and one is bounded rather than solvable.

The genuine build research: the per-subsystem coarse-step registry (defining, and determinism-
auditing, the coarse advance and its conserved projections for each subsystem beyond the ecology,
which is done), and the conservative cross-region synchronization core with lookahead derived from the
world's own propagation delays, wired to the event-driven promotion and demotion of a region's
resolution. Both are buildable and both have a settled determinism story; they are the work, and they
are prototype-in-isolation-first work as Part 32 says.

The bounded limit, preserved exactly from Part 54: cheap fast-forward through a busy, significant span
is not made possible by any of this. Coarsening is faithful only while the leap condition holds and the
region is not canonically active; a span that is truly active is bounded by the cost of computing
its activity at the fidelity that activity demands, because skipping or coarsening an active span would
require a coarse step that reproduces the fine trajectory exactly, which is unattainable for nonlinear
dynamics. Temporal LOD makes quiet time cheap and leaves busy time bounded by its own importance, which
is the correct and honest shape: the observer never wants to fast-forward through the war they are
watching, only through the quiet centuries around it, and those are exactly the spans the coarse tier
serves. This limit is not a gap to be closed by more engineering; it is a property of nonlinear
dynamics, and the design already states it.

## The determinism contract temporal LOD must meet

Every piece above is compatible with the engine's determinism bedrock, and the contract is statable:
a coarse step draws its randomness from counter-based RNG keyed on the region, the canonical span
covered, the seed, and a registered phase, so the coarse advance is a pure function of those
coordinates and replays bit for bit, and the coarse step over a span and the fine steps that would
cover the same span draw from a shared, reproducible source rather than a running stream; the cross-
region reconciliation is conservative (no rollback), with all inter-region events applied in a total
canonical order (R-CMD-ORDER, R-REDUCE-ORDER); the promote and demote schedule is a function of
canonical significance and the seed, never the camera or the wall clock; and every coarse-to-fine and
fine-to-coarse transition conserves the declared projections exactly in integer and fixed-point
(R-TIER-CONSIST), so a region's canonical timeline is a function of the seed and the world alone
(Principles 3 and 10) whatever resolution path it took through time.

## Reserved values, surfaced not set

The base-tick duration, already reserved, and, new to this pass and surfaced with their bases for the
owner to set: the leap-condition tolerance (how much a region's event rates may change over a coarse
span before the leap is refused and the region promoted, basis the statistical-faithfulness bound of
tau-leaping against the acceptable drift in conserved quantities); the per-region coarsening idle
threshold (how long a region must be canonically quiet before it demotes, basis the significance
thresholds Part 54 already reserves, on the time axis); and the cross-region lookahead floor (the
minimum inter-region propagation delay, basis the travel and communication times the world's own
geography and Part 46 infrastructure already imply, read not invented). None is fabricated here.

## What a build would look like, staged

If the owner directs a build after this pass, the shape, each stage prototyped in isolation and
determinism-audited before the next, as Part 32 asks: first the per-subsystem coarse-step registry
extending the ecology's existing coarse tier, with a determinism proof that a coarse span conserves
its declared projections exactly; then the tau-leap event emission for the aggregate demographic and
social events, with the leap condition as the promotion trigger; then the conservative cross-region
synchronization core with lookahead from propagation delays, proven bit-reproducible independent of
region scheduling; then the event-driven promote and demote schedule wired to significance, with
R-VIEW-ELAB the separate non-canonical view path. The honest gate Part 32 sets stands: if a single
machine proves able to hold the full-fidelity world over the spans the vision wants without this, it
is unnecessary; the profiling says it cannot, so this is the mechanism that makes deep time run, and
it is the temporal completion of the level-of-detail principle the engine otherwise only half
realizes.
