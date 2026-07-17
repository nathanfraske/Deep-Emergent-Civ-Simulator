# Disk-evolution arc: slice-1-closure + slice-2 audit dossier

This dossier lays out, in full and for review, every mechanism built after the last audited checkpoint (the EUV sibling, commit `4d761e0`). It is the batch the owner asked to have audited in one pass. Nothing here has a run-path caller: all of it is dormant, and both byte pins held bit-exact on every commit (default `40fe8a7269ee4da8974eb1787338c3a0`, living `be94e3100b9db82f7c1aea1d8091956d`).

## 0. What is under audit, and how to reproduce the gates

Seven commits on `claude/disk-evolution-arc`, `4d761e0..3b5e918`, all additive:

```
d205e19  the formation-epoch root and the gravitational radius (dormant)
986ad18  the tau_disk race as a closed-form crossing (dormant)
fbf730a  the X-ray photoevaporation wind rate as a declared band (dormant)
4fd12f4  scope-doc build log for the slice-1 closure root and slice-2 race
b8209fa  handoff entry
3a6dfa5  the absolute L_X fold, retiring the wind rate's L_X interim (dormant)
3b5e918  scope-doc build log for the absolute L_X fold
```

Diffstat against the last-audited head: `crates/sim/src/astro.rs` +667, the scope doc +19, `HANDOFFS.md` +12. Five new functions and one struct, all in `astro.rs`.

Gates, all green at head `3b5e918`: `cargo fmt --all --check`; `cargo clippy -p civsim-sim --all-targets -- -D warnings`; `python3 scripts/constructor_gate.py` (stone 0); `python3 scripts/determinism_gate.py`; prose customs (0 em dashes, 0 banned adverbs); the full sim lib suite (1236 passed, 0 failed); both byte pins via `cargo build --release --example run_world -p civsim-sim` on the default and `--scenario living` runs.

Every function carries three test kinds: a twin-independent oracle (the expected value computed OUTSIDE the code under test, in an f64 hand-computation, so the test is not the engine grading its own homework), a fail-loud test (non-physical inputs return `None`, never a plausible-looking number), and a mutation check (the code was broken on purpose once and the oracle test was confirmed to catch it).

## 1. The value-line ledger (the first thing to audit)

The value-authoring line is absolute: a physical number may be authored only in the physics floor. Across all five mechanisms the ledger is:

- **Bare algebra only, authored inline (allowed):** the LBP exponent pieces `5/2` and `2` (Lynden-Bell-Pringle self-similar solution); the `1/p` inversion; the decade constants `6` (year to Myr) and `7` (watt to erg/s); the halving `2` in the bisection; `31` and `2` in the representation-floor ceiling `31 * ln 2` (the Q32.32 integer ceiling `2^31`, an engine bound, not a physical one). None of these is a physical measurement.
- **Floor constants, read not authored:** `G`, `k_B`, `N_A` (from `civsim_units::fundamentals`), `SOLAR_MASS_KG`, `SOLAR_LUMINOSITY_W`, `ASTRONOMICAL_UNIT_M`, `m_H` as `1e-3 / N_A` (the molar-mass-of-hydrogen decade over Avogadro).
- **Reserved-with-basis, carried as DATA (never inline in the run path):** only the `XrayWindFit` coefficients (the Owen 2012 photoevaporation fit) and the condensation front `~1400 K` (the latter banked already, consumed here, not authored here). The wind-fit coefficients live in a struct the caller fills, exactly like `ConvectiveTurnoverFit`; the run path never hardcodes them. The values and their bases are in section 5.
- **Draw-pending interims (surfaced, not fabricated):** `Mdot_0` (solar interim, the layer-4 draw its destination), `R_1` (the disk birth size), `P_rot(age)` (the gyrochronology spin-down, the last interim in the L_X chain). Each is a caller-supplied input, tagged, with its destination named.

Four of the five mechanisms introduce ZERO reserved values of their own. The wind rate is the only one that carries reserved coefficients, and it carries them as a declared model band (section 5).

## 2. Mechanism: the formation-epoch root (slice-1 closure item 1)

`derive_formation_epoch_myr` derives `t_formation` as the root of `T_mid(1 AU, t) = T_condensation`. The formation-era midplane temperature rises with the accretion rate; the clock's `Mdot(t)` declines with age; so the midplane cools monotonically and crosses the condensation front exactly once. This bisects for that crossing. It is the referee that retires the 0.19 formation-rate landmark the owner pulled from the validation set: a hindcast on `Mdot(t_formation) = 0.19` refereed a degenerate product (rate times dust column times opacity), but this convicts `Mdot` because the dust column and opacity inside the temperature map are now derived, so the front fixes a temperature.

