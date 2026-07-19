// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! THE STATE-RESOLVED THERMOELASTIC LADDER: one query, four rungs, and a refusal.
//!
//! # Why this exists
//!
//! An interior column needs density, expansivity, bulk modulus and heat capacity AT the pressure and
//! temperature it actually sits at. What the repository holds is AMBIENT data: Grueneisen rows near 300 K
//! and 1 bar, adiabatic bulk moduli at about 298 K, standard-state molar volumes. Reading those at 1600 K
//! and 100 kbar produced a number that agreed with measurement at one temperature by CANCELLATION, a
//! high-temperature Dulong-Petit capacity against a 300 K gamma, modulus and volume, rather than by
//! physics. The audit that caught it is recorded on `civsim_materials::properties`, and the fixture-cluster
//! derivation now refuses outside its input rows' declared frame rather than returning that number.
//!
//! Refusing is correct and it is not the destination. This module is the destination: a ladder whose rungs
//! all answer the SAME state query, so a caller asks "what is this phase's response bundle at (P, T)?" and
//! gets either an answer with its rung and band, or a refusal naming what would answer it.
//!
//! # The rungs, strongest first
//!
//! 1. **Measured P-V-T surface.** A cited equation of state or tabulated volume surface supplies `V(P,T)`
//!    inside its domain, and alpha, density and `K_T` derive from that surface.
//! 2. **Compute-once free-energy surface.** A quasi-harmonic `F(V,T)` calculation, cached per phase and
//!    state bucket. This is the ALIEN-ADMITTING rung: a generated crystalline phase with no laboratory
//!    surface still gets an answer, carrying the method's approximation band.
//! 3. **Mie-Grueneisen-Debye estimator.** A reduced thermal equation of state over per-phase anchors.
//! 4. **Ambient measured response.** An ambient row, valid only INSIDE its measured frame. It is also an
//!    independent validation anchor for the deeper rungs.
//!
//! No rung: refusal. A phase with no state-local rung refuses, and no phase receives an Earth-mineral
//! default. That last sentence is the whole point: a default here would be an authored interior.
//!
//! # What is built today, stated plainly
//!
//! ALL SIX RUNG-3 ANCHORS ARE NOW BANKED: molar volume (phase registry), bulk modulus (mineral moduli),
//! `K'` and `gamma_0` (Grueneisen table), and the effective Debye temperature `theta_0` and volume
//! exponent `q` (`thermoelastic_anchors.toml`), and RUNG 3 ITSELF IS BUILT in
//! [`crate::mie_gruneisen_debye`]. What remains is wiring it into this ladder's dispatch, so a query
//! above the ambient frame reaches the solver instead of the refusal it still returns today.
//!
//! Two of those anchors arrived by routes worth distinguishing, because counting them all as fetches
//! overprices the work by a large factor:
//!
//! - `K'` was never a fetch. It had been banked in `gruneisen.toml` from the start, with its band and its
//!   adiabatic-versus-isothermal type, and the loader did not read it. A loader change.
//! - `theta_0` was never a fetch either, though for a different reason: its values were already inside the
//!   held extracts in `thermoelastic_anchors/manifest.toml`, which quote the full source table rows
//!   including the `theta0` column. A transcription from a witness already in the repository.
//!
//! # The correction this module carries, recorded because it was mine
//!
//! An earlier pass concluded that `theta_0` needed no column at all, because the Debye temperature
//! DERIVES from the banked moduli. That derivation is real and it is still here. The conclusion drawn from
//! it was wrong, and wrong in an instructive way: the derived quantity is the ELASTIC Debye temperature,
//! set by the three acoustic branches near `k = 0`, and the MGD equation of state consumes the EFFECTIVE
//! one, fit by its source to the vibrational entropy near 1000 K over the whole density of states. For a
//! 7-atom orthosilicate that is 3 modes against 21.
//!
//! The check that licensed the error was a single forsterite spot-check, and forsterite sits almost
//! exactly on the crossover where the two definitions coincide (762 K elastic against 809 K effective).
//! Across the seven phases the ratio runs 0.83 to 1.22 and does not cancel. Periclase alone would have
//! been 167 K out.
//!
//! The two are now separated BY TYPE, [`ElasticDebyeTemperature`] here against
//! `civsim_physics::thermoelastic_anchors::EffectiveDebyeTemperature` there, with no conversion in either
//! direction and no public constructor from a bare `Fixed` on either side. The mispairing is
//! unconstructible rather than warned against, because a defence carried in a comment is one that gets
//! dropped.
//!
//! Rungs 1 and 2 are further out: rung 1 wants a cited P-V-T surface per phase (forsterite has a lead to
//! about 14 GPa and 1900 K, which is one phase rather than a mechanism), and rung 2 wants a quasi-harmonic
//! calculation and its cache.

