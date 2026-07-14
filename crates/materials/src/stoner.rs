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

//! Stage 6, the magnetism sub-arc slice (c): the ITINERANT Stoner criterion
//! (`docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md` section 11.3, gate ruled), the branch a Localized (Mott)
//! dispatch escalates to. Where slice (a) gives a Localized centre its Hund local moment, an itinerant d-band metal's
//! magnetism is set by the Stoner criterion: the exchange-split band spontaneously polarizes (itinerant
//! ferromagnetism) when `I * N(E_F) > 1`, with `I` the Stoner exchange parameter (eV) and `N(E_F)` the per-spin
//! NONMAGNETIC band density of states at the Fermi level (states/eV/spin/atom).
//!
//! THE DEFINITION TAG ON N (the composition error made unrepresentable). The criterion needs the bare-band
//! nonmagnetic DOS, NOT the calorimetric `(1 + lambda)`-dressed exchange-split ground-state DOS that a measured
//! electronic specific-heat `gamma` produces. Those two are a name-and-units match (both "states/eV at `E_F`") but a
//! DEFINITION mismatch: the measured `gamma` is enhanced by electron-phonon `(1 + lambda)` and computed on the
//! magnetic ground state, so `I * N_gamma` understates the criterion and yields false negatives (Fe and Co come out
//! below 1). This is the exact composition error [`crate::definition`] guards. Here it is enforced at the type: the
//! criterion consumes only a [`NonmagneticDos`], so a calorimetric DOS (a different type) cannot be silently passed
//! (the compile-time DOS-axis newtype the definition-tag ruling named).
//!
//! THE ESTIMATOR-GRADE CLASSIFIER (owner/gate ruling). `I * N > 1` is a SHARP threshold on a FACTOR-GRADE quantity,
//! so it is reliable only at the extremes: well above the upper edge is an itinerant ferromagnet (Fe, Ni), well
//! below the lower edge is a Pauli paramagnet (the clean simple metals Al/Ag/Au/Na/Mg at `~0.1-0.2`), and the
//! marginal band ESCALATES rather than forcing a wrong binary. The criterion's own known failures prove the point:
//! Co lands just under 1 yet is ferromagnetic (a volume/vertex correction), Pt just over 1 yet is paramagnetic
//! (spin-orbit / spin-fluctuation suppression), and Cr orders at a finite `q` (a spin-density wave the uniform
//! `q = 0` criterion misses). The band edges are caller-supplied (the resolution pattern), reserved not fabricated: a
//! clean primary column can tighten toward 1, and a noisy secondary column takes a factor-2 band so a mis-scaled row
//! escalates rather than misclassifies (the gate's sharpening).
//!
//! THE COLUMN IS HELD. The Stoner `I` and `N` values are held for a citable primary (Janak 1977 / Moruzzi-Janak-
//! Williams 1978); the delivered compilations are mutually inconsistent (a per-spin / per-atom mis-scaling the
//! definition tag caught: Cu obeys the 2x between the two tables while Fe is identical, so a row is corrupt) and do
//! not seed a physics column. This slice is the mechanism and the negative-control gate; the numbers wire on the
//! owner's primary or an explicit labeled-secondary ruling. Byte-neutral: `civsim-materials` is a leaf.

use civsim_core::Fixed;

/// The per-spin NONMAGNETIC band density of states at the Fermi level, `N(E_F)`, in states/eV/spin/atom. A NEWTYPE so
/// the Stoner criterion cannot be silently handed the wrong DOS: the criterion needs the bare-band nonmagnetic DOS,
/// never the calorimetric `(1 + lambda)`-dressed exchange-split ground-state DOS a measured `gamma` produces (the
/// composition error the definition tag guards). Constructing one asserts the value IS the nonmagnetic band DOS; a
/// calorimetric DOS carries its own distinct type and the criterion refuses it at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonmagneticDos(Fixed);

impl NonmagneticDos {
    /// Tag a per-spin, per-eV, per-atom NONMAGNETIC band DOS value as the Stoner `N(E_F)` input. The caller vouches
    /// that this is the bare-band nonmagnetic DOS (not a dressed calorimetric one); the newtype carries that promise
    /// to the criterion.
    pub fn per_spin_per_ev(value: Fixed) -> Self {
        NonmagneticDos(value)
    }

    /// The underlying states/eV/spin/atom value.
    pub fn get(self) -> Fixed {
        self.0
    }
}

