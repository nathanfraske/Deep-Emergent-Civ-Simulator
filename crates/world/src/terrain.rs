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

//! Generic relief classification over derived elevation references.
//!
//! This module retains no authored terrain catalog. Both classification
//! references arrive from the physical state produced for the body.

use civsim_core::Fixed;

/// A relief class derived by crossing the supplied physical references.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TerrainRelief {
    /// Below the sea-level reference.
    Submarine,
    /// At or above sea level and below the relief datum.
    Lowland,
    /// At or above the relief datum.
    Upland,
}

/// Derive the relief datum as the mean of a nonempty elevation field.
///
/// Returns `None` for an empty field or when the fixed-point sum cannot be
/// represented.
pub fn relief_datum(elevations: &[Fixed]) -> Option<Fixed> {
    if elevations.is_empty() {
        return None;
    }
    let mut sum = Fixed::ZERO;
    for elevation in elevations {
        sum = sum.checked_add(*elevation)?;
    }
    sum.checked_div(Fixed::from_int(elevations.len() as i32))
}

/// Classify an elevation by crossing the supplied sea level and relief datum.
pub fn classify_relief(elevation: Fixed, sea_level: Fixed, relief_datum: Fixed) -> TerrainRelief {
    if elevation < sea_level {
        TerrainRelief::Submarine
    } else if elevation < relief_datum {
        TerrainRelief::Lowland
    } else {
        TerrainRelief::Upland
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relief_datum_derives_from_the_field_mean() {
        let datum =
            relief_datum(&[Fixed::from_int(1), Fixed::from_int(2), Fixed::from_int(3)]).unwrap();
        assert_eq!(datum, Fixed::from_int(2));
        assert!(relief_datum(&[]).is_none());
    }

    #[test]
    fn relief_crosses_the_supplied_references() {
        let sea_level = Fixed::from_int(10);
        let datum = Fixed::from_int(20);
        assert_eq!(
            classify_relief(Fixed::from_int(5), sea_level, datum),
            TerrainRelief::Submarine
        );
        assert_eq!(
            classify_relief(Fixed::from_int(15), sea_level, datum),
            TerrainRelief::Lowland
        );
        assert_eq!(
            classify_relief(Fixed::from_int(25), sea_level, datum),
            TerrainRelief::Upland
        );
        assert_eq!(
            classify_relief(Fixed::from_int(15), Fixed::from_int(18), datum),
            TerrainRelief::Submarine
        );
        assert_eq!(
            classify_relief(Fixed::from_int(15), sea_level, Fixed::from_int(12)),
            TerrainRelief::Upland
        );
    }
}
