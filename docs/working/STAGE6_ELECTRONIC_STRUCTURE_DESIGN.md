# Stage 6, the electronic-structure sub-arc: the design-first surface (for the gate's ruling)

This is the design opener for the ELECTRONIC-STRUCTURE sub-arc of Stage 6, surfaced design-first for the gate's
ruling before a line is built, on the same discipline the freezer output side and the mechanical/thermal core
opener followed. It authors no value and builds no mechanism. Its purpose is to scope the electronic-property
substrate and its contested calls, so the gate rules the depth and the reserved-coefficient set before building.

The mechanical/thermal core of Stage 6 is complete and validated end-to-end (density, the shear-aware Debye
temperature, the elastic moduli, hardness, the Debye heat capacity, strength, thermal expansion, the Slack lattice
conductivity, thermal diffusivity, creep, and the surface and grain-boundary energies). Everything left in Stage 6
is the ELECTRONIC half: the properties that cannot derive from the mechanical/thermal floor because they need the
electron structure. That is the heaviest single floor piece of the stage, so it earns the full design-first
treatment.

## 1. What the electronic sub-arc delivers

Three property families, each bottoming out on the electron structure rather than the lattice:

- ELECTRICAL CONDUCTIVITY, the Drude model `sigma = n_e * e^2 * tau / m_e`, needing the free-electron (carrier)
  density `n_e` and a scattering time `tau`. This is the electronic conductivity a metal's heat and charge ride on,
  the piece the Slack lattice conductivity explicitly deferred (the Slack model gives the phonon part; a metal's
  total is electronic-dominated by Wiedemann-Franz).
