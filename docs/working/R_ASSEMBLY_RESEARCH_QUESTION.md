# R-ASSEMBLY: the planetary-system assembly law (research question)

Status: flagged, not built. Slice 1 of the solar-system generator (#72) landed the oligarchic embryo field
(`crates/sim/src/planetary_system.rs`): the number, masses, and spacing of proto-planets fall out of the disk. The
next slice, the giant-impact assembly that merges those embryos into the final planets, carries a real derive-versus-
author and determinism-versus-chaos seam. This document scopes what I need resolved before I build it, as exhaustively
as I can state it. The owner's condition, as always: the resolution must be broadly generalizable, per-system and
alien-admitting, deterministic, and minimally authored (a value enters only as a fundamental constant, a per-system
initial condition, or a derived quantity, never a fabricated fit).

---

## 1. The gap, grounded in what is built

What slice 1 derives, so the packet is precise about where the frontier is:

- `oligarchic_embryo_field` sweeps the disk from the inner edge, placing an embryo at its local isolation mass
  (`astro::isolation_mass_earth`, the Kokubo-Ida oligarchic mass over the local solid surface density), then stepping
  one oligarchic spacing outward in the embryo's own Hill radius (`astro::hill_radius_au`).
- `SolidDisk` derives the solid reservoir: the gas surface density (`astro::disk_surface_density`, the Lynden-Bell and
  Pringle self-similar profile) times the condensed metal fraction, the per-system metal fraction Z beyond the derived
  ice line and its refractory share inside. The ice line is `astro::disk_effective_temperature` bisected to the water
  snow-line temperature the condensation table carries (182 K, read not authored).

The single reserved value on that path is the oligarchic spacing width `b` (about 10 mutual Hill radii, Kokubo-Ida).
Everything after it, the assembly, is unbuilt. That assembly is the subject of this question.

## 2. The core question

Given the derived oligarchic embryo field, how does the giant-impact phase assemble it into the final system of
planets, in a way that is deterministic (Principle 3), emergent rather than authored (Principle 8), per-system and
alien-admitting (Principle 7), and minimally authored (Principle 11)? The final planets are fewer, more massive, and
more widely spaced than the embryos, and their number, masses, spacing, and eccentricities must fall out of the disk
and the physics, not be painted on.

The crux, stated sharply: the giant-impact phase is chaotic (the specific final configuration is exponentially
sensitive to initial conditions), yet its statistics (the final count, the mass distribution, the spacing
distribution, the angular-momentum deficit) are robust and predictable from the initial disk. So I need a ruling on
what "emergence" means here, and a physical law for where the merging stops.

## 3. The sub-questions

### A. Is the oligarchic spacing `b` derivable, or an irreducible cited constant?

I currently reserve `b` at about 10 mutual Hill radii (Kokubo-Ida 1998, 2000). The physical picture is a balance
between orbital repulsion (embryos scatter each other apart) and random-velocity damping (dynamical friction from the
planetesimal sea and the gas). The questions:

1. Does `b` derive from a ratio of timescales (repulsion versus damping) or from the eccentricity equilibrium the
   embryos reach, or is it fundamentally a numerical result with no closed form?
2. Is `b` truly near-universal (roughly 10 across disk conditions, per Kokubo-Ida), which would make it a cited
   dimensionless constant, or does it vary with disk properties (surface-density slope, planetesimal size, gas
   presence), which would make it per-system data with a Mirror-pinned default?
3. Buckingham-Pi framing: `b` is dimensionless. Which dimensionless groups does it depend on (the Safronov number, the
   ratio of the embryo escape velocity to the local Keplerian velocity, the planetesimal-to-embryo mass ratio, the
   gas-damping-to-growth timescale ratio)? Enumerating those bounds how many authored inputs `b` may legally hide.

### B. What sets the final number, masses, and spacing (the stability law that ends the merging)?

After the gas disperses and the planetesimal sea thins, the undamped embryos pump each other's eccentricities until
orbits cross and bodies merge, over roughly 10 to 100 Myr for the terrestrial zone. The merging stops when the
survivors are dynamically stable for the system's age. The questions:

1. The two-body Hill-stability criterion (Gladman 1993) is analytic: two planets on circular orbits are Hill-stable if
   their separation exceeds 2*sqrt(3), about 3.46 mutual Hill radii, from the conservation of energy and angular
   momentum in the restricted three-body problem. But it is a two-body, sufficient-only condition. Is there a
   defensible analytic multi-body extension, or is the multi-body stability fundamentally empirical?
2. The empirical multi-body stability relation (Chambers, Wetherill and Boley 1996; Smith and Lissauer 2009; Pu and Wu
   2015; Obertas, Van Laerhoven and Tamayo 2017): a system survives roughly a Gyr if the spacing exceeds about 8 to 10
   mutual Hill radii, with a logarithmic dependence on the integration time and dependences on N, eccentricity, and
   mass. Is this the law that sets the final spacing, or is the observed terrestrial spacing (about 20 to 40 mutual
   Hill radii) WIDER than this floor because the giant-impact phase over-relaxes? If it over-relaxes, what sets the
   over-relaxed spacing?
3. Can the final spacing be DERIVED from an energy or angular-momentum argument about the relaxed end state (a
   maximum-entropy or minimum-crossing argument), or is it only reproducible as a distribution from N-body statistics?

### C. The determinism-versus-chaos realization (the ruling I most need)

The giant-impact phase is chaotic: nearly identical embryo fields produce different final systems (a Lyapunov
sensitivity). This collides with Principle 3. The candidate realizations, and the principle question each raises:

1. Full N-body integration in fixed-point arithmetic with a fixed seed. Deterministic and maximally emergent, but
   expensive (order N-squared per step, order 1e7 to 1e8 steps for 100 Myr) and the chaos means the result is one
   seed-dependent draw. Our fixed-point determinism is bit-identical across platforms, which the chaos would otherwise
   amplify into divergence, so this is feasible in principle but heavy.
2. A seeded statistical realization: do not simulate the chaos, but sample the final system (N, masses, spacing,
   eccentricities) from the outcome distributions the N-body literature establishes as robust functions of the initial
   disk (Kokubo and Ida 2006; Raymond, Quinn and Lunine 2009; Hansen 2009's dissipative-collapse annulus). The
   distribution is derived from the disk; the specific draw is seeded from the world identity. Cheap and reproducible.
3. A deterministic rule-based merge: relax the embryo field by merging adjacent unstable pairs (conserving mass and
   angular momentum) until every survivor is stable by the criterion in B, with seeded tie-breaks. Cheap, deterministic,
   emergent in structure, but the merge rule is a proxy for the chaos and the stability threshold is reserved.

The ruling I need: is option 2 (a physically-derived outcome distribution, sampled by a deterministic seed) an
acceptable emergence under Principle 8, on the ground that the statistics emerge from the disk and only the specific
realization is a seeded draw? Or does honesty demand option 1 (or 3), on the ground that sampling a distribution
authors the outcome? This is the same class of question as the giant-impact-versus-oligarchic determinism split, and I
do not want to pick the interpretation myself. A useful sharpening: the chaos is in the DETAILS, but the final count,
the mass function, the spacing distribution, and the angular-momentum deficit are statistically predictable from the
disk (this is a real result, not a hope), so "the statistics are derived, the realization is seeded" is a defensible
reading. I want your read on whether it is the RIGHT reading.

### D. The merge bookkeeping (mass and angular momentum, and the collision-outcome spectrum)

When two bodies collide, the simplest model is a perfectly inelastic merge (all mass retained, the merged orbit set by
angular-momentum conservation). The modern picture (Leinhardt and Stewart 2012) is a spectrum keyed on impact
velocity, angle, and mass ratio: merge, hit-and-run, erosion, or disruption. The questions:

1. Do we model the collision-outcome physics (the Leinhardt-Stewart pi-scaling of the disruption criterion), which is
   more derived and admits the alien (different materials, different strengths), or treat merges as perfectly
   inelastic as a first pass? We may already have adjacent machinery: the crater-scaling pi-groups (#70) and the
   impact-flux model (#71).
2. The escaping ejecta and hit-and-run fragments feed the small-body reservoir (#74). How much of the mass budget
   should the assembly leak to fragments rather than retain in planets?

### E. The eccentricity and inclination end state

The final planets are near-circular and near-coplanar. What damps them to that state (dynamical friction from the
residual planetesimal disk, tidal and gas damping), and can the final eccentricity and inclination distributions be
derived from the angular-momentum-deficit budget and the damping, or are they part of the chaotic outcome? The
angular-momentum deficit (AMD) is the natural conserved-ish quantity to track (Laskar 1997, 2000), and it couples to
the long-term stability the R-CELESTIAL-SECULAR arc (#44) will need.

### F. The giant and ice-giant branch (couples to #73)

Beyond the ice line the isolation masses are larger. If a core reaches the critical mass (Mizuno 1980, order 10 Earth
masses, but it depends on the accretion rate, the opacity, and the disk temperature) before the gas disperses, it
triggers runaway gas accretion and becomes a gas giant; otherwise it stays an ice giant or super-Earth. The questions,
which are #73's core but couple to the assembly because the assembly sets which outer cores reach critical mass:

1. Is the critical core mass derivable from the core's ability to radiate the envelope's Kelvin-Helmholtz contraction
   energy (Rafikov 2006), rather than cited as about 10 Earth masses?
2. The gas-disk dissipation timescale (order 1 to 10 Myr, photoevaporation plus viscous draining) sets the race
   between core growth and gas loss. It is a per-system contingent (disk mass, stellar UV, viscosity). Derivable or IC?
3. The runaway accretion rate and the final gas-giant mass (gap-opening plus the disk gas supply).

### G. The disk initial conditions (which inputs are legitimate)

The generator takes the disk profile (characteristic radius r_c, slope gamma, scale Sigma_c), the composition, and the
accretion parameters as per-system inputs. Which are legitimate contingent initial conditions (like star mass), and
which should derive from something deeper?

1. Sigma_c (the disk mass) is a fraction of the star mass (order 0.01 to 0.1) with large scatter: probably a contingent
   IC with a Mirror-pinned default. Confirm.
2. gamma (the slope, about 1) follows from the viscous self-similar spreading (Lynden-Bell and Pringle 1974): derivable
   from the viscosity prescription (the alpha-disk), or a per-system input given alpha's own uncertainty?
3. r_c (the characteristic radius) is set by the initial cloud's angular momentum: a per-system contingent.

## 4. The Buckingham-Pi authored-input budget (the ceiling)

Before building, count the dimensionless groups that govern the whole assembly. That count is the MAXIMUM number of
authored inputs the model may legally demand; if it needs more free parameters than groups, it is over-authored.
Candidate groups to enumerate and reduce: the disk-to-star mass ratio, the surface-density slope, the ice-line-to-disk
-size ratio, the Safronov number, the gas-dissipation-to-growth timescale ratio, the planetesimal-to-embryo mass
ratio, the isolation-mass-to-critical-core-mass ratio, and the spacing-in-Hill-radii. The research should produce this
list and, for each, say whether it is a fundamental constant, a per-system IC, or derivable, so the authored budget is
explicit and bounded.

## 5. The observational constraints (what a derived population must reproduce)

The derived systems should reproduce the STATISTICAL regularities, not the exact solar system (which is one draw):

1. The Kepler intra-system uniformity, "peas in a pod" (Weiss et al. 2018; Millholland, Wang and Laughlin 2017):
   adjacent planets similar in size and regularly spaced. Does this emerge from the oligarchic-plus-assembly dynamics?
2. The mass, radius, and spacing distributions of the exoplanet population.
3. The solar system's own features as a sanity draw: the terrestrial spacing, the asteroid belt as a planet that failed
   to assemble (truncated by Jupiter), the small-Mars problem, and the giant-planet architecture.
4. The angular-momentum-deficit distribution (the dynamical excitation).

The research should say which of these are emergence tests the model must pass and which are contingent to specific
initial conditions (the Grand Tack, for instance, is Jupiter-specific and not a universal requirement).

## 6. The literature to survey (the grounding)

Oligarchic growth and the spacing: Kokubo and Ida 1998, 2000, 2002; Kokubo, Kominami and Ida 2006. Terrestrial
assembly by N-body: Chambers and Wetherill 1998; Chambers 2001; Raymond, Quinn and Lunine 2004, 2006, 2009; Hansen
2009. Stability criteria: Gladman 1993 (the 2*sqrt(3) Hill criterion); Chambers, Wetherill and Boley 1996; Smith and
Lissauer 2009; Pu and Wu 2015; Obertas, Van Laerhoven and Tamayo 2017. Collision outcomes: Leinhardt and Stewart 2012;
Asphaug 2010. Population synthesis (the semi-analytic template for a deterministic proxy): Ida and Lin 2004 onward;
Mordasini, Alibert, Benz and the Bern model; Emsenhuber et al. 2021. Giant formation and the critical core mass:
Mizuno 1980; Pollack et al. 1996; Rafikov 2006. Disk structure: Lynden-Bell and Pringle 1974; Hartmann et al. 1998.
Secular dynamics and the AMD: Laskar 1997, 2000. Observational regularities: Weiss et al. 2018; Millholland, Wang and
Laughlin 2017; Goldberg and Batygin 2022.

## 7. The grounding in our own code (what the resolution must consume, not rebuild)

Built and to be consumed: `astro::disk_surface_density`, `astro::isolation_mass_earth`, `astro::hill_radius_au`,
`astro::disk_effective_temperature`, `astro::kepler_orbital_period_years`, `astro::planet_radius_m`, the ice line and
the condensation snow-line temperature, `DiskComposition`, and `planet::derive_planet` (per-orbit world). The crater-
scaling pi-groups (#70) and impact-flux (#71) are candidate machinery for the collision-outcome model. Mass and
angular-momentum conservation of a merge should use the apportionment and conserved-quantity ledger, not ad hoc
arithmetic. The numerical substrate (Tier-2 wide-intermediate, log-space for the wide orbital magnitudes) is what
keeps the orbital quantities representable past the Kuiper and Oort scales.

## 8. The couplings

R-ASSEMBLY sits upstream of #73 (giants, via which outer cores reach critical mass), #74 (small bodies, via the
un-merged planetesimals and collision fragments), and #75 (moons, via the same accretion-and-capture dynamics). It
couples to #44 (R-CELESTIAL-SECULAR, the long-term secular stability and the Milankovitch cycles) through the final
eccentricities and the AMD, and to the temporal level-of-detail arc through the 10-to-100 Myr assembly timescale.

## 9. What I need resolved, in priority order

1. The determinism-versus-emergence ruling in C: is a seeded sampling of a physically-derived outcome distribution an
   acceptable emergence, or is a seeded N-body (or the rule-based merge) required? This is the load-bearing decision;
   everything else is buildable once it is settled.
2. The final-spacing law in B: derived, or a single cited dimensionless constant with a stated physical basis, and
   which one (the 2*sqrt(3) Hill floor, the empirical Gyr-stability spacing, or an over-relaxed value).
3. Whether `b` in A is a universal constant or per-system data, so I know if it is a Mirror-defaulted IC or a fixed
   number.
4. The merge bookkeeping in D: perfectly inelastic first pass, or the Leinhardt-Stewart collision-outcome spectrum.
5. The authored-input budget in 4: the enumerated dimensionless groups, so the authored count is bounded before I
   build.

Items E, F, and G can follow, or fold into #73 and #44. The first three are what unblock the assembly slice.
