# The unified rate-law kernel: a domain-neutral physics-floor primitive (design-first)

This is the design opener for the next arc, owner-ruled and gate-sequenced: the unified Eyring/Arrhenius
rate law as a domain-neutral physics-floor primitive, with Stage 5 (the freezer) as its first consumer. It
authors no mechanism and moves no value yet. Its purpose is to surface the neutral kernel and its interface
for the gate's review (with the local-broker second opinion) before a single line of the kernel is built.
The build lands on this branch when the gate rules on the design.

The five run_world/materials pins are untouched (this is a doc-only change), and the disposer arc it branches
from is complete, audited, and merge-ready on `#186`.

## 1. What the kernel is, in one sentence

A reaction, a hop, or a crossing happens at a rate set by how the available thermal energy compares to the
barrier in the way, times how often the system attempts the crossing:

    rate = prefactor * exp(-reduced_barrier)

where `reduced_barrier` is the single dimensionless group `E* / (k_B * T)` (equivalently the molar
`E_a / (R * T)`, the same number), and `prefactor` is the attempt frequency the caller supplies in its own
working unit. Nothing about a metal, an enzyme, a mantle, or a mind enters the kernel's signature. The domain
lives entirely in what the caller computes for the barrier and the prefactor before the call; the kernel is
the one shared law that turns a barrier and a temperature into a rate.

## 2. Why one kernel: the consumers, reconciled

The gate named five reference documents to reconcile before designing, plus the standing charge that unifying
the abiogenesis item (#37) is exactly reconciling what those consumers each need into one kernel interface.
Read against source, every named consumer wants the same shape (a barrier over a thermal energy, exponentiated,
scaled by an attempt frequency), and each differs only in what supplies the barrier and the prefactor. That is
the definition of a domain-neutral primitive: fixed Rust law, per-consumer data.

**The freezer (Stage 5), the first consumer** (`MATERIALS_ORACLE_SPEC.md`, the freezer stage). Its diffusion
and viscosity rates are Arrhenius in the self-diffusion barrier: `E* = g * R * T_m` with `g` a class-and-
mechanism constant (the spec cites 17 to 18 for close-packed metals, 26 to 30 for covalent, a grain-boundary
fraction of 0.4 to 0.6), and the prefactor is the attempt frequency `nu = c_s / a` (sound speed over lattice
spacing), closing the canonical `D0 ~ a^2 * nu ~ 1e-5 m^2/s`. The freezer forms `E*` and `nu` from its own
material data, then calls the kernel. Section 6 treats `g` under the derive-first check.

**Forgetting and memory fade** (`LIFE_DERIVATION_FRONTIERS_SCOPING.md`). The e-folding time of a memory trace
is `tau = 1/k` with `k` the memory medium's Arrhenius turnover, `k = reference_rate * exp(-Ea / (k_B * T))`,
set by the being's own body temperature and the medium's decay constant and activation energy (both per-organism
data). A crystalline-lattice mind carries a near-zero decay constant and forgets on geological timescales, a
data row. This is the kernel with `prefactor = reference_rate` and the barrier `Ea` read from the tissue.

**Abiogenesis, the origin of evolving life** (#37, `LIFE_DERIVATION_FRONTIERS_SCOPING.md`, and the built
`laws::reaction`). Today `laws::reaction` returns `(delta_h, temperature >= barrier)`: a HARD BOOLEAN gate, a
step function that is either fully on or fully off at the ignition threshold. This is the crude stand-in the
wave-2 floor left in place pending the exp kernel. The rate kernel replaces that step with a smooth Arrhenius
rate: a reaction proceeds at `prefactor * exp(-barrier / (k_B * T))`, continuous in temperature, so a warm
compartment reacts fast, a cold one slow, and neither snaps on at a single threshold. The pre-biotic reaction
network's mass-action kinetics ride on this, driven by the free energy its bounding gradient supplies (the
already-built `battery_emf` and Nernst), under mass conservation, never reading life-status.

**Productivity and carbon fixation** (`PRODUCTIVITY_DERIVATION_KICKOFF.md`). This consumer is the sharpest
input-audit catch of the survey, and it constrains the kernel's shape. A carbon-fixation enzyme's thermal
response is NOT a monotonic Arrhenius (which only ever rises with temperature): it is a THERMAL OPTIMUM, rising
to a peak near the enzyme's optimum and FALLING above it as the enzyme denatures. That curve is the product of
an activation term and a high-temperature deactivation term (the Johnson-Lewin / Sharpe-Schoolfield form), i.e.
TWO Arrhenius factors, one forward and one for the denaturation, not one. The design consequence is a hard
boundary the kernel must respect: the kernel is the SINGLE monotonic Arrhenius factor, and a thermal-optimum
curve is COMPOSED from two calls (an activation rate and a deactivation rate), never wired as a special mode
inside the kernel. Baking a peaked shape into the kernel would author biology into a physics primitive. The
productivity doc's own Fork 1 (whether to build the exponential Johnson-Lewin now or a float-free tent) is a
consumer-side design call for that arc; the rate kernel only owes it a clean single-factor Arrhenius to compose
from.

