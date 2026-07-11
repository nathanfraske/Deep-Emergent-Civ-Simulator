# R-UNITS-PIN residuals: design-first scope, input-audit, and slice plan (Option 1)

The gate ruled Option 1: close R-UNITS-PIN by finishing its two residuals, the emic round-trip exactness rule and the sorted unit store, design-first, with the input-audit run for a generalization seam and the slice plan brought before code. This document is that plan. No code is written until the gate signs the plan.

## Grounding: what is built, what is specified, where the residual lives

The absolute layer of Part 55 is built and hardened in `crates/units`. A `Dimension` is a canonical sorted vector of base-exponent terms over a data-driven `BaseDimensionRegistry`, not the design document's `Length | Mass | ...` enum, so the closed-dimension seam is already closed. A `QuantityRegistry` holds each quantity's dimension, its per-quantity fixed-point scale, and its saturate-or-wrap policy, and it iterates in registration-id order for any canonical purpose, so the quantity store is already ordered and deterministic. `AbsoluteQuantity` is an `i64` magnitude at a quantity's scale; `rescale_bits` and `checked_convert` convert between quantity scales by a power-of-two shift (up) or a round-half-to-even divide (down); the tier2 module carries the scale-closed `mul`/`div`/`add`/`sub` and the `WideAccum` single-round chain; the planner sizes widths and enforces the floor invariant.

The emic layer of Part 55 is specified but not built. The design document gives it as `CultureMeasurementSystem { culture, units: HashMap<UnitName, EmicUnit> }`, `EmicUnit { dimension, to_absolute: Fixed, origin: UnitOrigin }`, and `PhysicalUnderstanding { ... }`. The two open residuals both live in this layer: the lossy `to_absolute: Fixed` conversion, and the unordered `HashMap<UnitName, EmicUnit>` store. Closing R-UNITS-PIN therefore means building the emic conversion substrate in the units crate, the exact round-trip conversion and the ordered store, as the mechanism the culture-data layer (Part 40) will later populate. It does not mean wiring a full live culture measurement system into the sim, which is a later content arc; it means providing the exact, ordered, deterministic mechanism so the item's numeric-hardening ask is met.

## Residual one: the emic round-trip exactness rule

### Where the epsilon enters

A cultural unit's factor (a cubit as a fraction of the base length, a market-stone as a fraction of the base mass) is in general a non-dyadic rational: it is not a power of two times the base. Storing it as a single `Fixed` (the design document's `to_absolute: Fixed`) rounds the true factor to the nearest canonical epsilon before any conversion runs. Then a forward conversion `abs = v_emic * factor` rounds a second time, and a back conversion `v_emic' = abs / factor` rounds a third. Even with an exact factor, the composition `(v * factor) / factor` cannot be exact for a non-dyadic factor in bounded fixed point, so the round trip drifts by up to one epsilon, and repeated conversion can ratchet it. That is the defect the flag names.

### Whether the exact path derives from the scale planner or needs a new primitive

It needs a new primitive, built on the arithmetic already there. The scale planner and `rescale_bits` convert between two scales that differ by a power of two, which is the shift-or-halving case; a cultural factor is a general rational and is not that case. The exact emic conversion is a scale-closed multiply-and-divide by the factor's numerator and denominator, which is exactly what the tier2 `mul`/`div` and the `WideAccum` chain already do with a single terminal round. So the exact-rescale path reuses the tier2 arithmetic, with the planner's interval sizing used only to choose an intermediate width that cannot overflow. No new arithmetic core; a thin exact-conversion primitive over the existing one.

### The round-trip-exactness rule, stated precisely

The rule rests on the item's own first ask, a single canonical storage direction, which is absolute.

1. Storage is canonical-absolute. An emic quantity is a view, a pair of an absolute magnitude and a unit id, with no stored emic magnitude of its own. Displaying the same absolute magnitude in the same unit twice yields the same emic reading, so a display conversion is idempotent and nothing ratchets across repeated reads.
2. The factor is stored as an exact rational, an integer numerator and denominator, not as a single rounded `Fixed`, so the factor contributes no approximation of its own. This is the substrate change that lets the arithmetic be as exact as the representation allows.
3. Each direction runs through one tier2 single-round chain: forward `abs = round(v_emic * num / den)`, back `v_emic = round(abs * den / num)`, each rounded once at the end of the chain rather than at each step.
4. The exactness guarantee, honest about its boundary: the round trip `v_emic -> abs -> v_emic'` returns `v_emic'` equal to `v_emic` exactly for every emic value the absolute scale can resolve, which is all of them when the denominator divides evenly at the absolute scale's headroom and the representable subset otherwise. Where the absolute epsilon is coarser than the emic step over that magnitude, the round trip is within one absolute epsilon, a bound that is stated and tested rather than hidden. Because storage is canonical-absolute, this bounded loss never compounds.

