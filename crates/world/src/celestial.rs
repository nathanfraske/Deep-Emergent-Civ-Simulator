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

//! The celestial substrate: a world's orbital elements, the source of its year and day
//! lengths in canonical world-seconds (design Part 14.6, Part 32, Parts 20 and 54).
//!
//! A world's year (one orbit) and day (one rotation) are physical properties of that world,
//! not Earth constants. The two periods live here as fixed-point world-seconds so the
//! canonical time cadences (the life-cadence beat of Part 20, the drift cadence of Part 33)
//! derive from the world's own orbit rather than a hardcoded 31,536,000. Physics may be an
//! authored cultural input (Principle 9): the owner sets these two scalars per world, and
//! Earth's values appear only as a labelled development fixture, one option among many, never
//! a silent default. The fields are [`Fixed`], so no float enters canonical state (Principle 3),
//! and the two values fold into a world's state hash at a pinned, documented position.
//!
//! This module is pure data with no calibration dependency: the manifest reader that fills
//! these fields from the two reserved owner scalars lives in the simulation crate, since the
//! manifest lives there and the spatial crate does not depend on it.

use civsim_core::Fixed;

/// A world's orbital elements: the length of its year and its day in canonical world-seconds.
/// The two periods are the physical inputs the canonical time cadences derive from, so a fast
/// world and a slow world beat aging, drift, and the calendar on their own orbits rather than
/// on a shared hardcoded year. Both fields are [`Fixed`], keeping the whole derivation
/// float-free and deterministic (Principle 3).
// @derives: a world's year and day (the time cadences: aging, drift, the calendar) <- the world's own orbit in canonical world-seconds; NEVER a hardcoded 365-day year (dev_earth is a labelled fixture, not the default). North-star: derive orbital_period_seconds from Kepler's third law over (semi-major axis, star mass) rather than carrying it as a scalar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OrbitalElements {
    /// World-seconds per orbit (the length of a year).
    pub orbital_period_seconds: Fixed,
    /// World-seconds per rotation (the length of a day).
    pub rotation_period_seconds: Fixed,
}

impl OrbitalElements {
    /// A labelled DEVELOPMENT FIXTURE, not owner values: Earth's year (31,536,000 world-seconds,
    /// 365 days of 86,400) and Earth's day (86,400 world-seconds), so fixtures and tests have a
    /// concrete world to run before the owner sets the per-world scalars. Earth is one option
    /// among many, surfaced here for development only, never the canonical default: the owner
    /// sets the real per-world values through the reserved manifest scalars.
    pub fn dev_earth() -> OrbitalElements {
        OrbitalElements {
            orbital_period_seconds: Fixed::from_int(31_536_000),
            rotation_period_seconds: Fixed::from_int(86_400),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dev_earth_fixture_is_the_labelled_earth_orbit() {
        let e = OrbitalElements::dev_earth();
        // 365 days of 86,400 world-seconds is Earth's year; the day is 86,400 world-seconds.
        assert_eq!(e.orbital_period_seconds, Fixed::from_int(31_536_000));
        assert_eq!(e.rotation_period_seconds, Fixed::from_int(86_400));
        assert_eq!(
            e.orbital_period_seconds,
            Fixed::from_int(365).mul(Fixed::from_int(86_400))
        );
    }

    #[test]
    fn orbital_elements_are_value_comparable() {
        // Two worlds with different orbits are distinct values; the same orbit compares equal,
        // which is what the state-hash sensitivity leans on.
        let earth = OrbitalElements::dev_earth();
        let fast = OrbitalElements {
            orbital_period_seconds: Fixed::from_int(86_400),
            rotation_period_seconds: Fixed::from_int(3_600),
        };
        assert_ne!(earth, fast);
        assert_eq!(earth, OrbitalElements::dev_earth());
    }
}
