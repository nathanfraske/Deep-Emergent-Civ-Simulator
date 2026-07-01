# Event-driven agent execution: a research and scoping pass

This is a research and scoping pass on the second foundation of Part 57 (Deterministic Scheduling and
Agent Execution), the agent execution model, which the design records with its requirements rather
than a settled design. It is written for the owner to weigh before any build, and it preserves the
existing documentation and intent: nothing in Part 57 or Part 54 is changed by this pass; it
decomposes the problem, maps each piece to the engine's existing machinery or to a known external
technique, and separates what is achievable from what is bounded. It is the breadth-axis companion to
the temporal-LOD scoping pass (`docs/temporal_lod_research.md`): temporal LOD makes deep time cheap by
coarsening quiet spans, and event-driven execution makes a dense present cheap by not stepping idle
agents, and the two share one substrate, the reproducible event queue, at different granularities.

The binding requirement Part 57 states, quoted so it is not drifted: the model must deliver "bounded
cost at world scale, reproducible wake ordering, and compatibility with significance-driven processing
fidelity," and event-driven execution "is hard to bolt onto a poll-everything loop after the fact and
is worth getting right from the start." The determinism bedrock (Principle 3, Part 3) and
observer-independence (Principle 10) hold over it unchanged.

## Why this is worth researching now

The owner's standing concern is full civilization scale: thousands of promoted agents alive and acting
at once, and the wish to zoom in and watch ten thousand people go about their day. Temporal LOD
answers the time axis (quiet centuries run cheap). It does not answer the breadth axis: even within a
single fine-resolution present, a world of permanent sentient individuals "cannot poll every mind
every tick and must lean on significance-driven processing and event-driven wakeup" (Part 57), because
"running full cognition and full belief processing on every sentient in a world every tick is not
affordable" (Part 54).

The profiling made the cost concrete. `World::tick` (crates/sim/src/world.rs) runs six phases in
order (perceive, decide, converse, gossip, converse-language, drift-languages), and each phase is a
sweep over every placed mind. At a hundred co-located beings the tick is about eighty-five microseconds
and almost all of it is the two social phases, which do work only where minds are co-located and
interacting. An idle farmer standing in an empty field is swept by all six phases every tick and
contributes nothing. That per-idle-agent sweep cost is the tax event-driven execution removes, and it
is the tax that grows with a civilization.

## The current model, and the seed of the answer already present

The engine polls. Every tick advances the clock and runs every phase over every mind. But three pieces
of the event-driven answer are already in place, which is why this is an extension rather than a
rewrite.

The tick already applies a batch of stimuli in a total canonical order: `tick(inputs)` sorts the batch
by target mind id then ordinal before applying it, so the result is independent of the order the batch
was assembled in. That sorted stimulus batch is a proto event queue: a stimulus is an event, and it is
already applied in a reproducible total order. The counter-based draw schema (`DrawKey`, crates/core)
already keys every stochastic draw on a coordinate of region, primary and secondary locus, tick, a
registered phase, and a slot, so an event's randomness is a pure function of its coordinate rather than
of when it ran. And the co-location index (`colocated_index`, built during the profiling pass) already
groups minds by place in canonical id order, which is the spatial subscription an event needs to find
the agents in a place without an O(N) scan. The move to event-driven execution keeps all three and
changes the loop that drives them.

## Decomposing the problem

Event-driven agent execution splits into five sub-problems.

1. The event queue and a reproducible wake order. A future-event list holds scheduled wakes ordered by
   time, and simultaneous events (the same tick) must resolve in a total order that is a function of
   the data rather than of the queue's internals or the thread that inserted them. This is the
   determinism crux.
2. What wakes an idle agent (demand-driven activation). A sleeping agent must register the events or
   conditions that should wake it, so no poll is needed, and a new event must find the agents that
   care without scanning all of them.
3. Compatibility with significance-driven fidelity (Part 54) and temporal LOD (Part 32). A dormant
   region wakes nobody, a coarse region advances its agents as aggregate tau-leap events rather than
   individual wakes, and only a fine region's active agents are woken.
