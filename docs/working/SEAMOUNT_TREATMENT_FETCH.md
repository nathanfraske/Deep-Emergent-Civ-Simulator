# The seamount treatment: what the primaries did with an axisymmetric load

This document is the primary-source fetch that decides `MIDBAND_ARC_AUDIT_PACKET.md` section B6. Slice 3 refused the circular load rather than faking it: `M = -DK` is the line-load form, the axisymmetric case carries `nu/r` against `1/r`, and a 2-D yield surface would be needed if the fibre state were biaxial. **That refusal is confirmed correct by the primaries, verbatim.** But the question B6 raises, whether the arc's own stated primary hindcast target is unserved, turns out to rest on a premise this fetch refutes: **the arc's hindcast row was never computed through the moment-curvature construction at all.**

This file is a verification record only. `GEOTHERM_FETCHES.md` and `TE_CONSTRUCTION_FETCH.md` are not edited here (one writer per doc); what this fetch answers or corrects in them is marked, and the owner decides what they carry forward.

Nothing here is set. Every value below is a cited literature value read from a primary, and the reserved-versus-set question is untouched.

---

## 1. What was read, through which channel, and what was not reached

**READ, PRIMARY, dual-channel: McNutt and Menard (1982).** "Constraints on yield strength in the oceanic lithosphere derived from observations of flexure", Geophysical Journal of the Royal Astronomical Society 71, 363-394. Fetched from `https://topex.ucsd.edu/pub/class/geodynamics/HW7_papers/G_moment_curvature.pdf`, 1,629,722 bytes, 32 pages, SHA256 `f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`, **which matches the hash recorded in `TE_CONSTRUCTION_FETCH.md` byte for byte**. The PDF is a scan of the print original. Every equation below was read twice: by text extraction (`pdftotext`, layout and raw modes) and by a visual read of the page rendered at 230 dpi, with printed page 365 re-rendered at 300 dpi and the one load-bearing defect (section 5.1) re-rendered at 500 dpi. Printed page N is PDF page N minus 362. **Appendix A, printed pages 385-390, is where the seamount treatment lives, and it is the target this fetch was sent for.**

**READ, PRIMARY, dual-channel: Calmant, Francheteau and Cazenave (1990).** "Elastic layer thickening with age of the oceanic lithosphere: a tool for prediction of the age of volcanoes or oceanic crust", Geophysical Journal International 100(1), 59-67, DOI 10.1111/j.1365-246X.1990.tb04568.x. Fetched from `https://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_5/b_fdi_31-32/34824.pdf`, 938,097 bytes, 9 pages, SHA256 `ce4990077613c00453cb228e2143a4fad031b938a234ed9cbed47ecf8abf1fdc` (a new fetch; no hash was on record). Read by text extraction and by visual reads at 200 dpi (printed 64, 65) and 300 dpi (printed 60, Table 1). Printed page N is PDF page N plus 58. **This is the paper whose Table 2 is the seamount row set and whose equation (6) is the arc's `Te = 2.7 sqrt(dt)` hindcast target, so its method section is load-bearing and it turns out to be decisive.**

**READ, PRIMARY, dual-channel: Watts and Burov (2003).** "Lithospheric strength and its relationship to the elastic and seismogenic layer thickness", Earth and Planetary Science Letters 213, 113-131, DOI 10.1016/S0012-821X(03)00289-9. Fetched from `https://www.whoi.edu/cms/files/watts_burov_TeTs_EPSL03_27427.pdf`, SHA256 `e0deedb52c4cee543133f2bf3cbb928974d2077699702760d3835e069591bcb5`, **matching the recorded hash**. Its font encoding is broken as previously recorded (it renders "flexure" as "£exure" and `10^-14` as `10314`), so quoted passages were read visually. **It contributes nothing on the axisymmetric treatment**, which is itself a finding: it treats trenches and continental foreland basins and never solves a seamount load.

**NOT REACHED: Watts and Zhong (2000).** "Observations of flexure and the rheology of oceanic lithosphere", Geophysical Journal International 142(3), 855-875, DOI 10.1046/j.1365-246X.2000.00189.x. **This refines the prior record rather than repeating it.** `TE_CONSTRUCTION_FETCH.md` section 1 recorded it as not reached with the ORA record metadata-only. Unpaywall and OpenAlex now **both** report `is_oa: true`, `oa_status: "bronze"`, with exactly one location, the publisher PDF at `academic.oup.com/gji/article-pdf/142/3/855/6020255/142-3-855.pdf`, and OpenAlex reports `any_repository_has_fulltext: false`. Four routes were tried; all return HTTP 403 behind Cloudflare. **So the paper is nominally free to read and the block is a fetch route rather than a paywall.** Nothing in this document is sourced to it. OpenAlex independently confirms the title correction the prior fetch made: the "the" in "the rheology" is present.

**NOT CONSULTED, and named because they are where the remaining gaps close:** Watts, Bodine and Ribe / Watts et al. (1988), whose figure 19 carries the aspect-ratio curve that section 4 quotes at second hand through Calmant; Cazenave and Dominh (1984), the source of the 3-D estimates Calmant prefers; Ribe (1982), the source of the aspect-ratio bias analysis; Watts et al. (1975), whose solutions Calmant states they used; Brotchie and Silvester (1969), the source of McNutt and Menard's cylindrical outer solution.

---

## 2. How they handle an axisymmetric load

### 2.1 The answer, and a terminology trap that must be cleared first

**McNutt and Menard solve the TRUE AXISYMMETRIC PLATE (modified Bessel functions ker and kei), parameterize at the nodal ring, and do NOT apply the line-load form to an isolated seamount.**

