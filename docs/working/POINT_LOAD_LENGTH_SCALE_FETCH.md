# The point-load length scale: which flexural parameter the axisymmetric kei Green's function is defined with

This document settles one physics convention behind `crates/physics/src/flexure.rs`. The function `point_load_deflection` computes the axisymmetric point-load plate deflection as `w(r) = q0 alpha^2/(2 pi D) kei(r/alpha)`, feeding the Kelvin function `kei` the argument `r/alpha` with `alpha = (4 D/(delta_rho g))^(1/4)`, the value `flexural_parameter` returns. The suspicion raised by the axisymmetric-seamount build is that the axisymmetric point-load Green's function is defined with the natural length `l = (D/(delta_rho g))^(1/4) = alpha/sqrt(2)`, not `alpha`, so that the code feeds `kei` a length scale too large by `sqrt(2)` and uses an amplitude too large by `2x`.

This is a verification record only. No code is changed. The one-line fix is stated at the end for a later slice to apply.

The question is settled three independent ways, and all three agree: by the plate ODE itself (decisive, needs no source), by the primary (McNutt and Menard 1982 equation A8, read dual-channel and hash-verified, already in this repo's record), and by the reproduction the code cites by name (the TAFI toolbox, Jha et al. 2017, fetched fresh and read dual-channel). The convention is `l = (D/(delta_rho g))^(1/4)`, and `flexure::point_load_deflection` errs by exactly `sqrt(2)` in length and `2x` in amplitude.

---

## 1. The decisive independent check: the plate ODE, ground truth, no source needed

The axisymmetric thin-plate equation over a fluid substrate, away from the load, is

`D (w'''' + (2/r) w''' - (1/r^2) w'' + (1/r^3) w') + delta_rho g w = 0`,

which is `D grad^4 w + delta_rho g w = 0` with the radial biharmonic written out. Substitute `w(r) = kei(r/L)` and ask which `L` makes it a solution.

The zeroth-order Kelvin functions satisfy the radial Laplacian identities `grad^2 kei = ker` and `grad^2 ker = -kei` (with `grad^2 = d^2/dx^2 + (1/x) d/dx`), so `grad^4 kei(x) = grad^2(grad^2 kei) = grad^2 ker = -kei(x)`. Both identities were checked to 40 significant digits in mpmath (`grad^2 kei - ker` and `grad^2 ker + kei` are each below `1e-41` at `x = 0.7, 1.5, 3.0`), and `grad^4 kei(x) = -kei(x)` was checked directly through the radial biharmonic operator (agreement to `1e-43` at `x = 1, 2.5, 4`). This is the property the seamount curvature module already leans on, coded as `kei'' + kei'/x = ker` in `moment_equivalence.rs`.

For `w(r) = kei(r/L)` the chain rule gives `grad^4_r w = (1/L^4) grad^4_x kei(r/L) = -(1/L^4) kei(r/L) = -(1/L^4) w`. Substituted into the plate equation,

`D (-(1/L^4)) w + delta_rho g w = (delta_rho g - D/L^4) w = 0`,

which holds for all `r` exactly when `D/L^4 = delta_rho g`, that is

`L = (D/(delta_rho g))^(1/4)`.

So the axisymmetric point-load `kei(r/L)` is a solution of the plate equation for one length scale only, `L = (D/(delta_rho g))^(1/4)`. The code's `alpha = (4 D/(delta_rho g))^(1/4)` is `4^(1/4) = sqrt(2)` times that, so `alpha = sqrt(2) L`, and `kei(r/alpha)` is not a solution: with `L = alpha` the residual is `(delta_rho g - D/alpha^4) w = (delta_rho g - delta_rho g/4) w = (3/4) delta_rho g w`, nonzero. Feeding the line-load `alpha` to `kei` solves a plate with one quarter of the real restoring stiffness, not the world's plate.

The numerical confirmation, at 40 digits so that finite-difference roundoff cannot masquerade as signal, with `D = 7`, `delta_rho g = 0.03234` (illustrative, any positive pair behaves the same), `L = (D/(delta_rho g))^(1/4) = 3.835655`, `alpha = (4 D/(delta_rho g))^(1/4) = 5.424435`, and `alpha/L = 1.41421356 = sqrt(2)` exactly:

| r | residual with L=(D/dpg)^(1/4) | residual with L=alpha | (3/4) dpg kei(r/alpha) |
| --- | --- | --- | --- |
| 1.00 | -9.3e-42 | -0.0184711 | -0.0184711 |
| 2.50 | -7.2e-43 | -0.0166017 | -0.0166017 |
| 4.00 | -5.4e-43 | -0.0142817 | -0.0142817 |
| 6.00 | +1.8e-43 | -0.0111095 | -0.0111095 |

The `L = (D/(delta_rho g))^(1/4)` column is machine-zero: `kei(r/L)` solves the plate equation. The `L = alpha` column is nonzero and reproduces the analytic prediction `(3/4) delta_rho g kei(r/alpha)` to every printed digit: `kei(r/alpha)` does not.

### The amplitude, from a force balance independent of the deflection formula

The amplitude is pinned without assuming any coefficient, by the whole-plate force balance. Integrating the plate equation over the infinite plate, the biharmonic term is a boundary flux that vanishes at infinity, so `delta_rho g times (integral of w over area) = P`, the applied point load. With `w(r) = C kei(r/L)`,

`delta_rho g C 2 pi L^2 (integral_0^inf u kei(u) du) = P`.

The standard integral `integral_0^inf u kei(u) du = -1` was checked in scipy (`quad` returns `-1.00000000`, estimated error `1.4e-8`). With `delta_rho g = D/L^4` this gives `C = -P L^2/(2 pi D)`. So the force-balanced Green's function is

`w(r) = -(P L^2/(2 pi D)) kei(r/L)`, `L = (D/(delta_rho g))^(1/4)`.

The coefficient is `P L^2/(2 pi D)`. In terms of the code's `alpha`, `L^2 = alpha^2/2`, so this is `P alpha^2/(4 pi D)`, half of the code's `P alpha^2/(2 pi D)`. The central deflection magnitude is `L^2/(8 D) = alpha^2/(16 D)`, half of the code's `alpha^2/(8 D)`. So the amplitude the code uses is `2x` too large, and this is a consequence of the same length-scale substitution: the coefficient carries `alpha^2 = 2 L^2`.

Ground truth, then: the axisymmetric point-load Green's function is `w(r) = -(P/(2 pi D)) L^2 kei(r/L)` with `L = (D/(delta_rho g))^(1/4)`. The code's length scale is `sqrt(2)` too large and its amplitude `2x` too large.

---

## 2. The primary and the reproduction, and what each defines the length scale as

### 2.1 The primary: McNutt and Menard 1982 equation A8, dual-channel and hash-verified

The axisymmetric point-load solution the code cites (Brotchie and Silvester 1969) is printed verbatim by McNutt and Menard 1982, whose Appendix A derives it and attributes the cylindrical lineage to Brotchie and Silvester. That paper was read dual-channel at 500 dpi for `docs/working/SEAMOUNT_TREATMENT_FETCH.md`, from a PDF whose SHA256 (`f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`) matches the hash on record in `TE_CONSTRUCTION_FETCH.md` byte for byte. Its printed page 387 states:

> "The deflection of a thin elastic plate beneath a point load is `w(r) = -(P l^2/2 pi D) kei r/l` (A8) in which `P` is the weight of the load, `l = 4throot(D/dp g)`, and `kei` is a modified Bessel function."

So the primary's axisymmetric point-load length scale is `l = (D/(delta_rho g))^(1/4)`, and the amplitude coefficient is `P l^2/(2 pi D)`. This is exactly the force-balanced form the ODE gives above. In the SAME appendix, printed page 385, the two-dimensional (line) case, which McNutt and Menard call the "rectangular" load, uses a DIFFERENT length scale: "resting on an elastic plate with flexural parameter `a = 4throot(4D/dp g)`". So the primary distinguishes the two geometries by symbol and by definition: the axisymmetric point load uses `l = (D/(delta_rho g))^(1/4)`, the line/rectangular load uses `a = (4D/(delta_rho g))^(1/4)`. The factor of 4 belongs to the line case only.

### 2.2 The reproduction the code cites: TAFI (Jha et al. 2017), read fresh, dual-channel

The code and `PIPELINE_FETCHES.md` section 1 pin the point-load form to "TAFI eq. 6". TAFI is the Toolbox for Analysis of Flexural Isostasy (Jha, S., Harry, D. L. and Schutt, D. L., 2017, "Toolbox for Analysis of Flexural Isostasy (TAFI): A MATLAB toolbox for modeling flexural deformation of the lithosphere", Geosphere 13(5), 1555-1565, DOI 10.1130/GES01421.1), a MATLAB-toolbox paper, so a reproduction rather than a primary. It was fetched fresh from `https://derekschutt.wordpress.com/wp-content/uploads/2018/01/flexural_modeling_geosphere_2017.pdf` (SHA256 `4abc31ee08633de676313a600f4d4bf1a34b70ce131dd20dee2d1a078bc8f009`, a clean InDesign PDF, 11 pages), read by text extraction and by a visual render of printed page 1557 at 220 dpi with the point-load flexural-parameter cell re-rendered at 450 dpi.

Its equations, printed page 1557, read visually:

> `w(x) = Q0 (alpha^3/8D) e^(-x/alpha) (cos(x/alpha) + sin(x/alpha))` infinite plate, line load; (4)
> `w(r) = Q0 (alpha^2/2 pi D) kei(r/alpha)` infinite plate, point load, (6)
> "where `alpha` is the flexural parameter, which depends on the flexural rigidity and density structure of the plate (Table 1)".

Read at face value with a single `alpha`, equation (6) is the code's exact form. The trap is that TAFI's `alpha` is not one quantity: the paper defers the definition to Table 1, and Table 1 gives each load model its OWN flexural parameter under the one symbol. Read visually, the "Flexural parameter (`alpha`)" column of Table 1 is:

- Elastic half-space, 2-D line load (cited to Turcotte and Schubert 2002): `alpha = [4D/((rho_m - rho_i) g)]^(1/4)`.
- Elastic continuous plate, 2-D line load (cited to Turcotte and Schubert 2002): `alpha = [4D/((rho_m - rho_i) g)]^(1/4)`.
- Elastic continuous plate, point load (cited to Brotchie and Silvester 1969): `alpha = [D/((rho_m - rho_i) g + E Te/R^2)]^(1/4)`.
- Elastic continuous plate, 2-D sinusoidal load (cited to Turcotte and Schubert 2002): `alpha = [D/(rho_i g)]^(1/4)`.

The point-load flexural parameter, re-rendered at 450 dpi to be sure of the exponent and the denominator, carries NO factor of 4 and reads `[D/((rho_m - rho_i) + E Te/R^2)]^(1/4)` on the page. The second denominator term `E Te/R^2` is Brotchie and Silvester's spherical-shell membrane stiffness (`R` is Earth's radius), which vanishes for a flat plate; the `g` on the buoyancy term is dropped in TAFI's printed cell, which is a dimensional slip in TAFI (the line-load cell directly above carries the `g`, and the term must carry it), not a change to the structure. In the flat-plate limit the code models (`R -> infinity`), the point-load flexural parameter reduces to `[D/((rho_m - rho_i) g)]^(1/4) = (D/(delta_rho g))^(1/4) = l`, exactly the primary's `l` and the ODE's `L`.