```rust
#[allow(clippy::too_many_arguments)]
pub fn derive_formation_epoch_myr(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    decline_gamma: Fixed,
    condensation_temperature_k: Fixed,
    midplane_temp_at_rate: impl Fn(Fixed) -> Option<Fixed>,
    t_lo_myr: Fixed,
    t_hi_myr: Fixed,
    iterations: u32,
) -> Option<Fixed> {
    if t_lo_myr < Fixed::ZERO || t_hi_myr <= t_lo_myr || condensation_temperature_k <= Fixed::ZERO {
        return None;
    }
    let temp_at = |age: Fixed| -> Option<Fixed> {
        let rate =
            viscous_similarity_accretion_rate(mdot_0_msun_myr, t_visc_myr, decline_gamma, age)?;
        midplane_temp_at_rate(rate)
    };
    // Temperature declines with age, so the bracket must straddle: T(t_lo) >= T_cond >= T(t_hi).
    if temp_at(t_lo_myr)? < condensation_temperature_k
        || temp_at(t_hi_myr)? > condensation_temperature_k
    {
        return None;
    }
    let mut lo = t_lo_myr;
    let mut hi = t_hi_myr;
    let two = Fixed::from_int(2);
    for _ in 0..iterations {
        let mid = lo.checked_add(hi)?.checked_div(two)?;
        // Still too hot at the midpoint: the crossing is at a later (larger) age.
        if temp_at(mid)? > condensation_temperature_k {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.checked_add(hi)?.checked_div(two)
}
```

**Design decisions.** The disk state (the many `formation_midplane_temperature` arguments) is kept out of the signature by passing a `midplane_temp_at_rate` closure: the caller composes the temperature map with its fixed disk parameters. Determinism holds by construction: a fixed iteration count, no unbounded loop, all fixed-point. The straddle guard refuses a bracket that does not cross the front (temperature at `t_lo` below it or at `t_hi` above it), so a non-crossing case returns `None` rather than an extrapolated root.

**Tests.** `the_formation_epoch_root_reproduces_the_condensation_front` uses a monotone stub map `T = 2000 * rate^(1/4)` and asserts the found `t_formation`, fed back through the same clock and map, reproduces 1400 K to within a kelvin (48-iteration bisection). `the_formation_epoch_refuses_a_non_straddling_bracket` asserts `None` when the front is never reached. The referee property (feed the root back, get the front) is the twin-independence: the test does not read the bisection's internal state, it checks the defining equation.

**Honest limits.** The referee is only as good as the temperature map it is handed; the map's own derivation (the dust column and opacity) is upstream. It finds A crossing, and monotonicity (which the clock and the rising-with-rate map together guarantee) is what makes it THE crossing.

## 3. Mechanism: the gravitational radius (slice-2 gap radius)

`gravitational_radius_au` derives `r_g = G M_star mu m_H / (k_B T_wind)`, the radius beyond which the wind's thermal energy exceeds gravitational binding. The gap radius where the wind first opens a gap is `r_g` times a wind-physics prefactor (~0.1 to 0.2), the banded class constant the caller supplies, so this returns `r_g` and the caller scales it. Log-domain, the surface-density precedent.

```rust
pub fn gravitational_radius_au(
    star_mass_ratio: Fixed,
    wind_temperature_k: Fixed,
    mean_molecular_weight: Fixed,
) -> Option<Fixed> {
    if star_mass_ratio <= Fixed::ZERO
        || wind_temperature_k <= Fixed::ZERO
        || mean_molecular_weight <= Fixed::ZERO
    {
        return None;
    }
    let ln_g = civsim_physics::saha::ln_of_decimal(
        civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
    )?;
    let ln_m_star = star_mass_ratio
        .ln()
        .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
    let ln_m_h = civsim_physics::saha::ln_of_decimal("1e-3")?.checked_sub(
        civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::AVOGADRO.value)?,
    )?;
    let ln_k_b = civsim_physics::saha::ln_of_decimal(civsim_units::fundamentals::BOLTZMANN.value)?;
    let ln_au = civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?;
    // ln r_g[AU] = ln G + ln M_star + ln mu + ln m_H - ln k_B - ln T - ln AU.
    let ln_rg = ln_g
        .checked_add(ln_m_star)?
        .checked_add(mean_molecular_weight.ln())?
        .checked_add(ln_m_h)?
        .checked_sub(ln_k_b)?
        .checked_sub(wind_temperature_k.ln())?
        .checked_sub(ln_au)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rg >= ln_ceiling {
        return None;
    }
    Some(ln_rg.exp())
}
```

