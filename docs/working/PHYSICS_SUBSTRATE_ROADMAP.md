# Physics substrate roadmap: the arc after the crust

This is a true-state map of the physics substrate and a prioritized plan for the arc that follows the crust and mountains work currently in flight (the mid-band anchoring slice into the flexure render wire, Seams A through D, branch `claude/topology-increment3` off `claude/seam4-deeptime`). It answers the owner's three questions in order: what exists but is not wired to the runner, what we need to build out, and a plan. It is written to surface gaps, not to flatter the build, so the honest limits carry the weight.

The map was placed against `docs/working/PHYSICS_FLOOR_REGISTRY.md` (the 124 floor axes, 76 declared laws, 118 direct kernels, 19 reference substances, 11 deriving substrates), reconciled against `docs/working/CONSENSUS_ROADMAP.md` (the living status board), and grounded in the real call graph by tracing `run_world` (the canonical runner, `crates/sim/examples/run_world.rs` into `crates/sim/src/runner.rs::step_inner`), the derived-planet viewer (`crates/viewer/src/main.rs` + `render.rs`), and per-module caller greps across `crates/`. Every wired-versus-dormant verdict below carries file:line evidence or the absence of a caller.

---

## 1. The central finding: three lanes that do not meet

The physics substrate is not one system. It is three lanes that share the floor (`crates/physics`) but barely share a call graph, and understanding the arc after the crust starts with seeing which lane it lives in.

**Lane 1, the canonical `run_world` simulation (where the pins live).** This is the living-being world: a founder population on a calibrated Mirror Earth. Its tick (`runner.rs::step_inner`, `runner.rs:4917`) steps the base heat field (`field.rs`), the environment (`environ.rs`: water cycle, productivity, diurnal heating), the material matter cycle and combustion (`material.rs`), decomposition (`decompose.rs`), the medium (`medium.rs`), then per-being metabolism, locomotion, and cognition. The geology under these beings is dev-fixture worldgen, the hydrology is a dimensionless moisture index, and the biosphere is generated once and then not stepped. The four canonical pins (default, full, discovery, viability) plus the living pin measure this lane.

**Lane 2, the derived-planet viewer (non-canon, display only).** This is the star-and-planet generative pipeline: from a star mass and an orbit, `build_derived_scene` (`main.rs:2694`) derives a disk composition, a condensation temperature, a crust and mantle assemblage, a planet radius and gravity, a young-thermal melt regime, a deep-time province field, and renders relief, crust colour, lava glow, impact craters, and a Rayleigh sky. It has a deep-time playback clock that evolves the interior and crust over billions of years. This is where the crust and mountains work lives, and it writes no canonical state (Principle 10): every function is tagged non-canon, and the two run pins are structurally untouched by any of it.

**Lane 3, fully dormant (built, tested, no caller anywhere but each other and tests).** A large fraction of the deepest physics sits here: the multi-body solar-system generator (embryo field, assembly, giants, small bodies, moons, secular resonances), the surface-process erosion and deposition drivers, the perpetual-cooling secular thermal history, the mid-band mountain physics stack now being wired into Lane 2, and a set of radiative and tidal kernels with zero consumers. Five of these modules self-declare their dormancy in their headers (`stellar_evolution.rs:49`, `moons.rs:15`, `smallbody.rs:59`, `giants.rs:60`, `planetary_assembly.rs:547`).

The consequence that shapes the whole plan: the crust and mountains work is Lane 2, so the arc after it is Lane 2, and it is non-canon by construction. Almost nothing in this roadmap moves the run_world pins. The two places a canonical re-pin could enter (arming the genesis surface substrate onto the run path, and the hydrology physicalization) are called out where they arise. The eventual convergence of Lane 2 into Lane 1 (the derived planet becoming the ground the beings live on) is the north star (R-COEVOLVE), noted at the end.

---

## 2. What is wired (so that "dormant" means something)

