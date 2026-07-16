# Capstone literature-fetch values

This file records the primary-source fetch for the pending value/form/condition list behind several capstone design rulings (R-ASSEMBLY seeded-draw, gate G disk-evolution, R-YOUNG-TEMPERATURE, the texture rulings, and the heat-producer abundance column). For each item the label is one of [M] VALUE (a number with units), FORM (an equation or closure with its own coefficients), or CONDITION (a branch criterion). Each carries the extracted content, the primary citation (author, year, journal, arXiv or DOI, and the specific equation, table, or figure), a confidence, and any caveat.

Discipline note: PDFs did not parse through the fetch path, so equations and tables were read from HTML sources (ar5iv, journal full-HTML, PMC, and arXiv HTML) and, where a value is quoted by a later paper, that reproduction is stated as such while the citation points to the primary. Nothing here is a fabricated number. Where a specific coefficient could not be pinned to the primary, it is flagged in the closing "Partially verified" section rather than guessed.

---

## Group A: planetary-assembly dynamics (R-ASSEMBLY seeded-draw, gate G)

### A1. Oligarchic spacing (Goldreich, Lithwick & Sari 2004)

- **Label:** FORM (physical balance) plus a provenance correction on the numeric spacing.
- **Extracted:** The Hill radius is defined in Section 2.1, Eq. (1) as R_H ~ R/alpha with alpha = (rho_sun/rho)^(1/3) (R_sun/a) (Eq. 2), which reduces to the standard R_H ~ a (M / M_sun)^(1/3). Oligarchy is governed by the balance between viscous stirring among the big bodies and dynamical friction cooling by the small bodies. It ends (Section 8) when the surface density of oligarchs Sigma becomes comparable to that of the small bodies sigma, at which point dynamical friction can no longer balance viscous stirring and the oligarchs' random velocities climb. This is the repulsion-versus-dynamical-friction balance the ruling refers to.
- **Primary citation:** Goldreich, Lithwick & Sari 2004, "Planet Formation by Coagulation: A Focus on Uranus and Neptune", Annual Review of Astronomy and Astrophysics 42, 549-601, DOI 10.1146/annurev.astro.42.053102.134004, arXiv astro-ph/0405215. Hill-radius definition Eqs. (1)-(2), Section 2.1; oligarchy-ending balance Section 8.
- **Confidence:** High on the physical balance and the Hill-radius definition. Low on attributing a specific numeric separation to this paper.
- **Caveat (input audit catch):** Goldreich, Lithwick & Sari 2004 does not state the "~10 mutual Hill radii" oligarch separation as a standalone analytic equation. That specific number is the empirical/analytic result of Kokubo & Ida 1998 ("Oligarchic growth of protoplanets", Icarus 131, 171-178, DOI 10.1006/icar.1997.5840), who find neighbouring oligarchs settle to a separation of roughly b ~ 5 to 10 mutual Hill radii (commonly taken as ~10 b_RH). For the seeded-draw arc, cite Goldreich, Lithwick & Sari 2004 for the balance mechanism and Kokubo & Ida 1998 for the ~10 mutual-Hill-radii coefficient. The Kokubo & Ida value should be read from that paper's own text before it is coded (not verified here to its equation number).

### A2. Instability time from three-body resonance overlap (Petit et al. 2020)

