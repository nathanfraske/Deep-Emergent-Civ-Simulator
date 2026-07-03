# Reserved-values worksheet: the owner's calibration batch across six worlds

This is a decision worksheet, not a change to the engine. Nothing here is written into `calibration/reserved.toml`, the physics floors, or the scenario files until you sign off. Every "recommended" value below is surfaced with its basis for you to ratify, adjust, or reject: it is a hypothesis with a citation, never a fabricated default the engine runs on (the prime directive, Principle 11). The mechanisms are fixed; these numbers are yours.

The worksheet covers the calibration manifest the lever system resolves against (`crates/sim/src/scenario.rs` over `calibration/reserved.toml`): 128 entries, 34 already set, **97 still reserved**. It organizes them the way the lever system does, and lays them across the six worlds: the four canonical (Mirror, Tempest, Arcanum, Confluence) and two new variants you asked for, **Venus** (a super-hot, toxic, dense-atmosphere world, some magic) and **Europa** (a full ocean world). A short appendix names the physics-floor reserved ranges, which are a separate reservation system (the `range_reserved` markers in `crates/physics/data/*.toml`), not part of this lever batch.

## How to read a row

Each reserved value has three things: an **id** (its manifest key), a **basis** (the ground on which you decide it, copied from the design), and, added here, a **recommended default** (a concrete value or a described shape, with a citation where the literature anchors one). Where a value differs by world, a per-world column gives the direction or the setting. Where it does not, it is a single value set once for every world.

There are three tiers, because the values fall into three kinds:

1. **The change-and-extremes dials** (§2): fifteen rates the lever system pushes per world with a `real`/`high`/`low` token. The magnitude behind each token is what you set; a world's column is which token it pulls. This is the tier the lever system fully covers today.
2. **The environmental levers** (§3): the field, thermal-band, and medium values that make a world hot, cold, toxic, or oceanic. These are what make Venus "Venus" and Europa "Europa," and they are the tier the medium-and-thermal work (R-MEDIUM, just completed) exercises. Only `field.*` is in the manifest today; the per-race thermal band and the medium content are reserved in code and in the physics floors and are **not yet manifest dials**. §3 recommends promoting them, so the two new worlds can be driven through the lever system rather than by hand.
3. **The world-invariant calibrations** (§4): the remaining reserved values, set once and shared by every world (a mutation clock, a metabolic drain, a wound property). These do not vary by world; a world deforms from them through the dials, not by re-setting them.

---

## §1. The six worlds at a glance

The four canonical worlds are unchanged (`scenarios/*.toml`, `docs/working/TEST_WORLDS.md`). The two new ones are proposed here; their full scenario definitions are in §5.

- **Mirror**: grounded baseline, no magic. The control. A few moderately-distinct races, every dial at its real-world analogue.
- **Tempest**: grounded, no magic, every change-and-extremes dial cranked high. The stress test of the change engines.
- **Arcanum**: high-magic stress test, minimally grounded. A potent `MagicLaws`, most races magical, the change dials set to churn magical traditions.
- **Confluence**: grounded reality with abundant magic, the headline target. Mirror's grounded dials run alongside a potent `MagicLaws` carried by a subset of races.
- **Venus** (new): a uniformly super-hot world under a dense, toxic, crushing atmosphere: a runaway-greenhouse surface hot enough to kill everywhere, a thick carbon-dioxide-and-acid air at scores of atmospheres, and almost no diurnal relief (the thick slow atmosphere smears day and night into one lethal temperature). Life, if it holds at all, holds high in the cooler cloud decks, buoyant in the dense air and away from the lethal surface, breathing a medium that is thin in anything respirable and laced with what harms. It is a world the medium substrate exercises hard on three fronts at once (R-MEDIUM): the lethal-hot in-medium thermal exchange (a body cooks toward the hot medium unless its band is shifted and widened toward the heat), respiration from a poor and toxic medium (the suffocation case, a large exchange surface straining a thin resource), and buoyancy in a dense medium (a light body floats up into the survivable cool cloud layer, a heavy one sinks to the lethal deep). The thermal challenge is one-directional here, flee-hot rather than the hot-and-cold gating, since the whole world is hot. Some magic: a modest `MagicLaws` present, a scarce adaptive edge rather than the ambient medium it is in Arcanum. Selection is harsh, viable populations are small and banded to the cool cloud decks.
- **Europa** (new): a global liquid-water ocean beneath an ice crust, lightless in the deep, warmed and fed at the seafloor by hydrothermal and tidal vents. Life is fully aquatic and chemosynthetic: no photosynthesis, energy from vent chemistry, not the sun. It is the world the medium substrate exists for (R-MEDIUM: respiration from dissolved gas, buoyancy from body density, in-medium thermal exchange in a cold dense medium, amphibious bands at the vents and the ice). The ocean's huge thermal mass buffers change, so the world is placid and slow-drifting. No magic (grounded, like Mirror but oceanic).

Postures (categorical, Part 20 and Part 34; the magic-intensity tokens become reserved ids as Part 34 builds):

| Posture | Mirror | Tempest | Arcanum | Confluence | Venus | Europa |
|---|---|---|---|---|---|---|
| races.count | few | several | many | several | few | several |
| races.diversity | moderate | high | high | moderate | moderate | high |
| races.magical_mix | no | no | no | yes | yes | no |
| magic.laws | false | false | true | true | **true** | false |
| magic.potency | n/a | n/a | high | high | **low-to-moderate** | n/a |
| magic.cost | n/a | n/a | low | real | **real** | n/a |
| magic.limit_looseness | n/a | n/a | high | moderate | **low** | n/a |
| magic.affinity_fraction | none | none | most | some | **few** | none |
| magic.affinity_weight | n/a | n/a | high | moderate-high | **moderate** | n/a |

Venus's "some magic" is deliberately the thinnest magical posture that still installs `MagicLaws`: present, costly, tightly limited, carried by few races and weighted moderately, so magic reads as a rare survival edge in a hostile world rather than an ambient utility. Europa carries no `MagicLaws` at all.

---

## §2. The change-and-extremes dials (the lever tier)

Fifteen dials, each pushed per world with a `real`/`high`/`low` token that resolves to a manifest id: `real` to the base id, `high` to the `.high` sibling, `low` to the `.low` sibling (`dial_manifest_id`, `scenario.rs`). You set the magnitude behind each token; the world's column is which token it pulls.

### 2a. The per-world direction matrix

| Dial | Mirror | Tempest | Arcanum | Confluence | Venus | Europa |
|---|---|---|---|---|---|---|
| genome.mutation_rates | real | high | high | real | real | **low** |
| genome.point_mutation_rate | real | high | high | real | real | **low** |
| genome.mutation_step | real | high | high | real | real | **low** |
| genome.selection_scaling | real | high | real | real | **high** | real |
| genome.effective_population_size | real | low | real | real | **low** | real |
| genome.speciation_distance | real | low | real | real | real | real |
| genome.speciation_incompatibilities | real | low | real | real | real | real |
| language.sound_change_rate | real | high | high | real | real | **low** |
| lang.drift_operator_rates | real | high | high | real | real | **low** |
| language.innovation_rate | real | high | high | real | real | **low** |
| axiom.calcification_rate | real | high | high | real | **high** | real |
| axiom.conformity_prestige_strengths | real | high | high | real | **high** | real |
| value_metric.conflict_coefficient | real | high | high | real | **high** | **low** |
| being.life_event_impulse | real | high | high | real | **high** | real |
| biosphere.predawn_generations | real | high | real | real | real | **high** |

Rationale for the two new worlds. **Venus** is a survival crucible: selection bites hard (`selection_scaling` high) on small, isolated cloud-deck populations (`effective_population_size` low), the constant environmental threat sharpens belief and in-group cohesion (`calcification`, `conformity_prestige`, `conflict`, `life_event_impulse` high), while the genetic and linguistic drift rates stay real (the extremity is environmental, in §3, not baked into the mutation clock). **Europa** is the placid opposite: the thermally-buffered ocean damps change, so the genetic and linguistic drift dials sit low and conflict is low (a resource-stable, slow world), but the pre-dawn is long (`predawn_generations` high) because a chemosynthetic vent ecology needs deep time to radiate into the dark, patchy niches life inherits.

### 2b. The magnitudes to set (base and pushed siblings)

The base `real` magnitude is shared by every world that pulls `real`; the `.high`/`.low` siblings are the stress ends. Several base values are already set (marked ✓ set); their siblings are what remain. A recommended default is given for each still-reserved magnitude, with its basis.

