# R-EVIDENCE: Belief Formation From The World

## Design Report — Origination of Belief from Witnessing, Physical Evidence, Inference, and Absence

### TL;DR
- **Adopt a two-layer inference engine: an integer log-odds (logit) weighted-evidence accumulator as the spine, with a small abductive hypothesis frame (inference-to-the-best-explanation) on top for multi-culprit "who-killed-whom" questions. Reject normalized Dempster–Shafer as the canonical combiner because its 1/(1−K) division is not exact-integer-safe; keep Dempster–Shafer only as an optional unnormalized (Smets/TBM) conjunctive variant for explicit ignorance modelling.** This is the single hardest choice, and the log-odds accumulator wins on determinism, order-independence, and direct composability with the existing `strength` field.
- **Adopt a lazy, event-driven absence mechanism: a `last_seen_tick` per tracked subject plus a deterministic per-subject "presumption check" scheduled in an ordered timer queue keyed by a visibility-scaled window, never a per-tick scan.** Ground it formally in the closed-world assumption / negation-as-failure and concretely in the legal doctrine of death in absentia (a real waiting-period escalation from whereabouts-unknown → missing → presumed-dead).
- **All numeric constants (evidence weights, commit thresholds, decay rates, window scalings) are RESERVED for owner calibration; this report fixes the mechanism, the data model, and the integer formulation, and surfaces every needed number with its calibration basis rather than fabricating values.**

---

## Key Findings

1. **Knowledge must be earned.** The existing Part 9 `Evidence` enum already names `Observed` and `Inferred` but leaves them unpopulated. R-EVIDENCE's job is to define the *world→facet* origination path so that a hidden murder the engine knows as ground truth does not become a belief until a mind perceives a trace or infers it. This is the propagation/origination split the document's flag identifies.

2. **The right inference rule is additive integer log-odds, not Dempster's rule.** The log-odds Bayesian update `L ← L + l_obs` is exact integer addition, associative, and order-independent — the standard introduced by Elfes (1989) and systematised in Thrun, Burgard & Fox, *Probabilistic Robotics* (2005), which defines `L = log(p/(1−p))` to convert multiplicative Bayesian updates into additive ones with clamping to a symmetric bound `|L| ≤ L_max` for numerical stability; it is the default in GMapping, Google Cartographer (Hess et al. 2016), and the ROS 2 SLAM Toolbox. Dempster's normalized rule instead requires division by (1−K), a data-dependent near-zero denominator that is numerically unstable and not exact in fixed point. The instability is the classic result of Lotfi Zadeh's "A Simple View of the Dempster–Shafer Theory of Evidence and Its Implication for the Rule of Combination," *AI Magazine* 7(2):85–90 (1986): two physicians agree a brain tumor is highly unlikely (0.01) but disagree on meningitis versus concussion, and Dempster's rule nonetheless yields "the belief that P has brain tumor is 1.0… regardless of the probabilities associated with the other possible diagnoses." The project's prior R-AXIOM note that Dempster–Shafer is "integer-friendly" holds *only* for the unnormalized conjunctive (Smets/TBM) and Yager variants, and Yager's rule is non-associative (order-dependent), which would break determinism.

3. **Absence is the architecturally novel piece and has the least precedent.** A non-event (no observation) must drive a belief. The closed-world assumption and negation-as-failure from logic programming (Reiter; Clark) are the formal analogue ("unprovable ⇒ assumed false / absent"); death in absentia is the real-world institution. The waiting period is genuinely graduated across jurisdictions: the traditional English common-law period is seven years of unexplained absence (codified e.g. in the Cestui Que Vie Act 1666), the Uniform Probate Code adopted a five-year standard, New York requires only three years (NY EPTL 2-1.7), and Minnesota and Georgia reduced it to four years — per Wikipedia's "Presumption of death": "Some states have amended their statutes to reduce the seven-year period to five consecutive years missing, and some, such as Minnesota and Georgia, have reduced the period to four years." The escalation can also accelerate under the "specific peril" doctrine — the element of peril accelerates the presumption (invoked after the September 11 attacks so death certificates could issue without the full waiting period). The cheap-detection problem (most subjects unobserved most of the time) is solved by a lazy timer queue, not scanning.

4. **The strongest game precedent is Shadows of Doubt; the most instructive failure-handling precedent is Dwarf Fortress.** Shadows of Doubt physically places real clues — blood spatter whose direction and volume encode where the shooter fired from, fingerprints, the murder weapon, witnesses — and lets an investigator assemble them into a conclusion, exactly the trace→facet→inference loop. Dwarf Fortress demonstrates the perfect-crime case directly: as the community describes it, "It's not broken. Nobody saw the crime… There's no suspects" — and the Dwarf Fortress justice design (Tarn Adams, Bay 12 Games) models wrongful conviction explicitly: vampires make false reports to frame other dwarves, and witnesses "sometimes make false reports to try and frame dwarves they have grudges against," with "practically anybody" convictable — the desirable false-inference emergent feature.

