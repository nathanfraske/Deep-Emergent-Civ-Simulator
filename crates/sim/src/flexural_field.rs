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
//! # TWO ENTRY POINTS, AND ONLY THE FINITE ONE IS A DIAGNOSTIC
//!
//! [`flexural_relief_profile_km`] evaluates the row as a FINITE line of columns while the Green's functions
//! are the INFINITE-plate ones. On a globe a province row closes on itself, so the physical domain is
//! periodic. Truncating the source list instead leaves every column short of the neighbours beyond the ends,
//! and the shortfall is largest at the ends and smallest in the middle.
//!
//! That truncation has a visible consequence which this module got WRONG at first and a review corrected: a
//! row of IDENTICAL columns does not come out flat there. On a translationally invariant infinite or periodic
//! plate a uniform load has a uniform response, necessarily, so the variation is a boundary artifact of the
//! truncation and NOT a derived free-edge condition or a property of the kernel. The first version of the
//! uniform-row test recorded that variation as though it were physics.
//!
//! So the finite profile remains a DIAGNOSTIC: correct for the interior of a row long against the flexural
//! parameter, and not to be read as a causal elevation field for a closed row.
//!
//! [`periodic_flexural_relief_profile_km`] is the closed-row answer, and it retires exactly that limitation
//! and no other. It solves the plate equation spectrally on the wrapped domain
//! ([`civsim_physics::flexural_relief::FlexedPlate::periodic_elevation_km`]), so a uniform closed row comes
//! out FLAT TO THE BIT rather than symmetric-but-sagging, and the ends are no longer edge columns because
//! there are no ends.
//!
//! WHAT THE PERIODIC PATH DOES NOT RETIRE, stated so the retirement is not read wider than it is. It is still
//! a PROFILE along one row rather than a 2-D field, which is the separate strip-geometry gap above. It still
//! carries ONE rigidity for the whole row, which is the SLOWLY-VARYING-RIGIDITY APPROXIMATION named at
//! [`civsim_physics::flexural_relief::FlexedPlate::periodic_deflection_km`] and measured there rather than
//! assumed. It resolves the load only down to the province grid's own Nyquist wavelength. And it is still
//! DORMANT.
//!
//! DORMANT: nothing on a pinned run path calls either function, so both byte pins hold bit-exact.
//! Deterministic fixed point in index order (Principle 3).

use civsim_core::Fixed;
use civsim_physics::flexural_relief::{ElevationKm, FlexedPlate, ReliefRefusal};
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
/// THIS ROW IS NOT WRAPPED. A province row on a globe closes on itself, and this evaluates a finite line of
/// columns instead, so the two end columns are missing the neighbours that would bend them from the far side.
/// A caller reading the ends should know they are edge columns.
///
/// THAT IS A PROPERTY OF THIS FUNCTION AND NOT OF THE SUBSTRATE, and the distinction was got wrong here. This
/// paragraph used to say wrapping "needs either a periodic Green's function or a summed image series, neither
/// of which this substrate carries", which was FALSE against the repository's own contents: the spectral
/// transfer function [`civsim_physics::flexure::flexural_response_ratio`] had been in the flexure kernel since
/// the filter slice landed, with no caller outside its own tests. A limitation was recorded as an absence, and
/// the absent thing was two modules away. Use [`periodic_flexural_relief_profile_km`] for a closed row.
// @derives: the flexural relief profile over a column row <- each column's buoyancy load, the plate rigidity and the grid spacing
pub fn flexural_relief_profile_km(
    thickness_km: &[Fixed],
    crust_density: Fixed,
    mantle_density: Fixed,
    gravity_km_s2: Fixed,
    cell_width_km: Fixed,
    plate: &FlexedPlate,
) -> Result<Vec<ElevationKm>, ProfileRefusal> {
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
        // ELEVATION, through the typed boundary. This pushed the raw DOWNWARD deflection while documenting
        // and testing itself as height above the compensation reference, so its peak was a hole wearing the
        // word mountain until a review caught it.
        let e = plate
            .elevation_km(&loads, centre, Fixed::ZERO)
            .map_err(ProfileRefusal::Relief)?;
        profile.push(e);
    }
    Ok(profile)
}

