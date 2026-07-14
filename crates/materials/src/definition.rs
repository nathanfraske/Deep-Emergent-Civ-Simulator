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

//! The DEFINITION-TAG mechanism (Stage 6, owner-ruled): a consumed quantity must match its PRODUCER'S DEFINITION,
//! beyond agreeing on name and units. The worked example that motivated it is the Stoner composition error, where
//! two individually-correct `[M]` facts (the Sommerfeld `gamma -> g(E_F)` identity and the Stoner criterion) were
//! joined on a name/units match while consuming DIFFERENT densities of states: the criterion needs the bare-band
//! nonmagnetic `N(E_F)`, the calorimetric `gamma` produces the `(1+lambda)`-dressed exchange-split ground-state
//! DOS, so the join was physically wrong though it type-checked. This module makes such a join fail at WIRING.
//!
//! THE PPLB UNIFICATION (owner ruling). The band-gap eigenvalue-routing law (a compute-once gap must be
//! hybrid/GW, never PBE/LDA) and the term-value law (an atomic term value must be HF-class, never Kohn-Sham LDA)
//! are the SAME law: the Perdew-Parr-Levy-Balduz piecewise-linearity condition (PRL 49, 1691 (1982)), whose missing
//! integer-`N` derivative discontinuity is what makes a semilocal (LDA/PBE) eigenvalue mis-state both the gap and
//! the atomic `eps ~ -IE` Koopmans level. One law, two crime scenes. So the band-gap `EigenvalueFunctional`
//! no-PBE guard and the term-value no-KS-LDA guard are TWO INSTANCES of one rule, [`EigenvalueProvenance`], not
//! parallel one-offs (this slice builds the shared rule; folding `EigenvalueFunctional` and `GapGrade` in as
//! instances is the named follow-on).
//!
//! THE COMPILE-TIME / RUNTIME SPLIT (owner ruling Q1). A silent-correctness axis (a name-and-units match that
//! hides a physically wrong join) becomes an unrepresentable type, the [`crate::band_gap::GapGrade`] /
//! `EigenvalueFunctional`-no-forbidden-variant pattern: a Koopmans-gated consumer simply cannot be handed a
//! Koopmans-incompatible value ([`EigenvalueProvenance::admits_koopmans_gated`] returns false and the
//! [`require_koopmans_gated`] join refuses it). The provenance tail (which cited GENERATION a value came from) is a
//! runtime tag ([`Generation`]) plus the join rule below, since a generation mismatch is a provenance concern, not
//! a silent-correctness one, until it enters a difference.
//!
//! THE SAME-GENERATION-PER-COMPOUND RULE (owner ruling, the sharp one). The Harrison polar energy
//! `V_3 = (eps_h(A) - eps_h(B)) / 2` is a DIFFERENCE of two atoms' term values across a compound. A systematic
//! generation offset partially cancels within one atom's `eps ~ -IE` but STOPS cancelling in the difference, so
//! mixing generations WITHIN a single compound is the worst case. [`compound_generation_consistent`] enforces that
//! every atom in one compound's `V_3` shares a generation tag, checked at the same join. Byte-neutral: the
//! `civsim-materials` leaf.

/// The provenance class of an eigenvalue-like quantity, the PPLB-unified "Koopmans-incompatible eigenvalue barred"
/// rule (one rule for the band-gap compute-once guard and the atomic term-value guard). A Koopmans-gated consumer
/// (an atomic term value for `eps ~ -IE`, or a compute-once band gap) admits only a measurement or a Koopmans-
/// compatible eigenvalue; a semilocal Kohn-Sham (LDA/PBE) eigenvalue is barred, because the missing PPLB
/// derivative discontinuity mis-states both the gap and the atomic level by 30-to-50 percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EigenvalueProvenance {
    /// A measurement (a spectroscopic gap, a measured ionization energy): always admissible.
    Measured,
    /// A Koopmans-compatible eigenvalue: true Hartree-Fock, Hartree-Fock-Slater / Xalpha (the Herman-Skillman
    /// line), a hybrid functional, or GW. The atomic `eps ~ -IE` holds to ~10 percent and the gap tracks the
    /// physical gap. The definition tag is "HF-class Koopmans-compatible", covering HFS/Xalpha and true HF, NOT
    /// literally "Hartree-Fock".
    KoopmansCompatible,
    /// A Koopmans-INCOMPATIBLE eigenvalue: a semilocal Kohn-Sham (LDA/PBE/LSD) eigenvalue. Barred from a
    /// Koopmans-gated consumer (the PPLB rule): the missing derivative discontinuity runs the atomic level 30-to-50
    /// percent too shallow and the gap far too small.
    KoopmansIncompatible,
}

impl EigenvalueProvenance {
    /// Whether this provenance is admitted to a KOOPMANS-GATED consumer (an atomic term value, a compute-once
    /// band gap): a measurement or a Koopmans-compatible eigenvalue, never a semilocal Kohn-Sham eigenvalue. The
    /// PPLB rule, the shared check the band-gap `EigenvalueFunctional` and the term-value HF-class tag both reduce
    /// to.
    pub fn admits_koopmans_gated(self) -> bool {
        matches!(
            self,
            EigenvalueProvenance::Measured | EigenvalueProvenance::KoopmansCompatible
        )
    }
}

