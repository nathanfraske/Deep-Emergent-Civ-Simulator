# Derived-output-is-live gate: slices 3+4 (coverage signal, remaining probes, CI harness)

Agent B, picking up C's stranded task #43. Off main `84c4446`. Doc-first opener; the build is
tooling and tests only, byte-neutral on all four pins (the gate authors nothing on the run path,
so no sim state moves).

## Where #43 stands

Slice 1 (the `@derives[id]:` cross-check, `crates/sim/tests/derived_output_live.rs`) and slice 2
(the site-local liveness probe mechanism plus the flagship `carbon_fixation_rate` probe, in
`crates/sim/src/derive_gate.rs`) are built and in main. The mechanism is settled: for a wired
derivation the probe reads the derived output twice at a fixed representative situation, once from
the base manifest and once with the derivation's declared reserved input perturbed, and `assess`
reads Live when the two differ and Dead when they hold identical (the soil_baseline bug). Nine
retired-floor derivations are registered; one (`carbon_fixation_rate`) is wired.

Slices 3+4 are the remaining eight probes, a CI harness that runs them and fails on any Dead, and
the softer coverage signal the gate ruled belongs beside the pass/fail gate (a live-but-byte-neutral
derivation whose effect lands in a downstream deadband is a coverage gap, never a liveness failure).

## Two input-audit catches, surfaced before building

Grounding the eight rows against source turned up two seams in the slice-2 registry data, both
Prime-Directive-2 catches (audit the input, not the output):

1. **`hydrology_water` declares a retired input.** The row's perturbable input is
   `hydrology.saturation_t_ref` (`crates/sim/src/derive_gate.rs` CANONICAL). That key was retired in
   the Arc-2 derive-vs-author pass: it derives from the temperature offset now and exists in no
   profile (`dev-fixtures.toml`, `mirror.toml`, and `reserved.toml` all mark it RETIRED). A probe
   that perturbs it would fail-loud on `require_fixed`, never reaching a reading. The row needs its
   input repointed to a live reserved key the hydrology derivation still reads (candidates:
   `hydrology.saturation_slope`, `hydrology.precipitation_rate`, `hydrology.evaporation_still`),
   chosen for the one whose response is least ambiguous at a representative cell. This is a data-fix
   to a registry row, surfaced rather than made unilaterally.

2. **`world_time_cadence` is a passthrough at its site.** Its site (`crates/world/src/celestial.rs`)
   reads `world.orbital_period_seconds` straight into `OrbitalElements.orbital_period_seconds`, an
   identity. A probe there is structurally guaranteed Live (an identity read can never go constant),
   so it proves nothing: the meaningful liveness is downstream, in the tick and calendar cadence the
   period feeds. That downstream cadence is exactly `clock_calendar_cell` (`ticks_from_seconds` over
   the same period), which IS a non-trivial floored division. So the registry carries the same input
   at two rows, one trivial (the raw read) and one substantive (the division). The coverage signal
   below names this Trivial rather than trusting its pass; the gate rules whether a passthrough
   derivation belongs in the registry or should point its probe at a downstream cadence.

## Slice 3: the coverage signal

Beside the Live/Dead pass/fail verdict, a per-derivation coverage classification, so a green gate is
not read as proof where the probe is weak or absent. Five outcomes, data-driven off the registry:

- **Live**: the site-local output responded to its declared input (the pass).
- **Dead**: the output held constant under the perturbation (the fail, the soil_baseline bug).
- **Unwired**: registered, cross-checked, but no probe closure yet (an honest gap, a `None`, never a
  false Dead).
- **Trivial**: the site-local output is an identity read of the perturbed input, so Live is
  structurally guaranteed and proves no derivation is alive (the `world_time_cadence` case).
- **DeadbandOnPins**: the probe reads Live at its site, but the derivation's effect lands in a
  downstream deadband on the four canonical pins, so a scenario-level hash sweep would show it
  byte-neutral. The gate ruled this a coverage gap reported softly, never a liveness failure, since
  the site-local read already proves the output is alive.

The signal is a report, not a hard gate: only Dead fails CI. Trivial, Unwired, and DeadbandOnPins are
surfaced for the gate to close over time (wire the probe, repoint a passthrough, add a scenario that
exercises the deadband).

## Slice 4: the remaining probes and the CI harness

Wire the tractable probes now, each a kernel call reachable from the sim crate (downstream of core,
physics, world), following the `probe_carbon_fixation_rate` pattern (a representative situation, a
base reading, a perturbed reading):

- `clock_calendar_cell`: `ticks_from_seconds(orbital_period, base_tick)`; perturb the period, the
  year-in-ticks moves. Non-trivial floored division.
- `metabolic_rate`: `basal_metabolic_rate(mass, kleiber_a, cap)`; perturb `metabolism.kleiber_coefficient`, the rate moves.
- `productivity_capacity` and `decomposition_recovery`: the matter-cycle kernels over a representative cell.
- `locomotion_speed_cell`: the grown-limb speed over `locomotion.base_speed`; needs a representative body, tractable but bulky.
- `weathering_soil_nutrient`: the mineral-weathering supply over a representative cell.

Held for the gate's ruling: `hydrology_water` (blocked on the input repoint, catch 1) and
`world_time_cadence` (the passthrough, catch 2). Both are surfaced as Unwired/Trivial in the coverage
signal until ruled, never wired against a stale or vacuous input.

The CI harness is a test that walks the registry, runs each wired probe, asserts Live (fails on Dead),
and prints the coverage classification for every row so the Unwired/Trivial/DeadbandOnPins gaps stay
visible. It authors nothing on the run path; it reads run state and asserts, the same discipline as
the constructor gate.

## Byte-neutrality

The whole change is tooling (`derive_gate.rs` probes and the coverage classifier) plus tests plus the
CI harness. None of it sits on the canonical run path (the gate reads run state and asserts, it never
feeds the sim), so the four pins hold byte-identical. Stated, not assumed; the CI run confirms.
