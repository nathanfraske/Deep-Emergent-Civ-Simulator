//! The domain-neutral Keplerian orbital state: one carrier for the circular-orbit frequency, period, and
//! specific angular momentum of a test body around a central mass, computed entirely in the log domain.
//!
//! WHY THIS EXISTS. A circular Keplerian orbit is fixed by two numbers, the orbital radius and the central mass,
//! and from them the frequency `Omega_K = sqrt(G M / r^3)`, the period `P = 2 pi / Omega_K`, and the specific
//! angular momentum `h = sqrt(G M r)` all follow. The engine had these buried in domain-specific kernels: the
//! protoplanetary [`crate::astro::kepler_orbital_period_seconds`] forms `orbit_au^3` in LINEAR Q32.32 before its
//! square root, which underflows to zero for a radius near a planetary body (a circumplanetary orbit at one
//! planetary radius is `orbit_au ~ 5e-4`, and its cube `~1e-10` falls below the Q32.32 resolution `~2.3e-10`),
//! and `astro::viscous_dissipation_flux` recomputes `ln Omega` inline. This state is the single
//! representable carrier: it holds `Omega_K`, `P`, and `h` as their base-ten logarithms, formed by the same
//! `ln Omega = (1/2)(ln G + ln M - 3 ln r)` identity the protoplanetary surface density already uses, so the
//! cube never forms and the state is live from a planetary surface out past the Oort cloud.
//!
//! DERIVE-FIRST and ADMIT-THE-ALIEN. No value is authored: `G` is the CODATA constant from the units floor, the
//! solar mass and astronomical unit are the cited anchors in [`crate::astro`], and the two per-body numbers (the
//! radius in AU and the central mass in solar masses) are ARGUMENTS. A moon around a gas giant, a planet around a
//! star, or an alien body around any central mass is a data row, never a rewrite: the same central-mass-in-solar
//! convention spans a Jovian CPD (`M ~ 1e-3 M_sun`) and a stellar orbit (`M ~ 1 M_sun`).
//!
//! DETERMINISM (Principle 3). Fixed-point throughout; every quantity is carried as a base-ten log so a wide
//! magnitude (a frequency of `1e-4` rad/s, a specific angular momentum of `1e15` m^2/s) is a moderate number, and
//! a degenerate input fails soft to `None` rather than saturating. This state is DORMANT: no run-path caller, so
//! the run pins are unaffected. It is the object the CPD viscous-heating flux and, after its next revision, the
//! tidal-heating path in [`crate::moons`] are to consume, so the Keplerian premise both depend on is constructed
//! once and cannot be violated by a caller passing an inconsistent frequency.

use civsim_core::Fixed;

use crate::astro::{ASTRONOMICAL_UNIT_M, SOLAR_MASS_KG};

/// A circular Keplerian orbit's frequency, period, and specific angular momentum, each as a base-ten logarithm in
/// SI units, derived from the orbital radius and the central mass. Constructed by [`KeplerianOrbitState::new`],
/// which fails soft (`None`) on a non-positive input.
///
/// The three log fields are the representable carriers: `Omega_K` in `1e-4` rad/s and `h` in `1e15` m^2/s are wide
/// magnitudes that a linear Q32.32 value cannot hold across the whole domain, so they are held as their logs and a
/// consumer works in the log domain (for a `Omega_K^2` term, double the log field and add). The radius and central
/// mass are carried alongside so a consumer can key a validity frame or a further derivation off the same inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeplerianOrbitState {
    /// The orbital radius in astronomical units, the constructor argument, carried for the validity frame.
    pub radius_au: Fixed,
    /// The central mass in solar masses, the constructor argument, carried for the validity frame.
    pub central_mass_solar: Fixed,
    /// `log10(Omega_K / (rad s^-1))`, the circular-orbit angular frequency (equivalently the mean motion `n`).
    pub log10_omega_s_inv: Fixed,
    /// `log10(P / s)`, the orbital period `P = 2 pi / Omega_K`, the secondary view of the frequency.
    pub log10_period_s: Fixed,
    /// `log10(h / (m^2 s^-1))`, the specific angular momentum `h = sqrt(G M r) = r^2 Omega_K` of the circular orbit.
    pub log10_specific_angular_momentum_si: Fixed,
}