So TAFI equation (6) is correct: its `kei` argument `r/alpha` and coefficient `alpha^2/(2 pi D)` are evaluated with the POINT-LOAD `alpha = (D/(delta_rho g))^(1/4)` from Table 1's point-load row, not the line-load `alpha = (4D/(delta_rho g))^(1/4)`. TAFI overloads the one symbol across four load models, disambiguated only by Table 1's row.

### 2.3 The three answers to the posed questions

1. The primary defines the axisymmetric (point) load flexural length as `l = (D/(delta_rho g))^(1/4)` (McNutt and Menard 1982 eq. A8, page 387; TAFI Table 1 point-load row, page 1557, flat-plate limit). It is not `(4D/(delta_rho g))^(1/4)`.
2. The point-load Green's function as the sources print it is `w(r) = -(P l^2/(2 pi D)) kei(r/l)` (McNutt and Menard A8) and `w(r) = Q0 (alpha^2/(2 pi D)) kei(r/alpha)` with `alpha` the point-load `l` (TAFI eq. 6). The amplitude coefficient is `l^2/(2 pi D)` and the `kei` argument's length scale is `l = (D/(delta_rho g))^(1/4)`, both from the source.
3. The `alpha = (4D/(delta_rho g))^(1/4)` form is the line-load parameter specifically. Turcotte and Schubert equation 3-127 defines it for the two-dimensional (line) load, McNutt and Menard use it for their "rectangular" load only, and TAFI's Table 1 uses it for the two line-load rows only. It is not the axisymmetric length scale, confirmed against three sources rather than from memory.

