# Abiogenesis closure formalism: correction to the fixed-axis sketch

Owner-directed follow-up, 2026-07-19, responding to the design review on PR #207. This document is a design-first correction to `ABIOGENESIS_DERIVE_FIRST_RESEARCH.md`. It changes no code, sets no value, and does not register a resolved research item. Until the parent document is consolidated, this addendum supersedes its fixed-field `ClosureReport` sketch and its placement of heredity inside organizational closure.

## The seam

The parent document represented closure through five named fields:

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

That representation freezes a taxonomy one level below the mechanism. It also mixes three different kinds of quantity:

1. `catalytic`, `energetic`, and `boundary` are diagnostic readings of physical organization.
2. `hereditary` is a property of reproduction and lineage continuity.
3. `persistence_time` is a temporal result.

The five fields are not an exhaustive formal basis. Leaving them as the universal report would create an alien-generality defect: a physically closed organization whose constitutive constraints do not map cleanly onto those names could be missed by the analyzer even though the causal graph is present.

## The decision

The closed universal object is a typed process-constraint graph extracted from realized physics. Named closure kinds are optional diagnostic projections over that graph. Heritable lineage is evaluated separately.

The world does not declare its own criterion for life. It supplies physical structures, processes, fluxes, production histories, and partition events. Fixed Rust constructs and evaluates the generic causal graph. A versioned, data-defined analysis registry supplies named projections for inspection and validation. A projection may be absent or unrecognized without preventing the graph-level closure test.

This preserves observer independence. The analyzer reads organization already present in written state and does not grant any structure a causal power, persistence advantage, or reproductive capability.

## The formal substrate

### Processes

A process transforms matter, energy, charge, momentum, information-bearing structure, or spatial organization over a stated domain and time scale.

```rust
pub struct ProcessNode {
    pub id: ProcessId,
    pub domain: SpatialDomain,
    pub time_scale: Band<Fixed>,
    pub transform: ProcessPattern,
    pub flux_receipt: ReceiptId,
}
```

Examples include a chemical reaction, diffusion through an interface, polymer extension, repair of a surface, fission of a compartment, or transport driven by a field. The type does not contain biological names.

### Constraints

A structure is a constraint relative to a process and a time scale when two conditions hold:

1. Its presence changes the accessible dynamics or outcome distribution of the process.
2. The aspects responsible for that causal difference remain sufficiently conserved over the process time scale.

```rust
pub struct ConstraintNode {
    pub id: ConstraintId,
    pub structure: StructureId,
    pub relevant_state: StateProjectionId,
    pub action_time_scale: Band<Fixed>,
    pub degradation_time_scale: Band<Fixed>,
    pub provenance: ReceiptId,
}
```

A catalyst, mineral surface, pore, interface, template, scaffold, field-maintaining structure, or later machine can instantiate the same type. Constraint identity comes from physical content and the state projection that carries its causal role, not from an authored enum of biological functions.

### Edges

The graph contains two mandatory causal relations.

```rust
pub enum OrganizationEdge {
    Constrains {
        constraint: ConstraintId,
        process: ProcessId,
        effect: ProcessModification,
        counterfactual_gap: ResidualBand,
    },
    ProducesOrMaintains {
        process: ProcessId,
        constraint: ConstraintId,
        maintenance: ResidualBand,
    },
}
```

`Constrains` records the measured or computed difference between the process with and without the structure. `ProducesOrMaintains` records the material and energetic path that constructs, repairs, replaces, or stabilizes the constraint over its degradation time scale.

Both edges require derivation receipts. A prose assertion that an enzyme catalyzes a reaction or that a membrane maintains a compartment is not an edge until the physical event path and its residual exist.

## Organizational closure

Let `C` be a candidate set of constraint nodes and `P` the processes connected to them. The component realizes overall organizational closure when every constraint in `C` is both dependent and generative:

1. Each constraint is produced or maintained by at least one process whose operation depends on one or more constraints in `C`.
2. Each constraint contributes, directly or through constrained processes, to producing or maintaining at least one constraint in `C`.
3. Every claimed maintenance path closes its material, energy, charge, and state residuals within its declared bands.
4. The component is evaluated across the set of time scales required to include both constraint action and constraint renewal.

