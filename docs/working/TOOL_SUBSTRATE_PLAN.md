# Tool substrate: the complete list and the derive-from-physics plan (2026-07-07)

The owner's rule for this work is absolute: the PHYSICS SUBSTRATE (the material axes and the physics
laws) is the ONLY place authored data may live. Everything a tool does, produces, or becomes MUST be
derived from it. No exceptions. This document is the complete list of where the tool system violates
that rule or leaves a physics law unused, from a six-agent completely-blind sweep (three authoring
lenses, three gap lenses, each fed only the current tool surface), and the physics-derived fix for
each, broken into implementable sections. The "physics is the only authored place" rule is a standing
audit seat on every panel; the arc-end blind audit carries it.

## The two structural roots (every gap agent landed here)

- ROOT R2: `WieldedTool { contact_area, substance }` is too thin. It presents only `mech.contact_area`
  to the capability dispatch (every other geometric axis reads zero), stores no volume, mass, length,
  or wear state, and is an inline fungible value with no identity. This one limitation blocks wear,
  breakage, impact/percussion, leverage, mass, thermal coupling, composite/hafted tools, re-work, and
  provenance. It is the load-bearing fix.
- ROOT R1: the `CapabilityKernel` set is a closed enum at Pierce (plus the Locomote and Refract laws).
  No shear, impact, lever, wear, bend, or friction kernel exists, so no affordance can require those
  laws, so the entire non-piercing action space (sever, crush, lever, abrade, strike, pry, bore, tear)
  is inexpressible. The affordance system can only express pierce-gated actions.

## The sections (dependency order)

