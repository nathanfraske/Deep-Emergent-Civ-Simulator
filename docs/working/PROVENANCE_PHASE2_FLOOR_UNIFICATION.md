# Provenance register Phase 2: the floor unification (design-first opener)

Phase 1 made the seven-tag provenance register mandatory and machine-enforced over `calibration/reserved.toml` (228 values, honesty number 205, merged at `45c510b`). Phase 2 extends that one register across the rest of the project so there is a single provenance axis and a single honesty number, not two checked side by side. This opener grounds the work against the actual substrate and scopes the slices; it authors no value and moves no pin. The gate gates per push.

## Where the two provenance systems stand today

The calibration register (Phase 1) carries the seven owner tags in `crates/sim/src/calibration.rs`: `[D]` derived, `[M]` measured, `[E]` estimator, `[C]` closure, `[A]` authored, `[W]` written-state, `[X]` contingency, with the worst-case DAG join, the `derived_from`/`inputs` two-field rule, the born-provenance `#[test]`, `scripts/provenance_gate.py`, and the `authoring_surface` honesty query.

The physics floor (Part 58) carries an older, coarser two-tag provenance in `crates/physics/src/lib.rs`: `Provenance::RealWithSource(citation)` and `Provenance::FantasyReserved(basis)`, read from each floor-manifest entry's `real = "..."` or `fantasy = "..."` field by `provenance_from`, which fails loud with `MissingProvenance` when neither is present. So the floor already enforces a mandatory real-versus-fantasy split; it is a two-tag register, not an absent one. The floor manifests are the nine `crates/physics/data/*.toml` files, carrying about 235 real/fantasy entries (the periodic table alone holds 92); `phase_registry.toml` uses a different schema and carries no `real`/`fantasy` line, which is a seam this opener flags rather than assumes.

## The unification: two tags into seven, by the ground the entry already states

The map is a refinement, never a re-authoring. Each floor entry keeps its `real`/`fantasy` string; the seven-tag value is DERIVED from which field it carries and what that field says, so the floor's own recorded ground decides the tag rather than a fresh judgement.

- `RealWithSource(citation)` refines to `[M]` measured when the citation is an observed datum with error bars (a datasheet density, a CODATA constant, a handbook measurement), and to `[D]` derived when the citation is a law that computes the value from other floor quantities. The discriminator is the citation's own content: a measurement pins a leaf, a derivation names inputs (and under the Phase-1 two-field rule it must declare a non-empty `derived_from`).
- `FantasyReserved(basis)` refines to `[A]` authored when the basis is a hand-picked magnitude, and to `[C]` closure when the basis describes a free knob whose turning changes outcomes without contradicting a measurement. Both are on the authoring surface, so this split records which KIND of authored value it is, not whether it counts.

The refinement is data-driven and per-entry: the mechanism is fixed Rust (a mapping over the two-tag plus a classification of the citation/basis), the membership is the floor manifests, and it grows with the world. Where a floor entry's ground is truly ambiguous between two tags, it surfaces as unsettled with the reason stated, exactly as the calibration blind pass did, never laundered into the flattering tag.

## What Phase 2 builds, in slices, each byte-neutral and gated

1. **The floor-provenance gate.** Extend the enforcement from `calibration/reserved.toml` to the nine floor manifests and the material/substance registry: a structural CI gate (a sibling of `provenance_gate.py`) plus a born-provenance `#[test]` over the floor loader, so a floor entry that ships without a resolvable seven-tag provenance fails the build. This resolves the `phase_registry.toml` schema seam (either it carries the field or its provenance derives from the substances it references).
2. **The two-tag-into-seven refinement.** The mapping above, applied per floor entry from its own `real`/`fantasy` ground, with the category-provenance consistency gate extended to the floor. Byte-neutral: the tags are accounting metadata read nowhere on the run path, as Phase 1 proved for the calibration side.
3. **The three consumer-side and form-side rules, as gate extensions.**
   - The exponential-consumer escalation: an `[E]` estimator is forbidden in an exponent (an estimator's error band, exponentiated, is unbounded), so a consumer that raises a value to a power requires that value to be `[M]` or `[D]`, not `[E]`.
   - The disposer resolution-ladder: a disposer (a comparison that selects among candidates by energy) may only discriminate at its energy model's resolution, so an `[E]` estimator whose error band exceeds the decision gap must escalate up the provenance ladder or resolve as a seeded contingency draw.
   - The `[D]`-closed-form re-evaluation: a `[D]` value with a closed form stores the FORM and recomputes it rather than hand-copying the number, so a transcription error fails the build. This pairs with the `derived_from` edges Phase 1 landed: the edge names the inputs, the form recomputes from them.
4. **The full-floor honesty number.** Surface `authoring_surface` over the unified register (calibration plus floor, about 463 entries), the single honest count of world-content values resting on set-points a laboratory could not refute without running the sim, with the same effective-DAG-join and unsettled discipline as Phase 1.

## Constraints and open seams for the gate

The whole arc is byte-neutral (the tags are inert metadata), proven by the five pins on every push. Each slice grounds against the merged substrate, runs the verification suite, and is gate-checkpointed; the section-9 blind panel runs at the arc boundary. No value is authored: a floor tag is derived from the entry's own recorded ground, and a genuine gap surfaces as unsettled with its reason.

Two seams surfaced by the grounding, neither silently resolved:

- **The `phase_registry.toml` schema.** It carries no `real`/`fantasy` field. Either it gains one per phase, or its provenance derives from the substances it composes (a phase is a state of a substance whose properties are themselves tagged). The second is the more emergence-faithful reading and pairs with the material-registry unification; flagged for the gate's call.
- **The material/substance registry versus the floor.** The gate's Phase-1 note recorded an authoring-defect in the material registry (bulk properties like a granite density authored on the substance rather than derived from its mineral or element components). The floor unification tags those bulk rows as `[A]`/`[C]` by their own basis, which makes the defect COUNTABLE in the honesty number rather than hidden, and sets up the materials-substrate buildout (the arc after Phase 2) to relocate the measured floor down to the components. Phase 2 tags and surfaces the defect; the materials buildout resolves it.
