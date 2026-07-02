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

//! The canonical runner's field layer: a located being exchanges heat with a physics temperature
//! field that diffuses and relaxes toward its baseline, and the whole run reproduces bit for bit.
//! The field baseline and the calibrations here are CLEARLY-LABELLED TEST FIXTURES, the only
//! sanctioned use of an authored number; the runner mechanism itself carries no default.

use civsim_core::{Fixed, StableId};
use civsim_sim::runner::{Field, FieldCalib, Runner};
use civsim_world::Coord3;

fn k(n: i64, d: i64) -> Fixed {
    Fixed::from_ratio(n, d)
}

// A labelled test fixture: a 5x5 field, cold everywhere (280 K) except a hot centre cell (400 K).
fn fixture_field() -> Field {
    let (w, h) = (5, 5);
    let mut baseline = vec![Fixed::from_int(280); (w * h) as usize];
    baseline[(2 * w + 2) as usize] = Fixed::from_int(400); // centre
    Field::new(w, h, baseline)
}

// Labelled test calibrations (not canonical values).
fn fixture_calib() -> FieldCalib {
    FieldCalib {
        diffusion: k(1, 10), // 0.1, within the 0.25 stability bound
        relaxation: k(1, 100),
        exchange: k(1, 5),
    }
}

#[test]
fn heat_flows_from_the_field_into_a_cold_body() {
    let mut r = Runner::new(fixture_field(), fixture_calib());
    let who = StableId(1);
    // A cold being (270 K) stands on the hot centre cell.
    r.place_being(who, Coord3::new(2, 2, 0), Fixed::from_int(270));
    let start = r.body_temp(who).unwrap().to_f64_lossy();
    for _ in 0..20 {
        r.step();
    }
    let end = r.body_temp(who).unwrap().to_f64_lossy();
    assert!(
        end > start + 1.0,
        "the body warms toward the hot cell it stands on ({start} -> {end})"
    );
    // It never overshoots the environment it is exchanging with (Newton cooling is contractive).
    assert!(end < 400.0, "the body cannot exceed the source temperature");
}

#[test]
fn the_field_diffuses_and_relaxes_deterministically() {
    let mut r = Runner::new(fixture_field(), fixture_calib());
    let edge_before = r.field().at(2, 0).to_f64_lossy(); // a cold edge cell
    let centre_before = r.field().at(2, 2).to_f64_lossy();
    for _ in 0..10 {
        r.step();
    }
    let centre_after = r.field().at(2, 2).to_f64_lossy();
    let near_after = r.field().at(2, 1).to_f64_lossy(); // adjacent to the hot centre
    assert!(
        centre_after < centre_before,
        "the hot centre cools as heat spreads"
    );
    assert!(
        near_after > edge_before,
        "a neighbour of the hot centre warms"
    );
}

#[test]
fn a_run_reproduces_bit_for_bit() {
    // The same fixture stepped twice yields the identical canonical hash at every tick: the runner is
    // a pure deterministic function of its inputs (Principle 3), independent of thread count (the step
    // walks cells and beings in canonical order, reads no camera, and draws no randomness).
    let build = || {
        let mut r = Runner::new(fixture_field(), fixture_calib());
        r.place_being(StableId(7), Coord3::new(2, 2, 0), Fixed::from_int(270));
        r.place_being(StableId(3), Coord3::new(0, 0, 0), Fixed::from_int(300));
        r
    };
    let mut a = build();
    let mut b = build();
    for _ in 0..30 {
        a.step();
        b.step();
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "identical inputs replay identically at tick {}",
            a.clock()
        );
    }
    assert!(a.clock() == 30);
}
