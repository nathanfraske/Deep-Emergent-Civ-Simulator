# R-GENOME: A Deterministic Genome and Inheritance Model for an Emergent-Civilization Simulator

*Design-document research deliverable. Prose-and-structs, not code. All numeric constants are surfaced as RESERVED owner calibrations with their basis, never fabricated.*

## TL;DR
- **Adopt a data-driven, per-race "genetic scheme" descriptor that wraps one fixed set of audited Rust mechanisms**: a multi-locus quantitative-genetics spine (many small-effect loci summed to a breeding value) with an *optional* Mendelian diploid layer (explicit allele pairs with Falconer's −a / d / +a dominance) layered on per-gene, all expressed in Q32.32 fixed-point and driven by per-entity SplitMix64 counter RNG. The existing R-BEING-REP midparent trait rule is provably the additive/infinitesimal reduction of this fuller model, so it composes cleanly rather than being replaced.
- **Intelligence is resolved as a heritable, gene-affected quantitative phenotype ("cognitive capacity / reasoning acuity"), NOT a free-floating attribute and NOT a mere data-trait**: it is one more polygenic phenotype produced by the genotype-to-phenotype map, distinct from memory and belief plasticity (which remain their own race parameters and their own phenotypes). Intelligence gates cognitive events (technology conception) and sets perception/inference quality; memory governs belief deterioration; plasticity governs update rate. They may share loci (pleiotropy) without being the same axis.
- **Deep-time divergence, speciation, and hybridization are in scope and unify with the project's existing structural-distance pattern**: allele-frequency pools drift (Wright–Fisher) and respond to selection (breeder's equation R = h²S); genetic distance is computed as another instance of the configurable integer ground-metric distance (an F_ST / Nei analogue); reproductive compatibility is a fertility function of that distance *plus* a discrete Dobzhansky–Muller incompatibility table; speciation is *declared* when distance crosses a reserved threshold. Every rate and threshold is RESERVED for the owner.

## Key Findings

### 1. The genetic-realism decision is correctly served by a two-layer model with a per-race scheme selector
Quantitative genetics supplies the deep-time-correct spine. Fisher's 1918 infinitesimal model showed that a trait influenced by many loci of small additive effect produces a genetic component normally distributed about the midparent value; Barton, Etheridge & Véber (2017, *Theoretical Population Biology*, "The infinitesimal model: Definition, derivation, and implications") proved this limit holds rigorously under selection, drift, mutation, and population structure. This is exactly the behaviour needed for selection and drift to "behave correctly over deep time."

The Mendelian diploid layer — explicit allele pairs, dominance, recessive characters that hide in heterozygotes and resurface — is needed only where the owner wants discrete, hideable, resurfacing characters. It is therefore an *optional layer declared per gene*, not a universal requirement, satisfying the owner's "where it earns its keep" instruction.

The critical added constraint — different genetic schemes for different races — is met by making the *scheme* a data object that selects among fixed mechanism variants (diploid sexual / haploid / clonal / eusocial / magically-determined), defaulting to the standard diploid quantitative model for ordinary creatures. This mirrors exactly how the project already treats a race as a *selection over registries with per-race magnitudes*: the genome is a vector over the GeneDef set, and the scheme is a selection over mechanism variants. One fixed, audited, deterministic set of mechanisms; the genetic system itself is per-race data.

### 2. The genotype-to-phenotype math is fully expressible in integer/fixed-point form
Falconer's single-locus parameterization assigns genotypic values about the homozygote midpoint: A₁A₁ = +a, A₁A₂ = d, A₂A₂ = −a, where a and d are deviations from the midpoint and the dominance coefficient is d/a (0 = additive, 1 = complete dominance, >1 = overdominance). The average effect of allele substitution is α = a + d(q − p); additive genetic variance V_A = 2pq·α² and dominance variance V_D = (2pqd)². These are confirmed against Falconer & Mackay (1996, *Introduction to Quantitative Genetics*, p. 114) via Wellmann & Bennewitz (2015, *Heredity*) and corroborated across multiple independent sources (Vitezica/Varona/Legarra 2013, *Genetics*; Dekkers course notes; Iowa State *Quantitative Genetics for Plant Breeding*). Each is a sum of products of fixed-point quantities — no transcendental functions on the authoritative path. Epistasis is a bounded interaction lookup in the map; environment is an additive fixed-point offset.

