# Mirror fully alive: the derive-first roadmap

The vision: Mirror (1:1 Earth) as a world where human-anatomy-driven people THINK, ENGINEER, CRAFT, HUNT,
CONFLICT, WORK, FEEL, CONVERSE, and live REAL LIVES dictated by their bodies. This is the sequenced arc the
cloud agent is guided through after the loader (Arc 1) and calibration (Arc 2). Built from a 7-component
run-path-state audit, then run through the DERIVE-FIRST pass: every arc is stated as a general causal
PRIMITIVE plus a PROXY that correlates plus selection/learning, with the FLOOR substrate to build, so the
outcome EMERGES. Author only in the physics floor. The authoring traps are flagged explicitly.

## The two structural truths the audit exposed

1. **Most of "thinking" is built but not wired onto the canonical path.** `build_dawn_runner` arms the
   first-order belief loop (inference, gossip, theory-of-mind lie detection, harm learning) canonically, but
   the entire deliberative half (appetitive reward learning, the discovery/hypothesis loop, the forward model,
   one- and multi-hop planning) is wired ONLY into `run_world.rs`, the quarantined dev harness. So a large part
   of "alive" is ACTIVATION (wire the emergence-clean substrate onto the canonical spine), which is exactly
   what Arc 1 gap (b) is doing, not new authoring.
2. **The genuine new build is a small set of keystones**, each a percept or primitive keyed on the being's own
   physics, from which whole capability layers emerge: the being-perception percept (hunting, conflict,
   witnessed-affect, social), the affect-to-decision percept wire (emotions shape behaviour), the inter-agent
   matter-transfer primitive (economy), the run-path relation-formation producer (multi-hop reasoning and
   engineering), and lifespan-from-anatomy (R3, real lives).

## The authoring traps flagged by the derive-first pass (do NOT let these be authored)

- **Deliberation must derive from the EVOLVED substrate, never the barred utility-AI.** `decision.rs`'s
  utility-AI (Part 8.1) is architecturally REFUSED on the canonical spine (`Runner::with_world` asserts
  `!world.has_behaviour()`) precisely because it is authored steering. Deliberation must emerge from the evolved
  controller plus the forward-model plus the planning/discovery loop (a being simulates candidate actions and
  picks by predicted reward, all evolved/learned), never a hand-authored utility function with authored
  considerations. Resurrecting `decision.rs` is the trap.
- **Emotions enter decisions as an evolved-controller PERCEPT, not an emotion-to-action table.** Affect
  (`affect.rs`) is already derive-clean (a signed affect delta from a measured DRIVE change, not an authored
  event-to-emotion map). Wire it in the conviction-percept pattern: affect is a percept the evolved controller
  learns to weight, so which feeling moves which choice emerges per being. Do NOT feed it into the barred
  `decide()` utility-AI, and do NOT author "feeling X causes action Y."
- **Economy, specialization, money, war, institutions must EMERGE, never be authored systems.** Build the
  inter-agent matter-transfer PRIMITIVE (a being can give/take located matter); exchange emerges from mutual
  benefit over the value/need substrate; specialization emerges from skill-learning plus comparative advantage;
  money emerges as a widely-valued exchange medium; institutions crystallize from recurring coordination over
  need-vectors (R-INST, already the design); group conflict emerges from individually-selected aggression
  correlating with proximity and scarcity. No trading system, no occupation registry, no authored currency, no
  named government/religion, no war rule.
- **Recursive composition must emerge from observed combination, not an authored tech tree.** Bridging a
  being's decided action to `crates/compose` must let a being combine known affordances into a new one when the
  combination is observed to succeed, not walk an authored technology graph.
- **Dialogue EFFECTS must derive through the evidence engine and the evolved controller.** A canonical
  dialogue-move registry is legitimate owner DATA (an authored input, like the physics floor), but persuasion,
  deliberation, and belief-to-action must run through the same evidence/value kernels and enter behaviour as a
  percept, never an authored "this move changes that opinion / causes that act."

## The sequenced arcs (derive-first)

**Arc 1 (in progress): the loader + canonical activation of the ideation stack.** Make the scenario name shape
world structure (done, slice 1); move the discovery / reward / felt-conviction learners onto `build_dawn_runner`
fail-loud from the manifest, opt-in per scenario (byte-neutral off). This brings the built deliberative-thinking
substrate alive on the canonical spine. ACTIVATION, emergence-clean.

**Arc 2: physics + units + Mirror/Tempest calibration.** Set the values (the 162 cited proposals), present
Mirror for owner sign-off.

