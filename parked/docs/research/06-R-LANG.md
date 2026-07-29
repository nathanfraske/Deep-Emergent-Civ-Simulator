# R-LANG: Emergent Language & Meaning System — Design Report

## TL;DR
- **Adopt Option C (hybrid origination):** a thin, deterministic, fixed-point "naming-game/iterated-learning abstraction" runs ONCE at a culture's dawn to bootstrap the very first form–meaning anchors and sound system, then hands off permanently to seeded procedural generation + drift. Pure bootstrapping (B) is too expensive and hard to make bit-deterministic at civilization scale; pure seeding (A) throws away the scientifically-attested "order emerges from coordination" property the owner wants. C captures the emergence story cheaply and degrades gracefully to A.
- **The legibility guarantee is engine-side and deterministic, never LLM-dependent — confirming the owner's Decision 4.** Every Concept is a deterministic integer structure (a region/feature-bundle) over a fixed "etic substrate" grounded in the 65 Natural Semantic Metalanguage primes plus ~50 semantic molecules; the English gist gloss is a deterministic function of that structure. The LLM is a strictly downstream, non-authoritative view-time elaborator that can never backfeed into canonical state.
- **Concepts, phonology, lexicon, grammar, and writing are all emergent data over fixed audited Rust mechanism.** Language distance reuses the R-VALUE-METRIC structural-distance machinery (etic substrate + emic projections + incommensurability floor) as a three-component fixed-point metric feeding mutual intelligibility as transmission friction; mistranslation becomes an additional deterministic distortion hop into the Part 9 belief / Part 37 ToM systems; writing emerges in the technology layer but its linguistic content lives in the language system.

## Key Findings

**1. The science says structure emerges fast and cheaply from coordination pressure, which validates the emergent ethos — but the expensive part can be abstracted.** Kirby's iterated-learning model shows compositional structure emerges because languages must pass through a learning "bottleneck." Kirby, Cornish & Smith (2008, *PNAS* 105(31):10681–10686) showed human transmission chains converging an initially random language into a compositional one within a few generations — in one chain "tuge" came to label all horizontally-moving items while "poi" labelled all spiral-moving items. Steels' Talking Heads / naming-game work shows a population of agents self-organize a shared lexicon "from scratch" through purely local pairwise interactions, with sharp transitions to consensus (Baronchelli et al. 2006). de Boer (2000) showed realistic vowel systems emerge from imitation games under production/perception constraints with no innate phonology. Nicaraguan Sign Language is the real-world proof that a full language emerges de novo from no linguistic input: after Nicaragua founded a Managua special-education center in 1977 with ~50 deaf students (200+ by 1981, 400+ yearly through the 1980s, per Polich 2005), its core grammatical rules took about a decade to emerge (Senghas). The key engineering insight: these are convergence dynamics whose *outcome* (a shared, structured code) is what we need — and the outcome can be reached far more cheaply by a seeded generator than by simulating the full dynamic at runtime for every culture.

**2. There is a ready-made universal grounding substrate: the Natural Semantic Metalanguage (NSM).** Wierzbicka & Goddard's NSM posits exactly 65 semantic primes (the expansion stopped in 2014 when 65 was reached, per Wierzbicka 2021, *RJL* 25(2):317–342) — e.g., I, YOU, SOMEONE, SOMETHING, GOOD, BAD, DO, HAPPEN, BIG, SMALL, BECAUSE, NOT — that are lexicalized in (nearly) all languages and cannot be decomposed further, plus ~50 universal "semantic molecules," validated across ~30 languages. This is the natural candidate for the engine's deterministic "etic substrate" — the same role the shared etic substrate plays in R-VALUE-METRIC. Concepts become integer-representable feature bundles / regions over a space whose axes are grounded in primes and in world-features the engine already represents.

**3. Cross-linguistic semantic typology proves concepts must be emergent, and shows HOW they vary lawfully.** Berlin & Kay's color hierarchy and the World Color Survey show languages carve a shared perceptual space into different numbers of categories along a constrained sequence (black/white → +red → +green/yellow → +blue → +brown → ...). Levinson's spatial frames of reference (intrinsic/relative/absolute) show languages differ in which frame they grammaticalize, with cognitive consequences. Levinson & Meira's topological work shows a single universal similarity space variably subdivided, where languages "conflate into single lexical concepts only neighbouring spatial relations." This is exactly the model to adopt: a shared etic similarity space, with per-culture "emic" partitions that must respect neighborhood/convexity constraints.