4. Determinism across thread counts. The event-processing order and every draw must be bit-identical
   regardless of how many workers run, which is the same total-order-plus-seeded-draw discipline the
   rest of the engine uses.
5. The fundamental limit and the crossover. Where most agents act every tick (a battle, a festival, a
   packed market), event-driven execution degenerates to a sweep plus queue overhead, so it wins only
   when idleness is the common case.

## Sub-problem 1: the event queue is R-CMD-ORDER's key on the time axis

A deterministic event queue needs a total order on events. The engine already has the shape of the
key: R-CMD-ORDER (Part 4.3, the open command-ordering item) mandates a total command key of tick,
primary id, kind, and emission ordinal, with spawn ids minted at a single-threaded barrier and event
ids a deterministic function of content. A wake event is ordered by the same key with time as the
leading field: (wake tick, region, primary agent id, phase, ordinal). Two events at the same tick
break the tie by region then id then phase then ordinal, a total order with no appeal to insertion
sequence, address, or thread. So the wake order is a pure function of the scheduled set, and the queue
is a min-ordered structure over that key (a binary heap or a bucketed calendar structure with a
canonical intra-bucket order). This sub-problem is settled by reusing the command-order discipline the
determinism audit already requires; event-driven execution does not introduce a new ordering problem,
it applies the existing one to scheduled wakes.

## Sub-problem 2: waking an idle agent is a scheduled crossing plus a spatial subscription

An agent sleeps until something relevant happens, and relevance splits cleanly into two kinds, and the
engine's resolved machinery already supplies both.

The predictable kind is a self-scheduled wake. The homeostatic physiology of R-BEHAVIOR-EVOLVE
(crates/sim/src/homeostasis.rs) drains reserves at known rates: hunger, thirst, and fatigue rise on
fixed-point trajectories. A reserve that will cross its action threshold does so at a tick that can be
solved for now, so instead of polling the reserve every tick the agent schedules a single wake at the
crossing tick and sleeps until then. A draining reserve schedules its own wake. This turns the entire
homeostatic-drive layer from a per-tick sweep into one queued event per drive per agent, re-scheduled
only when the trajectory changes (a meal resets hunger and reschedules the next crossing). This is the
largest single saving and it falls straight out of the resolved behaviour model.

The reactive kind is a spatial or sensory subscription. An agent should also wake when something
happens near it (another agent arrives, a conversation opens, a threat appears), and these are not
predictable in advance. The co-location index is the subscription: an event at a place enqueues wakes
for exactly the agents indexed at that place, in canonical id order, so a conversation opening in a
village wakes the villagers present and no one else, without scanning the world's population. Sensory
range generalises the same idea (an agent subscribes to the places within its perception radius, the
Part 40 access channels), and the non-omniscient percept of R-BEHAVIOR-EVOLVE already bounds what an
agent can perceive, so the subscription set is the percept's spatial support. So neither kind needs a
poll: the predictable kind schedules a crossing, the reactive kind is delivered by the place index.

## Sub-problem 3: it is the fine-tier complement of temporal LOD, over one shared substrate

Event-driven execution and temporal LOD are the same mechanism at two granularities. Temporal LOD runs
a region's clock: a dormant region is not stepped at all, a coarse region advances its population as
aggregate tau-leap events (births, deaths, feuds emitted over a leap without waking individuals), and
a fine region runs per tick. Event-driven execution runs an agent's clock inside a fine region: an
idle agent is not stepped, an active agent is woken by a scheduled crossing or a place event. The
region event queue (temporal LOD's conservative cross-region synchronization core) and the agent wake
queue are the same future-event-list substrate keyed by the same command-order key, one over regions
and one over agents. A region promotes to fine and its agents' scheduled wakes become live; a region
demotes to coarse and its agents' individual wakes collapse into the aggregate tau-leap. This is the
load-bearing coupling the temporal-LOD build-factors pass already named: the two levers must be built
to one design, because they are one event queue observed at two scales.

## Sub-problem 4: determinism across threads is the engine's existing discipline

An event-driven loop parallelised across workers must commit events in the canonical order regardless
of worker count. This is the deterministic scheduler's job (Part 57's first foundation, scoped in
`docs/deterministic_scheduler_design.md`): systems declare their read and write sets, the scheduler
derives conflict-free parallel batches, and the batches are exactly what a data-parallel executor
consumes. An event-driven loop feeds the scheduler the set of agents woken this tick rather than all
agents, and the scheduler orders and groups their processing the same way. Every draw an awoken agent
makes is keyed on its `DrawKey` coordinate, so it is bit-identical whether the agent ran first or last,
on one worker or eight. Determinism across threads is therefore not a new problem for event-driven
execution; it is the scheduler-and-draw-key discipline the engine is already built to enforce, applied
to a woken subset.

