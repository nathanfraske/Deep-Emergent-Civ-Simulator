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

## 5. Grain size from nucleation (CONTESTED, design-first, new physics)

The spec: "grain size writes from nucleation (CNT prefactor flag)." Classical nucleation theory gives the
nucleation rate `I = I_0 * exp(-dG*/(k_B*T))` where `dG* = 16*pi*gamma^3/(3*dGv^2)` is the nucleation barrier
(`gamma` the interfacial energy, `dGv` the volumetric free-energy driving force), and the grain size follows
from the balance of nucleation rate against growth rate against the cooling rate. This is NEW physics with its
own reserved constant: the interfacial energy `gamma` (a per-material `[E]`/`[M]` datum, surfaced reserved,
never planted), and the CNT prefactor `I_0` (the spec's "prefactor flag"). The nucleation barrier `dG*` reuses
the rate-law kernel (`exp(-dG*/(k_B*T))` is `arrhenius_rate` with a nucleation reduced barrier), so the kernel
is the consumer here too. This is the largest contested piece: I will surface the CNT form, the `gamma`
reservation with its basis, and the grain-size-from-rates balance for the gate's design-first ruling before
building, and it may be its own slice after the Dodson closure lands.

## 6. Provenance, byte-neutrality, and the build order

Provenance: every reserved value is surfaced with basis and caller-supplied or floor-read, never planted, on
the #187 discipline. The Dodson geometry constant `A` is derived from the named geometry; the `kT` boundary is
derived; the CNT interfacial energy `gamma` and prefactor are the reserved constants of the grain slice,
surfaced with basis. No rate value is authored.

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
