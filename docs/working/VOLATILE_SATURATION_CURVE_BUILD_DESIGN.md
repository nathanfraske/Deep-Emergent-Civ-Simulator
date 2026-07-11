# Volatile saturation curve: the build design (implements the ratified anchor)

Agent A, PR #143, 2026-07-11. This is the buildable design for the three-regime Rankine-Kirchhoff saturation
curve, implementing `docs/working/VOLATILE_THERMODYNAMICS_DESIGN_ANCHOR.md` (owner-ratified). The anchor is the
authority; this document works out the kernel forms, the integration-constant derivations, the molecular-
structure derivation of the Kirchhoff slope, the exact fixed-point implementation, the value inventory for the
derivation-hunter, and the byte-neutral build order. Design-first: nothing value-bearing lands until the gate
gates this design and the hunt clears the measured primitives.

It retires the affine Clausius-Clapeyron tangent (`laws::saturation_vapor_pressure`, the one runtime caller at
`environ.rs:1400`) for the exact curve, which the deferred-transcendental blocker no longer forbids: `Fixed::ln`
and `Fixed::exp` are integer-only, pin-safe on the canonical path (the D_v chain and the Nernst laws already use
them, no `f64`), and the surface-range exponents sit inside `Fixed::exp`'s representable window (about
`[-22, 21.5]`).

## 1. The mid-range curve, the load-bearing regime

Over the surface climate (roughly the triple point to about `0.75*T_c`) the vaporization curve governs. The
derivation is Clausius-Clapeyron with the Kirchhoff temperature-dependent latent heat, integrated in closed form.

