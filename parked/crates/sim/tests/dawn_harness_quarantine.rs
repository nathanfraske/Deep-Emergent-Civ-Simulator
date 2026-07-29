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

//! The dawn-harness quarantine, build-enforced (owner ruling 2026-07-18).
//!
//! `crates/sim/src/dawn_harness.rs` assembles the dawn band for the quarantined example harnesses
//! (`docs/QUARANTINE.md`). It is not canonical, and it must not be reused as though it were: the
//! assembly reads through six modules carrying quarantine-ledger entries (`anatomy`,
//! `conviction_experience`, `discovery`, `edibility`, `homeostasis`, `morphogen`), whose values have
//! absent or dead provenance, so canonical code built on this path silently inherits frozen-provenance
//! values. That is a defect under the value-authoring line (Principle 11).
//!
//! Prose could not hold the line here. The module previously called itself "the production world-build
//! path", and was repeatedly found and adopted as canon on the strength of that label. So the rule
//! moves leftward, from a claim in a document to a check the build runs: a canonical `src/` module that
//! references the harness in CODE fails this test.
//!
//! What is allowed, deliberately:
//!   - `lib.rs` may declare the module and re-export its surface, which is how tests, examples, and
//!     benches reach it. Those consumers are the harness's whole purpose.
//!   - Any module may reference it in a DOC comment (an intra-doc link, a note on why a path is off
//!     canon). Documenting the boundary is not crossing it.
//!
//! The honest ceiling: this catches a direct textual reference from canonical source. It cannot catch
//! a concept carried across by hand (a value copied out, a routine reimplemented with the harness's
//! numbers). `docs/QUARANTINE.md` owns that rule (a harness's code path may be reused, its numbers may
//! not), and it stays a review responsibility, stated here so the limit is not mistaken for coverage.

use std::fs;
use std::path::Path;

/// The harness surface a canonical module must not reach for.
const QUARANTINED_SYMBOLS: [&str; 5] = [
    "dawn_harness::",
    "build_dawn_runner",
    "DawnPeoples",
    "arm_dawn_languages",
    "assemble_dawn_embodiment",
];

/// Strip a line of anything that is not executable code, so a doc link or a note about the boundary
/// does not read as a use of it. A line whose code part is empty cannot reference anything.
fn code_part(line: &str) -> &str {
    let t = line.trim_start();
    if t.starts_with("//") {
        return "";
    }
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

/// `lib.rs` owns the module declaration and the re-export; both are the sanctioned door.
fn is_sanctioned_export(file: &str, code: &str) -> bool {
    if file != "lib.rs" {
        return false;
    }
    let c = code.trim();
    c.starts_with("pub mod dawn_harness")
        || c.starts_with("pub use dawn_harness::")
        // the re-export spans several lines; its body is a bare symbol list
        || c.chars()
            .all(|ch| ch.is_alphanumeric() || "_,{} ".contains(ch))
}

#[test]
fn no_canonical_module_reuses_the_quarantined_dawn_harness() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offences: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    let entries = fs::read_dir(&src).expect("the sim crate's src directory is readable");
    for entry in entries {
        let path = entry.expect("a readable dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a utf-8 file name")
            .to_string();
        // The harness may of course speak of itself.
        if name == "dawn_harness.rs" {
            continue;
        }
        let text = fs::read_to_string(&path).expect("a readable source file");
        scanned += 1;
        for (i, line) in text.lines().enumerate() {
            let code = code_part(line);
            if code.trim().is_empty() || is_sanctioned_export(&name, code) {
                continue;
            }
            for sym in QUARANTINED_SYMBOLS {
                if code.contains(sym) {
                    offences.push(format!(
                        "  {}:{}: `{}` in `{}`",
                        name,
                        i + 1,
                        sym,
                        code.trim()
                    ));
                }
            }
        }
    }

    assert!(
        scanned > 0,
        "the quarantine gate scanned no files, so it proves nothing; check the src path"
    );
    assert!(
        offences.is_empty(),
        "canonical sim modules reference the QUARANTINED dawn harness in code ({} site(s)):\n{}\n\n\
         The dawn harness is example scaffolding, not canon (docs/QUARANTINE.md). It reads through \
         modules carrying quarantine-ledger values with absent or dead provenance, so canonical code \
         built on it inherits them. Do not wire it into the run path. Reimplement what you need on the \
         canonical side reading manifest values fail-loud: a harness's code path may be reused, its \
         numbers may not. Tests, examples, and benches may use the harness freely.",
        offences.len(),
        offences.join("\n")
    );
}

/// The gate must be able to fail. A checker that cannot convict is not a gate, and this one is pure
/// text matching, so its detector is testable directly on synthetic lines.
#[test]
fn the_quarantine_gate_convicts_a_real_reuse_and_acquits_a_doc_reference() {
    // A genuine reuse from a canonical module.
    let reuse = "    let runner = crate::dawn_harness::build_dawn_runner(&scenario, &peoples)?;";
    let code = code_part(reuse);
    assert!(
        QUARANTINED_SYMBOLS.iter().any(|s| code.contains(s)),
        "the gate must convict a canonical call into the harness"
    );
    assert!(!is_sanctioned_export("runner.rs", code));

    // A doc comment naming the same symbol is not a use of it.
    let doc =
        "    /// see [`build_dawn_runner`](crate::dawn_harness::build_dawn_runner) for the seam";
    assert!(
        code_part(doc).is_empty(),
        "a doc comment must not read as code"
    );

    // A trailing comment is stripped, and the code before it still counts.
    let trailing = "    let x = 1; // build_dawn_runner is the harness entry point";
    assert!(
        !QUARANTINED_SYMBOLS
            .iter()
            .any(|s| code_part(trailing).contains(s)),
        "a trailing comment must not convict"
    );

    // lib.rs's declaration and re-export are the sanctioned door.
    assert!(is_sanctioned_export("lib.rs", "pub mod dawn_harness;"));
    assert!(is_sanctioned_export("lib.rs", "pub use dawn_harness::{"));
    assert!(
        !is_sanctioned_export("runner.rs", "pub use dawn_harness::{"),
        "only lib.rs may re-export the harness"
    );
}
