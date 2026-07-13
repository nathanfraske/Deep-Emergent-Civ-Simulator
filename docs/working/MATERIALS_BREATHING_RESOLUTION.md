# The finding-1 resolution: ionic charge is not an observable, and the breathing rung (owner ruling)

Owner-authored (2026-07-12), the definitive resolution of finding 1 (the divalent-oxide bulk-modulus overestimate) and B's honest negative. Provenance inline per the seven-tag register. This takes the reframe (not a fitted QEq), for a sharper reason, and fills the rung the three-refinement path missed.

## The premise chased a target that does not exist
The seam's root is NOT that derive-first QEq fails to reach the physical charge. It is that "the" charge of an ion in a crystal is not an OBSERVABLE. The spread for MgO: Bader's topological partition gives +1.7; Mulliken gives ~+1.2 to +1.5; fitted-QEq energy models sit near +1.6; and the Born effective charge, the ONLY one of these that is measured (via the LO-TO phonon splitting), comes out ~1.96 to 2.0, essentially FORMAL [M, verify Karki-lineage phonon papers]. Four definitions, 0.8e of spread, one ground truth. So "covalency bleeds the charge below +2" is not representation-independent physics, and there was never a derive-first path to +1.6 because +1.6 is a property of a fitted energy model, not of MgO. The premise borrowed authority from a number that only exists inside a representation. This is check-the-input applied to the concept of ionic charge.

## The physical root is one banked boolean: EA2 < 0
Free O2- does not exist: the second electron affinity of oxygen is negative by ~8 eV [M, verify]. The crystal's anion is CREATED by the Madelung potential, and its size, stiffness, and density tail are environment-dependent. That one boolean, `EA2 < 0`, forecasts all three findings before any computation:
1. Rung-1 point-charge rigidity must run STIFF for oxides and sulfides (the anion breathes under compression; the rigid model cannot).
2. The Cauchy diagnostic fires: MgO's C12 = 95 against C44 = 156 GPa is a 1.6x violation, while NaCl's 12.9 vs 12.7 passes. The substrate's own fingerprint flags periclase as outside the central-force class BEFORE the modulus is computed.
3. The Clementi-Raimondi escalation HAD to invert, because more accurate free-ion data doubles down on exactly the free-ion representation the EA2 boolean says is invalid here. (The manager's escalation-direction error was the O2- lesson in miniature; B's shielded-vs-bare-Ewald stability catch was the same lesson caught earlier.)

## The missing rung: Gordon-Kim / PIB (parameter-free, 40 years old, built for these exact oxides)
- Gordon-Kim (1972): computes the repulsion as the OVERLAP of atomic charge densities evaluated with electron-gas functionals. No `n`, no fitted `rho`. The exponential repulsive form the three-refinement path wanted is DERIVED (atomic density tails decay exponentially with a constant tied to `sqrt(2*IE)`, the banked column), at estimator grade, the proper value from the overlap integral rather than a formula.
- Muhlhausen-Gordon (1981): the Watson sphere. The O2- density is computed inside a potential well representing its own Madelung site, the `EA2 < 0` physics made self-consistent.
- PIB (Boyer, Mehl, Cohen, mid-1980s): the density RE-RELAXES with compression, potential-induced breathing. The breathing term is a MANY-BODY interaction, which makes it the ionic class's Cauchy breaker, completing the triptych the shear turn left asymmetric: EMBEDDING for metals, KEATING for covalents, BREATHING for Madelung-stabilized ionics.
- PIB-class results land the alkaline-earth-oxide moduli at 5 to 15% with the right-sign Cauchy violations [verify Mehl-Hemley-Boyer 1986, Wolf-Bukowinski 1988, Isaak-Cohen-Mehl 1990]. Its provenance is on-the-nose: it was built to supply mantle elasticity (MgO, CaO, perovskite), precisely R-COEVOLVE's consumer.

