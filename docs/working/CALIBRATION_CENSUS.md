# Calibration census: the three-way-test classification of `calibration/reserved.toml` (224 entries)

This is the census the reconciliation billboard names as step 1, posted for the gate's adversarial confirmation BEFORE the category-field sweep is built or enforced. It classifies every one of the 224 `reserved.toml` entries under the locked three-way test (AGENTIC_ADDENDUM.md section 9): each reserved value is exactly one of a fundamental universal constant (1), a per-world / per-substance / per-race datum (2), or derivable from (1) and (2) (3). It supersedes the prior `CALIBRATION_RECONCILIATION.md` pass, which used a two-verdict scheme (LEGITIMATE-DATUM / PER-WORLD-IFY) that folded the derivable category into "legitimate" and predates the locked test.

## Method and the anti-steering guard

The classification ran as a fan-out over the 224 entries, each classified on its own three-way-test merits and source-grounded (the cited `crates/*.rs` and `docs/design.md` Read or grepped whenever the category was not self-evident or the basis asserted a category). The construction was gated by a section-11 input-bias smoke test (strongest model, maximum reasoning, fail-closed) per the billboard's anti-steering guard: the smoke CLEARED the rubric as neutral, with two construction notes recorded as honest limits below. There was no predetermined target for how the manifest ends up; a value stays as a legitimate fundamental or per-world datum as readily as it moves. Every one of the 14 MOVE verdicts below was then verified against source by hand (Prime Directive 1): the panel is a lead generator, never a verdict.

## Headline result

Of the 224 entries: **210 STAY** in place and **14 MOVE** (7 relocate to per-world/per-race data, 7 derive). By category: **199 per-world (2)**, **25 derivable (3)**, and **0 fundamental (1)**. The zero-fundamental count is expected and load-bearing for the fundamentals-home piece: `reserved.toml` is the CALIBRATION manifest, so it holds contingent per-world data and derived pointers, never the fundamental constants themselves. The fundamentals (c, k_B, h, e, eps_0, N_A) do not live here today and are the separate closed table the arc's third piece instantiates; the composites that today sit in this manifest as authored decimals (Stefan-Boltzmann sigma) derive INTO that table rather than staying here.

The anti-steering guard held: 210 of 224 entries legitimately STAY, most as genuine category-2 per-world control set-points and per-race data, and no value was culled because the list "should" get shorter. Every MOVE is grounded in the value's own three-way-test category, and many of the moves confirm a STEER FLAG the entry's own basis already carries (the trust anchors, the calcification rate, the sensory granularities, the Kleiber coefficient), so the census is applying the locked test to flags the manifest already surfaced, never inventing a cull.

## The 14 MOVE verdicts (source-verified)

### Relocate to per-world / per-race data (7): a contingent Terran anchor authored as a global

