# TODOS.md: Live Backlog Mirror

The working view of the research backlog (audit Section 3) and the reserved-values queue. Updated every session: resolved items move to the resolved list, new flags are added, order is adjusted. Each open item is one bullet, identifier first, then the one-line question, then readiness and couplings, so the backlog tool reads it deterministically. The owner sets the order; the readiness tags are guidance.

Counts: 18 resolved, 26 open. Source of truth for the full reasoning is `docs/design.md` and `docs/audit.md`.

The build plan toward the first end-to-end emergent slice (generated map, seeded bands, the naming game over the primes, emergent dialogue) is in `ROADMAP.md`. Its milestones:

- **M0: lock the determinism keying contract.** Resolve R-RNG-COORD (canonical tick in every draw coordinate, a phase and draw-site registry replacing the hand-assigned `PHASE_*` tags) and pin R-REDUCE-ORDER for the gossip-conflict apply and the weighted pick. The one genuine research item on the path.
- **M1: a generated visual map (CPU, headless).** Topology module, a data-driven terrain and biome substrate, fixed-point worldgen, the chunk grid and quadtree, and a headless glyph frame. No research item (worldgen is unbuilt, not unresolved). Defer R-GPU-CANON-PIN, R-WEATHER, R-CATASTROPHE, R-SAVE-SCHEMA, R-UNITS-PIN.
- **M2: seed bands at the dawn of sentience.** Being registries and a `Race` definition, the genome and genotype-to-phenotype map, the allele-frequency pool with Hardy-Weinberg promotion, intrinsic-belief seeding, the axiom and value update kernel, and `seed_dawn_populations` onto the map. Defer R-BUILD-PHYS; register the genome and axiom projections ahead of R-PROJ-REGISTER. Reserved axiom, genome, and value-metric calibrations are surfaced, set as dev fixtures, fail-loud under Calibrated.
- **M3: the naming game over the primes.** Load the roughly sixty-five NSM primes as data, a thin concept-and-lemma representation, and the bounded once-per-culture dawn dynamic with the reserved `lang.dawn_round_cap`. Defer R-LANG-TYPOLOGY.
- **M4: emergent dialogue (implement R-CONVERSE) and the loop.** The move and force registries, the content gate, move-as-event, the conversation query, speak-as-intent, the response loop, grounding as said-evidence (regression-tested first), repair, the LOD gate, the four determinism pins, and the two Steering Audit invariants. Defer the deliberation, persuasion, and negotiation couplings.

---

## Open, ready to take (substrate or pattern already resolved)

- **R-INFRA.** The etic-and-emic question applied to the buildings institutions inhabit (infrastructure). Ready: reuses the etic-and-emic substrate pattern resolved for institutions (R-INST). Couples to Parts 36, 46.
- **R-LANG-TYPOLOGY.** Hardening the grammar typological parameters (word order, head direction, morphological type, alignment) from a closed enum to a data-defined substrate. Ready: reuses the data-substrate pattern; surfaced by the R-LANG-DET generalization audit. Couples to Parts 33, 25.
- **R-WOUND.** Emergent wound representation without an authored wound enum. Ready: pairs with R-FLUID; both sit in Part 35. Couples to Part 35.
- **R-FLUID.** Emergent body-fluid representation without an authored fluid enum. Ready: pairs with R-WOUND. Couples to Part 35.

## Open, coupled or partly reachable

- **R-COMMS.** Long-range communication and information flow. Partly reachable: the mail half is a composition of resolved institutions (R-INST) and open infrastructure (R-INFRA); the signal half waits on deep technology. Couples to Parts 41, 36, 46, 9.
- **R-RELATION.** How membership, role, lineage, and social edges are typed without a back-door taxonomy. Waits on the unified graph substrate; most-wired, so taken late. Couples to Part 10 and the graph substrate.
- **R-EVENT.** The event schema without an authored event enum. Most-wired item; taken last with R-RELATION. Couples to Part 7 and nearly everything that writes history.

## Open, far-horizon (held by standing instruction)

