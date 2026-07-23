# World substrate readiness (2026-07-08)

A floor-versus-requirements gap analysis for the canonical world roster, run as a blind 3-agent workflow
against the actual physics/biology floor and verified against source (file:line). The rule: build what the
floor supports as data; FLAG missing substrate rather than fabricating a fixture (owner directive). This
document is the roadmap for which worlds are buildable now and which need a substrate arc first.

## Verdict by world

- **Mirror (Earth 1:1): buildable now.** Earth-triad worldgen (`crates/world/src/worldgen.rs`), the data-driven
  `BiomeSet` registry (`terrain.rs`), and the standard floor cover it; only dial magnitudes are reserved. This
  is the owner-gated control.
- **Tempest (grounded, dials cranked): buildable now.** Same substrate as Mirror; only the `.high` dial
  siblings need setting.
- **Crucible (war emerges from scarce contested basins): substrate runs, DEFINING FEATURES UNBUILT.** The
  stack I have been developing against (salt toxicity, greenwell oilseed food, discovery loop, recurrent
  controller, experiential-conviction) is real and runs. But two design-defining pieces do not exist:
  (1) worldgen is scenario-BLIND (`run_world.rs:1838-1839` builds `WorldgenParams::dev_default()` /
  `BiomeSet::dev_default()` regardless of scenario), so the greenwell-basins-in-a-lethal-waste geography does
  NOT generate; Crucible runs on plain Earth terrain, its scarcity approximated only through dials.
  (2) `conflict_pressure` (`value.rs:525-537`) has ZERO run-loop callers (only its own test), so "war as the
  emergent equilibrium" has no mechanism consuming it. So a first Calibrated Crucible run is honestly
  Crucible-the-substrate on generic terrain with cranked dials, short of Crucible-the-design until a
  patchy-basin terrain mode and a raiding/war mechanism land.
- **Venus (hot dense CO2, sulfuric-acid clouds): partial, three flagged gaps.** The axis ranges do not clamp
  Venus (`therm.temperature` to 100000 K, `fluid.driving_pressure` to 100 MPa, `mat.density` to 23000), and the
  medium substrate is data-driven (a dense hot CO2 medium is a data row). MISSING: (1) the greenhouse/radiative
  energy-balance kernels EXIST in the physics floor (`law.radiant_emission`, `wien_peak`, `radiative_equilibrium`,
  `optical_depth`) but have NO consumer in `crates/sim` or `crates/world`, so 737 K cannot emerge from CO2
  opacity (it would have to be hand-authored); (2) no coupling from a corrosive ambient medium to organism
  tissue harm (an acidophile immersed in H2SO4 has no path to bodily harm; the toxin path doses only through
  ingestion + the salinity grazing cycle); (3) the chemosynthetic source needed for a cloud ecology has no
  field kind (see the AbioticField gap below).
- **Europa (ice shell over liquid ocean, tidal-heated, chemosynthetic): blocked on two majors.** (1) No
  volumetric/z-stacked medium or terrain: `TileMap` only populates z=0 (`worldgen.rs:152-215`), `MediumField`
  assigns one medium per (x,y) column (`medium.rs:206-234`), so ice-over-ocean-over-seafloor (an inherently
  three-layer structure) is unrepresentable. (2) No tidal-heating law and no orbital/tidal axes
  (`celestial.rs` carries only orbital and rotation period; no eccentricity, primary mass, Love number, or
  dissipation), so Europa's ocean cannot be kept liquid by its actual physics. Also: no redox-couple
  energy-yield mechanism, and the hydrostatic-pressure kernel (`laws.rs:942-958`) has no caller (no located
  depth for a submerged being).
- **Arcanum + Confluence (magic): blocked, the entire magic system is unbuilt.** `MagicLaws` / `ManaSource` /
  `CostModel` / `LimitModel` / `MagicalTradition` (design Part 34) exist only as pseudocode. In the crates,
  "magic" is a bool posture flag, a cosmetic mana-sac organ, and an UNUSED genome channel
  (`ImbuedChannel::MagicAffinity`, zero downstream consumers). There is no mana field. Running `--scenario
  arcanum` today produces a grounded, non-magical world identical to Mirror.

## Cross-cutting gaps (block more than one world)

- **The scenario name does not yet shape world STRUCTURE.** `run_world` builds `WorldgenParams`/`BiomeSet`/
  `GenesisParams` with `::dev_default()` regardless of scenario, and never reads `scenario.magic.laws`. The
  scenario-to-Calibrated-World loader must make worldgen, biomes, and magic scenario-dependent, not only the
  numeric dials.
- **`AbioticField` is a closed 3-variant enum {Light, Water, Soil}** (`environ.rs:325-333`). Arc 5 T1 made the
  AVAILABILITY of a source data-driven, but the field KIND a source binds to is still closed, so a
  chemosynthetic (Venus/Europa), geothermal/redox (Europa), or mana (Arcanum/Confluence) source has nothing to
  bind to. Opening this enum to a data-defined field-kind registry unblocks the alien-energy worlds at once.

## The buildable-now split

- **Build now (calibrate + wire features + run):** Mirror (owner-gated), Crucible-substrate, Tempest.
- **Flag as substrate arcs (do NOT fabricate):** the greenhouse/radiative-balance wiring (Venus), the
  corrosive-medium-to-tissue harm coupling (Venus), the volumetric z-stacked medium (Europa), the
  tidal-heating law + orbital/tidal axes (Europa), the data-defined abiotic field-kind registry
  (Venus/Europa/Arcanum), the redox-couple chemosynthetic energy mechanism (Europa), the magic system Part 34
  (Arcanum/Confluence), and Crucible's own patchy-basin terrain mode + a raiding/war mechanism consuming
  `conflict_pressure`.
