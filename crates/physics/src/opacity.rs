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

//! The disk-opacity GENERATOR (capstone front-end 3c-i, the owner-redirected build): the Rosseland-mean opacity
//! `kappa_R(T, rho)` DERIVED from physics rather than read from a fitted ladder. The owner held the Bell-Lin 1994
//! and Semenov 2003 piecewise fits permanently (they are solar-composition compressions that bake one grain model
//! and fixed regime boundaries into dimensional coefficients, violating the dimensionless-constant law and
//! admit-the-alien); Bell-Lin/Semenov re-enter only as the validation battery (`OPACITY_VALIDATION_BATTERY.md`).
//! The generator is `kappa_R = Rosseland . Mie . (optical constants x size distribution x mixing rule x condensate
//! fractions) + gas terms`, built as a multi-slice arc.
//!
//! This module holds the gas/plasma terms, which derive to the digit from the fundamentals with no fetched
//! coefficient. The first is ELECTRON SCATTERING ([`electron_scattering_opacity`]): the Thomson value the whole
//! ladder tops out at, reassembled from the Thomson cross section and a hydrogen mass fraction. The Mie grain
//! terms (which read the measured optical constants, `OPACITY_OPTICAL_CONSTANTS_SOURCES.md`) and the Rosseland
//! assembly are the later slices.

use crate::periodic::PeriodicTable;
use civsim_core::Fixed;
use civsim_units::bignum::{BigRat, BigUint};
use civsim_units::compute;
use civsim_units::fundamentals;

/// The decimal digits of pi the opacity computation carries, far above the ~10 significant figures a Q32.32
/// result holds, so the pi truncation never reaches the result's low bit. An engine-accuracy bound.
const OPACITY_PI_DIGITS: u32 = 40;

/// A non-negative `Fixed` as an exact rational (its bits over `2^FRAC_BITS`), so an order-one `Fixed` argument
/// enters the wide-magnitude `BigRat` computation without leaving exact arithmetic. The callers pass non-negative
/// values (a mass fraction in `[0, 1]`, a molar mass).
fn nonneg_fixed_to_bigrat(value: Fixed) -> BigRat {
    let bits = value.to_bits().max(0) as u64;
    BigRat::new(
        false,
        BigUint::from_u64(bits),
        BigUint::from_u64(1).shl_bits(Fixed::FRAC_BITS),
    )
}

/// A fundamental constant's CODATA value as an exact rational, read from the single fundamentals register (never
/// re-authored here). `None` if the symbol is not a registered fundamental.
fn fundamental_bigrat(symbol: &str) -> Option<BigRat> {
    BigRat::from_decimal_str(fundamentals::fundamental(symbol)?.value).ok()
}

