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

//! The canonical physics quantity catalogue (design Part 55, R-UNITS-PIN): the bridge that
//! populates the `civsim-units` `QuantityRegistry` from the loaded physics floors, deriving each
//! axis's per-quantity fixed-point scale from its owner-set envelope. Physics is the authored
//! quantity layer (Principle 9); the units crate carries the mechanism and ships no membership, so
//! this is where the two meet. The significance target and the guard headroom are the owner's
//! reserved numbers (R-UNITS-PIN), passed in; nothing here authors a scale.
//!
//! The catalogue is built from axes whose range the owner has set: a reserved (unset) axis has no
//! envelope to derive from and is omitted until its range is set (the electromagnetism axes join on
//! their set). Because the derivation keys off the reserved significance target, which axes keep the
//! canonical thirty-two-bit scale and which derive a finer one is the owner's choice, not a fixed
//! property of the domain: a lower significance target holds more axes at the canonical scale.

use crate::{AxisRange, Dimension, PerClassRange, PhysicsRegistry, QuantityAxis};
use civsim_core::Fixed;
use civsim_units::{
    derive_scale_bits, BaseDimensionRegistry, DerivedScale, Dimension as UnitsDimension,
    OverflowPolicy, QuantityDef, QuantityRegistry,
};

/// The canonical fixed-point scale the substrate stores `Fixed` in: Q32.32's thirty-two fractional
/// bits. A quantity whose envelope fits keeps it, so its `Fixed` and `AbsoluteQuantity`
/// representations coincide and it bridges with no change.
pub const CANONICAL_SCALE: u32 = Fixed::FRAC_BITS;

/// The fixed base-dimension order the units catalogue references by index: length, mass, time,
/// temperature, current (the five bases the floors use, current the wave-3 electricity addition).
const BASE_ORDER: [&str; 5] = ["length", "mass", "time", "temperature", "current"];

/// Register the five physics base dimensions into a units base registry in the canonical order, so a
/// derived dimension is a computed composition over known indices rather than an authored entry.
pub fn base_registry() -> BaseDimensionRegistry {
    let mut base = BaseDimensionRegistry::new();
    for name in BASE_ORDER {
        base.register(name);
    }
    base
}

/// Map a physics dimension to the units base-exponent form over [length, mass, time, temperature,
/// current]. Zero exponents drop out and the result is canonical-sorted, so equal dimensions compare
/// equal regardless of how they were built.
pub fn units_dimension(d: &Dimension) -> UnitsDimension {
    UnitsDimension::from_terms([
        (0u16, d.length),
        (1, d.mass),
        (2, d.time),
        (3, d.temperature),
        (4, d.current),
    ])
}

/// `floor(log2(|value|))` for a non-zero `Fixed`, in the value domain (negative for a value below
/// one), or `None` for zero. Computed from the raw bits at the canonical scale, so it is exact and
/// touches no floating point.
fn log2_floor(v: Fixed) -> Option<i32> {
    let raw = v.to_bits().unsigned_abs();
    if raw == 0 {
        None
    } else {
        Some((63 - raw.leading_zeros()) as i32 - CANONICAL_SCALE as i32)
    }
}

/// The fractional width the decimal-envelope log2 computes in: an i128 fixed-point wide enough that
/// a bound far below the Q32.32 epsilon (a picofarad, ~1e-12 ~ 2^-40) keeps its magnitude instead of
/// underflowing to zero, with headroom for the largest physical bound (~1e9 ~ 2^30) inside i128.
const WIDE_FRAC: u32 = 80;

