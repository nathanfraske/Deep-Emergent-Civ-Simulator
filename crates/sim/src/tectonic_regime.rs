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

//! # The tectonic-regime descriptive readout (the observer-side half of the corrected framing, #170)
//!
//! The blind framing panel (six panelists, unanimous) caught the proposed regime kernel as a closed-enum
//! template: a fixed set {mobile-lid, stagnant-lid, no-tectonics} consumed by downstream surface mechanisms
//! as a dispatch key is a high-level categorical fact read to produce a behaviour (the template case), and
//! it fails admit-the-alien (no slot for a heat-pipe, an episodic lid, or an ice-diapiric shell). The
//! corrected framing, gate-approved, keeps the tectonic BEHAVIOUR emergent from continuous physics: the
//! Rayleigh vigor governs whether the interior convects, and the convective driving stress against the lid's
//! own yield strength ([`civsim_physics::laws::convective_stress`] versus `mat.yield_strength`) governs
//! whether and where the lid mobilizes. The downstream mechanisms read those continuous quantities and
//! compute their own LOCAL outcomes, so a mobile lid, a stagnant lid, and everything between EMERGE.
//!
//! This module is the OTHER half: a regime NAME as a post-hoc DESCRIPTION of where a world sits in that
//! continuous (convection-onset, lid-mobilization) space, read for the glyph view, the event log, or
//! debugging. It is NEVER read back into a causal mechanism (P10) and NEVER written to canonical state: the
//! label describes the emergent outcome rather than producing it, the same shape as Hamilton's rule being a
//! description of kin cooperation, never its mechanism. The MEMBERSHIP (which named regimes exist and their
//! continuous-quantity bands) is DATA and a world extends it; the MECHANISM (match the state against the
//! bands) is fixed Rust, sibling to the value, semantic, and institution registries.

use civsim_core::Fixed;

/// The lid-mobilization margin: the convective driving stress over the lid's own yield strength. At or above
/// one the convective stress reaches the yield strength and the lid mobilizes; below one the lid is stagnant.
/// A non-positive yield strength (a lid with no strength) mobilizes under any stress, so the margin reads the
/// representable maximum. This is a physical criterion (stress against strength), not an authored regime
/// boundary. Deterministic fixed-point.
pub fn mobilization_margin(convective_stress: Fixed, yield_strength: Fixed) -> Fixed {
    if yield_strength <= Fixed::ZERO {
        // No strength to resist: fully mobilized regardless of the stress magnitude.
        return Fixed::MAX;
    }
    convective_stress
        .checked_div(yield_strength)
        .unwrap_or(Fixed::MAX)
        .max(Fixed::ZERO)
}

/// One regime DESCRIPTOR: a name and the continuous-quantity bands that describe it. All fields are data, so
/// a world adds a regime (heat-pipe, episodic, ice-diapiric, and so on) as a new row, never a code change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegimeDescriptor {
    /// The descriptive regime name (display and logs only).
    pub name: String,
    /// The convection state this regime describes: `Some(true)` convecting, `Some(false)` not, `None` either.
    pub requires_convection: Option<bool>,
    /// The inclusive lower bound of the lid-mobilization margin; `None` is unbounded below.
    pub min_mobilization: Option<Fixed>,
    /// The exclusive upper bound of the lid-mobilization margin; `None` is unbounded above.
    pub max_mobilization: Option<Fixed>,
}

impl RegimeDescriptor {
    /// Whether this descriptor's bands contain the given continuous state.
    fn matches(&self, convecting: bool, mobilization: Fixed) -> bool {
        if let Some(req) = self.requires_convection {
            if req != convecting {
                return false;
            }
        }
        if let Some(lo) = self.min_mobilization {
            if mobilization < lo {
                return false;
            }
        }
        if let Some(hi) = self.max_mobilization {
            if mobilization >= hi {
                return false;
            }
        }
        true
    }
}

/// The OPEN registry of regime descriptors, walked in registration order so the walk is deterministic (P3);
/// the first descriptor whose bands contain the state names it. The built-in membership is
/// [`RegimeDescriptorRegistry::canonical`]; a world registers more. This is OBSERVER-side: it reads the
/// continuous state and writes nothing back (P10).
#[derive(Clone, Debug, Default)]
pub struct RegimeDescriptorRegistry {
    descriptors: Vec<RegimeDescriptor>,
}

impl RegimeDescriptorRegistry {
    /// An empty registry.
    pub fn new() -> RegimeDescriptorRegistry {
        RegimeDescriptorRegistry::default()
    }

    /// Register a descriptor at the end of the walk order.
    pub fn register(&mut self, d: RegimeDescriptor) {
        self.descriptors.push(d);
    }

