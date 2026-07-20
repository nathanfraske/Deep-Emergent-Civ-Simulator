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

//! BRANCH A of the moon dispatch: CIRCUMPLANETARY-DISK (CPD) CO-ACCRETION, the regular satellites (the moon arc,
//! DORMANT). A gas or ice giant hosts a circumplanetary disk as it accretes, and its regular satellites co-accrete
//! from that disk (the Galilean and Saturnian systems are the type cases). This module is the first derive-first
//! primitive of Branch A: the SATELLITE-SYSTEM MASS BUDGET a CPD host can grow. That budget scales with the
//! planet's OWN mass through the Canup and Ward (2006) accretion-plus-loss equilibrium, `M_sat / M_planet ~ 1e-4`
//! (a band, a few times `1e-4` down to about `1e-4`, weakly model-dependent), set by the balance between
//! satellitesimal inflow and satellite loss to Type-I decay into the planet. So a heavier giant carries a
//! proportionally heavier satellite system, and no moon mass or count is ever authored.
//!
//! DERIVE-FIRST (Principle 8): the budget is a function of the planet's DERIVED mass and its DERIVED status as a
//! CPD host (a gas or ice giant, from the giant-formation verdict `crate::giants`), never an authored roster. The
//! individual satellites emerge LATER by accreting this budget, with a composition set by an equilibrium
//! condensation run in the CPD's own temperature and pressure profile (the `crate::physics::condensation`
//! substrate applied to the circumplanetary environment, so the Galilean rock-to-ice gradient falls out) rather
//! than a rostered ice-versus-rock list. That composition slice is a named follow-on: it needs a CPD structure
//! (temperature) model, a fetch not yet cleared, so this slice supplies the mass budget alone.
//!
//! ADMITS THE ALIEN (a prime directive): the only per-body inputs are the planet's mass and CPD-host verdict, both
//! on the argument list, so a super-Jupiter, an ice giant, or a giant of any composition is a data row through the
//! same scaling, never a new code path. Determinism (Principle 3, Principle 10): fixed-point throughout; a
//! degenerate input (a non-positive mass or ratio, an inverted band, a non-host planet) fails soft to `None`,
//! never a fabricated budget. DORMANT: no run-path caller (the dispatch wiring into `planetary_assembly` is a
//! gated follow-on), so the two run pins hold bit-exact.
//!
//! The value-authoring line (Principle 6): no number is authored here. The mass-ratio band is the CITED Canup and
//! Ward (2006) satellite-system mass scaling (`docs/working/PIPELINE_FETCHES.md` section 6, Nature 441, 834), and
//! it is a CALLER INPUT reserved-with-basis at the call site, exactly as `k2`/`Q` are for the tidal kernels: its
//! basis is the observed regular-satellite systems, whose totals bracket the range (the Galilean system is about
//! `2.07e-4` of Jupiter and the Saturnian regulars about `2.46e-4` of Saturn). The planet mass and the CPD-host
//! verdict are the assembly's derived data.

use civsim_core::Fixed;

