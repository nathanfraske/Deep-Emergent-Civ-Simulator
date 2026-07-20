//! The conservation-derived steady transport state of a gas-starved circumplanetary disk (CPD): the radial mass
//! flux, the viscous couple, and the product `nu Sigma`, solved from mass and angular-momentum conservation with
//! the two torque-free boundary conditions, in closed form. This is the substrate the CPD's surface density,
//! viscous heating, and the gas-flux geometry factor all read, replacing the discontinuous printed geometry
//! factor with a derived, continuous one.
//!
//! WHY THIS EXISTS. The CPD viscous-heating flux in [`crate::cpd_thermal`] took the gas-flux geometry factor
//! `Lambda/l` as a caller input because the source paper's printed radial branches (Schneeberger and Mousis 2025
//! Eqs. 4 to 6) are DISCONTINUOUS at the centrifugal radius, a flagged found-seam. The resolution is not to patch
//! the printed branches until they meet: it is to solve the transport problem those branches are a solution OF,
//! from the conservation laws upstream of them. What mass conservation makes continuous is the radial mass flux
//! `F_M`; absent a singular torque the viscous couple `g` is continuous too; and `Sigma` (hence `Lambda/l`) then
//! follows as continuous under a regular viscosity and a Keplerian specific angular momentum. Solving `F_M` and
//! `g` from conservation yields a `Lambda/l` that is continuous at `R_c` by construction, which the printed form
//! only approximates.
//!
//! THE MODEL, from Canup and Ward (2002) as reproduced by Schneeberger and Mousis (2025). A steady disk fed a
//! mass source `S_M(r)`, in circular Keplerian rotation, spreads viscously: some gas flows inward across the
//! inner boundary onto the planet, the rest outward across the outer edge. In steady state, with `F_M` the
//! outward-positive radial mass flux, `h = sqrt(G M r)` the specific angular momentum, and `g = 3 pi nu Sigma h`
//! the viscous couple:
//!
//! - mass:              `dF_M/dr = 2 pi r S_M(r)`
//! - angular momentum:  `F_M dh/dr = -dg/dr`  (deposition onto local circular orbits, `h_source = h`)
//! - boundaries:        `g(R_p) = 0` and `g(r_d) = 0`  (no torque at the planet surface or the disk edge)
//!
//! For the gas-starved UNIFORM source inside the centrifugal radius (`S_M = S_0` for `R_p <= r <= R_c`, zero
//! beyond), this has a CLOSED FORM: the whole solution is a function of two dimensionless ratios, `p = R_p/R_c`
//! and `d = r_d/R_c`, and the physical scale enters only through the total supply rate `Mdot`. So no JFNK
//! iteration is needed here (the paper's JFNK is for the coupled THERMAL solve, a later rung); this transport
//! solve is exact. The accreted fraction `a = Mdot_planet/Mdot` is LINEAR in the two boundary conditions:
//!
//! ```text
//! a = [ (1/(1-p^2)) ((2/5)(1 - p^(5/2)) - 2 p^2 (1 - sqrt(p))) + 2 (sqrt(d) - 1) ] / [ 2 (sqrt(d) - sqrt(p)) ]
//! ```
//!
//! and `nu Sigma(x) = -(Mdot / (6 pi)) g~(x) / sqrt(x)` with `x = r/R_c` and `g~` the dimensionless couple, the
//! wide `sqrt(G M)` and `R_c` factors cancelling, so the couple and `nu Sigma` are order-one dimensionless numbers
//! scaled by `Mdot`.
//!
//! DERIVE-FIRST and ADMIT-THE-ALIEN. Nothing is authored: the geometry ratios and the supply rate are arguments,
//! `G` is the units-floor constant reached through [`crate::orbital_state`]. A hotter or colder disc, a different
//! centrifugal radius, or an alien giant's CPD is a data row. The uniform-source-inside-`R_c` model is ONE member
//! of [`CpdSourceProfile`], declared as such, not universal CPD physics; a radial or hydrodynamic source profile
//! is a future member that keys the same state.
//!
//! DETERMINISM (Principle 3) and DORMANCY. Fixed-point throughout, order-one dimensionless arithmetic (no wide
//! magnitude in the solve, so no log domain needed here; the supply rate is carried as its base-ten log only for
//! the physical `nu Sigma` a consumer forms). A degenerate geometry fails soft to `None`. No run-path caller, so
//! the run pins are unaffected. This is the object the viscous-heating flux is to consume in place of a caller
//! scalar, so the geometry factor becomes a diagnostic read of a solved state rather than a free parameter.