/// The itinerant magnetic classification from the Stoner criterion, an ESTIMATOR-grade call that resolves only at
/// the extremes and escalates the marginal band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StonerClass {
    /// `I * N` above the upper edge: an itinerant ferromagnet (Fe, Ni). Classified only well above threshold.
    ItinerantFerromagnet,
    /// `I * N` below the lower edge: a Pauli paramagnet / nonmagnetic metal (the clean simple metals). Classified
    /// only well below threshold.
    PauliParamagnet,
    /// `I * N` within the marginal band: the uniform Stoner criterion cannot cleanly call it (Co, Pd, Pt, and
    /// finite-`q` cases like Cr's spin-density wave), and the band absorbs a noisy column's per-row uncertainty.
    /// Escalate rather than force a wrong binary.
    Escalate,
}

/// The Stoner product `S = I * N(E_F)` (dimensionless), the criterion's discriminant. `I` in eV, `N` the per-spin
/// nonmagnetic band DOS (states/eV/spin/atom), so the product is dimensionless. The `I * N` product is convention-
/// independent (a per-spin / per-atom or Rydberg / eV split that rescales one factor rescales the other inversely).
/// `None` on overflow.
pub fn stoner_product(i_ev: Fixed, n: NonmagneticDos) -> Option<Fixed> {
    i_ev.checked_mul(n.get())
}

/// Classify by the Stoner product against a lower and an upper band edge (the escalate band): ferromagnet above the
/// upper edge, paramagnet below the lower edge, escalate between. Reserves no value: the edges are caller-supplied
/// (the resolution pattern). The gate's sharpening for a noisy secondary column is a factor-2 band (lower `~0.5`,
/// upper `~2.0`) so a per-row mis-scaling lands in ESCALATE, not a wrong class; a clean primary column can tighten
/// the band toward 1. A non-sensible band (`lower >= upper`) still classifies deterministically (everything above
/// `upper` is a ferromagnet, below `lower` a paramagnet, and the empty middle never escalates), so the caller owns
/// the band's width.
pub fn stoner_classify(product: Fixed, lower: Fixed, upper: Fixed) -> StonerClass {
    if product > upper {
        StonerClass::ItinerantFerromagnet
    } else if product < lower {
        StonerClass::PauliParamagnet
    } else {
        StonerClass::Escalate
    }
}

/// A labeled negative-control: a substance whose Stoner product should land in an expected class under the band.
#[derive(Debug, Clone)]
pub struct StonerControl {
    /// The substance label (for the failure report).
    pub label: String,
    /// Its Stoner product `I * N`.
    pub product: Fixed,
    /// The class it must classify to under the band.
    pub expected: StonerClass,
}