Lane 1's per-tick physics is real and stepped every tick. The environment substrate is installed unconditionally (`worldbuild.rs:476`) and stepped at `runner.rs:4417` (water, productivity, evaporation, weathering, diurnal baseline, producer extraction, regrowth). The material matter cycle and combustion run at `runner.rs:4506` and `runner.rs:4723`, decomposition inside the matter cycle at `runner.rs:4631`, the medium's respiration and oxidiser at `runner.rs:6792`, body-to-field thermal exchange at `runner.rs:5627`, and metabolism plus locomotion inside `step_embodiment` at `runner.rs:5844`. The clock derives the life cadence and the diurnal and orbital periods from the world's own orbit (`clock.rs`, `world.rs:967`). Materials properties are read on the run path for tools, contact, and wounds (`runner.rs` references `material::` at 23 sites). This is a working physics engine for a living world.

The whole of Lane 1's contact with the planetary and stellar substrate is a single function: the runner arms `environ::DiurnalSky::mirror` (`run_world.rs:2590`, living scenario only), which fills its solar constant from `astro::stellar_flux` (`environ.rs:2635`). Orbital periods come from `clock::orbital_from_manifest`, which reaches neither `astro` nor `orbit`. So the runner never touches the disk, the condensation pipeline, the assembly, the Kepler integrator, or the deep-time interior.

Lane 2's viewer derives and renders, per the completed viewer and astro traces, a straight-line pipeline routed through the integration spine `planet::derive_planet` (`main.rs:2783`): disk composition (`main.rs:2647`), the formation-era condensation temperature with its full pre-main-sequence luminosity and viscous-time clock (`main.rs:1697` into `1597`), surface composition and crust (`main.rs:2732`), mantle density (`main.rs:2760`), the isolation-mass feeding zone (`main.rs:2772`), the star's luminosity, radius, and effective temperature through `stellar::main_sequence_star` (`planet.rs:106`), the two-regime disk temperature (`planet.rs:115`), the young-thermal verdict (`main.rs:2889`), the deep-time province field (`main.rs:2965`), crust colour under the star (`main.rs:3029`), atmospheric gas speciation for the sky (`main.rs:3050`), and lighting attitude and orbital phase from `orbit.rs` (`main.rs:206`). Its deep-time playback steps `step_provinces` (`main.rs:2155`), which evolves interior convection, monotonic crust growth, and impact bombardment.

The distinction that matters for the plan is inside the disk pipeline itself: the viewer consumes the disk up to condensation (formation midplane, condensation temperature, pre-main-sequence luminosity, feeding zone), but the step past condensation, the disk's own Lynden-Bell-Pringle surface-density realization that feeds the embryo field and the assembly, is dormant. Everything in the deep-time, planetary-assembly, and celestial-mechanics stack beyond that condensation cut is dormant on both the run path and the viewer path.

---

## 3. The dormant-but-built inventory (Question 1)

The pattern the owner named (built the early evolution, missing the perpetual dynamics) is the dominant one, and it generalizes past the deep-time example into five more clusters. The table below is the reference index; the prose after it explains the shape and the shortest activation path for each cluster.

