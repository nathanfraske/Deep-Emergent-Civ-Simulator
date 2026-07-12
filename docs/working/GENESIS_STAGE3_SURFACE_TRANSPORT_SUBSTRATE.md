# Genesis Stage 3 surface lane: the data-defined surface-mass-transport driver substrate (design-first scope)

This is the grounded design the gate ruled and approved on PR #160. It SUPERSEDES the five-mode surface-process
framing in `GENESIS_STAGE3_LAYOUT.md` section (c). The reframe was caught by the project's own discipline: a blind
section-11 input-bias smoke test, run on the five-mode construction before any build, failed closed and found the
five modes (fluvial, aeolian, glacial, karst, impact) to be a list curated by Earth landform salience rather than a
partition of the physical drivers of surface mass transport (a Principle 8 and Terran-bias defect), and found that a
pure-erosion set with no deposition cannot close its mass budget (a conservation defect). A second, hardened
construction also failed closed, and its findings, all verified against source, reshaped the design further. This
document is the result: the design written directly from the verified facts, per the gate's ruling, with the
build-time section-9 five-lens panel deferred to the complete geological-source segment boundary.

The governing rule the gate set is that this is a DATA-DEFINED, EXTENSIBLE substrate, a sibling of the value,
semantic, and provenance substrates: the transport-and-deposition solve over the driver kernels is fixed Rust, and
the driver MEMBERSHIP is data that grows with the world. A driver is a data row, never a hardcoded enum of Earth
landforms. The hardening the gate required, and that the second smoke test caught still smuggled once, is that the
row must not carry a closed set one level down (a fixed property tuple or a closed kernel vocabulary presented as
open data).

## Why a driver partition, not a mode list

Surface mass transport is the movement or alteration of regolith and rock across the surface. The physical drivers
of that movement partition the space: gravity acting downslope with no fluid, a moving fluid exerting shear,
thermal and chemical alteration in place, phase change moving a volatile's own mass, ballistic delivery from
outside the world, and biological reworking. Each driver keys on the world's own physical data (its fluids and
solvents as property data, its surface pressure and temperature, its lithology, its gravity), so a Mars, a Titan, a
Venus, and an Earth each run their own dominant driver as a data row rather than through an Earth default. The
partition also closes the mass budget, because deposition is the sink that balances every transport driver's
source. The five Earth modes are subsumed rather than lost: Mirror's water rivers and its wind are two cases of the
one fluid-shear driver, so Mirror's calibration is preserved while the substrate generalizes.

The generality is scoped to the KERNEL, the mechanism, not to the roadmap. The build order below is
Earth-frequency-ordered: the four drivers built first are the ones that dominate a warm, wet, silicate world, and a
cold volatile-rich world or an airless world is not correctly simulable until its deferred row lands. That ordering
is stated plainly here so that "alien-general" is not read as "every world runs today."

## The substrate contract: what a driver row carries

A driver row is a plain data record carrying five things, and the shape of each is where the alien-generality and
the determinism live.

First, its transport-law FORM, expressed in the fixed primitive vocabulary (below) rather than as free Rust. A row
names the kernel or the small op-graph its physics needs; the solve evaluates it. A driver whose form is expressible
in the vocabulary is a data row; a driver whose form needs a primitive the vocabulary lacks is a named floor
extension, not a data row, and the contract states that boundary plainly.

Second, its forcing read, over an OPEN NAMED KEY-SET rather than a fixed tuple. The fluid or solvent a driver reads
is described by named property keys, and the set is extensible: the geology packet's solvent-substance already
carries density, viscosity, surface tension, latent heat, and boiling point, and a driver may read a further key (a
saturation curve, a chemical aggressiveness, a triboelectric charge for electrostatic dust transport, a Bingham
yield stress for a mud or a lava, a thermal conductivity). The extensibility boundary is exact: a driver reading a
new PROPERTY key is a data row, but a driver needing a new PRIMITIVE is a floor extension. This is the correction to
the smuggled six-tuple the second smoke test caught, where a fixed property vector would have made a
charge-driven or a rheology-driven driver impossible as data.

Third, its PRIMITIVE, drawn from the vocabulary below, or a flagged missing one for a non-local driver.

Fourth, the conservation RESERVOIRS its mass touches, drawn from the four-reservoir ledger below, so a driver that
removes mass from the column states where that mass goes.

Fifth, its coupling to the shared elevation ledger, through the snapshot-apply tick below, so the write is
deterministic and conserves across lanes.

The mechanism that composes the rows, the transport-and-deposition solve, is fixed Rust and is the same for every
world; only the membership grows. The contract must NOT author a per-world driver, must NOT hardcode a closed enum
where world content should emerge, and must NOT presume a fluid or a chemistry.

## The primitive vocabulary (verified in the codebase)

