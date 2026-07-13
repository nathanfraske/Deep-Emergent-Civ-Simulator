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

The free-electron entry is not speculative; it reproduces measured electronic scales from valence and the built
density alone. The plasma energy `hbar * omega_p` computed from `n_e = z * rho * N_A / M` lands the measured values
across simple metals: sodium `5.9 eV` against a measured `5.7`, aluminium `15.8` against `15.8`, copper `10.8`
against `10.8`, each within roughly one percent, with the conduction-electron count `z` taken from the group
(one for the alkali and for copper, three for aluminium). The Drude conductivity closes the same way: copper's
measured resistivity backs out a scattering time `tau ~ 2.5e-10 ps` (in the picosecond working unit the freezer's
attempt frequency already uses), the phonon-limited relaxation time the model expects. So the near-ready entry is a
clean first slice: `n_e` and `omega_p` reserve no value, and the Drude conductivity reserves only the scattering
physics.

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
