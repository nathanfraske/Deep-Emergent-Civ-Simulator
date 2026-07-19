# Disk arc #206 derive-first audit and consumer-wire gate

Owner-directed audit, 2026-07-19. Design-first, grounded against current `main`, the merged history of PR #206, the disk and giant-gate code now present, and primary-source cross-checks of the reported truncation formula. This document changes no code, sets no value, registers no source as held, and does not authorize a realization re-pin.

## Executive verdict

The dormant work is directionally strong. The source-fetch discipline, the refusal to fabricate a Kraft metallicity slope, the retention of `alpha_viscosity` as a real transport input, and the boundary hold on the run-path wire are all correct decisions.

The run-path activation is **not yet approved**.

Two findings are blocking:

1. The reported binary-truncation expression appears to combine coefficients from two different models. The reported expression is

   $$
   f = 0.733(1-e)^{1.20}q^{0.01}.
   $$

   The Pichardo invariant-loop fit uses a mass exponent of `0.07`, not `0.01`. The `0.01` exponent belongs to the separate Manara/Papaloizou-Pringle torque-fit term `h * mu^k`, with `h = 0.88` and `k = 0.01`. The exact committed code and its held primary must be checked before this radius can answer any consumer.

2. The reported nine-commit state is not visible in current GitHub state. PR #206 is merged and closed, its old head is no longer a live branch, and current `main` still carries the earlier disk-literature manifest header rather than the reported `34 -> 44` expansion. The audit therefore cannot verify the new code, source receipts, byte pins, or source count until an exact commit SHA is supplied on a fresh branch.

The immediate ruling is:

- preserve the dormant work;
- recover or push its exact head on a fresh branch from current `main`;
- correct or disprove the formula-mixing finding against the held primary;
- classify the Pichardo result as a low-viscosity geometric ceiling or estimator rung, not as the universal viscous gas-disk edge;
- close the mass and angular-momentum residual created by truncation;
- propagate all branch and model bands through `tau_disk`, `DiskGas`, and the giant verdict;
- only then request the consumer flag and re-pin.

## Evidence boundary

### What is verified on current `main`

The current tree contains:

- the stellar structure dispatch and its `Convective`, `Radiative`, and `NearDegenerate` Kraft verdicts;
- the pre-main-sequence and main-sequence activity machinery;
- the radiative EUV bracket machinery;
- a viscous similarity surface-density derivation that consumes accretion rate, angular frequency, `alpha_viscosity`, temperature, and mean molecular mass;
- a binarity cap whose earlier landed form used the Roche lobe as a conservative upper edge;
- a `DiskClockState` and banded giant-gate composition;
- a derived-clock consumer path that remains described in source as dormant or gated rather than an unconditional canonical run-path replacement.

The current disk-literature manifest at the audit base does not show the reported 44-entry state. This may mean the nine commits exist only in an unpushed worktree, a deleted branch, or another ref. The audit does not infer which.

### What is conditional on the agent report

The following claims are not verified until the exact head is pushed:

- nine additional commits;
- both byte pins at every commit;
- ten additional primary-source records;
- the exact Pichardo implementation;
- the corrected broadband attenuation path;
- the Kraft metallicity-sign field;
- the EUV departure grid;
- the mean-photon-energy and reprocessing retirements;
- the claimed source manifest increase from 34 to 44.

The first action is therefore a premise-line action, not a physics change:

```text
push exact head
-> name exact SHA
-> compare against current main
-> run source and receipt gates
-> then audit code and values
```

A prose summary cannot substitute for that state.

## Finding P0-1: the reported truncation equation is a model hybrid

### Pichardo invariant-loop fit

Pichardo, Sparke, and Aguilar model test-particle invariant loops in an eccentric binary potential. Their paper states that:

- the calculation is coplanar;
- it represents the low-viscosity gas limit;
- gas dynamics may impose further restrictions;
- the result is an upper bound on circumstellar disk size;
- the fit covers approximately `0 <= e <= 0.9` and `0.01 <= q <= 0.99`;
- the fit reproduces their calculated radii to about `+-6.5%`.

