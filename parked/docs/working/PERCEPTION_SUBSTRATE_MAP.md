# Perception-substrate map: the grounded inventory

This is a read-only study of how perception is wired in the engine today, produced at the reviewer's
direction during the Arc-2 (Mirror/Tempest calibration) bridge. It is doc-only: no code was written or
changed to produce it, and it proposes none. Its purpose is to hold the being-percept framing gate:
before any being-percept code is written, this map records exactly what the perception paths are, where an
entity-emission would integrate, and which couplings would author a high-level fact where physics should
be read. Every claim carries its `file:line` so the reviewer and owner can check it against source.

Three of the most load-bearing anchors were re-read from source and confirmed for this synthesis: the
`Trace` struct shape (`crates/sim/src/world.rs:269-293`, `subject` at 280, `attr` at 282), the
`feature_subject` signature (`crates/sim/src/learn.rs:142-145`), and the deliberation-weight gate
(`crates/sim/src/runner.rs:5387`). The reconciliation section at the end records which anchors were taken
from the underlying aspect maps without independent re-verification, and the one immaterial line-number
discrepancy between them.

---

## 1. How each path works

### Path A: the Trace / perceive / consider path

A `Trace` (`crates/sim/src/world.rs:269-293`) is a placed struct that already carries a fully formed belief
proposition. Its fields: `id` (`:271`, keys the perception roll), `place` (`:273`, co-location gate),
`channel: SenseChannelId` (`:278`, the physical channel), `subject: StableId` (`:280`), `attr: AttrKindId`
(`:282`), `hyps: Vec<ValueId>` (`:284`), `value: ValueId` (`:286`, the proposed value), `salience: Fixed`
(`:288`, perceptibility 0..1), `weight: Fixed` (`:290`, belief weight on success), and `from: StableId`
(`:292`, provenance). The doc comment (`:263-267`) states salience and weight are data carried from the
trace kind's reserved calibration, so the perception step invents no number of its own.

An emitter drops a trace with `World::emit_trace` (`world.rs:2632-2634`, whose sole body is
`self.traces.push(trace)`). The consuming phase is `World::perceive` (`world.rs:3871-3944`), run inside
`World::tick` (called `world.rs:2731`, and in the profiling twin `tick_timed` at `:2760`). `perceive` is
two-pass. The gather pass (`:3878-3928`) sorts the trace slice by id (`:3879`) and calls `gather_trace`
(`world.rs:3951` onward) per trace, serially or across workers with an order-reproducing merge.
`gather_trace` is a pure read of `&self`: it skips non-co-located minds
(`self.place_of.get(mind_id) != Some(&t.place)`, `:3952-3955`), applies the channel gate (below), computes
`chance = t.salience.mul(acuity)` clamped to 0..1 (`:3968`), draws a deterministic roll
`DrawKey::pair(mind_id.0, t.id.0, self.clock, Phase::PERCEPTION)` (`:3969-3971`), and on `roll < chance`
pushes a `PerceptionHit` (carrier struct `world.rs:307-315`) copying `subject, attr, hyps.clone(), value,
weight, from` verbatim (`:3972-3980`). The apply pass (`:3932-3943`, single-threaded, canonical order)
calls `mind.consider(hit.subject, hit.attr, hit.hyps.iter().copied(), hit.value, hit.weight, hit.from)`
(`:3934-3941`). `Agent::consider` (`crates/sim/src/agent.rs:183-200`) gets-or-inserts the `InferenceFrame`
for `(subject, attr)`, merges the hyps, and calls `frame.add_evidence(toward=value, weight,
acuity=self.acuity, from)`.

