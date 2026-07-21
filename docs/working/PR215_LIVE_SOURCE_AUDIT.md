# PR #215 live-source audit

Status: implementation audit for the canonical integration on PR #215 branch
`claude/thermoelastic-rung3`, updated 2026-07-21.

This record covers the integration worktree that normalizes the physical floor,
separates SI representation from causal information, and preserves the Stage 1
refusal while the stellar-birth measure remains open.

## Governing invariant

The sealed absolute physics floor is the sole value-bearing input. Provenance,
tier, source custody, and accounting never admit a magnitude. Every later
quantity must derive from admitted earlier bits, or its non-derived leaf must
complete every required derive-first and floor-admission receipt after
exhaustion. Otherwise it remains a named refusal. The runpath must admit alien
physics and must not select the Sun, Earth, a familiar chemistry, a viewer
state, or an authored realization.

## Floor state after remediation

The generated physical-floor inventory contains three Universal entries. The
units layer separately publishes a ten-identity representation view and a
fourteen-identity execution view:

- three `[M]` physical invariants: `alpha`, `G`, and `m_e`;
- seven exact SI representation definitions with no provenance mark;
- one runtime `[D]` event for `eps_0`, derived from `e`, `alpha`, `h`, and `c`;
- three representation-derived values: `sigma`, `R`, and
  `A3_per_cm3_mol`, completing the ten-identity representation view;
- one fourteen-identity execution capability containing that representation
  view, the three admitted physical coordinates, and derived `eps_0`;
- zero `[E]`, `[C]`, `[A]`, `[W]`, and `[X]` entries;
- zero Reference, Residue, and Contingency entries.

The seven marks are `[D]`, `[M]`, `[E]`, `[C]`, `[A]`, `[W]`, and `[X]`.
The four tiers are Universal, Reference, Residue, and Contingency. The census
is generated from the sealed catalog at
`docs/working/CANONICAL_LEDGER_INVENTORY.txt`; it is not maintained by hand.

## Numerical execution boundary

The floor receipt is not globally Q32.32. Each admitted or derived constant is
published as a signed integer plus its own binary scale, chosen from the exact
decimal rational and its projection contract. Values such as `h` and `m_e`
therefore retain more than 32 fractional bits. Existing physical kernels use
`civsim_core::Fixed`, an integer Q32.32 type. A capability method performs the
one explicit rounding into that kernel representation and refuses values that
cannot be represented.

Floating-point comparisons are diagnostic or test oracles only. They do not
author the floor, bind a transcript value, or feed the current canonical
runner. The active GPU crate already implements integer Q32.32 arithmetic,
transcendentals, and field kernels against the CPU `Fixed` bit oracle, but it
is not reachable from the canonical planet package yet. A later typed adapter
must prove exact CPU and target-backend GPU parity before GPU output can enter
the causal bitstream.

## Findings closed in this slice

1. **Candidate declarations could authorize themselves.** The units table was
   both the declaration and the admission authority. The independent ordered
   registries in `civsim_units::physics_floor` now seal physical invariant
   admission, exact SI representation definitions, and every execution
   relation. Planet code can request the sealed floor but cannot define its
   authority. A candidate must match its complete fingerprint, and an extra
   candidate is rejected.
2. **Derived values did not replay from the ancestry bits in the transcript.**
   The gas constant and Stefan-Boltzmann projections were influenced by
   parallel reference decimals. Composite evaluation now consumes only the
   exact rational values represented by the published projected inputs. A
   higher working precision must produce the same output bits.
3. **A measured source decimal was being represented too coarsely.** Floor
   projection now preserves at least the source decimal's last stated place.
   Each `[M]` transcript record carries the source identifier, SHA-256,
   locator, source decimal, exact or standard uncertainty, projection rule,
   and maximum half-ULP projection error. The bit projection is explicit; the
   measured physical quantity is not mislabeled as an exact binary fact.