5. **Concealment is the dual of perception, not of lying.** Lying (Part 37) asserts a false *value* for a belief being communicated. Concealment suppresses *formation*: it acts on the witnessable traces (hide the body, clean the fluid, dispose of the weapon) so no facet originates. CK3's "Secret" is the macro-precedent — a hidden fact known to few, discoverable over time via the Find Secrets spymaster task or random events, then exposable or convertible to a blackmail Hook — but CK3 tracks secrets as character-level flags; R-EVIDENCE must keep concealment at the level of physical traces and per-mind facets.

---

## Details

### 0. Framing and invariants

Throughout, the event log (Part 7) is ground truth and is never read by an agent as belief. R-EVIDENCE adds exactly one new authoritative entity kind — the physical **Trace** — and one new authoritative scheduling structure — the **absence timer queue**. Everything else is a *projection* into a `BeliefStore` (Part 9) via the existing `Evidence` enum. The mechanism is fixed audited Rust; the weights, thresholds, windows, and decay rates are data and are all RESERVED.

Determinism rules inherited as hard constraints: Q32.32 `Fixed`, per-entity counter-based RNG `SplitMix64(hash(master_seed, entity_id, phase))`, canonical id-ordered aggregation, fixed rounding, float quarantined to view only.

### 1. PERCEPTION OF PHYSICAL TRACES

**A trace is a witnessable, world-placed entity with provenance in the event log.** When a consequential physical event occurs (a death, a wound, a fluid spill, a dropped artifact), the event handler emits one or more `Trace` records into the focus-scale local simulation (Parts 42/43). A trace is *not* a belief; it is ground-truth physical state that *affords* observation. This is the literal engine analogue of Locard's exchange principle — Edmond Locard (1877–1966), the "Sherlock Holmes of Lyon" who founded the first forensic laboratory in Lyon, formulated the principle in the early 20th century. The popular form "every contact leaves a trace" is a loose translation; the more accurate reading (cf. De Forest and colleagues), "if a contact leaves a trace, it is up to us to detect it," directly motivates the design's emphasis that traces *degrade* and must be *perceived* to matter.

```rust
/// A world-placed physical trace that can be perceived to form an Observed facet.
/// Authoritative state (Q32.32, counter-RNG); lives in focus-scale sim.
struct Trace {
    id: StableId,
    kind: TraceKind,
    pos: Position,
    created_tick: Tick,
    /// Links back to the ground-truth event that produced it (Part 7 provenance).
    origin_event: EventId,
    /// What facet(s) perceiving this trace should propose. A trace "means" something.
    implies: SmallVec<[TraceImplication; 2]>,
    /// Perceptibility that decays/changes over time (a corpse decays, blood dries/fades).
    salience_curve: SalienceCurve,
    /// Concealment state: reduces or zeroes perceptibility. See §4.
    concealment: ConcealmentState,
}

enum TraceKind {
    Corpse { subject: StableId, decay: DecayStage },
    Fluid  { fluid: FluidKind, subject: Option<StableId>, volume: Fixed }, // spilled blood/ichor/sap
    Wound  { on: StableId, severity: Fixed },                              // a wound on a living/dead body
    DroppedItem { artifact: StableId },                                    // weapon etc.; item carries its own event-log history
    Disturbance { /* dug earth, scorch, drag marks */ },
}

/// Maps a perceived trace to a proposed belief facet.
struct TraceImplication {
    subject: StableId,    // who/what the facet is about
    attr: AttrKind,       // e.g. Whereabouts, Condition(dead/alive), WasAtPlace, IsBleeding
    value: AttrValue,     // e.g. Dead, or "weapon X was at place P at/after tick T"
    base_weight: Fixed,   // RESERVED: evidential weight of this implication
}
```

**Perception produces an `Observed` facet through the existing Witness phase.** R-EVIDENCE redefines the Witness phase of the four-phase Gossamer loop so that, in addition to perceiving other agents' attributes (the Talk of the Town model), an agent within the perceptibility radius of a `Trace` rolls perception against the trace's *current* salience. On success it writes a `BeliefFacet` with `evidence = [Observed]`, `subject`/`attr`/`value` copied from the `TraceImplication`, `truth_at_obs` set from the event log at that tick, `strength` initialized to the (RESERVED) weight, and `parents` empty (origination, not propagation). Perception is fallible exactly as the project already models distortion at Witness: with probability gated by the agent's perception/acuity and the trace salience, the recorded `value` may mutate via the existing per-attribute mutation graph. This is grounded in the eyewitness-reliability literature (Elizabeth Loftus's misinformation-effect research): encoding is degraded under stress and low attention, and salience of the original detail predicts fidelity.

