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

//! The steering / convergence-without-a-target audit (design Part 60 Stage 14,
//! Part 41, Part 61).
//!
//! This harness belongs in CI alongside the determinism and conservation harnesses,
//! but its subject does not exist yet: it tests that two seeds independently
//! populate the same physical attractor, which requires the technology design space,
//! a grounded physics substrate, and the reserved calibration values the runbook
//! holds for the owner. Building it now would mean fabricating both the mechanism
//! and its numbers, which is exactly the defect the audit exists to prevent.
//!
//! It is left as an honest, ignored placeholder so the slot is visible and the next
//! session knows the contract is owed, not met. It is not a passing test pretending
//! coverage that does not exist.

#[test]
#[ignore = "Stage 14: gated on the technology design space (Part 41) and its reserved physics-proxy calibrations; not yet built, not faked"]
fn convergence_without_a_target_is_forced_by_physics() {
    unimplemented!(
        "Two isolated cultures must reach the same physical attractor from different \
         seeds, with the convergence explained by physics and not authored bias \
         (steering audit). Build after Stage 14 lands and its reserved values are set."
    );
}