**Admit-the-alien note.** The wind temperature is a caller-supplied banded class value (the EUV-heated ~1e4 K wind or the harder X-ray-heated wind, per the band the giant arc flagged), and the mean molecular weight of the launched gas is an input, so a non-hydrogen wind is a data row, not a rewrite.

**Tests.** `the_gravitational_radius_matches_the_solar_euv_wind_oracle` (M=1, T=1e4, mu=1 gives r_g ~ 10.673 AU, hand-computed). `the_gravitational_radius_scales_inverse_temperature_and_linear_mass` (a ten-times-colder wind gives a ten-times-larger r_g; half the mass halves it, each checked against the base case, not a second hand-number). `the_gravitational_radius_refuses_nonphysical_inputs` (fail-loud on each non-positive axis). Mutation: the `k_B` term was flipped from subtract to add and the oracle test caught it.

**Honest limits.** `r_g` is the isothermal gravitational radius; the true gap-opening radius is a fraction of it, and that prefactor is a banded wind-physics constant the caller owns, not settled here.

## 4. Mechanism: the tau_disk race (the arc's named output)

`derive_disk_lifetime_myr` derives `tau_disk`, the age at which the wind-versus-accretion race tips. When the declining accretion rate falls to the wind rate, the wind opens a gap and clears the disk on the much shorter local viscous time. Because the LBP decline is a monotone power law crossing a CONSTANT wind rate, the crossing `Mdot(t) = Mdot_wind` inverts in closed form, no root-finder: `tau_disk = t_visc * ((Mdot_0/Mdot_wind)^(1/p) - 1)`.

```rust
pub fn derive_disk_lifetime_myr(
    mdot_0_msun_myr: Fixed,
    t_visc_myr: Fixed,
    decline_gamma: Fixed,
    wind_rate_msun_myr: Fixed,
) -> Option<Fixed> {
    if mdot_0_msun_myr <= Fixed::ZERO
        || t_visc_myr <= Fixed::ZERO
        || wind_rate_msun_myr <= Fixed::ZERO
        || decline_gamma < Fixed::ZERO
        || decline_gamma >= Fixed::from_int(2)
    {
        return None;
    }
    // The wind already meets or beats peak accretion: the gap opens at (or before) birth, so no viscous era.
    if wind_rate_msun_myr >= mdot_0_msun_myr {
        return Some(Fixed::ZERO);
    }
    // 1/p = (2 - gamma) / (5/2 - gamma); at gamma = 1 this is 2/3 (p = 3/2).
    let two = Fixed::from_int(2);
    let inv_p = two
        .checked_sub(decline_gamma)?
        .checked_div(Fixed::from_ratio(5, 2).checked_sub(decline_gamma)?)?;
    // factor = (Mdot_0 / Mdot_wind)^(1/p), computed in the log domain (the ratio exceeds 1 here, so ln > 0).
    let ln_ratio = mdot_0_msun_myr.ln().checked_sub(wind_rate_msun_myr.ln())?;
    let ln_factor = inv_p.checked_mul(ln_ratio)?;
    // A REPRESENTATION-FLOOR guard (the clock precedent): a lifetime past the exp ceiling exceeds what the format
    // can hold, so surface it as unrepresentable rather than a saturated value. Unreachable for physical ratios.
    let ln_ceiling = Fixed::from_int(31).checked_mul(two.ln())?;
    if ln_factor >= ln_ceiling {
        return None;
    }
    // tau_disk = t_visc * (factor - 1).
    t_visc_myr.checked_mul(ln_factor.exp().checked_sub(Fixed::ONE)?)
}
```

**Replacement-circularity compliance.** This retires the reserved `disk_gas_lifetime_myr` the #73 giant gate races the Kelvin-Helmholtz time against. Per the standing rule, it does NOT calibrate against the value it replaces: it validates against the Haisch-Lada / Mamajek disk-fraction-versus-age band as an OUTPUT, and never as a median (the range is the statistic-with-hidden-conditioning class this arc has convicted before).