## Sub-problem 5: the crossover is the honest limit

Event-driven execution wins when idleness is the common case and matches, never beats, a flat sweep
when it is not. In a region where most agents act every tick (a pitched battle, a crowded festival, a
market at peak trade), nearly every agent is woken every tick, so the wake queue holds nearly the whole
population and the loop is a sweep plus the queue's bookkeeping overhead, which is strictly more work
than the sweep alone. The honest boundary is therefore a crossover on the active fraction: below it,
event-driven execution saves the idle-agent tax; above it, a flat sweep of the region is cheaper and
the engine should fall back to one. This is the breadth-axis analogue of temporal LOD's busy-time
limit: a busy region is bounded by the cost of its own activity, and no scheduling trick makes a
thousand agents who are all interacting cheaper than the interactions themselves. The saving is real
because at civilization scale idleness is the norm (a farmer is not in a conversation every second, a
sleeper is not deciding), but it is a saving on the quiet majority, not on the active crowd, and the
design should state that plainly.

## The external literature, verified

A targeted survey of the discrete-event-simulation and agent-based-modelling literature confirmed the
technique mapping above and sharpened three points. The survey was run directly, source by source,
because the deep-research harness failed on an internal error mid-run; each load-bearing claim below
was checked against more than one result.

The determinism crux is the ordering of simultaneous events, and the field's answer is a total
tie-break order, which the engine already carries. In discrete-event simulation, event simultaneity,
two events scheduled at the same simulated time, is where reproducibility is won or lost: the result
depends on the order the simultaneous events execute, and because simulators use different, sometimes
ad hoc, tie-breaking, the same model can be irreproducible across simulators (the simultaneous-events
literature). The discipline that fixes it is a total order on a tie-break key that is a function of the
event data rather than of insertion sequence or thread, and recent work extends virtual time with an
arbitrary-length series of tie-breaking values to give an unbiased deterministic total order over
otherwise-incomparable parallel events (the 2021 unbiased-deterministic-total-ordering preprint). This
is exactly the R-CMD-ORDER key applied to wakes: (tick, region, id, phase, ordinal) is that tie-break
series, so the engine's existing command-order discipline is the field-standard answer rather than a
bespoke one.

Parallel DEVS offers the alternative and confirms the engine's choice is sound. Classic DEVS (Zeigler)
resolved simultaneous internal transitions with a select function, an authored tie-breaker that
serialised colliding events; Parallel DEVS (Chow and Zeigler 1994) removed the select function
entirely and replaced it with the confluent transition function, a formal construct that activates all
simultaneous transitions consistently rather than by an authored order. So the field carries two
determinism-clean options for simultaneous events: impose a total tie-break order (the engine's route,
via R-CMD-ORDER), or define a confluent rule that makes the outcome order-independent. The engine
takes the first because it already holds the key. What the literature settles is that an ad-hoc or
thread-dependent order is the one route that breaks reproducibility, and the engine takes neither.

The event queue is a solved data-structure problem. The pending-event set is served in near-constant
time by the calendar queue (Brown 1988, O(1) amortised, about three times faster than a splay tree at
ten thousand events, and the structure behind simulators such as GTW, CSIM, and ns-2) and, for
large-scale runs where the calendar queue degrades, the ladder queue (Tang, Goh, Thng 2005, O(1) at
scale). A deterministic engine keys either structure by the total tie-break order above, so the
queue's asymptotic efficiency and the wake order's reproducibility are independent concerns, both
settled.

