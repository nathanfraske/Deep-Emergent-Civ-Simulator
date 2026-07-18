# Bridge (post #197 merge): the channel re-opened, the Hayashi-wall handoff

This doc-only bridge re-holds Agent C's channel now that #197 (the mu-retirement viewer wire) has merged, and carries the Hayashi-wall fetch handoff to the coordinator for the ratified split. Off `origin/main` `f694edc`.

## What #197 landed

The mu-retirement viewer wire merged to main (`f694edc`): the viewer's disk clock reads the world's own drawn disk-gas mu (via `astro::derive_disk_gas_mean_molecular_weight`), and `DISK_MEAN_MOLECULAR_WEIGHT` is retired. Byte-neutral: both pins bit-exact (`40fe8a72` / `be94e310`), the Mirror `--derived-globe` byte-identical old-versus-new by sha256, CI green. The coordinator's mid-band anchoring viewer increment (the `province_column_params` call site) rebases onto it cleanly; my change touched only the disk-clock region.

## The Hayashi-wall handoff (the ratified split)

The gate ratified the Hayashi-wall fetch: both findings real (the 4000 -> 4397 K solar digit correction, and the mass-bias seam), and a SPLIT lane ruling. The coordinator lands the physics side (vendor BHAC15, the mass-indexed grid read, the registry row with its modality and conditioning fields, because the grid will gain consumers beyond the viewer). Agent C wires the viewer to consume it, once the grid read is on main, same shape as the mu getter fix (surface, physics lands, consume).

### The primary's text (reproducible fetch, for the coordinator's physics vendoring)

- **Citation:** Baraffe, Homeier, Allard, Chabrier 2015 (BHAC15), A&A 577 A42, DOI 10.1051/0004-6361/201425481.
- **Recipe:** `GET http://perso.ens-lyon.fr/isabelle.baraffe/BHAC15dir/BHAC15_tracks+structure` (1418659 bytes, plain text; columns M/Ms, log t(yr), Teff, L/Ls, g, R/Rs, ...). The raw file is 1.4 MB, so it is not pasted here; re-fetch from the recipe and verify against the receipt below (the fetch is reproducible byte-for-byte).
- **Receipt (sha256):** `b95474c5d4284373a2fed3f06d969a44bcd925ac0e5b226cc0235acb7e068d2a`.

### The extracted Hayashi-wall grid (the parse cross-check, and the read's data)

The wall T_eff per stellar mass (the top of the Hayashi track, earliest tabulated age ~0.49 Myr), with the age-drift band over the first 2 Myr of the descent. Monotonic in mass, so the read interpolates cleanly.

```
# columns: mass_msun  wall_teff_k  drift_lo_k  drift_hi_k   (BHAC15, solar composition [M/H]=0, theory grade)
 0.010    2388    2240    2388
 0.015    2514    2454    2516
 0.020    2594    2571    2604
 0.030    2666    2666    2710
 0.040    2731    2731    2784
 0.050    2761    2761    2829
 0.060    2807    2807    2867
 0.070    2834    2834    2902
 0.072    2831    2831    2907
 0.075    2835    2835    2913
 0.080    2836    2836    2924
 0.090    2861    2861    2933
 0.100    2925    2925    2971
 0.110    2945    2945    2995
 0.130    3006    3006    3034
 0.150    3076    3076    3082
 0.170    3139    3126    3143
 0.200    3220    3203    3226
 0.300    3460    3426    3460
 0.400    3672    3608    3672
 0.500    3849    3764    3849
 0.600    3988    3907    3988
 0.700    4111    4035    4111
 0.800    4221    4156    4221
 0.900    4315    4260    4315
 1.000    4397    4350    4397
 1.100    4471    4432    4471
 1.200    4537    4508    4538
 1.300    4596    4581    4601
 1.400    4647    4647    4659
```

### The five steers, folded (build order)

1. **Interpolate, never snap.** The read interpolates between mass rows in the table's own spacing; a snap would quantize every star's wall to BHAC15's mass sampling, the condensation-grid defect recurring one arc over.
2. **The wall is a chord over age.** The read declares its epoch convention (top of the Hayashi track, the earliest tabulated age) and carries the drift band (the `drift_lo`/`drift_hi` columns), never a bare number whose age nobody can name later.
3. **Domain guards two-ended.** The grid refuses BY NAME below 0.010 and above 1.400 M_sun (the table's own range); the high-side refusal points at the planned radiative-branch dispatch, machinery planned rather than absent.
4. **Modality field.** The rows are model-derived track values, theory grade (like the Landin turnover anchor), not measurements, carried with the solar-composition conditioning and the Siess-class metallicity follow-on scoped. The wall is citable-with-receipt, not authorable: the value line's whole content.
5. **Atomic across the three pre-MS-L consumers (my side, load-bearing).** The correction is render-visible through the fourth power (`(4397/4000)^4` is a ~46% jump in pre-MS luminosity at solar), and pre-MS L now has three consumers, so my viewer wiring lands the digit-plus-grid change atomically across all three (the third-site precedent), one truth about every star's brightness, with the render delta measured and both pins diffed.

> DATED CORRECTION (2026-07-18, ratified by the gate against its own re-derivation): the ~46% above is WRONG, and both the gate and Agent C carried it before the wire's measurement convicted it. The `T_eff^4` counts one door of two: `pre_main_sequence_luminosity_lsun` contracts the radius as `R ~ T_eff^(-4/3)` at fixed formation age (from `R^3 ~ 1/(T_eff^4 t)`), so `L = (4 pi sigma T_eff^4)^(1/3) (G M^2 / 7 t)^(2/3) ~ T_eff^(4/3)`, not `T_eff^4`. The real lift is `(4397/4000)^(4/3)` = ~13% (about 3.8 to 4.3 L_sun at solar), the radius contraction cancelling most of the fourth power. The 46% was a partial derivative at FIXED R; the model's answer is the total derivative along its own contraction trajectory, where `T_eff` enters twice. The generalization this mints: a sensitivity claim declares its held-fixed set. "The lift is X%" is a chord over what was held constant, so every steer quoting a sensitivity names its conditioning (at fixed R, at fixed age, along the trajectory) or it has not stated a number yet.

## My part, and the ask

Once the coordinator's mass-indexed grid read is on main, I wire the viewer's `derive_pre_ms_bolometric_luminosity` (and the other two pre-MS-L consumers, atomically) to call it with the star's own mass, retiring `HAYASHI_WALL_T_EFF_K`, measuring the globe delta and diffing both pins. Live on this channel; flag me when the grid read lands and I wire the consumption, ending SIGNED OFF.