| Manifest id | Status | Recommended | Basis / citation |
|---|---|---|---|
| genome.point_mutation_rate | ✓ set 0.0001 | (set) | per-locus per-reproduction flip, molecular-clock anchored (Kimura) |
| genome.point_mutation_rate.high | reserved | **0.001** (10x) | one order up: standing variation and the clock run fast while a lineage stays stable across a generation |
| genome.mutation_rates (structural) | reserved | **0.001 /genome/gen** | structural (indel/rearrangement) rate per genome per generation, an order below point flips; set so rearrangements are rare novelty, not every birth |
| genome.mutation_rates.high | reserved | **0.02** | volatile end: genomes churn; upper bound where legibility or a determinism replay breaks |
| genome.mutation_step | reserved | **0.1 trait-SD** | continuous additive perturbation SD, in trait standard deviations (25.10 integer-Gaussian); small so a mutation is an increment |
| genome.mutation_step.high | reserved | **0.5 trait-SD** | large-step end: one mutation moves a trait far, novelty coarse |
| genome.selection_scaling | ✓ set 0.2 | (set) | breeder's-equation response scale |
| genome.selection_scaling.high | reserved | **0.5** | hard-selection end; upper bound where selection deterministically fixes or crashes a pool |
| genome.effective_population_size | ✓ set 200 | (set) | drift strength (Wright-Fisher) |
| genome.effective_population_size.low | reserved | **30** | violent-drift end; lower bound is pool persistence (a few tens is the small-Ne regime founder effects need) |
| genome.speciation_distance | ✓ set 0.30 | (set) | genetic distance declaring distinct species |
| genome.speciation_distance.low | reserved | **0.15** | near-speciation end: fast radiation and extinction (half the baseline) |
| genome.speciation_incompatibilities | ✓ set 2 | (set) | active DM pairs declaring isolation |
| genome.speciation_incompatibilities.low | reserved | **1** | a single complementary pair raises the firewall: fastest discrete speciation |
| language.sound_change_rate | ✓ set 0.02 | (set) | per-generation regular-change probability |
| language.sound_change_rate.high | reserved | **0.10** | fast-drift end: phonologies shift, languages splinter (5x) |
| language.sound_change_rate.low | reserved | **0.005** | placid end (Europa): a lineage drifts over many generations (quarter the baseline) |
| language.innovation_rate | ✓ set 0.02 | (set) | per-interaction coinage probability (naming game, Baronchelli 2006) |
| language.innovation_rate.high | reserved | **0.10** | high-coinage end; upper bound where a band cannot converge before the next coinage |
| language.innovation_rate.low | reserved | **0.005** | low-coinage end (Europa): stable lexicons |
| lang.drift_operator_rates | reserved | **see §4 (per-operator)** | the five drift operators; base rates per generation |
| lang.drift_operator_rates.high | reserved | **5x the base set** | fast lexical-and-grammatical churn |
| lang.drift_operator_rates.low | reserved | **0.25x the base set** | slow churn (Europa) |
| axiom.calcification_rate | reserved | **see §4** | belief-hardening rate |
| axiom.calcification_rate.high | reserved | **4x the base** | fast-hardening end: ideologies harden and schism (Venus, Tempest, Arcanum) |
| axiom.conformity_prestige_strengths | reserved | **see §4** | conformity and prestige strengths |
| axiom.conformity_prestige_strengths.high | reserved | **~2x the base** | strong-conformity end: belief cascades |
| value_metric.conflict_coefficient | reserved | **see §4** | value-distance to conflict-pressure coefficient |
| value_metric.conflict_coefficient.high | reserved | **~3x the base** | high-conflict end: differences ignite |
| value_metric.conflict_coefficient.low | reserved | **~0.3x the base** | low-conflict end (Europa): ordinary friction only |
| being.life_event_impulse | reserved | **see §4** | life-event and self-change burst sizes |
| being.life_event_impulse.high | reserved | **~2.5x the base** | strong-shock end: individual events ripple hard |
| biosphere.predawn_generations | ✓ set 40 | (set) | pre-dawn radiation depth |
| biosphere.predawn_generations.high | reserved | **120** (3x) | long, churned, boom-and-bust web (Tempest); also Europa's depth for a mature vent ecology; upper bound is aggregate-tier compute |

Note: `language.sound_change_rate.low`, `language.innovation_rate.low`, `lang.drift_operator_rates.low`, and `value_metric.conflict_coefficient.low` are new `.low` siblings that Europa's placid direction needs; they do not exist in the manifest yet. Adding them is a one-line data edit per sibling once you set the magnitude, and the resolver already supports the `.low` direction (`dial_manifest_id`). Until they are added, Europa's low dials fall back to the `real` baseline (a placid-enough starting point), so Europa runs without them.

---

## §3. The environmental levers (what makes Venus hot and Europa an ocean)

This is the tier the medium-and-thermal work exercises, and the one the lever system does not yet cover. A world's temperature field, its per-race thermal comfort band, and its medium are what make it a scorched rock or a buried ocean, and they are reserved values today, but only `field.*` is a manifest entry. The per-race thermal band lives in `BeingThermal` (reserved in `crates/sim/src/runner.rs`), and the medium content and the respiratory and buoyancy scales live in the physics floors (`range_reserved` in `crates/physics/data/*.toml`). To drive Venus and Europa through the lever system the way the four canonical worlds are driven, these should graduate to manifest dials. The recommendation and the per-world settings:

### 3a. The temperature field (`field.*`, already in the manifest)

| Manifest id | Basis | Mirror / Confluence / Arcanum | Tempest | Venus | Europa |
|---|---|---|---|---|---|
| field.diffusion | thermal diffusivity over cell and tick, below the 0.25 stencil stability bound | **0.15** | 0.15 | **0.22** (dense convective atmosphere smears heat toward uniform) | **0.22** (water conducts, near the stability bound) |
| field.relaxation | rate a cell relaxes to its solar-and-biome baseline; day-night and seasonal forcing timescale | **0.05** | 0.05 | **0.06** (thick slow atmosphere buffers day and night into one hot baseline) | **0.005** (under ice, lightless, almost no forcing: a near-static field) |
| field.body_exchange | body-to-environment convective coupling (Newton cooling, `law.convective_flux`) | **0.05** | 0.05 | **0.20** (the dense hot atmosphere couples the body fast: it cooks) | **0.20** (immersion in dense water couples the body fast to the medium) |

The recommended `field.diffusion` 0.15 and `field.relaxation` 0.05 baselines sit comfortably below the four-neighbour stencil's 0.25 stability bound and give a temperate world a slow diurnal breathing. Venus's high diffusion and mild relaxation are what produce a nearly uniform lethal-hot field with only a gentle gradient toward the cooler highlands and cloud decks; its high `body_exchange` is what makes the dense hot medium cook a body fast, the lethal-hot in-medium coupling. The signed thermoreceptor still reads "too hot" and drives a flee-hot climb toward the coolest cells, the one-directional half of thermotaxis (the full hot-and-cold bidirectional gating has no dedicated test world now, since every Venus cell is hot). Europa's near-zero relaxation and high diffusion are what produce a nearly uniform, thermally-buffered ocean.

### 3b. The per-race thermal comfort band (proposed manifest ids; today reserved in `BeingThermal`)

The comfort band (`BeingThermal.setpoint`, `.half_band`; `crates/sim/src/runner.rs`) is per race (Part 20 physiology). It is what the comfort-band map and the signed thermoreceptor read: a body outside `setpoint ± half_band` dies. Proposed manifest ids `physiology.thermal_setpoint` and `physiology.thermal_half_band`, per race, so a scenario can push them.

| Proposed id | Basis | Recommended (temperate race) | Venus race (thermophile) | Europa race (psychrophile) |
|---|---|---|---|---|
| physiology.thermal_setpoint | the race's homeostatic core-temperature set point (Part 20) | **310 K** (~37 C, mammalian) | **340 K** (a heat-shifted core, the thermophile adaptation) | **275 K** (~2 C, just above the ocean freezing point) |
| physiology.thermal_half_band | survivable half-range around the set point (Part 20 death conditions) | **8 K** | **15 K** (shifted and widened toward the heat, yet the surface still lies far outside it) | **6 K** (a narrow band in a stable cold ocean; no need to tolerate swings) |

The Venus recommendation encodes the design choice that its life adapts to the heat by shifting and widening the survivable band upward, yet even a heat-shifted band leaves the runaway surface lethal, so life must hold in the cooler cloud decks and flee the hottest cells; the shifted band plus the uniform hot field plus buoyancy (§3c) is what bands the population to the survivable altitude. Europa's narrow cold band encodes a stable-cold specialist.

### 3c. The medium (proposed manifest ids; today `range_reserved` in the fluid and biology floors)

The medium is a physics `Substance` defined by its axis values (density, respirable content, temperature). These are the R-MEDIUM floor values. Proposed manifest surfacing so a scenario can select a world's ambient medium.

| Proposed id (floor axis) | Basis | Temperate air (Mirror/Confluence/Arcanum) | Dense toxic atmosphere (Venus) | Cold water (Europa) |
|---|---|---|---|---|
| fluid.respirable_content (medium respirable-gas concentration) | dissolved-gas concentration or partial pressure the membrane-flux kernel reads | **~9 (mol·m⁻³ scale reserved)** air-oxygen analogue | **~0.1** (a toxic atmosphere thin in anything respirable: the suffocation case, a large surface straining a scarce resource) | **~0.3** vent-oxygenated deep water (low, so a large gill surface is needed) |
| medium.toxicity (proposed harm axis) | the per-tick harm the medium inflicts on a body immersed in it, beyond the respirable deficit | **0** (benign) | **> 0** (the acid-and-CO2 air corrodes: a toxic medium is a proposed harm-axis extension, mapping to the wound/edibility harm channel) | **0** (benign vent water) |
| mat.density (medium) | the buoyancy and drag medium density | **~1.2 kg·m⁻³** (air) | **~65 kg·m⁻³** (the crushing lower atmosphere; dense enough that buoyancy lifts a light body up toward the cooler cloud decks) | **~1000 kg·m⁻³** (water; buoyancy is central) |
| bio.respiratory_surface (per-quantity scale) | m² of exchange surface per unit organ quantity (the R-MEDIUM biology-floor axis) | **scale reserved** (set so a lung-sized surface breathes air) | **scale reserved** (set so even a large surface barely breathes the thin toxic medium) | **scale reserved** (set so a gill-sized surface extracts the low dissolved gas) |
| medium.in_medium_exchange_rate | the fraction per tick a body relaxes toward its medium temperature (the R-MEDIUM `in_medium_temperature` rate) | **0.05** (air, slow) | **0.20** (the dense hot atmosphere couples fast; equals `field.body_exchange` for Venus) | **0.20** (water immersion, fast; equals `field.body_exchange` for the ocean) |
| body.density_baseline | the body-density baseline buoyancy is measured against when no organ declares one (`BODY_DENSITY_BASELINE`, `medium.rs`) | **1000 kg·m⁻³** (water baseline) | **1000 kg·m⁻³** (same; a gas-filled body floats far below medium density, so light bodies rise) | **1000 kg·m⁻³** (buoyancy is the organ-composition deviation from it) |