4. **Identity depended on parallel array order.** Every projected magnitude is
   sealed to its symbol. The canonical floor view names each expected symbol
   at the binding site and rejects relabeling.
5. **Dimensions and ancestry were unchecked text.** Every floor value now
   carries a typed seven-exponent SI dimension. The composite formula parser
   evaluates dimensions with the same operation grammar used for values,
   records every exact symbol read, and rejects either a dimensional mismatch
   or an ancestry mismatch.
6. **A hidden reference-decimal composite path remained callable in normal
   builds.** The old comparison helpers are test-only. Production composite
   construction has one projected-input path.
7. **Richer evidence made a transcript enum needlessly large.** The future
   written-value payload now uses owned indirection. This changes no serialized
   field or causal rule.
8. **Exact SI definitions were counted as measured physical inputs.**
   `Delta_nu_Cs`, `c`, `h`, `e`, `k_B`, `N_A`, and `K_cd` now live in a
   versioned, untagged representation receipt with source custody. They encode
   all seven SI base coordinates but contribute no causal degrees of freedom.
9. **Vacuum permittivity duplicated the electromagnetic coupling coordinate.**
   The floor now admits measured dimensionless `alpha`; `eps_0` is recomputed as
   `e^2 / (2 * alpha * h * c)` and its cited decimal is only a drift oracle.
10. **Universal leaves bypassed derivation exhaustion.** Admission now requires
    a complete receipt for every non-derived leaf at every tier. The three
    current invariant receipts state derivation attempts, Buckingham-Pi budget,
    Gap Law with typed Chaos Protocol, Residual Law, and unique residual slot.
11. **Receipt prose could vary after structural admission.** Canonical preflight
    now exact-matches the sealed receipt fingerprints as well as the ledger, so
    a caller cannot replace the evidence narrative while keeping the same IDs.
12. **Physical helper APIs could bypass the verified floor.** Saha,
    polarizability, opacity, electronic transport, Harrison scaling, d-state
    radius, QEq, lattice modulus, and their active material consumers now
    require the sealed execution capability. Their physical folds derive from
    its published bits. Representation-only conversions use the separate
    noncausal representation view.
13. **Receipt construction and receipt verification shared authority.** The
    units authority now pins an independent length-prefixed SHA-256 digest for
    every ordered physical receipt. Changing constructor prose, budgets,
    attempts, evidence, or residual slots no longer changes the expected
    authority.
14. **Some declaration reorderings were accepted.** Representation,
    execution-relation, and physical-admission registries are all checked by
    exact length and position. A set-equivalent reorder now refuses.
15. **A verified capability could be mutated after construction.** Both SI
    view types now keep every projected value private and expose only read-only
    symbol lookup and derived operations. The private seals and private fields
    jointly prevent caller construction and post-verification reassignment.
16. **Dormant production APIs still crossed from floats into fixed state.**
    Covalent-radius and Badger tables now hold decimal text and parse through
    exact rationals. Perovskite temperature keys now use integer Q32.32
    arithmetic with explicit round-half-to-even behavior. The unused public
    `f32` and `f64` core quantizers were removed. The canonical planet gate now
    rejects floating-point types, and the GPU no-float test scans Stage 0,
    shared primitives, fields, and transcendentals.
17. **Gap Law did not carry its Chaos Protocol as typed evidence.** Admission now
    requires either a not-applicable basis or a nonempty dynamical regime
    partition with a transition law. Each regime proves that input bands remain
    resolved for direct evolution or carries the stationary measure,
    conservation projection, stability, validity, coordinate discipline, and
    exact replay required by a sub-resolution disposition. Empty evidence or an
    empty regime list refuses. Receipt fingerprints and transcripts cover the
    complete ordered partition.
18. **The Stage 1 contract could name leaves but could not evaluate them.** A
    fixed-order evaluator now consumes only two opaque repository-owned proof
    capabilities. It reports the exact unresolved leaf frontier, closes the
    root only when both capabilities exist, and exposes no string, citation,
    scalar, tag, or caller boolean as a closure path.