- **R-DEEPTECH-SCIENCE.** The emergence of abstract, explanatory, mathematical knowledge as a compounding enabler. Waits on the technology layer built and proven at small scale. Couples to Parts 41, 28, 9.
- **R-DEEPTECH-PHYSICS.** The tiered, deepening, discoverable physics substrate and its completeness and audit. The reach of technology equals the reach of this substrate. Couples to Part 41.
- **R-DEEPTECH-DEPTH.** Deep but emergent prerequisite structure from physical and logical necessity, as a standalone proof, not an authored tech tree. Largely satisfied structurally by the resolved composition mechanism; remains as its own proof. Couples to Part 41.
- **R-DEEPTECH-SCALE.** Running the whole compounding process over deep time at the aggregate tier, cheaply and deterministically. Couples to Parts 41, 54.
- **R-VIEW-ELAB.** The non-authoritative view elaboration that must stay consistent with the canonical tiers without writing canon. Waits on the level-of-detail model being built. Couples to Part 54.

## Open, world-content enum-opening (deferred until their session)

- **R-BUILD-PHYS.** The one remaining connected item from the being-model work (reserved). Couples to Part 20 and physics.
- **R-CATASTROPHE.** Emergent catastrophe representation without an authored catastrophe enum. Couples to Part 26.
- **R-CONTACT.** Emergent first-contact and contact-origin representation without an authored enum. Couples to Part 21.
- **R-DOMAIN.** The sacred-domain content that feeds the belief axis, without an authored domain enum. Couples to Part 38.
- **R-WEATHER.** Emergent weather representation without an authored weather enum. Couples to Part 18.

## Open, engine-foundation hardening (surfaced by the determinism red/green audit)

The design-level seams the red and green audit found, where a determinism, conservation, referential-integrity, or observer-independence invariant is asserted but not yet structurally guaranteed. The implemented-bedrock defects the same audit found are already fixed in code. These sit with the engine, scheduling, and level-of-detail foundations.

- **R-RNG-COORD.** Canonical RNG keying beyond (entity, phase): a tick coordinate, draw-site namespacing, and a camera-independent entity key. Site Part 3.2; couples to Parts 9, 11, 16, 33, 54.
- **R-CANON-WALK.** Enforce StableId-sorted iteration over every canonical container so no hash or walk is built over hash-map order. Site Part 3.5; couples to Parts 2, 7, 9, 19, 55.
- **R-CMD-ORDER.** A total command-application order independent of the thread count. Site Part 4.3; couples to Parts 2, 7.
- **R-GPU-CANON-PIN.** Pinned fixed-point multiply, divide, and transcendentals, and the full backend set, for cross-vendor bit-identity. Site Parts 3.4, 5.4; couples to Parts 12, 13, 60.
- **R-PROJ-REGISTER.** Each two-tier subsystem registers an exact conserved projection rather than a lossy mean. Site Part 58; couples to Parts 9, 11, 25, 33, 36, 54.
- **R-REDUCE-ORDER.** A total order for every non-associative canonical combine. Site Part 57; couples to Parts 9, 15, 17, 25, 41, 58.
- **R-SAVE-SCHEMA.** Versioned saves, restored id high-water marks, and a pinned on-disk layout. Site Part 7.3; couples to Parts 2, 40, 58.
- **R-HARNESS-COVER.** Extend the determinism harness over phases, command buffers, gossip, mutation, a thread sweep, and save and replay. Site Part 3.5; couples to Parts 4, 7, 9, 11, 54.
- **R-UNITS-PIN.** Pin the Part 55 unit system: scales, canonical direction, overflow policy, and exact round trips. Site Part 55; couples to Parts 3, 9, 40.

## Recorded candidate explorations (not yet formally flagged, not in the open count)

Two exploration documents the owner asked to record now and queue for later. They are stored verbatim under `docs/working/`, are vehicles for future research items rather than counted backlog entries, and do not change the 27-open total until they are formally flagged through the workflow. Both sit far out, sequenced after the simulation core and the epistemics are built and proven, since each reads through systems that have to exist first.

- **R-EMBODIMENT (candidate).** How a player could control a character or lead a band while holding determinism, observer-independence, the per-individual epistemics, and emergence. The on-ideology direction keeps the learned axes as the lens and the limit: the player supplies will and intent, the existing systems resolve perception, capability, and social outcome, player input enters the event log as exogenous events for replay, and the decision source becomes a per-agent data property (Principle 11). The hard build is the epistemic-rendering interface, showing a belief store rather than canon, including absence and falsehood. Vehicle: `docs/working/PLAYER_EMBODIMENT_EXPLORATION.md`. Couples to Parts 7, 8, 9, 33, 36, 37.
- **R-CONFIG (candidate).** How an observer configures a world, and where the line falls between tunable and restricted. The control surface is the profile, not the engine: selection levers are free, value overrides are free within their declared ranges, and generative levers run the steering and computability gates. Canon-affecting levers are seed-time and reproducible; observer-side controls are runtime and never enter canon. Open within it: the exact runtime-mutable set, the validation of override ranges against the substrate, and the configuration interface. Vehicle: `docs/working/WORLD_CONFIGURATION_AND_LEVERS.md`. Couples to Principles 3, 8, 9, 10, 11 and the world-profile and demand-closure model.

