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

A survey of the discrete-event-simulation and agent-based-modelling literature confirmed most of the
technique mapping above, corrected one framing, and added concrete named systems. The verification was
adversarial: a fan-out research pass ran the full search and fetch and then checked each extracted
claim against its primary source, sustaining sixty-seven claims and refuting eight, which are dropped;
the eight refutations caught a real over-claim in the first draft of this section, folded below. Each
load-bearing point cites a primary source.

The determinism crux is the ordering of simultaneous events, and the field offers three clean answers,
of which the engine takes the first. Event simultaneity, two events at the same simulated time, is
where reproducibility is won or lost: the result depends on the order the simultaneous events execute,
and an ad-hoc or thread-dependent tie-break is the one route that breaks it. The three principled
answers are (a) impose a total tie-break order that is a function of the event data, the standard
discrete-event practice and the route this engine takes through R-CMD-ORDER's key (tick, region, id,
phase, ordinal); (b) leave the order formally unspecified, treating any order as consistent with the
specification and adding a tie-break rule only per-application where determinism is required, the
symmetric-DEVS-for-agents position, which drops classic DEVS's select function outright (Goldstein and
colleagues 2018); or (c) assign each simultaneous event a pseudo-random-derived tie-break value in an
extended virtual-time vector, deliberately unbiased and seed-randomizable so that changing the seed
samples other valid orderings (McGlohon and Carothers 2021). The correction the verification forced is
worth stating plainly: option (c) is not a content-based key and does not endorse one; it exists
precisely because content-based ordering can leave events incomparable and imposes a bias, and a raw
payload comparison is provably incomplete since byte-identical events cannot be distinguished. The
engine wants reproducibility rather than an unbiased sample, so a fixed content-based order (a) is the
right choice, its determinism coming from the key being total and data-derived, with the seed
available as a final discriminator only if two events are ever otherwise incomparable.

Parallel DEVS confirms the engine's choice is one of the field's settled ones. Classic DEVS (Zeigler,
Theory of Modeling and Simulation) resolved simultaneous internal transitions with a select function,
an authored tie-breaker that serialised colliding events; Parallel DEVS (Chow and Zeigler 1994)
replaced it with the confluent transition function, a formal construct that combines simultaneous
internal and external transitions rather than ordering them, and the later multicomponent variant
carries the same confluent rule and a bag of simultaneous inputs (multiPDEVS, Foures, Albert, Nketsa
2018). So the field's determinism-clean options for simultaneity are a total order, a confluent rule,
or a declared-non-deterministic order made deterministic per-application, and an ad-hoc thread-
dependent order is the only one that breaks reproducibility. The engine takes the total order because
it already holds the key.

The event queue is a fast but distribution-dependent structure rather than an unconditional constant.
The pending-event set is served in near-constant time by the calendar queue (Brown 1988), but the O(1)
is an experimental result for standard priority-increment distributions with the bucket width tuned
near the average event separation, and Brown himself notes it degrades when the priority distribution
is very non-uniform or changes drastically, while adjacent doubling-and-halving resize thresholds can
thrash toward O(N) per operation under boundary oscillation. Comparative benchmarks bear out that the
best structure is application-dependent: the ladder queue outperforms a splay tree beyond a few
thousand events and the gap widens with scale (Franceschini, Bisgambiglia, Bisgambiglia 2015), yet at
large sizes the calendar and lazy queues give good average but very long worst-case access while skew
heaps and splay trees hold the best worst-case (Ronngren and Ayani 1997). So the queue is a solved but
tunable problem, efficient in the common case with a worst-case the engine must bound, and it is keyed
by the total tie-break order above so efficiency and reproducibility stay independent concerns.

The saving and its limit are both confirmed by named systems. Event-driven multi-agent kernels realise
the sleeping-agent design exactly: ABIDES (Byrd, Hybinette, Balch 2019) routes every interaction
through one central priority queue, activates an agent only through a wakeup call or a received message
with no polling loop, and has background agents self-schedule their next wakeup at a chosen interval,
which is the self-scheduled crossing of sub-problem 2; its reproducibility, though, comes from being
single-threaded (same seed, guaranteed identical, with the same-timestamp order left arbitrary), a
weaker guarantee than the thread-count-independence this engine requires, which is exactly why the
total-order key matters here and did not there. The crossover of sub-problem 5 is confirmed with a
quantified case from event-driven molecular dynamics: a discrete, event-driven potential outperforms
the continuous, time-stepped form at gas densities but is significantly slower at high densities
(Thomson, Lue, Bannerman 2014), the same shape as the design's honest limit, that event-driven
execution wins where interaction is sparse and loses where it is dense, so a dense region falls back to
a flat sweep.

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

