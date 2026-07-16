# R-HYDROSPHERE-WEATHER: surface fluids, weather, and erosion (research question)

Status: flagged, not built. Surfaced at the owner's direction (2026-07-16 visible-world buildout: "water or surface fluids and with it weather (that will need research, just surface it) and erosion"). This is the one item on the visible-world list that is not build-to-spec: a hydrosphere plus weather plus erosion is a coupled ocean-atmosphere-climate-surface arc, and it needs a ruling the way the six capstone blockers did, not a guess at the derivation. The owner's standing condition: broadly generalizable, per-world and alien-admitting, deterministic, minimally authored.

## The gap, grounded

The derived world already has: a surface with derived TOPOGRAPHY (the deep-time province relief, now accumulating volcanism and, in flight, impact craters), a derived ATMOSPHERE composition (#40 / #61, A's genesis-forward arc, in progress), a derived stellar flux and per-world attitude (obliquity, orbit, the DiurnalSky), and a secular spectrum on the way (#44, the Milankovitch forcing). What is missing: any SURFACE FLUID (ocean, lake, ice, or an alien fluid), any WEATHER or CLIMATE, and any EROSION that couples the fluids back onto the topography. The relief the impacts and volcanism build is currently never worn down.

## The core question

How does a world's hydrosphere (its surface fluids and their distribution), its weather and climate, and the erosion that couples them back to the surface EMERGE deterministically from the derived inputs (the outgassed volatile inventory, the atmosphere composition, the stellar flux, the topography, the obliquity and orbit), without authoring the fluid, the weather, or the erosion rates? Weather is chaotic, so this carries the same determinism-versus-chaos seam the assembly did.

## Sub-questions

### A. The hydrosphere substrate (which fluid, where, how much), alien-admitting
1. WHICH fluid condenses on the surface is a per-world question: water for an Earth-like world, but methane or ethane for a cold Titan-class world, ammonia or a supercritical fluid elsewhere. The condensing surface fluid should DERIVE from the volatile inventory (the outgassing plus delivery budget the condensation and impact arcs supply) crossed with the surface temperature and pressure (does the candidate volatile sit liquid, solid, or gas at the world's own surface conditions). Reuse the condensation and ice-sublimation substrates (the same phase machinery that placed the ice line) rather than assuming water. The alien is a data row: a methane hydrosphere is the same mechanism keyed on a different volatile.
2. WHERE the fluid sits is the derived topography: fluids pool in the basins the impacts and tectonics carved, to a sea level set by the volatile volume against the hypsometry. HOW MUCH is the outgassed plus delivered inventory minus what escaped (the atmosphere escape the young-temperature and atmosphere arcs already touch).

### B. The energy balance and the hydrological cycle
The surface energy balance (absorbed stellar flux, the greenhouse of the derived atmosphere, the outgoing longwave) sets the surface temperature field, and the temperature field drives evaporation and precipitation (the hydrological cycle) and the ocean and atmosphere heat transport that flattens the equator-to-pole gradient. The question: how much of this derives from the built atmosphere-radiation machinery (the same per-volatile radiation ceilings the young-temperature arc fetched from Lichtenberg) plus the topography, and what is the minimal reserved set (the drag or mixing-length closures that always exist in a fluid model).

### C. Weather and climate emergence (the Chaos-Protocol seam)
Weather is chaotic (Lyapunov-sensitive), exactly the class the assembly ruling forbade integrating as a fixed-point path. So the same CHAOS PROTOCOL should apply: do not integrate the chaotic weather trajectory (a byte-neutrality landmine over deep time), SAMPLE the climate STATISTICS (the stationary distribution, the invariant measure) from the derived boundary conditions, seeded. The CLIMATE (the temperature and precipitation distributions, the circulation pattern, the seasonal cycle from the obliquity and orbit, the ice-age cycles from #44's Milankovitch forcing) is the derived object; the specific weather is spent contingency, never a simulated trajectory. The ruling needed: is the climate the pushforward of the derived boundary conditions (a seeded draw from a derived measure, exactly option 2 of the assembly), and what is the derived measure (a general-circulation statistical closure, an energy-balance-model climatology, or an amortized ensemble the way the assembly amortizes its N-body)?

### D. Erosion, which closes the loop
Fluid-driven erosion (fluvial incision, weathering, glacial carving, aeolian transport already partly scoped in the Gap-and-Residual note) transports mass off the highlands into the basins, so the topography the impacts and volcanism build is worn down and the relief reaches a steady state between building (tectonics, impacts, volcanism) and destruction (erosion). This closes the surface loop and is what makes an old world look eroded rather than freshly cratered. The question: the erosion law (stream-power or its kin) keyed on the derived slope, discharge, and substrate strength, conserving mass into the sediment budget (the Residual Law), deterministic.

### E. The determinism, data-driven, and alien constraints
Deterministic (fixed-point, seeded draws only, no chaotic path integrals). Data-driven (the fluid is the volatile inventory's, never authored water; the erosion law reads the world's own slope and substrate). Alien-admitting (a methane hydrosphere, an ammonia ocean, a supercritical-CO2 surface, or no fluid at all, each a data row). Minimally authored (count the dimensionless groups: the greenhouse optical depth, the obliquity, the land-sea fraction, the erosion efficiency, and bound the authored inputs to that count).

## Couplings

Upstream: the atmosphere composition (#40 / #61, the greenhouse and the escape), the young-temperature and volatile-delivery arcs (the inventory), the deep-time topography (the basins and the relief), the DiurnalSky (the seasons), and #44's secular spectrum (the Milankovitch ice-age forcing). Downstream: the biosphere (life shapes the climate and the weathering, the Lovelock coupling), the visible world (oceans, ice caps, clouds, weather, and eroded terrain in the render), and habitability (the liquid-water, or liquid-fluid, band). The existing `OCEAN_HYDROSPHERE_TO_RESEARCH.md` stub is the seed for the hydrosphere half.

## What I need resolved

1. The Chaos-Protocol ruling for climate (sub-question C): is the climate a seeded draw from a derived statistical measure, and what is that measure (an energy-balance-model climatology, a statistical-dynamical closure, or an amortized GCM ensemble). This is the load-bearing determinism decision, the twin of the assembly's.
2. The hydrosphere derivation (A): confirm the surface fluid derives from the volatile inventory crossed with the surface phase conditions through the built condensation and sublimation substrates, so the alien fluid is a data row.
3. The minimal authored set for the energy balance, the circulation, and the erosion law (the dimensionless-group budget), so the authored inputs are bounded before the build.

The literature to survey and the exact forms are the researcher's to supply, as with the six blockers; this document scopes the question and the couplings so the ruling can be precise.
