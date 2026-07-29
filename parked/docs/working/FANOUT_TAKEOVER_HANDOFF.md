# Takeover Handoff: the full physics-and-materials substrate fan-out

This is the entry point for an agent taking over to run the full physics-and-materials substrate fan-out (the "full fat" fan-out). It is self-contained: read it, then the files in section 0, and you can start. The design document stays the source of truth; this is the mission brief that ties the pieces together.

## 0. Read order (orient before acting)

1. `CLAUDE.md` (the operating manual: the eleven principles, the prose customs, the two workflows, the reserved-value discipline, the verification suite, the prime directives). Read it in full; it governs everything, including your chat replies.
2. `HANDOFFS.md` (top entries: the current state and the last stopping point).
3. `TODOS.md` (the backlog, the reserved-values queue, the stress-test forks, the candidate explorations).
4. `docs/working/PHYSICS_SUBSTRATE_FANOUT_PLAN.md` (your spec: the fan-out structure, the waves, the per-facet output schema, the gates).
5. `docs/working/PHYSICS_SUBSTRATE_GUIDE.md` (the locked representation in Section 4, and the open questions in Sections 5 and 6 that the fan-out answers).
6. `docs/working/BIOSPHERE_SUBSTRATE_SCOPED_PROPOSAL.md` (the worked wave-0 example, biology and composition, already proposed, red-teamed, and hardened; your facet outputs should take this shape).
7. `docs/design.md` Part 0 (the principles), and Parts 19, 40, 41, 58 (where the resolved substrate consolidates).

## 1. The mission

Execute the full physics-and-materials substrate fan-out: the one authored layer of the project (Principle 9), the reach-bounding artifact whose completeness sets the expressiveness ceiling of technology, value, meaning, and biology. Populate the catalogue (quantity axes and interaction laws) across the eight domains and the dependent consumers, in waves, surfacing every value reserved-with-basis for the owner to set, red-teaming each domain with the Steering Audit, and consolidating the resolved output into the design document. `PHYSICS_SUBSTRATE_FANOUT_PLAN.md` is the spec; the biology wave-0 proposal is the template.

This is the project's single largest research effort. The owner wants it done deep and wide ("full fat"), not scoped down. You have the hardware for it (section 5).

## 2. What is already done (do not redo)

- The deterministic engine core (`crates/core`): `Fixed` Q32.32, the `DrawKey`/`Phase` canonical keying, the counter-based RNG, `StableId` and the registry, the event log, the state hasher.
- The being model, end to end (`crates/sim`): the genome and two-tier `GenePool` evolution (Wright-Fisher drift, selection, speciation, Dobzhansky-Muller incompatibilities), the genome-to-mind cognition bridge, the value-distance metric, the axiom kernel (representation, enculturation, bounded-confidence schism, calcification, inheritance), affect, aging and mortality, perception, gossip, modelled dialogue, and the naming game with drift. A generational-turnover capstone test passes.
- The generated world map (`crates/world`): topology, terrain, biomes, a headless glyph view.
- Wave 0 of this fan-out (the biology-and-composition floor): proposed, red-teamed, and hardened in `BIOSPHERE_SUBSTRATE_SCOPED_PROPOSAL.md`, sign-off ready. Confirm the owner's sign-off on its reserved axes and laws, then consolidate it as your first worked consolidation.
- `main` carries all of the above.

## 3. The non-negotiables (from CLAUDE.md, internalize these)

- **Never fabricate a value.** Surface it reserved-with-basis. The substrate's whole point is that a fabricated physics constant is a steering leak wearing the costume of physics. The agent never sets a value; the owner does.
- **Determinism.** Fixed-point Q32.32, closed-form integer law kernels with interval bounds, content-addressed stable ids, no float in canonical state, order-independent reductions. A law that cannot be put in closed-form fixed-point with a bound is out of scope until it can be.
- **Emergence over templates (Principle 8, Principle 11).** The axes and laws are a bounded-but-extensible data floor. A closed enum where world content should emerge is a defect. Extending the floor with a new axis is a deliberate, audited Principle-9 act, never a silent one.
- **The Steering Audit (Principle 9).** Every domain's axes and laws must pass the red-team test that they do not encode a preferred cultural or biological outcome (the descriptor-space caveat: the search illuminates only the dimensions it is given). Run a red team against every green output.
- **Prose customs.** No em dashes ever; never the three banned adverbs (the -ly forms of genuine, honest, and actual, though those adjectives are fine); keep the negated-contrast construction (X-but-Y) to a minimum; prose over bullets for explanations. These apply to every maintained document and to your chat replies; the full rule is CLAUDE.md section 3.
- **Audit the input.** Verify a research report against the actual parts before consolidating it. The most valuable catches in this project came from auditing the input, not the output. The biology wave-0 red team caught three blocking defects in its own facet proposals; expect the same and run the red pass.
- **The verification suite** after every consolidation (CLAUDE.md section 8): em dashes 0, banned adverbs 0, parts gapless, code fences balanced, the backlog count moved by exactly the right amount, records sequential, no stale reference.

