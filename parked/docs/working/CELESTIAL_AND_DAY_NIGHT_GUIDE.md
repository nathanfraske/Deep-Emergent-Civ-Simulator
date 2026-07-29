# The Celestial and Day-Night Model: Suns, Moons, Stars, and the Sky

**A standalone design for how a world's sky is modelled, how day and night, seasons, tides, phases, and eclipses arise, and how agents perceive and use the heavens, without authoring an Earth-like default.**

Status: a design doc in the project's style. The representation and the structure are derivable from the existing parts and the project's ideology and are stable enough to build against, with the standing clause that any locked decision yields to evidence. The orbital-mechanics and illumination law forms, the body-property axis catalogue, and every parameter are research-populated and owner-reserved, never fabricated here. The research questions are posed in Section 8.

---

## 1. What already exists

Several systems already touch the sky, and this design grounds and connects them rather than replacing them.

- **The light field (Part 5).** Sunlight reaching a cell is a GPU-computed environmental field, drawn on by plant growth as a data-defined input and trackable by drives, so an illuminance field over the grid already exists.
- **Climate and seasons (Part 18).** Seasons are the climate fields cycling annually, with amplitude set by latitude, driving plant growth and migration. The cycle is modelled, but with no stated physical cause.
- **Worldgen and topology (Part 12).** The world is generated, and its topology, including spherical, is a seeded start variable, which is what makes latitude, rotation, and a real sky meaningful.
- **The emic measurement system.** The measurement systems cultures invent already allow a unit of time to originate in a celestial cycle, which is the seam an emergent calendar lands on.
- **Perception, technique, belief, and magic.** Witnessing forms belief from what an agent perceives (R-EVIDENCE, Part 9); practical knowledge is a technique a culture accumulates and can lose (Part 23); belief hardens into axiom and cosmology (R-AXIOM, Part 28); and magic laws are authored under Principle 9 (Part 34, with the data definitions of Part 40). These are the systems that turn a sky into navigation, calendars, myth, and ritual.

## 2. The gap

What is wholly absent is a day-night cycle, any model of suns, moons, or stars as bodies, and therefore tides, lunar phases, and eclipses. The light field has no celestial source, seasons cycle with no physical cause, and there is no diurnal cycle at all.

## 3. The principle, consistent with the rest of the project

The same move the project makes everywhere: author the space, generate the instance, feed the fields, and let use emerge. A celestial body is a point over property axes rather than a hardcoded sun, moon, or star, the same shape as a material over the physics axes. The sky is a generated configuration that is part of the world's profile and its deterministic identity, the same as the physics and biome profiles. Day and night, seasons, tides, phases, and eclipses are measured consequences of the mechanics rather than stored states, the same way chopping is measured from physics. And what agents do with the sky emerges through the perception, technique, belief, and magic systems rather than being authored.

## 4. The locked representation (decided now, overridable on evidence)

**A celestial body is a point over property axes.** Whether it emits or only reflects light, its luminosity and spectrum, its reflectance, its mass, its orbital elements, and its apparent size are data, so a sun is a high-luminosity emitter, a moon is a low-luminosity body close enough to show a disk and raise a tide, a star is a distant point emitter, and a planet a distant reflector, all regions of one space. The engine never switches on a body's kind; it computes from the properties. Exotic axes (a mana-emitting body) sit on the same bounded-but-extensible floor as the physics axes, with the same real-with-source versus fantasy-reserved-with-basis provenance split.

```
CelestialBody {
    id,
    luminosity: Fixed,        // emitted radiant power; 0 for a pure reflector
    spectrum: SpectrumId,     // emitted or reflected spectrum (data-defined)
    reflectance: Fixed,       // fraction of incident light re-emitted; a reflector is lit by emitters
    mass: Fixed,              // sets tide-raising strength
    orbit: OrbitElements,     // period, semi-major axis, inclination, phase at epoch (all fixed-point)
    apparent_size: Fixed,     // angular size from the surface; sets disk-vs-point and eclipse coverage
    provenance: RealWithSource(citation) | FantasyReserved(basis),
    exotic: SmallVec<[(AxisId, Fixed)]>,   // mana emission and the like, on the extensible floor
}

WorldSky {
    rotation_period: Fixed,   // the world's spin; sets the day length
    axial_tilt: Fixed,        // sets the seasonal swing
    bodies: SmallVec<[CelestialBody]>,   // zero or more emitters and reflectors
    starfield: Option<StarfieldId>,      // present or absent; a fixed or slowly-precessing pattern
}
```