use civsim_core::Fixed;

/// The mass-source model of the circumplanetary disk, the declared assumption the transport solve rests on. The
/// gas-starved uniform source inside the centrifugal radius is the one built member; a radial profile or a
/// three-dimensional hydrodynamic surrogate that deposits mass and angular momentum non-uniformly are named
/// future members, each of which would key the same [`CpdSteadyTransportState`] with a different quadrature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpdSourceProfile {
    /// A uniform mass source per unit area inside the centrifugal radius `R_c`, zero beyond (Canup and Ward 2002,
    /// the gas-starved model). The built member.
    UniformInsideCentrifugalRadius,
}

/// The mass budget of the steady CPD as dimensionless fractions of the total supply rate: what the supplied gas
/// splits into, the planet-ward sink and the outer outflow. Interpretation-neutral, with an explicit conservation
/// invariant [`CpdMassLedger::is_closed`] (the #212 `retained + removed == total` pattern), so a consumer cannot
/// read an unbalanced or negative account.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpdMassLedger {
    /// The total supply rate as a fraction of itself, always one (the reference the other two partition).
    pub supply_fraction: Fixed,
    /// The fraction of the supplied gas that flows inward across the inner boundary and accretes onto the planet.
    pub planet_sink_fraction: Fixed,
    /// The fraction that flows outward across the disk edge and leaves the system.
    pub outer_outflow_fraction: Fixed,
}

impl CpdMassLedger {
    /// Whether the ledger conserves mass: the sink and the outflow partition the supply exactly and none is
    /// negative. A well-formed solve always closes; the check guards a hand-built or corrupted account.
    pub fn is_closed(&self) -> bool {
        let non_negative = self.supply_fraction >= Fixed::ZERO
            && self.planet_sink_fraction >= Fixed::ZERO
            && self.outer_outflow_fraction >= Fixed::ZERO;
        let partitions = self
            .planet_sink_fraction
            .checked_add(self.outer_outflow_fraction)
            == Some(self.supply_fraction);
        non_negative && partitions
    }
}

/// The steady transport state of a gas-starved CPD: the accreted fraction, the stagnation radius, the mass
/// ledger, and the angular-momentum residual, plus evaluators for the radial mass flux, the viscous couple, the
/// product `nu Sigma`, and the derived gas-flux geometry factor at any radius. Constructed by
/// [`CpdSteadyTransportState::new`], which fails soft on a degenerate geometry.
///
/// The state carries the two dimensionless geometry ratios and the base-ten log of the supply rate; the evaluators
/// take `x = r / R_c` and return dimensionless quantities (or, for `nu Sigma`, a value the caller scales by the
/// physical `Mdot`), so the wide-magnitude supply rate never enters the order-one solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpdSteadyTransportState {
    /// The source model this solve assumes.
    pub source_model: CpdSourceProfile,
    /// `p = R_p / R_c`, the inner boundary (planet radius) over the centrifugal radius, in `(0, 1)`.
    pub planet_radius_over_rc: Fixed,
    /// `d = r_d / R_c`, the disk outer radius over the centrifugal radius, greater than one.
    pub outer_radius_over_rc: Fixed,
    /// `log10(Mdot / (kg s^-1))`, the total supply rate, carried for the physical `nu Sigma` a consumer forms.
    pub log10_mdot_supply_kg_s: Fixed,
    /// The mass budget the solve conserves.
    pub mass: CpdMassLedger,
    /// `r_stag / R_c`, the stagnation radius where the radial mass flux reverses (inward within, outward beyond),
    /// solved from the flux, not assumed at `R_c`.
    pub stagnation_radius_over_rc: Fixed,
    /// The evaluated outer-boundary couple `g~(d)`, which the boundary condition forces to zero; carried as the
    /// angular-momentum conservation residual, the fixed-point departure from an exact `g(r_d) = 0`.
    pub angular_momentum_residual: Fixed,
}

