# Pinning the Unit System: Scoped Proposal for R-UNITS-PIN (owner-directed, two arcs)

The owner directed the R-UNITS-PIN resolution built in two arcs, Phase 1 the absolute
canonical layer and Phase 2 the emic layer, taking whichever option is best for the long
haul even where it is more work now. This document is the design-of-record that fixes the
architecture before the code touches the numeric foundation, in the same convention as the
mechanical, fluids, and electromagnetism substrate proposals.

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
backing), used pervasively (429 sites in `laws.rs` alone). Nothing maps a physics axis to a
`QuantityDef`, nothing derives a per-quantity scale, and nothing lets a kernel read or write
at a scale other than Q32.32. The one place a non-Q32.32 scale is handled today is the Coulomb
kernel, by hand: `k_coulomb` is passed on a reserved times-ten-to-the-nine output scale and
the kernel does the alignment shifts inline (`laws.rs`, `coulomb_force`). Generalizing that
one ad-hoc trick into a declared per-quantity scale is the heart of Phase 1.

## The seam auditing the input surfaced: uniform Q32.32 cannot hold the wide-range quantities

Auditing the premise rather than the flag alone: the reason the unit system needs per-quantity
scales at all, and the reason Option A (one uniform scale for every quantity, choosing a base
unit per quantity so the numbers land in the Q32.32 window) is not adequate for the long haul,
is that several electromagnetic quantities span a dynamic range no single fixed-point scale
can hold with usable precision at both ends.

Electric charge is the sharp case. The floor's proposed envelope is `[1e-9, 1e5]` coulombs
(`em_floor.toml`, `elec.charge`), fourteen orders of magnitude. In Q32.32 the fractional
resolution is `2^-32` (about `2.3e-10`), so the low end `1e-9` is representable but carries
only about two bits of significance, and in the Coulomb computation the intermediate `|q|/r`
underflows the Q32.32 floor at unit separation (the note the axis already carries). No choice
of base unit fixes this under one scale: shifting the unit to lift the low end pushes the high
end `1e5` toward or past the integer ceiling. Resistivity (windowed `[1e-8, 1e3]`),
capacitance (`[1e-12, 1e3]`), magnetic flux density (`[1e-5, 50]`), and several others carry
the same shape, and optics already flagged it too: `opt.source_power` keeps a per-quantity
scale reserved for its full envelope, the set Q32.32 range being only the representable
working subset.

A per-quantity scale holds these cleanly. Charge at Q17.46 (seventeen integer bits, so `1e5`
fits with headroom, and forty-six fractional bits) represents `1e-9` with about five
significant figures while still holding `1e5`. This is the concrete reason Option B
(per-quantity `scale_bits`, which the `units` crate already carries) is the long-haul design
and Option A is not: the wide-range quantities are unrepresentable under any single scale, and
Option B also directly unblocks the electromagnetism ranges, which have stayed reserved on
exactly this pin.

## Phase 1 architecture (Option B, per-quantity scale_bits)

The design has four moving parts, ordered so each lands as its own red-teamed, CI-green,
merged increment.

### 1. The canonical physics quantity catalogue

The `units` `QuantityRegistry` becomes the canonical catalogue of the physics quantities: one
`QuantityDef` per physics axis (and per derived law-output quantity that is not itself an
axis), carrying the axis's dimension, its `scale_bits`, and its overflow policy. The catalogue
is built from the floor data (the `*_floor.toml` axis set) rather than hardcoded, so it grows
with the world and stays a registry sibling to the value, semantic, and institution-function
substrates. The base-dimension membership is the five bases the floors already use (length,
mass, time, temperature, current), registered in a fixed canonical order.

The `scale_bits` for a quantity is *derived from its declared range by a fixed rule*, not
authored per quantity: with `i64` backing there is one sign bit and sixty-three magnitude
bits, so `integer_bits = ceil(log2(max(|range_lo|, |range_hi|))) + guard` and `scale_bits =
63 - integer_bits`, clamped to `[0, 62]`. The owner sets the physical envelope (the range,
already surfaced reserved-with-basis in the floor data); the scale follows by the rule. This
keeps Principle 11 clean: the derivation is fixed Rust, the envelope is data, and the crate
authors no scale. Quantities whose envelope fits Q32.32 with usable precision (the fluids,
chemistry, and mechanical axes, ranges within roughly `[1e-3, 1e6]`) derive `scale_bits = 32`,
so their `Fixed` representation and their `AbsoluteQuantity` representation coincide and
nothing about them changes. Only the wide-range quantities derive a different scale.