The trap: the paper's Appendix A has a section headed **CYLINDRICAL LOAD**, and that is **not** a "cylindrical approximation" in the sense the decision tree posed. It is an **axisymmetric disc** of radius `R` and height `h`. The paper's name for the two-dimensional case is **RECTANGULAR LOAD**, and it says so in its own words at printed page 385: "Consider a **two-dimensional load** with rectangular cross-section and half-width `L`, height `h`, density `p0`, resting on an elastic plate with flexural parameter `a = 4throot(4D/dp g)`". So of Appendix A's three cases, **one is 2-D (rectangular) and two are axisymmetric (point, cylindrical)**. A reader who greps for "cylindrical" and concludes the paper approximated has inverted the finding.

### 2.2 That `M = -DK` is the line-load form, in the paper's own words

Printed page 385, the Appendix A opening, visually confirmed at 300 dpi (text extraction rendered "M and K" as "k_f and K"; the visual channel has the truth):

> "The expressions for `w`, `w_b`, `x_b`, `M` and `K` given in the main text correspond to the solutions for bending a thin elastic plate under a **line load with infinitesimal thickness in the `x`-direction**. Here we derive the corresponding solutions for other loading cases and demonstrate the similarity in the resulting equations for `M` and `K` when expressed in terms of `w_b` and `x_b`."

**This is the sentence that convicts `M = -DK` (their equation 4) as the line-load form, printed by the authors themselves.** `TE_CONSTRUCTION_FETCH.md` section 6.4 inferred this from the presence of Appendix A's axisymmetric operators; it is stated outright, and the arc's refusal stands on the primary rather than on an inference.

### 2.3 The point-load solution, verbatim

Printed page 387, read at 230 dpi and re-read at 500 dpi:

> "The deflection of a thin elastic plate beneath a point load is
> `w(r) = -(P l^2/2 pi D) kei r/l`   (A8)
> in which `P` is the weight of the load, `l = 4throot(D/dp g)`, and `kei` is a modified Bessel function."

> "The first zero crossing `x_0` occurs at `r/l = 3.91467` (Abramowitz & Stegun 1965)"

`M(x_0) = -0.00704 P`   (A9), and at the peak in the arch `r/l = 4.93181`, `w_b = 0.01122 P l^2/(2 pi D)`, and `x_b = 1.01714` (that is `1.01714 l`, the distance from the nodal ring to the arch crest; printed page 389 writes it `x_b = 1.017 l`).

**They parameterize at the nodal ring, and they say why for the seamount case specifically**, printed page 371: "Likewise for flexure profiles caused by the loading of islands and seamounts, the optimal `x_0` corresponds to the first zero crossing seaward of the load (Fig. 4b). Although axial forces `N` may not be as important in the seamount case, we still require a reliable estimate for `K`. More importantly, inboard of the first zero crossing, sediments, volcanic flows and the load itself add additional moments and obscure the flexure profile, violating the requirement that `w` be an observed quantity." Figure 4's caption names the case: "(b) Seamount loading case."

### 2.4 The cylindrical (disc) load, and why the point load is used instead

Printed page 388, verbatim:

> "Now suppose we consider a cylindrical load of height `h` and radius `R`. Again we must solve for inner and outer solutions, matching the two at the edge of the load. The outer solution is (Brotchie & Silvester 1969)
> `w_o(r) = H(F_3 ker r/l - F_4 kei r/l)`   (A11)"
> with `H = p_0 h/dp`, `F_3 = R/l ber' R/l`, `F_4 = R/l bei' R/l`.

> "it is not possible to cast (A11) exactly into the form of (A8) and show that identical expressions hold for `M` and `K` in terms of `x_b` and `w_b`, as we did for the rectangular load case. Modified Bessel functions such as `ker` and `kei` are not as well behaved as sines and cosines ... Fortunately in the region of the rise, the values for `r/l` are large enough (>4) so that asymptotic expressions for `ker` and `kei` in terms of trigonometric functions are **accurate to within a few per cent**"

The Appendix then shows the point-load and disc asymptotic solutions agree, printed pages 389-390:

> "where `C_p` and `C_c` are the factors which differ in the point load (A13) and cylindrical load (A15) asymptotic solutions, respectively. **Since `C_c` is within 2 per cent of `C_p` for `x > 0`, for our purposes (A13) and (A15) can be considered equivalent, and therefore estimates of `M` and `K` in terms of `x_b` and `w_b` based on the point load approximation also apply to distributed loading geometries.**"

**So the point load is the working solution, the finite-radius disc is shown equivalent to it within 2 per cent at the nodal ring and beyond, and BOTH are axisymmetric.** Appendix B goes further still for the Hawaii volume estimate, printed page 390: "To approximate the case for seamount loading on the oceanic lithosphere, we adopt a **cylindrical coordinate system with no theta dependence**", a thick-plate Hankel-transform solution whose constitutive relations carry the hoop stress `tau_theta theta` explicitly.

### 2.5 The load class is a per-load choice, and the paper says so

Printed page 372, visually confirmed:

> "A similar parameterization exists for the moment at the first zero crossing seaward of seamount loads:
> `M = C_1 w_b x_b^2`   (17)
> `K = C_2 w_b/x_b^2`   (18)
> in which the values for `C_1` and `C_2` **depend upon the geometry**. **For a chain of seamounts that approximate a line load, `C_1` and `C_2` are identical to the values in (14) and (15)** regardless of whether the plate is continuous or fractured beneath the load. **For an isolated seamount with circular geometry,**
> `C_1 = 9.06 x 10^10`   (19)
> `C_2 = -2.58 x 10^-6`   (20)"

