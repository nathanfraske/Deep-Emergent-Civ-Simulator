# The T_e construction: verification against the primaries

This document is the primary-source fetch on the elastic-thickness construction that `GEOTHERM_FETCHES.md` section 2.5 recorded and then tagged against itself. That section states that `Te` is "the thickness of the equivalent elastic plate that reproduces the observed flexure, that is a geometric analogue of the lithosphere's integrated strength", and closes with the admission: "This statement is standard in the flexure literature and is the reading of Watts 2001; it was not read verbatim from the primary and is listed under 'Defaults taken'." Its Defaults-taken item 7 repeats the tag. **The construction has now been read from two primaries, dual-channel.** The statement's SUBSTANCE is verified from Watts's own text. Its ATTRIBUTION to Watts (2001) is not, and the sentence that circulates in that form traces to a different author (section 8.3).

This file is the verification record only. `GEOTHERM_FETCHES.md` is not edited here (one writer per doc); its section 2.5 and its Defaults-taken items 7 and 9 are answered by this file, and the owner decides what the fetch doc carries forward.

Nothing here is set. Every value below is a cited literature value read from a primary, and the reserved-versus-set question is untouched by this fetch.

---

## 1. How the primaries were reached, and which one was not

The task named two papers. One was reached, one was not, and the construction itself was found in a third that is the direct parent of the first.

**READ, PRIMARY, dual-channel: McNutt and Menard (1982).** "Constraints on yield strength in the oceanic lithosphere derived from observations of flexure", Geophysical Journal of the Royal Astronomical Society 71, 363-394. This is the paper that builds the moment-curvature construction, with McNutt as first author, and it PREDATES McNutt (1984), which applies it. Read from the copy at `https://topex.ucsd.edu/pub/class/geodynamics/HW7_papers/G_moment_curvature.pdf` (SHA256 `f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`, 1,629,722 bytes, 32 pages). The PDF is a scan of the print original. Because OCR silently alters exponents and subscripts, and because this construction lives exactly there, **every equation below was read twice: by text extraction (`pdftotext`, both layout and raw modes) and by a visual read of the page rendered at 230 dpi**, with the two load-bearing defects re-rendered at 500 dpi. Where the channels disagree, the disagreement is reported rather than resolved silently (section 5).

**READ, PRIMARY, dual-channel: Watts and Burov (2003).** "Lithospheric strength and its relationship to the elastic and seismogenic layer thickness", Earth and Planetary Science Letters 213, 113-131, DOI 10.1016/S0012-821X(03)00289-9. Read from the publisher's PDF at `https://www.whoi.edu/cms/files/watts_burov_TeTs_EPSL03_27427.pdf` (SHA256 `e0deedb52c4cee543133f2bf3cbb928974d2077699702760d3835e069591bcb5`, 19 pages, printed pages 113-131, complete). This is **A. B. Watts writing as first author**, the author of the 2001 book the arc wanted, on precisely the question the arc asks. It prints the moment-equivalence as a solved equation for `Te`, which McNutt and Menard leave implicit. It is a digital typeset PDF rather than a scan, but its font encoding is badly broken (it renders "flexure" as "£exure", `1.3x10^-6 m^-1` as "1.3U1036 m31", and `±` as "R"), so it was read visually as well, at 220 dpi.

**NOT REACHED: McNutt (1984).** "Lithospheric flexure and thermal anomalies", Journal of Geophysical Research 89(B13), 11180-11194, DOI 10.1029/JB089iB13p11180. **This paper is closed and has no open copy.** The negative is authoritative rather than a failure to look: Unpaywall reports `"is_oa": false`, `"has_repository_copy": false`, `"oa_locations": []`, `"oa_status": "closed"`; Semantic Scholar reports `openAccessPdf` status `CLOSED` and its abstract elided at the publisher's request; Wiley returns HTTP 403 behind Cloudflare to both curl and the fetch path; and it is absent from the two course archives that hold its siblings. **Nothing in this document is sourced to McNutt (1984).** What is known of it is reported in section 8.1 at its true grade.

**NOT REACHED: Watts (2001).** *Isostasy and Flexure of the Lithosphere*, Cambridge University Press, ISBN 0-521-62272-7 / 0-521-00600-7 (the ISBN-13 the task gives, 978-0-521-62272-1, is the same object). The Internet Archive holds a scan, identifier `isostasyflexureo0000watt`, 512 images, whose metadata confirms the edition. It is a lending-library item (`access-restricted-item = true`); its OCR text returns HTTP 401 and 403, and the search-inside endpoints are unreachable. **No sentence in this document is sourced to Watts (2001).**

**NOT REACHED: Watts and Zhong (2000)** (the lower-priority companion). Oxford's own repository record (ORA, uuid `a7e19f62-08aa-467f-aef6-066ead7d2f22`) is metadata-only and carries no file; Oxford Academic returns HTTP 403 behind Cloudflare. Note a citation correction: the true title is "Observations of flexure and **the** rheology of oceanic lithosphere", Geophysical Journal International 142(3), 855-875, DOI 10.1046/j.1365-246X.2000.00189.x. The task's citation drops the "the".

**NOT REACHED: Goetze and Evans (1979)**, "Stress and temperature in the bending lithosphere as constrained by experimental rock mechanics", Geophysical Journal of the Royal Astronomical Society 59, 463-478. This is the ORIGIN of the method: McNutt and Menard describe their own formulation as "an extension of the method of Goetze & Evans (1979)". Oxford Academic returns HTTP 403 to every route tried. Only its abstract was reached, through the fetch path, and it is used once below at ABSTRACT grade and labelled as such.

---

## 2. The construction, as the sources state it

### 2.1 The governing equation and the axial load

McNutt and Menard, printed page 365, equation (1), the homogeneous plate-bending equation:

`d²M/dx² - N d²w/dx² - Δρg w = 0`   (1)

"in which `M` is the bending moment, `x` is the horizontal coordinate, `N` is the axial load (+ for tension, - for compression), `Δρg` is the buoyancy force arising from a density difference between fluids below and above the plate, and `w` is the plate deflection. Consistent with standard usage, `w` is positive upward, `z` is positive downward, and the restoring force `Δρg` is positive for density increasing with depth."

The axial load, equation (2), printed page 365:

`N = ∫₀^H Δσ dz`   (2)

with `H` the plate thickness, `Δσ = σ_h - σ_v`, and `σ_h`, `σ_v` assumed to be principal stresses.

### 2.2 The bending moment of a yield-limited plate (the integral the task asked for)

McNutt and Menard, printed page 365. The source sentence, verbatim: "The moment is defined by the vertical integral of the fibre stresses `σ_f` weighted by the distance from the neutral plane of bending at a depth `z_n`:"

`M = ∫₀^H σ_f (z - z_n) dz`   (3)

**The limits are 0 to H**, that is surface to the base of the plate, where `H` is the depth "at which stress differences become insignificant" (printed page 369). The lever arm is `(z - z_n)` about the neutral plane at depth `z_n`. `σ_f` is bounded by the yield envelope, which McNutt and Menard give in tension and compression separately (section 3).

