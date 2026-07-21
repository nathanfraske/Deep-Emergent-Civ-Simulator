# R-DEEPTECH audit: literature claims verification

Verification of the thirteen claims the north-star audit
(`audits/deeptech-northstar-2026-06-29.md`) flagged for primary-source
checking. Five research agents checked them against primary sources. No claim
was refuted; the audit's substance holds. The corrections are citation-level,
plus three nuances that refine the design and are folded into the sharpened
flags and the Part 41 convergence audit. Apply the corrected citations when any
of this is consolidated into the design document.

## Verdicts

1. **BACON rediscoveries: partially-confirmed.** The rediscoveries are correct (BACON.3, Langley alone, IJCAI 1979, recovered Kepler's third, the ideal gas law, Ohm's law, Coulomb's law; BACON.5, Langley, Bradshaw, and Simon, IJCAI 1981, recovered conservation laws). Correction: the 1981 Cognitive Science article "Data-Driven Discovery of Physical Laws" (5(1):31-54) is authored by Langley alone, not by Langley, Bradshaw, and Simon; the three-author byline belongs to the BACON.5 paper and the 1987 book.

2. **Schmidt and Lipson plus the implicit-prior caveat: confirmed.** Schmidt and Lipson, Science 2009 (324(5923):81-85). The caveat is Hillar and Sommer, arXiv:1210.7273, a preprint comment (the method implicitly assumes Newton's second law or Hamilton's equations), so cite it as a preprint rather than a peer-reviewed Science reply.

3a. **Vector Quantized-Elites: confirmed.** Tsakonas and Chatzilygeroudis 2025, arXiv:2504.08057; an unsupervised, problem-agnostic quality-diversity method that learns a discrete behaviour space via a VQ-VAE.

3b. **AURORA: confirmed, with a design-relevant correction.** Grillotti and Cully 2021/2022, arXiv:2106.05648 (the acronym is AUtonomous RObots that Realise their Abilities). The descriptor encoder is trained online during the run and re-encodes the container on each update; it is not freezable offline as published. A determinism fix that depends on an offline-frozen learned descriptor must therefore use a separately pre-trained encoder frozen and integer-quantized before the run, which is a deliberate departure from AURORA, exactly as RD-NT-2 specifies.

4. **QD-score and coverage: partially-confirmed.** The definitions are canonical (QD-score is the sum of normalized elite fitness over filled cells; coverage is the proportion or count of filled cells). Correction: the term QD-score originates with Pugh, Soros, and Stanley 2016 (Frontiers in Robotics and AI), not Mouret and Clune 2015; cite Mouret and Clune 2015 for MAP-Elites and coverage and Flageat et al. 2022 (arXiv:2211.02193) for the normalized-sum benchmarking definition.

5. **Stayton convergence metrics and the cautions: confirmed.** Stayton 2015 (Evolution 69(8):2140-2153) introduced C1 through C4; Speed and Arbuckle 2017 (Biological Reviews) warn the metrics conflate convergence with stasis. Correction: the 2022 cautionary note is now published as Grossnickle et al. 2024 (Evolution 78(8):1355-1371), which shows the C-measures often misidentify divergent lineages as convergent and proposes time-corrected measures; cite the published version.

6. **Disparity-through-time against a Brownian-motion null: confirmed.** Harmon et al. 2003 (Science 301:961-964) and the morphological disparity index, with the empirical trajectory compared against a pure-drift Brownian-motion null (the geiger `dtt` implementation). The drift-only null framing is standard.

7. **Multiscale lift and restrict commutation: confirmed, with a nuance.** Gear and Kevrekidis 2003 (SIAM J. Sci. Comput. 24(4):1091-1106) and Zagaris et al. 2012 (DCDS 32(8):2759-2803): stability rests on a fast and slow spectral-gap separation. Nuance: the later literature (Kevrekidis et al. 2018) treats finite separation quantitatively with Newton and Broyden stabilization, so the accurate statement is that a gap is required and its degree governs stability, rather than an all-or-nothing clean gap.

8. **Aggregation requires linearity or knife-edge separability: confirmed.** Theil 1954, Leontief-type conditions, Gorman 1953/1961 (the Gorman polar form), and Nataf 1948 for production. Exact aggregation of a nonlinear process generally fails without linearity or the Gorman and Nataf separability restrictions, which grounds the claim that a nonlinear innovation yield is not an exactly conserved aggregate projection.

9. **Transistor prerequisite chain: partially-confirmed (one date).** Wilson 1931 (band theory), Schottky 1938 and Davydov 1938 (rectification), and high-purity germanium from WWII radar work are accurate. Correction: Mott's rectifier theory is 1939 (Proc. R. Soc. A 171:27-38), not 1938.

10. **Cultural ratchet: confirmed.** Tennie, Call, and Tomasello 2009 (Phil. Trans. R. Soc. B 364(1528):2405-2415) on faithful transmission via process copying plus teaching, conformity, and sanction; Vaesen et al. 2016 (PNAS 113(16):E2241-E2247) that population-size-as-cause is contested. They bound the mechanism but supply no quantitative critical-practitioner-mass number, so a mass threshold must be a derived function, never a constant.

11. **Gerber on the Raup morphospace: confirmed.** Gerber 2017 (Biological Reviews 92(2):1142-1155): the shell-coiling morphospace axes are non-metric and incommensurate, so occupation patterns are scaling-dependent and empty-versus-occupied regions can be parameterisation artifacts. This grounds the requirement for a descriptor-invariance test rather than trusting a single convergence index.

12. **Decoupling: confirmed.** Appelquist and Carazzone 1975 (Phys. Rev. D 11(10):2856-2861): heavy-mass effects below the mass are absorbed into light-theory couplings or suppressed by powers of the scale ratio. It fixes the form of suppression and supports "a tier is a regime of validity," but it does not fix the absolute band placement relative to an observer's perception scale, so tier-as-regime still leaves a free dial.

13. **Gero-Kannengiesser FBS of processes: confirmed.** Gero and Kannengiesser 2007 (AI EDAM 21(4):379-391) supports representation adequacy (a process can be cast as a function-behaviour-structure triple and classified) and does not establish generativity or that an artifact evaluator's combinators transfer to processes; narrow the basis to representation adequacy.

## Design-relevant nuances folded into the sharpening

Three verifications change a design statement rather than only a citation: claim 7 (the multiscale commutation depends on the degree of spectral separation, not an idealized clean gap, with Newton and Broyden stabilization available), claim 3b (the learned-descriptor determinism fix must use a separately pre-trained, offline-frozen, integer-quantized encoder, since AURORA as published trains online), and claim 12 (the decoupling theorem fixes the form of suppression but not the tier band placement, so tier-as-regime remains a steering surface).
