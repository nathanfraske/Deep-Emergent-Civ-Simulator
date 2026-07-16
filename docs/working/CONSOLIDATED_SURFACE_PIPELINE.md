# The consolidated derivation-and-surface pipeline (researcher ruling, 2026-07-16)

This is the single pipeline the surface arcs build against. It supersedes the four separate rulings (coarseness, metallicity, provenance, resolution). Recorded here as the standing reference; the researcher's own prose is preserved in the session record, this is the working consolidation in the project customs.

## Three steers before code (each is one afternoon from being built wrong)

1. The fine grid is where rendering SAMPLES, never where physics WRITES. Physics writes fields and rows at their own derived scales; the fine display grid is only the memoized sample cache of `Sample(lat, lon, level_of_detail)`.
2. The flexural middle is the wavelength band the eye reads as terrain, and it was missing from the first plan. It fills the gap between the coarse convective provinces and the fine crater rows. Its own arc (slice 5), fetch-gated on Turcotte-Schubert.
3. Decoupling impacts from provinces must keep the large-basin feedback (the cross-scale write rule): a process writes into a field only at or above that field's derived scale, so basins at or above the convective wavelength still feed thermal and province state while everything smaller writes rows and regolith.

## The stages

Stage 0, the vector. The metallicity fix is wiring, not design: [Fe/H], the abundance scatter draws, C/O, alpha enhancement, the s and r mix, the per-disk Mdot0, and the epoch already live in the system-draw layer. `Fixed::ONE` dies by connecting the draw that exists; no metallicity knob is created. Conditioning line: [Fe/H] scales amount, the scatter draws change kind, Mg/Si selects the silicate family, iron fraction selects Mercury-class density, C/O selects the roster branch. Convicting bodies: iron worlds and carbon stars. The C/O guard goes live at the next stage.

Stage 1, the disk realization. Sigma_gas(r,t) and T(r,t) from Mdot0 under the alpha closure with the wind-driven family as the declared band; the condensation roster at local state gives Z_solid(r) and the solid chemistry; their product is Sigma_solid(r). The FORMATION ERA stops being a reserved rate and becomes the disk's own clock (condensation-front timing out of the T(r,t) evolution), which also retires the R-YOUNG interim formation-time axis (one wire closing two flags). The MMSN is demoted to the solar-instance hindcast pin (a reconstruction-grade band, Hayashi digits fetched, attached to the Mirror only); the universal chain's normalization stays population-derived and is never multiplied by the MMSN.

Stage 2, embryos and assembly. Isolation mass on (b, Sigma_solid, a, M_star), b carrying its oligarchic branch condition; the standing generator on the Petit surface; validation is zone-scoped hindcast after the wiring lands (terrestrial multiplicity against 3.6 +/- 0.8, count reported never gated; the 20-over-[1,30] arithmetic is a consistency check never a target).

Stage 3, per-world state. Structure from the EOS on the derived assemblage (where BULK DENSITY stops being a fixture consequence and becomes composition-conditioned); R-YOUNG on the stage-1 clock; the #73 giant race per annulus; moons as the three-branch dispatch with a tidal-survival post-condition on all branches (CPD compositions from the engine's own condensation run in the circumplanetary environment); #44 secular modes feeding climate.

Stage 4, the surface. A superposition of representations, each at its own derived scale, NO global physics raster at any resolution. Resolved fields live at the convective wavelength (`provinces_across` correctly scoped to that ONE field, planform a seeded draw with the onset-aspect regime band declared). The FLEXURAL response fills the middle: rigidity from banked moduli and the derived elastic lid over density contrast times the world's own g, an analytic Green's-function convolution over the load list (Turcotte-Schubert, no free constants); a thin-lid young world and a thick-lid old world flex at different wavelengths from the same formula. DISCRETE OBJECTS ARE ROWS, never rasters: the bombardment writes crater rows (position, diameter, age) at true derived sizes, the renderer stamps them analytically at whatever the viewport resolves, retiring the main.rs coarse-grid limit, keeping counts as queries; the row list carries individuals down to the finest resolvable scale and the statistical tail below derives its roughness from the world's own size-frequency distribution at saturation (not a generic fractal). Crater morphology conditions on gravity and target class (Moon vs Earth g-scaling, icy Galileans target shift); the production function conditions on the local impactor reservoir, and until the debris-fed reservoir stands the lunar-calibrated production function ships as a tagged class instance with that conditioning named (Neukum-Ivanov on the fetch list). The CROSS-SCALE WRITE RULE holds it together: basins at or above the convective wavelength still feed thermal and province state, everything smaller writes rows and regolith. Below everything, the regime-conditioned texture floor (non-canon, one-way, named seed), now mostly serving resurfaced regimes since crater rows self-generate the cratered spectrum. The hydrosphere enters as the fluvial feature class at drainage scale under the Shields-form ruling (Titan the convicting row, question doc on relay). The atmosphere: AIR_FIXTURE is the loudest alien offender (modern post-photosynthesis air on every world, a fixture even the Earth draw fails at most of its own epochs); the outgassing arcs exist and are unwired, so this is a wiring job, and until the wire lands the fixture carries a loud non-canon tag.

Stage 5, render. `Sample(lat, lon, level_of_detail)` evaluates fields plus visible rows plus the texture floor; the fine display grid is legitimate exactly as the memoized sample cache of that function (thousands of tiles as a viewport property, conditioning on nothing); vertical exaggeration stays a labeled toggle, physical scale canonical.

## Provenance-flag retirement (no new machinery)

Accretion mass and bulk density retire through stages 0-1-3; formation era through the stage-1 clock; atmosphere volatiles through the stage-4 AIR_FIXTURE wire. Metallicity is the biggest lever, and it is not a parallel fix beside the Sigma calibration, it is the SAME arc (stage 0 into stage 1), one wiring job retiring two flag roots and the uniform look at once.

## Slice order

Spec closed by this document, START NOW:
1. Craters to rows with render stamping (the grey ball dies, #60 done right). RUNNING (agent).
2. Metallicity-to-Sigma wiring, C/O guard live, MMSN pin placed. RUNNING (agent).
3. The formation-era clock (stage-1 condensation-front timing; also retires the R-YOUNG interim formation-time axis).
4. The AIR_FIXTURE wire (the outgassing arcs into the viewer atmosphere).

Fetch-gated, write the arc while the fetch runs:
5. The flexure kernel (Turcotte-Schubert). Arc scoped in `FLEXURE_ARC_SCOPE.md`.
6. #73 giants into the assembly (unchanged from the morning memo, unlocks CPD moons).
7. The moon impact branch with the tidal-survival filter.

Relay-gated:
8. Hydrosphere against the Shields-form seam ruling (once `R_HYDROSPHERE_WEATHER_RESEARCH_QUESTION.md` returns from relay).

Trailing:
9. Planform-seeded province geometry.

Mirror re-run and zone-scoped rows after slice 2.

## Fetches outstanding this round

Turcotte-Schubert flexure chapter; Neukum-Ivanov production function (the tagged instance); Hayashi (the MMSN pin digits); the Shields curve; Titan transport; Canup-Ward; the Hill fraction; the recession digits (from prior rounds).

## Standing institutions

The resolution ledger and the conditioning-plus-convicting-body template are now standing. Nothing in this consolidation adds an authored scalar: two fixtures die, one derived length scale (the flexural wavelength), one write rule (cross-scale), one seed slot (the texture floor), and one wire (metallicity) enter. The world stops looking authored the moment it stops being fed authored inputs.
