//! The Pyykkö covalent-radius column (`crates/physics/data/covalent_radii_pyykko.toml`), the covalent bond-length
//! substrate the Badger force-constant column sums: a bond's equilibrium length is the sum of the two atoms'
//! covalent radii at the bond order, `r_e = r_a + r_b`. Cited [M] (Pyykkö 2015, J. Phys. Chem. A 119, 2326, over
//! the Pyykkö & Atsumi 2009 and Pyykkö-Riedel-Patzschke 2005 primaries), the single-bond fit good to a
//! mean-square deviation under 3 pm.
//!
//! SCOPE (the definition tag, enforced by the caller's dispatch, not this loader): agreement is good only when the
//! bond is not too ionic and the coordination is near the fit's input; an IONIC bond routes through the Shannon
//! ionic-radius column instead (the ionic-dispatch line). Two length sets: the MOLECULAR single/double/triple radii,
//! and the TETRAHEDRAL-crystal radii (Pyykkö 2015 Figure 2, the ~30 elements that form tetrahedral crystals) that
//! estimate a solid-lattice bond length additively. The tetrahedral set is the production chain's alien length
//! rung; the calibration battery reads MEASURED crystal geometry instead, so generator error is never convolved
//! with the length estimator's error (a second battery pass grades the estimator through the tetrahedral sum).

use civsim_core::Fixed;
use civsim_units::bignum::BigRat;
use civsim_units::fundamentals;
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
/// double and triple are absent for elements the fit did not cover at that order; the tetrahedral radius is present
/// only for the ~30 elements that form tetrahedrally-bonded crystals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CovalentRadius {
    /// Single-bond covalent radius (pm).
    pub single_pm: i32,
    /// Double-bond covalent radius (pm), if the fit covered it.
    pub double_pm: Option<i32>,
    /// Triple-bond covalent radius (pm), if the fit covered it.
    pub triple_pm: Option<i32>,
    /// Tetrahedral-crystal covalent radius (pm, sub-picometre), if the element forms tetrahedral crystals.
    pub tetrahedral_pm: Option<Fixed>,
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

/// The provenance GRADE of a covalent radius, the mixed-grade citation this cited column carries. Pyykko's
/// light-element radii are FITS to measured bond distances (`MeasuredFit`); the heavy, actinide-and-beyond radii are
/// RELATIVISTIC COMPUTED estimates (`RelativisticComputed`, a compute-once cited grade, not measured [M]). The two
/// share one citation file, so a grade-sensitive consumer must not read the computed region as if it were measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadiusGrade {
    /// A fit to measured bond distances (the light rows).
    MeasuredFit,
    /// A relativistic computed estimate (the heavy, actinide-and-beyond rows).
    RelativisticComputed,
}

