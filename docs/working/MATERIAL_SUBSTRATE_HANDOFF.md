# Material-Substrate Handoff: give the world matter, and let the economy fall out of it

This is a self-contained brief for the agent building the material-substrate arc, the next arc after harm-learning. Read `CLAUDE.md` first. The arc's one job: add MATTER (located, structured, physical stuff off the physics floor), from which mining, harvesting, gathering, tool-making, building, hoarding, trade, and settlement EMERGE from a single equation, need plus physics-derived affordance plus structured matter, never scripted. One OWNER-CALL seam is flagged inside (item 8: whether `DecayLaw` should become a data-defined `TransformKindRegistry` sibling to the value and semantic substrates); it is surfaced for your ruling, not resolved. File:line anchors are verified against the tree; re-grep after any edit.

---

# HANDOFF BRIEF: THE MATERIAL SUBSTRATE ARC

The arc after harm-learning. Its one job: give the world MATTER, and let mining, harvesting, gathering, tool-making, building, hoarding, trade, and settlement fall out of it. Nothing on this list is scripted. Every one of them is the same equation resolving against different matter.

---

## 1. THE UNIFYING GAP

The simulator has two kinds of world state and is missing a third. It has FIELDS: the temperature `Field` with its pinned four-neighbour diffusion stencil (`runner.rs:347,433`), the water/salt/productivity `EnvironFields` stack (`environ.rs:324`), each a per-cell scalar folded into `state_hash`. It has ORGANISMS: grown bodies whose `strength` is derived intact muscle mass times flesh fracture strength (`body.rs:790`), walkers that percept and move. What it does not have is MATTER: located, structured, mechanical stuff a being can break, carry, work, stack, burn, and deposit.

