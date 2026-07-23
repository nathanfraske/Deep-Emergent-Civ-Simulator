# The body arc: R-BUILD-PHYS, R-WOUND, R-FLUID (scope and autonomous fork decisions)

This is the scope for resolving the physiology/anatomy/wound arc as one dive, produced by a grounding
fan-out over Part 35, Part 20, Part 25.13/25.14, the resolved physics floors (R-PHYS-MECH, R-PHYS-BIO),
and the built substrate (`crates/sim/src/anatomy.rs`, `homeostasis.rs`, `controller.rs`; `crates/physics`).
The owner delegated the fork calls ("autonomously scope and wire all that you can, and audit; I will
review in the morning"), so the decisions below are taken on the on-ideology recommendation and the
project's standing defaults, recorded here for the owner's review. Every one follows the same principle
the arc converges on: the physics is authored, the outcome is measured (Principle 9); membership is data,
the mechanism is fixed Rust (Principle 11); no value is fabricated.

## What the three items are

- **R-BUILD-PHYS** (Part 20, Part 35): the full per-part body promoted from the Part 25.14 anatomy
  vector, and which build stats are primitive versus physics-derived.
- **R-WOUND** (Part 35): a wound without the authored `Wound { Cut, Blunt, Pierce, Burn }` enum, defined
  against a data-driven anatomy.
- **R-FLUID** (Part 35): a body fluid without the authored `FluidKind` enum, its loss and interactions
  data-defined.

The physics they read is already resolved and built: the R-PHYS-MECH material axes (`mat.*`, `therm.*`
in `crates/physics/data/mechanical_floor.toml`) and the closed-form fixed-point laws
(`crates/physics/src/laws.rs`: `contact_pressure`, `cut_penetrate`, `fracture_onset`, `bend_stress`,
`axial_stress`, `shear`, `kinetic_energy`, `impulse`, `thermal_stress`, `sensible_rise`, `combustion`),
and the R-PHYS-BIO composition axes (`biology_floor.toml`). The arc reads three resolved floors and adds
no new one, which is why it was the natural convergence.

## Fork decisions (autonomous, for review)

1. **A wound is measured, not a stored kind (the on-ideology endpoint, design.md line 2027).** The
   `Wound` enum is retired. A wound is the measured consequence of an insult (a force and delivered
   energy over a contact area, of a damage mode) meeting a body part's outer tissue material, computed by
   the resolved floor law and stored as the part's condition change plus a measured outcome tuple
   (penetration depth, severance, fracture, burn severity, vessel breach). No authored "this hit does
   X".

2. **Damage modes are a data registry keyed to a physics measurement (not a closed enum).** A
   `DamageMode` carries which measurement family the engine runs against the tissue: penetration
   (`contact_pressure` then `cut_penetrate`, cut and pierce differing only by the insult's contact area,
   the crush-versus-pierce geometry difference), fracture (`fracture_onset` on the delivered energy),
   or thermal (`thermal_stress` / `sensible_rise`). The mode set is data and grows with the world; the
   grounded members are cut, pierce, blunt, burn, and the fantasy members (corrosion, freeze,
   disintegration, curse) are registry entries whose measurement is a reserved floor extension (an
   audited Principle-9 addition, not built tonight). The measurement family is fixed Rust, the same
   pattern as the affordance registry: membership is data, enactment is physics.

3. **A body fluid is a composition, its volume a conserved stock (R-FLUID).** A fluid is a data-defined
   `FluidKind` with a composition vector over the biology floor axes (blood, ichor, sap, none), a
   volume as a Part 15 `Stock` (so bleeding is exact loss and refill exact gain), and a critical
   fraction. Bleeding is fluid loss from a wound that breaches a part's vessel, at a rate set by the
   breach severity less a reserved clot rate. Death is a pool past its critical fraction (Part 35, the
   existing rule). Ignition rides the resolved `combustion` law over the fluid's composition; corrosion
   waits on wave-2 chemistry (reserved); clotting is a reserved rate.

4. **Integrity is derived, never stored (Part 35's "no competing truth").** There is no separate health
   scalar. `Body::integrity()` is the aggregate of part condition and fluid levels, and `Body::is_alive()`
   is the canonical liveness (no vital part destroyed, no fluid past critical). The R-BEHAVIOR-EVOLVE
   controller reads integrity as a percept refreshed from the body each tick, a derived mirror rather
   than an independently-evolving reserve.

5. **Temperature is a two-sided homeostatic band (a mechanism extension).** The Stage-1 homeostatic axis
   carried only a low death floor; temperature adds an optional high floor, so a body dies of cold below
   the band and of heat above it. The metabolic heat production is buildable; the environmental thermal
   exchange (conduction to and from the climate and medium through the resolved thermal laws) is the
   reserved coupling, since it needs the located world.

6. **Build stats: raw morphology primitive, mechanical stats derived (R-BUILD-PHYS).** The genome build
   channels (size and the raw morphology) stay primitive genetic setpoints; the mechanical stats
   (effective strength from muscle cross-section and material yield, reach from the limb segment lengths
   via the resolved `reach` law) are read from the per-part body and the floor rather than authored,
   the same measured-from-physics move as edibility and the wound.

7. **A strike closes the loop.** A `STRIKE` affordance gated on the natural-weaponry anatomy
   (`MorphCategory::Weapon`), directional, whose enactment forms an insult from the weapon part's
   material and edge (the contact area) and the strike's kinetic energy, applied to a target's body part
   through the wound measurement, so predator-prey behaviour (strike, wound, death) becomes real and
   evolvable under R-BEHAVIOR-EVOLVE.

## What ships and what is reserved

Ships (built and tested against the resolved floors): the per-part `Body` promoted from the anatomy
vector with layered tissue materials; the damage-mode registry; the measured insult-to-wound over the
penetration, fracture, and thermal laws; the fluid registry, volume stocks, and bleeding; `Body::integrity`
and `is_alive`; the two-sided temperature band; the strike affordance and its insult path; derived
mechanical stats. Reserved (surfaced with basis, not fabricated): the tissue material property values
(from real datasheets, the floor's provenance discipline), the damage caps, the clot rate, the heal
rate, the fluid critical fractions, the strike energetics, and the temperature band and metabolic-heat
rates. Blocked (named, not faked): the environmental thermal exchange (needs the located world and a
medium/field floor), corrosion and full fluid chemistry (wave-2), and the digestion model that
R-BIOSPHERE deferred here (couples to the consumer physiology). These are the honest limits carried into
the consolidation.
