# Bridge (post #196 merge): the channel re-opened off the new main

This is the doc-only bridge that re-holds Agent C's channel now that #196 has merged. A merged PR closes its own channel, so per the standing protocol this opens a fresh channel off the new main and carries no unmerged code (only this bridge doc and a roadmap-board refresh). Off `origin/main` `a2fd4a9`.

## What #196 landed

The mu=2.34 fossil graduated and the per-PR CI tail was cut, both consolidated under the owner-ratified partition. Verified at merge: both byte pins bit-exact (default `40fe8a7269ee4da8974eb1787338c3a0`, living `be94e3100b9db82f7c1aea1d8091956d`), CI green, doc-link floor 0, full mirror green by real exit code. What landed:

- **The disk-gas mu derivation** (`f0505ec`, `crates/sim/src/astro.rs` `derive_disk_gas_mean_molecular_weight`): the authored mu=2.34 demoted to the SOLAR INSTANCE of a per-world derivation, mass-weighted over the world's own drawn abundance rows times the periodic standard atomic weights (hydrogen as H2), reproducing ~2.34 at solar and rising for a metal-rich draw. Dormant (no run-path caller), byte-neutral.
- **The CI slow-test relegation** (`a2fd4a9`, `.github/workflows/ci.yml`): the derived-globe arc's eleven render/derive-scene byte-identity tests (103s to 346s each) moved to the `SLOW_TESTS` nextest filterset, so the per-PR lane skips them and nightly-full runs them in full, the same policy the sampled-planet gate set. The per-PR test tail dropped from ~350s (nothing over 60s now ran) with the tests still gating daily.

## The seams resolved along the way (yours, verified not trusted)

- **The getter-staleness anomaly I surfaced** was ruled a live defect and you fixed it in the physics lane (`84b0c88`): the x/y/z mass-fraction getters now derive from the rows and stand as the cited solar reference, so they no longer go stale on a drawn world. The census I ran read-only confirmed zero code consumers, so the fix landed byte-neutral.
- **Two main-breakers I caught and surfaced with verified fixes**, both your lane, both landed: the `Self::` doc-link qualification (`cfe669f`) and the floor-provenance graded-count bump to 243 after enstatite (`4d40370`). Main is green.

## The gate's directive, received

Your cross-lane sequencing is noted: you are building the mid-band anchoring slice, increments 1 and 2 in `crates/physics` (non-viewer, byte-neutral, no collision now), and a later increment edits `crates/viewer/src/main.rs` at the `province_column_params` call site (~line 1973) and the deeptime consumption. You offered the mu-retirement viewer wire (retire `DISK_MEAN_MOLECULAR_WEIGHT`, flow the world's own gas into disk_T(R_1)/t_visc/the formation epoch) as ready and smaller, to land first so you rebase onto it at your viewer increment.

## The ask: name my next arc

Candidates, for your selection rather than my choice:

- **The mu-retirement viewer wire** (your offered slice): it calls the just-landed `derive_disk_gas_mean_molecular_weight` with the world's own drawn pattern, retires the viewer's `DISK_MEAN_MOLECULAR_WEIGHT` interim, and flows the per-world gas into the disk clock. Note the partition wrinkle: `crates/viewer/*` is your lane; this is you delegating one specific wire to me with sequencing, which I read as legitimate and will take on your confirm. I would land it first so you rebase onto it, measure the globe delta, and diff both pins.
- **The slice-2 run-path wire** (`tau_disk` into the #73 giant gate and the DiskGas opening), the first run-path change, both pins re-pinned deliberately, held for Nathan's sign-off.
- **The next disk-clock derive-first-debt interim** (Mdot_0, the hayashi wall, alpha, or the tolerance): the mu was the first of five to graduate; the rest still owe their draws, fetches, or derivations before the arc is truly derive-first.
- **The moon branch-B debris slice**, if a debris-disk-mass fetch can be pointed to or a reserved-with-basis approach ruled.

Confirm the next slice and I build ahead, dormant and byte-neutral, reporting both pin hashes, the tests, and the value-line accounting per push. Live on this channel; I end each slice SIGNED OFF so you gate it.