One design choice is surfaced for the gate rather than decided here. For a culture-stated exact quantity (an inscription that reads three cubits, a quantity a person names as a whole count of a unit) the display can be made exactly reversible by carrying the stated quantity as its exact rational absolute value until it must enter the fixed-point physics path, where the single quantization is the bounded, declared loss. That exact-rational carry buys perfect emic display fidelity at the cost of a second storage form for stated quantities. The alternative is the resolvable-subset rule above with no carry, simpler and with a stated one-epsilon boundary. My recommendation is the resolvable-subset rule as the substrate default with the rational carry reserved as an opt-in for inscriptions, but this is the gate's call and I flag it, not decide it.

## Residual two: the sorted unit store

The per-culture `HashMap<UnitName, EmicUnit>` is unordered, and it is one of the containers the R-CANON-WALK flag names (the culture unit maps of Part 55). The fix is the same discipline the `QuantityRegistry` already follows: intern each `UnitName` to an ordered `UnitId` at registration and hold the units in registration-id order (or an `Ord`-keyed sorted map), with an ordered-iteration accessor, so the only walk a hash is ever built over is the sorted one. The determinism guarantee is that a canonical walk over a culture's units is id-ordered and independent of insertion order, machine, and hash seed (Principle 10). This closes the units-local instance of R-CANON-WALK; it does not close R-CANON-WALK itself, whose other containers (the registry locations, the belief stores, the market tables, the event provenance index, the pool sets) stay open and dormant until the first parallelised phase, exactly as the gate ruled.

## The input-audit: generalization seams to catch

Run before building, the derive-versus-author and alien-feasibility lenses over the emic layer as the design document sketches it.

1. `UnitOrigin` is the primary seam. The design document lists it as a body part, a seed, a vessel, or a celestial cycle. Built as a closed enum, that authors the closed list of where a unit may come from, which forecloses a culture coining a unit from anything else and forecloses the alien (a people that measures length by a mana tide's reach or a redox front's advance). It must be an open provenance reference, a datum pointing at whatever world entity or feature the unit was derived from, data-driven and extensible, so a unit's origin is a data row and the alien origin is a data row too. This is the substrate treatment already applied to the value, semantic, institution-function, and access-channel registries.
2. `to_absolute: Fixed` is a seam in two ways: it is the lossy representation the exactness rule replaces with an exact rational, and a single `Fixed` also invites a hardcoded table of allowed factors. The factor stays per-culture data, produced by the culture's unit-coining process, never a closed set the crate ships.
3. `Dimension` reuse. The emic layer must key off the built data-driven `Dimension` vector and must not reintroduce a closed dimension enum at the emic level. Confirmed as a check, not a change.
4. `UnitName` ordering must key off an interned ordered id, not a hash of the string, so the walk is observer-independent and consistent with the `QuantityRegistry` discipline.
5. No closed list of unit kinds. A unit is a triple of a dimension, a factor, and an origin, dimension-general, so there is no length-unit-versus-weight-unit taxonomy authored one level down.

## Substrate-first on values

The exactness rule authors no new value. The factor is per-culture data (a rational the unit-coining process yields), the quantity scales are the owner's reserved envelope numbers already in the registry, and the intermediate width derives from the planner's interval sizing. If a display-rounding tolerance is ever needed at the resolvable-subset boundary, it is surfaced reserved-with-basis (basis: the emic step against the absolute epsilon over the quantity's range), not fabricated. Every value is proven against the floor registry before it is set.

## The slice plan

Each slice is self-audited against the section-9 lenses by me (cost directive: no spawned panels), byte-neutral against the four canonical pins (new and unwired substrate, so byte-neutral by construction, confirmed by running the pins), and gated by the gate before the next.

1. **The exact conversion primitive.** The exact-rational factor type and the exact emic-to-absolute and absolute-to-emic conversions over the tier2 single-round chain, with the round-trip-exactness test: exact on the resolvable subset, the one-epsilon boundary stated and tested, non-compounding across repeated conversion. Optionally the rational carry for stated quantities if the gate elects it.
2. **The ordered unit store.** The interned ordered `UnitId`, the ordered store shape replacing the hash map, and the ordered-iteration accessor, with the canonical-walk determinism test (a walk is id-ordered and insertion-order-independent).
3. **The input-audit hardening.** `UnitOrigin` as an open provenance reference, the closed-list checks, and a test that a unit can be coined from an arbitrary and an alien origin as data.
4. **Consolidation to RESOLVED, on the gate's sign-off.** Add the `Decided and reserved` blockquote at Part 55, decrement the open backlog count by one (thirty-six to thirty-five), update record 62.25 to record the residuals closed (or add a closing record), reconcile the four coupling records (Parts 41, 25, and the physics domains) that read around the open residual, note in the R-CANON-WALK flag that the units container is now ordered while the item stays open for its other containers, and update the audit log Section 1, 2, and 3 and the limitation counts. Run the verification suite.

## The deferred sibling stays deferred

The sky solar-baseline sigma sibling stays deferred, to be lifted only if a future scenario surfaces it as material, exactly as the survey left it. It is out of this arc's scope.

## The ask

Gate, sign off (or correct) this plan before I write code: the exactness rule as stated and its surfaced design choice (resolvable-subset default versus the exact-rational carry for inscriptions), the ordered-store shape, the input-audit hardening of `UnitOrigin`, and the four-slice order. On your sign-off I build slice one.
