# The `Fixed::powf` call-site audit

A complete, measured audit of every `powf` call site in the tree, run 2026-07-19 after a prior session
documented the rail on `Fixed::powf` and added `Fixed::checked_powf` without auditing the callers.

Every bound below is MEASURED against the real production inputs, not reasoned from a plausible range.
Where a site is called "safe", the number that makes it safe is in the table. Where a site rails, a test
in the tree demonstrates it railing and the fix.

---

## 1. The rail, measured

`Fixed::powf(x, y)` is `exp(y * ln x)`, so it inherits `Fixed::exp`'s window. Both edges were found by
bisection on the real function rather than read from a comment:

| edge | condition | result | error |
|---|---|---|---|
| upper | `y * ln(x) > 21.4875626` | `Fixed::MAX` (2.147e9) | UNBOUNDED |
| lower | `y * ln(x) < -22` | `Fixed::ZERO` | about one ulp |

The two are not the same kind of failure, and conflating them overstates the exposure. The upper rail's
error grows without limit: `(1e8)^2.5` returns `2.147e9` against a truth of `1e20`. The lower rail fires
only where the true value is already below `e^-22 = 2.79e-10`, and one ulp of Q32.32 is `2.33e-10`, so a
zeroed result was never more than 1.2 grid steps from zero. **The upper rail is a defect to guard; the
lower rail is the representable floor doing its job.** `checked_powf` refuses on both, which makes its
lower-rail refusal a conservative false positive rather than a caught defect.

A separate concern that neither rail catches: relative precision collapses well INSIDE the window. At
`y ln x = -18` the result is 65 ulp (1.5% quantum), at `-20` it is 8 ulp (12.5%). A site whose budget runs
past about `-16` is delivering a number with visible quantization even though nothing rails.

## 2. The exponent window, the bound that settles most sites

`ln` of a representable positive `Fixed` is bounded by the type itself: `ln(2^-32) = -22.1807098` at one
ulp and `ln(2^31) = 21.4875626` at `Fixed::MAX`. So `y * ln(x)` is bounded by the EXPONENT ALONE, and:

> An exponent inside `[-0.9687499, +0.9918528]` cannot reach either rail for ANY representable base.

The two edges differ because the rails are not symmetric: a negative exponent maps the most negative log
(the ulp) onto the UPPER rail, so the negative edge is the tighter one. Both edges were measured by
bisection over a spanning set of 66 bases (every power-of-two magnitude plus both extremes) and match the
prediction to seven decimal places; one step past either edge rails. The test is
`crates/core/src/fixed.rs`, `powf_rail_tests::the_exponent_window_cannot_rail_for_any_representable_base`,
which also pins each production exponent that relies on it.

This one fact classifies 27 of the 43 production sites with no argument about the base at all. Outside the
window, the caller owes a bound on the base, and the table gives it.

## 3. Inventory

162 occurrences of `powf` across `crates/`, classified mechanically (`#[cfg(test)]` brace-matched per
file, receiver type read from the expression):

| bucket | count |
|---|---|
| `Fixed::powf` production call sites | 43 |
| `Fixed::powf` test-only call sites | 21 |
| `f64`/`f32` `powf` (non-canon viewer, f64 test twins) | 41 |
| GPU CubeCL mirror (`crates/gpu`) | 3 |
| definitions, doc comments, re-exports | 54 |

The `f64` and `f32` sites carry no Q32.32 rail (IEEE 754 double spans about 600 decades) and all sit in
`crates/viewer` (the non-canon display layer) or in f64 test twins that exist precisely to check the fixed
form from outside. They are out of scope and listed only for completeness. The GPU sites are the CubeCL
mirror of `Fixed::powf`, pinned bit-identical under R-GPU-CANON-PIN; they inherit the rail exactly and any
change to the CPU form must be mirrored there.

## 4. Every production site

Classification: **(a)** provably safe with the bound stated, **(b)** the rail is reachable, **(c)**
demonstrated railing on a production path. The 43 rows split 27 / 9 / 5 / 2 across the four tables below.

