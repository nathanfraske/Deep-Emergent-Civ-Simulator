# Disk-evolution arc literature-fetch values

This file records the primary-source fetch for the value, form, and condition list behind the disk-evolution arc: the binary resonance-truncation fraction that sets a circumstellar disk's outer edge, the radiative-envelope EUV (ionizing) departure band, and two reserved calibration anchors. These are cited constants, the legal cited residents of the value-line: each is a published value with a precise citation, an authored or guessed number is not admitted here. For every entry the label is one of [M] VALUE (a number with units), FORM (an equation or closure with its own coefficients), or CONDITION (a branch criterion). Each carries the extracted content verbatim, the primary citation (author, year, title, journal or volume and page, DOI and where one exists arXiv identifier, and the specific equation, table, figure, or page), a confidence, and any caveat. Where a value could not be pinned to a primary, it is flagged in the closing "Flagged / not-found" section rather than guessed.

Discipline note: PDFs often do not parse through the fetch path, and one automated fetch summary of the priority table returned coefficient values that the primary text did not contain (they were reconstructed against the primary PDF and discarded). Every load-bearing number below was therefore read from the primary text itself: the arXiv PDFs of Manara et al. 2019 and Lanz & Hubeny 2003 and Landin et al. 2010 were downloaded and converted to layout text, and the tables and equations read from that conversion. Where a value is quoted by a later paper or an open reproduction, that reproduction is named alongside the primary. Nothing here is a fabricated number.

Prose custom: no em dashes, and the project's three banned -ly adverbs are avoided, matching the standard for maintained prose.

---

## Group A: binarity, the resonance-truncation fraction and its lineage

The disk-hosting star's circumstellar disk is tidally truncated inside its Roche lobe where the companion's resonant tidal torque overcomes the disk's viscous torque. Manara et al. 2019 give the explicit closed form that returns the truncation radius from the binary parameters (mass ratio, eccentricity) and the disk viscosity (Reynolds number), built by fitting the numerical results of Artymowicz & Lubow 1994 (eccentric, viscosity-dependent) and Papaloizou & Pringle 1977 (the zero-eccentricity, viscosity-independent limit) onto an Eggleton Roche-lobe scale. This is the fetch the arc needs.

### A1. Truncation radius over projected separation, the full form (Manara et al. 2019, Eq. 8)

- **Label:** FORM (the truncation fraction, full orbital-phase and inclination dependence).
- **Extracted (verbatim, Eq. 8):**
  `Rtrunc/ap = [ 0.49 qi^(2/3) / (0.6 qi^(2/3) + ln(1 + qi^(1/3))) ] * ( b e^c + 0.88 mu^0.01 ) * [ (1 - e^2)/(1 + e cos nu) * sqrt(1 - sin^2(omega + nu) sin^2 i) ]^(-1)`
  where (their text) "e is the eccentricity, nu the true anomaly, omega the longitude of periastron, and i the inclination of the plane of the orbit with respect to the line of sight", "qi is the mass ratio (either q1 = M1/M2 or q2 = q = M2/M1)", and "b and c are the parameters derived in Appendix C.1 and tabulated in Table C.1, which depend on the disk viscosity or equivalently on the Reynolds number, R." Here `ap` is the projected (observed) separation and `mu = M2/(M1 + M2)` (Appendix C). The bracketed factor with `(1 - e^2)/(1 + e cos nu)` is the ratio of the instantaneous separation to `ap`; it is a minimum at apoastron (nu = pi) and a maximum at periastron (nu = 0).
- **Primary citation:** Manara, C. F., Tazzari, M., Long, F., Herczeg, G. J., Lodato, G., et al. 2019, "Observational constraints on dust disk sizes in tidally truncated protoplanetary disks in multiple systems in the Taurus region", Astronomy & Astrophysics 628, A95, DOI 10.1051/0004-6361/201935964, arXiv 1907.03846. Equation 8, Section 5.
- **Confidence:** High. Read from the primary arXiv PDF (converted text). The leading factor is the Eggleton (1983) Roche-lobe radius (item A3).
- **Caveat:** The `mu^0.01` term is nearly flat in `mu` (see A3), so the eccentricity term `b e^c` and the Roche-lobe scale carry almost all of the variation. The task's separate symbol "F = a/ap" corresponds to Manara's Eq. 7, the projection factor; it is the inverse of the last bracket in Eq. 8, not a distinct multiplier.

