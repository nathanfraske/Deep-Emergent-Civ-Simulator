# Creatures-React Being-Percept Arc: opener and resolved framing (Agent B)

Doc-only opener for the owner-queued **creatures-react** arc: give the biosphere creature (Arc 7, a
genome-expressed controller with no belief store) a way to perceive another agent's emitted signal and come
to move toward or away from it, closing the asymmetry the owner spotted (founders perceive creatures;
creatures do not perceive anyone). No code lands until the gate rules the resolved framing below.

The framing was taken through the full frame-blind discipline before any code, because a creature coming to
move toward or away from a perceived emitter is exactly the emergence-critical coupling the blind panel
guards. The section-11 input-bias smoke test (strongest model, fail-closed) blocked the construction across
five rounds, each on a real seam, and cleared the fifth; the section-10 blind panel (six panelists across
types and models, five returning, one refused on a model content-safeguard error) then critiqued the cleared
statement. The result materially revises the arc's starting premise, which is what the discipline is for.

---

## 1. The asymmetry, grounded in source

A founder is a `Mind` (in `World.minds`) plus a `Walker` (in `Embodiment.walkers`). A creature is a `Walker`
only, its id tagged `CREATURE_ID_TAG` so it is provably absent from the mind registry (`runner.rs:6032`,
assert `:6033`). Every walker EMITS a being-signal derived from its own body state
(`physiology::being_signal_emission` from body temperature, `runner.rs:5297`), creature or founder alike, so
founders already perceive creatures. The perceive-and-learn loop is gated on `world.mind(w.id)` returning
`Some` (`runner.rs:5287`), so a creature, having no mind, is dropped from it: it emits but never perceives,
forms no belief, and computes no direction. The creature's controller already carries the being-directed
input slots (it is expressed against the same embodiment layout, `runner.rs:6039`), fed today only by the
belief-derived gradient; the slots exist, the belief-derived signal is what is missing.

The founder path itself (for reference): `perceive_being_signal` mints a subject id keyed on the emission
channel and the perceiver's own discrimination bucket, never on species, kind, trophic role, or relatedness
(`learn.rs:732`); the perceiver correlates that subject with its own reserve outcome that tick
(`is_harm_tick` / `is_reward_tick`, pure functions of its own homeostasis, `homeostasis.rs:1089,1105`) and
commits a valence from the labelled set {HARMS, BENIGN} or {REWARDS, NEUTRAL} into its belief store; the
avoidance and attraction gradients read that committed valence and yield an inverse-distance direction, fed
to a heritable freely-signed controller weight (founder-zero, expressed unclamped) whose sign selection sets
(`learn.rs:806,822`).

---

## 2. What the frame-blind changed

The arc was queued as "give the creature a lighter being-percept path so it forms a predator/prey belief,
like the founder." The frame-blind found that this presupposes a mechanism the creature's tier is not built
for, and that a cleaner mechanism the principles prescribe was foreclosed. The resolved framing puts three
candidate mechanisms as peers and judges them against the principles:

- **(i)** Give the creature a belief store and run the founder path: correlate the perceived signal with its
  own reserve outcome, commit a valence from the labelled set, read that committed valence to produce the
  direction.
- **(ii)** No within-life change: feed the raw perceived signal (its channel and the creature's own
  discrimination bucket, no valence, no belief) into a controller direction slot through a heritable
  freely-signed weight, the toward/away coupling fixed entirely by selection across generations.
- **(iii)** Within-life plasticity: a correlation-modulated primitive adjusts the controller's weight on the
  raw perceived signal within life, driven by the running correlation between that signal and the creature's
  own reserve delta, with no committed labelled category.

The panel converged (five of five returning), and the verdict is verified against source below.

### 2a. Reject (i)

Two independent grounds, unanimous on the panel:

- **Admit-the-alien failure.** A belief store is an evidence-accumulating, within-life inference structure. The
  creature tier is DEFINED as fixed-for-life, no belief store, no within-life learning. Installing one is a
  rewrite of what the creature is, not a data row, so a non-learning creature (and a non-learning alien) can
  only carry it by rewrite. This is the exact failure the admit-the-alien bar names.
- **Template-case violation.** (i) commits a valence from a closed labelled set {harmful, benign, rewarding,
  neutral} and then READS that committed category to produce the movement. The template case's general form
  names "a named emotion" as the paradigm high-level fact whose reading is authoring, and a committed valence
  is functionally that; the closed set also trips P8's closed-enum ban.

Two panelists observed that this defect is INHERITED from the existing founder path (which the smoke test
forbade assuming was de-authored). That is a genuine question about already-built, gate-signed work; it is
FLAGGED for a separate audit (Section 4), not folded into this arc. The narrower point stands for the
creature: even if a committed valence is defensible for the founder (a learner, whose tier is belief), it is
a rewrite for the creature (a non-learner).

### 2b. Adopt (ii) as the base

