# R-BIOSPHERE: Scoped Research Dive (for owner confirm)

Status: a scoping brief, not a resolution. It grounds the R-BIOSPHERE dive against the actual design parts and the built code, fixes what the dive must not re-resolve, names the two real gaps, and lays out the decision forks, the research sub-questions, the determinism and steering constraints, the coherence risks, the anticipated reserved values, the couplings, and the proposed sequence. Produced by a nine-scout grounding fan-out across the coupled systems and the code, synthesized, and red-teamed; the red team verified every build claim against the code (line numbers, axis counts, the reserved slots) and found no fabricated value, then surfaced the additions folded in here. Nothing here is decided; the forks are yours to confirm or redirect.

---

## 1. The shape of it

R-BIOSPHERE is the distinct reader that generates a world's species over the resolved biology-and-composition floor (R-PHYS-BIO). The orchestrator framing holds against the code: the three edibility, nutrition, and harm law kernels are built and twice-audited in `crates/physics/src/laws.rs`, the composition and consumer axes are data in `crates/physics/data/biology_floor.toml`, the Part 25 genome, pool, and speciation engine is built in `crates/sim/src/genome.rs`, and the zoomable map is built in `crates/world`. So the core edibility, nutrition, and toxicity mechanism is provided, and the dive must not re-resolve it. The medicinal-value, biphasic-hormesis, and preparation-lability refinements are the part of edibility that is reserved rather than built, so they are a gap below, not a re-resolution.

Two real gaps remain. The first is seeding the species set without hand-authoring: a generate-and-validate sampler over data-defined trait axes, drawn through seed-keyed counter-RNG, validated against food-web closure and biome-fit, then radiated by the genome engine over a deterministic pre-dawn epoch. The second is wiring organisms and consumers to the existing floor: a per-tissue composition `Substance` on each organism, a per-race physiology vector on each consumer (from drives, Part 20, and anatomy, Part 35), heritability of composition through a new genome channel, the medicinal, preparation, and dose dynamics the floor reserved, and the pre-dawn epoch driver itself.

---

## 2. What already exists (the dive must not re-resolve)

- RESOLVED and built: the three biology-floor law kernels in `crates/physics/src/laws.rs`: `net_nutrition` (Liebig minimum min-fold), `harm_class` and `net_harm` (per-(toxin-class, consumer) integer-Hill dose response, the saturation guard ordered before the power), and `edibility` returning the measured tuple `{net_nutrition, net_harm, margin}`. Race-blind, overflow-free, panic-free, order-independent.
- RESOLVED and built: the composition and consumer axes as data in `biology_floor.toml`: eight Tier-0 gross-composition axes, three Tier-1 terran toxin-class axes, and five consumer relation-kind axes. Ranges owner-set on 2026-06-30 except `bio.consumer.reference_tolerance`, still `AxisRange::Reserved` under R-UNITS-PIN.
- RESOLVED and built: the Part 58 substrate representation in `crates/physics/src/lib.rs` (`QuantityAxis`, `InteractionLaw`, `Substance` content-addressed by a 128-bit canonical-walk hash, `Provenance`, the cross-reference-validating registry loader).
- BUILT: the Part 25 deep-time engine R-BIOSPHERE radiates with, `crates/sim/src/genome.rs`: `GenePool` exact Wright-Fisher drift, directional select, interim frequency-distance, declared speciation, Hardy-Weinberg promote and demote, `GeneticScheme::reproduce`, and the Dobzhansky-Muller `IncompatibilityTable`. (Caveat in section 8: the cheap large-Ne path is reserved and unbuilt.)
- BUILT: the zoomable colour map, `crates/world` (the data-defined biome classification, the LOD `QuadTree`, the pure-read `Camera`).
- BUILT: the determinism primitives, `crates/core` (Q32.32 `Fixed`, counter `Rng`, the `DrawKey` schema and `Phase` registry, `StateHasher`).
- BUILT: the dawn convergence the pre-dawn epoch feeds into (`World::seed_dawn_populations`), the `Race` record, the two-tier promote/demote/merge/split conservation pattern, the calibration-manifest fail-loud reserved-value discipline, and the conservation registry.
- SPECIFIED (design only, the contract R-BIOSPHERE implements): Part 15 stocks and the logistic step, Part 16 `PlantSpecies`/`GrowthInput`, Part 17 `AnimalSpecies`/`FoodSource`/`TrophicLevel`, the generate-and-validate principle and the edibility-as-relation reframe (the vehicle guide), and the owner direction to run a deterministic pre-dawn genesis epoch at the aggregate pool tier.

