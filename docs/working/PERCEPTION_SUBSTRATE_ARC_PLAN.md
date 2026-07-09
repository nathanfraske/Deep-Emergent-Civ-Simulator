# Perception-substrate arc: the derive-first plan (proposal for the gate)

This is a PROPOSAL, not code and not a built design. It reframes Arc 3 per the gate agent's direction: the
being-perception percept is gated on a perception-substrate arc that precedes the predation loop, because six
independent section-11 framing catches showed the keystone's cleanliness depends on unbuilt substrate. Each
piece below is stated derive-first (how much derives from the physics floor, what substrate makes it derive)
so the gate can rule on it against the derive-vs-author line before it becomes a slice. Nothing here is built
until the gate rules and the piece is framed blind (section-11 fail-closed, then the section-10 panel), with
every decisive claim source-verified. The grounding is `PERCEPTION_GAPS_STUDY.md` and the three
source-verified catches.

## The design frame

The live perception-to-learning path is Path B (`runner.rs:4284-4361`), and its emergence-critical core
already derives clean and alien-clean: the felt value sign is the being's own conserved-reserve delta
(`homeostasis.rs:1089-1107`), and the raw percept is the true physical quantity in the being's own cell, learned
by correlation reading no label (`learn.rs:504-526`). The being-percept is built ON this core, never on the
Path A `Trace.value` stamped at emit (which has no production caller and would author meaning).

Three hard constraints, from the principles and the catches, bind every piece:

- (a) An emitted signal carries a PHYSICAL MAGNITUDE, never an implication or valence set at emit.
- (b) Perception keys on the being's OWN installed senses, so an alien lacking a sense does not perceive that
  channel. This reconciles Path B's current ungated universality (`percept.rs:129-134`).
- (c) Meaning and valence EMERGE receiver-side under selection, never stamped at emit or in an authored table.
  This reuses the existing felt-outcome correlation learner (`learn.rs:504-526`).

The general shape of the keystone, once the substrate is built: a being emits a physical signal (a magnitude on
a sense channel, derived from its own body state); the signal reaches nearby beings attenuated by distance and
medium; a perceiving being senses it only on a channel its own sensorium reads, at its own acuity; and the
being learns what the signal means by correlating it with its own felt outcome, exactly as it learns that salty
ground harms it. Predation, fleeing, alarm, and social response emerge from that one mechanism under selection,
with no species tag, no valence table, and no emit-time meaning.

## Slice 1: the reach wire (the clean first slice, physics-floor, no valence risk)

What it builds: the spatial half, a signal traveling from an emitter cell to a perceiver cell and attenuating
with distance and medium. No meaning, no valence, pure physical magnitude, so it carries no authoring risk on
constraint (a) or (c).

Derive-first analysis. The attenuation mechanism is fixed Rust reading floor laws that already exist as tier-0:
`law.inverse_square_falloff` (geometric spreading, `laws.rs:1527`), `law.optical_depth` (Beer-Lambert medium
attenuation, `laws.rs:1581`), `law.acoustic_absorption` (Stokes, `laws.rs:1238`). These have zero live
perception callers today (confirmed by grep); wiring them to a perception read is the slice. The distance the
laws need is a geometric quantity computable from the coordinate model Path B already carries (`w.coord()`, the
same coordinates the thermal field diffuses over), so distance derives from the situation, not from a new
authored value. The emitted magnitude at the source (the power the falloff law spreads) derives from the
emitter's own physical state (its body, its motion, its metabolism), or from a per-channel emission law.

What is authored/reserved, surfaced not fabricated: nothing new should be authored inline. The medium
attenuation reads the medium's own floor coefficients (the `MediumField` already carries these). If the emitted
source magnitude cannot yet derive from a body-state emission law for a given channel, the emission law is the
gap to build (below), not a fabricated source scalar.

