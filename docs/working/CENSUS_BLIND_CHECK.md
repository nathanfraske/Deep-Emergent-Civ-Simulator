# Census blind-bias-check (Agent B, independent third pass)

This is the independent blind-bias-check of the calibration census (`CALIBRATION_CENSUS.md`) and the `category` field on all 224 `calibration/reserved.toml` entries, run under the owner directive that each agent independently checks the reconciliation with its own section-11-style checker. Agent B did not author the census, so this is a free-to-disagree independent read, verified against source (Prime Directive 1: a panel is a lead generator, never a verdict).

## Method

A blind fan-out of 16 independent auditors re-classified all 224 census entries under the locked three-way test (fundamental / per-world / derivable / defect), each grounded ONLY in the mechanism code (`crates/*.rs`) and the design document, blind to the census verdict, the live `category` field, and every prior working doc that carries a classification. The auditors returned, per entry, a category, a disposition (stays / relocate / derive), a confidence, and a one-line source-grounded reason. The construction itself was gated by a section-11 input-bias smoke test on the strongest model, fail-closed: it FAILED the first rev on two modes, an answer-key leak (the rubric named `metabolism.stefan_boltzmann`, a live entry, as a derives-example) and an under-scoped blindness denylist (three files while other working docs echo the prior verdicts), both were fixed, and the re-smoke PASSED clean on all six modes. The independent classifications were then diffed against the census placement and the live category field, and every surfaced disagreement was verified against source by hand.

## Result 1: the 14 census MOVE verdicts all hold

The independent pass agrees with all 14 of the census's MOVE verdicts (0 disputes): the 7 relocates (`axiom.calcification_rate`, `gossip.trust_baseline`, `gossip.trust_penalty`, `metabolism.kleiber_coefficient`, `harm.feature_granularity`, `reward.feature_granularity`, `discovery.target_value_granularity`) and the 7 derives (`langdet.salience_decay_floor`, `time.life_cadence_ticks`, `body.strike_energetics`, `field.diffusion.high`, `metabolism.stefan_boltzmann`, `felt_conviction.move_threshold`, `felt_conviction.move_plasticity`). The census's MOVE direction is sound.

## Result 2: A's `loss_practitioner_floor` correction confirmed

The one entry where the independent pass disputes a `derivable` categorization is `transmission.loss_practitioner_floor`, which it reads as a legitimate `per_world` practitioner-count set-point rather than a derivable duplicate. This is exactly the correction Agent A's blind-check already applied (derivable to per_world), so the independent pass confirms A rather than finding anything new here.

## Result 3: the under-cull finding, eight confirmed entries the census kept that should MOVE

The blind pass flagged, in the under-cull direction the gate asked me to hunt (an entry the census kept as `per_world`/stays that should relocate or derive), a batch of candidates. I do NOT assert the whole batch, since a blind pass told to hunt derive/relocate can carry its own culling lean, and asserting every flag would be that lean rather than a verdict. Filtered to the entries whose OWN `basis` text self-declares a derive or relocate relationship (the hardest-to-dispute class, where the census kept an entry the entry itself says should move) and verified at source, eight hold:

The two clean DERIVE-from-sibling cases (a value whose basis says it equals another reserved value, the textbook derivable case, still authored as its own key):

- **`productivity.soil_baseline`** (high). Its own basis: "MUST equal soil_requirement so soil never limits at baseline ... a placeholder supply ... reversible." Yet `environ.rs` reads it as a separate `require_fixed("productivity.soil_baseline")` right beside `soil_req`. The codebase already retired the IDENTICAL pattern for `hydrology.saturation_t_ref` (the `environ.rs` comment states it "duplicated" the mean surface temperature and now derives from `climate.mean_surface_temperature`), so `soil_baseline` is the same uncaught duplicate: it should derive from `productivity.soil_requirement`, not be independently authored. The strongest single finding.
- **`behavior.selection_generations`** (high). Its own basis: "set equal to the pre-dawn radiation depth (biosphere.predawn_generations = 40) for consistency ... rather than an independent figure." Yet `evolve.rs` reads it as its own fail-loud manifest key while `biosphere.predawn_generations` is a separate dial. It should derive from `biosphere.predawn_generations`.

Four SUPERSEDED-flat-key cases (the flat manifest key has zero read-sites in the code because the value already lives as per-race or per-substance data, so the flat entry is a stale duplicate, and "stays as an authored per_world scalar" understates it, category `per_world` for the underlying data is defensible but the flat entry is superseded):

- **`being.plasticity_by_age`** (high). Basis says "derives"; zero read-sites; the value lives as per-race `TraitDef.plasticity_curve` evaluated by `plasticity_at`.
- **`being.maturity_targets`** (high). Basis says "derives", "per-race"; zero read-sites; lives as per-race `TraitDef.maturity_target` drifted by `age_personality`.
- **`compose.viability_threshold`** (high). Basis says "derives"; zero read-sites; `eval.rs`/`promote.rs` derive it to a safety-fraction-of-zero at the material's own yield/fracture boundary.
- **`body.fluid_critical_fraction`** (high). Basis: "a single 0.33 is the mammalian figure applied to every race and fluid. Author per fluid, not as a global default." Zero manifest read-sites (a hardcoded 0.33 in `FluidRegistry::dev_default`); should be per-fluid data.

Two RELOCATE cases (a paired or placeholder value authored as a global where its own basis says per-race/per-fluid):

- **`body.clot_and_breach_rates`** (high). `clot_rate` is correctly per-`FluidDef`, but the paired `breach_bleed` is a single global `BodyParams` scalar applied uniformly regardless of the breached fluid, contradicting the basis's "per race" pairing; move `breach_bleed` onto `FluidDef` beside `clot_rate`.
- **`value_metric.enculturation_pull_rate`** (medium). Basis says the honest home is a per-race `belief_plasticity` datum; no consumer or `belief_plasticity` field exists yet, so it is a placeholder for per-race data.

## Honest limits and what I do NOT assert

The blind pass surfaced roughly thirty further candidates in the derive/relocate direction (engine-clamp anchors like the `controller.taxis.*` quartet whose magnitude is pinned by the activation clamp; representability caps like `hydrology.saturation_cap` and `body.damage_caps` that arguably belong as hardcoded consts rather than owner-manifest entries, with a Terran-anchoring risk on `saturation_cap`; and per-race/per-substance globals like `metabolism.respiration_transfer_coefficient`, `body.burn_scale`, and the `hydrology.saturation_e_ref`/`evaporation_still`/`saturation_slope` group). These are a softer signal: on the entries I sampled, the census's decision to keep them `per_world` is a defensible judgment call, and I did not verify each to the standard needed to overturn it, so I flag them for the owner's optional deeper review rather than assert them as census errors. The full 224-entry independent classification is preserved alongside this record. The residual risk in my own pass is a derive-lean induced by asking auditors to hunt the move direction; the eight confirmed above are the entries where the entry's own basis removes that risk by self-declaring the move.

## Bottom line

The census is sound on the direction that matters most: its 14 MOVEs all hold, and its overall discipline (source-grounding, the anti-steer guard, the honest-limits notes) is intact. The independent pass adds eight confirmable under-culls the census kept, the clearest being `productivity.soil_baseline`, whose own basis says it MUST equal `soil_requirement` and whose twin `saturation_t_ref` the codebase already retired to a derive. These are surfaced for the owner's calibration, not a rejection of the census.