### 3. Intelligence belongs in the genome as a polygenic phenotype, parallel to (not duplicating) memory and belief plasticity
General cognitive ability (g) is among the most heritable behavioural traits measured, and its genetic architecture is the textbook fit for the infinitesimal/quantitative spine rather than a single "intelligence gene":
- Haworth et al. (2010, *Molecular Psychiatry*), the largest such study (≈11,000 twin pairs across four countries), found heritability of general cognitive ability rises "linearly from 41% in childhood (9 years) to 55% in adolescence (12 years) and to 66% in young adulthood (17 years)"; the commonly-cited 50–80% adult range is attributed to Bouchard (2013).
- Plomin & von Stumm (2018, *Nature Reviews Genetics*, "The new genetics of intelligence") report that "the biggest hits have miniscule effects — less than 0.05% of the variance — which means that hundreds of thousands of SNP associations are needed to account for the 50% heritability estimated by twin studies." This is highly polygenic, small-effect architecture — precisely the infinitesimal regime.

Working-memory capacity is strongly correlated with fluid intelligence yet conceptually and empirically separable: per Conway, Kane & Engle (2003, *Trends in Cognitive Sciences*) the WMC–g correlation is r ≈ 0.59–0.65, and Shipstead, Harrison & Engle (2016) describe them as "strongly correlated traits…measuring complementary processes," with WMC explaining "at least half" of fluid-intelligence variance (Kane, Hambrick & Conway 2005). This empirical separability justifies the design: **reasoning acuity is its own polygenic phenotype** that gates cognitive events and sets perception/inference quality; **memory** (belief-deterioration rate) and **belief plasticity** (update rate) remain distinct phenotypes/parameters. Sharing some loci (pleiotropy) is permitted and realistic; collapsing them into one axis is not.

### 4. Game precedent gives a clear "adopt vs avoid" ledger
- **Dwarf Fortress**: one gene-pair per trait/colour, explicit dominant/recessive by list position (first-listed dominates), genotype saved per historical figure, attributes inheritable, off-map creatures carry random genotypes. *Adopt*: per-individual diploid genotype for promoted beings; dominance resolved by data order; hidden recessive carriers. *Avoid*: no bit-determinism guarantee; one-locus-per-trait is too coarse for deep-time quantitative drift.
- **RimWorld Biotech**: genes are data (germline / xenogene / archite), xenotypes are gene sets, a hybrid forms when parents' xenotypes differ, and a metabolic-efficiency budget constrains gene combinations. *Adopt*: genes-as-data registry; xenotype ≈ race-scheme; a "budget" concept for balancing reach. *Avoid*: the documented order-dependent inheritance bug where genes were "rolled in sequence" against a running metabolism cap, so success depended on gene ordering and inheritance was not conserved — our per-locus independent counter-RNG draws are specifically designed to eliminate this. Archite genes (non-inheritable special-cases) are a flavour mechanic, not an engine.
- **Crusader Kings 3**: congenital traits carry active/inactive states (inactive ≈ recessive carrier, invisible but heritable), leveled trait chains, and explicit inheritance probabilities — if both parents share a trait there is an 80% chance of inheritance, and inbreeding is computed from common ancestors four generations back. *Adopt*: hidden recessive carriers; inbreeding depression; probabilistic inheritance. *Avoid*: fixed high inheritance probabilities that make selective breeding feel deterministic and gamey, and the asymmetry where positive traits propagate far more easily than the "inbred" penalty.
- **Niche – a Genetics Survival Game**: explicit Mendelian dominant/recessive/co-dominant genes ("over 100 genes"), built on "the five pillars of population genetics (genetic drift, gene flow, mutation, natural selection, sexual selection)." *Adopt*: an honest, legible Mendelian layer plus the five population-genetics pillars as a proven model that players can reason about.
- **Spore**: the cautionary tale. Creature parts are visually infinite but carry canned, discrete attributes ("Stubbtoe gives Sprint 2, Dance 1, Speed 2 regardless of limb length or body shape"); biologists judged that "the step-by-step process by which Spore's creatures change does not have much to do with real evolution," and the editor's effect on gameplay was deliberately limited — "form without function." *Avoid*: decoupling visible form from functional consequence. Genetics must actually drive phenotype, or it is theatre.
- **Thrive**: auto-evo advances species statistically via a "population dynamics driven simulation with random mutations" when the entity is not under direct control. *Adopt*: this is precisely the project's LOD design — advance the masses as statistical pools, fully model only the promoted.
- **The Sims**: discrete inherited trait/occult flags (e.g., werewolf lineage that is "somewhat spotty," hybrids). Useful only as UI/flavour precedent, not as a genetic engine.
- **Songs of Syx**: races are data (`TRAITS`, `STATS`, food/climate/structure preferences in init files) selected at world setup, with no generational genetics. *Adopt*: the fully data-driven race-as-selection pattern, which the project already shares; *note*: it has no inheritance model to borrow, confirming the gap this design fills.

