# Materials oracle, generator architecture: the Ewald summation kernel (design-first, the first buildable piece)

Agent B, the gate's third sharpening on #182: the generator architecture
(`docs/working/MATERIALS_ORACLE_GENERATOR_ARCH.md`, read at `origin/claude/materials-oracle-generator-arch`).
It dissolves both seams the radius-curvature carve surfaced by naming the GENERATOR behind each cache rather than
widening a registry gate: the Madelung constant is the Ewald charge-lattice sum (seam 2 becomes a kernel, not a
table), and ionicity is a charge-equilibration solve over banked columns (seam 1 becomes a solve, not a fork).
The gate's recommended first buildable piece, which I take, is the Ewald summation kernel: the load-bearing
generator everything downstream stands on, a pure deterministic mechanism with no floor data, and self-validating
against the known Madelung constants. This carve surfaces its design for the gate's ruling before building.

Carve-2 (`lattice_modulus.rs`, the Shannon-radius, Born-exponent, and prototype data files) is NOT reverted: the
gate ruled it the AB point-charge SPECIAL CASE that empirically validates the generator (its two honest walls,
the divalent-oxide overestimate and the refusal to tabulate the `A2B3` Madelung, are exactly the two problems
the architecture dissolves). It stays on the branch as the foundation: the Shannon radii feed the bond-valence
positions and the Born-Mayer repulsion, the Born exponents feed the repulsion, the prototype key demotes to a
memoization index, and `B = (n-1)A/(18 r0^4)` becomes the AB-prototype fast path and a validation check. What is
NOT added (the gate's ruling): the reserved point-charge correction factor (QEq's partial charge supplies it
from first principles) and the tabulated `A2B3` Madelung (the Ewald kernel computes it over positions). The
whole slice merges once QEq gives the corrected partial-charge `B`, since a known-approximate point-charge oxide
`B` is not landed alone. The constructor-gate failure carve-2 tripped is cleared by expressing its two physical
law constants as exact rationals (`from_ratio`) and classifying its three data-loader decimal parses as
deserialization, not by reverting. The two earlier retags stand: the atomization column is
`[M, floor-and-validation]`, and `E_coh/V` is the labeled metallic/quick-screen tier.

## Why Ewald is the foundation

Every downstream piece reads the Ewald kernel. The Madelung cache is its output. The charge-equilibration solve
(QEq) minimizes `E(q) = sum(chi_i q_i + 0.5 eta_i q_i^2) + Ewald(q)`, so the Coulomb term in the objective IS an
Ewald evaluation. The final energy assembly (Ewald on computed charges, plus Born-Mayer, plus London, plus
Keating) leads with Ewald. So the kernel is the base of the whole generator stack, and building it first proves
the architecture's load-bearing claim ("the Madelung constant is Ewald of the positions") before any floor
column is added.

## What the kernel is

The electrostatic energy of a periodic array of point charges is a conditionally convergent lattice sum; Ewald's
split makes it absolutely and rapidly convergent by adding and subtracting a screening Gaussian around each
charge, so the energy divides into three exactly-summable parts:

1. **The real-space sum**, short-ranged, the bare Coulomb interaction screened by the complementary error
   function: `sum over pairs and lattice images of q_i q_j erfc(alpha r) / r`, converging in a few near shells.
2. **The reciprocal-space sum**, long-ranged, the screening Gaussians summed in Fourier space:
   `(2 pi / V) sum over reciprocal vectors G of exp(-G^2 / 4 alpha^2) / G^2 times |S(G)|^2`, where the structure
   factor `S(G) = sum_i q_i exp(i G . r_i)` and `V` is the cell volume.
3. **The self-energy correction**, subtracting each charge's interaction with its own screening Gaussian:
   `-(alpha / sqrt(pi)) sum_i q_i^2`.

The splitting parameter `alpha` and the two cutoffs (the real-shell radius and the reciprocal-vector radius) are
CONVERGENCE parameters, not physics: the Ewald energy is independent of `alpha`, which only trades real-space
work against reciprocal-space work. They are reserved with basis (chosen so the truncated sums converge to a
stated tolerance, a performance-and-accuracy bound, not a world value), and their correctness is proven by the
kernel reproducing the known Madelung constants, not asserted.

The one honesty clause is the polar cell: a cell with a net dipole carries a conditionally convergent surface
term whose value depends on the boundary at infinity. The kernel resolves it by the standard TIN-FOIL
(conducting) boundary convention, which sets the surface term to zero, a DECLARED choice recorded in the output,
never a silent fudge. A charge-neutral non-polar cell (the common rock-forming case) does not reach this clause.

