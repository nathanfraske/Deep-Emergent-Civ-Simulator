# Embodied Play: Controlling a Character and Leading a Band (Exploration)

**A future-direction exploration of how a player could control a character or lead a band in the style of Dwarf Fortress, kept consistent with the project's principles.**

Status: an exploration, not a commitment. The project is simulation-first, and watching is the default mode; embodiment is an addition that has to hold determinism, observer-independence, the per-individual epistemics, and emergence, or it is not this project. This document lays out the design space, recommends the on-ideology direction, works through the hard reconciliations, and marks the open questions. It is a candidate future research item, not a settled design.

---

## 1. The question

The fork posed: substitute the learned axes for how the player interacts, or find another framing. The short answer this exploration reaches is that substitution is the least on-ideology option, and the strong version keeps the learned axes as the medium through which the player perceives and acts rather than discarding them. The rest is why, and how.

## 2. What already governs this

Any embodiment design has to satisfy what is already true of a character in this world.

- A character is a stat block in no part of this design; it is an emergent agent with beliefs formed from witnessing that can be true or false (R-EVIDENCE, Part 9), values over the value substrate (Part 21), a theory of mind about others (Part 37), language learned with mutual-intelligibility friction (Part 33), techniques and knowledge held as a stock that can be lost (Part 23), drives and personality, memory, and relationships. Its actions normally emerge from all of this through the decision system (Part 8).
- The simulation runs the same whether observed or not (Principle 10, observer-independence), every consequential change is an event in the log that is ground truth (Part 7), and the world is deterministic (Principle 3).
- A group is a social structure of real agents, carried by the emergent institutions (Part 36), relationships, values, and theory of mind.

These are the constraints, and they are also the materials: the systems that make embodiment hard are the same systems that make it rich.

## 3. The on-ideology principle

The player is an agent in the world, subject to the same epistemic, capability, and social constraints as any agent. The player supplies will and intent; the world's existing systems resolve perception, knowledge, capability, and social outcome. The learned axes are the lens and the limit, not a thing to bypass. The player does not see an orc, the player sees what the character believes about a figure from the evidence the character holds. The player cannot speak to a foreigner the character shares no language with. The player cannot wield a technique the character never learned. What the player adds is the choice of what to attempt; what the world adds is whether and how it succeeds, through the systems that were already running.

## 4. The framings

**Player as the character's will, epistemically bounded.** The player perceives the world through the character's belief store rather than through ground truth, so the fog is epistemic rather than spatial alone: a thing the character has not witnessed is not shown, and a thing the character believes falsely is shown as the character believes it. The player knows only what the character knows, acts within the character's physical and skill capabilities, and is subject to its drives, but the player chooses the action the decision system would otherwise compute. This is the most direct control, and it is on-ideology exactly to the degree the epistemic bound is honored. The character's values and personality enter as advisory dissonance, surfaced to the player as the cost of acting against the character's nature, rather than as a hard lock, which keeps the player in control while keeping the character's nature real.

**Player as influence, character keeps agency.** The player suggests or leads, and the character's emergent decision-making complies or resists according to personality, values, and, for others, loyalty and their beliefs about the player. This is the lighter touch, and it is the natural model for leading rather than puppeteering. A character led this way can refuse, misunderstand, or follow imperfectly, which is the source of the drama.

**Player provides intent, the simulation resolves capability and outcome.** The player declares an intent, persuade him, fell this tree, forge this, and whether it works is resolved through the character's actual skill, the target's theory of mind and values, and the shared language, by the systems that already exist. Intent in, emergent resolution out. This refines the other two rather than competing with them: it is how a chosen action becomes an outcome.

These are not exclusive. The strong design is direct, epistemically-bounded control of one character (the first and third together) and influence over others (the second) through the social systems.

## 5. Why substitution is the weak option

