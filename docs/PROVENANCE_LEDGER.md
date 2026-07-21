# Canonical provenance ledger

The canonical ledger is an accounting graph for the abiotic planet and
stellar-system runpath. It is not a value store and does not grant admission by
tag, tier, citation, or owner choice. The runner accepts only an
`AbsolutePhysicsFloor` whose complete ordered graph matches the repository-owned
audited catalog.

The implementation lives in `crates/ledger`. The catalog lives in
`crates/planet/src/canonical/catalog.rs`. The checked-in central report lives at
`docs/working/CANONICAL_LEDGER_INVENTORY.txt`.

## Seven provenance marks

- `[D]` Derived: computed by a named law from named ledger ancestry.
- `[M]` Measured: refutable by observation without running the simulator.
- `[E]` Estimator: a bounded approximation over admitted evidence.
- `[C]` Closure: a model closure accounting class, barred from the initial floor.
- `[A]` Authored: a hand-authored magnitude, barred from the initial floor.
- `[W]` Written state: computed history generated within the causal run.
- `[X]` Contingency: a realization draw generated from an admitted measure by
  a versioned sampler and approved realization-coordinate law. A caller seed,
  fixed hidden seed, operating-system entropy, viewer state, world identity,
  transcript ordinal, or enumeration order cannot select the coordinate. If
  the coordinate law is absent, the stage refuses.

These seven marks are the complete canonical taxonomy. Audit sentinels such as
`unverified_measurement_candidate` and `unclassified` are fail-closed states,
not additional marks.

## Four tiers

1. Universal: measured irreducible fundamentals and values derived from named
   ancestry.
2. Reference: admitted reference evidence or compute-once results.
3. Residue: estimators and irreducible residual accounting.
4. Contingency: generated realization contingency, never a caller input.

The tiers and marks form a complete 4 by 7 accounting matrix. Empty cells are
reported as zero. A tier or mark never turns a magnitude into a legal input.

## Admission contract

Universal leaves must be `[M]`. Derived entries must be `[D]` and name their
complete ancestry. Every non-derived leaf at every tier may enter only after
derive-first exhaustion is recorded with all of the following:

- nonempty derivation attempts;
- a per-phenomenon Buckingham-Pi residual budget;
- Gap Law evidence with a typed Chaos Protocol: either not applicable with a
  basis, or a nonempty validity-regime partition and transition law. Every
  regime proves that input bands remain resolved for direct evolution or uses
  a derived stationary measure with conservation projection, stability checks,
  coordinate discipline, and exact replay when divergence is sub-resolution;
- Residual Law evidence;
- a unique residual slot.

The admission layer rejects `[A]`, `[C]`, caller-supplied `[W]`, caller-supplied
`[X]`, incomplete receipts, duplicate residual slots, and candidates outside the
repository-owned catalog. Evidence custody supports a receipt but never admits
the value by itself. When a required measure cannot derive and lacks complete
admission, the physical stage refuses.

There is no generic value-binding API. `AbsolutePhysicsFloor` exposes identities,
ancestry, and receipts only. Typed physical magnitude access must be
repository-owned and specific to the admitted identity.

## Central inventory

Regenerate the complete tier-by-mark report with:

```sh
just ledger-inventory
```

Check that the checked-in report matches the catalog with:

```sh
just ledger-inventory-check
```

The current physical-floor catalog contains three Tier 1 `[M]` invariants:
`fundamental.alpha`, `fundamental.G`, and `fundamental.m_e`. Every other matrix
cell is zero. Exact SI representation definitions are versioned separately and
carry no provenance tag; runtime-derived execution values do not masquerade as
floor inputs. The inventory generator emits each tag, count, tier total, and
stable member identity, so no hand-maintained summary can drift silently.

The retired calibration-era ledger specification is preserved at
`parked/docs/PROVENANCE_LEDGER_LEGACY.md`. It is historical evidence only and
does not define the canonical runpath.