### 5. Deep-time divergence is another instance of the project's structural-distance pattern
Genetic distance between two pools is a fixed-point function of allele-frequency vectors. Wright's fixation index, F_ST = (H_T − H_S)/H_T = Var_among(p)/[p̄(1−p̄)] with heterozygosity H = 2p(1−p), and Nei's standard genetic distance are both pure rational arithmetic on frequency vectors. This is structurally the same shape as the project's existing per-race value-distance metric (a configurable structure compiled to integer ground-metric tables) and the planned language mutual-intelligibility distance.

**Recommendation: genetic distance IS another instance of the same structural-distance pattern for the *continuous* component** — reuse that machinery. But reproductive isolation additionally needs a *discrete* component that pure distance cannot capture: Dobzhansky–Muller incompatibilities, where alleles that are neutral in their own lineage interact lethally or sterilizingly in a hybrid (Bateson–Dobzhansky–Muller model; Coyne & Orr). Distance is necessary but not sufficient; a DMI table is the genuinely-different addition.

## Details

### 4.1 Determinism analysis (the top constraint)
Every operation on the authoritative path must be integer/fixed-point and counter-RNG driven.

- **Allele effects, dominance, epistasis, environment** are all sums and products of `Fixed` (Q32.32). Falconer's −a/d/+a scale, α = a + d(q−p), and the breeding-value contributions (A₁A₁ → 2qα, A₁A₂ → (q−p)α, A₂A₂ → −2pα) are rational arithmetic. Safe.
- **Counter-based RNG**: every stochastic draw — which allele segregates, whether a crossover fires, whether a mutation fires, the non-shared noise term — keys SplitMix64 on a hash of (master_seed, entity_id, phase, locus_index, event_ordinal). No sequential RNG state is threaded between entities or loci, so thread count and evaluation order cannot change results. This is precisely the structural fix for RimWorld's order-dependent inheritance bug: each locus's draw is independent of every other locus's draw.
- **Gaussian/normal draws** (needed for the infinitesimal segregation term and mutation step sizes) must NOT use floating-point `ln`/`sqrt`/Box–Muller on the authoritative path. Use a fixed-point inverse-CDF lookup table or an integer Irwin–Hall (sum-of-uniforms) approximation — both fully deterministic. *RESERVED implementation decision*: which integer Gaussian approximation and its table precision. This is the single highest-risk determinism detail.
- **Mapping functions** (Haldane r = ½(1 − e^(−2d)); Kosambi d = ¼·ln[(1+2r)/(1−2r)]) involve transcendentals. Resolution: do NOT evaluate them at runtime. Store the recombination fraction *directly* per adjacent-locus interval as a fixed-point constant (a "centimorgan" is just a reserved per-interval crossover probability), so crossover is one fixed-point comparison. Haldane/Kosambi are documentation-time tools for the *owner* to choose interval values, never runtime code.
- **Aggregation** at the pool tier is canonically ordered by id with fixed rounding, identical to the existing trait-pool advance.

