// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Stage 6, the optics sub-arc (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md` section 13, gate-ruled): the
//! OBSERVER-INDEPENDENT optical characteristic energies of a substance, dispatched on the banked electronic
//! classification. A substance's optical response emerges from its electronic structure through three routes, keyed
//! on the classification already built (never an authored appearance table):
//! - INTERBAND ONSET (a non-metal): absorption turns on at the band gap `E_gap`, transparent below and absorbing
//!   above.
//! - PLASMA EDGE (a metal): the Drude reflection edge at `hbar * omega_p`, reflecting below and transmitting above.
//! - D-D LINE (a Localized d-block cation): a ligand-field absorption line at the crystal-field splitting `Delta_o`.
//!
//! PRINCIPLE 10, THE RULED SEAM. This substrate produces the OBSERVER-INDEPENDENT physical quantity (the
//! characteristic optical energies, and the reflection/absorption spectrum they define), never a colour. A perceived
//! colour is observer-dependent: it is the observer's photoreceptor response projected against an illuminant, and
//! even the "visible window" is the observer's property (a human's `~1.6-3.1 eV`, an alien's whatever its
//! photoreceptors span), rather than the material's. So no per-material colour and no hardcoded visible band live
//! here: the window is a caller parameter ([`falls_in_observer_window`]), and the colour projection is a downstream,
//! per-observer computation, never in this floor. The admit-the-alien payoff: the same material spectrum yields a
//! different perceived colour to a being with a different eye, a data-row difference.
//!
//! THE COLOUR PROJECTION'S ONE LEGAL HOME (owner sharpening). A colour may be AUTHORED in exactly one place: the
//! engine's observability NON-CANON layer (the renderer / glyph view), where a human-baseline mapping may say "this
//! wavelength is red" for display. The hard invariant is ZERO effect on the canon: the view reads the canon spectrum
//! one-way, the canon NEVER reads the view, and a wavelength-to-colour mapping that ever moved a run's `state_hash`
//! is a canon leak that fails the gate. This module is the canon side, observer-independent energies only,
//! byte-neutral. A being's OWN perceived colour, when it matters in the sim, emerges from its own visual system in
//! the canon over the same spectrum, never from the human display mapping.
//!
//! Q1 (a), the ruled granularity. This slice is the CHARACTERISTIC ENERGIES (the onsets and lines), the
//! fabrication-free observer-independent core, reserving nothing (each energy is the substance's own electronic
//! datum: the gap, the plasma energy, the crystal-field splitting). The full absorption/reflection envelope is a
//! derived follow-on ONLY when its broadening widths derive from the floor (thermal `~ kT`, the lifetime width from
//! the Drude scattering time already built, phonon widths), never an authored linewidth.
//!
//! HONEST LIMITS. The metal route emits the plasma edge; the d-band interband transition that reddens copper and
//! gold (a `d`-band-to-Fermi-level onset, distinct from a gap) is a named follow-on within the metal route. The d-d
//! line is sited at `Delta_o` (the leading ligand-field transition); the full Tanabe-Sugano multiplet structure over
//! `Delta_o` and the Racah parameters is a follow-on. Byte-neutral: `civsim-materials` is a leaf.

use civsim_core::Fixed;

use crate::band_gap::ConductionClass;

/// The physical origin of an optical characteristic energy (observer-independent). The feature says WHAT the energy
/// is (an absorption onset, a reflection edge, a discrete line), so a downstream observer projection can weight it
/// correctly without the substrate committing to a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpticalFeature {
    /// The interband absorption onset at the band gap: transparent below, absorbing above (a non-metal). A step in
    /// absorption.
    InterbandOnset,
    /// The Drude / plasma reflection edge at `hbar * omega_p`: reflecting below, transmitting above (a metal). A step
    /// in reflectivity.
    PlasmaEdge,
    /// A d-d ligand-field absorption line at `Delta_o` (a Localized d-block cation). A discrete line.
    DdLine,
}

/// One observer-independent optical characteristic energy (eV) and its physical origin. NOT a colour: which of these
/// fall in a given observer's sensitivity window, and how they combine into a perceived colour, is the observer's
/// downstream computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpticalEnergy {
    /// The physical origin of the feature.
    pub feature: OpticalFeature,
    /// The characteristic energy in eV (positive), the substance's own electronic datum.
    pub energy_ev: Fixed,
}

/// The observer-independent optical characteristic energies of a substance, dispatched on the banked electronic
/// classification (the gate-ruled Q3: dispatch on the existing classification, no new route key). A metal emits its
/// plasma edge, a non-metal (including a correlated Mott insulator) its interband onset at the gap, and any substance
/// with a resolved d-d transition (a Localized d-block cation) adds its ligand-field line. Each energy is the
/// substance's own datum, supplied by the caller from the banked columns: `band_gap_ev` from the gap column or the
/// Harrison estimator, `plasma_energy_ev` from [`crate::electronic::plasma_energy_ev`], `dd_transition_ev` from the
/// crystal-field `Delta_o` (converted to eV). A `None` or non-positive input contributes no feature (the substance
/// has no such edge). Reserves no value, authors no colour.
pub fn optical_energies(
    class: &ConductionClass,
    band_gap_ev: Option<Fixed>,
    plasma_energy_ev: Option<Fixed>,
    dd_transition_ev: Option<Fixed>,
) -> Vec<OpticalEnergy> {
    let mut out = Vec::new();
    match class {
        ConductionClass::Metal => {
            if let Some(p) = plasma_energy_ev {
                if p > Fixed::ZERO {
                    out.push(OpticalEnergy {
                        feature: OpticalFeature::PlasmaEdge,
                        energy_ev: p,
                    });
                }
            }
        }
        ConductionClass::NonMetal { .. } | ConductionClass::CorrelatedInsulator => {
            if let Some(g) = band_gap_ev {
                if g > Fixed::ZERO {
                    out.push(OpticalEnergy {
                        feature: OpticalFeature::InterbandOnset,
                        energy_ev: g,
                    });
                }
            }
        }
        // Escalate: the classification did not resolve an edge, so no edge feature. A d-d line may still be present.
        ConductionClass::Escalate => {}
    }
    // The d-d ligand-field line rides on a Localized d-block cation's resolved transition, independent of the
    // metal/non-metal edge (a transition-metal oxide carries both its interband onset and its d-d line).
    if let Some(d) = dd_transition_ev {
        if d > Fixed::ZERO {
            out.push(OpticalEnergy {
                feature: OpticalFeature::DdLine,
                energy_ev: d,
            });
        }
    }
    out
}

