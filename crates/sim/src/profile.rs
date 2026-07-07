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

//! Coarse per-phase wall-clock profiling, gated on the `CIVSIM_PROFILE` environment variable. This is
//! OFF the canonical path by construction: it measures elapsed time only, mutates no simulation state,
//! draws no randomness, and folds into no `state_hash`, so it cannot perturb determinism (Principle 3).
//! It exists to answer, with real numbers rather than a hypothesis, where a tick's wall-clock goes
//! before any GPU offload target is chosen: the field stencil, the other grid folds, the per-being work,
//! or the mind. When the flag is unset every helper is a thin pass-through (one cached bool read plus a
//! branch), so a normal run pays effectively nothing.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

/// The phase buckets. Ordered so the grid folds, the per-being work, and the mind are separable.
pub const PHASES: usize = 9;
pub const NAMES: [&str; PHASES] = [
    "field.step",    // the thermal-diffusion stencil (GPU offload target 1)
    "env.step",      // hydrology + salinity + productivity stencils (grid fold)
    "regrow",        // regrow_supply: food/water/salinity write-back per cell (grid fold)
    "combustion",    // the fire beat (grid fold)
    "matter_cycle",  // decomposition / matter cycle (grid fold)
    "body_exchange", // per-being Newton cooling (GPU target 3)
    "embodiment",    // per-being perception/decision/movement (GPU target 2/3)
    "world.tick",    // the composed cognition world + conversation coupling
    "other",         // anything not wrapped
];
pub const P_FIELD: usize = 0;
pub const P_ENV: usize = 1;
pub const P_REGROW: usize = 2;
pub const P_COMBUST: usize = 3;
pub const P_MATTER: usize = 4;
pub const P_BODY: usize = 5;
pub const P_EMB: usize = 6;
pub const P_WORLD: usize = 7;

static ACC: [AtomicU64; PHASES] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

static ENABLED: OnceLock<bool> = OnceLock::new();

/// Whether profiling is armed, read once from the environment and cached.
#[inline]
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| std::env::var("CIVSIM_PROFILE").is_ok())
}

/// An RAII timer: when armed, it records the elapsed time into `phase` on drop, so a phase is timed by
/// opening a scope over it inline (no closure capture, so the borrow checker sees the wrapped code
/// exactly as it was). A no-op guard when unarmed.
pub struct Scope {
    phase: usize,
    start: Instant,
    armed: bool,
}

impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        if self.armed {
            ACC[self.phase].fetch_add(self.start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        }
    }
}

/// Open a timing scope over the current phase. Hold the returned guard for the duration of the phase
/// (`let _g = profile::scope(P_FIELD);`); it records on drop. Cheap and inert when unarmed.
#[inline]
pub fn scope(phase: usize) -> Scope {
    Scope {
        phase,
        start: Instant::now(),
        armed: enabled(),
    }
}

/// Print the accumulated per-phase breakdown to stderr, once, at the end of a run. A no-op when unarmed.
pub fn report() {
    if !enabled() {
        return;
    }
    let per: Vec<u64> = (0..PHASES)
        .map(|i| ACC[i].load(Ordering::Relaxed))
        .collect();
    let total: u64 = per.iter().sum();
    let denom = total.max(1) as f64;
    eprintln!(
        "=== per-phase profile (wrapped total {:.3}s) ===",
        total as f64 / 1e9
    );
    for i in 0..PHASES {
        eprintln!(
            "  {:14} {:9.4}s  {:5.1}%",
            NAMES[i],
            per[i] as f64 / 1e9,
            100.0 * per[i] as f64 / denom
        );
    }
}
