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

### D. Tool wear (the `wear` law, listed but unwired). STATUS: DONE (commit b16608a).
Each use removes volume by the Archard `wear` law (the tool material's own `mat.wear_coefficient`, the
being's force, the stroke distance, the tool's own indentation hardness), so a tool that works matter
spends out and, once worn below the craft-minimum volume, is a nub and is unwielded. Wired into the CUT
and EXTRACT dispatch gated on positive work; `wear_tool` no-ops for a bare-handed extract and when the
wear params are unarmed, so an opted-out world is byte-identical. The coefficient's storage scale is
derived from the axis (`QuantityAxis::storage_scale` over the canonical `x1e6` convention), not a
constant. Reserved-with-basis: the stroke distance and the worn-volume ceiling (`WearParams`). Falsifier
`a_worked_tool_wears_down_and_spends_out_over_repeated_use_and_an_unarmed_tool_is_immortal`.
NOT yet done (folds into G, R-CUT-DEPTH): the wear widens the CONTACT AREA (blunting), and sharpening as
`wear` applied to shrink the edge again. Today wear removes volume and spends the tool out; the
area-blunting and re-sharpening cycle needs the edge-as-state carried on the tool (deeper, with I).

### E. Tool breakage and failure (root laws on the tool's own body). STATUS: DONE (stress limb) (commit pending).
The tool's own body carries the reaction stress of its working force over its edge
([`laws::contact_pressure`]); the brittle-fracture criterion ([`laws::fracture_onset`]) snaps it when that
stress exceeds the tool material's own `mat.fracture_strength`, so the hardness-versus-fracture-strength
tradeoff bites: a hard edge that imposes a high cutting stress must survive it, and a brittle low-strength
edge shatters where a tough one bears the load. Wired into the CUT and EXTRACT dispatch, checked before
wear (sudden before gradual), opt-in via `set_breakage`, byte-identical unarmed. Fully derived, NO reserved
value. Falsifier
`a_brittle_tool_snaps_under_its_own_working_stress_where_a_tough_one_survives_and_an_unarmed_tool_never_breaks`.
NOT yet done (folds into a tool-geometry extension): the ENERGY (toughness) limb of `fracture_onset` (a
brittle edge shattering under a high-energy blow its stress margin would survive) rides inert (delivered
fracture energy zero) because it needs the stroke energy and the tool's own crack cross-section, which the
`WieldedTool` does not yet carry; the bend/buckle/shear failures (a slender haft overloaded, an edge pried
sideways) likewise need the tool's length and section modulus. These land when the tool carries its body
geometry (a deepening of C, alongside H).

### F. Capability-kernel expansion (root R1). STATUS: OPENED (SHEAR kernel landed) (commit pending).
The `CapabilityKernel` enum is no longer closed at Pierce: `CapabilityKernel::Shear` (the first non-piercing
action) is landed in the compose crate, registered as `ID_SHEAR` in the dev seed, so an `AffordanceDef` can
now require a shear law. The kernel reads `mech.contact_area`, `mat.shear_strength`, and `mat.yield_strength`:
an edge drives the reference force over its contact area as a shear stress ([`laws::shear`]), self-limited at
the part's own shear strength (von Mises from yield where the axis is silent), and if that effective shear
clears the reserved reference resistance it severs, graded above the threshold. Reserved-with-basis:
`reference_shear_resistance` (`CapabilityRefs`). Its CONSUMER: the CUT affordance now gates on `ID_SHEAR`
rather than the earlier PIERCE (normal-penetration) proxy (`homeostasis::dev_cutter`), since a cut is a
shear-parting action. Compose falsifier
`a_keen_strong_edge_reads_a_shear_capability_a_blunt_or_weak_or_ductileless_one_does_not`.
STILL PLANNED: the remaining kernels (impact/percussion, lever, crush, bend, friction) land as their actions
(Section G) read them, one per consumer, the "capability lands when read" discipline. The kernel SET stays
fixed Rust; which affordances a world declares is data.
DONE (R-CUT-SHEAR): the cut ENACT sever gate now reads the SHEAR contest. `cut_underfoot` drives a shear
stress over the edge's contact area ([`laws::shear`]), self-limited at the tool's own `mat.shear_strength`
(von Mises from yield where the axis is silent), and severs each constituent whose own `mat.shear_strength`
that deliverable shear beats, retiring the normal-stress `fracture_strength` proxy. The cut/wear/breakage
enact floors gained `mat.shear_strength`; the cut falsifier isolates the change with a `fibre` weak in
fracture but tough in shear that the shear cut leaves. Byte-neutral, sim 893. So the cut is now a shear
process end to end: it AFFORDS via the SHEAR kernel (F) and ENACTS via the shear contest.

### G. New tool actions derived from F + C. STATUS: SEVER done (R-CUT-SHEAR); the rest gated, see the gate-map below.
- SEVER/divide: DONE (R-CUT-SHEAR, the shear cut, afford + enact).
- CRUSH (compressive failure): DONE. `CapabilityKernel::Crush` (`ID_CRUSH`) + `crush_underfoot` gating per
  constituent on `mat.compressive_strength`, the compressive sibling of the shear cut, afforded via
  `dev_crusher` (`CRUSH` affordance id 11), wired into the dispatch with breakage-before-wear. Reserved:
  `reference_compressive_resistance`. Falsifier
  `a_crush_fails_compression_where_a_cut_parts_shear_the_same_tool_diverging_by_the_targets_axis`: the SAME
  tool crushes a compression-weak constituent and cuts a shear-weak one, diverging by the target's own axes.
  So the non-piercing action space now holds two kernels (shear + crush) and two enacts, byte-neutral.
