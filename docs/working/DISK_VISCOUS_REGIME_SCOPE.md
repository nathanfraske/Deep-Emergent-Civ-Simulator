# The disk-thermal viscous regime and opacity closure (design-first scope)

This scopes the next capstone front-end rung the gate directed after the irradiated-regime disk temperature
(`491e1e4`) signed off: complete the two-regime disk-thermal profile so the condensation sequence reads the
correct local temperature at each orbit. It is design-first, grounded, and surfaced for the gate to steer the
slicing and the residue treatment before any machinery is built.

## 1. Why the irradiated regime alone is incomplete

`irradiated_disk_temperature` (front-end slice 2) covers the IRRADIATED, passive, optically-thin regime: the disk
annulus at `r` reprocesses the stellar flux `F(r) = L/(4*pi*r^2)` and reaches `sigma*T^4 = reprocessing_factor*F(r)`.
That is the OUTER disk. The disk-thermal skeleton condensation needs has two derived heat sources, not one:

- The IRRADIATED-OUTER regime (built): starlight reprocessed at the surface, `T ~ r^(-1/2)`.
- The VISCOUS-INNER regime (this arc): accretional heating from the disk's own inward mass flow, dissipated by
  turbulent viscosity, which dominates inside a transition radius and runs hotter than pure irradiation there,
  with `T ~ r^(-3/4)`.

This bites at the Hadean gate specifically. Earth forms at 1 AU, near the regime transition, so the local
temperature that sets the refractory-versus-volatile split at 1 AU depends on which source dominates there. The
DRY-at-1-AU prediction is robust (1 AU sits well inside the water snow line under either regime), but the finer
condensation-front placement is regime-sensitive, so both regimes must be derived before condensation reads them.

## 2. The physics, grounded

**The viscous effective temperature.** For a steady thin accretion disk (Shakura-Sunyaev), the viscous
dissipation rate per unit area per face is `D(r) = (3/(8*pi)) * Mdot * Omega_K^2 * (1 - sqrt(R_in/r))`, with the
Keplerian frequency `Omega_K^2 = G*M_star/r^3`. Each face radiates `sigma*T_visc_eff^4 = D(r)`, so
`T_visc_eff = (D(r)/sigma)^(1/4)`, the same Stefan-Boltzmann inversion the irradiated regime uses (it reuses
`radiative_equilibrium`). `Mdot` is the mass-accretion rate, `R_in` the inner-boundary (truncation) radius.

**The regime combination.** The two heat sources add in flux, so the disk's effective temperature is
`T_eff^4 = T_visc_eff^4 + T_irr^4`. Viscous dominates the inner disk (steep `r^(-3/4)`), irradiation the outer
(`r^(-1/2)`), and the sum transitions between them at the radius where they cross. No authored transition radius:
it emerges from where the two derived terms cross (Principle 8).

**The optically-thick midplane and the opacity closure.** In the optically-thick inner disk the MIDPLANE runs
hotter than the surface effective temperature, boosted by the optical depth to the midplane: to leading order
`T_mid^4 = (3/4)*tau_R * T_visc_eff^4 + T_irr^4`, where the Rosseland optical depth `tau_R = kappa_R(T_mid)*Sigma/2`.
Irradiation heats the surface, so it is not boosted; the viscous term, dissipated in the interior, is. The
opacity `kappa_R(T)` is temperature-dependent (a piecewise Rosseland mean: ice grains, dust, dust sublimation,
molecular gas), so `T_mid` appears on both sides: it is IMPLICIT, a `T <-> kappa` fixed point. This is where the
surface density `Sigma(r)` (the deferred stage-2 half) re-enters, as the column the optical depth integrates.

## 3. Proposed slicing (three slices, each byte-neutral until a scenario arms them)

- **3a: the viscous effective temperature. BUILT (byte-neutral).** `viscous_disk_temperature` (with the private
  `viscous_dissipation_flux`), computing `D(r) = (3/(8*pi))*Mdot*Omega_K^2*inner_boundary_factor`,
  `Omega_K^2 = G*M_star/r^3`, in the wide-BigRat path (the operands `Mdot`, `G`, `M_star`, `r^3` overflow or
  underflow Q32.32 while the ~few W/m^2 result fits) and the fourth root through `radiative_equilibrium`. `Mdot` is
  the caller residue (in M_sun per megayear, Mirror ~0.01), `G` read from the fundamentals register (single
  source), `M_sun` and the Julian year the cited unit anchors. Derive-not-fit anchor: Mirror's disk at 1 AU
  derives `T_visc` ~85.1 K, below the ~278 K irradiation there (irradiation leads at 1 AU, viscous dominates well
  inside), with the `r^(-3/4)` slope. Reuses the exact pattern `stellar_flux` and the irradiated slice proved.
- **3b: the regime combination. BUILT (byte-neutral).** `disk_effective_temperature` sums the two heat sources
  at the FLUX level (`sigma*T_eff^4 = D_visc + reprocessing_factor*F_irr`) and inverts once through
  `radiative_equilibrium`, which sidesteps the unrepresentable `T^4` (`T_irr^4 ~ 6e9` overflows Q32.32 while the
  fluxes do not). With no accretion it reduces to `irradiated_disk_temperature` exactly; with strong accretion at a
  close orbit the effective temperature tracks the viscous term. At 1 AU it derives ~278.8 K (the ~278.2 K
  irradiation plus the ~85 K viscous in quadrature), the transition to viscous-dominated emerging inward with no
  authored boundary.
