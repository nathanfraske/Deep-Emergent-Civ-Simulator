# Disk-evolution slice 2: the run-path wire (tau_disk into the #73 giant gate), design of record

This is the design of record for the disk arc's FIRST run-path change, signed off by Nathan with six conditions (2026-07-18). It is written before any code, per condition 1 (design-first, built behind a flag, the wiring presented for audit before the flag flips). Nothing here is on a run path yet; this is the plan the focused build executes, and the artifact the audit reads before the flag flips.

## 1. What the wire closes

The composed disk clock `astro::disk_era_xray_disk_lifetime_myr` (`tau_disk`) is built, tested, and DORMANT: nothing in `run_world` calls it. The #73 giant-planet gate (`giants::giant_formation`) decides giant-hood in the log domain by `ln_tau_kh < ln(gas.disk_gas_lifetime_myr) + ln(1e6)` (giants.rs:446-451), and `disk_gas_lifetime_myr` is today the reserved `Fixed::from_int(3)` (~3 Myr), set at giants.rs:542 and planetary_assembly.rs:1244. This wire replaces that reserved scalar with the derived `tau_disk`, so the gas clock the giant gate reads is derived from the star's own X-ray-driven photoevaporation history rather than an authored mid-observed-disk-lifetime placeholder. It is the arc's thesis cashing out: the clock stops being a dormant proof and starts setting real disk lifetimes and thus real giant-versus-terrestrial verdicts.

It RE-PINS both canonical pins deliberately (`40fe8a72` default, `be94e310` living), the considered exception every dormant-and-byte-neutral slice to date was built to make possible.

## 2. The assembled call graph

`disk_era_xray_disk_lifetime_myr(mass_ratio, hayashi_temp_k, age_myr, rotation_period_days, mlt_coefficient, ro_sat, saturated_log10_fraction, beta, xray_fit, mdot_0_msun_myr, t_visc_myr, decline_gamma)`. Every input, with its source and grade:

- `mass_ratio`: the drawn star mass (per-world datum).
- `hayashi_temp_k`: the wall, DERIVED per-star from the BHAC15 grid `civsim_physics::hayashi_wall::HayashiWallGrid::standard()?.wall_teff(mass_ratio)?.wall_teff_k`. This is the rider-2 upgrade from #200 arriving: when disk_era_xray goes live, its wall reads the grid, and its integration test (below) consumes the real grid read (fixtures for arithmetic, grid for integration).
- `age_myr`: the disk-era evaluation age (the formation/disk-hosting epoch).
- `rotation_period_days`: disk-locked rotation (a tagged interim, the disk-locked ~8 d).
- `mlt_coefficient`, `ro_sat`, `saturated_log10_fraction`, `beta`: the activity and turnover fit constants (reserved-with-basis, banked).
- `xray_fit`: the wind row, ONE OF THREE (section 4, condition 3).
- `mdot_0_msun_myr`: the birth accretion rate, a TAGGED SOLAR INTERIM (the class-0/I band, `~1 M_sun/Myr`), its real retirement the layer-4 draw (condition 6, surfaced in the ledger).
- `t_visc_myr`: the viscous time, derived from `R_1` (the disk birth size, 30 AU solar pin), a TAGGED SOLAR INTERIM (condition 6).
- `decline_gamma`: the LBP decline index (`gamma = 1`, bare algebra).

## 3. Where the derived value lands, and the census (condition 4)

This wire RETIRES the reserved `disk_gas_lifetime_myr`. Per the standing retirement-census rule, every consumer is enumerated and dispositioned in the same commit:

- `giants.rs:95` (`GiantGasParams::disk_gas_lifetime_myr` field): the field is replaced by the derived band (section 4), or the struct carries the derived interval; dispositioned so `giant_formation` reads the derived clock.
- `giants.rs:448` (the log-domain read in `giant_formation`): reads the derived lifetime interval, not the scalar.
- `giants.rs:542` and `planetary_assembly.rs:1244` (the two `Fixed::from_int(3)` sites): retired; the reserved 3 stops being authored into the run path.
- Any other reader surfaced by `grep disk_gas_lifetime_myr` at build time gets its disposition recorded, so no second tenant keeps reading a lifetime the clock now derives.

## 4. The band flows, or the wire is wrong (condition 3)

`tau_disk` carries the DECLARED three-row wind ensemble (astro.rs:1948-1966), roughly `10^(1/p)` ~ 4.64x wide at `gamma = 1`. The three rows, each reserved-with-basis and cited, built as run-path data (they exist today only as one test helper):

1. Owen, Clarke, Ercolano 2012 Appendix-B fit (`6.25e-9 (M/M_sun)^-0.068 (L_X/1e30)^1.14`), the CENTRAL instance.
2. The same paper's Equation-9 analytic (`8e-9 (L_X/1e30)`, `l_x_exponent = 1`, `mass_exponent = 0`), the order-of-magnitude cross-check.
3. Sellek et al. 2024 (radiation-hydro revision), roughly an order of magnitude lower, the LOW EDGE.