## The self-validation (the proof that Madelung is Ewald of the positions)

The Madelung constant `M` of a structure is defined through its electrostatic energy per formula unit,
`E = -M q^2 / r0` for a binary with formal charges `+/- q` and nearest-neighbour distance `r0`. So the kernel,
handed the known crystal structure (lattice vectors and fractional coordinates) and formal charges, must
reproduce the tabulated Madelung constants to a grade:

- rock-salt (NaCl): `1.747565`
- caesium chloride (CsCl): `1.762675`
- fluorite (CaF2): `2.519394` (per the `MX2` convention stated with the result)
- corundum (Al2O3): the tabulated reduced value, the `A2B3` case the radius-curvature carve could not tabulate
  by hand, here an exact Ewald output

Reproducing these to a tight tolerance is the exact proof the architecture rests on, and it doubles as the
accuracy check on the fixed-point transcendental evaluation below.

## The determinism plan (fixed-point, and the transcendental seam)

The kernel is fixed-point and deterministic, the same discipline as the assemblage solve. The two transcendental
functions Ewald needs, the complementary error function `erfc` and the exponential `exp`, are not yet on the
`Fixed` type (which carries `powf` for the cube roots the density solve uses, precedent that a deterministic
fixed-point transcendental is in scope). This is the one real implementation seam, surfaced for the gate's
ruling: whether to implement `erfc` and `exp` as fixed-point series or as the Abramowitz-and-Stegun rational
approximations, each evaluated in Q32.32 with an accuracy budget the Madelung self-validation verifies (the
tolerance on the reproduced constants is the acceptance test on the approximation). The reciprocal sum's `1/G^2`
and `exp(-G^2/4 alpha^2)` terms and the `O(1)` charges keep the arithmetic well inside the Q32.32 range for
crystallographic cells; the lattice and reciprocal sums are over bounded, deterministic shells.

## The interface and byte-neutrality

A pure function over a unit cell: `ewald_energy(lattice, fractional_positions, charges) -> Fixed` (the
electrostatic energy per cell in reduced units), with a thin `madelung_constant(...)` wrapper that divides out
`q^2 / r0` for the binary validation cases. No floor data: the kernel is mechanism only, so it adds no authored
value and reads no registry. Byte-neutral and dormant: nothing calls it yet, so the pins hold, exactly as the
prior dormant derivations landed. The validation structures (NaCl, CsCl, fluorite, corundum cells) are cited
test fixtures in the test module, the known crystal structures, not floor data.

## Seams surfaced for the gate's ruling (before building)

1. **The fixed-point transcendental route.** `erfc` and `exp` in Q32.32 by series or by the Abramowitz-and-Stegun
   rational approximation, with the Madelung self-validation as the acceptance test on the accuracy budget.
   Confirm the approach, or name a preferred approximation and tolerance.
2. **The `alpha` and cutoff reservation.** The splitting parameter and the two shell cutoffs are reserved
   convergence parameters (basis: the tolerance at which the truncated sums reproduce the known Madelung
   constants, an `alpha`-independent result). Confirm they are engine convergence parameters, not world values.
3. **The tin-foil convention for polar cells.** Declared and recorded in the output. Confirm tin-foil, or name
   the boundary convention the world should carry.
4. **Build-now versus hold.** The kernel is pure mechanism, self-validating, and byte-neutral, so its design
   risk is low. Confirm I build it now on the gate's go, or hold for a ruling on the transcendental seam first.

## Built (gate ruled the transcendental seam)

The gate approved the carve and ruled: Abramowitz-Stegun rational `erfc`, fixed-form deterministic `exp`,
deterministic `alpha` and cutoffs, self-validate to 1e-4, tin-foil declared, Ewald-first single lane. Built on
`claude/materials-oracle-modulus-slice` (`crates/physics/src/ewald.rs`):

- The three-term split (real-space screened Coulomb, reciprocal-space Gaussian, self-energy), fixed-point and
  deterministic. `erfc` is the A-S 7.1.26 five-term rational over the crate's deterministic `Fixed::exp`; the
  structure factor uses the crate's CORDIC `sin_cos`; all transcendentals are fixed-form with no
  input-dependent trip count. `alpha = 3.2 / V^(1/3)` and the fixed shell half-widths (`N_REAL = 2`,
  `N_RECIP = 6`) are the deterministic convergence parameters. The A-S coefficients and `alpha` constant are
  exact `from_ratio` rationals (no `from_decimal_str`, so no constructor-gate site).
