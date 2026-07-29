# Broadening the derived-output-is-live gate (task #46): any derived output, any input source

Agent B, task #46, gate-ruled off main `7796370`. Design-first: this is a mechanism change to the
registry and the probe. The build is tooling and tests only, byte-neutral on all four pins.

## The two generality limits, both surfaced during #43

The gate built and merged the derived-output-is-live gate (C's slices 1+2, my slices 3+4). Wiring it
exposed two places where the mechanism is narrower than the liveness principle it enforces:

1. **Input source.** The registry declares each derivation's perturbable input as a manifest key
   string, and the probe perturbs `manifest.require_fixed(key)`. But `decomposition_recovery` derives
   its output from a `DecomposerDriver` parameter (`biomass_reference`, a data-defined driver-row
   value), not a reserved manifest scalar, so there is no manifest key to declare (the phantom
   `decompose.decomposer_rate` was the symptom). The liveness principle (perturb the input, assert the
   output responds) applies regardless of where the input lives.

2. **Derived-output scope.** The registry's type is `RetiredFloorDerivation`, and its whole premise is
   a derived value that REPLACED an authored floor. But `column_convection` (the convection subsystem
   I built) is a derived output that retired no floor: it is new physics, so it carries a bare
   `@derives:` to stay off the retired-floor cross-check. A dead new-physics derivation (constant
   under its input) is the same class of bug as a dead retired-floor one, and the gate cannot see it.

The gate ruled the broadening: the liveness principle applies to ANY derived output regardless of its
input source, so generalize the gate to cover any derived output and to perturb any input source.

## The mechanism change

The pass/fail core (`assess`: two site-local readings differ = Live, identical = Dead) is already
input-agnostic, so the broadening is in the membership shape and two proof probes, not the verdict.

**Generalize the derivation type.** `RetiredFloorDerivation` becomes `Derivation`, carrying a
`category` (`RetiredFloor` or `NewDerivation`), so the registry admits any derived output while
keeping the retired-floor class named (it is the soil_baseline-bug class, worth distinguishing). The
registry and its canonical membership rename in step (`DerivationRegistry`), and the two test files
that import them follow.

**Type the input source.** The row's `inputs: Vec<String>` (manifest keys) becomes
`inputs: Vec<InputSource>`, an enum over the three places a derivation's input can live:

- `ManifestKey(String)`: a reserved manifest scalar, perturbed via `require_fixed` (the current path,
  unchanged for the manifest-key derivations).
- `DriverParam { driver, param }`: a named parameter on a data-defined driver row (the
  `decomposition_recovery` case: the `biomass_reference` on a `DecomposerDriver` Life row).
- `ResidentField { holder, field }`: a field on a resident simulation-state struct (the
  `column_convection` case: a `ColumnParams` field such as `heat_production`).

The declared input source stays honest (it names what the probe perturbs), and the coverage
signal reads the source kind so a driver-param or field input is never mistaken for a missing manifest
key. `register` still forbids an empty input list.

**Wire the two proof cases**, each the flagship for one broadened axis, following the
`probe_carbon_fixation_rate` two-point pattern:

- `decomposition_recovery` (DriverParam): build a `DecomposerDriverRegistry` with a Life row at a base
  `biomass_reference`, read `activity_at` at a fixed life-stock where the Life factor is binding, then
  rebuild at a perturbed `biomass_reference` and assert the activity responds. This turns the phantom
  row into a real probe.
- `column_convection` (ResidentField): convert `convection_step`'s bare `@derives:` to
  `@derives[column_convection]`, add its registry row (category `NewDerivation`), and probe it by
  perturbing a `ColumnParams` input (`heat_production`) and asserting the stepped column temperature
  responds. This proves a non-retired-floor derivation can be covered.

The coverage signal and the CI harness carry over unchanged: they walk the registry and classify each
row, so the two new probes read Live and drop out of the Unwired list automatically.

## Scope and follow-on

This slice generalizes the mechanism and wires the two proof cases the gate named. It does NOT sweep
every remaining bare `@derives:` into coverage (for example `secular_step`): that broader sweep, one
row and one probe per bare annotation, is a follow-on now that the mechanism admits them. The coverage
signal reports the still-bare ones as off-registry, so the gap stays visible.

## Byte-neutrality

The whole change is the gate module (`derive_gate.rs`), the two test files, and a comment-only
annotation change in `geodynamics.rs` (bare `@derives:` to `@derives[column_convection]:`). None of it
sits on the canonical run path (the gate reads run state and asserts, it never feeds the sim), so the
four pins hold byte-identical. Stated, not assumed; the CI run confirms.