**4. Language distance is a solved structural-distance problem.** The ASJP program (Holman, Wichmann et al.) computes language distances via normalized Levenshtein distance (LDN) over phonetic transcriptions of a fixed 40-item Swadesh subset; LDN relates inversely to mutual intelligibility. This maps cleanly onto integer/fixed-point edit distance over phoneme-index strings and onto the R-VALUE-METRIC machinery for the lexicon and grammar components.

**5. Sound change is regular and deterministic — historical linguistics hands us the drift engine.** The Neogrammarian hypothesis ("sound laws suffer no exceptions"; Osthoff & Brugmann 1878) states a sound change applies simultaneously to all words meeting its phonetic environment. This is literally a deterministic rewrite-rule pass over phoneme strings — exactly what Zompist's SCA²/Lexurgy sound-change appliers implement. The comparative method + family-tree model give us descent with regular correspondences; the wave model gives us contact/borrowing diffusion. Grammaticalization gives a unidirectional cline (content word → function word → clitic → affix; Hopper & Traugott 2003) for deterministic grammatical drift.

**6. Typology gives the plausibility constraints the generator must obey.** Per WALS Chapter 81A (Dryer 2013, n=1,377 languages): SOV=564 (~41%), SVO=488 (~35%), VSO=95 (~7%), VOS=25, OVS=11, OSV=4, no-dominant=189 — so the generator should sample dominant word order from a distribution dominated by SOV and SVO. Greenbergian/Dryer (1992) correlations show harmony: OV languages tend to be postpositional, VO prepositional (WALS Ch. 95: 43 VO-with-postpositions vs only 13 OV-with-prepositions — disharmony is rare). Maddieson (WALS Ch. 1, 562-language sample) gives consonant inventories ranging from a low of 6 (Rotokas, PNG) to a high of 122 (!Xóõ, Botswana), "the mean for the 562 languages being 22.7, the modal value 22 and the median 21"; vowel quality inventories bin small (2–4) / average (5–6) / large (7–14). These are the empirical priors for RESERVED owner-calibration sampling.

**7. Game precedent: gloss-with-gist works, hollow names fail.** Dwarf Fortress proves procedural per-culture languages + a translation layer (every generated word maps to an English root via T_WORD entries) are beloved — but DF's concept set is an authored ~2,196-root word list and DF is not bit-deterministic. Caves of Qud's "gibberish that is still readable" and named-thing-with-description approach works because there is always a gloss. Ultima Ratio Regum generates per-culture languages/scripts lazily and consistently (generate-on-demand, cache forever). The cautionary tale is the "hollow name" failure mode — No Man's Sky's procedurally recombined names felt meaningless because they were descriptors with no underlying simulated meaning ("you'll sigh at yet another world of giant dogs," GameCloud). The lesson: a generated word MUST point at a simulated concept grounded in world-state, or it is noise.

## Details

### R-LANG-CONCEPT — What a Concept is and how the concept space stays extensible

**Decision:** A `Concept` is a deterministic, integer-representable **region (feature-bundle with tolerances) over a shared etic feature substrate**. Concepts are emergent: cultures form them by drawing category boundaries over entities/events/relations/affordances/values the world already represents. The English gist gloss is a deterministic engine-side function of the concept's structure. This is the direct sibling of the R-VALUE-METRIC etic substrate + emic projection pattern.

