# Hirth and Kohlstedt (2003) Table 1: verification against the primary

This document is the verification step on the candidate olivine creep rows that `GEOTHERM_FETCHES.md` section 1 recorded through a library. That fetch declared BLOCKER 1: "Hirth and Kohlstedt (2003) Table 1 was never read." **The primary has now been read.** Table 1 and Table 2 are transcribed below at source precision, and every library-mediated digit is checked against them.

This file is evidence custody and verification only. It resolves the source-access part of `GEOTHERM_FETCHES.md` BLOCKER 1. It does not admit a creep magnitude, authorize a caller choice, or turn the primary's pressure-conditioned spread into one value.

## How the primary was reached, and what the owner's mirror does and does not hold

The docslib mirror the owner found (`docslib.org/doc/12242645/`) is a **truncated preview**. Its text runs from the abstract to roughly page 85 of the 83-105 chapter and stops inside the oxygen-fugacity paragraph. It contains the sentence "the values of the flow law parameters are summarized in Table 1" and does not contain Table 1. Its `/download/` route returns HTML, not a file. This is the same wall the prior fetch hit, and the prior fetch's account of docslib was accurate.

The primary was reached elsewhere: a course archive at the University of Hawaii SOEST holds the published chapter PDF.

- URL: `https://www.soest.hawaii.edu/earthsciences_archive/FACULTY/smithkonter/GG631/other/HirthKohlstedt_2000.pdf`
- SHA256: `d76c5905bb1cdfb8d2d62a566daa85636e5ef7120ed07c6f174a6c2a150fcba7`, 2,918,350 bytes, 23 pages.
- The filename says 2000 and is misleading. The embedded metadata says: Title "Rheology of the Upper Mantle and the Mantle Wedge: A View from the Experimentalists", Author "Greg Hirth, David Kohlstedt", Keywords "Inside the Subduction Factory, Geophysical Monograph Series, vol. 138, doi:10.1029/138GM06". The page-1 copyright line reads "Inside the Subduction Factory / Geophysical Monograph 138 / Copyright 2003 by the American Geophysical Union / 10.1029/138GM06 / 83". It is the right object: DOI 10.1029/138GM06, pages 83-105.
- The PDF is an ABBYY FineReader OCR of the print original. Because OCR is an intermediary that can silently alter a digit, **both tables were additionally rendered to images at 220-230 dpi and read visually.** The visual read and the text extraction agree character for character on every value below. Table 1 is on printed page 86 (PDF page 4); Table 2 is on printed page 92 (PDF page 10).

Dixon and Durham (2018) was also fetched in full (OSTI `servlets/purl/1609770`, SHA256 `2a41294f4e7ed253950fe589b28eb654d2f4ab517821370bb75923d434079428`), so the Dixon contradiction is settled against both sources directly rather than against a report of them. The UWGeodynamics library was re-fetched and re-read (SHA256 `829831e56ad1ae534834a2a1871700186638653abba2490664634e70226b2a1e`).

## 1. Table 1, verbatim at source precision

The caption reads exactly: **"Table 1: Rheological Parameters for Equation (1)."** Equation (1), from page 86, is

`ε̇ = A σ^n d^-p f_H2O^r exp(αφ) exp( -(E* + PV*) / RT )`

The column headers as printed: `A^a | n | p | r^b | α | E* (kJ/mol) | V* (10^-6 m^3/mol)`.

| (row label as printed) | A^a | n | p | r^b | α | E* (kJ/mol) | V* (10^-6 m^3/mol) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dry diffusion | 1.5×10^9 | 1 | 3 | - | 30 | 375 ± 50 | 2-10 |
| wet diffusion | 2.5×10^7 ^d | 1 | 3 | 0.7-1.0 | 30 | 375 ± 75 | 0-20 |
| wet diffusion (constant C_OH)^c | 1.0×10^6 | 1 | 3 | 1 | 30 | 335 ± 75 | 4 |
| dry dislocation | 1.1×10^5 | 3.5 ± 0.3 | 0 | - | 30-45 | 530 ± 4 | (see Table 2) |
| wet dislocation | 1600 | 3.5 ± 0.3 | 0 | 1.2 ± 0.4 | 30-45 | 520 ± 40 | 22 ± 11 |
| wet dislocation (constant C_OH)^e | 90 | 3.5 ± 0.3 | 0 | 1.2 | 30-45 | 480 ± 40 | 11 |
| dry GBS, T>1250°C | 4.7×10^10 | 3.5 | 2 | - | 30-45 | 600 ^f | (see Table 2)^g |
| dry GBS, T<1250°C | 6500 | 3.5 | 2 | - | 30-45 | 400 ^f | (see Table 2)^g |