Strict closure additionally requires that the component cannot be decomposed into independent closed components under the same boundary conditions. The implementation may use a strongly connected component as a candidate generator, but a graph-theoretic cycle alone is insufficient. Every edge must carry a positive causal and maintenance residual rather than a bare topological relation.

The system remains thermodynamically and usually materially open. External photons, reactants, heat gradients, electron donors, or fluid fluxes are legitimate inputs. They are recorded as open fluxes, not mistaken for failures of organizational closure.

```rust
pub struct OrganizationalClosureReport {
    pub component: ClosureComponentId,
    pub constraint_support: BTreeMap<ConstraintId, ConstraintSupport>,
    pub open_fluxes: ConservationLedger,
    pub external_dependencies: Vec<ExternalDependency>,
    pub missing_edges: Vec<MissingEdge>,
    pub persistence: PersistenceReport,
    pub projections: Vec<ClosureProjection>,
    pub provenance: ReceiptId,
}
```

```rust
pub struct ConstraintSupport {
    pub constrains: Vec<ProcessId>,
    pub produced_or_maintained_by: Vec<ProcessId>,
    pub action_gap: ResidualBand,
    pub maintenance_residual: ResidualBand,
    pub action_time_scale: Band<Fixed>,
    pub renewal_time_scale: Band<Fixed>,
}
```

`persistence` remains in the report because a closed component can be transient, but it is orthogonal to the closure membership test. The report should expose transient closure, marginal closure, and robust closure without hiding them behind one label.

## Diagnostic projections are data-defined

Catalytic, energetic, boundary, repair, developmental, regulatory, and future alien-relevant readings are projections over the generic report.

```rust
pub struct ClosureProjectionDef {
    pub id: ClosureProjectionId,
    pub evaluator: ClosureProjectionKernelId,
    pub validity: ValiditySpec,
    pub required_relations: Vec<RelationPattern>,
    pub provenance: ProvenanceKey,
}
```

```rust
pub struct ClosureProjection {
    pub id: ClosureProjectionId,
    pub result: ResidualBand,
    pub missing_inputs: Vec<MissingEdge>,
    pub provenance: ReceiptId,
}
```

The evaluator set is fixed Rust. Registry membership is engine data and may grow as the research programme adds a defensible reading. The world supplies no projection definition and receives no causal advantage from matching one.

The initial registry may include:

- `catalytic`: internally maintained structures accelerate or redirect reactions supporting the component;
- `energetic`: constrained processes capture an external disequilibrium and route it into constraint renewal;
- `boundary`: internally maintained constraints localize the component or selectively condition exchange;
- `repair`: damage to constitutive constraints activates processes that restore their causal state;
- `regulatory`: constraints alter process selection in response to internal or external state;
- `developmental`: inherited or persistent constraints reconstruct an organized state through a time-ordered path.

These names aid validation and inspection. None is the universal definition of closure.

## Lineage capability is a separate derived question

Organizational closure does not imply reproduction or heredity. A persistent self-maintaining component may never divide. A reproducing compartment may partition material without reconstructing the parental organization. A copying polymer may propagate without sustaining a closed organization.

The origin-of-evolving-life target therefore composes two reports:

```rust
pub struct LifeReport {
    pub organization: OrganizationalClosureReport,
    pub lineage: LineageCapabilityReport,
}
```

```rust
pub struct LineageCapabilityReport {
    pub parent_events: Vec<LineageEventId>,
    pub inherited_constraints: BTreeMap<ConstraintId, InheritanceBand>,
    pub reconstruction_residual: ResidualBand,
    pub heritable_variation: VariationReport,
    pub descendant_persistence: Band<Fixed>,
    pub missing_edges: Vec<MissingEdge>,
    pub provenance: ReceiptId,
}
```

A lineage-capable result requires physical parent-descendant continuity, inherited constraint state or a reconstructive template, descendant reconstitution within band, and nonzero heritable variation where evolution is claimed. No mutation-rate or fitness scalar enters the mechanism.

The user-facing life label remains a derived summary. The organizational and lineage reports are authoritative because they preserve marginal and partial cases.

## Gap-Law handling

Closure is not a Boolean threshold over one score. Each causal edge and maintenance residual carries a band. The analyzer must preserve branch uncertainty when any edge can change component topology within its band.

