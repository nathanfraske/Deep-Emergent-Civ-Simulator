# R-DEEPTECH-COMPOSE: Hierarchical and Recursive Composition of Technology

## TL;DR
- **Encode every technology as a typed, content-addressed composition node** — a record of `{intent_tag, [child component-refs by stable hash-id], material(s), joining technique, exposed interface-port vector}`, all enum-and-integer, where a child ref is either a leaf form-primitive (the existing Part-41 representation) or the stable id of another previously-validated node. This is a generative/recursive grammar in the sense of Hornby's GENRE, not a tech tree: there are no authored levels, only a `uses` relation that bottoms out at leaf primitives and physics.
- **Promote a module to a reusable primitive by a compression-and-reuse criterion borrowed from DreamCoder's MDL/Bayesian library learning and Tomasello's cultural ratchet**: a validated module becomes a referenceable component only after it has (a) passed physics/use validation, (b) stabilised in transmission (survived drift/loss across a reserved number of transmission events), and (c) earned its keep by shortening the description of many downstream intents (reuse count over threshold). This bounds combinatorial explosion by shrinking the effective alphabet the next level searches over — encapsulation in the Maynard-Smith/Szathmáry and Parnas senses.
- **Compose evaluation as an interval-bounded, interface-gated aggregation of cheap closed-form integer proxies**, computed bottom-up over the composition DAG and memoised on the stable id, with explicit emergent system-level proxy terms (resonance, thermal/mass budgets, control loops) that part-wise scores cannot capture, and an interface-compatibility penalty. Sub-scores aggregate by typed combinators (series/parallel/limiting-factor), not naive summation. All Q32.32 fixed-point, counter-RNG, memoised so deep-time (~10,000-year) aggregate running re-uses stabilised modules instead of re-searching.

---

## Problem statement and why it is the crux of open-ended technological depth

The simulator already encodes a single artifact as a composition of form primitives + a material + a joining technique, as enums and integers, with a symbolic "conceived intent" front end. That is a *one-level* representation. Open-ended technological depth — a rocket that is engine + guidance + structure + fuel, each itself a multi-part artifact, nested arbitrarily deep — requires that a validated artifact or technique *become a building block* that an agent can reference as a component inside a larger conceived intent, recursively.

This is the structural crux of the whole deep-tech subsystem. Depth (R-DEEPTECH-DEPTH) is not a separate mechanism; it is what *accumulates* when composition is recursive and a child cannot be built until its sub-children exist and are stable. Scale (R-DEEPTECH-SCALE) is tractable only if stabilised modules can be carried as compact references rather than re-derived. The reach of the system (R-DEEPTECH-PHYSICS) is bounded by whether the physics substrate can score emergent, system-level interactions. So COMPOSE is the spine: get the recursive representation, the promotion criterion, and the composed evaluation right, and depth/scale/reach follow; get them wrong and you either (a) collapse into an authored tech tree, violating the project's emergence principle, or (b) suffer combinatorial explosion that makes search and deep-time running impossible.

The decisive design tension is between **emergence** (order must arise from physical/logical necessity, never be templated) and **tractability** (search and 10,000-year aggregate running must stay cheap, deterministic, integer). The literatures that resolve this tension are remarkably convergent: generative/compositional encodings in artificial life (Hornby, Stanley), the biology of encapsulation and exaptation (Maynard Smith & Szathmáry, Gould & Vrba), engineering modularity theory (Simon, Parnas, Baldwin & Clark, Ulrich, Suh), and automatic library learning in program synthesis (Ellis et al.'s DreamCoder). Each independently says the same thing: **complexity scales when validated sub-structures are encapsulated behind interfaces and reused as units, and the criterion for forming such a unit is that reuse compresses the description of everything built on top of it.**

---

## Deliverable 1: The recursive representation

### Requirements restated
Encode an artifact/technique so a validated one becomes a reusable primitive referenceable as a component, recursively, nested arbitrarily deep. Must stay integer-and-enum, deterministic. Must define: how a component reference works, how interfaces/parameters between levels are expressed, how leaves bottom out, and how to avoid being an authored tech tree in disguise.

### Candidate encodings and tradeoffs

**Candidate A — Flat parametric list (status quo extended).** Keep the Part-41 single record and just allow longer lists of form primitives. *Rejected.* It has no notion of a sub-assembly as a unit; it cannot reference a validated module by identity; it forces the search to re-discover internal structure every time (the very failure Hornby identifies for non-generative/direct encodings, which "use elements of encoded artifacts at most once in translation"). It does not scale.

**Candidate B — Generative grammar / L-system (Hornby GENRE; Lindenmayer; shape/graph grammars).** Encode designs as a grammar whose production rules can be labelled and reused, exactly Hornby's three properties: *combination* (build complex expressions from simpler ones down to atomic primitives), *control-flow* (iteration/repetition), and *abstraction* — defined verbatim by Hornby & Pollack (2002) as "the ability to label compound elements (to manipulate them as units) and the ability to pass parameters to procedures." Hornby's empirical result is the load-bearing justification: on table and robot design, generative representations that reuse labelled building blocks produced (Hornby 2004, verbatim) "higher fitness and a more regular structure," and "scale with design complexity because of their ability to hierarchically create assemblies of modules for reuse." *Strong fit*, but a full rewriting grammar with parameter passing risks Turing-complete genotypes whose evaluation is unbounded — dangerous for determinism and a fixed per-tick budget.

**Candidate C — Compositional DAG of typed, content-addressed nodes (CPPN-/program-synthesis-flavoured).** Encode each technology as a node that references children by *stable id*; the whole population of technologies is a directed acyclic graph (the `uses` relation). This is the DreamCoder library model (a growing library of reusable components referenced by later programs) and Stanley's CPPN insight that a compositional function-graph can produce regular, hierarchical structure. Content-addressing (the id is a hash of the node's canonical integer contents) gives determinism, deduplication, and memoisation for free.

