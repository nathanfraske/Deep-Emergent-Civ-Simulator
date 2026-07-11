# Geodynamics substrate: the points needing build-out (research surface)

Consolidated surfacing (owner request, 2026-07-11) of every geodynamics point where a mechanism wants to derive from solid-planet physics but the substrate is not built. Two layers: the DEEP INTERIOR, surfaced this session by the planetary and stellar physics map and largely NOT covered by the existing surface-geology arc, and the SURFACE GEOLOGY, already scoped in `GEODYNAMICS_ARC_PROPOSAL.md` and `GEOLOGY_ARC_PACKET.md` (R-TECTONICS). Plus the couplings and a consolidated per-world-data research list at the end. Everything here is a research target; nothing is fabricated.

## Layer A: the deep interior (new this session, not in the surface arc)

**A1. Interior structure and the equation of state, giving radius and surface gravity.** Derives `R(M, {x_i})` and `g` from hydrostatic equilibrium plus a cohesive-energy EOS, along with the density profile and the core-mantle-ice differentiation. Feeds everything that reads `R` or `g`: Jeans and hydrodynamic escape, the orbit, the eruption threshold, buoyancy, shear stress. Status: assumed, not built; the planetary map removed radius from the contingency vector on the premise that this derives. Research: the EOS forms (Birch-Murnaghan or Vinet), the iron, silicate, and ice phase boundaries and bulk moduli, the differentiation sequence.

**A2. Radiogenic heat budget and interior thermal evolution.** Derives from the isotope column of `{x_i}` (uranium, thorium, potassium) through the decay chains into the core and mantle heat production over deep time, the secular cooling history that powers the whole engine. Feeds the dynamo (A3), volcanism (B2), the tectonic vigour (the Rayleigh regime), and the TIME evolution of all of it. Status: the surface arc floors an internal-heat-production axis and a radiogenic decay law for the mantle, but the CORE budget and the primordial-versus-radiogenic split are not covered. Research: the isotope abundances (a chondritic anchor, per-world), the decay constants (floored), the primordial-versus-radiogenic heat partition.

**A3. The core dynamo and the magnetic field.** Derives core convection on or off (the core-mantle-boundary heat flux against adiabatic conduction), the field magnitude (Christensen-Aubert buoyancy-flux scaling), the dipolar-versus-multipolar geometry (rotation), and the magnetopause standoff (pressure balance). Feeds aurora, space-weather blackouts, Carrington-class events, and the escape-CHANNEL selection (per the corrected map seam, the field selects channels rather than shielding with a known sign). Status: entirely new this session, not in the surface arc; the chain is known, the residue is `k_core`. Research: the core iron conductivity `k_core` (20 to 150 W/m/K, the Pozzo versus Konopkova disagreement and the young-inner-core paradox), the Christensen-Aubert prefactor, the convection criterion.

**A4. Tidal dissipation and the quality factor Q.** Derives the despin rate (scaling as `a^-6`) toward locking or a spin-orbit resonance, the tidal heating (an Io or Europa interior heat source), and resonance capture. Feeds rotation-rate evolution (which gates the Held-Hou climate), the interior energy budget of an ice moon's ocean, and obliquity and spin evolution. Status: the surface arc notes an ice moon's internal heat coming from a tidal source, but the tidal-Q substrate itself (the despin law, the heating law, `Q`) is from the map and unbuilt. Research: `Q` for rock and for ice, the frequency dependence, the Maxwell or Andrade rheology.

## Layer B: the surface geology (already scoped; research its per-world data)

Pointer: `GEODYNAMICS_ARC_PROPOSAL.md` scopes this in five dependency-ordered arcs, and the headline is that the per-cell physics is largely floored (thermal buoyancy, Archimedes isostasy, the conduction geotherm, the strength axes, the solvent cycle, radiative equilibrium); the work is unfreezing the elevation into a resident field, a few source terms and rheology kernels, and the deep-time solver. The research targets in it:

**B1. Mantle convection and plate tectonics.** The reference creep viscosity (mantle, crust). The tectonic regime (mobile-lid, stagnant-lid, or no tectonics) EMERGES from the Rayleigh number rather than being authored.

**B2. Volcanism and orogeny.** The Clapeyron solidus slope per material; the partial-melt-fraction kernel.

**B3. Erosion and weathering.** The per-column LITHOLOGY field is the load-bearing prerequisite (erodibility derives from it, not from a bare height scalar); per-substance grain size and density; the mineral-weathering law.

**B4. Hydrology.** The stream-power exponents `m` and `n` (near 0.5 and 1, Whipple and Tucker); the solvent as a per-world reference substance (density, viscosity, surface tension, latent heat, boiling point); the fixed-point fractional-power primitive (an unresolved determinism gate).

**B5. Biomes and climate zones.** The lapse rate (`g/c_p`, derived), the prevailing-wind field, orographic precipitation; biomes as emergent niche-fit assemblages (the authored biome classifier demoted to display).

## Layer C: the couplings and seams (identified, not wired)

**C1. Outgassing to the atmosphere (R-COEVOLVE).** Volcanic degassing sources the atmosphere and the volatile inventory, which gates the carbon-dioxide thermostat and the escape budget. The arc has the volcanic source; the degassing-to-composition-to-climate loop is the seam.

**C2. The carbonate-silicate (silicate-weathering) thermostat.** The deep-time climate stabiliser (temperature-dependent carbon-dioxide drawdown by weathering, returned by volcanism), needing continents, tectonics, and the weathering law. The long-term habitability feedback.

**C3. Lithology-derived fertility to the biosphere.** The mineral-weathering law over the MaterialField gives soil nutrients and hence productivity (a non-limiting interim today). Flagged in MORNING_REVIEW.

**C4. Named seams from the arc.** Ores by province (Part 12, keyed to boundary type), tidal heating on a moon (`celestial.rs`), volcanic winter (Part 18), and the R-CATASTROPHE remainder (GreatFlood, Meteor).

## The consolidated per-world-data research list (each to be cited, none fabricated)

Interior (Layer A): the EOS moduli (Birch-Murnaghan or Vinet); the radiogenic isotope abundances (uranium, thorium, potassium, chondritic anchor); the core iron conductivity `k_core` (20 to 150 W/m/K); the tidal `Q` (rock, ice); the Christensen-Aubert dynamo prefactor.

Surface (Layer B): the mantle and crust reference creep viscosity; the Clapeyron solidus slope; the per-substance grain size and density; the stream-power exponents `m` and `n`.

Stellar boundary: the stellar wind mass-loss history `M_dot_star(t)` (Wood's astrosphere scaling).

A universal law-floor constant, NOT owner data: the critical Rayleigh number (near 1e3), sibling to sigma and R.

## The through-line

The deep interior (Layer A) is the root. The isotope column and the composition vector feed the thermal budget, which drives the dynamo, the tectonics, and the outgassing, which write the magnetic field, the topography, and the atmosphere, which the climate, the biosphere, and the technological ages ride on. The surface arc (Layer B) is the visible half; the deep interior (Layer A) is the half this session's physics work showed is load-bearing for the atmosphere, the magnetosphere, and the escape budget. Research the interior data (A) and the surface data (B), and the whole solid-planet-to-atmosphere-to-life chain derives.