Alien check and the derive-vs-author seam to flag. Inverse-square geometric spreading is channel-general (it is
geometry, not Terran chemistry), so it admits any channel. But medium attenuation and the source emission law
are channel-specific: a mana-field or redox-gradient channel need not fall off as inverse-square nor emit by
Stefan-Boltzmann. The current floor carries `opt.source_power` and `acoustic.source_power` as two hardcoded
channel axes, not a data-driven per-channel power-and-propagation registry. The alien-clean form reads the
channel's OWN emission-and-propagation law as data, so a novel channel's physics is a data row. Whether to build
that per-channel registry now, or wire the Terran channels first and flag the alien channels as data follow-ons,
is a sequencing question for the gate.

The one real floor question this slice raises: the reach laws take a distance PORT that currently borrows
`mech.arm_length` (`chem_optics_floor.toml:250`), a body dimension standing in for an inter-entity separation.
Whether to generalize that port to a first-class source-to-perceiver separation (derived from coordinates) is a
small floor-plumbing decision to surface with the gate, because it touches the floor.

## Slice 2: the sensorium-gated magnitude percept (subtle)

What it builds: a being perceives the attenuated signal only on a channel its own sensorium reads, at its own
acuity, and the percept it forms is the PHYSICAL MAGNITUDE, never a valence. This reconciles constraint (b) and
Path B's ungated universality.

Derive-first analysis. The sensorium substrate already exists and already gates Path A: `Sensorium::reads`
returns the being's per-channel acuity or None if it is blind to the channel (`sensorium.rs:127`), and
`Sensorium::resolution` carries the per-channel just-noticeable difference (`sensorium.rs:134`). The slice is to
gate the live Path B percept on the being's sensorium (perceive a channel only if the being reads it, scale the
magnitude by acuity, quantize at the being's own JND), the mechanism the study ranked as items 3 and 6. The
being's per-channel acuity and JND should DERIVE from its genome and anatomy through the same `GeneSet::express`
machinery that already produces `mind.acuity` (`sensorium.rs:52-63` names this the follow-on), so which senses a
being possesses, and how keen each is, emerges from its evolved body, never an authored per-being sense list.

The blocking sub-piece and the Terran default to flag. Two dependencies stand between here and a clean gate.
First, the percept currently keys on a substance-class string while the sensorium keys on a `SenseChannelId`, so
a percept-class-to-channel binding must be added before the JND can be read per percept (the study's item 3
block). Second, the anatomy-to-sense transduction that would derive per-channel acuity is optical-only:
`anatomy.rs:338-362` carries a data-defined sense list, but only the optical sense is physics-derived, and the
acoustic, chemical, field, and mana senses carry an `opt.refractive_index` placeholder. Deriving a non-optical
or alien channel's acuity from its own physics needs a per-channel transduction kernel; until it exists, this
slice would either wire only the optical channel cleanly or carry the Terran placeholder as a flagged interim.
This is the subtle derive-vs-author point in the slice and the gate should rule on how far to build it.

Alien check. Gating on the being's own sensorium IS the admit-the-alien fix: an anosmic being has no scent
channel in its sensorium and does not perceive scent, and a novel sense is one data row in the sensorium and
the channel registry (an opaque `SenseChannelId`, alien-clean by Muller's law of specific nerve energies). The
risk is entirely in the transduction derivation above, not in the gating.

## Slice 3: the receiver-side valence learner (subtle)

What it builds: a being learns what a perceived signal MEANS by correlating it with its own felt outcome, so a
signal comes to predict danger or reward receiver-side under selection, never stamped at emit. This reconciles
constraint (c) and catch 1.

Derive-first analysis. The mechanism already exists for environmental features and needs only to consume the
being-signal percept as one more feature. `feature_observations` (`learn.rs:504-526`) takes a raw percept and
the being's own interoceptive harm bit and mints a belief toward HARMS or BENIGN, "nothing read but the raw
feature and the felt sign (Principles 8, 9)". A being-signal, keyed by `feature_subject` on its sense channel,
is a feature; correlating it with the receiver's own reserve delta is the identical learner. So the valence
emerges from selection: a being learns another's alarm call means danger because perceiving it correlated with
its own harm, exactly as it learns salty ground harms it. Meaning is never at the emitter.

What is authored/reserved, and the deepest gap in the whole arc. The head of this learner is the study's item
1: the observation weight is `good_weight` over two authored likelihood scalars, `p_harm_given_harms` and
`p_harm_given_benign` (`learn.rs:288-289,315`). The `good_weight` composition is derive-clean (I.J. Good's
weight of evidence); the two probability inputs are authored simplifications. Their deriving substrate exists
(the floor dose-response harm law plus the metabolic-noise distribution of the reserve delta), but deriving them
is an ESTIMATOR of P(signal given cause) rather than a read, so this is the piece most likely to need real
design work and the one to surface most carefully to the gate. One step upstream, the noise-floor sign gates
(item 2) are a near one-line read of the existing per-axis `DerivedDrain.base`, and should be wired in the same
slice so the label the weight is applied to is itself derived.

Alien check. The learner reads no chemistry, kingdom, or body plan: it correlates a channel-keyed feature with a
conserved-reserve delta on whatever axis the being's data defines. A mana-metabolism perceiver learns a
mana-draining signal is harmful from its own mana delta. The learner is alien-clean by the same construction
that makes the environmental learner alien-clean.

## Open decisions surfaced for the owner, none fabricated

These are the forks the substrate raises that are the owner's to rule on, each with a recommendation and the
reason, per the gate agent's standing instruction (surface on the new PR, build past reversible calls, defer
only true owner-calls). None is a fabricated value.

