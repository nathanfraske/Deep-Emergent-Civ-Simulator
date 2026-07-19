# Abiogenesis derive-first research: from planetary material history to causal lineages

Owner-directed synthesis, 2026-07-19. Design-first and grounded against the current repository. This document changes no code, sets no value, and does not register a resolved research item. It defines the causal boundary, the missing substrate edges, the provenance contract, and the staged build needed before the existing biosphere generator can be replaced or preceded by a physical origin-of-life path.

## The question

How can the simulator begin with a star and its disk, derive a planet and its chemical history, and permit evolving life to arise without seeding a biochemical outcome, a starter organism, a chosen metabolism, or a probability that life appears?

The answer must satisfy the same constitution as the rest of the engine. Every world-content value derives from the physics floor and the situation, or enters as explicit world data. The mechanism must admit alien chemistry as data. Chaotic or sub-resolution outcomes use seeded draws from derived measures. Missing chemistry escalates or refuses rather than producing a confident result. A sterile world is a valid terminal state.

## The verdict

The canonical abiogenesis path may consume only the planet's computed material-and-energy history, plus sub-resolution draws from physically derived measures. It may not consume a life flag, a preselected biochemical system, a guaranteed autocatalytic network, a genome, or an authored probability that life appears.

No new universal numerical constants are required. The work needs new universal mechanisms and representations, a large measured or compute-once chemistry cache, explicit estimator bands, several model-closure fields, and substantial written state. None of those is a new constant of nature.

The remaining work is mainly topological. The repository already has much of the lower machinery, but six edges remain disconnected:

1. Disk composition does not yet become a conserved per-body material ledger through planetary assembly.
2. The assembly projector has causal merge order but no physical event chronology for thermal and chemical consumers.
3. Planetary history does not yet produce a spatial field of prebiotic reactors.
4. Equilibrium thermochemistry does not yet feed an open, reversible kinetic reaction network.
5. Chemical organization does not yet produce persistent constraints, compartments, copying, and causal lineages.
6. Static source provenance does not yet provide a complete runtime explanation from an output bit pattern back to exact input bits, kernel code, source claims, bands, branches, and random counters.

## Repository grounding

### The solar-system side is advanced but chemistry-incomplete

`crates/sim/src/planetary_system.rs` derives an oligarchic embryo field from the disk. `crates/sim/src/planetary_assembly.rs` relaxes that field under the Chaos Protocol, using a seeded draw from a derived instability measure rather than a fixed-point path integral of Lyapunov-sensitive dynamics. This is the correct determinism boundary for planetary architecture.

The body crossing that seam is still structurally narrow. `Embryo` and `SystemPlanet` carry orbit and mass. `MergeEvent` carries the two parents, the merged body, and causal merge order. They do not carry elemental or isotope inventories, molecular material, reservoir partitions, volatile loss, shock products, oxidation state, differentiated layers, or a physical event time. The assembly therefore conserves bulk mass while dropping the typed material identity that a downstream chemical history requires.

`crates/sim/src/planet.rs` supplies the capstone integration spine for stellar structure, disk temperature, radius, and gravity. The Hadean battery in that module still names final mass, bulk density, differentiated interior, outgassed atmosphere, and basaltic surface tiles as pending upstream outputs. The atmosphere derive map also states that the coupled gas-composition solve remains design-ahead of the tree.

Abiogenesis should begin only after those upstream outputs exist as causal data. A chemical origin running over an authored Earth fixture would validate the wrong system.

### The materials architecture is the correct foundation

`crates/materials/src/contract.rs`, `verdict.rs`, and `log.rs` already provide the right selection constitution:

```text
proposer -> disposer -> freezer
```

The proposer enumerates candidates. The disposer scores them through one sealed Gap-Law dispatch. The freezer writes path-dependent realized state. Candidate identity is content-addressed. Winner gaps, validity, provenance keys, escalation, and seeded draws are first-class rather than conventions.

`crates/materials/src/equilibrium_condensation.rs` adds a deterministic fixed-element chemical-potential solve. It minimizes Gibbs free energy under elemental conservation and proves reference-shift invariance. This is a useful thermodynamic inner solve for speciation and phase equilibrium. Abiogenesis needs a kinetic sibling over an open spatial system rather than an extension that turns the equilibrium disposer into a time integrator.

### The current biology path starts above the origin-of-life seam

`crates/sim/src/genesis.rs` currently generates a mature biosphere per region, runs a pre-dawn evolutionary radiation, and places surviving organisms. `crates/sim/src/biosphere.rs` creates species with trophic depth, niches, body plans or grown bodies, gene pools, food-web edges, and extinction state. Its development fixture supplies locus counts, founder pool sizes, niche ranges, trophic layers, rootedness priors, ploidy, and morphogen variance.

