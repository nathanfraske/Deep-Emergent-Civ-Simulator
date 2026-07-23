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

//! Stone 0, the local-firing provenance gate (task #79), wired into this crate's build so it runs on
//! every `cargo build`, `check`, `test`, and `clippy` of `civsim-sim`. The scan is whole-tree (the gate
//! finds a repository root and scans all gated crates), so wiring it here fires it for both the
//! workspace-wide commands and the `-p civsim-sim` forms an agent runs most. A positive detection
//! (a fixture with no provenance, a laundered override password, a tombstoned phrase) fails the build;
//! the owner's password override, supplied for one command through `STONE0_OVERRIDE`, releases a
//! provenance finding but never a laundering detection. The gate fails OPEN on any operational error
//! (python, git, or the secrets file absent), so it never bricks a build over infrastructure.
//!
//! The gate runs at BUILD time and emits nothing that reaches the compiled crate (no `cfg`, no generated
//! code, no compile-affecting env), so it cannot move the determinism state hash.

fn main() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    civsim_stone0::emit_cargo_rerun_inputs(&repo_root);

    let code = civsim_stone0::run(civsim_stone0::Mode::Local);
    if code != 0 {
        // The gate already printed its detailed report to stderr (captured and shown by cargo on a
        // failed build script). A short panic makes the block unmistakable and stops the build.
        panic!(
            "stone0 gate BLOCKED this build (see the report above). To proceed you must obtain the \
             current password from Nathan and set STONE0_OVERRIDE for this one command; do not write \
             the password into the repository."
        );
    }
}