So the three "principled refinements" (Bader charge, Born-Mayer form, Keating term) COLLAPSE into one representation upgrade: replace point charges with overlapping stabilized densities, whereupon the charge question does not get answered, it EVAPORATES, no charge parameter exists in the density picture.

## The class split (correcting a prior overreach)
"Ionics: no added term needed" (from the shear turn) was NaCl-calibrated overreach, the headline-outrunning-the-ledger pattern in class-constant clothing. The ionic class SPLITS: a RIGID-ION subclass (halides, Cauchy-passing, rung 1 in-band) and a BREATHING subclass (`EA2 < 0` anions: oxides, sulfides, nitrides), with the boolean as dispatcher. One derived boolean, three consumers: the B bias flag, the Cauchy prediction, the shear-model dispatch.

## The quantitative gift: the bias is a class constant, not noise
CaO by the rung-1 formula gives 187 GPa against measured ~113 (1.65x); MgO at n=7 gives 273 against 160-165 (1.65-1.7x); SrO and BaO land in the same band [verify Anderson compilations]. A bias that tight (1.6 to 1.75x across the whole alkaline-earth series) is a class constant with derived rationale (breathing), architecturally identical in standing to the Trouton 109 bin.

## The ruling (owner), and the manager's one deferred decision
1. Ship RUNG 1 exactly as B settled it: estimator with stated bias, NaCl in-band, periclase flagged. BUILT, confirmed unchanged.
2. RUNG 2 is Gordon-Kim / PIB, parameter-free, the principled middle tier (5-15% band). SPECIFIED.
3. RUNG 3 is the standing compute-once arbiter. SPECIFIED.
4. QEq stays demoted to disposer-coarse formation questions, with intrinsic over-ionization on raw parameters documented [M, B's finding].
5. The escalation rule is satisfied throughout: B is a POWER-LAW consumer, so estimator-plus-bias is legal for it in a way it never would be for anything feeding an exponent.
6. MANAGER DECISION (owner left it to me): the optional breathing-subclass bias factor `~0.60` [M class] is DEFERRED, not added now. Rationale: no current consumer reads the oxide modulus (dormant), the parameter-free rung 2 supersedes it, and even the cheap factor needs the `EA2` dispatcher floor addition. It is a READY cheap bridge if a consumer (R-COEVOLVE mantle elasticity) needs the accurate oxide modulus before rung 2 is built; until then the honest documented-bias estimator stands and the parameter-free rung 2 is the principled path.

## Built vs specified (honest status)
- BUILT: rung 1 (B's ionic bulk modulus, [E] with documented bias), the Ewald generator, the shielded QEq (demoted), the IE/EA first-affinity columns.
- SPECIFIED (not built): the `EA2 < 0` boolean floor datum (the first EA column exists; the second-affinity boolean does not), the Cauchy diagnostic and the Cij elastic tensor (the shear machinery is deferred, not built), Gordon-Kim/PIB rung 2, the breathing subclass dispatcher, the triptych shear terms, the escalation-rule gate (A's Phase-2 scope).

## Ledger delta
Layer 3's ionic-elasticity entry splits by the `EA2` boolean [D, from the banked EA column, once EA2 is added]; gains the Gordon-Kim/PIB tier [D-in-form, compute-once-lite, 5-15% band] and optionally the breathing-subclass bias factor [M class, deferred, manager's ruling]; the shear-turn "ionics: none" line is corrected; QEq's flag gains "intrinsic over-ionization on raw parameters" [M, B's finding]; layer 4 gains nothing (twentieth audit).

Verify Gordon & Kim 1972, Muhlhausen & Gordon 1981, Boyer et al. 1985, Mehl-Hemley-Boyer 1986, Isaak-Cohen-Mehl 1990, Wolf-Bukowinski 1988, plus measured Cij and K0 against Simmons & Wang and Anderson, and the Born-effective-charge phonon papers (Karki lineage).