- **`axiom.calcification_rate`** (confidence medium). Source (axiom.rs calcify field, reserved) and the entry's own STEER FLAG confirm this absolute per-tick rate bakes in a human-generation lifespan; it is a Terran-anchored global magnitude that must be re-expressed as a dimensionless per-race lifespans-to-harden datum (then the per-tick rate derives by dividing by each race's lifespan), so the contingent datum belongs in per-world data rather than as an authored global.
- **`gossip.trust_baseline`** (confidence high). A Terran empirical anchor: 0.5 is the human investment/trust-game figure (Berg, Dickhaut, McCabe) authored as a global scalar; its own basis carries a STEER FLAG that it should become per-race epistemic-stance data, so it is category-2 per-world that must move to a per-race row.
- **`gossip.trust_penalty`** (confidence high). A Terran trust-asymmetry figure (Slovic 1993) authored globally; its basis STEER FLAG states it should become per-race data paired with trust_baseline, so it is a contingent per-world/per-race datum that must be relocated off the global anchor.
- **`metabolism.kleiber_coefficient`** (confidence high). MetabolicAnchors reads a single global metabolism.kleiber_coefficient (3.4, mammalian) applied to all bodies, but the coefficient a is per-metabolism-type contingent (a cold-blooded or exotic metabolism scales it); a Terran empirical anchor authored as a global that must become per-race data, the rubric's named category-2 relocate example.
- **`harm.feature_granularity`** (confidence medium). A per-race sensory just-noticeable difference; sensorium.rs already carries a per-channel resolution (JND) map as per-race data, and the basis says this should BE that acuity 'rather than a free parameter'. learn.rs currently authors it as one global require_fixed scalar, so a contingent per-substance/per-race datum sits as a global magnitude and should become the per-channel sensorium-resolution data row.
- **`reward.feature_granularity`** (confidence medium). The reward twin of harm.feature_granularity, a per-race sensory JND the basis ties to the sensorium's per-class acuity (which sensorium.rs holds per-channel), authored as one global require_fixed scalar. A contingent per-race/per-substance datum sitting as a global magnitude; should become the per-channel sensorium-resolution data row.
- **`discovery.target_value_granularity`** (confidence medium). A quantization JND on the affordance value scalar, basis-tied to the same sensorium per-class just-noticeable difference as the harm/reward granularities. A per-race sensory-resolution datum authored as a single global scalar; belongs as the per-substance/per-channel sensorium-resolution data row rather than one authored number.

### Derive from the fundamentals and the situation (7): a composite or already-derived quantity that must not be stored

- **`langdet.salience_decay_floor`** (confidence medium). The floor is fully determined by the fixed-point scale (decay_bits >= ceil(2^32/usage_max_bits)), the smallest representable positive leak; it is computed from the quantization once usage_max is fixed, so it should be derived rather than stored as its own authored number.
- **`time.life_cadence_ticks`** (confidence high). clock.rs derives World::life_cadence_ticks as ticks_from_seconds(world.orbital_period_seconds, base_tick_seconds), confirmed by the test the_life_cadence_derives_from_the_orbit_and_base_tick; the authored 31536000 is the Earth interim composite of per-world orbital period over base tick and must derive, not be stored.
- **`body.strike_energetics`** (confidence medium). strike() consumes velocity/applied_force/contact_area as inputs that the basis and code frame as derived from the wielder's strength (physiology) and the weapon part's geometry/mass, feeding kinetic_energy and the contact laws; these are per-being mechanical outcomes computable from body data, not authored magnitudes.
- **`field.diffusion.high`** (confidence medium). the dense-medium diffusion regime is the same k/(rho*c) derivation over a denser medium's thermal properties (medium.water/dense_toxic rows), so the world-dial is the medium choice, not a separately authored ~0.22 coefficient; the recommended value tracks the numerics stability rail rather than physics, so it should derive like its baseline field.diffusion.
- **`metabolism.stefan_boltzmann`** (confidence high). sigma=5.670374e-8 is passed to laws::radiant_emission as an authored decimal, but it is the composite 2*pi^5*k_B^4/(15*h^3*c^2) computable from the fundamentals; a CODATA-labelled composite that must be re-derived, never stored as its own number, the rubric's canonical category-3 example.
- **`felt_conviction.move_threshold`** (confidence medium). Source (conviction_experience.rs:193, comment 147) confirms this is an interim standalone manifest key whose canonical form is axiom.entrenchment_curve read at the axiom's rank; the entrenchment threshold is already per-race curve data, so this scalar should derive from it and never be stored as its own number once the rank-scaled gate is wired.
- **`felt_conviction.move_plasticity`** (confidence medium). Source (conviction_experience.rs:194, comment 153) confirms this is an interim standalone key whose canonical value is the per-being Mind::plasticity phenotype the axiom kernel already uses; it duplicates existing per-being data and should derive from that phenotype rather than remain an authored scalar.

## Honest method limits (the section-11 smoke's two construction notes)