/// The ELECTRON-SCATTERING (Thomson) opacity `kappa_es` (cm^2/g), DERIVED from the fundamentals and the periodic
/// table, never fetched from an opacity fit. Free electrons scatter photons with the Thomson cross section, so the
/// opacity per unit mass is `kappa_es = sigma_T * n_e / rho = sigma_T * (1 + X) / (2 * m_H)`, where `X` is the
/// hydrogen mass fraction and the `(1 + X)/2` factor is the electrons per nucleon of an ionized hydrogen-helium
/// plasma (pure hydrogen contributes one electron per proton, helium two per four nucleons). It is constant in
/// density and temperature (the top regime of the opacity ladder).
///
/// Every constant is DERIVED, nothing fetched: the Thomson cross section is
/// `sigma_T = (8*pi/3) * r_e^2` with the classical electron radius `r_e = e^2 / (4*pi*eps_0*m_e*c^2)`, so it reads
/// only the fundamentals `e`, `eps_0`, `m_e`, `c` (the electron mass `m_e` is the eighth register fundamental,
/// reached by exactly this term); the mass per hydrogen `m_H = M_H / N_A` reads the periodic-table molar mass of
/// hydrogen and the Avogadro fundamental. So the Bell-Lin ladder's top value `0.348` (their `0.2(1 + X)` rounded)
/// is reassembled here from atomic principles as `0.1989(1 + X)`, the more precise coefficient. `X` is the only
/// per-world input (the admit-the-alien seam): a hydrogen-poor plasma is a data row, a lower `kappa_es`, never a
/// rewrite.
///
/// The wide-magnitude computation (`sigma_T ~ 6.65e-29 m^2` and `m_H ~ 1.67e-27 kg` underflow Q32.32 while the
/// ~0.35 cm^2/g result fits) runs in exact `BigRat` and rounds once; the `m^2/kg -> cm^2/g` conversion is the
/// exact factor 10. `None` if a fundamental or the hydrogen molar mass fails to resolve, or the result leaves the
/// representable range.
pub fn electron_scattering_opacity(
    hydrogen_mass_fraction: Fixed,
    table: &PeriodicTable,
) -> Option<Fixed> {
    let e = fundamental_bigrat("e")?;
    let eps_0 = fundamental_bigrat("eps_0")?;
    let m_e = fundamental_bigrat("m_e")?;
    let c = fundamental_bigrat("c")?;
    let n_a = fundamental_bigrat("N_A")?;
    let pi = compute::pi(OPACITY_PI_DIGITS);

    // The classical electron radius r_e = e^2 / (4*pi*eps_0*m_e*c^2), then the Thomson cross section
    // sigma_T = (8*pi/3) * r_e^2 (SI, m^2).
    let e2 = e.mul(&e);
    let four_pi_eps0 = BigRat::from_i64(4).mul(&pi).mul(&eps_0);
    let m_e_c2 = m_e.mul(&c).mul(&c);
    let r_e = e2.div(&four_pi_eps0.mul(&m_e_c2));
    let r_e2 = r_e.mul(&r_e);
    let sigma_t = BigRat::from_i64(8)
        .mul(&pi)
        .div(&BigRat::from_i64(3))
        .mul(&r_e2); // m^2

    // The mass per hydrogen m_H = M_H / N_A (kg): the periodic-table hydrogen molar mass (g/mol) to kg/mol, over
    // Avogadro. This reads the same molar-mass kernel the materials substrate uses, never an authored m_H.
    let m_h_g_per_mol = table.molar_mass_of(&[("H", 1)]).ok()?;
    let m_h_kg = nonneg_fixed_to_bigrat(m_h_g_per_mol)
        .div(&BigRat::from_i64(1000))
        .div(&n_a);

    // kappa_es = sigma_T * (1 + X) / (2 * m_H) [m^2/kg], then * 10 to cm^2/g (1 m^2/kg = 10 cm^2/g).
    let one_plus_x = BigRat::from_i64(1).add(&nonneg_fixed_to_bigrat(hydrogen_mass_fraction));
    let kappa_m2_per_kg = sigma_t
        .mul(&one_plus_x)
        .div(&BigRat::from_i64(2).mul(&m_h_kg));
    let kappa_cm2_per_g = kappa_m2_per_kg.mul(&BigRat::from_i64(10));

    let bits = kappa_cm2_per_g.round_to_scale(Fixed::FRAC_BITS)?;
    Fixed::from_bits_i128(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("the periodic table loads")
    }

    #[test]
    fn the_solar_electron_scattering_opacity_reassembles_the_bell_lin_digit() {
        // The pre-registered 0.348 gate: at the solar hydrogen mass fraction X = 0.75 the derived electron-
        // scattering opacity is kappa_es = 0.1989 * 1.75 = 0.348 cm^2/g, reassembling Bell & Lin 1994 Table 3's
        // top regime (their 0.2(1+X) rounded) from the Thomson cross section and m_H alone, nothing fetched.
        let k = electron_scattering_opacity(Fixed::from_ratio(75, 100), &table())
            .expect("the opacity derives");
        assert!(
            (k.to_f64_lossy() - 0.348).abs() < 1e-3,
            "solar electron scattering is ~0.348 cm^2/g, got {}",
            k.to_f64_lossy()
        );
    }

    #[test]
    fn the_electron_scattering_coefficient_is_the_thomson_value() {
        // At X = 0 the opacity is the bare coefficient sigma_T/(2 m_H) = 0.1989 cm^2/g (the pure-helium electron-
        // scattering value), and at X = 1 (pure ionized hydrogen) it is 2 * 0.1989 = 0.398. The coefficient is
        // the Thomson cross section over twice the hydrogen mass, derived, not the fitted 0.2.
        let bare = electron_scattering_opacity(Fixed::ZERO, &table()).unwrap();
        let pure_h = electron_scattering_opacity(Fixed::ONE, &table()).unwrap();
        assert!(
            (bare.to_f64_lossy() - 0.1989).abs() < 1e-3,
            "the bare coefficient is ~0.1989 cm^2/g, got {}",
            bare.to_f64_lossy()
        );
        assert!(
            (pure_h.to_f64_lossy() - 0.3978).abs() < 1e-3,
            "pure ionized hydrogen is ~0.398 cm^2/g, got {}",
            pure_h.to_f64_lossy()
        );
    }

    #[test]
    fn a_hydrogen_poor_plasma_scatters_less_admit_the_alien() {
        // X is the only per-world input: a hydrogen-poor plasma (lower X) has fewer electrons per unit mass and a
        // lower electron-scattering opacity, a data row, never a rewrite. The opacity rises monotonically with X.
        let poor = electron_scattering_opacity(Fixed::from_ratio(1, 10), &table()).unwrap();
        let solar = electron_scattering_opacity(Fixed::from_ratio(75, 100), &table()).unwrap();
        assert!(
            poor < solar,
            "a hydrogen-poor plasma scatters less than a solar one"
        );
    }

    #[test]
    fn the_opacity_is_a_deterministic_pure_derivation() {
        let t = table();
        let x = Fixed::from_ratio(75, 100);
        assert_eq!(
            electron_scattering_opacity(x, &t),
            electron_scattering_opacity(x, &t),
            "a pure derivation from the fundamentals replays byte for byte"
        );
    }
}