| Substrate (file) | Built (evidence) | Lane / caller | Missing wire (shortest activation) |
|---|---|---|---|
| Secular cooling `secular_step`, `SecularState`, `secular_history` (`geodynamics.rs:283`) | Decays the isotope reservoir, feeds falling heat into `convection_step`; the "spent-world relaxation the static-source step cannot express" | Fully dormant (no caller) | Thread a per-column reservoir into `DeepTimeState`, swap `convection_step` for `secular_step` in `step_deep_time` |
| Star brightening `stellar_luminosity_ratio`, `StarAgingParams` (`deeptime.rs:216, 810`) | Gough 1981 main-sequence climb; the aging clock already advances | Viewer reaches the module, but `step_deep_time` takes no aging param, so the header thread is unconnected | Give `step_deep_time` the aging param, read `stellar_luminosity_ratio(age)` each deep tick, recompute `t_eff` and insolation |
| Relief collapse `relax_to_support_bound` (`deeptime.rs:681`) | Mass-conserving lateral collapse to the derived support bound; retires the `1e8` yield literal | Dormant; viewer display uses a FLAG instead | Thread the crust's derived shear modulus into `SupportBoundParams`, call after `step_provinces`, retire the flag |
| Surface erosion `hillslope_diffuse`, `fluid_shear`, `thermal_chemical_alter`, `deposit` (`surface_drivers.rs:81, 291, 427, 580`) | Four-driver continuous mass budget, 30 tests, conserves per-driver | Fully dormant ("off the run path until a genesis pass arms a driver") | Arm onto the deeptime relief; the fluid-independent pair (hillslope, thermal-chemical) needs no hydrosphere, fluvial needs discharge |
| Surface mass budget `SurfaceMassBudget`, `DriverRegistry`, `reconcile_column`, `apportion` (`surface_transport.rs:72, 267, 362`) | Four-reservoir conservation ledger, snapshot-apply reconciliation | Fully dormant (only `nonlocal_coupling.rs`, itself uncalled) | The cross-writer whole-budget closure is a stated obligation, not built; then arm |
| Isostatic relaxation `relax_toward_isostasy` (`geodynamics_surface.rs:67`) | Single Jacobi pass to the Airy target | Fully dormant; outer loop deferred | Arm a genesis producer (bulk silicate Earth) to fill `isostatic_elevation` |
| Interior-field application `populate_interior_column`, `step_interior_field` (`geodynamics.rs:335, 372`) | Snapshot-apply on the `GeodynamicColumn` contract | Fully dormant ("no scenario calls this yet") | Arm a genesis interior scenario |
| Tectonic regime `mobilization_margin`, `RegimeDescriptorRegistry` (`tectonic_regime.rs:42, 137`) | Stagnant/mobile-lid classification from convective stress over lid yield | Dormant by design (observer-only, never causal) | Read into the viewer as a descriptive overlay only; never a mechanism |
| Multi-body embryo field `oligarchic_embryo_field`, `ice_line_au`, `SolidDisk` (`planetary_system.rs`) | Emergent embryo count, mass, spacing from the disk | Fully dormant (only assembly + giants) | Feed the LBP surface-density realization into it, then into assembly and the viewer |
| System assembly `assemble_system`, `assemble_system_with_giants`, `_with_history` (`planetary_assembly.rs`) | Chaos-Protocol seeded merge, mass and angular momentum conserved, 41-planet Mirror system | Fully dormant, self-declared (`:547`) | Wire into the viewer's system map, replacing the five independent `derive_planet` samples |
| Giants `giant_formation`, `DiskGasLedger` (`giants.rs`) | Ikoma critical mass, Kelvin-Helmholtz contraction, gas ledger | Fully dormant, self-declared (`:60`) | Enters with `assemble_system_with_giants` |
| Small bodies `sample_belt`, `derive_small_body` (`smallbody.rs`) | Snow-line split, Dohnanyi size, Rayleigh eccentricity, emergent belt | Fully dormant, self-declared (`:59`) | Enters with the assembly's swept zones |
| Moons `hill_radius`, `roche_limit`, `tidal_recession_rate` (`moons.rs`) | Tidal-survival primitives, cited Hill fractions | Fully dormant, self-declared (`:15`) | Enters with the moon-arc branch dispatch on the assembly's merges |
| Secular resonances `secular_spectrum` (`secular.rs:715`) | Laplace-Lagrange g and s eigenfrequencies (Milankovitch forcing), GR precession | Fully dormant (no caller) | Needs a fragmentation pass for nonzero mode amplitudes, then a climate-envelope consumer |
| Post-MS stellar track `stellar_evolution.rs` | Schonberg-Chandrasekhar trigger, Hayashi attractor, shell-burning luminosity | Fully dormant, self-declared (`:49`) | Enters with the system generator when a star ages past the main sequence |
| Mid-band mountain stack: `flexure`, `moment_equivalence`, `geotherm`, `yield_envelope`, `convective_viscosity`, `convection_scaling`, `creep_rows`, `mineral_moduli`, `hindcast_comparison`, `young_thermal` | The full T_e / D_eq / flexural filter chain, cited and tested | Being wired into Lane 2 NOW (the crust work in flight) | This is the arc in flight; Seams A through D land the render wire |
| Radiative kernels `wien_peak`, `interface_split` (`laws.rs`) | Wien peak wavelength, Fresnel interface split | Fully dormant (zero consumers) | Enter with the atmosphere radiative-balance closure (a build, not a wire) |
| Biosphere stepping `LivingWorld::step_once` (`genesis.rs`) | Generated once at `run_world.rs:2179`, stepped only in the generate-loop and viewer | Run-path dormant (generated, not stepped per tick) | Step the living world inside the runner tick (the biosphere run gap) |
| Conflict pressure `conflict_pressure` (`value.rs:525`) | Computed and tested | Run-path dormant (zero run-loop consumers) | A raiding/war mechanism that reads it (not a physics-floor item) |
| Nernst redox uptake-flux (`environ.rs`, `laws.rs`) | Yield and draw from the couple's galvanic EMF, catalyst-tissue kinetics | Opt-in, Earth binds no redox source | An alien world arms a redox source (Venus/Europa) |
| Materials Stage-4 disposer (composition-to-properties oracle) | Ionic, correlation, metallic, localized routes, all with real consumers | Viewer-wired via `surface_composition` | Already reaching the viewer crust; deeper properties await their consumers |