Honest limit: true per-individual diploid simulation of an entire civilization is too expensive, which is why the LOD split (§4.7) is mandatory and the statistical tier necessarily *approximates* the individual tier. The approximation is unbiased in expectation (allele-frequency dynamics are the correct mean-field of the individual process) but loses higher-moment and linkage-disequilibrium structure between promotions. This is an accepted, documented divergence, not a determinism violation — both tiers are independently bit-deterministic.

### 4.2 Data model (Rust-flavoured, in the document's style)

```rust
/// One entry in the world's GeneDef registry. Mechanism is fixed; which genes
/// exist, what they reach, and all magnitudes are data.
struct GeneDef {
    id: GeneId,
    /// What phenotypic channel(s) this gene feeds. A gene may be pleiotropic.
    effects: Vec<GeneEffect>,
    /// Ploidy-relevant resolution for this gene under a diploid scheme.
    dominance: DominanceMode,
    /// Which linkage group this gene sits in, and its integer map position.
    linkage: LinkageSite,
    /// Per-gene mutation regime (rates RESERVED).
    mutation: MutationRegime,
    /// Optional: marks this gene as a Dobzhansky–Muller partner.
    dm_partners: Vec<GeneId>,
}

enum GeneEffect {
    /// Adds an additive contribution to a personality trait SETPOINT
    /// (the R-BEING-REP axis). Reactivity may also be gene-fed.
    TraitSetpoint { axis: TraitAxisId, weight: Fixed },
    /// Feeds the cognitive-capacity phenotype (reasoning acuity, etc.).
    Cognition { channel: CognitionChannel, weight: Fixed },
    /// Physical build channels.
    Build { channel: BuildChannel, weight: Fixed }, // size, strength, speed,
                                                     // climate tol., locomotion
    /// Imbued/magical traits (magic affinity, disease immunity, regen, nightvis).
    Imbued { channel: ImbuedChannel, weight: Fixed },
    /// Lifespan / reproduction.
    LifeHistory { channel: LifeHistoryChannel, weight: Fixed },
    // NOTE: ANATOMY (which body parts/fluids exist) is intentionally absent.
    // See §4.8 reserved interface.
}

enum CognitionChannel {
    ReasoningAcuity, // gates tech conception; sets perception/inference quality
    MemoryCapacity,  // governs belief deterioration (existing race param)
    BeliefPlasticity // governs belief update rate (existing race param)
}

/// Falconer scale held in fixed point. a, d are deviations about the
/// homozygote midpoint; d/a is the dominance coefficient.
struct DominanceMode {
    a: Fixed,            // half the difference between the two homozygotes
    d: Fixed,            // heterozygote deviation from midpoint
    kind: DominanceKind,
}
enum DominanceKind { Additive, Incomplete, Complete, Over, Co }

/// A diploid allele at one locus carries a small-effect value (the QG view)
/// AND optionally a discrete state (the Mendelian view).
struct Allele {
    additive_value: Fixed,        // contribution to the breeding value
    discrete_state: AlleleState,  // for Mendelian/hideable characters
    origin_tag: u32,              // for genetic-distance / DMI bookkeeping
}

/// Per-individual genotype: a vector over the gene set, diploid where the
/// scheme says so. Promoted beings carry this explicitly.
struct Genome {
    scheme: SchemeId,
    haplotypes: SmallVec<[Haplotype; 2]>, // 2 for diploid; 1 for haploid/clonal
}
struct Haplotype { alleles: Vec<Allele> } // indexed by GeneId order

/// The per-race genetic SCHEME descriptor — selects mechanism variants.
struct GeneticScheme {
    id: SchemeId,
    reproduction: ReproductionMode,
    ploidy: Ploidy,
    dominance_handling: DominanceKind,      // default for genes that don't override
    linkage_groups: Vec<LinkageGroup>,      // ordered loci + interval recomb fractions
    mutation_default: MutationRegime,
    isolation: IsolationParams,             // distance→fertility curve (RESERVED)
}

enum ReproductionMode {
    SexualDiploid,                          // the common default (Mendelian + QG)
    Haploid,
    Clonal,                                 // offspring = parent genome + mutation
    Eusocial { caste_rule: CasteRuleId },
    MagicallyDetermined { rule: MagicInheritanceRuleId }, // exotic escape hatch
}
```

