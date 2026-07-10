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

## Blind framing result (section-10 panel, cleared the section-11 smoke on round 2): surfaced for the gate

The framing panel (five lenses, behind a section-11 smoke test) confirmed the steering line holds in principle
(insolation and heat are physical-floor growth, the phase counter withheld from the percept, no diurnal behaviour
authored) but caught that the naive diurnal wiring bakes several Terran-geometry assumptions and physics gaps. The
load-bearing findings are verified against source or by astronomy; they turn "mostly wiring" into a real physics-design
decision with a scope fork for the gate and owner.

Steering (the crux, both fixable and adoptable). Seam (a): the phase counter and tick must be AFFIRMATIVELY firewalled
from the percept and every behavioural substrate (a private field or separate module or explicit exclusion), not merely
"not exposed": an un-firewalled clock adjacent to the percept is a latent authoring channel (a substrate could branch
on phase to template nocturnality). Seam (b): the pre-existing hydrology and productivity consumers were authored
against a STATIC light field and now read a CYCLING one, so each authored threshold in them must be re-audited: a
continuous physical response that cycles is fine, a hard threshold that gates an outcome on the now-cycling value
authors a clock and must be found and fixed before this ships.

Physics and Terran-geometry seams (verified against source or astronomy). The day-night cycle is on the SYNODIC
(star-relative) period, not the sidereal `rotation_period_seconds` alone: the solar day combines rotation and orbit,
and using rotation alone makes the tidally-locked case (rotation equals orbit, so no day-night) come out wrong. The
multiplicative decomposition (`latitude_light(y,h)` times a phase/longitude diurnal factor) equals cos(lat)cos(hour),
which is correct ONLY for zero axial tilt; real insolation is cos(zenith) = sin(lat)sin(declination) +
cos(lat)cos(declination)cos(hour), which does not factor, so the multiplicative form cannot carry seasons or the polar
midnight-sun/polar-night, and `latitude_light`'s hardwired zero at the poles makes the poles permanently dark under any
diurnal factor. The static fallback for a tidally-locked or non-rotating world reuses the row-only `latitude_light`,
which has NO longitude term and so cannot represent the fixed day-face and night-face that DEFINE tidal lock. The
normalized [0,1] insolation is fed as `absorbed_irradiance` into `radiative_equilibrium` with NO stellar-luminosity or
orbital-distance scaling, so every world radiates from the same irradiance regardless of its star and distance.
Emissivity is a per-MATERIAL surface property (rock, water, ice, vegetation, an alien crust), not the single
world-level constant the statement reserved beside sigma (sigma is a legitimate universal floor constant, emissivity is
not). The design hardcodes a SINGLE light source ("the sun's height", singular) with no data field for the number or
arrangement of sources (a binary-star world). And `t_max`, a parameter the kernel already takes, was left unclassified.

Determinism guards to specify (verified as real). Guard `ticks_per_rotation` against zero (divide-by-zero) and one (the
phase collapses to a constant), and note that `ticks_from_seconds` truncation aliases nearby rotation periods to one
discrete `ticks_per_rotation`; fix the field-step order of the light recompute, the heat coupling, and the
hydrology/productivity reads; specify the self-counter's initialization when the environ is armed mid-run (offset from
tick zero), so the phase is deterministic; and bound the counter width. The `rotation_period` zero sentinel is
physically inverted (zero is infinitely-fast rotation, not non-rotating): use an explicit absent/None for the
non-rotating case.

The scope fork for the gate and owner. The panel makes clear that a correct diurnal insolation is a physics SUBSTRATE,
not a one-line wire: the minimal honest form (rotation-only, zero-tilt, single-sun, no luminosity scaling) is the
cos(lat)cos(hour) special case, defensible as an explicit floor-limited interim IF its Terran assumptions are declared
and the tidal-lock/poles/luminosity cases are handled or flagged, not silently baked. The complete form (synodic
period, cos(zenith) with declination from per-world obliquity and orbital phase, per-material emissivity, stellar
luminosity and orbital distance, a data-defined light-source set) is the true substrate, a larger build. This is the
gate and owner's scope call. Surfaced, not decided; no code until the gate rules the scope.

## Third-form blind framing result (arc 2, the overnight priority): corrections to the sun-angle spec

The section-10 panel (five lenses, section-11 cleared) confirmed the third form fixes the earlier interim-vs-substrate
seams in principle (a closed-form cos-zenith sum-over-stars with data obliquity/eccentricity/luminosity, the old poles=0
latitude tent gone, the phase firewalled) but corrected the spec on several load-bearing points, verified against
geometry and source.