The smoke cleared the construction but recorded two fidelity notes, both of which I carry forward rather than paper over. First, the classification schema offered only the three legitimate categories, with no fourth DEFECT state for a value that fits none of the three (a global authored magnitude that is a bug in the derivation). None of the 224 entries turned out to be such an irremediable defect (every MOVE has a clean remediation, a relocate or a derive), so the outcome is unaffected, but the category-field schema the sweep builds should carry a defect state so a future value that fits none of the three is recorded as a bug rather than laundered into a legitimate category. Second, the source-check distrust examples named the anti-cull direction (a basis over-claiming "a universal constant") more prominently than the culling direction (a basis that downplays a value as "a mere tuneable" where the source may show a datum that must STAY); the per-agent blind-checks (billboard step 4) are the guard against a residual culling bias, and they must be free to disagree with this census. Third, a section-9 alien-feasibility catch on the `metabolism.kleiber_coefficient` relocate verdict, flagged for the relocation follow-on and the blind-checks: the census correctly categorizes the coefficient `a` as a per-world/per-race datum, but the allometric SCALING EXPONENT it anchors (the 3/4 in `P = a*mass^(3/4)`) is hardcoded in `crates/physics/src/laws.rs` as a universal (the West/Brown/Enquist fractal-network derivation). That exponent is a branching-supply-network body-plan assumption, so an alien metabolism on a different supply geometry is only half a data row until the exponent is examined. It is outside this census's scope (a hardcoded law constant, not a `reserved.toml` entry), so it is not a mis-categorization here, but the kleiber relocation should carry the exponent's universality forward as its own three-way-test question. Fourth, a post-census correction from Agent A's independent blind-check (gate-confirmed, source-verified here): `transmission.loss_practitioner_floor` was re-categorized derivable to per_world. It is an authored minimum-viable-practitioner count read directly via `require_i64` (transmission.rs:140) with no built derivation, so it is a category-2 per-world datum, consistent with its sibling `genome.effective_population_size` (also per_world). Its placement (stays) is unchanged, so the headline placement counts hold (210 stay, 14 move); only this one label moved, giving 199 per-world / 25 derivable / 0 fundamental. The deeper question the blind-check surfaced, whether a minimum-viable count is ultimately a per-world authored datum or derivable from a population-genetics substrate, is recorded as an open question to revisit when that viability substrate is designed.

## Sequence

This census is posted for the gate's adversarial confirmation: does each of the 14 MOVEs hold, and does each of the 210 STAYs hold, against source. Only after the gate confirms the categorizations do the category field and its CI gate land (billboard step 2, additive, in a coordinated window), followed by the fundamentals-home (step 1's third piece) and the per-agent blind-checks (step 4). The token/magnitude collapse and the fixed-point sub-resolution representation stay out of this core.

## Appendix: the full 224-entry classification

