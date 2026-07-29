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

//! Build-graph singleton for the repository-wide Stone 0 guard.
//!
//! The generated module exists only after this package's build script has run
//! Stone 0 successfully. Canonical consumer build scripts call
//! [`assert_guard_linked`] so removing this package, its build script, or its
//! generated marker fails closed at compilation.

#![forbid(unsafe_code)]

mod generated {
    include!(env!("CIVSIM_STONE0_GUARD_MARKER"));
}

const EXPECTED_TOKEN: &str = "civsim.stone0.build-guard.v1";

/// Assert that this library was compiled from a successful shared guard run.
pub fn assert_guard_linked() {
    assert_eq!(
        generated::token(),
        EXPECTED_TOKEN,
        "Stone 0 build-guard linkage marker is invalid"
    );
}