The sentence that immediately follows equation (3) is the zero-net-axial-force condition, and it is the part a paraphrase drops: **"Since the fibre stresses must sum to zero over the thickness of the plate, in the absence of axial loading `σ_f = Δσ = σ_h - σ_v`."**

### 2.3 The equivalent uniform elastic plate, and the extraction of T_e

McNutt and Menard, printed page 366, for a thin elastic plate:

`M(x) = -D K(x)`   (4)

`D = E T_e³ / 12(1 - ν²) = the flexural rigidity`   (5)

with `E` = Young's modulus, `ν` = Poisson's ratio, and `K = d²w/dx²` = the curvature of the plate. For the purely elastic plate, "we would find from equations (3)-(5) that the base of the plate `H` equals `T_e`."

Watts and Burov print the same construction already solved for `T_e`. Their equation (1), printed page 119, for a thin elastic plate:

`T_e(elastic) = ( M_elastic · 12(1 - ν²) / (E K) )^(1/3)`   (1)

and their equation (2), printed page 120, for the yield-limited plate. The source sentence, verbatim: "In oceanic lithosphere, the equivalent elastic thickness, `T_e(YSE)`, that generates the bending moment `M_YSE` and has the same curvature of an elastic plate, `K`, is:"

`T_e(YSE) = ( M_YSE · 12(1 - ν²) / (E K) )^(1/3) = ( M_YSE / (D₀ K) )^(1/3)`   (2)

"where `D₀ = E/12(1 - ν²)`. Since `D₀` is determined by the elastic properties of the plate and `M_YSE` is determined by depth integration of the YSE, Eq. 2 can be used to calculate `T_e(YSE)` for different values of plate curvature, `K`."

**The two primaries agree exactly.** Rearranging McNutt and Menard's (4) with (5) gives `T_e = [12(1 - ν²)|M| / (E|K|)]^(1/3)`, which is Watts and Burov's (2) term for term. **Where `1/(1 - ν²)` sits, stated exactly:** `(1 - ν²)` is in the DENOMINATOR of `D` (equation 5), and therefore `(1 - ν²)` is in the NUMERATOR of the cube root that yields `T_e` (equation 2). `D₀` is `D` stripped of `T_e³`.

**How `T_e` is extracted, stated exactly.** It is `M_yielded = D(T_e) · K` solved for `T_e`, exactly as the owner's ruling has it, with `D` the flexural rigidity of equation (5). The moment is computed once from the yield envelope by equation (3), the elastic plate is required to carry THAT moment at THAT curvature, and its thickness is read off. Watts and Burov name the licence for this step: "The correspondence principle allows the behaviour of any competent plate (elastic, plastic, or viscous) to be related to that of an 'equivalent' elastic plate." The ratio form is `T_e(YSE)/T_e(elastic) = (M_YSE/M_elastic)^(1/3)`.

**What T_e is, in Watts's own words** (printed page 120, and this is the sentence section 2.5 needed): "Again, `T_e(YSE)` is not the actual thickness of the plate. Rather, it is a 'condensed' thickness that reflects the 'integrated' strength of the flexed, competent, plate." And at printed page 114: "the `T_e` revealed by flexure studies is the depth integral of the bending stress, which is not necessarily associated with a particular competent layer." Watts warns explicitly against the reading section 2.5 warns against: "Watts et al. [8] and Burov and Diament [9] dubbed this layer the elastic 'core' and it has become popular to associate `T_e` with its thickness. It is important to point out, however, that ..." (the sentence just quoted).

**CONFIDENCE: HIGH** on every equation in this section. Equations (1)-(5) of McNutt and Menard were read by text extraction in two modes and by a visual read at 230 dpi; equations (1) and (2) of Watts and Burov by text extraction and a visual read at 220 dpi. The two independent primaries reproduce each other.

### 2.4 The neutral surface

**Where it sits.** Equation (3) places it at depth `z_n` and calls it "the neutral plane of bending". McNutt and Menard's own worked illustration (printed page 366) sets `z_n = 20 km` for `H = 40 km`, that is exactly mid-plate, and their text at printed page 367 refers to "The observation that the neutral axis of bending occurs at mid-plate depths".

**What fixes it.** The condition is printed at page 365 and quoted in section 2.2: the fibre stresses must sum to zero over the thickness of the plate. Read against equation (2), `N = ∫₀^H Δσ dz`, this is the zero-net-axial-force condition, and it is what determines `z_n`. **The construction does require zero net axial force**, and the answer to the task's question is yes.

**INFERENCE, FLAGGED, NOT PRINTED.** Neither primary prints an explicit equation solving for `z_n`. That `z_n` is obtained by imposing `N = 0` follows from the stated condition joined to equation (2), and it is recorded here as a sound inference from what the source prints rather than as a read equation. This is the same grade the H&K verification gave the chord-`V*` "Table 1 means Table 2" reading.

**Whether it MOVES as the plate yields: the sources say yes, indirectly, and neither says so in one sentence.** Three things establish it. First, the yield envelope is strongly ASYMMETRIC between tension and compression (equations 7 and 8, section 3), so a yielded stress profile cannot be symmetric about mid-plate and still integrate to zero; `z_n` must move to satisfy the condition. Second, McNutt and Menard treat `z_n` as a model OUTPUT that responds to the strength profile: under their elevated-pore-pressure envelope, "this model predicts that the neutral axis of bending for 100 Myr old lithosphere lies below 40 km, which is inconsistent with the compressional focal mechanisms at 40 km reported by Chapple & Forsyth (1979)" (printed page 380). A quantity that a change of envelope relocates from mid-plate to below 40 km is not a constant. Third, "mid-plate" is offered as an observation about their adopted envelope, never as a definition. **CONFIDENCE: MEDIUM-HIGH that `z_n` moves, and LOW that either primary states the rule for moving it.** A consumer that pins `z_n = H/2` has adopted an assumption the primaries do not make and that their own asymmetric envelope contradicts.

**Why it is load-bearing arithmetic, in the source's own words.** McNutt and Menard, printed page 380: "the greatest contribution to `M` comes from stress differences at large `(z - z_n)`". The lever arm dominates the integral, so an error in `z_n` is an error in `M`, and `T_e` goes as `M^(1/3)`.

### 2.5 The curvature K

**Definition.** `K = d²w/dx²` (McNutt and Menard, printed page 366, following equation 5). Note this is the deflection's second derivative with no small-slope correction printed.

**Sign convention.** `w` is positive UPWARD and `z` is positive DOWNWARD (printed page 365). The sign of `K` is load-bearing rather than cosmetic: "Other factors being equal, a plate with negative curvature (concave downward) will appear to have a smaller `T_e` than a plate with positive curvature" (printed page 367). Their fitted trench curvatures are negative: equation (15) gives `K(0) = -3.83 x 10^-6 w_b/x_b² (m^-1)` and the worked illustration uses `K = -5 x 10^-7 m^-1`.

