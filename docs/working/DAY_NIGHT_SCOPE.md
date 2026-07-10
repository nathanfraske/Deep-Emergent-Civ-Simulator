# Day-night arc: derive-first scope (Agent C, gate-directed while the Nernst holds on the owner)

The gate directed a derive-first scope pass on local lighting, surface heat, rotation, and day-night: consume the
existing rotation to drive a diurnal cycle so the sun sweeps, giving per-cell insolation by latitude and time, feeding
a cycling light field and a diurnal surface-heat swing through the dead radiative-equilibrium kernel. Grounded against
source below: what already exists to wire, what is new to build, and the cross-surface touchpoints.

## What exists (wire, do not author)
- The rotation data: `celestial.rs OrbitalElements::rotation_period_seconds` (world-seconds per rotation, the length of
  a day; Earth dev fixture 86,400 s). Sibling `orbital_period_seconds` carries the year. These are owner-set per-world
  scalars, dev-fixtured to Earth, not authored constants.
- The seconds-to-ticks bridge: `clock.rs ticks_from_seconds` (a span of world-seconds over the base-tick duration,
  floored to whole ticks, fixed-point, fail-loud), the canonical way a world's own orbit sets a cadence rather than a
  hardcoded constant. The canonical tick count is the `Steppable::tick()` readout.
- The spatial insolation shape: `environ.rs latitude_light(y, height)` (mine), equator = 1, poles = 0, currently with
  NO time argument. It is called once in `from_map`, so today the light field is a STATIC per-cell latitude map.
- The dead surface-heat kernel: `laws.rs radiative_equilibrium(absorbed_irradiance, emissivity, sigma, t_max)` (A's
  file), the inverse emission law that SETS a surface temperature from absorbed irradiance. It exists and is tested but
  has no consumer. I would CONSUME it (call the pub fn), not edit `laws.rs`.

## What is new (build, derive-first, no authored value in the content path)
- A diurnal PHASE: from the canonical tick and `rotation_period_seconds` (via `ticks_from_seconds`), the sun's position
  over the day, a fixed-point phase in `[0, 1)` (tick modulo ticks-per-rotation). Deterministic, a pure function of the
  tick.
- Time-dependent insolation: the per-cell instantaneous insolation as the spatial `latitude_light` shape times a
  diurnal-angle factor (the sun's height over the local horizon from the phase and the cell's longitude offset),
  clamped at the night side to zero. This EXTENDS `latitude_light` to carry the phase, or adds a sibling that folds it.
- A per-tick light-field update: today's light is static (build-time). The diurnal field must recompute per tick from
  the phase, so the light a being perceives and a producer photosynthesizes with cycles day to night.
- The surface-heat consumer: feed the instantaneous insolation as absorbed irradiance into `radiative_equilibrium`
  (dead to live) to drive a diurnal surface-temperature swing, coupled into the temperature field the hydrology and
  productivity already read.

## Cross-surface touchpoints (coordinate before code, same discipline as the Nernst)
- `environ.rs` (MINE): `latitude_light`, the light field, the insolation, the heat coupling. The bulk is here.
- `laws.rs radiative_equilibrium` (A's): CONSUMED only (a pub-fn call), no edit. Confirm that reading is correct.
- The TICK into `environ.step`: `EnvironFields::step(&mut self, temp, calib)` takes NO time argument, so a diurnal
  step needs the current tick threaded in. Its caller is the runner loop (A's `runner.rs`). Options to weigh in
  framing: thread the tick through `environ.step` (touches A's runner call site), or have `EnvironFields` hold its own
  monotonic tick counter advanced each step (self-contained, no runner signature change, byte-neutral if the diurnal
  path is off by default). The self-counter keeps the surface disjoint from A; the threaded tick is cleaner but
  cross-surface. This is the load-bearing coordination question.
- Byte-neutrality: the diurnal path must default OFF (a world with no rotation, or the cycle unarmed) so the four
  determinism pins hold; the static latitude light is the fallback. The armed diurnal cycle is opt-in like `living`.

## Emergence and principles check (why this still needs blind framing)
Lighting and surface heat are PHYSICS-FLOOR inputs (Principle 9 permits authoring physics), so a diurnal insolation
law is legitimate floor growth. The risk to frame blind: that the day-night wiring authors a downstream cultural or
behavioural outcome (a hardcoded "beings sleep at night", a templated activity rhythm) rather than letting behaviour
emerge from the cycling light and temperature the evolved controller perceives. The insolation and heat must be pure
physical fields; any diurnal behaviour must EMERGE from selection over those fields, never be wired. That is the
steering line to hold, and the reason to frame the arc blind before code.

## Next step
Frame the arc blind (section-11 smoke then section-10 panel) on the design statement (the diurnal phase, the insolation
extension, the heat coupling, and the tick-threading choice), surface it and the A-coordination question to the gate,
then build byte-neutral or report. No code until the framing clears and the tick-plumbing surface is agreed with A.