That system is useful as a mature-biosphere generator, accelerated initialization route, and downstream civilization fixture. It is not an origin-of-life mechanism. The physical ordering must be:

```text
prebiotic chemistry
    -> molecular lineages
    -> protocellular or microbial evolution
    -> organism and species abstraction
    -> current biosphere and civilization layers
```

The existing generator should remain available under an explicit fixture or accelerated-start status. It should not be retrofitted by placing an `abiogenesis_probability` check before `generate()`.

### The repository already identified the first chemistry seam

`docs/working/LIFE_DERIVATION_FRONTIERS_SCOPING.md` identifies local mass-action kinetics in compartments, driven by free-energy gradients under conservation, as the abiogenesis primitive. The rate-law work has since built a domain-neutral Arrhenius and Eyring kernel. The backlog records abiogenesis as a future consumer that should replace `laws::reaction`'s hard temperature barrier with a smooth rate.

That is the correct immediate direction. The claim that the prebiotic reaction network already derives is ahead of the implementation. The repository has reaction energetics, thermochemical candidates, fixed-point exponentials, and a domain-neutral rate primitive. It does not yet have open molecular identity, reaction-channel generation, transition-state competition, spatial populations, catalysis, polymer sequences, interface transport, or parent-descendant molecular continuity.

## The absolute derive-first boundary

The canonical authored world input remains close to the capstone's minimal set:

$$
\mathcal C_0 = \{M_\star,\; \mathbf{x}_\star,\; \text{disk state},\; \text{orbital or embryo seed}\}.
$$

The biological result lies downstream:

$$
\begin{aligned}
\mathcal C_0
&\rightarrow \text{stellar spectrum and history}\\
&\rightarrow \text{disk condensation and body inventories}\\
&\rightarrow \text{planetary assembly and differentiation}\\
&\rightarrow \text{atmosphere, oceans, crust, and gradients}\\
&\rightarrow \text{local chemical populations}\\
&\rightarrow \text{kinetic reaction networks}\\
&\rightarrow \text{persistent catalytic constraints}\\
&\rightarrow \text{compartments and copying}\\
&\rightarrow \text{causally continuous lineages}\\
&\rightarrow \text{derived biological abstractions}.
\end{aligned}
$$

Every arrow may fail. A world with a solvent and complex organics but no persistent heredity is a valid output. A world with several independent origins is also valid.

### Legal uses of a seed

A seed may select a per-world physical contingency already licensed by the engine, a sub-resolution molecular population drawn from a distribution derived from flux and volume, a chaotic branch selected from a derived measure, material delivered by a computed impactor, or a physical partition of stored molecular state.

### Illegal uses of a seed

The canonical path must not contain `life_present`, `abiogenesis_probability`, an RNA-world switch, a metabolism selection, a preconstructed protocell, a starter genome, a fixed mutation probability, or a guaranteed autocatalytic set. A measured reaction row is legitimate. A hand-selected reaction network whose topology was chosen because it yields life is an authored outcome.

## The causal pipeline

The full path has four different mathematical regimes. Keeping them separate prevents one solver from accumulating incompatible responsibilities.

1. **Equilibrium and fast speciation.** Chemical-potential and phase-equilibrium solves determine fast local distributions under a fixed inventory and environment.
2. **Kinetic reaction and transport.** Reversible reaction channels, diffusion, advection, adsorption, radiation, and interface exchange advance molecular populations through time.
3. **Metastable written state.** Polymers, interfaces, trapped phases, compartments, gradients, and catalyst inventories persist as history.
4. **Derived classification.** Autocatalysis, closure, heredity, reproduction, fitness, and life are reads of the realized history. They never feed the chemistry as privileged facts.

The same world may hold many reactor cells, each on a different branch. The origin is local and historical rather than a scalar property of the planet.

## Structural edge 1: material identity through planetary assembly

Abiogenesis requires typed mass, not bulk mass alone. Each embryo and final planet needs a content-addressed inventory reference whose ledger conserves nuclei, isotope identity, charge, and named energy or redox reservoirs.

```rust
pub struct BodyInventory {
    pub isotopes: IsotopeLedger,
    pub elemental_reservoirs: ElementReservoirLedger,
    pub phases: PhaseInventory,
    pub molecular_inventory: SpeciesInventory,
    pub redox_capacity: RedoxLedger,
    pub provenance: ReceiptId,
}
```

`Embryo` and `SystemPlanet` should carry an `InventoryId`, not duplicate the whole ledger. The inventory derives from the disk condensation and parent-body alteration history. The current solid-disk refractory fraction is an interim compression. It must eventually become the condensed material distribution computed at the body's formation region and epoch.

A merge must satisfy, per conserved isotope and carrier:

$$
I_{\rm inner} + I_{\rm outer}
=
I_{\rm planet}
+ I_{\rm debris}
+ I_{\rm escaped}.
$$