/// The loaded covalent-radius column, keyed by element symbol.
#[derive(Debug, Clone, Default)]
pub struct CovalentRadii {
    radii: BTreeMap<String, CovalentRadius>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFile {
    // `[[radius]]`, the cited-data-column block kind (the Shannon ionic-radius idiom), NOT the reserved floor
    // `[[element]]` kind: a cited [M] import never participates in the floor's real/fantasy authorship axis, and an
    // immutable citation file never receives estimator-tier rows for authored elements (co-location laundering).
    #[serde(default)]
    radius: Vec<RawRadius>,
}

#[derive(Debug, Deserialize)]
struct RawRadius {
    symbol: String,
    single_pm: i32,
    #[serde(default)]
    double_pm: Option<i32>,
    #[serde(default)]
    triple_pm: Option<i32>,
    #[serde(default)]
    tetrahedral_pm: Option<f64>,
}

impl CovalentRadii {
    /// Parse and validate the column from a TOML string. Every element carries a positive single-bond radius, and no
    /// symbol repeats.
    pub fn from_toml_str(s: &str) -> Result<Self, CovalentRadiiError> {
        let file: RawFile =
            toml::from_str(s).map_err(|e| CovalentRadiiError::Parse(e.to_string()))?;
        let mut radii = BTreeMap::new();
        for raw in file.radius {
            if raw.single_pm <= 0
                || raw.double_pm.is_some_and(|d| d <= 0)
                || raw.triple_pm.is_some_and(|t| t <= 0)
            {
                return Err(CovalentRadiiError::NotPhysical(raw.symbol));
            }
            let tetrahedral_pm = raw
                .tetrahedral_pm
                .map(|v| Fixed::from_ratio((v * 10.0).round() as i64, 10));
            let entry = CovalentRadius {
                single_pm: raw.single_pm,
                double_pm: raw.double_pm,
                triple_pm: raw.triple_pm,
                tetrahedral_pm,
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

    /// The grade of an element's radius, classified by atomic number against the measured-computed boundary: at or
    /// below the boundary it is a `MeasuredFit`, above it a `RelativisticComputed` estimate. The boundary is a
    /// data-provenance fact (Pyykko 2015's own fit-versus-computed split, read from the paper's data-source notes,
    /// flagged until sourced), supplied by the caller rather than read from a world-content manifest.
    pub fn grade(atomic_number: u32, measured_grade_boundary_z: u32) -> RadiusGrade {
        if atomic_number <= measured_grade_boundary_z {
            RadiusGrade::MeasuredFit
        } else {
            RadiusGrade::RelativisticComputed
        }
    }

    /// The Pyykko-sum bond length, GUARDED to the measured grade: `None` (escalate) if either atom's radius is a
    /// relativistic computed estimate (its atomic number exceeds the boundary), so a consumer that requires
    /// measured-grade lengths (the force-constant chain, the grain lattices) never silently ingests a computed
    /// radius as if it were measured. `z_of` resolves an element symbol to its atomic number (the caller's periodic
    /// table, keeping this column decoupled from it); `measured_grade_boundary_z` is the reserved boundary. This
    /// guard costs nothing today (every current consumer reads only light rows) and fails loud the day a consumer
    /// reads the computed region (einsteinium optics).
    pub fn bond_length_measured_grade(
        &self,
        a: &str,
        b: &str,
        order: u8,
        measured_grade_boundary_z: u32,
        z_of: impl Fn(&str) -> Option<u32>,
    ) -> Option<i32> {
        for sym in [a, b] {
            if Self::grade(z_of(sym)?, measured_grade_boundary_z) != RadiusGrade::MeasuredFit {
                return None; // escalate: a computed-grade radius, not measured
            }
        }
        self.bond_length_pm(a, b, order)
    }

    /// The tetrahedral-crystal bond length `r_e = r_tet(a) + r_tet(b)` (pm), the length estimator for a
    /// tetrahedrally-bonded crystal (the production chain's alien rung when no crystal has been measured). Additive
    /// for tetrahedral crystals (SiC lands ~195 pm vs the measured 188.9); a strongly polar bond (silicate Si-O)
    /// needs an ionicity correction the sum omits, which the second battery pass measures as the estimator's honest
    /// band. `None` if either element lacks a tetrahedral radius.
    pub fn tetrahedral_bond_length_pm(&self, a: &str, b: &str) -> Option<Fixed> {
        let ra = self.radius(a)?.tetrahedral_pm?;
        let rb = self.radius(b)?.tetrahedral_pm?;
        ra.checked_add(rb)
    }

    /// BAND 4, the Schomaker-Stevenson partially-ionic correction on the tetrahedral-crystal radius sum:
    /// `r_AB = r_tet(a) + r_tet(b) - 9 |Delta-chi_P|` pm (Schomaker & Stevenson, J. Am. Chem. Soc. 63, 37 (1941)).
    /// The `9 pm / |Delta-chi|` is ONE class-universal constant, fit by Schomaker and Stevenson on a broad
    /// bond-length compilation that never saw this battery (the legality line: we fit nothing, the constant is
    /// class-universal). It shortens the polar-bond length the additive covalent sum over-estimates, because the
    /// more electronegative atom draws charge and contracts the bond. `Delta-chi_P` is Pauling's OWN thermochemical
    /// electronegativity difference (see [`pauling_electronegativity_difference`]), the scale the 9 pm constant was
    /// published against, so no cross-scale conversion enters. SiC (194.9 pm sum, `Delta-chi_P ~ 0.59-0.65`) lands
    /// ~189 pm against the measured 188.9, closing the pass-2 length band to the few-picometre class grade a 1941
    /// two-constant rule earns (never the decimal). `None` if either element lacks a tetrahedral radius or on
    /// overflow.
    pub fn schomaker_stevenson_tetrahedral_length_pm(
        &self,
        a: &str,
        b: &str,
        delta_chi_pauling: Fixed,
    ) -> Option<Fixed> {
        let sum = self.tetrahedral_bond_length_pm(a, b)?;
        let shift = Fixed::from_int(9).checked_mul(delta_chi_pauling.abs())?;
        sum.checked_sub(shift)
    }
}

/// The energy conversion `1 eV = e * N_A = 96.485 kJ/mol` (the Faraday constant per volt), DERIVED from the register
/// (`e`, `N_A`), so Pauling's thermochemical identity reads dissociation energies in the repo's kJ/mol with no bare
/// literal. `None` if a fundamental fails to resolve or the value leaves the representable range.
fn kj_per_mol_per_ev() -> Option<Fixed> {
    let e = BigRat::from_decimal_str(fundamentals::fundamental("e")?.value).ok()?;
    let n_a = BigRat::from_decimal_str(fundamentals::fundamental("N_A")?.value).ok()?;
    // e * N_A is J/mol per eV; / 1000 -> kJ/mol per eV.
    let v = e.mul(&n_a).div(&BigRat::from_i64(1000));
    Fixed::from_bits_i128(v.round_to_scale(Fixed::FRAC_BITS)?)
}

/// BAND 4, Pauling's thermochemical electronegativity DIFFERENCE from single-bond dissociation energies:
/// `|chi_A - chi_B| = sqrt(Delta_AB / eV)`, with the extra-ionic ("resonance") energy
/// `Delta_AB = D(AB) - (1/2)[D(AA) + D(BB)]` (Pauling, J. Am. Chem. Soc. 54, 3570 (1932), the arithmetic-mean form).
/// The three bond dissociation energies are in kJ/mol; the eV conversion is the register-derived Faraday constant
/// ([`kj_per_mol_per_ev`]). This lands `chi_P` on Pauling's OWN thermochemical scale, the reason to DERIVE it rather
/// than rescale a Mulliken `chi = (IE + EA)/2`: the scales measure different physics, so a conversion would smuggle
/// a fit (the Allred-Rochow lesson), and the Schomaker-Stevenson 9 pm constant was published against this scale.
///
/// The geometric-mean variant `Delta_AB = D(AB) - sqrt(D(AA) D(BB))` is Pauling's later refinement, FLAGGED not
/// built (build arithmetic first). HONEST LIMIT: for a near-nonpolar bond the arithmetic-mean postulate can fail
/// (`D(AB)` below the homonuclear mean, so `Delta_AB < 0`); this returns `None` there rather than fabricate an
/// imaginary root (Si-H is the worked case, which is why Si's `chi_P` anchors through a polar bond, not hydrogen).
/// `None` also on a bad conversion or overflow.
pub fn pauling_electronegativity_difference(
    d_ab_kj_per_mol: Fixed,
    d_aa_kj_per_mol: Fixed,
    d_bb_kj_per_mol: Fixed,
) -> Option<Fixed> {
    let mean = d_aa_kj_per_mol
        .checked_add(d_bb_kj_per_mol)?
        .checked_div(Fixed::from_int(2))?;
    let delta = d_ab_kj_per_mol.checked_sub(mean)?;
    if delta < Fixed::ZERO {
        return None; // the arithmetic-mean postulate fails; escalate, never an imaginary root
    }
    let delta_ev = delta.checked_div(kj_per_mol_per_ev()?)?;
    Some(delta_ev.sqrt())
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
    fn the_tetrahedral_crystal_radii_load() {
        // The alien-rung length estimator (Pyykko 2015 Figure 2): Si = 117.6, C = 77.3 pm, so the tetrahedral Si-C
        // sum is 194.9 pm (vs the measured 188.9, the pre-registered production-chain band). Only tetrahedral-crystal
        // formers carry a tetrahedral radius; sodium does not.
        let c = col();
        let si = c.radius("Si").unwrap().tetrahedral_pm.unwrap();
        assert!(
            (si.to_f64_lossy() - 117.6).abs() < 0.05,
            "Si tetrahedral 117.6, got {}",
            si.to_f64_lossy()
        );
        let sic = c.tetrahedral_bond_length_pm("Si", "C").unwrap();
        assert!(
            (sic.to_f64_lossy() - 194.9).abs() < 0.1,
            "the tetrahedral Si-C length is 194.9 pm, got {}",
            sic.to_f64_lossy()
        );
        assert!(
            c.radius("Na").unwrap().tetrahedral_pm.is_none(),
            "Na forms no tetrahedral crystal"
        );
        assert!(c.tetrahedral_bond_length_pm("Na", "Cl").is_none());
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
    fn the_measured_grade_guard_escalates_on_the_computed_region() {
        // The mixed-grade guard: a light bond (both atoms measured-fit grade) passes; a bond touching the
        // relativistic-computed region (an atom above the boundary) ESCALATES (None), so a measured-grade consumer
        // never silently ingests a computed radius. The real boundary is a data-provenance fact flagged until read
        // from Pyykko 2015; here it is a test boundary at Z = 83 (bismuth, the last stable element).
        let c = col();
        let z_of = |s: &str| match s {
            "C" => Some(6),
            "O" => Some(8),
            "U" => Some(92),
            _ => None,
        };
        let boundary = 83u32;
        assert_eq!(
            c.bond_length_measured_grade("C", "O", 3, boundary, z_of),
            Some(113),
            "a light measured-grade bond passes the guard"
        );
        assert!(
            c.bond_length_measured_grade("C", "U", 1, boundary, z_of)
                .is_none(),
            "a bond into the computed region (uranium, Z 92 > 83) escalates, never a silent computed radius"
        );
        assert_eq!(
            CovalentRadii::grade(92, boundary),
            RadiusGrade::RelativisticComputed
        );
        assert_eq!(CovalentRadii::grade(8, boundary), RadiusGrade::MeasuredFit);
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

    // BAND 4 acceptance. The single-bond dissociation energies (kJ/mol) are CITED literature fixtures, standard
    // thermochemical single-bond values (Pauling, "The Nature of the Chemical Bond", 3rd ed. 1960; Cottrell,
    // "The Strengths of Chemical Bonds", 1958; Darwent, NSRDS-NBS 31, 1970): H-H 436, C-C 346, Si-Si 222, Si-C 318,
    // C-H 413, Si-H 318, O-O 146, O-H 463. INPUT-AUDIT SEAM (reported): the repo's atomization column supplies only
    // D(H-H) = 2*atomization(H) = 436 cleanly; C/O/N atomization are graphite sublimation, half the O=O double, and
    // half the N triple, none the single-bond D-value, and no heteronuclear D-value is in the repo. A production
    // chi_P column needs a vendored single-bond D-value table (a named small build), so these live as cited [M]
    // fixtures, never seeded from memory.
    const D_HH: i32 = 436;
    const D_CC: i32 = 346;
    const D_SISI: i32 = 222;
    const D_SIC: i32 = 318;
    const D_CH: i32 = 413;
    const D_SIH: i32 = 318;
    const D_OO: i32 = 146;
    const D_OH: i32 = 463;

    #[test]
    fn the_band_four_pauling_difference_reproduces_the_scale_at_class_grade() {
        // Pauling's identity |chi_A - chi_B| = sqrt(Delta/eV), Delta = D(AB) - (1/2)[D(AA)+D(BB)]. Anchored at
        // chi_H = 2.20, the derived values reproduce the standard Pauling table (C 2.55, O 3.44) to the few-tenths
        // CLASS grade of the arithmetic-mean scheme, never the decimal.
        let d = |z: i32| Fixed::from_int(z);
        // chi_C - chi_H: sqrt((413 - (346+436)/2)/96.485) = sqrt(22/96.485) = 0.478; chi_C ~ 2.68 vs table 2.55.
        let dc_h = pauling_electronegativity_difference(d(D_CH), d(D_CC), d(D_HH)).unwrap();
        let chi_c = 2.20 + dc_h.to_f64_lossy();
        assert!(
            (chi_c - 2.55).abs() < 0.15,
            "derived chi_C ~ 2.55 at class grade, got {chi_c}"
        );
        // chi_O - chi_H: sqrt((463 - (146+436)/2)/96.485) = sqrt(172/96.485) = 1.335; chi_O ~ 3.54 vs table 3.44.
        let do_h = pauling_electronegativity_difference(d(D_OH), d(D_OO), d(D_HH)).unwrap();
        let chi_o = 2.20 + do_h.to_f64_lossy();
        assert!(
            (chi_o - 3.44).abs() < 0.15,
            "derived chi_O ~ 3.44 at class grade, got {chi_o}"
        );
        // The Si-H HONEST LIMIT: Delta(Si,H) = 318 - (222+436)/2 = -11 < 0, the arithmetic-mean postulate fails for
        // the near-nonpolar Si-H bond, so the identity returns None (escalate) rather than an imaginary root. Si's
        // chi_P must anchor through a polar bond (Si-C below), not hydrogen.
        assert!(
            pauling_electronegativity_difference(d(D_SIH), d(D_SISI), d(D_HH)).is_none(),
            "Si-H below the homonuclear mean returns None (no fabricated imaginary root)"
        );
    }

    #[test]
    fn the_band_four_schomaker_stevenson_closes_the_sic_length_band() {
        // The pass-2 SiC length re-grade. Delta-chi_P(Si,C) is DERIVED from the Si-C/Si-Si/C-C dissociation energies
        // (not looked up): sqrt((318 - (222+346)/2)/96.485) = sqrt(34/96.485) = 0.594 (vs the standard-table 0.65,
        // within the class grade). Schomaker-Stevenson then shortens the 194.9 pm additive sum by 9*0.594 = 5.3 pm to
        // ~189.6, against the measured 188.9. The acceptance is the FEW-PICOMETRE class grade of a 1941 two-constant
        // rule, NOT the 189.0 decimal (one lucky point must not masquerade as the estimator's band).
        let radii = col();
        let d = |z: i32| Fixed::from_int(z);
        let delta_chi = pauling_electronegativity_difference(d(D_SIC), d(D_SISI), d(D_CC)).unwrap();
        assert!(
            (delta_chi.to_f64_lossy() - 0.65).abs() < 0.1,
            "derived Delta-chi_P(Si,C) ~ 0.65 at class grade, got {}",
            delta_chi.to_f64_lossy()
        );
        let corrected = radii
            .schomaker_stevenson_tetrahedral_length_pm("Si", "C", delta_chi)
            .unwrap()
            .to_f64_lossy();
        let uncorrected = radii
            .tetrahedral_bond_length_pm("Si", "C")
            .unwrap()
            .to_f64_lossy();
        let measured = 188.9;
        assert!(
            (corrected - measured).abs() < 3.0,
            "the Schomaker-Stevenson SiC length lands within the few-pm class grade of 188.9, got {corrected}"
        );
        assert!(
            (corrected - measured).abs() < (uncorrected - measured).abs(),
            "the correction (over)closes the +6 pm additive miss ({corrected} vs sum {uncorrected}, measured {measured})"
        );
    }
}
