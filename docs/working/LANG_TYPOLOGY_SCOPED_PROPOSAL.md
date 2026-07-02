# The Typological Substrate: Scoped Proposal (R-LANG-TYPOLOGY)

Working scoping for R-LANG-TYPOLOGY, the hardening of the grammar typological parameters of the Part 33.4 generation model from a closed set of Rust enums into a data-defined typological substrate, sibling to the value substrate (Part 21), the semantic substrate (33.1), the institution-function substrate (Part 36), the access-channel registry (Part 40), and the modality substrate resolved for R-LANG-MODALITY (record 62.13). This is a scoping surfaced for the owner's sign-off, not a consolidation. Nothing here is edited into the design document until the owner signs off on the substrate shape and the reserved values; the mechanism is proposed, the values are reserved with their basis, and the open decisions are surfaced in batch.

## The locked mechanism it builds on

Part 33.4 emits a typologically plausible language from a culture's seed. The current representation is four closed enums bundled in one struct:

```rust
pub struct GrammarParams { pub word_order: WordOrder, pub head_dir: HeadDir,
                           pub morph_type: MorphType, pub alignment: Alignment }
```

The generator samples a dominant word order from the cross-linguistic distribution and then its harmonic correlates (an object-before-verb language tends to postpositions, a verb-before-object language to prepositions), with a small reserved probability of disharmony. The grammatical component of language distance (33.5) is a distance over this typological parameter vector. Drift (33.4) can move a culture's morphology over time through the grammaticalisation cline. All of this is seeded and deterministic per 33.10: integer and fixed-point, every draw keyed on the seed, the culture id, the phase, and an event ordinal, over sorted structures rather than hashed maps.

The determinism floor is already favourable, exactly as it was for R-LANG-MODALITY: the sampling distributions ("the word-order and morphological-type sampling distributions and the disharmony probability, basis in word-order and harmony typology") are already reserved in the 33.4 blockquote. This item hardens the representation the distributions sit over, so it adds data and reopens no 33.4 determinism rule.

## The seams (auditing the input, not the flag alone)

The flag names the surface seam: four closed enums where the discipline is a data registry. Auditing the mechanism against the sibling substrates surfaces four seams the flag implies but does not spell out, and the proposal hardens each.

1. **The parameter set is itself closed.** `GrammarParams` as a fixed four-field struct is a closed list one level up from the enum values: a world cannot add a typological dimension the engine's authors never enumerated (an adposition-order parameter, a classifier system, an evidential-ordering parameter, an exotic parameter a non-human cognition grammaticalises). The value substrate opens both the axes and their ranges; the typology substrate must open both the parameters and their values. The grounding survey sharpens this: the real typological space is broad (the World Atlas of Language Structures carries dominant word order, the order of adposition, genitive, adjective, demonstrative, numeral, relative clause, and degree word, plus tone, syllable structure, case, and marking locus, each its own parameter), and a fixed four-field struct cannot hold it. An open registry can.

   Two of the four current fields are themselves closed conflations the survey flags. **Morphological type** (isolating, agglutinating, fusional, polysynthetic) is, in modern typology, three independent axes rather than one enum: synthesis (morphemes per word, WALS 22A), fusion (how separable a formative is, WALS 20A), and exponence (how many categories cumulate in one formative, WALS 21A), with the four traditional labels recovered as regions of that space. Holding the three axes rather than the four-way label is the more faithful and more extensible choice. **Alignment** is not one global setting either: a split-ergative language aligns nouns one way and pronouns or verbs another, so WALS tracks case-marking-of-noun-phrases (98A), case-marking-of-pronouns (99A), and verbal-person-marking (100A) as separate parameters. The substrate should hold alignment per locus, which the open parameter registry does for free.

2. **The harmonic correlations are hardcoded control flow.** "An OV language tends to postpositions" is, in the current mechanism, a Rust branch. That is a closed correlation model authored one level down. The flag's load-bearing phrase, "the parameter values with their harmonic correlations data," requires the correlations themselves to be a data table of directional biases between parameter-values (the Greenbergian and Dryer word-order correlations as data), so a world can carry its own harmonic tendencies and a parameter added to the registry can declare its correlations with existing ones in data.