The bridge between `Fixed` and `AbsoluteQuantity` is a pair of total conversions: a `Fixed`
(always Q32.32) maps to an `AbsoluteQuantity` at a quantity's `scale_bits` by the same checked
rescale `checked_convert` already implements (left shift up, round-half-even divide down, i128
intermediate, `None` on out-of-range), and back. For a `scale_bits = 32` quantity the bridge
is the identity on the raw bits.

### 2. Scale-aware arithmetic, bounded to the wide-range kernels

A kernel computing a monomial (a product of powers of its input axes, which the physics graph
already declares per law as its `OutputCheck::Monomial` with the port axes and exponents) has
a determined output scale: the net fractional-bit exponent is the sum over inputs of
`scale_bits * exponent`, and the kernel then rescales that raw result to the declared output
quantity's `scale_bits`. This is exactly what the Coulomb kernel does by hand with its fixed
shifts; the generalization is a small set of raw-bit helpers (align two operands to a working
exponent, track the exponent through a checked multiply or divide, rescale to a target
`scale_bits` with round-half-even), which the wide-range kernels call in place of the inline
`<< 32` and `>> 32`. The helpers live in the physics crate and operate on i128 raw bits, the
totality discipline the kernels already use (every overflow and zero divisor routes to the
physical extreme).

This work is bounded to the electromagnetism kernels and the one optics kernel
(`inverse_square_falloff`, reading `opt.source_power`). The fluids, chemistry, and mechanical
kernels keep `scale_bits = 32` throughout and are not touched, so the change does not rewrite
the roughly sixty-three kernels: it touches the ten or so that read or write a wide-range
quantity. The kernels stay bespoke closed-form integer functions; they become
scale-parameterized rather than scale-hardcoded.

### 3. The electromagnetism ranges, unblocked

With the scale derived from the envelope, the fourteen electromagnetism axes graduate from
`range_reserved` to set `range_lo`/`range_hi`, the same graduation the fluids and chemistry
floors already took (`em_floor.toml` today carries every axis as a proposed window with a
cited basis, awaiting exactly this pin). The set range is the owner's envelope; the
`scale_bits` is its derived consequence. This is the concrete payoff that makes Phase 1 worth
its cost: the electromagnetism floor becomes owner-set data rather than a reserved sketch.

### 4. The constants reconciled onto the pinned scales

The reserved electromagnetism constants are re-expressed on the pinned scales rather than the
ad-hoc ones. `k_coulomb` is today passed on a reserved times-ten-to-the-nine output scale that
absorbs the charge-scale mismatch; once charge carries a declared `scale_bits`, the coefficient
is expressed against that scale and the ad-hoc factor is retired. `MU_0` (vacuum permeability),
the induction tick duration `DT`, and the caps `F_MAX`, `V_MAX`, `I_MAX`, `B_MAX`, `PHI_MAX`
are each reconciled to the pinned scales. Each stays the caller's reserved value passed in,
never fabricated inline, and each is surfaced with its basis for the owner to set.

## Determinism (held under review, the load-bearing constraint)

R-UNITS-PIN is a determinism-cluster item, so the bar is bit-reproducibility, not plausibility.
The design holds it three ways. The scale derivation is a pure integer function of declared
data, so it is identical on every machine and every run. The scale-aware helpers are integer
i128 arithmetic with a fixed rounding rule (round-half-to-even, the rule the canonical
quantizer and `checked_convert` already use), so no float enters and no ordering ambiguity
arises. The accumulation paths use the crate's order-independent `sum` (exact i128 total,
policy applied once at read), which is the clamp-at-read discipline the evidence engine and the
Part 57 order-independent reductions already rely on, so a fold of magnitudes is independent of
arrival order even under saturation. Every wide-range kernel change is proven bit-identical to
its current output where the derived scale is 32, and re-proven against a recomputed oracle
where the scale differs, in the physics determinism harness.

## Reserved values, surfaced with basis (nothing fabricated)

Phase 1 surfaces these for the owner to set; none is invented here.

- **The per-quantity envelopes for the fourteen electromagnetism axes** (charge, current,
  potential, emf, resistance, resistivity, capacitance, electric field, flux density, flux,
  permeability, magnetic moment, inductance, coupling coefficient). Basis: the cited datasheet
  and theory bounds each axis already carries in its `real` field and its proposed window
  (CRC, Griffiths, Kittel, Horowitz and Hill, Fink and Beaty, and the others named per axis).
  The owner confirms the window; the `scale_bits` follows by the derivation rule.
- **The scale-derivation guard bits** (how many integer bits above the envelope top the rule
  reserves before assigning the rest to the fraction). Basis: the headroom a law's product of
  in-range inputs can reach before the output cap catches it, a representability bound rather
  than a physical one; a small fixed guard (one or two bits) is the natural default and is the
  owner's to set.