Four of the 43 rows (`impact_flux.rs:150`, `:159`, `:306`, `:307`) are calls into the shared
`unsaturated_powf` helper rather than calls to `powf` itself; the single `powf` they all reach is at
`:175`. They are listed at their own line because each passes a DIFFERENT exponent, which is what decides
their classification.

### 4a. Safe by the exponent window (27 sites)

Each of these has an exponent inside `[-0.9687499, +0.9918528]`, so NO base can rail it. The base bound is
irrelevant and is not needed.

| site | computes | exponent | class |
|---|---|---|---|
| `crates/world/src/crater.rs:214` | crater gravity term `pi4^p1` | `p1 = -0.09091` (from mu=0.55, nu=0.4) | (a) |
| `crates/world/src/crater.rs:218` | crater strength inner `pi4^p2` | `p2 = +0.24242` | (a) |
| `crates/world/src/crater.rs:229` | crater efficiency `group^outer` | `outer = -0.64706` | (a) |
| `crates/world/src/impact_flux.rs:159` | size inversion `u_e^(1/e)` | `1/e = -0.4` at p=3.5 | (a) |
| `crates/sim/src/astro.rs:193` | stellar T_eff mass scaling | `alpha/4 - beta/2 = 0.475` | (a) |
| `crates/sim/src/astro.rs:1585` | Roche lobe `q^(1/3)` | `1/3` | (a) |
| `crates/sim/src/astro.rs:1586` | Roche lobe `q^(2/3)` | `2/3` | (a) |
| `crates/sim/src/astro.rs:3043` | X-ray wind metallicity, steep edge | cited `-0.77` / `-0.8` | (a) |
| `crates/sim/src/astro.rs:3044` | X-ray wind metallicity, shallow edge | cited `-0.4` / `-0.6` | (a) |
| `crates/sim/src/stellar.rs:138` | metallicity-luminosity factor | `lambda = -0.44` | (a) |
| `crates/sim/src/stellar.rs:162` | mass-radius factor | `beta = +0.8` | (a) |
| `crates/sim/src/stellar.rs:163` | metallicity-radius factor | `mu = -0.018` | (a) |
| `crates/sim/src/stellar.rs:258` | massive-star T_eff `flux_ratio^(1/4)` | `1/4` | (a) |
| `crates/sim/src/stellar_evolution.rs:241` | CNO metallicity factor | `Z_CNO_EXPONENT = 0.04` | (a) |
| `crates/sim/src/giants.rs:516` | Hill radius `(q/3)^(1/3)` | `1/3` | (a) |
| `crates/physics/src/laws.rs:1951` | Watson latent heat | `0.38` | (a) |
| `crates/physics/src/laws.rs:2121` | free-convection `D_v^(2/3)` | `2/3` | (a) |
| `crates/physics/src/laws.rs:2126` | free-convection Rayleigh cube root | `1/3` | (a) |
| `crates/physics/src/laws.rs:2167` | Lennard-Jones sigma `(T_c/P_c)^(1/3)` | `1/3` | (a) |
| `crates/physics/src/laws.rs:2191` | Neufeld collision integral `A/T*^B` | `B = 0.1561` | (a) |
| `crates/physics/src/laws.rs:3916` | boundary-layer thinning `(Ra_c/Ra)^(1/3)` | `1/3` | (a) |
| `crates/physics/src/ewald.rs:155` | Ewald cell side `V^(1/3)` | `1/3` | (a) |
| `crates/physics/src/ewald.rs:288` | Ewald cell side `V^(1/3)` | `1/3` | (a) |
| `crates/materials/src/conductivity.rs:142` | Hofmeister decline `(298/T)^a` | `0.95` or `0.33`, a two-valued step | (a) |
| `crates/materials/src/mie_gruneisen_debye.rs:275` | Birch-Murnaghan `(V0/V)^(1/3)` | `1/3` | (a) |
| `crates/materials/src/properties.rs:226` | Chen-Tse hardness `(k^2 G)^0.585` | `0.585` | (a) |
| `crates/physics/src/moment_equivalence.rs:1143` | cube root (OTHER LANE, not touched) | `1/3` | (a) |

