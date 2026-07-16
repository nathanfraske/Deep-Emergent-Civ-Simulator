# The geotherm arc: deriving T_e so the mid-scale relief band can exist

Authorized by the owner 2026-07-16. This document is the arc's specification, transcribed from the ruling. It is the bedrock the flexural band waits on: `T_e` is the sole unsupplied input to the flexural rigidity `D`, and until it derives, the surface is broad province swells plus tiny crater specks with nothing in between.

## The defect chain, recorded

The claim "the derived elastic lid thickness out of the thermal state" originated in the owner's coarseness ruling, propagated into `FLEXURE_ARC_SCOPE.md` and `CONSOLIDATED_SURFACE_PIPELINE.md` through the coordinator's transcription, and is carried verbatim at `crates/physics/src/flexure.rs:312`. The codebase never had a derived elastic lid. `ColumnState` is `{temperature, convecting}`, one lumped scalar per column; nothing carries temperature on a depth axis. The render agent held rather than ship either authoring shortcut, and surfaced the cross-lane false comment rather than quietly fix it. Both were correct.

THE RULE WIDENED (effective 2026-07-16, standing): the premise line triggered only on action verbs (wire, connect, route). This claim's verb was "derives from", so it passed. Any ruling premise of the form "X is already derived, carried, or owned by the engine" is an IMPLEMENTATION-STATUS CLAIM and carries the verify-or-condition line, identical to the wire verbs. Designed-exists does not imply built-exists, whatever the verb.

## The skeleton (owner-ruled, admit-the-alien checked)

**No new state axis.** `ColumnState` stays `{temperature, convecting}`. `T(z)` is a DERIVED EVALUATOR, never stored data. This is the arc's pleasant surprise: the geotherm is a function, not a field.

The three quantities that bound the profile already exist. The surface temperature comes from insolation. The lumped column scalar is the convecting interior's POTENTIAL TEMPERATURE. The lid profile between them is the ANALYTIC CONDUCTION SOLUTION, its thickness set by the flux the Ra machinery already computes. Two forms, dispatched on the lid's own nature: the half-space `erf` form where lids have ages, the steady conductive form for stagnant lids.

**Conditioning line.** Thermal conductivity is a banked material row. The convicting body is an ICE SHELL: ice conductivity is strongly temperature-dependent where rock's is not, so the `k` row keys on MATERIAL CLASS before a Europa-class world renders. The alien is a data row, not a rewrite.

## The dependency order (ratified as the render agent wrote it)

1. **The geotherm `T(z)`** across the lid, above.

2. **The pressure-dependent brittle branch: BYERLEE'S LAW.** The built `derived_crust_yield_pa` is independent of both temperature and pressure. Byerlee is an alien-admission gift: it is famously near material-INDEPENDENT for rock, so one friction law serves every silicate lid, with ICE as the named deviant.

3. **The silicate creep route.** The Mukherjee-Bird-Dorn form in `crates/materials/src/creep.rs` STAYS. Its diffusion input currently routes through `freezer::self_diffusivity` to `MetallicRoute::cohesive_energy`, keyed by element symbol and scoped to elemental metals, so an olivine lid has no jump rate. The fix routes the activation energy through the engine's own 3b class constant: `E*` from `g · R · T_m`, evaluated on the world's OWN melting temperature. Any material's creep then derives from its own solidus, with measured olivine flow laws as calibrated anchor rows.

   EXPONENT RIDER (checked, since `E*` lives in an Arrhenius exponential): `g` is measured-class and `T_m` is derived, so the product is class-grade in the exponent, the same legal status the freezer already relies on. Band propagated.

## The payoff: T_e falls out, and the isotherm is demoted

`T_e` emerges from the YIELD-STRENGTH ENVELOPE: the brittle curve intersecting the ductile curve at the world's own STRAIN RATE, itself derived from the convective timescale. Nothing in the arc authors a scalar.

The ~600 K limiting isotherm that both shortcuts wanted to author becomes what it always was: the Earth-olivine INSTANCE of the YSE construction. It is demoted from mechanism to HINDCAST CROSS-CHECK. The derived Earth `T_e` must reproduce the oceanic `T_e`-versus-plate-age data, with Mars and Venus elastic-thickness estimates as the second and third rows.

## Fetch list (this round)

- Hirth and Kohlstedt olivine flow-law rows (the calibrated creep anchors).
- Oceanic `T_e` versus plate age (the primary hindcast row).
- Mars and Venus elastic-thickness estimates (hindcast rows two and three).
- Ice conductivity and ice friction (the shell branch, the named deviant).
- Hartmann saturation (gating #87's saturation half, unverified until it lands).

Verify-on-pull discipline: a fetched value is a target to VERIFY against its citation at load, never a digit to trust from this doc.

## Build order

**Commit 1: the four-planet rayon spawn.** Orchestration-only over embarrassingly parallel worlds, gated on PER-PLANET BYTE-IDENTITY against the serial run. This is tooling for the correctness work rather than cosmetics: it makes every derive cycle of this arc several times faster. The measured baseline is roughly one core of eighteen (~100% CPU for the first 150 seconds of `--derived`, peaking at 167%), with the four planets independent by construction and each carrying its own irreducibly serial tick loop.

MEASURED RESULT (landed): the system map derives in 20.2 seconds against the ~150 second baseline, a 7.4x speedup, with the log line-identical (the 0.70 AU skip message still prints in orbit order before the summary). The gate passes: per-planet byte-identity against a serial reference reading the same orbit list, comparing every derived bit including the full tile field. At ~109 seconds it joins the `SLOW_TESTS` nightly filterset by name, never `#[ignore]`d, per the standing convention.

Then the arc proper, in the dependency order above.

## What this arc does not touch

The province field stays lat-lon; its migration is slice 9 (task #86), the next sim arc, under its own pin-freeze. Seam cosmetics are excluded from the render follow-on because #86 obsoletes them. The GPU stays a non-question until the CPU factor is spent.