impl CpdSteadyTransportState {
    /// Solve the steady transport state from the three radii (planet, centrifugal, disk outer, all in AU) and the
    /// base-ten log of the total gas supply rate in kg/s. The solve depends only on the ratios `p = R_p/R_c` and
    /// `d = r_d/R_c`; the supply rate is carried for the physical `nu Sigma`. `None` on a non-positive radius, a
    /// non-nested geometry (`R_p < R_c < r_d` is required), or an accreted fraction outside `[0, 1]` (an
    /// unphysical geometry the closed form would otherwise report).
    pub fn new(
        planet_radius_au: Fixed,
        centrifugal_radius_au: Fixed,
        outer_radius_au: Fixed,
        log10_mdot_supply_kg_s: Fixed,
    ) -> Option<Self> {
        if planet_radius_au <= Fixed::ZERO
            || centrifugal_radius_au <= Fixed::ZERO
            || outer_radius_au <= Fixed::ZERO
        {
            return None;
        }
        let p = planet_radius_au.checked_div(centrifugal_radius_au)?;
        let d = outer_radius_au.checked_div(centrifugal_radius_au)?;
        // The geometry must nest: the planet inside the centrifugal radius inside the disk edge.
        if p >= Fixed::from_int(1) || d <= Fixed::from_int(1) {
            return None;
        }
        let one = Fixed::from_int(1);
        let sqrt_p = p.sqrt();
        let sqrt_d = d.sqrt();
        let p2 = p.checked_mul(p)?;
        let one_minus_p2 = one.checked_sub(p2)?;
        let p_52 = p2.checked_mul(sqrt_p)?;
        // The source-region contribution to the outer-boundary angular-momentum constraint, evaluated on [p, 1]:
        //   (1/(1-p^2)) [ (2/5)(1 - p^(5/2)) - 2 p^2 (1 - sqrt(p)) ].
        let two_fifths = Fixed::from_ratio(2, 5);
        let src_term = two_fifths
            .checked_mul(one.checked_sub(p_52)?)?
            .checked_sub(
                Fixed::from_int(2)
                    .checked_mul(p2)?
                    .checked_mul(one.checked_sub(sqrt_p)?)?,
            )?
            .checked_div(one_minus_p2)?;
        // The source-free contribution on [1, d]: 2 (sqrt(d) - 1). Their sum is the constraint constant.
        let outer_term = Fixed::from_int(2).checked_mul(sqrt_d.checked_sub(one)?)?;
        let numerator = src_term.checked_add(outer_term)?;
        // a = numerator / (2 (sqrt(d) - sqrt(p))): the accreted fraction, linear in the boundary conditions.
        let denom = Fixed::from_int(2).checked_mul(sqrt_d.checked_sub(sqrt_p)?)?;
        let a = numerator.checked_div(denom)?;
        if a < Fixed::ZERO || a > one {
            return None;
        }
        let outflow = one.checked_sub(a)?;
        let mass = CpdMassLedger {
            supply_fraction: one,
            planet_sink_fraction: a,
            outer_outflow_fraction: outflow,
        };
        if !mass.is_closed() {
            return None;
        }
        // Stagnation radius: x_stag^2 = p^2 + a (1 - p^2), where the region-1 flux -a + (x^2-p^2)/(1-p^2) is zero.
        let x_stag = p2.checked_add(a.checked_mul(one_minus_p2)?)?.sqrt();
        let state = Self {
            source_model: CpdSourceProfile::UniformInsideCentrifugalRadius,
            planet_radius_over_rc: p,
            outer_radius_over_rc: d,
            log10_mdot_supply_kg_s,
            mass,
            stagnation_radius_over_rc: x_stag,
            // Filled below once the couple evaluator exists.
            angular_momentum_residual: Fixed::ZERO,
        };
        // The outer-boundary couple g~(d) is forced to zero by construction; carry its evaluated value as the
        // conservation residual (the fixed-point departure from an exact zero).
        let residual = state.dimensionless_couple(d)?;
        Some(Self {
            angular_momentum_residual: residual,
            ..state
        })
    }

