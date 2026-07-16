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

---

## RESOLUTION (owner and researcher ruling, 2026-07-15; grounded against current code)

The ruling arrived complete and resolves the packet in its priority order. The core is that this whole stage is the CHAOS PROTOCOL, a partition of the Gap Law: the giant-impact outcome is Lyapunov-sensitive to digits below the input bands, so the gap between candidate architectures sits below the input resolution. That is a sub-resolution verdict at the system level, and the typestate has no winner field to read, so the only legal moves are Escalate or SeededDraw. Escalation cannot outrun exponential divergence, so SeededDraw from the stationary measure is forced. Authorship lives in the measure's provenance, not the act of drawing: a seeded draw from a DERIVED measure is emergence under Principle 8, a draw from an authored measure is laundered authorship.

- **C (the how): option 2 (seeded draw) is constitutionally forced, not merely acceptable.** Option 1 (fixed-point N-body) is forbidden as a byte-neutrality landmine and is not derivation anyway (below the Lyapunov horizon the trajectory is a hash of sub-band digits the seed stream already provides). Ships under five gates: (1) DERIVATION, the outcome distributions are pushforwards of derived disk and embryo-field quantities, tagged compute-once (amortized N-body ensembles) or [M class], bands propagated; (2) CONSERVATION PROJECTION, the draw is projected onto the budget manifold (field mass = planets + debris + ejecta, each a named flux; AMD collisionally converted to the heat ledger or exported by ejection), the Residual Law demanding the debris and ejection edges; (3) STABILITY POSTCONDITION, the realized architecture satisfies t_inst > remaining age on B's surface; (4) VALIDITY DOMAIN (Principle 7), the source ensembles declare their calibration domain and the map conditions on dimensionless groups so an alien disk moves through the same map (outside the domain, widen bands, escalate, or refuse); (5) SEED DISCIPLINE, one named [X] slot content-hashed on world identity plus the embryo-field hash. Option 3 (the rule-based merge) is demoted from dynamics to the stability PROJECTOR (gate 3). Option 1's surviving role is the offline calibrator and validator of the map.
- **B (the spacing law): one derived surface, not three constants.** The object is the instability-time function t_inst(Delta, AMD, age; mu), increasing about exponentially with spacing in mutual Hill radii (about one decade per unit Hill spacing for five-Earth systems), with a 0.43 dex chaotic-diffusion scatter carried as part of the measure. Analytic basis: three-body resonance overlap (Petit et al.), the critical separation scaling as mass ratio to the 1/4 power. The three former candidates are evaluations of this one surface: Gladman's 2*sqrt(3) is the two-planet zero-AMD slice ([D] anchor), the Gyr edge is the surface at t = age (near 10 mutual Hill radii circular, near 12 at e about 0.02), and the settled value is the OVERSHOOT above the edge (a prediction: the Kepler peak near 20, near 12 after selection). The final-spacing law is the LEVEL SET: relax until minimum pairwise t_inst exceeds remaining age; the merge jumps generate the overshoot for free. Unify with E: build ONE boundary in (Delta, AMD, age) whose limits recover Gladman and the Gyr edge.
- **A (the oligarchic spacing b): a universal class constant with a band, [M class], not a per-system IC and not a knob.** Ship b = 10 r_H with the 5 to 10 band; the derived repulsion-versus-friction form (calibrated to the Kokubo-Ida ensembles) is optional and gives the weak per-system modulation, the alien-admitting behavior of a form over a bare number.
- **D (the merge books): perfect merging is honest for N, masses, and spacing** (hit-and-run is about half of impacts but leaves the architecture statistics largely unchanged). Pass 1 ships perfect-merge with the debris residual POSTED LOUD (the Residual Law needs the debris and ejecta edges); the Leinhardt-Stewart and Genda outcome map retires the flag later as compute-once decoration on the drawn impact list, reusing the crater pi-groups (#70). Impact-list distributions exist: about 24 giant impacts per system, final multiplicity 3.6 plus or minus 0.8, last giant impact 73 plus or minus 74 Myr, spin about 30 percent below perfect accretion and isotropic in obliquity (which drops into the existing seeded-draw spin pattern).
- **E (eccentricity and AMD): one stability object (above).** The genesis AMD filter demotes to the assembly draw's acceptance projection. Budget double-entry: scattering generates AMD, collisions convert it to heat, ejections export AMD, energy, and mass, the collide-versus-eject split governed by the Safronov group Theta = (v_esc/v_orb)^2 derived per annulus. Post-assembly AMD is handed to #44 as its initial condition.
- **F (giants): keep the cited about-10-Earth-mass anchor, implemented as the derived-form race** tau_KH(M_core, kappa_env) against the viscous disk clock (from Mdot and alpha_disk), so M_crit is a function of envelope opacity and therefore of the abundance draws (alien-admitting). Assembly hands #73 ranked core-mass-versus-time tracks per annulus; near-threshold cores are sub-resolution verdicts, seeded draw.
- **G (disk inputs): ZERO new per-system initial conditions.** Sigma_c, gamma, and r_c are VIEWS of the existing disk realization (Mdot plus alpha_disk plus age plus M_star through the viscous similarity family), with solids from the abundance draws through condensation. The oligarchy slice (`planetary_system.rs`) currently holds all three as free inputs (`DiskProfile`); retag them derived and delete the slots.
- **The Pi budget: twelve dimensionless groups**, every one landing on an existing draw, closure, class constant, or compute-once map. The authored ceiling consumed by new scalars is ZERO; the priced cost is one named seed slot.
- **The test battery, split.** Universal population-level (hindcast-tagged): the spacing distribution (edge at the derived surface, peak near 20 as the emergent overshoot), peas-in-a-pod uniformity, multiplicity and mass function versus disk mass, and population AMD-stability, each run THROUGH a forward-modeled detection function (the raw-to-raw comparison authors a selection bias; forward-modeling Kepler shifts the inferred minimum spacing to about 8 mutual Hill radii). IC-specific single-draw sanity only, never population gates: the solar system must lie in the support (it does, the ensemble multiplicity and last-impact window bracketing the Hf/W Moon constraint); Mars' small mass and the Grand-Tack or Nice narratives are reachable histories, forbidden as targets.

### Grounding against current code (2026-07-15)

- BUILT and ready to consume: the disk temperature from Mdot (`astro.rs` viscous dissipation), the condensation and the per-system disk composition (the abundance draws), the crater pi-groups (#70) and impact-flux (#71), the seeded-draw [X] named-slot mechanism (the freezer, `FREEZER_STAGE5_DESIGN.md`), and the conserved-quantity ledger for the conservation projection.
- TO WIRE: the viscous-similarity derivation of Sigma_c from Mdot and alpha_disk (gate G, the ingredients present bar alpha_disk, a standard disk parameter), the t_inst(Delta, AMD, age) surface in the Petit overlap form (B and E, coefficients compute-once to the ensembles), and the seeded-draw architecture map with its five gates (C).
- SPEC-ONLY, so no migration: the #44 occurrence-statistics genesis is not built, so R-ASSEMBLY builds the DERIVED map directly (the ruling's upgrade is a clean first build, not a refactor), and its post-assembly AMD feeds #44's secular spectrum.
- SLICE-1 CORRECTIONS (`crates/sim/src/planetary_system.rs`): retag `DiskProfile`'s Sigma_c, gamma, r_c as derived views of the disk realization (gate G); firm the reserved `b` to the universal [M class] class constant (10 r_H, 5 to 10 band) with the derived repulsion-versus-friction form as the optional alien-admitting upgrade.

### Fetch items flagged by the researcher (not verified this session, needed before writing the forms)

The Goldreich, Lithwick and Sari 2004 analytic oligarchic-spacing expression (for A's derived form); the Ikoma, Nakazawa and Emori 2000 Kelvin-Helmholtz exponents (for F's tau_KH); the Petit et al. resonance-overlap coefficients (for B's t_inst surface).
