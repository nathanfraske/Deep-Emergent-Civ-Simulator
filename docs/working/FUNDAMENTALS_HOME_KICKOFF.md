# Fundamentals-home: kickoff (the closed constants table, composites derived into it)

This is a DOC-ONLY kickoff, the bridge PR for the calibration-reconciliation core's second piece (billboard step 2), following the census and the category field (PR #122). It scopes the seam and states the discipline; it authors no mechanism and moves no value. The predecessor #122 (the census doc plus the machine-checked category field) merges once this bridge is open (the bridge rule). Opened off current `main`.

## The seam

The locked three-way test (AGENTIC_ADDENDUM section 9) names one authored universal layer: the small, closed, non-growing list of fundamental physical constants reality measures and cannot derive (c, k_B, h, e, eps_0, N_A). Everything derivable derives from them, and everything else is per-world data. The gate-confirmed census (`docs/working/CALIBRATION_CENSUS.md`) found that `calibration/reserved.toml` today carries zero fundamentals and 26 derivable entries, several of which are composites of the fundamentals wearing an authored-decimal disguise: `metabolism.stefan_boltzmann` is the Stefan-Boltzmann sigma (2*pi^5*k_B^4/(15*h^3*c^2)), and the gas constant R (k_B*N_A) and Faraday F (e*N_A) are the same shape where they appear. There is no one authored table the fundamentals live in, so a composite sits as its own number rather than deriving from a single source, and a fundamental (k_B was a reserved calibration value in the R-SOURCE-VECTOR arc) has no canonical home.

## What the arc opens (scope, not mechanism)

The arc instantiates the ONE closed table of fundamental constants (c, k_B, h, e, eps_0, N_A) as the single authored source, and re-derives the composites INTO it: sigma, R, and F are COMPUTED from the fundamentals rather than stored as their own numbers, with a byte-neutral composite drift-check (the derivation reproduces the intended value within the fixed-point tolerance) and a de-duplication pass so a fundamental is authored once and read everywhere. Its natural home is `crates/units`, which exists today as the dimensional MECHANISM only (the base-dimension registry, the quantity registry, deterministic integer arithmetic) and by its own design ships no base dimension, no quantity, and no constant; the fundamentals table is new content over that existing mechanism, not a new crate. The mechanism stays fixed Rust (the composites derive through code); the authored layer is the closed fundamentals list alone.

## The load-bearing byte-neutrality seam (surfaced for the gate before any build)

Re-deriving a composite that a live sim consumer reads is not automatically byte-neutral. `metabolism.stefan_boltzmann` is read into the radiant heat-loss law on the sim path (`crates/sim/src/physiology.rs`, into `laws::radiant_emission`), so its exact fixed-point value folds into the four determinism pins (default `4bbf6b59`, discovery `c9d5cc17`, viability `ad69f2bf`, full `1db633b3`). The roadmap already records that a sigma-precision correction would re-pin all four hashes for roughly 0.22 percent. So the composite re-derivation has two honest outcomes, and the choice is the gate's, not the agent's: either the derivation from the fundamentals is arranged to reproduce the currently-stored sigma value EXACTLY at the fixed-point scale (byte-neutral, the pins hold), or the precise derivation is accepted and the four pins move under a gate-approved RE-PIN. This seam is surfaced here so the gate rules it before the build, rather than a re-pin being forced silently. The composites with no live consumer (R, F where they are not yet read) derive byte-neutrally by construction.

## Out of the core (separate, owner-gated later arcs)

The token/magnitude collapse (per-world magnitudes moving out of the global manifest into each world's own fixture file) and the fixed-point sub-resolution fundamentals-representation (how a constant below the Q32.32 epsilon is stored) stay flagged, not built here.

## The discipline

The composites DERIVE, never authored; only the measured fundamentals are authored (the one permitted place). The whole arc runs the mandatory section-9 five-lens audit. The byte-neutrality seam above is ruled by the gate before the build. After the fundamentals-home lands, the per-agent blind-checks (billboard step 4) run: A, B, and C each independently blind-bias-check the census and the fundamentals-home.

## Status

Kickoff only. The census plus the category field (PR #122, byte-neutral, section-9-hardened) is the predecessor and merges once this bridge is open. The next step on this branch is to frame the composite re-derivation and its byte-neutrality seam for the gate's ruling, then build the fundamentals table and the derivations under that ruling, off current `main`.
