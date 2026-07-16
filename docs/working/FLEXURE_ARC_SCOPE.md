# Flexure kernel arc (pipeline slice 5): the wavelength band the eye reads as terrain

Status: KERNEL BUILT and dormant (`crates/physics/src/flexure.rs`); the RENDER WIRING is BLOCKED on the elastic lid thickness `T_e`, which nothing built derives (see the corrected premise below). The Turcotte-Schubert fetch has landed. Written-arc-first per the owner's split (truly fetch-gated, unlike slices 1-4). This fills the FLEXURAL MIDDLE the first surface plan missed: the band between the coarse convective provinces (~1000 km, the resolved field) and the fine crater rows (the discrete objects), which is exactly the ~100-1000 km wavelength band a human reads as mountain ranges, foreland basins, rifts, and flexural bulges. Without it the surface is a smooth ball plus craters and skips the scale terrain lives at.

## The gap, grounded

Stage 4 of `CONSOLIDATED_SURFACE_PIPELINE.md` places the surface as a superposition of representations, each at its own derived scale: the convective-wavelength province field, the crater rows, the texture floor. The middle band is empty. Real terrain in that band is the lithosphere's ELASTIC FLEXURE under loads: a load (a volcanic pile, a crater basin, a sediment wedge, a province thickness contrast) bends the elastic plate it sits on, and the plate's rigidity sets the wavelength of the bending. That bending is what makes mountain belts, moats around volcanoes, and peripheral bulges. It is a single analytic response, no per-world authoring.

## The mechanism (derive-first, no free constants)

The flexure of a thin elastic plate over a fluid substrate is `D * grad^4(w) + (rho_mantle - rho_infill) * g * w = q(load)`, where `w` is the deflection, `q` the load, and `D` the flexural rigidity. Everything on the left DERIVES:

- FLEXURAL RIGIDITY `D = E * T_e^3 / (12 * (1 - nu^2))`. `E` (Young's modulus) and `nu` (Poisson's ratio) come from the banked material moduli (the same modulus tier the support-bound yield derive-down used, `phase_bulk_modulus_ionic` and the shear/Pugh chain); `T_e` is the elastic lid thickness. **PREMISE CORRECTED (2026-07-16, the code-state check at the render-wiring slice): this document asserted the deep-time thermal state "already carries" the lid, and IT DOES NOT.** `ColumnState` is `{temperature, convecting}`, one LUMPED scalar per column, and nothing in `crates/` carries temperature on a DEPTH axis (no geotherm, no half-space cooling, no depth-resolved thermal field), so a mechanical lid base cannot be located against a profile that does not exist. The derived THERMAL boundary layer (`depth * Ra^(-1/3)`, in `column_readout`) is the mantle-scale thermal lithosphere, not the elastic thickness, and pinning one to the other through a ratio authors that ratio; a ~600 K limiting isotherm authors an empirical proxy that is neither a floor constant nor dimensionless. So T_e is the SOLE unsupplied input to D (E and nu derive from the banked moduli, g and the densities are derived), and the render wiring is BLOCKED on it rather than authored past. The honest route is McNutt (1984) moment-equivalence, where the limiting isotherm EMERGES as a description of the outcome rather than entering as a coded shortcut, and it needs, in dependency order: (1) a depth-resolved geotherm `T(z)` across the lid, the bedrock gap; (2) a pressure-dependent brittle branch (the built `derived_crust_yield_pa` is temperature- and pressure-independent); (3) a SILICATE creep route (`crates/materials/src/creep.rs` is a complete Mukherjee-Bird-Dorn law and is dormant, but its diffusion input is keyed by element symbol and scoped to ELEMENTAL METALS, so an olivine lid has no jump rate today). No authored rigidity.
- THE RESTORING TERM `(rho_mantle - rho_infill) * g`: the mantle and infill (water, air, or sediment) densities are derived from the world's composition; `g` is the world's own derived gravity.
- THE DERIVED LENGTH SCALE (the whole point): the flexural parameter `alpha = (4 * D / ((rho_mantle - rho_infill) * g))^(1/4)` is the wavelength the plate bends at. A thin-lid young world (small `T_e`, small `D`) flexes at a SHORT wavelength; a thick-lid old world (large `T_e`, large `D`) flexes at a LONG wavelength. Same formula, different worlds, the conditioning line. `alpha` is the one new derived length scale the consolidation admits, and it is derived, not authored.

## The load list and the render (rows, not rasters)

The flexure is an analytic GREEN'S-FUNCTION CONVOLUTION over the LOAD LIST, consistent with the rows-not-rasters rule: each load (a province thickness contrast at or above the convective wavelength, a volcanic construct, a large crater basin from the slice-1 row list) contributes the point-load or line-load flexure Green's function (the Kelvin-function / decaying-oscillatory solution for a point load on a plate, Turcotte-Schubert chapter 3), scaled by the load and the derived `alpha`. `Sample(lat, lon, level_of_detail)` (stage 5) adds the flexure deflection to the province base and the stamped crater rows. So the flexure is another queryable layer, never a raster.

## The cross-scale write rule

Flexure reads loads at or above its own response wavelength and writes a deflection field in that band; it does not manufacture sub-`alpha` detail (that is the crater rows and the texture floor). Large loads that flex the plate also feed back to the province thermal/isostatic state where they cross the convective wavelength (the same feedback slice 1 preserves for basins).

## What is fetch-gated

The Turcotte-Schubert flexure chapter (Geodynamics, ch. 3, the plate-flexure Green's functions and the flexural-parameter definition), for the exact analytic form of the point-load and line-load deflection and the Kelvin-function coefficients. These are cited mathematical forms (dimensionless function shapes), not authored world values; they enter as the kernel's shape the way the crater law's pi-groups did. Until the fetch lands, this arc is scoped-not-built.

## Gates when built

Viewer-side and dead-code substrate (non-canon, off the run path), so both pins hold. No free constants (E, nu from banked moduli; T_e, densities, g derived; the Green's-function shape is the cited Turcotte-Schubert form). The conditioning test: a young thin-lid world and an aged thick-lid world must flex at different derived wavelengths from the same call, and a zero load must give zero deflection. Prose customs, fmt, clippy, stone0.

## Couplings

Upstream: the deep-time thermal state (the lid thickness `T_e`), the banked moduli (E, nu), the derived gravity and composition densities, and the slice-1 crater-row load list. Downstream: `Sample` (stage 5) composes it between the province field and the crater rows; the hydrosphere (slice 8) adds sediment loads to the load list; the render reads the deflection as terrain.
