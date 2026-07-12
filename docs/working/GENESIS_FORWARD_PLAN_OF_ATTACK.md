# Genesis-forward foundational arc: the staged plan of attack (gated deliverable)

This is the plan the kickoff promised. It is synthesized from a five-reader scoping fan-out (stage 1 the Sun,
stage 2 early-Earth geodynamics, stage 3 the orbit and cycles, stage 4 atmosphere emergence, and a cross-cut
over the shared physics floor and the design parts). Each reader grounded its stage against its research doc
and the current tree, tagging every item already-built, derivable-kernel, reference-data, per-world-reserved,
or gap, with what it depends on beneath it. The plan below is the causal-order assembly of those findings, and
it is a gated deliverable: no line is built until the gate gates it derive-first. Where the spec reserves a
value it is surfaced with its basis and never invented; where a real gap is found it is flagged rather than
stalling the whole.

Before trusting the readers, I verified the four load-bearing convergent claims against source myself (Prime
Directive 1, hardest on the readers' own conclusions). All four hold: the gravitational constant G is absent
from the closed `FUNDAMENTALS` table (c, k_B, h, e, eps_0, N_A only, `fundamentals.rs:53-98`); sigma and R are
DERIVED composites, not authored floor constants (`fundamentals.rs:145,159`); `solar_constant` is the inline
literal `Fixed::from_int(1361)` at `environ.rs:2461`; and `Fixed::powf` exists and is built (`fixed.rs:527`).
Each of these changes a routing decision below, so each was checked rather than taken on the reader's word.

## The one correction to the kickoff's causal order (a catch on my own document)

The kickoff numbered the stages Sun (1), early-Earth geodynamics (2), orbit and cycles (3), atmosphere (4).
That is the narrative grouping the owner named overnight, and it is the right set of stages. It is not the
build order, and the readers make the reason plain. Geodynamics is a pure function of (bulk composition, P, T),
and its P,T field reads the orbit's surface-normal insolation for temperature and the world's gravity for the
lithostatic pressure column. So geodynamics sits causally ABOVE the orbit, not beneath it: the orbit must be
standing before the geology has a temperature field to equilibrate against. The build order is therefore Sun,
then orbit and cycles, then geodynamics, then atmosphere, with one shared floor beneath all four. I am
flagging this rather than quietly reordering, because the kickoff is a posted document and the inversion is
exactly the kind of dependency seam this arc exists to avoid (it is the same class of error as the soil
bootstrap: a layer built on an unbuilt one beneath it).

There is a second, deeper seam the kickoff's five stages omit. Geodynamics and atmosphere BOTH depend on a
planet-formation input that no stage supplies: the bulk composition (the element abundances of the
silicate/ice/metal reservoirs), the initial heat budget, and the mass-and-radius that set surface gravity g
and decide whether the world kept a primary hydrogen envelope or outgassed a secondary atmosphere. The
atmosphere reader confirmed the tree has no mass-gate or radius-valley module and ASSUMES a secondary
atmosphere rather than deriving "secondary" from primary-envelope stripping. Planet formation is itself
downstream of stellar nucleosynthesis, so it is a real stage that sits between the Sun and the geology. The
honest resolution, consistent with the code-to-spec framing and the flag-a-gap-rather-than-stall discipline,
is to carry planet formation's outputs (bulk composition, initial heat budget, g) as reserved-with-basis
per-world contingency leaves now, with Mirror pinning Earth's measured, cited values, and to flag the
planet-formation derivation as the deepest deferred stage-0.5. That keeps geodynamics and atmosphere buildable
while stating plainly that their deepest inputs are reserved, not yet derived.

## Layer 0: the cross-cut floor beneath all four stages (build first)

All four stages read one shared physics floor and write through one shared discipline, and the cross-cut
reader's central finding is that the floor is not yet complete for genesis: it needs three new authoring
places, one fundamental added, a set of determinism primitives, a set of memory primitives, and a provenance
accounting layer. Nothing above Layer 0 can stand until these do, so the floor is the first build.

The three new authoring places, each a legitimate floor growth under Principle 9 (physics may be an authored
cultural input) and none of them a place where a cultural or emergent outcome is authored:

- The periodic-table plus phase-thermodynamics reference-data tier: the element list with molar mass, valence,
  and standard thermochemical reference, plus a candidate-phase registry carrying per-phase Gibbs formation
  energy, entropy, molar volume, equation-of-state and thermal-expansion parameters, and the Clapeyron solidus
  slope in K/MPa. This is an extensible data registry, sibling to the existing fifteen reference substances
  (granite, hematite, halite, iron, water, and the rest already carried as vectors, "a datum not an axis"),
  NOT a closed enum of Earth minerals and NOT a growth of the closed fundamentals list. The
  periodic-table-as-cache map shows most of the table's shape is a derivable cache of the banked
  electromagnetic sector (Pauli plus Coulomb); the authored residue reduces to the isotope list plus a thin
  decimal-and-fit layer, and a future cache arc retires several columns to derivations.
- The internal-heat-production axis in W/kg as a new floor axis, with the radiogenic decay constants (U, Th, K
  half-lives and branching) as universal reference data. A tidal heat source feeds the same axis as
  heat-per-mass regardless of origin, which is the admit-the-alien form: the geology keys on watts per
  kilogram, not on the Earth-specific radiogenic story.
- The per-world surface-pressure and surface-temperature datum. This gates whether the declared solvent is
  liquid at the surface and which surface processes run, and it co-determines inside the atmosphere loop
  through line broadening, the lapse rate, and the greenhouse integral. Source-verified refinement (see the
  verification note below): the world surface-PRESSURE datum is absent, whereas a per-world
  surface-TEMPERATURE scalar already exists as `climate.mean_surface_temperature` (287 K, category
  per-world), though it is a worldgen Kelvin-mapping offset, not a solvent-phase gate. One value-line
  subtlety I checked at source rather than taking on the verifier's word: the `P_ref = 0.101325 MPa` (one
  atmosphere) in `laws.rs` is NOT a Terran-bias surface-pressure defect. It is the matched reference pressure
  parameter of `rankine_kirchhoff_constants` (`laws.rs:1600`), which by definition is the pressure at which a
  substance's normal boiling point T_b is measured, so anchoring the saturation curve at `(T_b, 1 atm)` is
  correct physics, not an authored world pressure. The genuine gap is separate: the world's own surface
  pressure, the quantity the phase gate compares against the saturation pressure at the world's surface
  temperature, is not carried. So this Layer 0 slice is narrower than a blanket absence: add the world
  surface-pressure datum, reconcile the phase gate with the existing temperature scalar, and confirm during
  the build whether any live ambient or driving-pressure read defaults to one atmosphere (which would be the
  real surface-pressure assumption to retire, a point I verify rather than assert). This is the Terran-bias
  fix: without a world surface-pressure datum, no non-Earth world can gate its solvent phase.