The impact disposer determines retention, differentiation, vaporization, atmospheric escape, orbiting debris, shock products, redox changes, and delivered surface-accessible material. The current perfect-merge zero-debris branch is correctly surfaced as a named residual, but it cannot remain the chemistry-bearing canonical branch. Volatile loss and debris strongly condition the later reactor inventory.

An organic inventory is not a scalar. It is a molecular or functional-group population produced by disk and parent-body chemistry, then transformed by heating, irradiation, aqueous alteration, and impact shock.

## Structural edge 2: a physical history for chemical consumers

The current merge `epoch` is causal order rather than physical time. That is correct for the stability projector. Abiogenesis nevertheless depends on when the final magma ocean freezes, when stable surface liquids appear, how long sterilizing impacts continue, when late volatiles and reactive material arrive, and how much uninterrupted reactor time exists.

Do not invent timestamps inside the assembly projector. Add a separate history-realization pass that samples a chronology from a derived conditional measure:

$$
P(t_1,\ldots,t_n,\; v_{\rm impact},\; \theta_{\rm impact}
\mid \text{assembled graph},\text{disk clock},\text{masses},\text{orbits}).
$$

```rust
pub struct PhysicalMergeHistory {
    pub events: Vec<TimedMergeEvent>,
    pub provenance: ReceiptId,
}

pub struct TimedMergeEvent {
    pub merge: MergeEvent,
    pub time_band_myr: Band<Fixed>,
    pub realized_time_myr: Fixed,
    pub impact_velocity: Band<Fixed>,
    pub impact_angle: Draw<Fixed>,
    pub energy_partition: ImpactEnergyLedger,
}
```

This preserves the Chaos Protocol. The engine still does not claim a precise N-body trajectory. It supplies a physically meaningful chronology because a downstream thermal and chemical consumer requires one. The distribution, its band, and the realized draw all remain visible.

The history writes into `DeepTimeState`: impact heating, volatile delivery and loss, crustal reset, hydrothermal circulation, crater reservoirs, and the interval since the last sterilizing event. Each quantity remains ordinary written state with a derivation receipt.

## Structural edge 3: a field of prebiotic reactors

Abiogenesis must not run in one homogeneous global ocean. The planet should emit a changing graph of local reactor domains:

```rust
pub struct ReactorCell {
    pub region: RegionId,
    pub geometry: ReactorGeometry,
    pub phases: PhaseVolumes,
    pub inventory: MolecularPopulation,
    pub surfaces: Vec<SurfacePatch>,
    pub temperature: Fixed,
    pub pressure: Fixed,
    pub radiation: SpectralFlux,
    pub electric_field: Fixed,
    pub fluid_flux: FlowState,
    pub exchange_edges: Vec<TransportEdge>,
}
```

The planetary solver provides the underlying fields: atmospheric composition and pressure, stellar spectrum, precipitation and evaporation, ocean and lake coverage, geothermal gradients, volcanic and hydrothermal flow, mineral surfaces and defects, lightning and plasma deposition, impact heating, tides, freeze-thaw cycles, and wet-dry cycles.

A pond, vent, ice brine, aerosol, pore network, or lightning channel is a derived classification of geometry and flux history. It does not select a special chemistry module. The same kinetic machinery reads different local data.

Quantities such as pH, redox potential, ionic strength, water activity, fugacity, and saturation generally derive from the current composition and phase state. They should not enter as independent reactor knobs when the composition already determines them.

The reactor graph should support open boundaries. Material, heat, photons, electrons, and fluid cross edges. Far-from-equilibrium maintenance comes from those fluxes rather than a life-specific source term.

## Structural edge 4: reversible kinetic reaction channels

The canonical prebiotic path should operate on chemical potentials and activities:

$$
\mu_i = \mu_i^\circ(T,P) + RT\ln a_i
$$

and

$$
\Delta_r G = \sum_i \nu_i \mu_i.
$$

For an elementary channel, the forward rate has the domain-neutral transition-state form already anticipated by the rate-law kernel:

$$
\ln k_f
=
\ln\left(\kappa\frac{k_B T}{h}\right)
-
\frac{\Delta G_f^\ddagger}{RT}.
$$

The reverse channel does not receive a separately authored rate. Detailed balance supplies it:

$$
\frac{k_f}{k_r}
=
\exp\left(-\frac{\Delta_rG^\circ}{RT}\right),
$$

so

$$
\ln k_r
=
\ln k_f
+
\frac{\Delta_rG^\circ}{RT}.
$$

A catalyst may alter the activation barrier, encounter geometry, transmission factor, or transport path. It must not change the reaction free energy or create a favorable equilibrium.

### Candidate generation

