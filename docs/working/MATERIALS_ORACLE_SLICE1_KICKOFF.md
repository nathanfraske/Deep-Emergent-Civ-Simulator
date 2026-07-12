# Materials oracle, first slice: composition-derived elastic modulus (design-first)

Agent B, the gate's downtime redirect (the interior arming holds for A's 3c). The owner's spec is
`docs/working/MATERIALS_ORACLE_SPEC.md` (on `origin/claude/materials-oracle-spec`, read at source). This
opener carves ONE buildable slice of the nine-stage oracle and surfaces the carve for the gate's ruling
before any build, off current main `64da409` on a branch separate from the interior arming.

## The arc, from the spec

The oracle deletes the authored named-substance floor (the mineral registry, the formula column, the
per-class strength anchor) and makes a material's properties a pure function of composition, environment,
path, and seed: `K_solid(x_local; E; h, seed) -> {phases, modes, solution compositions, texture, properties
+/- bands, records, flags}`. It is the mantle-density derivation (`derive_mantle_density` over A's petrology
kernel) generalized to the whole property set. Every value carries a provenance tag from the seven-tag
register ([D] derived, [M] measured, [E] estimator, [C] closure, [A] authored, [W] written state, [X]
contingency), the same enforced enum A is standing up in the register Phase 1.

## What is built, and the seam

On main: `crates/physics/src/petrology.rs` carries the disposer's core, `stable_assemblage` (the Gibbs-min
over the phase registry, Stage 4) returning an `Assemblage { phases, total_gibbs }`, and `assemblage_density`
/ `crustal_density` (Stage 6's density-from-composition, already derived and wired through
`derive_mantle_density`). The property to carve is the elastic modulus: `crates/physics/data/mechanical_floor.toml`
authors `mat.elastic_modulus = 200000` (Young's modulus, MPa), a bulk authored value with no composition
behind it. That authored row is the retirement target. The seam is Stage 6 property emission extending the
same assemblage read the density already uses: the assemblage's phases and modes are the input, the modulus
is the new derived output.

## The carved first slice

Stage-6 property emission for ONE property, the elastic (bulk) modulus, becoming a composition-derived output
from the assemblage, retiring the authored `mat.elastic_modulus`, byte-neutral (a new derivation armed by no
scenario, the `derive_mantle_density` pattern one property over). The derivation follows the spec's Stage 6
route and its provenance lookup order (measured row, then estimator, then compute-once, then
authored-with-flag):

1. **Per-phase elastic data ([M], relocated down to components).** Each phase carries a measured elastic
   modulus, a component-level measured datum (the owner's rule: measured/closure floor relocates DOWN to
   components, tagged not eliminated), the elastic sibling of the per-phase `rho0` the density already reads.
2. **The estimator fallback ([E]).** Where a phase has no measured elastic row, the stiffness scale
   `E ~ E_coh / V` (the spec's `B0 [E from E_coh/V]`, `1 eV/A^3 = 160 GPa`), the cohesive energy over the
   molar volume, so an invented phase still emits a modulus.
3. **The aggregate ([D]).** The assemblage modulus from the per-phase moduli over the mode fractions, the
   Voigt-Reuss-Hill mean as the first pass with the Hashin-Shtrikman bounds (Stage 7's aggregation form) as
   the refinement, so a rock's stiffness falls out of its mineralogy rather than an authored per-rock value.

The output is a pure function `assemblage_elastic_modulus(assemblage, ...) -> {value, band, provenance}`, the
Stage-6 shape (a value with its error band and its provenance tag). Byte-neutral: no scenario reads it yet,
so the pins hold, exactly as `derive_mantle_density` landed dormant.

## Seams surfaced for the gate's ruling (before building)

1. **The aggregation form.** Voigt-Reuss-Hill mean as the first pass, Hashin-Shtrikman bounds as the
   refinement (Stage 7). Confirm VRH-first, or HS bounds from the start.
2. **The per-phase elastic datum.** A new per-phase measured elastic column ([M], the `rho0` sibling), versus
   deriving every phase's modulus from `E_coh / V` ([E]) with no measured column at all. The spec's lookup
   order wants both (measured first, estimator fallback); confirm the slice adds the measured column now or
   ships estimator-only first and adds the measured rows as they are cited.
3. **`E_coh` and `V` exposure.** The estimator needs the cohesive energy and the molar volume per phase.
   `assemblage_density` computes the volume; `E_coh` is not yet exposed by the kernel (the phase data carries
   formation energies, not a cohesive-energy column). Confirm whether `E_coh` is a new per-phase [M/A] anchor
   (the spec's `E_b cohesive anchors [M/A]`) the slice adds, or derived from the existing bond data.
4. **The provenance-enum binding.** Every value the slice touches carries A's enforced seven-tag enum, not a
   parallel one. I design against the tag semantics now and bind to the real enum when A's register Phase 1
   lands (tell me when it does). Confirm I hold the actual tagging until the enum is on main, or scaffold a
   local shape to be swapped.
5. **The retirement target.** `mat.elastic_modulus` (Young's modulus, 200 GPa authored) is the row I retire.
   Confirm it, or name a cleaner first target (a `fluid.bulk_modulus` row).

## Constraints and scope

Byte-neutral per slice (a new derivation armed by no scenario holds the pins); no value authored that the
spec marks [D]; the [M]/[C] measured and closure constants are the legitimate floor, tagged and relocated
down to components, never eliminated; fixed-point determinism (the assemblage solve is already fixed-point).
This slice does NOT touch the other eight stages, the full disposer refactor, or the named-substance-floor
deletion beyond the one retired row: it proves the deletion pattern on one property end to end, so the larger
refactor builds on a demonstrated shape. No build until the gate rules the carve.