- **Label:** FORM plus critical-separation scaling; coefficient set partially pinned.
- **Extracted:** The central result is that the correct spacing unit is not the Hill radius (mass-ratio^(1/3)) but scales as epsilon^(1/4), where epsilon = m_p/m_0 is the planet-to-star mass ratio ("we confirm that measuring the orbital spacing in terms of Hill radii is not adapted and that the right spacing unit scales as epsilon^(1/4)"). The survival-time relation is written (Eq. 4) as log10(T_surv/P) = b' * Delta * (m_p/m_0)^(-1/4) - c' - log(m_p/m_0), with b' and c' positive numerical constants independent of the masses. The normalized spacing variable is delta_ij = 1 - alpha_ij ~= (2/3)(1 - nu_ij) (Eq. 43), combined for a three-planet system as delta = (delta_12 delta_23)/(delta_12 + delta_23) (Eq. 45). The resonance strength and the diffusion timescale carry the constant A = sqrt(38/pi) = 3.47 (Eq. 52) and the exponential spacing dependence exp(-2(p+q) delta) (Eq. 47). The underlying t_inst is a chaotic-diffusion timescale, not a single fitted line; the log-linear Eq. (4) is its practical form.
- **Primary citation:** Petit, Pichierri, Davies & Johansen 2020, "The path to instability in compact multi-planetary systems", Astronomy & Astrophysics 641, A176, DOI 10.1051/0004-6361/202038764, arXiv 2006.14903. Scaling statement in the abstract and Section 2.2; Eq. (4) survival relation; Eqs. (43), (45), (47), (52) for the spacing and diffusion machinery.
- **Confidence:** High on the epsilon^(1/4) scaling and the functional form. Medium on the exact numeric constants b' and c', which the paper calibrates against N-body integrations in its Section 6 comparison and which were not extracted to a number here.
- **Caveat:** The epsilon^(1/4) result is the headline correction to the older Hill-radius (mass-ratio^(1/3)) spacing. The specific b', c' still need to be read from the Section 6 tables/figures before coding.

### A3. Multi-planet Gyr stability-spacing fit (Obertas, Van Laerhoven & Tamayo 2017)

- **Label:** FORM (empirical log-linear fit with coefficients).
- **Extracted:** Eq. (7): log10(t_c / t_0) = b * Delta + c, where t_c is the close-encounter (instability) time, t_0 the innermost planet's initial orbital period, and Delta the spacing in mutual Hill radii. Two fits are reported: for 3.4 <= Delta < 8.4, b = 0.951 and c = -1.202; for the wider 3.4 <= Delta < 10, b = 1.086 and c = -1.881. Systems with Delta >= 8.6 survive at least 10^10 orbits. Large modulations sit on top of the linear trend near first- and second-order mean-motion resonances, where the survival time can move up or down by up to two orders of magnitude relative to the fit.
- **Primary citation:** Obertas, Van Laerhoven & Tamayo 2017, "The stability of tightly-packed, evenly-spaced systems of Earth-mass planets orbiting a Sun-like star", Icarus 293, 52-58, DOI 10.1016/j.icarus.2017.04.010, arXiv 1703.08426. Fit in Eq. (7).
- **Confidence:** High on the fit coefficients and the 10^10-orbit edge at Delta ~ 8.6.
- **Caveat:** The ruling's "~0.4 dex diffusion scatter" is not the number Obertas et al. report; their scatter is dominated by the resonant modulations (up to ~2 dex near MMRs), and the linear-fit residual away from resonances is smaller but not stated as a clean 0.4 dex. The ~0.4 dex intrinsic-diffusion figure should be taken from Smith & Lissauer 2009 (Icarus 201, 381) or Pu & Wu 2015 (ApJ 807, 44), which were grouped with Obertas in the ruling but were not fetched here.

### A4. Kelvin-Helmholtz contraction timescale for the critical-core race (Ikoma, Nakazawa & Emori 2000)

