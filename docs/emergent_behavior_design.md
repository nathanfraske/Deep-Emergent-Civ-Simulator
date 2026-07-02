# R-BEHAVIOR-EVOLVE: a design pass on evolved behaviour

This is a design proposal, not consolidated design. It is written for the owner to weigh and sign
off (or redirect) before any engine code is written, per the project's resolve-a-research-item
workflow. It follows an owner catch: the utility-AI decision layer (Part 8) selects from an action
repertoire that is authored, even though it is data. A fixed repertoire of behaviours chosen from
outside the simulation is steering at the level of behaviour, which Principle 9 forbids for cultural
and behavioural outcomes. The question this pass answers is how behaviour can arise the way anatomy
already does: from physics and selection, with nothing about the behaviour itself authored.

The owner's standing directions taken as given: physics may be authored, behaviour may not; the
result must be deterministic and observer-independent (Principles 3, 10); the mechanism is fixed
Rust and its variable content is data that grows and evolves with the world (Principle 11); no value
is fabricated, each is surfaced as reserved with its basis. The owner leans toward a small neural
network for the controller but has deferred the final representation to this pass.

## The seam, stated exactly

Part 8 (design.md line 785) makes the action and consideration sets "data-defined per race (Parts
20, 40), not in code, so a world or a race can have actions the engine's authors never enumerated."
That satisfies Principle 11 (no closed enum in the path of world content) but not the deeper form of
Principle 9. Data-defined is not the same as emergent. Whoever writes a race's action list has
authored its behavioural repertoire and, through the considerations and curves, its policy: this
race seeks water when a thirst drive is high because someone wrote that action and that curve. The
locomotion slice built this session inherits the same seam: its movement, perception, memory, and
exploration are physics, but the drives and actions it consults (`crate::decision::Behaviour`) are
an authored policy, marked in the code as a placeholder for exactly this reason.

The end state is that no one writes the policy. A being's needs are physical states of its body, its
options are the physical operations its body affords, and the mapping from state to operation is a
heritable controller that evolves under selection, so that the behaviours a lineage has are the ones
its ancestors survived by having.

## The architecture: physics in, behaviour out

Five layers. The first three are authored physics. The fourth is heritable and evolves. The fifth is
selection, which the engine already performs.

### 1. Homeostatic physiology, a consequence of the body

A being carries internal state variables that are not drives anyone declared but the physical
bookkeeping of a living body: an energy reserve depleted by basal metabolism and by movement and
restored by ingesting matter it can digest; a water level lost over time and by exertion and
restored by drinking; bodily integrity reduced by damage; a core temperature that must stay in a
viable band. These follow from the body's build (Part 20) and the biology-and-composition floor
already resolved (R-PHYS-BIO, the edibility relation in `edibility.rs`): what matter yields what
energy, what counts as water, what damages tissue, are physics, not preference. "Thirst" is not a
drive in this model; it is the state of low water. When any of these variables crosses its physical
floor (energy at zero, integrity at zero, temperature outside the survivable band), the being dies.
Death is the only consequence that matters, and it is physical, not a score.

### 2. Affordances, the operations the morphology permits

The being's options are not a menu of named actions but the primitive physical operations its parts
afford. A locomotion organ affords moving (a heading and a speed, the physics the locomotion slice
already computes from morphology and terrain). A mouth and gut afford ingesting the matter on the
current tile (resolved by the edibility floor into an energy or water gain, or a toxin). A body may
afford a few more primitives as its parts warrant (grasp, strike, rest). The set of primitives is
small, fixed, and physical, the verbs any body with those organs can perform; it is not the
behavioural repertoire, which is which primitive is issued when, and that is what evolves. This keeps
the authored surface to the physics of what a body can do, never to what it should do.

### 3. Sensing, the channels the morphology provides

The being perceives its surroundings within the sensory range its sense organs and acuity imply
(Part 9, the perception the locomotion slice already gates on): the local matter and its edibility,
water, terrain passability and cost, other beings and their salient features, and its own internal
state. The percept is a bounded vector of physical readings, the controller's input. A being senses
what its body can sense and no more, which is the same non-omniscience the locomotion slice enforces.

### 4. The controller, heritable and expressed from the genome

The mapping from the percept and the internal state to a choice of affordance primitive and its
parameters (which heading, ingest or not) is a controller whose parameters are heritable. It is a
new channel in the genome (`genome.rs` `Channel`), a sibling of the cognition, build, composition,
and life-history channels, expressed per individual by `GenePool::express` exactly as those are, so a
being's controller is its inheritance the way its size and acuity are. The mechanism that evaluates
the controller is fixed Rust; its parameters are data that vary, inherit, drift, and are selected
(Principle 11), the same split the RNG core and the substrate registries already use.

Representation is the open choice of this pass. Two candidates, with the trade the owner should
weigh:

