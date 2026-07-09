# Handoff: stand up Mirror and Tempest, then the liveliness arc

This is the brief for the agent standing up the canonical pipeline. The owner gates it through the PR (sign off
per slice, keep a standing build-ahead directive, end each gate with SIGNED OFF). Everything you need is here or
linked. The prime directives (`CLAUDE.md`) and the mandatory audit lenses (`AGENTIC_ADDENDUM.md` section 9)
bind every step.

## The cadence (how we work together, since you are a cloud agent)

We communicate through the PR. Read this before starting, and re-read it and the PR comments before starting
every new segment.

- **First action: create a branch off `main` (e.g. `claude/mirror-canonical-pipeline`) and open a PR to `main`,
  then do all work there.** Read the "Feature status board" in `docs/working/CONSENSUS_ROADMAP.md` first to
  know what is and is not built. Then start Arc 1 (the loader). Post a first comment on the PR naming the arc
  you are starting so I can find it.

- **Work in segments, push after each.** After you finish a coherent segment (a slice or a whole arc), commit,
  run the verification (below), and PUSH to the PR branch. Do not batch many arcs into one silent push; push
  each segment so I can review it while you build the next.
- **Audit after each ARC, before you push it for review.** At the end of each arc (not each tiny slice), run
  the mandatory five-lens audit (`/panel audit`, `AGENTIC_ADDENDUM.md` section 9): the five standing lenses
  (blind confirmation-bias, derive-versus-author, alien-feasibility, Terran-bias, steering/Principles) plus
  correctness, each an independent panelist, every finding verified against source, real defects fixed and
  honest limits logged. State the audit verdict in your push. An arc is not done until its audit has run.
- **My reviews come to you as PR COMMENTS. Subscribe to them and poll them.** After each push, I review the
  diff and leave PR comments: approvals, redirects, or fixes. Before you start the next segment, FETCH and read
  the new PR comments (issue + review comments) and address any I left. A comment from me that is a redirect or
  a fix takes precedence over the standing plan.
- **Build ahead.** Do not stall waiting for my review between segments. Keep building the next segment while I
  review the last. If I need you to change course, I will say so in a comment and you adjust; otherwise proceed.
- **I transition you to new arcs through the PR.** When an arc is done and reviewed, I will post a comment
  directing the next arc (or a new branch if the scope warrants). Read it before proceeding. My directives
  reach you only through the PR, so treat a new comment from me as authoritative.
- **I handle the merge.** Do not merge. When I judge an arc ready and its audit clean, I merge it. You keep the
  branch moving.
- **Sign-off protocol.** End each arc's push with a clear "ARC N COMPLETE, AUDIT: <verdict>, ready for review"
  line so I can find the review point. I end an approval with SIGNED OFF; a discussion comment without SIGNED
  OFF is a hold, not an approval, so do not treat steering as sign-off.
- **Verification every push:** `cargo test -p civsim-sim` (and `-p civsim-world` if touched), `cargo fmt --all
  --check`, `cargo clippy -p civsim-sim --lib --tests` (0 warnings), and the byte-neutral pins + worker
  invariance for any change to an existing path (default 2b7e1035, full 1873c44e, discovery 4eea5d06,
  viability bae5a82; t1==t8).
- **The roadmap status board is the living source of truth: read it on START, update it on STOP.** At the
  start of every work session read the "Feature status board" in `docs/working/CONSENSUS_ROADMAP.md` to know
  what is and is not built. Whenever a segment makes progress on any item there (a NOT DONE becomes DONE, a
  GATE clears, a new arc is flagged), update the board in the SAME push so it never goes stale. The board must
  always answer "is X done yet" from a plain read; a stale board is a defect. This is the point of the
  consensus roadmap, so treat keeping it current as part of the work, not an afterthought.

## The arcs, in order