No ecology code exists in `crates/` yet: no `Stock` type or logistic step, no `PlantSpecies`/`AnimalSpecies`, no food web, and no food-web-closure validator (confirmed by grep).

---

## 3. The two remaining gaps

**Gap 1, seeding the species set without hand-authoring.** A generate-and-validate sampler: sample a candidate as a point over data-defined trait axes via seed-keyed counter-RNG, validate niche, food-web closure, biome-fit, and fixed-point representability, and resample at the next ordinal on failure within a hard bound, then radiate the accepted founders by the genome engine over the pre-dawn epoch. Wholly R-BIOSPHERE's and untouched by the floor.

**Gap 2, wiring organisms and consumers to the floor.** Its parts: (2a) assign each generated organism a per-tissue composition `Substance` vector over the floor axes, each lump carrying a residual child for exact additivity; (2b) assemble a per-race physiology vector over the consumer relation-kind axes from drives (Part 20) and anatomy (Part 35), the designed relation-kind registry being broader than the built five axes and growing by demand-closure; (2c) add a composition heritability channel to the genome (the `Channel` enum has no composition variant today), so toxin and nutrient content can drift and respond to selection; the medicinal, preparation, and dose dynamics the floor reserved; and the pre-dawn epoch driver (a species/lineage registry, an environment-to-selection-coefficient kernel, speciation recording, founder-fork pool-sampling, and extinction).

---

## 4. The decision forks (yours to confirm)

Each fork lists its options, the dive's recommendation, and the basis. Group D forks have no safe default and are emergence-sensitive: they are the ones most in need of your judgment.

### 4.0 Confirmed decisions (owner, 2026-06-30)

- **F1 CLOSURE-DEPTH: hybrid.** Close topologically at seed time, then let Part 15 stock dynamics cull non-viable pools over early ticks and log the die-offs as history.
- **F14 TOXIN-FLOOR: emerge from chemistry.** No mandatory Tier-0 toxin baseline on every organism; defensive chemistry arises from selection in each world, so no dominant-plant-defense ecology is authored in (Principle 9 upheld).
- **F15 INTELLIGENCE-GATE: per-world data.** The mindless, plain-animal, and great-beast cut-points are calibration data each world profile sets, never hardcoded constants (Principle 11).
- **F16 AXIOM-SEED-RADIATION: seed fresh at the dawn for now.** The pre-dawn epoch radiates flora and fauna only; sentient races are seeded fresh at the dawn with their intrinsic beliefs set then. Full sentient co-evolution (heritable axiom seeds radiating per lineage through the reserved `AXIOM_INHERIT` phase) is recorded as a future candidate exploration, not in scope for this dive.
- The recommendation-bearing forks (F2, F3, F5, F6, F7, F8, F9, F10, F11, F12, F17) ride on the recommendations stated below unless the owner flags one on review of this doc.
- The two no-recommendation forks take a provisional, ideology-aligned default to be confirmed at dive time: **F4 DECOMPOSER** defaults to demand-closure driven (no mandatory decomposer pathway; a `Carrion`-backed dead-biomass `Stock` is added only where a generated web needs one); **F13 AGG-CONSUMER-PROMOTION** defaults to a reduced pool-tier summary that expands to the full consumer vector on promotion, sampled from `(pool.seed, slot)`, matching the R-TIER-CONSIST lifting and restriction operators.