### 4.3 The genotype-to-phenotype map and reconciliation with R-BEING-REP
For a phenotype channel P fed by loci L:

```
breeding_value(P)  = Σ_{l∈L} additive_contribution(l)          // additive
genotypic_value(P) = breeding_value(P)
                   + Σ_{l∈L} dominance_deviation(l)            // within-locus d
                   + epistasis_term(L)                          // bounded lookup
phenotype(P)       = genotypic_value(P) + environment(P)        // fixed-point offset
```

- **Additive contribution** at a diploid locus is the sum of the two alleles' `additive_value`s; the dominance deviation applies Falconer's d when the genotype is heterozygous.
- **Loci per trait** is a RESERVED per-channel count. Basis for choosing it: more loci → smoother, more Gaussian, more drift-stable distributions (the Fisher–Bulmer limit is "the limit as the number of loci contributing to variation in the trait increases," per Barton et al. 2017 — note that Dawson (1997, TPB) shows excessive linkage disequilibrium can prevent reaching that limit, and no specific "10–100 locus" sufficiency figure is established in the literature); fewer loci → more discrete, "major-gene" behaviour with visible single-gene jumps. Recommendation: many loci for continuous channels (build, cognition, trait setpoints), few for discrete imbued/Mendelian characters.
- **Reconciliation (the keystone result).** The R-BEING-REP rule
  `child_setpoint = h·midparent_genetic_value + (1−h)·population_mean + non_shared_noise + mutation_drift`
  is exactly the additive/infinitesimal reduction of this model. Under the infinitesimal model the offspring genetic value is normal about the midparent value with a within-family "segregation" variance that, with no inbreeding, equals half the base-population additive variance. Barton, Etheridge & Véber (2017) state this precisely: "if the variance in the parental population is V₁ … that of the offspring generation is V₁/2 + V₀: at equilibrium, V₁ = 2V₀; that is half the variance is between families, and half within them." Therefore:
  - `non_shared_noise` IS the segregation term — a counter-RNG Gaussian draw with variance V_A/2;
  - `h` IS narrow-sense heritability — the regression slope of offspring on midparent (Falconer & Mackay; de Villemereuil et al. 2013);
  - the regression toward `population_mean` is the standard shrinkage when h < 1;
  - `mutation_drift` is the per-generation mutational input.
  
  So when every gene feeding a channel is purely additive (d = 0, no epistasis), the full genome model *collapses to the existing equation*. The fuller model only adds structure — dominance, epistasis, linkage, explicit alleles — on top. **The existing R-BEING-REP rule is a clean special case, and the two compose without contradiction.**

