# Pinning the Unit System: Scoped Proposal for R-UNITS-PIN (owner-directed, two arcs)

The owner directed the R-UNITS-PIN resolution built in two arcs, Phase 1 the absolute
canonical layer and Phase 2 the emic layer, taking whichever option is best for the long
haul even where it is more work now. This document is the design-of-record that fixes the
architecture before the code touches the numeric foundation, in the same convention as the
mechanical, fluids, and electromagnetism substrate proposals. It has been revised after an
adversarial red-team of its first draft, whose findings are folded into the scope, the
derivation rule, the determinism rule, and the honest limits below.

## The directive it answers

Part 55 states the rules of the unit system but pins none of them: each quantity is to be
assigned a fixed-point scale chosen for its real range, conversions between scales are to be
explicit, and saturate-versus-wrap on overflow is to be a defined per-quantity choice rather
than an accident, and the emic conversion round trip is lossy by up to one epsilon for a
non-power-of-two factor. The flag (design.md, the R-UNITS-PIN blockquote at Part 55) asks a
session to build the `units` crate with a per-quantity scale registry, a single canonical
storage direction, an explicit per-quantity saturate-or-wrap policy wired into the
accumulation paths, a round-trip exactness rule, and a sorted unit store. The item couples to
Parts 3, 9, and 40, and it is one of the seven open determinism-cluster items (audit Section
3, roadmap Tier B item 4).

The owner's standing conditions on any resolution hold here: the result must be broadly
generalizable, per-quantity differentiable, and data-driven, with the mechanism fixed Rust
and the membership data (Principle 11), and no value fabricated (every reserved number
surfaced with its basis).

## Why two layers, and why this order

Part 55 is built on two layers, and keeping them apart is the whole design. The first is the
absolute canonical unit system, the engine's ground truth: physics is authored and computed
in one fixed set of base units and their derived units, each in fixed-point at a chosen
scale, and this layer is the one bounded place Principle 9 lets our physics bias enter. The
second is the emic layer, the measurement systems and the physical understanding the cultures
invent for themselves, which the engine holds as conversion factors and provenance relative
to the absolute system the culture never sees, and which drift, spread, and can be lost like
any belief (Part 9).

Phase 1 pins the absolute layer, because everything computes against it and nothing in the
emic layer is well-defined until the absolute scales are fixed. Phase 2 builds the emic layer
on top. This proposal specifies Phase 1 in full and scopes Phase 2 at the level needed to
confirm Phase 1 does not foreclose it.

## The locked mechanism it builds on

