# The LoadKind taxonomy audit (codex gpt-5.6-sol, maximum effort, 2026-07-20)

Captured from the audit task log by the gate: the auditor completed its analysis but could not write
this file itself, because its workspace was mounted read-only and the write was rejected. No source was
modified by it. Findings are its own, verbatim below; the file:line citations are its own and have not
all been re-verified here.

I completed the audit, but could not create `docs/working/LOADKIND_TAXONOMY_AUDIT.md`: the workspace is mounted read-only and the write was rejected. No files were modified, and the worktree is clean.

## Verdict

`LoadKind` is a Principle 11 defect in its present role.

The strongest argument for closure is sound: line, point, and strip responses are mathematical Green-function mechanisms for the fixed plate equation ([flexure.rs:24](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:24), [flexure.rs:68](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:68)). A closed internal enum selecting compiled kernels is legitimate.

The counterargument prevails because the public enum also defines every load geometry a world may contain ([flexure.rs:1208](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1208)). Adding a disc, finite segment, ellipse, polygon, or varying footprint requires editing Rust. The repository already treats load-class membership as data because a closed enum there would be a templating defect ([moment_equivalence.rs:252](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:252)).

The proper division is:

- Keep fixed numerical kernels in Rust.
- Put load-profile membership, geometry, orientation, pressure profile, dimensional metadata, provenance, load class, timescale, and approximation bands in a deterministic registry.
- Retain line and strip kernels as analytic accelerators for rows that prove the required invariance.

Closure would become correct if `LoadKind` became a private solver-strategy enum behind such a registry, or if the public input accepted arbitrary fields and these variants were internal accelerators.

## Spanning claim

The point Green function spans arbitrary two-dimensional fields only as a continuum convolution. The API accepts a finite `&[Load]` and provides no area integrator, mesh, radial quadrature, two-dimensional transform, or error estimator ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243)).

Current exact representation is limited to:

- Finite collections of point deltas.
- Infinite lines parallel to y.
- Infinite, uniform strips parallel to y.
- Finite piecewise-constant y-invariant fields assembled from strips.

Representable only through potentially expensive approximation:

- Disc or radial load: point quadrature over area.
- Finite line: point quadrature along its length.
- Ellipse or polygon: area tessellation.
- Varying footprint: weighted mesh or spectral expansion.
- Arbitrarily oriented lines: external coordinate rotation; `Load` carries no orientation.

No convergence criterion or provenance-backed quadrature tolerance exists.

## Disc-load answer

Yes, the axisymmetric delta singularity remains unfixed.

For Point, the origin amplitude scales as

`Q l² / D`, with `l = (D/R)^(1/4)`,

so it scales as `Q / sqrt(DR)` and diverges as `D^(-1/2)` ([flexure.rs:786](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:786)). A bounded finite-disc pressure should approach the finite Airy response `q/R`, just as the strip does ([flexure.rs:736](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:736)).

This matters because Point is documented for volcanic constructs and crater basins, both finite-area objects ([flexure.rs:1213](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1213), [flexural_relief.rs:140](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:140)). The upstream point-to-disc equivalence band applies only beyond the first nodal ring and does not validate central deflection or the `D -> 0` limit ([moment_equivalence.rs:1686](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1686)).

There is also a point sign defect: the implementation omits the leading minus required before `kei`, so positive Point loads deflect opposite to positive line and strip loads ([flexure.rs:813](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:813), [moment_equivalence.rs:1733](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1733)).

## Admissibility provenance

All four audited bounds are absent from the floor manifest, provenance sidecar, and reserved-value register. The ledger says an untagged value fails the floor ([PROVENANCE_LEDGER.md:3](/home/nathan/Deep-Emergent-Civ-Simulator/docs/PROVENANCE_LEDGER.md:3)).

| Bound | Status |
|---|---|
| `MAX_YOUNGS_MODULUS_GPA = 512` | Authored, uncited, untagged. Described as “generous” and a convenient power of two ([flexure.rs:163](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:163)). It conflicts with the data floor, which admits 1,200 GPa ([mechanical_floor.toml:270](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/data/mechanical_floor.toml:270)). |
| `MAX_ELASTIC_THICKNESS_KM = 800` | Authored requested-range envelope, uncited and untagged ([flexure.rs:183](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:183)). |
| `MAX_LINE_LOAD_GPA_KM = 500` | Authored requested-range ceiling, uncited and untagged ([flexure.rs:190](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:190)). |
| `MAX_POINT_LOAD_GPA_KM2 = 67,108,864` | Derived from authored premises, then given discretionary headroom and power-of-two rounding. It has no earned `[D]` chain and remains untagged ([flexure.rs:203](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:203)). |

If no empirical or derivational basis can be supplied, these values should be reserved with an explicit basis rather than assigned fabricated provenance.

