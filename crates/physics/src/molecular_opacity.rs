//! Low-temperature MOLECULAR Rosseland opacity as a REGIME HANDOFF, the occupant of the cold gas gap the
//! ionized-gas closure exposes.
//!
//! When electron scattering is restated as `sigma_T n_e/rho` (linear in the free-electron density), the grey
//! positive floor disappears, and in the `~1500 to 2500 K` window (grains sublimated, H- not yet risen, es and
//! free-free Saha-killed) the ionized-gas closure [`crate::opacity::total_gas_rosseland_opacity`] has NO opacity and
//! returns `None`. The physical occupant of that window is MOLECULAR gas opacity: the vibration-rotation bands of
//! water, CO, TiO, and their kin. This module supplies it as a TOTAL, a regime handoff, NOT an additive term:
//! molecular tables already fold in the low-temperature gas continuum, so summing them with es, ff, or H- would
//! double-count. The handoff SELECTS the molecular total below the gas regime and the gas total above it, blending
//! across the narrow overlap where both are valid (and, by the convergence check, agree).
//!
//! THE COORDINATE (definition tag): low-temperature Rosseland tables are indexed by `(log10 T, log10 R)` with the
//! density proxy `R = rho / (T / 10^6 K)^3` (cgs, `rho` in g/cm^3). `R` is NOT the density and NOT the pressure: it
//! is the combination that makes the opacity a slowly varying function across the disk's density-temperature run, so
//! a table on `(log T, log R)` stays smooth and interpolable. Reading `R` as `rho`, or omitting the `T^3` fold, is
//! the definition-mismatch class; the round-trip test pins the convention.
//!
//! ADMIT THE ALIEN: a grid is computed for a specific hydrogen mass fraction `X` and metallicity `Z` (the molecular
//! band strengths depend on the C, N, O, Ti abundances). A different composition is a DIFFERENT grid, a data row,
//! never a rewrite. The solar-scaled `(X, Z)` grid is one member of the family the loader holds.
//!
//! THE DATA (surfaced, not fabricated): the grid VALUES are the [M] tier, the Ferguson et al. 2005 (ApJ 623, 585)
//! low-temperature Rosseland opacity tables (machine-readable grids over `log T` in `~2.7 to 4.5` and `log R` in
//! `~-8 to 1`). This module builds the MACHINERY (the coordinate, the deterministic bilinear interpolation, the
//! handoff selector) and holds the grid as caller data; the bulk grid is a proof-of-fetch [M] dependency, cited to
//! its primary source per value, that stays reserved until fetched. The convergence acceptance row (that the gas
//! closure and a solar-composition molecular grid agree across `3000 to 4000 K`) is registered against the fetched
//! grid.

use civsim_core::Fixed;

/// The base-ten log of `x`, `log10 x = ln x / ln 10`, in the log domain the whole disk-opacity assembly uses.
fn log10(x: Fixed) -> Option<Fixed> {
    if x <= Fixed::ZERO {
        return None;
    }
    x.ln().checked_div(Fixed::from_int(10).ln())
}

/// `10^y = exp(y ln 10)`, the inverse of [`log10`], for lifting an interpolated `log10 kappa` back to `kappa`.
fn exp10(y: Fixed) -> Option<Fixed> {
    Some(y.checked_mul(Fixed::from_int(10).ln())?.exp())
}

/// The low-temperature opacity coordinate `log10 R` with `R = rho / (T/10^6)^3` (cgs), the density proxy the
/// Ferguson-style tables are indexed on. `log10 R = log10 rho - 3 (log10 T - 6)`. Takes the density as its natural
/// log because a cold disk's `rho ~ 1e-11 g/cm^3` underflows `Fixed`, so the density is carried in the log domain
/// throughout the assembly. `None` if the temperature is non-positive or a term leaves the representable range.
pub fn low_temperature_opacity_log_r(
    ln_density_g_cm3: Fixed,
    temperature_k: Fixed,
) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10_rho = ln_density_g_cm3.checked_div(ln10)?;
    let log10_t = log10(temperature_k)?;
    let t_term = Fixed::from_int(3).checked_mul(log10_t.checked_sub(Fixed::from_int(6))?)?;
    log10_rho.checked_sub(t_term)
}

