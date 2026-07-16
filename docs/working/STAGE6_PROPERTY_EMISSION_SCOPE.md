# Stage 6, property emission: a scoping pass (research-agent draft, to be hardened)

The materials oracle's next stage after the freezer: from a realized frozen assemblage (Stages 0 to 5), derive the OBSERVABLE material properties the capstone reports. This document is a first scoping pass produced by the local research tier (a tool-using agent over web search, sources cited below), reviewed by the gate. It is NOT the audited build spec: the reserved-value column in particular is a scoping approximation, and every property must go through the derivation-hunter (the discipline A applies) before a value is entered. Treat it as the starting packet for the Stage-6 arc.

## What is already on the floor (the inputs property emission reads)

Per phase the floor provides: elemental composition; bulk modulus `B_0`; atomic volume `V_atom`; cohesive energy `E_coh`; melting point `T_m`; sound speed `c_s = sqrt(B_0/rho)`; Debye temperature `Theta_D`; self-diffusivity `D(T)`; solid-liquid interfacial energy `gamma_sl`; heat of fusion `dH_f`; grain size (written state). The realized assemblage also carries the distance-from-equilibrium archive and, when built, the defect and dislocation population.

## The property scope (method, inputs, reserved residual, difficulty)

The `[reviewer]` notes mark where the scoping pass under-called a reserved value that the derivation-hunt will surface.

- **Density.** `rho = M / V_atom` from the molar mass (composition) and the atomic volume. Reserved: none. Easy.
- **Elastic moduli (K, G, E).** `K = B_0` (anchored). `G` from the cohesive energy via a Chen-Tse-class correlation (`G` scales with `E_coh / V_atom`). `E = 9KG / (3K + G)`. Reserved: `[reviewer]` the Chen-Tse proportionality is a per-class coefficient, not zero. Easy.
- **Strength and fracture toughness.** Yield strength from the shear modulus (`sigma_y ~ k*G`); fracture toughness `K_IC = sqrt(E * G * gamma_sl)` reading the floor's `gamma_sl`. Reserved: `[reviewer]` the `k` in `sigma_y ~ k*G`, and the real strength depends on the written defect population. Medium.
- **Hardness.** Vickers `H_V ~ k*G` (Chen-Tse / Teter correlation, `k ~ 0.15 to 0.2` for metals). Reserved: `[reviewer]` the per-class `k`. Easy.
- **Creep rate.** The Frost-Ashby deformation-mechanism map: diffusional flow scales with `D(T)/d^2`, dislocation creep with `sigma^n * D(T) / d^3`, reusing the built `D(T)` and the grain size `d`. Reserved: `[reviewer]` the stress exponent `n` per mechanism. Medium.
- **Thermal conductivity.** The Slack equation `kappa = (3/16pi) * (k_B * Theta_D^3) / (M * gamma_G^2 * V_atom)`, reading the built Debye temperature. Reserved: the Grueneisen parameter `gamma_G`. Medium.
- **Thermal expansion.** The Grueneisen relation `alpha = gamma_G * C_v / (K * V_atom)`, `C_v` from the Debye model. Reserved: the Grueneisen `gamma_G` (shared with thermal conductivity). Medium.
- **Electrical conductivity.** The Drude model `sigma = n * e^2 * tau / m`, the carrier density `n` from the valence-electron count, the scattering time `tau` from electron-phonon scattering (scales with `Theta_D` inverse). Reserved: `tau`'s scale, and this needs the electronic structure (below). Medium to hard.
- **Magnetic moment and ordering.** The band-structure Stoner criterion (`I * N(E_F) > 1`) or a Heisenberg exchange from the cohesive energy; requires the electronic density of states. Reserved: needs the DOS floor addition. Hard.
- **Optical color and reflectivity.** The dielectric function `epsilon(omega)` from the band structure (interband transitions) plus the free-electron Drude term; reflectivity `R = |(n-1)/(n+1)|^2`. Requires the band structure. Hard.

## The build order and the one floor addition

