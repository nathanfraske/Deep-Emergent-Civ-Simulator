# The flexure and moment-equivalence representation design

A `gpt-5.6-sol` research pass at maximum reasoning effort, 2026-07-19, commissioned after four point
fixes to the flexure chain failed to make it run at planetary scale. The brief asked for a designed
solution rather than a diagnosis, with worked worst-case magnitudes so the claim would be checkable.

WHY THIS IS HERE AND NOT IN A COMMIT MESSAGE. It supersedes my own conclusions in three places and I want
those visible to whoever picks this up:

  - I said the chain needed logs "the way the Rayleigh and Stokes paths already do". It says NOT to carry
    `D`, `alpha` or the amplitudes in logs at all after scaling, and gives the reason: those quantities
    participate in bracketing, subtraction and ordering, and `alpha` is best served by nested exact
    integer square roots, which a log form would replace with two approximate transcendental evaluations.
    Logs belong only at the final exponential-product boundary.
  - It located the fifth overflow, which I had failed to find by trial, AND found a defect in my own fix:
    the `8D` denominator uses `?`, so the function returns BEFORE reaching the log fallback I added. My
    fallback is unreachable on exactly the case it was written for.
  - It diagnosed the fixed-point start-dependence properly. I had already retracted my "step versus
    residual" explanation as false; this supplies the real one, CURVATURE QUANTIZATION, and checks it
    numerically: one curvature ulp is about `dD = 0.0753` and the two endpoints differ by `0.104`, which
    is 1.4 curvature bins. It also shows the map IS a contraction (`F' = (3/4)(1-n)`, bounded by 0.75) so
    damping would make it worse, and recommends curvature-space bracketing instead.

It also enumerates ELEVEN remaining range holes, most of which I had not found, including three
point-load `2 pi D` sites, the unfixed `flexural_length_axisymmetric` division, `solve_point_load`'s
unclamped trial, the spectral filter's `(lk)^4`, a far-field `exp` underflow that can zero a decay while
the amplitude times decay is still representable, and an `axial_force` reduction whose intermediate grows
with sample count.

The verbatim answer follows.

---

The right fix is a single coherent internal unit system, not more local fallbacks. I recommend:

- length: `L₀ = 32 km`
- time: `t₀ = 32 s`
- stress: `S₀ = 1 GPa`
- density: `ρ₀ = 1000 kg/m³`

All scales are powers of two, so boundary conversions are deterministic shifts/divisions. This keeps the entire requested range—and even the 800 km fully elastic starting plate—well inside Q32.32.

## 1. The fifth failure, and the remaining hazards

The observed second-iteration failure is at [flexure.rs:327](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:327):

```rust
let eight_d = Fixed::from_int(8).checked_mul(d)?;
```

At `D_new ≈ 8.89e8`, this attempts to form `8D ≈ 7.11e9`. The `?` returns before the log fallback is entered. At `α ≈ 731 km`, `α³ ≈ 3.91e8` still fits; the denominator is the immediate failure.

The full set of current range holes is:

- `E T³` in `flexural_rigidity`.
- `D/(Δρg)` in `flexural_parameter`.
- The same unfixed division in [flexural_length_axisymmetric](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:216).
- `α³` itself: at the requested maximum, `α ≈ 1911 km` and `α³ ≈ 6.98e9`; the log fallback cannot run because [the cube is formed first](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:324).
- `8D`, as above; it overflows for `D > 2.684e8`.
- `V₀α³`, when the preceding two happen to fit.
- `2πD` in [point-load deflection](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:402), and in both point-load curvature functions at [moment_equivalence.rs:1510](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1510) and [moment_equivalence.rs:1536](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1536). It overflows for `D > 3.418e8`.
- `solve_point_load` still forms the unclamped full-domain rigidity directly at [moment_equivalence.rs:1584](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1584).
- `mechanical_rigidity` and hindcast rigidity conversion inherit the rigidity-numerator ceiling.
- `(lk)^4` in the spectral filter overflows for `lk ≳ 215`, although the response is then simply a very small representable-or-zero number.
- `exp(-x/α)` underflows separately at about `x/α = 22`. With a large amplitude, `w₀ exp(-x/α)` can still be representable after the decay alone has become zero.
- `axial_force` sums unscaled stresses before multiplying by the grid step. Its intermediate grows with sample count even though the integral does not.

The current amplitude comments correctly describe a historical duplicate, but the active second-iteration refusal is now `8D`.

## 2. Proposed internal units

With `L₀ = 32 km`:

| Quantity | Internal scale | Conversion to internal |
|---|---:|---:|
| Length/deflection | `32 km` | `x̂ = x / 32` |
| Time | `32 s` | — |
| Acceleration | `0.03125 km/s²` | `ĝ = 32g` |
| Stress | `1 GPa` | unchanged |
| Density | `1000 kg/m³` | unchanged |
| Rigidity | `32768 GPa km³` | `D̂ = D / 32768` |
| Line load | `32 GPa km` | `V̂ = V₀ / 32` |
| Moment or point load | `1024 GPa km²` | `M̂ = M / 1024` |
| Curvature | `1/32 km⁻¹` | `κ̂ = 32κ` |
| Wavenumber | `1/32 km⁻¹` | `k̂ = 32k` |

The dimensionless requested ranges become:

```text
T̂       = 0.15625 .. 25
D̂       = 0.305176 .. 30517.578125
ĝ        = 0.032 .. 0.96
Δρ ĝ     = 0.0096 .. 3.84
V̂       = 0.003125 .. 15.625
x̂, r̂   = 0 .. 156.25
```

For a defensible rigidity proof, `E` and `ν` also need declared bounds; the request does not supply them. The numbers below use the generous contract `E ≤ 512 GPa`, `|ν| ≤ 0.5`. The Mars case is far inside it.

### Rigidity

```text
D̂ = Ê T̂³ / [12(1 - ν²)]
```

At `T = 800 km`:

```text
T̂²             = 625
T̂³             = 15625
Ê T̂³          ≤ 8.00e6       (E ≤ 512 GPa)
12(1 - ν²)      ≥ 9            (|ν| ≤ 0.5)
D̂              ≤ 8.89e5
```

That upper value corresponds to a physical fully elastic rigidity of roughly `2.91e10 GPa km³`. It cannot be returned through the old external unit, but it can safely exist as a private starting trial. The requested returned range `D ≤ 1e9` is only `D̂ ≤ 30517.6`.

### Flexural lengths

Let `R̂ = Δρ ĝ`.

```text
α̂ = sqrt(2 sqrt(D̂/R̂))
l̂ = sqrt(sqrt(D̂/R̂))
```

For the requested `D` range:

```text
D̂/R̂             = 0.07947 .. 3.1789e6
sqrt(D̂/R̂) max   = 1782.95
2 sqrt(...) max   = 3565.90
α̂                 = 0.75088 .. 59.7152
l̂                 = 0.53095 .. 42.2250
```

Including the full `800 km`, `E = 512`, `|ν| = 0.5` starting trial:

```text
D̂/R̂ max          = 9.2593e7
α̂ max             = 138.73
```

Still ample headroom.

### Line-load amplitude and curvature

```text
ŵ₀ = V̂ α̂³ / (8D̂)
κ̂  = C V̂ α̂ / (8D̂)
C   = 2 sqrt(2) exp(-3π/4) ≈ 0.268079
```

Across the requested range:

```text
α̂² max                 = 3565.90
α̂³ max                 = 212938.46
V̂ α̂³ max              = 3.3272e6
8D̂ max                 = 244140.63
largest independent
  amplitude quotient    = 1.363e6
largest physically
  linked ŵ₀            = 242.35
smallest linked ŵ₀     = 3.05e-5
linked κ̂ range          = 4.58e-8 .. 5.7614
```

The smallest curvature is about 197 internal ulps, so it does not disappear. The full 800 km starting trial gives:

```text
V̂ α̂³ ≤ 4.172e7
8D̂    ≤ 7.111e6
```

For the measured second iteration specifically:

```text
D̂      ≈ 27130
α̂      ≈ 22.84
V̂      = 2.5             (for V₀ = 80)
8D̂     ≈ 217040
V̂ α̂³  ≈ 29800
```

Nothing approaches the Q32.32 ceiling.

Curvature should use the direct expression above, rather than materializing `w₀/α²`. A private `ScaledLineLoadState` can own `D̂`, `α̂`, `ŵ₀`, and `κ̂`, retaining the “one home” invariant.

### Point load

```text
ŵ = P̂ l̂² kei(r̂/l̂) / (2πD̂)
```

For the requested rigidity range:

```text
l̂² max       = 1782.95
2πD̂ max      = 191747.60
r̂/l̂ max     ≈ 294.3
```

If the intended point-load range is also numerically `0.1..500 GPa km²`, then `P̂ ≤ 0.4883` and `P̂l̂² ≤ 870.6`. If point loads have another range, that bound must be supplied; the scale and expression remain the same.

### Moment-equivalence arithmetic

Internally:

```text
σ̂elastic = [Ê/(1-ν²)] κ̂ (ẑ-ẑn)
N̂        = ∫ σ̂ dẑ
M̂        = ∫ σ̂ (ẑ-ẑn) dẑ
D̂eq      = M̂/κ̂
T̂e       = cbrt[12(1-ν²)D̂/Ê]
```