### 4.4 Dominance, linkage, epistasis (deterministic integer form)
- **Dominance** is resolved per locus from `DominanceMode`. Complete / incomplete / co-dominance are all the one fixed-point expression `genotypic_value = midpoint + additive_part + (d if heterozygous)`. Co-dominance = both alleles' discrete states expressed (a flag). Recessive characters hide because a heterozygote's phenotype follows the dominant allele while the recessive allele persists in the haplotype — DF's hidden-carrier behaviour, but bit-deterministic and resurfacing on homozygous pairing.
- **Linkage & recombination**: loci are ordered within `LinkageGroup`s. Gamete formation walks the group; between adjacent loci a crossover fires iff `splitmix64(seed, parent_id, phase, group, interval_index) < recomb_fraction_fixed`. Recombination fractions are stored constants (the owner chooses them at design time using Haldane/Kosambi). Genes in separate groups assort independently. This yields true linkage disequilibrium and hitchhiking for free, deterministically.
- **Epistasis**: `epistasis_term(L)` is a bounded interaction lookup over the genotypes at interacting loci — a small data-defined table or a fixed-point bilinear form. It is the mechanism both for non-additive trait architecture and for Dobzhansky–Muller incompatibilities (§4.6). Bounded so cost is O(interacting pairs), never combinatorial.

### 4.5 Mutation (deterministic)
- **Point mutation**: per-locus, per-reproduction, fires iff a counter-RNG draw < per-locus rate (RESERVED). On firing it perturbs `additive_value` by a fixed-point step (step-size distribution RESERVED — basis: small steps → infinitesimal-like smooth drift; rare large steps → "major mutation" events) and/or flips `discrete_state`.
- **Structural mutation** (where a scheme opts in): duplication/deletion of a locus, or linkage-group rearrangement that changes recombination structure. Rare; this is the substrate by which schemes themselves can drift over very deep time.
- **Determinism**: every mutation is a pure function of (master_seed, parent entity_id, phase, locus, ordinal) — reproducible and machine-independent.
- **The clock**: under neutral theory the rate of substitution equals the per-individual neutral mutation rate, independent of population size (Kimura; "for any neutral genetic marker, the rate of substitution at the population level equals the rate of mutation at the individual level"). So the owner's chosen mutation rate directly sets the deep-time molecular-clock pace of divergence.

### 4.6 Deep-time divergence, speciation, hybridization
- **Pool-tier allele dynamics**: each statistical pool stores a fixed-point allele-frequency vector. Per generation: (a) selection shifts frequencies via the breeder's equation R = h²S (response = narrow-sense heritability × selection differential) for quantitative channels, and via genotype-fitness weighting for discrete loci; (b) drift perturbs frequencies by a Wright–Fisher sampling step parameterized by effective population size N_e (per-generation drift variance ≈ p(1−p)/(2N_e)), drawn via counter-RNG; (c) mutation adds input; (d) migration/gene flow mixes frequencies between pools.
- **Genetic distance**: between pools, a fixed-point F_ST analogue, F_ST = (H_T − H_S)/H_T with H = 2p(1−p), or a Nei-style distance — reusing the project's configurable structural-distance / integer ground-metric machinery (the continuous component).
- **Reproductive isolation = distance curve + DMI table**: a fertility/compatibility function maps genetic distance → probability a cross succeeds and → hybrid fertility (a smooth RESERVED curve). Layered on top is a discrete Dobzhansky–Muller incompatibility table: specific incompatible locus pairs/sets that, combined in a hybrid, cause inviability or sterility regardless of overall distance. Where a scheme has sex chromosomes, Haldane's rule ("when… one sex is absent, rare, or sterile in interspecific hybrids, that sex is the heterogametic sex") is expressible as a data-defined asymmetry: recessive DMI alleles linked to the sex chromosome are unmasked in the hemizygous sex.
- **Speciation is declared, not scripted**: when genetic distance between two diverged pools crosses a RESERVED threshold (and/or accumulated DMIs exceed a RESERVED count), the engine declares them distinct races/species with separate identities. This is "convergence without a target" — speciation emerges from drift + selection + isolation.
- **Hybridization outcome**: a cross between compatible races produces an offspring whose genome is the recombined union of parental haplotypes under the *child's resolved scheme* (a RESERVED rule for which parent's scheme governs, or a blend); hybrid phenotype follows the same genotype-to-phenotype map; hybrid fertility comes from the distance curve and DMI table. Sterile hybrids (mule-like) are the natural high-distance / DMI-triggered outcome.

