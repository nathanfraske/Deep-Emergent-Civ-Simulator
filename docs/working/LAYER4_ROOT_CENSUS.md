# Layer-4 root census: the world-seed schema as a causal graph (cross-lane)

This is the layer-4 draw arc's FIRST DELIVERABLE, ruled ahead of any wire (research agent, owner-signed 2026-07-18). It is a CROSS-LANE document, not disk-lane private: the root set IS the world-seed schema, shared infrastructure, and the midband lane's radiogenic inventories drink from the same epoch-and-environment aquifer, so it goes to the coordinator.

THE PREMISE THE CENSUS ENFORCES. The queued draws are NOT a list of independent dials; they are NODES IN A CAUSAL GRAPH. The honest draw set is the graph's ROOTS, with everything downstream DERIVED and the demographics repositioned as PUSHFORWARD VALIDATION (a derived distribution is validated against the observed one, never seeded by it). Drawing a downstream quantity independently while the engine also owns its cause is two doors to one fact, and worse, it AUTHORS AWAY the correlation the cause induces. Build order follows the census: roots first, derivables as ruled slices, demographics last and only as referees. This is the floor's standing lesson generalized: the descent does not end at a list of drawn scalars, it ends at the roots of one graph, every rung down having converted an input into a consequence.

## 1. The node classification (root / derivable / validation)

Each queued quantity is classified, with its correlation edges and its retirement.

- **Cosmic epoch (redshift z): ROOT.** No draw and no measure exist today (a named debt). The measure to DESIGN is a star-formation-history-class distribution (the cosmic star-formation-rate density versus z), the arc's one true design nut, because a fetch cannot design a schema. It is the cross-lane aquifer: z sets the CMB floor (the cloud-core temperature's absolute minimum, scaling as `(1+z)`), and the same z conditions metallicity (chemical enrichment over cosmic time) and the midband lane's radiogenic inventories (the live decay budget at formation). A root; carries its Terran conditioning (the Milky-Way present-epoch instance is one draw).

- **Environment class (cluster / field / association): ROOT.** No draw today (a named debt). The measure is the fraction of stars forming in each environment and the environment's own property distributions. It conditions the cloud-core temperature (cluster cores run warmer, a measured lever), the disk-truncation statistics (binarity, external photoevaporation), and the birth-rotation distribution. A root; Terran-conditioned.

- **Metallicity ([Fe/H]): ROOT (partial), drawn today.** The `scaled_metals_by_dex` abundance draw exists (`crates/physics/src/solar_abundances.rs`), so this axis is shaped. It is partly a CONSEQUENCE of epoch and environment (galactic chemical evolution), so the honest form conditions it on z and galactocentric position once those roots land; today it stands as an independent root draw, an approximation to be declared until the conditioning edges are built.

- **Core angular momentum (the specific angular momentum / velocity gradient of the collapsing core): ROOT.** This is the deeper root UNDER `R_1`. Measured core velocity-gradient distributions exist in the same NH3 survey literature the cloud-core-temperature basis cites. A root; conditions `R_1` and everything the core's angular momentum touches.

- **`R_1` (disk initial / characteristic radius): DERIVABLE, not a root.** The disk's birth size is the CENTRIFUGAL RADIUS of the collapsing core's angular momentum (the rotating-collapse relation `R_c ~ j^2 / (G M)`). So `R_1` DERIVES from the core-angular-momentum root, and the resolved-disk-size demographics (DSHARP, size-luminosity relation) demote to VALIDATION of the derived distribution. Drawing `R_1` independently while the engine owns core rotation would be two doors to one fact and would author away the correlation between disk size and everything else the core's angular momentum sets.

- **Cloud-core temperature `T_core`: DERIVABLE (route two), drawn interim today.** Reserved as `disk_clock.cloud_core_temperature_k` (cold-edge interim, owner-signature-pending). Route two (recorded) derives it from a thermal balance (cosmic-ray heating against line and dust cooling, the Goldsmith 2001 class), keyed on the environment vector (ionization rate, density, radiation field, abundances, CMB floor at epoch). At that rung the measured distribution demotes to a validation hindcast. Admits the alien only if the environment vector is itself drawn or derived per world; hardcoding the local values would be fossil-laundering one level below the reservation it retires.

- **`Omega_star_0` (stellar birth rotation at disk dispersal): DERIVABLE, not an independent root.** Birth rotation is regulated by DISK LOCKING, so the rotation at disk dispersal is CORRELATED with the engine's own `tau_disk`. An independent `Omega` draw authors that correlation to zero. The deeper form draws the primordial rotation and the locking parameters (the roots) and DERIVES the release-time spin, with the young-cluster rotation distributions validating the JOINT statistics rather than seeding an independent marginal. The gyrochronology spin-down (Skumanich / Barnes / Mamajek-Hillenbrand) ages it forward, but only AFTER the star leaves the disk-locked / C-sequence regime (the calibration is invalid inside its own birth window, roughly before the 100 Myr gyrochrone).

## 2. The correlation edges (what an independent draw would author away)

- **Disk-locking:** `P_rot` and disk presence are NOT independent at birth (disk-bearing stars are biased toward the slow-rotator mode; vendored source-verbatim, Rebull et al. 2018, Herbst et al. 2001). The joint `P(P_rot, disk)` does not factor. A TERMS-DROPPED line rides any independence assumption between the rotation draw and the disk draw.
- **Centrifugal radius:** `R_1` and core angular momentum are one fact; the disk-size-versus-everything correlations flow from the core's `j`.
- **The cosmic address:** epoch and environment condition temperature, metallicity, and (cross-lane) radiogenic inventories; independent draws of the downstream axes author those edges to zero.