**Geology: solid-state creep and radiogenic decay** (`GEOLOGY_ARC_PACKET.md`). The creep-viscosity kernel reads
`chem.activation_energy` plus a per-world reference viscosity and is Arrhenius in exactly this barrier: mantle
convection is solid-state Arrhenius creep. That is a direct call with the barrier read from the material's floor
datum. Radiogenic decay is a SIBLING but a DISTINCT law: `activity = lambda * N`, a first-order decay whose rate
constant `lambda` is temperature-INDEPENDENT (a nuclear constant, not a thermal barrier). It shares the "rate
constant times amount" shape but not the Arrhenius temperature dependence, so it is NOT a consumer of this kernel;
it is a separate one-line law reading a cited decay constant. Naming it here keeps the boundary honest: this
kernel is the THERMALLY-ACTIVATED rate, not every first-order rate.

**The two built precedents** (`crates/physics/src/laws.rs`). Two laws already on the canon path establish the
exact discipline and are the closest prior art. `nernst_emf` and `reversible_uptake_flux` both work at the
per-particle scale `k_B * T` (never the molar `R * T`), and the enzyme drive is literally
`1 - exp(-q E / (k_B * T))` over `Fixed::exp`. The rate kernel is the same arithmetic, generalized and named,
and it should share their guard-and-saturate idiom exactly.

## 3. The interface (proposed, for the gate's ruling)

Three functions in `crates/physics/src/laws.rs`, in the wave-2 chemistry neighbourhood where `reaction`,
`nernst_emf`, and `reversible_uptake_flux` already live. The core kernel carries no domain word:

```rust
/// The thermally-activated rate law: rate = prefactor * exp(-reduced_barrier), the one shared Arrhenius/
/// Eyring primitive. `reduced_barrier` is the SINGLE dimensionless group E*/(k_B*T) (equivalently the molar
/// E_a/(R*T), the same number), formed by the caller at its own working scale; `prefactor` is the attempt
/// frequency in the caller's own rate unit (a constant Arrhenius A, an Eyring k_B*T/h formed at a tractable
/// scale, or the freezer's nu = c_s/a). Domain-neutral: no material, organism, or mechanism enters. A non-
/// positive prefactor yields zero (no attempts, no rate). The reduced barrier is clamped non-negative (a
/// negative barrier is not a rate law: it would author an unbounded rate); above the exp window it saturates
/// to zero (the frozen regime, an honest Q32.32 limit; see the design note). Deterministic fixed-point
/// (`Fixed::exp`, the pinned R-GPU-CANON-PIN reference, integer-only and bit-identical on every backend).
pub fn arrhenius_rate(prefactor: Fixed, reduced_barrier: Fixed) -> Fixed

/// Form the dimensionless reduced barrier E*/(k_B*T) from a barrier energy and a thermal energy in MATCHING
/// units (both per-particle, or both molar; the ratio is scale-free either way). Guards a non-positive
/// thermal energy (returns the saturating sentinel so the rate collapses to zero, no thermal scale = no
/// crossing). This is where the Buckingham-Pi group is assembled; the kernel above never sees the units.
pub fn reduced_barrier(barrier_energy: Fixed, thermal_energy: Fixed) -> Fixed

/// The Eyring transition-state prefactor k_B*T/h (the universal attempt frequency of TST), formed at a
/// caller-supplied working scale. SURFACED, NOT ASSUMED: at SI scale k_B*T/h is ~6e12 /s, far outside the
/// Q32.32 range, so the fundamentals cannot be multiplied raw; the caller passes k_B*T and h pre-folded to
/// its own frequency unit (the same once-at-a-cited-scale fold nernst_emf and the collision integral use).
/// A constant-Arrhenius consumer or the freezer's nu = c_s/a does not call this at all.
pub fn eyring_prefactor(kt_over_h_scaled: Fixed) -> Fixed
```

The signature of `arrhenius_rate` is the whole point: `(prefactor, reduced_barrier)`, two scalars, no domain.
Every consumer computes those two from its own data and calls the same function. This is stricter than the
gate's framing `rate = prefactor * exp(-E*/(R*T))`, which names `R` and `T` in the kernel; assembling the
dimensionless group in the `reduced_barrier` helper instead keeps the kernel blind to whether the caller
worked in molar or per-particle units, and blind to `R` entirely (Section 6 shows why that matters for the
value line).