**Cluster one, the perpetual-dynamics terms (the treadmill the frozen ramp-up lacks).** The deep-time viewer steps `step_deep_time` (`deeptime.rs:319`), which calls `convection_step` with the same `ColumnParams` every tick, so the radiogenic heat production never decays and each column asymptotes to a fixed temperature and stays there. Crust growth is monotonically non-decreasing: `crust_growth` clamps the deficit non-negative (`deeptime.rs:295`) and the fold is a `saturating_add` (`deeptime.rs:350`), with the doc explicit that "a made crust does not un-form when the mantle cools" (`deeptime.rs:313`). So volcanism relaxes to zero, the interior freezes at steady state, and the surface saturates, exactly the class the owner named. The terms that would turn this ramp into a treadmill all exist and are dormant: `secular_step` decays the reservoir so the interior perpetually cools; `relax_to_support_bound` collapses over-tall relief; the four `surface_drivers` wear the surface down; `stellar_luminosity_ratio` brightens the star. Not one of them is called in the viewer loop.

**Cluster two, the surface-process erosion substrate (built for a lane that never armed it).** The four-driver continuous mass budget (gravity-downslope, fluid-shear, thermal-chemical alteration, deposition) plus the four-reservoir conservation ledger and the snapshot-apply reconciliation are complete and blind-panel audited, but they were built for the genesis-forward Stage 3 run-path lane that never armed, and they are framed as one-time relaxation passes rather than a perpetual treadmill. So the erosion the deep-time relief needs exists in a different lane from where the relief is built, and neither is wired to a live loop. Making them perpetual reframes the driver contract rather than merely wiring it.

**Cluster three, the multi-body solar-system generator (fully dormant, not even the viewer consumes it).** The embryo field, the Chaos-Protocol assembly, the giant-formation verdict and gas ledger, the small-body belt, the moon primitives, and the Laplace-Lagrange secular spectrum are all built, tested, and reference only each other and tests, and five of them self-declare it. The viewer derives independent single worlds through `derive_planet`, and its system map samples five independent orbits, explicitly labelled "NOT an emergent system." So the entire generator that turns a disk into a system of planets, giants, belts, and moons is dark: nothing renders it, nothing steps it. The disk pipeline is wired only up to condensation; the surface-density realization that would feed this cluster is the first dormant link past that cut.

**Cluster four, the mid-band mountain physics (dormant, being wired now).** The T_e construction (moment-equivalence over the yield-strength envelope), the geotherm evaluator, the Byerlee brittle branch, the silicate creep rows, the convective viscosity, the mineral-moduli aggregator, and the flexural filter are the crust and mountains work in flight. They are dormant today and land into the viewer through Seams A (crust moduli, landed), C (direct kei convolution), and D (render). This cluster is the arc being finished, not the arc after it.

**Cluster five, radiative and run-path orphans.** `wien_peak` and `interface_split` have zero consumers. The biosphere is generated once and never stepped in the runner tick (the biosphere run gap). `conflict_pressure` is computed and tested with no run-loop consumer. The Nernst redox uptake-flux is opt-in and inert because Earth binds no redox source. Each is a built capability waiting on a consumer that a later arc supplies.

---

## 4. The missing and partial inventory (Question 2)

Where a physics vector is absent or only half-built, the shape below names it, what exists, and what it needs. These are ordered by how many downstream arcs each one gates.

