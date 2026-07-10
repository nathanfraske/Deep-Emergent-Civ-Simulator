# Calibration reconciliation: the companion to the floor reconciliation (reserved.toml audit)

This is the companion to `PHYSICS_FLOOR_RECONCILIATION.md`, which audited the floor registry and scoped OUT the
`calibration/reserved.toml` values, naming them the likeliest home for a biological or cultural outcome wearing
calibration clothing and calling for their own basis-and-citation audit. This is that audit. It applies the standard the
owner locked on 2026-07-10 (`AGENTIC_ADDENDUM.md` section 9, the per-world-outcome rule, commit `81b541d`): a world
OUTCOME that is legitimately tunable is never authored as a global constant, because a single locked value that sets how
an emergent outcome comes out is Principle-9 steering wearing a number; such a value is a PER-WORLD calibrated scenario
datum read from the world's own data, with Mirror calibrated to real measured data so the Terran pattern EMERGES from
real inputs and any alien is a data-row override. The one-line discipline: default to Mirror by calibration to real
data, admit the alien by data, and author nothing globally.

Audited against `main`'s `reserved.toml` (222 entries, 119 set), not the branch copy. The sweep ran a section-11-gated
seven-lens blind panel (the five mandatory lenses plus a cross-cutting structural/missing-axis lens and a
joint/relational-pairs lens), with the burden of proof on the CLEARING verdict and the bases NEUTRALIZED so the panel
judged cold. The section-11 smoke test failed closed four times before the recorded run, each catch a real
construction disease: residual self-verdicts leaking exoneration; no whole-manifest lens for a MISSING per-race axis;
the DECOY-AXIS bake (a genuine per-race axis on one input covering a human-anchored shared scalar on another); and
unguarded clearing categories (a Terran empirical prefactor cleared as world-invariant, a behavioural magnitude cleared
as a normalization, and joint/relational authoring owned by no single scalar). Every load-bearing verdict below is
verified against source by hand (Prime Directive 1); the panel is a lead generator, not a verdict, and the
source-check overturned the panel's aggressive majority on two values.

## Verdict taxonomy

Two verdicts, per the locked rule. LEGITIMATE-DATUM: a control set-point the engine models no deeper whose value is
world-invariant with proof, or a true normalization that cancels downstream, or a datum whose per-race axis already
exists. PER-WORLD-IFY: a per-world/per-race outcome currently authored as a single global constant, which should become
a per-world calibrated scenario datum (reserved-with-basis, read from world/race data), with Mirror set to the
real-measured value so the Terran pattern emerges rather than being coded. PER-WORLD-IFY is NOT deletion: the value
stays, Mirror keeps its real-data number, and the alien becomes a data-row override.

## Of nineteen values audited: seventeen PER-WORLD-IFY, two LEGITIMATE

The result mirrors the floor reconciliation's own: with the burden on the clearing verdict, almost every suspect is an
outcome wearing calibration clothing. That is the expected shape (the floor sweep named this file as the likeliest home
for exactly this), and it is why the per-world-outcome rule was locked. Two values survive source-checked scrutiny as
legitimate, and the source-check pulled both back from the panel's relocate majority.

### LEGITIMATE-DATUM (source-verified, diverging from the panel)

`controller.taxis.ingest_drive` (1): a true engine-units NORMALIZATION, not a behavioural outcome. Verified at
`controller.rs:815-817` and the selection clamp: unity is the ceiling of the `[0,1]` INGEST activation clamp the MOVE
output shares, so any value at or above 1 clamps identically (an arbitrary saturating scale), and HOW MUCH a being
draws is governed downstream by the reserve-room bound in the ingest arm, not by this magnitude. The founding drive is a
reaction-norm SEED that then evolves; the foraging behaviour emerges from the evolved controller and the reserve-room,
so the magnitude carries no behavioural consequence relative to another setting at or above the clamp. Passes the
normalization proof. The panel split 3 legitimate / 2 relocate / 2 unsure; the source confirms legitimate.

`evidence.runner_up_margin` (2 nats): borderline, and legitimate on source. The panel's structural lens relocated it as
a commitment set-point with "no per-race axis," but that per-race axis already EXISTS: `design.md:1029` states a mind's
epistemic stance (Part 28) sets the prior and the margin, so a skeptic demands a wider lead and a credulous mind commits
readily. The reserved 2 nats is the decision-theoretic Mirror BASE (7:1 odds, inference to the best explanation) that
the existing per-mind epistemic-stance axis modulates, which is precisely the Mirror-base-plus-per-race-axis shape the
locked rule asks for. Kept legitimate with the note that its status depends on the epistemic-stance axis in fact
reaching it; if a future audit finds the margin is read unmodulated, it falls to PER-WORLD-IFY.

### PER-WORLD-IFY (a per-world/per-race outcome authored as a global constant)

Grouped by why each is an outcome, not a datum. Each keeps Mirror at its current real-data value and becomes a per-race
or per-world calibrated datum.

Self-flagged by their own basis (the manifest already carries a STEER FLAG). `gossip.trust_baseline` (0.5, the human
investment-game figure, Berg/Dickhaut/McCabe 1995) and `gossip.trust_penalty` (0.5, the human trust-asymmetry, Slovic
1993): the bases say each "should become per-race epistemic-stance data" pending the per-race stance axis. And their
EQUALITY (baseline == penalty) authors a trust build-versus-destroy symmetry owned by neither scalar alone (the
relational lens). `axiom.calcification_rate` (0.001): the basis says the absolute per-tick rate "bakes in a
human-generation lifespan" and should re-express as dimensionless lifespans-to-harden divided by each race's lifespan.
These three are the clearest: the design already knows they are per-race outcomes.

