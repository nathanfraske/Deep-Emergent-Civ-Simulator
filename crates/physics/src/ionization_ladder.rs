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

//! The successive-ionization-energy ladder floor (`crates/physics/data/ionization_ladder.toml`).
//!
//! Per element the successive ionization energies (IE1, IE2, IE3, ...) in electron-volts, MEASURED `[M]`
//! (spectroscopic; a laboratory refutes one by atomic spectroscopy). The correlation classifier's on-site
//! Coulomb axis reads it: the atomic Hubbard U of an ion of charge `q` is the differential ionization energy
//! `U_atomic(q) = IE_{q+1} - IE_q`, which for a d-electron ion is the d-d Coulomb repulsion the Mott-Hubbard
//! picture keys on (the spec's example: Ni2+ `IE3 - IE2 = 35.187 - 18.169 = 17.0 eV`).
//!
//! WHY A NEW COLUMN (the D2 grounding delta, gate-ruled). The periodic table carries only the FIRST ionization
//! energy (`Element::ionization_energy`, whose lone consumer is `qeq` for `chi`/`eta`), so the successive ladder
//! is not banked. The correlation-hardening spec's premise that this ladder was "already a banked column" did
//! not hold at source; this is the faithful build. A sibling of the periodic table and the Shannon radii, read
//! by the materials correlation classifier (D2b); no consumer is wired to it yet (a pure addition, byte-neutral).
//!
//! The loader checks each ladder is strictly increasing (a successive ionization energy must exceed the prior,
//! removing an electron from a more positive ion costs more), the physical reproduction-style guard, and fails
//! loud on a missing citation (every value is real-with-source).

use civsim_core::Fixed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// What can go wrong loading the ionization-ladder data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IonizationLadderError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A decimal value could not be parsed to fixed-point.
    BadValue(String),
    /// A row carries no citation (every value is real-with-source).
    MissingSource(String),
    /// An element appears twice.
    Duplicate(String),
    /// An element's ladder is empty or not strictly increasing (the physical monotonicity guard).
    NotMonotonic(String),
}

impl fmt::Display for IonizationLadderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IonizationLadderError::Parse(m) => write!(f, "ionization-ladder parse error: {m}"),
            IonizationLadderError::BadValue(m) => write!(f, "ionization-ladder value error: {m}"),
            IonizationLadderError::MissingSource(m) => {
                write!(f, "ionization-ladder row without citation: {m}")
            }
            IonizationLadderError::Duplicate(m) => {
                write!(f, "duplicate ionization-ladder element: {m}")
            }
            IonizationLadderError::NotMonotonic(m) => {
                write!(f, "ionization-ladder not strictly increasing: {m}")
            }
        }
    }
}

impl std::error::Error for IonizationLadderError {}

#[derive(Debug, Default, Deserialize, Serialize)]
struct LadderFile {
    #[serde(default)]
    ladder: Vec<LadderDef>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct LadderDef {
    symbol: String,
    #[serde(default)]
    ionization_energies_ev: Vec<String>,
    #[serde(default)]
    source: String,
}

/// The successive-ionization-energy ladder floor: per element symbol, the ordered ionization energies (eV).
#[derive(Debug, Clone, Default)]
pub struct IonizationLadder {
    by_symbol: BTreeMap<String, Vec<Fixed>>,
}

impl IonizationLadder {
    /// Load the ionization ladder from a TOML string. Every row must carry a citation and a strictly increasing
    /// ladder (the physical guard: each successive ionization energy exceeds the prior).
    pub fn from_toml_str(s: &str) -> Result<Self, IonizationLadderError> {
        let file: LadderFile =
            toml::from_str(s).map_err(|e| IonizationLadderError::Parse(e.to_string()))?;
        let mut by_symbol = BTreeMap::new();
        for entry in file.ladder {
            if entry.source.trim().is_empty() {
                return Err(IonizationLadderError::MissingSource(entry.symbol.clone()));
            }
            if entry.ionization_energies_ev.is_empty() {
                return Err(IonizationLadderError::NotMonotonic(format!(
                    "{} has an empty ladder",
                    entry.symbol
                )));
            }
            let mut energies = Vec::with_capacity(entry.ionization_energies_ev.len());
            for raw in &entry.ionization_energies_ev {
                let value = Fixed::from_decimal_str(raw.trim()).map_err(|d| {
                    IonizationLadderError::BadValue(format!("{} energy {raw}: {d}", entry.symbol))
                })?;
                energies.push(value);
            }
            // The physical reproduction-style guard: strictly increasing (each ionization harder than the last).
            for pair in energies.windows(2) {
                if pair[1] <= pair[0] {
                    return Err(IonizationLadderError::NotMonotonic(format!(
                        "{}: {} not greater than {}",
                        entry.symbol,
                        pair[1].to_f64_lossy(),
                        pair[0].to_f64_lossy()
                    )));
                }
            }
            if by_symbol.insert(entry.symbol.clone(), energies).is_some() {
                return Err(IonizationLadderError::Duplicate(entry.symbol));
            }
        }
        Ok(IonizationLadder { by_symbol })
    }

    /// The embedded standard ladder (`data/ionization_ladder.toml`).
    pub fn standard() -> Result<Self, IonizationLadderError> {
        Self::from_toml_str(include_str!("../data/ionization_ladder.toml"))
    }