The footnotes, verbatim:

- **a** "For stress in MPa, f_H2O in MPa (or C_OH in H/10^6Si) and grain size in μm."
- **b** "Uncertainty in r is correlated with uncertainty in V*"
- **c** "Example calculation for C_OH = 1000 H/10^6Si, d = 10 mm, T = 1400°C, P = 1 GPa, σ = 0.3 MPa: ε̇ = (1.0×10^6)*(0.3)^1*(10,000)^-3*(1000)^1*exp[-(335000+10^9*4×10^-6)/(8.314*1673)] = 7.8×10^-15/s"
- **d** "Value for A is given for r = 1."
- **e** "Example calculation for C_OH = 1000 H/10^6Si, T = 1400°C, P = 1 GPa, σ = 0.3 MPa: ε̇ = (90)*(0.3)^3.5 *(1000)^1.2*exp[-(480000+10^9*11×10^-6)/(8.314*1673)] = 2.5×10^-12/s"
- **f** "The activation energy for GBS is assumed to be that for slip on (010)[100], which changes with increasing temperature [Bai et al., 1991]. The values given here include the effect of temperature on oxygen fugacity."
- **g** "The value for V* is assumed to be the same as that for dislocation creep."

**Units, stated exactly as the source states them, since this is the hazard.** Table 1 prints **no unit for A at all.** The A column header is bare (`A^a`); footnote a carries the input-unit convention instead, and it fixes only the INPUTS: stress in MPa, water fugacity in MPa (or water content in H/10^6 Si), grain size in μm. E* is headed kJ/mol; V* is headed 10^-6 m^3/mol, so a Table 1 V* of 22 is 22×10^-6 m^3/mol, that is 22 cm^3/mol. Any unit string attached to A (for example "MPa^-3.5 s^-1") is a CONSUMER'S DERIVATION from footnote a, never a value read from the table. Nothing was converted in the table above.

The two worked examples in footnotes c and e are the source's own dimensional key, and they resolve the units the header leaves open: **P enters in Pa** (10^9 for 1 GPa), **V\* enters in m^3/mol** (4×10^-6, 11×10^-6), **E\* enters in J/mol** (335000, 480000), R = 8.314, **T in K** (1673 for 1400°C), and **grain size in μm** (10,000 for d = 10 mm). This confirms the unit convention `GEOTHERM_FETCHES.md` section 1.1 states, including its "pressure in Pa" and "grain size in micrometres". That convention is now read from the primary rather than inferred.

## 2. Table 2, verbatim at source precision

Table 1's dry-dislocation and GBS rows do not print a V\*; they defer to Table 2. Caption verbatim: **"Table 2. Determination of Activation Volumes"** (printed page 92).

| Technique | V* (10^-6 m^3/mol) | P range (GPa) | Reference |
| --- | --- | --- | --- |
| Deformation | 23 | 0.2-0.4 | Kohlstedt and Wang [2001]^b |
| Deformation | 13.4 (18)^a | 0.5-1.5 | Ross et al. [1979] |
| Deformation | 14 (18)^a | 0.3-2 | Karato and Jung [2002] |
| Deformation | 14 | 0.3-15 | Karato and Rubie [1997] |
| Deformation | 27 | 0.6-2.0 | Borch and Green [1989] |
| Recovery | 19^c | 10^-4-0.5 | Kohlstedt et al. [1980] |
| Recovery | 14 | 10^-4-2.0 | Karato and Ogawa [1982] |
| Recovery | 6 | 10^-4-10 | Karato et al. [1993] |
| Diffusion (Si) | -2 | 5-10 | Bejina et al. [1997] |

Footnotes: **a** "Higher value is corrected for pressure effect on thermocouple emf." **b** "Also, Wang et al., Activation volume for dislocation creep in olivine (manuscript in preparation)." **c** "Corrected value from Karato [1981]."

## 3. The library-mediated rows, checked row by row

The UWGeodynamics `ViscousRheologies.json` rows were re-read from the library itself and match what `GEOTHERM_FETCHES.md` section 1.2 reports, with one exception noted in 3.4. Checked against the primary:

### 3.1 What MATCHED

