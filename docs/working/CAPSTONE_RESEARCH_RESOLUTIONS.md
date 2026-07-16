# Capstone research resolutions (owner packet, 2026-07-15)

All six capstone blockers resolved in one packet, in leverage order. Ledger: nothing adds an authored scalar; the additions are compute-once constants with declared bands, two derived forms replacing reserved values, and one named closure (stellar winds). The consolidated mechanical-fetch list is at the end.

## 1. Petit instability-time coefficients (unblocks the assembly / system generator #72)

The analytic resonance-overlap `t_inst` surface (Petit et al. 2020, arXiv:2006.14903), the MANDATORY exponent carrier per the self-audit rider:
- General (arbitrary masses/spacings, initially circular coplanar), their Eq. (82):
  `<log10(T_surv/P1)> ~ -log10(eps_M * sqrt(eta*(1-eta))) - 6.72 + 6.08*(delta/delta_ov)`.
- Equal-mass equal-spacing, their Eq. (83):
  `<log10(T_surv/P1)> ~ -log10(m_p/m0) - 6.51 + 3.56*(m0/m_p)^(1/4)*Delta`.
  So the b'/c' pair is 3.56 and -6.51 in the eps^(1/4) spacing unit, with the -log10(m_p/m0) prefactor.
- Supporting: `eps_M ~ 1.22*(m_p/m0)` for equal masses/spacing (meaningful down to a test particle, varies < 3x by which body is small). The diffusion scatter is a LOG-NORMAL survival-time distribution, standard deviation 0.43 +/- 0.16 dex, INDEPENDENT of the instability-time magnitude, so the band ships as part of the measure (exactly the assembly ruling's requirement, and this corrects the earlier mis-attribution: the 0.43 dex is Petit's own).
- N >= 4: multiply the resonance density by K = 2, which raises the overlap spacing by K^(1/4) and divides the effective diffusion coefficient by K^2.
- Large masses / wide spacings: the overlap limit replaces `(1 - alpha)` with `-ln(alpha)` in the generalized spacing.
- Integrity checks (done): eps_M = 1.22*eps and eta = 1/2 in Eq. (82) reproduces Eq. (83)'s intercept (-6.72 + 0.21 = -6.51), so the surface's intercept and slope (3.56 and -6.51) are read directly from Eq. (83) and stand. CORRECTION (mechanical-fetch Eq. (61) check, `CAPSTONE_FETCH_VALUES_2.md`): the overlap spacing is delta_ov,eq = 1.16*(m_p/m0)^(1/4) per Eq. (61) (the generalized spacing; the physical orbital spacing is twice that, 2.32*(m_p/m0)^(1/4)), NOT the researcher's back-solved 0.85; the eps^(1/4) exponent is confirmed but the coefficient was off. This overlap threshold is a separate quantity from the Eq. (83) t_inst surface the assembly consumes, so the build (which merges on Eq. (83), never on delta_ov) is unaffected; use 1.16 wherever the overlap threshold itself is needed. The effective diffusion coefficient is confirmed (Eq. (62)/(76)).
- Compute-once source: the paper ships a public notebook reproducing the exact expressions (the effective diffusion coefficient, the mean and standard-deviation forms). All four constants are ensemble-calibrated compute-once, legal exponent residents.
- Also corrected by the earlier fetch input-audit: `b ~ 10 r_H` (oligarchic spacing) is Kokubo-Ida 1998, not GLS 2004; cite both.

## 2. Carbides (unblocks exotic carbon-rich worlds; branch structure closed, rows are a data pull)

Graphite/TiC/SiC condensation is governed by H, H2, C2H, C2H2, Ti, Si, and ANALYTIC formulae reproduce the detailed equilibrium (so the engine runs analytic equilibria over Gibbs rows, not table interpolation; fetch Sharp & Wasserburg 1995 for the formulae). Sequence structure (verified): graphite depends strongly on C/O and barely on pressure; TiC and SiC depend strongly on pressure and barely on C/O; TiC always before SiC. The bistable window (the Gap-Law near-degeneracy the scoping doc asked for): the order is C-TiC-SiC near unity, flips to TiC-C-SiC below C/O ~ 1.02, and to TiC-SiC-C for C/O 0.96 to 1.00 at low pressure, boundaries shifting to higher C/O at higher pressure; the presolar TiC-in-graphite window is 1.04 < C/O < 1.2 at 0.2 to 40 dyn/cm^2 with SiC ~170 K below graphite; carbon is always first above C/O ~ 1.5. Class anchors: graphite ~1600 K, SiC ~1400 K (composition and density dependent). Metallicity is a live axis (carbide temperatures rise with metallicity and pressure, TiC stays ahead of SiC), so the roster conditions on the existing abundance draws and alien admission is free; C/O itself is spent contingency from the audit-30 scatter family. Gibbs rows: SiC (both polymorphs), TiC, graphite (the zero reference) from NIST-JANAF; Fe3C is the no-JANAF metastable case, from a Barin-class compilation. Estimator tier for metal carbides without rows: the Miedema extension (Niessen & de Boer lineage), riding the 3a Miedema slot already in the ledger.

## 3. Post-main-sequence stellar tracks (#77; attractors + clocks, NO isochrone table)

