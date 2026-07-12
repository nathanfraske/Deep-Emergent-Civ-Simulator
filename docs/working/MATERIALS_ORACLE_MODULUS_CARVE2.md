# Materials oracle, corrected modulus carve: bulk modulus from lattice curvature on the radius (design-first)

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
