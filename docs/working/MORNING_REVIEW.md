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

- (none yet)

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