### 4.7 LOD: statistical pools vs promoted individuals
- **Masses (statistical tier)**: a pool carries allele-frequency vectors and the trait-distribution aggregates R-BEING-REP already advances. Generations advance the frequencies (§4.6). No individual genotypes exist.
- **Promoted beings (individual tier)**: carry an explicit `Genome`.
- **Promotion (pool → individual)**: generate an explicit genotype by sampling each locus's alleles from the pool's allele frequencies (Hardy–Weinberg expected genotype frequencies p², 2pq, q² under the default sexual scheme), via counter-RNG keyed on the new entity's id. The individual is statistically consistent with its pool of origin.
- **Demotion (individual → pool)**: fold the individual's genotype back into the pool's allele-frequency counts (canonically ordered accumulation, fixed rounding).
- Honest limit: linkage disequilibrium and family structure built up among promoted individuals are lost on demotion — only marginal allele frequencies survive. Accepted, documented approximation.

### 4.8 Interfaces to existing systems (noted, not designed here)
- **Selective breeding / domestication**: already shifts a managed pool's trait distribution; under this model that is directional selection (breeder's equation) on the pool's allele frequencies toward docility/yield/strength. The genome model supplies the substrate; the domestication system supplies the selection differential.
- **Disease resistance as heritable selection**: a plague applies a selection differential favouring resistance alleles — the "survivors become more disease-resistant" hook is exactly Wright–Fisher selection on a resistance locus/channel.
- **Anatomy genetics (RESERVED)**: `GeneEffect` deliberately has no anatomy variant. The interface is a future `GeneEffect::Anatomy` that would feed the data-driven body-plan/anatomy system; its design is out of scope per the owner.
- **Imbued/magical traits & exotic inheritance**: `Imbued` channels inherit through the same map for biological schemes; `ReproductionMode::MagicallyDetermined { rule }` is the escape hatch for genuinely non-Mendelian magical inheritance (trait determined by ritual, lineage curse, or environmental attunement rather than allele segregation).
- **Personality setpoints (R-BEING-REP)**: fed by `GeneEffect::TraitSetpoint`; reactivity may also be gene-fed. The midparent rule is the additive reduction (§4.3).

## Recommendations

**Stage 1 — Lock the mechanism, defer the numbers.** Implement the fixed Rust mechanisms exactly as in §4.2–§4.5: `GeneDef` / `Genome` / `GeneticScheme`, the additive + dominance + epistasis + environment map, counter-RNG segregation/crossover/mutation, and the proof-carrying reduction to the R-BEING-REP equation (§4.3). Ship with the standard diploid sexual scheme as the only scheme. **Benchmark/gate**: a population evolved under zero selection must show drift variance matching p(1−p)/(2N_e) and must be bit-identical across thread counts and machines. If it is not bit-identical, halt — determinism is sacrosanct.

**Stage 2 — Add the statistical tier and LOD crossing.** Implement pool allele-frequency advance (§4.6) and promotion/demotion (§4.7). **Benchmark/gate**: promoting many individuals from a pool and demoting them must leave marginal allele frequencies unchanged in expectation (unbiased round-trip).

**Stage 3 — Add deep-time isolation.** Implement the genetic-distance metric (reusing the structural-distance machinery), the fertility curve, the DMI table, and speciation declaration (§4.6). **Benchmark/gate**: two pools split with zero gene flow must cross the speciation threshold on a timescale governed by the mutation rate and N_e, matching the molecular-clock expectation.

**Stage 4 — Add exotic schemes incrementally.** Clonal, then eusocial, then `MagicallyDetermined`. Each new scheme is data plus a small audited mechanism variant; **none may alter the standard scheme's behaviour** (regression-test the Stage 1 benchmark after each).