**TERMS-DROPPED (the honest limits, in the code).** The wind rate is held CONSTANT across the crossing; the age-evolution of `L_X` during the race is omitted, valid because the disk clears within the few-Myr class-II window where `L_X` sits in the saturated-activity plateau and varies slowly. External (birth-environment) photoevaporation is omitted, its validity domain the isolated star-forming environment, the dense-cluster term named for the environment hook. The `Fixed::ZERO` return (immediate dispersal) is the honest answer when the wind already beats peak accretion, not an error.

**Tests.** `the_disk_lifetime_inverts_the_race_to_a_clean_oracle` (two clean-integer crossings: gamma=1, Mdot_0=8, wind=1, t_visc=1 gives 8^(2/3)-1 = 3; and 27,2,1,1 gives 16). `the_disk_lifetime_is_the_rate_crossing` (the deeper invariant: feed tau back through the accretion clock, an INDEPENDENT function, and it reproduces the wind rate, so the closed form and the clock agree on where the race tips). `the_disk_lifetime_is_zero_when_the_wind_beats_peak_accretion`. `the_disk_lifetime_refuses_nonphysical_inputs`. Mutation: the `- 1` term was dropped and both the oracle and the crossing test caught it.

## 5. Mechanism: the wind rate and its declared model band

`photoevaporative_wind_rate_msun_myr` derives the X-ray-driven wind mass-loss rate the race consumes, `Mdot_w = C (M/M_sun)^a (L_X/L_X_ref)^b`, from an `XrayWindFit` the caller supplies. Log-domain end to end, because both `L_X ~ 1e30 erg/s` and the `~6e-9` coefficient sit outside the Q32.32 range; `L_X` is passed as `log10(L_X in erg/s)`, never a raw value. The mechanism is fixed Rust; the coefficients are data (the `ConvectiveTurnoverFit` precedent).

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct XrayWindFit {
    pub log10_coefficient_msun_yr: Fixed,
    pub log10_l_x_reference_erg_s: Fixed,
    pub l_x_exponent: Fixed,
    pub mass_exponent: Fixed,
    pub mass_min_msun: Fixed,
    pub mass_max_msun: Fixed,
}