The widely reproduced circumstellar fit is

$$
R_i
=
R_{i,\mathrm{Egg}}
\left[
0.733(1-e)^{1.20}\mu^{0.07}
\right],
$$

where the mass-fraction symbol and each component's Eggleton ratio must be read exactly as the source defines them.

The load-bearing fingerprint is the `0.07` exponent.

### Manara torque-fit family

Manara et al. provide a different parameterization based on the Papaloizou-Pringle viscous-torque calculations:

$$
R_{\mathrm{trunc}}
=
R_{i,\mathrm{Egg}}
\left(
 b e^c + h\mu^k
\right).
$$

In that family:

- `h = 0.88`;
- `k = 0.01`;
- `b` and `c` vary with component, mass fraction, and Reynolds number;
- the Reynolds number depends on disk viscosity and aspect ratio, commonly through

  $$
  \mathcal{R}
  =
  \frac{1}{\alpha(H/r)^2}.
  $$

The load-bearing fingerprint is that `0.01` is the exponent `k` on the `h * mu^k` term. It is not the Pichardo mass exponent.

### Gate consequence

If the code contains `0.733`, `1.20`, and `0.01` in one Pichardo expression, it has crossed the two models.

That is a hard stop. The correction is not to change `0.01` to `0.07` on trust. The correction is:

1. read the held primary bytes;
2. identify the exact equation and symbol definitions;
3. add a source-fingerprint test;
4. use distinct types for the two model families;
5. preserve each model's validity and band.

A suitable shape is:

```rust
pub enum CircumstellarTruncationModel {
    PichardoInvariantLoop(PichardoModel),
    ViscousTorqueFit(ViscousTorqueModel),
}

pub struct BinaryMassFraction(pub Fixed); // mu = M2 / (M1 + M2)
pub struct ComponentMassRatio(pub Fixed); // qi = Mi / Mj

pub struct TruncationEvaluation {
    pub radius_au: Band<Fixed>,
    pub model: CircumstellarTruncationModelId,
    pub validity: ValidityFrame,
    pub provenance: ReceiptId,
}
```

The two mass-ratio concepts must not share one untyped `Fixed` argument.

## Finding P0-2: Pichardo is not the universal gas-disk truncation law

The Pichardo source is valuable, but its semantic altitude matters.

It answers:

> What is the outermost non-self-intersecting stable-loop region permitted by the binary potential in the coplanar, low-viscosity limit?

It does not fully answer:

> Where does a viscous, pressure-supported, possibly inclined gas disk settle after resonant torques balance internal transport?

The source itself distinguishes these questions. It contrasts its invariant-loop approach with resonant torque calculations that depend on viscosity, and it states that gas dynamics may further restrict the disk.

The Pichardo radius is therefore one of:

- a low-viscosity geometric ceiling;
- an estimator rung for a coplanar gas disk where that approximation is licensed;
- a validation target for a deeper torque or hydrodynamic solve.

It is not a replacement for the full `f(q,e,alpha,H/r,inclination,...)` problem.

This matters because the reported arc correctly retains `alpha_viscosity` as a floor. If the chosen truncation route does not consume `alpha_viscosity`, then the viscosity-dependent truncation frontier has not disappeared. The implementation has selected a low-viscosity model branch.

The recommended model ladder is:

1. **Viscous torque rung**: Manara/Papaloizou-Pringle style, conditioned on `mu`, `e`, Reynolds number, component, and the source table domain.
2. **Invariant-loop rung**: Pichardo low-viscosity ceiling, coplanar, with the source fit band.
3. **Roche-lobe ceiling**: pure geometric upper limit when no tighter rung resolves.
4. **Hydrodynamic or compute-once rung**: inclined, thick, self-gravitating, magnetized, or otherwise out-of-domain disks.
5. **No rung**: refusal.

The disposer applies the Gap Law among these rungs. It does not choose the most convenient radius.

## Finding P1-1: binarity truncation creates a conservation edge

Capping a scale radius is not only geometry. It changes the material and angular-momentum content of the disk.

