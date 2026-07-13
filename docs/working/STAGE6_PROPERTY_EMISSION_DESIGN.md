# Stage 6, property emission: the design-first opener (the #188 -> #189 bridge)

This is the design opener for STAGE 6, property emission, gate-sequenced after the freezer output side (#188)
completed and passed its section-9 audit. It is the BRIDGE PR: branched from the #188 head so #188's watch is
preserved while the gate merges it, exactly as #188 bridged #187. It authors no value and builds no mechanism.
Its purpose is to surface the Stage-6 scope and its contested calls for the gate's review before a line is built.
On the gate merging #188, this branch rebases onto the new `main` and the diff collapses to this one doc.

Property emission is where the materials substrate pays off into the visible world: a realized assemblage (the
disposer plus the freezer, Stages 4 and 5) carries a composition, a structure, a grain texture, and a
distance-from-equilibrium record, and Stage 6 reads those to emit the MEASURABLE PROPERTIES a world's matter has:
its density and stiffness, its hardness and strength, how it conducts heat and expands, how it conducts
electricity and takes colour. Each property is DERIVED from the floor and the realized state, never authored, so
an alien material emits its own properties from its own data.

## 0. A prove-it note on the scope packet (surfaced, not worked around)

The gate's #188 comment referenced a starting packet, `docs/working/STAGE6_PROPERTY_EMISSION_SCOPE.md`, from a
research-tier scoping pass. That file is NOT present in the repository: not in this branch, not on `origin/main`
(still at the #187 merge, `7f58431`), and not on any remote branch (checked exhaustively). So this opener is
grounded in the gate's own comment scope plus a direct reading of the current substrate, NOT in the packet. Two
consequences, flagged for the gate: (a) the packet's "six additions worth folding in" are named only ONE (the
`gamma_sv` reuse, section 5 below); the other five are not recoverable here, so the gate should push the packet
or enumerate them for this opener to absorb; (b) if the packet carries scope this opener misses, the gate's
review folds it in. Nothing below is fabricated from the missing file.

## 1. What Stage 6 delivers

A property-emission layer over the realized assemblage: given a substance's realized state (its composition and
the floor anchors it carries, its structure, its grain size, its temperature), emit its properties as derived
quantities. The scope splits cleanly into two halves by what substrate each half needs.