| Vector | State | What exists | What it needs |
|---|---|---|---|
| Atmosphere radiative balance (greenhouse) | Partial: kernels present, closure absent | `radiant_emission`, `radiative_equilibrium`, `optical_depth`, `wien_peak`, `interface_split` kernels; a static gas-mix composition for sky colour (`materials/atmosphere.rs`) | The closure that derives an atmospheric optical depth from the composition and drives `surface_balance_temperature`, so a planet's surface temperature emerges from its air; a dynamic composition field; per-cell albedo |
| Hydrosphere, weather, erosion (R-HYDROSPHERE-WEATHER) | Flagged research question, unbuilt | The erosion drivers (dormant), the condensation and ice-sublimation substrates, the derived topography and stellar flux | A Chaos-Protocol climate ruling (weather is chaotic, sampled not integrated), the surface-fluid placement (which fluid, where, how much) from the volatile inventory, and the fluvial coupling that wears relief to a steady state |
| Perpetual interior and plate tectonics | Missing the recycling half | The dormant `secular_step` gives perpetual cooling | Plate recycling, subduction, delamination, or overturn, so the crust is not monotonic; the interior treadmill's material half |
| Tidal heating and orbital/tidal axes (Europa) | Missing | `moons::tidal_recession_rate` (orbital recession only); the axis ranges admit Venus/Europa magnitudes | An interior tidal-heating law and the orbital/tidal axes (eccentricity, primary mass, Love number, dissipation) so an ocean stays liquid by its own physics |
| Volumetric z-stacked medium (Europa) | Missing | One medium per (x,y) column, z=0 only | A three-layer ice-over-ocean-over-seafloor medium and terrain, plus a located depth/pressure caller for a submerged being |
| Corrosive-medium-to-tissue harm (Venus) | Missing | Toxin path doses through ingestion only | A coupling from a corrosive ambient medium to bodily harm |
| Dynamic atmospheric composition and escape | Missing | A one-shot equilibrium gas mix for sky colour | A composition field that evolves with outgassing, escape, and (later) life; the shared dependency of the greenhouse, the hydrology physicalization, pollution, and co-evolution |
| Disk-evolution time boundary | Partial | The static disk-gas ledger, the pre-MS luminosity chain, the photoevaporation wind chain | The time-evolving dispersal race that turns `tau_disk` from a consulted clock into a derived output |
| Milankovitch envelope | Partial | `secular_spectrum` (dormant) | Nonzero mode amplitudes from a fragmentation pass, then the climate-envelope consumer that reads the spectrum into the surface |
| Magic and mana (Part 34) | Unbuilt | A bool posture, a cosmetic organ, an unused genome channel | The whole `MagicLaws`/`ManaSource`/`CostModel`/`LimitModel` substrate and a mana field |
| Run-path hydrology physicalization | Partial, gated | The three-regime Rankine-Kirchhoff saturation curve and derived `a_still` kernels, byte-neutral | The runtime swap (physical moisture, physical flux, coherent latent heat), gated on the atmosphere-composition arc; this is a canonical re-pin |
| Run-path productivity derivation | Partial | `carbon_fixation_rate` derives NPP from photosynthesis when armed | The Liebig requirement constants (water, light, temperature) still authored-interim for the abstract producer, retiring when the autotroph substrate lands |
| Biosphere stepping | Missing | `LivingWorld` generated at genesis | Stepping the living world in the runner tick (the biosphere run gap) so ecology runs each tick rather than only at generation |
| Temporal level-of-detail (R-TEMPORAL-LOD) | Missing | The playback clock banks deep-time ticks | The coarse-stepping and event-driven execution the geological-time-versus-sim-time strategy needs, with LOD-invariant contention |

The single vector that gates the most is the atmosphere radiative balance. Its kernels are present and two of them (`wien_peak`, `interface_split`) have no consumer at all. Deriving a planet's surface temperature from its atmosphere is the energy-balance keystone that Venus, the hydrological cycle, weather and climate, emergent pollution, and the Mirror co-evolution all wait on. It is a build rather than a wire, but every piece it composes already exists on the floor.

---

## 5. The research-versus-floor map

Placing each research thread against the floor sorts them into built, designed-not-built, and unresearched. The reference table below indexes the threads; the point it makes is that the deep planetary physics is over-built relative to what is wired, and the coupled surface-system physics (atmosphere, hydrosphere, climate) is where the true frontier sits.