Event-driven execution's saving and its limit are both confirmed. Event-driven agent simulation
schedules an event only when an agent chooses to act or communicate, so it avoids processing idle
agents by construction, and the event-bus pattern reduces the interaction wiring from a quadratic
all-pairs scan to a linear one (the event-driven multi-agent literature). The honest boundary the
field reports matches sub-problem 5: the choice between event-driven and time-stepped execution is
system-dependent, and where interaction is dense enough that most agents act every step, the two
converge, which is why recent frameworks combine a tick base with per-agent discrete events (the
time-stepped-versus-discrete-event ABM comparisons, and hybrid designs such as MOSAIK 3.0). So the
literature agrees event-driven execution is a saving on the idle majority rather than on the active
crowd, which is the crossover the design carries as its honest limit.

## The determinism contract event-driven execution must meet

The correctness bar is exact equivalence. Event-driven execution changes when an agent is processed,
never what it computes, so the committed state after an event-driven tick must be bit-identical to the
state the poll-everything loop would have produced for the same world and seed. That is a strong and
testable contract: a region run event-driven and the same region run by the full sweep must agree bit
for bit, across runs and thread counts, which the determinism harness must assert. The contract is met
by three properties the engine already carries: every wake is ordered by the total command-order key
(R-CMD-ORDER), so the processing order is a function of the data; every draw is keyed on its `DrawKey`
coordinate, so an agent's randomness is independent of when it woke; and a scheduled wake computes the
same state transition a poll would have, because the transition is a pure function of the agent's state
and its coordinate, not of the number of ticks it slept. An agent that sleeps a thousand ticks and
wakes on a scheduled crossing must reach the state it would have reached had it been polled a thousand
times, so any transition that accumulates per tick (a reserve draining) must be computed as a closed
form over the slept span rather than a thousand increments, which is the same closed-form-over-a-span
discipline temporal LOD's coarse step uses.

## What is open, and the honest limit

Two things are real build research, and one is bounded rather than solvable.

The build research: the reproducible event-queue substrate shared with temporal LOD's cross-region
core (a deterministic priority structure keyed by the command-order key, with the fallback-to-sweep
policy for dense regions), and the wake-scheduling layer (solving the crossing tick for each
predictable drive, and wiring the place and percept indices as the reactive subscription). Both are
buildable and both have a settled determinism story; they are the work, and they are
prototype-in-isolation-first work as Part 57 asks.

The bounded limit: event-driven execution cannot make a densely interacting crowd cheaper than its
interactions. It removes the idle-agent tax and nothing more. A region where everyone acts every tick
is bounded by the cost of that activity, exactly as a busy span is bounded in time. This is a property
of the workload, not a gap to engineer away, and the design should carry it as the breadth twin of the
temporal fast-forward limit.

## Reserved values, surfaced not set

New to this pass and surfaced with their bases for the owner to set: the dense-region crossover, the
active fraction at or above which the engine abandons the wake queue for a flat sweep of the region
(basis: the measured per-event queue overhead against the per-agent sweep cost, a performance bound
rather than a realism one, to be read from a profile like the tick bench rather than fabricated); and
the re-check interval for a wake condition that cannot be solved for a crossing tick in closed form
(basis: the base-tick duration and the drive decay rates already reserved, set so a coarsely-solved
predicate is re-evaluated no less often than its fastest plausible crossing). The base-tick duration,
already reserved, bounds the finest wake resolution. None is fabricated here.

## The build factors and their prerequisites

