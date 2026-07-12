# Mirror hydrology correctness fix: ground moisture into the Dalton vapour port (arc kickoff)

This is the design-first kickoff for the next arc the gate queued after the surface-energy-balance arc (#139) landed: a Mirror hydrology correctness fix a biosphere subagent surfaced. It establishes the comms bridge and states the scope. No code lands until the gate gates the grounded design, which follows on this PR.

## The bug (confirmed at source)

`environ.rs` `step_hydrology` calls `laws::evaporation_rate(moist, e_s, ...)`, passing the cell's `moisture` field into the kernel's `e_ambient` port, the ambient VAPOUR PRESSURE the floor binds it to. But `moisture` is a GROUND-moisture content, a different physical quantity from an ambient vapour pressure, so the Dalton vapour-pressure deficit `e_s - e_ambient` is being formed as `e_s - moisture`, a substance-quantity mismatch. The precipitation branch (`excess = moist - e_s`) shares the same conflation.

## The derive-first fix (to be grounded and detailed in the design that follows)

Couple the ground moisture to an ambient vapour pressure through the physics, rather than feeding the moisture content directly into the vapour-pressure port. The design will ground the correct coupling (a ground-moisture-to-ambient-vapour relationship, so the Dalton deficit `e_s - e_ambient` is formed over two like quantities, both vapour pressures) against `environ.rs` (`step_hydrology`), `laws::evaporation_rate` / `saturation_vapor_pressure`, and the `moisture` field's own definition and units, and confirm what the precipitation branch should read.

## Byte-neutrality and pins

Fixing the coupling changes the evaporation (and precipitation) the hydrology computes, so it is a STATED change on any pinned scenario that runs the hydrology, reported with its reason, never a silent drift. The design will name exactly which pins the fix touches (including `living`, whose latent surface cooling reads this evaporation) and whether any hold. No value is tuned to a target.

## Substrate-first and discipline

Before authoring or flagging any value owner-set, the physics floor registry is read and it is proven in writing that no substrate derives it. The coupling is a derivation from the moisture field and the floor, not an authored number; where a substrate is missing the honest output is to build it, not fake a value. Design-first: the grounded coupling and the pin impact are posted for the gate's design-gate before code. Section-9 is run once by me per slice (the standing cost directive), and the gate audits.
