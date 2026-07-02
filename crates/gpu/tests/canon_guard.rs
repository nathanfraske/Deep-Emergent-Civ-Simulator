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

//! A hardcoded steering guard for the GPU canonical path, the sibling of the physics substrate's
//! `no float in canon` guard (`crates/physics/tests/steering.rs`): the kernel source that runs on the
//! canonical path must carry no floating-point type, so a float can never silently enter a
//! bit-identity contract (design Part 3.4). This is a source scan and needs no device, so it always
//! runs (it is not gated on `CIVSIM_GPU`), turning a prose invariant into a build-enforced check.

use std::path::Path;

// The CubeCL floating-point element types. A float on the canonical path has to be one of these
// (a typed value or cast); a bare untyped literal will not compile in an integer-only `#[cube]`
// kernel, so scanning for the type tokens is the sound, false-positive-free guard. None of these
// tokens appears in the current sources, including the prose comments (`floor`, `float` do not match).
const FLOAT_TOKENS: [&str; 4] = ["f16", "bf16", "f32", "f64"];

fn assert_no_float(rel: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
    for (n, line) in src.lines().enumerate() {
        for tok in FLOAT_TOKENS {
            assert!(
                !line.contains(tok),
                "{rel}:{}: the canonical GPU kernel path must be integer-only, found `{tok}`: {}",
                n + 1,
                line.trim()
            );
        }
    }
}

#[test]
fn the_canonical_kernels_carry_no_floating_point() {
    // The two files that hold `#[cube]` kernels compiled onto the canonical path.
    assert_no_float("src/stage0.rs");
    assert_no_float("src/field.rs");
}
