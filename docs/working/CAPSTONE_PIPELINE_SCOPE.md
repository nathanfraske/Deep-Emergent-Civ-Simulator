# The star-planet capstone: the full generative-and-visible pipeline (design-first scope)

This is the design-first scope of the capstone the owner named: from a minimal authored input set, GENERATE a world you can SEE, its geology and tiles derived from the materials substrate, its atmosphere emergent, and it renders. Surfaced for the gate's derive-first scope pass before a line of code. The scope is grounded against the actual codebase (two full-pipeline grounding passes), so the BUILT / PARTIAL / NEW map is real, not hypothetical. Its top-line validation is the Hadean-Earth acceptance gate (section 2): the pipeline is trusted only when the Sun plus Earth's orbit DERIVE a Hadean Earth within grade.

## 1. The authored input set (owner-corrected, the whole of it)

Everything the world is derives from four authored things and nothing else:

- the fundamental constants (already the floor: `G`, `k_B`, `h`, `c`, the periodic table);
- the STAR: its mass and its composition;
- each PLANET: its ORBIT (orbital distance / state vector), and nothing else.

The planet's mass, bulk composition, and radius are OUTPUTS, not inputs: they derive from what accretes at the orbit. So an author writes a star and some orbits, and the engine derives the worlds. This makes gravity fully derived (section 8).

## 2. The acceptance gate: the Sun plus Earth's orbit derive a Hadean Earth (the pipeline's Mirror)

This is the top-line validation, the materials-ringer discipline lifted to planetary scale. Feed the pipeline the SUN (its mass and composition) and EARTH'S ORBIT (1 AU), author nothing else, and the DERIVED planet must land a Hadean Earth within grade. The comparison is the HADEAN specifically (~4.5 to 4.0 Gya), because the pipeline generates a freshly-accreted world: it produces the newborn planet, not modern Earth. Modern Earth is 4.5 Gyr of geodynamic and biological evolution ON TOP of that Hadean start, and that evolution is the deep-time co-evolution arc FROM this initial condition (R-COEVOLVE, Mirror-as-initial-conditions), never the capstone's output. The capstone's job is to derive the Hadean IC; the deep-time arc evolves it forward.

The pre-registered Hadean battery, derived and compared to the real Hadean, never fit (pre-registered here the way the band-gap and Stoner trends were, before a line of code):

- **Bulk.** Mass ~1 Earth mass; an iron core plus a silicate mantle from the DRY inner-disk condensation at 1 AU (water arrives late as a veneer, the sharp prediction that looks like a miss and is success); radius ~6371 km.
- **Gravity.** `g ~ 9.8 m/s^2`, fully DERIVED from the accretion-derived `M` and `R`, the hardcoded `9.80665` gone (section 8).
- **Interior.** Differentiated (core and mantle), a magma-ocean-to-first-crust surface, active early geodynamics.
- **Atmosphere.** A secondary-outgassed Hadean atmosphere (`CO2` / `N2` / `H2O`-dominated, reducing to neutral, NO free `O2`), emergent from the composition and the outgassing, never an authored gas mix.
- **Tiles.** A materials-substrate-derived early basaltic surface.

The discipline is absolute: the pipeline must DERIVE the Hadean from the Sun plus the orbit plus the physics floor, never be FIT to Earth. Fitting is authoring; Earth is the calibration check, not the target the knobs chase (the value-authoring line and the Mirror-by-calibration rule). A pass is right-within-grade OR a graceful, honest failure that flags where the physics is incomplete, NEVER a confident wrong number. If the pipeline does not land a Hadean Earth, that is a real finding to diagnose, exactly as the Mirror founder-extinction diagnosis was, not a knob to turn. This gate is built in from the start, so every slice is measured against it.

## 3. The pipeline and its BUILT / PARTIAL / NEW map

The corrected chain, each stage mapped against the real tree:

