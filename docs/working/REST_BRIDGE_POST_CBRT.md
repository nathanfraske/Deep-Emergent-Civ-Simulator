# Rest bridge: the surface-transport lane is complete, holding for the next critical slice

The gate approved the exact `Fixed::cbrt` (2026-07-12, #181) and offered a choice on the merge: keep #181 as my channel, or open a fresh rest-bridge so the cbrt lands on `main` now. I take the second, because the cbrt is a finished, approved, byte-neutral primitive and banking it on `main` keeps it from going stale as the tree advances, while this bridge gives me a clean channel that carries no unmerged code. This is a doc-only bridge off current `main` (`64da409`, after the momentum-unification merge #179); it authors no mechanism and moves no value.

## What is done, and where the lane rests

The surface-transport toolset is complete end to end: the non-local redistribution operator and its four-reservoir ledger coupling (#172, #174), the runout and ballistic integrators (#172, #177), the one unified velocity-vector law that reduces both to regimes of a single emergent force and recovers the angular resolution in both (#179, on `main`), and now the exact-root sibling to the square root, `Fixed::cbrt` (#181, bit-exact where `powf(1/3)` approximates, completing the exact-root family the arming's exponent field is scoped around). None of these authors a value; every one is fixed Rust reading the floor or a pure numerics primitive.

Two threads are held, each for a clear reason rather than for want of work:

- The Decision-2 retire (folding the runout and ballistic integrators out of the driver path into limits of the unified law, keeping the closed-form parabola as the test oracle) is held for Agent A's surface-transport arming, since it couples to A's `TransportKernelId` becoming the one unified transport-integrator arm, and A is on the priority-0 register work now. The gate reaches me here the moment A resumes it.
- The unified law's own deferred tail (airborne body forces through the open slot, free 2-D contact navigation, post-landing restitution) is documented in `momentum.rs`, available when the gate wants any of it, but none is a disjoint new arc to open unprompted on a completed arc.

## What I do not take, and why

Task #45 (the GPU-canon fractional-power residual) stays declined for me in this environment: the CPU and CUDA `powf` are both built, and what remains is the cross-vendor Stage 0 device gate on non-CUDA hardware, which this sandbox has none of and cannot even build the GPU crate for. Signing off a cross-vendor bit-identity claim I cannot run would violate prove-it-before-you-trust-it. It is hardware-lane work for a session with the GPUs, cleanly separate from the CPU-side exact cube root just built.

## The ask

Gate: merge the exact `Fixed::cbrt` (#181) onto `main`; this bridge is up so doing so does not strand me. I rest on this channel, and I take the next critical slice the moment you reach me here: the Decision-2 retire when A resumes the surface-transport arming, or a determinism-and-numerics primitive the materials or register lanes surface, or another disjoint fit I can prove on the CPU. Nothing here authors a value; the lane's mechanisms are settled and the one primitive banked is exact by construction.
