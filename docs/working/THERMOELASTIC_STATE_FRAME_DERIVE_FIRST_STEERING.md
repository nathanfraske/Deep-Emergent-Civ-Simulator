# State-coherent thermoelastic properties: derive-first steering for the interior property cluster

Owner-directed steering, 2026-07-19. Design-first and grounded against current `main` plus the in-flight fixture-cluster branch. This document changes no code, sets no value, and does not register a resolved research item. It answers the implementer’s frame-mismatch finding, names the deeper derivation target, and separates the immediate safe correction from the state-resolved thermoelastic arc that remains.

## The question

The current interior-property work tries to replace one self-consistent fixture cluster with a bundle derived from the world’s mineral assemblage. An audit found that the expansivity path evaluates ambient-frame Grüneisen, elastic-modulus, and molar-volume rows as though they were state-local values at interior temperature and pressure. The same branch then feeds the resulting expansivity into conductivity, buoyancy, viscosity, and the convection cluster.

What is the derive-first response that preserves atomicity, admits alien materials, avoids Terran mantle assumptions, and does not turn one forsterite measurement into a hidden universal rule?

## The decision

Adopt the frame refusal.

Apply the independent valence correction separately. Remove the claim that the seven-field property frontier is closed. Do not activate or re-pin the full interior property cluster while expansivity, density, and conductivity still cross incompatible thermodynamic frames.

The current ambient rows remain legitimate floor data. They are not wrong values. The defect is their use outside their declared state and, in the expansivity join, the loss of the definition, uncertainty, and frame that distinguish one Grüneisen rung from another.

The immediate code should make that misuse impossible. The complete solution is a state-local thermoelastic provider that derives a phase’s volume and response functions at the requested pressure, temperature, composition, phase, and written state. No new universal numerical constant is required.

The atomicity ruling survives, with a sharper statement:

> A column consumes one state-coherent property bundle or it refuses. Atomicity does not license moving seven fields together when the fields do not describe the same thermodynamic state.

## Repository grounding

### The warning was already in the substrate

`crates/physics/src/gruneisen.rs` states that its rows are ambient-frame values near 300 K and 1 bar. `GruneisenRow` stores the row temperature and pressure. `AssemblageGamma` carries `frame_temperature_k` and `frame_pressure_bar` so a caller cannot silently treat an ambient aggregate as a deep-interior value.

`crates/physics/data/gruneisen.toml` makes the same distinction more precisely. Its measured rung is the thermodynamic Grüneisen parameter

$$
\gamma_{\rm th}
=
\frac{\alpha_V K_S V}{C_P}
=
\frac{\alpha_V K_T V}{C_V}
$$

at the row’s stated reference conditions. Its cross-channel `gamma_eos_debye` is a different vibrational average. Its Slater rung is an estimator with declared failures for chain and framework silicates.

`crates/physics/data/mineral_moduli.toml` carries ambient adiabatic bulk moduli $K_S$ and shear moduli at about 298 K and 1 bar. `crates/physics/src/petrology_data.rs` carries standard-state molar volumes. The conductivity ladder states that its pressure frame is ambient and that its temperature propagation has no pressure dependence.

The implementer’s finding is therefore a real consumer defect: the data already declared the frame and the consumer dropped it.

### The in-flight join loses more than the frame

The in-flight `assemblage_volumetric_expansivity_per_k` currently:

1. reads one scalar gamma and discards its rung, band, and frame;
2. reads the ambient $K_S$ row as one scalar;
3. reads a standard-state molar volume;
4. supplies Dulong-Petit $C_V$;
5. computes one positive scalar $\alpha_V$;
6. volume-averages the phase scalars;
7. treats that scalar as constant from 298 K to the interior temperature;
8. passes its integral into a conductivity exponent.

This has four independent defects.

First, the inputs do not share a state.

Second, the conjugate pair can be inconsistent. A gamma defined through $K_S$ and $C_P$ must reconstruct alpha through $K_S$ and $C_P$. A gamma defined through $K_T$ and $C_V$ must use $K_T$ and $C_V$. Mixing measured $\gamma_{\rm th}$, ambient $K_S$, and a separately approximated $C_V$ is not one of the thermodynamic identities unless the $S/T$ conversion is performed.

Third, the apparent forsterite agreement at high temperature is not an independent validation when a gamma row was itself obtained by inverting measured expansivity. A second X-ray volume measurement can validate the result. Reconstructing the source’s own alpha through the source’s own gamma cannot.