The fixed vocabulary the driver forms compose over, each checked at source, is the honest boundary of the
data-extensibility. Three deterministic primitives are built and available: priority_flood
(`crates/world/src/flood.rs`, depression-fill and downstream drainage receivers over an integer elevation grid),
fixed_cap_solve (`crates/world/src/solve.rs`, a fixed-cap integer-residual bounded relaxation), and a canonical
connected-components labelling (`crates/world/src/label.rs`, basin and domain identity keyed by lowest cell index,
worker-invariant).

The fractional power that the fluid-shear and settling laws need is a constrained case, and the constraint is the
load-bearing design fact the smoke test surfaced. A general fixed-point fractional power (an arbitrary exponent) is
an unresolved GPU-CANON determinism gate (R-GPU-CANON-PIN, CubeCL bit-identity on the target GPU), flagged in
`GEODYNAMICS_ARC_PROPOSAL.md`, `GEOLOGY_ARC_PACKET.md`, and `GEODYNAMICS_RESEARCH_SURFACE.md`, and the deep-time
genesis pass is meant to run on that GPU. The docs state the constraint plainly: until the primitive is built the
incision-law form is constrained to exact-root exponents or deferred. The exact-root exponents are resolved and
GPU-canon: mul, div, and sqrt are pinned, and `Fixed::powf` (computed as `exp(y * ln x)` over the pinned ln and exp)
is CPU-deterministic and already on the run path. So every law here is built in the exact-root form (a half power is
a sqrt, a first power is linear, a cube root is composable), which is CPU-deterministic and GPU-portable now, and
the general arbitrary-exponent form is deferred behind the GPU-canon fractional-power primitive (the gate's task
#45). A law that would need the general power does not yet reduce to a proven GPU-canon kernel, and the design says
so rather than hiding it.

A non-local, ballistic redistribution primitive (deposit at a distance set by momentum, not at an adjacent cell)
does not exist. Any driver whose transport is non-local (impact ejecta, and the runout of a fluidized or granular
mass flow) names this missing primitive as a floor extension rather than force-fitting an adjacency dump.

## The conservation ledger: four reservoirs, not one sink

The mass budget the substrate closes is over four reservoirs, not column-to-column deposition alone. The second
smoke test caught that a single-sink budget cannot close for the alien cases the design itself cites. The four
reservoirs are the column solid (the elevation ledger the surface writes), the dissolved-load reservoir (mass a
chemical driver removes into solution before it precipitates as chemical sediment), the atmospheric-vapor reservoir
(mass a phase-change driver moves as vapor while it is in transit between sublimation and redeposition), and
permanent loss to space (a volatile sublimated above escape velocity, or impact ejecta above escape, on a
low-gravity world). A driver states which reservoirs its mass enters and leaves, and the solve requires the total
over all four to be conserved under integer arithmetic, so no cell silently creates or destroys mass. The accounts
are defined now, at the scaffold, even though the first drivers built move mass only column to column, so that the
deferred volatile and impact rows do not break the closure claim when they land.

## The snapshot-apply tick: the EarthworkField promotion and cross-lane determinism

The shared elevation ledger the whole contract couples to is `EarthworkField` (`crates/sim/src/material.rs`), which
today is the being dig-and-mound delta and is off the run path. The genesis-forward Stage 3 surface slice 1 added a
geological-delta sibling (`total_delta`, `adjust_geological`) as the start of its promotion, but the promotion to
the single resident field that the interior tectonics writes and the surface transport carves is not yet built, and
the within-tick write sequence and the cross-lane conservation it needs must be specified rather than asserted
(`GEODYNAMICS_ARC_PROPOSAL.md` itself flags the within-tick sequencing as asserted, not demonstrated).

The gate's owner-locked discipline for this is the snapshot-apply tick. Every writer, the interior tectonic lane and
each surface transport driver, computes its delta against the tick's SNAPSHOT of the elevation, not against a value
another writer may already have moved within the same tick. The apply phase then reconciles all the deltas, and a
column contested by more than one writer splits by exact integer apportionment (the field invariant holds and the
split is derived by a physical draw, never a traversal-order accident). This closes the within-tick determinism the
proposal only asserted, and it makes the interior-versus-surface write to a shared column a conserved, order-
independent reconciliation. It is the same double-buffer-and-gather shape the determinism mechanism below requires
of each single driver, lifted to the cross-lane tick.

## The determinism mechanism each driver must show

Two constraints secure the exact-closure and the order-invariance the substrate claims, and each driver states them
rather than asserting the target. First, per-iteration mass conservation independent of convergence: a
fixed_cap_solve stopped at its cap mid-residual still conserves total mass, so the bounded solve never leaks. Second,
a gather or double-buffer update, so a single pass is order-independent: no cell reads another cell's already-updated
value within the pass, and the result is identical across worker splits and replays (Principle 3, Principle 10).

## The build-now core: four drivers that close the continuous budget

The four drivers built first close the continuous surface mass budget, and gravity-downslope is foundational because
it sets the slope term the other drivers read.

The GRAVITY-DOWNSLOPE driver is gravity moving regolith down a slope with no transporting fluid, the continuum from
grain-by-grain creep through the diffusive hillslope limit to threshold slope failure (a landslide). Its law form is
hillslope diffusion (a mass flux proportional to slope) in the diffusive regime, with a threshold-failure branch
where the slope exceeds the regolith's critical angle. It uses fixed_cap_solve for the diffusive relaxation, keys on
surface gravity, slope, and the regolith depth and cohesion, moves mass column to column (the column-solid
reservoir), and sets the local slope the fluid-shear and solid-solvent drivers read. Its 2.5D limit is that a
heightfield cannot hold an overhang or a failure plane, so a landslide is a surface lowering and an adjacent raising,
not a modeled slip surface.

The FLUID-SHEAR driver is a moving fluid, a condensed liquid or an ambient gas, exerting boundary shear that
entrains and transports grains. Its law form is an entrainment threshold (Shields for a dense viscous fluid, Bagnold
for a thin one) and a transport capacity above threshold, with the flowing fluid routed over the terrain by
priority_flood. Whether one threshold-and-capacity form spans both the liquid and the gas grain-Reynolds regimes, or
switches with the fluid property key-set (so a Titan methane river and a Venus dense-carbon-dioxide wind are one row
with a regime branch or two rows), is a decision the grounding leaves to the build, keyed on the fluid data rather
than on a named fluid. The capacity form is built in the exact-root exponents (a half power on discharge is a sqrt, a
first power on slope is linear), CPU-deterministic and GPU-portable now, with the general arbitrary-exponent form
deferred to the GPU-canon primitive. It keys on the fluid property key-set, the shear or discharge, the mobile grain
size, and gravity, entrains from the column solid and carries mass as suspended and bed load to deposition. Its 2.5D
limit is that channel form below the surface (an undercut bank, a canyon overhang) is not represented.

The THERMAL-CHEMICAL ALTERATION driver alters the bedrock in place, and is the source the transport drivers
presuppose. It has two limbs: chemical dissolution and weathering, which removes mass into the dissolved-load
reservoir on a temperature-dependent (Arrhenius) kinetic keyed on the solvent's chemical aggressiveness and the
lithology's solubility, whose pre-factor and even whose meaning must be tested against a non-water solvent rather
than assumed; and thermal and frost fracturing, which produces the mobile grains the fluid-shear and gravity-
downslope drivers move, keyed on the diurnal temperature range and the solvent's freeze expansion. The karst
sub-case, dissolution of a soluble lithology into conduits, is where the 2.5D limit bites hardest, because a conduit
is inherently three-dimensional and the heightfield holds only its surface collapse. This driver is the answer to
the grain-source gap the smoke test found in the five-mode framing, where the aeolian mode presupposed grains that
nothing made.

DEPOSITION is the conservation sink that closes the budget. It is the negative half of every transport driver:
where a fluid slows, a slope flattens, or a basin fills, transport capacity drops and the excess load settles, sorted
by grain size (a settling law in the exact-root form). The priority_flood filled basins are the deposition sites for
the fluid case, and deposition into the column solid closes the column-to-column half of the budget while the
dissolved-load, atmospheric-vapor, and loss-to-space accounts close the rest. Its 2.5D limit is that deposition
layers stratigraphy, which a single elevation per column cannot record, so the substrate tracks the surface, not the
buried column.

## The deferred rows: data rows behind stated boundaries

Each deferred driver is a data row the substrate absorbs, held behind a stated boundary, and each names the worlds
not correctly simulable until it lands.

SOLID-SOLVENT FLOW (glacial) is a frozen solvent accumulating and creeping under gravity, abrading and quarrying its
bed, active where the solvent's solid phase is present at surface pressure and temperature. Its boundary is that
phase condition; a world too warm for its solvent to freeze does not run it.

BALLISTIC and EXOGENIC (impact) is non-local crater excavation and ejecta, and it is the clearest case of the
missing non-local redistribution primitive: ejecta deposits at a distance set by the impact energy and gravity, not
at an adjacent cell. It is deferred behind that primitive, and it is episodic and off the continuous-balance
critical path, so the missing primitive is a stated boundary rather than a blocker. Its mass can exceed escape
velocity, touching the loss-to-space reservoir. An airless or thin-atmosphere world, where impact dominates and the
atmosphere does not filter the impactors, is not correctly simulable until this row lands.

BIOLOGICAL reworking (bioturbation, biogenic soil production, root wedging) is a distinct driver that couples to the
biosphere lane, and it is deferred behind that cross-lane boundary rather than built inside the surface solve.

VOLATILE PHASE-CHANGE TRANSPORT moves a volatile's own mass by sublimation, then vapor migration along the
saturation gradient, then redeposition. Its activation derives from the volatile's saturation curve against the
surface temperature and pressure (active where the sublimation point sits near surface conditions), a property-vector
regime rather than a world list, and it reuses the Rankine-Kirchhoff saturation curve already built. It is distinct
from fluid-shear (which entrains foreign grains in a flowing fluid) and from thermal-chemical alteration (which
fractures and dissolves in place), because it transports the volatile itself through the atmospheric-vapor
reservoir, with an escape path to loss-to-space. A cold volatile-rich world (a carbon-dioxide-capped, a
nitrogen-capped, or a cometary surface) is not correctly simulable until this row lands.

GRAVITY-DRIVEN FLUIDIZED and GRANULAR MASS FLOWS (debris flows, lahars, pyroclastic and turbidity currents,
avalanches, and lava-flow runout) are a distinct driver between dry hillslope and clear-fluid shear, whose defining
feature is long non-local RUNOUT set by momentum and mobility rather than by local slope or adjacency. This is the
further omission the second smoke test's completeness lens surfaced (PD5 continuing on the driver partition itself).
It shares the impact row's missing non-local primitive and additionally needs a non-Newtonian (Bingham or granular)
rheology property key, and it is deferred behind both.

## Reserved values, surfaced with their bases, never fabricated

Every rate constant the drivers need is surfaced here as reserved-with-basis, an empirical rate with an error band
or a universal function, never a per-world steering scalar, and never invented. The hillslope diffusivity (the
transport efficiency of creep, a per-material rate with an error band, calibrated against measured hillslope
relaxation). The fluvial and aeolian erodibility K and the transport-capacity coefficient (empirical rates with wide
error bands, the erodibility notoriously so, to be surfaced with that honest spread). The chemical-weathering and
dissolution rate constants (Arrhenius pre-factors and activation energies per lithology and solvent, cited kinetic
data). The thermal and frost fracturing rate (a per-material fatigue rate keyed on the diurnal range). The settling
grain-size distribution and sediment density. The critical slope angle (a per-regolith failure threshold, the
material and cohesion data the floor already carries for fracture). The genesis deep-time and iteration-cap budgets
(a performance-versus-maturity bound needing a profiling pass, each cap a fixed integer count tested by an integer
tolerance, never wall-clock or float-convergence gated). None of these is set here; each is surfaced with the ground
on which the owner would set it, and the mechanism reads them from the calibration manifest, failing loud on an
unset value rather than defaulting to a plausible number.

## The slice plan

The build proceeds in byte-neutral opt-in slices, each leaving the five canonical pins bit-identical until an armed
geology scenario runs the transport, so the re-pin is stated and measured on its own scenario when it comes.

The first slice is the scaffold: the substrate contract (the driver-row record and the fixed-Rust solve skeleton),
the four-reservoir conservation ledger, and the snapshot-apply promotion of the elevation ledger, all off the run
path with no armed driver, so byte-neutral by construction. Then the four core drivers land one at a time,
gravity-downslope first (it sets the slope the others read), then fluid-shear in the exact-root form, then
thermal-chemical alteration, then deposition, each a data row over the scaffold, each byte-neutral until a scenario
arms it against a synthetic composition. The full section-9 five-lens blind panel runs on the concrete built
substrate at the complete geological-source segment boundary (the transport substrate plus the isostatic relaxation
plus the McDonough and Sun bulk-silicate-Earth producer that fills the isostatic elevation), per the gate's ruling,
with both restored shaping-catcher lenses and the correctness lenses. The deferred rows land later, each behind its
stated boundary, when its primitive (the non-local redistribution, the GPU-canon general fractional power) and its
lane (the biosphere) are ready.

## The determinism and honest-limits summary

The substrate is deterministic by the two mechanism constraints (per-iteration conservation and the gather or
double-buffer pass) and the snapshot-apply cross-lane tick, over the exact-root exponent forms that are CPU-pinned
and GPU-canon. The honest limits stand and are not hidden: the general arbitrary-exponent fractional power is an
open GPU-canon gate, so the incision and settling laws are built in exact-root form until it lands; the non-local
redistribution primitive does not exist, so impact and mass-flow are deferred; the heightfield is z=0, so conduits,
overhangs, ballistic arcs, and buried stratigraphy are surface projections, not modeled volumes; the build order is
Earth-frequency-ordered, so cold-volatile, airless, and lava worlds are not correctly simulable until their deferred
rows land; and the EarthworkField promotion is specified here but not yet built, so the run-path resident field and
its cross-lane reconciliation are the scaffold slice's work.