One fundamental is added: the gravitational constant G. It is absent from the closed floor (verified), and the
flux derivation in stage 1 does not need it (the dimensionless mass-luminosity ratio cancels it), but Kepler's
third law for the year-from-orbit derivation in stage 3 does. G is added to the `FUNDAMENTALS` table as a
sixth-plus fundamental, cited to CODATA, so the orbit stage can derive the year rather than carrying it.

The determinism primitives the geology and atmosphere solvers need, on top of the built and GPU-proven
arithmetic (record 62.23: mul/div/sqrt/exp/ln/powf/CORDIC bit-identical on the reference GPU): canonical
connected-components labelling (plate and watershed identity keyed by the lowest cell index, worker-invariant),
deterministic priority-flood depression and sill routing (so lakes fill and overflow), and the iterative
fixed-point convection/Stokes/DAE solve with an integer-residual convergence test under a FIXED iteration cap,
never an unbounded until-converged loop. One reconciliation of a stale map here: the geology packet lists the
fixed-point fractional-power primitive as not yet built, but `Fixed::powf` exists and is a proven GPU kernel
(verified at `fixed.rs:527`); the true residual is cross-vendor transcendental confirmation, not the primitive
itself, and that residual bites only if a geology or atmosphere kernel runs on a non-CUDA GPU vendor.

