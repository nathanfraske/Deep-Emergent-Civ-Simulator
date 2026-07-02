# The Medium and Respiration Substrate: Scoped Proposal (R-MEDIUM, owner-directed)

A scoped pre-sign-off proposal, in the shape the owner asked for ("scope R-MEDIUM as a proposal first"). It states the one new authored physics piece the generalized medium substrate needs, grounds it against the built floors and the anatomy substrate, surfaces every reserved value with its basis, and lists the decisions left to the owner. Nothing here is consolidated into the design document, and the resolved/open counts do not move, until the owner signs off. On sign-off this becomes the R-MEDIUM flag (audit Section 2, a `- **R-MEDIUM.` backlog bullet, open count 36 to 37) and, when built, the consolidation into Part 41 and Part 58 with a Part 62 record.

## The directive it answers

The owner asked for aquatic life, generalized: "generalize such that it can occur for any medium, a lava creature, some magical fluid creature, etc, and plants and sentient life," and confirmed the registry is "an axes of physics" (a medium is a physics `Substance` defined by physics-axis values). The chosen depth is full aquatic physiology: a being sits in a medium, respires it (or fails to), floats or sinks in it, exchanges heat with it, and an amphibious being carries a viable band in two media. The load-bearing constraint: medium affinity is EMERGENT, never a hardcoded "aquatic" flag. A high-surface-area membrane organ in contact with water respires water (a gill), one in contact with air respires air (a lung), from the organ's composition and the medium it touches plus selection, so a lava creature, a magical-fluid creature, a rooted autotroph drawing from its medium, and a sentient aquatic race all fall out of the same rules. The deferred bidirectional-thermotaxis gate (a being fleeing lethal heat needs a signed thermoreceptive percept, and lethal-hot media are what this work introduces) rides in here.

## The locked mechanism it builds on

The physics substrate is the data-over-registry floor Part 58 specifies and `crates/physics` implements: `QuantityAxis` (id, measures, unit, `Dimension`, range, tier, provenance), `InteractionLaw` (a fixed-Rust kernel bound through a contract table, with typed `LawPort`s and a declared output dimension), and `Substance` (a material or tissue as `vector: BTreeMap<String, Fixed>` over the axis ids, content-addressed by a label-blind 128-bit hash). A new floor is authored by adding axes and laws as data plus their kernels, the recipe waves 0 through 3 already followed.

The fluids floor (R-PHYS-W2) already carries the medium MECHANICS. `mat.density` runs down to 0.08 kg per cubic metre (the low bound widened to the lightest gas so a gaseous medium is expressible), `law.buoyant_force` is Archimedes (rho g V, reading `fluid.immersed_volume`), `law.hydrostatic_pressure` is rho g h, `law.ideal_gas_density` and `law.thermal_buoyancy` close the gas mechanics, `therm.temperature` is the shared state axis, and the floor already defines `air` (density 1.225) and `water` (density 998) as substances. So buoyancy, hydrostatics, and in-medium density are present today; what is absent is the gas-exchange CHEMISTRY (below).

The anatomy and homeostasis substrate this proposal extends is built and, as of the red-teamed hardening, keyed off floor axis ids as data. An organ's `TissueComposition` is a `BTreeMap<String, Fixed>` over biology-floor axis ids (the `Substance::vector` convention), an organ's reserve-backing function is DERIVED from that composition rather than tagged, and a `HomeostaticAxisDef` names the floor axis id it is `backing_component` on. `Homeostasis::new` sets a reserve's capacity to the development-weighted sum over the being's organs of their composition on that axis. Because the vocabulary is now floor data, a respiration reserve backed by a new respiratory-surface axis is a DATA edit: add the axis to the floor, add the axis to a gill or lung organ's composition, and name it on a respiration `HomeostaticAxisDef`. `Homeostasis::new` needs no change. This is the direct payoff of the hardening and the reason R-MEDIUM is now a small extension rather than a code rework.

The design document already gestures at this floor. Part 35's reserved blockquote states the environmental thermal exchange "waits on the located world and a medium floor," the single explicit naming of a medium floor as pending. Part 20 carries `DriveDynamics::FieldCoupled { field: FieldId }`, the shape a respiration drive coupled to an ambient-medium field would take. Part 18's `WeatherFields` (temp, humidity, wind, precip, pressure) is the resident field layer set an ambient-medium or dissolved-gas field would join. Part 15's `Stock` is the compartment a respiration reserve is. Part 41 is where a physics floor consolidates, under the open R-DEEPTECH-PHYSICS umbrella, adjacent to record 62.21 (the fluids floor).