use civsim_core::Fixed;
use civsim_physics::gruneisen::GruneisenTable;
use civsim_physics::mineral_moduli::MineralModuli;
use civsim_physics::petrology_data::PhaseRegistry;
use civsim_physics::thermoelastic_anchors::ThermoelasticAnchors;

/// The zero this module compares against, matching the sibling modules' idiom.
const ZERO: Fixed = Fixed::ZERO;

/// The state a caller is asking about. Both fields are REQUIRED and neither has a default, because a
/// thermoelastic property with no state is the defect this ladder exists to end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermoState {
    /// Temperature (K).
    pub temperature_k: Fixed,
    /// Pressure (bar). Bar rather than GPa because the phase registry and the moduli rows state bar.
    pub pressure_bar: Fixed,
}

/// Which rung answered, carried WITH the answer so a consumer can never mistake an ambient row for a
/// state-resolved one. This is the field whose absence caused the original defect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermoRung {
    /// A cited pressure-volume-temperature surface, inside its domain.
    MeasuredSurface,
    /// A compute-once quasi-harmonic free-energy surface.
    FreeEnergySurface,
    /// A Mie-Grueneisen-Debye reduced equation of state.
    MieGruneisenDebye,
    /// An ambient measured row, valid only inside its own measured frame.
    AmbientMeasured,
}

/// One phase's thermoelastic response at a state, with the rung that produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThermoResponse {
    /// Volumetric thermal expansivity (per kelvin).
    pub alpha_per_k: Fixed,
    /// Isothermal bulk modulus (GPa).
    pub bulk_modulus_gpa: Fixed,
    /// Molar volume (cm^3/mol) at the requested state.
    pub molar_volume_cm3: Fixed,
    /// Which rung answered.
    pub rung: ThermoRung,
    /// The frame this answer is valid in, carried so a consumer cannot silently reuse it elsewhere.
    pub valid_at: ThermoState,
}

/// Why no rung could answer. Each variant names the rung it is about and what would close it, so the
/// refusal is a work list rather than a dead end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThermoRefusal {
    /// The phase is not in the registry at all.
    UnknownPhase {
        /// The phase asked for.
        phase: String,
    },
    /// A rung exists in principle and its inputs are not banked. Names which rung and which inputs.
    RungUnavailable {
        /// The phase asked for.
        phase: String,
        /// The rung that would have answered.
        rung: ThermoRung,
        /// The inputs that rung needs and the repository does not hold.
        missing: Vec<String>,
    },
    /// Every rung declined, and the requested state is outside the only frame that could answer.
    OutsideEveryFrame {
        /// The phase asked for.
        phase: String,
        /// What was asked for.
        requested: ThermoState,
        /// The frame the strongest available rung is valid in.
        available_frame: ThermoState,
    },
}

impl core::fmt::Display for ThermoRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ThermoRefusal::UnknownPhase { phase } => {
                write!(f, "phase {phase} is not in the registry")
            }
            ThermoRefusal::RungUnavailable {
                phase,
                rung,
                missing,
            } => write!(
                f,
                "phase {phase} has no state-resolved answer: the {rung:?} rung would supply one and the \
                 repository does not bank {}. Refused rather than defaulted, because a default here would \
                 be an authored planetary interior.",
                missing.join(", ")
            ),
            ThermoRefusal::OutsideEveryFrame {
                phase,
                requested,
                available_frame,
            } => write!(
                f,
                "phase {phase} was asked at {} K and {} bar, and the only rung that can answer is valid at \
                 {} K and {} bar. Reading an ambient row at interior conditions is what this ladder exists \
                 to stop: it produced a number that matched measurement by cancellation rather than physics.",
                requested.temperature_k.to_f64_lossy(),
                requested.pressure_bar.to_f64_lossy(),
                available_frame.temperature_k.to_f64_lossy(),
                available_frame.pressure_bar.to_f64_lossy()
            ),
        }
    }
}

impl std::error::Error for ThermoRefusal {}