The channel gate lives in `gather_trace` at `world.rs:3956-3967`, keyed on `self.sensorium` (field
`world.rs:495`, populated by `set_sensorium` `world.rs:2557`). A mind with an installed sensorium reads a
channel only if `s.reads(t.channel)` returns `Some(a)` (`Sensorium::reads`,
`crates/sim/src/sensorium.rs:127-129`); `None` means blind and the loop `continue`s (`:3962-3964`). A mind
with no sensorium gets `channel_acuity = Fixed::ONE` (`:3965`), reading every channel at full acuity
(back-compatible default). The acuity product (`mind.acuity.mul(channel_acuity)`, `:3967`) scales the roll.
The channel id is a registry id `SenseChannelId(u32)` (`sensorium.rs:42-43`) with
`DEFAULT = SenseChannelId(0)` (`:48`), not a closed enum.

Note: no production path emits a trace today. Every `emit_trace` call site is a `#[cfg(test)]` unit test
(`world.rs:4693, 4724, 4746, 4771, 4789, 5753-5754, 5781`) or a demo (`crates/sim/examples/dawn_band.rs:116`,
`crates/sim/tests/dawn_band.rs:133`). The live runner reaches `consider` through the other arm:
`Stimulus::Observe` TickInputs flow through `tick` at `world.rs:2700-2717`, calling `mind.consider` directly
at `:2709`, bypassing traces, `perceive`, `gather_trace`, and the channel gate entirely.

### Path B: the feature-percept associative-learner path

Nothing here carries a proposition; the being senses a raw physical scalar and mints the proposition
locally. Three sibling registries share the pattern. The biology percept `PerceptRegistry::perceive`
(`crates/sim/src/percept.rs:129-135`) maps each declared `PerceptDef { id, class }` (`percept.rs:49-62`) to
`comp.map(|c| c.sensed(&p.class)).unwrap_or(Fixed::ZERO)`, a raw scalar vector in registry order over the
cell's edibility `Composition`; the runner reads it via `emb.percepts.perceive(emb.resources.composition(c))`
(`runner.rs:4302`). The material sibling `MaterialPerceptRegistry::perceive`
(`material_percept.rs:56-61, 117-125`) maps each substance to `mix.volume(&p.substance)` over the cell's
`SubstanceMix`; the runner reads the ground (`runner.rs:4377`) and what the being ate (`Some(&w.ate)`,
`runner.rs:4417`). Both registries are empty by default (`percept.rs:67-71`, `material_percept.rs:74-78`),
producing zero-width vectors. Temperature is not routed through a registry; the runner reads the field
gradient directly as a directional percept (`runner.rs:4849-4908`).

Quantization is `feature_bucket(raw, granularity)` (`percept.rs:150-157`): floor quantization
`raw.checked_div(granularity).map(|q| q.to_int() as i64)`, non-positive granularity reading as bucket zero
(fail-loud). Granularity is a reserved value with basis the sensorium per-class just-noticeable difference
(`percept.rs:145-149`), living as `HarmLearningCalib::feature_granularity` /
`RewardLearningCalib::feature_granularity` (read `learn.rs:518`, `:558`). The per-feature subject is
`feature_subject(channel: u16, bucket: i64)` (verified at `learn.rs:142-145`):
`StableId(FEATURE_SUBJECT_BASE | ((channel as u64) << FEATURE_CHANNEL_SHIFT) | bucket)`,
`FEATURE_SUBJECT_BASE = 1<<62` (`:120`), `FEATURE_CHANNEL_SHIFT = 32` (`:122`), bucket clamped non-negative
and 32-bit masked (`:143`). Material subjects offset by `MATERIAL_FEATURE_CHANNEL_BASE = 1<<15` (`:134`) so
they never alias biology subjects.

