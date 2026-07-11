# Volatile thermodynamics: the design anchor

Owner-ratified design anchor (2026-07-11). The condensed-phase thermodynamics of a volatile collapses to a small set of authored primitives per substance, and everything else DERIVES through established physics. This document is the standing reference for the volatile floor and its cascade; the Mirror hydrology arc implements it, and the procedural path is built when proc-gen worlds arrive.

## The per-volatile primitives (the whole authored floor for a volatile)

A volatile carries, by provenance:
- MEASURED volatiles (Mirror, handbook data exists): the boiling point `T_b` and the boiling-point latent heat `L_b`, both precise measured observables. The cohesive energy `E_b = L_b * M / N_A` DERIVES from them (a cross-check, never authored).
- PROCEDURAL volatiles (alien, no handbook): the liquid cohesive energy `E_b` (from the molecular model), from which `L_b = N_A * E_b / M` derives and `T_b` is DERIVED (below).
- Plus a cohesion-class or POLARITY flag (hydrogen-bonded versus not), used ONLY on the derived-`T_b` path.
- Plus, shared with the gas-transport arc, the critical temperature `T_c` (already authored as the critical point) for the near-critical regime; the fusion latent heat `L_fus` for the sublimation branch; and the triple point as the low-temperature reference.

For water (Mirror): `T_b = 373.15 K`, `L_b = 40.66 kJ/mol`; `E_b` derives to about 0.45 eV as the cross-check. These are the only authored numbers; the rest is theorem.

## The cascade (all derived)

- **Latent heat, definitional:** `L_molar = N_A * E_b` (the cohesive energy IS the energy to break all the bonds, which IS the latent heat). Equivalently `E_b` derives from `L_b`. Exact, not a fit.
- **Temperature dependence of L, Kirchhoff:** `dL/dT = c_p(gas) - c_p(liquid)`, differentiating the enthalpies along coexistence in the ideal-vapour limit. Both sides are already derivable in the engine: the gas side is equipartition (for water vapour, a nonlinear triatomic, `c_p = 4R`, with the small Einstein vibrational term), and the liquid side is Dulong-Petit (`3R` per atom, `9R` for water). So `delta_cp = 4R - 9R = -5R` for water (measured near -44 J/mol/K against -41.6), and `L(T) = L_b + delta_cp * (T - T_b)`, valid as the MID-RANGE form only (see the three-regime structure below). This closes water's L to about 4 percent from the 10 percent constant-L error over the mid range, adding ZERO authored values.
- **Vapour pressure, Rankine-Kirchhoff:** feed `L(T)` into Clausius-Clapeyron and integrate to `ln P = A - B/T + (delta_cp/R) * ln T`, the Rankine-Kirchhoff form; the power-law factor is exactly `T^(delta_cp/R)` (`T^(-5)` for water). This REPLACES the constant-L affine tangent: same primitives, the Kirchhoff `L(T)` on top, 10 percent becomes 4.
- **Boiling point:**
  - Measured volatiles: `T_b` is authored, so it is exact and carries no Trouton scatter; the polarity flag is not needed on this path.
  - Procedural volatiles: `T_b` is DERIVED from the cohesion by HILDEBRAND'S rule (the vaporization entropy evaluated at a fixed vapour CONCENTRATION rather than fixed pressure, which removes most of the volume-driven scatter Trouton carries), with the polarity flag correcting the residual hydrogen-bonding offset. Trouton's own scatter is irreducible, which is why measured `T_b` is authored rather than derived through it.
- **Liquid heat capacity, Dulong-Petit:** `3R` per atom (`9R` for water, about 74.8 against 75.3, one percent). Heuristic but strong; a mode-counting or two-state correction is the flagged refinement.
- **Humidity sensitivity, differentiate C-C:** `d(ln e_s)/dT = L / (R_v * T^2)`, about 6.4 percent per kelvin at 288 K, the water-vapour feedback, free from the same L.

## Range and the three regimes of L(T) and the saturation curve

The linear Kirchhoff `L(T)` is a mid-range approximation and must NOT be extrapolated: for water it hits zero near 1350 K, far past the critical point `T_c = 647 K`, which is unphysical (the latent heat must vanish at `T_c`, where liquid and gas become indistinguishable). The saturation curve is therefore piecewise in three regimes, each derived, reusing `T_c` from the critical point the gas-transport arc already authors:

- **Below the triple point (over solid):** the SUBLIMATION curve, `L_sub = L_vap + L_fus` (Hess's law, vaporization plus fusion). This adds the fusion latent heat `L_fus` as a measured per-substance primitive.
- **Triple point to about `0.75 * T_c` (mid range):** the linear Kirchhoff `L(T) = L_b + delta_cp * (T - T_b)` fed through the Rankine-Kirchhoff integral above.
- **Above about `0.75 * T_c` (near-critical):** the WATSON relation `L(T) = L_ref * ((T_c - T)/(T_c - T_ref))^0.38`, so `L` correctly vanishes at `T_c`, which the linear form cannot represent.

Validate the whole three-regime curve against a reference formulation: IAPWS-IF97 or the Wagner-Pruss equation fits for water, and the analogous reference EOS for other measured volatiles.

## Flagged rungs (derive-clean refinements, built when wanted)

- Vapour non-ideality: a factor `1/Z` on the vapour volume, `Z = 1 + B(T) * P / (R * T)`, the second virial coefficient `B(T)` derivable from the same intermolecular parameters the gas transport uses (the critical point or the LJ well). No new anchor.
- The dipole-moment polar correction (Stockmayer/Brokaw) keyed on a per-substance dipole moment, which corrects both the derived-`T_b` Hildebrand offset AND the gas-transport `D_v` corresponding-states deviation with one datum. See the gas-transport arc's flagged deviation.

## Honest limits

- The constant-L drift (about 10 percent) is closed to about 4 percent by Kirchhoff; the exact exponential curve (deferred to the transcendental kernels) removes the residual.
- Trouton's scatter cannot be derived away; measured volatiles author `T_b` to sidestep it, procedural volatiles use Hildebrand (more universal) plus the polarity flag, and both still carry a few-percent `T_b` uncertainty on the derived path.
- Dulong-Petit `3R` per atom is strong for water, rougher elsewhere, a default with a known correction direction.
- The cascade is for molecular VOLATILES (condensed molecular liquids). Metals, ionic solids, network solids, and exotic phases have different cohesion physics; each would carry its own anchor-and-law family, which the cohesion-class flag generalizes toward.

## Derive-first status
`T_b`, `L_b` (measured) or `E_b` (procedural), plus the polarity/cohesion-class flag, are the irreducible measured primitives (deriving them needs the many-body Hamiltonian, which we choose not to solve). Everything else on this page derives. This anchor supersedes authoring `therm.latent_heat`, the saturation-curve coefficients, the liquid heat capacity, and the boiling point as separate values.