The memory primitives, which are the cross-cut's biggest structural catch. The floor is memoryless today: only
`faraday_emf` and `inductive_emf` carry a one-step prior state (verified by grep of `laws.rs`), and every other
law kernel is a pure present-to-present function. Yet a large class of early-Earth geodynamics is a recorded
history rather than a present equilibrium: the thermal memory T_p decaying at its own tau kernel, the
damage-field healing kernel, inner-core nucleation, the one-way banded-iron surface-redox transition, remanent
magnetization locked at the Curie temperature, and metamorphic pressure-temperature-time paths. The atmosphere
solver's memoryless law is the same requirement from the other side: all memory must live in named
physics-state slots so the solver itself stays a pure function and hysteresis is physics rather than numerics.
So Layer 0 adds the missing law-forms and data-model slots: a world clock, an absolute-age stamp per material
parcel, an accumulator, and a one-time irreversible-threshold latch, each registering into the Part 58
conserved-projection registry. These are data-model additions, not reserved values.

The provenance-DAG accounting extends the built calibration Category test (fundamental/per-world/derivable/
defect) with the four provenance tags (measured, closure, contingency, derived) inherited up the existing
error-band DAG by worst-case join, plus a closure-reachability query that asks which derived outputs have a
closure in their transitive inputs. The operational test for a tag is whether an independent lab could refute
the number without running the sim (yes is measured, no is closure). A derived value is only as pinned as its
least-pinned input, so authorship hides in derived lines whose ancestry passes through a closure, and the
reachability query is how the arc keeps that honest. Two standing audits run over the two orthogonal axes: a
spec-to-repo status diff (built/stubbed/spec) and a closure-reachability provenance query.

## Stage 1: the Sun and its flux

Stage 1 is small, nearly standalone, and it is the gate's specific first charge: derive the surface flux from
the stellar source rather than anchoring it at the inline 1361 literal. The heat side that turns absorbed flux
into a temperature is already built and byte-neutral-tested (`radiative_equilibrium`, `surface_balance_
temperature`), the insolation geometry is built (`insolation_at`, the CORDIC cos-zenith sum over the data
star-list), and the #156 photosynthesis already reads `insolation * solar_constant` as the real irradiance. So
stage 1 changes only the watt-scale, and every downstream consumer is unchanged.

The order within the stage: first the stellar-source substrate, splitting the star's intrinsic luminosity from
the delivered flux (today `Star` at `environ.rs:2275` carries only the pre-attenuated luminosity, conflating
intrinsic L with delivered flux, so L/(4*pi*d^2) cannot be evaluated); then the mass-luminosity kernel L =
L_sun * (M_star/M_sun)^exponent, a pure fixed-point kernel in dimensionless-ratio form so it needs no G; then
the flux kernel flux = L/(4*pi*d^2), which replaces the inline literal and feeds the same insolation product
already wired. The main-sequence lifetime kernel tau proportional to M_star^-2.5 (the deep-time budget the
pre-dawn biosphere epoch gets) is lower priority and can be flagged and deferred within the stage.

Already built: the whole heat and insolation path (`radiative_equilibrium`, `surface_balance_temperature`,
`insolation_at`, `step_insolation`), the #156 photosynthesis consumer, sigma derived from k_B/h/c (the model
for how the flux constants bottom out), and the `DiurnalSky::mirror` arming that already makes the flux path
live and pinned in the living scenario. Reserved-with-basis: `solar_constant` (Mirror = Earth's ~1361 W/m^2,
NASA TSI, retired to the derived read), M_star (Mirror = 1 solar mass, a layer-4 contingency nature samples
rather than derives), the orbital distance d = a (Mirror = 1 AU), the mass-luminosity exponent (~3.5, a
closure-residue that is opacity-regime dependent from ~3 to ~5, surfaced as reserved-with-basis rather than a
single universal exponent), the reference anchors M_sun/L_sun/AU (IAU nominal values, reference data not
per-world), Bond albedo (Mirror 0.306, reference world 0 so it stays byte-identical), and back_radiation (the
night floor, its full derivation the stage-4 follow-on). Real gaps: G absent (feeds stage 3, added in Layer
0); no stellar-source substrate exists; the inline `solar_constant` literal is a Principle 11 defect with no
manifest calibration id; and the celestial model lives in a working-doc guide, not a numbered design Part with
a Decided-and-reserved blockquote, so the flux derivation has no home Part yet.

## Stage 2 (build order: after the orbit): the orbit and cycles