| Research thread (doc) | Status against the floor |
|---|---|
| Deep-time interior, crust, volcanism, impacts (`CONSOLIDATED_SURFACE_PIPELINE.md`) | Built, viewer-wired; monotonic and steady-state (perpetual dynamics missing) |
| Mid-band flexure and T_e (`GEOTHERM_ARC_SCOPE.md`, `TE_CONSTRUCTION_FETCH.md`, `FLEXURE_ARC_SCOPE.md`) | Built, being wired into the viewer now (the crust work) |
| Materials composition-to-properties oracle (`MATERIALS_ORACLE_SPEC.md`) | Built, viewer-wired via crust chemistry |
| Solar-system generator: embryo field, assembly, giants, small bodies (`R_ASSEMBLY_RESEARCH_QUESTION.md`, `DISK_EVOLUTION_ARC_SCOPE.md`) | Built, fully dormant (no viewer consumer) |
| Moon arc (`MOON_ARC_SCOPE.md`) | Primitives built; branch dispatch designed-not-built |
| Secular spectrum and Milankovitch (`PLANETARY_STELLAR_PHYSICS_DERIVE_MAP.md`) | Spectrum built dormant; the climate envelope designed-not-built |
| Disk-evolution time boundary (`DISK_EVOLUTION_EXPANSION_SCOPE.md`) | Static ledger built; time-evolving dispersal designed-not-built |
| Geodynamics surface processes (`GEODYNAMICS_ARC_PROPOSAL.md`, `GENESIS_STAGE3_SURFACE_TRANSPORT_SUBSTRATE.md`) | Built dormant (erosion drivers, isostasy), never armed |
| Hydrosphere, weather, erosion (`R_HYDROSPHERE_WEATHER_RESEARCH_QUESTION.md`, `OCEAN_HYDROSPHERE_TO_RESEARCH.md`) | Flagged research question, unbuilt; needs a Chaos ruling |
| Atmosphere radiative balance / greenhouse (`WORLD_SUBSTRATE_READINESS.md`) | Kernels built, closure unbuilt (the keystone gap) |
| Granular / mass-wasting arc (`GRANULAR_ARC_SCOPE.md`) | Scoped, unbuilt, sequenced behind the hydrosphere |
| Tidal heating and z-stack medium (Europa) | Unresearched substrate gap |
| Magic and mana (Part 34) | Unbuilt pseudocode |
| Emergent pollution and climate change (R-COEVOLVE prerequisites) | Unbuilt; depends on the atmosphere and radiative vectors |
| Mirror as early-Earth initial conditions (R-COEVOLVE) | Flagged north star, unbuilt |
| Temporal LOD (R-TEMPORAL-LOD) | Flagged, unbuilt |

---

## 6. The prioritized roadmap for the arc after the crust

The crust and mountains work draws relief on the derived globe. The natural question the owner already asked is why that relief then sits frozen: no perpetual cooling, no overturn, no erosion, no visible aging. So the arc after the crust is the arc that makes the derived world live and evolve, and it splits cleanly into activations that are one or a few wires from alive and deep builds that each open a new vector.

### The highest-leverage activations (dormant physics one wire from alive)

These are the fastest wins because the physics is already built, tested, and byte-neutral. All are Lane 2 (viewer-side, non-canon), so none moves the run pins.

The perpetual-cooling wire is the sharpest answer to the owner's example. The interior freezes because `step_deep_time` holds the radiogenic heat constant; the dormant `secular_step` decays the reservoir and cools the interior over deep time with no authored knob. Threading a per-column reservoir into `DeepTimeState` and swapping the convection call for the secular call turns the frozen steady state into a perpetually cooling one, so volcanism declines over real time and the tectonic regime can shift as the world ages. The star-brightening wire is smaller: the aging clock already advances and `stellar_luminosity_ratio(age)` is built, so giving `step_deep_time` the aging param and reading it each deep tick makes the star climb the main sequence and the insolation rise. The relief-collapse wire threads the crust's derived shear modulus into `relax_to_support_bound`, retiring the display-side yield-strength flag. Together these three make the derived world visibly age with almost no new physics.