| # | id | category | placement | confidence |
|---|----|----------|-----------|------------|
| 1 | `evidence.commit_threshold` | per_world | stays | high |
| 2 | `evidence.runner_up_margin` | per_world | stays | high |
| 3 | `evidence.log_odds_clamp` | per_world | stays | high |
| 4 | `absence.characteristic_lifespan_scan_ceiling` | per_world | stays | high |
| 5 | `evidence.concealment_suppression` | per_world | stays | high |
| 6 | `evidence.aggregate_diffusion_rate` | derivable | stays | high |
| 7 | `being.plasticity_by_age` | per_world | stays | high |
| 8 | `being.maturity_targets` | per_world | stays | high |
| 9 | `being.life_event_impulse` | per_world | stays | medium |
| 10 | `value_metric.axis_relationship_weights` | per_world | stays | high |
| 11 | `value_metric.conflict_coefficient` | per_world | stays | medium |
| 12 | `value_metric.enculturation_pull_rate` | per_world | stays | high |
| 13 | `value_metric.incommensurability_floor` | derivable | stays | high |
| 14 | `value_metric.etic_substrate_axes` | derivable | stays | high |
| 15 | `value_metric.etic_recurrence_min` | per_world | stays | high |
| 16 | `genome.gauss_approx` | per_world | stays | high |
| 17 | `genome.additive_mutation_step` | per_world | stays | high |
| 18 | `genome.environment_variance` | per_world | stays | high |
| 19 | `genome.loci_per_channel` | per_world | stays | high |
| 20 | `genome.mutation_rates` | per_world | stays | high |
| 21 | `genome.fertility_curve` | per_world | stays | high |
| 22 | `axiom.evidence_ring_curve` | per_world | stays | high |
| 23 | `axiom.evidence_ring_hard_cap` | per_world | stays | medium |
| 24 | `axiom.entrenchment_curve` | per_world | stays | high |
| 25 | `axiom.calcification_rate` | per_world | relocate_perworld **MOVE** | medium |
| 26 | `axiom.conformity_prestige_strengths` | per_world | stays | high |
| 27 | `axiom.group_aggregation_rule` | per_world | stays | high |
| 28 | `axiom.stubbornness_dogmatism_weight` | per_world | stays | medium |
| 29 | `lang.concept_thresholds` | derivable | stays | medium |
| 30 | `lang.phoneme_priors` | derivable | stays | high |
| 31 | `articulation.base_resonator_lengths` | per_world | stays | high |
| 32 | `articulation.vocal_tract_scale` | per_world | stays | high |
| 33 | `articulation.hearing_resolution` | per_world | stays | high |
| 34 | `articulation.producibility_threshold` | per_world | stays | medium |
| 35 | `articulation.word_min_len` | per_world | stays | medium |
| 36 | `articulation.word_max_len` | per_world | stays | medium |
| 37 | `lang.drift_operator_rates` | per_world | stays | high |
| 38 | `lang.distance_component_weights` | derivable | stays | high |
| 39 | `lang.l2_acquisition` | per_world | stays | medium |
| 40 | `lang.writing_invention_threshold` | per_world | stays | medium |
| 41 | `lang.dawn_round_cap` | per_world | stays | medium |
| 42 | `lang.prime_closing_threshold` | per_world | stays | medium |
| 43 | `lang.prime_founding_resilience` | per_world | stays | high |
| 44 | `lang.typology_temperature` | per_world | stays | high |
| 45 | `lang.typology_disharmony` | per_world | stays | high |
| 46 | `lang.typology_distance_weights` | derivable | stays | high |
| 47 | `langdet.salience_decay_rate` | per_world | stays | high |
| 48 | `langdet.salience_decay_floor` | derivable | derive **MOVE** | medium |
| 49 | `langdet.usage_recency_window` | per_world | stays | high |
| 50 | `langdet.retention_memory_scale` | per_world | stays | high |
| 51 | `langdet.substrate_quantization` | derivable | stays | high |
| 52 | `langdet.incommensurability_ceiling` | derivable | stays | high |
| 53 | `langmod.channel_registries` | per_world | stays | high |
| 54 | `langmod.perceptual_geometry` | derivable | stays | high |
| 55 | `langmod.capability_gates` | derivable | stays | high |
| 56 | `langmod.acquisition_split` | derivable | stays | high |
| 57 | `langmod.blend_propensity` | per_world | stays | high |
| 58 | `langmod.dawn_cap_unitsize` | per_world | stays | high |
| 59 | `langmod.mismatch_triggers` | per_world | stays | high |
| 60 | `inst.function_substrate_axes` | per_world | stays | medium |
| 61 | `inst.similarity_feature_weights` | per_world | stays | high |
| 62 | `inst.recognition_threshold` | per_world | stays | high |
| 63 | `inst.crystallization_rates` | per_world | stays | high |
| 64 | `inst.crystallization_threshold` | per_world | stays | high |
| 65 | `inst.crystallization_rate` | per_world | stays | high |
| 66 | `inst.distance_weights` | per_world | stays | high |
| 67 | `compose.max_depth` | per_world | stays | high |
| 68 | `compose.viability_threshold` | per_world | stays | high |
| 69 | `compose.transmission_stability` | derivable | stays | high |
| 70 | `compose.reuse_compression_threshold` | per_world | stays | high |
| 71 | `compose.interface_penalty_curve` | per_world | stays | high |
| 72 | `compose.emergent_proxy_weights` | per_world | stays | high |
| 73 | `compose.control_efficiency_floor` | per_world | stays | medium |
| 74 | `compose.resonance_floor` | per_world | stays | medium |
| 75 | `transmission.drift_rate` | per_world | stays | high |
| 76 | `transmission.loss_rate` | per_world | stays | high |
| 77 | `transmission.loss_practitioner_floor` | per_world | stays | medium |
| 78 | `tier.significance_thresholds` | per_world | stays | medium |
| 79 | `tier.decision_propensity` | per_world | stays | medium |
| 80 | `tier.belief_level_to_strength` | per_world | stays | high |
| 81 | `tier.belief_dispersion` | per_world | stays | high |
| 82 | `tier.partition_remainder_rule` | per_world | stays | high |
| 83 | `tom.access_weight.reachable` | per_world | stays | medium |
| 84 | `tom.access_weight.absence` | per_world | stays | medium |
| 85 | `tom.access_weight.denied` | per_world | stays | high |
| 86 | `gossip.told_weight` | per_world | stays | high |
| 87 | `gossip.trust_baseline` | per_world | relocate_perworld **MOVE** | high |
| 88 | `gossip.trust_penalty` | per_world | relocate_perworld **MOVE** | high |
| 89 | `language.innovation_rate` | per_world | stays | medium |
| 90 | `language.sound_change_rate` | per_world | stays | medium |
| 91 | `genome.effective_population_size` | per_world | stays | high |
| 92 | `reproduction.mutation_spread` | per_world | stays | medium |
| 93 | `genome.speciation_distance` | per_world | stays | high |
| 94 | `genome.selection_scaling` | per_world | stays | medium |
| 95 | `genome.point_mutation_rate` | per_world | stays | high |
| 96 | `genome.recombination_default` | per_world | stays | medium |
| 97 | `biosphere.predawn_generations` | per_world | stays | high |
| 98 | `genome.speciation_incompatibilities` | per_world | stays | high |
| 99 | `genome.allele_presence_threshold` | per_world | stays | high |
| 100 | `speciation.distance_threshold` | per_world | stays | high |
| 101 | `speciation.incompatibility_threshold` | per_world | stays | high |
| 102 | `time.base_tick_seconds` | per_world | stays | high |
| 103 | `time.life_cadence_ticks` | derivable | derive **MOVE** | high |
| 104 | `time.years_per_generation` | per_world | stays | medium |
| 105 | `world.orbital_period_seconds` | per_world | stays | high |
| 106 | `world.rotation_period_seconds` | per_world | stays | high |
| 107 | `physiology.base_metabolic_drain` | derivable | stays | high |
| 108 | `physiology.exertion_drain_coupling` | derivable | stays | high |
| 109 | `physiology.death_floor` | per_world | stays | medium |
| 110 | `behavior.controller_hidden_width` | per_world | stays | medium |
| 111 | `controller.taxis.move_bias` | per_world | stays | medium |
| 112 | `controller.taxis.heading_gain` | per_world | stays | medium |
| 113 | `controller.taxis.here_suppress` | per_world | stays | high |
| 114 | `controller.taxis.ingest_drive` | per_world | stays | high |
| 115 | `behavior.controller_target_mutations` | per_world | stays | high |
| 116 | `behavior.controller_mutation_step_fraction` | per_world | stays | high |
| 117 | `behavior.selection_pop_size` | per_world | stays | high |
| 118 | `behavior.selection_generations` | per_world | stays | medium |
| 119 | `behavior.episode_ticks` | per_world | stays | high |
| 120 | `body.tissue_properties` | per_world | stays | high |
| 121 | `body.damage_caps` | per_world | stays | medium |
| 122 | `body.fracture_damage` | per_world | stays | high |
| 123 | `body.function_loss_threshold` | per_world | stays | high |
| 124 | `body.burn_scale` | per_world | stays | high |
| 125 | `body.fluid_critical_fraction` | per_world | stays | high |
| 126 | `body.clot_and_breach_rates` | per_world | stays | high |
| 127 | `body.promotion_shape` | per_world | stays | medium |
| 128 | `body.strike_energetics` | derivable | derive **MOVE** | medium |
| 129 | `field.diffusion` | derivable | stays | high |
| 130 | `field.cell_size` | per_world | stays | high |
| 131 | `field.diffusion.high` | derivable | derive **MOVE** | medium |
| 132 | `field.relaxation` | per_world | stays | medium |
| 133 | `climate.mean_surface_temperature` | per_world | stays | high |
| 134 | `climate.latitude_temperature_range` | per_world | stays | high |
| 135 | `field.relaxation.low` | per_world | stays | medium |
| 136 | `field.body_exchange` | derivable | stays | high |
| 137 | `metabolism.kleiber_coefficient` | per_world | relocate_perworld **MOVE** | high |
| 138 | `metabolism.body_mass_kg_scale` | per_world | stays | high |
| 139 | `metabolism.stefan_boltzmann` | derivable | derive **MOVE** | high |
| 140 | `metabolism.respiration_transfer_coefficient` | per_world | stays | high |
| 141 | `medium.submersion_elevation` | per_world | stays | high |
| 142 | `field.body_exchange.high` | per_world | stays | high |
| 143 | `physiology.thermal_setpoint` | per_world | stays | high |
| 144 | `physiology.thermal_setpoint.high` | per_world | stays | high |
| 145 | `physiology.thermal_setpoint.low` | per_world | stays | high |
| 146 | `physiology.thermal_half_band` | per_world | stays | high |
| 147 | `physiology.thermal_half_band.high` | per_world | stays | high |
| 148 | `physiology.thermal_half_band.low` | per_world | stays | high |
| 149 | `medium.air` | per_world | stays | high |
| 150 | `medium.water` | per_world | stays | high |
| 151 | `medium.dense_toxic` | per_world | stays | high |
| 152 | `genome.mutation_rates.high` | per_world | stays | high |
| 153 | `genome.mutation_rates.low` | per_world | stays | high |
| 154 | `genome.point_mutation_rate.high` | per_world | stays | high |
| 155 | `genome.point_mutation_rate.low` | per_world | stays | high |
| 156 | `genome.additive_mutation_step.high` | per_world | stays | high |
| 157 | `genome.additive_mutation_step.low` | per_world | stays | high |
| 158 | `genome.selection_scaling.high` | per_world | stays | high |
| 159 | `language.sound_change_rate.high` | per_world | stays | high |
| 160 | `language.sound_change_rate.low` | per_world | stays | high |
| 161 | `lang.drift_operator_rates.high` | per_world | stays | high |
| 162 | `lang.drift_operator_rates.low` | per_world | stays | high |
| 163 | `language.innovation_rate.high` | per_world | stays | high |
| 164 | `language.innovation_rate.low` | per_world | stays | high |
| 165 | `axiom.calcification_rate.high` | per_world | stays | medium |
| 166 | `axiom.calcification_cap` | per_world | stays | high |
| 167 | `axiom.calcification_brittleness` | per_world | stays | high |
| 168 | `axiom.conformity_prestige_strengths.high` | per_world | stays | high |
| 169 | `axiom.fission_threshold` | per_world | stays | high |
| 170 | `axiom.deviation_threshold` | per_world | stays | high |
| 171 | `value_metric.conflict_coefficient.high` | per_world | stays | high |
| 172 | `value_metric.conflict_coefficient.low` | per_world | stays | high |
| 173 | `being.life_event_impulse.high` | per_world | stays | high |
| 174 | `biosphere.predawn_generations.high` | per_world | stays | high |
| 175 | `genome.effective_population_size.low` | per_world | stays | high |
| 176 | `genome.speciation_distance.low` | per_world | stays | high |
| 177 | `genome.speciation_incompatibilities.low` | per_world | stays | high |
| 178 | `hydrology.saturation_slope` | per_world | stays | medium |
| 179 | `hydrology.saturation_e_ref` | per_world | stays | high |
| 180 | `hydrology.saturation_cap` | per_world | stays | medium |
| 181 | `hydrology.precipitation_rate` | per_world | stays | high |
| 182 | `hydrology.evaporation_still` | per_world | stays | high |
| 183 | `hydrology.evaporation_wind` | per_world | stays | high |
| 184 | `hydrology.evaporation_cap` | per_world | stays | medium |
| 185 | `hydrology.routing_rate` | per_world | stays | high |
| 186 | `productivity.water_requirement` | per_world | stays | high |
| 187 | `productivity.light_requirement` | per_world | stays | high |
| 188 | `productivity.temperature_requirement` | per_world | stays | high |
| 189 | `productivity.soil_requirement` | per_world | stays | high |
| 190 | `productivity.soil_baseline` | per_world | stays | medium |
| 191 | `hydrology.max_water_depth` | per_world | stays | high |
| 192 | `productivity.regen_rate` | per_world | stays | high |
| 193 | `productivity.colonization` | per_world | stays | high |
| 194 | `salinity.weathering_rate` | per_world | stays | high |
| 195 | `salinity.salt_cap` | per_world | stays | high |
| 196 | `salinity.dose_scale` | per_world | stays | high |
| 197 | `salinity.reference_water` | per_world | stays | high |
| 198 | `belief.enculturation_stubbornness` | per_world | stays | high |
| 199 | `belief.diffusion_rate` | per_world | stays | medium |
| 200 | `harm.noise_floor` | per_world | stays | medium |
| 201 | `harm.feature_granularity` | per_world | relocate_perworld **MOVE** | medium |
| 202 | `harm.p_harm_given_harms` | per_world | stays | high |
| 203 | `harm.p_harm_given_benign` | per_world | stays | high |
| 204 | `reward.noise_floor` | per_world | stays | medium |
| 205 | `reward.feature_granularity` | per_world | relocate_perworld **MOVE** | medium |
| 206 | `discovery.target_value_granularity` | per_world | relocate_perworld **MOVE** | medium |
| 207 | `reward.p_reward_given_rewards` | per_world | stays | high |
| 208 | `reward.p_reward_given_neutral` | per_world | stays | high |
| 209 | `reward.eligibility_decay` | per_world | stays | medium |
| 210 | `harm.eligibility_decay` | per_world | stays | medium |
| 211 | `being_percept.emission_coefficient` | per_world | stays | medium |
| 212 | `affordance_percept.reference_stress` | per_world | stays | medium |
| 213 | `discovery.exploration_floor` | per_world | stays | high |
| 214 | `discovery.surprise_threshold` | derivable | stays | high |
| 215 | `discovery.surprise_gain` | per_world | stays | medium |
| 216 | `planning.depth_cap` | per_world | stays | medium |
| 217 | `planning.hop_cap` | per_world | stays | medium |
| 218 | `felt_conviction.retention` | per_world | stays | medium |
| 219 | `felt_conviction.move_threshold` | derivable | derive **MOVE** | medium |
| 220 | `felt_conviction.move_plasticity` | derivable | derive **MOVE** | medium |
| 221 | `promotion.stress_threshold` | per_world | stays | high |
| 222 | `promotion.budget` | per_world | stays | high |
| 223 | `body.tissue_turnover_rate` | per_world | stays | high |
| 224 | `aging.wear_energy_ceiling` | per_world | stays | medium |
