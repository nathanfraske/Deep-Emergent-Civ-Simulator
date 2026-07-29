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

//! Stage 6, the electronic-structure sub-arc: the near-ready entry (owner-ruled on #189,
//! `docs/working/STAGE6_ELECTRONIC_STRUCTURE_DESIGN.md`).
//!
//! This is the FREE-ELECTRON entry, the piece buildable on the current floor plus the periodic table, reserving no
//! value:
//!
//! - [`carrier_density_per_nm3`]: `n_e = z * rho * N_A / M`, the conduction-electron density, the conduction
//!   electrons per atom `z` (from the periodic-table valence, DATA) times the atomic number density (the built
//!   density over the molar mass). Stored in a `/nm^3` WORKING UNIT (the SI `~1e28 /m^3` overflows Q32.32; the
//!   `N_A * 1e-21` cm^3-to-nm^3 fold, derived from Avogadro, brings it to `~1..200 /nm^3`, representable, the
//!   range-census verdict).
//! - [`plasma_energy_ev`]: the plasma energy `hbar * omega_p = hbar * sqrt(n_e e^2 / (epsilon_0 m_e))`, returned in
//!   eV (`~5..16 eV`, representable, versus the SI `omega_p ~ 1e16 /s` which does not fit). The fold
//!   `hbar * sqrt(1e27 / (epsilon_0 m_e))` is ASSEMBLED from the verified SI execution capability and an exact
//!   unit power, with no folded dimensional decimal or independent physical constant path.
//!
//! GROUNDING (honest FEW-percent grade, sp-metals): sodium `5.92 eV` (measured `~5.7`), magnesium `10.9`
//! (`~10.6`), aluminium `15.8` (`~15.3`). THE NAMED d-BLOCK EXHIBIT: silver's free-electron plasma energy is
//! `~9.0 eV` against the OBSERVED screened plasmon `~3.8 eV`, a factor-`2.4` miss from d-band interband screening
//! (Ehrenreich and Philipp, Phys. Rev. 128, 1622 (1962)) that the free-electron model cannot see. That failure is
//! the motivation for the deep piece (the band structure), not a defect in this entry, which is scoped to the
//! sp-metals with the d-block flagged. The Drude conductivity, the gap tier, the DOS/magnetism, and the optics are
//! the later slices; each contested piece is ruled before it is built.
//!
//! Byte-neutral: `civsim-materials` is a leaf, not linked into the run_world binary.

use civsim_core::Fixed;
use civsim_physics::metal_eos::MetalEosAnchors;
use civsim_physics::periodic::PeriodicTable;
use civsim_units::constants::{SiExecutionMagnitudes, SiRepresentationMagnitudes};

use crate::properties::density_g_per_cm3;

const ZERO: Fixed = Fixed::ZERO;

/// The `cm^3 -> nm^3` carrier-density fold `N_A * 1e-21` (`~602.214076`), derived from Avogadro's number and the
/// exact `1 cm^3 = 1e21 nm^3` power (no authored decimal): `from_ratio(602214076, 1000000)`. This maps
/// `z * rho / M` (in `/cm^3`) to the representable `/nm^3` working unit (the SI `/m^3` value `~1e28` overflows).
fn avogadro_per_nm3_fold(representation: &SiRepresentationMagnitudes) -> Option<Fixed> {
    representation.avogadro_per_nm3_fold()
}

/// The conduction-electron (carrier) density `n_e` in the `/nm^3` working unit:
/// `n_e = z * rho * (N_A * 1e-21) / M`, the conduction electrons per atom `z` times the atomic number density.
/// Reserves NO value: `z` is DATA (the periodic-table valence for a simple metal; the d-band effective count is the
/// flagged follow-on), `rho` and `M` are floor quantities, and the fold is derived from Avogadro. Non-positive
/// inputs yield zero.
pub fn carrier_density_per_nm3(
    representation: &SiRepresentationMagnitudes,
    conduction_electrons_z: Fixed,
    mass_density_g_per_cm3: Fixed,
    molar_mass_g_per_mol: Fixed,
) -> Option<Fixed> {
    if conduction_electrons_z <= ZERO
        || mass_density_g_per_cm3 <= ZERO
        || molar_mass_g_per_mol <= ZERO
    {
        return Some(ZERO);
    }
    conduction_electrons_z
        .checked_mul(mass_density_g_per_cm3)
        .and_then(|x| x.checked_mul(avogadro_per_nm3_fold(representation)?))
        .and_then(|x| x.checked_div(molar_mass_g_per_mol))
}

