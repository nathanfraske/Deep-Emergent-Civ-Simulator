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

//! The utility-AI decision layer (design Part 8).
//!
//! Each candidate action is scored as its base weight times the product of its
//! considerations, where a consideration reads a named input through a data-defined
//! response curve into the unit interval, and the highest scorer is chosen. Actions,
//! considerations, and curves are data ([`Behaviour`]), not code: a race or a world can
//! carry actions the engine's authors never enumerated, and an action that satisfies a
//! drive exists only where that drive does (Principle 11). The engine is fixed Rust; the
//! set and the numbers are data the owner provides through the world definition.
//!
//! Everything here is integer and fixed-point and deterministic. Curve evaluation is a
//! single fixed-point linear interpolation between sorted points (a defined, non-zero
//! denominator, so it is the stable kind of division rather than the data-dependent
//! near-zero kind the belief engine avoids). The choice is an argmax with the lowest
//! action id breaking ties, so a tie resolves the same way on every machine.

use std::collections::BTreeMap;

use civsim_core::Fixed;

/// A data-defined drive (a need that rises over time and that actions reduce).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DriveId(pub u32);

/// A data-defined action an agent can take.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ActionId(pub u32);

/// A data-defined world-fact input: a named reading an agent (or an institution's norm) can
/// condition on, drawn from the same open registry a [`Consideration`] reads (a drive level, a
/// value-axis stance, a perceived world fact). It is a newtype like [`DriveId`] and [`ActionId`],
/// not a closed enum, so a race or a world can condition on facts the engine's authors never
/// enumerated (Principle 11). The institution substrate's ADICO conditions key off this
/// registry rather than authoring a predicate catalogue of their own (design Part 36).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct InputId(pub u32);

/// A response curve as data: sorted `(x, y)` points in the unit interval, linearly
/// interpolated, clamped to the end points outside the range. Any monotone or
/// non-monotone shape is expressible by its points, so the curve family is open rather
/// than a closed enum of named shapes.
#[derive(Clone, Debug)]
pub struct Curve {
    points: Vec<(Fixed, Fixed)>,
}

impl Curve {
    /// Build a curve from points; they are sorted by x so evaluation can scan in order.
    pub fn new(points: impl IntoIterator<Item = (Fixed, Fixed)>) -> Self {
        let mut points: Vec<(Fixed, Fixed)> = points.into_iter().collect();
        points.sort_by_key(|(x, _)| *x);
        Curve { points }
    }

    /// The curve's sorted `(x, y)` points, for a canonical fold into a state hash (design Part 20:
    /// the mortality-hazard curve is canonical timeline state). The slice is in ascending-x order,
    /// so two curves built from the same points in any order expose the same slice.
    pub fn points(&self) -> &[(Fixed, Fixed)] {
        &self.points
    }

    /// Evaluate the curve at `x`, clamped to the end points. A flat curve with no points
    /// reads as zero.
    pub fn eval(&self, x: Fixed) -> Fixed {
        match self.points.first() {
            None => Fixed::ZERO,
            Some(&(x0, y0)) if x <= x0 => y0,
            _ => {
                let &(xn, yn) = self.points.last().unwrap();
                if x >= xn {
                    return yn;
                }
                for win in self.points.windows(2) {
                    let (x0, y0) = win[0];
                    let (x1, y1) = win[1];
                    if x >= x0 && x <= x1 {
                        if x1 == x0 {
                            return y0;
                        }
                        // y0 + (y1 - y0) * (x - x0) / (x1 - x0); one stable fixed-point div.
                        let frac = (x - x0).div(x1 - x0);
                        return y0 + (y1 - y0).mul(frac);
                    }
                }
                yn
            }
        }
    }
}

/// A consideration: read one named input's current value through a curve. The input is an
/// [`InputId`] drawn from the shared decision-input registry, so a consideration reads a drive
/// level, a personality-trait value, or any world fact the world projects into that registry
/// through one uniform path (Principle 11), rather than a closed per-source field. The scorer reads
/// the value from a [`BTreeMap<InputId, Fixed>`] readings map; an input the readings do not carry
/// reads as zero.
#[derive(Clone, Debug)]
pub struct Consideration {
    /// The registry input whose value is read through the curve (a drive level, a trait value, a
    /// world fact). Keyed off the open [`InputId`] registry the world projects its readings into.
    pub input: InputId,
    /// The index of the response curve in [`Behaviour::curves`].
    pub curve: usize,
}

/// One action's scoring definition and what it reduces.
#[derive(Clone, Debug)]
pub struct ActionDef {
    /// The action's id.
    pub id: ActionId,
    /// The base weight (a constant factor before the considerations).
    pub weight: Fixed,
    /// The considerations multiplied into the score.
    pub considerations: Vec<Consideration>,
    /// The drives this action reduces when taken.
    pub satisfies: Vec<DriveId>,
}

/// A drive's dynamics: how fast it rises each tick and how much an action that satisfies
/// it reduces it. Data the owner provides; the dev fixtures are placeholders.
#[derive(Clone, Copy, Debug)]
pub struct DriveDef {
    /// The drive's id.
    pub id: DriveId,
    /// How much the drive's level rises each tick (clamped into the unit interval).
    pub rise_per_tick: Fixed,
    /// How much a satisfying action reduces the level.
    pub satisfy_amount: Fixed,
}