- **Label:** FORM plus exponents; exact primary coefficients not pinned.
- **Extracted:** Once the core reaches critical mass, the envelope contracts and gas is accreted on the Kelvin-Helmholtz timescale, which Ikoma et al. established is a strong function of both core mass and envelope grain opacity, of the form tau_KH proportional to (M_core)^(-p) (kappa)^(q), with the mass exponent p ~ 2.5 (they bracket 2 to 3) and an opacity dependence that is close to linear, q ~ 1. The form reproduced most often in the pebble-accretion literature is tau_KH ~ 10^8 yr (M_core / M_Earth)^(-2.5) (kappa / 1 cm^2 g^-1). Other reproductions quote the broader tau_KH ~ 10^c yr (M_core / M_Earth)^(-d) with 8 <= c <= 10 and 2 <= d <= 4, depending on the fit domain and the opacity assumption.
- **Primary citation:** Ikoma, Nakazawa & Emori 2000, "Formation of Giant Planets: Dependences on Core Accretion Rate and Grain Opacity", The Astrophysical Journal 537, 1013-1025, DOI 10.1086/309050.
- **Confidence:** High on the functional form and the strong M_core and kappa dependence. Medium on any single coefficient set, because the exact prefactor and exponent were read from later works that cite Ikoma et al., not from the (paywalled, non-parsing) original.
- **Caveat:** Different citing papers report a spread (prefactor 10^8 to 10^10 yr, mass exponent 2 to 4). To pin the exact figure the arc uses, read the equation in the ApJ 537 original (or its ADS scan). Flagged in the "Partially verified" section.

### A5. Pebble isolation mass and the pebble-versus-planetesimal branch (Lambrechts & Johansen 2012, 2014)

- **Label:** FORM (isolation mass, growth times) plus CONDITION (accretion-regime branch).
- **Extracted:**
  - Pebble isolation mass (Lambrechts, Johansen & Morbidelli 2014): M_iso ~= 20 M_Earth (H/r / 0.05)^3, that is, the core mass at which the core carves a pressure bump that halts the inward pebble flux and triggers the switch toward gas accretion.
  - Regime branch (Lambrechts & Johansen 2012): the transition mass between the drift (Bondi) regime and the Hill regime is M_t ~= 3e-3 (Delta/0.05)^3 (r/5 AU)^(3/4) M_Earth (Eq. 33), reached when the Bondi radius equals the Hill radius. Below M_t growth follows the drift-regime rate (Ṁ proportional to M_core^2, Eq. 31); above it the Hill-regime rate (Ṁ proportional to M_core^(2/3), Eq. 38).
  - Pebble versus planetesimal: pebble accretion draws from the full Hill sphere, whereas classical planetesimal accretion captures only a fraction ~ alpha^(1/2) r_H (with alpha ~ r_core/r_H ~ 1e-3), so pebble accretion shortens the growth time by a factor 30 to 1000 at 5 AU and 100 to 10000 at 50 AU. Hill-regime growth time (Eq. 44): Delta t_H ~= 4e4 yr (M_crit/10 M_Earth)^(1/3) (r/5 AU); drift-regime time (Eq. 42): Delta t_d ~= 8e6 yr (Delta/0.05)^3 (Sigma_p ratio)^(-1) (M_0/1e-5 M_Earth)^(-1) (r/5 AU)^2.
- **Primary citations:** Lambrechts & Johansen 2012, "Rapid growth of gas-giant cores by pebble accretion", Astronomy & Astrophysics 544, A32, DOI 10.1051/0004-6361/201219127, arXiv 1205.3030 (Eqs. 31, 33, 38, 42, 44). Lambrechts, Johansen & Morbidelli 2014, "Separating gas-giant and ice-giant planets by halting pebble accretion", Astronomy & Astrophysics 572, A35, DOI 10.1051/0004-6361/201423814, arXiv 1408.6094 (pebble isolation mass M_iso ~ 20 M_Earth (H/r/0.05)^3).
- **Confidence:** High on the isolation-mass form, the transition mass, and the growth-time scalings.
- **Caveat:** The "planetesimal-to-pebble flux/surface-density ratio" the ruling names is expressed in Lambrechts & Johansen 2012 as an accretion-efficiency contrast (full Hill sphere versus alpha^(1/2) r_H) and a growth-time contrast, rather than a single tabulated flux ratio. The branch is decided by whichever accretion rate dominates at the local pebble surface density.

### A6. Resonant-chain breaking fraction (Izidoro et al. 2017, 2021)