with the line-load constants at (14) and (15) being `M(0) = 5.67 x 10^10 w_b x_b^2 (N)` and `K(0) = -3.83 x 10^-6 w_b/x_b^2 (m^-1)`, `w_b` in m and `x_b` in km.

The Appendix also prints three consequences of the geometry, printed page 372: "(1) for a given flexural rigidity `D` and density contrast `dp`, `x_b` and the wavelength of the bulge are virtually independent of the details in loading geometry landward of the first zero crossing; (2) for a constant load volume, `w_b` (and `M`) decrease as the load radius increases; (3) **the reduction in moment compared to the point load approximation becomes appreciable when the radius of the load approaches `(D/dp g)^(1/4)`**."

**CONFIDENCE: HIGH** on every quotation in section 2, each read in two channels.

---

## 3. The moment and curvature relations for the axisymmetric load

Printed page 387, read at 230 dpi and re-read at 500 dpi. **The `nu/r` and the `1/r` are confirmed against the page, and the arc's refusal is vindicated on the primary:**

`M = -D(d^2w/dr^2 + nu/r dw/dr),`
`  = P/2 pi [ker r/l - (1-nu)/(r/l) kei' r/l]`

`K = d^2w/dr^2 + 1/r dw/dr`
`  = (P/2 pi D) (ker r/l + 1/r kei' r/l)`

**So they DO switch forms, and where and why is printed:** the main text's `M = -DK` is the line-load solution (section 2.2), and the POINT LOAD subsection of Appendix A derives the axisymmetric pair. `M` carries `nu/r`, `K` carries `1/r`, and the two differ, so `M = -DK` does not hold for a circular load.

### 3.1 The construction reproduces, and `nu` is proven to sit inside the seamount constant

Every printed constant of the point-load solution was recomputed from scratch (scipy Kelvin functions; `ker(x_0)` cross-checked in mpmath, a second library with a different algorithm):

| Quantity | Recomputed | As printed | |
| --- | --- | --- | --- |
| First zero crossing of `kei` | 3.91467 | 3.91467 | exact |
| Arch peak, `kei' = 0` | 4.93181 | 4.93181 | exact |
| `kei` at the peak | 0.01122 | 0.01122 | exact |
| `x_b` = peak minus zero | 1.01714 | 1.01714 | exact |
| `M` bracket `[ker - (1-nu)/x kei']` at `nu = 0.25` | -0.04420 | -0.04421 (A10) | exact to printed digits |
| `M(x_0)/P` at `nu = 0.25` | -0.007035 | -0.00704 (A9) | exact to printed digits |

**The `nu = 0.25` of Table 1 is provably baked into the seamount constant `C_1`.** The bracket carries `(1-nu)`, so `M(x_0)/P` is -0.007316 at `nu = 0`, **-0.007035 at `nu = 0.25`, which is what the paper prints**, and -0.006753 at `nu = 0.5`. Only `nu = 0.25` reproduces (A9). The spread across that `nu` range is 8.0 per cent in `M`, hence 2.6 per cent in `Te`. `C_1` also carries `dp = 2300 kg m^-3` and `g = 9.8 m s^-2` from the same Table 1. **`C_1 = 9.06 x 10^10` and `C_2 = -2.58 x 10^-6` are therefore not portable constants: they are conditioned on `(nu, dp, g)` and a world with different values cannot use them.**

### 3.2 How large the `M = -DK` error would be for a circular load

**DERIVED HERE, NOT STATED BY EITHER PRIMARY, and graded accordingly.** From their own printed operators at the first zero crossing with `nu = 0.25`, `M/(D K) = -1.136`. So applying the line-load `M = -DK` to a circular load at the nodal ring understates the moment by 13.6 per cent, which is **4.3 per cent in `Te`** since `Te` goes as `M^(1/3)`. This is an order-of-magnitude smaller than the load-class error of section 4.3, and it is offered so the two are not confused.

---

## 4. The fibre state, and whether the published values came through this treatment

### 4.1 The fibre state is UNIAXIAL, in both primaries, and neither states it as a choice

**The words "biaxial", "von Mises", "Tresca" and "hoop" appear NOWHERE in McNutt and Menard.** The yield side of the construction is a scalar throughout.

Printed page 365, visually confirmed at 300 dpi:

> "We assume that the horizontal and vertical stresses, `sigma_h` and `sigma_v`, are **principal stresses** so that `d sigma = sigma_h - sigma_v`."

> `M = integral_0^H sigma_f (z - z_n) dz.`   (3)

> "Since the fibre stresses must sum to zero over the thickness of the plate, in the absence of axial loading `sigma_f = d sigma = sigma_h - sigma_v`."

**One horizontal stress, not two.** The `sigma_f` of equation (3) is a scalar, and Figure 2(a)'s stress axis is labelled `sigma_1 - sigma_3`, the difference between the largest and smallest principal stress, which is a maximum-shear (Tresca-class) scalar measure rather than a 2-D yield surface.

Watts and Burov are the same. Their Figure 2 caption, printed page 116: "Thick solid line shows the **stress difference** for a load which generates a moment, `M`, of `2.2 x 10^17 N/m` and curvature, `K`, of `5 x 10^-6 m^-1`." And printed page 116: "We have based the discussion thus far on a YSE that only considers the stresses generated in a flexed plate by bending. We have not, therefore, taken into account the effect of any in-plane stresses that act on the plate due, for example, to tectonic boundary loads."