**Where it is evaluated: at the FIRST ZERO CROSSING, and NOT at the peak.** This is the answer to the task's question and it does not match the owner's ruling. The summary states the paper parameterizes "flexure profiles in terms of the moment and curvature at the first zero crossing". The reason is printed at page 369: "Ideally, we want (12) to depend only on observable quantities, with no assumptions. For this reason, the optimal choice for `x₀` in parameterizing trench profiles is the first zero crossing seaward of the trench axis. To begin with, because `w(x₀) = 0`, even if axial loading `N` is appreciable, it will not be a factor in equation (12)." For seamounts the same choice is made, for a second reason: "inboard of the first zero crossing, sediments, volcanic flows and the load itself add additional moments and obscure the flexure profile, violating the requirement that `w` be an observed quantity" (printed page 371).

A second, stronger reason is printed at page 371, and it is a numerical-stability argument the arc should weigh: "in both studies the two plate models essentially agree on the curvature estimate at the first zero crossing seaward of the trench axis", where elsewhere "curvature on the outer trench wall varies by a factor of 2" between elastic and elastic-plastic models of the same profile. The first zero crossing is the one place on the profile where the answer does not depend on the rheology you assumed.

**The hazard the primary states about curvature, verbatim** (printed page 369): "While the integration in (12) may be quite stable, the second derivative of `w`, the curvature, is notoriously unstable. Fig. 5 shows some examples of this effect: small changes in the bathymetric curve cause very large changes in curvature." Their own summary bounds the method: "The method used here is most appropriate for trench profiles with curvatures greater than 10^-7 m^-1. For lower curvatures, such as along seamount profiles, small errors in the curvature estimate cause large changes in rheological parameters."

**There is no single T_e for a plate.** Printed page 369: "At each step, the moment for a given rheology is calculated from equation (3) and transformed into an effective flexural rigidity `D_eff` by equation (4). **`D_eff` varies as the curvature changes along the profile.**" A derived `T_e` is a per-point quantity, and reporting one number per load is already a reduction the primary does not make.

**The rheology-independent moment.** Equation (12), printed page 369, which is what makes the observational side work:

`M(x₀) = ∫_{x₀}^{∞} Δρg w(x) (x - x₀) dx + N w(x₀)`   (12)

"which measures the moment at a point `x₀` regardless of rheological assumptions ... By equating the moments from equations (12) and (3), we can use deflection profiles `w` to constrain the stress differences in a vertical cross-section of the lithosphere at the point `x₀`." **This is the inverse of the arc's solve** and the distinction matters (section 7.2).

### 2.6 T_e decreasing with curvature, quoted

Both primaries state it, and both are quoted verbatim.

