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

//! Sparse, deterministic state at the geology-to-surface boundary.
//!
//! [`GeodynamicField`] holds the shared per-column interior state defined by
//! [`GeodynamicColumn`]. [`EarthworkField`] holds the elevation deltas that the
//! surface reads, with physical geology and later surface modification kept as
//! separate sources. Both fields use ordered maps, prune their zero state, and
//! fold an empty map as no bytes. A world that has not armed either lane remains
//! hash-neutral.

use std::collections::BTreeMap;

use civsim_core::{Fixed, StateHasher};

use crate::Coord3;

/// The shared per-column state between the interior and surface geology lanes.
pub use civsim_physics::geodynamics::GeodynamicColumn;

/// The sparse per-column elevation delta from the generated surface baseline.
///
/// Two sources accumulate onto the same effective elevation and stay in
/// separate maps so their causes remain legible: surface modification and the
/// geological delta from seed crust, isostatic relaxation, and interior uplift.
/// The physical surface reads their sum through [`Self::total_delta`]. Both maps
/// are empty by default, so an unarmed field folds no bytes.
#[derive(Clone, Debug, Default)]
pub struct EarthworkField {
    /// The surface-modification elevation delta at each reworked column, keyed
    /// by its ground [`Coord3`] at z zero. An absent column reads zero. Returning
    /// a column to zero prunes it from the canonical walk.
    deltas: BTreeMap<Coord3, Fixed>,
    /// The geological elevation delta at each column. Seed crust, isostatic
    /// relaxation, and interior uplift write this source. An absent column reads
    /// zero, and a zero result is pruned.
    geological: BTreeMap<Coord3, Fixed>,
}

impl EarthworkField {
    /// An empty field: generated terrain everywhere, with neither source armed.
    pub fn new() -> EarthworkField {
        EarthworkField::default()
    }

    /// Whether neither source carries a nonzero elevation delta.
    pub fn is_empty(&self) -> bool {
        self.deltas.is_empty() && self.geological.is_empty()
    }

    /// The surface-modification elevation delta at a column. An absent column
    /// reads zero. [`Self::total_delta`] adds the geological source.
    pub fn delta(&self, column: Coord3) -> Fixed {
        self.deltas.get(&column).copied().unwrap_or(Fixed::ZERO)
    }

    /// The geological elevation delta at a column. An absent column reads zero.
    pub fn geological_delta(&self, column: Coord3) -> Fixed {
        self.geological.get(&column).copied().unwrap_or(Fixed::ZERO)
    }

    /// The effective elevation delta: surface modification plus geology.
    /// Saturating addition keeps an extreme sum representable. With geology
    /// unarmed this remains byte-for-byte equivalent to [`Self::delta`].
    pub fn total_delta(&self, column: Coord3) -> Fixed {
        self.delta(column)
            .saturating_add(self.geological_delta(column))
    }

    /// Accumulate a surface-modification delta at a column. A zero change does
    /// nothing, and returning the accumulated value to zero prunes the column.
    pub fn adjust(&mut self, column: Coord3, change: Fixed) {
        if change == Fixed::ZERO {
            return;
        }
        let entry = self.deltas.entry(column).or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(change);
        if *entry == Fixed::ZERO {
            self.deltas.remove(&column);
        }
    }

    /// Accumulate a geological delta at a column. A zero change does nothing,
    /// and returning the accumulated value to zero prunes the column.
    pub fn adjust_geological(&mut self, column: Coord3, change: Fixed) {
        if change == Fixed::ZERO {
            return;
        }
        let entry = self.geological.entry(column).or_insert(Fixed::ZERO);
        *entry = entry.saturating_add(change);
        if *entry == Fixed::ZERO {
            self.geological.remove(&column);
        }
    }

    /// Fold the two sources in their established order: surface modification,
    /// then geology. Each ordered-map entry is written without a length prefix,
    /// so an empty field folds nothing and insertion order cannot affect bytes.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (column, delta) in &self.deltas {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(*delta);
        }
        for (column, delta) in &self.geological {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(*delta);
        }
    }
}

/// The sparse resident field for the shared [`GeodynamicColumn`] interface.
/// An absent column reads the all-zero default. The field stays empty until a
/// genesis pass arms geology, preserving the opt-in empty-default behavior.
#[derive(Clone, Debug, Default)]
pub struct GeodynamicField {
    columns: BTreeMap<Coord3, GeodynamicColumn>,
}

impl GeodynamicField {
    /// An empty field: no column carries geodynamic state.
    pub fn new() -> GeodynamicField {
        GeodynamicField::default()
    }

    /// Whether no column carries geodynamic state.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// The state at a column, or the all-zero default when it is absent.
    pub fn get(&self, column: Coord3) -> GeodynamicColumn {
        self.columns.get(&column).copied().unwrap_or_default()
    }

