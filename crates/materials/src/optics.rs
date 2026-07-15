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
//! classification. A substance's optical response emerges from its electronic structure through several routes, keyed
//! on the classification already built (never an authored appearance table):
//! - INTERBAND ONSET (a non-metal): absorption turns on at the band gap `E_gap`, transparent below and absorbing
//!   above.
//! - PLASMA EDGE (a metal): the Drude reflection edge at `hbar * omega_p`, reflecting below and transmitting above.
//! - D-D LINE (a Localized d-block cation): a ligand-field absorption line at the crystal-field splitting `Delta_o`,
//!   Laporte-forbidden and so weak and narrow.
//! - CHARGE-TRANSFER EDGE (an oxidized-metal phase, `O2- -> Fe3+` the worked case): a ligand-to-metal charge-transfer
//!   band whose onset is the derived optical-electronegativity energy; symmetry-ALLOWED, so intense and broad, its
//!   low-energy tail flooding the visible to redden and darken a ferric oxide.
//! - INTERVALENCE BAND (a mixed-valence phase, `Fe2+ -> Fe3+` in magnetite the worked case): a metal-to-metal charge-
//!   transfer band in the visible / near-infrared, broad and allowed, blackening the phase.
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
//! derived follow-on that reads GROUNDED broadening widths: the thermal width `~ k_B T`
//! ([`thermal_broadening_width_ev`]) and the lifetime width `hbar / tau` from the Drude scattering time
//! ([`lifetime_broadening_width_ev`]) land here, both reassembled from fundamental constants (the dimensionless-
//! constant law), never an authored linewidth. The spectrum envelope reads them: [`feature_response_at`] gives a
//! Lorentzian for a d-d line and a broadened step for an edge, evaluated per feature so the caller sums a
//! substance's features into the full spectrum, still observer-independent. The phonon broadening width is the
//! remaining follow-on.
//!
//! HONEST LIMITS. The metal route emits the plasma edge; the d-band interband transition that reddens copper and
//! gold (a `d`-band-to-Fermi-level onset, distinct from a gap) is a named follow-on within the metal route. The d-d
//! line is sited at `Delta_o` (the leading ligand-field transition); the full Tanabe-Sugano multiplet structure over
//! `Delta_o` and the Racah parameters is a follow-on. The charge-transfer and intervalence routes carry the derived
//! band ENERGY and a broad Marcus-Hush vibronic WIDTH ([`marcus_hush_width_ev`]), both fabrication-free; the relative
//! oscillator STRENGTH (an allowed charge-transfer band is orders of magnitude more intense than a forbidden d-d line,
//! the Laporte rule) is NOT yet carried per feature (every feature response peaks at one here), so the intensity that
//! makes the charge-transfer tail opaque in the visible is a class-grade weight the observer projection supplies
//! downstream, and a per-feature oscillator-strength column is the named canon follow-on. Byte-neutral:
//! `civsim-materials` is a leaf.

use civsim_core::Fixed;

use crate::band_gap::ConductionClass;