Four derived pieces:
- TRIGGER: the Schonberg-Chandrasekhar limit `q_max ~ 0.37*(mu_env/mu_core)^2` (the max isothermal core fraction, ~0.10 for a He core under an H envelope), fully derived.
- TEMPERATURE ATTRACTOR: the Hayashi boundary (photospheres ~2000 to 4000 K), from the engine's own H-minus opacity physics whose steep temperature sensitivity pins the photosphere; the envelope forgets its history (the tau_forget pattern in a stellar costume).
- LUMINOSITY: shell-burning homology (Kippenhahn, extending Refsdal-Weigert), `L ~ M_H^sigma1 * R_H^sigma2` in the shell's opacity and burning-rate indices; in the radiation-pressure regime L becomes proportional to CORE MASS alone. Calibrated instance: `L = 238000 * mu^3 * Z_CNO^0.04 * (M_c^2 - 0.0305*M_c - 0.1802)` for core masses 0.5 to 0.66 solar, with two loud domain flags (above ~0.8 solar core mass, envelope convection penetrates the burning shell and the linear relation fails; below the fitted range, L is not a function of core mass alone).
- CLOCK: `dM_c/dt = L(M_c)/(X_env*E_H)` with the hydrogen yield a measured constant, so the giant-branch track `L(t)`, `T_eff(t) = Hayashi`, `R(t)` from Stefan-Boltzmann is analytic; the main-sequence clock is fuel over the existing homology luminosity. A Betelgeuse-mass star is a red supergiant by construction (core-set L, Hayashi-pinned T, radius forced to follow).
The one honest knob is MASS LOSS, a named banded closure (Reimers / de Jager class), fetched before its constant is written. Isochrone grids demote to battery hindcast rows.

## 4. Ikoma Kelvin-Helmholtz coefficients (#73 giants; banded class constant)

`tau_KH = 3e5 yr * (M_c/10 M_earth)^(-2.5) * (kappa / 1 cm^2 g^-1)` for envelope masses comparable to the core (~1e8 yr at 1 M_earth, exponent -2.5, linear in opacity). The literature spread ships as the declared band: `tau_KH ~ 10^c yr * (M_p/M_earth)^(-d)` with `8 <~ c <~ 10`, `2 <~ d <~ 4`, Ida-Lin fiducial c=9, d=3. The race's other input: `M_crit ~ 10 M_earth * (Mdot_c / 1e-6 M_earth yr^-1)^(1/4)`. The dominant lever is OPACITY (the F-ruling's abundance coupling): at 3 M_earth the growth time runs 7e2 yr metal-free, 7e4 yr for alkali opacity, 2e6 yr at one percent grains; grain-free cores above 2 M_earth reach runaway in under a Myr. So the kappa band exceeds the c/d band, and honesty is in deriving kappa from the world's own grain state, not sharpening d. Power law not exponential, so the exponents are measured-class with a band, rider-legal.

## 5. R-YOUNG-TEMPERATURE cold-branch retention (a derivable FORM, not a number)

Retention is set by three length scales (the heat-deposition depth, the impact-stirring depth, and `kappa/nu` where nu is the accreting surface's advance rate) and is INDEPENDENT of the growth rate. Regimes: impacts larger than the cooling scale bury their heat (h ~ 0.5 to 1); hierarchical planetesimal sizes are generally too large to radiate even over several-Myr accretion; h >~ 0.3 melts ice in thousand-km bodies; planetesimals under ~20 km never melt during formation absent the radionuclide term. So RETIRE the reservation with a FORM: h as the deposition-depth to cooling-skin ratio, fed by the feedstock size spectrum the oligarchy slice already produces, banded; the flat [E] prior survives only where the spectrum is unresolved. The knob-containment self-test now has a physically located band. The cold tail is the small-impactor slow-growth corner where the form itself says h is small.

## 6. Secular spectrum (#44; settled, derived all the way down, no research)

Classical Laplace-Lagrange in the eccentricity and inclination vector variables; the A and B matrices are built from Laplace coefficients the engine COMPUTES (the Laskar-Robutel hypergeometric forms, whose asymptotics the Petit fetch already delivered), not cited. Numerics: the mass-and-frequency-weighted similarity transform makes the eigenproblem real symmetric (real frequencies, deterministic fold, fixed-point safe); mode amplitudes come from the post-assembly e/i state; the AMD decomposes exactly over the modes; the mode table is the archive object the climate module reads as Milankovitch-class forcing. Discipline: take the matrix element definitions VERBATIM from Murray & Dermott chapter 7 (sign conventions are the classic hand-built-LL defect). Gap-Law hooks are native: near-degenerate eigenfrequencies and secular resonances carry rather than assert and escalate; the validity domain excludes MMR proximity (which the assembly's stability object already maps); close-in planets trip the relativity smallness flag from orbital elements alone, so the GR precession term joins the diagonal as a derived correction (Mercury handled by the existing auto-flag).

## Consolidated mechanical-fetch list (all pulls, no research)

- The Petit public notebook, plus an Eq. (61) check of the reconstructed delta_ov against arXiv:2006.14903.
- Sharp & Wasserburg 1995 for the analytic carbide condensation formulae.
- NIST-JANAF rows for SiC (both polymorphs), TiC, graphite; a Barin-class row for Fe3C.
- The Niessen-de Boer Miedema-carbide (and nitride/silicide) parameterization.
- Murray & Dermott chapter 7: the two Laplace-Lagrange matrix element definitions.
- The Reimers or de Jager wind parameterization and the helium-ignition core-mass anchor (for stellar tracks).