**The sky is a generated profile.** How many emitters, how many secondary bodies, which emit and which reflect, whether there are stars, the rotation, and the axial tilt are seeded at worldgen and carried in the world profile, so multiple suns, no suns, a moon or none, a self-luminous moon, and a starless sky are all points in the configuration rather than special cases. The closure that derives a world's physics and biosphere from its profile derives its sky the same way.

**Everything observable is a measured consequence.** Illumination at a place and time is computed from the positions and properties of all bodies given the world's rotation and the observer's location, so day and night are an illumination threshold and a two-sun world's day is the aggregate of its emitters. Seasons are the consequence of axial tilt and orbit, which gives the existing annual cycle its cause. Tides are the consequence of a body's mass and position. Phases are the consequence of a reflector's geometry relative to its illuminator and the observer. Eclipses are the consequence of alignment and apparent size. None of these is stored as a state.

**Determinism binds all of it.** The bodies' positions, the illumination, and every derived field are fixed-point closed-form functions of the world profile and the world time, so the sky's state at any tick is reproducible from the seed, and the sky is part of the world's deterministic identity.

## 5. What the sky feeds

The model is upstream of fields that already exist, which is why the gap matters.

- **The light field (Part 5)** sources from the aggregate illumination, which adds the diurnal cycle plant growth does not currently get and lets a sunless world's light come from an ambient, bioluminescent, or magical source, since the light input cares about illuminance and not about what emits it.
- **Climate (Part 18)** gains a diurnal temperature swing and a physically caused seasonal swing from the illumination and the axial tilt.
- **Agent activity** gains a day-active or night-active disposition as a trait axis, which the being model and the food web already have the shape for.
- **Perception (R-EVIDENCE, Part 9)** couples to the light level, so night impairs witnessing and fewer beliefs form, ambushes are easier, moonlight partly restores it, and a nocturnal creature sees in low light. The celestial light level becomes a modulator on the witnessing system.

## 6. What emerges from the sky

Use is emergent and reach-bounded, never authored.

- **Perception of the heavens.** Bodies are observable features, witnessed through the same system as anything else when the light and sky allow, so agents perceive the sun, the moons, and the stars when they are present and visible.
- **Celestial navigation** emerges as a technique (Part 23) where there is a perceptible regular pattern and a real need, so wayfinding by a pole star can arise on an open sea, and a permanently clouded or starless world cannot develop it, which is a reach consequence rather than a missing feature.
- **Calendars and timekeeping** emerge from observing the day, the lunar phases, and the seasonal sun, landing on the existing emic hook that lets a time unit originate in a celestial cycle, so a world with a moon can grow lunar months and a world without one cannot.
- **Cosmology, myth, and religion** emerge from agents forming beliefs about the sky (R-EVIDENCE, R-AXIOM), so a sun becomes a sun-god, a constellation a story, and an eclipse, a computed alignment witnessed as an event, an omen that can shift an axiom or trigger a religious episode.
- **Magic** reads the computed celestial state as an authored magic axis under Principle 9, so magic stronger at full moon or a ritual timed to an alignment is the existing magic-axis pattern over a celestial input.

## 7. How this composes

It adds two things, the body-and-sky representation and its illumination and mechanics laws, and reuses everything else: the light field, the climate layer, the being model's trait axes, the witnessing system, the technique and belief and institution systems, and the magic axes. It draws its exotic axes and its provenance discipline from the physics and materials substrate, and it composes with the world profile, so a humans-and-base-physics world generates a single sun, perhaps a moon, real stars, and no magical bodies, while a fantasy world can carry two suns and three moons and a mana-emitting star, because the sky generator reads that world's active profile.

## 8. The research questions

These are the open questions this design poses, with what grounds each. None is answered here; a fabricated law or axis set would be the steering leak the project audits for.