A well-gapped component may receive a stable organizational report. A near-degenerate component carries alternative graph realizations. A sub-resolution relation may consume a named seeded draw only after the provenance ladder is exhausted, and the draw remains visible as contingency. The analyzer cannot choose the graph that makes the system alive.

## Numerical representation liveness

The convection work supplied a general numerical failure that applies directly to prebiotic chemistry: a load-bearing term can quantize to zero before any solver residual reports a problem. This is not gradual precision loss. It removes a physical edge from the graph.

Every numerical representation tier must therefore pass a liveness preflight over its declared validity domain:

1. Every load-bearing source, sink, and coupling term is representably nonzero where physics says it is nonzero.
2. Every branch-discriminating residual retains enough range to separate its declared band.
3. No required intermediate saturates to a common cap that erases candidate ordering.
4. Conservation residuals remain representable below the accepted physical error band.
5. Promotion and demotion between integer-count, direct fixed-point, logarithmic, or higher-precision forms preserve the same physical state within band.

```rust
pub struct RepresentationLiveness {
    pub live_terms: Vec<TermLiveness>,
    pub vanished_terms: Vec<TermLiveness>,
    pub saturated_discriminants: Vec<TermLiveness>,
    pub promotion_required: bool,
}
```

A vanished source term or saturated discriminator is a representation refusal. The caller must change the declared unit scale, remain in logarithmic form, use integer event counts, or promote precision before integration. The solver may not silently continue with a causal edge removed by arithmetic.

This preflight belongs in R-ABIOGENESIS-0 as part of the constitution and in every later numerical tier's validation battery.

## Staging delta

The parent research packet's stages change as follows.

### R-ABIOGENESIS-0

Add the process-constraint distinction, the graph-level closure definition, the projection-registry rule, the separate lineage gate, and the representation-liveness preflight to the boundary constitution.

### R-ABIOGENESIS-5 through R-ABIOGENESIS-7

Every catalyst, interface, polymer, compartment, repair route, and copying path emits constraint and process nodes plus causal edges. No module writes a high-level closure field directly.

### R-ABIOGENESIS-8

Build graph extraction, residual-backed closure analysis, strict-component decomposition, optional projection evaluation, and the separate lineage-capability report. The `closer` remains downstream and non-causal.

### R-ABIOGENESIS-2

Keep the runtime receipt schema open until the in-flight provenance audit lands. Its findings on bypassable gates, unexercised convictions, source custody, and claim-level linkage are direct design inputs rather than post-build checks.

## Validation delta

The validation battery gains the following cases:

- a graph cycle with no positive counterfactual constraint effect must not count as closure;
- a catalytic RAF without internally maintained constraints reports catalytic projection but no organizational closure;
- a closed organization without fission reports organization without lineage capability;
- physical division without reconstructive inheritance reports reproduction-like partition without lineage capability;
- a boundary-free organization can close if its physical constraints and fluxes support it, while the boundary projection remains absent;
- a novel constraint role with no registered projection still participates in graph-level closure;
- removing one maintenance edge breaks or branches the report according to its residual band;
- a load-bearing term that rounds to zero triggers representation refusal before the first integration step;
- changing only analysis projection membership does not alter canonical world state.

## Source basis and provenance status

The formal direction is grounded by two primary source leads:

- Montévil, Maël, and Matteo Mossio. 2015. "Biological organisation as closure of constraints." *Journal of Theoretical Biology* 372: 179-191. DOI `10.1016/j.jtbi.2015.02.029`. It distinguishes processes from constraints at stated time scales and defines closure through mutual dependence among constraints.
- Hordijk, Wim, and Mike Steel. 2015. "Autocatalytic sets and boundaries." *Journal of Systems Chemistry* 6: 1. DOI `10.1186/s13322-014-0006-2`. It shows that autocatalytic-set and boundary analysis can be composed without treating either as the exhaustive definition of living organization.

These are citation leads, not closed repository provenance. Before their equations, definitions, or exact wording enter code or the maintained design, they must pass the source pipeline with primary bytes or a licensed citation-plus-witness record, exact anchors, checksums, scope, and secondary cross-checks.

## Result

The review seam changes the architecture rather than adding one more closure kind. The universal substrate now has one formal causal object, the process-constraint graph. The named biological readings become extensible, non-causal projections. Heredity moves to the lineage report. Persistence stays orthogonal. Novel alien organization can therefore be detected from its causal structure even when no current projection gives it a familiar name.