# The molecular-transport parameter floor: what to author, what to derive

This report investigates how the deterministic engine should supply the per-species parameters the
Chapman-Enskog transport derivations need (gas diffusivity, viscosity, thermal conductivity), and
recommends the cleanest floor under the three-way value test (a value is a universal fundamental constant, a
per-world datum, or derived-else-defect). The current plan is to author a Lennard-Jones collision diameter
(sigma) and well-depth (epsilon) per substance. The finding is that this authors a DERIVED intermediate, and
that a smaller, more fundamental, more available, more citable per-substance primitive exists: the critical
point (critical temperature T_c and critical pressure P_c), optionally with the acentric factor omega. From
these the Lennard-Jones pair derives by corresponding-states, and the reduction is grounded in the
Lennard-Jones potential's own critical point rather than in a fit to Earth fluids, so it is alien-clean by
construction.

## 1. What the transport derivations consume

Chapman-Enskog kinetic theory gives the dilute-gas transport coefficients as a leading factor set by the
molecule's mass, size, and temperature, divided by a dimensionless COLLISION INTEGRAL that carries all of
the intermolecular-potential detail. For the Lennard-Jones (12-6) potential the binary diffusion coefficient
is proportional to `sqrt(T^3/M) / (P * sigma^2 * Omega_D(T*))`, the viscosity to `sqrt(M*T) / (sigma^2 *
Omega_v(T*))`, and the thermal conductivity follows the viscosity through the same collision integral. The
two potential parameters enter in exactly two places: sigma sets the geometric collision cross-section
(`sigma^2`), and epsilon sets the REDUCED TEMPERATURE `T* = k_B * T / epsilon` that the collision integral is
a function of. The collision integral itself is not a per-species datum: it is a fixed mathematical property
of the Lennard-Jones potential, tabulated once and, for the engine, evaluated by the Neufeld, Janzen, and
Aziz (1972) closed-form correlation `Omega(T*) = A/(T*)^B + C/exp(D*T*) + E/exp(F*T*) + G/exp(H*T*)` with the
universal constants A = 1.06036, B = 0.15610, C = 0.19300, D = 0.47635, E = 1.03587, F = 1.52996, G =
1.76474, H = 3.89411 over `0.3 < T* < 100`. So the entire per-species residue the transport laws need is the
pair (sigma, epsilon), and the question is how to supply that pair without authoring it.

## 2. Where sigma and epsilon come from, and why authoring them directly is authoring a derived value

A Lennard-Jones parameter pair is not a directly measured quantity. In practice sigma and epsilon are OBTAINED
by fitting the Lennard-Jones model to some real dataset, most often second-virial-coefficient or viscosity
data. The standard tabulation engineers cite, Appendix B of Poling, Prausnitz, and O'Connell, "The Properties
of Gases and Liquids" (5th edition, 2001), is titled "Lennard-Jones Potentials as Determined from Viscosity
Data": the numbers are back-fit to the transport property they will then be used to predict. This has three
consequences that bear on the floor. First, using such a pair to predict viscosity is partly circular (the
pair was chosen to reproduce viscosity), and the pair carries the temperature range and the fitting
convention of whoever produced it. Second, the tabulation is small, on the order of a couple hundred common
gases, so most substances have no cited pair. Third, and decisive for this project, sigma and epsilon are a
DERIVED intermediate: they are a two-parameter summary of a substance's intermolecular potential, itself
inferred from more primitive observables. Authoring them puts a derived quantity in the floor, which the
derive-else-defect rule forbids when a more primitive cited observable exists. One does.

## 3. The corresponding-states route: critical properties as the primitive

The critical point is a real, directly observed phase-behaviour event: the temperature and pressure above
which liquid and gas become indistinguishable. T_c and P_c (and the critical volume V_c) are measured, not
fit to a transport property, and they are tabulated far more widely than Lennard-Jones pairs (Poling et al.
Appendix A carries constants for 600-plus compounds; the CRC Handbook, NIST, and DIPPR carry thousands).
Corresponding-states theory maps the critical point onto the Lennard-Jones pair. The physical content is
simple: epsilon is a molecular ENERGY scale, and the only molecular energy the critical point offers is
`k_B * T_c`, so `epsilon/k_B` is proportional to T_c; sigma is a molecular LENGTH scale, and the molecular
volume the critical point offers is `V_c` or, through the ideal-gas combination, `k_B*T_c/P_c`, so `sigma` is
proportional to `V_c^(1/3)` or `(T_c/P_c)^(1/3)`. The published correlations differ only in their fitted
proportionality constants and in whether they add an acentric-factor term for shape. The main ones, with the
formulas verbatim and their citations, are the reference material below.

