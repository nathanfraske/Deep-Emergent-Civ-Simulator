# Lifespan derived from anatomy (R-AGING): grounding and loop-to-bedrock plan (Agent B)

Doc-only opener for the R-AGING arc, redeployed to Agent B by the gate: a standing owner directive to replace the authored `Race.lifespan_years` (and `maturity_years`, and the life-hazard curve behind them) with a derivation from the being's own body, so a large slow-metabolism lineage is long-lived and a small fast one short-lived BECAUSE of its anatomy, never an authored per-race number. Disjoint from Agent A (perception) and Agent C (environ). No code here; the gate rules on the framing before any code.

## The directive, at source

The owner directive is stated in place (`race.rs:84-93`): lifespan and maturity and the life-hazard curve must be DERIVED from the race's own anatomy and physiology, the way `physiology::derive_base_drain` derives metabolism from the body: from body mass and metabolic rate (the mass-longevity and rate-of-living scaling), organ integrity and repair capacity, and whatever else the body's own physics dictate. Author it in the physics floor and grow the rest; a magical, silicon, or photosynthetic race then gets its own lifespan as a data row from its own body. Recorded in `OWNER_DECISIONS_LOG.md` (R3) and the roadmap R-AGING entry.

## Grounding (what is authored, what the body already carries)

- **The authored values** (`race.rs`): `lifespan_years: u32` (`race.rs:94`) and `maturity_years: u32` (`race.rs:100`), plain owner-set counts with no formula. `Race::life_fraction` (`race.rs:253-266`) normalizes raw age to the fraction the life-hazard curve reads; `Race::maturation_fraction` (`race.rs:237-249`) normalizes to the maturation fraction. Both are per-race data feeding one shared curve (Principle 9), so the derivation replaces the two source numbers, not the curve-normalizing mechanism.
- **What the body already carries** (read-only, Agent A's surface): `BodyPlan.body_mass` (`anatomy.rs:577`), and the metabolic rate through the Kleiber basal drain (`homeostasis.rs:252`, `Homeostasis::from_mass` at `homeostasis.rs:335`, the sibling `derive_base_drain` the directive names as the model). So body mass and metabolic rate, the two primary allometric inputs, are already present and derivable, not new substrate.
- **Consumers to keep consistent** (read-only): `maturity_years` feeds language maturation cadence (`language.rs:543-705`) and personality plasticity (`personality.rs:23-274`); `lifespan_years` feeds the life-hazard fraction (`race.rs:253`). A derived lifespan flows through the same `life_fraction`/`maturation_fraction` functions, so downstream code is unchanged; only the source numbers become derived.

## Loop to bedrock (the layers, to be verified and framed blind)

- Layer 0: the authored `lifespan_years` / `maturity_years` counts.
- Layer 1: an aging/senescence DERIVATION reading the body: lifespan as a function of body mass and metabolic rate (the mass-longevity and rate-of-living relations), modulated by organ integrity and repair capacity.
- Layer 2 (what the derivation needs): body mass (present, `anatomy.rs:577`), metabolic rate (present, the Kleiber drain), organ integrity and repair capacity (to be located, or reserved fail-loud with basis if the body does not yet carry it), and the ALLOMETRIC SCALING RELATIONS (mass-longevity exponent, rate-of-living coupling), which are grounded biology and therefore physics-floor LAW CONSTANTS, the one authored place.
- Bedrock: the allometric law constants (the mass-longevity exponent and the rate-of-living metabolic coupling) authored in the physics floor as reserved-with-basis values, plus any organ-integrity substrate the body does not yet carry. The build target is a senescence-derivation module that reads the body's own mass, metabolic rate, and integrity and derives lifespan, maturity, and the hazard scale through those floor law constants, so the authored per-race numbers are retired.

## Boundary and discipline

- Edit surface: `crates/sim/src/race.rs` (the `lifespan_years`/`maturity_years` fields become derived) plus a NEW aging/senescence derivation module. READ-only on `anatomy.rs` / `body.rs` / `homeostasis.rs` (Agent A's slice-2 derivation may touch `anatomy.rs`); if the derivation needs an edit there, raise it to the gate to sequence with A. Disjoint from Agent C's `environ.rs`.
- Not byte-neutral: deriving lifespan changes the source numbers where lifespan and maturity are consumed (the hazard fraction, the language and personality cadences), so this is a behaviour-changing arc. The intended hash change against the four pins (`2b7e1035` / `1873c44e` / `4eea5d06` / `bae5a82`) will be stated with its reason when the derivation lands; the derivation module itself can land inert first (byte-neutral) before the `race.rs` fields are switched to read it.
- Every allometric constant is authored ONLY in the physics floor, reserved-with-basis, never fabricated; where the floor cannot yet supply a value (an organ-integrity term), it is flagged and reserved fail-loud with its basis, and the substrate grown rather than a number smuggled in.
- The framing is blind-panelled (section-11 fail-closed, then section-10) before any code, and surfaced to the gate.

## Next

Loop this to bedrock in source (locate organ integrity / repair capacity; confirm the mass and metabolic-rate reads; name the allometric law constants), frame the derivation blind, and post the framing and the named build target for the gate. Tier B (the single-axis transduction, PR #111) stays the built, signed-off foundation and is banked to `main` as this branch opens.
