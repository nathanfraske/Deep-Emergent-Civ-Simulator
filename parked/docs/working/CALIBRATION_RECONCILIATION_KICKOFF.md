# Calibration reconciliation: kickoff (the three-way-test census, category field, and fundamentals home)

This is a DOC-ONLY kickoff, the bridge PR for the owner-ruled first-priority arc that follows the R-SOURCE-VECTOR eater-draw lift (PR #121). It scopes the seam and states the discipline; it authors no mechanism and moves no value. The predecessor #121 merges once this bridge is open (the bridge rule). Opened off current `main`.

## The seam

The flat `calibration/reserved.toml` (224 entries) conflates the three categories of the locked value-authoring test under one schema with no category field. Every agent adding or retiring a reserved value (the R-SOURCE-VECTOR derive-sentinel, #119's stroke-rate entries, #121's retirements) works against a manifest that does not say, per entry, which of the three kinds a value is. Until each value is born categorized, that ambiguity is a standing source of cross-agent confusion and a latent value-authoring defect: a per-world outcome wearing a global constant's clothing, or a composite masquerading as an authored fundamental, is not caught by construction.

The standard is the owner's locked three-way test (`AGENTIC_ADDENDUM.md` section 9, the fundamental-constants floor): every reserved value is exactly one of (1) a fundamental universal physical constant reality cannot derive, (2) a per-world datum read from the world's own data, or (3) derivable-and-derived from the fundamentals and the situation. A value that is none of the three is a defect. The prior pass (`docs/working/CALIBRATION_RECONCILIATION.md`) audited the manifest under an earlier TWO-verdict scheme (LEGITIMATE-DATUM or PER-WORLD-IFY), which folded the derivable category into "legitimate," so it predates the locked test and is re-run here under the three categories.

## What the arc opens (scope, not mechanism)

Three pieces, each byte-neutral, both scoping critics converged on this core:

1. The three-way CENSUS. Classify all 224 `reserved.toml` entries as a fundamental constant, a per-world datum, or derivable. Each classification is SOURCE-GROUNDED and verified against the three-way test itself, never taken from the basis prose (the prose already hints, with entries saying "derive," "floor," "per-race," a handful CODATA, but a hint is a lead, not a verdict, Prime Directive 1). Analysis-only, byte-neutral: the census writes a categorization, it moves no value. The prior reconciliation is re-run under the three categories.

2. A machine-checked CATEGORY FIELD on `ReservedValue` (`crates/sim/src/calibration.rs`) plus a CI gate, so every entry is born categorized and a mislabel FAILS THE BUILD. This is the true precedent that ends the confusion: a value cannot enter the manifest uncategorized, and a category that does not hold up fails the gate. It is ADDITIVE: an absent field is tolerated as UNCLASSIFIED during migration, so it lands FIRST without breaking in-flight entries, in a window coordinated with the gate so the per-entry sweep does not collide with #119 (which adds entries) or #121 (which retires them).

3. The FUNDAMENTALS-HOME. Instantiate the single closed list of fundamental constants (c, k_B, h, e, eps_0, N_A) as ONE authored table, relocate or re-derive the scattered constants into it, add a byte-neutral composite drift-check, and de-dup. The composites that today sit as owner-set decimals must DERIVE rather than be authored: the Stefan-Boltzmann sigma, the gas constant R, and the Faraday constant F are compositions of the fundamentals, and `metabolism.stefan_boltzmann` (`reserved.toml:1568`, tagged "Arc 2 Mirror calibration") is the paradigm case, a CODATA composite standing where a derivation belongs.

Input-audit note (Prime Directive 2): the gate named `crates/units` or `crates/physics` as the home and observed it does not exist yet. Verified against source: `crates/units` DOES exist, but by its own design carries the MECHANISM only (the base-dimension registry, the quantity registry, deterministic integer arithmetic) and deliberately ships NO base dimension, NO quantity, NO scale, and NO constant. So the premise holds in substance: the fundamentals TABLE (the authored constant values) does not exist, and `crates/units` is its natural mechanism home. The table is new content over an existing mechanism, not a new crate.

## Out of this core (separate, owner-gated later arcs, not folded in)

Two heavy transforms are flagged, not built here: the token/magnitude collapse (moving per-world magnitudes out of the global manifest into each world's own fixture file) and the fixed-point sub-resolution fundamentals-representation (how a constant below the Q32.32 epsilon is stored). Both are large and gate-sequenced separately.

## The discipline (why this is a value-line arc)

The census sits squarely on the value-authoring line, so each categorization is framed with a DERIVE-FIRST self-check: a mislabel (a composite called a fundamental, a per-world outcome called derivable, an authored value that should be read as data) is a value-line defect, caught before it is trusted. The whole arc runs the mandatory section-9 five-lens audit. The census is POSTED for the gate BEFORE the category-field sweep lands, so the gate confirms the categorizations before they are machine-enforced (an enforced wrong category is a wrong reference). The category-field landing window is coordinated with the gate to avoid colliding with the in-flight #119 and #121 entry changes.

## Status

Kickoff only. The R-SOURCE-VECTOR eater-draw lift (PR #121, byte-neutral, section-9-hardened) is the predecessor and merges once this bridge is open. The next step on this branch is the three-way census, posted for the gate's confirmation before any enforcement mechanism, off current `main`.