- **Label:** [M] VALUE (breaking fraction) plus CONDITION (post-gas-dispersal instability).
- **Extracted:** After the gas disc disperses, a large majority of migration-built resonant chains must go dynamically unstable to reproduce the observed Kepler period-ratio distribution of sub-4-Earth-radii planets. Izidoro et al. 2017 find a good match when roughly 90 to 95 percent of resonant chains break; the 2021 follow-up refines this to about 95 to 99 percent unstable (1 to 5 percent surviving in resonance). The instability spreads the initially compact chains through a phase of giant impacts, yielding the wider, near-uniform spacing seen in the data.
- **Primary citations:** Izidoro et al. 2017, "Breaking the chains: hot super-Earth systems from migration and disruption of compact resonant chains", MNRAS 470, 1750-1770, DOI 10.1093/mnras/stx1232, arXiv 1703.03634. Izidoro et al. 2021, "Formation of planetary systems by pebble accretion and migration: Hot super-Earth systems from breaking compact resonant chains", Astronomy & Astrophysics 650, A152, DOI 10.1051/0004-6361/201935336, arXiv 1902.08772.
- **Confidence:** High on ~90 to 95 percent (2017) and ~95 to 99 percent (2021), read from the abstracts and summary text of both primaries.
- **Caveat:** The precise best-fit percentage depends on the assumed system multiplicity and the collisional-fragmentation treatment; the 2021 fragmentation study nudges the figure toward the high end.

### A7. Wind-driven disk-evolution closure (Suzuki et al. 2016)

- **Label:** FORM (surface-density evolution as the alternative to alpha-viscous, for gate G).
- **Extracted:** The surface-density evolution combining viscous/turbulent transport, magnetic disc-wind angular-momentum removal, and wind mass loss is Eq. (10):
  d(Sigma)/dt - (1/r) d/dr { (2/(r Omega)) [ d/dr (r^2 Sigma alphabar_rphi c_s^2) + r^2 alphabar_phiz (rho c_s^2)_mid ] } + C_w (rho c_s)_mid = 0.
  The first bracketed term is the radial (r-phi) turbulent stress alphabar_rphi (the alpha-viscous analogue); the second is the wind-driven vertical (phi-z) stress alphabar_phiz that removes angular momentum from the surface; the last term is direct mass loss with dimensionless wind mass-flux C_w (Eq. 9), bounded by C_w = min(C_w,0, C_w,e) (Eq. 22). The wind torque is set either constant at alphabar_phiz = 1e-4 or density-dependent, alphabar_phiz = min(1e-5 (Sigma/Sigma_init)^(-0.66), 1) (Eq. 33). A signature outcome: with preserved vertical magnetic flux the inner-disc (< 1 to 10 AU) surface-density slope can become positive, opposite to the negative slope of a standard alpha disc, which halts or reverses pebble drift.
- **Primary citation:** Suzuki, Ogihara, Morbidelli, Crida & Guillot 2016, "Evolution of Protoplanetary Discs with Magnetically Driven Disc Winds", Astronomy & Astrophysics 596, A74, DOI 10.1051/0004-6361/201628955, arXiv 1609.00437. Master equation Eq. (10); wind parameters Eqs. (8), (9), (22), (33).
- **Confidence:** High on the master equation and the wind-torque and mass-loss parameterization.
- **Caveat:** This is a declared model-structure alternative (gate G), so it should sit in an explicit model band beside the alpha-viscous profile, not replace it silently. Bai 2016 (ApJ 821, 80) is a valid sibling primary if a second wind closure is wanted.

---

## Group B: magma-ocean and partition petrology (R-YOUNG-TEMPERATURE, texture rulings)

### B8. Lattice-strain partition-coefficient model (Blundy & Wood 1994)