**THIS IS A CONVENTION THEY ASSUME AND NEVER STATE, and it is the sharpest one in this fetch.** McNutt and Menard use the **axisymmetric elastic solution** for the geometry, which by construction has two in-plane curvatures and therefore two in-plane fibre stresses (radial and hoop), while keeping a **uniaxial scalar yield envelope** on `sigma_h - sigma_v`. Their own Appendix B constitutive relations print `tau_theta theta` explicitly, so the hoop stress is in the paper. It is absent from the yield integral, and **the inconsistency is unremarked.** No primary reached in this fetch computes what it would cost.

### 4.2 McNutt and Menard's own seamount rows: computed through the axisymmetric treatment, proven by back-solve

Printed page 374: "For a suite of flexure profiles caused by either plate subduction or **seamount loading** we calculated moment, curvature and strain rate using equations (14)-(21)." Table 3, printed page 375, "Island and seamount data", carries three rows. **Every column reproduces exactly with the CIRCULAR constants and fails with the line-load constants:**

| Load | `w_b` (m) | `x_b` (km) | `M` line | `M` circ | **`M` printed** | `K` line | `K` circ | **`K` printed** |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Hawaii | 55 | 60 | 1.12 | **1.79** | **1.8** | 0.59 | **0.39** | **0.4** |
| Great Meteor | 30 | 45 | 0.34 | **0.55** | **0.55** | 0.57 | **0.38** | **0.4** |
| Rarotonga | 50 | 40 | 0.45 | **0.72** | **0.72** | 1.20 | **0.81** | **0.8** |

