# Parked legacy work

This tree is outside the canonical planet workspace. It contains biology,
civilization, and other work built under the retired methodology. Canonical
planet code may not depend on anything here; `scripts/planet_boundary_gate.py`
enforces that boundary through Cargo metadata.

Legacy compatibility crates may consume canonical crates one-way. Run parked
checks only with an explicit manifest, for example:

```sh
cargo test --manifest-path parked/Cargo.toml --target-dir target/parked
```

`just check-legacy` is the supported full compatibility gate. The parked causal
viewer keeps eight historical planet-construction expectations ignored because
they require fusion-volume evidence that is absent; its active replacement test
asserts the structured Gap Law refusal instead of supplying a value.

Compatibility package names are explicit: `civsim-physics-legacy`,
`civsim-units-legacy`, `civsim-world-legacy`, `civsim-materials-legacy`, and
`civsim-gpu-legacy`. They preserve old callers without re-exporting organism,
emic, celestial-fixture, disk-composition, or causal-viewer surfaces from the
canonical crates.

Archived project records live here too. `docs/` holds the retired design,
audit, research, and working briefs; `TODOS_LEGACY.md`, `ROADMAP_LEGACY.md`,
and `RUNBOOK_LEGACY.md` preserve the old operating queue and instructions.
Large session logs and superseded provenance plans, including
`MORNING_REVIEW.md`, `OWNER_DECISIONS_LOG.md`,
`DISK_ARC_BRIDGE_POST_197.md`, and
`PROVENANCE_PHASE2_FLOOR_UNIFICATION.md`, are retained under
`docs/working/` for archaeology. Their surviving abiotic obligations were
extracted to the active `docs/working/ABIOTIC_EVIDENCE_DEBT.md` before the
move. `MIDBAND_ARC_AUDIT_PACKET.md` is the commit-scoped audit snapshot of
the former geotherm and flexure arc; its surviving rheology, friction,
cohesive-energy, and flexure-evidence obligations were extracted to the same
active debt record before it was parked.

The former living-world MCP harness is retained under `tools/` for archaeology
only. None of these files is an active instruction, canonical specification,
planetary readiness receipt, or default developer entrypoint.

Moving a file out of this tree requires an explicit audit of its mechanism,
ledger inputs, provenance edges, and deterministic refusal behavior. Files are
not promoted merely because a legacy runner calls them.
