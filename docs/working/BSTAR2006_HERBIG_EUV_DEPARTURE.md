# BSTAR2006 Herbig-regime EUV departure: verified derivation, vendoring pending

This working note holds a load-bearing number derived from real data in an ephemeral
scratchpad, preserved here so it is not lost before the vendoring can be finished. It is NOT
yet wired into the code and NOT yet a gate-passing source entry: the archived witness step is
blocked in the session that produced it (see "Vendoring status"). Read this before wiring the
number or completing the vendoring.

## What this is

The `astro::nlte_departed_ionizing_spectrum` branch applies a photon-space departure band
`[departure_lo, departure_hi]` to a blackbody ionizing spectrum, where the departure is
`(real model atmosphere H-ionizing photon rate) / (same-Teff blackbody H-ionizing photon rate)`.
Above 25 kK the anchor is Sternberg, Hoffmann and Pauldrach 2003 (WM-Basic, with winds). Below
25 kK, the cool-B / Herbig-Be regime, it was UNCONSTRAINED. This note derives the departure
across 15,000 to 30,000 K from the BSTAR2006 emergent model SEDs (Lanz and Hubeny 2007), the
windless NLTE B-star grid whose coverage reaches down into the Herbig regime.

## Source (live, egress-allowed)

The SVO Theoretical Spectra service serves the BSTAR2006 emergent SEDs live. The VOTable
metadata confirms the identity: `DataID.Collection = "Tlusty BSTAR2006"`,
`DataID.Creator = 2007ApJS..169...83L`, bibcodes `1995ApJ...439..875H` and `2007ApJS..169...83L`.
Each SED is `F_lambda` in `ERG/CM2/S/A` versus `lambda` in `ANGSTROM`, 19998 points spanning
44.9 A to 3.0e6 A, so the Lyman continuum below 911 A is resolved (about 5420 points there).

- SSAP index (981 models, teff/logg/z plus a per-model download URL):
  `http://svo2.cab.inta-csic.es/theory/newov2/ssap.php?model=tlusty_bstarbin`
- Per-model SED: same URL with `&fid=<FID>`.
- Grid: Teff 15000 to 30000 K (1000 K step), log g 1.75 to 4.75 (0.25 step), z in {0, 0.1, 0.2, 0.5, 1.0, 2.0} solar.

The 16 models used, all at solar Z (z = 1.0) and log g = 4.0, held FIXED across the run (the
joint-fit-parameters-belong-together rule), each with its SVO FID, byte count, and sha256:

```
Teff   FID  bytes    sha256
15000  59   1195523  0706835e66101b83f946f69692ae49ec5ca4993a4e1951d5751d4b3569dfc14a
16000  131  1194944  a2112e18e8f17f78e88f914ee9326c4d35633f2903dc7aad47996be1c958944f
17000  203  1194483  bbb04090269dd291b9060d362d273968d66ec3a4f813b5186cb61855943933f7
18000  275  1193899  4d0d2e031554b42158553429381e97b9818582ec69977b483b2fd5165ed6e46b
19000  341  1193562  779ab874031a073c0efbef362d65226e2c869411a4f01882fa5609234e37d16a
20000  407  1193134  a689cbee5916fe3e994ac63aaa616818d513ca2037b705b36a35a42626586c97
21000  470  1192748  e341b5c63482b49e59540454cd69be9ef7c346196e24a0f85d712c5d53eea8b6
22000  530  1192277  3f32f7fbca2d65567cd2cca4387ca50bc668eaa699bf35521103d07002b60ae8
23000  590  1191916  9294d5c421a1d44c14f62cf57cca87df91e4fca75e1c2b1b6fac78e7b7e66224
24000  650  1191611  c5b24e664bf9a628c411c8cc8418f972bf410ba647f4eb535f95f27fd30e111d
25000  704  1191233  e7112b9e76d844f157753f735afb579cd62c3dbb09c985b9a645a0e4abca194c
26000  758  1191010  a190128568b264ec23f033d27a76a9f3355253551aa4685b10900679caab725b
27000  812  1190729  c5bb7f92d251292a3a31db4fb4995c5b5024128e6acc204b7cac4b0fd0055ec3
28000  866  1190376  96621f73d8b897ffefe7de573d5719eb89af9164d643134416e81c759ac596ed
29000  914  1189941  050fb914b51af380f13c3757cc0a888315ed432e639032053934a47b64be94b5
30000  962  1189596  8dfb4515e0a8dc538ae53e29787ec7af175051f8b1ea1c832e7c67804b0dfe64
```

## Method

nu_L = 3.2898e15 Hz (lambda_L = 911.28 A), the same threshold the P0-A code uses.

- Model ionizing photon rate: `N_model = integral over lambda < lambda_L of F_lambda * lambda / (h c) d_lambda`, with lambda in cm (lambda_A times 1e-8). Units: photons cm^-2 s^-1, surface.
- Blackbody reference: `N_bb = pi * integral over nu > nu_L of B_nu(Teff) / (h nu) d_nu`, the physical surface flux (F_nu = pi B_nu).
- departure(Teff) = N_model / N_bb.
- Flux-convention check, done from the data itself (not assumed): `integral F_lambda d_lambda / (sigma Teff^4)` came out 0.9994 to 1.0002 for all 16 models, so SVO serves the physical surface flux and the matching reference is pi B_nu. This is convention-consistent with the code's (pi/4) B_nu Eddington form: model-Eddington over (pi/4) B_nu equals model-physical over pi B_nu, the identical dimensionless ratio.

