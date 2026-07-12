# Genesis convection-evolution solve kernel

This is the gate-carved follow-on to the merged interior law-form lane (PR #166): the deriving subsystem that composes the four merged mantle law-forms into one per-step column update, driven by C's fixed-cap iterative solver, gated on the Rayleigh onset. Built and unit-tested against a synthetic column state, byte-neutral, with the resident-field wiring deferred to A's `GeodynamicColumn` contract (A's #160, still on the surface-process arc).

## What the merged floor already gives

Four disjoint interior law-forms landed in #166, each a pure single-relation kernel: `internal_heat_evolution` (the column heat balance), `thermal_density_anomaly` (the buoyancy source), `stokes_velocity` (the buoyant flow), and `rayleigh_number` (the convection onset). Plus the #163 memory primitive `threshold_latch` (a one-way irreversible latch). This slice composes them: the floor stays a set of single laws, and the composition into a per-step evolution lives in the deriving subsystem, the same split the productivity and matter-cycle subsystems already use.

## The one new floor relation: convective heat advection

The composition needs one physical relation the floor does not yet carry: the convective heat the buoyant flow carries out of a column. `heat_advection = specific_heat * |velocity| * |delta_t| / depth`, a specific power (W/kg), always a loss (a rising hot parcel removes heat from the interior). It is a single physical relation, so it lands as a floor law-form (`crates/physics/src/laws.rs`), a kernel plus a `KernelContract` plus a `[[law]]` row, sibling to the others, reusing `therm.specific_heat` with the velocity, temperature contrast, and depth caller-composed.

## The composition (the deriving subsystem)

A new `crates/sim/src/geodynamics.rs`, `@derives`-marked, composing the floor law-forms into a per-step column update the solver iterates:

The column state is its resident temperature and its Rayleigh-onset latch (whether convection has begun). One step reads the column's temperature contrast with the ambient, forms the buoyancy source (`thermal_density_anomaly`), the Rayleigh number (`rayleigh_number`), and the onset latch (`threshold_latch`, so convection fires once Ra crosses the derived critical Rayleigh number and stays on). When convection is active the buoyant flow (`stokes_velocity`) carries heat out (`heat_advection`), a convective loss that augments the conductive loss in the heat balance (`internal_heat_evolution`), so the column relaxes to a cooler steady state than pure conduction. When it is subcritical the loss is conduction alone.

C's `fixed_cap_solve` drives the step to a steady state: a bounded iteration with an integer residual (the temperature change), never an unbounded until-converged spin, deterministic by construction. The subsystem exposes both the pure per-step function and the solver-driven relaxation.

## Discipline and scope

No fabricated value: the convection-onset threshold is the derived critical Rayleigh number (marginal-stability eigenvalue), the Stokes coefficient is the derived 2/9, and every physical input is read from the floor or caller-composed. Determinism holds by construction (integer clock, fixed-point kernels, a monotone latch, and C's integer-residual bounded solve, no wall-clock or float-convergence gate). Byte-neutral: the subsystem is defined and unit-tested against a synthetic column state but armed by no scenario, so the canonical pins hold; the resident-field wiring (reading and writing A's `GeodynamicColumn`) and the plate-domain identity (C's `label.rs` connected-components) are the follow-on slices, sequenced behind A's contract reaching main. The files are B's own (the physics law-form and the new sim subsystem), disjoint from A's surface arc and C's units and determinism primitives. The gate runs the audit lenses before merge.