---

## Reserved-values queue

The numbers the resolved mechanisms need are surfaced, not invented, and are the owner's to set. The queue is the set of `status = "reserved"` entries in `calibration/reserved.toml` (see the runbook). Each carries its basis and a pointer to its mechanism. Work them through the reserved-values panel; set on the stated basis, validate against the stated target, then graduate the entry to `set` and annotate the matching reserved list in `docs/design.md`. No reserved value is decided by the agent.

## Inconsistencies

- **Inconsistency 5.** Part 23's authored technique web versus Part 41's emergent design space and content gate. Owner decision, reserved. The composition promotion gate (R-DEEPTECH-COMPOSE) now leans on this, since it promotes a stabilised technique into the join space as it promotes an artifact. Flagged in the Part 41 blockquote, record 62.10, and audit Section 4.
- **Inconsistency 6.** EventId as a dense storage index (Part 7.2) versus the truncation of demoted-entity events (Part 7.3). Surfaced by the determinism audit. Owner decision, reserved. A note is at the Part 7.3 site and the entry is audit Section 4, item 6.

---

## Resolved (18)

R-BEING-REP, R-VALUE-METRIC, R-AXIOM, R-GENOME, the six language-and-meaning items (R-LANG-CONCEPT, R-LANG-DISTANCE, R-LANG-LEARN, R-LANG-DISTORT, R-LANG-WRITING, R-LANG-GEN), R-EVIDENCE, R-INST, R-TIER-CONSIST, R-DEEPTECH-COMPOSE, R-TOM-UPDATE, R-LANG-DET, R-LANG-MODALITY, R-CONVERSE. Each has a mechanism in the design document, a record in Part 62, a bibliography group in Part 63, and a consolidation block in audit Section 1, with its calibrations reserved. R-TOM-UPDATE resolved 2026-06-29: the recursive theory-of-mind update is the evidence engine run recursively on whether a target believes a thing, fed only by access evidence (a data registry, not a closed enum) so it diverges from projection (Part 37, record 62.11). R-LANG-DET resolved 2026-06-30 as a deep focused session with two bounded adversarial rounds: the order-sensitive language procedures are pinned (minted concept ids, innovation-index form-change ordering, a widened leaky salience accumulator, a symmetric matching-free distance over the shared semantic substrate, a deterministic dawn trigger), worded over the generic form substrate so R-LANG-MODALITY adds only data; it surfaced the new sibling item R-LANG-TYPOLOGY (Part 33, record 62.12). R-LANG-MODALITY resolved 2026-06-30 by a five-facet fan-out and a three-skeptic verification pass: language is generalized to a data-defined modality substrate (production modalities, reception senses, feature dimensions, media; a channel a derived pairing; a form primitive a canonical simultaneous-feature bundle; per-being produce/perceive channels from the genome), adding only data so no R-LANG-DET pin reopens, with acquired injury-loss reserved to the open R-WOUND; the work retired R-LANG-DET's optional cross-modality ceiling and added a modality-swap invariant to the Steering Audit (Part 33, record 62.13). R-CONVERSE resolved 2026-06-30 by a five-facet fan-out and a steering-and-determinism verify pass: modelled dialogue is the promoted-tier refinement of the gossip loop, communicative acts first-class canonical events over a data-defined dialogue-move registry whose force composes from resolved primitives, a conversation an emergent query, speaking a utility-AI intent, the listener-side update the Part 37 single-utterance procedure with grounding reduced to said-evidence (no new common-ground prior); the verify caught two steering leaks (the register knob removed from canon, the conversational common-ground channel) and a determinism dependency on the open cluster, and added two Steering Audit invariants (Part 9.5, record 62.14). Its deeper couplings (deliberation, persuasion, negotiation) are scoped and reserved.