### A2. Face-on apoastron and periastron forms (Manara et al. 2019, Eq. 9)

- **Label:** FORM (the two bounding curves, inclination i = 0).
- **Extracted (verbatim, Eq. 9):** Under the face-on assumption (i = 0), the last bracket reduces to `(1 +/- e)^(-1)`, giving
  `Rtrunc/ap = [ 0.49 qi^(2/3) / (0.6 qi^(2/3) + ln(1 + qi^(1/3))) ] * ( b e^c + 0.88 mu^0.01 ) * (1 + e)^(-1)`  (apoastron)
  `Rtrunc/ap = [ 0.49 qi^(2/3) / (0.6 qi^(2/3) + ln(1 + qi^(1/3))) ] * ( b e^c + 0.88 mu^0.01 ) * (1 - e)^(-1)`  (periastron)
  "where the former refers to the truncation radius for an object located at apoastron and the latter at periastron."
- **Primary citation:** Manara et al. 2019 (as A1), Equation 9, Section 5.
- **Confidence:** High. Read from the primary PDF.
- **Caveat:** These are the i = 0 bounds; a real orbit at general phase and inclination lies between them per Eq. 8.

### A3. The Eggleton Roche-lobe scale and the fitting function (Manara et al. 2019, Eqs. C.1, C.2, C.3)

- **Label:** FORM (the Roche-lobe normalization and the exponential-in-eccentricity fit).
- **Extracted (verbatim):**
  - Fitting function, Eq. C.1: `Rt(M1, M2, e, a) = Ri,Egg * ( b e^c + h mu^k )`, "where b, c, h, and k are the fitting parameters".
  - Roche-lobe scale, Eq. C.2: `Ri,Egg / a = 0.49 qi^(2/3) / ( 0.6 qi^(2/3) + ln(1 + qi^(1/3)) )`, "where q1 = M1/M2 and q2 = M2/M1." (This is the Eggleton 1983 Roche-lobe approximation applied per star.)
  - Zero-eccentricity term: "the truncation in zero-eccentricity binaries is determined by the Papaloizou & Pringle (1977) mechanism: we can therefore obtain the value of h and k simply by fitting the results obtained by them ... where the fitted parameters are h = 0.88 and k = 0.01. The exponent of mu is very small, the dependence on the masses is only inside Ri,Egg, and in general the truncation occurs at 0.85 - 0.9 times the size of the Roche Lobe."
  - Eccentricity term, Eq. C.3: `Rt(M1, M2, e, a) = Ri,Egg ( b e^c + 0.88 mu^0.01 )`, with b and c "calculated by fitting the numerical results from Artymowicz & Lubow (1994)".
- **Primary citation:** Manara et al. 2019 (as A1), Appendix C.1, Eqs. C.1, C.2, C.3; the Roche-lobe form is Eggleton, P. P. 1983, "Approximations to the radii of Roche lobes", ApJ 268, 368, DOI 10.1086/160960 (the 0.49 q^(2/3) / (0.6 q^(2/3) + ln(1 + q^(1/3))) approximation).
- **Confidence:** High on Eqs. C.1-C.3, h = 0.88, k = 0.01, and the 0.85 - 0.9 Roche-lobe statement (read from the primary PDF). The Eggleton 1983 DOI is the standard reference for that formula; it was not re-fetched this session.
- **Caveat:** The zero-eccentricity truncation (the `0.88 mu^0.01` term) is set by nonresonant interaction and is viscosity-independent (item A6); the viscosity dependence enters only through b, c in the eccentricity term (item A4).

### A4. Table C.1 best-fit b and c versus mass parameter and Reynolds number (the load-bearing rows)

