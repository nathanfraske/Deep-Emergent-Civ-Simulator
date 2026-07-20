// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.

//! THE FLEXURAL RELIEF PROFILE over a row of derived crustal columns: the run-path consumer of the flexure
//! substrate, and the lateral-coupling generalization of the per-column Airy elevation
//! [`crate::deeptime::relax_to_support_bound`] computes today.
//!
//! # WHAT THIS REPLACES, AND WHY IT IS A GENERALIZATION RATHER THAN A RIVAL
//!
//! `relax_to_support_bound` derives the Airy buoyant fraction `k = (rho_m - rho_c) / rho_m` and stands each
//! column at `k * thickness`. That reads ONE column's own properties: no neighbour enters, so relief is exactly
//! as rough as the thickness field, and a boundary between two crustal types is a step at a single coordinate.
//!
//! Flexure carries bending stress across the boundary. The load is the same buoyancy anomaly
//! ([`civsim_physics::geodynamics::column_buoyancy_load`]) and the far field of a wide column returns the same
//! Airy elevation, so this contains the flotation law rather than competing with it: Airy is its `D -> 0` limit,
//! measured in `civsim_physics::flexural_relief` down to a residual of `3.26e-9 km`.
//!
//! # WHY THIS IS A PROFILE AND NOT YET A FIELD, WHICH IS A REAL LIMIT AND NOT A STAGING CHOICE
//!
//! The distributed load this substrate carries is [`civsim_physics::flexure::LoadKind::UniformStripY`], a strip
//! INFINITE along `y`. Along one row of provinces at fixed latitude that is exactly right: the row's neighbours
//! in longitude are what bend it, and treating each province as a band running perpendicular to the row is the
//! geometry the Green's function was integrated for.
//!
//! A full 2-D province FIELD is a different question. Each province is a finite patch, and superposing strips
//! over a two-dimensional grid would treat every province as an infinite band in `y`, which double-counts along
//! the row and omits meridional coupling. That needs a compactly supported 2-D load (a disc, a rectangle) whose
//! Green's function this substrate does not carry: `LoadKind::Point` is a DELTA and therefore singular in the
//! axisymmetric case the same way the line load was in the one-dimensional case, which is the gap the strip
//! closed for 1-D and nothing has closed for 2-D. Building the field on strips would be a modelling error
//! carried in code, so it is named here and left unbuilt.
//!
//! DORMANT: nothing on a pinned run path calls this, so both byte pins hold bit-exact. Deterministic fixed
//! point in index order (Principle 3).

use civsim_core::Fixed;
use civsim_physics::flexural_relief::{FlexedPlate, ReliefRefusal};
use civsim_physics::geodynamics::column_buoyancy_load;

/// Why a profile could not be built. Every arm is a stop; nothing falls back to a per-column Airy answer,
/// because a silent fallback to the law this generalizes is the one failure that would look like success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileRefusal {
    /// The row carries no columns.
    EmptyRow,
    /// A column's density, the mantle density, the gravity or the cell width is non-physical.
    NonPhysicalColumn,
    /// The flexure substrate refused, carrying its own reason.
    Relief(ReliefRefusal),
}