Build order (each reading the built anchors, in dependency order): density, then shear modulus `G`, Young's `E`, hardness, yield strength, fracture toughness, creep, thermal expansion, thermal conductivity, then the electronic set (conductivity, magnetics, optics).

The structural finding: every MECHANICAL and THERMAL property derives from anchors already on the floor, so that half of Stage 6 is buildable on the current substrate (with one per-class coefficient each, to be hunted). The ELECTRONIC properties (conductivity, magnetics, optical color) all bottom out on a single missing floor piece: an ELECTRONIC-STRUCTURE substrate (a valence-electron / free-electron density for Drude, and a density of states or band structure for magnetics and optics). That electronic-structure floor is the one new substrate the back half of Stage 6 needs, and it is the natural design-first sub-arc within Stage 6.

## Completeness gaps (from the doc's own audit loop)

A completeness pass over this document (the local research tier, auditing for what was missing) surfaced six additions, all legitimate:

- **Poisson's ratio `nu`.** Derives directly from the computed `K` and `G` (`nu = (3K - 2G)/(2(3K + G))`). A standard elastic output the list implied but did not emit. Easy.
- **Specific heat `C_p` / `C_v`.** From the Debye model over the built `Theta_D`. The Grueneisen relations already use `C_v` internally; emit it as a property. Easy.
- **Thermal diffusivity `alpha_th = kappa / (rho * C_p)`.** Composes the thermal conductivity, density, and specific heat, all derivable. Easy.
- **Solid-vapor surface energy `gamma_sv`.** DISTINCT from the floor's solid-liquid `gamma_sl` (nucleation) and absent from the list. It governs wetting, sintering, catalysis, and fracture surfaces. The satisfying part: `gamma_sv` is exactly what the broken-bond `E_coh` route yields, the route A REJECTED for `gamma_sl` in the §5 hunt because it gave the wrong (solid-vapor) energy. So that rejected route is not wasted, it is the `gamma_sv` derivation, and `gamma_sv` reuses the built `E_coh` with one per-class surface-bond fraction. Medium.
- **Defect formation energies (`E_f_vacancy`, `E_f_interstitial`).** The document reads a defect population but not the formation energies that set it; these tie to the vacancy fraction `f` the freezer barrier already reserves, and govern strength and creep. A floor addition to scope with the defect-population piece.
- **Grain boundary energy `gamma_gb`.** The document carries grain size but not the boundary energy, which governs grain growth and the Hall-Petch strengthening the strength property needs. Derives in the same family as `gamma_sl`/`gamma_sv`.

So the completeness verdict refines the earlier one: the mechanical and thermal core is buildable on the current floor once these elastic/thermal companions (Poisson, `C_p`, diffusivity) and the surface/boundary energies (`gamma_sv`, `gamma_gb`, both `E_coh`-derived) are added, and the electronic-structure substrate remains the one heavy missing floor piece for the electronic properties.

## Provenance and honest limits

This is a research-agent scoping pass, not an audited spec. The derivation methods and the build order are sound and sourced; the reserved-value accounting is optimistic (the scoping pass marked several as "none" where a per-class coefficient exists, corrected inline above). When Stage 6 becomes the arc, each property runs the derivation-hunter (does the coefficient derive from the floor, or is it an irreducible per-class residual reserved-with-basis and primary-verified) exactly as the freezer's `f`, `delta`, `C`, and `dH_f` did. The alien-feasibility check applies throughout: each method must key on the material's own data, never assume Earth minerals.

## Sources (web-verified this session)

Chen-Tse hardness and modulus correlations (Chen, Niu, Li, Li, Intermetallics 2011, and related modeling-hardness literature); the Frost-Ashby deformation-mechanism maps (Frost and Ashby, Deformation-Mechanism Maps); the Slack equation for lattice thermal conductivity (Slack, high-lattice-thermal-conductivity solids); the Grueneisen parameter and its thermal-expansion relation; the Drude model of electrical conductivity; and Nye, Physical Properties of Crystals, for the elastic-constant framework. Each cited by the research agent during the scoping run.