### A. Closure semantics (define the validator's shape and cost)

- **F1 CLOSURE-DEPTH.** What counts as a closed food web: topological resolution only, steady-state energy and mass-balance feasibility, or a hybrid. Recommend the **hybrid** (topological closure at seed time, then let Part 15 stock dynamics cull non-viable pools over early ticks and log the die-offs as history). Basis: Part 15 collapse already culls over-drawn stocks and Part 17 extinction is permanent and logged, so the hybrid reuses existing machinery and turns infeasible founders into history rather than rejecting them.
- **F2 CLOSURE-TIMING.** Does closure validation run only at seed time, or as a standing post-evolution invariant. Recommend **seed-time only**. Basis: Part 17 makes extinction and its cascades a feature; continuous repair would erase the histories the project wants.
- **F3 CLOSURE-SPATIAL.** Does a real food source require spatial co-location, or only global presence reachable by migration. Recommend **per-region closure with cross-region draws allowed for migratory species**. Basis: migration is a flow between region pools, so a strict same-region rule denies a real mechanism while a pure global rule lets a region read as empty yet validly fed.
- **F4 DECOMPOSER.** Does every web require a decomposer pathway closing the dead-biomass loop, is the carrion-and-dead-biomass stock optional, or is it driven by demand-closure. No recommendation: the red team flagged that `Carrion` is only an enum variant today with no backing `Stock`, so this is an open owner call that sets whether decomposition is mandatory ecology or an emergent option.

### B. Sequencing and architecture

- **F5 SOURCE-VECTOR-SEQ.** Does R-SOURCE-VECTOR (lifting `GrowthInput`/`FoodSource` to a draw over the Part 58 axis registry) land before R-BIOSPHERE, or does the first proof stay matter-eating. Recommend **matter-eaters first, defer R-SOURCE-VECTOR**, with the validator still written to resolve a draw against pools and axes rather than hardcoding the enum cases, so the later lift is data not a rewrite. Basis: the floor's Tier-0 completeness criterion scopes the first proof to matter-eaters and licenses the deferral.
- **F6 EPOCH-PLACEMENT.** Where the pre-dawn epoch sits and its time model: a bounded pre-dawn worldgen pass on its own generation counter, folded into `World::tick` at a cadence, or two-phase. Recommend the **bounded pre-dawn worldgen pass**. Basis: your direction is a distinct pre-dawn genesis epoch at the aggregate tier so people arrive into a mature self-made ecology, and worldgen pass 3 already stubs ecology seeding before the dawn.
- **F7 SPECIES-REGISTRY.** What holds species, pools, and lineage: a dedicated species/biosphere module with a parent-pointer lineage tree, an extension of `Race` into a `Taxon`, or a side-table on the existing pool machinery. Recommend the **dedicated module with a lineage tree**, keeping `genome.rs` the pure mechanism and promoted organisms StableId-keyed (so the hecs-versus-grow entity-world fork stays open). Basis: the design names `SpeciesId` and treats pools as the canonical aggregate unit, and this parallels the existing language lineage-fork pattern.
- **F8 RESAMPLE-POLICY.** When generate-and-validate cannot close a niche within the resample bound: resample then deterministically fail and lower the diversity target, resample then relax the constraint and retry with a generalist, or resample then leave the niche empty. Recommend **resample to a hard bound, then a deterministic fallback** (best-scoring candidate or a logged empty niche). Basis: determinism requires a bounded reproducible retry; the fallback choice is a real owner fork that couples to the diversity-target reserved values.

### C. Where state lives (engine-interface decisions)

