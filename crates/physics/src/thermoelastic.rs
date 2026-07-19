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
//! ONLY RUNG 4 CAN ANSWER, and the ladder says so rather than implying it by absence. Rung 3 needs six
//! per-phase anchors and the repository banks four of them: molar volume (phase registry), bulk modulus
//! (mineral moduli), `K'` with its adiabatic-versus-isothermal type recorded (Grueneisen table), and
//! `gamma_0` (Grueneisen table). It is missing the DEBYE TEMPERATURE, which appears in no data file at
//! all, and the VOLUME EXPONENT `q`. Both are fetches, and naming them here is what turns "the cluster
//! refuses at interior conditions" into "the cluster refuses, and rung 3 would answer if these two columns
//! existed".
//!
//! Rungs 1 and 2 are further out: rung 1 wants a cited P-V-T surface per phase (forsterite has a lead to
//! about 14 GPa and 1900 K, which is one phase rather than a mechanism), and rung 2 wants a quasi-harmonic
//! calculation and its cache.

use crate::gruneisen::GruneisenTable;
use crate::mineral_moduli::MineralModuli;
use crate::petrology_data::PhaseRegistry;
use civsim_core::Fixed;

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

/// The anchors the Mie-Grueneisen-Debye rung needs, and whether the repository banks each.
///
/// Reported as DATA rather than described in prose, so "rung 3 is blocked" is a machine-checkable claim
/// and closing it is a visible state change rather than an edit to a comment.
pub fn mie_gruneisen_debye_readiness(
    phase: &str,
    registry: &PhaseRegistry,
    moduli: &MineralModuli,
    gruneisen: &GruneisenTable,
) -> Vec<(&'static str, bool)> {
    let has_volume = registry.phase(phase).is_some();
    let key = crate::mineral_moduli::canonical_phase_key(phase);
    let has_k0 = moduli.row(key).is_some();
    let row = gruneisen.row(phase);
    let has_gamma = row.and_then(|r| r.gamma()).is_some();
    // K' IS BANKED BUT NOT LOADED, which is a different and much cheaper gap than a fetch.
    // `crates/physics/data/gruneisen.toml` carries a `kprime` for fourteen rows AND a `kprime_type`
    // recording whether each is adiabatic or isothermal, which is exactly the discipline rung 3 needs.
    // `GruneisenRow` does not expose either. So this reports FALSE while being honest about why: the
    // number exists, the loader does not read it, and closing that is an afternoon rather than a fetch.
    let has_kprime = false;
    vec![
        ("molar_volume_V0", has_volume),
        ("bulk_modulus_K0", has_k0),
        ("kprime_K0_prime_LOADER_GAP", has_kprime),
        ("gruneisen_gamma_0", has_gamma),
        // Neither of these appears in ANY data file. They are the two fetches between this ladder and a
        // state-resolved interior, and they are listed here so that claim is checkable rather than
        // asserted: a test reads this function, so the day the columns land the test changes.
        ("debye_temperature_theta_0", false),
        ("volume_exponent_q", false),
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
) -> Result<ThermoResponse, ThermoRefusal> {
    let row = registry
        .phase(phase)
        .ok_or_else(|| ThermoRefusal::UnknownPhase {
            phase: phase.to_string(),
        })?;

    // RUNGS 1 AND 2 are not built. They are absent rather than stubbed, because a stub that returns a
    // plausible value is the defect, and a stub that returns an error is what the fall-through already is.

    // RUNG 3: Mie-Grueneisen-Debye. Reports its own readiness rather than being described as blocked.
    let readiness = mie_gruneisen_debye_readiness(phase, registry, moduli, gruneisen);
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
            // Rung 3 could answer here once it is built; this branch is unreachable today and is written
            // so that landing the two columns produces a compile-checked hole rather than silent success.
            return Err(ThermoRefusal::RungUnavailable {
                phase: phase.to_string(),
                rung: ThermoRung::MieGruneisenDebye,
                missing: vec![
                    "the rung 3 solver itself, whose anchors are now all banked".to_string()
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
    crate::mineral_moduli::canonical_phase_key(phase)
}

fn abs_diff(a: Fixed, b: Fixed) -> Fixed {
    if a > b {
        a - b
    } else {
        b - a
    }
}

/// The ambient expansivity from the Grueneisen identity, at the rows' own frame and nowhere else.
fn ambient_alpha(
    gamma: Fixed,
    phase: &crate::petrology_data::Phase,
    bulk_gpa: Fixed,
) -> Result<Fixed, ThermoRefusal> {
    let unrepresentable = || ThermoRefusal::UnknownPhase {
        phase: phase.name.clone(),
    };
    // Dulong-Petit per mole of FORMULA UNITS, matching the registry molar volume's basis. Mixing that
    // with an atoms-per-primitive-cell count would be a basis error wearing a derivation's clothes, and
    // that column sits one table over.
    let atoms: u32 = phase.composition.iter().map(|(_, c)| *c).sum();
    let r = crate::gas_thermochemistry::molar_gas_constant().ok_or_else(unrepresentable)?;
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

    fn tables() -> (PhaseRegistry, MineralModuli, GruneisenTable) {
        (
            PhaseRegistry::standard().expect("registry"),
            MineralModuli::standard().expect("moduli"),
            GruneisenTable::standard().expect("gruneisen"),
        )
    }

    /// THE LADDER ANSWERS INSIDE ITS FRAME and refuses outside it, which is the whole contract.
    #[test]
    fn the_ambient_rung_answers_at_its_own_frame_and_refuses_at_interior_conditions() {
        let (reg, mod_, gr) = tables();
        let ambient = ThermoState {
            temperature_k: Fixed::from_int(300),
            pressure_bar: Fixed::ONE,
        };
        let r = response_at("forsterite", ambient, &reg, &mod_, &gr)
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

        // The mantle query that started all of this.
        let interior = ThermoState {
            temperature_k: Fixed::from_int(1600),
            pressure_bar: Fixed::from_int(100_000),
        };
        let err = response_at("forsterite", interior, &reg, &mod_, &gr)
            .expect_err("no built rung answers at interior conditions");
        let text = format!("{err}");
        assert!(
            text.contains("debye_temperature_theta_0") || text.contains("volume_exponent_q"),
            "the refusal NAMES the missing rung-3 anchors rather than saying no: {text}"
        );
    }

    /// The readiness report is DATA, so "rung 3 is blocked on two columns" is checkable rather than
    /// asserted in prose. When either column lands, this test changes, which is the intended signal.
    #[test]
    fn rung_three_is_blocked_on_exactly_two_unbanked_anchors() {
        let (reg, mod_, gr) = tables();
        let readiness = mie_gruneisen_debye_readiness("forsterite", &reg, &mod_, &gr);
        let missing: Vec<&str> = readiness
            .iter()
            .filter(|(_, have)| !have)
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(
            missing,
            vec![
                "kprime_K0_prime_LOADER_GAP",
                "debye_temperature_theta_0",
                "volume_exponent_q"
            ],
            "three anchors are unavailable, and they are NOT the same kind of unavailable: K-prime is \
             banked in gruneisen.toml with its adiabatic-versus-isothermal type recorded and merely not \
             loaded, while the Debye temperature and q appear in no data file at all. One is a loader \
             change, two are fetches, and conflating them would misprice the work."
        );
        let have: Vec<&str> = readiness
            .iter()
            .filter(|(_, h)| *h)
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(
            have.len(),
            3,
            "molar volume, K0 and gamma_0 are banked and loaded"
        );
    }

    /// A phase with no Grueneisen row refuses by name rather than borrowing another phase's numbers.
    #[test]
    fn a_phase_with_no_row_refuses_rather_than_inheriting_an_earth_mineral_default() {
        let (reg, mod_, gr) = tables();
        let state = ThermoState {
            temperature_k: Fixed::from_int(300),
            pressure_bar: Fixed::ONE,
        };
        let err = response_at("unobtainium", state, &reg, &mod_, &gr)
            .expect_err("an unknown phase must refuse");
        assert!(
            matches!(err, ThermoRefusal::UnknownPhase { .. }),
            "and it refuses as UNKNOWN rather than as a missing rung: {err}"
        );
    }
}