/// The data-driven decision definitions: the drives, the curves, and the actions. The
/// set is open data; the mechanism that scores and chooses is fixed Rust.
#[derive(Clone, Debug, Default)]
pub struct Behaviour {
    /// The drives an agent has.
    pub drives: Vec<DriveDef>,
    /// The response curves the considerations reference by index.
    pub curves: Vec<Curve>,
    /// The actions an agent can take.
    pub actions: Vec<ActionDef>,
}

impl Behaviour {
    /// Score one action against a set of input readings: the base weight times the product
    /// of its considerations, each read through its curve. A consideration whose curve
    /// index is out of range contributes zero, collapsing the score, so a malformed
    /// definition cannot inflate a choice. The readings are keyed by [`InputId`], so an action
    /// scores over drive levels and trait values through one map.
    pub fn score(&self, action: &ActionDef, readings: &BTreeMap<InputId, Fixed>) -> Fixed {
        let mut s = action.weight;
        for c in &action.considerations {
            let x = readings.get(&c.input).copied().unwrap_or(Fixed::ZERO);
            let y = match self.curves.get(c.curve) {
                Some(curve) => curve.eval(x),
                None => Fixed::ZERO,
            };
            s = s.mul(y);
        }
        s
    }

    /// Choose the highest-scoring action, breaking ties by the lowest action id so the
    /// choice is deterministic. Returns `None` only when there are no actions.
    pub fn choose(&self, readings: &BTreeMap<InputId, Fixed>) -> Option<ActionId> {
        let mut ordered: Vec<&ActionDef> = self.actions.iter().collect();
        ordered.sort_by_key(|a| a.id);
        let mut best: Option<(ActionId, Fixed)> = None;
        for a in ordered {
            let s = self.score(a, readings);
            match best {
                Some((_, bs)) if s <= bs => {}
                _ => best = Some((a.id, s)),
            }
        }
        best.map(|(id, _)| id)
    }

    /// The action with this id, if defined.
    pub fn action(&self, id: ActionId) -> Option<&ActionDef> {
        self.actions.iter().find(|a| a.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_ramp() -> Curve {
        // y = x on the unit interval: the level itself is the urgency.
        Curve::new([(Fixed::ZERO, Fixed::ZERO), (Fixed::ONE, Fixed::ONE)])
    }

    #[test]
    fn curve_interpolates_and_clamps() {
        let c = unit_ramp();
        assert_eq!(c.eval(Fixed::ZERO), Fixed::ZERO);
        assert_eq!(c.eval(Fixed::ONE), Fixed::ONE);
        assert_eq!(c.eval(Fixed::from_ratio(1, 2)), Fixed::from_ratio(1, 2));
        // clamps past the ends.
        assert_eq!(c.eval(Fixed::from_int(-3)), Fixed::ZERO);
        assert_eq!(c.eval(Fixed::from_int(3)), Fixed::ONE);
    }

    #[test]
    fn the_more_urgent_input_wins() {
        // The two inputs happen to be drive levels here (a drive reads at the input of its own id),
        // but the scorer sees only the InputId readings map: an action wins on whichever input the
        // curve turns into the higher urgency.
        let hunger = InputId(0);
        let fatigue = InputId(1);
        let forage = ActionId(0);
        let rest = ActionId(1);
        let b = Behaviour {
            drives: vec![],
            curves: vec![unit_ramp()],
            actions: vec![
                ActionDef {
                    id: forage,
                    weight: Fixed::ONE,
                    considerations: vec![Consideration {
                        input: hunger,
                        curve: 0,
                    }],
                    satisfies: vec![DriveId(hunger.0)],
                },
                ActionDef {
                    id: rest,
                    weight: Fixed::ONE,
                    considerations: vec![Consideration {
                        input: fatigue,
                        curve: 0,
                    }],
                    satisfies: vec![DriveId(fatigue.0)],
                },
            ],
        };
        let mut readings = BTreeMap::new();
        readings.insert(hunger, Fixed::from_ratio(3, 4));
        readings.insert(fatigue, Fixed::from_ratio(1, 4));
        assert_eq!(
            b.choose(&readings),
            Some(forage),
            "the hungry agent forages"
        );
        // Flip the urgencies and the choice flips.
        readings.insert(hunger, Fixed::from_ratio(1, 4));
        readings.insert(fatigue, Fixed::from_ratio(3, 4));
        assert_eq!(b.choose(&readings), Some(rest));
    }

    #[test]
    fn a_tie_breaks_to_the_lowest_action_id() {
        let i = InputId(0);
        let b = Behaviour {
            drives: vec![],
            curves: vec![unit_ramp()],
            actions: vec![
                ActionDef {
                    id: ActionId(5),
                    weight: Fixed::ONE,
                    considerations: vec![Consideration { input: i, curve: 0 }],
                    satisfies: vec![],
                },
                ActionDef {
                    id: ActionId(2),
                    weight: Fixed::ONE,
                    considerations: vec![Consideration { input: i, curve: 0 }],
                    satisfies: vec![],
                },
            ],
        };
        let mut readings = BTreeMap::new();
        readings.insert(i, Fixed::from_ratio(1, 2));
        assert_eq!(b.choose(&readings), Some(ActionId(2)));
    }
}