- **Label:** [M] VALUE (the coefficient table for Eq. C.3 / Eq. 8).
- **Extracted (verbatim, Table C.1):** caption "Best fit parameters for Equation C.3 for different values of mu and R, both for circumprimary and circumsecondary disks." Here `mu = M2/(M1 + M2)` and `R` is the disk Reynolds number (a viscosity measure; the paper writes it, as converted, `R = alpha nu^(-1) (r/H)^2`, with lower R meaning higher viscosity). The tabulated rows:

  Circumprimary disk (b, c):
  ```
  mu = 0.1:  R=1e4  b=-0.66  c=0.84   R=1e5  b=-0.75  c=0.68   R=1e6  b=-0.78  c=0.56
  mu = 0.2:  R=1e4  b=-0.72  c=0.88   R=1e5  b=-0.78  c=0.72   R=1e6  b=-0.80  c=0.60
  mu = 0.3:  R=1e4  b=-0.76  c=0.92   R=1e5  b=-0.80  c=0.75   R=1e6  b=-0.81  c=0.63
  mu = 0.4:  R=1e4  b=-0.77  c=0.95   R=1e5  b=-0.81  c=0.78   R=1e6  b=-0.82  c=0.66
  mu = 0.5:  R=1e4  b=-0.78  c=0.94   R=1e5  b=-0.81  c=0.78   R=1e6  b=-0.82  c=0.66
  ```
  Circumsecondary disk (b, c):
  ```
  mu = 0.1:  R=1e4  b=-0.81  c=0.98   R=1e5  b=-0.81  c=0.80   R=1e6  b=-0.83  c=0.69
  mu = 0.2:  R=1e4  b=-0.81  c=0.99   R=1e5  b=-0.82  c=0.82   R=1e6  b=-0.83  c=0.70
  mu = 0.3:  R=1e4  b=-0.79  c=0.97   R=1e5  b=-0.82  c=0.81   R=1e6  b=-0.83  c=0.69
  mu = 0.4:  R=1e4  b=-0.80  c=0.98   R=1e5  b=-0.82  c=0.80   R=1e6  b=-0.83  c=0.68
  mu = 0.5:  R=1e4  b=-0.79  c=0.95   R=1e5  b=-0.81  c=0.78   R=1e6  b=-0.82  c=0.66
  ```
  The paper adds: "The fitting parameters do not depend much on mu. For a general choice of mu we simply interpolate the fitting parameters reported in Table C.1." The three Reynolds numbers are R = 10^4, 10^5, 10^6, the same viscosity ladder plotted in the appendix figures (labeled Re = 1e4, 1e5, 1e6).
- **Primary citation:** Manara et al. 2019 (as A1), Table C.1 and Appendix C.1. The b, c fits are derived from the numerical truncation results of Artymowicz, P., & Lubow, S. H. 1994, ApJ 421, 651 (item A6).
- **Confidence:** High. Read directly from the primary arXiv PDF (converted text). An earlier automated fetch of the journal HTML returned a spurious three-row b,c set (b about -0.5 to -0.7, c about 0.1 to 0.2 at R = 1e6 to 1e8); those values are not in the paper and were discarded. The values above are the paper's own.
- **Caveat:** The table spans R = 10^4 to 10^6 only (not 10^7 to 10^8); a disk outside that viscosity range needs extrapolation or a return to the Artymowicz & Lubow (1994) grid. b is negative and c is positive throughout, so `b e^c` is a negative, eccentricity-growing correction that shrinks the disk as e rises. Higher viscosity (lower R) gives a larger truncation radius (b closer to zero, c larger), consistent with the resonance-versus-viscous balance.

### A5. The circular-orbit limit and the Roche-lobe fraction (Manara et al. 2019, Section 5)