- The MECHANICAL and THERMAL properties build on the CURRENT substrate (the EOS anchors, the Rose cohesive
  energy, the Lindemann `T_m`, the freezer's `D(T)`, sound speed, and grain size). This half is buildable now,
  each property a derivation over the floor with at most ONE per-class coefficient to run the derivation-hunter
  on, the Form-B shape the freezer and nucleation slices established.
- The ELECTRONIC properties (electrical conductivity, magnetism, optical colour) bottom out on a heavy NEW floor
  piece, an ELECTRONIC-STRUCTURE substrate (a free-electron density for the Drude model; a density of states or
  band structure for magnetism and optical absorption). That substrate is the contested design-first sub-arc,
  surfaced here for the gate's ruling BEFORE any of it is built.

## 2. The mechanical and thermal core (builds on the current substrate)

Each property below is a derivation over quantities already on the floor or built this arc, with its one reserved
per-class coefficient surfaced for the derivation-hunter (never planted). Listed with what it reads and the
coefficient to hunt:

- DENSITY: `rho = M / V_m`, the molar mass (periodic table) over the anchored molar volume. No reserved value; a
  pure ratio of floor data. (The freezer already forms `c_s = sqrt(B_0/rho)` from it.)
- ELASTIC MODULI (K, G, E): the bulk modulus `K = B_0` is ANCHORED directly. The shear modulus `G` and Young's
  modulus `E` are NOT anchored (the freezer flagged the bulk-only elastic limit), so `G`/`E` need a per-class
  Poisson ratio `nu_P` (from which `G = 3K(1-2nu_P)/(2(1+nu_P))` and `E = 2G(1+nu_P)`), OR a shear anchor. THE
  DESIGN CALL: reserve a per-class Poisson ratio `nu_P` (the Form-B coefficient, `~0.3` for many metals but
  class-varying), or add a measured `[M]` shear-modulus anchor column beside `B_0` (the framing-3 pattern, a
  measured datum over a reserved coefficient). Surfaced for the gate.
- HARDNESS: the Chen-Tse (2011) correlation `H = k * G * (G/K)^m` (or the Teter `H ~ G` proportionality), reading
  the derived `K`/`G`. The reserved per-class coefficient is the Chen-Tse `k` (the gate named it), hunted like
  the vacancy fraction.
- STRENGTH (yield / theoretical): the theoretical shear strength `~ G / (2*pi)` (Frenkel), reading `G`; the
  reserved residual is the per-class knock-down from the theoretical to the operative strength (dislocation
  physics), a coefficient to hunt or a flagged follow-on.
- CREEP: the Frost-Ashby deformation-mechanism map over the homologous temperature `T/T_m` (already built) and
  the built self-diffusivity `D(T)` and grain size, giving the strain rate. The reserved per-class coefficient is
  the stress exponent `n` (the gate named it), per creep regime.
- THERMAL CONDUCTIVITY: the Slack model `kappa ~ (M_avg * Theta_D^3 * delta_a) / (gamma_G^2 * n^(2/3) * T)`,
  reading the Debye temperature `Theta_D` (section 3) and the Grueneisen parameter. The reserved residual is the
  Slack prefactor / the Grueneisen `gamma_G` (shared with expansion).
- THERMAL EXPANSION: the Grueneisen relation `alpha = gamma_G * C_v * rho / (3 * K)`, reading `K`, the density,
  and the Debye heat capacity. The reserved per-class coefficient is the Grueneisen parameter `gamma_G` (the gate
  named it), shared with the Slack conductivity, so ONE hunt serves both.

So the mechanical/thermal core reserves a small set of per-class coefficients, each Form-B, each hunted before
reserving: the Poisson ratio `nu_P` (or a shear anchor), the Chen-Tse `k`, the creep stress exponent `n`, and the
Grueneisen `gamma_G`. Each is surfaced with basis, caller-supplied, never planted, primary-verified before entry.

## 3. The Debye temperature `Theta_D`: the deferred freezer sibling, now consumed

The freezer deferred `Theta_D` deliberately (`crates/materials/src/freezer.rs`: "the `theta_D` sibling, built
only when its S_vib / Debye-Cp consumer arrives"). Stage 6's Slack conductivity, Grueneisen expansion, and Debye
heat capacity ARE that consumer, so Stage 6 builds `Theta_D`. It is derivable NOW from the built sound speed:
`Theta_D = C_theta * c_s * n^(1/3)` with `c_s = sqrt(B_0/rho)` (built, `sound_speed_km_per_s`) and `n` the number
density, `C_theta` a pure-math/unit constant (like the Lindemann collapse's `(6*pi^2)^(2/3)` factor, from `PI`
and the built `cbrt`). No reserved value beyond the unit fold; it reuses the freezer's own sound speed. This is
the first mechanical/thermal slice (a prerequisite the thermal properties read), and it retires the freezer's
named `Theta_D` follow-on.

## 4. The electronic-structure substrate (CONTESTED, design-first, the gate's ruling before building)

The electronic properties cannot derive from the mechanical/thermal floor; they need the electron structure:

- ELECTRICAL CONDUCTIVITY: the Drude model `sigma = n_e * e^2 * tau / m_e`, needing the free-electron density
  `n_e` and a scattering time `tau` (phonon-limited, reading `Theta_D` and `T`). The free-electron density is the
  new floor piece: for a metal, `n_e = (valence electrons) * rho / M` (derivable from the periodic-table valence
  and the density), but the effective carrier count and the semiconductor/insulator gap need more.
- MAGNETISM: needs the density of states at the Fermi level (Stoner criterion for ferromagnetism) or the local
  moment (Hund's rules over the d/f occupancy), a density-of-states or occupancy datum.
- OPTICAL COLOUR: needs the band structure / the absorption spectrum (the interband gap for a semiconductor, the
  plasma frequency for a metal, the d-d transitions for a transition-metal compound).

THE DESIGN CALL for the gate: the electronic-structure substrate is a genuine new floor axis (a free-electron
density and a density-of-states or band-structure representation), the heaviest piece of Stage 6. It should be
its own design-first sub-arc, surfaced and ruled BEFORE building, the same discipline the freezer output side
followed. The free-electron density for a simple metal is the near-ready entry (derivable from valence and
density); the DOS/band-structure for magnetism and optics is the deep piece (a tight-binding or a
free-electron-plus-gap model, keyed off per-substance data, never a Terran lookup). I will surface the
electronic-structure sub-arc design separately once the mechanical/thermal core lands and the gate sequences it.

## 5. The `gamma_sv` reuse (the rejected freezer route, not wasted)

The gate's completeness pass flagged this reuse, and it is a clean one. The broken-bond `E_coh` route this arc
REJECTED for the solid-liquid interfacial energy `gamma_sl` (because it yields the solid-VAPOR energy, the wrong
quantity for melt nucleation) is EXACTLY the derivation for the solid-vapour surface energy `gamma_sv` itself.
So `gamma_sv = per-class surface-bond fraction * E_coh / atomic area` is a Stage-6 surface-energy property, the
rejected freezer hypothesis landing where it belongs: `gamma_sv` (a surface/interface property, feeding wetting,
fracture surface energy, and the heterogeneous-nucleation follow-on the freezer flagged) rather than `gamma_sl`.
The reserved residual is the per-class surface-bond fraction, the Form-B shape, hunted before reserving.

## 6. Provenance, byte-neutrality, and the build order

Provenance: every reserved coefficient (`nu_P`, Chen-Tse `k`, creep `n`, Grueneisen `gamma_G`, the `gamma_sv`
surface fraction) is surfaced with basis and caller-supplied or hunted, never planted, on the arc's standing
discipline. The unit/math folds (`Theta_D`'s `C_theta`, any `2*pi`) are derived from `PI` and the built exact
ops. No property value is authored.

Byte-neutrality: property emission lands in the materials crate (a leaf not linked into the run_world binary), so
it moves no run pin, proven per push as the freezer was. A new measured `[M]` anchor (a shear modulus, if the
gate rules that over a Poisson ratio) rides in `metal_eos` where `dH_f` did, verified pin-neutral by the call
graph (and the gate's pin re-run).

Build order, gated per push, each contested piece surfaced before building: (a) `Theta_D` (the deferred sibling,
the thermal prerequisite); (b) the elastic moduli `K`/`G`/`E` (the Poisson-ratio-versus-shear-anchor call ruled
first); (c) hardness and strength over `G`; (d) thermal conductivity and expansion over `Theta_D` and the shared
Grueneisen hunt; (e) creep over the built `D(T)`/grain size and the stress-exponent hunt; (f) `gamma_sv` (the
reused route); then the ELECTRONIC-STRUCTURE sub-arc as its own design-first ruling. The mechanical/thermal core
(a-f) builds on the current substrate; the electronic half waits for its substrate ruling.

## 7. Honest limits

The mechanical/thermal derivations inherit the freezer's crystalline-regime and bulk-elastic limits (a glass or
an amorphous solid needs its own moduli path; the shear modulus is not anchored today). The correlations
(Chen-Tse hardness, Slack conductivity, the theoretical-to-operative strength knock-down) are reduced-order fits
with a per-class coefficient, not first-principles, and each names its reach at its site when built. The
electronic-structure substrate is the honest frontier: a full band structure is beyond a reduced-order floor, so
the electronic properties will ship at the free-electron / density-of-states reduced order, with the deeper
band-structure model a named follow-on. Each limit is stated at its mechanism, on the arc's discipline of naming
the reach ceiling rather than hiding it.