3. **The sampling distribution is not per-race differentiable.** The generator samples from one cross-linguistic (human) distribution. The owner's standing condition is per-race differentiability: a race's typological priors should be data, defaulting to the cross-linguistic distribution but able to differ, so a race with a different cognition or a non-vocal channel (already first-class after R-LANG-MODALITY) can carry a different shape over the same descriptive space, never a "better" one.

4. **The sampling order matters and must be pinned.** Harmonic correlates are directional: the sampler draws an anchor parameter first, then conditions the dependents on it. The order the parameters are drawn in is therefore load-bearing for determinism, and cannot be a struct-field order or a hash-map walk. The substrate must carry a canonical sampling order over the parameter registry (an anchor-first priority, itself data, grounded in the branching-direction anchor of the typological record), so the seeded draw is a function of the data rather than of iteration order.

## The proposed substrate (mechanism fixed Rust, membership data)

The shape mirrors the modality substrate of record 62.13 (production modalities, reception senses, feature dimensions with their value sets, media) and the phonological-feature registry already in `language.rs` (`FeatureDimDef`, `FeatureValueDef`).

**A. The typological parameter registry.** A parameter is a registered dimension of typological variation; its values are a registered set. Both grow with the world.

```rust
pub struct TypologyParameterId(pub u32);
pub struct TypologyValueId(pub u32);

pub struct TypologyValueDef {
    pub id: TypologyValueId,
    pub description: String,   // "verb-object", "postpositional", "ergative-absolutive"
}

pub struct TypologyParameterDef {
    pub id: TypologyParameterId,
    pub description: String,    // "dominant word order", "adposition order", "alignment"
    pub values: Vec<TypologyValueDef>,   // canonically sorted by id
    pub sample_priority: u32,   // the anchor-first sampling order (data); lower draws earlier
}

pub struct TypologyRegistry { pub params: Vec<TypologyParameterDef> }  // sorted by id
```

**B. The typology profile.** A culture's grammar is a canonical vector over the registry, replacing `GrammarParams`.

```rust
pub struct TypologyProfile { pub values: Vec<(TypologyParameterId, TypologyValueId)> }  // sorted by param id
```

`Language.grammar: GrammarParams` becomes `Language.grammar: TypologyProfile`. Every consumer that read `grammar.word_order` now reads the profile's value for the word-order parameter id.

**C. The harmony model as data.** The correlations are a table of directional biases between a conditioning parameter-value and a conditioned parameter-value, read by the sampler. This is the Greenberg and Dryer correlation structure as data, not Rust branches.

```rust
pub struct HarmonyBias {
    pub given_param: TypologyParameterId, pub given_value: TypologyValueId,
    pub then_param: TypologyParameterId,  pub then_value: TypologyValueId,
    pub weight: Fixed,   // the strength of the correlation, reserved with basis
}
pub struct HarmonyModel { pub biases: Vec<HarmonyBias> }  // canonically sorted
```