/// The plasma-energy fold `C = hbar * sqrt(1e27 / (epsilon_0 * m_e))` (`~1.174 eV * nm^(3/2)`), mapping
/// `sqrt(n_e[/nm^3])` to the plasma energy in eV. ASSEMBLED from the exact SI mantissas and a single power of ten,
/// the dimensionless-constant law (no folded dimensional decimal): the powers collapse to
/// `C = (1.054571817 * 10) / sqrt(8.8541878128 * 9.1093837015)`, since `hbar` carries `10^-34`, `sqrt(1e27)` carries
/// `10^13.5`, and `sqrt(epsilon_0 * m_e)` carries `10^-21.5`, netting `10^1`. Runtime assembly reads the sealed
/// projected ancestry from [`SiExecutionMagnitudes`]; the displayed decimal decomposition is explanatory only.
fn plasma_energy_fold(execution: &SiExecutionMagnitudes) -> Option<Fixed> {
    execution.plasma_energy_fold_ev_nm_three_halves()
}

/// The plasma energy `hbar * omega_p` (eV) from the carrier density (`/nm^3`):
/// `hbar * omega_p = plasma_energy_fold() * sqrt(n_e)`. The eV energy (`~5..16` for a metal) is representable where
/// the SI `omega_p ~ 1e16 /s` is not. Reserves no value. Non-positive input yields zero.
pub fn plasma_energy_ev(
    execution: &SiExecutionMagnitudes,
    carrier_density_per_nm3: Fixed,
) -> Option<Fixed> {
    if carrier_density_per_nm3 <= ZERO {
        return Some(ZERO);
    }
    plasma_energy_fold(execution)?.checked_mul(carrier_density_per_nm3.sqrt())
}

/// The scattering-time fold `hbar * 1e15 / (2*pi*k_B)` (`~1215.7 fs*K`), mapping `1/(lambda_tr*T)` to the
/// relaxation time in femtoseconds. ASSEMBLED from the exact `hbar` and `k_B` mantissas, `2*pi` from `Fixed::PI`,
/// and a single power of ten (the dimensionless-constant law): `(1.054571817 / (2*pi*1.380649)) * 1e4`. The `fs`
/// working unit keeps `tau ~ 20..40 fs` representable where the SI `~2e-14 s` underflows Q32.32.
fn scattering_time_fold_fs_k(representation: &SiRepresentationMagnitudes) -> Option<Fixed> {
    representation.scattering_time_fold_fs_k()
}

/// The phonon-limited Drude scattering time `tau` (fs), the high-temperature (`T > Theta_D`) form
/// `hbar / tau = 2*pi*lambda_tr*k_B*T`, so `tau = scattering_time_fold_fs_k() / (lambda_tr * T)`. The ONE RESERVED
/// coefficient is the dimensionless transport electron-phonon coupling `lambda_tr` (`[M]` per material, McMillan
/// 1968 / Allen 1971, the SAME `lambda` Eliashberg consumes for superconducting `T_c`, a dual-consumer column),
/// caller-supplied and never planted (`~0.16` for copper). HONEST LIMITS: this is the `T > Theta_D` linear-in-`T`
/// regime; below `Theta_D` the Bloch-Grueneisen `T^5` law takes over (a derived-in-form follow-on), and a defect
/// residual-resistivity term adds by Matthiessen (tying to the damage floor). Non-positive inputs yield zero.
pub fn drude_scattering_time_fs(
    representation: &SiRepresentationMagnitudes,
    lambda_tr: Fixed,
    temperature: Fixed,
) -> Option<Fixed> {
    if lambda_tr <= ZERO || temperature <= ZERO {
        return Some(ZERO);
    }
    let denom = lambda_tr.checked_mul(temperature)?;
    (denom > ZERO)
        .then_some(())
        .and_then(|()| scattering_time_fold_fs_k(representation)?.checked_div(denom))
}