Substituting the learned axes for player interaction, letting the player act with god-knowledge and full capability and the axes ignored, is the least on-ideology choice, because it discards the per-individual epistemics and the capability bounds that are the core of the project. A player acting on ground truth the character cannot have breaks observer-independence and the evidence model at once, and a player wielding knowledge and language the character never learned erases the systems that make a character a particular person. The axes are what make this world's characters unlike a stat block; making them the interface, the lens and the limit, is what keeps embodiment inside this project rather than turning it into a generic action game wearing the project's content.

## 6. Leading a band

Leading a group reuses the social systems wholesale rather than adding a command layer. The player occupies a role in the emergent institution and relationship graph, and the authority of that role is real and contestable, not a control interface. The band members are agents with their own values, loyalties, and beliefs about the leader formed through theory of mind and evidence, so an order is an influence resolved through those, and a member can misunderstand it across a language or knowledge gap, follow it poorly, desert, or mutiny. The player leads by being a node in the social graph with real but contestable standing, which is the emergent governance and institution system with the player occupying a position in it. This is among the most on-ideology features available, because it is the existing mechanism with a human in a seat, and not a new mechanism at all.

## 7. The hard reconciliations

**Determinism.** A human introduces choices the engine cannot derive. The reconciliation is the event log: the player's inputs are exogenous events recorded in the log that is already ground truth, so determinism becomes the same seed and the same recorded input stream reproduce the same world. The player's choices are part of the world's recorded timeline, which is how a deterministic system accommodates an external hand without giving up replay.

**Observer-independence.** A controlled character behaves by an external will rather than the internal decision function, which looks like a violation. The reconciliation is to make the decision source a property of the agent, data rather than a special case: an agent's actions come from its internal decision function or from an external controller, declared the way other per-agent properties are, and everything downstream treats the two identically, so the simulation does not privilege or even distinguish the controlled agent in its own machinery. This is Principle 11 applied to agency. The condition that keeps it honest is the epistemic bound: the controlled agent must see and act on its own beliefs, not ground truth, or the god-knowledge feeding back through its actions breaks the independence the principle protects.

**The epistemic interface is the real new build.** The view can show the observer ground truth; an embodied player must be shown the character's beliefs, which means rendering the world from a particular belief store, including absence and falsehood, rather than from canon. This is the largest new piece of engineering embodiment requires, and it is also what makes the mode worth building, because seeing the world as a particular mind believes it to be is the experience the project's epistemics uniquely can offer.

## 8. Why this is the good kind of fun

A character whose physics or society holds a quirk, an exploit, a false belief, a rare skill, is a character the player experiences from inside that limit, and the discovery, the misreading, and the consequence are all already modelled. The player leading a band that fractures over a value disagreement, or that rises on a technique only this character knows, or that follows a belief that turns out false, is watching the project's own systems produce a story with a hand on one of the levers. Embodiment is a seat inside the simulation that already exists, rather than a separate game bolted onto it.

## 9. Open questions, if this is pursued

- Control granularity: how direct the control is, and whether it varies by mode (a settled-life mode closer to influence, an adventuring mode closer to direct control).
- How values and personality bound the player: advisory dissonance, soft resistance, or free choice, and how the character's nature pushes back without removing player control.
- The epistemic-rendering interface: showing a belief store rather than canon, including absence and falsehood, which is the hard build.
- Authority resolution for leading a band: how the institution and relationship systems carry a player-held role, and how orders resolve as influence.
- Continuity when control lapses: what a character does when the player stops controlling it, and how it returns to emergent decision-making, which couples to the theory-of-mind update and the decision system.
- The input stream and determinism: recording and replaying player input as events, and the reproducibility that follows.
- The interface and the experience: the parts that are game-feel design rather than simulation design.

This is a candidate far-future research item. The clean way to track it, if pursued, is as its own item with this document as its vehicle, sequenced after the simulation core and the epistemics are built and proven, since embodiment reads through the very systems that have to exist first.
