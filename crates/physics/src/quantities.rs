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

use crate::{AxisRange, Dimension, PhysicsRegistry, QuantityAxis};
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

/// The scale an axis derives from its set envelope, or `None` when the range is still reserved
/// (unset), so a reserved axis is left out of the catalogue until the owner sets it. `sig_target`
/// (the significant bits the low end must retain) and `guard` (integer headroom above the top) are
/// the owner's reserved numbers.
pub fn axis_scale(axis: &QuantityAxis, sig_target: u32, guard: u32) -> Option<DerivedScale> {
    let (lo, hi) = match &axis.range {
        AxisRange::Set { lo, hi } => (*lo, *hi),
        AxisRange::Reserved { .. } => return None,
    };
    // The largest bound magnitude drives the top; the smallest non-zero one drives the bottom.
    let present: Vec<i32> = [log2_floor(lo), log2_floor(hi)]
        .into_iter()
        .flatten()
        .collect();
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

/// Build the canonical physics quantity catalogue: one `QuantityDef` per axis with a set range, its
/// dimension the physics dimension in base-exponent form and its scale derived from the axis
/// envelope. Axes with a reserved (unset) range are omitted until their range is set. Iteration is
/// over the registry's id-sorted axes, so the catalogue is deterministic. The overflow policy is the
/// substrate default (saturate), which every physics quantity uses.
pub fn build_catalogue(reg: &PhysicsRegistry, sig_target: u32, guard: u32) -> QuantityRegistry {
    let mut q = QuantityRegistry::new();
    for axis in reg.axes() {
        if let Some(derived) = axis_scale(axis, sig_target, guard) {
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
    fn the_catalogue_covers_every_set_axis_and_omits_the_reserved_one() {
        let reg = floors();
        let cat = build_catalogue(&reg, 16, 1);
        // Every axis with a set range is in the catalogue; the one reserved axis over this stack
        // (mech.second_moment_of_area) has no envelope and is omitted.
        let set_axes = reg.axes().filter(|a| a.range.is_set()).count();
        assert_eq!(cat.len(), set_axes, "one quantity per set axis");
        assert!(
            !reg.axis("mech.second_moment_of_area")
                .unwrap()
                .range
                .is_set()
                && cat.id_of("mech.second_moment_of_area").is_none(),
            "the reserved axis is not in the catalogue"
        );
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
}
