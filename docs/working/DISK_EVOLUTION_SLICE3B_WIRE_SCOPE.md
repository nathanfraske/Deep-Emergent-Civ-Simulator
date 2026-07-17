# Disk-evolution slice 3b-ii: the render-visible wire, scoped

This is the design of record for the wire that makes the `--derived` globe's formation epochs trace back through a root-finder to a declining disk instead of a reserved scalar. It is scoped here before it is built, so the design precedes the code. Nothing in this document is committed to a run path; it is the plan the next focused build executes and verifies.

## 1. What the wire closes, and what it is not

The formation-epoch age dissolves from a reserved value into a derived root (`derive_formation_epoch_myr`, already built and tested). The wire lands that dissolution on the one live consumer, the viewer's `derive_formation_condensation_temperature`, so the epoch feeding the pre-main-sequence luminosity is computed, not picked. It is the first time the derived clock feeds an image a human looks at, which is why it is a focused build with fresh attention rather than a tail-of-session change.

It is NOT a run-path change: `run_world` never reaches `formation_midplane_temperature`, so both canonical pins are provably untouched (verified by diff, as every slice is). The visible surface it moves is the `--derived` globe render, and the measured expectation is that it moves nothing there either, for the reasons in section 6.

## 2. The assembled call graph

Every piece is already built; the wire is composition, not new physics. In `derive_formation_condensation_temperature`, for a star of the rendered mass:

1. Derive the viscous time from the disk's birth size: `t_visc = derive_viscous_time_myr(R_1, star_mass, disk_T(R_1), alpha, mu)`. `R_1` is the existing `DISK_CHARACTERISTIC_RADIUS_AU` (30 AU, the solar pin).
2. Build the one-AU midplane-at-rate closure over the existing disk parameters: `midplane_at_rate = |rate| formation_midplane_temperature(rate, star_mass, 3.5, None, 1 AU, reprocessing, inner, R_1, gamma_sigma, norm, kappa_ref, lo, hi)`. The `None` here is deliberate: the ROOT derives from the accretion decline (the strong, well-conditioned term), not from the sub-four-kelvin irradiation term, so the epoch does not depend on the very luminosity it is about to set (no self-reference).
3. Derive the epoch as the root: `t_formation = derive_formation_epoch_myr(Mdot_0, t_visc, gamma, T_condensation, midplane_at_rate, t_lo, t_hi, iterations)`.
4. Compute the pre-main-sequence bolometric luminosity at that epoch: `L_bol = pre_main_sequence_luminosity_lsun(star_mass, hayashi_wall_T_eff, t_formation)`.
5. Feed it to the DISPLAYED midplane at each orbit through slice 3b-i's door: `formation_midplane_temperature(FORMATION_ACCRETION_RATE, star_mass, 3.5, Some(L_bol), orbit, ...)`, then snap to the condensation grid as today.
6. Run the provenance-gated consistency check (section 4) and thread its verdict into the provenance readout (section 5).

The displayed midplane keeps the reserved 0.19 formation rate; the clock's job is to derive the EPOCH and to referee the 0.19 through the consistency check, not to replace it. That is the 0.19's ruled demotion from a landmark to a checked consequence.

## 3. The interim ledger (surfaced, never fabricated)

The wire pulls the Lynden-Bell-Pringle clock into the render path. Its inputs, each with grade and basis, so the owner sets what is his and the gate reads the provenance:

- `Mdot_0` (birth accretion rate): DRAW-PENDING, grade `CitedToPopulation`, basis the class-0/I protostellar accretion band. Its solar-interim VALUE is reserved-with-basis, surfaced for the owner: the class-0/I range is of order `1e-6` M_sun/yr (about 1 M_sun/Myr), well above the 0.19 class-II rate it declines to. NOT fabricated here; the wire ships with the interim tagged and its destination the layer-4 draw.
- `R_1` (disk birth size): grade `CitedToPopulation`, basis the resolved-disk size demographics, interim the 30 AU solar pin already in the viewer. Sets `t_visc` through `derive_viscous_time_myr`, so `t_visc` inherits `R_1`'s grade (its other inputs are banked).
- `disk_T(R_1)` (disk temperature at the scale radius): reserved-with-basis, the disk thermal profile at 30 AU (of order tens of kelvin for a solar disk), either a banked disk-temperature read or the disk thermal function evaluated at `R_1`. To source at build time.
- `alpha` (Shakura-Sunyaev viscosity): banked, of order `0.01`.
- `mu` (mean molecular weight): banked, `2.34` (a solar hydrogen-plus-helium mix).
- `gamma` (LBP decline index): `1` (bare algebra of the self-similar family, `p = 3/2`).
- `T_condensation`: banked, the 1400 K forsterite-enstatite front (`CONDENSATION_OPACITY_REFERENCE_K` already carries it).
- `hayashi_wall_T_eff`: FETCH TARGET, the ~4000 K solar-composition pre-main-sequence wall, digit source-verbatim from a pre-main-sequence grid (Hayashi 1961 for the track, a Baraffe-2015 or Siess-2000 class grid for the digit), carrying its composition-conditioning field and its #77 demotion destiny.
- `tolerance_frac` (the consistency band): reserved-with-basis, the observational band on the class-II accretion rate the landmark came from, carried with the fetched rate rather than authored.
- `t_lo`, `t_hi`, `iterations`: engine bounds on the bisection (the bracket that must straddle the front, and the fixed iteration count), not physical knobs.

## 4. The provenance-gated consistency check (built, consumed here)

`formation_rate_consistency` (landed, `f4d14aa`) is the referee. The wire calls it with `Mdot_0` and `t_visc` as `ProvenancedInterim`s carrying their grades from section 3. Because both are `CitedToPopulation` (the class-0/I band, the disk-size demographics), the check RUNS and reports `Consistent` or `Inconsistent` against the 0.19 landmark within `tolerance_frac`. The point the gate enforces: this verdict is meaningful precisely because `Mdot_0` and `R_1` were not fit to reproduce 0.19; a chosen-without-basis interim would make the check refuse, so the wire cannot launder a fitted agreement. If the verdict is `Inconsistent`, that is a Residual to surface (the interims and the calibration disagree, a finding), never a signal to tune an interim toward 0.19.

## 5. The provenance readout (the go's one rider)

The `--derived` globe's provenance readout gains a line stating that its formation epochs run on tagged interims (`Mdot_0` class-0/I, `R_1` disk-size demographics) until the layer-4 draws land, and surfacing the consistency verdict. The render says what it rests on, the way every other derived quantity in the engine does, so the image never reads as more settled than it is.

## 6. The render-delta expectation and how it is verified

Measured this session at illustrative values (epoch 1 Myr, wall 4000 K, giving 1.67 L_sun): the formation midplane is viscous-dominated in the inner disk, so the pre-main-sequence warming is sub-four-kelvin everywhere and VANISHES through the 100 K JANAF condensation-grid snap at every sampled orbit, leaving the globe byte-identical (the Mirror world at 1 AU snaps 1400 both ways). The render is robust: neutral for any `t_formation` in the ~0.3 to 3 Myr band, which holds across a wide range of interims. Verification at build time: derive `t_formation` and confirm it lands in that band; diff the `--derived` globe render (or the condensation-temperature outputs at the rendered orbits) None-versus-wired and confirm byte-identity; rebuild both canonical pins and confirm bit-exact. If `t_formation` falls outside the neutral band, the globe shift is a Residual to surface with the interim it rests on, not a red to suppress.

## 7. The I_Terran conditioning (recorded, carried into the notes)

