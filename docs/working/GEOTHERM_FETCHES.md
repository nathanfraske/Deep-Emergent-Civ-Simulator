# Geotherm arc literature-fetch values

This file records the primary-source fetch for the five rows the geotherm arc named in `GEOTHERM_ARC_SCOPE.md`: the olivine creep anchors, the oceanic elastic-thickness hindcast, the Mars and Venus hindcast rows, the ice shell branch, and Hartmann saturation. It follows the format and the discipline of `PIPELINE_FETCHES.md`. For every entry the quantity, the value or functional form with its band, the units, the primary citation, the validity domain, the population that convicted the fit, and the hazards a consumer must know are given. Where a value could not be pinned to a source that was read rather than summarized, it is flagged under "Defaults taken" rather than guessed.

Verify-on-pull discipline: a value in this doc is a target to VERIFY against its citation at load, never a digit to trust from this doc. Nothing here is set. These are cited literature values, the legal cited residents of the value line, and the reserved-versus-set question is untouched by this fetch.

Extraction note: most primaries here are PDFs that the fetch path could not parse. Where that happened the PDF was extracted locally with `pdftotext` and read directly, so the values below marked PRIMARY were read from the paper's own text rather than from a search summary. Two rows lean on a reproduction that cites the primary (the pattern the craterstats coefficient file set in `PIPELINE_FETCHES.md`), and each is labelled as such.

A note on grade. Three sources below are labelled SUMMARY-ONLY: their numbers were reported by a search engine's summary of an abstract rather than read from the paper's text. They are recorded so the trail exists, and they are listed again under "Defaults taken". A SUMMARY-ONLY value is not fit to code against.

---

## 1. Hirth and Kohlstedt olivine flow-law rows (the calibrated creep anchors)

### 1.1 The constitutive form

The flow law is a single Arrhenius power law with grain-size, water-fugacity, and melt terms folded into the pre-exponential:

`strain_rate = A sigma^n d^(-p) f_H2O^r exp(alpha phi) exp( -(E* + P V*) / (R T) )`

with `sigma` differential stress, `d` grain size, `f_H2O` water fugacity, `phi` melt fraction, `E*` activation energy, `V*` activation volume, `P` pressure, `T` temperature, `R` the gas constant. Dixon and Durham write the grain-size and fugacity split explicitly as their equation (1a), `A = A' d^(-p) f_H2O^r`, which is the same statement.

**Unit convention (HAZARD, load-bearing).** In this parameterization stress is in MPa, temperature in K, pressure in Pa, grain size in micrometres, and `COH` in H/10^6 Si. The pre-exponential `A` carries whatever units make the product come out in s^-1, so `A` is NOT portable across a change of stress unit. A consumer that works in Pa must rescale `A` by `10^(6n)` (and by the fugacity exponent as well for the wet rows). This is the single most likely way to get this row wrong.

### 1.2 The four rows

Read from a machine-readable reproduction that carries the primary citation string: the UWGeodynamics `ViscousRheologies.json` rheology library (`github.com/underworldcode/UWGeodynamics`, file `UWGeodynamics/ressources/ViscousRheologies.json`), whose entries cite "Hirth, G., & Kohlstedt, D. (2004). Rheology of the upper mantle and the mantle wedge: A view from the experimentalists. Inside the subduction Factory, 138, 83-105." This is a REPRODUCTION, the same class of source as the craterstats coefficient file. The primary's Table 1 was not read directly (see "Defaults taken").

| Regime | A | n | p (grain size) | r (water) | E* (kJ/mol) | V* (m^3/mol) |
| --- | --- | --- | --- | --- | --- | --- |
| Dry olivine, dislocation creep | 1.1e5 MPa^-3.5 s^-1 | 3.5 | 0 | n/a | 530 | 6e-6 |
| Dry olivine, diffusion creep | 1.5e9 MPa^-1 um^3 s^-1 | 1.0 | 3.0 | 0 | 375 | 6e-6 |
| Wet olivine, dislocation creep | 1600 MPa^-4.7 s^-1 | 3.5 | 0 | 1.2 | 520 | 23e-6 |
| Wet olivine, diffusion creep | 2.5e7 MPa^-1 um^3 s^-1 | 1.0 | 3.0 | 1.0 | 375 | 10e-6 |

The wet rows in that library are referenced to `COH = 1000` H/10^6 Si.

### 1.3 The bands, from sources that were read

The reproduction above carries no bands. The bands below were read from two open peer-reviewed papers that quote Hirth and Kohlstedt (2003) directly.

**Activation energies (PRIMARY-quoting, read).** Ohuchi et al. (2015), citing Hirth and Kohlstedt (2003) as their reference 10: dry olivine dislocation creep `E* = 530 +/- 4` kJ/mol; dry olivine diffusion creep `E* = 375 +/- 50` kJ/mol. The same sentence gives dry-olivine DisGBS `E* = 445 +/- 20` kJ/mol attributed to a different reference (their ref 7), so the DisGBS band is NOT Hirth and Kohlstedt's.

**Activation volumes (PRIMARY-quoting, read).** Dixon and Durham (2018), Table 1, first row, lists Hirth and Kohlstedt (2003) as: wet olivine `V* = 22 +/- 11` cm^3/mol; dry olivine `V* = 13` to `27` cm^3/mol; over a pressure range `P <= 2` GPa; sample "Various, polycrystalline"; apparatus "Various". Their table footnote sets the wet/dry boundary at `~70 +/- 30` H/10^6 Si, which is the definition of "wet" this whole row keys on.

**Stress exponent.** `n = 3.5`. The band is reported inconsistently across sources: `3.5 +/- 0.3` and `3.5 +/- 0.5` both appear attributed to Hirth and Kohlstedt (2003). Both are SUMMARY-ONLY; see "Defaults taken".

**The DisGBS row.** Dixon and Durham section 5.1 states that for their conditions "the expected mechanism from Hirth and Kohlstedt (2003) is dislocation creep accommodated by GBS, whose flow constants we take from their Table 1", and that "the relevant parameter in equation (1) is `E* = 400` kJ/mol", with `n = 3.5` "for GBS in olivine". Their Figure 6 caption gives the matching set as `V* = 14` cm^3/mol, `E* = 400` kJ/mol, `r = 1.2`. So Hirth and Kohlstedt's Table 1 carries a dislocation-accommodated grain-boundary-sliding row distinct from the four rows above.

### 1.4 Validity domain (part of the row, per the ruling)

