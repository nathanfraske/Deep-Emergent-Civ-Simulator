//! The Badger / Herschbach-Laurie force-constant column, the second phonon-generator input (with the electronic
//! polarizability column). It turns a bond's equilibrium length into its STRETCHING force constant `k` by the
//! generalized Badger rule, `k [mdyn/Angstrom] = 10^((a_ij - r_e[Angstrom]) / b_ij)`, with `(a_ij, b_ij)` the
//! per-row-pair parameters cited `[M]` to Herschbach & Laurie UCRL-9694 (1961) Table III. A row is a periodic-table
//! period (row 0 = hydrogen, row 1 = period 2, ... row 5 = period 6); transition metals bonded to hydrogen or a
//! first-row atom take the separate `tm` parameters (partial multiple-bond character).
//!
//! The bond length `r_e` is the Pyykkö covalent-radius sum ([`crate::covalent_radii`]) for a covalent bond; an
//! IONIC bond routes through the Shannon column instead (the caller's dispatch, per the covalent scope tag). The
//! force constant feeds `omega_TO = sqrt(k / mu)` in the phonon generator, so this module also derives the reduced
//! mass `mu` from the periodic-table atomic weights. Closures against the measured diatomics: CO 19.1, SiO 9.16, N2
//! ~22 mdyn/Angstrom.

use crate::covalent_radii::CovalentRadii;
use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use serde::Deserialize;
use std::collections::BTreeMap;

/// What can go wrong loading the force-constant parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForceConstantError {
    /// The data could not be parsed as TOML.
    Parse(String),
    /// A row-pair appears twice.
    Duplicate(String),
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    #[serde(default)]
    pair: Vec<RawPair>,
}

#[derive(Debug, Deserialize)]
struct RawPair {
    row_i: u8,
    row_j: u8,
    a2: f64,
    b2: f64,
    #[serde(default)]
    tm: bool,
}

/// The Herschbach-Laurie force-constant column, keyed by the row-pair `(row_lo, row_hi, transition_metal)`.
#[derive(Debug, Clone, Default)]
pub struct ForceConstants {
    // (row_lo, row_hi, tm) -> (a2, b2), a2/b2 in Angstrom.
    params: BTreeMap<(u8, u8, bool), (Fixed, Fixed)>,
}

/// The H-L row of an element from its atomic number: row 0 is hydrogen (and the period-1 gases), otherwise the
/// periodic-table period minus one (period 2 -> row 1, ... period 6 -> row 5, period 7 -> row 6).
fn hl_row(z: u8) -> u8 {
    match z {
        0..=2 => 0,
        3..=10 => 1,
        11..=18 => 2,
        19..=36 => 3,
        37..=54 => 4,
        55..=86 => 5,
        _ => 6,
    }
}

/// Whether an element is a d-block transition metal (the `tm` parameter set applies to its bonds with hydrogen or a
/// first-row atom).
fn is_transition_metal(z: u8) -> bool {
    matches!(z, 21..=30 | 39..=48 | 72..=80)
}