## 3. The circularity checks (spent-row vigilance one level down)

When a demographic demotes to validation of a derived pushforward, the root measure's OWN literature construction must be checked for circularity: a root measure partly INFERRED from the same downstream statistic it will later validate cannot referee that pushforward. Concretely: a core-rotation measure partly inferred from disk statistics could not then validate derived disk sizes. Each root measure carries a construction-provenance note stating what it was and was NOT inferred from, before it is admitted as a referee.

## 4. Terran conditioning (admit-the-alien on the roots)

Every root measure carries its Terran conditioning fields, since the Milky-Way present-epoch instance is one draw from a wider space: the environment-class fractions, the epoch, the metallicity relation, and the core property distributions are all measured in one galaxy at one epoch. A root measure that hardcodes the local value where an alien world's would differ is a defect; the door is built before the visitor exists.

## 5. Vendored root measures (status)

The fetch batch is landing the cited populations. Vendored source-verbatim so far (bytes in scratchpad, receipts pending the physics-lane manifest landing):

- **Birth rotation:** Herbst et al. 2001 (ONC, ~1 Myr bimodal, mass-dependent, `0d24c268...`); Rebull et al. 2018 (Upper Sco / rho Oph, K2, disk-locking direct evidence, `d5d190c5...`). The A&A 396, 513 Herbst 2002 bytes are a residual gap (publisher DB down), so the ~0.5-1 Myr disk-locking half-life is unvendored.
- **Spin-down:** Skumanich 1972 (`v ~ t^(-1/2)`, coefficient set by the calibrating points, `9c4f2d4a...`); Barnes 2007 (`P = f(B-V) g(t)`, `n = 0.5189`, `1b6e3a14...`); Mamajek-Hillenbrand 2008 (revised `a,b,c,n`, `9e407163...`). Validity: I-sequence dwarfs only, ~130 Myr to 4.566 Gyr; NOT the disk-locked birth population; the two coefficient sets disagree at the ~50% level in the Hyades, so the set is a modeling choice.
- **alpha (transport-side, for `t_visc`):** the census table (method x regime x mechanism) is vendored (Hartmann 1998 transport `~1e-2`; Flaherty 2015/2018 turbulence `<1e-3`; Gammie 1996 dead zone; Bai-Stone 2013 and Lesur 2023 PPVII simulation predictors). The LBP clock consumes the transport-inferred Cell A/B (`~1e-2`), NOT the ALMA-measured turbulence, and the turbulent-viscosity framework itself is contested for outer disks (winds, dead zones).
- **Cloud-core temperature (gas kinetic axis):** Jijina et al. 1999 (264-core NH3, `0bdb3db6...`), overall median 14.7 K (IQR 11.0 to 20.5), floor 7.5 K, ENVIRONMENT SPLIT field 12.0 K versus cluster 20.5 K (confirmed verbatim); Benson-Myers 1989 (`76d2e6c0...`, 10 to 15 K bulk); Planck ECC/PGCC (`ab66c479...`) DUST color-temperature median ~13 to 14.5 K, a SEPARATE axis from the gas kinetic temperature (not interchangeable). Input-audit: the "8.9 to 20.7 K, mean 12.3 K" Planck figure was misattributed and is retired.
- **`R_1` (disk size):** two distinct radii, kept on separate axes. For the viscous SCALE radius, Tazzari et al. 2017 (Lupus, self-similar `R_c`, `e13e1e15...`): bulk 25 to 100 AU, ~18/22 disks below 75 AU, floor ~14 AU, tail to ~200 to 430 AU, correlated with disk mass. For the dust effective radius, Tripathi et al. 2017 (`bf760ade...`, `log R_eff = 2.12 + 0.50 log L_mm`, 0.19 dex scatter) with Andrews 2020 (`ee01f355...`, `R_mm ~ M_star^0.9`, `L_mm ~ M_star^1.7`) for the stellar-mass conditioning; DSHARP (`61296dad...`) is a bright/large-selected anchor, NOT a demographic median. The gas `R_c` is the closer analogue to the LBP scale radius `t_visc` reads.
- **PENDING:** the core-angular-momentum velocity-gradient distributions (the `R_1` ROOT under the demographics), and the epoch and environment-class root measures (which must be DESIGNED, not only fetched).

## 6. Build order (the census dictates it)

1. Design the epoch measure (the star-formation-history distribution) and the environment-class measure: the roots with no existing draw. This is the design work fetches cannot do.
2. The core-angular-momentum root draw, then `R_1` derived via the centrifugal radius, demographics as validation.
3. The primordial-rotation + locking roots, then `Omega_star_0` derived through disk-locking (correlated with `tau_disk`), young-cluster distributions validating the joint.
4. `T_core` route two (Goldsmith thermal balance) when the environment vector exists; until then the reserved cold-edge interim stands.
5. Metallicity conditioning edges (on epoch and position) when the roots land.
6. Demographics wired last, as referees only.

Each derivable lands later as its OWN ruled slice; depth is recorded here, not sprinted at. The coordinator owns the world-seed schema this census defines.