## 4. The Buckingham-Pi budget: exactly one group

The gate set the budget at one dimensionless group, and the design holds to it. The exponent's argument is the
single group

    Pi_1 = E* / (k_B * T) = E_a / (R * T)

the activation energy in units of the thermal energy. `rate / prefactor = exp(-Pi_1)` is dimensionless, and
`Pi_1` is the only group inside the transcendental. The prefactor carries the rate's dimension and sets the
scale; it is not a second dimensionless group. Nondimensionalizing the Arrhenius law by the thermal energy and
the attempt frequency leaves exactly `Pi_1`, which is the budget the gate named. Any consumer that needs a
second dimensionless quantity (the productivity thermal optimum needs a second barrier for the deactivation
branch) forms a SECOND, independent `Pi_1` for its second Arrhenius factor and composes, so the kernel's own
budget stays at one group per call.

## 5. The transcendental coupling, surfaced (the exp gap)

The gate asked to surface how the fixed-point transcendental couples in, with no float. The honest state, read
against source rather than the pre-resolution memory of it:

`Fixed::exp` EXISTS and is deterministic on the CPU canon path. R-GPU-CANON-PIN is resolved and built (record
62.23, audit block 1y): the exp/ln oracle is `crates/core/src/fixed.rs`, an 18-term Maclaurin over a range-
reduced argument, integer-only and pinned. It is already consumed on the canon path by `nernst_emf`, the enzyme
drive in `reversible_uptake_flux`, the Neufeld collision integral, and a vapour-pressure curve. So the kernel is
buildable on CPU today; the transcendental is not a blocker there.

Two honest couplings remain and must be stated in the kernel's own docs, not buried:

First, the window. `Fixed::exp` is representable on about `[-22, 21.5]`; outside it saturates to zero or the
maximum, an honest Q32.32 limit. The kernel's argument is `-Pi_1`, always non-positive, so the only edge that
bites is `Pi_1 > 22`, which underflows the rate to zero. This is not a defect: a barrier more than 22 thermal
energies high has a rate below `e^-22 ~ 3e-10` of the prefactor, which is zero at any tick resolution the sim
runs. For the freezer this is exactly the frozen regime: with `g ~ 17` to `18`, `Pi_1 = g * T_m / T` crosses 22
near `T ~ 0.77 * T_m`, so below roughly three-quarters of the melting point the diffusion rate reads zero, which
is the physical freeze-out, correct behaviour rather than a clipped one. The kernel states this limit; it does
not paper over it.

Second, the GPU lane. The remaining OPEN follow-on under the resolved R-GPU-CANON-PIN is the cross-vendor GPU
`#[cube]` exp kernel (the CPU oracle is proven; the multi-vendor GPU transcendental run is not). So the rate
kernel is CPU-canon now and GPU-deferred until that follow-on lands, the same state every other exp-consuming
law is already in. No new determinism surface is opened by this arc.

## 6. The value line and the derive-first check on `g`

No rate value is authored anywhere. The kernel is fixed Rust law. The fundamentals it needs (`k_B`, `h`) are on
the closed fundamentals list and are floored; the molar gas constant `R = N_A * k_B` is DERIVABLE, never
authored, and the design avoids it entirely by working at the per-particle `k_B` scale in the `reduced_barrier`
helper (the same choice `nernst_emf` documents, which sidesteps the `R = N_A * k_B` and `F = N_A * e` composite
drift). Every barrier and every prefactor is per-consumer data or a per-consumer derivation. That leaves one
constant to run the derive-first check on, as the gate directed: the freezer's `g`.

The finding, surfaced for the owner rather than settled here. `g = E* / (R * T_m)` is the self-diffusion
activation energy in units of `R * T_m`. It is NOT a fundamental (not on the closed list c, k_B, h, e, eps_0,
N_A), and it is NOT authorable as a floor value. Read against the metallurgy, `g` is an empirical correlation,
the Van Liempt / Sherby-Simnad relation, and it is structure-and-valence dependent (close-packed near 17 to 18,
covalent higher, grain-boundary a fraction of the bulk), so it is a per-class datum, not one number. Its
physical basis is that the diffusion barrier is the vacancy formation plus migration enthalpy, and both scale
with the cohesive energy, which also sets `T_m`, so `g` exists because two bond-strength measures track each
other. Under the geology packet's category test, a cited, same-every-world, per-class physical correlation
classifies as UNIVERSAL REFERENCE DATA (a data vector in a floor `.toml`, sibling to the reference substances),
keyed off the material's own bonding-class and mechanism, so an alien solid of a different class is a data row,
never a rewrite. That is the conservative derive-first outcome: `g` is reference data, cited to Van Liempt 1935 /
Sherby-Simnad 1962 / Brown-Ashby 1980, per mechanism, not authored, not fundamental.