For a surface-density profile `Sigma(r)`, the same truncation state determines at least:

$$
M_{\rm gas}
=
2\pi
\int_{r_{\rm in}}^{r_{\rm out}}
\Sigma(r)r\,dr,
$$

and

$$
L_{\rm disk}
=
2\pi
\int_{r_{\rm in}}^{r_{\rm out}}
\Sigma(r)r^2\Omega(r)\,dr.
$$

A consumer must not:

- cap `R_1` in the viscous time;
- leave the old `DiskGas` mass untouched;
- leave the old disk angular momentum untouched;
- and call the disk truncated.

That would create or delete material state by changing one geometric read without reconciling the state whose geometry it describes.

The truncation transition needs a ledger:

```rust
pub struct DiskTruncationLedger {
    pub retained_gas: MassLedger,
    pub retained_angular_momentum: AngularMomentumLedger,
    pub transferred_to_companion: MassAngularMomentumLedger,
    pub transferred_to_circumbinary: MassAngularMomentumLedger,
    pub accreted: MassAngularMomentumLedger,
    pub ejected: MassAngularMomentumLedger,
    pub unresolved: Vec<MissingEdge>,
}
```

If the disk is initialized already truncated, the initial-condition generator must form the retained profile and its normalization consistently. If the disk is truncated dynamically, the removed mass and angular momentum must post somewhere.

This is a Residual-Law blocker on the consumer wire.

## Finding P1-2: one disk fact must have one authoritative producer

The planned wire names both:

- `tau_disk` into the giant gate;
- the `DiskGas` opening state.

Those must read one `DiskEvolutionState`, not two parallel derivations.

A suitable canonical state is:

```rust
pub struct DiskEvolutionState {
    pub stellar_structure_branches: Vec<StellarStructuralBranch>,
    pub high_energy_spectrum: SpectralState,
    pub wind_loss_rate: Band<Fixed>,
    pub truncation: TruncationVerdict,
    pub surface_density: SurfaceDensityState,
    pub gas_mass: Band<Fixed>,
    pub angular_momentum: Band<Fixed>,
    pub lifetime: Band<Fixed>,
    pub provenance: ReceiptId,
}
```

The giant verdict and every disk consumer read this state.

The old reserved lifetime may remain only as:

- a clearly labelled development fixture;
- a validation anchor;
- or an explicit fallback variant that is impossible on the canonical derived path.

A feature flag that leaves both canonical producers live is not a completed retirement. It is a same-fact-two-doors defect hidden behind configuration.

Before re-pin, verify in code:

1. which function the run path calls;
2. which feature or scenario chooses it;
3. whether the old reserved value is still reachable;
4. whether `DiskGas` and the giant gate read the same state object;
5. whether every near-degenerate stellar branch remains present.

## Finding P1-3: truncation uncertainty must reach the giant verdict

The Pichardo fit alone carries a reported scatter of about 6.5 percent. The torque model adds uncertainty from viscosity, aspect ratio, eccentricity, component assignment, and interpolation.

That uncertainty propagates into:

- disk outer radius;
- scale radius;
- surface-density normalization;
- gas mass;
- viscous time;
- photoevaporative clearing time;
- and the race that decides giant formation.

If the giant verdict changes anywhere inside the radius band, the output is branched.

```rust
pub enum TruncationVerdict {
    Resolved(TruncationEvaluation),
    Branched(Vec<TruncationEvaluation>),
    Refused(TruncationRefusal),
}
```

A central radius may be printed for inspection. It may not erase a branch that crosses the consumer's decision gap.

## Finding P1-4: the Kraft field is still a state problem

The report says the metallicity-shift sign is fixed while the kelvin-per-dex magnitude remains underconstrained. Holding the magnitude is correct.

Do not encode the unresolved magnitude as a universal zero.

Zero can mean one of three different things:

- the solar-reference row, where `[Fe/H] = 0` makes the shift term vanish;
- evidence that the slope itself is zero;
- absence of a slope.

Only the first is currently licensed. The type must distinguish them.

The deeper derivation target is stellar structure:

- convective-envelope depth or mass fraction;
- effective temperature;
- surface gravity;
- composition and abundance pattern;
- evolutionary phase;
- rotation where structure is altered by it.

The observational Kraft band is then a diagnostic projection of the structure, not the deepest causal switch.

Two current limits remain load-bearing:

1. a main-sequence Kraft calibration cannot silently classify a pre-main-sequence star;
2. a sign-only metallicity result cannot become a point slope.

A safe interim is:

```rust
pub enum KraftMetallicityConditioning {
    SolarReference,
    SignOnly {
        direction: MonotonicDirection,
        validity: ValidityFrame,
    },
    BandedSlope(StateValue<Fixed>),
    StructureDerived(StellarEnvelopeStateId),
}
```

Inside the unresolved region, `NearDegenerate` must continue to evaluate both wind branches.

## Finding P1-5: the EUV departure is a multidimensional spectral state

The reported lower-temperature EUV extension is not a one-dimensional `T_eff` patch.

The primary atmosphere grids have explicit domains:

- OSTAR2002: approximately `27,500 K <= T_eff <= 55,000 K`, multiple `log g` values, and multiple chemical compositions;
- BSTAR2006: approximately `15,000 K <= T_eff <= 30,000 K`, multiple `log g` values, multiple compositions, and microturbulence variants.

The audit report says `BASTAR2006`. The held source identity should be checked. The standard grid name is `BSTAR2006`.

A departure field therefore keys on at least:

```text
(T_eff, log_g, metallicity/composition, atmosphere family, microturbulence regime)
```

It is not `departure(T_eff)` unless the other axes have been marginalized with a declared band.

### Same-spectrum requirement

The ionizing luminosity and the mean ionizing photon energy must come from the same spectral energy distribution.

For threshold frequency `nu_0`:

$$
L_{\rm ion}
=
\int_{\nu_0}^{\infty}L_\nu\,d\nu,
$$

$$
Q_{\rm ion}
=
\int_{\nu_0}^{\infty}
\frac{L_\nu}{h\nu}
\,d\nu,
$$

and

$$
\langle E_\gamma\rangle
=
\frac{L_{\rm ion}}{Q_{\rm ion}}.
$$

A departure factor derived from one atmosphere grid and a mean photon energy derived from an unrelated blackbody approximation are correlated model choices. Their errors cannot be multiplied as independent scalars.

The preferred object is one spectral evaluation that returns both values and their covariance or common branch identity.

### Lower-temperature handling

Below the lowest atmosphere-grid temperature, there are three legal outcomes:

1. another state-compatible atmosphere grid;
2. a blackbody or LTE upper bound whose maximum wind contribution is proved sub-resolution for the giant verdict;
3. refusal.

Extrapolating the NLTE departure outside the grid is not legal.

The BSTAR gap is not optional if the radiative dispatch sends canonical stars into that domain and the wind can alter the disk decision. It can be deferred only after a Gap-Law proof shows that the maximum missing EUV contribution cannot change the result.

## Finding P1-6: broadband attenuation and reprocessing need explicit frames

Replacing a V-band attenuation scalar with a broadband radiation-field treatment is the right direction.

The Zucconi prestellar-core model has a specific domain:

- external radiation heats dust;
- the field is attenuated through a core;
- the core is assumed optically thin to its own radiation;
- visual/near-infrared, mid-infrared, and far-infrared contributions matter;
- the result was developed for dense prestellar-core conditions.

That does not make one scalar `chi(A_V)` universal for:

- a cloud core;
- a protoplanetary disk surface;
- a disk midplane;
- or a stellar EUV field.

The radiation state should carry its spectrum or band decomposition, extinction law, geometry, and validity frame.

Likewise, disk reprocessing is not naturally a universal fraction. It depends on:

- grazing angle and flaring geometry;
- albedo and scattering;
- optical depth;
- wavelength-dependent opacity;
- shadowing;
- and the stellar spectrum.

If the current reprocessing value is derived from these quantities, preserve the chain and band. If it is a scalar closure, leave it tagged as such.