pub fn photoevaporative_wind_rate_msun_myr(
    log10_l_x_erg_s: Fixed,
    star_mass_ratio: Fixed,
    fit: &XrayWindFit,
) -> Option<Fixed> {
    if star_mass_ratio <= Fixed::ZERO
        || star_mass_ratio < fit.mass_min_msun
        || star_mass_ratio > fit.mass_max_msun
    {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10_m = star_mass_ratio.ln().checked_div(ln10)?;
    // log10(Mdot in M_sun/yr) = log10(C) + a*log10(M_star) + b*(log10(L_X) - log10(L_X_ref)).
    let log10_l_x_term = fit
        .l_x_exponent
        .checked_mul(log10_l_x_erg_s.checked_sub(fit.log10_l_x_reference_erg_s)?)?;
    let log10_rate_yr = fit
        .log10_coefficient_msun_yr
        .checked_add(fit.mass_exponent.checked_mul(log10_m)?)?
        .checked_add(log10_l_x_term)?;
    // Convert per-year to per-Myr: add log10(1e6) = 6.
    let log10_rate_myr = log10_rate_yr.checked_add(Fixed::from_int(6))?;
    let ln_rate = log10_rate_myr.checked_mul(ln10)?;
    let ln_ceiling = Fixed::from_int(31).checked_mul(Fixed::from_int(2).ln())?;
    if ln_rate >= ln_ceiling {
        return None;
    }
    Some(ln_rate.exp())
}
```

**The anomaly, surfaced (an input-audit catch on my own recollection).** I went to author the fractional-exponent form from memory and checked it against Owen, Clarke and Ercolano 2012 first. The paper's equation 9 is a strictly LINEAR, mass-independent analytic estimate, `8e-9 (L_X/1e30)`, not the fractional form. The fractional form is real but lives in the same paper's appendix B (the population-synthesis fit), cross-confirmed by attribution; the ar5iv rendering truncated before appendix B, so the exact appendix-B text is reserved for confirmation against the primary at manifest-setting time. Rather than silently pick a form, the fit ships as a DECLARED MODEL BAND, the same treatment the arc gives the alpha-viscous-versus-MHD-wind transport dispute. Three cited rows on one mechanism:

1. **Owen 2012 appendix B (near-linear):** `Mdot_w = 6.25e-9 (M/M_sun)^-0.068 (L_X/1e30)^1.14 M_sun/yr`. So `log10 C = log10(6.25e-9) = -8.204`, `L_X ref = 1e30` (log10 = 30), `b = 1.14`, `a = -0.068`, sample low-mass stars (upper ~1.5 M_sun). Basis: the widely-used primordial-disc fit.
2. **Owen 2012 equation 9 (strictly linear):** `Mdot_w = 8e-9 (L_X/1e30)`, so `b = 1`, `a = 0`. Basis: the paper's own analytic order-of-magnitude form.
3. **Sellek et al. 2024 PLUTO+PRIZMO:** integrated rates roughly an order of magnitude LOWER from enhanced molecular cooling, a lower coefficient on the same shape. Basis: a live radiation-hydro rival.

The band membership (which row, or all three as an ensemble) is the owner's call. This is the only mechanism in the batch with reserved coefficients, and they are data in a struct, never inline in a run path.

**Domain guard.** The fit is measured over low-mass stars, so a stellar mass outside `[mass_min, mass_max]` returns `None` rather than extrapolate the wind physics into the intermediate-mass regime. This is admit-the-alien biting in the unfamiliar direction, the same guard the convective-turnover fit carries.

**Tests.** `the_wind_rate_matches_the_owen_solar_oracle` (solar analogue at log10 L_X = 30 gives 6.25e-3 M_sun/Myr; the half-solar weak-mass factor). `the_wind_rate_scales_near_linearly_with_luminosity` (a decade brighter raises the rate by 10^1.14 ~ 13.8). `the_wind_rate_guards_the_mass_domain_and_refuses_nonphysical_inputs`. `the_wind_rate_feeds_the_dispersal_race` (end-to-end into `tau_disk`). Mutation: the `L_X` term sign was flipped and the scaling test caught it (the oracle test alone did not, because at the reference luminosity the term is zero; the two tests are complementary by design).

## 6. Mechanism: the absolute L_X fold (retiring the wind rate's L_X interim)

`stellar_xray_luminosity_log10_erg_s` folds two dimensionless ratios the star already carries into an absolute `log10(L_X in erg/s)`: the bolometric ratio `L_bol/L_sun` and the activity fraction `L_X/L_bol`, through the solar luminosity, `L_X = (L_bol/L_sun) * L_sun * (L_X/L_bol)`. Returned as a log10 because `L_X ~ 1e30` overflows the format and because the wind rate consumes exactly this log10, so the two compose without forming the raw value.

```rust
pub fn stellar_xray_luminosity_log10_erg_s(
    bolometric_ratio: Fixed,
    activity_fraction: Fixed,
) -> Option<Fixed> {
    if bolometric_ratio <= Fixed::ZERO || activity_fraction <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    // log10(L_sun in erg/s) = log10(L_sun in W) + 7 (the watt-to-erg/s decade).
    let log10_l_sun_erg_s = civsim_physics::saha::ln_of_decimal(SOLAR_LUMINOSITY_W)?
        .checked_div(ln10)?
        .checked_add(Fixed::from_int(7))?;
    let log10_bol = bolometric_ratio.ln().checked_div(ln10)?;
    let log10_fraction = activity_fraction.ln().checked_div(ln10)?;
    // log10(L_X) = log10(L_bol/L_sun) + log10(L_sun) + log10(L_X/L_bol); it is a log, so it stays in range.
    log10_bol
        .checked_add(log10_l_sun_erg_s)?
        .checked_add(log10_fraction)
}
```

**What it closes.** This is the destination the coordinator's L_X-first ruling named. It retires the `L_bol`-times-fraction step of the wind rate's interim; the LAST remaining interim in the whole chain is the Rossby number's rotation-period input `P_rot(age)`, through the gyrochronology spin-down, which stays draw-pending (`Omega_star_0` is a layer-4 spec, not built, the same status as `Mdot_0`). Zero new values.

**Tests.** `the_absolute_xray_luminosity_folds_to_the_solar_oracle` (saturated young sun, L_bol=1, fraction=1e-3, gives log10 L_X = 30.583 = log10(L_sun in erg/s) - 3; and the full-bolometric fold reproduces log10(L_sun in erg/s) = 33.583). `the_absolute_xray_luminosity_is_a_decade_per_decade` (each ratio enters as a log10, so a decade in either adds one). `the_absolute_xray_luminosity_closes_the_chain_into_the_wind_rate` (end-to-end into the Owen rate). `the_absolute_xray_luminosity_refuses_nonphysical_ratios`. Mutation: the watt-to-erg/s `+7` decade was dropped and the oracle caught it.

## 7. The chain, composed

The five mechanisms plus the already-audited L_X activity slice form one derivation from stellar mass to disk lifetime:

```
M_star -> tau_conv(M) --.
                        Ro = P_rot/tau_conv -> L_X/L_bol -> [x L_bol] -> log10(L_X)
P_rot(age) -------------'                                                    |
                                                                            v
                                                          Owen fit -> Mdot_wind
                                                                            |
Mdot_0, t_visc, gamma -> Mdot(t) [the clock] ------------------------------ race -> tau_disk
                              |                                                         |
                              '-> T_mid(1 AU, t) -> [root] -> t_formation      -> #73 giant gate (slice 3, HELD)
```

Everything left of `tau_disk` is built and dormant. The two interims still in the chain are `P_rot(age)` (last L_X input) and `Mdot_0` / `R_1` (clock inputs), each surfaced with its destination. Slice 3, the only place `tau_disk` reaches the #73 gate and becomes a run-path behaviour change, is explicitly held.

## 8. Standing-panel-lens self-check (the five mandatory lenses)

This is my own pass; it does not substitute for the blind panel, it orients it.

- **Derive-versus-author:** four of five mechanisms author zero values; the fifth (wind rate) carries its reserved coefficients as struct data with a declared three-row band, never inline. The one thing to probe hardest is whether the `XrayWindFit` band is truly a band or a disguised default; my read is that the three rows are distinct physics (linear vs fractional vs cooled) and the caller must choose, but that is exactly the seam the panel should attack.
- **Alien-feasibility:** the gravitational radius keys on a caller-supplied wind temperature and mean molecular weight, so a non-hydrogen wind is a data row; the wind-rate and turnover fits carry both-ends domain guards that return `None` outside their measured range rather than extrapolate. No pathway assumes one chemistry.
- **Terran-bias:** the solar oracles in the tests are FIXTURES for twin-independence, not defaults baked into the run path; the functions key on per-star mass, luminosity, and rotation, so an M dwarf or a metal-poor star is a different input, not a special case.
- **Steering / Principle 9:** the model bands (the wind-rate three rows, and the alpha-viscous-vs-MHD transport dispute the clock names) are DECLARED physics inputs, which Principle 9 permits; no cultural or emergent outcome is authored. The replacement-circularity rule is honored (tau_disk validates against sources, never against the value it retires).
- **Confirmation-bias / blind correctness:** the un-audited code is in `astro.rs`; a blind correctness read should be built from the code alone (no tests, no this-dossier), which the panel skill's section-7 packet does. The specific claims to verify against source: the LBP exponent `p = (5/2 - gamma)/(2 - gamma)`; the gravitational-radius dimensional reduction; the Owen appendix-B coefficients (the one I could not transcribe verbatim); and the watt-to-erg/s and year-to-Myr decades.

## 9. Open items and the sequencing tension

**Ruled in the batch audit (owner).** Two items this dossier raised are now settled and folded into the code and the scope doc:

- **The wind-rate band membership:** all three rows ship as the DECLARED ENSEMBLE (appendix-B central pending verbatim confirmation, equation 9 the order-of-magnitude cross-check, Sellek 2024 the low edge), because they are distinct physics claims, the radiative-conductivity dispute pattern. The COST is made explicit and executable: the order-of-magnitude wind band propagates through the `(Mdot_0/Mdot_w)^(1/p)` inversion to a factor `10^(1/p) ~ 4.64` band on `tau_disk` at `gamma = 1`, proven by a new dormant test against a `10^(2/3)` oracle; the Haisch-Lada / Mamajek data is the independent within-band referee.
- **The sequencing tension:** adjudicated in the batch's favor on the merits. The coordinator's protections were the two substantive clauses (no interim baked into the wind rate, slice 3 held), never the ordering; both stand in what was delivered, and the ordering clause is discharged because the `L_X` it ordered first now exists. No fault; surfacing the departure was the required conduct.

**Still open for the gate:** the EUV `ro_sat`-sharing question from the L_X slice; slice 3 (wiring `tau_disk` into #73 and the DiskGas opening, the first run-path change, both pins re-pinned deliberately); the gyrochronology spin-down `P_rot(age)` that closes the last L_X interim (a value-line fetch plus a modeling choice, to be built parametric with a declared band); and slice-1 closure items 2 to 4 (the `t_mature` locus fetch, the `Mdot_0` draw, the `R_1` draw), design-only with tagged solar interims.