Fourth, the gamma and expansivity bands are discarded before an exponential consumer. The Hofmeister temperature correction contains

$$
\exp\left[-\left(4\gamma+\frac13\right)\int \alpha_V\,dT\right].
$$

A central gamma and central alpha are not legal substitutes for their bands in this exponent. The interval must propagate through the exponent, or the consumer must escalate when its width changes the conductivity or convection branch.

## The stronger reframe: derive the state function before the coefficient

Thermal expansivity is a response derivative:

$$
\alpha_V(P,T)
=
\frac{1}{V}
\left(\frac{\partial V}{\partial T}\right)_P.
$$

The most derived target is therefore not a free-standing alpha model. It is a state-local phase volume or free-energy model.

For a crystalline phase with a Helmholtz free-energy surface $F(V,T,\mathbf{x},s)$:

$$
P
=
-\left(\frac{\partial F}{\partial V}\right)_T,
$$

$$
K_T
=
V\left(\frac{\partial^2 F}{\partial V^2}\right)_T,
$$

$$
C_V
=
-T\left(\frac{\partial^2 F}{\partial T^2}\right)_V.
$$

Solving the first relation for $V(P,T)$ gives:

$$
\alpha_V
=
\frac{1}{V}
\left(\frac{\partial V}{\partial T}\right)_P.
$$

The thermodynamic Grüneisen parameter then becomes a consistency read:

$$
\gamma_{\rm th}
=
\frac{\alpha_V K_T V}{C_V}.
$$

This direction is one derivation hop shorter than reconstructing volume response from a scalar gamma. It also produces density, bulk modulus, heat capacity, expansivity, and thermal pressure from one state surface, so the values cannot silently describe different frames.

For the conductivity correction at fixed pressure, the same state surface removes the numerical integral:

$$
\int_{T_0}^{T}\alpha_V(P,T')\,dT'
=
\ln\frac{V(P,T)}{V(P,T_0)}.
$$

The conductivity consumer should prefer this log-volume ratio when a volume model exists. It is exact within that model, preserves the state frame, and avoids pretending alpha is constant across a 1300 K interval.

For buoyancy, a state-resolved volume model also permits the direct density contrast:

$$
\rho(P,T)=\frac{M}{V(P,T)},
$$

$$
\Delta\rho
=
\rho(P,T_{\rm reference})
-
\rho(P,T_{\rm parcel}).
$$

The current Boussinesq form $\rho\alpha\Delta T$ remains a small-perturbation approximation and a regression twin. It should not remain the deepest route for a large temperature contrast once both state densities are available.

## The thermodynamic definition gate

The state provider must carry which response-function basis it uses.

The two exact Grüneisen forms are:

$$
\gamma_{\rm th}
=
\frac{\alpha_V K_T V}{C_V}
$$

and

$$
\gamma_{\rm th}
=
\frac{\alpha_V K_S V}{C_P}.
$$

They are related by:

$$
\frac{K_S}{K_T}
=
\frac{C_P}{C_V}
=
1+\alpha_V\gamma_{\rm th}T.
$$

A legal evaluation must do one of the following:

- use the $K_T,C_V$ pair at one state;
- use the $K_S,C_P$ pair at one state;
- convert between the pairs through a self-consistent thermoelastic solve;
- derive alpha directly from $V(P,T)$ and use gamma only as a cross-check.

The API must refuse a mixed pair. A field called `bulk_gpa` without an isothermal or adiabatic definition is insufficient at this boundary.

## The state-resolved ladder

The phase property route should be a ladder whose rungs all answer the same state query.

### Rung 1: measured pressure-volume-temperature surface

A measured P-V-T equation of state or tabulated volume surface supplies $V(P,T)$ inside its cited domain. Alpha, density, and $K_T$ derive from that surface. Measured heat-capacity data may complete the response bundle, or a compatible model may supply it with a lower grade.

Forsterite has a direct high-pressure, high-temperature source lead extending to about 14 GPa and 1900 K. That is a strong measured rung and a validation target for one phase. It is not the universal mechanism.

### Rung 2: compute-once free-energy surface

A quasi-harmonic or anharmonic calculation supplies $F(V,T)$ for a phase and composition. The derivative bundle derives from it and is cached by phase, composition bucket, pressure bucket, temperature bucket, and structure branch.

This is the alien-admitting route for a generated crystalline phase without a laboratory P-V-T surface. The result carries the method’s approximation band and validity limits, including phonon instability, strong anharmonicity, electronic or magnetic transitions, and melting.

### Rung 3: Mie-Grüneisen-Debye or other equation-of-state estimator

A reduced thermal equation of state may use per-phase anchors such as

- $V_0$,
- $K_{T0}$ and $K'_{T0}$,
- Debye temperature,
- $\gamma_0$,
- the volume exponent $q$,
- and an anharmonic or electronic correction where required.

These are phase data or compute-once results, not new universal constants. The model returns a band and refuses outside its phase and state domain.

### Rung 4: ambient measured response

An ambient X-ray or dilatometry row may return alpha inside its measured temperature-pressure frame. It is also an independent validation anchor for a deeper model.

It may not be extended to mantle conditions by a silent constant-alpha assumption.

### No rung: refusal

A phase with no state-local rung refuses. A census containing such a phase either preserves a banded branch that excludes it for a physically derived reason, or the whole state-coherent property bundle refuses.

No phase receives an Earth-mineral default.

## The frame must be a type-level object

The immediate fix should not add temperature and pressure arguments that are ignored after a hardcoded ambient check. The arguments must meet a value whose provenance carries its frame.

A suitable shape is:

```rust
pub struct ThermodynamicState {
    pub temperature_k: Fixed,
    pub pressure_bar: Fixed,
    pub composition: CompositionId,
    pub phase_state: PhaseStateId,
    pub written_state: Option<WrittenStateId>,
}

pub struct ValidityFrame {
    pub temperature_band_k: Band<Fixed>,
    pub pressure_band_bar: Band<Fixed>,
    pub phase_regime: PhaseRegime,
    pub composition_domain: CompositionDomain,
    pub state_requirements: Vec<StateRequirement>,
}

pub enum ThermoelasticBasis {
    Isothermal,
    Adiabatic,
    FreeEnergySurface,
    DirectVolumeSurface,
}

pub struct StateValue<T> {
    pub value: T,
    pub band: ErrorBand,
    pub frame: ValidityFrame,
    pub basis: ThermoelasticBasis,
    pub provenance: ReceiptId,
}

pub enum StatePropertyRefusal {
    FrameMismatch {
        requested: ThermodynamicState,
        available: ValidityFrame,
    },
    DefinitionMismatch {
        required: ThermoelasticBasis,
        supplied: ThermoelasticBasis,
    },
    NoStateModel {
        phase: String,
    },
    PhaseBoundary {
        branches: Vec<PhaseStateId>,
    },
    WrittenStateRequired {
        state: StateRequirement,
    },
    RepresentationRefusal {
        reason: String,
    },
}
```

The exact type names are open. The contract is not.

A scalar accessor such as `gamma(name) -> Fixed` is not sufficient for a state consumer. A state consumer needs a framed evaluation or a refusal.

## Immediate implementation steering

### Apply now

1. Apply the valence fix independently. It consolidates one derivation and does not depend on thermoelastic state resolution.
2. Give the expansivity route a requested temperature and pressure.
3. Make every input row expose its frame and thermodynamic definition.
4. Refuse when the requested state is outside any load-bearing input frame.
5. Rename the ambient-only helper so its scope is visible, for example `ambient_assemblage_volumetric_expansivity`.
6. Remove `FRONTIER CLOSED` and the “all seven properties derive” claim.
7. Keep the property cluster dormant and do not re-pin the run path.
8. Preserve the existing fixture bundle as an explicitly labelled fixture variant until one state-coherent derived bundle is available.

### Do not apply

Do not insert a forsterite-at-1600-K constant into the universal function.

Do not widen the ambient frame until the source supports the wider domain.

Do not mix a high-temperature bulk modulus with ambient gamma and standard molar volume to improve one number.

Do not treat a matching point as validation when the point shares an input source with the derivation.

Do not let the atomicity requirement force an invalid property into the bundle.

## The cluster contract

The current type says “all derived or none.” The corrected type should say “one coherent provider or none.”

```rust
pub enum ColumnPropertyBundle {
    Fixture {
        values: ColumnParams,
        provenance: ProvenanceKey,
    },
    StateResolved {
        state: ThermodynamicState,
        values: StateResolvedColumnProperties,
        provenance: ReceiptId,
    },
    Refused(ColumnDerivationRefusal),
}
```

A state-resolved bundle should carry the state and the worst effective grade of every transitive input. If density is measured-state, conductivity is estimator-state, expansivity is unresolved, and viscosity is banded, the result is not a partially derived mantle bundle. It is a refusal with three independently useful dormant calculations behind it.

This preserves the original reason for atomicity: the convection kernel must not combine fields from incompatible unit systems, states, or provenance grades.

## Assemblage expansivity is not generally a volume average

For fixed phase amounts and mechanically independent phases, a volume-weighted alpha is a useful first approximation. A rock is a mechanically coupled polycrystal. Differences in stiffness and directional expansion generate internal stress, so effective thermoelastic response depends on the elastic tensor, texture, porosity, and homogenization rule.

At the deeper level, let:

$$
V_{\rm asm}(P,T,\mathbf{x},\mathcal{H})
=
\sum_i n_i(P,T,\mathbf{x},\mathcal{H})V_i(P,T,\mathbf{x}_i,\mathcal{H}_i).
$$

Here $\mathcal{H}$ is written state, including metastable phase retention, defects, hydration, texture, and other path-dependent state.

Away from a phase transition:

$$
\alpha_{\rm asm}
=
\frac{1}{V_{\rm asm}}
\left(
\frac{\partial V_{\rm asm}}{\partial T}
\right)_{P,\mathbf{x},\mathcal{H}}.
$$

This derivative includes both phase-volume response and any permitted phase-fraction response. At a first-order phase transition, one scalar derivative may not exist. The disposer and freezer preserve the competing branches and latent volume change.

The recommended aggregation ladder is:

1. derivative of a state-resolved assemblage volume where the petrology and kinetic state support it;
2. thermoelastic homogenization or rigorous bounds using phase stiffness and expansion tensors;
3. volume-weighted phase alpha as an estimator with a declared band;
4. refusal.

A volume mean must not wear a derived-exact grade.

## The exponent consequence

The conductivity path is the first consumer that makes the frame error multiplicative.

The current conductivity interval walks only the anchor conductivity band. It does not walk the gamma band, the state-volume band, the expansivity band, or the model branch. That is insufficient because gamma and the expansion integral sit in the exponent.

A state-resolved conductivity evaluation must propagate at least:

- conductivity-anchor uncertainty;
- gamma or log-volume uncertainty;
- equation-of-state uncertainty;
- temperature and pressure state bands;
- phase-fraction and mixing uncertainty;
- the estimator rung and validity domain.

The legal implementation may evaluate monotone interval corners where monotonicity is proved. Otherwise it preserves branches or escalates. A mean alpha inside the exponent is forbidden.

The same result propagates into viscosity and Rayleigh onset. A conductivity branch that changes thermal diffusivity enough to cross the onset gap must remain a branch.

## The sign and anisotropy audit

The present substrate treats non-positive gamma and non-positive alpha as nonphysical or unrepresentable. That is a Terran-bias and generality defect.

Solids with negative thermal expansion exist. Their low-energy modes can carry negative Grüneisen parameters. A signed alpha or gamma is therefore a physical result, not an absence sentinel.

Zero is also physical at a compensation point. Missing data must be represented by `Option`, `Result`, or a refusal variant, never by zero.

The phase-level representation should permit:

- positive volumetric expansion;
- zero expansion;
- negative volumetric expansion;
- anisotropic expansion tensors with positive and negative principal components;
- sign changes across temperature or pressure;
- multi-branch behavior at structural, electronic, magnetic, or ordering transitions.

The convection consumer may impose a regime-specific requirement that a parcel becomes buoyant under heating. That requirement belongs at the consumer and returns a physical no-buoyancy result. It must not rewrite the universal material floor to forbid negative expansion.

For anisotropic crystals, the scalar volumetric coefficient is the trace of the expansion tensor and is sufficient only for a hydrostatic density consumer. Tool deformation, fracture, texture evolution, and directional transport need the tensor. The scalar path should therefore be a derived projection, not the only stored shape.

## Admit-the-alien audit

No function should branch on `mantle`, `olivine`, `Earth`, or `rock-forming` to decide the physics.

The dispatch key is the material’s state representation:

- a crystalline phase may use a P-V-T or free-energy surface;
- a liquid may use a liquid equation of state;
- a gas or supercritical fluid uses its own equation of state;
- a glass or amorphous solid reads fictive temperature and path-dependent written state;
- an electronic or magnetic branch carries its competing free-energy state;
- a porous or damaged aggregate carries pores, cracks, and damage as written state;
- a generated alien phase adds a measured row, a compute-once surface, or an estimator rung.

The seed registry may begin with terrestrial minerals because those are the available validation set. The mechanism and failure modes must not assume that set is complete.

Hydration, iron content, defects, and solid solution are composition and written-state inputs. “Forsterite” is not one state if the water content, defect population, or substitutional chemistry differs.

## Layer placement

| Pipeline location | State-coherent thermoelastic addition |
| --- | --- |
| Layer 1 constants | None |
| Layer 1 mechanisms | Definitions of $V(P,T)$, $\alpha_V$, $K_T$, $K_S$, $C_V$, $C_P$, thermodynamic gamma, frame compatibility, and response-identity residuals |
| Layer 2 cache | Per-phase P-V-T or free-energy surfaces, elastic tensors, heat-capacity surfaces, expansion tensors, validity domains, and state-local derivative bundles |
| Layer 2.5 prototypes | Crystalline QHA, Mie-Grüneisen-Debye, liquid EOS, glass-state, magnetic/electronic branch, and thermoelastic homogenization model families |
| Layer 3 estimators | Ambient-to-state EOS estimates, anharmonic corrections, homogenization bounds, missing tensor reductions, and pressure-temperature conductivity corrections |
| Layer 3 closures | Model-family selection where physics does not decide, subgrid texture and crack statistics, anharmonic truncation, and unresolved phase kinetics |
| Layer 4 contingency | None added |
| Written state | Phase fractions, metastability, defects, hydration, texture, damage, porosity, fictive temperature, and branch history |
| Derived outputs | State-local density, moduli, heat capacity, expansivity, conductivity, diffusivity, buoyancy, and viscosity bands |

## Validation constitution

### Frame tests

- An ambient row succeeds inside its declared temperature-pressure frame.
- The same row refuses at 1600 K or deep pressure unless a state model promotes it.
- A state request cannot drop the row’s temperature, pressure, phase, composition, or definition.
- A mixed ambient/high-temperature input set refuses even when its central answer matches a target.

### Definition tests

- $K_T,C_V$ and $K_S,C_P$ routes agree after a self-consistent conversion.
- A $K_S,C_V$ or $K_T,C_P$ mix refuses.
- The response identity residual closes within the propagated band.
- `gamma_thermodynamic` and `gamma_eos_debye` remain distinct rungs.

### Independent-anchor tests

- The channel-relayed Ye, Schwering, and Smyth value is treated as a fetch target until the primary is held and the exact table or fit is read.
- An ambient X-ray volume fit validates ambient alpha without sharing the gamma inversion source.
- A high-pressure, high-temperature forsterite P-V-T surface validates the state route within its domain.
- High-temperature ambient forsterite data test the temperature path separately from the pressure path.

### Sign and tensor tests

- A negative-expansion phase evaluates to a signed negative alpha rather than refusing.
- A zero-crossing is represented as a physical result.
- An anisotropic phase can have mixed-sign principal expansion components while its volumetric trace remains well-defined.
- A scalar-only consumer reads the trace; a tensor consumer refuses a scalar-only row.

### Assemblage tests

- A one-phase census reproduces the phase state exactly.
- A two-phase volume mean is labelled estimator-grade and lies inside a thermoelastic bound.
- Internal-stress or texture changes widen or branch the aggregate result.
- A phase transition returns branches or a latent volume jump instead of a fabricated derivative.
- Changing written metastable state can change the assemblage response while the solver remains memoryless.

### Exponent and consumer tests

- Gamma, volume, and alpha bands propagate through the conductivity exponent.
- A conductivity interval that crosses a convection decision gap preserves both branches.
- The log-volume-ratio route twins the integrated-alpha route within band.
- Direct state-density contrast twins $\rho\alpha\Delta T$ only in the small-perturbation regime.
- No partial property bundle reaches `convection_step`.

### Alien-general tests

- A negative-expansion framework solid is admitted.
- A phase with no Earth analogue follows a compute-once or estimator rung.
- A liquid, glass, and crystalline solid answer through different state providers without changing the consumer API.
- No test requires an Earth mantle label.

## Source leads and provenance status

The repository already holds the ambient Grüneisen and elastic sources that exposed the frame mismatch. Those rows remain the authoritative evidence for their stated frames.

The following are source-acquisition leads, not closed provenance:

- Ye, Yu, Richard A. Schwering, and Joseph R. Smyth. 2009. “Effects of hydration on thermal expansion of forsterite, wadsleyite, and ringwoodite at ambient pressure.” *American Mineralogist* 94: 899-904. DOI `10.2138/am.2009.3122`. Candidate independent ambient X-ray anchor and anisotropic expansion source.
- Katsura, Tomoo, et al. 2009. “Thermal expansion of forsterite at high pressures determined by in situ X-ray diffraction: The adiabatic geotherm in the upper mantle.” *Physics of the Earth and Planetary Interiors* 174: 86-92. DOI `10.1016/j.pepi.2008.08.002`. Candidate P-V-T and Mie-Grüneisen-Debye state-surface anchor to 14 GPa and 1900 K.
- Bouhifd, M. A., D. Andrault, G. Fiquet, and P. Richet. 1996. “Thermal expansion of forsterite up to the melting point.” *Geophysical Research Letters* 23: 1143-1146. DOI `10.1029/96GL01118`. Candidate high-temperature ambient validation.
- Rosen, B. Walter, and Zvi Hashin. 1970. “Effective thermal expansion coefficients and specific heats of composite materials.” *International Journal of Engineering Science* 8: 157-173. DOI `10.1016/0020-7225(70)90066-2`. Candidate thermoelastic aggregate bounds.
- Mittal, R., S. L. Chaplot, H. Schober, and T. A. Mary. 2001. “Origin of negative thermal expansion in cubic ZrW2O8 revealed by high pressure inelastic neutron scattering.” *Physical Review Letters* 86: 4692-4695. DOI `10.1103/PhysRevLett.86.4692`. Candidate direct evidence that negative mode Grüneisen parameters and negative expansion are physical.

Every lead must pass the repository source pipeline before its numeric rows, exact definitions, or quoted claims enter code. The channel-relayed `29.1 ± 2.6 ppm/K` is a fetch-spec seed until the primary anchor is read and held. Do not enter it as though this document were the citation.

## Staged work

### R-THERMOFRAME-0: immediate refusal and claim correction

Add the requested state and frame check. Preserve bands and rung identity. Permit signed alpha and gamma in the generic types. Remove the closed-frontier and all-seven headline. Keep the cluster dormant.

### R-THERMOFRAME-1: independent ambient validation

Vendor the independent ambient X-ray source, identify the exact volume fit, and twin the ambient reconstruction using the thermodynamically matching conjugate pair.

### R-THERMOFRAME-2: phase state provider

Add the `ThermodynamicState`, `ValidityFrame`, framed-value, definition, and refusal types. Promote the standard molar-volume row into a state-local volume-provider interface rather than treating the standard row as the volume at every state.

### R-THERMOFRAME-3: forsterite P-V-T rung

Vendor and implement the high-pressure, high-temperature forsterite surface as the first measured state rung. Derive $V$, $\rho$, $K_T$, alpha, and the log-volume ratio from one model. Validate without fitting the target.

### R-THERMOFRAME-4: compute-once and estimator rungs

Add QHA or another compute-once surface and a banded Mie-Grüneisen-Debye fallback for phases without measured P-V-T coverage. Refuse beyond crystalline and model validity.

### R-THERMOFRAME-5: assemblage thermoelasticity

Replace the unbanded volume mean with the aggregation ladder. Couple phase fractions to written state and the freezer. Preserve phase-transition branches.

### R-THERMOFRAME-6: consumers

Feed state density into buoyancy, log-volume ratio into conductivity, and the fully propagated property band into viscosity and Rayleigh. Only here does the atomic fixture cluster retire and the realization digest re-baseline.

## Result

The implementer should adopt the frame refusal and land the valence fix independently.

The expansivity frontier is open. More broadly, the current branch has derived several ambient-frame phase and aggregate properties, not a complete interior-state bundle. That is useful progress and a precise work list, not a failed arc.

The deeper derivation is one state surface feeding all thermoelastic responses. It removes the mixed-frame error, shortens the conductivity path, admits negative and anisotropic expansion, preserves alien phases, and turns a matching forsterite number from a seductive endpoint into a real validation target.