## 4. How to run the fan-out (the recipe)

- **Structure** (per the plan): green domain teams (one per catalogue domain) propose axes and laws reserved-with-basis; consumer-interface agents confirm each dependent system reads exactly what it needs and no fabricated field; red Steering-Audit teams attack each domain along steering, determinism, and reach.
- **Waves** (two horizons, floor first): wave 0 biology (done); wave 1 the mechanical-and-materials floor (Mechanics, Materials, Energy and thermal at Tier 0, the floor the Part 41 Stage-14 convergence proof selects against); wave 2 fluids, chemistry, optics; wave 3 and beyond the deepening discoverable tiers. The magic and exotic axes ride whichever wave their consumer needs, on the same floor, owner-reserved.
- **Per-facet output schema** (in the plan): each axis as `{ name, what it measures, unit, plausible fixed-point range with basis, tier, provenance (real-with-source plus citation, or fantasy-reserved-with-basis) }`; each law as `{ inputs (axis ids), the closed-form integer kernel over the existing Fixed ops, output measure, interval bound, tier, basis plus citation, every reserved constant with its basis }`; a determinism feasibility note; the flagged steering seams. The red team returns confirmed findings with the attack that produced each.
- **Orchestration.** Go wide. The owner's "full fat" request is explicit opt-in to multi-agent orchestration: use the Workflow tool (a deterministic fan-out, then a red-team verify stage, then synthesis) or large parallel Agent fan-outs. Scale the green and red counts up to the breadth of each domain, and loop-until-dry per domain (keep spawning finders until two consecutive rounds surface nothing new) so coverage is exhaustive. Do the determinism arithmetic in the red pass (the biology red team found a fatal Q32.32 overflow and underflow by computing it, not hand-waving).
- **Surfacing.** Each wave's proposal is a working doc (sibling to the biology proposal), reserved-with-basis, presented to the owner for sign-off before consolidation. Surface genuine forks in batch (select-confirm, non-final, per the owner's standing principle that nothing chosen now is final).
- **Consolidation** (after sign-off, CLAUDE.md workflow 5a): replace the flag with the mechanism in `docs/design.md` (Parts 41, 19, 40, 58), add the Decided-and-reserved blockquote, a Part 62 record, a Part 63 bibliography group, reconcile cross-references, update the audit log Section 1 block plus Section 2, the backlog bullet, the queue, and the counts, then run the verify suite.

## 5. Your environment

- WSL on Windows: Core Ultra 7 265K, RTX 5090 (32 GB VRAM), 192 GB DDR5; LLM brokers configured; Docker up; the box is shared with other projects.
- The value of the box for this mission is running a wide multi-agent fan-out and fast `cargo` builds. The RTX 5090 matters later for R-GPU-CANON-PIN (cross-vendor fixed-point determinism on the GPU backends), not for the research fan-out itself.
- Build and check: `cargo build --workspace`; `cargo test --workspace`; `cargo fmt --all`; `cargo clippy --workspace --all-targets -- -D warnings`; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`. The prose-and-structure suite is `scripts/verify.sh`.
- Git: work on a fresh branch off `main` (for example `claude/physics-substrate-fanout`) and open a pull request into `main` per the project's PR-per-effort pattern. End commit messages with the `Co-Authored-By` and `Claude-Session` footer the existing history uses. Do not put any model identifier in commits, PRs, or code.

## 6. First steps

1. Read the order in section 0.
2. Confirm with the owner the wave-0 biology reserved axes and laws sign-off (or note it pending), then consolidate wave 0 into `docs/design.md` as the first worked consolidation, so the fan-out has a landed precedent.
3. Scope wave 1 (the mechanical-and-materials floor): its green and red structure, its domains, its consumer interfaces. Surface any forks for the owner in batch.
4. Run wave 1 as the first big fan-out. Surface its reserved-with-basis proposal for sign-off. Then climb into the deepening tiers.
5. Update `HANDOFFS.md` and `TODOS.md` each session, run the verify suite, and present the changed files with a short summary.

## 7. The parallel engine track (context, pick up if directed)

Separate from the fan-out, there is buildable engine work that needs no reserved values: the Phase-0 wiring (wire the life-process loop, `age_step` plus `apply_mortality` plus births, into `World::tick` on a reserved cadence; promote the map-to-place bridge into the library as place-as-coordinate, a tile with a zoomable sub-tile of moving individuals, per Parts 6, 42, 54; harden the Part 16 `GrowthInput` and Part 17 `FoodSource` enums to data registries), and the substrate machinery (the `QuantityAxis` / `InteractionLaw` / `Substance` registry per the locked representation in Section 4 of the substrate guide, which the fan-out's axis and law outputs load into). The owner greenlit these to run alongside the fan-out. Pick them up if directed, or if the fan-out is owner-paced and you have idle capacity.