The canonical reprocessing producer belongs in the physics or simulation data plane. The viewer may display it. The viewer must not be the only place where a causal disk temperature is assembled.

## Finding P1-7: source count is not the provenance result

The reported manifest growth is useful only if every consumed claim closes.

For each new source, require:

- exact artifact identity;
- SHA-256 receipt;
- primary citation;
- exact equation, table, grid node, or passage anchor;
- license and custody state;
- validity domain;
- `used_by` linkage to code;
- and a source-fingerprint test where the value or form is load-bearing.

A wrong-identity Pichardo artifact may remain as an errata or supersession witness. It must not count as support for the active formula.

A pending Wayback URL is not automatically a physics blocker when the exact bytes and hash are already held. It remains a custody limitation. It becomes blocking when the branch has neither held bytes nor a stable public witness for a consumed claim.

The mandatory gate for this arc is not `44 sources`. It is:

```text
all consumed claims
-> exact held or licensed-witness artifact
-> exact anchor
-> exact code consumer
-> exact band and validity
```

## Finding P2-1: `LBF surface-density retirement` is not auditable as named

`LBF` is not a repository-stable symbol identified by this audit. Do not retire anything under an acronym alone.

Current `main` already has a derived viscous-similarity surface-density function. The triage must name:

- exact file and symbol;
- current caller count;
- current provenance;
- whether it is a fixture, duplicate, or authoritative producer;
- and the intended replacement.

If the target is a Lynden-Bell-Pringle style profile, spell that out and verify whether the current derived function is already the canonical implementation. This is a premise-line task, not a new derivation until the symbol is located.

## Numerical audit

### Fractional powers

Both truncation families use small fractional exponents. The implementation should evaluate them in a declared deterministic log carrier:

$$
x^a
=
\exp(a\ln x),
$$

with domain guards before the logarithm.

Required refusals include:

- `e < 0`;
- `e >= 1` where the source fit does not license the endpoint;
- non-positive mass ratios;
- a mass fraction outside the source domain;
- a component assignment that does not resolve;
- and an exponentiation that underflows before a representable final radius is formed.

### Representation liveness

The composed path must preserve the effect of truncation on the final disk state. A radius that changes in bits while the integrated gas mass or lifetime does not, despite physics requiring a change, is a representation failure.

Test the full path:

```text
binary state
-> truncation band
-> surface-density profile
-> gas mass and angular momentum
-> tau_disk band
-> giant verdict branches
```

The preflight validates value agreement and branch agreement against a wider numerical twin.

## Required validation battery

### Source fingerprints

- The held Pichardo equation test asserts `0.733`, `1.20`, and the correct `0.07` mass exponent.
- The source-domain test asserts the fitted `e` and mass-fraction ranges.
- The fit band includes the source's reported approximate `6.5%` scatter.
- The Manara fit test asserts `h = 0.88`, `k = 0.01`, and source-table `b,c` membership.
- A test prevents the Pichardo and Manara coefficient sets from being constructed as one model.

### Binary invariants

- Increasing eccentricity cannot increase the Pichardo circumstellar ceiling inside its domain.
- Radius scales linearly with binary semimajor axis.
- Each component uses the correct Eggleton mass-ratio convention.
- Primary and secondary exchange is symmetric after swapping masses and component identity.
- The returned radius never exceeds the selected component's Roche-lobe ceiling.
- The formula refuses outside its source domain.
- The separation argument is semimajor axis. Eccentricity is not applied a second time through periastron distance unless the source equation requires it.

### Model-rung tests

- Pichardo identifies itself as invariant-loop, coplanar, and low-viscosity.
- The torque fit consumes `alpha_viscosity` and `H/r` through the Reynolds state.
- Model disagreement wider than the consumer gap produces branches.
- An inclined or out-of-domain system refuses or escalates.

### Conservation tests

- Truncation preserves mass double-entry.
- Truncation preserves angular-momentum double-entry.
- `DiskGas` integrates the same truncated profile that `tau_disk` reads.
- No old untruncated disk mass survives behind a truncated radius.
- No removed material disappears without a named sink.

### Kraft tests

