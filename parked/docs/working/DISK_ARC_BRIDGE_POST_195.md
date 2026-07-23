# Bridge (post #195 merge): the channel re-opened off the new main

This is the doc-only bridge that re-holds Agent C's channel now that #195 (the disk-evolution arc) has merged. A merged PR closes its own channel, so per the standing protocol this opens a fresh channel off the new main and carries no unmerged code. Off `origin/main` `53d296f`.

## Where things stand

The disk-evolution arc merged to main (`53d296f`), consolidated under the owner-ratified partition. Verified at merge: both byte pins bit-exact (default `40fe8a7269ee4da8974eb1787338c3a0`, living `be94e3100b9db82f7c1aea1d8091956d`), CI green, doc-links 0, no main content dropped in the astro.rs reconcile against the celestial edits. What landed, all dormant and byte-neutral:

- The composed disk clock (`disk_era_xray_disk_lifetime_myr`): `tau_disk` derived end to end.
- The Kraft-break band dispatch (`KraftBreakBand`, `kraft_band_dispatch`): three zones with the near-degenerate band carried, not asserted.
- The pre-main-sequence `L_bol` override door (`formation_midplane_temperature`'s `Option` parameter) and the delegation refactor (`stellar_flux_from_luminosity_lsun`).
- The provenance-gated consistency check (`formation_rate_consistency`, `InterimBasis`, `ProvenancedInterim`): the interim-fitting circularity made unconstructible in code.
- The closed-form constant-opacity midplane (`formation_midplane_temperature_constant_opacity`): the epoch-root perf fix.

## Held work, on the record so it is not dropped

- **3b-ii, the render-visible pre-main-sequence `L_bol` flip.** Built and proven this session, then surfaced to the render owner rather than committed, per the partition (`crates/viewer/*` is his). The measured data stands: `t_formation = 0.291 Myr` (the derived root), `consistency = Consistent` against the retired 0.19 rate (the arc thesis, reproduced without being fit to it), `L_bol = 3.80 L_sun`, render-neutral through the JANAF grid at the terrestrial-zone orbits, both canonical pins untouched. The `main.rs` wire patch is ready to hand over as data. It waits on Nathan's ratification of the epoch derivation.
- **The slice-2 wire.** `tau_disk` into the #73 giant gate and the DiskGas opening, the first run-path change, both pins re-pinned deliberately. Held for Nathan's sign-off.
- **The moon lane (queue item 3).** The shared tidal-survival filter is built (PR #193, `moons::tidal_survival`, byte-neutral). Branch B (giant-impact debris moons) surfaced a blocker: the impact-debris-disk-to-moon-mass relation is not in the fetch record, so its debris slice waits on a fetch rather than an authored scaling.

## The ask

Name the next slice for my lane (the partition: `crates/sim/astro.rs`, `crates/sim/planetary_assembly.rs`). Candidates, for your selection rather than my choice:

- The slice-2 wire, when Nathan ratifies the epoch derivation (the largest disk-arc piece remaining, the first run-path change).
- The moon branch-B debris slice, if a debris-disk-mass fetch can be pointed to or a reserved-with-basis approach ruled.
- The abiotic-field-registry arc this branch is named for, if that is the intended next arc; I have no scope for it yet and would design-first before any code.

Confirm the next slice and I build ahead, dormant and byte-neutral, reporting both pin hashes, the tests, and the value-line accounting per push. Live on this channel.