Render-identical is a (solar mass, ~1 Myr) statement, not a universal one. M-dwarf disks are irradiation-dominated much further in, and their pre-main-sequence-to-main-sequence brightness ratios are larger for longer, so their flips WILL be visible on the globe, and that visibility is the fix working on the population it matters for. This conditioning rides the flip's code notes so a neutral result on the solar Mirror is never read as a neutral result on the catalog.

## 8. Build and gate plan

One focused slice, dormant-to-render-visible, presented before it lands. The order: source the `disk_T(R_1)` input and the `Mdot_0` interim value (surfaced with basis); wire the call graph of section 2; tag the interims and call the consistency check; add the provenance-readout line; add the M-dwarf conditioning note. Then the full mirror by exit code (not the fast gate), the doc-link floor delta, fmt and clippy, the globe-render diff for byte-identity, and both pins by diff. The fetch-target `hayashi_wall_T_eff` digit and the `Mdot_0`/`R_1` draws remain surfaced as their destinations; the wire ships with the tagged interims and the consistency check that refuses to be gamed.

## 9. Why it is a fresh-session focused build

It is the first derived clock feeding a human-visible render, it pulls the full LBP clock into the viewer, and it rests on two draw-pending interims whose bases must be documented (not fit) for the consistency check to run. The gate it depends on is landed and cannot be gamed, so the build starts from solid ground. The discipline volunteered and kept: the highest-care slice is built deliberately, with the render-delta measured and the pins diffed before anything a human looks at moves.

## 10. Build attempt 1: correct, byte-neutral, but a nested-bisection perf regression (backed out)

The wire was built end to end and is CORRECT: a probe on the solar Mirror confirmed `t_visc` = 0.143 Myr, `disk_T(R_1)` = 50.8 K (the census wire), `t_formation` = 0.291 Myr (in the neutral band), `L_bol` = 3.80 L_sun, the guard delta = 1.53 K (below the 5 K tolerance, so the one-pass stands, no escalation), and the provenance-gated consistency check returned `Consistent` against the 0.19 landmark, the arc's thesis validated NON-CIRCULARLY (both interims population-based, so the check ran rather than refused). The Hadean gate test passed and the `--derived` globe rendered correctly.

It was BACKED OUT (stashed, patch preserved) for one reason: a performance regression on the render path. `derive_formation_epoch_myr` bisects the epoch (48 outer iterations) and evaluates the midplane at each step, and the midplane `formation_midplane_temperature` is itself a 60-step optically-thick bisection, so one epoch derivation is about 48 by 60 nested fixed-point solves. In debug (which is how CI runs the tests) that pushed a single `build_derived_scene` from seconds to about ninety seconds to three minutes, and any test exercising the formation epoch (the Hadean seam-3 test among them) inherited it. Shipping that would slow CI and the interactive render, so the correct-but-slow wire is held.

THE DIAGNOSED FIX (the completion this build now points to). The viewer passes a CONSTANT opacity to the midplane (`|_t| Some(kappa_ref)`, the grain Rosseland mean evaluated once because it is nearly flat in T). With a constant opacity the optically-thick midplane fixed point has a CLOSED FORM, `T_mid = radiative_equilibrium(3/4 * (kappa * Sigma / 2) * F_visc + reprocessing * F_stellar, sigma)`, so the 60-step inner bisection is UNNECESSARY for the root: the epoch's midplane map can be the closed form, O(1) per outer step instead of O(60). Combined with deriving the epoch ONCE per star (it is star-only, not per-orbit, so the system-map loop should hoist it out), that removes the regression. The closed form is verifiable against `formation_midplane_temperature` for constant kappa (both converge to the same fixed point), so it lands with a byte-match test, and the DISPLAYED midplane keeps the exact 60-step form untouched, so the render stays byte-identical. This is careful work (a new closed-form midplane path with its own correctness test plus a caller restructure), deliberately deferred rather than rushed at a session tail; the wire patch and this diagnosis carry it to a clean completion.