The associative link is in `Runner::couple_conversation` (`runner.rs:4242` onward). The harm felt-signal is
`harm = ...any(|axis| is_harm_tick(w.reserve_memory.delta(axis.id, &w.homeostasis), harm_learn.harm_noise_floor))`
(`runner.rs:4296-4301`), the being's own interoceptive reserve fall.
`feature_observations(harm, &features, plasticity, &harm_learn)` (`runner.rs:4309`, defined
`learn.rs:504-526`) buckets each present feature and mints `FeatureObservation { subject:
feature_subject(channel, bucket), toward: HARMS if harm else BENIGN, weight }`, weight being the reserved
observation weight scaled by heritable `plasticity` (`learn.rs:510-512`, plasticity from
`world.mind(w.id).plasticity`). Each is emitted as `Stimulus::Observe { subject, attr: HARM_ATTR, hyps:
vec![HARMS, BENIGN], toward, weight, from: w.id }` (`runner.rs:4313-4324`); `HARM_ATTR =
AttrKindId(u32::MAX-2)`, `HARMS=1`, `BENIGN=0` (`learn.rs:54-61`). The reward side (`runner.rs:4326-4441`) is
the sign complement on `REWARD_ATTR = AttrKindId(u32::MAX-3)`, `hyps vec![REWARDS, NEUTRAL]`
(`learn.rs:69-77`), feeding action-eligibility credit, material place-trace credit, and nutrition credit.
Belief-derived gradients `avoidance_gradient` (`learn.rs:581-625`) and `attraction_gradient`
(`learn.rs:627`+) read these committed beliefs back as direction percepts. Both paths commit through the
same general `Mind::consider` with no learner-specific commit logic.

The controller consumes these percepts as founder-zero input blocks in `ControllerLayout`
(`controller.rs:130`+): feature block (`:346`, base `:426-428`), appetitive, material-feature (`:347`),
attraction-direction, conviction (`:352`), then bias. The `with_percepts*` constructor chain delegates
outward to the terminal `with_percepts_appetitive_material_attraction_and_conviction` (`:335-388`); the
runner rebuilds the layout from installed registries via `rebuild_layout` (`runner.rs:1473-1481`), so a
percept must be installed before a controller is expressed.

## 2. Where an entity-emission would integrate

Two ingresses, differing in where the subject/attr/value are computed.

A matter-borne emission (a residue, a scent-substance, a deposited mound) integrates through Path B with no
new wiring beyond declaring the substance as a material percept. The source already contains a
being-to-material emission primitive: the geophage byproduct deposit (`world.rs:1170-1176`, whose comment
states it "reads only the eaten substance id and the map, never a belief, race, or kind") landing through
`MaterialField::deposit` (`material.rs:649`) and `SubstanceMix::add` (`material.rs:86`). A co-located being
re-earns a belief about that deposited volume through `MaterialPerceptRegistry::perceive`.

A non-matter emission (a sound, a flash, a transient EM or mana pulse) has no persistent cell-substance to
sense, so it integrates through Path A: the emitter drops a `Trace` on the matching `SenseChannelId` via
`emit_trace` (`world.rs:2632`), and co-located beings reading that channel perceive it. The belief-conversion
seam is therefore `emit_trace` time for Path A (subject/attr/value authored at emission) versus the
`feature_observations`/`reward_observations` mint site (`learn.rs:504-570`) for Path B (subject/attr/value
derived at perception).

## 3. Belief-conversion coupling risk

The two paths place the subject/attr/value computation in opposite places, and that is the whole coupling
story.

