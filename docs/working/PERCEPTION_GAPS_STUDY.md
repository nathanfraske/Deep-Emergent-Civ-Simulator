# Perceive-to-learn substrate-gap study: findings and reconciliation

This is the report of the exploratory research workflow that mapped, stage by stage, how a creature
perceives things to learn, and looped each stage to its bedrock to classify where perception bottoms out.
It is doc-only: no code was written to produce it. Every claim carries its `file:line`. The companion
`PERCEPTION_SUBSTRATE_ARC_PLAN.md` turns these findings into a derive-first design proposal for the gate.

## Method, and the honest limits of the method

Seven mappers each mapped one stage of the pipeline (signal genesis, propagation, reception, quantization,
belief formation, felt value, channel assignment and temporal sampling), each looping up to three rounds down
the derive chain and classifying every value the live path reads into one of six categories: a floor read, a
world/race datum, an engine bound, an authored simplification of a substrate that exists but is unwired, a
missing physics substrate, or a missing engine model. Four adversarial lenses then tried to refute each
classification (a floor-axis derive check, an absence proof by grep, a live-path-reads-the-floor check, and an
alien/Terran check), each defaulting to refuted. A synthesis and a completeness critic closed it out.

The construction itself was audited before any finding was trusted (the section-11 input-bias smoke test, on
the strongest model, fail-closed). Its first run was caught and killed on substantive grounds: the finding
schema was binary (missing-substrate versus legitimate-floor) with no bucket for the real state of the
thermal, optical, and acoustic channels (a deriving law that exists but is unwired), and the mappers were
handed a prior map that already asserted the gaps. The construction was reworked (a five-way classification, a
per-value enumeration, the prior map's conclusions stripped, the scopes de-loaded, a fourth verify lens added,
the learning stage re-seeded with the existing `base_rates.rs` so it would re-verify rather than presume
absence) and re-run. The second smoke test cleared it (minor issues, not blocking), with residual caveats
carried below. One verify agent failed its retry cap, so a single lens on one stage is unverified; noted where
it bears.

## The core finding

The live perception-to-learning path is Path B, the associative learner in `step_embodiment` /
`couple_conversation` (`runner.rs:4284-4361`). Its two emergence-critical bottoms already derive clean, and
clean under the alien lens:

- The felt value SIGN keys on no hazard label, dose threshold, kind, or race id. Harm is a conserved reserve
  fallen beyond a floor, reward a reserve risen (`runner.rs:4296-4341`, `homeostasis.rs:1089-1107`), read
  generically over the being's own reserve axes. A photosynthetic, redox, mana, or silicate perceiver adds its
  own reserve axis as data and feels harm from its own delta; an alien-clean unit test names a light-charge, a
  silicon heat store, and a mana reserve (`physiology.rs:679-714`).
- The raw percept is the true physical quantity in the being's own cell: `Composition.sensed` reads the
  biology-class dose (`percept.rs:129-134`), `SubstanceMix.volume` the material volume by conservation
  (`material_percept.rs:117-125`), and `feature_observations` correlates the raw feature with the being's own
  interoceptive harm bit, so the belief emerges from correlation with "nothing read but the raw feature and the
  felt sign (Principles 8, 9)" (`learn.rs:504-526`).

Path A, the `Trace` / `salience` machinery the design document describes, is not live: `emit_trace` has no
production caller (every caller is under the test module `world.rs:4657-5781` or the `dawn_band` example), so
its authored magnitudes are scaffolding, not a live blocker. That single fact reshapes the ranking: most of
the authored perception values sit in Path A and do not run today.

## The should-derive bottoms, ranked most load-bearing first

Every top item is a wiring gap to a substrate that already exists, not a missing physics law. The one deepest
is a coupled pair at the head of the live learning signal.

1. The harm and reward likelihoods (`learn.rs:288-289`, dev 0.9 / 0.1). The live observation weight is
   `good_weight(p_harm_given_harms, p_harm_given_benign, clamp)` (`learn.rs:315`), I.J. Good's weight of
   evidence over two probabilities. The composition is derive-clean; the two probability inputs are authored
   scalars. They set how much perceiving X teaches about X on every learning observation, so the learning RATE
   from every percept bottoms on two authored numbers. The deriving substrate (the floor dose-response harm law
   plus the metabolic-noise distribution of the reserve delta) exists, but wiring it needs an ESTIMATOR of
   P(signal given cause), not a read.