Half of this stage is already built and correct, and the plan should say so plainly so the build does not
re-derive what stands. The synodic-versus-sidereal day is built and correct (the hour angle subtracts the
orbital advance, so a tidally-locked world comes out as a permanent day face). Axial tilt, obliquity,
declination, and seasons are built (declination = obliquity * sin(2*pi*orbital_phase), Earth's 23.44-degree
obliquity in `DiurnalSky::mirror`), and midnight sun, polar night, and the mid-latitude summer-winter contrast
emerge from it. This built substrate is what stage 2 extends.

What is a gap: eccentricity and the orbital-distance term (today `insolation_at` multiplies a fixed
`star.luminosity` with no distance-versus-orbital-phase term, so a world radiates the same peak flux at
perihelion and aphelion), which needs a deterministic fixed-point Kepler eccentric-anomaly solve plus an
eccentricity datum and the derived distance d(nu) = a(1-e^2)/(1+e*cos nu); axial and apsidal precession (the
perihelion-versus-solstice phase that modulates the seasonal-amplitude hemispheric asymmetry); and the
Milankovitch deep-time envelope, the secular modulation of eccentricity, obliquity, and precession that Part 18
and Part 26 name as an outcome (warm ages, cold ages, baseline drift) with no physical cause. The research is
settled in the derive map (the secular frequency spectrum, obliquity stability, eccentricity damping) but the
design document has zero prose mention of Milankovitch, eccentricity, or precession, so this stage both builds
the mechanism and consolidates the settled day-night substrate into a numbered design Part with a reserved
list and a Part 62 record.

A documentation discrepancy to flag before the build assumes otherwise: the geology packet claims per-world
obliquity AND eccentricity are already carried by `celestial.rs` and #112, but only obliquity is carried;
eccentricity is not carried anywhere in the world data. The build must add the eccentricity datum, not assume
it present. Reserved-with-basis: obliquity (Mirror 0.4091 rad, already set), eccentricity e (Mirror ~0.0167,
not yet carried), semi-major axis a and central mass M (Kepler derives the year, G from the floor), spin period
(Mirror 86,400 s), and the precession phase at epoch (a per-world phase angle, load-bearing for the
season-amplitude asymmetry, not carried). The rotation rate is carried, but its atmospheric-dynamics consumer
(Coriolis, Hadley-cell width, banded jets versus a single cell) belongs to stage 4, so stage 2 carries the
datum and flags the consumer as downstream.

## Stage 3 (build order: after the orbit): early-Earth geodynamics

This is the largest and deepest stage, and it turns on one hinge: unfreezing the elevation ledger. The
`EarthworkField` (`material.rs`) and `recouple_terrain` (`environ.rs`, wrapped by `recouple_hydrology` in
`runner.rs`) both exist, and (source-verified refinement) `recouple_hydrology` is in fact called every tick in
`step_inner`, but it is inert by default: `recouple_terrain` returns early on an empty earthwork, and the
earthwork is populated only when a being lifts off founder-zero and enacts DIG or RELEASE, so the ledger is
effectively off the default path and byte-identical by default. Unfreezing it means populating it on the run
path (seed crust plus worldgen relaxation, then tectonic drift), promoting it to the single resident
effective-elevation field (base plus delta) that tectonics writes, surface processes carve, and biomes read,
under a pinned within-tick sequence (uplift, weather, transport, deposit, re-route). That is the hinge the
whole domain turns on. The owner's 2026-07-10 ruling holds: build the first pass static (seed crust plus
worldgen relaxation), but the field must not forbid live drift added later.

