# World Configuration and the Lever Control Surface

**How an observer configures a world to their liking, and the line between what is tunable and what is restricted.**

Status: this builds directly on the world-profile and demand-closure model (the Physics Substrate Guide and the seeded-variation discussion). It is mostly derivable from the existing principles and is stable enough to build against, with the standing clause that any locked decision yields to evidence. The runtime-mutable set and the configuration interface are open.

---

## 1. The control surface is the profile, not the engine

You do not tune a world by editing the substrate or the engine. The full catalogue produced through the fan-out is an immutable library; a world is a profile, a manifest that selects which consumers are active and overrides specific values, and the world's actual substrate is the closure computed over that profile. Configuring a world is authoring its profile; the world is then derived. This is what makes configuration safe and reversible at once: a profile change selects and re-derives, it does not mutate a shared artifact, so switching a world from full-fantasy to humans-and-base-physics destroys nothing and is reproducible.

## 2. What already governs this

- **Principle 9:** physics is the one authored layer, subject to the Steering Audit. Configuration may enter that bounded layer and no other.
- **Principle 8:** order emerges and is never templated, which is the reason an observer may set the space and the rules but not the outcome.
- **Principle 11:** the world is data-driven, and consumers read what the substrate presents rather than assuming a fixed set, which is the discipline that makes subtraction safe.
- **Principle 3 and Principle 10:** determinism and observer-independence, which fix what may change at seed time versus runtime and bar god-knowledge from feeding back into the world.
- **The world-profile and demand-closure model:** a world is a profile, its substrate is the closure over that profile, and the catalogue is the immutable library beneath all worlds.
- **The seeded-variation model:** a profile may carry a generative budget for bounded novelty, which runs the same gates as the catalogue.

## 3. The lever tiers

Three kinds of lever, by cost and safety.

**Selection levers, free.** Which races are active, magic on or off, which catalogue axes and laws are active, the tech ceiling, and which consumers are active. These are subtraction and subsetting over a validated catalogue, so they are always safe: the closure omits what the profile does not select, and because the value and semantic world-axes project from the active physics, the downstream layers omit the corresponding concepts and values on their own.

**Value levers, free within range.** Overrides of reserved and real values within their declared ranges: gravity stronger, iron harder, a drive weaker. These are value changes inside an already-validated closure, bounded by each value's declared range, so they are safe as long as they stay in range. A value outside its physical range is restricted, because it can break the substrate's validity.

**Generative levers, gated.** The seeded-novelty budget and the grammar selection: how many novel axes the grammar may generate for a world, and which grammar. These add to the authored layer, so they run the same gates a catalogue entry runs, the steering audit and the computability checks, and they are gated for that reason.

## 4. Seed-time versus runtime

Most levers are seed-time: they are set before or at world generation and become part of that world's deterministic identity, so changing one produces a different world rather than altering the running one. This is deliberate, because a mid-run change to the physics or the active consumers would break replay and the world's identity. Runtime controls are restricted to the non-authoritative: the observer's view (camera, level of detail, what to watch), the time rate, and pausing, none of which touch the canonical state. The split is clean: canon-affecting levers are seed-time and reproducible, and observer-side controls are runtime and never enter canon.

## 5. The allow-versus-restrict taxonomy

**Always allowed, free and safe:** selections and subtractions, value overrides within range, the observer view controls, and the seed choice.

**Allowed but gated, running the validation gates:** adding novel axes through the grammar, extending the catalogue, and supplying a custom grammar.

**Restricted, because they would break the ideology:** editing the engine core (the scheduler, the fixed-point and RNG layer, the storage), which is the determinism foundation; injecting an outcome directly (authoring a specific cultural, historical, or technological result), which is the steering leak Principle 9 exists to prevent; runtime edits to canon-affecting state, which break determinism and replay; giving the observer god-knowledge that feeds back into the simulation, which breaks observer-independence and the per-individual epistemics; and values outside their declared physical ranges, which break the substrate's validity.

The principle behind the line: you may tune the space, the rules, and the selections, and you may not author the outcome or break the determinism and the epistemics. That is Principle 8 and Principle 9 stated as a configuration policy.

## 6. Determinism and world identity

A world's profile and its seed are its reproducible identity: the same profile and seed, with the same recorded inputs, replay the same world. A smaller profile is also cheaper to run, so the simpler worlds are a quiet performance win. The non-authoritative runtime controls do not affect this, because they never enter canon.

## 7. Who tunes what

The observer and configurer author the profile before the seed: the selections, the value overrides, the ceiling, the magic switch, and the generative budget. The modder works one layer deeper, in the data the profile draws on: the catalogue, the grammars, and the consumer definitions, all under Principle 11. The embedded player, the subject of the embodiment exploration, does not tune; they act within the world as an agent, which is a different relationship covered separately.

## 8. The open parts

The exact runtime-mutable set (what, if anything, can change mid-run without breaking replay, which is probably a small set worth specifying precisely), the validation of value-override ranges against the substrate, and the configuration interface itself. These are deferred, not guessed here.
