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

//! The terrain and biome substrate (design Part 12, Part 16).
//!
//! A biome is a named terrain class recognised when a cell's elevation, moisture, and
//! temperature fall in its data-defined ranges, the same recognised-not-enumerated
//! pattern the value, semantic, and institution substrates use (Principle 8, Principle
//! 11): the mechanism (classify by matching ranges in order) is fixed, the membership
//! (which biomes exist and where they sit) is data. A starting set is supplied by
//! [`BiomeSet::dev_default`] as a clearly labelled development fixture; a TOML loader is
//! the next increment. The authoritative biome data and its thresholds are owner and data
//! choices for calibration, not values invented here.

use civsim_core::Fixed;

/// A biome identifier. The membership is data, so this is an index into a [`BiomeSet`],
/// never a closed enum.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BiomeId(pub u16);

/// A closed `[lo, hi)` band over a normalised `[0, ONE)` field. A field `v` is in band
/// when `lo <= v < hi`.
pub type Band = (Fixed, Fixed);

/// A biome: a named terrain class with the field ranges that recognise it and the glyph
/// that renders it.
#[derive(Clone, Debug)]
pub struct BiomeDef {
    pub id: BiomeId,
    pub name: String,
    pub glyph: char,
    pub elevation: Band,
    pub moisture: Band,
    pub temperature: Band,
}

impl BiomeDef {
    fn matches(&self, elev: Fixed, moist: Fixed, temp: Fixed) -> bool {
        in_band(elev, self.elevation)
            && in_band(moist, self.moisture)
            && in_band(temp, self.temperature)
    }
}

#[inline]
fn in_band(v: Fixed, (lo, hi): Band) -> bool {
    lo <= v && v < hi
}

/// An ordered set of biomes plus a fallback. Classification is first match in declaration
/// order, so priority is explicit: ocean and mountain are recognised before the lowland
/// biomes, and the fallback catches any cell no band claims.
#[derive(Clone, Debug)]
pub struct BiomeSet {
    biomes: Vec<BiomeDef>,
    fallback: BiomeId,
}

impl BiomeSet {
    /// A biome set from an ordered list and a fallback id.
    pub fn new(biomes: Vec<BiomeDef>, fallback: BiomeId) -> Self {
        BiomeSet { biomes, fallback }
    }

    /// The biome a cell's fields fall into, the first matching band in order, or the
    /// fallback if none matches.
    pub fn classify(&self, elev: Fixed, moist: Fixed, temp: Fixed) -> BiomeId {
        for b in &self.biomes {
            if b.matches(elev, moist, temp) {
                return b.id;
            }
        }
        self.fallback
    }

    /// The glyph for a biome, or `?` if the id is unknown.
    pub fn glyph(&self, id: BiomeId) -> char {
        self.biomes
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.glyph)
            .unwrap_or('?')
    }

    /// The name of a biome, or `"?"` if the id is unknown.
    pub fn name(&self, id: BiomeId) -> &str {
        self.biomes
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.name.as_str())
            .unwrap_or("?")
    }

    /// The number of biomes in the set.
    pub fn len(&self) -> usize {
        self.biomes.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.biomes.is_empty()
    }

    /// A clearly labelled DEVELOPMENT FIXTURE biome set, so the first map slice has
    /// something recognisable to generate and view. The bands are scaffolding, not
    /// owner-reserved values; the authoritative biome data and thresholds are owner and
    /// data choices for calibration. Ordered by priority: water and high ground first,
    /// then the lowland biomes by moisture and temperature, with grassland as the catch.
    pub fn dev_default() -> BiomeSet {
        // Percentage helper over the normalised [0, ONE) field range.
        let p = |n: i64| Fixed::from_ratio(n, 100);
        // "Any value in range" sentinel: a band wider than the field can reach.
        let any: Band = (Fixed::ZERO, Fixed::from_int(2));
        let mut id = 0u16;
        let mut def =
            |name: &str, glyph: char, elevation: Band, moisture: Band, temperature: Band| {
                let d = BiomeDef {
                    id: BiomeId(id),
                    name: name.to_string(),
                    glyph,
                    elevation,
                    moisture,
                    temperature,
                };
                id += 1;
                d
            };
        let biomes = vec![
            def("ocean", '~', (p(0), p(40)), any, any),
            def("coast", '.', (p(40), p(45)), any, any),
            def(
                "snowcap",
                '*',
                (p(78), Fixed::from_int(2)),
                any,
                (p(0), p(35)),
            ),
            def("mountain", '^', (p(75), Fixed::from_int(2)), any, any),
            def("tundra", ',', (p(45), p(75)), any, (p(0), p(35))),
            def(
                "desert",
                ':',
                (p(45), p(75)),
                (p(0), p(30)),
                (p(35), Fixed::from_int(2)),
            ),
            def(
                "forest",
                '#',
                (p(45), p(75)),
                (p(55), Fixed::from_int(2)),
                (p(35), Fixed::from_int(2)),
            ),
            def("grassland", '"', (p(45), p(75)), any, any),
        ];
        // The grassland catch is the last entry; its id is the fallback.
        let fallback = BiomeId(id - 1);
        BiomeSet::new(biomes, fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: i64) -> Fixed {
        Fixed::from_ratio(n, 100)
    }

    #[test]
    fn classify_picks_by_priority() {
        let set = BiomeSet::dev_default();
        let glyph = |e, m, t| set.glyph(set.classify(p(e), p(m), p(t)));
        assert_eq!(glyph(10, 50, 50), '~', "low elevation is ocean");
        assert_eq!(glyph(90, 50, 50), '^', "high warm ground is mountain");
        assert_eq!(glyph(90, 50, 10), '*', "high cold ground is snowcap");
        assert_eq!(glyph(60, 10, 80), ':', "warm dry lowland is desert");
        assert_eq!(glyph(60, 80, 60), '#', "wet warm lowland is forest");
        assert_eq!(glyph(60, 50, 10), ',', "cold lowland is tundra");
        assert_eq!(
            glyph(60, 50, 60),
            '"',
            "the rest of the lowland is grassland"
        );
    }

    #[test]
    fn unknown_id_is_marked() {
        let set = BiomeSet::dev_default();
        assert_eq!(set.glyph(BiomeId(999)), '?');
        assert_eq!(set.name(BiomeId(999)), "?");
    }

    #[test]
    fn the_dev_set_is_populated() {
        let set = BiomeSet::dev_default();
        assert_eq!(set.len(), 8);
        assert!(!set.is_empty());
    }
}
