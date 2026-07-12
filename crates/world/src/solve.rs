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

//! The fixed-cap iterative solve, a Layer-0 determinism primitive for the genesis-forward geology
//! solvers (the convection and Stokes-flow field relaxations that mantle dynamics and plate emergence
//! run, whose per-step kernels live in the physics floor).
//!
//! The determinism hazard this primitive removes is the UNBOUNDED "iterate until converged" loop: a
//! float-tolerance convergence test can take a different number of iterations on a different platform
//! (or a different compile), so two machines diverge on a bit that then propagates. This driver forbids
//! that shape. It applies the caller's `step` kernel at most a FIXED `cap` times, and it tests
//! convergence on an INTEGER residual against an integer threshold, so the stopping decision is an exact
//! comparison with no platform-dependent floating tolerance and the iteration count is bounded and
//! identical everywhere (Principle 3). Convergence is a reported outcome, never a precondition for
//! returning: the solve always terminates in at most `cap` steps, and whether it reached the threshold
//! is a fact the caller reads, not a spin it can hang on.
//!
//! The driver is generic over the state and the kernel, so it is the reusable harness the convection and
//! Stokes solves plug their physics-floor per-step update into; the update itself is the caller's, this
//! is only the deterministic outer loop.

/// The outcome of a [`fixed_cap_solve`]: the final state, how many steps ran, whether the integer
/// residual reached the threshold within the cap, and the last residual measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SolveOutcome<S> {
    /// The state after the last step (or the initial state if the cap was zero).
    pub state: S,
    /// The number of steps applied (at most the cap).
    pub iterations: u32,
    /// Whether the integer residual fell to or below the threshold within the cap.
    pub converged: bool,
    /// The residual after the last step, or `u64::MAX` if no step ran (a zero cap).
    pub final_residual: u64,
}

/// Iterate `step` from `initial` at most `cap` times, stopping the moment the INTEGER `residual` between
/// the previous and the new state is at or below `threshold`. Deterministic termination by construction:
/// the loop is bounded by `cap` (never an unbounded until-converged spin), and the convergence test is an
/// exact integer comparison (no float tolerance whose iteration count could differ across machines). The
/// returned [`SolveOutcome`] reports whether the threshold was reached; convergence is never a
/// precondition for returning.
pub fn fixed_cap_solve<S>(
    initial: S,
    cap: u32,
    threshold: u64,
    step: impl Fn(&S) -> S,
    residual: impl Fn(&S, &S) -> u64,
) -> SolveOutcome<S> {
    let mut state = initial;
    let mut iterations = 0u32;
    let mut final_residual = u64::MAX;
    let mut converged = false;
    while iterations < cap {
        let next = step(&state);
        let r = residual(&state, &next);
        state = next;
        iterations += 1;
        final_residual = r;
        if r <= threshold {
            converged = true;
            break;
        }
    }
    SolveOutcome {
        state,
        iterations,
        converged,
        final_residual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The max absolute change over an i64 field, a representative integer residual for a field relaxation
    // (the shape a convection or Stokes solve uses).
    fn max_change(a: &[i64], b: &[i64]) -> u64 {
        a.iter()
            .zip(b)
            .map(|(&x, &y)| x.abs_diff(y))
            .max()
            .unwrap_or(0)
    }

    // One Jacobi smoothing step over a 1-D field with fixed Dirichlet ends: each interior cell moves to
    // the integer mean of its neighbours. This relaxes to a straight line between the ends, a contraction,
    // so it converges, the same shape a diffusion/Stokes relaxation has.
    fn smooth(field: &[i64]) -> Vec<i64> {
        let n = field.len();
        (0..n)
            .map(|i| {
                if i == 0 || i + 1 == n {
                    field[i] // fixed ends
                } else {
                    (field[i - 1] + field[i + 1]) / 2
                }
            })
            .collect()
    }

    #[test]
    fn a_converging_relaxation_stops_before_the_cap() {
        // A field with a spike in the middle relaxes toward the line between the fixed ends (0 and 0), so
        // the residual falls to zero well before a generous cap.
        let initial = vec![0i64, 0, 64, 0, 0];
        let out = fixed_cap_solve(initial, 100, 0, |f| smooth(f), |a, b| max_change(a, b));
        assert!(
            out.converged,
            "the relaxation reaches the zero-residual fixed point"
        );
        assert!(
            out.iterations < 100,
            "it stops at convergence, not at the cap"
        );
        assert_eq!(out.final_residual, 0);
        // The fixed point is the flat field between the zero ends.
        assert_eq!(out.state, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_non_converging_solve_stops_at_the_cap_never_unbounded() {
        // A step that never settles below the threshold must terminate at the cap, reporting not-converged,
        // rather than spinning forever. Here the residual is always 1 (a perpetual unit shuffle).
        let out = fixed_cap_solve(
            0i64,
            7,
            0,
            |x| x + 1,             // never settles
            |a, b| a.abs_diff(*b), // always 1
        );
        assert!(!out.converged);
        assert_eq!(out.iterations, 7, "bounded by the cap");
        assert_eq!(out.final_residual, 1);
        assert_eq!(out.state, 7);
    }

    #[test]
    fn a_loose_threshold_converges_earlier_than_a_tight_one() {
        // The integer threshold is an exact comparison: a looser threshold accepts a larger residual and
        // so stops sooner. Both are deterministic; neither is a float tolerance.
        let initial = vec![0i64, 0, 100, 0, 0];
        let tight = fixed_cap_solve(
            initial.clone(),
            100,
            0,
            |f| smooth(f),
            |a, b| max_change(a, b),
        );
        let loose = fixed_cap_solve(initial, 100, 8, |f| smooth(f), |a, b| max_change(a, b));
        assert!(loose.converged && tight.converged);
        assert!(
            loose.iterations <= tight.iterations,
            "a looser integer threshold stops no later"
        );
    }

    #[test]
    fn the_solve_is_deterministic() {
        // Same inputs, bit-identical outcome: the cap and the integer residual leave no platform-dependent
        // convergence decision.
        let initial = vec![3i64, 9, 27, 81, 5, 1];
        let a = fixed_cap_solve(
            initial.clone(),
            50,
            2,
            |f| smooth(f),
            |x, y| max_change(x, y),
        );
        let b = fixed_cap_solve(initial, 50, 2, |f| smooth(f), |x, y| max_change(x, y));
        assert_eq!(a, b);
    }

    #[test]
    fn a_zero_cap_takes_no_step() {
        let out = fixed_cap_solve(
            vec![1i64, 2, 3],
            0,
            0,
            |f| smooth(f),
            |a, b| max_change(a, b),
        );
        assert_eq!(out.iterations, 0);
        assert!(!out.converged);
        assert_eq!(out.final_residual, u64::MAX);
        assert_eq!(
            out.state,
            vec![1, 2, 3],
            "the initial state is returned untouched"
        );
    }
}
