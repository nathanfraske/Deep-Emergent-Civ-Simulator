# Stage 3 file-structure layout (Agent A surface lane), with the interior/surface disjoint boundaries

This is the design-first layout the gate asked for before a line of Stage 3 is written. Stage 3 is early-Earth
geodynamics, and the gate carved it in two: B drives the INTERIOR (mantle convection, the tectonic-regime read,
plate emergence, crustal thickness, isostasy) as its speedup slice, and A drives the SURFACE (the elevation
ledger, seed crust and relaxation, the surface-process mass balance, and the effective-elevation promotion the
hydrology and biomes read). This doc maps every file the surface lane touches, confirms the disjoint-file
boundaries against B's interior lane so the gate can relay them, and names the one interface data structure the
two lanes meet at. The owner's 2026-07-10 ruling governs the first pass: build it static (seed crust plus
worldgen relaxation), but the field must not forbid live drift added later.

## The hinge, grounded

The stage turns on unfreezing the elevation ledger, and the ledger already exists, inert by default. Verified at
source: `EarthworkField` (`crates/sim/src/material.rs`) is a `BTreeMap<Coord3, Fixed>` of per-column elevation
deltas, absent-reads-zero, pruned to zero; the physics reads the EFFECTIVE elevation as the worldgen base plus
the delta. `EnvironFields::recouple_terrain` (`crates/sim/src/environ.rs`) rebuilds the downhill hydrology
routing from that effective elevation, and returns early on an empty earthwork (the byte-neutral no-op).
`Runner::recouple_hydrology` (`crates/sim/src/runner.rs`) calls it every tick, but the earthwork is populated
only when a being lifts off founder-zero and enacts a dig or a mound, so on the default and living pin paths the
ledger is empty and byte-identical. Unfreezing it means giving the ledger a GEOLOGICAL source on the run path
(seed crust plus relaxation first, then the interior uplift read), promoting the effective elevation to the one
resident field tectonics writes, surface processes carve, and biomes read.

## The interior/surface carve (the gate's explicit ask), disjoint by file

The two lanes meet at one per-column interface and otherwise never touch the same file.

- **B, interior lane.** The heat-to-plates chain: the internal-heat W/kg axis and the radiogenic reservoir
  (already landed, #163, `geology_floor.toml` and `laws.rs` memory primitives), a solid-state Arrhenius creep
  viscosity, mantle convection through the floored thermal-buoyancy law under a fixed-iteration Stokes solve,
  the continuous tectonic-regime read over the governing dimensionless groups (Rayleigh plus a
  lithospheric-strength or volatile-weakening parameter plus an advective-to-conductive ratio, never a
  three-way enum), plate emergence as connected coherent domains of the convection-velocity field, crustal
  thickness integrated from convergence and divergence, and isostatic elevation as the buoyant-force law read
  over thickness and density. B's new code lands in its own interior module (a new `geodynamics_interior.rs` or
  a `crates/physics` convection module) plus `geology_floor.toml`; my lane edits neither.
