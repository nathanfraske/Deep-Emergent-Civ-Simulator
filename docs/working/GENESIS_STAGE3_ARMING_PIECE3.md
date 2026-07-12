# Genesis Stage 3 arming, piece 3: arming Mirror as bulk silicate Earth (design-first)

This is the design-first grounding for piece 3, arming Mirror against a cited composition so the surface
substrate goes live. It is written to the owner's standing bar for the rest of the arc: the reserved list must be
VERY SHORT, because the physics derives nearly everything. The rule this design holds is composition and geometry
in, everything else derived, the reserved list as short as the physics permits, and every coefficient's derivation
hunted before it is reserved. Grounded against the merged petrology and geodynamics (PR #160 on main).

## The producer chain, derived end to end

The producer flows one per-world input, the world's major-element chemistry, through the merged kernels to the
elevation the relaxation reads, deriving every density on the way:

1. The bulk-silicate-Earth COMPOSITION (McDonough and Sun 1995), the mantle-plus-crust major-element abundances,
   is the per-world input.
2. `civsim_physics::petrology::crustal_density(composition, temperature_k, pressure_bar, registry, table)` minimizes
   the composition to its stable mineral assemblage and returns the assemblage density (grams per cubic
   centimetre). The crustal density is DERIVED end to end from the composition, the conditions, and the
   data-defined phase registry, never an authored per-rock-type density. The mantle density is the SAME derivation
   on the mantle composition, so the Airy density contrast is two derived densities, not one derived crust floating
   on an authored mantle (the correction the gate already made to the interior lane's mantle density).
3. `civsim_physics::geodynamics::airy_isostatic_elevation(crustal_density, mantle_density, crustal_thickness)`
   returns `crustal_thickness * (mantle_density - crustal_density) / mantle_density`, the buoyant freeboard. The
   isostatic elevation is DERIVED from the two densities and the thickness.
4. The producer writes that elevation into the per-column `GeodynamicColumn::isostatic_elevation`, and
   `civsim_sim::geodynamics_surface::relax_toward_isostasy` reads it and relaxes the effective elevation toward it,
   which the four transport drivers then carve.

So the whole density-to-elevation half of the arming reserves NOTHING: it derives from the composition and the
geometry. The composition is the input; the freeboard is the output.

## The reserved list, hunted for derivation, held to the minimum

Every candidate coefficient is hunted for its derivation before it is reserved. What derives is not reserved; what
remains is the short irreducible list.

The stream-power EXPONENTS `m` and `n` DERIVE, and this is the piece-2 default's promotion the gate flagged. The
law `E = K * A^m * S^n` is not fundamental: it is a reduction of an incision process model composed with the
channel hydraulic geometry and the flow resistance. The bed shear stress is `tau = rho * g * R * S` (R the
hydraulic radius); water conservation is `Q = w * h * v`; the flow resistance closes `v` on `h` and `S` (Manning
or Chezy); the downstream hydraulic geometry scales the width `w` on discharge as `w ~ Q^b` (b near 0.5); and the
discharge scales on drainage area as `Q ~ A^c` (c near 1 under uniform runoff, the discharge proxy the fluid-shear
driver already uses). Composing these, the incision reduces to `E = K * A^m * S^n` with `m` and `n` FIXED by the
process model and those scalings: a shear-stress model gives `m ~ 1/3`, `n ~ 2/3`; a unit-stream-power model gives
`m ~ 1/2`, `n ~ 1`; a total-stream-power model gives `m = 1`, `n = 1` (Whipple and Tucker 1999). So the Mirror
default `m = 1/2`, `n = 1` is the DERIVED unit-stream-power result for `b = 1/2`, `c = 1`, never a free reserved
pair. Piece 3 derives the driver-row exponents from a small derivation `stream_power_exponents(process_model, b, c)`
in the exact-root family, so the reserved input collapses to the PROCESS MODEL (a discrete, physically-named
choice: shear-stress, unit-stream-power, or total-stream-power) plus the near-universal hydraulic-geometry
exponent `b` and the discharge exponent `c` (both reserved-with-basis as near-constants, `c` derived from the
runoff field when a climate lane supplies one). The exponents themselves leave the reserved list.

The entrainment THRESHOLD `theta` (the Shields or Bagnold critical shear) largely DERIVES. The Shields relation
sets the critical shear stress as `theta_c = shields_number * (rho_s - rho_f) * g * d`: the grain and fluid
densities are composition-derived, gravity is geometry, the grain size `d` is the fracturing driver's grain
product, and the only reserved input is the dimensionless Shields critical number (a universal function of the
grain Reynolds number, near 0.045 in the rough-turbulent limit), reserved once as a universal constant rather than
per-world. So `theta` leaves the per-world reserved list and becomes a derivation reading composition, geometry,
and the universal Shields number.

What CANNOT derive without the sim or is cited universal data, and so remains reserved-with-basis, is the short
list: the bedrock ERODIBILITY `K_b` (the rock-strength-and-process coefficient of the incision rate, notoriously
wide error band, cited to measured incision rates, though its lithology dependence may read the floor's fracture
data), the hillslope DIFFUSIVITY `D` (a per-material creep rate, cited), the dissolution ARRHENIUS DATA (the
activation energy and pre-factor, cited kinetic data per lithology and solvent), the fracturing FATIGUE RATE
(cited), the settling grain-size distribution, and the seed CRUSTAL THICKNESS (a per-world geometry datum). Each is
surfaced with the ground on which the owner would set it, read from the calibration manifest, failing loud on an
unset value.

The per-world INPUTS are the composition (McDonough and Sun 1995) and the planetary geometry (gravity, the seed
crustal thickness, and the mantle composition for the mantle density). Everything else either derives from these
plus the constants, or is a cited universal number reserved once.

## The re-pin and the slice sequence

Piece 3 is where the drivers stop being byte-neutral. An armed genesis scenario runs the producer and the
transport, so the living pin re-pins on that scenario, stated and measured on its own when it comes, while the
other four canonical pins that run no genesis pass stay byte-identical. The slices, each grounded against the
merged code, each verified and gate-checkpointed:

- 3a: the exponent derivation `stream_power_exponents(process_model, b, c)` in the exact-root family, so the
  driver-row exponents are derived from the process model rather than reserved (byte-neutral: the unit-stream-power
  Mirror case derives `SQRT`, `LINEAR`, the current default).
- 3b: the Shields threshold derivation, so `theta` derives from composition, gravity, grain size, and the universal
  Shields number (byte-neutral off the run path).
- 3c: the Mirror bulk-silicate-Earth composition as world data, and the producer that flows it through
  `crustal_density` and `airy_isostatic_elevation` to `isostatic_elevation` (the mantle density derived from the
  mantle composition beside the crustal density).
- 3d: the armed scenario that runs the producer and the drivers, the stated living re-pin, measured on its own.

The full section-9 blind panel runs on the armed substrate at the arc boundary, per the owner's standing
requirement, both restored shaping-catcher lenses and the correctness lenses, every finding verified against source.

## The honest limits carried forward

The general arbitrary-exponent fractional power stays the open GPU-canon gate (the exact-root family covers the
process-model exponents 1/3, 1/2, 1; the cube root waits for #45's canon cube root rather than a speculative
`Fixed::cbrt`); the non-local redistribution primitive now exists on main (C's #172), so impact and mass-flow can
be revisited, but they stay deferred behind their process rows; the heightfield is a surface projection; the build
order is Earth-frequency-ordered; and the per-cell forcing fields the panel flagged (a climate-derived runoff for
the discharge exponent `c`, a solvent-availability gate for dissolution) are heterogeneous-world extensions the
arming carries as uniform defaults until their cross-lane inputs are wired.