/// THE CLOSED-ROW FLEXURAL RELIEF PROFILE: the same columns, the same derived buoyancy loads, solved on a
/// domain that WRAPS.
///
/// A province row on a globe has no ends, so this is the physically correct reading of one and
/// [`flexural_relief_profile_km`] is the diagnostic. The loads are the same
/// [`civsim_physics::geodynamics::column_buoyancy_load`] the finite path derives, and the solve is the
/// spectral one on [`civsim_physics::flexural_relief::FlexedPlate`], whose doc carries the mechanism, the
/// named slowly-varying-rigidity approximation, and the measured resolution limit.
///
/// The consequence a caller will notice first: a row of identical columns comes out FLAT TO THE BIT at the
/// isostatic elevation, where the finite path answers a sagging profile that is symmetric but not flat. That
/// sag was never physics, and this is the function that does not produce it.
///
/// Units are [`flexural_relief_profile_km`]'s exactly. The column positions do not enter: on a closed row a
/// column's place is its index, which is also why this path forms no `i * cell_width` product to overflow.
// @derives: the periodic flexural relief profile over a closed column row <- each column's buoyancy load, the plate rigidity and the grid spacing
pub fn periodic_flexural_relief_profile_km(
    thickness_km: &[Fixed],
    crust_density: Fixed,
    mantle_density: Fixed,
    gravity_km_s2: Fixed,
    cell_width_km: Fixed,
    plate: &FlexedPlate,
) -> Result<Vec<ElevationKm>, ProfileRefusal> {
    if thickness_km.is_empty() {
        return Err(ProfileRefusal::EmptyRow);
    }
    if cell_width_km <= Fixed::ZERO {
        return Err(ProfileRefusal::NonPhysicalColumn);
    }
    let half_width = cell_width_km
        .checked_div(Fixed::from_int(2))
        .ok_or(ProfileRefusal::NonPhysicalColumn)?;

    // THE SAME DERIVED LOAD, read for its pressure. `column_buoyancy_load` is the banked derivation of a
    // column's buoyancy anomaly, including the sign that makes light crust push the plate UP, so the pressure
    // is taken from it rather than recomputed here. Its position argument is what places a load in the finite
    // superposition and has no meaning on a closed row, so it is passed as zero and the row's own index
    // carries the geometry.
    let mut cell_pressure = Vec::with_capacity(thickness_km.len());
    for t in thickness_km {
        let load = column_buoyancy_load(
            crust_density,
            mantle_density,
            *t,
            gravity_km_s2,
            Fixed::ZERO,
            half_width,
        )
        .ok_or(ProfileRefusal::NonPhysicalColumn)?;
        cell_pressure.push(load.magnitude);
    }

    plate
        .periodic_elevation_km(&cell_pressure, cell_width_km)
        .map_err(ProfileRefusal::Relief)
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_physics::flexure::flexural_rigidity;

    fn f64_of(x: Fixed) -> f64 {
        x.to_f64_lossy()
    }

    fn f64_of_e(x: ElevationKm) -> f64 {
        x.km().to_f64_lossy()
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
        let peak = f64_of_e(profile[5]);
        for (i, v) in profile.iter().enumerate() {
            if i != 5 {
                assert!(
                    f64_of(v.km()) < peak,
                    "the thick column stands highest: index {i} at {} against {peak}",
                    f64_of(v.km())
                );
            }
        }
        // The lift DECAYS outward from the range on both sides.
        assert!(f64_of_e(profile[4]) > f64_of_e(profile[3]));
        assert!(f64_of_e(profile[3]) > f64_of_e(profile[2]));
        assert!(f64_of_e(profile[6]) > f64_of_e(profile[7]));
        assert!(f64_of_e(profile[7]) > f64_of_e(profile[8]));
        eprintln!(
            "ridge profile (km): {:?}",
            profile.iter().map(|v| f64_of(v.km())).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_uniform_row_is_symmetric_which_is_what_makes_the_lift_a_neighbour_effect() {
        // THE CONTROL, and what it is NOT. This test first asserted a flat interior, from the intuition that
        // identical columns stand at identical heights. That failed, and the second version explained the
        // failure by the strip kernel going negative past `z = 3 pi / 4`. A review corrected BOTH: on a
        // translationally invariant infinite or periodic plate a uniform load has a uniform response,
        // necessarily, so the non-flat result is a BOUNDARY ARTIFACT of truncating the source list while still
        // using infinite-plate Green's functions. It is not a kernel property and not a derived free edge.
        //
        // On a globe the row is periodic and the correct answer IS flat. Recording the artifact as physics is
        // what the second version did, so this asserts only the invariant that survives truncation.
        //
        // The invariant that DOES hold is SYMMETRY. A uniform row is its own mirror image, so column `i` and
        // column `n - 1 - i` must agree to the bit. That is a real property of the superposition rather than a
        // guess about it, and it fails immediately if the load list is built with an off-by-one centre or if the
        // Green's function is evaluated with a signed rather than an absolute distance.
        let profile = flexural_relief_profile_km(
            &[Fixed::from_int(30); 11],
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
                profile[i].km().to_bits(),
                profile[n - 1 - i].km().to_bits(),
                "a uniform row is its own mirror: index {i} against {}",
                n - 1 - i
            );
        }
        // And the ends sit LOWEST, missing the neighbours beyond the row that would have lifted them.
        assert!(
            f64_of_e(profile[0]) < f64_of_e(profile[5]),
            "the truncated end is short of the neighbours a periodic row would give it: {} against {}",
            f64_of_e(profile[0]),
            f64_of_e(profile[5])
        );
    }

    /// A PERIODIC UNIFORM ROW IS FLAT TO THE BIT, which is the requirement the review named and the reason
    /// the closed-row entry point exists.
    ///
    /// The finite sibling above can only assert MIRROR SYMMETRY for the same row, because its truncation
    /// leaves the ends short of the neighbours beyond them and its own test says so. On a closed row that
    /// shortfall has nowhere to come from: identical columns stand at one identical elevation, necessarily,
    /// and here they do to the last bit at every row length.
    #[test]
    fn a_periodic_uniform_row_is_flat_to_the_bit_where_the_finite_one_only_sags_symmetrically() {
        let plate = earthlike_plate();
        for n in [2usize, 5, 11, 32] {
            let row = vec![Fixed::from_int(30); n];
            let profile = periodic_flexural_relief_profile_km(
                &row,
                Fixed::from_ratio(29, 10),
                Fixed::from_ratio(33, 10),
                Fixed::from_ratio(98, 10_000),
                Fixed::from_int(60),
                &plate,
            )
            .expect("the closed row has a profile");
            assert_eq!(profile.len(), n);
            for (i, e) in profile.iter().enumerate() {
                assert_eq!(
                    e.km().to_bits(),
                    profile[0].km().to_bits(),
                    "a uniform closed row is flat to the bit: n = {n}, index {i} at {} against {}",
                    f64_of_e(*e),
                    f64_of_e(profile[0])
                );
            }
            // And it stands UP, since a crust lighter than the mantle is buoyant.
            assert!(
                f64_of_e(profile[0]) > 0.0,
                "a buoyant uniform row floats above the compensation reference, got {}",
                f64_of_e(profile[0])
            );
        }

        // THE CONTRAST, measured rather than asserted: the finite path on the very same row is NOT flat, and
        // that variation is the truncation artifact this function removes.
        let finite = flexural_relief_profile_km(
            &[Fixed::from_int(30); 11],
            Fixed::from_ratio(29, 10),
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
            Fixed::from_int(60),
            &plate,
        )
        .expect("the finite row has a profile");
        assert_ne!(
            finite[0].km().to_bits(),
            finite[5].km().to_bits(),
            "the finite path really does sag, or this comparison is moot"
        );
    }

    #[test]
    fn a_thick_column_still_lifts_its_neighbours_on_a_closed_row() {
        // THE LATERAL TERM SURVIVES THE CHANGE OF DOMAIN. The whole reason this substrate exists is that a
        // range lifts its surroundings and the lift decays with distance, which no per-column flotation law
        // can represent. The periodic solve must keep that, or it has traded one defect for another.
        let profile = periodic_flexural_relief_profile_km(
            &ridge_row(),
            Fixed::from_ratio(29, 10),
            Fixed::from_ratio(33, 10),
            Fixed::from_ratio(98, 10_000),
            Fixed::from_int(60),
            &earthlike_plate(),
        )
        .expect("the closed ridge row has a profile");
        assert_eq!(profile.len(), 11);
        let peak = f64_of_e(profile[5]);
        for (i, v) in profile.iter().enumerate() {
            if i != 5 {
                assert!(
                    f64_of_e(*v) < peak,
                    "the thick column stands highest: index {i} at {} against {peak}",
                    f64_of_e(*v)
                );
            }
        }
        // The lift decays outward on both sides.
        assert!(f64_of_e(profile[4]) > f64_of_e(profile[3]));
        assert!(f64_of_e(profile[6]) > f64_of_e(profile[7]));

        // AND THE ROW IS ITS OWN MIRROR ABOUT THE RIDGE, having no ends to break the symmetry.
        //
        // NOT TO THE BIT, and the difference from the uniform row's flatness is worth stating rather than
        // papering over. A uniform row is flat BY CONSTRUCTION: every deviation is exactly zero, so every
        // spectral bin is exactly zero and nothing is reconstructed. A general symmetric row's symmetry is a
        // mathematical identity ACROSS bins (`cos a cos(a+b) + sin a sin(a+b) = cos b`), which fixed point
        // cannot make exact, so what survives is a residue of the reconstruction's own rounding.
        //
        // The criterion is therefore derived from the profile rather than chosen: the departure must be far
        // smaller than the SMALLEST PHYSICAL VARIATION the profile carries, or it could be mistaken for one.
        // It reads about five orders under it here.
        let smallest_step = (0..10)
            .map(|i| (f64_of_e(profile[i]) - f64_of_e(profile[i + 1])).abs())
            .filter(|d| *d > 0.0)
            .fold(f64::INFINITY, f64::min);
        let worst_asymmetry = (0..11)
            .map(|i| (f64_of_e(profile[(5 + i) % 11]) - f64_of_e(profile[(16 - i) % 11])).abs())
            .fold(0.0_f64, f64::max);
        eprintln!(
            "closed ridge row: worst mirror asymmetry {worst_asymmetry:.3e} km against a smallest physical \
             step of {smallest_step:.3e} km"
        );
        assert!(
            worst_asymmetry < smallest_step * 1e-3,
            "the closed row is its own mirror to well within the relief it resolves: asymmetry \
             {worst_asymmetry:.3e} km against a smallest step of {smallest_step:.3e} km"
        );
        eprintln!(
            "periodic ridge profile (km): {:?}",
            profile.iter().map(|v| f64_of(v.km())).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_periodic_profile_refuses_on_the_same_degenerate_inputs() {
        let plate = earthlike_plate();
        let g = Fixed::from_ratio(98, 10_000);
        assert_eq!(
            periodic_flexural_relief_profile_km(
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
            periodic_flexural_relief_profile_km(
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
            periodic_flexural_relief_profile_km(
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