With `E ≤ 512`, `|ν| ≤ 0.5`, `|ẑ-ẑn| ≤ 25`, and the maximum linked curvature:

```text
E/(1-ν²) max            = 682.67
plane_strain * κ̂ max   = 3933
|σ̂elastic| max         = 9.83e4
|N̂| loose bound        = 2.46e6
|σ̂ lever| max          = 2.46e6
moment/tail loose bound = 6.15e7
T̂ inversion radicand   ≤ 15625
```

All are below `2.147e9`.

Use `Fixed::checked_sum` for trapezoidal stress and moment reductions, then apply the grid step once. It uses the numeric type’s deterministic wide-bit reduction and gives the same answer as the existing addition when the old prefixes fit, while removing sample-count-dependent prefix overflow.

## 3. Where logarithms belong

Do not carry `D`, `α`, or the central amplitudes permanently in logs after scaling.

- `D̂` participates in bracketing, subtraction, ordering, and `M/κ`; linear is the natural representation.
- `α̂` is best computed by nested exact integer square roots. A log form replaces an exact root with two approximate transcendental evaluations.
- The scaled line amplitude is `3.05e-5 .. 242.35`; point amplitudes are similarly tame. Logs buy no range.

Logs are appropriate at the final exponential-product boundary:

```text
ln|ŵ(x)| =
    ln|ŵ₀|
    - X
    + ln|cos X + sin X|,
X = |x̂|/α̂
```

Carry the sign separately and exponentiate once. That prevents `exp(-X)` from becoming zero while the amplitude-times-decay remains representable. If the combined log falls below the `exp` floor near `-22`, the final scaled result itself is at roughly one ulp and zero is honest.

The same principle should be used for a far-field Kelvin asymptotic: combine the Green-function decay with the load coefficient before exponentiating.

Existing log-domain strain rate, Rayleigh number, and creep calculations should remain logarithmic because their underlying linear quantities genuinely fall below Q32.32 resolution. That is a different situation from scaled plate rigidity.

## 4. Kelvin functions

The length scaling does not alter Kelvin arguments:

```text
r̂/l̂ = r/l
```

Therefore the existing series coefficients, roots, and moment-equivalence values remain dimensionless and unchanged.

The ascending series does have an intrinsic range limit. At `x = 12`:

```text
(x/2)^2 = 36
(x/2)^4 = 1296
largest individual recurrence term ≈ 4199
largest accumulated intermediate   ≈ 7248
```

So the current `0 < x ≤ 12` series is safely inside Q32.32. The problem is cancellation and accuracy, not overflow.

There are two existing non-scaling limits:

- Above `12`, `kei`, `ker`, and `kei'` return zero. At `x = 12`, the real magnitudes are still approximately `6.3e-5` and `3.9e-5`. A sufficiently large point-load coefficient can turn that into metres of deflection, so the hard zero is not generally amplitude-aware.
- Extremely close to zero, `kei_prime` forms `1/(x/2)`, and `ker` can form `ln(0)` after `x/2` rounds away.

The implementable completion is:

- retain the current series on a validated central interval;
- use small-`x` expansions below it, evaluating `ln(x/2)` as `ln x - ln 2`;
- use the complex `K₀(xe^{iπ/4})` asymptotic above `12`:
  `sqrt(π/(2z)) exp(-z) [1 - 1/(8z) + 9/(128z²) - …]`;
- carry the exponentially decaying prefactor in signed-log form and combine it with the point-load coefficient before `exp`.

The moment-equivalence solver itself always evaluates `ker` and `kei'` at `x₀ = 3.91467`, so its Kelvin arithmetic stays on the existing series and should remain bit-identical.

## 5. The fixed point

The “step versus residual” explanation is indeed false:

```text
Dnew = F(D)
|Dnew-D| = |F(D)-D|
```

The observed start dependence is much more consistent with curvature quantization.

For a line load:

```text
κ ∝ D^(-3/4)
```

At the Earth-like fixture near:

```text
D ≈ 216293
κ ≈ 8.91375e-4 /km
```

one external curvature ulp corresponds to approximately:

```text
ΔD ≈ (4D / 3κ) EPSILON ≈ 0.0753 GPa km³
```

The two reported endpoints differ by about `0.104`, or roughly 1.4 curvature bins. That is an excellent match to a quantized-curvature plateau.

The continuum map is not expected to be near identity. If

```text
n = d ln M / d ln κ,     0 ≤ n ≤ 1
```

for an elastic-plastic moment curve, then at a fixed point:

```text
F'(D) = (3/4)(1-n),      0 ≤ F' ≤ 0.75
```

Direct substitution is a contraction. Under-relaxation would move that derivative toward one and make convergence slower. I would not damp it.

