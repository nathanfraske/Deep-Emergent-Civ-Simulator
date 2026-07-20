# Pipeline literature-fetch values

This file records the primary-source fetch for a set of cited literature values that the physics pipeline needs: dimensionless function shapes (Green's functions, production-function polynomials, curves) and empirical coefficients. These are cited constants, the legal cited residents of the value-line: the point is that each is a published value with a precise citation, never an authored or guessed number. For every entry the quantity, the numeric value or functional form with its coefficients, the units, and the primary citation (author, year, title, and the equation, table, or page number, with a DOI, arXiv identifier, or ISBN where one exists) are given. Where a value could not be pinned to a primary source, it is flagged as not-found under its entry rather than guessed.

Extraction note: several primaries are paywalled PDFs that did not parse through the fetch path. Where that happened, the value was read from an open reproduction that carries the same equation or table (a course note that cites the textbook equation number, an open-access toolbox paper that reprints the Green's function, the maintainer-curated craterstats coefficient file that cites the primary), and the reproduction is named alongside the primary. Nothing here is fabricated; the closing "Flagged / not-found" notes list every value that remains unpinned.

Prose custom: no em dashes, and the project's three banned -ly adverbs are avoided, matching the standard for maintained prose.

---

## 1. Turcotte-Schubert flexure (flexure kernel, slice 5)

The thin-elastic-plate-over-fluid model. All forms below are the constant-rigidity, no-end-load solutions, keyed to Turcotte & Schubert equation numbers through two open reproductions that cite them.

**Flexural rigidity (FORM).**
`D = E h^3 / [12 (1 - nu^2)]`, units N m (a rigidity times length). `E` is Young's modulus, `h` (or `Te`) the elastic thickness, `nu` Poisson's ratio. This is Turcotte & Schubert, Geodynamics, chapter 3 ("Elasticity and Flexure"), the moment-curvature / plate-bending relation. Reproduced as equation (13) in the GEOL5690 flexure notes (which cite T&S sec. 3-9) and as equation (3) in the TAFI toolbox paper.

**Governing equation (FORM).**
`D d^4 w / dx^4 + (rho_m - rho_w) g w(x) = q_a(x)` in 2-D Cartesian form, and `D (d^2/dr^2 + (1/r) d/dr)^2 w + (rho_m - rho_w) g w = q_a(r)` in axisymmetric radial form. `rho_m - rho_w` is the density contrast between the mantle/asthenosphere below the plate and the material filling the deflection (water or sediment). TAFI equations (1) and (2).

**Flexural parameter (FORM).**
`alpha = [ 4D / ((rho_m - rho_w) g) ]^(1/4)`, units m. This is Turcotte & Schubert equation (3-127), reproduced verbatim with that equation number in the UCSD/topex geodynamics flexure notes (Sandwell). It is the LINE-LOAD flexural parameter; the depression-to-forebulge zero crossing sits at `x_0 = 3 pi alpha / 4` for the continuous LINE load.

CORRECTED 2026-07-17 (RUNBOOK section 11, erratum standard): the earlier form of this line said `alpha` is "used as the flexural parameter for both the line-load and the point-load solutions in the TAFI toolbox (Table 1)." THAT WELD IS WRONG, and it propagated a `sqrt(2)` length-scale error into `flexure::point_load_deflection` before it was caught. TAFI Table 1 gives the POINT load its OWN flexural parameter, `[D/((rho_m - rho_w)g)]^(1/4)` with NO factor of 4 (cited to Brotchie & Silvester 1969), distinct from the line-load rows; the toolbox overloads the symbol `alpha` by table row. The axisymmetric point-load Green's function `w = -(P l^2 / 2 pi D) kei(r/l)` runs on `l = (D/((rho_m - rho_w)g))^(1/4) = alpha / sqrt(2)`, settled three independent ways (the plate ODE `grad^4 kei = -kei` forcing `l^4 = D/dpg`; McNutt & Menard 1982 eq. A8; the TAFI point-load row read at 220/450 dpi). Full record: `docs/working/POINT_LOAD_LENGTH_SCALE_FETCH.md`. The point-load zero crossing is therefore at `x_0 = 3.91467 l = 3.91467 alpha / sqrt(2)`, not the line load's `3 pi alpha / 4`.

SETTLED 2026-07-18, the source question closed against the vendored primary: THE DEFECT WAS OURS AND TAFI IS NOT IN ERROR. The paper was vendored under the fetch discipline (`docs/working/VENDORING_CHECKLIST.md`, artifact `flexure_tafi`) and its Table 1 was re-read on the held bytes: the point-load row's flexural parameter, rendered at 900 dpi, prints `[D/((rho_m - rho_i) + E Te/R^2)]^(1/4)` with the numerator `D` alone, against `[4D/((rho_m - rho_i) g)]^(1/4)` in both line-load rows. The four rows are cited row by row, the line-load and sinusoidal rows to Turcotte and Schubert (2002) and the point-load row to Brotchie and Silvester (1969), so the one symbol `alpha` is bound to four different definitions by row and equation (6) never claimed the line-load value. The authors' own toolbox agrees: `flexparam.m` in `github.com/sumantjha/TAFI_v1.0` computes `alpha=(D/((E*Te/R^2)+gamma))^0.25` for the point-load branch and `alpha = (4*(D)/(gamma))^0.25` for the rest, having set `gamma = gamma*g` one line earlier. So no erratum is owed to these authors on this question, and the sentence that welded the two lengths was written here. ONE PRINTED SLIP IN TAFI IS RECORDED AND IS NOT LOAD-BEARING: the point-load Table 1 cell drops the `g` from its buoyancy term, leaving `(rho_m - rho_i)` in kg m^-3 added to `E Te/R^2` in N m^-3, which the paper's own line-load rows, its own table note defining `g`, and its own shipped code all contradict. It is a typesetting slip in one cell against correct software, so it is noted here and no errata entry is raised on it.

**Line load, continuous (unbroken) plate (FORM).**
For a line load of magnitude `V0` (or `Q0`) at `x = 0`,
`w(x) = (V0 alpha^3 / 8D) e^(-x/alpha) [ cos(x/alpha) + sin(x/alpha) ]`,
with maximum deflection under the load `w0 = V0 alpha^3 / (8D)`. This is Turcotte & Schubert equation (3-130) (so named in the Sandwell UCSD notes, which derive it by contour integration), and equation (4) in TAFI. The decaying-oscillatory shape gives the flexural moat plus forebulge; the forebulge peak is about 4.3 percent of the central depression.

**Line load, broken (semi-infinite) plate (FORM).**
`w(x) = (V0 alpha^3 / 4D) e^(-x/alpha) cos(x/alpha)`, TAFI equation (5). The broken-plate amplitude is twice the continuous-plate amplitude for the same load, as the notes state.

**Point load, axisymmetric, infinite plate (FORM, the Kelvin-function solution).**
For a point load of magnitude `Q0` at the origin,
`w(r) = Q0 (l^2 / (2 pi D)) kei(r/l)`,
where `kei` is the zeroth-order Kelvin function and `l = [D/((rho_m - rho_w) g)]^(1/4)` is the AXISYMMETRIC flexural length, which is NOT the line-load `alpha` above: `l = alpha / sqrt(2)`, because the factor of 4 belongs to the one-dimensional line-load ODE alone. This is equation (6) in TAFI, given there as the impulse response (Green's function) for `Q0 = 1`, and attributed to Brotchie & Silvester (1969) (with the Kelvin-function machinery to Hertz 1884 and Nadai 1963). TAFI prints equation (6) with the symbol `alpha` and defers that symbol's VALUE to Table 1 row by row, and Table 1's point-load row carries no factor of 4, so reading equation (6) with the line-load `alpha` is a misreading of TAFI rather than a fault in TAFI. `kei(0) = -pi/4`, so the central deflection magnitude is `Q0 l^2 / (8D)`, which in the line-load parameter is `Q0 alpha^2 / (16D)`. It is deliberately NOT structurally parallel to the line-load `w0`, and assuming that parallelism is what welded the two lengths here.

**Sinusoidal (periodic) load (FORM, degree of compensation).**
For a topographic load of wavelength `lambda`, `w0 = e0 / [ 1 + (D/(g rho_c))(2 pi/lambda)^4 + rho_a/rho_c - 1 ]` (GEOL5690 eq. following T&S sec. 3-14; TAFI eq. 7 gives the equivalent), recovering rigid support as `lambda -> 0` and Airy isostasy `w0 = e0 rho_c/(rho_a - rho_c)` as `lambda -> infinity`.

**Primary citation.** Turcotte, D. L. & Schubert, G., Geodynamics, 3rd ed., Cambridge University Press, 2014, ISBN 978-0-521-18623-0, chapter 3 "Elasticity and Flexure"; flexural rigidity in sec. 3-9, plate-on-fluid governing equation sec. 3-13, flexural parameter eq. (3-127), line-load deflection eq. (3-130). Point-load Kelvin-function Green's function: Brotchie, J. F. & Silvester, R., 1969, "On crustal flexure", Journal of Geophysical Research 74(22), 5240-5252, DOI 10.1029/JB074i022p05240.
**Reproductions used (open, equation-numbered).** Jha, S., Harry, D. L. & Schutt, D. L., 2017, "Toolbox for Analysis of Flexural Isostasy (TAFI): A MATLAB toolbox for modeling flexural deformation of the lithosphere", Geosphere 13(5), 1555-1565, DOI 10.1130/GES01421.1 (eqs. 1-7, Table 1), VENDORED as artifact `flexure_tafi` (slim SHA256 `b0733092524e96ce209b564725855deda9493eba31035b2ad8e710e0371a9240`, fetch receipt `4abc31ee08633de676313a600f4d4bf1a34b70ce131dd20dee2d1a078bc8f009`, archived at the Internet Archive 18 July 2026); see `docs/working/VENDORING_CHECKLIST.md` for the full entry, what was kept and what was dropped. Jones, C. H., GEOL5690 class notes "Flexure" (University of Colorado), which cite T&S sec. 3-9, 3-13, 3-14, 3-16 (eqs. 13, 17, 24, 27, 28). Sandwell, D. T., 2001, UCSD/topex geodynamics notes "Flexure of the Lithosphere", which reproduce eq. (3-127) and eq. (3-130) with those T&S numbers.
**Confidence.** High on the line-load and flexural-parameter forms. The point-load form's LENGTH SCALE was CORRECTED (see above): the original "one alpha for both loads" reading was wrong, the point load runs on `l = alpha/sqrt(2)`, settled against the plate ODE and two primaries. High on the corrected form, and the SOURCE question behind it is now closed on vendored bytes rather than on a link: the plate ODE re-derived through two libraries that share no code, McNutt and Menard 1982 eq. A8 read visually at 400 dpi from a freshly refetched PDF whose hash matches the one on record, TAFI Table 1 read visually at 900 dpi from the vendored slim, and TAFI's own published source code. All four agree, and the misreading was ours.

---

## 2. Neukum-Ivanov production function (crater rows, slice 1)

**Lunar production function polynomial (FORM).** The cumulative crater size-frequency production function is an eleventh-degree polynomial in log-diameter:
`log10( N_cum ) = a0 + sum_{k=1..11} a_k [ log10(D) ]^k`,
with `N_cum` the cumulative number of craters per km^2 (at the reference age of 1 Gyr) and `D` the crater diameter in km. Valid over `D = 0.01` to `300` km.

The Neukum et al. (2001) coefficient set (a0 through a11), read from the maintainer-curated craterstats configuration that cites the primary:
```
a0  = -3.0768        (see note below)
a1  = -3.557528
a2  =  0.781027
a3  =  1.021521
a4  = -0.156012
a5  = -0.444058
a6  =  0.019977
a7  =  0.086850
a8  = -0.005874
a9  = -0.006809
a10 =  8.25e-04
a11 =  5.54e-05
```
Note on a0: the craterstats configuration records that the published value is `a0 = -3.0876`, and that the -3.0768 it stores "appears to be a typo"; either way a0 is a vertical normalization superfluous to the shape of the production function and is set by the fitted density, not by the polynomial. Both values are recorded here so the discrepancy is on the record rather than silently resolved.

**Chronology function (FORM).** `N(1, t) = a [ e^(b t) - 1 ] + c t` with `a = 5.44e-14` km^-2, `b = 6.93` Gyr^-1, `c = 8.38e-4` km^-2 Gyr^-1, `t` in Gyr, giving the cumulative number of craters `D >= 1 km` per km^2 as a function of surface age.

**Mars production function (companion, FORM).** Ivanov (2001) recalculates the polynomial for Mars; coefficients (D in km, range 0.015 to 362 km): `[-3.383677, -3.197453, 1.256814, 0.7915374, -0.4860814, -0.3630098, 0.1015683, 0.06755923, -0.01180639, -0.004753462, 0.0006232845, 0.00005805492]`, with Mars chronology constants `a = 2.68e-14`, `b = 6.93`, `c = 4.13e-4` (Ivanov 2001).

**Mars/Moon cratering-rate ratio.** Ivanov (2001) tabulates the Mars-to-Moon impact (bolide) flux ratio as a function of Mars eccentricity; at the current Mars eccentricity of 0.093 the impactor flux ratio is 4.8. The crater-production-rate ratio (which differs from the bolide ratio through the gravity- and velocity-dependent crater-scaling law) is embodied in the differing chronology constants above (Mars `c = 4.13e-4` vs Moon `c = 8.38e-4`, in the Ivanov 2001 chronology). A single scalar "R" for the D >= 1 km crater-production ratio was not isolated to one equation in the accessible text; see the flag.

**Primary citation.** Neukum, G., Ivanov, B. A. & Hartmann, W. K., 2001, "Cratering Records in the Inner Solar System in Relation to the Lunar Reference System", Space Science Reviews 96(1-4), 55-86, DOI 10.1023/A:1011989004263 (production-function polynomial, Table). Ivanov, B. A., Neukum, G. & Wagner, R., 2001, "Size-Frequency Distributions of Planetary Impact Craters and Asteroids", in Collisional Processes in the Solar System, ASSL 261, Springer, DOI 10.1007/978-94-010-0712-2_1 (diameter range). Ivanov, B. A., 2001, "Mars/Moon Cratering Rate Ratio Estimates", Space Science Reviews 96(1-4), 87-104, DOI 10.1023/A:1011941121102 (Mars recalculation and flux ratio). Hartmann, W. K. & Neukum, G., 2001, "Cratering Chronology and the Evolution of Mars", Space Science Reviews 96(1-4), 165-194, DOI 10.1023/A:1011945222010.
**Reproduction used (open, machine-readable, primary-citing).** Michael, G. G., craterstats configuration file `src/craterstats/config/functions.txt` (github.com/ggmichael/craterstats), the "Moon, Neukum et al. (2001)" and "Mars, Ivanov (2001)" blocks, each carrying the coefficients, the diameter range, and the primary reference string. Michael is a coworker of Neukum in the Berlin group.
**Confidence.** High on the polynomial coefficients and the diameter range (read directly from the coefficient file that cites the primary). Medium on the single Mars/Moon crater-production scalar (the bolide flux ratio 4.8 at current eccentricity is cited; the crater ratio is carried by the chronology constants rather than one tabulated number in the fetched text).

---

## 3. Hayashi MMSN (MMSN solar-instance pin, slice 2)

The minimum-mass solar nebula reference disk. Power-law normalizations at 1 AU:

- **Gas surface density (FORM).** `Sigma_gas = 1700 (r/1 AU)^(-3/2)` g cm^-2. Confirmed verbatim through the fetched reproduction and standard across the MMSN literature.
- **Solid (dust) surface density (FORM).** Inside the snow line (rock only): `Sigma_solid = 7.1 (r/1 AU)^(-3/2)` g cm^-2. Beyond the snow line (rock + ice, the ice condensation raising the solid density by a factor ~4): `Sigma_solid = 30 (r/1 AU)^(-3/2)` g cm^-2. The 7.1 rock normalization is confirmed in the fetched reproductions; the ~30 ice+rock value beyond the snow line is the standard Hayashi value (see flag on the exact ice normalization).
- **Temperature profile (FORM).** `T = 280 (r/1 AU)^(-1/2)` K (optically thin radiative equilibrium, `q = 1/2`).
- **Snow line (VALUE).** Water ice condenses where `T < 170` K, which in this temperature profile falls at `r = 2.7` AU.

**Primary citation.** Hayashi, C., 1981, "Structure of the Solar Nebula, Growth and Decay of Magnetic Fields and Effects of Magnetic and Turbulent Viscosities on the Nebula", Progress of Theoretical Physics Supplement 70, 35-53, DOI 10.1143/PTPS.70.35. The gas normalization 1700 g cm^-2 at 1 AU, the solid normalizations, the `T = 280 r^(-1/2)` K temperature law, and the 170 K / 2.7 AU snow line are the model's defining values.
**Reproductions used.** Crida, A., 2009, "Minimum Mass Solar Nebulae and Planetary Migration", ApJ 698, 606, arXiv 0903.5077 (confirms the 1700 g cm^-2 gas normalization and Hayashi framing). Standard-reference confirmations of the 7.1 dust normalization, the 280 K temperature law, and the 170 K / 2.7 AU snow line via the MMSN and frost-line literature.
**Confidence.** High on the gas normalization (1700), the temperature law (280 K, exponent -1/2), the snow-line temperature (170 K) and location (2.7 AU), and the 7.1 rock dust normalization. Medium on the exact numeric of the beyond-snow-line ice+rock solid normalization (commonly quoted as ~30 g cm^-2 at 1 AU scaling, a factor ~4.2 over rock alone); see flag.

---

## 4. Shields curve (hydrosphere fluvial class, slice 8)

**Critical Shields parameter (FORM / dimensionless curve).** The threshold of sediment motion is set by the critical Shields parameter
`theta_cr = tau_cr / [ (rho_s - rho) g D ]`,
the ratio of the critical bed shear stress `tau_cr` to the submerged grain weight per unit area, where `rho_s` is grain density, `rho` fluid density, `g` gravity, and `D` grain diameter (dimensionless). `theta_cr` is a function of the particle (boundary) Reynolds number `Re_* = u_* D / nu` (with `u_* = sqrt(tau/rho)` the shear velocity and `nu` the kinematic viscosity). The Shields curve tends to two limits: a roughly constant `theta_cr ~ 0.06` for fully rough turbulent flow at large `Re_*` (Shields' own high-`Re_*` value), dipping to a minimum of about `theta_cr ~ 0.03` in the transitional regime near `Re_* ~ 10`, and rising again at low `Re_*` (viscous / smooth). The commonly used engineering constant for coarse grains is `theta_cr ~ 0.045` to `0.06`.

**Primary citation.** Shields, A., 1936, "Anwendung der Aehnlichkeitsmechanik und der Turbulenzforschung auf die Geschiebebewegung", Mitteilungen der Preussischen Versuchsanstalt fuer Wasserbau und Schiffbau, Heft 26, Berlin (his Fig. 6, the threshold curve with its data envelope). Standard modern reference for the curve and its `theta_cr(Re_*)` limits: Miller, M. C., McCave, I. N. & Komar, P. D., 1977, "Threshold of sediment motion under unidirectional currents", Sedimentology 24, 507-527, DOI 10.1111/j.1365-3091.1977.tb00136.x; and the definition and value ranges as compiled in the Shields-parameter literature.
**Confidence.** High on the definition and the dimensionless form. Medium on a single pinned numeric: the "0.03 to 0.06" range is the well-established envelope (minimum ~0.03 transitional, ~0.06 fully rough), but Shields' original 1936 curve is graphical and later compilations (Miller et al. 1977; Buffington & Montgomery 1997) place the fully-rough constant anywhere from ~0.045 to ~0.06, so the value is a band, not a single number.

---

## 5. Titan fluvial transport (slice 8, the convicting alien row)

Methane/ethane fluvial transport at Titan surface conditions (~90-94 K, ~1.5 bar), the alien liquid that runs the same Shields machinery on ice grains.

**Liquid densities (VALUE).** Pure liquid methane at Titan surface temperature (~90-94 K): `rho ~ 424` to `450` kg m^-3. Pure liquid ethane: `rho ~ 544` kg m^-3. A representative Titan-lake mixture (25 percent CH4, 70 percent C2H6, 5 percent N2, at 94 K and 0.15 MPa): `rho = 615.2` kg m^-3.
**Liquid viscosity (VALUE).** The same 25/70/5 mixture at 94 K: dynamic viscosity `mu = 547.8` micro-Pa s (`5.478e-4` Pa s). Pure liquid methane near 90-95 K has `mu ~ 2e-4` Pa s (~200 micro-Pa s) from the NIST methane viscosity correlation; the exact pure-methane figure at 94 K is flagged below rather than asserted to a single primary.
**Sediment transport (FORM / application).** Burr et al. (2006) apply the Shields threshold curve (section 4 above) to Titan: non-cohesive water-ice grains entrained by liquid-methane surficial flow, computing incipient-motion shear stress by grain size and settling velocities to classify washload / suspended load / bedload. The transport physics is the terrestrial Shields-Rouse framework re-evaluated with the Titan fluid properties (low `rho`, low `rho_s - rho` contrast for ice in methane) and Titan gravity (1.352 m s^-2), which is the point of carrying the general law rather than an Earth-tuned constant.

**Primary citation.** Burr, D. M., Emery, J. P., Lorenz, R. D., Collins, G. C. & Carling, P. A., 2006, "Sediment transport by liquid surficial flow: Application to Titan", Icarus 181(1), 235-242, DOI 10.1016/j.icarus.2005.09.014. Liquid-property compilations for Titan hydrocarbons: Lorenz, R. D. and collaborators (Cassini-era Titan surface-liquid property work); NIST methane viscosity correlation (the ab-initio-based CH4 viscosity reference). Mixture density/viscosity (615.2 kg m^-3, 547.8 micro-Pa s at 94 K) from the Titan-lake composition modeling in the Cassini-era literature.
**Confidence.** High that Burr et al. (2006) is the primary Titan Shields-transport paper and that ice-in-methane transport uses the Shields curve. Medium on the exact pure-component viscosity at 94 K (the mixture value is cited; the pure-methane figure is a NIST-correlation estimate, flagged).

---

## 6. Canup-Ward circumplanetary-disk moons (slice 7)

**Satellite-system mass ratio (VALUE / scaling).** The total mass of a regular satellite system around a gas giant is regulated to `M_sat / M_planet ~ 1e-4` (of order `10^-4`, weakly dependent on model parameters), set by the balance between satellitesimal inflow/growth and satellite loss to Type-I orbital decay into the planet. The Jovian (Galilean) and Saturnian systems both sit near this ratio (each a few `x 10^-4` down to `~1e-4`), and the scaling is approximately linear in planet mass, so the total satellite mass grows with the host.

**Primary citation.** Canup, R. M. & Ward, W. R., 2006, "A common mass scaling for satellite systems of gaseous planets", Nature 441, 834-839, DOI 10.1038/nature04860.
**Confidence.** High on the `~1e-4` regulated mass ratio and the inflow-versus-Type-I-loss regulation mechanism.

---

## 7. The Hill fraction (slice 7, moon orbital stability)

**Critical satellite semi-major axis (FORM, fraction of the Hill radius).** From N-body integrations, the outer stability limit for a satellite, in units of the planet's Hill radius `R_Hill`, is
`a_crit (prograde)  = 0.4895 ( 1 - 1.0305 e_p - 0.2738 e_sat ) R_Hill`,
`a_crit (retrograde) = 0.9309 ( 1 - 1.0764 e_p - 0.9812 e_sat ) R_Hill`,
where `e_p` is the planet's heliocentric eccentricity and `e_sat` the satellite's eccentricity. For circular orbits (`e_p = e_sat = 0`) this gives `~0.49 R_Hill` prograde and `~0.93 R_Hill` retrograde, the standard result that retrograde satellites are stable to roughly twice the prograde limit. (Earlier estimates put the prograde limit near `~0.5 R_Hill` and the retrograde near `~0.7` to `~1.0 R_Hill`; the Domingos et al. fit is the pinned modern form.)

**Primary citation.** Domingos, R. C., Winter, O. C. & Yokoyama, T. (Neto), 2006, "Stable satellites around extrasolar giant planets", Monthly Notices of the Royal Astronomical Society 373(3), 1227-1234, DOI 10.1111/j.1365-2966.2006.11104.x (the two fitted critical-`a` relations). Corroborating: Holman, M. J. & Wiegert, P. A., 1999, AJ 117, 621 (stability limits in binary/hierarchical systems); Hamilton, D. P. & Burns, J. A., 1991, Icarus 92, 118 (prograde/retrograde asymmetry).
**Confidence.** High on the Domingos et al. coefficients and the circular-orbit values (0.4895 prograde, 0.9309 retrograde). The task's "~0.7 retrograde" is a rounder figure from older work; the fitted primary gives 0.93 for the circular case, with the eccentricity terms bringing it down (e.g. at moderate `e_p` the retrograde limit falls toward ~0.7).

---

## 8. Tidal recession (slice 7, tidal-survival filter)

**Semi-major-axis evolution rate (FORM).** For a satellite raising tides on its planet, the orbital expansion (recession) rate is
`da/dt = 3 (k2 / Q) (m_sat / M_planet) (R_planet / a)^5 n a`,
where `k2` is the planet's degree-2 tidal Love number, `Q` the tidal quality factor, `m_sat` and `M_planet` the satellite and planet masses, `R_planet` the planet radius, `a` the semi-major axis, and `n = sqrt(G M_planet / a^3)` the mean motion. Equivalently `da/dt proportional to (k2/Q) a^(-11/2)`. The tidal-survival filter is that a satellite inside a critical distance recedes (or, inside synchronous/corotation, decays) on a timescale set by `k2/Q`.

**Anchor values (VALUE).** Present lunar recession rate: `da/dt = 3.82 +/- 0.07` cm yr^-1 (lunar laser ranging). Earth effective tidal parameters at the present dominant (semidiurnal) frequency: `k2 ~ 0.30`, effective `Q ~ 12` (the low effective Q is enhanced by ocean-shelf dissipation, not solid-body). Moon: `k2 = 0.0242`, `Q ~ 38 +/- 4`. Earth solid-body `Q ~ 280 +/- 60` for comparison.

**Primary citation.** Murray, C. D. & Dermott, S. F., 1999, Solar System Dynamics, Cambridge University Press, ISBN 978-0-521-57597-4, chapter 4 (tidal evolution, the `da/dt` form). Recession rate: Williams, J. G. & Boggs, D. H., 2016, "Secular tidal changes in lunar orbit and Earth rotation", Celestial Mechanics and Dynamical Astronomy 126, 89-129, DOI 10.1007/s10569-016-9702-3 (and Dickey, J. O. et al., 1994, Science 265, 482, DOI 10.1126/science.265.5171.482, for the LLR 3.8 cm yr^-1). Lunar `k2`, `Q`: Williams, J. G. et al., 2014, "Lunar interior properties from the GRAIL mission", JGR Planets 119, 1546, DOI 10.1002/2013JE004559.
**Confidence.** High on the `da/dt` proportionality and the 3.82 cm yr^-1 present lunar recession anchor. High on the lunar `k2`/`Q` and Earth effective `k2`/`Q`, with the caveat that the terrestrial effective `Q ~ 12` is an ocean-dominated present-day value and is frequency- and epoch-dependent (it was different in the past), so it is an anchor, not a constant of the physics.

---

## 8b. Tidal heating (moon interior dissipation, tidal-heating slice)

**Heat-production rate (FORM).** The tidal heat dissipated inside a synchronously rotating moon on an eccentric orbit is
`E_dot = (21/2) (k2 / Q) (G M_planet^2 R_moon^5 n e^2) / a^6`,
where `k2` and `Q` are the MOON's (the tidally deformed secondary's) degree-2 Love number and quality factor, `R_moon` the moon radius, `M_planet` the primary (planet) mass, `n = sqrt(G M_planet / a^3)` the mean motion, `e` the eccentricity, `a` the semi-major axis; the result is in watts. Using `G M_planet = n^2 a^3` to eliminate `G` and one power of `M_planet` gives the G-free equivalent `E_dot = (21/2) (k2 / Q) M_planet R_moon^5 n^3 e^2 / a^3`, the form the kernel evaluates (unit-agnostic in the same G-free spirit the section-8 recession rate uses; the two forms agree to <1e-9 relative on a numeric check). Because the heat spans about 10^0 to 10^15 W across the plausible moon population, far past the Q32.32 ceiling, the kernel carries it as `log10(E_dot / W)`, a weighted sum of logs with no exponentiation. This is DISTINCT from the recession rate of section 8: recession raises tides on the PLANET (the planet's `k2`/`Q`/`R`), heating dissipates in the MOON (the moon's `k2`/`Q`/`R`), so the two consume different bodies' parameters.

**Anchor values (VALUE).** The `21/2` is the standard algebra of the small-eccentricity, degree-2, constant-Q expansion, not an authored parameter (the counterpart of the `3` in the recession form). Lunar `k2 = 0.0242`, `Q ~ 38` (section 8). Satellite quality factors (Goldreich and Soter 1966): `Q_moon` in 10..150, `Q_venus <= 17`; whole-Earth effective `Q` 12..34 (Yoder 1995). Io is the order-of-magnitude anchor: with Io's orbit (`a = 4.217e8` m, `e = 0.0041`, Jupiter `M = 1.898e27` kg, `R = 1.822e6` m) and a fiducial `k2/Q ~ 3e-4`, `E_dot ~ 1.9e12` W (`log10 ~ 12.3`); the observed Io heat flow of about 1e14 W is reached with Io's higher effective `k2/Q`, so the FORM and MAGNITUDE are anchored while `k2` and `Q` stay the moon's reserved-with-basis data.

**Primary citation.** Murray, C. D. & Dermott, S. F., 1999/2005, Solar System Dynamics, Cambridge University Press, ISBN 978-0-521-57597-4, chapter 4 (the detailed derivation), with Peale, S. J. & Cassen, P., 1978, "Contribution of tidal dissipation to lunar thermal history", Icarus 36, 245-269, DOI 10.1016/0019-1035(78)90109-4, and Peale, Cassen & Reynolds 1979, "Melting of Io by tidal dissipation", Science 203, 892-894, DOI 10.1126/science.203.4383.892 (the Io application).
**Open reproduction (witness).** The `21/2` form is reprinted verbatim as Equation (1) of Henning, W. G., O'Connell, R. J. & Sasselov, D. D., 2009, "Tidally Heated Terrestrial Exoplanets: Viscoelastic Response Models", ApJ 707, 1000, arXiv:0912.1907v1, which identifies every variable's body (`k2`, `Q`, `R_sec` the secondary's; `M_pri` the primary's, squared). This witness is VENDORED in the modern registry as `sources/registry.toml` id `henning_2009_tidal_heating` (custody witness: sha256 `3db06bf4fd28826472b5a12d1135618e043e91a57497550cc7b520c24321d9bc`, 636621 bytes, byte-verified against the durable Internet Archive capture `20240430060355`), where the full receipt, extract, and licence finding live. This brief is the form record; the registry is the witness of record.

**Scope.** The fixed-Q, homogeneous, spin-synchronous, small-eccentricity leading term. A body far from synchronous rotation, at high eccentricity, or with a strongly frequency- or temperature-dependent `Q` (the viscoelastic regime Henning et al. treat past their Eq. 1) departs from this baseline; those are named follow-on rungs. The result is the heat PRODUCTION rate; coupling it to the moon's thermal state, surface heat flux, or habitability needs a moon thermal substrate and is a further rung, not this slice.

**Confidence.** High on the form and the `21/2` coefficient (dual-channel read from the byte-verified witness, cross-checked by the G-free reduction and the Io order-of-magnitude anchor). The magnitude for any specific moon is only as sound as its reserved `k2` and `Q`, which are the moon's own data.

---

## 9. Composition-draw dispersions (per-system abundance draw, conditional chain)

The per-system elemental-abundance draw is a conditional chain ordered by nucleosynthetic causality, never a product of independent marginals: independent draws would author a zero-correlation structure that the measured high-alpha thick-disk sequence contradicts. The chain conditions on ENVIRONMENT and EPOCH, never on stellar mass (the natal cloud is upstream of the IMF), so FGK-measured distributions legally serve draws for stars of any mass, M dwarfs included. Two disciplines are folded in throughout: (A) the local distributions are magnitude-limited, spectral-type-selected, kinematically-biased detected samples, so the selection-corrected (volume-complete / bias-corrected) form is used where a source gives one, else the selection function is recorded as part of the conditioning; and (B) the measured tails are carried with their uncertainty bands and never clipped, because the rare draws (the carbon worlds) are the reason for carrying a distribution rather than a mean.

### 9.1 The environment axis (outermost conditioning link)

The [Fe/H] draw conditions first on the chemical environment. The local Milky Way thin-plus-thick disk is one instance, the tagged default pin; the other Galactic populations are the other values of this axis, and they convict any local-only version by showing abundance combinations the local sample barely contains.

- **Local disk (thin + thick), the default pin.** See 9.2 and 9.3.
- **Population II halo (VALUE / plane).** Halo field stars at low metallicity separate into a high-alpha sequence (constant `[alpha/Fe] ~ +0.3`, from rapid Type-II enrichment) and a low-alpha sequence (`[alpha/Fe]` declining with `[Fe/H]`, from slow evolution with Type-Ia iron), over `-1.6 < [Fe/H] < -0.4`; the low-alpha population is preferentially retrograde and s-process/heavy-element poor. Primary: Nissen, P. E. & Schuster, W. J., 2010, "Two distinct halo populations in the solar neighborhood", Astronomy & Astrophysics 511, L10, DOI 10.1051/0004-6361/200913877; and Nissen & Schuster 2011, A&A 530, A15, DOI 10.1051/0004-6361/201116619 (Mn, Cu, Zn, Y, Ba, the s/r contrast). Sample: 94 dwarfs.
- **Galactic bulge (VALUE / MDF).** A wide, multi-peaked metallicity distribution with high `[Fe/H]` coexisting with high `[alpha/Fe]`, and the alpha-knee at a slightly higher `[Fe/H]` than the local thick disk, combinations the local disk sample barely spans. Microlensed-dwarf MDF peaks at `[Fe/H] = -1.09, -0.63, -0.20, +0.12, +0.41`. Primary: Bensby, T. et al., 2017, "Chemical evolution of the Galactic bulge as traced by microlensed dwarf and subgiant stars. VI.", Astronomy & Astrophysics 605, A89, DOI 10.1051/0004-6361/201730560; corroborating MDF: Ness, M. et al., 2013 (ARGOS III), MNRAS 430, 836, DOI 10.1093/mnras/sts629.
- **Dwarf galaxies / Magellanic Clouds (VALUE / plane).** Lower mean `[Fe/H]` and an alpha-knee at markedly lower `[Fe/H]` than the Milky Way (slow, inefficient star formation lets Type-Ia iron enter early), so `[alpha/Fe]` is lower at `-2 < [Fe/H] < -1` while still reaching the `[alpha/Fe] = 0.3-0.5` plateau below `[Fe/H] ~ -2`. These low-alpha, distinct-abundance regimes are the ones the carbide-condensation work shows can flip mineralogy classes outright. Primary: Tolstoy, E., Hill, V. & Tosi, M., 2009, "Star-Formation Histories, Abundances, and Kinematics of Dwarf Galaxies in the Local Group", Annual Review of Astronomy and Astrophysics 47, 371-425, DOI 10.1146/annurev-astro-082708-101650.

### 9.2 Local [Fe/H] metallicity distribution function, conditioned on epoch

- **MDF and dispersion (VALUE).** The recalibrated Geneva-Copenhagen survey (largest kinematically-unbiased solar-neighbourhood sample) shifts the MDF peak to near the solar value, with an intrinsic scatter in the age-metallicity relation of `sigma_intrinsic ~ 0.20` dex that is real and present at all ages; the thin-disk age-metallicity relation is nearly flat (weak epoch conditioning within the thin disk, strong thin-vs-thick separation).
- **Selection (discipline A).** The Geneva-Copenhagen sample is magnitude-limited but kinematically unbiased, so its MDF approximates a volume sample better than most; for an explicitly target-selection-corrected MDF, the SEGUE G/K-dwarf MDF (corrected for the SEGUE selection function) is the reference. Both the HARPS FGK and APOGEE local samples reproduce a peak slightly below solar, a concordance check across selection functions.
- **Primary citation.** Casagrande, L. et al., 2011, "New constraints on the chemical evolution of the solar neighbourhood and Galactic disc(s). Improved astrophysical parameters for the Geneva-Copenhagen Survey", Astronomy & Astrophysics 530, A138, DOI 10.1051/0004-6361/201016276, arXiv 1103.4651. Selection-corrected local MDF: Schlesinger, K. J. et al., 2012, "The Metallicity Distribution Functions of SEGUE G and K Dwarfs", ApJ 761, 160, DOI 10.1088/0004-637X/761/2/160.
- **Confidence.** High on the near-solar MDF peak, the ~0.20 dex intrinsic scatter, and the flat thin-disk AMR. High that the selection-corrected reference is Schlesinger et al. 2012.

### 9.3 The [alpha/Fe]-[Fe/H] plane with the knee (two-branch, not a Gaussian)

- **Plane structure (FORM, bimodal).** The solar-neighbourhood disk splits into two sequences in `[alpha/Fe]` at fixed `[Fe/H]`: an old, alpha-enhanced (thick-disk) branch with a high-alpha plateau near `[alpha/Fe] ~ +0.3` at sub-solar `[Fe/H]`, turning over (the alpha "knee") near `[Fe/H] ~ -0.4` and declining toward `[alpha/Fe] ~ +0.1` at solar metallicity; and a younger, low-alpha (thin-disk) branch near `[alpha/Fe] ~ 0.0` to `+0.05`. The bimodality (a gap between the branches, not a single spread) is the load-bearing structure: it is what independent marginals cannot reproduce. The separation sharpens when cool stars (`Teff < 5400 K`) are excluded.
- **Selection / tails (disciplines A, B).** The 714-star sample is a kinematically selected solar-neighbourhood FGK sample; the two-branch morphology is robust to the selection, and the high-alpha branch is the tail that a Gaussian would erase.
- **Primary citation.** Bensby, T., Feltzing, S. & Oey, M. S., 2014, "Exploring the Milky Way stellar disk. A detailed elemental abundance study of 714 F and G dwarf stars in the solar neighbourhood", Astronomy & Astrophysics 562, A71, DOI 10.1051/0004-6361/201322631, arXiv 1309.2631.
- **Confidence.** High on the two-branch (bimodal) structure and the high-alpha plateau / low-alpha level. Medium on the exact `[Fe/H]` of the knee (~-0.4 is the standard reading of their figures; the precise per-element knee is in their Figs. rather than a single tabulated scalar).

### 9.4 The C/O distribution with its modern tail treatment

- **Peak (VALUE).** The stellar C/O distribution in the solar neighbourhood peaks (median) at `C/O ~ 0.47` (849 F/G/K dwarfs), with the Sun at `C/O = 0.55`. The frequency of true carbon-rich (`C/O > 1`) dwarfs is very low, under `~0.13` percent in that sample.
- **Tail treatment (discipline A, load-bearing).** The high-C/O tail above `C/O ~ 0.8-1.0` has a known history of being inflated then revised down and must be applied knowingly, because the tail sets the carbide-branch firing rate. Earlier catalogs found `C/O > 0.8` in 25-30 percent and `C/O > 1.0` in ~6-10 percent of stars; Fortney (2012) showed these were overestimated (an over-high adopted solar C/O in differential analysis, and a Ni blend biasing the O abundance), inconsistent with the `<10^-3` frequency of dwarf carbon stars, and argued the true high-C/O fraction is nearer `1-5` percent. The tail is carried with this downward correction and its band, not clipped and not taken raw from the older catalogs.
- **Primary citation.** Brewer, J. M. & Fischer, D. A., 2016, "C/O and Mg/Si Ratios of Stars in the Solar Neighborhood", The Astrophysical Journal 831(1), 20, DOI 10.3847/0004-637X/831/1/20, arXiv 1608.06286 (peak/median C/O, carbon-rich frequency). Fortney, J. J., 2012, "On the Carbon-to-Oxygen Ratio Measurement in Nearby Sun-like Stars: Implications for Planet Formation and the Determination of Stellar Abundances", The Astrophysical Journal Letters 747(2), L27, DOI 10.1088/2041-8205/747/2/L27, arXiv 1201.1504 (the tail-inflation critique and the 1-5 percent revised high-C/O fraction).
- **Confidence.** High on the 0.47 peak, the 0.55 solar value, and the Fortney tail critique with the 1-5 percent revised fraction.

### 9.5 Solar-twin thorium variation (the s/r-mix / radiogenic axis)

- **Th variation (VALUE).** In a sample of 14 solar twins and analogs, thorium abundance spans `0.6` to `2.5` times solar (59 to 251 percent of solar), and the `Th/Si` ratio varies by a factor `~2.6`. Because the r-process fraction sets Th and U, this scatter propagates to the per-system radiogenic-heat budget (Th + U supply 30-50 percent of Earth's present radiogenic heat), feeding the young-planet thermal state and dynamo verdicts downstream. This is a physics axis (heat production), not a compositional flavor.
- **Primary citation.** Unterborn, C. T., Johnson, J. A. & Panero, W. R., 2015, "Thorium Abundances in Solar Twins and Analogs: Implications for the Habitability of Extrasolar Planetary Systems", The Astrophysical Journal 806(1), 139, DOI 10.1088/0004-637X/806/1/139, arXiv 1505.00280. Corroborating (Th in solar twins): Botelho, R. B. et al., 2019, MNRAS 482, 1690, DOI 10.1093/mnras/sty2643.
- **Confidence.** High on the 0.6-2.5x solar Th spread, the factor-2.6 Th/Si variation, and the 30-50 percent radiogenic-budget contribution.

### 9.6 Epoch conditioning at class grade

- **Note (discipline / scope).** Epoch conditioning is taken at class grade from the age-abundance relations already inside the sources above (Casagrande 2011 for the thin-disk flat age-metallicity relation with real ~0.2 dex scatter; Bensby 2014 and Bensby 2017 for the age separation of the alpha-branches; Nissen & Schuster for the halo age contrast). A full galactic-chemical-evolution model is not owed and is out of scope; the conditioning is the measured age-abundance trend at the grade the class needs.

---

## Flagged / not-found

- **Point-load Kelvin equation number in Turcotte & Schubert.** The point-load `kei` Green's function (section 1) is pinned to TAFI eq. (6) and to Brotchie & Silvester (1969); a specific T&S equation number for the axisymmetric point-load Kelvin solution was not confirmed from a fetchable copy (T&S present the line load with an equation number; the point-load Kelvin form is standardly attributed to Brotchie & Silvester). The coefficient is `Q0 l^2/(2 pi D)` on the axisymmetric length `l`, corrected 2026-07-17 from the `alpha` this line used to carry, and it is solid in that form.
- **Brotchie & Silvester (1969) was not retrieved.** The point-load Kelvin solution's originating primary (JGR 74(22), 5240-5252, DOI 10.1029/JB074i022p05240) sits behind Wiley and was not fetched, so it is cited at one remove throughout. Its content is pinned by two independent witnesses that DO print it and that were read directly, McNutt and Menard 1982 eq. A8 and TAFI Table 1's point-load row, both of which attribute the form to it and agree with each other and with the plate ODE. Fetching the 1969 original would close the last link, and nothing above depends on it.
- **Mars/Moon single crater-production scalar.** The bolide flux ratio 4.8 at current Mars eccentricity is cited (Ivanov 2001), and the crater-production ratio is carried by the differing chronology constants; a single tabulated "R(D >= 1 km)" scalar was not isolated to one equation in the accessible text.
- **Hayashi beyond-snow-line ice+rock solid normalization.** The 7.1 g cm^-2 rock dust normalization and the 1700 g cm^-2 gas normalization are confirmed; the exact numeric of the ice+rock solid normalization beyond 2.7 AU (commonly ~30 g cm^-2 at the 1 AU scaling, a factor ~4.2 over rock) should be read from the Hayashi 1981 text to confirm the precise figure before it is coded.
- **Shields critical value is a band.** `theta_cr` is a curve with a transitional minimum ~0.03 and a fully-rough constant in the ~0.045-0.06 range, not a single number; Shields' 1936 original is graphical.
- **Pure liquid methane viscosity at 94 K.** The Titan-lake mixture density (615.2 kg m^-3) and viscosity (547.8 micro-Pa s) at 94 K are cited; the pure-methane viscosity at 94 K (~2e-4 Pa s) is a NIST-correlation estimate and should be read from the NIST methane viscosity reference for the exact figure.
- **Bensby 2014 knee [Fe/H].** The two-branch structure and plateau/low-alpha levels are solid; the exact knee metallicity (~-0.4) is read from the paper's figures rather than a single tabulated value.