**The Clausius-Clapeyron ODE (molar, ideal-vapour, negligible-liquid-volume limit):**
`d(ln P)/dT = L_molar(T) / (R * T^2)`, with `R` the molar gas constant `N_A * k_B` and `L_molar` in J/mol. The
molar form means the curve needs no molar mass `M`; it reads only `R` (a composite of two CODATA fundamentals),
so it is not blocked on the periodic table (PR #145).

**The Kirchhoff latent heat (mid range):** `L(T) = L_b + delta_cp * (T - T_b)` with `delta_cp` derived in
section 2. Substituting and splitting the numerator into a constant part and a `1/T` part:
`d(ln P)/dT = (L_b - delta_cp*T_b)/(R*T^2) + (delta_cp/R)*(1/T)`.

**Integrating** gives the Rankine-Kirchhoff form the anchor names:
`ln P = A - B/T + (delta_cp/R) * ln T`, where the two integration constants DERIVE from the primitives:
- `B = (L_b - delta_cp*T_b) / R`, the "L extrapolated to `T = 0`" over `R`, in kelvin. It is fixed by
  `L_b`, `delta_cp`, `T_b`, and `R`, all measured primitives or derived, so `B` is authored NOWHERE.
- `A` derives from ONE reference point on the curve. The natural reference is the boiling point itself:
  `P_sat(T_b) = 1 standard atmosphere` is the DEFINITION of `T_b`. So
  `A = ln(P_ref) + B/T_b - (delta_cp/R)*ln(T_b)`, with `P_ref = 1 atm`. The standard atmosphere is a DEFINED
  unit constant (101325 Pa exactly), not a per-substance fit, so `A` too is authored nowhere.

The power-law factor is `T^(delta_cp/R)`, exactly `T^(-5)` for water, as the anchor states.

**Worked numbers for water (all derived from the anchor's primitives, checked):** `T_b = 373.15 K`,
`L_b = 40660 J/mol`, `delta_cp = -5R = -41.572 J/(mol K)`, `R = 8.314462618`, `P_ref = 0.101325 MPa`. Then
`B = (40660 - (-41.572)(373.15))/8.314463 = 56172.6/8.314463 = 6756.4 K`, and (anchoring `A` so the curve reads
MPa directly) `A = ln(0.101325) + 6756.4/373.15 + 5*ln(373.15) = -2.2890 + 18.1065 + 29.6099 = 45.4274`.

**Validation of the mid-range curve at three points (derived versus reference, straight):**
- `T = 273.16 K` (triple point): `ln P = 45.4274 - 24.7343 - 5*5.6101 = -7.3574`, `P = 637.9 Pa`. Reference
  611.66 Pa: `+4.3%`.
- `T = 288.15 K` (world mean): `ln P = 45.4274 - 23.4477 - 5*5.66353 = -6.3380`, `P = 1763.8 Pa`. Reference
  1704 Pa: `+3.5%`.
- `T = 373.15 K` (boiling): reads `0.101325 MPa` exactly, by construction of `A`.

The uniform `+3` to `+4%` bias is the Kirchhoff-form residual the anchor predicts (about 4%, down from the
constant-L 10%), an honest limit reported straight, never tuned. The derived triple-point pressure (638 Pa) is
the value the sublimation branch anchors to, so the regimes join continuously (section 3).

## 2. The Kirchhoff slope from molecular structure, the alien-general derivation

`delta_cp = c_p(gas) - c_p(liquid)`, and both sides derive from the substance's own molecular structure, so no
energy value is authored and an alien volatile is a data row.

**Gas side, equipartition.** An ideal gas has `C_v = (f/2)*R` over its active quadratic degrees of freedom and
`C_p = C_v + R`. The degrees of freedom follow the molecule's rotational class, a structural datum: three
translational always, plus rotational (zero for a monatomic gas, two for a linear molecule, three for a
nonlinear one), with the vibrational modes frozen out at surface temperatures (a flagged Einstein-term
refinement). Water vapour is a nonlinear triatomic, so `f = 3 + 3 = 6`, `C_v = 3R`, and `C_p = 4R`.

**Liquid side, Dulong-Petit.** A condensed phase carries `3R` per atom (three quadratic modes per atom,
kinetic and potential). Water has three atoms, so `c_p(liquid) = 9R`.

**So `delta_cp = 4R - 9R = -5R` for water** (about `-41.6 J/(mol K)` against the measured `-41.6`), derived from
the rotational class (nonlinear) and the atom count (three), both per-substance structural data. A diatomic
alien volatile would take `C_p(gas) = 7/2 R` and `c_p(liquid) = 6R`, so `delta_cp = -5/2 R`; the mechanism is
fixed, the structure is data. `delta_cp/R` is always a half-integer (an integer when the rotational degrees of
freedom are even), so the power-law factor is `T^(-5)` for water and a half-integer power in general, which the
`(delta_cp/R)*ln T` term evaluates uniformly through `Fixed::ln` and `Fixed::exp` for any substance.

## 3. The three regimes and their continuity

The linear Kirchhoff `L(T)` is a mid-range approximation and must not be extrapolated (for water it reaches zero
near 1350 K, far past the critical point 647 K, where the latent heat must instead vanish). The curve is
therefore piecewise in three regimes, each derived, joined continuously.

**Below the triple point (over solid), the sublimation branch.** `L_sub = L_vap(T_triple) + L_fus` (Hess's law).
The branch is the Clausius-Clapeyron integral with `L_sub`, anchored at `(T_triple, P_triple)` where `P_triple`
is the mid-range curve evaluated at `T_triple` (638 Pa for water above), so the two regimes join with no gap.
The solid heat capacity is the same Dulong-Petit `3R` per atom the liquid uses, so the sublimation Kirchhoff
slope is `delta_cp_sub = c_p(gas) - c_p(solid) = 4R - 9R = -5R` for water as well (the Dulong-Petit heuristic
gives solid and liquid the same molar heat capacity, a known roughness flagged in the anchor). This branch is
the least-exercised at a temperate surface (it governs only sub-freezing cells) but is required for correctness
where a cell drops below the triple point.

**Triple point to about `0.75*T_c`, the mid range,** section 1.

**Above about `0.75*T_c`, the near-critical branch.** The Watson relation gives the latent heat that correctly
vanishes at the critical point: `L(T) = L_ref * ((T_c - T)/(T_c - T_ref))^0.38`, anchored at a mid-range
reference (`L_ref = L_b`, `T_ref = T_b`, since `T_b = 373 K` is below `0.75*T_c = 485 K` for water). The
exponent `0.38` is a UNIVERSAL correlation constant of the Watson relation, fixed by the corresponding-states
behaviour of the reduced latent heat, never an Earth-water fit (the same status the Neufeld constants and the
Tee-Gotoh-Stewart `1.312` carry in the D_v chain). The saturation-pressure integral over the Watson `L(T)` has
no elementary closed form (the fractional exponent), so in this regime the curve is evaluated by a fixed-step
deterministic integration from the `0.75*T_c` boundary, or the branch supplies only `L(T)` and leaves `P_sat`
undefined where the surface hydrology never reaches it. For a temperate surface `0.75*T_c = 485 K (212 C)` is
never reached, so this branch is completeness for the curve's stated validity range and for hot alien worlds; I
flag it and propose building it last, after the load-bearing mid range and the sublimation branch.

**The regime boundaries.** `T_triple` is a measured per-substance primitive (the solid-liquid-gas coexistence
temperature, 273.16 K for water), and it is the physical switch between the sublimation and mid-range branches.
The upper boundary `0.75*T_c` is an ACCURACY boundary, not a world-content value: it selects which of two
derived latent-heat forms applies, chosen where the linear Kirchhoff extrapolation error crosses tolerance, the
standard reduced temperature at which the Watson relation is applied. It biases no outcome (both forms are
derived); it is an engine-accuracy switch. I surface it for the gate rather than treat it as free.

## 4. The value inventory (for the derivation-hunter)

Per the standing directive, every measured value gets the gate's independent hunt before it lands. This design
reaches for these, and holds all of them until the hunt clears each:

MEASURED primitives (irreducible, cited, held for the hunt):
- `T_b = 373.15 K`, water's boiling point at one standard atmosphere. Basis: a directly-measured phase-
  transition observable, the anchor's authored primitive; cited to a handbook or NIST.
- `L_b = 40.66 kJ/mol`, water's molar enthalpy of vaporization at `T_b`. Basis: a calorimetric observable, the
  anchor's authored primitive; cited.
- `L_fus = 6.01 kJ/mol`, water's molar enthalpy of fusion. Basis: a calorimetric observable, new for the
  sublimation branch; cited.
- `T_triple = 273.16 K`, water's triple-point temperature. Basis: the measured coexistence temperature, the
  physical regime boundary; cited.

ALREADY authored, reused (no new hunt):
- `T_c = 647.1 K`, water's critical temperature, added with the critical point for the D_v chain (the near-
  critical branch and the corresponding-states path share it).

DERIVED (no authored value, computed at runtime):
- `delta_cp = -5R` for water, from the molecular structure (section 2).
- `A`, `B`, the Rankine-Kirchhoff integration constants, from `T_b`, `L_b`, `delta_cp`, `R`, `P_ref` (section 1).
- `P_triple`, the triple-point pressure, from the mid-range curve at `T_triple` (section 3).
- The Watson exponent `0.38`, a universal correlation constant of the relation (not a substance fit).

DEFINED unit / floor-fundamental questions (for the gate):
- `P_ref = 1 standard atmosphere = 101325 Pa`, a defined unit constant (the definition of the boiling point),
  not a per-substance fit.
- `R = N_A * k_B`, a composite of two CODATA fundamentals. The floor today wires only the Stefan-Boltzmann
  `sigma` as a CODATA constant; `N_A` and `k_B` are the SI defining constants (exact), the physics-floor
  fundamentals from which `R` derives. Whether they are added as two floor fundamentals or `R` is added as one
  derived composite is the gate's call; either way `R` is never authored as a bare number.

This design SUPERSEDES authoring `therm.latent_heat` as a separate value (the anchor's instruction): `L_b` is
the primitive and `L(T)` derives, so the constant-`L` `therm.latent_heat` axis retires, its consumers rerouted
to the derived `L(T)` (the surface latent-cooling term and the physiology `1/L_vap` read it; each re-points to
`L(T)` at the relevant temperature, enumerated at the wiring slice).

## 5. The fixed-point implementation, pin-safe

The curve evaluates as `P_sat(T) = exp( A - B/T + (delta_cp/R) * ln T )`, a single exponential of the net
exponent (never `exp(A)` alone, which for water's `A = 45` would overflow Q32.32). The net exponent is small in
the surface range (about `-6` to `-7.4`, inside `Fixed::exp`'s `[-22, 21.5]` window), and every intermediate
(`A ~ 45`, `B/T ~ 24`, `(delta_cp/R)*ln T ~ -28`) is representable. `Fixed::ln(T)` for `T` in the hundreds of
kelvin is well inside its domain. The kernels:
- `kirchhoff_delta_cp(rot_class, atom_count, r) -> Fixed`: the molecular-structure slope (section 2).
- `rankine_kirchhoff_constants(t_b, l_b, delta_cp, r, p_ref) -> (a, b)`: the integration constants, derived once.
- `saturation_vapor_pressure_rk(temperature, a, b, delta_cp_over_r) -> Fixed`: the per-cell curve, one `ln` and
  one `exp`, guarded (a non-positive temperature yields zero; an out-of-window exponent saturates), matching the
  surrounding laws' fail-safe branches. The environ layer derives `(a, b)` once at calibration and holds them,
  the way `EnvironCalib` holds the affine `sat_slope`/`sat_e_ref` today, so the per-cell cost is one `ln` and
  one `exp`.

## 6. Byte-neutral build order and the re-pin

Each kernel is pure Rust taking its primitives as arguments, tested against a test-supplied water reference (the
D_v-chain pattern), so the STRUCTURE lands byte-neutral and unwired while the floor-data authoring and the
runtime wiring stay held:
1. The mid-range kernels (`kirchhoff_delta_cp`, `rankine_kirchhoff_constants`, `saturation_vapor_pressure_rk`)
   plus a test that reproduces the three validation points (611/1704/1-atm within the reported Kirchhoff band).
   Byte-neutral, all five pins hold.
2. The sublimation branch kernel plus a test (continuity at the triple point).
3. The Watson `L(T)` plus its near-critical treatment (built last, the completeness branch).
4. The floor data (`T_b`, `L_b`, `L_fus`, `T_triple`, and the `R = N_A*k_B` fundamentals), authored only once
   the hunt clears each, cited with basis. Byte-neutral (unread until wired).
5. The wiring: retire the affine `saturation_vapor_pressure` at its one runtime caller for the exact curve, and
   retire `therm.latent_heat` for the derived `L(T)`. This CHANGES the runtime saturation values, so it is a
   STATED foundational re-pin (every scenario runs `step_hydrology`, so all five pins move), enumerated for the
   OWNER via the gate with its physical reason (the exact curve replaces the affine tangent), never tuned to a
   target. The four canonical and `living` hold byte-identical through steps 1 to 4; only step 5 re-pins.

## 7. Validation against a reference formulation

Beyond the three-point check in section 1, the built curve is validated across the surface range (273 to 320 K)
against IAPWS-IF97 or the Wagner-Pruss reference equation for water, with the deviation reported straight as a
table (expected: a few percent high across the range, the Kirchhoff residual). The analogous reference equation
of state validates any other measured volatile. The deviation is a reported honest limit, never a tuning target.

## 8. Honest limits

- The Kirchhoff mid-range curve carries a few-percent high bias (about 4%), down from the constant-L 10%. The
  residual is the linear-`L` approximation; a quadratic Kirchhoff term (a temperature-dependent `delta_cp`) is
  the flagged refinement.
- Dulong-Petit `3R` per atom for the condensed phases is strong for water and rougher for other substances, a
  default with a known correction direction (mode-counting or a two-state model).
- The vibrational (Einstein) contribution to the gas heat capacity is dropped at surface temperatures, a
  flagged refinement that matters for hot regimes.
- The near-critical Watson branch is never reached at a temperate surface; its saturation-pressure integration
  is deferred (no elementary form) and flagged.
- Vapour non-ideality (a compressibility factor on the vapour volume) is the anchor's flagged rung, derivable
  from the same intermolecular parameters the D_v chain uses, built when wanted.