/// Parse a decimal string to an i128 fixed-point value with `WIDE_FRAC` fractional bits, the same
/// integer arithmetic as `Fixed::from_decimal_str` but wide enough that a sub-Q32.32-epsilon
/// magnitude survives. `None` on a malformed string or an envelope so wide it would overflow i128
/// (far outside any physical range), in which case the caller falls back to the Fixed-rounded bound.
fn decimal_to_wide(s: &str) -> Option<i128> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (neg, body) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int_str, frac_str) = body.split_once('.').unwrap_or((body, ""));
    if frac_str.len() > 30 {
        return None;
    }
    let int_val: i128 = if int_str.is_empty() {
        0
    } else {
        int_str.parse().ok()?
    };
    // The shift by WIDE_FRAC must not overflow i128: a magnitude needing more than 127 - WIDE_FRAC
    // bits is rejected (the caller then reads the Fixed-rounded bound).
    if int_val != 0 && int_val.unsigned_abs().leading_zeros() <= WIDE_FRAC {
        return None;
    }
    let mut bits: i128 = int_val << WIDE_FRAC;
    if !frac_str.is_empty() {
        let digits: i128 = frac_str.parse().ok()?;
        if digits != 0 && digits.unsigned_abs().leading_zeros() <= WIDE_FRAC {
            return None;
        }
        let mut den: i128 = 1;
        for _ in 0..frac_str.len() {
            den = den.checked_mul(10)?;
        }
        bits += (digits << WIDE_FRAC) / den;
    }
    if neg {
        bits = -bits;
    }
    Some(bits)
}

/// `floor(log2(|value|))` of a decimal string, exact and free of floating point, or `None` for a
/// zero or unparseable value. Computed from the wide fixed-point bits, so a bound like `1e-12` that
/// underflows the Q32.32 range reads its true magnitude (`-40`) rather than the zero the stored
/// `Fixed` would give.
fn decimal_log2_floor(s: &str) -> Option<i32> {
    let raw = decimal_to_wide(s)?.unsigned_abs();
    if raw == 0 {
        None
    } else {
        Some(127 - raw.leading_zeros() as i32 - WIDE_FRAC as i32)
    }
}

/// The scale an axis derives from its set envelope, or `None` when the range is still reserved
/// (unset), so a reserved axis is left out of the catalogue until the owner sets it. `sig_target`
/// (the significant bits the low end must retain) and `guard` (integer headroom above the top) are
/// the owner's reserved numbers. The envelope's log2 bounds are read from the declared decimal bounds
/// (retained on the axis), since a bound below the Q32.32 epsilon underflows the stored `Fixed` range
/// to zero; each bound falls back to the Fixed-rounded value only if the decimal is absent or too wide
/// to parse.
pub fn axis_scale(axis: &QuantityAxis, sig_target: u32, guard: u32) -> Option<DerivedScale> {
    let (lo_fx, hi_fx) = match &axis.range {
        AxisRange::Set { lo, hi } => (*lo, *hi),
        AxisRange::Reserved { .. } => return None,
    };
    let (lo_dec, hi_dec) = match &axis.range_decimal {
        Some((lo, hi)) => (Some(lo.as_str()), Some(hi.as_str())),
        None => (None, None),
    };
    // Prefer the declared decimal magnitude per bound; fall back to the Fixed-rounded bound only when
    // the decimal is absent or beyond the wide parser (never for a physical envelope).
    let lo_log = lo_dec
        .and_then(decimal_log2_floor)
        .or_else(|| log2_floor(lo_fx));
    let hi_log = hi_dec
        .and_then(decimal_log2_floor)
        .or_else(|| log2_floor(hi_fx));
    // The largest bound magnitude drives the top; the smallest non-zero one drives the bottom.
    let present: Vec<i32> = [lo_log, hi_log].into_iter().flatten().collect();
    let (hi_log2, lo_log2) = match (present.iter().max(), present.iter().min()) {
        (Some(h), Some(l)) => (*h, *l),
        // A degenerate [0, 0] envelope is a point at the origin: it needs no scale beyond the
        // canonical, so treat it as fitting.
        _ => (0, 0),
    };
    Some(derive_scale_bits(
        hi_log2,
        lo_log2,
        sig_target,
        guard,
        CANONICAL_SCALE,
    ))
}

