# TODOS.md: Live Backlog Mirror

The working view of the research backlog (audit Section 3) and the reserved-values queue. Updated every session: resolved items move to the resolved list, new flags are added, order is adjusted. Each open item is one bullet, identifier first, then the one-line question, then readiness and couplings, so the backlog tool reads it deterministically. The owner sets the order; the readiness tags are guidance.

Counts: 14 resolved, 28 open. Source of truth for the full reasoning is `docs/design.md` and `docs/audit.md`.

---

## Open, ready to take (substrate or pattern already resolved)

- **R-TOM-UPDATE.** How the nested mental model is populated from second-order evidence so it diverges from projection, and what is assumed at the depth bound. Ready: substrate is the resolved evidence engine (R-EVIDENCE) applied recursively to whether a target believes a thing. Couples to Parts 9, 21, 25, 28, 37, 54.
- **R-INFRA.** The etic-and-emic question applied to the buildings institutions inhabit (infrastructure). Ready: reuses the etic-and-emic substrate pattern resolved for institutions (R-INST). Couples to Parts 36, 46.
- **R-LANG-MODALITY.** Generalizing the language system to signed and non-vocal modalities. Ready: extends the resolved language-and-meaning cluster. Couples to Part 33.
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
- **R-LANG-DET.** Pin sound-change order, salience overflow, concept-id minting, the shared-concept traversal, and the dawn trigger. Site Part 33; couples to Parts 9, 21, 55.

---

## Reserved-values queue

The numbers the resolved mechanisms need are surfaced, not invented, and are the owner's to set. The queue is the set of `status = "reserved"` entries in `calibration/reserved.toml` (see the runbook). Each carries its basis and a pointer to its mechanism. Work them through the reserved-values panel; set on the stated basis, validate against the stated target, then graduate the entry to `set` and annotate the matching reserved list in `docs/design.md`. No reserved value is decided by the agent.

## Inconsistencies

- **Inconsistency 5.** Part 23's authored technique web versus Part 41's emergent design space and content gate. Owner decision, reserved. The composition promotion gate (R-DEEPTECH-COMPOSE) now leans on this, since it promotes a stabilised technique into the join space as it promotes an artifact. Flagged in the Part 41 blockquote, record 62.10, and audit Section 4.
- **Inconsistency 6.** EventId as a dense storage index (Part 7.2) versus the truncation of demoted-entity events (Part 7.3). Surfaced by the determinism audit. Owner decision, reserved. A note is at the Part 7.3 site and the entry is audit Section 4, item 6.

---

## Resolved (14)

R-BEING-REP, R-VALUE-METRIC, R-AXIOM, R-GENOME, the six language-and-meaning items (R-LANG-CONCEPT, R-LANG-DISTANCE, R-LANG-LEARN, R-LANG-DISTORT, R-LANG-WRITING, R-LANG-GEN), R-EVIDENCE, R-INST, R-TIER-CONSIST, R-DEEPTECH-COMPOSE. Each has a mechanism in the design document, a record in Part 62, a bibliography group in Part 63, and a consolidation block in audit Section 1, with its calibrations reserved.