/// The REGULAR-SATELLITE SYSTEM MASS BUDGET in Earth masses that a circumplanetary-disk host can grow: the banded
/// `[lo, hi] = [ratio_lo, ratio_hi] * M_planet` from the Canup and Ward (2006) satellite-system mass scaling
/// `M_sat / M_planet ~ 1e-4`. This is the total mass available to the regular-satellite system, from which the
/// individual moons later emerge; it is NOT a moon count or a per-moon mass.
///
/// `m_planet_earth` is the planet mass in Earth masses (the assembly's derived value). `is_cpd_host` is whether the
/// planet is a gas or ice giant that hosts a circumplanetary disk (the derived giant-formation verdict: a
/// terrestrial hosts no CPD and grows no regular satellites this way, so it returns `None`). `ratio_lo` and
/// `ratio_hi` are the Canup and Ward mass-ratio band, reserved-with-basis at the call site (its basis the observed
/// systems: the Galilean `~2.07e-4` and Saturnian `~2.46e-4` totals bracket the `~1e-4` to a few `1e-4` range).
///
/// Returns the banded budget `(lo, hi)` in Earth masses. `None` if the planet is not a CPD host, on a non-positive
/// mass, a non-positive `ratio_lo`, an inverted band (`ratio_hi < ratio_lo`), or an overflow: fail-soft, never a
/// fabricated budget.
pub fn regular_satellite_mass_budget_earth(
    m_planet_earth: Fixed,
    is_cpd_host: bool,
    ratio_lo: Fixed,
    ratio_hi: Fixed,
) -> Option<(Fixed, Fixed)> {
    if !is_cpd_host
        || m_planet_earth <= Fixed::ZERO
        || ratio_lo <= Fixed::ZERO
        || ratio_hi < ratio_lo
    {
        return None;
    }
    let lo = m_planet_earth.checked_mul(ratio_lo)?;
    let hi = m_planet_earth.checked_mul(ratio_hi)?;
    Some((lo, hi))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64, d: i64) -> Fixed {
        Fixed::from_ratio(n, d)
    }

    /// The Canup-Ward band `[1e-4, 4e-4]` applied to Jupiter's mass brackets the OBSERVED Galilean satellite-system
    /// mass (about `0.0658` Earth masses, `~2.07e-4` of Jupiter's `317.82`), the independent convicting anchor: the
    /// scaling is not fit to the Galilean number, the Galilean number is checked to fall inside the derived budget.
    #[test]
    fn the_galilean_system_falls_inside_the_derived_budget() {
        let jupiter_earth = r(31782, 100); // 317.82 M_earth
        let (lo, hi) =
            regular_satellite_mass_budget_earth(jupiter_earth, true, r(1, 10000), r(4, 10000))
                .expect("a giant grows a satellite budget");
        let galilean_observed = r(658, 10000); // 0.0658 M_earth
        assert!(
            lo <= galilean_observed && galilean_observed <= hi,
            "observed Galilean mass {} sits in the derived budget band [{}, {}]",
            galilean_observed.to_f64_lossy(),
            lo.to_f64_lossy(),
            hi.to_f64_lossy()
        );
    }

    /// The same band on Saturn brackets the observed Saturnian regular-satellite mass (about `0.0234` Earth masses,
    /// `~2.46e-4` of Saturn's `95.16`): a second independent system, a different host mass, the same scaling.
    #[test]
    fn the_saturnian_regulars_fall_inside_the_derived_budget() {
        let saturn_earth = r(9516, 100); // 95.16 M_earth
        let (lo, hi) =
            regular_satellite_mass_budget_earth(saturn_earth, true, r(1, 10000), r(4, 10000))
                .expect("a giant grows a satellite budget");
        let saturnian_observed = r(234, 10000); // 0.0234 M_earth
        assert!(
            lo <= saturnian_observed && saturnian_observed <= hi,
            "observed Saturnian mass {} sits in the derived budget band [{}, {}]",
            saturnian_observed.to_f64_lossy(),
            lo.to_f64_lossy(),
            hi.to_f64_lossy()
        );
    }

    /// The budget SCALES with the host mass at a fixed ratio: Jupiter's budget is heavier than Saturn's by exactly
    /// the mass ratio, the Canup-Ward "grows with the host" signature, not an authored per-system number.
    #[test]
    fn the_budget_scales_with_the_host_mass() {
        let ratio = r(2, 10000);
        let (jup_lo, _) =
            regular_satellite_mass_budget_earth(r(31782, 100), true, ratio, ratio).unwrap();
        let (sat_lo, _) =
            regular_satellite_mass_budget_earth(r(9516, 100), true, ratio, ratio).unwrap();
        assert!(
            jup_lo > sat_lo,
            "the heavier host carries the heavier satellite budget"
        );
        // The budgets are in the mass ratio: jup/sat budget == jup/sat mass, to fixed-point tolerance.
        let budget_ratio = jup_lo.to_f64_lossy() / sat_lo.to_f64_lossy();
        let mass_ratio = 317.82 / 95.16;
        assert!(
            (budget_ratio - mass_ratio).abs() < 1e-2,
            "the budget ratio {budget_ratio} tracks the host-mass ratio {mass_ratio}"
        );
    }

    /// A terrestrial planet hosts no circumplanetary disk, so it grows no regular satellites this way: the kernel
    /// returns `None` rather than a fabricated budget. This is the derive-first gate, keyed on the derived verdict.
    #[test]
    fn a_terrestrial_grows_no_regular_satellites() {
        assert!(
            regular_satellite_mass_budget_earth(
                Fixed::from_int(1),
                false,
                r(1, 10000),
                r(4, 10000)
            )
            .is_none(),
            "a non-CPD-host (terrestrial) has no regular-satellite budget"
        );
    }

    /// Determinism (Principle 3) and fail-soft: identical inputs give the identical budget, and a non-positive
    /// mass, a non-positive ratio, or an inverted band each return `None`, never a fabricated value.
    #[test]
    fn the_budget_is_deterministic_and_fails_soft() {
        let args = (r(31782, 100), true, r(1, 10000), r(4, 10000));
        assert_eq!(
            regular_satellite_mass_budget_earth(args.0, args.1, args.2, args.3),
            regular_satellite_mass_budget_earth(args.0, args.1, args.2, args.3)
        );
        assert!(
            regular_satellite_mass_budget_earth(Fixed::ZERO, true, r(1, 10000), r(4, 10000))
                .is_none()
        );
        assert!(
            regular_satellite_mass_budget_earth(r(31782, 100), true, Fixed::ZERO, r(4, 10000))
                .is_none()
        );
        // Inverted band (hi < lo) is rejected.
        assert!(
            regular_satellite_mass_budget_earth(r(31782, 100), true, r(4, 10000), r(1, 10000))
                .is_none()
        );
    }
}