/// THE FLEXURAL RELIEF PROFILE across one row of crustal columns, in kilometres above the compensation
/// reference, one value per column.
///
/// `thickness_km` is the row's derived crustal thickness per column (the deep-time ledger's own
/// `crust_thickness_km` slice for that row). `crust_density` and `mantle_density` are in `1000 kg/m^3` and
/// `gravity_km_s2` in `km/s^2`, the flexure substrate's coherent system: a caller holding the deep-time
/// `SupportBoundParams`, whose densities are in `kg/m^3` and whose gravity is in `m/s^2`, divides each by a
/// thousand ONCE at the boundary. That conversion is stated here rather than hidden, because feeding `2900` and
/// `2.9` to two functions that both call it a density is the drift the ledger's own unit note warns about.
///
/// `cell_width_km` is the province grid's own spacing, itself derived from the convection cell size, and each
/// column loads the plate over its own half of that. Nothing here declares a footprint.
///
/// THE ROW IS NOT WRAPPED. A province row on a globe closes on itself, and this evaluates a finite line of
/// columns instead, so the two end columns are missing the neighbours that would bend them from the far side.
/// Wrapping needs either a periodic Green's function or a summed image series, neither of which this substrate
/// carries; a caller reading the ends should know they are edge columns. Stated rather than silently wrong.
// @derives: the flexural relief profile over a column row <- each column's buoyancy load, the plate rigidity and the grid spacing
pub fn flexural_relief_profile_km(
    thickness_km: &[Fixed],
    crust_density: Fixed,
    mantle_density: Fixed,
    gravity_km_s2: Fixed,
    cell_width_km: Fixed,
    plate: &FlexedPlate,
) -> Result<Vec<Fixed>, ProfileRefusal> {
    if thickness_km.is_empty() {
        return Err(ProfileRefusal::EmptyRow);
    }
    if cell_width_km <= Fixed::ZERO {
        return Err(ProfileRefusal::NonPhysicalColumn);
    }
    let half_width = cell_width_km
        .checked_div(Fixed::from_int(2))
        .ok_or(ProfileRefusal::NonPhysicalColumn)?;

    // Every column becomes a load at its own centre. Built once and shared by every query, since the profile is
    // the SAME superposition evaluated at each column's position: a column's relief is the whole row's doing.
    let mut loads = Vec::with_capacity(thickness_km.len());
    for (i, t) in thickness_km.iter().enumerate() {
        let centre = Fixed::from_int(i32::try_from(i).map_err(|_| ProfileRefusal::EmptyRow)?)
            .checked_mul(cell_width_km)
            .ok_or(ProfileRefusal::NonPhysicalColumn)?;
        let load = column_buoyancy_load(
            crust_density,
            mantle_density,
            *t,
            gravity_km_s2,
            centre,
            half_width,
        )
        .ok_or(ProfileRefusal::NonPhysicalColumn)?;
        loads.push(load);
    }

    let mut profile = Vec::with_capacity(thickness_km.len());
    for i in 0..thickness_km.len() {
        let centre = Fixed::from_int(i32::try_from(i).map_err(|_| ProfileRefusal::EmptyRow)?)
            .checked_mul(cell_width_km)
            .ok_or(ProfileRefusal::NonPhysicalColumn)?;
        let w = plate
            .deflection_km(&loads, centre, Fixed::ZERO)
            .map_err(ProfileRefusal::Relief)?;
        profile.push(w);
    }
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::flexure::flexural_rigidity;

    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    fn earthlike_plate() -> FlexedPlate {
        let d = flexural_rigidity(
            Fixed::from_int(70),
            Fixed::from_ratio(1, 4),
            Fixed::from_int(40),
        )
        .expect("rigidity");
        FlexedPlate::from_rigidity_gpa_km3(
            d,
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
        )
        .expect("plate")
    }

    /// A row with one thick column in the middle, the rest thin: a range and its surroundings.
    fn ridge_row() -> Vec<Fixed> {
        let mut row = vec![Fixed::from_int(30); 11];
        row[5] = Fixed::from_int(60);
        row
    }

    #[test]
    fn a_thick_column_stands_high_and_its_neighbours_are_raised_with_it() {
        // THE LATERAL TERM, which is the whole reason this exists. Under Airy the neighbours of a thick column
        // are unaffected by it: each stands at `k * its own thickness`, so ten identical 30 km columns stand at
        // ten identical elevations no matter what sits between them. Under flexure the range lifts its
        // surroundings, and the lift DECAYS with distance, which is the neighbourhood the per-column law cannot
        // represent at any parameter.
        let profile = flexural_relief_profile_km(
            &ridge_row(),
            Fixed::from_ratio(29, 10),
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
            Fixed::from_int(60),
            &earthlike_plate(),
        )
        .expect("the row has a profile");
        assert_eq!(profile.len(), 11);
        let peak = f64_of(profile[5]);
        for (i, v) in profile.iter().enumerate() {
            if i != 5 {
                assert!(
                    f64_of(*v) < peak,
                    "the thick column stands highest: index {i} at {} against {peak}",
                    f64_of(*v)
                );
            }
        }
        // The lift DECAYS outward from the range on both sides.
        assert!(f64_of(profile[4]) > f64_of(profile[3]));
        assert!(f64_of(profile[3]) > f64_of(profile[2]));
        assert!(f64_of(profile[6]) > f64_of(profile[7]));
        assert!(f64_of(profile[7]) > f64_of(profile[8]));
        eprintln!(
            "ridge profile (km): {:?}",
            profile.iter().map(|v| f64_of(*v)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_uniform_row_is_symmetric_which_is_what_makes_the_lift_a_neighbour_effect() {
        // THE CONTROL, and it is NOT flatness. This test asserted a flat interior first, from the intuition that
        // identical columns must stand at identical heights, and it was wrong: the strip Green's function goes
        // NEGATIVE beyond `z = 3 pi / 4`, so at a 60 km spacing against an 83.8 km flexural parameter every
        // neighbour past about 197 km pulls DOWN rather than up. Each column therefore sees a different mixture
        // of lifting and pulling neighbours, and a finite unwrapped row of identical columns is genuinely not
        // flat. Measured rather than assumed: index 4 sits above index 5 by about 1.0e5 raw units.
        //
        // The invariant that DOES hold is SYMMETRY. A uniform row is its own mirror image, so column `i` and
        // column `n - 1 - i` must agree to the bit. That is a real property of the superposition rather than a
        // guess about it, and it fails immediately if the load list is built with an off-by-one centre or if the
        // Green's function is evaluated with a signed rather than an absolute distance.
        let profile = flexural_relief_profile_km(
            &vec![Fixed::from_int(30); 11],
            Fixed::from_ratio(29, 10),
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
            Fixed::from_int(60),
            &earthlike_plate(),
        )
        .expect("profile");
        let n = profile.len();
        for i in 0..n {
            assert_eq!(
                profile[i].to_bits(),
                profile[n - 1 - i].to_bits(),
                "a uniform row is its own mirror: index {i} against {}",
                n - 1 - i
            );
        }
        // And the ends sit LOWEST, missing the neighbours beyond the row that would have lifted them.
        assert!(
            f64_of(profile[0]) < f64_of(profile[5]),
            "the unwrapped end is short of its missing neighbours: {} against {}",
            f64_of(profile[0]),
            f64_of(profile[5])
        );
    }

    #[test]
    fn it_refuses_rather_than_falling_back_to_the_law_it_generalizes() {
        let plate = earthlike_plate();
        let g = Fixed::from_ratio(98, 10_000);
        assert_eq!(
            flexural_relief_profile_km(
                &[],
                Fixed::from_ratio(29, 10),
                Fixed::from_ratio(33, 10),
                g,
                Fixed::from_int(60),
                &plate
            ),
            Err(ProfileRefusal::EmptyRow)
        );
        assert_eq!(
            flexural_relief_profile_km(
                &ridge_row(),
                Fixed::from_ratio(29, 10),
                Fixed::from_ratio(33, 10),
                g,
                Fixed::ZERO,
                &plate
            ),
            Err(ProfileRefusal::NonPhysicalColumn)
        );
        assert_eq!(
            flexural_relief_profile_km(
                &ridge_row(),
                Fixed::from_ratio(29, 10),
                Fixed::ZERO,
                g,
                Fixed::from_int(60),
                &plate
            ),
            Err(ProfileRefusal::NonPhysicalColumn)
        );
    }
}
