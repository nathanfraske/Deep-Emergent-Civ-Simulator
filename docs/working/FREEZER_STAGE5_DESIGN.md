# The freezer (Stage 5): the barrier derive-first hunt and the slice design

This is the design opener for the freezer, the rate-law kernel's first consumer, gate-directed on #187 after
the kernel signed off. It authors no value. Its first job is the derive-first hunt the gate ruled DUE: whether
the freezing/self-diffusion barrier `E*` derives from the already-built Rose cohesive energy `E_coh` rather
than reading the class constant `g`. The design chain and the contested calls follow, for the gate's review
before any value is entered.

## 1. Where the freezer sits, and what it reads

The oracle spec Stage 5 (`MATERIALS_ORACLE_SPEC.md`, the g-framework): the freezer turns the equilibrium
assemblage into the REALIZED assemblage by racing kinetics against the world's cooling rate. Its atom is a
thermally-activated rate, `D ~ a^2 * nu * exp(-E*/(R*T))`, which is exactly the kernel just built:
`arrhenius_rate(prefactor, reduced_barrier)` with `prefactor = nu = c_s/a` (the attempt frequency) and
`reduced_barrier = E*/(R*T)` (formed by `reduced_barrier(E*, R*T)`). The spec's canonical closure is
`D0 ~ a^2 * nu ~ 1e-5 m^2/s`.

Grounded against source, the inputs the freezer needs are already built or one derivation away: `E_coh` (kJ/mol,
`metallic::MetallicRoute::cohesive_energy`, the Rose-anchored atomization enthalpy for the seven anchored
metals); `B_0` and `V_m` (the EOS anchors, `metal_eos`); the atomic mass `M` (periodic table); and the lattice
scale `a` (`rose_eos::wigner_seitz_radius_angstrom`). The sound speed `c_s = sqrt(B_0/rho)` and the Debye
temperature `theta_D` are each one closed-form step from those. What is NOT built and must be designed is `T_m`
(Lindemann plus the Slater gamma) and the barrier `E*`, which is where the derive-first hunt bites.

The gate also named a coupling to hold: the freezer is where the kinetic CONDENSATION departures will live
(fractional condensation, the CO and N2 quenches, the Prinn-Barshay machinery), so Stage 5 is a SHARED consumer
for the materials freezer and the future disk-condensation extension. The barrier and rate machinery must stay
keyed on the material's OWN data (its `E_coh`, its class), never specialized to metals, so the condensation
extension is a data path rather than a rewrite (admit-the-alien).

## 2. The barrier derive-first hunt (the DUE `g` fork)

The spec writes the barrier `E* = g * R * T_m` with `g` a class-and-mechanism constant, tagged `[M class]`
(17 to 18 for close-packed metals, 26 to 30 for covalent). The gate ruled the derive-first fork DUE: run the
hunt before entering `g`. Here it is, grounded.

**The two candidate forms.**

Form A, the spec's current: `E* = g * R * T_m`. It reads two things the freezer must produce: `T_m`, which is
NOT built and must be derived (Lindemann plus Slater gamma, itself an estimator with its own constant), and
`g`, a per-class empirical constant.

Form B, the sharper derive-first: `E* = f * E_coh`, where `f` is the per-class fraction of the cohesive energy
spent forming and moving the defect that carries diffusion. It reads `E_coh`, which is ALREADY BUILT (the Rose
route), and `f`, a per-class constant.

**The physics linking them, and why it matters.** The self-diffusion barrier is `Q = H_vf + H_vm` (the vacancy
formation plus migration enthalpy). Both terms scale with the cohesive energy: the atoms that must be broken to
make and move a vacancy are the same bonds `E_coh` measures. Independently, the melting point also scales with
the cohesive energy (both are bond-strength proxies): the ratio `k = E_coh/(R*T_m)` is itself a near-constant
per class. So the spec's `g` is a COMPOSITE, `g = k * f`: writing `g * R * T_m` is writing `(k * R * T_m) * f`,
which is `E_coh * f` once the cohesive-to-melting ratio `k` is folded in. Form A hides the cohesive-to-melting
ratio inside `g`; Form B pulls it out and reuses the already-derived `E_coh` directly.

**The verdict (surfaced, not settled: the owner rules `g`).** Form B is MORE derive-first than Form A, for three
grounded reasons. First, it reuses a quantity the floor already derives (`E_coh`, Rose UBER) rather than routing
the barrier through a not-yet-built `T_m`, so it is one derivation hop shorter and does not compound the `T_m`
estimator's uncertainty into the barrier. Second, `f` is the more fundamental constant: it is the vacancy
fraction of the cohesive energy, a single physical ratio with a direct literature basis (`H_vf` is a known
fraction of `E_coh`), whereas `g` multiplies that fraction by the separate cohesive-to-melting ratio, so `g`
carries two correlations where `f` carries one. Third, keying the barrier off the material's own `E_coh` and its
own bonding class is the admit-the-alien form the condensation coupling needs.