(moment `x10^16 N`, curvature `x10^-7 m^-1`, the paper's own column units)

**All three used `C_1 = 9.06 x 10^10` and `C_2 = -2.58 x 10^-6`, including Hawaii, which is a chain.** That is deliberate rather than an oversight: Appendix B models "the island of Hawaii, its root and the moat infill" with "a **circularly symmetric** Gaussian load".

Two honest limits sit on this. **McNutt and Menard report moment and curvature for seamounts, not `Te`.** And their own Summary bounds the method: "The method used here is most appropriate for trench profiles with curvatures greater than `10^-7 m^-1`. For lower curvatures, such as along seamount profiles, small errors in the curvature estimate cause large changes in rheological parameters." **All three of their seamount curvatures (0.4, 0.4, 0.8 `x10^-7 m^-1`) sit at or below that stated floor.** The data quality is thin in their own words too: "The amplitudes `w_b` for Hawaii and the Great Meteor seamount represent a **guess** as to the contribution to the arch from seamount loading alone."

### 4.3 The arc's actual hindcast row: NOT computed through this construction at all

**This is the finding that decides B6.** `GEOTHERM_FETCHES.md` section 2 pins the oceanic row to Calmant et al. (1990) equation (6), `Te (km) = (2.7 +/- 0.15) sqrt(dt) (Ma)`, fit over their Table 2. Their method section, printed page 60, visually confirmed:

> "The elastic part of the lithosphere has been modelled using the **thin elastic plate theory**. ... Subject to a vertical force `Q` per unit area, the plate will react by a deflection `w` given by
> `D grad^4 w = Q`   (1)
> where `D` is the flexural rigidity, related to the thickness `Te` of the layer through
> `D = Te^3 E/(12(1 - nu^2))`   (2)"

which becomes, with the buoyancy and infill terms, `D grad^4 w + g w (p_M - p_s) = P`   (3), with the load `P(x, y)` read off the bathymetry `H(x, y)`. And, printed page 59:

> "We used a **3-D model of flexure** constrained by geoid height data from the SEASAT satellite"

and printed page 60:

> "In the SYNBAPS data base (Van Wickhouse 1973), bathymetry is given on `5 min x 5 min` regular grid. The numerical integration of the plate deflection is computed on this grid. The solutions of (3) given by Watts et al. (1975) have been used. For each volcano, the total area of computation is 7-25 square degrees, centred on the volcano. ... **The deflection has been computed on the same grid as the bathymetry for a large variety of `D` values, ranging from `5 x 10^20` to `10^24 N m`.**"

**So Calmant's `Te` is a best-fitting UNIFORM ELASTIC PLATE RIGIDITY.** They scan `D`, integrate the plate equation numerically over the real two-dimensional bathymetry grid, compare the predicted geoid to SEASAT, take the best-fitting `D`, and re-express it as `Te` through equation (2) using an assumed `(E, nu)`. There is **no yield envelope, no fibre stress, no moment integral, no neutral surface, and no curvature evaluation point.** There is also no Bessel idealization: the load is the actual map.

**In Watts and Burov's own notation, Calmant's rows are `Te(elastic)`, not `Te(YSE)`.** The two are different quantities, and Watts and Burov distinguish them explicitly and give the ratio (their equation 2, and the McAdoo ratio of about 0.5 at `K = 10^-6 m^-1`, rising to 1 for `K < 10^-8 m^-1`).

**What "3-D" means in this literature is the LOAD GEOMETRY, not the yield surface**, and Calmant state their preference outright. Printed page 64: "Two sets of estimates are proposed, depending on whether a 2-D (admittance) or a **3-D (as in this paper)** model is used." Printed pages 64-65:

> "Ribe (1982) has provided an extended study of the bias which may occur when the admittance technique is used to determine a value of `Te`. He has shown that the **aspect ratio of the load, `chi`, (`chi = 1` when the volcanic edifice is circular and `chi -> infinity` when it is part of a continuous ridge) is a key factor.** Computations of the admittance function from bathymetric and gravimetric (or geoid height) profiles over the summit of a volcano assume that the load is infinitely long in a direction normal to the profiles (`chi -> infinity`). Thus, for a given plate deflection, this over-estimation of the load results in an **over-estimation of the plate stiffness**."

> "Watts et al. (1988) show (fig. 19 in their paper) how the estimate of the best fitting value of `Te` varies with the aspect ratio assumed for LR1 seamount. **This best fitting value increases from about 11 km at `chi = 1` to about 40 km when `chi -> infinity`.** ... Hence, for the southernmost isolated peaks, **the aspect ratio seems to account for most of the difference between the 2-D and 3-D estimates**."

And the decisive sentence, printed page 65:

> "Hence, according to the previous discussion, **we have preferred to use the 3-D estimates of `Te` to infer ages from relation (6)**."

**The literature already carries a row-method field.** Calmant's Table 3 footnotes label the source of each estimate by method: "2: Cazenave and Dominh, 1984 (**3D estimates**) / 3: Cazenave and Dominh, 1984 (**2D estimates**) / 4: Watts et al., 1988 / 5: This study".

**The size of the load-class penalty, in the literature's own numbers, is a FACTOR OF 3.6** (about 11 km against about 40 km for the same seamount). That is the number that dominates this fetch. It is reported at **SECOND-HAND grade**: it is Calmant reading Watts et al. (1988) figure 19, and Watts et al. (1988) was not fetched.

Table 2 is close to method-homogeneous. Printed page 62: the two imported rows (Crozet and Valerie Guyot) "were studied previously by Cazenave et al. (1980) and Cazenave & Dominh (1984) **using a similar method**". Contrast Watts and Burov's oceanic compilation, printed page 116, which is not: "The oceanic `Te` estimates plotted in Fig. 1 are based on **more than 48 individual studies**."

**CONFIDENCE: HIGH** that Calmant's `Te` is an elastic-`D` fit with no yield envelope, read verbatim in two channels from their method section. **HIGH** on the 3-D preference and the per-row method labels. **MEDIUM** on the 11 km to 40 km figure, which is accurate as a reading of Calmant but is second-hand to Watts et al. (1988).

---

## 5. Defects found, proven from the papers' own equations

Both were checked in three channels and are errors in the PUBLISHED PAPERS rather than extraction artifacts, which is why they are reported rather than silently corrected.

### 5.1 McNutt and Menard's `K(x_0)` does not reproduce, and the gap propagated into `C_2` (equation 20)

The page prints, at 500 dpi and unambiguously, twice:

`K(x_0) = (P/2 pi D) (-0.0289).`   and   `K(x_0) = (-0.0289/0.0112) w_b/x_b^2`, "from which we obtain (18) and (20)."

**Applying the paper's own printed definition of `K` to the paper's own printed (A8) gives 0.0389, not 0.0289.** Verified two independent ways:

- **Analytic route.** `K = d^2w/dr^2 + (1/r) dw/dr` applied to `w = -(P l^2/2 pi D) kei(r/l)` gives `K = -(P/2 pi D)[kei'' + kei'/x]`, and the Kelvin identity `kei'' + kei'/x = ker` (checked numerically) reduces it to `-(P/2 pi D) ker(x_0)`. `ker(3.91467) = -0.0388994`, agreeing to seven digits between **scipy and mpmath**, two libraries with different algorithms.
- **Numerical twin.** Finite differences taken directly on the printed profile, reusing no algebra, give `K(x_0) = +0.006191 = 0.0389/(2 pi)`, stable across `h = 1e-3`, `1e-4` and `1e-5`. The same finite differences return `M(x_0) = -0.007035`, reproducing the printed `-0.00704 P` exactly, so the routine is sound and the disagreement is isolated to `K`.

**The control rules out a method error on my side.** The line-load constants (14) and (15) reproduce by the identical method: `M(0) = 5.667 x 10^10` against a printed `5.67 x 10^10`, and `K(0) = -3.827 x 10^-6` against a printed `-3.83 x 10^-6`. And every other point-load constant on the same page reproduces to all printed digits (section 3.1).

**The consequence propagates into a published constant and into published data.** `-0.0289/0.0112 = -2.580`, which is exactly equation (20)'s `C_2 = -2.58 x 10^-6`, so (20) follows from the 0.0289. Substituting the value this fetch derives, `-0.0389/0.0112 = -3.473`, gives `C_2 = -3.47 x 10^-6`, **which equals the independent first-principles value computed from `ker(x_0)` and `kei(peak)` without touching the printed intermediate.** Since section 4.2 proves all three of McNutt and Menard's published seamount rows used `C_2 = -2.58 x 10^-6`, **their published seamount curvatures are about 26 per cent low, and a correct `C_2` would raise them by 34 per cent.**

**What I could NOT determine: which slip produced 0.0289.** No natural reading at `x_0` yields it. The Laplacian gives 0.0389, the radial term alone 0.0460, the hoop term alone 0.0071, the `M` bracket 0.0442, and the printed right-hand side read literally 0.0318. Reported, not attributed. Note separately that the printed second line `= (P/2 pi D)(ker r/l + 1/r kei' r/l)` is not equal to its own left-hand side under (A8): the correct substitution is `-(P/2 pi D) ker(r/l)`.

**A consumer that codes `C_2 = -2.58 x 10^-6` inherits this.** The rule that follows is the same one the arc already applies elsewhere: recompute the constant from the construction, never copy the printed number.

### 5.2 An unexplained 2 per cent in `C_1` (equation 19)

From (A10) with the paper's own printed numbers, `|C_1| = dp g x 0.04421/0.01122 = 22540 x 3.940 = 88,800`, that is `8.88 x 10^10`, against a printed `9.06 x 10^10`. A 2.0 per cent gap. It sits inside the paper's own declared few-per-cent tolerance band and may be an unprinted intermediate rounding, but the line-load (14) reproduces to 0.05 per cent by the same route, so a 2 per cent gap here is anomalous. **Reported, unresolved.** Note (A10) substitutes `x_b^2` for `l^2` directly where `x_b = 1.01714 l`, silently dropping 3.5 per cent, and that step does not close the gap in either direction.

### 5.3 Calmant's Table 1 prints an `E` its own paper refutes

Printed page 60, Table 1, "Numerical values of geophysical parameters", read at 300 dpi and confirming the text extraction:

| Parameter | Value as printed |
| --- | --- |
| Sea water density `p_w` | 1.03 |
| Load `p_v` / Infill `p_s` / Crust `p_c` | 2.8 |
| Mantle `p_M` | 3.4 |
| Mean gravity `g` | 9.81 m/s^2 |
| Crustal thickness `t_c` | 6 km |
| **Young's modulus `E`** | **10^12 N/m^2** |
| **Poisson's ratio `nu`** | **0.5** |

`E = 10^12 N/m^2` is 1000 GPa, more than twelve times the 80 GPa that McNutt and Menard and Watts and Burov both assume, and stiffer than diamond. **The paper's own numbers refute it.** Their scanned `D` range is printed as `5 x 10^20` to `10^24 N m`. Pushed through their own equation (2) with the printed pair, that range maps to `Te` from **1.65 km to 20.8 km**. But their own Table 2 lists Mayotte at `40 +/- 4 km`, with Kauai, Madeira and Bermuda at 32.5 km. **The printed pair cannot produce the paper's own table.**

Read as `10^12 dyn/cm^2`, which is `10^11 N/m^2` and the classic value in the older flexure literature Calmant draw their solutions from, the same `D` range maps to `Te` from **3.56 km to 44.8 km**, which brackets Table 2's span of about 4 to 40 km. **The value 10^12 is right and the printed unit N/m^2 is wrong: it is dyn/cm^2.** This is the same shape as, and the third instance of, the unit defects `TE_CONSTRUCTION_FETCH.md` section 5 found in this corpus (equation 6's 600 for 0.6, and the `10^-6` for `10^6 N m^-2`).