**Trace properties gate whether and when perception happens.** The `salience_curve` makes a fresh corpse highly perceptible and a skeletonized or buried one nearly imperceptible; a large fresh blood pool is salient and a dried trace faint. Salience is evaluated lazily only when an agent is co-located, so there is no per-trace per-tick cost.

Determinism: perception roll = `SplitMix64(hash(master_seed, observer_id, trace_id, tick, PHASE_WITNESS))` compared against an integer threshold derived from `salience(trace, tick) ⊗ acuity(observer)`. Co-located agents are enumerated in id order.

### 2. THE INFERENCE RULE (the central choice)

**Recommendation: integer log-odds weighted-evidence accumulation as the spine, with a bounded abductive hypothesis frame for multi-hypothesis questions. Reject normalized Dempster–Shafer.**

#### 2.1 Why log-odds wins

I evaluated the three named candidates against the determinism constraint:

- **Weighted evidence accumulation (log-odds form).** The posterior log-odds of a hypothesis is the prior plus the sum of independent evidence weights: `L_post = L_prior + Σ w_i`. This is **exact integer addition, commutative, associative, and order-independent** — the occupancy-grid update `L ← L + l_obs`, clamped to `±L_max` (Elfes 1989; Thrun, Burgard & Fox 2005). It composes natively with the existing `strength: Fixed` field, which is already defined as "the sum of evidence strengths." This is the decisive advantage. A 2026 analysis comparing Bayesian log-odds against Dempster's rule for 2D occupancy grids (Berlenko & Krinkin) concludes the log-odds accumulator "is additive and order-independent" while "Dempster's multiplicative combination with 1/(1−K) normalization converges more slowly to certainty… producing less decisive beliefs," directionally favoring Bayesian fusion in all 15 of 15 per-metric comparisons under pignistic matching.
- **Dempster–Shafer (normalized).** Handles ignorance elegantly (mass on the whole frame) and is genuinely attractive for "we don't know" states, but **its 1/(1−K) normalization is a data-dependent division by a near-zero denominator** — numerically unstable under conflict (Zadeh 1986) and not exact in fixed point. Sentz & Ferson's Sandia survey states the denominator "has the effect of completely ignoring conflict… [and] will yield counterintuitive results in the face of significant conflict." **Reject as the canonical combiner.**
- **Abduction / inference to the best explanation.** Not a combination arithmetic but a *hypothesis-selection* layer: choose the hypothesis that best explains the traces (Peirce; Harman 1965; Lipton; Josephson & Josephson 1994). Adopt it *on top* of the accumulator for the "who" question, where the hypothesis set is small and discrete.

**Synthesis (recommended):** a hypothesis frame `H = {h_1…h_n, unknown}` per (subject, attr) question; each piece of evidence contributes an integer signed weight to each hypothesis's log-odds accumulator; the agent commits an `Inferred` facet for the argmax hypothesis when its accumulated log-odds exceeds a RESERVED commit threshold *and* exceeds the runner-up by a RESERVED margin (the IBE "best, and decisively better than the next-best" criterion that guards against van Fraassen's "best of a bad lot" objection). Ignorance is represented natively: if no hypothesis crosses the threshold, the committed facet stays `whereabouts-unknown`/`unknown` — we get Dempster–Shafer's main benefit (explicit ignorance) without its division.

```rust
/// A deterministic, integer inference over a small hypothesis set for one (subject, attr) question.
struct InferenceFrame {
    subject: StableId,
    attr: AttrKind,
    hypotheses: SmallVec<[Hypothesis; 4]>, // includes an explicit Unknown
    /// log-odds accumulator per hypothesis, Q32.32, additive & order-independent
    logodds: SmallVec<[Fixed; 4]>,
    /// provenance: every facet/trace that contributed, for traceable wrong inferences
    support: SmallVec<[EvidenceRef; 8]>,
    clamp: Fixed,            // RESERVED: ±L_max to bound certainty (Thrun-style)
    commit_threshold: Fixed, // RESERVED
    margin: Fixed,           // RESERVED: argmax must beat runner-up by this
}

struct EvidenceRef { facet_or_trace: StableId, weight_applied: Fixed, toward: HypothesisId }

enum Hypothesis {
    Unknown,
    SubjectDead,
    SubjectAlive,
    KilledBy(StableId),
    /* attr-specific */
}
```