/// The negative-control gate: assert every control classifies to its expected class under the band. `Ok(())` when
/// all pass; `Err(label)` on the first control that lands in the wrong class. The gate the noisy column's magnitudes
/// ride on: the clean simple metals must classify [`StonerClass::PauliParamagnet`], the clear ferromagnets
/// [`StonerClass::ItinerantFerromagnet`], and a marginal control (correctly) escalate. It validates the CHOSEN band
/// against known controls rather than trusting an uncited magnitude.
pub fn negative_control_gate(
    controls: &[StonerControl],
    lower: Fixed,
    upper: Fixed,
) -> Result<(), String> {
    for control in controls {
        if stoner_classify(control.product, lower, upper) != control.expected {
            return Err(control.label.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tight band for a CLEAN column (correct magnitudes): classify close to the physical threshold of 1.
    fn tight_band() -> (Fixed, Fixed) {
        (Fixed::from_ratio(9, 10), Fixed::from_ratio(11, 10))
    }

    // The gate's factor-2 band for a NOISY secondary column: classify only the far-clear cases (< 0.5 paramagnet,
    // > 2.0 ferromagnet), escalate everything a per-row 2x mis-scaling could have moved across 1.
    fn wide_band() -> (Fixed, Fixed) {
        (Fixed::from_ratio(1, 2), Fixed::from_int(2))
    }

    fn ctrl(label: &str, product: f64, expected: StonerClass) -> StonerControl {
        // Test-only illustrative products (the column is HELD); labeled fixtures, not a cited physics column.
        StonerControl {
            label: label.to_string(),
            product: Fixed::from_ratio((product * 1000.0) as i64, 1000),
            expected,
        }
    }

    #[test]
    fn the_product_is_i_times_the_nonmagnetic_dos() {
        // S = I * N. A synthetic I = 0.9 eV and N = 1.5 states/eV/spin gives 1.35. The N is tagged nonmagnetic band
        // DOS through the newtype (a calorimetric DOS would be a different type and refused).
        let n = NonmagneticDos::per_spin_per_ev(Fixed::from_ratio(15, 10));
        let s = stoner_product(Fixed::from_ratio(9, 10), n).expect("product");
        assert!(
            (s.to_f64_lossy() - 1.35).abs() < 1e-6,
            "I*N = 1.35, got {}",
            s.to_f64_lossy()
        );
    }

    #[test]
    fn a_clean_column_classifies_only_at_the_extremes_and_escalates_the_margin() {
        // With CORRECT magnitudes and a tight band around 1: a clear ferromagnet (Ni ~ 2.0) classifies FM, a clean
        // simple metal (Al ~ 0.16) classifies paramagnet, and a marginal itinerant (Pd ~ 0.98) escalates. (Synthetic
        // test-only products; the real column is held.)
        let (lo, hi) = tight_band();
        assert_eq!(
            stoner_classify(Fixed::from_int(2), lo, hi),
            StonerClass::ItinerantFerromagnet,
            "Ni ~ 2.0 is an itinerant ferromagnet"
        );
        assert_eq!(
            stoner_classify(Fixed::from_ratio(16, 100), lo, hi),
            StonerClass::PauliParamagnet,
            "Al ~ 0.16 is a Pauli paramagnet"
        );
        assert_eq!(
            stoner_classify(Fixed::from_ratio(98, 100), lo, hi),
            StonerClass::Escalate,
            "Pd ~ 0.98 is marginal and escalates"
        );
    }

    #[test]
    fn the_wide_band_makes_a_noisy_column_safe_by_construction() {
        // THE GATE'S SHARPENING: under the noisy compilation's own magnitudes and a factor-2 band, every magnetic /
        // marginal / mis-scaled row escalates (none exceeds 2.0), and only the deep simple metals (< 0.5) classify.
        // So a 2x per-row error cannot flip a row into a WRONG class; the worst case is an over-cautious escalate.
        let (lo, hi) = wide_band();
        // The compilation's Fe (1.04, downward-mis-scaled from the textbook ~1.5) escalates rather than misclassifying.
        assert_eq!(
            stoner_classify(Fixed::from_ratio(104, 100), lo, hi),
            StonerClass::Escalate,
            "a mis-scaled Fe (1.04) escalates, never a wrong class"
        );
        // Ni (1.31) also escalates under the wide band (safe, if over-cautious, until a clean primary lands).
        assert_eq!(
            stoner_classify(Fixed::from_ratio(131, 100), lo, hi),
            StonerClass::Escalate
        );
        // The deep negative controls still classify paramagnet (robust across any source).
        assert_eq!(
            stoner_classify(Fixed::from_ratio(16, 100), lo, hi),
            StonerClass::PauliParamagnet,
            "Cu/Al ~ 0.16 classify paramagnet under any reasonable band"
        );
    }

    #[test]
    fn the_negative_control_gate_passes_the_clean_controls_and_flags_a_misclassification() {
        // The gate over the clean simple-metal negative controls (deep paramagnets) plus clear ferromagnets, under
        // the tight band. All classify as expected, so the gate passes.
        let (lo, hi) = tight_band();
        let controls = vec![
            ctrl("Al", 0.16, StonerClass::PauliParamagnet),
            ctrl("Ag", 0.11, StonerClass::PauliParamagnet),
            ctrl("Au", 0.09, StonerClass::PauliParamagnet),
            ctrl("Ni", 2.0, StonerClass::ItinerantFerromagnet),
            ctrl("Pd", 0.98, StonerClass::Escalate),
        ];
        assert_eq!(negative_control_gate(&controls, lo, hi), Ok(()));
        // A control whose expected class is wrong (an Al mislabeled ferromagnet) is flagged by the gate.
        let bad = vec![ctrl("Al", 0.16, StonerClass::ItinerantFerromagnet)];
        assert_eq!(
            negative_control_gate(&bad, lo, hi),
            Err("Al".to_string()),
            "a deep paramagnet cannot be a ferromagnet; the gate flags it"
        );
    }
}
