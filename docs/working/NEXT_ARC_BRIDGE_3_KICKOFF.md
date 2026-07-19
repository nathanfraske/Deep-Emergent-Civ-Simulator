# Next-arc bridge kickoff (doc-only): my #43 lane is fully delivered, what is my next slice?

This is a doc-only bridge opened off current `main` (`0c38821`), and its only purpose is to report a supersession and ask the gate for the next disjoint genesis-forward slice. No mechanism is authored and no value is moved.

## What changed since the sign-off

You signed off my task #43 slices 1 and 2 on #158, merged it, and directed me to build slices 3 and 4 on a fresh branch off the new main. Grounding against the actual new main first (Prime Directive 1) shows that lane is already fully delivered, so building it now would duplicate merged work:

- `#168` (`7796370`, Agent B, "completing C's stranded #43 lane"): slice 3 (the five-state `Coverage` signal, `coverage_report` over the registry in canonical order, only `Dead` failing CI) and slice 4 (the remaining probes plus the CI harness). B's PD2 audit also hardened three real data seams in my slice-2 registry, all verified at source: `hydrology_water` declared the RETIRED `saturation_t_ref` (repointed to the live `precipitation_rate`), `world_time_cadence` pointed at a `celestial.rs` passthrough identity (repointed and differentiated into the day-over-rotation vs the year-over-orbit cadences), and `decomposition_recovery` named a phantom manifest key (its rate is a `DecomposerDriver` driver-param). Good catches; the registry is stronger for them.
- `#169` (`0c38821`, Agent B, task #46): the broadening past the retired-floor-plus-manifest-key shape to ANY derived output (`DerivationCategory`) and ANY input source (`InputSource` {ManifestKey, DriverParam, ResidentField}), wiring the two proof cases (`column_convection` resident-field, `decomposition_recovery` driver-param) LIVE.

I verified the delivered state green on this base: 13 `derive_gate` unit tests and 3 `derived_output_live` cross-check tests pass, and CI ran green on both merges. Task #43 (all four slices) and task #46 are complete. Nothing on this lane is mine to build.

## The ask

Gate, name my next disjoint genesis-forward speedup slice. My recent lanes are all merged (G the 7th CODATA fundamental #161/#850efcd; the Layer-0 determinism primitives plus provenance-DAG accounting #162; task #43 slices 1 and 2 #158). I am ready to ground the next slice in the actual parts and the floor registry, run the input-audit for a generalization seam, and bring it self-audited before you gate.

Some genesis-forward pieces that look adjacent to my landed work, for your selection rather than my choice:

- The convection/Stokes KERNEL wiring into `laws.rs` that my `fixed_cap_solve` outer loop was built to host (the thin caller I flagged for B/A when the interior lane resumes; #167 landed the convection-evolution subsystem, so a `GeodynamicColumn`-contract caller may now be groundable).
- Extending the provenance-DAG accounting onto a live consumer (a `validate_provenance` CI surface, or classifying an existing manifest region's provenance edges), building on #162.
- The `label.rs` connected-components / `flood.rs` priority-flood primitives' first real consumer in the geology/hydrology path, if a disjoint one exists that does not collide with A's or B's active files.
- Or a units-crate or floor slice already in your view that these do not name.

Do not let me duplicate or step on the derive-gate lane (B's, done) or Agent A's genesis files. On your direction I begin. The cost directive holds unless you lift it: I self-audit and ask you to gate, no spawned panels or fleets. Whatever couples to a physics-floor value is surfaced reserved-with-basis and proven against the floor registry before it is set, never fabricated.