**`nu = 0.5` is NOT settled by this route and I could not settle it.** The back-solve is insensitive to it at the precision available. It is load-bearing at the 7.7 per cent level in `Te`, since `((1-0.25^2)/(1-0.5^2))^(1/3) = 1.077`.

**The consequence for the arc is direct.** At fixed `D`, `Te(Calmant at 10^11 Pa, nu = 0.5) / Te(at 8 x 10^10 Pa, nu = 0.25) = 0.862`. **So the arc's primary hindcast row set sits about 14 per cent below a standard-`(E, nu)` `Te` scale**, if the CGS reading and `nu = 0.5` both hold. Under the printed pair the factor would be 0.400, which is the measure of how much rides on resolving it.

---

## 6. Conventions the sources assume and do not state

Per the standing rule that a quantity quoted without its convention is a statistic with a hidden conditioning variable:

**6.1 The fibre state is uniaxial while the geometry is axisymmetric, and nobody remarks on it.** Section 4.1. The elastic solution has two in-plane curvatures; the yield integral has one scalar `sigma_f`. Appendix B's own constitutive relations carry `tau_theta theta`. This is the convention the decision tree was sent to find, and it is assumed rather than argued.

**6.2 (A8) is written in the opposite sign convention to the main text's declared one.** The main text, printed page 365, states "`w` is positive upward". But `w(r) = -(P l^2/2 pi D) kei(r/l)` with `kei(0) = -pi/4` puts the plate **up** under the load, so (A8) is a `w`-positive-downward form. Corroborating it from inside the paper: printed page 387 gives `w_b = +0.01122 P l^2/(2 pi D)` while printed page 389 gives `w_b = -0.0116 P/(pi dp g a^2)` for the same quantity, agreeing in magnitude to 3 per cent and **opposite in sign**. The authors carry magnitudes and assign signs by inspection. The final signs of (19) and (20) are self-consistent with (14) and (15), so this bites a consumer who re-derives rather than one who copies.

**6.3 `C_1` and `C_2` are conditioned on `(nu, dp, g)`, not on geometry alone.** Section 3.1 proves `nu = 0.25` is inside `C_1`, and Table 1's `dp` and `g` are inside it too. They are printed as though they were geometric constants.

**6.4 "3-D" in the seamount literature means the load geometry, not the yield surface.** Calmant's 3-D model is purely elastic. A reader who takes "3-D" to imply a three-dimensional stress state has imported a meaning the source does not carry.

**6.5 The two primaries print the bending moment in different units.** Watts and Burov's Figure 2 caption gives `M = 2.2 x 10^17 N/m`; McNutt and Menard's Tables 2 and 3 give moment in `N`, which is the dimension of `integral sigma (z - z_n) dz`. A notation difference for the same physical quantity, low severity, recorded so a consumer reading both does not insert a spurious length.

---

## 7. Disagreement between the primaries, reported and not averaged

**7.1 `(E, nu)`.** McNutt and Menard print `(8 x 10^10 N m^-2, 0.25)` and Watts and Burov print `(80 GPa, 0.25)`; the two agree. **Calmant print `(10^12 N m^-2, 0.5)` and disagree with both.** This is not a quibble between distant papers: **Calmant is the arc's row set and McNutt and Watts are the arc's construction**, so this is a direct row-against-construction conflict, and it is the concrete instance of the hazard `TE_CONSTRUCTION_FETCH.md` section 6.3 named in the abstract.