- **Label:** [M] VALUE plus CONDITION (the zero-eccentricity truncation and the Roche-lobe fraction).
- **Extracted (verbatim):** "it can be analytically computed that tidal torques dominate over viscous ones outside a truncation radius, which for a circular orbit is Rt ~ 0.3 * a, where a is the semimajor axis of the binary orbit, with a dependence on the mass ratio q." And, from Appendix C.1, "in general the truncation occurs at 0.85 - 0.9 times the size of the Roche Lobe." Their data finding: the observed dust-radius-to-separation ratio "is always lower than 0.3 in our sample", and matching the observed sizes to the truncation prediction requires high eccentricities, "the inferred minimum values of eccentricity are in general quite high (e > 0.5 in 9/11 cases)", or dust radii smaller than gas radii "by factors >~ 2-3, probably due to a more effective drift of the dust".
- **Primary citation:** Manara et al. 2019 (as A1), Section 5.1, 5.2, Appendix C.
- **Confidence:** High on `Rt ~ 0.3 a` (circular), the 0.85 - 0.9 Roche-lobe fraction, and the high-eccentricity or dust-drift reconciliation. Read from the primary PDF.
- **Caveat:** The `0.3 a` figure is the round circular-orbit number; the precise value follows from Eq. C.3 with e = 0 (that is, `0.88 mu^0.01 * Ri,Egg`, about 0.85 - 0.9 of the Eggleton radius), and the Eggleton radius is itself roughly 0.3 - 0.4 a near equal mass.

### A6. Lineage: the eccentric, viscosity-dependent grid (Artymowicz & Lubow 1994)