| Method | epsilon/k_B | sigma | Inputs | Citation |
|---|---|---|---|---|
| Bird-Stewart-Lightfoot | `0.77 * T_c` | `0.841 * V_c^(1/3)` or `2.44*(T_c/P_c)^(1/3)` | T_c; V_c or (T_c,P_c) | Bird, Stewart, Lightfoot, "Transport Phenomena," 2nd ed. (2002/2006) |
| Flynn | `1.77 * T_c^(5/6)` | `0.561*(V_c^(1/3))^(5/4)` | T_c; V_c | Flynn, L.W., M.S. thesis, Northwestern Univ. (1960) |
| Stiel-Thodos | `65.3 * T_c * Z_c^3.6` | `0.1866 * V_c^(1/3) * Z_c^(-6/5)` | T_c, Z_c; V_c, Z_c | Stiel and Thodos, J. Chem. Eng. Data 7(2), 234 (1962) |
| Tee-Gotoh-Stewart 1 | `0.7740 * T_c` | `2.3647*(T_c/P_c)^(1/3)` | T_c; (T_c,P_c) | Tee, Gotoh, Stewart, Ind. Eng. Chem. Fundam. 5(3), 356 (1966) |
| Tee-Gotoh-Stewart 2 | `(0.7915 + 0.1693*omega) * T_c` | `(2.3551 - 0.0874*omega)*(T_c/P_c)^(1/3)` | T_c, P_c, omega | Tee, Gotoh, Stewart, Ind. Eng. Chem. Fundam. 5(3), 356 (1966) |
| Silva-Liu-Macedo | (paired with sigma below) | `sigma^3 = 0.17791 + 11.779*(T_c/P_c) - 0.049029*(T_c/P_c)^2` | T_c, P_c (bar) | Silva, Liu, Macedo, Chem. Eng. Sci. 53(13), 2423 (1998) |

In the T_c/P_c forms the pressure is in atmospheres unless a method's note says otherwise, and sigma comes out
in angstroms; the engine's fixed-point unit system handles the conversion. The acentric-factor forms
(Tee-Gotoh-Stewart 2) carry the third corresponding-states parameter, molecular shape, which measurably
improves the fit for elongated and slightly polar molecules.

## 4. The reduction is grounded in the potential itself, not in an Earth-fluid fit

The corresponding-states constants above were fit to real (mostly nonpolar, terrestrial) fluids, which raises
a fair alien-cleanness worry: are 0.7740 and 2.3647 Earth-specific numbers smuggled into the floor? They are
not, and the reason matters. The Lennard-Jones fluid has its OWN critical point, a pure mathematical property
of the potential fixed by simulation, independent of any real substance: in reduced units `T_c* = k_B*T_c /
epsilon = 1.3120(7)`, `P_c* = P_c * sigma^3 / epsilon = 0.1279(6)` (NIST Lennard-Jones benchmark; Potoff and
Panagiotopoulos, J. Chem. Phys. 109, 10914 (1998)). Inverting these two universal reduced constants gives the
mapping from a substance's measured critical point to its Lennard-Jones pair with no free parameters at all:
`epsilon/k_B = T_c / 1.312 = 0.762 * T_c`, and `sigma^3 = (P_c*/T_c*) * k_B*T_c/P_c = 0.0975 * k_B*T_c/P_c`.
The theoretical coefficient 0.762 sits within two percent of the Tee-Gotoh-Stewart 0.7740 and the
Bird-Stewart-Lightfoot 0.77, which confirms that those constants are a small empirical polish on a mapping the
potential fixes on its own. So the correlation constants are UNIVERSAL kernel constants (siblings of the
Neufeld collision-integral constants), the mechanism is fixed Rust, and the only per-substance data are the
measured critical properties. An alien gas is a data row: any substance with a critical point (any real gas
that can be liquefied) has a T_c and a P_c, and the same reduction carries it into a Lennard-Jones pair
without special-casing a chemistry.