    /// The number of registered descriptors.
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// The descriptive regime NAME for a continuous state (whether the interior convects, and the
    /// lid-mobilization margin from [`mobilization_margin`]), or `None` where no descriptor matches (an honest
    /// gap, never a forced label). OBSERVER-side: it reads the state and writes nothing, so it is byte-neutral
    /// and never on the canonical run path.
    pub fn describe(&self, convecting: bool, mobilization: Fixed) -> Option<&str> {
        self.descriptors
            .iter()
            .find(|d| d.matches(convecting, mobilization))
            .map(|d| d.name.as_str())
    }

    /// The built-in descriptors: the Terran-familiar regimes as DATA rows (extensible, observer-side, never
    /// causal). No interior convection is the no-tectonics limit; a convecting interior whose stress reaches
    /// the lid yield strength (margin at or above one) is a mobile lid; a convecting interior whose stress
    /// stays below it (margin under one) is a stagnant lid. The margin of one is the physical stress-equals-
    /// strength point, not an authored regime boundary. A world with a different rheology adds its own regimes
    /// (heat-pipe, episodic, ice-diapiric) as new rows; nothing here is a closed set, and nothing reads the
    /// name to drive behaviour.
    pub fn canonical() -> RegimeDescriptorRegistry {
        let mut r = RegimeDescriptorRegistry::new();
        r.register(RegimeDescriptor {
            name: "no-convection".to_string(),
            requires_convection: Some(false),
            min_mobilization: None,
            max_mobilization: None,
        });
        r.register(RegimeDescriptor {
            name: "mobile-lid".to_string(),
            requires_convection: Some(true),
            min_mobilization: Some(Fixed::ONE),
            max_mobilization: None,
        });
        r.register(RegimeDescriptor {
            name: "stagnant-lid".to_string(),
            requires_convection: Some(true),
            min_mobilization: None,
            max_mobilization: Some(Fixed::ONE),
        });
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_margin_is_stress_over_strength_and_absence_reads_max() {
        // tau/sigma = 6/2 = 3, an over-yield lid.
        assert_eq!(
            mobilization_margin(Fixed::from_int(6), Fixed::from_int(2)),
            Fixed::from_int(3)
        );
        // A lid with no strength mobilizes under any stress.
        assert_eq!(
            mobilization_margin(Fixed::from_int(1), Fixed::ZERO),
            Fixed::MAX
        );
        // Zero stress is a zero margin (a still interior applies none).
        assert_eq!(
            mobilization_margin(Fixed::ZERO, Fixed::from_int(2)),
            Fixed::ZERO
        );
    }

    #[test]
    fn the_canonical_readout_names_the_three_familiar_regimes() {
        let r = RegimeDescriptorRegistry::canonical();
        // Not convecting: the no-tectonics limit, regardless of margin.
        assert_eq!(r.describe(false, Fixed::ZERO), Some("no-convection"));
        assert_eq!(r.describe(false, Fixed::from_int(5)), Some("no-convection"));
        // Convecting and the stress reaches the yield strength (margin >= 1): a mobile lid.
        assert_eq!(r.describe(true, Fixed::ONE), Some("mobile-lid"));
        assert_eq!(r.describe(true, Fixed::from_int(3)), Some("mobile-lid"));
        // Convecting but the stress stays below the yield strength (margin < 1): a stagnant lid, the case the
        // Rayleigh-only framing conflated with no convection (Mars and Venus convect under a stagnant lid).
        assert_eq!(
            r.describe(true, Fixed::from_ratio(1, 2)),
            Some("stagnant-lid")
        );
        assert_eq!(r.describe(true, Fixed::ZERO), Some("stagnant-lid"));
    }

    #[test]
    fn the_registry_is_open_and_a_world_regime_is_a_data_row() {
        // A world extends the taxonomy: a heat-pipe regime (convecting, extreme over-yield) is a new row that
        // takes precedence where it matches, proving the set is not closed.
        let mut r = RegimeDescriptorRegistry::new();
        r.register(RegimeDescriptor {
            name: "heat-pipe".to_string(),
            requires_convection: Some(true),
            min_mobilization: Some(Fixed::from_int(100)),
            max_mobilization: None,
        });
        for d in RegimeDescriptorRegistry::canonical().descriptors {
            r.register(d);
        }
        // An extreme-mobilization world reads the alien regime, not the Terran mobile-lid.
        assert_eq!(r.describe(true, Fixed::from_int(500)), Some("heat-pipe"));
        // A moderate one still reads mobile-lid.
        assert_eq!(r.describe(true, Fixed::from_int(3)), Some("mobile-lid"));
    }

    #[test]
    fn an_unmatched_state_reads_none_never_a_forced_label() {
        // An empty registry names nothing: an honest gap, not a forced default.
        let r = RegimeDescriptorRegistry::new();
        assert_eq!(r.describe(true, Fixed::ONE), None);
    }
}