---

## 3. Where the code's error entered, and the corroboration already inside the codebase

The defect is a transcription seam, and auditing the input finds it. `PIPELINE_FETCHES.md` section 1 records the point-load form as `w(r) = Q0 (alpha^2/(2 pi D)) kei(r/alpha)` "where `alpha` is the same flexural parameter `[4D/((rho_m - rho_w) g)]^(1/4)`", and states that this `alpha` is "used as the flexural parameter for both the line-load and the point-load solutions in the TAFI toolbox (Table 1)". That sentence is the seam: TAFI's Table 1 does not use one `alpha` for both. It gives the point load its own flexural parameter with no factor of 4. The fetch note took TAFI equation (6)'s form (correct) and welded it to Turcotte and Schubert's line-load `alpha` (equation 3-127, correct for the line load), and the two `alpha` symbols are not the same length scale. `flexure.rs` then implemented the welded form, and `point_load_deflection` feeds the line-load `alpha` into the axisymmetric `kei`.

The codebase already carries the correct convention one module over. `moment_equivalence.rs` documents the point-load deflection as `w(r) = -(P l^2/(2 pi D)) kei(r/l)` with `l = (D/(delta_rho g))^(1/4)`, citing McNutt and Menard eq. A8 (its `kelvin_ker` doc and `point_load_first_zero_crossing`, `point_load_curvature_at_first_zero_crossing`). That module is not affected by the bug, and the reason is instructive: its output is a curvature read at the dimensionless nodal ring `x_0 = r/l = 3.91467`, and it never converts `x_0` back to a physical radius with a length scale, so the `l`-versus-`alpha` choice cancels ("the flexural length `l` cancels between the deflection's `l^2` amplitude and the `1/l^2` of the second derivative", its own words). It reads `ker(x_0)` and `kei'(x_0)` at the cited dimensionless crossing and never calls `point_load_deflection`. So the seamount moment-equivalence path is correct, and the deflection kernel is the sole site of the error. The two documents disagree on the length scale, and the ODE and both sources side with `moment_equivalence.rs`.