/// What can go wrong joining a consumer's required definition against a producer's provided one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionMismatch {
    /// A Koopmans-gated consumer was handed a Koopmans-incompatible (semilocal Kohn-Sham) eigenvalue: the PPLB
    /// rule, refused at wiring.
    KoopmansIncompatibleValue,
    /// The atoms of one compound's term-value DIFFERENCE do not share a cited generation, so a systematic offset
    /// would fail to cancel in `V_3`.
    MixedGenerationInCompound {
        /// The two generation tags that clashed.
        first: String,
        /// The second, differing generation tag.
        second: String,
    },
}

/// The join guard for a KOOPMANS-GATED consumer: admit a measurement or a Koopmans-compatible eigenvalue, refuse a
/// semilocal Kohn-Sham one at wiring (the PPLB rule, with the ~20-percent Koopmans gate its runtime backstop:
/// Hartree-Fock passes at ~10 percent, LDA fails at 30-to-50 percent, so ~20 percent cleanly separates them).
pub fn require_koopmans_gated(provided: EigenvalueProvenance) -> Result<(), DefinitionMismatch> {
    if provided.admits_koopmans_gated() {
        Ok(())
    } else {
        Err(DefinitionMismatch::KoopmansIncompatibleValue)
    }
}

/// A cited provenance-generation tag (the runtime tail), e.g. the Froyen-Harrison 1979 term-value generation, the
/// Mann Hartree-Fock generation, or the Herman-Skillman generation. Distinct generations must not be mixed within
/// one compound's term-value difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generation(pub String);

/// The SAME-GENERATION-PER-COMPOUND rule: every atom feeding one compound's term-value difference (the Harrison
/// polar energy `V_3`) must share a cited generation, so a systematic generation offset cancels in the difference.
/// `Ok(())` for an empty or single-atom set (no difference to corrupt) or a uniform-generation set; otherwise the
/// first clashing pair. This is the sharp join rule (a within-compound mix is the worst case).
pub fn compound_generation_consistent(
    generations: &[Generation],
) -> Result<(), DefinitionMismatch> {
    let mut iter = generations.iter();
    let Some(first) = iter.next() else {
        return Ok(());
    };
    for g in iter {
        if g != first {
            return Err(DefinitionMismatch::MixedGenerationInCompound {
                first: first.0.clone(),
                second: g.0.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pplb_rule_bars_a_semilocal_eigenvalue_from_a_koopmans_gated_consumer() {
        // THE PPLB RULE (the worked example the term-value catch produced): a Kohn-Sham LDA/PBE eigenvalue is
        // Koopmans-incompatible and refused at wiring; a measurement and a Koopmans-compatible (HF-class / hybrid /
        // GW) eigenvalue are admitted. This is the single rule the band-gap no-PBE guard and the term-value
        // no-KS-LDA guard both reduce to.
        assert!(EigenvalueProvenance::Measured.admits_koopmans_gated());
        assert!(EigenvalueProvenance::KoopmansCompatible.admits_koopmans_gated());
        assert!(!EigenvalueProvenance::KoopmansIncompatible.admits_koopmans_gated());
        assert_eq!(
            require_koopmans_gated(EigenvalueProvenance::KoopmansIncompatible),
            Err(DefinitionMismatch::KoopmansIncompatibleValue),
            "a semilocal Kohn-Sham eigenvalue is refused at wiring (the NIST-DFT term-value case)"
        );
        assert_eq!(
            require_koopmans_gated(EigenvalueProvenance::KoopmansCompatible),
            Ok(()),
            "a Koopmans-compatible (HF-class) eigenvalue wires (the Froyen-Harrison / Mann case)"
        );
    }

    #[test]
    fn the_same_generation_rule_rejects_a_mixed_compound() {
        // THE SHARP RULE: a compound's V_3 differences the two atoms' term values, so mixing generations within one
        // compound is the worst case (the offset stops cancelling in the difference). A uniform-generation compound
        // wires; a mixed one is refused.
        let fh = Generation("Froyen-Harrison-1979".to_string());
        let mann = Generation("Mann-HF".to_string());
        // GaAs both from Froyen-Harrison: consistent.
        assert_eq!(
            compound_generation_consistent(&[fh.clone(), fh.clone()]),
            Ok(())
        );
        // GaAs with Ga from Froyen-Harrison and As from Mann: the mix is refused.
        assert!(matches!(
            compound_generation_consistent(&[fh.clone(), mann.clone()]),
            Err(DefinitionMismatch::MixedGenerationInCompound { .. })
        ));
        // A single atom (no difference to corrupt) is trivially consistent.
        assert_eq!(compound_generation_consistent(&[fh]), Ok(()));
        assert_eq!(compound_generation_consistent(&[]), Ok(()));
    }
}