- **Label:** FORM (partition-coefficient closure) plus its parameters.
- **Extracted:** The partition coefficient of an isovalent cation of radius r_i onto a crystal site is
  D_i = D_0 exp{ -4 pi E N_A [ (r_0/2)(r_i - r_0)^2 + (1/3)(r_i - r_0)^3 ] / (R T) },
  where D_0 is the strain-free partition coefficient for a fictive cation of the ideal site radius r_0, E is the effective Young's modulus of the site (in GPa/Pa), r_0 is the optimum (strain-free) site radius, r_i is the substituent cation radius, N_A is Avogadro's number, R is the gas constant, and T is temperature. The model builds on the elastic strain-energy treatment of Brice (1975). A given isovalent series traces a near-parabola in log D versus r_i peaking at r_0 with curvature set by E.
- **Primary citation:** Blundy & Wood 1994, "Prediction of crystal-melt partition coefficients from elastic moduli", Nature 372, 452-454, DOI 10.1038/372452a0. The lattice-strain equation is the paper's central result.
- **Confidence:** High on the equation form and the parameter meanings. The written form was confirmed against multiple faithful reproductions that attribute it to Blundy & Wood 1994; the sign, the (r_0/2) and (1/3) coefficients, and the 4 pi E N_A / RT grouping are consistent across them.
- **Caveat:** E here is the site-specific effective Young's modulus (not the bulk modulus); for a real mineral it is fit per site and per charge, and D_0, r_0, E are all functions of P, T, and composition. The arc must read them per crystal-site as data, not hardcode one triple.

### B9. Lithophile-to-chalcophile switch for U and Th (Wohlers & Wood 2015; Wood & Kiseeva 2015; Wohlers & Wood 2017)

- **Label:** [M] VALUES plus CONDITION (redox/FeO branch).
- **Extracted:** Under reduced, sulfur-rich conditions, when the silicate melt's FeO content falls below about 1 wt%, elements that are normally lithophile (U, Th, the rare earths) turn chalcophile and partition strongly into a sulfide/metal liquid, while Cu partitioning drops. Adding such a reduced sulfur-rich body to the accreting Earth can, at the experimental 2100 C, deliver up to ~10 ppb U to the core, and with the accompanying ~21 ppb Th, supply on the order of ~3 TW of power to the geodynamo. Wood & Kiseeva 2015 give the underlying partitioning mechanism (how the sulfide capacity for lithophile elements rises as FeO drops); Wohlers & Wood 2017 extend the U, Th, REE sulfide-liquid partition values for reduced S-rich bodies.
- **Primary citations:** Wohlers & Wood 2015, "A Mercury-like component of early Earth yields uranium in the core and high mantle 142Nd", Nature 520, 337-340, DOI 10.1038/nature14350 (the ~10 ppb U, ~21 ppb Th, ~3 TW figures, FeO < ~1 wt% threshold, 2100 C). Wood & Kiseeva 2015, "Trace element partitioning into sulfide: How lithophile elements become chalcophile and vice versa", American Mineralogist 100, 2371-2379, DOI 10.2138/am-2015-5358 (mechanism). Wohlers & Wood 2017, "Uranium, thorium and REE partitioning into sulfide liquids: Implications for reduced S-rich bodies", Geochimica et Cosmochimica Acta 205, 226-244, DOI 10.1016/j.gca.2017.01.050 (partition values).
- **Confidence:** High on the FeO < ~1 wt% condition and the ~10 ppb U / ~21 ppb Th / ~3 TW figures.
- **Caveat:** The 3 TW is contingent on the 2100 C experiments and on the mass and redox state of the accreted S-rich body; it is an upper-end estimate for a specific scenario, not a universal U-in-core budget. Treat the FeO threshold as the branch key and the partition values as data per redox state.

### B10. Rheological critical melt fraction (Abe 1993; Solomatov 2015)

