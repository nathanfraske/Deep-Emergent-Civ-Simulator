# Morning review: overnight interim decisions and deferred owner calls

The owner is away overnight; the delegation runs unattended (the cloud agent building Arc 2 and on, guided
and gated by me). Per the owner's directive, I build PAST decisions rather than stall: for a fork the owner
would normally weigh, I make a reasonable INTERIM call, state its basis and how to reverse it, tell the agent
to proceed, and log it here. Only a truly owner-only or hard-to-reverse decision waits. This doc is the single
place to review what happened overnight and what still needs your ruling. Newest entries at the top of each
section.

## 2026-07-09 (later, owner intermittently present): perception arc merged; two forks (one ruled, one held); the physics-floor map correction

**Perception-substrate arc MERGED (#109, commit 08a7cc1).** Slices 1-3 (reach wire, sensorium-gated percept, valence learner core) landed byte-neutral. Agent A is on the being-percept KEYSTONE next (branch `claude/being-percept-keystone`), the payoff that wires the substrate live so predation, hunting, and fleeing emerge, coordinated with B on the shared `learn.rs` hash and estimator.

**R-AGING lifespan: REOPENED and HELD for the owner (this SUPERSEDES the "(B) do now" verdict lower in this section).** Agent B's blind framing (section-11, Opus at max, fail-closed) found, before any code, four source-verified structural problems with the simplest (B) reading, and I verified the load-bearing ones against source myself: (1) DECISIVE, the naive (B) is infeasible at the pool tier where most deaths happen, because pools carry no per-part body (design.md:2497-2499, 789), so wear cannot run there; (2) wear proportional to the mass-Kleiber throughput relocates the same rate-of-living mass-longevity shape into wear-and-repair constants; (3) k_repair has no floor derivation (regeneration is an optional magical trait, clot_rate is haemostasis not integrity), so it becomes an authored outcome-constant; (4) retiring authored lifespan breaks the load-bearing cultural-drift-speed differentiation (design.md:1724). The naive (B) the earlier de-biaser recommended is dead on finding 1. The honest form is (B1): wear as a real physics-floor material-fatigue law, death emerges as time-to-failure, the pool tier's age-mortality is the aggregate PROJECTION of that law (satisfying R-TIER-CONSIST), and repair derives from a real tissue-turnover floor axis, not the magical regeneration trait. (B1) honours "wear on the body", retires the authored per-race number, and is alien-clean because per-race body data (tissue material, turnover rate) overrides the baseline. THE PIVOTAL OWNER QUESTION: (B1) reproduces the real mass-longevity pattern (about mass^0.25) by DERIVATION from wear physics, not by authoring, and stays overridable per-race; is that the correct realistic default the owner wants (the rate-of-living pattern EMERGING, the P8-clean form of a real law), or does the owner want lifespan fully decoupled from mass? My lean: (B1)-accept, held. Agent B pivoted to the composer/hash arc meanwhile. Reverse by picking (B1), (B2 insult-only), reopen-(A), or a hybrid.

**Redox yield (Nernst): RULED as derive-first engineering; floor-growth recorded for Principle-9 visibility.** Agent C's blind framing found four seams in the derive-clean Nernst spec I had verified (Prime Directive 2, auditing the input): the yield needs activities not concentrations (a gamma activity law, the Terran aqueous gamma=1 made an explicit overridable default); n as a per-substance `chem.electron_count` floor axis, not a per-source knob; the energy magnitude as n*F*max(0,E) with a universal molar R and Faraday F (the existing `gas_constant` is the SPECIFIC R_s, laws.rs:1294, verified); and emergent per-lineage metabolic efficiency (P8). All four are physics-correctness and P8, not owner-taste, so I RULED them in (the standing directive empowers me on the alien-clean-correct mechanism and the value-line). The floor grew by three legitimate physics additions (the activity law, the electron_count axis, the molar R and F constants); Prime Directive 6 makes developing the substrate the correct response rather than authoring around the gap, and I record the growth HERE for the owner's Principle-9 visibility, not as a blocker. RESERVED (owner sets, basis given): the gamma default (gamma=1 the ideal reference, per-medium real activity data overrides); molar R and Faraday F (CODATA universal constants, the legitimate floor authoring); the thermodynamic-efficiency ceiling (the real maximum conversion efficiency). The per-substance electron count is data (charge/mass balance). Efficiency wires as an evolvable trait through the existing genome/selection substrate. C builds corrected-T3 first (zero floor-growth), then the ruled Nernst.

**Physics-floor MAP correction (owner-directed 2026-07-09).** The owner directed that `docs/working/PHYSICS_FLOOR_REGISTRY.md` be made the ACTUAL TRUTH of the physics substrate ("so we know where to look for things, because this repo is only going to get bigger") and that the discipline be folded into the standing agent rules. The gap the owner's question surfaced: the map generates only from the `.toml` floor DATA, so the direct law kernels the agents add in `laws.rs` (the spreading law, the transduction family) bypass it (`laws.rs` has 77 kernels, the map documented 68). It was not stale (its entries matched the `.toml`) but incomplete. Fix DONE (commit 5f6ec2f): the generator now enumerates every `laws.rs` kernel (80, the 12 direct ones tagged `[direct]`), the map lists all declared and direct laws with `file:line`, the stop-gate regenerates-and-diffs so it blocks a stale map on a new bare kernel, and AGENTIC_ADDENDUM section 9 (the derive-vs-author lens) carries the rule that a new floor law, declared or a direct `laws.rs` kernel, must reach the map.

**Belief-subject encoding: RULED the HYBRID (supersedes the "(SEQ_FIELD_BITS) the HASH" verdict lower in this doc).** Agent B's blind framing of the composer arc (#115) input-audited my pure-hash directive and showed, verified against source, that the current bit-pack is INJECTIVE within its envelope (learn.rs:162-174), so a pure 61-bit hash would REGRESS the collision-free common case to a birthday collision (a P10 silent conflation the pack does not commit), and the source flag itself names widening FIRST. The ruled form is the HYBRID: an exact widened pack for every in-envelope sequence (zero collision for all realistic sequences and the composer's conjunctions) plus a hash ONLY on overflow, the subspaces separated by a marker bit, so the cap dissolves without the common-case regression. Derive-first collision correctness (my authority, not owner-taste). RESERVED (owner sets, basis given): the exact-pack envelope width (the smallest band holding every realistic sequence and conjunction at zero collision), the overflow hash digest width (the collision target on the rare overflow subspace alone), the marker-bit position. Agent B builds this key re-encoding FIRST as the shared foundation (the one not-byte-neutral re-pin), coordinated with Agent A whose keystone CONSUMES the re-encoded key (told on #109); then B's composer (byte-neutral tag-3 leaf). The re-pin evidence exercises a gossip-to-convergence and a tie-break path (the re-encoding re-shuffles the planning.rs:178 `subject.0` tie-break, a benign reshuffle).

---

## 2026-07-09 (standing mediator/manager directive): current effort held items and awareness

**DE-BIASED DECISION BRIEF (the three held forks, presented to the owner).** A section-11-de-biased,
adversarially-judged workflow (each verdict source-verified) resolved the three held decisions; it overturned
one of my leans, filled an option I had omitted, and sharpened the third:
- **Affordance-composer morphospace: UNIFY, the LIGHT form (cheap win that is also the true-complete).** One
  library for designed objects and sensed affordances; the light form adds one arm to the existing composition
  node, reuses promote/fold verbatim, leaves every existing object id bit-identical, sacrifices no completeness.
  (This OVERTURNS my earlier distinct-sibling lean.)
- **SEQ_FIELD_BITS: the HASH (true-complete, do now).** Mint the belief subject by a canonical hash of the full
  step, dissolving the 16-value and step caps. Widening cannot fit the open alphabet in the 61-bit budget (a
  false economy); deferring leaves a latent P8 ceiling. A one-time not-byte-neutral re-pin; owner sets the
  collision-probability / hash-width reserved value and the timing. (This was the option I had OMITTED.)
- **R-AGING: (B) emergent senescence (true-complete, do now).** No cheap win is just as good. Kleiber's exponent
  is legitimate floor physics because it has a MECHANISTIC derivation (fractal transport geometry) the
  rate-of-living longevity relation lacks and which is empirically falsified, so authoring a longevity exponent
  authors a contested outcome; (B) authors wear/repair RATES and lets lifespan emerge, reusing the existing
  death floor. (My original lean, now on honest grounds.)

**Biosphere-balance (Agent B computed study, verified): PURE CALIBRATION, no build.** The flow-viability ratio
is mass-INDEPENDENT (mass cancels in both denominators, `physiology.rs:471`/`503-505`), so small bodies are not
harder (the small-body effect is buffer/time-to-death, not viability). Owner items: promote and set the two
dev-fixture reserved values (`food_energy_density`, basis intake-offsets-Kleiber-drain; `ingest_efficiency`,
basis Lindeman transfer); the T3-arming design-intent decision (owner-gated, needs two NEW reserved values, and
worsens grazer survival as-is, NOT a starvation fix); and reconcile a doc-vs-code seam (`locomotion.rs:1224`
claims a per-plant value supersedes the scalar once T3 wires, but `:1225` multiplies unconditionally, the
supersede claim is aspirational).


The owner designated me mediator/manager for their absence (loop every piece to bedrock, HOLD genuine
owner-blockers with basis, keep building the unblocked substrate, sequence the agents). The Mirror sign-off
below is RESOLVED (owner signed off batches 1-3, applied, Arc 2 merged as #108). Current effort: the
perception-substrate arc (Agent A, #109), the affordance/composition substrate (Agent B, #111), and the
AbioticField field-kind registry (Agent C, my recommendation, owner-approved). Held and awareness items:

- **Floor growth (awareness, not a blocker).** The perception substrate is adding TWO parameterized floor
  laws: the general dimensionality-spreading law (slice 1, subsumes the hardcoded inverse-square) and the
  transduction-response family (slice 2, a monotone response law: linear, Fechner-log, Stevens-power, Weber).
  Both are Principle-9-legal (physics and psychophysics are authored floor inputs), grounded in real science,
  parameterized-and-derived (per-being parameters derive from genome and anatomy via `GeneSet::express`, or
  reserve fail-loud where the anatomy-to-sense transduction is not built), and byte-neutral (dead substrate
  until the keystone). I RULED both as derive-first engineering, not owner decisions. Flagging the PATTERN of
  floor additions for your eyes: confirm the derive-first-floor-law approach, or rule that floor additions
  should route to you.
- **Reserved-with-basis values accumulating (standard discipline, your set on return).** Non-optical sense
  transduction parameters (fail-loud, basis: the per-channel anatomy-to-sense transduction, never the borrowed
  `opt.refractive_index`); the acoustic absorption axis (the floor carries none; the reach dev row uses the
  optical axis as a labelled stand-in, flagged); the confinement substrate (sets D below 3 for surface/duct
  signals); the affordance transduction parameters and discrimination law. Each surfaced with basis, none
  fabricated.
- **The 3-agent expansion (owner-approved).** Agent C on the AbioticField field-kind registry (open the closed
  `{Light,Water,Soil}` enum at `environ.rs:325` into a data registry; unblocks chemosynthesis, geothermal/redox,
  mana). Disjoint from A (perception) and B (affordance); I gate and sequence it.
- **Prior reversible rulings (on the PR record and the recap).** The reach-wire general-spreading-law adoption
  (fork a), the affordance and composition fork (b) splits: all derive-first engineering rulings under the
  standing gate. Reversible; confirm or override on review.
- **GENUINE OWNER-BLOCKERS from Agent B's affordance/composition bedrock study (your call, held).** The study
  found (verified against source) that the honest bottom is to REUSE the already-resolved `crates/compose`
  substrate (R-DEEPTECH-COMPOSE, unwired on the sim live path except the capability leaf), not build a new
  composer. Wiring it needs two owner decisions:
  1. **The morphospace fork.** Is a composite affordance the SAME morphospace as an artifact `CompositionNode`,
     or a DISTINCT sibling? UNIFY (widen the artifact node with a perceiver operand, one library and promotion
     path) versus DISTINCT SIBLING (reuse compose's fold/promote/open-registry kernels, keep a
     perceiver-and-target node shape). Deciding seams: promote gate 3's reuse-compression has no affordance
     analog, and the artifact node has no perceiver operand. The code supports either. This decides the
     composer's node shape, so the build is held on it.
  2. **The `SEQ_FIELD_BITS` packing ceiling** (`learn.rs:162-174`, four bits per field, 16 values). Extending
     the belief-subject key to a conjunction may cross 16 primitives, forcing a packing widening that is NOT
     byte-neutral and changes every existing belief subject. An owner-call before the composer is wired.
  The composer extends Agent A's discovery/reward-belief learner (the shared bedrock) with a conjunction subject
  key, so it is owner-blocked on the node shape and the packing and A-coupled on the learner surface. Agent B is
  on the unblocked prep (the promote-gate-3 affordance-reuse-signal design) meanwhile.
- **Floor-growth update (awareness, not a blocker): a third parameterized floor mechanism.** Agent C's
  AbioticField arc, after its blind panel caught that the read-at-cell interface authors POINT-LOCALITY
  (foreclosing a redox or gradient-fed alien energy source), generalizes the supply query with a data-selected
  READ-SHAPE and VALUE-BACKING operator set (point, pairwise-difference for a redox reaction, finite-difference
  for a spatial gradient). Each operator is physics (the real forms a supply takes), the selection is data; I
  ruled it derive-first, not owner-taste, with the acceptance gate that a deep-vent chemolithotroph fed by a
  redox difference must be a zero-Rust data row. This is the third parameterized floor mechanism (after the
  perception substrate's spreading law and transduction family). Flagging the pattern for your eyes.

- **OWNER-BLOCKER: your R-AGING directive (held; my first framing here was CORRECTED by a section-11 self-audit).**
  Agent B's blind smoke test flagged this, and a section-11 audit of MY OWN framing of the decision then caught
  (both verified against source) that my first characterisation ("authoring an allometric longevity constant
  violates the value-line, recommend the emergent form") was overstated and biased toward (B). The correction:
  the project ALREADY authors Kleiber's allometric coefficient `kleiber_a` as a RESERVED floor anchor
  (`physiology.rs:132`), so an allometric scaling coefficient is not categorically a value-line violation. And
  (B) is NOT authoring-free: it authors a metabolic-wear rate and a repair rate, and needs substrate that does
  not exist today (integrity is wound-derived only; no metabolic-wear or repair mechanism). So BOTH options
  author floor constants, and the honest fork is which floor physics is more defensible:
  - **(A):** author an allometric longevity coefficient that sets lifespan directly from mass. Cheap; has the
    allometric-coefficient precedent (Kleiber); but sets the OUTCOME (lifespan) directly and imposes one exponent
    on every world.
  - **(B):** author a metabolic-wear rate and a repair rate; lifespan EMERGES as when a body's integrity and
    reserves cross the failure boundary. More emergence-shaped (author the rate, let the outcome emerge, like
    Kleiber's metabolic rate feeding emergent outcomes), and an alien fails on its own physics; but it authors
    two rates, needs the wear-and-repair substrate built (more work), and needs the failure-boundary path wired.
  My honest lean is still (B) for being emergence-shaped, but on those grounds, NOT "A is a violation." I am
  developing the full true-complete-versus-cheap-win brief for all three decisions in a section-11-de-biased
  workflow and will present it. I did not override your directive; held for your ruling.
- **Agent C AbioticField (Arc 5) register items, held with basis.** C's segment-2 blind panel dropped a bespoke
  difference operator I had pre-approved (it authored Terran choices) in favour of the existing Liebig-minimum
  plus the existing floor law `law.battery_emf` for the redox yield (verified). Two items for you:
  1. The EMF-to-biomass coupling reserved value (biomass per unit free energy). Basis: a floor
     thermodynamic-efficiency bound; reserved fail-loud until you set it or it derives from the bound.
  2. Modeling depth (true-complete versus cheap-win): the standard EMF as a per-source constant (cheaper) versus
     a full Nernst concentration-dependent yield (more-complete floor physics, since the fields carry the
     concentrations). C builds the byte-neutral parts (per-source conversion, per-role stoichiometry) as segment
     2 now; the floor-EMF yield is segment 3 pending these.

## Owner-only calls still waiting (need your ruling)

- **Mirror dial-set sign-off (the gate): READY (RESOLVED, signed off, Arc 2 merged as #108).** The agent completed the Earth-1:1 calibration: 34
  derive-audited values set (each with `set_by` + basis + source + a why-not-derivable clause), the temperature
  seam closed, all four run_world pins holding, 950 sim tests green (manifest 90 set / 131 reserved). Mirror is
  the one owner-GATED world and I have NOT treated it canonical. Your morning actions:
  1. Approve (or adjust) the 34-value dial-set (in `calibration/reserved.toml`, marked `set_by = "Arc 2 Mirror
     calibration (cited, pending owner sign-off)"`).
  2. Set the two climate values the temperature build reserved: `climate.mean_surface_temperature` (~288 K) and
     `climate.latitude_temperature_range` (~60 K full equator-to-pole).
  3. Rule on the ~40 `escalate_owner` design choices: the agent posted a grouped one-pass decision-list on
     PR #108 (groups A non-Mirror dials, B engine/determinism bounds, C playtest/gameplay, D units/convention,
     E AUDIT CATCHES), each with a recommendation. **Group E is highest priority: 5 places the agent caught
     errors in the calibration research** (`loss_practitioner_floor = 50` is a genetic Ne~50 analogy not a
     skill figure; `loss_rate`'s consistency pin is invalidated; `stubbornness_dogmatism_weight` is a
     key-vs-wiring mismatch; `emergent_proxy_weights` uniform-1 is flagged; `group_aggregation_rule` may
     derive from member variance). Do NOT set those at the research-tagged values. I verified two of the five
     against source and both hold.
  4. Decide the orbital year: it is set to 31536000 s (365.0 d, Julian); the tropical year is ~31556952 s
     (365.2422 d). The agent leans tropical for a strict 1:1 Earth.
  Plus the derive-vs-author items in the interim-calls section below (the social-transmission values,
  `thermal_half_band`). Once you sign off, I merge Arc 2 and we transition to Arc 3 (the liveliness keystones,
  framing-panelled). The units-mechanism wiring is deferred (non-blocking, forward-looking); the medium
  convective-coefficient dedup landed byte-neutral (your "dedup now" ruling; the agent re-ran all four pins
  itself). **The branch is fully ready: the §9 five-lens arc audit ran clean on the §11-de-biased packet, its
  findings (all on the medium-h dedup's framing and test coverage, no behaviour bug) hardened byte-neutral, and
  I did the arc-completion review. Your Mirror sign-off is the only remaining gate before the merge.**

## Interim calls I made overnight (proceed-with; reversible; confirm or override in the morning)

- **Two social-transmission values authored flat (your derive-vs-author ruling wanted).** In Arc 2 segment
  `002cbfc` the agent set two SOCIAL values, classifying them as "social data not on the physics floor, not
  derivable from a lower substrate": `transmission.drift_rate` (0.03, the copy-fidelity BASE, grounded in
  Weber's ~3% JND; per-copier drift already derives from it via `copy_drift(base, memory, perception)`, so
  only the base is authored) and `enculturation.stubbornness_split` (0.40, the conserved own-conviction-vs-
  band-mean split, flat). I ACCEPTED both as authored-with-basis to keep the agent moving (byte-neutral, cited,
  defensibly classified, pins confirmed). But per your rule I did not take "not derivable" at face value:
  because you are deepening the substrate this arc, these are the candidates to DERIVE from per-being
  cognition/personality (a being's enculturation-resistance from its own conviction-strength/personality; the
  copy-fidelity base from a perception-resolution axis). Your call: accept as authored social data, or derive
  (build the substrate). Reversible either way.

- **Temperature units seam: BUILT (be00b26), byte-neutral, two climate values reserved for your gate.** The
  agent found, and I verified against source (`worldgen.rs:260`, `runner.rs:443`, `fluids_floor.toml:15`), that
  the worldgen temperature field is normalized `[0,1]` but the `therm.temperature` floor axis is absolute K and
  the metabolism `T^4` physics needs Kelvin, so a Calibrated Mirror froze its beings instantly. I authorized
  the fix and the agent built it (`Field::from_map_absolute`: `T = mean + range*(normalised - 1/2)`). It is
  byte-neutral BY CONSTRUCTION: the dev fixtures set `mean = 1/2`, `range = 1`, an exact identity that
  reproduces the old `[0,1]` field, so no pin moved (provable, no run needed). The Calibrated profile reserves
  `climate.mean_surface_temperature` and `climate.latitude_temperature_range` for you. Nothing owed but the two
  values at the Mirror sign-off: mean surface temp ~288 K and full equator-to-pole range ~60 K (±30 K). World
  data, surfaced not fabricated.
- **Climate-productivity coarse scaffold: set with the abstract limit noted.** The coarse productivity model's
  params (a documented stand-in for the gated real biosphere) set as its calibration; reversible when the
  biosphere-balance calibration replaces it.
- **`compose.max_depth` / `reuse_compression_threshold`: held reserved.** They shape emergent composition
  DEPTH, so I kept them owner-tunable rather than authored; set them as emergence tuners if you want.
- **`thermal_half_band` re-classification, your call.** Your Arc-4 ruling (keep `thermal_half_band` +
  `burn_scale` reserved, build the tissue-tolerance substrate in Arc 4) stands overnight; I did NOT override
  it. But the agent's re-triage (verified) now assesses `thermal_half_band` as a per-race thermoregulation
  control datum, the same category as the `thermal_setpoint = 310` already set, and distinct from the
  tissue-tolerance / denaturation substrate that is truly Arc 4 (that is `burn_scale`'s home). You may have
  grouped it by name; set it now on reconsideration, or keep the Arc-4 deferral. `burn_scale` stays Arc-4
  either way.

## Notes and observations from the night

- **The section-11 input-bias smoke test you directed caught a real biased audit (validation).** When the agent
  ran its end-of-arc §9 five-lens audit, it first ran the §11 smoke test on the audit's own construction. The
  smoke test returned BIASED and failed CLOSED: the agent's first audit packet handed the panel the conclusions
  ("byte-neutral / all pins hold") and the load-bearing pivots as told facts instead of source questions. The
  smoke test's spot-checks found the claims TRUE, but it correctly gated the SETUP not the outcome, so the agent
  killed that run and re-launched the audit on a de-biased packet (conclusions stripped, pivots re-posed as
  source questions). That clean §9 run is in flight; I review its verdict as the arc-completion gate. This is
  the exact failure mode you built section 11 to catch, working in practice on the agent's own audit.
- **CI/test-speed work landed (no action needed).** Build cache + nextest merged; the 6 slow
  `evolve::tests` (one >9 min) no longer sit on the per-PR critical path. They are excluded from the
  PR lane by a nextest filterset (job env `SLOW_TESTS` in `ci.yml`) and run in full in a new
  `nightly-full` job (nightly schedule + manual dispatch). First cut used `#[ignore]` + `--run-ignored
  all`, which wrongly swept in the `#[ignore]d` unimplemented Stage-N placeholder tests (they
  `unimplemented!()` and panic by design) and failed nightly-full; corrected to the filterset, which
  never touches `#[ignore]`. The fast PR lane was green throughout. VALIDATED: fast PR lane test run
  is now ~52 s (1304 passed, 8 skipped = the 6 slow evolve tests + 2 `#[ignore]`d placeholders), down
  from the ~10-minute evolve tail; nightly-full is green running the full set, placeholders correctly
  skipped. Nothing owed here; noted for context only.
