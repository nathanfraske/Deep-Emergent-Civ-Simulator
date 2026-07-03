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

## Deferred for an owner decision

- `corrosion`: the kernel multiplies the driving margin by the raw `chem.acidity` value, so
  corrosion is maximal at pH fourteen (basic) and zero at pH zero (most acidic), the inverse of the
  acid-attack physics the cited source describes; the existing test shares the inversion. The fix
  depends on intent, and it was left unchanged rather than guessed. Either the axis is literal pH
  (the citation and the `pH` unit say so), and the kernel should make aggressiveness rise as pH
  falls, which also means baking the pH-scale ceiling into the kernel; or the axis is a loosely
  labelled aggressiveness lever where a higher value already means more corrosive, and the label and
  citation should change instead. The owner picks the reading; the fix follows in one edit.

## The latent overflow-direction class (a systemic pattern for a follow-on sweep)

The panel found one mechanical pattern across every domain: an overflow or a degenerate branch
that routes to the wrong physical extreme or is blind to sign. Each is reachable only on an
overflow or an out-of-range input, so none is wrong in normal operation, but each is a real
totality-discipline inconsistency, and the correct pattern already exists in the file (the
sign-aware `unwrap_or` of `faraday_emf`, and `sat_sub`). The instances: `satisfaction` (a positive
product overflow routes to zero rather than full), `contact_pressure` (an area overflow routes to
maximum pressure rather than near-zero), `sensible_energy` (a capacity overflow routes to the
positive cap regardless of sign, and a cooling returns a negative against a declared non-negative
bound), `ideal_gas_density` (a large gas-constant-times-temperature routes to maximum density
rather than minimum), `thermal_buoyancy` and `evaporation_rate` (sign-blind or baseline-reverting
overflow branches), and `ohm_voltage`, `solenoid_field`, `flux_linkage` (a signed product overflow
routes to the positive cap). Plus bare subtractions that can panic on extreme operands in
`bend_stress`, `axial_stress`, and `friction`. A single sweep replacing each with the established
sign-aware or saturating idiom closes the class.

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

The scale, precision, and logic fixes are landed. Next: the owner's `corrosion` decision (one
edit either way); then the latent overflow-direction sweep as one focused change across the named
instances; then the energy-wire-scale pin and the four specification seams, which touch the floor
data and the graph descriptor rather than the kernels.