**7.2 The load treatment.** McNutt and Menard idealize (point-load Bessel, or a disc shown equivalent within 2 per cent). Calmant integrate the plate equation numerically over the real bathymetry grid. Both admit the axisymmetric load and neither uses a line-load approximation for an isolated seamount, but they are different procedures and their `Te` values are not interchangeable at better than the tolerances each declares.

**7.3 What `Te` is.** McNutt and Menard and Watts and Burov compute a moment-equivalence `Te(YSE)` from a yield envelope at a curvature. Calmant fit a uniform-elastic `D`. **These are different quantities**, and Watts and Burov are the ones who say so: `Te(YSE)/Te(elastic) = (M_YSE/M_elastic)^(1/3)`, a ratio of 1 for `K < 10^-8 m^-1` and about 0.5 at `K = 10^-6 m^-1`.

---

## 8. What could NOT be verified

**8.1 Watts and Zhong (2000) was not read** (section 1). Bronze open access per two independent aggregators, sole location Cloudflare-blocked, no repository copy. Nothing here rests on it.

**8.2 The 11 km to 40 km aspect-ratio figure is SECOND-HAND.** It is a verified reading of Calmant reporting Watts et al. (1988) figure 19. Watts et al. (1988) was not fetched, so the curve itself, its plate age, and its assumed `(E, nu)` are unverified. **The factor of 3.6 is the number this fetch leans on hardest and the one whose primary was not reached.**

**8.3 Calmant's `nu = 0.5` is unresolved** (section 5.3). The `E` misprint is convicted by the paper's own `D` range and Table 2; `nu` is not reachable by that route. It is worth 7.7 per cent in `Te`.

**8.4 Which slip produced McNutt and Menard's 0.0289 is unknown** (section 5.1). The discrepancy and its consequence are proven; its cause is not.

**8.5 The 2 per cent in `C_1` is unexplained** (section 5.2).

**8.6 The cost of the uniaxial assumption is UNMEASURED in this corpus.** No primary reached here computes a biaxial fibre state for a flexed plate, so no source says what the uniaxial envelope costs for an axisymmetric load. **This is a gap, not a finding of "small".** It is reported rather than estimated.

**8.7 McNutt and Menard's seamount rows sit below their own stated method floor** (section 4.2). Their three curvatures are 0.4 to 0.8 `x10^-7 m^-1` against a stated floor of `10^-7 m^-1`, and their `w_b` values for two of the three are called a guess in the paper. Whether their seamount moments are usable at all is the authors' own open question, and this fetch does not close it.

**8.8 Where the yielding correction sits at seamount curvatures was NOT read.** Watts and Burov print the ratio at `K < 10^-8` (equal to 1) and at `K = 10^-6` (about 0.5). The seamount curvatures of section 4.2 fall at 4 to 8 `x10^-8`, between the two printed points. **No primary prints a value there, and interpolating on a log axis between two points is not a reading.** Flagged rather than filled.

---

## 9. The decision tree's answer

The tree offered two branches. **The evidence takes neither, because the premise both branches share is false for the arc's actual row.**

**On the literal question, it is NOT the cylindrical approximation.** McNutt and Menard solve the true axisymmetric plate: the Bessel point-load deflection (A8), the moment carrying `nu/r`, the curvature carrying `1/r`, parameterized at the nodal ring, with the finite-radius disc shown equivalent within 2 per cent. Their own word for the two-dimensional case is "rectangular", and their Appendix A opening states outright that the main text's `M = -DK` is the line-load solution. Their three published seamount rows all used the circular constants, proven by back-solving every column. And Calmant, whose Table 2 is the arc's row set, used a 3-D numerical plate model and wrote: "we have preferred to use the 3-D estimates of `Te`", pricing the 2-D alternative at about 11 km against about 40 km for the same seamount. **So the ruling's first branch cannot fire: there is no cylindrical approximation in the rows to ship the same as.**

**It is NOT biaxial either.** No primary treats the fibre state as biaxial. There is no 2-D yield surface, no von Mises, no Tresca, and no hoop stress anywhere in the yield integral. The measure is the scalar differential stress `sigma_1 - sigma_3` on a single horizontal stress `sigma_h`, in both McNutt and Menard and Watts and Burov. **So the ruling's second trigger, "ONLY IF their treatment is biaxial does the 2-D yield surface become the work", DOES NOT FIRE.** Building a von Mises-class fibre state to match these rows would ship a model the rows never used, which is the silent method mismatch the ruling exists to prevent, running in the opposite direction.

**And the premise underneath both branches is refuted.** The ruling's load-bearing sentence is "HINDCAST FIDELITY BEATS THEORETICAL SUPERIORITY WHEN THE ROWS WERE COMPUTED THROUGH THEIR TREATMENT". **The arc's primary hindcast rows were not computed through the moment-curvature treatment.** Calmant's `Te = 2.7 sqrt(dt)` is fit over `Te` values obtained by scanning a uniform elastic plate's rigidity `D` from `5 x 10^20` to `10^24 N m`, integrating `D grad^4 w + g w (p_M - p_s) = P` numerically over the real bathymetry, matching the SEASAT geoid, and re-expressing the best-fitting `D` through `D = Te^3 E/(12(1-nu^2))`. **No yield envelope, no fibre stress, no moment integral, no neutral surface, no curvature point.** In Watts and Burov's own vocabulary these rows are `Te(elastic)`, not `Te(YSE)`.

**So what the evidence supports, stated plainly:**