- A linear reaction norm: the output primitive and its parameters are a fixed-point weighted sum of
  the input vector, one weight matrix as the heritable parameters. It is trivial to keep bit-exact
  in fixed point, cheap to evaluate at world scale, easy to audit, and it evolves smoothly because
  small weight changes make small behaviour changes. Its ceiling is low: it cannot represent
  internal state, memory, or nonlinear or conditional behaviour, so it can express "move up the
  water gradient in proportion to thirst" but not "search in a pattern, then return."

- A small fixed-point recurrent network: a few layers with a small hidden state, fixed-point weights
  as the heritable parameters, a fixed nonlinearity. It can represent stateful, nonlinear, and
  conditional behaviour, which is the space real foraging and predator-prey behaviour lives in. The
  costs are real: fixed-point network evaluation must be pinned so it is bit-identical across
  machines and thread counts (no float in canonical state, Principle 3), it is heavier to evaluate at
  scale (a concern the temporal-LOD trade below addresses), and a larger parameter space is harder to
  evolve and to verify, with a higher risk of degenerate or brittle policies.

The owner leans toward the network for its expressive ceiling. The recommendation of this pass is a
staged choice: begin with the reaction norm as the substrate and the determinism proof, since it
forces the genome-channel, expression, and selection plumbing to be built and audited against a
policy that is trivially reproducible, then graduate the same plumbing to a small fixed-point
recurrent network once the plumbing is proven, keeping the network topology fixed Rust and its
weights the heritable data. The reserved list carries the network's size as a value to set, so the
step from one to the other is a parameter change, not a rewrite. This is a recommendation, not a
decision; the owner may direct the network from the start.

### 5. Selection, which the epoch already performs

The pre-dawn radiation (`epoch.rs`) already selects: each generation applies a selection kernel to
every pool's allele frequencies and drifts them (Wright-Fisher), and forks founders. Evolved
behaviour needs one thing added: the selection coefficient on the controller's alleles must be a
consequence of homeostatic survival, so a lineage whose controller keeps its bodies alive and
reproducing in their environment fixes its adaptive controller alleles, and one whose controller
lets its bodies starve or stray from water is selected against. The fitness is survival and
reproduction, which fall out of the physical floors of layer 1; it is not an authored objective and,
in particular, is not behavioural resemblance. A controller that keeps a body viable by some strategy
no one anticipated scores well; one that approaches water but mismanages its energy dies. Measuring
survival, not measuring "did it do the expected thing," is what keeps the behaviour emergent.

## The hard tension: scoring behaviour over deep time

The crux, and the main thing for the owner to weigh, is that behaviour is an individual-level,
time-extended property, while the deep-time epoch runs pools as allele frequencies, not individuals
living out lives. Scoring a controller means finding out whether it keeps a body alive, which in the
general case means simulating that body acting in its environment, and doing that for every pool
every generation over deep time is the same cost wall that keeps temporal level of detail (Part 32)
in the research tier. Three ways to pay it, in increasing fidelity and cost:

- A homeostatic-viability proxy. Evaluate the controller statically against a battery of physical
  situations drawn from the pool's environment (a percept with low water and a water gradient to one
  side, a percept with a predator cue, and so on) and score whether the controller's issued
  primitives keep the physical variables away from their floors under the environment's dynamics,
  without simulating a full life. Cheap, per pool per generation, and tractable over deep time. The
  risk is that a proxy is a partial picture of survival and a policy can be viable on the battery yet
  fail in the world, so the battery must be derived from the physics and kept honest, and it can
  never become an authored objective in disguise.

- Sampled behavioural episodes. Each generation, promote a small sample of individuals from the pool
  (the promotion machinery of Part 11 exists), run each for a bounded episode in its environment, and
  set the pool's behavioural selection coefficient from how long they stay viable and whether they
  reproduce. Direct and honest, but costly, and the sample introduces variance that must be handled
  deterministically.

- A hybrid on the significance-and-time gradient, which is the recommendation. Use the cheap proxy
  as the deep-time default for quiet pools, and run full sampled episodes at the dawn, for pools
  under selection pressure that is changing fast, and for any lineage that is observed or promoted.
  This is the same significance-driven allocation the engine already uses for spatial and processing
  detail (Principle 1, Part 54), applied to behavioural evaluation, so it inherits that philosophy
  rather than inventing a new one. It also means the answer to "can a machine afford to evolve
  behaviour over a whole world's deep time" is the same conditional the temporal-LOD work already
  carries, and this pass does not pretend to have solved that; it scopes the proxy as the lever that
  makes it feasible and flags the proxy's honesty as the thing to prove.

## Determinism and the observer