/// Whether an optical energy falls within an OBSERVER'S sensitivity window `[window_low_ev, window_high_ev]`. The
/// window is the OBSERVER'S property (a human's `~1.6-3.1 eV`, an alien's whatever its photoreceptors span), never
/// the material's, so it is a parameter here rather than a hardcoded constant (Principle 10). This is as far toward
/// "colour" as the observer-independent substrate reaches: which features a given observer can sense. The perceived
/// colour itself (the observer's photoreceptor response against an illuminant) is the observer's downstream
/// projection, not in this substrate.
pub fn falls_in_observer_window(
    energy: &OpticalEnergy,
    window_low_ev: Fixed,
    window_high_ev: Fixed,
) -> bool {
    energy.energy_ev >= window_low_ev && energy.energy_ev <= window_high_ev
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(x: i64, d: i64) -> Fixed {
        Fixed::from_ratio(x, d)
    }

    #[test]
    fn a_metal_emits_its_plasma_edge_and_a_non_metal_its_interband_onset() {
        // THE DISPATCH (Q3): a metal routes to the plasma edge, a non-metal to the interband onset at the gap. Each
        // is the substance's own electronic datum, no authored appearance.
        let metal = optical_energies(
            &ConductionClass::Metal,
            None,
            Some(ev(9, 1)), // hbar*omega_p ~ 9 eV (a typical metal plasma energy)
            None,
        );
        assert_eq!(metal.len(), 1);
        assert_eq!(metal[0].feature, OpticalFeature::PlasmaEdge);
        assert_eq!(metal[0].energy_ev, ev(9, 1));

        let semiconductor = optical_energies(
            &ConductionClass::NonMetal {
                ln_activation: None,
            },
            Some(ev(112, 100)), // Si gap 1.12 eV
            None,
            None,
        );
        assert_eq!(semiconductor.len(), 1);
        assert_eq!(semiconductor[0].feature, OpticalFeature::InterbandOnset);
        assert_eq!(semiconductor[0].energy_ev, ev(112, 100));
    }

    #[test]
    fn a_mott_insulator_with_a_dd_transition_carries_both_the_interband_onset_and_the_dd_line() {
        // A Localized transition-metal oxide (a correlated insulator) carries its interband/charge-transfer onset AND
        // its d-d ligand-field line. NiO-like: a wide gap (~4.3 eV) plus a d-d line in the visible (~1.1 eV).
        let features = optical_energies(
            &ConductionClass::CorrelatedInsulator,
            Some(ev(43, 10)),
            None,
            Some(ev(11, 10)),
        );
        assert_eq!(features.len(), 2);
        assert!(features
            .iter()
            .any(|f| f.feature == OpticalFeature::InterbandOnset && f.energy_ev == ev(43, 10)));
        assert!(features
            .iter()
            .any(|f| f.feature == OpticalFeature::DdLine && f.energy_ev == ev(11, 10)));
    }

    #[test]
    fn a_non_positive_or_absent_energy_contributes_no_feature() {
        // A metal with no resolved plasma energy, or an escalated substance, yields no edge feature (never a
        // fabricated zero-energy edge).
        assert!(optical_energies(&ConductionClass::Metal, None, None, None).is_empty());
        assert!(optical_energies(
            &ConductionClass::Escalate,
            Some(ev(2, 1)),
            Some(ev(9, 1)),
            None
        )
        .is_empty());
        // A non-positive energy is not a feature.
        assert!(
            optical_energies(&ConductionClass::Metal, None, Some(Fixed::ZERO), None).is_empty()
        );
    }

    #[test]
    fn the_visible_window_is_the_observers_property_not_the_materials() {
        // PRINCIPLE 10: the same material feature (a d-d line at 2.5 eV) is inside a human's window (~1.6-3.1 eV) but
        // outside an infrared-sensing being's window (~0.5-1.5 eV). The window is the OBSERVER'S parameter; the
        // substrate authored no colour and no window. The perceived colour is each observer's downstream projection.
        let dd = OpticalEnergy {
            feature: OpticalFeature::DdLine,
            energy_ev: ev(25, 10),
        };
        assert!(
            falls_in_observer_window(&dd, ev(16, 10), ev(31, 10)),
            "2.5 eV is inside a human visible window"
        );
        assert!(
            !falls_in_observer_window(&dd, ev(5, 10), ev(15, 10)),
            "2.5 eV is outside an infrared-sensing being's window (a different perceived world)"
        );
    }
}