/// The scale one class of a per-class-scale axis derives from its declared decimal envelope, keyed
/// off the same decimal-envelope log2 the single-scale axes use so a sub-epsilon per-class bound (a
/// nanogram-per-kilogram toxin tolerance) keeps its magnitude. `None` when the class's bounds are not
/// both set: an unset per-class entry is reserved and omitted from the catalogue, the same as a
/// reserved single axis, rather than silently taking the canonical scale (the `""` and `"0"` decimals
/// both read as no-magnitude, so a blank bound must be caught by its emptiness, not its log2).
pub fn per_class_scale(entry: &PerClassRange, sig_target: u32, guard: u32) -> Option<DerivedScale> {
    let (lo, hi) = &entry.bounds;
    if lo.trim().is_empty() || hi.trim().is_empty() {
        return None;
    }
    let present: Vec<i32> = [decimal_log2_floor(lo), decimal_log2_floor(hi)]
        .into_iter()
        .flatten()
        .collect();
    let (hi_log2, lo_log2) = match (present.iter().max(), present.iter().min()) {
        (Some(h), Some(l)) => (*h, *l),
        // A degenerate [0, 0] envelope (both bounds present and zero) is a point at the origin: it
        // needs no scale beyond the canonical, the same as the single-axis path.
        _ => (0, 0),
    };
    Some(derive_scale_bits(
        hi_log2,
        lo_log2,
        sig_target,
        guard,
        CANONICAL_SCALE,
    ))
}

/// Build the canonical physics quantity catalogue: one `QuantityDef` per axis with a set range, its
/// dimension the physics dimension in base-exponent form and its scale derived from the axis
/// envelope. An axis whose scale is reserved per class registers one quantity per class instead
/// (named `<axis>@<class>`), each with its own scale, the class being the quantity granularity
/// (R-UNITS-PIN). Axes with a reserved (unset) range and no per-class breakdown are omitted until
/// their range is set. Iteration is over the registry's id-sorted axes, so the catalogue is
/// deterministic. The overflow policy is the substrate default (saturate), which every physics
/// quantity uses.
pub fn build_catalogue(reg: &PhysicsRegistry, sig_target: u32, guard: u32) -> QuantityRegistry {
    let mut q = QuantityRegistry::new();
    for axis in reg.axes() {
        if !axis.per_class.is_empty() {
            // A per-class-scale axis: one quantity per class whose bounds are set, each on its own
            // derived scale; an unset (reserved) class is omitted like a reserved single axis.
            for entry in &axis.per_class {
                if let Some(derived) = per_class_scale(entry, sig_target, guard) {
                    q.register(QuantityDef {
                        name: format!("{}@{}", axis.id, entry.class),
                        dimension: units_dimension(&axis.dimension),
                        scale_bits: derived.scale_bits,
                        overflow: OverflowPolicy::Saturate,
                    });
                }
            }
        } else if let Some(derived) = axis_scale(axis, sig_target, guard) {
            q.register(QuantityDef {
                name: axis.id.clone(),
                dimension: units_dimension(&axis.dimension),
                scale_bits: derived.scale_bits,
                overflow: OverflowPolicy::Saturate,
            });
        }
    }
    q
}

#[cfg(test)]
mod tests {
    use super::*;
    use civsim_units::AbsoluteQuantity;

    fn data_path(file: &str) -> String {
        format!("{}/data/{}", env!("CARGO_MANIFEST_DIR"), file)
    }

    fn floors() -> PhysicsRegistry {
        let mut reg = PhysicsRegistry::load(data_path("mechanical_floor.toml")).unwrap();
        reg.extend(data_path("fluids_floor.toml")).unwrap();
        reg.extend(data_path("chem_optics_floor.toml")).unwrap();
        reg
    }

    fn all_floors() -> PhysicsRegistry {
        let mut reg = floors();
        reg.extend(data_path("em_floor.toml")).unwrap();
        reg
    }