## The verified departure table (solar Z, log g = 4.0)

```
Teff    log10(departure)
15000   -2.689
16000   -2.636
17000   -2.567
18000   -2.484
19000   -2.388
20000   -2.282
21000   -2.166
22000   -2.040
23000   -1.899
24000   -1.735
25000   -1.549
26000   -1.361
27000   -1.182
28000   -1.005
29000   -0.825
30000   -0.654
```

- Full 15 to 30 kK: log10 departure in [-2.689, -0.654] (departure in [2.05e-3, 0.222]).
- Cool 15 to 25 kK (the region below the Sternberg anchor): log10 departure in [-2.689, -1.549] (departure in [2.05e-3, 2.83e-2]).
- Monotonic and nearly linear in Teff: slope about +0.140 dex per 1000 K. The mapping spans two decades, so `[departure_lo, departure_hi]` is NOT a flat clamp: departure(Teff) should be interpolated in log space across this table, not pinned to a single endpoint.

## Independent verification and the caught error

The delegated fetch agent reported this same spectral SHAPE (same slope, monotonic) but a band
offset by a constant 10^8: its log10 values were each about 8.00 below the table above (its
15 kK was -10.688, its 30 kK -8.654). An independent recomputation here, with its own parser and
Planck integral, reproduced the agent's flux-convention factors and its N_bb exactly but gave an
N_model larger by 10^8, a unit error in the agent's photon integral (the Angstrom-to-cm
conversion in the photon energy). Three cross-checks fix the correct normalization:

1. A single-bin hand check at lambda = 690 A: photon energy 17.97 eV (12398/690 = 17.97, correct), giving 7.96e16 photons cm^-2 s^-1 A^-1 in that bin, consistent with the table's N_model.
2. A physical anchor: the table's N_model at 30000 K is 7.9e22 photons cm^-2 s^-1; Sternberg's B0V (log Q = 47.8, R about 7 R_sun) implies a surface Lyman-continuum photon flux of 2.1e23, the same order (the windless BSTAR2006 sitting a little lower than the windy WM-Basic value is expected).
3. The departure magnitude itself: 0.2 percent to 22 percent of the blackbody Lyman continuum across the range is physical for B-star Lyman jumps; the agent's 10^-11 to 10^-9 would mean essentially no ionizing photons at all, which even a 15 kK atmosphere does not reach.

The lesson stands recorded: a confident "everything is verified" conclusion still needed an
independent recomputation and a physical anchor to be trusted. Had the agent's band been wired
in, the Herbig EUV photoevaporation would have been suppressed by 10^8, a qualitative error.

## Honest limits

- WINDLESS. These are plane-parallel hydrostatic NLTE atmospheres with no wind. The far-Wien Lyman continuum is exactly where winds matter, which is why Sternberg (WM-Basic, windy) is the >25 kK anchor. So BSTAR2006 above 25 kK will NOT match the Sternberg value, and the two must NOT be stitched into one continuous interval: they are sibling grounded intervals for different regimes (windless cool-B versus windy hot-B), the same disjoint-evidence discipline P0-B enforces on the EUV fit domain. The value of this grid is the 15 to 25 kK region, where no windy grid reaches and where Herbig Be winds are weak, so the windless model is the appropriate one.
- Fixed (log g = 4.0, solar Z). The band is specific to these. Herbig Be stars sit near log g 3.5 to 4.5; a log g or sub-solar Z sensitivity run is a separate fixed-other-params run, and every FID is in the SVO SSAP index above.
- Threshold: lambda_L = 911.28 A was used (nu_L = 3.2898e15 Hz, the code's value). The textbook 13.6057 eV limit is 911.75 A, a 0.05 percent difference in the threshold, negligible against the two-decade span.

## Vendoring status: PENDING the archived witness

The bytes are sha256-pinned above and the source is live at SVO, but the vendoring is NOT
finished and NO gate-passing source entry exists yet:

- The `witness` custody the sources_gate requires needs a RESOLVING archive URL. `web.archive.org` SAVE and CONTENT are egress-blocked in this environment (403 "Blocked by egress policy"), so a byte-identical Wayback witness cannot be created or verified here.
- The SVO files are 1.19 MB each and carry no explicit redistribution licence (the service expects "acknowledge the SVO Theoretical Spectra service and cite the original paper"), so the bytes are not held in-repo either.

To finish, in an environment where `web.archive.org` is reachable: archive the SSAP index and the
per-model URLs, record the archive URL and the licence reason, and add the `witness` entry to
`sources/registry.toml` keyed to the sha256s above. Then the departure table can be wired into
the P0-A Herbig branch as its own windless grounded interval (log-space interpolation over the
table), sibling to the Sternberg windy anchor, never merged with it.
