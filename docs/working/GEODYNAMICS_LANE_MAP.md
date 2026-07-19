# Geodynamics lane map: four workstreams, their conflicts, and their order

A `gpt-5.6-sol` mapping pass, 2026-07-19, over the four candidate workstreams after the flexure
representation design landed. The brief asked it to verify every premise I stated rather than take them,
and to treat "it must not exist" as a hypothesis, because this repository produced four
banked-but-unread findings in one session.

IT CORRECTED TWO OF MY PREMISES AND FOUND FIVE MORE DORMANT PIECES.

  - I said the impact reservoir count "waits on a disk residual-mass model that does not exist". WRONG.
    `sim::smallbody::residual_disk_mass` already exists and its own documentation names it the precursor
    to a physical body count. It is dormant. What is genuinely missing is the ALLOCATION of that
    system-wide reservoir to one target's late accretion, which `planetary_assembly.rs` has already
    pre-registered as a future mass-transfer edge.
  - I said the stagnant-lid suppression is "derivable from rows already in the tree", quoting the law's
    own comment. Only PARTLY. `E*`, `V*` and the admitted mechanism exist; the stagnant-lid scaling
    coefficient and the causal lid-strength input do not. Worse, the production creep row is
    NON-NEWTONIAN (`n = 3.5`), so a Newtonian stagnant-lid coefficient cannot be silently reused, and the
    admitted viscosity already carries `E* + P V*`, so using `E*` alone would make the suppression
    inconsistent with the viscosity that produced `ln_viscosity`.
  - The `laws.rs` comment promising a `tectonic_regime` DISPATCH is stale and wrong: that module's
    contract says regime labels are observer-only and "NEVER causal". The branch must be selected from a
    continuous causal quantity (convective stress against lid yield strength, the existing
    `mobilization_margin`), with the enum remaining a readout.
  - Also dormant and unread: `world::eruption::gas_thrust_exit_velocity` (no production consumer),
    `relax_to_support_bound` (bit-conservative lateral crust transfer, tested but with no production
    consumer), and the resident `GeodynamicColumn` continuous-mobility path, which is not the path the
    viewer's `DeepTimeState` uses.

THE STRUCTURAL FINDING, which is what makes this a map rather than a list: the four lanes' PHYSICS
KERNELS are disjoint, and their RUN-PATH INTEGRATIONS all converge on `sim/deeptime.rs` and
`viewer/main.rs`. Parallelism is therefore safe at kernel scope and unsafe at integration scope, and the
plan below is built on that distinction rather than on the lanes' subject matter.

The verbatim map follows.

---

## Bottom line

- **A is file-disjoint from every other lane** and can run concurrently with all of them.
- **B, C(ii), and D are not safe as full parallel implementations**: all converge on `crates/sim/src/deeptime.rs`; production wiring also converges on `crates/viewer/src/main.rs`.
- **C(i)’s low-level reservoir/count derivation is parallel-safe**, but replacing the viewer’s `40` requires a missing late-accretion allocation edge.
- Several “missing” pieces already exist but are unread: the residual disk mass, stagnant-lid rheology inputs, eruption velocity law, continuous mobility observer, and conservative lateral crust redistribution.

## Premise verification

| Premise | Verdict |
|---|---|
| A’s stated footprint | Correct; it matches the design. |
| `tectonic_regime` is descriptive, not dispatch | Correct. The module explicitly says regime labels are observer-only and “NEVER causal” ([tectonic_regime.rs:15](/home/nathan/Deep-Emergent-Civ-Simulator/crates/foundation/src/tectonic_regime.rs:15), [lib.rs:71](/home/nathan/Deep-Emergent-Civ-Simulator/crates/foundation/src/lib.rs:71)). The comment in `laws.rs` claiming a regime dispatch is stale/wrong. |
| Stagnant-lid suppression is fully derivable from existing rows | Only partly. `E*`, `V*`, and the admitted creep mechanism exist; the stagnant-lid scaling coefficient/convention and causal lid-strength input do not. |
| The impact count waits on a residual-mass model that does not exist | Wrong. `smallbody::residual_disk_mass` already exists and is explicitly documented as the precursor to physical body count ([smallbody.rs:322](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/smallbody.rs:322)). It is dormant. What is missing is allocation of that global reservoir to a particular target’s late accretion. |
| `eruption.rs` has zero run-path consumers | Correct. It has tests and an example consumer, but no simulation/viewer run-path consumer ([eruption.rs:35](/home/nathan/Deep-Emergent-Civ-Simulator/crates/world/src/eruption.rs:35), [example:94](/home/nathan/Deep-Emergent-Civ-Simulator/crates/world/examples/eruption.rs:94)). |
| Crust has no destruction path | Correct. The exact identifier `crustal_growth` is absent, but `crust_growth` exists and is explicitly non-negative ([deeptime.rs:317](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:317)). |
| Dense crust is representable as foundering | Correct at the isostasy/readout level, but no mass transfer follows from it. |