- **A, surface lane (this doc).** The elevation ledger, its geological source (seed crust plus worldgen
  isostatic relaxation), the coupled surface-process mass balance (fluvial and chemical, impact, aeolian,
  glacial, karst, each gated on the world's regime and its surface P,T and declared solvent), and the
  effective-elevation promotion the hydrology routing and the biome classifier read. My new code lands in
  `material.rs` (the ledger, already mine), a new `geodynamics_surface.rs` (the seed-crust/relaxation source
  and the surface-process mass balance), and `environ.rs` (`recouple_terrain`, already mine, reads the promoted
  elevation). The petrology kernel (the stable assemblage and the P,T-dependent density from the per-world bulk
  composition and the local P,T, extending my Layer-0 `petrology_data.rs` and the cited `phase_registry.toml`)
  is a surface-lane physics module (`crates/physics/src/petrology.rs`) because it grows from the phase registry
  I already built; its DENSITY output is what B's isostasy reads, a producer-consumer boundary, not a shared
  edit.

## The one interface: the per-column geodynamic contribution

The two lanes exchange one per-column data structure, and it is the whole boundary. The interior WRITES a
per-column geodynamic contribution (the isostatic elevation from crustal thickness and density, and the uplift
or subsidence rate from convergence and divergence); the surface READS it as the geological input to the
elevation ledger, adds the surface-process delta (erosion lowers, deposition raises), and promotes the sum onto
the effective elevation. The petrology density flows the other way (A's petrology kernel produces the per-column
density B's isostasy reads). Each lane writes only its own field; neither mutates the other's. The shared type is
a small plain-data struct (a `GeodynamicColumn`, keyed by `Coord3`) that can live beside `EarthworkField` in
`material.rs` (the ledger's home) so the interior depends on the surface crate's data type without either lane
editing the other's kernel file. This mirrors the Layer-0 axis-versus-substance split: disjoint by file, meeting
only at a typed data boundary.

## My lane's file map, piece by piece

**(a) The elevation ledger, promoted to carry a geological delta.** `crates/sim/src/material.rs`, mine.
`EarthworkField` today carries only the being-driven dig and mound delta. The promotion adds a sibling
geological delta (or a second map on the same field) so the effective elevation is the worldgen base plus the
geological delta plus the being earthwork delta, with the geological delta empty and off by default (byte
neutral, the reference and living pins unchanged). The field must not forbid live drift: the geological delta is
a mutable per-column accumulator exactly like the being delta, so a later slice can advance it each tick from
the interior uplift read without a structural change.

**(b) The geological source: seed crust plus worldgen relaxation.** A new `crates/sim/src/geodynamics_surface.rs`,
mine. A pure deterministic first pass that seeds the geological delta from the worldgen base by an isostatic
relaxation (the crust relaxes toward the buoyancy-balanced elevation the density field implies), reading B's
isostatic contribution where it is present and falling back to a static relaxation of the base where it is not.
Off by default (an opt-in arm, like the diurnal sky), so it is byte-neutral until a scenario arms it.

**(c) The surface-process mass balance.** Same new `geodynamics_surface.rs`. One coupled mass balance over the
five modes, each a bounded closed-form integer kernel gated on the world's regime and its surface P,T and
declared solvent, so a Mars, a Titan, or a Venus runs its own dominant process rather than Earth's fluvial
default. Reads the reserved surface-process rates and the cited transport stencils; writes the surface-process
delta into the ledger. Built after the ledger and the source stand.

**(d) The petrology kernel.** A new `crates/physics/src/petrology.rs`, mine, extending the Layer-0
`petrology_data.rs` and `phase_registry.toml`. Given the per-world bulk composition (reserved, Mirror bulk
silicate Earth, McDonough and Sun 1995, the same anchor B's radiogenic isotope abundances already cite) and the
local P,T, it returns the stable phase assemblage and the P,T-dependent density by a reduced-normative
allocation first (deterministic by construction, no optimization loop), the free-energy Gibbs minimization the
determinism-heavier fork the plan flags. Its density output is B's isostasy input.

## Reserved-with-basis (surfaced, not fabricated)

The per-world bulk composition (Mirror bulk silicate Earth, McDonough and Sun 1995, the same cited anchor as
B's radiogenic abundances); the surface-process rate constants (fluvial erodibility, chemical-weathering rate,
aeolian and glacial and karst coefficients, each an empirical rate with an error band, never a per-world
steering scalar); grain size and sediment density; the impactor flux; and the genesis deep-time and
iteration-cap budgets (a performance-versus-maturity bound needing a profiling pass, each cap a fixed integer
count tested by an integer tolerance, never wall-clock or float-convergence gated). The regime-read thresholds
are B's, and the plan's discipline holds for both lanes: universal functions with error bands, never per-world
scalars, or the deleted sovereign-yield steering knob returns by the back door.

## Byte-neutral-or-stated

Every surface slice is byte-neutral by default: the geological delta is empty and the source and the
surface-process balance are opt-in arms, so the four base pins and the living pin are unchanged until a scenario
arms the geology. The first re-pin comes when an armed geology scenario runs the seed crust and relaxation, and
that re-pin is stated and measured like every other, on its own scenario, the four base pins held byte-identical.