    /// The `n`-th ionization energy (1-indexed: `n = 1` is the first) in eV, or `None` when the element is
    /// absent or its ladder does not reach `n`.
    pub fn ionization_energy(&self, symbol: &str, n: u32) -> Option<Fixed> {
        if n == 0 {
            return None;
        }
        self.by_symbol
            .get(symbol)
            .and_then(|ladder| ladder.get((n - 1) as usize))
            .copied()
    }

    /// The atomic Hubbard U of an ion of charge `q` (a positive integer), the differential ionization energy
    /// `U_atomic(q) = IE_{q+1} - IE_q`: the cost to remove the next electron from the `q`-plus ion minus the
    /// cost to have removed the last, the d-d Coulomb the Mott-Hubbard picture keys on. `None` when `q` is zero
    /// or the ladder does not reach `IE_{q+1}` (the classifier then escalates rather than fabricating a U).
    pub fn atomic_u(&self, symbol: &str, q: u32) -> Option<Fixed> {
        if q == 0 {
            return None;
        }
        let ie_q = self.ionization_energy(symbol, q)?;
        let ie_q_plus_1 = self.ionization_energy(symbol, q + 1)?;
        Some(ie_q_plus_1 - ie_q)
    }

    /// The number of elements with a seeded ladder.
    pub fn len(&self) -> usize {
        self.by_symbol.len()
    }

    /// Whether the ladder floor is empty.
    pub fn is_empty(&self) -> bool {
        self.by_symbol.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::periodic::PeriodicTable;

    fn ladder() -> IonizationLadder {
        IonizationLadder::standard().expect("the ionization ladder loads")
    }

    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_seed_ladder_is_the_3d_series() {
        let l = ladder();
        assert_eq!(l.len(), 10, "the seed is the 3d transition series Sc-Zn");
        for symbol in ["Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn"] {
            assert!(
                l.ionization_energy(symbol, 1).is_some(),
                "{symbol} carries its ladder"
            );
        }
    }

    #[test]
    fn the_atomic_u_is_the_mott_hubbard_differential() {
        // The spec's example: Ni2+ has U = IE3 - IE2 = 35.187 - 18.169 = 17.0 eV.
        let l = ladder();
        let u_ni = l.atomic_u("Ni", 2).expect("Ni2+ U computes");
        assert!(
            close(u_ni, 17.02, 0.1),
            "Ni2+ atomic U should be about 17 eV, got {}",
            u_ni.to_f64_lossy()
        );
        // The 3d monoxide cations all land in the 14 to 18 eV atomic-U band (screened in-crystal to ~7-8).
        for (symbol, expect) in [("Mn", 18.03), ("Fe", 14.45), ("Co", 16.42), ("Ni", 17.02)] {
            let u = l.atomic_u(symbol, 2).expect("2+ U computes");
            assert!(
                close(u, expect, 0.1),
                "{symbol}2+ atomic U should be about {expect} eV, got {}",
                u.to_f64_lossy()
            );
        }
    }

    #[test]
    fn the_atomic_u_falls_through_off_the_ladder() {
        // A charge whose IE_{q+1} is past the seeded depth (IE1-IE4) has no U: the classifier escalates rather
        // than fabricating one. IE5 is not seeded, so a 4+ ion (needs IE5) returns None.
        let l = ladder();
        assert!(
            l.atomic_u("Fe", 4).is_none(),
            "Fe4+ U needs IE5, not seeded"
        );
        assert!(
            l.atomic_u("Fe", 0).is_none(),
            "a zero charge has no differential U"
        );
        assert!(
            l.atomic_u("Xx", 2).is_none(),
            "an absent element has no ladder"
        );
    }

    #[test]
    fn the_first_rung_matches_the_periodic_table() {
        // The reproduction-style cross-check: the ladder's IE1 agrees with the periodic table's first-ionization
        // column (both are the NIST ground-state value), so the two floor sources are consistent, not divergent.
        let l = ladder();
        let table = PeriodicTable::standard().expect("the periodic table loads");
        for symbol in ["Fe", "Ni", "Cu"] {
            let ladder_ie1 = l.ionization_energy(symbol, 1).expect("ladder IE1");
            if let Some(table_ie1) = table.element(symbol).and_then(|e| e.ionization_energy) {
                assert!(
                    close(ladder_ie1, table_ie1.to_f64_lossy(), 0.01),
                    "{symbol} ladder IE1 {} should match the table IE1 {}",
                    ladder_ie1.to_f64_lossy(),
                    table_ie1.to_f64_lossy()
                );
            }
        }
    }

    #[test]
    fn a_non_increasing_ladder_is_rejected() {
        // The monotonicity guard fails loud on a physically impossible ladder (a later ionization cheaper than
        // an earlier one).
        let bad = r#"
[[ladder]]
symbol = "Zz"
ionization_energies_ev = ["10.0", "5.0"]
source = "test"
"#;
        assert!(matches!(
            IonizationLadder::from_toml_str(bad),
            Err(IonizationLadderError::NotMonotonic(_))
        ));
    }

    #[test]
    fn a_missing_citation_is_rejected() {
        let bad = r#"
[[ladder]]
symbol = "Zz"
ionization_energies_ev = ["5.0", "10.0"]
source = ""
"#;
        assert!(matches!(
            IonizationLadder::from_toml_str(bad),
            Err(IonizationLadderError::MissingSource(_))
        ));
    }
}