**Thresholds that would change the plan**: if profiling shows per-locus diploid simulation for promoted beings is too costly at the target promoted-entity count, reduce loci-per-channel (toward the major-gene end) *before* abandoning explicit genotypes. If the integer Gaussian approximation shows measurable bias in the drift/selection benchmarks, raise the lookup-table precision *before* changing the model. If players cannot perceive speciation events on a reasonable play timescale, raise mutation rate or lower the distance threshold *before* scripting speciation.

## Caveats and unsolved pieces
- **All numeric constants are RESERVED for owner calibration and are NOT fabricated here.** The reserved set, each with its basis given inline: per-axis narrow-sense heritability h (R-BEING-REP already centres it near one-half); loci-per-channel counts; allele effect-size scales (the a values); dominance coefficients (d/a per gene); recombination fractions per linkage interval; point- and structural-mutation rates; mutation step-size distribution; effective population size N_e per pool; the selection-differential scaling for domestication/disease; the genetic-distance → fertility curve; the speciation distance threshold and DMI-count threshold; the rule for which parent's scheme governs a hybrid; and the integer Gaussian approximation's table precision.
- **The statistical tier is lossy by design**: it tracks marginal allele frequencies, not linkage disequilibrium or family structure, so epistatic and LD-dependent phenomena are correctly modelled only among currently-promoted individuals. An accepted limit driven by civilization-scale cost.
- **Exotic/magical inheritance resists a single general mechanism**: `MagicallyDetermined` is deliberately an escape hatch — a named rule id dispatching to a bespoke audited function — because genuinely non-biological inheritance (ritual-determined, environmental, narrative-causal) cannot be expressed as allele segregation. The engine guarantees determinism and data-selection of *which* rule applies, but each exotic rule is its own small mechanism, not a parameterization of the standard one. This is the honest boundary of the "one set of mechanisms" goal.
- **Gaussian-on-the-authoritative-path** is the subtlest determinism risk; it must use the integer approximation, never hardware floats. The single highest-risk implementation detail.
- **The infinitesimal "Gaussian population" assumption is only weakly justified under strong selection.** Turelli (2017, *Theoretical Population Biology*, "Fisher's infinitesimal model: A story for the ages") clarifies that the *within-family* Gaussian-descendants approximation "provides a rigorous basis for understanding the consequences of selection, even when the Gaussian-population approximation becomes untenable" — i.e., offspring are Gaussian about the midparent, but the population-level breeding-value distribution need *not* stay Gaussian under strong directional selection. Acceptable for a game; should not be presented as exact population genetics, and benchmarks should not assume population normality under heavy selective pressure.

### Settled vs reserved — explicit split
**Settled (mechanism, audited Rust):** the two-layer QG-spine-plus-optional-Mendelian model; per-race `GeneticScheme` selecting fixed mechanism variants with diploid-sexual default; Falconer −a/d/+a dominance; ordered-locus linkage with stored recombination fractions; bounded epistasis lookup; counter-RNG segregation/crossover/mutation keyed per locus; the genotype-to-phenotype map; the proof that R-BEING-REP is its additive reduction; intelligence as a polygenic cognitive-capacity phenotype distinct from memory and plasticity; Wright–Fisher pool drift + breeder's-equation selection; genetic distance as a reuse of the structural-distance pattern plus a discrete DMI table; declared (not scripted) speciation; HWE-consistent promotion and frequency-folding demotion; the four interface stubs (domestication, disease, anatomy-reserved, magical).

**Reserved (data/numbers, owner-calibrated):** every constant listed in the first caveat bullet, plus the choice of integer Gaussian approximation and the choice of F_ST-vs-Nei distance form. The mechanisms are fixed and deterministic; the data — which genes exist, per-race schemes, magnitudes, and rates — are per-race and the owner's to set.
