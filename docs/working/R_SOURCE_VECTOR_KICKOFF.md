# R-SOURCE-VECTOR: kickoff (the metabolic composition-class ontology, data-defined)

This is a DOC-ONLY kickoff, the bridge PR for the arc that follows the corrected Nernst uptake-flux (PR #112). It scopes the seam and states the discipline; it authors no mechanism. The FULL blind framing (section-11 input-bias smoke, then the section-10 blind panel) runs before any code, because this arc sits squarely on the value-authoring line: it opens the composition-class ontology, and the section-9 panel lenses matter here more than anywhere.

## The seam

The engine's metabolic chemistry keys on a FIXED set of Terran composition classes: a being's food value derives from `bio.energy_density`, its tissue from `bio.protein`/`bio.lipid`/`bio.structural_carbohydrate`, its catalyst tissue (the corrected-Nernst `Vmax`) from `bio.protein`, its harm from the `bio.toxin.*` axes. Each is a real floor axis, but the SET is Terran biochemistry baked in one level down. A being's metabolism is defined by which substances it draws as SOURCES, which it emits as WASTE, and which it carries as CATALYST, and today that vector is a closed Terran list rather than a per-being datum.

Three live flags converge on this one substrate:

- The corrected Nernst (PR #112) proxies a being's catalyst tissue by `bio.protein` as a per-source datum, and flags that the fuller CATALYST AXIS (a first-class catalyst datum rather than a borrowed protein-composition class) belongs to R-SOURCE-VECTOR, so a silicon or mineral-catalyst alien whose catalyst is not protein is a data row rather than a mis-proxy.
- The aging arc flags the METABOLISM-WASTE vector (the byproducts a metabolism emits, which accumulate and couple to senescence and to pollution) as the same shared substrate.
- The owner named this as the walled-agent downtime arc and the "life beyond Terran heterotrophy" seam: a being that eats a redox couple, a photon flux, or a mana field, and emits its own waste, on its own source/waste/catalyst vector.

## What the arc opens (scope, not mechanism)

The arc makes the metabolic source/waste/catalyst VECTOR a per-being (per-race, per-lineage) datum over the floor's substance axes, so a metabolism is a data row: which substances it consumes, which it produces, which catalyse it, each keyed off the being's own composition and the floor's substance chemistry, never a hardcoded Terran class list. The mechanism stays fixed Rust (the same harden-to-registry pattern the value, semantic, institution, and abiotic-source substrates use); the membership is data and grows with the world. A Terran heterotroph is one row (the current bake), a chemolithotroph another, a phototroph another, an alien with a non-carbon backbone another.

## The discipline (why this is frame-blind-before-code)

This is the opening of an ontology, the sharpest form of the value-authoring line (Principle 6, Principle 11) and the emergence-over-templates rule (Principle 8). The risks the blind panel must catch before any code: authoring a closed enum of metabolism KINDS where the vector should be data; baking a Terran source/waste/catalyst list as the "default" in a way that forecloses the alien; reading a high-level metabolic KIND to drive a behaviour rather than letting the outcome emerge from the substance chemistry and selection; and any observer-dependent or per-lineage authoring that breaks observer-independence (Principle 10). The section-11 smoke runs on the framing construction first (it has caught real bias every prior arc), then the section-10 blind panel judges the design against the principles alone, then the owner rules. No code until then.

## Status

Kickoff only. The corrected Nernst (PR #112, day-night feature-complete plus the alien-energy Nernst substrate) is the predecessor and merges first. The next step on this branch is the blind framing, posted for the owner's ruling, before any mechanism is built.