- STRIKE/percussion: the ENACT and the MASS PAYOFF are DONE. `strike_underfoot` swings the wielded tool and
  its kinetic energy (`laws::kinetic_energy`, `1/2 m v^2` over the tool's own mass = density x volume)
  fractures the matter underfoot whose Griffith energy the blow exceeds (`fracture_onset`'s energy limb, the
  delivered energy scaled from the law's kJ to the J the toughness is on). Falsifier
  `a_heavy_struck_tool_shatters_rock_a_light_one_of_the_same_shape_cannot_the_mass_payoff`: a HEAVY tool
  shatters a rock a LIGHT one of the identical shape cannot, the mass payoff. Reserved: `StrikeParams`
  (swing velocity, energy ceiling). REMAINING for STRIKE: the AFFORDANCE (an Impact capability kernel reading
  the tool MASS via GATE 2, so a being can DECIDE to strike) and the dispatch arm; the enact is directly
  proven, the affordance is the world-wiring follow-on.
- LEVER/pry, ABRADE, BORE, and the depth-sizing of the cut (R-CUT-DEPTH): each needs a substrate expansion,
  listed in the gate-map below. Marked, not forced.

## SUBSTRATE-EXPANSION GATE-MAP (owner: these are where the physics substrate needs to grow to unblock the rest)
The absolute rule holds: the substrate is the only authored place. Each item below is a place the tool
work HITS the edge of the current substrate and cannot derive further without the owner adding a physics
axis, a law, or a piece of DERIVED tool/matter geometry. None is authored-behaviour; each is a substrate
grow. Ordered by how much it unblocks.

- GATE 1, TOOL BODY GEOMETRY (root R2): PARTLY DONE. `WieldedTool` now carries a characteristic LENGTH (a
  reserved `CraftParams::tool_length`, set at craft), from which its body CROSS-SECTION derives as
  `volume / length` (the prism relation, `WieldedTool::cross_section`, no cube root and no shape catalog).
  Its first CONSUMER is landed: `break_check` now also BUCKLES a tool whose slender body cannot bear its own
  axial working load (`laws::euler_buckle` over the elastic modulus, the derived cross-section's area moment
  `A^2/12`, and the length; the pinned-pinned end factor is the Euler reference, a per-tool end condition a
  reserved refinement). Falsifier
  `a_slender_tool_buckles_under_its_working_load_where_a_stout_one_of_the_same_stock_bears_it`: two tools of
  the SAME stock and material differing ONLY in length, the slender one buckles where the stout one bears the
  load, the geometry tradeoff. STILL to read the geometry: E's toughness (energy) limb (a DYNAMIC strike's
  crack cross-section, needs GATE 2's mass + swing), a bend failure (a sideways pry load), and LEVER/pry
  (`laws::lever` over the length as the effort arm, plus a reserved load arm). The length also feeds STRIKE's
  energy criterion. So the geometry is now CARRIED and one failure mode reads it; the rest land as their
  actions do.
- GATE 2, TOOL MASS IN THE CAPABILITY CLOSURE. The affordance capability closure exposes only
  `mech.contact_area` to a kernel; to afford a mass-driven action (STRIKE percussion, `kinetic_energy` over
  the tool mass = density x volume, both known) the closure must also expose the tool's volume/mass. A
  wiring grow, not a new axis. With it plus a reserved swing velocity, STRIKE is derivable (the blow's
  delivered energy fractures the target via `fracture_onset`'s energy limb, crack area the struck face).
- GATE 3, MATTER FORM/STATE. A cell's `SubstanceMix` is BULK composition with no geometry or state, so a
  TRANSFORMATIVE action (grind to powder, shape, cook) cannot represent its OUTCOME without authoring a
  transmutation (the very thing Section A purged), and a percussive/abrasive action cannot derive the
  target's struck effective-mass, crack area, or removal amount. The grow: a matter form/state
  representation (a particle-size or worked-state axis on the cell's matter), or a derived convention for a
  struck/removed geometry from a cell's composition. This gates: STRIKE's target-mass, ABRADE's removal
  amount, R-CUT-DEPTH's depth-sized cut amount (which otherwise stays carry-bounded; `mat.specific_cut_energy`
  and `cut_penetrate` already exist, only the stroke length and the amount->depth wiring are missing), and
  any cook/grind/shape action.
- GATE 4, HERTZIAN CONTACT for BORE. Drilling/boring wants a hertzian contact stress; confirm whether a
  hertzian law exists in the floor (R-PHYS-MECH lists one) and wire it, plus a bore-hole geometry (gated on
  GATE 3's matter geometry).
- GATE 5, TOOL OBJECT IDENTITY + THERMAL WIRING (Section I). A tool as a `StableId` object that accrues
  state across hands, plus wiring the EXISTING thermal laws (`conduction`, `combustion`, `phase_change`,
  `thermal_stress`) to the tool: friction heats, fire consumes a wooden tool, a fluid corrodes a metal one,
  and a QUENCH raises `mat.indentation_hardness` (feeding back into the tool's own strength). The thermal
  laws exist; the grow is the object-identity rider and the couplings.
- GATE 6, COMPOSITE TOOL STRUCTURE (Section H). `WieldedTool` becomes a multi-material jointed structure (a
  hard head bound to a tough shaft), the bind a physical joint (`shear`/`friction`), craft consuming a
  `SubstanceMix` or another tool as stock. The deepest grow, the composition stage of the made-world arc.

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
reworked cut, craft, edge_area_at, and mass. Confirmed fixed: the cut_yields map is gone (the
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