    /// The fourteen electromagnetism axes, now owner-set (2026-07-03), that this arc brings into the
    /// catalogue. Kept here so the coverage test fails loud if a future axis is added or renamed.
    const EM_AXES: [&str; 14] = [
        "elec.charge",
        "elec.current",
        "elec.potential",
        "elec.emf",
        "elec.resistance",
        "elec.resistivity",
        "elec.capacitance",
        "elec.electric_field",
        "mag.flux_density",
        "mag.flux",
        "mag.permeability",
        "mag.magnetic_moment",
        "mag.inductance",
        "mag.coupling_coefficient",
    ];

    #[test]
    fn every_owner_set_em_axis_derives_a_scale_that_resolves_its_low_end() {
        // The R-UNITS-PIN gate: each ratified electromagnetism range must derive a scale whose ULP
        // (2^-scale_bits) sits at or below the declared low end, so no axis silently loses its bottom
        // to the Q32.32 underflow the decimal-envelope path exists to prevent.
        let reg = all_floors();
        let cat = build_catalogue(&reg, 16, 1);
        for id in EM_AXES {
            let axis = reg.axis(id).unwrap_or_else(|| panic!("{id} present"));
            assert!(axis.range.is_set(), "{id} range is owner-set");
            let derived = axis_scale(axis, 16, 1).unwrap_or_else(|| panic!("{id} derives a scale"));
            if let Some((lo, _)) = &axis.range_decimal {
                if let Some(lo_log2) = decimal_log2_floor(lo) {
                    assert!(
                        derived.scale_bits as i32 + lo_log2 >= 0,
                        "{id}: scale_bits {} cannot resolve a low end at 2^{}",
                        derived.scale_bits,
                        lo_log2
                    );
                }
            }
            // Each set EM axis joins the built catalogue.
            assert!(cat.id_of(id).is_some(), "{id} is in the catalogue");
        }
        // Capacitance is the one whose picofarad low end underflows the Fixed range, so it must use
        // the decimal path and derive a windowed scale rather than the (wrong) canonical one.
        let cap = reg.axis("elec.capacitance").unwrap();
        assert!(
            axis_scale(cap, 16, 1).unwrap().windowed,
            "capacitance derives a windowed scale from its declared decimal envelope"
        );
        // Permeability [1, 250000] fits the canonical scale at a modest target.
        let perm = reg.axis("mag.permeability").unwrap();
        assert_eq!(
            axis_scale(perm, 16, 1).unwrap().scale_bits,
            CANONICAL_SCALE,
            "a fitting EM ratio keeps the canonical scale"
        );
    }

    #[test]
    fn a_physics_dimension_maps_to_the_canonical_base_exponent_form() {
        // Force = length * mass / time^2 over the base order, current exponent zero and dropped.
        let force = Dimension {
            length: 1,
            mass: 1,
            time: -2,
            temperature: 0,
            current: 0,
        };
        assert_eq!(
            units_dimension(&force).terms(),
            &[(0u16, 1i8), (1, 1), (2, -2)]
        );
        // A dimensionless ratio has no terms.
        assert!(units_dimension(&Dimension::default()).is_dimensionless());
    }

    #[test]
    fn the_catalogue_covers_every_set_axis_and_omits_the_reserved_ones() {
        let reg = floors();
        let cat = build_catalogue(&reg, 16, 1);
        // Every axis with a set range is in the catalogue (one quantity each, no per-class axis over
        // this stack), and every reserved axis is omitted for want of an envelope to derive from.
        let set_axes = reg.axes().filter(|a| a.range.is_set()).count();
        assert_eq!(cat.len(), set_axes, "one quantity per set axis");
        for axis in reg.axes().filter(|a| !a.range.is_set()) {
            assert!(
                cat.id_of(&axis.id).is_none(),
                "the reserved axis {} is not in the catalogue",
                axis.id
            );
        }
        // Each registered quantity carries the axis dimension.
        let acidity = cat.id_of("chem.acidity").expect("acidity is set");
        assert!(cat.get(acidity).unwrap().dimension.is_dimensionless());
    }