- Solar-reference metallicity produces no shift without claiming a zero slope.
- Sign-only conditioning cannot emit a point kelvin-per-dex magnitude.
- Main-sequence and pre-main-sequence validity remain distinct.
- A near-degenerate band evaluates both wind branches.

### EUV tests

- OSTAR and BSTAR grid nodes reproduce their tabulated spectra or integrated quantities.
- The overlap region is cross-checked rather than silently choosing one grid.
- Interpolation carries `T_eff`, `log g`, and composition axes.
- Out-of-grid requests refuse or use a separately labelled bound.
- `L_ion`, `Q_ion`, and mean photon energy derive from the same spectrum.
- Common spectral uncertainty remains correlated.
- A low-temperature missing branch may be dropped only when an upper bound proves it cannot change the disk or giant verdict.

### End-to-end cases

At minimum:

1. low-mass convective pre-main-sequence single star;
2. near-Kraft star carrying both branches;
3. radiative early-B or O star inside the atmosphere-grid domain;
4. radiative lower-temperature star at the grid boundary;
5. circular equal-mass binary;
6. eccentric unequal-mass binary;
7. extreme mass-ratio case inside the source domain;
8. inclined binary that refuses the coplanar rung;
9. branch whose truncation uncertainty changes the giant verdict;
10. branch whose truncation uncertainty does not change it and may collapse with a recorded gap.

Earth or Solar analogues are validation rows, not calibration targets for these cases.

## Layer placement

| Pipeline location | Disk-arc quantity |
| --- | --- |
| Layer 1 constants | No additions |
| Layer 1 mechanisms | Orbital geometry, Roche potential, resonance torque balance, spectral integration, viscous evolution, conservation |
| Layer 2 cache | Stellar atmosphere grids, extinction curves, opacity and spectral rows |
| Layer 2.5 prototypes | Atmosphere-model families, disk-structure model families, truncation-model families |
| Layer 3 estimators | Pichardo invariant-loop ceiling, torque-fit interpolation, blackbody EUV bounds, empirical Kraft projection |
| Layer 3 closures | `alpha_viscosity`, unresolved wind and reprocessing closures, model-family limitations, inclination or thickness closures where no solve exists |
| Layer 4 contingency | Binary masses, semimajor axis, eccentricity, inclination, stellar composition, birth rotation, disk initial state |
| Written state | Disk mass, angular momentum, surface-density history, gas opening, irradiation and clearing history |
| Derived outputs | Truncation band, wind band, disk lifetime band, giant-formation verdict |

No new universal constants are required.

## Gating decision by reported work item

### Accepted in principle

- Keep `alpha_viscosity` as a declared transport floor.
- Keep the Kraft slope magnitude unresolved rather than fit one.
- Keep fetch-first discipline and errata custody.
- Keep the run-path activation behind owner review.
- Keep the capstone coordinator and viewer lane boundaries.
- Keep every dormant commit byte-neutral until the causal wire is reviewed.

### Requires correction or proof before sign-off

- The reported Pichardo formula and its `q^0.01` exponent.
- The model label `expected truncation radius` without low-viscosity and upper-bound semantics.
- The branch/source state, which is not visible from current GitHub state.
- The use of one central truncation value without the source band.
- The absence of a mass and angular-momentum residual ledger.
- The exact mass-ratio and separation conventions.

### Required before the consumer wire

- one canonical `DiskEvolutionState`;
- no reachable second lifetime producer on the canonical path;
- full Kraft branch propagation;
- one spectral model supplying EUV luminosity and mean photon energy;
- grid-domain handling for the radiative branch;
- same-profile integration for `DiskGas`, mass, angular momentum, and `tau_disk`;
- source gates green on the exact head;
- deterministic numerical twins and branch agreement;
- explicit owner re-pin with causal diff.

### Deferred safely

- pending Wayback URLs where bytes and hashes are already held;
- an empirical Kraft slope until a source supplies magnitude;
- exact low-temperature atmosphere coverage when a proved upper bound makes it sub-resolution;
- surface-density retirement until the exact symbol and caller are identified.

## Ruled next work order