/// The inverse of [`low_temperature_opacity_log_r`]: recover `ln rho` from `log10 R` and the temperature, so the
/// coordinate is an invertible change of variables (the round-trip test). `ln rho = ln10 (log10 R + 3 (log10 T -
/// 6))`. `None` if the temperature is non-positive or a term leaves the representable range.
pub fn ln_density_from_log_r(log_r: Fixed, temperature_k: Fixed) -> Option<Fixed> {
    if temperature_k <= Fixed::ZERO {
        return None;
    }
    let ln10 = Fixed::from_int(10).ln();
    let log10_t = log10(temperature_k)?;
    let t_term = Fixed::from_int(3).checked_mul(log10_t.checked_sub(Fixed::from_int(6))?)?;
    log_r.checked_add(t_term)?.checked_mul(ln10)
}

/// Linear interpolation `a + f (b - a)`, the one-dimensional kernel the bilinear table interpolation composes.
fn lerp(a: Fixed, b: Fixed, f: Fixed) -> Option<Fixed> {
    a.checked_add(f.checked_mul(b.checked_sub(a)?)?)
}

/// Locate the query in a sorted ascending axis: return `(i0, i1, frac)` with `axis[i0] <= q <= axis[i1]` and `frac`
/// the fractional position in `[0, 1]` between them, CLAMPED to the grid edges (a query past either end reads the
/// edge value, no extrapolation). A single-point axis returns `(0, 0, 0)`. `None` only if the axis is empty.
fn bracket(axis: &[Fixed], q: Fixed) -> Option<(usize, usize, Fixed)> {
    if axis.is_empty() {
        return None;
    }
    if axis.len() == 1 {
        return Some((0, 0, Fixed::ZERO));
    }
    let last = axis.len() - 1;
    if q <= axis[0] {
        return Some((0, 1, Fixed::ZERO));
    }
    if q >= axis[last] {
        return Some((last - 1, last, Fixed::ONE));
    }
    let mut i = 0;
    while i + 1 < axis.len() && axis[i + 1] <= q {
        i += 1;
    }
    let lo = axis[i];
    let hi = axis[i + 1];
    let frac = q.checked_sub(lo)?.checked_div(hi.checked_sub(lo)?)?;
    Some((i, i + 1, frac))
}

/// A low-temperature Rosseland opacity grid for ONE composition, indexed on `(log10 T, log10 R)` and storing
/// `log10 kappa` (cm^2/g). The membership is data (the Ferguson-style [M] grid), the interpolation is fixed Rust.
/// The composition tags `(X, Z)` are the admit-the-alien key: a different composition is a different grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LowTempRosselandGrid {
    /// The hydrogen mass fraction `X` the grid was computed for (composition key, admit-the-alien).
    pub hydrogen_mass_fraction: Fixed,
    /// The metallicity `Z` the grid was computed for (composition key, admit-the-alien).
    pub metallicity: Fixed,
    /// The ascending `log10 T` axis (K).
    pub log_t: Vec<Fixed>,
    /// The ascending `log10 R` axis (`R = rho/(T/10^6)^3`, cgs).
    pub log_r: Vec<Fixed>,
    /// `log10 kappa` (cm^2/g), row-major `[i over log_t][j over log_r]`.
    pub log_kappa: Vec<Vec<Fixed>>,
}

impl LowTempRosselandGrid {
    /// The molecular Rosseland opacity `kappa` (cm^2/g) at `(log10 T, log10 R)`, by deterministic bilinear
    /// interpolation of the stored `log10 kappa` (interpolating in log-log, the standard for opacity tables, then
    /// `10^`). Queries outside the grid clamp to the edge (the caller's regime selector decides validity, so the
    /// table never extrapolates). `None` if the grid is empty or a term leaves the representable range.
    pub fn rosseland_opacity(&self, log_t: Fixed, log_r: Fixed) -> Option<Fixed> {
        let (i0, i1, ft) = bracket(&self.log_t, log_t)?;
        let (j0, j1, fr) = bracket(&self.log_r, log_r)?;
        let k00 = *self.log_kappa.get(i0)?.get(j0)?;
        let k01 = *self.log_kappa.get(i0)?.get(j1)?;
        let k10 = *self.log_kappa.get(i1)?.get(j0)?;
        let k11 = *self.log_kappa.get(i1)?.get(j1)?;
        let a = lerp(k00, k01, fr)?;
        let b = lerp(k10, k11, fr)?;
        let log_kappa = lerp(a, b, ft)?;
        exp10(log_kappa)
    }

    /// The molecular opacity at physical `(rho, T)`: the coordinate fold plus the interpolation. Takes `ln rho`
    /// (the density is carried in the log domain). `None` on the same conditions as [`Self::rosseland_opacity`].
    pub fn opacity_at(&self, ln_density_g_cm3: Fixed, temperature_k: Fixed) -> Option<Fixed> {
        let log_t = log10(temperature_k)?;
        let log_r = low_temperature_opacity_log_r(ln_density_g_cm3, temperature_k)?;
        self.rosseland_opacity(log_t, log_r)
    }
}