---

## 4. Verdict and the exact fix

`flexure::point_load_deflection` errs. For the axisymmetric point load it feeds `kei` a length scale `alpha = (4D/(delta_rho g))^(1/4)` that is `sqrt(2)` larger than the correct `l = (D/(delta_rho g))^(1/4)`, and it uses an amplitude coefficient `alpha^2/(2 pi D)` that is `2x` larger than the correct `l^2/(2 pi D)`. The observable consequences: the modelled moat around a point (volcanic) load is `sqrt(2)` too wide (its first zero crossing sits at `r = 3.91467 alpha = 3.91467 sqrt(2) l` rather than `3.91467 l`), and its central depression is `2x` too deep. The line-load function `line_load_deflection` is correct: `alpha = (4D/(delta_rho g))^(1/4)` is the right length for the one-dimensional ODE (the factor of 4 comes from the `1/sqrt(2)` in that ODE's characteristic roots), and the module's own line-load tests and force-balance integral confirm it. The bug is confined to the axisymmetric path.

The error is currently dormant. The flexure kernel has no run-path consumer yet (the module says so, and both run pins hold trivially), and `moment_equivalence.rs` does not call `point_load_deflection`. So no shipped result is wrong today; the latent defect would corrupt any future consumer of the point-load deflection field, `deflection_at` for a `LoadKind::Point`.

One test masks it. `the_point_load_central_deflection_is_the_analytic_magnitude` checks `w(0)` against `-500 alpha^2/(8 D)`, which is the code's own algebra restated in f64, so it validates the implementation against itself rather than against the physics; it passes with the wrong length scale. `the_point_load_kei_matches_reference_values` checks `kei` against tabulated values and is correct: the Kelvin function itself is right, only the argument and amplitude fed to it are wrong.

The exact fix, two equivalent framings.

The minimal numeric change inside `point_load_deflection`: convert the incoming line-load `alpha` to the axisymmetric length `l = alpha/sqrt(2)` and use `l` for both the coefficient and the argument. Because `l^2 = alpha^2/2` exactly, this is: replace the coefficient `alpha^2/(2 pi D)` with `(alpha^2/2)/(2 pi D) = alpha^2/(4 pi D)` (halving it), and replace the `kei` argument `r/alpha` with `r/l = r sqrt(2)/alpha` (dividing by `l = sqrt(alpha^2/2)`, which `Fixed::sqrt` forms exactly). In the current code, `let a2 = alpha.checked_mul(alpha)?;` becomes the axisymmetric `let l2 = alpha.checked_mul(alpha)?.checked_div(Fixed::from_int(2))?;` used in the coefficient in place of `a2`, and `let arg = r.checked_div(alpha)?;` becomes `let arg = r.checked_div(l2.sqrt())?;`.

The contract change that stops the line-load `alpha` from reaching the axisymmetric function by habit, which is the deeper fix: the two Green's functions need different length scales and must not share one `alpha` parameter. Add an axisymmetric constructor `flexural_length_axisymmetric(d, delta_rho, g) = (D/(delta_rho g))^(1/4)` beside `flexural_parameter` (it is the same computation without the `2 sqrt` step: `flexural_parameter` already forms `ratio = D/(delta_rho g)` internally and then does `sqrt(2 sqrt(ratio))`; the axisymmetric length is `sqrt(sqrt(ratio))`, the quarter power with no factor of 4), have `point_load_deflection` take that `l` rather than `alpha`, and have `deflection_at` compute both scales, `alpha` for `LineY` loads and `l` for `Point` loads. Then no caller can pass the line-load parameter to the point-load function, because the point-load function no longer accepts it. This matches TAFI's Table 1 (a distinct point-load flexural parameter) and McNutt and Menard's distinct `l` and `a`.

---

## Sources and channels

Primary, axisymmetric point load: McNutt, M. K. and Menard, H. W., 1982, "Constraints on yield strength in the oceanic lithosphere derived from observations of flexure", Geophysical Journal of the Royal Astronomical Society 71, 363-394, equation A8 and the flexural-parameter definitions on printed pages 385 and 387. Read dual-channel at 500 dpi in `docs/working/SEAMOUNT_TREATMENT_FETCH.md`; PDF SHA256 `f085ec2b73aff489372c75df899789ddea4eccb61259789f8f762e1cdda27f1f`, matching the hash on record. The axisymmetric lineage is McNutt and Menard's own attribution to Brotchie, J. F. and Silvester, R., 1969, "On crustal flexure", Journal of Geophysical Research 74(22), 5240-5252, DOI 10.1029/JB074i022p05240 (the source the code cites; its point-load Kelvin solution is reproduced by both McNutt and Menard and TAFI, and was not fetched directly here because two reproductions and the ODE already fix the convention).

Reproduction, the source the code cites by name: Jha, S., Harry, D. L. and Schutt, D. L., 2017, "Toolbox for Analysis of Flexural Isostasy (TAFI): A MATLAB toolbox for modeling flexural deformation of the lithosphere", Geosphere 13(5), 1555-1565, DOI 10.1130/GES01421.1, equations 4 and 6 and Table 1 on printed page 1557. Fetched fresh from `https://derekschutt.wordpress.com/wp-content/uploads/2018/01/flexural_modeling_geosphere_2017.pdf`, SHA256 `4abc31ee08633de676313a600f4d4bf1a34b70ce131dd20dee2d1a078bc8f009`, a clean InDesign PDF; read by text extraction and by visual render at 220 dpi, the point-load flexural-parameter cell re-rendered at 450 dpi to confirm the exponent and denominator. Labelled a reproduction, not a primary.

Line-load parameter: Turcotte, D. L. and Schubert, G., Geodynamics, equation 3-127, `alpha = (4D/(delta_rho g))^(1/4)`, cited by TAFI Table 1's line-load and sinusoidal rows and by `PIPELINE_FETCHES.md` section 1.

Independent check: the plate ODE derivation of section 1, verified in mpmath at 40 digits (the Kelvin Laplacian identities, `grad^4 kei = -kei`, and the plate residual for `L = (D/(delta_rho g))^(1/4)` versus `L = alpha`) and in scipy (`integral_0^inf u kei(u) du = -1`). This is the ground truth, reproducible with no source access.

Confidence: HIGH on the length scale `l = (D/(delta_rho g))^(1/4)`, established three independent ways that agree exactly. HIGH on the `sqrt(2)` length and `2x` amplitude error in `point_load_deflection`. HIGH that the line-load path and the `moment_equivalence.rs` curvature path are correct and unaffected. HIGH that the seam entered through `PIPELINE_FETCHES.md` section 1 reading TAFI's Table 1 as one `alpha` for both loads. The `g` dropped from TAFI's printed point-load cell is a dimensional slip in TAFI, noted and stepped around, not load-bearing on the verdict.