The creature's toward/away disposition emerges from cross-generational SELECTION on a raw-percept weight, no
belief store. The raw perceived being-signal (its channel and the creature's own discrimination bucket, no
valence, no committed category) feeds a new input into the creature's controller direction slot through a
heritable freely-signed weight (zero-default, unclamped); the sign (approach or avoid) and magnitude are set
by selection across generations, exactly as the creature's forage taxis already are. This is the
template-case cure stated almost verbatim (a general causal primitive, the raw signal to a direction, plus a
proxy that merely correlates, the channel and bucket, plus the outcome emerging from selection), it is native
to the creature's genome-expressed fixed-for-life controller, and it is feasible as data for a non-learning
and a non-Terran creature (a new channel is a new row in the same heritable weight table). The null outcome,
no disposition, is the true zero-weight default.

### 2c. Fold (iii) into (ii), do not branch

The four panelists who addressed it agreed (iii) is not a competing peer but a strict generalization of (ii):
make the within-life plasticity a heritable, genome-expressed plasticity COEFFICIENT on the SAME raw-percept
weight, defaulting to zero. At coefficient zero it IS (ii) exactly; at a positive coefficient the weight
drifts within life by the running correlation between the signal and the creature's own reserve delta. So
whether a lineage learns within life becomes a SELECTED emergent trait, legitimate under P9 (the coefficient
is a genome channel, an innate disposition as an authored input, whose value emerges), with no authored branch
or species-keyed dispatch between "learns" and "does not" (which would itself be a P8/P11 violation).

One refinement, from the strongest panelist and adopted: to keep the plasticity from AUTHORING the
"reserve-good therefore approach" coupling, the plasticity's DIRECTION is itself a heritable freely-signed
parameter whose sign selection sets, not the rule. A lineage can then inherit "learn to avoid what precedes
reserve loss" or "learn to approach it" (a scavenger or parasite drawn to a death-cue is reachable), so the
direction experience pushes a creature is selected, never coded.

---

## 3. The two shared upstream sites, and the one that is decisive

The frame-blind named two authoring sites all three mechanisms share, so neither is settled by the mechanism
choice.

- **The magnitude-bucket quantization (decisive), VERIFIED CLEAN in the existing substrate.** The panel's
  sharpest concern was that if the bucket count and boundaries were a global authored grid, the subject id
  would be an authored category (P11) in the path of world content and perception would not be de-authored
  regardless of the downstream mechanism. Checked against source: it is not a global grid. `sense`
  (`perception_percept.rs:89`) computes the bucket via the perceiver's OWN discrimination law and step (its
  just-noticeable-difference), and the type comment states the bucket "derives from the being's own sense,
  never an authored taxonomy" (`:70`). Every `ChannelTransduction` field is the being's own data (P11,
  `:44`). The BUILD CONSTRAINT this imposes: the creature must carry its OWN channel transduction (derived
  from its genome and anatomy, or declared as its data), never a shared global constant, so its bucketing is
  its own.
- **The sign-space of the heritable freely-signed weight.** All three resolve approach versus avoid through
  that weight. It must be symmetric, unclamped, and zero-defaulted, with no downstream normalization
  reintroducing an approach bias, so "no disposition" is the true null and neither sign is privileged. The
  founder being-block weight is already expressed unclamped and founder-zero (`controller.rs`), which the
  build inherits and a test must pin.

---

## 4. Flagged for a separate audit (not this arc)

The founder being-percept path commits a labelled valence from a closed set and reads that committed category
to produce the gradient. Two panelists flagged this as the same template-case pattern (i) is rejected for. It
is already built and gate-signed. Whether it is a real defect turns on a question this arc does not settle:
is "a reserve fell, therefore harmful" a floor-level primitive (harm as reserve loss is near-definitional, an
authored floor value the value-authoring line permits) or a cultural outcome that must emerge? Surfaced for
the gate to decide whether to open a separate audit; NOT changed here.

Also flagged, the shortcut to avoid in the build: the codebase derives a `trophic_label` from what a species
eats (`biosphere.rs:185`). It is derived-not-stored, but it is still a high-level categorical fact, and
reading it to decide flee versus hunt would be the template-case violation (a derived fact is not exempt; the
test is whether the mechanism reads it). Mechanism (ii) by construction never needs it; the build keys only on
the emitted signal and the creature's own data.

---

## 5. The one build fork for the gate (downstream of the framing)

Mechanism (ii) needs the creature to enter the perceive path without a full `Mind`. The perceive loop is
gated on `world.mind(w.id)` and the founder direction comes from a gradient that reads `mind.belief(...)`. A
creature has neither. Two ways to feed the raw signal to the creature's controller slot without a belief:
(a) a per-creature belief-free percept that computes a raw-signal direction (an inverse-distance vector over
perceived emitters, per the creature's own bucket) and writes it straight to the slot; or (b) refactor the
slot-feed so the creature tier supplies the raw percept where the founder tier supplies the belief gradient.
This is a build-structure choice, not a framing one; it is scoped in the build plan once the gate rules the
framing.

---

## 6. Discipline and sequencing

Frame-blind done (section-11 smoke cleared at V5; section-10 panel run and verified against source, this
document). The gate rules the resolved framing before any code. The arc is byte-neutral until the live wire
lands (arming a creature's raw-percept path changes behaviour and re-pins; that step states its hash change
and is sequenced with the gate). The branch is `claude/creatures-react-percept`, off current `main`
(`b7f99eb`), opened as a new PR per the bridge rule so the gate can merge the affordance-composer PR (#115)
once this is open. Section-9 five-lens audit before every push.
