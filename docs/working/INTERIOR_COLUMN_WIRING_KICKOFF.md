# Interior column-wiring to the GeodynamicColumn contract (design-first)

Agent B, the high-priority integration the gate reserved for me now that A's genesis segment merged
(`2be6c3c`) and landed the `GeodynamicColumn` contract. Design-first: this opener scopes the wiring and
surfaces the seams for the gate's ruling before a line is built. Off current main `2be6c3c`.

## What is on main, and what is dormant

A's Stage-3 segment landed the shared surface-interior boundary, all of it dormant (no scenario arms it,
so the pins hold):

- `GeodynamicColumn` (`crates/physics/src/geodynamics.rs`), the minimal additive-extensible per-column
  contract: `crustal_density` (written by the surface lane), `crustal_thickness` (the isostasy input the
  interior refines), and `isostatic_elevation` (derived through the buoyancy law).
- `airy_isostatic_elevation(crustal_density, mantle_density, crustal_thickness)`, the Airy flotation law:
  `elevation = T * (rho_m - rho_c) / rho_m`.
- `GeodynamicField` (`crates/sim/src/material.rs`), the sparse resident per-column map, the
  `EarthworkField` sibling: empty by default, all-zero states pruned, and it folds nothing into
  `state_hash` while empty, so an unarmed geology is byte-identical.
- `relax_toward_isostasy` (`crates/sim/src/geodynamics_surface.rs`), the surface relaxation that reads
  `isostatic_elevation` and relaxes the effective elevation toward it.