- **Label:** [M] VALUE plus FORM (suspension rheology).
- **Extracted:** The rheological critical melt fraction phi_c, the melt fraction at which a cooling magma ocean's rheology switches from a liquid-like suspension to a solid-like crystal framework, is about 0.4 (crystal fraction ~0.6), with experimental estimates spanning ~0.3 to 0.4. Crossing phi_c the effective viscosity jumps by many orders of magnitude and convective velocity and melt-crystal separation change abruptly. The suspension side is described by an Einstein-Roscoe type relation, effective viscosity increasing steeply as crystal fraction approaches the packing limit, so phi_c depends on crystal shape and polydispersity.
- **Primary citations:** Abe 1993, "Thermal evolution and chemical differentiation of the terrestrial magma ocean", in Evolution of the Earth and Planets, Geophysical Monograph 74 (IUGG Vol. 14), 41-54, AGU, DOI 10.1029/GM074p0041. Solomatov 2015, "Magma Oceans and Primordial Mantle Differentiation", Treatise on Geophysics (2nd ed.), Vol. 9, Elsevier (the RCMF and suspension-rheology synthesis).
- **Confidence:** High on phi_c ~ 0.4 (range 0.3 to 0.4). Medium on the exact suspension-viscosity coefficient, which is the Einstein-Roscoe/Krieger-Dougherty family with parameters that depend on crystal habit.
- **Caveat:** phi_c is not a universal constant; it is a rheological threshold that shifts with crystal shape and size distribution, so it should be a data-driven parameter (with 0.4 the default and the shape dependence exposed) rather than a hardcoded 0.4.

### B11. Type-I / type-II magma-ocean split and the runaway-greenhouse limit (Hamano, Abe & Genda 2013)

- **Label:** FORM (critical-distance branch) plus [M] VALUE (radiation limit).
- **Extracted:** Terrestrial planets split at a critical orbital distance. Type I (formed beyond the critical distance) solidify quickly, within several million years, and retain most of their water as the earliest oceans. Type II (formed inside the critical distance) sustain a magma ocean for as long as ~100 Myr and are desiccated by hydrodynamic escape during the slow solidification. The split is set by whether the net absorbed stellar flux keeps the surface above the point where the steam atmosphere's outgoing radiation saturates at the runaway-greenhouse (Nakajima-type) radiation limit, which for a water/steam atmosphere is ~280 to 310 W/m^2. Inside the critical distance the absorbed flux exceeds this ceiling, so the surface cannot radiate fast enough to solidify promptly.
- **Primary citation:** Hamano, Abe & Genda 2013, "Emergence of two types of terrestrial planet on solidification of magma ocean", Nature 497, 607-610, DOI 10.1038/nature12163.
- **Confidence:** High on the type-I/type-II distinction, the ~few-Myr versus ~100-Myr lifetimes, and the ~280 to 310 W/m^2 steam radiation limit.
- **Caveat:** Hamano et al. define the critical distance through a radiation-balance argument rather than a single closed-form flux equation reproduced here; the exact critical-distance expression (as a function of stellar luminosity and the radiation limit) should be taken from the paper's own equations before coding. Corroborated by Lichtenberg et al. 2021 (item B13), which measures 282.5 W/m^2 for the H2O tropospheric limit.

### B12. Secular mantle potential temperature (Herzberg, Condie & Korenaga 2010)

- **Label:** [M] VALUES (T_p trajectory) plus FORM (T_p from MgO).
- **Extracted:** Non-arc basalts and komatiites give mantle potential temperatures rising from ~1350 C today to a maximum of ~1500 to 1600 C at 2.5 to 3.0 Ga, with the mantle warming in Hadean-early-Archean time (internal heating exceeding surface loss) and cooling from ~2.5 to 3.0 Ga to the present. The petrological thermometer converts primary-magma MgO content (wt%) to potential temperature; the regression in this lineage is T_P (C) = 1463 + 12.74 * MgO - 2924 / MgO, invertible as MgO ~= 58 - 0.0977 * T_P + ... . Archean Al-undepleted komatiites with 27 to 30 wt% MgO imply eruption temperatures ~1550 to 1600 C and T_P up to and beyond ~1700 C.
- **Primary citations:** Herzberg, Condie & Korenaga 2010, "Thermal history of the Earth and its petrological expression", Earth and Planetary Science Letters 292, 79-88, DOI 10.1016/j.epsl.2010.01.022 (the T_p(t) trajectory and the ~1350 C present / ~1500 to 1600 C peak). The T_P(MgO) regression traces to Herzberg et al. 2007, "Temperatures in ambient mantle and plumes", Geochemistry Geophysics Geosystems 8, Q02006, DOI 10.1029/2006GC001390, and Herzberg & Asimow 2008 (PRIMELT2), G-cubed 9, Q09001, DOI 10.1029/2008GC002057.
- **Confidence:** High on the T_p(t) trajectory values. Medium on the exact T_P(MgO) coefficients, which were read from reproductions attributing them to Herzberg et al. 2007 / Herzberg & Asimow 2008 rather than from the primary equation itself.
- **Caveat:** The T_P(MgO) equation applies to anhydrous peridotite-derived primary magmas (the PRIMELT calibration domain); it is a thermometer for that petrology, so it should be read as the calibration for peridotite melting, not applied blindly to other source compositions.

