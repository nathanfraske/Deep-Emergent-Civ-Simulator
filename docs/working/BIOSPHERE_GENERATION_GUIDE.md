# Biosphere Generation: Seeding Flora, Fauna, and Edibility Without Hand-Authoring

**A standalone design for how a world is populated with many plant and animal varieties, and how edibility, nutrition, toxicity, and medicinal value are determined, without hand-authoring each species.**

Status: a design doc in the project's style. The principle and the structure are derivable from the existing ecology parts and the project's ideology, and are stable enough to build against, with the standing clause that any locked decision yields to evidence. The trait-axis catalogue, the composition axes, the law forms, and the seeding parameters are research-populated and owner-reserved, never fabricated here.

---

## 1. What already exists

The biosphere is not starting from nothing; most of the machinery is already specified, and this document fills two gaps in it rather than replacing it.

- **Stocks and flows (Part 15).** Vegetation biomass, game density, water, and population are all stocks that flow between compartments, regenerate logistically toward a capacity, and collapse when sustained draw exceeds regeneration. The food web is built from these.
- **Flora (Part 16).** A `PlantSpecies` already carries a climate envelope, a growth rate, a max biomass, a successional role, a fire response, a dispersal range, mutualists, and a data-defined `draws_on` set (the `GrowthInput` enum: light, water, soil nutrient, a data-defined resource, prey for a carnivorous plant, or carrion), so what nourishes a plant is a data reference rather than a hardcoded sun-water-soil assumption. Plants run the genome at the population tier and become an explicit genome only when a notable organism is promoted.
- **Fauna and the food web (Part 17).** An `AnimalSpecies` already carries a trophic level, a data-defined `feeds_on` set (the `FoodSource` enum: vegetation, another species for predation, carrion, or a data-defined resource), a climate envelope, biomes, reproduction and food and water needs, an intelligence dial that gates how much personality a creature carries, domesticability, an optional ecosystem-engineer effect, and a genetic scheme. The food web is a coupled set of stocks producing predator-prey oscillation, trophic cascades, migration, domestication, and extinction on its own.
- **The genome and deep-time evolution (Part 25, R-GENOME).** The genome model applies uniformly to sentient races, animals, and plants, with drift and breeder's-equation selection over allele-frequency pools, ecotype formation where one species holds different biomes, and declared rather than scripted speciation. Heritable channels already include climate tolerances, growth rate, fire response, and pathogen resistance.
- **The consumers of edibility.** A race's drives and their satisfaction sources decide which stocks it draws on and whom it competes with (Part 20); anatomy is data-driven (Part 35); dietary, agricultural, and medicinal knowledge is a technique a culture accumulates, transmits, and can lose (Part 23); poison and disease are their own system (Part 22); and food is a material and resource (Part 19, with the materials-as-points-over-axes model of the Physics Substrate Guide).

The species structures are therefore already data-driven, and the machinery to diversify a founder set over deep time already exists.

## 2. The two gaps this document fills

- **Seeding the species set without hand-authoring.** The structures exist, but nothing yet says how a freshly generated world acquires hundreds of coherent species without someone filling in each `PlantSpecies` and `AnimalSpecies` by hand.
- **Edibility, nutrition, toxicity, and medicinal value.** Neither species struct carries any of these. Whether an organism is food, poison, or medicine, and to whom, is unmodelled, and it is exactly the kind of functional outcome the project measures rather than stores.

## 3. The principle, consistent with the rest of the project

A species is a point, not a hand-authored special case: a vector of values over the trait axes the species structures already declare, generated rather than enumerated, the same move that makes a material a point over the physics axes. And edibility is a measured, relational consequence, not a stored flag: it is computed from an organism's composition and a consumer's physiology, the same way chopping and penetration are measured from physics rather than stored. Which trait and composition axes must exist is set by demand-closure: by what the consumers (the food web, the drives, agriculture, medicine, materials, anatomy, and disease) need to be able to evaluate, never by enumerating biology for its own sake.

## 4. Seeding the species set (the first gap)

The bulk of a world's variety comes from generation and then evolution, not from a hand-written list.

**Author the space, generate the species.** As with the seeded-physics grammar, you author the generative space (the trait axes and their ranges, the niche and food-web constraints, and the biome-fit laws), not the species themselves. A species is then a sampled vector over those axes, drawn through counter-RNG keyed on the world seed, so the biosphere is part of the world's deterministic identity and reproduces from the seed.

**The food web is the closure constraint.** The generator does not emit a random bag of organisms; it emits a closed food web. Generation is the ecological analogue of demand-closure: every generated consumer must have a viable niche and a real food source in the biomes this world has. Producers ground in the vegetation layer and the physics; herbivores reference producer pools; carnivores reference prey pools; decomposers close the loop on carrion and dead biomass. A candidate with no niche, an orphaned trophic level, a carnivore with no prey, or a specialist whose only host is absent, is a failed candidate that is resampled. This is generate-and-validate: sample a candidate species, check that it fits an open niche, that its `feeds_on` or `draws_on` resolves against existing pools, that the food web stays closed, and that it is representable in fixed-point, then keep it or resample at the next ordinal.