**Every pre-exponential A matches exactly.** Library dry dislocation 110000, dry diffusion 1.5×10^9, wet dislocation 1600, wet diffusion 2.5×10^7 against the primary's 1.1×10^5, 1.5×10^9, 1600, 2.5×10^7. Exact.

**Every activation energy matches the primary's central value exactly.** Library 530, 375, 520, 375 against Table 1's 530 ± 4, 375 ± 50, 520 ± 40, 375 ± 75. The prior fetch's independent confirmation of 530 ± 4 and 375 ± 50 through Ohuchi et al. (2015) is correct against the primary, digit for digit including the bands.

**The stress and grain-size exponents match**: n = 3.5 and 1.0, p = 0 and 3.

**The "480 to 530 kJ/mol class" is confirmed and now has an exact provenance.** Those are precisely the three dislocation-creep rows of Table 1: 530 ± 4 (dry), 520 ± 40 (wet), 480 ± 40 (wet at constant C_OH). 480 is not a competing estimate; it is the constant-water-content parameterization of the same wet dislocation creep.

**The library's wet diffusion r = 1.0 is defensible**: the primary prints r = 0.7-1.0 for that row with footnote d, "Value for A is given for r = 1", so A = 2.5×10^7 and r = 1 belong together.

### 3.2 What DIVERGED

**The repository uses Table 1's n = 3.5 ± 0.3.** The prior fetch listed both ± 0.3 and ± 0.5 as circulating, both SUMMARY-ONLY, and could not distinguish their roles. The primary's Table 1 prints **± 0.3**. Page 84 separately uses ±0.5 in a sensitivity illustration about extrapolating two orders of magnitude in stress, before any fit is presented. Several individual experiment fits also carry ±0.5-class bands. Those are parallel contexts rather than the Table 1 recommendation, so they do not supply this manifest field. No claim about an external work repeating another context is authorized without the external-claim release gate.

**Every band is missing from the library.** The library carries bare central values: no ± 4 on 530, no ± 40 on 520, no ± 50 or ± 75 on the diffusion rows, no ± 0.3 on n, no ± 0.4 on r. A consumer reading the library alone cannot know any of these values carries a band, and the wet dislocation E* band (± 40 kJ/mol on 520) sits inside an exponential.

**The melt term is absent from the library entirely.** Table 1 prints α for all eight rows (30 for diffusion, 30-45 for dislocation and GBS). The library has no melt field. Equation (1)'s `exp(αφ)` factor therefore has no representation in the library rows at all. The primary's text (page 94) gives the domain: "For both wet and dry conditions, the data at φ < 0.12 are well described by an exponential relationship ε̇ ∝ exp(αφ), where α is a constant between 25-30 for the diffusion creep regime and between 30-45 for the dislocation creep regime." Note the text says 25-30 for diffusion where Table 1 prints the single value 30, a minor internal narrowing by the table.

**Two library activation volumes are MIDPOINTS of the primary's printed ranges, carried as point values.**

| Row | Library V* | Primary Table 1 V* | Relation |
| --- | --- | --- | --- |
| dry diffusion | 6×10^-6 m^3/mol | **2-10** | 6 is the exact midpoint of 2-10 |
| wet diffusion | 1.0×10^-5 m^3/mol | **0-20** | 10 is the exact midpoint of 0-20 |

The primary prints ranges; the library prints their centres with no band. The wet diffusion case is the sharper one: the primary's range is **0 to 20**, which includes zero, that is the primary does not exclude no pressure dependence at all for wet diffusion creep. The library's 10 presents a value the primary never printed, with a 20-wide interval collapsed to a point, inside an exponential. The primary's own text (page 93) gives the reason for the dry diffusion range: "The comparison of diffusion creep data on samples deformed at 1 atm and 300 MPa gives a value in the range of 2×10^-6 to 10×10^-6 m^3/mol [Kohlstedt et al., 2000]. The range given here reflects uncertainties associated with the correction for cavitation during creep at ambient pressures."

**The wet dislocation V\* diverges from the primary's central value.** Library 2.3×10^-5 (23) against Table 1's **22 ± 11**. The prior fetch called this agreement "within rounding". It is not rounding: the primary prints 22, and 23 is a different digit. 23 does appear in the primary, as Table 2's first row (Kohlstedt and Wang [2001], deformation, 0.2-0.4 GPa). The library's 23 is inside the ± 11 band, so the divergence is immaterial to any result, and it is recorded because the digit does not come from where a consumer would think it does.