### A. Purge the authored cut output. STATUS: coded, pending falsifier + verify.
Violation (all three authoring agents, CRITICAL): `cut_yields` authored which targets are cut-openable
(a closed map), how much was freed (a fraction), and which substance appeared (a named 1:1
transmutation). Fix (done): `cut_underfoot` now gates PER CONSTITUENT of the cell's own `SubstanceMix`
(sever each substance whose own `mat.fracture_strength` the edge's effective pressure beats), frees the
strength-bounded carry (the grasp's own bound), and takes the cell's own matter. No map, no fraction,
no deposit. CUT is now distinct from EXTRACT (which gates on the aggregate hardest constituent), so a
keen edge frees the soft flesh from a tough composite EXTRACT cannot.

### B. Derive the crafted tool's edge from the worked material. STATUS: next.
Violation (all three, CRITICAL/HIGH): `craft_from_carried` gives every tool the same reserved
`edge_area`, so obsidian and sandstone come out equally sharp, and the material chosen is the first by
id order, not by fitness. Fix: the finest edge a material holds is the contact area at which the being's
forming pressure equals the material's own fracture strength (below that the tip crumbles), so
`edge_area = forming_force / (fracture_strength * bridge)` through a physics inverse of
`contact_pressure`; a harder, tougher stone derives a finer edge. Choose the carried substance that
maximises the resulting edge capability, not the id-first one. Both derived, no constant.

### C. Enrich WieldedTool as matter (root R2). STATUS: planned.
Store the tool's retained VOLUME (set at craft from the shaped stock) so `mass = mat.density * volume`
is recoverable (the struct comment already claims mass is derived; today it is not) and wear can
decrement it. Add the tool's characteristic LENGTH/geometry so the lever and bending laws have an arm to
read. Update the hash fold and constructors. This is the enabling change for D, E, G-impact, and lever.

### D. Tool wear (the `wear` law, listed but unwired). STATUS: planned, needs C.
Each use widens the tool's contact area and removes volume by the `wear` law (contact pressure, sliding
distance, tool-to-target hardness ratio), so a blunt tool cuts less and a spent tool must be reworked or
replaced. Sharpening is `wear` applied to shrink the edge again. Derived, no durability constant.

### E. Tool breakage and failure (root laws on the tool's own body). STATUS: planned, needs C.
The tool's own body carries reaction stress; `fracture_onset` (brittle chip or snap), `bend_stress`,
`buckling`, `axial_stress`, and `shear` decide whether it survives the load it imposes. A brittle edge
pried sideways or a slender haft overloaded fails, so the hardness-versus-toughness tradeoff (the central
tool-material choice) finally bites. Derived from the tool material's own strength axes.

### F. Capability-kernel expansion (root R1). STATUS: planned, compose-crate.
Add the missing `CapabilityKernel` variants (shear, impact, lever, wear, bend, friction, crush) so an
`AffordanceDef` can require the law its action needs, opening the non-piercing action space. The kernel
SET stays fixed Rust; which affordances a world declares is data.

### G. New tool actions derived from F + C. STATUS: planned, needs F and C.
Sever/divide (shear through a cut depth), strike/percussion (the `impact` law over the tool's mass and
swing energy, which also is how knapping physically happens), lever/pry (the `lever` law over the tool's
arm), crush (compressive strength), abrade (wear as an action), bore (hertzian contact). Each outcome
derived from the tool's and target's physics, no per-action table.

### H. Composite and hafted tools. STATUS: planned, deepest.
A tool of more than one material (a hard head bound to a tough shaft), each part with its own axes and
geometry, the bind a physical joint (`shear`, `friction`). Craft consuming a `SubstanceMix` or another
tool as stock. This is the composition stage of the made-world arc.

### I. Tool identity, provenance, and thermal coupling. STATUS: planned, deepest.
A tool as a distinct object with a `StableId` (the object-identity rider) so it accrues state (wear,
temperature, damage, provenance) across hands. Thermal coupling: friction heats, quench raises
`mat.indentation_hardness` (feeding straight back into the cut cap), fire consumes a wooden tool,
a fluid corrodes a metal one. Each derived from the physics substrate, with any new axis (thermal
conductivity, corrosion rate) added as data, the only authored place.

## Lower-severity authoring to fold in as the sections touch them
- `pressure_max` is a second, substance-blind pressure ceiling; the material cap (`min(tool_hardness)`)
  is the real one. Keep `pressure_max` only as a fixed-point overflow guard set far above any material
  value, never a behavioural limit.
- The bare-being extraction uses a reserved `working_area` and never self-blunts; derive its area from
  the acting body part's own geometry and cap at the part's own hardness (an anatomy-arc follow-on).
- CUT is bound to PIERCE because no shear kernel exists (root R1); once F lands, gate CUT on a shear or
  cut kernel against a present target's resistance rather than Pierce and a target-blind `min_capability`
  floor.
- The affordance-percept kind set is a closed enum where a data registry would sit (sensing, low
  severity).

## The arc-end audit
One deep blind panel over the whole reworked tool substrate, with the standing physics-only seat, told
to find any surviving authored decision and any physics law still left unused where a tool action should
consume it. Verify every finding against source before acting.

## Arc-CHECKPOINT audit (2026-07-07, on Sections A, B, C): verdict and applied fixes

A two-agent blind panel (physics-only seat plus correctness; determinism plus crash) verified the
reworked cut, craft, edge_area_at, and mass. Confirmed genuinely fixed: the cut_yields map is gone (the
severable set derives from the cell's own constituents), the fixed edge_area is gone, edge_area_at is
the exact algebraic inverse of contact_pressure with no crash, determinism and opt-in byte-neutrality
both hold. Applied on their verified findings: mass now uses checked_mul with a saturating fallback (was
an unchecked mul, a debug panic / release wrap and a cross-build hazard); the craft fitness now ranks
stones by the derived cutting power min(fracture_strength, indentation_hardness) rather than by an
argmax over the Pierce capability (which mis-ranked a cutter and, keyed to ID_PIERCE, authored "this is
a piercer"; the power form is also precision-safe, sidestepping the tiny-area rounding cliff at the
hard-material end); and the cut's pressure cap is now a fixed-point overflow guard (Fixed::MAX, never
binding since a wielded tool has a positive area) rather than a pressure_max borrowed from
ExtractionParams, so the cut reads no authored ceiling and is decoupled from the extraction subsystem.

THREE physics refinements the checkpoint surfaced, real and NOT yet fixed (they need the deeper substrate
sections, so they fold into B, D/G, and F rather than blocking A/B/C):

- REFINEMENT R-EDGE-INTRINSIC (folds into B, HIGH): the crafted edge is derived from the CRAFTER's
  forming force (edge_area = force / (fracture * bridge)), so at USE the same force cancels and the cut
  pressure is identically the tool's own fracture strength, independent of how hard the wielder presses;
  and across beings the cut power depends on the ratio of the wielder's force to the maker's, a
  dependence on the maker with no physical basis at use time. The physically correct edge is an INTRINSIC
  sharpness the material holds (a fracture-mode or grain LENGTH SCALE), derived from the material's own
  axes and independent of crafter force, so force/area stays a real function of the wielder's current
  force. This needs a material edge-length-scale axis (a new physics-substrate axis, the only authored
  place, e.g. a grain size or a fracture-toughness that yields a critical-flaw length), then re-derive
  contact_area from it. The current force-based edge is a working first cut; this is the correct form.
- REFINEMENT R-CUT-DEPTH (folds into D/G, HIGH): the cut sizes the freed amount by the constituent's
  whole cell volume, carry-bounded (pick_up), rather than by the depth of cut. cut_penetrate (built and
  cited) sizes a stroke from the delivered energy over the material's specific cut energy and the swept
  edge area; the correct form is depth times edge width times stroke length, then carry-bound the freed
  portion. Needs a mat.specific_cut_energy axis and a stroke/reach quantity. The current carry-bounded
  amount is a coarse stand-in.
- REFINEMENT R-CUT-SHEAR (folds into F, MEDIUM): the sever gate compares a NORMAL contact pressure to
  mat.fracture_strength, but cutting is a shear / new-surface-energy process; the correct resistance is a
  shear strength or fracture toughness with an energy term. This needs a shear capability kernel (root
  R1, Section F) so the cut gate reads a shear law rather than a normal-stress proxy. The current gate is
  a crude but honest initiation proxy.

These are recorded so the tool physics is not mistaken for complete: A/B/C purged the AUTHORING and
stood up the tool-as-matter foundation, and the derivations are sound as far as they go, but the edge is
force-derived rather than intrinsic, the cut amount is carry-bounded rather than depth-sized, and the
sever gate is normal rather than shear. Each is folded into its section above.
