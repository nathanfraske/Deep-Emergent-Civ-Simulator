# Temporal level of detail: a research and scoping pass

This is a research and scoping pass on Part 32 (Temporal Level of Detail), the design's own "hardest
single addition," recorded there as a direction to research rather than a settled design. It is
written for the owner to weigh before any build. It preserves the existing documentation and intent:
nothing in Part 32, Part 54 (R-TIER-CONSIST), or the R-VIEW-ELAB flag is changed by this pass; it
decomposes the problem, maps each piece to the engine's existing machinery or to a known external
technique, and separates what is achievable from the one part the design already marks as
open. A companion section at the end scopes R-VIEW-ELAB, the view-time elaboration flagged open at
Part 54, since temporal LOD and view elaboration are the two halves of one bargain, run the world
cheap and watch it in full. The binding commitments it honors, quoted from the design so they are
not drifted:
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

## The external literature, verified

A deep-research pass (fan-out search, source fetch, and three-vote adversarial verification of each
claim) confirmed the technique mapping above against the primary sources and sharpened three points.

The coarse population step is tau-leaping, and the conservation-exact form is binomial. Gillespie's
tau-leaping draws each event channel's count over a coarse step as a Poisson variable with mean rate
times the step, leaping over many events at once (Gillespie 2001; Cao, Gillespie, Petzold 2005,
2006). Its faithfulness has a precise bound, the leap condition: the step must be small enough that
no rate changes significantly over it (bounded by a tolerance times the total rate), and the largest
such step is the coarsest faithful one (Cao, Gillespie, Petzold 2006). The unbounded Poisson leap can
overshoot and drive a small population negative, exactly near exhaustion, so the form the engine
wants is binomial tau-leaping, which caps each channel's count by the population available to undergo
it and thereby conserves mass exactly (Chatterjee, Vlachos, Katsoulakis 2005; Cao, Gillespie, Petzold
2005, "Avoiding Negative Populations"). This is load-bearing here: the binomial cap is the temporal
analogue of R-TIER-CONSIST's exact conservation, so the coarse leap conserves the population and stock
quantities exactly rather than approximately, which is what sub-problem 3 requires. A single control
parameter, the critical-firing threshold, tunes continuously between the fast inexact leap (threshold
zero) and the exact per-event simulation (threshold infinity), with a value around five to fifteen
usually best (Cao, Gillespie, Petzold 2005), a principled reserved knob for how coarse a region runs.

The failure boundaries are the demotion triggers. The leap breaks precisely where coarsening is
inappropriate: near exhaustion (a population crashing), under stiffness (widely separated fast and
slow timescales, where implicit tau-leaping steps larger but damps the fast fluctuations, Rathinam,
Petzold, Cao, Gillespie 2003), and where rare pivotal events dominate. So the technique's boundary
coincides with the region's significance boundary, and the leap condition is the promotion signal.

Cross-region reconciliation is conservative parallel discrete-event simulation. Regions at different
local clocks are logical processes, and the two synchronization families are the canonical dichotomy:
conservative (lookahead, no rollback) and optimistic (rollback, no lookahead), per Jefferson and
Barnes citing Fujimoto. The conservative family (Chandy-Misra-Bryant with null messages to break the
deadlock a waiting cycle would cause, Fujimoto 2017) is deterministic and schedule-independent: a
process advances only with a proven lower bound on all future incoming timestamps, and that bound,
the lookahead, is the minimum send-to-receive delay, which for this engine is the travel or
propagation time of a migration, a caravan, an army, or a rumour, a physical fact of the world's
geography and Part 46 infrastructure. Optimistic Time Warp (Jefferson 1985) runs ahead speculatively
and rolls back on a straggler, its rollback and anti-message activity a function of message arrival
timing rather than the seed, which is the determinism hazard the engine cannot adopt. A subtle
result, surfaced by the research: even an optimistic simulator can be made externally deterministic
if a total event order with explicit tie-breaking, deterministic handlers, and reproducible per-
process RNG streams are all enforced (Jefferson and Barnes, Unified Virtual Time), but the
conservative family gives the same determinism without the rollback machinery, so it stays the
choice.

