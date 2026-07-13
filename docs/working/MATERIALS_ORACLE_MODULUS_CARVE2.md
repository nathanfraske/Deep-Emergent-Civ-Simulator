# Materials oracle, corrected modulus carve: bulk modulus from lattice curvature on the radius (design-first)

> GENERALIZED, not superseded-and-wasted (#182, generator architecture). The owner's generator architecture
> (`docs/working/MATERIALS_ORACLE_GENERATOR_ARCH.md`, `MATERIALS_ORACLE_EWALD_CARVE.md`) dissolves both seams this
> carve surfaced, and the gate ruled this build the AB point-charge SPECIAL CASE that empirically validates it:
> the tabulated-Madelung prototype library generalizes to the Ewald charge-lattice sum (an Ewald kernel computes
> it over positions, `A2B3` included), and the ionic-versus-covalent fork generalizes to one charge-equilibration
> solve (QEq's partial charge corrects the divalent-oxide overestimate from first principles). The build
> (`lattice_modulus.rs`, the Shannon-radius, Born-exponent, and prototype data files) STAYS on the branch as the
> foundation and validation: the radii feed the bond-valence positions and the Born-Mayer repulsion, the Born
> exponents feed the repulsion, the prototype key demotes to a memoization index, and `B = (n-1)A/(18 r0^4)`
> becomes the AB fast path and a validation check. NOT added, per the gate: the reserved correction factor (QEq
> supplies the partial charge) and the tabulated `A2B3` Madelung (Ewald computes it). The retags stand (the
> atomization column to `[M, floor-and-validation]`, `E_coh/V` to the labeled screen tier). The whole slice
> merges once QEq gives the corrected partial-charge `B`, not the point-charge value alone. This document is kept
> as the record of the two seams the special case surfaced and the architecture that dissolves them.

Agent B, the gate's reframe on #182. The owner's research redirects the modulus derivation from the
cohesive-energy-density estimator to the principled radius-curvature route for the seed registry's
ionic-covalent oxides, and splits the emitted property into the derivable bulk modulus and the class-dispatched
shear modulus. This carve confirms the reframe against source, records the retags already applied, and surfaces
the floor additions the corrected route needs for the gate's ruling before any authoring. No build beyond the
retags until the gate rules the additions.

## The reframe, confirmed against source

Four load-bearing claims, checked rather than trusted (Prime Directive 1 holds for the owner's input too):

- **The cancellation theorem holds against the actual disposer.** `apparent_gibbs_energy`
  (`crates/physics/src/petrology.rs:83`) reads `enthalpy_formation`, `standard_entropy`, and `molar_volume`
  only; `stable_assemblage` minimizes over these `dG_f` rows at fixed bulk composition. The elemental
  atomization sum is identical across candidate assemblages and cancels by Hess, so the disposer never reads the
  atomization column. The column was never petrology substrate; the gate is right.
- **The Born-Lande bulk modulus is textbook and reproduces NaCl.** `B = (n-1) A / (18 r0^4)` with the Coulomb
  and charge prefactor folded in: `n = 8` (the mean of the Na+ neon-core 7 and Cl- argon-core 9), `A = 1.7476`
  (the rock-salt Madelung constant), `r0 = 2.82 A` gives `B ~ 24.8 GPa` against a measured 24 to 25. This is the
  standard Kittel result; it is correct.
- **The sensitivity is radius-dominated.** `B ~ 1/r0^4`, so a fractional radius error enters the modulus
  fourfold. A pm-grade Shannon radius carries the derivation; a tens-of-percent `Z_eff` radius estimator gives
  order-unity modulus error. The load-bearing measured column is the radius, not the energy.
- **The shear modulus is the un-derived half.** A central-force model (Born-Lande, Rose, London) obeys the
  Cauchy relation `C12 = C44` as a theorem, so a purely central model cannot produce the shear violations that
  are the bonding-class fingerprint (NaCl near-Cauchy, gold a large violation, silicon inverted). Bulk modulus
  is the cleanly derivable half; shear needs one added class-dispatched ingredient. My slice emitted a single
  "elastic modulus" from `E_coh / V`, which conflates the derivable `B` with the un-derived `G` (Young's modulus
  is `E = 9BG/(3B+G)`, a mix of both). The critique is fair.

The reframe is sound. The estimator I built is the wrong tier for the oxide phases it was aimed at, and it
labeled a bonding-agnostic stiffness scale as though it were the modulus.

## Retags already applied (the gate's immediate directives)

Three cheap, unambiguous relabels are in this push, byte-neutral (doc and comment only, no value moved):

- **The atomization column retags `[M, floor-and-validation]`** (`periodic.rs`). Its doc now states it is not
  petrology substrate (the disposer cancels it), it is the metallic/quick-screen estimator input and the
  standing validation battery for the estimators across the Brewer/JANAF elements, the measured-helium-viscosity
  role with the arrow reversed. The column stays; its cited values are unchanged.
- **The `E_coh / V` code relabels the metallic / invented-element / quick-screen tier `[E]`**
  (`materials_oracle.rs`), a bonding-agnostic order-of-magnitude stiffness scale, explicitly not a separated bulk
  or shear modulus and not the principled route for the ionic-covalent oxides. The functions read
  `phase_elastic_modulus` still (the code name), documented as the quick-screen scale.
- **The `mat.elastic_modulus` retirement flag softens** (`mechanical_floor.toml`): Young's is retired by the
  principled `B`-derived-plus-`G`-class-dispatched route, not by the quick-screen scale alone.

The fmt break the gate flagged (`periodic.rs:549`, `:569`) is fixed in commit `bbb1e5a` (`cargo fmt --all`; my
earlier run was crate-scoped). CI re-running on that SHA.

## The corrected carve (surface the floor additions for the gate's ruling)

The principled bulk modulus for the ionic class, then the named shear debt:

1. **Add the Shannon ionic radius as the load-bearing `[M]` per-element column** (pm-grade, cited, per-element
   like the atomization column but this one is load-bearing). This is the real foundation, the column the
   modulus rides fourfold.
2. **Derive `B` the column-pure way for the ionic class**, `B = (n-1) A / (18 r0^4)`. The charge product `z+ z-`
   the Coulomb prefactor needs is already on the floor (the `valence` column on the periodic table), so a doubly
   charged pair (MgO) picks up its fourfold Coulomb factor from data, not an author.
3. **Emit `B` specifically** (the derivable half) as the property, and NAME the shear debt: `G` is
   class-dispatched to its own slice (ionics near-Cauchy so `G` follows from `B` and the Cauchy relation;
   covalents the Keating `beta/alpha` ratio; metals an EAM embedding functional; or the cheap Pugh/Poisson
   class-ratio tier `G = (G/B)_class * B_derived`). The quick-screen `E_coh / V` scale keeps its labeled tier for
   metals, invented phases, and first passes.
4. **Retag the spec's Stage-6 line** "K, G from lattice curvature" to "K derived, G class-dispatched."

The two class-constant inputs the `B` derivation needs, surfaced for your ruling on how they enter the floor:

- **The Madelung constant `A` per structure type.** For a phase that maps to a tabulated prototype (rock-salt,
  CsCl, fluorite, corundum) `A` is an exact class constant. Ruling wanted: seed a minimal prototype/Madelung set
  now, or carry the relevant structure constants as cited class data on the phase rows.
- **The Born exponent `n` per noble-gas core.** The Pauling noble-gas-core series (He 5, Ne 7, Ar 9, Kr 10, Xe
  12), a per-ion class constant averaged over the cation and anion cores. Ruling wanted: a per-element
  `born_exponent` column keyed off the ion's core, or a small cited core-to-exponent table the derivation reads.

## Two seams I found auditing the reframe (surfaced before building)

Auditing the input the same way the input-audit surfaced the atomization gap, two real seams sit inside the
"derive B for the ionic class" instruction:

- **Seam A, the class assignment is itself the crux, and half my registry is not cleanly ionic.** The seed
  phases split by bonding: periclase (MgO) is a clean rock-salt ionic prototype, corundum (Al2O3) and hematite
  (Fe2O3) are ionic with the corundum structure, but QUARTZ (SiO2) is a covalent corner-sharing framework and
  FORSTERITE and FAYALITE (Mg2SiO4, Fe2SiO4) are orthosilicates with multiple cation sites. A single-`A`,
  single-`r0` rock-salt Born-Lande does not cleanly apply to a covalent framework or a multi-site orthosilicate.
  So the per-phase class must be a data-driven assignment (a `bonding_class` or prototype key on the phase row),
  and quartz and the olivines likely route to the covalent Keating tier, not the ionic `B`. Ruling wanted:
  confirm the ionic-`B` slice covers only the phases that map to a tabulated ionic prototype (periclase,
  corundum, hematite), with quartz and the olivines carried on the labeled quick-screen tier until the covalent
  route lands, rather than forcing an ionic formula onto covalent phases.
- **Seam B, "exact from the prototype library" holds only for phases that map to a prototype.** The Madelung
  constant is a single tabulated number for a simple prototype, but for an arbitrary silicate it is an Ewald sum
  over the actual crystal structure, which needs the atom positions, a far larger data input than a per-element
  radius. So the prototype library is exact for the phases keyed to a prototype and undefined for the rest.
  Ruling wanted: seed the minimal prototype set (rock-salt, corundum) and gate the ionic-`B` derivation to
  phases carrying a prototype key, so a phase with no prototype falls through to the labeled quick-screen tier
  (an honest `None`-or-estimator, never a fabricated `A`), the same absent-not-zero discipline the rest of the
  floor uses.

Both seams point the same way: the corrected slice should derive `B` for the phases that map to a seeded ionic
prototype and carry a per-phase class/prototype key as the dispatch, rather than assume every registry phase is
a rock-salt ionic crystal. That keeps the derivation honest at the boundary the owner's own Keating/EAM tiers
mark.

## Scope and constraints

Byte-neutral and dormant per slice (no scenario reads the modulus). No value authored that a derivation should
produce: the Shannon radius and the Born exponent are measured/class `[M]` component constants (the legitimate
floor, keyed per element and per core so an alien chemistry is a data row), the Madelung constant is an exact
structure constant `[C]` for a seeded prototype, and `B` is derived `[D]` from them. Fixed-point determinism
(the `1/r0^4` is a `checked_mul` chain, the `r0^4` well inside Q32.32 for pm-to-angstrom radii). The provenance
tags bind to A's enforced enum when its register lands, a local placeholder until then. No build of the radius,
Madelung, Born, or `B` derivation until the gate rules the floor additions and the two seams above.

## Foundation slice built (gate ruled the additions and confirmed both seams)

The gate approved the floor additions and confirmed both seams (build it slightly larger than one property, the
radius column and the prototype dispatch are what the oracle stands on). Built on
`claude/materials-oracle-modulus-slice`:

- `crates/physics/data/shannon_radii.toml` (the load-bearing `[M]` column, Shannon 1976), keyed by (symbol,
  charge, coordination); `born_exponents.toml` (the `[M class]` Born exponent by noble-gas core, Pauling); and
  `prototypes.toml` (the minimal aristotype library: rock-salt populated, corundum seeded with its Madelung held
  absent). A per-phase `prototype` key on the phase registry (periclase rock-salt, corundum and hematite
  corundum) is the data-driven bonding-class dispatch.
- `crates/physics/src/lattice_modulus.rs`: `phase_bulk_modulus_ionic` derives `B = (n-1) A |z+ z-| (e^2/4pi
  eps0) / (18 r0^4)` for a prototype-mapped ionic phase. The charge product comes from the formula and the
  anion valence by charge balance (`identify_ionic_pair`, no authored per-phase charge), the Born exponent from
  each ion's electron count mapping to a noble-gas core, the Madelung constant and coordination from the
  prototype. The Coulomb energy and the eV/A^3-to-GPa conversion are the two cited fundamental law constants.
- `Cl` gained its cited common oxidation states on the periodic table (needed for the NaCl validation and any
  future chloride), byte-neutral (no chloride phase in the registry).

Two findings surfaced by the build, for your ruling on the follow-on:

- **The point-charge model overestimates the divalent oxides.** The bare full-formal-charge Born-Lande derives
  NaCl to about 2 percent (24.4 GPa against a measured 24 to 25, the clean monovalent validation), but periclase
  to about 266 GPa against a measured 160 to 165, a systematic 1.6-fold overestimate because covalency lowers
  the effective charge below the formal `+/-2` and the point-charge model omits it. The derivation is the correct
  derive-first FORM (`B` from radius, charge, structure, no per-rock authoring), and the overestimate is a
  documented honest limit with the partial-charge refinement named; the reserved correction factor
  (`B_measured / B_pointcharge` across the validation set) is surfaced on the function, the emitted band zero
  until it is set. Ruling wanted: accept the derived-but-approximate `B` with the documented divalent systematic
  and the reserved correction, or hold the oxide `B` behind the partial-charge refinement first.
- **The corundum `A2B3` phases fall through this slice, on both counts.** Corundum and hematite map to the
  corundum prototype structurally, but its reduced Madelung constant is held absent (the `A2B3` non-1:1 lattice
  sum is an Ewald sum, not a single tabulated `AB` constant, which I will not fabricate), and independently
  hematite's `Fe3+` (`[Ar]3d5`, 23 electrons) has no clean noble-gas Born core. So only periclase (and the NaCl
  reference) derive this slice; corundum and hematite fall through to the screen tier, an honest `None`. Ruling wanted:
  ground the corundum reduced-Madelung convention and the transition-metal Born value as the next prototype
  addition, or leave the `A2B3` oxides on the screen tier until the covalent and `d`-electron routes land.
