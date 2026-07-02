# The deterministic scheduler cluster: a design pass

This is a design proposal, not consolidated design, written for owner sign-off before the keystone
is built (Part 57 asks that the scheduler be given "a full brainstorm before it is locked"). It
covers the four items the red-team determinism audit and Part 57 raise together: the deterministic
scheduler (Part 57), R-CANON-WALK (Part 3.5), R-REDUCE-ORDER (Part 57), and R-CMD-ORDER (Part 4.3).
It also answers the owner's explicit question for this pass: whether to adopt hecs and Rayon now or
hold off.

## Where the code stands

Determinism holds today, but by convention rather than by structure. The tick is a fixed serial
sequence of phase methods; every canonical per-being store is a `BTreeMap`; the `Registry` carries a
sorted `entries_sorted` accessor; and `world.rs` states outright that this is "deliberately the
serial tick, not the parallel command scheduler," with the parallel form left for R-CMD-ORDER and
R-REDUCE-ORDER. Two pieces the original red-team flagged are already hardened: every canonical id
newtype now derives `Ord` (the audit bullet's claim that `PoolId` and `EventId` lack it is stale),
and the parallel-sum seam (C-05) is closed, `Fixed::sum_bits` folding in `i128` so a sum is
identical for any partition even when a prefix would overflow.

Since this pass was first drafted, the parallelism on-ramp has been built and adversarially audited,
so the substrate this design assumed it would create is now largely in place. The sanctioned
canonical-iteration helpers exist (`canonical_sorted` and `canonical_reduce` in `crates/core/src/canonical.rs`).
R-CANON-WALK is now enforced rather than a habit: a `clippy.toml` denies `std::collections::HashMap`/`HashSet`
in the canonical crates (core, sim, world), the three proven-safe sites carry a justified `#[allow]`,
the `EventLog` gained a `by_entity_sorted` ordered accessor, and a source-scan test in
`crates/core/tests/determinism.rs` fails if any canonical module grows an unordered container.
R-CMD-ORDER's mechanism is built and its barrier now surfaces a duplicate `CommandKey` rather than
leaking the worker schedule, and the determinism harness worker sweep is a real adversary again (it
drives a non-empty, thread-scrambled command set through the barrier). R-REDUCE-ORDER's two live
combines are both order-independent: the typology pick by construction, and the gossip conflict apply
after closing the order-dependence seam the audit found (a belief's candidate set is now the sorted
union of every asserted hypothesis, `InferenceFrame::merge_hyps`, rather than first-writer-wins).
What is now open is exactly one thing: there is no scheduler at all. The keystone is the last piece.

## The four items resolve as one

Part 57 says R-REDUCE-ORDER should be "settled together with the deterministic scheduler's declared
read and write sets." They form one substrate.

### The scheduler (Part 57)

Each system declares a read-set and a write-set over named data resources. The scheduler derives,
as a pure function of those declarations, a deterministic execution order plus safe parallel
batches: two systems conflict when one writes a resource the other reads or writes; conflicting
systems land in different batches and run in the order of a stable system id; non-conflicting
systems may share a batch. Because the schedule is a function of the sorted declarations alone, it
is deterministic and observer-independent (Principles 3, 10), and it is cheap to tune, the goal the
owner named: changing a declaration re-derives the schedule with no rewrite. The system id must be a
stable, canonical id (assigned like the phase registry, never registration order), or the schedule
would depend on registration order and cease to be deterministic. The scheduler is storage-agnostic:
a resource can be a `BTreeMap` store today or a hecs component column later, so it does not force the
storage decision (see the hecs question below).

The proposed algorithm is a deterministic layered assignment: walk systems in stable-id order; place
each in the earliest batch that has no resource conflict with any system already in it and that sits
after every lower-id system it conflicts with. This yields a fixed layering that respects conflicts
and orders every conflicting pair by id, so the same declarations always produce the same schedule,
and a parallel executor running each batch concurrently produces the same canonical state as running
the whole thing serially in the flattened order.

### R-CANON-WALK (Part 3.5)

The id-ordering half is done. What remains is structural: one sanctioned canonical-iteration helper
(a `canonical_sorted` that materialises any iterable in id or content-key order) as the single path a
hash, a save, a gossip selection, or a market clear takes, so a raw `HashMap` iteration cannot reach
a canonical stream; the rule that every canonical container is a sorted structure or carries an
ordered accessor (the Registry's `entries_sorted` is the model); and a determinism-harness check that
a container's canonical walk is insertion-order-independent (the Registry already has one). The few
remaining `HashMap`s (identity locations, the event provenance index, the calibration values, the
unit terms) are lookup-only or already guarded by a sorted accessor; the work is to make the
guarantee a structural rule rather than a habit, so the next feature cannot take the path of least
resistance the red-team named.

### R-REDUCE-ORDER (Part 57)

A single `canonical_reduce(items, key_fn, init, fold_fn)` that sorts the items by a total key before
folding, so any non-associative combine is a pure function of the data rather than of arrival or
thread order. It is the general form of what `sum_bits` already does for the associative sum: sort
first, then fold, and the result is order-independent by construction. The combine sites the audit
names (the gossip sub-batch conflict, the weighted pick, the migration renormalisation, the
technology product of efficiencies, the coupled-stock stepping, the graph traversal) each route
their fold through it with an explicit id or content key. Where a site already iterates a `BTreeMap`
its order is fixed and the helper is belt-and-braces; where a site draws from an unordered source the
helper is load-bearing.

### R-CMD-ORDER (Part 4.3)

This is the forward-looking one, because there is no parallel command stage yet. Its discipline can
be specified now and wired when that stage lands: a total command key of (tick, primary id, kind,
emission ordinal) so application never depends on thread count; spawn ids minted at the single-
threaded barrier rather than pre-minted in a parallel stage; and `EventId` a deterministic function
of canonical content. Until the command stage exists, the serial tick applies its stimuli in the
canonical (mind id, ordinal) order it already uses, which is the same key restricted to one kind.

### The scheduler core: the buildable shape

Now that the substrate is built, the keystone reduces to one new module (`crates/core/src/schedule.rs`),
with no new dependency and no calibration. The concrete shapes:

- A `ResourceId(u32)`: a named canonical data resource (a per-being store, the event log, an RNG
  stream, a field). Resources are a data-defined registry, so a new one is an id, not a code change
  (Principle 11).
- A `SystemId(u32)`: a stable canonical id per system (a tick phase, or a finer unit of work),
  assigned from a sorted declaration like the phase registry, never from registration order, so the
  schedule cannot depend on load order.
- An `Access { reads: BTreeSet<ResourceId>, writes: BTreeSet<ResourceId> }` per system. Two systems
  conflict when one writes a resource the other reads or writes.
- The derivation `schedule(&BTreeMap<SystemId, Access>) -> Vec<Vec<SystemId>>`: the layered assignment
  already specified (walk systems in `SystemId` order; place each in the earliest batch with no
  conflict against a member and after every lower-id system it conflicts with). A pure function of the
  sorted declarations, so it is deterministic and observer-independent (Principles 3, 10).
- A serial executor `run_serial(&schedule, &mut world)` that flattens the batches in order and runs
  each system, so the schedule is exercised and proven against the current serial tick before any
  parallelism. A Rayon executor is the same signature with a `par_iter` inside each batch, and is not
  built in this pass.

The proof obligations are determinism-harness checks: the derived schedule is a pure function of the
declarations (permuting declaration order yields the identical layering); the flattened serial order
reproduces the current tick's canonical state hash bit for bit (the scheduler changes how the tick is
expressed, not what it computes); and no conflict ever places two writers of one resource in the same
batch. The existing worker-count sweep then extends to the batch executor when Rayon is switched on.

## The owner's question: hecs now, Rayon now, or hold off?

Hold off on both for this pass, and build the scheduler and the hardening as the deterministic
substrate that makes each safe to adopt later. The reasoning:

hecs is a storage decision, and the scheduler does not need it. The scheduler reasons over declared
resources, and a resource maps equally to a `BTreeMap` store today or a hecs component column later,
so the scheduler is storage-agnostic by construction. Adopting hecs now would be a broad migration
of every per-being store into archetypes, orthogonal to the scheduler's determinism logic, and it is
a decision the owner already deferred ("stay StableId-keyed for the first integrated slice, decide at
scale when the full tick is profiled"). hecs also brings its own canonical-walk care, since a
`hecs::Entity` is a generational index unstable across promotion and demotion (the design already
pins `StableId` as the canonical id for exactly this reason), so an archetype iteration would itself
have to route through the canonical-walk helper. The clean order is: build the scheduler storage-
agnostic now, keep the StableId-keyed stores, and let a hecs migration later change only what a
resource maps to, not the scheduler.

Rayon is a performance decision, and turning it on now would be premature and unsafe. The red-team is
explicit: "the moment any phase is parallelised for performance, R-CMD-ORDER, R-REDUCE-ORDER, and the
full R-CANON-WALK become live blockers at once." The purpose of this pass is to build those guardrails
first, so that parallelism is safe rather than a hazard. There is also no measurement yet that the
serial tick is the bottleneck, so parallelising now would be optimisation without a profile. The
scheduler's output, a sequence of conflict-free parallel batches, is exactly what a Rayon executor
would consume, so the scheduler is Rayon-ready by design without Rayon being switched on; the switch
is thrown later, when the guardrails are proven and a profile shows the serial tick is the limit.

The synthesis is the move Part 57 prescribes: lock the foundation early, permit the answer, force
neither. This pass builds a storage-agnostic, serial-safe deterministic scheduling substrate designed
to be hecs- and Rayon-ready, and adopts neither. hecs stays the owner's storage call at scale; Rayon
stays a switch to throw when profiling demands it; both are unblocked, not required, by this pass.

## What to build now, and what waits

The substrate that used to be the "build now" list is now built and merged: the `canonical_sorted`
and `canonical_reduce` primitives, the R-CANON-WALK enforcement (clippy deny plus the source-scan
backstop and the ordered `EventLog` accessor), the R-CMD-ORDER duplicate-key guard, and the two
order-independent reduce sites. So the immediate next build, on this pass's sign-off, is the scheduler
core itself: the one `schedule.rs` module above (`ResourceId`, `SystemId`, `Access`, the layered
`schedule`, and the serial executor), still over the serial tick, with the phase methods expressed as
declared-access systems so the schedule is exercised and proven against the current tick without yet
parallelising. R-CMD-ORDER's command key is specified and its barrier built; it is wired to the
scheduler's write batches when the parallel command stage lands. Adopting hecs and switching on Rayon
stay separate later decisions this substrate unblocks.

## Reserved values and honest limits

The scheduling substrate carries no calibration values: it is structural, a function of declared
access and stable ids, not of tuned numbers, so there is nothing here to reserve. The honest limits:
the scheduler proves conflict-freedom and determinism of the schedule, not that any particular system
declared its access correctly, so a mis-declared read or write is a correctness bug the declaration
review must catch, which is why the declarations are data a reviewer reads rather than borrow
signatures inferred by a compiler. The agent execution model (poll-every-tick versus event-driven
wakeup, the second Part 57 foundation) is deliberately out of scope here and stays its own pass, since
it couples to temporal level of detail (Part 32) and significance-driven fidelity (Part 54) rather
than to the ordering guarantees this cluster settles.