My interior physics is on main and dormant too: the four mantle law-forms (#166), the convection-evolution
subsystem (#167, `crates/sim/src/geodynamics.rs`: `ColumnState`, `ColumnParams`, `convection_step`,
`secular_step`), and the `convective_stress` law plus the observer-side regime readout (#170).

I verified at source that `relax_toward_isostasy` has no run-path caller and `GeodynamicField` is neither
populated nor read in the runner, worldbuild, or any scenario. So the whole boundary is a dormant seam, and
the wiring can land byte-neutral.

## The wiring (what this arc builds)

The interior chain that populates the column the surface consumes:

1. Per column, the interior thermal-convection state evolves through the merged law-forms: the radiogenic
   reservoir spends down (`radiogenic_decay`) into a falling heat production (`radiogenic_heat`), which
   drives the column temperature (`internal_heat_evolution`), whose contrast against the cold reference sets
   the buoyancy (`thermal_density_anomaly`), the convective vigor (`rayleigh_number` against the derived
   onset via `threshold_latch`), the flow (`stokes_velocity`), and the convective driving stress
   (`convective_stress`). This is exactly the `convection_step` and `secular_step` composition from #167.
2. The interior writes its resident state and its `isostatic_elevation` into the column: the elevation
   through `airy_isostatic_elevation`, reading the surface lane's `crustal_density`, the world's
   `mantle_density`, and the interior's `crustal_thickness`.
3. The population walks the columns in canonical `Coord3` order (the `GeodynamicField` walk) so the fold is
   reproducible and thread-invariant.

## Byte-neutrality: achievable, and how

The wiring lands as functions (the interior chain to `GeodynamicColumn` population), fully built and tested,
but invoked by NO pinned scenario, the same dormant pattern as the interior law-forms (#166) and the
convection subsystem (#167). Nothing populates the `GeodynamicField` on the pinned runs, and nothing reads
it, so the four pins hold by construction (an empty field folds nothing). The ARMING, a genesis scenario
that runs the interior chain, populates the field, and lets the surface relaxation read it, is the later
gated step the gate sequences with A's Mirror arming (#175), so the interior and surface arm into one
coherent Mirror in a single move rather than half a world coming alive against a dormant half. Stated, not
assumed; the CI run confirms the pins.

## Seams surfaced for the gate's ruling (before building)

1. **The additive `GeodynamicColumn` extension.** The contract carries three fields today; the interior's
   deeper resident state is an additive extension A's doc anticipates. The set I propose to add: the column
   temperature (the evolved interior thermal state), a convecting flag (the one-way Rayleigh-onset latch),
   and the convective driving stress (the `convective_stress` the lid-mobilization and the regime readout
   consume). The `crustal_thickness` field already exists (the interior refines it). This changes A's shared
   contract additively, so I want the exact field set ruled before I touch it, and I keep every addition
   `Default`-zero so an unpopulated column stays the byte-neutral default.

2. **The coupling determinism (the within-tick write order).** The interior's `isostatic_elevation` reads
   the surface lane's `crustal_density`, so the boundary is two-way: the surface writes `crustal_density`,
   the interior reads it and writes `crustal_thickness` and `isostatic_elevation`, and the surface
   relaxation reads that elevation. That ordering must be pinned within a tick for determinism, and it
   couples with A's surface arming (#175) and C's redistribution-to-ledger coupling (#174) on the same
   substrate. I surface the ordering (surface density, then interior elevation, then surface relaxation) for
   the gate to rule, since the three lanes converge here and the sequence is a cross-lane determinism
   decision, not mine to fix alone.

3. **Reserved values the interior needs, surfaced with basis, none fabricated.** The `mantle_density` the
   isostasy floats the crust on (basis: the world's own mantle material density, a petrology or floor read,
   per-world, never a hardcoded 3.3 g/cm^3). The `convective_stress` shear length `L` (basis: the
   convecting-layer boundary-layer thickness or the layer depth the #167 `ColumnParams` already carries
   representable-scaled). The representable scales for viscosity and depth (basis: the same the #167
   convection subsystem uses, set equal for consistency). The tick `dt` (a caller parameter, as in #167).

## What this arc does not do

It does not arm any scenario (that is #175, gate-sequenced with A). It does not touch the surface lane's
`crustal_density` production (A's) or the ledger redistribution (C's #174). It extends `GeodynamicColumn`
only additively, on the gate's ruling of the field set. No build until the gate rules the three seams.

## Gate rulings applied, and the build status

The gate ruled all three seams (#176); seams 1 and 2 stand, seam 3 was corrected by the owner toward
derive-first.

- **Seam 1 (built):** the additive `GeodynamicColumn` fields are CONTINUOUS: `temperature`,
  `convective_stress`, and `rayleigh`, each `Default`-zero. No stored `convecting` flag: the discrete
  condition is derived from the Rayleigh number against the critical value (a false prior latch makes the
  onset reversible, so a cooling column can cease convecting). `column_readout` exposes the three continuous
  quantities; the wiring stores them.
- **Seam 2 (built):** the boundary is snapshot-apply. `populate_interior_column` reads only the start-of-tick
  snapshot (the resident temperature and the surface lane's `crustal_density`), so the boundary is
  order-independent, no cross-lane evaluation order pinned; the one-tick feedback lag is the owner-accepted
  determinism cost. `step_interior_field` folds the snapshot in canonical `Coord3` order and yields an empty
  field over an unarmed geology (byte-neutral).
- **Seam 3 (corrected to derive-first; the bridge is built, two completions flagged):** `mantle_density` is
  NOT authored. `derive_mantle_density` threads A's petrology kernel (`crustal_density`, a general
  composition-to-density derivation) over the world's mantle COMPOSITION at the mantle temperature (the
  interior heat chain's own thermal state) and pressure, so the density is what the material is under its
  conditions. Two completions are flagged, not silently authored: (a) the mantle COMPOSITION datum (Mirror's
  peridotite in the Mg-Fe-Si-O system, with citation) and the lithostatic-pressure derivation with a
  reference-pressure first pass for the density-pressure self-consistency, which arrive with the arming
  (sequenced with A's #175); (b) the `convective_stress` boundary-layer shear length, which the owner ruled
  derives as `depth * Ra^(-1/3)`, needs the deterministic fixed-point fractional-power primitive (task #45,
  not yet built), so the layer depth stands in as the reference-pass value until #45 lands. The
  truly-per-world inputs shrink to COMPOSITION and planetary GEOMETRY; every density, boundary layer, and
  stress derives.

Also noted for A (not changed here): A's `crustal_density` is not crust-specific (it is a general
composition-to-density derivation), so a neutral name like assemblage-density-at-conditions would read
truer; surfaced for A, not renamed across the shared contract unilaterally.

Byte-neutral throughout: the wiring and the extended fields are armed by no scenario, and the resident field
folds nothing while empty, so the four pins hold. What is built is the structural wiring and the derive-first
density bridge; the arming (with #175) and the two seam-3 completions (the composition datum plus P-T
self-consistency, and the `#45`-blocked boundary-layer refinement) are the sequenced follow-ons.