1. **Arc 1, the loader** (task #5). Make the scenario name shape world structure (worldgen + biomes, then
   magic) and move the learning/discovery/tool/conviction features onto the canonical `build_dawn_runner` path
   fail-loud from the manifest. Prove a scenario builds under `Profile::Calibrated` (it will fail-loud on unset
   values; that maps Arc 2's work).
2. **Arc 2, the physics + units + Mirror/Tempest calibration** (tasks #6, #8). Set the values from the research,
   with citations; pin the missing reference scales rather than fabricating; boot Mirror and Tempest under
   Calibrated. Present Mirror to the owner for sign-off (the one GATED world).
3. **Arc 3, the liveliness arc** (task #10 and its siblings). The being-perception percept, run-path
   being-vs-being harm, the hunt loop, the biosphere-meets-made-world seam (pick plants, fell trees), and the
   biosphere-balance calibration. Each emergence-critical wire gets a blind FRAMING panel first.

The remaining worlds (Venus, Europa, Arcanum, Confluence, Crucible-full) and their substrate arcs stay flagged
in the roadmap and `WORLD_SUBSTRATE_READINESS.md`; I will direct you onto them (or a fresh agent) when their
turn comes.

## The goal, in order

1. **The scenario-to-Calibrated-World loader.** `build_dawn_runner` already takes a `Profile` and a
   `ScenarioResolution` and arms the social, belief, language, reproduction, and biosphere-supply layers
   fail-loud from the manifest. Two gaps close here: (a) the scenario NAME must shape world STRUCTURE
   (worldgen params, biome set, and later magic), which today are `dev_default()` regardless of scenario
   (`run_world.rs:1838-1839`); (b) the learning/discovery/tool/conviction features are `run_world` opt-ins that
   must move onto the canonical `build_dawn_runner` path (fail-loud from the manifest), gated by what a
   scenario declares.
2. **Set the Mirror (Earth 1:1) and Tempest values.** Chew through `docs/working/MIRROR_CALIBRATION_RESEARCH.md`
   (162 cited proposals across 6 domains). Set the `earth_real` ones with their `basis` + `source`. For the
   `needs_scale_work` and `needs_owner` items, do NOT fabricate: pin the missing reference scale first (in code
   or a documented anchor) so the value DERIVES, or surface it to the owner. Tempest is Mirror's `real` values
   plus the `.high` dial siblings. Also set the four universal constants (Stefan-Boltzmann sigma, Coulomb k,
   vacuum permeability mu_0, gas constant R) to CODATA in their fixed-point per-quantity scales, and
   forward-declare big G (no consumer until orbital mechanics; little g stays per-world).
3. **Boot under `Profile::Calibrated` and prove it.** Iterate: build, read the first fail-loud unset value, set
   it (with citation) or flag it, repeat, until Mirror and Tempest boot. Then prove: determinism replay,
   worker-invariant (t1==t8), the Steering Audit invariants, and a watchable multi-generation run.
4. **Present Mirror to the owner for sign-off.** Mirror is the one GATED world (a 1:1-Earth control). Do not
   treat it as canonical until the owner approves its dial-set. Present the full set with every value's basis +
   source.
5. **Then the liveliness arc** (the owner's headline want: a world TRULY alive, beings acting on materials of
   their own volition, picking plants, hunting and being hunted). Ordered in the roadmap status board; the
   keystone is the being-perception percept (task #10). Each emergence-critical wire gets a blind FRAMING panel
   first (`/panel framing`), and the arc ends with the mandatory five-lens audit.

## The hard rules (non-negotiable)

- **Never fabricate a value.** Surface it as reserved with its basis, or derive it. A `needs_scale_work` item
  means the reference anchor is missing; pin the anchor, do not invent the number.
- **Every per-world value carries its reasoning AND a real-world citation** (`basis` + `source`), never a bare
  number. This is the owner's explicit rule. Mirror especially: every dial traced to a real Earth measurement.
- **Byte-neutral opt-in discipline** for anything added to an existing path; the pins are default 2b7e1035,
  full 1873c44e, discovery 4eea5d06, viability bae5a82, and worker-invariance t1==t8.
- **Framing-panel emergence-critical wires first** (any physical/felt input into a cultural/social/belief
  outcome; any "this KIND causes that KIND" mechanism).
- **Keep the CONSENSUS_ROADMAP status board current** (the "is X done yet" lookup) on every arc; the stop hook
  enforces it.
- **Gate through the PR.** Sign off per slice; the owner reviews and merges.

## Pointers

- `docs/working/MIRROR_CALIBRATION_RESEARCH.md`: the 162 cited Earth-value proposals to chew through.
- `docs/working/WORLD_SUBSTRATE_READINESS.md`: what each world still needs built (Venus greenhouse, Europa
  z-stacking + tidal heating, the AbioticField field-kind registry, the magic system Part 34, Crucible terrain
  + war).
- `docs/working/CONSENSUS_ROADMAP.md` "Feature status board": the authoritative is-X-done lookup, kept current.
- `docs/working/OWNER_DECISIONS_LOG.md`: R1-R6 (band placement, the felt-experience framing, lifespan-derive,
  the Branch-1/Branch-2 corrections, the arc audit) and the biosphere-balance item.
- The task list: #5 loader, #6 physics/units calibration, #7 Crucible (substrate runs; design needs
  terrain+war), #8 Mirror sign-off, #9 Venus/Europa (assessment done; substrate arcs flagged), #10
  being-perception percept (the liveliness keystone).
- `calibration/reserved.toml`: 58 set / 163 reserved; the fail-loud manifest.

## The liveliness arc, scoped (after calibration)

- **Being-perception percept (keystone).** A new percept block on the shared `ControllerLayout` (same pattern
  as the conviction-percept, prove-panelled) that lets a being sense nearby beings, keyed on their BODY PHYSICS
  (mass, weapons, tissue, distance + direction) and never on a species/identity tag, so pursue/flee emerges
  from selection. Founder-zero, byte-neutral opt-in.
- **Run-path being-vs-being harm.** Wire the strike/bite to target a perceived live being's body
  (`body::strike` is built + tested, not yet run-wired for being-vs-being), leaving a corpse (`corpse_matter`)
  the hunter then eats/butchers. Keyed on body physics.
- **The hunt loop.** perceive a creature -> pursue -> strike/kill -> butcher -> use resources; and the mirror,
  a creature hunts a person. All emergent from the being-percept + body physics, nothing authored.
- **Biosphere-meets-made-world seam.** Expose a producer (a tree, a plant) as a forageable/pickable/cuttable
  affordance-target with a resource yield, so a being picks a plant for its parts and fells a tree for wood of
  its own volition.
- **Biosphere-balance calibration.** Set the real per-plant food value (T3 owner-gate) and the creature
  metabolic scale so the full real ecology thrives rather than the loop merely turning.