/// A phase's volume PER ATOM in cubic angstroms, from the registry's own molar volume and formula unit.
///
/// ONE home for this conversion, because two consumers need it and two copies of a unit bridge is how a
/// factor of `1e24` quietly diverges. The elastic Debye temperature reads it for the `(3n/4 pi V)^(1/3)`
/// wavevector, and the Slack lattice-conductivity estimator reads it for the interatomic spacing
/// `delta = cbrt(V_atom)`. `1 cm^3/mol` over Avogadro is `1e24 / 6.02214076e23 = 1.66053906717` cubic
/// angstroms per particle.
///
/// The atom count is the FORMULA UNIT's, matching the registry molar volume's own basis. Mixing it with an
/// atoms-per-primitive-cell count would be a basis error wearing a derivation's clothes, and that column
/// sits one table over.
// @derives: a phase's volume per atom <- its registry molar volume and formula-unit atom count
pub fn atomic_volume_angstrom3(phase: &str, registry: &PhaseRegistry) -> Option<Fixed> {
    let row = registry.phase(phase)?;
    if row.molar_volume <= ZERO {
        return None;
    }
    let atoms: u32 = row.composition.iter().map(|(_, c)| *c).sum();
    if atoms == 0 {
        return None;
    }
    let per_atom_cm3_mol = row
        .molar_volume
        .checked_div(Fixed::from_int(atoms as i32))?;
    let angstrom3_per_cm3_mol = Fixed::from_decimal_str("1.66053906717").ok()?;
    per_atom_cm3_mol.checked_mul(angstrom3_per_cm3_mol)
}

/// A phase's ELASTIC Debye temperature (K), DERIVED from banked elasticity rather than read from a column.
///
/// Private field and no public constructor from a bare `Fixed`, so this cannot be built anywhere except
/// the derivation below, and no route exists between it and
/// `civsim_physics::thermoelastic_anchors::EffectiveDebyeTemperature`. See the module documentation for
/// why the two must not meet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ElasticDebyeTemperature(Fixed);

impl ElasticDebyeTemperature {
    /// The value in kelvin. One-way, like its counterpart: a `Fixed` comes out for arithmetic and none
    /// goes back in.
    pub fn kelvin(self) -> Fixed {
        self.0
    }
}

impl core::fmt::Display for ElasticDebyeTemperature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} K (elastic, acoustic-branch)", self.0.to_f64_lossy())
    }
}

/// A phase's ELASTIC Debye temperature, from the mean sound velocity and the atomic volume.
///
/// The sound velocity follows from the banked bulk and shear moduli and the density, the density is the
/// registry's own molar mass over its own molar volume (so this uses the SAME density the rest of the
/// cluster derives rather than a second opinion about it), and the atomic volume is the molar volume
/// shared over the formula unit's atoms.
///
/// # What this is, and the one thing it is not
///
/// It is the acoustic average: set by the three branches near `k = 0`, asymptotically correct as
/// `T -> 0`, and the right quantity for a low-temperature `C_V = beta T^3` coefficient or an acoustic
/// density of states, PROVIDED it is paired with its own acoustic `gamma_el = 1/3 - dln(v_m)/dln(V)`.
///
/// It is NOT the effective Debye temperature an MGD equation of state consumes, and the return type says
/// so. That distinction cost a wrong conclusion in this repository on 2026-07-19: this derivation was
/// checked against forsterite's measured 760 K, agreed, and was taken as evidence that no `theta_0`
/// column was needed. Forsterite is within a few percent of the crossover between the two definitions.
/// Periclase, the phase whose simplicity made it look like the safer check, is 167 K apart (940 elastic
/// against 773 effective).
///
/// # Coverage
///
/// Seven of the eight registry phases. Hematite is not one of them: its bulk and shear moduli are
/// recorded `UNSOURCED` in `mineral_moduli.toml`, so there is nothing to derive from. An earlier version
/// of this comment claimed the moduli file "carries K AND G for all eight phases"; it does not, and the
/// claim was never checked before it was written.
///
/// `None` when the phase lacks a moduli row or a registry row, or when an intermediate leaves the
/// representable window. No phase receives a default.
// @derives: a phase's elastic Debye temperature <- its banked bulk and shear moduli, density and atomic volume
pub fn derived_elastic_debye_temperature(
    phase: &str,
    registry: &PhaseRegistry,
    moduli: &MineralModuli,
    periodic: &civsim_physics::periodic::PeriodicTable,
) -> Option<ElasticDebyeTemperature> {
    let row = registry.phase(phase)?;
    let m = moduli.row(civsim_physics::mineral_moduli::canonical_phase_key(phase))?;
    // Density in g/cm^3: the registry's own molar mass over its own molar volume, so the density this
    // uses is the same one the rest of the cluster derives rather than a second opinion about it.
    let molar_mass = civsim_physics::petrology::phase_molar_mass(row, periodic)?;
    if row.molar_volume <= ZERO {
        return None;
    }
    let density_g_cm3 = molar_mass.checked_div(row.molar_volume)?;
    let v_d = crate::properties::debye_velocity_km_per_s(m.bulk_gpa, m.shear_gpa, density_g_cm3);
    if v_d <= ZERO {
        return None;
    }
    // Atomic volume in cubic angstroms: the molar volume shared over the formula unit's atoms, converted
    // from cm^3/mol. 1 cm^3/mol over Avogadro is 1e24/6.02214076e23 cubic angstroms per particle.
    let atomic_volume_a3 = atomic_volume_angstrom3(phase, registry)?;
    let theta = crate::properties::debye_temperature(v_d, atomic_volume_a3);
    if theta <= ZERO {
        None
    } else {
        Some(ElasticDebyeTemperature(theta))
    }
}