| # | Stage | Input -> output | Status | Grounding |
|---|-------|-----------------|--------|-----------|
| 1 | Stellar structure | star mass + composition -> `L`, `T_eff`, flux | **PARTIAL** | `astro::stellar_flux` (mass-luminosity + inverse-square flux) is built; the `T_eff` / mass-radius Stefan-Boltzmann solve is NOT (`sim/src/astro.rs:79`, `PLANETARY_STELLAR_PHYSICS_DERIVE_MAP.md`). |
| 2 | Disk structure | `L` -> disk thermal + surface-density profile | **NEW** | not in the tree. |
| 3 | Disk condensation | disk `T(r)` + elemental abundance -> solid composition at each orbit | **NEW (substrate exists)** | spec'd (`MATERIALS_ORACLE_SPEC.md:12`, "condensation disposer across the disk"); the disposer is the substrate but is condensed-ionic and does NOT read `T`/`P`/mu or gas phases. NOT a resolved named research item (section 7). |
| 4 | Accretion | disk surface density + local composition + orbit -> planet mass, bulk composition, radius | **NEW** | not in the tree; the arc the owner made explicit. Where the planet's bulk emerges from nothing but its orbit. |
| 5 | Materials substrate | composition -> materials + properties | **BUILT** | `Composition{amounts: BTreeMap<String,Fixed>}` -> `propose_candidates` -> disposer; the property oracle over the assemblage (`thermochemical/proposer.rs:55`, `physics/src/materials_oracle.rs`, `petrology::stable_assemblage`). |
| 6 | Geology | composition + physics floor -> interior structure + surface elevation | **BUILT but DORMANT** | full interior convection, Airy isostasy, and crust/mantle density that ALREADY derives from composition via the petrology kernel; byte-neutral, no run-path consumer, not wired to tiles (`physics/src/geodynamics.rs`, `sim/src/geodynamics.rs`, `sim/src/geodynamics_surface.rs`). |
| 7 | Tiles | geology + materials -> a grid whose terrain and material derive | **BUILT grid, terrain to be re-derived** | `Tile{axes, biome}` exists, but the axes are FRACTAL NOISE and the biome is an authored band table. The old authored-terrain ruling is overridden (section 4). |
| 8 | Atmosphere | composition + stellar flux -> the gas mix and the climate | **energy-balance PARTIAL, COMPOSITION ABSENT** | surface energy balance, hydrology/vapour, saturation curves are built and the flux derives from star mass + orbit; albedo and greenhouse are authored scalars; the gas-mix composition from outgassing is design-spec only (`physics/src/laws.rs:2557/2606`, `sim/src/environ.rs`, `EMERGENT_ATMOSPHERE_PIPELINE_DERIVE_MAP.md`). |
| 9 | Render | canon world -> a visible tile view | **BUILT** | a runnable windowed viewer plus headless PPM/JSON/glyph, a one-way read of canon, with a physics-derived terrain colour path already present (`viewer/src/main.rs`, `render.rs`). |

The shape of the work: the capstone is largely an INTEGRATION arc (stages 5, 6, 7, 9 exist; 1 is half-built) that wires the built-but-isolated pieces into one deterministic chain and re-derives the tile terrain, PLUS four genuinely new front-end arcs (stellar `T_eff`, disk structure, disk condensation, accretion) and one composition arc (atmosphere gas mix). None of the stages currently feed the next; the authored star -> planet -> materials -> geology -> tiles chain is assembled nowhere today.

## 4. Terrain now DERIVES: the old authored-terrain ruling is overridden

The codebase carried a banked owner ruling (OWNER_DECISIONS_LOG R1, enforced in `world/src/structure.rs`) that a scenario's terrain is a SELECTED input on the Principle-9 authored side, and the current tiles honour it: their elevation is fractal noise and their biome is an authored band table. The owner has now OVERRIDDEN that ruling: in the capstone, and going forward, a tile's terrain and material DERIVE from the materials substrate and the derived geology (Principle 8), never an authored terrain table. A tile is what the substrate says is there at that place.

So the tile axes stop being fractal noise: elevation comes from the geology's Airy isostatic surface (section 6, already built and composition-derived), and the surface material comes from the substrate's stable assemblage at that place's composition and conditions. The classification into a terrain kind emerges from those derived quantities rather than from a hardcoded band table. The old fractal-noise-plus-authored-biome path retires to a clearly-labelled test-only fixture (a scaffold for viewer development), not the canonical world path. This override is folded into the scope; `structure.rs` and OWNER_DECISIONS_LOG R1 are updated to record it when the tile stage is built.

## 5. The build order aimed at SEEING it: a minimal end-to-end visible slice first

The owner wants the world to show up, then to deepen. Because geology (dormant), the tile grid, and the viewer already exist, a visible end-to-end slice is reachable WITHOUT the full front end, by wiring the pieces that exist and standing in shallow inputs for the pieces that do not:

- **Slice 0 (the visible spine).** Author an orbit and a stand-in bulk composition (a shallow placeholder for the unbuilt accretion output, clearly labelled a fixture). Run the DORMANT geology on it: composition -> crust/mantle density (petrology, built) -> Airy isostatic elevation (built). WIRE that derived elevation into the tile axes (replacing the fractal noise), classify each tile by its derived elevation and material, and render it through the existing viewer. This proves the spine: an authored orbit yields a world whose terrain is what the materials substrate says is there, and it shows up. Every layer is shallow, but the pipeline is visible and deterministic end to end, and Slice 0 is already measurable against the Hadean gate's tile and elevation targets.
- **Then deepen each layer behind the spine, in dependency order:** (1) the stellar `T_eff` solve so the star's output fully derives from its mass; (2) the disk thermal + surface-density structure; (3) the disk-condensation disposer (the gas-phase extension, section 7) so the composition-by-orbit derives; (4) the accretion arc so the planet's mass, composition, and radius derive and the stand-in fixture retires (this closes the Hadean bulk and gravity targets); (5) the full geodynamics field (interior convection and differentiation, not only surface isostasy, closing the Hadean interior target); (6) the atmosphere composition arc so the gas mix, albedo, and greenhouse emerge rather than being authored scalars (closing the Hadean atmosphere target).

Prove the pipeline visibly, then add depth, rather than perfecting one layer before anything is seeable, with each deepening step retiring a fixture and closing one pre-registered Hadean target.

## 6. Derive-first analysis per new piece (authored inputs counted against the Buckingham groups)

Each new piece must read only the floor and the situation it is handed, reserving only what its dimensional analysis truly demands.

- **Stellar structure (T_eff solve).** Reads the star mass, composition, and the floor constants (`sigma` itself derives from `k_B`, `h`, `c`). `L ~ M^alpha` homology and `T_eff = (L / (4 pi R_star^2 sigma))^(1/4)`. The one honest residue is the opacity-regime exponent `alpha` (~3 to ~5), a physics-floor law constant with a stated basis (the opacity law of the regime), surfaced not fabricated. Authored inputs: the constants and the star vector, nothing per-world.
- **Disk structure.** Reads `L`, the star mass, and the orbit. The mid-plane temperature `T(r)` from irradiation balance and the surface density `Sigma(r)` from a self-similar profile. The profile's normalization and slope are the reserved residues, each with its basis (the disk-mass fraction and the viscous-spreading exponent), counted against the Buckingham groups of the accretion-disk similarity solution.
- **Disk condensation.** Reads `T(r)` and the elemental abundance vector at fixed total abundance; runs the disposer (section 7). Reserves nothing new beyond the disposer's own thermochemical floor; the gas-phase chemical potentials are floor data (the fugacity references), not per-world tunables. The dry-inner-disk condensation at 1 AU (no water condensing) is the emergent prediction the Hadean bulk target checks.
- **Accretion.** Reads the disk surface density and local condensed composition at the orbit and the orbit itself; yields the planet mass (a feeding-zone integral of `Sigma`), bulk composition (the condensed species in the zone), and radius (mass over the derived bulk density). The reserved residue is the feeding-zone width in mutual Hill radii, a dimensionless number with its basis (the isolation-mass criterion), counted against the accretion similarity groups. This is where a ~1 Earth mass, ~6371 km iron-core-plus-silicate-mantle Hadean planet must emerge from nothing but the Sun and 1 AU.

## 7. The disk-condensation proof-obligation, sharpened by the grounding

The owner flagged one proof-obligation: does the disposer's cancellation theorem survive the switch to fixed-elemental-abundance condensation. The grounding sharpens it into a clear split, and the news is mostly good.

The disposer's cancellation is the apparent-Gibbs ELEMENT-REFERENCE cancellation (Benson-Helgeson, `petrology.rs:53`): the element reference drops out of the comparison BECAUSE every competing assemblage forms from the same element budget. That precondition IS fixed elemental abundance. So at fixed abundance the cancellation does not merely survive, it is in exactly its home case; the theorem holds. That half of the obligation discharges cleanly, and the grounding shows it.

The real remaining risk is the GAS PHASE, which the owner folded into the same sentence. Today the disposer scores condensed ionic lattice energy only and does not read the environment mu-vector (the gas fugacities `fO2`, `fS2`, `fH2`, `aH2O`, `aCO2`); a covalent diatomic falls through as `None`. Condensation down a cooling path REQUIRES those gas terms in the objective (the full `G = E_lattice + P dV - T S_config - T S_vib` with the gas chemical potentials `mu_i = mu_i^0 + RT ln f_i`). The sharp obligation is therefore: does the order-invariant, resolution-banded comparison stay valid once the objective carries the `RT ln f` gas terms, given that those terms shift with the cooling path rather than cancelling. That is the piece to prove before the condensation disposer is trusted, and its self-check is already named in the spec: the disposer, run down the cooling path, must reproduce the CAI-first condensation sequence and yield the Bowen reaction series as an EMERGENT execution order, not an authored one.