## 5. Can one cited number replace the pair? The irreducible residue

The cleanest reduction is two dimensioned numbers, T_c and P_c, not one. sigma and epsilon carry independent
dimensions (a length and an energy), so no single number can fix both in general: a molecular energy scale and
a molecular size scale are separate facts. Corresponding-states theory does tighten this. The critical
compressibility `Z_c = P_c*V_c/(R*T_c)` is nearly the same (about 0.27 to 0.29) for simple nonpolar fluids, so
V_c is roughly `0.29*R*T_c/P_c` and the three critical constants are not independent; T_c and P_c alone then
fix both Lennard-Jones parameters (which is exactly the T_c/P_c form above). A truly one-number floor is
reachable only for a strict two-parameter corresponding-states fluid where a single reduced constant (say the
boiling point, or T_c with a universal Z_c assumed) stands in for the rest, at a real accuracy cost. The
honest floor is therefore two cited numbers per substance, with a third (the acentric factor omega) as an
optional accuracy improver, versus the two authored Lennard-Jones numbers of the current plan: the same
count, but measured observables in place of fitted derived quantities, and observables that ALSO feed the
equation of state, vapour pressure, and other floor derivations rather than serving transport alone.

## 6. Group contribution: the fallback when even critical properties are missing

When a substance has no measured critical point (a novel or an exotic molecule), T_c, P_c, and V_c are
themselves estimable from the molecular structure by group contribution: Joback (Joback and Reid, Chem. Eng.
Commun. 57, 233 (1987)) and the more accurate Constantinou-Gani (AIChE J. 40, 1697 (1994)) sum tabulated
group increments over the molecule's functional groups. There are also direct group-contribution estimates of
the Lennard-Jones parameters (for example the method surveyed in the `chemicals` library and in Poling et
al.), which skip the critical point. Group contribution is the ultimate alien-clean fallback in the sense that
it needs no experimental datum at all, only the molecule's composition, which the engine's own formula map
(the periodic-table molar-mass primitive) already expresses. Its cost is a second layer of approximation
stacked on the corresponding-states one, so it belongs as the fallback below the measured critical point, not
the primary path.

## 7. The polar and associating exception, kept as a data extension

The Lennard-Jones (12-6) potential is a spherical, nonpolar model. Strongly polar or hydrogen-bonding gases
(water, ammonia, the alcohols) are fit poorly by any single sigma-epsilon pair, and the corresponding-states
constants above degrade for them. The established remedy is the Stockmayer potential, a Lennard-Jones core
plus a point-dipole term, with a Brokaw (1969) correction to the collision integral that adds a term in the
reduced dipole moment `delta = 1.94e3 * mu^2 / (V_b * T_b)` (Brokaw, Ind. Eng. Chem. Process Des. Dev. 8, 240
(1969)). The dipole moment mu is itself a measured, citable, primitive molecular property (Nelson, Lide,
Maryott dipole-moment tables; CRC Handbook). This keeps the floor alien-clean: a polar substance is a data
row carrying one more cited number (its dipole moment), and the mechanism reads it exactly as a nonpolar
substance carries a zero dipole. Nothing about polarity forces a rewrite; it is a data-driven extension of the
same kernel.

## 8. Recommendation

Author the CRITICAL POINT per substance as the transport primitive: the critical temperature T_c and the
critical pressure P_c, each a measured observable with a citation and a basis, plus the acentric factor omega
where the substance's data support it (omega sharpens the derivation and is already needed by the equation of
state). Derive the Lennard-Jones pair inside a fixed-Rust kernel by the Tee-Gotoh-Stewart 2 relations (the
acentric-factor forms), whose constants are universal kernel constants grounded in the Lennard-Jones fluid's
own reduced critical point, and evaluate the Chapman-Enskog collision integral by the Neufeld correlation.
Provide two data-driven extensions so the floor admits the alien and the awkward: a per-substance dipole
moment feeding the Stockmayer-Brokaw polar correction (defaulting to zero for a nonpolar gas), and a group-
contribution fallback that estimates the critical properties from the molecule's composition when no measured
critical point exists. This route puts three real, widely tabulated, dual-use observables (T_c, P_c, omega) in
the floor instead of two fitted derived numbers, satisfies derive-else-defect (the Lennard-Jones pair becomes
a derived quantity, never authored), and keeps every gas, terrestrial or alien, a single data row.