The molecular proposer lazily generates locally legal atom-mapped graph transformations. Initial reaction families should include bond formation and cleavage, proton and electron transfer, radical reactions, substitution and addition, condensation and hydrolysis, oxidation and reduction, photodissociation, adsorption and desorption, surface-mediated transformations, and polymer extension and cleavage.

Every candidate must conserve nuclei by isotope, net charge, electrons or named redox reservoirs, and mass-energy across the full coupled event. Candidate identity is the atom-mapped transformation plus its participants, not an enum such as `amino_acid_synthesis` or `replication`.

The lookup ladder is:

```text
measured channel
    -> compute-once quantum or transition-path result
    -> validated surrogate
    -> mechanism-class estimator with propagated band
    -> unresolved or refused
```

### The exponent rule

An estimator-grade activation barrier must never collapse to one number inside the exponential. If

$$
\Delta G^\ddagger \in [G_-,G_+],
$$

then

$$
k \in
\left[
A e^{-G_+/(RT)},
A e^{-G_-/(RT)}
\right].
$$

If that interval spans regimes that alter network topology, the disposer escalates or preserves both branches. A mean barrier is not an acceptable tiebreak. This is the highest-leverage instance of the project's rule that estimator uncertainty must remain visible through exponential consumers.

### Numerical representation

Reaction networks span concentrations and rates far beyond a single direct Q32.32 SI scale. Canonical chemistry should therefore use integer counts at low copy number, fixed or log concentrations at high copy number, dimensionless chemical potentials, and log rates. Unit systems must be declared by type.

Low-copy-number cells should use an exact stochastic reaction step. High-copy-number populations may promote to controlled leaping or deterministic flux integration once a conservation-preserving promotion contract proves cross-tier agreement. Every random draw keys on world identity, reactor identity, channel identity, and event ordinal rather than thread order.

## Structural edge 5: persistent constraints, polymers, compartments, and heredity

The new universal concept is a persistent conditional constraint: a structure changes the accessible transition rates or transport paths of nearby matter while persisting through the event.

```rust
pub struct ConstraintEffect {
    pub structure: StructureId,
    pub process_pattern: ProcessPattern,
    pub domain: SpatialDomain,
    pub conditions: ConditionSet,
    pub modification: ProcessModification,
    pub lifetime: LifetimeModel,
}
```

`ProcessModification` may alter activation barrier, binding geometry, encounter rate, permeability, diffusion, branching ratio, or coupling between reactions. This one representation covers mineral catalysts, molecular catalysts, templates, membranes, pores, droplets, scaffolds, receptors, and later engineered machines.

### Polymers and copying

A polymer is an ordered molecular structure rather than a genome primitive:

```rust
pub struct Polymer {
    pub backbone: BackboneId,
    pub units: PersistentSequence<MonomerId>,
    pub stereochemistry: StereoState,
    pub modifications: ModificationMap,
}
```

Copying is ordinary competitive chemistry. Candidate units bind. Binding and geometry alter extension rates. Extension forms a bond. Products separate or remain associated. Hydrolysis, damage, side reactions, and mismatches compete. Replication fidelity derives from the gaps among those channels. There is no mutation-rate parameter.

RNA is one possible polymer family in the data. A world may instead discover a different backbone, alphabet, solvent-compatible polymer, or mixed chemistry if the molecular floor and local conditions support it.

### Compartments

A compartment is a connected interface enclosing one or more reactor regions:

```rust
pub struct Compartment {
    pub boundary: SurfaceComponentId,
    pub enclosed_regions: Vec<RegionId>,
    pub topology: BoundaryTopology,
}
```

Permeability, growth, rupture, fusion, and fission derive from boundary composition, molecular defects, phase behavior, surface tension, osmotic stress, material insertion, and external mechanics. A vesicle is not alive because it exists. It becomes biologically consequential only when its contents and boundary participate in persistent causal lineage.

### Heredity and lineage

A fission event physically partitions written state:

```rust
pub struct LineageEvent {
    pub parent: CompartmentId,
    pub daughters: Vec<CompartmentId>,
    pub material_partition: PartitionReceipt,
    pub inherited_structures: Vec<StructureId>,
}
```

A lineage is a derived causal graph over these events. Fitness is a statistic over lineage persistence and descendant production. It is never a driving input.

The first handoff to `civsim-bio` should occur only after a physical lineage exists. A microbial abstraction layer will likely be needed between molecular lineages and the current body-plan species model. Promotion must preserve composition, hereditary structures, reaction networks, compartment topology, material and energy fluxes, damage, lineage identity, and provenance.

## Structural edge 6: detect life without causing it

Add a non-causal analyzer outside the proposer-disposer-freezer loop:

```text
proposer -> disposer -> freezer
                         |
                         v
                       closer
```