The three crater exponents deserve a note: they are DERIVED from the coupling constants
(`p1 = (6nu-2-mu)/(3mu)`, `outer = -3mu/(2+mu)`), so their membership in the window is conditional on the
production coupling (`mu = 0.55`, `nu = 0.4`, the only non-test `CraterCoupling` construction, at
`crates/viewer/src/main.rs:2285` and `:6199`). `outer` leaves the window at `mu > 0.954`; a coupling row
that steep would need re-checking. Likewise the two X-ray wind slopes and the mass-radius exponents are
CALLER PARAMETERS whose production values sit inside the window; the window is the bound the functions
state, not one they enforce.

### 4b. Safe by a measured base bound (9 sites)

The exponent is outside the window, so the base carries the bound. Budget is `y * ln(x)` against the
`21.4876` / `-22` rails, measured at the real production inputs.

| site | computes | input bound (measured) | budget | class |
|---|---|---|---|---|
| `crates/world/src/crater.rs:219` | crater strength term `inner^s` | `s = 1.275`; base is the strength group, measured over four production configurations (Mirror escape, Mirror low-v, Moon-class, dense/fast) | `-6.7` to `-13.9` of `-22` | (a) |
| `crates/sim/src/astro.rs:576` | disk surface density `x^gamma` | `gamma` = 1.0 (dust) or 1.5 (solid), `crates/viewer/src/main.rs:1030`, `:1082`; `x = orbit_au/30` over the sampled orbits 0.7 to 1.5 AU | `-5.6` to `+1.8` | (a) |
| `crates/sim/src/astro.rs:580` | disk exponential cutoff `x^(2-gamma)` | exponent 1.0 or 0.5, same `x` | `-3.8` to `+1.2` | (a) |
| `crates/materials/src/mie_gruneisen_debye.rs:243` | Grueneisen `gamma_0 (V/V0)^q` | `q` in `[1.3, 7.8]` across the six usable rows of `crates/physics/data/thermoelastic_anchors.toml`; `V/V0` in `[1/3, 1.5]` (lower edge `v0/3`, upper edge the searched spinodal) | `-8.57` worst (enstatite at full compression) | (a) |
| `crates/materials/src/mie_gruneisen_debye.rs:263` | Debye theta `(1 - (V/V0)^q)` | same | `-8.57` | (a) |
| `crates/materials/src/disk_composition.rs:208` | `ten_pow(x) = 10^x` for the metallicity dex | ONE caller (`:131`, `z_ratio = ten_pow(fe_h)`); `fe_h` is pinned to exactly 0 on the Mirror path, and on the drawn path is hard-bounded to `+-1.2 dex` by the sum-of-12-uniforms `+-6 sigma` | `2.76` of `21.49`, an 8x margin | (a) |
| `crates/world/src/impact_flux.rs:150` | size draw, `umin^(1-p)` | `p = 3.5` so the exponent is `-2.5`, outside the window; min 2000 m, max 8000 m (`crates/viewer/src/main.rs:1197`, `:1218`, `:1229`), so `u_min = 0.25` | `+3.47` of `21.49` | (a) |
| `crates/world/src/impact_flux.rs:306` | number fraction, `u^(1-p)` | same reservoir, `u` in `[u_min, 1]` | `0` to `+3.47` | (a) |
| `crates/world/src/impact_flux.rs:307` | number fraction, `umin^(1-p)` | same | `+3.47` | (a) |

`disk_composition.rs:208` is the one site whose EXPONENT varies and whose base is fixed, the inverse shape
of every other site. It rails at `|x| > 9.33`, and the `+-6 sigma` sampler bound holds it at `1.2`. If the
metallicity scatter is ever widened past about 4 dex, this becomes the first site to break.

The three `impact_flux` paths are safe at the PRODUCTION reservoir and correctly REFUSE when widened,
which is the guard working: measured, a 100 m to 100 km reservoir spends `17.3` and still answers, and a
1 m to 1000 km reservoir spends `34.5`, rails, and `size_at_number_fraction` returns `None` at every
quantile rather than a size. The rail is reachable by widening the size bounds and it is caught.

### 4c. Reachable rails, not demonstrated on a production path (5 sites)