Path B is structurally unable to read the emitter's kind. subject = `feature_subject(channel,
feature_bucket(volume, granularity))`, where channel is the substance's registry slot and bucket is the
quantized raw volume; attr is a generic constant (`HARM_ATTR`/`REWARD_ATTR`); value is the perceiver's own
felt reserve sign. `feature_subject` (verified `learn.rs:142`) takes only a `u16` channel and an `i64`
bucket, and the value comes from the perceiver's interoception. The emitter is never named at the mint site:
it is dissolved into a cell scalar. This path cannot couple to an authored high-level fact about the emitter
even if one wanted it to. It is the alien-safe form: a photosynthetic or silicate emitter is one substance
id.

Path A carries the coupling risk, because the `Trace` fields subject/attr/value are set at emission, not
derived at perception. A `StableId` subject is by itself an opaque handle (id equality, no kind read), and
`from` provenance flows through as an opaque `EvidenceRef.from` (`evidence.rs:56-63`), so pointing a belief
at "this individual" stays observable. The seam opens if the emission picks the attr or the value by reading
the emitter's kind, species, lineage, or trophic role: e.g. `value = DANGEROUS` because the emitter is a
predator species, or selecting the `AttrKindId` a call implies from the caller's race. The machinery to keep
Path A physical-only already exists on the perception side: `TraceKindDef` (`trace.rs:163-187`) carries
`reliability` and `false_attribution` as data and computes evidence weight through Good's weight of evidence
over the race's own base rates (`good_weight`, `evidence.rs:305`; `trace.rs:30-33`), and
`TraceImplicationSpec` (`trace.rs:59-64`) declares `(attr, toward)` as a per-kind data row rather than a
code branch on a `TraceKindId`. So the answer to "can subject/attr/value be computed without reading kind":
for Path B, yes, by construction; for Path A, only if the emitted signal is made a data-defined signal-kind
row (the `TraceKindDef`/`TraceImplicationSpec` pattern) whose weight derives from base rates via
`good_weight`, so the conversion branches on a signal id and channel, never on what the emitter is.

## 4. Emission-physics finding: floor-law-derivable vs authored-table, per channel

The overriding structural fact: none of the emission laws below is wired to a perception channel today. The
current perception "signal" is the authored `salience` scalar on an opaque `channel` tag (`world.rs:278,
288`), and the reach-and-attenuation half is explicitly declared unbuilt in the sensorium module header
("What this brick does not build is spatial propagation ... The reach-and-attenuation half waits on a
coordinate and field model", `sensorium.rs:27-33`). The two feature-percept channels that read raw state read
it directly, not through an emission law: `comp.sensed(class)` (`percept.rs:129-135`) and per-cell substance
volume (`material_percept.rs:113-125`).

Per channel:

- Thermal / infrared: floor-law-derivable. `law.radiant_emission` (Stefan-Boltzmann,
  `crates/physics/data/chem_optics_floor.toml:220-232`, kernel `crates/physics/src/laws.rs:1479-1513`)
  derives net radiant power from `opt.emissivity`, `mech.contact_area`, two `therm.temperature` reads.
  `law.wien_peak` (`chem_optics_floor.toml:234-243`, kernel `laws.rs:1517-1522`) derives
  colour-from-temperature. `law.radiative_equilibrium` (`chem_optics_floor.toml:293-302`, kernel
  `laws.rs:1605-1631`) is the inverse.

- Visual / optical contrast: floor-law-derivable. `law.interface_split` (`chem_optics_floor.toml:257-267`,
  kernel `laws.rs:1554-1576`) splits incident flux from `opt.reflectance`/`opt.transmittance`.
  `opt.spectral_band` supplies per-band reflectance/emitted power over a data-defined band basis ("colour,
  not a hardcoded RGB triple", `chem_optics_floor.toml:146-155`). Supporting axes `opt.reflectance`,
  `opt.absorption_coefficient`, `opt.refractive_index`, `opt.scattering_coefficient`
  (`chem_optics_floor.toml:91-166`), with `law.optical_depth` (`:269-279`, kernel `laws.rs:1581-1586`) and
  `law.refractive_contrast` (`:281-291`, kernel `laws.rs:1590-1599`).

- Optical/acoustic spatial reach: floor-law-derivable and shared. `law.inverse_square_falloff`
  (`chem_optics_floor.toml:245-255`, kernel `laws.rs:1527-1548`) derives irradiance at distance from
  `opt.source_power` and `mech.arm_length`; the kernel doc calls it "the geometric-spreading half of a
  stimulus's spatial reach (light or sound)" (`laws.rs:1524-1526`). This is the exact "attenuating with
  range" computation the sensorium says has nothing to compute over: the law exists, the wire does not.

- Acoustic / vibration: floor-law-derivable. `acoustic.source_power` ("how far a call, shout, or song
  carries", `fluids_floor.toml:202-210`), `law.tube_resonance` (formant from resonator length and sound
  speed, `fluids_floor.toml:479-489`, kernel `laws.rs:1263-1292`), `acoustic.formant_frequency`
  (`fluids_floor.toml:247-253`), `law.speed_of_sound` (kernel `laws.rs:1213-1225`), `law.acoustic_absorption`
  (Stokes, `fluids_floor.toml:466-473`, kernel `laws.rs:1238-1253`).

- Chemical / scent: authored-table risk; the standing gap. The chemistry floor has `law.reaction`
  (`chem_optics_floor.toml:170-181`), `law.corrosion` (`:183-195`), `law.dissolution` (`:209-218`), and the
  corrosion/reaction kernels that decay a trace's salience over time (`trace.rs:294, 352`), but none converts
  a chemical source's state into a scent-concentration-at-distance signal. The nearest transport primitives
  `fluid.vapor_pressure`/`evaporation_rate` (`fluids_floor.toml:136-139`, kernel `laws.rs:1375-1397`) model
  the water cycle, not a per-emitter scent channel. So a scent channel today reads a raw local composition
  amount or needs a new diffusion/advection floor law; a curated channel-to-signal formula is the specific
  risk here.

- Electric / magnetic: field-generation laws exist, reach law does not. `elec.electric_field`
  (`em_floor.toml:92-93`), `mag.flux_density` (`:105-106`), generation laws `law.solenoid_field`,
  `law.dipole_torque`, `law.coulomb_force`, `law.lorentz_force`, `law.faraday_emf` (`em_floor.toml:173-338`).
  A body's emitted field is derivable, but no law converts field state to a perception-channel signal at
  distance (same unbuilt reach follow-on).

Bottom line: thermal, visual/optical, and acoustic channels avoid an authored channel-to-signal table by
construction (the signal is a read of grown physics); the chemical/scent channel is the standing curated-set
risk, resolvable by a diffusion/transport floor law rather than a per-channel formula table; and across every
channel the emission laws are not yet connected to perception.

## 5. Cognition-gate finding: possession vs level, and where the deliberative tier is gated

Cognition is graded as component-possession, not an authored capability tier. There is no boolean "is
sentient" flag, no assigned level, no predicate reading a status rank to decide a being thinks.

The `Mind` struct (`crates/sim/src/agent.rs:84`) holds three live cognition scalars, `acuity` (`:90`),
`memory` (`:97`), `plasticity` (`:101`), plus the belief store, the theory-of-mind `models` map (`:103`),
`wondering`, and `relations`. `Mind::from_genome` (`agent.rs:148-174`) reads each channel off the gene set
through the same deterministic `GeneSet::express` every phenotype uses: acuity from
`Channel::Cognition(CognitionChannel::ReasoningAcuity)` (`:149-153`), memory from `MemoryCapacity`
(`:154-158`), plasticity from `BeliefPlasticity` (`:159-163`). `express` (`genome.rs:404-426`) is a plain
additive walk over loci with dominance deviation, no thresholds, no tier gate; a being with no gene feeding a
channel expresses the bare environment baseline and still produces a `Mind`. `CognitionChannel`
(`genome.rs:85-93`) is exactly three kept-distinct axes under `Channel::Cognition(...)` (`:144`); the header
(`:137-138`) states the channel set is the fixed engine interface while which genes reach it is data
(Principle 11). At seeding, every band member gets a mind from its promoted genome (`world.rs:1362`, inserted
`:1373`), and each birth does the same (`world.rs:1834, 1845`). `Mind::new(id, acuity)` exists for fixtures
(`agent.rs:127`, used by `World::spawn` `world.rs:1310`), but `from_genome` is documented as the real
derivation (`agent.rs:24-27, 124-126`).

Two further heritable drives feed the acting body, not the `Mind`: `Channel::Exploration`
(`genome.rs:196-204`) to `Walker::exploration` (`locomotion.rs:606`), and `Channel::Deliberation`
(`genome.rs:205-211`) to `Walker::deliberation` (`locomotion.rs:625`). Both are founder-zero via an
unseeded-locus mechanism (`genome.rs:199-202, 209-210`), expressed through `express_unit` clamped to [0,1]
(`:435-438`). Each newborn expresses its own exploration, deliberation, and social-learning off its genome
(`runner.rs:5916-5926`).

The deliberative tier is gated by two possessions in series, not a rank. First, possessing a `Mind` at all:
the deliberation/planning pass filters on `let mind = world.mind(w.id)?;` (`runner.rs:5122`), so a walker
with no mind entry is skipped by the `filter_map`; planning (`plan_chain`, `runner.rs:5160-5168`) reads only
that mind's own beliefs and relational edges. Second, possessing a positive deliberation weight: the
enactment gate is `if w.deliberation > Fixed::ZERO` (verified `runner.rs:5387`), after which a counter-keyed
draw fires with probability equal to the weight (`:5388-5392`). The comment (`:5377-5385`) states the
founder-zero contract: a founder with zero weight never deliberates, so goal-directed pursuit emerges by
selection, "never a coded 'when idle, plan' (Principle 9)." The weight folds into `state_hash` only when
positive (`runner.rs:6074-6079`), so an opted-out run stays byte-identical. Theory-of-mind is likewise
possession: every `Mind` carries the `models` map (`agent.rs:103`) and `Mind::model` (`agent.rs:217-233`),
which admits only access evidence about the target; no predicate assigns "this being has ToM".

The one authored-tier phrase ("sentient deliberative tier") names the authored decision policy of design
Part 8.1 (`World::set_behaviour`), which the canonical runner refuses: `with_world` asserts
`!world.has_behaviour()` and rejects a world carrying an authored decision repertoire (`runner.rs:3039-3052`),
because the canonical-emergent behaviour source is the evolved controller, not an authored policy. So the one
place a capability level could enter is barred from the canonical spine.

## Cross-map reconciliation and unverified flags

This map was synthesized from five independent aspect studies. The reconciliation of their small
discrepancies, and the flags on which anchors were not independently re-verified here:

- Trace field line numbers: one aspect cites `subject:280, attr:282`; another cites `subject:281, attr:283`.
  Verified against source: the struct is `world.rs:269-293`, `subject` at 280, `attr` at 282. The first is
  correct; the second is off by one. Immaterial to any claim.
- Trace struct span: one aspect says 268-293, another says 269-292. Verified: the `#[derive]` is at 268,
  `pub struct Trace` at 269, closing brace at 293. Both are loose framings of the same struct.
- All five maps agree that Path B (feature-percept) never touches the sensorium channel gate and that the
  gate lives only in `gather_trace` (`world.rs:3956-3966`); one aspect verifies `percept.rs` has no
  `use crate::sensorium`. No disagreement.
- All five maps agree the Trace/perceive path has no production emitter today (test and demo call sites
  only). No disagreement.
- Not independently re-verified in this synthesis (taken from the aspect maps, each with cited file:line the
  reviewer can check): the physics floor law kernel line ranges in the emission-physics section (`laws.rs`,
  `chem_optics_floor.toml`, `fluids_floor.toml`, `em_floor.toml`), the `TraceKindDef`/`good_weight` anchors
  (`trace.rs:59-187`, `evidence.rs:305`), the geophage deposit primitive (`world.rs:1170-1176`), and the
  controller block bases (`controller.rs`). The three most load-bearing anchors for the framing (Trace struct
  shape, `feature_subject` signature, the deliberation-weight gate) were re-read from source and confirmed.