**But the win is PARTIAL, stated plainly (the gate's standing instruction).** Neither form derives the barrier
to zero class constants. The vacancy energetics `H_vf` and `H_vm` are not floored, and deriving them from first
principles needs a defect-energy calculation the floor does not carry. So the barrier reads exactly ONE per-class
empirical constant either way: `g` in Form A, `f` in Form B. Form B moves the constant from a wholesale
composite to a smaller, more physical residual, and reuses a built quantity, but it does not eliminate the
class constant. The derive-first gain is real and worth taking; it is not a derivation to zero.

**A consistency twin the fork gives for free.** Because `T_m` is derived anyway (Section 3 needs it for the
`D0` normalization and the Frost-Ashby creep axis), BOTH `g * R * T_m` and `f * E_coh` are computable at run
time, and they must AGREE within the class scatter (they are the same barrier expressed two ways, related by
`E_coh = k * R * T_m`). That agreement is a numerical twin on the barrier: if the derived `E_coh`, the derived
`T_m`, and the entered class constant do not reconcile to a single barrier within tolerance, the build fails.
It validates whichever constant the owner enters against the other route, the same twin discipline the kernel
carries.

**Reserved, never entered (surfaced with basis, for the owner's ruling):**
- The barrier form itself (Form B recommended, `E* = f * E_coh`, over Form A `g * R * T_m`). Basis: the
  derive-first analysis above; the owner rules the form.
- If Form B: the per-class vacancy fraction `f` (the fraction of cohesive energy forming plus moving the
  diffusion carrier). Basis: `H_vf + H_vm` as a fraction of `E_coh`, cited to the vacancy-energetics literature
  (Brown and Ashby 1980, the deformation-mechanism correlations; the primary source verified before entry, the
  same flag-first discipline that caught every book-seam), per bonding class.
- If Form A: the class constant `g` (17 to 18 close-packed, 26 to 30 covalent), cited to the Van Liempt 1935 /
  Sherby-Simnad 1962 correlation, verified at primary source before entry.
- Either constant keys off the material's OWN bonding class (a data row), never a hardcoded scalar.

## 3. The freezer derivation chain (design)

The freezer is a pure function of the built anchors plus the reserved constants, computing the realized
assemblage. The chain, each step a closed-form fixed-point law over the built floor:

1. `c_s = sqrt(B_0 / rho)` from the EOS anchor `B_0` and the density `rho` (from `V_m` and `M`). The bulk sound
   speed; `sqrt` is a built `Fixed` op.
2. `a = wigner_seitz_radius(V_m)`, the lattice scale (already built).
3. `nu = c_s / a`, the attempt frequency (the kernel's `prefactor`).
4. `theta_D` (Debye) from `c_s` and the atomic volume, the standard Debye relation.
5. `T_m` via Lindemann (`T_m ~ theta_D^2 * M * V^(2/3)` up to the Lindemann constant) plus the Slater gamma
   correction, with cryoscopic depression for solutions. THIS IS A CONTESTED DESIGN CALL (Section 4).
6. `E*` via the barrier form the owner rules (Section 2), keyed off `E_coh` and the material's class.
7. `reduced_barrier(E*, R * T)` then `arrhenius_rate(nu, .)` gives the mobility `D`, closing `D0 ~ a^2 * nu`.
8. Dodson closure per exchange reaction against the world's cooling rate `h`, quenching compositions. The
   closure temperature `T_c` is an IMPLICIT solve (a root-find the freezer wires AROUND the kernel), not a
   kernel concern.
9. Sub-kT polymorphs resolve by seeded draw (the `[X]` named slot), the same content-keyed seeded draw the
   Verdict kernel already carries. THIS IS A CONTESTED DESIGN CALL (Section 4).

Output: the realized assemblage and its distance from equilibrium (the archive record).

## 4. The contested design calls (design-first, gate reviews before a value)

Two steps are design calls the gate reviews before I build them, per its ruling:

- **The Lindemann `T_m` form (step 5).** The Lindemann criterion has several equivalent statements and a
  reserved Lindemann constant; the Slater gamma correction and the cryoscopic depression each add form. The
  float-free fixed-point realization (the `sqrt` and the `^(2/3)` power) needs care, and the `^(2/3)` is a
  fractional power, which couples to the determinism gate the way the geology stream-power exponent does (the
  fractional-power primitive, task-adjacent). I will surface the exact `T_m` form and its determinism handling
  for the gate before entering the Lindemann constant.
- **The seeded-draw terminal (step 9).** The spec names it `[X]`, a named slot: sub-kT polymorphs resolve by a
  seeded draw. The Verdict kernel already carries a content-keyed seeded draw, so the mechanism exists; the
  design call is the resolution boundary (what counts as sub-kT) and the draw's provenance key. I will surface
  the terminal's framing for the gate.

The Dodson root-find (step 8) is a freezer-side numerical concern (a bounded fixed-iteration solve with an
integer tolerance, like the Kepler and the assemblage solves), not a kernel concern and not a value; it is
noted here so the wiring is expected.

## 5. Byte-neutrality, honest limits, the reserved list

Byte-neutral: the freezer is a materials leaf (like the disposer), with no run_world consumer, so building it
moves no run pin, proven differentially per push as the kernel was.

Honest limits, stated up front. The barrier reads one per-class constant either way (Section 2); the derive
gain is partial. `T_m` is a Lindemann ESTIMATOR (the spec tags it `[E]`), so its uncertainty propagates to the
`D0` normalization and the creep axis. The seven anchored metals are the current coverage; a material without a
banked `E_coh` and EOS anchors escalates (the honest refusal the metallic route already gives), so the freezer
serves the anchored set first and the condensation extension is a later data path. The fractional-power `T_m`
form couples to the open determinism fractional-power primitive.

The reserved list (never entered, surfaced with basis, the owner rules each): the barrier form (Form B
recommended); the per-class barrier constant (`f` or `g`) with its cited basis; the Lindemann constant and the
Slater gamma form; the cryoscopic-depression form; the seeded-draw resolution boundary; the Dodson closure
tolerance and iteration cap. Nothing is entered until the gate reviews this design and rules the barrier fork.