### 3.3 The dry dislocation V\*, which the primary does not print

**The library's 6×10^-6 m^3/mol for dry dislocation creep has no counterpart in Table 1, because Table 1 prints no value for it.** The cell reads "(see Table 2)". This is section 4.

### 3.4 A defect in the prior fetch's reading of the library, not in the library

`GEOTHERM_FETCHES.md` section 1.2 states "The wet rows in that library are referenced to `COH = 1000` H/10^6 Si", and section 1.5 raises the matching hazard: "Mixing a `COH`-referenced pre-exponential with a fugacity-referenced exponent is a silent error. The reproduction's rows are `COH`-referenced at 1000 H/10^6 Si."

**The hazard is real and correctly identified as a class. The factual claim about the library is inverted.** The library's field is named `waterFugacity`, with value 1000, and the parameters it carries are the primary's **fugacity-referenced** rows, not the constant-C_OH rows:

| Library row | Library A, E*, V* | Primary row it matches |
| --- | --- | --- |
| Wet Olivine, Dislocation | 1600, 520, 2.3e-5 | **wet dislocation** (A = 1600, E* = 520 ± 40, V* = 22 ± 11, r = 1.2 ± 0.4) |
| Wet Olivine, Diffusion | 2.5e7, 375, 1e-5 | **wet diffusion** (A = 2.5×10^7, E* = 375 ± 75, V* = 0-20, r = 0.7-1.0) |

The primary's constant-C_OH rows (A = 90, E* = 480 ± 40, V* = 11; and A = 1.0×10^6, E* = 335 ± 75, V* = 4) **are not in the library at all.** So the library is internally consistent on its own reading, that `waterFugacity = 1000` means f_H2O = 1000 MPa, which is inside the fugacity range of the primary's Figure 5a (10^2 to 10^4 MPa) and pairs correctly with A = 1600.

The danger is that 1000 is a plausible number in BOTH conventions: the primary's Figure 5b spans OH concentrations of 10^2 to 10^4 H/10^6 Si, and 1000 H/10^6 Si is the asthenospheric value the primary uses throughout ("Olivine in the oceanic asthenosphere contains ~1000 H/10^6Si"). A reader who sees `1000` and reads it as a water content, which is what the prior fetch did, has silently switched parameterization while every number still looks reasonable. **The primary settles which set belongs with a water content of 1000 H/10^6 Si**: its own worked application does exactly that calculation, and it uses the OTHER rows. Footnote e computes at C_OH = 1000 H/10^6 Si with A = 90, E* = 480000, V* = 11×10^-6. The Figure 9 caption states "We use a water content of 1000 H/10^6Si based on the study of Hirth and Kohlstedt [1996]; flow law parameters calculated for a constant water content are listed in Table 1", and Figure 10's profiles at C_OH = 1000 H/10^6 Si use V* = 11×10^-6 m^3/mol.

So: **feeding 1000 H/10^6 Si of water content into the library's wet dislocation row is the exact silent error the prior fetch's hazard describes.** The correct row for that input is A = 90, E* = 480 ± 40, V* = 11, r = 1.2, and it is absent from the library. This is the practical consequence of the finding, and it matters more than the bookkeeping: the two parameterizations differ by a factor of ~18 in A and 40 kJ/mol in E*.

A second, smaller library defect follows from the same place. The library declares its wet dislocation A units as `1 / megapascal ** 3.5 / second` while also carrying r = 1.2. Under footnote a, with f_H2O in MPa, A's dimension must absorb the fugacity term as well, so the dimensionally complete unit is MPa^-4.7 s^-1. The library's declared unit drops the fugacity dimension (the same omission appears on the wet diffusion row, declared MPa^-1 s^-1 with r = 1.0). The prior fetch's own table prints "1600 MPa^-4.7 s^-1", which is the dimensionally correct form under footnote a but is NOT what the library it cites says. The prior fetch's rescaling hazard ("A must be rescaled by 10^(6n), and by the fugacity exponent as well for the wet rows") is correct, and the library's declared units would mislead precisely that rescale.

## 4. The V\* conflict band, logged explicitly

The prior fetch recorded the conflict as: reproduction 6 cm^3/mol against Dixon and Durham's report of H&K's dry olivine V\* as 13-27 cm^3/mol, non-overlapping, unresolved, and load-bearing because V\* sits inside an exponential. Both numbers were verified against the primary, and against Dixon's own PDF.