**Arc 3: the liveliness keystones (the heart of "alive").** Each a percept or primitive keyed on the being's
own physics; each framing-panelled before it is built. REFRAMED (2026-07-09, after six section-11 framing
catches showed the being-percept's cleanliness depends on unbuilt substrate): Arc 3 OPENS with the
perception-substrate arc below, which the being-percept and the predation loop consume second. The full plan
and the derive-first proposal are in `PERCEPTION_SUBSTRATE_ARC_PLAN.md`; the grounding study is
`PERCEPTION_GAPS_STUDY.md`.
- **Arc 3.0, the perception-substrate arc (precedes the being-percept).** Three slices, each framed blind
  (section-11 fail-closed, then section-10) before code, under three hard constraints: a signal carries a
  PHYSICAL MAGNITUDE never a valence at emit; perception keys on the being's OWN installed senses (an alien
  lacking a sense does not perceive that channel); meaning and valence EMERGE receiver-side under selection.
  (1) The REACH WIRE, the clean first slice: wire the existing tier-0 floor reach laws
  (`inverse_square_falloff`, `optical_depth`, `acoustic_absorption`) to a perception read, distance derived from
  the Path B coordinate model, no valence risk. (2) The SENSORIUM-GATED MAGNITUDE PERCEPT: gate the live Path B
  percept on the being's own sensorium (`Sensorium::reads`), acuity and JND deriving from genome and anatomy,
  reconciling Path B's current ungated universality. (3) The RECEIVER-SIDE VALENCE LEARNER: reuse the existing
  felt-outcome correlation learner (`learn.rs:504-526`) so a signal's meaning is learned from the receiver's own
  reserve delta, never stamped at emit; the two authored likelihoods at its head (`good_weight` inputs) are the
  arc's deepest derive target. The emergence-critical core (the felt sign as the being's own reserve delta, the
  raw percept as its own-cell quantity) already derives clean and alien-clean, so it is reused, not rebuilt.
- **The being-perception percept (CONSUMES Arc 3.0).** An opt-in, hash-neutral controller input block carrying,
  per nearby cell, occupant-derived scalars (body mass, weapon development, tissue, distance and direction),
  keyed on NO species/identity tag, sensed through the perception substrate above (a physical magnitude,
  sensorium-gated, its meaning learned receiver-side). The evolved controller learns to pursue or flee under
  selection. Emergent target: hunting, fleeing, cannibalism, cross-species, from one mechanism. KEYSTONE.
- **Run-path being-vs-being harm.** A STRIKE-affordance arm that resolves a being's aimed heading to a target
  occupant via the located index and wounds its body through the existing floor wound laws (`body::strike`), a
  kill leaving a corpse the hunter eats/butchers. Keyed on body physics. Predation emerges.
- **The affect-to-decision percept wire.** Affect as an evolved-controller percept (the conviction pattern),
  plus per-race `AppraisalBinding` data (a P9-legal innate disposition, legitimate for affect), plus
  witnessed-event appraisal once the being-percept exists. Emotions shape behaviour, emergently.
- **The inter-agent matter-transfer primitive.** Give/take located matter between beings; exchange, sharing,
  and theft emerge from the value/need substrate and selection. The seed of the economy.
- **The run-path relation-formation producer.** A being forms `(action, YIELDS, thing)` relations from its own
  observed action-outcome co-occurrence (the existing evidence engine), so `plan_chain`'s multi-hop tool
  reasoning finally has a producer. Engineering/tool-reasoning emerges.
- **The made-world affordances on canonical + the biosphere-meets-made-world seam.** Arm CUT/CRAFT/EXTRACT/etc.
  on the canonical path (opt-in), and expose a biosphere producer (a tree) as a cuttable target with a physics
  yield, so a being fells a tree for wood of its own volition. Crafting/engineering emerges.

**Arc 4: real lives dictated by anatomy.** Lifespan derived from the body (R3, the senescence law reading mass,
metabolism, organ integrity); a fatigue homeostatic axis plus a rest percept (sleep emerges); progressive
childhood/development (the morphogen kernel run across the life cadence); a disease/pathogen substrate keyed on
the alien-clean toxin-tolerance primitives; injury-on-the-run plus healing from the body's own repair physics
(medicine a discovered technique).

**Arc 5: the social depth (gated on the being-percept + transfer).** Institutions from recurring coordination
(the crystallization detector wired live); emergent specialization and an emergent medium of value; dialogue
depth (persuasion/deliberation/negotiation through the evidence and value kernels, a canonical data-authored
move registry); group conflict emergent from individual aggression and scarcity; justice (R-JUSTICE).

## How this reaches the agent

This is the forward plan; the status board in `CONSENSUS_ROADMAP.md` stays the current is-X-done state, updated
in place by whoever lands the code. I hand the agent Arc 3 onward through PR comments at the Arc-2 -> Arc-3
transition, each keystone framing-panelled first and five-lens-audited at arc end, with these derive-first flags
as the standing guard against authoring.