The controller evaluation, its expression from the genome, and its selection must be integer and
fixed-point, with no float entering canonical state, so a world's evolved behaviour reproduces bit
for bit (Principle 3). Every draw the scoring needs (a sampled individual, a battery situation, an
episode's stochastic events) keys through the canonical `DrawKey` schema on the seed, the pool or
being, and the generation or tick, never on the camera, so which behaviours a world evolves is a
function of the seed and the world alone (Principle 10). This is the same discipline the epoch and
the locomotion slice already hold; the network representation is the one piece that needs explicit
care, since fixed-point network evaluation is the new determinism surface.

## How the current code maps on

The authored `Behaviour` (drives, considerations, actions) in `decision.rs` becomes an interface,
not a policy: the drives become read-outs of the homeostatic state of layer 1, the actions become
the affordance primitives of layer 2, and the scoring is replaced by the expressed controller of
layer 4. The utility-AI layer of Part 8 does not disappear; it remains the shape of a fast per-tick
decision, but its weights come from the genome and evolve rather than being written per race. The
locomotion slice's movement, perception, memory, and exploration are the physics substrate the
controller drives, unchanged. This is why the slice was built with the policy fenced off behind a
placeholder: the substrate is reusable, and only the policy is replaced.

## The intelligence dial

A creature's cognition is a dial (Part 20, the intelligence field): a mindless species lives as pure
pool statistics, a plain animal carries a dispositional layer, a great beast carries goals and, when
promoted, a belief store. The evolved controller sits at the low end of this dial as the reflex-and-
drive policy of a non-sentient creature. It does not replace the sentient layers above it (the
axioms of Part 28, the values of Part 21, the theory of mind of Part 37); it underlies them, the
evolved substrate on which, for a sentient race, the deliberative and cultural layers are built. So
this pass is about how animals and the pre-sentient substrate behave, and about what sentient
behaviour rests on, not about replacing sentient deliberation with a network.

## Reserved values, surfaced with bases (none set here)

- Controller representation and size: the reaction-norm-then-network staging above, and, for the
  network, its layer sizes and hidden-state width. Basis: the smallest network that can represent
  the conditional foraging and predator-prey behaviours the ecology needs, against the per-tick and
  per-generation evaluation budget, a performance-and-expressiveness bound to be found by trial.

- The controller-allele mutation rate and selection strength: how fast controller alleles mutate and
  how hard homeostatic fitness pushes them. Basis: the mutation and selection scales the epoch
  already uses for the other channels (`epoch.rs` `EpochParams`), set in the same range so behaviour
  evolves on the same clock as the rest of the genome, adjusted for the larger parameter space.

- The homeostatic rates: basal metabolic energy draw, water loss rate, the damage-to-integrity and
  temperature dynamics, and the death floors. Basis: the metabolic and thermal models of Part 20 and
  the biology floor (R-PHYS-BIO), scaled to the base tick the owner set (one in-world second), so a
  being's energy and water deplete on a realistic timescale.

- The behavioural-scoring policy: the proxy-versus-episode mix, the battery composition (derived from
  the pool's environment, never an authored target behaviour), the episode length and sample size,
  and the significance thresholds that promote a pool to full episodes. Basis: the significance-and-
  time allocation of Part 54 and the temporal-LOD problem of Part 32, a performance bound, with the
  battery's honesty (that proxy-viable implies world-viable) the thing to validate rather than a
  number to set.

## Honest limits and what remains to prove

The open question this pass does not answer, and cannot on paper, is whether adaptive behaviour
actually evolves in feasible compute over a world's deep time. That is an empirical result to be
shown in the engine: that a controller lineage under homeostatic selection comes to approach water
when dry, food when hungry, and away from damage, from a random start, without those behaviours being
authored, and that it does so cheaply enough at the deep-time tier through the proxy. The proxy's
honesty (that scoring well on the battery predicts surviving in the world) is the central risk and
must be validated by cross-checking proxy-selected lineages against full episodes. The network's
fixed-point determinism is a build risk to pin before it is trusted. And the coupling to temporal
level of detail (Part 32) and to the tier-consistency work (R-TIER-CONSIST, Part 54) is real: the
behavioural-scoring proxy is a temporal-LOD device and must stay consistent with the full-episode
tier the way the other coarse-and-fine pairs must. This pass scopes the mechanism and names these as
the things to prove; it does not claim them proven.

## What would be built, once signed off

In order, each determinism-audited and tested before the next: the homeostatic physiology and the
affordance primitives (layers 1 and 2, the physical substrate, buildable now against the edibility
floor); the controller genome channel and its expression (layer 4, starting as the reaction norm);
the homeostatic-survival selection coefficient wired into the epoch (layer 5), with the proxy as the
deep-time scorer and a small proof that behaviour shifts under selection; then the graduation of the
controller to the fixed-point network and the full-episode tier at the dawn. The authored
`Behaviour` is retired as each layer replaces its role.