**Dixon transcribed the primary faithfully.** Dixon and Durham (2018) Table 1, first row, reads: "Various, polycrystalline | Wet | Various | **22 ± 11** | ≤2 | (Hirth & Kohlstedt, 2003)" with a second line "Dry | **13-27**". The wet figure 22 ± 11 is Table 1 of the primary, exactly. The dry 13-27 is **the span of every determination in the primary's Table 2 whose stated P range lies at or below 2 GPa**: 23 (0.2-0.4), 13.4 (0.5-1.5), 14 (0.3-2), 27 (0.6-2.0), 19 (10^-4-0.5), 14 (10^-4-2.0). Minimum 13.4, maximum 27. Dixon's "≤2 GPa" column is the filter that produces his own range, and it reproduces exactly. His rendering of the primary is sound.

**The library's 6 is also in the primary.** It is Table 2's Karato et al. [1993] row: Recovery, V\* = 6, **P range 10^-4 to 10 GPa**. That row's pressure range extends to 10 GPa, which is why Dixon's ≤2 GPa filter excludes it. The only other row Dixon's filter excludes are Karato and Rubie [1997] (0.3-15 GPa) and Bejina et al. [1997] (5-10 GPa).

**So the conflict DISSOLVES as a contradiction and does NOT close as a number.** There is no disagreement between the two sources about what the primary says. Both are correct selections from the primary's Table 2, which prints **nine determinations spanning -2 to 27 ×10^-6 m^3/mol**. They fail to overlap for a reason the primary states in its own text (page 93): V\* is not a constant.

> "In practical determinations of the activation volume it is important to account for the potential change in V\* with increasing pressure. Specifically, because V\*(P) = -(∂lnε̇/∂P)RT + P∂V/∂P, then V\*(P) = -(∂lnε̇/∂P)RT only if ∂V/∂P = 0. However, since V\* apparently decreases with increasing pressure, determining (∂lnε̇/∂P)RT at high pressure underestimates V\*, unless the data are compared to similar data at low pressure."

The 6 is low **because** it was fitted over a pressure range reaching 10 GPa. The 13-27 is high **because** it is the ≤2 GPa subset. The primary predicts the non-overlap. It is a real physical effect, not a transcription error in either source.

**The primary's own guidance does not close the band either, and its pointer is misprinted.** Page 93 continues:

> "In practice, V\* in equation 1 is a 'chord V\*', representing RT[(lnε̇)_P - (lnε̇)_P=0]/P. The values for V\* shown in **Table 1** for which the pressure range extends to 1 atm (i.e., 1×10^-4 GPa) provide a good approximation for the chord V\*."

The reference to "Table 1" is a **misprint in the published paper**. Table 1 has no pressure-range column; Table 2 does. This was checked three ways: layout-mode extraction, raw-mode extraction, and a visual read of the rendered page 93. All three read "Table 1". The sentence describes a column that exists only in Table 2, so it can only mean Table 2. Under that reading, the rows whose P range extends to 1 atm (10^-4 GPa) are the three Recovery rows: **19, 14, and 6**. H&K's own recommended set for the chord V\* therefore **still spans a factor of 3**, and the library's 6 is the lowest member of it. The primary adds, page 92: "In general both the deformation and recovery data are reasonably bracketed using the theoretical treatments", which presents the spread as bracketed rather than resolved.

**Status: the source conflict is narrowed in understanding, while the candidate remains unadmitted and open as a number.**

- The primary prints **no single dry-dislocation V\***. Table 1 defers, and Table 2 offers nine values from -2 to 27.
- Neither 6 nor 13-27 is "the H&K value", because there is no such thing to be. Any consumer that reports one number here has made a selection the primary did not make.
- The narrowing the primary does supply: V\* is a **chord** quantity, defined over the pressure interval it was fitted across, and it decreases with pressure. Selecting a V\* therefore requires naming the pressure range it will be used over.

**A conditioning-variable finding, of the class the arc already knows.** `GEOTHERM_ARC_SCOPE.md` records that a limiting isotherm is not a property of the lithosphere but a property of the lithosphere joined to an age convention, so a single number quoted without its convention is "a statistic with a HIDDEN CONDITIONING VARIABLE". **V\* is the same shape of quantity, and the hidden conditioning variable is the pressure interval.** A V\* quoted bare is a chord whose endpoints have been dropped. The primary names this outright by calling it a chord V\*. The library's 6 is a chord over 10^-4 to 10 GPa presented as a constant; Dixon's 13-27 is the set of chords over intervals at or below 2 GPa. The DEFAULTS-TAKEN discipline the scope extended to fetched rows' CONVENTIONS covers this case: a V\* row must carry its fitted pressure interval, or it is not a row.

