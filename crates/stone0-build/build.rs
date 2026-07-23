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

use std::path::Path;

const MARKER_NAME: &str = "stone0-build-guard.rs";
const MARKER_SOURCE: &str = "pub fn token() -> &'static str { \"civsim.stone0.build-guard.v1\" }\n";

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    civsim_stone0::emit_cargo_rerun_inputs(&repo_root);

    let marker = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR to the Stone 0 build guard"),
    )
    .join(MARKER_NAME);
    let _ = std::fs::remove_file(&marker);

    let code = civsim_stone0::run(civsim_stone0::Mode::Local);
    if code != 0 {
        panic!(
            "Stone 0 blocked the canonical build graph. Resolve the reported provenance finding, or obtain the current one-command owner override out of band. Never write that override into the repository."
        );
    }

    std::fs::write(&marker, MARKER_SOURCE)
        .expect("the successful Stone 0 build guard must write its linkage marker");
    println!(
        "cargo:rustc-env=CIVSIM_STONE0_GUARD_MARKER={}",
        marker.display()
    );
}