One shared code path runs the whole stage: the universal periodic table plus the per-world bulk composition
plus the local P,T field feed a free-energy-minimizing (or reduced-normative CIPW-style) petrology kernel that
returns, per column, the stable mineral assemblage, a P,T-dependent density field, ores, and caves. Above that,
the internal-heat-production axis and a solid-state Arrhenius creep viscosity drive mantle convection through
the already-floored `law.thermal_buoyancy`; plates emerge as connected coherent domains of the convection
velocity field (not world data that then moves); crustal thickness integrates from convergence and divergence;
and isostatic elevation is `law.buoyant_force` read over thickness and density. Volcanism and orogeny read
plate and phase state to derive eruptions (threshold = floored `mat.fracture_strength`), quakes (= floored
`mat.yield_strength`), and uplift (mass conservation from convergence), turning Part 26's authored Eruption and
Earthquake enums into derived events. Surface processes carry one coupled mass balance over five modes
(fluvial and chemical, impact, aeolian, glacial, karst), each gated on the world's regime and the per-world
surface P,T and declared solvent, so a Mars, Titan, or Venus runs its own dominant process rather than Earth's.
Phase 4 reads out a pure-geology climate (lapse rate g/c_p, orographic precipitation, a regime-selecting wind
field); the biological read-out is deferred to a separate biology lens.

A bias-control fix runs through the whole stage and the plan should hold it: the tectonic regime is a
continuous READ over the governing dimensionless groups (Rayleigh plus a lithospheric-strength/volatile-
weakening parameter plus an advective-to-conductive heat-pipe ratio), NOT a three-way mobile/stagnant/none
enum; mobile-lid, stagnant-lid, heat-pipe, episodic, and no-tectonics all fall out of the numbers.

Already built and reused: `EarthworkField` and `recouple_terrain`, the #156 mineral-weathering floor, the
convection and isostasy and conduction laws, the weathering and dissolution laws, the strength axes the
thresholds read, `chem.activation_energy`, the 726-row periodic-table molar-mass floor, the merged #112
insolation, and the pinned integer transport stencils the sediment field reuses. Reserved-with-basis: the
per-world bulk composition (Mirror = bulk silicate Earth, McDonough and Sun 1995), reference creep viscosity
(anchored to post-glacial rebound, an in-sim discoverable), radiogenic heat and isotope abundance, the surface
P/T datum, the declared solvent identity, the Clapeyron solidus slope, grain size and density, impactor flux,
the regime-read thresholds (universal functions with error bands, never per-world scalars, or the deleted
sovereign-yield steering knob returns by the back door), and the genesis deep-time and iteration-cap budgets (a
performance-versus-maturity bound needing a profiling pass on the owner's hardware, each cap a fixed integer
count tested by an integer tolerance, never wall-clock or float-convergence gated).

The real gaps here are the two largest new determinism surfaces in the project: the fixed-point Gibbs
minimization of the petrology solver (a constrained optimization, the strongest argument for the
reduced-normative fork), and the iterative convection/Stokes solve (a fixed-point matrix solve with an
integer-residual test). Both must run under a fixed iteration cap tested by an integer tolerance or replay and
worker-invariance break. Also flagged: the z=0 2.5D limit (magma chambers, crustal roots, karst conduits, and
ocean-world ice-shell layering are inherently vertical; the arc carries the column as 2.5D per-cell attributes,
a workaround that keeps the alien admissible); the memoryless-substrate gap (handled in Layer 0); ore genesis
as the least-certain emergent claim (the first pass may reach element-bearing phase distribution without
economic-grade concentration); and one input-audit catch the reader raised and I confirmed at source.

That catch (Prime Directive 2, audit the input): the geodynamics proposal calls the critical Rayleigh number "a
law-floor constant sibling to sigma and R." But sigma and R were retired from the authored floor and are now
DERIVED composites (verified at `fundamentals.rs:145,159`), so the analogy misclassifies Ra_c. The honest
classification is that Ra_c is derivable (the linear-stability eigenvalue, ~1707.8 rigid-rigid or ~657.5
free-free, Chandrasekhar 1961) or embedded in the fixed Rust regime kernel, NOT a new authored fundamental. It
is not an owner call and not per-world; the proposal's wording needs reconciling with the locked closed
fundamentals list before the regime kernel is built.

## Stage 4: atmosphere emergence (the capstone; unblocks #143)