**Fit to the generated world.** Candidates are constrained to the biomes and climate the world generation produced, so organisms fill the niches that exist rather than being scattered at random: a cold-dry region is seeded with cold-dry-enveloped producers and the herbivores and carnivores that close on them. The biosphere is grown to fit the map, the way the demand register fits the consumers.

**Diversification does the rest.** The seeded founders are not the final roster. The genome, selection, and speciation machinery of Part 25 runs over deep time, radiating founders into ecotypes and daughter species fitted to local biomes, which is where the hundreds of varieties come from without anyone authoring them. The variety count is the product of generation (the founders) and evolution (the radiation), not of enumeration. A mod or the owner can still author a specific notable species by hand, a named dragon or a remembered ancient tree, and it drops into the same structures; hand-authoring becomes the exception for the memorable, not the means of populating the world.

## 5. Edibility, nutrition, toxicity, and medicine (the second gap)

The reframe is the same one that resolved materials: edibility is not a property of an organism, it is a relation between an organism's composition and a consumer's physiology. The same berry is food to one race, poison to another, and medicine to a third, so storing a single edible flag on the species would be the authored-outcome mistake the project avoids.

**Store composition and physiology; measure the rest.** An organism carries a composition vector, its tissues as points over nutrient axes, toxin axes, and digestibility, in the same shape as a material's vector over physics axes, and tied into the biology axes of the Physics Substrate Guide. A consumer carries a physiology, its nutrient needs (from its drives' satisfaction sources, Part 20), its toxin tolerances, and its digestive capability (from its anatomy, Part 35). Edibility-to-this-consumer is then a law, a measured consequence computed from the two: the nutritional value the organism supplies against this consumer's needs, the harm its toxin load does against this consumer's tolerances (feeding into sickness and death through the disease and poison system, Part 22), and the net reading of food, poison, medicine, or inert. All of it is relational, fixed-point, and closed-form, and none of it is stored as a verdict.

**Dose and preparation make it dynamic.** Toxicity is dose-dependent and often preparation-dependent: a tuber that is poison raw becomes safe cooked or fermented, which is a discovered preparation technique (Part 23). Edibility therefore shifts with a culture's knowledge, so a people can discover that a staple must be processed, or fail to and poison itself, which is emergent culinary and medicinal history rather than an authored property.

**Dietary knowledge is emergent and fallible.** A culture does not begin knowing which plants are nutritious or toxic to its people; it discovers this through the evidence system (R-EVIDENCE), and that knowledge is a technique that can be true or false, can spread and distort, and can be lost (Parts 9, 23). What a people eats, what it calls poison, what it uses as medicine, and what it falls back on in famine all emerge from its physiology meeting its environment and its accumulated, fallible knowledge.

**Composition is heritable, so the biosphere evolves its chemistry.** The composition axes are heritable channels under Part 25, so toxin content and nutritional content drift and respond to selection: a prey plant under grazing pressure evolves defensive toxins, and a crop under domestication selection loses them and gains yield, which is the real domestication syndrome falling out of the existing machinery rather than being scripted.

## 6. How this composes with the rest

This adds two things to the existing structures, a generative space plus its food-web-closure validator, and a composition vector on organisms plus the edibility laws, and reuses everything else. It draws its composition axes from the physics and materials substrate, its consumer physiology from drives and anatomy, its diversification from the genome, its dietary knowledge from the evidence and technique systems, and its harm channel from the disease and poison system. It composes with the world profile: a humans-and-base-physics world generates a biosphere with no mana or exotic axes, because the generator reads that world's active physics and biome profile, so the biosphere is as bounded or as rich as the profile that produced it. And it is deterministic throughout, generation keyed on the world seed and every law fixed-point closed-form, so a world's biosphere is part of its reproducible identity.

## 7. Settled versus open

**Locked now, overridable on evidence.** The principle (species as generated points over trait axes rather than hand-authored entries; edibility, nutrition, toxicity, and medicine as measured relational consequences rather than stored flags); the generate-and-validate seeding with the food-web-closure constraint and biome-fit; the reuse of the genome machinery for diversification; the composition-vector extension to organisms and the relational edibility laws; dose and preparation dependence; and the determinism constraints.

**Open, research-populated and owner-reserved, not guessed here.** The trait-axis catalogue (which axes a species varies over beyond those already on the structs) and the composition axes (which nutrient classes, toxin classes, and digestibility dimensions), grounded in real ecology, nutrition, and toxicology, with the fantasy axes (mana content, exotic toxins) owner-reserved; the niche and food-web-closure validation criteria and the biome-fit law forms; the edibility, nutrition, and harm law forms, chosen, converted to fixed-point, and validated; and the seeding parameters (founder counts, diversity targets, resample bounds), reserved with their basis. A fabricated axis set or law here would be the steering leak the project audits for, so it is deferred to the fan-out, the datasheets, and the owner.

## 8. Tracking and the fan-out

This is a candidate research item in its own right, the biosphere analogue of the physics substrate work, and it warrants the same treatment: a fan-out across ecology, nutrition, and toxicology and across the dependent consumers, with a red team that attacks whether the generator produces coherent closed food webs, whether edibility stays relational rather than collapsing to a stored verdict, and whether the diversification stays deterministic and bounded. If it is taken up, the clean way to track it is as its own backlog item with this document as its vehicle, sequenced after the physics and materials substrate and the genome are built, since the composition axes and the diversification both read through those.
