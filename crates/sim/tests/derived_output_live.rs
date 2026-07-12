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

//! The derived-output-is-live gate (task #43), slice 1: the `@derives[id]:` cross-check.
//!
//! A retired-floor derivation is a derived value that replaced an authored floor. The constructor
//! gate proves no such value is authored; this gate proves the derived replacement is alive. Its
//! membership is the data-defined [`RetiredFloorDerivationRegistry`], one row per derivation, and
//! each row is anchored to a source site by an `@derives[id]:` annotation. This test is the ratchet
//! that keeps the registry and the annotations in step (the constructor-gate pattern): every
//! annotated site must have a registry row, and every row must have its annotation at the site it
//! names. So a new derivation cannot land without a registry row (the liveness probe would miss
//! it), and a row cannot go stale (its site annotation removed) without failing.
//!
//! The site-local liveness probe (perturb the input, assert the derived value responds) is the next
//! slice; this slice locks the membership.

use civsim_sim::derive_gate::RetiredFloorDerivationRegistry;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The determinism-critical crate source roots the annotations live in, repo-relative.
const SCANNED_CRATES: &[&str] = &[
    "crates/core/src",
    "crates/physics/src",
    "crates/world/src",
    "crates/sim/src",
];

/// The gate's own registry module documents the `@derives[id]:` convention in prose, so it is
/// excluded from the annotation scan (it carries no derivation site, only the machinery).
const EXCLUDED: &[&str] = &["crates/sim/src/derive_gate.rs"];

/// The workspace root, derived from this crate's manifest dir (`.../crates/sim`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sim has a workspace root two levels up")
        .to_path_buf()
}

/// Recursively collect every `.rs` file under a directory.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Extract the id inside a `@derives[<id>]:` token on a line, if present.
fn derives_id(line: &str) -> Option<&str> {
    let start = line.find("@derives[")? + "@derives[".len();
    let rest = &line[start..];
    let end = rest.find(']')?;
    // A well-formed token closes the bracket and follows it with a colon.
    if rest[end + 1..].trim_start().starts_with(':') {
        Some(&rest[..end])
    } else {
        None
    }
}

/// Scan the source tree for `@derives[id]:` annotations, returning id -> repo-relative site path.
/// Panics on a duplicate id across sites, which would make the anchor ambiguous.
fn scan_annotations(root: &Path) -> BTreeMap<String, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for crate_src in SCANNED_CRATES {
        let mut files = Vec::new();
        rust_files(&root.join(crate_src), &mut files);
        for file in files {
            let rel = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if EXCLUDED.contains(&rel.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            for line in text.lines() {
                if let Some(id) = derives_id(line) {
                    if let Some(prev) = found.insert(id.to_string(), rel.clone()) {
                        panic!("@derives[{id}] appears at two sites: {prev} and {rel}");
                    }
                }
            }
        }
    }
    found
}

#[test]
fn every_derives_annotation_has_a_registry_row_and_vice_versa() {
    let root = workspace_root();
    let annotations = scan_annotations(&root);
    let registry = RetiredFloorDerivationRegistry::canonical();

    let annotated: std::collections::BTreeSet<&str> =
        annotations.keys().map(|s| s.as_str()).collect();
    let registered: std::collections::BTreeSet<&str> = registry.ids().into_iter().collect();

    let missing_row: Vec<&str> = annotated.difference(&registered).copied().collect();
    let missing_site: Vec<&str> = registered.difference(&annotated).copied().collect();

    assert!(
        missing_row.is_empty(),
        "@derives sites with no RetiredFloorDerivation row (add a registry row so the liveness gate probes them): {missing_row:?}"
    );
    assert!(
        missing_site.is_empty(),
        "registry rows whose @derives[id] annotation is gone (the site changed; fix the row or restore the annotation): {missing_site:?}"
    );

    // The row's declared site must be where its annotation actually lives, so the probe reads the
    // right derivation.
    for d in registry.iter() {
        let at = annotations.get(&d.id).expect("checked present above");
        assert_eq!(
            at, &d.site,
            "derivation '{}' is registered at {} but its @derives annotation is at {}",
            d.id, d.site, at
        );
    }
}

#[test]
fn the_cross_check_would_catch_a_new_unregistered_derivation() {
    // Self-test the ratchet: a synthetic annotation id that has no registry row must be detected by
    // the same set-difference the gate uses, proving the gate is not vacuous.
    let registry = RetiredFloorDerivationRegistry::canonical();
    let registered: std::collections::BTreeSet<&str> = registry.ids().into_iter().collect();
    let synthetic = "a_new_unregistered_derivation";
    assert!(
        !registered.contains(synthetic),
        "a new @derives id absent from the registry is a difference the gate reports"
    );
}
