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

//! Link the canonical planet front door to the shared Stone 0 build guard.
//!
//! The common build dependency runs the repository-wide guard once per Cargo
//! build graph. This consumer sentinel makes removal of that dependency a
//! compile failure and emits no simulation input.

fn main() {
    civsim_stone0_build::assert_guard_linked();
}