The gap is visible at file scope. A `Tile` is a surface record only (elevation, moisture, temperature, biome) and worldgen fills only the `z==0` layer (`worldgen.rs:31-37,132`), so the z-column below the surface is empty even though `Coord3` carries `z` and `FlatBounded` carries `layers` (`topology.rs:31-35,99-103`). The one per-cell matter field that exists, `ResourceField` (`locomotion.rs:199`), carries a biological `Composition` (nutrient supply, toxin dose over the edibility floor's axes), the grazable food-and-water stock, and nothing mechanical: no hardness, no density, no substance a claw could fracture or a fire could consume. The physics floor already defines matter in full, a data-driven `Substance` registry loaded from TOML with `mat.density`, `mat.indentation_hardness`, `mat.fracture_strength`, `mat.fracture_energy` axes (`physics/lib.rs:408`, `mechanical_floor.toml`), but no cell in the running world is keyed to a `Substance` id. A grep of the sim crate confirms the absence from the other end: no inventory or carry, no worked objects in-world, no fuel or fire or light field, no corpse-to-matter, ash, tailings, spoil, or waste deposit anywhere. `world.rs Trace` is a perception record, not a mass deposit.

So the world is fields and organisms but not matter. The gap is a single missing substrate, and closing it is the whole arc.

**The single rule that closes it.** Every behaviour on the deliverable list is one equation:

> **behaviour = need + physics-derived affordance + structured matter**

A being mines because a homeostatic reserve wants metal (NEED), its grown or gripped limb clears a rock's hardness in a raw force contest (physics-derived AFFORDANCE), and the rock is a located substance with a derived hardness (structured MATTER). Change the matter and the same three-term resolution becomes harvesting, digging, cooking, or building. Nothing reads "this being is a miner." The role is the outcome of the equation, never its input. Hold that rule and the cascade emerges; break it anywhere and you have authored the thing the design forbids.

---

## 2. THE GOAL

Add MATTER as a data-defined material substrate: a per-cell registry of located substances sitting on the physics floor, sibling to the value substrate (Part 21), the semantic substrate (Part 33), and the institution-function substrate (Part 36). The mechanism is fixed Rust; the membership is data and grows with the world at zero code cost. Then wire the cascade so it emerges.

The hard constraints on the shape:

- **No per-race anything.** No `Material{Rock,Soil,Ore,...}` enum, no recipe table, no `build_type` catalog, no carry table, no `flammable` boolean, no `sheltered=true` tag, no "miner"/"builder" role. Each of those is a closed list authored one level down, the exact defect the manual's Principle-8 gate names.
- **Matter kinds are `Substance` rows in TOML** off the existing physics registry (`physics/lib.rs:408,585`). A cell's mechanical properties are DERIVED by reading the registry, never stored as authored scalars on the tile, mirroring how `ResourceField` already derives axis-presence from a `HomeostaticRegistry` rather than tagging a tile (`locomotion.rs:233-248`).
- **Every behaviour is a physics contest, not a branch.** Extraction reads force vs the cell's derived hardness (`cut_penetrate`/`fracture_onset`). Carry reads grown `Body::strength` vs a load's derived weight (`laws::weight`). Fire reads fuel-mass vs ignition temperature (`combustion`). Capability is a pure read of (geometry, material) whatever the provenance, so one dispatch serves the body, the tool, and the target the tool works.
- **One axis vocabulary.** The per-cell substance mixture keys off floor ids (Substance ids, `mat.*` and `bio.*` axes) so the mechanical floor and the biology floor read the same data-defined vocabulary. Do not fork the world into two incompatible per-cell vectors.
- **One identity space.** A tool or relic mints a `StableId` from the same `Registry` a being uses (`core/id.rs:87`), so legend and provenance attach exactly as they do to a being. Do not fork identity.

The feasibility is favorable because most of the substrate is already standing: the density/hardness axes, the cut/fracture/combustion laws, the Coord3-keyed hashed field, the `Substance` registry, the grown strength, the `StableId` identity, and the affordance-by-derived-capability gate all exist. The unbuilt pieces are the tile-to-`Substance` material layer, the inventory binding, vertical adjacency, the worked-object wiring into the run, the fire field, the terrain-delta field, and the hash extensions that carry all of them.

---

## 3. THE ORDERED, INCREMENTAL BUILD PLAN

Use the slice cadence the anatomy and harm-learning arcs proved: each slice is READ-then-WIRE. The READ half adds the substrate, the derivation, and the tests OFF the run path and hash-neutral, so every existing scenario stays byte-identical. The WIRE half, which changes `state_hash` by folding a new field or pointing a live contest at new matter, is isolated into its own slice and made OPT-IN (an off-by-default scenario flag or an empty default field), so a scenario that does not enable the new matter replays bit-for-bit. Nothing hash-changing lands in the same slice as its substrate.

The eight cascade items are in strict dependency order. Everything downstream reads a located material with a derived hardness, weight, and fuel value, so item 1 comes first and the rest layer onto it.

### Item 1 (base): SUBSTANCE IN THE GROUND

**Mechanism.** Each z-cell carries a substance MIXTURE keyed by physics `Substance` id (a `MaterialField`, a Coord3-keyed sibling of `ResourceField` of the same shape), and its hardness and density are DERIVED by reading `PhysicsRegistry`, never stored on the tile. Worldgen fills the empty z-layers with rock/soil/clay/ore/salt/water-table as `Substance` rows.

**Substrate it builds on.** `ResourceField`'s exact shape (`locomotion.rs:199-344`): sparse `BTreeMap<Coord3, Composition>`, `set`/`take`/`total_supply`, canonical `hash_into`, and the registry-read derivation pattern at `locomotion.rs:233-248`.

**Seam hardened.** Kinds are `Substance` TOML rows, not an enum. Properties are read from the registry, not authored scalars. One axis vocabulary shared with the biology floor.

**Determinism / P8-9-11 guard.** The `MaterialField.hash_into` walks canonical (Coord3, substance-id) order and folds into `state_hash` beside `ResourceField` at `runner.rs:2115`. No new RNG; population is a deterministic worldgen scan. P11: every property is data-derived. P9: no identity is read.

**Emergent milestone.** None yet by itself; this slice is the floor. Its READ half is hash-neutral (field exists, no run-path consumer), its WIRE half folds it into `state_hash` behind the opt-in flag.

### Item 2: RESOURCES AS OBJECTS

**Mechanism.** A discrete chunk of matter (a detached tissue part, a mined ore lump, a fallen log) is a matter object carrying a `Substance`/`Composition` vector and a `StableId`, rather than a field decrement. HARVESTING is a read of a species' organ/segment tissue composition (`TissueComposition`/`Segment.material`, already carried on bio+mat axes) that detaches a PART into such an object; its use derives from that tissue (edible where it reads `bio.energy_density`, a fibre or timber where it reads a structural axis), blind to a "fruit"/"wood" tag.

**Substrate it builds on.** Item 1's material layer, the tissue composition on the anatomy floor, and the `StableId` + `Registry` identity (`core/id.rs:33-59,87`).

**Seam hardened.** A resource's kind is a reading of its composition axes, never a tag; the same detach path serves a berry, a bone, and a plank.

**Determinism / P8-9-11 guard.** Detach is an id-sorted sequential draw off hashed state, no fresh RNG (the `ResourceField::take` discipline, `locomotion.rs:302`). P9: no "fruit" fact is read.

**Emergent milestone.** A being with a food reserve near floor detaches an edible part from a plant and it exists in the world as a carriable object graded by its own tissue composition.

### Item 3 (the hinge): CARRIABLE MATTER

**Mechanism.** A net-new inventory substrate binds a carried load (a `Substance` id plus a quantity, whose weight is density times volume through `laws::weight`, `laws.rs:517`) to a carrier, bounded by the grown `Body::strength`. Pickup and drop are an id-sorted sequential walk. Carried weight feeds back into locomotion cost, so an over-laden being slows.

**Substrate it builds on.** Item 2's objects, `Body::strength` (`body.rs:790`, which drops when a limb is lost), `laws::weight`, and the `take` pattern.

**Seam hardened.** The carry limit is grown `Body::strength` vs the load's derived weight, never a per-race carry table. This is the hinge: it is what turns a located resource into a hoardable, tradeable, buildable thing, so mining, gathering, hoarding, trade, and building all depend on it and it precedes all of them.

**Determinism / P8-9-11 guard.** No new RNG; the inventory binding folds into `state_hash` in canonical carrier-id then substance-id order. Fixed-point weight is already checked-mul/div. GPU floor untouched (CPU world state).

**Emergent milestone.** A being picks up a detached object, its movement cost rises, it sets the object down elsewhere, and the drop persists at a Coord3 with its identity intact.

### Item 4: CRAFTING

**Mechanism.** Run the SAME `derive_capabilities` dispatch (`capability.rs:440`, which takes geometry and material CLOSURES and is source-agnostic) over a worked object's (FormDef geometry, `Substance` material), so its FUNCTION derives: a sharp hard edge reads a positive CUT/PIERCE. This unifies the worked-object structural evaluator (`eval.rs:304`, bend_stress + fracture_onset into a viability) with the capability read. SMELTING is the thermal floor (`combustion` energy plus `phase_change_energy`) transforming one substance content-id into another. The made tool then supplies the geo/mat closures that read a HIGHER capability than the bare limb, so the tool multiplies the affordance that made it.

**Substrate it builds on.** The provenance-blind `derive_capabilities` dispatch, `eval_leaf`, the `Substance` registry, and the thermal floor kernels.

**Seam hardened (the load-bearing one for the whole arc).** `CapabilityRefs.target_hardness` and `target_specific_cut_energy` (`capability.rs:232-237`) measure every part against ONE reserved GLOBAL reference target (a stand-in "reference prey hide"). That is a fixed reference sitting where a per-matter target belongs. Harden it so the contest reads the TARGET's material axes from the substance/ground registry (a second `mat` closure or a target `Substance` handle), so mining flint vs granite, and cutting hide vs wood vs stone, diverge on substance DATA alone, never a code branch. FRACTURE/CUT enters `FunctionLawRegistry` as a data entry bound to a floor kernel, not a new match arm.

**Determinism / P8-9-11 guard.** The capability read is already pure fixed-point (`capability.rs:108`); threading a target material keeps it pure. Any "recipe" must be a transformation the thermal/fracture floor licenses, not an authored lookup. Every new constant is surfaced reserved-with-basis, fail-loud.

**Emergent milestone.** A being works a hard substance into a form whose derived CUT capability exceeds its bare limb's, then uses that object to clear a hardness the limb could not.

### Item 5: MODIFIABLE TERRAIN

**Mechanism.** A per-cell EARTHWORK DELTA field (a Coord3-keyed, canonically-hashed sibling of `ResourceField`) that beings write via a dig/deposit affordance PAIR, read by the physics as `effective_elevation = worldgen_elevation + delta`. Digging is a force-vs-hardness fracture contest over the ground material's derived hardness; the removed matter is conserved as an item-3 carriable load and deposited elsewhere as a mound. The hydrology's downhill target recomputes from the effective elevation (retiring the one-time precompute at `environ.rs:281`), so a dug pit pools water, a mound sheds it, a dam raises a hydrostatic column.

**Substrate it builds on.** The `ResourceField` field shape, the `Terrain` trait's `passable`/`cost` seam (`locomotion.rs:190-197`), the fracture and hydrostatic laws, `compute_downhill` (`environ.rs:605`), and the `ConservationRegistry`.

**Seam hardened.** The named forms (pit, well, shaft, mound, wall, dam, terrace, channel) are NOT a build-type enum: they emerge from ONE primitive pair, remove-matter-here / deposit-matter-there, under the geometry the affordance and physics permit. A wall is deposited matter impassable-by-height through `Terrain::passable`; a dam is deposited matter across a routing path. The named forms are descriptions of the delta-field configuration.

**Determinism / P8-9-11 guard.** The delta field is a pinned integer fold in canonical order. The real feasibility risk is the downhill recompute: making routing respond to digging means recomputing the lowest-neighbour target from the effective-elevation field, which must stay a deterministic scan with the fixed up-down-left-right tie-break (`environ.rs:605-631`), no float, recomputed incrementally where a delta changed, portable to a `#[cube]` stencil so the GPU-canon-pin is untouched. CONSERVATION: removed-here equals carried equals deposited-there, registered as a `ConservationRegistry` projection so a dig/deposit cannot silently create or destroy earth.

**Emergent milestone.** A being digs a pit, the removed earth appears as a mound elsewhere, and rain routing changes so water pools in the pit, all with conservation intact.

### Item 6: LIVE FIRE

**Mechanism.** A per-cell FIRE field sibling to water/salt/productivity in `EnvironFields`, advanced inside `step_field` (`runner.rs:1371`) in the pinned order alongside the temperature `Field`. Per cell each tick: read the tile's FUEL as a composition axis measured by mass off the material layer (per-substance fuel_value/oxidiser_demand/ignition_temperature/incombustible-residue as floor data); run the existing `combustion` (`laws.rs:833`) over (fuel mass, ambient oxidiser, the same-tick temperature value as source_temperature); consume fuel with `ResourceField::take`; convert energy to a temperature rise via `sensible_rise` written into the temperature `Field`, whose diffusion stencil carries heat to neighbours. SPREAD EMERGES: a hot cell raises a neighbour past its fuel's ignition gate next tick. EXTINCTION EMERGES: a cell out of fuel or cooled below ignition falls out of the gate. Light emits via `wien_peak` (colour) plus `inverse_square_falloff` (reach) into a light quantity for the communication-reach arc.

**Substrate it builds on.** The whole fire physics set already exists as closed-form fixed-point laws (`combustion`, `reaction`, `radiant_emission`, `wien_peak`, `inverse_square_falloff`, `conduction`, `convective_flux`, `sensible_rise`, `phase_change_energy`), the `Field` diffusion stencil and `Field::hash`, and the fuel-bearing material layer from items 1 and 2.

**Seam hardened.** FUEL is a substance axis measured by mass off floor data (fuel_value, oxidiser_demand, ignition_temperature, incombustible-residue are already `combustion`'s parameters), never a "flammable" boolean or a closed list of burnables, so peat, oil, dry grass, coal, and corpse fat are data rows. A cell burns because `combustion` reads a raw fuel-mass-vs-ignition-temperature contest, never because it "is a forest" or a being "is an arsonist."

**Determinism / P8-9-11 guard.** The fire step couples the temperature `Field`, the `ResourceField`, and the new fuel/fire/light fields within one tick, so slot it into `step_field` in the pinned canonical order, declare its resource access (RES_FIELD plus a new RES_FUEL) so the deterministic scheduler serializes it, and fold every new field into `state_hash` via the `environ.rs:556` discipline. Spread and extinction need NO RNG (diffusion plus the ignition gate is fully deterministic). Any future stochastic ignition (lightning) is a `DrawKey`/Phase draw keyed on (cell, tick, a registered FIRE phase), never wall-clock. CONSERVATION: combustion sends mass to gas, so the volatile fraction must be accounted to an air/sink class inside a registered projection, or the total-matter guard fires (correctly).

**Emergent milestone.** A being with a low temperature reserve strikes a friction/percussion spark on a fuel-bearing tile; the tile ignites, warms it, spreads to a neighbour, and burns out when the fuel is spent, with no scripted spread rule. A wildfire is the same field with no being in the loop.

### Item 7: SHELTER / MICROCLIMATE

**Mechanism.** A built structure or dug burrow writes a per-cell PHYSICAL field modifier: it attenuates the being's effective body-to-field thermal exchange at that cell (a per-cell insulation term multiplying the rate in `phase_body_exchange`, `runner.rs:1597`) or buffers the cell's relaxation toward the harsh baseline (`Field::step`), so a being metabolizing inside loses heat slower and its TEMPERATURE reserve stays in band through a night that kills an exposed being. It builds BECAUSE its TEMPERATURE reserve is falling toward its floor AND it bears a dig/build affordance, exactly as it moves when its food percept is bad.

**Substrate it builds on.** The full thermal half-loop already built: the TEMPERATURE homeostatic axis (`homeostasis.rs:228`) with a real death floor, the thermoreceptor and thermotaxis (`runner.rs:506-517,415-427`), `phase_body_exchange` (`runner.rs:1584`), and item 5's delta/modifier field shape.

**Seam hardened.** The shelter effect is a per-cell PHYSICAL quantity (an exchange-attenuation scalar) in a hashed sibling field, never a `sheltered=true` tag. The buffering is read by the same `phase_body_exchange` for every being on the cell (a predator wandering into the burrow is buffered too, Principle 10).

**Determinism / P8-9-11 guard.** The modifier field is a pinned integer fold folded into `state_hash`. The build fires on a raw thermal-reserve-falling percept plus an available affordance, never a builder role or a race label (the affordance registry already gates on derived capabilities, `homeostasis.rs:535-555`).

**Emergent milestone (the arc's headline blind-audit target).** A cold lineage is observed in a run log to dig burrows and survive a killing night while a warm lineage in the same run builds nothing, with no shelter code reading a race label.

### Item 8: THE MATTER CYCLE

**Mechanism.** Close the loop by wiring the deferred trace follow-on. Give a remains its own substance vector, so death deposits the body's reserves (from `body.rs`/`anatomy.rs` composition) as a corpse `Composition` into the material layer, and `organic_salience`'s exponential schedule becomes the RATE at which that corpse mass is TRANSFERRED into the tile's soil nutrient classes (the enthalpy/mass sums that today enter as zero now carry real substance). Burning deposits an ash `Composition` (the incombustible residue) raising a soil/mineral class and scarring productivity. Eating, mining, and digging deposit waste, tailings, and spoil the same way. The world is scarred by real mass moved, not decremented into the void.

**Substrate it builds on.** The trace decay substrate (`trace.rs`), which already DERIVES salience from the reaction/corrosion kernels but whose wiring into the running world is a documented deferred follow-on (`trace.rs:35-39`, mass/enthalpy sums entering as zero), plus the `ConservationRegistry`.

**Seam hardened.** `DecayLaw` (`trace.rs:77`) is EXPLICITLY a closed enum flagged as an OWNER CALL: the doc reserves the other reading, that if decay laws should be composable or world-authored the way the value and semantic substrates are, it should become a data-defined registry keyed to a kernel by id. A live matter cycle presses exactly here, because combustion, cooking, and smelting are new transform modes that would each become a fourth or fifth enum arm. Harden to a DATA-DEFINED `TransformKindRegistry` keyed to a physics-kernel id, sibling to the value/semantic/institution substrates, so decomposition, corrosion, combustion, cooking, and smelting are data rows over the fixed kernel set. (This is an owner call, so it is FLAGGED per workflow 5b, not resolved unilaterally.)

**Determinism / P8-9-11 guard.** Deposits are id-sorted sequential draws, folded into `state_hash`. The conserved-mass totals are REGISTERED in the `ConservationRegistry`, which is designed so a new reservoir (an atmosphere/volatiles sink, a magic-ash class) is covered the moment it registers, nothing special-cased. An unconserved deposit reads as a nondeterministic leak and the guard fires.

**Emergent milestone.** A being dies, its corpse deposits mass, and over ticks that mass transfers into the soil nutrient classes so a plant grows richer where the body fell, with the total-matter projection flat throughout.

### The riders

**Object identity (rides on items 2-4).** A specific tool or relic mints a `StableId` from the same `Registry` a being uses and gets a location that is a Coord3, a carrier `StableId`, or a pool (`EntityLocation{Loose|Pooled}`, `core/id.rs:33-59,87`). Legend and provenance attach through the unchanged identity substrate; deposited matter reuses the trace decay substrate (`DecayLaw::Static` for a moved boulder or a carved scar, `trace.rs:110`). Do not fork identity.

**Force affordances (ride throughout).** The anatomy arc supplies the derived affordances this arc consumes: a part's function is DERIVED from its own geometry and material via `derive_capabilities`, source-agnostic across a grown `Segment`, a catalog `KindDef`, and a worked object. This arc adds DIG/EXTRACT as a new affordance gated on a new FRACTURE/CUT `FunctionLawId` (a data entry, sibling to PIERCE/LOCOMOTE), whose kernel runs the being's derived force (`whole_body_muscle_force`, `physiology.rs:198`) as a contact pressure against the located matter's derived hardness. Never a "miner" gate.

---

## 4. HONEST LIMITS AND COUPLINGS

This arc does not stand alone. Name the seams to the neighbours so they are not silently assumed.

- **The anatomy arc supplies the affordances.** DIG/EXTRACT, carry, and craft all read grown geometry-and-material capability (`Body::strength`, `derive_capabilities`, `whole_body_muscle_force`). If anatomy has not exposed a limb's force and hardness as a clean derived read, this arc has nothing to point the contest at. That dependency is upstream and firm.
- **The biosphere-into-the-run arc supplies harvestable organisms.** Item 2 (resources as objects) detaches parts from a species' tissue composition. Until the biosphere arc has living plants and animals carrying tissue on bio+mat axes in the run, harvesting has nothing to detach and only the mineral half of the cascade (mining, digging, building) is exercisable.
- **The harm-learning edibility read consumes cooking, not the reverse.** Item 6's fire heats a cell and transforms its `Composition` BEFORE `edibility::assess` (`edibility.rs:285`) measures it, so the harm-learning arc sees a shifted nutrient/toxin tuple with NO new verdict logic. Cooking is a matter transform this arc owns; the safety judgment stays in harm-learning. Do not duplicate the assessment here.
- **The communication-reach arc reads a beacon.** Item 6 emits light via `wien_peak` + `inverse_square_falloff` into a light quantity. That is the signal the communication-reach arc reads as a beacon; this arc produces it and stops there.
- **Mass conservation is a hard invariant, not a nicety.** Every removal, carry, deposit, burn, and decay must balance in the `ConservationRegistry`. Combustion's volatile fraction must be sinked to an air class or the guard fires correctly and stops the run. The guard is a feature: it is what keeps a barren scar honest mass rather than a decrement into nothing.
- **The GPU floor is untouched by the CPU-side world state, but the hydrology recompute reaches it.** Items 1-3, 7-8 are CPU world state and do not touch the GPU-canon-pin. Item 5's downhill recompute must stay portable to a `#[cube]` stencil like the diffusion kernel, and item 6's fire field must keep to the `Field::step`/`ScalarField` closed-form integer-fold posture, so the pin holds.
- **A real topology gap.** `FlatBounded::neighbours` keeps `c.z` (`topology.rs:154`), so there is NO vertical adjacency today: digging DOWN between z-layers is not yet expressible. Adding a vertical neighbour is a topology change that must keep the neighbour walk canonical and the key fold gapless. Treat it as a scoped sub-task inside item 5, not a free assumption.

---

## 5. PRIME CONSTRAINTS AND THE GATE

Non-negotiable, on every slice:

- **Determinism and the 5090 GPU floor.** Every new field folds into `state_hash` in canonical (Coord3, id) order via the `hash_into` discipline (`locomotion.rs:325`, `environ.rs:91,556`). No float in world state, no fresh RNG in any extraction, carry, dig, deposit, or spread; those are id-sorted sequential draws off hashed state. Any stochastic source added later is a `DrawKey`/Phase draw keyed on (cell, tick, registered phase), never wall-clock. New systems declare their resource access so the deterministic scheduler serializes them.
- **Principle 8 (order emerges).** No closed enum, recipe table, or role tag where world content should emerge. Every kind is a data row; every behaviour is a physics contest.
- **Principle 9 (physics may be authored, outcomes may not).** The material floor is authored data; mining, cooking, and shelter are outcomes and stay unauthored. No substrate reads a high-level fact (a race id, a "miner" role, a "flammable" tag) to produce a behaviour.
- **Principle 11 (data-driven).** A hardcoded constant in the path of world content is a defect until it earns its place. Matter kinds and their properties are TOML.
- **Reserved physical constants are surfaced with basis, never fabricated.** The FRACTURE law's reference levels, a harvest detach threshold, the per-substance fuel and ignition parameters, an insulation attenuation scalar, and a decomposition rate are each RESERVED with the basis stated (the yield the mechanical floor already defines, the reaction barrier the physics data carries, the exchange rate `phase_body_exchange` already uses), fail-loud on unset, per the Section 6 blockquote contract. The agent surfaces the number and stops.
- **Tooling gate.** `cargo fmt --all` before every commit (CI enforces `--check` on main), `cargo clippy --all-targets`, and `verify.sh` green. The consensus-roadmap stop hook requires updating `docs/working/CONSENSUS_ROADMAP.md` whenever source or design files change.

**The acceptance gate is behavioural, per emergent behaviour.** Each cascade item's milestone is proven by a BLIND concept-verification on a run log: an independent read of the log must SEE mining, harvesting, crafting, digging, shelter, fire, and the closing matter cycle ARISING from need + affordance + matter, with no scripted branch and no race label in the responsible code path. Packet fidelity matters here (per memory): Section B of the blind packet must be verbatim code, not pseudocode, or the panel converges on phantom findings. A behaviour that cannot be observed emerging in a log is not accepted, however clean the code reads.

---

## RECOMMENDED FIRST SLICE

**Substance in the ground (cascade item 1), READ half first: the `MaterialField` and its registry-read derivation, off the run path and hash-neutral.**

It is the base of the dependency order, and the arc note flags it as such. Every other item reads a located material with a derived hardness, weight, or fuel value: mining reads its hardness, carry weighs its density, crafting works its material, digging fractures it, fire burns its fuel, the cycle deposits into its soil classes. Nothing downstream can be built or observed until a cell knows what it is made of.

It is also the lowest-risk landing. The READ slice adds a `MaterialField` (the proven `ResourceField` shape) populated at worldgen from `Substance` TOML rows, deriving hardness and density by reading `PhysicsRegistry` exactly as `ResourceField` derives axis-presence today, with NO run-path consumer and NO fold into `state_hash`. Existing scenarios stay byte-identical because nothing reads it yet. The hash-changing WIRE slice that follows, folding `MaterialField` into `state_hash` and pointing `cut_penetrate` at the cell's derived hardness instead of the global reference target, is isolated and opt-in, so the first observable milestone (a being clears a rock's hardness and yields ore because it needs metal) lands behind a flag that leaves every prior run reproducible.

Files to open first: `crates/sim/src/locomotion.rs:199-344` (the field to clone), `crates/physics/src/lib.rs:402-447,585` and `crates/physics/data/mechanical_floor.toml` (the substance registry and axes to read), `crates/sim/src/runner.rs:2115` (the fold site), and `crates/world/src/worldgen.rs:31-37,131-137` (the surface-only tile and the `z==0`-only fill to extend down the column).


---

## MATTER-AFFORDANCE GAP ADDENDUM (surveyed on the owner's local models, 2026-07-05)

The cascade above affords GRASP, EXTRACT, and the craft and cut kernels, and plans terrain, fire, shelter, and the cycle. The owner asked what else a creature should be able to DO with a material that none of that affords. A survey (two local Qwen models, split across mechanical, ingestive, body, locomotive, constructive, and social modes) surfaced the following, each held to the one rule (need + grown physics-derived affordance + structured matter, founder-zero, physics-gated) and sorted into materials-core additions to build here versus ones that bridge a neighbour arc.

### Materials-core (add to this arc, in a force-and-manipulation extension after item 4)

Force affordances, the biggest named-but-unbuilt rider, now specified:
- THROW: launch matter as a projectile, a grown impulse over object mass into a ballistic arc, impact stress against the target's fracture hardness. Ranged predation and defence.
- PUSH / DRAG / ROLL: displace a load too heavy to lift, grown force against static and dynamic friction. Heavy transport, terrain clearing.
- LEVER: multiply force through a rigid grown limb on a fulcrum object, torque and moment-arm against a threshold. Breaks bonded matter, precision force.
- DAM / BLOCK: place matter to obstruct a flow, hydrostatic pressure against the block's structural integrity. Water storage and trapping (couples to item 5's hydrology).
- STRIKE-WITH-A-HELD-OBJECT: a carried object multiplies the grown STRIKE against a target, effective mass of limb plus tool into a contact pressure against the target's fracture hardness. The recursive tool-use loop (a tool to make a better tool), and it rides the crafting seam just signed off (the target-material cut read). The survey's top pick, and the one that decouples force from biology.

Other materials-core:
- INGEST-FOR-COMPOSITION: eat matter because a reserve needs it (geophagy for a mineral, salt, or grit a homeostatic axis is low on), the NEED-side complement to harm-learning, which reads the same cell composition to AVOID a harm; this reads it to SEEK what a reserve lacks, closing that loop.
- FLOAT / RAFT: buoyant matter carrying a being over water, Archimedes displacement against total mass. Extends carry to load-bearing platforms.
- BURROW-THROUGH: move through soft matter by displacing it, contact pressure against soil cohesion and yield. Bridges extraction to terrain.
- MIX / KNEAD: a physical, non-thermal combination (mud, mortar, dough), mechanical work against particle adhesion, a transform beside smelt.
- STACK / ASSEMBLE: place discrete objects into a structure held by their own centre-of-mass and friction interlock, distinct from item 5's elevation-field deposit. Nest, cairn, dry-stack wall.
- WEAVE / BIND: join matter by tension and friction (fibre into cordage or a net, knot friction against slip), a topological joining with no thermal or chemical bond. The survey's other top pick; unlocks cordage, nets, traps, textiles, and load-sharing.
- CACHE / HOARD: deposit matter in a concealed cell and retrieve it (needs a place-memory), the physical basis of a larder.
- GIVE / TRANSFER: a load moving from one carrier to another by proximity, a simultaneous release and acquire, the physical seed of provisioning and trade.

### Bridges a neighbour arc (flag, do not build here)
- COAT / WEAR / ADORN (bridges anatomy): apply matter to one's own body to change its thermal, optical, or defensive physics (mud, a covering, pigment). Reads on the being's own covering and thermal exchange.
- CLIMB / GRIP (bridges anatomy): the grip is a grown part; the holds are matter. A locomotion mode over the anatomy read.
- MARK / SIGNAL with matter (bridges communication-reach): a deposited marker or pattern as a non-linguistic signal.
- PLANT / SOW / TEND (bridges biosphere-into-run): place a biological propagule to grow; needs living organisms and germination.

### Placement and acceptance
Most ride the built carry and extract substrate and the anatomy affordances, so they slot into a force-and-manipulation extension after item 4 and before or alongside item 5 (which DAM and BURROW couple to). The most important still-missing are STRIKE-WITH-A-HELD-OBJECT (the tool loop), INGEST-FOR-COMPOSITION (the harm-learning complement), the force affordances as a set, and WEAVE / BIND. Each is gated by the same acceptance as the rest of the arc: a blind concept-verification that the behaviour emerges from need plus affordance plus matter, never scripted, never a per-race table.


### Round 2: a deeper second-order sweep (eight quality-worker lenses, 2026-07-05)

A second survey (cec-worker-quality through eight second-order lenses: matter states, time-delayed transformation, collective handling, matter-as-information, environmental engineering, adversarial, body-repair, and failure) found new affordances beyond the round-1 list, after discarding re-names. Three lenses hit the token cap (states-and-phase lost its verdict; collective-handling degenerated into a repetition loop and yielded nothing usable; failure-and-degradation truncated but its find is clear), so the COLLECTIVE matter-handling angle (cooperative carry of a too-heavy load, relays, coordinated raising) remains unsurveyed and is worth a re-run. The solid new materials-core finds:

- PROCESS-OVER-TIME (cure, dry, ferment, season, weather): a being sets matter aside so PHYSICS transforms it over time, distinct from every immediate transform already listed. Dry to preserve (moisture loss below microbial viability), ferment or seal to convert and detox (bridges biosphere for the microbes), SEASON green wood by thermal cycling so a tool does not fail in use, weather or leach to soften and detox. This is the deliberate use of the transform-over-time kernels, so it ties directly to item 8 (the matter cycle) and the TransformKindRegistry the DecayLaw seam becomes. The standout: the first survey's constructive mode was all immediate, and this whole time-dependent layer (the preservation and seasoning a civilization runs on) was missing.
- GRIND / ABRASE: wear-based reduction by friction (Archard's law), a DIFFERENT physics from the built fracture contest. Grind grain to flour or ore to powder, re-sharpen a worn edge, smooth a surface. Materials-core, and it is the food-processing and tool-maintenance layer fracture cannot express.
- ASSAY (non-destructive probe): read a material's internal soundness or quality BEFORE committing to it, tap a beam or a tool-blank or a food and read the vibration decay (elastic-wave attenuation). A sense affordance that reads matter rather than acting on it, feeding tool, shelter, and food selection.
- TRAP / SNARE: store elastic energy and release it on a threshold trigger, a snare or a deadfall that captures without direct engagement. The passive-hunting layer, distinct from the static WEAVE/BIND.
- FILTER / CLARIFY / WICK: matter's microstructure against a fluid, settle particulates for safe water (sedimentation below the laminar threshold), wick moisture through porous matter against gravity. The fluid-conditioning layer.
- FOUL / CONTAMINATE: deny a rival a resource by fouling it past its usable cohesion. The adversarial resource-denial, and it ties directly to the ingestion-toxicity seam: fouling is deliberately making a resource harmful to eat.

A systemic property, not one affordance: MATTER WEAR AND FAILURE as a driver. Tools wear and break, structures weaken, stored goods spoil, so matter's degradation over use drives a remaking treadmill (a worn tool must be re-ground or remade), which ties to persistent object identity and the matter cycle.

Bridges a neighbour arc (flag, do not build here): body-repair and medicine (POULTICE to draw a toxin transdermally, SPLINT, groom, thermal contact) bridge anatomy; scent-gradient and thermal-trace tracking (reading matter as a passive record) bridge communication-reach.