    /// The radial mass flux at `x = r / R_c` as a fraction of the total supply rate `Mdot`, outward-positive.
    /// Negative within the stagnation radius (inflow onto the planet), positive beyond (outflow). `None` inside
    /// the planet (`x < p`).
    pub fn mass_flux_fraction(&self, x: Fixed) -> Option<Fixed> {
        let p = self.planet_radius_over_rc;
        if x < p {
            return None;
        }
        let a = self.mass.planet_sink_fraction;
        if x <= Fixed::from_int(1) {
            // Region 1 (source region): -a + (x^2 - p^2)/(1 - p^2).
            let p2 = p.checked_mul(p)?;
            let one_minus_p2 = Fixed::from_int(1).checked_sub(p2)?;
            let rise = x
                .checked_mul(x)?
                .checked_sub(p2)?
                .checked_div(one_minus_p2)?;
            rise.checked_sub(a)
        } else {
            // Region 2 (source-free): the constant outward flux, the outflow fraction.
            Some(self.mass.outer_outflow_fraction)
        }
    }

    /// The dimensionless viscous couple `g~(x) = integral_p^x F_M(x') x'^(-1/2) dx'`, from which the physical
    /// couple is `g = -(sqrt(G M) / 2) Mdot R_c^(1/2) g~`. Zero at both boundaries (`x = p` and `x = d`) by the
    /// torque-free conditions, negative in the interior. `None` inside the planet.
    pub fn dimensionless_couple(&self, x: Fixed) -> Option<Fixed> {
        let p = self.planet_radius_over_rc;
        if x < p {
            return None;
        }
        let a = self.mass.planet_sink_fraction;
        let one = Fixed::from_int(1);
        let sqrt_p = p.sqrt();
        let p2 = p.checked_mul(p)?;
        let one_minus_p2 = one.checked_sub(p2)?;
        // Region-1 contribution up to min(x, 1).
        let x1 = if x < one { x } else { one };
        let sqrt_x1 = x1.sqrt();
        let x1_52 = x1.checked_mul(x1)?.checked_mul(sqrt_x1)?;
        let p_52 = p2.checked_mul(sqrt_p)?;
        let two_fifths = Fixed::from_ratio(2, 5);
        // -2 a (sqrt(x1) - sqrt(p)) + (1/(1-p^2)) [ (2/5)(x1^(5/2) - p^(5/2)) - 2 p^2 (sqrt(x1) - sqrt(p)) ].
        let term_a = Fixed::from_int(-2)
            .checked_mul(a)?
            .checked_mul(sqrt_x1.checked_sub(sqrt_p)?)?;
        let src = two_fifths
            .checked_mul(x1_52.checked_sub(p_52)?)?
            .checked_sub(
                Fixed::from_int(2)
                    .checked_mul(p2)?
                    .checked_mul(sqrt_x1.checked_sub(sqrt_p)?)?,
            )?
            .checked_div(one_minus_p2)?;
        let mut g = term_a.checked_add(src)?;
        // Region-2 contribution: 2 (1 - a) (sqrt(x) - 1) for x beyond the centrifugal radius.
        if x > one {
            let sqrt_x = x.sqrt();
            let term2 = Fixed::from_int(2)
                .checked_mul(self.mass.outer_outflow_fraction)?
                .checked_mul(sqrt_x.checked_sub(one)?)?;
            g = g.checked_add(term2)?;
        }
        Some(g)
    }

    /// The product `nu Sigma` at `x = r / R_c`, as a fraction of `Mdot`: `nu Sigma = -(Mdot / (6 pi)) g~(x) /
    /// sqrt(x)`. This returns the dimensionless part `-g~(x) / (6 pi sqrt(x))`, which the caller multiplies by the
    /// physical `Mdot` (in kg/s) to get `nu Sigma` in kg/s. Non-negative in the interior, zero at both boundaries.
    /// `None` inside the planet or at a non-positive `x`.
    pub fn nu_sigma_over_mdot(&self, x: Fixed) -> Option<Fixed> {
        if x <= Fixed::ZERO {
            return None;
        }
        let g = self.dimensionless_couple(x)?;
        let six_pi = Fixed::from_int(6).checked_mul(Fixed::PI)?;
        // -g / (6 pi sqrt(x)).
        g.checked_div(six_pi.checked_mul(x.sqrt())?)
            .map(|v| Fixed::ZERO.checked_sub(v))?
    }