One honesty note, since I audit the input and not only the output: the owner referred to this as "research-resolved (R-DISK-CONDENSE)". The grounding finds no such resolved named research item in the repo; the concept is a design-spec chain in the oracle spec, and the gas-phase condensation disposer is unbuilt. So disk condensation is not a widen-the-candidate-set afternoon; it is a real disposer extension (read `T`/`P`/mu, add gas phases, discharge the gas-term obligation), and I am scoping it as such rather than as a resolved item.

## 8. Gravity, now fully derived, and the reconciliation the capstone must respect

With the planet's mass and radius derived by accretion, gravity is fully derived: `g = G M / R^2`, no authored mass left in it, and the hardcoded `standard_gravity()` = 9.80665 (`runner.rs:896`, called at 2377 and 6824) retires. Three grounding facts bind how:

- A derived surface gravity ALREADY EXISTS, `surface_gravity(radius_m, mean_density)` = `(4/3) pi G R rhobar`, but on branch `claude/genesis-arming-step` (commit `05a18e8`), NOT in the current tree. `G M / R^2` reconciles with it EXACTLY for a uniform sphere (`M = (4/3) pi R^3 rhobar`), so the capstone must reuse or re-land that function, never author a parallel second gravity. The capstone branch base must therefore include the genesis-arming work.
- The mean density it needs is the WHOLE-PLANET mean (~5514 for Earth), not the silicate composition mean (~3300, which gives a wrong `g ~ 5.9`). The accretion-plus-differentiation arc is exactly what produces that whole-planet mean density (bulk composition plus the differentiated core), so the capstone closes the loop the existing derived-g left open (its `rhobar` is tagged measured "pending its own derivation from bulk composition plus core"). This is why the Hadean gravity target (`g ~ 9.8`) is a real derived check, not an input.
- Migration is small: the weight law already takes `gravity` as a parameter (`laws::weight(mass, gravity, force_max)`, `CarriedLoad::weight(reg, gravity, force_max)`, and the world-side `momentum.rs`), so the two `runner.rs` call sites pass the derived `g` with no law change.

## 9. Couplings and the disciplines that bind the whole pipeline

The couplings to hold: the star's `L` sets the disk thermal structure (2 reads 1), which sets both the condensation sequence (3 reads 2) and, through the flux, the atmosphere climate (8 reads 1); the accretion output is the single composition that feeds materials, geology, and the atmosphere composition alike (4 feeds 5, 6, 8), so the same derived composition must reach all three consistently; the geology's surface elevation feeds the tiles (6 feeds 7) while the tiles feed the render (7 feeds 9).

The disciplines, each a gate on every slice: tiles and geology DERIVE from the materials substrate (Principle 8, no authored terrain, R1 overridden); the atmosphere composition EMERGES from the accreted composition and the stellar flux (no authored gas mix); the world stays observer-independent in the canon and renders per-observer in the non-canon view (the colour rule just banked: a wavelength-to-colour mapping that moved `state_hash` is a canon leak); every value the pipeline reserves is surfaced with basis, never fabricated; and the whole is measured against the Hadean gate (section 2), never fit to it. Determinism binds the orbit substrate specially: chaos must be SAMPLED, never integrated as a fixed-point path integral of a chaotic trajectory (a byte-neutrality landmine), so an N-body / orbit substrate (issue #44, spec) uses the closed-form Laplace-Lagrange secular spectrum, not a trajectory integration.

## 10. The prove-it catches this scope surfaced

Four, each grounded, so they steer the build rather than surprise it:

1. The derived gravity lives on another branch and reads mean density (not mass); the capstone must reuse it and its branch base must include it, never author a second gravity (section 8).
2. "R-DISK-CONDENSE" is not a resolved research item in the repo; it is a spec concept, and the gas-phase condensation disposer is unbuilt, so disk condensation is a real disposer extension, not a candidate-set widening (section 7).
3. The condensation proof-obligation splits: the element-reference cancellation is in its home case at fixed abundance and discharges; the live risk is the gas-phase `RT ln f` terms in the objective, whose self-check is the emergent CAI-first / Bowen-series order (section 7).
4. The old authored-terrain ruling (R1) is now overridden by the owner; the tile stage re-derives terrain from the substrate and records the override in `structure.rs` and OWNER_DECISIONS_LOG (section 4).

The recommended first move is Slice 0, the visible spine (section 5), so the pipeline is seen before it is deepened, measured against the Hadean acceptance gate (section 2) from the first slice.