Venus and Europa are the two worlds where the medium substrate does the most work. Venus's very low `respirable_content` against a large recommended surface is the suffocation case the respiration increment proves (a being suffocates in a poor medium from the medium's content, not a label), sharpened by the proposed `medium.toxicity` harm; its high `body_exchange` is the lethal-hot in-medium thermal exchange; and its dense atmosphere makes buoyancy the mechanism that lifts life into the survivable cool cloud decks (a light, gas-filled body rises, a heavy one sinks to the lethal deep). Europa's low `respirable_content` against a large gill surface is the emergent-affinity case the medium work proves: a high-surface membrane organ in a low-gas dense medium respires water (a gill), the same kernel a lung uses in air, with no "aquatic" flag; its high density makes buoyancy load-bearing the other way (a dense body sinks to the vents, a light one rises to the ice). The `medium.toxicity` axis is a proposed extension, not yet in the floors; it is what makes "toxic" more than "thin," and it is the natural next medium increment.

**Recommendation.** Promote §3b and §3c to manifest ids with `.air`/`.water` (or per-race) siblings, so Venus and Europa resolve their environment through the lever system rather than by hand-wiring. This is a small data-and-plumbing change (the floors already hold the axes; the manifest gains the entries and the scenario gains an `[environment]`/`[medium]` block). It is the natural next build after this batch is set, and it is what lets the two new worlds be first-class levered scenarios.

---

## §4. The world-invariant calibrations (set once, shared by all six)

The remaining reserved values do not vary by world; every world deforms from them through the dials above. Grouped by subsystem, each with a recommended default (a value where a scalar is anchored, a described shape where the value is a curve, set, or vector). Curves and sets are marked (shape); they need a structured value, not a single number, and the recommendation describes the shape and its anchor.

### R-EVIDENCE (Part 9, belief from the world)

| id | Recommended | Basis |
|---|---|---|
| evidence.implication_weights | (shape) a per-observation-type log-odds table: a fresh corpse ~5 nats, a stale stain ~1 | weight-of-evidence per observation type (Good 1950) |
| evidence.trace_decay_curves | (shape) exponential decay, half-life per trace type (blood ~days, bone ~years) at the tick rate | real decay and fading timescales |
| evidence.absence_windows | (shape) death-in-absentia waiting periods over the prominence axis, ~weeks to ~years, scaled by visibility | absence-window spread |
| evidence.concealment_suppression | **0.5** (halves trace perceptibility) with skill and time costs | suppression factor on perceptibility |
| evidence.aggregate_diffusion_rate | **0.02 /tick** | delayed knowledge diffusion at the aggregate tier |
| evidence.knowledge_to_strength | (shape) monotone saturating map, pool knowledge level to promoted belief strength | level-to-strength mapping |

### R-BEING-REP (Part 20, personality)

| id | Recommended | Basis |
|---|---|---|
| being.heritable_fraction | **0.5** per axis | literature centres near one half (do not fabricate into a constant) |
| being.plasticity_by_age | (shape) high in youth, falling to a low plateau by maturity, so drift and rank-order stability match 62.2 | plasticity-by-age curve |
| being.maturity_targets | (shape) small directional shifts (e.g. rising conscientiousness, falling novelty-seeking) with age | maturity-target directions |
| being.life_event_impulse | **~0.3 axis-SD per major event** (base) | life-event and self-change burst sizes; the dial's `real` end |

### R-VALUE-METRIC (Part 21, value distance)

| id | Recommended | Basis |
|---|---|---|
| value_metric.axis_relationship_weights | (shape) compatibility/opposition weights in [-1,1] per axis pair, default 0 (independent) | default relationship weights |
| value_metric.conflict_coefficient | **~1.0** (base, so unit value distance yields unit conflict pressure) | distance-to-conflict coefficient; the dial's `real` end |
| value_metric.enculturation_pull_rate | **0.01 /tick** toward the group mean, along the geodesic | enculturation pull rate |
| value_metric.incommensurability_floor | **~0.2 distance** | cross-race incommensurability floor |
| value_metric.etic_substrate_axes | (set) the shared cross-race comparison axes; start with the Part 21 standard set | etic substrate membership |

### R-GENOME residuals (Part 25)

| id | Recommended | Basis |
|---|---|---|
| genome.fertility_curve | (shape) soft falloff of cross-fertility with genetic distance, ~1 below the speciation distance tapering to 0 above | genetic-distance-to-fertility, beyond the binary hybrid outcome |

(The other genome values are set; see the manifest.)

### R-AXIOM (Part 28, axiomatic belief)

| id | Recommended | Basis |
|---|---|---|
| axiom.evidence_ring_capacity | **8** recent evidences (FIFO) | evidence-ring capacity |
| axiom.entrenchment_curve | (shape) threshold rising with hold; accommodation step ~0.1 | entrenchment-threshold curve and accommodation step |
| axiom.calcification_rate | **0.001 /tick** toward a cap ~0.95, brittleness rising with near-miss challenges | calcification rate, cap, brittleness; the dial's `real` end |
| axiom.conformity_prestige_strengths | **conformity ~0.1, prestige ~0.1**; fission threshold on member variance | conformity and prestige strengths, fission/deviation thresholds; the `real` end |
| axiom.group_aggregation_rule | (rule) entrenchment from member variance (low variance hardens), not the mean | group aggregation rule |

### R-LANG (Part 33, language and meaning)

| id | Recommended | Basis |
|---|---|---|
| lang.concept_thresholds | (shape) discrimination and lexicalisation thresholds; concept drift ~0.01/gen | thresholds and drift rate |
| lang.phoneme_priors | (shape) UPSID/PHOIBLE-grounded inventory priors with dispersion and implicational tendencies | phoneme sampling priors |
| lang.drift_operator_rates | (set) per-generation rates: sound change ~0.02, lexical replacement ~0.01, grammaticalisation ~0.005, splitting ~0.002, borrowing ~0.01 | the five drift operators; the dial's `real` end |
| lang.distance_component_weights | (shape) three weights summing to 1, lexical dominant (~0.6/0.25/0.15), residual absorbed into lexical (R-LANG-DET) | language-distance component weights |
| lang.l2_acquisition | (shape) rate, aptitude range, age-of-acquisition breakpoint after the late teens | L2 acquisition |
| lang.writing_invention_threshold | (shape) pressure threshold, script-type continuum weights, literacy spread, record decay | writing invention |
| lang.dawn_round_cap | **~50 rounds**, anchor-set ~7 | dawn dynamic round cap; performance vs emergence |
| language.generation_ticks | **31,536,000** (one in-world year, matching `time.life_cadence_ticks`) | ticks per generation, the drift cadence |

### R-LANG-TYPOLOGY (Part 33.4)

| id | Recommended | Basis |
|---|---|---|
| lang.typology_harmony_strong | (tilt) strong multiplicative tilt anchored ~94-97% harmonic on the adposition axis (WALS 95A) | strong-tier harmony tilt |
| lang.typology_harmony_weak | (tilt) weak tilt anchored on the genitive pair (Dryer 1992: 0.89 OV vs 0.45 VO) | weak-tier harmony tilt |
| lang.typology_disharmony | **~0.05** per axis (WALS 95A adposition disharmony) | disharmony draw probability |
| lang.typology_distance_weights | (shape) per-parameter grammatical-distance weights, lexical-dominant consistent | 33.5 grammatical distance weights |

### R-LANG-DET (Part 33, order-sensitive procedures)

| id | Recommended | Basis |
|---|---|---|
| langdet.salience_decay_rate | (rate) usage-recency half-life, with the hard underflow lower bound `decay_bits >= ceil(2^32/usage_max_bits)` | salience leaky-accumulator decay |
| langdet.usage_recency_window | (ticks) the retention record's recency window | usage recency window |
| langdet.substrate_quantization | (granularity) coarsest grid keeping concept regions separable, consistent with the Part 21 ground metric | substrate quantisation |
| langdet.incommensurability_ceiling | (distance) equal to the existing language-distance floor per cell | no-shared-form distance ceiling |

### R-LANG-MODALITY (Part 33.3, all modalities)

| id | Recommended | Basis |
|---|---|---|
| langmod.channel_registries | (registry) production modalities, reception senses, media, and each modality's feature dimensions; sensory-ecology menu, data per world | channel-registry membership |
| langmod.perceptual_geometry | (shape) per-modality distance geometry dispersed to the acoustic modality's target separation | perceptual geometry |
| langmod.capability_gates | (shape) capability floors equal to the wound model's function-loss threshold; curve knee at the grading bands | capability gates |
| langmod.acquisition_split | (shape) comprehension vs production tracks, production capped by channel capacity (receptive-bilingual asymmetry) | acquisition split |
| langmod.blend_propensity | (probability) measured blend-vs-switch rate, addressee/register dependent | code-blend propensity |
| langmod.dawn_cap_unitsize | (rounds) modality-blind function of coordinating-unit participant count | dawn cap by unit size |
| langmod.mismatch_triggers | (thresholds) homesign-dawn and modality-shift population fractions over the R-LANG-DET window | channel-mismatch triggers |

### R-INST (Part 36, institutions)

| id | Recommended | Basis |
|---|---|---|
| inst.function_substrate_axes | (set) force, the sacred, exchange, knowledge, care as a starting menu; exotic axes per race | function-substrate membership |
| inst.similarity_feature_weights | (shape) feature weights diagnostic of institutional sameness | similarity feature weights |
| inst.recognition_threshold | (threshold) trade between over-labelling and generic fallback | recognition threshold |
| inst.crystallization_rates | (shape) thresholds and rates a recurring pattern becomes an institution, against playtest cadence | crystallization rates (the hardest to set) |

### R-DEEPTECH-COMPOSE (Part 41, technology composition)

