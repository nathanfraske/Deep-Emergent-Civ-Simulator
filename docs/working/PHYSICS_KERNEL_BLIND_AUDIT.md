# Physics Kernel Blind Audit: findings, fixes, and remaining work

This record captures a fully-blind audit of the physics kernels (AGENTIC_ADDENDUM.md
section 7). Eight independent auditors on the cheapest fitting model, each walled off from
the repository and un-primed about a prior pilot's findings, audited all sixty-eight closed-form
kernels in `crates/physics/src/laws.rs` against a self-contained packet (the substrate contract,
the kernel source, and the declared law and axis specifications, with the tests and prior reviews
withheld). Every finding below was then verified against the real source before it counted, and
the fixes in the first section are built and tested.

## Why this caught what sighted review did not

The tests pass on several of these kernels because the tests were written to the kernels'
outputs, so a scale error is encoded identically in both. A blind auditor, given only what the
kernel claims and the substrate it computes in, reconstructs the physics from first principles and
does not inherit that shared assumption. The two bug-bearing domains were double-covered, and the
un-primed panelists independently reproduced the pilot's two flagship scale bugs (a triple
confirmation), which is the convergence evidence the method rests on.

## Fixed in this pass (confirmed wrong in the normal operating range)

Each fix carries a regression test that pins the physically-correct magnitude.

Missing unit-scale reconciliation:

- `euler_buckle`: the elastic modulus is on the megapascal scale but the buckling load is a
  newton force, and the megapascal-to-pascal bridge was absent, so the load read a million times
  too small. Fixed by promoting the product with `C_PA` after the reducing divide.
- `wear`: the indentation hardness is megapascals but the wear volume is cubic metres, and the
  bridge was absent, so the volume read a million times too large. Fixed by dividing by the pascal
  hardness (the megapascal value promoted by `C_PA`). This is the SI cubic-metre reading of the
  declared `volume` dimension; the two prior tests encoded the un-promoted value and were rewritten.
- `thermal_stress`: the modulus (megapascals) times the expansion coefficient (ppm per kelvin,
  stored times a million) cancels the two prefixes to leave pascals, but the result was compared
  against a megapascal fracture strength, so a mild heating fractured spuriously. Fixed by descaling
  the stress by `C_PA` before the comparison.
- `phase_change_energy`: the sensible term is joules (the specific heat is J/(kg K)) while the
  latent term is kilojoules (the latent heat is kJ/kg), summed with no bridge. Fixed by dividing the
  sensible term by `C_KJ` before the sum, and the bare temperature difference is now a saturating
  subtraction.

Reduce-before-grow and precision:

- `poiseuille_flow`: dividing the driving pressure by the tiny viscosity before applying the
  radius power overflowed a representable flow to the cap for ordinary air and water. Fixed by
  interleaving the four radius multiplies with the viscosity, length, and eight divides so the
  running value tracks the bounded true flow.
- `reynolds_number`: the same shape, dividing by the tiny viscosity before multiplying the
  characteristic length. Fixed by multiplying the length in first.
- `radiative_equilibrium`: forming emissivity times sigma (sigma is only about eight fixed-point
  bits) underflowed to zero for a low emissivity and returned the cap. Fixed by rooting each factor
  first, since the square root of the product is the product of the roots.

In-range logic:

- `lever`: the mechanical-advantage success path omitted the `advantage_max` cap that its own
  zero-load and overflow branches apply. Fixed by capping the success path too.
- `interface_split`: reflectance and transmittance were clamped independently, so a pair whose
  sum exceeds one returned a triple summing to more than the incident flux. Fixed by clamping
  transmittance to the budget reflectance leaves, so reflected plus absorbed plus transmitted
  equals the incident flux.

## corrosion (resolved: literal pH, on the owner's call)

The kernel multiplied the driving margin by the raw `chem.acidity` value, so corrosion peaked at
pH fourteen (basic) and was zero at pH zero (most acidic), the inverse of the acid-attack physics
the cited source describes, and the test shared the inversion. On the owner's decision the axis is
literal pH, so the kernel now takes the aggressiveness as the distance below the pH ceiling
(`sat_sub(14, pH)`, the fourteen being the definitional pH-scale maximum the axis range carries, a
scale bound and not a fabricated value), and the test is rewritten to assert that corrosion rises
as pH falls. Fixed.

## The latent overflow-direction class (resolved: one sweep)

The panel found one mechanical pattern across every domain: an overflow or a degenerate branch that
routes to the wrong physical extreme or is blind to sign. Each was reachable only on an overflow or
an out-of-range input, so none was wrong in normal operation, but each was a real totality-discipline
inconsistency. The whole class is now closed with the established sign-aware and saturating idioms:
`satisfaction` (an overflowing supply product now reads full, not starving), `contact_pressure` (an
overflowing area now reads zero pressure, not the max), `sensible_energy` (a non-positive gradient
now reads zero over its [0, E_MAX] law, which also keeps its overflow branch sign-correct),
`ideal_gas_density` (an overflowing R*T now reads the minimum density, not the maximum),
`thermal_buoyancy` (the division overflow now routes by the gradient sign like its sibling branch),
`evaporation_rate` (an overflowing wind term now saturates at the cap, not the still-air baseline),
and `ohm_voltage`, `solenoid_field`, and `flux_linkage` (now non-negative magnitudes over their
declared `[0, MAX]` axes, so the overflow cap is sign-correct; the Lenz-law sign `faraday_emf`
recovers comes from the signed tick-to-tick difference of two non-negative flux samples, not from a
signed flux, so `solenoid_field` and `flux_linkage` agree on the magnitude reading). The bare
subtractions in `bend_stress`, `axial_stress`, and `friction` are now saturating. Each carries a
regression test.

## Specification and wiring seams (not kernel arithmetic)

- Energy is not uniformly scaled on the wire: `kinetic_energy`, `power`, and torque emit
  kilo-scaled values, while `sensible_energy` and the sensible half of a phase change emit raw
  joules. The `phase_change_energy` fix bridges the one internal case; the broader convention
  should be pinned so a future consumer wiring an energy port knows the scale.
- `net_nutrition` declares a `fermentation` port its kernel never consumes; `edibility` binds
  `dose_aggregate` to the nutrition `bio.consumer.requirement` axis rather than an aggregate toxin
  dose; `sensible_rise` has a kernel but no law entry in the floor data; and `harm_class` divides a
  mg/kg dose by a per-toxin-class tolerance whose scale R-UNITS-PIN settles (the per-class scale is
  a quantity-per-class registry entry, section 7 of the units proposal). These are the substrate's
  reader-and-spec seams, distinct from the closed-form arithmetic fixed above.

## Recommended order

The scale, precision, and logic fixes are landed; `corrosion` and the latent overflow-direction
class are now closed as well. Remaining, in order: the energy-wire-scale pin and the four
specification seams (which touch the floor data and the graph descriptor rather than the kernels),
folded in alongside the R-UNITS-PIN arc, since `harm_class`'s per-toxin-class tolerance scale is a
quantity-per-class registry entry that arc settles.