## The seams: what is absent and must be authored (auditing the input, not the flag alone)

A sweep of `crates/physics` and the sim and world crates finds the medium mechanics present and the gas-exchange chemistry and the ambient medium entirely absent. Specifically absent:

- A respirable-gas partial-pressure or concentration axis. The nearest neighbours are `fluid.vapor_pressure` (the partial pressure of water vapour, the moisture the water cycle transports) and `fluid.driving_pressure` (local absolute pressure the gas law reads). Neither is the partial pressure of a respirable gas.
- A gas-solubility (Henry's-law) axis and law. `chem.solute_affinity` plus `law.dissolution` model a leached fraction of a solute, not a gas partition against a partial pressure.
- A diffusivity axis and a Fick's-law membrane-diffusion kernel. Entirely absent; the only "diffusion" in the repository is the fixed-point HEAT-field stencil, a thermal field, not a substrate law.
- A membrane-permeability axis. Absent (`mag.permeability` is magnetic).
- A dedicated gas-exchange surface-area axis. Absent; only `mech.contact_area` exists to borrow.
- The respirable content of a medium. The `air` and `water` substances carry density, viscosity, and bulk modulus only, no dissolved or partial-pressure oxygen content.
- An ambient-medium field. Nothing in the sim or world tells a being which medium it occupies. The only water-being interaction that exists is terrain passability in one example (deep water blocks a non-swimmer), a movement gate, not a respiratory medium. Nothing floats, drowns, suffocates, or breathes.

The one new authored piece, then, is a gas-exchange and respirable-medium physics law, plus the ambient-medium field that tells a being what it is immersed in. Everything downstream (which organ respires which medium, whether a body floats, whether an amphibian survives on land) is emergent from that floor plus the existing mechanics plus selection.

## The proposed floor (mechanism fixed Rust, membership data)

Three parts: new floor axes, one new law kernel, and the ambient-medium field.

New floor axes (data, added to the fluids or a new respiration sub-floor, each range reserved-with-basis below):

- `fluid.respirable_content`: the concentration or partial pressure of the respirable species in a medium (dissolved oxygen in water, oxygen partial pressure in air, an exotic respirable in a magical fluid). A property of the medium `Substance`, so `air` and `water` gain this axis and an authored exotic medium sets its own.
- `fluid.gas_solubility`: the Henry's-law partition coefficient relating a dissolved concentration to a partial pressure, so a medium's respirable content is expressible either way and cross-medium comparison is grounded.
- `bio.membrane_permeability`: the permeability of an exchange membrane to the respirable species, a tissue property of a respiratory organ.
- `bio.respiratory_surface`: the specific gas-exchange surface a respiratory tissue presents per unit organ development, a tissue-composition property. This is the axis a gill or lung organ carries in its `TissueComposition`, and the axis a respiration `HomeostaticAxisId` is `backing_component` on. Because the anatomy substrate is now floor-axis-keyed, adding this axis is a data edit.

One new law kernel (closed-form integer over `Fixed`, no transcendental, so it is GPU-portable and pins bit-identically under R-GPU-CANON-PIN):

- `law.membrane_gas_flux`: Fick's law across a membrane, the exchange rate `J = permeability * area * (P_medium - P_internal)`, where `area` is the being's total exchange surface (the development-weighted sum over its organs of `bio.respiratory_surface`, the same composition-derived sum the reserve capacity already uses), `permeability` is `bio.membrane_permeability`, and the gradient is the medium's `fluid.respirable_content` (partitioned through `fluid.gas_solubility` where a dissolved-versus-partial-pressure conversion is needed) minus the being's internal reserve level. The flux is the per-tick intake into the respiration reserve, so a being in a medium its organs exchange with well breathes, and one out of its medium (a fish in air: high surface, but the ambient respirable content it can partition is wrong) fluxes toward zero and its respiration reserve drains to the death floor. Suffocation and drowning are the same law with the medium swapped.

The ambient-medium field: a per-cell medium `Substance` id (content-addressed), the GPU-portable stencil shape the temperature field already uses, so a cell is air, water, lava, or an authored fluid, and a being reads the medium of the cell it occupies. This joins Part 18's resident field layers and couples to Part 20's `FieldCoupled` respiration drive. Buoyancy reads the being's body density (already derivable from its tissue composition and the `mat.density` axis) against the ambient medium's density through the existing `law.buoyant_force`, so a dense body sinks and a light one floats, no flag. In-medium thermal exchange reads the ambient medium's thermal axes through the existing conduction and convection laws (the Part 35 thermal-exchange blockquote this floor unblocks). An amphibious being is one whose organ set and permeability give it a viable respiration flux in two media; nothing declares it amphibious.

## Emergent medium affinity (the anti-flag discipline, Principle 9)

No being carries an "aquatic" or "terrestrial" tag. A being respires a medium to the extent its respiratory organs' surface and permeability exchange with that medium's respirable content, measured by `law.membrane_gas_flux`. A gill is an organ with a high `bio.respiratory_surface` and a permeability tuned to a dissolved species; placed in water, its flux is high; placed in air, the same organ's flux collapses because the ambient partition is wrong. A lung is the mirror. A walking amphibian carries both, or one broad-tolerance organ, and pays a cost in each medium. A rooted autotroph draws from its medium the same way. A lava creature is a being whose permeability and internal chemistry give it a positive flux in a medium whose `therm.temperature` is lethal to everything else, and whose body density floats it on that medium. A magical-fluid creature is an authored medium `Substance` with an exotic respirable content and an organ tuned to it. All of these are points in the same data space, selected over, never authored as kinds. This is the same physics-in, behaviour-out discipline the organ substrate already holds: we author the floor (the gas-exchange law, the medium substances, the tissue axes), and the affinity emerges.

The bidirectional-thermotaxis gate rides in here. Once a medium can be lethally hot (lava, a desert-air medium), a being needs to flee heat as well as seek it, which the current even comfort reserve plus raw temperature-gradient percept cannot express (a linear controller reading an unsigned gradient learns warmth-seeking in a cold world but not hot-fleeing). The medium substrate is where lethal-hot environments first exist, so the signed thermoreceptive percept (a warm-versus-cold receptor as a new input channel, letting selection evolve the hot-versus-cold gating) lands with it, not before.

## Determinism and Principles 9 and 11 (held under review)

Every axis is fixed-point `Fixed` (Q32.32), every law a closed-form integer kernel, the composition and medium walks are over `BTreeMap` in sorted id order, and the ambient-medium field is the same deterministic stencil the temperature field uses, so the whole extension reproduces bit for bit (Principle 3) and pins under R-GPU-CANON-PIN. The mechanism is fixed Rust; the axes, the medium substances, and the organ compositions are data that grow with the world (Principle 11), the same substrate discipline the value, semantic, institution-function, physics, and now organ layers hold. The only authored inputs are physics (the gas-exchange law form, the medium substances, the tissue-axis vocabulary); no behavioural or affinity outcome is authored (Principle 9).

## Reserved values, surfaced with basis (nothing fabricated)

Each is the owner's to set through the calibration manifest, defaulting to a fail-loud sentinel, never a fabricated default. Each is given with the basis on which the owner would decide it.

- `fluid.respirable_content` range and the `air` and `water` values. Basis: the physical partial pressure of atmospheric oxygen (about 21 kPa at sea level) and the saturated dissolved-oxygen concentration of water (single-digit mg per litre near room temperature), the datasheet values the medium substances carry.
- `fluid.gas_solubility` range and the water value. Basis: the Henry's-law constant for oxygen in water at the reference temperature, the standard physical-chemistry constant relating the two above.
- `bio.membrane_permeability` range. Basis: the measured gas permeability of a respiratory membrane (a gill lamella, an alveolar surface), a physiological range from the literature, per tissue.
- `bio.respiratory_surface` range and the gill and lung fixture values. Basis: the specific surface area of a real respiratory organ per unit mass (the alveolar and lamellar surface densities), reserved as a labelled fixture until the owner sets the range.
- The `law.membrane_gas_flux` scale (the per-quantity scale relating permeability times area times partial-pressure gradient to a per-tick reserve intake). Basis: the resting oxygen-uptake rate of an organism divided by the base tick, so a being at rest holds its respiration reserve steady and exertion draws it down, the same tick-scaling the metabolic drains use.
- The respiration `HomeostaticAxisDef` drain and death floor (`base_drain`, `exertion_drain`, `death_floor`). Basis: the resting and working oxygen demand of Part 20 mapped onto the base tick, and the reserve fraction at which hypoxia is lethal, set equal in shape to the energy and water axis bases for consistency.
- The lethal-medium temperature bands (for the bidirectional-thermotaxis gate). Basis: the temperature past which a being's tissue fails, which the mechanical and thermal floors already define (protein denaturation, the melting and ignition axes), read rather than invented.

## Steering seams (Principle 9, to be red-teamed before build)

The seams a build must audit, so no cultural or affinity outcome is authored one level down:

- The medium substance set must be content-addressed and label-blind, so `water` is its axis vector, not a privileged string. A lava or magical medium must be expressible as data with no code path special-casing "water" or "air".
- The gas-exchange law must key off the ambient medium's `fluid.respirable_content` and the organ's composition, never off a being's kingdom or a habitat tag. There must be no "if aquatic" branch anywhere.
- `bio.respiratory_surface` must be a tissue property an organ bears, so a gill and a lung differ only in their composition and permeability, not in a category. The Steering Audit must confirm no organ carries a medium tag.
- The amphibious band must emerge from a two-medium positive flux, not from an "amphibious" flag on a race.

## Honest limits (surfaced, not hidden)

- The reduced-order Fick's-law flux is a lumped membrane model, not a spatially resolved gas-transport simulation; it floors respiration and drowning faithfully but does not model circulation or oxygen debt beyond the single reserve. Those are deepenings under R-DEEPTECH-PHYSICS.
- The single respirable species (an oxygen analogue) is the first cut; multiple respirable species (a carbon-dioxide off-gas, an anaerobic path) are a data extension the axis set allows but this proposal does not build.
- The ambient-medium field is a per-cell substance id, not a resolved fluid dynamics; currents, stratification, and mixing are a Part 18 weather deepening, not this floor.
- Buoyancy and in-medium thermal exchange reuse the existing fluids-floor laws, so their honest limits (lumped coefficients, no turbulence) carry over.

## Open decisions for the owner (batch, non-final)

1. Scope of the first build: land the full aquatic physiology (medium field, respiration reserve, buoyancy, in-medium thermal exchange, amphibious bands) in one pass, or stage it (respiration first, then buoyancy and thermal)?
2. Where the respirable-content axis lives: extend the fluids floor, or open a dedicated respiration sub-floor? (Recommendation: extend the fluids floor, the medium mechanics already live there.)
3. Whether `bio.respiratory_surface` is a biology-floor axis (a tissue property, the recommendation, so it flows through the organ composition) or a geometry axis read separately.
4. Single respirable species now, or the multi-species axis set from the start?
5. Whether the ambient-medium field is authored per world (a data map) or generated from the worldgen (oceans and atmosphere as emergent regions).
6. The bidirectional-thermotaxis percept: land the signed warm-versus-cold receptor channel with this floor (the recommendation, since lethal-hot media arrive here), or defer it again.

## Cross-references to reconcile on consolidation

On sign-off and build: Part 41 gains a medium-and-respiration floor subsection and a "Decided and reserved" blockquote; Part 58 gains the registry note; Part 35's medium-floor blockquote is reconciled to resolved; Part 20's respiration drive and Part 18's ambient-medium field are wired; a Part 62 record is added adjacent to 62.21, and a Part 63 bibliography group (respiratory physiology, Fick's law, Henry's law) is added. Audit Section 2 gains the R-MEDIUM flag and Section 3 a backlog bullet; TODOS moves the open count 36 to 37 on the formal flag. The reserved values above seed `calibration/reserved.toml` under a `medium.*` and `physiology.respiration.*` grouping when the floor is consumed.

## Citations

Grounded against the built substrate: `crates/physics/src/lib.rs` (the `QuantityAxis`, `InteractionLaw`, `Substance` registry), `crates/physics/data/fluids_floor.toml` (the medium mechanics and the `air` and `water` substances), `crates/physics/data/biology_floor.toml` (the composition axes), `crates/physics/src/laws.rs` (`law.buoyant_force`, `law.hydrostatic_pressure`, `law.ideal_gas_density`, and the absence of a gas-exchange kernel), `crates/sim/src/anatomy.rs` and `crates/sim/src/homeostasis.rs` (the floor-axis-keyed organ and reserve substrate), and design document Parts 15, 18, 20, 35, 41, and 58 with record 62.21. Physical bases for the reserved values are the standard respiratory-physiology and physical-chemistry constants (oxygen partial pressure, dissolved-oxygen saturation, the Henry's-law constant for oxygen, respiratory-membrane permeability and specific surface, resting oxygen uptake), each to be set by the owner from the cited datasheet rather than fabricated here.