impl KeplerianOrbitState {
    /// Derive the orbital state from the radius in AU and the central mass in solar masses. All three quantities
    /// are formed in the natural-log domain from `ln Omega = (1/2)(ln G + ln M - 3 ln r)` with `M = central_mass *
    /// M_sun` and `r = radius * AU`, then converted to base ten, so the `r^3` that overflows the seconds kernel
    /// never forms. `None` on a non-positive radius or mass, or on any log-domain step that leaves the
    /// representable range.
    pub fn new(radius_au: Fixed, central_mass_solar: Fixed) -> Option<Self> {
        if radius_au <= Fixed::ZERO || central_mass_solar <= Fixed::ZERO {
            return None;
        }
        // The three floor constants, in the natural-log domain: G (CODATA, units floor), the solar mass and the
        // astronomical unit (the cited anchors in `astro`). Reused, never re-authored.
        let ln_g = civsim_physics::saha::ln_of_decimal(
            civsim_units::fundamentals::GRAVITATIONAL_CONSTANT.value,
        )?;
        let ln_m = central_mass_solar
            .ln()
            .checked_add(civsim_physics::saha::ln_of_decimal(SOLAR_MASS_KG)?)?;
        let ln_r = radius_au
            .ln()
            .checked_add(civsim_physics::saha::ln_of_decimal(ASTRONOMICAL_UNIT_M)?)?;
        let half = Fixed::from_ratio(1, 2);
        // ln Omega_K = (1/2)(ln G + ln M - 3 ln r): the circular-orbit frequency, the same identity the
        // protoplanetary surface density forms inline, here the shared source.
        let ln_omega = half.checked_mul(
            ln_g.checked_add(ln_m)?
                .checked_sub(Fixed::from_int(3).checked_mul(ln_r)?)?,
        )?;
        // ln P = ln(2 pi) - ln Omega_K: the period is the reciprocal frequency scaled by the full turn.
        let ln_two_pi = Fixed::from_int(2).checked_mul(Fixed::PI)?.ln();
        let ln_period = ln_two_pi.checked_sub(ln_omega)?;
        // ln h = (1/2)(ln G + ln M + ln r): the circular-orbit specific angular momentum h = sqrt(G M r), the
        // quantity the transport solve's viscous couple g = 3 pi nu Sigma h needs.
        let ln_h = half.checked_mul(ln_g.checked_add(ln_m)?.checked_add(ln_r)?)?;
        // Convert each to base ten once (log10 x = ln x / ln 10), the convention the CPD flux kernel consumes.
        let ln_ten = civsim_physics::saha::ln_of_decimal("10")?;
        Some(Self {
            radius_au,
            central_mass_solar,
            log10_omega_s_inv: ln_omega.checked_div(ln_ten)?,
            log10_period_s: ln_period.checked_div(ln_ten)?,
            log10_specific_angular_momentum_si: ln_h.checked_div(ln_ten)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The floor constants as f64, matching the cited strings the kernel reads, for the independent numerical
    // twin. This is the reference channel, computed by a different route (linear f64) than the fixed-point
    // log-domain kernel, so an agreement convicts both.
    const G_F64: f64 = 6.674_30e-11;
    const M_SUN_F64: f64 = 1.989e30;
    const AU_M_F64: f64 = 149_597_870_700.0;

    // The anchor inputs, each as an exact integer ratio so the Fixed input and the f64 reference carry the same
    // value. Jupiter's mass in solar masses is 1.898e27 / 1.989e30 = 1898 / 1_989_000.
    const R_JUPITER_AU_NUM: i64 = 71_492_000;
    const AU_DEN: i64 = 149_597_870_700;
    const GANYMEDE_A_AU_NUM: i64 = 1_070_400_000;
    const M_JUPITER_NUM: i64 = 1898;
    const M_JUPITER_DEN: i64 = 1_989_000;

    fn f64_ref(radius_au: f64, central_mass_solar: f64) -> (f64, f64, f64) {
        let m = central_mass_solar * M_SUN_F64;
        let r = radius_au * AU_M_F64;
        let omega = (G_F64 * m / (r * r * r)).sqrt();
        let period = 2.0 * std::f64::consts::PI / omega;
        let h = (G_F64 * m * r).sqrt();
        (omega.log10(), period.log10(), h.log10())
    }

    // LIVENESS at one Jovian radius: the exact regime where the linear seconds kernel underflows (orbit_au ~ 5e-4,
    // its cube ~1e-10 below Q32.32 resolution). The log-domain state resolves it and matches the physical value
    // (Jupiter's ~3-hour surface orbital period), the P0-C fix demonstrated.
    #[test]
    fn live_at_one_jovian_radius_where_the_linear_cube_underflows() {
        let radius = Fixed::from_ratio(R_JUPITER_AU_NUM, AU_DEN);
        let mass = Fixed::from_ratio(M_JUPITER_NUM, M_JUPITER_DEN);
        let s = KeplerianOrbitState::new(radius, mass).expect("well-posed orbit resolves");
        let (ref_omega, ref_period, ref_h) = f64_ref(
            R_JUPITER_AU_NUM as f64 / AU_DEN as f64,
            M_JUPITER_NUM as f64 / M_JUPITER_DEN as f64,
        );
        // The surface orbital period is about 3 hours (~1.06e4 s, log10 ~ 4.03), a known physical anchor.
        assert!((s.log10_period_s.to_f64_lossy() - 4.03).abs() < 0.05);
        assert!((s.log10_omega_s_inv.to_f64_lossy() - ref_omega).abs() < 1e-3);
        assert!((s.log10_period_s.to_f64_lossy() - ref_period).abs() < 1e-3);
        assert!((s.log10_specific_angular_momentum_si.to_f64_lossy() - ref_h).abs() < 1e-3);
        // The seconds kernel underflows here: its cube of the radius rounds to zero, so it cannot serve the CPD.
        let cube = radius
            .checked_mul(radius)
            .and_then(|x| x.checked_mul(radius))
            .expect("mul is representable");
        assert_eq!(
            cube,
            Fixed::ZERO,
            "the linear cube underflows to zero at 1 R_J"
        );
    }

    // LIVENESS at Ganymede's orbit: the mid-CPD anchor, matching the ~7.15-day period.
    #[test]
    fn matches_the_reference_at_ganymede() {
        let radius = Fixed::from_ratio(GANYMEDE_A_AU_NUM, AU_DEN);
        let mass = Fixed::from_ratio(M_JUPITER_NUM, M_JUPITER_DEN);
        let s = KeplerianOrbitState::new(radius, mass).expect("well-posed orbit resolves");
        let (ref_omega, ref_period, ref_h) = f64_ref(
            GANYMEDE_A_AU_NUM as f64 / AU_DEN as f64,
            M_JUPITER_NUM as f64 / M_JUPITER_DEN as f64,
        );
        // Ganymede's period is ~7.15 days (~6.18e5 s, log10 ~ 5.79).
        assert!((s.log10_period_s.to_f64_lossy() - 5.79).abs() < 0.05);
        assert!((s.log10_omega_s_inv.to_f64_lossy() - ref_omega).abs() < 1e-3);
        assert!((s.log10_period_s.to_f64_lossy() - ref_period).abs() < 1e-3);
        assert!((s.log10_specific_angular_momentum_si.to_f64_lossy() - ref_h).abs() < 1e-3);
    }

    // LIVENESS at a stellar orbit (Earth around the Sun): the far end of the domain the same state spans, one
    // sidereal year, so the carrier is domain-neutral and not CPD-only.
    #[test]
    fn matches_the_reference_at_an_earth_orbit() {
        let s = KeplerianOrbitState::new(Fixed::from_int(1), Fixed::from_int(1))
            .expect("well-posed orbit resolves");
        let (ref_omega, _ref_period, ref_h) = f64_ref(1.0, 1.0);
        // One sidereal year is ~3.156e7 s (log10 ~ 7.499).
        assert!((s.log10_period_s.to_f64_lossy() - 7.499).abs() < 0.02);
        assert!((s.log10_omega_s_inv.to_f64_lossy() - ref_omega).abs() < 1e-3);
        assert!((s.log10_specific_angular_momentum_si.to_f64_lossy() - ref_h).abs() < 1e-3);
    }

    // The period is the exact reciprocal-frequency view: log10 P + log10 Omega = log10(2 pi) at every radius.
    #[test]
    fn period_and_frequency_are_reciprocal_through_two_pi() {
        let radius = Fixed::from_ratio(GANYMEDE_A_AU_NUM, AU_DEN);
        let mass = Fixed::from_ratio(M_JUPITER_NUM, M_JUPITER_DEN);
        let s = KeplerianOrbitState::new(radius, mass).expect("resolves");
        let sum = s.log10_period_s.checked_add(s.log10_omega_s_inv).unwrap();
        let log10_two_pi = (2.0 * std::f64::consts::PI).log10();
        assert!((sum.to_f64_lossy() - log10_two_pi).abs() < 1e-3);
    }

    // Specific angular momentum obeys h = r^2 Omega_K: log10 h = 2 log10 r + log10 Omega (r in metres). A
    // cross-identity independent of the sqrt(G M r) route the kernel took.
    #[test]
    fn specific_angular_momentum_obeys_r_squared_omega() {
        let radius = Fixed::from_ratio(GANYMEDE_A_AU_NUM, AU_DEN);
        let mass = Fixed::from_ratio(M_JUPITER_NUM, M_JUPITER_DEN);
        let s = KeplerianOrbitState::new(radius, mass).expect("resolves");
        let log10_r_m = (GANYMEDE_A_AU_NUM as f64 / AU_DEN as f64 * AU_M_F64).log10();
        let expected = 2.0 * log10_r_m + s.log10_omega_s_inv.to_f64_lossy();
        assert!((s.log10_specific_angular_momentum_si.to_f64_lossy() - expected).abs() < 1e-3);
    }

    #[test]
    fn degenerate_inputs_fail_soft() {
        assert!(KeplerianOrbitState::new(Fixed::ZERO, Fixed::from_int(1)).is_none());
        assert!(KeplerianOrbitState::new(Fixed::from_int(1), Fixed::ZERO).is_none());
        assert!(KeplerianOrbitState::new(Fixed::from_int(-1), Fixed::from_int(1)).is_none());
    }

    #[test]
    fn determinism_same_inputs_same_state() {
        let radius = Fixed::from_ratio(GANYMEDE_A_AU_NUM, AU_DEN);
        let mass = Fixed::from_ratio(M_JUPITER_NUM, M_JUPITER_DEN);
        let a = KeplerianOrbitState::new(radius, mass).expect("resolves");
        let b = KeplerianOrbitState::new(radius, mass).expect("resolves");
        assert_eq!(a, b);
    }
}