#### 2.2 Integer mechanics and the readout

The accumulator never needs floating point. Each evidence weight is a RESERVED Q32.32 constant (basis: the log-likelihood ratio "weight of evidence" of that observation type — a fresh corpse of subject X is heavy evidence toward `SubjectDead`; a faint old bloodstain is light). Accumulation is integer add with clamp. The **only** non-exact operation is converting a committed log-odds to a displayable confidence; per the determinism analysis this is done with a **precomputed integer logistic LUT or a shift-based piecewise-linear (PLAN-style) approximation** — the standard embedded-systems technique, since the sigmoid otherwise requires an exponentiation and a division — and crucially it is needed only for the view layer or for a single threshold comparison, never inside canonical accumulation. (The threshold test itself can be done in log-odds space by pre-converting the threshold to an integer, so the sigmoid never touches the hot path.) Where a stochastic step is wanted (e.g. whether a low-acuity mind jumps to a conclusion below threshold), it is keyed `SplitMix64(hash(master_seed, agent_id, frame_id, tick, PHASE_INFER))`.

#### 2.3 Genome and epistemic-stance coupling

The same accumulator yields different beliefs per mind by parameterizing, all from existing systems:
- **R-GENOME reasoning-acuity** scales applied evidence weights and/or lowers the effective commit threshold: a smarter mind extracts more signal per trace and needs less to commit (but is therefore not necessarily more *correct* — a clever mind can commit a confident wrong inference faster).
- **R-AXIOM epistemic stance** sets the prior `L_prior` and the margin: a skeptical mind starts nearer `unknown` and demands a larger margin; a credulous mind commits readily. This reuses the AGM-style belief revision the project already uses for axioms.

#### 2.4 Composition with propagation and defeasibility

Once committed, an `Inferred` facet is an ordinary `BeliefFacet`: it propagates and mutates through the existing gossip loop exactly like an `Observed` or `Told` belief, carrying its `parents`/`predecessor`/`evidence` provenance so the legends view can show "real event vs each generation's drifted belief," including drifted *inferences*. Defeasibility (the non-monotonic requirement) falls out for free: new evidence adds a weight of the opposite sign; if a later observation (the subject walks into town alive) contributes a large negative weight, the accumulator crosses back and a new facet with a new `predecessor` link supersedes the inference. This is precisely AGM/defeasible revision realized as integer accumulation — "adding the strength of the evidence to your prior," and able to retract.

### 3. INFERENCE FROM ABSENCE

**Recommendation: a lazy, event-driven `last_seen_tick` with a deterministic timer queue; never a per-tick per-subject scan. Escalate whereabouts-unknown → missing → presumed-dead on a visibility-scaled window. Ground formally in closed-world / negation-as-failure; ground the escalation in death in absentia.**

#### 3.1 Cheap deterministic detection

Each *tracked* subject (only those that are the subject of at least one live facet in at least one mind — i.e. socially referenced; most of the world is never tracked) carries, per observer-community or per prominent-observer, a `last_seen_tick`. Whenever any Witness-phase observation of that subject occurs, `last_seen_tick` is updated and a **single re-check event** is (re)scheduled into an ordered timer queue at `last_seen_tick + window(subject)`. There is no scanning: the queue pops due checks in canonical (tick, StableId) order. If the subject was seen again before the check fires, the stale check is a cheap no-op (its scheduled tick is behind the current `last_seen_tick`) and the next check is already scheduled. This is O(events), not O(subjects × ticks).

```rust
struct AbsenceTracker {
    subject: StableId,
    last_seen_tick: Tick,
    state: AbsenceState,
    /// window scales with visibility/salience/social-embeddedness (RESERVED scaling)
    window: Ticks,
}
enum AbsenceState { Present, WhereaboutsUnknown, Missing, PresumedDead }

/// Ordered timer queue keyed by (due_tick, StableId) for determinism.
struct AbsenceQueue { due: BinaryHeapDeterministic<(Tick, StableId)> }
```

#### 3.2 Visibility-scaled window