- **F9 CONSUMER-VECTOR-FORM.** Is the consumer physiology vector a stored heritable value seeded once from drives and anatomy, a live projection recomputed at law time, or a hybrid. Recommend the **hybrid** (a genesis derivation into a stored heritable vector, evolved thereafter). Basis: the floor calls the consumer half per-race heritable data while Parts 20 and 35 source it from drives and anatomy; the hybrid reconciles both and gives heritability and LOD a place to attach.
- **F10 COMPOSITION-HERITABILITY.** How heritable tissue composition is expressed, given the genome `Channel` enum has no composition variant: add an axis-keyed `Composition(AxisId)` channel, carry pool-tier frequencies only, or both. Recommend the **axis-keyed `Composition(AxisId)` channel** (mirroring the `TraitSetpoint(TraitId)` precedent: fixed mechanism, data membership). Basis: `genome.rs` declares new phenotype interfaces an engine extension never world data, and this is the minimal Principle-11-clean extension that backs the floor's heritability claim.
- **F11 TROPHIC-LEVEL.** Is `trophic_level` stored on the species or derived from the resolved `feeds_on` graph. Recommend **derived** (with at most a denormalized cache). Basis: the measure-not-store stance and the risk that a stored level disagrees with `feeds_on` after evolution.
- **F12 DOMESTICABLE.** Is domesticability a stored bool or a measured relation over heritable temperament axes and the Part 25 selection machinery. Recommend the **measured relation**. Basis: a bool is the authored-outcome pattern the project audits for; the domestication syndrome is meant to emerge from directional selection on docility.
- **F13 AGG-CONSUMER-PROMOTION.** How the consumer vector behaves under promotion and demotion: a reduced pool summary that expands on promotion, a full vector sampled on promotion, or a scalar derived cache, tied to the R-TIER-CONSIST lifting and restriction operators. No firm recommendation: the red team flagged this as an owner call that sets the conserved projection's shape at the pool tier.

### D. Emergence and steering gates (no safe default, the most in need of your judgment)

- **F14 TOXIN-FLOOR mandatoriness.** Is the coarse Tier-0 `toxin_load_coarse` floor mandatory on every generated organism, or driven by a generation-time analysis of the world's generated defensive chemistry. No recommendation: a mandatory floor would author a dominant-plant-defense ecology into every world, a Principle-9 and Steering-Audit concern; it should be driven from generated chemistry or stated as an explicit affordance, your call which.
- **F15 INTELLIGENCE-GATE.** Where the intelligence-dial cut-points live (the mindless / plain-animal / great-beast thresholds that decide which creatures carry a personality, a values-and-goals layer, and a belief store), and whether they are global or per-world data. No recommendation: this gate decides by setting which creatures carry minds, so it must be data (Principle 11) rather than hardcoded constants; the scope (global versus per-world) and the cut-point values are yours.
- **F16 AXIOM-SEED-RADIATION.** Does the pre-dawn epoch radiate sentient founder races (co-evolving a heritable axiom and intrinsic-belief seed per daughter lineage via the reserved `AXIOM_INHERIT` phase), or does it restrict to non-sentient life and seed sentient races fresh at the dawn. No recommendation: this sets whether the people arrive with a pre-dawn evolutionary and belief history or are seeded clean, a scope call only you should make.

### E. Harm routing (triggers a dependency rather than owning it)

- **F17 HARM-SINK.** Where dose-driven toxin harm (`net_harm`) writes, given Part 35 removed the health scalar and Part 22 is contagion-only: a unified affliction substrate shared with R-WOUND and Part 52, separate sibling tracks for disease and poison, or extending Part 22 with a non-communicable track. Recommend the **unified affliction substrate, settled with R-WOUND** (Part 52 already keys medical efficacy by `AfflictionKind` and R-WOUND asks for a data-defined damage-mode registry). Basis: this is a dependency R-BIOSPHERE triggers rather than owns, so the dive confirms the routing and defers the representation to the R-WOUND and Part 22 work.

---

## 5. Research sub-questions (answered during the dive, not now)

