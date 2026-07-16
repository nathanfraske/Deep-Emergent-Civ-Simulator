# Capstone literature-fetch values, batch 2

This file records the primary-source fetch for the second mechanical pull list behind the resolved capstone arcs: the carbon-condensation temperature formulae, the JANAF and Barin-class carbide/cementite thermochemistry, the Miedema carbide extension, the Murray and Dermott secular matrix definitions, the Reimers wind law with the helium-flash core-mass anchor, and the Petit et al. 2020 overlap-spacing verification. For each item the label is one of [M] VALUE (a number with units), FORM (an equation or closure with its own coefficients), DATA ROW (thermochemistry), DEFINITION, or VERIFICATION. Each carries the extracted content, the primary citation (author, year, journal, arXiv or DOI, and the specific equation, table, or figure), a confidence, and any caveat.

Discipline note: arXiv and journal PDFs were downloaded and read with pdftotext where the fetch path returned compressed streams; equations and data below are read from that extracted text or from the NIST-JANAF WebBook condensed-phase entries. Where a value is quoted by a later peer-reviewed paper that reproduces a paywalled primary, that reproduction is stated as such while the citation points to the primary. Nothing here is a fabricated number. Where a specific coefficient, table row, or equation number could not be pinned to the primary, it is flagged in the closing "Partially verified / NOT-VERIFIED" section rather than guessed.

---

## Item 1. Carbon-condensate condensation-temperature formulae (Sharp and Wasserburg 1995)

- **Label:** FORM (analytic condensation-temperature relations for graphite, TiC, SiC as functions of C/O ratio, total pressure, and metallicity).
- **Primary citation (original method):** Sharp, C. M., and Wasserburg, G. J. 1995, "Molecular equilibria and condensation temperatures in carbon-rich gases", Geochimica et Cosmochimica Acta 59, 1633. Confirmed as GeCoA 59, 1633 (1995) from the reference list of the reproduction below.
- **Primary citation (usable analytic reproduction and extension):** Adams, G. M., and Lodders, K. 2025, "Effects of Metallicity on Graphite, TiC, and SiC Condensation in Carbon Stars", The Astrophysical Journal, DOI 10.3847/1538-4357/adc40f, arXiv:2411.11832. Governing equilibria in Section 2, analytic derivation Eqs. (20) to (32), regression fits Table 2, cross-C/O relation Eq. (39). The paper states its calculations use the CONDOR code and the same governing gas species Sharp and Wasserburg identified.

- **Governing gas species (confirmed for both papers):** graphite condensation is set by monatomic C(g), C2H2, C3, and (per Sharp and Wasserburg) C2H, with H and H2 controlling the hydrogenation balance; the carbides add Ti and Si. Adams and Lodders note the difference from Sharp and Wasserburg and Chigai et al. (1999) is that C2H is less stable than those earlier works assumed.