This also sharpens the prior fetch's validity-domain note. It reported "The activation volumes are constrained only over P <= 2 GPa (Dixon and Durham, Table 1)." That is Dixon's filter on the primary, not a limit of the primary: the primary's Table 2 carries determinations to 15 GPa. What is true is that the P ≤ 2 GPa subset is the one Dixon selected, and that the determinations at higher pressure return systematically LOWER V\* for the reason quoted above.

## 5. The Dixon contradiction, settled against the primary

Both halves of the contradiction were verified verbatim in Dixon's own PDF, so the prior fetch's report of it is confirmed with one correction.

**The caption.** It is the **Figure 4** caption, not Figure 6. `GEOTHERM_FETCHES.md` sections 1.3 and 1.5 attribute it to Figure 6 twice. Figure 6's caption is about "Results of all 22 run steps plotted as log stress versus P" and carries none of these constants. The Figure 4 caption reads:

> "All points have been adjusted to a common pressure (5 GPa), temperature (nearest 73 K), and water content (500 H/10^6 Si) using equations (1) and (1a) and constants guided by Table 1 in this paper and **Table 1 (wet dislocation creep) in Hirth & Kohlstedt (2003): V\* = 14 cm^3/mol, E\* = 400 kJ/mol, and r = 1.2.**"

**The body.** Section 5.1 reads: "the expected mechanism from Hirth and Kohlstedt (2003) is **dislocation creep accommodated by GBS**, whose flow constants we take from their Table 1 for both this conversion and the strain rate adjustment associated with the V\* calculation below. For the ±T to ±σ_ss conversion, the relevant parameter in equation (1) is E\* = 400 kJ/mol." Section 5.2 adds "Taking n = 3.5 for GBS in olivine (Hirth & Kohlstedt, 2003)".

**What the primary's Table 1 says that row is.** `E* = 400 kJ/mol` appears **exactly once in Table 1**, in the row labelled **`dry GBS, T<1250°C`**, whose full entry is: A = 6500, n = 3.5, p = 2, r = **-**, α = 30-45, E\* = 400^f, V\* = (see Table 2)^g.

**Evidence adjudication:**

1. **The mechanism is GBS. Dixon's BODY TEXT is correct against the primary; his FIGURE 4 CAPTION is wrong.** There is no wet dislocation creep row in H&K Table 1 carrying E\* = 400. The wet dislocation rows carry 520 ± 40 and 480 ± 40. The prior fetch's instruction to "Treat the 400/14 pair as GBS until the primary's Table 1 says otherwise" was the right call, and the primary now says so.

2. **The row is DRY, and the caption's r = 1.2 cannot come from it.** This is the part neither reading anticipated. The `dry GBS, T<1250°C` row's water exponent is printed as `-`: it has **no water-fugacity term at all**. `r = 1.2` appears only in the two wet dislocation rows (1.2 ± 0.4, and 1.2 at constant C_OH). So **Dixon's triple (E\* = 400, V\* = 14, r = 1.2) corresponds to no single row of H&K Table 1.** It is a COMPOSITE: E\* = 400 from the dry GBS row, r = 1.2 from a wet dislocation row.

3. **This explains the caption rather than excusing it.** Dixon's Figure 4 adjusts data to a common water content of 500 H/10^6 Si, so he needs a water exponent. H&K parameterize GBS only under dry conditions and supply none. Dixon borrowed r = 1.2 from the wet dislocation rows and named that row in his caption, while E\* = 400 came from the GBS row. The caption's label is right about where r = 1.2 lives and wrong about where E\* = 400 lives. His word "guided" concedes the set is assembled rather than transcribed.

4. **H&K parameterize GBS as dry by construction, so the gap Dixon bridged is real.** Table 1 has two GBS rows, both dry. The primary's text (page 95) explains why: "The lack of grain size dependence for dislocation creep processes under wet conditions and the observation that creep rates even in the coarse-grained natural samples are similar to the easiest slip systems suggests that von Mises criterion is satisfied without slip on the hardest slip system... Hence, under hydrous conditions, dislocation climb apparently accommodates the strain that is accommodated by either GBS or slip on the hardest slip system." There is no wet GBS row in the primary because H&K hold that GBS is not the wet accommodation mechanism.