The closer reads realized history and reports catalytic closure, energetic closure, boundary or constraint closure, hereditary closure, and persistence. It does not grant metabolism, protect a compartment, improve copying, or trigger reproduction.

```rust
pub struct ClosureReport {
    pub catalytic: ResidualBand,
    pub energetic: ResidualBand,
    pub boundary: ResidualBand,
    pub hereditary: ResidualBand,
    pub persistence_time: Band<Fixed>,
    pub missing_edges: Vec<MissingEdge>,
    pub provenance: ReceiptId,
}
```

Near the classification boundary, the Gap Law applies:

```rust
pub enum LifeVerdict {
    NotClosed(ClosureReport),
    Marginal {
        branches: Vec<ClosureReport>,
    },
    LineageCapable(ClosureReport),
}
```

The continuous report is more informative than the label. A world may sustain metabolism-like flux without heredity, autocatalysis without compartment persistence, copying without energetic closure, or compartments without copying. Those are separate, visible outcomes.

## Chemical source families

The reactor inventory has two legitimate molecular source families. Both write ordinary molecular populations into the same kinetic system.

### Inherited and exogenous material

Disk, comet, and parent-body chemistry may produce or carry simple volatiles, carbon and nitrogen compounds, reduced phosphorus, amphiphiles, aromatic or heterocyclic compounds, and mineral catalysts. Impact delivery computes:

$$
\text{delivered inventory}
=
\text{impactor inventory}
-
\text{shock destruction}
-
\text{atmospheric loss}
-
\text{ejecta loss}
+
\text{impact synthesis}.
$$

Carbonaceous meteorites provide measured evidence for diverse extraterrestrial nucleobases. That supports an exogenous source branch. It does not license seeding nucleobases directly because Earth meteorites contain them. The body-class inventory must derive from formation location, radiation, aqueous alteration, thermal history, and measured parent-body rows until the deeper chemistry is built.

### In-situ production

Local source mechanisms include ultraviolet and particle photochemistry, electrical discharge and plasma chemistry, impact shock, hydrothermal and water-rock reactions, mineral-surface catalysis, wet-dry and freeze-thaw concentration cycles, aerosol and ice chemistry, and radiolysis.

The implementation should retain the causal intermediates. For example:

```text
lightning
    -> electron-energy distribution
    -> excitation, ionization, and dissociation channels
    -> radicals and ions
    -> ordinary reaction network
    -> surviving molecular population
```

A direct `lightning -> amino_acid_yield` table compresses away the chemistry and prevents alien atmospheric compositions from producing their own outcomes.

Origin-of-life experiments should become a regression battery of separate causal edges rather than one master pathway. Cyanosulfidic photochemistry, mineral-assisted oligomerization, wet-dry synthesis, membrane permeability, vesicle growth and fission, template copying, and cooperative catalytic networks each validate a local mechanism under a stated regime. None establishes a complete historical route from geology to life.

## Placement in the existing layering pipeline

| Pipeline location | Abiogenesis addition |
| --- | --- |
| Layer 1 constants | None |
| Layer 1 mechanisms | Chemical potentials and activities; atom, isotope, charge, and electron conservation; reversible mass-action kinetics; domain-neutral rate laws; photon absorption; adsorption; interface transport; physical partitioning |
| Layer 2 cache | Molecular structures, conformers, thermochemistry, solvation, spectra, collision cross-sections, adsorption energies, transition states, activation barriers, diffusivities, and permeability data |
| Layer 2.5 prototypes | Generic reaction topologies, binding-site geometries, polymer backbones, bilayers, pores, droplets, and mineral surface sites; never RNA, ribosomes, cells, or metabolic pathways as privileged templates |
| Layer 3 estimators | Group-contribution thermochemistry, barrier relations, solvation corrections, conformer ensembles, adsorption estimates, diffusion and permeability estimates, and folding or motif surrogates |
| Layer 3 closures | Reaction-grammar coverage, molecular resolution, solvent many-body corrections, transition-state transmission treatment, subgrid mixing, surface-defect populations, molecular crowding, and rare-event acceleration |
| Layer 4 contingency | Existing physical microstate and history draws only; no life or biomolecule seed |
| Written state | Molecular counts, surface occupancy, polymer sequences, conformers, compartments, gradients, catalyst inventories, damage, fission partitions, and the lineage graph |
| Derived analytics | Autocatalysis, constraint closure, protocell, lineage, reproduction, fitness, species, and life verdicts |

The most consequential closure is likely reaction-space resolution: which structures and elementary transformations the representation can express. That defines the simulator's adjacent possible and must be a first-class ledger entry rather than a hidden implementation detail.

## Crate architecture

### `civsim-physics`

Keep the physics crate domain-neutral. Appropriate additions are chemical-potential and activity folds, reversible mass-action propensities, log-rate operations, diffusion and adsorption transport, photon-deposition laws, exact conservation helpers, and unit-safe dimensionless transforms. It should not know what a nucleotide, membrane, organism, or prebiotic pathway is.