### R-DISK-AUDIT-0: recover the state

Push the exact nine-commit head on a fresh branch from current `main`, or cherry-pick the work onto one. Post the SHA and compare result. Do not continue using merged PR #206 as though its deleted head were a live branch.

### R-DISK-AUDIT-1: split and verify truncation models

Read the held Pichardo and Manara primaries. Correct the exponent or the report. Add source-fingerprint tests and distinct model types. Carry bands and validity.

### R-DISK-AUDIT-2: close the disk residual

Build one truncated surface-density state and reconcile retained and removed mass and angular momentum. Make `DiskGas` and `tau_disk` consume that same state.

### R-DISK-AUDIT-3: harden stellar conditioning

Preserve sign-only Kraft information without inventing a slope. Make evolutionary phase explicit. Build one spectral state returning EUV luminosity, photon rate, and mean energy with shared provenance.

### R-DISK-AUDIT-4: close or bound the atmosphere-grid gap

Use BSTAR2006 inside its full `T_eff`, `log g`, and composition domain. Validate its overlap with OSTAR2002. Below the grid, add a licensed bound or refuse. Do not extrapolate silently.

### R-DISK-AUDIT-5: activate once

Wire the single canonical disk state into the giant gate and `DiskGas`, remove or quarantine the old producer, run the end-to-end branch battery, and request one owner-controlled re-pin.

## Source cross-checks and repository provenance status

The following primary works were used as external audit cross-checks. Their statements in this document do not make them closed repository provenance for the unpushed branch. The branch must prove its own held bytes, receipts, anchors, and `used_by` links.

- Pichardo, B., L. S. Sparke, and L. A. Aguilar. 2005. “Circumstellar and circumbinary discs in eccentric stellar binaries.” *MNRAS* 359: 521-530. DOI `10.1111/j.1365-2966.2005.08905.x`. Invariant-loop, coplanar, low-viscosity limits; gas may be further restricted; equation 6 fit and source-domain scatter.
- Manara, C. F., et al. 2019. “Observational constraints on dust disk sizes in tidally truncated protoplanetary disks in multiple systems in the Taurus region.” *A&A* 628: A95. DOI `10.1051/0004-6361/201935964`. Appendix C torque-fit family and dependence on viscosity-related state.
- Papaloizou, J., and J. E. Pringle. 1977. “Tidal torques on accretion discs in close binary systems.” *MNRAS* 181: 441-454. DOI `10.1093/mnras/181.3.441`. Viscous transport and tidal torque balance.
- Lanz, T., and I. Hubeny. 2003. “A Grid of Non-LTE Line-Blanketed Model Atmospheres of O-Type Stars.” *ApJS* 146: 417-441. DOI `10.1086/374373`. OSTAR2002 domain.
- Lanz, T., and I. Hubeny. 2007. “A Grid of NLTE Line-Blanketed Model Atmospheres of Early B-Type Stars.” *ApJS* 169: 83-104. DOI `10.1086/511270`. BSTAR2006 domain.
- Zucconi, A., C. M. Walmsley, and D. Galli. 2001. “The dust temperature distribution in prestellar cores.” *A&A* 376: 650-662. DOI `10.1051/0004-6361:20010778`. Dense-core broadband attenuation and thermal-balance frame.
- Avallone, E. A., et al. 2022. “Rotation Distributions around the Kraft Break with TESS and Kepler: The Influences of Age, Metallicity, and Binarity.” *ApJ* 930: 7. DOI `10.3847/1538-4357/ac60a1`. Metallicity and binarity conditioning, with observational limits on recovering a metallicity trend.

## Final ruling

The agent made the right call by holding at the ownership boundary instead of moving a pin or editing another lane.

Continue that hold.

The immediate task is not the optional Wayback cleanup, the unnamed surface-density retirement, or another fetch. It is to recover the exact branch and resolve the truncation-formula and model-semantics findings.

After those are closed, the next highest-value work is the conservation-backed canonical disk state. The lower-temperature EUV grid follows if its missing contribution is not proven sub-resolution.

The consumer wire is the final step, not the next step.
