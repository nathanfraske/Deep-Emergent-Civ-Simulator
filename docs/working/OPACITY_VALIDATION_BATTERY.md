# The opacity validation battery (Bell-Lin 1994 / Semenov 2003 as targets, not the floor)

This is the pre-registered validation battery for the disk-opacity GENERATOR (capstone front-end slice 3c-i, the
owner-redirected build). Bell & Lin 1994 and Semenov 2003 are NOT the floor: the owner ruled their piecewise fits
held permanently (they are solar-composition compressions that bake one grain model and fixed regime boundaries
into dimensional coefficients, violating the dimensionless-constant law and admit-the-alien). They re-enter here as
VALIDATION TARGETS: the generator, run at solar composition, must land inside their mutual envelope. Their factor-2
to factor-3 disagreement is the honest band, booked as measured intra-generation spread.

## The primary ladder, verified against the source

Bell, K. R. & Lin, D. N. C. 1994, ApJ 427, 987, Appendix "OPACITY," Table 3, p. 1003. The law is
`kappa = kappa_i * rho^a * T^b` in cgs (kappa in cm^2/g, rho in g/cm^3, T in K), over eight regions in ascending
temperature. Transcribed from the ADS page scan and VERIFIED by reading the rendered Table 3 image directly (the
prove-before-trust pass): the eight rows below match the printed table exactly.

| # | Region | Dominant source | kappa_i (cgs) | a | b |
|---|---|---|---|---|---|
| 1 | Ice grains | ice grains | 2e-4 | 0 | 2 |
| 2 | Evaporation of ice grains | ice sublimation | 2e16 | 0 | -7 |
| 3 | Metal grains | dust / silicate grains | 0.1 | 0 | 1/2 |
| 4 | Evaporation of metal grains | dust / silicate sublimation | 2e81 | 1 | -24 |
| 5 | Molecules | molecular gas | 1e-8 | 2/3 | 3 |
| 6 | H-scattering | H- | 1e-36 | 1/3 | 10 |
| 7 | Bound-free and free-free | Kramers | 1.5e20 | 1 | -5/2 |
| 8 | Electron scattering | Thomson | 0.348 | 0 | 0 |

## The paper's own construction validates the redirect

Two verbatim points from the Appendix, read from the scan:

- "**Transitions occur where kappa_i = kappa_{i+1}** and are smoothed following the method of Lin & Papaloizou
  (1985)." So the regime boundaries (the ice line, the sublimation fronts) are the CROSSINGS of adjacent segments,
  never independently authored temperatures. This is the boundaries-from-crossings principle (Principle 8) stated
  by the source itself; our generator computes the same crossings from its own segments (or, better, from the
  disposer's condensation fronts).
- "a priority is given to a good fit to tabulated values rather than to **explicit temperature dependence as
  defined from atomic principles**." The paper openly says its coefficients are FITS, not atomic-principle
  derivations (which is why bound-free/free-free is fit as `T^-5/2` rather than the textbook Kramers `T^-7/2`, and
  why `kappa_i` carries cgs dimensions). This is exactly the structure the owner's redirect banished: the
  generator derives what these fits compress.

## The three pre-registered gates (the owner's ruling)

1. **Solar-composition reproduction inside the envelope.** The generator, at solar composition, must reproduce the
   Bell-Lin ladder to within the Bell-Lin / Semenov mutual band (a factor of a few in magnitude; the shape near
   sublimation and across the molecular gap is where they most disagree). Not a point match, an envelope match.
2. **The 0.348 electron-scattering digit.** Row 8 is `kappa = 0.348`, constant in rho and T. This is the Thomson
   value `0.2(1+X)` at solar `X ~ 0.74`. Our derivation `kappa_es = sigma_T(1+X)/(2 m_H) = 0.1989(1+X)` lands 0.348
   at X=0.75 (0.1989 x 1.75 = 0.348); Bell-Lin rounds `sigma_T/(2 m_H)` to 0.2, so the derived coefficient is the
   more precise one and both reproduce the printed 0.348. VERIFIED by hand.
3. **The ice-line jump at the disposer's front.** The opacity drop at the ice line must EMERGE where the
   condensation front (the Verdict disposer's gas-solid flip, Clausius-Clapeyron carried) places it, not at a
   hardcoded temperature. Bell-Lin's own crossing `2e-4 T^2 = 2e16 T^-7 -> T = 10^(20/9) = 166.8 K` (rho-
   independent) is the fixed-temperature PROXY for that front; our front is pressure-dependent and strictly better.

## The crossing temperatures (Bell-Lin's own boundaries, arithmetic verified)

Each is the solution of `kappa_i = kappa_{i+1}`. Spot-verified: the ice line `10^(20/9) = 166.8 K` and the
electron-scattering coefficient `0.2 x 1.74 = 0.348` both check by hand.

- 1<->2 ice line: `T = 10^(20/9) = 166.8 K` (rho-independent). In the 150-170 K window.
- 2<->3: `T = 202.7 K` (rho-independent).
- 3<->4 (metal-grain evaporation onset): `T = 2286.8 * rho^(2/49) K` (~900-980 K over rho 1e-10 to 1e-9).
- 4<->5 (dust sublimation completion): `T = 2029.6 * rho^(1/81) K` (~1530-1615 K). In the 1400-1600 K window.
- 5<->6: `T = 1e4 * rho^(1/21) K`.
- 6<->7: `T = 31190 * rho^(4/75) K`.
- 7<->8: `T = 1.794e8 * rho^(2/5) K`.

## Semenov 2003 cross-check and the honest band

Semenov et al. 2003 (A&A 410, 611) give graphical opacities with six dust regimes; the check is on the physical
transition temperatures, not on fitted coefficients. Agreement: the ice line (Semenov ~155-165 K vs Bell-Lin
166.8 K) and the silicate/iron sublimation (Semenov ~1500 K vs Bell-Lin ~1530-1615 K) both agree to ~10 K and to a
factor of a few in magnitude. Material disagreements (the honest band the generator must sit within): Bell-Lin
lumps all refractory grains into one power law while Semenov resolves organics (~425 K) and troilite (~680 K)
steps; through ~1000-1500 K Bell-Lin under-predicts opacity (its metal-grain evaporation starts ~900-1000 K and
plunges as `T^-24`, while Semenov holds silicate+iron opacity to ~1500 K). Zhu et al. 2009 note the Bell-Lin fit
"has a lower dust sublimation temperature" and "lacks water vapor and TiO opacity around 2000 K."

## Citation-hygiene seam (do not conflate)

Many papers cite "Bell & Lin 1994" while using a different ladder. Two look-alikes:
- **Lin & Papaloizou 1985** (as in Mueller & Kley 2012 Table 1): 7 regimes, dust grains `5e-3 T^1` (not Bell-Lin's
  `0.1 T^(1/2)`), no electron-scattering regime. Its `rho^(1/21)` and `rho^(4/75)` transition exponents are
  identical to Bell-Lin's (shared molecular / H- / Kramers segments), which is why the two are confused.
- **Bell et al. 1997** low-T refinement: replaces Bell-Lin regimes 1-3 with a finer 7-segment dust fit below
  3730 K, keeping regimes 4-8. The pure 1994 ladder is the eight rows above.

## Sources

Primary: Bell & Lin 1994, ApJ 427, 987, Appendix "OPACITY," Table 3, p. 1003 (verified against the ADS page scan).
Coefficient corroboration: Stamatellos et al. 2007 (A&A 475, 37) Table 1; Coleman et al. 2021 (arXiv 2107.00380)
Eq. 11. Cross-check: Semenov et al. 2003 (A&A 410, 611). Fit-limitation note: Zhu et al. 2009 (ApJ 694, 1045).
Do-not-conflate sibling: Mueller & Kley 2012 (A&A 539, A18) = Lin & Papaloizou 1985.
