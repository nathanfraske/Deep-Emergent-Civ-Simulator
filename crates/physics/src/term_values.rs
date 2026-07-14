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

//! The atomic term-value column (`eps_s`, `eps_p`, `crates/physics/data/term_values.toml`), the Harrison sp
//! bond-orbital gap estimator's polar-energy input, GATED on the owner's book fetch (Stage 6, owner ruling Part 4,
//! rider 2).
//!
//! Per element the `s` and `p` orbital term values in eV (negative, a bound orbital sits below the vacuum level),
//! provenance `[compute-once, cited, historical Herman-Skillman HF]`. The gap estimator reads this to form the
//! polar energy `V_3 = (eps_h_A - eps_h_B) / 2` with the sp3 hybrid term value `eps_h = (eps_s + 3 * eps_p) / 4`.
//! No consumer is wired to it in any pinned run path yet (byte-neutral).
//!
//! GATED ON THE FETCH. The standard column is seeded EMPTY: the term values are the owner's reserved book fetch
//! (Herman-Skillman Hartree-Fock, cited at delivery), NEVER seeded from memory. The estimator escalates on any
//! element until the fetch populates the column, the same shape as the compute-once guard: the mechanism is built
//! now, the data lands on the fetch.
//!
//! THE KOOPMANS CROSS-CHECK. When populated, each row carries a cross-check, never a substitute. Koopmans' theorem
//! relates the highest occupied orbital energy to the first ionization energy, `eps_HOMO ~ -IE_1`, and for the
//! p-block semiconductors the HOMO is the `p` orbital, so `eps_p ~ -IE_1` at the 10-to-20-percent grade.
//! [`TermValues::koopmans_residual_fraction`] reads the banked ionization-energy column and returns
//! `|eps_p + IE_1| / IE_1`, so a fetched term value that disagrees with the banked IE is FLAGGED, a coherence gate
//! on the fetch (the fetched value is authoritative; the residual is a sanity bound, not a substitute).

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the term-value column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermValueError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// An element appears twice.
    Duplicate(String),
    /// A term value is non-negative (a bound orbital sits BELOW the vacuum level, so `eps_s` and `eps_p` are
    /// negative; a non-negative value is not a bound term value).
    NotBound(String),
}

impl fmt::Display for TermValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TermValueError::Parse(m) => write!(f, "term-value parse error: {m}"),
            TermValueError::BadValue(m) => write!(f, "term-value value error: {m}"),
            TermValueError::MissingSource(m) => write!(f, "term-value row without citation: {m}"),
            TermValueError::Duplicate(m) => write!(f, "duplicate term-value element: {m}"),
            TermValueError::NotBound(m) => write!(f, "term-value not bound (>= 0): {m}"),
        }
    }
}

impl std::error::Error for TermValueError {}

/// One element's atomic term values (eV, negative): the `s` and `p` orbital energies the sp3 hybrid reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermValue {
    /// The `s` orbital term value in eV (negative).
    pub eps_s: Fixed,
    /// The `p` orbital term value in eV (negative).
    pub eps_p: Fixed,
}

impl TermValue {
    /// The sp3 hybrid term value `eps_h = (eps_s + 3 * eps_p) / 4`, the average orbital energy of the four sp3
    /// hybrids (one `s`, three `p`), the polar-energy input the bond-orbital model reads. A standard sp3 structure,
    /// not a fetched coefficient.
    pub fn sp3_hybrid_energy(self) -> Option<Fixed> {
        let three_p = Fixed::from_int(3).checked_mul(self.eps_p)?;
        self.eps_s
            .checked_add(three_p)?
            .checked_div(Fixed::from_int(4))
    }
}