The experiments behind the compilation were "conducted under gas confinement at pressures rarely exceeding 300 MPa" (Dixon and Durham, section 2, citing Hansen et al. 2011 and Hirth and Kohlstedt 1995a, 1995b). The activation volumes are constrained only over `P <= 2` GPa (Dixon and Durham, Table 1). Grain sizes in the underlying dislocation-creep experiments are of order micrometres to tens of micrometres; Dixon and Durham note "grain size was approximately 5 um for most of the points shown here", with one study at 0.5 um. Laboratory strain rates are of order `1e-5` s^-1 (Dixon and Durham adjust to a common `1e-5` s^-1); geological strain rates of interest are `1e-15` to `1e-10` s^-1 (Kohlstedt, Treatise chapter, section on deformation mechanism maps). The extrapolation therefore spans roughly ten orders of magnitude in strain rate. Hirth and Kohlstedt's own caveat, read from the chapter text: "Due to the required extrapolation from laboratory to geologic conditions, the accuracy of these constraints is not as high as their precision."

### 1.5 Hazards and a disagreement to report

**DISAGREEMENT (dry activation volume).** The reproduction gives dry dislocation creep `V* = 6e-6` m^3/mol, that is 6 cm^3/mol. Dixon and Durham's Table 1 reports Hirth and Kohlstedt's dry olivine `V*` as 13 to 27 cm^3/mol. These do not overlap. The reproduction's 6 cm^3/mol sits below the bottom of the range the peer-reviewed source attributes to the same primary. This conflict is not resolved here and MUST be settled against the primary's Table 1 before either number is coded. The wet rows agree within rounding (reproduction 23 cm^3/mol against Dixon and Durham's 22 +/- 11 cm^3/mol).

**HAZARD (which wet row).** Dixon and Durham's Figure 6 caption labels the `E* = 400` kJ/mol, `V* = 14` cm^3/mol, `r = 1.2` set as "Table 1 (wet dislocation creep) in Hirth & Kohlstedt (2003)", while their body text at section 5.1 identifies the same constants as the dislocation-accommodated GBS row. The caption and the body disagree. A consumer that takes the caption at face value will wire the GBS constants into the dislocation-creep path. Treat the 400/14 pair as GBS until the primary's Table 1 says otherwise.

**HAZARD (V* does not extrapolate).** Dixon and Durham's entire point is that `V*` is the weakest-constrained parameter: measured values across apparatus "vary from 0 to 23 cm^3/mol, which corresponds to an uncertainty in viscosity" of orders of magnitude at depth. Their own measurement is `V* = 15 +/- 5` cm^3/mol for dry olivine over 2 to 9 GPa. Because `V*` sits in an exponential multiplied by `P`, its band dominates the deep extrapolation. Any lid deeper than the ~2 GPa calibration ceiling is outside the fitted domain.

**HAZARD (the wet parameterization is not unique).** Hirth and Kohlstedt express the water dependence through `COH` in these rows. The literature also carries a constant-water-fugacity form with different `E*` and `V*`. Mixing a `COH`-referenced pre-exponential with a fugacity-referenced exponent is a silent error. The reproduction's rows are `COH`-referenced at 1000 H/10^6 Si.

**Population and selection.** Hirth and Kohlstedt (2003) is a compilation and reanalysis rather than a single experiment: it re-fits data from the Kohlstedt group's own gas-medium apparatus experiments (Hirth and Kohlstedt 1995a, 1995b; Mei and Kohlstedt 2000a, 2000b) plus the earlier literature. Bürgmann and Dresen (2008, read) record that the `n = 3.5` value came from "reanalyzing the data from Mei & Kohlstedt (2000b) and earlier studies", against a prior consensus of `n ~ 3`. So the stress exponent is a re-fit of a specific laboratory's dataset, not an independent global determination.

**Corroboration.** Bürgmann and Dresen (2008) note that the dislocation-creep activation energies "are in close agreement with `Q = 529` kJ mol^-1 estimated from silicon-diffusion experiments in olivine (Dohmen et al. 2002)", which is independent support for the ~530 kJ/mol dry dislocation figure from a different measurement class.

**Primary citation.** Hirth, G. and Kohlstedt, D. L., 2003, "Rheology of the upper mantle and the mantle wedge: A view from the experimentalists", in Inside the Subduction Factory, ed. J. Eiler, Geophysical Monograph 138, American Geophysical Union, Washington DC, pp. 83-105, DOI 10.1029/138GM06. (Some sources date it 2004; the DOI is the same object.)
**Reproduction used (open, machine-readable, primary-citing).** UWGeodynamics `ViscousRheologies.json`, `github.com/underworldcode/UWGeodynamics`.
**Sources read for the bands.** Dixon, N. A. and Durham, W. B., 2018, "Measurement of Activation Volume for Creep of Dry Olivine at Upper-Mantle Conditions", Journal of Geophysical Research: Solid Earth 123, DOI 10.1029/2018JB015853 (Table 1; sections 2, 5.1, 5.2; Figure 6 caption). Ohuchi, T. et al., 2015, "Dislocation-accommodated grain boundary sliding as the major deformation mechanism of olivine in the Earth's upper mantle", Science Advances 1(9), e1500360, DOI 10.1126/sciadv.1500360. Bürgmann, R. and Dresen, G., 2008, "Rheology of the Lower Crust and Upper Mantle: Evidence from Rock Mechanics, Geodesy, and Field Observations", Annual Review of Earth and Planetary Sciences 36, 531-567, DOI 10.1146/annurev.earth.36.031207.124326. Kohlstedt, D. L., "Equations, Rheological Behavior, and Viscosity of Rocks", Treatise on Geophysics chapter 2.14.
**Confidence.** Medium-high on the four-row parameter set (read from a primary-citing machine-readable reproduction, and the `E*` values independently confirmed at 530 and 375 by a read source). High on the activation-energy bands and the activation-volume bands and the `P <= 2` GPa ceiling. LOW on the dry `V*`, which carries an unresolved conflict between sources. Medium on the `n` band.

---

## 2. Oceanic elastic thickness versus plate age (the primary hindcast row)

### 2.1 The relation (PRIMARY, read)

**Te versus square root of age (VALUE plus FORM).**

`Te (km) = 2.70 sqrt(dt)`, with `dt` the age of the lithosphere at the time of loading in Ma.

This is equation (6) of Calmant, Francheteau and Cazenave (1990), read from the paper's own text. The paper's summary states the same relation with its band: "the values are very consistent for the three main oceans and follow the empirical relationship: `Te (km) = (2.7 +/- 0.15) sqrt(dt) (Ma)`". The paper states directly: "The standard deviation of the rate, sigma, is 0.15, indicating good coherency within the data set." So the `+/- 0.15` is the standard deviation of the FITTED RATE, not the scatter of the data about the line.

### 2.2 Population and selection (load-bearing, per the ruling)

The fit is over `Te` estimates at oceanic intraplate VOLCANIC loads (seamounts and oceanic islands), compiled from this paper and previous studies across the Pacific, Atlantic and Indian oceans (their Table 2 lists the individual volcanoes with their plate ages and `Te` values with error bars, for example Tahiti 20 +/- 2 km at 71 Ma load age, Mangaia 7 +/- 0.5 km, Cape Verde 30 +/- 5 km).

Two selection facts govern this number:

1. **An excluded population.** The summary states the relation holds "Excluding the anomalously low estimates from the south-central Pacific". The paper explains that region separately: "This scheme does not hold in the south-central Pacific. The very low estimates found in this region argue for a major change in the thermal structure of the lithosphere." The 2.70 coefficient is therefore conditioned on dropping a real, physically-explained subpopulation. A hindcast that reproduces the global ocean including the south-central Pacific is being checked against a curve that excluded it.
2. **A weighted regression.** "Te values have been weighted according to their error bars."

**The raw scatter.** Before the exclusion and weighting, the paper is blunt: "Although an increasing trend can be observed, a very large scatter is present." The `+/- 0.15` on the rate must not be read as the predictive band for a single locality.

### 2.3 The isotherm (fetched as a HINDCAST TARGET, never as an input)

The scope demotes the limiting isotherm to a cross-check. The literature does not supply one number; it supplies a band whose value depends on which age you compare against.

- **The rheological expectation.** Calmant et al.: "Models and experiments based on mantle rock rheology (mainly olivine) agree in identification of the apparently elastic core of the oceanic lithosphere with the upper part of the lithosphere, where the temperature is less than 600 °C".
- **What the seamount data give.** Calmant et al.: "the thickening rate deduced from seamounts loading is closer to 2.5-3 km Ma^-1/2, which correspond to an isotherm between 350 and 450 °C". They add: "It must be noted that no estimate is close to the 600 °C isotherm".
- **The age-definition dependence (HAZARD).** Calmant et al., reporting McNutt (1984): "elastic thickness estimates fit well for 550-600 °C isotherms, whereas they only fit for lower isotherms (350-450 °C) when compared to the age from isochrons." So the SAME `Te` data imply ~550-600 °C against thermal (bathymetric) age and ~350-450 °C against isochron age. The isotherm is not a property of the lithosphere alone; it is a property of the lithosphere plus the age convention. A hindcast must state which age it used.
- **The classical single-number statement.** "the effective elastic thickness of oceanic lithosphere, Te, is given approximately by the depth to the `450 +/- 150` °C isotherm" under the Parsons and Sclater cooling plate model, as stated in the Emperor Seamount study below.
- **The compilation range.** The Watts et al. (2013) compilation is described in the same study as a "300-600 °C range of Te values" characterizing "most other seamounts and oceanic islands in the world's ocean basins".
- **Trenches differ from seamounts.** Hunter and Watts (2016) deduce "342-349 °C for the seaward wall of circum-Pacific trenches"; the Emperor Seamounts study finds controlling isotherms of 340 °C (their models A, B, C) and 400 °C (model D), and gives `Te ~ 3.05 sqrt(t_sf - t_l)` for the 400 °C controlling isotherm.

### 2.4 The unit finding (reported, not resolved)

`GEOTHERM_ARC_SCOPE.md` names this "the ~600 K class number" twice ("this is the ~600 K class number", and "The ~600 K limiting isotherm that both shortcuts wanted to author"). Every source read here states the oceanic limiting isotherm in degrees CELSIUS, not kelvin:

| Reading | In °C | In K |
| --- | --- | --- |
| Rheological elastic-core limit (Calmant et al.) | 600 | 873 |
| Watts, classical single number | 450 +/- 150 | 723 +/- 150 |
| Watts et al. 2013 compilation range | 300 to 600 | 573 to 873 |
| Seamounts against isochron age (Calmant et al.) | 350 to 450 | 623 to 723 |
| Trench seaward wall (Hunter and Watts 2016) | 342 to 349 | 615 to 622 |

600 K is 327 °C, which sits BELOW the bottom of every band above except the trench row. The arc's "~600 K" and the literature's "~600 °C" are not the same quantity, and they differ by 273 K in a place where the scope's own reasoning puts this number inside a yield-strength-envelope construction. This is surfaced as a finding for the owner, not corrected here: the scope is the owner's ruling and the number in it is his to set. The coincidence that the trench-derived isotherm (342-349 °C, or 615-622 K) has a KELVIN value near 600 is a numerical accident of two different conventions and must not be used to reconcile the two readings.

### 2.5 Hazard the consumer must know

`Te` is not a depth to a boundary. It is the thickness of the equivalent elastic plate that reproduces the observed flexure, that is a geometric analogue of the lithosphere's integrated strength. Mapping it onto an isotherm is an INTERPRETATION under an assumed thermal model (Parsons and Sclater cooling plate) and an assumed yield-strength envelope, not a measurement. This is why the scope's demotion of the isotherm to a hindcast cross-check is the right call, and it is also why the hindcast must compare DERIVED `Te` against MEASURED `Te`, never derived `Te` against an isotherm depth. (This statement is standard in the flexure literature and is the reading of Watts 2001; it was not read verbatim from the primary and is listed under "Defaults taken".)

**Primary citation.** Calmant, S., Francheteau, J. and Cazenave, A., 1990, "Elastic layer thickening with age of the oceanic lithosphere: a tool for prediction of the age of volcanoes or oceanic crust", Geophysical Journal International 100(1), 59-67, DOI 10.1111/j.1365-246X.1990.tb04568.x (summary; equation (6); Table 2; the isotherm discussion in "Analysis of results"). Read from the open copy at `horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_5/b_fdi_31-32/34824.pdf`.
**Companion primaries (not read this round).** Watts, A. B., 2001, Isostasy and Flexure of the Lithosphere, Cambridge University Press, ISBN 978-0-521-62272-1. Watts, A. B. and Zhong, S., 2000, "Observations of flexure and rheology of oceanic lithosphere", Geophysical Journal International 142, 855-875. McNutt, M. K., 1984, "Lithospheric flexure and thermal anomalies", JGR 89, 11180-11194.
**Source read for the isotherm statements.** The Emperor Seamount Chain elastic-thickness study, Geophysical Journal International 240(1), 61-, `academic.oup.com/gji/article/240/1/61/7826795` (which reports the `450 +/- 150` °C statement, the Watts et al. 2013 compilation range, the Hunter and Watts 2016 trench figures, and its own 340/400 °C controlling isotherms).
**Confidence.** High on `Te = 2.70 sqrt(dt)` with rate standard deviation 0.15, on the exclusion of the south-central Pacific, and on the weighted regression, all read from the primary. High on the isotherm being stated in °C across the literature and on the age-convention dependence. High that the raw data carry large scatter.

---

## 3. Mars and Venus elastic-thickness estimates (hindcast rows two and three)

### 3.1 Mars, per region

Read from Ruiz (2014), Table 1. This is a COMPILATION (a secondary source), peer-reviewed and open, which draws its `Te` values from the primaries named below. Its own note: "the range of elastic thicknesses consistent with all the estimates" was selected where a region had several estimates. Ages are the surface/loading epoch (N = Noachian, H = Hesperian, A = Amazonian).

| Region | Te (km) | Epoch | Surface age (Ga) |
| --- | --- | --- | --- |
| North Pole | > 300 | current | current |
| South Pole | > 110 | current | current |
| Valles Marineris | >= 60 | H-A | 3.6 to 1.8 |
| Alba Patera | 43 to 65 | H-A | < 3.5 |
| Arsia Mons | 20 to 35 | H or A | ~3.5 or lower |
| Pavonis Mons | 50 to 100 | H or A | ~3.6 or lower |
| Ascraeus Mons | 50 to 80 | H or A | ~3.6 or lower |
| Olympus Mons | > 70 | H-A | 3.7 to 2.5 |
| Elysium rise | 36 to 45 | H | 3.7 to 3.0 |
| Isidis Planitia | 100 to 180 | H | 3.7 to 3.0 |
| Noachis Terra | < 12 | N | > 3.8 |
| Terra Cimmeria | < 12 | N | > 3.8 |
| Hellas Basin | < 13 | N | 4.1 to 3.9 |

The compilation's headline reading: "Te estimates proposed for Noachian times are lower than 25 km", while most post-Noachian values exceed 40 to 50 km.

**Method.** Localized gravity/topography admittance and correlation spectra (McGovern et al. 2002, corrected 2004) for most regions; flexural modelling of specific loads for the polar caps. The compilation converts `Te` to heat flow "following the equivalent strength envelope formalism", which is the same construction the arc uses in reverse, so the row is dimensionally compatible with the arc's YSE payoff.

**HAZARD (many of these are one-sided bounds).** Six of the thirteen rows are inequalities rather than intervals. `> 300` km for the north polar cap is a lower bound from a load that the lithosphere barely deflects under; it does not mean 300 km of elastic plate was measured. A hindcast scoring against these rows must score against the bound, not against a midpoint. There is no midpoint.

**HAZARD (the compilation's own caveat).** Ruiz records that many `Te` estimates "are not adequate for performing well-constrained heat flow calculations" because the load curvatures were not always derived.

**HAZARD (a moving target).** The McGovern et al. (2004) correction to McGovern et al. (2002) revised the estimates downward for several regions: "The lower bound on Te at Olympus Mons is 70 km instead of 140 km". Any older citation of the 2002 values is stale by a factor of two at Olympus Mons. Check the publication year of any Mars `Te` number before trusting it.

**DISAGREEMENT (Mars, between compilations).** Ding et al. (2019), via a search summary and not read, report: Noachian southern highlands 20 to 60 km; Hesperian northern lowlands > 90 km; Hesperian Elysium Mons < 55 km; Hesperian/Amazonian Olympus Mons > 105 km. Against Ruiz's Table 1 the Olympus Mons bound (> 105 versus > 70) and the Elysium figure (< 55 versus 36 to 45) do not sit on top of each other, and the Noachian southern-highlands range (20 to 60) is far wider and higher than Ruiz's Noachis Terra and Terra Cimmeria rows (< 12). This is a live disagreement in the Mars `Te` literature and is reported rather than adjudicated. The Ding figures are SUMMARY-ONLY; see "Defaults taken".

### 3.2 Venus, per region

Read from Smrekar and Anderson (2005), the LPSC abstract of the Anderson and Smrekar (2006) global mapping study.

- **Previous local admittance studies (VALUE, range).** "In general previous local admittance models for Venus have found values of 20-50 km for `Zc`, and 5-55 km for `Te`."
- **The global admittance mapping (VALUE, range).** Inverting 360x180 admittance spectra on a 1-degree grid against top-loading, bottom-loading and hot-spot models: "the estimated range of `Zc` (0-90 km) and `Te` (0-100 km) is larger than results for most previous studies, primarily because of the incorporation of bottom loading."
- **The distribution is multimodal (VALUE, load-bearing).** "three ranges of `Te` are common, including values < 20 km, values between 40-70 km, and a few locations with `Te` greater than 90 km". And: "47% of the planet has `Te` < 20 km, for which we cannot distinguish loading from isostasy, suggesting that these regions are tectonically inactive."
- **Coverage and selection.** Best-fit compensation models were fitted to 26 of 35 spectral classes: 15 classes top loading (39% of the surface), 7 bottom loading (35%), 4 hot spot (15%), total 89% coverage. The remaining 9 classes (11% of the surface) "generally had large amplitude top loading signatures that could not be fit with our models". So the Venus `Te` map has an 11% hole by construction, and that hole is not random: it is the places the model class failed.
- **Flexural modelling companion (SUMMARY-ONLY).** Barnett et al. (2002) report `Te` ~10 to 40 km or greater from flexural modelling of residual topography, and 20 to 60 km for seven volcano-like structures, "generally more tightly constrained".

**HAZARD (Venus Te depends on the loading model, over and above the region).** The abstract states plainly: "estimates of `Te` varied significantly between bottom loading and top or hot spot models." The loading model is an assumption, and swapping it moves the answer. A Venus `Te` number without its loading model attached is incomplete.

### 3.3 The planet-wide-Te question (flagged as the ruling requires)

The task asked that a planet-wide single `Te` be flagged as a fiction if a source offered one. **No source read here offers one.** Every source is explicitly regional or explicitly a distribution:

- Mars spans `< 12` km (Noachis Terra) to `> 300` km (north polar cap) in the same compilation, a range of more than an order of magnitude on one planet.
- Venus is reported as trimodal (`< 20`, `40-70`, `> 90` km) with 47% of the planet below 20 km, and the authors decline to distinguish loading from isostasy in that 47%.

So the fiction does not need flagging in a source; it needs flagging in any CONSUMER that reduces these rows to one number per planet. The correct hindcast target is a per-region distribution with its loading model and its epoch attached, and a derived model that produces one `Te` per planet cannot be scored against these rows at all.

**Primary citations.** McGovern, P. J. et al., 2002, "Localized gravity/topography admittance and correlation spectra on Mars: Implications for regional and global evolution", Journal of Geophysical Research: Planets 107(E12), 5136, DOI 10.1029/2002JE001854, AND the correction: McGovern, P. J. et al., 2004, Journal of Geophysical Research: Planets 109, E07007, DOI 10.1029/2004JE002286 (Table 1, the revised best-fitting `Te`). Anderson, F. S. and Smrekar, S. E., 2006, "Global mapping of crustal and lithospheric thickness on Venus", Journal of Geophysical Research: Planets 111, E08006, DOI 10.1029/2004JE002395. Barnett, D. N., Nimmo, F. and McKenzie, D., 2002, "Flexure of Venusian lithosphere measured from residual topography and gravity", Journal of Geophysical Research: Planets 107(E2), DOI 10.1029/2000JE001398.
**Compilation read (secondary, open, primary-citing).** Ruiz, J., 2014, "The early heat loss evolution of Mars and their implications for internal and environmental history", Scientific Reports 4, 4338, DOI 10.1038/srep04338 (Table 1). It traces to McGovern et al. (2004), Zuber et al. (2000), Ruiz et al. (2011), and work by Grott, Kiefer and Wieczorek (2004-2012).
**Conference abstract read (Venus).** Smrekar, S. E. and Anderson, F. S., 2005, "Global admittance estimates of elastic and crustal thickness of Venus: results from top, hot spot, and bottom loading models", Lunar and Planetary Science XXXVI, abstract 1804.
**Confidence.** High on the Mars per-region table as Ruiz reports it and on its epoch column. High on the Venus ranges, the trimodality, the 47% figure, and the model dependence. Medium on the Mars values as absolute truth, given the live disagreement with Ding et al. (2019) and the 2002-to-2004 revision history. The Mars primaries' own tables were not read directly (paywalled, HTTP 403).

---

## 4. Ice thermal conductivity and ice friction (the shell branch, the named deviant)

### 4.1 Ice Ih thermal conductivity, temperature-dependent (the point of the row)

**The new fitted model (PRIMARY, read).** `k(T) = 612 / T` W m^-1 K^-1, with `T` in K. Carnahan et al. (2021) state: "We find that the model `k = 612/T` fits the data well over temperatures ranging from 30 to 273 K". This is their equation (1d).

- **Validity domain.** 30 to 273 K, that is essentially the whole range of an icy shell from surface to melting point.
- **Band.** "Generally experiments are quoted to have accuracy on the order of 10%, which is consistent with the deviation we observe around our fit (see Slack, 1980, and references therein)."
- **Robustness of the coefficient.** "Selectively excluding particular datasets generally resulted in changes to the best fit coefficient within `+/- 5`. Excluding the data of Ashworth (1972) had the largest effect (resulting in a best fit coefficient of 619)". So the coefficient is `612 +/- 5` under dataset resampling, a stability statement rather than a measurement band.
- **Population and selection.** Fitted to a compiled collection of published ice Ih thermal-conductivity data spanning 1929 to 1994 from 11 primary sources, admitted under three stated criteria: the data were referenced in the derivation of a published `k` model for ice Ih; the original source was available for digitization or transcription; and the data were acquired at atmospheric pressure or could be scaled to it. Plot data were digitized with WebPlotDigitizer to 2-3 digits.
- **Why `1/T`.** The inverse-temperature form is "motivated by the simple theoretical model applicable to high temperature, monocrystalline ice at constant volume (Andersson et al., 1980)", and the authors report that adding higher-order coefficients "contribute minimally", so "a model of the form `k = a/T` is sufficient".

**The competing forms (PRIMARY, read, from the same paper's equation (1)).**

| Model | Form | Source |
| --- | --- | --- |
| Constant | `k = 2.26` W m^-1 K^-1 | used at the basal temperature `T_b` |
| Hobbs | `k(T) = 0.4685 + 488.12 / T` | Hobbs (1974) |
| Rabin | `k(T) = 2135 / T^1.235` | Rabin (2000) |
| Carnahan et al. | `k(T) = 612 / T` | Carnahan et al. (2021) |

**The classical form (read from a source that cites the primary).** `k_S,C(T) = 567 / T` W m^-1 K^-1 for crystalline hexagonal ice, "which is about 7.1 W/m/K at `T = 80` K (Klinger 1980)", as reported by Ferrari and Lucas (2016) section 2.1.

**DISAGREEMENT (report, do not pick).** The four temperature-dependent forms do not agree within the quoted 10% experimental accuracy across the range. Evaluated at 100 K they give: Klinger `567/T` = 5.67; Carnahan `612/T` = 6.12; Hobbs `0.4685 + 488.12/T` = 5.35; Rabin `2135/T^1.235` = 7.23 W m^-1 K^-1. The spread from Hobbs to Rabin is a factor 1.35, well outside 10%. Carnahan et al. state the practical consequence: the commonly used model "may under predict the thermal conductivity by approximately a fifth", and works assuming a constant `k` "could underpredict the thermal conductivity by an order of magnitude" at the cold surface. There is no consensus single form; the choice is a modelling decision with a stated ~20% consequence near the surface.

**HAZARD (sample-dependence).** Carnahan et al.: "thermal conductivity data are highly sensitive to sample characteristics and preparation, such as anisotropy (Klinger, 1975) and freezing rate (Bonales et al., 2017)." The ~10% accuracy is per-experiment; the inter-experiment spread is what the fit's deviation captures.

**The alien row, made concrete.** Amorphous ice yields "values about 0.2 W/m/K (Andersson & Suga 1994; Klinger 1980)" per Ferrari and Lucas (2016), against ~7 W/m/K for crystalline ice at 80 K. That is a factor of ~35 at the same temperature, from phase alone. This is the strongest available support for the scope's conditioning line: a `k` row that keys only on "ice" and not on the material's own state is wrong by more than an order of magnitude for a plausible shell. The `k` row must key on material class AND phase.

### 4.2 Ice friction, the brittle branch (SUMMARY-ONLY, from the abstract)

Beeman, Durham and Kirby (1988), from the published abstract: triaxial testing of pure water ice cylinders containing a 45-degree inclined sawcut, at `77 <= T <= 115` K and confining pressures `0.1 <= P <= 250` MPa, gives

`tau = 0.20 sigma_n + 8.3` MPa for `P >= 10` MPa
`tau = 0.55 sigma_n + 1.0` MPa for `P <= 5` MPa

with `tau` and `sigma_n` the shear and normal stresses on the sawcut. The abstract states that friction "is independent of `T` and, over the one order of magnitude tested, of average sliding velocity", that "the sliding behavior is invariably stick slip", and that "the frictional strength of ice seems to be well below that for all other rocks".

**Validity domain.** 77 to 115 K. This is a COLD-ice calibration: it is Ganymede-and-outward temperatures, not the warm ice near a Europa-class shell's base. The abstract's claim of `T`-independence is asserted over that 38 K window only, and must not be extrapolated to ice near its melting point, where premelting and the whole ice-friction-at-high-homologous-temperature literature take over.

**HAZARD (a gap in the fit).** The two branches are calibrated for `P >= 10` MPa and `P <= 5` MPa. The interval `5 < P < 10` MPa is covered by neither expression. The two lines cross where `0.20 sigma_n + 8.3 = 0.55 sigma_n + 1.0`, that is at `sigma_n = 20.9` MPa, which is OUTSIDE both stated domains. So the two branches cannot be joined at their intersection without leaving the calibrated range, and a naive `min` or `max` of the two will produce a discontinuity or an unphysical kink somewhere in `5` to `10` MPa. This is an integration decision the arc must make explicitly.

### 4.3 Byerlee's law, the rock anchor the ice deviates FROM (PRIMARY, read)

Included because "ice is the named deviant" has no meaning without the thing it deviates from, and because the primary carries a unit trap.

Byerlee (1978), read from the paper's own text, section "Discussion":

`tau = 0.85 sigma_n` for `sigma_n < 2` kb
`tau = 0.5 + 0.6 sigma_n` for `2 kb < sigma_n < 20` kb

**HAZARD (UNITS, severe, a 100x scale offset).** Byerlee's original equations are in KILOBARS, not MPa. `1 kb = 100 MPa`. In MPa the second relation reads `tau = 50 + 0.6 sigma_n` MPa over `200 MPa < sigma_n < 2000 MPa`. A consumer that copies `tau = 0.5 + 0.6 sigma_n` and feeds it MPa gets a cohesion of 0.5 MPa instead of 50 MPa, a factor of 100 too small, and the error is silent because the equation stays dimensionally plausible. This is exactly the class of hazard the fetch format exists to catch.

**HAZARD (a rounded upper limit in the secondary literature).** Secondary sources commonly state the upper validity limit as 1700 MPa. The primary says `20 kb`, which is 2000 MPa. Prefer the primary's 2 kb and 20 kb.

**Why it serves the alien (the scope's claim, checked against the primary).** Byerlee states at high normal stress the friction "is nearly independent of rock type", and that "roughness has little or no effect on friction" at those stresses. The primary supports the scope's reasoning that one friction law serves every silicate lid. The primary ALSO shows the limit of that claim: at low normal stress (up to 50 bars, that is 5 MPa) the data are dominated by Barton's civil-engineering compilation and "the variation in friction is due to the variation of friction with surface roughness". So the material-independence is a HIGH-STRESS property. A thin, low-gravity lid whose brittle layer never reaches 200 MPa of normal stress is operating in the regime where Byerlee's universality does not hold and roughness matters. That is a real alien-admission caveat on the arc's step 2.

**Primary citations.** Carnahan, E., Wolfenbarger, N. S., Jordan, J. S. and Hesse, M. A., 2021, "New insights into temperature-dependent ice properties and their effect on ice shell convection for icy ocean worlds", Earth and Planetary Science Letters 563, 116886, DOI 10.1016/j.epsl.2021.116886, arXiv 2011.12502 (equations 1a-1d; the `612/T` fit; the 30-273 K range; the 10% accuracy statement). The companion open dataset: "A comprehensive dataset for the thermal conductivity of ice Ih for application to planetary ice shells", Data in Brief, PMC8134708, Mendeley Data DOI 10.17632/ttzbgxs9fw.1 (the 11-source compilation and its three inclusion criteria). Klinger, J., 1980 (the `567/T` crystalline-ice form). Hobbs, P. V., 1974, Ice Physics, Clarendon Press. Rabin, Y., 2000. Beeman, M., Durham, W. B. and Kirby, S. H., 1988, "Friction of ice", Journal of Geophysical Research: Solid Earth 93(B7), 7625-7633, DOI 10.1029/JB093iB07p07625. Byerlee, J., 1978, "Friction of rocks", Pure and Applied Geophysics 116, 615-626, DOI 10.1007/BF00876528, read from the open USGS copy at `earthquake.usgs.gov/static/lfs/research/rockphysics/Friction_of_rocks.pdf`.
**Source read for the classical form and the amorphous contrast.** Ferrari, C. and Lucas, A., 2016, "Low thermal inertias of icy planetary surfaces: evidence for amorphous ice?", Astronomy and Astrophysics 588, A133, DOI 10.1051/0004-6361/201527625, section 2.1.
**Confidence.** High on the `612/T` fit, its 30-273 K range, the `+/- 5` coefficient stability, the ~10% accuracy, and the three competing forms with their coefficients, all read from the primary. High on the `567/T` classical form and the ~0.2 W/m/K amorphous value (read from a source citing Klinger 1980). High on Byerlee's two relations, their kbar units, and the rock-type-independence and roughness statements, read from the primary. LOW-MEDIUM on the Beeman ice-friction coefficients: read from the abstract only, not the paper's text.

---

## 5. Hartmann crater saturation (gates the separate saturation task)

### 5.1 The definitions, which is where this number's meaning lives

Three distinct things are routinely called "saturation", and the source literature says the conflation has caused real damage. Minton et al. (2019), read: the mixing of equilibrium "with the geometry-based construct of saturation into the amalgamation 'saturation equilibrium'" has "no doubt been responsible for a great deal of confusion."

- **Geometric saturation** (Gault 1970 called it simply "saturation"; Melosh 1989 and modern usage call it geometric): the maximum theoretical packing density of circular features, craters of the same size placed rim-to-rim in a hexagonal close-packed arrangement. A geometric limit, containing no physics.
- **Equilibrium** (Gault 1970; Marcus 1970): a DYNAMICAL balance in which each new crater destroys, on average, one old crater of the same size, so the observed density stops rising. A process statement.
- **Empirical saturation** / **saturation equilibrium** (Hartmann 1984; the term "empirical saturation" is from Basaltic Volcanism Study Project 1981 chapter 8, written by a team led by Hartmann): the OBSERVED upper-limit crater density measured on the most heavily cratered real surfaces. An observational statement.

**Hartmann's number is the third of these, and it is explicitly NOT the first.** Hartmann and Morbidelli (2020), with Hartmann as first author, read: "The observed saturation curve is not a 'geometric' saturation, since more small craters could, geometrically, be squeezed into existing empty spaces on these surfaces. However, nature does not allow the addition of craters in only a narrow size range; the whole SFD must be added all at once, which requires occasional, large, basin-scale impacts, whose excavation and ejecta blankets completely resurface some areas."

**And it is not an equilibrium in the stability sense.** Same source: "'equilibrium' connotes stability, whereas empirical saturation in a given region does not represent a static situation." The observed density oscillates: "different parts of the saturation line can thus oscillate over time by factors about 2 to 4 (Hartmann and Gaskell 1997). The dashed lines on either side of the saturation curve thus show the typical range of oscillation of visible saturation crater densities."

**The factor 2 to 4 oscillation IS the band on this row.** It is not measurement error; it is the physical variability of the quantity.

### 5.2 The values

**Geometric saturation (PRIMARY-quoting, read).** In cumulative-number-per-unit-area notation, `n_geom,r = 0.385 r^-2` (Gault 1970), as given by Minton et al. (2019). Because the slope is exactly 2 and cumulative SFDs are per unit area, the coefficient is DIMENSIONLESS, a consequence of geometric similarity.

**HAZARD (radius versus diameter, a factor of 4).** The `0.385` coefficient is defined on RADIUS. Converting to diameter with `r = D/2` gives `n_geom,D = 0.385 (D/2)^-2 = 1.54 D^-2`, which is the form usually quoted in the diameter-based crater literature. The two coefficients differ by exactly 4 and describe the same curve. Reading `0.385` as a diameter coefficient understates geometric saturation fourfold.

**The fraction of geometric saturation at which surfaces equilibrate (the number the task asked for).**

| Source | Fraction of geometric saturation | Coefficient (Minton notation, cumulative in radius) | Slope beta |
| --- | --- | --- | --- |
| Gault (1970) | 1 to 10% at all crater sizes | `n_eq,r = 0.021 +/- 0.017` | 2 |
| Xiao and Werner (2015) | 0.69 to 3.9% (terrains with equilibrium at `r < 500` m) | `n_eq,r = 0.009 +/- 0.006` | ~2 |
| Hartmann (1984) | shallower curve, see below | `n_eq,r = 0.0064 r^-1.83` | 1.83 |
| Minton et al. (2019) own fit | 2.2% | (their Fit 1, Apollo 15 site) | fitted |

Gault's "1 to 10% of geometric saturation" is the canonical answer to "what fraction". Minton et al. read it as "an empirical estimate of the line of `n_eq,r = 0.021 +/- 0.017` and `beta = 2`", so the `+/- 0.017` is Gault's 1-to-10% band re-expressed as a coefficient, not an independent uncertainty.

**Xiao and Werner disagree with Gault (report, do not pick).** Their measured 0.69 to 3.9% is "lower than that estimated by Gault (1970)", though "they found that the equilibrium slope was consistently `beta ~ 2` across multiple terrains". So the SLOPE is robust and the LEVEL is contested by a factor of ~2 to 3 between the founding estimate and the modern count.

**Hartmann's own line and its slope.** Minton et al. render Hartmann (1984) in their notation as `n_eq,r = 0.0064 r^-1.83` with `beta = 1.83`, describing it as "a similar, though somewhat shallower empirical equilibrium SFD" built from "observed crater densities across both maria and highlands terrains". The units of the `0.0064` coefficient follow from the form: with `n` per unit area and `beta = 1.83 != 2`, geometric similarity is broken and the coefficient is DIMENSIONFUL, carrying `length^(beta-2) = length^-0.17`. The PDF's own rendering of that unit did not extract cleanly and is flagged below.

**HAZARD (Hartmann's fraction is size-dependent, by construction).** Because Hartmann's slope is 1.83 and geometric saturation's is exactly 2, the ratio between them is NOT constant with size. "Hartmann saturation is X% of geometric saturation" is only true at one crater size. Evaluated at `D = 1` km (`r = 500` m), Minton's rendering of Hartmann's line gives `0.0736` km^-2 against geometric saturation of `1.54` km^-2, that is 4.8% of geometric saturation. At any other diameter that percentage moves. A consumer that hardcodes a single percentage has silently authored a slope of 2.

**Hartmann's own density anchor (read).** Hartmann and Morbidelli (2020) state the saturation level as a multiple of a measured reference rather than as an absolute: lunar highland surfaces match "saturation equilibrium as measured by Hartmann (1984)" at "~32" times the average post-mare (mare) crater density, and the paper's figures use "saturation density (32x average mare density)". So Hartmann's saturation is anchored to the lunar mare density, which is itself a measured, dated quantity. It is a RATIO to a Terran-system observable, and a consumer must resolve that anchor before the number means anything off the Moon.

### 5.3 A numerical disagreement between two renderings of Hartmann (1984)

The commonly quoted diameter form of Hartmann's saturation line is `log10 N_S = -1.33 - 1.83 log10 D_cr` with `D` in km and `N` per km^2, giving `N = 0.047` km^-2 at `D = 1` km (SUMMARY-ONLY; see below). Minton et al.'s radius-notation rendering, read from their text, gives `0.0736` km^-2 at the same size. These differ by a factor of **1.57**, and a cumulative SFD evaluated at `r = 500` m and at `D = 1` km must give the same number, so the two are not trivially reconcilable.

The most likely cause is a CONVENTION difference rather than an error: Hartmann's own diagrams are INCREMENTAL (his isochron plots bin craters in `sqrt(2)`-diameter bins, and Hartmann and Morbidelli describe using a "bin, rather than smoothing the curve's structure over many sizes, as cumulative curves do"), whereas Gault, Xiao and Werner, and Minton all work in CUMULATIVE SFDs. Silently mixing an incremental coefficient into a cumulative formula is a several-fold error. Both renderings do land inside Gault's 1-to-10% band (3.0% and 4.8% respectively at `D = 1` km), so neither is absurd, which is what makes this hazard dangerous rather than obvious.

**This disagreement is NOT resolved here.** Hartmann (1984) itself is a 1984 Icarus paper that could not be reached in readable form. The incremental-versus-cumulative convention MUST be settled against the primary before either coefficient is coded.

**Primary citations.** Hartmann, W. K., 1984, "Does crater 'saturation equilibrium' occur in the solar system?", Icarus 60(1), 56-74, DOI 10.1016/0019-1035(84)90138-6. Gault, D. E., 1970, "Saturation and equilibrium conditions for impact cratering on the lunar surface: criteria and implications", Radio Science 5, 273-291, DOI 10.1029/RS005i002p00273. Hartmann, W. K. and Gaskell, R. W., 1997, "Planetary cratering 2: Studies of saturation equilibrium", Meteoritics and Planetary Science 32, 109-121, DOI 10.1111/j.1945-5100.1997.tb01246.x. Basaltic Volcanism Study Project, 1981, Basaltic Volcanism on the Terrestrial Planets, Pergamon, chapter 8 (the "empirical saturation" term).
**Sources read.** Minton, D. A., Fassett, C. I., Hirabayashi, M., Howl, B. A. and Richardson, J. E., 2019, "The equilibrium size-frequency distribution of small craters reveals the effects of distal ejecta on lunar landscape morphology", Icarus (accepted 19 February 2019), arXiv 1902.07746, sections 1.1 and 1.2. Hartmann, W. K. and Morbidelli, A., 2020, "Effects of early intense bombardment on megaregolith evolution and on lunar (and planetary) surface samples", arXiv 2010.14275. Xiao, Z. and Werner, S. C., 2015, "Size-frequency distribution of crater populations in equilibrium on the Moon", Journal of Geophysical Research: Planets 120, DOI 10.1002/2015JE004860 (read via Minton et al.'s quotation of it).
**Confidence.** High on the three definitions and on Hartmann's saturation being empirical rather than geometric, read from Hartmann's own text. High on the factor-2-to-4 oscillation band, read from Hartmann's own text. High on Gault's 1-to-10% and the `0.385 r^-2` geometric form, read from Minton et al. High on the Xiao and Werner disagreement with Gault. MEDIUM on Hartmann's own coefficient, which carries the unresolved 1.57x discrepancy and an unrecovered unit exponent.

---

## Defaults taken

Nothing in this document was defaulted, guessed, or interpolated: no value here is a digit I chose. What follows is the list of things a consumer will need that this fetch could NOT source to the standard the arc requires. Every one is a row to close before the corresponding build step, and three of them are blockers.

**BLOCKER 1. Hirth and Kohlstedt (2003) Table 1 was never read.** The primary is a paywalled AGU monograph chapter (DOI 10.1029/138GM06). Every access route failed: Wiley (paywall), an escholarship PDF (unparseable streams), the Kohlstedt Treatise chapter (cites Table 1, does not reprint it), Bürgmann and Dresen 2008 (its olivine flow-law table is Supplemental Material, not in the PDF), a DocsLib copy of the chapter text (contains the sentence "the values of the flow law parameters are summarized in Table 1" but not the table). The four-row parameter set in section 1.2 therefore rests on a machine-readable reproduction (UWGeodynamics) plus two peer-reviewed papers that quote individual values. **The dry-olivine activation volume carries an unresolved conflict** (reproduction 6 cm^3/mol against Dixon and Durham's report of Hirth and Kohlstedt's 13-27 cm^3/mol). Since `V*` sits in an exponential, this is not a cosmetic disagreement. Read Table 1 before coding the creep anchors.

**BLOCKER 2. Hartmann (1984)'s own saturation equation was never read.** Icarus 60, 56-74 was not reachable in readable form. The two available renderings of his line disagree by a factor of 1.57 at `D = 1` km, probably through an incremental-versus-cumulative convention difference. The commonly quoted diameter form `log10 N_S = -1.33 - 1.83 log10 D_cr` (`~0.047` km^-2 at `D = 1` km) is **SUMMARY-ONLY**: it was reported by a search summary and was never read from any source's text. Do not code it.

**BLOCKER 3. Hartmann's coefficient unit did not extract.** In Minton et al.'s rendering `n_eq,r = 0.0064 r^-1.83`, the exponent on the coefficient's length unit came through the PDF's font encoding as an unrecoverable glyph sequence. Dimensional analysis of their own definition requires `length^-0.17`, and that is stated as a derivation from the quoted form rather than as a read value. Confirm against the published version.

**4. The Beeman et al. (1988) ice-friction constants are SUMMARY-ONLY.** The two relations (`tau = 0.20 sigma_n + 8.3` MPa for `P >= 10` MPa; `tau = 0.55 sigma_n + 1.0` MPa for `P <= 5` MPa), the `77 <= T <= 115` K window, the `0.1 <= P <= 250` MPa range, and the `T`-independence and stick-slip claims all come from a search summary of the published abstract. The paper's text was not read. The whole ice brittle branch rests on this row, so it should be read before use. Related open gap: the `5 < P < 10` MPa interval is covered by neither branch, and the branches cross at `sigma_n = 20.9` MPa, outside both stated domains.

**5. The Hirth and Kohlstedt stress-exponent band is SUMMARY-ONLY and contested.** Both `n = 3.5 +/- 0.3` and `n = 3.5 +/- 0.5` are attributed to the same primary by different sources, neither read from the primary. The central value 3.5 is solid and multiply corroborated; the band is not.

**6. The Ding et al. (2019) Mars figures are SUMMARY-ONLY.** The disagreement reported in section 3.1 (Olympus Mons `> 105` km against Ruiz's `> 70` km, and the Noachian southern highlands at 20-60 km against `< 12` km for Noachis Terra) rests on a search summary. The disagreement is real enough to report and too unverified to adjudicate. The Mars admittance primaries (McGovern et al. 2002 and the 2004 correction) both returned HTTP 403 and their tables were not read; the Mars row comes from Ruiz's compilation of them.

**7. The "Te is not a depth to a boundary" statement is SUMMARY-ONLY.** The formulation in section 2.5 (that `Te` is a purely geometric analogue of integrated strength rather than a depth to any lithospheric boundary) is standard and is the reading of Watts (2001), but it was reported by a search summary and not read verbatim from the primary. The physics of the caveat is independently supported by what WAS read (the same `Te` data mapping to 350-450 °C or 550-600 °C depending only on the age convention, which is impossible for a real material boundary).

**8. The scope's claim that rock thermal conductivity is not strongly temperature-dependent was not sourced.** `GEOTHERM_ARC_SCOPE.md` conditions the `k` row on the contrast "ice conductivity is strongly temperature-dependent where rock's is not". The ice half is now sourced hard (`612/T` over 30-273 K). The ROCK half was not fetched this round: no source read here characterizes silicate `k(T)` or its temperature sensitivity. The conditioning line's premise is therefore half-verified. Since the whole justification for keying `k` on material class rests on the contrast, the rock half should be fetched before the row is called grounded. Note the contrast survives regardless on other grounds: the crystalline-versus-amorphous ice spread (a factor ~35 at 80 K) is by itself enough to require the key.

**9. Watts and Zhong (2000) and Watts (2001) were not read.** The oceanic row is pinned to Calmant et al. (1990), read in full, which is a primary compilation and fit. The task named Watts's work as the classical source; the classical `450 +/- 150` °C statement and the Watts et al. (2013) compilation range are reported here through the Emperor Seamount study that quotes them, not from Watts directly. Watts et al. (2013) itself was not located this round.

**10. The Venus primaries were not read.** The Venus row comes from Smrekar and Anderson's 2005 LPSC abstract, which is the conference version of Anderson and Smrekar (2006). Barnett et al. (2002) is SUMMARY-ONLY. The Venus numbers are ranges and modes rather than per-feature values with bands, and a per-region Venus table comparable to the Mars table was not assembled.

**11. The unit finding in section 2.4 is surfaced, not resolved.** The arc scope's "~600 K" does not match any isotherm the literature states, all of which are in degrees Celsius and none of which is 327 °C. This is the owner's number in the owner's ruling and is his to set or correct. It is recorded here because a fetch that quietly returned "600 °C" against a ruling that says "600 K" would have hidden the discrepancy behind a plausible digit.