- **3c: the opacity closure and the optically-thick midplane.** The Rosseland opacity `kappa_R(T)` as a
  data-defined piecewise law, the bounded `T <-> kappa` midplane fixed point, and the optical-depth correction,
  reading `Sigma(r)` (built alongside as the surface-density input). The heaviest slice; the fixed-point machinery.

## 4. Reserved residues, each surfaced with its basis (never fabricated)

- **The accretion rate `Mdot`** (the deep residue). Two ways to carry it, a question for the gate:
  (a) `Mdot` as a caller residue directly (one number, ~10^-8 M_sun/yr for a T-Tauri disk, its basis the observed
  class-II accretion rate of the disk's age); or (b) the `alpha`-turbulence residue as the primitive, with
  `Mdot = 3*pi*nu*Sigma` and `nu = alpha*c_s*H` derived (more first-principles, but it pulls in the sound speed and
  scale height, so it needs `T` and `Sigma` already, a tighter coupling). Recommendation: (a) for slice 3a (clean,
  one residue), with (b) as the deepening that retires it, mirroring how the mass-luminosity exponent is a residue
  now and the deeper stellar-structure solve retires it later.
- **The opacity law `kappa_R(T)`.** A piecewise Rosseland-mean power law, its coefficients and exponents cited
  (Bell and Lin 1994, or Semenov 2003), carried as a DATA-DEFINED opacity registry (a sibling to the phase
  registry: fixed Rust evaluator, data membership that grows). The regime boundaries (the ice line, dust
  sublimation) are NOT authored temperatures: they EMERGE from where adjacent power-law segments cross (Principle 8),
  the same pattern the band-gap and relief classifications use.
- **The inner-boundary radius `R_in`** (the `(1 - sqrt(R_in/r))` factor), its basis the stellar radius or the
  magnetospheric truncation radius; ~1 away from the inner edge, so it can default to unity for slice 3a and be
  surfaced when it matters.
- **The midplane-correction structural factor** (the `(3/4)*tau` leading form versus the fuller
  `(3/8)*tau + 1/2 + 1/(4*tau)` Hubeny closure), a modeling choice with its basis in the radiative-transfer
  closure adopted, surfaced in slice 3c.

## 5. The determinism discipline (the fixed-point is the landmine)

The `T <-> kappa` midplane fixed point MUST be a BOUNDED fixed-cap solve, never an unbounded until-converged spin
(the byte-neutrality and determinism hazard the capstone scope calls out). It mirrors the surface-energy balance
already in the tree: a bounded bisection over `[0, T_max]` with a fixed iteration count set so the bracket collapses
below the Q32.32 resolution (the existing `SURFACE_BALANCE_ITERS = 64` is the model), so the root is the exact
fixed-point solution and any count at or above the collapse threshold gives the identical result. Fixed-point
kernels, a fixed cap, integer-only: determinism holds by construction.

## 6. The reprocessing-factor reconciliation (the gate's flag on slice 2)

The gate flagged that `irradiated_disk_temperature`'s `1/4` folds the surface-versus-midplane optical-depth
distinction into one number. The resolution in the completed profile: the irradiation term KEEPS its surface
form (the `1/4` optically-thin value is the correct passive-surface equilibrium, and irradiation heats the surface,
not the interior), while the optically-thick BOOST applies to the VISCOUS term through `tau_R`. So
`irradiated_disk_temperature` stays the irradiation contribution (`T_irr`), and the two-regime function is the new
completed reader condensation consumes. Question for the gate: keep `irradiated_disk_temperature` as the `T_irr`
term unchanged, or fold it into the combined function.

## 7. Derive-not-fit anchors (the acceptance checks, never fit)

- The viscous-versus-irradiation transition: viscous dominates the inner disk, irradiation the outer, and the
  transition radius emerges from the crossing (for a Mirror-like `Mdot` it sits inside ~1 AU, so 1 AU is
  irradiation-leaning, the regime the gate noted).
- The optically-thick inner midplane runs hotter than its surface effective temperature (the optical-depth boost
  is greater than one where `tau_R > 1`).
- The water snow line lands at ~2 to 3 AU, the radius where the completed `T(r)` crosses ~150 to 170 K. This is
  the anchor the DRY-at-1-AU Hadean prediction rides on, and it must fall out of the derivation, never be placed.

## 8. What is asked of the gate

Confirm the slicing (3a viscous `T_eff`, 3b regime sum, 3c opacity closure plus midplane fixed point), and rule
on three choices: (1) `Mdot` as a caller residue now versus `alpha` as the primitive with `Mdot` derived;
(2) the opacity as a data-defined registry with emergent regime boundaries (the recommended shape); and
(3) whether `irradiated_disk_temperature` stays the `T_irr` term or is subsumed. Slice 3a is ready to build the
moment the slicing and the `Mdot` treatment are confirmed.