2. The harm and reward noise-floor sign gates (`runner.rs:4299`), one step upstream, which set the label an
   observation carries. One flat reserved scalar is applied to every axis of every being, whereas its own
   stated basis (the resting per-tick metabolic drain per axis) exists live as `DerivedDrain.base`
   (`runner.rs:853-937`), a near one-line read.
3. `feature_granularity` / the sensorium JND (`percept.rs:150-157`), which sets the belief subject, hence what
   a being can distinguish and generalize. The deriving substrate is `Sensorium::resolution`
   (`sensorium.rs:75,134`), fed by a per-race datum and already wired to the language path (`langmod.rs:162`),
   but not to the feature-quantization path; the block is a missing percept-class-to-channel binding.
4. `base_rates` plus trace reliability plus the flat prior (`base_rates.rs`, `trace.rs:166-269`), the largest
   unwired substrate by volume. `seed_prior` has no live caller (`evidence.rs:145`, referenced only in docs and
   tests, confirmed by grep), and the live learner seeds a flat prior (`evidence.rs:118`).
5. `commit_threshold` and `runner_up_margin` (`world.rs:997`), one world-global boundary shared by every mind,
   though a per-mind `EpistemicStance` substrate exists unwired (`axiom.rs:257-266`).
6. `channel_acuity` and the no-sensorium `Fixed::ONE` fallback (`world.rs:3965`), which currently make the
   channel gate a live no-op because no sensorium is installed in the canonical runner
   (`set_sensorium` is test-only, `world.rs:5750`). The deriving substrate is `GeneSet::express`, the same
   machinery that already produces `mind.acuity`.
7. `eligibility_decay`, the reward TD lambda (`learn.rs:394`). This is a genuine missing modelled quantity (not
   a floor axis): its grounding, an interoceptive action-to-reserve latency, is modelled nowhere; reserve
   dynamics are immediate per tick.
8. Per-being temporal sampling resolution (none exists): every being samples at the one global tick
   (`world.rs:2731`), so a slow integrator and a fast one cannot differ. It needs a temporal-JND datum on the
   sensorium, a modelling decision, not a floor axis.
9. The quantization grid shape (uniform linear versus Weber-log), which does not bite while the JND is authored
   as a constant absolute step; no Weber-Fechner substrate exists anywhere.