/// The Drude conductivity fold `e^2 * 1e12 / m_e` (`~2.818e4`), mapping `n_e[/nm^3] * tau[fs]` to `sigma[S/m]`.
/// ASSEMBLED from the sealed `e` and `m_e` projections and a single exact unit power:
/// `(1.602176634^2 / 9.1093837015) * 1e5`, since `e^2` carries `10^-38`, the `n_e` cm-to-nm and `tau` fs
/// conversions carry `10^(27-15) = 10^12`, and `m_e` carries `10^-31`, netting `10^5`.
fn drude_conductivity_fold(execution: &SiExecutionMagnitudes) -> Option<Fixed> {
    execution.drude_conductivity_fold()
}

/// The Drude conductivity `sigma` (S/m) from the carrier density (`/nm^3`) and the scattering time (fs):
/// `sigma = n_e * e^2 * tau / m_e`, folded to `n_e[/nm^3] * tau[fs] * drude_conductivity_fold()`. This is the
/// fundamental Drude relation, no reserved value; the reserved coupling enters through `tau`. The `S/m` value
/// (`~1e5..1e8` for a metal) is representable. This is the leg the `sigma` ROUND-TRIP TEST exercises: a `tau` that
/// yields a cited resistivity, run back through here, must rebuild that resistivity, so a units fold fails loudly.
/// Non-positive inputs yield zero.
pub fn drude_conductivity_from_tau(
    execution: &SiExecutionMagnitudes,
    carrier_density_per_nm3: Fixed,
    scattering_time_fs: Fixed,
) -> Option<Fixed> {
    if carrier_density_per_nm3 <= ZERO || scattering_time_fs <= ZERO {
        return Some(ZERO);
    }
    carrier_density_per_nm3
        .checked_mul(scattering_time_fs)
        .and_then(|x| x.checked_mul(drude_conductivity_fold(execution)?))
}

/// The Drude electrical conductivity `sigma` (S/m) from the carrier density, the reserved transport coupling
/// `lambda_tr`, and the temperature: the phonon-limited `tau` ([`drude_scattering_time_fs`]) into the Drude
/// relation ([`drude_conductivity_from_tau`]). Reserves the one coefficient `lambda_tr`. HONEST LIMITS: the
/// free-electron Drude form is few-percent for a good simple metal and degrades for the d-block (the band mass);
/// the Mott-Ioffe-Regel bound (the mean free path cannot fall below a lattice spacing; Gunnarsson-Calandra-Han
/// 2003) marks where Drude itself dies, the resistivity-saturation ceiling. Non-positive inputs yield zero.
pub fn drude_conductivity_s_per_m(
    execution: &SiExecutionMagnitudes,
    carrier_density_per_nm3: Fixed,
    lambda_tr: Fixed,
    temperature: Fixed,
) -> Option<Fixed> {
    let tau = drude_scattering_time_fs(&execution.representation(), lambda_tr, temperature)?;
    if tau <= ZERO {
        return Some(ZERO);
    }
    drude_conductivity_from_tau(execution, carrier_density_per_nm3, tau)
}

/// The electronic route bound to the periodic table and the EOS anchors, so the free-electron density and the
/// plasma energy read the molar mass and the derived density for an anchored metal. The conduction-electron count
/// `z` is caller-supplied DATA (the periodic-table valence for a simple metal; the d-band effective count is the
/// flagged follow-on), never planted. A metal missing an anchor escalates (`None`) rather than fabricating.
pub struct ElectronicRoute<'a> {
    table: &'a PeriodicTable,
    anchors: &'a MetalEosAnchors,
    execution: &'a SiExecutionMagnitudes,
}

impl<'a> ElectronicRoute<'a> {
    /// Bind the electronic route to the periodic table (the molar mass) and the EOS anchors (the molar volume, for
    /// the density).
    pub fn new(
        table: &'a PeriodicTable,
        anchors: &'a MetalEosAnchors,
        execution: &'a SiExecutionMagnitudes,
    ) -> Self {
        ElectronicRoute {
            table,
            anchors,
            execution,
        }
    }

    /// The density `rho` (g/cm^3) for an anchored metal, from the molar mass and the anchored molar volume.
    fn density(&self, symbol: &str) -> Option<Fixed> {
        let molar_volume = self.anchors.molar_volume(symbol)?;
        let molar_mass = self.table.element(symbol)?.standard_atomic_weight;
        if molar_mass <= ZERO {
            return None;
        }
        Some(density_g_per_cm3(molar_mass, molar_volume))
    }