| id | Recommended | Basis |
|---|---|---|
| compose.max_depth | **~6 levels** (a determinism-and-performance bound) | depth where marginal proxy gain falls below noise |
| compose.viability_threshold | (physics units) the failure boundary the material/physics data define (e.g. pressure > yield) | viability threshold |
| compose.transmission_stability | (ticks) equal to the transmission subsystem's drift and loss rates | stability span and drift-similarity radius |
| compose.reuse_compression_threshold | (count) integer reuse-count surrogate for description-length decrease, fitting the memory budget | reuse compression threshold |
| compose.interface_penalty_curve | (shape) mismatch penalty from the interface-axis loss physics, and where an adapter is impossible | interface penalty curve |
| compose.emergent_proxy_weights | (shape) physics-substrate units and criticality per aggregate quantity | emergent proxy weights |

### R-TIER-CONSIST (Part 54, tier consistency)

| id | Recommended | Basis |
|---|---|---|
| tier.significance_thresholds | (shape) consequential events always promote, quiet drift never does | significance thresholds |
| tier.decision_propensity | (shape) threshold and accumulation rate for aggregate undertakings, matched to the detailed tier | decision propensity |
| tier.belief_level_to_strength | (shape) curve and per-mind dispersion so the population mean reconstructs the pool level within tolerance | belief level-to-strength |

### R-BEHAVIOR-EVOLVE physiology (Part 20, per homeostatic axis)

| id | Recommended | Basis |
|---|---|---|
| physiology.reserve_capacity_per_mass | (per axis) energy ~ high multiple of mass, water lower | reserve capacity per body mass |
| physiology.base_metabolic_drain | (per axis) energy ~1/300 of capacity/tick, water slower, at the one-second tick | basal metabolic rate on the base tick |
| physiology.exertion_drain_coupling | (per axis) energy couples strongly to exertion (~1/400/tick/exertion), water weakly | exertion drain coupling |
| physiology.death_floor | **0** (fraction of capacity) per axis, unless a race fails above empty | viable floor (Part 20 death conditions) |
| physiology.intake_yield | **0.25** of capacity per tick of ingesting (interim, until the edibility floor is wired) | intake yield (R-PHYS-BIO stand-in) |

### R-BEHAVIOR-EVOLVE controller (Part 8, the evolved behaviour controller)

| id | Recommended | Basis |
|---|---|---|
| behavior.controller_init_spread | **2.0** (weight half-range) | founder weight scale where the clamp is neither saturated nor near zero |
| behavior.controller_mutation_rate | **~1/6 per weight** | mutation scale the epoch uses, adjusted for the controller parameter space |
| behavior.controller_mutation_step | **~0.4** (weight) | bounded step: a small change is a small behaviour change |
| behavior.selection_pop_size | **~32 lineages** | sample size resolving the fitness ranking against the budget |
| behavior.selection_generations | (generations) tied to the pre-dawn radiation depth | selection generations |
| behavior.episode_ticks | **~200** | long enough to separate viable from unviable survival |

(`behavior.controller_hidden_width` is ✓ set to 4. The recommended controller values above match the development fixtures the emergent-behaviour work already runs and proves against, so they are low-risk starting points.)

### The body arc (Part 35, R-BUILD-PHYS / R-WOUND / R-FLUID)

| id | Recommended | Basis |
|---|---|---|
| body.tissue_properties | (vector, per tissue) hardness, fracture strength/energy, cutting energy, modulus, thermal expansion from datasheets (Yamada; Currey) | tissue material properties |
| body.damage_caps | (floor maxima) contact-pressure, energy, stress caps from the floor axis maxima | damage saturation caps |
| body.fracture_damage | **~0.9** integrity removed by a clean fracture of a load-bearing part | fracture-to-condition mapping |
| body.burn_scale | (temperature rise) the protein-denaturation threshold in floor units | full-severity burn scale |
| body.fluid_critical_fraction | **~0.33** of a fluid pool (mammalian blood-loss, Guyton and Hall), per fluid | survivable fractional loss |
| body.clot_and_breach_rates | (per fluid) coagulation and haemorrhage timescales per tick | clot and breach rates |
| body.promotion_shape | (shape) torso mass fraction and base tissue thickness at unit mass, allometric | body promotion parameters |
| body.strike_energetics | (shape) force, velocity, contact area from the wielder's strength and weapon geometry | natural-weapon strike inputs |

---

## §5. Proposed scenario definitions (Venus, Europa) for sign-off

These are the two new scenario files, in the `scenarios/*.toml` schema (direction tokens and postures only, no magnitudes, so nothing is fabricated). They are proposed here for your sign-off; I write them to `scenarios/` only once you approve, at which point the `every_canonical_scenario_resolves` test extends to cover them.

### venus.toml (proposed)

```toml
[scenario]
id = "venus"
name = "Venus"
summary = "A uniformly super-hot world under a dense, toxic, crushing atmosphere: a runaway-greenhouse surface lethal everywhere, life banded high in the cooler cloud decks, buoyant in the dense air and breathing a medium thin in anything respirable and laced with harm. A thin thread of magic. The world the medium substrate stresses hardest: lethal-hot exchange, toxic-poor respiration, and dense-medium buoyancy at once."
grounding = "real"

[races]
count = "few"
diversity = "moderate"
magical_mix = true

[magic]
laws = true
potency = "low_to_moderate"     # present but scarce, a survival edge not an ambient utility
cost = "real"
limit_looseness = "low"
affinity_fraction = "few"
affinity_weight = "moderate"

[dials]
"genome.mutation_rates" = "real"
"genome.point_mutation_rate" = "real"
"genome.mutation_step" = "real"
"genome.selection_scaling" = "high"           # a lethal world selects hard
"genome.effective_population_size" = "low"    # small, isolated cloud-deck pools drift
"genome.speciation_distance" = "real"
"genome.speciation_incompatibilities" = "real"
"language.sound_change_rate" = "real"
"lang.drift_operator_rates" = "real"
"language.innovation_rate" = "real"
"axiom.calcification_rate" = "high"           # constant threat hardens belief and cohesion
"axiom.conformity_prestige_strengths" = "high"
"value_metric.conflict_coefficient" = "high"  # scarce habitable band, contested
"being.life_event_impulse" = "high"
"biosphere.predawn_generations" = "real"

# Environment (proposed [environment]/[medium] blocks, pending §3 graduation to dials):
# field.diffusion = high, field.relaxation = mild, field.body_exchange = high (the dense hot medium cooks)
# physiology.thermal_setpoint = heat-shifted, half_band = widened; medium = dense (buoyancy lifts to the cloud
# decks), low respirable_content (the suffocation case), medium.toxicity > 0 (proposed harm axis)
```

### europa.toml (proposed)

```toml
[scenario]
id = "europa"
name = "Europa"
summary = "A global liquid-water ocean beneath an ice crust, lightless in the deep, warmed and fed at the seafloor by hydrothermal and tidal vents. Life is fully aquatic and chemosynthetic. The world the medium substrate exists for. No magic."
grounding = "real"

[races]
count = "several"
diversity = "high"

[magic]
laws = false
affinity_fraction = "none"

[dials]
"genome.mutation_rates" = "low"               # a buffered, stable world drifts slowly
"genome.point_mutation_rate" = "low"
"genome.mutation_step" = "low"
"genome.selection_scaling" = "real"
"genome.effective_population_size" = "real"
"genome.speciation_distance" = "real"
"genome.speciation_incompatibilities" = "real"
"language.sound_change_rate" = "low"          # placid lexicons (needs the .low sibling, §2b)
"lang.drift_operator_rates" = "low"
"language.innovation_rate" = "low"
"axiom.calcification_rate" = "real"
"axiom.conformity_prestige_strengths" = "real"
"value_metric.conflict_coefficient" = "low"   # resource-stable, low friction (needs .low sibling)
"being.life_event_impulse" = "real"
"biosphere.predawn_generations" = "high"      # deep time for a chemosynthetic vent ecology to radiate

# Environment (proposed [environment]/[medium] blocks, pending §3 graduation to dials):
# field.relaxation = low (near-static under ice), field.diffusion = high, field.body_exchange = high(water)
# physiology.thermal_setpoint = cold, half_band = narrow; medium = dense water, low respirable_content
```

Europa's `low` dials on `language.sound_change_rate`, `lang.drift_operator_rates`, `language.innovation_rate`, and `value_metric.conflict_coefficient` need the `.low` siblings surfaced in §2b; until those are added it falls back to the `real` baseline (still a placid-enough world).

---

## §6. Out of scope of this lever batch (named, not dropped)

The physics floors carry their own reserved values, a separate reservation system from the calibration manifest: the `range_reserved` axis bounds and reserved law constants in `crates/physics/data/*.toml` (mechanics, biology, fluids, chemistry, optics, thermal, and the wave-3 electromagnetism floor). These include the medium and respiratory axes R-MEDIUM added (`fluid.respirable_content`, `fluid.gas_transfer_coefficient`, `bio.respiratory_surface`) and the EM constants (`MU_0`, `K_COULOMB`, the field caps). They are set directly in the floor files, not through the scenario lever system, and they are world-invariant physics rather than per-world dials. They are a distinct owner batch, surfaced separately when you take up the physics floors; §3c pulls forward only the few the two new worlds need. This worksheet is the calibration-manifest (lever-system) batch you asked for; the physics-floor batch is its named sibling.

---

## Summary of what needs your call

- **§2b**: the change-dial magnitudes, mostly the `.high`/`.low` stress ends (the `real` bases are largely set). Recommended defaults given for each, with citations.
- **§3**: the environmental levers, the real differentiators of the two new worlds. Recommendation: promote the thermal band (§3b) and the medium (§3c) to manifest dials so Venus and Europa are first-class levered scenarios; per-world settings recommended.
- **§4**: the world-invariant calibrations, set once. Scalars recommended with anchors; curves and sets described as shapes needing a structured value.
- **§5**: sign-off on the two new scenario definitions, after which I write `venus.toml` and `europa.toml` and extend the resolution test.