## Buckingham count

For the declared flat, static, constant-rigidity equation:

| Load | Required groups | Current result |
|---|---:|---|
| Point delta | 2 | Complete for a true delta: `r/l`, normalized response |
| Infinite line | 2 | Complete for the stated infinite line |
| Uniform infinite strip | 3 | Complete: `wR/q`, `x/alpha`, `a/alpha` |
| Uniform disc | 3 | Missing `a/l` when collapsed to Point |
| Finite line | 4 | Missing `y/l` and `L/l`; orientation adds an angle |
| Axis-aligned ellipse | 5 | Missing both footprint scales and two-dimensional position; orientation adds an angle |

Further validity groups include `w/Te`, a thickness-to-load or response scale, and `l/R_body`. `FlexedPlate` stores neither thickness nor body radius, so it cannot preflight them ([flexural_relief.rs:46](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:46)).

## Gap and Residual Laws

No target function returns or dispatches on a `Delta`. Exact caller-selected mathematical support does not require candidate ranking. The defects arise where Point substitutes for a disc, scalar rigidity substitutes for an upstream band, and Kelvin tails are silently set to zero above `x = 12` ([flexure.rs:842](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:842)).

Residual-law status:

- Conservation: line has a whole-domain balance test; strip and point do not. Point has the sign defect and truncated tail.
- Disequilibrium: the model assumes static, instantaneous inviscid support. `Load` carries no age or history, and `FlexedPlate` drops the timescale-bearing chord.
- Fluctuation-dissipation: vacuous for the current static elastic model. It becomes mandatory if viscous or viscoelastic relaxation is added.
- Dimensional pre-count: implemented ideal kernels have the right counts, but disc, finite footprints, spherical-shell validity, and small-deflection validity do not.

## Alien feasibility

The core equation is material-neutral and its scaled arithmetic includes a Europa-like restoring pair ([flexure.rs:396](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:396)). End-to-end ice support still fails for the real ice row because required creep data are absent ([moment_equivalence.rs:6679](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:6679)). The authored 512 GPa cap and absence of body radius also prevent a clean alien-feasibility pass.

## Determinism

Successful sums are order-independent to the bit because contributions are accumulated as raw fixed-point values in `i128` ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243), [flexural_relief.rs:166](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:166)). Tests cover mixed line, point, and strip ordering ([flexural_relief.rs:620](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:620)).

Exceptions:

- `ReliefRefusal` returns the first invalid load, so failure identity can vary by order.
- Line and point admissibility can panic on `Fixed::MIN`.
- Algebraically regrouping sources can differ because each contribution is rounded before summation.

## Defects found

1. `LoadKind` conflates kernel mechanism with world membership ([flexure.rs:1208](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1208)).
2. Finite `Load` lists do not span arbitrary continuous fields ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243)).
3. Point-as-disc deletes radius and leaves a `D^(-1/2)` singularity ([flexure.rs:786](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:786)).
4. Positive Point load has the wrong sign ([flexure.rs:813](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:813)).
5. One rigidity is applied to loads whose thickness is defined per load class and timescale ([flexure.rs:36](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:36), [flexure.rs:1172](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1172)).
6. `FlexedPlate` drops `LoadChord` and cannot propagate `RigidityBand` ([flexural_relief.rs:78](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:78)).
7. All four admissibility bounds are untagged ([flexure.rs:163](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:163)).
8. The 512 GPa cap contradicts the floor’s 1,200 GPa range ([mechanical_floor.toml:270](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/data/mechanical_floor.toml:270)).
9. The symmetric absolute Poisson-ratio bound is not physically justified by the cited incompressible upper limit ([flexure.rs:173](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:173)).
10. Line and point admissibility panic on `Fixed::MIN` ([flexure.rs:225](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:225), [fixed.rs:207](/home/nathan/Deep-Emergent-Civ-Simulator/crates/core/src/fixed.rs:207)).
11. Kelvin tails are silently zeroed without amplitude-aware bounds ([flexure.rs:842](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:842)).
12. Strip and point lack whole-domain conservation residuals ([flexure.rs:736](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:736)).
13. Static equilibrium lacks a disequilibrium precondition or relaxation state ([flexure.rs:129](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:129)).
14. Small-deflection, thin-plate, and spherical-shell validity groups cannot be checked ([flexural_relief.rs:46](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:46)).
15. `ReliefRefusal` is order-sensitive for multiple invalid loads ([flexural_relief.rs:166](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:166)).
16. `Load.magnitude` changes physical units by variant while remaining a bare `Fixed` ([flexure.rs:1226](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1226)).
17. Geodynamics maps a finite province to an infinite-y strip and has a load-sign description inconsistent with `FlexedPlate` ([geodynamics.rs:105](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/geodynamics.rs:105), [geodynamics.rs:358](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/geodynamics.rs:358)).
18. Real ice lacks required creep data, while spherical-shell support cannot be checked ([moment_equivalence.rs:6679](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:6679)).