impl ForceConstants {
    /// Parse and validate the parameter column from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self, ForceConstantError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| ForceConstantError::Parse(e.to_string()))?;
        let mut params = BTreeMap::new();
        for raw in file.pair {
            let lo = raw.row_i.min(raw.row_j);
            let hi = raw.row_i.max(raw.row_j);
            let key = (lo, hi, raw.tm);
            let a2 = Fixed::from_ratio((raw.a2 * 1_000_000.0).round() as i64, 1_000_000);
            let b2 = Fixed::from_ratio((raw.b2 * 1_000_000.0).round() as i64, 1_000_000);
            if params.insert(key, (a2, b2)).is_some() {
                return Err(ForceConstantError::Duplicate(format!(
                    "{lo}-{hi} tm={}",
                    raw.tm
                )));
            }
        }
        Ok(ForceConstants { params })
    }

    /// Load the standard Herschbach-Laurie column from the checked-in data file.
    pub fn standard() -> Result<Self, ForceConstantError> {
        Self::from_toml_str(include_str!("../data/badger_hl_parameters.toml"))
    }

    /// The `(a2, b2)` parameters for a bond between two elements (by atomic number), selecting the transition-metal
    /// row when one atom is a d-block metal and the other is hydrogen or a first-row atom, else the plain row pair.
    fn parameters(&self, a_z: u8, b_z: u8) -> Option<(Fixed, Fixed)> {
        let (ra, rb) = (hl_row(a_z), hl_row(b_z));
        let (lo, hi) = (ra.min(rb), ra.max(rb));
        let tm_involved = is_transition_metal(a_z) || is_transition_metal(b_z);
        if tm_involved {
            if let Some(p) = self.params.get(&(lo, hi, true)) {
                return Some(*p);
            }
        }
        self.params.get(&(lo, hi, false)).copied()
    }

    /// The stretching force constant `k` (mdyn/Angstrom) of a bond between elements `a_z`, `b_z` at equilibrium
    /// length `r_e` (Angstrom): `k = 10^((a2 - r_e)/b2)`. `None` if the row pair has no parameters.
    pub fn force_constant_mdyn_per_angstrom(
        &self,
        a_z: u8,
        b_z: u8,
        r_e_angstrom: Fixed,
    ) -> Option<Fixed> {
        let (a2, b2) = self.parameters(a_z, b_z)?;
        let exponent = a2.checked_sub(r_e_angstrom)?.checked_div(b2)?;
        Some(exponent.checked_mul(Fixed::from_int(10).ln())?.exp())
    }

    /// The force constant of a COVALENT bond, reading `r_e` as the Pyykkö covalent-radius sum at the bond order
    /// (1/2/3). `None` if the radii, the row pair, or the element data are unavailable.
    pub fn covalent_bond_force_constant(
        &self,
        radii: &CovalentRadii,
        table: &PeriodicTable,
        a: &str,
        b: &str,
        order: u8,
    ) -> Option<Fixed> {
        let r_e_pm = radii.bond_length_pm(a, b, order)?;
        let r_e_angstrom = Fixed::from_ratio(r_e_pm as i64, 100); // pm -> Angstrom
        let a_z = table.element(a)?.z;
        let b_z = table.element(b)?.z;
        self.force_constant_mdyn_per_angstrom(a_z, b_z, r_e_angstrom)
    }

    /// The reduced mass `mu = m_a m_b / (m_a + m_b)` (atomic mass units) of a bonded pair, from the periodic-table
    /// standard atomic weights, for the phonon generator's `omega = sqrt(k/mu)`. `None` if an element is absent.
    pub fn reduced_mass_amu(table: &PeriodicTable, a: &str, b: &str) -> Option<Fixed> {
        let ma = table.element(a)?.standard_atomic_weight;
        let mb = table.element(b)?.standard_atomic_weight;
        ma.checked_mul(mb)?.checked_div(ma.checked_add(mb)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col() -> ForceConstants {
        ForceConstants::standard().expect("the force-constant column loads")
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn the_co_and_sio_closures_reproduce_the_measured_force_constants() {
        // The pinned Badger closures at the MEASURED bond lengths: CO (row pair 1-1) at r_e = 1.128 A gives 19.1
        // mdyn/A (measured ~19.0); SiO (1-2) at 1.510 A gives 9.16 (measured ~9.24); N2 (1-1) at 1.098 A gives ~22
        // (measured ~22.4). Carbon Z=6, oxygen Z=8, silicon Z=14, nitrogen Z=7.
        let c = col();
        let co = c
            .force_constant_mdyn_per_angstrom(6, 8, Fixed::from_ratio(1128, 1000))
            .unwrap();
        assert!(
            close(co.to_f64_lossy(), 19.1, 0.4),
            "CO force constant ~19 mdyn/A, got {}",
            co.to_f64_lossy()
        );
        let sio = c
            .force_constant_mdyn_per_angstrom(14, 8, Fixed::from_ratio(1510, 1000))
            .unwrap();
        assert!(
            close(sio.to_f64_lossy(), 9.16, 0.3),
            "SiO ~9.2 mdyn/A, got {}",
            sio.to_f64_lossy()
        );
        let n2 = c
            .force_constant_mdyn_per_angstrom(7, 7, Fixed::from_ratio(1098, 1000))
            .unwrap();
        assert!(
            close(n2.to_f64_lossy(), 22.4, 0.8),
            "N2 ~22 mdyn/A, got {}",
            n2.to_f64_lossy()
        );
    }

    #[test]
    fn the_covalent_bond_path_reads_the_pyykko_length() {
        // The lattice path: r_e from the Pyykko covalent-radius sum. Si-O single bond (116 + 63 = 179 pm = 1.79 A)
        // is a longer, softer bond than the diatomic SiO, so its force constant is lower, still positive and in the
        // few-mdyn/A range.
        let radii = CovalentRadii::standard().unwrap();
        let table = PeriodicTable::standard().unwrap();
        let k = col()
            .covalent_bond_force_constant(&radii, &table, "Si", "O", 1)
            .unwrap();
        assert!(
            k.to_f64_lossy() > 0.5 && k.to_f64_lossy() < 9.0,
            "the Si-O single-bond force constant is a few mdyn/A, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_reduced_mass_lands_the_known_pairs() {
        let table = PeriodicTable::standard().unwrap();
        // mu(CO) = 12.011*15.999/(12.011+15.999) = 6.86 amu.
        let mu = ForceConstants::reduced_mass_amu(&table, "C", "O").unwrap();
        assert!(
            close(mu.to_f64_lossy(), 6.86, 0.02),
            "reduced mass of CO ~6.86 amu, got {}",
            mu.to_f64_lossy()
        );
    }

    #[test]
    fn the_transition_metal_and_missing_pairs_dispatch() {
        let c = col();
        // Fe (Z=26, a d-block metal) bonded to O (first-row) takes the tm=true 1-3 parameters (a2=1.98, b2=0.44),
        // distinct from the plain 1-3 (a2=2.15, b2=0.60): at the same r_e the tm parameters give a different k.
        let plain = c.force_constant_mdyn_per_angstrom(20, 8, Fixed::from_ratio(18, 10)); // Ca(Z20)-O, plain 1-3
        let tm = c.force_constant_mdyn_per_angstrom(26, 8, Fixed::from_ratio(18, 10)); // Fe(Z26)-O, tm 1-3
        assert!(plain.is_some() && tm.is_some());
        assert!(
            (plain.unwrap().to_f64_lossy() - tm.unwrap().to_f64_lossy()).abs() > 0.5,
            "the transition-metal row gives a distinct force constant"
        );
    }
}
