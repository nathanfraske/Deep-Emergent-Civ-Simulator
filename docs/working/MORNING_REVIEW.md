# Morning review: overnight interim decisions and deferred owner calls

The owner is away overnight; the delegation runs unattended (the cloud agent building Arc 2 and on, guided
and gated by me). Per the owner's directive, I build PAST decisions rather than stall: for a fork the owner
would normally weigh, I make a reasonable INTERIM call, state its basis and how to reverse it, tell the agent
to proceed, and log it here. Only a truly owner-only or hard-to-reverse decision waits. This doc is the single
place to review what happened overnight and what still needs your ruling. Newest entries at the top of each
section.

## Owner-only calls still waiting (need your ruling)

- **Mirror dial-set sign-off (the gate) — READY.** The agent completed the Earth-1:1 calibration: 34
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
  convective-coefficient dedup the agent is building overnight (your "dedup now" ruling).

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
