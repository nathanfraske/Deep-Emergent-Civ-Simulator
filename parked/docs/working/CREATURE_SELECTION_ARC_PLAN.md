# Creature Reproduction / Behaviour-Selection Arc: kickoff opener (Agent B)

Doc-only bridge opener for the owner-queued **creature reproduction / behaviour-selection** arc, the Arc-7
payoff. No framing and no code yet: this PR exists so the gate can merge the creatures-react capacity (#118)
without stranding the follow-on, per the bridge rule. The frame-blind runs next, on this branch, and the
resolved framing is posted here for the gate's ruling before any code.

## What the arc is, and why it is the payoff

Arc 7 spawns biosphere creatures as living walker-agents that perceive, forage, metabolize, and die on the
same loop as the founders, but its own code flags the missing piece: "behaviour selection awaits the
reproduction slice." Today a creature's controller weights are FOUNDER-ZERO and stay there: creatures do not
reproduce, so selection never lifts a weight, so all creature behaviour (its forage taxis and, now, its
being-directed reaction) is latent. The creatures-react capacity (#118) proved this precisely: a creature
perceives a being and gets a real magnitude-graded pull, but a real pull times a founder-zero weight is zero
movement, so the reaction is byte-neutral until a weight is lifted.

This arc builds the piece that lifts the weights: creature REPRODUCTION and SELECTION over the creature
population, so a creature's disposition (forage and reaction alike) is set by differential survival across
generations rather than staying at founder-zero. With it, the being-directed reaction the #118 wire computes
becomes OBSERVABLE in the shipped `full --creatures` world (a creature that inherits a flee-the-strong-signal
weight outlives one that does not, so fleeing emerges; a hunt-the-weak-signal weight where it pays, so hunting
emerges), and `full --creatures` re-pins with the emergent behaviour. That is the last piece before
`full --creatures` is the integrated living-world scenario: creatures plus predation (the strike, #117) plus
the reaction, all emergent and watchable together, the capstone the owner named.

## Dependency and sequencing

This arc consumes the #118 creatures-react capacity (the founder-zero creature being-weights it lifts through
selection), so it branches off the merged `main` once #118 lands. Per the bridge rule, opening this PR
unblocks the gate's merge of #118; the frame-blind and the build then proceed here.

## The derive-first, emergence-critical caution (why it earns a frame-blind)

This is emergence-critical: selection over a FREELY-SIGNED disposition is exactly where an authored outcome
can hide. The whole point is that WHICH way a creature reacts (and forages) emerges from selection, never from
an authored fitness term, a species reaction, or a seeded sign. So the framing must be taken through the full
blind discipline before any code: the section-11 input-bias smoke test (fail-closed, strongest model) then the
section-10 blind panel, guarding against a selection mechanism that quietly authors the outcome it is supposed
to let emerge (a hand-tuned fitness that rewards the wanted behaviour, a seeded non-zero weight that authors
the sign, a reproduction rule that reads a high-level fact). The founder tier already has homeostatic-survival
selection (`GenePool::select`, the evolve substrate); the honest question the frame-blind will scope is whether
the creature tier reuses that same substrate as data (a creature is a data row in the same selection) or needs
its own, and how a creature reproduces without authoring who reproduces.

## Plan

Frame-blind first (section-11 then section-10), post the resolved framing here for the gate's ruling, then
build under the gate's ruling: byte-neutral scaffold first where possible, section-9 before every push, the
one intended re-pin (`full --creatures`, when the emergent behaviour goes live) stated and sequenced with the
gate. The branch is `claude/creature-selection`, off `main`. B2 (the per-bucket discrimination follow-on for
the reaction) stays a separate later item; this arc is the selection substrate that makes ALL creature
behaviour emergent.