    #[test]
    fn a_fitting_axis_derives_the_canonical_scale_and_bridges_fixed_unchanged() {
        let reg = floors();
        let cat = build_catalogue(&reg, 16, 1);
        // chem.acidity [0, 14] fits the canonical scale (a small top, a zero low bound).
        let acidity = cat.id_of("chem.acidity").unwrap();
        assert_eq!(
            cat.get(acidity).unwrap().scale_bits,
            CANONICAL_SCALE,
            "a fitting axis keeps the canonical Q32.32 scale"
        );
        // A Fixed value bridges to and from this canonical quantity unchanged (the identity property
        // the bulk of the substrate keeps).
        let ph = Fixed::from_int(7).to_bits();
        let a = AbsoluteQuantity::from_fixed_bits(acidity, ph, Fixed::FRAC_BITS, &cat).unwrap();
        assert_eq!(a.bits, ph, "the canonical bridge is the identity");
        assert_eq!(a.to_fixed_bits(Fixed::FRAC_BITS, &cat), Some(ph));
    }

    #[test]
    fn the_derived_scale_tracks_the_reserved_significance_target() {
        let reg = floors();
        // fluid.dynamic_viscosity [~1e-5, ~1e2] sits at the canonical boundary: at a high
        // significance target its low end forces a finer scale, at a lower one it keeps the
        // canonical, which is exactly why the target is the owner's reserved lever rather than a
        // fixed property of the axis.
        let visc = reg
            .axis("fluid.dynamic_viscosity")
            .expect("viscosity axis present");
        if visc.range.is_set() {
            let fine = axis_scale(visc, 16, 1).unwrap().scale_bits;
            let coarse = axis_scale(visc, 8, 1).unwrap().scale_bits;
            assert!(
                fine >= coarse,
                "a higher significance target never yields a coarser scale"
            );
            assert_eq!(
                coarse, CANONICAL_SCALE,
                "a modest target keeps the canonical scale"
            );
        }
    }

    #[test]
    fn decimal_log2_floor_reads_sub_epsilon_bounds_the_fixed_range_loses() {
        // 1e-12 underflows the Q32.32 range to zero as a Fixed (2^-32 ~ 2.3e-10 is the floor), but
        // the decimal reads its true magnitude, floor(log2(1e-12)) = -40.
        assert_eq!(decimal_log2_floor("0.000000000001"), Some(-40));
        // Bounds at or above the epsilon agree with the plain floor-log2.
        assert_eq!(decimal_log2_floor("1000"), Some(9));
        assert_eq!(decimal_log2_floor("1"), Some(0));
        assert_eq!(decimal_log2_floor("0.00000001"), Some(-27)); // 1e-8
                                                                 // A signed bound reads its magnitude; a zero or empty bound reads nothing.
        assert_eq!(decimal_log2_floor("-100000000"), Some(26)); // 1e8
        assert_eq!(decimal_log2_floor("0"), None);
        assert_eq!(decimal_log2_floor(""), None);
    }

    #[test]
    fn a_sub_epsilon_low_bound_derives_a_windowed_scale_that_holds_it() {
        // A capacitance-like envelope whose low end (1e-12) underflows the Fixed range to zero: the
        // stored Fixed bound is lost, but the declared decimal is retained and the derivation sizes a
        // scale fine enough to resolve the picofarad, windowed because the ~52-order envelope exceeds
        // one canonical scale. This is the sub-epsilon path the EM capacitance axis needs.
        let toml = r#"
[[axis]]
id = "test.capacitance"
measures = "charge stored per volt"
unit = "F"
dimension = "dimensionless"
scale = "F"
tier = 2
range_lo = "0.000000000001"
range_hi = "1000"
real = "test envelope"
"#;
        let reg = PhysicsRegistry::from_toml_str(toml).unwrap();
        let axis = reg.axis("test.capacitance").unwrap();
        // The stored Fixed low bound underflowed to zero, but the declared decimal survived.
        assert_eq!(
            axis.range_decimal.as_ref().map(|(lo, _)| lo.as_str()),
            Some("0.000000000001"),
            "the declared decimal envelope is retained"
        );
        assert!(
            matches!(axis.range, AxisRange::Set { lo, .. } if lo == Fixed::ZERO),
            "the sub-epsilon low bound underflowed the stored Fixed to zero"
        );
        let derived = axis_scale(axis, 16, 1).unwrap();
        assert!(
            derived.windowed,
            "the wide envelope forces a windowed scale"
        );
        assert!(
            derived.scale_bits >= 40,
            "scale_bits = {} must resolve 2^-40 ~ 1e-12",
            derived.scale_bits
        );
    }