10. The channel-identity token (`sensorium.rs:42-48`), a bare `u32` with no physical quantity bound to it.
    Alien-clean by construction (a novel sense is one data row; Muller's law of specific nerve energies), so it
    is the least of the "gaps".
11. The Path A structural story: `Trace.salience`/`weight`, the unwired reach laws, the place-tag reach model,
    and the absent inter-entity distance axis. This is the largest architectural narrative, but Path A has no
    production emitter, so it blocks nothing on the live path today.

## The genuine missing-physics items (as against wiring gaps)

Three bottoms are not present-but-unwired; a substrate would have to be built:

- An inter-entity spatial-separation port for the reach laws. `inverse_square_falloff` borrows
  `mech.arm_length` as its distance input (`chem_optics_floor.toml:250`), a body dimension standing in for a
  source-to-perceiver separation. Path B does carry cell coordinates (the thermal field and `w.coord()` use
  them), so a distance is computable from the coordinate model; whether the reach law's distance port should be
  generalized from `mech.arm_length` to a first-class separation is the open question.
- A per-channel emission-and-propagation law for non-Terran channels. `laws.rs` carries Stefan-Boltzmann
  radiant emission, inverse-square, optical depth, and acoustic absorption; a mana-field or redox-gradient
  channel need not emit by Stefan-Boltzmann nor fall off as inverse-square. `opt.source_power` and
  `acoustic.source_power` are two hardcoded channel-specific axes, not a data-driven per-channel power
  registry. For Terran channels the reach laws are a wiring gap; for the alien case they are a missing physics
  substrate.
- The `eligibility_decay` action-to-reserve latency (item 7 above), a metabolism-model quantity.

## What the completeness critic added (verified against source)

The seven stages mapped the direct environmental-percept-to-learner path cleanly but omitted several live
perceive-to-learn mechanisms, all confirmed at source:

- The retention and forgetting half of learning is unmapped, and for the associative harm/reward beliefs it is
  a deferred follow-on: those beliefs never decay live (`agent.rs:29,91-95`), a present-but-unwired gap. Other
  belief families do decay live and were also unmapped: conviction experience folds and decays each tick
  (`runner.rs:4133-4180`), cultural knowledge runs a per-tick loss roll (`transmission.rs:250-277`), the axiom
  evidence ring evicts by recency (`axiom.rs:50-51`), and concept salience decays (`language.rs:568`).
- Three more live Path B percept families the map never touched: the affordance percept, which derives graded
  physics scalars over material axes reading a reserved reference (`affordance_percept.rs:62-79`) and whose
  `AffordancePerceptKind` is a CLOSED enum (`affordance_percept.rs:96-121`), a Principle-8 authoring seam the
  map's lenses never checked; the conviction percept (a being perceiving its own axiom stances); and appetitive
  salience (`learn.rs:724`, a being reading its own committed belief).
- The anatomy-to-sense transduction substrate is optical-only wired: `anatomy.rs:338-362` carries a
  data-defined sense list (vision, smell, hearing, vibration, echolocation, electroreception, mana-sight,
  aura-sense), but only the optical SIGHT function is physics-derived, and the acoustic, chemical, and field
  senses "carry a placeholder index for now" (an `opt.refractive_index` stand-in). So the proposed
  acuity-from-anatomy fix rests on a substrate that is optically Terran for every non-optical and alien sense,
  a Terran default inside the fix.
- The forward-model / surprise (prediction-error) learning signal is unmapped (`forward_model.rs:60-107`, live
  at `runner.rs:4460-4482`), with an honest limit: the prediction is categorical, not a learned reserve-delta
  magnitude.
- A channel-transport asymmetry the synthesis smoothed over: the thermal channel has a diffusion field and a
  sensed gradient (`runner.rs:401,517`); the chemical channel has neither, so a being can thermotax on real
  physics but can only chemotax on the cognitive belief-gradient, never on a sensed chemical gradient. Distal
  chemoreception is absent not because "no propagation is due" but because a scent/resource diffusion field is
  unmodeled.
- Social belief transmission (an SI-contagion diffusion law, `belief.rs:172-189`, and the TOLD/SAID testimony
  channel) was excluded as "not a physical sense channel", a defensible scope line, but learning-from-others is
  squarely perceive-to-learn and the exclusion carries its own unclassified reserved value.
- Two notes: Path B has no per-percept detection roll (it perceives every present own-cell feature
  deterministically), so the reception "detection probability" stage is Path A only; and there is no
  attention/selection bottleneck among simultaneous percepts.

## Reconciliation with the three source-verified catches

The gate agent's section-11 framing catch surfaced three couplings, each of which I verified against source
independently. They reconcile with the study as follows.

Catch 1 (a being-signal's meaning would be authored at the emit site). Confirmed: `Trace.value` is set at
emit (`world.rs:286`), and `base_rates.rs` carries only `natural_mortality`, `visibility`, and
`decay_multiplier`, scoped as the death-implication and absence-window derivations, so it holds no
live-signature-to-danger/reward base rate and could not supply a live signal's meaning even if wired. The
study locates the correct receiver-side alternative already in the live code: `feature_observations`
(`learn.rs:504-526`) learns a percept's valence from the being's own felt outcome, reading no label. So the
being-percept must be built on that receiver-side learner, never on a value stamped at emit.

Catch 2 (Path B is not sensorium-gated, an admit-the-alien violation). Confirmed: `PerceptRegistry::perceive`
(`percept.rs:129-134`) reads the raw cell composition for every declared class with no per-being sense gate,
while the sensorium that does gate (`sensorium.rs:127`) is not consulted by the live path. The study ranks
gating the percept on the being's own sensorium as item 6, with the deriving substrate (`GeneSet::express`)
present.

Catch 3 (the spatial-reach wire is unbuilt). Confirmed: `sensorium.rs:28-31` declares the reach-and-attenuation
half unbuilt (an abstract place tag, not a coordinate). The study's propagation stage matches it exactly and
adds the missing-physics detail: the reach laws exist as tier-0 floor laws with zero live perception callers,
sitting on a place-tag engine model and an absent inter-entity distance port.

The three catches and the study converge: the being-percept is gated on a perception-substrate arc whose clean
first slice is the reach wire (physical magnitude, no valence risk), whose subtle pieces are the
sensorium-gated magnitude percept and the receiver-side valence learner, and whose emergence-critical core (the
felt sign and the raw percept) already derives clean and can be reused rather than rebuilt.