**McNutt and Menard, printed page 366**, the illustrative calculation. With `H = 40 km`, `z_n = 20 km`, `K = -5 x 10^-7 m^-1`, `Δσ₀ = 500 MPa`: "and calculate the moment in the elastic-plastic plate using equation (3). According to equation (4), a purely elastic plate with the same moment and curvature would be less than 37 km thick. **Thus a lithospheric plate which is more sharply bent will appear thinner than an identical plate with lower curvature if finite yield strength in not taken into account.**" (The "in" for "is" is the paper's own typo; all three extraction channels agree.) The paper's thesis, printed page 364: "the observed `T_e` will be a function of both the curvature of the deformed plate and the magnitude and sign of axial loading forces."

**Watts and Burov, printed page 116**: "Differences persist into flexed lithosphere where **`T_e` slowly decreases with increasing curvature and, hence, bending stress** [11,21] while `T_s` simply reflects the local stress level." And: "Because of the high curvatures that are experienced by the oceanic lithosphere as it approaches a trench, `T_e` is less than it would otherwise be on the basis of plate age because of yielding."

**The quantitative statement, printed page 120**, which is the closest thing to a calibration curve the fetch found: "McAdoo et al. [55] used Eqs. 1 and 2 to calculate the ratio of `T_e(YSE)` to `T_e(elastic)` for a plate of thermal age 80 Ma, an olivine rheology, and a uniform strain rate of 10^-14 s^-1. They showed that for low curvatures (i.e. `K < 10^-8 m^-1`) the ratio is 1, indicating little difference between the elastic thickness values. However, as curvature increases, the ratio decreases as `T_e(YSE)` decreases and the flexed plate yields. **For `K = 10^-6 m^-1` the ratio is ~0.5, indicating 50% yielding and a corresponding reduction in the elastic thickness.**"

| Quantity | Value | Units as printed | Source |
| --- | --- | --- | --- |
| Curvature below which `T_e(YSE)/T_e(elastic) = 1` | `< 10^-8` | m^-1 | Watts and Burov (2003) p. 120, attributed to McAdoo et al. |
| Ratio `T_e(YSE)/T_e(elastic)` at `K = 10^-6 m^-1` | ~0.5 | dimensionless | same |
| Observed curvature range, deep-sea trench-outer rise | `1 x 10^-7` to `1 x 10^-6` | m^-1 | Watts and Burov (2003) Fig. 2 caption |
| Observed curvature range, continental foreland basins | `10^-8` (sub-Andean) to `5 x 10^-7` (West Taiwan) | m^-1 | Watts and Burov (2003) p. 121 |
| Highest reported continental curvature | `4-5 x 10^-6` (Apennine, Dinaride) | m^-1 | Watts and Burov (2003) p. 121, citing Kruse and Royden |
| Method floor, McNutt and Menard's own bound | curvatures `> 10^-7` | m^-1 | McNutt and Menard (1982), Summary |

**A worked data point, printed page 116**: at the northern Chile trench, "curvature `K = 1.3 x 10^-6 m^-1`, `T_e` is `22 ± 2 km`, which is less than the `T_e` of 34 km that these workers expected on the basis of the thermal age of the subducting oceanic lithosphere" (attributed to Judge and McNutt). A 35 percent reduction at a measured curvature.

**CONFIDENCE: HIGH** on every quotation in this section, each read in both channels.

### 2.7 The elastic core, and decoupled layers

The task asked whether the construction assumes a single mechanical layer or admits decoupled layers, and what decoupling does to `T_e`. **It admits them, and the answer is a Kirchhoff sum, not an arithmetic one.**

Watts and Burov, printed page 120: "The significance of `T_e` in the continents is not as clear as it is in the oceans. This is because the continents may comprise more than one brittle layer that is de-coupled from an underlying layer by an intermediate ductile layer. As Burov and Diament [2] have shown for thermally 'young' continental lithosphere, a weak ductile layer in the lower crust does not allow bending stresses to be transferred between the strong brittle layers that 'sandwich' it and this leads to a mechanical de-coupling between them."

Printed page 121: "`T_e` reflects the strength of each elastic layer and the combined strength of all the brittle and ductile layers. **It is not simply a sum of the thickness of these layers (`h₁, h₂ ... h_n`), however, but is given by the following Kirchhoff relation [2]:**"

`T_e(YSE) ≈ (h₁³ + h₂³ + h₃³ ...)^(1/3) = ( Σ_{l=1}^{n} h_l³ )^(1/3)`   (3)

"In the case of equally strong layers (`h₁ = h₂ = h₃ ... = h`): `T_e(YSE) ≈ n^(1/3) h`, which yields `T_e ≈ 1.25h` for two strong layers (`n = 2`)." (`2^(1/3) = 1.2599`; the paper prints 1.25, a rounding.) The consequence is large and counterintuitive: two decoupled 20 km layers give `T_e ≈ 25 km`, not 40 km. "It reflects the integrated effect of all the competent layers that are involved in the support of a load, **including the weak ones**."

For the multi-layer case the curvature dependence is carried by equations (4) and (5), printed page 121: `T_e(YSE) = T_e(elastic) · C(K, T, h₁, h₂ ...)`, "where `C` is a function of the curvature, `K`, the thermal age, `T`, and the rheological structure. A precise analytical expression for `C` is bulky", and the Burov and Diament first-order approximation they print is stated valid only for `10^-9 ≤ K ≤ 10^-6 m^-1` and carries its own authored constants (`T_e(max) = 120 km`, `T_e(min) = 15 km`, and a `K_max` expression). Those constants are NOT recommended for transfer; they are fitted to a "typical" continental case with a 35 km quartz-dominated crust.

**Where T_e is bounded**, from the Fig. 2 caption, printed page 116: "`T_e`, in contrast, could extend from the thickness of the elastic 'core', `T_e(min)`, to the thickness of the entire elastic plate, `T_e(max)`. Both `T_s` and `T_e` depend on the moment generated by the load and, hence, the plate curvature."

**The oceanic case is the single-layer one**, and Watts and Burov say so: "in the case of the oceans, with their single-layer rheology, the BDT just happens to fall approximately mid-way in the strong elastic portion of the lithosphere."

**CONFIDENCE: HIGH** on equation (3) and the decoupling statements, read in both channels. **MEDIUM** on equations (4) and (5), whose constants were read cleanly but which the source itself calls a first-order approximation.

---

## 3. The yield envelope McNutt and Menard integrate

Recorded because equation (3) is meaningless without the `σ_f` that bounds it, and because this envelope carries the two defects in section 5.

**Brittle, printed page 367.** "The shear stress `τ` necessary to overcome static friction on a surface with normal stress `σ_n` is (Byerlee 1968, 1978)"

`τ = 80 + 600 σ_n (MPa).`   (6)   [**AS PRINTED. This is wrong; see section 5.1.**]

Resolved to differential stress at first yielding on faults at 30 degrees to the largest compressional stress:

`Δσ₀ = -2.17 ρgz - 283 MPa   (Δσ < 0)`   (7)

`Δσ₀ = 0.68 ρgz + 89 MPa   (Δσ > 0)`   (8)

"in which `ρgz` is the overburden pressure in units of `10^-6 N m^-2`." [**AS PRINTED. This is wrong; see section 5.2.**]

The asymmetry is the load-bearing part: "Note that the upper plate is significantly stronger in compression as compared to tension."

**Ductile, printed page 368**, two branches. For `Δσ < 200 MPa`, a power law after Goetze (1978) and Evans and Goetze (1979) with `B₁`, `Q_L`, `R = 1.987 x 10^-3 kcal (mol K)^-1`, `n ≈ 3`. For `Δσ > 200 MPa`, a Dorn-law form `Δσ₀ = σ_H (1 - [RT/Q_H ln(B₂/ε̇)]^(1/2))` with `σ_H = 8.5 x 10^9 Pa` and `B₂ = 5.7 x 10^11 s^-1`, and "`Q_H` is slightly larger than `Q_L` to ensure continuity at `Δσ = 200 MPa`". **The exponents and pre-exponentials in equations (9) and (10) did not extract cleanly and were not all recovered by the visual read either; they are listed in section 8.2 as unverified.** The arc does not need them: it takes its creep route from Hirth and Kohlstedt, already verified.

**Semi-brittle is excluded by construction**, printed page 367, and the reason is a fact about the neutral surface: "The observation that the neutral axis of bending occurs at mid-plate depths and the fact the regions of most intense curvature are concave downward conspire to render semi-brittle failure inconsequential to the bending lithosphere."

**Thermal structure, printed page 369**, equation (11): `T(t, z) = T₀ + α/√t z`, "where `T₀` has units K and `α` is the temperature gradient in `K √Myr km^-1`". A linear gradient decreasing as the square root of age.

**The assumed elastic constants, Table 1, printed page 372, verbatim:**

| Parameter | Value as printed |
| --- | --- |
| `Δρ` = (asthenosphere-water) density | 2300 kg m^-3 |
| `g` | 9.8 m s^-2 |
| `ν` | 0.25 |
| `E` | 8 x 10^10 N m^-2 |

Watts and Burov use the same pair: "Elastic moduli, `E` and `ν`, equal 80 GPa and 0.25, respectively" (Fig. 5 caption). This agreement across two independent studies is what makes section 6.3 a finding rather than a quibble.

**The parameterization, printed page 372**, with `w_b` in m, `x_b` in km, `v` in mm yr^-1:

`M(0) = 5.67 x 10^10 w_b x_b² (N)`   (14)
`K(0) = -3.83 x 10^-6 w_b/x_b² (m^-1)`   (15)
`ε̇(0) = 3.07 x 10^-18 v w_b x_b^(-5/3) (s^-1)`   (16)
`M = C₁ w_b x_b²`   (17)
`K = C₂ w_b/x_b²`   (18)
For an isolated seamount with circular geometry, `C₁ = 9.06 x 10^10`   (19), `C₂ = -2.58 x 10^-6`   (20).

---

## 4. The result the construction was built to produce

Recorded because it is the only end-to-end check on the construction that either primary supplies.

McNutt and Menard's Summary: "Saturation of moment at large curvature is interpreted in terms of a depth-dependent yield strength for the lithosphere using relations adopted from laboratory experiments of rock deformation. A comparison of theoretical curves with observed moments indicates that **old oceanic lithosphere has no long-term strength below about 40 km depth**, with no difference between 100 and 165 Myr old crust. Moderate axial loading forces (±200 MPa) can explain most variations in the moment/curvature observations, except in the case of the Kuril Trench which appears anomalous given the age of the crust."

The fit required WEAKENING the laboratory envelope: "moving the base of the yield envelope in Fig. 3 from 70 to 40 km depth" (printed page 376). "The observations point to a lithosphere weaker than the prediction from experimental deformation of rocks." Their favoured explanation is a lower activation energy at geological strain rates than dry-olivine laboratory extrapolations give: "If recent oceanic geotherms are reliable, `Q` in the lower lithosphere must be lower than 100 kcal mol^-1."

**This is a warning the arc must hear.** 100 kcal/mol is about 418 kJ/mol. The arc's creep anchors come from Hirth and Kohlstedt (2003), whose dry-olivine dislocation-creep `E*` is `530 ± 4 kJ/mol` (verified in `HK2003_VERIFICATION.md`, Table 1). McNutt and Menard's flexure observations require an activation energy BELOW ~418 kJ/mol to reproduce the moments. **The arc intends to derive `T_e` from a yield envelope built on the laboratory flow law, and the primary that built this construction reports that the laboratory flow law gives moments "clearly too large to explain most of the data".** That is a 20 percent gap in an exponent. It is reported here, not adjudicated: the paper is from 1982 and the flexure and rheology literatures have both moved, but a hindcast that misses would be reproducing a known 1982 discrepancy rather than finding a new bug.

**The authors flag their own attribution as a choice**, printed page 383, and the caveat matters because it names the same constants the arc's creep anchors carry: "We have assumed that the activation energy (`Q_L` or `Q_H`) in the flow equations (9) and (10) is the only uncertain parameter, but the pre-exponential `B` values and the stress exponent `n` are also empirically determined constants. Although we chose to examine the effect on the yield criteria of `Q` alone because the exponential dependence makes the equations more sensitive to small changes in `Q`, published estimates of `B₁` ..." So the discrepancy is real and its ASSIGNMENT to `Q` is the authors' selection rather than a measurement. The gap could sit in `A`, in `n`, in the geotherm, or in the flexure data.

**Also load-bearing for the isotherm question the arc already closed**, printed page 364: "When plotted as a function of lithospheric age, the estimates of effective elastic thickness (`T_e`) lie between the 300 and the 600 °C isotherms (Fig. 1) according to the thermal plate model of Parsons & Sclater (1977). It is unlikely that this factor of 2 uncertainty in the temperature at the base of the elastic layer is caused solely by errors in the data." Stated in CELSIUS, consistent with `GEOTHERM_FETCHES.md` section 2.4 and with the scope's ruling that the isotherm is dead. This paper is the reason the isotherm is a factor-of-2 quantity: the whole point of the construction is that the scatter is CURVATURE, not temperature.

---

## 5. Two defects in the published paper, proven from the paper's own equations

Both were checked in three channels (`pdftotext -layout`, `pdftotext -raw`, and a visual read at 230 dpi, re-rendered at 500 dpi). **All three channels agree on what the page prints. These are errors in the PUBLISHED PAPER, not OCR artifacts**, which is why they are reported here rather than silently corrected.

### 5.1 Equation (6) is wrong by a factor of 1000

The page prints `τ = 80 + 600 σ_n (MPa).` A friction coefficient of 600 is not physical, and Byerlee's law is nowhere near it.

**The paper's own equations (7) and (8) settle it.** They are the Mohr-Coulomb resolution of (6) onto optimally oriented faults, and that resolution is invertible. For `τ = S₀ + μσ_n` on faults at the optimal orientation, the differential stress at failure is `Δσ = 2(S₀ + μσ₃)/(√(1+μ²) - μ)`. Solving for the printed coefficients:

| Trial | Compression branch | Tension branch |
| --- | --- | --- |
| **Paper as printed, (7) and (8)** | `-2.17 ρgz - 283` | `+0.68 ρgz + 89` |
| `μ = 0.60`, `S₀ = 80 MPa` | `-2.119 ρgz - 282.6` | `+0.679 ρgz + 90.6` |
| `μ = 0.61`, `S₀ = 80 MPa` | `-2.173 ρgz - 285.0` | `+0.685 ρgz + 89.8` |
| `μ = 0.60`, `S₀ = 50 MPa` | `-2.119 ρgz - 176.6` | `+0.679 ρgz + 56.6` |

`μ ≈ 0.6` and `S₀ = 80 MPa` reproduce BOTH printed equations to their printed digits. `S₀ = 50 MPa` is excluded outright (177 against a printed 283). **The self-consistent reading of equation (6) is `τ = 80 + 0.6 σ_n (MPa)`.** The published "600" has lost a decimal point.

**The operative relations are (7) and (8), not (6).** It is the differential-stress envelope that enters the moment integral, and it is internally consistent. So the defect does not propagate into the paper's results. It propagates into any consumer that copies equation (6).

### 5.2 The units note on equations (7) and (8) inverts a sign

The page prints: "in which `ρgz` is the overburden pressure in units of `10^-6 N m^-2`." `10^-6 N m^-2` is a micropascal. The overburden pressure at lithospheric depth is not micropascals, and equations (7) and (8) return MPa. **The self-consistent reading is `10^6 N m^-2`, that is MPa**, which is what the back-solve in section 5.1 assumes and confirms. The published exponent has lost its sign.

### 5.3 Why this matters to this arc specifically

`GEOTHERM_ARC_SCOPE.md` has the Byerlee row BLOCKED on a source conflict, and names the unit trap: "Byerlee's law is in KILOBARS. Reading `tau = 0.5 + 0.6 sigma` as MPa is a silent 100x error." **This fetch adds a third reading to that conflict, and it is not Byerlee's.** McNutt and Menard cite "Byerlee 1968, 1978" for a law whose cohesion is **80 MPa (0.8 kbar)**, where Byerlee (1978)'s high-normal-stress branch, verified in `GEOTHERM_FETCHES.md` section 4.3 from the primary, is `τ = 0.5 + 0.6 σ_n` in kbar, that is **50 MPa (0.5 kbar)**. The friction coefficient 0.6 agrees; the cohesion does not, and it is 60 percent higher. McNutt and Menard do not explain the difference. **A consumer that treats McNutt and Menard's brittle branch as "Byerlee's law" has adopted a 30 MPa cohesion increase that the cited source does not carry.** This is surfaced for the owner, not resolved: it may be a considered choice, a transcription from Byerlee (1968) rather than (1978), or an error, and the fetch cannot tell which without Byerlee (1968).

Note also that McNutt and Menard apply a SINGLE friction branch at all depths, with no low-normal-stress regime. `GEOTHERM_ARC_SCOPE.md`'s finding that Byerlee's universality has a stress floor, and that "the worlds that never reach the universal regime are exactly the small, low-gravity ones", is therefore unaddressed by this construction's brittle branch. The construction does not depend on which friction law fills the slot; `σ_f` in equation (3) is whatever the envelope says.

---

## 6. Conventions the sources assume and do not state

Per the standing rule that a quantity quoted without its convention is a statistic with a hidden conditioning variable, each convention below is one the source USES and does not STATE.

**6.1 The stress sign convention is tension-positive, and it is never written down.** McNutt and Menard define `Δσ = σ_h - σ_v` and assign the STRONG branch (`-2.17 ρgz - 283`) to `Δσ < 0` while stating "the upper plate is significantly stronger in compression as compared to tension". Those two facts are consistent only if compression is NEGATIVE. The back-solve in section 5.1 confirms it: the thrust-faulting case reproduces equation (7) exactly under tension-positive, and reproduces nothing under compression-positive. **Independent corroboration:** Watts and Burov's Fig. 2 axis is labelled "Compression" on the negative side and "Tension" on the positive side, reaching -2000 MPa against +1000 MPa. Both papers use tension-positive; neither says so. A consumer on the geological compression-positive convention will silently swap the strong and weak branches of the envelope, and will get the sign of the `T_e`-versus-curvature asymmetry backwards.

**6.2 `ρgz` enters as a positive magnitude** even under the tension-positive convention for `σ_h` and `σ_v`. Not stated.

**6.3 Every published `T_e` is conditioned on assumed `E` and `ν`.** This is the sharpest convention finding, and it is of exactly the class the arc already named for the age convention. `T_e` is never measured. It is computed from an observed `M` and `K` through equation (2), which contains `E` and `ν`, and both are ASSUMED rather than derived: McNutt and Menard's Table 1 prints `E = 8 x 10^10 N m^-2` and `ν = 0.25` under the heading "Assumed values for physical parameters", and Watts and Burov assume the same pair. Since `T_e ∝ (1/E)^(1/3)`, a hindcast whose engine derives a different Young's modulus for its own lithosphere **is not comparing like with like against the published `T_e` rows.** A `T_e` quoted without its `(E, ν)` is a chord whose endpoints have been dropped, in the same sense the H&K verification found for `V*` and its pressure interval. The compiled hindcast rows the scope calls for should carry an `(E, ν)` field beside the mandatory age-convention field.

**6.4 The moment-equivalence assumes a LINE load, and `M = -DK` is not general.** McNutt and Menard's Appendix A prints, for the axisymmetric point-load case, `M = -D(d²w/dr² + (ν/r) dw/dr)` while defining `K = d²w/dr² + (1/r) dw/dr`. The `ν/r` and the `1/r` differ, so `M = -D K` (equation 4) is the beam or line-load form and does NOT hold for a circular load. This is why equations (19) and (20) give an isolated circular seamount different constants (`C₁ = 9.06 x 10^10`, `C₂ = -2.58 x 10^-6`) from the line-load case. A per-load scalar solve that applies `M = -DK` to a circular load has adopted the line-load geometry without saying so.

**6.5 The strain rate is an input to the envelope, not an output.** McNutt and Menard modify Kirby's envelope "for a strain rate of 10^-16 s^-1", and their equation (16) computes `ε̇` from the observed profile and plate velocity. Watts and Burov's Fig. 2 uses "a uniform strain rate of 10^-14 s^-1" via McAdoo et al., and their Fig. 5 "a fixed background strain rate of 10^-15 s^-1". **Three sources, three strain rates, two orders of magnitude apart, each unstated as a choice.** The arc derives strain rate from the convective timescale, which is a defensible route the literature does not take; the consequence is that the arc's envelope will not be the literature's envelope even with identical flow laws.

**6.6 `H` is defined by a vanishing, not by a boundary.** In equation (3) the upper limit `H` is "the depth at which stress differences become insignificant" (printed page 369). "Insignificant" is not quantified at that site. Section 7.1 is where it does get quantified, and the number is 50 MPa.

---

## 7. Against the owner's ruling: what verifies, and what does not

The ruling was to be verified rather than re-litigated, and disagreements reported. Four of the five clauses verify from the primaries. Two clauses do not, and one of those is load-bearing.

### 7.1 VERIFIED, with a correction to the definition: T_mech

**Ruling:** "`T_mech`, the MECHANICAL thickness, is the crossing of the brittle and ductile curves: the depth extent of strength."

**The primaries do not define it that way, and the two clauses of the ruling name two different depths.** "The depth extent of strength" is right. "The crossing of the brittle and ductile curves" is a different quantity.

McNutt and Menard, printed page 369, verbatim and visually confirmed: "**the base of the mechanical lithosphere, defined here as the depth at which the yield strength at geological strain rates is less than 50 MPa, corresponds to `Q/RT = 60`**. In this study, we use actual observations of lithospheric deflection to constrain the depth, as a function of lithospheric age, at which `Q/RT` falls to 60." That is a STRENGTH THRESHOLD, and the threshold is an authored number (50 MPa) that the paper states outright.

The crossing of the brittle and ductile curves is the BDT, and in this literature the BDT bounds the SEISMOGENIC layer, not the mechanical one. Watts and Burov, Fig. 2 caption: "`T_s` corresponds to the depth of the intersection of the moment-curvature curve with the brittle deformation field, but could extend from the surface, `T_s(min)`, to the BDT, `T_s(max)`."

The depth ordering follows from the geometry of the envelope and is not a matter of opinion: the brittle curve rises with depth and the ductile curve falls with depth, so their crossing is the envelope's strength PEAK; below the crossing the ductile branch keeps falling, and it reaches 50 MPa somewhere DEEPER. **The base of the mechanical lithosphere is therefore systematically DEEPER than the brittle-ductile crossing, and the ruling's definition names a depth shallower than the quantity it wants.** In McNutt and Menard's own fitted result the mechanical base is "about 40 km" for old oceanic lithosphere; in Watts and Burov's Fig. 2 for 80 Ma lithosphere the BDT is drawn well above the base of the plate, with `T_e(max)` extending to ~80 km.

**Recommendation, surfaced not taken:** if the arc wants "the depth extent of strength", the primary's construction is a strength threshold, and the threshold is a value the arc cannot author. Note the arc may not need `T_mech` at all: `T_e` comes out of equation (2) from `M_YSE` and `K` without ever locating a boundary, and `H` in equation (3) only has to be deep enough that the integrand has died.

### 7.2 VERIFIED as physics, NOT VERIFIED as procedure: the curvature evaluation point

**Ruling:** "The construction is a per-load scalar fixed-point solve: trial `T_e` -> elastic deflection -> peak curvature -> recompute `T_e` from the moment integral -> iterate."

The moment-equivalence step is verified exactly (section 2.3). **The PEAK curvature is not what either primary uses.** McNutt and Menard evaluate at the FIRST ZERO CROSSING, and they give reasons that are about the observation rather than the physics: `w(x₀) = 0` removes the axial-load term from equation (12), only observed quantities enter, and elastic and elastic-plastic models agree on the curvature there while differing by a factor of 2 on the outer trench wall.

**This is a real difference, and it is smaller than it looks, for a reason worth stating.** The two constructions run in OPPOSITE directions. McNutt and Menard solve the INVERSE problem: an observed deflection profile is given, and the rheology is constrained from it. The choice of `x₀` is driven by which point of a MEASURED profile is trustworthy. The arc solves the FORWARD problem: the rheology is given and the deflection is derived, so noise in `w` and contamination by sediments do not exist, and the two reasons the primary gives for the first zero crossing do not apply.

**What the primaries DO contradict is the premise underneath "a per-load scalar".** Printed page 369: "`D_eff` varies as the curvature changes along the profile." There is no single `T_e` for a load. The literature `T_e` values the arc will hindcast against are themselves single numbers extracted at one chosen point of a profile, under the reading that a uniform plate reproduces it. So a per-load scalar is defensible as a matching convention, and it is a convention, not a fact about the plate. **If the arc reads the peak curvature where the compiled data read the first zero crossing, the arc will report systematically lower `T_e` than the rows it is scored against**, because `T_e` falls as curvature rises and the peak is the highest curvature on the profile. That is a bias with a known sign, and it is the kind of thing the hindcast should be able to see.

**Recommendation, surfaced not taken:** the evaluation point is a convention and belongs in the compiled row beside the age convention and `(E, ν)`. Which point the arc's forward solve reads is the owner's call; that it must MATCH the point the hindcast row used is not.

### 7.3 VERIFIED: T_e falls as curvature rises, because the moment saturates

**Ruling:** "`T_e` FALLS AS CURVATURE RISES, because more of the real plate yields and the moment saturates."

Verified verbatim from both primaries; see section 2.6 for the quotations. The mechanism clause is verified too. McNutt and Menard's Summary names "Saturation of moment at large curvature", and their Figure 6 plots log(moment) against curvature with theoretical curves that flatten. Watts and Burov: "as curvature increases, the ratio decreases as `T_e(YSE)` decreases and the flexed plate yields".

**One qualification, from Goetze and Evans, at ABSTRACT grade** (the full text was not reached): "The strength curves show that as a first approximation it is better to assume that bending moment is independent of curvature of the plate than to assume that bending moment and curvature are linearly related." Read literally, the saturation is strong enough that at the curvatures of interest `M` is closer to CONSTANT than to `M = -DK`. If that holds, `T_e ∝ (M/K)^(1/3)` tends toward `T_e ∝ K^(-1/3)` at high curvature. **This is a testable prediction the arc's solve should reproduce, and it is flagged at abstract grade rather than trusted.**

**The ruling is incomplete in one respect.** Both primaries state that `T_e` depends on the SIGN of the curvature and on the axial load, not on the curvature magnitude alone. McNutt and Menard, page 364: "the observed `T_e` will be a function of both the curvature of the deformed plate and the magnitude and sign of axial loading forces." Page 367: "a plate with negative curvature (concave downward) will appear to have a smaller `T_e` than a plate with positive curvature." This follows directly from the envelope's tension-compression asymmetry (equations 7 and 8), so any envelope with an asymmetric brittle branch will show it. A solve that reads `|K|` will collapse two physically different cases.

### 7.4 VERIFIED: the moment-equivalence is what the flexure literature means by T_e

**Ruling:** "`T_e`, the ELASTIC thickness, is McNutt's MOMENT-EQUIVALENCE: the uniform elastic plate reproducing the yield-envelope's BENDING MOMENT at a given CURVATURE. This is what the flexure literature and the hindcast data mean by `T_e`."

Verified. Watts and Burov's equation (2) IS this, printed as an equation and named "the equivalent elastic thickness". Watts's own gloss is the one section 2.5 wanted: `T_e(YSE)` "is not the actual thickness of the plate. Rather, it is a 'condensed' thickness that reflects the 'integrated' strength of the flexed, competent, plate."

**One attribution correction, minor.** The construction is McNutt AND MENARD (1982), not McNutt (1984), and the 1982 paper describes its own moment-curvature formulation as "an extension of the method of Goetze & Evans (1979)". McNutt (1984) APPLIES the construction to convert `T_e` to `T_m`. Calling it "McNutt's moment-equivalence" is well-founded; citing it to McNutt (1984) would not be, and citing it to McNutt and Menard (1982) is correct and available.

### 7.5 A clause the ruling does not carry: decoupling

The ruling is silent on decoupled layers. Section 2.7 shows the construction admits them and that `T_e` is then `(Σ h_l³)^(1/3)`. **For a single-layer oceanic lid this is a non-issue and the arc is safe.** For any world whose lid has a weak ductile layer between two strong ones, a solve that integrates equation (3) across the whole column as if stresses transmit through the weak layer will overestimate `T_e` substantially (40 km against a correct 25 km, for two decoupled 20 km layers). This is an alien-admission point rather than a Terran one: whether a lid decouples depends on the world's own thermal structure and its own layer strengths, and the arc derives both.

---

## 8. What could NOT be verified

**8.1 McNutt (1984) was never read, and nothing here rests on it.** The paper is closed (section 1). Its abstract was not reached verbatim either: Wiley returns 403 and Semantic Scholar reports the abstract elided at the publisher's request. What circulates through search summaries, at **SUMMARY-ONLY grade and not fit to code against**, is that `T_e` increases with the square root of age with much scatter; that correcting for finite yield strength "makes `Te` for more sharply bent plates underestimate the true depth `Tm` to the rheological boundary at the base of the high strength mechanical lithosphere"; that after converting `T_e` to `T_m` the mechanical lithosphere is thinner beneath islands and seamounts than beneath lithosphere of similar age flexed at subduction zones; and that McNutt suggested plate age is "reset" by plume activity. The first two are consistent with what McNutt and Menard (1982) prints and was read. **The `T_e`-to-`T_m` conversion itself, which is the specific contribution of the 1984 paper, is NOT verified**, and its `T_m` is presumably the same 50 MPa-threshold mechanical thickness as the 1982 paper's, but that presumption was not checked against the 1984 text.

**8.2 `GEOTHERM_FETCHES.md` section 2.3's attribution of the age-convention finding to McNutt (1984) is NOT verified here.** That section reports, through Calmant et al. (1990): "elastic thickness estimates fit well for 550-600 °C isotherms, whereas they only fit for lower isotherms (350-450 °C) when compared to the age from isochrons", attributed to McNutt (1984). Calmant et al. was read by the prior fetch, so the row is a verified reading OF CALMANT reporting McNutt. **It remains unverified against McNutt (1984) itself**, and this fetch could not close it. The finding it grounds (that a limiting isotherm is a property of the lithosphere joined to an age convention) is not weakened: it is independently supported by what WAS read here, since McNutt and Menard (1982) page 364 records the same factor-of-2 isotherm spread (300 to 600 °C) and attributes it to curvature rather than to temperature.

**8.3 The sentence in section 2.5 is NOT Watts (2001), and its true source is a review.** The formulation that circulates, "Te does not, in general, represent a depth to any boundary within the lithosphere: it is a purely geometric analogue of the integrated strength of the lithosphere", traces to **Kirby, 2014, "Estimation of the effective elastic thickness of the lithosphere using inverse spectral methods: The state of the art", Tectonophysics** (ScienceDirect, paywalled, reached only at search-summary grade). That is a REVIEW, a different class of source from a primary, and it must not be promoted. **The SUBSTANCE of section 2.5's claim is nonetheless verified from a primary written by Watts himself** (section 2.3): "`T_e(YSE)` is not the actual thickness of the plate. Rather, it is a 'condensed' thickness that reflects the 'integrated' strength of the flexed, competent, plate", and "the `T_e` revealed by flexure studies is the depth integral of the bending stress, which is not necessarily associated with a particular competent layer" (Watts and Burov 2003, pp. 120 and 114). **So section 2.5's physics stands on a primary; only its citation was wrong.** Whether Watts (2001) also says it is unknown and unchecked.

**8.4 The rule that positions the neutral surface was not read.** Section 2.4 records the zero-sum condition verbatim and infers from it that `z_n` is fixed by `N = 0`. Neither primary prints that solve. Goetze and Evans (1979), the origin of the method, is the place it would most likely be printed, and it could not be reached.

**8.5 McNutt and Menard's ductile flow-law constants (equations 9 and 10) were not recovered.** The pre-exponentials `B₁`, the activation energies `Q_L` and `Q_H`, and the exponents did not survive extraction, and the visual read did not recover all of them either. Section 3 records only what both channels agree on. The arc does not need them.

**8.6 The internal-consistency oddity in Watts and Burov's Fig. 2 caption.** The caption's thick solid line is drawn for `K = 5 x 10^-6 m^-1`, while the same caption states that the dashed lines at `1 x 10^-7 m^-1` and `1 x 10^-6 m^-1` "bracket the range of observed values at deep-sea trench-outer rise systems". The illustrated case is therefore five times above the top of the range the caption calls observed. Both channels agree on all three digits, so this is what the paper prints. It is recorded rather than corrected, and it touches only the figure's illustrative case.

**8.7 Not verified: whether the 80 MPa cohesion is Byerlee (1968).** Section 5.3 reports that McNutt and Menard's `S₀ = 80 MPa` disagrees with Byerlee (1978)'s `0.5 kbar = 50 MPa` while citing "Byerlee 1968, 1978". Byerlee (1968) was not fetched. The arc's Byerlee row is already BLOCKED pending the primary, and this is one more reading for that adjudication rather than a new blocker.

**8.8 Watts and Zhong (2000) and Watts (2001) were not read** (section 1). The task ranked Watts and Zhong lower priority; it is recorded as not reached.

---

## 9. The bottom line

**The construction is verified and it is not a paraphrase any more.** `T_e` is obtained by computing the bending moment of the yield-limited plate as the depth integral of fibre stress about the neutral surface, `M = ∫₀^H σ_f (z - z_n) dz` (McNutt and Menard 1982, eq. 3), then demanding that a uniform elastic plate carry that same moment at that same curvature, `M = -D K` with `D = E T_e³/12(1 - ν²)` (their eqs. 4 and 5), which solves to `T_e = [M · 12(1 - ν²)/(E K)]^(1/3)` (Watts and Burov 2003, eq. 2). The neutral surface is fixed by requiring the fibre stresses to sum to zero over the thickness. `T_e` falls as curvature rises because the moment saturates. Two independent primaries print the same construction and were each read in two channels.

**The owner's ruling survives on the physics and takes two corrections on the definitions.** The moment-equivalence, the falling `T_e`, the saturating moment, and the attribution to McNutt are all verified. `T_mech` is NOT the brittle-ductile crossing in this literature: it is a 50 MPa strength threshold, and it lies deeper than the crossing, which bounds the seismogenic layer instead. The curvature is read at the first zero crossing in the compiled data, not at the peak, and since `T_e` falls with curvature that mismatch has a known sign.

**The construction's own honest limits, from the sources.** Curvature is "notoriously unstable" as a second derivative. `D_eff` varies along a profile, so a per-load scalar `T_e` is a convention rather than a property. `M = -DK` is the line-load form and does not hold for a circular load. Every published `T_e` carries assumed `E` and `ν` as hidden conditioning variables, exactly as it carries an age convention. And the primary that built the construction reports that the laboratory olivine flow law makes the lithosphere too STRONG to fit the flexure data, by enough that it proposed cutting the activation energy below ~418 kJ/mol against the 530 kJ/mol the arc's own creep anchors carry.

**Primary citations.** McNutt, M. K. and Menard, H. W., 1982, "Constraints on yield strength in the oceanic lithosphere derived from observations of flexure", Geophysical Journal of the Royal Astronomical Society 71, 363-394 (Summary; equations 1-20 on pp. 365-372; Table 1 on p. 372; the brittle zone on p. 367; the mechanical-lithosphere definition and the moment-curvature formulation on p. 369; the curvature-stability discussion on pp. 369-371; the neutral-axis statement on p. 380; Appendix A on pp. 386-388). Read from `https://topex.ucsd.edu/pub/class/geodynamics/HW7_papers/G_moment_curvature.pdf` (SHA256 `f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`). Watts, A. B. and Burov, E. B., 2003, "Lithospheric strength and its relationship to the elastic and seismogenic layer thickness", Earth and Planetary Science Letters 213, 113-131, DOI 10.1016/S0012-821X(03)00289-9 (equations 1-5 on pp. 119-121; Fig. 2 caption on p. 116; the `T_e` gloss on p. 120; the elastic-core warning on p. 114; Fig. 5 caption on p. 122). Read from `https://www.whoi.edu/cms/files/watts_burov_TeTs_EPSL03_27427.pdf` (SHA256 `e0deedb52c4cee543133f2bf3cbb928974d2077699702760d3835e069591bcb5`).

**Named and NOT read.** McNutt, M. K., 1984, "Lithospheric flexure and thermal anomalies", Journal of Geophysical Research 89(B13), 11180-11194, DOI 10.1029/JB089iB13p11180 (closed; no open copy exists per Unpaywall and Semantic Scholar). Watts, A. B., 2001, *Isostasy and Flexure of the Lithosphere*, Cambridge University Press, ISBN 0-521-62272-7 (access-restricted scan at Internet Archive `isostasyflexureo0000watt`). Watts, A. B. and Zhong, S., 2000, "Observations of flexure and the rheology of oceanic lithosphere", Geophysical Journal International 142(3), 855-875, DOI 10.1046/j.1365-246X.2000.00189.x. Goetze, C. and Evans, B., 1979, "Stress and temperature in the bending lithosphere as constrained by experimental rock mechanics", Geophysical Journal of the Royal Astronomical Society 59, 463-478 (abstract only, used once at abstract grade in section 7.3). Kirby, J. F., 2014, Tectonophysics (search-summary grade only, section 8.3).

**Confidence.** HIGH on the moment integral, the flexural rigidity, the moment-equivalence solve, and the extraction of `T_e`, each read from two independent primaries in two channels each, the two primaries reproducing each other. HIGH on `T_e` decreasing with curvature and on the moment saturating, quoted verbatim from both. HIGH on the Kirchhoff decoupling relation and on the zero-net-axial-force condition, read verbatim. HIGH on the two published defects in section 5, each agreed by three extraction channels and each convicted by the paper's own equations. HIGH on the mechanical-lithosphere definition being a 50 MPa threshold, read visually. MEDIUM-HIGH that the neutral surface moves, which is inferred from an asymmetric envelope plus a model-dependent neutral-axis depth rather than read as a statement. MEDIUM on the tension-positive convention, which is not stated by either source and is established by back-solving the printed equations and corroborated by a figure axis. LOW on anything attributed to McNutt (1984), which was not read at all.