    /// The conduction-electron density `n_e` (`/nm^3`) for an anchored metal, with the caller's conduction-electron
    /// count `z`. `None` (escalate) when the metal has no anchored molar volume or no standard atomic weight.
    pub fn carrier_density(&self, symbol: &str, conduction_electrons_z: Fixed) -> Option<Fixed> {
        let molar_mass = self.table.element(symbol)?.standard_atomic_weight;
        let rho = self.density(symbol)?;
        carrier_density_per_nm3(
            &self.execution.representation(),
            conduction_electrons_z,
            rho,
            molar_mass,
        )
    }

    /// The plasma energy `hbar * omega_p` (eV) for an anchored metal, from the free-electron density. `None`
    /// (escalate) when the metal has no anchor. HONEST LIMIT: a free-electron value, few-percent for an sp-metal
    /// and a factor-two overestimate for a d-band metal (the silver exhibit), where the band structure is needed.
    pub fn plasma_energy(&self, symbol: &str, conduction_electrons_z: Fixed) -> Option<Fixed> {
        let n_e = self.carrier_density(symbol, conduction_electrons_z)?;
        plasma_energy_ev(self.execution, n_e)
    }

    /// The Drude electrical conductivity `sigma` (S/m) for an anchored metal at a temperature, over the free-
    /// electron density and the caller's reserved transport coupling `lambda_tr`. `None` (escalate) when the metal
    /// has no anchor. Both `z` and `lambda_tr` are caller-supplied, never planted. Carries the free-electron and
    /// Mott-Ioffe-Regel limits of [`drude_conductivity_s_per_m`].
    pub fn conductivity(
        &self,
        symbol: &str,
        conduction_electrons_z: Fixed,
        lambda_tr: Fixed,
        temperature: Fixed,
    ) -> Option<Fixed> {
        let n_e = self.carrier_density(symbol, conduction_electrons_z)?;
        drude_conductivity_s_per_m(self.execution, n_e, lambda_tr, temperature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execution() -> SiExecutionMagnitudes {
        civsim_units::constants::canonical_si_execution_magnitudes()
            .expect("the sealed physical floor projects")
    }

    fn table() -> PeriodicTable {
        PeriodicTable::standard().expect("periodic table")
    }
    fn anchors() -> MetalEosAnchors {
        MetalEosAnchors::standard().expect("metal EOS anchors")
    }
    fn close(a: Fixed, b: f64, tol: f64) -> bool {
        (a.to_f64_lossy() - b).abs() < tol
    }

    #[test]
    fn the_free_electron_density_and_plasma_energy_land_the_sp_metals() {
        let execution = execution();
        let representation = execution.representation();
        // Sodium: z = 1, rho = 0.97 g/cm^3, M = 22.99 g/mol -> n_e = 1 * 0.97 * 602.214 / 22.99 ~25.4 /nm^3.
        let n_na = carrier_density_per_nm3(
            &representation,
            Fixed::from_int(1),
            Fixed::from_ratio(97, 100),
            Fixed::from_ratio(2299, 100),
        )
        .expect("the carrier density derives");
        assert!(
            close(n_na, 25.4, 0.5),
            "sodium carrier density ~25.4 /nm^3: {n_na:?}"
        );
        // Plasma energy: 1.174 * sqrt(25.4) ~5.92 eV against the measured ~5.7 (few-percent sp-metal grade).
        let ep_na = plasma_energy_ev(&execution, n_na).expect("the plasma energy derives");
        assert!(
            close(ep_na, 5.92, 0.15),
            "sodium plasma energy ~5.92 eV (measured ~5.7): {ep_na:?}"
        );

        // The fold is the derived ~1.174 eV*nm^1.5, assembled from hbar/eps0/m_e mantissas (no folded decimal).
        assert!(
            close(
                plasma_energy_fold(&execution).expect("the fold derives"),
                1.174,
                0.01
            ),
            "plasma fold ~1.174: {:?}",
            plasma_energy_fold(&execution)
        );
        // Higher valence or denser packing raises n_e; monotone plasma energy in n_e.
        let n_al = carrier_density_per_nm3(
            &representation,
            Fixed::from_int(3),
            Fixed::from_ratio(270, 100),
            Fixed::from_ratio(2698, 100),
        )
        .expect("the carrier density derives");
        assert!(
            n_al > n_na,
            "aluminium (z=3, dense) has more carriers than sodium"
        );
        assert!(
            close(
                plasma_energy_ev(&execution, n_al).expect("the plasma energy derives"),
                15.8,
                0.6
            ),
            "aluminium plasma energy ~15.8 eV (measured ~15.3): {:?}",
            plasma_energy_ev(&execution, n_al)
        );
        // Guards.
        assert_eq!(
            carrier_density_per_nm3(
                &representation,
                ZERO,
                Fixed::from_int(1),
                Fixed::from_int(1)
            ),
            Some(ZERO)
        );
        assert_eq!(plasma_energy_ev(&execution, ZERO), Some(ZERO));
    }

    #[test]
    fn the_silver_d_block_exhibit_overestimates_by_the_screening_factor() {
        let execution = execution();
        // THE NAMED d-BLOCK FAILURE (not a defect): silver z = 1, rho = 10.49 g/cm^3, M = 107.868 g/mol ->
        // n_e ~58.6 /nm^3 -> free-electron plasma energy ~9.0 eV, against the OBSERVED screened plasmon ~3.8 eV, a
        // factor ~2.4 miss from d-band interband screening (Ehrenreich-Philipp 1962). The free-electron entry is
        // scoped to sp-metals; this row is why the d-block needs the deep band-structure piece.
        let n_ag = carrier_density_per_nm3(
            &execution.representation(),
            Fixed::from_int(1),
            Fixed::from_ratio(1049, 100),
            Fixed::from_ratio(107868, 1000),
        )
        .expect("the carrier density derives");
        assert!(
            close(n_ag, 58.6, 1.0),
            "silver carrier density ~58.6 /nm^3: {n_ag:?}"
        );
        let ep_ag = plasma_energy_ev(&execution, n_ag).expect("the plasma energy derives");
        assert!(
            close(ep_ag, 9.0, 0.3),
            "silver FREE-ELECTRON plasma energy ~9.0 eV (the model's prediction): {ep_ag:?}"
        );
        // The free-electron prediction is far above the observed 3.8 eV: the documented d-screening exhibit.
        assert!(
            ep_ag.to_f64_lossy() > 2.0 * 3.8,
            "the free-electron value overshoots the observed 3.8 eV by the d-screening factor"
        );
    }

    #[test]
    fn the_electronic_route_reads_the_anchors_and_escalates_unanchored() {
        let t = table();
        let a = anchors();
        let execution = execution();
        let route = ElectronicRoute::new(&t, &a, &execution);

        // Sodium through the substrate (density from molar mass / anchored molar volume): plasma energy ~5.9 eV.
        let ep_na = route
            .plasma_energy("Na", Fixed::from_int(1))
            .expect("Na plasma");
        assert!(
            close(ep_na, 5.9, 0.3),
            "route sodium plasma energy ~5.9 eV: {ep_na:?}"
        );
        // Magnesium (z = 2) lands ~10.9 eV against the measured ~10.6 (few-percent).
        let ep_mg = route
            .plasma_energy("Mg", Fixed::from_int(2))
            .expect("Mg plasma");
        assert!(
            close(ep_mg, 10.9, 0.5),
            "route magnesium plasma energy ~10.9 eV: {ep_mg:?}"
        );
        // Aluminium (z = 3) lands ~15.8 eV.
        let ep_al = route
            .plasma_energy("Al", Fixed::from_int(3))
            .expect("Al plasma");
        assert!(
            close(ep_al, 15.8, 0.6),
            "route aluminium plasma energy ~15.8 eV: {ep_al:?}"
        );
        // An unanchored metal escalates rather than fabricating.
        assert!(
            route.carrier_density("Xx", Fixed::from_int(1)).is_none(),
            "an unanchored symbol has no carrier density"
        );
        assert!(
            route.plasma_energy("Xx", Fixed::from_int(1)).is_none(),
            "an unanchored symbol has no plasma energy"
        );
    }

    #[test]
    fn the_drude_conductivity_closes_on_the_transport_coupling_and_round_trips_tau() {
        let execution = execution();
        let representation = execution.representation();
        // Copper: n_e ~85 /nm^3, lambda_tr ~0.16 (cited test-only, McMillan/Allen), T = 300 K.
        // tau = 1215.7 / (0.16*300) ~25.3 fs; sigma = n_e * tau * fold ~6.06e7 S/m against the measured ~5.9e7.
        let n_cu = Fixed::from_int(85);
        let lambda_cu = Fixed::from_ratio(16, 100);
        let tau = drude_scattering_time_fs(&representation, lambda_cu, Fixed::from_int(300))
            .expect("the scattering time derives");
        assert!(
            close(tau, 25.3, 1.0),
            "copper scattering time ~25 fs: {tau:?}"
        );
        let sigma = drude_conductivity_s_per_m(&execution, n_cu, lambda_cu, Fixed::from_int(300))
            .expect("the conductivity derives");
        assert!(
            close(sigma, 6.06e7, 6.0e6),
            "copper conductivity ~6e7 S/m (measured ~5.9e7): {sigma:?}"
        );

        // THE sigma ROUND-TRIP TEST (the owner's requirement): a tau that yields copper's cited sigma (5.88e7),
        // run back through the Drude relation, must rebuild that sigma, so a units fold fails loudly. Copper's
        // cited resistivity 1.7e-8 ohm*m -> sigma 5.88e7 -> the physical tau is ~24.6 fs; recompute and assert.
        let tau_from_cited = Fixed::from_ratio(246, 10); // 24.6 fs, backed out of the cited resistivity
        let sigma_round = drude_conductivity_from_tau(&execution, n_cu, tau_from_cited)
            .expect("the conductivity derives");
        assert!(
            close(sigma_round, 5.88e7, 3.0e6),
            "the tau that yields the cited resistivity rebuilds sigma ~5.9e7 (units round-trip): {sigma_round:?}"
        );

        // Monotone: a stronger coupling (more scattering) shortens tau and lowers sigma; a higher temperature too.
        assert!(
            drude_scattering_time_fs(
                &representation,
                Fixed::from_ratio(30, 100),
                Fixed::from_int(300)
            )
            .expect("the scattering time derives")
                < tau,
            "a stronger transport coupling shortens the scattering time"
        );
        assert!(
            drude_conductivity_s_per_m(
                &execution,
                n_cu,
                Fixed::from_ratio(30, 100),
                Fixed::from_int(300)
            )
            .expect("the conductivity derives")
                < sigma,
            "a stronger coupling lowers the conductivity"
        );
        assert!(
            drude_conductivity_s_per_m(&execution, n_cu, lambda_cu, Fixed::from_int(600))
                .expect("the conductivity derives")
                < sigma,
            "a higher temperature lowers the conductivity (more phonon scattering)"
        );
        // Guards.
        assert_eq!(
            drude_scattering_time_fs(&representation, ZERO, Fixed::from_int(300)),
            Some(ZERO)
        );
        assert_eq!(
            drude_conductivity_from_tau(&execution, ZERO, tau),
            Some(ZERO)
        );
        assert_eq!(
            drude_conductivity_s_per_m(&execution, n_cu, ZERO, Fixed::from_int(300)),
            Some(ZERO)
        );

        // Through the route (reads n_e from the anchors; z and lambda_tr caller-supplied). Sodium lambda_tr ~0.11.
        let t = table();
        let a = anchors();
        let route = ElectronicRoute::new(&t, &a, &execution);
        let sigma_na = route
            .conductivity(
                "Na",
                Fixed::from_int(1),
                Fixed::from_ratio(11, 100),
                Fixed::from_int(300),
            )
            .expect("Na conductivity");
        assert!(
            sigma_na.to_f64_lossy() > 1.0e7 && sigma_na.to_f64_lossy() < 4.0e7,
            "route sodium conductivity is a sensible metal value ~2e7 S/m: {sigma_na:?}"
        );
        assert!(
            route
                .conductivity(
                    "Xx",
                    Fixed::from_int(1),
                    Fixed::from_ratio(16, 100),
                    Fixed::from_int(300)
                )
                .is_none(),
            "an unanchored metal escalates in the conductivity route"
        );
    }
}
