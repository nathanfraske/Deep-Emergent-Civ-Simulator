# Moon arc scope (pipeline slices 6-7): moons as a three-branch dispatch, tidal-survival filtered

Status: scoped, fetch-cleared (Canup-Ward, the Domingos Hill fractions, and the tidal-recession form are in `PIPELINE_FETCHES.md`), dependency-gated on the assembly work (see couplings). Written-arc-first because the build couples to the delicate north-star assembly, which is gated awake. This is the derive-first moon substrate for the star-planet generator: a planet's satellites emerge from its formation, never an authored moon count.

## The design (from the consolidated pipeline, Stage 3)

Moons are a THREE-BRANCH DISPATCH, each branch a different formation channel keyed on per-world data, with a TIDAL-SURVIVAL post-condition on ALL branches. No branch authors a moon; each derives whether and what forms.

### Branch A: circumplanetary-disk (CPD) co-accretion, the regular satellites
A gas or ice giant (the #73 race) hosts a circumplanetary disk as it accretes, and regular satellites co-accrete from it. The satellite-system mass scales with the planet: `M_sat / M_planet ~ 1e-4` (Canup & Ward 2006, an accretion-plus-loss equilibrium, the fetch). The CPD COMPOSITION derives from the engine's OWN condensation run in the circumplanetary environment (the built condensation substrate applied to the CPD's temperature and pressure profile, so the Galilean ice-versus-rock gradient emerges, never an authored roster). Convicting bodies: the Galilean and Saturnian regular satellites. This branch DEPENDS on #73 giants being in the assembly (slice 6), so a planet is known to be a giant with a CPD.

### Branch B: giant-impact moons (the Earth-Moon channel)
A late giant impact in the assembly's merge history can eject a debris disk that re-accretes into a moon (the canonical Earth-Moon origin). This branch reads the ASSEMBLY'S MERGE EVENTS (each merge is a giant impact with a mass ratio and a velocity): a merge above an impact-energy and mass-ratio threshold spawns a debris disk whose mass and angular momentum set the moon. The moon composition is the impactor-plus-target silicate (volatile-depleted, the derived signature of the impact origin). Convicting body: Earth's Moon (and Pluto-Charon as the high-mass-ratio corner). This branch DEPENDS on the assembly EMITTING its merge history (currently the assembly returns only the final planets, the same gap the "watch it build" mode needs closed).

### Branch C: capture
A body from the small-body reservoir (the #74 arc) passing within the planet's Hill sphere can be captured under the right energy conditions (gas drag in the early disk, a three-body exchange, or a collision). Convicting bodies: Triton (a captured Kuiper body, retrograde), Phobos and Deimos (captured or impact-origin, the open case). This branch reads the #74 reservoir plus an encounter model, and is the most model-heavy; it may ship last.

## The tidal-survival filter (all branches, fetched)

Every candidate moon passes a survival post-condition before it enters the catalog, from the fetched tidal machinery:
- ORBITAL STABILITY: the moon must sit inside the derived stable fraction of the planet's Hill radius, `~0.49 R_Hill` prograde, `~0.93 R_Hill` retrograde (Domingos, Winter & Yokoyama 2006, the fetch), and outside the Roche limit (the derived tidal-disruption radius from the densities). A candidate outside the stable band is not retained.
- TIDAL EVOLUTION: the recession or decay `da/dt = 3 (k2/Q) (m/M) (R/a)^5 n a` (the fetch, with the ~3.82 cm/yr lunar recession as the anchor) evolves the orbit over the system age; a moon that recedes past the stability bound or decays inside the Roche limit within the age does not survive. `k2` (the Love number) and `Q` (the tidal quality factor) are reserved-with-basis per-world (the planet's rigidity and dissipation), banded.

## The value line

Derived or read: the CPD mass ratio (Canup-Ward, cited), the CPD composition (the condensation substrate), the giant-impact debris mass (the assembly's merge energy and angular momentum), the Hill radius and Roche limit (from masses and densities), the tidal recession form (cited). Reserved-with-basis (surfaced, not invented): the giant-impact moon-forming threshold (the impact-energy and mass-ratio boundary, banded), and `k2/Q` (the planet's tidal Love number and dissipation factor, per-world). No authored moon count; the count and the satellites emerge.

## Couplings and the build order

Slice 6, #73 GIANTS INTO THE ASSEMBLY (delicate, the north-star core, gated awake): the giant-formation verdict (`giants.rs`, built) is wired into the assembly so a final system carries giants, which UNLOCKS branch A (CPD moons). This modifies `planetary_assembly.rs`, so it is gated with the owner awake, not built blind.

Slice 7, THE MOON IMPACT BRANCH (branch B) with the survival filter: needs the assembly to EMIT its merge history (a shared prerequisite with the "watch it build" construction-montage mode, task #80). Once the assembly exposes its merges, branch B reads them, applies the debris-disk and tidal-survival machinery, and produces the giant-impact moons.

Branch C (capture) and the full CPD condensation are the trailing refinements. The Hill radius, the Roche limit, and the tidal form are buildable now as a standalone `moons` module (a dormant substrate) ahead of the assembly wiring, so the survival filter and the orbital-stability bounds can land and be tested against the Earth-Moon and Galilean anchors before the branch dispatch is wired.

## The standalone first slice (buildable now, dormant)

A `moons` module exposing the derive-first primitives the branches share: `hill_radius`, `roche_limit(rho_moon, rho_planet, R_planet)`, `stable_semimajor_axis_band(R_hill, prograde|retrograde)` (the Domingos fractions), and `tidal_recession_rate(k2, Q, m, M, R, a, n)` (the fetched form), each cited and tested against its anchor (the Earth-Moon Hill/Roche/recession, the Galilean stability). This is byte-neutral (dormant, no run-path consumer) and unblocks the survival filter without touching the assembly, so it is the clean first commit when the moon arc starts.
