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

2. **The pressure-dependent brittle branch: BYERLEE'S LAW.** The built `derived_crust_yield_pa` is independent of both temperature and pressure.

   **THE ROW IS BLOCKED and does not ship until Byerlee 1978 resolves a source conflict** (owner ruling, after the fetch). The gift this document sold, that Byerlee is near material-INDEPENDENT so one friction law serves every silicate lid, HAS A STRESS FLOOR, and the floor INVERTS the gift: the worlds that never reach the universal regime are exactly the small, low-gravity ones the rule was meant to admit, plus the shallowest kilometres of every world. THE CONFLICT: the classical reading of the primary has THREE regimes (roughness-dominated scatter at the lowest normal stresses, a `0.85 sigma_n` branch up to about 200 MPa, and the `0.6 sigma_n` branch with the half-kilobar intercept above), whereas the fetch's source places the UNIVERSALITY FLOOR ITSELF at about 200 MPa. Those are different claims and the primary settles which. Either way the low-stress regime ships with ROUGHNESS AND COHESION BANDS, and the CONDITIONING AXIS IS NORMAL STRESS through `rho g z` FIRST, material SECOND.

   **THE UNIT TRAP, defused by process rather than by care: Byerlee's law is in KILOBARS.** Reading `tau = 0.5 + 0.6 sigma` as MPa is a silent 100x error. Measured rows carry their SOURCE UNITS VERBATIM with the conversion applied ONCE AT INGESTION, newtyped and tested (R-UNITS-PIN jurisdiction), so reading `0.5` as megapascals dies at the TYPE BOUNDARY instead of inside a verdict.

   **ICE UNBLOCKS, and the alien-admitting form arrived from the field itself.** Beeman's laws are recoverable at citation grade without the paper: `tau = 0.20 sigma_n + 8.3 MPa` at pressures at or above 10 MPa, `tau = 0.55 sigma_n + 1.0 MPa` at or below 5 MPa, measured at 77 to 115 K, friction independent of temperature and of velocity over the tested decade (the through-origin variant `mu ~ 0.69` also circulates). The modern work supplies the domain structure the abstract could not: cold and warm ice differ, velocity dependence appears with warmth across 98 to 263 K, near-melt friction collapses to a few hundredths, and the icy-satellite fault literature frames friction in HOMOLOGOUS TEMPERATURE `T/T_m`. So `T/T_m` is the class variable, which conditions ammonia oceans for free (ammonia-doped ice carries its own melting point at 176 K). The headline the abstract states plainly: ice's frictional strength sits well below every rock, so ICE LIDS ARE WEAK ON BOTH BRANCHES of their law.

3. **The silicate creep route.** The Mukherjee-Bird-Dorn form in `crates/materials/src/creep.rs` STAYS. Its diffusion input currently routes through `freezer::self_diffusivity` to `MetallicRoute::cohesive_energy`, keyed by element symbol and scoped to elemental metals, so an olivine lid has no jump rate. The fix routes the activation energy through the engine's own 3b class constant: `E*` from `g · R · T_m`, evaluated on the world's OWN melting temperature. Any material's creep then derives from its own solidus, with measured olivine flow laws as calibrated anchor rows.

   EXPONENT RIDER (checked, since `E*` lives in an Arrhenius exponential): `g` is measured-class and `T_m` is derived, so the product is class-grade in the exponent, the same legal status the freezer already relies on. Band propagated.

## The payoff: T_e falls out, and the isotherm is demoted

`T_e` emerges from the YIELD-STRENGTH ENVELOPE: the brittle curve intersecting the ductile curve at the world's own STRAIN RATE, itself derived from the convective timescale. Nothing in the arc authors a scalar.

THE ISOTHERM IS DEAD, AND NOTHING NUMERIC REPLACES IT (owner ruling 2026-07-16, after the fetch read the primaries). This document previously called it "the ~600 K class number" and demoted it to a hindcast cross-check. Both the number and the demotion were wrong.

The number first: every source states the oceanic limiting isotherm in degrees CELSIUS. The classical statement is 450 +/- 150 C. The arc's "~600 K" is 327 C, BELOW every measured band, and Calmant et al. 1990 state plainly that "no estimate is close to the 600 C isotherm". The error entered as a ruling's summary statistic and propagated verbatim through this document into `crates/physics/src/geotherm.rs`.

The deeper finding is the load-bearing one, and it is why no corrected number replaces it: A LIMITING ISOTHERM IS NOT A PROPERTY OF THE LITHOSPHERE. It is a property of the lithosphere JOINED TO AN AGE CONVENTION. The same measurements imply 550 to 600 C against thermal age and 350 to 450 C against isochron age (McNutt 1984, via Calmant et al. 1990), and trench loads diverge again near 340 C. A single number quoted without its convention is a statistic with a HIDDEN CONDITIONING VARIABLE, so it could never have been a target: it would have validated whichever convention it was silently born under. This is the silent-parameter class living inside the LITERATURE rather than inside our code, which is a place the project had not thought to look, so the DEFAULTS-TAKEN discipline now extends to fetched rows' CONVENTIONS.

THE HINDCAST ROW IS THE DATASET, per the standing rule this produced (hindcast targets in rulings name DATASETS, never summary statistics). The derived `T_e` is checked against the MEASURED `T_e`-versus-age data directly, with a MANDATORY AGE-CONVENTION FIELD per compiled entry and the LOADING ENVIRONMENT SPLIT: oceanic interior loads are the primary set, trenches are a separate tagged environment. Mars (13 regions with epochs, Ruiz 2014 Table 1) and Venus (trimodal, 47 percent below 20 km, Smrekar and Anderson 2005) are the second and third rows. The classical 450 +/- 150 C may appear in prose as commentary, in Celsius, with the convention rider, and nowhere else.

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
