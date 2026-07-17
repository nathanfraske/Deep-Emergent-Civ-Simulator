# The metallicity dependence of the photoevaporation wind rate: slope, sign, and the axis it does not share with the Owen-versus-Sellek model choice

This document is the primary-source fetch that settles one number and its sign for the disk-evolution arc's photoevaporation wind rate. The wind rate is a contested family of coefficient rows on one shape, `Mdot_w ~ coeff * (M_star)^a * (L_X)^b`, with Owen, Clarke and Ercolano 2012 the high-rate row and Sellek et al. 2024 (PLUTO+PRIZMO radiation-hydro) roughly an order of magnitude lower. The coordinator has already ruled the physics direction, two ways: more metallicity means more molecular cooling means a lower wind rate (toward Sellek's low edge), and less metallicity means a higher rate (toward Owen's high edge). This fetch does not re-derive that sign. It fetches the MAGNITUDE of the metallicity slope and confirms the sign from the sources, and it keeps two axes distinct that the coordinator's ruling needs kept distinct: the model-structure axis (Owen versus Sellek as rival thermochemistry models at fixed metallicity) and the metallicity axis (one model evaluated across metallicity).

This is a verification record only. No code is changed and nothing under `crates/` is touched. Every value below is a cited literature value read from a source through a stated channel, and the reserved-versus-set question is untouched.

A note on the code reference this fetch was pointed at. The task named `crates/sim/src/astro.rs` lines 1590 to 1600 as the site of the three wind-rate rows (Owen 2012 Appendix B, Owen 2012 eq. 9, Sellek 2024). On the current branch (`claude/seam4-deeptime`) those lines hold viscous-disk-temperature test code, and a grep across `crates/` for `Sellek`, `photoevap`, `6.25e-9`, and `molecular cooling` returns nothing. The photoevaporation wind-rate rows are not on this branch. The code reference is a forward pointer to work not yet landed here. This does not change the fetch: the row identities and the code's Sellek characterization were supplied verbatim in the task, and this document verifies them against the sources rather than against the code.

---

## 1. What was read, through which channel, and what was not reached

**READ, PRIMARY, dual-channel: Ercolano and Clarke (2010).** "Metallicity, planet formation, and disc lifetimes", Monthly Notices of the Royal Astronomical Society 402(4), 2735-2743, DOI 10.1111/j.1365-2966.2009.16094.x, arXiv:0910.5110. Fetched as PDF (193,946 bytes) and read by local text extraction (`pdftotext -layout`) after the WebFetch reader failed on the flate-encoded stream. Cross-channel confirmation of every load-bearing quote comes from the WebSearch result summaries, which agree with the extracted text on the sign and on the lifetime scaling. This is the classic result the task named for the metallicity scaling of X-ray photoevaporation, and it is the second-channel lifetime source as well. The version read is arXiv:0910.5110v1 (submitted 27 Oct 2009), which carries the same equations and figures as the published MNRAS 402, 2735.

**READ, PRIMARY, dual-channel: Nakatani, Hosokawa, Yoshida, Nomura and Kuiper (2018).** "Radiation Hydrodynamics Simulations of Photoevaporation of Protoplanetary Disks II: Metallicity Dependence of UV and X-ray Photoevaporation", Astrophysical Journal 865(1), 75, DOI 10.3847/1538-4357/aad9fd, arXiv:1805.07992v2. Fetched as PDF (667,174 bytes) and read by `pdftotext -layout` after the WebFetch reader failed on the binary; the WebSearch summary corroborates the turnover metallicity and the sign. This is the source that supplies an explicit continuous power-law fit of the wind rate against metallicity over a wide range and that maps the non-monotonic turnover, so it is the strongest continuous-scaling source. It is the second paper of a series; Paper I is Nakatani et al. 2018a (ApJ 857, 57, UV-only), not fetched.

**READ, PRIMARY, single-channel through the full-text reader: Sellek, Grassi, Picogna, Rab, Clarke and Ercolano (2024).** "Photoevaporation of protoplanetary discs with PLUTO+PRIZMO I. Lower X-ray-driven mass-loss rates due to enhanced cooling", Astronomy and Astrophysics 690, A296, DOI 10.1051/0004-6361/202450171, arXiv:2408.00848, ADS bibcode 2024A&A...690A.296S. Read through the WebFetch reader against the arXiv HTML full text (`arxiv.org/html/2408.00848`) and against the arXiv abstract page. This is the low-rate row and the model-structure revision the arc contests against Owen. Its role in this fetch is to fix WHERE it sits on the two axes: it is the load-bearing subtlety.

**NOT READ as a PDF, taken from the task and confirmed by WebSearch: Owen, Clarke and Ercolano (2012).** "On the theory of disc photoevaporation", Monthly Notices of the Royal Astronomical Society 422(3), 1880-1901, DOI 10.1111/j.1365-2966.2011.20337.x. The high-rate row `Mdot_w = 6.25e-9 (M_star/M_sun)^-0.068 (L_X/1e30 erg s^-1)^1.14 M_sun/yr` was supplied verbatim by the task and confirmed by a WebSearch result quoting the same coefficients. This fetch does not re-verify Owen from the PDF, because the row identity and its sign role are already settled; what matters here is that Owen is the solar-metallicity anchor, established in section 3.

**NOT REACHED, and named because they are where the remaining detail closes:** Nakatani et al. 2018a (Paper I, ApJ 857, 57, the UV-only precursor whose chemistry Paper II extends); Ercolano, Clarke and Drake 2009 (ECD09, ApJ 699, 1639, the MOCASSIN solar-metallicity X-ray photoevaporation model that Ercolano and Clarke 2010 rerun across metallicity); Picogna et al. 2019 and 2021 (MNRAS 487, 691 and 508, 3611, the newer X-ray photoevaporation profiles that Sellek benchmarks against); Yasui et al. 2009 and later (the extreme-outer-Galaxy disc-fraction observations both lifetime channels calibrate against).

---

## 2. The metallicity slope of the wind rate, verbatim, and its sign

Two independent radiation-hydro or photoionisation calculations give an explicit metallicity slope of the photoevaporation rate. Both are negative: the rate DECREASES as metallicity increases. This confirms the coordinator's sign from the sources, with no disagreement in the range relevant to solar-type stars.

### 2.1 Ercolano and Clarke 2010: `Mdot_w ~ Z^-0.77` over 0.01 to 2 solar

Printed in the results, read from the extracted text:

> "The left panel of Figure 1 shows the dependance of the total photoevaporation rates on metallicity, `Mdot_W(Z)`, which can be approximated by a power-law of index -0.77."

The range is stated in the methods:

> "we ... obtain the temperature structure and photoevaporation rates for gaseous discs with a range of metallicities, spanning from 0.01 solar to twice solar."

The model behind the slope is X-ray plus EUV photoevaporation, the MOCASSIN photoionisation code coupled to a hydrostatic disc, rerunning ECD09's solar model FS0H2Lx1 at each metallicity with the dust-to-gas ratio and metal abundances multiplied by `Z/Z_sun`. The paper is explicit that the slope is channel-specific:

> "Therefore the predicted metallicity dependance shown here pertains only to X-ray photoevaporated discs."

The physical cause is opacity, with cooling secondary and in the same direction:

> "The increasing X-ray photoevaporation rates at lower metallicity can be readily understood ... The reduced extinction in the low metallicity cases allows high density gas at larger columns to be ionised and heated to temperatures sufficiently high for the gas to be entrained into a photoevaporative flow."

and, listed as a secondary effect working the same way, "a low metallicity implies a reduced cooling by fine structure lines of ions and neutrals (such as [O i] and [C ii]) ... which leads to higher temperatures", plus a reduced dust-gas collisional cooling term at lower metallicity.

**Sign, verbatim and unambiguous: the rate rises as metallicity falls, over the whole 0.01 to 2 solar range, monotonically, with a power-law index of `-0.77`.** So `Mdot_w ~ Z^-0.77`: about 0.77 dex of wind rate per dex of metallicity, with the rate DECREASING as metallicity increases. **CONFIDENCE: HIGH.** The number and the range are read directly, and the reading is validated by the internal consistency check in section 4 (the `-0.77` rate slope and the paper's own `-2/3` lifetime-to-rate exponent reproduce the paper's own quoted lifetime slope of `0.52`).

### 2.2 Nakatani et al. 2018: `Mdot ~ Z^-0.6` to `Z^-0.4` over 0.1 to 3.16 solar, with a turnover far below

Nakatani et al. vary metallicity over `10^-3 Z_sun <= Z <= 10^0.5 Z_sun` and fit the rate in the upper, solar-adjacent part of that range. From section 3.4, read from the extracted text:

> "the resulting photoevaporation rate of Run FEX has metallicity dependences of `Mdot_ph ~ Z^-0.6` for `rS = 200 au`, in `0.1 Z_sun <= Z <= 10^0.5 Z_sun`, while in Run FE `Mdot_ph ~ Z^-0.4`."

Run FEX is FUV plus EUV plus X-ray; Run FE is FUV plus EUV without X-ray. So over 0.1 to about 3.16 solar the rate slope is `-0.6` (X-ray included) or `-0.4` (no X-ray), both negative, both DECREASING with metallicity. This agrees with Ercolano and Clarke's `-0.77` in sign and is close in magnitude, the residual difference being the heating channel and the fitted range.

The abstract states the sign directly, and it names the non-monotonic turnover that becomes the boundary of validity:

> "At sub-solar metallicities in the range of `Z >~ 10^-1.5 Z_sun`, the photoevaporation rate increases as metallicity decreases owing to the reduced opacity of the disk medium. The result is consistent with the observational trend that disk lifetimes are shorter in low metallicity environments. Contrastingly, the photoevaporation rate decreases at even lower metallicities of `Z <~ 10^-1.5 Z_sun`, because dust-gas collisional cooling remains efficient compared to far UV photoelectric heating whose efficiency depends on metallicity."

and the X-ray recovery below the turnover:

> "However, adding X-ray radiation significantly increases the photoevaporation rate, especially at `Z ~ 10^-2 Z_sun`."

**Sign, verbatim: over 0.1 to 3.16 solar the rate decreases as metallicity increases, slope `-0.4` to `-0.6`; the sign reverses only below `Z ~ 10^-1.5 Z_sun` (about 0.03 solar) in the FUV-driven case, and the X-ray channel partly recovers the rate even there.** **CONFIDENCE: HIGH** on the slope, the range, and the turnover metallicity, each read directly and corroborated by the WebSearch summary.

### 2.3 The two X-ray-inclusive results are consistent, and the turnover is not a disagreement

Ercolano and Clarke fit a single `-0.77` down to 0.01 solar with no turnover, while Nakatani find a turnover at about 0.03 solar. These do not conflict, and the reason is instructive. Ercolano and Clarke's model is X-ray plus EUV, and in it the dust-gas collisional cooling term REDUCES at lower metallicity, working in the same direction as the opacity effect, so the rate keeps rising to 0.01 solar. Nakatani's turnover is a property of the FUV-driven case (Run FE), in which at very low metallicity the dust-gas cooling REMAINS efficient relative to the falling FUV photoelectric heating, so cooling wins and the rate turns over. Nakatani then show that adding X-rays (Run FEX) significantly raises the rate again at `Z ~ 10^-2 Z_sun`, that is, the X-ray channel erases most of the FUV-only turnover, which is exactly why Ercolano and Clarke's X-ray-inclusive model has none. The two are consistent once the heating channel is matched. For any world in the solar-type range 0.1 to 2 solar, both give a clean, negative, monotonic slope near `-0.6` to `-0.8`.

---

## 3. The load-bearing subtlety: the model-structure axis and the metallicity axis are separate

This is the distinction the coordinator's ruling needs kept distinct, and the sources keep it distinct. There are two different axes, and conflating them double-counts a factor of about ten.

**Axis A, model structure at fixed metallicity: Owen versus Sellek.** Owen, Clarke and Ercolano 2012 and Sellek et al. 2024 are BOTH solar-metallicity models. Sellek states its abundances explicitly, read through the full-text reader:

> "ISM gas-phase abundances consistent with previous photoevaporation models (e.g. Ercolano et al. 2009; Wang and Goodman 2017): He/H = 0.1, C/H = 1.4e-4 and O/H = 3.2e-4"

with "a uniform ISM dust-to-gas mass ratio of `10^-2` everywhere". These are solar/ISM values, the same abundance set Owen's lineage (ECD09) uses. Sellek's roughly order-of-magnitude reduction relative to Owen is a THERMOCHEMISTRY revision at fixed solar metallicity, attributed to a specific cooling channel:

> "We find that additional cooling results from the excitation of O by neutral H, which leads to dramatically reduced mass-loss across the disc compared to previous X-ray photoevaporation models"

with an integrated rate "`~10^-9 M_sun yr^-1`". **Sellek does not vary metallicity and gives no `Mdot`-versus-`Z` scaling.** The Owen-to-Sellek difference is a difference of cooling model at one metallicity, so the code's framing (Sellek as a model revision at solar, not a low-metallicity instance of Owen) is CORRECT and confirmed from the source. **CONFIDENCE: HIGH** that Sellek is solar-metallicity and that its reduction is a cooling-model effect; **CONFIDENCE: MEDIUM** on the precise reduction factor, because Sellek's abstract says "dramatically reduced" without a printed factor and does not print the fiducial `L_X`, so the "roughly an order of magnitude" comes from comparing Owen's `6.25e-9 M_sun/yr` at `L_X = 1e30` and solar mass against Sellek's `~10^-9 M_sun/yr`, a factor of about six, which is order-of-magnitude but not pinned to a matched `L_X`.

**Axis B, metallicity at fixed model structure.** The slope of section 2 is one model (Ercolano and Clarke's X-ray-plus-EUV, or Nakatani's) evaluated across metallicity. This is a different operation from swapping Owen for Sellek. Ercolano and Clarke did not change the cooling model between their metallicity points; they scaled the abundances and dust-to-gas ratio by `Z/Z_sun` and reran the same code.

**The two axes are separate as measured, and comparable in size.** Over the solar-neighbourhood range 0.1 to 2 solar, the metallicity band spans a factor `(0.1/2)^-0.77 ~ 10` in `Mdot` (Ercolano and Clarke) or `(0.1/3.16)^-0.6 ~ 8` (Nakatani). The model-structure band, Owen to Sellek at fixed solar metallicity, is a factor of about six to ten. So each axis is individually about an order of magnitude, and they are the same size. That is precisely why they must not be conflated: a rate that is low because the world is metal-rich (axis B) and a rate that is low because the thermochemistry carries more cooling (axis A) are different physical statements of comparable magnitude, and attributing one to the other mis-prices the wind by up to a factor of ten. **CONFIDENCE: HIGH** that the axes are conceptually and operationally distinct, read from the sources; **CONFIDENCE: MEDIUM** on the exact factor of the model-structure band per the `L_X`-matching caveat above.

**The one physical coupling to flag, and its honest bound.** The two axes are distinct as measured but not fully orthogonal in the physics, and the sources show why. The metallicity slope is MODEL-DEPENDENT: `-0.77` for Ercolano and Clarke's X-ray-plus-EUV, `-0.6` for Nakatani's Run FEX, `-0.4` for Nakatani's Run FE. The slope depends on which cooling channels dominate, because those channels are what carry the metallicity dependence. Sellek's enhanced cooling is the excitation of O by neutral H, whose strength scales with the O abundance and hence with metallicity, so a Sellek-generation model evaluated across metallicity would plausibly give a STEEPER slope than the older Owen or Ercolano-and-Clarke X-ray models, because more of its cooling budget sits in metal-bearing channels. **The sources do not quantify this: Sellek ran only solar metallicity.** So the honest position is that the slope to use should be matched to the model chosen on axis A, and the Sellek-across-metallicity slope is an OPEN quantity, flagged rather than fabricated. What is settled: the axes are separate, the sign is the same on the metallicity axis for every model here, and the magnitude is `-0.4` to `-0.8` for the models that have been run across `Z`.

---

## 4. The second-channel cross-check: disc lifetime versus metallicity

The task asks that the net disc-lifetime direction be checked against the wind-rate direction; they should agree, low metallicity meaning faster clearing. They agree, and both papers state it.

Ercolano and Clarke 2010 give the lifetime slope directly. From the abstract, "Our models show `t_phot ~ Z^0.52` for a pure photoevaporation model", and in the body, "This Z dependence is rather weak, with a power law exponent of `0.52` (for `p = 1`) and `0.38` (for `p = 1.5`)", where `p` is the surface-density power-law index. In physical terms, "An increase in metallicity from solar to twice solar causes the disc lifetime to increase from `~2` to `~3.1` Myr, and a decrease from solar to `-0.7` dex solar ... produces a decrease in the disc lifetime from `~2` to `~0.7` Myr." The direction is stated in prose too: the disc lifetime is "a mildly increasing function of metallicity, resulting from the rather higher photoevaporation rates in the case of low metallicity gas for which opacities are lower and line cooling less efficient."

The two directions are tied together by the paper's own lifetime-to-rate relation (their equation A10): `t_phot ~ Mdot_W(Z)^((4-2p)/(-5+2p))`, which for `p = 1` is `t_phot ~ Mdot_W^-2/3`. This is the relation Nakatani cite as "disk lifetimes are approximately calculated as `T_life ~ Mdot_ph^-2/3` (Ercolano and Clarke 2010)".

**Internal consistency check, which also validates the extraction.** Ercolano and Clarke's rate slope is `-0.77` and their lifetime-to-rate exponent for `p = 1` is `-2/3`. Compose them: `(-0.77) * (-2/3) = 0.513`, which reproduces their quoted lifetime slope of `0.52` to two figures. The rate slope, the lifetime-to-rate exponent, and the lifetime slope form a closed triangle, so the `-0.77` and the `0.52` are not two independent readings that could each be a transcription slip; each implies the other. **This is a strong check that the `pdftotext` reading is faithful.**

Nakatani give the observed lifetime slope and their own consistency argument. From section 3.4: "typical lifetimes of protoplanetary disks are 3 Myr for solar metallicity disks and 1 Myr for those with `Z = 0.2 Z_sun` (Yasui et al.) ... This metallicity dependence of the lifetimes can be fit as `T_life ~ Z^0.7`", and their model rate slopes of `-0.6` and `-0.4` "are consistent with the observational metallicity dependence of the lifetimes because disk lifetimes are approximately calculated as `T_life ~ Mdot_ph^-2/3`". Composing `-0.6` with `-2/3` gives a model lifetime slope of `0.40`, shallower than the observed `0.7`; the sign agrees and the model is milder than the data, which the paper acknowledges by calling the agreement one of consistency rather than a fit.

**The lifetime channel agrees with the wind-rate channel.** Lower metallicity gives a higher wind rate (section 2) and a shorter disc lifetime (this section), that is, faster clearing at low metallicity. Both channels point the same way, and both agree with the observed trend that metal-poor clusters lose their discs faster (Yasui et al.). **CONFIDENCE: HIGH.**

---

## 5. Where solar sits, and the boundary of the sign

Solar metallicity is the anchor of both the model-structure axis and the metallicity axis, and it is where the arc's high and low rows both live. Owen 2012 and Sellek 2024 are both solar-metallicity models (section 3), so the Owen-high, Sellek-low contrast is a MODEL choice AT solar, not a solar-versus-low-metallicity contrast. On the metallicity axis, solar is one point on the `Z^-0.77` (or `Z^-0.6`) line, with metal-poor worlds sitting at higher rates and metal-rich worlds at lower rates.

The sign is robust in the range that matters for solar-type, planet-hosting stars, roughly 0.1 to 2 solar: every model here gives a negative rate slope there. The sign is NOT universal at extremely low metallicity. Below about `10^-1.5 Z_sun` (0.03 solar) the FUV-driven rate turns over and decreases with decreasing metallicity (Nakatani), because dust-gas collisional cooling wins over the weakening FUV heating; the X-ray channel partly recovers the rate even there, and Ercolano and Clarke's X-ray-inclusive model shows no turnover down to 0.01 solar. For a data-driven engine that admits metal-poor worlds, the operational statement is: the negative slope holds for `Z >~ 0.03 Z_sun`, and below that the sign becomes model-and-channel-dependent and should be treated as a separate regime rather than an extrapolation of the `-0.77` line.

---

## 6. Confidence, defaults taken, and what could not be verified

**CONFIDENCE by item.**
- The metallicity slope of the wind rate is negative (rate decreases as metallicity increases) over 0.1 to 2 solar: HIGH, two independent primaries agree in sign, and the sign matches the coordinator's ruling.
- The slope magnitude is `-0.77` (Ercolano and Clarke, X-ray plus EUV, 0.01 to 2 solar) and `-0.6` to `-0.4` (Nakatani, X-ray-inclusive to FUV-only, 0.1 to 3.16 solar): HIGH as readings, with the residual `-0.4` to `-0.8` spread being a real model-and-channel dependence rather than a measurement error.
- The model-structure axis (Owen versus Sellek) is separate from the metallicity axis, both at solar: HIGH.
- The disc-lifetime direction agrees with the wind-rate direction (low metallicity clears faster): HIGH, with an internal consistency triangle closing to two figures.
- The reduction factor of the model-structure axis (Owen to Sellek) is "roughly an order of magnitude": MEDIUM, because Sellek prints "dramatically reduced" and `~10^-9 M_sun/yr` but not a factor and not a matched `L_X`.
- The Sellek-generation slope across metallicity would be steeper than the older X-ray slope: this is a physically motivated expectation, not a measured value; LOW as a number, and flagged as open rather than fabricated.

**DEFAULTS TAKEN.** Where a single slope is wanted for the solar-type range, the defensible default is Ercolano and Clarke's `-0.77` for an X-ray-plus-EUV model, or Nakatani's `-0.6` for an X-ray-inclusive radiation-hydro model; both are cited, both negative, and the choice between them should be matched to the cooling model the arc adopts on the Owen-versus-Sellek axis. No slope was invented; the range `-0.4` to `-0.8` is the honest spread of what has been run across metallicity. The `-2/3` lifetime-to-rate exponent is the `p = 1` case of Ercolano and Clarke's equation A10; other `p` change it (for example `p = 1.5` gives the `0.38` lifetime slope), so `p` is itself a modelling choice that should be read from the disc's surface-density profile rather than fixed.

**WHAT COULD NOT BE VERIFIED.**
- Sellek's fiducial `L_X` and the exact Owen-to-Sellek reduction factor were not printed in the abstract or HTML full text reached; the "order of magnitude" is a comparison of two separately quoted numbers, not a single printed ratio.
- The Sellek-generation metallicity slope is not in the literature reached: Sellek et al. 2024 ran only solar metallicity and defer nothing explicit on a metallicity grid within the text extracted, so the steeper-slope expectation is unquantified.
- Owen, Clarke and Ercolano 2012 was not read from the PDF; the `6.25e-9` coefficient row is taken from the task and confirmed by one WebSearch summary, not by a dual-channel read of the paper. Its solar-metallicity status is inferred from its shared ECD09 abundance lineage, not from a quoted abundance line in that paper.
- The two low-metallicity primaries (Ercolano and Clarke, Nakatani) were read through a single extraction channel each (`pdftotext`) because the WebFetch reader could not parse the encoded PDF streams; the WebSearch summaries corroborate the sign and the turnover but are not a second verbatim channel for the exact exponents. The internal consistency triangle in section 4 is the strongest independent check on the Ercolano and Clarke numbers.

---

## Sources and channels

Ercolano, B. and Clarke, C. J., 2010, "Metallicity, planet formation, and disc lifetimes", Monthly Notices of the Royal Astronomical Society 402(4), 2735-2743, DOI 10.1111/j.1365-2966.2009.16094.x, arXiv:0910.5110 (the `-0.77` rate slope and Figure 1 in the results; the 0.01-to-2-solar range and the X-ray-plus-EUV MOCASSIN modelling in the methods; the `t_phot ~ Z^0.52` and `Z^0.38` lifetime slopes and the solar-to-twice-solar and solar-to-minus-0.7-dex lifetime numbers; equation A10, `t_phot ~ Mdot_W^-2/3` for `p = 1`). Read by `pdftotext -layout` from the arXiv PDF; corroborated on sign and lifetime by WebSearch.

Nakatani, R., Hosokawa, T., Yoshida, N., Nomura, H. and Kuiper, R., 2018, "Radiation Hydrodynamics Simulations of Photoevaporation of Protoplanetary Disks II: Metallicity Dependence of UV and X-ray Photoevaporation", Astrophysical Journal 865(1), 75, DOI 10.3847/1538-4357/aad9fd, arXiv:1805.07992 (the `10^-3` to `10^0.5` solar range and the sign in the abstract; the turnover at `Z ~ 10^-1.5 Z_sun` and the X-ray recovery at `Z ~ 10^-2 Z_sun`; the `Mdot_ph ~ Z^-0.6` (Run FEX) and `Z^-0.4` (Run FE) fits over 0.1 to `10^0.5` solar and the `T_life ~ Z^0.7` observed slope with the `T_life ~ Mdot_ph^-2/3` relation, section 3.4). Read by `pdftotext -layout` from the arXiv PDF; corroborated on the turnover and sign by WebSearch.

Sellek, A. D., Grassi, T., Picogna, G., Rab, Ch., Clarke, C. J. and Ercolano, B., 2024, "Photoevaporation of protoplanetary discs with PLUTO+PRIZMO I. Lower X-ray-driven mass-loss rates due to enhanced cooling", Astronomy and Astrophysics 690, A296, DOI 10.1051/0004-6361/202450171, arXiv:2408.00848, ADS 2024A&A...690A.296S (the solar/ISM abundances He/H = 0.1, C/H = 1.4e-4, O/H = 3.2e-4 and dust-to-gas `10^-2`; the integrated rate `~10^-9 M_sun/yr`; the attribution to additional cooling from excitation of O by neutral H; the absence of any metallicity variation). Read through the WebFetch full-text reader against the arXiv HTML and abstract.

Owen, J. E., Clarke, C. J. and Ercolano, B., 2012, "On the theory of disc photoevaporation", Monthly Notices of the Royal Astronomical Society 422(3), 1880-1901, DOI 10.1111/j.1365-2966.2011.20337.x (the high-rate row `Mdot_w = 6.25e-9 (M_star/M_sun)^-0.068 (L_X/1e30)^1.14 M_sun/yr`). Taken from the task and confirmed by a WebSearch summary quoting the same coefficients; not read from the PDF, and its solar-metallicity status inferred from its ECD09 abundance lineage.

---

## Plain closing statement

The metallicity slope of the photoevaporation wind rate is `Mdot_w ~ Z^-0.77` (Ercolano and Clarke 2010, X-ray plus EUV, 0.01 to 2 solar) and `Mdot_w ~ Z^-0.6` (Nakatani et al. 2018, X-ray-inclusive radiation-hydro, 0.1 to 3.16 solar), that is, about `-0.4` to `-0.8` dex of wind rate per dex of metallicity depending on the model and heating channel, with the sign being: the rate DECREASES as metallicity increases. Confidence HIGH on the sign, HIGH on the slope as a model-dependent band of `-0.4` to `-0.8`. The sign matches the coordinator's ruling, and no source disagrees with it in the solar-type range 0.1 to 2 solar; the sign reverses only below about 0.03 solar in the FUV-driven case, a separate low-metallicity regime.

The metallicity axis is SEPARATE from the Owen-versus-Sellek model-structure axis: Owen 2012 and Sellek 2024 are both solar-metallicity models, so their roughly-order-of-magnitude difference is a thermochemistry (cooling) revision at fixed metallicity, not a low-metallicity instance, exactly as the code frames it. The two axes are each about an order of magnitude in `Mdot` and must be kept distinct, with the one physical coupling being that the slope itself depends on the cooling model, so the slope used should be matched to the model chosen on the Owen-versus-Sellek axis; a Sellek-generation slope across metallicity is not in the literature reached and is flagged open rather than fabricated. The second-channel lifetime cross-check agrees: disc lifetime is `t_phot ~ Z^0.52` (Ercolano and Clarke) or `T_life ~ Z^0.7` (Nakatani, observed), rising with metallicity, so low metallicity clears the disc faster, the same direction as the wind rate, tied together by `t_phot ~ Mdot_W^-2/3`.