/// The physical origin of an optical characteristic energy (observer-independent). The feature says WHAT the energy
/// is (an absorption onset, a reflection edge, a discrete line, a charge-transfer band), so a downstream observer
/// projection can weight it correctly without the substrate committing to a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpticalFeature {
    /// The interband absorption onset at the band gap: transparent below, absorbing above (a non-metal). A step in
    /// absorption.
    InterbandOnset,
    /// The Drude / plasma reflection edge at `hbar * omega_p`: reflecting below, transmitting above (a metal). A step
    /// in reflectivity.
    PlasmaEdge,
    /// A d-d ligand-field absorption line at `Delta_o` (a Localized d-block cation). A discrete line, Laporte-
    /// forbidden and so intrinsically WEAK and narrow (its width is the near-thermal linewidth the caller supplies).
    DdLine,
    /// A ligand-to-metal charge-transfer absorption EDGE (an LMCT band, an oxide `O2-` donating into an oxidized
    /// metal `d`-shell, the `O2- -> Fe3+` case that reddens and darkens a ferric oxide). Its onset is the derived
    /// charge-transfer energy (the Jorgensen optical-electronegativity route). Unlike the Laporte-forbidden `DdLine`
    /// it is a symmetry-ALLOWED transition, so it is intense and broad: the caller supplies a broad (vibronic-scale)
    /// width and a high oscillator-strength weight, and the band's low-energy tail floods down into the visible.
    ChargeTransferBand,
    /// A metal-to-metal intervalence charge-transfer absorption BAND (an IVCT band, an electron hopping between two
    /// coexisting valences of the same metal, the `Fe2+ -> Fe3+` case that blackens a mixed-valence oxide such as
    /// magnetite). Its centre is the derived intervalence energy (the optical-electronegativity difference of the two
    /// valences). Allowed and broad like the charge-transfer edge, but a BAND (a peak) rather than an edge, sitting in
    /// the visible / near-infrared, so a strong broad response absorbs across the visible and reads dark.
    IntervalenceBand,
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
/// plasma edge, a non-metal (including a correlated Mott insulator) its interband onset at the gap, any substance
/// with a resolved d-d transition (a Localized d-block cation) adds its ligand-field line, an oxidized-metal phase
/// adds its ligand-to-metal charge-transfer edge, and a mixed-valence phase adds its metal-to-metal intervalence
/// band. Each energy is the substance's own datum, supplied by the caller from the banked columns: `band_gap_ev` from
/// the gap column or the Harrison estimator, `plasma_energy_ev` from [`crate::electronic::plasma_energy_ev`],
/// `dd_transition_ev` from the crystal-field `Delta_o` (converted to eV), and `charge_transfer_ev` /
/// `intervalence_ev` from the crystal-field optical-electronegativity route (present only when the composition's iron
/// oxidation state, read from its phase, carries an oxidized or mixed-valence cation). A `None` or non-positive input
/// contributes no feature (the substance has no such edge). Reserves no value, authors no colour.
pub fn optical_energies(
    class: &ConductionClass,
    band_gap_ev: Option<Fixed>,
    plasma_energy_ev: Option<Fixed>,
    dd_transition_ev: Option<Fixed>,
    charge_transfer_ev: Option<Fixed>,
    intervalence_ev: Option<Fixed>,
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
    // The ligand-to-metal charge-transfer edge, present when the phase carries an oxidized cation (the caller derives
    // its onset from the optical-electronegativity route and passes `None` when there is no ferric-like centre).
    if let Some(ct) = charge_transfer_ev {
        if ct > Fixed::ZERO {
            out.push(OpticalEnergy {
                feature: OpticalFeature::ChargeTransferBand,
                energy_ev: ct,
            });
        }
    }
    // The metal-to-metal intervalence band, present only for a mixed-valence phase (two coexisting cation valences).
    if let Some(iv) = intervalence_ev {
        if iv > Fixed::ZERO {
            out.push(OpticalEnergy {
                feature: OpticalFeature::IntervalenceBand,
                energy_ev: iv,
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

/// The Boltzmann constant in eV/K (`~8.617e-5`), reassembled from the exact `k_B` and `e` mantissas (the
/// dimensionless-constant law): `k_B[eV/K] = k_B[J/K] / e[C] = (1.380649 / 1.602176634) * 1e-4`, the `1e-4` the net
/// of `k_B`'s `1e-23` over `e`'s `1e-19`. No folded dimensional decimal is authored.
fn kb_ev_per_k() -> Fixed {
    let kb_mantissa = Fixed::from_ratio(1_380_649, 1_000_000);
    let e_mantissa = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    kb_mantissa
        .checked_div(e_mantissa)
        .and_then(|r| r.checked_div(Fixed::from_int(10_000)))
        .unwrap_or(Fixed::ZERO)
}

/// The reduced Planck constant in eV*fs (`~0.6582`), reassembled from the exact `hbar` and `e` mantissas:
/// `hbar[eV*fs] = hbar[J*s] / e[C] * 1e15 = 1.054571817 / 1.602176634`, the powers `1e-34 / 1e-19 * 1e15` netting to
/// `1e0`, so the mantissa ratio is the value.
fn hbar_ev_fs() -> Fixed {
    let hbar_mantissa = Fixed::from_ratio(1_054_571_817, 1_000_000_000);
    let e_mantissa = Fixed::from_ratio(1_602_176_634, 1_000_000_000);
    hbar_mantissa.checked_div(e_mantissa).unwrap_or(Fixed::ZERO)
}

/// The thermal broadening width (eV) of an absorption edge, `~ k_B * T`: at finite temperature an absorption onset
/// is thermally smeared (the Urbach-tail scale), rather than a sharp step. Derived from the Boltzmann constant and
/// the world temperature, reserving no value (`k_B` in eV/K reassembles from fundamental constants). Zero for a
/// non-positive temperature. One of the grounded broadening widths the derived spectrum envelope (Q1 (b)) reads,
/// never an authored linewidth.
pub fn thermal_broadening_width_ev(temperature_k: Fixed) -> Fixed {
    if temperature_k <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    kb_ev_per_k()
        .checked_mul(temperature_k)
        .unwrap_or(Fixed::ZERO)
}

/// The lifetime broadening width (eV), `hbar / tau`, from the Drude carrier scattering time `tau` in femtoseconds
/// (the same `tau` [`crate::electronic::drude_scattering_time_fs`] produces): a finite carrier lifetime broadens a
/// spectral feature by `hbar / tau`. Derived from `hbar` in eV*fs (reassembled from fundamental constants), reserving
/// no value. Zero for a non-positive `tau`. The second grounded broadening width the derived spectrum envelope
/// (Q1 (b)) reads.
pub fn lifetime_broadening_width_ev(scattering_time_fs: Fixed) -> Fixed {
    if scattering_time_fs <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    hbar_ev_fs()
        .checked_div(scattering_time_fs)
        .unwrap_or(Fixed::ZERO)
}

/// The Lorentzian lineshape (a relative response, peak 1 at the centre): `hw^2 / ((probe - centre)^2 + hw^2)` with
/// `hw = width / 2`, so the width is the full width at half maximum (the response is `0.5` at `centre +/- width/2`).
/// The natural lifetime-broadened line for a discrete transition (the d-d line at `Delta_o`). `None` for a
/// non-positive width or on overflow.
pub fn lorentzian_response(probe_ev: Fixed, centre_ev: Fixed, width_ev: Fixed) -> Option<Fixed> {
    if width_ev <= Fixed::ZERO {
        return None;
    }
    let hw = width_ev.checked_div(Fixed::from_int(2))?;
    let hw_sq = hw.checked_mul(hw)?;
    let d = probe_ev.checked_sub(centre_ev)?;
    let d_sq = d.checked_mul(d)?;
    hw_sq.checked_div(d_sq.checked_add(hw_sq)?)
}

/// A broadened absorption / reflection STEP (a relative response rising `0` to `1`), the edge features (the interband
/// onset, the plasma edge) smeared over the broadening width: the logistic `1 / (1 + exp(-(probe - onset) / width))`,
/// `0.5` at the onset, rising above and falling below. The `exp` census window is guarded (far below the onset the
/// response is `0`, far above it is `1`), so a wide probe range never overflows the transcendental. `None` for a
/// non-positive width or on overflow.
pub fn broadened_step_response(probe_ev: Fixed, onset_ev: Fixed, width_ev: Fixed) -> Option<Fixed> {
    if width_ev <= Fixed::ZERO {
        return None;
    }
    let x = probe_ev.checked_sub(onset_ev)?.checked_div(width_ev)?;
    // Guard the exp window ([-22, 21.5]): far above the onset saturates to 1, far below to 0.
    let bound = Fixed::from_int(20);
    if x > bound {
        return Some(Fixed::ONE);
    }
    if x < Fixed::ZERO.checked_sub(bound)? {
        return Some(Fixed::ZERO);
    }
    let e = Fixed::ZERO.checked_sub(x)?.exp(); // exp(-x)
    Fixed::ONE.checked_div(Fixed::ONE.checked_add(e)?)
}

/// The derived-spectrum response of one optical feature at a probe energy (Q1 (b), the spectrum envelope): a `DdLine`
/// is a Lorentzian centred at its energy, an `InterbandOnset` or `PlasmaEdge` a broadened step at its energy, each
/// smeared by the caller's GROUNDED broadening `width_ev` (from [`thermal_broadening_width_ev`] or
/// [`lifetime_broadening_width_ev`], never an authored linewidth). Evaluated per feature so the caller sums over a
/// substance's features into the full envelope. Still observer-independent: this is the physical spectrum, not a
/// colour. `None` on a non-positive width or overflow.
pub fn feature_response_at(
    probe_ev: Fixed,
    feature: &OpticalEnergy,
    width_ev: Fixed,
) -> Option<Fixed> {
    match feature.feature {
        // A discrete line (`DdLine`) and a peaked band (`IntervalenceBand`) are Lorentzians centred at their energy;
        // the two differ in the width the caller supplies (near-thermal for the forbidden d-d line, the broad
        // vibronic width for the allowed intervalence band).
        OpticalFeature::DdLine | OpticalFeature::IntervalenceBand => {
            lorentzian_response(probe_ev, feature.energy_ev, width_ev)
        }
        // An edge (`InterbandOnset`, `PlasmaEdge`, the `ChargeTransferBand` onset) is a broadened step at its energy;
        // the charge-transfer edge's broad vibronic width gives it the long low-energy tail that floods the visible.
        OpticalFeature::InterbandOnset
        | OpticalFeature::PlasmaEdge
        | OpticalFeature::ChargeTransferBand => {
            broadened_step_response(probe_ev, feature.energy_ev, width_ev)
        }
    }
}

/// The charge-transfer / intervalence band width (eV), the classical (high-temperature) Marcus-Hush bandwidth
/// `FWHM = sqrt(16 * ln2 * lambda * k_B * T)`, with the reorganization energy `lambda` taken as the band's own
/// characteristic energy (exact for a symmetric intervalence transition where `E_op = lambda`, a class-grade
/// over-estimate for a ligand-to-metal band where `E_op = lambda + Delta_G`). This is the vibronic (electron-phonon)
/// broadening that makes an allowed charge-transfer band broad, in contrast to the near-thermal linewidth of a
/// forbidden d-d line: it consumes only the already-derived Boltzmann factor `k_B T` and the band energy, reserving
/// no value (`16 * ln2` reassembles from `Fixed::ln`, never a folded decimal). Zero for a non-positive input. The
/// grounded width the caller supplies to [`feature_response_at`] for a `ChargeTransferBand` or `IntervalenceBand`,
/// the charge-transfer counterpart to [`thermal_broadening_width_ev`] and [`lifetime_broadening_width_ev`].
pub fn marcus_hush_width_ev(band_energy_ev: Fixed, temperature_k: Fixed) -> Fixed {
    if band_energy_ev <= Fixed::ZERO || temperature_k <= Fixed::ZERO {
        return Fixed::ZERO;
    }
    // 16 * ln(2), reassembled from Fixed::ln rather than a folded decimal.
    let sixteen_ln2 = match Fixed::from_int(2).ln().checked_mul(Fixed::from_int(16)) {
        Some(v) => v,
        None => return Fixed::ZERO,
    };
    let kt = kb_ev_per_k()
        .checked_mul(temperature_k)
        .unwrap_or(Fixed::ZERO);
    sixteen_ln2
        .checked_mul(kt)
        .and_then(|v| v.checked_mul(band_energy_ev))
        .map(|v| v.sqrt())
        .unwrap_or(Fixed::ZERO)
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
            None,
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
            None,
            None,
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
        assert!(optical_energies(&ConductionClass::Metal, None, None, None, None, None).is_empty());
        assert!(optical_energies(
            &ConductionClass::Escalate,
            Some(ev(2, 1)),
            Some(ev(9, 1)),
            None,
            None,
            None
        )
        .is_empty());
        // A non-positive energy is not a feature.
        assert!(optical_energies(
            &ConductionClass::Metal,
            None,
            Some(Fixed::ZERO),
            None,
            None,
            None
        )
        .is_empty());
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

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_broadening_widths_derive_from_the_constants() {
        // Thermal ~ k_B T: kT(300 K) ~ 0.0259 eV (25.9 meV). Lifetime hbar/tau: tau = 20 fs -> ~0.033 eV. Both from
        // fundamental constants, no reserved value; zero at a non-positive input.
        let thermal = thermal_broadening_width_ev(Fixed::from_int(300));
        assert!(
            close(thermal, 0.02585, 5e-4),
            "kT(300) ~ 0.0259 eV, got {}",
            thermal.to_f64_lossy()
        );
        assert_eq!(thermal_broadening_width_ev(Fixed::ZERO), Fixed::ZERO);
        let lifetime = lifetime_broadening_width_ev(Fixed::from_int(20));
        assert!(
            close(lifetime, 0.0329, 5e-4),
            "hbar/tau(20 fs) ~ 0.033 eV, got {}",
            lifetime.to_f64_lossy()
        );
        assert_eq!(lifetime_broadening_width_ev(Fixed::ZERO), Fixed::ZERO);
    }

    #[test]
    fn the_lorentzian_peaks_at_the_centre_and_halves_at_the_half_width() {
        // Peak 1 at the centre, 0.5 at centre +/- width/2 (the width is the FWHM), and small far away. A d-d line at
        // 2.0 eV with a 0.2 eV width.
        let centre = ev(2, 1);
        let width = ev(2, 10); // 0.2 eV
        assert!(close(
            lorentzian_response(centre, centre, width).unwrap(),
            1.0,
            1e-6
        ));
        // At centre + width/2 = 2.1 eV, the response is 0.5 (half maximum).
        assert!(close(
            lorentzian_response(ev(21, 10), centre, width).unwrap(),
            0.5,
            1e-3
        ));
        // Far off resonance (1.0 eV, five half-widths away) the response is small.
        assert!(
            lorentzian_response(ev(1, 1), centre, width)
                .unwrap()
                .to_f64_lossy()
                < 0.02
        );
        assert!(lorentzian_response(centre, centre, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_broadened_step_rises_through_the_onset() {
        // A broadened edge: 0.5 at the onset, saturating to ~0 far below and ~1 far above (the exp window guarded, so
        // a wide probe range never overflows). An interband onset at 2.0 eV with a 0.1 eV width.
        let onset = ev(2, 1);
        let width = ev(1, 10); // 0.1 eV
        assert!(close(
            broadened_step_response(onset, onset, width).unwrap(),
            0.5,
            1e-6
        ));
        // Far above the onset (probe 3 eV, ten widths up) saturates to ~1; far below (1 eV) to ~0.
        assert!(
            broadened_step_response(ev(3, 1), onset, width)
                .unwrap()
                .to_f64_lossy()
                > 0.99
        );
        assert!(
            broadened_step_response(ev(1, 1), onset, width)
                .unwrap()
                .to_f64_lossy()
                < 0.01
        );
        assert!(broadened_step_response(onset, onset, Fixed::ZERO).is_none());
    }

    #[test]
    fn the_feature_response_lineshapes_the_right_way_per_feature() {
        // A d-d line reads a Lorentzian (peak at its energy); an interband onset reads a broadened step (0.5 at its
        // energy). The dispatch on the feature type, the derived spectrum envelope (Q1 (b)).
        let width = ev(2, 10);
        let dd = OpticalEnergy {
            feature: OpticalFeature::DdLine,
            energy_ev: ev(2, 1),
        };
        assert!(close(
            feature_response_at(ev(2, 1), &dd, width).unwrap(),
            1.0,
            1e-6
        ));
        let onset = OpticalEnergy {
            feature: OpticalFeature::InterbandOnset,
            energy_ev: ev(2, 1),
        };
        assert!(close(
            feature_response_at(ev(2, 1), &onset, width).unwrap(),
            0.5,
            1e-6
        ));
    }

    #[test]
    fn a_ferric_phase_adds_a_charge_transfer_edge_and_a_mixed_valence_phase_adds_an_intervalence_band(
    ) {
        // The two new routes emit only from their supplied energies (the caller derives them from the phase's iron
        // oxidation state): a ferric oxide passes a charge-transfer onset (~4 eV) and gets the edge; a mixed-valence
        // oxide passes both the charge-transfer onset and the intervalence centre (~1 eV) and gets both. A ferrous
        // (weak-d-d-only) phase passes neither and gets no new feature. Each energy is the substance's own datum.
        let ferric = optical_energies(
            &ConductionClass::CorrelatedInsulator,
            None,
            None,
            None,
            Some(ev(41, 10)), // O2- -> Fe3+ LMCT onset ~4.1 eV
            None,
        );
        assert!(ferric
            .iter()
            .any(|f| f.feature == OpticalFeature::ChargeTransferBand && f.energy_ev == ev(41, 10)));
        assert!(!ferric
            .iter()
            .any(|f| f.feature == OpticalFeature::IntervalenceBand));

        let mixed = optical_energies(
            &ConductionClass::CorrelatedInsulator,
            None,
            None,
            None,
            Some(ev(41, 10)),
            Some(ev(11, 10)), // Fe2+ -> Fe3+ IVCT ~1.1 eV
        );
        assert!(mixed
            .iter()
            .any(|f| f.feature == OpticalFeature::ChargeTransferBand));
        assert!(mixed
            .iter()
            .any(|f| f.feature == OpticalFeature::IntervalenceBand && f.energy_ev == ev(11, 10)));

        // A ferrous phase (only the weak near-IR d-d line, no oxidized or mixed centre) adds neither charge-transfer
        // feature: the honest per-phase distinction (ferrous stays light, ferric/mixed darken).
        let ferrous = optical_energies(
            &ConductionClass::CorrelatedInsulator,
            None,
            None,
            Some(ev(93, 100)), // FeO d-d ~0.93 eV
            None,
            None,
        );
        assert!(ferrous
            .iter()
            .all(|f| f.feature != OpticalFeature::ChargeTransferBand
                && f.feature != OpticalFeature::IntervalenceBand));
    }

    #[test]
    fn the_marcus_hush_width_is_broad_and_grows_with_the_band_energy_and_temperature() {
        // The Marcus-Hush vibronic width FWHM = sqrt(16 ln2 * lambda * kT). For a ~4 eV charge-transfer band at 300 K
        // it is ~1.07 eV (an order of magnitude broader than the ~0.026 eV thermal linewidth of a forbidden d-d
        // line), so its tail can flood the visible; it grows with both the band energy and the temperature. Derived
        // from kT and the band energy, reserving no value.
        let ct = marcus_hush_width_ev(Fixed::from_int(4), Fixed::from_int(300));
        assert!(
            close(ct, 1.071, 5e-3),
            "sqrt(16 ln2 * 4 eV * kT(300)) ~ 1.07 eV, got {}",
            ct.to_f64_lossy()
        );
        // Much broader than the thermal linewidth at the same temperature.
        assert!(ct > thermal_broadening_width_ev(Fixed::from_int(300)) * Fixed::from_int(10));
        // Grows with the band energy and with temperature.
        assert!(
            marcus_hush_width_ev(Fixed::from_int(4), Fixed::from_int(300))
                > marcus_hush_width_ev(Fixed::from_int(1), Fixed::from_int(300))
        );
        assert!(
            marcus_hush_width_ev(Fixed::from_int(4), Fixed::from_int(600))
                > marcus_hush_width_ev(Fixed::from_int(4), Fixed::from_int(300))
        );
        assert_eq!(
            marcus_hush_width_ev(Fixed::ZERO, Fixed::from_int(300)),
            Fixed::ZERO
        );
        assert_eq!(
            marcus_hush_width_ev(Fixed::from_int(4), Fixed::ZERO),
            Fixed::ZERO
        );
    }

    #[test]
    fn a_broad_charge_transfer_edge_floods_the_visible_a_narrow_one_does_not() {
        // THE FLOODING MECHANISM: a charge-transfer edge whose onset sits above the visible (~4.1 eV) reaches down
        // into the visible ONLY because its Marcus-Hush width is broad. With the broad width the tail at 2.75 eV
        // (blue) is substantial and stronger than at 1.9 eV (red), so blue is absorbed more than red (the reddening);
        // with a narrow thermal width the same edge is essentially dark-free in the visible. This is why the intensity
        // and breadth of the ALLOWED charge-transfer band, not a hardcoded visible wavelength, carries the darkening.
        let ct = OpticalEnergy {
            feature: OpticalFeature::ChargeTransferBand,
            energy_ev: ev(41, 10), // 4.1 eV onset, above the human visible window
        };
        let broad = marcus_hush_width_ev(ev(41, 10), Fixed::from_int(300));
        let blue = feature_response_at(ev(275, 100), &ct, broad)
            .unwrap()
            .to_f64_lossy();
        let red = feature_response_at(ev(19, 10), &ct, broad)
            .unwrap()
            .to_f64_lossy();
        assert!(
            blue > red,
            "the tail floods blue more than red (reddening): blue {blue} vs red {red}"
        );
        assert!(
            blue > 0.05,
            "the broad tail reaches the visible, got {blue}"
        );
        // The same edge with only the near-thermal width barely reaches the visible.
        let narrow = thermal_broadening_width_ev(Fixed::from_int(300));
        let blue_narrow = feature_response_at(ev(275, 100), &ct, narrow)
            .unwrap()
            .to_f64_lossy();
        assert!(
            blue_narrow < 1e-3,
            "a narrow edge does not flood the visible, got {blue_narrow}"
        );
    }
}