    /// Walk populated columns in canonical [`Coord3`] key order.
    pub fn iter(&self) -> impl Iterator<Item = (Coord3, GeodynamicColumn)> + '_ {
        self.columns.iter().map(|(coord, state)| (*coord, *state))
    }

    /// Set one column. An all-zero state is pruned from the canonical walk.
    pub fn set(&mut self, column: Coord3, state: GeodynamicColumn) {
        if state == GeodynamicColumn::default() {
            self.columns.remove(&column);
        } else {
            self.columns.insert(column, state);
        }
    }

    /// Fold columns in canonical key order using the established interface
    /// fields. No length prefix is written, so an empty field folds no bytes.
    pub fn hash_into(&self, h: &mut StateHasher) {
        for (column, state) in &self.columns {
            h.write_i64(column.x as i64);
            h.write_i64(column.y as i64);
            h.write_i64(column.z as i64);
            h.write_fixed(state.crustal_density);
            h.write_fixed(state.crustal_thickness);
            h.write_fixed(state.isostatic_elevation);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earthwork_accumulates_prunes_and_hashes_in_canonical_order() {
        let mut earthwork = EarthworkField::new();
        assert!(earthwork.is_empty());
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);

        assert_eq!(earthwork.delta(a), Fixed::ZERO);
        earthwork.adjust(a, Fixed::from_int(-2));
        earthwork.adjust(a, Fixed::from_int(-1));
        assert_eq!(earthwork.delta(a), Fixed::from_int(-3));
        earthwork.adjust(b, Fixed::from_int(4));
        assert_eq!(earthwork.delta(b), Fixed::from_int(4));
        assert!(!earthwork.is_empty());

        earthwork.adjust(a, Fixed::from_int(3));
        assert_eq!(earthwork.delta(a), Fixed::ZERO);

        let mut first = EarthworkField::new();
        first.adjust(Coord3::ground(0, 0), Fixed::from_int(1));
        first.adjust(Coord3::ground(9, 9), Fixed::from_int(-1));
        let mut second = EarthworkField::new();
        second.adjust(Coord3::ground(9, 9), Fixed::from_int(-1));
        second.adjust(Coord3::ground(0, 0), Fixed::from_int(1));
        assert_eq!(hash_earthwork(&first), hash_earthwork(&second));

        let mut empty_hash = StateHasher::new();
        EarthworkField::new().hash_into(&mut empty_hash);
        assert_eq!(empty_hash.finish(), StateHasher::new().finish());
    }

    #[test]
    fn geological_and_surface_sources_remain_distinct_and_sum_exactly() {
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);

        let mut surface_only = EarthworkField::new();
        surface_only.adjust(a, Fixed::from_int(-3));
        surface_only.adjust(b, Fixed::from_int(4));
        assert_eq!(surface_only.geological_delta(a), Fixed::ZERO);
        assert_eq!(surface_only.total_delta(a), surface_only.delta(a));

        let mut earthwork = EarthworkField::new();
        earthwork.adjust(a, Fixed::from_int(-3));
        earthwork.adjust_geological(a, Fixed::from_int(10));
        earthwork.adjust_geological(a, Fixed::from_int(2));
        assert_eq!(earthwork.geological_delta(a), Fixed::from_int(12));
        assert_eq!(earthwork.delta(a), Fixed::from_int(-3));
        assert_eq!(earthwork.total_delta(a), Fixed::from_int(9));

        let mut first = EarthworkField::new();
        first.adjust_geological(Coord3::ground(0, 0), Fixed::from_int(1));
        first.adjust_geological(Coord3::ground(9, 9), Fixed::from_int(-1));
        assert!(!first.is_empty());
        let mut second = EarthworkField::new();
        second.adjust_geological(Coord3::ground(9, 9), Fixed::from_int(-1));
        second.adjust_geological(Coord3::ground(0, 0), Fixed::from_int(1));
        assert_eq!(hash_earthwork(&first), hash_earthwork(&second));

        let mut pruned = EarthworkField::new();
        pruned.adjust_geological(a, Fixed::from_int(5));
        pruned.adjust_geological(a, Fixed::from_int(-5));
        assert!(pruned.is_empty());
    }

    #[test]
    fn geodynamic_field_carries_prunes_and_hashes_in_canonical_order() {
        let mut field = GeodynamicField::new();
        assert!(field.is_empty());
        let a = Coord3::ground(2, 3);
        let b = Coord3::ground(5, 1);
        assert_eq!(field.get(a), GeodynamicColumn::default());

        let state = GeodynamicColumn {
            crustal_density: Fixed::from_ratio(33, 10),
            crustal_thickness: Fixed::from_int(35_000),
            isostatic_elevation: Fixed::from_int(5),
            ..GeodynamicColumn::default()
        };
        field.set(a, state);
        assert_eq!(field.get(a).crustal_density, Fixed::from_ratio(33, 10));
        assert_eq!(field.get(a).crustal_thickness, Fixed::from_int(35_000));
        assert_eq!(field.get(a).isostatic_elevation, Fixed::from_int(5));
        assert!(!field.is_empty());

        field.set(a, GeodynamicColumn::default());
        assert!(field.is_empty());

        let first_state = GeodynamicColumn {
            crustal_density: Fixed::from_int(3),
            ..GeodynamicColumn::default()
        };
        let second_state = GeodynamicColumn {
            isostatic_elevation: Fixed::from_int(-1),
            ..GeodynamicColumn::default()
        };
        let mut first = GeodynamicField::new();
        first.set(a, first_state);
        first.set(b, second_state);
        let mut second = GeodynamicField::new();
        second.set(b, second_state);
        second.set(a, first_state);
        assert_eq!(hash_geodynamics(&first), hash_geodynamics(&second));

        let mut empty_hash = StateHasher::new();
        GeodynamicField::new().hash_into(&mut empty_hash);
        assert_eq!(empty_hash.finish(), StateHasher::new().finish());
    }

    fn hash_earthwork(field: &EarthworkField) -> u128 {
        let mut hasher = StateHasher::new();
        field.hash_into(&mut hasher);
        hasher.finish()
    }

    fn hash_geodynamics(field: &GeodynamicField) -> u128 {
        let mut hasher = StateHasher::new();
        field.hash_into(&mut hasher);
        hasher.finish()
    }
}
