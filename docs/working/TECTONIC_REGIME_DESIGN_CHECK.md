# Tectonic-regime classification: blind-panel design-check

Agent B, the gate-offered design lane while the interior column-wiring waits on A's `GeodynamicColumn`
contract (#160). Doc-only: a blind framing panel on the closed-enum risk in the proposed tectonic-regime
kernel, with the panel's verdict verified against source and the corrected framing surfaced for the gate
to rule before any build.

## The seam

`GEODYNAMICS_ARC_PROPOSAL.md` (line 32) proposes, among the geodynamics floor growth, "a Rayleigh regime
kernel deciding mobile-lid versus stagnant-lid versus no-tectonics," and line 50 states the intent that
"the tectonic REGIME itself EMERGES from the Rayleigh number rather than being flagged." The selection is
meant to emerge, but the SET of outcomes, {mobile-lid, stagnant-lid, no-tectonics}, is a fixed three-way
taxonomy, and the downstream surface mechanisms (crustal fragmentation, relief, resurfacing) would read
the selected regime to produce their outcomes. A fixed category set consumed as a dispatch key is the
emergence-over-templates risk, so the construction went to a blind panel before any code.

## The panel

The section-10 blind framing panel: six panelists across three agent types and three models
(general-purpose/opus, general-purpose/sonnet, Plan/opus, Plan/sonnet, claude/fable, claude/sonnet), each
isolated, each handed the identical sealed packet (the guiding principles, the neutral mechanism facts
traced to source, and the raw de-narrivatized statement to attack), with no author or owner conclusion and
no hint of the suspected flaw. The statement was phrased as a claim to attack: a world's tectonic behaviour
is assigned to exactly one of three named regimes by comparing a convective-vigor ratio against critical
thresholds, and its surface behaviour follows from the assigned regime.

## The verdict: unanimous, two convergent findings

All six panelists returned a flaw verdict (five "significant-flaw-fixable," one "reframe-needed"), converging
on two findings.

**Finding 1, the core seam (P8 and the template case).** The closed three-member regime set, consumed by
downstream surface mechanisms as a dispatch key, is a high-level categorical fact read to produce a
behaviour: the literal form of the template case (as reading genetic relatedness authors Hamilton's rule).
Plate fragmentation, relief, and resurfacing would branch on the regime NAME rather than on the continuous
physics that produced it, so the label is a coded shortcut sitting where world content should emerge. It
fails the alien test: {mobile-lid, stagnant-lid, no-tectonics} is Solar-System planetology vocabulary with
no slot, without a code change, for a heat-pipe regime (Io), an episodic or sluggish lid, ice-shell diapirism
and chaos terrain (Europa), or a mana-modulated crust. A closed enum where the world's own physics should
place it continuously.

**Finding 2, a load-bearing physics catch (the value-authoring line).** The marginal-stability onset value
(the Rayleigh critical number) is a derivable law constant and marks whether the interior convects at all.
But the mobile-lid versus stagnant-lid boundary is NOT a threshold on the Rayleigh ratio: it is a distinct
physical competition between the convective driving stress and the lithosphere's yield strength. Authoring
that boundary as "a critical value of the ratio" fabricates a number for a transition the ratio alone does
not govern, a value outside the physics floor.

## Source verification (Prime Directive 1)

Finding 2 is the decisive technical claim, so it was verified against source rather than trusted.

The design's own line 50 carries exactly the imprecision the panel names: it attributes a stagnant lid to
"low internal heat or high creep viscosity dropping the ratio sub-critical, so the lithosphere never
fragments." That conflates a stagnant lid with the absence of convection. A stagnant-lid world (Mars, Venus)
runs a SUPER-critical Rayleigh number: its interior does convect, under a lithosphere that does not mobilize
into plates. Sub-critical Rayleigh is the separate no-convection limit (the static-lid, no-tectonics case).
The mobile-versus-stagnant distinction is set by whether the convective stress reaches the lithosphere's
yield strength, which is the established result in the mantle-convection literature (the yield-stress
criterion for lid mobilization), not a function of the Rayleigh number alone. So the design's stated
"regime emerges from the Rayleigh number" is physically imprecise, and the panel's correction is sound.

The fix reads a quantity the floor already supplies: `mat.yield_strength` is a floored axis
(`crates/physics/data/mechanical_floor.toml:215`, "the stress at the onset of permanent plastic
deformation"), and the arc already reads it as the quake threshold (proposal lines 32 and 46). So keying lid
mobilization on convective stress against `mat.yield_strength` is a derivation from existing floor data, not
a new authored constant.

## The corrected framing (verified, for the gate to rule)

The interior model does not classify a world into a named regime and no downstream mechanism reads a regime
label. Instead:

- The floor keeps the Rayleigh convective-vigor ratio (buoyancy against viscous and diffusive resistance)
  computed from per-world axes, and the marginal-stability onset value as the one derivable law constant.
  The ratio above onset means the interior convects; below onset is the no-convection static-lid limit.
- A second continuous quantity, physically distinct, derives from the same floor: the convective driving
  stress the interior exerts on the base of the lithosphere, compared against the lithosphere's own yield
  strength (`mat.yield_strength`, already floored). This margin governs whether and where the lid mobilizes.
- The downstream surface mechanisms read these continuous quantities directly and each computes its own
  local outcome: the outer shell fragments locally where convective stress exceeds lithospheric yield
  strength (so fragmentation is partial, patchy, or episodic, never a global on-or-off switch); relief and
  resurfacing scale continuously with the convective vigor and the stress margin; heat flux and
  boundary-layer thickness feed the same way.
- A regime NAME (mobile-lid, stagnant-lid, heat-pipe, episodic, an ice-diapiric regime, and any a world's
  physics implies) is a post-hoc observer-side descriptive readout of where a world sits in the continuous
  space, computed on demand for the glyph view, the event log, or debugging, from an OPEN data-registered
  taxonomy a world's data can extend. It is never read back into a causal mechanism (P10) and never written
  to canonical state.

The regime label becomes a description of an emergent outcome rather than the mechanism that produces it,
the template-case resolution in its exact form.

## Reserved values, surfaced not fabricated

- The marginal-stability onset value (the Rayleigh critical number): a derivable law constant from linear
  stability analysis (the classical free-free and rigid boundary values), basis the boundary conditions the
  world's rheology sets. A floor law constant, sibling to the existing derived constants.
- The convective driving stress on the lithosphere base: a floor law to add, derived from the convective
  vigor, the viscosity, and the layer geometry, basis the boundary-layer stress balance. No world value.
- The lid-mobilization criterion is the ratio of convective stress to `mat.yield_strength`, a physical
  criterion (mobilization where the ratio exceeds one), so no authored regime-boundary number enters.

## Honest limits

The continuous stress-against-strength criterion is a first-order model of lid mobilization. The real
transition also depends on the temperature-dependence of viscosity and can be hysteretic or episodic. The
value of keying on continuous fields is that those refinements are further derivations on the same
substrate rather than new named regimes, but a full episodic-overturn model is a later arc, not this design.
This is a design-check verdict; the build is the gate's to authorize, and the affected passages in
`GEODYNAMICS_ARC_PROPOSAL.md` (lines 32 and 50) are reworded to the corrected framing only once the gate
rules.