### B13. Radiation ceiling and magma-ocean lifetime for non-water blankets (Lichtenberg et al. 2021)

- **Label:** FORM/VALUE (per-volatile radiation behaviour and solidification time).
- **Extracted:** For a solidifying Earth-sized rocky planet with a single-volatile blanket, the H2O case sets the reference: a near-constant tropospheric outgoing radiation limit of 282.5 W/m^2 across ~300 to 2000 K surface temperature (at 260 bar H2O). The non-water end-members differ strongly. H2 depresses the outgoing flux well below the H2O limit and keeps the interior partially molten far longer (magma ocean not fully solidified within a 100 Myr run). CO2 and CH4 are intermediate. CO, O2, and N2 impose only a weak greenhouse and let the planet radiate freely. Time to the rheological transition (melt fraction ~0.4): H2 ~10^6 yr; H2O, CO2, CH4 ~10^4 yr; CO, O2, N2 ~10^3 yr. Full solidification is ~1 Myr for every case except H2, which remains molten beyond 100 Myr. This gives the per-world ceiling: the runaway-greenhouse-style saturation is a property of a condensable absorber (H2O), and a non-condensable blanket sets the lifetime by its opacity rather than by a single saturated flux.
- **Primary citation:** Lichtenberg, Bower, Hammond, Boukrouche, Sanan, Tsai & Pierrehumbert 2021, "Vertically Resolved Magma Ocean-Protoatmosphere Evolution: H2, H2O, CO2, CH4, CO, O2, and N2 as Primary Absorbers", Journal of Geophysical Research: Planets 126, e2020JE006711, DOI 10.1029/2020JE006711, arXiv 2101.10991. H2O radiation limit and per-volatile solidification from their Figure 5 and surrounding text.
- **Confidence:** High on the H2O 282.5 W/m^2 limit and the per-volatile ordering and solidification timescales.
- **Caveat:** These are idealized clear-sky, single-volatile end-members for an Earth-mass planet at a fixed volatile inventory; clouds, mixed atmospheres, and different masses shift the numbers. There is no single "radiation limit W/m^2" for the non-condensable blankets in the way there is for water, because they do not saturate; the arc should read the per-volatile OLR behaviour, not seek one ceiling number.

---

## Group C: heat-producer abundance and specific power column

### C14. U, Th, K abundances and per-isotope specific heat production (Lodders 2003; Ruedas 2017)