    #[test]
    fn a_fitting_axis_derives_the_same_scale_through_the_decimal_path() {
        // The decimal-envelope path is behaviour-preserving for a bound at or above the epsilon:
        // chem.acidity [0, 14] keeps the canonical scale exactly as before, the only change being
        // where the sub-epsilon low end (which acidity does not have) would now read correctly.
        let reg = floors();
        let acidity = reg.axis("chem.acidity").expect("acidity is set");
        assert_eq!(
            axis_scale(acidity, 16, 1).unwrap().scale_bits,
            CANONICAL_SCALE,
            "a fitting axis still keeps the canonical Q32.32 scale under the decimal path"
        );
    }

    #[test]
    fn the_two_ratified_windows_derive_scales_that_fit_their_envelopes() {
        // The 2026-07-03 windows derive correctly through the catalogue, not just as raw ranges: the
        // second moment of area's 1e-12 low end (which underflows the stored Fixed to zero) forces a
        // fine per-quantity scale that resolves it, and the respiratory surface fits the canonical.
        let reg = floors(); // mechanical carries second_moment_of_area
        let smoa = reg.axis("mech.second_moment_of_area").unwrap();
        let d = axis_scale(smoa, 16, 1).unwrap();
        let lo_log2 = decimal_log2_floor("0.000000000001").unwrap();
        assert!(
            d.scale_bits as i32 + lo_log2 >= 0,
            "second moment scale {} must resolve the 1e-12 low end at 2^{}",
            d.scale_bits,
            lo_log2
        );
        // respiratory_surface lives in the biology floor, which the shared fixture does not load.
        let bio = PhysicsRegistry::load(data_path("biology_floor.toml")).unwrap();
        let resp = bio.axis("bio.respiratory_surface").unwrap();
        assert_eq!(
            axis_scale(resp, 16, 1).unwrap().scale_bits,
            CANONICAL_SCALE,
            "the [0, 200] m^2 respiratory surface fits the canonical scale"
        );
    }