The wire evaluates `disk_era_xray_disk_lifetime_myr` ONCE PER ROW, producing a THREE-VALUED `tau_disk` band `[tau_low, tau_central, tau_high]`. The #73 gate consumes the INTERVAL, never a silently chosen central row: `giant_formation`'s log-domain test becomes an interval test. A draw whose `ln_tau_kh` falls BELOW `ln(tau_low)` is Giant under every row; above `ln(tau_high)` is Terrestrial under every row; INSIDE the band gets a NEAR-DEGENERATE verdict (Giant under the high-lifetime row, Terrestrial under the low), carried as a banded outcome per the Gap Law, not collapsed. A point-valued wire would collapse a declared model band at the exact moment it first touches world content, the one failure mode that would make this slice dishonest. The `GiantVerdict` (or a wrapper) gains the band-membership state so the near-degeneracy is a carried datum, surfaced, not silently resolved.

## 5. The flag and the pre-flip machine conditions (conditions 1 and 2)

Built behind a feature flag (`disk_clock_live` or equivalent), dormant and byte-neutral while off, so the wiring lands and is auditable before it moves a pin. The wiring is presented for audit before the flag flips (condition 1).

Condition 2's saturation-margin assert is ALREADY in the tree and must be green before the flip: `astro::tests::the_pre_ms_turnover_saturates_the_disk_era_rossby` (astro.rs:5493), the wrong-Ro trace's machine closure. It asserts disk-era Ro sits below the knee (`ro_sat = 0.13`) by more than the turnover fit's own error band (a factor-2, 0.30 dex margin exceeding both the pre-MS coefficient band and the Wright RMS), evaluated with the pre-MS `tau_conv`, across the disk-era mass (0.3 to 1.36 M_sun) and rotation (8 to 15 d) range. Confirmed present; the flip gates on it staying green, not on a fresh commit.

## 6. Replacement-circularity and the first population hindcast (condition 5)

Replacement-circularity stays absolute through the flip: NOTHING in CI references the Haisch-Lada few-Myr range or the retired `disk_gas_lifetime_myr` value, and the disk clock's own CI tests stay mechanistic (byte-equality, monotonicity, units bracket, determinism pin), never keyed to the observed band. After the wire lands, the arc collects its reward: the derived `tau_disk` distribution across a draw ensemble is compared against the disk-fraction-versus-age data (Haisch-Lada, Mamajek) as the arc's FIRST population hindcast. That comparison is EXTERNAL, reported, band-aware (it discriminates WITHIN the 4.64x wind band, legal because it is independent data, never a value calibrated against), and NEVER a gate. It lives in the ensemble validator, out of CI's reach, the destination the retired value was quarantined for. The design specifies this harness as a reported validator, not a test. Per the tolerance three-lives spec, this is the moment the cited class-II dispersion graduates from an input-side referee to an output-side hindcast row.

## 7. The attribution and the signature, first of two (condition 6)

The measured delta is attributed in full: which draws' giant verdicts flipped (Giant to Terrestrial or the reverse, and which entered the near-degenerate band), the DiskGas openings that moved, and the downstream world content that changed. Both pins re-pin under Nathan's signature, with the ledger entry recording that the clock runs on TAGGED SOLAR INTERIMS for `Mdot_0` (the class-0/I birth accretion band) and `R_1` (the 30 AU disk birth size), so the pin is honest about what it rests on. The signature is EXPLICITLY the first of two: when the layer-4 draws for `Mdot_0`, `Omega_star_0`, and `R_1` land, the clock becomes per-world and the pins move again as a SECOND attributed event. Signing the topology change now and the per-world contingency later, and saying so in the ledger, is what keeps both honest.

## 8. Build and gate plan

The order, each step gated, the flag off until the last:

1. The three wind rows as cited run-path data (`XrayWindFit` constructors: `owen_appendix_b`, `owen_equation_9`, `sellek_2024`), each reserved-with-basis, verbatim-cited, with a fetch follow-on where a coefficient needs primary confirmation.
2. The band-carrying giant gate: `giant_formation` (or a wrapper) consumes the tau_disk interval and carries the near-degenerate verdict, with tests that a mid-band draw is flagged near-degenerate and an out-of-band draw is decided.
3. The wall-grid-read for the live disk_era_xray path, and its integration test consuming the grid (the rider-2 upgrade landing).
4. The wire itself behind the flag: the derived tau_disk band replacing the reserved `disk_gas_lifetime_myr`, dormant while the flag is off.
5. The census executed and recorded (section 3).
6. The population-hindcast harness (reported, out of CI).
7. Present the wiring for audit. On sign-off, flip the flag, measure and attribute the delta (section 7), re-pin both pins under Nathan's signature, record the ledger entry.

Then the full gate: fmt, clippy `-D warnings`, doc-link delta 0, the mirror by real exit code, the saturation assert green, and the two pins re-pinned (not held) with the diff attributed.