Missing the per-cell LONGITUDE term (a correctness bug). The proposed theta_s(t) took latitude, a single GLOBAL diurnal
phase, obliquity, and orbital phase, with no per-cell longitude, so every cell at one latitude would be at the same sun
angle simultaneously (the whole planet noon at once). The hour angle must carry the cell's longitude (its x-column
mapped to a longitude offset), so the sun sweeps across columns: hour_angle = 2*pi*(phase + longitude_fraction). The
full geometry is cos(zenith) = sin(lat)sin(decl) + cos(lat)cos(decl)cos(hour_angle), with declination from obliquity and
orbital phase.

Synodic, not bare sidereal. The phase = tick modulo the rotation period is the SIDEREAL day; the solar day (successive
noons) combines rotation and orbit, so the hour angle must subtract the orbital phase (hour = sidereal_phase +
longitude - orbital_phase) for the tidally-locked case (rotation = orbit, a permanent day face) to come out right. The
spec left this open; it must be specified.

Each star its own orbital geometry. The star-list is open (admits a binary or trinary system as data), but a single
shared "world orbital phase" applied to every star is wrong: each star carries its own orbital position, so a binary
system's two suns rise and set independently. L_s is attenuated by the inverse-square distance, and under eccentricity
the distance varies with orbital phase (Kepler), so distance is a function of orbital phase, not a fixed scalar.

The heat path's "no authored per-material curve" is illusory as stated. Dropping a per-material emissivity curve does
not remove the absorption: it lumps it into the reserved relaxation rate and an UNSPECIFIED flux-to-baseline conversion,
which makes absorption UNIFORM across ice, rock, water, and an alien crust (a Terran/uniform bake, and uniform thermal
lag is authored). The insolation-to-baseline conversion must be specified: either a floor radiative-equilibrium map with
a floor emissivity (a flagged uniform-absorption limit) or, better, per-material absorption/thermal-inertia as material
DATA so ice and rock and water lag differently and the alien crust is a data row. The diurnal swing and thermal lag then
EMERGE from relaxation-plus-diffusion, bounded by a MAXIMUM PRINCIPLE (not "energy conservation", which was imprecise),
conditional on the reserved rates.

Mirror does NOT reduce to today's static field, so this is NOT byte-neutral. Today's light is a static always-on
latitude map (equator 1 for every tick); the new law cycles every non-polar cell from full at noon to zero at night even
at the bare default (obliquity 0, one star), because max(0, cos theta_s(t)) is time-varying. So the drive changes the
reference world's field fundamentally (which is the POINT, the owner wants day-night heating), and a normalization scale
(the static map was [0,1]; the cycling insolation needs its own scale) is a missing reserved value. Two design choices
for the gate: on-by-default (every scenario cycles, all four determinism pins re-baseline) versus opt-in (a static
latitude fallback when the drive is unarmed, so the four pins hold and the diurnal cycle arms per scenario like living).
And Mirror (Earth's real data) has obliquity ~23.4 degrees, so it is NOT the "tilt 0" bare default: the tilt-0 reference
row is the minimal default a world that declares nothing gets; Mirror the demo world carries Earth's real tilt (real
seasons plus day-night). Whether the overnight demo runs Mirror-real-tilt or the tilt-0 clean-diurnal default is a scope
choice.

Determinism and the firewall. The cos/sin needs a deterministic fixed-point trig primitive. The phase counter should be
an environ-local unreadable counter (or the canonical tick) that NO behavioural substrate can read, since a controller
that reads the integer tick computes tick mod rotation and authors a template; the cycling FIELD is itself an entrainment
clock, but that is the intended emergent signal (beings evolve a rhythm by selection over the physical field), so the
firewall is on the RAW phase/tick, not the field, and its enforcement is partly a code-review invariant. The
now-cycling field creates a recompute-order seam for the hydrology and productivity reads that must be pinned. And the
mandatory pre-cycle audit: every authored threshold in the pre-existing hydrology/productivity consumers that was fitted
to the STATIC field becomes a de-facto on/off time-gate once the field cycles (authoring a diurnal forcing rhythm one
level down), so each must be found and shown to be a genuine physical phase boundary (freezing, evaporation onset) or
replaced with a graded response, BEFORE the field is allowed to cycle.