- **Label:** FORM plus CONDITION (resonant-torque versus viscous-torque truncation; b, c source).
- **Extracted:** Truncation is set "by balancing the resonant torques with the disk viscous torques and hence depends on the mass ratio, the orbital eccentricity, and the Reynolds number ... in the disk" (Manara's summary of Artymowicz & Lubow). Larger disk viscosity (lower Reynolds number) yields a larger truncation radius. The numerical `Rtrunc(q, e, R)` results of this paper are the grid Manara et al. fit to obtain b and c (Table C.1, item A4).
- **Primary citation:** Artymowicz, P., & Lubow, S. H. 1994, "Dynamics of Binary-Disk Interaction. I. Resonances and Disk Gap Sizes", The Astrophysical Journal 421, 651-667, DOI 10.1086/173679.
- **Confidence:** High on the mechanism (resonant versus viscous torque balance, dependence on q, e, and Reynolds number) and on the DOI (confirmed 10.1086/173679, ApJ 421, 651). The paper's own truncation-radius tables and figures (e.g. their Fig. 8, gap size versus eccentricity and Reynolds number) were not read verbatim this session; the numeric values reach the arc through Manara's Table C.1 fit.
- **Caveat:** If the arc needs the truncation radius outside the Manara fit domain (R > 10^6, or circumbinary rather than circumstellar disks), read Artymowicz & Lubow 1994 directly; it is the underlying grid. This paper has no arXiv version (1994).

### A7. Lineage: the zero-eccentricity, viscosity-independent limit (Papaloizou & Pringle 1977)

- **Label:** CONDITION (the nonresonant truncation branch, e = 0).
- **Extracted:** For a circular orbit the truncation is due mostly to nonresonant interaction, "in which case the truncation radius does not depend on viscosity and is just a function of the mass ratio" (Manara's summary), expressed through `mu = M2/(M1 + M2)`. This is the source of the `h = 0.88`, `k = 0.01` term in Eq. C.1 / C.3. The paper itself computes the tidal torque that increases with radius until it balances the local viscous torque, truncating the disc near where free-particle orbits first intersect, inside the Roche lobe.
- **Primary citation:** Papaloizou, J., & Pringle, J. E. 1977, "Tidal torques on accretion discs in close binary systems", Monthly Notices of the Royal Astronomical Society 181, 441-454, DOI 10.1093/mnras/181.3.441 (ADS bibcode 1977MNRAS.181..441P).
- **Confidence:** High on the role (the zero-eccentricity, viscosity-independent truncation branch) and on the DOI (confirmed 10.1093/mnras/181.3.441, MNRAS 181, 441). The specific numeric truncation fraction reaches the arc through Manara's fit (h = 0.88, k = 0.01).
- **Caveat:** The `0.88 mu^0.01` term encodes this branch; its provenance is the Papaloizou & Pringle 1977 zero-eccentricity result, not a free parameter.

### A8. Lineage: the Roche-lobe disk in a binary (Paczynski 1977)

- **Label:** CONDITION (the disk fits well inside the Roche lobe).
- **Extracted:** From periodic test-particle orbits, the radial extent of an accretion disk in a close binary is limited well below the Roche-lobe size; the outer disk edge is set by the largest non-intersecting periodic orbit. This is the foundational statement that a binary caps the disk radius inside the Roche lobe, made quantitative by the later resonance and torque work (A6, A7) and by Manara's 0.85 - 0.9 Roche-lobe fraction (A5).
- **Primary citation:** Paczynski, B. 1977, "A model of accretion disks in close binaries", The Astrophysical Journal 216, 822-826, DOI 10.1086/155524.
- **Confidence:** High on the qualitative result and the DOI (confirmed 10.1086/155524, ApJ 216, 822). The paper's own numeric disk-edge fractions were not read verbatim this session.
- **Caveat:** Paczynski 1977 gives the geometric orbit-crossing limit; the viscosity- and eccentricity-resolved fraction the arc uses is the Manara / Artymowicz & Lubow line.

---

## Group B: the radiative-envelope EUV (ionizing) departure band

A hot, radiative (dynamo-dark) star's ionizing output (EUV, photon energy above 13.6 eV, the H Lyman continuum) is not an LTE blackbody at its effective temperature: the Lyman jump (the discontinuity at the 912 Angstrom edge) suppresses the ionizing flux below the Planck value, strongly at cooler effective temperatures and high gravity, weakly at the hottest temperatures. The NLTE line-blanketed model-atmosphere grids give the emergent ionizing photon flux. Below is the pinnable quantity, the grid ionizing flux versus effective temperature; the single clean L_EUV/L_bol-versus-T_eff blackbody-departure band is flagged in the closing section as the quantity still needing a specific grid-versus-Planck comparison.

### B9. OSTAR2002 H-ionizing photon surface flux versus effective temperature (Lanz & Hubeny 2003, Table 4)

- **Label:** [M] VALUE (grid H-ionizing photon flux, the pinnable quantity).
- **Extracted (verbatim, Table 4):** "Ionizing fluxes in the H I Lyman continuum as function of effective temperature, gravity and metallicity", column "q0 = log NLyC [s^-1 cm^-2]", the surface flux of H-ionizing (> 13.6 eV) photons. At solar metallicity (Z/Zsun = 1) and log g = 4.0:
  ```
  Teff = 27500 K   q0 = 22.39
  Teff = 30000 K   q0 = 23.01
  Teff = 32500 K   q0 = 23.50
  Teff = 35000 K   q0 = 23.84
  Teff = 37500 K   q0 = 24.08
  Teff = 40000 K   q0 = 24.28
  ```
  The paper states: "The ionizing fluxes, q0 and q1, are given as logarithms of the number of photons in these two continua, per second and per square centimeter at the stellar surface." (q1 is the He I lambda-504 continuum, Table 5.) "Lower gravity models have larger ionizing fluxes due to higher hydrogen and helium ionization (thus the Lyman and lambda-504 jumps are smaller). Model atmospheres with low metallicity have lower Lyman continuum fluxes."
- **Primary citation:** Lanz, T., & Hubeny, I. 2003, "A Grid of NLTE Line-Blanketed Model Atmospheres of O-Type Stars" (the OSTAR2002 grid), The Astrophysical Journal Supplement Series 146, 417, DOI 10.1086/374373, arXiv astro-ph/0210157. Table 4 (q0), Table 5 (q1), Section 7.2.
- **Confidence:** High. Read directly from the primary arXiv PDF (converted text), solar-metallicity log g = 4.0 rows.
- **Caveat:** `q0` is the emergent flux per square centimeter of stellar surface; the total H-ionizing photon rate is `Q0 = 4 pi R^2 * 10^q0` for stellar radius R. These are O-star temperatures (27500 - 55000 K in the grid). For B stars (roughly 15000 - 30000 K) use BSTAR2006 (Lanz & Hubeny 2007, ApJS 169, 83, DOI 10.1086/511270), where the Lyman jump is deeper and the H-ionizing flux falls further below blackbody. A companion grid tabulating total Q0 per spectral type (rather than surface flux) is Sternberg, Hoffmann & Pauldrach 2003 (ApJ 599, 1333, DOI 10.1086/379506); its numeric Q0 table was not extracted this session (see flag).

### B10. The Lyman-jump departure from blackbody and model-to-model differences (Lanz & Hubeny 2003)

- **Label:** FORM/CONDITION plus [M] VALUE (the direction and size of the departure).
- **Extracted (verbatim):** "The Lyman jump gradually weakens with increasing effective temperature, and essentially disappears at 50 000 K" (Fig. 10, log g = 4, solar). For fixed Teff = 40000 K, "The most striking feature is the magnitude of the Lyman jump, which is very strong at log g = 4.5, and almost disappears at log g = 3.5" (Fig. 11). Compared with LTE Kurucz (1993) models: "NLTE models predict higher flux than Kurucz model in the Lyman continuum, although the effect is relatively small at 40 000 K." Compared with WM-Basic wind models: "wm-basic models predict ionizing fluxes smaller by 0.57 and 0.24 dex, respectively."
- **Primary citation:** Lanz & Hubeny 2003 (as B9), Sections 6 and 7.2, Figs. 10, 11, 13.
- **Confidence:** High on the qualitative departure (Lyman jump deep at cool Teff and high gravity, vanishing near 50000 K) and on the model-to-model dex differences, read from the primary PDF.
- **Caveat:** These are NLTE-versus-LTE and NLTE-versus-wind-model differences (0.24 - 0.57 dex), not a direct NLTE-grid-versus-Planck ratio. The direct departure from a blackbody at the same Teff, expressed as a single L_EUV/L_bol(Teff) band, is not stated as one number in this grid and is flagged below. The departure grows with wavelength beyond the H edge and is largest in the He II Lyman continuum (> 54.4 eV), where the flux sits many orders of magnitude below blackbody (see flag).

---

## Group C: two reserved-with-basis-and-cited anchors

These two items are reserved calibration anchors, not free data columns. Each is a cited literature value that supplies the BASIS for a reserved quantity the arc will need (a convection-onset temperature; a mixing-length turnover-time scale), surfaced here for the owner's ratification and never planted into code as a hardcoded default. The fetch discipline is the same: verbatim, cited, flagged if unpinnable.

### C11. The Kraft break effective temperature (reserved-with-basis-and-cited: convection-onset / dynamo boundary)

- **Label:** [M] VALUE, reserved-with-basis-and-cited.
- **Extracted (verbatim / cited):** Kraft 1967 found that main-sequence rotation rates drop sharply near spectral type F5, attributed to the onset of a deep convective envelope and magnetic braking in cooler stars. The classic effective-temperature reading of the break is ~6200 K (mid-F, near 1.3 solar masses), the boundary between stars with deep convective envelopes and efficient magnetic dynamos and hotter radiative-envelope stars that spin fast. Recent single-star refinements place the break at Teff ~ 6550 K with a width of about 200 K (a mass range ~1.32 - 1.41 solar masses), and a unified obliquity/rotation break near ~6500 K.
- **Primary citation:** Kraft, R. P. 1967, "Studies of Stellar Rotation. V. The Dependence of Rotation on Age among Solar-Type Stars", The Astrophysical Journal 150, 551, DOI 10.1086/149359 (the rotation break near F5). Modern effective-temperature refinements: the ~6550 K +/- 200 K value is from the 2024 study "The Kraft Break Sharply Divides Low Mass and Intermediate Mass Stars" (arXiv 2408.02638), and a unified ~6500 K break is reported in arXiv 2511.15610.
- **Confidence:** High on Kraft 1967 (DOI 10.1086/149359, ApJ 150, 551, title confirmed) and on the classic ~6200 K / F5 reading. Medium on the exact modern refinement value: the ~6550 K +/- 200 K and ~6500 K figures were read from search summaries of the recent arXiv papers, not from their primary tables, so the modern citation and number should be confirmed against those papers before ratification.
- **Caveat (reserved-anchor basis):** The owner sets the convection-onset / dynamo-cutoff temperature the arc keys on; the basis is the observed rotation break, classic ~6200 K (F5, Kraft 1967) or the modern ~6500 K refinement. The mechanism (a radiative envelope loses the surface convection zone that drives the dynamo and the braking wind) is the physics; the exact Teff is per-star and metallicity-dependent, so it is surfaced as reserved, not hardcoded.

### C12. A pre-main-sequence convective turnover time (reserved-with-basis-and-cited: mixing-length turnover scale)

- **Label:** [M] VALUE, reserved-with-basis-and-cited.
- **Extracted (verbatim, Landin et al. 2010, Table 1, 1 solar-mass PMS track):** columns are log Age (yr), log L/Lsun, log Teff (K), log g (cgs), log tau_c (local convective turnover time, seconds), tau_g (global convective turnover time, days), and Rossby number Ro. Early PMS rows:
  ```
  log Age = 6.2324 (~1.7 Myr):  log tau_c = 6.3275 s (~24.7 d)   tau_g = 766.6288 d   Ro = 0.1436
  log Age = 6.4053 (~2.5 Myr):  log tau_c = 7.1018 s (~146 d)    tau_g = 488.2106 d   Ro = 0.0183
  log Age = 6.5840 (~3.8 Myr):  log tau_c = 7.0810 s (~139 d)    tau_g = 369.1187 d   Ro = 0.0146
  log Age = 6.7666 (~5.8 Myr):  log tau_c = 7.0074 s (~124 d)    tau_g = 291.4590 d   Ro = 0.0131
  ```
  The global convective turnover time reaches its maximum early (a fully or deeply convective PMS star) at `tau_g ~ 767 days` for the 1 solar-mass model, then "decreasing substantially during contraction to the zero-age main sequence." The local turnover time tau_c is of order tens to ~150 days over the same interval.
- **Primary citation:** Landin, N. R., Mendes, L. T. S., & Vaz, L. P. R. 2010, "Theoretical values of convective turnover times and Rossby numbers for solar-like, pre-main sequence stars", Astronomy & Astrophysics 510, A46, DOI 10.1051/0004-6361/200913015, arXiv 1001.2754. Table 1 (the 1 solar-mass track).
- **Confidence:** High. Read directly from the primary arXiv PDF (converted text); Table 1 is explicitly the 1 solar-mass model. An earlier automated fetch summary misreported log-age 6.2324 as "6.23 Myr" (it is 10^6.2324 yr ~ 1.7 Myr); the corrected reading is above.
- **Caveat (reserved-anchor basis):** The owner sets the mixing-length turnover scale the young-star dynamo scaling anchors to; the basis is a published pre-MS global convective turnover time, `tau_g ~ 300 - 770 days` for a deeply convective 1 solar-mass PMS star (or the local `tau_c ~ 25 - 150 days`), declining as the star contracts. The value is mass- and age-dependent (and code-dependent: ATON 2.3 grey and non-grey models here), so it is surfaced as a reserved anchor with its basis, not a hardcoded constant.

---

## Flagged / not-found

- **The single L_EUV/L_bol(T_eff) blackbody-departure band (the ionizing "departure factor").** What is pinned: the OSTAR2002 grid H-ionizing surface flux q0(Teff) (item B9), and the direction and gravity/temperature dependence of the Lyman-jump departure (item B10). What is not pinned to one primary number: a clean ratio of the NLTE-grid ionizing luminosity to the Planck (blackbody-at-Teff) ionizing luminosity, as a function of Teff, expressed as a single band. This needs a specific grid-versus-Planck integration (integrate the OSTAR2002 or BSTAR2006 emergent flux shortward of 912 Angstrom and divide by the blackbody value at the model Teff). It is flagged rather than guessed. Note the departure is smallest for the hottest O stars (Lyman jump vanishes near 50000 K, so the H-ionizing output approaches blackbody) and grows to many orders of magnitude in the He II continuum (> 54.4 eV) and for cooler B / Herbig stars where the Lyman jump is deep.
- **The He II (> 54.4 eV) departure, orders of magnitude.** Search-level (not primary-read) statements indicate the He+ ionizing flux from wind-blanketed and spherical NLTE models can be ~2 to 3 orders of magnitude above plane-parallel NLTE and ~3 to 6 orders of magnitude above LTE, and that spherical line blanketing raises the H I Lyman continuum by up to a factor ~5 over plane-parallel. These quantify a large departure but were read from search summaries, likely tracing to the WM-Basic / wind-blanketing literature (for example Pauldrach, Hoffmann & Lennon 2001, A&A 375, 161); the specific factors should be confirmed against that primary before use. Flagged as partially verified.
- **Sternberg, Hoffmann & Pauldrach 2003 total Q0 per spectral type.** Cited (ApJ 599, 1333, DOI 10.1086/379506) as the companion grid tabulating total ionizing photon rates Q0 per spectral type; the numeric table was not extracted this session (an automated fetch returned an unrelated document). Read it directly if the arc needs Q0 keyed to spectral type rather than the OSTAR2002 surface flux q0(Teff).
- **BSTAR2006 (B-star) ionizing fluxes.** Cited (Lanz & Hubeny 2007, ApJS 169, 83, DOI 10.1086/511270) as the grid covering the B-star / radiative-envelope temperatures where the H-ionizing departure from blackbody is largest; its q0(Teff) table was not extracted this session.
- **Eggleton 1983 Roche-lobe DOI.** The Roche-lobe formula in Manara Eq. C.2 is the Eggleton 1983 approximation (ApJ 268, 368, DOI 10.1086/160960 per standard references); the DOI was not independently re-fetched this session. The formula itself is read verbatim from Manara.
- **Artymowicz & Lubow 1994 and Papaloizou & Pringle 1977 native truncation numbers.** The DOIs are confirmed (10.1086/173679; 10.1093/mnras/181.3.441) and the mechanisms pinned, but their own tabulated truncation-radius values were not read verbatim; they reach the arc through Manara's Table C.1 fit (item A4). Read the originals if a truncation radius outside the fit domain (Reynolds number above 10^6, or circumbinary geometry) is needed.
- **Modern Kraft-break refinement.** The ~6550 K +/- 200 K and ~6500 K modern values (item C11) were read from search summaries of arXiv 2408.02638 and arXiv 2511.15610, not from their primary tables; confirm before ratifying the reserved anchor. Kraft 1967 (DOI 10.1086/149359) and the classic ~6200 K reading are solid.
- **The Larson-Penston collapse eigenvalue (m0 = 46.9 at A = 8.85), and the m0(A) family table.** CHANNEL-RELAYED, PROVISIONAL, FETCH-FLAGGED (research-agent audit of f369bdf, 2026-07-18). The values 46.9 and A = 8.85 reached the `astro::CollapseModel::larson_penston` constructor through a secondary review's restatement of the isothermal-sphere similarity-solution family, not a vendored primary: no bytes were downloaded, no receipt computed, no printed table read source-verbatim. They stay LIVE for the collapse-model band under the modality `channel-relayed-provisional`, never a citation, per the standing rule (channel-supplied numbers are fetch-spec seeds, only vendored bytes retire a fetch-flag). Retirement spec: locate the primary table where `A = 8.85` and `m0 = 46.9` are printed (candidate primaries Hunter 1977, MNRAS 178, 179, and Whitworth & Summers 1985, MNRAS 214, 1, verified BY CONTENT, routes are not findings), download and SHA256 the bytes, read the eigenvalue and its abscissa verbatim, then flip the modality to vendored with the receipt on file (the `af390700` Shu row is the template). Same treatment for the general Shu 1977 Table 1 `m0(A)` row (the concordance target once `m0(A)` is derived in-engine from the similarity ODE) and the Foster-Chevalier 1993 / Larson 2003 peaked `~13 c_s^3/G` rate-law member (the named rate-law debt). The Ori-Piran 1988 stability note and the factor-48 framing carry the same channel-relayed marker pending these fetches.