- **The reconciled electromagnetism constants** (`k_coulomb` on the pinned charge scale, `MU_0`,
  the induction `DT`, and the caps `F_MAX`, `V_MAX`, `I_MAX`, `B_MAX`, `PHI_MAX`). Basis: the
  physical constant expressed against the now-pinned scales, and each cap the declared range's
  top routed through the law, a derivation from the envelope and the scale rather than a free
  choice.
- **The overflow policy per quantity** (saturate or wrap). Basis: whether the quantity is a
  gate-thresholded magnitude (saturate, the default, so a runaway reads as the bound) or a
  modular accumulator whose wrap is intended (wrap). Every physics quantity is expected to
  saturate; the field is surfaced so the choice is declared rather than assumed.
- **The `opt.source_power` full-envelope scale** (the optics axis that already reserves a
  per-quantity scale for its sub-microwatt low end). Basis: the same envelope-to-scale
  derivation as the electromagnetism axes.

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
factor held exactly rather than as a single rounded multiply), and Phase 1's `checked_convert`
already establishes the round-half-even rescale it will build on. The sorted unit store is a
Phase 2 container keyed by a stable id, the canonical-walk discipline (R-CANON-WALK) the engine
already applies elsewhere. Phase 2 couples to Parts 9 and 40 (the culture and belief layers)
and is the second arc; it is not built here.

## Honest limits (surfaced, not hidden)

- Phase 1 pins the absolute layer only. The emic round-trip loss and the sorted unit store, the
  two defects the flag names by number, are Phase 2 and remain open until that arc lands.
- The scale-derivation rule assigns a scale from the declared envelope. A quantity whose true
  physical range exceeds what sixty-three magnitude bits can hold at usable precision (the
  conductor-to-insulator resistivity envelope spans about twenty-four orders, the axis is
  windowed to fit) is represented over a windowed sub-range, with the extreme tail clamped or
  reserved, the same honest windowing the floor data already documents. Per-quantity scales
  widen the representable window; they do not make it unbounded.
- Scale-aware arithmetic is bounded to the wide-range kernels by design. A future domain that
  introduces a new wide-range quantity inherits the mechanism but must have its kernel made
  scale-aware, the deliberate audited extension rather than a silent one.
- The derived scale is a representability choice, not a physical claim. Two quantities of the
  same dimension at different scales convert through the checked rescale, which rounds; a
  conversion that does not divide evenly loses up to one unit at the target scale, reported
  through the `checked` contract rather than hidden.

## The increment plan (each a red-teamed, CI-green, merged PR)

1. **PR-0: this design-of-record.** Lands the architecture as the arc's vehicle (docs only).
2. **PR-1: the canonical quantity catalogue and the Fixed bridge.** Build the catalogue from
   the floor axis set, the scale-derivation rule, and the total `Fixed`-to-`AbsoluteQuantity`
   bridge, with the fluids/chem/mechanical quantities deriving `scale_bits = 32` and proven
   unchanged. Mechanism, no behavior change to any kernel.
3. **PR-2: scale-aware electromagnetism kernels.** Generalize the Coulomb inline scaling into
   the raw-bit helpers, and route the wide-range electromagnetism and `source_power` kernels
   through them, proven bit-identical where the scale is 32 and against a recomputed oracle
   where it differs.
4. **PR-3: set the electromagnetism ranges and reconcile the constants.** Graduate the fourteen
   axes from `range_reserved` to set, reconcile `k_coulomb`, `MU_0`, `DT`, and the caps onto the
   pinned scales, and surface the reserved decisions for owner ratification.

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
variables), and the audit's determinism cluster (R-UNITS-PIN as one of the seven, roadmap Tier
B item 4). The electromagnetism floor's `R-UNITS-PIN` scale-pending notes and the optics
`opt.source_power` note reconcile to set on PR-3.

## Citations

The physical envelopes and their bases are the citations the floor axes already carry per
axis: CRC Handbook of Chemistry and Physics (charge, resistivity, capacitance, dielectric
strength, magnetic properties, refractive index, emissivity); Griffiths, Introduction to
Electrodynamics (charge, current, potential, resistance, capacitance, flux, magnetic moment);
Kittel, Introduction to Solid State Physics (resistivity, permeability); Horowitz and Hill, The
Art of Electronics (current, capacitance, inductance, coupling); Fink and Beaty, Standard
Handbook for Electrical Engineers (flux, inductance, coupling); Jiles, Magnetism and Magnetic
Materials (permeability); Purcell and Morin, Electricity and Magnetism (flux density); NIST
CODATA (the constants). The fixed-point discipline is design Parts 3 and 55; the substrate
representation is Part 58; the order-independent reduction discipline is Part 57
(R-REDUCE-ORDER); the canonical-walk discipline is Part 3.5 (R-CANON-WALK).