1. The trait-axis catalogue and per-axis fixed-point range a plant and an animal vary over, beyond the struct fields and the floor composition axes, grounded in ecology, nutrition, and toxicology, fantasy axes profile-gated.
2. What food-web closure requires (topological resolution, steady-state feasibility, or the hybrid), pinned by F1.
3. The biome-fit law form and cutoff, and how the Part 16 climate-envelope triplet (temperature, rainfall, soil) reconciles with the `terrain.rs` band axes (elevation, moisture, temperature).
4. The resample-versus-relax-versus-fail policy and the resample bound that guarantees deterministic termination.
5. How a drive's satisfaction source maps onto the per-nutrient-class requirement vector, and whether requirement couples to body size.
6. Whether digestive capability is derived from anatomy functions (Part 35) or is primitive per-race data (the digestion analogue of R-BUILD-PHYS).
7. How per-locus selection coefficients are derived from the environment so founders radiate into biome-specific ecotypes rather than drifting neutrally.
8. What records a speciation event as a distinct identity and what grows new Dobzhansky-Muller incompatibilities as lineages diverge.
9. The founder-effect pool-fork mechanism and the founder size relative to Ne.
10. The extinction triggers and event semantics at the pool tier, and the event-log schema.
11. The relational medicinal-credit form and the reserved biphasic hormesis curve shape.
12. The per-class preparation-lability matrix (cook, soak, ferment) and how it routes through technique (Part 23) given the open Inconsistency 5.
13. The seeding parameters: founder counts, diversity targets, resample bounds, and pre-dawn generation count.
14. (added) The intelligence-dial cut-points and their scope (global versus per-world), gating F15.
15. (added) The animal temperament-palette membership ({boldness, exploration, activity, sociability, aggressiveness}): data the species declares from, never a prose-fixed default.
16. (added) Whether the microbiome contribution to fermentation and detox folds into the consumer fermentation and detox axes or is an evolving symbiont pool with its own conserved projection.
17. (added) The genetic-distance measure (fixation index versus Nei) and whether multi-allele pools are in scope, since the built `distance()` is an explicit interim biallelic placeholder the speciation declarations key off.
18. (added) Whether `PlantSpecies` gains a biomes list and a foundation/keystone field, an asymmetry with `AnimalSpecies` that otherwise blocks the cascade coupling.

---

## 6. Determinism constraints (hard, non-negotiable)

The whole epoch is keyed on the world seed through counter-RNG, so the biosphere is part of the world's reproducible identity. Every new draw site (species sampling, closure resample, radiation and speciation events, founder-fork sampling, migration mixing, on-demand promotion) registers its own `Phase` id and slot under the R-RNG-COORD rule so concurrent draws cannot collide on counter zero; the Phase numbers are engine mechanics, reserved not fabricated. Generate-and-validate resamples at a fixed ordinal with a hard bound and a deterministic fallback. No float enters canonical state: every composition value and law output is Q32.32, and a candidate not representable in fixed-point is a failed candidate. Law kernels and any new aggregation stay overflow-free, panic-free, and order-independent. The `Option<Fixed>` sentinels are load-bearing (requirement `None` is not-required, distinct from zero; tolerance `None` is not-applicable, distinct from maximal sensitivity), so a fabricated zero is a defect. On-demand promotion regenerates the organism from `(pool.seed, slot)` so re-promotion reproduces it, and the camera never drives canonical fidelity. The ecology two-tier subsystem registers an exact-integer conserved projection (the additive composition mass only) with remainder to the lowest id, never a mean, and a nonlinear law output is left tier-resolution-dependent by construction (honest cross-tier non-invariance under R-TIER-CONSIST). Generated organisms are content-addressed by a canonical-walk hash so identical organisms dedup identically on every machine. `bio.consumer.reference_tolerance` reads fail-loud while reserved.

---

## 7. Data-driven and steering constraints (Principles 8, 9, 11)