/// The atomic term-value column: per element symbol, the cited `eps_s` and `eps_p`.
#[derive(Debug, Clone, Default)]
pub struct TermValues {
    by_symbol: BTreeMap<String, TermValue>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct TermValueFile {
    #[serde(default)]
    term_value: Vec<TermValueDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct TermValueDef {
    symbol: String,
    #[serde(default)]
    eps_s_ev: String,
    #[serde(default)]
    eps_p_ev: String,
    #[serde(default)]
    source: String,
}

impl TermValues {
    /// Load the column from a TOML string. Every row must carry a citation and bound (negative) term values.
    pub fn from_toml_str(s: &str) -> Result<Self, TermValueError> {
        let file: TermValueFile =
            toml::from_str(s).map_err(|e| TermValueError::Parse(e.to_string()))?;
        let mut by_symbol = BTreeMap::new();
        for tv in file.term_value {
            if tv.source.trim().is_empty() {
                return Err(TermValueError::MissingSource(tv.symbol.clone()));
            }
            let eps_s = Fixed::from_decimal_str(tv.eps_s_ev.trim())
                .map_err(|d| TermValueError::BadValue(format!("{} eps_s: {d}", tv.symbol)))?;
            let eps_p = Fixed::from_decimal_str(tv.eps_p_ev.trim())
                .map_err(|d| TermValueError::BadValue(format!("{} eps_p: {d}", tv.symbol)))?;
            if eps_s >= Fixed::ZERO || eps_p >= Fixed::ZERO {
                return Err(TermValueError::NotBound(tv.symbol.clone()));
            }
            if by_symbol
                .insert(tv.symbol.clone(), TermValue { eps_s, eps_p })
                .is_some()
            {
                return Err(TermValueError::Duplicate(tv.symbol));
            }
        }
        Ok(TermValues { by_symbol })
    }

    /// The embedded standard column (`data/term_values.toml`), seeded EMPTY until the owner's book fetch populates
    /// it (the term values are the reserved fetch, never seeded from memory).
    pub fn standard() -> Result<Self, TermValueError> {
        Self::from_toml_str(include_str!("../data/term_values.toml"))
    }

    /// The term values for an element, or `None` when the element is not in the seeded column (the gap estimator
    /// then escalates: the column is gated on the fetch).
    pub fn term_value(&self, symbol: &str) -> Option<TermValue> {
        self.by_symbol.get(symbol).copied()
    }

    /// The Koopmans cross-check residual fraction `|eps_p + IE_1| / IE_1` for an element (the p-block HOMO check):
    /// a fetched `eps_p` that agrees with the banked first ionization energy via Koopmans' theorem lands below the
    /// 10-to-20-percent grade. `None` when the element carries no term value or no banked ionization energy. A
    /// coherence gate on the fetch, never a substitute for it.
    pub fn koopmans_residual_fraction(&self, symbol: &str, table: &PeriodicTable) -> Option<Fixed> {
        let tv = self.term_value(symbol)?;
        let ie = table.element(symbol)?.ionization_energy?;
        if ie <= Fixed::ZERO {
            return None;
        }
        // |eps_p - (-IE_1)| / IE_1 = |eps_p + IE_1| / IE_1.
        let deviation = tv.eps_p.checked_add(ie)?;
        let magnitude = if deviation < Fixed::ZERO {
            Fixed::ZERO.checked_sub(deviation)?
        } else {
            deviation
        };
        magnitude.checked_div(ie)
    }

    /// The number of seeded elements (zero until the fetch).
    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    /// Whether the column is empty (the gated-on-the-fetch state).
    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_standard_column_is_gated_empty_until_the_fetch() {
        // The term values are the owner's reserved book fetch, so the standard column is seeded EMPTY: the
        // mechanism is built now, the data lands on the fetch.
        let tv = TermValues::standard().expect("the empty column loads");
        assert!(
            tv.is_empty(),
            "the standard column is gated empty until the fetch"
        );
        assert!(
            tv.term_value("Si").is_none(),
            "no term value until the fetch"
        );
    }

    #[test]
    fn the_sp3_hybrid_is_the_weighted_orbital_average() {
        // eps_h = (eps_s + 3 eps_p) / 4, the standard sp3 hybrid structure (one s, three p). A synthetic fixture:
        // eps_s = -12, eps_p = -8 gives eps_h = (-12 + -24)/4 = -9.
        let tv = TermValue {
            eps_s: Fixed::from_int(-12),
            eps_p: Fixed::from_int(-8),
        };
        let eps_h = tv.sp3_hybrid_energy().expect("hybrid energy");
        assert!(
            close(eps_h, -9.0, 1e-6),
            "eps_h = -9, got {}",
            eps_h.to_f64_lossy()
        );
    }

    #[test]
    fn the_koopmans_cross_check_passes_a_consistent_term_value_and_flags_an_off_one() {
        // THE KOOPMANS GATE: a fetched eps_p near -IE_1 lands within grade; a wildly-off one is flagged. (Synthetic
        // fixtures near a REAL element's banked IE, clearly test-only; the real values are the reserved fetch.)
        let table = PeriodicTable::standard().expect("periodic table");
        let ie_si = table
            .element("Si")
            .expect("Si")
            .ionization_energy
            .expect("Si IE")
            .to_f64_lossy();
        // A synthetic eps_p near -IE_1 (Si IE ~ 8.15 eV): a small residual, within the 10-to-20-percent grade.
        let consistent = format!(
            "[[term_value]]\nsymbol = \"Si\"\neps_s_ev = \"-14.0\"\neps_p_ev = \"{:.2}\"\nsource = \"test-only synthetic fixture near Si IE\"\n",
            -(ie_si)
        );
        let tv = TermValues::from_toml_str(&consistent).expect("loads");
        let residual = tv
            .koopmans_residual_fraction("Si", &table)
            .expect("residual");
        assert!(
            residual.to_f64_lossy() < 0.05,
            "a term value at -IE_1 has a tiny Koopmans residual, got {}",
            residual.to_f64_lossy()
        );
        // A wildly-off eps_p (-20 eV against an ~8 eV IE) is flagged well above the grade.
        let off = "[[term_value]]\nsymbol = \"Si\"\neps_s_ev = \"-25.0\"\neps_p_ev = \"-20.0\"\nsource = \"test-only off fixture\"\n";
        let tv_off = TermValues::from_toml_str(off).expect("loads");
        let residual_off = tv_off
            .koopmans_residual_fraction("Si", &table)
            .expect("residual");
        assert!(
            residual_off.to_f64_lossy() > 1.0,
            "a term value far from -IE_1 is flagged, got {}",
            residual_off.to_f64_lossy()
        );
    }

    #[test]
    fn a_missing_citation_and_a_non_bound_value_are_rejected() {
        let no_src = "[[term_value]]\nsymbol = \"Si\"\neps_s_ev = \"-14.0\"\neps_p_ev = \"-7.6\"\nsource = \"\"\n";
        assert!(matches!(
            TermValues::from_toml_str(no_src),
            Err(TermValueError::MissingSource(_))
        ));
        // A non-negative term value is not bound (below vacuum), so it is rejected.
        let unbound = "[[term_value]]\nsymbol = \"Si\"\neps_s_ev = \"14.0\"\neps_p_ev = \"-7.6\"\nsource = \"test\"\n";
        assert!(matches!(
            TermValues::from_toml_str(unbound),
            Err(TermValueError::NotBound(_))
        ));
    }
}