| site | computes | the input that reaches the rail | remedy | class |
|---|---|---|---|---|
| `crates/world/src/impact_flux.rs:175` | the shared `unsaturated_powf`, which guards `power >= Fixed::MAX` but NOT the zero rail | the MAX guard is correct and fires (see 4b). The ZERO rail is unguarded: the helper's name and its doc both claim it refuses "where the pinned `Fixed::powf` would SATURATE", and it catches only half of that | read `checked_powf`'s sentinel rather than re-implementing one side of it | (b) |
| `crates/physics/src/laws.rs:2673` | `ResponseLaw::Power`, `magnitude^shape * gain` | `shape` is unbounded and `magnitude` is unbounded. `ResponseLaw::Power` is constructed ONLY at `laws.rs:4713`, `:4724`, `:4743`, all past the `#[cfg(test)]` boundary at `:4008` | `checked_powf`, falling to `activation_max` as the multiply already does | (b) |
| `crates/physics/src/laws.rs:3230` | Hill uptake `stock^hill` | `stock` is a RAW stock in the source's own units (`laws.rs:3202`), not a normalized fraction, so it has no structural ceiling; `hill` is unbounded. Dead today: `RedoxKinetics` is armed only at `environ.rs:4471`, past the test boundary at `:2875` | `checked_powf`, or restructure the Hill term in log space | (b) |
| `crates/physics/src/laws.rs:3231` | Hill uptake `km^hill` | same | same | (b) |
| `crates/sim/src/stellar_evolution.rs:298` | Hayashi `L_ratio^weak_exponent` | `weak_luminosity_exponent` is `Fixed::ZERO` at every call site, and the zero case short-circuits before the `powf` (`:285-292`), so the line is dead. If armed above `0.9919` against a luminosity near `1.26e5`, it is exposed | `checked_powf` when the exponent is armed | (b) |

The Hill pair has a partial structural defence worth recording: the term is `sh/(kmh+sh)`, and the rail
PRESERVES the ordering, so `stock >> km` still gives 1 and `km >> stock` still gives 0. The failure is
confined to the middle band where both stock and km are large and comparable, where the saturating add
makes `MAX/(MAX+MAX)` read as 1 instead of 0.5.

### 4d. Demonstrated railing, FIXED in this pass (2 sites)

| site | computes | demonstrated failure | fix |
|---|---|---|---|
| `crates/sim/src/stellar.rs:137` | main-sequence luminosity `M^alpha` | at the production `alpha = 3.5` the power rails above **464 solar masses** and returned `Some(Fixed::MAX)` with NO refusal. At 2000 M_sun it reported `2.147e9` against a truth of `3.578e11`, 166x low | `checked_powf` + `?` |
| `crates/sim/src/astro.rs:108` | stellar flux `M^alpha / (4 pi d^2)` | same rail. At 1 AU the wide divide refused it for the WRONG reason (the flux itself overflowed), but that cover fails with distance: at 2000 M_sun and 100 AU the railed luminosity ESCAPED as `2.923e8 W/m^2` against a truth of `4.869e10`, 166x low and plausible | `checked_powf` + `?` |

**Why these are (c) and not (b).** The star mass is a raw command-line argument:
`crates/viewer/src/main.rs:464`, `let star_mass = parse_fixed(argv.get(3), Fixed::ONE);`, and
`parse_fixed` (`:902`) applies no clamp, falling back to `Fixed::ONE` only on a parse failure. Neither
`luminosity_ratio` (`stellar.rs:133`) nor `stellar_flux` guards anything but a non-positive mass. The path
runs `main.rs:468` -> `build_derived_scene` -> `planet::derive_planet` (`main.rs:3200`) ->
`stellar::main_sequence_star` (`planet.rs:106`) and -> `astro::disk_effective_temperature`
(`planet.rs:115`) -> `astro::stellar_flux` (`astro.rs:351`). No default run rails (the default mass is 1,
budget 0; the documented exercised range tops out at 100 M_sun, budget 16.1), but the code accepts the
input and returns a wrong number with no signal.