- Tin-foil boundary declared in the module doc, the polar surface term set to zero.
- Self-validated: NaCl reproduces `1.747563` against the reference `1.747565`, CsCl `1.762675` against
  `1.762675`, both inside the 1e-4 acceptance tolerance, the exact proof that the Madelung constant is the Ewald
  sum over the positions. Fluorite (CaF2) validates the non-1:1, mixed-charge case (the generality corundum's
  `A2B3` needs): the kernel handles arbitrary stoichiometry and lands a physical Madelung constant. `erfc`
  itself is checked against `erfc(0)=1`, `erfc(1)=0.157299`, `erfc(2)=0.004678`.
- Byte-neutral: nothing reads the kernel, the sim determinism and invariant pins are unmoved. 186 physics tests
  green, constructor / determinism / prose gates clean.

The Madelung convention is stated and consistent: `M = -(E_total / formula_units) * reference_distance` in
reduced units with the full ionic charges, referenced to the nearest cation-anion distance. Corundum's exact
1e-4 Madelung validation is the immediate follow-on (it needs the experimental rhombohedral cell and a cited
reference constant under this convention); the kernel's generality for it is already proven by the non-1:1
fluorite case. Next piece per the gate's order: the IE and EA columns and the derived `chi`/`eta`.

## The dependency order after the kernel (from the architecture, for context, not this slice)

Per the architecture, after the Ewald kernel: the IE and EA per-element columns `[M]` with the free hardness
`eta = (IE - EA)/2` and electronegativity `chi = (IE + EA)/2` derived `[D]` from them (an independent, purely
additive floor piece that could parallelize); then QEq (needs both the columns and Ewald); then the bond-valence
positions (`R = R0 - b ln s`, the `R0` pair table and universal `b`); then the energy assembly (Ewald on computed
charges plus Born-Mayer plus London plus Keating); then the modulus emission. The disposer RESOLUTION-LADDER rule
is routed to Agent A's provenance enforcement, not built here: I build the generators, the ladder gates which
tier may answer which question. No build of any piece beyond the Ewald kernel until the gate rules its carve.

## Slice complete at the honest [E] estimator grade (gate ruling #182)

The generator stack is built and the derive-first thesis was tested to a rigorous negative. The gate ruled the
slice COMPLETE at the honest `[E]` grade: do not fit, do not add the wrong-direction column.

- **The Ewald kernel** (`ewald.rs`): the Madelung constant from positions, validated to 1e-4 (NaCl, CsCl,
  fluorite), plus the periodic potential matrix `A_ij`. Dissolves the `A2B3` seam. `[D]`, exact.
- **The IE/EA columns and derived `chi`/`eta`** (`qeq.rs`): `[M]` measured, `[D]` derived, with the
  unbound-anion `chi = eta = IE/2` limit `[E]` for the elements whose anion is unbound.
- **The shielded SCC-DFTB QEq solve** (`qeq.rs`): `tau = (16/5)U` parameter-free from the hardness, the
  shielded periodic Coulomb, the linear solve. Stable, symmetric, neutral, bounded where the bare Ewald runs
  away. The honest `[E]` estimator-grade partial charge.
- **The ionic bulk modulus** (`lattice_modulus.rs`): the point-charge Born-Lande `B`, emitted `[E]` WITH the
  documented systematic-overestimate band for stiff ionic oxides (NaCl in-band at ~24.4 GPa, periclase flagged
  systematic-high at ~266 vs measured 165). A stated bias, not a hidden error.

**The proven negative:** the derive-first QEq partial charge does NOT dissolve finding 1 (the divalent-oxide
overestimate). Raw-Mulliken parameters over-ionize a stiff ionic (periclase Mg ~+2.08, above formal), the cited
SCF Slater exponent makes it worse (wrong direction), and the overestimate is MULTI-CAUSAL: even the correct
Bader charge Mg ~+1.7 reaches only ~192 GPa against 165. The charge is one lever of three.

**The three principled refinements** (all unbuilt, all no-fit, HELD for the owner's architecture ruling, which
the gate is raising since the build contradicts the generator architecture's premise that the partial charge
dissolves finding 1): the compute-once DFT/Bader charge (the amortized first-principles rung), the Born-Mayer
exponential repulsive form (versus the overstiffening Born-Lande power), and the Keating covalent term (the
named shear debt). A fitted `[C]` parameterization is not the accuracy path.

The slice is byte-neutral and dormant throughout; the whole materials-oracle foundation (Ewald generator,
charge-equilibration, the tiered modulus estimator) is on the branch, honest at its grade, with the accuracy
path flagged rather than fitted.