### `civsim-materials`

Add a kinetic sibling to the equilibrium thermochemical machinery:

```text
materials::molecular
materials::reaction
materials::kinetic
materials::recognition
materials::interface
materials::polymer
```

The same contract remains:

```text
molecular proposer
    proposes locally legal graph transformations

kinetic disposer
    evaluates free energies, activation bands, rates, validity, and gaps

freezer
    writes metastable molecules, polymers, interfaces, and trapped state
```

A channel evaluation should expose its bands and gap:

```rust
pub struct ChannelEvaluation {
    pub channel: ReactionChannelId,
    pub environment: EnvironmentBucketId,
    pub delta_g_over_rt: Band<Fixed>,
    pub activation_over_rt: Band<Fixed>,
    pub log_rate_forward: Band<Fixed>,
    pub log_rate_reverse: Band<Fixed>,
    pub gap: Fixed,
    pub validity: ValidityVerdict,
    pub provenance: ReceiptId,
}
```

### `civsim-sim`

Add the spatial and historical consumer:

```text
sim::prebiotic::reactor
sim::prebiotic::transport
sim::prebiotic::population
sim::prebiotic::compartment
sim::prebiotic::lineage
sim::prebiotic::closure
```

The simulation owns molecular populations, reactor topology, event scheduling, transport, written state, and lineage. It never asks whether a molecule is biological before applying chemistry.

### `civsim-bio`

The prebiotic path must not depend on `civsim-bio`. The first handoff occurs after a physical lineage exists and a promotion contract proves conservation, referential integrity, determinism, and cross-tier agreement. The current mature biosphere generator remains a development fixture or accelerated start until that bridge lands.

## Runtime derivation receipts and byte-explainability

The static provenance system is already strong. `sources/registry.toml` requires source identity, citation, SHA-256 receipt, archive or pending archive reason, scope, slim or witness extract, licence, and custody. `FloorGrade.sources` closes the floor-to-source hop. `JoinedRegister` performs the worst-case provenance join across calibration and floor nodes.

The missing layer is a runtime derivation DAG that answers:

> For these output bits, which exact input bits, kernel implementation, source claims, uncertainty bands, solver branches, and random counters caused them?

`VerdictRecord` currently stores verdict kind, provenance key, and contingency slot. `DeepTimeState::realization_digest` correctly records the realized state but is not a causal explanation. Abiogenesis needs both.

The crate layering should remain acyclic. Put content-addressed receipt primitives in `civsim-core`. Physics and materials emit opaque receipt nodes with stable kernel and input IDs. `civsim-foundation` resolves source claims and effective provenance. `civsim-sim` stores the receipt graph alongside canonical state.

```rust
pub struct DerivationReceipt {
    pub schema_version: u32,
    pub id: ReceiptId,
    pub output_type: StableTypeId,
    pub output_bits_hash: Hash256,
    pub kernel: KernelReceipt,
    pub input_receipts: Vec<ReceiptId>,
    pub claims: Vec<ClaimId>,
    pub declared_provenance: Provenance,
    pub effective_provenance: Provenance,
    pub numerical_band: ErrorBand,
    pub winner_gap: Option<Fixed>,
    pub validity: ValidityVerdict,
    pub branch: BranchReceipt,
    pub residuals: Vec<ResidualReceipt>,
    pub seed_span: Option<SeedSpan>,
}
```

`KernelReceipt` records a stable kernel ID, source blob hash, equation or mechanism ID, unit-scale contract, solver version, and validity domain.

A claim is narrower than a source artifact:

```rust
pub struct ClaimRecord {
    pub id: ClaimId,
    pub primary_sources: Vec<SourceId>,
    pub secondary_sources: Vec<SourceId>,
    pub locator: SourceLocator,
    pub extract_sha256: Hash256,
    pub transcription_sha256: Hash256,
    pub unit: UnitId,
    pub uncertainty: ErrorBand,
    pub covariance_group: Option<CovarianceId>,
}
```

The source registry proves which artifact was read. The claim record proves where the load-bearing equation, table cell, or value sits in that artifact and how it became canonical bits.

Do not log one receipt per molecule. Use a content-addressed Merkle-style DAG with one receipt per unique cache calculation, reaction-channel and environment bucket, and aggregate event batch.

```rust
pub struct ReactionBatchReceipt {
    pub channel_evaluation: ReceiptId,
    pub initial_state_hash: Hash256,
    pub event_count: u64,
    pub rng_span: SeedSpan,
    pub final_state_hash: Hash256,
    pub conservation_residual: ConservationLedger,
}
```

The audit interface should support:

```text
civsim explain <world> <state-path>

output bits
    -> reaction batch
        -> channel evaluation
            -> transition barrier
                -> exact source claim or compute-once solve
            -> temperature and activities
            -> rate-law kernel blob
        -> prior molecular state
        -> RNG counters
        -> conservation projection
```

New abiogenesis data should receive no provenance grandfathering. Every measured or estimator input must resolve to an exact claim and artifact receipt from its first commit.

## Staged build

The proposed research identifier is `R-ABIOGENESIS`. This document does not register it in the audit backlog. The owner can retain or rename the identifier when the item is formally flagged.

### R-ABIOGENESIS-0: boundary and refusal constitution

Write the types and design gates before causal code. Enforce no `LifeSeed`, no prebiotic dependency on `civsim-bio`, no mutation or fitness scalar, no causal closure analyzer, and no silent fallback for missing barriers or transport properties. A sterile output must pass.

This slice is documentation and type-level scaffolding, dormant and byte-neutral.

### R-ABIOGENESIS-1: material ledger through system assembly

Extend disk annuli, embryos, planets, debris, and merge events with content-addressed material inventories. Required tests cover isotope conservation, charge conservation, deterministic merge order, exact replay from the embryo field and event history, and explicit volatile, debris, and escape residuals.

This is the highest-leverage upstream topology change.

### R-ABIOGENESIS-2: runtime derivation receipts

Add claim records, kernel receipts, value receipts, reaction-batch receipts, and explain traversal. Prove the machinery on a small existing physics calculation before chemistry consumes it.

### R-ABIOGENESIS-3: molecular identity and kinetic channel contract

Add canonical molecular graphs, isotope and formal-charge identity, spin and stereochemistry where the model resolves them, atom-mapped reaction channels, conservation verification, reversible rate evaluation, and interval propagation through exponentials.

Replace the hard Boolean reaction barrier only on the new kinetic path. Keep the existing floor law for backward compatibility until all consumers migrate.

Begin with a small source-grounded chemistry set, but keep the representation open. Unimplemented reaction families are declared coverage walls rather than impossible chemistry.

### R-ABIOGENESIS-4: spatial reactor state

Build reactor cells and transport with synthetic fixtures first: gas, liquid, and solid phases; mineral surfaces; diffusion and advection; phase exchange; radiation and energy flux; and concentration cycles. Use equilibrium or speciation inner solves for fast processes and kinetic steps for slow transformations.

### R-ABIOGENESIS-5: planetary source wiring

Wire one source family per slice:

1. Stellar photochemistry.
2. Volcanic and hydrothermal chemistry.
3. Impact delivery and shock chemistry.
4. Wet-dry and freeze-thaw cycles.
5. Lightning and plasma chemistry.
6. Mineral-surface concentration and catalysis.

Each source writes ordinary species, radicals, excitation, heat, or flux into the same reactor state.

### R-ABIOGENESIS-6: recognition and evolvable polymers

Add sequence-bearing polymers, local binding models, conformation ensembles, template-directed extension, degradation, recombination, and catalytic barrier modification. Copying errors emerge from channel competition and physical damage.

### R-ABIOGENESIS-7: interfaces and compartments

Add amphiphilic assembly, other phase-separated compartments where the chemistry supports them, permeability, osmotic and mechanical state, boundary growth, fusion, rupture, fission, and physical state partitioning.

### R-ABIOGENESIS-8: closure, lineage, and biology handoff

Add the non-causal closure analyzer and lineage graph. Promote to a microbial or biological abstraction only when the conservation and cross-tier contracts pass. Keep the molecular state available beneath the promoted representation for provenance and refinement.

## Validation constitution

Do not use "Earth produced life" as the calibration target. That would validate the replacement against the phenomenon it is meant to explain. Earth is an out-of-sample system-level hindcast after the local mechanisms have been validated independently.

### Mechanism battery

Validate local mechanisms against independent experiments: electrical-discharge chemistry, ultraviolet photochemistry, mineral adsorption and oligomerization, wet-dry synthesis, membrane permeability, vesicle growth and fission, template copying, and autocatalytic or cooperative molecular networks. Each row validates one causal edge under its own conditions.

### Negative battery

The system must produce nulls where the physics demands them. No sustained gradient should relax toward equilibrium. No concentration mechanism should leave polymerization dominated by hydrolysis. Excessive leakage should prevent retained networks. Strong scavenging should suppress radical chemistry. An impermeable boundary should starve its interior. Copying errors above the physically derived information threshold should collapse lineages. A catalyst whose persistence cost exceeds its benefit should disappear.

### Alien-general battery

Run the same machinery across different solvents, atmospheric redox states, condensates, mineral surfaces, monomer alphabets, stellar spectra, and reactor geometries. An Earth-only reaction table may serve as a measured coverage tier. It cannot define the universal candidate space.