A species is a sampled vector over authored trait axes, never a hand-authored entry (hand-authoring is reserved for the memorable). Which axes exist is set by demand-closure, not by enumerating biology. Edibility, nutrition, toxicity, and medicine are measured relational consequences, never stored verdict flags: R-BIOSPHERE adds no edible flag to either species struct, and any food/poison/medicine label is a read-time band over the tuple. Composition axes are heritable, so defensive toxins and the domestication syndrome fall out of selection. Consumer physiology is per-race heritable data and an extensible relation-kind registry with a stated closure test, never a closed enum; the gain-versus-danger risk valuation lives in the agent layer (Parts 8, 20), not the substrate. The biosphere is bounded by the world profile: exotic axes are `FantasyReserved` and gated on the profile carrying the magic axis. Every reserved axis range and seeding parameter flows through the fail-loud calibration manifest. The closed lists in the path of world content (`GrowthInput`, `FoodSource`, `Succession`, `FireResponse`, `EngineerEffect`, `TrophicLevel`, the `domesticable` bool, the `ClimateEnvelope` triplet, and the animal temperament palette) are defects to harden to data-defined registries, source vectors, or measured readings, argued at each site.

---

## 8. Coherence risks (flagged, to bound the dive)

- The pre-dawn epoch declares speciation and grows the lineage tree off `genome.rs` `distance()`, an explicit interim biallelic placeholder (fixation-index-versus-Nei and multi-allele reserved). The radiated species set is therefore a pure function of a placeholder metric and is not stable across the eventual measure change, so the epoch's reproducible identity is bounded by a value still to be set. The dive must treat the distance measure as a coupling that bounds the epoch, and decide whether to settle the measure before running a long epoch.
- The cheap deep-time path is reserved and unbuilt: the large-Ne Wright-Fisher Gaussian approximation and the integer-Gaussian inverse-CDF are reserved, and the built drift is the exact O(Ne) Bernoulli sum. A deep epoch at realistic Ne is therefore not cheaply runnable as-is, so `biosphere.predawn_generations` silently inherits a performance ceiling: a naive realistic-Ne setting would not terminate within budget. The dive must state this ceiling and decide whether the cheap path is a prerequisite for a long epoch.

---

## 9. Anticipated reserved values (surfaced, never set here)

Founder species counts (per world and per biome; basis: the founder-to-radiated ratio in real adaptive radiations and the profile's biome area and productivity). Diversity targets (basis: profile-derived richness, framed as profile-derived rather than a terran richness target, to avoid steering the ecology's shape). Resample bounds (basis: the smallest cap that clears valid niches while guaranteeing termination, a performance bound). Pre-dawn epoch generation count and drift-selection schedule (basis: the span for founders to radiate into mature ecotypes without collapsing onto the selection optimum, bounded by the section-8 performance ceiling). The trait-axis catalogue and ranges (basis: real ecology, nutrition, and toxicology envelopes at or just above the documented maximum; fantasy axes profile-gated). Niche and closure thresholds and the biome-fit cutoff (basis: steady-state feasibility from carrying capacity and climate-tolerance literature). Per-race consumer physiology vectors and their founding variance (basis: the published figure for an anchoring taxon scaled allometrically, never a human default). `bio.consumer.reference_tolerance` range and per-toxin-class scale (basis: the published tolerance for an anchoring taxon, per class because the envelope exceeds one Q32.32 scale, R-UNITS-PIN). The Hill exponent, saturation ratio, harm cap, and margin cap (basis: the observed sigmoidicity, the saturation ratio the min of the within-one-epsilon ratio and the overflow-safe power ceiling). Allometric scaling exponents (basis: comparative-physiology literature, an authored affordance or per-race datum). The medicinal-credit form and hormesis curve (basis: the consumer's deficiency state attenuated by harm, the biphasic shape a reserved modelling choice). The relocated risk weights and founding prior (basis: the consumer's heritable risk tolerance, surfaced so a 1:1 default does not reinstate an authored attitude). The tolerance-zero direction (basis: the not-applicable-versus-infinite-sensitivity reading, ratified so missing data is not biased toward edible). The preparation-lability matrix (basis: the class-by-operation detoxification literature). The drive-to-requirement magnitudes (basis: per-race data, mirroring the `AppraisalBinding` precedent). The genome radiation parameters (Ne, selection scaling, mutation rate, recombination default, speciation thresholds, and the new environment-to-coefficient kernel scale; basis: population-genetics literature, already reserved slots). Exotic axis ranges (basis: fantasy-reserved by analogy to the closest real class, profile-gated).

