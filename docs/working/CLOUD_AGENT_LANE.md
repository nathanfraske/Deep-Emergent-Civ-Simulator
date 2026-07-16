# The cloud agent's lane, queue, and protocol

Two full agents work this repo in parallel, each with its own sub-agent budget, with the local coordinator as the shared gate for both. This document is the cloud agent's standing reference. The channel is the cloud agent's own PR: the coordinator monitors branch pushes and PR comments, and replies there.

## The lane split (disjoint files, no collisions)

- COORDINATOR'S LANE: the viewer and render (`crates/viewer/**`: the cube-sphere sample cache, the flexure render wiring, the three-band surface, the LoD quadtree) and the composition chain (`crates/materials/src/disk_composition.rs`, `crates/physics/src/solar_abundances.rs`: the [alpha/Fe], C/O, s/r links).
- CLOUD AGENT'S LANE: the assembly side. `crates/sim/src/planetary_assembly.rs`, `crates/sim/src/giants.rs`, `crates/sim/src/moons.rs`.

Neither lane touches the other's files. If a slice needs a file across the line, post on the PR and the coordinator sequences it rather than both editing.

## The work queue (in order)

1. THE ASSEMBLY MERGE-HISTORY EMISSION. `assemble_system` returns only the final planets and drops its merge events. Make it optionally record its merge history (per giant impact: the two bodies, their masses, the merge orbit, the epoch). One slice, two unlocks: the giant-impact moon branch, and the owner's "watch it build" construction-montage mode (task #80). Keep the existing merge math and the mass and angular-momentum conservation bit-exact.
2. #73 GIANTS INTO THE ASSEMBLY. Wire the built `giants.rs` verdict in so a final system carries giants, which unlocks circumplanetary-disk moons. DELICATE: the assembly is the north-star core. Post the design on the PR and get the coordinator's sign-off BEFORE the code lands.
3. THE MOON BRANCHES (read `docs/working/MOON_ARC_SCOPE.md`): branch B (giant-impact moons off the merge history), then branch A (circumplanetary-disk co-accretion, the Canup-Ward satellite mass ratio, needs #73), each with the tidal-survival post-condition built on the landed `moons.rs` primitives (Hill radius, Roche limit, the Domingos stability fractions, tidal recession).

## The protocol

- Branch off main. Your own branch, your own PR. The coordinator monitors your pushes and comments and replies on your PR, which you are subscribed to.
- Report each slice on your PR: what you built, BOTH pin hashes from a fresh build, the tests, the value-line accounting, and anything you stopped on.
- BUILD AHEAD. Do not wind down after a slice; take the next queue item. If an item blocks, post the blocker and keep building what is unblocked.
- Every push gets gated. A gate ends with SIGNED OFF or with findings to fix.

## Standing rules (non-negotiable)

- THE BYTE PINS: default `40fe8a7269ee4da8974eb1787338c3a0`, living `be94e3100b9db82f7c1aea1d8091956d`. Every push holds them byte-exact: `export PATH="$HOME/.cargo/bin:$PATH"; cargo build --release --example run_world -p civsim-sim; ./target/release/examples/run_world | grep 'final state_hash'`, then again with `--scenario living`. A dormant, off-run-path slice holds them trivially. If a slice would MOVE a pin, STOP and surface it: a re-pin is the owner's call.
- THE VALUE LINE: a number is legal only as a fundamental constant, a per-world contingent datum, a derived quantity, or a cited unit anchor. Anything else ships reserved-with-basis WITH a citation, or you STOP and surface it. Never invent a number.
- THE PREMISE LINE: before you WIRE, CONNECT, or ROUTE anything, verify IN THE CODE that the upstream exists. Designed-exists does not imply built-exists (a ruling was corrected on exactly this). If it is not built, STOP and surface it rather than author it into being.
- GUARDS HOLD, NEVER REROLL: a draw landing in unimplemented territory is held and flagged, never resampled. A guard that rerolls is an invisible author that deletes the tail.
- SILENT PARAMETERS ARE AUTHORED ONES. If your implementation implies a quantity the source did not supply (a dispersion, a weight, a characteristic scale, a collapsed vector), name it in a DEFAULTS TAKEN list and tag it loud interim. This is the standing card discipline; the review reads that list first.
- stone0 provenance gate: classify any new `::from_bits(` or `::from_decimal_str(` in `scripts/constructor_baseline.tsv` with an honest reason. Never use the override password.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` clean. Update `docs/working/CONSENSUS_ROADMAP.md` IN PLACE (a living tracker, not an append log).
- PROSE CUSTOMS: no em dashes anywhere; never the adverbs "genuinely", "honestly", or "actually"; avoid the "not just X but Y" construction.

## References

- `docs/working/CONSOLIDATED_SURFACE_PIPELINE.md`: the consolidated Stage 0-5 pipeline and the standing rules. Build against it.
- `docs/working/MOON_ARC_SCOPE.md`: your queue items 2 and 3.
- `docs/working/PIPELINE_FETCHES.md`: the cited literature constants (Canup-Ward, the Domingos Hill fractions, tidal recession, and more), with verify-on-pull discipline: a fetched value is a target to VERIFY against its citation when you load it, never a digit to trust from the doc.
- `docs/working/GRANULAR_ARC_SCOPE.md`, `docs/working/FLEXURE_ARC_SCOPE.md`, `docs/working/RENDER_STRATEGY_RESEARCH.md`: adjacent arcs, for context on where your lane meets the coordinator's.