The seamount hindcast does not need the axisymmetric yield-envelope machinery, because the target never used one. **The comparison the rows license is in RIGIDITY SPACE**, which is what slice 3 already emits canonically (`D_eq`, per audit packet A5), and A5's reasoning is confirmed from an independent direction by this fetch: Calmant's rows are `D` fits, and their `Te` is that `D` re-expressed through an assumed pair. The hindcast is therefore: derive `D`, convert to `Te` through **Calmant's** `(E, nu)`, compare. That makes `(E, nu)` a **mandatory row field**, and section 5.3 shows why it cannot be defaulted: the pair the arc would assume moves `Te` by 14 per cent under the charitable reading of Calmant's table and by a factor of 2.5 under its printed one.

The row-method field the ruling called for is still the right instrument, and the literature already has one: Calmant's Table 3 labels each estimate 2-D or 3-D. The fields the evidence requires are **load class** (isolated-circular against chain-line-load, which McNutt and Menard make a per-load choice and Calmant price at a factor of 3.6), **`(E, nu)`**, and **whether the row is `Te(elastic)` or `Te(YSE)`**, which is the one that reclassifies the arc's primary target.

**What this fetch does NOT settle, and exactly what would settle it.** Whether the uniaxial envelope is adequate for an axisymmetric load is **unmeasured in this corpus** (section 8.6). No source reached computes it, so the honest position is that the biaxial question is open on the physics and moot on the hindcast: moot because the rows carry no yield envelope to mismatch, open because the arc's forward solve does carry one. If the owner wants it closed on the physics rather than deferred, the sources that would close it are outside these three, and the cheaper prior step is section 8.8: the seamount curvatures sit at 4 to 8 `x10^-8 m^-1`, in the decade where Watts and Burov's ratio is near 1, so **the yielding correction the biaxial question refines may be small at exactly these loads**. That is a reason to measure the ratio there before building a yield surface for it, and it is not a claim that it is small, which no primary prints.

---

**Primary citations.** McNutt, M. K. and Menard, H. W., 1982, "Constraints on yield strength in the oceanic lithosphere derived from observations of flexure", Geophysical Journal of the Royal Astronomical Society 71, 363-394 (the fibre-stress definition and equation 3 on p. 365; equations 14-20 and Table 1 on p. 372; the nodal-ring choice for seamounts and Fig. 4b on p. 371; the data statement on p. 374; Table 3 on p. 375; Appendix A on pp. 385-390, with the line-load statement on p. 385, the point load and equations A8-A10 on p. 387, the cylindrical load and A11 on p. 388, the 2 per cent equivalence on pp. 389-390; Appendix B on pp. 390-391; the method floor in the Summary). SHA256 `f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`. Calmant, S., Francheteau, J. and Cazenave, A., 1990, "Elastic layer thickening with age of the oceanic lithosphere: a tool for prediction of the age of volcanoes or oceanic crust", Geophysical Journal International 100(1), 59-67, DOI 10.1111/j.1365-246X.1990.tb04568.x (the 3-D model statement on p. 59; Table 1, equations 1-4' and the `D` scan on p. 60; Table 2 on p. 62; the 2-D/3-D discussion and Table 3 with its method footnotes on p. 64; the aspect-ratio bias, the 11-to-40 km figure and the 3-D preference on p. 65). Read from `https://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_5/b_fdi_31-32/34824.pdf`, SHA256 `ce4990077613c00453cb228e2143a4fad031b938a234ed9cbed47ecf8abf1fdc`. Watts, A. B. and Burov, E. B., 2003, "Lithospheric strength and its relationship to the elastic and seismogenic layer thickness", Earth and Planetary Science Letters 213, 113-131, DOI 10.1016/S0012-821X(03)00289-9 (the Fig. 2 caption, the in-plane-stress caveat and the 48-study compilation on p. 116). SHA256 `e0deedb52c4cee543133f2bf3cbb928974d2077699702760d3835e069591bcb5`.

**Named and NOT read.** Watts, A. B. and Zhong, S., 2000, GJI 142(3), 855-875, DOI 10.1046/j.1365-246X.2000.00189.x (bronze open access, sole location Cloudflare-blocked, no repository copy). Watts, A. B. et al., 1988 (the source of the 11-to-40 km aspect-ratio curve, reported at second hand). Cazenave, A. and Dominh, K., 1984 (the source of the 3-D estimates Calmant prefer). Ribe, N., 1982 (the aspect-ratio bias analysis). Watts, A. B. et al., 1975 (the solutions Calmant state they used). Brotchie, J. F. and Silvester, R., 1969 (McNutt and Menard's cylindrical outer solution).

**Confidence.** HIGH that the treatment is the true axisymmetric plate rather than a cylindrical approximation, read verbatim from Appendix A in two channels and independently confirmed by back-solving all three of the paper's own published seamount rows onto the circular constants. HIGH that `M` carries `nu/r` and `K` carries `1/r`, read at 230 and 500 dpi. HIGH that the fibre state is uniaxial in both primaries, established by the printed definition of `sigma_f` on a single `sigma_h` and by the total absence of "biaxial", "von Mises", "Tresca" and "hoop" from the yield formulation. HIGH that Calmant's `Te` is a uniform-elastic `D` fit with no yield envelope, read verbatim from their method section. HIGH on the `K(x_0)` defect, whose correct value is confirmed by two libraries and by a numerical twin, and whose control case reproduces exactly. HIGH that Calmant's printed `E` is refuted by their own `D` range against their own Table 2. MEDIUM on the factor-of-3.6 load-class penalty, accurate as a reading of Calmant but second-hand to Watts et al. (1988). MEDIUM that Calmant's `E` is `10^12 dyn/cm^2`, which is a back-solve and a plausibility argument rather than a printed statement. LOW on `nu = 0.5`, which is printed but unexplained and unconfirmed. NOT ASSESSED: the cost of the uniaxial assumption, which no source reached computes.
