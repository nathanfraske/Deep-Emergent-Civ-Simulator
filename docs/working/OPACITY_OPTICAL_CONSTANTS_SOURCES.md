# Optical-constants source map for the disk-opacity generator (the [M] inputs)

This is the source map for the per-species measured optical constants `n(lambda), k(lambda)` the disk-opacity
GENERATOR (capstone front-end 3c-i) consumes through its Mie + effective-medium step. These are the primary [M]
inputs the owner's redirect pointed the fetch at, in place of the Bell-Lin / Semenov fits (which are the validation
battery, `OPACITY_VALIDATION_BATTERY.md`, not the floor). Each species is refutable without simulation and
alien-composable: a carbon-rich or metal-poor disk is a different membership over the same generator, not a
rewrite. Located and characterized by a cited-fetch subagent that sampled the live files directly; the loader
verifies the ingested values against the files at build time.

## The species, their primary sources, and file formats

### 1. Astronomical silicate and graphite: Draine 2003 (primary, live-sampled)

Draine & Lee 1984 (ApJ 285, 89) updated in Draine 2003 (ApJ 598, 1017), from Bruce Draine's own site (the primary
distribution). Index: `https://www.astro.princeton.edu/~draine/dust/dust.diel.html`; data dir:
`https://www.astro.princeton.edu/~draine/dust/diel/`.

- Astronomical silicate: `callindex.out_silD03` (ICOMP=17).
- Graphite (anisotropic): `callindex.out_CpaD03_{0.01,0.10}` (E parallel) and `callindex.out_CpeD03_{0.01,0.10}`
  (E perpendicular), at two grain radii; average with the 1/3 parallel, 2/3 perpendicular rule.

Format: 4 metadata lines + 1 column-header line, then rows of `wave(um)  eps_1-1  eps_2  Re(n)-1  Im(n)`, five
columns, wavelength DESCENDING. The loader forms `n = 1 + (Re(n)-1)` (column 4 plus one) and `k = Im(n)`
(column 5). Coverage ~6.2e-4 to 1.24e5 micron; 837 wavelengths (silicate), 386 (graphite). Sampled: silicate at
1.0 micron n=1.6863, k=0.030770; graphite E-parallel at 1.0 micron n=2.2355, k=0.069422.

### 2. Water ice: Warren & Brandt 2008 (primary compilation, live-sampled)

Warren & Brandt 2008 (JGR 113, D14220), supersedes Warren 1984. Page: `https://atmos.uw.edu/ice_optical_constants/`;
file: `.../IOP_2008_ASCIItable.dat`. Format: no header, 3 columns `wavelength(micron)  n  k`, wavelength ASCENDING.
Crystalline ice Ih only, a single blended n,k table (reference temperature varies by region, near 266 K in the
far-IR). Coverage 0.0443 micron to 2e6 micron. Sampled: 0.5 micron n=1.3130, k=5.889e-10; 1.0 micron n=1.3015,
k=1.62e-6; 45.5 micron n=1.1853, k=0.6782. AMORPHOUS-ICE GAP: Warren-Brandt is crystalline only; the amorphous
branch primaries are Mastrapa et al. 2008 (Icarus 197, 307) / 2009 (ApJ 701, 1347) for 1.1-2.6 micron and Hudgins
et al. 1993 (ApJS 86, 713) for the mid-IR.

### 3. Iron, troilite (FeS), amorphous carbon: the Jena DOCCD (primary papers cited; live host SSHADE)