**The etic substrate.** A fixed, audited set of feature axes — the only "innate" semantic content in the engine — built from three layers:
- **NSM-grounded primitive axes** (the 65 semantic primes + ~50 molecules as named, fixed axes: AGENT/PATIENT, DO/HAPPEN, GOOD/BAD, BIG/SMALL, LIVING, KIN, etc.). These are universal and never emergent; they are the grounding floor that prevents the "Chinese-Chinese dictionary" infinite regress (Harnad's symbol grounding problem — symbols must bottom out in non-symbolic categorical features, not other symbols).
- **World-feature axes** projected directly from existing engine state: physical properties (material, size, mass, sharpness, temperature), affordances (cuts, contains, burns), social/role features (Part 36 institutions/roles), value features (R-VALUE-METRIC axes), genome features (R-GENOME). A newly invented tool category or institution automatically exposes its feature signature on these axes, so a concept can form over it with no human in the loop.
- **Relation axes** (kinship dimensions, spatial frames after Levinson, taxonomic is-a/part-of).

**How a culture forms a new concept (deterministic).** A culture/pool maintains a discrimination structure (a Steels-style *discrimination tree*) over the etic substrate. When agents repeatedly need to distinguish a referent that current concepts fail to separate (a *discrimination game* failure — measured as referential ambiguity exceeding a RESERVED threshold over the culture's recent salient experience), the engine splits the tightest containing region along the etic axis that maximizes integer information gain (deterministic argmax with id-ordered tie-breaks). The new region becomes a candidate concept; if it earns enough usage (a RESERVED reinforcement threshold) under the fixed-point naming-game dynamic, it lexicalizes (gets a word — see R-LANG-GEN). This reproduces the attested emergence of color/spatial categories in agent populations (Steels & Belpaeme; de Boer) but as a cheap discrete dynamic.

**Representation (Rust-flavored):**
```rust
struct ConceptId(u64);            // stable, hashed from (culture_id, birth_event_id)
struct Concept {
    id: ConceptId,
    // Region over the etic substrate: per-axis interval in fixed point, or "don't care".
    constraints: Vec<AxisConstraint>,      // canonically sorted by AxisId
    prototype: Vec<Fixed>,                 // Rosch-style prototype point (centroid), per active axis
    parent: Option<ConceptId>,             // for split/merge lineage
    salience: Fixed,                       // usage-weighted, drives retention
    gloss_cache: GlossKey,                 // deterministic pointer into gloss machinery
}
struct AxisConstraint { axis: AxisId, lo: Fixed, hi: Fixed, weight: Fixed }
```
Prototype theory (Rosch) is honored by storing a prototype point; graded membership = fixed-point distance to prototype, thresholded.

**Drift, split, merge, cross-cultural difference.**
- *Split*: discrimination pressure divides a region (one culture lexicalizes "ice" vs "snow" where another has one word).
- *Merge*: two low-salience neighboring concepts whose words fall into disuse collapse (canonically: lower-id absorbs higher-id).
- *Drift*: the prototype point migrates as the culture's salient exemplars shift (semantic shift); migration is a deterministic fixed-point update over aggregated experience.
- *Cross-cultural difference*: because each culture runs its own discrimination structure over the SAME etic substrate, two cultures naturally end up with non-aligned partitions — the source of intrigue the owner wants. Comparison across cultures is done on the shared substrate (etic), with each culture's partition as its emic projection, exactly as R-VALUE-METRIC compares values.

**The deterministic gloss/legibility mechanism.** Every concept gets an English gist with no human and no LLM by composing fixed English lemmas attached to the etic axes (the axes are authored ONCE, in English, as part of the audited mechanism — this is the one legitimate hardcoding, justified because the substrate is finite, universal, and not "content"). The gloss algorithm is deterministic:
1. Find the nearest authored "anchor gloss" among the NSM primes/molecules and world-feature lemmas (nearest in fixed-point on the substrate).
2. Qualify it with the most distinctive constraining axes (the axes that most separate this concept from its parent/siblings), rendered via their authored English lemmas (e.g., region near molecule "tool" + axis CUTS-high + axis LONG → gloss "cutting tool, long" → "blade/sword-like tool").
3. Emit a bounded gist string (e.g., "a long sharp cutting-tool") plus a confidence tag.

This guarantees a legible gloss for ANY emergent concept, including ones formed over newly invented artifacts/institutions, because those expose authored etic axes. The gloss is the deterministic engine-side FACT.

**Where the LLM sits (confirming Decision 4).** The LLM receives the deterministic gloss + concept structure + culture flavor at *view time only* and may elaborate ("The Khorzu word *brenmar* refers to a long single-edged blade carried by river-clan duelists...") for player-facing color. Its output is never read back into canonical state, never used to compute distance, never used by any agent's reasoning. If the LLM is absent/offline, the deterministic gist still renders. **We confirm Decision 4 and strengthen it:** the LLM should additionally be sandboxed so its outputs are tagged non-canonical in the data model itself (a `ViewOnly<String>` newtype that no canonical-state function accepts), making backfeed a compile-time impossibility rather than a discipline.

### R-LANG-DISTANCE — Mutual intelligibility as a fixed-point structural distance

**Decision:** Reuse the R-VALUE-METRIC structural-distance machinery. Language distance is a weighted combination of three fixed-point sub-distances; mutual intelligibility is a decreasing function of it, consumed as transmission friction.

- **Phonological distance** `D_phon`: feature-based distance between phoneme inventories + phonotactic profiles. Computed as the structural distance between two inventories represented as sets over a shared phonetic feature substrate (place/manner/voicing for consonants; height/backness/rounding for vowels). This is the etic-substrate pattern applied to sounds.
- **Lexical distance** `D_lex`: the ASJP/LDN analogue. Over the shared Swadesh-style core-concept list (concepts both cultures possess via the etic substrate), compute normalized integer Levenshtein distance between the two cultures' word-forms (phoneme-index strings), averaged. Cognate detection falls out of low edit distance. Concepts one culture lacks contribute via an **incommensurability floor** (borrowed directly from R-VALUE-METRIC): unmatchable concepts add a fixed, capped distance rather than being silently dropped.
- **Grammatical distance** `D_gram`: Hamming-style distance over a typological parameter vector (word order, head-directionality, morphological type, alignment, etc. — the WALS-style parameters), weighted by salience for intelligibility.

**Combination:** `D_lang = w_p·D_phon + w_l·D_lex + w_g·D_gram`, all in Q32.32, with weights RESERVED for owner calibration (basis: lexical distance dominates real mutual-intelligibility studies, so `w_l` should be the largest; phonology second; grammar contributes less to first-contact intelligibility than to learning difficulty). Aggregation canonically ordered by concept-id; rounding fixed.

**Mutual intelligibility** `MI = clamp(FULL − D_lang)` (fixed point, monotone decreasing). MI feeds the belief-propagation system as a friction/throughput multiplier AND gates the mistranslation distortion (below). Dialects (low D) → near-full MI; separate families (high D) → MI at floor.

### R-LANG-LEARN — Per-individual multilingualism for promoted beings; aggregate for pools

**Decision: model it per-individual for promoted beings, aggregate for pools — mirroring the existing LOD split.** SLA research justifies a graded, integer-friendly proficiency model: proficiency is a continuous graded variable (not binary), there is an age-of-acquisition effect (Hartshorne, Tenenbaum & Pinker 2018, *Cognition* 177:263–277, dataset of 669,498 speakers: grammar-learning ability "is preserved almost to the crux of adulthood (17.4 years old) and then declines steadily" — a critical-period cutoff at ~17.4 years), and acquisition costs time/effort proportional to language distance. The literature broadly agrees ability "declines continuously with age" even where a sharp critical period is disputed (Bialystok & Hakuta).

- **Promoted being:** holds a small map `lang_id → Proficiency` where `Proficiency` is fixed-point [0,FULL]. Acquisition is a deterministic per-tick increment driven by exposure (contact frequency), aptitude (a per-being trait), an age-of-acquisition multiplier (decreasing step function — RESERVED breakpoint near ~17, basis: Hartshorne et al.), and inverse to `D_lang` from a known language (typological closeness speeds learning). A **translator/interpreter role** (Part 36 institutions) emerges where a being has high proficiency in ≥2 languages; interpreters reduce the mistranslation error term (below).
- **Pools:** carry an aggregate "languages-known distribution" histogram per culture, and intelligibility stays an aggregate friction at this tier, exactly as today. Promotion/demotion maps between the two representations deterministically (sampling the pool histogram by counter-RNG on promotion; folding the individual back into the histogram on demotion).

```rust
struct LangKnowledge {                 // promoted beings only
    native: LangId,
    others: Vec<(LangId, Fixed)>,       // proficiency, sorted by LangId
}
```

Consequences for diplomacy/intrigue: an interpreter who is himself low-proficiency or disloyal becomes a deliberate distortion/lie vector into Part 9 — a feature, not a bug.

### R-LANG-DISTORT — A language barrier distorts belief, not just slows it

**Decision:** Crossing a language barrier adds *error*, not only delay. The belief system already distorts per hop; mistranslation is an additional, gated distortion applied to the transmitted belief facet.

When a belief facet crosses a barrier (speaker lang A → hearer lang B), the engine computes an **error budget** `E = f(D_lang(A,B), interpreter_proficiency)` in fixed point (high distance + low proficiency → high E). `E` is applied deterministically to the facet via counter-based RNG keyed on `(master_seed, transmission_event_id, phase)`:
- *Concept-level mistranslation*: the conveyed concept is snapped to the nearest concept in the hearer's partition of the etic substrate — if B lacks A's distinction, nuance is lost (deterministic nearest-region). This is where Levinson-style partition mismatch produces realistic loss.
- *False-cognate error*: if a word-form in B is within edit-distance `E` of the heard form but maps to a different concept, the hearer may bind the wrong concept (deterministic threshold).
- *Nuance/attribute loss*: graded attributes on the facet are coarsened proportional to `E`.

This feeds straight into Part 9 (the distorted facet propagates and decays as any belief) and Part 37 (a being's theory of mind about another can now be corrupted specifically by a language barrier, enabling mistranslation-driven misunderstanding and intrigue). The Talk-of-the-Town belief-facet model (owner/subject/source + fallible memory) plugs in directly. Translation theory's "information loss across translation" is honored, but only to the depth needed: the mechanism is the deterministic nearest-region snap + edit-distance false-cognate test.

### R-LANG-WRITING — Writing/literacy emergence and placement

**Decision on placement:** Writing is invented in the **emergent-technology layer** (it is an invented function arising from need — record-keeping, accounting, religion), but the *script content and its effect on language* live in the **language system**. The two layers interface: the tech layer decides *whether/when* writing is invented and how literacy spreads as a skill/technology; the language system owns *what kind of script* emerges and *how it changes belief fidelity*.

**Mechanism.**
- *Invention*: triggered in the tech layer by accumulated need-pressure (trade volume, institutional record-keeping, large populations) crossing a RESERVED threshold — matching the historical "recording marks → writing" path (tallies/tokens → logographic accounting → fuller script) and the fact that writing was independently invented only a few times (most commonly recognized as four "pristine" inventions: Mesopotamia c.3400–3100 BC, Egypt c.3250 BC, China c.1200 BC, Mesoamerica by c.500 BC; Schmandt-Besserat counts three, treating Egyptian as derivative).
- *Script type emerges from the language's own structure, not authored*: the script's position on the logographic↔syllabic↔alphabetic continuum (treated as graded per Sproat/Rogers, not categorical) is a deterministic function of phonological complexity. Heuristic (RESERVED weights): large syllable inventory / complex phonotactics → logographic or syllabic pressure; small, simple syllable structure → syllabic/alphabetic pressure. This reproduces the typological reality without a forced trajectory assumption.
- *Literacy spread*: literacy is a per-being skill (promoted) / pool fraction (masses) that spreads as a technology/skill via the existing diffusion machinery, gated by institutions (scribal schools, religions).
- *Effect on belief (the payoff)*: written records change Part 9 dynamics. A written facet has (a) sharply reduced decay rate, (b) locked provenance (the record fixes the source and original content, resisting per-hop distortion), and (c) the ability to transmit across time without a living carrier. Oral transmission keeps today's higher decay/distortion. This operationalizes the orality-vs-literacy literature (Ong; Havelock; Goody) on writing transforming cultural memory — as concrete fixed-point modifiers on decay and distortion, RESERVED for calibration.

### R-LANG-GEN — Generation & drift internals, explicitly seeded and deterministic

All generation and drift are seeded from the per-culture seed via SplitMix64 counter-RNG over `(master_seed, culture_id, phase)`, exactly like worldgen. Bit-identical across machines/threads.

**Dawn generation (per culture, from seed):**
1. *Phonology*: sample a phoneme inventory size and composition from the WALS-grounded priors (consonants: mean ~22.7, median 21, modal 22, range 6–122; vowels binned 2–4 / 5–6 / 7–14). Inventory is assembled by selecting features over the phonetic feature substrate honoring implicational universals (e.g., if it has /g/ it tends to have /k/) and spacing (de Boer-style dispersion so vowels stay distinct). Sample phonotactics (syllable templates like CV, CVC, (C)(C)V(C)).
2. *Morphology*: sample morphological type (isolating/agglutinative/fusional/polysynthetic — drawn from a RESERVED distribution; NOTE this four-way scheme is the Sapir/Greenberg tradition, NOT a single WALS table, so its priors are owner-set, not WALS-quoted) and affixation tendency (WALS Ch. 26 prefix vs suffix).
3. *Grammar (GrammarParams)*: sample dominant word order from the WALS 81A distribution (SOV ~41%, SVO ~35%, VSO ~7%, remainder rare/none) then sample harmonic correlates (Dryer 1992): OV→postpositions/prenominal genitives, VO→prepositions/postnominal genitives, with a small RESERVED probability of disharmony (real languages are overwhelmingly harmonic — the 43:13 VO-postposition vs OV-preposition ratio in WALS Ch. 95).
4. *Lexicon*: for each concept the culture holds, generate a word-form by sampling syllables under the phonotactics (Zompist `Gen`-style), with form length/shape RESERVED. The bootstrapping dynamic (Option C, below) sets the very first ~core anchors; the generator fills the rest.

```rust
struct Phonology { consonants: Vec<PhonemeId>, vowels: Vec<PhonemeId>,
                   syllable_templates: Vec<SyllableShape>, /* sorted */ }
struct GrammarParams { word_order: WordOrder, head_dir: HeadDir,
                       morph_type: MorphType, alignment: Alignment, /* ... */ }
struct Word { form: Vec<PhonemeId>, concept: ConceptId }   // form + gloss-via-concept
struct Language {
    id: LangId, parent: Option<LangId>,
    phonology: Phonology, morphology: Morphology, grammar: GrammarParams,
    lexicon: Vec<Word>,                       // sorted by ConceptId
    sound_change_log: Vec<SoundChangeRule>,   // the drift history, for descent
}
struct CultureLangState {               // pool tier
    lang: LangId, drift_accumulator: Fixed,
    languages_known_hist: Histogram, mutual_intel: Vec<(LangId, Fixed)>,
}
```

**Drift operators (deterministic, generational):**
- *Regular sound change* (Neogrammarian): a sound-change rule `X → Y / context` is selected from a typologically-weighted catalogue and applied as a deterministic rewrite over EVERY lexicon form's phoneme string (Zompist/Lexurgy-style). Regularity is what makes descendants reconstructable and gives real cognate correspondences.
- *Lexical replacement & semantic shift*: words fall out of use (low salience) and are replaced; concept prototypes migrate (semantic shift handled in R-LANG-CONCEPT).
- *Grammaticalization*: content words drift along the unidirectional cline (content → function → clitic → affix; Hopper & Traugott 2003) under deterministic frequency thresholds, changing morphology over time.
- *Splitting into families*: when two pools of the same language fall below a contact threshold (geography/politics), they accumulate independent sound-change logs from divergent seeds → they become sister languages with a shared parent. The `parent` pointers + sound_change_logs reconstruct true family trees (tree model); contact adds wave-model borrowing.
- *Borrowing/loanwords*: on contact, high-salience foreign words for concepts the borrower lacks (or for prestige) enter the lexicon, adapted to the borrower's phonotactics (deterministic repair). This is the contact distortion that makes trees reticulate realistically.

This produces a DISTINCT but typologically plausible language per culture from seed, and drift produces real trees of descent.

### THE ORIGINATION DECISION — A/B/C with determinism & cost analysis

**Option A — Pure seeded generation.** The procedural generator emits the dawn language directly; iterated learning is never simulated.
- *Determinism*: trivially bit-deterministic (pure function of seed).
- *Cost*: cheapest — O(lexicon size) once per culture at worldgen.
- *Legibility*: full (gloss machinery is independent).
- *Weakness*: the dawn language is templated, not coordinated-into-existence. The owner explicitly wants "order emerges from agents trying to coordinate." A delivers the *look* of emergence (distinct languages) but not the *mechanism*.

**Option B — Full fixed-point bootstrapping.** Run a naming-game/iterated-learning dynamic at runtime to converge the entire lexicon and sound system from nothing.
- *Determinism*: achievable but fragile — requires every agent interaction, score update, and tie-break to be counter-RNG keyed and id-ordered; convergence-dependent loops need non-termination caps that must be bounded (hurting realism). String operations and hashing are deterministic (see below), so it CAN be made bit-identical, but the engineering surface is large.
- *Cost*: prohibitive at civilization scale — naming games need many interactions per concept per culture (Baronchelli's sharp transition still needs O(population) rounds), multiplied by thousands of concepts and many cultures. Cannot run per-individual for the masses.
- *Legibility*: preserved (concepts ground the gloss regardless).
- *Weakness*: cost and determinism-engineering burden; over-delivers on a property the player cannot directly observe anyway.

**Option C — Hybrid (RECOMMENDED).** A thin fixed-point naming-game dynamic runs ONCE per culture at dawn, over a SMALL set of bootstrap anchor concepts (the most salient core: self/other, key affordances, a starter sound system via a de Boer-style discrete dispersion game). It establishes the first form–meaning mappings and the seed phoneme inventory through genuine coordination, then hands the inventory + anchors to seeded generation (Option A machinery) to fill the full lexicon, and to the drift engine thereafter.
- *Determinism*: the bootstrap is a bounded, fixed-iteration dynamic (hard cap on rounds, counter-RNG keyed, id-ordered aggregation) → bit-deterministic by construction. Run at the culture/pool tier only.
- *Cost*: cheap and bounded — small anchor set × capped rounds × number of cultures, once at worldgen. No per-individual bootstrapping.
- *Legibility*: full.
- *Why it wins*: it is the only option that honors "order emerges from coordination" as a real (if abstracted) mechanism, while staying within the determinism and performance budgets. It matches the science (the Talking Heads outcome is what we instantiate; we just don't pay for the full continuous simulation) and matches the proven game pattern (DF/URR seed-then-vary) with an emergent front-end.

**Primary recommendation: Option C.** If the owner later wants to dial cost to zero, C degrades gracefully to A by setting bootstrap rounds to 0; if they want more emergence, the anchor set can grow. This tunability is itself a reason to pick C.

## Determinism Analysis

- **Fixed-point everywhere in canonical state.** All distances, proficiencies, saliences, prototype points, and thresholds are Q32.32 `Fixed`. No floating point touches canonical language state.
- **Counter-based RNG.** Every stochastic choice (phoneme selection, word-form sampling, sound-change selection, naming-game tie-breaks, mistranslation error draws) is keyed on a hash of `(master_seed, entity_or_culture_id, phase, event_id)` via SplitMix64 — reproducible regardless of evaluation order or thread count.
- **String handling is deterministic.** Phoneme strings are `Vec<PhonemeId>` (integer indices into the inventory), never platform strings, so edit distance, hashing, and sound-change rewrites are integer operations with fixed, defined results. Levenshtein is computed with integer costs and a canonical DP order. The English gloss lemmas are authored ASCII constants. Any Unicode rendering happens at view time only.
- **Canonical ordering.** All aggregations (lexicon, concept lists, distance sums) are ordered by id; rounding is fixed and specified.
- **Quarantine.** Exactly one thing is non-deterministic and it is fully quarantined: the LLM view-time elaboration, wrapped in a `ViewOnly<_>` type that no canonical function accepts. The deterministic gloss is the guarantee; the LLM is optional polish.
- **Limits surfaced.** Naming-game convergence must be hard-capped (a determinism necessity that slightly reduces realism); typological plausibility is enforced by sampling priors + implicational filters, which constrain but cannot guarantee a fully natural language; emergent-concept glosses degrade to coarse gists when a concept sits far from any anchor (legibility floor, not perfection).

## Game/Simulation Precedent — adopt/avoid

- **Dwarf Fortress** — *Adopt*: per-culture generated lexicon with a universal English-root translation layer (the legibility-via-gloss pattern); symbol/word categories with cultural preferences. *Avoid*: authored fixed concept list (~2,196 roots); not bit-deterministic; "the Lazy Sabre of Buttering" nonsense from ungrounded recombination.
- **Ultima Ratio Regum** — *Adopt*: lazy generate-on-demand + cache-forever for words/scripts (efficient and consistent); per-culture scripts tied to culture. *Avoid*: syllable selection from hand-authored arbitrary decisions rather than typology-grounded phonology.
- **Caves of Qud** — *Adopt*: "gibberish that is still readable," named-thing-with-gloss, history-as-biased-accounts (pairs with Part 9). *Avoid*: purely flavorful proc-gen text with no semantic hook (players skim it); ensure our generated language always points at simulated meaning.
- **Talk of the Town / Bad News (Ryan, Mateas, Samuel)** — *Adopt*: per-character belief facets with owner/subject/source, fallible memory, distortion on transmission — our R-LANG-DISTORT plugs directly into this model. *Avoid*: float-valued memory attributes (we use Fixed).
- **Conlang tools (Vulgarlang, Lexurgy, Zompist SCA²/`Gen`)** — *Adopt*: rule-based regular sound-change appliers as the literal model for deterministic drift; phonotactics-driven word generation; featural sound definitions. *Avoid*: their reliance on hand-authored rule sets per language — ours must sample rules from typological catalogues by seed.
- **No Man's Sky** — *Avoid (cautionary)*: the "hollow name" failure mode — procedurally recombined descriptors with no underlying simulated meaning feel repetitive and empty ("yet another world of giant dogs"). Our defense: every name maps to a concept grounded in real world-state and a deterministic gloss.

## Recommendations

1. **Build the etic substrate first** (65 NSM primes + ~50 molecules + world-feature projections + relation axes) as audited Rust with authored English lemmas. Everything else depends on it. Benchmark: a newly invented artifact category must auto-expose feature axes and receive a non-empty gist gloss with zero human input.
2. **Implement the Concept as a fixed-point region + discrimination tree**, with deterministic split/merge/drift. Benchmark: two cultures seeded differently produce demonstrably non-aligned partitions of the same domain (e.g., color/space), and cross-culture comparison runs through the etic substrate.
3. **Reuse R-VALUE-METRIC for D_lang** with the three components; calibrate weights so lexical distance dominates. Benchmark: dialect chains show graded MI; unrelated families sit at the incommensurability floor.
4. **Adopt Option C origination** with bootstrap rounds as a RESERVED knob (default small; 0 collapses to pure seeding). Benchmark: identical seeds → bit-identical dawn languages across machines/thread counts.
5. **Model L2 per-individual for promoted beings, aggregate for pools**, with an interpreter role feeding R-LANG-DISTORT. Benchmark: an interpreter's proficiency measurably changes belief-distortion error in a controlled scenario.
6. **Place writing invention in the tech layer, script/effects in the language system**, with literacy reducing belief decay and locking provenance. Benchmark: a literate culture's records survive a carrier's death with lower distortion than oral transmission.
7. **Enforce the LLM quarantine in the type system** (`ViewOnly<_>`), so backfeed is a compile error. Benchmark: removing the LLM entirely changes zero canonical state.

**Thresholds that would change these recommendations:** if profiling shows the Option-C bootstrap is still too costly at target civilization counts, drop to Option A (set rounds=0). If players report concepts feel incoherent, increase anchor-set size and tighten neighborhood/convexity constraints on partitions. If gloss quality is poor for far-from-anchor concepts, expand the authored lemma set on the etic axes (mechanism, not content).

## Settled vs Reserved for Owner Calibration

**Settled (design decisions):**
- Concept = deterministic integer region over a 65-prime NSM-grounded etic substrate; concepts emergent; gloss is a deterministic engine-side function.
- LLM is view-time only, type-quarantined; Decision 4 confirmed and strengthened.
- Language distance reuses R-VALUE-METRIC; three components (phonology/lexicon/grammar); MI = friction.
- L2 per-individual for promoted, aggregate for pools; interpreter role exists.
- Barriers add deterministic distortion (concept-snap + false-cognate + nuance loss) into Part 9 / Part 37, not just delay.
- Writing invented in tech layer; script content + fidelity effects in language system; written records reduce decay and lock provenance.
- Generation & drift are seeded/deterministic; sound change is regular (Neogrammarian rewrite); descent produces real family trees; borrowing on contact.
- Origination = Option C hybrid.

**Reserved for owner calibration (numeric — basis given, never fabricated):**
- All `D_lang` component weights `w_p, w_l, w_g` (basis: lexical should dominate per mutual-intelligibility empirics).
- Discrimination-game ambiguity threshold and naming-game reinforcement/retention thresholds (basis: tune to target concept-churn rate).
- Naming-game bootstrap round cap + anchor-set size (basis: performance budget vs emergence richness).
- Phoneme inventory sampling parameters (basis: WALS Ch. 1 — consonants mean 22.7 / median 21 / modal 22 / range 6–122; vowels 2–4 / 5–6 / 7–14).
- Word-order sampling distribution (basis: WALS 81A — SOV 564/~41%, SVO 488/~35%, VSO 95/~7%, others rare) and disharmony probability (basis: WALS Ch. 95 ~43:13 harmonic:disharmonic).
- Morphological-type distribution (basis: Sapir/Greenberg tradition — NOT a WALS table; owner-set priors).
- Word-form length/shape parameters.
- SLA acquisition rate, aptitude range, and age-of-acquisition breakpoint (basis: Hartshorne, Tenenbaum & Pinker 2018 place the cutoff at ~17.4 years; decline continuous with age thereafter).
- Mistranslation error-budget function constants.
- Writing-invention need-pressure threshold; script-type continuum weights; literacy spread rate; written-record decay/provenance modifiers.

## Caveats & Unsolved Pieces
- **Full de novo emergence is abstracted, not simulated.** Option C instantiates the *outcome* of naming-game/iterated-learning convergence cheaply; it does not run the full continuous dynamic. This is a deliberate, honest trade — the player cannot observe the difference, but a linguist could.
- **Typological plausibility is constrained, not guaranteed.** Sampling priors + implicational filters keep generated languages in the plausible region, but procedural generation can still emit a globally odd-but-locally-legal system; we cannot certify every generated language is natural.
- **Emergent-concept legibility has a floor.** Concepts far from any authored anchor on the etic substrate get coarse gists ("some kind of tool/relation"), not crisp glosses. Legibility is guaranteed; precision is not.
- **The morphological four-way typology is owner-prior, not WALS-sourced** — flagged so it is never mis-cited as a WALS distribution. (WALS distributes morphology across Chapters 20–22 and 26; it has no single four-way percentage table.)
- **The number of pristine writing inventions is itself contested** (three vs four, depending on whether Egyptian is treated as independent) — the engine's "few independent inventions" assumption is robust to either count.
- **The LLM's non-determinism bounds its trust.** It can never be more than flavor; any feature that would require its output to be canonical is out of scope by construction.
- **Sound-change/naming-game caps slightly dent realism** in exchange for guaranteed determinism — an accepted cost.
