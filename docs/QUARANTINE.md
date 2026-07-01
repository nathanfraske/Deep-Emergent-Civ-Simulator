# Quarantine: the example harnesses are not canonical

The files under `crates/*/examples/` are **dev-fixture harnesses**. They exist to demonstrate and test a subsystem in isolation, and to do that they use **authored, dev-fixture numbers**: scaffolding seeds, `dev_default` calibrations, the dev syllable pool, hand-picked scenario values, and the like. They are not the canonical simulation, and their behaviour is not authoritative. Each carries a banner saying so.

This is the reserved-value discipline (design Principle 11, and the prime directive against fabricated values): an authored constant sitting in the path of world content is a defect until it earns its place. A dev fixture earns its place only as clearly-labelled scaffolding for a test or a demo, never as canon.

## The rule for the canonical runner

The canonical runner (the real, watchable simulation) is a different thing from these harnesses. It is:

- **Manifest-driven and fail-loud.** Every calibration it needs is a reserved value read from the calibration manifest (`calibration/reserved.toml`), which fails loud if the value is unset. There is no silent dev default on the canonical path.
- **Zero unapproved authored features.** No fabricated number, no `dev_default`, no scenario fixture reaches canonical state. A value is either owner-set in the manifest or surfaced as reserved-with-basis and fail-loud until the owner sets it.
- **A proper implementation, not an example.** It lives as library code plus a real binary, not under `examples/`.

Concepts may be carried over from the harnesses (the placement bridge, the tick shape, the glyph frame), but any authored number that came with them must be replaced by a reserved manifest value. Reusing a harness's *code path* is fine; reusing its *numbers* is not.

## The quarantined harnesses

`crates/sim/examples/`: controller_tax, conversation, dawn_band, dawn_world, living_world, naming_game, the_first_slice, tick_bench, two_bands, walkers. `crates/world/examples/`: map, snapshot, zoom_map. `crates/core/examples/`: diffusion_bench, mul_throughput (perf benches; non-canonical by nature).

They stay useful as subsystem demos and regression aids. They must not be mistaken for the game.