    /// The gas-flux geometry factor `Lambda/l = 3 pi nu Sigma / Mdot = -g~(x) / (2 sqrt(x))` at `x = r / R_c`, the
    /// DERIVED, continuous replacement for the discontinuous printed radial branches. Continuous at the
    /// centrifugal radius by construction, since both `F_M` and `g~` are continuous there. `None` inside the
    /// planet or at a non-positive `x`.
    pub fn geometry_factor(&self, x: Fixed) -> Option<Fixed> {
        if x <= Fixed::ZERO {
            return None;
        }
        let g = self.dimensionless_couple(x)?;
        let two_sqrt_x = Fixed::from_int(2).checked_mul(x.sqrt())?;
        g.checked_div(two_sqrt_x)
            .map(|v| Fixed::ZERO.checked_sub(v))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A representative Jovian-scale CPD geometry (all in AU): the planet radius, the centrifugal radius, and the
    // disk outer radius, as exact ratios so the Fixed inputs and the f64 references carry the same values.
    // R_p ~ 1 R_J = 4.779e-4 AU, R_c ~ 0.015 AU, r_d ~ 0.15 AU.
    fn geom() -> (Fixed, Fixed, Fixed) {
        (
            Fixed::from_ratio(4_779, 10_000_000),
            Fixed::from_ratio(15, 1000),
            Fixed::from_ratio(150, 1000),
        )
    }

    fn p_d_f64() -> (f64, f64) {
        let rp = 4_779.0 / 10_000_000.0;
        let rc = 15.0 / 1000.0;
        let rd = 150.0 / 1000.0;
        (rp / rc, rd / rc)
    }

    // The independent f64 reference for the accreted fraction, the closed form computed by a different route.
    fn accreted_fraction_ref(p: f64, d: f64) -> f64 {
        let src = (1.0 / (1.0 - p * p))
            * ((2.0 / 5.0) * (1.0 - p.powf(2.5)) - 2.0 * p * p * (1.0 - p.sqrt()));
        let outer = 2.0 * (d.sqrt() - 1.0);
        (src + outer) / (2.0 * (d.sqrt() - p.sqrt()))
    }

    fn state() -> CpdSteadyTransportState {
        let (rp, rc, rd) = geom();
        CpdSteadyTransportState::new(rp, rc, rd, Fixed::from_int(13))
            .expect("well-posed CPD resolves")
    }

    #[test]
    fn accreted_fraction_matches_the_reference_and_is_physical() {
        let s = state();
        let (p, d) = p_d_f64();
        let a_ref = accreted_fraction_ref(p, d);
        assert!((s.mass.planet_sink_fraction.to_f64_lossy() - a_ref).abs() < 1e-3);
        // Most of the gas-starved supply accretes onto the planet, a fraction flows out: a physical split in (0,1).
        assert!(s.mass.planet_sink_fraction > Fixed::ZERO);
        assert!(s.mass.planet_sink_fraction < Fixed::from_int(1));
    }

    #[test]
    fn mass_is_conserved_supply_equals_sink_plus_outflow() {
        let s = state();
        assert!(s.mass.is_closed());
        let sum = s
            .mass
            .planet_sink_fraction
            .checked_add(s.mass.outer_outflow_fraction)
            .unwrap();
        assert_eq!(sum, Fixed::from_int(1));
    }

    // The outer torque-free boundary condition g(r_d) = 0: the carried angular-momentum residual is the
    // fixed-point departure from an exact zero, and it is small.
    #[test]
    fn outer_boundary_is_torque_free() {
        let s = state();
        assert!(s.angular_momentum_residual.to_f64_lossy().abs() < 1e-3);
    }

    // The inner torque-free boundary condition g(R_p) = 0: the couple vanishes at the planet radius by
    // construction (the integral from p to p).
    #[test]
    fn inner_boundary_is_torque_free() {
        let s = state();
        let g_at_p = s.dimensionless_couple(s.planet_radius_over_rc).unwrap();
        assert!(g_at_p.to_f64_lossy().abs() < 1e-6);
    }

    // The stagnation radius sits between the planet and the centrifugal radius: the flux reverses inside the
    // source region, not at R_c.
    #[test]
    fn stagnation_radius_is_interior_to_the_source_region() {
        let s = state();
        assert!(s.stagnation_radius_over_rc > s.planet_radius_over_rc);
        assert!(s.stagnation_radius_over_rc < Fixed::from_int(1));
        // The flux is zero there.
        let f = s.mass_flux_fraction(s.stagnation_radius_over_rc).unwrap();
        assert!(f.to_f64_lossy().abs() < 1e-3);
    }

    // THE HEADLINE: the derived gas-flux geometry factor is CONTINUOUS at the centrifugal radius, where the
    // paper's printed branches (Eqs. 4 and 5) jump by about 0.13. Evaluated just inside and just outside R_c, the
    // derived Lambda/l agrees, so solving from conservation resolves the found seam rather than patching it.
    #[test]
    fn geometry_factor_is_continuous_at_the_centrifugal_radius() {
        let s = state();
        let just_inside = Fixed::from_ratio(999, 1000);
        let just_outside = Fixed::from_ratio(1001, 1000);
        let inside = s.geometry_factor(just_inside).unwrap();
        let outside = s.geometry_factor(just_outside).unwrap();
        assert!(
            (inside.to_f64_lossy() - outside.to_f64_lossy()).abs() < 5e-3,
            "derived Lambda/l is continuous at R_c: inside {} vs outside {}",
            inside.to_f64_lossy(),
            outside.to_f64_lossy()
        );
    }

    // The radial mass flux is continuous at the centrifugal radius: the source region meets the source-free region
    // at the same outflow fraction.
    #[test]
    fn mass_flux_is_continuous_at_the_centrifugal_radius() {
        let s = state();
        let at_one = s.mass_flux_fraction(Fixed::from_int(1)).unwrap();
        let just_outside = s.mass_flux_fraction(Fixed::from_ratio(1001, 1000)).unwrap();
        assert!(
            (at_one.to_f64_lossy() - s.mass.outer_outflow_fraction.to_f64_lossy()).abs() < 1e-6
        );
        assert!((at_one.to_f64_lossy() - just_outside.to_f64_lossy()).abs() < 1e-6);
    }

    // nu Sigma is non-negative in the interior and vanishes at both boundaries: the viscous couple is physical.
    #[test]
    fn nu_sigma_is_non_negative_and_vanishes_at_the_boundaries() {
        let s = state();
        let p = s.planet_radius_over_rc;
        let d = s.outer_radius_over_rc;
        assert!(s.nu_sigma_over_mdot(p).unwrap().to_f64_lossy().abs() < 1e-6);
        assert!(s.nu_sigma_over_mdot(d).unwrap().to_f64_lossy().abs() < 1e-3);
        // Sampled through the interior, nu Sigma stays non-negative.
        for k in 1..20u32 {
            let x = Fixed::from_ratio(k as i64, 20).checked_mul(d).unwrap();
            if x <= p {
                continue;
            }
            let ns = s.nu_sigma_over_mdot(x).unwrap();
            assert!(
                ns.to_f64_lossy() > -1e-6,
                "nu Sigma non-negative at x={}: {}",
                x.to_f64_lossy(),
                ns.to_f64_lossy()
            );
        }
    }

    #[test]
    fn degenerate_geometry_fails_soft() {
        // Non-nested: R_p >= R_c.
        assert!(CpdSteadyTransportState::new(
            Fixed::from_int(2),
            Fixed::from_int(1),
            Fixed::from_int(3),
            Fixed::from_int(13)
        )
        .is_none());
        // Non-nested: r_d <= R_c.
        assert!(CpdSteadyTransportState::new(
            Fixed::from_ratio(1, 100),
            Fixed::from_int(1),
            Fixed::from_int(1),
            Fixed::from_int(13)
        )
        .is_none());
        // Non-positive radius.
        assert!(CpdSteadyTransportState::new(
            Fixed::ZERO,
            Fixed::from_int(1),
            Fixed::from_int(2),
            Fixed::from_int(13)
        )
        .is_none());
    }

    #[test]
    fn determinism_same_inputs_same_state() {
        let a = state();
        let b = state();
        assert_eq!(a, b);
    }

    // ADMIT THE ALIEN: a wider disk (larger d) accretes a larger fraction onto the planet (more of the outward
    // angular-momentum budget is carried by less mass), a monotonic response to a data-row change in geometry.
    #[test]
    fn a_wider_disk_is_a_data_row() {
        let (rp, rc, _) = geom();
        let narrow =
            CpdSteadyTransportState::new(rp, rc, Fixed::from_ratio(80, 1000), Fixed::from_int(13))
                .unwrap();
        let wide =
            CpdSteadyTransportState::new(rp, rc, Fixed::from_ratio(300, 1000), Fixed::from_int(13))
                .unwrap();
        assert!(wide.mass.planet_sink_fraction > narrow.mass.planet_sink_fraction);
    }
}