/// The anchors the Mie-Grueneisen-Debye rung needs, and whether the repository banks each.
///
/// Reported as DATA rather than described in prose, so "rung 3 is blocked" is a machine-checkable claim
/// and closing it is a visible state change rather than an edit to a comment.
pub fn mie_gruneisen_debye_readiness(
    phase: &str,
    registry: &PhaseRegistry,
    moduli: &MineralModuli,
    gruneisen: &GruneisenTable,
    anchors: &ThermoelasticAnchors,
) -> Vec<(&'static str, bool)> {
    let has_volume = registry.phase(phase).is_some();
    let key = civsim_physics::mineral_moduli::canonical_phase_key(phase);
    let has_k0 = moduli.row(key).is_some();
    let row = gruneisen.row(phase);
    let has_gamma = row.and_then(|r| r.gamma()).is_some();
    // K' WAS BANKED AND UNREAD, and that gap is now closed. The data file carried
    // `bulk_modulus_pressure_derivative_kprime` for every row along with its band and its `kprime_type`
    // (adiabatic `K_S'` versus isothermal `K_T'`), and the loader simply did not read it. Distinguishing
    // that from a genuine fetch mattered: it was being counted as one of rung 3's missing anchors when it
    // was a loader change, and conflating the two would have mispriced the work by a fetch.
    let has_kprime = row
        .and_then(|r| r.bulk_modulus_pressure_derivative_kprime)
        .is_some();

    // THETA_0 AND q COME FROM THE COLUMN, and each must be FIT rather than estimated from systematics.
    //
    // This slot previously reported true whenever the phase had the elasticity to DERIVE a Debye
    // temperature from, which was the wrong quantity: the derivation yields the elastic average and the
    // MGD form consumes the entropy-fit effective one. A readiness report that answers with the wrong
    // quantity is worse than one that answers "missing", because it licenses the solver to run.
    //
    // `usable_as_anchor` is what makes quartz report NOT ready on `q`: its cell is italic in the source
    // table, meaning the authors estimated it rather than fitting it, and an assumed exponent entering
    // under a measured grade is exactly what the font-metadata read exists to stop.
    let arow = anchors.row(phase);
    let has_theta = arow
        .map(|r| {
            r.theta_0.is_some()
                && r.theta_0_grade
                    .map(|g| g.usable_as_anchor())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    let has_q = arow
        .map(|r| r.q.is_some() && r.q_grade.map(|g| g.usable_as_anchor()).unwrap_or(false))
        .unwrap_or(false);
    // The whole set must come from ONE inversion. A row whose own fit does not reproduce the banked
    // gamma_0 was never jointly constrained with it, so the parameters are not a set even when every
    // individual cell is present and fit.
    let one_fit = arow.map(|r| r.pairs_with_banked_gamma()).unwrap_or(false);

    vec![
        ("molar_volume_V0", has_volume),
        ("bulk_modulus_K0", has_k0),
        ("kprime_K0_prime", has_kprime),
        ("gruneisen_gamma_0", has_gamma),
        ("debye_temperature_theta_0", has_theta),
        ("volume_exponent_q", has_q),
        ("anchors_from_one_joint_fit", one_fit),
    ]
}

/// The state query. Ask for a phase's response at a state; get an answer with its rung, or a refusal.
///
/// The dispatch runs strongest rung first and falls through on unavailability, which is what makes adding
/// a rung a pure addition: land the Debye and `q` columns and rung 3 starts answering above the ambient
/// frame with no consumer change.
// @derives: a phase's thermoelastic response at a state <- the strongest available rung over the banked per-phase anchors
pub fn response_at(
    phase: &str,
    state: ThermoState,
    registry: &PhaseRegistry,
    moduli: &MineralModuli,
    gruneisen: &GruneisenTable,
    anchors: &ThermoelasticAnchors,
) -> Result<ThermoResponse, ThermoRefusal> {
    let row = registry
        .phase(phase)
        .ok_or_else(|| ThermoRefusal::UnknownPhase {
            phase: phase.to_string(),
        })?;

    // RUNGS 1 AND 2 are not built. They are absent rather than stubbed, because a stub that returns a
    // plausible value is the defect, and a stub that returns an error is what the fall-through already is.

    // RUNG 3: Mie-Grueneisen-Debye. Reports its own readiness rather than being described as blocked.
    let readiness = mie_gruneisen_debye_readiness(phase, registry, moduli, gruneisen, anchors);
    let missing: Vec<String> = readiness
        .iter()
        .filter(|(_, have)| !have)
        .map(|(name, _)| (*name).to_string())
        .collect();

    // RUNG 4: the ambient measured row, valid ONLY inside its own measured frame. The Grueneisen row
    // carries that frame explicitly, which is what lets this refuse honestly instead of extrapolating.
    let gr = gruneisen.row(phase);
    if let Some(gr) = gr {
        let frame = ThermoState {
            temperature_k: gr.temperature_k,
            pressure_bar: gr.pressure_bar,
        };
        let t_off = abs_diff(state.temperature_k, frame.temperature_k);
        let p_off = abs_diff(state.pressure_bar, frame.pressure_bar);
        // The frame's own slack: a query at the row's stated conditions is inside it, a mantle query is not.
        let inside = t_off <= Fixed::from_int(25) && p_off <= Fixed::from_int(1000);
        if inside {
            if let (Some((gamma, _)), Some(mrow)) = (gr.gamma(), moduli.row(key_of(phase))) {
                let alpha = ambient_alpha(gamma, row, mrow.bulk_gpa)?;
                return Ok(ThermoResponse {
                    alpha_per_k: alpha,
                    bulk_modulus_gpa: mrow.bulk_gpa,
                    molar_volume_cm3: row.molar_volume,
                    rung: ThermoRung::AmbientMeasured,
                    valid_at: frame,
                });
            }
        } else if missing.is_empty() {
            // RUNG 3 ANSWERS HERE. This branch was a deliberate tripwire while the solver did not exist:
            // it refused by naming the solver, so landing the anchor columns failed a test rather than
            // succeeding silently. It fired, and this is what replaced it.
            //
            // The pressure crosses from bar to GPa on the way in: the ladder's state is stated in bar
            // because the registry and moduli rows are, and the equation of state works in GPa.
            if let Some(a) =
                crate::mie_gruneisen_debye::MgdAnchors::from_banked(phase, gruneisen, anchors)
            {
                let p_gpa = state
                    .pressure_bar
                    .checked_div(Fixed::from_int(10_000))
                    .ok_or_else(|| ThermoRefusal::UnknownPhase {
                        phase: phase.to_string(),
                    })?;
                match crate::mie_gruneisen_debye::response_at(&a, p_gpa, state.temperature_k) {
                    Ok(r) => {
                        return Ok(ThermoResponse {
                            alpha_per_k: r.alpha_per_k,
                            bulk_modulus_gpa: r.bulk_modulus_gpa,
                            molar_volume_cm3: r.molar_volume_cm3,
                            rung: ThermoRung::MieGruneisenDebye,
                            // Valid AT THE REQUESTED STATE, which is the whole point of the rung: unlike
                            // the ambient row below, it does not carry someone else's frame.
                            valid_at: state,
                        });
                    }
                    Err(why) => {
                        // The solver refused for a stated physical reason (past the spinodal, or an
                        // unrepresentable intermediate). It is carried through rather than flattened into
                        // "no rung answered", so a caller can tell a phase with no stable state at these
                        // conditions from one the repository cannot describe.
                        return Err(ThermoRefusal::RungUnavailable {
                            phase: phase.to_string(),
                            rung: ThermoRung::MieGruneisenDebye,
                            missing: vec![format!("{why:?}")],
                        });
                    }
                }
            }
            return Err(ThermoRefusal::RungUnavailable {
                phase: phase.to_string(),
                rung: ThermoRung::MieGruneisenDebye,
                missing: vec![
                    "a coherent single-fit anchor set: every cell fit rather than \
                     estimated, and the row's own gamma_0 reproducing the banked one"
                        .to_string(),
                ],
            });
        } else {
            return Err(ThermoRefusal::RungUnavailable {
                phase: phase.to_string(),
                rung: ThermoRung::MieGruneisenDebye,
                missing,
            });
        }
        return Err(ThermoRefusal::OutsideEveryFrame {
            phase: phase.to_string(),
            requested: state,
            available_frame: frame,
        });
    }

    Err(ThermoRefusal::RungUnavailable {
        phase: phase.to_string(),
        rung: ThermoRung::AmbientMeasured,
        missing: vec!["a Grueneisen row for this phase".to_string()],
    })
}

fn key_of(phase: &str) -> &str {
    civsim_physics::mineral_moduli::canonical_phase_key(phase)
}

fn abs_diff(a: Fixed, b: Fixed) -> Fixed {
    if a > b {
        a - b
    } else {
        b - a
    }
}

/// The ambient expansivity from the Grueneisen identity, at the rows' own frame and nowhere else.
// @derives: a phase's ambient volumetric expansivity <- its banked gamma, bulk modulus, molar volume and Dulong-Petit capacity
fn ambient_alpha(
    gamma: Fixed,
    phase: &civsim_physics::petrology_data::Phase,
    bulk_gpa: Fixed,
) -> Result<Fixed, ThermoRefusal> {
    let unrepresentable = || ThermoRefusal::UnknownPhase {
        phase: phase.name.clone(),
    };
    // Dulong-Petit per mole of FORMULA UNITS, matching the registry molar volume's basis. Mixing that
    // with an atoms-per-primitive-cell count would be a basis error wearing a derivation's clothes, and
    // that column sits one table over.
    let atoms: u32 = phase.composition.iter().map(|(_, c)| *c).sum();
    let r =
        civsim_physics::gas_thermochemistry::molar_gas_constant().ok_or_else(unrepresentable)?;
    let cv = Fixed::from_int(3)
        .checked_mul(Fixed::from_int(atoms as i32))
        .and_then(|x| x.checked_mul(r))
        .ok_or_else(unrepresentable)?;
    let denom = Fixed::from_int(1000)
        .checked_mul(bulk_gpa)
        .and_then(|x| x.checked_mul(phase.molar_volume))
        .ok_or_else(unrepresentable)?;
    if denom <= Fixed::ZERO {
        return Err(unrepresentable());
    }
    gamma
        .checked_mul(cv)
        .and_then(|x| x.checked_div(denom))
        .ok_or_else(unrepresentable)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables() -> (
        PhaseRegistry,
        MineralModuli,
        GruneisenTable,
        ThermoelasticAnchors,
    ) {
        (
            PhaseRegistry::standard().expect("registry"),
            MineralModuli::standard().expect("moduli"),
            GruneisenTable::standard().expect("gruneisen"),
            ThermoelasticAnchors::standard().expect("anchors"),
        )
    }

    /// THE LADDER ANSWERS ON THE RIGHT RUNG FOR THE STATE, which is the whole contract: the ambient row
    /// inside its own measured frame, and the equation of state at depth. Neither is ever read as the
    /// other, and every response carries the rung that produced it.
    #[test]
    fn the_ladder_answers_in_frame_on_rung_four_and_at_depth_on_rung_three() {
        let (reg, mod_, gr, anc) = tables();
        let ambient = ThermoState {
            temperature_k: Fixed::from_int(300),
            pressure_bar: Fixed::ONE,
        };
        let r = response_at("forsterite", ambient, &reg, &mod_, &gr, &anc)
            .expect("forsterite has an ambient row and answers at its own frame");
        assert_eq!(
            r.rung,
            ThermoRung::AmbientMeasured,
            "and it SAYS which rung"
        );
        let ppm = r.alpha_per_k.to_f64_lossy() * 1e6;
        assert!(
            (20.0..=45.0).contains(&ppm),
            "an ambient forsterite expansivity, read {ppm:.1} ppm/K"
        );

        // THE MANTLE QUERY THAT STARTED ALL OF THIS, and it now gets a real answer.
        //
        // This module exists because reading an ambient row at 1600 K and 100 kbar returned a number that
        // matched measurement by CANCELLATION (a high-temperature Dulong-Petit capacity against a 300 K
        // gamma, modulus and volume). The ladder's first version refused instead, which was correct and
        // was not the destination. Rung 3 is the destination: it solves the equation of state AT the
        // requested state, and the response says which rung produced it and that it is valid THERE.
        let interior = ThermoState {
            temperature_k: Fixed::from_int(1600),
            pressure_bar: Fixed::from_int(100_000),
        };
        let deep = response_at("forsterite", interior, &reg, &mod_, &gr, &anc)
            .expect("rung 3 answers at interior conditions");
        assert_eq!(
            deep.rung,
            ThermoRung::MieGruneisenDebye,
            "and it SAYS it was the equation of state, not an ambient row reused out of frame"
        );
        assert_eq!(
            deep.valid_at, interior,
            "and the frame it reports is the REQUESTED one, unlike rung 4 which reports its row's frame"
        );

        // The three magnitudes, each checked for direction as well as value, because a sign error here
        // would still land inside a loose band.
        let v = deep.molar_volume_cm3.to_f64_lossy();
        assert!(
            (40.0..=43.0).contains(&v),
            "at 10 GPa compression beats 1300 K of heating, so V falls below the 43.60 reference; \
             read {v:.2}"
        );
        let k = deep.bulk_modulus_gpa.to_f64_lossy();
        assert!(
            (130.0..=165.0).contains(&k),
            "and K_T RISES above the 128 GPa reference: 10 GPa at K' ~ 4.2 adds far more than 1300 K \
             removes; read {k:.1}"
        );
        let ppm = deep.alpha_per_k.to_f64_lossy() * 1e6;
        assert!(
            (18.0..=34.0).contains(&ppm),
            "and expansivity FALLS under compression to roughly 26 ppm/K, well below the ~40 ppm/K it \
             shows at 1000 K and ambient pressure; read {ppm:.1}"
        );
    }

    /// THE ELASTIC DEBYE TEMPERATURE DERIVES, checked at TWO magnitudes rather than one.
    ///
    /// The single-point version of this test is what licensed a wrong conclusion on 2026-07-19. It
    /// checked forsterite, got 762 K against a measured 760 K, and was read as proof that the derivation
    /// could stand in for the MGD `theta_0`. Forsterite is within a few percent of the crossover between
    /// the two definitions, so that agreement was the least informative point in the set.
    ///
    /// Periclase is the second magnitude, and it is the one that separates the definitions: a simpler,
    /// stiffer, lighter-per-atom structure whose measured ELASTIC Debye temperature is about 940 K while
    /// its entropy-fit EFFECTIVE one is 773 K. A derivation returning either a structure-independent
    /// value or the effective average would fail here while passing forsterite.
    ///
    /// The earlier version of this test asserted only that periclase came out HIGHER than forsterite, an
    /// ordering rather than a magnitude, and it did so inside an `if let Some(..)` so it also passed
    /// silently when the derivation returned nothing. Both are fixed: the value is bounded, and its
    /// absence is a failure.
    #[test]
    fn the_elastic_debye_temperature_derives_and_matches_measurement_at_two_magnitudes() {
        use civsim_physics::periodic::PeriodicTable;
        let (reg, mod_, _gr, _anc) = tables();
        let periodic = PeriodicTable::standard().expect("periodic table");

        let theta = derived_elastic_debye_temperature("forsterite", &reg, &mod_, &periodic)
            .expect("forsterite has the elasticity to derive a Debye temperature from")
            .kelvin()
            .to_f64_lossy();
        assert!(
            (680.0..=840.0).contains(&theta),
            "forsterite's measured ELASTIC Debye temperature is about 760 K; derived {theta:.0} K"
        );

        let peri = derived_elastic_debye_temperature("periclase", &reg, &mod_, &periodic)
            .expect("periclase has moduli, so its absence would itself be the finding")
            .kelvin()
            .to_f64_lossy();
        assert!(
            (860.0..=1020.0).contains(&peri),
            "periclase's measured ELASTIC Debye temperature is about 940 K; derived {peri:.0} K. Note \
             this is well clear of its EFFECTIVE 773 K, which is the whole reason the two are separate \
             types"
        );

        // A phase with no moduli row cannot derive one, and says so rather than defaulting.
        assert!(
            derived_elastic_debye_temperature("unobtainium", &reg, &mod_, &periodic).is_none(),
            "an unknown phase refuses rather than inheriting an Earth mineral's Debye temperature"
        );
        // HEMATITE is the coverage limit, and it is a real absence rather than a hypothetical: its bulk
        // and shear moduli are UNSOURCED, so seven of eight phases derive, not eight.
        assert!(
            derived_elastic_debye_temperature("hematite", &reg, &mod_, &periodic).is_none(),
            "hematite's moduli are UNSOURCED in mineral_moduli.toml, so nothing can be derived for it. \
             An earlier comment claimed the moduli file covers all eight phases; it does not."
        );
    }

    /// THE TWO DEBYE TEMPERATURES ARE DIFFERENT QUANTITIES, and this is the measurement that says so.
    ///
    /// This test exists because the claim "they are the same fact in two places" was made in this
    /// repository and acted on. The ratio is reported per phase so the spread is visible: if these were
    /// two copies of one fact the column would be flat at 1.0, and it is not.
    #[test]
    fn the_elastic_and_effective_debye_temperatures_are_not_two_copies_of_one_fact() {
        use civsim_physics::periodic::PeriodicTable;
        let (reg, mod_, _gr, anc) = tables();
        let periodic = PeriodicTable::standard().expect("periodic table");

        let mut ratios: Vec<(String, f64)> = Vec::new();
        for phase in [
            "periclase",
            "corundum",
            "spinel",
            "forsterite",
            "fayalite",
            "enstatite",
        ] {
            let el = derived_elastic_debye_temperature(phase, &reg, &mod_, &periodic)
                .expect("phase derives an elastic theta")
                .kelvin()
                .to_f64_lossy();
            let eff = anc
                .row(phase)
                .and_then(|r| r.theta_0)
                .expect("phase has a transcribed effective theta")
                .kelvin()
                .to_f64_lossy();
            ratios.push((phase.to_string(), el / eff));
        }

        let worst = ratios
            .iter()
            .map(|(_, r)| (r - 1.0).abs())
            .fold(0.0_f64, f64::max);
        assert!(
            worst > 0.15,
            "the two averages must be measurably different somewhere, or the type split would be \
             ceremony. Ratios: {ratios:?}"
        );

        // Periclase is the sharpest separation and the one a forsterite-only check misses entirely.
        let (_, peri) = ratios
            .iter()
            .find(|(p, _)| p == "periclase")
            .expect("periclase");
        assert!(
            *peri > 1.15,
            "periclase's elastic average runs well ABOVE its effective one (940 K against 773 K), \
             read {peri:.3}"
        );
        // Forsterite is the crossover, which is exactly why a single forsterite spot-check proved nothing.
        let (_, fo) = ratios
            .iter()
            .find(|(p, _)| p == "forsterite")
            .expect("forsterite");
        assert!(
            (fo - 1.0).abs() < 0.10,
            "forsterite sits near the crossover, read {fo:.3}: this is the agreement that misled, and it \
             is asserted so the reason the earlier check passed stays on the record"
        );
    }

    /// The readiness report is DATA, so "rung 3 is blocked" is checkable rather than asserted in prose.
    ///
    /// It reports READY for forsterite: all six anchors banked and all from one joint fit. The solver
    /// that consumes them is built and wired, so this is now the report that licenses rung 3 to run.
    #[test]
    fn rung_three_reports_every_anchor_banked() {
        let (reg, mod_, gr, anc) = tables();
        let readiness = mie_gruneisen_debye_readiness("forsterite", &reg, &mod_, &gr, &anc);
        let missing: Vec<&str> = readiness
            .iter()
            .filter(|(_, have)| !have)
            .map(|(n, _)| *n)
            .collect();
        assert!(
            missing.is_empty(),
            "every rung-3 anchor is banked for forsterite; still missing {missing:?}. K-prime was banked \
             all along and merely unread (a loader change), and theta_0 was inside the held manifest \
             extracts (a transcription). Neither was the fetch it was counted as."
        );
        assert_eq!(
            readiness.len(),
            7,
            "six anchors plus the one-joint-fit condition"
        );
    }

    /// QUARTZ IS NOT READY, and the reason is one italic cell in a source table.
    ///
    /// Its Debye temperature was fit and its volume exponent was assumed, so a row-level grade would have
    /// called the whole row usable. The per-cell grade refuses it on `q` alone. This is the live firing of
    /// the font-metadata read: without it the assumed `q = 1` would have entered wearing a measured grade
    /// and nothing downstream would ever have known.
    #[test]
    fn a_row_whose_exponent_was_assumed_rather_than_fit_is_not_ready() {
        let (reg, mod_, gr, anc) = tables();
        let readiness = mie_gruneisen_debye_readiness("quartz", &reg, &mod_, &gr, &anc);
        let missing: Vec<&str> = readiness
            .iter()
            .filter(|(_, have)| !have)
            .map(|(n, _)| *n)
            .collect();
        assert!(
            missing.contains(&"volume_exponent_q"),
            "quartz's q is italic in Table A1, that is estimated from systematics, so it is refused as an \
             anchor: {missing:?}"
        );
        assert!(
            !missing.contains(&"debye_temperature_theta_0"),
            "and its theta_0 is Roman, so the SAME ROW carries one usable cell and one that is not. A \
             row-level grade would have lost that in whichever direction it rounded."
        );
    }

    /// HEMATITE IS REFUSED WHOLE, which is the correct outcome for a phase outside the sources' system.
    #[test]
    fn a_phase_absent_from_the_compilations_is_refused_rather_than_estimated() {
        let (reg, mod_, gr, anc) = tables();
        let readiness = mie_gruneisen_debye_readiness("hematite", &reg, &mod_, &gr, &anc);
        let missing: Vec<&str> = readiness
            .iter()
            .filter(|(_, have)| !have)
            .map(|(n, _)| *n)
            .collect();
        assert!(
            missing.contains(&"debye_temperature_theta_0") && missing.contains(&"volume_exponent_q"),
            "hematite is a ferric phase outside both mantle-species compilations, so it has neither \
             anchor and gets no neighbour's: {missing:?}"
        );
    }

    /// A phase with no Grueneisen row refuses by name rather than borrowing another phase's numbers.
    #[test]
    fn a_phase_with_no_row_refuses_rather_than_inheriting_an_earth_mineral_default() {
        let (reg, mod_, gr, anc) = tables();
        let state = ThermoState {
            temperature_k: Fixed::from_int(300),
            pressure_bar: Fixed::ONE,
        };
        let err = response_at("unobtainium", state, &reg, &mod_, &gr, &anc)
            .expect_err("an unknown phase must refuse");
        assert!(
            matches!(err, ThermoRefusal::UnknownPhase { .. }),
            "and it refuses as UNKNOWN rather than as a missing rung: {err}"
        );
    }
}