---

## A. Flexure and moment equivalence

I did not redesign it.

### File set

Exactly the design’s implementation footprint:

- `crates/physics/src/flexure.rs`
- `crates/physics/src/moment_equivalence.rs`
- `crates/physics/src/hindcast_comparison.rs` only if the migration includes the designed Mars/hindcast range checks

The staged footprint is recorded in the existing design ([design:397](/home/nathan/Deep-Emergent-Civ-Simulator/docs/working/FLEXURE_REPRESENTATION_DESIGN.md:397)). Tests are inline in each corresponding module: [flexure.rs:846](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/flexure.rs:846), [moment_equivalence.rs:2699](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/moment_equivalence.rs:2699), and [hindcast_comparison.rs:209](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/hindcast_comparison.rs:209).

### Smallest real slice

The design’s first scaling migration in `flexure.rs`, with its operating-range tests. `moment_equivalence.rs` should remain a separate commit/slice if agent ownership is being optimized.

### Blocker

None identified beyond implementing the already-decided representation design.

---

## B. Stagnant-lid branch

### What the suppression actually is

For the zero-pressure Arrhenius approximation,

\[
\Delta T_{\mathrm{rh}} = \frac{R T_i^2}{E^*},
\qquad
\theta = \frac{\Delta T}{\Delta T_{\mathrm{rh}}}
       = \frac{E^*\Delta T}{R T_i^2}.
\]

A conventional stagnant-lid scaling has the form

\[
Nu_{\mathrm{stag}} \propto Ra_i^{1/3}\theta^{-4/3},
\]