I completed the audit, but could not create `docs/working/LOADKIND_TAXONOMY_AUDIT.md`: the workspace is mounted read-only and the write was rejected. No files were modified, and the worktree is clean.

## Verdict

`LoadKind` is a Principle 11 defect in its present role.

The strongest argument for closure is sound: line, point, and strip responses are mathematical Green-function mechanisms for the fixed plate equation ([flexure.rs:24](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:24), [flexure.rs:68](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:68)). A closed internal enum selecting compiled kernels is legitimate.

The counterargument prevails because the public enum also defines every load geometry a world may contain ([flexure.rs:1208](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1208)). Adding a disc, finite segment, ellipse, polygon, or varying footprint requires editing Rust. The repository already treats load-class membership as data because a closed enum there would be a templating defect ([moment_equivalence.rs:252](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:252)).

The proper division is:

- Keep fixed numerical kernels in Rust.
- Put load-profile membership, geometry, orientation, pressure profile, dimensional metadata, provenance, load class, timescale, and approximation bands in a deterministic registry.
- Retain line and strip kernels as analytic accelerators for rows that prove the required invariance.

Closure would become correct if `LoadKind` became a private solver-strategy enum behind such a registry, or if the public input accepted arbitrary fields and these variants were internal accelerators.

## Spanning claim

The point Green function spans arbitrary two-dimensional fields only as a continuum convolution. The API accepts a finite `&[Load]` and provides no area integrator, mesh, radial quadrature, two-dimensional transform, or error estimator ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243)).

Current exact representation is limited to:

- Finite collections of point deltas.
- Infinite lines parallel to y.
- Infinite, uniform strips parallel to y.
- Finite piecewise-constant y-invariant fields assembled from strips.

Representable only through potentially expensive approximation:

- Disc or radial load: point quadrature over area.
- Finite line: point quadrature along its length.
- Ellipse or polygon: area tessellation.
- Varying footprint: weighted mesh or spectral expansion.
- Arbitrarily oriented lines: external coordinate rotation; `Load` carries no orientation.

No convergence criterion or provenance-backed quadrature tolerance exists.

## Disc-load answer

Yes, the axisymmetric delta singularity remains unfixed.

For Point, the origin amplitude scales as

`Q l² / D`, with `l = (D/R)^(1/4)`,

so it scales as `Q / sqrt(DR)` and diverges as `D^(-1/2)` ([flexure.rs:786](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:786)). A bounded finite-disc pressure should approach the finite Airy response `q/R`, just as the strip does ([flexure.rs:736](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:736)).

This matters because Point is documented for volcanic constructs and crater basins, both finite-area objects ([flexure.rs:1213](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1213), [flexural_relief.rs:140](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:140)). The upstream point-to-disc equivalence band applies only beyond the first nodal ring and does not validate central deflection or the `D -> 0` limit ([moment_equivalence.rs:1686](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1686)).

There is also a point sign defect: the implementation omits the leading minus required before `kei`, so positive Point loads deflect opposite to positive line and strip loads ([flexure.rs:813](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:813), [moment_equivalence.rs:1733](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:1733)).

## Admissibility provenance

All four audited bounds are absent from the floor manifest, provenance sidecar, and reserved-value register. The ledger says an untagged value fails the floor ([PROVENANCE_LEDGER.md:3](/home/nathan/Deep-Emergent-Civ-Simulator/docs/PROVENANCE_LEDGER.md:3)).

| Bound | Status |
|---|---|
| `MAX_YOUNGS_MODULUS_GPA = 512` | Authored, uncited, untagged. Described as “generous” and a convenient power of two ([flexure.rs:163](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:163)). It conflicts with the data floor, which admits 1,200 GPa ([mechanical_floor.toml:270](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/data/mechanical_floor.toml:270)). |
| `MAX_ELASTIC_THICKNESS_KM = 800` | Authored requested-range envelope, uncited and untagged ([flexure.rs:183](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:183)). |
| `MAX_LINE_LOAD_GPA_KM = 500` | Authored requested-range ceiling, uncited and untagged ([flexure.rs:190](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:190)). |
| `MAX_POINT_LOAD_GPA_KM2 = 67,108,864` | Derived from authored premises, then given discretionary headroom and power-of-two rounding. It has no earned `[D]` chain and remains untagged ([flexure.rs:203](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:203)). |

If no empirical or derivational basis can be supplied, these values should be reserved with an explicit basis rather than assigned fabricated provenance.

## Buckingham count

For the declared flat, static, constant-rigidity equation:

| Load | Required groups | Current result |
|---|---:|---|
| Point delta | 2 | Complete for a true delta: `r/l`, normalized response |
| Infinite line | 2 | Complete for the stated infinite line |
| Uniform infinite strip | 3 | Complete: `wR/q`, `x/alpha`, `a/alpha` |
| Uniform disc | 3 | Missing `a/l` when collapsed to Point |
| Finite line | 4 | Missing `y/l` and `L/l`; orientation adds an angle |
| Axis-aligned ellipse | 5 | Missing both footprint scales and two-dimensional position; orientation adds an angle |

Further validity groups include `w/Te`, a thickness-to-load or response scale, and `l/R_body`. `FlexedPlate` stores neither thickness nor body radius, so it cannot preflight them ([flexural_relief.rs:46](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:46)).

## Gap and Residual Laws

No target function returns or dispatches on a `Delta`. Exact caller-selected mathematical support does not require candidate ranking. The defects arise where Point substitutes for a disc, scalar rigidity substitutes for an upstream band, and Kelvin tails are silently set to zero above `x = 12` ([flexure.rs:842](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:842)).

Residual-law status:

- Conservation: line has a whole-domain balance test; strip and point do not. Point has the sign defect and truncated tail.
- Disequilibrium: the model assumes static, instantaneous inviscid support. `Load` carries no age or history, and `FlexedPlate` drops the timescale-bearing chord.
- Fluctuation-dissipation: vacuous for the current static elastic model. It becomes mandatory if viscous or viscoelastic relaxation is added.
- Dimensional pre-count: implemented ideal kernels have the right counts, but disc, finite footprints, spherical-shell validity, and small-deflection validity do not.

## Alien feasibility

The core equation is material-neutral and its scaled arithmetic includes a Europa-like restoring pair ([flexure.rs:396](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:396)). End-to-end ice support still fails for the real ice row because required creep data are absent ([moment_equivalence.rs:6679](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:6679)). The authored 512 GPa cap and absence of body radius also prevent a clean alien-feasibility pass.

## Determinism

Successful sums are order-independent to the bit because contributions are accumulated as raw fixed-point values in `i128` ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243), [flexural_relief.rs:166](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:166)). Tests cover mixed line, point, and strip ordering ([flexural_relief.rs:620](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:620)).

Exceptions:

- `ReliefRefusal` returns the first invalid load, so failure identity can vary by order.
- Line and point admissibility can panic on `Fixed::MIN`.
- Algebraically regrouping sources can differ because each contribution is rounded before summation.

## Defects found

1. `LoadKind` conflates kernel mechanism with world membership ([flexure.rs:1208](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1208)).
2. Finite `Load` lists do not span arbitrary continuous fields ([flexure.rs:1243](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1243)).
3. Point-as-disc deletes radius and leaves a `D^(-1/2)` singularity ([flexure.rs:786](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:786)).
4. Positive Point load has the wrong sign ([flexure.rs:813](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:813)).
5. One rigidity is applied to loads whose thickness is defined per load class and timescale ([flexure.rs:36](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:36), [flexure.rs:1172](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1172)).
6. `FlexedPlate` drops `LoadChord` and cannot propagate `RigidityBand` ([flexural_relief.rs:78](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:78)).
7. All four admissibility bounds are untagged ([flexure.rs:163](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:163)).
8. The 512 GPa cap contradicts the floor’s 1,200 GPa range ([mechanical_floor.toml:270](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/data/mechanical_floor.toml:270)).
9. The symmetric absolute Poisson-ratio bound is not physically justified by the cited incompressible upper limit ([flexure.rs:173](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:173)).
10. Line and point admissibility panic on `Fixed::MIN` ([flexure.rs:225](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:225), [fixed.rs:207](/home/nathan/Deep-Emergent-Civ-Simulator/crates/core/src/fixed.rs:207)).
11. Kelvin tails are silently zeroed without amplitude-aware bounds ([flexure.rs:842](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:842)).
12. Strip and point lack whole-domain conservation residuals ([flexure.rs:736](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:736)).
13. Static equilibrium lacks a disequilibrium precondition or relaxation state ([flexure.rs:129](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:129)).
14. Small-deflection, thin-plate, and spherical-shell validity groups cannot be checked ([flexural_relief.rs:46](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:46)).
15. `ReliefRefusal` is order-sensitive for multiple invalid loads ([flexural_relief.rs:166](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexural_relief.rs:166)).
16. `Load.magnitude` changes physical units by variant while remaining a bare `Fixed` ([flexure.rs:1226](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:1226)).
17. Geodynamics maps a finite province to an infinite-y strip and has a load-sign description inconsistent with `FlexedPlate` ([geodynamics.rs:105](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/geodynamics.rs:105), [geodynamics.rs:358](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/geodynamics.rs:358)).
18. Real ice lacks required creep data, while spherical-shell support cannot be checked ([moment_equivalence.rs:6679](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:6679)).