The multi-body assembly activation is the highest-visibility pure win. The entire generator (embryo field, assembly, giants, belts, moons) is dark because the viewer samples independent worlds. Wiring `assemble_system_with_giants` into the system map replaces the five independent `derive_planet` samples with the emergent system the generator already produces, and it lights up a large dormant subsystem in one move. The only real link to build ahead of it is the disk's Lynden-Bell-Pringle surface-density realization that feeds the embryo field, since the viewer's disk pipeline stops at condensation.

The fluid-independent surface weathering activation arms the two erosion drivers that need no hydrosphere (hillslope diffusion and thermal-chemical alteration) onto the deep-time relief, so sharp relief slowly rounds and weathers even before water exists. This is the first half of closing the build-versus-destroy loop, and it reuses the built four-driver budget.

### The deep new builds (each opens a new vector)

The atmosphere radiative-balance keystone is the highest-leverage build because it gates the most. The kernels exist (`radiant_emission`, `optical_depth`, `radiative_equilibrium`, `wien_peak`, `interface_split`) and a static composition is already derived for sky colour, but nothing closes composition into an optical depth into a surface temperature. Building that closure derives a planet's temperature from its own atmosphere, which is Venus at 737 K, Earth's greenhouse, and the energy balance every downstream surface arc needs. It composes only floor kernels, so it authors no new physics.

The hydrosphere, weather, and erosion arc is the project's own stated big arc after the mountain, and it is a flagged research question because weather is chaotic. It needs the owner's Chaos-Protocol climate ruling first (is the climate a seeded draw from a derived statistical measure, and what measure), because that decision is load-bearing and is the twin of the assembly's determinism ruling. With the energy balance from the keystone build and the erosion drivers already built, the arc places the surface fluid (derived from the volatile inventory crossed with the surface phase conditions, so a methane ocean is a data row), sets sea level against the hypsometry, and couples fluvial erosion so relief wears to a steady state between building and destruction. This closes the surface loop and makes an old world look eroded.

The remaining builds are the interior recycling half of the treadmill (plate recycling, subduction, or delamination, so the crust is not monotonic), the Europa vector (an interior tidal-heating law, the orbital and tidal axes, and the volumetric z-stacked medium), the disk-evolution time boundary (turning `tau_disk` into a derived output), the Milankovitch envelope (a fragmentation pass for mode amplitudes, then a climate consumer), and, at the far horizon, the magic system and the run-path convergence (biosphere stepping, hydrology physicalization, and the co-evolution that folds the derived planet into the ground the beings live on).

### The re-pin discipline

The arc after the crust is structurally pin-free: it lives in the viewer, which writes no canonical state, so the activations and the keystone build move no run pins. The two re-pin candidates are separate and should be scheduled deliberately: arming the genesis Stage-3 surface substrate onto the run path (canonical, moves the pins, connects erosion to Lane 1), and the run-path hydrology physicalization (canonical, gated on the atmosphere-composition arc). Neither belongs in the first moves; both are convergence steps for later.

### Recommended first three moves

The first move is the perpetual-dynamics activation, bundled as one arc: wire `secular_step` (perpetual cooling and declining volcanism), the star-brightening luminosity coupling, and `relax_to_support_bound` (relief collapse), and arm the fluid-independent weathering drivers. This is mostly built and dormant, it is viewer-side and moves no pins, and it is the most direct response to the owner's named gap. Its only dependency is the drawn crust and relief, which the in-flight work delivers.

The second move is the atmosphere radiative-balance keystone: close the greenhouse so a planet's surface temperature emerges from its atmosphere. It is a build rather than a wire, but every kernel it composes is on the floor, and it unlocks the energy balance that the hydrosphere, the climate, the pollution vector, and the co-evolution north star all depend on. Two of its kernels have no consumer today, so it also retires a pair of orphans.

The third move is the hydrosphere, weather, and erosion arc, opened by surfacing the Chaos-Protocol climate ruling to the owner. With the energy balance from move two and the erosion drivers from move one, this arc places the surface fluid, samples the climate as a seeded draw from a derived measure, and couples fluvial erosion so the relief the mountains built wears to a steady state. It closes the surface loop and is the project's own next major arc after the mountain.

Running alongside these, as an independent high-payoff activation with no dependency on them, is wiring the dormant multi-body assembly into the viewer's system map, which lights the largest single dark subsystem in the substrate in one move.