- **Label:** [M] VALUES (abundances and specific heat production).
- **Extracted:**
  - Specific radiogenic heat production (Ruedas 2017, Table 2), the strongest primary for the power side:
    - U-238: 9.4946e-5 W/kg (half-life 4468 Myr)
    - U-235: 5.68402e-4 W/kg (half-life 704 Myr)
    - Th-232: 2.6368e-5 W/kg (half-life 14000 Myr)
    - K-40: 2.8761e-5 W/kg (half-life 1248 Myr)
    - Natural U (present isotopic mix): 9.8314e-5 W/kg
    - Natural Th (232Th only): 2.6368e-5 W/kg
    - Natural K (with 40K fraction): 3.4302e-9 W/kg
    - Also given: 26Al 0.3583 W/kg (0.717 Myr), 60Fe 3.6579e-2 W/kg (2.62 Myr).
  - CI carbonaceous chondrite abundances (Lodders solar-abundance lineage): U ~7.8 ppb, Th ~29.3 ppb, K ~552 ppm by mass.
  - Bulk Silicate Earth reference (McDonough & Sun 1995): U ~20.6 ppb, Th ~77.6 ppb, K ~260 ppm, giving Th/U ~3.8 and K/U ~1.3e4; present-day BSE radiogenic power ~20 TW split roughly U 40 percent, Th 40 percent, K 20 percent.
- **Primary citations:** Ruedas 2017, "Radioactive heat production of six geologically important nuclides", Geochemistry Geophysics Geosystems 18, 3530-3541, DOI 10.1002/2017GC006997, arXiv 1710.06721 (specific heat production, Table 2). Lodders 2003, "Solar System Abundances and Condensation Temperatures of the Elements", The Astrophysical Journal 591, 1220-1247, DOI 10.1086/375492, updated in Lodders 2021, for the CI-chondrite U, Th, K abundances (AGSS09 stops at Mo, so this is the source that supplies U and Th). McDonough & Sun 1995, "The composition of the Earth", Chemical Geology 120, 223-253, DOI 10.1016/0009-2541(94)00140-4, for the BSE reference.
- **Confidence:** High on the Ruedas 2017 specific-heat-production values (read from the primary's Table 2). High on the CI and BSE abundance figures, which were read from a canonical 2025 compilation (McDonough, "Earth's composition", arXiv 2505.02641, Table 3) that attributes the CI values to the Lodders lineage and Palme & Zipfel 2022 and the BSE values to McDonough & Sun 1995.
- **Caveat:** The CI abundances were read from the compilation, not directly from a Lodders 2003 table image; the exact Lodders 2003 CI numbers (and their solar-photosphere log-scale counterparts) should be confirmed against that paper's tables before they are the sole source. Ruedas 2017 flags K-40 as the least certain nuclide (half-life and branching), so the K specific power carries a slightly larger uncertainty than U and Th.

---

## Partially verified: what still needs a further pin

These items have a verified primary and a verified form or value, but a specific coefficient was read from a reproduction rather than from the primary equation, or a number the ruling assumed traces to a different paper. None is a fabricated value.

1. Item A1: the "~10 mutual Hill radii" oligarch separation is Kokubo & Ida 1998 (Icarus 131, 171-178), not Goldreich, Lithwick & Sari 2004. Read the coefficient b (~5 to 10) from Kokubo & Ida before coding.
2. Item A2: the numeric constants b' and c' in Petit et al. 2020 Eq. (4) were not extracted; they are calibrated in that paper's Section 6. The epsilon^(1/4) scaling and the functional form are verified.
3. Item A3: the "~0.4 dex" intrinsic scatter is not Obertas et al.'s reported number; take it from Smith & Lissauer 2009 or Pu & Wu 2015.
4. Item A4: the exact Ikoma, Nakazawa & Emori 2000 prefactor and exponents (form verified as tau_KH proportional to M_core^(-2.5) kappa^(1); citing works span prefactor 10^8 to 10^10 yr and mass exponent 2 to 4). Read the equation from ApJ 537, 1013 (or its ADS scan) to fix the arc's exact figure.
5. Item B12: the T_P(MgO) regression coefficients (1463, 12.74, 2924) were read from reproductions attributing them to Herzberg et al. 2007 / Herzberg & Asimow 2008; confirm against those primaries.
6. Item C14: the CI-chondrite U, Th, K values were read from a compilation; confirm the exact Lodders 2003/2021 CI table entries.

No item on the fetch list is fully NOT-VERIFIED; every one resolved to a primary source with the needed value, form, or condition.