---

## 10. Couplings

Part 15 stocks is the shared abstraction, Parts 16 and 17 its instances, so closure spans both resolutions. Part 16 flora couples to Part 17 fauna (pollinators, dispersers), so a pollinator collapse dooms a plant a generation later. R-BIOSPHERE reads its axes and laws from R-PHYS-BIO. Part 25 (`genome.rs`) radiates the founders; composition heritability needs a new channel there. Worldgen and the biome substrate (`crates/world`) supply the biomes the validator fits to. Parts 20 and 35 supply the consumer physiology; Parts 8 and 20 host the relocated risk valuation. Part 22 is the harm sink (currently contagion-only), Part 35 carries condition, Part 52 treats afflictions, all through a unified `AfflictionKind` registry shared with R-WOUND. Parts 23 and 9 carry the emergent fallible dietary knowledge and preparation-as-technique (on Inconsistency 5). Part 19 makes a tissue both a composition vector and a tradeable material. Parts 54 and 58 require the conserved projection. The two carried-open floor seams bound R-BIOSPHERE: R-SOURCE-VECTOR and R-BIO-REGISTRY. R-VIEW-ELAB and Principle 10: on-demand promotion must not alter canonical pool history.

---

## 11. Proposed resolution sequence

Ground the dive in the actual parts and code. Settle the closure forks first (F1 to F4), since they define the validator before any code or axis catalogue. Confirm the sequencing forks (F5 to F8) so the build target is fixed. Run the trait-axis catalogue research as the dive's own fan-out with a red team, every axis reserved-with-basis. Harden the closed lists at the moment the flora and fauna structs are first written. Resolve the wiring forks (F9 to F13) and the steering gates (F14 to F16). Specify the pre-dawn epoch driver on registered Phases. Resolve the medicinal, preparation, and dose dynamics as a relational refinement, routing the harm sink (F17) through the unified affliction substrate. Register the ecology conserved projection and prove cross-tier non-invariance is honest and promotion is observer-independent. Consolidate per the workflow (mechanism, blockquote, Part 62 record, Part 63 bibliography, audit log, counts, verification suite). Surface the remaining open siblings (R-SOURCE-VECTOR, R-BIO-REGISTRY, R-WOUND, Inconsistency 5, R-VIEW-ELAB) as seams R-BIOSPHERE reads around rather than resolves.

---

## 12. Red-team targets (for the dive's own adversarial pass)

Coherent closed food webs under the chosen closure depth. Edibility staying relational (no stored flag, the same organism food-poison-medicine across three consumer vectors, no risk attitude reinstated). Deterministic bounded diversification (a pure function of the seed across machines and thread counts, bounded resample, non-colliding Phases, replay-from-`(pool.seed, slot)` promotion, no float in canon). Terran-genesis circularity (registry membership upstream-reserved pending R-BIO-REGISTRY). Cross-tier honesty (only additive composition mass conserved, nonlinear outputs left tier-dependent, LD loss documented). Closed-list creep (no new authored enum where content should emerge; each engine-interface extension a deliberate audited act). Steering at the consumers, not the floor (toxin harm and dietary knowledge map no stance or trait to a named affliction, recipe, or institution).
