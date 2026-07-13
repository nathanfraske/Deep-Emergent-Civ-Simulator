# The freezer output side (Stage 5, part 2): the realized assemblage, design-first

This is the design opener for the freezer's OUTPUT side, the second half of Stage 5, gate-sequenced on #187
after the kinetics engine (the rate law, barrier, derived `T_m`, attempt frequency, creep axis) passed its
section-9 audit. It authors no value and builds no mechanism. Its purpose is to surface the output-side design
and its contested calls for the gate's review before a line is built, so #187's watch bridges to this PR before
#187 merges. Each contested piece below is surfaced design-first; nothing is built until the gate rules it.

This PR is branched from the #187 head; the diff collapses to this one doc once #187 squash-merges and this
branch rebases onto the new main.

## 1. What the output side delivers

The kinetics engine (Stage 5, part 1, on #187) is per-substance: it turns a substance's `B_0`, `V_m`, `E_coh`
into a barrier, a melting point, an attempt frequency, and a diffusivity. The OUTPUT side is the deliverable the
spec names for Stage 5: "the realized assemblage, and its distance from equilibrium is the archive"
(`MATERIALS_ORACLE_SPEC.md`). It consumes the EQUILIBRIUM assemblage (the disposer's output over a composition)
and the environment path, and returns the REALIZED assemblage after kinetics race the cooling rate: which
compositions quench frozen, which re-equilibrate, which polymorph is drawn, and the grain texture.

So the output side is a new interface, not a per-substance call: composition plus environment plus path in, a
realized assemblage out. That is a genuine Stage-3/4 integration, which is why it is its own PR (the boundary
the gate ruled).

## 2. The assemblage interface (design-first)

The disposer already returns `Verdict<Compound>` (`crates/materials/src/verdict.rs`, `crates/materials/src/
thermochemical/disposer.rs`): a sealed typestate over candidate `Compound` phases, with `Decided`/`Trivial`
(a winner), `Escalate` (near-degenerate, no winner), and `SeededDraw` (a content-hash-keyed pick). The
equilibrium assemblage is the set of disposer-decided phases over the composition.

The output side takes that equilibrium assemblage plus the environment `E = (T, P)` and the path `h` (the
cooling rate, and any prior assemblage), and returns the realized assemblage plus the distance-from-equilibrium
archive. The design calls for the gate:

- How the equilibrium assemblage is represented at the interface: the set of `Compound` winners the disposer
  decided, or a richer assemblage type carrying phase fractions. The minimal form is a set of `(Compound,
  amount)` with the disposer's `Verdict` retained for the escalate/draw cases.
- What the "distance from equilibrium" archive records: the realized-versus-equilibrium composition difference
  per exchange, the quench temperature, and the metastable-inheritance flags. This is a written record `[W]`,
  not a reserved value.
- The metastable inheritance rule (the spec's "diamond persists"): a phase whose re-equilibration is closed
  by the cooling rate is inherited unchanged. This falls out of the Dodson closure (below): if `T_c` is above
  the current temperature, the exchange is frozen and the phase inherits.

## 3. The Dodson closure: the `T_c` root-find (CONTESTED, design-first)

The spec: "Dodson closure per exchange reaction against h's cooling rate quenches compositions." Dodson (1973)
defines the CLOSURE TEMPERATURE `T_c`, the temperature at which diffusive re-equilibration can no longer keep
pace with cooling, so the composition freezes in. The Dodson relation is implicit in `T_c`:

    E*/(R*T_c) = ln( A * R * T_c^2 * D_0 / (E* * a^2 * |dT/dt|) )

where `E*` is the exchange barrier (Form B, built on #187), `D_0 = a^2 * nu` is the pre-exponential diffusivity
(built on #187: the attempt frequency times the squared spacing), `a` is the diffusion length, `|dT/dt|` is the
cooling rate from `h`, and `A` is a geometry constant (Dodson's, depending on the diffusion geometry: plane
sheet, cylinder, sphere). `T_c` appears on both sides, so it is a BOUNDED ROOT-FIND, the same shape as the
Kepler solve and the assemblage LP: a fixed iteration cap tested by an integer `Fixed` tolerance, never an
unbounded loop, so replay and worker-invariance hold. The freezer already supplies `D(T)` and `D_0`, so the
Dodson balance is evaluable; what it adds is the root-find and the cooling-rate coupling.

The contested design calls for the gate before I build this:

- The cooling-rate coupling: `|dT/dt|` reads from `h` (the path record). Confirm the path carries the cooling
  rate as a datum the freezer reads, versus deriving it from the environment's temperature history.
- The geometry constant `A`: Dodson's `A` is a derived value for a named geometry (a well-known set: 55 for a
  sphere, 27 for a cylinder, 8.7 for a plane sheet, from the diffusion-equation eigenvalues). It is DERIVED
  from the geometry, not authored, but the choice of geometry is a per-exchange modeling call. Surface it.
- The diffusion length `a`: the grain size or the exchange length. This couples to the grain-size output
  (Section 5), so the two are designed together.
- The root-find determinism: the fixed iteration cap and the integer tolerance, on the Kepler/assemblage
  pattern. No new transcendental beyond the built `Fixed::exp`/`ln`.

## 4. The seeded-draw terminal on the derived `kT` boundary (design-first)

The spec: "sub-kT polymorphs resolve by seeded draw." The gate ruled the boundary is the DERIVED `kT`, not a
reserved threshold: when competing polymorphs sit within the thermal energy `kT` of each other in free energy,
they are thermally unresolvable, and the content-hash-keyed `Verdict::SeededDraw` (already built) is the correct
terminal; above `kT` the disposer decides. So there is no value to reserve here, only the derived `kT`
comparison feeding the existing seeded draw. The design call: `kT` at which temperature (the freezing
temperature `T_c` from the Dodson closure, the natural choice), and the free-energy gap the disposer already
computes as the comparison input. This piece is the near-ready one: it wires the built `Verdict::SeededDraw` to
the derived `kT` boundary, no new physics.

## 5. Grain size from nucleation: the derivation-hunt on `gamma` and `I_0` (design-first, gate ruling)

The spec: "grain size writes from nucleation (CNT prefactor flag)." Classical nucleation theory gives the
steady-state nucleation rate

    I = I_0 * exp(-dG*/(k_B*T)),   dG* = 16*pi*gamma^3 / (3*dGv^2),

with `dG*` the nucleation barrier, `gamma` the solid-liquid interfacial energy, and `dGv` the volumetric
free-energy driving force. The barrier reuses the rate-law kernel directly: `exp(-dG*/(k_B*T))` is
`arrhenius_rate` with the nucleation reduced barrier `dG*/(k_B*T)`, so the kernel is the consumer here too.
Before reserving `gamma` and `I_0`, I hunted each the way the vacancy fraction and the Lindemann ratio were
hunted (the gate's instruction: reserve only the irreducible residual). The result is that almost nothing is
left to reserve.

**The `gamma` hunt (and a prove-it catch on this document's own opener).** The opener hypothesized
`gamma ~ per-class surface-fraction * E_coh / area`, the broken-bond model. Testing it against the source: the
broken-bond model over `E_coh` gives the SOLID-VAPOR surface energy `gamma_sv` (the bonds cut to expose a
surface to vacuum), which is the wrong interfacial energy for freezing from a melt. Nucleation from the liquid
keys on the SOLID-LIQUID interfacial energy `gamma_sl`, five-to-ten times smaller than `gamma_sv` because the
melt is a condensed phase and few bonds are fully broken across the interface. Forcing the `E_coh` broken-bond
route to reproduce `gamma_sl` would demand a "surface fraction" secretly absorbing the melt entropy, hidden work
rather than an honest coordination fraction. So the `E_coh` route is rejected for `gamma_sl`.

The derive-clean route is Turnbull (1950): `gamma_sl = C * dH_f / (N_A^(1/3) * V_m^(2/3))`, the interfacial
energy as a coefficient `C` times the heat of fusion `dH_f` over the atomic interfacial area. The heat of fusion
is not a new datum: by Richards' rule the entropy of fusion `dS_f` is near-constant (about `R` for close-packed
metals), so `dH_f = T_m * dS_f ~ R * T_m`, keyed on the BUILT Lindemann `T_m`. The two combine and collapse
(`N_A^(1/3) V_m^(2/3) = N_A * V_atom^(2/3)`, `R/N_A = k_B`) to

    gamma_sl ~ beta_gamma * k_B * T_m / V_atom^(2/3),   beta_gamma = C * (dS_f / R),

the interfacial energy as the thermal energy per unit atomic area, scaled by ONE dimensionless per-class
coefficient. Every other factor is built: `T_m` (the Lindemann collapse), `k_B` (the exact fold already in the
freezer), and `V_atom^(2/3) = cbrt(V_atom)^2` (the exact `cbrt` the Lindemann factor already uses). The source
confirms the physics: the measured `gamma_sl` of close-packed metals is "primarily entropic in origin, linearly
correlated with the melting temperature," which is the `k_B * T_m / a^2` scaling. A magnitude sanity-check (NOT
a planted value): iron at `C = 0.45`, `dH_f = 13.81 kJ/mol`, `V_atom ~ 11.77 A^3` gives `gamma_sl ~ 0.20 J/m^2`
by both the direct Turnbull form and the collapse, against the measured `~0.204 J/m^2`.

So `gamma_sl` reserves exactly ONE dimensionless per-class coefficient `beta_gamma` (Turnbull's `C` folded with
Richards' `dS_f/R`, about 0.45 for close-packed metals and about 0.32 for the nonmetals Bi, Sb, Ge, and water),
the Form-B shape, sibling to the vacancy fraction `f` and the Lindemann ratio `delta`. Its basis, surfaced for
the owner: the interfacial energy as a fraction of `k_B*T_m/a^2`, cited Turnbull 1950 and Kelton and Greer, per
bonding class, verified at the primary source before entry. If the elastic anchors later carry a measured heat
of fusion `dH_f`, `gamma_sl` reads it directly and only Turnbull's `C` remains reserved (the read-if-available,
derive-if-not pattern).

**The `I_0` hunt (nothing left to reserve).** The steady-state CNT prefactor is `I_0 = N_s * Z * beta*` (Kelton
and Greer; Christian): the number density of nucleation sites `N_s`, the Zeldovich factor `Z`, and the atomic
attachment rate `beta*` to the critical nucleus. Each decomposes into a built primitive. `N_s` is the site
density = the material density (anchored) over the atomic mass (the periodic-table floor), no new datum.
`Z = sqrt(eta / (2*pi*k_B*T))` with `eta = -d^2G/dn^2` at the critical size is a pure function of the barrier
curvature, computed from `dG*`, `k_B*T`, and the critical size, all in hand once `gamma` is set, over the built
`Fixed::sqrt`; no new datum. `beta*` is the interface-attachment rate, diffusion-limited from the melt, so it is
the BUILT self-diffusivity (`self_diffusivity`, the attempt frequency `nu` through the kernel) times the
geometric surface-atom count of the critical nucleus; no new datum. So the spec's "CNT prefactor flag" resolves
to NO authored prefactor: `I_0` composes from the anchored density, the periodic-table mass, the built attempt
frequency, and the computed barrier curvature.

The remaining driving force `dGv` also derives: `dGv = dS_f * dT / V_atom ~ k_B * (dS_f/R) * (T_m - T) / V_atom`,
the undercooling `dT = T_m - T` (built `T_m`, environment `T`) times the same `dS_f/R` fraction over the atomic
volume (anchored). The grain size then follows from the balance of the nucleation rate `I` against the growth
rate against the cooling rate (the Johnson-Mehl-Avrami / time-temperature-transformation logic), a dynamics over
the built rates, no new constant.

**The reserved residual, and the honest limits.** After the hunt, the whole CNT grain mechanism reserves exactly
ONE new value: the per-class interfacial coefficient `beta_gamma` (or Turnbull's `C` alone, if a measured `dH_f`
is anchored). `I_0`, `dG*`, `dGv`, and the Zeldovich factor reserve nothing. The honest limits, each to be
stated at its site when built: FIRST, this is the HOMOGENEOUS baseline; real solidification is often
HETEROGENEOUS (on walls, impurities, prior grains), which multiplies `dG*` by the contact-angle potency
`f(theta) = (2 - 3*cos(theta) + cos^3(theta))/4` in `[0,1]` and changes `N_s` to substrate sites, so the wetting
contact angle `theta` (a per-interface-pair dimensionless datum) is the reserved residual of the heterogeneous
follow-on, not this slice. SECOND, the CRYSTALLINE regime carries over from the kinetics engine (a glass has no
CNT nucleation barrier of this form). THIRD, the lumped single-rate treatment is reduced-order, not a full
cluster population balance. I will build the grain slice only on the gate's ruling of this hunt; if the gate
concurs, `beta_gamma` is the one reserved constant, surfaced with its basis, and everything else is built or
derived.

## 6. Provenance, byte-neutrality, and the build order

Provenance: every reserved value is surfaced with basis and caller-supplied or floor-read, never planted, on
the #187 discipline. The Dodson geometry constant `A` is derived from the named geometry; the `kT` boundary is
derived; after the section-5 hunt the CNT interfacial energy `gamma_sl` reserves ONE per-class coefficient
`beta_gamma` (the Turnbull-Richards fraction, keyed on the built `T_m`), and the prefactor `I_0` reserves
nothing (it composes from the anchored density, the periodic-table mass, and the built attempt frequency and
barrier curvature), each surfaced with basis. No rate value is authored.

Byte-neutrality: materials is a leaf not linked into the run_world binary, so the output side moves no run pin,
proven differentially per push as the kinetics engine was.

Build order, gated per push, each contested piece surfaced for the gate's review before I build it: (a) the
assemblage interface and the Dodson `T_c` root-find with the metastable-inheritance rule (the core quench);
(b) the seeded-draw terminal on the derived `kT` boundary (the near-ready wire); (c) grain size from CNT (the
new-physics slice, its own design-first ruling). On completion the output side delivers a working Stage 5: a
composition and environment in, a realized frozen assemblage and its distance-from-equilibrium archive out.

## 7. Honest limits

The output side inherits the kinetics engine's limits (the crystalline regime, the bulk-sound-speed Debye
velocity, the per-class reserved constants). It adds the Dodson single-diffusion-length approximation (one `a`
per exchange, not a full spatial profile), the CNT reduced-order nucleation (a lumped rate, not a full
population balance), and the named-geometry choice for `A`. Each is stated at its site as it is built, on the
kinetics engine's discipline of naming the reach ceiling rather than hiding it.
