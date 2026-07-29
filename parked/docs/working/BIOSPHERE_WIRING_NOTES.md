# Biosphere wiring: owner directives (captured 2026-07-07, for the biosphere arc after arc 3)

The owner set the scope and constraints for the biosphere-wiring arc (TODOS task 21) in a run of sign-off
messages. Captured here verbatim-in-substance so the arc picks them up without loss. This is a working note,
not canon; fold each item into the arc's slices as they land.

## 1. Wire the generated ecology into the world, opt-in plus a real demo, with a flip note

Build each biosphere mechanism byte-neutral opt-in (the standing discipline), then deliver a REAL, runnable
wired world (a worldgen path or example composing the pre-dawn biosphere epoch, then the dawn, then play) that
truly populates a generated ecology, while leaving the canonical `run_world` (`a465919e`) and crucible
(`254bc17c`) hashes intact so nothing regresses. Leave an EASILY-ACCESSIBLE note on how to flip the wiring
into the main/default world (the owner's words: "that's the easy part"), so the owner can take the deliberate
re-baseline (a new canonical hash) when ready. Do not detonate the byte-neutral invariant unilaterally.

## 2. The heart of it: organisms made of usable MATERIAL STUFF

The core of the biosphere wiring is that the creatures and plants (and the people, which the owner says are
mostly done) are made of STUFF: real material substances (the material substrate, `crates/sim/src/material.rs`
and the physics floor) that can be foraged, manipulated, eaten, cooked, and otherwise used through the tool
and material systems the made-world arc (arc 3) builds. A generated species is not an opaque food token; its
body is composition a being can cut, carry, cook, and metabolise. This couples the biosphere's generated
species and its per-part body (anatomy, R-BUILD-PHYS) to the material substrate, the tool-use arc, and the
matter cycle. People being "mostly done" means their tissue-and-material bodies already exist; the work is to
give plants and animals the same material composition so they interact with the same substrates.

## 3. Decomposition as EMERGENCE, not an authored decay law (Principle 8)

The matter cycle today carries an AUTHORED decay: organic matter (carrion, the spent_hull trace, a corpse)
fades by a coded matter-cycle law keyed on `bio.decomposition_barrier` and a reserved decay rate (see
`crates/sim/src/trace.rs` `DecayLaw` and the matter-cycle weathering the physical-trace slice D proved). The
owner ruled this a steering defect: there must NOT be a default "matter decays always" law authored into the
engine. Replace it so decay is DRIVEN by either (a) decomposer LIFE (microbes, fungi, scavengers as generated
biosphere species carrying a decompose function that consumes the dead matter, the F4 DECOMPOSER food-web
closure fork), or (b) world-specific or evolved CONDITIONS (chemical or physical: moisture, temperature,
oxygen, a microbial-activity proxy) that a world generates rather than the engine hardcoding. So in a sterile
or frozen or anoxic world, matter does NOT decay (no decomposers, no conditions), and in a living warm wet
world it decays fast, because the decomposers or the conditions are there, never because the engine says all
matter must rot. The physical-trace weathering's falsifiability-by-physics (an unsupported trace fades) must
be preserved, but its DRIVER becomes emergent: a trace persists in a world that cannot decompose it. This is
the decomposer half of the food-web closure the R-BIOSPHERE resolution already scoped, sharpened to "no
authored universal decay".

Reserved-value and steering discipline apply throughout: never fabricate a rate, surface it with basis;
keep each mechanism byte-neutral opt-in until the flip; let the outcome emerge from generated life or
conditions, never a coded category.

---

## THE FLIP NOTE (how to turn the biosphere seams on), updated 2026-07-07

Both landed seams are byte-neutral opt-in and stay off on the canonical run (`run_world` `a465919e`). Each
is one setter on the `Runner`, so a world flips it on when its worldgen is ready, with no engine default
changed. The flips:

**Decomposition as emergence (directive 3, `crates/sim/src/decompose.rs`).** The matter cycle no longer
decays all warm matter unconditionally; the rate a cell realises is a per-cell ACTIVITY the world derives
from decomposer life and conditions. To flip it on: arm the matter cycle (`runner.set_matter_cycle(calib)`)
AND a decomposer registry (`runner.set_decomposer(DecomposerDriverRegistry)`), choosing the driver rows and
the `CombineMode` the world means (Life-only for life-driven decay, a Conditions row for the abiotic proxy,
both under `All` for life gated by conditions). The Life kernel reads a per-cell decomposer-biomass stock
(`runner.set_decomposer_stock(...)`), hand-seeded today and filled from a generated decomposer species once
that lands. Unarmed, the matter cycle keeps its old unconditional rate; with no matter cycle at all, nothing
decays. Reserved values (moisture saturation, oxygen reference, warmth span, biomass reference) are surfaced
with basis in `calibration/reserved.toml`, owner-set at the flip.

**Organisms as usable material stuff (directive 2, first slice, `crates/sim/src/material.rs`
`TissueField`).** A death can leave the being's body as located matter, a composition vector derived by a
development-weighted fold over its own body plan (`physiology::whole_body_composition_vector`), never a
minted substance or a species-to-substance map. To flip it on: `runner.set_corpse_matter(true)`. A death
then deposits the corpse into the embodiment's tissue field where it fell, and the extraction contest reads
its fracture hardness through the same axis as rock (a forager can work it). Off by default, so a death just
retires the body. Reserved as data awaiting owner sign-off: the tissue density, ash fraction, and
decomposition barrier on each organ/covering/weapon kind in `crates/sim/src/anatomy.rs` (today's dev
fixtures carry only what the mechanism proof needs, so a corpse carries what its body declares and no more),
and the corpse volume derivation (the body-size trait now, the physical mass-over-density once tissue
density lands). Follow-on slices route cutting, the matter cycle, and eating (edibility reads a substance-id
table, so eating tissue routes through the composition vector) onto the tissue field, and fill it from the
generated biosphere rather than a death alone (task 21).

The two seams COMPOSE: a corpse left by directive 2 is exactly the organic matter directive 3 decomposes,
and a generated decomposer species is both the Life kernel's biomass source and an organism made of usable
material stuff, so the biosphere's death, matter, and decay close one loop once worldgen fills the fields.
