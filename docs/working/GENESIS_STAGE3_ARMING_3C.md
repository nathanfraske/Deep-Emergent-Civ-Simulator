# Genesis Stage 3 arming 3c: the producer wiring and the gravity derivation (design-first)

3c is the big integration slice and the arc's first byte-CHANGING step: it arms the derivations, so it is a stated
re-pin the gate verifies manually. This design specs the wiring before the build, per the gate, grounded against
the merged code. Two coordination constraints from the gate: the mantle-composition SOURCE must be ONE datum
shared with B's interior arming (#178), not two divergent ones; and the new pins are surfaced with their basis for
manual verification.

## The producer: composition to elevation, deriving every density

The producer flows the world's chemistry to the buoyant elevation the relaxation reads, deriving each density on
the merged kernels:

1. The bulk-silicate-Earth CRUSTAL and MANTLE compositions (McDonough and Sun 1995) are the per-world inputs, world
   data reserved-with-basis and cited, never authored densities.
2. `civsim_physics::petrology::crustal_density(composition, temperature_k, pressure_bar, registry, table)` (to be
   renamed to a neutral `assemblage_density_at_conditions`, since both lanes flagged the crust-specific name)
   minimizes each composition to its stable assemblage and returns its density. The crustal density is that read
   on the crustal composition at the surface conditions; the mantle density is the SAME kernel on the mantle
   composition at the mantle conditions, so the Airy contrast is two derived densities.
3. `civsim_physics::geodynamics::airy_isostatic_elevation(crustal_density, mantle_density, crustal_thickness)`
   returns the buoyant freeboard, written into `GeodynamicColumn::isostatic_elevation`.
4. `civsim_sim::geodynamics_surface::relax_toward_isostasy` reads that target through the `GeodynamicField` and
   relaxes the effective elevation toward it, which the four transport drivers carve.

The producer reserves NOTHING in the density-to-elevation half: it derives from the two compositions, the
conditions, and the geometry. The seed `crustal_thickness` is the one per-world geometry initial condition.

## The gravity derivation: retiring the flagship authoring-defect

The owner named the hardcoded `standard_gravity()` = 9.80665 (`runner.rs:896`) the flagship authoring-defect for
the clean starting point. It is read on the run path at `runner.rs:2377` (the gravity scalar) and `runner.rs:6824`
(a carried-weight computation), so retiring it is a byte-changing re-pin.

The derivation is `g = (4/3) * pi * G * R * rhobar`, the surface gravity of a uniform sphere: the gravitational
constant `G` is already in the floor (`civsim_units::fundamentals::GRAVITATIONAL_CONSTANT`, the one authored
place), the planet RADIUS `R` is a per-world geometry datum, and the mean density `rhobar` is the
composition-derived mean of the world's material (the same petrology density kernel, mass-weighted over the
column, or the bulk-silicate mean for a first pass). So `g` derives from `G` plus two inputs already in the
producer's hands, no authored 9.80665. The full `g = G * M / R^2` with `M` the density integrated over `R` is the
exact form; the uniform-sphere `(4/3) * pi * G * R * rhobar` is its constant-density reduction and the gate's
accepted first pass. The derived `g` is read everywhere `standard_gravity()` is read now (the three sites above),
through a single derived-gravity accessor so no call site keeps the literal.

The re-pin is stated: the derived Mirror `g` is near but not equal to 9.80665 (the uniform-sphere value omits
rotation and oblateness), so the living pin and any pin whose scenario reads gravity move. The new pins are
measured and surfaced with their basis (the derived `g` value and why it differs), and the gate verifies the move
manually, as with the segment.

## The B coordination: one mantle-composition datum

B's interior arming (#178) reads the mantle composition plus the lithostatic pressure, and my producer's mantle
density reads the mantle composition too. These MUST be one datum, not two divergent ones. The proposal, for the
gate to ratify with B: the mantle composition is a single world-data key (a manifest key or a floor field, for
example `world.mantle_composition`, the McDonough and Sun 1995 pyrolite major-element abundances), read by BOTH
the interior lithostatic and density path and the surface Airy density path. The crustal composition is a sibling
key (`world.crustal_composition`). I hold the surface read against whatever key B and the gate settle, so we never
source two mantle compositions. I will not add a second mantle-composition datum; if B has already landed one on
#178, I read that.

## The slice shape and the verify plan

3c is built after this design is gated and the mantle-composition key is settled with B. The steps, each verified:
the neutral rename of `crustal_density` (byte-neutral, no behaviour change); the world composition and radius data
(cited, reserved-with-basis); the gravity derivation and its read-site retirement (byte-changing, the stated
re-pin); and the producer that writes `isostatic_elevation` from the two derived densities and the thickness. The
verify surfaces the new pins with their basis for the gate's manual check, runs clippy strict, fmt, the
constructor gate, and the floor registry, and holds the prose customs. 3d then arms the scenario that runs the
producer and the drivers, the final living re-pin, followed by the section-9 blind panel on the armed substrate.

## The reserved list 3c adds, each hunted

Held to the owner's short-reserved-list bar: the crustal and mantle COMPOSITIONS (McDonough and Sun 1995, cited
world data), the planet RADIUS `R` and the seed CRUSTAL THICKNESS (per-world geometry initial conditions), and the
surface and mantle CONDITIONS the density reads (temperature and pressure, themselves derived from the geometry
and the geotherm where the floor supplies them). No density and no gravity is reserved: both derive. Everything
surfaced with basis, read from the manifest, failing loud on an unset value.