- **The orbital-mechanics law form.** How body positions are computed over world time in fixed-point closed form and cheaply enough to run for the whole world over deep time. A full N-body integration is neither closed-form nor cheap nor trivially reproducible, so the question is which reduced model (Keplerian elements evaluated per tick, tabulated ephemerides, or a bounded approximation) gives deterministic positions at acceptable cost, and how its error is bounded.
- **The illumination law.** How the bodies' positions and properties combine into the illuminance at a cell, including multi-emitter aggregation, atmospheric scattering and the sky's own brightness, and the day-and-night threshold, and how this drives the existing light field.
- **The climate coupling.** The forms of the diurnal temperature swing and the seasonal swing as consequences of illumination and axial tilt, and whether they re-ground or replace the current abstract annual cycle.
- **The tide law and its couplings.** The form of the tide-raising effect from a body's mass and position, and what it couples to: coastlines, the water field, fishing, and navigation.
- **Phase and eclipse geometry.** How a reflector's phase and a body's eclipse are computed from geometry and apparent size, and how an eclipse surfaces as a witnessed event into the belief system.
- **The perception coupling.** How the light level gates the witnessing system: the visibility threshold, the contribution of moonlight, and how nocturnal vision is represented.
- **The starfield model.** Whether stars are a fixed pattern, a slowly precessing one, individually resolved or a field, what makes them perceptible and therefore usable for navigation and calendars, and how precession over deep time interacts with a culture's accumulated star knowledge.
- **The body-property axis catalogue.** Which axes a celestial body varies over beyond those locked above, and the exotic and fantasy axes, owner-reserved.
- **The parameters.** Day length, axial tilt, orbital periods and distances, luminosities, the illumination and temperature coupling constants, and the visibility thresholds, each surfaced with its basis and split real-with-source or fantasy-reserved-with-basis.
- **The seasons decision.** Whether to re-ground seasons on axial tilt and orbit or to keep the existing abstract annual cycle, which is an owner decision.

## 9. Settled versus open

**Locked now, overridable on evidence.** The principle (a celestial body as a point over property axes, the sky as a generated profile-level configuration, and day and night, seasons, tides, phases, and eclipses as measured consequences); the body-and-sky representation; the couplings into the light field, climate, agent activity, and perception; the emergence of navigation, calendars, cosmology, and magic through the existing systems; and the determinism constraints.

**Open, research-populated and owner-reserved.** Everything in Section 8: the law forms, the axis catalogue, the parameters, and the seasons decision.

## 10. Tracking and the fan-out

This is a candidate research item in its own right, and it warrants the same treatment as the physics and biosphere work: a fan-out across celestial mechanics, climatology, and the perception and cosmology consumers, with a red team that attacks whether the chosen position model is deterministic and cheap at world scale, whether day and night and seasons stay measured rather than stored, and whether celestial use stays emergent and reach-bounded rather than slipping into an authored capability. If it is taken up, the clean way to track it is as its own backlog item with this document as its vehicle, sequenced alongside the physics substrate, since the illumination and exotic axes read through it.

---

## Editorial note (added on intake, 2026-06-30): reference verification

The part references in this document were checked against the design on intake, since this is a candidate vehicle the project will build against. The original prose above is preserved; this note records the audit.

Sound as written: the light field (Part 5), which the design's own flora input enum cites as "(Part 5)" for sunlight as a GPU-computed field; climate and seasons (Part 18), where seasons cycle annually with amplitude set by latitude and, confirmed, with no stated physical cause, which is the gap this design fills; worldgen (Part 12); perception and belief from witnessing (R-EVIDENCE, Part 9); technique accumulation and loss (Part 23); belief hardening into axiom and cosmology (R-AXIOM, Part 28); and magic laws authored under Principle 9 (Part 34) with the data definitions of Part 40.

Two refinements. First, the emic measurement hook, "a unit of time to originate in a celestial cycle," is left uncited above; its precise home is Part 55, the emic layer of the unit system, whose text names exactly "a span of time from a celestial cycle." Second, the topology reference is imprecise: worldgen is Part 12, but the macro topology including the spherical case, carried as a seeded start variable, is settled in Part 56 (the `Topology` enum of FlatBounded, FlatWrapped, Cylindrical, Spherical) under the start-variable framing of Part 40. So the sentence "Worldgen and topology (Part 12)" should read worldgen (Part 12) and topology (Part 56, a seeded start variable per Part 40). Latitude and rotation, which the doc leans on for a real sky, follow from that Part 56 topology choice.
