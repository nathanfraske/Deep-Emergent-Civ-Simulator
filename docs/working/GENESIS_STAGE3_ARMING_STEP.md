# Genesis Stage 3 arming step: the design-first opener (the surface substrate goes live)

This is the design-first opener for the arming-step arc, the bridge PR the gate asked for so the merge of the
Stage-3 surface substrate (PR #160) does not strand the watch. It is grounded against current main now and will be
re-grounded against the merged substrate once #160 lands (the four-reservoir ledger, the driver-row contract, the
snapshot-apply reconciliation, the four core drivers, and the isostatic relaxation all arrive with that merge). The
arc moves the world from a dormant interior toward a coupled running early Earth: the drivers stop being
byte-neutral kernels and begin carving a real armed scenario.

The arc has three pieces, sequenced so no hardcoded value ever fires on the run path. Pieces 1 and 2 close the two
design seams the section-9 blind panel surfaced on #160 (both ruled by the gate); piece 3 arms Mirror against a
cited bulk-silicate-Earth composition and is where the substrate goes live and re-pins.

## Piece 1: the reconcile-honored composition contract (unified with the redistribution lane)

The seam the panel caught, and that Agent C reached independently from the redistribution side on #174, is a single
architectural boundary: "each piece conserves, so the composition conserves" is false at `reconcile_column`. A
source driver bounds its entrainment by the slope drop and routes that full DEMANDED mass downstream, while
`reconcile_column` clamps a contested column's honored removal to the snapshot mass it holds. So the downstream
sinks (the deposition, the reservoir transfers, and C's fan destinations) receive more than left the source, and
the ledger gains the difference as fabricated mass. C hit the same shape: the redistribution operator credited its
destinations the full demand `M` while reconcile honored only `H <= M`, fabricating `M - H`.

The contract is one form, stated once, honored by both the surface arming step and C's redistribution coupling:
reconcile the removals against the tick snapshot FIRST, then key EVERY downstream sink write off the honored
removal, never the raw demand.

The exactness refinement matters and is the load-bearing detail. The downstream is NOT keyed off a per-writer
honored FRACTION multiplied into the raw sinks: a fixed-point fraction-multiply rounds, so the rescaled sinks would
not sum to exactly the honored removal, and the leak would reopen at the rounding scale. Instead the downstream is
keyed off the honored ABSOLUTE removal `H`, and where that honored mass splits across several sinks or destinations
it is split with the same exact-integer largest-remainder `apportion` the reconciliation already carries, so the
sinks sum to exactly `H` with no drift. For a single-sink driver (entrain, then deposit) the contract collapses to
"deposit the honored removal," exact by construction: the deposition source is the honored removal, not the raw
entrainment, so total deposited equals total removed. For a fan across destinations it is `apportion(H, weights)`,
which is precisely the redistribution form C is building. The two lanes reference the one contract rather than each
carrying a private copy.

The determinism obligation the contract inherits: the apportionment is the exact-integer, order-independent split
already proven for the reconciliation (Principle 3, Principle 10), so the composed source-reconcile-sink pass is
conservative and worker-invariant under fixed-point arithmetic. The arming step demonstrates it with a composition
test that the current suite lacks: a contested column with several removing writers, run through the drivers plus
the reconciliation plus the sinks, asserting the ledger neither gains nor loses mass across the whole pass.

## Piece 2: the driver-row exact-root exponent, data drawn from the GPU-canon-buildable family

The stream-power incision exponents (the area exponent `m` and the slope exponent `n`) are physical parameters of
the incision law that differ by fluid and process, so the pair `m = 1/2`, `n = 1` fixed in the kernel is a Terran
fluvial default sitting in the path of world content, a Principle 11 and admit-the-alien defect the moment a
world's fluid differs. The fix hardens the exponent the way the value substrate and the semantic substrate are
hardened: the mechanism stays fixed Rust, and the exponent becomes DATA on the driver row, drawn from the
exact-root family that is GPU-canon-buildable today (`m` in the set the exact roots reach, a half power via the
exact integer `Fixed::sqrt`, a first power via the linear term, a cube root by composition). The general
arbitrary-exponent membership is the deferred GPU-canon primitive (the gate's task #45), which later EXTENDS the
membership without touching the kernel, the same fixed-mechanism / growing-data shape the other substrates use.

The default `m = 1/2`, `n = 1` is reserved-with-basis as the Mirror fluvial default (the standard detachment-limited
stream-power values), a per-world and per-driver datum rather than a global authored constant, surfaced with its
basis and never fabricated. The field is byte-neutral by construction: a driver that reads the default reproduces
the current kernel exactly. It is sequenced BEFORE the drivers go live, so no hardcoded Terran exponent ever fires
on the run path; the exponent is a world datum from the first armed tick.

## Piece 3: arm Mirror as bulk silicate Earth, the producer that fills the isostatic elevation

With the composition contract and the exponent field in place, the substrate is armed against a cited composition.
The producer is Mirror's bulk silicate Earth (McDonough and Sun 1995), the world's mantle-plus-crust major-element
chemistry, which flows through the Layer-0 petrology density kernel (composition to the stable mineral assemblage by
free-energy minimization, to the assemblage density) and the Airy isostasy law to the `isostatic_elevation` target
the relaxation reads. The crust floats at the freeboard its own chemistry sets, and the surface transport drivers
carve the relaxed terrain. This is the step where the drivers stop being byte-neutral: an armed genesis scenario
runs the transport, so the living pin re-pins on that scenario, stated and measured on its own when it comes, while
the other canonical pins that do not run a genesis pass stay byte-identical.

The reserved values the producer needs are surfaced-with-basis and never fabricated: the bulk-silicate-Earth
major-element abundances (cited to McDonough and Sun 1995), the mantle reference density for the Airy flotation
(the physics floor's mantle density), and the seed crustal thickness (a per-world datum). None is set here; each is
surfaced with the ground on which the owner would set it, read from the calibration manifest, failing loud on an
unset value.

## The slice sequence and the verify plan

The arc proceeds in slices, each grounded against the merged substrate, each verified with the full suite (the five
pins with the stated genesis re-pin measured on its own armed scenario, clippy strict, fmt, the constructor gate,
the floor registry, and the prose customs), and each gate-checkpointed. Pieces 1 and 2 are byte-neutral (the
composition contract corrects the composed write, and the exponent default reproduces the current values), so they
land clean alongside the merge. Piece 3 is the stated genesis re-pin. The full section-9 blind panel runs again on
the armed substrate at the arc boundary, per the owner's standing requirement, with both restored shaping-catcher
lenses and the correctness lenses, every finding verified against source before it is trusted.

The honest limits carried forward from the substrate stand and are not hidden: the general arbitrary-exponent
fractional power is the open GPU-canon gate (piece 2 opens the exact-root family, #45 extends it); the non-local
redistribution primitive does not exist, so impact and mass-flow remain deferred; the heightfield is a surface
projection, so conduits, overhangs, ballistic arcs, and buried stratigraphy are not modeled volumes; the build order
is Earth-frequency-ordered, so cold-volatile, airless, and lava worlds are not correctly simulable until their
deferred rows land; and the per-cell forcing fields the panel flagged (a climate-derived runoff for the
discharge proxy, a solvent-availability gate for dissolution) are heterogeneous-world extensions the arming step
carries as uniform defaults until their cross-lane inputs are wired.