`window(subject)` is short for a prominent, highly-referenced subject (a king, missed in days) and long or effectively infinite for a hermit (years, or never). The window is computed from the existing **salience-as-familiarity** signal (normalized reference count) — the more embedded and observed a subject normally is, the more their absence is *informative*, exactly the closed-world intuition that absence of an expected observation is evidence. All window scalings are RESERVED (basis: the death-in-absentia waiting-period spread — seven years at English common law per the Cestui Que Vie Act 1666, five under the Uniform Probate Code, three in New York under NY EPTL 2-1.7, four in Minnesota and Georgia, and accelerated to months under the "specific peril" doctrine — mapped onto the engine's prominence axis, which is the analogue of "circumstances strongly indicating death").

#### 3.3 Escalation as facets, reconciled with truth

Each fired check that finds `current_tick − last_seen_tick ≥ threshold(state)` advances the state and **writes/strengthens a facet** via `Evidence::Inferred` feeding the §2 accumulator: `WhereaboutsUnknown` contributes a small weight toward `SubjectDead`; `Missing` more; `PresumedDead` enough to (with RESERVED thresholds) commit a `Condition=Dead` inferred facet of modest strength. The strength is deliberately low and decaying so the presumption is fragile. Reconciliation with ground truth is automatic and bidirectional: the subject may actually be alive and travelling (a later sighting fires a heavy negative weight and **collapses the presumption**, superseding the facet) or actually dead (the presumption happens to be correct, and may later be corroborated by a found corpse, which upgrades evidence from `Inferred` to `Observed`). The formal model is negation-as-failure under a *locally* closed world (the community assumes what it cannot observe), which is exactly the defeasible, rebuttable presumption of the legal doctrine.

### 4. CONCEALMENT

**Recommendation: a utility-driven action that mutates the witnessable `Trace` set so facets never form; distinct from Part 37 lying. Partial concealment leaves partial evidence; observed concealment is itself a trace.**

```rust
/// A goal-directed action available to an agent (Part 8 utility AI).
enum ConcealAction {
    HideBody { trace: StableId, dest: Position },    // move corpse out of perceptibility
    DestroyBody { trace: StableId },                 // remove trace entirely (costly, slow)
    CleanFluid { trace: StableId, fraction: Fixed }, // reduce Fluid volume/salience
    DisposeItem { artifact: StableId },              // remove dropped-weapon trace
    Bury { trace: StableId },                        // creates a Disturbance trace (see below)
}
struct ConcealmentState { perceptibility_mult: Fixed, hidden_at: Option<Tick> }
```

Concealment lowers or zeroes a trace's effective salience (so the §1 perception roll fails) or removes the trace, thereby preventing or delaying §1 facet formation and starving the §2 accumulator. **Partial concealment** (a hastily cleaned fluid pool, `fraction < 1`) leaves a faint trace that a skilled investigator can still perceive at reduced probability — the residual signal that makes the perfect crime imperfect. Concealment **interoperates with absence**: a successfully hidden body means the only remaining path to belief is the §3 absence timer (no corpse to find), which is exactly why the murder becomes a slow-burning "missing person" rather than an instant "murder." **Concealment is itself witnessable**: each conceal action *emits its own event* (Part 7) and may spawn a `Disturbance`/`Bury` trace; if an agent is within the perceptibility radius during the act, they form an `Observed` facet like "saw Y burying something at place P," which feeds the §2 accumulator toward `KilledBy(Y)` — the "someone seen burying something" clue. Utility to conceal scales with the actor's exposure risk and (R-GENOME) disposition; thoroughness scales with skill and time available.

This is deliberately distinct from Part 37's recursive theory-of-mind lying. Lying asserts a false *value* into a communication channel with preserved provenance (`Told { by }`); concealment denies the channel an input at all by removing the world-state that would seed a facet. The two interoperate cleanly: a killer can both conceal the body *and* lie about the victim's whereabouts, and an investigator who detects the lie (Part 37) gains a `Told`-weighted evidence contribution toward `KilledBy(liar)` even if no physical trace survives.

### 5. INVESTIGATION

**Recommendation: a goal-directed evidence-gathering goal in the Part 8 utility AI, run by an aggrieved kin (relationship motive) or an emergent legal authority (Part 36), driving movement, perception, interviewing, and §2 combination. It can fail and can reach a wrong conclusion.**

Triggers (any raises the goal's utility): a §3 `Missing`/`PresumedDead` presumption about a socially-close subject; a found corpse trace; a grievance/feud relationship. The goal decomposes into reused primitives:
- **Move to loci** (last-seen place, home, the corpse) so the agent enters perceptibility radii and runs §1 perception — actively manufacturing `Observed` facets.
- **Interview witnesses** — a directed gossip-loop interaction that solicits others' facets, producing `Told { by }` evidence that feeds the investigator's §2 accumulator. This is Dwarf Fortress's interrogation, where the captain of the guard's social skills against the target's determine whether the target "confesses what they know."
- **Combine** via §2; commit an `Inferred` `KilledBy(·)` facet when threshold+margin are met.

Investigator **skill/intelligence** (R-GENOME acuity, plus relevant skills from Parts 35/23) raises perception success on faint/partial traces, raises interview yield, and sharpens the margin test. **Failure modes are first-class and desirable**: insufficient evidence → no commit (cold case); or a misleading trace set (a coincidental fingerprint, a grudge-driven false `Told`) crosses the threshold and commits a **wrong** `KilledBy` facet → a **false accusation** that then propagates and mutates as an ordinary belief (Dwarf Fortress's wrongful conviction from grudge-driven false reports; Talk of the Town's observation that "patently false information could have actually originated with the lover"). Because the wrong inference carries full provenance, it remains traceable and correctable by later evidence (§2.4).

### 6. THE AGGREGATE TIER

**Recommendation: pools propagate *knowledge of* a canonical event with a distance/delay diffusion, never instant common knowledge; concealment and inference degrade to coarse pool-level effects or defer to promotion.**

A demographic event (a death) is canonical at the pool the moment it happens (Part 8), but the pool's *belief* in it spreads on a cheap integer diffusion. Each pool tracks, per prevailing belief, a `knowledge_level: Fixed` that rises over time toward saturation as a function of (graph distance from the origin pool) and a RESERVED diffusion rate, advanced once per pool per knowledge timestep (not per agent), reusing the same mutation rules applied once per pool that Part 9 already specifies. A death in a distant province thus becomes "known" there only after a delay proportional to distance — the required non-instant spread.

```rust
struct PoolBelief {
    subject: StableId,
    attr: AttrKind,
    value: AttrValue,
    knowledge_level: Fixed,  // 0..1 Q32.32, rises by integer diffusion
    strength: Fixed,
    drift_seed_phase: u64,   // counter-RNG phase for once-per-pool mutation
}
```

- **Absence** at the aggregate tier is even cheaper: a pool need not track individuals; a presumed-dead status diffuses as just another `PoolBelief`, or is deferred — the subject is only promoted to an individual `AbsenceTracker` when an event becomes load-bearing.
- **Concealment** degrades to a RESERVED suppression factor on the diffusion rate (a concealed death spreads slower / may stall below saturation), or is deferred to promotion if a high-LOD actor is involved.
- **Tier consistency (R-TIER-CONSIST):** on promotion, a pool member instantiates an individual `BeliefStore` seeded from the pool's prevailing `PoolBelief`s at their current `knowledge_level` (mapped to facet `strength`), so a promoted agent's knowledge is consistent with what the pool "knew." Demotion folds the individual's prevailing facets back into pool statistics. The mapping (knowledge_level ↔ facet strength) is RESERVED.

### Academic grounding — adopt / avoid

- **Dempster–Shafer / evidence theory (Dempster; Shafer; Smets' Transferable Belief Model; Yager).** *Adopt:* the frame-of-discernment idea (explicit hypothesis set including ignorance) and, optionally, the **unnormalized conjunctive (Smets) rule**, which is purely multiplicative, commutative, and associative — integer-friendly, with conflict kept on the empty set rather than divided away. *Avoid:* the **normalized** Dempster rule's 1/(1−K) division (Zadeh 1986; Dezert & Smarandache counter-examples; fixed-point instability) and **Yager's** rule (division-free but non-associative ⇒ order-dependent ⇒ non-deterministic accumulation).
- **Bayesian updating / log-odds (logit) form.** *Adopt as the spine:* additive, order-independent, exact integer (Elfes 1989; Thrun, Burgard & Fox 2005); readout via integer LUT/PWL sigmoid. This is the recommended combiner.
- **Subjective logic (Jøsang).** *Adopt the idea* of carrying belief+disbelief+uncertainty and trust-discounting of sources (maps to weighting `Told` evidence by trust in the teller). *Avoid* its division-bearing fusion operators in canonical math; use it as conceptual scaffolding.
- **Possibility theory (Dubois & Prade).** *Note:* min/max combination is the most trivially integer-exact (no division, idempotent, order-independent) but information-poor (qualitative — a "hyper-cautious" version of the TBM). Reserve as a fallback for cheap/coarse questions.
- **Abduction / IBE (Peirce; Harman 1965; Lipton; Josephson & Josephson 1994; Thagard's explanatory coherence).** *Adopt:* the best-and-decisively-better hypothesis-selection criterion on top of the accumulator; guard against "best of a bad lot" with the margin test.
- **Epistemology of testimony/perception + defeasible/non-monotonic reasoning + AGM.** *Adopt:* justified belief from observation/testimony/inference; defeasible retraction realized as opposite-sign evidence; AGM revision (already used for axioms) for superseding facets.
- **Closed-world assumption / negation-as-failure / default logic (Reiter; Clark; Minker).** *Adopt:* the formal "unseen ⇒ assumed absent" under a *locally* closed world for the absence mechanism; *avoid* global CWA (computationally intractable — stable-model entailment is NP/co-NP-complete — and the world is genuinely open) — keep it local and defeasible.
- **Forensic/evidential inference (Locard's exchange principle; forensic Bayesian networks; trace evidence).** *Adopt:* "every contact leaves a trace" as the literal model for emitting `Trace`s from physical events, and degradation of traces over time; *avoid* full forensic Bayesian-network inference at civilization scale (too expensive) — approximate with the bounded log-odds frame.
- **Eyewitness reliability / misinformation effect (Loftus).** *Adopt:* perception is fallible at encoding (gate by acuity/salience/stress) and beliefs are corruptible post-event — already modeled as distortion at Witness and via the mutation graph.

### Game/simulation precedent — adopt / avoid

- **Shadows of Doubt (ColePowered Games).** *Adopt:* physically-placed, real clues (blood spatter whose direction and volume encode where the shooter fired from, fingerprints, the murder weapon, witnesses), an evidence-assembly board linking clues to a conclusion, and the genuine "no clue, no solution" perfect-crime outcome players actually encounter. *Avoid:* the de-facto guarantee that "every single murderer drops a crucial key-evidence item at the crime scene" — that defeats the perfect crime; in R-EVIDENCE concealment can leave *zero* perceptible traces, leaving only the absence path.
- **Dwarf Fortress (Bay 12 Games).** *Adopt:* unwitnessed crime ⇒ no suspect ("Nobody saw the crime… there's no suspects"); interrogation as a social-skill contest; **wrongful conviction and false reports from grudges** (and vampire framing) as desirable emergent features; the requirement that a creature be witnessed and identified. *Avoid:* the unsorted, opaque witness-list UX, and any path to conviction on no evidence as a *player* override — keep convictions evidence-driven.
- **Crusader Kings 2/3 (Paradox).** *Adopt:* a hidden fact known to few, discoverable over time and exposable or convertible to leverage (the secret/scheme/agent-network discovery loop; Find Secrets task; discovery via random events). *Avoid:* representing the secret as a character-level flag "discovered" by a dice roll — R-EVIDENCE keeps the truth in the event log and the *knowledge* strictly in the minds/pools that earned it from traces.
- **Talk of the Town / Bad News (Ryan & Mateas).** *Adopt (already adopted):* the agent-driven, per-mind belief facets bounded to who actually learned a fact, the eleven-phenomenon evidence typology, the belief mutation graph, salience gating, and ~300–500 fully-modelled characters as the LOD ceiling. Per Ryan & Mateas, *Game AI Pro 3* ch. 37, the framework simulates a town day-by-day across roughly 140 years of generation to reach that population; Bad News stored its town's state in roughly 400 variables. This is the direct ancestor of Part 9.
- **The Sims; Façade; Versu; Neverwinter Nights reputation.** *Note:* gossip/witnessing precedent; mostly ancillary or scripted — adopt only the witnessing-radius intuition.

### Determinism analysis

- **Integer/fixed-point safe:** evidence weights, log-odds accumulation (`L ← L + w`, clamp `±L_max`), thresholds, margins, decay, diffusion — all Q32.32 add/compare/shift. The Smets fallback is multiply-only (128-bit intermediate, fixed right-shift). **No canonical division.**
- **The one quarantined conversion:** log-odds→probability for *display* or a single threshold test uses a precomputed integer LUT or shift-based PWL; it never enters canonical accumulation, and the threshold test is done in log-odds space directly (compare `L` to a pre-converted integer threshold), avoiding the sigmoid entirely on the hot path.
- **Absence ordering:** the timer queue pops in canonical `(due_tick, StableId)` order; stale checks are deterministic no-ops; no per-tick scan and no iteration-order dependence.
- **Stochastic steps:** every roll (perception success, sub-threshold "jumping to conclusions," confabulation draw) is keyed `SplitMix64(hash(master_seed, entity_id, object_id, tick, phase))` with a distinct phase per mechanism, so results are bit-identical across machines and thread counts.
- **Aggregation:** co-located observers, contributing evidence refs, and pool updates are all enumerated in StableId order before combination; since log-odds accumulation is associative this ordering is belt-and-braces, but it is enforced anyway for the non-associative-adjacent steps (e.g. mutation draws).

### Settled vs. RESERVED for owner calibration

**Settled (mechanism, fixed audited Rust):**
- `Trace` as a witnessable entity with event-log provenance and a time-varying salience curve; perception writes `Observed` facets via the Witness phase.
- Log-odds additive accumulator + bounded abductive hypothesis frame as the inference rule; `Inferred` facets carry full provenance; defeasible via opposite-sign evidence.
- Lazy `last_seen_tick` + ordered timer queue for absence; whereabouts-unknown → missing → presumed-dead escalation as facets.
- Concealment as a trace-mutating utility action, distinct from lying, itself witnessable.
- Investigation as a utility goal reusing move/perceive/interview/combine; false accusation as an emergent outcome.
- Aggregate-tier knowledge diffusion with delay/distance and promotion-time seeding for tier consistency.
- The full determinism scheme above.

**RESERVED (data; surfaced with calibration basis, never fabricated):**
- Per-implication evidence weights (basis: log-likelihood "weight of evidence" per observation type; corpse ≫ stale bloodstain).
- Commit threshold and runner-up margin (basis: desired false-accusation rate vs. cold-case rate; the IBE "decisively better" gap).
- Log-odds clamp `±L_max` (basis: maximum admissible certainty, Thrun-style).
- Trace salience curves and decay (basis: corpse decay stages, fluid drying/fading timescales at the sim's tick rate).
- Absence windows and their visibility scaling (basis: death-in-absentia waiting periods — 7 yrs English common law, 5 under the Uniform Probate Code, 3 in New York, 4 in Minnesota/Georgia, accelerated under evident peril — mapped onto the prominence axis).
- Presumption strengths and decay (basis: fragility of an unrebutted presumption).
- Concealment perceptibility multipliers and skill/time costs.
- Genome-acuity and axiom-stance couplings (weight scale, prior offset, margin).
- Aggregate diffusion rate, concealment suppression factor, and knowledge_level↔strength mapping.

---

## Recommendations

1. **Build the log-odds accumulator first** as the canonical combiner, with the abductive frame and margin test layered on; wire `Evidence::Observed` (from §1 traces) and `Evidence::Inferred` (from §2/§3) into it. Benchmark on the perfect-crime scenario: a witnessed murder should commit `KilledBy` fast; a concealed one should reach at most `Missing`/`PresumedDead` via §3.
2. **Implement the lazy absence timer queue second**; it is the novel piece and the determinism risk if done by scanning. Validate that adding/removing tracked subjects and re-sightings never changes results across thread counts.
3. **Add concealment and investigation as utility goals third**, since they compose existing Part 8 primitives. Tune so that false accusations occur at a low but nonzero, *intended* rate.
4. **Defer all numeric calibration to the owner.** Ship with placeholder RESERVED tables clearly marked, plus the calibration basis noted above.
5. **Thresholds that would change these recommendations:** if profiling shows the per-(subject,attr) frame is too expensive at civilization scale, fall back to the single-hypothesis log-odds-only form (drop the frame, keep dead/alive); if owners want richer "we don't know" semantics and can accept the cost, promote the Smets unnormalized DS variant for the specific `who-killed-whom` frame only. If determinism audits ever flag the LUT sigmoid, eliminate it entirely by keeping all comparisons in log-odds space.

## Caveats and unsolved pieces

- **Full forensic inference is too expensive at civilization scale.** The bounded log-odds frame is a deliberate approximation of a forensic Bayesian network; it cannot represent rich conditional dependencies between traces. This is accepted.
- **The cheap absence detection trades accuracy for cost.** The lazy timer assumes a single re-check per subject; rapid alternation of sightings is handled by no-op stale checks, but very bursty visibility could in principle delay an escalation by up to one window. Accepted as a cost trade.
- **Wrongful conviction / false inference is a feature, not a bug.** The system is explicitly designed so that misleading trace sets and grudge-driven testimony can commit wrong, propagating beliefs — this is the desirable emergent drama (Dwarf Fortress, Talk of the Town), not an error to be eliminated.
- **Local closed-world is a simplification.** "Unseen ⇒ presumed absent" is false in an open world (the subject is travelling); the defeasible, rebuttable design contains the damage but cannot prevent a community from confidently presuming a living traveller dead — which is, again, realistic and intended.
- **Subjective-logic/Dempster ignorance modelling is only partially captured.** The "explicit ignorance" benefit is approximated by the `Unknown` hypothesis and the commit threshold rather than a full belief-function calculus; if this proves too coarse, the Smets fallback is the upgrade path.

**Primary recommendation, restated decisively:** for the inference rule, use an **integer log-odds weighted-evidence accumulator with a bounded abductive (best-and-decisively-better) hypothesis frame**, and reject normalized Dempster–Shafer on determinism grounds. For the absence mechanism, use a **lazy `last_seen_tick` plus an ordered timer queue with a visibility-scaled window**, escalating whereabouts-unknown → missing → presumed-dead as defeasible `Inferred` facets, grounded in negation-as-failure and death in absentia. These two choices resolve the open half of the belief system — origination from evidence — while preserving every existing Part 7/8/9/37 constraint and keeping all numeric calibration reserved for the owner.