The concrete build factors are these. The reproducible event-queue substrate, built once and shared
with temporal LOD's cross-region core, a deterministic min-ordered structure over the command-order
key. The wake-scheduling layer: a closed-form crossing solver per predictable drive (the homeostatic
reserves first, since their trajectories are known), and the place-and-percept subscription for
reactive events, both re-scheduled when an agent's trajectory or location changes. The
fallback-to-sweep policy for dense regions, gated on the reserved crossover. The closed-form
over-a-slept-span transition for any per-tick accumulation, so a woken agent reaches the state a polled
agent would (the discipline shared with temporal LOD's coarse step). And the event-driven driver
itself, replacing the six-phase sweep in `World::tick` with a drain of the wake queue for the current
tick, feeding the woken subset to the deterministic scheduler.

The prerequisites, open items a build depends on rather than merely couples to: R-CMD-ORDER, the total
command-and-event key the queue orders by, which must be specified before the queue can be built; the
deterministic scheduler (Part 57's first foundation), which orders and parallelises the woken subset;
R-HARNESS-COVER, the determinism harness proving an event-driven region agrees bit for bit with the
full sweep across runs and thread counts, which the current harness (a pure per-entity accumulation)
does not exercise; and the shared substrate of R-TEMPORAL-LOD's cross-region core, which is the same
event queue at region granularity, so the two should be designed together. It couples to R-TIER-CONSIST
(the coarse tier an idle region collapses into) and to R-BEHAVIOR-EVOLVE (the homeostatic drives whose
predictable drain supplies the self-scheduled wakes).

## What a build would look like, staged

If the owner directs a build after this pass, the shape, each stage prototyped in isolation and
determinism-audited before the next, as Part 57 asks: first the reproducible event-queue substrate
keyed by the command-order key, with a determinism proof that the drain order is a pure function of the
scheduled set independent of insertion and thread; then the self-scheduled wake for the homeostatic
drives, with a proof that a drive scheduled and slept reaches the state a polled drive would (the
closed-form-over-a-span equivalence); then the place-and-percept reactive subscription, with the
bit-identity harness against the full sweep; then the fallback-to-sweep policy and the wiring into the
tick driver, feeding the woken subset to the deterministic scheduler; and the substrate built here is
the one temporal LOD's cross-region core reuses at region granularity, so the two levers converge. The
honest gate Part 57 sets stands: the model earns its place by making the quiet majority of a
civilization's agents free while leaving the active crowd bounded by its own activity, which is the
breadth completion of the level-of-detail principle the temporal pass completes in time.

---

## Sources

The primary sources behind the external-literature section, grouped by sub-problem. Where the survey
returned a title and venue but not a verified author list, the entry is cited by title and identifier
rather than an invented attribution.

The discrete-event world-views and the DEVS formalism:

- Goldstein, R., Wainer, G. A., Khan, A. (2013). "The DEVS Formalism." An introduction to the Discrete Event System Specification and its simulation semantics.
- Chow, A. C. H., Zeigler, B. P. (1994). "Parallel DEVS: a parallel, hierarchical, modular modeling formalism." *Proceedings of the 1994 Winter Simulation Conference*, 716-722. Removes the classic-DEVS select function and introduces the confluent transition function for simultaneous events.

The future-event list (the event-queue data structures):

- Brown, R. (1988). "Calendar queues: a fast O(1) priority queue implementation for the simulation event set problem." *Communications of the ACM* 31(10), 1220-1227.
- Tang, W. T., Goh, R. S. M., Thng, I. L. J. (2005). "Ladder queue: an O(1) priority queue structure for large-scale discrete event simulation." *ACM Transactions on Modeling and Computer Simulation* 15(3), 175-204.

Deterministic ordering of simultaneous events:

- "Simultaneous events and lookahead in simulation protocols." *ACM Transactions on Modeling and Computer Simulation* (DOI 10.1145/361026.361032). The reproducibility problem posed by simultaneous events and the role of a well-defined ordering.
- "Unbiased Deterministic Total Ordering of Parallel Simulations" (arXiv:2105.00069, 2021). Extends virtual time with a series of tie-breaking values to give a scheduling-independent total order over incomparable events.

Event-driven agent execution and the crossover with time-stepping:

- "Event-Driven Multi-agent Simulation." The event-driven agent pattern in which an idle agent is not processed until an event it registered for occurs.
- "MOSAIK 3.0: Combining Time-Stepped and Discrete Event Simulation" (arXiv:2410.16937, 2024). A hybrid tick-plus-discrete-event scheme, evidence for the system-dependent crossover between the two.