Nothing above is written into the manifest, the floors, or the scenario files yet. On your sign-off (per value, per world, or in bulk), each recommended value graduates to `status = "set"` with your name and the date, and the reserved-and-set counts move in step.

---

## §7. Graduation record (2026-07-03): Batches 1 through 11, post-audit

Batch 1 (the deep-time genome stress ends) graduated earlier. Batches 2 through 11 were then run as one whole-batch fully-blind steering and basis audit (the hardened AGENTIC_ADDENDUM §7 method: tiered panel, Opus synthesis over flagged findings, the packet basis fidelity-checked against the manifest, every surviving flag re-verified against source before any change). Of the 42 audited values, 32 were clean and 10 drew a surviving flag: nine were wording, not the number, and one was a magnitude for the owner to re-decide.

On the audit and your sign-off (including "keep it and flag" on the one magnitude), the following graduated to `status = "set"` in `calibration/reserved.toml`, 30 in all (27 existing entries plus 3 new `.low` siblings Europa needs):

- Clean scalars, set as recommended: `genome.mutation_step` (0.1) and `.high` (0.5), `genome.speciation_incompatibilities.low` (1), `language.sound_change_rate.high` (0.10), `language.generation_ticks` (31,536,000), `value_metric.conflict_coefficient` (1.0), `being.life_event_impulse` (0.3), `biosphere.predawn_generations.high` (120), `being.heritable_fraction` (0.5), `evidence.concealment_suppression` (0.5), `physiology.death_floor` (0), `physiology.intake_yield` (0.25, disclosed interim), `behavior.controller_init_spread` (2.0), `behavior.controller_mutation_rate` (0.166667), `behavior.controller_mutation_step` (0.4), `behavior.selection_pop_size` (32), `behavior.selection_generations` (40, set equal to the pre-dawn radiation depth), `body.fracture_damage` (0.9), `body.fluid_critical_fraction` (0.33), `behavior.episode_ticks` (200).
- Basis reworded on graduation (the number unchanged, the ground regrounded per the audit): `language.innovation_rate.high` (0.10, the naming-game overclaim dropped for a plain fast-drift dial), `value_metric.enculturation_pull_rate` (0.01, anchored to an enculturation half-life through the tick-to-year scale), `value_metric.incommensurability_floor` (0.2, restated as an additive projection residual, not a cap), `axiom.evidence_ring_capacity` (8, grounded on the update-dynamics recency depth), `evidence.aggregate_diffusion_rate` (0.02, anchored to a diffusion half-life).
- Steering reworded (the two conflict dials): `value_metric.conflict_coefficient.high` (3.0) and the new `value_metric.conflict_coefficient.low` (0.3), both regrounded on the generated-physics scarcity variable (`conflict_pressure`'s caller-supplied coefficient) rather than a named-world outcome.
- New `.low` siblings Europa needs, set: `language.sound_change_rate.low` (0.005), `language.innovation_rate.low` (0.005), `value_metric.conflict_coefficient.low` (0.3).
- Kept and flagged, on your call: `being.life_event_impulse.high` (0.75 axis-SD, 2.5x the base). The basis carries the flag that this exceeds the attested single-event envelope in the personality-change literature, a deliberate stress-test lever rather than a realism claim.

Held reserved on purpose, surfaced rather than fabricated:

- Entangled with a pending semantic design-change (they graduate with it): `genome.mutation_rates` and `.high` (the structural-rearrangement re-semanticization), `lang.drift_operator_rates` and `.high` (the non-sound-change re-semanticization; the `.low` sibling waits with them).
- Compound or per-axis values with no consumer format yet (a single manifest string cannot represent them without inventing a format their unbuilt reader would have to match): `axiom.calcification_rate` and `.high` (rate, cap, brittleness), `axiom.conformity_prestige_strengths` and `.high` (conformity, prestige, fission and deviation thresholds), `physiology.base_metabolic_drain` and `physiology.exertion_drain_coupling` (per homeostatic axis), `body.burn_scale` (a physics-derived temperature rise from the tissue thermal-damage threshold).

Manifest counts after graduation: 131 entries, 66 set, 65 reserved. Still open in this lever batch: the eleven held entries above (their design-changes and structured-value formats), the §3 environmental levers (thermal band and medium promotion to dials), and the §5 scenario files. The physics floors (§6) remain their own separate owner batch.

---

## §8. Design-changes applied (2026-07-03): the three that unblock the held entries

The three semantic design-changes are now made, which unblocks part of the held set and corrects two double-counts.

1. **`genome.mutation_rates` re-semanticized to the structural-rearrangement rate.** The design already distinguished a per-locus point mutation from a per-genome structural mutation (design 33.9 and Part 25: a structural mutation duplicates, deletes, or rearranges a locus or linkage group, the rare deep-time substrate by which schemes drift). The manifest had already split out the point rate (`genome.point_mutation_rate`) and the step size (`genome.mutation_step`), so `genome.mutation_rates` is the remaining structural rate. Its unit is now `per_genome_per_generation` and its basis names the structural semantics; it graduates at 0.001 per genome per generation (an order below the point-flip rate, so a rearrangement is rare novelty, not every birth), `.high` at 0.02. Two more values set. The scenario dial name is unchanged, so the lever wiring is untouched.

2. **`lang.drift_operator_rates` de-double-counted.** Sound change has its own dial (`language.sound_change_rate`), so the drift-operator set now covers only the other operators: lexical replacement, grammaticalisation, splitting, borrowing. The design blockquote (33.9), the audit reserved list, and the manifest basis are reconciled. The value stays reserved: a four-operator set needs its structured-value format.

3. **The dawn of language grounds on the NSM prime floor, with differentiated founding coverage (resolved as record 62.24 after a blind audit you asked for).** The dawn's anchor set is the NSM semantic prime floor (the primitive axes of 33.1). Your first instinct was that every founding cohort establishes all sixty-five at the dawn; you asked that be blind-audited against the source so the "thin language" property would not be steered. A fully-blind panel (three auditors over the primary sources) found that the NSM universality claim is established for mature languages, not founding events, and that the emergence record shows founding coverage is differentiated and accretes over generations (Nicaraguan Sign Language cohort one lacked THINK and KNOW forms, closing in the second cohort, Pyers and Senghas 2009; the same lag for IF and BECAUSE, Sandler 2011, and in child acquisition, Bartsch and Wellman 1995), while a cohort-size cap that thins the floor is a steer and runs backwards to the evidence (Trudgill 2011). On your sign-off (option one, the differentiated emergent floor): the resilient primes coordinate at the dawn and a lagging subset (epistemic, logical, exact-quantification) closes over generations through one uniform mechanism read from simulated state (community size, transmission depth, contact), never a per-culture rule; each prime carries a data-defined founding resilience; richness above the floor is semantic molecules; the round cap is a determinism safety bound only. Design 33.9, 33.1, 33.10, 33.3, 33.4, the R-LANG and R-LANG-MODALITY records (62.6, 62.13), a new record 62.24, a Part 63 bibliography, and the audit log (block 1z) are all reconciled. New reserved values, surfaced not fabricated: `lang.prime_founding_resilience` (the per-prime rank, data) and `lang.prime_closing_threshold` (the lagging-prime closing threshold); `lang.dawn_round_cap` reworded to the safety bound.

Manifest counts after the design-changes: 131 entries, 68 set, 63 reserved. Still held reserved: the four-operator drift set and its siblings (pending a structured-value format), and the compound, per-axis, and physics-derived shapes named in §7.

---

## §9. Crucible: the war world (2026-07-03), the seventh world

You approved a war world in the spirit of Dune or Warhammer without being either, name my choice. It is **Crucible**, and its defining discipline is that war is never authored: it is the emergent equilibrium of a world whose generated physics leaves too little habitable ground for too many peoples.

**The world.** The habitable ground is a scatter of fertile basins ("greenwells") in a vast lethal waste of ash, salt, and killing heat. Every well is a prize; the waste between is a killing ground crossed only by raiders and the desperate. Many peoples are pressed into few zones with nowhere to expand but into each other, so niche overlap is extreme and the habitable band is always contested. Isolation in the basins breeds many divergent, mutually-unintelligible, hardened peoples. A scarce and dangerous war-magic is one more contested edge, the thinnest posture that still installs `MagicLaws` (present, costly, tightly limited, carried by few), so magic reads as a rare and deadly advantage rather than an ambient utility.

**Why it is not steering.** Nothing sets "this world has war." The constant conflict falls out of the physics: `conflict_pressure` (`value.rs`) reads a high coefficient because the generated world carries scarce, contested range and extreme niche overlap, the regime keyed on the generated scarcity physics rather than on the world's name (the audit-regrounded basis, §7). The dials crank the emergent drivers of war, not war itself.

**Postures:** `races.count` many (many peoples, few zones), `races.diversity` high (isolation breeds divergence), `races.magical_mix` yes; `magic.laws` true, `magic.potency` low-to-moderate, `magic.cost` real, `magic.limit_looseness` low, `magic.affinity_fraction` few, `magic.affinity_weight` moderate.

**Dials.** Cranked high: `genome.selection_scaling` (harsh selection), `genome.effective_population_size` low and `genome.speciation_distance` low (small basin pools drift and radiate fast into many distinct peoples), `language.sound_change_rate`, `lang.drift_operator_rates`, `language.innovation_rate` high (basins diverge into mutual unintelligibility), `axiom.calcification_rate` and `axiom.conformity_prestige_strengths` high (war hardens creeds and the prestige of arms), `value_metric.conflict_coefficient` high (the scarce contested range earns it), `being.life_event_impulse` high (violence and loss ripple hard). Held at real: the genetic mutation clock (`mutation_rates`, `point_mutation_rate`, `mutation_step`) and `biosphere.predawn_generations`. That is the distinction: Crucible cranks the social and divergence dials, not the mutation churn, so it is neither Tempest (every change dial high, including mutation) nor Venus (environmental lethality rather than contest).

The scenario file is `scenarios/crucible.toml`, and the resolution and load tests now cover it alongside Venus and Europa. Crucible's scarce-fertile-ground physics is a resource-distribution property beyond the §3 thermal-and-medium levers; it is carried today through the dials and a harsh (not lethal-everywhere) field, with a resource-patchiness lever a natural later addition.

Manifest counts after the scenario-file pass: 135 entries, 68 set, 67 reserved (the four reserved `.low` genome and drift siblings Europa's placid dials resolve to were added, surfaced not fabricated).

---

## §10. The structured-value format, and the compound held entries graduated (2026-07-03)

The compound entries §7 held (a set of operator rates, a bundle of named strengths) needed a value shape a single manifest string could carry and a consumer could parse. That format now exists: `CalibrationManifest::require_map` (`crates/sim/src/calibration.rs`) reads a `"key1=v1,key2=v2"` string into a deterministically-ordered `BTreeMap<String, Fixed>`, each value taking the same exact decimal-to-fixed path as `require_fixed`, the membership growing with the data rather than being fixed in code (Principle 11). It fails loud on a reserved, malformed, empty, or duplicate-keyed entry. Tested against both a fixture and the real manifest.

On that format the fully-specified compounds graduated, and the compounds with an unspecified component were split so the specified parts graduate and the unspecified stay reserved, surfaced rather than fabricated:

- **`lang.drift_operator_rates`** and its `.high` and `.low` siblings graduated as maps of the four non-sound-change operators (lexical replacement 0.01, grammaticalisation 0.005, splitting 0.002, borrowing 0.01; the stress siblings 5x and 0.25x each). Sound change stays its own dial.
- **`axiom.calcification_rate`** split into the rate (0.001, the dial the worlds push) with its `.high` (0.004), plus a new set `axiom.calcification_cap` (0.95). The brittleness (rising with near-miss challenges, a direction the design gives without a magnitude) is a new reserved entry `axiom.calcification_brittleness`.
- **`axiom.conformity_prestige_strengths`** and its `.high` graduated as maps of the two strengths (conformity 0.1, prestige 0.1; the `.high` 2x each). The fission and deviation thresholds, keyed on group stance-variance with no magnitude given, are new reserved entries `axiom.fission_threshold` and `axiom.deviation_threshold`.

Still held for a component value the design does not fix: the per-axis physiology drains (`physiology.base_metabolic_drain`, `physiology.exertion_drain_coupling`), whose energy-axis rate is given (~1/300 and ~1/400 of capacity per tick) but whose water and other axes are only described as "slower" and "weakly," so they wait on the per-axis magnitudes; and `body.burn_scale`, which is derived from the tissue thermal-damage (protein-denaturation) threshold in the physics floor rather than being an owner-picked scalar, so it graduates from the floor data, not here.

Manifest counts after the compound pass: 141 entries, 76 set, 65 reserved.

---

## §11. The environmental levers promoted (2026-07-03): the field and the thermal band

§3a (the temperature field) and §3b (the per-race thermal band) are now first-class levers, resolved through the same direction-token mechanism as the change dials. A scenario carries a new `[environment]` block (`crates/sim/src/scenario.rs`, `Scenario::environment`) whose entries push `field.*` and `physiology.thermal_*` levers by the same `real`/`high`/`low` tokens, resolved by `Scenario::resolve` alongside the change dials and carried into the same review queue, so a world's environment is levered and calibration-gated exactly like its change engine, and a dangling environment reference fails loud the same way.

The manifest gained the sibling and thermal-band entries, all **reserved** (surfaced with the recommended value in the basis, not set: §3 was not part of the ratified change-dial batches, so the promotion adds the plumbing and the magnitudes stay yours to set): `field.diffusion.high` (~0.22, dense-medium diffusion below the 0.25 stencil bound), `field.relaxation.low` (~0.005, near-static under ice), `field.body_exchange.high` (~0.20, fast body-to-medium coupling), and the per-race thermal band `physiology.thermal_setpoint` (~310 K base, `.high` ~340 K thermophile, `.low` ~275 K psychrophile) and `physiology.thermal_half_band` (~8 K base, `.high` ~15 K widened, `.low` ~6 K narrow).

`venus.toml` and `europa.toml` carry `[environment]` blocks: Venus levers a dense diffusive field, fast body coupling, and a heat-shifted widened band; Europa levers a diffusive but near-static (under-ice) field, fast immersion coupling, and a cold narrow band. The four canonical worlds carry no `[environment]` block, so temperate is the unlevered baseline. The resolution test extends to cover the environment, and a dedicated test confirms both worlds resolve their environment against the real manifest and surface the environment magnitudes in the review queue.

Remaining in §3: the medium (§3c, gill respiration, buoyancy, the toxicity harm axis). Unlike the field and thermal band, a world's medium is a physics `Substance` selected categorically (air, water, dense-toxic atmosphere) from the floors (`crates/physics/data/*.toml`), not a `real`/`high`/`low` dial, and `medium.toxicity` is a proposed floor extension. That is a separate, larger increment (a categorical medium selection plus the harm-axis extension), the next step to make Venus's toxic-poor respiration and Europa's gill breathing first-class.

Manifest counts after the environment-lever pass: 150 entries, 76 set, 74 reserved.

---

## §12. The medium lever (2026-07-03), the last §3 piece

The medium (§3c) is the ambient physics `Substance` a world's life breathes and floats in. Unlike the field and thermal band, it is not a `real`/`high`/`low` dial: a medium is a coherent bundle (water is dense and low in dissolved gas together), so it is selected categorically by name. A scenario's `[scenario]` block carries a `medium = "water"` key (alongside `grounding`), which `Scenario::resolve` resolves to the manifest profile `medium.{name}`, a `require_map` bundle of the medium's axis values, carried into the same review queue as the dials and the environment levers. A world naming a medium with no manifest profile fails loud like a dangling dial.

Three reserved medium profiles were added, the recommended axis map in each basis (surfaced, not set):

- **`medium.air`**: the temperate default (Mirror and the grounded worlds name no medium and fall to it). Recommended density ~1.2, respirable_content ~9 (an air-oxygen analogue), toxicity 0, in_medium_exchange_rate ~0.05.
- **`medium.water`** (Europa): density ~1000, respirable_content ~0.3 (low dissolved gas, so a large gill surface is needed), toxicity 0, exchange ~0.20. Buoyancy is load-bearing (a dense body sinks to the vents, a light one rises to the ice).
- **`medium.dense_toxic`** (Venus): density ~65 (buoyancy lifts a light body toward the cooler cloud decks), respirable_content ~0.1 (the suffocation case), toxicity greater than 0 (the acid-and-carbon-dioxide air corrodes, the proposed harm-axis extension), exchange ~0.20 (the dense hot medium cooks).

`venus.toml` selects `dense_toxic`, `europa.toml` selects `water`; the four canonical worlds name none and default to air. A dedicated test confirms both worlds resolve their medium against the real manifest and surface the profile in the review queue, and that a bogus medium fails loud.

Two honest limits stay open: `medium.toxicity` is a proposed floor harm-axis extension, so a profile can carry the value but the kernel that applies toxicity harm to an immersed body is a future floor build; and the per-organ `respiratory_surface` scale is a separate reserved value, not part of the medium profile. With the field, the thermal band, and the medium all levered, §3 is complete as plumbing; the magnitudes are the reserved `[environment]`-and-medium dials to ratify.

Manifest counts after the medium pass: 153 entries, 76 set, 77 reserved.

---

## §13. The derive-from-fundamentals audit (2026-07-03): what supersedes the graduations

After the batches graduated (§7 through §12), the whole 153-entry manifest was swept a second time under a sharper lens: not "what number does this take," but "should this be a set number at all." The governing rule, your standing one, is Principle 9 read strictly. A value that can be computed from a deeper invariant (a physics law, information theory, the genome's own variance structure, the substrate geometry, a per-race datum already authored) should derive, so a non-human race gets its own value for free. A value whose recommended magnitude anchors on a human or terran number (a twin-study heritability, a mammalian blood-loss fraction, Miller's memory span, WALS word-order proportions, an Earth calendar year) is a steer: authoring it stamps one body's or one culture's particular onto every race and collapses the divergence Principle 8 demands. Only an irreducible world-character knob that names no human particular is a legitimate set value, and those go to the world-dialing agent (§14), not into a world-invariant default here.

The sweep ran as two source-grounded fan-outs (one over the physics kernel's derivation capability and the physiology and body values, one over every other subsystem, each agent reading the real mechanism and `crates/physics/src/laws.rs` and citing file and line) plus the recovered whole-batch blind Steering Audit of §7 (five independent auditors, which converged on the same human-anchoring cluster). Where the audit conflicts with a §7 through §12 graduation, the audit is the later decision and governs, per your instruction that the recent chatlog wins on conflict.

Result across the 128 manifest values: 46 derive from a fundamental, 40 are irreducible world-levers, 9 are per-race or per-substance constants, 9 are engine budgets, and (with the physiology and body set folded in) roughly a dozen more derive or are substance constants. Seventeen carry a HIGH human or terran steer, twenty-seven a LOW one. Nothing below is applied to `calibration/reserved.toml` yet; each row is staged for your sign-off exactly as the original batches were.

### §13a. The overturn: graduated values that revert on the audit

These are `status = "set"` today (graduated in §7 through §12) but the audit finds each either anchors on a human or terran number or has a derivation already sitting in the tree. On the chatlog-wins rule they revert from set to derive (or to per-race data), each with the fundamental it derives through. The HIGH-steer rows are the fix-first list.

| Graduated value (was set to) | Steer | Reverts to | Derives through (file:line) |
|---|---|---|---|
| `genome.narrow_sense_heritability` (0.5) and `being.heritable_fraction` (0.5) | HIGH | derive, a readout not an input | h² = V_A/(V_A+V_E) from the `express()` variance decomposition (genome.rs:324-346); 0.5 is the human twin-study midpoint and contradicts the value's own "must not be fabricated into a constant" basis |
| `physiology.intake_yield` (0.25, "interim") | HIGH | derive | `laws::edibility` / `net_nutrition` over the eaten matter's composition (laws.rs:118-213); needs `ResourceField` to carry a `Composition` per tile. The design already calls it interim (R-PHYS-BIO) |
| `time.life_cadence_ticks` (31,536,000) | HIGH | derive (Earth value kept as flagged interim only) | world `orbital_period_seconds / base_tick_seconds` once R-CELESTIAL orbital elements exist; 31,536,000 is Earth's year, and Venus and Europa do not orbit on it |
| `value_metric.incommensurability_floor` (0.2) | HIGH | derive per race-pair | the measured etic projection loss for that pair (`project_to_etic` drop, value.rs:377-389), not a flat floor on every pairing. Matches your lean toward no fixed floor |
| `axiom.evidence_ring_capacity` (8) | HIGH | per-race constant | the race's Part 20 memory datum through a monotone map; 8 structurally echoes Miller's 7±2. Reserve only a hard replay ceiling separately as a budget |
| `body.fluid_critical_fraction` (0.33) | HIGH | per-fluid substance constant | a per-fluid circulatory-physiology datum in the biology registry (sibling to `tissue_properties`); 0.33 is the mammalian (Guyton and Hall) figure applied to every race and fluid |
| `gossip.trust_baseline` (0.5) and `gossip.trust_penalty` (0.5) | HIGH | per-race constant | per-race epistemic-stance data, or derive from the race's reciprocal-altruism structure; 0.5 is the Berg/Dickhaut/McCabe US-college investment-game figure. Human value only as the human-analogue default |
| `axiom.calcification_rate` (0.001/tick) | HIGH | re-express, then lever | a dimensionless "lifespans-to-harden" divided by each race's lifespan datum; the absolute per-tick rate bakes in a human-generation clock. The dimensionless lever then goes to §14 |
| `genome.effective_population_size` (200) | LOW | derive canonical, lever the founder seed | Ne from census, sex ratio, and reproductive-success variance (drift(), genome.rs:604-626); route only the pre-dawn founder seed (30) to §14 |
| `language.generation_ticks` (31,536,000) | LOW | derive | the race's maturation age (R-AGING) times `base_tick_seconds`, not one Earth year |
| `value_metric.enculturation_pull_rate` (0.01) | LOW | derive | each race's `belief_plasticity`, mirroring `axiom.enculturate`'s per-mind stubbornness (axiom.rs:587-591); a second flat cross-race constant would regress against the sibling already built |
| `evidence.aggregate_diffusion_rate` (0.02) | LOW | derive | a mean-field of the individual-tier gossip parameters and contact density, calibrated to the tier-consistency invariant |

A second cluster of set values reverts for a Principle-11 single-source-of-truth reason rather than a steer: the theory-of-mind access weights (`tom.access_weight.witnessed`/`told`/`said`/`absence`, `tom.meta_commit_threshold`, `tom.meta_runner_up_margin`, `tom.meta_log_odds_clamp`) and `gossip.told_weight` should alias to the `evidence.*` and meta thresholds by the formula their own basis names, not carry independent literals that can drift; `genome.recombination_default` (0.5) is the Mendel independent-assortment limit, a law constant to tag as derived rather than a knob; `genome.allele_presence_threshold` (0.9) derives from the Hardy-Weinberg contribution; and the evolved-controller values (`behavior.controller_init_spread` from Xavier/He scaling, `controller_mutation_rate` from target-mutations over weight count, `controller_mutation_step` as a fraction of the clamp width, `selection_generations` as a reference to `biosphere.predawn_generations`, `episode_ticks` as a small multiple of capacity over base drain) derive from their own mechanism rather than standing as free numbers. These are lower priority than the steers but tighten the manifest to one source of truth per quantity.

### §13b. The still-reserved HIGH steers to de-anchor before they ever graduate

These were held reserved in §7, so there is no graduation to overturn; the audit fixes their basis now so a human or terran number never becomes the ratified value. Each re-points to a per-race or physics derivation, with the human data kept only as the human race's own data row.

`lang.phoneme_priors` derive from the race's producible-sound set, its `perceptual_geometry`, and its `capability_gates` (language.rs:186-243 already reads only the per-race producible set); UPSID/PHOIBLE becomes the human row, never the global prior. `lang.typology_harmony_strong` and `_weak` derive from a parse-and-processing-cost principle gated by the modality's simultaneous-versus-sequential flag; the WALS 95A and Dryer 1992 proportions become the human data row. `langmod.perceptual_geometry` derives per-modality confusability from channel physics (`speed_of_sound`, medium coupling, laws.rs:1103-1172) composed with the race's own sensorium, not the human auditory target. `lang.l2_acquisition` derives its age breakpoint from the race's own neuro-maturation (`being.plasticity_by_age`, R-AGING), not the human "late teens." `evidence.implication_weights` compute at runtime from Good's likelihood ratio over the Part 40 trace-kind registry, replacing the "corpse 5, stain 1" forensic table. `evidence.trace_decay_curves` derive each trace's persistence from the material's decomposition and reaction susceptibility through the reaction and corrosion kernel (laws.rs:1298-1341), replacing "blood days, bone years." `evidence.absence_windows` derive from each race's mortality hazard curve (demography.rs:56-64) and its visibility interval, dropping the seven-year Anglo-American death-in-absentia anchor. `inst.function_substrate_axes` cluster the race's own recurring need-vectors (Parts 21, 15, 19, 8), keeping the human five (force, the sacred, exchange, knowledge, care) only as the human-analogue fallback and carrying the R-BIO-REGISTRY-style circularity flag the design gives elsewhere. `being.plasticity_by_age` and `being.maturity_targets` stay per-race `TraitDef` data (the architecture is sound, per-race, not a fixed Big Five); the recommended defaults (rising rank-order stability, rising conscientiousness, falling novelty-seeking) are the human maturity principle and are the steer as the only worked example, so the inflection derives from each race's `maturity_years`/`lifespan_years` and only the residual shape is reserved. `value_metric.etic_substrate_axes` grow bottom-up from axes that recur across the world's per-race emic data rather than a hardcoded human moral-psychology set (record 62.3 states outright that no single human structure can be hardcoded as universal). One flag for you on the last: design.md:1799 feeds the etic substrate into `conflict_pressure` and theory-of-mind friend-or-foe judgment, which sits in tension with its description as "never a behavioural input"; worth a look before it graduates.

### §13c. Honest limits: where "derive" is aspirational today

Several derivations name a law the kernel does not carry yet, so the honest interim is the human data as a flagged row, not a ratified universal: the typology harmony values need an information-processing parse-cost law gated by modality (not in laws.rs); `langmod.perceptual_geometry` has `speed_of_sound` but no attenuation or formant law; `time.life_cadence_ticks` and `language.generation_ticks` need R-CELESTIAL orbital elements (not wired); `inst.function_substrate_axes` need-vector clustering depends on Parts 36, 41, and 54, which are design-stage; the trust values prefer a reciprocal-altruism substrate that does not exist; and `evidence.aggregate_diffusion_rate`, `tier.decision_propensity`, and `tier.belief_level_to_strength` derive against tier-consistency mechanics not yet built. In each case the audit names the missing law rather than pretending the derivation runs today, and the human-anchored number stays as an explicit interim, not a silent default.

---

## §14. The world-dialing-agent handoff (the authored lever list)

These are the irreducible world-character knobs. Each names no human particular; each is a legitimate authored choice about how a world behaves, and each is reserved for the separate world-dialing agent that composes worlds from the authored lever list, set per world (through the `real`/`high`/`low` dial mechanism or a categorical selection), never derived and never set as a world-invariant default here. Forty in all, grouped.

**Change, drift, and tempo rates.** `genome.mutation_rates`, `genome.point_mutation_rate`, `genome.mutation_step`, `language.innovation_rate`, `language.sound_change_rate`, `lang.drift_operator_rates`, `inst.crystallization_rates`, `axiom.conformity_prestige_strengths`, and `axiom.calcification_rate` (as the dimensionless lifespans-to-harden lever after §13a's re-expression). How fast a world's genomes, languages, and institutions churn is its defining character, not a computable quantity. Two riders: drop the human-historical-pace justification from the language "real" baselines (keep the number, change the ground), and add `inst.crystallization_rates` to the fifteen-dial matrix as the sixteenth dial.

**Selection, drift, and speciation intensity.** `genome.selection_scaling` (breeder's-equation hardness), `genome.speciation_distance`, `genome.speciation_incompatibilities`, `genome.fertility_curve` (the taper steepness, its inflection anchored to `speciation_distance`), `biosphere.predawn_generations`, and the `genome.effective_population_size` founder seed. How hard selection bites and how discrete the reproductive firewall is are irreducible knobs over race-neutral distance and count metrics.

**Epistemic and behavioural magnitudes.** `evidence.commit_threshold` (Wald SPRT), `evidence.runner_up_margin` (Lipton/Harman), `evidence.log_odds_clamp` (the defeasibility ceiling), `evidence.concealment_suppression`, `tom.access_weight.reachable`, `tom.access_weight.denied`, `langmod.blend_propensity`, `langmod.mismatch_triggers`, `lang.typology_disharmony` (the softmax temperature on harmony pressure), `lang.writing_invention_threshold` (the pressure and literacy parts, split from the record-decay part which derives), and `time.years_per_generation` (presentation-only, never canonical state). These are general decision-theory or dynamics results, or mechanism-strength knobs, that name no human biology. One fidelity rider, not a steer: `agent.rs:190` passes one shared `InferenceParams` to every mind, so the three `evidence.*` thresholds and the `tom.*` meta thresholds should carry a per-race or per-individual epistemic-stance axis riding on the world baseline, so paranoid and credulous minds diverge.

**Stress-end siblings (.high/.low).** Every `.high` and `.low` sibling is a bounded self-referential multiplier of its base and is a world-lever by construction: the genome, language, drift, axiom, conflict, life-event, population, speciation, and pre-dawn ends. Rider: where the base is human-anchored (calcification, conformity, life-event impulse), the race-authoring step must set a per-race base for a non-human-typical mind, so the stress end scales a per-race value and not the terran default.

**Environment and medium levers (from §11 and §12).** The `field.*.high`/`.low` siblings, the per-race thermal band (`physiology.thermal_setpoint`/`thermal_half_band` and their siblings), and the categorical medium profiles (`medium.air`, `medium.water`, `medium.dense_toxic`). These are what make Venus scorched and Europa an ocean; they are levers or physics-substance selections, owned by the world-dialing agent, with the thermal band and the medium content grounded in the physics floors rather than fabricated.

---

## §15. Scale-dependent values, now settable

You noted the scale-gated tier can now be set. With the world scale fixed, the values previously deferred as "resolves against a scale not yet chosen" can graduate, with three destinies rather than one. Some are engine budgets the owner sets from measured cost: `compose.max_depth` (about 6 levels), `compose.reuse_compression_threshold`, `behavior.selection_pop_size` (32), `behavior.controller_hidden_width` (already set at 4), and `lang.dawn_round_cap` and `langmod.dawn_cap_unitsize` (keep the round cap as a determinism-and-performance bound, but derive the embedded anchor-set from per-race cognitive capacity or drop the Miller-7 coincidence and call it a pure buffer). `genome.loci_per_channel` becomes a per-race genome-architecture property capped by a compute budget (coupled to the derived heritability). `tier.significance_thresholds` ties to the Part 54 conservation invariant rather than an independent magnitude. The scale-dependent values that the audit derives do not get set here even though the scale is known, because deriving them is the stronger fix: canonical `genome.effective_population_size` (from demography), `behavior.episode_ticks` (about 60, from capacity over base drain), and `behavior.selection_generations` (a reference to the pre-dawn depth). And `biosphere.predawn_generations` and the `effective_population_size` founder seed are world-levers, so they go to §14. So "now settable" resolves cleanly: the budgets and per-race architecture graduate, the derivable ones derive, and the world-character ones hand off.

---

## §16. The race-seed recipe: authoring a starting race without templating

You asked how a named starting race (dwarves, say) fits without becoming a template. The derive-from-physics reframe of §13 is exactly what makes it work: a race is a minimal physical seed you let run, and because physiology derives from the body against a universal substrate, a differently-seeded body produces a different physiology of its own and lets a different culture emerge, or not. If the physiology were the human-anchored constant the audit just overturned, a dwarf seed could not diverge, because every race would share one authored body. Deriving from a universal substrate and making a non-human seed possible are the same act.

A race is a data seed of physics and disposition only, a selection over the world's registries with per-race magnitudes, never a subclass. Six authorable layers, all grounded in the built code (`race.rs`, `genome.rs`, `anatomy.rs`, `homeostasis.rs`, `edibility.rs`, `sensorium.rs`, `genesis.rs`): the genome (a `GeneSet` whose genes feed fixed phenotype channels, sampled by Hardy-Weinberg at the dawn); the body plan and anatomy (each organ carries a tissue composition over the biology floor, its function derived from that composition, never a tag, so an energy-dense tissue is a store because it is energy-dense); the physiology, derived not authored (reserve capacity is already the development-weighted organ composition against the floor); edibility and consumer physiology (measured, never stored, so the same organism is food to one race and poison to another with no authored verdict); the sensorium and producible sounds (the physical channels and the sound inventory the anatomy can form, the phonology a later cultural selection from it); and the environment and placement (a band seeded onto a worldgen place with its abiotic sources). The one permitted non-physical input is the dawn intrinsic-belief seed, a Principle-9 cultural input (a starting prior), never a cultural outcome, and never re-imposed after the dawn.

The dwarf, then, is a point in the same physical space as a wolf or an elf. You author the physics and the disposition: organs weighted to a dense, mineral-and-protein composition and a modest body mass (so mass-per-volume is high and absolute size small); genes feeding the strength and climate-tolerance channels at high pool frequency, tuned to a low-temperature, low-light envelope; a high-development vision sense at high acuity or a night-vision gene; a subterranean placement whose environment carries no light source; life-history genes for a long-lived, slow-breeding trait; and a modest intrinsic-belief prior. You never author that dwarves are greedy, clannish, or industrious, or that they eat mushrooms or carve stone; those are templated culture and forbidden. Dwarf-ness in the body derives (reserve capacities from the dense composition, metabolism from that mass and tissue through the allometric law once it lands, edibility from the composition against each consumer's physiology), and dwarf culture, values-in-practice, language, technology, and institutions emerge from those primitives plus the subterranean niche plus selection plus history. No dwarf-specific engine code and no culture template exists anywhere in the seed.

The affordances the design still needs to make this land without fabrication are the same ones §13 named as missing laws: a metabolic-rate kernel that computes the drain from body mass and tissue energy through Kleiber's law (the law is reserved, the kernel is not built), a whole-body convective surface-area axis in the biology floor (both `base_metabolic_drain` and `field.body_exchange` block on it), the wiring of `edibility::assess` into ingest (a `Composition` per resource tile), reading the floor maxima at runtime for `damage_caps` and the floor registry per material for `tissue_properties`, a per-tissue denaturation axis for `burn_scale`, a poiseuille-based bleed path and a split of the conflated clot-and-breach id, a per-organ mass-share field for `promotion_shape`, and a solar and orbital substrate before `field.relaxation` and `life_cadence_ticks` can derive. The honest limit stands plainly: a seed sets starting conditions, it does not guarantee the archetype. The dwarf seed fixes bodies, allele frequencies, a niche, and a prior; two runs from it can end in different cultures, and that is the design working as specified. Two further limits are load-bearing: the behaviour policy must evolve rather than be authored, so a seed can bias temperament only through body and prior, and the scoring environment for evolved behaviour is itself an authored input that can imprint a behaviour the world did not, so the seed's physics must be both body-derived and behaviour-neutral for the divergence to be honest.

---

## §17. What was applied to the manifest (2026-07-03)

On your sign-off the audit was applied to `calibration/reserved.toml`, in batches, keeping the build green (the full sim suite passes, 390 lib plus every integration test). The manifest moved from 76 set to **68 set, 85 reserved** across 153 entries. What changed:

**Batch A, eight steers reverted from set to reserved (the fail-loud derive sentinels).** `being.heritable_fraction` and `genome.narrow_sense_heritability` (the emergent h² readout), `physiology.intake_yield` (the edibility measure), `body.fluid_critical_fraction` (per-fluid), `value_metric.incommensurability_floor` (per-pair projection loss), `value_metric.enculturation_pull_rate` (per-race belief plasticity), `evidence.aggregate_diffusion_rate` (the tier mean-field), and `axiom.evidence_ring_capacity` (per-race memory). Each now carries a `DERIVE, worksheet §13` basis and an empty value, so `Profile::Calibrated` refuses to run on it until the derivation is wired, which is the correct state. Safe because none of the eight is read by code today (they were graduated but never consumed).

**Batch B, five steers kept set with a flagged basis** because a working system reads each through a hard `require_fixed` and reverting would break the build, or because the value is a dial base. `gossip.trust_baseline` and `gossip.trust_penalty` (read by `GossipParams`), `time.life_cadence_ticks` and `language.generation_ticks` (the Earth-year interims the audit itself keeps until R-CELESTIAL and R-AGING land), and `genome.effective_population_size` (the founder-seed dial base). Each basis now names the steer and the derivation or lever it awaits. `axiom.calcification_rate` is also kept set here rather than reverted, because it is a dial base worlds push; its basis flags the dimensionless-per-lifespan re-expression it needs.

**Batch C, twelve still-reserved steers de-anchored in basis.** `lang.phoneme_priors`, `lang.typology_harmony_strong` and `_weak`, `lang.l2_acquisition`, `langmod.perceptual_geometry`, `evidence.implication_weights`, `evidence.trace_decay_curves`, `evidence.absence_windows`, `inst.function_substrate_axes`, `being.plasticity_by_age`, `being.maturity_targets`, and `value_metric.etic_substrate_axes`. These were already reserved, so no status moved; the basis now carries the per-race or physics derivation and marks the human data (WALS, Dryer, UPSID, Miller, Guyton, the maturity principle) as the human race's own data row rather than the global default.

**Not applied, and why.** The roughly eleven single-source-of-truth rows (the `tom.*` access weights, `gossip.told_weight`, the `behavior.*` controller values, `genome.recombination_default` and `allele_presence_threshold`) stay set: aliasing them to the value their basis names is a code refactor (the manifest cannot carry a formula), and reverting them would break the systems that read them for no gain. They are recorded in §13a for that refactor. The forty §14 levers stay set or reserved as they were, now owned by the world-dialing agent; `axiom.calcification_rate` and `genome.effective_population_size` carry the `world-dialing lever, worksheet §14` tag in their `source`. The §15 scale-dependent budgets are not graduated, because setting a number is yours, not the agent's (Principle 3); they are surfaced with their recommended value for you to ratify.

**Where it leaves the manifest.** The set-once world-invariant tier is now much smaller and cleaner: the genuine engine budgets, the deterministic tie-breaks, the defined units, and the flagged interims that a working system still reads. The eight reverted values are fail-loud until their derivation is built (the metabolic and edibility wiring, the h² variance readout, the per-race memory and projection-loss reads), which is the honest state: the engine will not run on a human number standing in for a derivation. The next builds are those derivations and the single-source-of-truth refactor; the world-levers are ready for the world-dialing agent's list.