The honest limits: the corresponding-states mapping carries a few percent error in sigma and epsilon for
well-behaved nonpolar fluids and more for polar or associating ones (hence the Stockmayer extension); the
Lennard-Jones (12-6) model itself is an approximation to the true intermolecular potential, so the transport
coefficients it yields are engineering-accurate (typically within five to ten percent of experiment for
diffusivity and viscosity), not exact; and the group-contribution fallback stacks a second approximation for
substances lacking a measured critical point. None of these is a determinism or an alien-feasibility hazard:
the kernels are closed-form fixed-point, and every per-substance input is a cited data row.

## 9. Reserved values and provenance discipline

No value is authored here. The per-substance primitives this route puts in the floor (T_c, P_c, omega, and the
optional dipole moment mu) are per-world data, each surfaced as reserved-with-basis and each carrying its
citation when a world declares a gas: the basis for T_c and P_c is the substance's measured critical point
(CRC Handbook of Chemistry and Physics; NIST Chemistry WebBook; Poling, Prausnitz, and O'Connell, "The
Properties of Gases and Liquids," 5th ed., Appendix A; DIPPR), the basis for omega is the same source's
tabulated acentric factor, and the basis for mu is a dipole-moment table (CRC; Lide). The correlation
constants (Tee-Gotoh-Stewart, Neufeld, Brokaw) are not per-world reserved values but universal constants of
the fixed-Rust kernel, grounded as shown in the Lennard-Jones potential's own critical point, and cited to
their source papers. Under the three-way test the classification is: the fundamental constants (k_B, N_A) are
universal; T_c, P_c, omega, and mu are per-world Mirror-calibrated data; sigma, epsilon, and the collision
integral are derived; and the correlation and collision-integral coefficients are universal kernel constants
of the derivation, not authored per-world numbers.

## Sources

- Tee, L. S., Gotoh, S., and Stewart, W. E. "Molecular Parameters for Normal Fluids: The Lennard-Jones 12-6 Potential." Industrial & Engineering Chemistry Fundamentals 5(3): 356-363 (1966).
- Bird, R. B., Stewart, W. E., and Lightfoot, E. N. "Transport Phenomena," 2nd edition. Wiley (2002/2006).
- Poling, B. E., Prausnitz, J. M., and O'Connell, J. P. "The Properties of Gases and Liquids," 5th edition. McGraw-Hill (2001). Appendix A (critical constants and acentric factors), Appendix B (Lennard-Jones potentials from viscosity data).
- Neufeld, P. D., Janzen, A. R., and Aziz, R. A. "Empirical Equations to Calculate 16 of the Transport Collision Integrals for the Lennard-Jones (12-6) Potential." Journal of Chemical Physics 57(3): 1100-1102 (1972).
- Flynn, L. W. M.S. thesis, Northwestern University, Evanston, Illinois (1960).
- Stiel, L. I., and Thodos, G. "Lennard-Jones Force Constants Predicted from Critical Properties." Journal of Chemical & Engineering Data 7(2): 234-236 (1962).
- Silva, C. M., Liu, H., and Macedo, E. A. "Models for Self-Diffusion Coefficients of Dense Fluids, Including Hydrogen-Bonding Substances." Chemical Engineering Science 53(13): 2423-2429 (1998).
- Potoff, J. J., and Panagiotopoulos, A. Z. "Critical Point and Phase Behavior of the Pure Fluid and a Lennard-Jones Mixture." Journal of Chemical Physics 109: 10914 (1998); NIST Lennard-Jones Fluid Benchmarks (T_c* = 1.3120, P_c* = 0.1279).
- Brokaw, R. S. "Predicting Transport Properties of Dilute Gases." Industrial & Engineering Chemistry Process Design and Development 8(2): 240-253 (1969).
- Joback, K. G., and Reid, R. C. "Estimation of Pure-Component Properties from Group-Contributions." Chemical Engineering Communications 57: 233-243 (1987).
- Constantinou, L., and Gani, R. "New Group Contribution Method for Estimating Properties of Pure Compounds." AIChE Journal 40(10): 1697-1710 (1994).
- CRC Handbook of Chemistry and Physics; NIST Chemistry WebBook (critical constants, acentric factors, dipole moments).