    #[test]
    fn a_per_class_axis_registers_one_quantity_per_class_each_on_its_own_scale() {
        // The bio.consumer.reference_tolerance case: a scale reserved per toxin class, whose
        // pg/kg-to-g/kg envelope no single scale spans. The class is the quantity granularity, so the
        // catalogue registers one quantity per class, each deriving its own scale from that class's
        // envelope through the same decimal-envelope path, so a nanogram-per-kilogram class keeps its
        // magnitude while a milligram class fits the canonical scale.
        let toml = r#"
[[axis]]
id = "test.tolerance"
measures = "per-toxin-class reference tolerance"
unit = "mg/kg-body"
dimension = "dimensionless"
scale = "per-class"
tier = 0
range_reserved = "per toxin class (R-UNITS-PIN)"
real = "test"
per_class = [
  { class = "alkaloid", range_lo = "0.001", range_hi = "5000" },
  { class = "cardiac_glycoside", range_lo = "0.000000000001", range_hi = "4000" },
]
"#;
        let reg = PhysicsRegistry::from_toml_str(toml).unwrap();
        let cat = build_catalogue(&reg, 16, 1);
        // Two per-class quantities, and the bare axis id is not registered (the class is the grain).
        let alkaloid = cat
            .id_of("test.tolerance@alkaloid")
            .expect("the alkaloid class is a quantity");
        let cardiac = cat
            .id_of("test.tolerance@cardiac_glycoside")
            .expect("the cardiac-glycoside class is a quantity");
        assert!(
            cat.id_of("test.tolerance").is_none(),
            "the bare per-class axis is not itself a quantity"
        );
        // The milligram class fits the canonical scale; the sub-epsilon class (a picogram low end
        // underflowing Q32.32) derives a finer per-quantity scale, so the two differ.
        let alk_scale = cat.get(alkaloid).unwrap().scale_bits;
        let car_scale = cat.get(cardiac).unwrap().scale_bits;
        assert_eq!(alk_scale, CANONICAL_SCALE, "the mg/kg class fits canonical");
        assert!(
            car_scale > CANONICAL_SCALE,
            "the sub-epsilon class derives a finer scale ({car_scale} > {CANONICAL_SCALE})"
        );
        // Each per-class quantity carries the axis dimension (dimensionless here).
        assert!(cat.get(alkaloid).unwrap().dimension.is_dimensionless());
    }

    #[test]
    fn a_reserved_per_class_entry_is_omitted_and_a_duplicate_class_fails_loud() {
        // An entry with unset bounds is reserved: omitted from the catalogue, not silently taking the
        // canonical scale (the failure mode this arc exists to prevent).
        let toml = r#"
[[axis]]
id = "test.tol"
measures = "per-class tolerance"
unit = "mg/kg-body"
dimension = "dimensionless"
scale = "per-class"
tier = 0
range_reserved = "per class (R-UNITS-PIN)"
real = "test"
per_class = [
  { class = "set", range_lo = "0.001", range_hi = "5000" },
  { class = "reserved" },
]
"#;
        let reg = PhysicsRegistry::from_toml_str(toml).unwrap();
        let cat = build_catalogue(&reg, 16, 1);
        assert!(
            cat.id_of("test.tol@set").is_some(),
            "the set class registers"
        );
        assert!(
            cat.id_of("test.tol@reserved").is_none(),
            "the reserved class is omitted, not registered at the canonical scale"
        );
        // A duplicate class id would collide the quantity name, so it fails to load, not at build.
        let dup = r#"
[[axis]]
id = "test.dup"
measures = "per-class tolerance"
unit = "mg/kg-body"
dimension = "dimensionless"
scale = "per-class"
tier = 0
range_reserved = "per class"
real = "test"
per_class = [
  { class = "x", range_lo = "1", range_hi = "2" },
  { class = "x", range_lo = "3", range_hi = "4" },
]
"#;
        assert!(
            PhysicsRegistry::from_toml_str(dup).is_err(),
            "a duplicate per-class class fails loud at load"
        );
    }

    #[test]
    fn an_at_sign_in_an_axis_or_class_id_fails_loud() {
        // `@` is the per-class quantity separator, so an id carrying it could alias a per-class
        // quantity name and hit the catalogue's duplicate-name panic. Both are rejected at load.
        let bad_axis = r#"
[[axis]]
id = "tox@bar"
dimension = "dimensionless"
scale = "1"
tier = 0
range_lo = "0"
range_hi = "1"
real = "test"
"#;
        assert!(
            PhysicsRegistry::from_toml_str(bad_axis).is_err(),
            "an axis id containing '@' fails loud"
        );
        let bad_class = r#"
[[axis]]
id = "test.tol"
dimension = "dimensionless"
scale = "per-class"
tier = 0
range_reserved = "per class"
real = "test"
per_class = [
  { class = "al@kaloid", range_lo = "0.001", range_hi = "5000" },
]
"#;
        assert!(
            PhysicsRegistry::from_toml_str(bad_class).is_err(),
            "a class id containing '@' fails loud"
        );
    }
}