so the suppression relative to the corresponding isoviscous \(Ra^{1/3}\) term is \(\theta^{-4/3}\), subject to a different regime coefficient and the existing `Nu >= 1` conduction bound. Primary stagnant-lid treatments support this form, but also show that coefficients and definitions vary with rheology and geometry ([GJI treatment](https://academic.oup.com/gji/article/220/1/18/5571091), [Arrhenius scaling study](https://www.sciencedirect.com/science/article/abs/pii/S0031920104003942)).

The premise that `RT²/E*` is the exact production scale is incomplete. The admitted viscosity law already uses pressure-dependent activation volume through

\[
\exp[-(E^*+PV^*)/(RT)].
\]

At nonzero pressure, the local rheological derivative therefore involves `E* + P V*`, with an additional term if pressure changes along the thermal reference path. Using `E*` alone would make the stagnant suppression inconsistent with the viscosity that produced `ln_viscosity`.

### Banked rows

The relevant bank is `crates/physics/src/creep_rows.rs`:

- Production-admitted dry dislocation: `E*=530 kJ/mol`, `n=3.5`, fitted ([creep_rows.rs:532](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/creep_rows.rs:532)).
- Dry GBS: `E*=400 kJ/mol`, but the value is `Assumed` and deliberately rejected by the exponent gate ([creep_rows.rs:549](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/creep_rows.rs:549)).
- Wet rows have fitted energies but are unavailable because no water state is supplied.
- Activation-volume brackets are already selected and enforced by the production viscosity derivation ([sim/geodynamics.rs:905](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/geodynamics.rs:905)).

The row fields are public, so `creep_rows.rs` need not be edited. What is currently lost is row identity and activation parameters: `ColumnThermalProperties` retains only a `ViscosityBand` ([sim/geodynamics.rs:280](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/geodynamics.rs:280)).

### Dispatch

Do not turn `civsim_foundation::tectonic_regime` into a switch. Its contract rejects that architecture.

The branch should be selected from continuous causal quantities, such as convective stress versus lid yield strength—equivalently the existing `mobilization_margin` with a boundary near one ([tectonic_regime.rs:37](/home/nathan/Deep-Emergent-Civ-Simulator/crates/foundation/src/tectonic_regime.rs:37)). The enum/string remains a readout of that result.

Currently the SI run path cannot do this:

- `SiColumnParams` has viscosity and thermal inputs, but no activation parameters or lid yield strength ([sim/geodynamics.rs:1094](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/geodynamics.rs:1094)).
- `convection_step_si` locally reimplements the mobile-lid/isoviscous log-domain Nusselt expression; it does not call `mantle_convective_heat_flux` directly ([sim/geodynamics.rs:1201](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/geodynamics.rs:1201)).
- The viewer has only a reserved display-side crust strength constant and explicitly says the material yield strength is not threaded ([main.rs:1387](/home/nathan/Deep-Emergent-Civ-Simulator/crates/viewer/src/main.rs:1387)).
- The current convection catalogue contains only the isoviscous scaling family ([convection_scaling.toml:1](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/data/convection_scaling.toml:1)).

### File set

Smallest physics slice:

- `crates/physics/data/convection_scaling.toml`
- `crates/physics/src/convection_scaling.rs`
- `crates/physics/src/laws.rs`

Full run-path integration:

- the three files above
- `crates/sim/src/geodynamics.rs`
- `crates/sim/src/deeptime.rs`
- `crates/viewer/src/main.rs` to thread the causal material/lid strength

No edit should be made to `foundation/src/tectonic_regime.rs` unless only documentation/tests are being clarified.

### Smallest real slice

Bank a properly cited stagnant-lid coefficient/convention and add a pure, log-domain stagnant-lid Nusselt kernel accepting explicit `Ti`, `ΔT`, `E*`, `P`, and `V*`, with limiting and `Nu >= 1` tests. Do not claim this is production dispatch yet.

### Blockers

- No stagnant-lid coefficient/convention is banked.
- No causal lid yield strength reaches the SI convection consumer.
- `ColumnThermalProperties` drops the admitted creep row’s `E*`/`V*`.
- The production row is non-Newtonian (`n=3.5`), so a Newtonian stagnant-lid coefficient cannot be silently reused.
- Viscosity is presently derived once for the initial state and then held fixed while temperature evolves, which becomes more conspicuous once temperature-sensitive suppression is introduced.

---

## C(i). Impact reservoir count

The viewer placeholder and its stated derivation are exactly as described ([main.rs:1198](/home/nathan/Deep-Emergent-Civ-Simulator/crates/viewer/src/main.rs:1198)).

### Already built but unread

`sim::smallbody` already has:

- A `DiskReservoir` with surface-density profile and size-distribution parameters.
- Feeding-zone masking.
- A derived residual disk-mass integral.
- Documentation explicitly saying physical count is residual mass divided by characteristic body mass.

The module itself calls this path dormant. The disk composition and giant-planet gas modules are not substitutes for it.

The remaining problem is that this is a system-wide unaccreted reservoir, not the number of bodies that will encounter one particular planet. `planetary_assembly.rs` explicitly pre-registers late accretion as a future mass-transfer edge from this reservoir into planets ([planetary_assembly.rs:646](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/planetary_assembly.rs:646)).

The impact-size parameters already supply enough information to calculate a mean body mass—absolute size bounds, power-law slope, and density—but no analytic number-weighted mass helper exists in `impact_flux.rs`.

### File set

Smallest derivation slice:

- `crates/world/src/impact_flux.rs`: analytic mean-body-mass/count helper and tests
- `crates/sim/src/smallbody.rs`: bridge residual mass to physical reservoir count, still explicitly system-wide

Full replacement of `IMPACT_RESERVOIR_BODY_COUNT`:

- the two files above
- `crates/sim/src/planetary_assembly.rs`: late-accretion/delivery allocation and mass ledger
- `crates/viewer/src/main.rs`: use the target-specific count

`astro.rs`, `giants.rs`, and disk-composition files need not be changed for this slice.

### Smallest real slice

Derive and test the physical **global residual body count** from residual disk mass plus the existing size-frequency distribution. Keep the viewer’s target count flagged until an encounter/delivery fraction exists.

### Blocker

A target-specific late-accretion allocation/capture model, not residual disk mass itself.

---

## C(ii). Eruption wiring

### Verification

`world::eruption::gas_thrust_exit_velocity` has no production consumer. The only non-test use is the example.

The existing deep-time model grows crust by relaxing toward melt-column equilibrium and clamps the increment non-negative ([deeptime.rs:331](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:331)). Eruption must therefore consume or classify that same increment; it must not create another melt/crust source.

### Required wiring

A coherent integration needs:

1. Capture the positive per-column `crust_growth` increment before it is folded into thickness.
2. Partition that increment into intrusive and erupted mass/thickness.
3. Evaluate exit velocity from magma temperature, gas mass fraction, gas species/molar mass, chamber or fragmentation pressure, and ambient pressure.
4. Store an eruption event/result if it affects rendering, deposition, volatile transfer, or diagnostics.
5. Ensure intrusive plus erupted material equals the original melt-derived increment.

Temperature exists. Gas fraction/species and chamber/fragmentation/ambient pressures do not currently exist on the deep-time province path.

### File set

Smallest API/event-accounting slice:

- `crates/sim/src/deeptime.rs`

It can call the existing `civsim_world::eruption` function without changing that file.

Full production/render integration:

- `crates/sim/src/deeptime.rs`
- `crates/viewer/src/main.rs`
- `crates/viewer/src/render.rs` only if event-resolved eruption visuals are added
- `crates/world/src/eruption.rs` only if the existing velocity law is expanded into a larger eruption-result helper

### Smallest real slice

Refactor the melt step to expose one conservative production transaction—intrusive plus erupted equals `crust_growth`—and accept explicit eruption inputs. This lands a usable simulation API without fabricating volatile or chamber-pressure defaults.

### Blocker

Derived volatile inventory/partitioning and pressure state. The velocity law itself is not the blocker.

C(i) and C(ii) are coherent as a broad “impact and eruption surface events” programme, but they are poor single-agent scope: they have different physics and only share the viewer integration file.

---

## D. Crustal recycling

### What already exists

- The Airy law preserves the sign of a density inversion; it does not clamp a dense crust to zero elevation ([physics/geodynamics.rs:76](/home/nathan/Deep-Emergent-Civ-Simulator/crates/physics/src/geodynamics.rs:76)).
- `relax_to_support_bound` already performs bit-conservative lateral transfer of excessive crust to low columns ([deeptime.rs:721](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:721)).
- That function explicitly says crust denser than mantle should founder, but leaves it unchanged because delamination is outside its scope ([deeptime.rs:738](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:738)).
- The support-collapse function has no production consumer; it is tested but unread.
- The separate resident `GeodynamicColumn`/continuous mobility path is also not the path used by the viewer’s `DeepTimeState`.

### What a mass-conserving loop needs

At minimum:

1. Explicit crust and mantle/source areal-mass stocks, rather than only `crust_thickness_km`.
2. Melt extraction as a transaction: subtract mantle/source, add crust.
3. Recycling as the reverse transaction: subtract crust, add mantle.
4. A causal criterion:
   - density inversion/delamination for stagnant lids, and/or
   - stress/yield mobilization for mobile-lid subduction.
5. A rate or timescale; isostatic sign alone establishes instability, not how much founders per tick.
6. Density conversion and conservation invariants.
7. Eventually, composition transfer and feedback into solidus, density, and viscosity.

The existing crust-growth “finite source” is an equilibrium clamp, not a mass ledger: it does not subtract anything from a tracked mantle reservoir ([deeptime.rs:317](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:317)).

### File set

Smallest integrated ledger slice:

- `crates/sim/src/deeptime.rs`
- `crates/viewer/src/main.rs`, because every `DeepTimeState` constructor must initialize the new stock

Full first recycling mechanism:

- the two files above
- `crates/physics/src/geodynamics.rs` for a pure density-inversion/foundering law and tests

Later compositional recycling would additionally touch material differentiation/composition files, but that is not part of the smallest honest slice.

### Smallest real slice

Introduce a two-stock areal-mass ledger and make existing crust growth transact from mantle/source to crust, with an exact invariant test. Also wire the already-built lateral support collapse into the run path.

Do not add an arbitrary foundering rate in this slice. Once the ledger exists, an instantaneous dense-column-foundering limit or a sourced timescale can be added without redesigning state again.

### Blocker

The ledger itself is not blocked. Production destruction is blocked on a sourced recycling rate and, for mobile-lid subduction, B’s continuous stress/yield result. Compositionally faithful return is additionally blocked on mantle composition being mutable per province.

---

## Conflict matrix

This is for the **full proposed file sets**, not the deliberately disjoint kernel slices.

|  | A | B | C(i) | C(ii) | D |
|---|---|---|---|---|---|
| **A** | — | None | None | None | None |
| **B** | None | — | `viewer/main.rs` | `sim/deeptime.rs`, `viewer/main.rs` | `sim/deeptime.rs`, `viewer/main.rs` |
| **C(i)** | None | `viewer/main.rs` | — | `viewer/main.rs` | `viewer/main.rs` |
| **C(ii)** | None | `sim/deeptime.rs`, `viewer/main.rs` | `viewer/main.rs` | — | `sim/deeptime.rs`, `viewer/main.rs` |
| **D** | None | `sim/deeptime.rs`, `viewer/main.rs` | `viewer/main.rs` | `sim/deeptime.rs`, `viewer/main.rs` | — |

### Shared data and test ownership

- **B × C(ii):** both alter `DeepTimeState` stepping and read column temperature; both would edit the single inline `deeptime::tests` module ([deeptime.rs:1100](/home/nathan/Deep-Emergent-Civ-Simulator/crates/sim/src/deeptime.rs:1100)).
- **B × D:** B produces mobility/thermal state; D consumes it and adds reservoir state. Shared columns are `columns`, temperature/mobility, and the `step_deep_time` transaction.
- **C(ii) × D:** strongest semantic collision. Both need the same melt increment and crust/source mass transaction. They share `crust_thickness_km`, new reservoir fields, state digest/constructors, and `deeptime::tests`.
- **C(i) × D:** distinct physical columns—impact relief/craters versus crust/mantle stocks—but both edit the viewer’s province construction and stepping loop.
- **C(i) × C(ii):** no shared banked data column, but both replace/add parameters in the viewer’s deep-time setup.
- **A:** no shared production file, data column, or test module with B–D.

If D independently modifies `sim/geodynamics.rs` to invent its own mobility result, it would add another B×D collision. Avoid that: let B own mobility production and D consume it from `deeptime.rs`.

## Recommended order and parallel plan

1. **Run A immediately in parallel with everything.**
2. In a parallel kernel batch:
   - B’s three physics files: stagnant-lid scaling row/kernel.
   - C(i)’s `impact_flux.rs` + `smallbody.rs` residual-count bridge.
   - D’s ledger design/implementation can proceed if it exclusively owns `deeptime.rs` and `main.rs`.
3. Integrate **B into the run path**, serially owning `sim/geodynamics.rs`, `deeptime.rs`, and `main.rs`.
4. Integrate **D recycling after B**, so it consumes one canonical mobility result.
5. Integrate **C(ii) after D**, because eruption should use the final conserved melt/crust transaction rather than forcing that transaction to be refactored twice.
6. C(i)’s final viewer replacement can land whenever its late-accretion edge is ready, but its `main.rs` edit should be serialized with steps 3–5.

Safe concurrent pairs at full scope:

- A + B
- A + C(i)
- A + C(ii)
- A + D

Safe only as restricted low-level slices:

- B physics kernel + C(i) count bridge
- B physics kernel + D ledger
- C(i) count bridge + D ledger
- `world::eruption` extensions + any non-eruption lane

Not safe concurrently at full integration scope:

- B + C(ii)
- B + D
- C(ii) + D
- C(i) + any lane currently editing `viewer/main.rs`

## Value per effort

1. **A** — already designed, unblocked, fully isolated.
2. **B** — high systemic value because the current run applies a mobile-lid/isoviscous heat law to the catalogue majority; moderate sourcing and wiring cost.
3. **D** — high foundational value, but the honest ledger and recycling criterion are a larger state-model change.
4. **C** — C(i) has a valuable, cheap derivation slice but cannot honestly replace `40` yet; C(ii) is already kernel-complete and remains input-blocked. Within C, prioritize **C(i) before C(ii)**.