The primary sources behind the external-literature section, grouped by sub-problem, each carried
through the adversarial verification pass. Where the sources gave an inconsistent author list, the
entry uses "and colleagues" rather than commit to a wrong order.

The DEVS formalism and simultaneous-event handling:

- Zeigler, B. P., Muzy, A., Kofman, E. (2018). *Theory of Modeling and Simulation*, 3rd ed. Academic Press. The DEVS formalism, first introduced by Zeigler in the 1976 first edition, and its simulation semantics.
- Chow, A. C. H., Zeigler, B. P. (1994). "Parallel DEVS: a parallel, hierarchical, modular modeling formalism." *Proceedings of the 1994 Winter Simulation Conference*, 716-722. Replaces the classic-DEVS select function with the confluent transition function for simultaneous events.
- Foures, D., Albert, V., Nketsa, A. (2018). "multiPDEVS: a parallel multicomponent system specification formalism." *Complexity* 2018, article 3751917 (DOI 10.1155/2018/3751917). The confluent transition function and the bag of simultaneous inputs, with state collisions as an added conflict class in the nonmodular approach.
- Goldstein, R., and colleagues (2018). "A symmetric formalism for discrete event simulation with agents." *Proceedings of the 2018 Winter Simulation Conference* (Autodesk Research). Drops the classic-DEVS select function and treats simultaneous-event order as unspecified unless a per-application tie-break is added.

Deterministic ordering of simultaneous events:

- McGlohon, N., Carothers, C. D. (2021). "Unbiased deterministic total ordering of parallel simulations with simultaneous events" (arXiv:2105.00069). Extends virtual time with pseudo-random-derived tie-break values for a deliberately unbiased, seed-randomizable total order, deterministic for a given seed.
- Piccione, A., Pellegrini, A. (2023). "Practical tie-breaking for parallel and distributed simulations." *DS-RT 2023*. A content-based deterministic tie-break, provably incomplete as a fully general scheme since byte-identical events are indistinguishable.
- "Simultaneous events and lookahead in simulation protocols." *ACM Transactions on Modeling and Computer Simulation* (DOI 10.1145/361026.361032). The reproducibility problem posed by simultaneous events.

The future-event list (the event-queue data structures and their limits):

- Brown, R. (1988). "Calendar queues: a fast O(1) priority queue implementation for the simulation event set problem." *Communications of the ACM* 31(10), 1220-1227. Experimental O(1) for standard priority-increment distributions, degrading under very non-uniform or rapidly changing distributions.
- Tang, W. T., Goh, R. S. M., Thng, I. L. J. (2005). "Ladder queue: an O(1) priority queue structure for large-scale discrete event simulation." *ACM Transactions on Modeling and Computer Simulation* 15(3), 175-204.
- Ronngren, R., Ayani, R. (1997). "A comparative study of parallel and sequential priority queue algorithms." *ACM Transactions on Modeling and Computer Simulation* (DOI 10.1145/249204.249205). At large sizes the calendar and lazy queues give good average but poor worst-case access, while skew heaps and splay trees hold the best worst-case.
- Franceschini, R., Bisgambiglia, P.-A., Bisgambiglia, P. (2015). "A comparative study of pending event set implementations for PDEVS simulation." *DEVS Integrative M&S Symposium (SpringSim '15)*, 77-84 (DOI 10.5555/2872965.2872976). The ladder queue outperforms a splay tree beyond a few thousand events; the best structure is application-dependent.

Event-driven agent kernels and the density crossover:

- Byrd, D., Hybinette, M., Balch, T. H. (2019). "ABIDES: towards high-fidelity market simulation for AI research" (arXiv:1904.12066). A discrete-event multi-agent kernel: one central priority queue, agents activated only by a wakeup or a message with no polling, background agents self-scheduling their next wakeup; reproducible by being single-threaded.
- Amrouni, S., and colleagues (2021). "ABIDES-Gym: gym environments for multi-agent discrete event simulation and application to financial markets" (arXiv:2110.14771). The discrete-event multi-agent taxonomy (time-step versus event-time) and the sleeping-agent self-wakeup pattern.
- Thomson, C., Lue, L., Bannerman, M. N. (2014). "Mapping continuous potentials to discrete forms." *Journal of Chemical Physics* 140, 034105 (arXiv:1309.7292). Event-driven, discrete molecular dynamics outperforms time-stepped, continuous form at gas densities but is significantly slower at high densities, a quantified density crossover.
