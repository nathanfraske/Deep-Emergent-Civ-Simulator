//! The Pyykkö covalent-radius column (`crates/physics/data/covalent_radii_pyykko.toml`), the covalent bond-length
//! substrate the Badger force-constant column sums: a bond's equilibrium length is the sum of the two atoms'
//! covalent radii at the bond order, `r_e = r_a + r_b`. Cited [M] (Pyykkö 2015, J. Phys. Chem. A 119, 2326, over
//! the Pyykkö & Atsumi 2009 and Pyykkö-Riedel-Patzschke 2005 primaries), the single-bond fit good to a
//! mean-square deviation under 3 pm.
//!
//! SCOPE (the definition tag, enforced by the caller's dispatch, not this loader): agreement is good only when the
//! bond is not too ionic and the coordination is near the fit's input; an IONIC bond routes through the Shannon
//! ionic-radius column instead (the ionic-dispatch line). These are MOLECULAR bond radii; the tetrahedral-crystal
//! set that fits solid lattices to subpicometre accuracy is a pending refinement (a flagged SI fetch), so grain-
//! lattice bond lengths use the molecular radii as a first cut.

use serde::Deserialize;
use std::collections::BTreeMap;

/// What can go wrong loading the covalent-radius column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CovalentRadiiError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A symbol appears twice.
    Duplicate(String),
    /// A radius is non-positive (a covalent radius must be a positive length).
    NotPhysical(String),
}

/// The covalent radii of one element at each bond order (picometres). The single-bond radius is always present;
/// double and triple are absent for elements the fit did not cover at that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CovalentRadius {
    /// Single-bond covalent radius (pm).
    pub single_pm: i32,
    /// Double-bond covalent radius (pm), if the fit covered it.
    pub double_pm: Option<i32>,
    /// Triple-bond covalent radius (pm), if the fit covered it.
    pub triple_pm: Option<i32>,
}

impl CovalentRadius {
    /// The radius at a bond order (1, 2, or 3), or `None` if the order is unsupported or the element lacks a value
    /// there.
    pub fn at_order(&self, order: u8) -> Option<i32> {
        match order {
            1 => Some(self.single_pm),
            2 => self.double_pm,
            3 => self.triple_pm,
            _ => None,
        }
    }
}

/// The loaded covalent-radius column, keyed by element symbol.
#[derive(Debug, Clone, Default)]
pub struct CovalentRadii {
    radii: BTreeMap<String, CovalentRadius>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    element: Vec<RawElement>,
}

#[derive(Debug, Deserialize)]
struct RawElement {
    symbol: String,
    single_pm: i32,
    #[serde(default)]
    double_pm: Option<i32>,
    #[serde(default)]
    triple_pm: Option<i32>,
}

impl CovalentRadii {
    /// Parse and validate the column from a TOML string. Every element carries a positive single-bond radius, and no
    /// symbol repeats.
    pub fn from_toml_str(s: &str) -> Result<Self, CovalentRadiiError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| CovalentRadiiError::Parse(e.to_string()))?;
        let mut radii = BTreeMap::new();
        for raw in file.element {
            if raw.single_pm <= 0
                || raw.double_pm.is_some_and(|d| d <= 0)
                || raw.triple_pm.is_some_and(|t| t <= 0)
            {
                return Err(CovalentRadiiError::NotPhysical(raw.symbol));
            }
            let entry = CovalentRadius {
                single_pm: raw.single_pm,
                double_pm: raw.double_pm,
                triple_pm: raw.triple_pm,
            };
            if radii.insert(raw.symbol.clone(), entry).is_some() {
                return Err(CovalentRadiiError::Duplicate(raw.symbol));
            }
        }
        Ok(CovalentRadii { radii })
    }

    /// Load the standard Pyykkö column from the checked-in data file.
    pub fn standard() -> Result<Self, CovalentRadiiError> {
        Self::from_toml_str(include_str!("../data/covalent_radii_pyykko.toml"))
    }

    /// The covalent radii of an element, or `None` if it is not in the column.
    pub fn radius(&self, symbol: &str) -> Option<&CovalentRadius> {
        self.radii.get(symbol)
    }

    /// The Pyykkö-sum equilibrium bond length `r_e = r_a + r_b` (pm) between two elements at a bond order (1/2/3),
    /// or `None` if either element or its radius at that order is absent. The CO triple bond (`60 + 53 = 113 pm`
    /// against the measured `112.8 pm`) is the closure check.
    pub fn bond_length_pm(&self, a: &str, b: &str, order: u8) -> Option<i32> {
        let ra = self.radius(a)?.at_order(order)?;
        let rb = self.radius(b)?.at_order(order)?;
        Some(ra + rb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col() -> CovalentRadii {
        CovalentRadii::standard().expect("the covalent-radius column loads")
    }

    #[test]
    fn the_column_loads_the_pyykko_single_bond_radii() {
        let c = col();
        assert_eq!(
            c.radius("H").unwrap().single_pm,
            32,
            "Pyykko H single-bond radius"
        );
        assert_eq!(c.radius("C").unwrap().single_pm, 75);
        assert_eq!(c.radius("O").unwrap().single_pm, 63);
        assert_eq!(c.radius("Si").unwrap().single_pm, 116);
        assert_eq!(c.radius("Fe").unwrap().single_pm, 116);
        assert!(
            c.radius("H").unwrap().double_pm.is_none(),
            "H has no double-bond radius"
        );
    }

    #[test]
    fn the_co_triple_bond_sum_lands_the_closure() {
        // The pinned closure: the CO triple bond, C (60) + O (53) = 113 pm, against the measured 112.8 pm.
        let c = col();
        assert_eq!(
            c.bond_length_pm("C", "O", 3),
            Some(113),
            "the CO triple-bond Pyykko sum is 113 pm (measured 112.8)"
        );
    }

    #[test]
    fn the_orders_and_missing_elements_are_handled() {
        let c = col();
        // A single bond always resolves; a triple bond for an alkali (no triple radius) does not.
        assert!(c.bond_length_pm("Na", "Cl", 1).is_some());
        assert_eq!(
            c.bond_length_pm("Na", "Na", 3),
            None,
            "Na has no triple-bond radius"
        );
        assert_eq!(
            c.bond_length_pm("C", "Xx", 1),
            None,
            "an unknown element is absent"
        );
        assert_eq!(
            c.radius("C").unwrap().at_order(4),
            None,
            "order 4 is unsupported"
        );
    }
}