- The per-channel emission-and-propagation registry (alien-feasibility). Recommendation: build the reach wire
  on the Terran channels first (slice 1), flag the non-Terran channels' emission and falloff as data
  follow-ons, so the arc delivers a working reach substrate without stalling on the mana/redox physics. The
  registry is the alien-clean end state; sequencing it after the first wire is reversible.
- The inter-entity distance port on the reach laws. Recommendation: derive distance from the existing Path B
  coordinate model and pass it into the reach law, rather than adding a floor axis, since distance is a
  geometric quantity of the situation and not a material property. Flag whether the law's `mech.arm_length` port
  should be renamed/generalized, a floor-touching change.
- The transduction kernel for non-optical senses (Terran-default risk). Recommendation: wire the optical channel
  cleanly in slice 2 and carry the non-optical acuity as a flagged interim placeholder until per-channel
  transduction kernels exist, rather than deriving a non-optical acuity from a borrowed refractive index. The
  interim is reversible; the kernels are their own build.
- The likelihood estimator (the deepest gap). This is a genuine design question, not a value to set: how P(signal
  given cause) is estimated from the dose-response law and the reserve-delta noise. Recommendation: scope it as
  its own framed-blind piece within slice 3, because getting it wrong authors the learning rate. Surface it to
  the owner as the arc's real design risk.
- A chemical/scent diffusion field (for distal chemoreception). Recommendation: note it as the natural home for a
  matter-borne being-signal (a scent), a `Field::step` analogue that derives its diffusion coefficient from the
  medium exactly as the thermal field does (`runner.rs:299-303`); sequence it as an Arc extension after the
  three core slices, since sound and light reach cleanly through slice 1 without it.
- Retention of the associative beliefs (a coupling, not a gap in this arc). The harm/reward beliefs never decay
  live (`agent.rs:29,91-95`); a being-signal belief probably should decay via the existing `RetentionLaw`.
  Recommendation: flag it as a coupling to resolve when the learner slice lands, not a blocker.
- The closed `AffordancePerceptKind` enum (a Principle-8 seam the study surfaced, adjacent to this arc). Not part
  of the being-percept, but a live perceive-path authoring seam (`affordance_percept.rs:96-121`). Recommendation:
  log it for a separate audit, do not fold it into this arc.

## The sequence and the discipline

The order is reach wire (slice 1, the clean physics-floor magnitude), then the sensorium-gated magnitude percept
(slice 2, admit-the-alien gating), then the receiver-side valence learner (slice 3, emergence of meaning). The
predation loop (the strike-affordance arm and the being-vs-being harm) consumes the substrate second, once the
three slices land.

Each slice is framed blind BEFORE any code: the section-11 input-bias smoke test fail-closed on the slice's
framing, then the section-10 panel, with every decisive claim source-verified. Cadence is push per segment,
byte-neutral opt-in, and the section-9 five-lens audit at arc end. The gate rules on each proposal here against
the derive-vs-author line before it becomes a slice.