- MAGNETISM, needing the density of states at the Fermi level `g(E_F)` (the Stoner criterion `g(E_F) * I > 1` for
  itinerant ferromagnetism) or the local moment (Hund's-rule filling of the d or f shell), a density-of-states or
  occupancy datum.
- OPTICAL COLOUR, needing the absorption or reflection spectrum: the interband gap for a semiconductor's colour,
  the plasma frequency `omega_p` for a metal's reflectivity and its warm or white cast, the d-d transitions for a
  transition-metal compound's colour.

## 2. The substrate, from the near-ready entry to the deep piece

The electron-structure floor splits into a near-ready entry buildable on the current substrate plus the periodic
table, and a deep piece that is a new floor axis.

- THE NEAR-READY ENTRY: the free-electron density and its immediate consequences. For a metal, the conduction-
  electron density is `n_e = z * rho * N_A / M`, the conduction electrons per atom `z` (from the periodic-table
  group or valence, a data row) times the number density of atoms (the built density over the molar mass). This is
  a pure derivation over the built density and periodic-table data, no reserved value. From it the plasma frequency
  `omega_p = sqrt(n_e * e^2 / (epsilon_0 * m_e))` derives directly, and the Drude conductivity follows once a
  scattering time is supplied.
- THE DEEP PIECE: the density of states and the band structure. The band gap `E_gap` (whose sign and size sort a
  substance into metal, semiconductor, or insulator), the density of states at the Fermi level `g(E_F)` (for
  magnetism), and the effective carrier mass and interband structure (for optics) are the deep floor axis. A full
  first-principles band structure is beyond a reduced-order floor; the honest floor is a reduced-order model, either
  a measured band-structure `[M]` column (gap, effective mass, `g(E_F)`) per substance, or a tight-binding or
  free-electron-plus-gap model over the substance's own valence and orbital occupancy.

## 3. The near-ready entry, grounded

The free-electron entry reproduces measured electronic scales from valence and the built density alone, at the
FEW-PERCENT grade honest for a simple-metal (sp-band) free-electron model, NOT the "~1 percent" the first draft
overclaimed. The plasma energy `hbar * omega_p` computed from `n_e = z * rho * N_A / M` lands the sp-metal trio:
sodium `5.92 eV` against a measured `~5.7` (`~4 percent`), magnesium `10.9` against `~10.6`, aluminium `15.8`
against `~15.3`, with the conduction-electron count `z` from the group (one for the alkali, two for magnesium,
three for aluminium). The earlier copper row was dropped: copper's plasma response is wrecked by d-band interband
screening and yields no clean `10.8 eV` observation, so a "10.8 against 10.8" comparison was calculated-versus-
calculated (circular) and is removed.

THE NAMED d-BLOCK FAILURE EXHIBIT (it motivates the deep piece better than prose). Silver's free-electron
prediction is `9.0 eV` (`n_e = 5.86e28 /m^3` gives `hbar * omega_p = 8.99 eV`), against the OBSERVED screened
plasmon `~3.8 eV`: a factor `2.4` miss, the d-electron interband screening the free-electron model cannot see
(Ehrenreich and Philipp, Phys. Rev. 128, 1622 (1962)). This one row is why the deep piece (the band structure) is
required for the d-block, and why the near-ready entry is scoped to the sp-metals with the d-block flagged.

The Drude conductivity closes with the CORRECTED scattering time (the first draft's `~2.5e-10 ps` was an
eight-decade units error). Copper's measured resistivity `~1.7e-8 ohm*m` backs out `tau = 2.5e-14 s = 0.025 ps`
(NOT `2.5e-10 ps`), the phonon-limited relaxation time the model expects. The Drude slice therefore carries a
mandatory `sigma` ROUND-TRIP TEST: store `tau`, recompute `sigma = n_e * e^2 * tau / m_e`, and assert it rebuilds
the cited resistivity, so a units fold in `tau` fails loudly. So the near-ready entry is a clean first slice
(`n_e` and `omega_p`, no reserved value), and the Drude conductivity that follows reserves only the transport
electron-phonon coupling (section 5, Call 3).

## 4. The metal / semiconductor / insulator distinction must EMERGE (the Principle-8 line)

The three classes are not an authored enum. They emerge from the band gap keyed per substance: a zero (or negative)
gap is a metal, a small gap is a semiconductor (its carrier density thermally activated, `n_e ~ exp(-E_gap / 2kT)`),
a large gap is an insulator. The classification is a derived readout of the substance's own gap datum, exactly the
freezer's emergent-regime shape, never a Terran material lookup that says "iron is a metal." A world's alien
conductor sorts itself from its own gap. This is the admit-the-alien line applied to the electron structure, and it
is the reason the gap must be a per-substance datum rather than a hardcoded class.

## 5. The contested design calls, surfaced for the gate

- CALL 1, the depth of the deep piece. A reduced-order model (a measured `[M]` band-gap and effective-mass column
  per substance, plus the free-electron density) versus a first-principles tight-binding band structure over the
  periodic-table orbitals. The reduced-order path is the honest floor, and the band gap and effective mass are
  measured `[M]` data of the same status as `B_0` and `dH_f` (refutable, per-substance, source-cited), with the
  full band structure a named follow-on. The gate's call on whether the sub-arc targets the reduced-order `[M]`
  column or attempts the tight-binding model.
- CALL 2, the conduction-electron count `z`. The near-ready proxy is the periodic-table group or valence (a data
  row), which lands the simple metals (section 3). But the EFFECTIVE carrier count departs from the nominal valence
  for the d-band transition metals and for semiconductors, where the band structure sets it. The gate's call on
  whether `z` keys on the nominal valence (the near-ready proxy, honest for simple metals, flagged for d-band cases)
  or waits for the band-structure datum.
- CALL 3, the scattering time `tau` and its reserved coefficient. The phonon-limited `tau` reads the built Debye
  temperature (`1/tau ~ T` above `Theta_D`, the Bloch-Grueneisen `T^5` below), so its temperature dependence is
  derived; the reserved residual is the electron-phonon coupling strength per material (or, equivalently, the
  characteristic resistivity), plus a residual-resistivity term from defect scattering that ties to the defect-
  population floor piece the mechanical core flagged. The gate's call on the coupling as the one reserved
  coefficient of the Drude slice, hunted before reserving like every other.
- CALL 4, magnetism's model. The Stoner criterion over `g(E_F)` for itinerant ferromagnetism versus Hund's-rule
  local moments over the d/f occupancy. Both are reduced-order; the gate's call on which the sub-arc targets first,
  and whether `g(E_F)` is a measured `[M]` datum or derived from the reduced-order DOS.

## 6. Admit the alien, keyed on per-substance electron data

Every piece keys on the substance's own electron data: the conduction-electron count from its valence, the gap from
its measured `[M]` gap, the DOS from its occupancy. So an alien conductor is a data row, never a rewrite: a being or
material that carries charge on an exotic band structure, a redox-gradient carrier, or a mana-field mobile charge
enters as its own `n_e`, gap, and coupling, and the Drude and gap-classification mechanisms read them unchanged. No
pathway assumes the Terran d-block or a single carrier chemistry.

## 7. Byte-neutrality and the build order

Byte-neutrality: the electronic properties land in the materials leaf (not linked into the run_world binary), so
they move no run pin, proven per push as the mechanical/thermal core was. A new measured `[M]` electronic column
(the band gap, the effective mass, the conduction-electron count) rides in a new electronic-anchors data file or in
`metal_eos` where `dH_f` did, verified pin-neutral by the call graph and the gate's pin re-run.

Build order, gated per push, each contested piece ruled before building: (a) the free-electron density `n_e` and
the plasma frequency `omega_p` (near-ready, from valence and the built density, no reserved value); (b) the Drude
conductivity with the phonon-limited `tau` (one reserved electron-phonon coupling, reading `Theta_D`), the
electronic conductivity the Slack lattice model deferred; (c) the band-gap `[M]` column and the emergent
metal/semiconductor/insulator classification; (d) the density of states and magnetism (Stoner or Hund); (e) optical
colour (the interband gap, the plasma frequency, the d-d transitions). The near-ready entry (a) is buildable the
moment the gate rules the sub-arc; the deep pieces (c-e) wait on the `[M]`-versus-tight-binding depth ruling.

## 8. Honest limits

A full band structure is beyond a reduced-order floor, so the electronic properties ship at the free-electron and
density-of-states reduced order, with the deeper band-structure model a named follow-on. The Drude free-electron
model is accurate for simple metals (the plasma frequencies of section 3) and degrades for the d-band transition
metals, where the effective mass and multiple bands need the band structure. Magnetism via Stoner or Hund is
reduced-order, without the itinerant-band detail a full treatment carries. Optical colour at the plasma-frequency
and interband-gap order captures the metal cast and the semiconductor edge, not the fine absorption spectrum. Each
limit is stated at its mechanism when built, on the arc's discipline of naming the reach ceiling rather than hiding
it.

## 9. Amendment (owner research audit, 36th): the sharpened calls and the coherence redirects

The research tier ran this opener and ratified its shape while sharpening the four calls and catching three
coherence redirects where the sub-arc was about to rebuild or bypass banked machinery. This section supersedes
section 5 where they differ, and the two numeric defects it caught are already fixed in section 3 (the copper
`tau` eight-decade units error, corrected to `2.5e-14 s = 0.025 ps` with a mandatory `sigma` round-trip test; the
inconsistent validation trio, replaced with the Na/Mg/Al few-percent set plus the silver d-screening exhibit).

- CALL 1 (depth): reduced-order `[M]` now, with a MISSING MIDDLE RUNG that is already half-banked. The ladder is
  `[M]` measured rows at top, then a HARRISON universal tight-binding estimator (the same `V ~ hbar^2 * r_d^3 /
  (m * d^5)` matrix elements the banked Friedel-Harrison cohesion estimator already uses give band widths and gaps
  at factor grade from table columns, the admit-the-alien rung), then compute-once at the bottom. The compute-once
  gap rung is bound by the banked eigenvalue-routing law: HYBRID or GW class, NEVER plain PBE (the derivative-
  discontinuity entry), cited explicitly so no one wires PBE gaps in good faith.
- CALL 2 (`z`): nominal valence proxy now, and the `[M]` TOP RUNG is the HALL COEFFICIENT `R_H = 1/(n_eff * e)`,
  the measured effective carrier density, refutable-without-sim, carrying SIGN. Its sign failures (Be, Zn, Cd
  positive) are exactly the band-structure boundary the valence proxy cannot cross, so the d-band flag AUTO-RAISES
  from the periodic table's occupancy columns and routes to the Hall row where one exists.
- CALL 3 (`tau`): the reserved coefficient is the DIMENSIONLESS transport electron-phonon coupling `lambda_tr`.
  Above `Theta_D` the phonon-limited rate has the clean form `hbar/tau = 2*pi*lambda_tr*k_B*T`, so `lambda_tr` is
  the ONLY reserved number, `[M]` per material from the McMillan (1968) / Allen (1971) lineage, and it is the SAME
  `lambda` Eliashberg consumes for conventional superconducting `T_c` (a dual-consumer column). Consistency: `lambda
  ~ 0.13` for copper predicts `tau(300 K) = 3.1e-14 s` against the backed-out `2.5e-14`, within 25 percent, so
  `lambda_tr,Cu ~ 0.16` closes it. Bloch-Grueneisen `T^5` below `Theta_D` is derived-in-form; Matthiessen
  additivity ties the defect residual-resistivity term to the damage floor (with RRR a free in-sim purity meter).
  Honest-limit ceiling: the Mott-Ioffe-Regel bound (the mean free path cannot fall below a lattice spacing;
  Gunnarsson, Calandra, Han 2003) marks where Drude itself dies.
- CALL 4 (magnetism): dispatch on the banked `U/W` classifier, Hund's-rule local moments first (already built),
  Stoner itinerant as the follow-on. `g(E_F)` has a measured route: the Sommerfeld coefficient `g(E_F) = 3 *
  gamma_el / (pi^2 * k_B^2)` from the low-temperature electronic heat capacity, refutable-without-sim, so `g(E_F)`
  starts `[M]` with the reduced-order DOS as estimator; the Stoner `I` is `[M]` (Janak 1977). Validation: only
  Fe, Co, Ni clear `g*I > 1` among the elements, and Pd sits just under with a `~10x` Stoner-enhanced
  susceptibility, a `delta -> 0` Gap Law near-miss the classifier FLAGS rather than scoring as a failure.

THREE COHERENCE REDIRECTS (do not rebuild or bypass banked machinery):

1. OPTICAL COLOUR is partially built: the d-d transitions ARE the banked `10Dq` crystal-field machinery (ruby
   versus emerald), so the sub-arc CONSUMES that column and builds only the interband and plasma pieces.
2. The gap-keyed emergence MISROUTES DERIVED gaps: it is safe for measured-`[M]`-gap rows, but a reduced-order band
   model returns NiO metallic when it is a Mott insulator, so the banked `U/W` preflight MUST run BEFORE gap
   classification on any non-`[M]` route, or this sub-arc reintroduces the exact failure the Mott turn closed.
   (This corrects section 4's Principle-8 claim, which was clean only for the measured-gap case.)
3. WIEDEMANN-FRANZ: the phonon-dominated-insulator versus electron-dominated-metal crossover is an EMERGENT readout
   of the computed lattice-versus-electronic `kappa`, never a class label.

VALIDATION BATTERY additions: the Na/Mg/Al plasma trio at few-percent grade, the silver `9.0`-versus-`3.8 eV`
d-screening failure exhibit, Pd's Stoner near-miss, the `tau` `sigma` round-trip test. LAYER-3 additions:
`lambda_tr` `[M, dual-consumer]`, the Hall and Sommerfeld `[M]` routes, the Harrison band rung, and the
Mott-Ioffe-Regel ceiling. Citations verified at source: Ehrenreich and Philipp 1962, Janak 1977, McMillan 1968,
Allen 1971, Gunnarsson-Calandra-Han 2003, Ashcroft and Mermin tables 1.2 and 14.1.

Build order unchanged: the near-ready entry (`n_e`, `omega_p`) first, then the Drude conductivity on the corrected
`tau` with `lambda_tr` and its round-trip test, then the `[M]`-plus-Harrison-plus-compute-once gap tier with the
`U/W` preflight, then DOS and Hund magnetism, then the interband and plasma optics over the banked `10Dq`.

## 10. The band-gap tier: the integration surface and the Harrison-rung premise (design-first, for the gate's ruling)

Building the gap tier ruled in section 9 (Call 1: an `[M]` top rung, a Harrison estimator middle rung, a
compute-once bottom rung, with the `U/W` preflight of redirect 2) began with grounding the banked machinery the
tier is to CONSUME rather than rebuild: the correlation classifier (`correlation.rs`), the metallic route
(`metallic.rs`), the localized route (`localized.rs`), the d-state radius floor (`d_state_radius.rs`), the MIT
reference set, and the periodic table. That grounding surfaced one premise in Call 1 that does not hold against
the code, and it changes what the middle rung can be built from. It is surfaced here design-first, on the arc's
discipline of proving the input before trusting it, most of all when the input is a prior ruling.

### 10.1 The Harrison-rung premise, checked against the code

Call 1 reads the Harrison estimator as half-banked: "the same `V ~ hbar^2 * r_d^3 / (m * d^5)` matrix elements the
banked Friedel-Harrison cohesion estimator already uses give band widths and gaps at factor grade from table
columns." Four facts from the code contradict the reusability that premise assumes.

First, there is no banked Friedel-Harrison cohesion estimator. The metallic route (`metallic.rs`) computes the
elemental cohesive energy from the ROSE universal binding-energy relation (Rose, Smith, Guinea, Ferrante 1984): the
banked measured `E_coh` and the dimensionless ratio `B_0 * V_m / E_coh`. It reads no Harrison matrix element. The
cohesion machinery the tier would reuse is a Rose equation of state, not a Harrison tight-binding sum.

Second, the one place the Harrison band-width form does appear, the correlation classifier, uses it as a RATIO with
the absolute prefactor deliberately unfetched. The classifier's own note is explicit: `U/W = (screening /
C_Harrison) * U_atomic * d^5 / r_d^3`, where the in-crystal screening and Harrison's prefactor are DEGENERATE in
the MIT fit (only their ratio is determined), Harrison's prefactor is in his book, unfetchable, and is not
fabricated. The classifier keys on the raw ratio `rho = U_atomic * d^5 / r_d^3` and never needs an absolute
bandwidth in eV. So the absolute Harrison prefactor `C_Harrison`, the dimensionless universal coefficient a band
width in eV requires, is not banked: it was surfaced as unfetchable and left unfabricated, the correct refusal.

Third, the d-state radius that feeds that form is trustworthy only in RELATIVE scale. The d-state radius floor
states that the absolute `r_d` scale is absorbed by the MIT-calibrated screening (a uniform scaling of `r_d` by `k`
scales `W` by `k^3`, which the screening re-fits to the same `U/W = 1` boundary), so only the relative contraction
across the series is load-bearing. An absolute band width built from `r_d` would inherit an absolute scale the
floor never had to validate and never did.

Fourth, on the target rather than the inputs: the semiconductor gap the tier most needs (the sp-bonded solids,
silicon, the III-Vs, the covalent and ionic non-metals whose colour and carrier density the sub-arc delivers) is
not the `d^{-5}` d-band matrix element at all. Harrison's semiconductor gap is the bond-orbital construct `E_gap ~
2 sqrt(V_2^2 + V_3^2)`, with the covalent energy `V_2 ~ eta * hbar^2 / (m d^2)` (the `d^{-2}` sp matrix element, a
different power law from the `d^{-5}` d-d form) and the polar energy `V_3` the difference of the two atoms' hybrid
term values. Building it needs Harrison's dimensionless universal coefficients `eta_{l l' m}` (pure numbers, his
solid-state table) and a per-element atomic term-value column (`epsilon_s`, `epsilon_p`), neither of which is
banked. The periodic table carries the first ionization energy and the electron affinity in eV, not the orbital
term values the bond-orbital model reads.

The conclusion is not that the Harrison rung is impossible. It is that the rung cannot be built by reusing
already-banked matrix elements as Call 1 assumes: it needs cited pure-number inputs (Harrison's `eta` coefficients
and a term-value column) that the floor does not yet carry, and the one number a naive build would reach for (the
absolute Harrison prefactor) is the number the correlation work already ruled unfetchable and refused to fabricate.
Fabricating it here to make the rung compile would break the value-authoring line the whole arc runs on.

### 10.2 The tier that IS buildable now, fabrication-free

Separating the rungs by what each needs shows most of the tier is buildable now with no fabricated value, and only
the Harrison middle rung waits on the gate.

The `[M]` top rung is a measured band-gap column, a new cited data file of the same status as the bulk modulus and
the formation enthalpy: per substance, the measured gap in eV, source-cited, refutable without the sim. This is
world data read, never a reserved value, and it is the rung the emergent classification of section 4 rides safely
(redirect 2 confirmed the emergence is clean for the measured-gap case).

The emergent metal / non-metal classification is a pure readout of the gap sign, buildable now: a gap at or below
zero is a metal, a gap above zero is a non-metal. This carries no threshold to fabricate.

The semiconductor-versus-insulator split, by contrast, should NOT be a hardcoded eV boundary. There is no sharp
physical line between a semiconductor and an insulator; the distinction is whether the thermally-activated carrier
density at the world temperature is appreciable, a continuous derived readout of the gap and the temperature rather
than an authored cutoff. So the tier reports the metal / non-metal boundary (physical, the gap sign) and, for a
non-metal, the DERIVED carrier activation rather than a discrete semiconductor-or-insulator label with an invented
boundary. A discrete label, if a consumer needs one, emerges when the activation crosses a stated fraction of a
reference density (a reserved fraction with a clear basis, the thermal-activation threshold), never a planted eV
number. This is a derive-first catch on the classification itself.

The thermally-activated carrier density is the section-4 `n_e ~ exp(-E_gap / 2kT)`, and it is the census-flagged
exp-family piece: for a five-eV insulator at world temperature the factor is `exp(-96)`, far below the fixed-point
floor, so it is carried in LOG SPACE (the exponent `-E_gap / 2kT`, always representable) on the creep discipline,
exponentiated only when a consumer needs an in-range ratio. The Boltzmann constant in the working units (eV per
kelvin) reassembles as `k_B[J/K] / e`, a ratio of two fundamental constants (the dimensionless-constant law's
fundamental-constant fold), so the eV and the kelvin cancel and the exponent is dimensionless by construction. This
rung reserves nothing and is buildable now.

The compute-once bottom rung is a provenance LAW, not a computation the engine runs: a compute-once gap must carry
a hybrid-functional or GW provenance tag, never a plain PBE or LDA gap, because the semilocal functionals
underestimate the gap by the derivative discontinuity (often by half or more). This is an eigenvalue-routing rule
stated so no one wires a PBE gap in good faith, buildable now as a tag discipline on the `[M]` and compute-once
column.

The `U/W` preflight (redirect 2) is the tier's load-bearing integration, and it composes the banked classifier
exactly: on any NON-measured route (the estimator or compute-once rungs, where a reduced-order band model could
call NiO a metal), run `CorrelationClassifier::classify` FIRST. A Localized result is an insulator regardless of
what a naive band model returns (route away from the metal / non-metal gap sort, the Mott insulator the correlation
turn closed). An Itinerant result proceeds to the gap-based sort. A Window or OutOfScope result escalates
(estimators forbidden). On the measured-`[M]` route the preflight is unnecessary (the measurement already encodes
the Mott gap), but running it there too is coherent and cheap, and it keeps one code path. This is the ordering
that stops the sub-arc from reintroducing the exact failure the Mott turn closed.

### 10.3 The gate's call, and the recommended build order

The one piece that needs the gate is the Harrison middle rung, and the call is a fork. Either the gate delivers the
verified inputs from the literature (Harrison's dimensionless `eta_{l l' m}` universal coefficients as pure numbers
and a cited atomic term-value column), as the gate delivered the Slack Leibfried-Schlomann structure when the
first-principles reassembly did not close, and the sp-bonded bond-orbital gap estimator is built over them; or the
Harrison rung is deferred as a named follow-on and the tier ships with the `[M]` top rung, the emergent
classification, the log-space carrier activation, the `U/W` preflight, and the compute-once law, escalating between
the measured rows and the compute-once floor where an estimator would otherwise sit. Both are honest; neither
fabricates the Harrison prefactor.

The recommended order, gated per push and independent of the Harrison call: build the fabrication-free core first
(the log-space carrier activation as the census-discharge piece, then the emergent metal / non-metal classification
with the `U/W` preflight over the banked classifier, then the measured `[M]` gap column and its cited data file
with the compute-once provenance law), and take the Harrison rung as the next slice once the gate rules the fork.
This keeps the tier moving on the pieces that need no ruling while holding the one piece that needs the gate, on the
arc's rhythm of building the certain part and surfacing the contested part rather than fabricating past it.

Byte-neutrality holds throughout: the gap tier lands in the materials leaf and the new `[M]` gap column in a data
file, moving no run pin, proven per push as the mechanical and near-ready electronic slices were.