- **General analytic form (Adams and Lodders Eq. 24), graphite from monatomic carbon.** Writing the equilibrium constant as a linear fit log(K) = A + B/T, the initial condensation temperature obeys

  1/T = ( log(1 + eps_He) - log(eps'_C) - A ) / B  -  (1/B) log(P_Tot)  -  (1/B) [M/H]

  where eps_He is the He/H abundance ratio, eps'_C the effective (available) carbon abundance ratio, P_Tot the total pressure in bar, [M/H] the metallicity, and A, B the linear-fit constants of the relevant equilibrium constant. Graphite is present where its activity a_Gr = 1.

- **Region-specific analytic solutions and regression fits at C/O = 1.2** (temperature in K, P_Tot in bar, form 10^4 / T = A + B log(P_Tot) - C [M/H]):
  - R1 (second most abundant C gas is monatomic C or C3, most H is atomic): analytic Eq. (25) 10^4/T = 3.2512 - 0.2671 log(P_Tot) - 0.2671 [M/H]; regression fit Eq. (26) 10^4/T = 3.7438 - 0.2267 log(P_Tot) - 0.2604 [M/H] (max uncertainty +/- 15 K).
  - R2 (reaction C2H2(g) = 2 C(Gr) + 2 H(g), atomic H): fit Eq. (29) 10^4/T = 8.2822 + 0.3704 log(P_Tot) - 1.5004 [M/H] (+/- 15 K). Pressure dependence flips sign here relative to R1 because P appears in the numerator of K.
  - R3 (reaction C2H2(g) = 2 C(Gr) + H2(g), molecular H2): analytic Eq. (32) 10^4/T = 5.7335 - 0.8685 [M/H]; fit 10^4/T = 5.8140 + 0.0013 log(P_Tot) - 0.8541 [M/H] (+/- 1 K); the pressure dependence nearly cancels because the mole numbers balance across the reaction, so graphite in R3 depends almost solely on metallicity.
  - Table 2 carbide fits (same 10^4/T = A + B log(P_Tot) - C [M/H] form): TiC (before graphite) A = 4.5513, B = -0.2517, C = -0.3771 (+/- 5 K); TiC (after graphite) A = 4.3092, B = -0.2985, C = -0.2895 (+/- 5 K); SiC (before graphite) A = 4.8570, B = -0.2892, C = -0.4679 (+/- 10 K); SiC (after graphite) A = 4.6039, B = -0.3672, C = -0.3614 (+/- 10 K).
  - Sequence-boundary pressures (C/O = 1.2): TiC-graphite equal at log(P_Tot) = 1.8854 [M/H] - 4.9909 (Eq. 8); SiC-graphite equal at log(P_Tot) = 1.3370 [M/H] - 3.2839 (Eq. 9).
  - C/O generalization (Eq. 39, for 1e-5 < eps'_C < 1e-3, valid to +/- 0.1 in log P): log(P_R3) = 2.38827 - 0.62099 [M/H] + 2.52033 log(eps'_C) - 0.60591 log(C/O). Carbides are near C/O-independent when they condense after graphite (graphite activity pinned at unity); graphite is C/O-sensitive because carbon abundance and gas speciation both shift with C/O.

- **Confidence:** High on the analytic form, the governing species, and the Adams and Lodders 2025 coefficients (read directly from the extracted text). High on the Sharp and Wasserburg 1995 citation.
- **Caveat (input audit):** the exact numeric coefficients of Sharp and Wasserburg's own 1995 formulae are behind the GeCoA paywall and were not read from that paper. The usable analytic relations above are the Adams and Lodders 2025 reproduction and extension of the same method and species. Adams and Lodders differ deliberately from Sharp and Wasserburg on C2H stability, so the two coefficient sets are close but not identical. For the build, cite Adams and Lodders 2025 for the coefficients that are coded and Sharp and Wasserburg 1995 for the original derivation; if the original S&W coefficients are needed verbatim, the GeCoA paper must be pulled.

---

## Item 2. NIST-JANAF carbide and graphite thermochemistry (SiC, TiC, C graphite)

- **Label:** DATA ROWS (standard formation enthalpy, entropy, and Shomate heat-capacity coefficients, sufficient to build g = mu_standard(T)/RT across roughly 1000 to 2000 K).
- **Primary citation:** Chase, M. W., Jr. 1998, NIST-JANAF Thermochemical Tables, Fourth Edition, J. Phys. Chem. Ref. Data, Monograph 9. Read through the NIST Chemistry WebBook condensed-phase entries. Heat capacity uses the Shomate form Cp = A + B t + C t^2 + D t^3 + E/t^2 with t = T(K)/1000; the Shomate H parameter equals delta-f-H(298.15 K) in kJ/mol.

- **SiC (alpha, hexagonal), CAS 409-21-2:** delta-f-H(298.15 K) = -71.546 kJ/mol. Standard entropy S(298.15 K) computed from the Shomate coefficients is approximately 16.5 J/(mol K). Shomate coefficients:
  - 298 to 1000 K: A = 20.55859, B = 64.57962, C = -52.98827, D = 16.95813, E = -0.781847, F = -82.73693, G = 19.90848, H = -71.54598.
  - 1000 to 4000 K: A = 46.90222, B = 5.845968, C = -1.085410, D = 0.093021, E = -3.448876, F = -95.46716, G = 56.97520, H = -71.54598.

- **SiC (beta, cubic 3C):** JANAF tabulates the alpha (hexagonal) phase. The cubic 3C beta polymorph is metastable and differs by only a small transition enthalpy; the JANAF-derived alpha-to-beta transition is delta-tr-H(298) = -1.7 +/- 8.9 kJ/mol (that is, the two polymorphs are equal within uncertainty). A third-law value delta-f-H(298, alpha-SiC) = -74.4 kJ/mol is also reported. Primary for the modification-resolved Gibbs energies: Kleykamp, H. 1998, "Gibbs energy of formation of SiC: A contribution to the thermodynamic stability of the modifications", Berichte der Bunsengesellschaft fuer physikalische Chemie 102, 1231.

- **TiC (cubic), CAS 12070-08-5:** delta-f-H(298.15 K) = -184.096 kJ/mol; S(298.15 K) = 24.21 J/(mol K). Shomate coefficients:
  - 298 to 1000 K: A = 5.554330, B = 149.8650, C = -171.0690, D = 67.13540, E = -0.269136, F = -191.9420, G = -8.253841, H = -184.0960.
  - 1000 to 3290 K: A = 43.27000, B = 4.533360, C = 2.021400, D = -0.203272, E = 1.701881, F = -195.8940, G = 73.95100, H = -184.0960.
  - liquid, 3290 to 4500 K: A = 62.76230, B = -0.001173, C = 0.000225, D = -0.000015, E = -0.003425, F = -146.8340, G = 95.72310, H = -108.3340; delta-f-H(liquid) = -108.334 kJ/mol.

- **Graphite C(cr):** the carbon reference state, so delta-f-H = 0 by definition at all temperatures and mu_standard(graphite, T) fixes the carbon zero. Standard entropy S(298.15 K) = 5.74 J/(mol K) (JANAF reference value; the WebBook also lists an averaged 5.6 +/- 0.5 J/(mol K) from Markelov, Volga and Buchnev 1973, Zhur. Fiz. Khim. 47, 1824). Cp(298.15) is about 8.5 J/(mol K).

- **Confidence:** High for SiC(alpha) and TiC (full Shomate rows read directly from the JANAF WebBook entries) and for graphite as the zero reference. Medium for the SiC beta-versus-alpha split, which rests on the small, uncertainty-dominated transition enthalpy.
- **Caveat:** the NIST WebBook returns the Shomate fits and the 298 K anchors, not the full per-temperature delta-f-G(T) table row by row. The build computes delta-f-G(T) = delta-f-H(T) - T [S(T) - sum S(elements, T)] from these coefficients plus the Si, Ti, and C element Shomate rows, which reproduces the JANAF Gibbs-energy-of-formation column across 1000 to 2000 K. If the tabulated delta-f-G(T) values are wanted verbatim, the Monograph 9 tables (or the WebBook JANAF table view, which was blocked here by a 403) are needed. The beta-cubic SiC polymorph should be taken from Kleykamp 1998 rather than JANAF.

---

## Item 3. Cementite Fe3C thermochemistry (Barin-class, the metastable no-JANAF case)

- **Label:** DATA ROW (Gibbs, enthalpy, entropy of the metastable carbide with no NIST-JANAF entry).
- **Primary citation (modern re-evaluation):** Hallstedt, B., Djurovic, D., von Appen, J., Dronskowski, R., Dick, A., Koermann, F., Hickel, T., and Neugebauer, J. 2010, "Thermodynamic properties of cementite (Fe3C)", Calphad 34(1), 129-133, DOI 10.1016/j.calphad.2010.01.004. Gibbs energy functions valid from 0 K upward.
- **Primary citation (compilation):** Barin, I., Thermochemical Data of Pure Substances (VCH, 3rd ed. 1995), the Fe3C rows.

- **Extracted values:** enthalpy of formation of Fe3C from Fe(bcc) plus graphite: delta-f-H(298.15 K) = +27.0 kJ/mol and delta-f-H(0 K) = +23.5 kJ/mol (Hallstedt et al. 2010 evaluation; the positive sign marks cementite as metastable with respect to iron plus graphite). The DFT formation energy at 0 K in the same paper is about +8 kJ/mol, and solution calorimetry gives about +18.8 kJ/mol at 298.15 K, bracketing the evaluated value. A Barin-derived compilation gives delta-f-H(298) = 25.104 kJ/mol, S(298.15 K) = 104.6 J/(mol K), and delta-f-G(298) = 20.083 kJ/mol.
- **Confidence:** High on the Hallstedt et al. 2010 citation and the +27.0 kJ/mol (298 K) and +23.5 kJ/mol (0 K) evaluated formation enthalpies. Medium on the exact Barin row (25.104 kJ/mol, 104.6 J/(mol K), 20.083 kJ/mol), which was read from a secondary compilation quoting Barin rather than from the Barin volume directly.
- **Caveat:** the spread across sources (+18.8 to +27.0 kJ/mol at 298 K) is real and reflects the metastability, not a fetch error. Use the Hallstedt et al. 2010 Gibbs energy function for the temperature-dependent g(T); the exact Barin S(298) and Cp(T) row should be confirmed against the Barin volume before coding.

---

## Item 4. Miedema-model extension to carbides (Niessen and de Boer; de Boer et al. Cohesion in Metals)

- **Label:** FORM plus parameters (the semi-empirical formation-enthalpy model that covers metal carbides, nitrides, and silicides without measured rows).
- **Primary citations:** Niessen, A. K., and de Boer, F. R. 1981, "The enthalpy of formation of solid borides, carbides, nitrides, silicides and phosphides of transition and noble metals", Journal of the Less-Common Metals 82, 75-80. Niessen, de Boer, Boom, de Chatel, Mattens, and Miedema 1983, "Model predictions for the enthalpy of formation of transition metal alloys II", Calphad 7, 1-70. de Boer, F. R., Boom, R., Mattens, W. C. M., Miedema, A. R., and Niessen, A. K. 1988, Cohesion in Metals: Transition Metal Alloys (North-Holland), the tabulated element parameter set.

- **Model form (interfacial / chemical enthalpy of A dissolved in B, macroscopic-atom model):**

  delta-H(A in B) = X_A V_A^(2/3) f_B^A [ -P (delta phi*)^2 + Q (delta n_ws^(1/3))^2 - R ] / [ (n_ws,A)^(-1/3) + (n_ws,B)^(-1/3) ]

  where phi* is the electronegativity parameter (in volts), n_ws the electron density at the Wigner-Seitz cell boundary (density units), V the molar volume, X the surface concentration, and f_B^A the degree to which an A atom is surrounded by B. For an ordered compound the composition prefactor f(c) multiplies the whole interfacial term.

- **Constants:** P depends on the combination of the two elements: P = 14.2 for two transition metals, 10.7 for two non-transition metals, and 12.35 for a transition plus a non-transition metal (units such that delta-H comes out in kJ/mol with V in cm^3 and n_ws in density units). Q = 9.4 P. R is the hybridization term, nonzero only when a transition metal is paired with a p-block or polyvalent partner, and it is set by R/P values that grow with the partner's p-electron count (for the chalcogen group the extension work derives R/P near 1.86 to 2.45, illustrating the magnitude; the carbide, nitride, boride, silicide R/P values are tabulated in de Boer et al. 1988).

- **Carbide / nitride / silicide extension (the key point for the metalloid rows):** Si, Ge, C, and N are treated as macroscopic atoms with their own phi* and n_ws, plus an additional positive enthalpy required to convert the element from its standard state into a hypothetical metallic state before it enters the compound. Niessen and de Boer give these metallization enthalpies as 8, 6, 24, and 57 kcal per gram-atom for Si, Ge, C, and N respectively (24 kcal/g-atom for carbon is about 100 kJ/mol for graphite going to metallic carbon). This positive term is added to the Miedema chemical enthalpy to yield the carbide/nitride/silicide formation enthalpy.

- **Confidence:** High on the model form and on P, Q = 9.4P, and the four metallization enthalpies (8, 6, 24, 57 kcal/g-atom). Medium on the exact per-carbide R/P values.
- **Caveat:** the per-element phi*, n_ws, molar-volume, and R/P tables for C, N, Si, and the transition metals (Ti, Fe) live in the de Boer et al. 1988 book tables, which are not web-fetchable. The R/P illustrative range quoted here is from a published chalcogen-group extension of the same model, given to fix the magnitude, not as the carbide value. To code Ti-C or Fe-C formation enthalpies, pull the element parameter rows from Cohesion in Metals (1988) or the Calphad 7 (1983) appendix.

---

## Item 5. Laplace-Lagrange secular matrix definitions (Murray and Dermott 1999, chapter 7)

- **Label:** DEFINITIONS (the A eccentricity/pericenter matrix and the B inclination/node matrix elements, with sign conventions).
- **Primary citation:** Murray, C. D., and Dermott, S. F. 1999, Solar System Dynamics (Cambridge University Press), Section 7.7 (secular perturbation theory for N planets). The A-matrix form below is reproduced verbatim, with attribution to Murray and Dermott 1999, in Camargo, B. C. B., Winter, O. C., and Foryta, D. W. 2018, "Comparison between Laplace-Lagrange Secular Theory and Numerical Simulation", arXiv:1806.03122, Eqs. (1) to (3); the B-matrix form is the standard companion result.

- **Second-order secular disturbing function** (Camargo et al. Eq. 1, after Murray and Dermott 1999): R_j = n_j a_j^2 [ (1/2) A_jj e_j^2 + sum_{k != j} A_jk e_j e_k cos(pomega_j - pomega_k) ], with the eccentricity variables h_j = e_j sin(pomega_j), k_j = e_j cos(pomega_j).

- **A matrix (eccentricity and longitude of pericenter):**

  A_jj = + (n_j / 4) sum_{k = 1, k != j}^{N} [ m_k / (m_c + m_j) ] alpha_jk alphabar_jk b_{3/2}^{(1)}(alpha_jk)

  A_jk = - (n_j / 4) [ m_k / (m_c + m_j) ] alpha_jk alphabar_jk b_{3/2}^{(2)}(alpha_jk)    (j != k)

- **B matrix (inclination and longitude of ascending node):**

  B_jj = - (n_j / 4) sum_{k = 1, k != j}^{N} [ m_k / (m_c + m_j) ] alpha_jk alphabar_jk b_{3/2}^{(1)}(alpha_jk)

  B_jk = + (n_j / 4) [ m_k / (m_c + m_j) ] alpha_jk alphabar_jk b_{3/2}^{(1)}(alpha_jk)    (j != k)

- **Sign conventions verbatim (the classic hand-built-LL defect):** the diagonal A_jj is positive and uses the first Laplace coefficient b_{3/2}^{(1)}; the off-diagonal A_jk is negative and uses the second Laplace coefficient b_{3/2}^{(2)}. The diagonal B_jj is negative and the off-diagonal B_jk is positive, and both B elements use b_{3/2}^{(1)}. So A_jj = -B_jj (same magnitude, opposite sign), while the off-diagonals differ in both sign and in which Laplace coefficient enters (b^{(2)} for A_jk, b^{(1)} for B_jk). Here m_c is the central mass, n_j the mean motion of planet j, and the Laplace coefficient is b_s^{(m)}(alpha) = (1/pi) integral_0^{2pi} cos(m psi) [1 - 2 alpha cos psi + alpha^2]^{-s} d psi.

- **alpha and alphabar convention (Camargo et al., stated verbatim, generalized):** for the pair (j, k), if planet j is the inner body being perturbed by an external planet k (a_j < a_k), then alpha_jk = a_j / a_k and alphabar_jk = a_j / a_k (that is, alphabar_jk = alpha_jk); if planet j is the outer body perturbed by an internal planet k (a_j > a_k), then alpha_jk = a_k / a_j and alphabar_jk = 1. In both cases the Laplace coefficient takes the argument alpha_jk which is less than 1.

- **Confidence:** High on the A_jj and A_jk forms and the alpha/alphabar convention (read verbatim from Camargo et al. 2018, which cites Murray and Dermott 1999). High on the B_jj and B_jk forms and the sign pattern, which is the standard companion result and matches an independent reproduction.
- **Caveat:** the exact Murray and Dermott equation numbers (commonly cited as 7.132 to 7.135) could not be confirmed from a fetchable copy of the book; cite as Solar System Dynamics 1999, Section 7.7, and confirm the equation numbers against the printed text. The verbatim expressions themselves are confirmed by two independent sources.

---

## Item 6. Reimers wind law and the helium-flash core-mass anchor

- **Label:** FORM (mass-loss parameterization) plus [M] VALUE (helium-ignition core mass).
- **Reimers form, primary citation:** Reimers, D. 1975, Memoires de la Societe Royale des Sciences de Liege, 6e serie, 8, 369. The law is Mdot = eta L* R* / M* with L*, R*, M* in solar units and eta a dimensionless fitting parameter, confirmed verbatim in Schroeder, K.-P., and Cuntz, M. 2005, "A New Version of Reimers' Law of Mass Loss Based on a Physical Approach", arXiv:astro-ph/0507598 (ApJ 630, L73), which cites Reimers 1975, 1977 and states the form and the solar-unit convention.
- **Reimers constant:** in the dimensionless-eta convention the standard scaling is Mdot [M_sun/yr] = 4e-13 eta (L/L_sun)(R/R_sun)/(M/M_sun) with eta of order 0.4 to 0.5 (the 4e-13 prefactor is the value fixed by Kudritzki, R. P., and Reimers, D. 1978, A&A 70, 227). Equivalently, some works absorb the constant into eta_R with units, so that Mdot = eta_R L R / M with eta_R of order (0.4 to 3) x 1e-13 M_sun/yr; Schroeder and Cuntz work in this second convention and quote eta_R near 2 to 2.4 x 1e-13 for globular-cluster RGB fits.
- **de Jager alternative (named in the pull):** de Jager, C., Nieuwenhuijzen, H., and van der Hucht, K. A. 1988, A&AS 72, 259, an all-HR-diagram mass-loss fit, refined by Nieuwenhuijzen, H., and de Jager, C. 1990, A&A 231, 134 (adds the total-mass dependence). Named here as the cited alternative; its polynomial coefficients were not extracted (not requested as the primary form).

- **Helium-flash core-mass anchor, primary citation:** Sweigart, A. V., and Gross, P. G. 1978, "Evolutionary Sequences for Red Giant Stars", ApJS 36, 405. The helium-core mass at the tip of the red giant branch (the helium flash) is M_c approximately 0.45 to 0.48 M_sun for low-mass stars (roughly 0.7 to 2.2 M_sun initial mass), nearly independent of initial mass and only weakly composition dependent (higher for lower metallicity and lower helium, toward about 0.50 M_sun). The degenerate helium core ignites explosively when M_c reaches this value. The observational verification framework is Raffelt, G. G. 1990, ApJ 365, 559, updated by Catelan, M., de Freitas Pacheco, J. A., and Horvath, J. E. 1996, "The helium-core mass at the helium flash in low-mass red giant stars: observations and theory", ApJ 461, 231, arXiv:astro-ph/9509062, which constrains departures from the standard M_c to be at most a few 0.01 M_sun.

- **Confidence:** High on the Reimers functional form and the Reimers 1975 citation (both read verbatim from Schroeder and Cuntz). High on the helium-flash core mass falling in the 0.45 to 0.48 M_sun band and the Sweigart and Gross 1978 primary. Medium on the 4e-13 prefactor being attributable to the original Reimers 1975 text rather than to Kudritzki and Reimers 1978.
- **Caveat:** the two conventions for where the 4e-13 lives (in the prefactor with dimensionless eta, versus inside a dimensional eta_R) are both in the literature; state which the build uses. The exact standard M_c value depends on the adopted composition; Sweigart and Gross 1978 give the composition grid, and a single representative figure often quoted is M_c approximately 0.475 M_sun near solar composition.

---

## Item 7. VERIFICATION: Petit et al. 2020 diffusion coefficient and overlap spacing delta_ov

- **Label:** VERIFICATION (does Eq. 61 confirm the reconstructed delta_ov approximately 0.85 eps^(1/4)?).
- **Primary citation:** Petit, A. C., Pichierri, G., Davies, M. B., and Johansen, A. 2020, "The path to instability in compact multi-planetary systems", Astronomy and Astrophysics 641, A176, DOI 10.1051/0004-6361/202038764, arXiv:2006.14903 (v2). Read from the extracted PDF text.

- **Diffusion coefficient (confirmed).** The scalar (Chirikov 1979) diffusion coefficient for the resonance locator near a resonance is Eq. (62): D_{p+q} = (omega_pq / 2 pi) (delta-eta_pq)^2, where omega_pq is the small-oscillation frequency at the resonance (Eq. 52: omega_pq = n_2 eps_M A sqrt(eta(1-eta))/delta (p+q) e^{-(p+q) delta}, with A = 3.47) and delta-eta_pq the resonance width in period-ratio space (Eq. 55). The effective, locally constant diffusion coefficient after integrating over the resonance index is Eq. (76), a compact expression D_eff proportional to eps_M A n_2 sqrt(eta(1-eta)) that depends on delta and delta_ov and vanishes as delta approaches delta_ov (no exponential term). The relevant mass ratio is eps_M (Eq. 53): eps_M = sqrt( m1 m3 + m2 m3 eta^2 alpha_12^{-2} + m1 m2 alpha_23^2 (1-eta)^2 ) / m0.

- **Overlap spacing (the check).** The critical generalized spacing at which the zeroth-order three-planet MMR network fills the whole space is Eq. (59): delta_ov = (6.55 eps_M)^{1/4} (eta(1-eta))^{3/8}. For equal masses and equal spacing this reduces to Eq. (61):

  delta_ov,eq = 1.16 (m_p / m_0)^{1/4}

  where m_p is the planet mass and m_0 the stellar mass, that is eps = m_p/m_0.

- **Verdict:** the exponent is confirmed: Eq. (61) scales as eps^{1/4} = (m_p/m_0)^{1/4}, exactly the reconstructed power. The numeric coefficient is NOT confirmed: Eq. (61) gives 1.16, not 0.85. So the reconstructed "delta_ov approximately 0.85 eps^{1/4}" does not match Eq. (61) as written. Two structural facts explain where a different coefficient can arise, and neither yields 0.85: (a) delta_ov,eq is the generalized spacing of Eq. (45), delta = (delta_12 delta_23)/(delta_12 + delta_23), which for an equal-spacing system equals half the physical orbital spacing, so the actual orbit spacing is 2 delta_ov,eq = 2.32 (m_p/m_0)^{1/4}; (b) evaluating the general Eq. (59) with eps_M rather than the reduced m_p/m_0 and eta at the symmetric point gives a coefficient near 1.0, still not 0.85. The reconstruction should be reconciled against Eq. (61): the authoritative coefficient for the generalized overlap spacing is 1.16, and 2.32 for the physical orbital spacing.

- **Confidence:** High. Eq. (61), Eq. (59), Eq. (62), Eq. (52), Eq. (53), Eq. (76), and the generalized-spacing definition Eq. (45) were all read from the extracted PDF text.
- **Caveat:** the pull referenced a public companion notebook; the check here is against the published equations, which are the authoritative form. If the notebook computes 0.85, it is either using a different normalization (for example a Hill-radius-scaled or a per-pair spacing) or a specific eta and mass distribution; that discrepancy against the paper's 1.16 should be flagged to whoever reconstructed the 0.85 before it is coded.

---

## Partially verified / NOT-VERIFIED (what is still needed)

- **Item 1, original Sharp and Wasserburg 1995 coefficients (needs the paywalled paper).** The usable analytic relations are the Adams and Lodders 2025 reproduction and extension; the exact S&W-1995 numeric coefficients require pulling Geochimica et Cosmochimica Acta 59, 1633 (1995). Adams and Lodders deliberately revise C2H stability, so the two coefficient sets are close but not identical.
- **Item 2, per-temperature delta-f-G(T) table and SiC beta polymorph (needs Monograph 9 / Kleykamp).** The JANAF WebBook returns the Shomate fits and 298 K anchors used above; the row-by-row delta-f-G(T) column and the resolved beta-cubic SiC Gibbs energy need the printed Monograph 9 tables (the WebBook JANAF table view returned HTTP 403 here) and Kleykamp 1998 (Ber. Bunsenges. Phys. Chem. 102, 1231) respectively.
- **Item 3, exact Barin Fe3C row (needs the Barin volume).** The Hallstedt et al. 2010 formation enthalpies (+27.0 kJ/mol at 298 K, +23.5 kJ/mol at 0 K) are confirmed to the primary. The Barin S(298) = 104.6 J/(mol K), delta-f-H = 25.104 kJ/mol, and delta-f-G = 20.083 kJ/mol values were read from a secondary compilation quoting Barin and should be confirmed against Barin, Thermochemical Data of Pure Substances (1995).
- **Item 4, de Boer et al. 1988 element parameter tables (needs the book).** The model form, P, Q = 9.4P, and the four metallization enthalpies (Si 8, Ge 6, C 24, N 57 kcal/g-atom) are confirmed. The per-element phi*, n_ws, molar volumes, and the carbide-specific R/P values live in Cohesion in Metals (1988) and the Calphad 7 (1983) appendix, which are not web-fetchable.
- **Item 5, Murray and Dermott equation numbers (needs the printed book).** The A and B matrix element forms and sign conventions are confirmed by two independent reproductions; the exact equation numbers in Solar System Dynamics 1999 (Section 7.7) should be confirmed against the printed text.
- **Item 6, the 4e-13 Reimers prefactor provenance.** The functional form and the Reimers 1975 citation are confirmed. Whether the 4e-13 constant is stated in Reimers 1975 itself or is the Kudritzki and Reimers 1978 (A&A 70, 227) calibration should be checked against those two papers before the constant is coded.