The Jena Database of Optical Constants for Cosmic Dust (Jaeger et al. 2003, JQSRT 79, 765). The legacy
`astro.uni-jena.de/Laboratory/OCDB` now permanently redirects to the institute homepage; the live citable host is
SSHADE, `https://www.sshade.eu/db/doccd` (DOI 10.26302/SSHADE/DOCCD). Only part of the legacy collection is
migrated, so plan to pull from SSHADE or hold local copies of the specific author files. Files are ASCII n,k tables
with per-file headers (column order varies per contributed dataset, so key off each file's own header).

- Metallic iron (Fe): Ordal et al. 1988 (Applied Optics 27, 1203), the standard IR set (the one Pollack 1994 used).
- Troilite (FeS): Begemann et al. 1994 (ApJ 423, L71); Henning & Stognienko 1996 (A&A 311, 291) is the FeS set
  DSHARP adopts.
- Amorphous carbon: Jaeger, Mutschke & Henning 1998 (A&A 332, 291), the cel400/600/800/1000 pyrolysis series
  (increasing graphitization with temperature); or Zubko et al. 1996 (MNRAS 282, 1321), the ACAR/ACH2/BE samples.
  Refractory organics (CHON) sit in the same carbonaceous group.

### 4. Pollack et al. 1994 reference composition (primary table, read from the scan)

Pollack et al. 1994 (ApJ 421, 615), Table 2 ("Grain Species and Properties," p. 621). The disk mass fractions
(relative to total gas+solid, so they sum to the dust-to-gas ratio ~0.014) for the "Disk, This Paper" column, with
bulk densities (g/cm^3):

| Species | Bulk density | Disk mass fraction (of total) |
|---|---|---|
| Water ice | 0.92 | 5.55e-3 |
| Refractory organic (CHON, C:H:O:N = 1:1:0.5:0.12) | 1.5 | 3.53e-3 |
| Olivine (Fe/(Fe+Mg) = 0.30) | 3.49 | 2.64e-3 |
| Orthopyroxene | 3.40 | 7.70e-4 |
| Troilite (FeS) | 4.83 | 7.68e-4 |
| Volatile organic (CH3OH, H2CO) | 1.0 | 6.02e-4 |
| Metallic iron | 7.87 | 1.26e-4 (rises to 6.14e-4 above 680 K as FeS -> Fe + H2S) |

Solids-only normalization (the column sums to 0.013986; arithmetic shown so nothing is fabricated): water ice
39.7%, refractory organics 25.2%, silicates together 24.4% (olivine 18.9% + orthopyroxene 5.5%), troilite 5.5%,
volatile organics 4.3%, metallic iron 0.9%. This is the canonical ~40% ice / ~25% organics / ~24% rock split. Note
the generator does NOT read these as authored weights: the disposer's condensate fractions at (T, P) supply the
per-species mass fractions, and Pollack is the solar-composition VALIDATION target the disposer must reproduce, so
the dust-to-gas ratio becomes computed rather than the authored 0.01.

Modern faithful reproduction (secondary, pairs each species to its optical-constants file): the DSHARP mix,
Birnstiel et al. 2018 (ApJL 869, L45) via Ricci et al. 2010: water ice 0.2000 (Warren-Brandt 2008), astronomical
silicate 0.3291 (Draine 2003), troilite 0.0743 (Henning-Stognienko 1996), refractory organics 0.3966
(Henning-Stognienko 1996), Bruggeman-mixed, bulk density 1.675 g/cm^3. DSHARP drops metallic iron and volatile
organics and sets ice lower, so it is a variant, not identical to Pollack Table 2.

## Honest gaps

- The live Jena `astro.uni-jena.de/Laboratory/OCDB` direct file index is gone (permanent redirect); SSHADE is the
  working substitute, with only part of the collection migrated.
- Warren-Brandt is crystalline ice only; amorphous ice needs Mastrapa / Hudgins.
- Pollack Table 2 is an image scan with no text layer; the numbers were read from the rendered page, not parsed
  from a data file, so the loader should re-verify against the paper when the composition is ingested.

## Sources

Draine 2003 (ApJ 598, 1017) and Draine & Lee 1984 (ApJ 285, 89), astro.princeton.edu/~draine/dust. Warren & Brandt
2008 (JGR 113, D14220), atmos.uw.edu/ice_optical_constants. Jaeger et al. 2003 DOCCD (JQSRT 79, 765) on SSHADE
(DOI 10.26302/SSHADE/DOCCD); Ordal et al. 1988 (Appl. Opt. 27, 1203); Begemann et al. 1994 (ApJ 423, L71);
Henning & Stognienko 1996 (A&A 311, 291); Jaeger et al. 1998 (A&A 332, 291); Zubko et al. 1996 (MNRAS 282, 1321).
Pollack et al. 1994 (ApJ 421, 615) Table 2. Birnstiel et al. 2018 (ApJL 869, L45), the DSHARP secondary.
