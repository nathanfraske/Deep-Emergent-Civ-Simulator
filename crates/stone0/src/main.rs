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

//! `stone0-gate`: the Stone 0 provenance gate binary. CI runs it with `--ci`, its self-proof with
//! `--self-test`, and a developer runs it with no flag for the full local check. It is not yet wired
//! into any build script (increment 4 is reserved for the owner to gate).

use civsim_stone0::{run, Mode};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = if args.iter().any(|a| a == "--self-test") {
        Mode::SelfTest
    } else if args.iter().any(|a| a == "--ci") {
        Mode::Ci
    } else {
        Mode::Local
    };
    std::process::exit(run(mode));
}