/// The GAS-to-MOLECULAR regime handoff `kappa_R` (cm^2/g): a TOTAL, never a sum. Below the gas regime the ionized-
/// gas closure returns `None` (the cold molecular gap) and the molecular total owns the opacity; above it (hotter
/// than the molecular grid's ceiling, or where the grid is unavailable) the gas total owns it; across the narrow
/// overlap where BOTH are valid, the two are blended in log space (a geometric interpolation weighted by
/// temperature), which is a SELECTION between regimes, not an addition, so the low-temperature gas continuum the
/// molecular table already carries is never double-counted with es, ff, or H-.
///
/// The overlap bounds `overlap_lo_k` and `overlap_hi_k` are CALLER data, not authored here: they are the reserved
/// calibration window where the two regimes co-exist, basis the temperature range over which H- rises through the
/// molecular bands (the `3000 to 4000 K` overlap the convergence row checks), and because the convergence row
/// proves the two agree there, the exact window smooths the handoff rather than setting physics. Below `lo` the
/// result is pure molecular, above `hi` pure gas. `None` if neither regime is valid, or a blend term leaves the
/// representable range.
pub fn gas_molecular_handoff_opacity(
    temperature_k: Fixed,
    overlap_lo_k: Fixed,
    overlap_hi_k: Fixed,
    gas_opacity: Option<Fixed>,
    molecular_opacity: Option<Fixed>,
) -> Option<Fixed> {
    match (gas_opacity, molecular_opacity) {
        (None, Some(m)) => Some(m),
        (Some(g), None) => Some(g),
        (None, None) => None,
        (Some(g), Some(m)) => {
            if temperature_k <= overlap_lo_k {
                return Some(m);
            }
            if temperature_k >= overlap_hi_k {
                return Some(g);
            }
            let w = temperature_k
                .checked_sub(overlap_lo_k)?
                .checked_div(overlap_hi_k.checked_sub(overlap_lo_k)?)?;
            let log_m = log10(m)?;
            let log_g = log10(g)?;
            exp10(lerp(log_m, log_g, w)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn the_log_r_coordinate_round_trips() {
        // The definition-tag round-trip: log10 R = log10 rho - 3 (log10 T - 6) is an invertible change of variables,
        // so recovering rho from (log R, T) returns the input. rho = 1e-8 g/cm^3, T = 2000 K gives
        // R = 1e-8/(2e-3)^3 = 1.25, log R = 0.097; the inverse recovers ln(1e-8) = -18.42.
        let ln_rho = crate::saha::ln_of_decimal("1e-8").unwrap();
        let temp = Fixed::from_int(2000);
        let log_r = low_temperature_opacity_log_r(ln_rho, temp).unwrap();
        assert!(
            close(log_r.to_f64_lossy(), 0.0969, 0.01),
            "log R for rho=1e-8, T=2000 is ~0.097, got {}",
            log_r.to_f64_lossy()
        );
        let recovered = ln_density_from_log_r(log_r, temp).unwrap();
        assert!(
            close(recovered.to_f64_lossy(), ln_rho.to_f64_lossy(), 0.02),
            "the coordinate round-trips: ln rho {} recovered as {}",
            ln_rho.to_f64_lossy(),
            recovered.to_f64_lossy()
        );
    }

    fn planar_grid() -> LowTempRosselandGrid {
        // A synthetic grid with a PLANAR log kappa = 0.5(log T - 3.5) - 0.5(log R + 2), for which bilinear
        // interpolation is exact (a plane is a degenerate bilinear surface). Verifies the interpolation KERNEL
        // independent of any physics values.
        let log_t = vec![
            Fixed::from_int(3),
            Fixed::from_ratio(7, 2),
            Fixed::from_int(4),
        ];
        let log_r = vec![
            Fixed::from_int(-3),
            Fixed::from_int(-2),
            Fixed::from_int(-1),
        ];
        let log_kappa = vec![
            vec![
                Fixed::from_ratio(1, 4),
                Fixed::from_ratio(-1, 4),
                Fixed::from_ratio(-3, 4),
            ],
            vec![
                Fixed::from_ratio(1, 2),
                Fixed::ZERO,
                Fixed::from_ratio(-1, 2),
            ],
            vec![
                Fixed::from_ratio(3, 4),
                Fixed::from_ratio(1, 4),
                Fixed::from_ratio(-1, 4),
            ],
        ];
        LowTempRosselandGrid {
            hydrogen_mass_fraction: Fixed::from_ratio(7, 10),
            metallicity: Fixed::from_ratio(2, 100),
            log_t,
            log_r,
            log_kappa,
        }
    }

    #[test]
    fn the_bilinear_interpolation_is_exact_on_a_planar_grid() {
        // At (log T = 3.25, log R = -2.5) the planar log kappa is 0.5(-0.25) - 0.5(-0.5) = 0.125, so kappa = 10^0.125
        // = 1.3335; the bilinear interpolation recovers it exactly (the plane is reproduced), and replays byte for
        // byte.
        let grid = planar_grid();
        let k = grid
            .rosseland_opacity(Fixed::from_ratio(13, 4), Fixed::from_ratio(-5, 2))
            .unwrap();
        assert!(
            close(k.to_f64_lossy(), 1.3335, 1e-3),
            "planar bilinear interpolation is exact: expected 1.3335, got {}",
            k.to_f64_lossy()
        );
        assert_eq!(
            k,
            grid.rosseland_opacity(Fixed::from_ratio(13, 4), Fixed::from_ratio(-5, 2))
                .unwrap(),
            "the interpolation replays byte for byte"
        );
    }

    #[test]
    fn the_bilinear_interpolation_clamps_at_the_grid_edges() {
        // A query past the grid corner reads the corner value (no extrapolation): the top-right corner is
        // log kappa = -0.25, kappa = 10^-0.25 = 0.562.
        let grid = planar_grid();
        let corner = grid
            .rosseland_opacity(Fixed::from_int(9), Fixed::from_int(9))
            .unwrap();
        assert!(
            close(corner.to_f64_lossy(), 0.5623, 1e-3),
            "a query past the grid clamps to the corner (10^-0.25 = 0.562), got {}",
            corner.to_f64_lossy()
        );
    }

    #[test]
    fn the_handoff_selects_molecular_in_the_cold_gap_and_gas_when_hot() {
        // The regime handoff is a TOTAL, not a sum. In the cold gap the gas closure is None (es/ff Saha-killed, H-
        // asleep), so the molecular total owns the opacity; hot, the gas total owns it; with neither valid the
        // handoff is None.
        let m = Fixed::from_ratio(1, 100); // 0.01 cm^2/g molecular
        let g = Fixed::from_int(2); // 2 cm^2/g gas
        let (lo, hi) = (Fixed::from_int(3000), Fixed::from_int(4000));
        assert_eq!(
            gas_molecular_handoff_opacity(Fixed::from_int(1800), lo, hi, None, Some(m)),
            Some(m),
            "the cold gap hands off to the molecular total"
        );
        assert_eq!(
            gas_molecular_handoff_opacity(Fixed::from_int(6000), lo, hi, Some(g), None),
            Some(g),
            "the hot regime is the gas total"
        );
        assert_eq!(
            gas_molecular_handoff_opacity(Fixed::from_int(1800), lo, hi, None, None),
            None,
            "with neither regime valid the handoff is None"
        );
    }

    #[test]
    fn the_handoff_blend_is_geometric_and_never_additive() {
        // Across the overlap the two totals blend in log space (a SELECTION, not an addition). At the midpoint of
        // [3000, 4000] with molecular = 1 and gas = 100, the blend is the geometric mean 10^(0.5*0 + 0.5*2) = 10,
        // NOT the sum 101: the low-temperature gas continuum the molecular table carries is never double-counted.
        let (lo, hi) = (Fixed::from_int(3000), Fixed::from_int(4000));
        let blended = gas_molecular_handoff_opacity(
            Fixed::from_int(3500),
            lo,
            hi,
            Some(Fixed::from_int(100)),
            Some(Fixed::ONE),
        )
        .unwrap();
        assert!(
            close(blended.to_f64_lossy(), 10.0, 0.1),
            "the overlap blend is the geometric mean (10), not the sum (101), got {}",
            blended.to_f64_lossy()
        );
        // Below the window it is pure molecular, above it pure gas.
        assert_eq!(
            gas_molecular_handoff_opacity(lo, lo, hi, Some(Fixed::from_int(100)), Some(Fixed::ONE)),
            Some(Fixed::ONE),
            "at the low edge the handoff is pure molecular"
        );
        assert_eq!(
            gas_molecular_handoff_opacity(hi, lo, hi, Some(Fixed::from_int(100)), Some(Fixed::ONE)),
            Some(Fixed::from_int(100)),
            "at the high edge the handoff is pure gas"
        );
    }
}