The fundamental limit is provable and shared across all of these, and it matches what the design
already states. Every coarse technique can conserve exactly-conserved quantities (through the bounded
binomial draw) and reproduce low-order statistics (the mean, and in the linear regime the variance to
first order), but none can reproduce the exact trajectory of a nonlinear or chaotic fine system. A
span dominated by sharply changing rates, near-exhaustion, stiffness, or rare pivotal events is
exactly where coarse advancement stops being faithful and the region must run fine. This is the
busy-time fast-forward bound the design set, now with its primary-source grounding, and it confirms
the honest shape: quiet time is cheap and conserving, busy time is bounded by its own importance.

Bit-reproducibility is achievable, and the recipe is stated. Key every coarse draw and the fine steps
it stands in for to the same reproducible seed-derived stream (a hash of the region, the event
channel, and the virtual-time interval, the engine's existing draw-key pattern), and use conservative
synchronization at region boundaries, so promoting or demoting a region does not change the committed
result and the world stays bit-identical across runs and machine counts.

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
tau-leaping against the acceptable drift in conserved quantities); the critical-firing threshold, the
single knob that tunes a region continuously between the fast inexact leap and exact per-event
simulation by declaring a channel critical (and so simulated event by event) once its remaining
population falls within that many firings of exhaustion (basis the tau-leaping literature's measured
optimum, a value around five to fifteen, traded against the per-tick budget: lower runs cheaper and
coarser, higher runs more exactly and dearer, Cao, Gillespie, Petzold 2005); the per-region
coarsening idle threshold (how long a region must be canonically quiet before it demotes, basis the
significance thresholds Part 54 already reserves, on the time axis); and the cross-region lookahead
floor (the minimum inter-region propagation delay, basis the travel and communication times the
world's own geography and Part 46 infrastructure already imply, read not invented). None is
fabricated here.

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

## The build factors and their prerequisites

Naming the technique for each sub-problem is not the whole scope. A build needs every load-bearing
factor named, and the mapping above had left several implicit. Enumerated in full, the build factors
and the open items they depend on are these, so the scope is closed in the sense that matters: no
load-bearing factor is unnamed and the prerequisites order the work.

The per-subsystem coarse-step registry, and the catalogue it needs. The registry mechanism is clear:
each subsystem declares a coarse advance that is the temporal analogue of the spatial restrict,
conserving its declared projections. What the pass had not enumerated is the catalogue itself, which
of the engine's systems already carry a coarse advance (the ecology does), which need one built
(demography, stocks, culture, belief diffusion, institutions), and which are fine-only, so their
canonical activity forces a region to fine resolution rather than being coarsened at all. That
catalogue is the first thing the coarse-step-registry stage produces.

The event-channel and rate model. Tau-leaping draws a count for each event channel from that
channel's rate over the leap, so a build must enumerate the demographic and social channels a quiet
region emits (a birth, a death, a feud, a small migration, per Part 8.3, Part 15, and Part 25) and fix
where each channel's propensity is read from the aggregate pool state. The channel set is data
(Principle 11); the reading is the build factor.

The fixed-point deterministic distribution sampler. The counter-based RNG gives a reproducible uniform
draw; tau-leaping needs a Poisson count and binomial tau-leaping needs a binomial count, and turning a
uniform draw into either exactly in Q32.32 is a specific algorithm rather than a free consequence of
the RNG. This sampler is determinism-sensitive and was unnamed in the mapping. It is a named build
factor now, and it couples to R-GPU-CANON-PIN, which pins the multiply, divide, and transcendental
emulation such a sampler uses, so the sampler must be declared canonical under that item.

The conservative cross-region synchronization core. This is the per-region future-event list, the
lookahead-and-null-message protocol that lets a region advance on a proven lower bound of incoming
timestamps, and the total canonical order on simultaneous inter-region events. It is the same
event-queue substrate as event-driven agent execution (Part 57's second foundation) at the coarser
granularity of a region rather than an agent, so the two levers share machinery and should be built to
one design.

The stiffness policy, a scoping choice surfaced rather than hidden. A region with widely separated
fast and slow timescales is where explicit tau-leaping fails; the simplest faithful answer is that
such a region promotes to fine, and the more complex answer is implicit tau-leaping (Rathinam et al.
2003). The recommendation is promote-on-stiffness, so stiffness joins exhaustion and rare pivotal
events as a demotion-refusing trigger, and implicit tau-leaping stays a later option rather than a
first-build requirement.

The entity-handoff-across-clocks protocol. When a migrating individual, a promoted mind, or a caravan
crosses from a region at one local clock to a region at another, the transfer must be deterministic
and conserving. It rides on the lookahead (the crossing takes at least the propagation time) and on
R-TIER-CONSIST's lift and restrict (the entity is lifted into the destination's resolution), but the
handoff itself is a build factor the cross-region core owns.

The event-driven promote-and-demote schedule, wired to significance and the leap condition, with the
hysteresis the reserved idle threshold sets, so a region does not thrash between resolutions.

Underneath the factors sit prerequisites, open items a temporal-LOD build depends on rather than
merely couples to. The load-bearing one is R-PROJ-REGISTER. A coarse step conserves exactly only what
each subsystem has registered as an exact projection, and today only population and wealth are
registered, with belief mass folded as a truncating mean and genetics, ecology, language, and
institutions registering nothing. Until each registers an exact integer partition, a coarse span
cannot conserve them exactly, so the conservation guarantee sub-problem 3 rests on is only as complete
as that registry, and R-PROJ-REGISTER is a hard prerequisite rather than a sibling. The determinism
harness (R-HARNESS-COVER) is the second: proving that a region run coarse-then-fine and one run fine
throughout agree bit for bit on conserved state, across runs and thread counts, needs the harness to
exercise the resolution path, which the current harness (a pure per-entity accumulation) does not.
R-GPU-CANON-PIN is the third, through the sampler above. R-CMD-ORDER and R-REDUCE-ORDER, already
named, pin the total order on inter-region events and non-associative combines. R-SAVE-SCHEMA is the
last: a saved world holds regions at different local ticks and resolutions, which the snapshot schema
must version and restore.

The ordering falls out of the prerequisites. R-PROJ-REGISTER and the fixed-point sampler come first,
because the conservation guarantee and the coarse draw depend on them; then the per-subsystem
coarse-step registry with its catalogue and rate model; then the conservative cross-region core shared
with event-driven execution; then the promote-and-demote schedule. The honest limit sits outside the
factor list, because it is a property of nonlinear dynamics rather than a thing to build.

---

# The companion mechanism: R-VIEW-ELAB, watching the aggregate in full

Temporal level of detail and the coarse-processing tier make the world cheap by running most of it
as aggregates: a quiet region is a population distribution, the sparse coarse state of its named
sentients, its stocks and culture, and its event log, not ten thousand simulated bodies. R-VIEW-ELAB
is the other half of that bargain, and the design flags it open at Part 54: Part 54 settles the ruling
(the camera never promotes anything to canon; only significance and the seed do) but not the
mechanism. This section scopes that mechanism, since it is the direct answer to "keep everyone as a
cheap aggregate, yet let the observer zoom in and watch ten thousand people go about their days." It
preserves the Part 54 ruling and the R-VIEW-ELAB flag's requirements unchanged; it specifies how they
are met.

## Two layers: the aggregate is the truth, the crowd is a dramatization

The canonical layer holds a coarse region wholly but as statistics, identities, and events rather
than bodies: the population as a distribution (occupations, ages, wealth), the permanent identity and
sparse coarse state of every sentient (no one is lost, since every person keeps a stable identity and
history whether or not anyone watches, Part 54), the prevailing culture, beliefs, and stocks, and the
recorded event log. That is everyone whole as an aggregate: the truth is all present, as numbers and
sparse records, and it costs almost nothing. The crowd an observer sees on zooming in is a second,
non-canonical layer the view invents from that truth, renders, and discards.

## The core trick: seeded, stateless invention

The view stores no crowd. It generates each individual as a pure function of a coordinate, the
counter-based-RNG idea the engine already runs on, but view-side, so it may use floating point and
the GPU and need not be bit-identical, only consistent on re-look. Individual number `k` in region
`R` at canonical time `T` is a draw keyed on `(R, T, pool_seed, k)`, and from that seed the view
draws everything about them (age, occupation, appearance, and the activity they are performing this
second) by sampling the region's actual distribution: if the pool is three-fifths farmers, about
three-fifths of the invented people farm; if a famine event is on record, they read as hungry and the
granary is empty. The crowd is therefore a faithful sample of the aggregate truth rather than a
fabrication divorced from it, and because each person is a function of the coordinate rather than a
stored object, zooming out and back, or two observers looking at the same region at the same
canonical time, see the same crowd doing the same things. This is reproducibility without storage,
the R-VIEW-ELAB analogue of the canonical draw schema, one level down and non-authoritative.

## Motion between coarse updates

The canonical layer updates coarsely; the view wants smooth per-second motion at the base tick. The
elaboration runs each invented person's micro-life view-side and seeded: a walk to the well, a field
tended, a stopped conversation, generated from the person's seed and sampled occupation, animated
smoothly between the coarse canonical anchor points. This is the locomotion and evolved-behaviour
machinery the engine already has (drives, non-omniscient perception, physics-bounded walking, the
controller), run on ephemeral invented individuals and discarded each frame rather than written to
canon, so the same code renders the crowd it would otherwise simulate.

## The two identity cases

The flag names the distinction, and it is load-bearing. The anonymous mass (the ten thousand
peasants) is invented wholesale from the pool seed: no canonical identity, ephemeral render puppets,
regenerated identically on re-look, discarded when the camera leaves. A named sentient run at coarse
processing (a person who exists canonically but not at full fidelity) is animated from their real
coarse state rather than invented: the view elaborates the actual farmer, his real occupation and
sparse beliefs and location, into per-tick motion. Invented-from-nothing for the mass;
animated-from-coarse-truth for the named. The second case is where R-VIEW-ELAB touches R-LIVELINESS,
since generating a named person's plausible daily routine from sparse coarse state is the same
question that item asks, and the two should be settled together.

## The one-way wall

The elaboration is structurally unable to write canon: it lives on the non-canonical side of the
typed `Canonical`/`NonCanonical` boundary the core already carries (design Part 58, Part 3.4), so a
write of authoritative state from the elaboration is a compile error, not a discipline to remember. A
duel watched between two invented peasants does not become a canonical event, they never gain
identities, and nothing they do feeds back. If something consequential should happen in that region,
it is the canonical significance-and-seed schedule (Part 54) that promotes the region or a person to
real fidelity, driven by the world and the seed and never by the camera, so looking changes nothing
and stays reproducible (Principle 10). This is what lets an observer zoom into any of a million
villages and watch, for free, without the act of watching altering a single fate.

## What it reuses, what is open, and the reserved values

R-VIEW-ELAB reuses almost entirely what exists: the counter-based seeding (a view-side draw key), the
aggregate pools, the locomotion and evolved-behaviour machinery for the micro-lives, and the
`NonCanonical` wall for the boundary. The open, hard parts are the ones the flag names: the faithful
and cheap projection from pool statistics to a sampled individual, so the crowd never contradicts the
canonical demographics, beliefs, or recorded events; the named-sentient daily-routine generation from
sparse coarse state (shared with R-LIVELINESS); and rendering ten thousand disposable bodies at the
frame rate, which is GPU-instanced procedural work, affordable precisely because it is view-side and
thrown away. It rides on temporal level of detail for the quiet spans that make regions coarse in the
first place, but it is its own mechanism and does not depend on temporal LOD being built to work over
a region already run coarse by significance. What is reserved, surfaced not set: the base-tick
duration that fixes the near-one-second elaboration resolution (already reserved); and the sampling
fidelity budget (how many invented individuals a zoom elaborates before it degrades to a coarser
crowd, a performance bound on the view rather than a canonical value, so it may be a view-side setting
rather than a reserved calibration). No canonical value is fabricated here, since the elaboration
writes no canon.

## The two halves together

Temporal LOD and R-VIEW-ELAB are the two halves of one sentence: run the world cheap, and watch it in
full. Temporal LOD (with the coarse-processing tier and event-driven execution) makes a planet of
civilizations affordable by spending fidelity only where significance and the seed put it; R-VIEW-ELAB
lets the observer stand anywhere in that cheap world and see it teeming, by inventing a faithful,
seeded, disposable dramatization of the aggregate truth that costs nothing and cannot lie about what
is canonically true. Both are significance-and-seed mechanisms with a strict one-way boundary to
canon, and both should be prototyped in isolation before either is committed, as Part 32 asks of its
half.

---

## Sources

The primary sources behind the external-literature section, grouped by sub-problem, cited so the
owner and any later build can trace each claim to its origin. These are the long-form grounding; the
technique mapping in the body is the consolidation.

Coarse population advancement (tau-leaping and its conservation-exact form):

- Gillespie, D. T. (2001). "Approximate accelerated stochastic simulation of chemically reacting systems." *Journal of Chemical Physics* 115(4), 1716-1733. The tau-leaping method: leaping over many reaction events in one step by drawing each channel's firing count as a Poisson variable with mean rate times the step.
- Cao, Y., Gillespie, D. T., Petzold, L. R. (2006). "Efficient step size selection for the tau-leaping simulation method." *Journal of Chemical Physics* 124(4), 044109. The leap condition and its adaptive step selection: the coarsest faithful step is the largest one over which no propensity changes by more than a tolerance.
- Cao, Y., Gillespie, D. T., Petzold, L. R. (2005). "Avoiding negative populations in explicit Poisson tau-leaping." *Journal of Chemical Physics* 123(5), 054104. The critical-firing threshold that separates critical from non-critical channels, and the value range (about five to fifteen) that trades speed against exactness.
- Chatterjee, A., Vlachos, D. G., Katsoulakis, M. A. (2005). "Binomial distribution based tau-leap accelerated stochastic simulation." *Journal of Chemical Physics* 122(2), 024112. Binomial tau-leaping: capping each channel's count by the population available to undergo it, so mass is conserved exactly and populations cannot go negative.
- Rathinam, M., Petzold, L. R., Cao, Y., Gillespie, D. T. (2003). "Stiffness in stochastic chemically reacting systems: the implicit tau-leaping method." *Journal of Chemical Physics* 119(24), 12784-12794. Implicit tau-leaping for stiff systems: stepping over fast timescales while damping their fluctuations, and the boundary where explicit leaping fails.

Cross-region reconciliation (parallel discrete-event simulation synchronization):

- Chandy, K. M., Misra, J. (1979). "Distributed simulation: a case study in design and verification of distributed programs." *IEEE Transactions on Software Engineering* SE-5(5), 440-452. Conservative synchronization with lookahead and null messages to break deadlock.
- Jefferson, D. R. (1985). "Virtual time." *ACM Transactions on Programming Languages and Systems* 7(3), 404-425. The Time Warp mechanism: optimistic execution with rollback, anti-messages, and global virtual time, the optimistic family this engine declines for its rollback-timing nondeterminism.
- Jefferson, D. R., Barnes, P. D. (2022). "Virtual time III, part 1: Unified virtual time synchronization for scalable parallel discrete-event simulation." *ACM Transactions on Modeling and Computer Simulation* 32(4), 26. The conservative-versus-optimistic dichotomy and the conditions (total event order, deterministic handlers, reproducible per-process streams) under which even an optimistic simulator is externally deterministic.
- Fujimoto, R. M. (2017). "Parallel and distributed simulation." *Proceedings of the 2017 Winter Simulation Conference*, 111-125. The survey framing of both synchronization families and the role of lookahead as the minimum send-to-receive delay.

Multiscale and coarse-grained integration (the fundamental limit):

- Kevrekidis, I. G., Gear, C. W., Hyman, J. M., Kevrekidis, P. G., Runborg, O., Theodoropoulos, C. (2003). "Equation-free, coarse-grained multiscale computation: enabling microscopic simulators to perform system-level analysis." *Communications in Mathematical Sciences* 1(4), 715-762. Coarse projective integration and the equation-free framework: advancing a system at the coarse level from short bursts of fine simulation, and the limit that only coarse observables, not the fine trajectory, are recovered.