A retired-to-derive sibling proves them derivable. `gossip.told_weight` (2 nats) and `tom.access_weight.denied` (4 nats):
the theory-of-mind assertion ladder was already RETIRED and made to DERIVE (told = commit_threshold, witnessed = commit
+ margin), so these hand-set evidence weights on the same nats scale are the same concept left authored; they should
derive from the evidence primitives. `harm.p_harm_given_harms` (0.9) and `harm.p_harm_given_benign` (0.1): each basis
says it is a dose-and-physics rate the floor implies (the fraction of ticks a naive being is worn faster than it heals;
the false-attribution rate of a transient reserve dip), derivable per-body from the wear-heal physiology, yet frozen as
round shared scalars. Their RATIO (ln(0.9/0.1)) is a signal-detection sensitivity that sets the belief-update strength,
an outcome owned by the pair (the relational lens).

Decoy-axis: a human-anchored shared scalar under a per-race sibling wrapper that does not discharge it.
`lang.typology_temperature` (a single shared softmax temperature calibrated so the sampled word-order proportion
reconstructs the human WALS 95A / Dryer 1992 distribution; the per-race working-memory input sits on a different term).
`transmission.drift_rate` (0.03, the shared base half-width anchored to Weber's ~3% human just-noticeable difference;
the per-copier `copy_drift` layer is the sibling). `langmod.blend_propensity` (0.85, a human bimodal-bilingual
blend-versus-switch measurement read as the shared base under a per-culture/per-being wrapper). `axiom.evidence_ring_curve`
(the ring size is read per being over its own memory, but the two curve anchors 0=0,8=16 encode the human Miller 7+/-2
span uniformly). In each, the alien's outcome distribution is still pulled toward the human one through the shared
scalar.

A Terran empirical prefactor riding a genuine floor law. `metabolism.kleiber_coefficient` (3.4): the 3/4 exponent is a
universal fractal-network affordance (floor), but the coefficient 3.4 is a measured mammalian normalization loaded once
(`physiology.rs:151`, one `kleiber_a` applied to every being via `basal_metabolic_rate`). The basis itself calls it "a
per-race datum rather than a universal"; a photosynthetic, chemosynthetic, or silicon metabolism does not share the
mammalian prefactor. Belongs as per-race data on the race registry, Mirror = 3.4.

Basis is the outcome it produces, or it authors a social/behavioural dynamic directly. `value_metric.conflict_coefficient`
(1.0): wires value-distance times the coefficient into conflict pressure, the canonical authored social dynamic the
Steering Audit forbids; `design.md:1801` already reserves "the coefficient mapping value distance to conflict pressure"
as a numeric calibration "with no empirical anchor for non-human minds," and the same value-distance also feeds
enculturation pull and deity favour, so the coupling itself, even at 1.0, is the per-world knob. `being.life_event_impulse`
(0.3): a personality self-change burst size whose only basis is the outcome it produces; its siblings (plasticity-by-age,
maturity targets) already derive or are per-race trait data, so a resilient race should shift less. `axiom.conformity_prestige_strengths`
(conformity=0.1, prestige=0.1): two interaction-strength knobs that scale how far an opinion moves toward a conformity
target and a prestige target, authoring the opinion-change dynamic directly; and their EQUALITY authors an equal social
weighting of conformity versus prestige (the relational lens). `genome.selection_scaling` (0.2): its only basis is the
outcome it reproduces (set so domestication and disease resistance shift at the historical pace), a coefficient tuned to
a Terran timeline. `genome.speciation_distance` (0.30): a single threshold declares reproductive isolation, but the
distance at which incompatibility arises is per-lineage (Coyne and Orr's own data vary by taxon, and the sibling basis
concedes the specific cutoff is a design ratio, not grounded), so it should emerge per-world.

## Cross-cutting: joint and relational authoring (owned by no single scalar)

Four authored outcomes live in a RELATIONSHIP between scalars, so per-scalar treatment alone under-colors them, and the
relational lens surfaced them: the harm likelihood RATIO (a d-prime sensitivity), the conformity-versus-prestige
EQUALITY, the trust build-versus-destroy SYMMETRY (trust_baseline == trust_penalty), and the shared log-odds/nats
EVIDENCE SCALE spanning `evidence.runner_up_margin`, `gossip.told_weight`, and the theory-of-mind weights (what "2 nats
= meaningful, 4 nats = strong" means is a jointly-authored magnitude scale). When these are per-world-ified, the
RELATIONSHIP should be what a world's data determines, not two independently-set numbers whose ratio happens to encode a
Terran outcome.

## Honest limits

The sweep judged nineteen values (the six the floor reconciliation named plus a representative spread across the social,
cognitive, linguistic, and genetic mechanisms), not all 119 set values; the seventeen-of-nineteen relocate rate says the
pattern is pervasive, so the remaining set values (the physical/world data: `medium.air`/`water`, `climate.*`, the
orbital and rotation periods, `metabolism.stefan_boltzmann`, `world.*`) are the likelier legitimate-datum population and
a full pass should confirm them, but any behavioural, social, linguistic, or per-race-biological set value not yet judged
is a candidate for the same verdict. The relocate verdicts identify WHERE the per-world axis belongs; the derivations
and the per-race axes themselves (the reciprocal-altruism trust structure, the lifespan-normalized calcification, the
per-body harm reliabilities, the per-race personality plasticity) are follow-on work, not done here. Two verdicts turn
on a claim to be re-checked at build time: `runner_up_margin` is legitimate only if the epistemic-stance axis truly
modulates it, and `ingest_drive` is legitimate only while the reserve-room bound (not the drive magnitude) governs
intake. This audits the reserved MANIFEST; a Principle-9 literal hardcoded in Rust and never surfaced as a reserved
value is out of this sweep's reach and belongs to the floor registry's own `laws.rs` audit.