The resulting projected composite values are replayable from the transcript:

- `sigma`: bits `2042967686`, scale `55`;
- `R`: bits `35710345014`, scale `32`;
- `A3_per_cm3_mol`: bits `7131960987`, scale `32`.

## Source custody

The held floor remains the 2018 CODATA set. This slice did not silently upgrade
values to the later CODATA adjustment.

- NIST 2018 ASCII table, 40,689 bytes, SHA-256
  `8c47c05db62c4d314a5244db51a47b4831616e55a8d357ced373a8620ff43be1`;
- NIST 2018 values PDF, 189,260 bytes, SHA-256
  `0baec5980ef4956f3047fe6b6113a27013483999ceb8078421f4b3acdaa6159a`;
- CODATA 2018 adjustment paper, 2,312,070 bytes, SHA-256
  `6d712bdc99719540bec65c7d1ef11b00f5d321e6083e9e6ed7d3de6fb8062908`.

Live NIST bytes matched their archived witnesses. The source registry carries
the identifiers, hashes, archives, scope, and extract anchors. No source bytes
were added because redistribution rights were not established. Custody proves
what was consulted; it does not admit a physical value.

## Canonical runner receipt

Two direct no-argument runs produced:

- exit code `2` on both runs;
- zero stderr bytes;
- six transcript events;
- no `[W]`, no `[X]`, and no snapshot;
- byte-identical stdout of `27,941` bytes, SHA-256
  `382bb176b8fbb85afeae07f4200925f63c1c00cb1daadf7924d2bfb692a373af`;
- receipt `civsim.planet.run.v6` and transcript schema major `4`;
- refusal `stellar_birth.realization_measure` at Stage 1.

The umbrella refusal has two unresolved leaves:
`stellar_birth.joint_physical_measure` and
`stellar_birth.realization_coordinate_law`. The executable evaluator serializes
those leaves in fixed canonical order. The repository resolver supplies neither
opaque proof capability yet, so the root refusal is the correct output.

## Verified reachability

The active viewer borrows an immutable snapshot and cannot construct or
advance a world. The canonical package depends at runtime only on ledger and
units. Retained star, disk, planet, crust, geodynamics, deep-time, moon, and
flexure code is private in `civsim-planet-substrate`; it is not reachable from
the canonical stages until a typed adapter is admitted. Biology,
civilization, authored world generation, and the old causal viewer remain in
`parked/` and are not canonical readiness evidence.

## Validation

The units, ledger, physics, materials, planet substrate, and canonical planet
targets pass their focused test suites. The complete Linux `check-pr` recipe
passed with exit `0` in `867.6` seconds. It covered hook and gate canaries, all
declarative PR gates, Stone 0, formatting, Clippy with warnings denied, the
ten-package all-target suite, the expanded GPU no-float guard, available
backend bit-parity tests, rustdoc, and doctests.

Two post-gate direct Linux binary runs independently confirmed exit `2`, zero
stderr bytes, `27,941` stdout bytes, byte equality, and SHA-256
`382bb176b8fbb85afeae07f4200925f63c1c00cb1daadf7924d2bfb692a373af`.
No earlier receipt or inventory pin is evidence for the v6 result.

One blind-generalizer pass was run against frozen candidate and specification
packets, as requested instead of the historical six-pass overnight loop:

- candidate SHA-256
  `d91f029fd8c54d1a7a5f9062d1ac21b4505683f27e5fab9d8ae12f49c2542c46`;
- specification SHA-256
  `07f03d9a4bd200fce7fca547635148af4af12d99cbf0db02a4995105827a5eb2`;
- three live findings: unaudited physical API reachability, shared receipt
  construction and verification authority, and order-insensitive declaration
  checks;
- all three findings repaired in this worktree without selecting a desired
  physical outcome.