I would replace direct substitution with a curvature-space bracket.

For line loads, define:

```text
A = C V̂ 4^(1/4) / [8 R̂^(1/4)]
κ̂ = A D̂^(-3/4)
```

At the fixed point:

```text
M̂load(κ̂) = A cbrt(A/κ̂)
H(κ̂)      = |M̂yield(κ̂)| - M̂load(κ̂)
```

`M̂yield` is nondecreasing with curvature, while `M̂load` is strictly decreasing. This gives a unique, naturally bracketable crossing. Do not form `A⁴`: at the weakest-load corner it underflows. The factored `A*cbrt(A/κ̂)` stays safe:

```text
A                    = 1.06e-4 .. 2.366
A/κ̂ at a valid root = D̂^(3/4) = 0.411 .. 2309
```

For point loads, the target is simpler:

```text
B = |bracket(ν)| P̂/(2π)
solve |M̂yield(κ̂)| = B
```

Use the declared `D` ceiling and floor to derive the initial curvature bracket. Bisect raw Q32.32 values until the endpoints are adjacent and return a declared canonical side, for example the smallest curvature with `H ≥ 0`. This gives exact start independence and removes the need for a guessed iteration tolerance or initial-trial clamp.

Before switching, one measurement should be made: sweep `H(κ̂)` over all production profiles and adversarial envelopes and assert monotonicity. The continuum argument is strong, but the sampled neutral-surface solve can introduce one-ulp stair steps; that property should be measured rather than assumed.

Scaling alone should reduce the observed curvature plateau width by about 32×, but bracketing is what removes the start-selection ambiguity.

## 6. Bitwise migration

Dimensional results should be expected to change in their low bits. Scaling changes where Q32.32 truncation occurs, even though the scales are powers of two.

Must remain bit-identical:

- direct `kelvin_kei`, `kelvin_ker`, and `kelvin_kei_prime` results on the retained series interval;
- the Kelvin-derived point-load coefficients at `x₀`;
- mathematical constants;
- exact zero-load results;
- validation/refusal and sign-convention behavior;
- chord and band metadata.

Expected to change in low bits:

- `D`, `α`, flexural lengths, amplitudes and deflections;
- curvature;
- neutral depth, moment, and equivalent rigidity;
- iteration count;
- results whose path currently switches between linear and log arithmetic.

Stage the migration as separate observable changes:

1. Add golden Earth-like vectors using current `to_bits`, plus the full Mars case that currently refuses at the second iteration.
2. Add private scaled kernels and dual-evaluate them in tests without changing returned results. Record absolute and relative differences.
3. Switch `flexure.rs` to scaled linear arithmetic. Review and commit the golden deltas separately.
4. Switch the moment profile, curvature, moment and rigidity internals to scaled quantities, while retaining direct substitution temporarily.
5. Add the Mars end-to-end regression and the full range-corner matrix.
6. Replace direct substitution with curvature bracketing in a separate commit; this isolates solver movement from unit-system movement.
7. Add Kelvin small/large-argument branches separately.
8. Remove obsolete fallbacks and clamp tests only after the scaled path has proven the range table.

Do not bulk-update exact expectations in the same change that introduces scaling. Keep the pre-migration values visible in a checked comparison table so a physical regression cannot masquerade as a rebaseline.

## 7. What I would remove or retain

Remove:

- `max_representable_elastic_thickness`;
- `initial_trial_rigidity`’s clamp;
- the logarithmic `flexural_parameter` fallback;
- central line- and point-amplitude log fallbacks;
- tests whose sole purpose is proving those fallbacks match the linear path.

Retain:

- the single shared ownership of `line_load_amplitude`;
- checked arithmetic and fail-loud public conversions;
- logarithmic strain rate, Rayleigh, and creep paths;
- Kelvin’s dimensionless series and its existing reference tests.

I would not:

- merely change `8D` to `D`-then-`8`, because the cube, point denominators, initial trial, axisymmetric length, filter, and far-field underflow remain;
- preserve old Earth bits by continuing to return a legacy linear-first path, because that leaves two authoritative numerical semantics;
- damp the current iteration;
- clamp or saturate physical intermediates;
- form the line-load bracket constant as `A⁴`;
- raise the Kelvin series cap without adding an asymptotic form;
- use floats or a second canonical numeric type.

The only missing range declaration needed to turn this into a literal proof is the supported `(E, ν)` envelope and, for point deflection, the point-load magnitude range. With `E ≤ 512 GPa`, `|ν| ≤ 0.5`, and point loads up to `500 GPa km²`, the `32 km / 32 s / 1 GPa` system has at least an order of magnitude of headroom at every material intermediate and substantially more through the main flexure chain.
