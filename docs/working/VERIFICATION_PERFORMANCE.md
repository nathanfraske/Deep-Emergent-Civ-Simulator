# Verification performance profile

This note records the measured PR #215 verification throughput pass. It covers
developer and CI scheduling only. No profiler, cache, scheduler, timing result,
or performance receipt has physical, admission, merge, or scientific
authority.

## Result

On the current WSL development host, the complete PR-quality route fell from
651.68 seconds to 227.83 seconds. The all-target test gate fell from 566.88
seconds to 182.87 seconds in the final complete route and to 176.85 seconds in
an isolated run. The independent warm Cargo-native serial replay completes in
535.19 seconds, so the final scheduler is 2.93-fold faster than its direct
serial cross-check.

| Stage | Baseline wall (s) | Final wall (s) | Baseline average cores | Final average cores |
|---|---:|---:|---:|---:|
| Format | 1.77 | 2.01 | 0.96 | 0.90 |
| Gate runner canaries | 0.25 | 0.25 | 0.61 | 0.64 |
| Gate runner regression tests | 3.27 | 3.78 | 0.40 | 0.43 |
| Target collector tests | 1.76 | 2.01 | 0.96 | 0.92 |
| Structural pre-phase | 0.25 | 0.25 | 0.30 | 0.30 |
| Detector self-tests | 10.81 | 6.80 | 0.98 | 1.70 |
| Stone 0 self-test | 0.50 | 0.50 | 0.88 | 0.99 |
| Stone 0 CI route | 5.53 | 5.54 | 1.64 | 1.92 |
| Structural post-phase | 8.32 | 8.30 | 1.44 | 1.62 |
| Clippy | 37.34 | 9.82 | 6.12 | 1.77 |
| Complete all-target tests | 566.88 | 182.87 | 4.83 | 14.73 |
| Private-item Rustdoc | 11.60 | 3.27 | 2.51 | 1.78 |
| Doctests | 3.27 | 2.26 | 1.13 | 0.90 |
| **Complete route** | **651.68** | **227.83** |  |  |

The complete-route reduction is 2.86-fold. The test-gate reduction is
3.10-fold in the complete receipts and 3.21-fold against the isolated parallel
receipt.

On GitHub's hosted runner, the canonical planet job fell from 43 minutes 10
seconds to 24 minutes 5 seconds, a 1.79-fold reduction. The former automatic
parked compatibility job then became the 31-minute workflow critical path.
That job supplied no canonical readiness evidence, so automatic PR and
scheduled CI now contain only Stone 0 and the canonical planet route. The
complete parked compatibility suite remains available through a
`workflow_dispatch`-only maintenance workflow.

The baseline began with a colder target and grew it by 3.76 GB. The final route
grew it by 194 MB, so compiler state contributes to the full-route comparison.
Cargo reported 41.68 seconds of compilation inside the baseline test gate.
Removing that compile interval leaves about 525.20 seconds of serialized test
execution. The direct warm serial receipt measures 535.19 seconds at 4.42
average cores and the final parallel receipt measures 182.87 seconds at 14.73
average cores. The independent comparison is therefore a 2.93-fold wall-time
reduction. The isolated 176.85-second parallel receipt is 3.03-fold faster than
the warm serial receipt.

## Scheduling model

Cargo compiles the selected package and all-target inventory once with JSON
artifact reporting. The scheduler then:

1. reads one Cargo metadata inventory;
2. independently matches every expected package target to one compiled test
   executable;
3. validates each executable as a regular runnable file below the managed
   target and seals its SHA-256;
4. lists every complete libtest binary to prove harness execution and test
   count;
5. schedules complete binaries across a bounded worker pool;
6. serializes GPU-package binaries behind one hardware lock;
7. re-hashes every executable after execution;
8. refuses on an inventory mismatch, launch failure, mutation, failure, or
   omitted result.

Inventory work is linear in packages, targets, and Cargo artifact records.
Executable sealing is linear in executable bytes. Test work is unchanged, but
wall time moves from the sum of independent binary durations toward the larger
of total CPU work divided by available cores and the longest complete binary.
No individual test is split, so package-local `OnceLock` fixtures and libtest
semantics remain intact.

The default is at most eight complete-binary workers and respects the process
CPU affinity. `CIVSIM_TEST_JOBS` or `--jobs` can set a host-specific bound.

## Resource profile

The final test gate used 2,692.89 child CPU-seconds over 182.87 wall-seconds,
or 14.73 of the 18 available logical cores on average. Per-core host busy time
ranged from 72.95 percent to 96.92 percent, with an 82.44 percent mean. The
isolated parallel receipt averaged 15.60 cores.

Sampled test pressure reached 24 processes, 143 threads, and 1.13 GiB
process-tree RSS. Host pressure-stall accounting recorded 65.05 seconds of CPU
`some` pressure across the interval, while memory pressure was effectively zero
and I/O full pressure was 0.0015 seconds. This identifies CPU scheduling as the
remaining resource boundary rather than storage or memory.

The guest reports 18 private 48 KiB L1 data caches, 18 private 64 KiB L1
instruction caches, 18 guest-visible 3 MiB L2 caches, and one shared 30 MiB L3
cache. The WSL kernel exposes no CPU PMU event source, so direct cycles,
instructions, cache-reference, and cache-miss counters are unavailable inside
this guest. The profiler records that absence instead of inventing cache-miss
data. It still records guest cache topology, page faults, context switches,
process-tree RSS, filesystem block I/O, page-cache deltas, pressure stalls,
per-core load, and `sccache` counters.

The Windows host does enumerate `CacheMisses`, `LLCReference`, `LLCMisses`,
retired-instruction, and cycle PMU sources through WPR. Starting the required
kernel trace from this non-elevated session is denied, so the profiler records
the available source names but does not claim counter samples. A future
elevated, bounded host trace can add that diagnostic without changing the Linux
quality route.

## Commands

```bash
just profile-pr
just profile-fast
just profile-test-serial
```

Receipts and per-stage logs default to the bounded machine-local repository
cache. They do not enter the tracked tree. Owned test runs and profiles retain
at most 32 recent entries after a one-hour concurrency grace and expire after
30 days. Pruning accepts only exact marker-bearing directories below the
resolved managed root and never follows links. A path can be selected with
`--output`.

The routine `just test` uses the complete-binary scheduler. `just test-serial`
preserves Cargo's independent serial executor. `just check-nightly` runs the
ordinary quality route and then the serial cross-check, so the checker remains
sparse and independent without charging every development cycle.

The canonical package-boundary and archive-structure gates remain in every PR
route. They prove the nested parked workspace cannot enter the root dependency
graph and preserve the maintained historical document structure without
compiling biology, civilization, or the retired causal viewer.

The independent serial replay passed the same all-target selection after the
parallel route. It used 2,364.15 child CPU-seconds over 535.19 wall-seconds.
The final parallel run used 2,692.89 child CPU-seconds, a 13.9 percent CPU-work
premium from contention and scheduling, while removing 352.32 seconds of wall
time. The extra CPU work is visible and bounded rather than hidden.

## Remaining opportunities

The test route is close to its CPU-work lower bound. Further scheduling changes
have a small ceiling and need measured evidence before adoption. The next
useful automatic-CI optimization targets are inside the three longest complete
binaries: planet, physics, and the canonical CLI integration suite. Any fixture
or algorithm change must preserve complete coverage and exact receipt behavior.

Structural post gates, Rustdoc, and Clippy have spare core capacity, but their
combined wall share is now small. Overlapping authority phases or compiler
commands would add coordination risk for little return, so this pass leaves
their ordering explicit.
