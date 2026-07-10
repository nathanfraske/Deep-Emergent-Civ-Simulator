# Biosphere-balance substrate map: the small-body-starvation bedrock study (Agent B)

This is the read-only bedrock study the gate commissioned while Agent B was owner-blocked on both build-arcs: why do creatures starve at small body scale, and is the honest target a build, the owner-gated T3 real-per-plant food wire, or pure calibration? It is the biosphere sibling of the affordance and perception studies. No code is built here.

## How it ran

The construction was gated by the section-11 input-bias smoke test (fail-closed, Opus at maximum reasoning) BEFORE the mappers ran. It blocked three times, each a real fix: the first construction never sent a mapper to COMPUTE whether small bodies are viable at all, so the "the metabolic scale is the bedrock" conclusion was unreachable; the second fused the two levers (calibrate the food scalar versus arm the T3 wire) into one ceiling, so the no-wire null was asserted rather than computed; the corrected construction computes two separate ceilings and cleared. Then five mappers (drain, intake, balance path, owner-gate, and the computed counterfactual), each adversarially verified against source, then a synthesis and a completeness critic that had to confirm the no-wire ceiling was a computed number. Every claim carries a file:line, and the decisive crux was re-verified from source by hand.

## The layer map

- **M1, the metabolic drain and reserves.** Built, wired, floor-correct. A small body drains a larger FRACTION of its reserve per tick and holds a smaller absolute reserve, and both are physics-of-scale, not authored defects: basal power scales `m^(3/4)` (`laws.rs:1642-1665`, an authored floor exponent) and resting heat loss scales as surface `~m^(2/3)`, while absolute stored energy scales `~m^1` (`reserve_mass = energy_capacity * mass_kg`, `physiology.rs:471`). The knee placement is the reserved calibration scalars (`kleiber_a`, `body_mass_kg_scale`, the energy-density units); the steepening shape is bedrock floor. M1 alone cannot decide the outcome, because viability is the drain/intake RATIO.
- **M2, the food intake.** The current live food value is a default scalar: `content = supply * food_energy_density` (`locomotion.rs:1225`), with `food_energy_density` and `ingest_efficiency` reserved. The T3 per-plant-composition path is built but owner-gated off.
- **M3, the balance path.** Each tick the reserve updates by intake minus drain, and death is the emergent reserve-through-floor cull ("when any axis falls through its floor the body dies", `homeostasis.rs:30,239`), not an authored event.
- **M4, the owner-gate.** The T3 wire is built but owner-gated off (`worldbuild.rs:450-457`, the arm call `environ.set_producer_food`). The three candidate targets (pure calibration, arm the wire, scale-bedrock) are left for M5's computation to decide.

## The computed counterfactual (M5), the crux, verified by hand

M5 computed, as actual numbers with file:line, whether a small body is viable, where viability is the per-tick FLOW balance (best-case intake versus drain), with the mass-derived reserve treated as a buffer, not added to the flow.

- Drain at `m = 0.1 kg`: `basal = a * m^(3/4) = 3.4 * 0.1^0.75 = 0.60 W`, thermoneutral (`laws.rs:1650`, `a = 3.4` from `reserved.toml:1540`); per-tick spend `0.60 J` at `tick = 1 s`.
- C1 (no wire): the default-scalar intake ceiling, `food_energy_density` and `ingest_efficiency` at their upper bounds, T3 not armed (`environ.rs:315`). Mass-cancelled ceiling `supply * fed * assim * eta = 1 * 3000 * 1 * 0.5 = 1500` (up to 3000 at the physical bound).
- C2 (T3 armed): `set_producer_food` seeds a normalized sum-to-one simplex (`environ.rs:848-858`), and intake still multiplies by the global scalar unconditionally (`locomotion.rs:1225`), so `C2 = C1 * energy_axis_density <= C1`. Arming the wire cannot raise the ceiling; it only makes food composition-realistic, which lowers energy per biomass.

**The crux, re-verified from source by hand:** the being's mass CANCELS. The drain-fraction denominator is `energy_capacity * mass_kg * energy_density` (`physiology.rs:471` into `laws.rs:1697`) and the intake-fraction denominator is `body_mass * storage_density` (`physiology.rs:503-505`), so the per-tick flow balance reduces to the mass-free inequality `supply * fed * assim * eta >= (basal + heat_loss) * tick`, satisfied by roughly 2500x at every small mass. C1 CLOSES.

## Terminal finding: PURE CALIBRATION (no build, no wire)

- **(c) metabolic-scale-bedrock: refuted.** The `m^(3/4)` exponent is a floor law, but it does not make small bodies unviable in flow, because the mass cancels. The only real small-body effect is the shrinking BUFFER (mass-specific drain `~m^(-1/4)`, absolute reserve `~m^1`), a shorter time-to-death between meals, which is buffer, not flow viability.
- **(b) arm the T3 wire: refuted as the fix.** `C2 <= C1`; arming it as-is worsens grazer survival (source records a collapse from about 25 to about 1, `worldbuild.rs:454`), because the absolute joule scale stays the unconditional scalar.
- **(a) pure calibration: confirmed.** The knee is where the owner sets the reserved food and metabolism scalars.

The honest target is owner calibration of the reserved scalars, not a substrate and not the wire.

## The doc-versus-code seam (Prime Directive 2, verified by hand)

`locomotion.rs:1223-1224`'s comment, `OWNER_DECISIONS_LOG.md:202-204`, and `worldbuild.rs:445` claim a real per-plant value SUPERSEDES the scalar once T3 wires the food composition. The code at `locomotion.rs:1225` keeps `content = supply * food_energy_density` UNCONDITIONAL, with no supersession branch. So arming T3 does not replace the scalar; the supersede claim is aspirational, not implemented, and it is why arming T3 lowers rather than raises the ceiling. Reconcile: either correct the aspirational comment, or add the supersede branch when T3 wires.

## The owner-blockers (routed to the register)

- `food_energy_density`: reserved R-UNITS-PIN units anchor (`locomotion.rs:139/194`, dev `3000`), absent from the manifest. The owner promotes and sets it; basis: a foraging being's intake offsets its Kleiber drain.
- `ingest_efficiency`: reserved trophic-transfer efficiency (`locomotion.rs:129/189`, dev `1/2`), absent from the manifest. The owner sets it; basis: Lindeman trophic transfer.
- The T3-arming decision: a genuine design-intent call, owner-gated, coupled to two NEW reserved values the owner must first set (a per-axis food-value conversion and per-axis assimilation), and it worsens grazer survival as-is. Not a fix for the starvation symptom.
- `kleiber_coefficient = 3.4` and `body_mass_kg_scale = 70` are already set (`reserved.toml:1537/1547`, Arc 2), pending final owner sign-off; they place the buffer knee.

## Honest limits carried forward

The verdict (pure calibration, mass cancels) is robust to roughly 2500x, but three items keep the operational picture one layer short and are disclosed: M5 computed the CEILING (`supply = 1`, the productivity cap) rather than the realized steady-state supply from regrow, deplete, and colonize dynamics; the reserve-density cancellation is O(1)-approximate rather than identical (drain reads the organs-only `whole_body_energy_density`, intake reads the whole-body composition vector); and the cold-medium heat loss was bounded, not computed from a modelled body's surface and its cell's medium. All are numerically inert against the margin, but the realized-supply dynamics is the layer to compute if the biosphere still fails to thrive after the owner sets the two scalars.