### Determinism and tier battery

Repeat every scenario across worker counts. Permuting candidate enumeration must not change a verdict. Promotion between molecule counts, leaping, and deterministic flux tiers must preserve conserved totals and remain inside the declared error band. A moved realization digest must be attributable to a changed derivation, changed source row, changed branch measure, or changed seed span.

## Source acquisition queue

The papers below are candidate primary sources for the mechanism battery. Each must enter the repository through the source registry with a primary and secondary claim witness, exact scope, licence finding, and claim locator before any value or mechanism is treated as landed.

| Mechanism claim | Candidate primary |
| --- | --- |
| Electrical discharge can produce amino acids under a stated gas-water apparatus | Miller, S. L. 1953, *Science* 117, 528-529, DOI 10.1126/science.117.3046.528 |
| A common cyanosulfidic network can produce precursor families for nucleotides, amino acids, and lipids under its stated photochemical regime | Patel, B. H. et al. 2015, *Nature Chemistry* 7, 301-307, DOI 10.1038/nchem.2202 |
| Mineral surfaces can promote substantially longer prebiotic oligomers than the corresponding solution route | Ferris, J. P. et al. 1996, *Nature* 381, 59-61, DOI 10.1038/381059a0 |
| Heterogeneous oligomers can participate in template-directed complementary synthesis | Ertem, G. and Ferris, J. P. 1996, *Nature* 379, 238-240, DOI 10.1038/379238a0 |
| Wet-dry cycling can support continuous formation of canonical and non-canonical nucleosides under a stated geothermal sequence | Becker, S. et al. 2018, *Nature Communications* 9, 163, DOI 10.1038/s41467-017-02639-1 |
| Fatty-acid membranes can retain polymers while permitting activated nucleotide uptake and template copying | Mansy, S. S. et al. 2008, *Nature* 454, 122-125, DOI 10.1038/nature07018 |
| Fatty-acid vesicle growth can produce thread-like structures that divide under modest shear while retaining contents | Zhu, T. F. and Szostak, J. W. 2009, *Journal of the American Chemical Society* 131, 5705-5713, DOI 10.1021/ja900919c |
| Cooperative RNA replicator networks can outgrow competing selfish autocatalytic cycles in the stated in-vitro system | Vaidya, N. et al. 2012, *Nature* 491, 72-77, DOI 10.1038/nature11549 |
| Carbonaceous meteorites contain a broad measured distribution of purine and pyrimidine nucleobases | Oba, Y. et al. 2022, *Nature Communications* 13, 2008, DOI 10.1038/s41467-022-29612-x |
| Exact stochastic simulation of coupled reaction channels | Gillespie, D. T. 1977, *Journal of Physical Chemistry* 81, 2340-2361, DOI 10.1021/j100540a008 |

These rows are validation targets and source candidates, not a canonical historical pathway. The simulator may combine them only through the ordinary causal graph and may derive that none closes on a given world.

## Open walls and owner rulings required before code

The mechanism has no new constant-of-nature request, but several scope and closure decisions need explicit rulings before implementation:

1. **Reaction-space resolution.** Define the molecular size, charge, spin, stereochemistry, and reaction-family frontier carried in the canonical tier, with higher tiers promoted on demand.
2. **Electronic-structure ladder.** Decide which barriers and binding energies require measured rows, compute-once calculations, validated surrogates, or class estimators.
3. **Estimator-in-exponent enforcement.** Ratify that exponent consumers preserve barrier intervals and escalate when the interval changes network topology.
4. **Solvent and activity model.** Choose the first built activity and solvation rung, while keeping the data model open to other solvents.
5. **Spatial resolution.** Define the reactor-cell promotion and demotion rules so pores, droplets, films, and bulk phases conserve material across tiers.
6. **Rare-event acceleration.** Permit only methods whose derived measure and error band can be audited, with no direct boost to a life-coded event.
7. **Life verdict.** Ratify the closure report and physical lineage as the classification basis, with the verdict non-causal.
8. **Biology handoff.** Decide the first microbial abstraction and the state projection required before `civsim-bio` may consume a molecular lineage.
9. **Fixture status.** Mark the current mature biosphere generator as an accelerated or development route once the physical origin path exists, without deleting its test value.

Every numerical residue surfaced by those rulings remains reserved with its basis. No value is set in this document.

## Status

Design-first research synthesis. The derive-first boundary, six structural edges, crate placement, provenance extension, build sequence, and validation constitution are scoped. Nothing here claims the origin-of-life path is built. The immediate work, if the owner opens the arc, is a narrow design gate over `R-ABIOGENESIS-0` and `R-ABIOGENESIS-1`: make the refusal boundary explicit, then carry typed material identity through planetary assembly. The kinetic chemistry should not begin until that upstream material ledger and the runtime receipt spine are standing.
