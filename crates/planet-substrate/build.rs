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

//! Local-firing Stone 0 guard for the active planet mechanism candidates.

fn main() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    civsim_stone0::emit_cargo_rerun_inputs(&repo_root);

    let code = civsim_stone0::run(civsim_stone0::Mode::Local);
    if code != 0 {
        panic!(
            "Stone 0 blocked the active planet substrate build. Resolve the reported provenance finding, or obtain the current one-command owner override out of band. Never write that override into the repository."
        );
    }
}