**D. The seeded sampler (mechanism fixed).** Walk the parameters in canonical sampling order (`sample_priority`, then id, as the total tiebreak). For each parameter, form its per-culture draw distribution as its per-race prior tilted by the harmony biases whose `given` was already sampled this pass, then draw one value under the 33.4 draw key (seed, culture id, phase, an event ordinal that is the parameter's canonical position). A reserved disharmony probability keeps the tilt from being absolute, so a rare disharmonic language can emerge. The pass is a pure function of the seed, the culture, and the data, so it replays bit for bit and is independent of thread count, exactly as 33.10 requires.

**E. The per-race prior.** A race carries, as data, a prior distribution over each parameter's values, defaulting to the cross-linguistic distribution. This is the per-race-differentiable knob, a different shape over the same descriptive space, carrying no ranking.

**F. The grammatical-distance consumer.** The 33.5 grammatical component becomes a distance over the generic `TypologyProfile` vector, reusing the Part 21 value-distance machinery (a per-parameter categorical or structured distance combined under reserved per-parameter weights, with a single canonical rounding at the final narrowing), rather than a bespoke distance over four named fields. Where a parameter's values have their own similarity structure (word orders are not all equidistant), that structure is a per-parameter `GroundMetric` compiled offline exactly as value axes are.

**G. The drift consumer.** The grammaticalisation drift of 33.4 (morphology changing over generations) becomes a typology-shift operator that moves a culture's value along a registered dimension, fired in the within-generation phase order 33.4 already pins, keyed and deterministic, at a reserved rate.

## The reserved values, surfaced with basis (nothing fabricated)

- **The parameter registry seed membership and each parameter's value set.** Basis: the World Atlas of Language Structures parameter and value inventory as the human-grounded starting menu (dominant word order WALS 81A; object-verb order 83A; adposition order 85A; genitive 86A; adjective 87A; demonstrative 88A; numeral 89A; relative clause 90A; the morphological axes 20A, 21A, 22A; alignment 98A, 99A, 100A), the same "starting menu, not a ceiling" stance the modality feature registry took. The four current parameters are the floor.
- **The per-parameter cross-linguistic prior distributions.** Basis: the per-feature WALS sample frequencies, cited per feature and not pooled, since WALS samples differ by feature (81A about 1,377 languages, 98A about 190). For dominant word order the sample splits roughly SOV 41 percent, SVO 35 percent, VSO 7 percent, VOS 2 percent, OVS and OSV each under 1 percent, no-dominant-order 14 percent (WALS 81A). Already reserved in the 33.4 blockquote for word order and morphological type; this item attaches them to the registry as data and extends them per seeded parameter.
- **The harmony-model biases, reserved as ordinal strength tiers rather than fabricated per-pair numbers.** Basis: Dryer's Greenbergian correlations, stated as proportions of genera across six continental areas rather than as single coefficients, so the honest encoding is a tier per pair with the underlying proportion as the documented basis: a strong bias for the adposition, relative-clause, and verbal-and-function-word pairs (the adposition axis is roughly 94 to 95 percent harmonic in WALS chapter 95: 472 OV-postpositional and 456 VO-prepositional against 42 and 14 disharmonic), a weak bias for the genitive pair (0.89 of OV genera Genitive-Noun against 0.45 of VO genera, and reversed in one area), and no bias for the adjective, demonstrative, numeral, and intensifier parameters, which Dryer's negative results show do not correlate with verb-object order and so are drawn from their own marginals. Encoding the non-correlating parameters with zero bias is as load-bearing as the positive correlations: it keeps the harmony model from over-reaching into an order the data does not support.
- **The disharmony probability.** Basis: the observed per-axis disharmony rate, roughly 5 to 6 percent on the adposition axis (WALS chapter 95), the empirical anchor for how strongly the harmony tilt biases without being absolute. Already reserved in 33.4; kept, now cited.
- **The per-parameter grammatical-distance weights.** Basis: the contribution each typological dimension makes to measured mutual intelligibility, consistent with the 33.5 three-component weighting where lexical distance dominates.
- **The typological-drift rate.** Basis: the grammaticalisation-cline rates 33.4 already reserves (cited to Hopper and Traugott, bibliography L3537), set equal to them for consistency rather than introduced as a new free constant.
- **The per-parameter sampling priority (the anchor order).** Basis: object-verb order (WALS 83A) as the anchor every Dryer correlation is stated against, grounded in his Branching Direction Theory and Hawkins's processing account, a determinism-ordering value rather than a realism one. The honest caveat to carry in the basis: object-verb order is the reference dimension, not a claimed cause; the branching-direction consistency is the underlying variable, so the anchor orders the draw without asserting causation.

## Steering seams (Principle 9, to be red-teamed)

- **No sophistication ranking over types, and complexity only as a descriptive count.** No parameter and no value carries an "advancement" or "sophistication" score, and no word-order type, alignment, or morphological type is more evolved than another. The grounding confirms this is a foundational commitment of typology since Boas and Sapir (Sapir 1921), restated by Comrie (1989) and Evans and Levinson (2009), and the nineteenth-century isolating-to-fusional hierarchy is repudiated. The substrate may hold descriptive complexity axes where WALS does (inflectional synthesis, case count, tone, syllable complexity), because those are measured local properties, but a complexity count must never attach an ordinal ranking to a type and must never feed a civilizational-advancement signal. This is the direct analogue of the modality hold that a channel carries no sophistication field.
- **A typology-permutation invariant** (the analogue of the modality-swap invariant of record 62.13 and the value-metric basis-independence test). Relabelling which value is index 0, or permuting a parameter's value ids, must leave emergent richness and trajectory invariant, so no value is privileged by its index and the prior is not a hidden designer attractor toward one "natural" type.
- **Per-race priors carry no ranking.** A race's prior is a different distribution over the same descriptive space, never a "more advanced grammar" for a favoured race. The Steering Audit must confirm no prior encodes a race as linguistically superior.
- **Harmony is a tendency, not a correctness rule.** A disharmonic language is rarer, not broken; the reserved disharmony probability keeps disharmony reachable, and no consumer may treat a harmonic language as "correct" and a disharmonic one as "degenerate."

## Honest limits (surfaced, not hidden)

Two limits from the grounding carry into the reserved bases. First, WALS samples differ per feature, so the per-parameter priors are cited per feature and never pooled onto one denominator; a proposal that averaged them would misstate the frequencies. Second, Dryer's correlation strengths are proportions of genera across areas rather than single coefficients, so the harmony biases are reserved as ordinal tiers (strong, weak, none) with the proportions as the documented basis rather than as fabricated per-pair numbers; this is the never-fabricate discipline applied to a case where a single number would look precise and be invented. A third limit is inherited from 33.10: the typological priors and harmony filters keep generated languages in the plausible region but cannot certify every one as natural.

## Determinism feasibility note

Every added surface is integer, fixed-point, and seeded. The registry and the profile are sorted vectors keyed by id, not hashed maps, so every walk is canonical (33.10, R-CANON-WALK-aligned). The sampler draws in the pinned `sample_priority`-then-id order under the existing 33.4 draw key, so the outcome is a function of the seed, the culture, and the data, independent of evaluation order and thread count. The harmony tilt and the per-race prior combine in the widened internal scale with one rounding at the narrowing, matching the 33.4 accumulation discipline. The grammatical distance reuses the already-pinned Part 21 `GroundMetric` path. No floating point touches canon. The drift operator fires in the within-generation phase order 33.4 already fixes.

## Open decisions for the owner (batch, non-final)

1. **Parameter-set seed breadth.** Seed only the four current parameters, or the broader WALS harmonically-correlated set from the start? Recommendation: seed the broader set, so the harmony model has correlated parameters to act over (a harmony model with one anchor and no dependents is inert). Non-final; the registry grows either way.
2. **Per-race priors: default-shared or required-per-race?** Recommendation: default to one cross-linguistic prior shared across races (data), with per-race override optional, so per-race differentiability is available without forcing every race to carry a full prior at the dawn.
3. **Harmony-model granularity.** Pairwise directional biases (the grounded, determinism-friendly default), or a richer conditional model over multiple already-sampled values? Recommendation: pairwise, matching how the correlations are reported in the typological record.
4. **Typological drift in scope now, or phase two?** The flag requires keeping the drift consumer consistent; whether the typology-shift operator is built now or deferred behind the static generator is the owner's sequencing call.

5. **Does per-race genome bias enter grammar sampling?** The flag names a Part 25 coupling, but the design sweep confirms no site couples the four grammar parameters directly to the genome: the Part 25 coupling in the language cluster runs through the producible-sound and anatomy path (33.3, a race's phonemes are gene-affected through the vocal apparatus), not through word order or alignment. So the coupling is a design choice, not an existing wire: whether a race's genome tilts its typological priors the way it fixes its producible sounds, or whether typology stays a per-culture seed draw over a per-race (but not per-genome) prior. Recommendation: keep typology a per-race prior (data) without a direct genome tilt in the first pass, since real typological distribution is not known to track a species' biology the way a phoneme inventory tracks its anatomy; a genome tilt can be added later as data if the owner wants it.

## Adjacent seams the sweep surfaced (same closed-list character, not named by the flag)

The design sweep found sibling closed-list seams in 33.4 that carry the same "authored one level down" character and belong in, or beside, this substrate: the **affixation tendency** sampled alongside morphological type (L2315), the **`Morphology`** struct carried beside `GrammarParams` (L2323), the phonotactic **`SyllableShape`** vocabulary (L2295, partly mitigated as reserved data), the **`SoundChangeRule` rule-kind** space that is part of the canonical drift key (L2333), and the **script-type** typology (logographic, syllabic, alphabetic) derived from phonology in 33.8 (L2362, softened to a reserved-weight continuum). The affixation tendency and the `Morphology` struct are squarely morphological typology and should fold into the typology registry as further parameters. The phonotactic, sound-change-kind, and script-type seams are noted for the owner to decide whether they ride this item or their own; the channel registries of 33.3 are already the hardened template, not a seam.

## Cross-references to reconcile on consolidation

From the completed design sweep, the sites the consolidation must reconcile: the flag blockquote (L2339) itself; the `GrammarParams` struct and sampling prose (L2315, L2318-2319, L2323); the grammatical-distance component over the typological parameter vector (L2343, 33.5); the determinism statement naming typological priors (L2370, L2374, 33.10); the two "remains a closed set pending R-LANG-TYPOLOGY" notes (L2311, L2337); the grammaticalisation drift operator and its place in the within-generation phase order (L2329, L2333); and the research-record echoes that name the item as open (L3320-3324 in 62.6, L3384-3386 in 62.12, L3398 in 62.13). The consolidation, after sign-off, replaces the flag blockquote with the mechanism, adds a Decided-and-reserved blockquote, a Part 62 record, and a Part 63 bibliography group, and updates the audit log Section 1 block, Section 2, the backlog bullet, the queue, and the counts, then runs the verification suite. The Dryer (1992) Greenbergian-word-order-correlations entry is already in the bibliography (L3538), so the harmony-model biases cite an existing source; the grammaticalisation cline is already cited to Hopper and Traugott (L3537).

## Citations

The reserved bases draw on the following, verified against the primary sources during the grounding survey. The Dryer (1992) entry is already in the design bibliography (L3538); the grammaticalisation cline is already cited to Hopper and Traugott (L3537). The rest are added to the Part 63 group on consolidation.

- World Atlas of Language Structures Online, Dryer and Haspelmath (eds.), 2013, Max Planck Institute: the parameter and value inventories and the per-feature frequencies. Feature 81A (dominant word order, sample about 1,377); 83A (object-verb order, the anchor); 85A (adposition); 86A (genitive); 87A (adjective); 88A (demonstrative); 89A (numeral); 90A (relative clause); chapters 20, 21, 22 (fusion, exponence, synthesis, the morphological decomposition); features 98A, 99A, 100A (alignment per locus); chapter 95 (the object-verb-to-adposition harmony counts 472, 456, 42, 14).
- Dryer, M. S. (1992), "The Greenbergian Word Order Correlations," Language 68:81-138: the correlation pairs, the non-correlating pairs (adjective, demonstrative, numeral, intensifier), the genus-counting method across six areas, the weak genitive correlation, and Branching Direction Theory.
- Hawkins, J. A. (1983, 2004): the processing account of the branching-direction anchor.
- Greenberg, J. H. (1963), "Some universals of grammar": the original implicational universals the correlations restate.
- Sapir (1921), Comrie (1989), Evans and Levinson (2009): the descriptive-not-evaluative commitment grounding the Steering hold; McWhorter (2001) and the equicomplexity debate for complexity as a local descriptive property rather than a ranking.