Stage 4 sits on top of stages 1 through 3 and the geodynamics outgassing source, and it is not a new substrate:
it is a memoryless deterministic continuation solver that assembles the already-standing modules (the
surface-energy balance, the volatile-saturation curve, the still-air convection coefficient,
`PeriodicTable::molar_mass`) into one coupled fixed point. It marches the timescale ladder (fast tiers as inner
contraction solves given slow state, slow state advancing as a deterministic DAE, attractor selection by
pseudo-arclength continuation warm-started from the previous converged state), with every exchanged variable
quantized onto a lattice below its error band so the coupled iteration is a deterministic map on a finite set,
convergence is exact equality, and a limit cycle on the lattice is detected and classified as a feature
(episodic-lid IS a limit cycle). Composition and pressure co-determine inside the loop through line broadening,
the lapse rate, and the greenhouse integral; they are not post-processed readouts. Instantiated with Earth's
vector the loop exhales Earth's abiotic sky; flipped vector entries land the Venus, Mars, and Titan branches
from the same trace.

The order within the stage: the continuation-solver harness and the lattice quantization (architecture, not
values); seam one, the redox anchor as computed-with-bounded-override (accretion energy from the vector sets
magma-ocean depth, depth sets the oxidation setpoint through the measured melt-redox dG(P) curve, so one vector
entry, mass, drives both atmospheric retention and redox); the exhale-speciation proposer producing the
emergent volatile roster {x_i} as a data-defined extensible roster (mechanism fixed Rust, membership grows with
the world's volatile inventory, an alien volatile a data row); M_air over the solved roster; the hydrostatic
surface pressure p; the condensable-winner selection; and the greenhouse optical-depth column that derives the
downwelling-longwave back-radiation from the roster's opacity, replacing the flat reserved scalar and feeding
back into the built surface-energy balance. Building M_air, p, and the condensable selection unblocks the #143
runtime physicalization as a byproduct, because #143 is blocked precisely on M_air and p not existing.

Already built and assembled: `surface_balance_temperature`, `radiative_equilibrium`, `convective_flux`, the
exact Rankine-Kirchhoff saturation family and the three-regime `VolatileSaturationCurve`,
`free_convection_a_still`, `virtual_density_buoyancy` (already parameterized on M_air),
`PeriodicTable::molar_mass`, and the live surface-balance and hydrology wiring. Reserved-with-basis: the model
closures (escape efficiency eta ~0.1, thermospheric heating efficiency, the K_zz eddy-diffusion profile, the
WHAK weathering barrier set, the H2O-continuum parameterization, the mobile-lid friction band and lambda ~0.9),
each a real free knob distinct from the measured floor; the near-critical boundary tolerance (an
engine-accuracy value derived from T_c and the reserved tolerance, never a literal); the lattice spacing per
variable (a numerical-error-band budget); and back_radiation, solar_constant, and albedo as the reserved
stand-ins the arc derives. The measured-floor additions (the melt-redox dG(P) curve, the E_HB donor-class
constants F/O/N ~29/21/13 kJ/mol) are reference data, refutable without the sim.

Real gaps: the atmospheric composition {x_i}, M_air, and the greenhouse column are carried nowhere today
(confirmed grep); the entire genesis chain beneath the roster is design-ahead of the tree (no mass-gate, no
magma-ocean fO2 setpoint, no outgassing source, no escape/weathering/photochemistry sinks); the continuation
solver and lattice quantization are architecture spec only; and one verification flag the reader raised
plainly: recent high-pressure work reports a reversal in the depth-to-fO2 pressure dependence, so the
"super-Earths more oxidized" staircase is clean over the terrestrial range but bounded, not guaranteed
monotone, at super-Earth pressures.

## The reserved-with-basis ledger (never fabricated)

Every value below is surfaced with the basis on which the owner would set it, and none is invented. The
per-world contingency leaves (Mirror pins the cited Earth value): M_star, semi-major axis a, eccentricity e,
obliquity, spin period, precession phase at epoch, bulk composition, isotope abundance, impactor flux, surface
pressure, surface temperature, declared solvent identity, Bond albedo. The closure-residue and closure knobs
(surfaced with an error band, never a per-world scalar): the mass-luminosity exponent, the regime-read
thresholds, the atmosphere closures (eta, K_zz profile, WHAK barriers, H2O continuum, mobile-lid lambda), the
metal-silicate equilibration efficiency. The engine-accuracy and performance bounds (a computed budget, not a
realism value): the lattice spacing per variable, the near-critical boundary tolerance, the genesis deep-time
and iteration-cap and steady-state-tolerance budgets. The reference-data leaves (measured, refutable without
the sim): M_sun/L_sun/AU, the radiogenic decay constants, the Clapeyron solidus slopes, grain size and density,
the melt-redox dG(P) curve, the E_HB donor-class constants, G. Each reserved value's full basis is in its
stage above; the consolidated list reaches code as named manifest ids that fail loud when unset, per the
reserved-value runbook, never a silent default.

## The real gaps (flagged, not stalled)

The gaps the four stages surface, grouped by whether they block the build or ride alongside it. The blocking
prerequisites, all in Layer 0: G on the floor (blocks Kepler in stage 2); the surface-P/T datum (blocks the
solvent-phase gate in stage 3 and the coupled loop in stage 4); the internal-heat-production axis (blocks the
geodynamics thermal engine); the memory primitives (block the recorded-history geodynamics and the memoryless
solver); the determinism primitives (block the plate and watershed labelling and the two iterative solves). The
deferred deeper stage: planet formation (bulk composition, initial heat budget, the mass-gate that decides
primary versus secondary atmosphere), carried as reserved leaves now and flagged as stage-0.5. The
consolidation gaps: no numbered design Part for the celestial or the atmosphere substrate yet (each needs a
Decided-and-reserved blockquote, a Part 62 record, and a Part 63 bibliography once its mechanism is built). The
honest-limit flags that do not block: the z=0 2.5D limit, ore genesis as the least-certain emergent claim, the
fO2 monotonicity verification flag at super-Earth pressures, and the cross-vendor transcendental confirmation
residual.

## Source-verification of the plan's load-bearing claims (Prime Directive 1)

The plan rests on roughly twenty factual "already-built / gap / reserved" classifications drawn from the
five-reader fan-out. Because a reader is a lead generator and not a verdict, every load-bearing claim was
adversarially re-checked against the real tree by a dedicated verifier charged to refute it (in addition to the
four I checked by hand before posting: G absent, sigma and R derived, the 1361 literal, `Fixed::powf` built).
Eighteen of the twenty confirmed at source. Two refined, and both refinements are folded into the stages
above. First, the surface-pressure and surface-temperature datum (Layer 0): the world surface-pressure datum
is absent, while a per-world surface-temperature scalar already exists as `climate.mean_surface_temperature`,
so the slice is narrower than a blanket absence. A second-order check on the verifier's own conclusion belongs
here: the verifier flagged the `P_ref = 0.101325 MPa` in `laws.rs` as a hardcoded surface-pressure defect, but
grounding the site shows it is the matched reference-pressure parameter of `rankine_kirchhoff_constants`
(`laws.rs:1600`), the pressure at which a substance's normal boiling point T_b is defined, so anchoring the
saturation curve at `(T_b, 1 atm)` is correct physics and per-substance reference data, not an authored world
pressure to retire. The genuine gap is the separate world surface-pressure quantity the phase gate compares
against saturation. Second, the elevation ledger (Stage 3): `recouple_hydrology` is called every tick but is
inert by default via the empty-earthwork guard, so the ledger is off the default path by inertness rather than
by being uninvoked. Neither refinement
reverses a conclusion; both sharpen a build slice. No claim was refuted.

## Discipline and the gate ask

The build, when it starts, is byte-neutral-or-stated per slice: every re-pin of the five canonical scenarios is
enumerated for the owner and never tuned to a target. Each major milestone gets the section-9 lenses run once
by me (the standing cost directive; the gate runs the adversarial audit). The two guardrails hold at every
stage: a reserved value is surfaced with its basis and never invented, and a real gap is flagged rather than
stalling the whole. The parallelizable disjoint-file slices, once the plan is gated, are the gate's to carve to
B as the overnight speedup while I hold the main thread.

The gate ask is the derive-first gate the kickoff promised: gate this staged plan before a line is built. The
specific rulings the plan needs are the two structural seams I flagged against my own kickoff (the build-order
inversion, orbit before geodynamics; and the missing planet-formation stage carried as reserved leaves), the
input-audit catch (Ra_c reclassified as derivable, not a floor constant sibling to sigma and R), and the owner
Class-C forks the geology reader surfaced (full Gibbs versus reduced-normative petrology; petrology now versus
an authored-substance registry now; the declared-solvent generalization; first-class impact cratering). Once
gated, Layer 0 is the first build, and Stage 1's flux derivation is the first slice on top of it.