**The worst shape of the failure, and the reason it hid.** At 463 and 464 M_sun the derived effective
temperature saturates at the `t_max` cap, so the error is masked. Above that the RADIUS power (`beta =
0.8`) keeps growing while the luminosity is pinned at `Fixed::MAX`, and `T_eff ~ (L/R^2)^(1/4)` therefore
falls back UNDER the cap and lands in a plausible band: `main_sequence_star` reported **59387 K at 2000
M_sun where the truth is 213459 K**. A number that re-enters the believable range while being wrong by a
factor of 3.6 is the hardest kind to notice.

**The fix is bit-neutral below the rail.** `checked_powf` runs the identical `y.mul(self.ln()).exp()` and
differs only on the sentinel, so every input that resolved before resolves to the same bits. Proved by
sweep, not by argument: `stellar.rs`, `the_guard_changes_no_bits_below_the_rail`, walks every integer mass
1 to 463 and every hundredth 0.01 to 1.99 and asserts bit equality with the pre-guard form;
`astro.rs`, the tail of `the_stellar_flux_power_refuses_rather_than_letting_a_railed_luminosity_escape`,
does the same over 1 to 463 at 100 AU.

**The one behaviour change beyond the rail**, measured and stated rather than discovered later. Below
`1.865e-3` solar masses the mass power UNDERFLOWS, and the bare form returned exactly zero where the guard
now returns `None`. Both sit at the representable floor (the true value is under one ulp), so neither is a
magnitude error; the change turns a zero-luminosity "star" into a refusal. That mass is 43x below the
hydrogen-burning limit of about 0.08 solar masses, so no object in the affected range is a main-sequence
star, and the function already refuses the population-III boundary on the same "a flagged boundary rather
than an extrapolation" grounds. The refusal is consistent with what the function documents about itself.

## 5. A documentation defect found and fixed

The prior session's rail write-up was attached to the WRONG FUNCTION. The whole block, from "Real power
`x^y = exp(y * ln x)`" through the survey of budgets, sat above `pub fn checked_powf`, and `pub fn powf`
carried no doc comment at all. A caller hovering `powf`, or reading `cargo doc`, saw nothing about the
rail the block was written to warn them about, which is the exact failure the block was written to end.
The block also opened by stating "A non-positive base returns zero (the domain guard)", true of `powf`
and false of `checked_powf`, contradicting the code two paragraphs below it.

The block is now split at its natural seam: `powf` carries the rail documentation plus the two-rail
distinction and the exponent window, and `checked_powf` carries its own summary and both of its false
positives. The survey paragraph's stale claim, that the impact size-frequency ratio is "the one shape that
rails in practice", is corrected: that site is now guarded and refuses correctly, and the shape that was
found railing and escaping is the stellar mass-luminosity power.

## 6. What this audit did not cover

The three files owned by other lanes were read for classification but not touched:
`crates/physics/src/moment_equivalence.rs` (one site, exponent `1/3`, safe by the window),
`crates/sim/src/deeptime.rs` (no direct `powf`; it calls `size_at_number_fraction`, audited above), and
`crates/physics/src/convection_scaling.rs` (one f64 site in a test).

The precision-degradation concern in section 1 is a real and separate finding this audit did not chase to
ground: `checked_powf` catches neither the 1% quantization at `y ln x = -18` nor the 12% at `-20`, and no
site in the tree currently runs that deep, with `crater.rs:219` at `-13.9` the closest. A budget past
about `-16` should be treated as needing a log-space form (the precedent is `laws::ln_rayleigh_number`,
`laws::ln_stokes_velocity`, and `impact_flux::ln_mean_cube_size_ratio`) rather than a checked power.

Two adjacent sites use `Fixed::exp` directly on a dex-scale quantity rather than through `powf`, and so
fell outside the grep: `crates/physics/src/solar_abundances.rs:347` and `crates/sim/src/astro.rs:1456`,
both computing `10^(log_eps - 12)`. Five photospheric rows in
`crates/physics/data/solar_abundances_agss09.toml` sit below the `log_eps = 2.45` underflow threshold and
contribute exactly zero to the mass fractions. This is the LOWER rail, so each is individually below one
ulp and zero is the correct representable answer; it is recorded because the SUM of many sub-ulp trace
contributions is not necessarily sub-ulp, which is an accumulation question this audit did not settle.
