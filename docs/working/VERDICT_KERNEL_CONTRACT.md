# The Verdict kernel contract: reifying the meta-laws as types (owner directive, twenty-eighth audit)

Owner-authored (2026-07-13). The precise-scope reification of the proposer -> disposer -> freezer topology and the Gap/Residual meta-laws. SCOPE: the KERNEL owns the CONTRACT and the DISCIPLINE; the physics content stays in PLUGIN MODULES implementing traits. This is the seam where the meta-laws live, and reification converts them from convention-enforced to TYPE-enforced. Provenance inline per the seven-tag register.

## The core object: the Verdict type (the Gap Law made unrepresentable-to-violate)
Every selection-like call returns `Verdict { winner, runner_up, delta, resolution_s, band, provenance_key, tie_slot }`, with `delta` carried alongside the deciding model's own `resolution_s` so consumers read `delta/s`.

**The typestate move that pays for the whole refactor:** when `delta < s`, the type HAS NO `winner` FIELD to read. The only variants are `Escalate` (up the provenance ladder) and `SeededDraw` (the named contingency slot). The resolution-ladder rule (from the silica seam) stops being a rule anyone remembers and becomes a STATE YOU CANNOT CONSTRUCT. Same for the preflight: `fn preflight(x, E) -> Validity` runs the representation theorems (the EA2 boolean, the Cauchy class, the U/W window, Bohr-van-Leeuwen-class checks) BEFORE `propose` is ever called, so reference-validity failures are compile-path, not review-path.

## The correct factoring: two functions and a fold, NOT a three-stage monolith
- PROPOSER and DISPOSER are PURE functions of `(x, E, seed)`, memoized on quantized keys.
- The FREEZER is NOT stage three: it consumes the path `h`, so it is a FUNCTIONAL OVER TRAJECTORIES, `quench(equilibrium_fn, path) -> RealizedState`, owned by the TIME-MARCHING layer as a FOLD. This matches the physics exactly: equilibrium is a STATE function, realization is a PATH functional. Forcing the freezer inline would smuggle history into what must stay a pure oracle, violating the solver law's memory clause.

## One contract, two instantiations, and a guard against pattern-hammering
- The THERMOCHEMICAL kernel: stoichiometric/MO proposer, free-energy disposer with the provenance ladder as chain-of-responsibility, Dodson quench.
- The ATTRACTOR kernel: candidates from root-find/continuation, disposal by basin measure, persistence via hysteresis slots.
They SHARE the `Verdict` type, the seeded-draw machinery, the memo, and the ledger hooks, but NOT the energy plumbing. Spin-orbit capture, tectonic regime, Venus's basins, and electronic branch competition all land on the SECOND instantiation naturally.
What does NOT implement the contract: transport, radiation, field solvers, anything that is a FLUX BALANCE rather than a SELECTION. Allow a `Trivial` verdict constructor for truly single-candidate queries (so the discipline stays cheap where physics is unambiguous), but LOG it, so ceremony-avoidance is itself auditable.

## Two engineering laws (descendants of banked landmines)
1. **Candidate canonicalization:** seeded draws must key on a CONTENT HASH of the candidate, NEVER on enumeration order, or refactoring a proposer silently reshuffles every tie-broken world in the shipped universe (the chaos reviewer's byte-neutrality landmine one level down). Sort candidate sets canonically before dispose; iterate memos in ordered form; MONOMORPHIZE the hot paths (disposer calls sit in inner loops under Q32.32 determinism).
2. **Delta is only as honest as the proposer's COVERAGE** (the soft underbelly the type system cannot fix): a gap computed against an INCOMPLETE candidate set is false confidence, and the CO lesson proves it (the valence-arithmetic proposer never nominated the winner). Mitigations, the existing institutions pointed INWARD: seeded structure-search spot checks that the winner is not beaten by anything outside the proposed set; the Residual Law's disequilibrium clause (an assemblage sitting oddly far from equilibrium with no cited barrier suggests a missing candidate); periodic compute-once arbitration sampling. COVERAGE IS AUDITED, NOT PROVEN.

## The free payoffs (things this conversation kept legislating by hand)
Counts-are-queries becomes literal: "how many authored draws did this world consume" is a FILTER over the Verdict log (the layer-4 discipline becomes a grep). The Gap Law flag map (the civilization layer's prospecting map) is a QUERY over near-degenerate Verdicts. The validation battery runs against ONE interface. Status fields render per trait implementation. Provenance-keyed covariance rides the `provenance_key` every Verdict already carries.

## Prior art (this is boring engineering, not invention)
Generate-and-test with branch-and-bound discipline; strategy plus chain-of-responsibility for the ladder; typestate for the resolution rule; MELTS and GULP as the thermo oracle's ancestors; Keller continuation already banked for the attractor side.

## Ledger delta (twenty-eighth audit)
Engine spec gains the Verdict contract, the two-instantiation scope, the canonicalization law, and the coverage-audit hooks; NO physics entries move; layer 4 gains nothing (and that sentence is now a database query). Verify Pickard & Needs 2011 (search backstop), Gale 1997 and Ghiorso & Sack 1995 (oracle prior art), and the Rust typestate literature (sub-resolution encoding).

## Build implication (manager note)
This is the SPINE of the materials substrate buildout, and it LEADS it: the materials buildout (A primary, after Phase 2) starts with the Verdict CONTRACT + the preflight + the two-functions-and-a-fold factoring in the kernel, THEN the thermochemical instantiation (proposer/disposer/freezer) and the attractor instantiation, THEN the property-emission stages consume Verdicts. The owner's ruling: "keep building as planned unless this changes the picture materially" -- it refines the buildout's first slices (the Verdict contract leads) but does NOT change A's current Phase-2 work, which continues.
