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

1. **The parameter set is itself closed.** `GrammarParams` as a fixed four-field struct is a closed list one level up from the enum values: a world cannot add a typological dimension the engine's authors never enumerated (an adposition-order parameter, a classifier system, an evidential-ordering parameter, an exotic parameter a non-human cognition grammaticalises). The value substrate opens both the axes and their ranges; the typology substrate must open both the parameters and their values.

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

- **The parameter registry seed membership and each parameter's value set.** Basis: the typological record as the human-grounded starting menu (the World Atlas of Language Structures parameter and value inventory), the same "starting menu, not a ceiling" stance the modality feature registry took. The four current parameters (dominant word order, head-directionality, morphological type, alignment) are the floor; the broader harmonically-correlated set (adposition order, genitive order, adjective order, relative-clause order, and the rest) is the owner's call in the open decisions below.
- **The per-parameter cross-linguistic prior distributions.** Basis: the cross-linguistic frequencies of the typological record (WALS chapter frequencies, Dryer's samples). Already reserved in the 33.4 blockquote for word order and morphological type; this item attaches them to the registry as data and extends them to each seeded parameter.
- **The harmony-model biases.** Basis: the measured strength of each Greenbergian and Dryer word-order correlation in the typological record (the correlation of verb-object order with adposition order, genitive order, and relative-clause order, and their sampled strengths). This is the new reserved surface the flag calls for: the correlations as cited data rather than code.
- **The disharmony probability.** Basis: the observed rate of disharmonic languages in the typological record. Already reserved in 33.4; kept.
- **The per-parameter grammatical-distance weights.** Basis: the contribution each typological dimension makes to measured mutual intelligibility, consistent with the 33.5 three-component weighting where lexical distance dominates.
- **The typological-drift rate.** Basis: the grammaticalisation-cline rates 33.4 already reserves, set equal to them for consistency rather than introduced as a new free constant.
- **The per-parameter sampling priority (the anchor order).** Basis: the branching-direction anchor of the typological record (verb-object order as the anchor other orders harmonise to, per Dryer's Branching Direction Theory and Hawkins), a determinism-ordering value rather than a realism one.

## Steering seams (Principle 9, to be red-teamed)

- **No complexity or sophistication field.** No parameter and no value carries an "advancement", "complexity", or "sophistication" score. No typology is more evolved than another; the typological record is descriptive, ranking none. This is the direct analogue of the modality hold that a channel carries no sophistication field.
- **A typology-permutation invariant** (the analogue of the modality-swap invariant of record 62.13 and the value-metric basis-independence test). Relabelling which value is index 0, or permuting a parameter's value ids, must leave emergent richness and trajectory invariant, so no value is privileged by its index and the prior is not a hidden designer attractor toward one "natural" type.
- **Per-race priors carry no ranking.** A race's prior is a different distribution over the same descriptive space, never a "more advanced grammar" for a favoured race. The Steering Audit must confirm no prior encodes a race as linguistically superior.
- **Harmony is a tendency, not a correctness rule.** A disharmonic language is rarer, not broken; the reserved disharmony probability keeps disharmony reachable, and no consumer may treat a harmonic language as "correct" and a disharmonic one as "degenerate."

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

To be finalised from the typology grounding survey (in progress): the World Atlas of Language Structures (Dryer and Haspelmath) for the parameter and value inventories and the cross-linguistic frequencies; Greenberg's universals and Dryer's word-order-correlation work for the harmony biases and their strengths; Hawkins and Dryer's Branching Direction Theory for the anchor order. Precise chapter and figure citations are attached in the consolidation.