The `units` crate already carries the Phase 1 *mechanism* and ships no membership
(`crates/units/src/lib.rs`, design Part 59). It holds a `BaseDimensionRegistry` (base
dimensions are data, the owner's authored physics catalogue under Principle 9); a canonical
sorted `Dimension` of integer base-exponent terms so a derived dimension is a computed
composition rather than an authored entry; a `QuantityDef { name, dimension, scale_bits,
overflow }` and a `QuantityRegistry` indexed by id and name; an `AbsoluteQuantity { quantity,
bits: i64 }` magnitude at the quantity's scale; an order-independent `sum` that accumulates in
i128 and applies the overflow policy once at read; and a `checked_convert` that rescales
between two quantities of the same dimension with round-half-to-even in i128, returning `None`
on a dimension mismatch or an out-of-range result. The crate's own doc is explicit that it
carries no base dimension, quantity, or scale, and that its tests use a fixture catalogue, not
the authored set.

What does not yet exist is the bridge from that mechanism to the running physics. The physics
crate computes in `civsim_core::Fixed`, a Q32.32 fixed-point (32 fractional bits, i64
backing, one sign bit and sixty-three magnitude bits, integer ceiling about `2.1e9`, and a
fractional floor `2^-32` about `2.3e-10`), used pervasively (429 sites in `laws.rs` alone).
Nothing maps a physics axis to a `QuantityDef`, nothing derives a per-quantity scale, and
nothing lets a kernel read or write at a scale other than Q32.32. The one place a non-Q32.32
scale is handled today is the Coulomb kernel, by hand: `k_coulomb` is passed on a reserved
times-ten-to-the-nine output scale and the kernel does the alignment shifts inline (`laws.rs`,
`coulomb_force`). Generalizing that one ad-hoc trick into a declared per-quantity scale is the
heart of Phase 1.

## Why per-quantity scale_bits (Option B) over a per-quantity base unit (Option A)

Both options can *represent* a wide-range quantity: a per-quantity base unit at the uniform
Q32.32 scale and a per-quantity scale at the SI base unit are the same bits reached two ways
(choosing a charge base unit of `2^-14` coulomb at scale 32 is arithmetically the Q17.46 scale
at the coulomb). The red-team was right to reject a representability argument between them: the
case for Option B is not that Option A cannot hold the numbers.

The case for Option B is that it keeps the SI base units intact and makes the scale an
explicit, declared, per-quantity reserved number, where Option A smuggles the scale into the
choice of base unit. Under Option A a "coulomb" silently becomes some non-SI count of internal
charge units, so the authored physics constants (`k_coulomb`), the cross-quantity compositions
(a charge times a potential is an energy), and the emic conversions of Phase 2 all have to
carry and reconcile a hidden unit factor per quantity. Under Option B every quantity stays in
its SI base unit, the scale is one declared integer the loader reads, the conversions between
scales are the explicit rescales Part 55 asks for, and the composition of two quantities
tracks their declared scales by a clean integer sum of fractional bits. Option B is the
long-haul design because the scale is data in the open rather than a factor hidden in a unit
choice, which is exactly the Principle-11 discipline the rest of the substrate holds to.

The genuine representability limit is separate and stands: charge at `[1e-9, 1e5]` (fourteen
orders) or capacitance at `[1e-12, 1e3]` cannot be held at usable low-end precision at the
*uniform* Q32.32 scale (charge's `1e-9` lands at about two significant bits, capacitance's
`1e-12` underflows the `2^-32` fractional floor to zero), so those quantities need a scale
other than 32 under either option. Option B is where that scale lives as a declared number.

## The seam auditing the input surfaced: the scale-reserved set spans six domains, not EM alone

Auditing the premise rather than the flag alone, and correcting the first draft's scope error:
the axes that need a scale other than the canonical 32 are not the electromagnetism axes as a
block. Seven of the fourteen electromagnetism axes carry an explicit per-quantity-scale
reservation (`elec.charge`, `elec.current`, `elec.resistance`, `elec.resistivity`,
`elec.capacitance`, `mag.magnetic_moment`, `mag.inductance`); the other seven
(`elec.potential`, `elec.emf`, `elec.electric_field`, `mag.flux_density`, `mag.flux`,
`mag.permeability`, `mag.coupling_coefficient`) fit one scale and carry no scale reservation,
though all fourteen carry a range reservation awaiting the owner's set. And several
scale-reserved axes live outside electromagnetism entirely, each flagged R-UNITS-PIN at its
site:

- `opt.source_power` (`chem_optics_floor.toml`), the sub-microwatt low end.
- `acoustic.source_power` (`fluids_floor.toml`), the same sub-microwatt shape.
- `fluid.dynamic_viscosity` (`fluids_floor.toml`), the gas-to-glass envelope spanning about
  sixteen orders.
- `fluid.channel_radius` (`fluids_floor.toml`), the capillary-to-aorta radius that underflows
  the Poiseuille `r^4`.
- `mech.second_moment_of_area` (`mechanical_floor.toml`), where a small section underflows
  `m^4`.
- `bio.consumer.reference_tolerance` (`biology_floor.toml`), whose picogram-per-kilogram to
  gram-per-kilogram envelope exceeds one Q32.32 scale, and which reserves its scale *per toxin
  class* (a case the section on per-class scales below addresses), with the sibling
  `bio.consumer.requirement` trace-class deficiency floor and `bio.respiratory_surface`
  carrying the same shape.

Phase 1 pinning the absolute layer therefore covers this whole scale-reserved set across the
six domains, not the electromagnetism axes alone. This is the wider, more-work scope the owner
signed up for, and it is the correct one: leaving a scale-reserved axis in another floor
unpinned would leave R-UNITS-PIN half-resolved.

## Phase 1 architecture (Option B, per-quantity scale_bits)

The design has four moving parts.

### 1. The canonical physics quantity catalogue, with the scale defaulting to 32

The `units` `QuantityRegistry` becomes the canonical catalogue of the physics quantities: one
`QuantityDef` per physics axis (and per derived law-output quantity that is not itself an
axis), carrying the axis's dimension, its `scale_bits`, and its overflow policy. The catalogue
is built from the floor data (the `*_floor.toml` axis set) rather than hardcoded, so it grows
with the world and stays a registry sibling to the value, semantic, and institution-function
substrates. The base-dimension membership is the five bases the floors already use (length,
mass, time, temperature, current), registered in a fixed canonical order.

The `scale_bits` for a quantity defaults to 32, the canonical scale coincident with `Fixed`,
and deviates only when the declared envelope cannot be held there. The rule, correcting the
first draft's maximize-the-fraction error: let `lo` be the smallest nonzero magnitude of
interest in the range and `hi` the largest, let `P` be the required low-end significance in
bits, and let `guard` be the integer headroom bits above the top. The quantity keeps
`scale_bits = 32` when the top fits (`ceil(log2(hi)) + guard <= 31`) and thirty-two fractional
bits already resolve the bottom to `P` significant bits (`floor(log2(lo)) + 32 >= P`).
Otherwise the scale is derived to hold both ends: `integer_bits = ceil(log2(hi)) + guard`,
`frac_bits` set so `floor(log2(lo)) + frac_bits >= P`, and `scale_bits = frac_bits` subject to
`integer_bits + frac_bits <= 63`. That budget bites in practice, and it does so two ways. Charge
`[1e-9, 1e5]` fits it exactly (Q17.46, seventeen integer plus forty-six fractional, at `P = 16`
and `guard = 0`), but capacitance `[1e-12, 1e3]` at `P = 16` needs ten integer plus fifty-six
fractional bits, sixty-six in all, so the budget forces a reduced significance target
(capacitance's `P` caps near thirteen, a Q10.53) rather than the full sixteen the charge
example uses: the low-end significance a wide envelope can carry is itself bounded by the
budget, quantity by quantity. When even a reduced `P` down to a floor cannot hold both ends
(the conductor-to-insulator resistivity envelope spans about twenty-four orders and exceeds
sixty-three magnitude bits outright), the envelope is windowed to a representable sub-range with
the tail clamped or reserved, the honest windowing the floor data already documents. So the
budget resolves in three stages: full `P` where it fits (charge), a reduced `P` where the
envelope is wide but bounded (capacitance), and a windowed envelope where even the reduced `P`
cannot span it (resistivity, dynamic viscosity). Under this rule the bulk of the substrate
(the fluids, chemistry, mechanical, and the seven one-scale electromagnetism axes, whose tops
fit `2^31` and whose bottoms resolve at scale 32) derives `scale_bits = 32` and is unchanged:
their `Fixed` representation and their `AbsoluteQuantity` representation coincide. Only the
scale-reserved set derives a different scale. The owner sets the physical envelope and the
significance target; the scale follows by the rule, so the crate authors no scale (Principle
11).

The bridge between `Fixed` and `AbsoluteQuantity` is a pair of total conversions in the
canonical rounding discipline (below): a `Fixed` (always Q32.32) maps to an `AbsoluteQuantity`
at a quantity's `scale_bits` by a checked rescale (left shift up, truncating shift down, i128
intermediate, `None` on out-of-range), and back. For a `scale_bits = 32` quantity the bridge
is the identity on the raw bits.

### 2. Scale-aware arithmetic over the declared port scales

A kernel computes a consequence from its input axes whose scales are now declared. The physics
graph already declares each law's port exponents on its `PortContract` (present on every law,
including the ones whose output check is `Asserted` rather than `Monomial`), so the output
scale of a product-of-powers kernel is a determined integer: the net fractional-bit exponent
is the sum over inputs of `scale_bits * exponent`, plus the `scale_bits` any dimensional
constant carries (this is the `k_coulomb` term the first draft's formula omitted), and the
kernel then rescales that raw result to the declared output quantity's `scale_bits`. Four of
the fifteen electromagnetism kernels are `Asserted` because a dimensional constant or a
finite-difference rate sits outside the port monomial (`coulomb_force`, `solenoid_field`,
`faraday_emf`, `inductive_emf`, confirmed in `graph.rs`); the scale tracking still works for
them off the declared port exponents and the constant's declared scale, so the `Asserted`
output check is about dimensional homogeneity, not about whether the scale is derivable.

The generalization of the Coulomb kernel is therefore more than a find-and-replace, and the
red-team was right to flag it: the kernel's inline `<< 32` and `>> 32` hardcode the assumption
that both operands are Q32.32, which fails when charge is (say) Q17.46 and the separation is
Q32.32. The helpers replace the fixed shifts with shifts computed from the operands' declared
scales and the target output scale (for `|q| / r` the alignment shift is a function of the
charge scale, the radius scale, and the working scale), tracking the fractional-bit exponent
through each checked multiply and divide and rescaling once to the output scale at the end. The
helpers live in the physics crate and operate on i128 raw bits, the totality discipline the
kernels already use (every overflow and zero divisor routes to the physical extreme).

This work follows the wide-range quantity set, not a domain boundary, and it is larger than
the first draft claimed: it touches every kernel that reads or writes a scale-reserved axis,
which spans electromagnetism (`coulomb_force`, `ohm_voltage`, `circuit_current`,
`power_dissipation`, the resistance and capacitance and inductance kernels), fluids
(`reynolds_number`, `poiseuille_flow` reading viscosity and channel radius, `laplace_pressure`
reading channel radius), mechanics (`euler_buckle` reading the second moment of area), optics
(`inverse_square_falloff` reading source power), acoustics (the source-power kernel), and
biology (the kernels reading the consumer tolerance and respiratory surface). The kernels that
read only `scale_bits = 32` quantities are untouched and stay bit-identical; the blast radius
is the wide-range readers, a bounded but cross-domain set rather than an electromagnetism-only
one.

### 3. The ranges set and the scales pinned

Two owner-set actions land here, kept distinct. The range graduation sets the fourteen
electromagnetism axes (and any other `range_reserved` axis in scope) from `range_reserved` to
`range_lo`/`range_hi`, the same graduation the fluids and chemistry floors already took; the
set range is the owner's declared envelope. The scale pinning then derives and records the
`scale_bits` for each scale-reserved axis from its now-set envelope and significance target by
the rule above. Setting the envelope is the prerequisite for deriving the scale, so the two
run together, and the electromagnetism floor becomes owner-set data rather than a reserved
sketch, the concrete payoff of the arc.

### 4. The constants reconciled onto the pinned scales

The reserved constants are re-expressed on the pinned scales rather than the ad-hoc ones.
`k_coulomb` is today passed on a reserved times-ten-to-the-nine output scale that absorbs the
charge-scale mismatch; once charge carries a declared `scale_bits`, the coefficient is
expressed against that scale (and enters the output-scale sum as the dimensional constant's own
`scale_bits`) and the ad-hoc factor is retired. `MU_0` (vacuum permeability), the induction
tick duration `DT`, and the caps `F_MAX`, `V_MAX`, `I_MAX`, `B_MAX`, `PHI_MAX` are each
reconciled to the pinned scales. Each stays the caller's reserved value passed in, never
fabricated inline, and each is surfaced with its basis for the owner to set.

## Per-class scales (the `bio.consumer.reference_tolerance` case)

One scale-reserved axis reserves its scale *per toxin class*, and a single `scale_bits` per
`QuantityDef` cannot express a scale that varies by class. The resolution respects the
data-driven substrate rather than widening the struct: where the physics of an axis truly
demands a different scale per class, the class is the quantity granularity, so the catalogue
registers one `QuantityDef` per class (a tolerance-for-class-X quantity, a tolerance-for-class-Y
quantity), the membership being data that grows with the world exactly as the rest of the
registry does (Principle 11). The mechanism stays one scale per `QuantityDef`; the world grows
the set of quantities to the granularity its physics needs. This keeps the toxin-class set
open and emergent rather than authoring a closed enum of classes into the type. The same holds
for the `bio.consumer.requirement` trace-class deficiency floor.

## Determinism (held under review, the load-bearing constraint)

R-UNITS-PIN is a determinism-cluster item, so the bar is bit-reproducibility, and the
first draft carried a real inconsistency the red-team caught: it promised round-half-to-even
rescaling in the helpers *and* bit-identity with the current kernels, which truncate. The two
cannot both hold. The resolution follows the pinned oracle: the canonical fixed-point path uses
the same truncate-toward-zero and floor discipline the `Fixed` oracle and R-GPU-CANON-PIN
already pin (the Q32.32 multiply floors and the divide truncates, the owner's ratified
asymmetry, record 62.23), so every scale-32 kernel stays bit-identical to its current output
and every wide-range kernel is proven against a recomputed oracle in the same truncating
discipline. The `units` crate's round-half-to-even `checked_convert` is retained for the emic
conversion layer of Phase 2 (a culture's unit to the absolute system), which is a distinct,
non-oracle path and not on the bit-identity path; the canonical Phase 1 bridge and the
kernel-internal rescales truncate. Determinism holds either way (both roundings are
deterministic); bit-identity with the already-merged kernels requires the truncating one, so
that is the canonical rule.

The design holds determinism three further ways. The scale derivation is a pure integer
function of declared data, identical on every machine and run. The scale-aware helpers are
integer i128 arithmetic with a fixed rounding rule, so no float enters and no ordering
ambiguity arises. The accumulation paths use the crate's order-independent `sum` (exact i128
total, policy applied once at read), the clamp-at-read discipline the evidence engine and the
Part 57 order-independent reductions already rely on, so a fold of magnitudes is independent of
arrival order even under saturation.

## Reserved values, surfaced with basis (nothing fabricated)

Phase 1 surfaces these for the owner to set; none is invented here.

- **The per-quantity envelopes for the range-reserved axes** (the fourteen electromagnetism
  axes; `mech.second_moment_of_area`; the biology consumer tolerance and requirement axes).
  Basis: the cited datasheet and theory bounds each axis already carries in its `real` field
  and its proposed window (CRC, Griffiths, Kittel, Horowitz and Hill, Fink and Beaty, White,
  Fung, and the others named per axis). The owner confirms the window.
- **The low-end significance target `P`** (the significant bits a quantity's smallest
  meaningful value must retain). Basis: the resolution the consuming kernels need at the
  bottom of the envelope, a precision requirement the owner sets per quantity or as a substrate
  default; it trades against the guard and the top of the envelope within the sixty-three-bit
  budget.
- **The scale-derivation guard bits** (the integer headroom above the envelope top before the
  rest is assigned to the fraction). Basis: the headroom a law's product of in-range inputs can
  reach before the output cap catches it, a representability bound rather than a physical one;
  a small fixed guard is the natural default, and the worked charge example (Q17.46) corresponds
  to `guard = 0` at `P = 16`, so the guard and `P` are surfaced together against the budget.
- **The reconciled constants** (`k_coulomb` on the pinned charge scale, `MU_0`, the induction
  `DT`, and the caps `F_MAX`, `V_MAX`, `I_MAX`, `B_MAX`, `PHI_MAX`). Basis: the physical
  constant expressed against the now-pinned scales, and each cap the declared range's top
  routed through the law, a derivation from the envelope and the scale rather than a free
  choice.
- **The overflow policy per quantity** (saturate or wrap). Basis: whether the quantity is a
  gate-thresholded magnitude (saturate, the default, so a runaway reads as the bound) or a
  modular accumulator whose wrap is intended (wrap). Every physics quantity is expected to
  saturate; the field is surfaced so the choice is declared rather than assumed.

The reserved list seeds `calibration/reserved.toml` when the engine consumes it, in step with
the design record, per the manifest discipline.

## Phase 2 scoped (the emic layer, a later arc, confirmed not foreclosed)

Phase 2 builds the emic layer Part 55 specifies: a `CultureMeasurementSystem` holding a
culture's own named units, each `EmicUnit` carrying its dimension, its conversion to the
absolute system (which the engine knows and the culture does not), and its origin (a body
part, a seed, a vessel, a celestial cycle); and a `PhysicalUnderstanding` holding a culture's
theories of physics, which may be incomplete or wrong and which evolve by the same
conception-and-transmission machinery as technology and belief (Parts 9, 23, 41). The flag
names two specific requirements Phase 1 must not foreclose: a round-trip exactness rule for
conversions (the emic-to-absolute-to-emic round trip is lossy today by up to one epsilon for a
non-power-of-two factor), and a sorted unit store so any canonical walk over it is ordered.

Phase 1 leaves both open in the right way. The absolute layer is the fixed pin the emic
conversions resolve against, so pinning it first is the prerequisite. The round-trip rule is a
Phase 2 mechanism (a canonical direction plus a stored residual, or a rational conversion
factor held exactly rather than as a single rounded multiply), and the crate's round-half-even
`checked_convert` is the rescale it will build on, retained for exactly this emic path. The
sorted unit store is a Phase 2 container keyed by a stable id, the canonical-walk discipline
(R-CANON-WALK) the engine already applies elsewhere. Phase 2 couples to Parts 9 and 40 (the
culture and belief layers) and is the second arc; it is not built here.

## Honest limits (surfaced, not hidden)

- Phase 1 pins the absolute layer only. The emic round-trip loss and the sorted unit store, the
  two defects the flag names by number, are Phase 2 and remain open until that arc lands.
- The scale-derivation rule assigns a scale from the declared envelope and significance target,
  and the low-end significance a wide envelope can carry is itself bounded by the sixty-three-bit
  budget: capacitance `[1e-12, 1e3]` cannot hold the full `P = 16` the charge example uses and
  caps near thirteen significant bits at its low end. A quantity whose range exceeds what
  sixty-three magnitude bits can hold even at a reduced significance (resistivity's full
  twenty-four orders, dynamic viscosity's sixteen) is represented over a windowed sub-range with
  the tail clamped or reserved, the same honest windowing the floor data documents. Per-quantity
  scales widen the representable window; they do not make it unbounded.
- Scale-aware arithmetic is cross-domain, not electromagnetism-only: the blast radius is every
  kernel reading a scale-reserved axis, across six domains. A future domain that introduces a
  new wide-range quantity inherits the mechanism but must have its reader kernels made
  scale-aware, the deliberate audited extension rather than a silent one.
- A per-class scale is expressed by registering one quantity per class, so a quantity whose
  scale varies over a large class set grows the catalogue by that many entries. This is the
  data-driven cost of keeping the class set open rather than authoring a closed enum.
- The derived scale is a representability choice, not a physical claim. Two quantities of the
  same dimension at different scales convert through the checked rescale, which under the
  canonical truncating discipline drops the sub-scale remainder rather than rounding it,
  reported through the `checked` contract rather than hidden.

## The increment plan (each a red-teamed, CI-green, merged PR)

1. **PR-0: this design-of-record.** Lands the corrected architecture as the arc's vehicle
   (docs only).
2. **PR-1: the canonical quantity catalogue, the scale-derivation rule, and the Fixed bridge.**
   Build the catalogue from the floor axis set, the default-32 derivation rule, and the total
   truncating `Fixed`-to-`AbsoluteQuantity` bridge, with every fitting quantity deriving
   `scale_bits = 32` and proven unchanged. Mechanism, no behavior change to any kernel.
3. **PR-2: scale-aware arithmetic and the electromagnetism readers.** Build the raw-bit helpers
   (scales-in, output-scale-out, truncating) generalizing the Coulomb inline shifts, and route
   the electromagnetism wide-range kernels through them, proven bit-identical where the scale is
   32 and against a recomputed truncating oracle where it differs.
4. **PR-3: the remaining wide-range readers, the ranges, and the constants.** Route the fluids,
   mechanics, optics, acoustics, and biology wide-range kernels through the helpers; graduate
   the range-reserved axes to set; pin each scale-reserved axis's `scale_bits`; reconcile
   `k_coulomb`, `MU_0`, `DT`, and the caps; and surface the reserved decisions for owner
   ratification. Larger scale-reserved sets may split PR-3 by domain. One floor-data seam a
   value audit surfaced closes here: the `opt.source_power` and `acoustic.source_power` range
   bases name no source and no anchor magnitudes, unlike their sibling axes, so setting those two
   ranges must add a cited radiometric and acoustic source-power reference rather than restate the
   axis name.

On the arc's completion, R-UNITS-PIN's absolute half is consolidated into design.md (Part 55,
a `Decided and reserved` blockquote, a Part 62 record, a Part 63 bibliography group) and the
audit log (a Section 1 block, the Section 3 bullet rewritten toward resolved with the emic half
noted as the remaining Phase 2 work, and the counts moved), per the resolution workflow. Phase
2 (the emic layer) is the second arc.

## Cross-references to reconcile on consolidation

Part 55 (the flag and the two-layer spec), Part 58 (the physics-substrate representation, which
states every quantity is fixed-point with a declared range and a per-quantity scale), Part 41
(the authored physics layer the scales serve), Parts 3, 9, 40 (the flag's declared couplings:
the fixed-point discipline, the belief layer the emic understanding rides, the seeded start
variables), the R-GPU-CANON-PIN record 62.23 (the pinned truncating arithmetic this arc's
canonical rounding follows), and the audit's determinism cluster (R-UNITS-PIN as one of the
seven, roadmap Tier B item 4). Every floor's `R-UNITS-PIN` scale-pending note reconciles to set
on PR-3: the electromagnetism, `opt.source_power`, `acoustic.source_power`,
`fluid.dynamic_viscosity`, `fluid.channel_radius`, `mech.second_moment_of_area`, and the
biology consumer-axis notes.

## Citations

The physical envelopes and their bases are the citations the floor axes already carry per axis:
CRC Handbook of Chemistry and Physics (charge, resistivity, capacitance, dielectric strength,
magnetic properties, refractive index, emissivity, viscosity); Griffiths, Introduction to
Electrodynamics (charge, current, potential, resistance, capacitance, flux, magnetic moment);
Kittel, Introduction to Solid State Physics (resistivity, permeability); Horowitz and Hill, The
Art of Electronics (current, capacitance, inductance, coupling); Fink and Beaty, Standard
Handbook for Electrical Engineers (flux, inductance, coupling); Jiles, Magnetism and Magnetic
Materials (permeability); Purcell and Morin, Electricity and Magnetism (flux density); White,
Viscous Fluid Flow (dynamic viscosity); Fung, Biomechanics: Circulation (channel radius); NIST
CODATA (the constants). The fixed-point discipline is design Parts 3 and 55; the substrate
representation is Part 58; the pinned truncating arithmetic is Parts 3.4 and 5.4 (record
62.23); the order-independent reduction discipline is Part 57 (R-REDUCE-ORDER); the
canonical-walk discipline is Part 3.5 (R-CANON-WALK).