**Recommendation: Candidate C as the core, with Candidate B's abstraction discipline as the rule for *how nodes are formed*.** Concretely:

A technology is a **CompositionNode**, an integer/enum record:

- `id: u64` — content address = counter-based hash (master seed ⊕ canonical serialization of the fields below). Deterministic, stable, collision-checked.
- `intent_tag: enum` — the conceived-intent symbol (front-end only; the authoritative core treats it as an opaque enum label, never as a recipe key).
- `kind: enum { Leaf, Composite }`.
- For `Leaf`: the existing Part-41 payload — `form_primitives: SmallVec<FormEnum>`, `material: MaterialId`, `joining: JoinEnum`. Leaves bottom out here: the recursion terminates at form primitives + material + join, which are themselves *data* (the project's data-driven default), grounded directly in the physics substrate.
- For `Composite`: `children: SmallVec<ComponentRef>`, plus `material`/`joining` for the *assembly-level* bonds (how the sub-modules are fastened together), plus the assembly-level interface.
- `ports: PortVector` — the exposed interface (see below). This is the *only* thing higher levels see; internals are hidden (Parnas information hiding / abstraction barrier).
- `param: SmallVec<Q32_32>` — bound parameters at this composition (e.g., a scaling factor, a count for iterated children), enabling Hornby-style "families of designs" from one node by parameter variation.

A **ComponentRef** is `{ target_id: u64, transform: enum/int, param_overrides: SmallVec<Q32_32> }`. Reference is **by stable id, not by type** — this is the key decision. By-id reference means a composite depends on a *specific validated module* (giving genuine prerequisite depth: you cannot reference what does not yet exist and is not yet stable), while the `intent_tag` + `ports` provide a *type-like* signature that lets agents search for "some module that satisfies this port contract." So the system gets both: identity for dependency/determinism, and a structural type for substitutability (Baldwin & Clark's "design rules"/visible interface vs. hidden module; Ulrich's interface specification).

### Interfaces and parameters between levels
Interfaces are the linchpin against combinatorial explosion. Following Ulrich's (1995) definition of product architecture — "(1) the arrangement of functional elements; (2) the mapping from functional elements to physical components; and (3) the specification of the interfaces among interacting physical components" — and Suh's independence axiom (good designs keep functional requirements independent so one parameter affects one requirement), each node exposes a **PortVector**: a fixed-width integer/enum descriptor of what the module offers and demands at its boundary — e.g. `{mechanical_mount: enum, load_rating: Q32_32, energy_in: (form enum, rate Q32_32), energy_out: (...), signal_in/out: enum, thermal_flux: Q32_32, mass: Q32_32, envelope: (x,y,z Q32_32)}`. Composition succeeds only where ports are compatible; incompatibility is not a hard reject but a scored penalty (see Deliverable 3), modelling adapters, losses, and bodging. Parameters pass *down* via `param_overrides` and aggregate constraints pass *up* via the PortVector — a bounded, fixed-width, integer contract at every level. Because ports are fixed-width, the interface between any two levels costs O(1) to check.

### How it stays integer/enum/deterministic
Every field is an enum or Q32.32 integer. `id` is a counter-based hash (hash of master seed, the node's serialized bytes, and a phase tag) — no floats, no pointer identity, reproducible across machines. The DAG is stored as an arena of nodes indexed by id; recursion is bounded by a reserved maximum depth and a reserved maximum node count per evaluation (RESERVED — see calibrations). Search over compositions is fixed-point and counter-RNG: candidate children and parameters are drawn by hashing (seed, agent id, intent id, attempt counter).

### How it avoids being an authored tech tree in disguise
This is the hard project principle, so make the avoidance mechanical, not aspirational:

1. **No node is authored.** The only authored data are *leaf* form primitives, materials, joins, the port axes, and the proxy formulas. Every Composite node is *discovered* by agents at runtime by search + physics validation. There is no table of "to build X you need Y and Z."
2. **The `uses` edges are emergent, not declared.** A prerequisite exists iff a discovered composite happens to reference a child; nobody wrote "rocket requires engine." If the physics makes a one-shot monolithic solution viable, the system will find it flat; depth appears *only* where decomposition is physically/logically favoured (Simon's near-decomposability: stable intermediate forms assemble faster, so they dominate — in Simon's 1962 formulation, "complex systems will evolve from simple systems much more rapidly if there are stable intermediate forms than if there are not").
3. **Order emerges from necessity via the evaluator.** Because a composite's score depends on its children actually existing-and-stable and on ports matching real physics, the *only* compositions that survive are those the modelled physics rewards. The tech "tree" is a read-out of physics + reuse history, never an input.

This is exactly the boundary that separates an emergent system from an authored automation game. In Factorio, the dependency graph is hand-written, node by node and quantity by quantity, in Wube's own data files: in `base/prototypes/recipe.lua`, advanced-circuit is declared as `ingredients = {electronic-circuit ×2, plastic-bar ×2, copper-cable ×4} → results = {advanced-circuit ×1}` — a designer-chosen edge *and* a designer-chosen ratio. The leaf recipes are equally authored: electronic-circuit is "Craft time: 0.5 seconds; Ingredients: 3× copper cable, 1× iron plate," with an authored throughput ratio of "2 electronic circuit assemblers to 3 copper cable assemblers" (so "a single electronic circuit effectively requires one and a half units of copper ore"). The whole vanilla graph is finite and curated: per Kirk McDonald's "Calculating Factorio" analysis, "the complete recipe graph in vanilla Factorio (0.15.2) has 204 items and 215 recipes" — every node and edge authored by hand. That tiered-intermediate, recipes-as-nodes, dependency-DAG *shape* is precisely what we want to *emerge*; under R-DEEPTECH-COMPOSE no such edge or quantity is ever written down. The DAG shape is the *output* of physics + reuse, not an input.

---

## Deliverable 2: The promotion criterion and encapsulation

### Requirements restated
When does a conceived-and-physics-validated module become a "part" available to the next level? Tie to the cultural ratchet and to encapsulation. Solve the encapsulation obstacle: new levels build on stabilised lower ones without combinatorial explosion.

### Why a promotion *gate* is needed at all
If every validated artifact instantly became a referenceable primitive, the alphabet over which the next level searches would grow without bound and the branching factor of search would explode — the combinatorial-explosion obstacle named in the brief. The biology and the program-synthesis literatures give the same answer: you do not promote everything; you promote *units that compress the future*. Maynard Smith & Szathmáry's major transitions are precisely events where "entities that were capable of independent replication before the transition can replicate only as part of a larger whole after it" — encapsulation produces a new higher-level unit *and suppresses lower-level competition*, i.e. it removes degrees of freedom from the search. Parnas's information hiding is the engineering twin: a module hides a design decision behind an interface so the rest of the system need not search over it.

### Candidate criteria and tradeoffs

**Candidate 1 — Validation-only ("if it works, it's a part").** Promote any artifact that passes physics/use validation. *Rejected:* explodes the alphabet; promotes one-offs and noise; no notion of stability or reuse.

**Candidate 2 — Usage-frequency only ("promote the popular").** Promote whatever gets referenced often. *Partial:* captures reuse but ignores stability and compression; rewards fads; can promote redundant near-duplicates.

**Candidate 3 — Compression/MDL (DreamCoder).** Promote the sub-structure that maximally *compresses* the corpus: add to the library the component that minimises the total description length of (library + all solutions), evaluated over refactorings. In Ellis et al. (2021), DreamCoder's abstraction-sleep phase updates the library "by refactoring programs found during waking and abstracting out common components," and "a new refactoring algorithm based on E-graph matching identifies common sub-components across synthesized programs, building a progressively deepening library of abstractions." The criterion is intrinsically *multi-task*: the description-length objective sums over the whole task corpus, so a component is abstracted only if reusing it shortens *many* solutions — and adding such a component "crucially bootstraps solutions to many harder problems in later wake cycles." *Strong fit* for tractability and for emergent depth, because it formally prefers exactly the modules that make the next level cheaper to search.

**Candidate 4 — Cultural-ratchet stability (Tomasello).** Promote only what has *stabilised in transmission*: a design that has spread and survived drift and loss through the same transmission machinery as belief and knowledge. Tennie, Call & Tomasello (2009) require "faithful social transmission that can work as a ratchet to prevent slippage backward — so that the newly invented artifact ... preserves its new and improved form ... until a further modification or improvement comes along," and stress that "for many animal species it is not the creative component, but rather the stabilizing ratchet component, that is the difficult feat." *Strong fit* for the project's existing cultural-ratchet model and for emergence (promotion is earned socially, not declared).

### Recommendation: a three-gate promotion criterion (validation ∧ stability ∧ compressive reuse)

A validated module is promoted to a referenceable primitive — added to the "library" that agents can reference by id — only when **all three** hold:

1. **Physics/use validation gate.** The module passes the composed evaluation (Deliverable 3) above a reserved viability threshold. (Necessary, never sufficient.)
2. **Stability gate (cultural ratchet).** The design has been transmitted and *re-converged* — i.e. it has survived a reserved number of transmission events / persisted over a reserved span without being lost or drifting beyond a reserved similarity radius. This is encapsulation-as-stabilisation: only a design stable enough to be copied faithfully can safely become a hidden module others build on. It directly imports Tomasello's "stabilising ratchet" as the gate.
3. **Compressive-reuse gate (MDL).** The module is referenced by enough distinct higher intents that promoting it *reduces* the aggregate description length of the technology corpus — i.e. carrying it as one primitive is cheaper than re-expanding it everywhere. This is DreamCoder's criterion, made the explicit anti-explosion control.

The three gates map cleanly onto the three literatures and onto the project's invariants: validation = physics substrate; stability = cultural ratchet; compressive reuse = encapsulation/abstraction. The *conjunction* is what prevents explosion: validation alone over-promotes; the stability and compression gates ensure the alphabet grows only by units that are both durable and economical.

### How this controls combinatorial explosion
Promotion *shrinks* the effective search space of the next level rather than enlarging it. Once module M is a primitive, an agent conceiving a larger intent searches over `{leaf primitives} ∪ {promoted modules}` but treats M as a single token whose internals are hidden (Parnas) — so the branching factor at the new level is set by the count of *promoted* modules, not by the exponentially larger count of all validated artifacts or all leaf-combinations. This is exactly Simon's near-decomposability buying tractability: the time to achieve complexity depends on the number of potential intermediates, and stable intermediates are the ones that survive to be built upon. Baldwin & Clark (2000) quantify the upside as real-option value: modularisation lets designers "replace early, inferior solutions with later, superior solutions" inside a hidden module without disturbing the rest — so improvement at one level does not re-open search at every other level.

### Exaptation: promoted modules can be repurposed
Because a promoted module is referenced by `id` + matched by `ports`/`intent_tag`, a module promoted for one intent can be referenced inside an unrelated intent whenever its ports fit — Gould & Vrba's (1982) exaptation, "features that now enhance fitness but were not built by natural selection for their current role" (their canonical example: feathers, plausibly first for thermoregulation, later co-opted for flight). This is a feature to *enable*, not suppress: it is a major source of emergent, non-templated cross-domain depth. Mechanically it falls out for free from id-reference + port-matching; no special case needed.

### Interaction with Inconsistency 5 (flag only, do not resolve)
The promotion criterion is where "a module becomes a reusable primitive" meets "how techniques originate." The document's two parts disagree on technique-origination (Inconsistency 5). Flag: **the stability gate's transmission machinery presupposes a definite answer to where a technique first comes from** — whether a *technique* (the `joining`/process enum) originates by the same conceive-validate-promote loop as artifacts, or by a different origination path. If techniques originate differently from artifacts, gate 2 (stability via transmission) may need a technique-specific variant. This is left unresolved per instruction; the composition mechanism is compatible with either resolution because it treats `joining` as an enum reference exactly like a child reference — but the *promotion* of a new technique to that enum space is the precise point of contact with Inconsistency 5.

---

## Deliverable 3: The composed-evaluation method

### Requirements restated
Make the physics/use evaluation compose across levels or be approximated cheaply at the system level. Address: do sub-scores aggregate (and how, under nonlinearity)? Are there emergent system-level properties absent from parts (so part-wise scores can't sum)? Keep it cheap, deterministic, fixed-point integer, inside a search loop. How do interface incompatibilities affect the composed score?

### The core problem: composition is not summation
Naive aggregation (sum or average of child scores) is wrong because of two facts the literature insists on. First, Simon's near-decomposability is only *near*: the cross-cutting connections between subsystems are nonlinear and their effects are hard to predict in general. Second, systems have emergent properties no part possesses — a rocket's structural resonance, an engine-plus-airframe's thermal balance, a control loop's stability — what the major-transitions literature frames as the whole behaving as a new unit. So composed evaluation must (a) aggregate sub-scores through *typed, nonlinear combinators*, and (b) add explicit *system-level emergent proxy terms*, and (c) charge for *interface mismatch*.

### Recommendation: interval-bounded, interface-gated, memoised bottom-up aggregation

Evaluate a CompositionNode by a deterministic post-order traversal of its DAG, computing for each node a fixed-width **EvalVector** of Q32.32 quantities (mass, stiffness, load capacity, energy in/out, thermal flux, failure pressure, control margin, cost/effort, …) plus a scalar viability score. Three mechanisms:

**1. Typed combinators instead of summation.** How children's EvalVectors combine depends on the *interface topology*, using closed-form integer rules drawn from the relevant physics, e.g.:
- **Series / load-path:** limiting-factor (min) combinators — a structure is as strong as its weakest load-bearing member; contact pressure = force / min-area (the existing base-case proxy generalised).
- **Parallel / redundant:** additive or saturating-additive (capacities add; reliability via complement-of-product).
- **Resource budgets:** conservation sums that must satisfy a constraint (Σ mass ≤ envelope rating; Σ energy draw ≤ supply), where violation drives the score down.
- **Conversion chains:** multiplicative efficiencies (η_total = Πη_i in fixed-point), so loss compounds — naturally yields diminishing returns and depth limits without authoring them.

These are exactly the limiting-factor / series-parallel rules of engineering reliability and Suh's design-matrix coupling: an *uncoupled* composition (diagonal design matrix) aggregates cleanly; a *coupled* one incurs interaction penalties.

**2. Explicit emergent system-level proxy terms.** Add a small, fixed set of cheap *whole-system* proxies computed from the aggregated EvalVector, not from any single child — e.g. a resonance proxy (compare aggregate stiffness/mass ratio against excitation), a thermal-balance proxy (aggregate generation vs. dissipation), a control-stability proxy (loop gain/margin from guidance + structure together). Each is a closed-form integer formula over already-aggregated quantities, so it is O(1) per node. This is where composed evaluation *depends on the physics substrate* (R-DEEPTECH-PHYSICS): the reach of the tech system equals the reach of these system-level proxies. Where the substrate cannot model an emergent interaction, that capability ceiling is real and should be surfaced, not faked.

**3. Interface compatibility as a scored penalty.** For each ComponentRef, compare child `ports` against the parent's expected contract. Exact match = no penalty; mismatch within an adaptable range = a reserved penalty (modelling adapters, losses, added mass); mismatch outside range = the composition is non-viable (score floored). This makes incompatibility *graded*, supporting bodging and adapters as emergent intermediate technologies, and it is the mechanism by which "interfaces/incompatibilities between components affect the composed score."

### Handling nonlinearity honestly: interval bounds
Because near-decomposability is only approximate, carry each EvalVector quantity as a **[lo, hi] interval** in Q32.32 rather than a point estimate. Combinators propagate intervals (cheap integer min/max/add/mul). A wide interval flags a composition whose cross-couplings the proxies cannot pin down; the viability gate can require the *lower* bound to clear threshold (conservative) — so the system neither pretends to FEA precision nor silently trusts a fragile sum. This is the cheap analogue of "compose the sub-evaluations or approximate at the system level": you compose bounds, and the width tells you how much you are approximating.

### How it stays cheap, deterministic, integer, and inside the search loop
- **Memoisation on stable id.** A node's EvalVector is a pure function of its (content-addressed) id, so it is computed once and cached. Re-evaluating a higher system that references M reuses M's cached vector — evaluation cost is O(new nodes), not O(total tree). This is the single most important performance lever and it is *why* promotion/encapsulation pays off computationally.
- **Fixed-width vectors, closed-form proxies.** Every combinator is a handful of integer ops; no iterative solvers, no floats. Per-node cost is O(1) in the number of EvalVector channels.
- **Counter-RNG only where stochastic.** Any Monte-Carlo-flavoured proxy (e.g. reliability sampling) is replaced by closed-form expectation or by counter-based hashing, never `rand()`.
- **Bounded recursion.** Reserved max depth/node-count per evaluation guarantees a hard per-tick ceiling.

---

## How the recommendation yields emergent prerequisite depth (R-DEEPTECH-DEPTH) without an authored tech tree

Depth is an *emergent read-out* of three rules already specified, with nothing authored:

1. **By-id reference makes prerequisites real.** A composite literally cannot be conceived-and-validated until the children it references exist *and* (to be referenced as primitives) have passed the three promotion gates. The `uses` DAG *is* the prerequisite graph, and it is generated, never declared.
2. **Promotion gating makes depth accumulate monotonically (the ratchet, structurally).** Each promoted module is a stabilised rung; the next level reaches only as high as the rungs below it are stable. This is the cultural ratchet applied to structure — Tomasello's "ratchet to prevent slippage backward" instantiated as: you can only build on what has already stabilised.
3. **Physics decides where depth is favoured.** Because composed evaluation rewards decomposition only where near-decomposable structure actually scores better (series/parallel/budget/efficiency combinators + emergent proxies), depth appears in domains the physics makes decomposable and stays shallow where a monolith wins. Simon's watchmaker parable is the exact dynamic: stable sub-assemblies accumulate faster, so they come to dominate — depth is *selected for*, not scripted.

The contrast with an authored tech tree is sharp and concrete. In Factorio the designer literally wrote the edge and the quantity — `advanced-circuit ← 2× electronic-circuit + 2× plastic-bar + 4× copper-cable` — into a data file, one of 215 hand-authored recipes over 204 items. Here, nobody writes that edge or that ratio; if such a composition is physically rewarded and reused-and-stabilised, the edge appears, and if not, it doesn't. The Factorio DAG (tiered intermediates, recipes-as-nodes, dependency DAG) is the *shape we want to emerge*, explicitly not to author.

---

## Implications for deep-time aggregate scale (R-DEEPTECH-SCALE) and physics dependence (R-DEEPTECH-PHYSICS)

**Scale.** The ~10,000-year aggregate tier is feasible precisely because of encapsulation + memoisation. A culture's technological state at the aggregate tier is a *compact set of promoted-module ids* (a library), not a re-derivable search. Advancing the aggregate tier means occasionally attempting new compositions over the existing library (cheap, because children are cached and ports are O(1)), promoting the few that pass the gates, and letting the ratchet/transmission model drift or lose modules. Because promotion is the MDL-compressive subset, the library stays small relative to the space of all validated artifacts — the alphabet the aggregate tier carries is bounded by the compression gate. This is the direct pay-off of DreamCoder-style library growth: later problems are solved by reference, not re-search.

**Physics dependence.** Composed evaluation's emergent-proxy terms are the exact place the system's reach is bounded by the substrate. Series/parallel/budget aggregation needs only the base-case physics already present; but the *emergent* proxies (resonance, thermal balance, control stability) require the substrate to expose the relevant aggregate quantities. The honest design stance: the set of emergent proxies is *data*, and the reach of technology equals the reach of that set. New physics → new proxies → new attainable depth, with no change to the composition mechanism itself.

---

## The failure mode to avoid: the authored-recipe-graph-in-disguise

The seductive failure is to implement composition as a lookup table of `intent → required components` — a Factorio assembly graph wearing an emergence costume. Symptoms to watch for in code review:
- A data file that enumerates which children a given intent "needs," or in what quantities (the Factorio `recipe.lua` pattern: `advanced-circuit ← 2× electronic-circuit + 2× plastic-bar + 4× copper-cable`). If it exists, you have authored the tree.
- Promotion that fires on validation alone with a designer-curated allow-list of which modules "count." (Curation = authoring.)
- Evaluation that returns a designer-assigned score per named technology rather than computing it from physics proxies over the actual children. (Scored authoring.)
- `intent_tag` used as a *recipe key* that selects a fixed decomposition, rather than as an opaque label the search must satisfy via ports + physics.

The design avoids all four by construction: the only authored data are leaves, axes, and proxy formulas (data-driven default); every edge and every node above the leaves is discovered by search + physics + the three-gate promotion; and `intent_tag` is non-authoritative front-end sugar. The test of success is **non-templatedness**: run two cultures from different seeds and the *shape* of their `uses` DAGs should differ (different intermediates, different depth profiles) while both respect the same physics — order emerging from necessity, not from a shared authored skeleton.

---

## Consolidated reserved calibrations (basis given, numbers not invented)

All specific numeric values below are **RESERVED for the owner to tune**. Each is given with its basis; none is fabricated.

1. **Max composition depth per evaluation.** *Basis:* per-tick CPU budget and the observed depth at which marginal proxy improvement falls below sensor noise; set so worst-case post-order traversal fits the frame budget. Bounded by determinism/perf invariant, not by realism.
2. **Max node count per evaluation.** *Basis:* same frame-budget ceiling; interacts with memoisation hit-rate.
3. **Viability threshold (gate 1).** *Basis:* the base-case physics/use proxies' natural failure boundary (e.g. contact pressure exceeding material yield) — read from the material/physics data, not chosen arbitrarily.
4. **Stability span / transmission-event count (gate 2).** *Basis:* the project's existing cultural-transmission model's drift/loss rates — the number of faithful transmissions empirically needed for re-convergence rather than loss (Tomasello's stabilising-ratchet threshold). Should be set equal to whatever the belief/knowledge transmission subsystem already uses, for consistency.
5. **Drift similarity radius (gate 2).** *Basis:* the edit-distance in the composition encoding beyond which two designs count as different modules; tied to the transmission model's mutation operator granularity.
6. **Reuse count / MDL compression threshold (gate 3).** *Basis:* DreamCoder's criterion — promote iff aggregate description length strictly decreases; the integer reuse-count proxy is the cheap surrogate, calibrated so the promoted-library growth rate matches the desired aggregate-tier memory budget over 10,000 years.
7. **Interface-mismatch penalty curve and adaptable range.** *Basis:* the physics of the relevant port axis (e.g. impedance/area/voltage mismatch losses); penalty magnitude read from loss formulas, range from where adapters become physically impossible.
8. **Emergent-proxy weights (resonance, thermal, control).** *Basis:* the physics substrate's units for each aggregate quantity; weights are unit-conversion/criticality factors surfaced from the substrate, not aesthetic tuning.
9. **Interval-width rejection threshold.** *Basis:* acceptable approximation error for the aggregate tier vs. the per-tick tier; how conservative (lower-bound-must-clear) the viability gate should be.
10. **EvalVector channel set.** *Basis:* exactly the quantities the physics substrate can expose; this is data, and it defines the reach of the tech system (R-DEEPTECH-PHYSICS).

---

## Recommendations (staged, with thresholds that would change them)

1. **Build the CompositionNode + ComponentRef arena first**, with content-addressed ids and the existing Part-41 leaf payload as the `Leaf` variant. *Benchmark to advance:* two distinct seeds produce structurally different `uses` DAGs over the same leaf set. If they produce identical DAGs, the representation is templating somewhere — stop and find the authored edge.
2. **Implement memoised bottom-up evaluation with typed combinators and interval bounds** before any promotion logic, because promotion's compression gate *depends on* evaluation being cheap and cached. *Benchmark:* evaluating a depth-N system costs O(new nodes), confirmed by cache-hit instrumentation. If cost scales with total tree size, the memoisation key (id) is wrong.
3. **Add the three-gate promotion criterion last**, wiring gate 2 to the *existing* cultural-transmission subsystem rather than a new one. *Benchmark:* the promoted-library size grows sub-linearly in validated-artifact count; if the library tracks validated-artifact count linearly, the compression gate is not binding — raise the MDL/reuse threshold.
4. **Defer all numeric calibration** to the owner; ship with the ten reserved values as named constants in a config file, each annotated with the basis above. *Threshold to revisit:* if deep-time runs blow the aggregate-tier memory budget, tighten gate 3 (compression) before touching gates 1–2.
5. **Treat the emergent-proxy set as the project's true tech-reach dial.** When you want technology to reach further, add a physics-grounded system-level proxy — never an authored recipe. *Red line:* if a feature request can only be satisfied by writing an `intent → components` table, it is out of scope for COMPOSE and belongs in the physics substrate instead.

---

## Caveats

- **The compression gate is a proxy for an NP-hard ideal.** True MDL over all refactorings (DreamCoder's E-graph search) is expensive; the recommended integer reuse-count surrogate is a deliberate cheapening. It can occasionally promote a near-duplicate or miss a deep shared sub-structure. The interval-bound and drift-similarity-radius machinery mitigate but do not eliminate this.
- **Near-decomposability is an assumption, not a guarantee.** Simon's own framing (and its critics) note cross-cutting nonlinear couplings that interval bounds *flag* but cannot *resolve*. Domains where systems are strongly integral (Ulrich) rather than modular will show wide intervals and may be under-served by composed evaluation — this is a real reach limit, surfaced honestly.
- **Inconsistency 5 is unresolved by design.** The technique-origination disagreement touches gate 2; whichever way the owner resolves it, the stability gate for *techniques* may need a variant. Flagged, not fixed.
- **Some primary quantitative claims are secondhand.** The Hornby 2002 *Artificial Life* full text was paywalled; the verbatim three-property/abstraction definitions are verified from Hornby's contemporaneous and consistent 2003 AAAI paper and 2003 PhD thesis, with the "higher fitness / more regular structure" result verified verbatim from the 2004 *Environment and Planning B* abstract. The DreamCoder "Nature Communications" version could not be confirmed; the verified peer-reviewed venues are PLDI 2021 and *Phil. Trans. R. Soc. A* 2023.
- **Factorio figures are version-specific.** The "204 items / 215 recipes" count is for vanilla 0.15.2 (Kirk McDonald's analysis); later versions and Space Age expand it. The point — that the graph is finite and authored — is version-independent; the exact counts are not.

---

## Academic grounding / references

**Artificial life, generative/compositional encodings, open-endedness.**
- G. S. Hornby & J. B. Pollack (2002), "Creating High-Level Components with a Generative Representation for Body-Brain Evolution," *Artificial Life* 8(3):223–246. — Defines generative representations by genotype reuse; abstraction = "the ability to label compound elements (to manipulate them as units) and the ability to pass parameters to procedures."
- G. S. Hornby (2004), "Functional Scalability through Generative Representations: The Evolution of Table Designs," *Environment and Planning B* 31(4):569–588. — Generative reps give "higher fitness and a more regular structure"; reuse of building blocks scales with complexity.
- G. S. Hornby (2003), *Generative Representations for Evolutionary Design Automation*, PhD thesis, Brandeis University; and "Creating Complex Building Blocks through Generative Representations," AAAI Spring Symposium SS-03-02. — Abstraction = "encapsulate part of the genotype and label it such that it can be used like a procedure."
- G. S. Hornby, H. Lipson & J. B. Pollack (2003), "Generative Representations for the Automated Design of Modular Physical Robots," *IEEE T. Robotics & Automation* 19(4):703–719. — Reusable subprocedures in loops/recursion let the design system scale to more complex tasks in fewer steps than a non-generative representation.
- G. S. Hornby & J. B. Pollack (2001), "Evolving L-systems to Generate Virtual Creatures," *Computers & Graphics* 25(6):1041–1048.
- K. O. Stanley (2007), "Compositional Pattern Producing Networks: A Novel Abstraction of Development," *Genetic Programming and Evolvable Machines* 8(2):131–162.
- K. O. Stanley & J. Lehman (2015), *Why Greatness Cannot Be Planned: The Myth of the Objective*, Springer. — Novelty/stepping-stone view of open-endedness; judge progress by what it spawns.
- J. Clune, J.-B. Mouret & H. Lipson (2013), "The evolutionary origins of modularity," *Proc. R. Soc. B* 280:20122863; and H. Mengistu, J. Huizinga, J.-B. Mouret & J. Clune (2016), "The Evolutionary Origins of Hierarchy," *PLoS Comput. Biol.* 12(6):e1004829. — Connection-cost pressure drives modularity *and* hierarchy ("the recursive composition of sub-modules") and improves evolvability.

**Program synthesis / library learning.**
- K. Ellis, C. Wong, M. Nye, M. Sablé-Meyer, L. Cary, L. Morales, L. Hewitt, A. Solar-Lezama & J. B. Tenenbaum (2021), "DreamCoder: Bootstrapping Inductive Program Synthesis with Wake-Sleep Library Learning," *PLDI 2021*, pp. 835–850 (journal version: *Phil. Trans. R. Soc. A* 381:20220050, 2023). — MDL/Bayesian compression criterion for promoting reusable abstractions; E-graph refactoring; multi-task reuse ("abstracting out common components," which "bootstraps solutions to many harder problems in later wake cycles").

**Biology of major transitions and exaptation.**
- J. Maynard Smith & E. Szathmáry (1995), *The Major Transitions in Evolution*, OUP. — Encapsulation produces new higher-level units; "entities ... capable of independent replication before the transition can replicate only as part of a larger whole after it."
- S. J. Gould & E. S. Vrba (1982), "Exaptation—a Missing Term in the Science of Form," *Paleobiology* 8(1):4–15. — Exaptations are "features that now enhance fitness but were not built by natural selection for their current role."

**Engineering design theory and systems modularity.**
- H. A. Simon (1962), "The Architecture of Complexity," *Proc. American Philosophical Society* 106(6):467–482. — Hierarchy, near-decomposability, the watchmaker parable; "complex systems will evolve from simple systems much more rapidly if there are stable intermediate forms than if there are not."
- D. L. Parnas (1972), "On the Criteria To Be Used in Decomposing Systems into Modules," *CACM* 15(12). — Information hiding; modules hide a design decision behind an interface.
- C. Y. Baldwin & K. B. Clark (2000), *Design Rules, Vol. 1: The Power of Modularity*, MIT Press. — Visible design rules vs. hidden modules; modular operators; option value of substituting hidden modules.
- K. T. Ulrich (1995), "The role of product architecture in the manufacturing firm," *Research Policy* 24:419–440. — Product architecture as arrangement of functional elements, function→component mapping, and interface specification; modular vs. integral.
- N. P. Suh (1990, 2001), *The Principles of Design* / *Axiomatic Design*, OUP. — Independence axiom (keep functional requirements independent), information axiom (minimise information content); design matrices (uncoupled/decoupled/coupled).
- Engineering design grammars: shape grammars (Stiny), graph grammars / graph rewriting for mechanical synthesis (e.g. Königseder & Shea), L-systems (Lindenmayer; Prusinkiewicz & Lindenmayer, *The Algorithmic Beauty of Plants*).

**Cultural evolution / the ratchet.**
- M. Tomasello (1999), *The Cultural Origins of Human Cognition*, Harvard UP; and C. Tennie, J. Call & M. Tomasello (2009), "Ratcheting up the ratchet: on the evolution of cumulative culture," *Phil. Trans. R. Soc. B* 364:2405–2415. — The ratchet effect; the *stabilising* component (faithful transmission) is the hard part, not invention.

**Game/simulation precedent.**
- Factorio (Wube Software): hand-authored recipe/assembly graph — e.g. `base/prototypes/recipe.lua`'s advanced-circuit (`2× electronic-circuit + 2× plastic-bar + 4× copper-cable → 1`); vanilla 0.15.2 has 204 items and 215 recipes (Kirk McDonald, "Calculating Factorio"). With kin such as Satisfactory and Dyson Sphere Program, these tiered-intermediate dependency DAGs are the structural *shape* to reach emergently, explicitly not to author.