The sharper derive-first alternative, surfaced for the owner's call (this is the seam, and it is the owner's to
rule, not mine to plant). The materials arc now DERIVES the cohesive energy directly (the Rose UBER `E_coh` in
`crates/materials/src/metallic.rs`). Since the diffusion barrier and `T_m` both track `E_coh`, the freezer could
key the barrier off the already-derived cohesive energy plus the vacancy-fraction physics, `E* = f(E_coh)`,
reducing `g` from a read constant to a smaller derived residual (the vacancy-formation fraction of the cohesive
energy), rather than reading `g * R * T_m` wholesale. This is more derive-first and less reference-data, and it
reuses a quantity the floor already carries. It is also more work and may not close cleanly (the vacancy
energetics are not floored). I am NOT choosing between the two; I am surfacing that the conservative form (g as
per-class reference data) is safe and buildable now, and the sharper form (barrier from E_coh) is the deeper
derivation to weigh. The gate and owner rule which the freezer uses; the kernel is identical either way, since
the freezer forms `E*` before the call.

## 7. The numerical twin

The standing numerical-twin rule (RUNBOOK section 5, added this session) applies. The kernel's analytic
signature is the Arrhenius slope: `d ln(rate) / d(1/T) = -E*/k_B` (per-particle) or `-E_a/R` (molar), a straight
line whose slope IS the barrier. The kernel ships a numerical twin that recovers this slope by central
differences of `ln(arrhenius_rate)` over `1/T`, with a step-size sweep confirming the `h^2` plateau, and checks
the recovered slope against the barrier within fixed-point tolerance. This is a strong test: it confirms the
kernel's temperature dependence is the barrier and nothing else, and it is the same discipline the analytic
derivatives across the physics floor now carry.

## 8. What this arc builds, after the gate rules

Nothing until the design is reviewed. On the gate's ruling (with the local-broker second opinion), the build is
one small, byte-neutral slice: the three functions of Section 3 in `laws.rs`, their guard-and-saturate idiom
copied from `nernst_emf`, the determinism and numerical-twin tests, and a load/round-trip proof. The freezer
wiring is a SEPARATE later slice (it depends on the freezer stage existing, which is downstream); this arc
delivers the neutral primitive and proves it in isolation, exactly as the physics floor's other kernels were
proven before their consumers arrived. The abiogenesis smooth-rate replacement of the `reaction` boolean gate,
the memory-turnover consumer, and the creep-viscosity consumer are each their own later, flag-first slice.

## 9. Honest limits and the open questions for the gate

- The kernel is the single monotonic Arrhenius factor. Peaked thermal-optimum curves (the productivity enzyme)
  are composed from two calls and are the consumer's arc, not this kernel's shape. Confirm the composition
  boundary is where the gate wants it.
- The reduced barrier above 22 reads as zero rate (the frozen regime). This is physically correct for the
  freezer and for slow chemistry, but it is a real ceiling: a consumer that needs a meaningful rate at
  `Pi_1 > 22` would be asking for a number below Q32.32 resolution, and the honest answer is zero at this
  precision. Confirm no named consumer needs a live rate that deep in the tail.
- `g` is surfaced two ways (per-class reference data, or the sharper barrier-from-E_coh derivation). The owner
  rules which the freezer uses. The kernel does not depend on the choice.
- The Eyring `k_B*T/h` prefactor overflows Q32.32 at SI scale and must be folded to a working frequency unit by
  the caller. Constant-Arrhenius and the freezer's `nu = c_s/a` avoid the fold. Confirm the caller-supplied-
  prefactor contract is acceptable rather than forming `k_B*T/h` inside the kernel.
- Radiogenic decay is named as a sibling but NOT a consumer (temperature-independent). Confirm it stays a
  separate first-order-decay law.

## 10. Discipline this branch holds to

Design-first: no kernel line until this design is gate-reviewed with the local-broker second opinion. The
section-9 five-lens plus adversarial verify before every push. Byte-neutral-or-stated-and-sequenced against the
five materials pins (default `40fe8a72`, full `d05a6488`, discovery `9a28f113`, viability `967b22bd`, living
`be94e310`) and the run_world pins. `cargo fmt --check` before every push. Never fabricate a value (surface it
reserved-with-basis or classify it as cited reference data). Derive over author (Section 6 is the derive-first
check the gate directed). Admit the alien as data (`g` and every barrier key off the being's or material's own
data, so an alien chemistry or a crystalline mind is a data row). And prove it before trusting it, most of all
when the conclusion is this session's own.
