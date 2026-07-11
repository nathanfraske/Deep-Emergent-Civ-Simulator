# Sigma fine-scale consumption: kickoff (and a PD2 entanglement finding for the gate)

This is a DOC-ONLY bridge kickoff, opened off current `main` to keep the pipeline up while the gate merges the units R-UNITS-PIN composite-compute arc (#127). It scopes the follow-on the gate offered and, before any code, surfaces an input-audit (Prime Directive 2) that bears on whether this is the right next arc or should wait behind the queued advisor design. It authors no mechanism.

## The offered follow-on

At the #127 sign-off the gate offered, as the natural next arc, the sigma FINE-SCALE consumption: realize the arbitrary-precision sigma the composite compute now derives, rather than truncating it to the raw Q32.32 `Fixed` (~3 significant figures) at consumption. Today `MetabolicAnchors.sigma` and `SkyModel.sigma` are Q32.32 `Fixed` (244 x 2^-32), so the radiant law reads sigma at ~3 significant figures and the fine precision the bignum evaluator produces is thrown away. Realizing it is a larger pin move (a real value change, sanctioned and re-pinned with the gate).

## The PD2 finding: this is not cleanly separable from the queued general representation

The gate also noted that the general R-UNITS-PIN representation (the full per-quantity-scale number system) is the bigger target the owner has queued, pending a dedicated advisor deep-dive design, and said not to start it blind, with the composite-compute named as one proven consumer of it. Auditing the offered follow-on against source, it sits ON that boundary rather than cleanly inside the narrower scope:

- The consumer is `civsim_physics::laws::radiant_emission(emissivity, area, t_hot, t_cold, sigma, flux_max)`, which takes `sigma: Fixed` and computes the Stefan-Boltzmann `sigma * (T_hot^4 - T_cold^4)` term in Q32.32 `Fixed` throughout. Its precision is bounded by Q32.32 at every step, not by sigma's incoming precision.
- So passing a more-precise sigma INTO a Q32.32 law realizes almost nothing: the T^4 and the product are themselves Q32.32-limited, so the result stays ~3 significant figures regardless of how precise sigma is going in.
- To realize sigma's precision at all, the radiant law's arithmetic (and every other sigma-consuming law) must move to a finer per-quantity scale, holding T, T^4, and the product at scales that preserve significance. That IS the per-quantity-scale number system, which is the queued advisor-designed target, not a narrow sigma-only rework.

In short: there is no narrow, high-value sigma fine-scale consumption that does not either (a) require the general per-quantity representation the advisor is to design, or (b) rework one law's arithmetic ahead of that design in a way that would likely be redone once the general system lands. The composite compute is a proven consumer that should INFORM the advisor's design, not race ahead of it on a single law.

## Recommendation, for the gate's ruling

Two clean paths, the gate to choose:

1. DEFER sigma fine-scale consumption behind the advisor's general per-quantity-representation design (the queued target), carrying the composite compute forward as the proven consumer that informs it. Take a LIGHTER interim bridge now. Candidate interims, each self-contained and not blind: the owed step-3 per-agent blind-bias-check (my Agent-C independent section-11-style check of the census #122 and the fundamentals-home #125, the deliverable named at the fundamentals-home sign-off); or a housekeeping sweep of the maintained docs for stale references and prose-custom drift left by the recent arcs.

2. Or, if the gate wants a NARROW interim sigma-precision step now (accepting it may be partly redone at the general-representation design), scope it explicitly to what realizes value without the full number system, and frame-blind that scope first (the fixed-point representation of a high-dynamic-range radiant computation is a substrate choice with real seams: where the scale lives, how T^4 stays in range, the pin move).

The recommendation is path 1 (defer, take a lighter bridge), because the finding is that the offered follow-on is not separable from the queued advisor work, and starting it narrow risks exactly the blind-start the gate warned against. Surfaced for the gate rather than chosen unilaterally.

## Status

Kickoff only, doc-only, the bridge for the merge of #127. No mechanism authored, no value moved. The next step is the gate's ruling on path 1 versus path 2; on path 1, I open the lighter interim (frame-blind where it applies) and this bridge carries that scope; on path 2, I frame-blind the narrow sigma-precision scope before any code. Off current `main`.