The executable-leaf and Chaos Protocol slice received one additional
packet-only generalizer pass, not a six-pass loop. Its strongest-model
input-bias smoke first returned `BLOCKED`: the packet treated the initial
three-way chaos classification as exhaustive and tested false admission more
than false refusal. The packet was neutralized with symmetric valid, absent,
and invalid-proof cases plus mixed, stochastic, multi-attractor,
nonstationary, and regime-changing falsifiers; the smoke then returned
`CLEAR` before the auditor ran.

The blind pass found that one exclusive not-applicable, dissipative, or
Lyapunov-sensitive tag could not express mixed or changing dynamics. Source
verification confirmed the enum had that shape. It is repaired as a nonempty
validity-regime partition with a transition law. Each regime now records one
of two resolution dispositions: direct evolution proves admitted input bands
remain resolved, while sub-resolution evolution requires the stationary
measure, conservation projection, stability, coordinate discipline, and
replay gates. Multiple regimes can coexist; unsupported regimes refuse instead
of being misclassified.

The pass also correctly identified the future danger of treating runtime type
and presence as semantic proof. Source verification found no current live
bypass: production has no proof constructors and returns typed absence for
both leaves, while the artifact seals are private. The exact open obligations
now include absolute-floor binding, artifact-schema version,
semantic-checker version, dependency digest, and coordinate-to-joint-measure
binding. Those checkers remain required work before either production proof
constructor may exist. This packet summarized structure rather than carrying
the complete comment-stripped source, so it is evidence for the named
generalization findings only, not a passed full code panel.

## Remaining audit debt

- Admission structures still carry human-readable receipt strings. The current
  three receipts are independently sealed and exact-matched, but future leaves
  at any tier must replace prose-only proofs with typed, machine-resolvable
  artifacts before they can become live.
- Opaque Stage 1 proof types have no production constructors. Future smart
  constructors must validate every obligation, bind the absolute floor,
  schema, checker, dependencies, validity domain, and joint measure, and return
  structured invalid reasons. Runtime type and presence cannot stand in for
  those checks.
- Stage 1 leaf proofs remain absent. The next artifact must implement a
  machine-readable dimensional rank and null-space census rather than
  constructing either opaque proof from human-readable assertions.
- Future contingency coordinates must not rely on a bounded integer domain
  without proving the physical support and mapping law.
- Exact rational helpers need explicit resource and exponent domains before
  formulas from untrusted or expanding catalogs can reach them.

## Audit-process limitation

This was a direct live-source audit with independent value-flow,
admit-the-alien, observer-reachability, and post-seal bypass lenses plus the one
frozen-packet blind-generalizer pass recorded above. It deliberately was not
the repository's standing six-pass panel workflow. The older harness attempts
remained invalid because they lacked required controller artifacts or mutated
the frozen candidate during verification. This record is evidence for the one
identified pass and the named independent reviews only; it must not be cited
as a passed six-pass standing panel.

## Next derivation order

1. Produce a machine-readable derive-first census for the joint stellar-birth
   measure. Start from the admitted floor, enumerate dimensional bases and
   Buckingham-Pi groups, prove what remains underdetermined, and preserve
   correlations rather than authoring independent stellar knobs.
2. For each irreducible survivor, complete source custody, uncertainty,
   support, normalization, conditioning, Gap Law including the typed Chaos
   Protocol branch, Residual Law, and unique residual-slot receipts. If any
   item is incomplete, keep the Stage 1 refusal.
3. Define the realization-coordinate law independently of callers, hidden
   seeds, enumeration order, transcript ordinals, and observer state.
4. Only after both leaves close, issue the first `[X]`, derive `[W]` identity,
   and expose SI-native typed adapters for collapse, disk formation, and embryo
   systems.
5. Carry the same bitstream and refusal discipline through assembly,
   composition, orbits, moons, young thermal state, crust, mantle,
   geodynamics, atmosphere, hydrology, loads, flexure, and immutable snapshot
   transport.
