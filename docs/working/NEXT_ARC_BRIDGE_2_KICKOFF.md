# Next-arc bridge kickoff (doc-only): what should the next arc be?

This is a doc-only bridge opened off current `main` (`d60ac35`), and its only purpose is to ask the gate what the next arc should be. No mechanism is authored and no value is moved.

## Where the last arc left things

R-UNITS-PIN is fully resolved and merged (#140). Its two residuals are built in `crates/units/src/emic.rs`: the exact-rational emic unit conversion (canonical-absolute storage, exact where the absolute scale resolves and within one absolute epsilon at the boundary, with the `StatedQuantity` exact-rational carry for a culture-stated quantity), the id-ordered `MeasurementSystem` unit store (the units-local instance of R-CANON-WALK), and the open-provenance `UnitOrigin` hardening that admits the alien as a data row. The open backlog count is 35, and the verification suite is clean.

## The ask

Gate, name the next arc. I am ready to ground it in the actual parts, run the input-audit for a generalization seam, and bring a slice plan before code. Some open items that are adjacent or high-leverage, for your selection rather than my choice:

- The remaining determinism-and-reproducibility cluster (R-CANON-WALK for its other containers, R-CMD-ORDER, R-REDUCE-ORDER, R-SAVE-SCHEMA, R-HARNESS-COVER, R-PROJ-REGISTER), several of which are dormant until the first parallelised phase.
- The anti-steering hardening items (R-DIM-HOMO, the dimensional-homogeneity load check; R-EVOLVE-STEER, the controller-evolution scoring-environment audit).
- The world-content registry items, the deeper R-DEEPTECH questions, R-LANG-TYPOLOGY, R-COMMS, or an item from the gaps-and-holes cluster.
- Or something already in your view that these do not name.

On your direction I begin. The cost directive holds unless you lift it: I self-audit and ask you to gate, no spawned panels or fleets. Whatever couples to a physics-floor value is surfaced reserved-with-basis and proven against the floor registry before it is set, never fabricated.
