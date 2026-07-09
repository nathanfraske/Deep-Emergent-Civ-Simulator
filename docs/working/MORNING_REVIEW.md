# Morning review: overnight interim decisions and deferred owner calls

The owner is away overnight; the delegation runs unattended (the cloud agent building Arc 2 and on, guided
and gated by me). Per the owner's directive, I build PAST decisions rather than stall: for a fork the owner
would normally weigh, I make a reasonable INTERIM call, state its basis and how to reverse it, tell the agent
to proceed, and log it here. Only a truly owner-only or hard-to-reverse decision waits. This doc is the single
place to review what happened overnight and what still needs your ruling. Newest entries at the top of each
section.

## Owner-only calls still waiting (need your ruling)

- **Mirror dial-set sign-off (the gate).** Mirror is the one owner-GATED world. When the agent completes its
  Earth-1:1 calibration it presents the full dial-set (every value with basis + source) and I hold it here for
  your sign-off. I will NOT let Mirror be treated as canonical without you. Sub-item flagged by the agent: the
  orbital year is set to 31536000 s (365.0 d, Julian convention); the tropical year is ~31556952 s (365.2422
  d). The agent leans tropical for a strict 1:1 Earth. Your call at the sign-off.

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
  tissue-tolerance / denaturation substrate that is genuinely Arc 4 (that is `burn_scale`'s home). You may have
  grouped it by name; set it now on reconsideration, or keep the Arc-4 deferral. `burn_scale` stays Arc-4
  either way.

## Notes and observations from the night

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