5. **V\* = 14 is not uniquely attributable.** Table 1's GBS rows print "(see Table 2)^g", and footnote g says "The value for V\* is assumed to be the same as that for dislocation creep." Table 2 carries 14 in three separate rows (Karato and Jung [2002] at 0.3-2 GPa, Karato and Rubie [1997] at 0.3-15 GPa, Karato and Ogawa [1982] at 10^-4-2.0 GPa). A consumer cannot recover which one Dixon used from the primary alone.

**Consequence for the evidence record.** The prior fetch's section 1.3 records "So Hirth and Kohlstedt's Table 1 carries a dislocation-accommodated grain-boundary-sliding row distinct from the four rows above." That is confirmed, and it is **two** rows, not one, split at 1250°C, and both dry. Any future GBS evidence adapter must preserve these two candidate rows:

| | A | n | p | r | α | E* (kJ/mol) | V* |
| --- | --- | --- | --- | --- | --- | --- | --- |
| dry GBS, T>1250°C | 4.7×10^10 | 3.5 | 2 | - | 30-45 | 600 | same as dislocation creep (Table 2) |
| dry GBS, T<1250°C | 6500 | 3.5 | 2 | - | 30-45 | 400 | same as dislocation creep (Table 2) |

Note what footnote f says about those two E\* values, since it bears on whether they are measurements: "The activation energy for GBS is assumed to be that for slip on (010)[100], which changes with increasing temperature [Bai et al., 1991]. The values given here include the effect of temperature on oxygen fugacity." The primary's text agrees (page 95): "we assume that E\*_gbs is the same as that for dislocation creep of olivine on its easiest slip system." **The GBS activation energies are ASSUMED, transferred from single-crystal easy-slip data, not fitted to GBS experiments.** The 600/400 split at 1250°C is inherited from the temperature dependence of that slip system. A consumer that treats 400 kJ/mol as a measured GBS activation energy has promoted an assumption to a measurement. Neither 600 nor 400 carries a band in Table 1, and that absence is consistent: an assumed value has no fitted uncertainty.

## 6. Validity domain, as the primary states it

The domain is part of the row, so what the primary says about its own reach:

- **The extrapolation, in the authors' words** (page 84): "Experimental studies constrain the magnitude of mantle viscosity. However, due to the required extrapolation from laboratory to geologic conditions, the accuracy of these constraints is not as high as their precision. Thus, the laboratory data provide stronger constraints on the change in viscosity as a function of pressure, temperature, grain size, water content and melt fraction." The prior fetch quotes the middle sentence correctly.
- **Where the accuracy limit comes from** (page 84): "Laboratory experiments are usually conducted near upper mantle temperatures. Therefore, the primary limitation on accuracy comes from the relatively large extrapolation in stress." The dominant extrapolation the authors name is in **stress**, not temperature.
- **Pressure.** The V\* determinations behind Table 2 span 10^-4 to 15 GPa by technique (Table 2's own P range column). The flow-law experiments themselves are gas-medium work at confining pressures of order 300 MPa; the figures state their conditions directly (Figure 5: σ = 150 MPa, T = 1250°C, P = 300 MPa; Figure 6b: T = 1300°C, P = 300 MPa).
- **Grain size.** Figure 6's caption and legends span roughly 15 to >100 μm, with diffusion-creep work on fine-grained synthetic aggregates. Table 1's own worked example (footnote c) evaluates at d = 10 mm, that is the authors extrapolate their own p = 3 law three orders of magnitude in grain size beyond the samples.
- **Melt.** α applies at φ < 0.12 (page 94). Outside that, the exponential form is not the fitted description.
- **Water.** The wet rows are referenced through f_H2O in MPa or C_OH in H/10^6 Si (footnote a). The primary's anchor: "Olivine in the oceanic asthenosphere contains ~1000 H/10^6Si, which is approximately 20% of the solubility at a depth of 120 km."
- **The GBS split** is at T = 1250°C, and both GBS rows are dry.

The prior fetch's statement that the underlying experiments were "conducted under gas confinement at pressures rarely exceeding 300 MPa" is attributed to Dixon section 2 and is consistent with the primary's figure conditions. The prior fetch's claim of a wet/dry boundary at ~70 ± 30 H/10^6 Si is Dixon's table footnote; **the primary does not state that boundary**, and it was not verified here.

## 7. What could not be read, and what remains open

- **Nothing in Tables 1 or 2 was unreadable.** Every cell above was read twice, by text extraction and by a visual read of the rendered page, and the two agree.
- **A source-internal difference, minor and not load-bearing.** The primary's page 92 prose prints the Bejina Si self-diffusion activation volume with a `10^6` exponent, while Table 2 prints **-2** for the corresponding row. The prose also describes that value as lower than the recovery value. These parallel source records are preserved without selecting or correcting either one. The candidate Si-diffusion row is excluded because the repository cannot derive one unique value from the source as printed.
- **The chord-V\* prose names "Table 1"** (section 4), while the described chord-V\* column occurs in Table 2. Treating the prose as a reference to Table 2 would be an inference not stated by the source, so the repository preserves both records and does not silently substitute one table identifier for the other.
- **Still open, and the primary is the reason it is open**: the dry dislocation V\*. The primary declines to print one. See section 4.
- **Not verified here**: Ohuchi et al. (2015), Bürgmann and Dresen (2008), and the Kohlstedt Treatise chapter were not re-fetched; the prior fetch's values from them agree with the primary where they overlap (530 ± 4, 375 ± 50), which is corroboration of those readings. Dixon's own measurement (V\* = 15 ± 5 for dry olivine over 2-9 GPa) was not re-verified against his text in this pass.

## 8. The bottom line for BLOCKER 1

Table 1 is read. Relative to it, the four-row parameter record the arc was going to use agrees on the A, n, p, and E\* central values but does not carry the printed uncertainty bands or melt term. Its n band is 0.5 rather than Table 1's 0.3, two V\* entries select undeclared midpoints of printed ranges, one V\* has no Table 1 counterpart, and the wet rows use a different water parameterization. The dry V\* material remains an unresolved source spread conditioned on pressure interval, which a bare scalar would omit.

Nothing here is admitted. Every value above is candidate evidence read from the primary. A canonical use must derive from the sealed floor, complete the full admission receipt for one unique residual, or refuse.

**Primary citation.** Hirth, G. and Kohlstedt, D., 2003, "Rheology of the Upper Mantle and the Mantle Wedge: A View from the Experimentalists", in Inside the Subduction Factory, ed. J. Eiler, Geophysical Monograph 138, American Geophysical Union, Washington DC, pp. 83-105, DOI 10.1029/138GM06. Table 1 on p. 86; Table 2 on p. 92; equation (1) on p. 86; the chord V\* discussion on p. 93; the GBS discussion on p. 95; the melt-fraction discussion on p. 94; the extrapolation caveats on p. 84. Read from the copy at `https://www.soest.hawaii.edu/earthsciences_archive/FACULTY/smithkonter/GG631/other/HirthKohlstedt_2000.pdf` (SHA256 `d76c5905bb1cdfb8d2d62a566daa85636e5ef7120ed07c6f174a6c2a150fcba7`), whose embedded metadata carries DOI 10.1029/138GM06 and whose page-1 copyright line reads "Copyright 2003 by the American Geophysical Union".

**Second primary read.** Dixon, N. A. and Durham, W. B., 2018, "Measurement of Activation Volume for Creep of Dry Olivine at Upper-Mantle Conditions", Journal of Geophysical Research: Solid Earth 123, DOI 10.1029/2018JB015853 (Table 1; Figure 4 caption; sections 5.1 and 5.2), read from `https://www.osti.gov/servlets/purl/1609770` (SHA256 `2a41294f4e7ed253950fe589b28eb654d2f4ab517821370bb75923d434079428`).

**Reproduction checked.** UWGeodynamics `ViscousRheologies.json`, `github.com/underworldcode/UWGeodynamics`, `UWGeodynamics/ressources/ViscousRheologies.json` (SHA256 `829831e56ad1ae534834a2a1871700186638653abba2490664634e70226b2a1e`), the four entries whose citation string is "Hirth, G., & Kohlstedt, D. (2004)".

**Confidence.** High on every cell of Tables 1 and 2, read from the primary twice by independent extraction paths. High on the n = 3.5 ± 0.3 adjudication and on the traced origin of the ± 0.5. High on the GBS resolution of the Dixon contradiction, both halves read verbatim from Dixon's own text. High on the V\* reconciliation, since Dixon's 13-27 and the library's 6 both reproduce exactly from the primary's Table 2 under stated filters. Medium on the chord-V\* sentence meaning Table 2, an inference from the column it describes against a misprinted reference.
