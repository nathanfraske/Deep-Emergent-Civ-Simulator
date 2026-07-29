# R-AGING (c): the first-passage damage-accumulator lifespan build plan (Agent B)

The owner-signed, gate-ruled reframed target for R-AGING. Operational lifespan is the first-passage time of a per-part cumulative damage accumulator against each part's own material tolerance, an emergent OUTPUT of independently-grounded floor physics, never an authored size-to-duration law. This plan supersedes the earlier allometric plan (`LIFESPAN_FROM_ANATOMY_PLAN.md`, option (a), rejected) and the mass-neutrality engineering (dropped: the size-longevity slope is a pure output, and suppressing it is as much a distortion as authoring it).

## The ruled target and its discipline

The discipline is NOT "no size-longevity slope", it is "no term justified SOLELY by the aging theory it encodes". Each insult stands on its own independently-grounded floor physics keyed on the being's own material; repair is a real physical maintenance process with its real extensive demand; and WHATEVER size-longevity relation emerges from that arithmetic is the correct output. The validation target: prove no term authors the slope (each floor law independently grounded, each keyed on the being's own data), never that lifespan is mass-neutral, and never that it matches a longevity curve.

## The extension points (grounded against source)

- **The accumulator.** `PartCondition` (`body.rs:355`, today `{ integrity, severed }`) gains a cumulative `damage_energy: Fixed` in the common ENERGY currency (energy dissipated into irreversible micro-damage). Integrity becomes the DERIVED normalized `1 - damage_energy / tolerance` rather than a competing store (consistent with `body.rs:31`, integrity is the derived aggregate).
- **The per-material tolerance.** `TissueMaterial.fracture_energy` (`body.rs:74`) is the part's own failure-energy tolerance, already floor data; a part dies (first-passage) when its `damage_energy` reaches it. The death rule extends `BodyPart::destroyed()` (`body.rs:426`), it does not add a competing store. For an alien whose failure mode is non-mechanical the tolerance is whatever failure-energy its material declares, read from the part's `mat.*` data (the alien is a data row).
- **The data-defined insult registry.** A fixed Rust kernel SET (Archard mechanical wear `laws.rs:628`, thermal stress `laws.rs:880`, toxin/`net_harm` `laws.rs:161`, corrosion `laws.rs:1422`, dissolution `laws.rs:1464`, starvation from the homeostatic drain), each a row that reads the part's own `mat.*` and situation and returns an energy increment. Membership is data (the harden-to-registry pattern, sibling to the capability and combinator registries), so a new insult (including the held oxidative row) is a row, not a rewrite. Each insult's energy contribution is dimensionally DERIVED against the fracture-energy scale, never a free per-insult weight (the Archard-weight backdoor stays closed).
- **The repair flux.** A per-tissue-material tissue-turnover rate, a new `mat.*` axis on the part's material data (reserved-with-basis from real tissue-turnover data), converted to energy-repaired-per-tick through the same fracture-energy scale, funded by a maintenance-energy draw with its real EXTENSIVE demand (repair across all tissue, so demand scales with tissue amount). No mass-neutrality is engineered; whatever the real energy economy produces is the output.
- **The pool projection (R-TIER-CONSIST).** Most beings die at the aggregate pool tier with no per-part body (`design.md:2497-2499`), so a per-archetype pushforward approximates duration = tolerance / net-accumulation-rate. It MUST be demonstrated to track the promoted per-part first-passage simulation to fixed-point tolerance, not asserted; if real fatigue is strongly nonlinear or cascades across parts and the projection cannot track, that is reported with what it would take.
- **The consumer.** `lifespan` feeds generational turnover and cultural-drift speed (`design.md:1724`); the derived value replaces the authored interim as its input.

## The reserved values (fail-loud sentinels, never fabricated, each basis given)

- Per-tissue tissue-turnover REPAIR RATE (per material). Basis: real measured tissue-turnover / cell-renewal rates per tissue type. THE GATING DEPENDENCY: if it cannot be grounded from real data (reverts to an outcome-constant whose only basis is the lifespan it yields), STOP and report why and what it would take.
- Each insult's energy-commensuration is derived through the fracture-energy scale (no free weight); any floor constant an insult law already reserves keeps its own real-data basis.
- The maintenance-energy draw's magnitude and its extensive-demand coupling. Basis: real maintenance-metabolism data; modelled as a real energy cost, not tuned for a target lifespan.
- Per-material death tolerances: the part's own fracture-energy (or declared failure-energy), already floor data.

## The build slices (each framed by this plan; section-9 five-lens audit before every push)

1. **Inert data-defined insult registry + the energy accumulator** (byte-neutral, no live death change): the registry kernel and its rows, the `PartCondition.damage_energy` field, the derived-integrity read, all landed inert (no scenario reads it yet), so the four `run_world` pins are unchanged.
2. **The repair flux + the extensive-demand maintenance economy** (inert): the per-tissue turnover repair and its maintenance funding, still not wired to the live death path.
3. **Switch the death path to first-passage** (the stated hash change): a vital part's `damage_energy` reaching its tolerance kills, alongside the existing destroyed-part rule; the four pins are re-captured with the intended change stated, re-pin timing coordinated through the gate (batched with C's corrected-T3 and any pending re-pins).
4. **The pool projection + its R-TIER-CONSIST tracking validation**: the per-archetype pushforward, with the demonstration that it tracks the promoted per-part simulation to fixed-point tolerance.
5. **Feed the consumer the derived lifespan and retire the authored interim**: the cultural-drift consumer reads the derived value; the authored per-race `lifespan_years` drops from a live input to a fail-loud reserved interim only, then retires.

## The held owner-call (a registry row, does not block the core)

The fed-state metabolic/oxidative-damage insult. INCLUDE it keyed on the being's own metabolic byproducts and antioxidant/repair data (a real per-being physical insult; a size-longevity slope may EMERGE but from real physics, and per-race data overrides it, the bird / bat / naked-mole-rat case) versus EXCLUDE it (the slope then comes only from the mechanical and chemical insults). Both are non-authoring; the owner's earlier objection was to the AUTHORED shape, which neither commits. The core is built as the pluggable-insult registry so the oxidative row lands or does not once the owner rules, a data row either way.

## Discipline

Every floor input is validated against real material / tissue-turnover / metabolic data, never the lifespan output against a longevity curve, and never tuned until lifespans "look right" (that is authoring through calibration). Read-only on `anatomy.rs` where Agent A's keystone will add a transduction marker; raise to the gate if an edit there is needed. Disjoint from Agent C's `environ.rs`. The death-path switch (slice 3) is the one not-byte-neutral step, its re-pin timing coordinated through the gate.
