# Ocean and hydrosphere: the to-research backlog

Status: planetary evidence backlog only. This record predates the canonical seven-stage planet runner and names parked living-world fixtures and paths. Nothing here admits a value or authorizes a runpath. Canonical hydrosphere work must enter from the sealed absolute floor, derive first under the Gap and Residual laws, and refuse when closure is absent. References under `parked/` are historical evidence, not implementation targets.

This record queues hydrosphere research when time allows. It records what existed on the old path, what was classified (designed or researched in a document but not built), and what was absent from the 2026-07-15 mapping. Citations are retained so physical evidence can be recovered without treating the former execution path as canonical.

## What is already built (no research owed, works today)

The volatile thermodynamics floor includes derived mechanisms: the water snow line reproduces the Lodders front (`crates/materials/src/equilibrium_condensation.rs`, ~182 K), the Murphy-Koop H2O gas-ice sublimation front (`crates/physics/src/ice_sublimation.rs`), and per-substance saturation, latent heat, and evaporation curves (`crates/physics/src/laws.rs` saturation_vapor_pressure, saturation_slope_from_latent_heat, evaporation_rate; design anchor `docs/working/VOLATILE_THERMODYNAMICS_DESIGN_ANCHOR.md`, owner-ratified 2026-07-11). The ocean-vs-land split and relief classification are derived by crossing a derived sea-level reference (`crates/world/src/terrain.rs` classify_relief, with relief_datum the field mean, never an authored metre band). The former pointwise hydrologic cycle and four-reservoir transport ledger now live under `parked/crates/sim/`; they are legacy evidence and are not on the canonical runpath. Canonical assembly must rebuild any retained mechanism behind the immutable planet snapshot.

## What is classified (designed or researched, needs building not researching)

The old planetary water and volatile inventory was fixture #40, documented in `docs/working/EMERGENT_ATMOSPHERE_PIPELINE_DERIVE_MAP.md` and the retired `parked/docs/working/CAPSTONE_PIPELINE_SCOPE.md`. The old causal viewer and `parked/crates/sim/src/geodynamics.rs` also carried a zero sea-level fixture. Those are forbidden inputs, not pending defaults. The canonical path must derive volatile delivery and partitioning from the sealed floor or emit a structured refusal. The retained invariant is recorded in `docs/working/ABIOTIC_EVIDENCE_DEBT.md`: sea level emerges from the generated water budget and is never a viewer or caller input. Wind-driven circulation remains research debt, and the reduced Hadley-plus-Coriolis parameterization is Terran-biased (`docs/working/GEOLOGY_ARC_PACKET.md` Part B).

## What is absent (research owed, queue these)

1. Ocean currents and circulation (thermohaline overturning, wind-driven gyres, upwelling and downwelling). Nothing in code or docs. It is a partial-differential transport field with no pointwise form, so it needs a transport or field layer the current pointwise substrate does not own (`docs/working/PHYSICS_SUBSTRATE_UPPER_BOUND_AUDIT.md`). Its prerequisite is the highest-value single catch below.

2. A Coriolis law and derived planetary rotation state. No `coriolis` token appears in the physics crate; this is named as the highest-value catch in `PHYSICS_SUBSTRATE_UPPER_BOUND_AUDIT.md`, and it underlies both ocean and atmospheric circulation. The old parked world carried an authored `rotation_period_seconds` and `DiurnalSky`; neither is an admissible datum. Rotation must emerge from the generated angular-momentum state before Coriolis can consume it.

3. Bathymetry and hypsometry (a sub-sea depth field). Today Submarine is a binary relief class with no modelled ocean-floor topography or depth field. Zero hits for the term anywhere.

4. Sea ice, glaciers, and ice sheets as surface features, and glacial, periglacial, and aeolian erosion. Absent entirely (`GEOLOGY_ARC_PACKET.md` names U-valleys, fjords, nitrogen glaciers on Triton and Pluto, and CO2 ice caps on Mars as the uncovered cold-world modes). Water-ice phase chemistry is built (the sublimation and eutectic thermodynamics above), but ice as a landform is not.

5. Ocean-world ice-shell-over-ocean layering (a Europa-style z-stacked ice, ocean, high-pressure ice column). The old authored structure selector is parked at `parked/crates/world/src/structure.rs`; it is historical evidence of the missing representation, not a canonical input. The generated substrate must carry depth layers before this can be represented.

## Named research flags already on the books

R-HYDROSPHERE (audited in `PHYSICS_SUBSTRATE_UPPER_BOUND_AUDIT.md`): the point-laws exist (hydrostatic pressure, buoyant force, drag, speed of sound, column height); the missing half is the water-body depth field plus the transport or field layer, plus the owed Coriolis and rotation primitive. R-WEATHER and R-CELESTIAL-SECULAR cover the Milankovitch orbital-climate drift. `GEOLOGY_ARC_PACKET.md` Part B records the missing glacial and aeolian erosion, hydrosphere-lithosphere coupling, solvent-general fluvial mechanics, and ocean-world interior layering. The retired `parked/docs/working/GAPS_AND_HOLES.md` carries additional historical living-world questions.

## The one discrepancy to keep in mind

The retired design and `parked/docs/working/GAPS_AND_HOLES.md` describe carved rivers, GPU hydraulic erosion, and a fully closed water cycle as if present or covered. The relevant implementations are parked, so the canonical runpath has no admitted hydrologic closure today. Reuse requires a derive-first planetary implementation with snapshot-only observation; copying or wiring the dormant legacy substrate is not an admissible shortcut.
