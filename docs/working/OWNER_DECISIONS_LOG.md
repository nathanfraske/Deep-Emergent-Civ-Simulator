# Owner decisions log

Decisions the owner needs to make, accumulated during autonomous work and surfaced at the end (owner
directive 2026-07-08: derive value gates and move on with dev values; leave decisions for the end, do not
stop mid-work to ask). Each entry states the decision, what I did in the interim (derived or dev-set), and
the basis, so the owner can confirm or override in one pass. This is not a blocker: work continues past every
entry using a derived or dev-set value.

## Open

TWO SIBLING-DERIVES, byte-neutral, gate-directed interim (#128, 2026-07-10, from Agent B's census blind-check; section-9 three-lens cleared, every finding verified at source). Two reserved values whose OWN basis declared them equal to a sibling now DERIVE from that sibling rather than being authored as independent duplicate keys, the same pattern as the Stefan-Boltzmann sigma and the retired `hydrology.saturation_t_ref`: `productivity.soil_baseline` reads `productivity.soil_requirement` (basis: MUST equal it so bare soil is exactly non-limiting at baseline), and `behavior.selection_generations` reads `biosphere.predawn_generations` (basis: set equal to the pre-dawn radiation depth). Both keys retired from `reserved.toml` and `dev-fixtures.toml`; the reads move to the sibling (`crates/sim/src/environ.rs`, `crates/sim/src/evolve.rs`). BYTE-NEUTRAL on all four pins, MEASURED (default 4bbf6b59, discovery c9d5cc17, viability ad69f2bf, full 1db633b3): soil_baseline holds despite the dev-fixtures value changing 1 to 0.5, because the soil Liebig satisfaction saturates to one for any baseline at or above the requirement; selection_generations holds because run_world uses `EvolveParams::dev_default`, not the manifest path, and both keys were 40. No re-pin is owed. TWO owner-facing notes the section-9 surfaced, neither a defect: (1) a fidelity GAIN, the selection_generations derive now makes the controller-adaptation depth track a scenario that dials `biosphere.predawn_generations` high (Europa), which the static duplicate key would not have followed, so the "adaptation depth tracks the deep-time radiation" invariant its basis demands is now real; (2) a pre-existing coarse-model note, with soil_baseline equal to soil_requirement the soil factor saturates at baseline, so the matter-cycle fertility field is inert on productivity until the step-4 soil field lets the per-cell baseline drop below the requirement (the fertility test forces soil_baseline to zero to exercise the hook). Locked by tests (`baseline_soil_equals_the_soil_requirement_the_derived_invariant`, the generations assertion), the RETIRED-comment convention followed in `reserved.toml`, and the census updated. The queued general R-UNITS-PIN per-quantity representation (advisor deep-dive) is the deferred bigger target the sigma fine-scale consumption folds into.

R15. **STROKE-RATE STEP 2, the actuation-kind substrate: resolved framing, GATE-SIGNED-OFF 2026-07-10 (Agent A, PR #124).** The arc generalizes the delivered strike energy so a non-rigid striker (an elastic recoil, a hydrostatic jet, a hydraulic actuator) delivers kinetic energy as a data row rather than a code branch, retiring the alien limit the step-1b section-9 named (the Impact kernel hardcodes the rigid-lever axis ids). The gate proposed a per-segment discrete ACTUATION-KIND axis, grown and heritable, whose value selects which energy kernel runs (rigid-lever one registered value among whip, jet, hydrostat). The frame-blind CAUGHT that shape as an authored coupling: section-11 input-bias smoke FIRST (fail-closed, it caught steering in Agent A's OWN packet, four pre-loaded defences, neutralized and re-smoked to NEUTRAL before the panel saw it); then the section-10 blind panel, six panelists across three agent types and three models, five returning (one Fable safeguard error), UNANIMOUS (four significant-flaw-fixable, one reframe-needed). REJECTED: a grown discrete actuation-kind value that selects a kernel is the template case in its general form, a closed categorical fact read to pick which physical law governs a body, a closed kind-to-kernel lookup in the world-content path (P8, P11, the value-authoring line); heredity moves the value only among author-drawn buckets so evolution cannot reach an intermediate or a novel regime, and turning morphogen's continuous [0,1] grown fraction into a discrete kind needs an authored quantization threshold bolted on outside the floor; it fails admit-the-alien (a closed enum needs a code rewrite per regime), and the gate's own verification added the decisive point that it would REINTRODUCE the `MorphCategory` gate this project already retired for the derived continuous read. The decisive physics (verified against source): rigid (`F d`) and non-rigid (`P dV`, elastic recoil) are the SAME work integral of a generalized force over a generalized displacement, `P dV = (F/A)(A d) = F d` and elastic `= integral F dx`, differing only in a force-displacement CURVE the floor carries as continuous material axes. ADOPTED FORM (signed off, build shape (b) the run-all-gate-to-zero pattern, over a single monolithic integral): extend the delivery-path registry (`contact_transfer`'s `TransferKernel`, an OPEN set keyed on axis id) with new grounded delivered-energy laws, each GATED TO ZERO by the segment's continuous grown axes, with `F d` (`TransferKernel::Kinetic` reading `actuator_work`) the rigid limit; which law contributes DERIVES from the continuous grown axes only, mirroring how `capability.rs` `derive_capabilities` already runs every law blind to id and zeroes the inapplicable, so "rigid / hydraulic / elastic" is an emergent DESCRIPTION of where a segment lands in axis space, never a grown categorical selector. HARD INVARIANT: no grown categorical selector, no authored quantization threshold, the registry OPEN and keyed on axis id. NO NEW FLOOR AXES (all verified present: `mat.elastic_modulus`, `mat.yield_strength`, `mech.restitution`, `fluid.driving_pressure`, `fluid.immersed_volume`, `fluid.bulk_modulus`, `fluid.channel_radius`); the stored-elastic-energy density derives as `sigma_yield^2 / (2 E)`. ONE OR TWO NEW GROUNDED FLOOR LAWS, P9-licensed authored physics: the elastic-recoil delivered-energy relation (elastic strain energy per unit volume integrated over the strained volume, `1/2 k x^2` = `sigma^2 / (2 E)`), and a pressure-over-volume-change hydraulic work kernel (`integral P dV`) if the existing pressure laws plus `actuator_work` do not compose. RESERVED for the owner, surfaced not fabricated: any CONSTANT a new law introduces, basis the textbook mechanics relation (elastic strain energy; hydraulic `integral P dV`), never a declared number. IN SCOPE (a real parallel hole the panel found and the gate confirmed): fix `capability.rs`'s Impact kernel to read its axis ids from the registry row as `contact_transfer` does, so the grade path and the delivery path stay in lockstep and both admit the alien as a data row. Byte-neutral at the rigid limit (a segment growing only the rigid axes, elastic-storage and working-pressure at floor-low, reads exactly today's `F d`), verified against all four canonical pins (default `4bbf6b59`, full `1db633b3`, discovery `c9d5cc17`, viability `ad69f2bf`) before each push; section-9 five-lens before it lands. THE OWNER'S NORTH STAR, noted by the gate as the eventual graduation: transforms emerging so NO registry is needed at all; if the delivered-energy laws are ever proven to collapse into one integral, that is a later simplification, and the owner has accepted a data-defined open registry as the interim. BUILT (PR #124, byte-neutral, four pins hold): SLICE 1 (`7b8012b`) the run-all-gate-to-zero substrate; SLICE 2 (`c416eaa`) the `capability.rs` Impact axis-id-from-registry fix; SLICE 3a (`745266c`) the pure `laws::elastic_recoil_energy` (the modulus of resilience `yield^2 / (2 E)` over the strained volume); SLICE 3b (`616ef77`) `TransferKernel::ElasticRecoil` wired into `resolve_delivered_energy` as the run-all-gate-to-zero MAX over the shared-metabolic-source mechanical family, the strained volume the SWEPT `cross_section * stroke`; SLICE 3c (`50b288a`) the capability IMPACT grade reading the SAME MAX (grade and delivery in lockstep). THE GATE RULED the aggregation (2026-07-10): MAX over the family CONFIRMED (the members are alternative delivery paths for one metabolic source, a spring's stored recoil energy IS the loading work, so SUM double-counts); the INDEPENDENT-RESERVE case (a kernel drawing a separate reserve, additive not shared) is a FLAGGED future coupling, NOT folded into the MAX, compile-enforced by the exhaustive `Kinetic | ElasticRecoil` match; the swept-volume-as-element-volume PROXY limit is ON RECORD with a dedicated `mech.elastic_element_volume` floor axis as the reserved-with-basis refinement. SLICE 3 SIGNED OFF (gate re-confirmed the four pins + CI, 2026-07-10) with two follow-on rulings: the POUND/tool-IMPACT fix is a DISTINCT tool-delivery arc (a tool has no actuator, so its percussion gates on the WIELDER's delivered energy concentrated over the tool's contact area, `mech.mass` a live read there, the founded coupling), NOT step 2; the named-vs-positional binding unification is the PRIORITY follow-on right after step 2 (all six grade bindings to named fields in one pass, the caveat + drift-test holding until then). SLICE 4 BUILT (`26d7979`, byte-neutral, the last mechanical-family member): the hydraulic `integral P dV` COMPOSES from the existing floor laws with NO new law, so the gate's "build the law OR prove it composes" resolved to a composition proof: for an incompressible fluid at constant driving pressure `integral P dV = P (A d) = (P A) d = F d`, so `TransferKernel::Hydraulic` is `actuator_work(stress_force(fluid.driving_pressure, cross_section), stroke)`, the SAME two laws the rigid kernel uses keyed on the fluid driving pressure (megapascal-stored, the same MPa-to-N bridge), a hydrostat actuator a data row; `resolve_delivered_energy` is the MAX over {rigid, elastic, hydraulic}, the IMPACT grade the same three-way MAX. The COMPRESSIBLE gas-expansion case (varying P, reading `fluid.bulk_modulus` + an equation of state) does NOT compose and is the flagged future kernel; the piston-cross-section-as-fluid-channel-area reuse is a proxy, `fluid.channel_radius`-derived area the reserved refinement. Four pins hold; section-9 the diff. STEP 2 kernels COMPLETE (Kinetic, ElasticRecoil, Hydraulic); the gate merges #124 once slice 4's section-9 lands.

RESOLVED framing, units fixed-point composite compute (R-UNITS-PIN, #127), 2026-07-10 (blind section-11 input-bias smoke across three diverse strong models, five rework rounds to CLEAR plus a final Opus-max confirmation; then the section-10 blind panel, 6/6 panelists across three agent types and three models, unanimous on the same seam and the same fix; verified against source, Prime Directive 1). The framing under test: compute a composite constant (the retired Stefan-Boltzmann sigma) by evaluating its declared formula over the fundamentals in the units crate's per-quantity scaled fixed-point representation, in place of the authored decimal. THE SEAM the panel caught, unanimous: the framing placed two categorically different parameter classes "on the same footing" as adjustable inputs, and treating them as co-equal is authoring wearing derivation's clothes. Class A is the formula's LAW CONTENT (its rational coefficients, exponents, and which inputs it names), which IS legitimately authored physics-floor data under P9. Class B is the REPRESENTATION pipeline (the per-quantity scale, its envelope/significance/guard, the per-operation rounding rule, and the accumulation ORDER), which is arithmetic bookkeeping that must only add a bounded, checked rounding error and must NEVER move the mathematical value. With Class B left free and (for order and rounding) unspecified, the composite is NON-IDENTIFIABLE (enough input-independent knobs to reach any pre-decided target by choosing an order that makes a chosen intermediate underflow, or a scale/rounding that truncates to the wanted step), which is the value-authoring line / P11 breach, and NON-DETERMINISTIC (two conforming implementations differ), which is a P3 breach; removing the stored decimal also deletes the only existing anchor with no replacement and no mismatch behaviour. THE CORRECTED FRAMING (unanimous convergence, my synthesis verified at source): SPLIT the classes and do not place them on the same footing. Class A stays the only authored, value-bearing surface (P9 floor law-content). Class B is PINNED as fixed mechanism, not caller data: fix the accumulation order and the per-operation rounding to ONE canonical rule (round-half-to-even, which is already the crate's `idiv_round_half_even` rescale primitive, lib.rs:341, so determinism is restored, P3); compute the per-quantity scale MECHANICALLY from the quantity's own declared magnitude bounds (P11 derive-from-the-situation), with the significance target and guard ONE global reserved value applied uniformly, never a per-composite knob (the crate's `derive_scale_bits` already takes the envelope from the quantity's own log2 bounds, so this is a tightening not a rewrite); and add a MANDATORY independent higher-precision (arbitrary-precision) evaluation of the IDENTICAL formula over the IDENTICAL inputs as the ground-truth cross-check, to a declared tolerance, failing loudly on mismatch per the calibration-manifest sentinel convention. That cross-check is ALIEN-FEASIBLE (the panel confirmed the alien test passes and this is not the weak point): the formula is its own reference, so it needs no external measured value even for a world whose inputs are non-physical constants. THE CLEANEST FORM two panelists converged on, adopted into the resolved framing: evaluate the formula in exact/arbitrary-precision arithmetic and round ONCE to the composite's canonical scale, which DISSOLVES accumulation order and per-operation rounding as choices entirely (exact evaluation with a single terminal rounding is order-independent by construction), so Class B contributes zero degrees of freedom beyond one bounded, checked terminal rounding. Since the composite derivation is a ONE-TIME computation at catalogue or manifest load, not a per-tick hot path, the arbitrary-precision evaluation is off the canonical simulation path and does not touch determinism there. HONEST NUANCE surfaced at source, not by the panel: sigma = 2*pi^5*k_B^4/(15*h^3*c^2) carries pi, so the composite is transcendental and the "exact rational" ground truth is really an ARBITRARY-PRECISION evaluation to a declared working precision, which makes that working precision and the comparison tolerance reserved values. RESERVED for the owner, surfaced not authored: the comparison tolerance and the working precision of the cross-check (basis: the fixed-point epsilon at the composite's canonical quantity scale, or equivalently the significance target the scale already carries), and the ONE global significance target and guard (already R-UNITS-PIN reserved). VERIFIED against source (Prime Directive 1): round-half-to-even is the existing rescale primitive; the scale envelope already derives from the quantity's own bound magnitudes; the fundamentals ship exact CODATA decimal strings (k_B `1.380649e-23`, h `6.62607015e-34`, c `299792458`, sigma `5.670374419e-8`), so the arbitrary-precision cross-check is feasible and deterministic. META-NOTE for the gate: the section-11 smoke pushed the framing toward describing both channels "on the same footing" so the blind panel would judge their status without being steered; the panel then ruled that the substantive DESIGN must split them. The two are complementary, not contradictory: neutral DESCRIPTION let the panel find the real seam, and the seam is that a co-equal-adjustable DESIGN launders authoring. Posted to the gate on #127 for its ruling before any compute mechanism is written; the retire-sentinel-and-interim-together and re-arm-Mirror-coverage scope items are unchanged. GATE SIGNED OFF the corrected framing in full (adopt the class split, the arbitrary-precision-eval-then-round-once form, and one determinism condition held hard: the arbitrary-precision eval must itself be deterministic, bignum integer/rational, NO hardware float, pi by a deterministic series). BUILT (#127, byte-neutral): a self-contained deterministic float-free bignum + exact-rational evaluator (`crates/units/src/bignum.rs`), the composite evaluator (`crates/units/src/compute.rs`: parse the formula string, evaluate EXACTLY as a rational, pi by an integer Machin series, round ONCE, cross-check against the stored reference failing loud), and the sim wiring (`crates/sim/src/physiology.rs` `derived_stefan_boltzmann`, memoized; `MetabolicAnchors` and the sky model DERIVE sigma; the reserved.toml sentinel and the dev-fixtures interim RETIRED together; `mirror_calibrated_boot` RE-ARMED, the calibrated boot now reaches its determinism and worker-invariance checks). Consumed sigma is 244 x 2^-32 (round-half-even nearest of true sigma; the old value was 243, a truncation of a three-significant-figure approximation). BYTE-NEUTRAL on all four run_world pins (default 4bbf6b59, discovery c9d5cc17, viability ad69f2bf, full 1db633b3), so no re-pin is owed. HONEST REASON (a section-9 confirmation-bias catch that corrected my own conclusion, Prime Directive 1): sigma IS consumed in all four scenarios, folding into the resting metabolic drain through the radiant term (the audit forced sigma x1000 and every pin moved); the neutrality is because the 243 -> 244 ONE-ULP shift lands in a downstream Fixed-rounding deadband, NOT because sigma is unconsumed, so a larger sigma change would move the pins. My earlier "sigma is not consumed in those scenarios" report to the gate was wrong and is retracted; the consumed value is now locked by a test (`derived_stefan_boltzmann_is_the_expected_q32_bits`). RESERVED, surfaced not authored: the two representation knobs `COMPOSITE_SIG_TARGET` (30) and `COMPOSITE_GUARD_BITS` (1) set the intermediate derived scale; because the consumed sigma is invariant to them at the Q32.32 consumption scale (at the shipped values and a neighbourhood), they are treated as fixed-point representation constants in code (the family of the canonical `FRAC_BITS`), NOT fail-loud manifest values, since a fail-loud reserved read would block the calibrated boot and they change no consumed value. The invariance is NOT universal (an extreme retune can drive the intermediate scale below the canonical bits and perturb the consumed value), so the doc names that limit and a retune must re-verify the pins. Basis: resolve a CODATA composite's ~10 significant figures. The working precision and the cross-check tolerance are DERIVED, not fabricated: the working precision follows from the derived scale (`working_digits_for_scale`), and the cross-check tolerance is the stored reference decimal's own unit-in-the-last-place (`BigRat::decimal_ulp`, scale-independent, so it neither false-fails a fine-scale derivation nor passes a diverged formula). SECTION-9 FIVE-LENS DONE (six lenses across diverse models, every finding verified at source before it was trusted and every real one hardened): the confirmation-bias catch above (my byte-neutral reason was wrong); the cross-check tolerance was scale-coupled and would false-fail at a high significance target (fixed to reference-precision-keyed); `round_to_scale` wrapped a magnitude in [2^127, 2^128) to a negative i128 (fixed to report out of range, unreachable by sigma but a latent contract defect); the knob-invariance was overclaimed (softened plus the lock test); and stale docs still called sigma "reserved/authored" and cited the retired manifest key (corrected in `physiology.rs`, `environ.rs`, and `reserved.toml`). HONEST LIMITS the owner should know: the compute's symbol universe (pi plus the closed six-member fundamentals table) is COMPILE-TIME P9 floor, so a world can declare a new FORMULA and new fundamental VALUES as data but cannot declare a brand-new fundamental or transcendental symbol without a code change, a principled floor boundary (the earlier "the fundamentals are data" phrasing slightly overstated it); and `BigRat::floor_log2` brackets a magnitude by a fixed `2^-256` shift, a fail-loud assumption that a physical constant never approaches that floor (safe for any real constant, roughly 1e-77). Sigma's fine-scale consumption (reworking the radiative arithmetic in `physiology.rs` so the arbitrary-precision precision is realized rather than truncated at Q32.32), the token/magnitude collapse, and the per-agent blind-checks are flagged out of this arc.

RESOLVED framing, catalog-compatible predation wound (#120, predation-integration fork 2), 2026-07-10 (blind section-11 smoke CLEAR on the third rework, then section-10 panel, 6/6 significant-flaw-fixable, unanimous; posted to the gate for its ruling). The gate ruled fork (2), a catalog-compatible mortality so the ambush predator's strike actually kills catalog-bodied prey, under a hard derive-first bar (one wound law reading the target's body granularity as data, the magnitude derived from the target's own tolerance not authored, death through an existing cull). The panel caught my V3 error unanimously: using the world's whole-body SURFACE area (a metabolic/convective quantity) as the strike CONTACT AREA is a category substitution, not a derivation; surface over-counts the local contact patch, inflates the Griffith tolerance, shrinks the wound toward zero, fails the three-way test, and breaks admit-the-alien (a high-surface, low-cross-section body becomes unwoundable). The convergent fix (my synthesis, verified at source, Prime Directive 1): (1) CONTACT AREA = the STRIKER's own delivery-part presented area (its weapon `mech.contact_area` / a segment's `presented_contact_area`, real per-part data on the authored fine-bodied striker), never the target's whole-body surface, so the target's shape never enters the area term (alien-safe); (2) FRACTURE-ENERGY = the prey's OUTERMOST / first-contacted tissue layer (a data read via `BodyPart::surface()` -> the tissue registry -> `fracture_energy`), the same most-presented-material locality the fine path uses, never an authored whole-body aggregation operator; (3) DEATH = the wound fraction -> a whole-body damage accumulator -> integrity = one minus damage -> an INTEGRITY axis -> the existing cull, for a body carrying tissue-material fracture data (dimensionless, no wound-to-reserve magnitude), and alien-clean (a non-fracture body returns no wound and routes lethality through its own reserves; the law degrades to data-absent rather than forcing a universal fracture death). This also dissolves the strike-no-op barrier: the shipped predator is authored WITH a minimal fine body (real delivery mass + contact area), and the catalog prey take the coarse branch. Flagged follow-ons: the existing fine path reads the TARGET segment's area for contact (a latent #117 seam, a contact patch is a striker-target property; surfaced, not changed here, a separate audit); a sequential layered-penetration cascade (the per-layer `depth()` data exists) is the fuller derivation, the outermost-layer read the proportionate first step; the coarse-striker case stays deferred; growing run-path Structures for creatures (route 1) supersedes the coarse branch with per-segment armor/vitals. Posted on #120 (`4936858752`); the gate rules the corrected framing before code. On the ruling, the build folds in three secondary section-9 findings (seed `body_temp` on `spawn_predator`, harden the teeth positional literal, gate the predator out of step-2 breeding with `PREDATOR_ID_BIT`).

RESOLVED framing, creature behaviour-selection arc (#120), 2026-07-10 (blind section-11 smoke CLEAR on the third rework, then section-10 panel, 5 of 6 seats valid, Fable errored on its safeguard, 5/5 significant-flaw-fixable, unanimous; posted to the gate for its ruling on build order). The framing asked whether the creature tier gains reproduction and selection by (i) evolving the shared pool through its existing survival-advantage frequency map with spawn-from-evolved-pool, or (ii) a new in-run breeding driver gated on each agent's own reserve. The panel dissolved the peer choice, and I verified every decisive claim against source (Prime Directive 1). Findings: (A) the signed direction weight is a SOCIAL, outward locus, and route (i) is structurally blind to it, because every offline scorer (`episode_survival_dir`, `full_episode_survival_dir`, `two_reserve_episode_survival_dir`, `crates/sim/src/evolve.rs`) runs a SINGLE walker alone (`vec![walker]`, no second agent), so the outward behaviour is never expressed or scored; route (i) stays correct for solo-expressed loci (forage, metabolism). (B) the coupling that would make the weight selectable in the live run is the predation path (#117 strike, wound, integrity, the one unified cull), a CAPACITY present but which `no run_world scenario reaches` yet (`runner.rs:6075-6083`), so the payoff channel is UNMET even in shipped `full --creatures`; until it is live NEITHER route selects the sign, it drifts. (C) a shared alien limit, not a differentiator: both routes read fitness and eligibility through fixed Terran reserve axes (WATER and ENERGY backed by `bio.water_fraction` and `bio.energy_density`, `evolve.rs:504-512, 582-602`), so a non-Terran floor class forms no selection under either; an already-documented honest limit, generalizing it (read the being's own backing-component) a named sibling arc. (D) if an in-run driver is adopted, close two authorings: own-reserve or own-phenotype eligibility (never genotype-reading mate choice), and a pairing primitive named as spatial or temporal co-presence (a proxy correlating with proximity, the kin-bias template proxy), never a rank, trait, or type match; and at an all-zero locus selection has nothing to act on until mean-zero mutation supplies variance. The resolved framing (my synthesis, for the gate to rule): the arc is a precondition-first order, (1) wire the being-directed behaviour's own-survival consequence in the shipped scenario (predation for creatures), (2) in-run reproduction and selection for the social loci with the two authorings closed, route (i)'s pool selection retained for the solo-expressed loci, (3) flag the shared Terran-reserve-axis limit as a named sibling. Posted on #120; the gate rules whether step 1 is in-scope here or a preceding slice (it is where the one intended `full --creatures` re-pin lands).

GATE RULED 2026-07-10 (on #120): the three-step precondition-first order STANDS. Step 1 (arm the predation survival-consequence for creatures in `full --creatures`) is IN-SCOPE as this arc's first slice, through the EXISTING strike and intake paths (a creature approaching a strong signal incurs a real strike, approaching a food signal feeds), no new authored consequence; it is where the one `full --creatures` re-pin lands and it is the integration substance of the capstone. Scope step 1 and post the scope for the gate's ruling BEFORE code, with a STOP+report escape if arming is disproportionately large (a full predator-integration piece rather than a scenario-arming plus the existing strike/intake firing on co-located prey), in which case step 1 becomes a preceding slice. Step 2 (in-run creature reproduction/selection, route ii's shape, for the SOCIAL loci) ruled in, route (i)'s pool selection kept for the solo-expressed loci; close route (ii)'s two authorings (own-reserve or own-phenotype eligibility never genotype-reading mate choice; a pairing primitive named as spatial or temporal co-presence, never a rank, trait, or type match), and note the all-zero-locus no-variance-until-mutation point as real and self-healing. Step 3 (flag the shared Terran-reserve-axis fitness limit as a named sibling arc) correct. Merge current `main` into #120 first to carry #118's capacity, then scope step 1; no code until the gate rules the step-1 scope.

R14. **R-SOURCE-VECTOR resolved framing, GATE-SIGNED-OFF 2026-07-10 (Agent C, PR #121). The formal R-SOURCE-VECTOR consolidation is the OWNER's resolution step.** The arc lifts the eater half of the biology floor off its fixed mass-fraction-of-fresh-tissue supply unit onto a general source-vector draw, so a field-and-gradient feeder (photovore, thermovore, lithotroph, mana-feeder) is first-class rather than a `Resource` escape hatch (canonical scope: the Part 16 `GrowthInput` and Part 17 `FoodSource` enums; Part 19 `MaterialProps` is mechanical, already subsumed by R-PHYS-MECH, out of scope). The frame-blind (section-11 input-bias smoke, three rounds to CLEAR; section-10 blind panel, five of five usable panelists convergent, verified against source) caught the seam in the design doc's OWN flag wording: "lift the supply-axis unit off mass-fraction-of-fresh-tissue PER SOURCE KIND" and "the SOURCE KINDS and axes are data" (design.md:1422) smuggle a closed source-kind taxonomy (tissue/field/redox), the template case merely data-fied into a per-kind unit table. RESOLVED FORM (signed off, strictly more P11-clean than the flag): the UNIT, the DEPLETION/CONSERVATION character (depletable stock / non-rivalrous flux / reservoir), and the REDUCTION to the floor's common conserved energy/matter currency live on the AXIS as floor metadata (the registry already carries dimension/unit/scale per axis; conservation-as-data already exists as `AbioticBinding::depletes`, FINDING-1; the conserved projection is R-TIER-CONSIST); an eater's draw and a source's supply are each a SPARSE VECTOR over the ONE shared Part 58 axis registry; the single fixed-Rust mechanism is a DIMENSION-CHECKED CONTRACTION pairing like axes with NO branch on any source kind, so tissue/field/redox/photovore/thermovore/mana name nothing in code (they are emergent labels for which axes a draw is nonzero on), matching the R-PHYS-MECH house style exactly ("a single quantity axis at one canonical scale per physical quantity", material-blind, "a mace and a spiked morningstar are one conceived design the laws resolve differently"). Relational feeders (the thermovore's gradient) are admitted by a source publishing a DERIVED axis (gradient-magnitude) by demand-closure, drawn on with zero new mechanism (its coupling to the located-scalar-field substrate is the named honest limit). Two ADOPTED HARD-GATE acceptance tests: mixotrophy costs zero mechanism code (nonzero coefficients in two axis groups on one draw vector), and an unlisted modality (a gravity-gradient feeder) is addable as pure new registry rows with zero edits to the contraction; a build that forces a new branch for either has a hidden taxonomy and fails. RESERVED for the owner, surfaced not authored: the per-axis reduction coefficients to the conserved energy/matter currency (basis: the floor's own energy/matter equivalence for each axis's physical quantity) and the per-axis depletion-character defaults (basis: the conservation law of the quantity); no source-kind enum, no per-kind unit table anywhere. THE FORMAL R-SOURCE-VECTOR RESOLUTION IS FLAGGED FOR THE OWNER: the design-doc consolidation, the Part 62 record, the flag-phrasing correction (correct design.md:1422's "per source kind" so the doc no longer carries the taxonomy it flagged), the Part 63 bibliography, and the audit-log Section 1/3 resolved/open counts are the owner's resolution step. Agent C builds the byte-neutral substrate on the signed-off framing next (existing matter-eaters bit-for-bit identical, their draw nonzero only on the tissue-composition axes; the source-vector machinery opt-in until a world declares a non-matter feeder, so the four pins hold: default 4bbf6b59, full 1db633b3, discovery c9d5cc17, viability ad69f2bf; any re-pin routes through the gate), proves the two acceptance tests, and runs a section-9 five-lens audit before the push. GATE RULING at build time (2026-07-10, PR #121): a build-time source-check (Prime Directive 1) caught that the resolved-framing wording "dimension-checked CONTRACTION (reduction to the conserved currency)", read as a GLOBAL cross-axis scalar sum, is incompatible with the current draw, which is verified PER-RESERVE and INDEPENDENT (locomotion.rs:1234-1276: each reserve reads exactly its own `backing_component` class, fills through its own `room` clamp via `physical_intake`, depletes only that class; no cross-axis sum, no Liebig min in the draw), so a global contraction cannot reproduce it bit-for-bit. The gate (this is an input-audit on the gate's OWN sign-off, which had verified the house style and the axis-as-data-carrier but not the per-reserve independence that sets the contraction's SCOPE) RULED the corrected form (B): the mechanism is a PER-RESERVE FOLD over that reserve's draw-axis SET, a fold that is a no-op on a singleton, NOT a global cross-axis sum; a matter-eater's reserve is a singleton draw set so the fold is exactly today's single `physical_intake` call (byte-identical, matter-eating the special case, the true one-path lift); a mixotroph's reserve is a two-axis draw set (the fold sums both into that reserve, a data row); the conserved-currency REDUCTION keeps its role in cross-axis COMPARISON (the R-TIER-CONSIST pool projection), never the intake arithmetic. When the owner records the formal consolidation, the design-doc mechanism carries this per-reserve-fold wording, NOT "dimension-checked contraction". The gate also RULED (i) on the unit bridge: the matter path's PER-BEING `food_energy_density` bridge (locomotion.rs:139, R-UNITS-PIN's interim global scale, superseded per-cell by T3 at locomotion.rs:1301) stays UNCHANGED this arc; the per-axis unit/reduction/depletion metadata is carried and read so matter axes reproduce today's computation exactly and new feeder axes carry their own unit; the full unit-unification (lifting the matter unit onto the axis) is a FOLLOW-ON coupled to R-UNITS-PIN, flagged not absorbed. The gate REJECTED (A), a global cross-axis rewrite that would move the four pins and collapse the per-reserve independence (a real physical structure, each reserve a separate stock with its own room) for no gain. FURTHER GATE SHARPENING (2026-07-10, the owner's locked fundamental-constants floor, AGENTIC_ADDENDUM section 9's three-way test): the two items above are NOT both reserved-with-basis owner values. The per-axis REDUCTION-COEFFICIENT to the conserved currency is case (3) DERIVABLE, not authored: an axis's reduction IS the energy-equivalence of its physical quantity, so it derives from the fundamental constants times the substance's own floor physics (a redox axis through its couple's EMF times carrier charge, inheriting #112's `nFE` bridge; a thermal-gradient axis through the heat capacity and the temperature difference; a mass-flux axis through `bio.energy_density`). It is therefore a DERIVE target for the FEEDER-ARMING follow-on, carried this arc as a fail-loud `ReductionCoefficient::Derive` sentinel read by nothing until a world arms a non-matter feeder, never a declared number. The per-axis DEPLETION-CHARACTER is case (2) per-world floor DATA (the conservation law of the quantity), declared in the floor toml (`depletable_stock` on the matter composition axes), an undeclared axis carrying the fail-loud `DepletionCharacter::Reserved` sentinel. So the R-SOURCE-VECTOR substrate authors no value: the depletion-character is read as world data, the reduction-coefficient is derived at feeder-arming, and the matter unit bridge stays R-UNITS-PIN's per-being interim. Commit 1 built and byte-neutral (the four pins reproduce exactly: default 4bbf6b59, discovery c9d5cc17, viability ad69f2bf, full 1db633b3; physics and sim suites green): `QuantityAxis` carries `depletion_character` + `reduction_coefficient` (physics `lib.rs`, declared in `biology_floor.toml`), `HomeostaticAxisDef` carries an empty-by-default `draw_set: Vec<DrawTerm>` whose emptiness derives the matter singleton, and the `locomotion.rs` INGEST arm gains the per-reserve fold as an early guard so the matter path is character-for-character untouched. The feeder-arming reduction-coefficient DERIVE (inheriting #112's `nFE` redox bridge) is the flagged follow-on that makes an alien redox or thermovore feeder first-class.

R12. **Creatures-react being-percept: the frame-blind revised the mechanism. Resolved framing surfaced for the gate 2026-07-10 (Agent B).** The arc was queued as "give the creature a lighter being-percept path so it forms a predator/prey belief like the founder." The frame-blind (section-11 smoke, five rounds to CLEAR; section-10 panel, five of five returning convergent, verified against source) revised it: REJECT mechanism (i), transplanting the founder belief path onto the creature, on two grounds, that installing a within-life belief store into a definitionally non-learning tier is a rewrite not data (admit-the-alien), and that committing a labelled valence {harmful, benign, rewarding, neutral} and reading that committed category to produce the movement is the template-case authoring pattern plus a closed-enum (P8); ADOPT mechanism (ii) as the base, the raw perceived signal (channel plus the creature's own discrimination bucket, no valence, no belief) into a controller direction slot through a heritable freely-signed weight, the toward/away coupling set by cross-generational selection, the template-case cure native to the creature's genome-expressed controller; FOLD in within-life plasticity (iii) as a heritable genome-expressed plasticity coefficient on the same weight, defaulting to zero so it reduces to (ii), with the plasticity direction itself a heritable freely-signed parameter whose sign selection sets, so whether and which way a lineage learns is a selected emergent trait (P9). The decisive shared site, the magnitude bucket, is VERIFIED clean in the existing substrate (the bucket derives from the perceiver's own discrimination step, `perception_percept.rs:70,89`, not a global grid); the build constraint is that the creature carries its own transduction. Full detail and the source citations are in `docs/working/CREATURES_REACT_ARC_PLAN.md`. Gate to rule the framing before code (PR off `main`, the bridge rule for merging #115). RESOLVED 2026-07-10 (recorded by the log audit): the gate ruled the framing (adopt (ii) as base, fold in (iii), reject (i)), Agent B built the capacity byte-neutral and off by default (founder-zero), and it is MERGED as #118 (`373e0d8`, the creatures-react being-percept capacity, live-wired in `full --creatures`). The emergent creature reaction (making the flee and hunt sign a real SELECTABLE pressure, then selecting on it) is the follow-on predation-integration slice, in progress on #120.

R13. **Inherited founder-path question, flagged by the creatures-react panel (Agent B, 2026-07-10).** Two blind panelists observed that the existing, gate-signed founder being-percept path commits a labelled valence from a closed set and reads that committed category to produce the gradient, the same pattern mechanism (i) is rejected for. Whether this is a real defect turns on whether "a reserve fell, therefore harmful" is a floor-level primitive (harm as reserve loss is near-definitional, an authored floor value the value-authoring line permits) or a cultural outcome that must emerge. Surfaced for the gate to decide whether to open a separate audit of the founder path; NOT changed by the creatures-react arc.

R7. **The world is 3D (`Coord3`), not 2D; perception must key on it. OWNER DIRECTIVE + CORRECTION 2026-07-09.**
While framing the reach wire (Arc 3 slice 1), I stated the world was a 2D grid and proposed a 2D `1/r` falloff.
The owner corrected it: the world should be 2.5D minimum (space above) with subsurface (things in the ground).
Verified in source: the world coordinate is `Coord3 { x, y, z }` (`locomotion.rs:698`), z vertical, the material
field carrying subsurface strata at negative z (`material.rs`, hematite at z:-2); a being carries a `Coord3`.
The 2D I saw is only the perception/place projection: the perception path keys on an opaque flat `PlaceId`
(`world.rs:83`), and the environmental fields are a 2D surface grid. Consequence: `inverse_square_falloff`'s
`P/(4*pi*r^2)` is the correct 3D law over the 3D `Coord3` separation, so the reach wire keys distance on `Coord3`
directly and bypasses `PlaceId`. Owner ruling: build the perception-substrate framework out first with the reach
wire scoped on `Coord3` directly (the small change), and SCOPE the 3D perception-place lift (raise the whole
perception place model from `PlaceId` to `Coord3`) now and do it NEXT after the framework. Captured in
`PERCEPTION_SUBSTRATE_ARC_PLAN.md` (slice 1 and the sequence section).

RESOLVED slice-1 (reach wire) framing, 2026-07-09 (blind section-11 then section-10 panel, 6/6
significant-flaw-fixable, unanimous; gate accepted fork (a)). The reach wire is: for a signal on channel c
from a source `Coord3` to a perceiver `Coord3`, a received physical scalar computed as a general
dimensionality-parameterized geometric-spreading kernel (D derives from the traversed path/medium geometry,
3D bulk to `1/d^2`, a 2D surface to `1/d`, a duct to no spreading; D reserved fail-loud with its geometry
basis where it cannot yet derive, never fabricated) applied to the emitted power and the 3D separation, then
attenuated by the medium's own `opt.absorption_coefficient` sampled along the `Coord3` segment (so occlusion
emerges from the strata, no authored line-of-sight). Channel c is a data-registry row naming its kernel and
axis ids (dispatch by named id, never a code branch on channel identity). The received value is a pure
per-perceiver read (P10). Five build conditions from the gate: D derives or is reserved fail-loud; the general
kernel is byte-identical to `inverse_square_falloff` at D=3; absorption reads the medium axis, never a label;
the registry dispatches by id; the non-geometric propagation-law-as-data stays flagged as the deeper
substrate. The general geometric-spreading kernel at any integer dimensionality is legitimate floor authoring
(physics is an authored floor input, Principle 9), subsuming inverse-square. Section-11 caveat: an earlier
frequency clause over-committed (stated body-resonance as settled); the body-resonance-reserved ruling for the
acoustic frequency stands, judged alien-safe on the clean axis.

BUILT + AUDITED slice-1 (reach wire), 2026-07-09 (Agent A, PR #109 `claude/liveliness-arc`). The reach wire
is built in four byte-neutral off-path segments: the general `geometric_spread` kernel (`physics/laws.rs`,
byte-identical to `inverse_square_falloff` at D=3, proven by test), the channel reach registry
(`perception_reach.rs`, dispatch by the row's named kernel id, never channel identity), the reach read
(`received_reach`: 3D `Coord3` separation with the vertical z, structural dimensionality, medium-sampled
optical depth), and the run-path resolver (`resolve_reach`/`absorption_along`: reads the row, samples the
medium's own absorption along the `Coord3` line, dispatches by kernel id). All four run_world pins hold
bit-exact (default 2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82), corroborated by a caller
sweep (zero run-path callers). The mandatory section-9 five-lens audit ran (6 blind panelists across 3 types
and 3 models, adversarial per-finding verify): 13 findings, all verified against source and hardened. The one
MAJOR: `resolve_reach` silently ignored the row's `frequency_dependent` field, so the acoustic dev row would
resolve to a frequency-independent read of the OPTICAL absorption axis. Hardened to implement the framing's
already-approved "reserved fail-loud": `resolve_reach` now asserts a row is not `frequency_dependent` (the
emitter body-resonance frequency source and the acoustic-law application are a reserved follow-on, not wired
in slice 1), so a frequency-dependent row fails loudly rather than reading the medium axis as if it were
frequency-independent; no shipped row sets the flag; a `#[should_panic]` test proves the fail-loud. The nits
hardened: `MAX_REPRESENTABLE_SEP2` derived from the `i32` cast bound rather than a magic `1<<30` (and the
comment corrected: the guard is a representability clamp, and the D=3 kernel already overflows its own
denominator to zero far below it, so it clips no result); the optical-depth accumulator uses `saturating_add`
before the cap (an unchecked `Fixed +` could overflow a large `tau_max`); the medium-aggregation is flagged
as the volume-mean-only limit with the aggregation-kernel-as-data follow-on named; the fluid-medium limit
reworded medium-agnostic (a fluid-dweller's dominant occluder); the endpoint sampling convention documented
and pinned by test; two doc precisions (byte-neutrality is from the absent caller, not the D=3 identity; the
`bulk_axis` doc de-Terran-ised). Reserved for calibration, surfaced not fabricated: the acoustic absorption
axis the floor does not yet carry (the dev acoustic row reuses the optical axis as a labelled stand-in, a
flagged floor gap); the confinement substrate that would set D below 3 (the geometric kernel already handles
D=2/D=1); the frequency-dependent absorption path and its body-resonance frequency source.

FRAMED slice-2 (sensorium-gated magnitude percept), 2026-07-09 (Agent A, PENDING gate ruling). Blind
section-11 smoke test caught my first construction BIASED (a "carries no valence, category, label, or
meaning" clause pre-answered the panel, when the just-noticeable-difference quantization the clause called
category-free is the operation that mints the downstream belief-category grid, verified at
`percept.rs::feature_bucket`; and I had scoped out the transduction-derivation risk the arc plan names as the
slice's real risk). Rebuilt, section-11 re-run cleared it MINOR_ISSUES, the three named fixes applied. The
section-10 blind panel (6 seats, 5 valid, Fable errored on safeguards) returned 5/5 significant-flaw-fixable,
UNANIMOUS on the class: the framing authored the transduction and quantization SHAPE (linear scale, uniform
absolute quantization step, no detection threshold) into fixed mechanism code, when the value-authoring line
(Principle 6/11) and admit-the-alien require it to be per-being DATA, so a logarithmic, power-law, thresholded,
or Weber-scaled sense is a data row rather than a code rewrite. Verified against source: the existing perceive
beat scales salience by acuity linearly (`world.rs`), `feature_bucket` quantizes at an absolute step
(`percept.rs`), and the floor carries no parameterizable transduction primitive, so the concern is grounded.
The corrected framing (my synthesis, for the gate to rule): a being forms a percept on channel c when the
received magnitude m, mapped through a general monotone transduction PRIMITIVE parameterized by the being's
OWN response parameters (a gain, and where its body carries them a compression parameter and a detection
threshold), clears the being's own threshold; the percept is then quantized by the being's OWN discrimination
law (absolute or magnitude-relative) into the bucket that keys belief-minting. Linear gain, no threshold, and
a uniform absolute step are degenerate DEFAULTS of the family, never the fixed form. Perceptibility is one
being-derived quantity (the read/not-read gate and the sensitivity collapse into the threshold, absence
meaning zero sensitivity, the pre-sensorium default-open convention a distinct explicit case, folding
panelist plan-sonnet's two-gates catch). The transduction parameters and discrimination law DERIVE from the
being's genome and anatomy via the same expression machinery that produces `mind.acuity`; until that
derivation and the floor transduction primitive are built, they are authored per-being data flagged reserved.
Build dependencies surfaced: a new floor transduction primitive (a parameterized monotone response law); the
percept-class-to-channel binding; the anatomy-to-sense transduction generalized beyond optical. Posted to the
gate for its ruling before any code.

BUILT + AUDITED slice-2 (sensorium-gated magnitude percept), 2026-07-09 (Agent A, gate ACCEPTED the framing
under 5 conditions and signed off to build). Built in four byte-neutral off-path segments: the transduction
and discrimination floor family (`physics/laws.rs`: `ResponseLaw {Linear, Power, LogCompressive}` +
`transduce`, `DiscriminationLaw {AbsoluteStep, WeberRelative}` + `discriminate`, the Linear/AbsoluteStep
defaults byte-identical to `m*gain`/`feature_bucket`, condition 1); the percept module (`perception_percept.rs`:
`sense` = transduce then threshold then discriminate, `perceive` folding the read/not-read gate into the one
being-derived threshold, condition 3); the derivation (the optical gain derives from the eye's REFRACT
focusing capability, non-optical reserved fail-loud, no placeholder borrow, condition 2); the
percept-class-to-channel binding keyed on the stable class string (condition 4). All four run_world pins hold
bit-exact (default 2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82). The mandatory section-9
five-lens audit (6 blind panelists) returned 9 findings, all verified against source and hardened: two majors
(the binding keyed on the POSITIONAL `PerceptId` rather than the stable class string, re-keyed on the class;
the optical derivation equated light-sensing with a refracting lens, reworded to the honest lens-eye limit
with lensless light detection a data row and a light-absorption floor capability the flagged deeper build);
and minors/nits flagged (the monotone-only response family with saturating Naka-Rushton/Hill and non-monotone
tuned responses named as floor-extension follow-ons; the class->channel binding a world-data interim that
should derive from the substance's per-channel floor coupling; the single-valued/global binding
multiplicity limit; a test gap filled).

The gate ruled fork (a) for the KEYSTONE (not slice 2): the optical-vs-reserved distinction becomes an
explicit per-sense transduction-KIND marker on the anatomy as DATA (`Optical` / `ReservedKernel` / ...), the
harden-to-registry pattern, retiring the `opt.refractive_index = 1.05` placeholder borrow on non-optical
senses. Two keystone conditions the gate set: it edits `anatomy.rs` (a shared file, sequence the ADD with
Agent B's lifespan arc #113); and retiring the placeholder must be byte-neutral or a stated hash change
(verify no live consumer reads the non-optical senses' placeholder index).

Reserved for calibration, surfaced with basis, never fabricated (`ReservedSenseParams`, the per-sense params
the anatomy does not yet derive; the anatomy derives only the optical gain today): the `response` law a
sense's transduction follows (basis: the modality's established psychophysics); the `shape` exponent or
compression (basis: the modality's measured response curve); the `discrimination` law (basis: Weber's law
holds across most senses, so magnitude-relative is the usual case); the `step` just-noticeable difference
(basis: the sensorium's per-channel resolution, once it derives from anatomy); the `threshold` detection
floor (basis: the organ's noise floor). The deeper derive targets flagged to the owner: the per-channel
anatomy-transduction kernel (would derive these five from the body); a light-absorption floor capability
(would derive lensless and absorptive optical sensing); the substance-per-channel coupling (would derive
the class->channel binding); the saturating and non-monotone response laws (would extend the family). One
item surfaced to the gate for its ruling: whether to add the saturating Naka-Rushton/Hill response law to
the floor family now (the dominant real transducer nonlinearity) or defer it as a flagged follow-on; the
gate registered the floor-growth pattern (this arc adds the spreading law and the transduction family) in
the owner-blocker register for the owner's awareness.

FRAMED slice-3 (receiver-side valence learner), 2026-07-10 (Agent A, PENDING gate ruling). Blind section-11
smoke test caught my first construction BIASED (a false-settled: I stated the noise floor "derives from a
per-axis baseline" when it is a flat authored scalar today; an overstated "harm or benefit" when the harm
learner is harms/benign and reward is a separate pole; a suppressed deception case: a being-signal is
agent-emitted and manipulable unlike a substance). Rebuilt, section-11 re-run cleared it MINOR_ISSUES, and I
folded its one new catch (the same-tick vs predictive-lag structure: the harm path carries no eligibility
trace while the reward pole does, verified at `learn.rs`). The section-10 blind panel (6 seats) returned 6/6
significant-flaw-fixable, UNANIMOUS: the template-case CORE is sound (it correlates a low-level (channel,
bucket) key with a low-level reserve-fall proxy, never branching on "this is a being"), but two authored
seams must move to derived. Verified against source: the two likelihoods `p_harm_given_harms`/`p_harm_given_benign`
are reserved-from-manifest fixed scalars (defaults 9/10, 1/10, `learn.rs:288-289,304-305`); the floor
dose-response harm law `harm_class` (integer-Hill, `laws.rs:132`), the per-axis drain baseline `DerivedDrain`
(`homeostasis.rs:251`), and the harm bit `is_harm_tick` (`homeostasis.rs:1089`) all exist; the reward pole
carries `eligibility_decay` (`learn.rs:356-362`) the harm pole lacks. The corrected framing (my synthesis, for
the gate to rule): keep the template-case correlation core, and (1) the evidence weight DERIVES per-being and
per-(channel, bucket) as the weight-of-evidence of P(harm-bit given the feature harms) and P(harm-bit given
benign), each ESTIMATED from the being's own `harm_class` dose-response crossed with its own reserve-delta
noise distribution, so an alien with a different dose-response gets a different weight, RESERVED with basis
until the estimator is built, never a fixed global 0.9/0.1; (2) the noise floor DERIVES per-axis from
`DerivedDrain`, and the interoceptive outcome is per-axis (a signal harmful on one reserve and beneficial on
another can be learned); (3) the harm path carries the eligibility-decay trace the reward pole already has, so
a lagged co-occurrence is credited and same-tick stops being an authored ceiling; (4) referential meaning (a
signal predicting harm elsewhere or to another) is a flagged explicit open limit. The scope fork for the gate:
how much to build in slice 3 (the likelihood estimator is the deepest design work; the per-axis noise floor
and the eligibility trace are smaller wires) versus reserve with basis. Convergence flag (as the gate
directed): the learner keys on `feature_subject(channel, bucket)`, the belief-subject key shared with Agent B's
affordance composer and the owner-held `SEQ_FIELD_BITS` packing; slice 3 consumes the existing per-feature key
without re-encoding, and whether a `SenseChannelId` fits the channel field is a packing question flagged, not
decided. Posted to the gate for its ruling before any code.

BUILT + AUDITED slice-3 (receiver-side valence learner core), 2026-07-10 (Agent A, gate ACCEPTED the framing
and ruled the scope: build the byte-neutral valence CORE, reserve the four derive targets). Built as one
byte-neutral off-path function, `learn::being_signal_observation(channel, bucket, harm, plasticity, calib)`:
a being correlates a perceived being-signal (keyed by `feature_subject` on its sense channel and discriminated
bucket) with its own interoceptive harm bit, minting one weight-of-evidence observation toward HARMS or BENIGN
through the SHARED `observation_toward` minting the environmental-feature learner also uses, never branched on
as "a being", so a signal's valence emerges receiver-side. All four run_world pins hold bit-exact (default
2b7e1035, full 1873c44e, discovery 4eea5d06, viability bae5a82). The mandatory section-9 five-lens audit (6
blind panelists) confirmed the core clean across all five lenses and returned 8 findings, all minor/nit, all
verified against source and hardened: the subject-namespace collision (the core had no channel-base offset, so
a being-signal would alias an environmental biology feature, flagged as a keystone-wiring seam the keystone
resolves with the owner's packing ruling); the "identical learner" copy (extracted a shared `observation_toward`
helper, byte-neutral, so identity is structural); the reserved likelihood basis mis-aim (corrected: a
being-signal is a harm PREDICTOR not a CAUSE, so its likelihood is the receiver's own empirical co-occurrence
reliability, not `harm_class` which is the basis for an environmental harm-cause); and the alien granularity
limit (the reserved per-axis-outcome item).

Reserved for calibration, surfaced with basis, deferred by the gate to their own builds (all four are keystone
or shared builds, so this core moves no pin): the per-being LIKELIHOOD estimator (the empirical co-occurrence
reliability for a being-signal, `harm_class` crossed with the reserve-noise for an environmental harm-cause; a
build SHARED with Agent B's affordance composer, framed and sequenced separately); the per-axis NOISE FLOOR
(from `DerivedDrain`) and the per-axis outcome (a live-learner behaviour change that moves the pins, reserved
for the keystone); the harm-path ELIGIBILITY TRACE (the reward pole's `eligibility_decay`, which credits a
lagged co-occurrence, load-bearing for the predation payoff, a live behaviour change reserved for the
keystone); REFERENTIAL meaning (a flagged open limit); and the SUBJECT-NAMESPACE offset plus whether a
`SenseChannelId` fits the 16-bit channel field (the owner-held `SEQ_FIELD_BITS`/belief-subject-hash packing
decision, resolved for both Agent A and B together once the owner rules).

With slice 3 built and audited, the perception-substrate arc (slices 1 through 3) is complete and presented
for the gate's whole-arc gate. The being-percept keystone that wires the percept live (and does the reserved
estimator, the per-axis noise floor, the eligibility trace, and the packing offset, sequenced with Agent B) is
the payoff follow-on that unblocks predation on the run path.

FRAMED hunt-kill strike (the contact physics that completes predation: an approaching predator wounds prey on
the run path), 2026-07-10 (Agent A, PR #117, PENDING gate ruling). Framed blind before any code. The section-11
input-bias smoke test (fail-closed) BLOCKED my packet four times, each a real leak where I pre-supplied the
derive-clean verdict (outright emergence/authoring assertions; then founder-zero and physics-floor honorifics
with a one-axis-only defense; then law and heritable category labels; then no-run-path-code-does-X wiring
disclosures implying the machinery was disconnected), and cleared the fifth. The section-10 blind panel (6
panelists across 3 agent types and 3 models, 4 returned; plan-opus a transient server error, claude-fable
safeguard-flagged the combat content) came back 4/4 significant-flaw-fixable, converging on FOUR distinct
authoring seams my clean framing hid, each verified against source. (1) The death rule (my cumulative-damage
accumulator plus a structural-failure threshold) is a hidden hit-point system: an authored body-level value
outside the floor that flattens the Structure-of-Segments body into one scalar and adds a second death path.
CORRECTED: the wound degrades the struck region's own material, impairs the physiology that region sustains,
the reserve falls, and the ONE existing reserve-cull removes the being (matching the gate's one-currency,
death-through-the-reserve-cull model). (2) `kinetic_energy` hardcodes the wound channel (verified: `body::strike`
computes energy only through it), so a non-kinetic contact attack needs a rewrite. CORRECTED: an axis-dispatched
contact-energy-transfer law selected by the acting part's own energy/material axis, kinetic the first instance,
so a new channel is a data row. (3) The damage mode reads as a categorical dispatch (verified: `DamageModeRegistry`
CUT/PIERCE/BLUNT/BURN passed to `apply_insult`). CORRECTED: derive the contact area and mode from the acting
part's own geometry, so the wound-shape emerges continuously. (4) "Targeting a body" classifies the occupant as
a `body::Body` (parts and tissues), a second body model beside the run-path `Structure` of Segments (verified:
the Walker carries `body: BodyPlan` plus `structure: Option<Structure>`, not `body::Body`). CORRECTED: one strike
primitive over whatever Segments occupy the cell (terrain, matter, or a being uniform), "this was combat" a
description not a branch, with the deeper `body::Body`-to-Structure unification flagged as coupled to the
plant-as-a-body / composition arcs, not this arc. Unchanged and sound: the strike decision is the emergent
controller (the keystone being-percept gradient plus a founder-zero freely-signed strike-affordance weight), so
nothing reads a species, role, or relatedness. Couples to Agent B's run-path damage state (sequence with B).
Posted to the gate on #117 (comment 4932785384) for its ruling before any strike code.

GATE RULED 2026-07-10 (on #117): framing SIGNED OFF, build it. The gate verified all four seams at source
itself (seam 2 at body.rs:1077/912 and laws.rs:337; seam 3 at body.rs:205-213; seam 4 at locomotion.rs:512/521;
seam 1 sharpened against runner.rs:5545-5556). Corrections 1-3 adopted with two precisions. SEAM 1 death path
made exact (the cross-arc coupling): no new reserve mechanic; the wound writes Agent B's `Segment.damage`,
`whole_body_viability_aged` reads it, the INTEGRITY axis reflects it (runner.rs:5551-5556 sets INTEGRITY to
`whole_body_viability` over the struck Structure), and the ONE unified cull (`is_alive` over every axis then
`reconcile_lifecycle`, no morphology predicate, P8) removes the being when any axis floors; the strike is the
fast-increment sibling of B's slow aging accrual on the SAME `Segment.damage -> whole_body_viability_aged ->
INTEGRITY -> unified cull` chain (a vital-Segment wound floors INTEGRITY fast, a non-vital one degrades slower).
SEAM 3 pushed to a DESCRIPTION not a dispatch: the physical quantity is pressure = delivered energy / contact
area against the struck Segment's own material strength, and CUT/PIERCE/BLUNT/BURN are labels for regions of
that continuum, never a per-part selector (derive the mode from geometry if the floor law needs one, never pass
it in). SEAM 4 scope RULED to the HONEST FIRST CUT: one strike primitive over the run-path Structure's Segments
(reads whatever Segments occupy the cell, computes the wound from the floor laws against the Segment's own
material, writes `Segment.damage`, identical for terrain/matter/being), NOT routed through `body::Body`/`apply_insult`
(which stay for their uses); the deep `body::Body`-to-Structure unification is its own arc sequenced with the
plant-as-a-body / composition arcs, and the bridge is flagged as the named limit, do NOT fold it in here. UNITS:
write `Segment.damage` in B's exact accrual convention (delivered energy commensurate with `failure_tolerance =
fracture_energy * contact_area`, the units B's slice-3 section-9 hardened by removing the erroneous `* 1000`), so
a discrete strike is a large one-tick increment to the same fraction accumulator, one currency, no double-count.
SEQUENCING: build everything that does NOT touch the accumulator NOW (the contact-energy-transfer law, the
geometry-derived contact area, the strike affordance off founder-zero, the controller consumption) against the
Structure/Segment surface already on main; hold ONLY the final `Segment.damage` write until the gate lands B's
#113 (which puts `Segment.damage` + `whole_body_viability_aged` on main) and clears the damage-write onto the
merged accumulator, so Agent A does not branch against an unmerged surface. Acknowledged on #117 (comment
4932824584) with the build order. Building on `claude/hunt-kill-strike` (#117), each step frame-blind-clean and
posted for the gate's review, section-9 lens audit over the arc before the accumulator write.

FRAMED being-percept keystone (the payoff arc: wire the percept live so predation and fleeing emerge),
2026-07-10 (Agent A, PENDING gate ruling). Framed blind before any code. The section-11 input-bias smoke test
(strongest model, fail-closed) BLOCKED my construction four times, each a real source-verified seam on a
distinct axis (a whole-loop emergence claim scoped over receiver-only facts; the emitter curated alien-clean
when a non-Terran channel is a missing-physics substrate; a symmetric receiver alien over-claim introduced
while fixing the second; predation and fleeing framed as symmetric when only the harm pole had a substrate),
and cleared the fifth. The section-10 blind panel (6 diverse panelists across 3 agent types and 3 models, 5
returned, the 6th hit an infrastructure safeguard) came back 5/5 significant-flaw-fixable on two seams.
SEAM 1 (unanimous strongest): the being-directed gradient's away-from-harm / toward-reward SIGN is an authored
valence-to-direction coupling that forecloses approach-to-a-harm (a parasite, a scavenger, mobbing). REFUTED
at source (Prime Directive 1: a unanimous panel is a lead generator, not a verdict): the controller weight is
expressed from the genome UNCLAMPED (`GeneSet::express`, genome.rs:404-426, a sum of signed `genotypic.mul(weight)`
terms with no clamp; contrast `express_unit`, genome.rs:435-437, which clamps to [0,1] and is used only for
propensity channels, never for controller weights, which use `express` at controller.rs:942), and a working
taxis test uses a -1 weight (controller.rs:1488). So the weight is FREELY SIGNED and founder-zero: selection
lifts it positive (follow the percept) or negative (invert it), so a negative being-avoidance weight yields
approach-to-a-harm-believed emitter and the full approach/avoid space is spanned, with harm and reward as
separate percepts under independent weights. The panel reasoned correctly from my framing; my fact-3 phrasing
("only a heritable weight lifted off founder-zero turns it into avoidance or approach", "the fixed sign is an
open seam") MIS-STATED the mechanism, so the fix is a framing correction (state the weight is freely signed,
the approach/avoid sign emerges, retract the open seam), never a mechanism change. The panel's proposed
single-signed-percept would AUTHOR the reward-minus-harm combination, the less-emergent choice, so it is
declined. SEAM 2 (valid, verified): the build list omitted the subject-namespace offset, a LIVE wiring
requirement; `being_signal_observation` (learn.rs:583) keys `feature_subject(channel, bucket)` with no
channel-base offset while the material `reward_observations` (learn.rs:610) takes a `channel_base`, so without
the offset a being-signal aliases the environmental biology feature at the same index under HARM_ATTR (P11);
the fix sequences the offset FIRST. Folded-in completeness: the named limits carry both alien gaps (receiver
fail-loud on a non-optical channel, emitter alien emission a flagged floor extension) and the bootstrap
precondition (an out-of-loop first contact and a survivable sublethal harm to have an outcome to learn from),
and the being-directed gradient keys on perceived EMITTERS on a channel (any source, being or material),
never on being-hood. THE RESOLVED FRAMING (survives my own check): a being passively emits on a channel from
its own material (a Terran channel reads an existing floor source-power axis, a non-Terran channel's axis and
law are a flagged missing-physics extension); the emission reaches another attenuated by geometry and the
medium, and is threshold-gated into a percept (the receiver data-defined and fail-loud on a non-optical
channel); the receiver-side learner correlates it (same-tick today, a keystone-built harm-path eligibility
trace, its latency a reserved calibration, credits a lagged outcome) into a harm belief (the built core) or a
reward belief (a keystone-built reward-frame counterpart) on a subject offset into its own namespace band; the
evolved controller reads a being-directed geometric-direction gradient over those beliefs through a
founder-zero FREELY-SIGNED weight, so approach (predation) and avoidance (fleeing) emerge from selection, with
no mechanism reading a species, kingdom, trophic role, relatedness, named state, or being-hood. The keystone
builds, in order: the subject-namespace offset (P11, first); the being-signal reward-frame counterpart core;
the two being-directed gradients (consuming the reach and percept substrate); the harm-path eligibility trace;
the per-being likelihood estimator (shared with Agent B's composer); the live wire (behaviour-changing, a
stated hash change re-baselining the four pins). Flagged follow-ons: a discretionary emit affordance and
referential meaning (an alarm call is both, so alarm is dropped from the keystone, narrowing the gate's
predation/hunting/fleeing/alarm charge under Prime Directive 5); alien-channel emission and reception. THE
SCOPE FORK for the gate: (A) build both poles now (reward core plus both being-directed gradients plus the
eligibility trace), so predation and fleeing both emerge, my recommendation, since both poles need the
being-directed gradient anyway and the reward core is a small mirror of the built material reward core; or
(B) scope the keystone to fleeing (the built harm core plus a being-avoidance gradient plus the eligibility
trace) and flag predation's reward core and attraction gradient as the next sub-slice. Reserved and
owner-held, surfaced not fabricated: the belief-subject packing (the subject-namespace offset and whether a
`SenseChannelId` fits the 16-bit channel field, the `SEQ_FIELD_BITS` / belief-subject-hash decision the gate
surfaced to the owner); the per-being likelihood estimator (a build shared with Agent B, sequenced by the
gate); the eligibility-trace latency and the per-axis noise floor (reserved calibrations with basis). Posted
to the gate for its ruling before any keystone code.

GATE RULED 2026-07-10 (on #116): the framing is signed off (the gate verified the Seam-1 refutation against
source itself and confirmed it holds). The scope fork is ruled (A) BOTH POLES: build the six-step sequence,
each step gated. The alarm DROP is accepted under Prime Directive 5, so the honest keystone charge is
PREDATION, HUNTING, and FLEEING, with alarm (a discretionary emit plus referential meaning, both flagged
follow-ons) named as the next substrate. The belief-subject packing is ruled to the HYBRID (an exact widened
pack in-envelope plus a hash on overflow), which Agent B builds as the shared belief-subject key, so step 1's
being-signal band is COORDINATED with Agent B as three disjoint top-level bands: environmental features at
bit 62 (existing), sequences and conjunctions at bits 62 and 61 (Agent B's existing), being-signals proposed
at `(1<<62) | (1<<60)` (a new `being_signal_subject`, disjoint from both, so the slice-3 aliasing seam is
closed by construction), with the `SenseChannelId`-fits question falling out of Agent B's hybrid encoding
within the being-signal band. Sequencing: build steps 1-4 now (independent of Agent B); sequence step 5 (the
per-being likelihood estimator, shared with Agent B's composer) with the gate when Agent B frees; step 6 (the
live wire) last. Route every re-pin (step 6, and step 4 if its eligibility trace touches the live
environmental harm path rather than staying scoped to the dead being-signal path) through the gate; each
re-pins once and the gate sequences them against Agent B's and Agent C's re-pins on the four tracked pins.
The eligibility-trace latency and the per-axis noise floor stay reserved-with-basis (owner-set). Building on
`claude/being-percept-keystone` (#116), each step pushed for the gate's per-step review.

R1. **Founder band placement is an AUTHORED gameplay input, NOT an engine-solved cultural outcome. RESOLVED
   by the owner 2026-07-08.** The CONTINUED-4 living-world finding reported a seed-dependent collapse (a band
   spawning on a dry corner far from water starves) and surfaced "habitability-aware placement" as a candidate
   engine fix. The owner ruled: band placement is where the gameplay comes in, so it is legitimately authored
   (a player/scenario choice), a Principle-9 INPUT like the physics floor, not a cultural OUTCOME. If a band
   is placed somewhere barely survivable and dies, that is the player's choice faithfully simulated, not a bug.
   If a band claws through the hardship and, say, comes to resent its god for the suffering it endured, that
   emergent narrative is the fun. So the engine must NOT auto-place founders for survivability, and the
   seed-dependent death is correct behaviour, not a defect. Consequence for the finding: the ONLY genuine
   remaining engine gap is forage COMPETENCE (the recurrent controller freezing one tile short of REACHABLE
   known food, so a being dies where it should survive), because that corrupts the hardship signal: death must
   come from real scarcity, not from a controller that cannot walk to visible food. The emergent payoff the
   owner named (sustained material hardship shaping a band's beliefs, e.g. resentment of its god) is the
   demonstration target that rides on the axiom/belief kernel (Parts 21/28) plus the harm-learning felt-
   experience path. Keep the recurrent controller as the emergent foundation; the run-matched dawn bootstrap
   (train forage competence under partial knowledge on real terrain, alien-general food matter) is the work
   that makes survival reflect genuine hardship rather than a forage bug.

R2. **How first-hand felt experience feeds a being's convictions: the LEARNED-COUPLING framing. RESOLVED by
   the owner 2026-07-08 via a blind framing panel.** Before wiring "hardship shapes belief" (the resent-god
   loop), the proposed framing was: felt hardship enters the axiom-update kernel as generic SIGNED EVIDENCE on
   a conviction axis, a magnitude plus a DIRECTION toward a pole, the resulting shift emerging from the being's
   disposition. A fully-blind panel (six agents, three types, three models, each seeing only the guiding
   principles and the de-narrativized statement, none seeing the author's or owner's conclusion,
   AGENTIC_ADDENDUM.md section 10) UNANIMOUSLY and independently caught the seam: the DIRECTION clause authors
   the exact coupling it claims to forbid. Deciding that hardship bears on providence and points to its
   negative pole reads the high-level MEANING of the experience to produce the outcome, the kin-template
   violation (Hamilton's rule as a mechanism) relocated from kinship to belief. Verified against source: the
   magnitude side is floor-derivable and clean; the axis-and-pole selection is the authored crux; the alien
   test confirms it (magnitude generalizes to any being with a reserve, an authored direction needs
   per-species authoring). THE RESOLVED FRAMING to build: first-hand felt experience emits ONLY a magnitude
   (interoceptive salience of the reserve-delta) and a valence sign (the floor sign of the delta), with NO
   axis and NO pole; which conviction it bears on, and in which direction, is a per-being LEARNED COUPLING
   (the same associative/credit-assignment primitive that already lets a being learn "this ground harms me"
   from felt reserve-fall, extended so a conviction can be credited), with the pole following from whether the
   felt outcome confirmed or disconfirmed the stance the being was acting on; where no association exists the
   felt event changes no conviction (the honest default). So "hardship erodes faith" is a DESCRIPTION of a
   learned outcome for some beings and its opposite for others, never a coded route. CONCRETE TRAP recorded:
   do NOT reuse `affect.rs`'s `AppraisalBinding` (a per-race `DriveId -> AffectAxisId` table) for axioms; it is
   legitimate for affect (a felt-coloring layer close to an innate disposition) but would be the violation for
   axioms (which hold emergent cultural content). This is the framing for the next build; the design-doc Part
   28 consolidation happens when it is built. Reserved: the interoceptive-signal-to-salience scale (basis: the
   existing salience range of social events), owner's to set.

R5. **Branch-2 (credit-assignment) framing CORRECTED by a third blind framing panel (2026-07-08); one OPEN
   fork surfaced for the owner.** The move half of the R2 substrate. The first-cut framing decided the pole a
   conviction moves toward by comparing the sign of its accumulated felt-experience association A (Branch 1) to
   the sign of its current stance s (same sign = confirm, strengthen toward the current pole; opposite = erode,
   flip toward the other). A fully-blind six-agent framing panel (three types, three models, de-narrivatized,
   none seeing the conclusion) UNANIMOUSLY (6 of 6, none "sound-as-is") caught it, and I verified the algebra
   against source: the comparison is VACUOUS. In both branches the kernel target reduces to `sign(A)` (same-sign:
   target = sign(s) = sign(A); opposite-sign: target = -sign(s) = sign(A)), so s never affects the direction.
   The actual rule is "reserves rose this span -> move the stance toward the numerically +1 pole; reserves fell
   -> toward -1," for every axis and every world. That AUTHORS the axis's meaning: it fixes the +numbered pole
   as the thriving/good pole everywhere, the exact "read the meaning of a symbol/axis" the template case forbids
   (P9 / the value-authoring line). The panel's FORMAL TEST (the load-bearing tool): the move must be INVARIANT
   under relabeling an axis (negating which pole is +1 must leave every being's physical trajectory unchanged);
   the first-cut fails it, a correct move must pass it. THE CORRECTED FRAMING (mandatory, all six): make the move
   relabel-invariant by making the association POLE-REFERENCED. Branch 1's accumulator changes its engagement
   weight from `|stance|` to the SIGNED stance, so `A = sum(felt.valence * intensity * stance)` and `sign(A)` is
   a per-being LEARNED, relabel-invariant fact ("which pole was good to hold, for me"); Branch 2 then feeds the
   AGM kernel `toward = sign(A)`'s pole, magnitude `|A|`, gated by entrenchment. Under relabel, stance -> -stance
   so A -> -A and the target tracks the SAME physical pole (invariant, meaning-free). This captures the owner's
   target: a being that HELD "providence is benevolent" and SUFFERED accrues A pointing away from that pole and
   erodes/flips toward its opposite (resentment), whichever numeric sign the world gave the poles. THE OPEN FORK
   (gp-sonnet's deeper catch, surfaced for the owner): even relabel-invariant, `target = sign(A)` hardcodes ONE
   epistemic polarity, "felt-good confirms a conviction, felt-bad erodes it," which forecloses the real cultural
   mode where felt HARDSHIP VALIDATES a conviction (asceticism, martyrdom, costly-signal belief). The fix is a
   per-being (or per-race) epistemic-polarity disposition `p` mediating the move (`target = sign(p * A)`): p>0
   hedonic, p<0 ascetic, so which epistemology a being/lineage has EMERGES rather than being authored. Options
   the owner is deciding between: (a) p as a per-race innate EPISTEMIC DISPOSITION (a field on `EpistemicStance`,
   the sibling of dogmatism, P9-legal authored input, hedonic default, per-race variable, seeded and inherited
   like the axiom seeds); (b) p as a per-individual HERITABLE genome-expressed trait, founder-zero, selected
   (gp-sonnet's strict reading, fullest emergence, but the coupling only fires once selection lifts p); (c) a
   hedonic always-on base now (like the floor-grounded harm/reward learners) with the ascetic-p generalization
   flagged as a follow-on. Reserved either way: the move threshold/rate (basis: the axiom kernel's existing
   entrenchment and plasticity), and p's default and range. The relabel-invariance fix is not optional and is
   built regardless; only the polarity fork awaits the owner. OWNER CHOSE (a) 2026-07-08: p is a per-race innate
   EPISTEMIC DISPOSITION (a field on `EpistemicStance`, the sibling of dogmatism), a P9-legal authored input
   with a hedonic default and per-race variation, seeded at the dawn and inherited like the axiom seeds, so a
   hedonic race and an ascetic race are data rows and asceticism is representable without foreclosure. Built on
   this basis.

R6. **The experiential-conviction arc PASSED the mandatory five-lens audit; follow-on refinements surfaced.**
   The whole arc (Prereq A felt_salience, Prereq B the conviction-percept threading, Branch 1 the correlation
   record, Branch 2 the conviction move) was audited by the five standing lenses plus correctness (six blind
   panelists, then adversarial per-finding verification, 19 agents), each finding re-verified against source.
   VERDICT: SOUND. The load-bearing claims held under blind scrutiny (relabel-invariance of the Branch-2 move,
   byte-neutral opt-in, weight-agnostic Branch 1, felt_salience alien-clean, the rising-reserve valence
   convention is the floor-wide normalized-level convention not a Terran more-is-better assumption). Confirmed
   findings were fixed (a determinism-contract RES_WORLD phase-access omission; the felt-move overflow/abs
   robustness; the retention doc's inaccurate basis; the flat-gate "entrenchment" overclaim) or answered with
   tests (the per-race polarity proven through the real dawn-seeding and inheritance path; a hedonic-being
   reinforce test showing the move turns on the lived correlation, not only the polarity knob). FOLLOW-ON
   REFINEMENTS the owner may want, none blocking (recorded here so they are not lost): (a) the ENTRENCHMENT-RANK
   -SCALED felt move gate (today the felt move uses a flat gate and does not read the axiom's entrenchment rank,
   so a labile and a calcified conviction are equally movable by felt experience; the rank-scaled gate over the
   reserved entrenchment curve is the faithful refinement); (b) a from_manifest fail-loud calibration read for
   FeltConvictionCalib (the shared follow-on with the other opt-in learners' dev-fixture calibs, due when a
   Calibrated production scenario first arms the learner; none does today); (c) felt_salience is a NET
   reserve-health primitive, so reserve-neutral churn (trading one reserve for another) reads calm and the felt
   intensity scales with how many reserve axes a world declares; a churn-sensitive or per-axis-normalized felt
   measure is a refinement of the Prereq-A primitive; (d) run_world's dev races all use the hedonic polarity
   default (the ascetic path is proven by test through the dawn, but no dev scenario yet declares an ascetic
   race to watch it in a run). The Part 28 design-doc consolidation (a Part 62 record and the Decided-and-
   reserved blockquote, per R2's "the design-doc Part 28 consolidation happens when it is built") is the
   remaining formal step, surfaced for when the owner wants the arc folded into the canonical design document.

R4. **Branch-1 (correlation-record) framing CORRECTED by a second blind framing panel (2026-07-08).** The R2
   substrate's Branch 1. The first-cut framing defined a conviction's eligibility for felt-experience
   association as (evolved controller weight x stance value), glossed as "the degree the conviction influenced
   behaviour this tick." A fully-blind six-agent framing panel (three types, three models, de-narrivatized,
   none seeing the conclusion) caught the SAME seam UNANIMOUSLY (6 of 6, none "sound-as-is"), verified against
   source: (1) STATICNESS: both factors are constant within a life (controller weights change only across
   generations; stance is read-only on the run path, moving only by cadence-gated enculturation that run_world
   does not even arm), so weight x stance is a per-being CONSTANT MASK recomputed to the same value every tick,
   the "decaying trace" does no credit assignment, and it becomes non-defeasible glue between a persistent trait
   and an outcome (a template-case / P9 violation, indistinguishable from authoring). (2) MOTILITY-PARASITISM:
   gating on the behaviour weight means a founder's unweighted convictions, and any conviction on a
   sessile/immobile being, can never form an association however strongly their value tracks the felt swings, so
   the substrate's coverage is silently contingent on motility (an admit-the-alien / P8 violation) and can only
   rediscover what selection already wired into movement. THE CORRECTED FRAMING (derived, preserves R2's core):
   drop the behaviour-weight coupling; mirror R2's own cited precedent, the harm learner (which correlates
   felt-harm with the varying perceived feature weight-agnostically, defeasible via a BENIGN counter-signal).
   Branch 1 is a per-being DECAYING SIGNED ACCUMULATOR per HELD conviction axis: each felt event folds the felt
   summary (`physiology::felt_salience` valence x intensity) into the accumulator of each conviction the being
   currently holds, engagement-weighted by the conviction's strength (|stance|), with a retention decay so it
   tracks RECENT lived valence and can un-form (defeasible). It reads NO behaviour weight (weight-agnostic,
   alien-clean); it changes no conviction and no behaviour (inert recording; Branch 2 consumes it); a being that
   holds no conviction or a world that does not arm the learner records nothing (byte-neutral honest default).
   Per-axis divergence emerges from WHEN each conviction was held relative to the being's lived valence (a
   conviction held through hard years accumulates negative, one acquired after fortunes improved accumulates
   positive), so the selectivity is lived-contingent, not seed-pinned. DIVERGENCE FROM R2's WORDING, surfaced
   for owner review (not blocking, derivable): R2 said the pole follows "whether the felt outcome confirmed or
   disconfirmed the stance the being was ACTING ON"; the panel found the literal behaviour-coupled reading of
   "acting on" IS the defect, so the corrected framing uses "held while experiencing" (weight-agnostic), which
   is MORE faithful to R2's own cited harm-learner primitive and preserves R2's core (felt experience emits only
   magnitude + valence; which conviction it bears on is a learned per-being coupling). HONEST LIMIT (unchanged,
   panel-confirmed): across-conviction attribution is DIFFUSE at the controller-percept tier (a felt outcome
   folds into every held conviction, not one specific conviction); crisp per-conviction attribution needs the
   deliberative tier, a future refinement. Reserved: the association retention/decay rate (basis CORRECTED
   2026-07-10 by the log audit: this is a LIFETIME-INTEGRATION window, so a conviction averages felt experience
   across a life, and it is NOT the action-eligibility decay of the reward and harm learners; the built dev value
   `Fixed::from_ratio(15, 16)` (`conviction_experience.rs`, a slow ~11-tick half-life) is deliberately slower than
   and NOT equal to those learners' `1/2` eligibility decay, so the owner sets it by the integration window a
   conviction should average over, never by equating it to the learners; the earlier "set equal for consistency"
   basis was wrong and would have misdirected to ~1/2, the pathological forget-in-a-few-ticks setting) and the
   engagement weighting (|stance| the interim; uniform the alternative), owner's to confirm.

R3. **Lifespan must be DERIVED from anatomy, not authored. OWNER DIRECTIVE 2026-07-08, a note to honor when
   the lifespan/R-AGING work is built (not built now, surfaced so it is recalled then).** Today `Race.lifespan_years`
   and `maturity_years` are authored per-race numbers (a plain owner-set count, `crates/sim/src/race.rs`,
   design Part 20), and the individual-tier mortality rolls each being against an owner-supplied age-hazard
   curve (`World::apply_mortality`). This is data-driven and per-race differentiable (a short-lived and a
   long-lived race are different data rows, the R-AGING keystone, built and tested), but the lifespan is not
   grown from the body. The owner's directive: it must be DERIVED from the race's own anatomy and physiology,
   the way `physiology::derive_base_drain` derives metabolism from the body, so lifespan follows from body mass
   and metabolic rate (the mass-longevity and rate-of-living scalings), organ integrity and repair capacity,
   and the body's own physics, and a large slow-metabolism race lives for decades or centuries while a small
   fast one lives briefly BECAUSE of its body rather than a typed value. The authored number is the interim
   scaffold; the target is a senescence law that reads the being's own body (the derive-not-author line: author
   only in the physics floor, grow the rest), so a magical / silicon / photosynthetic race gets its lifespan as
   a data row from its own anatomy, and medicine (design Part 22/34) later modifies it. Recorded durably at the
   `lifespan_years` field doc-comment and the R-AGING design flag (`docs/audit.md`) so the builder sees it.
   Not a blocker for the current arc; a directive for the lifespan build.

0. **The `--scenario full` collapse: RE-DIAGNOSED 2026-07-08 (my earlier soil-draw diagnosis was WRONG,
   corrected here).** With the edibility grounding, default/discovery/viability all THRIVE; only `--scenario
   full` collapses. EARLIER (incorrect) claim: the producer food-override drives the extract cycle to
   over-draw the soil. FALSIFIED by controlled A/B: zeroing `draw_fraction` (no soil/water draw at all) still
   collapses full identically, so the extract DRAW is NOT the cause. THE REAL CAUSE (instrumented via
   `take_obs_deaths`): the full/viability founders are a GRAZER + OILSEED HYBRID (viability_homeostatic =
   `dev_grazer` energy(0)/water(1)/temperature reserves PLUS the oilseed seed reserves), and they die of
   THIRST (death axis 1 = WATER), not starvation. `set_producer` writes each real plant's biomass as the
   ENERGY food only at PRODUCER cells, which makes those cells an energy ATTRACTOR: the founders' forage taxis
   pulls them to congregate on producer cells, and where those cells are dry they die of thirst. This is why
   `MAX(producer, climate)` and bumping the producer biomass did NOT help (the cells stay an energy attractor)
   but disabling `set_producer` DID (uniform energy, the founders spread out and reach water). So it is a
   SPATIAL food-versus-water foraging coupling, tangled with the confused hybrid founder food setup, NOT a
   metabolism-rate or soil-depletion issue. IMPLICATION for the owner's "not just authored oilseed eaters"
   question: the clean fix is not a rate tweak, it is to RATIONALIZE the full-scenario founder food (forage
   the real biosphere producers cleanly like the DEFAULT grazer world already does, retiring the oilseed
   hybrid) and ensure the real producer food does not create a dry-cell thirst trap (a spatial energy/water
   balance). This IS the food-web integration (`docs/working/FOODWEB_INTEGRATION_PLAN.md` slices D + I), now
   understood to be the actual `--scenario full` fix, not a separate biosphere-balance rate pass.

1. **R-UNITS-PIN: the reserve's absolute joule scale** (the `MetabolicAnchors` energy-density-to-joule
   anchor). Dev-set INTERIM value: `LocomotionParams::food_energy_density = 3000` (the forage reconciliation,
   calibrated so the default/discovery/viability worlds thrive). The geophage direct-fill needs no separate
   scale. Owner sets the canonical anchor. TWO HONEST LIMITS the end-of-arc audit confirmed, both surfaced not
   hidden: (a) the value was tuned by watching an AGGREGATE outcome (the population trend at seed 0x5EED,
   0xBEEF, 0xF00D) that is downstream of many OTHER simultaneously-dev-set reserved values, so "the world
   thrives" is a dev-harness calibration that a viable world EXISTS at this point, NOT a validated proof the
   physical model's absolute scale is correct; the owner's calibration replaces it against a real target.
   (b) `food_energy_density` is a SINGLE GLOBAL scale applied to EVERY backing class uniformly (energy, water,
   a mineral, a mana axis all reconciled by the same 3000), a functional simplification: the correct form is
   PER-CLASS content (each food's own per-supply content on each class), which lands NATURALLY when T3 is
   wired (the standing food carries the producer's own composition, so the plant's own `bio.energy_density`
   supersedes the global scale per cell). Until then the global scale is the interim, alien-imperfect (a
   mana-fed world's mana food is scaled by the energy reconciliation); the mechanism keys on the class as
   data, only the reconciliation magnitude is shared.

5. **The per-class physiological REQUIREMENT datum is no longer read on the physical intake path** (the audit
   flagged this). The old satisfaction intake read `laws::satisfaction(supply, assim, requirement)`, using a
   being's per-class per-tick REQUIREMENT to shape the fill. The physical intake fills toward the reserve's
   ROOM (capacity minus amount) instead, so the requirement datum is not gated on the physical path (it is
   still read on the no-physiology fallback path). This is a deliberate model change (a being eats until
   sated, room-bounded, rather than to a per-tick requirement curve), not a silent bug, but the owner should
   confirm the requirement datum's role is subsumed by the reserve capacity or restore it as an intake gate
   if a distinct per-tick need is wanted.

6. **The viability calibration has NO scenario-level CI protection yet** (the audit's confirmation-bias
   finding). No `#[test]` asserts a population survives across generations, so the "world thrives" proof is a
   manual run of the (non-canonical) run_world example, and the known `--scenario full` collapse is unflagged
   by any red test. INTERIM: a unit-level regression guard now ties `food_energy_density` to a survivable
   intake regime (a foraging being's per-tick gain stays a meaningful fraction of its reserve), so a scale
   regression fails CI; a full scenario-level survival test (a foraging cohort holds a population over N
   generations, and `full` marked as a known-collapse) is the follow-on the biosphere-balance pass should add.

2. **The T2/T3/T5 axis-conversion sign-off + Part 62 consolidation** (the chemistry arc, PR #105 merged).
   The mechanism is built and byte-neutral; the design-doc consolidation (a Part 62 record, the
   Decided-and-reserved blockquote, the bibliography, the audit Section 1/2/3 and counts) is the owner's
   resolution step (= R-SOURCE-VECTOR / R-BIO-REGISTRY).

3. **Cluster-I merge checkpoints.** Each Cluster-I arc branch (edibility grounding, Arc 5, 6, 7) is built
   autonomously and pushed as a PR; the owner runs final sims and merges. The chemistry arc (PR #105) was
   merged on the owner's standing authorization; later arcs are queued as PRs for owner review unless the
   owner extends the merge authorization.

4. **The genuine physics-CONSTANT reserved values** surfaced by the grounding, none fabricated: the Kleiber
   coefficient `kleiber_a`, the trophic/assimilation efficiency `ingest_efficiency`, the rock-weathering and
   per-substance decomposition kinetics. These are legitimate physics-floor authored inputs (Principle 9);
   dev-set until the owner calibrates. Not fudges.

7. **Arc 5 T4 residue (byte-changing, flagged not built).** `derive_region` still pads the region ENV vector
   to a fixed four slots with a moisture-DUPLICATE soil-fertility axis (the terrain has three real generated
   axes; the fourth is a moisture copy standing in for a real soil Stock that has not landed). Unifying the
   niche env-axis count with the tile-axis registry and grounding the derived soil axis (a real soil field)
   would drop that duplicate, which re-pins the biosphere, so it is deferred. The floor now carries
   `fluid.moisture_content` with `range_hi = 0.5` (a physics-floor authored bound, Hillel saturation basis);
   the owner may refine it.

8. **Arc 6 grown-body reserved values + the selection follow-on.** `GeneratorParams.ploidy = 2` (the
   sexual-diploid fixture; DATA, so a haploid/clonal alien is a world choice) and `morphogen_gauss` = the
   stamped `SumOfUniforms { k: 12 }` identity (the unset sentinel `k = 0` PANICS on draw, a trap avoided).
   Both reserved-with-basis; the owner sets the world's canonical ploidy and gauss stamp. HONEST LIMIT for the
   owner's awareness: epoch selection applies one uniform per-species fitness scalar across every locus, so a
   grown body's SHAPE is selected only as a side effect of regional niche fit, never because the grown
   Structure's own capability or viability was read. A fitness term reading the Structure (so morphology is
   selected on its own merits) is a natural next research item, surfaced not silently carried.

9. **Arc 7 (creatures-have-simpler-minds) first slice BUILT (`45b269e`), creature-SURVIVAL is the follow-on.**
   Behind `--creatures` (requires `--scenario full`), 131 biosphere consumers spawn as living `Walker`-agents
   riding the founder embodiment loop, byte-neutral off (full unchanged 1c7cf2f2), worker-invariant. The
   runner-lifecycle trap is fixed (a creature is retired when IT dies, a founder when its MIND dies) and the
   creature id namespace is provably disjoint (asserted). HONEST LIMIT the owner should know: the creatures
   spawn with FULL reserves but DIE within the first tick, because the metabolic Kleiber drain exceeds their
   reserve at the small biosphere body scale (body_mass ~0.06 to 0.9) in the oilseed-based dev food world.
   This is the SAME metabolism-calibration class as R-UNITS-PIN and the item-0 `--scenario full` collapse, and
   it is the crux of the owner's "not just authored oilseed eaters" question: making creatures (and the whole
   real biosphere) survive is a metabolism/food BALANCE, surfaced for a dedicated pass, being scoped in
   `docs/working/FOODWEB_INTEGRATION_PLAN.md`. Two further Arc-7 slices, status corrected 2026-07-10 by the log
   audit: the being-perception percept (so flee/chase can EMERGE) is NOW BUILT and MERGED as the creatures-react
   capacity #118 (`373e0d8`), landing byte-neutral and off by default (founder-zero), NOT the re-pinning
   `ControllerLayout` change this entry predicted, so "the controller perceives no other beings today" is stale;
   see R12. Creature reproduction/selection (so good foragers are selected) stays deferred, in progress on #120 as
   the predation-integration slice that makes the reaction emerge. Both stay a general percept + evolved
   controller + selection so predator-avoidance EMERGES, never authored.
